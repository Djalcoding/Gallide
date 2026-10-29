use std::{fmt::Display, sync::Arc};

use ratatui::text::Line;

use crate::explore::{
    EntryType::{File, Folder},
    tree::hierarchy::SafeNode,
};

pub mod ringbuffer;
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
    display: Line<'static>,
    pub entry_type: EntryType,
}
pub const PAGE_SIZE: usize = 1000;
pub type GallideEntryVec = Vec<GallideEntry>;

impl GallideEntry {
    pub fn previous_directory(start:&SafeNode) -> Self {
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
    pub fn set_type(&mut self) {}
    pub fn display(&self) -> &Line<'static> {
        &self.display
    }
}
