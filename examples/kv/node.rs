use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::replica::Replica;

pub struct Node {
    replica: watch::Receiver<Option<Replica>>,
    pub current: tokio::sync::Mutex<watch::Sender<Option<Replica>>>,
    bookmarks_dir: PathBuf,
    pub stop: CancellationToken,
    pub tasks: TaskTracker,
}

impl Node {
    pub fn new(bookmarks_dir: impl Into<PathBuf>) -> Arc<Self> {
        let (current, replica) = watch::channel(None);
        Arc::new(Self {
            replica,
            current: tokio::sync::Mutex::new(current),
            bookmarks_dir: bookmarks_dir.into(),
            stop: CancellationToken::new(),
            tasks: TaskTracker::new(),
        })
    }
}

mod read;
mod claim_bookmark;
mod seed;
mod join;
mod wait_for_replica;
mod update;
mod gossip_over;
