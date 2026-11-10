//! Atomic file storage for [`Bookmark`].

use std::fs::{self, File};
use std::future::Future;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tempfile::NamedTempFile;
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

/// A bookmark stored as one atomically replaced file.
///
/// [`new`](Self::new) takes a [`RunBlocking`] runner that moves each
/// synchronous filesystem job onto the application's blocking executor;
/// [`TokioBlocking`] is the runner for Tokio applications. Stores write and sync a
/// temporary file beside the destination, rename it over the previous record,
/// and sync the parent directory before reporting success.
///
/// Jobs run one at a time, in the order they were dispatched, even when the
/// future awaiting a job is cancelled while the job still runs. An operation
/// waits for its turn inside its future, so a future leaked (with
/// `mem::forget`, say) while it waits keeps its turn forever, and every later
/// operation on this bookmark waits behind it. Reclaiming the peer does not
/// release that wait; see
/// [`Rumors::try_into_peer`](crate::Rumors::try_into_peer).
pub struct FileBookmark<R> {
    /// The file replaced by each successful store.
    path: PathBuf,
    /// The application's bridge to a context where blocking is permitted.
    runner: R,
    /// Keep dispatched work ordered even if its awaiting task is cancelled.
    serial: Arc<AsyncMutex<()>>,
}

impl<R: RunBlocking> FileBookmark<R> {
    /// Store a bookmark at `path`, running filesystem jobs through `runner`.
    ///
    /// The parent directory must already exist. Pass [`TokioBlocking`] from a
    /// Tokio application, or a [`RunBlocking`] of your own for another
    /// executor.
    pub fn new(path: impl Into<PathBuf>, runner: R) -> Self {
        Self {
            path: path.into(),
            runner,
            serial: Arc::new(AsyncMutex::new(())),
        }
    }

    /// Run one filesystem operation without letting cancellation reorder it.
    async fn run<T, G>(&self, operation: G) -> io::Result<T>
    where
        T: Send + 'static,
        G: FnOnce() -> io::Result<T> + Send + 'static,
    {
        let serial = self.serial.clone().lock_owned().await;
        let result = Arc::new(Mutex::new(None));
        let returned = result.clone();
        let job = Box::new(move || {
            if let Ok(mut returned) = returned.lock() {
                *returned = Some(operation());
            }
            drop(serial);
        });
        self.runner.run_blocking(job).await?;
        result
            .lock()
            .map_err(poisoned)?
            .take()
            .ok_or_else(|| io::Error::other("blocking runner did not complete the bookmark job"))?
    }
}

/// Show the destination without requiring the blocking runner to be debuggable.
impl<R> std::fmt::Debug for FileBookmark<R> {
    /// Format the path and hide the runner and serialization state.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileBookmark")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

/// Read and atomically replace the configured file.
impl<R: RunBlocking> Bookmark for FileBookmark<R> {
    /// Filesystem or blocking-executor failure.
    type Error = io::Error;
    /// A complete in-memory snapshot of the stored file.
    type Reader = std::io::Cursor<Vec<u8>>;

    /// Read the complete file, treating an absent path as an empty bookmark.
    async fn load(&self) -> Result<Option<Self::Reader>, Self::Error> {
        let path = self.path.clone();
        self.run(move || load(&path))
            .await
            .map(|bytes| bytes.map(std::io::Cursor::new))
    }

    /// Replace the file atomically and sync its directory entry.
    async fn store(&self, bytes: Vec<u8>) -> Result<(), Self::Error> {
        let path = self.path.clone();
        self.run(move || store(&path, &bytes)).await
    }
}

/// Read one complete stored record.
fn load(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Replace one record and make its new directory entry durable.
fn store(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = parent(path);
    let mut temporary = NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    sync_directory(parent)
}

/// Return the destination's parent, including `.` for a bare file name.
fn parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

/// Convert an internal poisoned result slot into an I/O error.
fn poisoned<T>(_error: std::sync::PoisonError<T>) -> io::Error {
    io::Error::other("bookmark result slot is poisoned")
}

/// Open and sync a directory after its entry has been replaced.
#[cfg(not(windows))]
fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

/// Open and sync a directory after its entry has been replaced.
#[cfg(windows)]
fn sync_directory(path: &Path) -> io::Result<()> {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;

    /// Permit opening a directory through Windows' file API.
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;

    OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?
        .sync_all()
}

#[cfg(test)]
mod tests;
