use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use proptest::prelude::*;

use crate::Version;
use crate::message::Message;
use crate::tree::Root;
use crate::tree::arb::{
    arb_divergent_pair, arb_tree_root, leaf_parent_dispute_pair, leaf_parent_redaction_pair,
    nth_party,
};
use crate::tree::mirror::alternating;
use crate::tree::mirror::streaming::{
    Handshaking, Local, Root as StreamingRoot, mirror as drive_streaming,
};
use crate::tree::traverse::{Action, act};
use crate::tree::typed::{Node, Path, height};

/// Reconcile `a` and `b` through the streaming local backend, returning both
/// sides' reconciled roots in argument order, with no convergence assertion.
fn streaming_mirror_sides(a: Root<()>, b: Root<()>) -> (Root<()>, Root<()>) {
    let (a, b): (StreamingRoot<Local, ()>, StreamingRoot<Local, ()>) = (a.into(), b.into());
    let client = Handshaking::start(Local, a.clone());
    let server = Handshaking::start(Local, b.clone());
    let (ours, theirs) = pollster::block_on(drive_streaming(client, server))
        // `Local` is infallible, so the session's only inhabited errors are
        // violations — which two honest local endpoints must never speak.
        .expect("local mirror speaks no violations")
        // Equal handshake versions: already converged, both sides unchanged.
        .unwrap_or((a, b));
    (ours.into(), theirs.into())
}

/// Reconcile `a` and `b` through the streaming local backend, asserting the
/// two sides converge to the same root, and return it.
fn streaming_mirror(a: Root<()>, b: Root<()>) -> Root<()> {
    let (ours, theirs) = streaming_mirror_sides(a, b);
    assert_eq!(ours, theirs, "streaming endpoints should converge");
    ours
}

/// Reconcile `a` and `b` through the alternating implementation — the
/// behavioral oracle the streaming protocol must reproduce exactly —
/// returning both sides' roots in argument order, with no convergence
/// assertion.
fn alternating_mirror_sides(a: Root<()>, b: Root<()>) -> (Root<()>, Root<()>) {
    pollster::block_on(async {
        let local_a = alternating::local::Exchange::start(a);
        let local_b = alternating::local::Exchange::start(b);
        match alternating::mirror(local_a, local_b).await {
            Err(e) => match e {},
            Ok(pair) => pair,
        }
    })
}

/// Reconcile `a` and `b` through the alternating oracle, asserting the two
/// sides converge to the same root, and return it.
fn alternating_mirror(a: Root<()>, b: Root<()>) -> Root<()> {
    let (ours, theirs) = alternating_mirror_sides(a, b);
    assert_eq!(ours, theirs, "oracle endpoints should converge");
    ours
}

/// Run `f` under a watchdog: a deadlocked session fails the test in bounded
/// time instead of hanging the suite.
fn with_watchdog<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => {
            panic!("streaming mirror deadlocked: no result within the watchdog timeout")
        }
        Err(RecvTimeoutError::Disconnected) => panic!("streaming mirror worker panicked"),
    }
}

/// Run [`streaming_mirror`] under the watchdog.
fn streaming_mirror_with_timeout(a: Root<()>, b: Root<()>) -> Root<()> {
    with_watchdog(move || streaming_mirror(a, b))
}

/// Build a divergent pair whose every difference is one-sided, shaped by
/// `spec`.
///
/// For each `(radix, shared, extra)` root child, both trees hold `shared`
/// identical leaves under it and `b` additionally holds `extra` concurrent
/// ones.
///
/// Leaves are placed at hand-picked paths (first byte the root radix, second
/// byte a counter), not content-addressed ones: the reconciliation machinery
/// keys purely by prefix, and controlling the first two bytes is what lets a
/// test pin the exact fan-out each walk routes. Because no key is present on
/// both sides with different content, every root child disputes but nothing
/// disputes below it: the session's descent is empty, and the whole diff
/// resolves in the first descending stage.
fn one_sided_pair(spec: &[(u8, u8, u8)]) -> (Root<()>, Root<()>) {
    let path = |b0: u8, b1: u8| {
        let mut bytes = [0u8; 32];
        bytes[0] = b0;
        bytes[1] = b1;
        Path::from(bytes)
    };

    // The shared base: one version chain on party 0, identical in both trees
    // (b is built on top of a's node, so the shared subtrees are literally
    // the same nodes and their hashes match by construction).
    let shared_party = nth_party(0);
    let mut version = Version::new();
    let mut shared = Vec::new();
    for &(radix, n_shared, _) in spec {
        for i in 0..n_shared {
            version.tick(&shared_party);
            shared.push((
                path(radix, i),
                version.clone(),
                Action::Insert(Message::new(())),
            ));
        }
    }
    let a_node = act(None, shared, |_| ());

    // b's extras: a separate chain on a disjoint party, so they are causally
    // concurrent with a's version and survive deletion-pruning when provided.
    // Extras count down from 0xff so they never collide with a shared radix.
    let b_party = nth_party(1);
    let mut b_version = Version::new();
    let mut extras = Vec::new();
    for &(radix, _, n_extra) in spec {
        for i in 0..n_extra {
            b_version.tick(&b_party);
            extras.push((
                path(radix, 0xff - i),
                b_version.clone(),
                Action::Insert(Message::new(())),
            ));
        }
    }
    let b_node = act(a_node.clone(), extras, |_| ());

    let root = |node: Option<Node<(), height::Root>>| Root {
        ceiling: node
            .as_ref()
            .map(Node::ceiling)
            .cloned()
            .unwrap_or_default(),
        root: node,
    };
    (root(a_node), root(b_node))
}

/// A dispute that survives to leaf-parent height — both sides hold the same
/// `S<Z>` prefix with different leaf sets — converges to the union.
///
/// The responder's closing `uncertain` lists its leaves, and the leaf-height
/// `Closing`/`Complete` words carry the difference in both directions.
#[test]
fn converges_on_leaf_parent_dispute() {
    let (a, b, expected) = leaf_parent_dispute_pair();
    assert_eq!(
        with_watchdog(move || streaming_mirror(a, b)),
        expected,
        "both sides should hold the union",
    );
}

/// A leaf redacted on one side under a disputed leaf-parent must disappear
/// from the other side too: the closing request for it prunes against the
/// redactor's version and drops on both sides instead of shipping.
#[test]
fn honors_redaction_under_leaf_parent_dispute() {
    let (a, b, expected) = leaf_parent_redaction_pair();
    assert_eq!(
        with_watchdog(move || streaming_mirror(a, b)),
        expected,
        "the redacted leaf should survive nowhere",
    );
}

/// A stage's walk may route more reconciled children into its reassembly
/// channels than one fan before anything beneath it resolves; the session
/// must keep draining rather than deadlock.
///
/// This is the minimal overflow: 257 same-direction reassembly sends (256
/// Matched + 1 Provide verdicts into `level` on one side, the mirror-image
/// absorbs into `keep` on the other), one more than a FAN-bounded channel
/// plus the merge's single item of lookahead can absorb — while the descent
/// is empty, so no upward flush can relieve the pressure mid-walk.
#[test]
fn reassembly_survives_fan_overflow_minimal() {
    let (a, b) = one_sided_pair(&[(0x00, 254, 1), (0x01, 1, 1)]);
    let expected = alternating_mirror(a.clone(), b.clone());
    assert_eq!(streaming_mirror_with_timeout(a, b), expected);
}

/// The reassembly overflow grows with the number of dispute cells, not by a
/// constant: two full-fan cells route 512 reconciled children with nothing
/// below to flush them, so no constant channel bound absorbs this shape.
#[test]
fn reassembly_survives_fan_overflow_wide() {
    let (a, b) = one_sided_pair(&[(0x00, 255, 1), (0x01, 255, 1)]);
    let expected = alternating_mirror(a.clone(), b.clone());
    assert_eq!(streaming_mirror_with_timeout(a, b), expected);
}

proptest! {
    /// On divergent trees sharing causal history — matched subtrees, one-sided
    /// inserts, and redactions the other side must honor — the streaming
    /// mirror reconciles both sides to exactly the alternating oracle's root.
    #[test]
    fn matches_oracle_on_divergent_pair((a, b) in arb_divergent_pair()) {
        let expected = alternating_mirror(a.clone(), b.clone());
        prop_assert_eq!(streaming_mirror(a, b), expected);
    }

    /// On causally independent trees — including the bootstrap shape, where
    /// one side is empty and receives everything — the streaming mirror
    /// matches the alternating oracle.
    #[test]
    fn matches_oracle_on_independent_trees(
        a in arb_tree_root(0, 0..=8),
        b in arb_tree_root(1, 0..=8),
    ) {
        let expected = alternating_mirror(a.clone(), b.clone());
        prop_assert_eq!(streaming_mirror(a, b), expected);
    }

    /// Mirroring a tree with itself is a no-op: the handshake versions are
    /// equal, the session short-circuits before reconciliation, and both
    /// sides come back unchanged.
    #[test]
    fn idempotent(a in arb_tree_root(0, 0..=8)) {
        prop_assert_eq!(streaming_mirror(a.clone(), a.clone()), a);
    }
}
