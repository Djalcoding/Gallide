use std::path::Path;

use djal_parser::{datastructure::ParsedData, error_handling::FileReadingError};
use ratatui::{
    style::{Color, Style, Stylize},
    text::Span,
    widgets::{Block, BorderType, Borders},
};

fn white_block() -> Block<'static> {
    Block::default()
        .bg(Color::Black)
        .fg(Color::White)
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
}

fn white_on_black() -> Style {
    Style::default().fg(Color::White).bg(Color::Black)
}

type DColor = djal_parser::color::Color;
type TColor = ratatui::style::Color;

pub fn tui_color(djal_color: Result<DColor, FileReadingError>) -> Result<TColor, FileReadingError> {
    let color: DColor = djal_color?;
    Ok(match color {
        DColor::RGB(r, g, b) => Color::Rgb(r, g, b),
        DColor::RGBA(_, _, _, 0) => Color::Reset,
        DColor::RGBA(r, g, b, _) => Color::Rgb(r, g, b),
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

fn get_color<'a>(data: &'a ParsedData, name: &'a str) -> Result<TColor, FileReadingError<'a>> {
    tui_color(data.as_color(name))
}

pub fn get_border_type(key: &str, data_map: &ParsedData) -> Option<BorderType> {
    let (_, raw_border_value) = data_map.as_raw(key).unwrap_or((0, String::from("plain")));

    match raw_border_value.to_lowercase().as_str() {
        "rounded" => Some(BorderType::Rounded),
        "plain" => Some(BorderType::Plain),
        "thick" => Some(BorderType::Thick),
        "double" => Some(BorderType::Double),
        "light2" => Some(BorderType::LightDoubleDashed),
        "light3" => Some(BorderType::LightTripleDashed),
        "light4" => Some(BorderType::LightQuadrupleDashed),
        "heavy2" => Some(BorderType::HeavyDoubleDashed),
        "heavy3" => Some(BorderType::HeavyTripleDashed),
        "heavy4" => Some(BorderType::HeavyQuadrupleDashed),
        "inquad" => Some(BorderType::QuadrantInside),
        "outquad" => Some(BorderType::QuadrantOutside),
        "none" => None,
        _ => Some(BorderType::Plain),
    }
}
pub fn get_style(background_key: &str, foreground_key: &str, data: &ParsedData) -> Style {
    Style::default()
        .bg(get_color(data, background_key).unwrap_or(Color::Black))
        .fg(get_color(data, foreground_key).unwrap_or(Color::White))
}

pub fn get_block(
    background: TColor,
    border_key: &str,
    color_key: &str,
    data: &ParsedData,
) -> Block<'static> {
    let borders = get_border_type(border_key, data);
    Block::default()
        .style(
            Style::default()
                .bg(background)
                .fg(get_color(data, color_key).unwrap_or(TColor::White)),
        )
        .border_type(borders.unwrap_or(BorderType::Plain))
        .borders(if borders.is_some() {
            Borders::ALL
        } else {
            Borders::NONE
        })
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
    pub entry_style: Style,
    pub highlight_style: Style,
    pub write_mode_style: Style,
    pub focus_symbol: String,
    pub folder_symbol: Span<'static>,
    pub file_symbol: Span<'static>,
    pub block: Block<'static>,
    pub write_mode_block: Block<'static>,
    pub display_file_size: bool,
    pub default_depth: u8,
}
impl Default for MainBoxConfig {
    fn default() -> Self {
        MainBoxConfig {
            entry_style: white_on_black(), // text & background
            write_mode_style: white_on_black(),
            highlight_style: Style::default().fg(TColor::Black).bg(TColor::White),
            block: white_block(),
            write_mode_block: white_block(),
            focus_symbol: String::from("> "),
            folder_symbol: Span::from("").style(white_on_black()),
            file_symbol: Span::from("").style(white_on_black()),
            display_file_size: true,
            default_depth: 1,
        }
    }
}

impl ParsedConstructable for MainBoxConfig {
    fn from_file(data: &ParsedData) -> Self {
        let default = Self::default();
        let style: Style = get_style("background color", "text color", data);
        let highlight_style: Style = get_style("focus color", "focus text color", data);
        let folder_symbol_color = get_color(data, "folder symbol color")
            .unwrap_or(default.folder_symbol.style.fg.unwrap());
        let file_symbol_color =
            get_color(data, "file symbol color").unwrap_or(default.file_symbol.style.fg.unwrap());
        MainBoxConfig {
            entry_style: style,
            write_mode_style: style,
            highlight_style,
            block: get_block(style.bg.unwrap(), "border type", "border color", data),
            write_mode_block: get_block(
                style.bg.unwrap(),
                "write mode border type",
                "write mode border color",
                data,
            ),
            focus_symbol: data.as_text("focus symbol").unwrap_or(default.focus_symbol),
            folder_symbol: Span::styled(
                data.as_text("folder symbol").unwrap_or_default(),
                Style::default().fg(folder_symbol_color),
            ),
            file_symbol: Span::styled(
                data.as_text("file symbol").unwrap_or_default(),
                Style::default().fg(file_symbol_color),
            ),
            display_file_size: data
                .as_boolean("display file size")
                .unwrap_or(default.display_file_size),
            default_depth: data
                .as_number("default depth")
                .unwrap_or(default.default_depth as f64) as u8,
        }
    }
}

pub struct SearchBarConfig {
    pub style: Style,
    pub block: Block<'static>,
    pub insert_mode_block: Block<'static>,
    pub enabled: bool,
}

impl Default for SearchBarConfig {
    fn default() -> Self {
        SearchBarConfig {
            style: white_on_black(),
            block: white_block().title(" Searchbar "),
            insert_mode_block: white_block().fg(Color::Red).title(" Searchbar "),
            enabled: true,
        }
    }
}

impl ParsedConstructable for SearchBarConfig {
    fn from_file(data: &ParsedData) -> Self {
        let default: Self = Self::default();
        let style = get_style("searchbar background color", "searchbar text color", data);
        let title = data
            .as_text("searchbar title")
            .unwrap_or(String::from("Searchbar"));
        Self {
            style,
            block: get_block(
                style.bg.unwrap(),
                "searchbar border type",
                "searchbar border color",
                data,
            )
            .title(title.clone()),
            insert_mode_block: get_block(
                style.bg.unwrap(),
                "searchbar border type", // TODO : change this
                "insert searchbar border color",
                data,
            )
            .title(title),
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
            text_color: get_color(data, "tooltip text color").unwrap_or(default.text_color),
            keybind_color: get_color(data, "tooltip keybind color")
                .unwrap_or(default.keybind_color),
            highlight_color: get_color(data, "tooltip highlight color")
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
    pub fn from_file(path: &'_ Path) -> Result<Config, FileReadingError<'_>> {
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
