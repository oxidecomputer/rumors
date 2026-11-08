//! The session memory budget against the transient heap a session allocates,
//! under the most adverse link and data this suite constructs.
//!
//! [`Peer::sync_memory_budget`](rumors::Peer::sync_memory_budget) caps the
//! memory one session uses beyond the replica. This suite measures that claim
//! directly: with budget `B`, the heap a session allocates *transiently*,
//! meaning allocations made during the session that do not survive into the
//! replica, stays near `B`. The replica's own growth, the messages the session
//! supplies, is outside the claim.
//!
//! # The measurement
//!
//! Each test drives two peers through one session on this thread, over the held
//! link of the [`link`] module: every byte written waits until both peers
//! stall, so each side fills whatever its admission lets it have outstanding
//! before any reply arrives, and each release hands the reader everything its
//! peer produced meanwhile. At each stall the schedule can also hold back kinds
//! of either side's tasks, as a slow storage backend would: walk stages, so
//! replies park while the questions they answer stay queued, or assemblers, so
//! resolutions pile up. An [`alloc_meter::trace`] records every allocation and
//! free over the session, and the driver labels each allocation with the peer
//! whose poll made it.
//!
//! After the session's state is dropped, the trace classifies every block the
//! session allocated. A block still live is replica content. A block freed was
//! transient. Each peer's *transient peak* is the highest number of its
//! transient bytes live at once, which is the quantity the claim bounds. A
//! block allocated before the session is neither, even when the session frees
//! it: the replica's old nodes, which the session holds through its starting
//! root until it publishes the merged tree, were allocated at rest and count
//! toward the replica at rest. The link's buffers and the driver's own state
//! are allocated outside the trace, so they count toward nothing, and every
//! block the trace records is some peer's.
//!
//! Three checks hold the survivors to being replica content. The peers are
//! dropped while a snapshot of each replica keeps it alive, and then the
//! snapshots: every survivor must be freed by one of the two, so none belongs
//! to a leak or a global cache that would hide transient bytes. A survivor the
//! peer frees while its replica lives is session state the peer kept, which may
//! be no more than a session that compares almost nothing leaves (see [the
//! reference](#the-reference-session)). The survivors the replica keeps may be
//! no more than building the merged replica directly allocates: the peer's
//! messages as its own sends built them, and the interior a join of the two
//! replicas builds. And each peer's survivors must number at least the messages
//! it gained, since every gained message becomes at least one new block of its
//! replica, so a trace that stopped seeing the session cannot pass.
//!
//! # The data
//!
//! The budget binds when two large replicas differ throughout. A peer catching
//! up from nothing disputes no subtree: it receives the set as supplies, and
//! with nothing disputed the budget barely binds the session, so each fixture
//! is two forks of one network holding equal, disjoint message sets.
//!
//! A leaf's tree path is its version's hash keyed by the network's secret, so
//! the suite cannot place leaves: two hashed sets of `n` messages each
//! interleave as uniformly random keys do. At 100,000 messages per side, every
//! depth-1 node and most depth-2 nodes hold children from both replicas, and
//! deeper nodes rarely do. Placing leaves so that deeper nodes also hold both
//! would mean searching versions for chosen hash prefixes, at a cost of `256^k`
//! versions per shared byte `k`, and would leave the uniform-hash model, under
//! which the crate's guarantees are stated. Equal, fully divergent hashed sets
//! are the most adverse fixture within that model.
//!
//! One fixture has side `A` redact messages both sides received before they
//! diverged ([`redacting_replicas_keep_the_bound`]), so `B`'s supplies pass
//! through the deletion filter over subtrees that mix leaves `A` lacks with
//! leaves `A`'s version knows.
//!
//! Both peers set their run target to [`COMPARISON_FRAME_BYTES`], the largest
//! frame that carries one comparison, so no frame of the session is larger,
//! unless one supplied record alone outgrows it, and the wire pools below hold
//! a couple of such frames each way.
//!
//! One test instead sends byte vectors larger than serde's preallocation cap at
//! the default target ([`large_byte_vectors_keep_the_bound`]), so that decoding
//! a payload outgrows a buffer and runs travel whole, within the run budget and
//! past it, beside the pools.
//!
//! # Attribution
//!
//! The session marks the code that allocates for each place it keeps state in
//! ([`Holding`]), and this suite labels each block with the peer and the place
//! its allocation served, so every place's peak sits beside the bound that
//! covers it. An allocation counts toward the place whose code made it: a
//! question's child listing where the walk explodes the node it asks about, a
//! parked reply where the decoder assembles it. The listing allocates only the
//! vector that holds the children's handles, since the backend shares each
//! child rather than copying it. Places the ledger leaves out, such as wire
//! buffering and assembly, are labelled too. Blocks no marked region allocated,
//! the session's tasks and setup, are *unattributed*.
//!
//! Each label's peak is its own, so the labels' peaks need not occur together;
//! each side's transient peak is the peak of all its labels at once.
//!
//! # The bounds
//!
//! Every label has a bound, and each side's whole transient peak must stay
//! within their sum: every byte the session holds at once is some label's, and
//! no label holds more than its own peak.
//!
//! The places the ledger charges ([`CHARGED`]) are bounded together, by the
//! ledger: at every allocation event their live bytes may exceed what the
//! ledger holds by at most the scratch its stages fill before they charge what
//! they built, and the ledger holds at most its budget beside what its kinds
//! and depths of unit hold within their bounds ([the ledger](#the-ledger)).
//!
//! The scratch is what a stage, encoder, or decoder fills for a place while it
//! builds items it charges only once built: an answer's plan, a chunk's
//! reactions before its question is admitted, a decoder's ready reactions, the
//! scopes derived from a chunk's questions as its stage takes it. Each place is
//! owed its own scratch for each of the
//! [`SESSION_STREAMS`](rumors::testing::SESSION_STREAMS) logical streams,
//! computed from the types it covers ([`ledger_scratch`]).
//!
//! The classes the ledger leaves out, the session's fixed state, are bounded by
//! their capacities:
//!
//! - **Wire buffering** holds at most the session's two wire pools at its run
//!   target, which bound every supply run on its way through the pipe, beside
//!   the pipe's fixed state ([`wire_allowance`] derives the bound from the
//!   production definition). Every frame that is not a run is charged to the
//!   place its chunk or its question's room belongs to.
//! - **Assembly** holds the queues of fixed capacity around it and the
//!   in-memory backend's build of each supplied subtree, which keeps only the
//!   subtree's open right spine however large it is ([`assembly_allowance`]
//!   derives the bound from both, independent of the replicas' sizes).
//! - **Decode fans** hold at most a fan of decoded leaves for each logical
//!   stream a side receives on.
//! - **Hashing** holds the preimages the in-memory backend builds to compute
//!   nodes' hashes. Listing or comparing a node computes its hash on first use,
//!   and a peeled view, which keeps no hash, computes its own on each read. A
//!   preimage lives only within the call that computes its hash, and a node
//!   keeps only the hash, inside its own allocation. The driver polls every
//!   task on one thread and hashing never yields, so a side holds the preimages
//!   of at most one hashing descent at once, at most one per path byte
//!   ([`HASHING_SCRATCH_BYTES`] derives the bound, independent of the budget
//!   and the replicas' sizes).
//! - **The unattributed blocks** are the session's fixed overhead: the tasks,
//!   channels, and buffers it sets up for every logical stream and tree level,
//!   and its greeting's root fan, independent of the budget and of the sets'
//!   sizes once the root's fan is full. Admission's own bookkeeping
//!   ([`Holding::Admission`]) is fixed overhead too. Together they may be no
//!   more than [the reference session](#the-reference-session)'s whole
//!   transient peak.
//!
//!   That bound is a measurement rather than a figure derived from types,
//!   because two of admission's terms belong to the platform and the executor,
//!   not to the crate: the lock inside each waiting unit, which the standard
//!   library boxes on some platforms (illumos among them), and the clone of its
//!   task's waker, which allocates under some executors. Its meaning is
//!   understood. A unit admitted as it asks allocates nothing, and what each
//!   kind and depth holds is a count in the ledger's own account, so admission
//!   allocates only for units that wait: each a waiter, its lock, its waker's
//!   clone, and a place in the queue's buffer. Each walk stage waits on one
//!   unit at a time, so at most one unit per stage, of the
//!   [`SESSION_STREAMS`](rumors::testing::SESSION_STREAMS), waits at once,
//!   whatever the budget and the sets' sizes: a few hundred bytes a side,
//!   measured. The reference session sets up the same streams as the session
//!   under test and its own work beside them, so its peak exceeds that setup by
//!   far more than admission ever holds.
//!
//! # The ledger
//!
//! A peer's memory budget sets aside the floor allowance, the most its ledger's
//! kinds and depths of unit can hold within their bounds at once, which the
//! budget never counts, and the session allowance for the session's fixed state
//! and wire pools: the bounds above on the decode fans, assembly, hashing, and
//! the wire, and every place's scratch. The ledger admits work beyond both
//! against the rest. Every budget this suite names is that rest, what the
//! ledgers admit against: each peer's memory budget is the named budget beside
//! the two allowances at its run target
//! ([`ledger_budget`](rumors::testing::ledger_budget)), so that the suite's
//! fixtures admit work as their names say. Some of those budgets lie below the
//! largest grant, which the public setter's minimum
//! ([`MIN_SYNC_MEMORY_BUDGET`](rumors::MIN_SYNC_MEMORY_BUDGET)) keeps every
//! application's session above, so the suite sets each ledger budget directly
//! ([`ledger_budget`](rumors::testing::ledger_budget)).
//!
//! Each side keeps a ledger of what its session holds and of the room it sets
//! aside for the replies its questions invite, and admits its walk's work
//! against its budget. The suite holds every session it measures to three
//! identities:
//!
//! - **Coverage.** At every allocation event, and for each place it charges
//!   ([`CHARGED`]), the ledger's charges for the place are at least the place's
//!   live transient bytes, less the place's own scratch ([`ledger_scratch`]),
//!   and so in total too. The trace's position at each ledger change lines the
//!   two up, event by event; checking each place alone keeps room set aside in
//!   one place from hiding a charge missing in another.
//! - **Fit.** Every reply a stage finishes held no more reactions, and its
//!   queries no more listing entries, than its question reserved room for, and
//!   the records derived from its questions held no more than the reservation's
//!   records part, which pays for nothing else. Coverage checks that part's
//!   place together with the question records' own charges, and does not see
//!   the part missing at these fixtures' sizes; this check does.
//! - **Zero.** Each side's ledger holds nothing once the session ends.
//! - **Outgoing floor.** Over the session, each side's charges to its outgoing
//!   replies, the sum of every rise in that place's account, are at least half
//!   the bytes it allocated for them ([`OUTGOING_REPLIES_FLOOR_DIVISOR`]
//!   derives the share). Coverage alone cannot see a missing charge there: a
//!   chunk's charge returns once the multiplexer has written the chunk's last
//!   frame, which carries it, so its items never outlive it by more than the
//!   place's scratch. Computing a node's hash while an answer lists or compares
//!   it is hashing's work, not the outgoing replies' (above), so the placed
//!   fixture's half fans, each hashed anew on its peeled view, stay out of the
//!   share. The floor's premise fails where the deletion filter prunes: a
//!   supply answer can list a fan of children of which one survives to be
//!   charged as a reaction, so no share is derived, and the floor is not
//!   asserted for the redacting fixture.
//!
//! It also holds each ledger to its bound, as two checks: its excess, what its
//! kinds and depths hold beyond their bounds, at most the budget, and what they
//! hold within their bounds at most the floor allowance, so at most the peer's
//! memory budget, less the session allowance, in all; and every item built
//! within its unit's grant. Each ledger must also have counted, some reply must
//! have finished, and some unit must have waited for admission, so that none of
//! the checks passes vacuously. The ledgers' peaks in the fixed fixtures are
//! committed as baselines: what each side held and set aside at once. The
//! ledger peaks agree between macOS and illumos, but the heap peaks do not, so
//! no heap peak is committed. Instead, a held-back fixture shows that it raised
//! a heap peak by running the same session with nothing held back, in the same
//! test, and comparing the two.
//!
//! # The reference session
//!
//! Before its fixture, each test measures a session between two disjoint
//! replicas of [`REFERENCE_MESSAGES`] messages each in the serial
//! configuration, a ledger budget of zero with every kind and depth's bound
//! zero. Its root's fan is full and it sets up every logical stream, so its
//! setup is the setup of any larger session, and its transient peak, that setup
//! and its work of one unit of each kind and depth at a time together, bounds
//! the setup of the session under test. A ledger budget of zero beside the
//! walk's bounds would pipeline within them instead, and its larger peak would
//! loosen that bound by what its work held. Its peers also show what a session
//! leaves in the peer beyond the replica.
//!
//! # Upper bounds only
//!
//! The suite asserts no lower bound on what a session holds: replicas that
//! agree from the start stop at the root and hold almost nothing, whatever the
//! budget, so no floor proportional to `B` would mean anything.
//!
//! An upper bound means something only where the budget constrains the session;
//! on a session it never constrains, every bound would pass without judging the
//! budget. So each test also checks that it does: some unit of the session's
//! work waited for admission. Each side's session report must count exactly the
//! units its ledger counted waiting ([`SessionStats::budget_waits`]), the
//! user's view of the same constraint.
//!
//! # Network seeds
//!
//! A committed figure holds for one tree shape, and so for one network secret:
//! the ledger peaks, and the reference session's peak, which bounds every other
//! session's setup. The tests that commit figures, the held-back fixtures built
//! on the same sessions, and the reference session fork the network
//! [`BUDGET_NETWORK_SEED`] names, and also check every bound there. Every other
//! test states a claim for every network, so it draws its network's seed as a
//! property input: the schedule search, the redacting and placed fixtures, and
//! the hop sweep.
//!
//! # Hops against the budget
//!
//! A larger budget must never make a session slower. One test sweeps one
//! fixture across budgets on the held link, releasing every byte at every
//! stall, where each release is one hop and the count is exact, independent of
//! time and load ([`hops_never_rise_with_the_budget`]). The sweep needs no
//! trace, so it runs untraced, at the schedule search's size, and its sessions
//! run at once, each driven on a thread of its own.

#[path = "session_budget/link.rs"]
mod link;

use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::OnceLock;
use std::thread::{self, ScopedJoinHandle};

use alloc_meter::{
    Event, EventKind, Label, LabelLifetimes, Stats, labelled, measure, trace, untraced,
};
use link::{Drive, Held, Link, Portion, Release, Side, drive, polling, prepare_pacing};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};
use rumors::testing::{
    Admission, COMPARISON_FRAME_BYTES, DECODE_AHEAD_BYTES, FLOOR_ALLOWANCE, HASHING_SCRATCH_BYTES,
    Holding, LedgerChange, OUTGOING_REPLIES_FLOOR_DIVISOR, PLACES, ReplyFit, assembly_allowance,
    attribute_allocations, budgeted_message_bytes, join_snapshots, ledger_scratch,
    observe_admissions, observe_fits, observe_ledgers, wire_allowance, with_leaf_paths,
};
use rumors::{Peer, Rumors, SessionStats, Version};
use rumors_testkit::common::budget::BudgetChoice;
use rumors_testkit::common::property::divided_cases;
use rumors_testkit::common::seed::{NetworkSeed, network_seed};
use rumors_testkit::common::wire::{block_on, bootstrap_fork_then};
use rumors_testkit::divided_proptest;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::BufReader;

/// Messages each side of the per-budget fixtures holds that the other lacks:
/// large enough that units still wait for admission at [`WIDE_BUDGET`].
const MESSAGES: u64 = 100_000;

/// Messages per side for the schedule search, small enough that each case costs
/// a few seconds.
const SEARCH_MESSAGES: u64 = 10_000;

/// The schedule search's budget, 2 MiB, at which its sessions' units wait for
/// admission, so their bounds are judged where the budget constrains them.
const SEARCH_BUDGET: usize = 2 << 20;

/// The wide fixtures' budget, 32 MiB, four times the near-floor fixture's, at
/// which units still wait for admission at [`MESSAGES`] per side, so the queued
/// work it admits, rather than the fixed work near the root, dominates.
const WIDE_BUDGET: usize = 32 << 20;

/// Messages each side of [the reference session](self#the-reference-session)
/// holds: enough that its root has a full fan of children.
const REFERENCE_MESSAGES: u64 = SEARCH_MESSAGES;

/// Trace events reserved per message the session moves, in both directions
/// together, beside [`SETUP_EVENTS`].
///
/// A session makes fewer than a hundred allocator events per moved message at
/// these sizes, and one that outgrows the reservation fails with the count it
/// needed. Systems that do not overcommit memory, such as illumos, commit a
/// reservation up front, so it is sized near what a session uses.
const EVENTS_PER_MESSAGE: usize = 128;

/// Trace events reserved for a session's setup, whatever its size.
const SETUP_EVENTS: usize = 1 << 16;

/// What one fixture's construction allocated, and the peers it built, whose
/// messages are `T`.
struct Fixture<T: Send + Sync + 'static> {
    /// The two peers, `A` then `B`.
    peers: [Rumors<T>; 2],
    /// What each side's sends allocated, in [`Side::BOTH`] order: building its
    /// own messages into its replica.
    sends: [Stats; 2],
    /// What joining the two replicas in memory allocated beyond them: the
    /// merged tree's new interior.
    join: Stats,
    /// Every allocation the fixture made, an upper bound on the blocks either
    /// replica holds before the session.
    allocations: usize,
}

/// The first value of side `B`'s messages; side `A`'s start at zero.
const B_VALUES: u64 = 1_000_000;

/// The seed of the network every fixture with committed figures forks, and the
/// reference session's (see [the module docs](self#network-seeds)).
const BUDGET_NETWORK_SEED: NetworkSeed = NetworkSeed::new(0xb0d9_e75e_5510_0001);

/// Builds two forks of the network `network` names, holding `messages`
/// messages, in [`Side::BOTH`] order, that the other lacks.
///
/// Each fork runs on the suite's run target, with the ledger `ledger` chooses,
/// set through the test path: a ledger budget beside the walk's bounds
/// ([`BudgetChoice::Ledger`]), which reaches below the public minimum, or the
/// serial configuration ([`BudgetChoice::Serial`]).
///
/// The network first holds `redacted` messages both forks receive, which side
/// `A` then redacts, so `B`'s subtrees mix leaves `A` lacks with leaves `A`'s
/// version knows and `B` must not resend.
///
/// `A`'s messages are the values from 0 and `B`'s from [`B_VALUES`], so a
/// message's value names its side and its place among that side's sends. The
/// seed makes the network reproducible, and with it every message's version,
/// since forking and sending are deterministic.
fn fixture(
    network: NetworkSeed,
    messages: [u64; 2],
    redacted: u64,
    ledger: BudgetChoice,
) -> Fixture<u64> {
    let configure = |peer: Peer<u64>| {
        ledger
            .apply(peer)
            .target_message_size(COMPARISON_FRAME_BYTES)
    };
    let (built, [a, b]) = measure(|| {
        block_on(async {
            let seed = network.peer::<u64>().into_rumors();
            seed.send_all((0..redacted).map(|value| 2_000_000 + value))
                .expect("the shared messages encode");
            [
                bootstrap_fork_then(&seed, Peer::bootstrap(), configure).await,
                bootstrap_fork_then(&seed, Peer::bootstrap(), configure).await,
            ]
        })
    });
    let (redactions, ()) = measure(|| {
        let shared: Vec<_> = a.snapshot().versions().cloned().collect();
        a.redact_all(&shared);
    });
    let [a_messages, b_messages] = messages;
    let (a_sends, ()) = measure(|| a.send_all(0..a_messages).expect("A's messages encode"));
    let (b_sends, ()) = measure(|| {
        b.send_all((0..b_messages).map(|value| B_VALUES + value))
            .expect("B's messages encode")
    });
    let (join, joined) = measure(|| join_snapshots(&a.snapshot(), &b.snapshot()));
    drop(joined);
    let allocations = [built, redactions, a_sends, b_sends, join]
        .iter()
        .map(|stats| stats.allocations + stats.reallocations)
        .sum();
    Fixture {
        peers: [a, b],
        sends: [a_sends, b_sends],
        join,
        allocations,
    }
}

/// The survivors of one session, split by what frees them.
#[derive(Clone, Copy, Debug, Default)]
struct Kept {
    /// Bytes freed when the replica's last snapshot is dropped: replica
    /// content.
    replica_bytes: usize,
    /// Blocks freed when the peer is dropped while a snapshot keeps its
    /// replica: state the peer kept beyond the replica.
    peer_blocks: usize,
    /// The largest of those blocks.
    largest_peer_block: usize,
}

/// One session run and classified, before it is judged.
struct Run {
    /// Each side's session report, in [`Side::BOTH`] order.
    stats: [SessionStats; 2],
    /// Each side's classified blocks, all labels together.
    sides: [LabelLifetimes; 2],
    /// Each side's classified blocks by holding, in [`Holding::ALL`] order.
    holdings: [[LabelLifetimes; Holding::ALL.len()]; 2],
    /// Each side's blocks allocated outside any holding's region.
    unattributed: [LabelLifetimes; 2],
    /// The blocks allocated under no side's label.
    unlabelled: LabelLifetimes,
    /// Each side's survivors, split by what frees them.
    kept: [Kept; 2],
    /// The fixture the session ran between.
    sends: [Stats; 2],
    /// What joining the fixture's two replicas allocated.
    join: Stats,
    /// What the driver did.
    drive: Drive,
    /// Each side's ledger, held against the places it charges.
    ledgers: [LedgerReport; 2],
    /// The replies the session's stages finished, and those that held more than
    /// their questions reserved for.
    fits: Fits,
}

/// One side's ledger over a session, against the transient heap of the places
/// it charges ([`CHARGED`]) and against its budget.
#[derive(Clone, Copy, Debug, Default)]
struct LedgerReport {
    /// The ledger changes the side made.
    changes: usize,
    /// For each charged place, in [`CHARGED`] order, the bytes charged to it
    /// over the session: the sum of every rise in its account.
    charged: [usize; CHARGED.len()],
    /// The most the ledger held at once.
    peak: usize,
    /// The ledger's largest excess, what its kinds and depths held beyond their
    /// bounds at once: what its budget bounds.
    excess_peak: usize,
    /// The most the ledger's kinds and depths held within their bounds at once,
    /// what it held less its excess: what the floor allowance bounds.
    within_peak: usize,
    /// What the ledger held when the session ended.
    left: usize,
    /// Bytes the side's items needed beyond their grants.
    short: u64,
    /// Units that waited for admission.
    waited: u64,
    /// The most the charged places' live transient bytes exceeded the ledger,
    /// at any allocation event.
    uncovered: usize,
    /// For each charged place, in [`CHARGED`] order, the most its live
    /// transient bytes exceeded the ledger's charges for it.
    uncovered_by_place: [usize; CHARGED.len()],
    /// For each charged place, the most its live transient bytes reached as a
    /// share of the ledger's charges for it, in thousandths, where those
    /// charges were at least a page: how tight the place's prices run.
    tightest: [usize; CHARGED.len()],
}

/// The replies one session's stages finished, against their reservations.
#[derive(Clone, Debug, Default)]
struct Fits {
    /// Replies finished.
    replies: usize,
    /// The bytes the records derived from those replies' questions held,
    /// summed.
    records: usize,
    /// Those that held more than their reservation allowed, or whose questions
    /// derived records beyond its records part.
    over: Vec<ReplyFit>,
}

/// The most the charged places' transient heap may exceed a side's ledger in
/// total: every charged place's scratch ([`ledger_scratch`]).
fn ledger_slack() -> usize {
    CHARGED.iter().map(|&holding| ledger_scratch(holding)).sum()
}

/// The places the ledger charges: everything a session holds but its fixed
/// state (wire buffering, decode fans, assembly) and its setup.
const CHARGED: [Holding; 5] = [
    Holding::Questions,
    Holding::Resolutions,
    Holding::QuestionRecords,
    Holding::ParkedReplies,
    Holding::OutgoingReplies,
];

/// The position of `holding` in [`CHARGED`].
fn charged_index(holding: Holding) -> usize {
    CHARGED
        .iter()
        .position(|other| *other == holding)
        .expect("the place is one the ledger charges")
}

/// One ledger change, where the trace stood when it happened.
#[derive(Clone, Copy, Debug)]
struct LedgerEvent {
    /// The index the trace's next event took.
    position: usize,
    /// The ledger that changed.
    ledger: usize,
    /// What the ledger held after the change.
    held: usize,
    /// What it held for each place after the change, in [`Holding::ALL`] order.
    places: [usize; PLACES],
    /// Its excess after the change: what its kinds and depths held beyond their
    /// bounds.
    excess: usize,
    /// Bytes its items have needed beyond their grants.
    short: u64,
    /// Units that have waited for its admission.
    waited: u64,
    /// The side the driver was polling, if any.
    side: Option<Side>,
}

// Clippy's `missing_const_for_thread_local` can reject const-block
// initializers on targets that lower `thread_local!` through fallback TLS.
// The allows keep `-D warnings` clean on those targets.
thread_local! {
    /// The ledger changes of the session being run, on this thread.
    #[allow(clippy::missing_const_for_thread_local)]
    static LEDGER_EVENTS: RefCell<Vec<LedgerEvent>> = const { RefCell::new(Vec::new()) };
    /// The reply fits of the session being run, on this thread.
    #[allow(clippy::missing_const_for_thread_local)]
    static FITS: RefCell<Fits> = const { RefCell::new(Fits { replies: 0, records: 0, over: Vec::new() }) };
}

/// Records one ledger change of a traced session, outside the trace.
fn ledger_change(change: LedgerChange) {
    let Some(position) = alloc_meter::position() else {
        return;
    };
    let event = LedgerEvent {
        position,
        ledger: change.ledger,
        held: change.held,
        places: change.places,
        excess: change.excess,
        short: change.short,
        waited: change.waited,
        side: polling(),
    };
    untraced(|| LEDGER_EVENTS.with(|events| events.borrow_mut().push(event)));
}

/// Records one finished reply of a traced session, outside the trace.
fn reply_fit(fit: ReplyFit) {
    untraced(|| {
        FITS.with(|fits| {
            let mut fits = fits.borrow_mut();
            fits.replies += 1;
            fits.records += fit.records;
            if fit.reactions > fit.reaction_bound
                || fit.listing > fit.listing_bound
                || fit.records > fit.records_bound
            {
                fits.over.push(fit);
            }
        });
    });
}

/// Each side's ledger report from a session's ledger changes and its trace.
fn ledger_reports(events: &[LedgerEvent], session: &alloc_meter::Trace) -> [LedgerReport; 2] {
    Side::BOTH.map(|side| {
        // A ledger belongs to the side that was polled when it first
        // changed: every charge is made by a poll of its own side.
        let ledger = events
            .iter()
            .find(|event| event.side == Some(side))
            .map(|event| event.ledger);
        let mine: Vec<&LedgerEvent> = events
            .iter()
            .filter(|event| Some(event.ledger) == ledger)
            .collect();
        let labels: Vec<Label> = CHARGED
            .iter()
            .map(|&holding| side.holding_label(holding))
            .collect();
        // The ledger holds one state from each change to the next. Over that
        // stretch of the trace, the most a place's transient bytes exceed
        // its charge, or reach as a share of it, is therefore what its peak
        // within the stretch does, so each stretch's peaks decide coverage
        // at every allocation event in it.
        let starts: Vec<usize> = mine.iter().map(|event| event.position).collect();
        let peaks = session
            .segment_peaks(&starts, CHARGED.len(), |label| {
                labels.iter().position(|&own| own == label)
            })
            .expect("the trace is complete");
        let slots = CHARGED.map(|holding| {
            Holding::ALL
                .iter()
                .position(|other| *other == holding)
                .expect("ALL lists every holding")
        });
        let mut uncovered = 0;
        let mut uncovered_by_place = [0; CHARGED.len()];
        let mut tightest = [0; CHARGED.len()];
        for segment in 0..peaks.len() {
            // The first stretch precedes every change, while the ledger
            // holds nothing.
            let (held, places) = match segment.checked_sub(1) {
                Some(change) => (mine[change].held, mine[change].places),
                None => (0, [0; PLACES]),
            };
            uncovered = uncovered.max(peaks.all(segment).saturating_sub(held));
            for (place, slot) in slots.iter().enumerate() {
                let level = peaks.groups(segment)[place];
                let charged = places[*slot];
                uncovered_by_place[place] =
                    uncovered_by_place[place].max(level.saturating_sub(charged));
                if charged >= 4096 {
                    tightest[place] = tightest[place].max(level * 1000 / charged);
                }
            }
        }
        let last = mine.last();
        // Each event follows one change, so each rise is one charge.
        let charged = slots.map(|slot| {
            mine.iter()
                .fold((0, 0), |(sum, before), event| {
                    let now = event.places[slot];
                    (sum + now.saturating_sub(before), now)
                })
                .0
        });
        LedgerReport {
            changes: mine.len(),
            charged,
            peak: mine.iter().map(|event| event.held).max().unwrap_or(0),
            excess_peak: mine.iter().map(|event| event.excess).max().unwrap_or(0),
            within_peak: mine
                .iter()
                .map(|event| event.held - event.excess)
                .max()
                .unwrap_or(0),
            left: last.map_or(0, |event| event.held),
            short: last.map_or(0, |event| event.short),
            waited: last.map_or(0, |event| event.waited),
            uncovered,
            uncovered_by_place,
            tightest,
        }
    })
}

/// Runs one session between the peers [`fixture`] builds from `network`, both
/// on `budget`, over the held link released by `schedule`, and classifies what
/// it allocated.
///
/// The peers hold `messages` messages, in [`Side::BOTH`] order, that the other
/// lacks, and `redacted` more that both received and `A` redacted.
///
/// # Panics
///
/// Panics if the session fails, if a trace overflows its reservation, or if a
/// surviving block outlives both its peer and its replica.
fn run_session(
    network: NetworkSeed,
    messages: [u64; 2],
    redacted: u64,
    budget: usize,
    schedule: impl FnMut([usize; 2]) -> Release,
) -> Run {
    run_fixture(
        || fixture(network, messages, redacted, BudgetChoice::Ledger(budget)),
        messages,
        schedule,
    )
}

/// Runs one session between the peers `build` builds, holding `messages`
/// messages in [`Side::BOTH`] order that the other lacks, as [`run_session`]
/// does.
///
/// # Panics
///
/// As [`run_session`].
fn run_fixture<T>(
    build: impl FnOnce() -> Fixture<T>,
    messages: [u64; 2],
    schedule: impl FnMut([usize; 2]) -> Release,
) -> Run
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    assert!(
        attribute_allocations(attribute),
        "this suite's hook is the process's attribution hook"
    );
    assert!(
        observe_ledgers(ledger_change),
        "this suite's hook is the process's ledger hook"
    );
    assert!(
        observe_fits(reply_fit),
        "this suite's hook is the process's fit hook"
    );
    LEDGER_EVENTS.with(|events| events.borrow_mut().clear());
    FITS.with(|fits| *fits.borrow_mut() = Fits::default());
    let Fixture {
        peers: [a, b],
        sends,
        join,
        allocations,
    } = build();
    let link = Link::new();
    prepare_pacing();
    let (mut a_read, mut a_write) = (BufReader::new(link.reader(Side::A)), link.writer(Side::A));
    let (mut b_read, mut b_write) = (BufReader::new(link.reader(Side::B)), link.writer(Side::B));
    let moved = usize::try_from(messages[0] + messages[1]).expect("the fixture size fits usize");
    let capacity = EVENTS_PER_MESSAGE * moved + SETUP_EVENTS;

    let (session, (stats, drive)) = trace(capacity, || {
        let (a_out, b_out, drive) = drive(
            &link,
            a.gossip_once(&mut a_read, &mut a_write),
            b.gossip_once(&mut b_read, &mut b_write),
            schedule,
        );
        // Only the copied statistics leave the traced region; the reports
        // themselves are session state and are dropped inside it.
        let a_stats = a_out.expect("A's session completes").0.stats;
        let b_stats = b_out.expect("B's session completes").0.stats;
        ([a_stats, b_stats], drive)
    });
    let lifetimes = session
        .lifetimes()
        .unwrap_or_else(|incomplete| panic!("raise EVENTS_PER_MESSAGE: {incomplete:?}"));
    assert_eq!(
        lifetimes.reused_while_live, 0,
        "no other thread frees the session's blocks"
    );
    let survivors = session.survivors().expect("the trace is complete");
    let ledgers = LEDGER_EVENTS.with(|events| ledger_reports(&events.borrow(), &session));
    let fits = FITS.with(|fits| std::mem::take(&mut *fits.borrow_mut()));
    let by_side = session
        .lifetimes_grouped(Side::of_label)
        .expect("the trace is complete");
    drop(session);

    // The peers go first, while a snapshot keeps each replica alive, so
    // what their drop frees is what they kept beyond the replica.
    let teardown = allocations + survivors.len();
    let snapshots = (a.snapshot(), b.snapshot());
    let (peers, ()) = trace(teardown, || drop((a, b)));
    let (replicas, ()) = trace(teardown, || drop(snapshots));
    assert_eq!(
        peers.dropped() + replicas.dropped(),
        0,
        "raise the teardown reservation"
    );
    let freed = |teardown: &[Event]| -> HashSet<(usize, usize)> {
        teardown
            .iter()
            .filter(|event| event.kind != EventKind::Allocated)
            .map(|event| (event.address, event.size))
            .collect()
    };
    let (by_peers, by_replicas) = (freed(peers.events()), freed(replicas.events()));
    let mut kept = [Kept::default(); 2];
    let mut orphans = 0;
    for block in &survivors {
        let side = Side::BOTH
            .into_iter()
            .find(|side| Side::of_label(block.label) == side.label());
        let key = (block.address, block.size);
        match side {
            Some(side) if by_peers.contains(&key) => {
                let kept = &mut kept[side.index()];
                kept.peer_blocks += 1;
                kept.largest_peer_block = kept.largest_peer_block.max(block.size);
            }
            Some(side) if by_replicas.contains(&key) => {
                kept[side.index()].replica_bytes += block.size;
            }
            _ => orphans += 1,
        }
    }
    assert_eq!(
        orphans,
        0,
        "{orphans} of {} surviving blocks outlive both their peer and its replica, so they are \
         neither replica content nor peer state",
        survivors.len(),
    );

    Run {
        stats,
        sides: Side::BOTH.map(|side| by_side.label(side.label())),
        holdings: Side::BOTH
            .map(|side| Holding::ALL.map(|holding| lifetimes.label(side.holding_label(holding)))),
        unattributed: Side::BOTH.map(|side| lifetimes.label(side.label())),
        unlabelled: lifetimes.label(Label::NONE),
        kept,
        sends,
        join,
        drive,
        ledgers,
        fits,
    }
}

/// The reference session (see [the module docs](self#the-reference-session)),
/// measured once per process.
fn reference() -> &'static Run {
    static REFERENCE: OnceLock<Run> = OnceLock::new();
    REFERENCE.get_or_init(|| {
        run_fixture(
            || {
                fixture(
                    BUDGET_NETWORK_SEED,
                    [REFERENCE_MESSAGES; 2],
                    0,
                    BudgetChoice::Serial,
                )
            },
            [REFERENCE_MESSAGES; 2],
            |_| Release::bytes([Portion::All; 2]),
        )
    })
}

/// One measured session: the configuration, the run, and what it is judged
/// against.
struct Measured {
    /// Messages per side.
    messages: u64,
    /// The budget both peers ran.
    budget: usize,
    /// The bound on wire buffering.
    wire: u128,
    /// The bound on the decode fans.
    decoded: u128,
    /// Bytes each side's replica may keep beyond what building the merged
    /// replica directly allocates, for messages whose decoding keeps more than
    /// their sender's sends built.
    replica_slack: u128,
    /// The bound on assembly.
    assembly: u128,
    /// The bound on the unattributed blocks and admission's bookkeeping
    /// together: the reference session's larger transient peak.
    fixed: u128,
    /// Whether the outgoing replies' floor's premise holds for the fixture, so
    /// the floor is asserted (see the module docs).
    outgoing_floor: bool,
    /// The session under test.
    run: Run,
}

impl Measured {
    /// Measures the session [`run_session`] runs between fully divergent forks
    /// of the network `network` names, beside the bounds it is judged against.
    fn new(
        network: NetworkSeed,
        messages: u64,
        budget: usize,
        schedule: impl FnMut([usize; 2]) -> Release,
    ) -> Self {
        Self::redacting(network, messages, 0, budget, schedule)
    }

    /// Measures the session [`run_session`] runs with `redacted` shared
    /// messages that side `A` redacted, beside the bounds it is judged against.
    fn redacting(
        network: NetworkSeed,
        messages: u64,
        redacted: u64,
        budget: usize,
        schedule: impl FnMut([usize; 2]) -> Release,
    ) -> Self {
        let run = run_session(network, [messages; 2], redacted, budget, schedule);
        Self::of(messages, redacted, budget, run)
    }

    /// The measurement of `run`, a session of `messages` messages per side,
    /// `redacted` of them redacted by `A`, at `budget`, beside the bounds it is
    /// judged against.
    fn of(messages: u64, redacted: u64, budget: usize, run: Run) -> Self {
        let fixed = Side::BOTH
            .map(|side| reference().sides[side.index()].transient_peak)
            .into_iter()
            .max()
            .expect("a session has two sides") as u128;
        Self {
            messages,
            budget,
            wire: wire_allowance(COMPARISON_FRAME_BYTES) as u128,
            decoded: DECODE_AHEAD_BYTES as u128,
            replica_slack: 0,
            assembly: assembly_allowance() as u128,
            fixed,
            outgoing_floor: redacted == 0,
            run,
        }
    }

    /// The highest transient peak one side's allocations for `holding` may
    /// reach, for each class the ledger leaves out.
    ///
    /// The places the ledger charges ([`CHARGED`]) are bounded together, by the
    /// ledger's own bound ([`Self::charged_bound`]), and admission's
    /// bookkeeping together with the unattributed blocks ([`Self::fixed`]).
    ///
    /// Each class the ledger leaves out may reach its allowance; the module
    /// docs derive every bound.
    fn holding_bound(&self, holding: Holding) -> Option<u128> {
        match holding {
            Holding::DecodedLeaves => Some(self.decoded),
            Holding::Wire => Some(self.wire),
            Holding::Assembly => Some(self.assembly),
            Holding::Hashing => Some(HASHING_SCRATCH_BYTES as u128),
            _ => None,
        }
    }

    /// The highest transient peak `side`'s charged places may reach together.
    ///
    /// That is its ledger's bound, the budget and the most its kinds and depths
    /// held within their bounds, beside the scratch its stages fill before they
    /// charge what they built ([`ledger_slack`]).
    fn charged_bound(&self, side: Side) -> u128 {
        let ledger = self.run.ledgers[side.index()];
        (self.budget as u128) + (ledger.within_peak as u128) + (ledger_slack() as u128)
    }

    /// The highest transient peak `side`'s allocations may reach in all: its
    /// charged places' bound, every other holding's, and the bound on the
    /// unattributed blocks and admission's bookkeeping together.
    fn side_bound(&self, side: Side) -> u128 {
        Holding::ALL
            .iter()
            .filter_map(|&holding| self.holding_bound(holding))
            .sum::<u128>()
            + self.charged_bound(side)
            + self.fixed
    }

    /// The most bytes of survivors `side`'s replica may keep: the bytes the
    /// peer's messages took when its own sends built them, the interior a join
    /// of the two replicas builds, and the replica slack.
    fn replica_bound(&self, side: Side) -> u128 {
        let built = |stats: Stats| u128::try_from(stats.net_bytes).unwrap_or(0);
        built(self.run.sends[side.peer().index()]) + built(self.run.join) + self.replica_slack
    }

    /// A side's classified blocks.
    fn side(&self, side: Side) -> LabelLifetimes {
        self.run.sides[side.index()]
    }

    /// A side's session report.
    fn stats(&self, side: Side) -> SessionStats {
        self.run.stats[side.index()]
    }

    /// The bytes `side` allocated as work for `holding` over the session,
    /// transient and surviving together.
    fn allocated(&self, side: Side, holding: Holding) -> usize {
        let lifetimes = self.holding(side, holding);
        lifetimes.transient_bytes + lifetimes.surviving_bytes
    }

    /// A side's classified blocks allocated as work for `holding`.
    fn holding(&self, side: Side, holding: Holding) -> LabelLifetimes {
        let place = Holding::ALL
            .iter()
            .position(|other| *other == holding)
            .expect("ALL lists every holding");
        self.run.holdings[side.index()][place]
    }

    /// A one-line report of the measurement, for test output.
    fn report(&self) -> String {
        let mut line = format!(
            "{} messages per side, budget {} B: {} releases, largest delivery {} B; \
             bounds: wire {} B, assembly {} B, fixed {} B, sides {:?} B; unlabelled {:?}",
            self.messages,
            self.budget,
            self.run.drive.releases,
            self.run.drive.largest_delivery,
            self.wire,
            self.assembly,
            self.fixed,
            Side::BOTH.map(|side| self.side_bound(side)),
            self.run.unlabelled,
        );
        for side in Side::BOTH {
            let ledger = self.run.ledgers[side.index()];
            line.push_str(&format!("; {side:?} ledger:"));
            for (place, holding) in CHARGED.iter().enumerate() {
                line.push_str(&format!(
                    " {holding:?} gap {} B of {} B scratch, tightest {}‰, charged {} B of {} B \
                     allocated;",
                    ledger.uncovered_by_place[place],
                    ledger_scratch(*holding),
                    ledger.tightest[place],
                    ledger.charged[place],
                    self.allocated(side, *holding),
                ));
            }
            line.push_str(&format!(
                " total gap {} B of {} B; hashing peak {} B of {} B, {} B allocated",
                ledger.uncovered,
                ledger_slack(),
                self.holding(side, Holding::Hashing).transient_peak,
                HASHING_SCRATCH_BYTES,
                self.allocated(side, Holding::Hashing),
            ));
        }
        for side in Side::BOTH {
            let lifetimes = self.side(side);
            let kept = self.run.kept[side.index()];
            line.push_str(&format!(
                "; {side:?}: transient peak {} B ({:.3} of budget), {} B kept by the replica of \
                 {} B built directly, {} blocks kept by the peer, {} admissions waited, {:?} \
                 refused",
                lifetimes.transient_peak,
                lifetimes.transient_peak as f64 / self.budget as f64,
                kept.replica_bytes,
                self.replica_bound(side),
                kept.peer_blocks,
                self.run.ledgers[side.index()].waited,
                self.run.drive.refused[side.index()],
            ));
            for holding in Holding::ALL {
                let peak = self.holding(side, holding).transient_peak;
                match self.holding_bound(holding) {
                    Some(bound) => line.push_str(&format!(", {holding:?} {peak} B of {bound} B")),
                    None => line.push_str(&format!(", {holding:?} {peak} B")),
                }
            }
            line.push_str(&format!(
                ", charged places of {} B",
                self.charged_bound(side)
            ));
            line.push_str(&format!(
                ", unattributed {} B",
                self.run.unattributed[side.index()].transient_peak
            ));
            line.push_str(&format!(", ledger {:?}", self.run.ledgers[side.index()]));
        }
        line.push_str(&format!(
            "; {} replies finished, {} over their reservations",
            self.run.fits.replies,
            self.run.fits.over.len()
        ));
        line
    }
}

/// Runs a region the session attributes to `holding` under the label of that
/// holding on the side being polled.
///
/// Outside a poll the region runs under whatever label is current: the session
/// allocates only while the driver polls it.
fn attribute(holding: Holding, region: &mut dyn FnMut()) {
    match polling() {
        Some(side) => labelled(side.holding_label(holding), region),
        None => region(),
    }
}

/// Returns why a measured session falls outside a bound or fails to show that
/// the budget constrained it, or `None` if it passes.
fn violation(measured: &Measured) -> Option<String> {
    if let Some(violation) = bound_violation(measured) {
        return Some(violation);
    }
    let report = measured.report();
    // Every divergent fixture's replies ask questions; a joining session's
    // need not, so this floor lives here rather than with the ledger's.
    if measured.run.fits.records == 0 {
        return Some(format!(
            "no reply's questions derived a record, so the records part's fit stopped \
             counting: {report}"
        ));
    }
    if Side::BOTH
        .into_iter()
        .all(|side| measured.run.ledgers[side.index()].waited == 0)
    {
        return Some(format!("no unit ever waited for admission: {report}"));
    }
    None
}

/// Returns why a measured session falls outside a bound, or `None` if it stays
/// within every one: [`violation`]'s checks short of the two that show the
/// budget constrained a divergent fixture.
fn bound_violation(measured: &Measured) -> Option<String> {
    let report = measured.report();
    let unlabelled = measured.run.unlabelled;
    if unlabelled.transient_blocks + unlabelled.surviving_blocks > 0 {
        return Some(format!(
            "every block a session allocates carries its side's label: {report}"
        ));
    }
    let reference = reference();
    for side in Side::BOTH {
        let stats = measured.stats(side);
        if stats.messages_gained != measured.messages {
            return Some(format!(
                "{side:?} gained {} of {} messages: {report}",
                stats.messages_gained, measured.messages,
            ));
        }
        let waited = measured.run.ledgers[side.index()].waited;
        if stats.budget_waits != waited {
            return Some(format!(
                "{side:?} reported {} budget waits where its ledger counted {waited}: {report}",
                stats.budget_waits,
            ));
        }
        let lifetimes = measured.side(side);
        if (lifetimes.surviving_blocks as u64) < stats.messages_gained {
            return Some(format!(
                "{side:?} kept {} blocks for {} gained messages, so the trace missed \
                 replica content: {report}",
                lifetimes.surviving_blocks, stats.messages_gained,
            ));
        }
        let kept = measured.run.kept[side.index()];
        if kept.replica_bytes as u128 > measured.replica_bound(side) {
            return Some(format!(
                "{side:?}'s replica kept more than building the merged replica directly \
                 allocates: {report}"
            ));
        }
        let reference_kept = reference
            .kept
            .iter()
            .fold(Kept::default(), |most, kept| Kept {
                peer_blocks: most.peer_blocks.max(kept.peer_blocks),
                largest_peer_block: most.largest_peer_block.max(kept.largest_peer_block),
                ..most
            });
        if kept.peer_blocks > reference_kept.peer_blocks
            || kept.largest_peer_block > reference_kept.largest_peer_block
        {
            return Some(format!(
                "{side:?} kept {} blocks beyond its replica, the largest {} B, where the \
                 reference session keeps {} of at most {} B: {report}",
                kept.peer_blocks,
                kept.largest_peer_block,
                reference_kept.peer_blocks,
                reference_kept.largest_peer_block,
            ));
        }
        for holding in Holding::ALL {
            let peak = measured.holding(side, holding).transient_peak as u128;
            let Some(bound) = measured.holding_bound(holding) else {
                continue;
            };
            if peak > bound {
                return Some(format!(
                    "{side:?}'s {holding:?} peak of {peak} B exceeds its bound of {bound} B: \
                     {report}"
                ));
            }
        }
        let outgoing =
            measured.run.ledgers[side.index()].charged[charged_index(Holding::OutgoingReplies)];
        let built = measured.allocated(side, Holding::OutgoingReplies);
        // The floor is asserted only where its premise holds (see the
        // module docs).
        if measured.outgoing_floor && outgoing * OUTGOING_REPLIES_FLOOR_DIVISOR < built {
            return Some(format!(
                "{side:?}'s ledger was charged {outgoing} B for outgoing replies that allocated \
                 {built} B, less than 1/{OUTGOING_REPLIES_FLOOR_DIVISOR} of it: {report}"
            ));
        }
        let unattributed = measured.run.unattributed[side.index()].transient_peak as u128;
        let admission = measured.holding(side, Holding::Admission).transient_peak as u128;
        if unattributed + admission > measured.fixed {
            return Some(format!(
                "{side:?}'s unattributed peak of {unattributed} B and admission's peak of \
                 {admission} B exceed the reference session's transient peak of {} B: {report}",
                measured.fixed,
            ));
        }
        if lifetimes.transient_peak as u128 > measured.side_bound(side) {
            return Some(format!(
                "{side:?}'s transient peak exceeds the sum of its labels' bounds: {report}"
            ));
        }
    }
    if let Some(violation) = ledger_violation(&measured.run, measured.budget) {
        return Some(format!("{violation}: {report}"));
    }
    None
}

/// Returns why a session's ledgers fail one of their three accounting
/// identities or their bound, or `None` if they hold.
///
/// Each side's ledger must have counted, must cover the transient heap of each
/// place it charges, less that place's scratch, at every allocation event, and
/// must return to zero once the session ends; every reply the session's stages
/// finished must fit the room its question reserved for it, records part
/// included, with at least one reply finished. And each side's ledger,
/// admitting against `budget`, must keep its excess within the budget and what
/// its kinds and depths hold within their bounds within the floor allowance, so
/// that it holds at most what its peer's memory budget leaves it beside the
/// session allowance, with every item built within its unit's grant.
fn ledger_violation(run: &Run, budget: usize) -> Option<String> {
    for side in Side::BOTH {
        let ledger = run.ledgers[side.index()];
        if ledger.changes == 0 {
            return Some(format!(
                "{side:?}'s ledger never changed, so it stopped counting"
            ));
        }
        if ledger.excess_peak > budget {
            return Some(format!(
                "{side:?}'s ledger's excess reached {} B, over its {budget} B budget: \
                 {ledger:?}",
                ledger.excess_peak,
            ));
        }
        if ledger.within_peak > FLOOR_ALLOWANCE {
            return Some(format!(
                "{side:?}'s kinds and depths held {} B within their bounds, over the \
                 {FLOOR_ALLOWANCE} B floor allowance: {ledger:?}",
                ledger.within_peak,
            ));
        }
        if ledger.short > 0 {
            return Some(format!(
                "{side:?}'s items needed {} B beyond their units' grants: {ledger:?}",
                ledger.short,
            ));
        }
        if ledger.uncovered > ledger_slack() {
            return Some(format!(
                "{side:?}'s charged places exceeded its ledger by {} B, over the {} B of its \
                 stages' scratch: {ledger:?}",
                ledger.uncovered,
                ledger_slack(),
            ));
        }
        for (place, holding) in CHARGED.iter().enumerate() {
            let scratch = ledger_scratch(*holding);
            if ledger.uncovered_by_place[place] > scratch {
                return Some(format!(
                    "{side:?}'s {holding:?} exceeded the ledger's charges for it by {} B, \
                     over its {scratch} B of scratch: {ledger:?}",
                    ledger.uncovered_by_place[place],
                ));
            }
        }
        if ledger.left != 0 {
            return Some(format!(
                "{side:?}'s ledger still held {} B after the session: {ledger:?}",
                ledger.left
            ));
        }
    }
    if run.fits.replies == 0 {
        return Some("no reply was finished, so the fit check stopped counting".to_owned());
    }
    if let Some(over) = run.fits.over.first() {
        return Some(format!(
            "{} replies held more than their questions reserved for, the first {over:?}",
            run.fits.over.len()
        ));
    }
    None
}

/// Measures one fully divergent session at `budget` under `schedule`, prints
/// its report, and asserts it passes the bounds and `reached`, which returns
/// why the session failed to reach the state its test targets.
///
/// The session's peers hold [`MESSAGES`] messages each, and are forks of the
/// network [`BUDGET_NETWORK_SEED`] names.
fn assert_budget_holds(
    budget: usize,
    schedule: impl FnMut([usize; 2]) -> Release,
    reached: impl Fn(&Measured) -> Option<String>,
) {
    let measured = Measured::new(BUDGET_NETWORK_SEED, MESSAGES, budget, schedule);
    println!("{}", measured.report());
    if let Some(violation) = violation(&measured).or_else(|| reached(&measured)) {
        panic!("{violation}");
    }
}

/// Releases every held byte at every stall and holds back no task.
fn release_everything(_: [usize; 2]) -> Release {
    Release::bytes([Portion::All; 2])
}

/// Each side's ledger peak, in [`Side::BOTH`] order, in the session at 8 MiB.
///
/// Admission fills each side's budget with work beyond the bounds of its kinds
/// and depths, as far as each depth's claims under the invitation share let its
/// questions take it, and each kind and depth holds up to its bound beside it:
/// each peak is at most the 8 MiB budget plus what its kinds and depths hold
/// within their bounds.
const NEAR_FLOOR_LEDGER_PEAKS: [usize; 2] = [10_453_812, 11_821_620];

/// Each side's ledger peak, in [`Side::BOTH`] order, in the session at
/// [`WIDE_BUDGET`]: the work its budget and its depths' claims admit, beside
/// what its kinds and depths hold within their bounds.
///
/// A claim reserves room for processing still to come, so a side's peak can
/// stay below its budget.
const WIDE_LEDGER_PEAKS: [usize; 2] = [33_255_888, 36_981_288];

/// Each side's ledger peak, in [`Side::BOTH`] order, when side `A` joins from
/// empty.
///
/// It asks one question, about the root, and the responder answers it with
/// supplies alone, so neither side holds more than that one question and that
/// one answer.
const JOINING_LEDGER_PEAKS: [usize; 2] = [29_368, 39_200];

/// Targets a session whose ledgers peak at exactly `expected`, the fixture's
/// committed baseline.
fn ledger_peaks(expected: [usize; 2]) -> impl Fn(&Measured) -> Option<String> {
    move |measured| {
        let peaks = measured.run.ledgers.map(|ledger| ledger.peak);
        (peaks != expected).then(|| {
            format!(
                "the ledger peaks moved from {expected:?} to {peaks:?}: {}",
                measured.report()
            )
        })
    }
}

/// At 8 MiB, just above the floor, each place a peer's session keeps in-flight
/// state stays within its bound, and each peer's whole transient peak stays
/// within their sum.
///
/// At 8 MiB most of the session's units wait for admission at [`MESSAGES`] per
/// side, and the fixed work near the root, which the budget does not shrink,
/// weighs most against it.
#[test]
fn budget_near_the_floor_bounds_the_transient_peak() {
    assert_budget_holds(
        8 << 20,
        release_everything,
        ledger_peaks(NEAR_FLOOR_LEDGER_PEAKS),
    );
}

/// At [`WIDE_BUDGET`], each place a peer's session keeps in-flight state stays
/// within its bound, and each peer's whole transient peak stays within their
/// sum.
///
/// Units still wait for admission, and the queued work the budget admits,
/// rather than the fixed work near the root, dominates the session's transient
/// heap. Its ledger peaks are committed baselines.
#[test]
fn wide_budget_bounds_the_transient_peak() {
    assert_budget_holds(
        WIDE_BUDGET,
        release_everything,
        ledger_peaks(WIDE_LEDGER_PEAKS),
    );
}

/// At [`WIDE_BUDGET`], with one side's walk stages held back while any bytes
/// are in flight, every label and each peer's whole transient peak stay within
/// their bounds.
///
/// The held side cannot take the replies its decoders park, nor send more
/// questions, while its peer answers every question it already sent, and the
/// answers park beside the questions the walk still holds: the state the
/// reservations for parked replies cover. The held side changes only at a stall
/// with nothing left to deliver, which also releases its stages, so each side's
/// parking fills in turn from every question its budget admitted.
///
/// The test checks that the state was reached: each side's walk was held back,
/// and each side's parked replies peaked higher than in the same session with
/// nothing held back, which the test runs first.
#[test]
fn held_back_takes_bound_the_transient_peak() {
    let unheld = unheld_wide_peaks(Holding::ParkedReplies);
    let mut side = Side::A;
    assert_budget_holds(
        WIDE_BUDGET,
        |held| {
            let mut release = Release::bytes([Portion::All; 2]);
            if held.iter().any(|&bytes| bytes > 0) {
                release.held[side.index()].walk = true;
            } else {
                side = side.peer();
            }
            release
        },
        |measured| {
            held_back_raised(measured, Holding::ParkedReplies, unheld, |refused| {
                refused.walk
            })
        },
    );
}

/// At [`WIDE_BUDGET`], with assembly held back whenever a stall has bytes to
/// release, each place stays within its bound, and each peer's whole transient
/// peak stays within their sum.
///
/// Held-back assemblers consume no resolutions, so resolutions pile up while
/// the walk keeps asking until admission or its own sends stop it, at every
/// depth at once. Assembly runs only at stalls with nothing left to deliver.
///
/// The test checks that the state was reached: each side's assembly was held
/// back, and each side's resolutions peaked higher than in the same session
/// with nothing held back, which the test runs first.
#[test]
fn held_back_assembly_bounds_the_transient_peak() {
    let unheld = unheld_wide_peaks(Holding::Resolutions);
    assert_budget_holds(
        WIDE_BUDGET,
        |held| {
            let assembly = Held {
                assembly: held.iter().any(|&bytes| bytes > 0),
                ..Held::NOTHING
            };
            Release {
                bytes: [Portion::All; 2],
                held: [assembly; 2],
            }
        },
        |measured| {
            held_back_raised(measured, Holding::Resolutions, unheld, |refused| {
                refused.assembly
            })
        },
    );
}

/// Each side's transient peak of `holding`, in [`Side::BOTH`] order, in the
/// session [`assert_budget_holds`] measures at [`WIDE_BUDGET`] with nothing
/// held back: the peak a held-back fixture must exceed.
fn unheld_wide_peaks(holding: Holding) -> [usize; 2] {
    let unheld = Measured::new(
        BUDGET_NETWORK_SEED,
        MESSAGES,
        WIDE_BUDGET,
        release_everything,
    );
    Side::BOTH.map(|side| unheld.holding(side, holding).transient_peak)
}

/// Returns why the session failed to hold back the tasks `refused` counts on
/// each side, or to raise each side's `holding` peak above its peak in the same
/// session with nothing held back, `unheld`, or `None` if it did both.
fn held_back_raised(
    measured: &Measured,
    holding: Holding,
    unheld: [usize; 2],
    refused: impl Fn(&link::Refused) -> usize,
) -> Option<String> {
    Side::BOTH.into_iter().find_map(|side| {
        let held = refused(&measured.run.drive.refused[side.index()]);
        let peak = measured.holding(side, holding).transient_peak;
        let unheld_peak = unheld[side.index()];
        (held == 0 || peak <= unheld_peak).then(|| {
            format!(
                "{side:?} must be held back and raise its {holding:?} peak above the \
                 unheld session's {unheld_peak} B: {held} polls refused, peak {peak} B; {}",
                measured.report(),
            )
        })
    })
}

/// Generates the portion one stall releases from one direction.
fn arb_portion() -> impl Strategy<Value = Portion> {
    prop_oneof![
        Just(Portion::Hold),
        (1u8..16).prop_map(Portion::Prefix),
        Just(Portion::All),
    ]
}

/// Generates which of one side's task kinds one stall holds back.
fn arb_held() -> impl Strategy<Value = Held> {
    (any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>()).prop_map(
        |(walk, assembly, encoder, decoder)| Held {
            walk,
            assembly,
            encoder,
            decoder,
        },
    )
}

/// Generates one stall's release: a portion of each direction's held bytes and
/// the task kinds each side holds back.
fn arb_release() -> impl Strategy<Value = Release> {
    ([arb_portion(), arb_portion()], [arb_held(), arb_held()])
        .prop_map(|(bytes, held)| Release { bytes, held })
}

/// The divisor of [`any_release_schedule_keeps_the_bound`]'s case count.
///
/// Each case builds two [`SEARCH_MESSAGES`]-message replicas and runs a traced
/// session, a few seconds of work in a debug build, so the default count runs
/// about a dozen cases.
const SCHEDULE_DIVISOR: u32 = 20;

/// Shrink iterations [`any_release_schedule_keeps_the_bound`] spends on a
/// failure: each costs a case's few seconds.
const SCHEDULE_SHRINK_ITERS: u32 = 24;

/// The divisor of the case count of a test that builds replicas of thousands of
/// messages and runs one or more whole sessions: the redacting and placed
/// fixtures and the hop sweep.
///
/// A case costs seconds to tens of seconds in a debug build, as much as the
/// default count's whole run of ordinary cases, so the default count runs one
/// case and a raised count one per this many.
const WHOLE_FIXTURE_DIVISOR: u32 = 256;

/// Under any release schedule, each place a peer's session keeps in-flight
/// state stays within its bound, and each peer's whole transient peak stays
/// within their sum.
///
/// Units wait for admission.
///
/// At each stall the schedule releases all, a prefix, or none of each
/// direction's held bytes, and holds back any kinds of each side's tasks until
/// the next stall, cycling through the generated releases. So deliveries split
/// frames at arbitrary points, either direction can lag the other, and any of a
/// side's walk stages, assemblers, encoders, and decoders can stall while the
/// rest run.
///
/// The claim holds for every network seed, drawn after the schedule.
///
/// The property runs the configured case count divided by [`SCHEDULE_DIVISOR`],
/// and shrinks a failure for at most [`SCHEDULE_SHRINK_ITERS`] iterations. Its
/// runner is built here rather than by `divided_proptest!`, which keeps
/// proptest's shrink budget; it keeps the macro's failure persistence by naming
/// this file.
#[test]
fn any_release_schedule_keeps_the_bound() {
    let mut config = Config::default();
    config.cases = divided_cases(config.cases, SCHEDULE_DIVISOR);
    config.max_shrink_iters = SCHEDULE_SHRINK_ITERS;
    config.source_file = Some(file!());
    let mut runner = TestRunner::new(config);
    let strategy = (vec(arb_release(), 1..=8), network_seed());
    let result = runner.run(&strategy, |(releases, network)| {
        let mut next = releases.iter().cycle();
        let measured = Measured::new(network, SEARCH_MESSAGES, SEARCH_BUDGET, |_| {
            *next
                .next()
                .expect("a cycle over a non-empty list never ends")
        });
        println!("{}", measured.report());
        let violation = violation(&measured);
        prop_assert!(violation.is_none(), "{}", violation.unwrap_or_default());
        Ok(())
    });
    if let Err(failure) = result {
        panic!("{failure}");
    }
}

/// Depth-31 nodes the placed fixture's sides dispute.
const PLACED_NODES: u64 = 8;

/// Leaves each side holds under each placed node: half a fan, so the two sides
/// fill it.
const PLACED_HALF_FAN: u64 = 128;

/// The placed fixture's budget, 64 KiB: room for many leaf requests at once,
/// beside units that still wait.
const PLACED_BUDGET: usize = 64 << 10;

/// The placed path of the message of `value`: under placed node `index / 128`,
/// its first byte, at a last byte from the lower half of the fan for `A` and
/// the upper half for `B`, with zeros between.
///
/// Every path below one node shares its first 31 bytes, so the two sides
/// dispute that node one byte above the leaves.
fn placed_path(value: u64) -> [u8; 32] {
    let (half, index) = match value.checked_sub(B_VALUES) {
        Some(index) => (PLACED_HALF_FAN, index),
        None => (0, value),
    };
    let mut path = [0; 32];
    path[0] = u8::try_from(index / PLACED_HALF_FAN).expect("the placed nodes fit a byte");
    path[31] = u8::try_from(half + index % PLACED_HALF_FAN).expect("a radix fits a byte");
    path
}

// Clippy's `missing_const_for_thread_local` can reject const-block
// initializers on targets that lower `thread_local!` through fallback TLS.
// The allow keeps `-D warnings` clean on those targets.
thread_local! {
    /// Leaf requests each side's ledger admitted, in [`Side::BOTH`] order of
    /// the ledgers' first admissions, for the session the placed test runs
    /// on this thread.
    ///
    /// A ledger admits while the driver polls the session, on the thread
    /// that runs it, so every admission the placed session makes lands in
    /// that thread's counts. Keeping them per thread stops another test's
    /// session, running at once in the same process under `cargo test`,
    /// from adding leaf requests that would meet the placed test's floor
    /// for it.
    #[allow(clippy::missing_const_for_thread_local)]
    static LEAF_REQUESTS: RefCell<Vec<(usize, usize)>> = const { RefCell::new(Vec::new()) };
}

/// Counts one admitted leaf request, a question about a scope at depth 32,
/// against its ledger, outside the trace.
fn count_leaf_request(admission: Admission) {
    if admission.question && admission.depth == 32 && !admission.leftover {
        untraced(|| {
            LEAF_REQUESTS.with_borrow_mut(|counts| {
                match counts
                    .iter_mut()
                    .find(|(ledger, _)| *ledger == admission.ledger)
                {
                    Some((_, count)) => *count += 1,
                    None => counts.push((admission.ledger, 1)),
                }
            });
        });
    }
}

divided_proptest! {
    #![divisor(WHOLE_FIXTURE_DIVISOR)]

    /// When the two sides differ under nodes one byte above the leaves, each
    /// place a session keeps state in stays within its bound and the ledgers
    /// keep their identities, with leaf requests in flight.
    ///
    /// Hashing cannot place leaves that share 31 bytes, so the fixture
    /// places them ([`with_leaf_paths`]): under each of [`PLACED_NODES`]
    /// nodes, `A` holds the lower half of a fan of leaves and `B` the upper
    /// half. Every level above disputes, and each node's leaf-parent answer
    /// asks a leaf request for each leaf the peer listed that it lacks,
    /// half a fan per node. At [`PLACED_BUDGET`] many are in flight at
    /// once, more than the questions' scratch allows per stream, so
    /// coverage sees a leaf request's charge go missing; and every record
    /// derived for a leaf request is held to the records part of the
    /// question that asked about its parent.
    ///
    /// Each leaf parent's peeled view is hashed anew over its half fan of
    /// children, so each side must hash: the hashing place's bound and the
    /// outgoing replies' floor, which leaves that hashing out, then both judge
    /// work that happened.
    ///
    /// A first build of the same network, outside the placement, learns each
    /// message's version, since forking and sending a network from one seed
    /// are deterministic; the placement panics on any version it does not
    /// list, so a rebuild that drifted fails loudly.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn leaf_requests_keep_the_bound(network in network_seed()) {
        assert!(
            observe_admissions(count_leaf_request),
            "this suite's hook is the process's admission hook"
        );
        // Each case counts its own session's leaf requests.
        LEAF_REQUESTS.with_borrow_mut(Vec::clear);
        let messages = [PLACED_NODES * PLACED_HALF_FAN; 2];
        // The reference session's leaves are hashed, so it runs outside the
        // placement.
        reference();
        let dry = fixture(network, messages, 0, BudgetChoice::Ledger(PLACED_BUDGET));
        let paths: Vec<(Version, [u8; 32])> = dry
            .peers
            .iter()
            .flat_map(|peer| {
                peer.snapshot()
                    .iter()
                    .map(|(version, value)| (version.clone(), placed_path(*value)))
                    .collect::<Vec<_>>()
            })
            .collect();
        drop(dry);
        let run = with_leaf_paths(paths, || {
            run_fixture(
                || fixture(network, messages, 0, BudgetChoice::Ledger(PLACED_BUDGET)),
                messages,
                release_everything,
            )
        });
        let measured = Measured::of(messages[0], 0, PLACED_BUDGET, run);
        let leaf_requests = LEAF_REQUESTS.with_borrow(Vec::clone);
        println!(
            "{}; leaf requests per ledger {leaf_requests:?}",
            measured.report()
        );
        if let Some(violation) = violation(&measured) {
            panic!("{violation}");
        }
        for side in Side::BOTH {
            assert!(
                measured.allocated(side, Holding::Hashing) > 0,
                "{side:?} must hash a peeled view: {}",
                measured.report()
            );
        }
        let most = leaf_requests
            .iter()
            .map(|(_, count)| *count)
            .max()
            .unwrap_or(0);
        assert!(
            most as u64 >= PLACED_NODES * PLACED_HALF_FAN / 2,
            "the placed fixture must make leaf requests: {leaf_requests:?}"
        );
    }
}

divided_proptest! {
    #![divisor(WHOLE_FIXTURE_DIVISOR)]

    /// When side `A` has redacted every message the two sides shared, each
    /// place a session keeps state in stays within its bound, and the ledgers
    /// keep their identities.
    ///
    /// Each peer's whole transient peak stays within its labels' bounds, at
    /// [`SEARCH_BUDGET`], where units wait.
    ///
    /// The fixture's peers share [`SEARCH_MESSAGES`] messages, which `A`
    /// redacts, beside [`SEARCH_MESSAGES`] of each side's own, so `B`'s
    /// subtrees mix leaves `A` lacks with leaves `A`'s version knows. The test
    /// checks that the state was reached: `B` sheds every shared message, and
    /// `A` sheds none.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn redacting_replicas_keep_the_bound(network in network_seed()) {
        let measured = Measured::redacting(
            network,
            SEARCH_MESSAGES,
            SEARCH_MESSAGES,
            SEARCH_BUDGET,
            release_everything,
        );
        println!("{}", measured.report());
        if let Some(violation) = violation(&measured) {
            panic!("{violation}");
        }
        let shed = Side::BOTH.map(|side| measured.stats(side).messages_shed);
        assert_eq!(
            shed,
            [0, SEARCH_MESSAGES],
            "B must shed every message A redacted: {}",
            measured.report()
        );
    }
}

/// When one replica joins from empty, the side receiving every message as whole
/// supplied subtrees holds no more in assembly than the constant bound,
/// whatever the subtrees' size.
///
/// The peer holds [`MESSAGES`] messages, so each subtree supplied below the
/// root holds about four hundred leaves, and the receiving side assembles every
/// one. The bound ([`assembly_allowance`]) counts each of the
/// [`SESSION_STREAMS`](rumors::testing::SESSION_STREAMS) reply decoders
/// assembling one subtree at a time, each holding only its open right spine.
///
/// Each subtree of two or more leaves leaves at least one branch in the
/// replica, so the receiver's assembly must leave at least one surviving block
/// per root child of its peer's replica: a trace that stopped seeing assembly
/// cannot pass. That floor rests on every one of the peer's 256 root children
/// holding two or more of its messages, which fails at this size with
/// probability below 2⁻⁴⁰⁰.
#[test]
fn joining_from_empty_bounds_assembly() {
    let run = run_session(
        BUDGET_NETWORK_SEED,
        [0, MESSAGES],
        0,
        WIDE_BUDGET,
        release_everything,
    );
    let place = Holding::ALL
        .iter()
        .position(|holding| *holding == Holding::Assembly)
        .expect("ALL lists every holding");
    let assembly = run.holdings[Side::A.index()][place];
    println!(
        "joining from empty: assembly {assembly:?} of {} B; ledgers {:?}; {} replies finished, \
         {} over",
        assembly_allowance(),
        run.ledgers,
        run.fits.replies,
        run.fits.over.len()
    );
    assert!(
        assembly.transient_peak <= assembly_allowance(),
        "the joining side's assembly held {} B, over its {} B bound",
        assembly.transient_peak,
        assembly_allowance(),
    );
    assert!(
        assembly.surviving_blocks >= 256,
        "the joining side's assembly left {} blocks, fewer than one per supplied root child",
        assembly.surviving_blocks,
    );
    if let Some(violation) = ledger_violation(&run, WIDE_BUDGET) {
        panic!("{violation}");
    }
    assert_eq!(
        run.ledgers.map(|ledger| ledger.peak),
        JOINING_LEDGER_PEAKS,
        "the joining session's ledger peaks moved"
    );
}

/// Messages each side of the hop sweep's fixture holds that the other lacks:
/// enough that the budget binds across a wide range of budgets, small enough
/// that the sweep's sessions run in seconds together.
const SWEEP_MESSAGES: u64 = SEARCH_MESSAGES;

/// The hop sweep's smallest budget above zero: 64 KiB.
const SWEEP_FIRST: usize = 64 << 10;

/// The hop sweep's budgets above zero rise by a factor of `2^(1/STEPS)`.
const SWEEP_STEPS_PER_DOUBLING: u32 = 2;

/// The doublings the hop sweep spans above [`SWEEP_FIRST`]: to 64 MiB, which
/// the sweep requires to run the fixture as with no budget.
const SWEEP_DOUBLINGS: u32 = 10;

/// The hop sweep's budgets, rising: zero, then from [`SWEEP_FIRST`] by a factor
/// of `2^(1/SWEEP_STEPS_PER_DOUBLING)` for [`SWEEP_DOUBLINGS`] doublings, each
/// rounded down to a byte.
fn sweep_budgets() -> Vec<usize> {
    let steps = SWEEP_STEPS_PER_DOUBLING * SWEEP_DOUBLINGS;
    std::iter::once(0)
        .chain((0..=steps).map(|step| {
            let factor = 2f64.powf(f64::from(step) / f64::from(SWEEP_STEPS_PER_DOUBLING));
            (SWEEP_FIRST as f64 * factor) as usize
        }))
        .collect()
}

/// One session of the hop sweep, untraced: its hops, the releases the held link
/// made with every byte released at every stall, and the units both sides
/// report waiting.
///
/// Its peers are the sweep's fixture, forked from the network `network` names,
/// on `budget`. The seed makes the fixture reproducible, so every budget's
/// session runs between the same two replicas.
fn sweep_session(network: NetworkSeed, budget: usize) -> (usize, u64) {
    let Fixture { peers: [a, b], .. } = fixture(
        network,
        [SWEEP_MESSAGES; 2],
        0,
        BudgetChoice::Ledger(budget),
    );
    let link = Link::new();
    prepare_pacing();
    let (mut a_read, mut a_write) = (BufReader::new(link.reader(Side::A)), link.writer(Side::A));
    let (mut b_read, mut b_write) = (BufReader::new(link.reader(Side::B)), link.writer(Side::B));
    let (a_out, b_out, drive) = drive(
        &link,
        a.gossip_once(&mut a_read, &mut a_write),
        b.gossip_once(&mut b_read, &mut b_write),
        release_everything,
    );
    let waits = [a_out, b_out].map(|out| {
        out.expect("the sweep's session completes")
            .0
            .stats
            .budget_waits
    });
    (drive.releases, waits.iter().sum())
}

/// The result of a scoped thread, propagating its panic, message and all, if it
/// panicked.
fn joined<T>(thread: ScopedJoinHandle<'_, T>) -> T {
    thread
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

divided_proptest! {
    #![divisor(WHOLE_FIXTURE_DIVISOR)]

    /// A session never takes more hops at a larger budget.
    ///
    /// On the held link, releasing every byte at every stall, each release is
    /// one hop: everything either side sent since the last stall crosses at
    /// once, so the releases count the session's serialized round trips
    /// exactly, whatever the machine's load. The sweep runs one fixture at
    /// zero and at budgets rising geometrically past its no-cost point, and
    /// the hops must never rise from one budget to the next.
    ///
    /// The sweep must span the budgets that matter: at zero some unit waits
    /// and the session takes more hops than with no budget, and at the largest
    /// budget no unit waits and the session takes exactly the hops it takes
    /// with no budget.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn hops_never_rise_with_the_budget(network in network_seed()) {
        // Each session drives its own peers and link on the thread that runs
        // it, and its hops are exact, so the sessions run at once, one thread
        // each.
        let ((unlimited, unlimited_waits), runs) = thread::scope(|scope| {
            let unlimited = scope.spawn(move || sweep_session(network, usize::MAX));
            let sweep: Vec<_> = sweep_budgets()
                .into_iter()
                .map(|budget| (budget, scope.spawn(move || sweep_session(network, budget))))
                .collect();
            let runs: Vec<(usize, usize, u64)> = sweep
                .into_iter()
                .map(|(budget, session)| {
                    let (hops, waits) = joined(session);
                    (budget, hops, waits)
                })
                .collect();
            (joined(unlimited), runs)
        });
        assert_eq!(unlimited_waits, 0, "no unit waits with no budget");
        for (budget, hops, waits) in &runs {
            println!("budget {budget} B: {hops} hops, {waits} units waited");
        }
        println!("no budget: {unlimited} hops");
        for pair in runs.windows(2) {
            let [(smaller, smaller_hops, _), (larger, larger_hops, _)] = pair else {
                unreachable!("windows of two hold two runs");
            };
            assert!(
                larger_hops <= smaller_hops,
                "{larger_hops} hops at {larger} B exceed {smaller_hops} at {smaller} B"
            );
        }
        let (_, first_hops, first_waits) = runs[0];
        let &(top, top_hops, top_waits) = runs.last().expect("the sweep runs budgets");
        assert!(
            first_waits > 0 && first_hops > unlimited,
            "at zero the budget must cost the session hops: {first_hops} against {unlimited}"
        );
        assert!(
            top_waits == 0 && top_hops == unlimited,
            "at {top} B the session must run as with no budget: {top_hops} hops against \
             {unlimited}, {top_waits} units waited"
        );
    }
}

/// The byte-vector payloads [`large_byte_vectors_keep_the_bound`] sends: bytes,
/// messages per side, the target both peers run at, and whether the message
/// lies past the largest budgeted message at that target.
///
/// The first is past the preallocation cap serde's sequence decoding keeps to,
/// 1 MiB, so decoding it outgrows a buffer. Its peers run at [`WITHIN_TARGET`],
/// whose run budget holds it, so it travels in a run of its own beside the wire
/// pools. The second is a mebibyte past the largest budgeted message at the
/// default target, at which its peers run, so it travels alone, beyond the run
/// budget, and holds its excess beside the pools as it crosses.
const LARGE_PAYLOADS: [(usize, u64, usize, bool); 2] = [
    ((1 << 20) + (1 << 16), 4, WITHIN_TARGET, false),
    (
        rumors::MAX_BUDGETED_MESSAGE_SIZE + (1 << 20),
        2,
        rumors::default::TARGET_MESSAGE_SIZE,
        true,
    ),
];

/// The message-size target of the first of [`LARGE_PAYLOADS`]: large enough
/// that its largest budgeted message, about twice the target, holds a payload
/// past serde's preallocation cap.
const WITHIN_TARGET: usize = 1 << 20;

/// The most a lone record's message holds beyond its payload's encoding: its
/// frame and run heads, at most nine bytes each, and its version, which encodes
/// in under a hundred bytes in the two-party networks here.
const RECORD_FRAMING: usize = 1 << 10;

/// Two forks of the network `network` names, each holding `count` byte-vector
/// messages of `payload` bytes that the other lacks, at target `target` and on
/// a memory budget whose ledger admits work against `budget`.
///
/// Each message's first eight bytes are its index among its side's sends, and
/// the rest its side's number, so every message is distinct.
fn payload_fixture(
    network: NetworkSeed,
    payload: usize,
    count: u64,
    target: usize,
    budget: usize,
) -> Fixture<Vec<u8>> {
    let configure = |peer: Peer<Vec<u8>>| {
        rumors::testing::ledger_budget(peer, budget).target_message_size(target)
    };
    let (built, [a, b]) = measure(|| {
        block_on(async {
            let seed = network.peer::<Vec<u8>>().into_rumors();
            [
                bootstrap_fork_then(&seed, Peer::bootstrap(), configure).await,
                bootstrap_fork_then(&seed, Peer::bootstrap(), configure).await,
            ]
        })
    });
    let payloads = move |side: u8| {
        (0..count).map(move |index| {
            let mut bytes = vec![side; payload];
            bytes[..8].copy_from_slice(&index.to_le_bytes());
            bytes
        })
    };
    let (a_sends, ()) = measure(|| a.send_all(payloads(1)).expect("A's messages encode"));
    let (b_sends, ()) = measure(|| b.send_all(payloads(2)).expect("B's messages encode"));
    let (join, joined) = measure(|| join_snapshots(&a.snapshot(), &b.snapshot()));
    drop(joined);
    let allocations = [built, a_sends, b_sends, join]
        .iter()
        .map(|stats| stats.allocations + stats.reallocations)
        .sum();
    Fixture {
        peers: [a, b],
        sends: [a_sends, b_sends],
        join,
        allocations,
    }
}

/// The allocator events reserved for tracing [`decoding`]: a vector's decoding
/// makes one allocation and one reallocation per doubling of its buffer, a few
/// dozen for any payload here.
const DECODING_EVENTS: usize = 1 << 10;

/// What decoding one `payload`-byte vector from its encoding allocates, as a
/// session decodes a received message's payload: `ciborium` from the encoded
/// bytes, through serde's sequence visitor.
///
/// Returns the encoding's length, the bytes the decoded vector keeps, and the
/// transient peak of the blocks the decoding freed before returning, classified
/// by the same trace the session's blocks are.
fn decoding(payload: usize) -> (usize, usize, usize) {
    let mut encoded = Vec::new();
    ciborium::into_writer(&vec![1u8; payload], &mut encoded).expect("a byte vector encodes");
    let (traced, decoded) = trace(DECODING_EVENTS, || {
        ciborium::from_reader::<Vec<u8>, _>(encoded.as_slice()).expect("the encoding decodes")
    });
    assert_eq!(decoded.len(), payload, "the decoding restores the vector");
    let blocks = traced
        .lifetimes()
        .unwrap_or_else(|incomplete| panic!("raise DECODING_EVENTS: {incomplete:?}"))
        .label(Label::NONE);
    (encoded.len(), blocks.surviving_bytes, blocks.transient_peak)
}

/// Byte-vector payloads larger than serde's preallocation cap, within the run
/// budget and past it, keep each place within its bound, the decoding and the
/// lone record's excess within what the budget documents as outside it.
///
/// The suite's other fixtures carry `u64` payloads at a run target of one
/// comparison, so they never decode a payload past serde's cap or read a run
/// over one payload granule. Here each side sends [`LARGE_PAYLOADS`], each at
/// its own target, and the session is held to the suite's bounds
/// ([`bound_violation`]) with three of them widened, each by what
/// [`Peer::sync_memory_budget`](rumors::Peer::sync_memory_budget) names as held
/// beyond the budget, computed independently of the session:
///
/// - Wire buffering may also hold, in each direction, a lone record's excess
///   over the largest budgeted message at its target
///   ([`budgeted_message_bytes`]), its message taken as its payload's encoding
///   and [`RECORD_FRAMING`].
/// - The decode fans may also hold one message's decoding transient, which
///   [`decoding`] measures by decoding the same payload alone.
/// - Each replica may keep, for each message it gains, what the decoded vector
///   keeps beyond the payload's own length, since serde's decoding grows its
///   buffer by doubling and keeps the capacity it reached, where the sender
///   built the vector at its exact length.
///
/// So that the bounds judge the regions they name, each payload's message must
/// lie within or past the largest budgeted message as [`LARGE_PAYLOADS`] says,
/// each session must have decoded beyond the decode fans' own bound, and a
/// payload past the run budget must have held more on the wire than one
/// budgeted message.
#[test]
fn large_byte_vectors_keep_the_bound() {
    reference();
    for (payload, count, target, past) in LARGE_PAYLOADS {
        let (encoded, kept, transient) = decoding(payload);
        let budgeted = budgeted_message_bytes(target);
        let excess = (encoded + RECORD_FRAMING).saturating_sub(budgeted);
        assert_eq!(
            excess > 0,
            past,
            "a {payload}-byte payload's message, {encoded} B encoded beside {RECORD_FRAMING} B of \
             framing, must lie {} the largest budgeted message at a {target}-byte target, \
             {budgeted} B",
            if past { "past" } else { "within" },
        );
        let run = run_fixture(
            || payload_fixture(BUDGET_NETWORK_SEED, payload, count, target, 0),
            [count; 2],
            release_everything,
        );
        let mut measured = Measured::of(count, 0, 0, run);
        measured.wire = (wire_allowance(target) + 2 * excess) as u128;
        measured.decoded = (DECODE_AHEAD_BYTES + transient) as u128;
        measured.replica_slack = u128::from(count) * (kept - payload) as u128;
        println!("{payload}-byte payloads: {}", measured.report());
        if let Some(violation) = bound_violation(&measured) {
            panic!("{payload}-byte payloads: {violation}");
        }
        for side in Side::BOTH {
            let decoded = measured
                .holding(side, Holding::DecodedLeaves)
                .transient_peak;
            assert!(
                decoded > DECODE_AHEAD_BYTES,
                "{side:?} decoded {payload}-byte payloads within the decode fans' own bound, \
                 {decoded} B, so no decoding transient was judged",
            );
            if excess > 0 {
                let wire = measured.holding(side, Holding::Wire).transient_peak;
                assert!(
                    wire > budgeted,
                    "{side:?} held {wire} B on the wire for {payload}-byte payloads past the run \
                     budget, no more than one budgeted message, so no lone record's excess was \
                     judged",
                );
            }
        }
    }
}
