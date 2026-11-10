use std::fs::TryLockError;
use std::io;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use futures::channel::mpsc;
use proptest::prelude::*;
use tokio::sync::oneshot;

use super::{FileBookmark, RunBlocking, TokioBlocking};
use crate::testing::property::{LOCAL_DIVISOR, divided_proptest};
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
            .await
            .unwrap()
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
    let opening = tokio::spawn(FileBookmark::new(path.clone(), Queue(queued_tx)));
    finish(queued_rx.next().await.unwrap()).await;
    let bookmark = Arc::new(opening.await.unwrap().unwrap());

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

    drop(bookmark);
    let reopened = FileBookmark::new(path, TokioBlocking).await.unwrap();
    let stored = reopened.load().await.unwrap().map(io::Cursor::into_inner);
    assert_eq!(stored, Some(vec![2]));
}

/// A session of a peer bookmarked in a file runs on a task spawned onto a
/// multi-threaded runtime, and reports a transport failure when its
/// counterparty has already hung up.
#[tokio::test(flavor = "multi_thread")]
async fn bookmarked_sessions_run_on_spawned_tasks() {
    let directory = tempfile::tempdir().unwrap();
    let bookmark = FileBookmark::new(directory.path().join("bookmark"), TokioBlocking)
        .await
        .unwrap();
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

/// A runner that runs each job on the awaiting thread, for tests that drive a
/// bookmark without an async runtime.
struct Inline;

/// Run the job when the returned future is first polled.
impl RunBlocking for Inline {
    async fn run_blocking(&self, job: Job) -> io::Result<()> {
        job();
        Ok(())
    }
}

/// The number of bookmark paths the lock model draws from.
const LOCK_PATHS: usize = 3;

/// One way the lock model names a bookmark path.
#[derive(Clone, Copy, Debug)]
enum Spelling {
    /// The path itself.
    Direct,
    /// A symbolic link to the bookmark file, which need not exist yet.
    #[cfg(unix)]
    LinkedFile,
    /// The bookmark file's name under a symbolic link to its directory.
    #[cfg(unix)]
    LinkedDirectory,
}

/// Every spelling this platform can create.
#[cfg(unix)]
const SPELLINGS: &[Spelling] = &[
    Spelling::Direct,
    Spelling::LinkedFile,
    Spelling::LinkedDirectory,
];

/// Every spelling this platform can create.
#[cfg(not(unix))]
const SPELLINGS: &[Spelling] = &[Spelling::Direct];

/// The path `spelling` names for the bookmark at `index` under `directory`,
/// whose links [`spell_all`] created.
fn spelled(directory: &std::path::Path, index: usize, spelling: Spelling) -> std::path::PathBuf {
    let name = format!("{index}.bookmark");
    match spelling {
        Spelling::Direct => directory.join(name),
        #[cfg(unix)]
        Spelling::LinkedFile => directory.join(format!("link-{name}")),
        #[cfg(unix)]
        Spelling::LinkedDirectory => directory.join("alias").join(name),
    }
}

/// Create the symbolic links every spelling of every model path goes through.
fn spell_all(directory: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        symlink(".", directory.join("alias")).unwrap();
        for index in 0..LOCK_PATHS {
            let name = format!("{index}.bookmark");
            symlink(&name, directory.join(format!("link-{name}"))).unwrap();
        }
    }
    #[cfg(not(unix))]
    let _ = directory;
}

/// One step of the lock model, on the bookmark path at an index below
/// [`LOCK_PATHS`].
#[derive(Clone, Copy, Debug)]
enum LockStep {
    /// Open the bookmark at the path, under one of its spellings, keeping it if
    /// the open succeeds.
    Open(usize, Spelling),
    /// Drop the live bookmark at the path, if there is one.
    Drop(usize),
}

/// Draw one lock-model step over every path and spelling.
fn lock_step() -> impl Strategy<Value = LockStep> {
    prop_oneof![
        (0..LOCK_PATHS, prop::sample::select(SPELLINGS))
            .prop_map(|(index, spelling)| LockStep::Open(index, spelling)),
        (0..LOCK_PATHS).prop_map(LockStep::Drop),
    ]
}

divided_proptest! {
    #![divisor(LOCAL_DIVISOR)]

    /// At most one live `FileBookmark` holds each bookmark file, under any of
    /// its names, and dropping it frees the file.
    ///
    /// Across any sequence of opens and drops over a few paths, each open
    /// naming its path directly or through a symbolic link to the file or its
    /// directory, an open succeeds exactly when no live bookmark holds its
    /// file, and fails with `WouldBlock` when one does.
    #[test]
    fn an_open_succeeds_exactly_when_no_live_bookmark_holds_its_path(
        steps in prop::collection::vec(lock_step(), 1..24),
    ) {
        let directory = tempfile::tempdir().unwrap();
        spell_all(directory.path());
        let mut live: [Option<FileBookmark<Inline>>; LOCK_PATHS] = Default::default();
        for step in steps {
            match step {
                LockStep::Open(index, spelling) => {
                    let path = spelled(directory.path(), index, spelling);
                    let opened = pollster::block_on(FileBookmark::new(path, Inline));
                    match (opened, &live[index]) {
                        (Ok(bookmark), None) => live[index] = Some(bookmark),
                        (Err(TryLockError::WouldBlock), Some(_)) => {}
                        (opened, held) => panic!(
                            "path {index} as {spelling:?}, held by {held:?}, opened as {opened:?}"
                        ),
                    }
                }
                LockStep::Drop(index) => live[index] = None,
            }
        }
    }
}

/// A hard link is another name for the same bookmark.
///
/// While one name holds it, opening the other is refused, even for a link made
/// while held and followed by a store, and once released either name opens it
/// holding the latest record.
#[cfg(unix)]
#[test]
fn a_hard_link_names_the_same_bookmark() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("0.bookmark");
    let other = directory.path().join("other.bookmark");
    let load = |bookmark: &FileBookmark<Inline>| {
        pollster::block_on(bookmark.load())
            .unwrap()
            .map(io::Cursor::into_inner)
    };

    let held = pollster::block_on(FileBookmark::new(&path, Inline)).unwrap();
    pollster::block_on(held.store(vec![1])).unwrap();
    std::fs::hard_link(&path, &other).unwrap();
    pollster::block_on(held.store(vec![2])).unwrap();
    let opened = pollster::block_on(FileBookmark::new(&other, Inline));
    assert!(
        matches!(opened, Err(TryLockError::WouldBlock)),
        "{opened:?}"
    );
    drop(held);

    for name in [&other, &path] {
        let reopened = pollster::block_on(FileBookmark::new(name, Inline)).unwrap();
        assert_eq!(load(&reopened), Some(vec![2]), "through {}", name.display());
    }
}

/// A loop of symbolic links fails to open with an error instead of resolving
/// forever.
#[cfg(unix)]
#[test]
fn a_symbolic_link_loop_is_an_error() {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir().unwrap();
    symlink("b.bookmark", directory.path().join("a.bookmark")).unwrap();
    symlink("a.bookmark", directory.path().join("b.bookmark")).unwrap();
    let opened = pollster::block_on(FileBookmark::new(
        directory.path().join("a.bookmark"),
        Inline,
    ));
    assert!(matches!(opened, Err(TryLockError::Error(_))), "{opened:?}");
}

/// Storing through a symbolic link writes the file it names and leaves the
/// link in place, so the link and the file stay one bookmark.
#[cfg(unix)]
#[test]
fn storing_through_a_symbolic_link_keeps_the_link() {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir().unwrap();
    let link = directory.path().join("link.bookmark");
    let target = directory.path().join("0.bookmark");
    symlink("0.bookmark", &link).unwrap();
    pollster::block_on(async {
        let bookmark = FileBookmark::new(&link, Inline).await.unwrap();
        bookmark.store(vec![7]).await.unwrap();
    });
    let link_type = std::fs::symlink_metadata(&link).unwrap().file_type();
    assert!(link_type.is_symlink());
    let reopened = pollster::block_on(FileBookmark::new(&target, Inline)).unwrap();
    let stored = pollster::block_on(reopened.load()).unwrap();
    assert_eq!(stored.map(io::Cursor::into_inner), Some(vec![7]));
}
