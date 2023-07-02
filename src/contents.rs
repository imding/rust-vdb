use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Ok, Result};

use crate::error::NotAvailableError;

enum FileState {
    None,
    CodeBlock,
    Sentence,
    Meta,
}

pub struct File {
    pub path: String,
    pub contents: String,
    pub sentences: Vec<String>,
}

impl File {
    pub fn new(path: String, contents: String) -> Self {
        Self {
            path,
            contents,
            sentences: Vec::new(),
        }
    }

    pub fn parse(&mut self) {
        let mut contents = Vec::new();
        let mut state = FileState::None;
        let mut sentence = String::new();

        // TODO: ensure self.contents ends with 2 empty lines
        for line in self.contents.lines() {
            match state {
                FileState::None => {
                    if line.starts_with("```") {
                        state = FileState::CodeBlock;
                        sentence = String::new();
                        sentence.push_str(line);
                        sentence.push('\n');
                    } else if line.starts_with("---") {
                        state = FileState::Meta;
                    } else if !line.starts_with('#') && !line.is_empty() {
                        sentence = String::new();
                        sentence.push_str(line);
                        sentence.push('\n');
                        state = FileState::Sentence;
                    }
                }

                FileState::CodeBlock => {
                    sentence.push_str(line);

                    if line.starts_with("```") {
                        contents.push(sentence);
                        sentence = String::new();
                        state = FileState::None;
                    }
                }

                FileState::Meta => {
                    if line.starts_with("---") {
                        state = FileState::None;
                    }
                }

                FileState::Sentence => {
                    if line.is_empty() {
                        contents.push(sentence);
                        sentence = String::new();
                        state = FileState::None;
                    } else {
                        sentence.push_str(line);
                        sentence.push('\n');
                    }
                }
            }
        }

        self.sentences = contents;
    }
}

fn has_file_extension(path: &PathBuf, ending: &str) -> bool {
    if let Some(path) = path.to_str() {
        return path.ends_with(ending);
    }

    false
}

pub fn load_files_from_dir(dir: PathBuf, prefix: &PathBuf, ending: &str) -> Result<Vec<File>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let path = entry?.path();

        if path.is_dir() {
            let mut sub_files = load_files_from_dir(path, &prefix, ending)?;
            files.append(&mut sub_files);
        } else if path.is_file() && has_file_extension(&path, ending) {
            let path = Path::new(&path).strip_prefix(prefix)?.to_owned();

            println!("Path: {:?}", path);

            let contents = fs::read_to_string(&path)?;
            let key = path.to_str().ok_or(NotAvailableError {})?;
            let mut file = File::new(key.to_string(), contents);

            file.parse();
            files.push(file);
        }
    }

    Ok(files)
}
