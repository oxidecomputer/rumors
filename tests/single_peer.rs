//! Single-peer correctness for `Known`, with no gossip.
//!
//! Exercises the surface area of `Known::message`: callback fan-out,
//! `Key` distinctness within a batch, and strict monotonicity of the
//! local party's component of each emitted `Version`.

use std::collections::BTreeSet;

use imbl::OrdMap;
use proptest::collection::vec;
use proptest::prelude::*;
use rumors::sync::Known;
use rumors::{Key, Version};

// The sync API's callback bound is `FnMut(...) + Send + 'a`, so the
// closures below borrow locally-owned state directly (`&mut`); no
// `Arc<Mutex<_>>` shuttling is needed.

proptest! {
    /// Every value passed to `Known::message` fires `on_message`
    /// exactly once: no duplicates, no omissions.
    #[test]
    fn insert_fires_once_per_value(values in vec(any::<u64>(), 0..=32)) {
        let mut peer = Known::<u64>::seed();
        let mut observed = 0usize;
        peer.message_then(values.clone(), |_, _, _| observed += 1);
        prop_assert_eq!(observed, values.len());
    }

    /// All `Key`s emitted within a single `Known::message` call are
    /// distinct, even when several values in the batch are equal.
    #[test]
    fn distinct_keys_per_batch(values in vec(any::<u64>(), 1..=32)) {
        let mut peer = Known::<u64>::seed();
        let mut keys: Vec<Key> = Vec::new();
        peer.message_then(values.clone(), |k, _, _| keys.push(k));
        prop_assert_eq!(keys.len(), values.len());
        let unique: BTreeSet<_> = keys.iter().copied().collect();
        prop_assert_eq!(unique.len(), keys.len(), "keys must be distinct");
    }

    /// The same value inserted `n` times in one batch still yields
    /// `n` distinct `Key`s — content equality does not collapse keys.
    #[test]
    fn duplicate_values_get_distinct_keys(n in 1usize..=16, value in any::<u64>()) {
        let mut peer = Known::<u64>::seed();
        let mut keys: Vec<Key> = Vec::new();
        peer.message_then(std::iter::repeat_n(value, n), |k, _, _| keys.push(k));
        prop_assert_eq!(keys.len(), n);
        prop_assert_eq!(keys.iter().copied().collect::<BTreeSet<_>>().len(), n);
    }

    /// Every `Version` reported to `on_message` for a locally-inserted
    /// message strictly dominates the immediately prior one from the
    /// same peer, both within a single `message` batch and across
    /// successive batches. (Strict precedence on vector clocks is
    /// transitive, so adjacent-pair monotonicity implies full
    /// monotonicity across the whole sequence.)
    #[test]
    fn local_versions_strictly_increasing(
        batches in vec(vec(any::<u64>(), 1..=8), 1..=8),
    ) {
        let mut peer = Known::<u64>::seed();

        // Values in the order of insertion.
        let mut values = Vec::new();

        // Map of each value to the version assigned it.
        let mut all_versions: OrdMap<u64, Version> = OrdMap::new();

        for batch in &batches {
            values.extend(batch.clone());
            peer.message_then(batch.clone(), |_, v, m| {
                all_versions.insert(**m, v.clone());
            });
        }

        // For any two consecutive values, their versions must be ordered.
        for window in values.windows(2) {
            prop_assert!(
                all_versions.get(&window[0]) < all_versions.get(&window[1]),
                "{:?} must strictly precede {:?}", window[0], window[1],
            );
        }
    }

    /// Final state after `Known::message(values)` does not depend on
    /// the input order. Inserting `values` and a Fisher-Yates shuffle
    /// of `values` into two fresh peers yields equal live value
    /// multisets.
    ///
    /// Callback order within a batch is unspecified by the public
    /// API; this test pins down that the *resulting state* is
    /// order-independent even when the callback firing order isn't.
    #[test]
    fn message_state_is_input_order_independent(
        values in vec(any::<u64>(), 0..=16),
        seed in any::<u64>(),
    ) {
        use std::collections::BTreeMap;

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
        // `Known::iter`.
        let multiset_of = |values: Vec<u64>| -> BTreeMap<u64, usize> {
            let mut peer = Known::<u64>::seed();
            peer.message(values);
            let mut out = BTreeMap::new();
            for (_, _, v) in peer.iter() {
                *out.entry(**v).or_insert(0) += 1;
            }
            out
        };
        prop_assert_eq!(multiset_of(values), multiset_of(shuffled));
    }
}
