pub mod read_ls;

pub mod ui {
    use tui::{
        Frame,
        backend::Backend,
        layout::{Constraint, Layout},
        style::{Color, Style},
        widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    };

    use crate::read_ls::get_directories;

    pub enum Mode {
        INSERT,
        SELECTING, 
    }
    
    pub struct Config {
        focus_color:Color,
        unfocus_color: Color,
        search_bar_size: u16
    }

    impl Default for Config {
        fn default() -> Self {
            Config{focus_color: Color::Red, unfocus_color: Color::White, search_bar_size:5}
        }
    }

    impl Config { 
        pub fn from(focus_color:Color, unfocus_color:Color, search_bar_size:u16)-> Self {
            let mut size = search_bar_size;
            if search_bar_size > 100 {
                size = 100;
            }
            Config {focus_color, unfocus_color, search_bar_size:size}
        }
    }

    pub struct State {
        selected_box: usize,
        directories: Vec<String>,
        search_bar_text: String,
        running: bool,
        mode:Mode,
    }


    impl State {
        pub fn new(directories: Vec<String>) -> State{
            State {
                selected_box: if directories.len() > 1 { 1 } else { 0 },
                directories,
                search_bar_text: String::from(""),
                running: true,
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

        pub fn trim_directories(&mut self){
            let mut new_list:Vec<String> = vec![String::from("..")];
            
            for directory in get_directories(){
                if directory.starts_with(format!("./{}", self.search_bar_text).as_str()){
                    new_list.push(directory);
                } 
            }

            self.directories = new_list;

            if self.selected_box >= self.directories.len() {
                self.selected_box = self.directories.len()-1; 
            }
        }


        pub fn get_selected_directory(&self) -> String {
            self.directories[self.get_selected_box()].clone()
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

    }



    fn build_entries(directories: &Vec<String>) -> Vec<ListItem<'static>> {
        let mut entries = Vec::new();

        for directory in directories {
            entries.push(ListItem::new(directory.clone()));
        }
        entries
    }

    fn build_directory_list(directories: &Vec<String>) -> List<'static> {
        let items = build_entries(directories);

        List::new(items)
            .block(Block::default().title("Directories").borders(Borders::ALL))
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

    pub fn build_ui<B: Backend>(f: &mut Frame<B>, state: &State, config:Config) {
        let constraints = vec![Constraint::Percentage(100-config.search_bar_size), Constraint::Percentage(config.search_bar_size)];

        let mut list_state: ListState = ListState::default();
        let chunks = Layout::default()
            .direction(tui::layout::Direction::Vertical)
            .margin(1)
            .constraints(constraints)
            .split(f.size());

        list_state.select(Some(state.selected_box));

        f.render_stateful_widget(
            build_directory_list(&state.directories),
            chunks[0],
            &mut list_state,
        );
        f.render_widget(build_search_bar(state, &config), chunks[1]);
    }
}
