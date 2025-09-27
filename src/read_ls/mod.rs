    use std::{char, process::Command};


    pub fn get_directories() -> Vec<String>{
        let mut ls_command = Command::new("sh");
        ls_command
            .arg("-c")
            .arg("ls -d */");
        let mut ascci_vec:Vec<char> = Vec::new();
        for digit in ls_command.output().unwrap().stdout {
            ascci_vec.push(digit as char); 
        } 
        let mut current_string:String = String::new();
        let mut directory_list:Vec<String> = Vec::new();
        directory_list.push(String::from(".."));
        for character in &ascci_vec {
            if *character == '\n' {
                directory_list.push(format!("./{}", current_string.clone()));
                current_string.clear();
                continue;
            }
            current_string.push(*character);
        }
        directory_list
    }
