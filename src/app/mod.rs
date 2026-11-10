pub mod user_input;

use crate::{
    app::render::Screen,
    config::Config,
    explore::{
        EntryType, GallideEntry, GallideEntryVec, PageBounds,
        tree::hierarchy::{EntryCreationContext, FileHierarchy, SafeNode},
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
    depth: u8,
    mode: Mode,
    jump_buffer: usize,
    cursor: usize,
    page_bounds: PageBounds,
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
                cursor: 0,
                page_bounds: PageBounds(0, 1),
                jump_buffer: 0,
                depth: config.main_box.default_depth,
                search_bar_text: String::new(),
                mode: Mode::SELECTING,
                question_box_text: String::new(),
            },
            curr_node: file_hierarchy.root.clone(),
            curr_list: GallideEntryVec::with_capacity(config.behavior.page_size),
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
        app.rebuild_directories(false);

        app.screen
            .build_search_bar(false, String::new(), &app.config.search_bar);
        app.screen
            .build_tooltips(&app.config.tooltips, app.config.main_box.entry_style);
        app
    }

    pub fn selected_index(&self) -> usize {
        self.user_state
            .cursor
            .saturating_sub(self.user_state.page_bounds.page_start_index())
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
    pub fn move_down(&mut self) {
        let on_last_entry = self.user_state.cursor == self.user_state.page_bounds.last_index();
        let on_last_page = self.user_state.page_bounds.last_page();
        self.user_state.cursor += std::cmp::max(1, self.user_state.jump_buffer);
        self.user_state.jump_buffer = 0;
        if self.user_state.cursor >= self.user_state.page_bounds.page_end_index() {
            if on_last_page && on_last_entry {
                self.user_state.cursor = 0;
            } else if on_last_page {
                self.user_state.cursor = self.user_state.page_bounds.last_index();
            }
            self.rebuild_directories(false);
        }
    }

    pub fn move_up(&mut self) {
        let at_start = self.user_state.cursor == 0;
        self.user_state.cursor = self
            .user_state
            .cursor
            .saturating_sub(std::cmp::max(1, self.user_state.jump_buffer));
        self.user_state.jump_buffer = 0;
        if at_start || self.user_state.cursor < self.user_state.page_bounds.page_start_index() {
            self.rebuild_directories(at_start);
        }
    }

    pub fn increase_depth(&mut self) {
        self.user_state.depth = self.user_state.depth.saturating_add(1);
        self.rebuild_directories(false);
    }
    pub fn decrease_depth(&mut self) {
        if self.user_state.depth != 1 {
            self.user_state.depth -= 1;
        }
        self.rebuild_directories(false);
    }
    pub(super) fn rebuild_directories(&mut self, goto_end: bool) {
        let page_start = (self.user_state.cursor / self.config.behavior.page_size)
            * self.config.behavior.page_size;
        let mut config = EntryCreationContext {
            whitelist: &self.user_state.search_bar_text,
            max_depth: self.user_state.depth as usize,
            skips: page_start.saturating_sub(1),
            config: &self.config.behavior,
            explore_fully: goto_end,
        };
        if goto_end {
            config.skips = 0;
        }
        let selected_node = self.selected_entry().map(|entry| entry.node.clone());
        let (spins, end_position) = self.file_hierarchy.get_entries(
            &mut self.curr_list,
            &self.curr_node,
            selected_node,
            config,
        );
        self.user_state.page_bounds.0 = spins * self.config.behavior.page_size;
        self.user_state.page_bounds.1 =
            (end_position.abs() + self.user_state.page_bounds.0 as isize) * end_position.signum();
        if goto_end {
            self.user_state.cursor = self.user_state.page_bounds.last_index();
        }
        self.prev_entry_count = self.available_entries();
        if self.curr_list.is_empty() {
            self.rebuild_directories(true);
        }
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
        self.rebuild_directories(false);
    }

    pub fn backspace(&mut self) {
        self.user_state.search_bar_text.pop();
        self.rebuild_searchbar();
        self.rebuild_directories(false);
    }

    pub fn add_character(&mut self, character: char) {
        self.user_state.search_bar_text.push(character);
        self.rebuild_searchbar();
        self.rebuild_directories(false);
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
        self.curr_node = match self.selected_entry() {
            Some(entry) => entry.node.clone(),
            None => self.curr_node.clone(),
        };
        self.user_state.cursor = 0;
        self.rebuild_directories(false);
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
