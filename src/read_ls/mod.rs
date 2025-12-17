use std::{
    fs,
    io::Error,
    path::{Path, PathBuf},
    process::Command,
};

pub enum Item {
    File,
    Folder,
}

pub struct Entry {
    path: PathBuf,
    name: String,
    pub entry_type: Item,
}

impl Entry {
    pub fn new(path: PathBuf, name: String, entry_type: Item) -> Self {
        Entry {
            path,
            name,
            entry_type,
        }
    }
    pub fn set_type(&mut self) {}
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
    pub fn name(&self) -> &String {
        &self.name
    }
}

fn get_folders(current_folder: &str) -> Result<String, Error> {
    let mut ls_command = Command::new("sh");
    ls_command
        .arg("-c")
        .arg(format!("ls -d {current_folder}/*/"));
    let output = &ls_command.output()?.stdout;
    Ok(String::from_utf8(output.to_vec()).expect("unknown folder"))
}

fn get_files(current_folder: &str) -> Result<String, Error> {
    let mut find_command = Command::new("sh");
    find_command
        .arg("-c")
        .arg(format!("find {current_folder} -maxdepth 1 -type f"));
    let output = &find_command.output()?.stdout;
    Ok(String::from_utf8(output.to_vec()).expect("unknown file"))
}

pub fn get_folder_contents(
    current_folder: &str,
    folder_symbol: String,
    file_symbol: String,
) -> Result<Vec<Entry>, Error> {
    let folder_string: String = get_folders(current_folder)?;
    let file_string: String = get_files(current_folder)?;
    let mut entries: Vec<Entry> = Vec::new();

    let mut previous_folder: PathBuf = Path::new(current_folder).to_path_buf();
    previous_folder.pop();
    entries.push(Entry::new(
        previous_folder,
        String::from(".."),
        Item::Folder,
    ));
    for string in folder_string.trim().split("\n") {
        let possible_path = Path::new(&String::from(string)).canonicalize();
        if possible_path.is_err() {
            continue;
        }
        let path = possible_path.unwrap().to_path_buf();
        let name = format!(
            "{folder_symbol}{}",
            String::from(path.file_name().unwrap().to_str().unwrap())
        );
        entries.push(Entry::new(path, name, Item::Folder))
    }

    for string in file_string.trim().split("\n") {
        let possible_path = Path::new(&String::from(string)).canonicalize();
        if possible_path.is_err() {
            continue;
        }
        let path = possible_path.unwrap().to_path_buf();
        let name = format!(
            "{file_symbol}{}",
            String::from(path.file_name().unwrap().to_str().unwrap())
        );
        entries.push(Entry::new(path, name, Item::File));
    }
    Ok(entries)
}

pub fn get_absolute_path_from_str(path: &str) -> PathBuf {
    fs::canonicalize(path).unwrap_or(PathBuf::from("ERROR"))
}
