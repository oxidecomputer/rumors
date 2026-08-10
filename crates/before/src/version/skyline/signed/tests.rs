//! Differential pins for the fused signed-gamma coder.
//!
//! [`gamma_code_signed`] and its [`Int`] twin fuse the zigzag map with the
//! gamma coder, and a fast-path guard routes word-scale magnitudes through
//! machine arithmetic. The reference here is the unfused composition —
//! [`zigzag_signed`] then [`codec::encode_int`] — which shares neither the
//! fused mantissa expression nor the guard, so a wrong guard direction or an
//! off-by-one mantissa near the fast-path magnitude bound reads red
//! deterministically under the seam sweep and under committed seeds of the
//! generalized family.

use bitvec::field::BitField;
use proptest::prelude::*;

use crate::codec::{self, Base, BitsMut, Code, Int};

use super::{gamma_code_signed, gamma_code_signed_int, zigzag_signed, Sign};

/// Render a payload code into live bits by [`Code`]'s own representation
/// contract (a small code sits value-packed at the register's low end, first
/// bit most significant) — independent of both coders under comparison.
fn bits_of(code: &Code) -> BitsMut {
    match code {
        Code::Small { bits, len } => {
            let mut out = BitsMut::new();
            out.resize(usize::from(*len), false);
            out[..].store_be::<u64>(*bits);
            out
        }
        Code::Wide(bits) => bits.clone(),
    }
}

/// Assert both fused coders agree with the unfused composition on one signed
/// delta, bit for bit.
fn assert_fused_matches(sign: Sign, magnitude: &Base) {
    let mut reference = BitsMut::new();
    codec::encode_int(&mut reference, &zigzag_signed(sign, magnitude.clone()));
    assert_eq!(
        bits_of(&gamma_code_signed(sign, magnitude)),
        reference,
        "gamma_code_signed disagrees with the unfused composition on {sign:?} {magnitude:?}"
    );
    assert_eq!(
        bits_of(&gamma_code_signed_int(
            sign,
            &Int::from_base(magnitude.clone())
        )),
        reference,
        "gamma_code_signed_int disagrees with the unfused composition on {sign:?} {magnitude:?}"
    );
}

/// Magnitudes concentrated on the coder's seams.
///
/// The dense small range, random values of exactly 29..=34 bits (the band
/// holding the fast-path magnitude bound), uniform words, and `2^k ± {0, 1}`
/// through the wide (past-`u64`) range.
fn arb_seam_magnitude() -> impl Strategy<Value = Base> {
    prop_oneof![
        (0u64..=64).prop_map(Base::from),
        any::<u64>().prop_map(Base::from),
        (29u32..=34, any::<u64>())
            .prop_map(|(w, raw)| Base::from((1u64 << (w - 1)) | (raw & ((1u64 << (w - 1)) - 1)))),
        (0u32..=97, -1i32..=1).prop_map(|(k, d)| {
            let p = Base::from(1u8) << k;
            match d {
                -1 => p - &Base::from(1u8),
                0 => p,
                _ => p + 1u32,
            }
        }),
    ]
}

/// Deterministic seam sweep: both fused coders match the unfused composition
/// on the fast-path bound's dense neighborhood, the dense small range, the
/// word edges, and `2^k ± 1` across the wide band — both signs throughout.
///
/// The point tripwire riding beside the generalized family below: a flipped
/// fast-path guard or a mantissa error at the `2^31` seam fails here without
/// any random exploration.
#[test]
fn fused_coders_match_at_the_fast_path_seam() {
    let mut magnitudes: Vec<Base> = Vec::new();
    for m in 0..=64u64 {
        magnitudes.push(Base::from(m));
    }
    let bound = 1u64 << 31;
    for m in bound - 4..=bound + 4 {
        magnitudes.push(Base::from(m));
    }
    magnitudes.push(Base::from(u64::MAX - 1));
    magnitudes.push(Base::from(u64::MAX));
    for k in 0..=97u32 {
        let p = Base::from(1u8) << k;
        magnitudes.push(p.clone() - &Base::from(1u8));
        magnitudes.push(p.clone() + 1u32);
        magnitudes.push(p);
    }
    for magnitude in &magnitudes {
        assert_fused_matches(Sign::Positive, magnitude);
        if *magnitude != Base::ZERO {
            assert_fused_matches(Sign::Negative, magnitude);
        }
    }
}

proptest! {
    /// The fused signed-gamma coders are bit-identical to the unfused
    /// composition under both signs.
    ///
    /// The reference is zigzag, then the arbitrary-precision gamma encoder;
    /// magnitudes span the fast path, its bound's bit band, the word range,
    /// and wide (past-`u64`) values.
    #[test]
    fn fused_coders_match_the_unfused_composition(
        magnitude in arb_seam_magnitude(),
        negative in any::<bool>(),
    ) {
        let sign = if negative && magnitude != Base::ZERO {
            Sign::Negative
        } else {
            Sign::Positive
        };
        assert_fused_matches(sign, &magnitude);
    }
}
