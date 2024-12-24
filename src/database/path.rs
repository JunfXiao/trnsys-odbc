use std::{fs, io};

pub fn clean_and_ensure_path(file_path: &str) -> Result<String, io::Error> {
    let file_path = std::path::absolute(file_path)?;
    fs::create_dir_all(file_path.parent().unwrap())?;

    Ok(file_path.to_str().expect("Empty Path").to_string())
}
