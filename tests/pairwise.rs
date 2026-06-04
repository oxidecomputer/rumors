//! Pairwise gossip semantics for `Known::learn` and `Known::join`.
//!
//! Covers convergence, symmetry, idempotence, and the algebraic laws of the
//! `join` merge. Equivalence between this in-process merge and `Known::gossip`
//! over the wire lives in the `async_wire` and `sync_wire` test binaries.
//!
//! Two `Known`s are compared with `Known::eq`, which compares *live content*
//! (the underlying tree) and ignores the party tag — so two peers that reached
//! the same content by different fork/merge paths compare equal. Where a test
//! wants the `Key`-level multiset explicitly it goes through `readout`.
//!
//! Every peer in a test is forked from one shared [`Known::seed`], so all
//! peers are pairwise *disjoint* and `learn`/`join` between them never fails
//! (the `before` crate's Law of Disjointness).

mod common;

use borsh::{BorshDeserialize, BorshSerialize};
use proptest::prelude::*;
use rumors::sync::Known;

use crate::common::action::{arb_local_actions, build_local};
use crate::common::oracle::readout;

/// Bidirectional gossip between two raw `Known`s, discarding observation
/// callbacks. The two must be disjoint (forked from a shared seed), which
/// every caller guarantees, so the `learn`s never fail.
fn gossip_step_local<T>(a: &mut Known<T>, b: &mut Known<T>)
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    let a_snapshot = a.fork();
    let b_snapshot = b.fork();
    a.join(b_snapshot)
        .unwrap_or_else(|_| unreachable!("disjoint operands"));
    b.join(a_snapshot)
        .unwrap_or_else(|_| unreachable!("disjoint operands"));
}

/// Merge `b` into `a` and return the result: the `join`-based stand-in for the
/// old `a + b`. Operands must be disjoint (forked from a shared seed).
fn merged<T>(mut a: Known<T>, b: Known<T>) -> Known<T>
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    a.join(b)
        .unwrap_or_else(|_| unreachable!("disjoint operands"));
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
        let mut seed = Known::<u64>::seed();
        let mut a = build_local(seed.fork(), &a_actions);
        let mut b = build_local(seed.fork(), &b_actions);
        gossip_step_local(&mut a, &mut b);
        prop_assert_eq!(readout(&a), readout(&b));
    }

    /// The final pair `(a, b)` is independent of which side initiates
    /// the merge first — `a.learn(b)` then `b.learn(a)` yields
    /// the same content as `b.learn(a)` then `a.learn(b)`.
    #[test]
    fn gossip_step_symmetric(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let mut seed = Known::<u64>::seed();
        let mut a0 = build_local(seed.fork(), &a_actions);
        let mut b0 = build_local(seed.fork(), &b_actions);

        let (mut a_fwd, mut b_fwd) = (a0.fork(), b0.fork());
        gossip_step_local(&mut a_fwd, &mut b_fwd);

        let (mut a_rev, mut b_rev) = (a0.fork(), b0.fork());
        let a_snap = a_rev.fork();
        let b_snap = b_rev.fork();
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
        let mut seed = Known::<u64>::seed();
        let mut a = build_local(seed.fork(), &a_actions);
        let mut b = build_local(seed.fork(), &b_actions);
        gossip_step_local(&mut a, &mut b);

        let a_before = a.fork();
        let b_before = b.fork();

        let mut observed = 0usize;
        let a_snap = a.fork();
        let b_snap = b.fork();
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
        let mut seed = Known::<u64>::seed();
        let mut a = build_local(seed.fork(), &a_actions);
        let mut b = build_local(seed.fork(), &b_actions);
        prop_assert_eq!(
            readout(&merged(a.fork(), b.fork())),
            readout(&merged(b.fork(), a.fork())),
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
        let mut seed = Known::<u64>::seed();
        let mut a = build_local(seed.fork(), &a_actions);
        let mut b = build_local(seed.fork(), &b_actions);
        let mut c = build_local(seed.fork(), &c_actions);
        prop_assert_eq!(
            readout(&merged(merged(a.fork(), b.fork()), c.fork())),
            readout(&merged(a.fork(), merged(b.fork(), c.fork()))),
        );
    }

    /// `join` is idempotent: merging two copies of the same content yields
    /// that content. `Known::eq` compares the tree (party-independent), so a
    /// direct equality is meaningful.
    #[test]
    fn join_idempotent(a_actions in arb_local_actions()) {
        let mut seed = Known::<u64>::seed();
        let mut a = build_local(seed.fork(), &a_actions);
        prop_assert_eq!(merged(a.fork(), a.fork()), a);
    }

    /// `Known::learn` against an empty source is a no-op: zero
    /// callbacks fire and `self` is unchanged.
    #[test]
    fn process_empty_source_is_noop(actions in arb_local_actions()) {
        let mut seed = Known::<u64>::seed();
        let mut original = build_local(seed.fork(), &actions);
        let mut subject = original.fork();
        let empty = seed.fork();

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

        let mut seed = Known::<u64>::seed();
        let mut alice = seed.fork();
        let mut bob = seed.fork();

        let mut va: Option<Version> = None;
        let mut vb: Option<Version> = None;
        alice.message_then([a_value], |_, v, _| va = Some(v.clone()));
        bob.message_then([b_value], |_, v, _| vb = Some(v.clone()));

        let va = va.expect("alice's insert must fire on_message");
        let vb = vb.expect("bob's insert must fire on_message");
        prop_assert_eq!(va.partial_cmp(&vb), None);
    }

    /// Asymmetric merge: after `a.learn(b)`, `a`'s live readout
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
        let mut seed = Known::<u64>::seed();
        let a0 = build_local(seed.fork(), &a_actions);
        let mut b0 = build_local(seed.fork(), &b_actions);

        let a_before = readout(&a0);
        let b_before = readout(&b0);
        let mut expected = a_before;
        expected.extend(b_before.clone());

        let b_snapshot = b0.fork();
        let mut a_after = a0;
        a_after.join_then(b_snapshot, |_, _, _| {}).unwrap();

        prop_assert_eq!(readout(&a_after), expected);
        prop_assert_eq!(readout(&b0), b_before);
    }

    /// `Known::learn` against a fork of `self` is a no-op: zero
    /// callbacks fire and the readout is unchanged. The "true"
    /// idempotence of the merge.
    #[test]
    fn self_process_is_noop(actions in arb_local_actions()) {
        let mut seed = Known::<u64>::seed();
        let mut original = build_local(seed.fork(), &actions);
        let readout_before = readout(&original);

        let mut subject = original.fork();
        let mut callbacks = 0usize;
        subject.join_then(original, |_, _, _| callbacks += 1).unwrap();

        prop_assert_eq!(callbacks, 0);
        prop_assert_eq!(readout(&subject), readout_before);
    }
}
