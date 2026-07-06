use proptest::prelude::*;

use crate::tree::arb::{arb_divergent_pair, arb_tree_root};
use crate::tree::mirror::alternating;
use crate::tree::mirror::streaming::{Handshaking, Local};

/// Reconcile `a` and `b` through the streaming local backend, asserting the
/// two sides converge to the same root, and return it.
fn streaming_mirror(a: crate::tree::Root<()>, b: crate::tree::Root<()>) -> crate::tree::Root<()> {
    let (a, b): (super::Root<Local, ()>, super::Root<Local, ()>) = (a.into(), b.into());
    let client = Handshaking::start(Local, Local, a.clone());
    let server = Handshaking::start(Local, Local, b.clone());
    let (ours, theirs) = pollster::block_on(super::mirror(client, server))
        .unwrap_or_else(|e| match e {})
        // Equal handshake versions: already converged, both sides unchanged.
        .unwrap_or((a, b));
    let (ours, theirs) = (ours.into(), theirs.into());
    assert_eq!(ours, theirs, "streaming endpoints should converge");
    ours
}

/// Reconcile `a` and `b` through the alternating implementation: the
/// behavioral oracle the streaming protocol must reproduce exactly.
fn alternating_mirror(a: crate::tree::Root<()>, b: crate::tree::Root<()>) -> crate::tree::Root<()> {
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
