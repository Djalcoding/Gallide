pub mod user_input;

use crate::{
    config::Config,
    read_ls::{Entry, Item, get_absolute_path_from_str, get_folder_contents},
    reporter::Reporter,
};
use std::{cell::RefCell, collections::VecDeque, path::PathBuf, rc::Rc};

#[derive(PartialEq, Clone)]
pub enum Mode {
    INSERT,
    WRITING,
    SELECTING,
    DISCARD,
}

pub struct State {
    selected_box: usize,
    elements: VecDeque<Entry>,
    search_bar_text: String,
    current_dir: PathBuf,
    running: bool,
    pub mode: Mode,
    config: Config,
    reporter: Reporter,
    user_input: String,
    pub user_input_request: user_input::UserInputRequest,
}

pub type StateRcCell = Rc<RefCell<State>>;
impl State {
    pub fn new(config: Config, reporter: Reporter) -> Self {
        let mut state = State {
            selected_box: 0,
            elements: VecDeque::new(),
            search_bar_text: String::from(""),
            running: true,
            current_dir: get_absolute_path_from_str("."),
            mode: Mode::SELECTING,
            config,
            reporter,
            user_input: String::new(),
            user_input_request: user_input::UserInputRequest {
                edit_ressource: false,
                title: String::new(),
                on_enter: None,
            },
        };
        state.rebuild_directories();
        if state.elements.len() > 1 {
            state.selected_box = 1;
        }
        state
    }

    pub fn get_selected_box(&self) -> usize {
        self.selected_box
    }
    pub fn read_selected_entry(&self) -> &Entry {
        &self.elements[self.selected_box]
    }
    pub fn get_selected_entry(&mut self) -> &mut Entry {
        &mut self.elements[self.selected_box]
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

    fn get_current_directory_str(&mut self) -> String {
        let lossy = self.current_dir.to_string_lossy().to_string();
        if self.current_dir.to_str().is_none() {
            self.reporter
                .push(format!("Invalid UTF-8 : {lossy}").as_str());
        }
        lossy
    }

    fn get_current_dir_folder_contents(&mut self) -> Vec<Entry> {
        let contents = get_folder_contents(self.get_current_directory_str().as_str());
        match contents {
            Ok(entries) => entries,
            Err(e) => {
                self.reporter
                    .push(format!("could not read contents of directory because : {e}").as_str());
                self.stop();
                vec![]
            }
        }
    }

    pub fn trim_directories(&mut self) {
        let mut new_list: VecDeque<Entry> = VecDeque::new();
        let mut curated_search_bar_text = String::from(self.search_bar_text.trim());
        if !self.config.case_sensitive {
            curated_search_bar_text = curated_search_bar_text.to_lowercase();
        }
        for element in self.get_current_dir_folder_contents() {
            let mut curated_name = element.name().clone();
            if !self.config.case_sensitive {
                curated_name = curated_name.to_lowercase();
            }
            if let Item::SpecialSign = element.entry_type {
                new_list.push_back(element);
                continue;
            } else if curated_name.starts_with(&curated_search_bar_text) {
                new_list.push_back(element);
            }
        }

        self.elements = new_list;
    }

    pub fn add_top_priority_entry(&mut self, entry: Entry) {
        let previous_directory = self.elements.pop_front().unwrap();
        self.elements.push_front(entry);
        self.elements.push_front(previous_directory);
    }

    pub fn rebuild_directories(&mut self) {
        self.elements = VecDeque::from_iter(self.get_current_dir_folder_contents());
        self.move_selected_box_to_start()
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
        self.set_current_directory(self.read_selected_entry().path().to_path_buf());
        self.reset_search_bar();
        self.move_selected_box_to_start()
    }
    pub fn get_bash_string(&self, exited: bool) -> String {
        format!(
            "{}'{}",
            if self.is_selecting_directory() || exited {
                "D"
            } else {
                "F"
            },
            if exited {
                self.get_current_directory()
            } else {
                self.read_selected_entry().path()
            }
            .display()
        )
    }

    pub fn get_current_directory(&self) -> &PathBuf {
        &self.current_dir
    }

    pub fn elements(&self) -> &VecDeque<Entry> {
        &self.elements
    }

    pub fn current_searchbar_text(&self) -> String {
        self.search_bar_text.clone()
    }

    pub fn is_selecting_directory(&self) -> bool {
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

    pub fn publish_reports(&mut self) {
        self.reporter.publish();
    }

    pub fn user_input(&mut self) -> &mut String {
        &mut self.user_input
    }
    pub fn read_user_input(&self) -> &String {
        &self.user_input
    }
    pub fn user_input_title(&self) -> &String {
        &self.user_input_request.title
    }

    pub fn ask_input(&mut self, config: user_input::UserInputRequest) {
        self.user_input.clear();
        self.user_input_request = config;
        self.mode = Mode::WRITING;
    }
    pub fn is_in_write_mode(&self) -> bool {
        matches!(self.mode, Mode::DISCARD | Mode::WRITING,)
    }

    pub fn selecting_previous_dir(&self) -> bool {
        self.get_selected_box() == 0
    }

    pub fn remove_selected(&mut self) {
        self.elements.remove(self.get_selected_box());
    }
    pub fn show_message(&mut self, title: &str, info: &str) {
        self.user_input = String::from(info);
        self.user_input_request.title = String::from(title);
        self.mode = Mode::DISCARD;
    }
}
