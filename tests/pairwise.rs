//! Pairwise gossip semantics for `Local::process` (and its alias `+`).
//!
//! Covers convergence, symmetry, idempotence, the algebraic laws of
//! `+`, and equivalence with `Local::gossip` over a real
//! `tokio::io::duplex` channel.
//!
//! Post-gossip equality is asserted against *readouts*, not `Local`s
//! directly: `Local::eq` includes the party tag, so two peers can
//! have identical content but distinct parties and never compare
//! equal. The publicly meaningful equality is "live content multiset
//! with consistent `Key`s," which is exactly what `readout` returns.

mod common;

use std::sync::{Arc, Mutex};

use borsh::{BorshDeserialize, BorshSerialize};
use proptest::prelude::*;
use rumors::sync::{Local, ignore};

// Closures handed to the sync API satisfy `Send + 'static`, so locally
// owned mutable state is routed through `Arc<Mutex<_>>` clones rather
// than captured by reference.

use crate::common::action::{arb_local_actions, arb_string_actions, build_local};
use crate::common::oracle::readout;
use crate::common::wire::wire_gossip;

/// Bidirectional gossip between two raw `Local`s, discarding
/// observation callbacks. Used by the algebraic tests below, which
/// care about final content but not about which keys fired callbacks.
fn gossip_step_local<T>(a: &mut Local<T>, b: &mut Local<T>)
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    let a_snapshot = a.clone();
    let b_snapshot = b.clone();
    a.process(b_snapshot, ignore);
    b.process(a_snapshot, ignore);
}

proptest! {
    /// After one bidirectional `gossip_step`, the two peers' live
    /// content (as exposed through `readout`) is equal.
    #[test]
    fn gossip_step_converges(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let mut a = build_local("alice", &a_actions);
        let mut b = build_local("bob", &b_actions);
        gossip_step_local(&mut a, &mut b);
        prop_assert_eq!(readout(&a), readout(&b));
    }

    /// The final pair `(a, b)` is independent of which side initiates
    /// the merge first — `a.process(b)` then `b.process(a)` yields
    /// the same content as `b.process(a)` then `a.process(b)`.
    #[test]
    fn gossip_step_symmetric(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let a0 = build_local("alice", &a_actions);
        let b0 = build_local("bob", &b_actions);

        let (mut a_fwd, mut b_fwd) = (a0.clone(), b0.clone());
        gossip_step_local(&mut a_fwd, &mut b_fwd);

        let (mut a_rev, mut b_rev) = (a0.clone(), b0);
        let a_snap = a_rev.clone();
        let b_snap = b_rev.clone();
        b_rev.process(a_snap, |_, _, _| {});
        a_rev.process(b_snap, |_, _, _| {});

        prop_assert_eq!(readout(&a_fwd), readout(&a_rev));
        prop_assert_eq!(readout(&b_fwd), readout(&b_rev));
    }

    /// A second `gossip_step` immediately after the first is a no-op:
    /// zero `on_message` callbacks fire and neither peer's content
    /// changes. (Same-party comparison is safe here — each peer is
    /// being compared to a clone of itself.)
    #[test]
    fn gossip_step_idempotent(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let mut a = build_local("alice", &a_actions);
        let mut b = build_local("bob", &b_actions);
        gossip_step_local(&mut a, &mut b);

        let a_before = a.clone();
        let b_before = b.clone();

        let observed = Arc::new(Mutex::new(0usize));
        let a_snap = a.clone();
        let b_snap = b.clone();
        {
            let observed_in = Arc::clone(&observed);
            a.process(b_snap, move |_, _, _| *observed_in.lock().unwrap() += 1);
        }
        {
            let observed_in = Arc::clone(&observed);
            b.process(a_snap, move |_, _, _| *observed_in.lock().unwrap() += 1);
        }

        prop_assert_eq!(*observed.lock().unwrap(), 0, "no new observations on second gossip");
        prop_assert_eq!(&a, &a_before);
        prop_assert_eq!(&b, &b_before);
    }

    /// Bidirectional `Local::process` produces the same final
    /// `(a, b)` as driving the same two `Local`s through
    /// `Local::gossip` over `tokio::io::duplex` — proving the wire
    /// protocol is faithful to the in-process merge.
    #[test]
    fn process_matches_wire_gossip(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let a0 = build_local("alice", &a_actions);
        let b0 = build_local("bob", &b_actions);

        let mut a_proc = a0.clone();
        let mut b_proc = b0.clone();
        gossip_step_local(&mut a_proc, &mut b_proc);

        let (a_wire, b_wire) = wire_gossip(a0, b0);

        prop_assert_eq!(readout(&a_proc), readout(&a_wire));
        prop_assert_eq!(readout(&b_proc), readout(&b_wire));
        prop_assert_eq!(readout(&a_wire), readout(&b_wire));
    }

    /// `+` is commutative on live content: `readout(a + b) ==
    /// readout(b + a)`. Compared via `readout` rather than `Local::eq`
    /// because the two operands carry distinct party tags.
    #[test]
    fn add_commutative(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let a = build_local("alice", &a_actions);
        let b = build_local("bob", &b_actions);
        prop_assert_eq!(readout(&(a.clone() + b.clone())), readout(&(b + a)));
    }

    /// `+` is associative on live content: `readout((a + b) + c) ==
    /// readout(a + (b + c))`.
    #[test]
    fn add_associative(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
        c_actions in arb_local_actions(),
    ) {
        let a = build_local("alice", &a_actions);
        let b = build_local("bob", &b_actions);
        let c = build_local("carol", &c_actions);
        prop_assert_eq!(
            readout(&((a.clone() + b.clone()) + c.clone())),
            readout(&(a + (b + c))),
        );
    }

    /// `+` is idempotent: `a + a == a`. Same party on both sides, so
    /// direct `Local::eq` is meaningful here.
    #[test]
    fn add_idempotent(a_actions in arb_local_actions()) {
        let a = build_local("alice", &a_actions);
        prop_assert_eq!(a.clone() + a.clone(), a);
    }

    /// `a += b` produces the same live content as `a + b`. Guards
    /// against `AddAssign` drifting away from `Add` (it's currently
    /// implemented as `self = self.clone() + rhs`).
    #[test]
    fn add_assign_matches_add(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let a = build_local("alice", &a_actions);
        let b = build_local("bob", &b_actions);

        let mut a_assign = a.clone();
        a_assign += b.clone();
        let a_plus = a + b;

        prop_assert_eq!(readout(&a_assign), readout(&a_plus));
    }

    /// `Local::process` against an empty source is a no-op: zero
    /// callbacks fire and `self` is unchanged.
    #[test]
    fn process_empty_source_is_noop(actions in arb_local_actions()) {
        let original = build_local("alice", &actions);
        let mut subject = original.clone();
        let empty = Local::<u64, _>::for_party("ghost", 0).unwrap().fork();

        let callbacks = Arc::new(Mutex::new(0usize));
        let callbacks_in = Arc::clone(&callbacks);
        subject.process(empty, move |_, _, _| *callbacks_in.lock().unwrap() += 1);

        prop_assert_eq!(*callbacks.lock().unwrap(), 0);
        prop_assert_eq!(&subject, &original);
    }

    /// String-T variant of [`process_matches_wire_gossip`]: same
    /// invariant for `T = String`, exercising the borsh round-trip
    /// for a non-primitive value type. Uses the same Insert/Redact
    /// action shape as the `u64` variant, so redaction-tombstone
    /// serialization is also covered.
    #[test]
    fn process_matches_wire_gossip_string(
        a_actions in arb_string_actions(),
        b_actions in arb_string_actions(),
    ) {
        let a0 = build_local("alice", &a_actions);
        let b0 = build_local("bob", &b_actions);

        let mut a_proc = a0.clone();
        let mut b_proc = b0.clone();
        gossip_step_local(&mut a_proc, &mut b_proc);

        let (a_wire, b_wire) = wire_gossip(a0, b0);

        prop_assert_eq!(readout(&a_proc), readout(&a_wire));
        prop_assert_eq!(readout(&b_proc), readout(&b_wire));
        prop_assert_eq!(readout(&a_wire), readout(&b_wire));
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

        let mut alice = Local::<u64, _>::for_party("alice", 0).unwrap();
        let mut bob = Local::<u64, _>::for_party("bob", 0).unwrap();

        let va: Arc<Mutex<Option<Version>>> = Arc::new(Mutex::new(None));
        let vb: Arc<Mutex<Option<Version>>> = Arc::new(Mutex::new(None));
        {
            let va_in = Arc::clone(&va);
            alice.message([a_value], move |_, v, _| *va_in.lock().unwrap() = Some(v.clone()));
        }
        {
            let vb_in = Arc::clone(&vb);
            bob.message([b_value], move |_, v, _| *vb_in.lock().unwrap() = Some(v.clone()));
        }

        let va = va.lock().unwrap().clone().expect("alice's insert must fire on_message");
        let vb = vb.lock().unwrap().clone().expect("bob's insert must fire on_message");
        prop_assert_eq!(va.partial_cmp(&vb), None);
    }

    /// Asymmetric merge: after `a.process(b)`, `a`'s live readout
    /// equals the union of the two pre-merge readouts. `b` itself
    /// is unchanged. This pins down the underlying primitive of `+`
    /// and `gossip_step` directly, independent of the bidirectional
    /// wrapper.
    ///
    /// The "union of readouts" is computed by `BTreeMap::extend`,
    /// which is sound here only because `Key`s derive from
    /// `(party, version_counter)` and `alice` / `bob` therefore
    /// can't mint the same `Key`. Across same-party operands the
    /// math would be more subtle.
    #[test]
    fn asymmetric_process_unions_content(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let a0 = build_local("alice", &a_actions);
        let b0 = build_local("bob", &b_actions);

        let a_before = readout(&a0);
        let b_before = readout(&b0);
        let mut expected = a_before;
        expected.extend(b_before.clone());

        let mut a_after = a0;
        let b_snapshot = b0.clone();
        a_after.process(b_snapshot, |_, _, _| {});

        prop_assert_eq!(readout(&a_after), expected);
        prop_assert_eq!(readout(&b0), b_before);
    }

    /// `Local::process` against a clone of `self` is a no-op:
    /// zero callbacks fire and the readout is unchanged. The
    /// "true" idempotence of `process`, distinct from idempotence
    /// of `+` (which always wraps `process`).
    #[test]
    fn self_process_is_noop(actions in arb_local_actions()) {
        let original = build_local("alice", &actions);
        let readout_before = readout(&original);

        let mut subject = original.clone();
        let callbacks = Arc::new(Mutex::new(0usize));
        let callbacks_in = Arc::clone(&callbacks);
        subject.process(original, move |_, _, _| *callbacks_in.lock().unwrap() += 1);

        prop_assert_eq!(*callbacks.lock().unwrap(), 0);
        prop_assert_eq!(readout(&subject), readout_before);
    }
}
