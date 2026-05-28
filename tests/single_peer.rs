//! Single-peer correctness for `Local`, with no gossip.
//!
//! Exercises the surface area of `Local::message`: callback fan-out,
//! `Key` distinctness within a batch, and strict monotonicity of the
//! local party's component of each emitted `Version`.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use imbl::OrdMap;
use proptest::collection::vec;
use proptest::prelude::*;
use rumors::sync::{Local, ignore};
use rumors::{Key, Version};

// The sync API's callback bound is `FnMut(...) + Send + 'a`. Direct
// `&mut` capture of locally-owned state is fine in principle (see the
// `sync_callback_can_borrow_local_state` regression in
// `api_send_bounds.rs`); the proptest helpers below still use
// `Arc<Mutex<_>>` because proptest's `move` closure infrastructure makes
// the borrow lifetimes awkward to thread through `prop_assert_eq!`.

proptest! {
    /// Every value passed to `Local::message` fires `on_message`
    /// exactly once: no duplicates, no omissions.
    #[test]
    fn insert_fires_once_per_value(values in vec(any::<u64>(), 0..=32)) {
        let mut peer = Local::<u64, _>::for_party("alice", 0).unwrap();
        let observed = Arc::new(Mutex::new(0usize));
        let observed_in = Arc::clone(&observed);
        peer.message(values.clone(), move |_, _, _| *observed_in.lock().unwrap() += 1);
        prop_assert_eq!(*observed.lock().unwrap(), values.len());
    }

    /// All `Key`s emitted within a single `Local::message` call are
    /// distinct, even when several values in the batch are equal.
    #[test]
    fn distinct_keys_per_batch(values in vec(any::<u64>(), 1..=32)) {
        let mut peer = Local::<u64, _>::for_party("alice", 0).unwrap();
        let keys: Arc<Mutex<Vec<Key>>> = Arc::new(Mutex::new(Vec::new()));
        let keys_in = Arc::clone(&keys);
        peer.message(values.clone(), move |k, _, _| keys_in.lock().unwrap().push(k));
        let keys = keys.lock().unwrap();
        prop_assert_eq!(keys.len(), values.len());
        let unique: BTreeSet<_> = keys.iter().copied().collect();
        prop_assert_eq!(unique.len(), keys.len(), "keys must be distinct");
    }

    /// The same value inserted `n` times in one batch still yields
    /// `n` distinct `Key`s — content equality does not collapse keys.
    #[test]
    fn duplicate_values_get_distinct_keys(n in 1usize..=16, value in any::<u64>()) {
        let mut peer = Local::<u64, _>::for_party("alice", 0).unwrap();
        let keys: Arc<Mutex<Vec<Key>>> = Arc::new(Mutex::new(Vec::new()));
        let keys_in = Arc::clone(&keys);
        peer.message(std::iter::repeat_n(value, n), move |k, _, _| keys_in.lock().unwrap().push(k));
        let keys = keys.lock().unwrap();
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
        let mut peer = Local::<u64, _>::for_party("alice", 0).unwrap();

        // Values in the order of insertion
        let mut values = Vec::new();

        // Map of each value to the version assigned it
        let all_versions: Arc<Mutex<OrdMap<u64, Version>>> = Arc::new(Mutex::new(OrdMap::new()));

        // Process the batches, tracking values and versions
        for batch in &batches {
            values.extend(batch.clone());
            let all_versions_in = Arc::clone(&all_versions);
            peer.message(batch.clone(), move |_, v, m| {
                all_versions_in.lock().unwrap().insert(**m, v.clone());
            });
        }

        // Ensure that for any two consecutive values, their
        // corresponding versions are ordered
        let all_versions = all_versions.lock().unwrap();
        for window in values.windows(2) {
            prop_assert!(
                all_versions.get(&window[0]) < all_versions.get(&window[1]),
                "{:?} must strictly precede {:?}", window[0], window[1],
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

        // `Local::for_party` is one-per-party-per-process, so we
        // can't hold both "alice"s alive simultaneously. Take each
        // peer's multiset readout in turn, then compare.
        //
        // `start` >= prior `event()` is the public-API contract for
        // reuse across drops; we use 0 here because the test never
        // gossips between the two `alice`s and so cannot witness any
        // version-vector corruption that might result.
        let multiset_of = |values: Vec<u64>| -> BTreeMap<u64, usize> {
            let mut peer = Local::<u64, _>::for_party("alice", 0).unwrap();
            peer.message(values, ignore);
            let out: Arc<Mutex<BTreeMap<u64, usize>>> = Arc::new(Mutex::new(BTreeMap::new()));
            let out_in = Arc::clone(&out);
            let mut lens = Local::<u64, _>::for_party(b"\x00READOUT\x00", 0).unwrap();
            lens.process(peer.fork(), move |_, _, v| {
                *out_in.lock().unwrap().entry(**v).or_insert(0) += 1;
            });
            Arc::try_unwrap(out)
                .expect("callback closure dropped after `process` returns")
                .into_inner()
                .unwrap()
        };
        prop_assert_eq!(multiset_of(values), multiset_of(shuffled));
    }
}
