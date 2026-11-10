use std::io;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use futures::channel::mpsc;
use tokio::sync::oneshot;

use super::{FileBookmark, RunBlocking, TokioBlocking};
use crate::{Bookmark, Error, Peer};

/// A boxed filesystem operation accepted by the test runners.
type Job = Box<dyn FnOnce() + Send + 'static>;

/// A blocking job retained by the cancellation-ordering test.
struct QueuedJob {
    /// The synchronous filesystem operation.
    job: Job,
    /// Completes the runner future with the job's result.
    done: oneshot::Sender<io::Result<()>>,
}

/// Execute one retained job on Tokio's blocking pool and report its result.
async fn finish(queued: QueuedJob) {
    let result = tokio::task::spawn_blocking(queued.job)
        .await
        .map_err(io::Error::other);
    let _ = queued.done.send(result);
}

/// A runner that hands each job to the test, which decides when it runs.
struct Queue(mpsc::UnboundedSender<QueuedJob>);

/// Queue each job and resolve once the test finishes it.
impl RunBlocking for Queue {
    async fn run_blocking(&self, job: Job) -> io::Result<()> {
        let (done, complete) = oneshot::channel();
        self.0.unbounded_send(QueuedJob { job, done }).unwrap();
        complete
            .await
            .map_err(|_| io::Error::other("test runner dropped the job"))?
    }
}

/// The public conformance suite accepts the file backend's replacement and
/// cancellation behavior.
#[tokio::test]
async fn conforms() {
    let directory = tempfile::tempdir().unwrap();
    let next = std::sync::atomic::AtomicUsize::new(0);
    crate::conformance::bookmark::check(
        async || {
            let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            FileBookmark::new(
                directory.path().join(format!("bookmark-{index}")),
                TokioBlocking,
            )
        },
        || tokio::time::sleep(Duration::from_secs(10)),
    )
    .await;
}

/// A cancelled store keeps its place ahead of a later replacement.
#[tokio::test]
async fn cancelled_store_cannot_overwrite_its_successor() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("bookmark");
    let (queued_tx, mut queued_rx) = mpsc::unbounded();
    let bookmark = Arc::new(FileBookmark::new(path.clone(), Queue(queued_tx)));

    let first = tokio::spawn({
        let bookmark = bookmark.clone();
        async move { bookmark.store(vec![1]).await }
    });
    let first_job = queued_rx.next().await.unwrap();
    first.abort();
    first.await.unwrap_err();

    let second = tokio::spawn({
        let bookmark = bookmark.clone();
        async move { bookmark.store(vec![2]).await }
    });
    tokio::task::yield_now().await;
    assert!(matches!(queued_rx.try_recv(), Err(error) if error.is_empty()));

    finish(first_job).await;
    finish(queued_rx.next().await.unwrap()).await;
    second.await.unwrap().unwrap();

    let bytes = tokio::task::spawn_blocking(move || std::fs::read(path))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bytes, [2]);
}

/// A session of a peer bookmarked in a file runs on a task spawned onto a
/// multi-threaded runtime, and reports a transport failure when its
/// counterparty has already hung up.
#[tokio::test(flavor = "multi_thread")]
async fn bookmarked_sessions_run_on_spawned_tasks() {
    let directory = tempfile::tempdir().unwrap();
    let bookmark = FileBookmark::new(directory.path().join("bookmark"), TokioBlocking);
    let Ok(peer) = Peer::<String>::seed().bookmark(bookmark).await else {
        panic!("a pristine seed attaches its bookmark without storage I/O");
    };
    let rumors = peer.into_rumors();
    let (near, far) = tokio::io::duplex(64 * 1024);
    drop(far);
    let (read, write) = tokio::io::split(near);
    let session = tokio::spawn(async move {
        let read = tokio::io::BufReader::new(read);
        rumors.gossip_once(read, write).await.map(drop)
    });
    assert!(matches!(session.await.unwrap(), Err(Error::Transport(_))));
}
