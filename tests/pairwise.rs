//! Pairwise gossip semantics for `Known::gossip`, the merge primitive.
//!
//! With the shared-state `Known`, wire gossip *is* the merge: there is no
//! in-process `join`. These properties pin the algebraic laws of one
//! bidirectional session — convergence, side-symmetry, idempotence,
//! order-independence across three peers, and the union of live content —
//! plus the causal-concurrency basics the merge rests on.
//!
//! Live content is compared through `readout` (the `(Key, value)` lens the
//! oracle checks also use) or through `hash`/`latest` where the assertion
//! is "nothing changed at all".
//!
//! Every peer in a test is a genuine, party-disjoint fork of one shared
//! [`Known::seed`], minted by [`bootstrap_fork`]. They share a
//! [`Network`](rumors::Network) but tick disjoint parties, so their
//! concurrent inserts stay incomparable and gossip between them never
//! fails.

mod common;

use borsh::{BorshDeserialize, BorshSerialize};
use proptest::prelude::*;
use rumors::{Known, Version, causally};

use crate::common::action::{arb_local_actions, build_local_async};
use crate::common::oracle::readout;
use crate::common::wire::{bootstrap_fork, wire_gossip};

/// A genuine, party-disjoint copy of `k`'s content: a fresh originator that
/// holds the same live messages but ticks its own party region.
fn dup<T>(k: &mut Known<T>) -> Known<T>
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    bootstrap_fork(k)
}

/// The `(hash, latest)` fingerprint of a peer: equal fingerprints mean the
/// same live content *and* the same causal frontier — gossip between two
/// peers with equal fingerprints is a guaranteed no-op.
fn fingerprint<T>(k: &Known<T>) -> ([u8; 32], Version) {
    (k.hash(), k.latest())
}

proptest! {
    /// After one bidirectional gossip session, the two peers' live
    /// content (as exposed through `readout`) is equal.
    #[test]
    fn gossip_converges(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let mut seed = Known::<u64>::seed();
        let mut a = build_local_async(dup(&mut seed), &a_actions);
        let mut b = build_local_async(dup(&mut seed), &b_actions);
        wire_gossip(&mut a, &mut b);
        prop_assert_eq!(readout(&a.snapshot()), readout(&b.snapshot()));
        prop_assert_eq!(a.hash(), b.hash());
        prop_assert_eq!(a.latest(), b.latest());
    }

    /// The converged pair is independent of which peer sits on which side
    /// of the duplex: gossiping `(a, b)` and gossiping `(b, a)` from
    /// identically-built starting points yield the same content on both
    /// sides.
    #[test]
    fn gossip_side_symmetric(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let mut seed = Known::<u64>::seed();
        let mut a0 = build_local_async(dup(&mut seed), &a_actions);
        let mut b0 = build_local_async(dup(&mut seed), &b_actions);

        let (mut a_fwd, mut b_fwd) = (dup(&mut a0), dup(&mut b0));
        wire_gossip(&mut a_fwd, &mut b_fwd);

        let (mut a_rev, mut b_rev) = (dup(&mut a0), dup(&mut b0));
        wire_gossip(&mut b_rev, &mut a_rev);

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
        let mut seed = Known::<u64>::seed();
        let mut a = build_local_async(dup(&mut seed), &a_actions);
        let mut b = build_local_async(dup(&mut seed), &b_actions);
        wire_gossip(&mut a, &mut b);

        let a_before = fingerprint(&a);
        let b_before = fingerprint(&b);
        let cursor = a.latest();

        wire_gossip(&mut a, &mut b);

        prop_assert_eq!(fingerprint(&a), a_before);
        prop_assert_eq!(fingerprint(&b), b_before);
        prop_assert_eq!(
            a.snapshot().range(causally::since(&cursor)).count(), 0,
            "no new observations on second gossip",
        );
    }

    /// Pairwise gossip is order-independent across three peers: routing
    /// everything through `a` first (`a·b` then `a·c`) and routing through
    /// `b` first (`b·c` then `a·b`) both leave `a` holding the same
    /// content — the union of all three.
    #[test]
    fn gossip_order_independent(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
        c_actions in arb_local_actions(),
    ) {
        let mut seed = Known::<u64>::seed();
        let mut a0 = build_local_async(dup(&mut seed), &a_actions);
        let mut b0 = build_local_async(dup(&mut seed), &b_actions);
        let mut c0 = build_local_async(dup(&mut seed), &c_actions);

        // Path one: (a·b), then (a·c).
        let (mut a1, mut b1, mut c1) = (dup(&mut a0), dup(&mut b0), dup(&mut c0));
        wire_gossip(&mut a1, &mut b1);
        wire_gossip(&mut a1, &mut c1);

        // Path two: (b·c), then (a·b).
        let (mut a2, mut b2, mut c2) = (dup(&mut a0), dup(&mut b0), dup(&mut c0));
        wire_gossip(&mut b2, &mut c2);
        wire_gossip(&mut a2, &mut b2);

        prop_assert_eq!(readout(&a1.snapshot()), readout(&a2.snapshot()));
    }

    /// Gossip with a fresh fork of oneself is a no-op: the fork's version
    /// equals the original's, so the session converges immediately and
    /// changes neither side. The "true" idempotence of the merge.
    #[test]
    fn gossip_with_own_fork_is_noop(actions in arb_local_actions()) {
        let mut seed = Known::<u64>::seed();
        let mut a = build_local_async(dup(&mut seed), &actions);
        let mut fork = dup(&mut a);

        let a_before = fingerprint(&a);
        let fork_before = fingerprint(&fork);
        wire_gossip(&mut a, &mut fork);

        prop_assert_eq!(fingerprint(&a), a_before);
        prop_assert_eq!(fingerprint(&fork), fork_before);
    }

    /// Gossip against an empty same-universe peer leaves the populated
    /// side untouched (nothing new to learn, no observation fires) while
    /// the empty side catches up to the populated side's content.
    #[test]
    fn gossip_with_empty_peer_is_one_sided(actions in arb_local_actions()) {
        let mut seed = Known::<u64>::seed();
        let mut empty = dup(&mut seed);
        let mut a = build_local_async(dup(&mut seed), &actions);

        let a_before = fingerprint(&a);
        let cursor = a.latest();
        wire_gossip(&mut a, &mut empty);

        prop_assert_eq!(fingerprint(&a), a_before, "the populated side is unchanged");
        prop_assert_eq!(
            a.snapshot().range(causally::since(&cursor)).count(), 0,
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
        let mut seed = Known::<u64>::seed();
        let alice = dup(&mut seed);
        let bob = dup(&mut seed);

        let pre_a = alice.latest();
        alice.send(a_value);
        let snap_a = alice.snapshot();
        let (_, va, _) = snap_a
            .range(causally::since(&pre_a))
            .next()
            .expect("alice's insert mints a live leaf");

        let pre_b = bob.latest();
        bob.send(b_value);
        let snap_b = bob.snapshot();
        let (_, vb, _) = snap_b
            .range(causally::since(&pre_b))
            .next()
            .expect("bob's insert mints a live leaf");

        prop_assert_eq!(va.partial_cmp(vb), None);
    }

    /// One session unions live content: after gossip, each side's readout
    /// equals the union of the two pre-session readouts.
    ///
    /// The "union of readouts" is computed by `BTreeMap::extend`,
    /// which is sound here only because `Key`s derive from the leaf
    /// version's canonical bytes and `alice` / `bob` tick disjoint
    /// parties, so they can't mint the same `Key`.
    #[test]
    fn gossip_unions_content(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let mut seed = Known::<u64>::seed();
        let mut a = build_local_async(dup(&mut seed), &a_actions);
        let mut b = build_local_async(dup(&mut seed), &b_actions);

        let a_before = readout(&a.snapshot());
        let b_before = readout(&b.snapshot());
        let mut expected = a_before;
        expected.extend(b_before);

        wire_gossip(&mut a, &mut b);

        prop_assert_eq!(readout(&a.snapshot()), expected.clone());
        prop_assert_eq!(readout(&b.snapshot()), expected);
    }
}
