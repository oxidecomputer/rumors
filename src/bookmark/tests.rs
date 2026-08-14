//! The persistence driver's read path is total over byte sources: bounded at
//! [`BOOKMARK_MAX_BYTES`] with the boundary exact, so no stored record — however
//! corrupt, foreign, or endless — can force an unbounded buffer.

use super::*;

/// A [`Bookmark`] whose stored record is `len` zero bytes, produced lazily:
/// the byte source allocates nothing, so any unbounded buffering is the
/// reader's fault alone. `u64::MAX` models a source that never ends.
struct ZeroBytes {
    len: u64,
}

impl BookmarkError for ZeroBytes {
    type Error = std::convert::Infallible;
}

impl Bookmark for ZeroBytes {
    type Reader = tokio::io::Take<tokio::io::Repeat>;

    async fn load(&self) -> Result<Option<Self::Reader>, Self::Error> {
        Ok(Some(tokio::io::AsyncReadExt::take(
            tokio::io::repeat(0),
            self.len,
        )))
    }

    async fn store<F>(&self, _write: F) -> Result<(), Self::Error>
    where
        F: for<'a> FnOnce(&'a mut (dyn AsyncWrite + Unpin + Send)) -> Serialized<'a> + Send,
    {
        unreachable!("the read-path tests never store");
    }
}

/// Decode `len` stored zero bytes through the full read path.
fn read_zeros(
    len: u64,
) -> Result<BTreeMap<Network, Vec<Clock>>, BookmarkIo<std::convert::Infallible>> {
    pollster::block_on(Persist::read(&ZeroBytes { len }))
}

/// One byte past [`BOOKMARK_MAX_BYTES`] is refused as
/// [`FormatError::Oversized`]: the ceiling is exclusive above, and the refusal
/// arrives without buffering past the boundary.
#[test]
fn one_byte_over_the_ceiling_is_oversized() {
    assert!(matches!(
        read_zeros(BOOKMARK_MAX_BYTES + 1),
        Err(BookmarkIo::Format(FormatError::Oversized)),
    ));
}

/// Exactly [`BOOKMARK_MAX_BYTES`] stored bytes pass the size gate and fail
/// only on their content (zeros are not a bookmark frame): the ceiling is
/// inclusive at the boundary, so the size check never eats a legal frame.
#[test]
fn a_frame_at_the_ceiling_reaches_validation() {
    assert!(matches!(
        read_zeros(BOOKMARK_MAX_BYTES),
        Err(BookmarkIo::Format(FormatError::BadMagic { .. })),
    ));
}

/// A byte source that never ends still terminates in
/// [`FormatError::Oversized`]: the refusal is the ceiling's doing, never the
/// source's EOF.
#[test]
fn an_endless_source_terminates_oversized() {
    assert!(matches!(
        read_zeros(u64::MAX),
        Err(BookmarkIo::Format(FormatError::Oversized)),
    ));
}
