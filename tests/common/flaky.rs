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

/// Decode retained identities with their network's shared write frontier.
///
/// This independent CBOR reader preserves the clock-shaped view used by the
/// behavioral tests. It does not implement recovery or retention policy.
pub fn persisted_record(store: &DurableStore) -> BTreeMap<Network, Vec<Clock>> {
    let guard = store.lock().unwrap();
    let Some(bytes) = &*guard else {
        return BTreeMap::new();
    };
    let Value::Tag(55799, frame) = ciborium::de::from_reader(bytes.as_slice()).unwrap() else {
        panic!("the bookmark must be self-described CBOR");
    };
    let Value::Array(frame) = *frame else {
        panic!("the frame must be an array")
    };
    let Value::Tag(24, payload) = frame.into_iter().nth(2).unwrap() else {
        panic!("embedded CBOR")
    };
    let Value::Bytes(payload) = *payload else {
        panic!("an embedded byte string")
    };
    let Value::Array(networks) = ciborium::de::from_reader(payload.as_slice()).unwrap() else {
        panic!("the record must be an array");
    };
    networks
        .into_iter()
        .map(|value| {
            let Value::Array(fields) = value else {
                panic!("network fields")
            };
            let [network, frontier, identities]: [Value; 3] = fields.try_into().unwrap();
            let mut encoded_network = Vec::new();
            ciborium::ser::into_writer(&network, &mut encoded_network).unwrap();
            let network: Network = ciborium::de::from_reader(encoded_network.as_slice()).unwrap();
            let Value::Tag(rumors::tags::VERSION_TAG, frontier) = frontier else {
                panic!("tagged frontier")
            };
            let Value::Bytes(frontier) = *frontier else {
                panic!("frontier bytes")
            };
            let frontier = before::Version::decode(frontier.as_slice()).unwrap();
            let Value::Array(identities) = identities else {
                panic!("identities")
            };
            let clocks = identities
                .into_iter()
                .map(|value| {
                    let Value::Tag(rumors::tags::PARTY_TAG, identity) = value else {
                        panic!("tagged identity")
                    };
                    let Value::Bytes(identity) = *identity else {
                        panic!("identity bytes")
                    };
                    Clock::from_parts(
                        before::Party::decode(identity.as_slice()).unwrap(),
                        frontier.clone(),
                    )
                })
                .collect();
            (network, clocks)
        })
        .collect()
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
