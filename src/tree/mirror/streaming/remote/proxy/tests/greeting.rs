//! Full-stack sessions exercising the greeting-carried opening listing.
//!
//! The V2 greeting carries each side's root-fan listing so the elected
//! responder answers the opening question without a dedicated wire hop.
//! These sessions pin what that design bought: both election directions
//! consume a carried listing correctly, an empty carried listing means
//! "send everything", a converged session carries its listings entirely
//! unused, and a mixed empty/populated pair converges one-sidedly.

use crate::message::Message;
use crate::testing::{IoPlan, run_to_quiescence};
use crate::tree::{
    Action, Root as TreeRoot, Tree,
    arb::{early_first_child_dispute_pair, nth_party},
};

use super::harness;

/// Reconcile through the two-proxy wire harness, requiring both sides to
/// succeed, and return `(left, right)` reconciled roots.
fn wire_reconcile(left: TreeRoot, right: TreeRoot) -> (TreeRoot, TreeRoot) {
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

/// A deep divergent session converges through the carried listing when the
/// physical-left endpoint wins the initiator election: the responder answers
/// the opening straight out of the greeting, and both sides reach the union.
#[test]
fn carried_listing_converges_with_left_initiator() {
    let (a, b) = early_first_child_dispute_pair();
    let expected = harness::join_oracle(&a, &b);
    let (initiator, responder) = harness::order_by_election(a, b);

    let (left, right) = wire_reconcile(initiator, responder);
    assert_eq!(left, expected);
    assert_eq!(right, expected);
}

/// The same deep divergence converges when the physical-*right* endpoint
/// wins the election, so each harness side is exercised in both elected
/// roles across the two tests.
#[test]
fn carried_listing_converges_with_right_initiator() {
    let (a, b) = early_first_child_dispute_pair();
    let expected = harness::join_oracle(&a, &b);
    let (initiator, responder) = harness::order_by_election(a, b);

    let (left, right) = wire_reconcile(responder, initiator);
    assert_eq!(left, expected);
    assert_eq!(right, expected);
}

/// An empty-tree initiator's carried listing is empty and asks for everything.
///
/// The tree is empty but the version is not (everything was redacted). An
/// empty set is smaller than any populated one, so this side wins the
/// initiator election; its greeting carries an *empty* listing, which the
/// responder reads as the empty opening query — "I lack the root, send
/// everything" — and the session converges on the responder's content.
#[test]
fn empty_carried_listing_asks_for_everything() {
    // The populated responder: one message on party 0.
    let mut populated = Tree::<()>::default();
    populated.act(&nth_party(0), [Action::Insert(Message::new(()))]);

    // The emptied initiator: insert-then-forget on party 1 ticks its version
    // while redaction keeps the tree (and so its advertised set) empty.
    let mut emptied = Tree::default();
    emptied.act(&nth_party(1), [Action::Insert(Message::new(()))]);
    let versions: Vec<_> = emptied.iter().map(|(v, _)| v.clone()).collect();
    emptied.act(&nth_party(1), versions.into_iter().map(Action::Forget));
    assert!(emptied.is_empty(), "the initiator's tree must be empty");

    let expected = {
        let mut union = populated.clone();
        union.join(emptied.clone());
        union
    };
    let (left, right) = wire_reconcile(emptied.root, populated.root.clone());
    assert_eq!(left, expected.root);
    assert_eq!(right, expected.root);
    assert_eq!(
        Tree::<()>::from_root(crate::network::NetworkKeys::fixture(), left).hash(),
        populated.hash(),
        "the populated side's content survives; nothing was redacted away"
    );
}

/// A converged session (equal versions) ends at the greeting: both carried
/// listings go unused — the documented cost of carrying them
/// unconditionally — and no frame crosses in either direction.
///
/// Each side flushes exactly one item, its greeting, and reads exactly the
/// bytes its peer wrote.
#[test]
fn converged_session_carries_listings_unused() {
    let build = || {
        let mut tree = Tree::<()>::default();
        tree.act(&nth_party(0), [Action::Insert(Message::new(()))]);
        tree
    };
    let (a, b) = (build(), build());
    assert_eq!(a.latest(), b.latest(), "the fixture must be converged");
    let expected = a.root.clone();

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
    assert_eq!(left, expected);
    assert_eq!(right, expected);

    let (left_io, right_io) = (outcome.left_io.snapshot(), outcome.right_io.snapshot());
    for (side, io, peer) in [("left", left_io, right_io), ("right", right_io, left_io)] {
        assert_eq!(
            io.flushes, 1,
            "{side}: the greeting is the only item written"
        );
        assert_eq!(
            io.read_bytes, peer.write_bytes,
            "{side}: reads exactly the peer's greeting"
        );
    }
}

/// A pristine peer (empty tree, identity version) converges
/// against a populated one.
///
/// The pristine side wins the election (the smaller set initiates), its
/// empty carried listing asks for everything, and both sides converge on
/// the populated content shipped whole by the responder.
#[test]
fn mixed_empty_and_populated_converges() {
    let (empty, populated) = harness::disjoint_pair(0, 4);
    let expected = harness::join_oracle(&empty, &populated);
    let (left, right) = wire_reconcile(empty, populated);
    assert_eq!(left, expected);
    assert_eq!(right, expected);
}
