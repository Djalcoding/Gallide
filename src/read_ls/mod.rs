    use std::{char, fmt::format, fs, io::{self, Write}, path::{Path, PathBuf}, process::Command};

use crate::ui::Entry;
    
    pub enum ListCommandSettings{
        FOLDERS,
        FILES,
        ALL
    }

    fn get_output(current_folder:&str) -> String {
        let mut ls_command = Command::new("sh");
        ls_command
            .arg("-c")
            .arg(format!("ls -d {current_folder}/*/"));
        let output = &ls_command.output().unwrap().stdout;
        String::from_utf8(output.to_vec()).expect("NAN")
    }

    fn get_files_from_cli(current_folder:&str) {
        let mut find_command = Command::new("sh");
        find_command.arg("-c")
        .arg(format!("find {current_folder} -maxdepth 1 -type f"));
    }

    pub fn get_directories(current_folder:&str, symbol:String) -> Vec<Entry>{
        let output_string:String = get_output(current_folder);
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
            let name = format!("{symbol}{}",String::from(path.file_name().unwrap().to_str().unwrap()));
            entries.push(Entry::new(path, name));
        }
        entries
    }

pub fn get_absolute_path_from_str(path:&str) -> PathBuf {
    fs::canonicalize(path).unwrap_or(PathBuf::from("ERROR"))
}


