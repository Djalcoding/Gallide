use ratatui::{
    Frame,
    layout::{Constraint, Direction::Vertical, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::{
    app::App,
    config::{Config, SearchBarConfig, TooltipConfig},
    widget::EntryList,
};

pub(crate) struct Screen {
    pub constraints: Vec<Constraint>,
    pub top_bar: Option<Line<'static>>,
    pub main_box: EntryList<'static>,
    pub search_bar: Option<Paragraph<'static>>,
    pub tooltips: Option<Paragraph<'static>>,
}

impl Screen {
    pub fn start(config: &Config) -> Screen {
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
        Screen {
            constraints,
            top_bar: None,
            main_box: EntryList::new()
                .style(config.main_box.entry_style)
                .highlight_style(config.main_box.highlight_style)
                .highlight_symbol(config.main_box.focus_symbol.clone()),
            search_bar: None,
            tooltips: None,
        }
    }

    pub fn build_top_bar(
        &mut self,
        area: &Rect,
        title: String,
        info: Option<String>,
        style: Style,
    ) {
        let len = title.len();
        let mut spans = vec![Span::styled(
            title,
            Style::default().add_modifier(Modifier::BOLD),
        )];
        if let Some(information) = info {
            spans.push(Span::raw(
                " ".repeat(
                    (area.width as usize)
                        .saturating_sub(len)
                        .saturating_sub(information.len())
                        .saturating_sub(3),
                ),
            ));
            spans.push(Span::raw(information));
        }
        self.top_bar = Some(Line::from(spans).style(style));
    }

    pub fn build_search_bar(&mut self, searchmode: bool, text: String, config: &SearchBarConfig) {
        if !config.enabled {
            return;
        }
        self.search_bar = Some(
            Paragraph::new(text)
                .block(if searchmode {
                    config.insert_mode_block.clone()
                } else {
                    config.block.clone()
                })
                .style(config.style),
        );
    }

    pub fn build_tooltips(&mut self, config: &TooltipConfig, style: Style) {
        if !config.display {
            return;
        }
        let header_style = Style::default()
            .bg(config.highlight_color)
            .fg(config.text_color)
            .add_modifier(Modifier::BOLD);

        let keybind_style = Style::default()
            .fg(config.keybind_color)
            .add_modifier(Modifier::ITALIC);

        self.tooltips = Some(
            Paragraph::new(Line::from(vec![
                Span::styled("Move", header_style),
                Span::styled(": jk/↓↑  ", keybind_style),
                Span::styled("Exit", header_style),
                Span::styled(": q/ESC  ", keybind_style),
                Span::styled("Search", header_style),
                Span::styled(": i  ", keybind_style),
                Span::styled("Select", header_style),
                Span::styled(": ENTER↵/l/→  ", keybind_style),
                Span::styled("Go back", header_style),
                Span::styled(": h/←  ", keybind_style),
                Span::styled("Create", header_style),
                Span::styled(": a  ", keybind_style),
                Span::styled("Remove", header_style),
                Span::styled(": d  ", keybind_style),
                Span::styled("Rename", header_style),
                Span::styled(": r  ", keybind_style),
                Span::styled("Depth", header_style),
                Span::styled(": +/-  ", keybind_style),
            ]))
            .style(style),
        )
    }
}

impl App {
    pub fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Vertical)
            .constraints(&self.screen.constraints)
            .split(frame.area());

        let mut chunk_iter = chunks.iter();
        let area = *chunk_iter.next().unwrap();
        self.screen.build_top_bar(
            &area,
            self.current_directory().to_string_lossy().to_string(),
            Some(format!("Depth {}", self.user_state.depth)),
            self.config.main_box.entry_style,
        ); // TODO : move this for only resizes
        if let Some(widget) = &self.screen.top_bar {
            frame.render_widget(widget, area);
        }

        self.screen
            .main_box
            .set_block(
                self.config.main_box.block.clone().title(format!(
                    "{} Entries",
                    self.curr_node
                        .read()
                        .get_nested_entries(self.user_state.depth as usize) -1 
                )),
            );
        self.screen.main_box.render(
            &self.curr_list,
            *chunk_iter.next().unwrap(),
            frame.buffer_mut(),
            self.selected_index(),
        );
        if let Some(widget) = &self.screen.search_bar {
            let area = *chunk_iter.next().unwrap();
            frame.render_widget(widget, area);
            if self.is_inserting() {
                frame.set_cursor_position((area.x + 1 + self.user_state.search_bar_text.len() as u16,area.y + 1));
            }
        }
        if let Some(widget) = &self.screen.tooltips {
            frame.render_widget(widget, *chunk_iter.next().unwrap());
        }
    }
}
