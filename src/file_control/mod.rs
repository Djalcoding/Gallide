use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use crate::{
    read_ls::Entry,
    ui_brain::{
        StateRcCell,
        user_input::{ExitType, UserInputRequest, UserOperationResult},
    },
};

pub fn create_file(name: &str, path: &Path) -> std::io::Result<()> {
    let mut buffer: PathBuf = path.to_path_buf();
    buffer.push(name);
    let _ = File::create(buffer)?;
    Ok(())
}

pub fn rename_ressource_request(path: PathBuf, state: StateRcCell) -> UserInputRequest {
    let title = format!(
        " Insert new name for {} ? ",
        path.file_name().unwrap().to_string_lossy()
    );
    UserInputRequest::new(
        title,
        true,
        Box::new(move |new_name| {
            let message: Option<String>;
            let exit_type: ExitType = match fs::rename(&path, new_name) {
                Ok(_) => {
                    message = Some(format!(
                        "'{}' was properly renamed to '{new_name}'",
                        path.display()
                    ));
                    state.borrow_mut().get_selected_entry().set_name(new_name);
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

pub fn delete_ressource_request(path: PathBuf, state: StateRcCell) -> UserInputRequest {
    let title = format!(
        " Delete {} ? (y/n)",
        path.file_name().unwrap().to_string_lossy()
    );
    UserInputRequest::new(
        title,
        true,
        Box::new(move |user_input| {
            let mut message: Option<String> = None;
            let result: ExitType = match user_input.to_lowercase().as_str() {
                "yes" | "y" => {
                    if let Err(e) = fs::remove_file(&path) {
                        message = Some(format!("could not remove '{}' ({e})", path.display()));
                        ExitType::Error
                    } else {
                        state.borrow_mut().remove_selected();
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

pub fn create_ressource_request(state: StateRcCell) -> UserInputRequest {
    UserInputRequest::new(
        String::from(" Insert new file name "),
        false,
        Box::new(move |name| {
            let mut directory = state.borrow().get_current_directory().clone();
            let message: Option<String>;
            let exit_type: ExitType = match create_file(name, &directory) {
                Err(e) => {
                    message = Some(format!("Could not create file '{name}' : {e}"));
                    ExitType::Error
                }
                Ok(_) => {
                    message = Some(format!("'{name}' was properly created !"));
                    directory.push(name);
                    state.borrow_mut().add_top_priority_entry(Entry::new(
                        directory.to_path_buf(),
                        String::from(name),
                        crate::read_ls::Item::File,
                    ));
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
