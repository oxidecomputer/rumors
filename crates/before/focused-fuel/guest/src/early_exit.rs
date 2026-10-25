//! Probes of operations that may stop once their answer is decided.
//!
//! Each fixture decides its answer at the first elementary interval of a
//! comb whose tail grows with the probe's tooth count, so a walk that stops
//! there costs the same at every tail length. `C` is the comb `C(1024,
//! teeth)`, whose first leaf sits at `2^1024 − 1`; `S` is one tick through the
//! seed; `R` is a plateau on the right half, concurrent with `C`.

use before::causally::{self, Query};
use before::testing::meter::registry::Shape;
use before::testing::meter::version;
use before::{Count, Party, Span, Version};

use crate::{measured, queries};

/// The magnitude in bits of the refuted comb's teeth, fixed whatever the
/// tooth count, so the deciding first interval's codes are the same at every
/// count and only the tail the walk must not read grows.
const REFUTED_TOOTH_BITS: usize = 1_024;

/// Measures the semantic equality walk of the comb `C(1024, teeth)` against
/// the empty version.
///
/// The comb's first leaf sits at `2^1024 − 1` and the empty version's one
/// plateau at zero, so the first elementary interval refutes equality.
pub fn eq_refuted_at_first_interval(teeth: usize) -> u64 {
    let comb = Shape::CliffComb.build2(REFUTED_TOOTH_BITS, teeth).version();
    let empty = Version::new();
    measured(
        || version::equal(&comb, &empty),
        |equal| assert!(!equal, "the comb differs from the empty version"),
    )
}

/// The magnitude in bits of the right-half plateau that makes a version
/// concurrent with the refuted comb: above every comb leaf, on the half of the
/// interval the comb's first leaf does not cover.
const RIGHT_PLATEAU_BITS: usize = 2_048;

/// Builds the refuted comb `C(1024, teeth)`, whose first leaf sits at
/// `2^1024 − 1`.
fn comb(teeth: usize) -> Version {
    Shape::CliffComb.build2(REFUTED_TOOTH_BITS, teeth).version()
}

/// The small fixed version `S`: one tick through the seed, height 1
/// everywhere, below the comb's first leaf.
fn small() -> Version {
    let mut version = Version::new();
    Party::seed().tick(&mut version);
    version
}

/// The version `R`: height `2^2048` on the right half of the interval, zero on
/// the left, so it is concurrent with every refuted comb.
fn right_plateau() -> Version {
    // The right half: a node with only a right child, an owned cell, then the
    // `1` marker, most significant bit first.
    let right = Party::decode(&[0b0100_1000][..]).expect("the right half is canonical");
    let mut height = Count::from(1u8);
    for _ in 0..RIGHT_PLATEAU_BITS {
        height += height.clone();
    }
    let mut version = Version::new();
    right.ticks(&mut version, height);
    version
}

/// Returns `version` raised by one more tick through the seed, so it lies
/// strictly above `version` in a buffer of its own.
fn above(version: &Version) -> Version {
    let mut above = version.clone();
    Party::seed().tick(&mut above);
    above
}

/// Measures `span.dominance(probe)` and checks it against the documented
/// verdict.
fn dominance(lo: Version, hi: Version, probe: Version) -> u64 {
    let span = Span::new(&lo, &hi).expect("the span's endpoints are ordered");
    let expected = queries::dominance(&lo, &hi, &probe);
    measured(
        || span.dominance(&probe),
        |verdict| assert_eq!(verdict, expected, "the documented dominance"),
    )
}

/// Measures `span.precedence(probe)` and checks it against the documented
/// verdict.
fn precedence(lo: Version, hi: Version, probe: Version) -> u64 {
    let span = Span::new(&lo, &hi).expect("the span's endpoints are ordered");
    let expected = queries::precedence(&lo, &hi, &probe);
    measured(
        || span.precedence(&probe),
        |verdict| assert_eq!(verdict, expected, "the documented precedence"),
    )
}

/// Measures `span.contains(&probe)` and checks it against the documented
/// membership.
fn contains(lo: Version, hi: Version, probe: Version) -> u64 {
    let span = Span::new(&lo, &hi).expect("the span's endpoints are ordered");
    let expected = queries::contained(&lo, &hi, &probe);
    measured(
        || span.contains(&probe),
        |held| assert_eq!(held, expected, "lo <= probe <= hi"),
    )
}

/// Measures `dominance` of `R` against `[C, C | R]`: the start is concurrent
/// with the probe and refuted at the first interval.
pub fn dominance_concurrent_start(teeth: usize) -> u64 {
    let (lo, probe) = (comb(teeth), right_plateau());
    let hi = &lo | &probe;
    dominance(lo, hi, probe)
}

/// Measures `dominance` of `S` against `[S | C, above(S | C)]`: the start
/// strictly dominates the probe, refuted at the first interval.
pub fn dominance_dominating_start(teeth: usize) -> u64 {
    let probe = small();
    let lo = &probe | &comb(teeth);
    let hi = above(&lo);
    dominance(lo, hi, probe)
}

/// Measures `precedence` of `C` against `[empty, R]`: the end is concurrent
/// with the probe and refuted at the first interval.
pub fn precedence_concurrent_end(teeth: usize) -> u64 {
    precedence(Version::new(), right_plateau(), comb(teeth))
}

/// Measures `precedence` of `S | C` against `[empty, S]`: the end strictly
/// precedes the probe, refuted at the first interval.
pub fn precedence_preceded_end(teeth: usize) -> u64 {
    let hi = small();
    let probe = &hi | &comb(teeth);
    precedence(Version::new(), hi, probe)
}

/// Measures `contains` of `S | C` against `[empty, S]`: the probe lies above
/// the end, refuted at the first interval.
pub fn contains_above_end(teeth: usize) -> u64 {
    let hi = small();
    let probe = &hi | &comb(teeth);
    contains(Version::new(), hi, probe)
}

/// Measures `contains` of `S` against `[S | C, above(S | C)]`: the probe lies
/// below the start, refuted at the first interval.
pub fn contains_below_start(teeth: usize) -> u64 {
    let probe = small();
    let lo = &probe | &comb(teeth);
    let hi = above(&lo);
    contains(lo, hi, probe)
}

/// Measures `dominance` of `S` against `[empty, S | C]`: the probe is
/// contained, the start confirms over two small streams, and the end's
/// domination is refuted at its first interval, which drops its stream.
pub fn dominance_drops_the_end(teeth: usize) -> u64 {
    let probe = small();
    let hi = &probe | &comb(teeth);
    dominance(Version::new(), hi, probe)
}

/// The height of the uniform probe that lies above every leaf of the narrow
/// comb `C(64, teeth)`.
const UNIFORM_HEIGHT_BITS: usize = 65;

/// Measures `precedence` of the uniform `U = 2^65` against `[C(64, teeth),
/// U + 1]`, a contained probe whose start stream is dropped.
///
/// The end confirms over two one-leaf streams, and `probe <= lo` is refuted
/// at the first interval, which drops the start's stream.
pub fn precedence_drops_the_start(teeth: usize) -> u64 {
    let lo = Shape::CliffComb.build2(64, teeth).version();
    let mut height = Count::from(1u8);
    for _ in 0..UNIFORM_HEIGHT_BITS {
        height += height.clone();
    }
    let mut probe = Version::new();
    Party::seed().ticks(&mut probe, height);
    let hi = above(&probe);
    precedence(lo, hi, probe)
}

/// Measures `Query::from(causally::before(S)).contains(S | C)`: the bound is
/// refuted at the first interval.
pub fn ceiling_refuted(teeth: usize) -> u64 {
    let bound = small();
    let probe = &bound | &comb(teeth);
    let query = Query::from(causally::before(&bound));
    measured(
        || query.contains(&probe),
        |held| {
            assert_eq!(
                held,
                queries::le(&probe, &bound),
                "before(S) holds exactly y <= S"
            )
        },
    )
}

/// Measures `causally::since(S).contains(S | C)`: the hole is satisfied at the
/// first interval.
pub fn hole_satisfied(teeth: usize) -> u64 {
    let bound = small();
    let probe = &bound | &comb(teeth);
    let query = causally::since(&bound);
    measured(
        || query.contains(&probe),
        |held| {
            assert_eq!(
                held,
                !queries::le(&probe, &bound),
                "since(S) holds exactly y not <= S"
            )
        },
    )
}
