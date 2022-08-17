use std::env;
use std::path::{Path, PathBuf};

pub struct Arg {
    pub index_file: PathBuf,
    pub folder: PathBuf,
    pub solr_url: String,
    pub is_index_folder: bool,
}

impl Arg {
    pub fn new() -> Self {
        Self {
            index_file: PathBuf::new(),
            folder: PathBuf::new(),
            solr_url: String::new(),
            is_index_folder: false,
        }
    }

    pub fn parse(mut self) -> Result<Self, String> {
        self.solr_url = match env::var("BASE_URL") {
            Ok(val) => val,
            Err(_) => "http://localhost:8984".to_string(),
        };

        let file = match env::args().nth(1) {
            Some(file) => file,
            None => {
                return Err(format!("Index file path is required!"));
            }
        };

        if !Path::new(&file).exists() {
            return Err(format!(
                "Please input the correct index file! Current file is not exists: {}",
                &file
            ));
        }

        let option = match std::env::args().nth(2) {
            Some(option) => option,
            None => String::new(),
        };

        self.is_index_folder = option == "--folder";
        let mut repo_folder = PathBuf::new();
        match env::args().nth(3) {
            Some(folder) => repo_folder.push(&folder),
            None => repo_folder.push("repos"),
        };

        self.folder = repo_folder;

        Ok(self)
    }
}
