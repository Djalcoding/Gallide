use std::{
    io::{self, stdin},
    sync::mpsc,
    thread,
};

use gallide::{
    config::*, progress_bar, read_ls::{get_absolute_path_from_str, get_folder_contents}, ui, ui_brain::State
};
use termion::{
    event::Key,
    input::TermRead,
    raw::IntoRawMode,
    screen::{ToAlternateScreen, ToMainScreen},
};
use tui::{Terminal, backend::TermionBackend, symbols::braille};

fn main() -> Result<(), io::Error> {
    let stdout = io::stderr().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let config = Config::default();
    let directories = get_folder_contents(
        get_absolute_path_from_str(".").to_str().unwrap(),
        config.directory_symbol(),
        config.file_symbol()
    ).unwrap();
    let mut state = State::new(directories, config);
    println!("{ToAlternateScreen}");

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let stdin = stdin();
        for c in stdin.keys().flatten() {
            let _ = tx.send(c);
        }
    });

    let mut exited = false;

    while state.is_running() {
        let _ = terminal.draw(|f| {
            ui::build_ui(f, &state, Config::default());
        });
        if let Ok(key) = rx.recv() {
            if state.is_inserting() {
                // Inserting
                match key {
                    Key::Esc | Key::Char('\n') => state.switch_mode(),
                    Key::Backspace => state.backspace(),
                    Key::Char(character) => state.add_character(character),
                    Key::Up => {
                        state.decrement_selected_box();
                    }
                    Key::Down => {
                        state.increment_selected_box();
                    }
                    _ => {}
                }
            } else {
                // Selecting
                match key {
                    Key::Up | Key::Char('k') => {
                        state.decrement_selected_box();
                    }
                    Key::Down | Key::Char('j') => {
                        state.increment_selected_box();
                    }
                    Key::Esc | Key::Char('q') => {
                        state.stop();
                        exited = true;
                    }
                    Key::Right | Key::Char('l') => {
                        state.open_selected_directory();
                        state.rebuild_directories();
                    }
                    Key::Char('\n') => {
                        state.stop();
                    }
                    Key::Left | Key::Char('h') => {
                        state.go_back_one_directory();
                        state.rebuild_directories();
                    }
                    Key::Char('i') => state.switch_mode(),
                    Key::Char('c') => state.clear_search_bar(),
                    _ => {}
                }
            }
        }
    }
    println!("{ToMainScreen}");
    println!("{}", state.get_bash_string(exited));
    Ok(())
}
