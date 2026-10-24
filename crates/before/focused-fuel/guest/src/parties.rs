//! Probes of the party walks that resolve deep subtrees whole: `sync`'s
//! fused join-and-fork, and `without`'s splice and block paths.
//!
//! The operands are spelled bit by bit in the party encoding (two presence
//! bits per node, a terminal owned cell for `00`, a `1` marker, then zero
//! padding) and decoded through the public, validating `Party::decode`, which
//! rejects any spelling that is not canonical.

use before::{Clock, Party, Version};

use crate::measured;

/// A party's encoded tree, as bits in preorder.
#[derive(Clone)]
struct Bits(Vec<bool>);

impl Bits {
    /// Decodes the tree as a party.
    ///
    /// # Panics
    ///
    /// Panics if the tree is not a canonical party.
    fn party(&self) -> Party {
        let mut bytes = vec![0u8; (self.0.len() + 1).div_ceil(8)];
        for (index, &bit) in self.0.iter().chain([&true]).enumerate() {
            if bit {
                bytes[index / 8] |= 0x80 >> (index % 8);
            }
        }
        Party::decode(&bytes[..]).expect("the constructed party is canonical")
    }
}

/// The owned whole interval, a terminal cell.
fn full() -> Bits {
    Bits(vec![false, false])
}

/// A node over the present children.
fn node(left: Option<&Bits>, right: Option<&Bits>) -> Bits {
    let mut bits = vec![left.is_some(), right.is_some()];
    for child in [left, right].into_iter().flatten() {
        bits.extend_from_slice(&child.0);
    }
    Bits(bits)
}

/// A unary chain of `levels` nodes, each with only its left child (or only
/// its right, if `left_side` is false), over `tail`.
fn spine(levels: usize, left_side: bool, tail: &Bits) -> Bits {
    let mut bits = Vec::with_capacity(2 * levels + tail.0.len());
    for _ in 0..levels {
        bits.extend([left_side, !left_side]);
    }
    bits.extend_from_slice(&tail.0);
    Bits(bits)
}

/// The leftmost `2^-depth` cell.
fn leftmost(depth: usize) -> Bits {
    spine(depth, true, &full())
}

/// The complement of [`leftmost`]`(depth)`: the right half owned at every
/// level.
fn complement_leftmost(depth: usize) -> Bits {
    let mut bits = Vec::with_capacity(4 * depth);
    for _ in 1..depth {
        bits.extend([true, true]);
    }
    bits.extend([false, true]);
    bits.extend(full().0);
    for _ in 1..depth {
        bits.extend(full().0);
    }
    Bits(bits)
}

/// The delegated pair at `depth`: two spines sharing their path, which split
/// the deep cells below it and own opposite quarters of the right side.
fn delegated(depth: usize) -> (Bits, Bits) {
    let quarter_left = node(Some(&full()), None);
    let quarter_right = node(None, Some(&full()));
    let a = spine(
        depth,
        true,
        &node(Some(&leftmost(depth)), Some(&quarter_left)),
    );
    let b = spine(
        depth,
        true,
        &node(Some(&complement_leftmost(depth)), Some(&quarter_right)),
    );
    (a, b)
}

/// The spliced pair at `depth`: the left half, and a deep cell of the right.
fn splice(depth: usize) -> (Bits, Bits) {
    (
        node(Some(&full()), None),
        node(None, Some(&leftmost(depth))),
    )
}

/// The adjacent pair at `depth`: the leftmost `2^-depth` cell and its
/// sibling.
fn adjacent(depth: usize) -> (Bits, Bits) {
    (
        leftmost(depth),
        spine(depth - 1, true, &node(None, Some(&full()))),
    )
}

/// Measures `Clock::sync` of two clocks with empty versions whose parties
/// are `pair`, checking the two halves against `Party::join` then
/// `Party::fork` of the same pair.
fn sync((a, b): (Bits, Bits)) -> u64 {
    let mut joined = a.party();
    joined
        .join(b.party())
        .unwrap_or_else(|_| panic!("the pair is disjoint"));
    let forked = joined.fork();
    let mut left = Clock::from_parts(a.party(), Version::new());
    let mut right = Clock::from_parts(b.party(), Version::new());
    measured(
        || {
            let synced = left.sync(&mut right).is_ok();
            (synced, left, right)
        },
        |(synced, left, right)| {
            assert!(synced, "a disjoint pair syncs");
            let halves = [left.party(), right.party()];
            assert!(
                halves == [&joined, &forked] || halves == [&forked, &joined],
                "sync leaves the halves that join then fork makes"
            );
        },
    )
}

/// Measures `Party::join` then `Party::fork` of `pair`.
fn join_fork((a, b): (Bits, Bits)) -> u64 {
    let (a, b) = (a.party(), b.party());
    measured(
        || {
            let mut joined = a;
            let forked = joined.join(b).is_ok().then(|| joined.fork());
            (joined, forked)
        },
        |(_, forked)| assert!(forked.is_some(), "a disjoint pair joins"),
    )
}

/// Measures `Clock::sync` of the delegated pair at `depth`.
pub fn sync_delegated(depth: usize) -> u64 {
    sync(delegated(depth))
}

/// Measures `Clock::sync` of the spliced pair at `depth`.
pub fn sync_splice(depth: usize) -> u64 {
    sync(splice(depth))
}

/// Measures `Clock::sync` of the adjacent pair at `depth`.
pub fn sync_adjacent(depth: usize) -> u64 {
    sync(adjacent(depth))
}

/// Measures `Party::join` then `Party::fork` of the delegated pair at
/// `depth`.
pub fn join_fork_delegated(depth: usize) -> u64 {
    join_fork(delegated(depth))
}

/// Measures `Party::join` then `Party::fork` of the spliced pair at `depth`.
pub fn join_fork_splice(depth: usize) -> u64 {
    join_fork(splice(depth))
}

/// Measures `Party::join` then `Party::fork` of the adjacent pair at
/// `depth`.
pub fn join_fork_adjacent(depth: usize) -> u64 {
    join_fork(adjacent(depth))
}

/// Measures `self.without(other)`, checking the remainder against
/// `expected`.
fn without(this: &Bits, other: &Bits, expected: Option<&Bits>) -> u64 {
    let (this, other) = (this.party(), other.party());
    let expected = expected.map(Bits::party);
    measured(
        || this.without(&other),
        |remainder| assert_eq!(remainder, expected, "the constructed remainder"),
    )
}

/// The party holding the leftmost `2^-depth` cell and the right half.
fn deep_cell_and_right_half(depth: usize) -> Bits {
    node(Some(&leftmost(depth)), Some(&full()))
}

/// Measures `without` where the deep cell lies under an unowned cover and
/// survives whole, so the walk copies it as one block.
pub fn without_splice(depth: usize) -> u64 {
    let this = deep_cell_and_right_half(depth);
    let other = node(None, Some(&node(Some(&full()), None)));
    let remainder = node(Some(&leftmost(depth)), Some(&node(None, Some(&full()))));
    without(&this, &other, Some(&remainder))
}

/// Measures `without` where the deep cell lies under an owned cover and
/// vanishes whole, so the walk drops it as one block.
pub fn without_block(depth: usize) -> u64 {
    let this = deep_cell_and_right_half(depth);
    let other = node(Some(&full()), Some(&node(Some(&full()), None)));
    let remainder = node(None, Some(&node(None, Some(&full()))));
    without(&this, &other, Some(&remainder))
}

/// Measures carving the leftmost `2^-depth` cell out of the seed, which
/// builds the cell's complement level by level.
pub fn without_complement(depth: usize) -> u64 {
    without(&full(), &leftmost(depth), Some(&complement_leftmost(depth)))
}
