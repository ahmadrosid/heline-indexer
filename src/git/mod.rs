pub mod github;

use crate::utils;
use std::process::Command;

pub fn get_branch_name(dir: &str) -> String {
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


pub fn clone_repo(cwd: &str, repo_url: &str, repo_name: &str) -> bool {
    utils::exec_command(
        Command::new("git")
            .current_dir(cwd)
            .arg("clone")
            .arg(repo_url)
            .arg(repo_name),
    )
}