mod git;
mod indexer;
mod parser;
mod solr;
mod utils;

use reqwest::Url;
use std::path::{Path, PathBuf};

#[tokio::main]
pub async fn main() {
    let base_url = match std::env::var("BASE_URL") {
        Ok(val) => val,
        Err(_) => "http://localhost:8984".to_string(),
    };

    let file = match std::env::args().nth(1) {
        Some(file) => file,
        None => {
            println!("Please provide path!");
            std::process::exit(1);
        }
    };

    let option = match std::env::args().nth(2) {
        Some(option) => option,
        None => String::new(),
    };

    let is_index_folder = option == "--folder";
    let mut repo_folder = String::new();
    match std::env::args().nth(3) {
        Some(folder) => repo_folder.push_str(&folder),
        None => repo_folder.push_str("repos"),
    }

    let mut value: Vec<String> = vec![];

    match Url::parse(&file) {
        Ok(_) => value.push(file.to_string()),
        Err(_) => {
            if !Path::new(&file).exists() {
                let cwd = std::env::current_dir().unwrap();
                println!(
                    "{}",
                    format!("File not exists: {}/{}", cwd.as_path().display(), file)
                );
                std::process::exit(1);
            }
            value = utils::parse_json(&file)
        }
    }

    let max_index = 100;
    for (index, git_url) in value.into_iter().enumerate() {
        if index == max_index {
            break;
        }

        let mut repository_directory = PathBuf::new();
        repository_directory.push("repos");

        if is_index_folder {
            println!("git_url: {}", git_url);
            let git_host = utils::get_url_host(&git_url).unwrap_or("github.com".to_string());
            indexer::index_directory(&repository_directory, &git_url, &base_url, &git_host).await;
            continue;
        }

        match git::get_repo(&git_url).await {
            Ok(_repo_id) => indexer::process(&repository_directory, &git_url, &base_url).await,
            Err(e) => {
                print!("{}: Error {}\n", git_url, e);
                continue;
            }
        };
    }
}
