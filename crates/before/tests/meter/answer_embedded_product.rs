//! A rank family whose exact answer requires wide multiplication.
//!
//! The close-time settle's wide × dense case — and the floor under
//! every settle. The plateau-puncture family `PP(w, d)`
//! (`meter::plateau_puncture`) embeds its excess in the exact answer,
//! not in any ledger accounting: every turn leaf sits on one
//! incompressible pseudorandom plateau `x` of `w` digits, the turn
//! positions spell a jittered mass `y` of `d` isolated digits, and
//! the rank numerator is exactly `2·x·y + 1` — a Θ(w)-digit ×
//! Θ(d)-term integer product bought with Θ(w + d) input bits, both
//! factors' content beyond the settle's own balanced-digit compaction
//! (`mul_bound_embedding_is_alive` pins exactly that). No promotion
//! ever fires (the one wide plunge parks once and no later freeze
//! arrives), so the cost sits in the close-time settle `P · segment`,
//! outside the promotion ledger and its product tree entirely:
//! computing the answer *is* one wide × dense multiplication, which
//! the shipped settle delegates whole to the backend at its
//! multiplication bound `M(|v|)`. The floor is a reduction, not a
//! bet on this instance: the same constructor embeds `2·x·y + 1` for
//! arbitrary factors (`meter::puncture_product`; the query fold's
//! `arbitrary_factors_embed_their_product_in_exact_rank` proptest
//! pins it), so any fold that answers exactly multiplies arbitrary
//! input-funded integers — `Ω(M(|v|))` floors every settle and the
//! `# Complexity` claims' worst case can never reach `O(|v|)` while
//! integer multiplication is superlinear. The fold's own
//! deterministic counters price its traffic — operand reads, the
//! compacted segment, and the product's width — and read flat per byte.

use super::min_ticks_from_big;
use before::testing::meter;
use before::testing::meter::registry::Shape;
use num_bigint::BigUint;
use suanpan::touch_meter;

/// Places a binary point `exp` digits from the right of an odd numerator.
fn binary_rank_text(num: &BigUint, exp: usize) -> String {
    let mut digits = format!("{num:b}");
    if exp == 0 {
        return digits;
    }
    if digits.len() <= exp {
        return format!("0.{}{}", "0".repeat(exp - digits.len()), digits);
    }
    digits.insert(digits.len() - exp, '.');
    digits
}

/// One public `Version::rank` run over `PP(s, s)`: input bytes and
/// the touch and densify counters over the rank body alone.
///
/// Carries the `min_ticks` closed form (`s · x + 1` over the
/// committed factors) as the generator's semantic leg, the
/// exact-rank leg (the answer is the product `2·x·y + 1` — the
/// `Ω(M(|v|))` mandate's witness), and the
/// one-touch-per-operand-byte liveness floor.
fn run(s: usize) -> (u64, u64, u64) {
    let v = Shape::PlateauPuncture.build2(s, s).version();
    let bytes = v.encode().len() as u64;
    let (x, y) = meter::plateau_puncture_factors(s, s);
    let expected = BigUint::from(s as u64) * &x + 1u8;
    assert_eq!(
        v.min_ticks(),
        min_ticks_from_big(&expected),
        "the family's stored-code sum disagrees with min_ticks: the \
         generator does not build the tree this band reasons about"
    );
    touch_meter::reset();
    meter::reset_densified_digits();
    let rank = v.rank();
    std::hint::black_box(&rank);
    let touches = touch_meter::touches();
    let densified = meter::densified_digits();
    // The answer itself is the product, so the value check keeps the
    // measured operation tied to the intended family.
    let numerator = ((&x * &y) << 1usize) + 1u8;
    assert_eq!(
        rank.to_string(),
        binary_rank_text(&numerator, 66 * s),
        "the exact rank is the plateau times the punctured turn mass"
    );
    assert!(
        touches >= bytes,
        "rank at {bytes} operand bytes: {touches} digit touches under \
         the one-per-byte floor: the fold's accumulator work is not \
         metered",
    );
    (bytes, touches, densified)
}

/// Plateau digits (and turn count) of the band's small run (the
/// large run doubles both).
const PLATEAU_PUNCTURE_SMALL: usize = 500;

/// Absolute two-scale touch ceilings for rank on the
/// plateau-puncture family, measured ×1.25 (the record and every
/// re-pin's movement live in the pin commits).
///
/// Flat per encoded byte across the doubling. A close-time settle that paid
/// the parked plateau's width once per trailing-mass digit would exceed
/// these ceilings.
const PLATEAU_PUNCTURE_CEILINGS: [u64; 2] = [60_525, 121_058];

/// Absolute two-scale densify ceilings for rank on the
/// plateau-puncture family: the measured record ×1.25, rounded up (the
/// record and every re-pin's movement live in the pin commits).
///
/// Flat per encoded byte across the doubling: the close-time settle
/// densifies the jittered punctured mass, whose span scales with the
/// turn count exactly as the input does. The position axis — an image
/// sized by a cluster's absolute digit index — is isolated (and
/// killed) by the hoisted-window band, where spans stay fixed as
/// positions grow.
const PLATEAU_PUNCTURE_DENSIFY_CEILINGS: (u64, u64) = (2_580, 5_158);

/// rank is flat per byte on the plateau-puncture family: per-byte
/// touch work stays within ×1.25 across a `PP(s, s)`
/// doubling, under absolute two-scale ceilings.
///
/// Flat in the fold's own traffic, never in total work: the
/// answer-embedded product runs inside the backend at the
/// multiplication bound (the exact-rank leg in [`run`] proves the
/// answer is still the product), so this band and that leg
/// together witness both directions of the settle's bound —
/// `O(M(|v|))` achieved, `Ω(M(|v|))` mandatory.
#[test]
fn rank_plateau_puncture_is_flat_per_unit() {
    let (small_bytes, small_touches, small_densify) = run(PLATEAU_PUNCTURE_SMALL);
    let (large_bytes, large_touches, large_densify) = run(2 * PLATEAU_PUNCTURE_SMALL);
    eprintln!(
        "MEASURED rank_plateau_puncture: small={small_touches}/{small_bytes}B \
         (densify {small_densify}) \
         large={large_touches}/{large_bytes}B \
         (densify {large_densify})"
    );
    for (name, small, large, ceilings) in [
        (
            "touches",
            small_touches,
            large_touches,
            (PLATEAU_PUNCTURE_CEILINGS[0], PLATEAU_PUNCTURE_CEILINGS[1]),
        ),
        (
            "densified digits",
            small_densify,
            large_densify,
            PLATEAU_PUNCTURE_DENSIFY_CEILINGS,
        ),
    ] {
        assert!(
            small <= ceilings.0 && large <= ceilings.1,
            "rank ({name}) exceeds the pinned ceilings on the \
             plateau-puncture family ({small}/{small_bytes}B -> \
             {large}/{large_bytes}B against {} / {})",
            ceilings.0,
            ceilings.1,
        );
        assert!(
            u128::from(large) * u128::from(small_bytes) * 4
                <= u128::from(small) * u128::from(large_bytes) * 5,
            "rank ({name}) grew more than x1.25 per byte across the \
             plateau-puncture doubling ({small}/{small_bytes}B -> \
             {large}/{large_bytes}B): the close-time settle is paying \
             the parked width times the segment's density again",
        );
    }
}
