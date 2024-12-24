use path_clean::PathClean;
use std::{fs, io};

pub fn clean_and_ensure_path(file_path: &str) -> Result<String, io::Error> {
    let mut file_path = std::path::absolute(file_path)?;
    file_path.clean();
    let dir = file_path.parent().expect("Cannot find path's parent");
    fs::create_dir_all(&dir)?;

    Ok(file_path.to_str().expect("Empty Path").to_string())
}
