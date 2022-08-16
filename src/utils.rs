use std::path::PathBuf;
use std::process::{Command, Stdio};
use serde_json::Value;
use reqwest::Url;

fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}

pub fn exec_command(cmd: &mut Command) -> bool {
    let output = cmd.stderr(Stdio::null()).output();
    match output {
        Ok(out) => out.status.success(),
        _ => false,
    }
}

pub fn parse_json(path: &str) -> Vec<String> {
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

pub fn delete_dir(dir_path: &PathBuf) {
    match std::fs::remove_dir_all(dir_path) {
        Ok(_) => {}
        Err(err) => println!("Failed to delete dir, {}", err)
    }
}

pub fn get_url_host(url: &str) -> Option<String> {
    match Url::parse(url) {
        Ok(val) => Some(val.domain()?.to_string()),
        Err(_) => None
    }
}

pub fn get_repo_name(git_url: &str) -> String {
    let repo_path = get_git_repo_path(git_url);
    let paths: Vec<&str> = repo_path.split("/").collect();
    let repo_name = paths.last().unwrap();
    repo_name.to_string()
}

pub fn get_git_repo_path(git_url: &str) -> String {
    let paths: Vec<&str> = git_url.split("/").collect();
    let path = format!("{}/{}", paths[paths.len() - 2], paths[paths.len() - 1]);
    if path.ends_with(".git") {
        return truncate(&path, path.len() - 4).to_string();
    }
    path
}

pub fn get_git_ssh_url(git_url: &str) -> String {
    let git_host = get_url_host(git_url).unwrap_or(String::new());
    let repo_path = get_git_repo_path(git_url);
    format!("git@{}:{}.git", git_host, repo_path)
}
