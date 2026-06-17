//! A flaky in-memory [`Bookmark`] for adversarial identity-persistence tests.
//!
//! [`FlakyInMemoryBookmark`] is the durable identity store a real deployment
//! would back with a disk: it holds the canonical `BTreeMap<Network,
//! Vec<Clock>>` record and survives a peer's in-memory crash. Two things make
//! it a test instrument rather than a toy:
//!
//! - **It fails on a schedule.** Each read and each write consults a
//!   [`FaultFeed`] — a proptest-generated, shrinkable sequence of booleans —
//!   and returns [`FlakyError`] when the next decision says so. A failed write
//!   is exactly the moment the crate's `Bookmarked` cache reverts to its
//!   on-disk state, the persistence gap this whole test exists to probe.
//! - **It round-trips through Borsh.** A [`Clock`] owns an identity region and
//!   is `!Clone`, so the store cannot hand out owned copies by cloning. Every
//!   [`read`](Bookmark::read) re-decodes the record from bytes and every
//!   [`write`](Bookmark::write) re-encodes it — which also faithfully models a
//!   real store's serialize-to-disk / deserialize-on-load round trip, rather
//!   than aliasing the live record.
//!
//! The `store` and `faults` are held behind [`Arc`]s so a crashed peer recovers
//! by wrapping a *fresh* `FlakyInMemoryBookmark` around the *same* durable
//! state: the in-memory peer is gone, but its disk and its remaining fault
//! schedule are not.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex};

use before::Clock;
use rumors::{Bookmark, BookmarkError, Network};

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
/// each [`read`](Bookmark::read)/[`write`](Bookmark::write) pops the next. An
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
    /// The canonical persisted record — the "disk". Shared so it survives the
    /// peer that wrote it.
    store: Arc<Mutex<BTreeMap<Network, Vec<Clock>>>>,
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
    pub fn new(
        store: Arc<Mutex<BTreeMap<Network, Vec<Clock>>>>,
        faults: Arc<Mutex<FaultFeed>>,
        label: usize,
    ) -> Self {
        Self {
            store,
            faults,
            label,
        }
    }
}

/// Deep-copy a record by round-tripping it through Borsh, the only way to
/// duplicate `!Clone` [`Clock`]s — and the same byte round trip a real store
/// makes against its disk.
fn round_trip(record: &BTreeMap<Network, Vec<Clock>>) -> BTreeMap<Network, Vec<Clock>> {
    let bytes = borsh::to_vec(record).expect("encode bookmark record");
    borsh::from_slice(&bytes).expect("decode bookmark record")
}

impl BookmarkError for FlakyInMemoryBookmark {
    type Error = FlakyError;
}

impl Bookmark for FlakyInMemoryBookmark {
    async fn read(&self) -> Result<BTreeMap<Network, Vec<Clock>>, Self::Error> {
        let _ = self.label;
        if self.faults.lock().unwrap().next_read() {
            return Err(FlakyError { op: "read" });
        }
        Ok(round_trip(&self.store.lock().unwrap()))
    }

    async fn write(&self, bookmarks: &BTreeMap<Network, Vec<Clock>>) -> Result<(), Self::Error> {
        if self.faults.lock().unwrap().next_write() {
            return Err(FlakyError { op: "write" });
        }
        // Commit a private deep copy: the durable record must not alias the
        // caller's live one, exactly as a serialize-to-disk write would not.
        *self.store.lock().unwrap() = round_trip(bookmarks);
        Ok(())
    }
}
