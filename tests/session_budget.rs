//! The session memory budget against the transient heap a session
//! allocates, under the most adverse link and data this suite constructs.
//!
//! [`Peer::sync_memory_budget`](rumors::Peer::sync_memory_budget) caps the
//! memory one session uses beyond the replica. This suite measures that
//! claim directly: with budget `B`, the heap a session allocates
//! *transiently*, meaning allocations made during the session that do not
//! survive into the replica, stays near `B`. The replica's own growth, the
//! messages the session supplies, is outside the claim.
//!
//! # The measurement
//!
//! Each test drives two peers through one session on this thread, over the
//! held link of the [`link`] module: every byte written waits until both
//! peers stall, so each side fills whatever its admission lets it have
//! outstanding before any reply arrives, and each release hands the reader
//! everything its peer produced meanwhile. At each stall the schedule can
//! also hold back kinds of either side's tasks, as a slow storage backend
//! would: walk stages, so replies park while the questions they answer stay
//! queued, or assemblers, so resolutions pile up. An [`alloc_meter::trace`]
//! records every allocation and free over the session, and the driver
//! labels each allocation with the peer whose poll made it.
//!
//! After the session's state is dropped, the trace classifies every block
//! the session allocated. A block still live is replica content. A block
//! freed was transient. Each peer's *transient peak* is the highest number of
//! its transient bytes live at once, which is the quantity the claim bounds.
//! A block allocated before the session is neither, even when the session
//! frees it: the replica's old nodes, which the session holds through its
//! starting root until it publishes the merged tree, were allocated at rest
//! and count toward the replica at rest. The link's buffers and the
//! driver's own state are allocated outside the trace, so they count toward
//! nothing, and every block the trace records is some peer's.
//!
//! Three checks hold the survivors to being replica content. The peers are
//! dropped while a snapshot of each replica keeps it alive, and then the
//! snapshots: every survivor must be freed by one of the two, so none
//! belongs to a leak or a global cache that would hide transient bytes. A
//! survivor the peer frees while its replica lives is session state the
//! peer kept, which may be no more than a session that compares almost
//! nothing leaves (see [the reference](#the-reference-session)). The
//! survivors the replica keeps may be no more than building the merged
//! replica directly allocates: the peer's messages as its own sends built
//! them, and the interior a join of the two replicas builds. And each
//! peer's survivors must number at least the messages it gained, since every
//! gained message becomes at least one new block of its replica, so a trace
//! that stopped seeing the session cannot pass.
//!
//! # The data
//!
//! The budget binds when two large replicas differ throughout. A peer
//! catching up from nothing disputes no subtree: it receives the set as
//! supplies, and with nothing disputed the budget barely binds the session,
//! so each fixture is two forks of one network holding equal, disjoint message
//! sets.
//!
//! A leaf's tree path is the hash of its version, so the suite cannot place
//! leaves: two hashed sets of `n` messages each interleave as uniformly
//! random keys do. At 100,000 messages per side, every depth-1 node and
//! most depth-2 nodes hold children from both replicas, and deeper nodes
//! rarely do. Placing leaves so that deeper nodes also hold both would mean
//! searching versions for chosen hash prefixes, at a cost of `256^k`
//! versions per shared byte `k`, and would leave the uniform-hash model,
//! under which the crate's guarantees are stated. Equal, fully divergent
//! hashed sets are the most adverse fixture within that model.
//!
//! Both peers set their run target to [`COMPARISON_FRAME_BYTES`], the
//! largest frame that carries one comparison, so no frame of the session is
//! larger and the wire's allowance below is a few frames' worth.
//!
//! # Attribution
//!
//! The session marks the code that allocates for each place it keeps state
//! in ([`Holding`]), and this suite labels each block with the peer and the
//! place its allocation served, so every place's peak sits beside the bound
//! that covers it. An allocation counts toward the place whose code made
//! it: a question's child
//! listing where the walk explodes the node it asks about, a parked reply
//! where the decoder assembles it. Places the ledger leaves out are labelled
//! too: wire buffering, decode fans, and assembly. Blocks no marked region allocated, the session's tasks and
//! setup, are *unattributed*.
//!
//! Each label's peak is its own, so the labels' peaks need not occur
//! together; each side's transient peak is the peak of all its labels at
//! once.
//!
//! # The bounds
//!
//! Every label has a bound, and each side's whole transient peak must stay
//! within their sum: every byte the session holds at once is some label's,
//! and no label holds more than its own peak.
//!
//! The places the ledger charges ([`CHARGED`]) are bounded together, by
//! the ledger: at every allocation event their live bytes may exceed what
//! the ledger holds by at most the scratch its stages fill before they
//! charge what they built, and the ledger holds at most its budget beside
//! its floor slots' largest grants ([the ledger](#the-ledger)).
//!
//! The scratch is what a stage, encoder, or decoder fills for a place while
//! it builds items it charges only once built: an answer's plan, a chunk's
//! reactions before its question is admitted, a decoder's ready reactions,
//! a take's derived scopes. Each place is owed its own scratch for each of
//! the [`SESSION_STREAMS`] logical streams, computed from the types it
//! covers ([`ledger_scratch`]).
//!
//! The classes the ledger leaves out, the session's fixed state, are
//! bounded by their capacities:
//!
//! - **Wire buffering** holds at most three frames for each logical stream
//!   a side sends on and two for each it receives on, and one more on the
//!   receiving side ([`WIRE_FRAMES`]), each at most
//!   [`COMPARISON_FRAME_BYTES`] here and counted at twice that for its
//!   vector.
//! - **Assembly** holds the queues of fixed capacity around it and the
//!   in-memory backend's build of each supplied subtree, which keeps only
//!   the subtree's open right spine however large it is
//!   ([`assembly_allowance`] derives the bound from both, independent of
//!   the replicas' sizes).
//! - **Decode fans** hold at most a fan of decoded leaves for each logical
//!   stream a side receives on.
//! - **The unattributed blocks** are the session's fixed overhead: the
//!   tasks, channels, and buffers it sets up for every logical stream and
//!   tree level, and its greeting's root fan, independent of the budget and
//!   of the sets' sizes once the root's fan is full. Admission's own
//!   bookkeeping ([`Holding::Admission`]) is fixed overhead too: each walk
//!   stage waits on one unit at a time, and a floor slot allocates nothing.
//!   Together they may be no more than [the reference
//!   session](#the-reference-session)'s whole transient peak.
//!
//! Each side must also copy at least one node, so that an exploded-node
//! label that stopped counting cannot pass its bound vacuously. Wherever a
//! prefix holds exactly one of a side's leaves and some of its peer's, the
//! merged tree holds that leaf at least a byte below where the side's own
//! tree stores it, and only peeling a copy of the stored node builds it. At
//! the suite's smallest size, 10,000 messages a side, about 1,200 depth-2
//! prefixes alone are such places.
//!
//! Before each node it explodes, a session reads its backend's account of
//! what the explosion will allocate, which is what it can charge for an
//! explosion before making it, and reports it to this suite. Each side's
//! reports must sum to exactly the bytes its exploded-node label allocated.
//! The walk charges each of those explosions to its ledger before making
//! it, so each side's charges to the exploded nodes' place must sum to the
//! same bytes: the sum of every rise in that place's account. The one
//! explosion no charge covers, the deletion filter's below a node whose
//! leaves its peer partly knows, never happens here, since neither side's
//! version knows a leaf of the other's.
//!
//! # The ledger
//!
//! A peer's memory budget sets aside the floor allowance for its ledger's
//! floor slots, the most they can hold at once, and the ledger admits work
//! beyond them against the rest. Every budget this suite names is that
//! rest, what the ledgers admit against: each peer's memory budget is the
//! named budget beside the floor allowance
//! ([`admitting`]), so that the suite's fixtures admit work as their
//! names say.
//!
//! Each side keeps a ledger of what its session holds and of the room it
//! sets aside for the replies its questions invite, and admits its walk's
//! work against its budget. The suite holds every session it measures to
//! three identities:
//!
//! - **Coverage.** At every allocation event, and for each place it charges
//!   ([`CHARGED`]), the ledger's charges for the place are at least the
//!   place's live transient bytes, less the place's own scratch
//!   ([`ledger_scratch`]), and so in total too. The trace's position at each
//!   ledger change lines the two up, event by event; checking each place
//!   alone keeps room set aside in one place from hiding a charge missing in
//!   another.
//! - **Fit.** Every reply a stage finishes held no more reactions, and its
//!   queries no more listing entries, than its question reserved room for.
//! - **Zero.** Each side's ledger holds nothing once the session ends.
//!
//! It also holds each ledger to its bound: at most the budget outside its
//! floor slots, at most the peer's memory budget in all, and every item
//! built within its unit's grant. Each ledger must also have
//! counted, some reply must have finished, and some unit must have waited
//! for admission, so that none of the checks passes vacuously. The ledgers'
//! peaks in the fixed fixtures are committed as baselines: what each side
//! held and set aside at once. The ledger peaks agree between macOS and
//! illumos, but the heap peaks do not, so no heap peak is committed.
//! Instead, a held-back fixture shows that it raised a heap peak by running
//! the same session with nothing held back, in the same test, and comparing
//! the two.
//!
//! # The reference session
//!
//! Before its fixture, each test measures a session between two disjoint
//! replicas of [`REFERENCE_MESSAGES`] messages each at a budget of zero.
//! Its root's fan is full and it sets up every logical stream, so its
//! setup is the setup of any larger session, and its transient peak, that
//! setup and its one-question-at-a-time work together, bounds the setup of the
//! session under test. Its peers also show what a session leaves in the
//! peer beyond the replica.
//!
//! # Upper bounds only
//!
//! The suite asserts no lower bound on what a session holds: replicas that
//! agree from the start stop at the root and hold almost nothing, whatever
//! the budget, so no floor proportional to `B` would mean anything.
//!
//! An upper bound means something only where the budget constrains the
//! session; on a session it never constrains, every bound would pass
//! without judging the budget. So each test also checks that it does: some
//! unit of the session's work waited for admission. Each side's session
//! report must count exactly the units its ledger counted waiting
//! ([`SessionStats::budget_waits`]), the user's view of the same constraint.

#[path = "session_budget/link.rs"]
mod link;

use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use alloc_meter::{
    Event, EventKind, Label, LabelLifetimes, Stats, labelled, measure, trace, untraced,
};
use link::{Drive, Held, Link, Portion, Release, Side, drive, polling, prepare_pacing};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::test_runner::TestRunner;
use rumors::testing::{
    COMPARISON_FRAME_BYTES, Holding, LedgerChange, PLACES, ReplyFit, SESSION_STREAMS,
    assembly_allowance, attribute_allocations, join_snapshots, ledger_scratch, observe_fits,
    observe_ledgers, report_allocations, supply_decode_envelope_bytes,
};
use rumors::{Peer, Rumors, SessionStats};
use rumors_testkit::bench::session::admitting;
use rumors_testkit::common::wire::{block_on, bootstrap_fork_then};

/// Messages each side of the per-budget fixtures holds that the other
/// lacks: large enough that units still wait for admission at
/// [`WIDE_BUDGET`].
const MESSAGES: u64 = 100_000;

/// Messages per side for the schedule search, small enough that each case
/// costs a few seconds.
const SEARCH_MESSAGES: u64 = 10_000;

/// The schedule search's budget, 2 MiB, at which its sessions' units wait
/// for admission, so their bounds are judged where the budget constrains
/// them.
const SEARCH_BUDGET: usize = 2 << 20;

/// The wide fixtures' budget, 32 MiB, four times the near-floor fixture's,
/// at which units still wait for admission at [`MESSAGES`] per side, so the
/// queued work it admits, rather than the fixed work near the root,
/// dominates.
const WIDE_BUDGET: usize = 32 << 20;

/// Messages each side of [the reference session](self#the-reference-session)
/// holds: enough that its root has a full fan of children.
const REFERENCE_MESSAGES: u64 = SEARCH_MESSAGES;

/// Trace events reserved per message the session moves, in both
/// directions together, beside [`SETUP_EVENTS`].
///
/// A session makes fewer than a hundred allocator events per moved message
/// at these sizes, and one that outgrows the reservation fails with the
/// count it needed. Systems that do not overcommit memory, such as illumos,
/// commit a reservation up front, so it is sized near what a session uses.
const EVENTS_PER_MESSAGE: usize = 128;

/// Trace events reserved for a session's setup, whatever its size.
const SETUP_EVENTS: usize = 1 << 16;

/// The most frames one side's wire buffering holds at once.
///
/// Sending, each logical stream holds at most three: one its encoder is
/// waiting to hand over, one in its channel to the multiplexer, and one the
/// multiplexer has staged or is writing. Receiving, each holds at most two,
/// one in its channel from the demultiplexer and one its decoder is
/// decoding, and the demultiplexer holds one more for the whole direction
/// while it waits for a full channel.
const WIRE_FRAMES: usize = 3 * SESSION_STREAMS + 2 * SESSION_STREAMS + 1;

/// What one fixture's construction allocated, and the peers it built.
struct Fixture {
    /// The two peers, `A` then `B`.
    peers: [Rumors<u64>; 2],
    /// What each side's sends allocated, in [`Side::BOTH`] order: building
    /// its own messages into its replica.
    sends: [Stats; 2],
    /// What joining the two replicas in memory allocated beyond them: the
    /// merged tree's new interior.
    join: Stats,
    /// Every allocation the fixture made, an upper bound on the blocks
    /// either replica holds before the session.
    allocations: usize,
}

/// Builds two forks of one fresh network, holding `messages` messages, in
/// [`Side::BOTH`] order, that the other lacks.
///
/// Each fork runs on the suite's run target and on a memory budget whose
/// ledger admits work against `budget` ([`admitting`]).
fn fixture(messages: [u64; 2], budget: usize) -> Fixture {
    let configure = |peer: Peer<u64>| {
        peer.sync_memory_budget(admitting(budget))
            .target_message_size(COMPARISON_FRAME_BYTES)
    };
    let (built, [a, b]) = measure(|| {
        block_on(async {
            let seed = Peer::<u64>::seed().into_rumors();
            [
                bootstrap_fork_then(&seed, Peer::bootstrap(), configure).await,
                bootstrap_fork_then(&seed, Peer::bootstrap(), configure).await,
            ]
        })
    });
    let [a_messages, b_messages] = messages;
    let (a_sends, ()) = measure(|| a.send_all(0..a_messages).expect("A's messages encode"));
    let (b_sends, ()) = measure(|| {
        b.send_all((0..b_messages).map(|value| 1_000_000 + value))
            .expect("B's messages encode")
    });
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
    /// Each side's sum of the bytes its backend reported its explosions
    /// would allocate.
    reported_explosions: [usize; 2],
    /// Each side's ledger, held against the places it charges.
    ledgers: [LedgerReport; 2],
    /// The replies the session's stages finished, and those that held more
    /// than their questions reserved for.
    fits: Fits,
}

/// One side's ledger over a session, against the transient heap of the
/// places it charges ([`CHARGED`]) and against its budget.
#[derive(Clone, Copy, Debug, Default)]
struct LedgerReport {
    /// The ledger changes the side made.
    changes: usize,
    /// Bytes charged to the exploded nodes' place over the session: the
    /// sum of every rise in its account.
    exploded_charged: usize,
    /// The most the ledger held at once.
    peak: usize,
    /// The most the ledger held outside its floor slots at once: what its
    /// budget bounds.
    normal_peak: usize,
    /// The most the ledger held beyond its peer's memory budget, the
    /// budget it admits against beside the floor allowance, at any change:
    /// zero when the session holds at most its memory budget throughout.
    over_bound: usize,
    /// What the ledger held when the session ended.
    left: usize,
    /// Bytes the side's items needed beyond their grants.
    short: u64,
    /// Units that waited for admission.
    waited: u64,
    /// The sum of the floor slots' largest grants, at the session's end.
    floor_bound: usize,
    /// The most the charged places' live transient bytes exceeded the
    /// ledger, at any allocation event.
    uncovered: usize,
    /// For each charged place, in [`CHARGED`] order, the most its live
    /// transient bytes exceeded the ledger's charges for it.
    uncovered_by_place: [usize; CHARGED.len()],
    /// For each charged place, the most its live transient bytes reached
    /// as a share of the ledger's charges for it, in thousandths, where
    /// those charges were at least a page: how tight the place's prices
    /// run.
    tightest: [usize; CHARGED.len()],
}

/// The replies one session's stages finished, against their reservations.
#[derive(Clone, Debug, Default)]
struct Fits {
    /// Replies finished.
    replies: usize,
    /// Those that held more than their reservation allowed.
    over: Vec<ReplyFit>,
}

/// The most the charged places' transient heap may exceed a side's ledger
/// in total: every charged place's scratch ([`ledger_scratch`]).
fn ledger_slack() -> usize {
    CHARGED.iter().map(|&holding| ledger_scratch(holding)).sum()
}

/// The places the ledger charges: everything a session holds but its
/// fixed state (wire buffering, decode fans, assembly) and its setup.
const CHARGED: [Holding; 6] = [
    Holding::Questions,
    Holding::Resolutions,
    Holding::QuestionRecords,
    Holding::ParkedReplies,
    Holding::OutgoingReplies,
    Holding::ExplodedNodes,
];

/// One ledger change, where the trace stood when it happened.
#[derive(Clone, Copy, Debug)]
struct LedgerEvent {
    /// The index the trace's next event took.
    position: usize,
    /// The ledger that changed.
    ledger: usize,
    /// What the ledger held after the change.
    held: usize,
    /// What it held for each place after the change, in [`Holding::ALL`]
    /// order.
    places: [usize; PLACES],
    /// What charges built out of floor grants held after the change.
    floor: usize,
    /// What its floor slots carried for charges admitted against the
    /// budget after the change.
    carried: usize,
    /// Its floor slots' largest grants so far, summed.
    floor_bound: usize,
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
    static FITS: RefCell<Fits> = const { RefCell::new(Fits { replies: 0, over: Vec::new() }) };
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
        floor: change.floor,
        carried: change.carried,
        floor_bound: change.floor_bound,
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
            if fit.reactions > fit.reaction_bound || fit.listing > fit.listing_bound {
                fits.over.push(fit);
            }
        });
    });
}

/// Each side's ledger report from a session's ledger changes and its
/// trace, each side's ledger admitting against `budget`.
fn ledger_reports(
    events: &[LedgerEvent],
    session: &alloc_meter::Trace,
    budget: usize,
) -> [LedgerReport; 2] {
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
        let exploded = Holding::ALL
            .iter()
            .position(|other| *other == Holding::ExplodedNodes)
            .expect("ALL lists every holding");
        // Each event follows one change, so each rise is one charge.
        let (exploded_charged, _) = mine.iter().fold((0, 0), |(sum, before), event| {
            let now = event.places[exploded];
            (sum + now.saturating_sub(before), now)
        });
        LedgerReport {
            changes: mine.len(),
            exploded_charged,
            peak: mine.iter().map(|event| event.held).max().unwrap_or(0),
            normal_peak: mine
                .iter()
                .map(|event| event.held - event.floor - event.carried)
                .max()
                .unwrap_or(0),
            over_bound: mine
                .iter()
                .map(|event| event.held.saturating_sub(admitting(budget)))
                .max()
                .unwrap_or(0),
            left: last.map_or(0, |event| event.held),
            short: last.map_or(0, |event| event.short),
            waited: last.map_or(0, |event| event.waited),
            floor_bound: last.map_or(0, |event| event.floor_bound),
            uncovered,
            uncovered_by_place,
            tightest,
        }
    })
}

/// Each side's running sum of the explosion bytes its backend reports, in
/// [`Side::BOTH`] order, for the session being run.
static REPORTED_EXPLOSIONS: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];

/// Adds a report the session makes while the driver polls a side to that
/// side's sum; a report outside a poll belongs to no session.
fn report(holding: Holding, bytes: usize) {
    if holding == Holding::ExplodedNodes
        && let Some(side) = polling()
    {
        REPORTED_EXPLOSIONS[side.index()].fetch_add(bytes, Ordering::Relaxed);
    }
}

/// Runs one session between two fully divergent peers holding `messages`
/// messages, in [`Side::BOTH`] order, both on `budget`, over the held link
/// released by `schedule`, and classifies what it allocated.
///
/// # Panics
///
/// Panics if the session fails, if a trace overflows its reservation, or if
/// a surviving block outlives both its peer and its replica.
fn run_session(
    messages: [u64; 2],
    budget: usize,
    schedule: impl FnMut([usize; 2]) -> Release,
) -> Run {
    assert!(
        attribute_allocations(attribute),
        "this suite's hook is the process's attribution hook"
    );
    assert!(
        report_allocations(report),
        "this suite's hook is the process's report hook"
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
    } = fixture(messages, budget);
    let link = Link::new();
    prepare_pacing();
    for sum in &REPORTED_EXPLOSIONS {
        sum.store(0, Ordering::Relaxed);
    }
    let (mut a_read, mut a_write) = (link.reader(Side::A), link.writer(Side::A));
    let (mut b_read, mut b_write) = (link.reader(Side::B), link.writer(Side::B));
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
    let ledgers = LEDGER_EVENTS.with(|events| ledger_reports(&events.borrow(), &session, budget));
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
        reported_explosions: REPORTED_EXPLOSIONS
            .each_ref()
            .map(|sum| sum.load(Ordering::Relaxed)),
        ledgers,
        fits,
    }
}

/// The reference session (see [the module docs](self#the-reference-session)),
/// measured once per process.
fn reference() -> &'static Run {
    static REFERENCE: OnceLock<Run> = OnceLock::new();
    REFERENCE.get_or_init(|| {
        run_session([REFERENCE_MESSAGES; 2], 0, |_| {
            Release::bytes([Portion::All; 2])
        })
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
    /// The bound on assembly.
    assembly: u128,
    /// The bound on the unattributed blocks and admission's bookkeeping
    /// together: the reference session's larger transient peak.
    fixed: u128,
    /// The session under test.
    run: Run,
}

impl Measured {
    /// Measures the session [`run_session`] runs, beside the bounds it is
    /// judged against.
    fn new(messages: u64, budget: usize, schedule: impl FnMut([usize; 2]) -> Release) -> Self {
        let fixed = Side::BOTH
            .map(|side| reference().sides[side.index()].transient_peak)
            .into_iter()
            .max()
            .expect("a session has two sides") as u128;
        let run = run_session([messages; 2], budget, schedule);
        Self {
            messages,
            budget,
            wire: (WIRE_FRAMES * 2 * COMPARISON_FRAME_BYTES) as u128,
            assembly: assembly_allowance() as u128,
            fixed,
            run,
        }
    }

    /// The highest transient peak one side's allocations for `holding` may
    /// reach, for each class the ledger leaves out.
    ///
    /// The places the ledger charges ([`CHARGED`]) are bounded together, by
    /// the ledger's own bound ([`Self::charged_bound`]), and admission's
    /// bookkeeping together with the unattributed blocks ([`Self::fixed`]).
    ///
    /// Each class the ledger leaves out may reach its allowance; the module
    /// docs derive every bound.
    fn holding_bound(&self, holding: Holding) -> Option<u128> {
        match holding {
            Holding::DecodedLeaves => Some(2 * supply_decode_envelope_bytes() as u128),
            Holding::Wire => Some(self.wire),
            Holding::Assembly => Some(self.assembly),
            _ => None,
        }
    }

    /// The highest transient peak `side`'s charged places may reach
    /// together.
    ///
    /// That is its ledger's bound, the budget and its floor slots' largest
    /// grants, beside the scratch its stages fill before they charge what
    /// they built ([`ledger_slack`]).
    fn charged_bound(&self, side: Side) -> u128 {
        let ledger = self.run.ledgers[side.index()];
        (self.budget as u128) + (ledger.floor_bound as u128) + (ledger_slack() as u128)
    }

    /// The highest transient peak `side`'s allocations may reach in all:
    /// its charged places' bound, every other holding's, and the bound on
    /// the unattributed blocks and admission's bookkeeping together.
    fn side_bound(&self, side: Side) -> u128 {
        Holding::ALL
            .iter()
            .filter_map(|&holding| self.holding_bound(holding))
            .sum::<u128>()
            + self.charged_bound(side)
            + self.fixed
    }

    /// The most bytes of survivors `side`'s replica may keep: the bytes the
    /// peer's messages took when its own sends built them, and the interior
    /// a join of the two replicas builds.
    fn replica_bound(&self, side: Side) -> u128 {
        let built = |stats: Stats| u128::try_from(stats.net_bytes).unwrap_or(0);
        built(self.run.sends[side.peer().index()]) + built(self.run.join)
    }

    /// A side's classified blocks.
    fn side(&self, side: Side) -> LabelLifetimes {
        self.run.sides[side.index()]
    }

    /// A side's session report.
    fn stats(&self, side: Side) -> SessionStats {
        self.run.stats[side.index()]
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
                    " {holding:?} gap {} B of {} B scratch, tightest {}‰;",
                    ledger.uncovered_by_place[place],
                    ledger_scratch(*holding),
                    ledger.tightest[place],
                ));
            }
            line.push_str(&format!(
                " total gap {} B of {} B",
                ledger.uncovered,
                ledger_slack()
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

/// Runs a region the session attributes to `holding` under the label of
/// that holding on the side being polled.
///
/// Outside a poll the region runs under whatever label is current: the
/// session allocates only while the driver polls it.
fn attribute(holding: Holding, region: &mut dyn FnMut()) {
    match polling() {
        Some(side) => labelled(side.holding_label(holding), region),
        None => region(),
    }
}

/// Returns why a measured session falls outside a bound or fails to show
/// that the budget constrained it, or `None` if it passes.
fn violation(measured: &Measured) -> Option<String> {
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
        let copies = measured.holding(side, Holding::ExplodedNodes);
        if copies.transient_blocks + copies.surviving_blocks == 0 {
            return Some(format!(
                "{side:?} copied no node, so its exploded-node label stopped counting: {report}"
            ));
        }
        let reported = measured.run.reported_explosions[side.index()];
        let allocated = copies.transient_bytes + copies.surviving_bytes;
        if reported != allocated {
            return Some(format!(
                "{side:?}'s backend reported {reported} B of explosions where they allocated \
                 {allocated} B: {report}"
            ));
        }
        let charged = measured.run.ledgers[side.index()].exploded_charged;
        if charged != reported {
            return Some(format!(
                "{side:?}'s ledger was charged {charged} B for explosions its backend reported \
                 as {reported} B: {report}"
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
    if Side::BOTH
        .into_iter()
        .all(|side| measured.run.ledgers[side.index()].waited == 0)
    {
        return Some(format!("no unit ever waited for admission: {report}"));
    }
    None
}

/// Returns why a session's ledgers fail one of their three accounting
/// identities or their bound, or `None` if they hold.
///
/// Each side's ledger must have counted, must cover the transient heap of
/// each place it charges, less that place's scratch, at every allocation
/// event, and must return to zero once the session ends; every reply the
/// session's stages finished must fit the room its question reserved for
/// it, with at least one reply finished. And each side's ledger, admitting
/// against `budget`, must hold at most the budget outside its floor slots
/// and at most its peer's memory budget in all, the budget beside the
/// floor allowance, with every item built within its unit's grant.
fn ledger_violation(run: &Run, budget: usize) -> Option<String> {
    for side in Side::BOTH {
        let ledger = run.ledgers[side.index()];
        if ledger.changes == 0 {
            return Some(format!(
                "{side:?}'s ledger never changed, so it stopped counting"
            ));
        }
        if ledger.normal_peak > budget {
            return Some(format!(
                "{side:?}'s ledger held {} B outside its floor slots, over its {budget} B \
                 budget: {ledger:?}",
                ledger.normal_peak,
            ));
        }
        if ledger.over_bound > 0 {
            return Some(format!(
                "{side:?}'s ledger exceeded its memory budget of {} B by {} B: {ledger:?}",
                admitting(budget),
                ledger.over_bound,
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

/// Measures one fully divergent session at `budget` under `schedule`,
/// prints its report, and asserts it passes the bounds and `reached`, which
/// returns why the session failed to reach the state its test targets.
fn assert_budget_holds(
    budget: usize,
    schedule: impl FnMut([usize; 2]) -> Release,
    reached: impl Fn(&Measured) -> Option<String>,
) {
    let measured = Measured::new(MESSAGES, budget, schedule);
    println!("{}", measured.report());
    if let Some(violation) = violation(&measured).or_else(|| reached(&measured)) {
        panic!("{violation}");
    }
}

/// Releases every held byte at every stall and holds back no task.
fn release_everything(_: [usize; 2]) -> Release {
    Release::bytes([Portion::All; 2])
}

/// Each side's ledger peak, in [`Side::BOTH`] order, in the session at
/// 8 MiB.
///
/// Admission fills each side's budget with work, and the floor slots hold
/// one more unit per depth and kind beside it: each peak is the 8 MiB
/// budget plus at most its slots' largest grants.
const NEAR_FLOOR_LEDGER_PEAKS: [usize; 2] = [9_888_920, 8_478_688];

/// Each side's ledger peak, in [`Side::BOTH`] order, in the session at
/// [`WIDE_BUDGET`]: the budget filled with admitted work, beside the floor
/// slots' grants, as at 8 MiB.
const WIDE_LEDGER_PEAKS: [usize; 2] = [34_401_492, 33_597_284];

/// Each side's ledger peak, in [`Side::BOTH`] order, when side `A` joins
/// from empty.
///
/// It asks one question, about the root, and the responder answers it with
/// supplies alone, so neither side holds more than that one question and
/// that one answer.
const JOINING_LEDGER_PEAKS: [usize; 2] = [12_872, 18_648];

/// Targets a session whose ledgers peak at exactly `expected`, the
/// fixture's committed baseline.
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

/// At 8 MiB, just above the floor, each place a peer's session keeps
/// in-flight state stays within its bound, and each peer's whole transient
/// peak stays within their sum.
///
/// At 8 MiB most of the session's units wait for admission at [`MESSAGES`]
/// per side, and the fixed work near the root, which the budget does not
/// shrink, weighs most against it.
#[test]
fn budget_near_the_floor_bounds_the_transient_peak() {
    assert_budget_holds(
        8 << 20,
        release_everything,
        ledger_peaks(NEAR_FLOOR_LEDGER_PEAKS),
    );
}

/// At [`WIDE_BUDGET`], each place a peer's session keeps in-flight state
/// stays within its bound, and each peer's whole transient peak stays
/// within their sum.
///
/// Units still wait for admission, and the queued work the budget admits,
/// rather than the fixed work near the root, dominates the session's
/// transient heap. Its ledger peaks are committed baselines.
#[test]
fn wide_budget_bounds_the_transient_peak() {
    assert_budget_holds(
        WIDE_BUDGET,
        release_everything,
        ledger_peaks(WIDE_LEDGER_PEAKS),
    );
}

/// At [`WIDE_BUDGET`], with one side's walk stages held back while any
/// bytes are in flight, every label and each peer's whole transient peak
/// stay within their bounds.
///
/// The held side cannot take the replies its decoders park, nor send more
/// questions, while its peer answers every question it already sent, and
/// the answers park beside the questions the walk still holds: the state
/// the reservations for parked replies cover. The held side changes only at
/// a stall with nothing left to deliver, which also releases its stages, so
/// each side's parking fills in turn from every question its budget
/// admitted.
///
/// The test checks that the state was reached: each side's walk was held
/// back, and each side's parked replies peaked higher than in the same
/// session with nothing held back, which the test runs first.
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

/// At [`WIDE_BUDGET`], with assembly held back whenever a stall has bytes
/// to release, each place stays within its bound, and each peer's whole
/// transient peak stays within their sum.
///
/// Held-back assemblers consume no resolutions, so resolutions pile up
/// while the walk keeps asking until admission or its own sends stop it,
/// at every depth at once. Assembly runs only at stalls with nothing left
/// to deliver.
///
/// The test checks that the state was reached: each side's assembly was
/// held back, and each side's resolutions peaked higher than in the same
/// session with nothing held back, which the test runs first.
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
    let unheld = Measured::new(MESSAGES, WIDE_BUDGET, release_everything);
    Side::BOTH.map(|side| unheld.holding(side, holding).transient_peak)
}

/// Returns why the session failed to hold back the tasks `refused` counts
/// on each side, or to raise each side's `holding` peak above its peak in
/// the same session with nothing held back, `unheld`, or `None` if it did
/// both.
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

/// Generates one stall's release: a portion of each direction's held bytes
/// and the task kinds each side holds back.
fn arb_release() -> impl Strategy<Value = Release> {
    ([arb_portion(), arb_portion()], [arb_held(), arb_held()])
        .prop_map(|(bytes, held)| Release { bytes, held })
}

/// Cases [`any_release_schedule_keeps_the_bound`] runs: each builds two
/// [`SEARCH_MESSAGES`]-message replicas and runs a session, a few seconds
/// of work, so the count is fixed rather than raised with CI's.
const SCHEDULE_CASES: u32 = 12;

/// Shrink iterations [`any_release_schedule_keeps_the_bound`] spends on a
/// failure, for the same reason.
const SCHEDULE_SHRINK_ITERS: u32 = 24;

/// Under any release schedule, each place a peer's session keeps in-flight
/// state stays within its bound, and each peer's whole transient peak stays
/// within their sum.
///
/// Units wait for admission.
///
/// At each stall the schedule releases all, a prefix, or none of each
/// direction's held bytes, and holds back any kinds of each side's tasks
/// until the next stall, cycling through the generated releases. So
/// deliveries split frames at arbitrary points, either direction can lag
/// the other, and any of a side's walk stages, assemblers, encoders, and
/// decoders can stall while the rest run.
///
/// The property runs exactly [`SCHEDULE_CASES`] cases. Its runner is built
/// here rather than by the `proptest!` macro, which would let
/// `PROPTEST_CASES` replace the configured count; it keeps the macro's
/// failure persistence by naming this file.
#[test]
fn any_release_schedule_keeps_the_bound() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: SCHEDULE_CASES,
        max_shrink_iters: SCHEDULE_SHRINK_ITERS,
        source_file: Some(file!()),
        ..ProptestConfig::default()
    });
    let result = runner.run(&vec(arb_release(), 1..=8), |releases| {
        let mut next = releases.iter().cycle();
        let measured = Measured::new(SEARCH_MESSAGES, SEARCH_BUDGET, |_| {
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

/// When one replica joins from empty, the side receiving every message as
/// whole supplied subtrees holds no more in assembly than the constant
/// bound, whatever the subtrees' size.
///
/// The peer holds [`MESSAGES`] messages, so each subtree supplied below the
/// root holds about four hundred leaves, and the receiving side assembles
/// every one. The bound ([`assembly_allowance`]) counts each of the
/// [`SESSION_STREAMS`] reply decoders assembling one subtree at a time, each
/// holding only its open right spine.
///
/// Each subtree of two or more leaves leaves at least one branch in the
/// replica, so the receiver's assembly must leave at least one surviving
/// block per root child of its peer's replica: a trace that stopped seeing
/// assembly cannot pass. That floor rests on every one of the peer's 256
/// root children holding two or more of its messages, which fails at this
/// size with probability below 2⁻⁴⁰⁰.
#[test]
fn joining_from_empty_bounds_assembly() {
    let run = run_session([0, MESSAGES], WIDE_BUDGET, release_everything);
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
