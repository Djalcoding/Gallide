use std::{
    cmp::Ordering::{Greater, Less},
    fs,
    io::Error,
    path::{Path, PathBuf},
    process::Command,
};

use crate::read_ls::EntryType::{File, Folder};

#[derive(std::cmp::PartialEq, Eq, Clone, Copy)]
pub enum EntryType {
    File,
    Folder,
    SpecialSign,
}

pub struct GallideEntry {
    path: PathBuf,
    name: String,
    pub entry_type: EntryType,
    size: Option<u64>,
}

impl GallideEntry {
    pub fn new(path: PathBuf, name: String, entry_type: EntryType, size: Option<u64>) -> Self {
        GallideEntry {
            path,
            name,
            entry_type,
            size,
        }
    }
    pub fn set_type(&mut self) {}
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
    pub fn name(&self) -> &String {
        &self.name
    }
    pub fn size(&self) -> Option<u64> {
        self.size
    }
    pub fn set_name(&mut self, new_name: &str) {
        self.name = String::from(new_name);
        self.path.pop();
        self.path.push(new_name);
    }
}

fn get_folders(current_folder: &Path, depth: u8) -> Result<String, Error> {
    let mut ls_command = Command::new("sh");
    ls_command.arg("-c").arg(format!(
        "find \"{}\" -type d -maxdepth {depth} -printf \"%p\\n\"",
        current_folder.to_string_lossy()
    ));
    let output = &ls_command.output()?.stdout;
    Ok(String::from_utf8(output.to_vec()).expect("unknown folder"))
}

fn get_files(current_folder: &Path, depth: u8) -> Result<String, Error> {
    let mut find_command = Command::new("sh");
    find_command.arg("-c").arg(format!(
        "find \"{}\" -type f -maxdepth {depth} -printf \"%p__FILE_SIZE=%k\\n\"",
        current_folder.to_string_lossy()
    ));
    let output = &find_command.output()?.stdout;
    Ok(String::from_utf8(output.to_vec()).expect("unknown file"))
}

fn cmp_entries(a: &GallideEntry, b: &GallideEntry) -> std::cmp::Ordering {
    if a.entry_type == EntryType::SpecialSign || (a.entry_type == Folder && b.entry_type == File) {
        return Less;
    } else if b.entry_type == EntryType::SpecialSign
        || (a.entry_type == File && b.entry_type == Folder)
    {
        return Greater;
    }
    if a.name().starts_with('.') {
        return Greater;
    } else if b.name().starts_with('.') {
        return Less;
    }
    a.name().to_lowercase().cmp(&b.name().to_lowercase())
}

fn process_stdout(
    has_size: bool,
    entries: &mut Vec<GallideEntry>,
    split: String,
    current_path: &Path,
    t: EntryType,
) {
    for string in split.trim().split('\n') {
        if string.is_empty() {
            continue;
        }
        let first_part;
        let mut size = None;
        if has_size {
            let parts = string.split("__FILE_SIZE=").collect::<Vec<&str>>();
            first_part = parts[0];
            size = parts[1].parse().ok()
        } else {
            first_part = string;
        }
        let possible_path = Path::new(&String::from(first_part)).canonicalize();
        if possible_path.is_err() {
            continue;
        }
        let path = possible_path.unwrap().to_path_buf();
        if path == current_path {
            continue;
        }
        let filename = path.file_name();
        if filename.is_none() {
            continue;
        }
        let name = String::from(
            path.strip_prefix(current_path)
                .unwrap() // This should never fail
                .to_string_lossy(),
        );
        entries.push(GallideEntry::new(path, name, t, size))
    }
}

pub fn get_folder_contents(current_folder: &Path, depth: u8) -> Result<Vec<GallideEntry>, Error> {
    let mut entries: Vec<GallideEntry> = Vec::new();

    let mut previous_folder: PathBuf = current_folder.to_path_buf();
    previous_folder.pop();
    entries.push(GallideEntry::new(
        previous_folder,
        String::from(".."),
        EntryType::SpecialSign,
        None,
    ));
    process_stdout(
        false,
        &mut entries,
        get_folders(current_folder, depth)?,
        current_folder,
        Folder,
    );
    process_stdout(
        true,
        &mut entries,
        get_files(current_folder, depth)?,
        current_folder,
        File,
    );

    entries.sort_by(cmp_entries);
    Ok(entries)
}

pub fn get_absolute_path_from_str(path: &str) -> PathBuf {
    fs::canonicalize(path).unwrap_or(PathBuf::from("ERROR"))
}
