use std::io;

use fzf_clone::{read_ls, ui};
use termion::{clear, raw::IntoRawMode};
use tui::{backend::TermionBackend, Terminal};

fn main() -> Result<(), io::Error>{

    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut selected_id = 0;
    println!("{}", clear::All);

    read_ls::get_directories();
    return Ok(());
    loop {
        let _ = terminal.draw(|f| {
            ui::build_ui(f,selected_id ); 
        });

    }
    Ok(())
}
