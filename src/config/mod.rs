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
