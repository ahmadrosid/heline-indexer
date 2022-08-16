mod git;
mod indexer;
mod utils;
mod parser;
mod solr;

use std::path::{Path, PathBuf};
use reqwest::Url;

#[tokio::main]
pub async fn main() {
    let mut base_url = String::new();
    match std::env::var("BASE_URL") {
        Ok(val) => base_url.push_str(&val),
        Err(e) => {
            print!("{}\n",format!("BASE_URL: {}!", e));
            std::process::exit(1);
        }
    }

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

    let mut value: Vec<String> = vec![];

    match Url::parse(&file) {
        Ok(_) => value.push(file.to_string()),
        Err(_) => {
            if !Path::new(&file).exists() {
                let cwd = std::env::current_dir().unwrap();
                println!("{}", format!(
                    "File not exists: {}/{}",
                    cwd.as_path().display(),
                    file
                ));
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

        match &option[..] {
            "--folder" => {
                let git_host = utils::get_url_host(&git_url).unwrap_or("github.com".to_string());
                indexer::index_directory(&repository_directory, &git_url, &base_url, &git_host).await
            },
            _ => {
                match git::get_repo(&git_url).await {
                    Ok(_repo_id) => indexer::process(&repository_directory, &git_url, &base_url).await,
                    Err(e) => {
                        print!("{}: Error {}\n", git_url, e);
                        continue;
                    }
                };
            }
        }
    }
}
