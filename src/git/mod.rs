pub mod github;


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
