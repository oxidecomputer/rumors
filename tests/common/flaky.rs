//! A flaky in-memory [`Bookmark`] for adversarial identity-persistence tests.
//!
//! [`FlakyInMemoryBookmark`] is the durable identity store a real deployment
//! would back with a disk: it holds the exact framed bytes the crate serialized
//! (or `None` until the first write) and survives a peer's in-memory crash. Two
//! things make it a test instrument rather than a toy:
//!
//! - **It fails on a schedule.** Each read and each write consults a
//!   [`FaultFeed`] — a proptest-generated, shrinkable sequence of booleans —
//!   and returns [`FlakyError`] when the next decision says so. A failed write
//!   is exactly the moment the crate's `Bookmarked` cache reverts to its
//!   on-disk state, the persistence gap this whole test exists to probe.
//! - **It stores opaque bytes.** The crate owns the on-disk format, so this
//!   store only shuttles the framed bytes it is handed — keeping it a faithful
//!   model of a real disk-backed store, which sees bytes and not records. Tests
//!   that need to inspect *what* was persisted decode through
//!   [`persisted_record`].
//!
//! The `store` and `faults` are held behind [`Arc`]s so a crashed peer recovers
//! by wrapping a *fresh* `FlakyInMemoryBookmark` around the *same* durable
//! state: the in-memory peer is gone, but its disk and its remaining fault
//! schedule are not.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex};

use before::Clock;
use rumors::{BOOKMARK_MAGIC, Bookmark, BookmarkError, Network, Serialized};
use tokio::io::AsyncWrite;

/// The durable "disk": the framed bytes last persisted, or `None` until the
/// first write. Shared across a node's incarnations so it outlives a crash.
pub type DurableStore = Arc<Mutex<Option<Vec<u8>>>>;

/// The fixed-header width of a bookmark frame — magic, the 2-byte format
/// version, and the 32-byte BLAKE3 integrity hash — before the borsh payload.
///
/// Mirrors the crate-private `format::HEADER_LEN`. Integration tests cannot
/// reach the crate's codec, so they strip this known header to read the payload;
/// the format-pin snapshots guard the layout against drift.
const FRAME_HEADER_LEN: usize = BOOKMARK_MAGIC.len() + 2 + 32;

/// Decode the record a persisted store holds, or an empty record if nothing has
/// been written. Strips the crate's frame header and borsh-decodes the payload.
pub fn persisted_record(store: &DurableStore) -> BTreeMap<Network, Vec<Clock>> {
    match &*store.lock().unwrap() {
        None => BTreeMap::new(),
        Some(bytes) => borsh::from_slice(&bytes[FRAME_HEADER_LEN..])
            .expect("decode persisted bookmark payload"),
    }
}

/// The error a scheduled read/write failure reports. Carries which operation
/// tripped, only for legible test diagnostics.
#[derive(Debug)]
pub struct FlakyError {
    op: &'static str,
}

impl fmt::Display for FlakyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "flaky bookmark: injected {} failure", self.op)
    }
}

impl std::error::Error for FlakyError {}

/// One peer's bookmark fail schedule, consumed in call order.
///
/// `reads` and `writes` are independent queues of "fail this one?" decisions;
/// each bookmark load/store pops the next. An
/// exhausted queue defaults to success, so shrinking a schedule toward empty
/// shrinks monotonically toward fault-free — the minimal counterexample is the
/// shortest prefix of failures that still reproduces a bug.
///
/// `enabled` is the master switch the heal phase flips off: a fault-free heal
/// is what makes the convergence and disjointness assertions reachable.
pub struct FaultFeed {
    reads: VecDeque<bool>,
    writes: VecDeque<bool>,
    enabled: bool,
}

impl FaultFeed {
    /// A feed that fails the reads and writes flagged `true`, in order.
    pub fn new(reads: Vec<bool>, writes: Vec<bool>) -> Self {
        Self {
            reads: reads.into(),
            writes: writes.into(),
            enabled: true,
        }
    }

    /// Stop injecting faults: every later read and write succeeds. Irreversible,
    /// and called on every feed before the heal phase.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    fn next_read(&mut self) -> bool {
        self.enabled && self.reads.pop_front().unwrap_or(false)
    }

    fn next_write(&mut self) -> bool {
        self.enabled && self.writes.pop_front().unwrap_or(false)
    }
}

/// A durable identity store that fails on a [`FaultFeed`]'s schedule.
///
/// One per peer incarnation; a crash drops the peer but the `store` and
/// `faults` [`Arc`]s outlive it, so the next incarnation reloads the same
/// record and the same remaining schedule.
pub struct FlakyInMemoryBookmark {
    /// The persisted framed bytes — the "disk". Shared so they survive the peer
    /// that wrote them.
    store: DurableStore,
    /// The fail schedule, shared for the same reason.
    faults: Arc<Mutex<FaultFeed>>,
    /// The owning peer's label, for diagnostics only.
    label: usize,
}

impl fmt::Debug for FlakyInMemoryBookmark {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FlakyInMemoryBookmark")
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

impl FlakyInMemoryBookmark {
    /// Wrap shared durable `store` and `faults` for peer `label`.
    pub fn new(store: DurableStore, faults: Arc<Mutex<FaultFeed>>, label: usize) -> Self {
        Self {
            store,
            faults,
            label,
        }
    }
}

impl BookmarkError for FlakyInMemoryBookmark {
    type Error = FlakyError;
}

impl Bookmark for FlakyInMemoryBookmark {
    type Reader = std::io::Cursor<Vec<u8>>;

    async fn load(&self) -> Result<Option<Self::Reader>, Self::Error> {
        let _ = self.label;
        if self.faults.lock().unwrap().next_read() {
            return Err(FlakyError { op: "read" });
        }
        Ok(self.store.lock().unwrap().clone().map(std::io::Cursor::new))
    }

    async fn store<F>(&self, write: F) -> Result<(), Self::Error>
    where
        F: for<'a> FnOnce(&'a mut (dyn AsyncWrite + Unpin + Send)) -> Serialized<'a> + Send,
    {
        // The fault stands in for a commit that never lands: return before
        // touching the durable bytes, so a failed write leaves the prior frame
        // exactly as it was — the atomicity the crate's recovery relies on.
        if self.faults.lock().unwrap().next_write() {
            return Err(FlakyError { op: "write" });
        }
        let mut buf: Vec<u8> = Vec::new();
        write(&mut buf)
            .await
            .expect("writing to an in-memory buffer is infallible");
        *self.store.lock().unwrap() = Some(buf);
        Ok(())
    }
}
