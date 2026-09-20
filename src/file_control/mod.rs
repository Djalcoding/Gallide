use std::{
    fs::File,
    path::{Path, PathBuf},
};


pub fn create_file(name: &str, path: &Path) -> std::io::Result<()> {
    let mut buffer: PathBuf = path.to_path_buf();
    buffer.push(name);
    let _ = File::create(buffer)?;
    Ok(())
}
