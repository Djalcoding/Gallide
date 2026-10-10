pub mod user_input;

use ratatui::widgets::ListState;

use crate::{
    app::render::Screen,
    config::Config,
    explore::{
        EntryType, GallideEntry, GallideEntryVec, build_entry_list, tree::hierarchy::FileHierarchy,
    },
};
use std::{path::{Path, PathBuf}, time::Instant};

mod render;
mod runtime;

#[derive(PartialEq, Clone)]
pub enum Mode {
    INSERT,
    WRITING,
    SELECTING,
    DISCARD,
}

pub struct UserState {
    jump_buffer: u16,
    depth: u8,
    search_bar_text: String,
    question_box_text: String,
    cursor: ListState,
    current_dir: PathBuf,
    mode: Mode,
}

pub struct App {
    screen: Screen,
    elements: GallideEntryVec,
    file_hierarchy: FileHierarchy,
    user_state: UserState,
    config: Config,
    running: bool,
    exited: bool,
    last_update: Instant
}

impl App {
    pub fn new(config: Config) -> Self {
        let start = Path::new(".").canonicalize().unwrap();
        let mut app = App {
            file_hierarchy: FileHierarchy::new().unwrap(), // This starts a worker thread
            user_state: UserState {
                cursor: ListState::default().with_selected(Some(0)),
                jump_buffer: 0,
                depth: config.main_box.default_depth,
                search_bar_text: String::new(),
                mode: Mode::SELECTING,
                current_dir: start,
                question_box_text: String::new(),
            },
            running: true,
            exited: true,
            elements: GallideEntryVec::new(),
            screen: Screen::start(&config),
            last_update: Instant::now(),
            config,
        };
        app.user_state.search_bar_text.reserve(200);
        app.user_state.question_box_text.reserve(200);
        app.rebuild_directories();

        app.screen
            .build_search_bar(false, String::new(), &app.config.search_bar);
        app.screen
            .build_tooltips(&app.config.tooltips, app.config.main_box.entry_style);

        if app.elements.len() > 1 {
            app.user_state.cursor.select(Some(1));
        }
        app
    }

    pub fn cursor(&self) -> usize {
        self.user_state.cursor.selected().unwrap_or(0)
    }

    pub fn current_directory(&self) -> &Path {
        &self.user_state.current_dir
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
        if let Mode::INSERT = &self.user_state.mode {
            return true;
        }
        false
    }

    pub fn is_in_write_mode(&self) -> bool {
        matches!(self.user_state.mode, Mode::DISCARD | Mode::WRITING,)
    }

    pub fn is_selecting_directory(&self) -> bool {
        matches!(self.selected_entry().entry_type, EntryType::Folder)
    }

    pub fn move_down(&mut self) {
        if self.user_state.cursor.selected().unwrap() == self.elements.len() - 1 {
            self.user_state.cursor.select_first();
            return;
        }
        self.user_state.cursor
            .scroll_down_by(std::cmp::max(1, self.user_state.jump_buffer));
        if self.user_state.cursor.selected().unwrap() >= self.elements.len() {
            self.user_state.cursor.select(Some(self.elements.len() - 1));
        }
        self.user_state.jump_buffer = 0;
    }

    pub fn move_up(&mut self) {
        if self.user_state.cursor.selected().unwrap() == 0 {
            self.user_state.cursor.select(Some(self.elements.len() - 1));
            return;
        }
        self.user_state.cursor.scroll_up_by(std::cmp::max(1, self.user_state.jump_buffer));
        self.user_state.jump_buffer = 0;
    }

    pub fn increase_depth(&mut self) {
        self.user_state.depth = self.user_state.depth.saturating_add(1);
        self.rebuild_directories();
    }
    pub fn decrease_depth(&mut self) {
        if self.user_state.depth != 1 {
            self.user_state.depth -= 1;
        }
        self.rebuild_directories();
    }

    pub fn rebuild_directories(&mut self) {
        self.elements.clear();
        let highlights: Vec<u16> = Vec::new();
        self.elements = self.file_hierarchy.get_entries(
            self.current_directory(),
            &self.user_state.search_bar_text,
            self.user_state.depth as u16,
            false,
        );
        self.user_state.cursor.select(Some(0));
        self.screen.update_main_box(build_entry_list(
            &self.elements,
            highlights,
            &self.config.main_box,
        ));
    }

    pub fn add_top_priority_entry(&mut self, entry: GallideEntry) {
        let previous_directory = self.elements.pop_front().unwrap();
        self.elements.push_front(entry);
        self.elements.push_front(previous_directory);
    }

    pub fn go_back_one_directory(&mut self) {
        self.user_state.current_dir.pop();
        self.file_hierarchy.go_back();
        self.rebuild_directories();
    }

    pub fn backspace(&mut self) {
        self.user_state.search_bar_text.pop();
        self.rebuild_searchbar();
        self.rebuild_directories();
    }

    pub fn add_character(&mut self, character: char) {
        self.user_state.search_bar_text.push(character);
        self.rebuild_searchbar();
        self.rebuild_directories();
    }

    fn rebuild_searchbar(&mut self) {
        self.screen
            .build_search_bar(true, self.user_state.search_bar_text.clone(), &self.config.search_bar);
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn toggle_insert_mode(&mut self) {
        self.user_state.mode = if self.is_inserting() {
            Mode::SELECTING
        } else {
            Mode::INSERT
        };
        self.screen.build_search_bar(
            self.is_inserting(),
            self.user_state.search_bar_text.clone(),
            &self.config.search_bar,
        );
    }

    pub fn open_selected_directory(&mut self) {
        if !self.is_selecting_directory() {
            self.stop();
            return;
        }
        self.user_state.current_dir = self.selected_path().to_path_buf();
        self.user_state.search_bar_text.clear();
        self.user_state.cursor.select_first();
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
                &self.user_state.current_dir
            } else {
                self.selected_path()
            }
            .display()
        )
    }
}
