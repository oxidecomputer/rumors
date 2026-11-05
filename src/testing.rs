//! Executor-agnostic test support shared across protocol and API suites.

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

use tokio::io::{AsyncRead, DuplexStream};

use crate::{
    Snapshot,
    tree::{
        mirror::streaming::{
            Backend, FAN, Local,
            materialized::{Resolution, Resolve},
            remote::{self, RunBudget, SUPPLY_DECODE_ENVELOPE_BYTES, codec::LeafRun},
        },
        typed::{
            ErasedPrefix,
            hash::{Hash, MAX_BRANCH_PREIMAGE_LEN, MERKLE_HASH_LEN as TREE_HASH_LEN, PATH_LEN},
            height::{Height, Root},
            untyped::{self, census},
        },
    },
};

pub use crate::tree::mirror::streaming::remote::codec::{
    DecodeError as CodecDecodeError, DecodeErrorKind as CodecDecodeErrorKind, DecodeLeafError,
    FramePart, HeadError, LeafRunError,
};

pub use crate::tree::mirror::streaming::holding::{
    Holding, Hook, ReportHook, attribute_allocations, report_allocations,
};
pub use crate::tree::mirror::streaming::ledger::{
    Admission, AdmissionHook, ChargeUnit, FitHook, LedgerChange, LedgerHook, PLACES, ReplyFit,
    SharePeak, observe_admissions, observe_fits, observe_ledgers,
};
pub use crate::tree::mirror::streaming::tasks::{Pacer, TaskRole, pace_tasks};

pub use transport::{
    AdversarialRead, AdversarialWrite, FaultUnit as IoFaultUnit, InjectedIo, IoFault, IoPlan,
    IoReport, IoReportHandle, Operation as IoOperation, Side as IoSide, wrap_io,
};

pub use crate::tree::mirror::streaming::remote::{
    FrameShape, HookCapture, HookStream, MultiplexerHook, MultiplexerPoll, PipeCapture,
    PreparedFrame, observe_multiplexers,
};

/// The completion marker each side writes in a session's closing exchange,
/// after all of its session work: the CBOR text item `"."`.
pub const COMPLETION_MARKER: [u8; 2] = crate::peer::COMPLETION_MARKER;

/// The fixed width of the preamble that opens every session.
pub const PREAMBLE_LEN: usize = crate::tree::mirror::preamble::V2_PREAMBLE_LEN;

/// Bytes each direction of a [`memory_connection`] buffers by default
/// before its writer waits for its reader.
///
/// Sessions are live at any positive capacity, so the buffer sets the
/// pipe's granularity, not its correctness: frames smaller than this cross
/// without waiting, and larger transfers stream through in refills.
pub const MEMORY_CAPACITY: usize = 8 * 1024;

/// One end of an in-memory connection: the two halves a session takes.
///
/// Pass `&mut end.read, &mut end.write` to a session to keep the end for
/// later sessions.
#[derive(Debug)]
pub struct MemoryEnd {
    /// The read half: the peer's write half delivers here.
    pub read: DuplexStream,
    /// The write half: it delivers to the peer's read half.
    pub write: DuplexStream,
}

/// Create the two ends of one in-memory connection, each direction a
/// [`tokio::io::duplex`] pipe buffering `capacity` bytes.
///
/// The directions are separate pipes, so each progresses independently,
/// as the transport requirements demand: a read proceeds while the same
/// end's write waits.
///
/// # Panics
///
/// If `capacity` is zero: a zero-capacity pipe could never carry a byte.
pub fn memory_connection(capacity: usize) -> (MemoryEnd, MemoryEnd) {
    assert!(capacity > 0, "a connection must buffer at least one byte");
    let (a_write, b_read) = tokio::io::duplex(capacity);
    let (b_write, a_read) = tokio::io::duplex(capacity);
    (
        MemoryEnd {
            read: a_read,
            write: a_write,
        },
        MemoryEnd {
            read: b_read,
            write: b_write,
        },
    )
}

/// Width of the subtree digests consumed by [`branch_hash`].
pub const MERKLE_HASH_LEN: usize = TREE_HASH_LEN;

/// Hash a branch through the production preimage builder.
///
/// The branch-hashing benchmark uses this test-only entry so it cannot drift
/// from the implementation whose feeding strategy it measures.
pub fn branch_hash(
    prefix: &[u8],
    children: impl IntoIterator<Item = (u8, [u8; MERKLE_HASH_LEN])>,
) -> [u8; MERKLE_HASH_LEN] {
    Hash::branch(
        prefix,
        children
            .into_iter()
            .map(|(radix, hash)| (radix, Hash::from(hash))),
    )
    .into()
}

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

/// Render two hook captures grouped by logical stream.
pub fn render_hook_capture(a: &HookCapture, b: &HookCapture) -> String {
    remote::render_hook_capture(a, b)
}

/// Assert the concatenation of observed items reproduces `wire` exactly:
/// the totality witness behind rendering hook items as a wire-byte pin.
pub fn assert_items_account_for(items: &[Vec<u8>], wire: &[u8]) {
    remote::assert_items_account_for(items, wire);
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

/// Approximate wire overhead per differing message, excluding its encoded payload.
///
/// `tests/dispute_wire.rs` counts all protocol writes in both directions,
/// divides by the differing messages, and subtracts their encoded payloads.
/// The reference fixture has 2,048 shared messages and 8,192 new ones per
/// side. Its whole-byte mean gives 45; the unrounded mean is about 45.4.
/// Two fully divergent 75,000-message sets instead average about 42.6,
/// since supplies dominate them and listings, whose entries each carry a
/// grandchild count, weigh less. These include hashes, versions, framing,
/// and amortized session setup; none is a fixed header size or a bound on
/// other workloads.
const DISPUTE_OVERHEAD_BYTES: usize = 45;

/// The reference message: a small, round encoded payload size.
const REFERENCE_RECORD_BYTES: usize = 100;

/// Wire cost of one reference-size message, checked by the wire calibration
/// suite.
const DISPUTE_WIRE_BYTES: usize = DISPUTE_OVERHEAD_BYTES + REFERENCE_RECORD_BYTES;

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

/// The most bytes one session's decode fans hold at once on the in-memory
/// backend.
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
/// writer into `out`, exactly as a session's multiplexer writes it.
///
/// The caller pre-reserves `out`, allowing the encoder allocation tests to
/// exclude growth of the destination buffer.
pub async fn write_prepared_frame(frame: &PreparedFrame, out: &mut Vec<u8>) {
    remote::write_prepared_frame(frame, out).await
}

/// Run `run` with each listed version's leaf placed at its chosen 32-byte
/// path instead of at the hash of the version.
///
/// This exists only to construct placements that hashing cannot reach:
/// two replicas that dispute a node one byte above the leaves need leaf
/// paths sharing 31 bytes, a 248-bit collision. Build, use, and drop every
/// tree inside `run`, and drive every peer on this thread, since the
/// placement is the calling thread's. A version missing from the placement
/// panics when its leaf's path is derived.
///
/// # Panics
///
/// Panics if two listed versions are equal or two listed paths are equal.
pub fn with_leaf_paths<R>(
    paths: impl IntoIterator<Item = (crate::Version, [u8; 32])>,
    run: impl FnOnce() -> R,
) -> R {
    crate::tree::typed::Path::with_leaf_paths(
        paths
            .into_iter()
            .map(|(version, path)| (version, crate::tree::typed::Path::from(path))),
        run,
    )
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
    // Sized for the deepest closed-world runs the suites drive, such as the
    // capacity stress fixtures over a one-byte pipe, so a legitimate long
    // session is not misreported as a runaway. Measure those runs before
    // lowering this.
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

/// The logical reply streams each direction of a session carries, one per
/// height its speaker answers at: also the most walk stages, reply encoders,
/// and reply decoders one side of a session runs.
pub const SESSION_STREAMS: usize = remote::STREAM_COUNT;

/// The most one side's transient bytes for `holding` may exceed its
/// ledger's charges for that place on the in-memory backend.
///
/// That is one stage, encoder, and decoder's scratch for the place per
/// logical stream, each computed from the types it covers
/// (`materialized::scratch` derives every term), and for the exploded
/// nodes, the copy the greeting makes of the root to list its fan, before
/// any unit is admitted to charge it: at most one node and a full fan's
/// heap slots. Each term counts requested bytes, as the allocation meter
/// does, so the allocator's rounding is not part of it.
pub fn ledger_scratch(holding: Holding) -> usize {
    let streams = SESSION_STREAMS
        * crate::tree::mirror::streaming::materialized::scratch::<
            untyped::Node,
            <Local as Backend>::Error,
        >(holding);
    let greeting = match holding {
        Holding::ExplodedNodes => untyped::MAX_EXPLOSION_BYTES,
        _ => 0,
    };
    streams + greeting
}

/// The most bytes the in-memory backend's hashing holds at once on one
/// thread ([`Holding::Hashing`]).
///
/// Computing a branch's hash allocates its preimage, a record per child,
/// and frees it before the hash returns; the node keeps only the hash.
/// Hashing never yields, so a thread holds the preimages of one hashing
/// descent at a time, which nests one preimage per branch whose hash it
/// computes along a path: at most one per path byte, since each branch
/// below another sits at least one byte deeper. Each preimage holds at
/// most a full fan's records beside a whole path's prefix.
pub const HASHING_SCRATCH_BYTES: usize = PATH_LEN * MAX_BRANCH_PREIMAGE_LEN;

/// The divisor `d` of the least share of its outgoing replies' allocations
/// that one side's ledger charges to them over a session.
///
/// That is, `charged ≥ allocated / d` for [`Holding::OutgoingReplies`],
/// wherever the deletion filter prunes no subtree whose leaves the peer
/// partly knows.
///
/// Every allocation a session marks for its outgoing replies is one of
/// two kinds. A chunk's reactions, and the listing each of its queries
/// carries, are counted exactly once, at their capacities, by the chunk's
/// charge. A supply answer's children, which it lists before it builds one
/// supply reaction per child, are not charged; a child, a radix beside a
/// node handle, is no larger than a reaction (asserted where the prices
/// are defined), so those children are at most the reactions charged for
/// them. The charges therefore cover at least half of what is allocated.
///
/// Where the filter prunes a subtree whose leaves the peer partly knows,
/// it lists every child but charges a reaction only for each that
/// survives, so no share holds there.
pub const OUTGOING_REPLIES_FLOOR_DIVISOR: usize = 2;

/// The wire bytes of the largest frame that carries one comparison rather
/// than supplied records: a query listing a full fan of children.
///
/// A session whose run target is at most this sends no frame larger, unless
/// one supplied record alone outgrows it.
pub const COMPARISON_FRAME_BYTES: usize = remote::codec::FULL_FAN_QUERY_FRAME_LEN;

/// The most bytes one side's assembly can hold at once on the in-memory
/// backend, whatever the replicas' sizes: the bound the session-budget suite
/// holds [`Holding::Assembly`] to.
///
/// The ledger charges none of assembly: the queues of fixed capacity around
/// it, and the in-memory backend's build of each supplied subtree, are
/// fixed state. Counted at their capacities:
///
/// - each assembled height, the root's through the leaves', returns at most
///   a fan of child handles to the height above, and builds at most one
///   parent at a time from at most a fan of children;
/// - the terminal leaf resolutions wait at most a fan deep, each with one
///   slot;
/// - each of the [`SESSION_STREAMS`] reply decoders assembles at most one
///   supplied subtree at a time, holding only its open right spine
///   ([`ASSEMBLY_SPINE_BYTES`]) however many leaves it has, and queues at
///   most a fan of assembled supplies for its reader.
///
/// Every queued item is counted at twice its size: the most a vector holds
/// for its length, or a channel node beside its item.
pub fn assembly_allowance() -> usize {
    use std::mem::size_of;
    let heights = Root::HEIGHT + 1;
    let fan = FAN;
    let returned = size_of::<Option<untyped::Node>>();
    let child = size_of::<(u8, Option<untyped::Node>)>();
    let terminal = size_of::<Resolution<<Local as Backend>::Erased>>()
        + size_of::<(u8, Resolve<<Local as Backend>::Erased>)>();
    let supplied = size_of::<(ErasedPrefix, untyped::Node)>();
    let levels = 2 * (heights * fan * (returned + child) + fan * terminal);
    let decoders = SESSION_STREAMS * (ASSEMBLY_SPINE_BYTES + 2 * fan * supplied);
    levels + decoders
}

/// The most transient bytes the in-memory backend holds while it assembles
/// one supplied subtree, whatever the subtree's size.
///
/// That is one open branch per path byte, each with a full fan's heap
/// slots, counted twice for the blocks a growing vector frees as it
/// reallocates.
pub const ASSEMBLY_SPINE_BYTES: usize = untyped::SortedLeaves::MAX_TRANSIENT_BYTES;

/// Leaves at version-derived paths, in path order, ready for an assembly
/// measurement: the in-memory leaves a reply decoder hands the backend's
/// assembler for one supplied subtree.
pub struct SuppliedLeaves(
    Vec<(
        crate::tree::typed::Prefix<crate::tree::typed::height::Z>,
        crate::tree::typed::Node<crate::tree::typed::height::Z>,
    )>,
);

/// `count` leaves, one per version a fresh party ticks, at their
/// version-derived paths, in path order.
pub fn supplied_leaves(count: usize) -> SuppliedLeaves {
    use crate::tree::typed::{Node, Path, Prefix};
    let party = before::Party::seed().fork();
    let mut version = crate::Version::new();
    let mut leaves: Vec<_> = (0..count)
        .map(|_| {
            version.tick(&party);
            let path = Prefix::containing(&Path::for_leaf(&version));
            (path, Node::leaf(version.clone(), unit_message()))
        })
        .collect();
    leaves.sort_unstable_by_key(|(path, _)| *path);
    SuppliedLeaves(leaves)
}

/// Assemble `leaves` into one root-height subtree through the in-memory
/// backend's assembler, the path a reply decoder's supplies take, and
/// return it, keeping the assembled nodes alive until the caller drops
/// them.
///
/// # Panics
///
/// Panics if `leaves` is empty: a supplied subtree holds at least one leaf.
pub fn assemble_supplied(leaves: SuppliedLeaves) -> AssembledSubtree {
    use crate::tree::typed::height::Z;
    use futures::StreamExt;
    let stream: crate::tree::mirror::streaming::BoxNodeStream<'static, Local, Z> =
        Box::pin(futures::stream::iter(leaves.0.into_iter().map(Ok)));
    let assembled = Local.assemble::<Root>(stream);
    let (_, root) = run_to_quiescence(async move { std::pin::pin!(assembled).next().await })
        .expect("the in-memory assembler never waits")
        .expect("a nonempty run assembles to one subtree")
        .unwrap_or_else(|never| match never {});
    AssembledSubtree(root)
}

/// A subtree [`assemble_supplied`] built, kept alive until it is dropped.
pub struct AssembledSubtree(crate::tree::typed::Node<Root>);

/// Summarize the subtree by its leaf count.
impl std::fmt::Debug for AssembledSubtree {
    /// Formats the subtree's leaf count.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssembledSubtree")
            .field("leaves", &self.leaves())
            .finish()
    }
}

/// Summarize the leaves by their count.
impl std::fmt::Debug for SuppliedLeaves {
    /// Formats the number of leaves.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SuppliedLeaves")
            .field("leaves", &self.0.len())
            .finish()
    }
}

impl AssembledSubtree {
    /// The subtree's leaf count.
    pub fn leaves(&self) -> usize {
        self.0.len()
    }
}

/// The floor allowance of a session over the in-memory replica: the most
/// its floor slots can hold at once, which its memory budget sets aside
/// for them, so its ledger admits work against the budget less this.
///
/// A budget at or below it admits work through the floor slots alone.
pub fn floor_allowance() -> usize {
    crate::tree::mirror::streaming::materialized::floor_allowance::<
        <Local as Backend>::Erased,
        <Local as Backend>::Error,
    >()
}

/// Unit tests for this support module.
#[cfg(test)]
mod tests;

/// The unit payload every [`supplied_leaves`] leaf carries.
fn unit_message() -> crate::message::Message {
    crate::message::Message::try_from_arc(
        Arc::new(()),
        crate::message::PayloadDepthLimit::default(),
    )
    .expect("the unit payload encodes within any depth limit")
}
