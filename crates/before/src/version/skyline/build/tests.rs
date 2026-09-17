//! Deterministic examples of skyline construction and collapse.
//!
//! Each test feeds a hand-written leaf sequence and asserts the exact canonical
//! stream, keeping the topology and payload effects visible together.

use num_bigint::{BigInt, BigUint, Sign};

use crate::codec::{gamma, gamma::Sink, BitsBuf};

use super::SkylineBuilder;

/// A semantic payload used by the hand-written leaf sequences.
enum Payload {
    /// An absolute first-leaf height.
    Absolute(u64),
    /// A signed delta for a later leaf.
    Delta(Sign, u64),
}

impl Payload {
    /// Write this payload's canonical gamma code.
    fn write(&self, out: &mut (impl Sink + ?Sized)) {
        match self {
            Payload::Absolute(value) => gamma::encode(&BigUint::from(*value), out),
            Payload::Delta(sign, magnitude) => {
                let value = BigInt::from_biguint(*sign, BigUint::from(*magnitude));
                gamma::encode_signed(&value, out);
            }
        }
    }

    /// The encoded payload length.
    fn len(&self) -> u64 {
        let mut bits = BitsBuf::new();
        self.write(&mut bits);
        bits.len()
    }
}

/// One absolute first-leaf payload.
fn gamma(value: u64) -> Payload {
    Payload::Absolute(value)
}

/// One signed delta payload.
fn delta(sign: Sign, magnitude: u64) -> Payload {
    Payload::Delta(sign, magnitude)
}

/// Append one test payload to a builder.
fn feed(builder: &mut SkylineBuilder, depth: u64, payload: Payload) {
    builder.leaf(depth, |out| payload.write(out));
}

/// Drive a builder over `(depth, payload)` leaves and return the stream.
fn built(leaves: Vec<(u64, Payload)>) -> BitsBuf {
    let mut builder = SkylineBuilder::with_capacity(64);
    for (depth, payload) in leaves {
        feed(&mut builder, depth, payload);
    }
    builder.finish()
}

/// A stream literal from a `0`/`1` string, whitespace ignored.
fn bits(s: &str) -> BitsBuf {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            '0' => false,
            '1' => true,
            other => panic!("stream literals hold only bits: {other}"),
        })
        .collect()
}

/// A single depth-0 leaf builds the two-bit stream `1 gamma(v)`: no
/// topology derivation, no collapse.
#[test]
fn single_leaf_is_flag_plus_code() {
    assert_eq!(built(vec![(0, gamma(0))]), bits("1 1"));
}

/// Distinct sibling leaves keep their pair: `(3, 6)` at depth 1 builds `0 1
/// gamma(3) 1 zigzag(+3)` with no truncation anywhere.
#[test]
fn distinct_siblings_stay_a_pair() {
    let stream = built(vec![(1, gamma(3)), (1, delta(Sign::Plus, 3))]);
    // gamma(3) = 00100, zigzag(+3) = 6 -> gamma(6) = 00111.
    assert_eq!(stream, bits("0 1 00100 1 00111"));
}

/// Equal sibling leaves collapse to their parent, preserving the left leaf's
/// payload.
#[test]
fn equal_siblings_absorb() {
    let stream = built(vec![(1, gamma(5)), (1, delta(Sign::Plus, 0))]);
    // gamma(5) = 00110; the depth-1 pair collapsed to one depth-0 leaf.
    assert_eq!(stream, bits("1 00110"));
}

/// Four equal depth-two leaves collapse pairwise to a single root leaf.
#[test]
fn uniform_region_cascades_to_one_leaf() {
    let stream = built(vec![
        (2, gamma(7)),
        (2, delta(Sign::Plus, 0)),
        (1, delta(Sign::Plus, 0)),
    ]);
    assert_eq!(stream, bits("1 0001000"));
}

/// When a right subtree collapses to match its left sibling, their parent also
/// collapses: `(4, (4, 4))` becomes one leaf.
#[test]
fn merged_right_subtree_reanchors_over_left_leaf() {
    let stream = built(vec![
        (1, gamma(4)),
        (2, delta(Sign::Plus, 0)),
        (2, delta(Sign::Plus, 0)),
    ]);
    assert_eq!(stream, bits("1 00101"));
}

/// A zero delta across a subtree boundary is canonical and survives: in `((3,
/// 5), 5)` the right leaf equals its predecessor but its sibling is internal,
/// so nothing may collapse.
#[test]
fn zero_delta_against_internal_sibling_survives() {
    let stream = built(vec![
        (2, gamma(3)),
        (2, delta(Sign::Plus, 2)),
        (1, delta(Sign::Plus, 0)),
    ]);
    // 0 (root) 0 (left pair) 1 gamma(3) 1 zigzag(+2)=4 1 zigzag(0).
    assert_eq!(stream, bits("0 0 1 00100 1 00101 1 1"));
}

/// A deep uniform region with a wide payload still collapses to one leaf.
#[test]
fn deep_uniform_collapse_preserves_the_wide_payload() {
    const DEPTH: u64 = 8;
    const WIDE: u64 = u64::MAX >> 1;
    let mut leaves = vec![(DEPTH, gamma(WIDE)), (DEPTH, delta(Sign::Plus, 0))];
    for level in (1..DEPTH).rev() {
        leaves.push((level, delta(Sign::Plus, 0)));
    }
    assert_eq!(built(leaves), built(vec![(0, gamma(WIDE))]));
}

/// Nested equal left pairs collapse repeatedly to a single root leaf.
#[test]
fn absorb_cascade_climbs_a_left_spine() {
    let leaves = vec![
        (4, gamma(3)),
        (4, delta(Sign::Plus, 0)),
        (3, delta(Sign::Plus, 0)),
        (2, delta(Sign::Plus, 0)),
        (1, delta(Sign::Plus, 0)),
    ];
    assert_eq!(built(leaves), bits("1 00100"));
}

/// Nested equal right pairs collapse repeatedly to a single root leaf.
#[test]
fn reanchor_cascade_climbs_chained_levels() {
    let leaves = vec![
        (1, gamma(5)),
        (2, delta(Sign::Plus, 0)),
        (3, delta(Sign::Plus, 0)),
        (3, delta(Sign::Plus, 0)),
    ];
    assert_eq!(built(leaves), bits("1 00110"));
}

/// Collapse is value-driven, not shape-driven: the mixed tiling `(2, (2, 2))`
/// collapses even though the equal leaves arrive at different depths, while
/// `(2, (2, 9))` keeps its whole shape.
#[test]
fn partial_equality_collapses_only_the_equal_pair() {
    let collapsed = built(vec![
        (1, gamma(2)),
        (2, delta(Sign::Plus, 0)),
        (2, delta(Sign::Plus, 0)),
    ]);
    assert_eq!(collapsed, bits("1 011"));
    let kept = built(vec![
        (1, gamma(2)),
        (2, delta(Sign::Plus, 0)),
        (2, delta(Sign::Plus, 7)),
    ]);
    // 0 (root) 1 gamma(2) 0 (right pair) 1 zigzag(0) 1 zigzag(+7)=14.
    assert_eq!(kept, bits("0 1 011 0 1 1 1 0001111"));
}

/// One subtree's continuation range for [`SkylineBuilder::continue_verbatim`]:
/// the stream bits between the first leaf's payload code and the subtree's end,
/// re-derived by the forced flip-and-descend.
///
/// `first_depth` is the already-fed first leaf's depth; `leaves` are the
/// remaining leaves in preorder. Returns the range with the first and last
/// leaves' relative depths and the last code's length.
fn continuation(
    root_depth: u64,
    first_depth: u64,
    leaves: &[(u64, Payload)],
) -> (BitsBuf, u64, u64, u64) {
    let mut range = BitsBuf::new();
    // The within-subtree path to the previous leaf; the subtree's first leaf is
    // its leftmost, so the path starts all left branches.
    let mut path = vec![false; (first_depth - root_depth) as usize];
    for (depth, payload) in leaves {
        // Close the ancestors the previous leaf completed and flip the
        // deepest left branch, then descend, emitting one internal flag
        // per level entered (the builder's own derivation, mirrored).
        while let Some(bit) = path.pop() {
            if !bit {
                path.push(true);
                break;
            }
        }
        let rel = depth - root_depth;
        let entered = (rel - path.len() as u64) as usize;
        range.extend(std::iter::repeat_n(false, entered));
        path.extend(std::iter::repeat_n(false, entered));
        range.push(true);
        payload.write(&mut range);
    }
    let (last_depth, last_payload) = leaves.last().expect("a continuation has at least one leaf");
    (
        range,
        first_depth - root_depth,
        last_depth - root_depth,
        last_payload.len(),
    )
}

/// Splicing a subtree's continuation is stream-identical to feeding
/// its leaves one by one.
///
/// And it leaves the builder able to keep collapsing: a later zero delta
/// against the spliced subtree's internal sibling survives, exactly as under
/// per-leaf feeding.
#[test]
fn continue_verbatim_matches_per_leaf_feeding() {
    // Tiling of `((3, (5, 6)), 5)`: subtree `(5, 6)` at depth 2 arrives
    // as first-leaf feed + continuation; the final depth-1 leaf is a
    // canonical zero delta across the subtree boundary.
    let per_leaf = built(vec![
        (2, gamma(3)),
        (3, delta(Sign::Plus, 2)),
        (3, delta(Sign::Plus, 1)),
        (1, delta(Sign::Minus, 1)),
    ]);
    let mut spliced = SkylineBuilder::with_capacity(64);
    feed(&mut spliced, 2, gamma(3));
    feed(&mut spliced, 3, delta(Sign::Plus, 2));
    let (range, first_rel, last_rel, last_len) = continuation(2, 3, &[(3, delta(Sign::Plus, 1))]);
    let range_view = crate::codec::built_view(&range);
    spliced.continue_verbatim(
        range_view,
        0,
        range_view.len(),
        2,
        first_rel,
        last_rel,
        last_len,
    );
    feed(&mut spliced, 1, delta(Sign::Minus, 1));
    assert_eq!(spliced.finish(), per_leaf);
}

/// After a multi-level continuation, the next leaf closes the same ancestors
/// as leaf-by-leaf construction.
#[test]
fn continue_verbatim_reanchors_across_levels() {
    // Tiling of `((2, ((4, 7), 6)), 9)`: the depth-2 subtree's last
    // leaf sits two levels below its root.
    let leaves = vec![
        (2, gamma(2)),
        (4, delta(Sign::Plus, 2)),
        (4, delta(Sign::Plus, 3)),
        (3, delta(Sign::Minus, 1)),
        (1, delta(Sign::Plus, 3)),
    ];
    let per_leaf = built(leaves);
    let mut spliced = SkylineBuilder::with_capacity(64);
    feed(&mut spliced, 2, gamma(2));
    feed(&mut spliced, 4, delta(Sign::Plus, 2));
    let (range, first_rel, last_rel, last_len) = continuation(
        2,
        4,
        &[(4, delta(Sign::Plus, 3)), (3, delta(Sign::Minus, 1))],
    );
    let range_view = crate::codec::built_view(&range);
    spliced.continue_verbatim(
        range_view,
        0,
        range_view.len(),
        2,
        first_rel,
        last_rel,
        last_len,
    );
    feed(&mut spliced, 1, delta(Sign::Plus, 3));
    assert_eq!(spliced.finish(), per_leaf);
}

/// An equal pair after a spliced continuation collapses exactly as it does
/// under leaf-by-leaf construction.
#[test]
fn collapse_after_a_splice_matches_per_leaf_feeding() {
    // Tiling of `((3, (5, 6)), (8, 8))`: the right pair collapses to
    // one leaf whichever way the left subtree arrived.
    let per_leaf = built(vec![
        (2, gamma(3)),
        (3, delta(Sign::Plus, 2)),
        (3, delta(Sign::Plus, 1)),
        (2, delta(Sign::Plus, 2)),
        (2, delta(Sign::Plus, 0)),
    ]);
    let mut spliced = SkylineBuilder::with_capacity(64);
    feed(&mut spliced, 2, gamma(3));
    feed(&mut spliced, 3, delta(Sign::Plus, 2));
    let (range, first_rel, last_rel, last_len) = continuation(2, 3, &[(3, delta(Sign::Plus, 1))]);
    let range_view = crate::codec::built_view(&range);
    spliced.continue_verbatim(
        range_view,
        0,
        range_view.len(),
        2,
        first_rel,
        last_rel,
        last_len,
    );
    feed(&mut spliced, 2, delta(Sign::Plus, 2));
    feed(&mut spliced, 2, delta(Sign::Plus, 0));
    assert_eq!(spliced.finish(), per_leaf);
}

/// After a wide collapse selects split output, copying a canonical subtree
/// continuation remains identical to feeding each of its leaves.
#[test]
fn split_output_splices_like_leaf_feeding() {
    const WIDE: u64 = u64::MAX >> 1;
    let per_leaf = built(vec![
        (2, gamma(WIDE)),
        (3, delta(Sign::Plus, 0)),
        (3, delta(Sign::Plus, 0)),
        (2, delta(Sign::Plus, 1)),
        (3, delta(Sign::Plus, 1)),
        (3, delta(Sign::Plus, 1)),
    ]);

    let mut spliced = SkylineBuilder::with_capacity(256);
    feed(&mut spliced, 2, gamma(WIDE));
    feed(&mut spliced, 3, delta(Sign::Plus, 0));
    feed(&mut spliced, 3, delta(Sign::Plus, 0));
    feed(&mut spliced, 2, delta(Sign::Plus, 1));
    let (range, first_rel, last_rel, last_len) = continuation(
        1,
        2,
        &[(3, delta(Sign::Plus, 1)), (3, delta(Sign::Plus, 1))],
    );
    let range = crate::codec::built_view(&range);
    spliced.continue_verbatim(range, 0, range.len(), 1, first_rel, last_rel, last_len);
    assert_eq!(spliced.finish(), per_leaf);
}

/// The collapse recognition's coupling pin: the zero delta's payload code is
/// exactly `ZERO_DELTA_CODE_BITS` bits, and every nonzero delta's is wider.
///
/// The builder recognizes zero by width, so the codec and the collapse
/// predicate must continue to agree on both zero and nonzero deltas.
#[test]
fn zero_delta_has_the_lone_shortest_code() {
    assert_eq!(delta(Sign::Plus, 0).len(), super::ZERO_DELTA_CODE_BITS);
    for magnitude in 1..=64u64 {
        assert!(delta(Sign::Plus, magnitude).len() > super::ZERO_DELTA_CODE_BITS);
        assert!(delta(Sign::Minus, magnitude).len() > super::ZERO_DELTA_CODE_BITS);
    }
}
