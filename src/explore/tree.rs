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
            Arc, Condvar, Mutex, RwLock, RwLockReadGuard, Weak,
            atomic::{AtomicBool, AtomicU64},
            mpsc::{Receiver, Sender, channel},
        },
        thread::{JoinHandle, sleep},
        time::Duration,
    };

    use crate::explore::{EntryType, GallideEntry, GallideEntryVec, tree::hierarchy};
    type SingleThreadMap = HashMap<PathBuf, Vec<(EntryType, PathBuf)>>;
    type MapType = RwLock<SingleThreadMap>;

    struct Node {
        pub(crate) path: PathBuf,
        pub(crate) parent: Option<Weak<RwLock<Node>>>,
        pub(crate) children: Option<Vec<SafeNode>>,
    }

    impl Node {
        pub fn is_dir(&self) -> bool {
            self.children.is_some()
        }
    }
    type SafeNode = Arc<RwLock<Node>>;
    type WeakSafeNode = Weak<RwLock<Node>>;

    pub struct AtomicElements {
        pub counter: AtomicU64,
    }

    pub struct FileHierarchy {
        pub root: SafeNode,
        pub prev_root: SafeNode,
        pub atomic: Arc<AtomicElements>,
        pub sender: Sender<SafeNode>,
        pub last_read: u64,
    }

    impl FileHierarchy {
        // pub fn get_entries(
        //     &self,
        //     path: &Path,
        //     whitelist: &str,
        //     max_depth: u16,
        //     case_sensitive: bool,
        // ) -> GallideEntryVec {
        //     let mut entries: GallideEntryVec =
        //         GallideEntryVec::with_capacity(self.entry_count() as usize);
        //     let tree = &self.atomic.map.read().unwrap();
        //     let origin = path;
        //     self.recurse(
        //         tree,
        //         origin,
        //         path,
        //         whitelist,
        //         &mut entries,
        //         1,
        //         max_depth,
        //         case_sensitive,
        //     );
        //     entries
        // }
        //
        #[allow(clippy::too_many_arguments)]
        fn recurse(&self, curr: SafeNode, origin: &Path, path: &Path, whitelist: &str) {
            let node_reader = curr.read().unwrap();
            //println!("{}", node_reader.path.display());
            for child in node_reader.children.iter().flatten() {
                self.recurse(child.clone(), origin, path, whitelist);
            }
        }
        //
        // pub fn get_all_entries(&self) -> GallideEntryVec {
        //     self.get_entries(&self.root, "", u16::MAX, false)
        // }
        //
        // pub fn entry_count(&self) -> u64 {
        //     self.atomic
        //         .counter
        //         .load(std::sync::atomic::Ordering::Relaxed)
        // }
        //
        // pub fn new_entries(&mut self) -> u64 {
        //     let out = self.entry_count() - self.last_read;
        //     self.last_read = self.entry_count();
        //     self.last_read
        // }

        pub fn new() -> Result<Self> {
            let (tx, rx): (Sender<SafeNode>, Receiver<SafeNode>) = channel();
            let atomic = Arc::from(AtomicElements {
                counter: AtomicU64::new(0),
            });
            let root = Arc::from(RwLock::from(Node {
                path: PathBuf::from(".").canonicalize().unwrap(),
                parent: None,
                children: Some(Vec::with_capacity(20)),
            }));
            let hierarchy = FileHierarchy {
                prev_root: root.clone(),
                root,
                atomic,
                sender: tx,
                last_read: 0,
            };
            let h_ref = hierarchy.atomic.clone();
            let prev_root = hierarchy.prev_root.clone();
            let root = hierarchy.root.clone();
            std::thread::spawn(move || {
                let mut queue: VecDeque<SafeNode> = VecDeque::new();
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
                            if Arc::ptr_eq(&origin, &prev_root) && !Arc::ptr_eq(&root, &prev_root) {
                                continue;
                            }
                            let mut origin_rw = origin.write().unwrap();
                            let Ok(directory_contents) = fs::read_dir(&origin_rw.path) else {
                                continue;
                            };
                            for entry in directory_contents {
                                let Ok(entry) = entry else { continue };
                                let path: PathBuf = entry.path();
                                let Ok(filetype) = entry.file_type() else {
                                    continue;
                                };
                                // TODO : ignore seen
                                if (!filetype.is_file() && !filetype.is_dir()) {
                                    continue;
                                }
                                let Ok(metadata) = entry.metadata() else {
                                    continue;
                                };
                                let node_children = if metadata.is_dir() {
                                    Some(Vec::new())
                                } else {
                                    None
                                };
                                let child_node = Arc::from(RwLock::from(Node {
                                    path,
                                    parent: Some(Arc::downgrade(&origin)),
                                    children: node_children,
                                }));
                                origin_rw
                                    .children
                                    .as_mut()
                                    .unwrap()
                                    .push(child_node.clone());
                                if metadata.is_dir() {
                                    queue.push_back(child_node.clone());
                                }
                                h_ref
                                    .counter
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            }
                            size -= 1;
                        }
                    }
                }
            });

            hierarchy.sender.send(hierarchy.root.clone());
            Ok(hierarchy)
        }

        pub fn step_back(&mut self) {
            let mut path = self.root.read().unwrap().path.clone();
            if path == *PathBuf::from(".") {
                return; 
            }
            path.pop();
            self.prev_root = self.root.clone();
            self.root = Arc::from(RwLock::from(Node {
                path,
                parent: None,
                children: Some(vec![self.prev_root.clone()]),
            }));
            self.prev_root.write().unwrap().parent = Some(Arc::downgrade(&self.root));
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
            file_hierarchy.step_back();
            file_hierarchy.step_back();
            file_hierarchy.step_back();
            file_hierarchy.step_back();
            file_hierarchy.step_back();
            file_hierarchy.step_back();
            file_hierarchy.step_back();
            sleep(Duration::from_millis(10000));
            file_hierarchy.recurse(
                file_hierarchy.root.clone(),
                &PathBuf::from("."),
                &PathBuf::from("."),
                "",
            );
        }
    }
}
