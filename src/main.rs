mod git;
mod indexer;
mod utils;
mod parser;
mod solr;

use std::path::Path;
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

    let mut index = 0;
    let max_index = 100;
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
            "--folder" => {
                indexer::index_directory(&dir, &github_repo, &base_url).await
            },
            _ => {
                match git::github::get_repo(&github_repo).await {
                    Ok(_) => {}
                    Err(e) => {
                        print!("{}\n", format!("{}: Error {}", github_repo, e));
                        continue;
                    }
                }

                print!("{}\n", format!("Cloning '{}'", val.to_string()));
                let success = git::clone_repo(cwd, &val, &repo_name);

                if success {
                    let dir = &format!("{}/{}", cwd, repo_name);
                    indexer::index_directory(dir, &github_repo, &base_url).await;
                    utils::delete_dir(&format!("{}/{}", cwd, repo_name));
                } else {
                    print!("{}\n", format!("Failed to clone: {}", git_url));
                }
            }
        }
    }
}
