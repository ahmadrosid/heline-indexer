mod lexers;
mod solr;

use crate::lexers::*;
use select::document::Document;
use select::predicate::{Class, Name};
use std::fs;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

#[tokio::main]
pub async fn main() {
    let dir = String::from(".");

    let mut ignore_list: Vec<String> = Vec::new();
    ignore_list.push(".git".to_string());
    if let Ok(content) = std::fs::read_to_string(format!("{}/.gitignore", dir)) {
        let mut lines = content.clone();
        for line in lines.lines() {
            ignore_list.push(line.trim_start_matches("/").to_string());
        }
    };

    WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| !ignore(e, ignore_list.clone()))
        .filter_map(|v| v.ok())
        .for_each(|entry| match entry {
            Ok(entry) => {
                if entry.path().is_file() {
                    process_file(&entry.path()).await;
                }
            }
            Err(e) => {
                println!("{}", e);
            }
        });
}

fn ignore(entry: &DirEntry, ignore_list: Vec<String>) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| {
            for ignore in ignore_list {
                if s.starts_with(&ignore) {
                    return true;
                }
            }
            false
        })
        .unwrap_or(false)
}

async fn process_file(path: &Path) {
    match read_file(path) {
        Ok((input, lang)) => {
            let html = render_html(input, lang);
            extract(&html, lang, path.to_str().unwrap()).await;
        }
        Err(msg) => {
            println!("{}", msg);
        }
    }
}

fn read_file(file_path: &Path) -> Result<(Vec<char>, &str), String> {
    let path = file_path.to_str().unwrap_or("");
    if let Ok(source) = fs::read(path) {
        let input: Vec<char> = source.iter().map(|c| *c as char).collect();
        let lang = match file_path.extension() {
            Some(ext) => match ext.to_str().unwrap_or("raw") {
                "rs" => "rust",
                "sh" => "bash",
                "js" => "javascript",
                "go" => "go",
                "ts" | "tsx" => "typescript",
                "c" => "c",
                "cpp" => "cpp",
                "py" => "python",
                "java" => "java",
                "lua" => "lua",
                "cs" => "c#",
                "yml" | "yaml" => "yml",
                _ => "raw",
            },
            _ => "raw",
        };
        Ok((input, lang))
    } else {
        Err(format!("Failed to read file path {}", path))
    }
}

async fn extract(content: &str, lang: &str, path: &str) {
    let document = Document::from(content);
    let table = document.find(Class("highlight-table"));
    if let Some(el) = table.last() {
        let mut index = 0;
        let mut child: Vec<String> = Vec::new();
        let mut data = solr::GithubFile {
            id: path.to_string(),
            file_id: path.to_string(),
            owner_id: "123".to_string(),
            path: path.to_string(),
            repo: "ahmadrosid/heline-indexer".to_string(),
            branch: "main".to_string(),
            lang: lang.to_string(),
            content: vec![],
        };
        for td in el.find(Name("tr")) {
            index += 1;
            child.push(td.html());
            if index == 5 {
                index = 0;
                data.content = child.clone();
                let result = solr::insert(&data).await;
                match result {
                    Ok(_) => print!("."),
                    Err(e) => println!("{}", e),
                }
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
        "go" => go::render::render_html(input),
        "hs" | "haskell" => haskell::render::render_html(input),
        "html" => html::render::render_html(input),
        "rust" => rust::render::render_html(input),
        "cpp" => cpp::render::render_html(input),
        "cs" | "c#" => cs::render::render_html(input),
        "java" => java::render::render_html(input),
        "js" | "javascript" => javascript::render::render_html(input),
        "json" => json::render::render_html(input),
        "lua" => lua::render::render_html(input),
        "php" => php::render::render_html(input),
        "python" => python::render::render_html(input),
        "ts" | "typescript" => typescript::render::render_html(input),
        "yaml" | "yml" => yaml::render::render_html(input),
        _ => raw::render::render_html(input),
    };
}
