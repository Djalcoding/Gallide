use core::time;
use std::{
    cell::RefCell,
    env,
    io::{self, stdin},
    path::Path,
    rc::Rc,
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
    let state = Rc::new(RefCell::new(State::new(config, reporter)));
    println!("{ToAlternateScreen}");

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let stdin = stdin();
        for c in stdin.keys().flatten() {
            let _ = tx.send(c);
        }
    });

    let mut exited = false;
    while state.borrow().is_running() {
        let _ = terminal.draw(|f| {
            let borrowed = state.borrow();
            ui::build_ui(f, &borrowed, borrowed.get_config());
        });
        if let Ok(key) = rx.recv_timeout(time::Duration::from_millis(50)) {
            let mode = state.borrow().mode.clone();
            match mode {
                Mode::INSERT => match key {
                    Key::Esc | Key::Char('\n') => state.borrow_mut().switch_mode(),
                    Key::Backspace => state.borrow_mut().backspace(),
                    Key::Char(character) => state.borrow_mut().add_character(character),
                    Key::Up => state.borrow_mut().decrement_selected_box(),
                    Key::Down => state.borrow_mut().increment_selected_box(),
                    _ => {}
                },
                Mode::SELECTING => match key {
                    Key::Up | Key::Char('k') => state.borrow_mut().decrement_selected_box(),
                    Key::Down | Key::Char('j') => state.borrow_mut().increment_selected_box(),

                    Key::Esc | Key::Char('q') => {
                        state.borrow_mut().stop();
                        exited = true;
                    }
                    Key::Right | Key::Char('l') => {
                        if state.borrow().is_selecting_directory() {
                            state.borrow_mut().open_selected_directory();
                            state.borrow_mut().rebuild_directories();
                        } else {
                            state.borrow_mut().stop();
                        }
                    }
                    Key::Left | Key::Char('h') => {
                        let mut mut_borrow = state.borrow_mut();
                        mut_borrow.go_back_one_directory();
                        mut_borrow.rebuild_directories();
                    }
                    Key::Char('\n') => state.borrow_mut().stop(),
                    Key::Char('i') => {
                        if enable_searchbar {
                            state.borrow_mut().switch_mode()
                        }
                    }
                    Key::Char('c') => state.borrow_mut().clear_search_bar(),
                    Key::Char('a') => {
                        state
                            .borrow_mut()
                            .ask_input(create_ressource_request(Rc::clone(&state)));
                    }
                    Key::Char('d') => {
                        let path = state.borrow().read_selected_entry().path().clone();
                        state
                            .borrow_mut()
                            .ask_input(delete_ressource_request(path, Rc::clone(&state)));
                    }
                    Key::Char('r') => {
                        let path = state.borrow().read_selected_entry().path().clone();
                        state
                            .borrow_mut()
                            .ask_input(rename_ressource_request(path, Rc::clone(&state)));
                    }
                    _ => {}
                },
                #[allow(clippy::needless_late_init)]
                Mode::WRITING => match key {
                    Key::Esc => state.borrow_mut().mode = Mode::SELECTING,
                    Key::Char('\n') => {
                        let user_input = state.borrow().read_user_input().clone();
                        let request = state.borrow_mut().user_input_request.get_closure();

                        let result: UserOperationResult;
                        if state.borrow().user_input_request.edit_ressource
                            && state.borrow().selecting_previous_dir()
                        {
                            result = UserOperationResult::operation_on_prev();
                        } else {
                            result = request(&user_input);
                        }

                        if let Some(message) = result.message {
                            state
                                .borrow_mut()
                                .show_message(result.exit.get_header(), &message);
                        } else {
                            state.borrow_mut().mode = Mode::SELECTING;
                        }
                    }
                    Key::Char(character) => state.borrow_mut().user_input().push(character),
                    Key::Backspace => {
                        state.borrow_mut().user_input().pop();
                    }
                    _ => {}
                },
                Mode::DISCARD => {
                    state.borrow_mut().mode = Mode::SELECTING;
                }
            }
        }
    }

    println!("{ToMainScreen}");
    state.borrow_mut().publish_reports();
    eprintln!("{}", state.borrow().get_bash_string(exited));
    Ok(())
}
