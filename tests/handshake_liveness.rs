//! Session-shape liveness matrix over a minimal-capacity connection.
//!
//! Every cell drives one complete session — both peers polled concurrently —
//! over [`rumors::testing::memory_connection`] at its one-byte minimum: the
//! most adversarial transport the in-memory harness supports, where nothing
//! larger than a single byte is ever absorbed in flight. The protocol's
//! liveness must not rest on frame-size assumptions: the greeting's version
//! frame grows with party count, so any phase whose two sides write
//! symmetrically before reading deadlocks the pair the moment its frame
//! outgrows the transport's window. That hazard class is invisible over the
//! roomy connections the rest of the suite uses; these cells force every session
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

use rumors_testkit::common;
use rumors_testkit::common::wire::ROOMY_CAPACITY;

use rumors::{Connection, Peer, Retire, Rumors};

use rumors_testkit::common::property::FLEET_DIVISOR;
use rumors_testkit::common::seed::{NetworkSeed, network_seed};
use rumors_testkit::divided_proptest;

use crate::common::{
    budget::BudgetChoice,
    wire::{
        assert_connection_drained, block_on, bootstrap_fork_async, bootstrap_fork_with_budget_async,
    },
};

/// Per-direction byte capacity of the connection every cell's session runs over: the
/// harness minimum, so no frame of any phase fits in flight.
const MIN_CAPACITY: usize = 1;

/// Disjoint originators forked into a seasoned universe, each contributing
/// its own events.
///
/// They stay live through seasoning, so their regions'
/// uneven tick counts keep the version's event tree branchy (a fully
/// rejoined, uniformly ticked universe would normalize back to a
/// one-or-two-byte version).
const ORIGINATORS: u64 = 6;

/// Base message count per originator; originator `i` commits `(i + 1)` times
/// this many, so no two regions' tick counts agree and normalization cannot
/// flatten the event tree.
const MESSAGES_PER_ORIGINATOR: u64 = 2;

/// Minimum ratio of a seasoned greeting version's encoded width to the
/// connection's window: the fixture self-check that the greeting overflows the
/// window many times over, without pinning its layout.
///
/// ITC versions stay compact even across many parties. The one-byte window
/// guarantees a multi-fill greeting; this floor keeps the fixture's version
/// from normalizing back to a trivial frame.
const GREETING_FLOOR: usize = 8;

/// Messages each side commits on top of a converged pair to make a
/// divergent cell's descent move content in both directions.
const DIVERGENT_MESSAGES: u64 = 8;

/// Gossip `a` and `b` to completion over a fresh connection of the given
/// per-direction capacity, both sides polled concurrently. Every session is
/// held to the clean-drain invariant at its boundary.
async fn gossip_over(a: &Rumors<u64>, b: &Rumors<u64>, capacity: usize) {
    let (mut a_end, mut b_end) = rumors::testing::memory_connection(capacity);
    let (a_out, b_out) = tokio::join!(
        a.gossip_once(&mut a_end.read, &mut a_end.write),
        b.gossip_once(&mut b_end.read, &mut b_end.write)
    );
    a_out.expect("gossip completes on side A");
    b_out.expect("gossip completes on side B");
    assert_connection_drained(a_end, b_end);
}

/// Widen `rumors`'s causal version over roomy connections: fork [`ORIGINATORS`]
/// disjoint peers with pairwise-distinct commit counts, gossip each back,
/// and run one bootstrap→retire cycle.
///
/// The version then spans several party
/// regions whose uneven tick counts defeat event-tree normalization.
///
/// `payload_base` keeps distinct seasoned replicas' payloads disjoint.
async fn season(rumors: &Rumors<u64>, payload_base: u64) {
    let mut payload = payload_base;
    // Live originators: dropped at the end of seasoning (their regions leak,
    // benignly), but their events — a different count per region — stay in
    // the version forever, keeping it branchy.
    let mut originators = Vec::new();
    for originator in 0..ORIGINATORS {
        let fork = bootstrap_fork_async(rumors).await;
        for _ in 0..(originator + 1) * MESSAGES_PER_ORIGINATOR {
            fork.send(payload).unwrap();
            payload += 1;
        }
        // The seed ticks between forks too, so regions interleave.
        rumors.send(payload).unwrap();
        payload += 1;
        gossip_over(rumors, &fork, ROOMY_CAPACITY).await;
        originators.push(fork);
    }
    // A second, differently uneven round: each originator advances again
    // after the whole cohort exists, so late regions hold events the early
    // convergence rounds could not have lifted into the tree's base.
    for (index, fork) in originators.iter().enumerate() {
        for _ in 0..=index {
            fork.send(payload).unwrap();
            payload += 1;
        }
    }
    for fork in &originators {
        gossip_over(rumors, fork, ROOMY_CAPACITY).await;
    }
    // One bootstrap→retire cycle: the retiree's region rejoins the seed's,
    // exercising the id-space shape a recycled identity leaves behind.
    let cycled = bootstrap_fork_async(rumors).await;
    cycled.send(payload).unwrap();
    let cycled = cycled
        .try_into_peer()
        .await
        .expect("cycled fork is the sole handle");
    let (mut fork_end, mut seed_end) = rumors::testing::memory_connection(ROOMY_CAPACITY);
    let (retired, gossiped) = tokio::join!(
        cycled.retire(&mut fork_end.read, &mut fork_end.write),
        rumors.gossip_once(&mut seed_end.read, &mut seed_end.write)
    );
    gossiped.expect("absorbing gossip completes");
    assert!(
        matches!(
            retired,
            Retire::Retired {
                connection: Connection::Reusable { .. }
            }
        ),
        "fixture fork retires cleanly, got {retired:?}"
    );
    assert_connection_drained(fork_end, seed_end);
    // Keep the seasoned version wide and branchy. The one-byte window alone
    // guarantees a multi-fill greeting; this check guards the fixture shape.
    let width = rumors.snapshot().latest().as_bytes().len();
    assert!(
        width >= GREETING_FLOOR * MIN_CAPACITY,
        "seasoned version is {width} bytes; the matrix needs at least \
         {GREETING_FLOOR}x the {MIN_CAPACITY}-byte window"
    );
}

/// The network `network` names, given a wide version and the selected
/// budget.
async fn seasoned(network: NetworkSeed, budget: BudgetChoice) -> Rumors<u64> {
    let seed: Rumors<u64> = budget.apply(network.peer()).into_rumors();
    season(&seed, 0).await;
    seed
}

/// A converged, party-disjoint pair of seasoned replicas.
///
/// Converged means equal versions, so both sides' greetings carry the same
/// wide frame; cells that need divergence commit on top of the pair. The
/// arguments select the budgets used by the session under test.
async fn seasoned_pair(
    network: NetworkSeed,
    a_budget: BudgetChoice,
    b_budget: BudgetChoice,
) -> (Rumors<u64>, Rumors<u64>) {
    let a = seasoned(network, a_budget).await;
    let b = bootstrap_fork_with_budget_async(&a, b_budget).await;
    // The fork originates a little of its own before converging, so the
    // pair's shared version includes events from b's region too.
    for payload in 0..MESSAGES_PER_ORIGINATOR {
        b.send(10_000 + payload).unwrap();
    }
    gossip_over(&a, &b, ROOMY_CAPACITY).await;
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    (a, b)
}

// ---- session shapes -------------------------------------------------------
//
// Each shape runs one complete session over the minimal connection and asserts
// only liveness (the session completes) and convergence (the replicas agree)
// — never wire layout. The test cells below instantiate one shape each.

/// Converged: equal wide versions, so the session ends at the greeting.
async fn converged_session(network: NetworkSeed) {
    let (a, b) = seasoned_pair(network, BudgetChoice::Serial, BudgetChoice::Serial).await;
    gossip_over(&a, &b, MIN_CAPACITY).await;
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
}

/// Divergent gossip: both sides hold unshared content, so the session runs
/// the full descent under the selected budgets and moves messages in
/// both directions.
async fn divergent_session(network: NetworkSeed, a_budget: BudgetChoice, b_budget: BudgetChoice) {
    let (a, b) = seasoned_pair(network, a_budget, b_budget).await;
    let converged_len = a.snapshot().len();
    for v in 0..DIVERGENT_MESSAGES {
        a.send(100_000 + v).unwrap();
        b.send(200_000 + v).unwrap();
    }
    gossip_over(&a, &b, MIN_CAPACITY).await;
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    assert_eq!(
        a.snapshot().len(),
        converged_len + 2 * DIVERGENT_MESSAGES as usize,
        "both sides' unshared messages survive the session"
    );
}

/// Bulk initiator: the smaller side holds many exclusive root children,
/// which cross as supplies answering the responder's root-level requests
/// while the responder's opening reply may still be flushing.
///
/// One extra message on the other side makes the bulk holder the smaller
/// set, so the size election routes it into the initiator role
/// deterministically, and every supply frame dwarfs the one-byte window.
async fn bulk_initiator_session(network: NetworkSeed) {
    let (a, b) = seasoned_pair(network, BudgetChoice::Serial, BudgetChoice::Serial).await;
    let converged_len = a.snapshot().len();
    for v in 0..DIVERGENT_MESSAGES {
        a.send(300_000 + v).unwrap();
    }
    for v in 0..=DIVERGENT_MESSAGES {
        b.send(400_000 + v).unwrap();
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
async fn empty_meets_populated_session(network: NetworkSeed) {
    let seed: Rumors<u64> = network.peer().sync_memory_budget(0).into_rumors();
    // Fork the empty side out before any content exists, then season only
    // the seed.
    let empty = bootstrap_fork_async(&seed).await;
    season(&seed, 0).await;
    gossip_over(&seed, &empty, MIN_CAPACITY).await;
    assert_eq!(seed.snapshot().hash(), empty.snapshot().hash());
    assert!(
        !empty.snapshot().is_empty(),
        "the empty side learned the content"
    );
}

/// Bootstrap: a seasoned provider serves a newcomer, so the whole tree and
/// the trailing party donation cross the minimal connection.
async fn bootstrap_session(network: NetworkSeed) {
    let provider = seasoned(network, BudgetChoice::Serial).await;
    let (mut p_end, mut n_end) = rumors::testing::memory_connection(MIN_CAPACITY);
    let (served, joined) = tokio::join!(
        provider.gossip_once(&mut p_end.read, &mut p_end.write),
        Peer::<u64>::bootstrap().join(&mut n_end.read, &mut n_end.write),
    );
    served.expect("the serving session completes");
    let newcomer = (match joined {
        rumors::Joined::Joined {
            peer,
            connection: rumors::Connection::Reusable { .. },
        } => peer,
        _ => panic!("the provider serves the bootstrap"),
    })
    .into_rumors();
    assert_eq!(newcomer.snapshot().hash(), provider.snapshot().hash());
    assert_connection_drained(p_end, n_end);
}

/// Retire: a seasoned, divergent retiree hands its content and then its
/// whole party to the absorber, all through the minimal connection.
async fn retire_session(network: NetworkSeed) {
    let (a, b) = seasoned_pair(network, BudgetChoice::Serial, BudgetChoice::Serial).await;
    let converged_len = b.snapshot().len();
    for v in 0..DIVERGENT_MESSAGES {
        a.send(300_000 + v).unwrap();
    }
    let retiree = a.try_into_peer().await.expect("a is the sole handle");
    let (mut r_end, mut p_end) = rumors::testing::memory_connection(MIN_CAPACITY);
    let (retired, gossiped) = tokio::join!(
        retiree.retire(&mut r_end.read, &mut r_end.write),
        b.gossip_once(&mut p_end.read, &mut p_end.write)
    );
    gossiped.expect("the absorbing session completes");
    assert!(
        matches!(
            retired,
            Retire::Retired {
                connection: Connection::Reusable { .. }
            }
        ),
        "the retiree is absorbed, got {retired:?}"
    );
    assert_eq!(
        b.snapshot().len(),
        converged_len + DIVERGENT_MESSAGES as usize,
        "nothing the retiree held is lost"
    );
    assert_connection_drained(r_end, p_end);
}

/// Mutual bootstrap: both sides are identity-less newcomers, so the session
/// bails with `None` on both ends.
///
/// The bail lands only after it is certified
/// through a symmetric greeting exchange whose fixed-width frames overflow
/// the one-byte window many times over.
/// No seasoning is possible here: a
/// bootstrapping peer holds no identity to widen, so the fixed frames are
/// the whole hazard.
async fn mutual_bootstrap_session() {
    let (mut a_end, mut b_end) = rumors::testing::memory_connection(MIN_CAPACITY);
    let (a_out, b_out) = tokio::join!(
        Peer::<u64>::bootstrap().join(&mut a_end.read, &mut a_end.write),
        Peer::<u64>::bootstrap().join(&mut b_end.read, &mut b_end.write),
    );
    assert!(
        matches!(a_out, rumors::Joined::Bailed { .. }),
        "a mutually-bootstrapping peer returns its builder"
    );
    assert!(
        matches!(b_out, rumors::Joined::Bailed { .. }),
        "a mutually-bootstrapping peer returns its builder"
    );
    assert_connection_drained(a_end, b_end);
}

/// Retire into a bootstrapper: a seasoned retiree hands its whole tree and
/// then its whole party to an identity-less newcomer.
///
/// This is the maximally
/// asymmetric greetings of bootstrap plus the trailing donation of retire,
/// all through the minimal connection.
async fn retire_into_bootstrapper_session(network: NetworkSeed) {
    let retiree = seasoned(network, BudgetChoice::Serial).await;
    let before = retiree.snapshot();
    let retiree = retiree
        .try_into_peer()
        .await
        .expect("the seasoned replica is the sole handle");
    let (mut r_end, mut n_end) = rumors::testing::memory_connection(MIN_CAPACITY);
    let (retired, joined) = tokio::join!(
        retiree.retire(&mut r_end.read, &mut r_end.write),
        Peer::<u64>::bootstrap().join(&mut n_end.read, &mut n_end.write),
    );
    assert!(
        matches!(
            retired,
            Retire::Retired {
                connection: Connection::Reusable { .. }
            }
        ),
        "the bootstrapper absorbs the retiree, got {retired:?}"
    );
    let successor = (match joined {
        rumors::Joined::Joined {
            peer,
            connection: rumors::Connection::Reusable { .. },
        } => peer,
        _ => panic!("the retiree serves the bootstrap"),
    })
    .into_rumors();
    let after = successor.snapshot();
    assert_eq!(after.len(), before.len(), "no content is lost in handoff");
    assert_eq!(
        after.iter().collect::<Vec<_>>(),
        before.iter().collect::<Vec<_>>(),
        "the successor holds exactly the retiree's content"
    );
    assert_connection_drained(r_end, n_end);
}

/// Mutual retire: both sides declare `Retire`, so the session early-outs
/// right after the preamble and both replicas survive intact.
async fn mutual_retire_session(network: NetworkSeed) {
    let (a, b) = seasoned_pair(network, BudgetChoice::Serial, BudgetChoice::Serial).await;
    let a = a.try_into_peer().await.expect("a is the sole handle");
    let b = b.try_into_peer().await.expect("b is the sole handle");
    let (mut a_end, mut b_end) = rumors::testing::memory_connection(MIN_CAPACITY);
    let (a_out, b_out) = tokio::join!(
        a.retire(&mut a_end.read, &mut a_end.write),
        b.retire(&mut b_end.read, &mut b_end.write)
    );
    assert!(
        matches!(a_out, Retire::Declined { .. }),
        "a mutual retire declines side A, got {a_out:?}"
    );
    assert!(
        matches!(b_out, Retire::Declined { .. }),
        "a mutual retire declines side B, got {b_out:?}"
    );
    assert_connection_drained(a_end, b_end);
}

// ---- the matrix -----------------------------------------------------------

divided_proptest! {
    #![divisor(FLEET_DIVISOR)]

    /// A V2 session between converged seasoned replicas stays live and
    /// re-converges over a one-byte-window connection.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn converged(network in network_seed()) {
        block_on(converged_session(network));
    }

    /// A divergent V2 gossip session stays live over a one-byte-window connection and
    /// converges both replicas.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn divergent(network in network_seed()) {
        block_on(divergent_session(network, BudgetChoice::Serial, BudgetChoice::Serial));
    }

    /// Default budgets remain live when the transport admits only one byte at a
    /// time, and the divergent replicas converge completely.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn divergent_default_budgets(network in network_seed()) {
        block_on(divergent_session(
            network,
            BudgetChoice::Default,
            BudgetChoice::Default,
        ));
    }

    /// A serial peer and a default one remain live together over a one-byte
    /// transport and converge completely.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn divergent_asymmetric_budgets(network in network_seed()) {
        block_on(divergent_session(network, BudgetChoice::Serial, BudgetChoice::Default));
    }

    /// A V2 session whose initiator supplies bulk root children stays live
    /// over a one-byte-window connection and converges both replicas.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn bulk_initiator(network in network_seed()) {
        block_on(bulk_initiator_session(network));
    }

    /// A V2 session between an empty replica and a seasoned one — maximally
    /// asymmetric greetings — stays live over a one-byte-window connection.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn empty_meets_populated(network in network_seed()) {
        block_on(empty_meets_populated_session(network));
    }

    /// A V2 bootstrap served by a seasoned provider stays live over a
    /// one-byte-window connection, tree and party donation included.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn bootstrap(network in network_seed()) {
        block_on(bootstrap_session(network));
    }

    /// A V2 retirement of a divergent seasoned replica stays live over a
    /// one-byte-window connection and loses none of the retiree's content.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn retire(network in network_seed()) {
        block_on(retire_session(network));
    }
}

/// A mutual V2 bootstrap bails cleanly on both sides over a
/// one-byte-window connection, its fixed-width greeting exchange included.
#[test]
fn mutual_bootstrap() {
    block_on(mutual_bootstrap_session());
}

divided_proptest! {
    #![divisor(FLEET_DIVISOR)]

    /// A V2 retirement into a bootstrapper stays live over a one-byte-window
    /// connection and hands the newcomer the retiree's exact content.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn retire_into_bootstrapper(network in network_seed()) {
        block_on(retire_into_bootstrapper_session(network));
    }

    /// A mutual V2 retirement early-outs cleanly over a one-byte-window connection,
    /// declining both sides.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn mutual_retire(network in network_seed()) {
        block_on(mutual_retire_session(network));
    }
}
