use core::time;
use std::{
    env,
    io::{self, stdin},
    path::Path,
    sync::mpsc,
    thread,
};

use gallide_bin::{
    config::*,
    file_control::{create_ressource_request, delete_ressource_request, rename_ressource_request},
    reporter::Reporter,
    ui,
    ui_brain::{Mode, State, user_input::UserOperationResult},
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
    let enable_searchbar = config.search_bar.enabled;
    let mut state = State::new(config, reporter);
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
        if let Ok(key) = rx.recv_timeout(time::Duration::from_millis(50)) {
            let mode = state.mode.clone();
            match mode {
                Mode::INSERT => match key {
                    Key::Esc | Key::Char('\n') => state.switch_mode(),
                    Key::Backspace => state.backspace(),
                    Key::Char(character) => state.add_character(character),
                    Key::Up => state.decrement_selected_box(),
                    Key::Down => state.increment_selected_box(),
                    _ => {}
                },
                Mode::SELECTING => match key {
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
                    Key::Left | Key::Char('h') => {
                        state.go_back_one_directory();
                        state.rebuild_directories();
                    }
                    Key::Char('\n') => state.stop(),
                    Key::Char('i') => {
                        if enable_searchbar {
                            state.switch_mode()
                        }
                    }
                    Key::Char('c') => state.clear_search_bar(),
                    Key::Char('a') => {
                        state.ask_input(create_ressource_request());
                    }
                    Key::Char('d') => {
                        state.ask_input(delete_ressource_request(state.read_selected_entry()));
                    }
                    Key::Char('r') => {
                        state.ask_input(rename_ressource_request(state.read_selected_entry()));
                    }
                    _ => {}
                },
                #[allow(clippy::needless_late_init)]
                Mode::WRITING => match key {
                    Key::Esc => state.mode = Mode::SELECTING,
                    Key::Char('\n') => {
                        let user_input = state.read_user_input().clone();
                        let request = state.user_input_request.get_closure();

                        let result: UserOperationResult;
                        if state.user_input_request.edit_ressource && state.selecting_previous_dir()
                        {
                            result = UserOperationResult::operation_on_prev();
                        } else {
                            result = request(&mut state, &user_input);
                        }

                        if let Some(message) = result.message {
                            state.show_message(result.exit.get_header(), &message);
                        } else {
                            state.mode = Mode::SELECTING;
                        }
                    }
                    Key::Char(character) => state.user_input().push(character),
                    Key::Backspace => {
                        state.user_input().pop();
                    }
                    _ => {}
                },
                Mode::DISCARD => {
                    state.mode = Mode::SELECTING;
                }
            }
        }
    }

    println!("{ToMainScreen}");
    state.publish_reports();
    eprintln!("{}", state.get_bash_string(exited));
    Ok(())
}
