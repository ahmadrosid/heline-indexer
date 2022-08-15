use std::process::{Command, Stdio};
use serde_json::Value;

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
