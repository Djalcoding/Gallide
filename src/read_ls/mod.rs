use std::{
    fs,
    io::Error,
    path::{Path, PathBuf},
    process::Command,
};


#[derive(std::cmp::PartialEq)]
pub enum EntryType {
    File,
    Folder,
    SpecialSign,
}

pub struct GallideEntry {
    path: PathBuf,
    name: String,
    pub entry_type: EntryType,
}

impl GallideEntry {
    pub fn new(path: PathBuf, name: String, entry_type: EntryType) -> Self {
        GallideEntry {
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
    pub fn set_name(&mut self, new_name:&str) {
        self.name = String::from(new_name);
        self.path.pop();
        self.path.push(new_name);
    }
}

fn get_folders(current_folder: &str) -> Result<String, Error> {
    let mut ls_command = Command::new("sh");
    ls_command
        .arg("-c")
        .arg(format!("ls -a -d \"{current_folder}\"/*/"));
    let output = &ls_command.output()?.stdout;
    Ok(String::from_utf8(output.to_vec()).expect("unknown folder"))
}

fn get_files(current_folder: &str) -> Result<String, Error> {
    let mut find_command = Command::new("sh");
    find_command
        .arg("-c")
        .arg(format!("find \"{current_folder}\" -maxdepth 1 -type f"));
    let output = &find_command.output()?.stdout;
    Ok(String::from_utf8(output.to_vec()).expect("unknown file"))
}

pub fn get_folder_contents(current_folder: &str) -> Result<Vec<GallideEntry>, Error> {
    let folder_string: String = get_folders(current_folder)?;
    let file_string: String = get_files(current_folder)?;
    let mut entries: Vec<GallideEntry> = Vec::new();

    let mut previous_folder: PathBuf = Path::new(current_folder).to_path_buf();
    previous_folder.pop();
    entries.push(GallideEntry::new(
        previous_folder,
        String::from(".."),
        EntryType::SpecialSign,
    ));
    for string in folder_string.trim().split("\n") {
        let possible_path = Path::new(&String::from(string)).canonicalize();
        if possible_path.is_err() {
            continue;
        }
        let path = possible_path.unwrap().to_path_buf();
        let name = String::from(path.file_name().unwrap().to_string_lossy());
        entries.push(GallideEntry::new(path, name, EntryType::Folder))
    }

    for string in file_string.trim().split("\n") {
        let possible_path = Path::new(&String::from(string)).canonicalize();
        if possible_path.is_err() {
            continue;
        }
        let path = possible_path.unwrap().to_path_buf();
        let name = String::from(path.file_name().unwrap().to_str().unwrap());
        entries.push(GallideEntry::new(path, name, EntryType::File));
    }
    Ok(entries)
}

pub fn get_absolute_path_from_str(path: &str) -> PathBuf {
    fs::canonicalize(path).unwrap_or(PathBuf::from("ERROR"))
}
