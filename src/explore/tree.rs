#[allow(unused)]
pub mod hierarchy {
    use std::{
        borrow::Cow,
        collections::{HashMap, VecDeque},
        error::Error,
        fs::{self, DirEntry, Metadata},
        io::Result,
        path::{Path, PathBuf},
        process::exit,
        sync::{
            Arc, Condvar, Mutex, RwLock, RwLockReadGuard,
            atomic::{AtomicBool, AtomicU64},
            mpsc::{Receiver, Sender, channel},
        },
        thread::{JoinHandle, sleep},
        time::Duration,
    };

    use crate::explore::{EntryType, GallideEntry, GallideEntryVec, tree::hierarchy};
    type SingleThreadMap = HashMap<PathBuf, Vec<(EntryType, PathBuf)>>;
    type MapType = RwLock<SingleThreadMap>;

    pub struct AtomicElements {
        pub map: MapType,
        pub counter: AtomicU64,
    }

    pub struct FileHierarchy {
        pub root: PathBuf,
        pub atomic: Arc<AtomicElements>,
        pub sender: Sender<PathBuf>,
        pub last_read: u64,
    }

    impl FileHierarchy {
        pub fn get_entries(
            &self,
            path: &Path,
            whitelist: &str,
            max_depth: u16,
            case_sensitive: bool,
        ) -> GallideEntryVec {
            let mut entries: GallideEntryVec =
                GallideEntryVec::with_capacity(self.entry_count() as usize);
            let tree = &self.atomic.map.read().unwrap();
            let origin = path;
            self.recurse(
                tree,
                origin,
                path,
                whitelist,
                &mut entries,
                1,
                max_depth,
                case_sensitive,
            );
            entries
        }

        #[allow(clippy::too_many_arguments)]
        fn recurse(
            &self,
            tree: &RwLockReadGuard<'_, SingleThreadMap>,
            origin: &Path,
            path: &Path,
            whitelist: &str,
            entries: &mut GallideEntryVec,
            depth: u16,
            max_depth: u16,
            case_sensitive: bool,
        ) {
            let tree_entry = tree.get(path);
            if tree_entry.is_none() {
                return;
            }
            let children = tree_entry.unwrap();
            for entry in children {
                let mut has_match = false;
                let name = entry.1.strip_prefix(origin).unwrap().to_string_lossy();
                if !whitelist.is_empty() {
                    for component in name.split('/') {
                        let matches = if case_sensitive {
                            component.starts_with(whitelist)
                        } else {
                            component
                                .get(..whitelist.len())
                                .is_some_and(|start| start.eq_ignore_ascii_case(whitelist))
                        };
                        if matches {
                            has_match = true;
                            break;
                        }
                    }
                } else {
                    has_match = true;
                }

                if (!has_match) {
                    continue;
                }

                entries.push_back(GallideEntry {
                    path: entry.1.clone(),
                    name: name.into_owned(),
                    entry_type: entry.0,
                    size: None,
                });
                if let EntryType::Folder = entry.0
                    && depth < max_depth
                {
                    self.recurse(
                        tree,
                        origin,
                        &entry.1,
                        whitelist,
                        entries,
                        depth + 1,
                        max_depth,
                        case_sensitive,
                    )
                }
            }
        }

        pub fn get_all_entries(&self) -> GallideEntryVec {
            self.get_entries(&self.root, "", u16::MAX, false)
        }

        pub fn entry_count(&self) -> u64 {
            self.atomic
                .counter
                .load(std::sync::atomic::Ordering::Relaxed)
        }

        pub fn new_entries(&mut self) -> u64 {
            let out = self.entry_count() - self.last_read;
            self.last_read = self.entry_count();
            self.last_read
        }

        pub fn new() -> Result<Self> {
            let (tx, rx): (Sender<PathBuf>, Receiver<PathBuf>) = channel();
            let atomic = Arc::from(AtomicElements {
                map: RwLock::from(HashMap::new()),
                counter: AtomicU64::new(0),
            });
            let hierarchy = FileHierarchy {
                root: PathBuf::from(".").canonicalize().unwrap(),
                atomic,
                sender: tx,
                last_read: 0,
            };
            let h_ref = hierarchy.atomic.clone();
            std::thread::spawn(move || {
                let mut queue: VecDeque<PathBuf> = VecDeque::new();
                queue.reserve(200);
                loop {
                    let origin = rx.recv();
                    if let Err(e) = origin {
                        return;
                    }
                    queue.push_back(origin.unwrap());
                    while !queue.is_empty() {
                        let mut size = queue.len();
                        while size > 0 {
                            let Some(origin) = queue.pop_back() else {
                                continue;
                            };
                            let mut children: Vec<(EntryType, PathBuf)> = Vec::new();
                            let Ok(directory_contents) = fs::read_dir(&origin) else {
                                continue;
                            };
                            for entry in directory_contents {
                                let Ok(entry) = entry else { continue };
                                let path: PathBuf = entry.path();
                                let Ok(filetype) = entry.file_type() else {
                                    continue;
                                };
                                if h_ref.map.read().unwrap().contains_key(&path)
                                    || (!filetype.is_file() && !filetype.is_dir())
                                {
                                    continue;
                                }
                                let Ok(metadata) = entry.metadata() else {
                                    continue;
                                };
                                h_ref
                                    .counter
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                let entry_type = if metadata.is_dir() {
                                    EntryType::Folder
                                } else {
                                    EntryType::File
                                };

                                children.push((entry_type, path.clone()));
                                if metadata.is_dir() {
                                    queue.push_back(path);
                                }
                            }
                            h_ref.map.write().unwrap().insert(origin, children);
                            size -= 1;
                        }
                    }
                }
            });

            hierarchy.sender.send(hierarchy.root.clone());
            Ok(hierarchy)
        }

        pub fn go_back(&mut self) {
            self.root.pop();
            self.sender.send(self.root.clone());
        }
    }

    #[cfg(test)]
    mod tests {
        use std::{
            arch::x86_64::_mm256_sll_epi64,
            path::{Path, PathBuf},
            sync::atomic::Ordering::Acquire,
            thread::sleep,
            time::{Duration, Instant},
        };

        use crate::explore::tree::hierarchy::FileHierarchy;

        #[test]
        fn make() {
            let mut file_hierarchy = FileHierarchy::new().unwrap();
            let mut next_check = Instant::now() + Duration::from_secs(3);
            file_hierarchy.go_back();
            file_hierarchy.go_back();
            file_hierarchy.go_back();
            file_hierarchy.go_back();
            file_hierarchy.go_back();
            sleep(Duration::from_millis(3000));
            for entry in file_hierarchy.get_all_entries() {
                println!("{}:{}", entry.entry_type, entry.name());
            }
            println!("{}", file_hierarchy.entry_count());
        }
    }
}
