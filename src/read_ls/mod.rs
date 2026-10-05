use std::{
    collections::VecDeque,
    fs,
    io::Error,
    path::{Path, PathBuf},
    process::Command,
};

use ratatui::widgets::ListItem;

use crate::read_ls::EntryType::{File, Folder};

#[derive(std::cmp::PartialEq, Eq, Clone, Copy)]
pub enum EntryType {
    File,
    Folder,
    SpecialSign,
}

pub struct GallideEntryVec {
    pub names: VecDeque<String>,
    pub paths: VecDeque<PathBuf>,
    pub size: VecDeque<Option<u64>>,
    pub types: VecDeque<EntryType>,
    pub items: VecDeque<ListItem<'static>>,
}

impl GallideEntryVec {
    pub fn new() -> Self {
        Self {
            names: VecDeque::new(),
            paths: VecDeque::new(),
            size: VecDeque::new(),
            types: VecDeque::new(),
            items: VecDeque::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    fn less_or_equal(&self, left: usize, right: usize) -> bool {
        let left_sign = self.types[left];
        let right_sign = self.types[right];
        let left_name = self.names[left].to_lowercase();
        let right_name = self.names[right].to_lowercase();
        if left_sign == EntryType::SpecialSign
            || (left_sign == EntryType::File && left_sign == EntryType::Folder)
        {
            false
        } else if right_sign == EntryType::SpecialSign
            || (right_sign == EntryType::File && left_sign == EntryType::Folder)
        {
            true
        } else if left_name.starts_with('.') {
            false
        } else if right_name.starts_with('.') {
            true
        } else {
            left_name.cmp(&right_name).is_le()
        }
    }

    fn swap(&mut self, i: usize, j: usize) {
        self.names.swap(i, j);
        self.paths.swap(i, j);
        self.size.swap(i, j);
        self.types.swap(i, j);
        self.items.swap(i, j);
    }

    fn partition(&mut self, low: usize, high: usize) -> usize {
        let mut i = low;

        for j in low..high {
            if self.less_or_equal(j, high) {
                i += 1;
                self.swap(i, j);
            }
        }
        self.swap(i + 1, high);
        i + 1
    }
    fn sort_recurse(&mut self, low: usize, high: usize) {
        if low < high {
            let pi = self.partition(low, high);

            self.sort_recurse(low, pi - 1);
            self.sort_recurse(pi + 1, high);
        }
    }
    pub fn sort(&mut self) {
        self.sort_recurse(0, self.names.len() - 1);
    }

    pub fn push(
        &mut self,
        path: PathBuf,
        name: String,
        entry_type: EntryType,
        size: Option<u64>,
        item: ListItem<'static>,
    ) {
        self.paths.push_front(path);
        self.names.push_front(name);
        self.types.push_front(entry_type);
        self.size.push_front(size);
        self.items.push_front(item);
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

fn process_stdout(
    has_size: bool,
    entries: &mut GallideEntryVec,
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
        let item = ListItem::new("template_entry");
        entries.push(path, name, t, size, item)
    }
}

pub fn get_folder_contents(current_folder: &Path, depth: u8) -> Result<GallideEntryVec, Error> {
    let mut entries: GallideEntryVec = GallideEntryVec::new();

    let mut previous_folder: PathBuf = current_folder.to_path_buf();
    previous_folder.pop();
    entries.push(
        previous_folder,
        String::from(".."),
        EntryType::SpecialSign,
        None,
        ListItem::new(".."),
    );
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

    entries.sort();
    Ok(entries)
}

pub fn get_absolute_path_from_str(path: &str) -> PathBuf {
    fs::canonicalize(path).unwrap_or(PathBuf::from("ERROR"))
}
