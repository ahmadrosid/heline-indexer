mod github;
mod parser;
mod solr;

use serde_json::Value;

use crate::solr::GithubFile;
use ignore::Walk;
use loading::Loading;
use reqwest::Url;
use select::document::Document;
use select::predicate::{Class, Name};
use std::path::Path;
use std::process::{Command, Stdio};

#[tokio::main]
pub async fn main() {
    let mut log = Loading::new();
    log.start();

    let mut base_url = String::new();
    match std::env::var("BASE_URL") {
        Ok(val) => base_url.push_str(&val),
        Err(e) => {
            log.fail(format!("BASE_URL: {}!", e));
            log.end();
            std::process::exit(1);
        }
    }

    let file = match std::env::args().nth(1) {
        Some(file) => file,
        None => {
            log.fail("Please provide path!");
            log.end();
            std::process::exit(1);
        }
    };

    let option = match std::env::args().nth(2) {
        Some(option) => option,
        None => String::new(),
    };

    let mut value: Vec<String> = vec![];

    match Url::parse(&file) {
        Ok(_) => value.push(file.to_string()),
        Err(_) => {
            if !Path::new(&file).exists() {
                let cwd = std::env::current_dir().unwrap();
                log.fail(format!(
                    "File not exists: {}/{}",
                    cwd.as_path().display(),
                    file
                ));
                std::process::exit(1);
            }
            value = parse_json(&file)
        }
    }

    let mut index = 0;
    let max_index = 1;
    for val in value {
        if index == max_index {
            break;
        }
        index += 1;
        let cwd = "repos";
        let git_url = val.to_string();
        let paths: Vec<&str> = git_url.split("/").collect();
        let repo_name = paths.last().unwrap();
        let github_repo = format!("{}/{}", paths[paths.len() - 2], paths[paths.len() - 1]);
        let dir = &format!("{}/{}", cwd, repo_name);

        match &option[..] {
            "--folder" => index_directory(dir, &github_repo, log.to_owned(), &base_url).await,
            _ => {
                match github::get_repo(&github_repo).await {
                    Ok(_) => {}
                    Err(e) => {
                        log.warn(format!("{}: Error {}", github_repo, e));
                        continue;
                    }
                }

                log.text(format!("Cloning '{}'", val.to_string()));
                let success = exec_command(
                    Command::new("git")
                        .current_dir(cwd)
                        .arg("clone")
                        .arg(&val.to_string())
                        .arg(repo_name),
                );

                if success {
                    let dir = &format!("{}/{}", cwd, repo_name);
                    index_directory(dir, &github_repo, log.to_owned(), &base_url).await;
                    exec_command(
                        Command::new("rm")
                            .current_dir(".")
                            .arg("-rf")
                            .arg(format!("{}/{}", cwd, repo_name)),
                    );
                } else {
                    log.fail(format!("Failed to clone: {}", git_url));
                }
            }
        }
    }
    log.end();
}

fn get_branch_name(dir: &str) -> String {
    let file_path = [dir, ".git/HEAD"].join("/");
    return match std::fs::read_to_string(file_path) {
        Ok(file) => file
            .split("/")
            .last()
            .unwrap()
            .to_string()
            .trim_end_matches('\n')
            .to_string(),
        Err(_) => "master".to_string(),
    };
}

#[track_caller]
pub fn exec_command(cmd: &mut Command) -> bool {
    let output = cmd.stderr(Stdio::null()).output();
    match output {
        Ok(out) => out.status.success(),
        _ => false,
    }
}

fn parse_json(path: &str) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let data: String = std::fs::read_to_string(path).unwrap_or(String::new());
    let value: Result<Value, serde_json::Error> = serde_json::from_str(&*data);
    match value {
        Ok(val) => {
            if let Some(arr) = val.as_array() {
                for val in arr {
                    result.push(val.as_str().unwrap().to_string())
                }
            }
        }
        Err(e) => {
            println!("Got an error! {}", e);
        }
    }

    result
}

async fn index_directory(dir: &str, github_repo: &str, log: Loading, base_url: &str) {
    let mut total = 0;
    let username = github_repo.split("/").next().unwrap();
    let branch = get_branch_name(dir);
    exec_command(Command::new("rm").arg("-rf").arg(format!("{}/.git", dir)));

    match github::get_user_id(username).await {
        Ok(user_id) => {
            let dirs = Walk::new(dir).into_iter().filter_map(|v| v.ok());

            for entry in dirs {
                if entry.path().is_file() {
                    log.text(format!("Indexing {}", entry.path().display()));
                    process_file(
                        &entry.path(),
                        github_repo,
                        &user_id,
                        &branch,
                        log.to_owned(),
                        base_url,
                    )
                    .await;
                    total += 1;
                }
            }

            if total == 0 {
                log.fail(format!("Folder '{}' not found!", github_repo));
            } else {
                log.success(format!(
                    "Done indexing '{}' total {} files!",
                    github_repo, total
                ));
            }
        }
        Err(e) => {
            log.fail(e);
        }
    }
}

async fn process_file(
    path: &Path,
    github_repo: &str,
    user_id: &str,
    branch: &str,
    log: Loading,
    base_url: &str,
) {
    match parser::read_file(path) {
        Ok((input, lang)) => {
            let html = parser::render_html(input, lang);
            let paths = path.to_str().unwrap().split("/").collect::<Vec<_>>();
            let file_path = paths[1..paths.len()].to_vec().join("/");
            let id = [github_repo, &paths[2..paths.len()].to_vec().join("/")].join("/");
            let data = solr::GithubFile {
                id: id.to_owned(),
                file_id: format!("g/{}/{}", github_repo, file_path.to_string()),
                owner_id: user_id.to_string(),
                path: paths[2..paths.len() - 1].to_vec().join("/"),
                repo: github_repo.to_string(),
                branch: branch.to_owned(),
                lang: lang.to_string(),
                content: Vec::new(),
            };
            store(data, &html, log, base_url).await;
        }
        Err(msg) => {
            log.warn(msg);
        }
    }
}

async fn store(mut data: solr::GithubFile, html: &str, log: Loading, base_url: &str) {
    let document = Document::from(html);
    let table = document.find(Class("highlight-table"));
    if let Some(el) = table.last() {
        let mut update = false;
        let mut index = 0;
        let mut max_index = 3;
        let max_chars = 2000;
        let mut child: String = String::new();
        for td in el.find(Name("tr")) {
            index += 1;
            child.push_str(&td.html());
            child.push('\n');
            if index == max_index && child.len() < max_chars {
                max_index += 1;
            }
            if index >= max_index {
                index = 0;
                max_index = 3;
                data.content = vec![];
                data.content.push(child.to_string());
                child = String::new();
                create_or_update(&mut update, &data, base_url, log.to_owned()).await;
            }
        }

        // If there any left content that less than 8 line then store it to DB!
        if index != 0 {
            data.content = vec![];
            data.content.push(child.to_string());
            create_or_update(&mut update, &data, base_url, log.to_owned()).await;
        }
    }
}

async fn create_or_update(update: &mut bool, data: &GithubFile, base_url: &str, log: Loading) {
    if *update == false {
        match solr::insert(&data, base_url).await {
            Ok(_) => {}
            Err(e) => log.warn(e.to_string()),
        }
        *update = true;
    } else {
        match solr::update(&data, base_url).await {
            Ok(_) => {}
            Err(e) => log.warn(e.to_string()),
        }
    }
}
