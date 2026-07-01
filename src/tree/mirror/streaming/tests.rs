use std::cell::OnceCell;
use std::future::Future;
use std::time::Duration;

use proptest::prelude::*;
use tokio::runtime::Runtime;

use crate::Network;
use crate::tree::arb::{arb_divergent_pair, arb_tree_root};
use crate::tree::mirror::alternating;

thread_local! {
    /// One current-thread tokio runtime per test thread, initialized lazily on
    /// first use (see the alternating tests for the rationale).
    static RT: OnceCell<Runtime> = const { OnceCell::new() };
}

/// The per-case deadline. The streaming session is a set of channel-coupled
/// pumps driven concurrently; a scheduling bug in it manifests as a deadlock,
/// so every drive is bounded to turn a hang into a failure.
const DEADLINE: Duration = Duration::from_secs(30);

/// Drive an async future to completion on the per-thread runtime, panicking
/// if it exceeds [`DEADLINE`].
fn block_on<F: Future>(fut: F) -> F::Output {
    RT.with(|cell| {
        cell.get_or_init(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("build tokio current-thread runtime")
        })
        .block_on(async {
            tokio::time::timeout(DEADLINE, fut)
                .await
                .expect("streaming mirror session deadlocked")
        })
    })
}

/// Reconcile `a` and `b` through the streaming local backend, asserting the
/// two sides converge to the same root, and return it.
fn streaming_mirror(a: crate::tree::Root<()>, b: crate::tree::Root<()>) -> crate::tree::Root<()> {
    let (ours, theirs) = block_on(super::mirror(a, b));
    assert_eq!(ours, theirs, "streaming endpoints should converge");
    ours
}

/// Reconcile `a` and `b` through the alternating implementation: the
/// behavioral oracle the streaming protocol must reproduce exactly.
fn alternating_mirror(a: crate::tree::Root<()>, b: crate::tree::Root<()>) -> crate::tree::Root<()> {
    block_on(async {
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

/// A deterministic minimal session — two small causally independent trees —
/// agrees with the oracle: fast, seedless signal ahead of the proptests.
#[test]
fn smoke() {
    let mk = |party: usize, n: usize| {
        let p = crate::tree::arb::nth_party(party);
        let mut t = crate::tree::Tree::new();
        t.act(
            &p,
            (0..n).map(|_| crate::tree::Action::Insert(crate::message::Message::new(()))),
        );
        t.root
    };
    let a = mk(0, 2);
    let b = mk(1, 1);
    let expected = alternating_mirror(a.clone(), b.clone());
    assert_eq!(streaming_mirror(a, b), expected);
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
