pub mod worker;
#[allow(unused)]
pub mod hierarchy {

    use std::{
        borrow::Cow,
        cmp::Ordering::{Greater, Less},
        collections::{HashMap, HashSet, VecDeque},
        error::Error,
        ffi::OsStr,
        fmt::format,
        fs::{self, DirEntry, Metadata},
        io::Result,
        os::unix::{process, thread},
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

    use crate::{
        config::BehaviorConfig,
        explore::{
            EntryType, GallideEntry, GallideEntryVec, Page,
            tree::{
                hierarchy::{self},
                worker::{BFSQueueEntry, BFSThread},
            },
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
        is_dir: bool,
    }

    impl TreeNode {
        pub fn satisfies_whitelist(
            &self,
            start_path: &OsStr,
            whitelist: &str,
            highlight_index: &mut Option<usize>,
        ) {
            if highlight_index.is_none() && self.filename_starts_with(whitelist) {
                (*highlight_index) =
                    Some(self.filename_index().saturating_sub(start_path.len() + 1))
            }
        }

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
                is_dir: children.is_some(),
                data: RwLock::from(Node::new(parent, children)),
            })
        }

        pub fn path(&self) -> &Path {
            &self.path
        }

        pub fn is_dir(&self) -> bool {
            self.is_dir
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
    pub(super) type ExclusionList = Arc<RwLock<HashSet<PathBuf>>>; // we don't keep a reference here

    pub struct FileHierarchy {
        // TODO private
        pub root: SafeNode,
        pub config: BehaviorConfig,
        exclusion_list: ExclusionList,
        sender: Sender<BFSQueueEntry>,
    }

    pub struct EntryCreationConfig<'a> {
        pub curr_page: &'a Page,
        pub max_depth: usize,
        pub whitelist: &'a str,
    }

    impl EntryCreationConfig<'_> {
        pub(crate) fn get_ideal_queue_length(&self) -> usize {
            self.curr_page.size() * self.max_depth * 100
        }
    }

    impl FileHierarchy {
        pub fn get_entries(
            &self,
            buffer: &mut GallideEntryVec,
            start: &SafeNode,
            monitored_node: Option<SafeNode>,
            config: EntryCreationConfig,
        ) -> Page {
            let mut output = Page {
                start: -1,
                length: 0,
                is_end: true,
            };
            let mut node_position: Option<usize> = None;
            buffer.clear();
            if config.curr_page.start() == 0 {
                output.length += 1;
                buffer.push(GallideEntry::previous_directory(start));
            }

            let start_path = start.path().as_os_str();
            let mut depth: usize = 1;
            let mut skipped = 0usize;

            let mut queue: VecDeque<(SafeNode, Option<usize>)> =
                VecDeque::with_capacity(config.get_ideal_queue_length());
            queue.push_back((start.clone(), None));
            'main_loop: while (!queue.is_empty() && depth <= config.max_depth) {
                let size = queue.len();
                for _ in 0..size {
                    let (parent_node, mut parent_highlight_index) = queue.pop_front().unwrap();
                    let parent_reader = parent_node.read();
                    if parent_node.path() != start_path {
                        parent_node.satisfies_whitelist(
                            start_path,
                            config.whitelist,
                            &mut parent_highlight_index,
                        );
                    }
                    for child_node in parent_reader.children.iter().flatten() {
                        let mut child_highlight_index: Option<usize> = parent_highlight_index;
                        child_node.satisfies_whitelist(
                            start_path,
                            config.whitelist,
                            &mut child_highlight_index,
                        );
                        let child_reader = child_node.read();
                        if let Some(highlight_index) = child_highlight_index {
                            if let Some(ref node) = monitored_node
                                && Node::is_same(node, child_node)
                            {
                                node_position = Some(buffer.len());
                            }
                            buffer.push(GallideEntry::from_node(
                                child_node,
                                start_path,
                                highlight_index,
                                config.whitelist.len(),
                            ));
                            if output.start == -1 {
                                output.start = skipped as isize;
                            }
                            output.length += 1;
                        }
                        if buffer.len() == self.config.page_size {
                            output.is_end = queue.is_empty();
                            break 'main_loop;
                        }
                        if child_reader.is_dir() {
                            queue.push_back((child_node.clone(), child_highlight_index));
                        }
                    }
                }
                depth += 1;
            }
            output
        }

        pub fn new(config: &BehaviorConfig) -> Result<Self> {
            let mut exclusion_list = ExclusionList::from(RwLock::from(HashSet::from_iter(
                config.directory_exclusion_list.iter().cloned(),
            )));
            let root = TreeNode::new(
                PathBuf::from(".").canonicalize().unwrap(),
                None,
                Some(Vec::with_capacity(20)),
            );
            let thread_count: u8 = std::thread::available_parallelism().unwrap().get() as u8;
            let (tx, rx): (Sender<BFSQueueEntry>, Receiver<BFSQueueEntry>) = channel();
            let bored_queue_mask: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
            let mut workers: Vec<BFSThread> = Vec::with_capacity((thread_count - 1) as usize);
            for id in 0..thread_count - 1 {
                workers.push(BFSThread::spin(
                    bored_queue_mask.clone(),
                    id,
                    tx.clone(),
                    exclusion_list.clone(),
                ));
            }
            let hierarchy = FileHierarchy {
                root,
                sender: tx,
                exclusion_list,
                config: config.clone(),
            };
            std::thread::spawn(move || Self::contractor_thread(rx, workers, bored_queue_mask));
            hierarchy.sender.send(hierarchy.root.clone());
            Ok(hierarchy)
        }

        fn contractor_thread(
            incoming_dispatches: Receiver<BFSQueueEntry>,
            workers: Vec<BFSThread>,
            bored_queue_mask: Arc<AtomicU64>,
        ) {
            let mut ten_x_enginner_index = 0;
            loop {
                let task = incoming_dispatches.recv().unwrap();
                let queue_mask = bored_queue_mask.load(Relaxed);
                let mut index: usize = queue_mask.trailing_zeros() as usize;
                if queue_mask == 0 {
                    index = ten_x_enginner_index;
                    ten_x_enginner_index = (ten_x_enginner_index + 1) % workers.len()
                }
                bored_queue_mask.fetch_and(!(1 << index), Ordering::Release);
                workers[index].dispatch(task);
            }
        }

        pub fn step_back(&mut self) -> SafeNode {
            let mut path = self.root.path().to_path_buf();
            if !path.pop() {
                return self.root.clone();
            }
            let prev_root = self.root.clone();
            self.root = TreeNode::new(path, None, Some(vec![prev_root.clone()]));
            prev_root.write().parent = Some(Arc::downgrade(&self.root));
            let mut writer = self.root.write();
            writer.depths[1] = 1;
            for element in prev_root.read().depths.iter().skip(1) {
                writer.depths.push(*element);
            }
            drop(writer);
            self.exclusion_list
                .write()
                .unwrap()
                .insert(prev_root.path().to_owned());
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

        use ratatui::buffer;

        use crate::{
            config::BehaviorConfig,
            explore::{
                GallideEntryVec,
                tree::hierarchy::{EntryCreationConfig, FileHierarchy},
            },
        };
        // #[test]
        // fn make() {
        //     let config = BehaviorConfig::default();
        //     let mut file_hierarchy = FileHierarchy::new(&config).unwrap();
        //     file_hierarchy.step_back();
        //     file_hierarchy.step_back();
        //     file_hierarchy.step_back();
        //     println!("{:?}", file_hierarchy.exclusion_list.read().unwrap())
        // }
    }
}
