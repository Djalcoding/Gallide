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
        mem::offset_of,
        os::unix::{ffi::OsStrExt, process, thread},
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
        widgets::canvas::Context,
    };

    use crate::{
        config::{BehaviorConfig, MainBoxConfig}, explore::{
            EntryType, GallideEntry, GallideEntryVec,
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
            whitelist_queries: &[&str],
            highlight_index: &mut Option<usize>,
            current_query: &mut usize,
            case_sensitive: bool,
        ) {
            if *current_query == whitelist_queries.len() {
                return;
            }
            let offset: usize = !(start_path == "/") as usize;
            if self.filename_starts_with(whitelist_queries[*current_query], case_sensitive) {
                (*current_query) += 1;
                if highlight_index.is_none() {
                    (*highlight_index) = Some(
                        self.filename_index()
                            .saturating_sub(start_path.len() + offset),
                    )
                }
            } else {
                (*current_query) = 0;
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
        fn filename_starts_with(&self, query: &str, case_sensitive: bool) -> bool {
            let file_name = self.path().file_name();
            let Some(file_name) = file_name else {
                return false;
            };
            if file_name.len() < query.len() {
                return false;
            }
            let utf8_string = file_name.to_string_lossy();
            let mut file_name_chars = utf8_string.chars();
            query.chars().all(|q_c| {
                file_name_chars.next().is_some_and(|f_c| {
                    if case_sensitive {
                        q_c == f_c
                    } else {
                        q_c.to_lowercase().eq(f_c.to_lowercase())
                    }
                })
            })
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
    #[derive(Clone, Copy)]
    pub struct EntryCreationContext<'a> {
        pub max_depth: usize,
        pub skips: usize,
        pub whitelist: &'a str,
        pub explore_fully: bool,
        pub config: &'a BehaviorConfig,
    }

    impl EntryCreationContext<'_> {
        pub(crate) fn get_ideal_queue_length(&self) -> usize {
            self.config.page_size * self.max_depth * 100
        }
    }

    impl FileHierarchy {
        /// Returns spincount and remaing entries
        /// Returns a negative integer if the returned page is the last
        /// Currently the most expensive operation is searching the full tree to go up
        pub fn get_entries(
            &self,
            buffer: &mut GallideEntryVec,
            start: &SafeNode,
            monitored_node: Option<SafeNode>,
            mut context: EntryCreationContext,
            style_config: &MainBoxConfig,
        ) -> (usize, isize) {
            let length = 0;
            buffer.clear();
            let queries: Vec<&str> = context.whitelist.split('/').collect::<Vec<&str>>();
            let start_path = start.path().as_os_str();
            let mut queue: VecDeque<(SafeNode, usize, Option<usize>)> =
                VecDeque::with_capacity(context.get_ideal_queue_length());
            let mut is_end = true;
            let mut current_depth: usize = 1;
            let mut spin = 0usize;
            if context.skips == 0 {
                buffer.push(GallideEntry::previous_directory(start));
            }
            let mut processed = 1usize; // '..'
            queue.push_back((start.clone(), 0, None));
            'main_loop: while (!queue.is_empty() && current_depth <= context.max_depth) {
                let size = queue.len();
                for _ in 0..size {
                    let (parent_node, mut current_query, mut parent_highlight_index) =
                        queue.pop_front().unwrap();
                    let parent_reader = parent_node.read();
                    for child_node in parent_reader.children.iter().flatten() {
                        let mut child_current_query = current_query;
                        let mut child_highlight_index: Option<usize> = parent_highlight_index;
                        child_node.satisfies_whitelist(
                            start_path,
                            &queries,
                            &mut child_highlight_index,
                            &mut child_current_query,
                            context.config.case_sensitive,
                        );
                        let child_reader = child_node.read();
                        if child_current_query == queries.len()
                            && let Some(highlight_index) = child_highlight_index
                        {
                            if processed != 0 && processed.is_multiple_of(context.config.page_size)
                            {
                                spin += 1;
                            }
                            processed += 1;
                            if context.skips == 0 {
                                buffer.push(GallideEntry::from_node(
                                    highlight_index,
                                    context.whitelist.len(),
                                    child_node,
                                    start_path,
                                    style_config,
                                    
                                ));
                                if buffer.len() == context.config.page_size  {
                                    if context.explore_fully {
                                        buffer.clear();
                                    } else {
                                        is_end = queue.is_empty();
                                        break 'main_loop;
                                    }
                                }
                            } else {
                                context.skips = context.skips.saturating_sub(1);
                            }
                        }
                        if child_reader.is_dir() {
                            queue.push_back((
                                child_node.clone(),
                                child_current_query,
                                child_highlight_index,
                            ));
                        }
                    }
                }
                current_depth += 1;
            }
            (
                spin,
                buffer.len() as isize
                    * match is_end {
                        true => -1,
                        false => 1,
                    },
            )
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
            let thread_count = config.thread_count;
            let (tx, rx): (Sender<BFSQueueEntry>, Receiver<BFSQueueEntry>) = channel();
            let bored_queue_mask: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
            let mut workers: Vec<BFSThread> = Vec::with_capacity(thread_count- 1);
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
                tree::hierarchy::{EntryCreationContext, FileHierarchy},
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
