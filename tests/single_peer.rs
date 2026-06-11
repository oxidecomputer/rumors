//! Single-peer correctness for a lone rumor set, with no gossip.
//!
//! Exercises the surface area of [`Batch`](rumors::Batch) commits:
//! live-leaf fan-out, `Key` distinctness within a batch, and strict
//! monotonicity of the local party's component of each minted
//! [`Version`](rumors::Version).

mod common;

use std::collections::{BTreeMap, BTreeSet};

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::sync::{Peer, Rumors};
use rumors::{Key, Version, causally};

/// Commit `values` to `peer` as one batch, returning the `(Key, Version)`
/// pairs it minted (recovered as the live leaves above the pre-commit
/// frontier).
fn batch_send(peer: &Rumors<u64>, values: &[u64]) -> Vec<(Key, Version)> {
    let pre = peer.snapshot().latest().clone();
    {
        let mut batch = peer.batch();
        for v in values {
            batch.send(*v);
        }
    }
    peer.snapshot()
        .range(causally::since(&pre))
        .map(|(k, v, _)| (k, v.clone()))
        .collect()
}

proptest! {
    /// Every value committed in a batch becomes exactly one live leaf:
    /// no duplicates, no omissions.
    #[test]
    fn batch_mints_once_per_value(values in vec(any::<u64>(), 0..=32)) {
        let peer = Peer::<u64>::seed().into_rumors();
        let minted = batch_send(&peer, &values);
        prop_assert_eq!(minted.len(), values.len());
        prop_assert_eq!(peer.snapshot().len(), values.len());
    }

    /// All `Key`s minted within a single batch are distinct, even when
    /// several values in the batch are equal.
    #[test]
    fn distinct_keys_per_batch(values in vec(any::<u64>(), 1..=32)) {
        let peer = Peer::<u64>::seed().into_rumors();
        let minted = batch_send(&peer, &values);
        prop_assert_eq!(minted.len(), values.len());
        let unique: BTreeSet<_> = minted.iter().map(|(k, _)| *k).collect();
        prop_assert_eq!(unique.len(), values.len(), "keys must be distinct");
    }

    /// The same value inserted `n` times in one batch still yields
    /// `n` distinct `Key`s — content equality does not collapse keys.
    #[test]
    fn duplicate_values_get_distinct_keys(n in 1usize..=16, value in any::<u64>()) {
        let peer = Peer::<u64>::seed().into_rumors();
        let values: Vec<u64> = std::iter::repeat_n(value, n).collect();
        let minted = batch_send(&peer, &values);
        prop_assert_eq!(minted.len(), n);
        let unique: BTreeSet<_> = minted.iter().map(|(k, _)| *k).collect();
        prop_assert_eq!(unique.len(), n);
    }

    /// Every `Version` minted by a lone peer is totally ordered against
    /// every other — both within a single batch (the batch docs promise
    /// strictly increasing versions per action) and across successive
    /// batches. With one party and no gossip there is no concurrency, so
    /// any incomparable or equal pair would betray a versioning bug.
    #[test]
    fn local_versions_form_a_chain(
        batches in vec(vec(any::<u64>(), 1..=8), 1..=8),
    ) {
        let peer = Peer::<u64>::seed().into_rumors();

        // Versions in commit order: per batch, the minted versions sorted
        // into their (total) causal order; batches concatenated in commit
        // order. Each batch's recovery is scoped by the pre-commit frontier.
        let mut versions: Vec<Version> = Vec::new();
        for batch in &batches {
            let mut minted: Vec<Version> =
                batch_send(&peer, batch).into_iter().map(|(_, v)| v).collect();
            minted.sort_by(|a, b| {
                a.partial_cmp(b).expect("a lone peer's versions are totally ordered")
            });
            versions.extend(minted);
        }

        // Strict precedence on causal versions is transitive, so
        // adjacent-pair monotonicity implies the full chain.
        for window in versions.windows(2) {
            prop_assert!(
                window[0] < window[1],
                "{:?} must strictly precede {:?}", window[0], window[1],
            );
        }
    }

    /// Final state after a batch commit does not depend on the input
    /// order. Inserting `values` and a Fisher-Yates shuffle of `values`
    /// into two fresh peers yields equal live value multisets.
    #[test]
    fn batch_state_is_input_order_independent(
        values in vec(any::<u64>(), 0..=16),
        seed in any::<u64>(),
    ) {
        let shuffled = {
            let mut v = values.clone();
            // Inline PCG-derived shuffle: deterministic from `seed`,
            // no extra dependency. The two `wrapping_mul` /
            // `wrapping_add` lines step a 64-bit LCG; any decent step
            // function works — we just need a uniform-enough draw
            // over `0..=i` at each Fisher-Yates iteration.
            let mut state = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
            for i in (1..v.len()).rev() {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let j = ((state >> 33) as usize) % (i + 1);
                v.swap(i, j);
            }
            v
        };

        // Each peer is its own fresh seed (the two never gossip, so they
        // need not share a universe). Read the live multiset directly off
        // the snapshot.
        let multiset_of = |values: &[u64]| -> BTreeMap<u64, usize> {
            let peer = Peer::<u64>::seed().into_rumors();
            batch_send(&peer, values);
            let mut out = BTreeMap::new();
            for (_, _, v) in peer.snapshot().iter() {
                *out.entry(**v).or_insert(0) += 1;
            }
            out
        };
        prop_assert_eq!(multiset_of(&values), multiset_of(&shuffled));
    }
}
