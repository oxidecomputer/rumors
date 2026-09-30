//! Resource envelopes for ranks and rank arithmetic.

use super::*;

// ─── rank scenarios ─────────────────────────────────────────────────────────
//
// The rank fold and the Rank operations. The fold rows run on the
// digit-routed merge fold (child numerators land in their sibling's
// accumulator at the exponent gap, never through a materialized shift of
// the accumulated value), so their arithmetic shows up in the accumulator
// touch column, which is also the fold's liveness floor. RANK_HARMONIC is the
// fold's separating family — a
// numerator as wide as the depth already walked at every level — pinned
// linear where a re-shifting fold reads magnitude-quadratic. RANK_DENSE
// and RANK_BIGROOT are the controls: one-bit and root-heavy numerators
// respectively.
//
// RANK_PAIR_MISMATCH pins the class-first comparison's expected remainder
// (the subtraction and addition outputs' own content). RANK_SUM_MIXED drives
// successively finer scales after a wide numerator, where shifting the held
// value for every summand would be superlinear.

// Pins per the file doc's convention; each row's trailing comment states
// the mechanism that prices it.
#[rustfmt::skip]
mod rank_env {
    use super::{band, envelope, Envelope};
    pub const RANK_DENSE: Envelope         = envelope( 30_720,             band(7, 3), band(937_515, 562_509)); // the depth control: word-scale numerators fold in the accumulator's quick register, so the work columns sit near zero and the heap is the at-rest form
    pub const RANK_BIGROOT: Envelope       = envelope( 67_145,     band(8_993, 5_395), band(275_023, 165_013)); // the wide-magnitude control: one root-wide decode and one root-wide fold; the segment feed opens only at the first freeze
    pub const RANK_HARMONIC: Envelope      = envelope( 52_500, band(248_285, 148_971), band(491_530, 294_918)); // the separating family: each level's one-leaf sibling lands at the exponent gap, so touches stay linear in depth and no accumulated numerator is re-shifted
    pub const RANK_PAIR_MISMATCH: Envelope = envelope(234_400,             band(0, 0),             band(0, 0)); // class-first comparison decides the order without scanning either Version
    pub const RANK_SUM_MIXED: Envelope     = envelope(300_000,   band(55_000, 33_000),             band(0, 0)); // each full-width shift at least doubles the occupied span
}

/// The rank fold on the dense spine stays within its envelope.
///
/// The control: the spine's numerator stays one bit wide, so the fold's
/// per-level shifts are word-scale and the walk is linear. The rank is
/// the version kernel's, exactly.
#[test]
fn rank_dense_envelope() {
    let p = Shape::Dense.build1(DENSE_DEPTH);
    let v = version_of(&p);
    let r = metered("rank_dense", p.bytes.len(), &rank_env::RANK_DENSE, || {
        v.rank()
    });
    assert_eq!(r, kernel_rank(&v), "the public rank must be the kernel's");
}

/// The rank fold on the bigroot spine stays within its envelope.
///
/// The wide-magnitude control: the first leaf's magnitude seeds the
/// frozen component and is read once, in the closing shifted add. The
/// rank is the version kernel's, exactly.
#[test]
fn rank_bigroot_envelope() {
    let p = Shape::Bigroot.build2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
    let v = version_of(&p);
    let r = metered(
        "rank_bigroot",
        p.bytes.len(),
        &rank_env::RANK_BIGROOT,
        || v.rank(),
    );
    assert_eq!(r, kernel_rank(&v), "the public rank must be the kernel's");
}

/// The rank fold on the harmonic spine stays within its envelope — the
/// fold's separating family, pinned linear. The rank is the Version's
/// kernel's, exactly.
///
/// The accumulated numerator is as wide as the depth already walked at
/// every level, and the digit-routed merge folds each level's one-leaf
/// sibling into it at the exponent gap instead of re-shifting it.
#[test]
fn rank_harmonic_envelope() {
    let p = Shape::Harmonic.build1(RANK_HARMONIC_DEPTH);
    let v = version_of(&p);
    let r = metered(
        "rank_harmonic",
        p.bytes.len(),
        &rank_env::RANK_HARMONIC,
        || v.rank(),
    );
    assert_eq!(r, kernel_rank(&v), "the public rank must be the kernel's");
}

/// `Rank::cmp` + `checked_sub` + `+` on the mismatched-exponent pair stay
/// within their envelope.
///
/// The class-first comparison decides the order and the pre-check in O(1),
/// so the pinned cost is the `Some`-arm subtraction and the addition —
/// transients that are the outputs' own value content, not amplification.
///
/// The pair is built through the public API outside measurement: the
/// dense spine's rank is the maximal-exponent operand (`1/2^d`, a
/// one-bit numerator, so the pinned cost is pure mismatch), against a
/// small integer rank at exponent zero.
#[test]
fn rank_pair_mismatch_envelope() {
    let a = version_of(&Shape::Dense.build1(RANK_PAIR_DEPTH)).rank();
    let b = uniform_version(3u8).rank();
    // Informational denominator: the pair's value content in bytes
    // (numerator bits + exponent, over eight).
    let content_bytes = RANK_PAIR_DEPTH / 8 + 1;
    let r = metered(
        "rank_pair_mismatch",
        content_bytes,
        &rank_env::RANK_PAIR_MISMATCH,
        || {
            let ord = a.cmp(&b);
            let diff = b.checked_sub(&a);
            let sum = &a + &b;
            (ord, diff, sum)
        },
    );
    let (ord, diff, sum) = r;
    assert_eq!(ord, std::cmp::Ordering::Less, "1/2^d is under 3");
    assert!(
        diff.is_some(),
        "3 dominates 1/2^d, so the difference exists"
    );
    consumed((ord, diff, sum));
}

/// `Sum` stays linear when progressively finer fractions follow a wide rank.
///
/// Each new fraction lies one bit beyond the previous scale. Re-anchoring the
/// wide numerator for every summand would multiply its width by the number of
/// fractions. Reserving a wider exponent range instead makes the total digit
/// work proportional to the ranks' combined value content.
#[test]
fn rank_sum_mixed_envelope() {
    let wide = "1"
        .repeat(RANK_SUM_WIDE_BITS)
        .parse::<before::Rank>()
        .expect("a nonzero binary integer is a canonical rank");
    let fractions: Vec<before::Rank> = (1..=RANK_SUM_FRACTIONS)
        .map(|exp| {
            format!("0.{}1", "0".repeat(exp - 1))
                .parse()
                .expect("a binary fraction ending in one is canonical")
        })
        .collect();
    let fraction_bits = RANK_SUM_FRACTIONS * (RANK_SUM_FRACTIONS + 1) / 2 + RANK_SUM_FRACTIONS;
    let content_bytes = (RANK_SUM_WIDE_BITS + fraction_bits).div_ceil(8);
    let ranks: Vec<before::Rank> = std::iter::once(wide).chain(fractions).collect();
    let expected = ranks
        .iter()
        .fold(before::Rank::ZERO, |sum, rank| sum + rank);
    let r = metered(
        "rank_sum_mixed",
        content_bytes,
        &rank_env::RANK_SUM_MIXED,
        || ranks.into_iter().sum::<before::Rank>(),
    );
    assert_eq!(r, expected, "the reserved exponent range preserves the sum");
    consumed(r);
}
