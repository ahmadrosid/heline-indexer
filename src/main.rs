mod lexers;

use crate::lexers::*;
use select::document::Document;
use select::predicate::{Class, Name};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub fn main() {
    for entry in WalkDir::new("src") {
        match entry {
            Ok(entry) => {
                if entry.path().is_file() {
                    process_file(&entry.path());
                }
            }
            Err(e) => {
                println!("Failed parse the path!");
            }
        }
    }
}

fn process_file(path: &Path) {
    match parse_file(path) {
        Ok((input, lang)) => {
            let html = render_html(input, lang);
            extract(&html);
        }
        Err(msg) => {
            println!("{}", msg);
        }
    }
}

fn parse_file(file_path: &Path) -> Result<(Vec<char>, &str), String> {
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

fn extract(content: &str) {
    let document = Document::from(content);
    let table = document.find(Class("highlight-table"));
    let mut result: Vec<Vec<String>> = Vec::new();
    if let Some(el) = table.last() {
        let mut index = 0;
        let mut child: Vec<String> = Vec::new();
        for td in el.find(Name("tr")) {
            index += 1;
            child.push(td.html());
            println!("Yes we got the html > table > tbody > td!");
            if index == 5 {
                result.push(child.to_vec());
                index = 0;
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
