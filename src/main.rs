mod github;
mod solr;
mod loading;

use serde_json::Value;

use hl::lexers::*;
use select::document::Document;
use select::predicate::{Class, Name};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use walkdir::{DirEntry, WalkDir};
use crate::loading::Loading;

#[tokio::main]
pub async fn main() {
    let path = "sh.json";
    let value = parse_json(path);
    let mut index = 0;
    for val in value {
        let cwd = "repos";
        let git_url = val.to_string();
        let paths = git_url.split("/").collect::<Vec<_>>();
        let repo_name = paths.last().unwrap();
        let github_repo = format!("{}/{}", paths[paths.len() - 2], paths[paths.len() - 1]);
        // index_directory(&format!("{}/{}", cwd, repo_name), &github_repo).await;

        let success = exec_command(
            Command::new("git")
                .current_dir(cwd)
                .arg("clone")
                .arg(&val.to_string())
                .arg(repo_name),
        );
        if success {
            index_directory(&format!("{}/{}", cwd, repo_name), &github_repo).await;
            exec_command(
                Command::new("rm")
                    .current_dir(".")
                    .arg("-rf")
                    .arg(format!("{}/{}", cwd, repo_name)),
            );
        } else {
            println!("Failed to clone {}!", git_url);
        }
        index += 1;
        if index == 4 {
            std::process::exit(0);
        }
    }
}

#[track_caller]
pub fn exec_command(cmd: &mut Command) -> bool {
    let output = cmd
        .stderr(Stdio::inherit())
        .output()
        .expect("Failed to run command");
    output.status.success()
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

async fn index_directory(dir: &str, github_repo: &str) {
    let files = WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| ignore(e))
        .filter_map(|v| v.ok());
    let mut total = 0;
    let username = github_repo.split("/").next().unwrap();
    let mut loading = Loading::new();
    loading.start();
    let branch = get_branch_name(dir);
    match github::get_user_id(username).await {
        Ok(user_id) => {
            for entry in files {
                if entry.path().is_file() {
                    process_file(&entry.path(), github_repo, &user_id, &branch).await;
                    total += 1;
                    loading.text(format!("Indexing {}", entry.path().display()));
                }
            }
            loading.success(format!("Done indexing '{}' total {} files!", github_repo, total));
        }
        Err(e) => {
            loading.fail(e);
        },
    }
    loading.end();
}

fn ignore(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| !s.starts_with(".git"))
        .unwrap_or(false)
}

async fn process_file(path: &Path, github_repo: &str, user_id: &str, branch: &str) {
    match read_file(path) {
        Ok((input, lang)) => {
            let html = render_html(input, lang);
            let paths = path.to_str().unwrap().split("/").collect::<Vec<_>>();
            let file_path = paths[1..paths.len()].to_vec().join("/");
            let data = solr::GithubFile {
                id: file_path.to_string(),
                file_id: format!("g/{}/{}", github_repo, file_path.to_string()),
                owner_id: user_id.to_string(),
                path: path.to_str().unwrap().to_string(),
                repo: github_repo.to_string(),
                branch: branch.to_owned(),
                lang: lang.to_string(),
                content: vec![],
            };
            store(data, &html).await;
        }
        Err(msg) => {
            println!("{}", msg);
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

fn read_file(file_path: &Path) -> core::result::Result<(Vec<char>, &str), String> {
    let path = file_path.to_str().unwrap_or("");
    if let Ok(source) = fs::read_to_string(path) {
        let input = source.chars().collect();
        let lang = match file_path.extension() {
            Some(ext) => match ext.to_str().unwrap_or("raw") {
                "rs" => "rust",
                "sh" | "zsh" => "bash",
                "js" => "javascript",
                "go" => "Go",
                "ts" | "tsx" => "typescript",
                "c" => "c",
                "cpp" => "cpp",
                "html" => "html",
                "java" => "java",
                "lua" => "lua",
                "md" => "Markdown",
                "py" => "python",
                "cs" => "c#",
                "yml" | "yaml" => "yml",
                _ => "raw"
            },
            _ => "raw",
        };
        Ok((input, lang))
    } else {
        Err(format!("Failed to read file path {}", path))
    }
}

async fn store(mut data: solr::GithubFile, html: &str) {
    let document = Document::from(html);
    let table = document.find(Class("highlight-table"));
    if let Some(el) = table.last() {
        let mut index = 0;
        let mut child: String = String::new();
        for td in el.find(Name("tr")) {
            index += 1;
            child.push_str(&td.html());
            child.push('\n');
            // Store as array with length of 5!
            if index >= 8 {
                index = 0;
                data.content = vec![];
                data.content.push(child.to_owned());
                match solr::insert(&data).await {
                    Ok(_) => {},
                    Err(e) => println!("{}", e),
                }
            }
        }

        // If there any left content that less than 5 line then store it to DB!
        if index != 0 {
            data.content = vec![];
            data.content.push(child.to_owned());
            match solr::insert(&data).await {
                Ok(_) => {},
                Err(e) => println!("{}", e),
            }
        }

    }
}

fn render_html(input: Vec<char>, lang: &str) -> String {
    return match lang {
        "bash" => bash::render::render_html(input),
        "c" => c::render::render_html(input),
        "clojure" | "clj" => clojure::render::render_html(input),
        "css" => css::render::render_html(input),
        "cuda" => cuda::render::render_html(input),
        "edn" => edn::render::render_html(input),
        "Go" => go::render::render_html(input),
        "hs" | "haskell" => haskell::render::render_html(input),
        "html" => html::render::render_html(input),
        "rust" => rust::render::render_html(input),
        "cpp" => cpp::render::render_html(input),
        "cs" | "c#" => cs::render::render_html(input),
        "java" => java::render::render_html(input),
        "js" | "javascript" => javascript::render::render_html(input),
        "json" => json::render::render_html(input),
        "lua" => lua::render::render_html(input),
        "Markdown" => markdown::render::render_html(input),
        "php" => php::render::render_html(input),
        "python" => python::render::render_html(input),
        "ts" | "typescript" => typescript::render::render_html(input),
        "yaml" | "yml" => yaml::render::render_html(input),
        _ => {
            let mark_bash = String::from("#!/bin/bash");
            if input.len() > mark_bash.len() {
                let result: String = input[0..mark_bash.len()].iter().collect();
                if result == mark_bash {
                    return bash::render::render_html(input);
                } else if result == "#!/usr/bin/env zsh" {
                    return bash::render::render_html(input);
                }
            }

            raw::render::render_html(input)
        }
    };
}
