use std::io;

use fzf_clone::ui;
use termion::raw::IntoRawMode;
use tui::{backend::TermionBackend, Terminal};

fn main() -> Result<(), io::Error>{

    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    loop {
        let _ = terminal.draw(|f| {
            ui::build_ui(f); // TODO: actually build ui
        });

        if true{break;}
    }
    Ok(())
}
