//! The persistence driver is total at the [`BOOKMARK_MAX_BYTES`] ceiling.
//!
//! On both sides: reads are bounded with the boundary exact, so no stored
//! record — however corrupt, foreign, or endless — can force an unbounded
//! buffer, and writes refuse an over-ceiling frame before storage sees it,
//! so no record can be committed that a later load would refuse.

use super::*;

/// A [`Bookmark`] whose stored record is `len` zero bytes, produced lazily:
/// the byte source allocates nothing, so any unbounded buffering is the
/// reader's fault alone. `u64::MAX` models a source that never ends.
///
/// Its `store` panics: the write-side tests drive records that must be
/// refused *before* the storage is invoked, so reaching it is the failure.
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
        unreachable!("an over-ceiling frame must be refused before the store is invoked");
    }
}

/// Decode `len` stored zero bytes through the full read path.
fn read_zeros(
    len: u64,
) -> Result<BTreeMap<Network, Vec<Clock>>, BookmarkIo<std::convert::Infallible>> {
    pollster::block_on(Persist::read(&ZeroBytes { len }))
}

/// One byte past [`BOOKMARK_MAX_BYTES`] is refused as
/// [`FormatError::Oversized`]: the ceiling is exclusive above, and the read
/// buffers exactly one byte past it — the byte that proves the excess —
/// and not one more.
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

/// A record whose encoded frame exceeds [`BOOKMARK_MAX_BYTES`] is refused by
/// the write side as [`FormatError::Oversized`] before the storage is
/// invoked: storing can never brick the bookmark.
///
/// [`ZeroBytes`]'s panicking `store` is the proof that the refusal precedes
/// the storage call, and the previously committed record stays loadable.
///
/// The record is the cheapest legal over-ceiling encoding: networks mapped
/// to empty clock lists, each costing its 16 network bytes plus a 4-byte
/// list length, with the entry count derived from the ceiling rather than
/// hand-counted and the overshoot asserted as a precondition.
#[test]
fn an_over_ceiling_record_is_refused_before_the_store_runs() {
    // 20 encoded bytes per entry: the 16-byte network id plus borsh's
    // 4-byte length prefix for the empty clock list.
    const ENTRY_LEN: u64 = 16 + 4;
    let entries = BOOKMARK_MAX_BYTES / ENTRY_LEN + 1;
    let record: BTreeMap<Network, Vec<Clock>> = (0..entries)
        .map(|n| {
            let mut id = [0u8; 16];
            id[..8].copy_from_slice(&n.to_be_bytes());
            (Network::from_bytes(id), Vec::new())
        })
        .collect();
    assert!(
        format::encode(&record).len() as u64 > BOOKMARK_MAX_BYTES,
        "precondition: the constructed record must encode over the ceiling",
    );

    let refused = pollster::block_on(Persist::write(&ZeroBytes { len: 0 }, &record));
    assert!(matches!(
        refused,
        Err(BookmarkIo::Format(FormatError::Oversized)),
    ));
}

/// A realistic record sits far under the ceiling: the *measured* backing for
/// the headroom claim in [`Bookmark::load`]'s docs.
///
/// The record here is deliberately heavy — many universes, each holding a
/// deeply forked clock family, the shape produced by heavy churn without a
/// roll call — and its encoded frame must still sit at least three orders of
/// magnitude under [`BOOKMARK_MAX_BYTES`]. If record encodings ever grow
/// toward the ceiling, this pin trips before deployments do.
#[test]
fn a_heavily_forked_record_sits_far_under_the_ceiling() {
    let record: BTreeMap<Network, Vec<Clock>> = (0..16u8)
        .map(|n| {
            let mut clock = Clock::seed();
            let mut clocks: Vec<Clock> = Vec::new();
            for _ in 0..64 {
                let mut fork = clock.fork();
                fork.tick();
                clocks.push(fork);
            }
            clock.tick();
            clocks.push(clock);
            (Network::from_bytes([n; 16]), clocks)
        })
        .collect();

    let encoded = format::encode(&record).len() as u64;
    assert!(
        encoded * 1024 <= BOOKMARK_MAX_BYTES,
        "a heavily forked record must sit three orders of magnitude under the \
         ceiling, but {encoded} bytes * 1024 exceeds {BOOKMARK_MAX_BYTES}",
    );
}
