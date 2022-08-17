use crate::git;
use crate::parser;
use crate::solr;
use crate::solr::client::GitFile;
use crate::utils;

use ignore::Walk;
use select::document::Document;
use select::predicate::{Class, Name};
use std::path::{Path, PathBuf};

pub struct MetaIndexFile {
    path: PathBuf,
    root_path_len: usize,
    git_repo: String,
    user_id: String,
    branch: String,
    base_url: String,
    git_host: String,
}

pub async fn process(repo_dir: &PathBuf, git_url: &str, base_url: &str, with_delete_dir: bool) {
    let git_host = utils::get_url_host(git_url).unwrap_or("github.com".to_string());
    let repo_name = utils::get_repo_name(git_url);
    let git_repo = utils::get_git_repo_path(git_url);
    let ssh_url = utils::get_git_ssh_url(git_url);
    let success = git::clone_repo(repo_dir, &ssh_url, &repo_name);

    if success {
        index_directory(repo_dir, &git_repo, &base_url, &git_host).await;
        if with_delete_dir {
            utils::delete_dir(&repo_dir.join(Path::new(&repo_name)));
        }
    } else {
        print!("{}\n", format!("Failed to clone: {}", ssh_url));
    }
}

pub async fn index_directory(repo_index_dir: &Path, git_url: &str, base_url: &str, git_host: &str) {
    println!("Start indexing on folder: {}", repo_index_dir.display());

    let mut total = 0;
    let git_repo = utils::get_git_repo_path(git_url);
    let username = git_repo.split("/").next().unwrap();
    let branch = git::get_branch_name(repo_index_dir);

    let user_id = match git_host {
        "gitlab.com" => String::from("0000"),
        _ => match git::github::get_user_id(username).await {
            Ok(user_id) => user_id,
            Err(e) => {
                print!("{}", e);
                String::from("00000")
            }
        },
    };

    let dirs = Walk::new(repo_index_dir).into_iter().filter_map(|v| v.ok());
    let root_path_len = repo_index_dir
        .to_str()
        .unwrap()
        .split("/")
        .collect::<Vec<_>>()
        .len();

    for entry in dirs {
        if !entry.path().is_file() {
            continue;
        }

        print!("{}\n", format!("Indexing {}", entry.path().display()));
        let meta = MetaIndexFile {
            path: PathBuf::from(entry.path()),
            root_path_len: root_path_len,
            git_repo: git_repo.to_string(),
            user_id: user_id.to_string(),
            branch: branch.to_string(),
            base_url: base_url.to_string(),
            git_host: git_host.to_string(),
        };
        process_file(meta).await;
        total += 1;
    }

    if total == 0 {
        print!("{}\n", format!("Folder '{}' not found!", git_repo));
    } else {
        print!(
            "{}\n",
            format!("Done indexing '{}' total {} files!", git_repo, total)
        );
    }
}

async fn process_file(meta: MetaIndexFile) {
    match parser::read_file(&meta.path) {
        Ok((input, lang)) => {
            let html = parser::render_html(input, lang);
            let paths = meta.path.to_str().unwrap().split("/").collect::<Vec<_>>();
            let file_path = paths[meta.root_path_len..paths.len()].to_vec().join("/");
            let id = [
                meta.git_repo.to_string(),
                paths[2..paths.len()].to_vec().join("/"),
            ]
            .join("/");
            let data = GitFile {
                id: id.to_owned(),
                file_id: format!(
                    "{}/{}/{}",
                    &meta.git_host,
                    &meta.git_repo,
                    file_path.to_string()
                ),
                owner_id: meta.user_id.to_string(),
                path: paths[meta.root_path_len..paths.len() - 2]
                    .to_vec()
                    .join("/"),
                repo: meta.git_repo.to_string(),
                branch: meta.branch.to_owned(),
                lang: lang.to_string(),
                content: Vec::new(),
            };
            store(data, &html, &meta.base_url).await;
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
