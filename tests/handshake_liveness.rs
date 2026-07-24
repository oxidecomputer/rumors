//! Session-shape liveness matrix over a minimal-capacity link.
//!
//! Every cell drives one complete session — both peers polled concurrently —
//! over [`rumors::link::memory_with_capacity`] at its one-byte minimum: the
//! most adversarial transport the in-memory harness supports, where nothing
//! larger than a single byte is ever absorbed in flight. The protocol's
//! liveness must not rest on frame-size assumptions: the greeting's version
//! frame grows with party count, so any phase whose two sides write
//! symmetrically before reading deadlocks the pair the moment its frame
//! outgrows the transport's window. That hazard class is invisible over the
//! roomy links the rest of the suite uses; these cells force every session
//! shape through the window that hides nothing.
//!
//! The fixtures make the version frames non-trivial on purpose — several
//! disjoint originators plus bootstrap→retire cycles, self-checked to exceed
//! the window [`GREETING_FLOOR`]-fold — and every cell asserts liveness and
//! convergence only, never greeting-frame contents.
//!
//! The watchdog is [`block_on`]'s quiescence detector: the sessions are
//! closed-world in-memory futures, so a deadlock surfaces deterministically
//! as a failed poll-progress check (with this file's cell name attached)
//! rather than as a hung gate or a wall-clock guess.

mod common;

use rumors::{Peer, Protocol, Retire, Rumors};

use crate::common::wire::{assert_control_drained, block_on, bootstrap_fork_async_with_protocol};

/// Per-stream byte capacity of the link every cell's session runs over: the
/// harness minimum, so no frame of any phase fits in flight.
const MIN_CAPACITY: usize = 1;

/// Per-stream capacity for fixture-building sessions, which are scaffolding
/// rather than subject: roomy, like the rest of the suite's wire tests.
const FIXTURE_CAPACITY: usize = 64 * 1024;

/// Disjoint originators forked into a seasoned universe, each contributing
/// its own events; they stay live through seasoning, so their regions'
/// uneven tick counts keep the version's event tree branchy (a fully
/// rejoined, uniformly ticked universe would normalize back to a
/// one-or-two-byte version).
const ORIGINATORS: u64 = 6;

/// Base message count per originator; originator `i` commits `(i + 1)` times
/// this many, so no two regions' tick counts agree and normalization cannot
/// flatten the event tree.
const MESSAGES_PER_ORIGINATOR: u64 = 2;

/// Minimum ratio of a seasoned greeting version's encoded width to the
/// link window: the fixture self-check that the greeting overflows the
/// window many times over, without pinning its layout. ITC versions are
/// bit-packed and stay compact even across many parties, so the one-byte
/// window is what guarantees a multi-fill greeting; this floor guards the
/// fixture against normalizing back to a trivial frame.
const GREETING_FLOOR: usize = 8;

/// Messages each side commits on top of a converged pair to make a
/// divergent cell's descent move content in both directions.
const DIVERGENT_MESSAGES: u64 = 8;

/// Gossip `a` and `b` to completion over a fresh link pair of the given
/// per-stream capacity, both sides polled concurrently. Every session is
/// held to the clean-drain invariant at its boundary.
async fn gossip_over(a: &Rumors<u64>, b: &Rumors<u64>, capacity: usize) {
    let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(capacity);
    let (a_out, b_out) = tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link));
    a_out.expect("gossip completes on side A");
    b_out.expect("gossip completes on side B");
    assert_control_drained(a_link, b_link);
}

/// Widen `rumors`'s causal version over roomy links: fork [`ORIGINATORS`]
/// disjoint peers with pairwise-distinct commit counts, gossip each back,
/// and run one bootstrap→retire cycle, so the version spans several party
/// regions whose uneven tick counts defeat event-tree normalization.
///
/// `payload_base` keeps distinct seasoned replicas' payloads disjoint.
async fn season(rumors: &Rumors<u64>, protocol: Protocol, payload_base: u64) {
    let mut payload = payload_base;
    // Live originators: dropped at the end of seasoning (their regions leak,
    // benignly), but their events — a different count per region — stay in
    // the version forever, keeping it branchy.
    let mut originators = Vec::new();
    for originator in 0..ORIGINATORS {
        let fork = bootstrap_fork_async_with_protocol(rumors, protocol).await;
        for _ in 0..(originator + 1) * MESSAGES_PER_ORIGINATOR {
            fork.send(payload);
            payload += 1;
        }
        // The seed ticks between forks too, so regions interleave.
        rumors.send(payload);
        payload += 1;
        gossip_over(rumors, &fork, FIXTURE_CAPACITY).await;
        originators.push(fork);
    }
    // A second, differently uneven round: each originator advances again
    // after the whole cohort exists, so late regions hold events the early
    // convergence rounds could not have lifted into the tree's base.
    for (index, fork) in originators.iter().enumerate() {
        for _ in 0..=index {
            fork.send(payload);
            payload += 1;
        }
    }
    for fork in &originators {
        gossip_over(rumors, fork, FIXTURE_CAPACITY).await;
    }
    // One bootstrap→retire cycle: the retiree's region rejoins the seed's,
    // exercising the id-space shape a recycled identity leaves behind.
    let cycled = bootstrap_fork_async_with_protocol(rumors, protocol).await;
    cycled.send(payload);
    let cycled = cycled
        .try_into_peer()
        .await
        .expect("cycled fork is the sole handle");
    let (mut fork_link, mut seed_link) = rumors::link::memory_with_capacity(FIXTURE_CAPACITY);
    let (retired, gossiped) =
        tokio::join!(cycled.retire(&mut fork_link), rumors.gossip(&mut seed_link));
    gossiped.expect("absorbing gossip completes");
    assert!(
        matches!(retired, Retire::Retired),
        "fixture fork retires cleanly, got {retired:?}"
    );
    assert_control_drained(fork_link, seed_link);
    // Fixture self-check: the greeting's version frame must dwarf the
    // minimal window, or the matrix stops exercising the hazard class. A
    // width bound only — cells never assert greeting contents.
    let width = rumors.snapshot().latest().as_bytes().len();
    assert!(
        width >= GREETING_FLOOR * MIN_CAPACITY,
        "seasoned version is {width} bytes; the matrix needs at least \
         {GREETING_FLOOR}x the {MIN_CAPACITY}-byte window"
    );
}

/// A seasoned replica: a fresh universe on `protocol` with a wide version.
async fn seasoned(protocol: Protocol) -> Rumors<u64> {
    let seed: Rumors<u64> = Peer::seed().protocol(protocol).into_rumors();
    season(&seed, protocol, 0).await;
    seed
}

/// A converged, party-disjoint pair of seasoned replicas.
///
/// Converged means equal versions, so both sides' greetings carry the same
/// wide frame; cells that need divergence commit on top of the pair.
async fn seasoned_pair(protocol: Protocol) -> (Rumors<u64>, Rumors<u64>) {
    let a = seasoned(protocol).await;
    let b = bootstrap_fork_async_with_protocol(&a, protocol).await;
    // The fork originates a little of its own before converging, so the
    // pair's shared version includes events from b's region too.
    for payload in 0..MESSAGES_PER_ORIGINATOR {
        b.send(10_000 + payload);
    }
    gossip_over(&a, &b, FIXTURE_CAPACITY).await;
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    (a, b)
}

// ---- session shapes -------------------------------------------------------
//
// Each shape runs one complete session over the minimal link and asserts
// only liveness (the session completes) and convergence (the replicas agree)
// — never wire layout. The `_v1`/`_v2` cells below instantiate each shape
// per protocol.

/// Converged: equal wide versions, so the session ends at the greeting.
async fn converged(protocol: Protocol) {
    let (a, b) = seasoned_pair(protocol).await;
    gossip_over(&a, &b, MIN_CAPACITY).await;
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
}

/// Divergent gossip: both sides hold unshared content, so the session runs
/// the full descent and moves messages in both directions.
async fn divergent(protocol: Protocol) {
    let (a, b) = seasoned_pair(protocol).await;
    let converged_len = a.snapshot().len();
    for v in 0..DIVERGENT_MESSAGES {
        a.send(100_000 + v);
        b.send(200_000 + v);
    }
    gossip_over(&a, &b, MIN_CAPACITY).await;
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    assert_eq!(
        a.snapshot().len(),
        converged_len + 2 * DIVERGENT_MESSAGES as usize,
        "both sides' unshared messages survive the session"
    );
}

/// Bulk initiator: the smaller side holds many exclusive root children, so
/// its opening supplies cross a stream the responder drains only while its
/// own opening reply is still flushing.
///
/// One extra message on the other side makes the bulk holder the smaller
/// set, so the size election routes it into the initiator role
/// deterministically, and every early-supply frame dwarfs the one-byte
/// window.
async fn bulk_initiator(protocol: Protocol) {
    let (a, b) = seasoned_pair(protocol).await;
    let converged_len = a.snapshot().len();
    for v in 0..DIVERGENT_MESSAGES {
        a.send(300_000 + v);
    }
    for v in 0..=DIVERGENT_MESSAGES {
        b.send(400_000 + v);
    }
    assert!(
        a.snapshot().len() < b.snapshot().len(),
        "the bulk-exclusive side must advertise the smaller set"
    );
    gossip_over(&a, &b, MIN_CAPACITY).await;
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    assert_eq!(
        a.snapshot().len(),
        converged_len + (2 * DIVERGENT_MESSAGES + 1) as usize,
        "every exclusive message survives the session"
    );
}

/// Empty meets populated: one replica has committed nothing (a tiny
/// greeting), the other is seasoned (a wide one), so the two greeting
/// frames are maximally asymmetric and the descent is one-sided.
async fn empty_meets_populated(protocol: Protocol) {
    let seed: Rumors<u64> = Peer::seed().protocol(protocol).into_rumors();
    // Fork the empty side out before any content exists, then season only
    // the seed.
    let empty = bootstrap_fork_async_with_protocol(&seed, protocol).await;
    season(&seed, protocol, 0).await;
    gossip_over(&seed, &empty, MIN_CAPACITY).await;
    assert_eq!(seed.snapshot().hash(), empty.snapshot().hash());
    assert!(
        !empty.snapshot().is_empty(),
        "the empty side learned the content"
    );
}

/// Bootstrap: a seasoned provider serves a newcomer, so the whole tree and
/// the trailing party donation cross the minimal link.
async fn bootstrap(protocol: Protocol) {
    let provider = seasoned(protocol).await;
    let (mut p_link, mut n_link) = rumors::link::memory_with_capacity(MIN_CAPACITY);
    let (served, joined) = tokio::join!(
        provider.gossip(&mut p_link),
        Peer::<u64>::bootstrap_with_protocol(protocol, &mut n_link),
    );
    served.expect("the serving session completes");
    let newcomer = joined
        .expect("the bootstrap session completes")
        .expect("the provider serves the bootstrap")
        .into_rumors();
    assert_eq!(newcomer.snapshot().hash(), provider.snapshot().hash());
    assert_control_drained(p_link, n_link);
}

/// Retire: a seasoned, divergent retiree hands its content and then its
/// whole party to the absorber, all through the minimal link.
async fn retire(protocol: Protocol) {
    let (a, b) = seasoned_pair(protocol).await;
    let converged_len = b.snapshot().len();
    for v in 0..DIVERGENT_MESSAGES {
        a.send(300_000 + v);
    }
    let retiree = a.try_into_peer().await.expect("a is the sole handle");
    let (mut r_link, mut p_link) = rumors::link::memory_with_capacity(MIN_CAPACITY);
    let (retired, gossiped) = tokio::join!(retiree.retire(&mut r_link), b.gossip(&mut p_link));
    gossiped.expect("the absorbing session completes");
    assert!(
        matches!(retired, Retire::Retired),
        "the retiree is absorbed, got {retired:?}"
    );
    assert_eq!(
        b.snapshot().len(),
        converged_len + DIVERGENT_MESSAGES as usize,
        "nothing the retiree held is lost"
    );
    assert_control_drained(r_link, p_link);
}

/// Mutual bootstrap: both sides are identity-less newcomers, so the session
/// bails with `None` on both ends — but only after the bail is certified
/// through a symmetric greeting exchange whose fixed-width frames overflow
/// the one-byte window many times over. No seasoning is possible here: a
/// bootstrapping peer holds no identity to widen, so the fixed frames are
/// the whole hazard.
async fn mutual_bootstrap(protocol: Protocol) {
    let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(MIN_CAPACITY);
    let (a_out, b_out) = tokio::join!(
        Peer::<u64>::bootstrap_with_protocol(protocol, &mut a_link),
        Peer::<u64>::bootstrap_with_protocol(protocol, &mut b_link),
    );
    assert!(
        a_out.expect("side A handshake completes").is_none(),
        "a mutually-bootstrapping peer bails with None"
    );
    assert!(
        b_out.expect("side B handshake completes").is_none(),
        "a mutually-bootstrapping peer bails with None"
    );
    assert_control_drained(a_link, b_link);
}

/// Retire into a bootstrapper: a seasoned retiree hands its whole tree and
/// then its whole party to an identity-less newcomer — the maximally
/// asymmetric greetings of bootstrap plus the trailing donation of retire,
/// all through the minimal link.
async fn retire_into_bootstrapper(protocol: Protocol) {
    let retiree = seasoned(protocol).await;
    let before = retiree.snapshot();
    let retiree = retiree
        .try_into_peer()
        .await
        .expect("the seasoned replica is the sole handle");
    let (mut r_link, mut n_link) = rumors::link::memory_with_capacity(MIN_CAPACITY);
    let (retired, joined) = tokio::join!(
        retiree.retire(&mut r_link),
        Peer::<u64>::bootstrap_with_protocol(protocol, &mut n_link),
    );
    assert!(
        matches!(retired, Retire::Retired),
        "the bootstrapper absorbs the retiree, got {retired:?}"
    );
    let successor = joined
        .expect("the bootstrap session completes")
        .expect("the retiree serves the bootstrap")
        .into_rumors();
    let after = successor.snapshot();
    assert_eq!(after.len(), before.len(), "no content is lost in handoff");
    assert_eq!(
        after.iter().map(|(k, _, _)| k).collect::<Vec<_>>(),
        before.iter().map(|(k, _, _)| k).collect::<Vec<_>>(),
        "the successor holds exactly the retiree's content"
    );
    assert_control_drained(r_link, n_link);
}

/// Mutual retire: both sides declare `Retire`, so the session early-outs
/// right after the preamble and both replicas survive intact.
async fn mutual_retire(protocol: Protocol) {
    let (a, b) = seasoned_pair(protocol).await;
    let a = a.try_into_peer().await.expect("a is the sole handle");
    let b = b.try_into_peer().await.expect("b is the sole handle");
    let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(MIN_CAPACITY);
    let (a_out, b_out) = tokio::join!(a.retire(&mut a_link), b.retire(&mut b_link));
    assert!(
        matches!(a_out, Retire::Declined { .. }),
        "a mutual retire declines side A, got {a_out:?}"
    );
    assert!(
        matches!(b_out, Retire::Declined { .. }),
        "a mutual retire declines side B, got {b_out:?}"
    );
    assert_control_drained(a_link, b_link);
}

// ---- the matrix -----------------------------------------------------------

/// A V2 session between converged seasoned replicas stays live and
/// re-converges over a one-byte-window link.
#[test]
fn converged_v2() {
    block_on(converged(Protocol::V2));
}

/// A V1 session between converged seasoned replicas stays live and
/// re-converges over a one-byte-window link.
#[cfg(feature = "protocol-v1")]
#[test]
fn converged_v1() {
    block_on(converged(Protocol::V1));
}

/// A divergent V2 gossip session stays live over a one-byte-window link and
/// converges both replicas.
#[test]
fn divergent_v2() {
    block_on(divergent(Protocol::V2));
}

/// A divergent V1 gossip session stays live over a one-byte-window link and
/// converges both replicas.
#[cfg(feature = "protocol-v1")]
#[test]
fn divergent_v1() {
    block_on(divergent(Protocol::V1));
}

/// A V2 session whose initiator ships bulk opening supplies stays live
/// over a one-byte-window link and converges both replicas.
#[test]
fn bulk_initiator_v2() {
    block_on(bulk_initiator(Protocol::V2));
}

/// A V1 session with the bulk-initiator shape (the smaller set holding the
/// bulk) stays live over a one-byte-window link and converges both
/// replicas.
#[cfg(feature = "protocol-v1")]
#[test]
fn bulk_initiator_v1() {
    block_on(bulk_initiator(Protocol::V1));
}

/// A V2 session between an empty replica and a seasoned one — maximally
/// asymmetric greetings — stays live over a one-byte-window link.
#[test]
fn empty_meets_populated_v2() {
    block_on(empty_meets_populated(Protocol::V2));
}

/// A V1 session between an empty replica and a seasoned one — maximally
/// asymmetric greetings — stays live over a one-byte-window link.
#[cfg(feature = "protocol-v1")]
#[test]
fn empty_meets_populated_v1() {
    block_on(empty_meets_populated(Protocol::V1));
}

/// A V2 bootstrap served by a seasoned provider stays live over a
/// one-byte-window link, tree and party donation included.
#[test]
fn bootstrap_v2() {
    block_on(bootstrap(Protocol::V2));
}

/// A V1 bootstrap served by a seasoned provider stays live over a
/// one-byte-window link, tree and party donation included.
#[cfg(feature = "protocol-v1")]
#[test]
fn bootstrap_v1() {
    block_on(bootstrap(Protocol::V1));
}

/// A V2 retirement of a divergent seasoned replica stays live over a
/// one-byte-window link and loses none of the retiree's content.
#[test]
fn retire_v2() {
    block_on(retire(Protocol::V2));
}

/// A V1 retirement of a divergent seasoned replica stays live over a
/// one-byte-window link and loses none of the retiree's content.
#[cfg(feature = "protocol-v1")]
#[test]
fn retire_v1() {
    block_on(retire(Protocol::V1));
}

/// A mutual V2 bootstrap bails cleanly on both sides over a
/// one-byte-window link, its fixed-width greeting exchange included.
#[test]
fn mutual_bootstrap_v2() {
    block_on(mutual_bootstrap(Protocol::V2));
}

/// A mutual V1 bootstrap bails cleanly on both sides over a
/// one-byte-window link, its fixed-width greeting exchange included.
#[cfg(feature = "protocol-v1")]
#[test]
fn mutual_bootstrap_v1() {
    block_on(mutual_bootstrap(Protocol::V1));
}

/// A V2 retirement into a bootstrapper stays live over a one-byte-window
/// link and hands the newcomer the retiree's exact content.
#[test]
fn retire_into_bootstrapper_v2() {
    block_on(retire_into_bootstrapper(Protocol::V2));
}

/// A V1 retirement into a bootstrapper stays live over a one-byte-window
/// link and hands the newcomer the retiree's exact content.
#[cfg(feature = "protocol-v1")]
#[test]
fn retire_into_bootstrapper_v1() {
    block_on(retire_into_bootstrapper(Protocol::V1));
}

/// A mutual V2 retirement early-outs cleanly over a one-byte-window link,
/// declining both sides.
#[test]
fn mutual_retire_v2() {
    block_on(mutual_retire(Protocol::V2));
}

/// A mutual V1 retirement early-outs cleanly over a one-byte-window link,
/// declining both sides.
#[cfg(feature = "protocol-v1")]
#[test]
fn mutual_retire_v1() {
    block_on(mutual_retire(Protocol::V1));
}
