use std::fs::{self, TryLockError};

use anyhow::Result;
use rumors::{FileBookmark, TokioBlocking};

use super::Node;

impl Node {
    pub(super) async fn claim_bookmark(&self) -> Result<FileBookmark<TokioBlocking>> {
        fs::create_dir_all(&self.bookmarks_dir)?;
        let mut slot = 0;
        loop {
            let path = self.bookmarks_dir.join(format!("{slot}.bookmark"));
            match FileBookmark::new(path, TokioBlocking).await {
                Ok(bookmark) => return Ok(bookmark),
                Err(TryLockError::WouldBlock) => slot += 1,
                Err(TryLockError::Error(error)) => return Err(error.into()),
            }
        }
    }
}
