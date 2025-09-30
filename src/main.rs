use std::{
    io::{self, stdin},
    sync::mpsc,
    thread,
};

use gallide::{
    read_ls::get_directories,
    ui::{self, Config, State},
};
use termion::{
    event::Key,
    input::TermRead,
    raw::IntoRawMode,
    screen::{ToAlternateScreen, ToMainScreen},
};
use tui::{Terminal, backend::TermionBackend, style::Color};

fn main() -> Result<(), io::Error> {
    let stdout = io::stderr().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let directories = get_directories();

    let mut state = State::new(directories);

    let mut directory_name: String = String::from(".");
    println!("{ToAlternateScreen}");

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let stdin = stdin();
        for c in stdin.keys().flatten() {
            let _ = tx.send(c);
        }
    });

    while state.is_running() {
        let _ = terminal.draw(|f| {
            ui::build_ui(f, &state, Config::from(Color::Red, Color::White, 7));
        });
        if let Ok(key) = rx.recv() {

            
            if state.is_inserting() { // Inserting
                match key {
                    Key::Esc | Key::Char('\n')=> state.switch_mode(),
                    Key::Backspace => state.backspace(),
                    Key::Char(character) =>{ state.add_character(character)}
                    Key::Up => {
                        state.decrement_selected_box();
                    }
                    Key::Down =>{
                        state.increment_selected_box();
                    }
                    _ => {} 
                }

            } else { // Selecting
                match key {
                    Key::Up | Key::Char('k') => {
                        state.decrement_selected_box();
                    }
                    Key::Down | Key::Char('j') => {
                        state.increment_selected_box();
                    }
                    Key::Esc | Key::Char('q') => {
                        directory_name = String::from('.');
                        state.stop();
                    }
                    Key::Right | Key::Char('l') | Key::Char('\n') => {
                        directory_name = state.get_selected_directory();
                        state.stop();
                    }
                    Key::Left | Key::Char('h') => {
                        directory_name = String::from("..");
                        state.stop();
                    }
                    Key::Char('i') => state.switch_mode(),
                    Key::Char('c') => state.clear_search_bar(),
                    _ => {}
                }
            }


        }
    }
    println!("{ToMainScreen}");
    println!(" |{directory_name}|");
    Ok(())
}
