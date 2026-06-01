//! Exhaustive small-scope differential checking.
//!
//! The op-trace generator only ever produces the tree *shapes operations
//! produce*, and the arbitrary-normal-form strategies sample *arbitrary*
//! normal-form trees at random. Neither guarantees that any *particular* small
//! corner is hit. This module closes that gap by brute force: it **enumerates
//! every distinct normal-form id tree** up to a depth bound, and **every
//! distinct normal-form event tree with bases in `{0, 1, 2}`** up to the same
//! bound, then runs every operation on every tree and every ordered pair,
//! diffing each result structurally against the recursive oracle (the same
//! ground truth the sampled differentials use).
//!
//! The small-scope hypothesis fits ITC especially well: the normal-form trees
//! of a given small depth are few, so total enumeration is cheap, yet it
//! deterministically reaches the edge shapes random sampling under-hits — a
//! `grow` tie at the very root, an empty-child spine corner, the `close_node`
//! truncate-adjacency boundary, the `is_disjoint`/`compare` overlap arms, the
//! concurrent (`None`) verdict.
//!
//! The id and event depth bounds are **decoupled**, because the two corpora
//! grow at very different rates: an id node branches binary (leaves `0`/`1`),
//! so the id corpus is `2`, `4`, `16`, `256`, `65536` at depths `0..=4`; an
//! event node also chooses a base from `{0, 1, 2}`, so the event corpus is `3`,
//! `19`, `691`, ~1.4M at depths `0..=3` — a single shared bound deep enough for
//! ids (depth 3) would make the event cross-product (`O(corpus²)`) intractable.
//! So events are held one level shallower than ids.
//!
//! Each corpus is lowered to its impl form once and the pair loops *borrow* it
//! (not re-lowering both operands per pair), and the cross-products run on a
//! `rayon` pool. Two variants:
//!
//! - [`exhaustive_small`] runs in the normal gate at [`ID_SMALL_DEPTH`] /
//! [`EV_SMALL_DEPTH`] (256 ids, 691 events); the full op cross-product is well
//! under a second.
//!
//! - [`exhaustive_deep`] is `#[ignore]`d and runs at [`ID_DEEP_DEPTH`] /
//! [`EV_DEEP_DEPTH`] (65536 ids, 691 events); the `O(corpus²)` id pair-product
//! dominates — ~4.5 minutes on a 16-core M4 Max. See its doc comment for how to
//! run it.

#[cfg(test)]
mod tests;

use crate::oracle;

/// Inclusive id depth bound for the fast, gate-resident enumeration. A "depth"
/// is the number of interior-node levels: depth 0 is a bare leaf, depth 1 is a
/// node over two leaves, etc. Depth 3 yields 256 ids — enough to reach the
/// close-up corners while the `corpus²` id cross-product (~65k pairs) stays
/// well under a second.
pub(crate) const ID_SMALL_DEPTH: usize = 3;

/// Inclusive event depth bound for the fast, gate-resident enumeration. Held
/// one level below [`ID_SMALL_DEPTH`]: depth 2 yields 691 events (depth 3 would
/// be ~1.4M, whose `O(corpus²)` cross-product is intractable). 691 events
/// already exercises every base-ordering at a node.
pub(crate) const EV_SMALL_DEPTH: usize = 2;

/// Inclusive id depth bound for the `#[ignore]`d deep enumeration: 65536 ids.
/// The id cross- product is `O(corpus²)` (~4.3 billion pairs); with the
/// per-tree precompute and `rayon` it runs in ~4.5 minutes on a 16-core M4 Max.
/// See [`tests::exhaustive_deep`].
pub(crate) const ID_DEEP_DEPTH: usize = 4;

/// Inclusive event depth bound for the deep enumeration. Stays at 691 events:
/// the depth-3 event corpus (~1.4M) makes the event/tick cross-products
/// intractable even off the gate.
pub(crate) const EV_DEEP_DEPTH: usize = 2;

/// The event-base alphabet for the exhaustive enumeration: `{0, 1, 2}`. Zero is
/// required for normal form (every event node carries a zero-base child); `1`
/// and `2` give both a minimal nonzero increment and a value that can dominate
/// it, so relative-base orderings at a node are exercised in both directions.
const BASES: [u64; 3] = [0, 1, 2];

/// Every distinct **normal-form** id tree of depth `≤ depth`.
///
/// Enumerated by building all *raw* trees up to the bound and folding each
/// through the oracle's normalizing constructor [`oracle::Party::node`] (which
/// collapses `(0, 0)` and `(1, 1)`), then deduplicating. Dedup is essential:
/// many raw shapes normalize to the same canonical tree, and the differential
/// harness keys `Eq`/`Hash` on canonical form, so the corpus must be the set of
/// *canonical* trees. Iterative worklist over depth levels (the impl's own
/// traversals are iterative; this is test scaffolding, but the same discipline
/// keeps it allocation-bounded and obvious).
pub(crate) fn all_normal_ids(depth: usize) -> Vec<oracle::Party> {
    use oracle::Party as P;
    use std::collections::BTreeSet;

    // `pool` holds the deduped *canonical* trees of depth `≤ d`, keyed for
    // de-dup by a cheap injective preorder encoding (`Party` has no `Ord`).
    // Deduping at every level — not just at the end — is what keeps the corpus
    // (and the op cross-product) tractable: a node built from two undeduped
    // children multiplies the redundancy.
    let mut seen: BTreeSet<Vec<u8>> = BTreeSet::new();
    let mut pool: Vec<P> = Vec::new();
    for leaf in [P::Leaf(false), P::Leaf(true)] {
        if seen.insert(id_key(&leaf)) {
            pool.push(leaf);
        }
    }
    for _ in 1..=depth {
        // A depth-`d` (or shallower) tree is a leaf (already in `pool`) or a
        // node whose children are each of depth `≤ d - 1` (the current `pool`).
        // Generate the new nodes against a frozen snapshot, then fold the
        // canonical fresh ones back in.
        let prev: Vec<P> = pool.clone();
        for l in &prev {
            for r in &prev {
                let t = P::node(l.clone(), r.clone());
                if seen.insert(id_key(&t)) {
                    pool.push(t);
                }
            }
        }
    }
    pool
}

/// Every distinct **normal-form** event tree of depth `≤ depth` with every base
/// in [`BASES`]. Built and deduped exactly as [`all_normal_ids`], folding each
/// raw node through [`oracle::Version::node`] (which enforces the
/// zero-base-child rule and collapses equal leaves).
pub(crate) fn all_normal_events(depth: usize) -> Vec<oracle::Version> {
    use oracle::Version as V;
    use std::collections::BTreeSet;

    let mut seen: BTreeSet<Vec<u8>> = BTreeSet::new();
    let mut pool: Vec<V> = Vec::new();
    for &b in &BASES {
        let leaf = V::leaf(b);
        if seen.insert(ev_key(&leaf)) {
            pool.push(leaf);
        }
    }
    for _ in 1..=depth {
        let prev: Vec<V> = pool.clone();
        for &n in &BASES {
            for l in &prev {
                for r in &prev {
                    let t = V::node(n, l.clone(), r.clone());
                    if seen.insert(ev_key(&t)) {
                        pool.push(t);
                    }
                }
            }
        }
    }
    pool
}

/// Injective preorder encoding of a (canonical) id tree, used only as a de-dup
/// key.
fn id_key(t: &oracle::Party) -> Vec<u8> {
    use oracle::Party as P;
    let mut out = Vec::new();
    let mut stack = vec![t];
    while let Some(n) = stack.pop() {
        match n {
            P::Leaf(b) => {
                out.push(if *b { 2 } else { 1 });
            }
            P::Node(l, r) => {
                out.push(0);
                stack.push(r);
                stack.push(l);
            }
        }
    }
    out
}

/// Injective preorder encoding of a (canonical) event tree, used only as a
/// de-dup key. Bases are in [`BASES`] (single-digit), so a one-byte tag per
/// base is injective.
fn ev_key(t: &oracle::Version) -> Vec<u8> {
    use oracle::Version as V;
    let mut out = Vec::new();
    let mut stack = vec![t];
    while let Some(n) = stack.pop() {
        match n {
            V::Leaf(b) => {
                out.push(1);
                out.extend_from_slice(&b.to_bytes_le());
                out.push(0xff); // base terminator (bases are small; this never collides)
            }
            V::Node(b, l, r) => {
                out.push(2);
                out.extend_from_slice(&b.to_bytes_le());
                out.push(0xff);
                stack.push(r);
                stack.push(l);
            }
        }
    }
    out
}
