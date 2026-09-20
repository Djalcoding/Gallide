pub mod config;
pub mod file_control;
pub mod read_ls;
pub mod reporter;
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

    fn optionally_add_borders<'a>(block: Block<'a>, border_type: &Option<BorderType>) -> Block<'a> {
        block
            .borders(if border_type.is_some() {
                Borders::ALL
            } else {
                Borders::NONE
            })
            .border_type(border_type.unwrap_or(BorderType::Plain))
    }

    fn optional_bg_style(color: Option<Color>) -> Style {
        if let Some(c) = color {
            Style::default().bg(c)
        } else {
            Style::default()
        }
    }

    fn build_entries<'a, I>(directories: I, config: &'a Config) -> Vec<ListItem<'a>>
    where
        I: Iterator<Item = &'a Entry>,
    {
        let mut entries = Vec::new();
        directories.for_each(|directory| {
            let symbol: Span;
            if let Item::File = directory.entry_type {
                symbol = Span::styled(
                    &config.main_box.file_symbol,
                    Style::default().fg(config.main_box.file_symbol_color),
                );
            } else if let Item::Folder = directory.entry_type {
                symbol = Span::styled(
                    &config.main_box.directory_symbol,
                    Style::default().fg(config.main_box.directory_symbol_color),
                );
            } else {
                symbol = Span::raw("");
            }

            entries.push(ListItem::new(Spans::from(vec![
                symbol,
                Span::styled(
                    directory.name(),
                    Style::default().fg(config.main_box.text_color),
                ),
            ])));
        });
        entries
    }

    fn build_text_input<'a>(title: &'a str, state: &'a State, config: &'a Config) -> Paragraph<'a> {
        let mut style =
            Style::default().fg(config.search_bar.insert_mode_border_config.border_color); // TODO : add config for main box
        if let Some(background) = config.main_box.background_color {
            style = style.bg(background)
        }
        Paragraph::new(state.read_user_input().as_str())
            .style(Style::default().fg(Color::White)) // TODO : add config for text color
            .block(optionally_add_borders(
                Block::default().title(title).style(style),
                &config.main_box.border_config.border_type,
            ))
    }

    fn build_directory_list<'a, I>(directories: I, config: &'a Config) -> List<'a>
    where
        I: Iterator<Item = &'a Entry>,
    {
        let items = build_entries(directories, config);
        let mut style = Style::default().fg(config.main_box.border_config.border_color);
        if let Some(background) = config.main_box.background_color {
            style = style.bg(background)
        }
        let block = if config.main_box.title.is_empty() {
            Block::default()
        } else {
            Block::default().title(config.main_box.title.clone())
        };

        List::new(items)
            .block(optionally_add_borders(
                block,
                &config.main_box.border_config.border_type,
            ))
            .style(style)
            .highlight_style(
                Style::default()
                    .fg(config.main_box.focus_text_color)
                    .bg(config.main_box.focus_color),
            )
            .highlight_symbol(&config.main_box.focus_symbol)
    }

    fn build_search_bar<'a>(state: &State, config: &Config) -> List<'a> {
        let border_style = Style::default().fg(if state.is_inserting() {
            config.search_bar.insert_mode_border_config.border_color
        } else {
            config.search_bar.border_config.border_color
        });
        let block = optionally_add_borders(
            Block::default()
                .title(config.search_bar.title.clone())
                .border_style(border_style)
                .style(optional_bg_style(config.search_bar.background_color)),
            &config.search_bar.border_config.border_type,
        );
        List::new(vec![ListItem::new(state.current_searchbar_text())]).block(block)
    }

    fn build_tooltips(config: &Config) -> Paragraph<'static> {
        let header_style = Style::default()
            .bg(config.tooltips.highlight_color)
            .fg(config.tooltips.text_color)
            .add_modifier(Modifier::BOLD);

        let keybind_style = Style::default()
            .fg(config.tooltips.keybind_color)
            .add_modifier(Modifier::ITALIC);

        Paragraph::new(Spans::from(vec![
            Span::styled("Movement", header_style),
            Span::styled(": jk/↓↑  ", keybind_style),
            Span::styled("Exit", header_style),
            Span::styled(": q/ESC  ", keybind_style),
            Span::styled("Insert Mode", header_style),
            Span::styled(": i  ", keybind_style),
            Span::styled("Select", header_style),
            Span::styled(": ↵/l/→  ", keybind_style),
            Span::styled("Parent dir", header_style),
            Span::styled(": h/←  ", keybind_style),
        ]))
        .style(optional_bg_style(config.main_box.background_color))
    }

    fn build_path(state: &State, config: &Config) -> Paragraph<'static> {
        Paragraph::new(Spans::from(vec![Span::styled(
            state.get_current_directory().display().to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        )]))
        .style(optional_bg_style(config.main_box.background_color))
    }

    pub fn build_ui<B: Backend>(f: &mut Frame<B>, state: &State, config: &Config) {
        let mut constraints = vec![];
        if config.directory_line.display {
            constraints.push(Constraint::Length(1));
        }
        constraints.push(Constraint::Min(0));
        if config.search_bar.enabled {
            constraints.push(Constraint::Length(3));
        }
        if config.tooltips.display {
            constraints.push(Constraint::Length(1));
        }
        let mut list_state: ListState = ListState::default();
        let mut chunks = Layout::default()
            .direction(tui::layout::Direction::Vertical)
            .constraints(constraints)
            .split(f.size())
            .into_iter();

        list_state.select(Some(state.get_selected_box()));
        if config.directory_line.display {
            f.render_widget(build_path(state, config), chunks.next().unwrap());
        }

        if state.is_in_write_mode() {
            f.render_widget(
                build_text_input(state.user_input_title(), state, config),
                chunks.next().unwrap(),
            );
        } else {
            f.render_stateful_widget(
                build_directory_list(state.elements().iter(), config),
                chunks.next().unwrap(),
                &mut list_state,
            );
        }
        if config.search_bar.enabled {
            f.render_widget(build_search_bar(state, config), chunks.next().unwrap());
        }
        if config.tooltips.display {
            f.render_widget(build_tooltips(config), chunks.next().unwrap());
        }
    }
}
