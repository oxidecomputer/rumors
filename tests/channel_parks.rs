//! How many sends on each kind of protocol edge park their sender, on one
//! fixed session.
//!
//! Every edge between a session's stages is a bounded `futures-channel`
//! channel, and that crate parks a sender on the send that *fills* its
//! channel: it boxes a node holding the sender's task, takes the task's
//! lock, and wakes the task when the receiver next takes an item. A send
//! that leaves room behind it pays only for its item's node. So how often
//! sends fill their edges sets much of what a handover between stages
//! costs. This suite counts sends, sends that found their edge full, and
//! sends that parked, per kind of edge, through the process's send hook
//! ([`observe_edge_sends`]) on one deterministic session.
//!
//! The session runs each peer in its own task on a current-thread runtime,
//! as an application runs each peer in its own task. Both tasks run on the
//! thread that drives the runtime, so the hook adds each report to that
//! thread's tally, which no session on another test thread can reach, and
//! every receiver runs on the sending thread, where the hook's reading of a
//! park is exact.

use std::cell::RefCell;
use std::collections::BTreeMap;

use rumors::testing::{
    EdgeSend, MEMORY_CAPACITY, MemoryEnd, QueueKind, memory_connection, observe_edge_sends,
};
use rumors::{DEFAULT_SYNC_MEMORY_BUDGET, Rumors};
use rumors_testkit::bench::session::{Shape, pair};

/// What the sends on one kind of edge did, summed over both sides.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Sends {
    /// Completed sends.
    sent: u64,
    /// Sends that found the edge full and waited for room.
    waited: u64,
    /// Sends that filled the edge and so parked their sender.
    parked: u64,
}

/// One session's tally beside the frames both sides' session statistics
/// report sending.
#[derive(Debug)]
struct Session {
    /// What the hook reported, per kind of edge.
    tally: BTreeMap<QueueKind, Sends>,
    /// The sum of both sides' [`frames_sent`](rumors::SessionStats::frames_sent).
    frames_sent: u64,
}

// On targets without native thread-local storage (illumos among the gate's),
// `thread_local!` expands through the OS-keyed path, and clippy's
// `missing_const_for_thread_local` misreads that expansion, reporting the
// `const` block below as missing. The allow is scoped to that target so the
// lint keeps guarding everywhere else.
thread_local! {
    /// The tally of the session this thread is running.
    #[cfg_attr(target_os = "illumos", allow(clippy::missing_const_for_thread_local))]
    static TALLY: RefCell<BTreeMap<QueueKind, Sends>> = const { RefCell::new(BTreeMap::new()) };
}

/// Add one send to this thread's tally.
fn record(send: EdgeSend) {
    TALLY.with(|tally| {
        let mut tally = tally.borrow_mut();
        let sends = tally.entry(send.kind).or_default();
        sends.sent += 1;
        sends.waited += u64::from(send.waited);
        sends.parked += u64::from(send.parked);
    });
}

/// Gossip once over `end`, returning the frames this side sent.
async fn gossip(peer: Rumors<String>, end: MemoryEnd) -> u64 {
    let (gossiped, ..) = peer
        .gossip_once(end.read, end.write)
        .await
        .expect("an in-memory session completes");
    gossiped.stats.frames_sent
}

/// Reconcile the fixed pair once, each peer in its own task, and return what
/// the session's sends did.
///
/// The pair is two peers of one deterministic network that share 1,000
/// messages and each hold 1,000 the other lacks, at the default memory
/// budget: the pair the lane-poll suite counts. Building it runs the peers'
/// own joining sessions, before the tally starts.
fn session() -> Session {
    assert!(
        observe_edge_sends(record),
        "this suite installs the process's send hook"
    );
    let (a, b) = pair(Shape::overlapping(2_000, 50), DEFAULT_SYNC_MEMORY_BUDGET);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("build a current-thread runtime");
    let (a_end, b_end) = memory_connection(MEMORY_CAPACITY);
    TALLY.with(|tally| tally.borrow_mut().clear());
    let frames_sent = runtime.block_on(async {
        let a = tokio::spawn(gossip(a, a_end));
        let b = tokio::spawn(gossip(b, b_end));
        a.await.expect("peer A's task completes") + b.await.expect("peer B's task completes")
    });
    Session {
        tally: TALLY.with(|tally| tally.take()),
        frames_sent,
    }
}

/// The hook sees every frame the session hands to a multiplexer, and on
/// every kind of edge a send waits only after an earlier send parked.
///
/// These follow from the mechanism, so they hold whatever the baseline is.
/// Each frame a side writes passed through one encoder's lane to its
/// multiplexer, so the lanes' sends equal the frames both sides' session
/// statistics count, an account kept apart from the hook: the tally is
/// counting. A send waits only while its sender is parked, which only the
/// send that filled the edge did, and only the next send can wait on that
/// park, so no kind of edge waits more often than it parks.
#[test]
fn every_frame_is_one_lane_send() {
    let Session { tally, frames_sent } = session();
    assert!(frames_sent > 0, "the fixed session sends frames");
    let lanes = tally
        .get(&QueueKind::PipeOutgoing)
        .copied()
        .unwrap_or_default();
    assert_eq!(lanes.sent, frames_sent, "{tally:?}");
    for (kind, sends) in &tally {
        assert!(sends.waited <= sends.parked, "{kind:?}: {sends:?}");
        assert!(sends.parked <= sends.sent, "{kind:?}: {sends:?}");
    }
}

/// Sends, waits, and parks on one kind of edge, for the baseline.
const fn sends(sent: u64, waited: u64, parked: u64) -> Sends {
    Sends {
        sent,
        waited,
        parked,
    }
}

/// The fixed session's sends, waits, and parks per kind of edge equal the
/// committed baseline.
///
/// A send parks exactly when it fills its edge. No send on an edge of a fan
/// or more parks, since none fills it. The edges whose progress needs only
/// one slot have a second, so a send parks there only when it finds the
/// item before it still queued, and the parks show which end of each such
/// edge runs ahead:
///
/// - The three edges that carry one item a session never park: their one
///   send finds them empty.
/// - The responder's root assembler takes each root return as it arrives,
///   so few of those sends park.
/// - Nearly half the frames on either kind of lane park, and about half
///   the chunks on the outgoing replies. An encoder builds its next frame
///   while the multiplexer writes the one it staged, and takes the lane's
///   next only once that write is done; a walk runs ahead of its encoder in
///   the same way one stage further back; and the demultiplexer can read a
///   frame while its decoder is still on the one before. Each of those
///   producers sometimes finds two items queued, and waits.
///
/// The counts are deterministic: the fixture's network is seeded, the
/// runtime polls its tasks in a fixed order, and nothing in a session reads
/// a clock. They move when an edge's capacity, the session's scheduling, or
/// the fixture changes, and a commit that moves them says why.
#[test]
fn sends_parked_per_edge_match_the_baseline() {
    let Session { tally, .. } = session();
    let baseline = BTreeMap::from([
        (QueueKind::OutgoingResponses, sends(2_482, 1_019, 1_226)),
        (QueueKind::AssemblyLevelReturns, sends(1_369, 0, 0)),
        (QueueKind::InitiatorRootQuery, sends(1, 0, 0)),
        (QueueKind::InitiatorRootReturn, sends(1, 0, 0)),
        (QueueKind::ResponderChildQueries, sends(256, 0, 0)),
        (QueueKind::ResponderRootResolution, sends(1, 0, 0)),
        (QueueKind::ResponderRootReturns, sends(256, 10, 23)),
        (QueueKind::InternalChildQueries, sends(1_053, 0, 0)),
        (QueueKind::InternalParentResolutions, sends(1_310, 0, 0)),
        (QueueKind::InternalChildResolutions, sends(316, 0, 0)),
        (QueueKind::DecodedLeaves, sends(4_950, 0, 0)),
        (QueueKind::AssembledSupplies, sends(1_990, 0, 0)),
        (QueueKind::ProxyResponses, sends(2_481, 0, 0)),
        (QueueKind::ProxyLocalQuestions, sends(1_310, 0, 0)),
        (QueueKind::ProxyNextScopes, sends(1_310, 0, 0)),
        (QueueKind::PipeOutgoing, sends(3_591, 982, 1_602)),
        (QueueKind::PipeIncoming, sends(3_587, 882, 1_771)),
    ]);
    assert_eq!(tally, baseline);
}
