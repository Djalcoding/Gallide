pub mod user_input;

use crate::{
    app::render::Screen,
    config::Config,
    explore::{
        EntryType, GallideEntry, GallideEntryVec,
        tree::hierarchy::{FileHierarchy, SafeNode},
    },
};
use std::{path::PathBuf, time::Instant};

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
    cursor: usize,
    mode: Mode,
}

pub struct App {
    screen: Screen,
    file_hierarchy: FileHierarchy,
    user_state: UserState,
    curr_node: SafeNode,
    curr_list: GallideEntryVec,
    config: Config,
    running: bool,
    exited: bool,
    next_update: Instant,
}

impl App {
    pub const fn page_size(&self) -> usize {
        1000
    }
    pub fn new(config: Config) -> Self {
        let file_hierarchy = FileHierarchy::new().unwrap(); // This starts a worker thread
        let mut app = App {
            user_state: UserState {
                cursor: 0,
                jump_buffer: 0,
                depth: config.main_box.default_depth,
                search_bar_text: String::new(),
                mode: Mode::SELECTING,
                question_box_text: String::new(),
            },
            curr_node: file_hierarchy.root.clone(),
            curr_list: GallideEntryVec::new(),
            file_hierarchy,
            running: true,
            exited: true,
            screen: Screen::start(&config),
            next_update: Instant::now(),
            config,
        };
        app.user_state.search_bar_text.reserve(200);
        app.user_state.question_box_text.reserve(200);
        app.rebuild_directories();

        app.screen
            .build_search_bar(false, String::new(), &app.config.search_bar);
        app.screen
            .build_tooltips(&app.config.tooltips, app.config.main_box.entry_style);
        app
    }

    pub fn selected_index(&self) -> usize {
        self.user_state.cursor % self.page_size()
    }

    pub fn current_directory(&self) -> PathBuf {
        self.curr_node.path().to_path_buf()
    }

    pub fn selected_entry(&self) -> &GallideEntry {
        &self.curr_list[self.selected_index()]
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
        self.user_state.cursor += 1;
        if self.user_state.cursor != 0
            && self.user_state.cursor.is_multiple_of(self.page_size())
            && !self.rebuild_directories()
            || self.selected_index() >= self.curr_list.len()
        {
            self.user_state.cursor = 0;
            self.rebuild_directories();
        }
    }

    pub fn move_up(&mut self) {
        self.user_state.cursor -= 1;
        if self.user_state.cursor != 0 && self.user_state.cursor.is_multiple_of(self.page_size()) {
            self.rebuild_directories();
        }
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

    ///Returns {true} if there are directories to build
    pub fn rebuild_directories(&mut self) -> bool {
        /*self.curr_list = self.file_hierarchy.get_entries(
            &self.curr_node,
            "",
            (self.user_state.cursor / self.page_size()) * self.page_size(),
            self.user_state.depth as usize,
        );*/

        !self.curr_list.is_empty()
    }

    pub fn go_back_one_directory(&mut self) {
        self.file_hierarchy.step_back();
        let parent = self.curr_node.read().parent.clone();
        if let Some(dad) = parent {
            self.curr_node = dad.upgrade().unwrap(); // TODO : check this
        }
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
        self.screen.build_search_bar(
            true,
            self.user_state.search_bar_text.clone(),
            &self.config.search_bar,
        );
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
        self.user_state.search_bar_text.clear();
        self.user_state.cursor = 0;
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
                self.current_directory()
            } else {
                self.curr_list[self.user_state.cursor]
                    .node
                    .path()
                    .to_path_buf()
            }
            .to_string_lossy()
        )
    }
}
