use std::{
    fs::{self, File},
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
};

use crate::{
    read_ls::{EntryType, GallideEntry},
    ui_brain::user_input::{ExitType, UserInputRequest, UserOperationResult},
};

// TODO : move this to Entry

pub fn create_ressource(name: &str, path: &Path) -> std::io::Result<GallideEntry> {
    let mut buffer: PathBuf = path.to_path_buf();
    if !name.chars().any(|c| c.is_ascii_alphanumeric()) {
        return Err(Error::from(ErrorKind::InvalidFilename));
    }
    let parts: Vec<&str> = name.split('/').collect::<Vec<&str>>();
    let mut is_file: bool = true;
    for (i, part) in parts.iter().enumerate() {
        buffer.push(part);
        if i == parts.len() - 1 {
            is_file = part.contains('.');
            if is_file {
                File::create(&buffer)?;
            } else {
                fs::create_dir(&buffer)?;
            }
        } else {
            if !fs::exists(&buffer)? {
                fs::create_dir(&buffer)?;
            }
        }
    }
    let mut first_added_entry = path.to_path_buf();
    first_added_entry.push(parts[0]);
    Ok(GallideEntry::new(
        first_added_entry,
        parts[0].to_string(),
        if parts.len() == 1 && is_file {
            crate::read_ls::EntryType::File
        } else {
            crate::read_ls::EntryType::Folder
        },
    ))
}

pub fn rename_ressource_request(entry: &GallideEntry) -> UserInputRequest {
    let path = entry.path().clone();
    let title = format!(
        "Insert new name for {} ? ",
        path.file_name().unwrap().to_string_lossy()
    );
    UserInputRequest::new(
        title,
        true,
        Box::new(move |state, new_name| {
            let message: Option<String>;
            let exit_type: ExitType = match fs::rename(&path, new_name) {
                Ok(_) => {
                    message = Some(format!(
                        "'{}' was properly renamed to '{new_name}'",
                        path.display()
                    ));
                    state.get_selected_entry().set_name(new_name);
                    ExitType::Sucess
                }
                Err(e) => {
                    message = Some(format!(
                        "Could not rename '{}' to '{new_name}' ({e})",
                        path.display()
                    ));
                    ExitType::Error
                }
            };
            UserOperationResult {
                exit: exit_type,
                message,
            }
        }),
    )
}

pub fn delete_ressource_request(entry: &GallideEntry) -> UserInputRequest {
    let path = entry.path().clone();
    let is_file: bool = entry.entry_type == EntryType::File;
    let title = format!(
        "Are you sure you want to delete {}? {} (y/n)",
        entry.path().file_name().unwrap().to_string_lossy(),
        if is_file {
            ""
        } else {
            "(this will perform recursive deletion) "
        }
    );
    UserInputRequest::new(
        title,
        true,
        Box::new(move |state, user_input| {
            let mut message: Option<String> = None;
            let result: ExitType = match user_input.to_lowercase().as_str() {
                "yes" | "y" => {
                    if let Err(e) = if is_file {
                        fs::remove_file(&path)
                    } else {
                        fs::remove_dir(&path)
                    } {
                        message = Some(format!("could not remove '{}' ({e})", path.display()));
                        ExitType::Error
                    } else {
                        state.remove_selected();
                        message = Some(format!("'{}' was properly removed", path.display()));
                        ExitType::Sucess
                    }
                }
                _ => ExitType::Aborted,
            };
            UserOperationResult {
                exit: result,
                message,
            }
        }),
    )
}

pub fn create_ressource_request() -> UserInputRequest {
    UserInputRequest::new(
        String::from("Insert new ressource name"),
        false,
        Box::new(move |state, name| {
            let mut directory = state.get_current_directory().clone();
            let message: Option<String>;
            let exit_type: ExitType = match create_ressource(name, &directory) {
                Err(e) => {
                    message = Some(format!("Could not create file '{name}' : {e}"));
                    ExitType::Error
                }
                Ok(entry) => {
                    message = Some(format!(
                        "new {} '{name}' was properly created !",
                        if entry.entry_type == EntryType::File {
                            "file"
                        } else {
                            "directory"
                        }
                    ));
                    directory.push(name);
                    state.add_top_priority_entry(entry);
                    ExitType::Sucess
                }
            };
            UserOperationResult {
                exit: exit_type,
                message,
            }
        }),
    )
}
