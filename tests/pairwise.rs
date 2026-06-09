//! Pairwise gossip semantics for `Known::join`.
//!
//! Covers convergence, symmetry, idempotence, and the algebraic laws of the
//! `join` merge. Equivalence between this in-process merge and `Known::gossip`
//! over the wire lives in the `async_wire` and `sync_wire` test binaries.
//!
//! Two `Known`s are compared with `Known::eq`, which compares *live content*
//! (the underlying tree) and ignores the party tag — so two peers that reached
//! the same content by different merge paths compare equal. Where a test wants
//! the `Key`-level multiset explicitly it goes through `readout`.
//!
//! Every peer in a test is a genuine, party-disjoint fork of one shared
//! [`Known::seed`], minted by [`sync_bootstrap_fork`]. They share a [`Network`]
//! but tick disjoint parties, so their concurrent inserts stay incomparable and
//! `join` (a content merge, network-guarded) between them never fails. A
//! [`rumors`](Known::rumors) snapshot shares its origin's party, so it is only
//! ever used as a *content* source for a `join`, never to originate.

mod common;

use borsh::{BorshDeserialize, BorshSerialize};
use proptest::prelude::*;
use rumors::sync::Known;

use crate::common::action::{arb_local_actions, build_local};
use crate::common::oracle::readout;
use crate::common::sync_wire::sync_bootstrap_fork;

/// A genuine, party-disjoint Facts copy of `k`'s content: the replacement for
/// the old `k.fork()` wherever a *fresh originator* is needed (a `join`
/// receiver, or a peer that will be mutated). A [`rumors`](Known::rumors)
/// snapshot won't do — it shares `k`'s party, so two such copies would carry
/// non-concurrent versions and corrupt the merge's deletion-honoring.
fn dup<T>(k: &Known<T>) -> Known<T>
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    sync_bootstrap_fork(k)
}

/// Bidirectional gossip between two raw `Known`s, discarding observation
/// callbacks. Each side merges the other's content snapshot in; the two share a
/// network, so the `join`s never fail.
fn gossip_step_local<T>(a: &mut Known<T>, b: &mut Known<T>)
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    let a_snapshot = a.rumors();
    let b_snapshot = b.rumors();
    a.join(b_snapshot)
        .unwrap_or_else(|_| unreachable!("same-universe operands"));
    b.join(a_snapshot)
        .unwrap_or_else(|_| unreachable!("same-universe operands"));
}

/// Merge `b`'s content into `a` and return the result: the `join`-based stand-in
/// for the old `a + b`. `a` is a fresh Facts receiver; `b` is a content
/// snapshot. Both descend from one seed, so the `join` never fails.
fn merged<T>(mut a: Known<T>, b: Known<T, rumors::Rumors>) -> Known<T>
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    a.join(b)
        .unwrap_or_else(|_| unreachable!("same-universe operands"));
    a
}

proptest! {
    /// After one bidirectional `gossip_step`, the two peers' live
    /// content (as exposed through `readout`) is equal.
    #[test]
    fn gossip_step_converges(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let seed = Known::<u64>::seed();
        let mut a = build_local(dup(&seed), &a_actions);
        let mut b = build_local(dup(&seed), &b_actions);
        gossip_step_local(&mut a, &mut b);
        prop_assert_eq!(readout(&a), readout(&b));
    }

    /// The final pair `(a, b)` is independent of which side initiates
    /// the merge first — `a.join(b)` then `b.join(a)` yields
    /// the same content as `b.join(a)` then `a.join(b)`.
    #[test]
    fn gossip_step_symmetric(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let seed = Known::<u64>::seed();
        let a0 = build_local(dup(&seed), &a_actions);
        let b0 = build_local(dup(&seed), &b_actions);

        let (mut a_fwd, mut b_fwd) = (dup(&a0), dup(&b0));
        gossip_step_local(&mut a_fwd, &mut b_fwd);

        let (mut a_rev, mut b_rev) = (dup(&a0), dup(&b0));
        let a_snap = a_rev.rumors();
        let b_snap = b_rev.rumors();
        b_rev.join_then(a_snap, |_, _, _| {}).unwrap();
        a_rev.join_then(b_snap, |_, _, _| {}).unwrap();

        prop_assert_eq!(readout(&a_fwd), readout(&a_rev));
        prop_assert_eq!(readout(&b_fwd), readout(&b_rev));
    }

    /// A second `gossip_step` immediately after the first is a no-op:
    /// zero `on_message` callbacks fire and neither peer's content
    /// changes.
    #[test]
    fn gossip_step_idempotent(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let seed = Known::<u64>::seed();
        let mut a = build_local(dup(&seed), &a_actions);
        let mut b = build_local(dup(&seed), &b_actions);
        gossip_step_local(&mut a, &mut b);

        let a_before = a.rumors();
        let b_before = b.rumors();

        let mut observed = 0usize;
        let a_snap = a.rumors();
        let b_snap = b.rumors();
        a.join_then(b_snap, |_, _, _| observed += 1).unwrap();
        b.join_then(a_snap, |_, _, _| observed += 1).unwrap();

        prop_assert_eq!(observed, 0, "no new observations on second gossip");
        prop_assert_eq!(&a, &a_before);
        prop_assert_eq!(&b, &b_before);
    }

    /// `join` is commutative on live content: `readout(a join b) ==
    /// readout(b join a)`.
    #[test]
    fn join_commutative(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let seed = Known::<u64>::seed();
        let a = build_local(dup(&seed), &a_actions);
        let b = build_local(dup(&seed), &b_actions);
        prop_assert_eq!(
            readout(&merged(dup(&a), b.rumors())),
            readout(&merged(dup(&b), a.rumors())),
        );
    }

    /// `join` is associative on live content: `readout((a join b) join c) ==
    /// readout(a join (b join c))`.
    #[test]
    fn join_associative(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
        c_actions in arb_local_actions(),
    ) {
        let seed = Known::<u64>::seed();
        let a = build_local(dup(&seed), &a_actions);
        let b = build_local(dup(&seed), &b_actions);
        let c = build_local(dup(&seed), &c_actions);
        prop_assert_eq!(
            readout(&merged(merged(dup(&a), b.rumors()), c.rumors())),
            readout(&merged(dup(&a), merged(dup(&b), c.rumors()).rumors())),
        );
    }

    /// `join` is idempotent: merging two copies of the same content yields
    /// that content. `Known::eq` compares the tree (party-independent), so a
    /// direct equality is meaningful.
    #[test]
    fn join_idempotent(a_actions in arb_local_actions()) {
        let seed = Known::<u64>::seed();
        let a = build_local(dup(&seed), &a_actions);
        prop_assert_eq!(merged(dup(&a), a.rumors()), a);
    }

    /// `Known::join` against an empty source is a no-op: zero
    /// callbacks fire and `self` is unchanged.
    #[test]
    fn process_empty_source_is_noop(actions in arb_local_actions()) {
        let seed = Known::<u64>::seed();
        let original = build_local(dup(&seed), &actions);
        let mut subject = dup(&original);
        let empty = seed.rumors();

        let mut callbacks = 0usize;
        subject.join_then(empty, |_, _, _| callbacks += 1).unwrap();

        prop_assert_eq!(callbacks, 0);
        prop_assert_eq!(&subject, &original);
    }

    /// Two peers each insert a single value with no intervening
    /// gossip. The two `Version`s are causally concurrent, so
    /// `PartialOrd::partial_cmp` must return `None`.
    #[test]
    fn concurrent_inserts_have_incomparable_versions(
        a_value in any::<u64>(),
        b_value in any::<u64>(),
    ) {
        use rumors::Version;

        let seed = Known::<u64>::seed();
        let mut alice = dup(&seed);
        let mut bob = dup(&seed);

        let mut va: Option<Version> = None;
        let mut vb: Option<Version> = None;
        alice.message_then([a_value], |_, v, _| va = Some(v.clone()));
        bob.message_then([b_value], |_, v, _| vb = Some(v.clone()));

        let va = va.expect("alice's insert must fire on_message");
        let vb = vb.expect("bob's insert must fire on_message");
        prop_assert_eq!(va.partial_cmp(&vb), None);
    }

    /// Asymmetric merge: after `a.join(b)`, `a`'s live readout
    /// equals the union of the two pre-merge readouts. `b` itself
    /// is unchanged. This pins down the underlying merge primitive
    /// directly, independent of the bidirectional wrapper.
    ///
    /// The "union of readouts" is computed by `BTreeMap::extend`,
    /// which is sound here only because `Key`s derive from the leaf
    /// version's canonical bytes and `alice` / `bob` tick disjoint
    /// parties, so they can't mint the same `Key`.
    #[test]
    fn asymmetric_process_unions_content(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let seed = Known::<u64>::seed();
        let a0 = build_local(dup(&seed), &a_actions);
        let b0 = build_local(dup(&seed), &b_actions);

        let a_before = readout(&a0);
        let b_before = readout(&b0);
        let mut expected = a_before;
        expected.extend(b_before.clone());

        let b_snapshot = b0.rumors();
        let mut a_after = a0;
        a_after.join_then(b_snapshot, |_, _, _| {}).unwrap();

        prop_assert_eq!(readout(&a_after), expected);
        prop_assert_eq!(readout(&b0), b_before);
    }

    /// `Known::join` against a snapshot of `self` is a no-op: zero
    /// callbacks fire and the readout is unchanged. The "true"
    /// idempotence of the merge.
    #[test]
    fn self_process_is_noop(actions in arb_local_actions()) {
        let seed = Known::<u64>::seed();
        let original = build_local(dup(&seed), &actions);
        let readout_before = readout(&original);

        let mut subject = dup(&original);
        let mut callbacks = 0usize;
        subject.join_then(original.rumors(), |_, _, _| callbacks += 1).unwrap();

        prop_assert_eq!(callbacks, 0);
        prop_assert_eq!(readout(&subject), readout_before);
    }
}
