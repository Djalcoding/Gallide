pub mod user_input;

use crate::{
    app::render::Screen,
    config::Config,
    explore::{
        EntryType, GallideEntry, GallideEntryVec, Page,
        tree::hierarchy::{EntryCreationConfig, FileHierarchy, SafeNode},
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
    pinned: bool,
    depth: u8,
    mode: Mode,
    current_page: Page,
    jump_buffer: usize,
    cursor: usize,
    search_bar_text: String,
    question_box_text: String,
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
    prev_entry_count: usize,
}

impl App {
    pub fn new(config: Config) -> Self {
        let file_hierarchy = FileHierarchy::new(&config.behavior).unwrap(); // This starts a worker thread
        let mut app = App {
            user_state: UserState {
                pinned: false,
                cursor: 0,
                jump_buffer: 0,
                depth: config.main_box.default_depth,
                search_bar_text: String::new(),
                mode: Mode::SELECTING,
                current_page: Page::default(),
                question_box_text: String::new(),
            },
            curr_node: file_hierarchy.root.clone(),
            curr_list: GallideEntryVec::from([GallideEntry::previous_directory(
                &file_hierarchy.root,
            )]),
            file_hierarchy,
            running: true,
            exited: false,
            screen: Screen::start(&config),
            next_update: Instant::now(),
            prev_entry_count: 0,
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
        self.user_state.cursor % self.user_state.current_page.size()
    }

    pub fn current_directory(&self) -> PathBuf {
        self.curr_node.path().to_path_buf()
    }

    pub fn selected_entry(&self) -> Option<&GallideEntry> {
        let index = self.selected_index();
        if index < self.curr_list.len() {
            Some(&self.curr_list[self.selected_index()])
        } else {
            None
        }
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
        if let Some(entry) = self.selected_entry() {
            matches!(entry.entry_type, EntryType::Folder)
        } else {
            false
        }
    }

    // TODO move this and else to UserState
    pub fn move_down(&mut self) {
        self.user_state.pinned = false;
        let on_end = self.on_list_end();
        self.user_state.cursor += std::cmp::max(1, self.user_state.jump_buffer);
        self.user_state.jump_buffer = 0;
        if on_end {
            self.user_state.cursor = 0;
        }
        if !self
            .user_state
            .current_page
            .contains(self.user_state.cursor)
        {
            self.rebuild_directories();
        }
    }

    pub fn move_up(&mut self) {
        self.user_state.pinned = false;
        let on_start = self.user_state.cursor == 0;
        self.user_state.cursor = self
            .user_state
            .cursor
            .saturating_sub(std::cmp::max(1, self.user_state.jump_buffer));
        self.user_state.jump_buffer = 0;
        if on_start {
            self.user_state.pinned = true;
            self.rebuild_directories();
            return;
        }
        if !self
            .user_state
            .current_page
            .contains(self.user_state.cursor)
        {
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
    pub(super) fn rebuild_directories(&mut self) {
        let config = EntryCreationConfig {
            whitelist: &self.user_state.search_bar_text,
            curr_page: &self.user_state.current_page,
            max_depth: self.user_state.depth as usize,
        };
        let selected_node = self.selected_entry().map(|entry| entry.node.clone());
        self.user_state.current_page = self.file_hierarchy.get_entries(
            &mut self.curr_list,
            &self.curr_node,
            selected_node,
            config,
        );
        self.prev_entry_count = self.available_entries();
    }

    pub fn available_entries(&self) -> usize {
        // check if I can use my variable
        self.curr_node
            .read()
            .get_nested_entries(self.user_state.depth as usize)
            - 1
    }

    pub fn go_back_one_directory(&mut self) {
        let weak_parent = self.curr_node.read().parent.clone();
        let new_root = match weak_parent {
            Some(node) => node.upgrade().unwrap().clone(),
            None => self.file_hierarchy.step_back(),
        };
        self.next_update = Instant::now();
        self.curr_node = new_root;
        self.user_state.cursor = 1;
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

    fn on_list_end(&self) -> bool {
        self.user_state.current_page.is_end()
            && self.user_state.cursor == self.user_state.current_page.end()
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
        self.curr_node = match self.selected_entry() {
            Some(entry) => entry.node.clone(),
            None => self.curr_node.clone(),
        };
        self.user_state.cursor = 0;
        self.rebuild_directories();
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
