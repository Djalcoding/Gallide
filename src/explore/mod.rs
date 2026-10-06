use std::{
    cmp::Ordering::{Greater, Less},
    collections::VecDeque,
    io::Error,
    path::{Path, PathBuf},
};

use ratatui::{
    text::{Line, Span},
    widgets::List,
};
use walkdir::WalkDir;

use crate::{
    config::MainBoxConfig,
    explore::EntryType::{File, Folder},
};

#[derive(std::cmp::PartialEq, Eq, Clone, Copy)]
pub enum EntryType {
    File,
    Folder,
    SpecialSign,
}

pub struct GallideEntry {
    path: PathBuf,
    name: String,
    pub entry_type: EntryType,
    size: Option<u64>,
}

pub type GallideEntryVec = VecDeque<GallideEntry>;

impl GallideEntry {
    pub fn new(path: PathBuf, name: String, entry_type: EntryType, size: Option<u64>) -> Self {
        GallideEntry {
            path,
            name,
            entry_type,
            size,
        }
    }
    pub fn set_type(&mut self) {}
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
    pub fn name(&self) -> &String {
        &self.name
    }
    pub fn size(&self) -> Option<u64> {
        self.size
    }

    pub(crate) fn to_line(&self, highlight_length: u16, config: &MainBoxConfig) -> Line<'static> {
        let symbol = match self.entry_type {
            EntryType::File => config.file_symbol.clone(),
            EntryType::Folder => config.folder_symbol.clone(),
            _ => Span::from(""),
        };
        let (highlight, rest) = self.name.split_at(highlight_length as usize);

        let highlight = Span::styled(
            String::from(highlight),
            config
                .entry_style
                .to_owned()
                .fg(ratatui::style::Color::Green),
        ); // TODO : color
        let text = Span::styled(String::from(rest), config.entry_style.to_owned());

        Line::from(vec![symbol, highlight, text])
    }
}

fn cmp_entries(a: &GallideEntry, b: &GallideEntry) -> std::cmp::Ordering {
    if a.entry_type == EntryType::SpecialSign || (a.entry_type == Folder && b.entry_type == File) {
        return Less;
    } else if b.entry_type == EntryType::SpecialSign
        || (a.entry_type == File && b.entry_type == Folder)
    {
        return Greater;
    }
    if a.name().starts_with('.') {
        return Greater;
    } else if b.name().starts_with('.') {
        return Less;
    }
    a.name().to_lowercase().cmp(&b.name().to_lowercase())
}

pub fn get_folder_contents(current_folder: &Path, depth: u8) -> Result<GallideEntryVec, Error> {
    let mut entries: GallideEntryVec = GallideEntryVec::new();
    entries.reserve(300);
    let mut previous_folder: PathBuf = current_folder.to_path_buf();
    previous_folder.pop();
    entries.push_back(GallideEntry::new(
        previous_folder,
        String::from(".."),
        EntryType::SpecialSign,
        None,
    ));
    for element in WalkDir::new(current_folder)
        .max_depth(depth as usize)
        .into_iter()
        .flatten()
    {
        let metadata = element.metadata()?;
        let path = element.path();
        let name = path.strip_prefix(current_folder).unwrap().to_string_lossy(); // TODO : Candy
        if name.is_empty() {
            continue;
        }
        entries.push_back(GallideEntry::new(
            path.to_path_buf(),
            name.to_string(),
            if metadata.is_file() {
                EntryType::File
            } else {
                EntryType::Folder
            },
            None,
        ))
    }
    entries.make_contiguous().sort_by(cmp_entries);
    Ok(entries)
}

pub fn build_entry_list(
    entries: &GallideEntryVec,
    highlights: Vec<u16>,
    config: &MainBoxConfig,
) -> List<'static> {
    let list_items = entries
        .iter()
        .enumerate()
        .map(|(i, e)| e.to_line(highlights[i], config))
        .collect::<Vec<Line>>();

    List::new(list_items)
        .block(
            config
                .block
                .clone()
                .title(format!(" {} Entries ", entries.len())),
        )
        .style(config.entry_style)
        .highlight_style(config.highlight_style)
        .highlight_symbol(config.focus_symbol.clone())
}
