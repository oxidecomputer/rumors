//! Atomic file storage for [`Bookmark`].

use std::fs::TryLockError;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use redb::{Builder, Database, DatabaseError, ReadableDatabase, TableDefinition, TableError};
use tokio::runtime::Handle;
use tokio::sync::Mutex as AsyncMutex;

use super::Bookmark;

/// Runs a [`FileBookmark`]'s synchronous filesystem jobs where blocking is
/// permitted.
///
/// [`TokioBlocking`] runs them on Tokio's blocking pool. For another executor,
/// implement this trait on a type of your own; this runner gives each job a
/// thread of its own:
///
/// ```
/// use std::io;
///
/// use futures::channel::oneshot;
/// use rumors::RunBlocking;
///
/// struct OwnThread;
///
/// impl RunBlocking for OwnThread {
///     async fn run_blocking(&self, job: Box<dyn FnOnce() + Send>) -> io::Result<()> {
///         let (finished, done) = oneshot::channel();
///         std::thread::spawn(move || {
///             job();
///             let _ = finished.send(());
///         });
///         done.await.map_err(io::Error::other)
///     }
/// }
/// ```
pub trait RunBlocking: Send + Sync {
    /// Run `job` where blocking is permitted, and resolve only after it
    /// finishes.
    ///
    /// Calling this method must not block. Its result reports whether the
    /// runner completed the job; the bookmark method that dispatched the job
    /// reports filesystem errors separately.
    fn run_blocking(
        &self,
        job: Box<dyn FnOnce() + Send + 'static>,
    ) -> impl Future<Output = io::Result<()>> + Send;
}

/// Runs each job on the blocking pool of the Tokio runtime that dispatches it.
///
/// # Errors
///
/// A job dispatched outside a Tokio runtime fails with an I/O error, as does a
/// job whose blocking task panics or is cancelled by runtime shutdown.
#[derive(Clone, Copy, Debug, Default)]
pub struct TokioBlocking;

/// Dispatch onto the current Tokio runtime's blocking pool.
impl RunBlocking for TokioBlocking {
    async fn run_blocking(&self, job: Box<dyn FnOnce() + Send + 'static>) -> io::Result<()> {
        let runtime = Handle::try_current().map_err(io::Error::other)?;
        runtime.spawn_blocking(job).await.map_err(io::Error::other)
    }
}

/// A bookmark stored in one file, as a [redb](https://docs.rs/redb) database.
///
/// [`new`](Self::new) takes a [`RunBlocking`] runner that moves each
/// synchronous filesystem job onto the application's blocking executor;
/// [`TokioBlocking`] is the runner for Tokio applications. Each store commits
/// the new record in one durable, two-phase transaction, so a crash leaves the
/// file holding either the previous record or the new one. Opening verifies the
/// checksum of every stored page, so a damaged file fails to open with an error
/// instead of being trusted. The file starts at about 1 MiB, which redb
/// allocates up front, and grows in power-of-two steps for a record too large
/// to fit.
///
/// A `FileBookmark` holds an exclusive operating-system lock on its file for as
/// long as it lives (`flock` on Unix, `LockFileEx` on Windows), so no two live
/// `FileBookmark`s on one machine open the same bookmark, whatever name each
/// opens it by: a symbolic link or a hard link names the same file, and so the
/// same lock. Sharing a bookmark lets the peers reuse versions, which corrupts
/// the set. The lock binds only processes that open the bookmark through
/// `FileBookmark`, and no lock recognizes a copy: copying a bookmark duplicates
/// it, which the [storage obligations](Bookmark#storage-obligations) forbid.
/// Dropping the bookmark releases the lock. To give several peers on one
/// machine bookmarks of their own, try paths in turn until `new` succeeds:
///
/// ```
/// # #[cfg(feature = "fs")]
/// # async fn claim(directory: &std::path::Path) -> std::io::Result<()> {
/// use std::fs::TryLockError;
///
/// use rumors::{FileBookmark, TokioBlocking};
///
/// let mut slot = 0;
/// let bookmark = loop {
///     let path = directory.join(format!("{slot}.bookmark"));
///     match FileBookmark::new(path, TokioBlocking).await {
///         Ok(bookmark) => break bookmark,
///         Err(TryLockError::WouldBlock) => slot += 1,
///         Err(TryLockError::Error(error)) => return Err(error),
///     }
/// };
/// # let _ = bookmark;
/// # Ok(())
/// # }
/// ```
///
/// Jobs run one at a time, in the order they were dispatched, even when the
/// future awaiting a job is cancelled while the job still runs. An operation
/// waits for its turn inside its future, so a future leaked (with
/// `mem::forget`, say) while it waits keeps its turn forever, and every later
/// operation on this bookmark waits behind it. Reclaiming the peer does not
/// release that wait; see
/// [`Rumors::try_into_peer`](crate::Rumors::try_into_peer).
pub struct FileBookmark<R> {
    /// The path the bookmark was opened at, for diagnostics.
    path: PathBuf,
    /// The database holding the record; it holds its file's lock until dropped.
    database: Arc<Database>,
    /// The application's bridge to a context where blocking is permitted.
    runner: R,
    /// Keep dispatched work ordered even if its awaiting task is cancelled.
    serial: Arc<AsyncMutex<()>>,
}

impl<R: RunBlocking> FileBookmark<R> {
    /// Open the bookmark at `path`, creating it if absent, and lock its file,
    /// running filesystem jobs through `runner`.
    ///
    /// The parent directory must already exist. Pass [`TokioBlocking`] from a
    /// Tokio application, or a [`RunBlocking`] of your own for another
    /// executor. Opening never waits for the lock: a bookmark another live
    /// `FileBookmark` holds fails at once.
    ///
    /// # Errors
    ///
    /// - [`TryLockError::WouldBlock`]: another live `FileBookmark`, in this
    ///   process or another, holds the file `path` names.
    /// - [`TryLockError::Error`]: the file could not be opened or created (its
    ///   parent directory is missing, say), holds something other than a
    ///   bookmark, or fails its integrity check; or the runner failed to run
    ///   the job.
    pub async fn new(path: impl Into<PathBuf>, runner: R) -> Result<Self, TryLockError> {
        let path = path.into();
        let opening = path.clone();
        let database = dispatch(&runner, move || open(&opening))
            .await
            .map_err(TryLockError::Error)??;
        Ok(Self {
            path,
            database: Arc::new(database),
            runner,
            serial: Arc::new(AsyncMutex::new(())),
        })
    }

    /// Run one filesystem operation without letting cancellation reorder it.
    async fn run<T, G>(&self, operation: G) -> io::Result<T>
    where
        T: Send + 'static,
        G: FnOnce() -> io::Result<T> + Send + 'static,
    {
        let serial = self.serial.clone().lock_owned().await;
        dispatch(&self.runner, move || {
            let result = operation();
            drop(serial);
            result
        })
        .await?
    }
}

/// Run `operation` through `runner` and return what it returned.
///
/// The outer error reports the runner's own failure; the operation's result,
/// errors included, is the inner value.
async fn dispatch<R, T, G>(runner: &R, operation: G) -> io::Result<T>
where
    R: RunBlocking,
    T: Send + 'static,
    G: FnOnce() -> T + Send + 'static,
{
    let result = Arc::new(Mutex::new(None));
    let returned = result.clone();
    let job = Box::new(move || {
        let value = operation();
        if let Ok(mut returned) = returned.lock() {
            *returned = Some(value);
        }
    });
    runner.run_blocking(job).await?;
    result
        .lock()
        .map_err(poisoned)?
        .take()
        .ok_or_else(|| io::Error::other("blocking runner did not complete the bookmark job"))
}

/// The table holding the record, under [`RECORD_KEY`].
const RECORD: TableDefinition<u8, &[u8]> = TableDefinition::new("bookmark");

/// The record's key: the table holds nothing else.
const RECORD_KEY: u8 = 0;

/// Open or create the database at `path`, verifying every stored page, and take
/// its file's lock without waiting.
fn open(path: &Path) -> Result<Database, TryLockError> {
    match Builder::new()
        .set_check_integrity_on_open(true)
        .create(path)
    {
        Ok(database) => Ok(database),
        Err(DatabaseError::DatabaseAlreadyOpen) => Err(TryLockError::WouldBlock),
        Err(error) => Err(TryLockError::Error(io::Error::other(error))),
    }
}

/// Show the path without requiring the blocking runner to be debuggable.
impl<R> std::fmt::Debug for FileBookmark<R> {
    /// Format the path and hide the database, the runner, and serialization
    /// state.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileBookmark")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

/// Read and atomically replace the record in the database.
impl<R: RunBlocking> Bookmark for FileBookmark<R> {
    /// Filesystem, database, or blocking-executor failure.
    type Error = io::Error;
    /// A complete in-memory copy of the stored record.
    type Reader = std::io::Cursor<Vec<u8>>;

    /// Read the record, treating a database without one as an empty bookmark.
    async fn load(&self) -> Result<Option<Self::Reader>, Self::Error> {
        let database = self.database.clone();
        self.run(move || load(&database))
            .await
            .map(|bytes| bytes.map(std::io::Cursor::new))
    }

    /// Replace the record in one durable, two-phase transaction.
    async fn store(&self, bytes: Vec<u8>) -> Result<(), Self::Error> {
        let database = self.database.clone();
        self.run(move || store(&database, &bytes)).await
    }
}

/// Read the stored record, if one was ever stored.
fn load(database: &Database) -> io::Result<Option<Vec<u8>>> {
    let read = database.begin_read().map_err(io::Error::other)?;
    let table = match read.open_table(RECORD) {
        Ok(table) => table,
        Err(TableError::TableDoesNotExist(_)) => return Ok(None),
        Err(error) => return Err(io::Error::other(error)),
    };
    let record = table.get(RECORD_KEY).map_err(io::Error::other)?;
    Ok(record.map(|record| record.value().to_vec()))
}

/// Replace the record and make it durable before returning.
///
/// Two-phase commit costs a second sync, and in exchange recovery after a crash
/// never relies on a checksum to tell a torn commit from a whole one.
fn store(database: &Database, bytes: &[u8]) -> io::Result<()> {
    let mut write = database.begin_write().map_err(io::Error::other)?;
    write.set_two_phase_commit(true);
    {
        let mut table = write.open_table(RECORD).map_err(io::Error::other)?;
        table.insert(RECORD_KEY, bytes).map_err(io::Error::other)?;
    }
    write.commit().map_err(io::Error::other)
}

/// Convert an internal poisoned result slot into an I/O error.
fn poisoned<T>(_error: std::sync::PoisonError<T>) -> io::Error {
    io::Error::other("bookmark result slot is poisoned")
}

#[cfg(test)]
mod tests;
