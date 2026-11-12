use std::path::PathBuf;
use std::sync::Arc;

use http::uri::Authority;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::replica::Replica;
use crate::tls::Tls;

pub(crate) struct Node {
    replica: watch::Receiver<Option<Replica>>,
    current: tokio::sync::Mutex<watch::Sender<Option<Replica>>>,
    bookmarks_dir: PathBuf,
    tls: Tls,
    peers: Vec<Authority>,
    stop: CancellationToken,
    tasks: TaskTracker,
}

impl Node {
    pub(crate) fn new(tls: Tls, bookmarks_dir: PathBuf, peers: Vec<Authority>) -> Arc<Self> {
        let (current, replica) = watch::channel(None);
        Arc::new(Self {
            replica,
            current: tokio::sync::Mutex::new(current),
            bookmarks_dir,
            tls,
            peers,
            stop: CancellationToken::new(),
            tasks: TaskTracker::new(),
        })
    }
}

mod read;
mod claim_bookmark;
mod seed;
mod join;
mod update;
mod gossip_over;
mod dial;
mod serve;
mod start;
mod retire;
mod shutdown;
