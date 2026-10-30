use std::{
    collections::VecDeque, fs, os::unix::thread, path::PathBuf, sync::{
        Arc,
        atomic::{
            AtomicBool, AtomicU64,
            Ordering::{self, Release},
        },
        mpsc::{Receiver, Sender, channel},
    },
};

use crate::explore::tree::hierarchy::{SafeNode, TreeNode};

pub(crate) type BFSQueueEntry = (SafeNode, Option<SafeNode>);

pub struct BFSThread {
    transmitter: Sender<BFSQueueEntry>, // sends messages to the thread
    bored: Arc<AtomicBool>,
}

impl BFSThread {
    pub fn spin(bored_queues: Arc<AtomicU64>, dispatcher: Sender<BFSQueueEntry>) -> Self {
        let (tx, rx) = channel::<BFSQueueEntry>();
        let bored = Arc::from(AtomicBool::new(false));
        let bored_clone = bored.clone();
        let _ = std::thread::spawn(move || {
            Self::bfs(dispatcher, rx, bored_clone, bored_queues);
        });
        Self {
            transmitter: tx,
            bored,
        }
    }

    fn bfs(
        dispatcher: Sender<BFSQueueEntry>,
        receiver: Receiver<BFSQueueEntry>,
        bored: Arc<AtomicBool>,
        bored_queues: Arc<AtomicU64>,
    ) {
        let mut queue: VecDeque<BFSQueueEntry> = VecDeque::new();
        queue.reserve(200);
        loop {
            bored_queues.fetch_add(1, Release);
            bored.store(true, Release);
            let origin_node = receiver.recv();
            bored.store(false, Release);
            bored_queues.fetch_sub(1, Release);
            if origin_node.is_err() {
                return;
            }
            queue.push_back(origin_node.unwrap());
            while !queue.is_empty() {
                let mut size = queue.len();
                while size > 0 {
                    size-=1;
                    let Some(origin) = queue.pop_back() else {
                        continue;
                    };
                    let (origin, node_to_ignore) = origin;
                    let origin_weak_ptr = Arc::downgrade(&origin);
                    let Ok(directory_contents) = fs::read_dir(origin.path()) else {
                        continue;
                    };
                    let mut found_duplicate = false;
                    for child in directory_contents {
                        let Ok(entry) = child else { continue };
                        let child_path: PathBuf = entry.path();
                        if !found_duplicate
                            && let Some(ref node) = node_to_ignore
                            && node.path() == child_path
                        {
                            found_duplicate = true;
                            continue;
                        }
                        let Ok(filetype) = entry.file_type() else {
                            continue;
                        };
                        if !filetype.is_file() && !filetype.is_dir() {
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
                        let child_node =
                            TreeNode::new(child_path, Some(origin_weak_ptr.clone()), node_children);
                        origin
                            .write()
                            .children
                            .as_mut()
                            .unwrap()
                            .push(child_node.clone());
                        if metadata.is_dir() {
                            if bored_queues.load(Ordering::Acquire) > 0 {
                                dispatcher.send((child_node.clone(), None)).unwrap();
                            } else {
                                queue.push_back((child_node.clone(), None));
                            }
                        }
                    }
                    origin
                        .write()
                        .children
                        .as_mut()
                        .unwrap()
                        .sort_unstable_by(TreeNode::order);
                    origin.update_parents();
                }
            }
        }
    }

    pub fn bored(&self) -> bool {
        self.bored.load(Ordering::Acquire)
    }

    pub fn dispatch(&self, task: BFSQueueEntry) {
        let _ = self.transmitter.send(task);
    }
}
