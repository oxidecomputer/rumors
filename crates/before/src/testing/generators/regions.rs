//! Parties drawn as unions of dyadic intervals, deep and fragmented, and
//! families of pairwise-disjoint parties.
//!
//! [`arb_oracle_party_nonempty`](super::arb_oracle_party_nonempty) recurses at
//! most four levels, and [`arb_party_family`](super::arb_party_family) draws
//! its parties from a pool of at most four with repetition. A repeated party
//! overlaps itself, so a family it draws that `join_all` accepts holds at
//! most four parties, receiver included. The strategies here reach deep
//! parties and accepted families of up to ten unrelated parties.
//!
//! A party is drawn as a set of *pieces*: the intervals between sorted
//! dyadic *cut points*, each piece owned or not. Some cut points are
//! scattered at depths weighted toward the shallow end, and some cluster
//! around one base point at many depths, so the canonical tree nests many
//! two-child branches along one path. A party holds at most
//! [`MAX_INTERIOR_CUTS`] cut points inside the unit interval. Every union of
//! dyadic intervals with as many interior endpoints as the scattered arm
//! draws ([`SCATTERED_POINTS`]), each at most [`DEEP_PARTY_DEPTH`] levels
//! deep, is reachable; more endpoints are reachable only around one
//! cluster's base point.
//!
//! | Strategy | What it draws |
//! |---|---|
//! | [`arb_deep_party`] | one nonempty party, to depth [`DEEP_PARTY_DEPTH`] |
//! | [`arb_disjoint_parties`] | `k` pairwise-disjoint nonempty parties, the pieces of one cut set dealt among them |
//! | [`arb_deep_party_family`] | a receiver and items, 2 to [`FAMILY_ARITY_MAX`] parties in all: pairwise disjoint, or with one overlapping party inserted |
//! | [`arb_deep_clock_family`] | the same family, each party paired with a deep version |
//!
//! Nothing is rejected: each party closes through the recursive oracle's
//! normalizing constructors. The census in this module's sibling tests floors
//! the depth and the accepted-family regimes.

use core::ops::Range;

use proptest::prelude::*;
use proptest::sample::Index;

use crate::testing::oracles::tree;
use crate::testing::shape_rows::party_from_rows;

use super::leaf_list::{arb_deep_version, LeafList};

/// The exponent of the cut-point grid: a cut point is a numerator over
/// `2^POINT_BITS`, held in a `u128`.
///
/// It leaves room below [`DEEP_PARTY_DEPTH`] for the bisections
/// [`arb_disjoint_parties`] makes to reach `k` pieces.
const POINT_BITS: u32 = 126;

/// The point `1`, the end of the unit interval, on the cut-point grid.
const ONE: u128 = 1 << POINT_BITS;

/// The deepest cut point the strategies place.
pub(crate) const DEEP_PARTY_DEPTH: u32 = 80;

/// The largest family [`arb_deep_party_family`] draws, receiver included.
pub(crate) const FAMILY_ARITY_MAX: usize = 10;

/// Builds the party owning exactly the pieces `owned` selects, iteratively.
///
/// Piece `i` is `[cuts[i], cuts[i + 1])`. A cell of the dyadic tree that
/// holds no cut point strictly inside it lies within one piece and becomes
/// a row; any other cell splits in two. [`party_from_rows`] closes the rows
/// through the oracle's normalizing [`tree::Party::node`], so the result is
/// in normal form.
fn party_of_pieces(cuts: &[u128], owned: impl Fn(usize) -> bool) -> tree::Party {
    let mut rows = Vec::new();
    let mut cells = vec![(0u128, 0u32)];
    while let Some((start, depth)) = cells.pop() {
        let end = start + (ONE >> depth);
        // The first cut point past the cell's start.
        let next = cuts.partition_point(|&cut| cut <= start);
        if cuts[next] < end {
            let middle = start + (ONE >> (depth + 1));
            cells.push((middle, depth + 1));
            cells.push((start, depth + 1));
        } else {
            rows.push((owned(next - 1), u64::from(depth)));
        }
    }
    party_from_rows(&rows)
}

/// Draws a cut point at most `max_depth` levels deep, at depths weighted
/// toward the shallow end.
fn arb_point(max_depth: u32) -> impl Strategy<Value = u128> {
    let depth = prop_oneof![
        4 => 1..=max_depth.min(4),
        4 => 1..=max_depth.min(10),
        2 => 1..=max_depth.min(24),
        1 => 1..=max_depth,
    ];
    (depth, any::<u128>()).prop_map(|(depth, bits)| (bits >> (128 - depth)) << (POINT_BITS - depth))
}

/// Draws cut points clustered around one base point: for each of several
/// depths, the boundary of the base point's cell at that depth, below or
/// above it.
fn arb_cluster(max_depth: u32) -> impl Strategy<Value = Vec<u128>> {
    (
        any::<u128>(),
        proptest::collection::vec((1..=max_depth, any::<bool>()), CLUSTER_LEVELS),
    )
        .prop_map(|(bits, levels)| {
            let base = bits >> (128 - POINT_BITS);
            levels
                .into_iter()
                .map(|(depth, above)| {
                    let cell = ONE >> depth;
                    let below = base / cell * cell;
                    if above {
                        (below + cell).min(ONE)
                    } else {
                        below
                    }
                })
                .collect()
        })
}

/// Cut points the scattered arm of [`arb_cuts`] draws.
const SCATTERED_POINTS: Range<usize> = 0..10;

/// Cut points the scattered part of [`arb_cuts`]' mixed arm draws.
const SCATTERED_BESIDE_CLUSTER: Range<usize> = 0..4;

/// Levels, and so cut points, [`arb_cluster`] draws.
const CLUSTER_LEVELS: Range<usize> = 1..12;

/// The most cut points [`arb_cuts`] places strictly inside the unit
/// interval: the most any of its arms draws.
const MAX_INTERIOR_CUTS: usize = larger(
    SCATTERED_POINTS.end - 1,
    larger(
        CLUSTER_LEVELS.end - 1,
        (SCATTERED_BESIDE_CLUSTER.end - 1) + (CLUSTER_LEVELS.end - 1),
    ),
);

/// The most pieces a cut set from [`arb_cuts`] divides the unit interval
/// into.
const MAX_PIECES: usize = MAX_INTERIOR_CUTS + 1;

// Each piece of a cut set takes its own bit of the ownership mask that
// `arb_deep_party_within` draws.
const _: () = assert!(
    MAX_PIECES <= u64::BITS as usize,
    "a cut set can have more pieces than the ownership mask has bits"
);

/// Returns the larger of two counts, in a constant context.
const fn larger(a: usize, b: usize) -> usize {
    if a > b {
        a
    } else {
        b
    }
}

/// Draws sorted distinct cut points at most `max_depth` levels deep,
/// always including 0 and 1: scattered, clustered, or both.
///
/// Between 0 and 1 lie at most [`MAX_INTERIOR_CUTS`] points, so the cut set
/// has at most [`MAX_PIECES`] pieces.
fn arb_cuts(max_depth: u32) -> impl Strategy<Value = Vec<u128>> {
    prop_oneof![
        3 => proptest::collection::vec(arb_point(max_depth), SCATTERED_POINTS),
        2 => arb_cluster(max_depth),
        1 => (
            proptest::collection::vec(arb_point(max_depth), SCATTERED_BESIDE_CLUSTER),
            arb_cluster(max_depth),
        )
            .prop_map(|(mut scattered, cluster)| {
                scattered.extend(cluster);
                scattered
            }),
    ]
    .prop_map(|mut cuts| {
        cuts.extend([0, ONE]);
        cuts.sort_unstable();
        cuts.dedup();
        cuts
    })
}

/// Draws a nonempty party whose cut points sit at most `max_depth` levels
/// deep.
///
/// Each piece is owned or not by its own bit of a mask, which has a bit for
/// each of up to [`MAX_PIECES`] pieces, and an empty choice owns one piece
/// instead.
fn arb_deep_party_within(max_depth: u32) -> impl Strategy<Value = tree::Party> {
    (arb_cuts(max_depth), any::<u64>(), any::<Index>()).prop_map(|(cuts, mask, fallback)| {
        let pieces = cuts.len() - 1;
        let bit = |piece: usize| mask >> piece & 1 == 1;
        if (0..pieces).any(bit) {
            party_of_pieces(&cuts, bit)
        } else {
            let only = fallback.index(pieces);
            party_of_pieces(&cuts, |piece| piece == only)
        }
    })
}

/// Draws a nonempty party at most [`DEEP_PARTY_DEPTH`] levels deep.
pub(crate) fn arb_deep_party() -> impl Strategy<Value = tree::Party> {
    arb_deep_party_within(DEEP_PARTY_DEPTH)
}

/// Draws `k` pairwise-disjoint nonempty parties.
///
/// The cut set gains `k` extra points and is refined, by bisecting its
/// widest piece, until it has at least `k` pieces. Each color is then dealt
/// to a distinct piece, and every other piece takes a random color or none,
/// so every party is nonempty and no piece has two owners.
pub(crate) fn arb_disjoint_parties(k: usize) -> impl Strategy<Value = Vec<tree::Party>> {
    (
        arb_cuts(DEEP_PARTY_DEPTH),
        proptest::collection::vec(arb_point(DEEP_PARTY_DEPTH), k),
        proptest::collection::vec(any::<u8>(), 1..=32),
        proptest::collection::vec(any::<Index>(), k),
    )
        .prop_map(move |(mut cuts, extra, colors, dealt)| {
            cuts.extend(extra);
            cuts.sort_unstable();
            cuts.dedup();
            while cuts.len() - 1 < k {
                let widest = (0..cuts.len() - 1)
                    .max_by_key(|&i| cuts[i + 1] - cuts[i])
                    .expect("the cut set always holds 0 and 1, so one piece");
                let middle = cuts[widest] + (cuts[widest + 1] - cuts[widest]) / 2;
                cuts.insert(widest + 1, middle);
            }
            let pieces = cuts.len() - 1;
            // Color `k` is no color: the piece stays unowned.
            let mut color: Vec<usize> = (0..pieces)
                .map(|piece| usize::from(colors[piece % colors.len()]) % (k + 1))
                .collect();
            let mut undealt: Vec<usize> = (0..pieces).collect();
            for (c, pick) in dealt.iter().enumerate() {
                let piece = undealt.swap_remove(pick.index(undealt.len()));
                color[piece] = c;
            }
            (0..k)
                .map(|c| party_of_pieces(&cuts, |piece| color[piece] == c))
                .collect()
        })
}

/// How a family from [`arb_deep_party_family`] departs from pairwise
/// disjointness.
#[derive(Clone, Debug)]
enum Overlap {
    /// The family is pairwise disjoint.
    Disjoint,
    /// A copy of one member joins the family.
    Alias(Index),
    /// An independently drawn party joins, which may or may not overlap.
    Unrelated(tree::Party),
    /// An independently drawn party joined with one member joins, which
    /// overlaps that member.
    Covering(tree::Party, Index),
}

/// Draws a receiver and items for the family laws: from 2 to
/// [`FAMILY_ARITY_MAX`] parties in all.
///
/// Two fifths of the families are pairwise disjoint, so `join_all` must
/// accept them. The rest are pairwise disjoint but for one party inserted at
/// a random position, the receiver's included: a copy of a member, an
/// unrelated party, or an unrelated party covering a member. Those are
/// rejected, except where an unrelated party happens to be disjoint.
pub(crate) fn arb_deep_party_family() -> impl Strategy<Value = (tree::Party, Vec<tree::Party>)> {
    let overlap = prop_oneof![
        2 => Just(Overlap::Disjoint),
        1 => any::<Index>().prop_map(Overlap::Alias),
        1 => arb_deep_party().prop_map(Overlap::Unrelated),
        1 => (arb_deep_party(), any::<Index>())
            .prop_map(|(party, member)| Overlap::Covering(party, member)),
    ];
    (2..=FAMILY_ARITY_MAX, overlap)
        .prop_flat_map(|(arity, overlap)| {
            let disjoint = arity - usize::from(!matches!(overlap, Overlap::Disjoint));
            (
                Just(overlap),
                arb_disjoint_parties(disjoint),
                any::<Index>(),
            )
        })
        .prop_map(|(overlap, mut family, position)| {
            let extra = match overlap {
                Overlap::Disjoint => None,
                Overlap::Alias(member) => Some(family[member.index(family.len())].clone()),
                Overlap::Unrelated(party) => Some(party),
                Overlap::Covering(party, member) => {
                    Some(party.union(family[member.index(family.len())].clone()))
                }
            };
            if let Some(extra) = extra {
                family.insert(position.index(family.len() + 1), extra);
            }
            let items = family.split_off(1);
            let receiver = family.pop().expect("a family holds at least two parties");
            (receiver, items)
        })
}

/// Draws a family from [`arb_deep_party_family`] with each party paired
/// with a version: one of up to three deep versions, or the empty version.
///
/// Drawing versions from a small pool makes several clocks share one
/// history, as forks of one clock do.
#[allow(clippy::type_complexity)]
pub(crate) fn arb_deep_clock_family() -> impl Strategy<
    Value = (
        (tree::Party, tree::Version),
        Vec<(tree::Party, tree::Version)>,
    ),
> {
    (
        arb_deep_party_family(),
        proptest::collection::vec(arb_deep_version(), 1..=3),
        proptest::collection::vec(any::<Index>(), FAMILY_ARITY_MAX + 1),
    )
        .prop_map(|((receiver, items), pool, picks)| {
            let mut versions: Vec<tree::Version> = pool.iter().map(LeafList::tree).collect();
            versions.push(tree::Version::new());
            let mut pick = picks
                .iter()
                .map(|pick| versions[pick.index(versions.len())].clone());
            let mut clock = |party| (party, pick.next().expect("one pick per party"));
            let receiver = clock(receiver);
            let items = items.into_iter().map(&mut clock).collect();
            (receiver, items)
        })
}
