use std::{fmt::Display};

use crate::explore::{
        EntryType::{File, Folder}, ringbuffer::RingBuffer, tree::hierarchy::SafeNode,
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
    display_name: String,
    pub entry_type: EntryType,
}
pub const PAGE_SIZE:usize = 1000;
pub type GallideEntryVec = RingBuffer<GallideEntry, PAGE_SIZE>;

impl GallideEntry {
    pub fn new(node: SafeNode, display_name: String, entry_type: EntryType) -> Self {
        GallideEntry {
            node,
            display_name,
            entry_type,
        }
    }
    pub fn set_type(&mut self) {}
    pub fn name(&self) -> &String {
        &self.display_name
    }
}
