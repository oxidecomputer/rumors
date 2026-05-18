//! Group A — single-peer correctness, no gossip.

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::{Key, Local, Version};
use std::collections::BTreeSet;

proptest! {
    /// A1: every value passed to `Local::message` fires `on_message`
    /// exactly once, with no duplicates or omissions.
    #[test]
    fn a1_insert_observation_count(values in vec(any::<u64>(), 0..=32)) {
        let mut peer: Local<u64> = Local::for_party("alice");
        let mut observed = 0usize;
        peer.message(values.clone(), |_, _, _| observed += 1);
        prop_assert_eq!(observed, values.len());
    }

    /// A2: every callback in a single `Local::message` call receives a
    /// distinct `Key`, regardless of value duplication within the batch.
    #[test]
    fn a2_distinct_keys_per_batch(values in vec(any::<u64>(), 1..=32)) {
        let mut peer: Local<u64> = Local::for_party("alice");
        let mut keys: Vec<Key> = Vec::new();
        peer.message(values.clone(), |k, _, _| keys.push(k));
        prop_assert_eq!(keys.len(), values.len());
        let unique: BTreeSet<_> = keys.iter().copied().collect();
        prop_assert_eq!(unique.len(), keys.len(), "keys must be distinct");
    }

    /// A2 (duplicate content variant): the same value inserted N times
    /// in one batch still yields N distinct keys.
    #[test]
    fn a2_duplicate_content_distinct_keys(n in 1usize..=16, value in any::<u64>()) {
        let mut peer: Local<u64> = Local::for_party("alice");
        let mut keys: Vec<Key> = Vec::new();
        peer.message(std::iter::repeat(value).take(n), |k, _, _| keys.push(k));
        prop_assert_eq!(keys.len(), n);
        prop_assert_eq!(keys.iter().copied().collect::<BTreeSet<_>>().len(), n);
    }

    /// A3: a peer's own party-component of the `Version` reported to
    /// `on_message` is strictly increasing across all locally-inserted
    /// messages, both within a single `message` batch and across
    /// successive batches.
    ///
    /// We compare versions via the public `PartialOrd` (the happens-
    /// before lattice): each subsequent local insert must strictly
    /// dominate every prior local insert at the same peer.
    #[test]
    fn a3_local_versions_strictly_increasing(
        batches in vec(vec(any::<u64>(), 1..=8), 1..=8),
    ) {
        let mut peer: Local<u64> = Local::for_party("alice");
        let mut all_versions: Vec<Version> = Vec::new();
        for batch in &batches {
            peer.message(batch.clone(), |_, v, _| all_versions.push(v.clone()));
        }
        // Every later local-insert version strictly dominates every
        // earlier one.
        for i in 0..all_versions.len() {
            for j in (i + 1)..all_versions.len() {
                prop_assert!(
                    all_versions[i] < all_versions[j],
                    "version[{}] ({:?}) must strictly precede version[{}] ({:?})",
                    i, all_versions[i], j, all_versions[j],
                );
            }
        }
    }

}
