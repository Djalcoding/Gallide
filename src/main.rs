use std::{
    env,
    io::{self},
    path::Path,
};

use gallide_bin::{config::*, app::App};

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = env::args().collect();
    let config_path: Option<&Path> = if args.len() == 1 {
        None
    } else {
        Some(Path::new(&args[1]))
    };
    let config = if let Some(path) = config_path {
        Config::from_file(path).unwrap_or_else(|_| Config::default())
    } else {
        Config::default()
    };
    let mut state = App::new(config);

    ratatui::run(|terminal| state.run(terminal))?;

    eprintln!("{}", state.get_bash_string());
    Ok(())
}
