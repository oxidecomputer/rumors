//! Deterministic pins for the output builder's collapse genres.
//!
//! Each test feeds a hand-written leaf sequence and asserts the exact canonical
//! stream, so a bookkeeping error in absorb, re-anchor, or the cascade fails
//! against bits a reader can re-derive in the margin.

use crate::codec::{self, Base, BitsMut, Code};
use crate::version::skyline::signed::gamma_code_signed;

use super::SkylineBuilder;

/// One leaf payload code: `gamma(value)` for absolutes.
fn gamma(value: u64) -> Code {
    codec::code_int(&Base::from(value))
}

/// One leaf payload code: `gamma(zigzag(delta))` for later leaves.
fn delta(negative: bool, magnitude: u64) -> Code {
    gamma_code_signed(negative, &Base::from(magnitude))
}

/// Drive a builder over `(depth, code)` leaves and return the stream.
fn built(leaves: Vec<(usize, Code)>) -> BitsMut {
    let mut builder = SkylineBuilder::with_capacity(64);
    for (depth, code) in leaves {
        builder.leaf(depth, code);
    }
    builder.finish()
}

/// A stream literal from a `0`/`1` string, whitespace ignored.
fn bits(s: &str) -> BitsMut {
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
    let stream = built(vec![(1, gamma(3)), (1, delta(false, 3))]);
    // gamma(3) = 00100, zigzag(+3) = 6 -> gamma(6) = 00111.
    assert_eq!(stream, bits("0 1 00100 1 00111"));
}

/// A zero-delta right sibling absorbs into its held left sibling: the pair's
/// parent flag truncates and the merged leaf keeps the left code, collapsing
/// `(5, 5)` at depth 1 to the single leaf 5.
#[test]
fn equal_siblings_absorb() {
    let stream = built(vec![(1, gamma(5)), (1, delta(false, 0))]);
    // gamma(5) = 00110; the depth-1 pair collapsed to one depth-0 leaf.
    assert_eq!(stream, bits("1 00110"));
}

/// The absorb cascade climbs: four equal leaves at depth 2 collapse pairwise
/// all the way to a single depth-0 leaf, one parent-flag truncation per level,
/// with the held code never moving.
#[test]
fn uniform_region_cascades_to_one_leaf() {
    let stream = built(vec![
        (2, gamma(7)),
        (2, delta(false, 0)),
        (1, delta(false, 0)),
    ]);
    assert_eq!(stream, bits("1 0001000"));
}

/// Re-anchor: a right subtree that merges into a leaf equal to its
/// already-flushed left sibling truncates back over that sibling's code and
/// keeps it as the held code — `(4, (4, 4))` collapses to the leaf 4.
#[test]
fn merged_right_subtree_reanchors_over_left_leaf() {
    let stream = built(vec![
        (1, gamma(4)),
        (2, delta(false, 0)),
        (2, delta(false, 0)),
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
        (2, delta(false, 2)),
        (1, delta(false, 0)),
    ]);
    // 0 (root) 0 (left pair) 1 gamma(3) 1 zigzag(+2)=4 1 zigzag(0).
    assert_eq!(stream, bits("0 0 1 00100 1 00101 1 1"));
}

/// Deep uniformity around a wide code stays a single leaf: a depth-8
/// left spine of equal plateaus collapses level by level while the wide
/// held code is written exactly once.
#[test]
fn deep_uniform_collapse_holds_the_wide_code() {
    const DEPTH: usize = 8;
    const WIDE: u64 = u64::MAX >> 1;
    let mut leaves = vec![(DEPTH, gamma(WIDE)), (DEPTH, delta(false, 0))];
    for level in (1..DEPTH).rev() {
        leaves.push((level, delta(false, 0)));
    }
    assert_eq!(built(leaves), built(vec![(0, gamma(WIDE))]));
}

/// The absorb cascade climbs a left spine: the tiling of `((((3, 3), 3), 3),
/// 3)` collapses to the single leaf 3 through one parent-flag truncation per
/// level, never moving the held code.
#[test]
fn absorb_cascade_climbs_a_left_spine() {
    let leaves = vec![
        (4, gamma(3)),
        (4, delta(false, 0)),
        (3, delta(false, 0)),
        (2, delta(false, 0)),
        (1, delta(false, 0)),
    ];
    assert_eq!(built(leaves), bits("1 00100"));
}

/// Re-anchor cascades down a right spine: the uniform tiling of `(5, (5, (5,
/// 5)))` collapses through chained re-anchors — each level's flushed
/// left-sibling code is truncated back out and re-held — to the single leaf 5.
#[test]
fn reanchor_cascade_climbs_chained_levels() {
    let leaves = vec![
        (1, gamma(5)),
        (2, delta(false, 0)),
        (3, delta(false, 0)),
        (3, delta(false, 0)),
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
        (2, delta(false, 0)),
        (2, delta(false, 0)),
    ]);
    assert_eq!(collapsed, bits("1 011"));
    let kept = built(vec![
        (1, gamma(2)),
        (2, delta(false, 0)),
        (2, delta(false, 7)),
    ]);
    // 0 (root) 1 gamma(2) 0 (right pair) 1 zigzag(0) 1 zigzag(+7)=14.
    assert_eq!(kept, bits("0 1 011 0 1 1 1 0001111"));
}

/// One subtree's continuation range for [`SkylineBuilder::continue_verbatim`]:
/// the stream bits between the first leaf's payload code and the subtree's end,
/// re-derived by the forced flip-and-descend.
///
/// `first_depth` is the already-fed first leaf's depth; `leaves` are the
/// remaining leaves in preorder. Returns the range with the last leaf's
/// relative depth and code length — the coordinates the splice re-anchors the
/// builder around.
fn continuation(
    root_depth: usize,
    first_depth: usize,
    leaves: &[(usize, Code)],
) -> (BitsMut, usize, usize) {
    let mut range = BitsMut::new();
    // The within-subtree path to the previous leaf; the subtree's first leaf is
    // its leftmost, so the path starts all left branches.
    let mut path = vec![false; first_depth - root_depth];
    for (depth, code) in leaves {
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
        let entered = rel - path.len();
        range.extend(std::iter::repeat_n(false, entered));
        path.extend(std::iter::repeat_n(false, entered));
        range.push(true);
        match code {
            Code::Small { bits, len } => {
                for i in (0..*len).rev() {
                    range.push(bits >> i & 1 == 1);
                }
            }
            Code::Wide(code) => range.extend_from_bitslice(code),
        }
    }
    let (last_depth, last_code) = leaves.last().expect("a continuation has at least one leaf");
    (range, last_depth - root_depth, last_code.len())
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
        (3, delta(false, 2)),
        (3, delta(false, 1)),
        (1, delta(true, 1)),
    ]);
    let mut spliced = SkylineBuilder::with_capacity(64);
    spliced.leaf(2, gamma(3));
    spliced.leaf(3, delta(false, 2));
    let (range, last_rel, last_len) = continuation(2, 3, &[(3, delta(false, 1))]);
    spliced.continue_verbatim(&range, 2, last_rel, last_len);
    spliced.leaf(1, delta(true, 1));
    assert_eq!(spliced.finish(), per_leaf);
}

/// A spliced continuation spanning several levels re-anchors the path to the
/// subtree's rightmost leaf, so the very next leaf's close/flip bookkeeping
/// matches per-leaf feeding bit for bit.
#[test]
fn continue_verbatim_reanchors_across_levels() {
    // Tiling of `((2, ((4, 7), 6)), 9)`: the depth-2 subtree's last
    // leaf sits two levels below its root.
    let leaves = vec![
        (2, gamma(2)),
        (4, delta(false, 2)),
        (4, delta(false, 3)),
        (3, delta(true, 1)),
        (1, delta(false, 3)),
    ];
    let per_leaf = built(leaves);
    let mut spliced = SkylineBuilder::with_capacity(64);
    spliced.leaf(2, gamma(2));
    spliced.leaf(4, delta(false, 2));
    let (range, last_rel, last_len) =
        continuation(2, 4, &[(4, delta(false, 3)), (3, delta(true, 1))]);
    spliced.continue_verbatim(&range, 2, last_rel, last_len);
    spliced.leaf(1, delta(false, 3));
    assert_eq!(spliced.finish(), per_leaf);
}

/// An absorb arriving right after a spliced sibling still collapses: the held
/// last leaf of the continuation participates in the normal flush, and a later
/// equal-sibling pair merges exactly as under per-leaf feeding.
#[test]
fn collapse_after_a_splice_matches_per_leaf_feeding() {
    // Tiling of `((3, (5, 6)), (8, 8))`: the right pair collapses to
    // one leaf whichever way the left subtree arrived.
    let per_leaf = built(vec![
        (2, gamma(3)),
        (3, delta(false, 2)),
        (3, delta(false, 1)),
        (2, delta(false, 2)),
        (2, delta(false, 0)),
    ]);
    let mut spliced = SkylineBuilder::with_capacity(64);
    spliced.leaf(2, gamma(3));
    spliced.leaf(3, delta(false, 2));
    let (range, last_rel, last_len) = continuation(2, 3, &[(3, delta(false, 1))]);
    spliced.continue_verbatim(&range, 2, last_rel, last_len);
    spliced.leaf(2, delta(false, 2));
    spliced.leaf(2, delta(false, 0));
    assert_eq!(spliced.finish(), per_leaf);
}

/// The collapse recognition's coupling pin: the zero delta's payload code is
/// exactly `ZERO_DELTA_CODE_BITS` bits, and every nonzero delta's is wider.
///
/// Recognizing a zero delta by code length alone is sound only under that pair
/// of facts. The recognizer and the coder implement the arithmetic
/// independently; a different integer code (the `implementation` essay
/// contemplates ζ₂, whose zero costs two bits) would silently turn every
/// collapse check into a no-op — this pin turns that into a red test.
#[test]
fn zero_delta_has_the_lone_shortest_code() {
    assert_eq!(delta(false, 0).len(), super::ZERO_DELTA_CODE_BITS);
    for magnitude in 1..=64u64 {
        assert!(delta(false, magnitude).len() > super::ZERO_DELTA_CODE_BITS);
        assert!(delta(true, magnitude).len() > super::ZERO_DELTA_CODE_BITS);
    }
}
