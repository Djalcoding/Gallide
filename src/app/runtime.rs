use std::time::Duration;

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
                    self.toggle_insert_mode()
                }
            }
            KeyCode::Char('c') => self.search_bar_text.clear(),
            KeyCode::Char('a') => {
                //self.ask_input(create_ressource_request());
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
                    .saturating_add(c.to_digit(10).unwrap() as u16)
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

    fn handle_write_mode_input(&mut self, _key: KeyEvent) {
        /*match key.code {
            KeyCode::Esc => self.mode = Mode::SELECTING,
            KeyCode::Char('\n') => {
                let user_input = self.question_box_text.clone();
                let request = self.user_input_request.get_closure();

                let result: UserOperationResult;
                if self.user_input_request.edit_ressource && self.selecting_previous_dir() {
                    result = UserOperationResult::operation_on_prev();
                } else {
                    result = request(self, &user_input);
                }

                if let Some(message) = result.message {
                    //self.show_message(result.exit.get_header(), &message);
                } else {
                    self.mode = Mode::SELECTING;
                }
            }
            KeyCode::Char(character) => self.question_box_text.push(character),
            KeyCode::Backspace => {
                self.question_box_text.pop();
            }
            _ => {}
        }*/
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        while self.is_running() {
            terminal.draw(|frame| self.render(frame))?;
            if event::poll(Duration::from_millis(50))?
                && let Key(key) = event::read()?
            {
                match self.mode {
                    Mode::INSERT => self.handle_insert_mode_input(key),
                    Mode::SELECTING => self.handle_select_mode_input(key),
                    #[allow(clippy::needless_late_init)]
                    Mode::WRITING => self.handle_write_mode_input(key),
                    Mode::DISCARD => {
                        self.mode = Mode::SELECTING;
                    }
                }
            }
        }
        Ok(())
    }
}
