//! Integration tests for `rumors::Peer::retire` (and its synchronous twin
//! `sync::Peer::retire`): a peer hands its ITC party to a peer, so its
//! id-region is reclaimed rather than leaked.
//!
//! A retire session begins with a round of gossip — the ordinary mirror
//! descent — so the absorbing peer comes to causally dominate the retiree
//! before the party changes hands, and nothing the retiree held is lost.
//! The assertions here are about *outcomes* (the [`Retire`] variants),
//! content survival across the hand-off, and the invariants the absorbing
//! peer must preserve when the retiree is already converged (its tree and
//! version are untouched). Declines remain only for a counterparty that is
//! itself retiring; a bootstrapping counterparty *absorbs* the retiree —
//! it receives the whole tree through the descent and the whole party as
//! the trailing frame, becoming the retiree's successor.
//!
//! (The old typestate-era tests of a retire refused by outstanding
//! snapshots have no equivalent: the `Peer`/`Rumors` XOR makes "retire
//! while observers share the party" unrepresentable at compile time. The
//! party-accounting side — every retire reconstituting the seed's whole
//! id-space — lives in the crate-level tests, which can read the party.)

mod common;

use proptest::prelude::*;
use rumors::{Peer, Retire, Rumors, causally};

use crate::common::action::{LocalAction, arb_local_actions, build_local, build_local_async};
use crate::common::oracle::readout;
use crate::common::sync_wire::{sync_bootstrap_fork, sync_wire_gossip};
use crate::common::wire::{block_on, bootstrap_fork, wire_gossip};

/// Capacity for the in-memory duplex pipe. A divergent retiree's session moves
/// content through the gossip round, so keep the other wire tests' headroom.
const DUPLEX_BUF: usize = 64 * 1024;

// ---- builders ------------------------------------------------------------

/// Build an async `Rumors<u64>` by inserting `vals` into a disjoint originator
/// (a genuine bootstrap fork: its own party region, ready to originate).
fn async_known(peer: Rumors<u64>, vals: &[u64]) -> Rumors<u64> {
    let actions: Vec<LocalAction<u64>> = vals.iter().map(|&v| LocalAction::Insert(v)).collect();
    build_local_async(peer, &actions)
}

/// Build a synchronous `sync::Rumors<u64>` by inserting `vals` into a disjoint
/// originator (a genuine bootstrap fork: its own party region).
fn sync_known(peer: rumors::sync::Rumors<u64>, vals: &[u64]) -> rumors::sync::Rumors<u64> {
    let actions: Vec<LocalAction<u64>> = vals.iter().map(|&v| LocalAction::Insert(v)).collect();
    build_local(peer, &actions)
}

// ---- wire harnesses ------------------------------------------------------

/// Drive `retiree.retire` against `peer.gossip` concurrently over a duplex
/// pipe, returning the retiree's outcome. The retiree arrives as the sole
/// `Rumors` handle on its set and is converted into the unique `Peer`
/// retirement requires.
fn retire_into_gossip(retiree: Rumors<u64>, peer: &Rumors<u64>) -> Retire<u64> {
    block_on(async move {
        let retiree = retiree
            .try_into_peer()
            .await
            .expect("the sole handle reclaims the Peer");
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        let (retire_out, gossip_out) = tokio::join!(
            retiree.retire(&mut a_r, &mut a_w),
            peer.gossip(&mut b_r, &mut b_w),
        );
        gossip_out.expect("gossiping peer");
        retire_out
    })
}

/// Like [`retire_into_gossip`], but also counts the novel messages the
/// session delivered into the gossiping peer (the live leaves above the
/// peer's pre-session frontier).
fn retire_into_counting_gossip(retiree: Rumors<u64>, peer: &Rumors<u64>) -> (Retire<u64>, usize) {
    let pre = peer.snapshot().latest().clone();
    let outcome = retire_into_gossip(retiree, peer);
    let novel = peer.snapshot().range(causally::since(&pre)).count();
    (outcome, novel)
}

/// Drive `a.retire` against `b.retire` concurrently: a mutual retirement.
fn retire_into_retire(a: Rumors<u64>, b: Rumors<u64>) -> (Retire<u64>, Retire<u64>) {
    block_on(async move {
        let a = a.try_into_peer().await.expect("a's sole handle");
        let b = b.try_into_peer().await.expect("b's sole handle");
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        tokio::join!(a.retire(&mut a_r, &mut a_w), b.retire(&mut b_r, &mut b_w))
    })
}

/// Drive `retiree.retire` against a fresh `bootstrap`. Returns the retiree's
/// outcome and the bootstrapper's successor (as a data-plane handle).
fn retire_into_bootstrap(retiree: Rumors<u64>) -> (Retire<u64>, Option<Rumors<u64>>) {
    block_on(async move {
        let retiree = retiree
            .try_into_peer()
            .await
            .expect("the sole handle reclaims the Peer");
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        let (retire_out, boot_out) = tokio::join!(
            retiree.retire(&mut a_r, &mut a_w),
            Peer::<u64>::bootstrap(&mut b_r, &mut b_w),
        );
        (
            retire_out,
            boot_out.expect("bootstrapper").map(Peer::into_rumors),
        )
    })
}

/// Synchronous counterpart of [`retire_into_gossip`]: one peer per OS thread,
/// connected by a pair of `std::io::pipe`s, exactly as `sync_wire_gossip` does.
fn sync_retire_into_gossip(
    retiree: rumors::sync::Rumors<u64>,
    peer: &mut rumors::sync::Rumors<u64>,
) -> rumors::sync::Retire<u64> {
    let retiree = retiree
        .try_into_peer()
        .expect("the sole handle reclaims the Peer");
    let (mut a_to_b_r, mut a_to_b_w) = std::io::pipe().expect("pipe a→b");
    let (mut b_to_a_r, mut b_to_a_w) = std::io::pipe().expect("pipe b→a");

    std::thread::scope(|s| {
        let peer_thread = s.spawn(move || {
            peer.gossip(&mut a_to_b_r, &mut b_to_a_w)
                .expect("sync gossiping peer")
        });
        let retire_out = retiree.retire(&mut b_to_a_r, &mut a_to_b_w);
        peer_thread.join().expect("join peer thread");
        retire_out
    })
}

// ---- async behavioral tests ---------------------------------------------

/// Retiring into a peer that has gossiped to convergence (equal versions, so it
/// reflexively dominates) succeeds: the retiree is consumed ([`Retire::Retired`])
/// and the absorbing peer's tree and version are untouched (no content crosses).
#[test]
fn retire_into_converged_peer_succeeds() {
    let seed = Peer::<u64>::seed().into_rumors();
    let a = async_known(bootstrap_fork(&seed), &[1, 2]);
    let b = async_known(seed, &[3, 4]);

    wire_gossip(&a, &b);
    let pre = b.snapshot();
    let (hash, version) = (pre.hash(), pre.latest().clone());

    let outcome = retire_into_gossip(a, &b);
    assert!(
        matches!(outcome, Retire::Retired),
        "a dominating peer absorbs the retiree, got {outcome:?}"
    );
    let post = b.snapshot();
    assert_eq!(post.hash(), hash, "absorbing a retiree moves no content");
    assert_eq!(
        post.latest(),
        &version,
        "absorbing a retiree is a version no-op"
    );
}

/// Equal versions satisfy the `<=` domination precondition reflexively: a
/// fresh, empty bootstrap fork can retire into the peer it forked from with
/// no prior gossip.
#[test]
fn empty_equal_version_retire_succeeds() {
    let seed = Peer::<u64>::seed().into_rumors();
    let a = bootstrap_fork(&seed);
    let b = seed;

    let outcome = retire_into_gossip(a, &b);
    assert!(
        matches!(outcome, Retire::Retired),
        "equal versions dominate reflexively, so retire commits; got {outcome:?}"
    );
}

/// A retiree whose peer does *not* dominate it (the two diverged concurrently)
/// is not declined: the session's gossip round reconciles the two, after
/// which the peer dominates by construction and absorbs the retiree. Nothing
/// either side held is lost.
#[test]
fn divergent_retiree_reconciles_then_retires() {
    let seed = Peer::<u64>::seed().into_rumors();
    let a = async_known(bootstrap_fork(&seed), &[1]);
    let b = async_known(seed, &[2]);

    let outcome = retire_into_gossip(a, &b);
    assert!(
        matches!(outcome, Retire::Retired),
        "the in-session gossip round brings the peer to dominance, got {outcome:?}"
    );
    let mut live: Vec<u64> = b.snapshot().iter().map(|(_, _, m)| **m).collect();
    live.sort_unstable();
    assert_eq!(
        live,
        vec![1, 2],
        "the retiree's content survives in the absorber"
    );
}

/// A redaction the retiree performed locally propagates through retirement's
/// gossip round: the absorber evicts the message before absorbing the party,
/// exactly as a plain gossip session would have spread it.
#[test]
fn retiree_redaction_propagates_through_retire() {
    // Both peers hold 1 and 2 (inserted before the fork, so the keys are
    // shared); the retiree then redacts 1 while the peer inserts 3.
    let seed = Peer::<u64>::seed().into_rumors();
    seed.batch().send(1).send(2);
    let key_of_1 = seed
        .snapshot()
        .iter()
        .find_map(|(k, _, m)| (**m == 1).then_some(k))
        .expect("key recorded for 1");

    let a = bootstrap_fork(&seed);
    let b = async_known(seed, &[3]);

    a.redact(key_of_1);

    let outcome = retire_into_gossip(a, &b);
    assert!(
        matches!(outcome, Retire::Retired),
        "the reconciled peer absorbs the retiree, got {outcome:?}"
    );
    let mut live: Vec<u64> = b.snapshot().iter().map(|(_, _, m)| **m).collect();
    live.sort_unstable();
    assert_eq!(
        live,
        vec![2, 3],
        "the retiree's redaction evicts 1 from the absorber"
    );
}

/// Two peers that both try to retire into each other both decline: each sees
/// the other's retire-intent in the preamble and refuses to absorb a peer
/// that is itself leaving. Both are handed back intact.
#[test]
fn mutual_retire_declines() {
    let seed = Peer::<u64>::seed().into_rumors();
    let a = async_known(bootstrap_fork(&seed), &[1, 2]);
    let b = async_known(seed, &[3, 4]);

    let a_hash = a.snapshot().hash();
    let b_hash = b.snapshot().hash();
    let (ra, rb) = retire_into_retire(a, b);
    let (Retire::Declined { peer: a }, Retire::Declined { peer: b }) = (ra, rb) else {
        panic!("mutual retirement must decline both sides intact");
    };
    let (a, b) = (a.into_rumors(), b.into_rumors());
    assert_eq!(
        a.snapshot().hash(),
        a_hash,
        "declined retiree A is handed back intact"
    );
    assert_eq!(
        b.snapshot().hash(),
        b_hash,
        "declined retiree B is handed back intact"
    );

    // Both parties are still live and disjoint: a clean retire of one into
    // the other commits.
    let outcome = retire_into_gossip(a, &b);
    assert!(
        matches!(outcome, Retire::Retired),
        "a declined retire leaves both parties whole, got {outcome:?}"
    );
}

/// A retiree that meets a *bootstrapping* counterparty is absorbed by it:
/// the newcomer pulls the retiree's whole tree through the descent, then
/// receives its whole party as the trailing frame — it *becomes* the
/// retiree, in the same universe, and its subsequent originations are
/// first-class.
#[test]
fn retire_into_bootstrapper_hands_off_the_identity() {
    let seed = Peer::<u64>::seed().into_rumors();
    let retiree = async_known(bootstrap_fork(&seed), &[1, 2]);
    let network = retiree.network();
    let content = readout(&retiree.snapshot());

    let (outcome, successor) = retire_into_bootstrap(retiree);
    assert!(
        matches!(outcome, Retire::Retired),
        "a bootstrapper absorbs the retiree, got {outcome:?}"
    );
    let successor = successor.expect("the retiree served the bootstrap");
    assert_eq!(
        successor.network(),
        network,
        "the successor joins the universe"
    );
    assert_eq!(
        readout(&successor.snapshot()),
        content,
        "the successor holds the retiree's content"
    );

    // The inherited party is live: an origination from the successor
    // survives gossip with the rest of the universe.
    successor.send(99);
    wire_gossip(&successor, &seed);
    assert!(
        seed.snapshot().iter().any(|(_, _, m)| **m == 99),
        "the successor's origination survives gossip"
    );
}

/// Ordinary `gossip` learns a divergent retiree's novel content through the
/// session's gossip round — the messages land above the absorber's
/// pre-session frontier, observable to any checkpoint — before absorbing the
/// party.
#[test]
fn gossip_learns_content_from_divergent_retiree() {
    let seed = Peer::<u64>::seed().into_rumors();
    let a = async_known(bootstrap_fork(&seed), &[1, 2]);
    let b = async_known(seed, &[3]);

    let (outcome, novel) = retire_into_counting_gossip(a, &b);
    assert!(
        matches!(outcome, Retire::Retired),
        "the reconciled peer absorbs the retiree, got {outcome:?}"
    );
    assert_eq!(
        novel, 2,
        "the absorber observes each of the retiree's novel messages"
    );
    assert_eq!(
        b.snapshot().len(),
        3,
        "the absorber holds the union of live content"
    );
}

/// Ordinary `gossip` transparently absorbs an already-converged retiree: the
/// gossiping peer ends `Ok(())`, observes *zero* novel messages (no content
/// moves when it already dominates), and its tree and version are unchanged.
#[test]
fn gossip_absorbs_retiree_without_observations() {
    let seed = Peer::<u64>::seed().into_rumors();
    let a = async_known(bootstrap_fork(&seed), &[1, 2]);
    let b = async_known(seed, &[3, 4]);

    wire_gossip(&a, &b);
    let pre = b.snapshot();
    let (hash, version) = (pre.hash(), pre.latest().clone());

    let (outcome, novel) = retire_into_counting_gossip(a, &b);
    assert!(
        matches!(outcome, Retire::Retired),
        "the gossiping peer absorbs the retiree, got {outcome:?}"
    );
    assert_eq!(novel, 0, "absorbing a retiree delivers no messages");
    let post = b.snapshot();
    assert_eq!(post.hash(), hash, "absorbing a retiree moves no content");
    assert_eq!(
        post.latest(),
        &version,
        "absorbing a retiree is a version no-op"
    );
}

// ---- synchronous parity tests -------------------------------------------

/// The synchronous `retire` surface behaves like the async one: retiring into a
/// converged (dominating) peer succeeds and leaves that peer's tree and version
/// untouched.
#[test]
fn sync_retire_into_converged_peer_succeeds() {
    let mut seed = rumors::sync::Peer::<u64>::seed().into_rumors();
    let mut a = sync_known(sync_bootstrap_fork(&mut seed), &[1, 2]);
    let mut b = sync_known(seed, &[3]);

    sync_wire_gossip(&mut a, &mut b);
    let pre = b.snapshot();
    let (hash, version) = (pre.hash(), pre.latest().clone());

    let outcome = sync_retire_into_gossip(a, &mut b);
    assert!(
        matches!(outcome, rumors::sync::Retire::Retired),
        "a dominating peer absorbs the retiree, got {outcome:?}"
    );
    let post = b.snapshot();
    assert_eq!(post.hash(), hash, "absorbing a retiree moves no content");
    assert_eq!(
        post.latest(),
        &version,
        "absorbing a retiree is a version no-op"
    );
}

/// Synchronous divergence parity: over the blocking wire, the retire session's
/// gossip round reconciles a divergent pair before the absorbing peer takes
/// the retiree's party, and the retiree's content survives.
#[test]
fn sync_divergent_retiree_reconciles_then_retires() {
    let mut seed = rumors::sync::Peer::<u64>::seed().into_rumors();
    let a = sync_known(sync_bootstrap_fork(&mut seed), &[1]);
    let mut b = sync_known(seed, &[2]);

    let outcome = sync_retire_into_gossip(a, &mut b);
    assert!(
        matches!(outcome, rumors::sync::Retire::Retired),
        "the in-session gossip round brings the peer to dominance, got {outcome:?}"
    );
    let mut live: Vec<u64> = b.snapshot().iter().map(|(_, _, m)| **m).collect();
    live.sort_unstable();
    assert_eq!(
        live,
        vec![1, 2],
        "the retiree's content survives in the absorber"
    );
}

// ---- wire-equivalence property tests -------------------------------------

proptest! {
    /// Retiring A into B over the wire (after gossiping to convergence)
    /// leaves B with the same live content (`hash`) and causal version
    /// (`latest`) as a plain gossip session in an identically-built
    /// universe: the party hand-off moves no content and no version. Two
    /// independently-seeded universes built from identical action sequences
    /// are compared; `hash`/`latest` are network-independent, so the
    /// distinct `Network` ids do not perturb the comparison.
    #[test]
    fn retire_matches_plain_gossip(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        // Wire path: converge, then retire A into B.
        let (retire_hash, retire_version) = {
            let seed = Peer::<u64>::seed().into_rumors();
            let a = build_local_async(bootstrap_fork(&seed), &a_actions);
            let b = build_local_async(seed, &b_actions);
            wire_gossip(&a, &b);
            let outcome = retire_into_gossip(a, &b);
            prop_assert!(
                matches!(outcome, Retire::Retired),
                "a converged peer dominates, so retire commits; got {outcome:?}",
            );
            let snapshot = b.snapshot();
            (snapshot.hash(), snapshot.latest().clone())
        };

        // Oracle: a plain gossip session in an identically-built universe.
        let (gossip_hash, gossip_version) = {
            let seed = Peer::<u64>::seed().into_rumors();
            let a = build_local_async(bootstrap_fork(&seed), &a_actions);
            let b = build_local_async(seed, &b_actions);
            wire_gossip(&a, &b);
            let snapshot = b.snapshot();
            (snapshot.hash(), snapshot.latest().clone())
        };

        prop_assert_eq!(
            retire_hash, gossip_hash,
            "retire-over-wire leaves the same live content as plain gossip"
        );
        prop_assert_eq!(
            retire_version, gossip_version,
            "retire-over-wire leaves the same causal version as plain gossip"
        );
    }

    /// Retiring A into B with *no prior synchronization* also matches the
    /// plain-gossip oracle: the gossip round inside the retire session
    /// performs the reconciliation itself.
    #[test]
    fn unsynchronized_retire_matches_plain_gossip(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        // Wire path: retire A into B directly, while they may still diverge.
        let (retire_hash, retire_version) = {
            let seed = Peer::<u64>::seed().into_rumors();
            let a = build_local_async(bootstrap_fork(&seed), &a_actions);
            let b = build_local_async(seed, &b_actions);
            let outcome = retire_into_gossip(a, &b);
            prop_assert!(
                matches!(outcome, Retire::Retired),
                "the in-session gossip round always brings the peer to dominance; got {outcome:?}",
            );
            let snapshot = b.snapshot();
            (snapshot.hash(), snapshot.latest().clone())
        };

        // Oracle: a plain gossip session in an identically-built universe.
        let (gossip_hash, gossip_version) = {
            let seed = Peer::<u64>::seed().into_rumors();
            let a = build_local_async(bootstrap_fork(&seed), &a_actions);
            let b = build_local_async(seed, &b_actions);
            wire_gossip(&a, &b);
            let snapshot = b.snapshot();
            (snapshot.hash(), snapshot.latest().clone())
        };

        prop_assert_eq!(
            retire_hash, gossip_hash,
            "unsynchronized retire leaves the same live content as plain gossip"
        );
        prop_assert_eq!(
            retire_version, gossip_version,
            "unsynchronized retire leaves the same causal version as plain gossip"
        );
    }
}
