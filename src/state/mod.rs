pub mod user_input;

use crossterm::event::{self, Event::Key, KeyCode};
use ratatui::{DefaultTerminal};

use crate::{
    config::Config, file_control::{create_ressource_request, delete_ressource_request, rename_ressource_request}, read_ls::{EntryType, GallideEntry, get_absolute_path_from_str, get_folder_contents}, reporter::Reporter, state::user_input::UserOperationResult, ui::render,
};
use std::{collections::VecDeque, path::PathBuf, time::Duration};

#[derive(PartialEq, Clone)]
pub enum Mode {
    INSERT,
    WRITING,
    SELECTING,
    DISCARD,
}

pub struct State {
    cursor: usize,
    depth: u8,
    elements: VecDeque<GallideEntry>,
    search_bar_text: String,
    current_dir: PathBuf,
    running: bool,
    pub exited: bool,
    pub mode: Mode,
    config: Config,
    user_input: String,
    pub user_input_request: user_input::UserInputRequest,
}

impl State {
    pub fn new(config: Config) -> Self {
        let mut state = State {
            cursor: 0,
            depth: config.main_box.default_depth,
            elements: VecDeque::new(),
            search_bar_text: String::from(""),
            running: true,
            exited: true,
            current_dir: get_absolute_path_from_str("."),
            mode: Mode::SELECTING,
            config,
            user_input: String::new(),
            user_input_request: user_input::UserInputRequest {
                edit_ressource: false,
                title: String::new(),
                on_enter: None,
            },
        };
        state.rebuild_directories();
        if state.elements.len() > 1 {
            state.cursor = 1;
        }
        state
    }

    pub fn run(&mut self, terminal:&mut DefaultTerminal) -> std::io::Result<()> {
        while self.is_running() {
            terminal.draw(|frame| render(frame, self, &self.config))? ;
            if event::poll(Duration::from_millis(50))?
                && let Key(key) = event::read()?
            {
                match self.mode {
                    Mode::INSERT => match key.code {
                        KeyCode::Esc | KeyCode::Char('\n') => self.switch_mode(),
                        KeyCode::Backspace => self.backspace(),
                        KeyCode::Char(character) => self.add_character(character),
                        KeyCode::Up => self.decrement_selected_box(),
                        KeyCode::Down => self.increment_selected_box(),
                        _ => {}
                    },
                    Mode::SELECTING => match key.code {
                        KeyCode::Up | KeyCode::Char('k') => self.decrement_selected_box(),
                        KeyCode::Down | KeyCode::Char('j') => self.increment_selected_box(),

                        KeyCode::Esc | KeyCode::Char('q') => {
                            self.exited = true;
                            self.stop();
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            if self.is_selecting_directory() {
                                self.open_selected_directory();
                                self.rebuild_directories();
                            } else {
                                self.stop();
                            }
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            self.go_back_one_directory();
                            self.rebuild_directories();
                        }
                        KeyCode::Char('\n') => self.stop(),
                        KeyCode::Char('i') => {
                            if self.config.search_bar.enabled {
                                self.switch_mode()
                            }
                        }
                        KeyCode::Char('c') => self.clear_search_bar(),
                        KeyCode::Char('a') => {
                            self.ask_input(create_ressource_request());
                        }
                        KeyCode::Char('d') => {
                            self.ask_input(delete_ressource_request(self.read_selected_entry()));
                        }
                        KeyCode::Char('r') => {
                            self.ask_input(rename_ressource_request(self.read_selected_entry()));
                        }
                        KeyCode::Char('+') => {
                            self.increase_depth();
                        }
                        KeyCode::Char('-') => {
                            self.decrease_depth();
                        }
                        _ => {}
                    },
                    #[allow(clippy::needless_late_init)]
                    Mode::WRITING => match key.code {
                        KeyCode::Esc => self.mode = Mode::SELECTING,
                        KeyCode::Char('\n') => {
                            let user_input = self.read_user_input().clone();
                            let request = self.user_input_request.get_closure();

                            let result: UserOperationResult;
                            if self.user_input_request.edit_ressource
                                && self.selecting_previous_dir()
                            {
                                result = UserOperationResult::operation_on_prev();
                            } else {
                                result = request(self, &user_input);
                            }

                            if let Some(message) = result.message {
                                self.show_message(result.exit.get_header(), &message);
                            } else {
                                self.mode = Mode::SELECTING;
                            }
                        }
                        KeyCode::Char(character) => self.user_input().push(character),
                        KeyCode::Backspace => {
                            self.user_input().pop();
                        }
                        _ => {}
                    },
                    Mode::DISCARD => {
                        self.mode = Mode::SELECTING;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_selected_box(&self) -> usize {
        self.cursor
    }
    pub fn read_selected_entry(&self) -> &GallideEntry {
        &self.elements[self.cursor]
    }
    pub fn get_selected_entry(&mut self) -> &mut GallideEntry {
        &mut self.elements[self.cursor]
    }

    pub fn increment_selected_box(&mut self) {
        self.cursor = (self.cursor + 1) % self.elements.len();
    }

    pub fn decrement_selected_box(&mut self) {
        if self.cursor == 0 {
            self.cursor = self.elements.len() - 1;
        } else {
            self.cursor -= 1;
        }
    }

    pub fn move_selected_box_to_start(&mut self) {
        if self.elements.len() == 1 {
            self.cursor = 0;
        } else {
            self.cursor = 1;
        }
    }

    pub fn increase_depth(&mut self) {
        self.depth = self.depth.saturating_add(1);
        self.rebuild_directories();
    }
    pub fn decrease_depth(&mut self) {
        if self.depth != 1 {
            self.depth -= 1;
        }
        self.rebuild_directories();
    }

    pub fn depth(&self) -> u8 {
        self.depth
    }

    fn reset_search_bar(&mut self) {
        self.search_bar_text = String::new();
        self.trim_directories();
    }

    fn get_current_dir_folder_contents(&mut self) -> Vec<GallideEntry> {
        let contents = get_folder_contents(self.get_current_directory(), self.depth);
        match contents {
            Ok(entries) => entries,
            Err(_) => {
                self.stop();
                vec![]
            }
        }
    }

    pub fn trim_directories(&mut self) {
        let mut new_list: VecDeque<GallideEntry> = VecDeque::new();
        let mut curated_search_bar_text = String::from(self.search_bar_text.trim());
        if !self.config.case_sensitive {
            curated_search_bar_text = curated_search_bar_text.to_lowercase();
        }
        for element in self.get_current_dir_folder_contents() {
            let mut curated_name = element.name().clone();
            if !self.config.case_sensitive {
                curated_name = curated_name.to_lowercase();
            }
            if let EntryType::SpecialSign = element.entry_type {
                new_list.push_back(element);
                continue;
            } else if curated_name.starts_with(&curated_search_bar_text) {
                new_list.push_back(element);
            }
        }

        self.elements = new_list;
    }

    pub fn add_top_priority_entry(&mut self, entry: GallideEntry) {
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
        self.cursor = self.elements.len() - 1;
    }

    pub fn add_character(&mut self, character: char) {
        self.search_bar_text.push(character);
        self.trim_directories();
        self.cursor = self.elements.len() - 1;
    }

    pub fn clear_search_bar(&mut self) {
        self.search_bar_text = String::new();
        self.trim_directories();
        self.cursor = self.elements.len() - 1;
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
    pub fn get_bash_string(&self) -> String {
        format!(
            "{}'{}",
            if self.is_selecting_directory() || self.exited {
                "D"
            } else {
                "F"
            },
            if self.exited {
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

    pub fn elements(&self) -> &VecDeque<GallideEntry> {
        &self.elements
    }

    pub fn current_searchbar_text(&self) -> &String {
        &self.search_bar_text
    }

    pub fn is_selecting_directory(&self) -> bool {
        if let EntryType::Folder = self.elements[self.cursor].entry_type {
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
        let index = self.get_selected_box();
        self.elements.remove(index);
        self.cursor = std::cmp::min(0, index - 1);
    }
    pub fn show_message(&mut self, title: &str, info: &str) {
        self.user_input = String::from(info);
        self.user_input_request.title = String::from(title);
        self.mode = Mode::DISCARD;
    }
}
