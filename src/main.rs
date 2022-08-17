mod arg;
mod git;
mod indexer;
mod parser;
mod solr;
mod utils;

use arg::Arg;
use std::path::PathBuf;

#[tokio::main]
pub async fn main() {
    let mut arg = Arg::new();
    match arg.parse() {
        Ok(new_arg) => arg = new_arg,
        Err(msg) => {
            eprintln!("{}", msg);
            std::process::exit(1);
        }
    }

    let value: Vec<String> = utils::parse_json(&arg.index_file);
    for git_url in value {
        let mut repository_directory = PathBuf::new();
        repository_directory.push("repos");

        if arg.is_index_folder {
            println!("git_url: {}", git_url);
            let git_host = utils::get_url_host(&git_url).unwrap_or("github.com".to_string());
            indexer::index_directory(&repository_directory, &git_url, &arg.solr_url, &git_host)
                .await;
            continue;
        }

        match git::get_repo(&git_url).await {
            Ok(_repo_id) => indexer::process(&repository_directory, &git_url, &arg.solr_url).await,
            Err(e) => {
                print!("{}: Error {}\n", git_url, e);
                continue;
            }
        };
    }
}
