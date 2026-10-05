pub mod user_input;

use crossterm::event::{self, Event::Key, KeyCode, KeyEvent, KeyModifiers};
use ratatui::DefaultTerminal;

use crate::{
    config::Config,
    file_control::{create_ressource_request, delete_ressource_request, rename_ressource_request},
    read_ls::{EntryType, GallideEntryVec, get_absolute_path_from_str, get_folder_contents},
    state::user_input::UserOperationResult,
};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

mod render;

#[derive(PartialEq, Clone)]
pub enum Mode {
    INSERT,
    WRITING,
    SELECTING,
    DISCARD,
}

pub struct UserInput {
    value: String,
}

impl Default for UserInput {
    fn default() -> Self {
        Self::new()
    }
}

impl UserInput {
    pub fn new() -> Self {
        let mut value: String = String::new();
        value.reserve(100);
        Self { value }
    }
    pub fn read(&self) -> &String {
        &self.value
    }

    pub fn push(&mut self, c: char) {
        self.value.push(c);
    }
    pub fn pop(&mut self) {
        self.value.pop();
    }
    pub fn clear(&mut self) {
        self.value.clear();
    }

    pub fn len(&self) -> usize {
        self.value.len()
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }
}

pub struct State {
    jump_buffer: usize,
    cursor: usize,
    depth: u8,
    elements: GallideEntryVec,
    search_bar_text: UserInput,
    question_box_text: UserInput,
    current_dir: PathBuf,
    config: Config,
    mode: Mode,
    pub user_input_request: user_input::UserInputRequest,
    running: bool,
    exited: bool,
}

impl State {
    pub fn new(config: Config) -> Self {
        let mut state = State {
            cursor: 0,
            jump_buffer: 0,
            depth: config.main_box.default_depth,
            elements: GallideEntryVec::new(),
            search_bar_text: UserInput::new(),
            running: true,
            exited: true,
            current_dir: get_absolute_path_from_str("."),
            mode: Mode::SELECTING,
            config,
            question_box_text: UserInput::new(),
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

    fn handle_select_mode_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_up(),
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),

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
            KeyCode::Enter => self.stop(),
            KeyCode::Char('i') => {
                if self.config.search_bar.enabled {
                    self.switch_mode()
                }
            }
            KeyCode::Char('c') => self.search_bar_text.clear(),
            KeyCode::Char('a') => {
                self.ask_input(create_ressource_request());
            }
            KeyCode::Char('d') => {
                //self.ask_input(delete_ressource_request(self.read_selected_entry()));
            }
            KeyCode::Char('r') => {
                // self.ask_input(rename_ressource_request(self.read_selected_entry()));
            }
            KeyCode::Char('+') => {
                self.increase_depth();
            }
            KeyCode::Char('-') => {
                self.decrease_depth();
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                self.jump_buffer = self.jump_buffer.saturating_mul(10);
                self.jump_buffer = self
                    .jump_buffer
                    .saturating_add(c.to_digit(10).unwrap() as usize)
            }
            _ => {}
        }
    }

    fn handle_insert_mode_input(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            self.handle_select_mode_input(key);
            return;
        }
        match key.code {
            KeyCode::Esc | KeyCode::Enter => self.switch_mode(),
            KeyCode::Backspace => self.backspace(),
            KeyCode::Char(character) => self.add_character(character),
            KeyCode::Up => self.move_down(),
            KeyCode::Down => self.move_up(),
            _ => {}
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        while self.is_running() {
            terminal.draw(|frame| self.render(frame, &self.config))?;
            if event::poll(Duration::from_millis(50))?
                && let Key(key) = event::read()?
            {
                match self.mode {
                    Mode::INSERT => self.handle_insert_mode_input(key),
                    Mode::SELECTING => self.handle_select_mode_input(key),
                    #[allow(clippy::needless_late_init)]
                    Mode::WRITING => match key.code {
                        KeyCode::Esc => self.mode = Mode::SELECTING,
                        KeyCode::Char('\n') => {
                            let user_input = self.question_box_text.read().clone();
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
                        KeyCode::Char(character) => self.question_box_text.push(character),
                        KeyCode::Backspace => {
                            self.question_box_text.pop();
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

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn move_down(&mut self) {
        if self.cursor == self.elements.len() - 1 {
            self.cursor = 0;
        } else {
            self.cursor = std::cmp::min(
                self.elements.len() - 1,
                self.cursor + std::cmp::max(1, self.jump_buffer),
            );
        }
        self.jump_buffer = 0;
    }

    pub fn move_up(&mut self) {
        if self.cursor == 0 {
            self.cursor = self.elements.len() - 1;
        } else {
            self.cursor = self
                .cursor
                .saturating_sub(std::cmp::max(1, self.jump_buffer));
        }
        self.jump_buffer = 0;
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
        if self.depth != 1  {
            self.depth -= 1;
        }
        self.rebuild_directories();
    }


    pub fn trim_directories(&mut self) {
        let mut new_list: GallideEntryVec = GallideEntryVec::new();
        let user_input = self.search_bar_text.read().trim().clone();
        let mut curated_search_bar_text = String::from(user_input.trim());
        if !self.config.case_sensitive {
            curated_search_bar_text = curated_search_bar_text.to_lowercase();
        }
        let case_sensitive: bool = self.config.case_sensitive;
        let fresh_list = get_folder_contents(&self.current_dir, self.depth).unwrap();
        for idx in 0..fresh_list.len() {
            let mut name = fresh_list.names[idx].clone();
            let entry_type = fresh_list.types[idx];
            let path = fresh_list.paths[idx].clone();
            let size = fresh_list.size[idx].clone();
            let item = fresh_list.items[idx].clone();
            if !case_sensitive {
                name = name.to_lowercase();
            }
            if let EntryType::SpecialSign = entry_type {
                new_list.push(path, name, entry_type, size, item);
                continue;
            } else if name.starts_with(&curated_search_bar_text) {
                new_list.push(path, name, entry_type, size, item);
            }
        }

        self.elements = new_list;
    }

    /*
    pub fn add_top_priority_entry(&mut self, entry: GallideEntry) {
        let previous_directory = self.elements.pop_front().unwrap();
        self.elements.push_front(entry);
        self.elements.push_front(previous_directory);
    }*/

    pub fn rebuild_directories(&mut self) {
        // self.elements = self.get_current_dir_folder_contents();
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

    pub fn current_directory(&self) -> &Path {
        &self.current_dir
    }

    pub fn selected_path(&self) -> &Path {
        &self.elements.paths[self.cursor()]
    }

    pub fn open_selected_directory(&mut self) {
        if !self.is_selecting_directory() {
            self.stop();
            return;
        }
        self.current_dir = self.elements.paths[self.cursor()].to_path_buf();
        self.search_bar_text.clear();
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
                &self.current_dir
            } else {
                self.selected_path()
            }
            .display()
        )
    }

    pub fn is_selecting_directory(&self) -> bool {
        if let EntryType::Folder = self.elements.types[self.cursor] {
            return true;
        }
        false
    }

    pub fn ask_input(&mut self, config: user_input::UserInputRequest) {
        self.question_box_text.clear();
        self.user_input_request = config;
        self.mode = Mode::WRITING;
    }
    pub fn is_in_write_mode(&self) -> bool {
        matches!(self.mode, Mode::DISCARD | Mode::WRITING,)
    }

    pub fn selecting_previous_dir(&self) -> bool {
        self.cursor() == 0
    }

    pub fn remove_selected(&mut self) {
        let index = self.cursor();
        // self.elements.remove(index); // TODO
        self.cursor = std::cmp::min(0, index - 1);
    }
    pub fn show_message(&mut self, title: &str, info: &str) {
        self.question_box_text.value = String::from(info);
        self.user_input_request.title = String::from(title);
        self.mode = Mode::DISCARD;
    }
}
