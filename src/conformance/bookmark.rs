//! Conformance checks for caller-built [`Bookmark`] storage.
//!
//! [`check`] exercises empty storage, repeatable loads, replacement, and
//! cancellation. Supply a factory of fresh, empty test instances; the checks
//! overwrite their contents with opaque bytes, including empty records.
//!
//! ```
//! # async fn validate<B: rumors::Bookmark>(fresh: impl AsyncFnMut() -> B) {
//! rumors::conformance::bookmark::check(
//!     fresh,
//!     || tokio::time::sleep(std::time::Duration::from_secs(30)),
//! ).await;
//! # }
//! ```
//!
//! Each focused check starts a deadline before constructing its instance.
//! An expired deadline cancels the check and panics with its name. The caller
//! supplies the runtime and clock; a deterministic stall detector can use
//! [`std::future::pending`] for the deadline. A deadline cannot interrupt code
//! that never returns from a poll.
//!
//! Run [`check_failed_store`] separately with a fixture configured to fail its
//! second store, repeating it for the failure points your backend exposes.
//! A failed or cancelled store may leave either complete record. It must never
//! leave a partial record or overwrite a later successful store.
//!
//! # Limits of these checks
//!
//! These are live-process observations at sampled sizes and schedules. They
//! cannot prove durability across process or power loss, exercise every failure
//! point, or rule out a delayed overwrite after the check ends. Review the
//! backend's sync and replacement ordering, and test crashes and unfinished
//! background writes using its own fault controls. Cancellation here occurs
//! after one poll; use a controlled fixture to make that poll reach the stage
//! you intend to test.

use std::pin::pin;
use std::task::Poll;

use tokio::io::AsyncReadExt;

use super::timed;
use crate::Bookmark;

/// Sizes that exercise growth, shrinkage, empty records, and buffer boundaries.
const SIZES: &[usize] = &[4097, 17, 8192, 1, 0, 4096];

/// Generate distinct contents that also exercise every byte value in long records.
fn contents(len: usize, tag: u8) -> Vec<u8> {
    (0..len).map(|i| (i as u8).wrapping_mul(31) ^ tag).collect()
}

/// Run the storage and cancellation checks on fresh, empty instances.
///
/// `fresh` is called separately for each check. Use test storage whose ordinary
/// loads and stores succeed. Fault injection belongs in [`check_failed_store`].
///
/// # Panics
///
/// On a contract violation, unexpected I/O failure, or expired deadline.
pub async fn check<B: Bookmark, D: Future<Output = ()>>(
    mut fresh: impl AsyncFnMut() -> B,
    mut deadline: impl FnMut() -> D,
) {
    check_storage(&mut fresh, &mut deadline).await;
    check_cancelled_store(&mut fresh, &mut deadline).await;
}

/// Check absent storage, exact replacements, and repeatable loads.
///
/// Reading all, part, or none of a returned reader must not consume storage.
/// A present empty record must remain distinguishable from an absent one.
/// The deadline covers construction and all operations on this instance.
///
/// # Panics
///
/// On a contract violation, unexpected I/O failure, or expired deadline.
pub async fn check_storage<B: Bookmark, D: Future<Output = ()>>(
    fresh: impl AsyncFnOnce() -> B,
    deadline: impl FnOnce() -> D,
) {
    timed("bookmark::check_storage", deadline(), async {
        let bookmark = fresh().await;
        for _ in 0..2 {
            assert!(
                load(&bookmark).await.is_none(),
                "conformance: fresh bookmark is empty"
            );
        }
        for (index, &size) in SIZES.iter().enumerate() {
            let bytes = contents(size, index as u8);
            bookmark
                .store(bytes.clone())
                .await
                .expect("conformance: store succeeds");
            assert_stored(&bookmark, &bytes).await;

            // Discarding a reader at any of these stages must not consume the
            // stored record, even if load or its reader uses shared state.
            {
                let mut loading = pin!(bookmark.load());
                if let Poll::Ready(result) = futures::poll!(loading.as_mut()) {
                    drop(result.expect("conformance: load succeeds"));
                }
            }
            assert_stored(&bookmark, &bytes).await;
            let mut reader = bookmark
                .load()
                .await
                .expect("conformance: load succeeds")
                .expect("conformance: stored bytes are present");
            let mut prefix = [0; 3];
            let read = reader
                .read(&mut prefix)
                .await
                .expect("conformance: reader succeeds");
            assert_eq!(
                &prefix[..read],
                &bytes[..read],
                "conformance: partial reads preserve stored bytes"
            );
            drop(reader);
            assert_stored(&bookmark, &bytes).await;
        }
    })
    .await;
}

/// Check replacement when a store is cancelled after its first poll.
///
/// The fixture's first store must succeed. The second is polled once and
/// dropped. If it completes successfully, its bytes must be present; if it is
/// pending, either complete record is allowed. The next store must succeed and
/// remain readable. An immediately completing store also conforms.
///
/// The deadline covers construction and the whole check. A controlled fixture
/// can pause the second store at a chosen stage to exercise that cancellation.
///
/// # Panics
///
/// On a contract violation, unexpected I/O failure, or expired deadline.
pub async fn check_cancelled_store<B: Bookmark, D: Future<Output = ()>>(
    fresh: impl AsyncFnOnce() -> B,
    deadline: impl FnOnce() -> D,
) {
    timed("bookmark::check_cancelled_store", deadline(), async {
        let bookmark = fresh().await;
        let previous = contents(4097, 0x31);
        let replacement = contents(113, 0x72);
        bookmark
            .store(previous.clone())
            .await
            .expect("conformance: initial store succeeds");
        assert_stored(&bookmark, &previous).await;
        let completed = {
            let mut storing = pin!(bookmark.store(replacement.clone()));
            match futures::poll!(storing.as_mut()) {
                Poll::Ready(result) => {
                    result.expect("conformance: uncancelled store succeeds");
                    true
                }
                Poll::Pending => false,
            }
        };
        if completed {
            assert_stored(&bookmark, &replacement).await;
        } else {
            assert_atomic(&bookmark, &previous, &replacement).await;
        }
        assert_next_store(&bookmark).await;
    })
    .await;
}

/// Check a failed store using caller-controlled fault injection.
///
/// Configure the fresh fixture so its first store succeeds, its second returns
/// an error, and subsequent operations succeed. Run this check at each failure
/// point the backend supports. Either complete record may remain after the
/// error; the following successful store must replace it and remain readable.
/// The deadline covers construction and the whole check.
///
/// # Panics
///
/// If the second store succeeds, any other operation fails, a record is torn
/// or lost, a subsequent store is rolled back, or the deadline expires.
pub async fn check_failed_store<B: Bookmark, D: Future<Output = ()>>(
    fresh: impl AsyncFnOnce() -> B,
    deadline: impl FnOnce() -> D,
) {
    timed("bookmark::check_failed_store", deadline(), async {
        let bookmark = fresh().await;
        let previous = contents(4097, 0x31);
        let replacement = contents(113, 0x72);
        bookmark
            .store(previous.clone())
            .await
            .expect("conformance: initial store succeeds");
        assert_stored(&bookmark, &previous).await;
        assert!(
            bookmark.store(replacement.clone()).await.is_err(),
            "conformance: fixture must fail the second store"
        );
        assert_atomic(&bookmark, &previous, &replacement).await;
        assert_next_store(&bookmark).await;
    })
    .await;
}

/// Read the complete record without interpreting its opaque bytes.
async fn load(bookmark: &impl Bookmark) -> Option<Vec<u8>> {
    let mut reader = bookmark.load().await.expect("conformance: load succeeds")?;
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .await
        .expect("conformance: reader succeeds");
    Some(bytes)
}

/// Repeated loads must return the complete last successful replacement.
async fn assert_stored(bookmark: &impl Bookmark, expected: &[u8]) {
    for _ in 0..2 {
        let actual = load(bookmark).await;
        assert!(
            actual.as_deref() == Some(expected),
            "conformance: load preserves the exact stored bytes (expected {} bytes, got {:?})",
            expected.len(),
            actual.as_ref().map(Vec::len),
        );
    }
}

/// An unconfirmed replacement may leave either complete record, never a mixture.
async fn assert_atomic(bookmark: &impl Bookmark, previous: &[u8], replacement: &[u8]) {
    for _ in 0..2 {
        let actual = load(bookmark).await;
        assert!(
            actual.as_deref() == Some(previous) || actual.as_deref() == Some(replacement),
            "conformance: an interrupted store leaves a complete previous or replacement record"
        );
    }
}

/// A later successful store takes precedence over any interrupted work.
async fn assert_next_store(bookmark: &impl Bookmark) {
    let bytes = contents(257, 0xb4);
    bookmark
        .store(bytes.clone())
        .await
        .expect("conformance: store after interruption succeeds");
    assert_stored(bookmark, &bytes).await;
}

#[cfg(test)]
mod tests;
