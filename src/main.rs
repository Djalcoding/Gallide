use std::{
    env,
    io::{self, stdin},
    path::Path,
    sync::mpsc,
    thread,
};

use gallide_bin::{
    config::*,
    read_ls::{get_absolute_path_from_str, get_folder_contents},
    reporter::Reporter,
    ui,
    ui_brain::State,
};
use termion::{
    event::Key,
    input::TermRead,
    raw::IntoRawMode,
    screen::{ToAlternateScreen, ToMainScreen},
};
use tui::{Terminal, backend::TermionBackend};

fn main() -> Result<(), io::Error> {
    let mut reporter = Reporter::new();
    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let args: Vec<String> = env::args().collect();
    let config_path: Option<&Path> = if args.len() == 1 {
        None
    } else {
        Some(Path::new(&args[1]))
    };
    let config = if let Some(path) = config_path {
        Config::from_file(path).unwrap_or_else(|e| {
            reporter.push(format!("Could not parse config : {e}",).as_str());
            Config::default()
        })
    } else {
        Config::default()
    };
    let directories =
        get_folder_contents(get_absolute_path_from_str(".").to_str().unwrap_or_else(|| {
            reporter.push("Invalid UTF8 in start location");
            ""
        }))
        .unwrap_or_else(|e| {
            reporter.push(format!("could not get start location folder contents : {e}").as_str());
            vec![]
        });
    let enable_searchbar = config.search_bar.enabled;
    let mut state = State::new(directories, config, reporter);
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
            ui::build_ui(f, &state, state.get_config());
        });
        if let Ok(key) = rx.recv() {
            if state.is_inserting() {
                // Inserting
                match key {
                    Key::Esc | Key::Char('\n') => state.switch_mode(),
                    Key::Backspace => state.backspace(),
                    Key::Char(character) => state.add_character(character),
                    Key::Up => state.decrement_selected_box(),
                    Key::Down => state.increment_selected_box(),
                    _ => {}
                }
            } else {
                // Selecting
                match key {
                    Key::Up | Key::Char('k') => state.decrement_selected_box(),
                    Key::Down | Key::Char('j') => state.increment_selected_box(),

                    Key::Esc | Key::Char('q') => {
                        state.stop();
                        exited = true;
                    }
                    Key::Right | Key::Char('l') => {
                        if state.is_selecting_directory() {
                            state.open_selected_directory();
                            state.rebuild_directories();
                        } else {
                            state.stop();
                        }
                    }
                    Key::Char('\n') => state.stop(),
                    Key::Left | Key::Char('h') => {
                        state.go_back_one_directory();
                        state.rebuild_directories();
                    }
                    Key::Char('i') => {
                        if enable_searchbar {
                            state.switch_mode()
                        }
                    }
                    Key::Char('c') => state.clear_search_bar(),
                    _ => {}
                }
            }
        }
    }
    println!("{ToMainScreen}");
    state.publish_reports();
    eprintln!("{}", state.get_bash_string(exited));
    Ok(())
}
