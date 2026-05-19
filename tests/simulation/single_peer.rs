//! Single-peer correctness for `Local`, with no gossip.
//!
//! Exercises the surface area of `Local::message`: callback fan-out,
//! `Key` distinctness within a batch, and strict monotonicity of the
//! local party's component of each emitted `Version`.

use std::collections::BTreeSet;

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::{Key, Local, Version};

proptest! {
    /// Every value passed to `Local::message` fires `on_message`
    /// exactly once: no duplicates, no omissions.
    #[test]
    fn insert_fires_once_per_value(values in vec(any::<u64>(), 0..=32)) {
        let mut peer: Local<u64> = Local::for_party("alice");
        let mut observed = 0usize;
        peer.message(values.clone(), |_, _, _| observed += 1);
        prop_assert_eq!(observed, values.len());
    }

    /// All `Key`s emitted within a single `Local::message` call are
    /// distinct, even when several values in the batch are equal.
    #[test]
    fn distinct_keys_per_batch(values in vec(any::<u64>(), 1..=32)) {
        let mut peer: Local<u64> = Local::for_party("alice");
        let mut keys: Vec<Key> = Vec::new();
        peer.message(values.clone(), |k, _, _| keys.push(k));
        prop_assert_eq!(keys.len(), values.len());
        let unique: BTreeSet<_> = keys.iter().copied().collect();
        prop_assert_eq!(unique.len(), keys.len(), "keys must be distinct");
    }

    /// The same value inserted `n` times in one batch still yields
    /// `n` distinct `Key`s — content equality does not collapse keys.
    #[test]
    fn duplicate_values_get_distinct_keys(n in 1usize..=16, value in any::<u64>()) {
        let mut peer: Local<u64> = Local::for_party("alice");
        let mut keys: Vec<Key> = Vec::new();
        peer.message(std::iter::repeat_n(value, n), |k, _, _| keys.push(k));
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
        let mut peer: Local<u64> = Local::for_party("alice");
        let mut all_versions: Vec<Version> = Vec::new();
        for batch in &batches {
            peer.message(batch.clone(), |_, v, _| all_versions.push(v.clone()));
        }
        for w in all_versions.windows(2) {
            prop_assert!(
                w[0] < w[1],
                "{:?} must strictly precede {:?}", w[0], w[1],
            );
        }
    }

    /// Final state after `Local::message(values)` does not depend on
    /// the input order. Inserting `values` and a Fisher-Yates shuffle
    /// of `values` into two fresh peers (same party tag) yields
    /// equal live value multisets.
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

        let mut a: Local<u64> = Local::for_party("alice");
        a.message(values, |_, _, _| {});
        let mut b: Local<u64> = Local::for_party("alice");
        b.message(shuffled, |_, _, _| {});

        // Live content (value multiset) must match, even though the
        // per-value `Key`s differ — different version-counter
        // positions assigned by `message`.
        let multiset_of = |peer: &Local<u64>| -> BTreeMap<u64, usize> {
            let mut out = BTreeMap::new();
            let mut lens: Local<u64> = Local::for_party(b"\x00READOUT\x00");
            lens.process(peer.clone(), |_, _, v| {
                *out.entry(**v).or_insert(0) += 1;
            });
            out
        };
        prop_assert_eq!(multiset_of(&a), multiset_of(&b));
    }
}
