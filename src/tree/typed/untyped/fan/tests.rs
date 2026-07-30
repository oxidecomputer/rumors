use std::collections::BTreeMap;

use proptest::collection::vec;
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

use crate::{Version, message::Message};

use super::super::Node;
use super::Fan;

/// A fresh child handle with its own backing allocation.
///
/// Each call yields a distinct `Arc`, so `Node::ptr_eq` distinguishes any
/// two children a test creates: the differential comparisons below check
/// *which* handle a fan holds, not merely that one is present.
fn child() -> Node<()> {
    Node::leaf(Version::new(), Message::new(()))
}

/// One step of the differential op sequence.
#[derive(Debug, Clone)]
enum Op {
    Insert(u8),
    Remove(u8),
}

/// A short sequence of inserts and removals over arbitrary radixes.
fn arb_ops() -> impl Strategy<Value = Vec<Op>> {
    vec(
        prop_oneof![
            any::<u8>().prop_map(Op::Insert),
            any::<u8>().prop_map(Op::Remove),
        ],
        1..64,
    )
}

/// Check every observation of `fan` against the `oracle` holding clones of
/// the same handles.
///
/// Compared: length, emptiness, both iteration directions (radix sequence
/// and per-child identity), the values-only walk both ways, point lookups
/// across the whole alphabet, and the successor probe against the oracle's
/// range query.
fn equivalent(fan: &Fan<()>, oracle: &BTreeMap<u8, Node<()>>) -> Result<(), TestCaseError> {
    prop_assert_eq!(fan.len(), oracle.len());
    prop_assert_eq!(fan.is_empty(), oracle.is_empty());

    for ((radix, child), (expected_radix, expected)) in fan.iter().zip(oracle.iter()) {
        prop_assert_eq!(radix, *expected_radix);
        prop_assert!(child.ptr_eq(expected));
    }
    prop_assert_eq!(fan.iter().count(), oracle.len());

    for ((radix, child), (expected_radix, expected)) in fan.iter().rev().zip(oracle.iter().rev()) {
        prop_assert_eq!(radix, *expected_radix);
        prop_assert!(child.ptr_eq(expected));
    }

    prop_assert_eq!(fan.values().len(), oracle.len());
    for (child, expected) in fan.values().zip(oracle.values()) {
        prop_assert!(child.ptr_eq(expected));
    }
    for (child, expected) in fan.values().rev().zip(oracle.values().rev()) {
        prop_assert!(child.ptr_eq(expected));
    }

    for radix in u8::MIN..=u8::MAX {
        match (fan.get(radix), oracle.get(&radix)) {
            (Some(child), Some(expected)) => prop_assert!(child.ptr_eq(expected)),
            (None, None) => {}
            (ours, theirs) => prop_assert!(
                false,
                "get({}) diverged: fan {:?}, oracle {:?}",
                radix,
                ours.is_some(),
                theirs.is_some(),
            ),
        }
        match (fan.successor(radix), oracle.range(radix..).next()) {
            (Some((at, child)), Some((expected_at, expected))) => {
                prop_assert_eq!(at, *expected_at);
                prop_assert!(child.ptr_eq(expected));
            }
            (None, None) => {}
            (ours, theirs) => prop_assert!(
                false,
                "successor({}) diverged: fan {:?}, oracle {:?}",
                radix,
                ours.map(|(at, _)| at),
                theirs.map(|(at, _)| *at),
            ),
        }
    }
    Ok(())
}

proptest! {
    /// After every step of any insert/remove sequence, a fan observes
    /// identically to a `BTreeMap` fed the same operations.
    ///
    /// Same length, same ascending and descending iteration (radixes and
    /// handle identities), same point lookups, same successor answers, and
    /// the same displaced/removed handles returned from the mutations.
    #[test]
    fn fan_matches_btreemap_oracle(ops in arb_ops()) {
        let mut fan = Fan::new();
        let mut oracle = BTreeMap::new();
        for op in ops {
            match op {
                Op::Insert(radix) => {
                    let node = child();
                    let displaced = fan.insert(radix, node.clone());
                    let expected = oracle.insert(radix, node);
                    prop_assert_eq!(displaced.is_some(), expected.is_some());
                    if let (Some(displaced), Some(expected)) = (displaced, expected) {
                        prop_assert!(displaced.ptr_eq(&expected));
                    }
                }
                Op::Remove(radix) => {
                    let removed = fan.remove(radix);
                    let expected = oracle.remove(&radix);
                    prop_assert_eq!(removed.is_some(), expected.is_some());
                    if let (Some(removed), Some(expected)) = (removed, expected) {
                        prop_assert!(removed.ptr_eq(&expected));
                    }
                }
            }
            equivalent(&fan, &oracle)?;
        }
    }

    /// Any insert/remove sequence leaves the fan strictly ascending by
    /// radix with no duplicates: the structural invariant the hash
    /// preimage, the wire encoding, and the merge walks read without
    /// re-sorting.
    #[test]
    fn ops_preserve_strict_ascent(ops in arb_ops()) {
        let mut fan = Fan::new();
        for op in ops {
            match op {
                Op::Insert(radix) => {
                    fan.insert(radix, child());
                }
                Op::Remove(radix) => {
                    fan.remove(radix);
                }
            }
        }
        let radixes: Vec<u8> = fan.iter().map(|(radix, _)| radix).collect();
        prop_assert!(
            radixes.windows(2).all(|pair| pair[0] < pair[1]),
            "fan radixes not strictly ascending: {:?}",
            radixes,
        );
    }

    /// `size_hint` stays exact under partial consumption from both ends,
    /// for the borrowing and the consuming walks alike.
    ///
    /// After taking `j` from the front and `k` from the back of an
    /// `n`-entry fan, both report exactly `n - j - k` remaining.
    #[test]
    fn size_hint_is_exact_from_both_ends(
        radixes in proptest::collection::btree_set(any::<u8>(), 0..16),
        splits in (0usize..=16, 0usize..=16),
    ) {
        let fan: Fan<()> = radixes.iter().map(|radix| (*radix, child())).collect();
        let n = fan.len();
        let (j, k) = splits;
        let (j, k) = (j.min(n), k.min(n - j.min(n)));

        let mut iter = fan.iter();
        for _ in 0..j {
            iter.next();
        }
        for _ in 0..k {
            iter.next_back();
        }
        prop_assert_eq!(iter.size_hint(), (n - j - k, Some(n - j - k)));
        prop_assert_eq!(iter.len(), n - j - k);

        let mut into_iter = fan.into_iter();
        for _ in 0..j {
            into_iter.next();
        }
        for _ in 0..k {
            into_iter.next_back();
        }
        prop_assert_eq!(into_iter.size_hint(), (n - j - k, Some(n - j - k)));
        prop_assert_eq!(into_iter.len(), n - j - k);
    }

    /// Collecting an arbitrary pair list — duplicates included, any order —
    /// builds the same fan as inserting the pairs one by one into an empty
    /// fan: later pairs displace earlier ones at the same radix.
    #[test]
    fn collect_agrees_with_sequential_insert(radixes in vec(any::<u8>(), 0..32)) {
        let pairs: Vec<(u8, Node<()>)> =
            radixes.into_iter().map(|radix| (radix, child())).collect();

        let collected: Fan<()> = pairs.iter().map(|(radix, node)| (*radix, node.clone())).collect();
        let mut sequential = Fan::new();
        for (radix, node) in pairs {
            sequential.insert(radix, node);
        }

        prop_assert_eq!(collected.len(), sequential.len());
        for ((radix, child), (expected_radix, expected)) in
            collected.iter().zip(sequential.iter())
        {
            prop_assert_eq!(radix, expected_radix);
            prop_assert!(child.ptr_eq(expected));
        }
    }

    /// Consuming a fan and collecting it back reproduces the fan exactly,
    /// entry identities included: the consuming walk hands out the very
    /// handles the fan held, in order.
    #[test]
    fn into_iter_collect_round_trips(radixes in proptest::collection::btree_set(any::<u8>(), 0..16)) {
        let fan: Fan<()> = radixes.iter().map(|radix| (*radix, child())).collect();
        let round_tripped: Fan<()> = fan.clone().into_iter().collect();

        prop_assert_eq!(round_tripped.len(), fan.len());
        for ((radix, child), (expected_radix, expected)) in
            round_tripped.iter().zip(fan.iter())
        {
            prop_assert_eq!(radix, expected_radix);
            prop_assert!(child.ptr_eq(expected));
        }
    }

    /// Building a fan by ascending `push` yields the same fan as collecting
    /// the same pairs: the O(1) append path and the general constructor
    /// agree wherever both apply.
    #[test]
    fn push_agrees_with_collect(radixes in proptest::collection::btree_set(any::<u8>(), 0..16)) {
        let pairs: Vec<(u8, Node<()>)> =
            radixes.iter().map(|radix| (*radix, child())).collect();

        let mut pushed = Fan::new();
        for (radix, node) in &pairs {
            pushed.push(*radix, node.clone());
        }
        let collected: Fan<()> = pairs.into_iter().collect();

        prop_assert_eq!(pushed.len(), collected.len());
        for ((radix, child), (expected_radix, expected)) in
            pushed.iter().zip(collected.iter())
        {
            prop_assert_eq!(radix, expected_radix);
            prop_assert!(child.ptr_eq(expected));
        }
    }
}

/// `unit` builds the one-entry fan: the entry is retrievable at its radix,
/// the length is one, and nothing else is present.
#[test]
fn unit_holds_exactly_one_child() {
    let node = child();
    let fan = Fan::unit(7, node.clone());
    assert_eq!(fan.len(), 1);
    assert!(fan.get(7).expect("unit child present").ptr_eq(&node));
    assert!(fan.get(8).is_none());
    assert_eq!(fan.iter().count(), 1);
}

/// The fan occupies 40 bytes on 64-bit targets.
///
/// An entry is 16 bytes (`(u8, Node<T>)` padded to the 8-byte handle
/// alignment), so the inline form is 8 bytes of capacity plus
/// `16 × FAN_INLINE` of storage — the packed overlay of inline and spilled
/// forms that `smallvec`'s `union` feature provides (without it a
/// discriminant adds 8 bytes). The pin keeps inline-capacity growth a
/// deliberate decision — every node allocation pays for these bytes,
/// leaves included.
#[cfg(target_pointer_width = "64")]
#[test]
fn fan_is_forty_bytes() {
    assert_eq!(std::mem::size_of::<Fan<()>>(), 40);
}
