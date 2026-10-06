pub mod user_input;

use ratatui::widgets::ListState;

use crate::{
    app::render::Screen,
    config::Config,
    explore::{EntryType, GallideEntry, GallideEntryVec, build_entry_list, get_folder_contents},
};
use std::path::{Path, PathBuf};

mod render;
mod runtime;

#[derive(PartialEq, Clone)]
pub enum Mode {
    INSERT,
    WRITING,
    SELECTING,
    DISCARD,
}

pub struct App {
    cursor: ListState,
    screen: Screen,
    jump_buffer: u16,
    depth: u8,
    elements: GallideEntryVec,
    search_bar_text: String,
    question_box_text: String,
    current_dir: PathBuf,
    config: Config,
    mode: Mode,
    running: bool,
    exited: bool,
}

impl App {
    pub fn new(config: Config) -> Self {
        let start = Path::new(".").canonicalize().unwrap();
        let mut app = App {
            cursor: ListState::default().with_selected(Some(0)),
            jump_buffer: 0,
            depth: config.main_box.default_depth,
            search_bar_text: String::new(),
            running: true,
            exited: true,
            elements: get_folder_contents(&start, config.main_box.default_depth).unwrap(),
            current_dir: start,
            mode: Mode::SELECTING,
            screen: Screen::start(&config),
            config,
            question_box_text: String::new(),
        };
        app.search_bar_text.reserve(200);
        app.question_box_text.reserve(200);
        app.rebuild_directories();

        app.screen
            .build_search_bar(false, String::new(), &app.config.search_bar);
        app.screen
            .build_tooltips(&app.config.tooltips, app.config.main_box.entry_style);

        if app.elements.len() > 1 {
            app.cursor.select(Some(1));
        }
        app
    }

    pub fn cursor(&self) -> usize {
        self.cursor.selected().unwrap_or(0)
    }

    pub fn current_directory(&self) -> &Path {
        &self.current_dir
    }

    pub fn selected_entry(&self) -> &GallideEntry {
        &self.elements[self.cursor()]
    }

    pub fn selected_path(&self) -> &Path {
        self.selected_entry().path()
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn is_inserting(&self) -> bool {
        if let Mode::INSERT = &self.mode {
            return true;
        }
        false
    }

    pub fn is_in_write_mode(&self) -> bool {
        matches!(self.mode, Mode::DISCARD | Mode::WRITING,)
    }

    pub fn is_selecting_directory(&self) -> bool {
        matches!(self.selected_entry().entry_type, EntryType::Folder)
    }

    pub fn move_down(&mut self) {
        if self.cursor.selected().unwrap() == self.elements.len() - 1 {
            self.cursor.select_first();
            return;
        }
        self.cursor
            .scroll_down_by(std::cmp::max(1, self.jump_buffer));
        if self.cursor.selected().unwrap() >= self.elements.len() {
            self.cursor.select(Some(self.elements.len() - 1));
        }
        self.jump_buffer = 0;
    }

    pub fn move_up(&mut self) {
        if self.cursor.selected().unwrap() == 0 {
            self.cursor.select(Some(self.elements.len() - 1));
            return;
        }
        self.cursor.scroll_up_by(std::cmp::max(1, self.jump_buffer));
        self.jump_buffer = 0;
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

    pub fn rebuild_directories(&mut self) {
        self.elements.clear();
        let mut search_bar_text = String::from(self.search_bar_text.trim());
        if !self.config.case_sensitive {
            search_bar_text = search_bar_text.to_lowercase();
        }
        let mut highlights:Vec<u16> = Vec::new();
        let fresh_list = get_folder_contents(&self.current_dir, self.depth).unwrap(); // TODO candy
        for entry in fresh_list {
            let mut matches = false;
            for part in entry.name().split('/') {
                let part = if self.config.case_sensitive {
                    part
                } else {
                    &part.to_lowercase()
                };
                if part.starts_with(&search_bar_text) {
                    matches = true;
                    break;
                }
            }
            if let EntryType::SpecialSign = entry.entry_type {
                highlights.push(search_bar_text.len() as u16);
                highlights.push(0);
            } else if matches {
                highlights.push(search_bar_text.len() as u16);
                self.elements.push_back(entry);
            } else {
                highlights.push(0);
            }
        }
        self.cursor.select(Some(0));
        self.screen
            .update_main_box(build_entry_list(&self.elements, highlights, &self.config.main_box));
    }

    pub fn add_top_priority_entry(&mut self, entry: GallideEntry) {
        let previous_directory = self.elements.pop_front().unwrap();
        self.elements.push_front(entry);
        self.elements.push_front(previous_directory);
    }

    pub fn go_back_one_directory(&mut self) {
        self.current_dir.pop();
        self.rebuild_directories();
    }

    pub fn backspace(&mut self) {
        self.search_bar_text.pop();
        self.rebuild_searchbar();
        self.rebuild_directories();
    }

    pub fn add_character(&mut self, character: char) {
        self.search_bar_text.push(character);
        self.rebuild_searchbar();
        self.rebuild_directories();
    }

    fn rebuild_searchbar(&mut self) {
        self.screen
            .build_search_bar(true, self.search_bar_text.clone(), &self.config.search_bar);
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn toggle_insert_mode(&mut self) {
        self.mode = if self.is_inserting() {
            Mode::SELECTING
        } else {
            Mode::INSERT
        };
        self.screen.build_search_bar(
            self.is_inserting(),
            self.search_bar_text.clone(),
            &self.config.search_bar,
        );
    }

    pub fn open_selected_directory(&mut self) {
        if !self.is_selecting_directory() {
            self.stop();
            return;
        }
        self.current_dir = self.selected_path().to_path_buf();
        self.search_bar_text.clear();
        self.cursor.select_first();
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
}
