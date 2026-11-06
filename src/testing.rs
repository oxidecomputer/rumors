//! Executor-agnostic test support shared across protocol and API suites.

pub mod property;
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

use tokio::io::{AsyncRead, BufReader, DuplexStream};

use crate::{
    Snapshot,
    peer::MemoryBudget,
    tree::{
        mirror::streaming::{
            Backend, Local,
            ledger::Unit,
            materialized::{
                self, Allowances, fixed_allowance, floor_allowance, largest_grant,
                session_allowance,
            },
            remote::{self, RunBudget, codec::LeafRun},
        },
        typed::{
            hash::{HMAC_KEY_LEN, Hash, HashKey, MERKLE_HASH_LEN as TREE_HASH_LEN},
            height::{Height, Root, Z},
            untyped::{
                self, census,
                memo_census::{self, Work},
            },
        },
    },
};

pub use crate::tree::mirror::streaming::remote::codec::{
    CUTS_TO_GROW, DecodeError as CodecDecodeError, DecodeErrorKind as CodecDecodeErrorKind,
    DecodeLeafError, FramePart, HeadError, LeafRunError,
};

pub use crate::tree::mirror::streaming::holding::{Holding, Hook, attribute_allocations};
pub use crate::tree::mirror::streaming::ledger::{
    Admission, AdmissionHook, ChargeUnit, FitHook, LedgerChange, LedgerHook, PLACES, ReplyFit,
    SharePeak, observe_admissions, observe_fits, observe_ledgers,
};
pub use crate::tree::mirror::streaming::tasks::{Pacer, TaskRole, pace_tasks};
pub use crate::tree::mirror::streaming::{EdgeSend, EdgeSendHook, QueueKind, observe_edge_sends};

pub use transport::{
    AdversarialRead, AdversarialWrite, FaultUnit as IoFaultUnit, InjectedIo, IoFault, IoPlan,
    IoReport, IoReportHandle, Operation as IoOperation, Side as IoSide, wrap_io,
};

pub use crate::tree::mirror::streaming::remote::{
    FrameShape, HookCapture, HookStream, MultiplexerHook, MultiplexerPoll, PipeCapture,
    PreparedFrame, observe_multiplexers,
};

/// The floor allowance of a session over the in-memory replica: the most
/// its kinds and depths of unit can hold within their bounds at once
/// ([`grant_bound`]), which the budget never counts.
///
/// Its memory budget sets this aside for them beside the session allowance
/// ([`SESSION_ALLOWANCE`] at the default target), so its ledger admits
/// work against the budget less both.
///
/// A memory budget at or below the two together, which only
/// [`ledger_budget`] reaches, admits work within the bounds alone.
pub const FLOOR_ALLOWANCE: usize =
    floor_allowance::<<Local as Backend>::Erased, <Local as Backend>::Error>();

/// The fixed allowance of a session over the in-memory replica: the most
/// the state it holds outside its ledger and its wire pools whatever its
/// budget can hold.
///
/// It is the sum of the decode fans' read-ahead ([`DECODE_AHEAD_BYTES`]),
/// assembly's bound ([`assembly_allowance`]), hashing's
/// ([`HASHING_SCRATCH_BYTES`]), every place's working buffers
/// ([`ledger_scratch`]), and the pipe's fixed state, part of the wire's
/// bound ([`wire_allowance`]).
pub const FIXED_ALLOWANCE: usize = fixed_allowance::<Local>();

/// The session allowance of a session over the in-memory replica at the
/// default message-size target: the fixed allowance ([`FIXED_ALLOWANCE`])
/// beside the session's two wire pools at that target.
///
/// Its memory budget sets this aside beside the floor allowance
/// ([`FLOOR_ALLOWANCE`]).
pub const SESSION_ALLOWANCE: usize =
    session_allowance::<Local>(crate::default::TARGET_MESSAGE_SIZE);

/// The most bytes one side's wire buffering holds at once at the
/// message-size target `target`, a supply record larger than a pool
/// aside: its two wire pools beside the pipe's fixed state.
pub const fn wire_allowance(target: usize) -> usize {
    crate::tree::mirror::streaming::remote::fixed_wire_bytes().saturating_add(
        crate::tree::mirror::streaming::remote::wire_pools_bytes(target),
    )
}

/// The memory budget whose sessions at the default message-size target
/// admit work against exactly `bytes` in their ledgers.
///
/// That is `bytes` beside the floor allowance ([`FLOOR_ALLOWANCE`]) and
/// the session allowance ([`SESSION_ALLOWANCE`]), saturating at
/// `usize::MAX`. A session at another target sets aside another session
/// allowance.
pub const fn memory_budget_admitting(bytes: usize) -> usize {
    Allowances::of::<Local>(crate::default::TARGET_MESSAGE_SIZE)
        .ledger(bytes)
        .memory
}

/// The budget the ledgers of sessions at the default message-size target
/// on the memory budget `budget` admit work against.
///
/// That is what `budget` leaves beside the floor allowance
/// ([`FLOOR_ALLOWANCE`]) and the session allowance
/// ([`SESSION_ALLOWANCE`]), or zero. Inverse to
/// [`memory_budget_admitting`] wherever that does not saturate.
pub const fn ledger_budget_within(budget: usize) -> usize {
    Allowances::of::<Local>(crate::default::TARGET_MESSAGE_SIZE).left(budget)
}

/// Set the budget `peer`'s sessions' ledgers admit work against to exactly
/// `bytes`, at whatever message-size target the peer runs at.
///
/// Its sessions run on that budget beside the floor allowance and the
/// session allowance at the peer's target.
///
/// Unlike a budget set through
/// [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget), which
/// lowers a target it cannot afford and which a session raises to the
/// minimum for its target
/// ([`MIN_SYNC_MEMORY_BUDGET`](crate::MIN_SYNC_MEMORY_BUDGET) at the
/// default target), this one sessions run on as given, at the peer's
/// target as given. So it reaches ledger budgets below the largest grant,
/// zero included, where sessions pipeline within each kind and depth's
/// bound alone, and the starved regime the budget analyses probe. The
/// serial configuration, one unit of each kind and depth at a time, is
/// [`serial_budget`]'s.
pub fn ledger_budget<T: Send + Sync + 'static, B: crate::Bookmark>(
    mut peer: crate::Peer<T, B>,
    bytes: usize,
) -> crate::Peer<T, B> {
    peer.sync_memory_budget = MemoryBudget::Ledger(bytes);
    peer
}

/// Run `peer`'s sessions' ledgers in the serial configuration: a budget of
/// zero with every kind and depth's bound zero.
///
/// Admission then grants a unit only while its kind and depth hold
/// nothing, so each walk has at most one unit of each kind and depth
/// admitted at a time: the configuration the liveness argument rests on at
/// its most serial. A ledger budget of zero alone
/// ([`ledger_budget`]) still lets each kind and depth hold up to its bound
/// ([`grant_bound`]).
pub fn serial_budget<T: Send + Sync + 'static, B: crate::Bookmark>(
    mut peer: crate::Peer<T, B>,
) -> crate::Peer<T, B> {
    peer.sync_memory_budget = MemoryBudget::Serial;
    peer
}

/// The largest grant any unit of work can take in a session over the
/// in-memory replica: a bound on every charge its ledger holds.
///
/// It is a type maximum, computed from sizes and capacities, and a session's
/// own grants are usually far smaller.
pub const LARGEST_GRANT: usize =
    largest_grant::<<Local as Backend>::Erased, <Local as Backend>::Error>();

/// The most the grant of a unit of `unit`'s kind and depth can hold in a
/// session over the in-memory replica: that kind and depth's bound, within
/// which the bytes it holds never count against the budget.
///
/// The floor allowance ([`FLOOR_ALLOWANCE`]) is the larger of two sums of
/// these bounds, one per side of a session: the initiator asks about scopes
/// at even depths and answers at odd ones, and the responder the reverse.
/// What a ledger's kinds and depths hold within their bounds stays within
/// it while every grant is within its bound and the ledger admits only one
/// side's kinds and depths.
///
/// # Panics
///
/// Panics if `unit.depth` exceeds 32, the depth of a leaf.
pub const fn grant_bound(unit: ChargeUnit) -> usize {
    assert!(
        unit.depth <= Root::HEIGHT,
        "a scope sits at most 32 bytes deep"
    );
    let kind = if unit.question {
        Unit::Question
    } else {
        Unit::Answer
    };
    materialized::grant_bound::<<Local as Backend>::Erased, <Local as Backend>::Error>(
        kind, unit.depth,
    )
}

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
    /// The read half, buffered as a session requires: the peer's write half
    /// delivers here.
    pub read: BufReader<DuplexStream>,
    /// The write half: it delivers to the peer's read half.
    pub write: DuplexStream,
}

/// Create the two ends of one in-memory connection, each direction a
/// [`tokio::io::duplex`] pipe buffering `capacity` bytes, read through a
/// [`BufReader`].
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
            read: BufReader::new(a_read),
            write: a_write,
        },
        MemoryEnd {
            read: BufReader::new(b_read),
            write: b_write,
        },
    )
}

/// Width of the subtree digests consumed by [`BranchHasher`].
pub const MERKLE_HASH_LEN: usize = TREE_HASH_LEN;

/// Bytes of the raw HMAC key a [`BranchHasher`] takes: one SHA-256 output.
pub const HASH_KEY_LEN: usize = HMAC_KEY_LEN;

/// The tree hash under a raw key, for hashing branches through the
/// production preimage builder ([`branch_hash`](Self::branch_hash)).
///
/// The branch-hashing benchmark uses this test-only entry so it cannot drift
/// from the implementation whose feeding strategy it measures. As in the
/// tree, constructing one keys the hash once, and each branch then clones
/// the keyed state.
#[derive(Clone, Debug)]
pub struct BranchHasher(HashKey);

impl BranchHasher {
    /// Key the tree hash with the raw key `key`.
    pub fn new(key: &[u8; HASH_KEY_LEN]) -> Self {
        Self(HashKey::new(key))
    }

    /// Hash the branch at `height` with compressed prefix `prefix` over
    /// `children`, given in ascending radix order with their digests.
    pub fn branch_hash(
        &self,
        height: usize,
        prefix: &[u8],
        children: impl IntoIterator<Item = (u8, [u8; MERKLE_HASH_LEN])>,
    ) -> [u8; MERKLE_HASH_LEN] {
        self.0
            .branch(
                height,
                prefix,
                children
                    .into_iter()
                    .map(|(radix, hash)| (radix, Hash::from(hash))),
            )
            .into()
    }
}

/// The 32-byte path at which `snapshot`'s network places a message stamped
/// with `version`: the version's hash keyed by the network's secret, the
/// address the replica's own tree uses.
///
/// Fixtures that stage a tree shape search versions by this path. It is the
/// only way a test outside the crate can compute a path, since the secret
/// never leaves the crate.
pub fn leaf_path<T: Send + Sync + 'static>(
    snapshot: &Snapshot<T>,
    version: &crate::Version,
) -> [u8; 32] {
    crate::tree::typed::Path::for_leaf(snapshot.tree().path_key(), version).into()
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
    Snapshot::new(tree)
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

/// The work tree nodes did building branches and computing their memos,
/// crate-wide, since the last [`memo_work_reset`].
///
/// The counts are process-global, like the [`node_census`]; tests that
/// assert on them must own the process.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemoWork {
    /// Bare branches assembled from a fan of children, as reassembly
    /// assembles them. The subtrees a session receives are built leaf by
    /// leaf and not counted.
    pub branches: u64,
    /// Nodes reassembly returned whole because their children came back
    /// unchanged.
    pub reused: u64,
    /// Hashes computed rather than read from their memos.
    pub hashes: u64,
    /// Branch bounds computed from all of the branch's children.
    pub bounds_full: u64,
    /// Rebuilt branches whose bounds were seeded whole from the node they
    /// replace.
    pub bounds_seeded: u64,
    /// Rebuilt branches whose bounds were seeded at one endpoint, the other
    /// folded from all of the branch's children.
    pub bounds_half: u64,
    /// Rebuilt branches seeded, whole or at one endpoint, with the help of
    /// an unchanged child's bound.
    pub bounds_by_sibling: u64,
    /// Rebuilt branches whose bounds could not be seeded exactly, left for
    /// warming to compute in full.
    pub bounds_declined: u64,
    /// Encoded version sizes computed from all of a branch's children.
    pub version_bytes_full: u64,
}

/// Read the work tree nodes did since the last [`memo_work_reset`].
pub fn memo_work() -> MemoWork {
    MemoWork {
        branches: memo_census::read(Work::Branch),
        reused: memo_census::read(Work::Reused),
        hashes: memo_census::read(Work::Hash),
        bounds_full: memo_census::read(Work::BoundsFull),
        bounds_seeded: memo_census::read(Work::BoundsSeeded),
        bounds_half: memo_census::read(Work::BoundsHalf),
        bounds_by_sibling: memo_census::read(Work::BoundsBySibling),
        bounds_declined: memo_census::read(Work::BoundsDeclined),
        version_bytes_full: memo_census::read(Work::VersionBytesFull),
    }
}

/// Restart every count [`memo_work`] reads from zero.
pub fn memo_work_reset() {
    memo_census::reset();
}

/// Approximate wire overhead per differing message, excluding its encoded payload.
///
/// `tests/dispute_wire.rs` counts all protocol writes in both directions,
/// divides by the differing messages, and subtracts their encoded payloads.
/// The reference fixture has 2,048 shared messages and 8,192 new ones per
/// side. Its whole-byte mean gives 38; the unrounded mean is about 38.8.
/// Two fully divergent 75,000-message sets instead average about 36.8,
/// since supplies dominate them and listings, whose entries each carry a
/// grandchild count, weigh less. These include hashes, versions, framing,
/// and amortized session setup; none is a fixed header size or a bound on
/// other workloads.
const DISPUTE_OVERHEAD_BYTES: usize = 38;

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
/// backend: a full fan of decoded leaves and one more per logical stream a
/// session decodes, each counted at twice its size for its channel node.
pub const DECODE_AHEAD_BYTES: usize = remote::decode_ahead_bytes::<<Local as Backend>::Node<Z>>();

/// Read exactly `declared` payload bytes under the framing layer's
/// growth policy, returning the payload.
///
/// This is the declared-length payload read used for party hand-offs and
/// the greeting, without the surrounding session. The decoder
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
        run.push(&version, &message, RunBudget::default())
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
/// path instead of at the version's keyed hash.
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

/// The logical reply streams each direction of a session carries, one per
/// height its speaker answers at: also the most walk stages, reply encoders,
/// and reply decoders one side of a session runs.
pub const SESSION_STREAMS: usize = remote::STREAM_COUNT;

/// The frames each logical stream's channel holds between the pipe and a
/// stage's encoder or decoder, in either direction.
pub const LANE_CAPACITY: usize = remote::LANE_CAPACITY;

/// The most one side's transient bytes for `holding` may exceed its
/// ledger's charges for that place on the in-memory backend.
///
/// That is one stage, encoder, and decoder's scratch for the place per
/// logical stream, each computed from the types it covers
/// (`materialized::scratch` derives every term). Each term counts requested
/// bytes, as the allocation meter does, so the allocator's rounding is not
/// part of it. Summed over every place, these are the working buffers the
/// fixed allowance sets aside ([`FIXED_ALLOWANCE`]).
pub const fn ledger_scratch(holding: Holding) -> usize {
    SESSION_STREAMS
        * crate::tree::mirror::streaming::materialized::scratch::<
            <Local as Backend>::Erased,
            <Local as Backend>::Error,
        >(holding)
}

/// The most bytes the in-memory backend's hashing holds at once on one
/// thread ([`Holding::Hashing`]).
///
/// That is the preimages of one hashing descent, at most one per path
/// byte, each at most a full fan's records beside a whole path's prefix.
pub const HASHING_SCRATCH_BYTES: usize = crate::tree::typed::hash::HASHING_SCRATCH_BYTES;

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
/// backend, whatever the replicas' sizes.
///
/// It is the bound the session-budget suite holds [`Holding::Assembly`]
/// to, and a term of the fixed allowance ([`FIXED_ALLOWANCE`]).
///
/// The charge leaves assembly out: its queues have fixed capacities, and
/// the in-memory backend builds each supplied subtree over its open right
/// spine ([`ASSEMBLY_SPINE_BYTES`]) however many leaves it has, at most one
/// per reply decoder, of the [`SESSION_STREAMS`], at a time.
pub const fn assembly_allowance() -> usize {
    crate::tree::mirror::streaming::materialized::assembly_allowance::<Local>()
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
    use crate::tree::typed::{Node, Path, PathKey, Prefix};
    let key = PathKey::fixture();
    let party = before::Party::seed().fork();
    let mut version = crate::Version::new();
    let mut leaves: Vec<_> = (0..count)
        .map(|_| {
            version.tick(&party);
            let path = Prefix::containing(&Path::for_leaf(&key, &version));
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
        Box::pin(futures::stream::iter(leaves.0.into_iter().map(
            |(path, leaf)| Ok((path, crate::tree::typed::View::from(leaf))),
        )));
    let assembled = Local.assemble::<Root>(stream);
    let (_, root) = run_to_quiescence(async move { std::pin::pin!(assembled).next().await })
        .expect("the in-memory assembler never waits")
        .expect("a nonempty run assembles to one subtree")
        .unwrap_or_else(|never| match never {});
    AssembledSubtree(root.into_node())
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
