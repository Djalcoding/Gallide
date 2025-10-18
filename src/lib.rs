pub mod read_ls;

pub mod ui {

    use std::{path::{Path, PathBuf}};

    use tui::{
        backend::Backend, layout::{Constraint, Layout}, style::{Color, Modifier, Style}, text::{Span, Spans}, widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph}, Frame
    };

    use crate::read_ls::{ get_absolute_path_from_str, get_directories};


    pub enum Mode {
        INSERT,
        SELECTING, 
    }
    
    pub struct Config {
        focus_color:Color,
        unfocus_color: Color,
    }

    pub struct Entry {
        path:PathBuf,
        name:String
    }

    impl Entry {
        pub fn new(path:PathBuf, name:String)->Self{
            Entry { path, name }  
        } 
        pub fn path(&self) -> &PathBuf {
            &self.path
        }
        pub fn name(&self) -> &String {
            &self.name
        }
    }

    impl Default for Config {
        fn default() -> Self {
            Config{focus_color: Color::Red, unfocus_color: Color::White}
        }
    }

    impl Config { 
        pub fn from(focus_color:Color, unfocus_color:Color)-> Self {
            Config {focus_color, unfocus_color}
        }
    }

    pub struct State {
        selected_box: usize,
        directories: Vec<Entry>,
        search_bar_text: String,
        current_dir:PathBuf, 
        running: bool,
        mode:Mode,
    }


    impl State {
        pub fn new(directories: Vec<Entry>) -> State{
            State {
                selected_box: if directories.len() > 1 { 1 } else { 0 },
                directories,
                search_bar_text: String::from(""),
                running: true,
                current_dir: get_absolute_path_from_str("."),
                mode: Mode::SELECTING,
            }
        }

        pub fn get_selected_box(&self) -> usize {
            self.selected_box
        }

        pub fn increment_selected_box(&mut self) {
            self.selected_box = (self.selected_box + 1) % self.directories.len();
        }

        pub fn decrement_selected_box(&mut self) {
            if self.selected_box == 0 {
                self.selected_box = self.directories.len() - 1;
            } else {
                self.selected_box -= 1;
            }
        }

        pub fn reset_selected_box(&mut self) {
            self.selected_box = 0;
        }

        pub fn trim_directories(&mut self){
            let mut new_list:Vec<Entry> = Vec::new();
            new_list.push(Entry::new(get_absolute_path_from_str(".."),String::from("..")));
            for directory in get_directories(self.current_dir.to_str().unwrap()) {
                if directory.name() == &String::from(".."){
                    continue;
                }
                else if directory.name().starts_with(&self.search_bar_text) {
                    new_list.push(directory); 
                }  
            }

            self.directories = new_list;
        }

        pub fn rebuild_directories(&mut self) {
            self.directories = get_directories(self.current_dir.to_str().expect("INVALID UNICODE"));
        }


        pub fn get_selected_directory(&self) -> PathBuf{
            self.directories[self.get_selected_box()].path().to_path_buf()
        }

        pub fn go_back_one_directory(&mut self) {
            self.current_dir = self.directories[0].path().to_path_buf(); 
        }

        pub fn backspace(&mut self) {
            self.search_bar_text.pop();
            self.trim_directories();
            self.selected_box = self.directories.len()-1;
        }

        pub fn add_character(&mut self, character:char){
            self.search_bar_text.push(character);
            self.trim_directories();
            self.selected_box = self.directories.len()-1;
        }

        pub fn clear_search_bar(&mut self) {
            self.search_bar_text = String::new();
            self.trim_directories();
            self.selected_box = self.directories.len()-1;
        }

        pub fn stop(&mut self) {
            self.running = false;
        }

        pub fn is_running(&self) -> bool {
            self.running
        }

        pub fn get_mode(&self) -> &Mode{
            &self.mode
        }

        pub fn switch_mode(&mut self){

            self.mode = if self.is_inserting() {Mode::SELECTING} else {Mode::INSERT}
        }

        pub fn is_inserting(&self) -> bool{
            if let Mode::INSERT = self.get_mode() {
                return true;
            } 
            false
        }

        pub fn set_current_directory(&mut self, new_directory:PathBuf) {
            self.current_dir = new_directory; 
        }


        pub fn get_current_directory(&self)-> PathBuf {
            self.current_dir.clone()
        }

    }

    fn build_entries(directories: Vec<&String>) -> Vec<ListItem<'static>> {
        let mut entries = Vec::new();
        for directory in directories {
            entries.push(ListItem::new(directory.clone()));
        }
        entries
    }

    fn build_directory_list(directories: &[Entry]) -> List<'static> {
        let names:Vec<&String> = directories.iter().map(|f| {f.name()}).collect();
        let items = build_entries(names);
        List::new(items)
            .block(Block::default()
                .title("Directories")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Black).bg(Color::White))
            .highlight_symbol("> ")
    }

    fn build_search_bar<'a>(state: &State, config:&Config) -> List<'a> {
        List::new(vec![ListItem::new(state.search_bar_text.clone())]).block(
            Block::default()
                .title("Search bar")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(
                    Style::default().fg(
                        if state.is_inserting() {config.focus_color}
                        else {config.unfocus_color}
                    )
                ),
        )
    }

    fn build_tooltips()-> Paragraph<'static>{
        Paragraph::new(Spans::from(vec![
                Span::styled("Movement", Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),       
                Span::styled(": jk/↓↑  ", Style::default().add_modifier(Modifier::ITALIC)),       
                Span::styled("Exit", Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),       
                Span::styled(": q/ESC  ", Style::default().add_modifier(Modifier::ITALIC)),
                Span::styled("Insert Mode", Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),       
                Span::styled(": i  ", Style::default().add_modifier(Modifier::ITALIC)),
                Span::styled("Select", Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),       
                Span::styled(": ↵/l/→  ", Style::default().add_modifier(Modifier::ITALIC)),
                Span::styled("Parent dir", Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),       
                Span::styled(": h/←  ", Style::default().add_modifier(Modifier::ITALIC)),
        ]))
    }

    fn build_path(state:&State) -> Paragraph<'static> {
        Paragraph::new(Spans::from(vec![
                Span::styled(state.get_current_directory().display().to_string(), Style::default().add_modifier(Modifier::BOLD))
        ]))
    }


    pub fn build_ui<B: Backend>(f: &mut Frame<B>, state: &State, config:Config) {
        let constraints = vec![Constraint::Length(1),Constraint::Min(0), Constraint::Length(3), Constraint::Length(1)];

        let mut list_state: ListState = ListState::default();
        let chunks = Layout::default()
            .direction(tui::layout::Direction::Vertical)
            .margin(1)
            .constraints(constraints)
            .split(f.size());

        list_state.select(Some(state.selected_box));
        f.render_widget(build_path(state), chunks[0]);
        f.render_stateful_widget(
            build_directory_list(&state.directories),
            chunks[1],
            &mut list_state,
        );
        f.render_widget(build_search_bar(state, &config), chunks[2]);
        f.render_widget(build_tooltips(), chunks[3]);


    }
}
