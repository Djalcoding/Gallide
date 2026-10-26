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

    use crate::explore::{
        EntryType, GallideEntry, GallideEntryVec, PAGE_SIZE,
        tree::hierarchy::{self, EntryCreationPolicy::Skip},
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
        fn order(node1: &SafeNode, node2: &SafeNode) -> std::cmp::Ordering {
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

        pub(crate) fn update_parents(&self) {
            let mut target = self.read().parent.clone();

            let children_count = if let Some(vec) = &self.read().children {
                vec.len()
            } else {
                1
            };
            self.write().depths[1] = children_count;
            let mut depth = 1;
            while let Some(ref parent_node) = target {
                let parent_node = parent_node.upgrade().unwrap();
                if parent_node.read().depths.len() <= depth {
                    parent_node.write().depths.push(children_count + 1);
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
        pub root: SafeNode,
        pub prev_root: SafeNode,
        pub sender: Sender<SafeNode>,
    }

    #[derive(Clone, Copy)]
    pub enum EntryCreationPolicy {
        Skip(usize),
        Tail,
    }

    impl EntryCreationPolicy {
        pub(crate) fn should_skip(&self) -> bool {
            match self {
                Self::Tail => false,
                Self::Skip(skips_left) => *skips_left != 0,
            }
        }

        pub(crate) fn optional_skip(&self) -> Self {
            match self {
                Self::Tail => Self::Tail,
                Self::Skip(skips_left) => Self::Skip(skips_left.saturating_sub(1)),
            }
        }

        pub(crate) fn allow_early_exits(&self) -> bool {
            !matches!(self, Self::Tail)
        }
    }

    impl FileHierarchy {
        pub fn get_entries(
            &self,
            buffer: &mut GallideEntryVec,
            start: &SafeNode,
            query: &str,
            mut policy: EntryCreationPolicy,
            max_depth: usize,
        ) {
            buffer.clear();
            let mut queue: VecDeque<(SafeNode, bool)> =
                VecDeque::with_capacity(PAGE_SIZE * max_depth);
            let start_path = start.path(); // cloning is better here
            queue.push_back((start.clone(), false));
            let mut depth: usize = 1;
            'main_loop: while (!queue.is_empty() && depth <= max_depth) {
                let size = queue.len();
                for _ in 0..size {
                    let (front_node, mut parent_matches) = queue.pop_front().unwrap();
                    let reader = front_node.read();
                    parent_matches = parent_matches || front_node.filename_starts_with(query);
                    for child in reader.children.iter().flatten() {
                        let child_reader = child.read();
                        let entry = GallideEntry {
                            node: child.clone(),
                            entry_type: EntryType::from_is_dir(child_reader.is_dir()),
                            display_name: child
                                .path()
                                .strip_prefix(start_path)
                                .unwrap()
                                .to_string_lossy()
                                .to_string(),
                        };
                        let matches_query: bool =
                            parent_matches || child.filename_starts_with(query);
                        if matches_query && !policy.should_skip() {
                            buffer.push_back(entry);
                        }
                        policy.optional_skip();
                        if policy.allow_early_exits() && buffer.full() {
                            break 'main_loop;
                        }
                        if child_reader.is_dir() {
                            queue.push_back((child.clone(), matches_query));
                        }
                    }
                }
                depth += 1;
            }
        }

        pub fn new() -> Result<Self> {
            let (tx, rx): (Sender<SafeNode>, Receiver<SafeNode>) = channel();
            let root = TreeNode::new(
                PathBuf::from(".").canonicalize().unwrap(),
                None,
                Some(Vec::with_capacity(20)),
            );
            let hierarchy = FileHierarchy {
                prev_root: root.clone(),
                root,
                sender: tx,
            };
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
                            if Node::is_same(&origin, &prev_root)
                                && !Node::is_same(&root, &prev_root)
                            {
                                continue;
                            }
                            let Ok(directory_contents) = fs::read_dir(origin.path()) else {
                                continue;
                            };
                            for entry in directory_contents {
                                let Ok(entry) = entry else { continue };
                                let path: PathBuf = entry.path();
                                let Ok(filetype) = entry.file_type() else {
                                    continue;
                                };
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
                                let child_node = TreeNode::new(
                                    path,
                                    Some(Arc::downgrade(&origin)),
                                    node_children,
                                );
                                origin
                                    .write()
                                    .children
                                    .as_mut()
                                    .unwrap()
                                    .push(child_node.clone());
                                if metadata.is_dir() {
                                    queue.push_back(child_node.clone());
                                }
                            }
                            origin
                                .write()
                                .children
                                .as_mut()
                                .unwrap()
                                .sort_unstable_by(TreeNode::order);
                            origin.update_parents();
                            size -= 1;
                        }
                    }
                }
            });

            hierarchy.sender.send(hierarchy.root.clone());
            Ok(hierarchy)
        }

        pub fn step_back(&mut self) -> SafeNode {
            let mut path = self.root.path().to_path_buf();
            if !path.pop() {
                return self.root.clone();
            }
            self.prev_root = self.root.clone();
            self.root = TreeNode::new(path, None, Some(vec![self.prev_root.clone()]));
            self.prev_root.write().parent = Some(Arc::downgrade(&self.root));
            self.sender.send(self.root.clone());
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

        use crate::explore::{
            GallideEntryVec,
            tree::hierarchy::{
                EntryCreationPolicy::{Skip, Tail},
                FileHierarchy,
            },
        };
        #[test]
        fn make() {
            let mut file_hierarchy = FileHierarchy::new().unwrap();
            sleep(Duration::from_millis(1000));
            println!("Size {}", file_hierarchy.root.read().nested_entries());
            let mut buffer = GallideEntryVec::new();
            file_hierarchy.get_entries(&mut buffer, &file_hierarchy.root, "", Tail, 1000);
            for (idx, entry) in buffer.into_iter().enumerate() {
                println!("{} ({}) {}", entry.entry_type, idx + 1, entry.name())
            }
            println!("Size {}", file_hierarchy.root.read().nested_entries());
        }
    }
}
