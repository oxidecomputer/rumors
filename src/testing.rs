//! Executor-agnostic test support shared across protocol and API suites.

mod memnet;
pub(crate) mod schedule;
mod transport;

use std::{
    future::Future,
    pin::pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

use tokio::io::AsyncRead;

use crate::{
    Snapshot,
    tree::{
        mirror::streaming::{
            Backend, Local,
            remote::{self, RunBudget, codec::LeafRun},
            window::{
                self, DISPUTE_OVERHEAD_BYTES, DISPUTE_WIRE_BYTES, REFERENCE_SCOPE_BYTES,
                ReplicaSize, SUPPLY_DECODE_ENVELOPE_BYTES, Window,
            },
        },
        typed::{
            height::{Height, Root},
            untyped::census,
        },
    },
};

pub use crate::tree::mirror::streaming::remote::codec::{
    DecodeError as CodecDecodeError, DecodeErrorKind as CodecDecodeErrorKind, DecodeLeafError,
    FramePart, HeadError, LeafRunError,
};

pub use memnet::{MemoryDial, MemoryListen, MemoryName, MemoryNet};
pub use transport::{
    AdversarialAcceptor, AdversarialConnector, AdversarialRead, AdversarialWrite,
    FaultUnit as IoFaultUnit, InjectedIo, IoFault, IoPlan, IoReport, IoReportHandle,
    Operation as IoOperation, ReorderingAcceptor, Side as IoSide, reorder_accepts, wrap_io,
    wrap_link,
};

pub use crate::tree::mirror::streaming::remote::{
    FrameShape, HookCapture, HookStream, LinkCapture, PreparedFrame,
};

/// Join snapshots from one network in memory, preparing the result for readers.
///
/// This exposes the local join to allocation tests without including transport
/// setup, wire decoding, or a runtime in the measured work.
pub fn join_snapshots<T: Send + Sync>(ours: &Snapshot<T>, theirs: &Snapshot<T>) -> Snapshot<T> {
    assert_eq!(
        ours.network(),
        theirs.network(),
        "snapshots belong to different networks"
    );
    let mut tree = ours.tree().clone();
    tree.join(theirs.tree().clone());
    tree.warm_memos();
    Snapshot::new(ours.network(), tree)
}

/// Render two hook captures grouped by labeled logical streams.
pub fn render_hook_capture(a: &HookCapture, b: &HookCapture) -> String {
    remote::render_hook_capture(a, b)
}

/// Assert the concatenation of observed items reproduces `wire` exactly:
/// the totality witness behind rendering hook items as a wire-byte pin.
pub fn assert_items_account_for(items: &[Vec<u8>], wire: &[u8]) {
    remote::assert_items_account_for(items, wire);
}

/// Parse one data stream's on-wire open label, returning
/// `((epoch, index), label byte length)`.
pub fn stream_label(bytes: &[u8]) -> ((u8, u8), usize) {
    remote::stream_label(bytes)
}

/// A snapshot of the crate-wide census of live tree-node handles.
#[derive(Clone, Copy, Debug)]
pub struct NodeCensus {
    /// Handles alive at the snapshot.
    pub live: usize,
    /// The most handles ever concurrently alive since the last
    /// [`node_census_reset`].
    pub peak: usize,
}

/// Read the census of live tree-node handles.
///
/// Every constructed or cloned node handle counts one and every drop
/// releases one, so `peak` is exact concurrent residency: the measurable
/// shadow of the memory bound
/// [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget) derives.
/// The counters are process-global; tests that assert on them must own
/// the process (one test per process under nextest).
pub fn node_census() -> NodeCensus {
    let (live, peak) = census::read();
    NodeCensus { live, peak }
}

/// Restart the census high-water mark from the current live count.
pub fn node_census_reset() {
    census::reset_peak();
}

/// Mean modeled memory per scope in the reference sizing example.
///
/// This workload-specific estimate helps validate the explanatory model. The
/// window calculation derives its prices from the actual session instead.
pub fn reference_scope_bytes() -> usize {
    REFERENCE_SCOPE_BYTES
}

/// Mean wire bytes per differing reference-size message in the calibration fixture.
pub fn reference_wire_bytes() -> usize {
    DISPUTE_WIRE_BYTES
}

/// Approximate protocol bytes per differing message, excluding its payload.
///
/// Includes both directions' hashes, versions, framing, and session setup,
/// averaged over a calibration corpus. The actual mean varies by workload.
pub fn dispute_overhead_bytes() -> usize {
    DISPUTE_OVERHEAD_BYTES
}

/// Worst-case bytes one session's decode fans keep resident, under the
/// in-memory backend's pricing.
///
/// The window calculation reserves this amount before sizing its scope queues.
/// Measurement tests add it back when expressing results against the complete
/// session budget.
pub fn supply_decode_envelope_bytes() -> usize {
    SUPPLY_DECODE_ENVELOPE_BYTES
}

/// Read exactly `declared` payload bytes under the framing layer's
/// growth policy, returning the payload.
///
/// This is the declared-length payload read used for party hand-offs and
/// framed codec bodies, without the surrounding session. The decoder
/// allocation tests use it to measure memory requested for a declared length.
pub async fn read_declared_payload(
    mut read: impl AsyncRead + Unpin,
    declared: usize,
) -> std::io::Result<Vec<u8>> {
    crate::tree::mirror::framing::read_payload(&mut read, declared).await
}

/// The initial reservation granule of the framed-payload readers.
///
/// The decoder allocation tests use this value so their ceilings cannot drift
/// from the implementation.
pub fn frame_payload_chunk_len() -> usize {
    crate::tree::mirror::framing::PAYLOAD_CHUNK_LEN
}

/// A canonical supply run built outside an allocator-metered region.
#[derive(Debug)]
pub struct PreparedRecordRun(LeafRun);

/// Build a canonical run of `len` identical records for allocation metering.
pub fn prepare_record_run(len: usize) -> PreparedRecordRun {
    let mut run = LeafRun::new();
    let version = crate::Version::new();
    let message = crate::message::Message::try_from_arc(
        std::sync::Arc::new(0_u64),
        crate::message::PayloadDepthLimit::default(),
    )
    .expect("the integer fixture is an admissible payload");
    for _ in 0..len {
        run.push(&version, &message)
            .expect("the tiny fixture record fits a supply run");
    }
    PreparedRecordRun(run)
}

/// Decode and discard every record in a prepared supply run.
///
/// The allocation meter uses this narrow entry to price record-content
/// decoding after the run buffer and its canonical records are built.
pub fn decode_record_run(run: &PreparedRecordRun) -> Result<usize, DecodeLeafError> {
    let codec =
        crate::message::PayloadCodec::new::<u64>(crate::message::PayloadDepthLimit::default());
    run.0
        .records(codec)
        .try_fold(0, |count, record| record.map(|_| count + 1))
}

/// The wire prefix of one streaming-codec supply frame declaring a
/// `declared`-byte run.
///
/// Prepend it to a run body to hand [`decode_supply_frame`] a decodable
/// byte stream; the prefix is built by the codec's own head writers, so
/// the meter cannot drift from the wire.
pub fn supply_frame_head(declared: usize) -> Vec<u8> {
    remote::supply_frame_head(declared)
}

/// A structurally valid lone-record run of exactly `len` bytes, with
/// arbitrary record content.
///
/// The decoder allocation tests use this to build run bodies through the wire
/// implementation rather than a copy of its format.
pub fn lone_record_run(len: usize) -> Vec<u8> {
    remote::lone_record_run(len)
}

/// Decode one streaming-codec supply frame, discarding the decoded run.
///
/// The decoder allocation tests drive the codec's supply-read path through
/// this function and retain [`CodecDecodeError`] for failure classification.
/// It admits every framing-valid run; [`decode_supply_frame_budgeted`] covers
/// rejection at the session-budget boundary.
pub async fn decode_supply_frame(read: impl AsyncRead + Unpin) -> Result<(), CodecDecodeError> {
    decode_supply_frame_budgeted(read, usize::MAX).await
}

/// Decode one streaming-codec supply frame under a session run budget of
/// `budget` bytes, discarding the decoded run.
///
/// The decoder allocation tests use this to measure rejection at the
/// run-budget boundary. Budgets at or above the framing ceiling admit the same
/// inputs as [`decode_supply_frame`].
pub async fn decode_supply_frame_budgeted(
    read: impl AsyncRead + Unpin,
    budget: usize,
) -> Result<(), CodecDecodeError> {
    remote::decode_frame_discarded(read, RunBudget::from_bytes(budget)).await
}

/// Build one canonical streaming-codec frame outside the encoder allocation
/// measurement.
pub fn prepare_frame(shape: FrameShape) -> PreparedFrame {
    remote::prepare_frame(shape)
}

/// Write a prepared frame through the streaming codec's async frame
/// writer into `out`, exactly as a session's stream sender writes it.
///
/// The caller pre-reserves `out`, allowing the encoder allocation tests to
/// exclude growth of the destination buffer.
pub async fn write_prepared_frame(frame: &PreparedFrame, out: &mut Vec<u8>) {
    remote::write_prepared_frame(frame, out).await
}

/// Render the [sizing guide](crate::sizing)'s table from the window calculation.
///
/// Each cell estimates slowdown as `max(1, BDP_messages / window)`. The
/// window tests compare the committed table with this deterministic output.
pub fn window_tradeoff_table() -> String {
    window::tradeoff_table()
}

/// Messages per replica in the reference sizing example.
pub const REFERENCE_SESSION_MESSAGES: usize = window::REFERENCE_SESSION_MESSAGES as usize;

/// The largest canonical version-bound encoding in a snapshot's tree, in
/// bytes.
///
/// Covers every bound the tree holds (leaf versions and every interior
/// ceiling and floor); the result is the exact per-node aggregate the
/// greeting exchanges.
pub fn max_version_bytes<T: Send + Sync + 'static>(snapshot: &Snapshot<T>) -> usize {
    snapshot.tree().max_version_bytes()
}

/// The largest canonical per-node version-bound encoding in a snapshot's
/// tree, recomputed by direct walk with no aggregate memo consulted.
///
/// The session memory model prices every bound a session can hold
/// within the exchanged pair sum (`local_max + remote_max`), and
/// deletion-honoring can assemble bounds over survivor subsets neither
/// input materialized; this walk measures a reconciled tree against the
/// pre-session exchange, so tests can pin the model side of the account
/// to reality.
pub fn max_bound_bytes<T: Send + Sync + 'static>(snapshot: &Snapshot<T>) -> usize {
    snapshot.tree().max_bound_bytes()
}

/// The per-height channel capacities a session derives from its budget
/// and the two replicas' exchanged set sizes.
///
/// The result is ordered by typed height, from leaves through the root, using
/// the in-memory backend's node price. Integration tests use it to predict
/// where a session should encounter backpressure.
pub fn window_capacities(local_len: u64, remote_len: u64, budget_bytes: usize) -> Vec<usize> {
    let window = Window::from_budget(
        [
            ReplicaSize::new(local_len, 0),
            ReplicaSize::new(remote_len, 0),
        ],
        budget_bytes,
        <Local as Backend>::node_bytes,
    );
    (0..=<Root as Height>::HEIGHT)
        .map(|height| window.capacity(height))
        .collect()
}

/// Why polling stopped before a closed in-memory future completed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Quiescence {
    /// The future returned `Pending` without arranging another poll.
    Stalled,
    /// The future kept self-waking beyond the runaway guard.
    PollBudget,
}

/// Records whether the subject future requested another poll.
struct WakeFlag(AtomicBool);

/// Convert wakeups into a flag the closed-world poller can inspect.
impl Wake for WakeFlag {
    /// Record an owned wakeup.
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::Release);
    }

    /// Record a borrowed wakeup.
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.store(true, Ordering::Release);
    }
}

/// Poll a closed, in-memory future until it completes or becomes quiescent.
///
/// Every legitimate suspension must arrange another wake. A `Pending` poll
/// without one is therefore a deterministic deadlock witness rather than a
/// wall-clock guess. Futures waiting on external events do not satisfy this
/// closed-world premise and should use their real liveness mechanism instead.
/// Tokio's cooperative budget is disabled around the subject so that invoking
/// this detector from within a Tokio task cannot turn a scheduler yield into a
/// false deadlock report.
pub fn run_to_quiescence<F: Future>(future: F) -> Result<F::Output, Quiescence> {
    // More than an order of magnitude above the deepest closed-world run
    // the suites drive (the whole link conformance suite at one-byte
    // windows), so a legitimate long session is never misreported as a
    // runaway. Re-measure that run before lowering this.
    const MAX_POLLS: usize = 1_000_000;

    let wake = Arc::new(WakeFlag(AtomicBool::new(true)));
    let waker = Waker::from(wake.clone());
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(tokio::task::coop::unconstrained(future));

    for _ in 0..MAX_POLLS {
        wake.0.store(false, Ordering::Release);
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(output) => return Ok(output),
            Poll::Pending if !wake.0.swap(false, Ordering::AcqRel) => {
                return Err(Quiescence::Stalled);
            }
            Poll::Pending => {}
        }
    }
    Err(Quiescence::PollBudget)
}

/// Unit tests for this support module.
#[cfg(test)]
mod tests;
