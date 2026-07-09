use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use proptest::prelude::*;

use crate::Version;
use crate::message::Message;
use crate::tree::Root;
use crate::tree::arb::{arb_divergent_pair, arb_tree_root, nth_party};
use crate::tree::mirror::streaming::{Converted, Handshaking, Local};
use crate::tree::mirror::{Error, alternating};
use crate::tree::traverse::{Action, act};
use crate::tree::typed::{Node, Path, height};

/// Reconcile `a` and `b` through the streaming local backend, asserting the
/// two sides converge to the same root, and return it.
fn streaming_mirror(a: Root<()>, b: Root<()>) -> Root<()> {
    let (a, b): (super::Root<Local, ()>, super::Root<Local, ()>) = (a.into(), b.into());
    let client = Handshaking::start(Local, a.clone());
    let server = Handshaking::start(Local, b.clone());
    let (ours, theirs) = pollster::block_on(super::mirror(client, server))
        .unwrap_or_else(|e| match e {})
        // Equal handshake versions: already converged, both sides unchanged.
        .unwrap_or((a, b));
    let (ours, theirs) = (ours.into(), theirs.into());
    assert_eq!(ours, theirs, "streaming endpoints should converge");
    ours
}

/// Reconcile `a` and `b` with the server wrapped in [`Converted`], asserting
/// the two sides converge, and return the client's root.
///
/// `Local` converts to itself, so this pairs a backend with its own node types
/// through the whole conversion machinery — every crossing node explodes to
/// leaves and reassembles — rather than through the identity the unwrapped
/// session enjoys. It pins two things at once: that a wrapped party still
/// satisfies [`Server`](super::protocol::Server), and that a round trip
/// through [`Converted`] is the identity on nodes.
fn converted_mirror(a: Root<()>, b: Root<()>) -> Root<()> {
    let (a, b): (super::Root<Local, ()>, super::Root<Local, ()>) = (a.into(), b.into());
    let client = Handshaking::start(Local, a.clone());
    let server = Converted::new(Handshaking::start(Local, b.clone()), Local, Local);
    let (ours, theirs) = pollster::block_on(super::mirror(client, server))
        .unwrap_or_else(|e| match e {
            Error::Client(e) => match e {},
            // The server's faults are its own or its counterparty's
            // representation of a node; `Local` is infallible in both.
            Error::Server(e) => match e {
                Error::Client(e) | Error::Server(e) => match e {},
            },
        })
        .unwrap_or((a, b));
    let (ours, theirs) = (ours.into(), theirs.into());
    assert_eq!(ours, theirs, "converted endpoints should converge");
    ours
}

/// Reconcile `a` and `b` through the alternating implementation: the
/// behavioral oracle the streaming protocol must reproduce exactly.
fn alternating_mirror(a: Root<()>, b: Root<()>) -> Root<()> {
    pollster::block_on(async {
        let local_a = alternating::local::Exchange::start(a);
        let local_b = alternating::local::Exchange::start(b);
        match alternating::mirror(local_a, local_b).await {
            Err(e) => match e {},
            Ok((ours, theirs)) => {
                assert_eq!(ours, theirs, "oracle endpoints should converge");
                ours
            }
        }
    })
}

/// Run [`streaming_mirror`] under a watchdog: a deadlocked session fails the
/// test in bounded time instead of hanging the suite.
fn streaming_mirror_with_timeout(a: Root<()>, b: Root<()>) -> Root<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(streaming_mirror(a, b));
    });
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(root) => root,
        Err(RecvTimeoutError::Timeout) => {
            panic!("streaming mirror deadlocked: no result within the watchdog timeout")
        }
        Err(RecvTimeoutError::Disconnected) => panic!("streaming mirror worker panicked"),
    }
}

/// Build a divergent pair whose every difference is one-sided, shaped by
/// `spec`: for each `(radix, shared, extra)` root child, both trees hold
/// `shared` identical leaves under it and `b` additionally holds `extra`
/// concurrent ones.
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

    /// A session with one party wrapped in `Converted` reconciles to exactly
    /// the same root as the unwrapped session: re-representing every node that
    /// crosses the wire changes nothing an endpoint can observe.
    #[test]
    fn converted_matches_oracle_on_divergent_pair((a, b) in arb_divergent_pair()) {
        let expected = alternating_mirror(a.clone(), b.clone());
        prop_assert_eq!(converted_mirror(a, b), expected);
    }

    /// Wrapping survives the bootstrap shape, where one side is empty and every
    /// leaf in the tree crosses the conversion boundary.
    #[test]
    fn converted_matches_oracle_on_independent_trees(
        a in arb_tree_root(0, 0..=8),
        b in arb_tree_root(1, 0..=8),
    ) {
        let expected = alternating_mirror(a.clone(), b.clone());
        prop_assert_eq!(converted_mirror(a, b), expected);
    }

    /// Mirroring a tree with itself is a no-op: the handshake versions are
    /// equal, the session short-circuits before reconciliation, and both
    /// sides come back unchanged.
    #[test]
    fn idempotent(a in arb_tree_root(0, 0..=8)) {
        prop_assert_eq!(streaming_mirror(a.clone(), a.clone()), a);
    }
}
