mod github;
mod parser;
mod solr;

use serde_json::Value;

use loading::Loading;
use select::document::Document;
use select::predicate::{Class, Name};
use std::path::Path;
use std::process::{Command, Stdio};
use walkdir::WalkDir;

#[tokio::main]
pub async fn main() {
    let mut log = Loading::new();
    log.start();
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        log.fail("Please provide path!");
        log.end();
        std::process::exit(1);
    }

    let value = parse_json(&args[1]);
    let mut index = 0;
    for val in value {
        if index == 1 {
            break;
        }
        index += 1;
        let cwd = "repos";
        let git_url = val.to_string();
        let paths: Vec<&str> = git_url.split("/").collect();
        let repo_name = paths.last().unwrap();
        let github_repo = format!("{}/{}", paths[paths.len() - 2], paths[paths.len() - 1]);

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
            index_directory(dir, &github_repo, log.to_owned()).await;
            exec_command(
                Command::new("rm")
                    .current_dir(".")
                    .arg("-rf")
                    .arg(format!("{}/{}", cwd, repo_name)),
            );
        } else {
            log.fail(format!("Failed to clone '{}'!", git_url));
        }
    }
    log.end();
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
    let data: String = std::fs::read_to_string(path).unwrap_or("".to_string());
    let value: Result<Value, serde_json::Error> = serde_json::from_str(&*data);
    match value {
        Ok(val) => {
            if let Some(arr) = val.as_array() {
                for val in arr {
                    result.push(val.as_str().unwrap().to_string())
                }
            }
        }
        Err(_) => {}
    }

    result
}

async fn index_directory(dir: &str, github_repo: &str, log: Loading) {
    let mut total = 0;
    let username = github_repo.split("/").next().unwrap();
    let branch = get_branch_name(dir);
    exec_command(Command::new("rm").arg("-rf").arg(format!("{}/.git", dir)));

    match github::get_user_id(username).await {
        Ok(user_id) => {
            let dirs = WalkDir::new(dir).into_iter().filter_map(|v| v.ok());

            for entry in dirs {
                if entry.path().is_file() {
                    log.text(format!("Indexing {}", entry.path().display()));
                    process_file(
                        &entry.path(),
                        github_repo,
                        &user_id,
                        &branch,
                        log.to_owned(),
                    )
                    .await;
                    total += 1;
                }
            }

            log.success(format!(
                "Done indexing '{}' total {} files!",
                github_repo, total
            ));
        }
        Err(e) => {
            log.fail(e);
        }
    }
}

async fn process_file(path: &Path, github_repo: &str, user_id: &str, branch: &str, log: Loading) {
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
                path: id.to_string(),
                repo: github_repo.to_string(),
                branch: branch.to_owned(),
                lang: lang.to_string(),
                content: vec![],
            };
            store(data, &html, log).await;
        }
        Err(msg) => {
            log.warn(msg);
        }
    }
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

async fn store(mut data: solr::GithubFile, html: &str, log: Loading) {
    let document = Document::from(html);
    let table = document.find(Class("highlight-table"));
    if let Some(el) = table.last() {
        let mut index = 0;
        let mut max_index = 3;
        let max_chars = 2500;
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
                data.content.push(child.to_owned());
                child = String::new();
                match solr::insert(&data).await {
                    Ok(_) => {}
                    Err(e) => log.warn(e.to_string()),
                }
            }
        }

        // If there any left content that less than 8 line then store it to DB!
        if index != 0 {
            data.content = vec![];
            data.content.push(child.to_owned());
            match solr::insert(&data).await {
                Ok(_) => {}
                Err(e) => log.warn(e.to_string()),
            }
        }
    }
}
