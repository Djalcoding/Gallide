use std::{ffi::OsStr, fmt::Display, sync::Arc};

use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::{
    config::MainBoxConfig,
    explore::{
        EntryType::{File, Folder},
        tree::hierarchy::SafeNode,
    },
};

pub mod tree;

#[derive(std::cmp::PartialEq, Eq, Clone, Copy)]
pub enum EntryType {
    File,
    Folder,
    SpecialSign,
}

impl EntryType {
    fn from_is_dir(is_dir: bool) -> EntryType {
        match is_dir {
            true => Folder,
            false => File,
        }
    }
}
impl Display for EntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntryType::File => write!(f, "File"),
            EntryType::Folder => write!(f, "Folder"),
            EntryType::SpecialSign => write!(f, "Special"),
        }
    }
}

pub struct GallideEntry {
    pub node: SafeNode,
    display: Vec<Span<'static>>,
    pub entry_type: EntryType,
}
pub type GallideEntryVec = Vec<GallideEntry>;

impl GallideEntry {
    pub fn previous_directory(start: &SafeNode) -> Self {
        GallideEntry {
            node: start
                .read()
                .parent
                .clone()
                .unwrap_or(Arc::downgrade(start))
                .upgrade()
                .unwrap(),
            display: vec![Span::from("..")],
            entry_type: EntryType::SpecialSign,
        }
    }
    pub fn from_node(
        highlight_index: usize,
        highlight_length: usize,
        node: &SafeNode,
        start_path: &OsStr,
        config: &MainBoxConfig,
    ) -> Self {
        let full_display_name = node
            .path()
            .strip_prefix(start_path)
            .unwrap()
            .to_string_lossy()
            .to_string();
        let (start, string_remainder) = full_display_name.split_at(highlight_index);
        let (highlight_section, end) = string_remainder.split_at(highlight_length);
        let entry_type = EntryType::from_is_dir(node.is_dir());
        GallideEntry {
            node: node.clone(),
            display: vec![
                match entry_type {
                    EntryType::File => config.file_symbol.clone(),
                    EntryType::Folder => config.folder_symbol.clone(),
                    _ => Span::from(".."),
                },
                Span::from(start.to_owned()),
                Span::styled(highlight_section.to_owned(), config.query_match_style),
                Span::from(end.to_owned()),
            ],
            entry_type,
        }
    }
    pub fn set_type(&mut self) {}
    pub fn display(&self, selected: bool, highlight_style: Style) -> Line<'static> {
        let mut display_vec = self.display.clone();
        if selected && display_vec[0].style.fg == highlight_style.bg {
            display_vec[0] = display_vec[0].clone().style(highlight_style);
        }
        Line::from(display_vec)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct PageBounds(pub usize, pub isize);
impl PageBounds {
    pub fn page_start_index(&self) -> usize {
        self.0
    }
    pub fn page_end_index(&self) -> usize {
        self.1.unsigned_abs()
    }
    pub fn last_index(&self) -> usize {
        let index = self.page_end_index().saturating_sub(1);
        if index == 0 { 1 } else { index } // TODO : Find why this weird edge case happens
    }
    pub fn last_page(&self) -> bool {
        self.1 < 0
    }
}
