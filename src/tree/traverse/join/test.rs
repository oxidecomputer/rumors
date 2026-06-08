//! `Tree::join` is observationally identical to mirroring two local trees: for
//! any divergent pair it produces the same merged tree and fires the same
//! callbacks (as a multiset). The oracle is the mirror engine driven directly
//! (not `Known::join_then`, which *is* `Tree::join` now).

use std::sync::Arc;

use proptest::prelude::*;

use crate::network::Network;
use crate::tree::arb::{arb_divergent_pair, arb_shared_delta_pair, arb_tree_root};
use crate::tree::key::Key;
use crate::tree::traverse::mirror::{local, mirror};
use crate::tree::{Root, Tree};
use crate::version::Version;

/// A captured callback log entry. Keys are unique per merge (one observation
/// per learned leaf, each at a distinct path), so sorting by key is a total,
/// canonical order — which lets us compare logs as multisets without needing a
/// total order on the (only partially-ordered) [`Version`].
type Log = Vec<(Key, Version)>;

fn by_key(mut log: Log) -> Log {
    log.sort_by_key(|(k, _)| *k);
    log
}

/// The no-callback type, for the silent side / unused direction.
type Silent = fn(Key, &Version, &Arc<()>) -> std::future::Ready<()>;

/// Drive the local-local mirror directly, capturing what the *first* side
/// learns from the second. This is the oracle: the merge engine `Tree::join`
/// must match, invoked without going through `Known` (whose `join_then` now
/// delegates to `Tree::join`).
fn mirror_capture(a: Root<()>, b: Root<()>) -> (Root<()>, Log) {
    let mut learned: Log = Vec::new();
    let merged = pollster::block_on(async {
        // Local-local oracle: the network/party greeting fields are inert here
        // (no lib-level dispatch runs), so a placeholder network and absent
        // party suffice — only the version the handshake carries is consumed.
        let l = local::Exchange::start(
            a,
            Network::ZERO,
            None,
            None::<Silent>,
            Some(|k: Key, v: &Version, _m: &Arc<()>| {
                learned.push((k, v.clone()));
                std::future::ready(())
            }),
        );
        let r = local::Exchange::silent(b);
        match mirror(l, r).await {
            Ok((merged, _)) => merged,
            Err(e) => match e {},
        }
    });
    (merged, learned)
}

/// Merge via `Tree::join`, capturing both directions: what `a` learns (`recv`)
/// and what `b` would learn from `a` (`send`).
fn join_capture(a: Root<()>, b: Root<()>) -> (Root<()>, Log, Log) {
    let mut recv: Log = Vec::new();
    let mut send: Log = Vec::new();
    let mut a = Tree { root: a };
    pollster::block_on(a.join(
        Tree { root: b },
        Some(|k: Key, v: &Version, _m: &Arc<()>| {
            recv.push((k, v.clone()));
            std::future::ready(())
        }),
        Some(|k: Key, v: &Version, _m: &Arc<()>| {
            send.push((k, v.clone()));
            std::future::ready(())
        }),
    ));
    (a.root, recv, send)
}

fn join_tree(a: Root<()>, b: Root<()>) -> Root<()> {
    join_capture(a, b).0
}

proptest! {
    /// `Tree::join` produces a byte-identical merged tree and the same
    /// `on_recv` / `on_send` multisets as the mirror, including honoring
    /// deletions by version dominance.
    #[test]
    fn join_matches_mirror((a, b) in arb_divergent_pair()) {
        let (tree_j, recv_j, send_j) = join_capture(a.clone(), b.clone());
        let (tree_m, recv_m) = mirror_capture(a.clone(), b.clone());

        // Same merged tree (version + structure; equal trees ⟹ equal hash).
        prop_assert_eq!(&tree_j, &tree_m);

        // `on_recv`: what `a` learns from `b`, same multiset as the mirror.
        prop_assert_eq!(by_key(recv_j.clone()), by_key(recv_m));

        // `on_send`: what `b` would learn from `a` — by symmetry, the mirror's
        // "b learns from a" run.
        let (_tree_ba, recv_ba) = mirror_capture(b, a);
        prop_assert_eq!(by_key(send_j), by_key(recv_ba));

        // `Tree::join` fires `on_recv` in its own deterministic ascending-key
        // order (a leaf's key is its full path; the recursion is radix-DFS).
        prop_assert_eq!(by_key(recv_j.clone()), recv_j);
    }

    // Note on delivery order: we checked empirically whether the recursion's
    // callback order matches the mirror's exactly. It does not, and cannot
    // naturally: the mirror delivers level-ordered (BFS by frontier-discovery
    // depth, a message-round artifact) while the recursion is DFS (ascending
    // key). They diverge as soon as divergences sit at different depths. Since
    // the public API contracts delivery order as unspecified, `join_matches_
    // mirror` asserts multiset parity; `Tree::join`'s own order is the clean,
    // deterministic ascending-key checked there.

    /// A small delta against a *wide shared* (forked) prefix: the steady-state
    /// gossip shape `join_small_delta` benchmarks, and the case `join`'s
    /// `OrdMap::diff` exists to make cheap. A wide shared base gives the children
    /// maps real B-tree depth, so this drives `diff`'s cross-level pointer-
    /// pruning (which the narrow `arb_divergent_pair` bases never reach) and
    /// confirms it still enumerates *exactly* the divergent radixes — no more, no
    /// fewer: `join` produces the same merged tree and callbacks as the mirror.
    ///
    /// Fewer cases than the block above: each draw builds two wide trees and runs
    /// the mirror twice, so the fixtures dominate.
    #[test]
    #[ignore = "wide fixtures: slow; run explicitly with --include-ignored"]
    fn join_wide_shared_small_delta_matches_mirror((a, b) in arb_shared_delta_pair(32..256)) {
        let (tree_j, recv_j, send_j) = join_capture(a.clone(), b.clone());
        let (tree_m, recv_m) = mirror_capture(a.clone(), b.clone());

        prop_assert_eq!(&tree_j, &tree_m);
        prop_assert_eq!(by_key(recv_j.clone()), by_key(recv_m));

        let (_tree_ba, recv_ba) = mirror_capture(b, a);
        prop_assert_eq!(by_key(send_j), by_key(recv_ba));

        prop_assert_eq!(by_key(recv_j.clone()), recv_j);
    }

    /// Merging a tree with itself is a content no-op and observes nothing.
    #[test]
    fn join_idempotent((a, _b) in arb_divergent_pair()) {
        let (tree_j, recv_j, send_j) = join_capture(a.clone(), a.clone());
        prop_assert_eq!(tree_j, a);
        prop_assert!(recv_j.is_empty());
        prop_assert!(send_j.is_empty());
    }

    /// The merged tree is independent of merge direction.
    #[test]
    fn join_commutative((a, b) in arb_divergent_pair()) {
        prop_assert_eq!(join_tree(a.clone(), b.clone()), join_tree(b, a));
    }

    /// The merge is associative over three mutually-disjoint trees. (Uses
    /// `arb_tree_root` on three distinct party indices so the three are pairwise
    /// disjoint; `arb_divergent_pair` bakes in parties 0/1/2 and so cannot be
    /// composed three-way. Associativity in the presence of redactions is
    /// covered transitively: `join` matches the mirror, which proves it under
    /// its own redacting generators.)
    #[test]
    fn join_associative(
        a in arb_tree_root(0, 0..6),
        b in arb_tree_root(1, 0..6),
        c in arb_tree_root(2, 0..6),
    ) {
        let left = join_tree(join_tree(a.clone(), b.clone()), c.clone());
        let right = join_tree(a, join_tree(b, c));
        prop_assert_eq!(left, right);
    }
}
