use std::path::Path;

use djal_parser::{datastructure::ParsedData, error_handling::FileReadingError};
use tui::{style::Color, widgets::BorderType};


type DColor = djal_parser::color::Color;
type TColor = tui::style::Color;

pub fn tui_color(
    djal_color: Result<DColor, FileReadingError>,
) -> Result<TColor, FileReadingError> {
    let color: DColor= djal_color?;
    Ok(match color {
        DColor::RGB(r, g, b) => Color::Rgb(r, g, b),
        DColor::RGBA(_, _, _,0) => Color::Reset,
        DColor::RGBA(r, g, b,_) => Color::Rgb(r, g, b),
        DColor::PALETTE(0) => Color::Black,
        DColor::PALETTE(1) => Color::Red,
        DColor::PALETTE(2) => Color::Green,
        DColor::PALETTE(3) => Color::Yellow,
        DColor::PALETTE(4) => Color::Blue,
        DColor::PALETTE(5) => Color::Magenta,
        DColor::PALETTE(6) => Color::Cyan,
        DColor::PALETTE(7) => Color::White,
        DColor::PALETTE(8) => Color::DarkGray,
        DColor::PALETTE(9) => Color::LightRed,
        DColor::PALETTE(10) => Color::LightGreen,
        DColor::PALETTE(11) => Color::LightYellow,
        DColor::PALETTE(12) => Color::LightBlue,
        DColor::PALETTE(13) => Color::LightMagenta,
        DColor::PALETTE(14) => Color::LightCyan,
        DColor::PALETTE(15) => Color::Gray,
        DColor::PALETTE(16..) => Color::Reset,
    })
}

pub fn get_border_type(key: &str, data_map: &ParsedData) -> Option<BorderType> {
    let (_, raw_border_value) = data_map.as_raw(key).unwrap_or((0, String::from("plain")));

    match raw_border_value.to_lowercase().as_str() {
        "rounded" => Some(BorderType::Rounded),
        "plain" => Some(BorderType::Plain),
        "thick" => Some(BorderType::Thick),
        "double" => Some(BorderType::Double),
        "none" => None,
        _ => None,
    }
}

trait ParsedConstructable {
    fn from_file(data: &ParsedData) -> Self;
}

pub struct BorderConfig {
    pub border_color: Color,
    pub border_type: Option<BorderType>,
}

pub struct DirectoryLineConfig {
    pub display: bool,
}
impl Default for DirectoryLineConfig {
    fn default() -> Self {
        DirectoryLineConfig { display: true }
    }
}
impl ParsedConstructable for DirectoryLineConfig {
    fn from_file(data: &ParsedData) -> Self {
        Self {
            display: data
                .as_boolean("display directory")
                .unwrap_or(Self::default().display),
        }
    }
}

pub struct MainBoxConfig {
    pub border_config: BorderConfig,
    pub text_color: Color,
    pub focus_text_color: Color,
    pub background_color: Option<Color>,
    pub focus_color: Color,
    pub focus_symbol: String,
    pub directory_symbol: String,
    pub directory_symbol_color: Color,
    pub file_symbol: String,
    pub file_symbol_color: Color,
    pub title: String,
}
impl Default for MainBoxConfig {
    fn default() -> Self {
        MainBoxConfig {
            border_config: BorderConfig {
                border_color: Color::White,
                border_type: Some(BorderType::Plain),
            },
            text_color: Color::White,
            focus_text_color: Color::Black,
            background_color: Some(Color::Black),
            focus_color: Color::White,
            focus_symbol: String::from("> "),
            directory_symbol: String::new(),
            directory_symbol_color: Color::White,
            file_symbol: String::new(),
            file_symbol_color: Color::White,
            title: String::from("Directories"),
        }
    }
}

impl ParsedConstructable for MainBoxConfig {
    fn from_file(data: &ParsedData) -> Self {
        let default = Self::default();
        MainBoxConfig {
            border_config: BorderConfig {
                border_color: tui_color(data.as_color("border color"))
                    .unwrap_or(Self::default().border_config.border_color),
                border_type: get_border_type("border type", data),
            },
            text_color: tui_color(data.as_color("text color"))
                .unwrap_or(Self::default().text_color),
            focus_text_color: tui_color(data.as_color("focus text color"))
                .unwrap_or(Self::default().text_color),
            background_color: tui_color(data.as_color("background color")).ok(),
            directory_symbol: data
                .as_text("directory symbol")
                .unwrap_or(default.directory_symbol),
            directory_symbol_color: tui_color(data.as_color("directory symbol color"))
                .unwrap_or(default.directory_symbol_color),
            file_symbol: data.as_text("file symbol").unwrap_or(default.file_symbol),
            file_symbol_color: tui_color(data.as_color("file symbol color"))
                .unwrap_or(default.file_symbol_color),
            focus_color: tui_color(data.as_color("focus color")).unwrap_or(default.focus_color),
            focus_symbol: data.as_text("focus symbol").unwrap_or(default.focus_symbol),
            title: data.as_text("main title").unwrap_or(default.title),
        }
    }
}

pub struct SearchBarConfig {
    pub border_config: BorderConfig,
    pub insert_mode_border_config: BorderConfig,
    pub text_color: Color,
    pub background_color: Option<Color>,
    pub title: String,
    pub enabled: bool,
}

impl Default for SearchBarConfig {
    fn default() -> Self {
        SearchBarConfig {
            border_config: BorderConfig {
                border_color: Color::White,
                border_type: Some(BorderType::Plain),
            },
            insert_mode_border_config: BorderConfig {
                border_color: Color::Red,
                border_type: Some(BorderType::Plain),
            },
            text_color: Color::White,
            background_color: Some(Color::Black),
            title: String::from("Searchbar"),
            enabled: true,
        }
    }
}

impl ParsedConstructable for SearchBarConfig {
    fn from_file(data: &ParsedData) -> Self {
        let default: Self = Self::default();
        Self {
            border_config: BorderConfig {
                border_color: tui_color(data.as_color("searchbar border color"))
                    .unwrap_or(default.border_config.border_color),
                border_type: get_border_type("searchbar border type", data),
            },
            insert_mode_border_config: BorderConfig {
                border_color: tui_color(data.as_color("insert searchbar color"))
                    .unwrap_or(default.insert_mode_border_config.border_color),
                border_type: get_border_type("insert searchbar border type", data),
            },
            text_color: tui_color(data.as_color("searchbar text color"))
                .unwrap_or(default.text_color),
            background_color: tui_color(data.as_color("searchbar background color")).ok(),
            title: data.as_text("searchbar title").unwrap_or(default.title),
            enabled: data
                .as_boolean("enable searchbar")
                .unwrap_or(default.enabled),
        }
    }
}
pub struct TooltipConfig {
    pub display: bool,
    pub text_color: Color,
    pub keybind_color: Color,
    pub highlight_color: Color,
}
impl Default for TooltipConfig {
    fn default() -> Self {
        TooltipConfig {
            display: true,
            text_color: Color::Black,
            keybind_color: Color::White,
            highlight_color: Color::White,
        }
    }
}

impl ParsedConstructable for TooltipConfig {
    fn from_file(data: &ParsedData) -> Self {
        let default: Self = Self::default();
        Self {
            display: data
                .as_boolean("display tooltips")
                .unwrap_or(default.display),
            text_color: tui_color(data.as_color("tooltip text color"))
                .unwrap_or(default.text_color),
            keybind_color: tui_color(data.as_color("tooltip keybind color"))
                .unwrap_or(default.keybind_color),
            highlight_color: tui_color(data.as_color("tooltip highlight color"))
                .unwrap_or(default.highlight_color),
        }
    }
}

#[derive(Default)]
pub struct Config {
    pub directory_line: DirectoryLineConfig,
    pub main_box: MainBoxConfig,
    pub search_bar: SearchBarConfig,
    pub tooltips: TooltipConfig,
    pub case_sensitive: bool,
}

impl Config {
    pub fn from_file(path: &Path) -> Result<Config, FileReadingError> {
        let parsed_data = ParsedData::from_file(path)?;

        Ok(Config {
            case_sensitive: parsed_data.as_boolean("case sensitive").unwrap_or(false),
            directory_line: DirectoryLineConfig::from_file(&parsed_data),
            main_box: MainBoxConfig::from_file(&parsed_data),
            search_bar: SearchBarConfig::from_file(&parsed_data),
            tooltips: TooltipConfig::from_file(&parsed_data),
        })
    }
}
