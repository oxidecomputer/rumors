//! Integration tests for `rumors::Peer::retire`: a peer hands its ITC party
//! to a counterparty, so the identity space it held is reclaimed rather
//! than leaked.
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
//! Retirement consumes the unique `Peer`, excluding concurrent writers.
//! Snapshot and message observers may remain and drain its final state.

mod common;

use std::collections::BTreeMap;

use proptest::prelude::*;
use rumors::{Peer, Retire, Rumors, causally};

use crate::common::action::{LocalAction, arb_local_actions, build_local};
use crate::common::fault::{self, FaultPlan};
use crate::common::observer::{Step, drain, step};
use crate::common::oracle::readout;
use crate::common::wire::{assert_control_drained, block_on, bootstrap_fork, wire_gossip};
use rumors::testing::run_to_quiescence;

/// Capacity for each in-memory link stream. A divergent retiree's session moves
/// content through the gossip round, so keep the other wire tests' headroom.
const LINK_BUF: usize = 64 * 1024;

/// Send a fixed fixture's messages and return the same handle.
fn with_messages(peer: Rumors<u64>, messages: &[u64]) -> Rumors<u64> {
    peer.send_all(messages.iter().copied()).unwrap();
    peer
}

/// Network-independent state used to compare equivalent sessions.
#[derive(Debug, PartialEq, Eq)]
struct Fingerprint {
    /// The Merkle root of the live message set.
    hash: Vec<u8>,
    /// The complete causal frontier.
    latest: rumors::Version,
}

/// Capture the live content and causal frontier of a replica.
fn fingerprint(peer: &Rumors<u64>) -> Fingerprint {
    let snapshot = peer.snapshot();
    Fingerprint {
        hash: snapshot.hash().to_vec(),
        latest: snapshot.latest().clone(),
    }
}

/// Fingerprint plain gossip for comparison with an equivalent retirement.
fn plain_gossip_fingerprint(
    a_actions: &[LocalAction<u64>],
    b_actions: &[LocalAction<u64>],
) -> Fingerprint {
    let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
    let a = build_local(bootstrap_fork(&seed), a_actions);
    let b = build_local(seed, b_actions);
    wire_gossip(&a, &b);
    fingerprint(&b)
}

/// Retire the sole handle into a gossiping peer and check the session boundary.
/// Message observers may outlive the retiring replica.
fn retire_into_gossip<T>(retiree: Rumors<T>, peer: &Rumors<T>) -> Retire<T>
where
    T: Send + Sync + 'static,
{
    block_on(async move {
        let retiree = retiree
            .try_into_peer()
            .await
            .expect("the sole handle reclaims the Peer");
        let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(LINK_BUF);
        let (retire_out, gossip_out) =
            tokio::join!(retiree.retire(&mut a_link), peer.gossip_once(&mut b_link),);
        gossip_out.expect("gossiping peer");
        assert_control_drained(a_link, b_link);
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
        let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(LINK_BUF);
        let outcome = tokio::join!(a.retire(&mut a_link), b.retire(&mut b_link));
        assert_control_drained(a_link, b_link);
        outcome
    })
}

/// Drive `retiree.retire` against a fresh `bootstrap`. Returns the retiree's
/// outcome and the bootstrapper's successor (as a data-plane handle).
fn retire_into_bootstrap(retiree: Rumors<u64>) -> (Retire<u64>, Rumors<u64>) {
    block_on(async move {
        let retiree = retiree
            .try_into_peer()
            .await
            .expect("the sole handle reclaims the Peer");
        let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(LINK_BUF);
        let (retire_out, boot_out) = tokio::join!(
            retiree.retire(&mut a_link),
            Peer::<u64>::bootstrap().join(&mut b_link),
        );
        assert_control_drained(a_link, b_link);
        let rumors::Joined::Joined { peer } = boot_out else {
            panic!("the retiree must serve the bootstrap");
        };
        (retire_out, peer.sync_window_floor().into_rumors())
    })
}

/// A fresh, empty bootstrap fork can retire into its provider without prior
/// gossip because their content and causal frontiers already agree.
#[test]
fn empty_equal_version_retire_succeeds() {
    let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
    let a = bootstrap_fork(&seed);
    let b = seed;

    let outcome = retire_into_gossip(a, &b);
    assert!(
        matches!(outcome, Retire::Retired),
        "equal versions dominate reflexively, so retire commits; got {outcome:?}"
    );
}

/// A redaction the retiree performed locally propagates through retirement's
/// gossip round: the absorber evicts the message before absorbing the party,
/// exactly as a plain gossip session would have spread it.
#[test]
fn retiree_redaction_propagates_through_retire() {
    // Both peers hold 1 and 2 (inserted before the fork, so the messages
    // and their versions are shared); the retiree then redacts 1 while the
    // peer inserts 3.
    let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
    seed.send_all([1, 2]).unwrap();
    let version_of_1 = seed
        .snapshot()
        .iter()
        .find_map(|(v, m)| (*m == 1).then_some(v.clone()))
        .expect("version recorded for 1");

    let a = bootstrap_fork(&seed);
    let b = with_messages(seed, &[3]);

    a.redact(&version_of_1);

    let outcome = retire_into_gossip(a, &b);
    assert!(
        matches!(outcome, Retire::Retired),
        "the reconciled peer absorbs the retiree, got {outcome:?}"
    );
    let mut live: Vec<u64> = b.snapshot().iter().map(|(_, m)| *m).collect();
    live.sort_unstable();
    assert_eq!(
        live,
        vec![2, 3],
        "the retiree's redaction evicts 1 from the absorber"
    );
}

/// A retiree's later redaction of a message it originated and previously
/// shared reaches the absorber during retirement.
///
/// Unlike the shared-before-fork case above, the absorber learned this version
/// directly from the retiree. Retirement must carry the newer causal frontier
/// that makes the now-absent message stay absent.
#[test]
fn retiree_redaction_of_previously_shared_message_propagates() {
    let seed = Peer::<String>::seed().sync_window_floor().into_rumors();
    let retiree = bootstrap_fork(&seed);
    let absorber = seed;
    let version = retiree.send("presence".to_owned()).unwrap();

    wire_gossip(&retiree, &absorber);
    assert!(absorber.snapshot().contains(&version));
    retiree.redact(&version);

    let outcome = retire_into_gossip(retiree, &absorber);
    assert!(matches!(outcome, Retire::Retired), "{outcome:?}");
    assert!(
        !absorber.snapshot().contains(&version),
        "the retiree's later frontier must preserve the redaction"
    );
}

/// Two peers that both try to retire into each other both decline: each sees
/// the other's retire-intent in the preamble and refuses to absorb a peer
/// that is itself leaving. Both are handed back intact.
#[test]
fn mutual_retire_declines() {
    let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
    let a = with_messages(bootstrap_fork(&seed), &[1, 2]);
    let b = with_messages(seed, &[3, 4]);

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

/// A bootstrapper succeeds a retiree without duplicating or losing its party.
///
/// The successor receives the retiree's complete content and party. Together
/// with the untouched seed peer, its party therefore remains disjoint and the
/// two parties reconstruct the network's complete identity space.
#[test]
fn retire_into_bootstrapper_hands_off_the_identity() {
    let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
    let retiree = with_messages(bootstrap_fork(&seed), &[1, 2]);
    let network = retiree.network();
    let content = readout(&retiree.snapshot());

    let (outcome, successor) = retire_into_bootstrap(retiree);
    assert!(
        matches!(outcome, Retire::Retired),
        "a bootstrapper absorbs the retiree, got {outcome:?}"
    );
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
    crate::common::sim::assert_party_invariants(&[seed.clone(), successor.clone()], 0);

    // Exercise the transferred party through the public behavior as well.
    successor.send(99).unwrap();
    wire_gossip(&successor, &seed);
    assert!(
        seed.snapshot().iter().any(|(_, m)| *m == 99),
        "the successor's origination survives gossip"
    );
}

/// Ordinary `gossip` learns a divergent retiree's novel content through the
/// session's gossip round — the messages land above the absorber's
/// pre-session frontier, observable to any checkpoint — before absorbing the
/// party.
#[test]
fn gossip_learns_content_from_divergent_retiree() {
    let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
    let a = with_messages(bootstrap_fork(&seed), &[1, 2]);
    let b = with_messages(seed, &[3]);

    let (outcome, novel) = retire_into_counting_gossip(a, &b);
    assert!(
        matches!(outcome, Retire::Retired),
        "the reconciled peer absorbs the retiree, got {outcome:?}"
    );
    assert_eq!(
        novel, 2,
        "the absorber observes each of the retiree's novel messages"
    );
    let mut live: Vec<_> = b.snapshot().iter().map(|(_, message)| *message).collect();
    live.sort_unstable();
    assert_eq!(live, [1, 2, 3], "the absorber holds the exact union");
}

/// Ordinary `gossip` transparently absorbs an already-converged retiree: the
/// gossiping peer ends `Ok(())`, observes *zero* novel messages (no content
/// moves when it already dominates), and its tree and version are unchanged.
#[test]
fn gossip_absorbs_retiree_without_observations() {
    let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
    let a = with_messages(bootstrap_fork(&seed), &[1, 2]);
    let b = with_messages(seed, &[3, 4]);

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

proptest! {
    /// Retirement publishes the reconciled state before either message observer ends.
    ///
    /// Both replicas retain new messages from either side and honor shared redactions;
    /// the retiring replica's observers deliver that final state exactly once.
    #[test]
    fn retirement_publishes_final_state_before_observers_end(
        shared in prop::collection::vec(any::<u64>(), 0..8),
        retiree_sends in prop::collection::vec(any::<u64>(), 0..8),
        survivor_sends in prop::collection::vec(any::<u64>(), 1..8),
        redactions in prop::collection::vec((any::<bool>(), any::<prop::sample::Index>()), 0..10),
    ) {
        let survivor = Peer::<u64>::seed().sync_window_floor().into_rumors();
        survivor.send_all(shared).unwrap();
        let retiree = bootstrap_fork(&survivor);
        let shared_versions: Vec<_> = survivor.snapshot().versions().cloned().collect();
        let mut expected = readout(&survivor.snapshot());
        let mut unordered = retiree.unordered_messages();
        let mut causal = retiree.causal_messages();

        // The survivor always sends something new: returning the retiree's
        // old state must miss it. Record sends before any reconciliation so
        // the expected set does not depend on gossip.
        for (peer, values) in [(&retiree, retiree_sends), (&survivor, survivor_sends)] {
            for value in values {
                let version = peer.send(value).unwrap();
                expected.insert(version.as_bytes().to_vec(), value);
            }
        }
        for (remote, index) in redactions {
            if !shared_versions.is_empty() {
                let version = &shared_versions[index.index(shared_versions.len())];
                let peer = if remote { &survivor } else { &retiree };
                peer.redact(version);
                expected.remove(version.as_bytes());
            }
        }
        let mut latest = retiree.snapshot().latest().clone();
        latest |= survivor.snapshot().latest();
        let outcome = retire_into_gossip(retiree, &survivor);
        prop_assert!(matches!(outcome, Retire::Retired), "{outcome:?}");
        prop_assert_eq!(readout(&survivor.snapshot()), expected.clone());
        prop_assert_eq!(survivor.snapshot().latest().clone(), latest.clone());

        // Neither observer has captured a pass yet. It must see the final
        // reconciliation, not the retiring replica's pre-session snapshot.
        let (unordered_items, unordered_ended) = drain(&mut unordered);
        let (causal_items, causal_ended) = drain(&mut causal);
        prop_assert!(unordered_ended && causal_ended);
        prop_assert_eq!(step(&mut unordered), Step::Ended);
        prop_assert_eq!(step(&mut causal), Step::Ended);
        prop_assert_eq!(unordered.checkpoint(), &latest);
        prop_assert_eq!(causal.checkpoint(), &latest);
        for (name, items) in [("unordered", unordered_items), ("causal", causal_items)] {
            prop_assert_eq!(items.len(), expected.len(), "{}: one delivery per live version", name);
            let actual: BTreeMap<_, _> = items.into_iter()
                .map(|(version, value)| (version.as_bytes().to_vec(), value)).collect();
            prop_assert_eq!(&actual, &expected, "{}: complete final content", name);
        }
    }
}

proptest! {
    /// Retiring A into B over the wire (after gossiping to convergence)
    /// leaves B with the same live content (`hash`) and causal version
    /// (`latest`) as a plain gossip session in an identically-built
    /// universe.
    ///
    /// The party hand-off moves no content and no version. Two
    /// independently-seeded universes built from identical action sequences
    /// are compared; `hash`/`latest` are network-independent, so the
    /// distinct `Network` ids do not perturb the comparison.
    #[test]
    fn retire_matches_plain_gossip(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        // Wire path: converge, then retire A into B.
        let actual = {
            let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
            let a = build_local(bootstrap_fork(&seed), &a_actions);
            let b = build_local(seed, &b_actions);
            wire_gossip(&a, &b);
            let outcome = retire_into_gossip(a, &b);
            prop_assert!(
                matches!(outcome, Retire::Retired),
                "a converged peer dominates, so retire commits; got {outcome:?}",
            );
            fingerprint(&b)
        };

        let expected = plain_gossip_fingerprint(&a_actions, &b_actions);
        prop_assert_eq!(
            actual, expected,
            "retire-over-wire must match plain gossip",
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
        let actual = {
            let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
            let a = build_local(bootstrap_fork(&seed), &a_actions);
            let b = build_local(seed, &b_actions);
            let outcome = retire_into_gossip(a, &b);
            prop_assert!(
                matches!(outcome, Retire::Retired),
                "the in-session gossip round always brings the peer to dominance; got {outcome:?}",
            );
            fingerprint(&b)
        };

        let expected = plain_gossip_fingerprint(&a_actions, &b_actions);
        prop_assert_eq!(
            actual, expected,
            "unsynchronized retire must match plain gossip",
        );
    }
}

/// Run one converged retirement, measuring a clean send or severing it at
/// `cut`. Any returned peer must remain usable and disjoint from the absorber.
fn retirement_cut(cut: Option<usize>, initial: usize) -> usize {
    let absorber = Peer::<u64>::seed().into_rumors();
    absorber.send_all(0..initial as u64).unwrap();
    let retiree = bootstrap_fork(&absorber);
    let original = retiree.dangerously_alias_party();
    run_to_quiescence(async {
        let retiree = retiree.try_into_peer().await.unwrap();
        let (retiring, mut receiving) = rumors::link::memory();
        let (mut link, meter) = if let Some(cut) = cut {
            (
                fault::faulty(
                    retiring,
                    FaultPlan {
                        write_cut: Some(cut),
                        ..FaultPlan::NONE
                    },
                ),
                None,
            )
        } else {
            let (link, meter) = fault::metered(retiring);
            (link, Some(meter))
        };
        let outgoing = async move { retiree.retire(&mut link).await };
        let (retired, received) = tokio::join!(outgoing, absorber.gossip_once(&mut receiving));
        if cut.is_none() {
            assert!(
                matches!(retired, Retire::Retired),
                "clean retirement: {retired:?}"
            );
        }
        match retired {
            Retire::Recovered { peer, .. } => {
                assert_eq!(peer.dangerously_alias_party(), original);
                assert!(original.is_disjoint(&absorber.dangerously_alias_party()));
                let live = peer.into_rumors();
                live.send(999).unwrap();
                assert_eq!(live.snapshot().len(), initial + 1);
            }
            Retire::Retired => {
                if cut.is_none() {
                    received.unwrap();
                }
                assert!(absorber.dangerously_alias_party().is_seed());
            }
            Retire::Uncertain { .. } => {}
            Retire::Declined { .. } => panic!("ordinary gossip accepts retirement"),
        }
        meter.map_or(0, |meter| meter.written())
    })
    .expect("a cut retirement must not deadlock")
}

proptest! {
    /// A cut anywhere in retirement's outgoing traffic never returns an
    /// identity the absorber holds; a recovered peer can still commit events.
    #[test]
    fn retirement_write_cuts_preserve_exclusive_identity(
        initial in 0usize..6,
        offset in any::<proptest::sample::Index>(),
    ) {
        let extent = retirement_cut(None, initial);
        retirement_cut(Some(offset.index(extent + 1)), initial);
    }
}
