use std::{
    collections::VecDeque,
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{
            AtomicU64,
            Ordering::{self, Release},
        },
        mpsc::{Receiver, Sender, channel},
    },
};

use crate::explore::tree::hierarchy::{SafeNode, TreeNode};

pub(crate) type BFSQueueEntry = (SafeNode, Option<SafeNode>);

pub struct BFSThread {
    transmitter: Sender<BFSQueueEntry>, // sends messages to the thread
}

impl BFSThread {
    pub fn spin(bored_queues: Arc<AtomicU64>, id: u8, dispatcher: Sender<BFSQueueEntry>) -> Self {
        let (tx, rx) = channel::<BFSQueueEntry>();
        let _ = std::thread::spawn(move || {
            Self::bfs(dispatcher, rx, bored_queues, id);
        });
        Self { transmitter: tx }
    }

    fn bfs(
        dispatcher: Sender<BFSQueueEntry>,
        receiver: Receiver<BFSQueueEntry>,
        bored_queues: Arc<AtomicU64>,
        id: u8,
    ) {
        let mut queue: VecDeque<BFSQueueEntry> = VecDeque::new();
        let mask = 1 << id;
        queue.reserve(1000);
        loop {
            bored_queues.fetch_or(mask, Release);
            let origin_node = receiver.recv();
            if origin_node.is_err() {
                return;
            }
            queue.push_back(origin_node.unwrap());
            while !queue.is_empty() {
                let mut size = queue.len();
                while size > 0 {
                    size -= 1;
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
                        let node_children = if filetype.is_dir() {
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
                        if filetype.is_dir() {
                            if bored_queues.load(Ordering::Relaxed) > 0 {
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

    pub fn dispatch(&self, task: BFSQueueEntry) {
        let _ = self.transmitter.send(task);
    }
}
