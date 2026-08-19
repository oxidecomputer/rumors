//! Unit suites for the numerator's two-arm storage: every wide-arm
//! operation differentially against the backend as oracle.
//!
//! The host backend's ceiling is astronomically past memory, so under a
//! lowered test ceiling ([`ceiling::force`]) every wide-arm value here is
//! *also* representable as a [`UBig`] — which makes the backend itself the
//! exact oracle for the arm that exists because the backend (on 32-bit
//! targets, at production widths) cannot hold the value. The value-level
//! rank suites (`version/tests.rs`) drive the same arms through the public
//! doors; these tests pin the arm mechanics — dispatch, assembly, shifts,
//! bias steps, windows, and rendering — one operation at a time.

use dashu_int::ops::BitTest;
use dashu_int::UBig;
use proptest::prelude::*;

use super::*;

/// The lowered ceiling every suite here forces: small enough that a few
/// dozen bytes cross it, large enough that both arms and the seam get
/// populated by the generators.
const TEST_CEILING_BITS: u64 = 96;

/// A `Num` from oracle bytes (little-endian), through the canonical
/// dispatch.
fn num_from_oracle(value: &UBig) -> Num {
    let bytes = value.to_be_bytes();
    let lead = bytes.iter().take_while(|&&byte| byte == 0).count();
    Num::materialize_be(&bytes[lead..], 0)
}

/// The oracle value a `Num` denotes, reconstructed from its test bytes.
fn oracle_of(num: &Num) -> UBig {
    UBig::from_le_bytes(&num.to_bytes_le())
}

/// Arbitrary oracle values straddling the test ceiling: sub-word, near
/// the ceiling on both sides, and several limbs past it.
fn arb_value() -> impl Strategy<Value = UBig> {
    proptest::collection::vec(any::<u64>(), 1..=6).prop_map(|limbs| {
        let bytes: Vec<u8> = limbs.iter().flat_map(|limb| limb.to_le_bytes()).collect();
        UBig::from_le_bytes(&bytes)
    })
}

proptest! {
    /// Materialization dispatches on the canonical arm — wide exactly
    /// when the value outgrows the ceiling — and denotes the input value
    /// exactly, at every sub-byte pad.
    #[test]
    fn materialize_is_exact_and_canonical(value in arb_value(), pad in 0u32..8) {
        let _guard = ceiling::force(TEST_CEILING_BITS);
        // The pad contract: the low `pad` bits of the image are the
        // value's dropped zeros, so build the image as `value << pad`.
        let image = value.clone() << (pad as usize);
        let bytes = image.to_be_bytes();
        let lead = bytes.iter().take_while(|&&byte| byte == 0).count();
        let num = Num::materialize_be(&bytes[lead..], pad);
        prop_assert_eq!(oracle_of(&num), value.clone());
        prop_assert_eq!(num.is_wide(), value.bit_len() as u64 > TEST_CEILING_BITS);
        prop_assert_eq!(num.bits(), value.bit_len() as u64);
    }

    /// `shr` equals the oracle's shift at every amount up to past the
    /// width, and the result re-dispatches onto the canonical arm (a wide
    /// value shrinking below the ceiling comes back as the base arm).
    #[test]
    fn shr_matches_the_oracle_and_redispatches(value in arb_value(), amount in 0u64..512) {
        let _guard = ceiling::force(TEST_CEILING_BITS);
        let num = num_from_oracle(&value);
        let shifted = num.shr(amount);
        let expected = value >> usize::try_from(amount).unwrap();
        prop_assert_eq!(oracle_of(&shifted), expected.clone());
        prop_assert_eq!(shifted.is_wide(), expected.bit_len() as u64 > TEST_CEILING_BITS);
    }

    /// The bias steps are exact inverses across the arm seam: `plus_one`
    /// then `minus_one` is the identity, each matches the oracle, and
    /// each lands on the canonical arm.
    #[test]
    fn bias_steps_match_the_oracle(value in arb_value()) {
        let _guard = ceiling::force(TEST_CEILING_BITS);
        let num = num_from_oracle(&value);
        let up = num.clone().plus_one();
        prop_assert_eq!(oracle_of(&up), value.clone() + 1u8);
        prop_assert_eq!(up.is_wide(), (value.clone() + 1u8).bit_len() as u64 > TEST_CEILING_BITS);
        let down = up.minus_one();
        prop_assert_eq!(oracle_of(&down), value.clone());
        prop_assert_eq!(&down, &num);
    }

    /// Bit reads, widths, and trailing zeros agree with the oracle on
    /// both arms.
    #[test]
    fn bit_reads_match_the_oracle(value in arb_value(), probe in 0u64..400) {
        let _guard = ceiling::force(TEST_CEILING_BITS);
        let num = num_from_oracle(&value);
        prop_assert_eq!(num.bits(), value.bit_len() as u64);
        prop_assert_eq!(num.bit(probe), usize::try_from(probe).is_ok_and(|i| value.bit(i)));
        prop_assert_eq!(num.trailing_zeros(), value.trailing_zeros().map(|n| n as u64));
    }

    /// The class-tie window comparison agrees with the oracle's numeric
    /// order under MSB alignment, across every arm pairing.
    ///
    /// Alignment: `msb_cmp` compares `a` and `b` as MSB-aligned bit
    /// strings, which is the numeric order of `a · 2^(width(b))` versus
    /// `b · 2^(width(a))` — the oracle spelled with materialized shifts.
    #[test]
    fn msb_cmp_matches_the_aligned_oracle(a in arb_value(), b in arb_value()) {
        let _guard = ceiling::force(TEST_CEILING_BITS);
        // Odd operands: the tail rule's normalization premise (the
        // stored numerator invariant).
        let (a, b) = (a | UBig::ONE, b | UBig::ONE);
        let na = num_from_oracle(&a);
        let nb = num_from_oracle(&b);
        let aligned_a = a.clone() << b.bit_len();
        let aligned_b = b.clone() << a.bit_len();
        prop_assert_eq!(Num::msb_cmp(&na, &nb), aligned_a.cmp(&aligned_b));
    }

    /// The decimal rendering equals the oracle's on both arms — the wide
    /// arm's long division against the backend's own conversion.
    #[test]
    fn decimal_rendering_matches_the_oracle(value in arb_value()) {
        let _guard = ceiling::force(TEST_CEILING_BITS);
        let num = num_from_oracle(&value);
        prop_assert_eq!(format!("{num}"), format!("{value}"));
    }

    /// Structural equality and hashing are value equality across the
    /// dispatch: equal values are one arm and equal, unequal values are
    /// unequal whatever their arms.
    #[test]
    fn equality_is_value_equality(a in arb_value(), b in arb_value()) {
        let _guard = ceiling::force(TEST_CEILING_BITS);
        let na = num_from_oracle(&a);
        let nb = num_from_oracle(&b);
        prop_assert_eq!(na == nb, a == b);
        prop_assert_eq!(&na, &na.clone());
    }
}

/// The limb increment grows by a limb on a full carry and is exact at
/// the all-ones corners the proptest families essentially never draw.
#[test]
fn increment_carries_across_full_limbs() {
    assert_eq!(increment(vec![u64::MAX]), vec![0, 1]);
    assert_eq!(increment(vec![u64::MAX, u64::MAX]), vec![0, 0, 1]);
    assert_eq!(increment(vec![u64::MAX, 7]), vec![0, 8]);
    assert_eq!(increment(vec![5]), vec![6]);
}

/// The arm seam's exact corners: a base value at the ceiling crosses to
/// wide on `plus_one` only when the carry outgrows the ceiling, and the
/// wide power of two at the ceiling's edge falls back to base on
/// `minus_one`.
#[test]
fn arm_seam_corners_redispatch_exactly() {
    let _guard = ceiling::force(TEST_CEILING_BITS);
    // All ones at the ceiling: the carry crosses the seam upward.
    let all_ones = (UBig::ONE << usize::try_from(TEST_CEILING_BITS).unwrap()) - 1u8;
    let num = num_from_oracle(&all_ones);
    assert!(!num.is_wide(), "at the ceiling exactly: base arm");
    let carried = num.plus_one();
    assert!(carried.is_wide(), "the carry outgrows the ceiling");
    assert_eq!(oracle_of(&carried), all_ones + 1u8);
    // The power of two just past the ceiling: minus one falls back.
    let back = carried.minus_one();
    assert!(!back.is_wide(), "the borrow re-dispatches downward");
    // A ceiling-width value whose increment does not carry stays base.
    let even = UBig::ONE << usize::try_from(TEST_CEILING_BITS - 1).unwrap();
    let stays = num_from_oracle(&even).plus_one();
    assert!(!stays.is_wide(), "no carry, no crossing");
    assert_eq!(oracle_of(&stays), even + 1u8);
}

/// `from_limbs` strips high zero limbs, reads empty as zero, and
/// dispatches canonically — the accumulator readout's contract.
#[test]
fn from_limbs_normalizes_and_dispatches() {
    let _guard = ceiling::force(TEST_CEILING_BITS);
    assert_eq!(Num::from_limbs(vec![]), Num::ZERO);
    assert_eq!(Num::from_limbs(vec![0, 0]), Num::ZERO);
    let small = Num::from_limbs(vec![7, 0, 0]);
    assert!(!small.is_wide());
    assert_eq!(oracle_of(&small), UBig::from(7u8));
    let wide = Num::from_limbs(vec![1, 0, 5, 0]);
    assert!(wide.is_wide(), "129 bits exceeds the 96-bit test ceiling");
    assert_eq!(oracle_of(&wide), (UBig::from(5u8) << 128usize) + 1u8);
}
