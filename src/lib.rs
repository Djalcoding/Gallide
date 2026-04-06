pub mod config;
pub mod read_ls;
pub mod ui_brain;

pub mod ui {
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
        read_ls::{Entry, Item},
        ui_brain::State,
    };

    fn optionally_add_borders<'a>(block: Block<'a>, border_type: &Option<BorderType>) -> Block<'a>{
        block
            .borders(if border_type.is_some() {
                Borders::ALL
            } else {
                Borders::NONE
            })
            .border_type(border_type.unwrap_or(BorderType::Plain))
    }

    fn build_entries<'a>(directories: &'a Vec<Entry>, config: &'a Config) -> Vec<ListItem<'a>> {
        let mut entries = Vec::new();
        for directory in directories {
            let symbol: Span;
            if let Item::File = directory.entry_type {
                symbol = Span::styled(
                    config.file_symbol(),
                    Style::default().fg(config.file_symbol_color()),
                );
            } else if let Item::Folder = directory.entry_type {
                symbol = Span::styled(
                    config.directory_symbol(),
                    Style::default().fg(config.directory_symbol_color()),
                );
            } else {
                symbol = Span::raw("");
            }

            entries.push(ListItem::new(Spans::from(vec![
                symbol,
                Span::styled(directory.name(), Style::default().fg(config.list_color())),
            ])));
        }
        entries
    }

    fn build_directory_list<'b>(directories: &'b Vec<Entry>, config: &'b Config) -> List<'b> {
        let items = build_entries(directories, config);
        let mut style = Style::default().fg(config.list_color());
        if config.draw_background() {
            style = style.bg(config.background_color())
        }
        List::new(items)
            .block(
                optionally_add_borders(
                    Block::default()
                    .title("Directories"),
                    &config.border_type
                ),
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
        List::new(vec![ListItem::new(state.current_searchbar_text().clone())]).block(
            Block::default()
                .title(config.search_bar_title.clone())
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
                    .bg(config.tooltip_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(": jk/↓↑  ", Style::default().add_modifier(Modifier::ITALIC)),
            Span::styled(
                "Exit",
                Style::default()
                    .fg(Color::Black)
                    .bg(config.tooltip_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(": q/ESC  ", Style::default().add_modifier(Modifier::ITALIC)),
            Span::styled(
                "Insert Mode",
                Style::default()
                    .fg(Color::Black)
                    .bg(config.tooltip_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(": i  ", Style::default().add_modifier(Modifier::ITALIC)),
            Span::styled(
                "Select",
                Style::default()
                    .fg(Color::Black)
                    .bg(config.tooltip_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(": ↵/l/→  ", Style::default().add_modifier(Modifier::ITALIC)),
            Span::styled(
                "Parent dir",
                Style::default()
                    .fg(Color::Black)
                    .bg(config.tooltip_color)
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

    pub fn build_ui<B: Backend>(f: &mut Frame<B>, state: &State, config: &Config) {
        let mut constraints = vec![
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(3),
        ];
        if config.display_tooltips {
            constraints.push(Constraint::Length(1));
        }
        let mut list_state: ListState = ListState::default();
        let chunks = Layout::default()
            .direction(tui::layout::Direction::Vertical)
            .constraints(constraints)
            .split(f.size());

        list_state.select(Some(state.get_selected_box()));
        f.render_widget(build_path(state, config), chunks[0]);
        f.render_stateful_widget(
            build_directory_list(state.elements(), config),
            chunks[1],
            &mut list_state,
        );
        f.render_widget(build_search_bar(state, config), chunks[2]);
        if config.display_tooltips {
            f.render_widget(build_tooltips(config), chunks[3]);
        }
    }
}
