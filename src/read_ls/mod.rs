    use std::{char, fmt::format, fs, io::{self, Write}, path::{Path, PathBuf}, process::Command};

use crate::ui::Entry;

    pub fn get_directories(current_folder:&str) -> Vec<Entry>{
        let mut ls_command = Command::new("sh");
        ls_command
            .arg("-c")
            .arg(format!("ls -d {current_folder}/*/"));
        let output = &ls_command.output().unwrap().stdout;
        let output_string:String = String::from_utf8(output.to_vec()).expect("NAN");
        let mut entries:Vec<Entry> = Vec::new();
        
        let mut previous_folder:PathBuf = Path::new(current_folder).to_path_buf();
        previous_folder.pop();
        entries.push(Entry::new(previous_folder, String::from("..")));
        for string in output_string.trim().split("\n") {
            let possible_path = Path::new(&String::from(string)).canonicalize();
            if possible_path.is_err() {
                continue;
            } 
            let path = possible_path.unwrap().to_path_buf();
            let name = String::from(path.file_name().unwrap().to_str().unwrap());
            entries.push(Entry::new(path, name));
        }
        entries
    }

pub fn get_absolute_path_from_str(path:&str) -> PathBuf {
    fs::canonicalize(path).unwrap_or(PathBuf::from("ERROR"))
}


