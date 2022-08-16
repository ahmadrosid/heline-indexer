use crate::solr;
use crate::parser;
use crate::git;
use crate::utils;
use crate::solr::client::GitFile;

use ignore::Walk;
use std::path::{Path, PathBuf};
use select::document::Document;
use select::predicate::{Class, Name};

pub async fn process(repo_dir: &PathBuf, git_url: &str, base_url: &str) {
    let repo_name = utils::get_repo_name(git_url);
    let git_repo = utils::get_git_repo_path(git_url);
    let ssh_url = utils::get_git_ssh_url(git_url);
    let success = git::clone_repo(repo_dir, &ssh_url, &repo_name);

    if success {
        index_directory(repo_dir, &git_repo, &base_url).await;
        utils::delete_dir(&repo_dir.join(Path::new(&git_repo)));
    } else {
        print!("{}\n", format!("Failed to clone: {}", ssh_url));
    }
}

pub async fn index_directory(dir: &Path, git_url: &str, base_url: &str) {
    let mut total = 0;
    let git_repo = utils::get_git_repo_path(git_url);
    let username = git_repo.split("/").next().unwrap();
    let branch = git::get_branch_name(dir);

    match git::github::get_user_id(username).await {
        Ok(user_id) => {
            let dirs = Walk::new(dir).into_iter().filter_map(|v| v.ok());

            for entry in dirs {
                if entry.path().is_file() {
                    print!("{}\n", format!("Indexing {}", entry.path().display()));
                    process_file(
                        &entry.path(),
                        &git_repo,
                        &user_id,
                        &branch,
                        base_url,
                    )
                    .await;
                    total += 1;
                }
            }

            if total == 0 {
                print!("{}\n", format!("Folder '{}' not found!", git_repo));
            } else {
                print!("{}", format!(
                    "Done indexing '{}' total {} files!",
                    git_repo, total
                ));
            }
        }
        Err(e) => {
            print!("{}", e);
        }
    }
}


async fn process_file(
    path: &Path,
    github_repo: &str,
    user_id: &str,
    branch: &str,
    base_url: &str,
) {
    match parser::read_file(path) {
        Ok((input, lang)) => {
            let html = parser::render_html(input, lang);
            let paths = path.to_str().unwrap().split("/").collect::<Vec<_>>();
            let file_path = paths[1..paths.len()].to_vec().join("/");
            let id = [github_repo, &paths[2..paths.len()].to_vec().join("/")].join("/");
            let data = GitFile {
                id: id.to_owned(),
                file_id: format!("g/{}/{}", github_repo, file_path.to_string()),
                owner_id: user_id.to_string(),
                path: paths[2..paths.len() - 1].to_vec().join("/"),
                repo: github_repo.to_string(),
                branch: branch.to_owned(),
                lang: lang.to_string(),
                content: Vec::new(),
            };
            store(data, &html, base_url).await;
        }
        Err(msg) => {
            print!("{}\n", msg);
        }
    }
}

async fn store(mut data: GitFile, html: &str, base_url: &str) {
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
                create_or_update(&mut update, &data, base_url).await;
            }
        }

        // If there any left content that less than `max_index` line then store it to DB!
        if index != 0 {
            data.content = vec![];
            data.content.push(child.to_string());
            create_or_update(&mut update, &data, base_url).await;
        }
    }
}

async fn create_or_update(update: &mut bool, data: &GitFile, base_url: &str) {
    if *update == false {
        match solr::client::insert(&data, base_url).await {
            Ok(_) => {}
            Err(e) => print!("{}\n", e.to_string()),
        }
        *update = true;
    } else {
        match solr::client::update(&data, base_url).await {
            Ok(_) => {}
            Err(e) => print!("{}\n", e.to_string()),
        }
    }
}
