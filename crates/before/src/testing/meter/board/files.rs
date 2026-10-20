//! Writes the board's output files, captures and recalibrated tables, so that
//! no reader ever sees a partial one.

use std::io::{self, Write};
use std::path::Path;

use tempfile::NamedTempFile;

/// Replaces the file at `path` with `bytes` in one rename, or leaves it
/// untouched.
///
/// The bytes go first to a temporary file in `path`'s directory, so the
/// rename stays on one filesystem, and reach the disk before the rename, so a
/// crash cannot leave an empty file in `path`'s place. The temporary file
/// deletes itself if the write or the rename fails.
///
/// # Errors
///
/// Returns any error creating, writing, syncing, or renaming the temporary
/// file.
pub(super) fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut file = NamedTempFile::new_in(dir)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}
