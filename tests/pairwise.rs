//! Pairwise gossip semantics for `Rumors::gossip_once`, the wire merge primitive.
//!
//! With the shared-state rumor set, wire gossip *is* the merge: there is no
//! in-process `join`. These properties pin the algebraic laws of one
//! bidirectional session — convergence, side-symmetry, idempotence,
//! order-independence across three peers, and the union of live content —
//! plus the causal-concurrency basics the merge rests on.
//!
//! Live content is compared through `readout` (the identity → value lens
//! the oracle checks also use), or through [`Snapshot`](rumors::Snapshot)
//! equality where the assertion is "nothing changed at all".
//!
//! Every peer in a test is a genuine, party-disjoint fork of one shared
//! [`Peer::seed`](rumors::Peer::seed), created by [`bootstrap_fork`]. They
//! share a [`Network`](rumors::Network) but tick disjoint parties, so their
//! concurrent inserts stay incomparable and gossip between them never
//! fails.

mod common;

use std::fmt::Debug;

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestCaseResult;
use proptest::test_runner::TestRunner;
use rumors::{Rumors, Snapshot, causally};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::common::action::{LocalAction, arb_local_actions, arb_string_actions, build_local};
use crate::common::oracle::{readout, readout_multiset};
use crate::common::wire::{bootstrap_fork, wire_gossip};

/// Maximum values generated for each side of the carrier-path property.
const MAX_CARRIER_VALUES: usize = 8;

/// A genuine, party-disjoint copy of `k`'s content: a fresh originator that
/// holds the same live messages but ticks its own party region.
fn dup<T>(k: &Rumors<T>) -> Rumors<T>
where
    T: Clone + Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    bootstrap_fork(k)
}

/// The state used for fixed-point checks: equal snapshots make gossip a no-op.
fn fingerprint<T: Send + Sync + 'static>(k: &Rumors<T>) -> Snapshot<T> {
    k.snapshot()
}

/// Check that one session produces the exact union of two local histories.
fn assert_gossip_unions<T>(
    a_actions: &[LocalAction<T>],
    b_actions: &[LocalAction<T>],
) -> TestCaseResult
where
    T: Clone + Debug + Ord + Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    let seed = rumors::Peer::<T>::seed().sync_window_floor().into_rumors();
    let a = build_local(dup(&seed), a_actions);
    let b = build_local(dup(&seed), b_actions);

    let mut expected = readout(&a.snapshot());
    expected.extend(readout(&b.snapshot()));

    wire_gossip(&a, &b);

    prop_assert_eq!(readout(&a.snapshot()), expected.clone());
    prop_assert_eq!(readout(&b.snapshot()), expected);
    prop_assert_eq!(fingerprint(&a), fingerprint(&b));
    Ok(())
}

proptest! {
    /// After one bidirectional gossip session, the two peers' live
    /// content (as exposed through `readout`) is equal.
    #[test]
    fn gossip_converges(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let seed = rumors::Peer::<u64>::seed().sync_window_floor().into_rumors();
        let a = build_local(dup(&seed), &a_actions);
        let b = build_local(dup(&seed), &b_actions);
        wire_gossip(&a, &b);
        prop_assert_eq!(readout(&a.snapshot()), readout(&b.snapshot()));
        prop_assert_eq!(fingerprint(&a), fingerprint(&b));
    }

    /// The converged pair is independent of which peer sits on which end
    /// of the link: gossiping `(a, b)` and gossiping `(b, a)` from
    /// identically-built starting points yield the same content on both
    /// sides.
    #[test]
    fn gossip_side_symmetric(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let seed = rumors::Peer::<u64>::seed().sync_window_floor().into_rumors();
        let a0 = build_local(dup(&seed), &a_actions);
        let b0 = build_local(dup(&seed), &b_actions);

        let (a_fwd, b_fwd) = (dup(&a0), dup(&b0));
        wire_gossip(&a_fwd, &b_fwd);

        let (a_rev, b_rev) = (dup(&a0), dup(&b0));
        wire_gossip(&b_rev, &a_rev);

        prop_assert_eq!(readout(&a_fwd.snapshot()), readout(&a_rev.snapshot()));
        prop_assert_eq!(readout(&b_fwd.snapshot()), readout(&b_rev.snapshot()));
    }

    /// A second gossip session immediately after the first is a no-op:
    /// neither peer's live content nor causal version changes, and
    /// observing either peer across the second session yields nothing new.
    #[test]
    fn gossip_idempotent(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let seed = rumors::Peer::<u64>::seed().sync_window_floor().into_rumors();
        let a = build_local(dup(&seed), &a_actions);
        let b = build_local(dup(&seed), &b_actions);
        wire_gossip(&a, &b);

        let a_before = fingerprint(&a);
        let b_before = fingerprint(&b);
        let checkpoint = a.snapshot().latest().clone();

        wire_gossip(&a, &b);

        prop_assert_eq!(fingerprint(&a), a_before);
        prop_assert_eq!(fingerprint(&b), b_before);
        prop_assert_eq!(
            a.snapshot().range(causally::since(&checkpoint)).count(), 0,
            "no new observations on second gossip",
        );
    }

    /// Pairwise gossip is order-independent across three peers.
    ///
    /// Two different session orders both converge every peer on the exact
    /// union of the three initial histories.
    #[test]
    fn gossip_order_independent(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
        c_actions in arb_local_actions(),
    ) {
        let seed = rumors::Peer::<u64>::seed().sync_window_floor().into_rumors();
        let a0 = build_local(dup(&seed), &a_actions);
        let b0 = build_local(dup(&seed), &b_actions);
        let c0 = build_local(dup(&seed), &c_actions);
        let mut expected = readout(&a0.snapshot());
        expected.extend(readout(&b0.snapshot()));
        expected.extend(readout(&c0.snapshot()));

        // Path one routes through A; check both recipients before completing
        // the remaining edge.
        let (a1, b1, c1) = (dup(&a0), dup(&b0), dup(&c0));
        wire_gossip(&a1, &b1);
        wire_gossip(&a1, &c1);
        prop_assert_eq!(
            readout(&a1.snapshot()), expected.clone(),
            "path one/A did not reach the three-peer union",
        );
        prop_assert_eq!(
            readout(&c1.snapshot()), expected.clone(),
            "path one/C did not receive B's history through A",
        );
        wire_gossip(&b1, &c1);

        // Path two routes through B. Check A before its direct session with C,
        // which could otherwise hide a failure to forward C's history through B.
        let (a2, b2, c2) = (dup(&a0), dup(&b0), dup(&c0));
        wire_gossip(&b2, &c2);
        wire_gossip(&a2, &b2);
        prop_assert_eq!(
            readout(&a2.snapshot()), expected.clone(),
            "path two/A did not receive C's history through B",
        );
        wire_gossip(&a2, &c2);

        for (name, peer) in [
            ("path one/A", &a1),
            ("path one/B", &b1),
            ("path one/C", &c1),
            ("path two/A", &a2),
            ("path two/B", &b2),
            ("path two/C", &c2),
        ] {
            prop_assert_eq!(
                readout(&peer.snapshot()), expected.clone(),
                "{} did not reach the three-peer union", name,
            );
        }
    }

    /// Gossip with a fresh fork of oneself is a no-op: the fork's version
    /// equals the original's, so the session converges immediately and
    /// changes neither side. The "true" idempotence of the merge.
    #[test]
    fn gossip_with_own_fork_is_noop(actions in arb_local_actions()) {
        let seed = rumors::Peer::<u64>::seed().sync_window_floor().into_rumors();
        let a = build_local(dup(&seed), &actions);
        let fork = dup(&a);

        let a_before = fingerprint(&a);
        let fork_before = fingerprint(&fork);
        wire_gossip(&a, &fork);

        prop_assert_eq!(fingerprint(&a), a_before);
        prop_assert_eq!(fingerprint(&fork), fork_before);
    }

    /// Gossip against an empty same-universe peer leaves the populated
    /// side untouched (nothing new to learn, no observation fires) while
    /// the empty side catches up to the populated side's content.
    #[test]
    fn gossip_with_empty_peer_is_one_sided(actions in arb_local_actions()) {
        let seed = rumors::Peer::<u64>::seed().sync_window_floor().into_rumors();
        let empty = dup(&seed);
        let a = build_local(dup(&seed), &actions);

        let a_before = fingerprint(&a);
        let checkpoint = a.snapshot().latest().clone();
        wire_gossip(&a, &empty);

        prop_assert_eq!(fingerprint(&a), a_before, "the populated side is unchanged");
        prop_assert_eq!(
            a.snapshot().range(causally::since(&checkpoint)).count(), 0,
            "the populated side observes nothing",
        );
        prop_assert_eq!(readout(&empty.snapshot()), readout(&a.snapshot()));
    }

    /// Two peers each insert a single value with no intervening
    /// gossip. The two `Version`s are causally concurrent, so
    /// `PartialOrd::partial_cmp` must return `None`.
    #[test]
    fn concurrent_inserts_have_incomparable_versions(
        a_value in any::<u64>(),
        b_value in any::<u64>(),
    ) {
        let seed = rumors::Peer::<u64>::seed().sync_window_floor().into_rumors();
        let alice = dup(&seed);
        let bob = dup(&seed);

        let alice_version = alice.send(a_value).unwrap();
        let bob_version = bob.send(b_value).unwrap();

        prop_assert_eq!(alice_version.partial_cmp(&bob_version), None);
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        // Exercise the u64 union oracle across two default case budgets.
        cases: ProptestConfig::default().cases * 2,
        ..ProptestConfig::default()
    })]

    /// One session unions `u64` content: each side reaches the union of the
    /// two pre-session readouts and the complete snapshots converge.
    ///
    /// The union oracle is sound because the peers tick disjoint parties and
    /// each redacts only its own pre-session sends. Shared-message redactions
    /// are exercised by the multi-peer schedule properties.
    #[test]
    fn gossip_unions_content(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        assert_gossip_unions(&a_actions, &b_actions)?;
    }
}

proptest! {
    /// The same union property for `String`, exercising payload encoding and
    /// decoding for a variable-size, non-primitive value.
    #[test]
    fn gossip_unions_string_content(
        a_actions in arb_string_actions(),
        b_actions in arb_string_actions(),
    ) {
        assert_gossip_unions(&a_actions, &b_actions)?;
    }

    /// Carrying one peer's state through a fresh fork reaches the same live
    /// multiset as gossiping the original peers directly.
    #[test]
    fn forked_carrier_matches_direct_gossip(
        a_values in prop::collection::vec(any::<u64>(), 0..=MAX_CARRIER_VALUES),
        b_values in prop::collection::vec(any::<u64>(), 0..=MAX_CARRIER_VALUES),
    ) {
        let seed = rumors::Peer::<u64>::seed().sync_window_floor().into_rumors();
        let a = dup(&seed);
        a.send_all(a_values).unwrap();
        let b = dup(&seed);
        b.send_all(b_values).unwrap();

        let carrier = dup(&a);
        let carried_b = dup(&b);
        wire_gossip(&carrier, &carried_b);
        wire_gossip(&a, &b);

        prop_assert_eq!(
            readout_multiset(&carrier.snapshot()),
            readout_multiset(&a.snapshot()),
        );
    }
}

/// Among 64 deterministic samples, `arb_local_actions` emits a `Redact`
/// that follows at least one `Insert` (the position where `build_local`
/// applies it), so the redaction dimension is live in the population.
#[test]
fn action_population_contains_effectual_redactions() {
    let mut runner = TestRunner::deterministic();
    let strategy = arb_local_actions();
    let mut effectual = 0usize;
    for _ in 0..64 {
        let actions = strategy
            .new_tree(&mut runner)
            .expect("action strategy always generates")
            .current();
        let mut inserted = false;
        for action in &actions {
            match action {
                LocalAction::Insert(_) => inserted = true,
                LocalAction::Redact(_) if inserted => effectual += 1,
                LocalAction::Redact(_) => {}
            }
        }
    }
    assert!(
        effectual > 0,
        "no sampled action sequence redacts after an insert: the redaction \
         dimension has silently left the population"
    );
}
