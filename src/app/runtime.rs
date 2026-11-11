use std::time::{Duration, Instant};

use crossterm::event::{self, Event::Key, KeyCode, KeyEvent, KeyModifiers};
use ratatui::DefaultTerminal;

use crate::app::{App, Mode};

impl App {
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
                } else {
                    self.stop();
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.go_back_one_directory();
            }
            KeyCode::Enter => self.stop(),
            KeyCode::Char('i') => {
                if self.config.search_bar.enabled {
                    self.toggle_insert_mode()
                }
            }
            KeyCode::PageUp => {
                self.user_state.cursor = 0;
                self.rebuild_directories(false);
            }
            KeyCode::PageDown => {
                self.rebuild_directories(true);
            }
            KeyCode::Char('+') => {
                self.increase_depth();
            }
            KeyCode::Char('-') => {
                self.decrease_depth();
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                self.user_state.jump_buffer = self.user_state.jump_buffer.saturating_mul(10);
                self.user_state.jump_buffer = self
                    .user_state
                    .jump_buffer
                    .saturating_add(c.to_digit(10).unwrap() as usize)
            }
            _ => {}
        }
    }

    fn handle_insert_mode_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => self.toggle_insert_mode(),
            KeyCode::Backspace => self.backspace(),
            KeyCode::Char(character) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.add_character(character)
            }
            _ => {
                self.handle_select_mode_input(key);
            }
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        while self.is_running() {
            if Instant::now() > self.next_update {
                if self.available_entries() != self.prev_entry_count {
                    self.rebuild_directories(false);
                }
                self.next_update = Instant::now() + Duration::from_millis(100); // TODO check if I
                // can increase this
            }
            if let Some(timestamp) = self.next_text_related_update
                && timestamp < Instant::now()
            {
                self.user_state.cursor = 1;
                self.rebuild_directories(false);
                if self.curr_list.len() == 1 {
                    self.user_state.cursor = 0;
                }
                self.next_text_related_update = None;
            }
            terminal.draw(|frame| self.render(frame))?;
            if event::poll(Duration::from_millis(50))?
                && let Key(key) = event::read()?
            {
                match self.user_state.mode {
                    Mode::INSERT => self.handle_insert_mode_input(key),
                    Mode::SELECTING => self.handle_select_mode_input(key),
                }
            }
        }
        Ok(())
    }
}
