use std::path::Path;

use tui::{style::Color, widgets::BorderType};
use djal_parser::{datastructure::ParsedData, error_handling::FileReadingError};


pub fn tui_color(djal_color:djal_parser::color::Color) -> tui::style::Color {
    Color::Rgb(djal_color.red(), djal_color.green(), djal_color.blue())
}

pub fn get_border_type(key:&str, data_map:&ParsedData) -> Option<BorderType>{
        let (_, raw_border_value) = data_map.as_raw(key).unwrap_or((0,String::from("plain")));
        
        match raw_border_value.to_lowercase().as_str() {
            "rounded" => Some(BorderType::Rounded),
            "plain" => Some(BorderType::Plain),
            "thick" => Some(BorderType::Thick),
            "double" => Some(BorderType::Double),
            "none" => None,
            _ => None
        }
}

pub struct Config {
    pub border_color: Color,
    pub field_color: Color,
    pub search_bar_default_color: Option<Color>,
    pub insert_mode_color: Color,
    pub background_color: Option<Color>,
    pub focus_color: Option<Color>,
    pub focus_symbol: String,
    pub directory_symbol: String,
    pub directory_symbol_color: Color,
    pub file_symbol: String,
    pub file_symbol_color: Color,
    pub display_tooltips: bool,
    pub tooltip_color: Color,
    pub tooltip_keybind_color: Color,
    pub tooltip_background_color: Color,
    pub case_sensitive: bool,
    pub search_bar_title: String,
    pub title: String,
    pub border_type: Option<BorderType>,
    pub search_bar_border_type: Option<BorderType>,
    pub display_directory: bool,
    pub enable_searchbar: bool
}

impl Default for Config {
    fn default() -> Self {
        Config {
            display_directory: true,
            insert_mode_color: Color::Red,
            border_color: Color::White,
            field_color: Color::White,
            background_color: Some(Color::White),
            search_bar_default_color: Some(Color::White),
            focus_color: Some(Color::White),
            focus_symbol: String::from("> "),
            directory_symbol: String::from(" "),
            directory_symbol_color: Color::White,
            file_symbol: String::from("󰈔 "),
            file_symbol_color: Color::White,
            display_tooltips: true,
            tooltip_color: Color::Black,
            tooltip_background_color: Color::White,
            tooltip_keybind_color: Color::White,
            case_sensitive: false,
            title: String::from("Directories"),
            search_bar_title: String::from("Search bar"),
            border_type: Some(BorderType::Plain),
            search_bar_border_type: Some(BorderType::Plain),
            enable_searchbar:true,
        }
    }
}

type DColor = djal_parser::color::Color;


impl Config {
    pub fn from_file(path: &Path) -> Result<Config, FileReadingError> {
        let parsed_data = ParsedData::from_file(path)?;
        
        let color = |c:DColor| {move || {c.clone()}};
        let white = color(DColor::from_hexadecimal("#FFFFFF").unwrap());
        let red = color(DColor::from_hexadecimal("#FF0000").unwrap());

        let tooltip_background_color = parsed_data.as_color("tooltip background color").unwrap_or(DColor::rgb(255,255,255));

        Ok(Config {
            field_color: tui_color(parsed_data.as_color("field color").unwrap_or(white())),
            border_color: tui_color(parsed_data.as_color("border color").unwrap_or(white())),
            search_bar_default_color: Some(tui_color(parsed_data.as_color("search bar color").unwrap_or(white()))),
            insert_mode_color: tui_color(parsed_data.as_color("insert mode search bar color").unwrap_or(red())),
            background_color: Some(tui_color(parsed_data.as_color("background color").unwrap_or_default())),
            focus_color: Some(tui_color(parsed_data.as_color("focus color").unwrap_or(white()))),
            focus_symbol: parsed_data.as_text("focus text").unwrap_or(String::from(">")),
            directory_symbol: parsed_data.as_text("directory symbol").unwrap_or_default(),
            directory_symbol_color: tui_color(parsed_data.as_color("directory symbol color").unwrap_or_default()),
            file_symbol: parsed_data.as_text("file symbol").unwrap_or_default(),
            file_symbol_color: tui_color(parsed_data.as_color("file symbol color").unwrap_or(white())),
            display_tooltips: parsed_data.as_boolean("display tooltips").unwrap_or(true),
            tooltip_background_color: tui_color(tooltip_background_color.clone()),
            tooltip_color: tui_color(parsed_data.as_color("tooltip text color").unwrap_or(tooltip_background_color.clone().inverted())),
            tooltip_keybind_color: tui_color(parsed_data.as_color("tooltip keybind color").unwrap_or(tooltip_background_color.clone())),
            case_sensitive: parsed_data.as_boolean("case sensitive").unwrap_or_default(),
            title : parsed_data.as_text("main title").unwrap_or(String::from("Directories")),
            search_bar_title: parsed_data.as_text("search bar title").unwrap_or(String::from("Search bar")),
            border_type: get_border_type("border type", &parsed_data),
            search_bar_border_type: get_border_type("search bar border type", &parsed_data),
            display_directory:parsed_data.as_boolean("display directory").unwrap_or(true),
            enable_searchbar:parsed_data.as_boolean("enable searchbar").unwrap_or(true),
        })
    }

    pub fn insert_mode_color(&self) -> Color {
        self.insert_mode_color
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

    pub fn focus_symbol(&self) -> &String{
        &self.focus_symbol
    }

    pub fn directory_symbol(&self) -> &String {
        &self.directory_symbol
    }
    pub fn file_symbol(&self) -> &String {
        &self.file_symbol
    }
    pub fn file_symbol_color(&self) -> Color {
        self.file_symbol_color
    }
    pub fn directory_symbol_color(&self) -> Color {
        self.directory_symbol_color
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
