use std::{
    io::{self, stdin, Write}, process, sync::mpsc, thread
};

use fzf_clone::{read_ls::get_directories, ui};
use termion::{
    event::Key,
    input::TermRead,
    raw::IntoRawMode,
    screen::{ToAlternateScreen, ToMainScreen},
};
use tui::{Terminal, backend::TermionBackend};

fn main() -> Result<(), io::Error> {
    let stdout = io::stderr().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let directories = get_directories();
    let mut selected_id: usize = 0;
    let mut running: bool = true;

    let mut directory_name:String = String::from(".");
    println!("{ToAlternateScreen}");
    
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let stdin = stdin();
        for c in stdin.keys().flatten() {
            let _ =tx.send(c);
        }
    });

    while running {        
        let _ = terminal.draw(|f| {
            ui::build_ui(f, selected_id, &directories);
        });
        if let Ok(key) = rx.recv() {
            match key {
                Key::Up | Key::Char('k') => {
                    if selected_id == 0 {
                        selected_id = directories.len()-1;
                    }
                    else{ selected_id -= 1;}
                }
                Key::Down | Key::Char('j') => {
                    selected_id = (selected_id + 1) % directories.len();
                }
                Key::Char('q') => {
                    println!("{ToMainScreen}");
                    process::exit(0);
                }
                Key::Right | Key::Char('l') | Key::Char('\n')=>{
                    directory_name = directories[selected_id].clone();
                    running = false;
                }
                _ => {} 
            }
        }

    }
    print!("{ToMainScreen}");
    println!(" {directory_name}");

    Ok(())
}
