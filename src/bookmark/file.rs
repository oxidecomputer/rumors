//! Atomic file storage for [`Bookmark`].

use std::fs::{self, File};
use std::future::Future;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tempfile::NamedTempFile;
use tokio::sync::Mutex as AsyncMutex;

use super::Bookmark;

/// A bookmark stored as one atomically replaced file.
///
/// [`new`](Self::new) takes a callback that moves each synchronous filesystem
/// job onto the application's blocking executor. Stores write and sync a
/// temporary file beside the destination, rename it over the previous record,
/// and sync the parent directory before reporting success.
pub struct FileBookmark<F> {
    /// The file replaced by each successful store.
    path: PathBuf,
    /// The application's bridge to a context where blocking is permitted.
    run_blocking: F,
    /// Keep dispatched work ordered even if its awaiting task is cancelled.
    serial: Arc<AsyncMutex<()>>,
}

impl<F> FileBookmark<F> {
    /// Store a bookmark at `path`, running filesystem jobs through
    /// `run_blocking`.
    ///
    /// The parent directory must already exist. The callback must execute its
    /// supplied job where blocking is permitted and resolve only after the job
    /// finishes. Calling the callback itself must not block. Its result reports
    /// whether the runner completed the job; filesystem errors are returned
    /// separately by the bookmark method. The callback may use a runtime's
    /// blocking executor, a dedicated worker, or another equivalent facility.
    pub fn new<Fut>(path: impl Into<PathBuf>, run_blocking: F) -> Self
    where
        F: Fn(Box<dyn FnOnce() + Send + 'static>) -> Fut,
        Fut: Future<Output = io::Result<()>> + Send,
    {
        Self {
            path: path.into(),
            run_blocking,
            serial: Arc::new(AsyncMutex::new(())),
        }
    }

    /// Run one filesystem operation without letting cancellation reorder it.
    async fn run<R, G, Fut>(&self, operation: G) -> io::Result<R>
    where
        R: Send + 'static,
        G: FnOnce() -> io::Result<R> + Send + 'static,
        F: Fn(Box<dyn FnOnce() + Send + 'static>) -> Fut,
        Fut: Future<Output = io::Result<()>> + Send,
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
        (self.run_blocking)(job).await?;
        result
            .lock()
            .map_err(poisoned)?
            .take()
            .ok_or_else(|| io::Error::other("blocking runner did not complete the bookmark job"))?
    }
}

/// Show the destination without requiring the blocking runner to be debuggable.
impl<F> std::fmt::Debug for FileBookmark<F> {
    /// Format the path and hide the runner and serialization state.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileBookmark")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

/// Read and atomically replace the configured file.
impl<F, Fut> Bookmark for FileBookmark<F>
where
    F: Fn(Box<dyn FnOnce() + Send + 'static>) -> Fut + Send + Sync,
    Fut: Future<Output = io::Result<()>> + Send,
{
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
