mod arg;
mod git;
mod indexer;
mod parser;
mod solr;
mod utils;

use arg::Arg;

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
        match git::get_repo(&git_url).await {
            Ok(_repo_id) => {
                indexer::process(&arg.folder, &git_url, &arg.solr_url, arg.with_delete_folder).await
            }
            Err(e) => {
                print!("{}: Error {}\n", git_url, e);
                continue;
            }
        };
    }
}
