//! Full-stack sessions exercising the greeting-carried opening listing.
//!
//! The V2 greeting carries each side's root-fan listing so the elected
//! responder answers the opening question without a dedicated wire hop.
//! These sessions pin the behaviors that design bought: both election
//! directions consume a carried listing correctly, an empty carried listing
//! means "send everything", a converged session carries its listings
//! entirely unused, and a mixed empty/populated pair converges one-sidedly.

use crate::message::Message;
use crate::testing::{IoPlan, run_to_quiescence};
use crate::tree::{
    Action, Tree,
    arb::{early_first_child_dispute_pair, nth_party},
};

use super::harness;

/// The observable root hash of a reconciled `tree::Root`.
fn hash_of(root: &crate::tree::Root<()>) -> [u8; 16] {
    Tree { root: root.clone() }.hash()
}

/// Reconcile through the two-proxy wire harness, requiring both sides to
/// succeed, and return `(left, right)` reconciled roots.
fn wire_reconcile(
    left: crate::tree::Root<()>,
    right: crate::tree::Root<()>,
) -> (crate::tree::Root<()>, crate::tree::Root<()>) {
    let outcome = run_to_quiescence(harness::reconcile(
        left,
        right,
        usize::MAX,
        IoPlan::default(),
        IoPlan::default(),
    ))
    .expect("the session must terminate");
    (
        outcome.left.expect("left endpoint reconciles"),
        outcome.right.expect("right endpoint reconciles"),
    )
}

/// The deep divergent pair's expected union, computed by the in-memory join
/// oracle.
fn union_hash(a: &crate::tree::Root<()>, b: &crate::tree::Root<()>) -> [u8; 16] {
    let mut union = Tree { root: a.clone() };
    union.join(Tree { root: b.clone() });
    union.hash()
}

/// A deep divergent session converges through the carried listing when the
/// physical-left endpoint wins the initiator election: the responder answers
/// the opening straight out of the greeting, and both sides reach the union.
#[test]
fn carried_listing_converges_with_left_initiator() {
    let (a, b) = early_first_child_dispute_pair();
    let expected = union_hash(&a, &b);
    let (initiator, responder) = order_by_election(a, b);

    let (left, right) = wire_reconcile(initiator, responder);
    assert_eq!(hash_of(&left), expected);
    assert_eq!(hash_of(&right), expected);
}

/// The same deep divergence converges when the physical-*right* endpoint
/// wins the election, so each harness side is exercised in both elected
/// roles across the two tests.
#[test]
fn carried_listing_converges_with_right_initiator() {
    let (a, b) = early_first_child_dispute_pair();
    let expected = union_hash(&a, &b);
    let (initiator, responder) = order_by_election(a, b);

    let (left, right) = wire_reconcile(responder, initiator);
    assert_eq!(hash_of(&left), expected);
    assert_eq!(hash_of(&right), expected);
}

/// Order a divergent pair so the first returned root is the one the session
/// will elect initiator (the greater causal version in canonical bytes).
fn order_by_election(
    a: crate::tree::Root<()>,
    b: crate::tree::Root<()>,
) -> (crate::tree::Root<()>, crate::tree::Root<()>) {
    assert_ne!(
        a.ceiling.as_bytes(),
        b.ceiling.as_bytes(),
        "a divergent fixture must elect deterministically"
    );
    if a.ceiling.as_bytes() > b.ceiling.as_bytes() {
        (a, b)
    } else {
        (b, a)
    }
}

/// An empty-tree initiator's carried listing is empty and asks for everything.
///
/// The tree is empty but the version is not (everything was redacted), so
/// this side still wins the election; its greeting carries an *empty*
/// listing, which the responder reads as the empty opening query — "I lack
/// the root, send everything" — and the session converges on the
/// responder's content.
#[test]
fn empty_carried_listing_asks_for_everything() {
    // The populated responder: one message on party 0.
    let mut populated = Tree::new();
    populated.act(&nth_party(0), [Action::Insert(Message::new(()))]);

    // The emptied initiator: insert-then-forget on party 1 until its ceiling
    // outranks the populated peer's in the canonical-bytes election. Each
    // round ticks the version while redaction keeps the tree empty, so the
    // bounded search only ever varies the ceiling.
    let mut emptied = Tree::new();
    for _ in 0..16 {
        emptied.act(&nth_party(1), [Action::Insert(Message::new(()))]);
        let keys: Vec<_> = emptied.iter().map(|(key, _, _)| key).collect();
        emptied.act(&nth_party(1), keys.into_iter().map(Action::Forget));
        if emptied.latest().as_bytes() > populated.latest().as_bytes() {
            break;
        }
    }
    assert!(emptied.is_empty(), "the initiator's tree must be empty");
    assert!(
        emptied.latest().as_bytes() > populated.latest().as_bytes(),
        "the emptied side must win the initiator election"
    );

    let expected = {
        let mut union = populated.clone();
        union.join(emptied.clone());
        union
    };
    let (left, right) = wire_reconcile(emptied.root, populated.root.clone());
    assert_eq!(hash_of(&left), expected.hash());
    assert_eq!(hash_of(&right), expected.hash());
    assert_eq!(
        hash_of(&left),
        populated.hash(),
        "the populated side's content survives; nothing was redacted away"
    );
}

/// A converged session (equal versions) ends at the greeting: both carried
/// listings go unused — the documented cost of carrying them
/// unconditionally — and no data stream is ever opened or accepted in
/// either direction.
#[test]
fn converged_session_carries_listings_unused() {
    let build = || {
        let mut tree = Tree::new();
        tree.act(&nth_party(0), [Action::Insert(Message::new(()))]);
        tree
    };
    let (a, b) = (build(), build());
    assert_eq!(a.latest(), b.latest(), "the fixture must be converged");
    let before = a.hash();

    let outcome = run_to_quiescence(harness::reconcile(
        a.root,
        b.root,
        usize::MAX,
        IoPlan::default(),
        IoPlan::default(),
    ))
    .expect("a converged session terminates at the greeting");
    let left = outcome.left.expect("left endpoint completes");
    let right = outcome.right.expect("right endpoint completes");
    assert_eq!(hash_of(&left), before);
    assert_eq!(hash_of(&right), before);

    for (side, io) in [("left", &outcome.left_io), ("right", &outcome.right_io)] {
        let report = io.snapshot();
        assert_eq!(report.connects, 0, "{side}: equal versions open no stream");
        assert_eq!(report.accepts, 0, "{side}: equal versions accept no stream");
    }
}

/// A genuinely pristine peer (empty tree, identity version) against a
/// populated one: the populated side wins the election, its carried listing
/// is answered by the empty responder, and both converge on the populated
/// content.
#[test]
fn mixed_empty_and_populated_converges() {
    let empty = Tree::<()>::new();
    let mut populated = Tree::new();
    populated.act(
        &nth_party(0),
        (0..4).map(|_| Action::Insert(Message::new(()))),
    );
    assert!(
        populated.latest().as_bytes() > empty.latest().as_bytes(),
        "the populated side must win the initiator election"
    );

    let expected = populated.hash();
    let (left, right) = wire_reconcile(empty.root, populated.root);
    assert_eq!(hash_of(&left), expected);
    assert_eq!(hash_of(&right), expected);
}
