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
//! - **It can roll back.** Alongside the fail decisions, a [`FaultFeed`]
//!   carries a schedule of *rollback* decisions: a load flagged `true` serves
//!   the frame the most recent commit displaced instead of the current one —
//!   a store regressing exactly one commit, the violation of
//!   [`Bookmark::store`]'s atomic-replace obligation that the rollback probes
//!   demonstrate the consequences of. The crate declares this violation
//!   undetectable (the bookmark is a peer's only durable state, so there is
//!   nothing to compare a served frame against); these faults exist to show
//!   what it corrupts, not to be caught.
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
    rollbacks: VecDeque<bool>,
    /// What the most recent successful commit displaced: what a scheduled
    /// rollback serves.
    ///
    /// An inner `None` means that commit was the store's first (a rollback
    /// to the empty state); the outer `None` means no commit has displaced
    /// anything yet.
    ///
    /// Lives in the feed (not the [`DurableStore`]) so it rides the same
    /// [`Arc`] across a peer's incarnations without changing the durable
    /// bytes' shape.
    displaced: Option<Option<Vec<u8>>>,
    /// How many write decisions have been consulted, fault or not: lets a
    /// test pin its schedule indices mechanically instead of narrating them.
    writes_consulted: usize,
    enabled: bool,
}

impl FaultFeed {
    /// A feed that fails the reads and writes flagged `true`, in order, and
    /// never rolls back.
    pub fn new(reads: Vec<bool>, writes: Vec<bool>) -> Self {
        Self {
            reads: reads.into(),
            writes: writes.into(),
            rollbacks: VecDeque::new(),
            displaced: None,
            writes_consulted: 0,
            enabled: true,
        }
    }

    /// How many write decisions this feed has been consulted for so far —
    /// every [`Bookmark::store`] attempt consults exactly one, fault or not —
    /// so a schedule's boundary indices can be asserted rather than narrated.
    pub fn writes_consulted(&self) -> usize {
        self.writes_consulted
    }

    /// Schedule rollback decisions: each non-failing load — including one
    /// that finds nothing stored — pops the next, and `true` makes it serve
    /// what the most recent commit displaced instead of the current bytes.
    ///
    /// # Panics
    ///
    /// A load whose rollback decision fires before any commit has displaced
    /// anything panics: the schedule is misplaced, and a silent fallback
    /// would turn the probe into a no-op.
    pub fn with_rollbacks(mut self, rollbacks: Vec<bool>) -> Self {
        self.rollbacks = rollbacks.into();
        self
    }

    /// Stop injecting faults: every later read and write succeeds (and serves
    /// current bytes). Irreversible, and called on every feed before the heal
    /// phase.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    fn next_read(&mut self) -> bool {
        self.enabled && self.reads.pop_front().unwrap_or(false)
    }

    fn next_write(&mut self) -> bool {
        self.writes_consulted += 1;
        self.enabled && self.writes.pop_front().unwrap_or(false)
    }

    fn next_rollback(&mut self) -> bool {
        self.enabled && self.rollbacks.pop_front().unwrap_or(false)
    }

    /// Record that a commit has replaced `frame` (`None` when the commit was
    /// the store's first): the one-step rollback target.
    fn displace(&mut self, frame: Option<Vec<u8>>) {
        self.displaced = Some(frame);
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
        let mut faults = self.faults.lock().unwrap();
        if faults.next_read() {
            return Err(FlakyError { op: "read" });
        }
        // Every non-failing load consumes one rollback decision: `true`
        // serves what the most recent commit displaced — the store
        // regressing one commit, possibly to its empty state — instead of
        // the current bytes.
        let bytes = if faults.next_rollback() {
            faults
                .displaced
                .clone()
                .expect("rollback fault fired before any commit displaced anything")
        } else {
            self.store.lock().unwrap().clone()
        };
        Ok(bytes.map(std::io::Cursor::new))
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
        // Commit and record the displaced frame under one feed-lock hold, so
        // no interleaved commit can wedge a stale frame between the two.
        let mut faults = self.faults.lock().unwrap();
        let replaced = self.store.lock().unwrap().replace(buf);
        faults.displace(replaced);
        Ok(())
    }
}
