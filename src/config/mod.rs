use std::path::Path;

use djal_parser::{datastructure::ParsedData, error_handling::FileReadingError};
use tui::{style::Color, widgets::BorderType};

pub fn tui_color(djal_color: djal_parser::color::Color) -> tui::style::Color {
    Color::Rgb(djal_color.red(), djal_color.green(), djal_color.blue())
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

struct BorderConfig {
    pub border_color: Color,
    pub border_type: Option<BorderType>,
}

struct DirectoryLineConfig {
    pub display: bool,
}
struct MainBoxConfig {
    pub border_config: BorderConfig,
    pub text_color: Color,
    pub background_color: Option<Color>,
    pub focus_color: Color,
    pub focus_symbol: String,
    pub directory_symbol: String,
    pub directory_symbol_color: Color,
    pub file_symbol: String,
    pub file_symbol_color: Color,
    pub title: String,
}
struct SearchBarConfig {
    pub border_config: BorderConfig,
    pub insert_mode_border_config: BorderConfig,
    pub text_color: Color,
    pub background_color: Option<Color>,
    pub title: String,
    pub enabled: bool,
}
struct TooltipConfig {
    pub display: bool,
    pub tooltip_color: Color,
    pub tooltip_keybind_color: Color,
    pub tooltip_background_color: Color,
}

pub struct Config {
    pub directory_line: DirectoryLineConfig,
    pub main_box: MainBoxConfig,
    pub search_bar: SearchBarConfig,
    pub tooltips: TooltipConfig,
    pub case_sensitive: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            directory_line: DirectoryLineConfig { display: true },
            main_box: MainBoxConfig {
                border_config: BorderConfig {
                    border_color: Color::White,
                    border_type: Some(BorderType::Plain),
                },
                text_color: Color::White,
                background_color: Some(Color::Black),
                focus_color: Color::White,
                focus_symbol: String::from("> "),
                directory_symbol: String::new(),
                directory_symbol_color: Color::White,
                file_symbol: String::new(),
                file_symbol_color: Color::White,
                title: String::from("Directories"),
            },
            case_sensitive: false,
            search_bar: SearchBarConfig {
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
            },
            tooltips: TooltipConfig {
                display: true,
                tooltip_color: Color::Black,
                tooltip_keybind_color: Color::White,
                tooltip_background_color: Color::White,
            },
        }
    }
}

type DColor = djal_parser::color::Color;

impl Config {
    pub fn from_file(path: &Path) -> Result<Config, FileReadingError> {
        let parsed_data = ParsedData::from_file(path)?;

        let color = |c: DColor| move || c.clone();
        let white = color(DColor::from_hexadecimal("#FFFFFF").unwrap());
        let red = color(DColor::from_hexadecimal("#FF0000").unwrap());

        let tooltip_background_color = parsed_data
            .as_color("tooltip background color")
            .unwrap_or(DColor::rgb(255, 255, 255));

        let directory_line = DirectoryLineConfig { display: true };
        let main_box = MainBoxConfig {
            border_config:BorderConfig { border_color: (), border_type: () }
        };
        let search_bar = SearchBarConfig { };
        let tooltips = TooltipConfig {};

        Ok(Config {
            case_sensitive: parsed_data.as_boolean("case sensitive").unwrap_or(false),
            directory_line,
            main_box,
            search_bar,
            tooltips

            // border_color: tui_color(parsed_data.as_color("border color").unwrap_or(white())),
            // search_bar_default_color: Some(tui_color(
            //     parsed_data.as_color("search bar color").unwrap_or(white()),
            // )),
            // insert_mode_color: tui_color(
            //     parsed_data
            //         .as_color("insert mode search bar color")
            //         .unwrap_or(red()),
            // ),
            // background_color: Some(tui_color(
            //     parsed_data.as_color("background color").unwrap_or_default(),
            // )),
            // focus_color: Some(tui_color(
            //     parsed_data.as_color("focus color").unwrap_or(white()),
            // )),
            // focus_symbol: parsed_data
            //     .as_text("focus text")
            //     .unwrap_or(String::from(">")),
            // directory_symbol: parsed_data.as_text("directory symbol").unwrap_or_default(),
            // directory_symbol_color: tui_color(
            //     parsed_data
            //         .as_color("directory symbol color")
            //         .unwrap_or_default(),
            // ),
            // file_symbol: parsed_data.as_text("file symbol").unwrap_or_default(),
            // file_symbol_color: tui_color(
            //     parsed_data.as_color("file symbol color").unwrap_or(white()),
            // ),
            // display_tooltips: parsed_data.as_boolean("display tooltips").unwrap_or(true),
            // tooltip_background_color: tui_color(tooltip_background_color.clone()),
            // tooltip_color: tui_color(
            //     parsed_data
            //         .as_color("tooltip text color")
            //         .unwrap_or(tooltip_background_color.clone().inverted()),
            // ),
            // tooltip_keybind_color: tui_color(
            //     parsed_data
            //         .as_color("tooltip keybind color")
            //         .unwrap_or(tooltip_background_color.clone()),
            // ),
            // case_sensitive: parsed_data.as_boolean("case sensitive").unwrap_or_default(),
            // title: parsed_data
            //     .as_text("main title")
            //     .unwrap_or(String::from("Directories")),
            // search_bar_title: parsed_data
            //     .as_text("search bar title")
            //     .unwrap_or(String::from("Search bar")),
            // border_type: get_border_type("border type", &parsed_data),
            // search_bar_border_type: get_border_type("search bar border type", &parsed_data),
            // display_directory: parsed_data.as_boolean("display directory").unwrap_or(true),
            // enable_searchbar: parsed_data.as_boolean("enable searchbar").unwrap_or(true),
        })
    }
}
