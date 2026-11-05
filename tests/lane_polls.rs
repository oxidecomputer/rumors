//! How many lane channels the multiplexer polls per frame it writes, on one
//! fixed session.
//!
//! Each side of a session sends its frames through one *multiplexer*, which
//! takes frames from one channel per logical stream (a *lane*) and writes
//! them to the connection. A poll of the multiplexer stages frames from the
//! lanes' channels, then returns at most one frame to write, so the lane
//! channels each poll polls set much of what a frame costs to send. This
//! suite counts those polls through the process's multiplexer hook
//! ([`observe_multiplexers`]) on one deterministic session.
//!
//! The session runs each peer in its own task on a current-thread runtime,
//! as an application runs each peer in its own task. Both tasks run on the
//! thread that drives the runtime, so the hook adds each report to that
//! thread's tally, which no session on another test thread can reach.

use std::cell::Cell;

#[cfg(doc)]
use rumors::testing::LANE_CAPACITY;
use rumors::testing::{
    MEMORY_CAPACITY, MemoryEnd, MultiplexerPoll, memory_connection, observe_multiplexers,
};
use rumors::{DEFAULT_SYNC_MEMORY_BUDGET, Rumors};
use rumors_testkit::bench::session::{Shape, pair};

/// What a session's multiplexers did, summed over both sides.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Tally {
    /// Polls of either multiplexer.
    polls: u64,
    /// Polls that returned a frame to write.
    frames: u64,
    /// Lane channels the polls polled.
    polled: u64,
    /// Lanes that were open with nothing staged when a poll reached them,
    /// summed over the polls: the lane channels that polling every lane
    /// would poll.
    unstaged: u64,
}

/// One session's tally beside the frames both sides' session statistics
/// report sending.
#[derive(Debug)]
struct Session {
    /// What the multiplexers reported through the hook.
    tally: Tally,
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
    static TALLY: Cell<Tally> = const {
        Cell::new(Tally {
            polls: 0,
            frames: 0,
            polled: 0,
            unstaged: 0,
        })
    };
}

/// Add one multiplexer poll to this thread's tally.
fn record(poll: MultiplexerPoll) {
    TALLY.with(|tally| {
        let mut sum = tally.get();
        sum.polls += 1;
        sum.frames += u64::from(poll.frame);
        sum.polled += poll.polled as u64;
        sum.unstaged += poll.unstaged as u64;
        tally.set(sum);
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
/// the session's multiplexers did.
///
/// The pair is two peers of one deterministic network that share 1,000
/// messages and each hold 1,000 the other lacks, at the default memory
/// budget. Building it runs the peers' own joining sessions, before the
/// tally starts.
fn session() -> Session {
    assert!(
        observe_multiplexers(record),
        "this suite installs the process's multiplexer hook"
    );
    let (a, b) = pair(Shape::overlapping(2_000, 50), DEFAULT_SYNC_MEMORY_BUDGET);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("build a current-thread runtime");
    let (a_end, b_end) = memory_connection(MEMORY_CAPACITY);
    TALLY.with(|tally| tally.set(Tally::default()));
    let frames_sent = runtime.block_on(async {
        let a = tokio::spawn(gossip(a, a_end));
        let b = tokio::spawn(gossip(b, b_end));
        a.await.expect("peer A's task completes") + b.await.expect("peer B's task completes")
    });
    Session {
        tally: TALLY.with(Cell::get),
        frames_sent,
    }
}

/// The hook sees every frame the session sends, and the multiplexers poll
/// at least one lane channel per frame and only lanes that are open with
/// nothing staged.
///
/// These bounds follow from the mechanism, so they hold whatever the
/// baseline is. The multiplexer takes each frame it writes from a lane's
/// channel, through a poll of that channel that yields the frame, and a
/// poll yields at most one frame, so the lane polls are at least the
/// frames. A lane with a staged frame waits for that frame's turn, and a
/// closed lane has no channel left, so no poll polls either. The hook's
/// frames equal the frames both sides' session statistics count, an
/// account kept apart from the hook, so the hook reports every poll that
/// returns a frame, and the tally is counting.
#[test]
fn every_frame_costs_at_least_one_lane_poll() {
    let Session { tally, frames_sent } = session();
    assert!(frames_sent > 0, "the fixed session sends frames");
    assert_eq!(tally.frames, frames_sent, "{tally:?}");
    assert!(tally.polled >= tally.frames, "{tally:?}");
    assert!(tally.polled <= tally.unstaged, "{tally:?}");
}

/// The fixed session's lane polls, polls, and frames equal the committed
/// baseline.
///
/// A poll polls a lane's channel only when that channel has woken the
/// multiplexer since its last poll, or has just yielded a frame and may
/// hold another. So a frame costs one or two lane polls: the one that
/// stages it, and, once it is written, one that finds the channel empty
/// and registers the lane's waker, unless the channel holds the lane's
/// next frame already, which that poll then stages. Each lane adds two
/// more: its first poll, which registers its waker, and the poll that finds
/// its channel closed.
///
/// The unstaged lanes count what polling every open lane with nothing
/// staged would cost instead. Each side has one lane per level it answers
/// at, and nearly every lane is open with nothing staged at nearly every
/// poll, because a lane holds at most [`LANE_CAPACITY`] frames and the
/// multiplexer writes each frame soon after staging it. The multiplexer is
/// polled a few times per frame. Having written a frame, it polls once for
/// the next, which is ready when some lane's channel already holds one;
/// otherwise the encoder that handed over the last one must run again
/// before it can hand over another. Finding none, it flushes and polls once
/// more before it waits, and is polled again by the wake that brings the
/// next frame. The session polls it as one member of a task set that polls
/// each member only for its own wakes, so nothing else polls it.
///
/// The counts are deterministic: the fixture's network is seeded, the
/// runtime polls its tasks in a fixed order, and nothing in a session reads
/// a clock. They move when the multiplexer, the session's scheduling, or
/// the fixture changes, and a commit that moves them says why.
#[test]
fn lane_polls_per_frame_match_the_baseline() {
    let Session { tally, .. } = session();
    assert_eq!(
        tally,
        Tally {
            polls: 7_332,
            frames: 3_587,
            polled: 5_634,
            unstaged: 117_507,
        }
    );
}
