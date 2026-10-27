//! Allocator metering for one gossip session between divergent peers.
//!
//! The meter counts only the thread driving the session. Both peers' session
//! futures run on that thread under the testkit's single-threaded executor,
//! so the reading covers the whole session and nothing else.
//!
//! The session is priced in *over-aligned* allocations, those aligned beyond
//! two machine words. The system allocator serves them through the platform's
//! aligned-allocation entry point rather than `malloc`, and illumos's libumem
//! serves that entry point from a single locked arena, so concurrent sessions
//! there serialize on each one.

use alloc_meter::measure;
use rumors::Rumors;
use rumors_testkit::common::{
    gossip_snapshot::seeded,
    wire::{bootstrap_fork, diverge, wire_gossip},
};

/// Values each peer holds that the other lacks before the session.
const VALUES_PER_SIDE: u64 = 64;

/// The over-aligned allocations one session of this fixture makes.
///
/// Every over-aligned allocation in a session is the shared state of a Tokio
/// `mpsc` channel, which embeds cache-padded fields aligned to 128 bytes on
/// 64-bit targets. The count is therefore the number of such channels the
/// session creates: a fixed set per tree height for each peer's walk and
/// remote proxy, one per decoded wire reply, and a few per session for the
/// link and its error routes.
const OVER_ALIGNED_ALLOCATIONS: usize = 383;

/// Two forks of one deterministically seeded network, each holding
/// [`VALUES_PER_SIDE`] values the other lacks.
fn divergent_fixture() -> (Rumors<u64>, Rumors<u64>) {
    let seed = seeded::<u64>();
    let a = bootstrap_fork(&seed);
    let b = bootstrap_fork(&seed);
    diverge(&a, &b, VALUES_PER_SIDE);
    (a, b)
}

/// A session between divergent peers makes exactly
/// [`OVER_ALIGNED_ALLOCATIONS`] over-aligned allocations.
///
/// The count is deterministic: the fixture's network is seeded from a fixed
/// stream, and the executor polls both peers in a fixed order. An exact
/// assertion makes every change in either direction visible in the commit
/// that causes it.
///
/// The liveness floor guards against a reading that stops seeing the
/// session, for instance because its work moved to another thread: each
/// peer materializes every record it lacks, which takes at least one
/// allocation per record, so the session allocates at least twice
/// [`VALUES_PER_SIDE`] times on this thread.
#[test]
fn divergent_session_over_aligned_allocations() {
    let (a, b) = divergent_fixture();
    let (stats, ()) = measure(|| wire_gossip(&a, &b));

    let materialized = 2 * usize::try_from(VALUES_PER_SIDE).expect("the fixture size fits usize");
    assert!(
        stats.allocations >= materialized,
        "the session made {} allocations on the measuring thread, fewer than \
         the {materialized} its record materialization requires",
        stats.allocations,
    );
    assert_eq!(
        stats.over_aligned, OVER_ALIGNED_ALLOCATIONS,
        "over-aligned allocations per session moved",
    );

    let mut ours: Vec<u64> = a.snapshot().iter().map(|(_, value)| *value).collect();
    let mut theirs: Vec<u64> = b.snapshot().iter().map(|(_, value)| *value).collect();
    ours.sort_unstable();
    theirs.sort_unstable();
    assert_eq!(ours, theirs, "the session converges");
    assert_eq!(ours.len(), materialized, "the peers held disjoint values");
}
