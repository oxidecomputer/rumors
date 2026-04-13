//! Tests for [`Children`].

use super::*;
use ::proptest::prelude::*;
use ::proptest::proptest;
use std::collections::BTreeMap;

/// A freshly constructed `Children` reports empty at every observable level
/// and yields nothing from any iterator.
#[test]
fn empty_children_is_empty() {
    let c: Children<u32> = Children::new();
    assert!(c.is_empty());
    assert_eq!(c.len(), 0);
    for i in 0..=255u8 {
        assert!(!c.contains(i));
        assert_eq!(c.get(i), None);
    }
    assert_eq!(c.iter().count(), 0);
    assert_eq!(c.keys().count(), 0);
}

/// Insert at a previously absent index returns `None`, makes the value
/// retrievable, and updates `len`.
#[test]
fn insert_then_get() {
    let mut c = Children::new();
    assert_eq!(c.insert(7, "a"), None);
    assert_eq!(c.insert(200, "b"), None);
    assert_eq!(c.get(7), Some(&"a"));
    assert_eq!(c.get(200), Some(&"b"));
    assert_eq!(c.len(), 2);
    assert!(!c.contains(8));
}

/// Insert at an occupied index returns the previous value and replaces it
/// without changing `len`.
#[test]
fn insert_replaces() {
    let mut c = Children::new();
    c.insert(42, 1);
    assert_eq!(c.insert(42, 2), Some(1));
    assert_eq!(c.get(42), Some(&2));
    assert_eq!(c.len(), 1);
}

/// Remove at an absent index returns `None`; remove at a present index
/// returns the value and clears the slot.
#[test]
fn remove_returns_value() {
    let mut c = Children::new();
    assert_eq!(c.remove(0), None);
    c.insert(0, 99);
    assert_eq!(c.remove(0), Some(99));
    assert!(!c.contains(0));
    assert!(c.is_empty());
}

/// `iter` yields children in ascending index order regardless of insertion
/// order.
#[test]
fn iter_is_ordered() {
    let mut c = Children::new();
    for i in [200u8, 7, 0, 99, 255] {
        c.insert(i, i);
    }
    let collected: Vec<u8> = c.iter().map(|(i, _)| i).collect();
    assert_eq!(collected, vec![0, 7, 99, 200, 255]);
}

/// `iter_mut` allows in-place mutation and preserves index order.
#[test]
fn iter_mut_mutates_in_place() {
    let mut c: Children<i32> = Children::new();
    for i in [10u8, 20, 30] {
        c.insert(i, i as i32);
    }
    for (_, v) in c.iter_mut() {
        *v *= 2;
    }
    assert_eq!(c.get(10), Some(&20));
    assert_eq!(c.get(20), Some(&40));
    assert_eq!(c.get(30), Some(&60));
}

/// `IntoIterator` for an owned `Children` consumes it and yields owned values
/// in index order.
#[test]
fn into_iter_consumes() {
    let mut c: Children<String> = Children::new();
    c.insert(1, "one".into());
    c.insert(3, "three".into());
    let collected: Vec<(u8, String)> = c.into_iter().collect();
    assert_eq!(collected, vec![(1, "one".into()), (3, "three".into())]);
}

/// The `Entry` API inserts when vacant and exposes the existing slot when
/// occupied.
#[test]
fn entry_api() {
    let mut c: Children<i32> = Children::new();
    *c.entry(5).or_insert(10) += 1;
    *c.entry(5).or_insert(0) += 1;
    assert_eq!(c.get(5), Some(&12));
    assert_eq!(c.entry(6).or_insert_with(|| 42), &mut 42);
    c.entry(6).and_modify(|v| *v += 1);
    assert_eq!(c.get(6), Some(&43));
}

/// `OccupiedEntry::remove` clears the bit and returns the value.
#[test]
fn entry_remove() {
    let mut c: Children<i32> = Children::new();
    c.insert(5, 100);
    match c.entry(5) {
        Entry::Occupied(e) => assert_eq!(e.remove(), 100),
        Entry::Vacant(_) => panic!("expected occupied"),
    }
    assert!(!c.contains(5));
}

/// `keys()` enumerates exactly the bits set in `which`, including the
/// boundary keys that span every word of the bitmap.
#[test]
fn keys_spans_word_boundaries() {
    let mut c: Children<()> = Children::new();
    let probes = [0u8, 63, 64, 127, 128, 191, 192, 255];
    for i in probes {
        c.insert(i, ());
    }
    let got: Vec<u8> = c.keys().collect();
    assert_eq!(got, probes.to_vec());
}

/// `From<[(u8, T); N]>` agrees with `FromIterator`.
#[test]
fn from_array_agrees_with_from_iter() {
    let arr = [(0u8, "a"), (10, "b"), (100, "c"), (200, "d"), (255, "e")];
    let from_array: Children<&str> = Children::from(arr);
    let from_iter: Children<&str> = arr.into_iter().collect();
    assert_eq!(from_array, from_iter);
}

/// `FromIterator` from an `ExactSizeIterator` allocates a single tight
/// buffer: after `collect`, capacity should not have been doubled past the
/// known size. This verifies that `size_hint` propagates through.
#[test]
fn from_iter_exact_size_allocates_tight() {
    let arr: [(u8, i32); 7] = [(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6)];
    let c: Children<i32> = arr.into_iter().collect();
    assert_eq!(c.len(), 7);
    assert!(c.capacity() >= 7);
    assert!(
        c.capacity() < 16,
        "capacity {} suggests size_hint was ignored",
        c.capacity()
    );
}

/// All 256 possible keys can be inserted, retrieved, and removed.
#[test]
fn full_capacity() {
    let mut c: Children<u32> = Children::new();
    for i in 0..=255u8 {
        assert_eq!(c.insert(i, i as u32), None);
    }
    assert_eq!(c.len(), 256);
    for i in 0..=255u8 {
        assert_eq!(c.get(i), Some(&(i as u32)));
    }
    for i in 0..=255u8 {
        assert_eq!(c.remove(i), Some(i as u32));
    }
    assert!(c.is_empty());
}

proptest! {
    /// `Children` is observationally equivalent to a `BTreeMap<u8, T>` under
    /// arbitrary sequences of inserts and removes.
    #[test]
    fn matches_btreemap(
        ops in prop::collection::vec(
            (any::<bool>(), any::<u8>(), any::<u32>()),
            0..200,
        ),
    ) {
        let mut c: Children<u32> = Children::new();
        let mut m: BTreeMap<u8, u32> = BTreeMap::new();
        for (insert, idx, val) in ops {
            if insert {
                prop_assert_eq!(c.insert(idx, val), m.insert(idx, val));
            } else {
                prop_assert_eq!(c.remove(idx), m.remove(&idx));
            }
            prop_assert_eq!(c.len(), m.len());
            prop_assert_eq!(c.is_empty(), m.is_empty());
            prop_assert_eq!(c.contains(idx), m.contains_key(&idx));
            prop_assert_eq!(c.get(idx), m.get(&idx));
        }
        let c_pairs: Vec<(u8, u32)> = c.iter().map(|(k, v)| (k, *v)).collect();
        let m_pairs: Vec<(u8, u32)> = m.iter().map(|(&k, &v)| (k, v)).collect();
        prop_assert_eq!(c_pairs, m_pairs);
    }

    /// The internal popcount invariant: bits set in `which` always equals
    /// `what.len()` and equals `len()`. Any divergence means the bitmap and
    /// the value vector have desynced.
    #[test]
    fn popcount_invariant(c in any::<Children<u32>>()) {
        let total: u32 = c.which.iter().map(|w| w.count_ones()).sum();
        prop_assert_eq!(total as usize, c.what.len());
        prop_assert_eq!(total as usize, c.len());
    }

    /// `iter` yields entries in strictly ascending index order, with the
    /// length matching `len()`.
    #[test]
    fn iter_ascending(c in any::<Children<u32>>()) {
        let keys: Vec<u8> = c.iter().map(|(k, _)| k).collect();
        prop_assert_eq!(keys.len(), c.len());
        prop_assert!(keys.windows(2).all(|w| w[0] < w[1]));
    }

    /// `position(which, idx)` equals `popcount(which & ((1 << idx) - 1))`,
    /// matching the canonical HAMT slot computation.
    #[test]
    fn position_matches_popcount_below(c in any::<Children<u32>>(), idx in any::<u8>()) {
        let expected: u32 = (0..idx)
            .filter(|&i| {
                let (w, b) = ((i / 64) as usize, i % 64);
                c.which[w] & (1u64 << b) != 0
            })
            .count() as u32;
        prop_assert_eq!(super::bits::position(&c.which, idx), expected as usize);
    }

    /// All four traversal modes (`iter`, `&c.into_iter()`, `c.into_iter()`,
    /// `keys()`) yield the same key sequence in the same order.
    #[test]
    fn iter_modes_agree(c in any::<Children<u32>>()) {
        let from_iter: Vec<(u8, u32)> = c.iter().map(|(k, v)| (k, *v)).collect();
        let from_ref: Vec<(u8, u32)> = (&c).into_iter().map(|(k, v)| (k, *v)).collect();
        let from_keys: Vec<u8> = c.keys().collect();
        let from_owned: Vec<(u8, u32)> = c.clone().into_iter().collect();

        prop_assert_eq!(&from_iter, &from_ref);
        prop_assert_eq!(&from_iter, &from_owned);
        let iter_keys: Vec<u8> = from_iter.iter().map(|(k, _)| *k).collect();
        prop_assert_eq!(iter_keys, from_keys);
    }

    /// `iter_mut` walks the same key sequence as `iter`, so reordering between
    /// the two access modes can't happen.
    #[test]
    fn iter_mut_keys_match_iter(c in any::<Children<u32>>()) {
        let immut: Vec<u8> = c.iter().map(|(k, _)| k).collect();
        let mut c2 = c.clone();
        let mutable: Vec<u8> = c2.iter_mut().map(|(k, _)| k).collect();
        prop_assert_eq!(immut, mutable);
    }

    /// `Indices::size_hint` is exact (lower bound equals upper bound equals
    /// remaining count) at every step of iteration. This is what justifies
    /// the `ExactSizeIterator` impl.
    #[test]
    fn keys_size_hint_exact(c in any::<Children<u32>>()) {
        let mut it = c.keys();
        loop {
            let (low, high) = it.size_hint();
            prop_assert_eq!(Some(low), high);
            if it.next().is_none() {
                prop_assert_eq!(low, 0);
                break;
            }
        }
    }

    /// `Iter::size_hint` is exact at every step and matches the remaining
    /// element count.
    #[test]
    fn iter_size_hint_exact(c in any::<Children<u32>>()) {
        let mut it = c.iter();
        let mut remaining = c.len();
        loop {
            let (low, high) = it.size_hint();
            prop_assert_eq!(low, remaining);
            prop_assert_eq!(high, Some(remaining));
            if it.next().is_none() {
                prop_assert_eq!(remaining, 0);
                break;
            }
            remaining -= 1;
        }
    }

    /// `Clone` produces a value equal to the original.
    #[test]
    fn clone_eq_original(c in any::<Children<u32>>()) {
        prop_assert_eq!(c.clone(), c);
    }

    /// `Default::default()` equals `Children::new()`.
    #[test]
    fn default_eq_new(_unit in Just(())) {
        prop_assert_eq!(<Children<u32> as Default>::default(), Children::<u32>::new());
    }

    /// Building a `Children` by inserting the same `{idx -> value}` mapping
    /// in two different orderings (ascending and descending by index) yields
    /// equal results. The popcount-indexed layout cannot be order-sensitive.
    #[test]
    fn insertion_order_independent(
        pairs in prop::collection::btree_map(any::<u8>(), any::<u32>(), 0..200),
    ) {
        let mut a = Children::new();
        for (k, v) in &pairs {
            a.insert(*k, *v);
        }
        let mut b = Children::new();
        for (k, v) in pairs.iter().rev() {
            b.insert(*k, *v);
        }
        prop_assert_eq!(a, b);
    }

    /// After `insert(idx, val)`, `get(idx)` returns `Some(&val)` and
    /// `contains(idx)` is true, regardless of the prior state at `idx`.
    #[test]
    fn insert_then_get_returns_value(
        mut c in any::<Children<u32>>(),
        idx in any::<u8>(),
        val in any::<u32>(),
    ) {
        c.insert(idx, val);
        prop_assert_eq!(c.get(idx), Some(&val));
        prop_assert!(c.contains(idx));
    }

    /// After `remove(idx)`, the slot is absent regardless of prior state.
    #[test]
    fn remove_makes_absent(
        mut c in any::<Children<u32>>(),
        idx in any::<u8>(),
    ) {
        c.remove(idx);
        prop_assert!(!c.contains(idx));
        prop_assert_eq!(c.get(idx), None);
    }

    /// `position` is monotonically non-decreasing in `idx`. This guards
    /// against off-by-one or word-boundary mistakes that would let a higher
    /// index map to a lower slot.
    #[test]
    fn position_monotonic(
        c in any::<Children<u32>>(),
        a in any::<u8>(),
        b in any::<u8>(),
    ) {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        prop_assert!(
            super::bits::position(&c.which, lo)
                <= super::bits::position(&c.which, hi)
        );
    }

    /// For every set bit, the value stored at the popcount-derived position
    /// in `what` is exactly the value `get` returns. This is the structural
    /// invariant that links the bitmap to the value vector.
    #[test]
    fn what_position_matches_get(c in any::<Children<u32>>()) {
        for idx in c.keys() {
            let pos = super::bits::position(&c.which, idx);
            prop_assert_eq!(c.get(idx), Some(&c.what[pos]));
        }
    }

    /// `entry(idx).or_insert(v)` populates `idx` if vacant, and returns the
    /// existing value (without overwriting) if occupied.
    #[test]
    fn entry_or_insert_preserves_existing(
        mut c in any::<Children<u32>>(),
        idx in any::<u8>(),
        v in any::<u32>(),
    ) {
        let prior = c.get(idx).copied();
        let returned = *c.entry(idx).or_insert(v);
        match prior {
            Some(p) => prop_assert_eq!(returned, p),
            None => prop_assert_eq!(returned, v),
        }
        prop_assert!(c.contains(idx));
    }

    /// `and_modify` applies the closure exactly when the slot is occupied
    /// and never inserts.
    #[test]
    fn and_modify_only_when_occupied(
        mut c in any::<Children<u32>>(),
        idx in any::<u8>(),
    ) {
        let was_present = c.contains(idx);
        let prior = c.get(idx).copied();
        c.entry(idx).and_modify(|v| *v = v.wrapping_add(1));
        if was_present {
            prop_assert_eq!(c.get(idx).copied(), prior.map(|p| p.wrapping_add(1)));
        } else {
            prop_assert!(!c.contains(idx));
        }
    }

    /// `insert` followed by `remove` of the same index, with the prior value
    /// restored if any, returns `Children` to its original state. This
    /// exercises both branches of insert (replace vs. add) and remove against
    /// arbitrary baseline state.
    #[test]
    fn insert_remove_round_trip(
        c in any::<Children<u32>>(),
        idx in any::<u8>(),
        v in any::<u32>(),
    ) {
        let before = c.clone();
        let mut after = c;
        let prior = after.insert(idx, v);
        let removed = after.remove(idx);
        prop_assert_eq!(removed, Some(v));
        if let Some(p) = prior {
            after.insert(idx, p);
        }
        prop_assert_eq!(before, after);
    }

    /// `keys().rev()` yields keys in strictly descending order with
    /// length matching `len()`.
    #[test]
    fn rev_keys_descending(c in any::<Children<u32>>()) {
        let keys: Vec<u8> = c.keys().rev().collect();
        prop_assert_eq!(keys.len(), c.len());
        prop_assert!(keys.windows(2).all(|w| w[0] > w[1]));
    }

    /// Reverse iteration is the exact reverse of forward iteration, across
    /// all four iterator types.
    #[test]
    fn reverse_equals_collected_then_reversed(c in any::<Children<u32>>()) {
        let forward: Vec<(u8, u32)> = c.iter().map(|(k, v)| (k, *v)).collect();
        let mut forward_reversed = forward.clone();
        forward_reversed.reverse();

        let from_iter_rev: Vec<(u8, u32)> = c.iter().rev().map(|(k, v)| (k, *v)).collect();
        prop_assert_eq!(&from_iter_rev, &forward_reversed);

        let from_keys_rev: Vec<u8> = c.keys().rev().collect();
        let forward_keys_rev: Vec<u8> =
            forward_reversed.iter().map(|(k, _)| *k).collect();
        prop_assert_eq!(from_keys_rev, forward_keys_rev);

        let mut c2 = c.clone();
        let from_iter_mut_rev: Vec<(u8, u32)> =
            c2.iter_mut().rev().map(|(k, v)| (k, *v)).collect();
        prop_assert_eq!(&from_iter_mut_rev, &forward_reversed);

        let from_owned_rev: Vec<(u8, u32)> = c.into_iter().rev().collect();
        prop_assert_eq!(from_owned_rev, forward_reversed);
    }

    /// Alternating `next` and `next_back` consumes each element exactly
    /// once, partitioning the forward sequence into a front half and a
    /// reversed back half. This is the contract of [`DoubleEndedIterator`].
    #[test]
    fn mixed_ends_partition_full_set(c in any::<Children<u32>>()) {
        let expected: Vec<(u8, u32)> = c.iter().map(|(k, v)| (k, *v)).collect();

        let mut it = c.iter();
        let mut from_front: Vec<(u8, u32)> = Vec::new();
        let mut from_back: Vec<(u8, u32)> = Vec::new();
        let mut take_front = true;
        loop {
            let next = if take_front { it.next() } else { it.next_back() };
            match next {
                Some((k, v)) => {
                    if take_front {
                        from_front.push((k, *v));
                    } else {
                        from_back.push((k, *v));
                    }
                    take_front = !take_front;
                }
                None => break,
            }
        }
        from_back.reverse();
        from_front.extend(from_back);
        prop_assert_eq!(from_front, expected);
    }

    /// After mixed-end iteration drains an `Iter`, both `next` and
    /// `next_back` continue to return `None` (fused at both ends).
    #[test]
    fn drained_iter_is_fused_both_ends(c in any::<Children<u32>>()) {
        let mut it = c.iter();
        // Drain by alternating ends.
        let mut take_front = true;
        loop {
            let n = if take_front { it.next() } else { it.next_back() };
            if n.is_none() {
                break;
            }
            take_front = !take_front;
        }
        prop_assert!(it.next().is_none());
        prop_assert!(it.next_back().is_none());
        prop_assert!(it.next().is_none());
    }

    /// `FromIterator` produces the same result as inserting each pair
    /// individually, with last-write-wins on duplicate keys.
    #[test]
    fn from_iter_matches_inserts(
        pairs in prop::collection::vec((any::<u8>(), any::<u32>()), 0..200),
    ) {
        let collected: Children<u32> = pairs.iter().copied().collect();
        let mut built = Children::new();
        for (k, v) in &pairs {
            built.insert(*k, *v);
        }
        prop_assert_eq!(collected, built);
    }

    /// `Extend` is equivalent to inserting each pair on top of the existing
    /// `Children`.
    #[test]
    fn extend_matches_inserts(
        base in any::<Children<u32>>(),
        more in prop::collection::vec((any::<u8>(), any::<u32>()), 0..200),
    ) {
        let mut a = base.clone();
        a.extend(more.iter().copied());
        let mut b = base;
        for (k, v) in &more {
            b.insert(*k, *v);
        }
        prop_assert_eq!(a, b);
    }

    /// `first` returns the lowest-indexed entry; equivalent to taking the
    /// first item of `iter`.
    #[test]
    fn first_matches_iter(c in any::<Children<u32>>()) {
        let from_iter = c.iter().next().map(|(k, v)| (k, *v));
        let from_first = c.first().map(|(k, v)| (k, *v));
        prop_assert_eq!(from_first, from_iter);
    }

    /// `last` returns the highest-indexed entry; equivalent to taking the
    /// last item of `iter`.
    #[test]
    fn last_matches_iter(c in any::<Children<u32>>()) {
        let from_iter = c.iter().next_back().map(|(k, v)| (k, *v));
        let from_last = c.last().map(|(k, v)| (k, *v));
        prop_assert_eq!(from_last, from_iter);
    }

    /// `clear` empties `self` and preserves capacity.
    #[test]
    fn clear_empties_preserves_capacity(c in any::<Children<u32>>()) {
        let cap_before = c.capacity();
        let mut c = c;
        c.clear();
        prop_assert!(c.is_empty());
        prop_assert_eq!(c.len(), 0);
        prop_assert_eq!(c.keys().count(), 0);
        prop_assert_eq!(c.capacity(), cap_before);
    }

    /// `drain` yields exactly the entries that were present, in ascending
    /// order, and leaves `self` empty.
    #[test]
    fn drain_yields_all_then_empties(c in any::<Children<u32>>()) {
        let expected: Vec<(u8, u32)> = c.iter().map(|(k, v)| (k, *v)).collect();
        let mut c = c;
        let drained: Vec<(u8, u32)> = c.drain().collect();
        prop_assert_eq!(drained, expected);
        prop_assert!(c.is_empty());
    }

    /// A partially consumed `Drain`, when dropped, still leaves `self` empty
    /// (the remaining elements are dropped, not leaked back into `Children`).
    #[test]
    fn partial_drain_still_empties(c in any::<Children<u32>>()) {
        let mut c = c;
        {
            let mut it = c.drain();
            // Consume just one if available.
            let _ = it.next();
        }
        prop_assert!(c.is_empty());
    }

    /// `retain` keeps exactly the entries for which the predicate returns
    /// true, equivalent to filtering via the public iterator API.
    #[test]
    fn retain_matches_filter(
        c in any::<Children<u32>>(),
        threshold in any::<u32>(),
    ) {
        let mut a = c.clone();
        a.retain(|_, v| *v >= threshold);

        let mut b = Children::new();
        for (k, v) in c.iter() {
            if *v >= threshold {
                b.insert(k, *v);
            }
        }
        prop_assert_eq!(a, b);
    }

    /// `values`, `values_mut`, and `into_values` all yield values in the
    /// same ascending-index order as `iter`.
    #[test]
    fn values_match_iter(c in any::<Children<u32>>()) {
        let from_iter: Vec<u32> = c.iter().map(|(_, v)| *v).collect();
        let from_values: Vec<u32> = c.values().copied().collect();
        let from_owned: Vec<u32> = c.clone().into_values().collect();

        prop_assert_eq!(&from_iter, &from_values);
        prop_assert_eq!(&from_iter, &from_owned);

        let mut c2 = c;
        let from_values_mut: Vec<u32> = c2.values_mut().map(|v| *v).collect();
        prop_assert_eq!(from_iter, from_values_mut);
    }

    /// After any sequence of operations, the inner `Vec` capacity is bounded
    /// above by `MAX_CHILDREN`, and after a remove that crosses the shrink
    /// threshold (`len * 4 <= cap`), capacity is at most the previous value
    /// (never grows on remove).
    #[test]
    fn capacity_never_grows_on_remove(
        c in any::<Children<u32>>(),
        idx in any::<u8>(),
    ) {
        let mut c = c;
        let before = c.capacity();
        c.remove(idx);
        prop_assert!(c.capacity() <= before);
    }

    /// `shrink_to_fit` brings capacity down to no more than `len` plus
    /// whatever slack the allocator insists on, and never below `len`.
    #[test]
    fn shrink_to_fit_bounds(c in any::<Children<u32>>()) {
        let mut c = c;
        let len = c.len();
        c.shrink_to_fit();
        prop_assert!(c.capacity() >= len);
    }

    /// The auto-shrink hysteresis hint is invariant: after `maybe_shrink` is
    /// called (via `remove`), repeated removes at the same capacity won't
    /// re-attempt. We verify the indirect contract: after a remove that
    /// triggers shrink, the recorded `last_shrink_capacity` equals the
    /// current capacity, so subsequent same-capacity removes are no-ops.
    #[test]
    fn shrink_hint_matches_capacity_after_trigger(
        c in any::<Children<u32>>(),
        idx in any::<u8>(),
    ) {
        let mut c = c;
        let had = c.contains(idx);
        c.remove(idx);
        if had {
            // Either the trigger condition wasn't met (cap > 4*len) or the
            // hint now equals the current capacity.
            let cap = c.capacity();
            let len = c.len();
            let triggered = len.saturating_mul(4) <= cap;
            if triggered {
                prop_assert_eq!(c.last_shrink_capacity, cap);
            }
        }
    }

    /// Capacity is always one of {0, 4, 8, 16, 32, 64, 128, 256} after any
    /// sequence of inserts (allocator may bump slightly, but we never grow
    /// past 256).
    #[test]
    fn capacity_bounded_by_max_children(c in any::<Children<u32>>()) {
        prop_assert!(c.capacity() <= 256);
    }

    /// `pop_first` removes and returns the lowest entry, equivalent to
    /// `first()` followed by `remove(idx)`.
    #[test]
    fn pop_first_matches_first_then_remove(c in any::<Children<u32>>()) {
        let mut a = c.clone();
        let from_pop = a.pop_first();

        let mut b = c;
        let from_first = b.first().map(|(k, v)| (k, *v));
        if let Some((idx, _)) = from_first {
            b.remove(idx);
        }

        prop_assert_eq!(from_pop, from_first);
        prop_assert_eq!(a, b);
    }

    /// `pop_last` removes and returns the highest entry, equivalent to
    /// `last()` followed by `remove(idx)`.
    #[test]
    fn pop_last_matches_last_then_remove(c in any::<Children<u32>>()) {
        let mut a = c.clone();
        let from_pop = a.pop_last();

        let mut b = c;
        let from_last = b.last().map(|(k, v)| (k, *v));
        if let Some((idx, _)) = from_last {
            b.remove(idx);
        }

        prop_assert_eq!(from_pop, from_last);
        prop_assert_eq!(a, b);
    }

    /// Repeated `pop_first` drains in ascending order; `pop_last` in
    /// descending order. Together they reach every entry exactly once.
    #[test]
    fn pop_first_drains_ascending(c in any::<Children<u32>>()) {
        let expected: Vec<(u8, u32)> = c.iter().map(|(k, v)| (k, *v)).collect();
        let mut c = c;
        let mut popped = Vec::new();
        while let Some(p) = c.pop_first() {
            popped.push(p);
        }
        prop_assert_eq!(popped, expected);
        prop_assert!(c.is_empty());
    }

    #[test]
    fn pop_last_drains_descending(c in any::<Children<u32>>()) {
        let mut expected: Vec<(u8, u32)> = c.iter().map(|(k, v)| (k, *v)).collect();
        expected.reverse();
        let mut c = c;
        let mut popped = Vec::new();
        while let Some(p) = c.pop_last() {
            popped.push(p);
        }
        prop_assert_eq!(popped, expected);
        prop_assert!(c.is_empty());
    }

    /// `range(..)` (full bounds) yields the same sequence as `iter`.
    #[test]
    fn range_full_matches_iter(c in any::<Children<u32>>()) {
        let from_iter: Vec<(u8, u32)> = c.iter().map(|(k, v)| (k, *v)).collect();
        let from_range: Vec<(u8, u32)> = c.range(..).map(|(k, v)| (k, *v)).collect();
        prop_assert_eq!(from_range, from_iter);
    }

    /// `range(start..end)` yields exactly the entries with `start <= idx < end`,
    /// equivalent to filtering `iter` by the range condition.
    #[test]
    fn range_matches_filter(
        c in any::<Children<u32>>(),
        start in any::<u8>(),
        end in any::<u8>(),
    ) {
        let (lo, hi) = if start <= end { (start, end) } else { (end, start) };
        let from_range: Vec<(u8, u32)> = c.range(lo..hi).map(|(k, v)| (k, *v)).collect();
        let from_filter: Vec<(u8, u32)> = c
            .iter()
            .filter(|(k, _)| (lo..hi).contains(k))
            .map(|(k, v)| (k, *v))
            .collect();
        prop_assert_eq!(from_range, from_filter);
    }

    /// `range(start..=end)` includes `end`. Same equivalence as above with
    /// inclusive upper bound.
    #[test]
    fn range_inclusive_matches_filter(
        c in any::<Children<u32>>(),
        start in any::<u8>(),
        end in any::<u8>(),
    ) {
        let (lo, hi) = if start <= end { (start, end) } else { (end, start) };
        let from_range: Vec<(u8, u32)> = c.range(lo..=hi).map(|(k, v)| (k, *v)).collect();
        let from_filter: Vec<(u8, u32)> = c
            .iter()
            .filter(|(k, _)| (lo..=hi).contains(k))
            .map(|(k, v)| (k, *v))
            .collect();
        prop_assert_eq!(from_range, from_filter);
    }

    /// `range_mut` mutates exactly the entries in the range and leaves
    /// others untouched.
    #[test]
    fn range_mut_only_affects_range(
        c in any::<Children<u32>>(),
        start in any::<u8>(),
        end in any::<u8>(),
    ) {
        let (lo, hi) = if start <= end { (start, end) } else { (end, start) };
        let mut c2 = c.clone();
        for (_, v) in c2.range_mut(lo..hi) {
            *v = v.wrapping_add(1);
        }
        for idx in 0u8..=255 {
            let original = c.get(idx).copied();
            let mutated = c2.get(idx).copied();
            if (lo..hi).contains(&idx) {
                prop_assert_eq!(mutated, original.map(|v| v.wrapping_add(1)));
            } else {
                prop_assert_eq!(mutated, original);
            }
        }
    }

    /// `union` produces exactly `BTreeMap::extend`-equivalent results, with
    /// the combiner applied to keys present in both.
    #[test]
    fn union_matches_btreemap(
        a in any::<Children<u32>>(),
        b in any::<Children<u32>>(),
    ) {
        let union = a.clone().union(b.clone(), |_, l, r| l.wrapping_add(r));
        let mut expected: BTreeMap<u8, u32> = BTreeMap::new();
        for (k, v) in a.iter() {
            expected.insert(k, *v);
        }
        for (k, v) in b.iter() {
            expected
                .entry(k)
                .and_modify(|e| *e = e.wrapping_add(*v))
                .or_insert(*v);
        }
        let got: BTreeMap<u8, u32> = union.into_iter().collect();
        prop_assert_eq!(got, expected);
    }

    /// `intersection` produces exactly the keys present in both, with values
    /// combined.
    #[test]
    fn intersection_matches_btreemap(
        a in any::<Children<u32>>(),
        b in any::<Children<u32>>(),
    ) {
        let inter = a.clone().intersection(b.clone(), |_, l, r| l.wrapping_add(r));
        let mut expected: BTreeMap<u8, u32> = BTreeMap::new();
        for (k, va) in a.iter() {
            if let Some(vb) = b.get(k) {
                expected.insert(k, va.wrapping_add(*vb));
            }
        }
        let got: BTreeMap<u8, u32> = inter.into_iter().collect();
        prop_assert_eq!(got, expected);
    }

    /// `difference` yields exactly the keys in `a` that are not in `b`,
    /// preserving values from `a`.
    #[test]
    fn difference_matches_filter(
        a in any::<Children<u32>>(),
        b in any::<Children<u32>>(),
    ) {
        let diff = a.clone().difference(&b);
        let mut expected: BTreeMap<u8, u32> = BTreeMap::new();
        for (k, v) in a.iter() {
            if !b.contains(k) {
                expected.insert(k, *v);
            }
        }
        let got: BTreeMap<u8, u32> = diff.into_iter().collect();
        prop_assert_eq!(got, expected);
    }

    /// `symmetric_difference` yields exactly the keys present in one but not
    /// both, preserving values from whichever side has them.
    #[test]
    fn symmetric_difference_matches_xor(
        a in any::<Children<u32>>(),
        b in any::<Children<u32>>(),
    ) {
        let xor = a.clone().symmetric_difference(b.clone());
        let mut expected: BTreeMap<u8, u32> = BTreeMap::new();
        for (k, v) in a.iter() {
            if !b.contains(k) {
                expected.insert(k, *v);
            }
        }
        for (k, v) in b.iter() {
            if !a.contains(k) {
                expected.insert(k, *v);
            }
        }
        let got: BTreeMap<u8, u32> = xor.into_iter().collect();
        prop_assert_eq!(got, expected);
    }

    /// Set operations preserve the popcount invariant on their results.
    #[test]
    fn set_ops_preserve_invariant(
        a in any::<Children<u32>>(),
        b in any::<Children<u32>>(),
    ) {
        let union = a.clone().union(b.clone(), |_, l, _| l);
        prop_assert_eq!(
            union.which.iter().map(|w| w.count_ones() as usize).sum::<usize>(),
            union.what.len()
        );

        let inter = a.clone().intersection(b.clone(), |_, l, _| l);
        prop_assert_eq!(
            inter.which.iter().map(|w| w.count_ones() as usize).sum::<usize>(),
            inter.what.len()
        );

        let diff = a.clone().difference(&b);
        prop_assert_eq!(
            diff.which.iter().map(|w| w.count_ones() as usize).sum::<usize>(),
            diff.what.len()
        );

        let sym = a.symmetric_difference(b);
        prop_assert_eq!(
            sym.which.iter().map(|w| w.count_ones() as usize).sum::<usize>(),
            sym.what.len()
        );
    }

    /// `split_off(at)` partitions `self` into entries with `idx < at`
    /// (retained) and entries with `idx >= at` (returned). The two halves
    /// together reconstruct the original.
    #[test]
    fn split_off_partitions(
        c in any::<Children<u32>>(),
        at in any::<u8>(),
    ) {
        let original = c.clone();
        let mut left = c;
        let right = left.split_off(at);

        // Left half: all keys strictly less than `at`.
        for (k, _) in left.iter() {
            prop_assert!(k < at);
        }
        // Right half: all keys >= `at`.
        for (k, _) in right.iter() {
            prop_assert!(k >= at);
        }
        // Together they account for every entry, with values preserved.
        prop_assert_eq!(left.len() + right.len(), original.len());
        for (k, v) in original.iter() {
            if k < at {
                prop_assert_eq!(left.get(k), Some(v));
            } else {
                prop_assert_eq!(right.get(k), Some(v));
            }
        }
    }

    /// Splitting at 0 moves everything to the right half; splitting at every
    /// possible higher key would move everything left, except `at` here is
    /// `u8` so we use the 0-case as the boundary.
    #[test]
    fn split_off_at_zero_moves_all_right(c in any::<Children<u32>>()) {
        let original = c.clone();
        let mut left = c;
        let right = left.split_off(0);
        prop_assert!(left.is_empty());
        prop_assert_eq!(right, original);
    }

    /// `contains_key` matches `contains`; `get_key_value` and `remove_entry`
    /// match `get` and `remove` with the index attached.
    #[test]
    fn naming_aliases_match_originals(
        c in any::<Children<u32>>(),
        idx in any::<u8>(),
    ) {
        prop_assert_eq!(c.contains_key(idx), c.contains(idx));
        prop_assert_eq!(c.get_key_value(idx), c.get(idx).map(|v| (idx, v)));

        let mut a = c.clone();
        let mut b = c;
        let from_alias = a.remove_entry(idx);
        let from_remove = b.remove(idx).map(|v| (idx, v));
        prop_assert_eq!(from_alias, from_remove);
        prop_assert_eq!(a, b);
    }

    /// `append(&mut other)` produces the same final state as inserting every
    /// pair from `other` into `self`, with last-write-wins on collisions.
    /// `other` is left empty.
    #[test]
    fn append_matches_extend(
        a in any::<Children<u32>>(),
        b in any::<Children<u32>>(),
    ) {
        let mut left = a.clone();
        let mut right = b.clone();
        left.append(&mut right);

        let mut expected = a;
        for (k, v) in b.iter() {
            expected.insert(k, *v);
        }
        prop_assert_eq!(left, expected);
        prop_assert!(right.is_empty());
    }

    /// `first_entry` and `last_entry` return entries equivalent to
    /// `entry(idx)` for the lowest/highest key.
    #[test]
    fn first_last_entry_match_entry(c in any::<Children<u32>>()) {
        let first_idx = c.first().map(|(k, _)| k);
        let mut a = c.clone();
        let from_first_entry = a.first_entry().map(|e| e.key());
        prop_assert_eq!(from_first_entry, first_idx);

        let last_idx = c.last().map(|(k, _)| k);
        let mut a = c;
        let from_last_entry = a.last_entry().map(|e| e.key());
        prop_assert_eq!(from_last_entry, last_idx);
    }

    /// `Hash` is consistent with `PartialEq`: equal `Children` produce equal
    /// hashes. We don't check the converse (different inputs may collide).
    #[test]
    fn hash_consistent_with_eq(c in any::<Children<u32>>()) {
        use std::hash::{BuildHasher, Hasher};
        let s = std::collections::hash_map::RandomState::new();
        let mut h1 = s.build_hasher();
        std::hash::Hash::hash(&c, &mut h1);
        let mut h2 = s.build_hasher();
        std::hash::Hash::hash(&c.clone(), &mut h2);
        prop_assert_eq!(h1.finish(), h2.finish());
    }

    /// `Index<u8>` returns the same value as `get` for present keys.
    #[test]
    fn index_matches_get_when_present(c in any::<Children<u32>>()) {
        for (k, v) in c.iter() {
            prop_assert_eq!(&c[k], v);
        }
    }
}
