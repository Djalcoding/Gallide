pub mod worker;
#[allow(unused)]
pub mod hierarchy {

    use std::{
        borrow::Cow,
        cmp::Ordering::{Greater, Less},
        collections::{HashMap, VecDeque},
        error::Error,
        fmt::format,
        fs::{self, DirEntry, Metadata},
        io::Result,
        os::unix::thread,
        path::{self, Path, PathBuf},
        process::exit,
        str::CharIndices,
        sync::{
            Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard, Weak,
            atomic::{
                AtomicBool, AtomicU64,
                Ordering::{self, Acquire, Relaxed},
            },
            mpsc::{Receiver, Sender, channel},
        },
        thread::{JoinHandle, sleep},
        time::Duration,
    };

    use ratatui::{
        style::{Style, Styled},
        text::{Line, Span},
    };

    use crate::explore::{
        EntryType, GallideEntry, GallideEntryVec,
        tree::{
            hierarchy::{self},
            worker::{BFSQueueEntry, BFSThread},
        },
    };

    pub struct Node {
        pub(crate) parent: Option<WeakSafeNode>,
        pub(crate) children: Option<Vec<SafeNode>>,
        pub(crate) depths: Vec<usize>,
    }

    impl Node {
        fn new(parent: Option<WeakSafeNode>, children: Option<Vec<SafeNode>>) -> Node {
            Node {
                parent,
                children,
                depths: vec![1, 0],
            }
        }

        pub fn is_same(left: &SafeNode, right: &SafeNode) -> bool {
            Arc::ptr_eq(left, right)
        }

        pub fn is_dir(&self) -> bool {
            self.children.is_some()
        }

        pub fn get_nested_entries(&self, depth: usize) -> usize {
            let mut sum = 1;
            for index in 1..=std::cmp::min(self.depths.len() - 1, depth) {
                sum += self.depths[index];
            }
            sum
        }

        pub fn nested_entries(&self) -> usize {
            self.get_nested_entries(self.depths.len() - 1)
        }
    }
    pub struct TreeNode {
        data: RwLock<Node>,
        path: PathBuf,
    }

    impl TreeNode {
        pub fn read(&self) -> RwLockReadGuard<'_, Node> {
            self.data.read().unwrap()
        }
        pub fn write(&self) -> RwLockWriteGuard<'_, Node> {
            self.data.write().unwrap()
        }

        pub fn new(
            path: PathBuf,
            parent: Option<WeakSafeNode>,
            children: Option<Vec<SafeNode>>,
        ) -> Arc<Self> {
            Arc::from(TreeNode {
                path,
                data: RwLock::from(Node::new(parent, children)),
            })
        }

        pub fn path(&self) -> &Path {
            &self.path
        }
        pub(crate) fn order(node1: &SafeNode, node2: &SafeNode) -> std::cmp::Ordering {
            let reader_1 = node1.read();
            let reader_2 = node2.read();
            let is_1_dir = reader_1.is_dir();
            let is_2_dir = reader_2.is_dir();
            let filename1 = node1.path().file_name().unwrap().to_str().unwrap(); // maybe a copy is better ?
            let filename2 = node1.path().file_name().unwrap().to_str().unwrap();
            if is_1_dir && !is_2_dir {
                return Less;
            } else if !is_1_dir && is_2_dir
                || filename1.starts_with('.') && !filename2.starts_with('.')
            {
                return Greater;
            } else if filename2.starts_with('.') && !filename1.starts_with('.') {
                return Less;
            }
            filename1.cmp(filename2)
        }
        fn filename_starts_with(&self, query: &str) -> bool {
            let file_name = self.path().file_name();
            let Some(file_name) = file_name else {
                return false;
            };
            file_name.to_str().unwrap().starts_with(query)
        }

        fn filename_index(&self) -> usize {
            self.path().as_os_str().len()
                - match self.path().file_name() {
                    Some(l) => l.len(),
                    None => 0,
                }
        }

        pub(crate) fn update_parents(&self) {
            let mut target = self.read().parent.clone();

            let children_count = if let Some(vec) = &self.read().children {
                vec.len()
            } else {
                1
            };
            self.write().depths[1] = children_count;
            let mut depth = 2;
            while let Some(ref parent_node) = target {
                let Some(parent_node) = parent_node.upgrade() else {
                    continue;
                };
                if parent_node.read().depths.len() <= depth {
                    parent_node.write().depths.push(children_count);
                } else {
                    parent_node.write().depths[depth] += children_count;
                }
                target = parent_node.read().parent.clone();

                depth += 1;
            }
        }
    }
    pub type SafeNode = Arc<TreeNode>;

    type WeakSafeNode = Weak<TreeNode>;

    pub struct FileHierarchy {
        // TODO private
        pub root: SafeNode,
        pub prev_root: SafeNode,
        sender: Sender<BFSQueueEntry>,
    }

    pub struct EntryListConfiguration<'a> {
        pub whitelist: &'a str,
        pub max_size: usize,
        pub skips: usize,
        pub max_depth: usize,
    }

    impl EntryListConfiguration<'_> {
        pub(crate) fn get_ideal_queue_length(&self) -> usize {
            self.max_size * self.max_depth * 100
        }
    }

    impl FileHierarchy {
        pub fn get_entries(
            &self,
            buffer: &mut GallideEntryVec,
            start: &SafeNode,
            monitored_node: Option<SafeNode>,
            mut config: EntryListConfiguration,
        ) -> Option<usize> {
            let mut cursor_location: Option<usize> = None;
            buffer.clear();
            buffer.push(GallideEntry::previous_directory(start));
            let mut queue: VecDeque<(SafeNode, Option<usize>)> =
                VecDeque::with_capacity(config.get_ideal_queue_length());
            let start_path = start.path();
            queue.push_back((start.clone(), None));
            let mut depth: usize = 1;
            'main_loop: while (!queue.is_empty() && depth <= config.max_depth) {
                let size = queue.len();
                for _ in 0..size {
                    let (parent_node, mut parent_highlight_index) = queue.pop_front().unwrap();
                    let parent_reader = parent_node.read();
                    if parent_highlight_index.is_none()
                        && parent_node.path() != start_path
                        && parent_node.filename_starts_with(config.whitelist)
                    {
                        parent_highlight_index = Some(
                            parent_node
                                .filename_index()
                                .saturating_sub(start_path.as_os_str().len() + 1),
                        )
                    }
                    for child_node in parent_reader.children.iter().flatten() {
                        let child_reader = child_node.read();
                        let child_highlight_index: Option<usize> = match parent_highlight_index {
                            Some(index) => Some(index),
                            None => {
                                if child_node.filename_starts_with(config.whitelist) {
                                    Some(
                                        child_node
                                            .filename_index()
                                            .saturating_sub(start_path.as_os_str().len() + 1),
                                    )
                                } else {
                                    None
                                }
                            }
                        };
                        let matches_query: bool = parent_highlight_index.is_some()
                            || child_node.filename_starts_with(config.whitelist);
                        if matches_query && config.skips == 0 {
                            if let Some(ref node) = monitored_node
                                && Node::is_same(node, child_node)
                            {
                                cursor_location = Some(buffer.len());
                            }
                            let full_display_name = child_node
                                .path()
                                .strip_prefix(start_path)
                                .unwrap()
                                .to_string_lossy()
                                .to_string();
                            let (start, string_remainder) =
                                full_display_name.split_at(child_highlight_index.unwrap_or(0));
                            let (highlight_section, end) =
                                string_remainder.split_at(config.whitelist.len());
                            let entry = GallideEntry {
                                node: child_node.clone(),
                                entry_type: EntryType::from_is_dir(child_reader.is_dir()),
                                display: Line::from(vec![
                                    Span::from(start.to_owned()),
                                    Span::styled(
                                        highlight_section.to_owned(),
                                        Style::default().blue(),
                                    ),
                                    Span::from(end.to_owned()),
                                ]),
                            };
                            buffer.push(entry);
                        }
                        config.skips = config.skips.saturating_sub(1);
                        if buffer.len() == config.max_size {
                            break 'main_loop;
                        }
                        if child_reader.is_dir() {
                            queue.push_back((child_node.clone(), child_highlight_index));
                        }
                    }
                }
                depth += 1;
            }
            cursor_location
        }

        pub fn new() -> Result<Self> {
            let root = TreeNode::new(
                PathBuf::from(".").canonicalize().unwrap(),
                None,
                Some(Vec::with_capacity(20)),
            );
            let thread_count =  2;
            let (tx, rx): (Sender<BFSQueueEntry>, Receiver<BFSQueueEntry>) = channel();
            let bored_queue_counter: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
            let mut workers: Vec<BFSThread> = Vec::with_capacity(thread_count - 1);
            for _ in 0..thread_count - 1 {
                workers.push(BFSThread::spin(bored_queue_counter.clone(), tx.clone()));
            }
            let hierarchy = FileHierarchy {
                prev_root: root.clone(),
                root,
                sender: tx
            };
            std::thread::spawn(move || Self::contractor_thread(rx, workers));

            hierarchy.sender.send((hierarchy.root.clone(), None));
            Ok(hierarchy)
        }

        fn contractor_thread(
            incoming_dispatches: Receiver<BFSQueueEntry>,
            workers: Vec<BFSThread>,
        ) {
            loop {
                let task = incoming_dispatches.recv().unwrap();
                let mut found_worker = false;
                for idx in 0..=workers.len() {
                    if idx == workers.len() { // borrow checker :(
                        workers[0].dispatch(task);
                        break;
                    }
                    if workers[idx].bored() {
                        workers[idx].dispatch(task);
                        found_worker = true;
                        break;
                    }
                }
            }
        }

        pub fn step_back(&mut self) -> SafeNode {
            let mut path = self.root.path().to_path_buf();
            if !path.pop() {
                return self.root.clone();
            }
            self.prev_root = self.root.clone();
            self.root = TreeNode::new(path, None, Some(vec![self.prev_root.clone()]));
            self.prev_root.write().parent = Some(Arc::downgrade(&self.root));
            let mut writer = self.root.write();
            writer.depths[1] = 1;
            for element in self.prev_root.read().depths.iter().skip(1) {
                writer.depths.push(*element);
            }
            drop(writer);
            self.sender
                .send((self.root.clone(), Some(self.prev_root.clone())));
            self.root.clone()
        }
    }

    #[cfg(test)]
    mod tests {
        use std::{
            arch::x86_64::_mm256_sll_epi64,
            path::{Path, PathBuf},
            sync::atomic::Ordering::{Acquire, Relaxed},
            thread::sleep,
            time::{Duration, Instant},
        };

        use crate::explore::{GallideEntryVec, tree::hierarchy::{EntryListConfiguration, FileHierarchy}};
        #[test]
        fn make() {
            let mut file_hierarchy = FileHierarchy::new().unwrap();
            file_hierarchy.step_back();
            file_hierarchy.step_back();
            file_hierarchy.step_back();
            file_hierarchy.step_back();
            file_hierarchy.step_back();
            let mut buffer = GallideEntryVec::new();
            loop {
                let entries = file_hierarchy.root.read().nested_entries();
                println!("{} :  {entries}", file_hierarchy.root.path().display())
            }
            // file_hierarchy.get_entries(&mut buffer, &file_hierarchy.root, None, EntryListConfiguration { whitelist: "", max_size: 100, skips: 0, max_depth: 100 });
            // for entry in buffer {
            //     println!("{}, {}", entry.entry_type, entry.display)
            // }
        }
    }
}
