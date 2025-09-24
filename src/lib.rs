pub mod read_ls;

pub mod ui{
    use tui::{backend::Backend, layout::{Constraint, Layout}, style::{Color, Style}, widgets::{Block, Borders, List, ListItem, ListState}, Frame};

    
    fn build_entries() -> Vec<ListItem<'static>>{
        vec![ListItem::new("Ahke"), ListItem::new("Not Ahke")]

    }

    fn build_directory_list() -> List<'static>{
        let items = build_entries();

        List::new(items)
            .block(
                Block::default()
                .title("Directories")
                .borders(Borders::ALL)
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default()
                .fg(Color::Black)
                .bg(Color::White)
            )
            .highlight_symbol("> ")
    }

    pub fn build_ui<B: Backend>(f: &mut Frame<B>, selected:usize){

        let mut constraints = vec![Constraint::Percentage(50), Constraint::Percentage(50)];

        let mut state:ListState = ListState::default();
        let chunks = Layout::default()
            .direction(tui::layout::Direction::Horizontal)
            .margin(1)
            .constraints(constraints)
            .split(f.size());

        state.select(Some(selected));

        f.render_stateful_widget(build_directory_list(), chunks[0], &mut state);
        f.render_widget(Block::default(), chunks[1]);
    }
}
