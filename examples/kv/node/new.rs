use std::path::PathBuf;

use tokio::sync::watch;
use tokio_util::task::TaskTracker;

struct Node {
    replica: watch::Receiver<Option<Replica>>,
    current: tokio::sync::Mutex<watch::Sender<Option<Replica>>>,
    bookmarks_dir: PathBuf,
    tls: Tls,
    peers: Vec<Authority>,
    stop: CancellationToken,
    tasks: TaskTracker,
}

impl Node {
    fn new(tls: Tls, bookmarks_dir: PathBuf, peers: Vec<Authority>) -> Arc<Self> {
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
