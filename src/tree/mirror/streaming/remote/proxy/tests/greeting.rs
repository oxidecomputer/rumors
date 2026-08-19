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
    Action, Tree,
    arb::{early_first_child_dispute_pair, nth_party},
    typed::hash::MERKLE_HASH_LEN,
};

use super::harness;

/// The observable root hash of a reconciled `tree::Root`.
fn hash_of(root: &crate::tree::Root<()>) -> [u8; MERKLE_HASH_LEN] {
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
fn union_hash(a: &crate::tree::Root<()>, b: &crate::tree::Root<()>) -> [u8; MERKLE_HASH_LEN] {
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
/// will elect initiator (the smaller live set; ties fall back to the greater
/// causal version in canonical bytes).
fn order_by_election(
    a: crate::tree::Root<()>,
    b: crate::tree::Root<()>,
) -> (crate::tree::Root<()>, crate::tree::Root<()>) {
    assert_ne!(
        a.ceiling.as_bytes(),
        b.ceiling.as_bytes(),
        "a divergent fixture must elect deterministically"
    );
    let len = |root: &crate::tree::Root<()>| {
        root.root
            .as_ref()
            .map(|node| node.len() as u64)
            .unwrap_or_default()
    };
    if crate::tree::mirror::streaming::message::initiates(len(&a), &a.ceiling, len(&b), &b.ceiling)
    {
        (a, b)
    } else {
        (b, a)
    }
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
    let mut populated = Tree::new();
    populated.act(&nth_party(0), [Action::Insert(Message::new(()))]);

    // The emptied initiator: insert-then-forget on party 1 ticks its version
    // while redaction keeps the tree (and so its advertised set) empty.
    let mut emptied = Tree::new();
    emptied.act(&nth_party(1), [Action::Insert(Message::new(()))]);
    let paths: Vec<_> = emptied
        .iter()
        .map(|(v, _)| crate::tree::typed::Path::for_leaf(v))
        .collect();
    emptied.act(&nth_party(1), paths.into_iter().map(Action::Forget));
    assert!(emptied.is_empty(), "the initiator's tree must be empty");

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

/// A genuinely pristine peer (empty tree, identity version) converges
/// against a populated one.
///
/// The pristine side wins the election (the smaller set initiates), its
/// empty carried listing asks for everything, and both sides converge on
/// the populated content shipped whole by the responder.
#[test]
fn mixed_empty_and_populated_converges() {
    let empty = Tree::<()>::new();
    let mut populated = Tree::new();
    populated.act(
        &nth_party(0),
        (0..4).map(|_| Action::Insert(Message::new(()))),
    );
    assert_ne!(
        populated.latest().as_bytes(),
        empty.latest().as_bytes(),
        "the fixture must diverge so an election happens at all"
    );

    let expected = populated.hash();
    let (left, right) = wire_reconcile(empty.root, populated.root);
    assert_eq!(hash_of(&left), expected);
    assert_eq!(hash_of(&right), expected);
}
/// WITNESS (the gossip install must merge-walk clone-derived fans):
///
/// Two honest, overlapping sessions at one peer must not silently delete a
/// message nobody redacted.
///
/// The shape: session S2 (with a peer converged at T0) forks at T0; equal
/// handshake versions resolve without opening the descent, so S2's
/// reconciled root is the fork-time root handle itself — its children fan
/// is the very object M0. Meanwhile session S1 (with a peer that redacted
/// one message at root radix `r_h`) installs first: the install's
/// `Tree::join` builds its merged fan as `ours.clone()` + `remove(r_h)` —
/// a clone-derived sibling M1 sharing every remaining child handle with
/// M0. S2's install then joins M1 against M0: a wide pair, identical
/// except at `r_h`, whose equal children form pointer-shared run after
/// pointer-shared run. A shortcut walk that elides entries after a shared
/// run — anything less than pairing the two fans radix by radix — desyncs
/// onto *neighboring, unchanged* radixes, which version dominance then
/// redacts as phantom deletions.
///
/// T0 is S2's causal past, so S2's install must be an identity on the
/// tree: the assertion is total (post-install hash equals pre-install
/// hash), swept over the redaction of each leaf in turn so every radix
/// position plays the divergence point in some round. `Tree::join`
/// merge-walks the two radix fans in lockstep, pruning equal pairs by
/// pointer-or-hash before descending; this witness holds the join to
/// that, and fails if a shortcut walk over shared runs ever returns.
#[test]
fn overlapping_sessions_lose_innocent_leaf_after_honored_redaction() {
    use crate::tree::Action;

    // T0: 25 unit messages. Hash-uniform keys give the root a wide fan
    // (~25 children), every radix holding one leaf, so honoring one leaf's
    // redaction empties its root radix — and sweeping the redacted leaf
    // puts shared runs on both sides of every divergence point.
    let p = nth_party(0);
    let mut t0 = Tree::new();
    t0.act(&p, (0..25).map(|_| Action::Insert(Message::new(()))));
    let leaves: Vec<_> = t0
        .iter()
        .map(|(v, _)| (crate::tree::typed::Path::for_leaf(v), v.clone()))
        .collect();

    // S2's wire session against a peer converged at T0, forked at T0:
    // equal versions resolve to the fork-time root.
    let (s2_reconciled, _) = wire_reconcile(t0.root.clone(), t0.root.clone());

    let mut lost = Vec::new();
    for (k, _) in &leaves {
        // S1's counterparty: converged at T0, then redacted the leaf at
        // `k` (a local act rebuilds its own fans afresh; the sharing that
        // matters is created by our install below, not here).
        let mut twin = Tree {
            root: t0.root.clone(),
        };
        twin.act(&nth_party(1), [Action::Forget(*k)]);

        // S1's session and install: reconcile T0 against the redacting
        // twin over the wire, then join the result into the live tree.
        // Deletion honoring drops radix `r_h`: the live tree's root fan is
        // now a clone-derived sibling of M0 missing one radix.
        let (s1_reconciled, _) = wire_reconcile(t0.root.clone(), twin.root.clone());
        let mut live = Tree {
            root: t0.root.clone(),
        };
        live.join(Tree {
            root: s1_reconciled,
        });
        let expected = live.hash();

        // S2's install, after S1's: joining our own causal past must be an
        // identity on the tree.
        live.join(Tree {
            root: s2_reconciled.clone(),
        });

        if live.hash() != expected {
            let missing: Vec<_> = leaves
                .iter()
                .filter(|(k2, v2)| k2 != k && live.get(v2).is_none())
                .map(|(k2, _)| *k2)
                .collect();
            lost.push((*k, missing));
        }
    }
    assert!(
        lost.is_empty(),
        "S2's install lost innocent leaves after S1 honored a redaction \
         (redacted key, innocent leaves missing): {lost:#?}",
    );
}
