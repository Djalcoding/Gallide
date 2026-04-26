use crate::{
    config::Config,
    read_ls::{Entry, Item, get_absolute_path_from_str, get_folder_contents},
};
use std::path::PathBuf;

pub enum Mode {
    INSERT,
    SELECTING,
}

pub struct State {
    selected_box: usize,
    elements: Vec<Entry>,
    search_bar_text: String,
    current_dir: PathBuf,
    running: bool,
    mode: Mode,
    config: Config,
}

impl State {
    pub fn new(elements: Vec<Entry>, config: Config) -> State {
        State {
            selected_box: if elements.len() > 1 { 1 } else { 0 },
            elements,
            search_bar_text: String::from(""),
            running: true,
            current_dir: get_absolute_path_from_str("."),
            mode: Mode::SELECTING,
            config,
        }
    }

    pub fn get_selected_box(&self) -> usize {
        self.selected_box
    }

    pub fn increment_selected_box(&mut self) {
        self.selected_box = (self.selected_box + 1) % self.elements.len();
    }

    pub fn decrement_selected_box(&mut self) {
        if self.selected_box == 0 {
            self.selected_box = self.elements.len() - 1;
        } else {
            self.selected_box -= 1;
        }
    }

    pub fn move_selected_box_to_start(&mut self) {
        if self.elements.len() == 1 {
            self.selected_box = 0;
        } else {
            self.selected_box = 1;
        }
    }

    fn reset_search_bar(&mut self) {
        self.search_bar_text = String::new();
        self.trim_directories();
    }

    pub fn trim_directories(&mut self) {
        let mut new_list: Vec<Entry> = Vec::new();
        let mut curated_search_bar_text = String::from(self.search_bar_text.trim());
        if !self.config.case_sensitive {
            curated_search_bar_text  = curated_search_bar_text.to_lowercase();
        }

        for element in get_folder_contents(
            self.current_dir.to_str().unwrap(),
        )
        .unwrap()
        {
            let mut curated_name = element.name().clone();
            if !self.config.case_sensitive {
                curated_name = curated_name.to_lowercase(); 
            }
            if let Item::SpecialSign = element.entry_type {
                new_list.push(element);
                continue;
            } else if curated_name.starts_with(&curated_search_bar_text){
                new_list.push(element);
            }
        }

        self.elements = new_list;
    }

    pub fn rebuild_directories(&mut self) {
        self.elements = get_folder_contents(
            self.current_dir.to_str().expect("INVALID UNICODE"),
        )
        .unwrap(); // TODO : Handle this
        self.move_selected_box_to_start()
    }

    pub fn get_selected_path(&self) -> PathBuf {
        self.elements[self.get_selected_box()].path().to_path_buf()
    }

    pub fn go_back_one_directory(&mut self) {
        self.current_dir.pop();
    }

    pub fn backspace(&mut self) {
        self.search_bar_text.pop();
        self.trim_directories();
        self.selected_box = self.elements.len() - 1;
    }

    pub fn add_character(&mut self, character: char) {
        self.search_bar_text.push(character);
        self.trim_directories();
        self.selected_box = self.elements.len() - 1;
    }

    pub fn clear_search_bar(&mut self) {
        self.search_bar_text = String::new();
        self.trim_directories();
        self.selected_box = self.elements.len() - 1;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn switch_mode(&mut self) {
        self.mode = if self.is_inserting() {
            Mode::SELECTING
        } else {
            Mode::INSERT
        }
    }

    pub fn is_inserting(&self) -> bool {
        if let Mode::INSERT = &self.mode {
            return true;
        }
        false
    }

    pub fn open_selected_directory(&mut self) {
        if !self.is_selecting_directory() {
            self.stop();
            return;
        }
        self.set_current_directory(self.get_selected_path());
        self.reset_search_bar();
        self.move_selected_box_to_start()
    }
    pub fn get_bash_string(&self, exited: bool) -> String {
        format!(
            "({}'{})",
            if self.is_selecting_directory() || exited{
                "D"
            } else {
                "F"
            },
            if exited {
                self.get_current_directory()
            }
            else {
                self.get_selected_path()
            }.display()
        )
    }

    pub fn get_current_directory(&self) -> PathBuf {
        self.current_dir.clone()
    }

    pub fn elements(&self) -> &Vec<Entry> {
        &self.elements
    }

    pub fn current_searchbar_text(&self) -> String {
        self.search_bar_text.clone()
    }

    fn is_selecting_directory(&self) -> bool {
        if let Item::Folder = self.elements[self.selected_box].entry_type {
            return true;
        }
        false
    }

    fn set_current_directory(&mut self, new_directory: PathBuf) {
        self.current_dir = new_directory;
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }
}
