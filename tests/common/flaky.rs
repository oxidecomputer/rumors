//! Byte storage with scheduled read and write failures.
//!
//! Storage and fault schedules outlive each peer, so a restarted peer reloads
//! the same durable record. A failed write preserves the previous bytes.
//! Tests inspect records through `persisted_record`; the store itself treats
//! them as opaque.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex};

use before::Clock;
use ciborium::value::Value;
use rumors::{Bookmark, Network};

/// The durable "disk": the framed bytes last persisted, or `None` until the
/// first write. Shared across a node's incarnations so it outlives a crash.
pub type DurableStore = Arc<Mutex<Option<Vec<u8>>>>;

/// Decode the record a persisted store holds, or an empty record if nothing has
/// been written.
///
/// Integration tests cannot reach the crate's codec, and don't need to: the
/// stored file is one self-described CBOR item, so a generic walk — unwrap
/// tag 55799, take the frame array's payload item, unwrap tag 24, then strip
/// each stored clock's tag — recovers the untagged record serde understands.
/// The format-pin snapshots guard the frame shape against drift; a panic here
/// means this harness fell behind the format, not that the peer is broken.
pub fn persisted_record(store: &DurableStore) -> BTreeMap<Network, Vec<Clock>> {
    match &*store.lock().unwrap() {
        None => BTreeMap::new(),
        Some(bytes) => persisted_record_bytes(bytes),
    }
}

/// Walk one persisted bookmark file into its record, generically.
fn persisted_record_bytes(bytes: &[u8]) -> BTreeMap<Network, Vec<Clock>> {
    let file: Value =
        ciborium::de::from_reader(bytes).expect("the persisted bookmark parses as CBOR");
    let Value::Tag(55799, frame) = file else {
        panic!("the persisted bookmark is not self-described CBOR");
    };
    let Value::Array(items) = *frame else {
        panic!("the persisted frame is not an array");
    };
    let payload = items.into_iter().nth(2).expect("a three-item frame array");
    let Value::Tag(24, payload) = payload else {
        panic!("the persisted payload is not an embedded CBOR item");
    };
    let Value::Bytes(payload) = *payload else {
        panic!("the embedded payload is not a byte string");
    };

    let record: Value = ciborium::de::from_reader(payload.as_slice())
        .expect("the persisted payload parses as CBOR");
    let Value::Map(entries) = record else {
        panic!("the persisted record is not a map");
    };
    // Strip each clock's identity tag so the plain serde impls (which are
    // deliberately untagged) can decode the record.
    let untagged = Value::Map(
        entries
            .into_iter()
            .map(|(key, clocks)| {
                let Value::Array(clocks) = clocks else {
                    panic!("a persisted record entry is not an array of clocks");
                };
                let stripped = clocks
                    .into_iter()
                    .map(|clock| match clock {
                        Value::Tag(_, inner) => *inner,
                        untagged => untagged,
                    })
                    .collect();
                (key, Value::Array(stripped))
            })
            .collect(),
    );
    let mut buf = Vec::new();
    ciborium::ser::into_writer(&untagged, &mut buf).expect("re-encoding a record is infallible");
    ciborium::de::from_reader(buf.as_slice()).expect("decode persisted bookmark payload")
}

/// The error a scheduled read/write failure reports. Carries which operation
/// tripped, only for legible test diagnostics.
#[derive(Debug)]
pub struct FlakyError {
    op: &'static str,
}

/// Construct a fault for tests without a generated schedule.
impl FlakyError {
    /// The error an injected write failure reports, for tests that need the
    /// value without a scheduled fault.
    pub fn injected_write() -> Self {
        FlakyError { op: "write" }
    }
}

/// Identify which storage operation failed.
impl fmt::Display for FlakyError {
    /// Format the operation named by the injected failure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "flaky bookmark: injected {} failure", self.op)
    }
}

/// Expose injected faults through the standard error interface.
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

/// Consume independent read and write failure schedules.
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

    /// Whether a scheduled failure remains: the feed is enabled and a `true`
    /// decision is still queued for a read or a write.
    ///
    /// An exhausted or all-`false` queue never fails, so a session over such
    /// a feed cannot legitimately report a bookmark error.
    pub fn may_fail(&self) -> bool {
        self.enabled && (self.reads.contains(&true) || self.writes.contains(&true))
    }

    /// Consume the next read decision, defaulting to success.
    fn next_read(&mut self) -> bool {
        self.enabled && self.reads.pop_front().unwrap_or(false)
    }

    /// Consume the next write decision, defaulting to success.
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

/// Identify the peer without printing its durable state.
impl fmt::Debug for FlakyInMemoryBookmark {
    /// Show only the peer’s diagnostic label.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FlakyInMemoryBookmark")
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

/// Attach a peer incarnation to shared storage and fault decisions.
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

/// Model repeatable reads and atomic stores under injected failures.
impl Bookmark for FlakyInMemoryBookmark {
    /// The storage failure reported by this implementation.
    type Error = FlakyError;
    /// An independent snapshot of the stored bytes.
    type Reader = std::io::Cursor<Vec<u8>>;

    /// Return current storage unless this read is scheduled to fail.
    async fn load(&self) -> Result<Option<Self::Reader>, Self::Error> {
        if self.faults.lock().unwrap().next_read() {
            return Err(FlakyError { op: "read" });
        }
        Ok(self.store.lock().unwrap().clone().map(std::io::Cursor::new))
    }

    /// Replace storage unless this write is scheduled to fail.
    async fn store(&self, bytes: Vec<u8>) -> Result<(), Self::Error> {
        // This harness injects failure before replacement. The transmission
        // boundary tests also exercise errors after a complete replacement.
        if self.faults.lock().unwrap().next_write() {
            return Err(FlakyError { op: "write" });
        }
        *self.store.lock().unwrap() = Some(bytes);
        Ok(())
    }
}
