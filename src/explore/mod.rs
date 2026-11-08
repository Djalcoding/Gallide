use std::{ffi::OsStr, fmt::Display, sync::Arc};

use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::explore::{
    EntryType::{File, Folder},
    tree::hierarchy::SafeNode,
};

pub mod tree;
pub mod ringbuffer;

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
    display: Line<'static>,
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
            display: Line::from(".."),
            entry_type: EntryType::SpecialSign,
        }
    }
    pub fn new(node: SafeNode, display: Line<'static>, entry_type: EntryType) -> Self {
        GallideEntry {
            node,
            display,
            entry_type,
        }
    }

    pub fn from_node(
        node: &SafeNode,
        start_path: &OsStr,
        highlight_index: usize,
        highlight_length: usize,
    ) -> Self {
        let full_display_name = node
            .path()
            .strip_prefix(start_path)
            .unwrap()
            .to_string_lossy()
            .to_string();
        let (start, string_remainder) = full_display_name.split_at(highlight_index);
        let (highlight_section, end) = string_remainder.split_at(highlight_length);
        GallideEntry {
            node: node.clone(),
            entry_type: EntryType::from_is_dir(node.is_dir()),
            display: Line::from(vec![
                Span::from(start.to_owned()),
                Span::styled(highlight_section.to_owned(), Style::default().blue()),
                Span::from(end.to_owned()),
            ]),
        }
    }
    pub fn set_type(&mut self) {}
    pub fn display(&self) -> &Line<'static> {
        &self.display
    }
}

#[derive(Default, Debug)]
pub struct Page {
    start: isize,
    length: usize,
    is_end: bool,
}

impl Page {
    pub(crate) fn start(&self) -> usize {
        if self.start < 0 {
            return 0;
        }
        self.start as usize
    }
    pub(crate) fn end(&self) -> usize {
        self.start() + self.length
    }
    pub(crate) fn is_end(&self) -> bool {
        self.is_end
    }
    pub(crate) fn size(&self) -> usize {
        self.length
    }
    pub(crate) fn contains(&self, cursor: usize) -> bool {
        cursor < self.end() && cursor >= self.start()
    }
}
