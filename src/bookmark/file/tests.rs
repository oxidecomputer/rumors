use std::io;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::oneshot;

use super::FileBookmark;
use crate::Bookmark;

/// A boxed filesystem operation accepted by the test runners.
type Job = Box<dyn FnOnce() + Send + 'static>;

/// Run one file job on Tokio's blocking pool for tests.
async fn run_blocking(job: Job) -> io::Result<()> {
    tokio::task::spawn_blocking(job)
        .await
        .map_err(io::Error::other)
}

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
                run_blocking,
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
    let (queued_tx, mut queued_rx) = tokio::sync::mpsc::unbounded_channel();
    let bookmark = Arc::new(FileBookmark::new(path.clone(), move |job| {
        let queued_tx = queued_tx.clone();
        async move {
            let (done, complete) = oneshot::channel();
            queued_tx.send(QueuedJob { job, done }).unwrap();
            complete
                .await
                .map_err(|_| io::Error::other("test runner dropped the job"))?
        }
    }));

    let first = tokio::spawn({
        let bookmark = bookmark.clone();
        async move { bookmark.store(vec![1]).await }
    });
    let first_job = queued_rx.recv().await.unwrap();
    first.abort();
    first.await.unwrap_err();

    let second = tokio::spawn({
        let bookmark = bookmark.clone();
        async move { bookmark.store(vec![2]).await }
    });
    tokio::task::yield_now().await;
    assert!(matches!(
        queued_rx.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    ));

    finish(first_job).await;
    finish(queued_rx.recv().await.unwrap()).await;
    second.await.unwrap().unwrap();

    let bytes = tokio::task::spawn_blocking(move || std::fs::read(path))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bytes, [2]);
}
