pub mod read_ls;

pub mod config {
    use tui::style::Color;

    pub struct Config<'a> {
        list_color: Option<Color>,
        search_bar_default_color: Option<Color>,
        insert_mode_color: Color,
        background_color: Option<Color>,
        focus_color: Option<Color>,
        focus_symbol: &'a str,
        directory_symbol: String,
        file_symbol: String,
    }

    impl<'a> Default for Config<'a> {
        fn default() -> Self {
            Config {
                insert_mode_color: Color::Red,
                list_color: None,
                background_color: Some(Color::Rgb(25, 23, 36)),
                search_bar_default_color: Some(Color::White),
                focus_color: Some(Color::White),
                focus_symbol: "> ",
                directory_symbol: String::from(" "),
                file_symbol: String::from("󰈔 "),
            }
        }
    }

    impl<'a> Config<'a> {
        pub fn insert_mode_color(&self) -> Color {
            self.insert_mode_color
        }

        pub fn list_color(&self) -> Color {
            if let Some(color) = self.list_color {
                return color;
            }
            Color::Reset
        }

        pub fn draw_background(&self) -> bool {
            self.background_color.is_some()
        }

        pub fn focus_color(&self) -> Color {
            if let Some(color) = self.focus_color {
                return color;
            }
            Color::Reset
        }

        pub fn focus_symbol(&self) -> &'a str {
            self.focus_symbol
        }

        pub fn directory_symbol(&self) -> String {
            self.directory_symbol.clone()
        }
        pub fn file_symbol(&self) -> String {
            self.file_symbol.clone()
        }

        pub fn background_color(&self) -> Color {
            self.background_color.unwrap_or(Color::Reset)
        }

        pub fn search_bar_color(&self) -> Color {
            if let Some(color) = self.search_bar_default_color {
                return color;
            }
            Color::Reset
        }
    }
}

pub mod ui {

    use std::path::PathBuf;

    use tui::{
        Frame,
        backend::Backend,
        layout::{Constraint, Layout},
        style::{Color, Modifier, Style},
        text::{Span, Spans},
        widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    };

    use crate::{
        config::Config,
        read_ls::{get_absolute_path_from_str, get_folder_contents},
    };

    pub enum Mode {
        INSERT,
        SELECTING,
    }

    pub enum Item {
        File,
        Folder,
    }

    pub struct Entry {
        path: PathBuf,
        name: String,
        entry_type: Item,
    }

    impl Entry {
        pub fn new(path: PathBuf, name: String, entry_type:Item) -> Self {
            Entry {
                path,
                name,
                entry_type,
            }
        }
        pub fn set_type(&mut self) {}
        pub fn path(&self) -> &PathBuf {
            &self.path
        }
        pub fn name(&self) -> &String {
            &self.name
        }
    }

    pub struct State {
        selected_box: usize,
        elements: Vec<Entry>,
        search_bar_text: String,
        current_dir: PathBuf,
        running: bool,
        mode: Mode,
        config: Config<'static>,
    }

    impl State {
        pub fn new(elements: Vec<Entry>, config: Config<'static>) -> State {
            State {
                selected_box: if elements.len() > 1 { 1 } else { 0 },
                elements,
                search_bar_text: String::from(""),
                running: true,
                current_dir: get_absolute_path_from_str("."),
                mode: Mode::SELECTING,
                config,
            }
        }

        fn get_selected_box(&self) -> usize {
            self.selected_box
        }

        pub fn increment_selected_box(&mut self) {
            self.selected_box = (self.selected_box + 1) % self.elements.len();
        }

        pub fn decrement_selected_box(&mut self) {
            if self.selected_box == 0 {
                self.selected_box = self.elements.len() - 1;
            } else {
                self.selected_box -= 1;
            }
        }

        fn move_selected_box_to_start(&mut self) {
            self.selected_box = self.elements.len() - 1;
        }

        fn reset_search_bar(&mut self) {
            self.search_bar_text = String::new();
            self.trim_directories();
        }

        pub fn trim_directories(&mut self) {
            let mut new_list: Vec<Entry> = Vec::new();
            for element in get_folder_contents(
                self.current_dir.to_str().unwrap(),
                self.config.directory_symbol(),
                self.config.file_symbol(),
            )
            .unwrap()
            {
                //TODO handle this
                if element.name() == &String::from("..") {
                    new_list.push(element);
                    continue;
                } else if element.name().starts_with(
                    format!("{}{}", self.config.directory_symbol(), self.search_bar_text).as_str(),
                ) || element.name().starts_with(
                    format!("{}{}", self.config.file_symbol(), self.search_bar_text).as_str(),
                ) {
                    new_list.push(element);
                }
            }

            self.elements = new_list;
        }

        pub fn rebuild_directories(&mut self) {
            self.elements = get_folder_contents(
                self.current_dir.to_str().expect("INVALID UNICODE"),
                self.config.directory_symbol(),
                self.config.file_symbol(),
            )
            .unwrap(); // TODO : Handle this
            self.move_selected_box_to_start()
        }

        pub fn get_selected_path(&self) -> PathBuf {
            self.elements[self.get_selected_box()].path().to_path_buf()
        }

        pub fn go_back_one_directory(&mut self) {
            self.current_dir.pop();
        }

        pub fn backspace(&mut self) {
            self.search_bar_text.pop();
            self.trim_directories();
            self.selected_box = self.elements.len() - 1;
        }

        pub fn add_character(&mut self, character: char) {
            self.search_bar_text.push(character);
            self.trim_directories();
            self.selected_box = self.elements.len() - 1;
        }

        pub fn clear_search_bar(&mut self) {
            self.search_bar_text = String::new();
            self.trim_directories();
            self.selected_box = self.elements.len() - 1;
        }

        pub fn stop(&mut self) {
            self.running = false;
        }

        pub fn is_running(&self) -> bool {
            self.running
        }

        pub fn switch_mode(&mut self) {
            self.mode = if self.is_inserting() {
                Mode::SELECTING
            } else {
                Mode::INSERT
            }
        }

        pub fn is_inserting(&self) -> bool {
            if let Mode::INSERT = &self.mode {
                return true;
            }
            false
        }

        pub fn open_selected_directory(&mut self) {
            if !self.is_selecting_directory() {
                self.stop();
                return
            }
            self.set_current_directory(self.get_selected_path());
            self.reset_search_bar();
            self.move_selected_box_to_start()
        }
        pub fn get_bash_string(&self) ->String {
            format!("({}'{})", if self.is_selecting_directory() {"D"} else {"F"}, self.get_selected_path().display())
        }

        fn get_current_directory(&self) -> PathBuf {
            self.current_dir.clone()
        }

        fn is_selecting_directory(&self)->bool {
            if let Item::Folder = self.elements[self.selected_box].entry_type {
                return true;
            }
            false
        }

        fn set_current_directory(&mut self, new_directory: PathBuf) {
            self.current_dir = new_directory;
        }
    }

    fn build_entries(directories: Vec<&String>) -> Vec<ListItem<'static>> {
        let mut entries = Vec::new();
        for directory in directories {
            entries.push(ListItem::new(directory.clone()));
        }
        entries
    }

    fn build_directory_list<'b>(directories: &[Entry], config: &Config<'b>) -> List<'b> {
        let names: Vec<&String> = directories.iter().map(|f| f.name()).collect();
        let items = build_entries(names);
        let mut style = Style::default().fg(Color::White);
        if config.draw_background() {
            style = style.bg(config.background_color())
        }
        List::new(items)
            .block(
                Block::default()
                    .title("Directories")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .style(style)
            .highlight_style(Style::default().fg(Color::Black).bg(config.focus_color()))
            .highlight_symbol(config.focus_symbol())
    }

    fn build_search_bar<'a>(state: &State, config: &Config) -> List<'a> {
        let border_style = Style::default().fg(if state.is_inserting() {
            config.insert_mode_color()
        } else {
            config.search_bar_color()
        });
        List::new(vec![ListItem::new(state.search_bar_text.clone())]).block(
            Block::default()
                .title("Search bar")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style)
                .style(Style::default().bg(config.background_color())),
        )
    }

    fn build_tooltips(config: &Config) -> Paragraph<'static> {
        Paragraph::new(Spans::from(vec![
            Span::styled(
                "Movement",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(": jk/↓↑  ", Style::default().add_modifier(Modifier::ITALIC)),
            Span::styled(
                "Exit",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(": q/ESC  ", Style::default().add_modifier(Modifier::ITALIC)),
            Span::styled(
                "Insert Mode",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(": i  ", Style::default().add_modifier(Modifier::ITALIC)),
            Span::styled(
                "Select",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(": ↵/l/→  ", Style::default().add_modifier(Modifier::ITALIC)),
            Span::styled(
                "Parent dir",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(": h/←  ", Style::default().add_modifier(Modifier::ITALIC)),
        ]))
        .style(Style::default().bg(config.background_color()))
    }

    fn build_path(state: &State, config: &Config) -> Paragraph<'static> {
        Paragraph::new(Spans::from(vec![Span::styled(
            state.get_current_directory().display().to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        )]))
        .style(Style::default().bg(config.background_color()))
    }

    pub fn build_ui<B: Backend>(f: &mut Frame<B>, state: &State, config: Config) {
        let constraints = vec![
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Length(1),
        ];

        let mut list_state: ListState = ListState::default();
        let chunks = Layout::default()
            .direction(tui::layout::Direction::Vertical)
            .constraints(constraints)
            .split(f.size());

        list_state.select(Some(state.selected_box));
        f.render_widget(build_path(state, &config), chunks[0]);
        f.render_stateful_widget(
            build_directory_list(&state.elements, &config),
            chunks[1],
            &mut list_state,
        );
        f.render_widget(build_search_bar(state, &config), chunks[2]);
        f.render_widget(build_tooltips(&config), chunks[3]);
    }
}
