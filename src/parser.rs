use hl::lexers::*;
use std::fs;
use std::path::Path;

pub fn read_file(file_path: &Path) -> core::result::Result<(Vec<char>, &str), String> {
    let path = file_path.to_str().unwrap_or("");
    if let Ok(source) = fs::read_to_string(path) {
        let input = source.chars().collect();
        let lang = match file_path.extension() {
            Some(ext) => match ext.to_str().unwrap_or("Raw") {
                "sh" | "zsh" | "bash" => "Shell",
                "js" => "JavaScript",
                "go" => "Go",
                "groovy" => "Groovy",
                "ts" | "tsx" => "TypeScript",
                "c" | "h" => "C",
                "cpp" | "c++" => "C++",
                "html" => "HTML",
                "hs" => "Haskell",
                "java" => "Java",
                "lua" => "Lua",
                "md" => "Markdown",
                "py" => "Python",
                "php" => "PHP",
                "rs" => "Rust",
                "toml" => "TOML",
                "cs" => "C#",
                "yml" | "yaml" => "YAML",
                "lock" => {
                    if file_path.file_name().unwrap().to_str().unwrap() == "Cargo.lock" {
                        "TOML"
                    } else {
                        "lock"
                    }
                }
                _ => {
                    ext.to_str().unwrap()
                },
            },
            _ => parse_file_name(file_path.file_name().unwrap().to_str().unwrap()),
        };
        Ok((input, lang))
    } else {
        Err(format!("Failed to read file path {}", path))
    }
}

fn parse_file_name(file: &str) -> &str {
    return match file {
        "Jenkinsfile" => "Groovy",
        "Dockerfile" => "Dockerfile",
        "Makefile" => "Makefile",
        _ => "Raw"
    };
}

pub fn render_html(input: Vec<char>, lang: &str) -> String {
    return match lang {
        "Shell" => bash::render::render_html(input),
        "C" => c::render::render_html(input),
        "C++" => cpp::render::render_html(input),
        "Clojure" => clojure::render::render_html(input),
        "CSS" => css::render::render_html(input),
        "CUDA" => cuda::render::render_html(input),
        "edn" => edn::render::render_html(input),
        "Go" => go::render::render_html(input),
        "Groovy" => groovy::render::render_html(input),
        "Haskell" => haskell::render::render_html(input),
        "HTML" => html::render::render_html(input),
        "Rust" => rust::render::render_html(input),
        "C#" => cs::render::render_html(input),
        "Java" => java::render::render_html(input),
        "JavaScript" => javascript::render::render_html(input),
        "JSON" => json::render::render_html(input),
        "Lua" => lua::render::render_html(input),
        "Makefile" => makefile::render::render_html(input),
        "Markdown" => markdown::render::render_html(input),
        "PHP" => php::render::render_html(input),
        "Python" => python::render::render_html(input),
        "TOML" => toml::render::render_html(input),
        "TypeScript" => typescript::render::render_html(input),
        "YAML" => yaml::render::render_html(input),
        _ => {
            let mark_bash = String::from("#!/bin/sh");
            if input.len() > mark_bash.len() {
                let result: String = input[0..mark_bash.len()].iter().collect();
                if result == mark_bash {
                    return bash::render::render_html(input);
                }
            }

            let mark_bash = String::from("#!/bin/bash");
            if input.len() > mark_bash.len() {
                let result: String = input[0..mark_bash.len()].iter().collect();
                if result == mark_bash{
                    return bash::render::render_html(input);
                }
            }

            let mark_bash = String::from("#!/usr/bin/env bash");
            if input.len() > mark_bash.len() {
                let result: String = input[0..mark_bash.len()].iter().collect();
                if result == mark_bash{
                    return bash::render::render_html(input);
                }
            }

            let mark_bash = String::from("#!/usr/bin/env sh");
            if input.len() > mark_bash.len() {
                let result: String = input[0..mark_bash.len()].iter().collect();
                if result == mark_bash{
                    return bash::render::render_html(input);
                }
            }

            let mark_bash = String::from("#!/usr/bin/env zsh");
            if input.len() > mark_bash.len() {
                let result: String = input[0..mark_bash.len()].iter().collect();
                if result == mark_bash{
                    return bash::render::render_html(input);
                }
            }

            raw::render::render_html(input)
        }
    };
}
