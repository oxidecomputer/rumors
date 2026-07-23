//! Pins for the board's denomination criterion: the radix-work term, the
//! output-honesty ceiling, and the κ discriminator against the schoolbook
//! parser.

use crate::meter::{bigroot, dense, hugeleaf, Packed};
use crate::{Party, Version};

use super::{
    assert_honest_text, radix_units_party, radix_units_version, version_output_bytes,
    MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT, TEXT_BYTES_PER_CONTENT_BIT,
};

/// Decode a meter-generated packed event shape.
fn version_of(p: &Packed) -> Version {
    Version::decode(&p.bytes[..]).expect("meter shapes are strict normal form")
}

/// The radix-work term is exact on shapes whose digit and limb counts are
/// hand-derivable.
///
/// An empty version is one single-digit, single-limb value, and
/// `hugeleaf(b)` is one value of `2b + 1` gamma bits spelling
/// `floor(b * log10(2)) + 1` digits over `ceil(b / 64)` limbs.
#[test]
fn radix_units_match_hand_counts() {
    // Version::new() renders "0": 1 digit x 1 limb.
    assert_eq!(radix_units_version(&Version::new()), 1);
    // hugeleaf(200): the value 2^200 - 1 has 61 decimal digits and 4 limbs;
    // the tree is that single leaf.
    assert_eq!(radix_units_version(&version_of(&hugeleaf(200))), 61 * 4);
    // The seed party renders "1": one token.
    assert_eq!(radix_units_party(&Party::seed()), 1);
    // dense(2) stores five bases (0, 0, 0, 1, 0), each 1 digit x 1 limb.
    assert_eq!(radix_units_version(&version_of(&dense(2))), 5);
}

/// Every family's rendered text sits under the output-honesty ceiling, and
/// the ceiling is tight enough that a text stream padded past
/// [`TEXT_BYTES_PER_CONTENT_BIT`] bytes per content bit trips it.
#[test]
fn rendered_text_is_honest_and_padding_trips() {
    for (v, name) in [
        (Version::new(), "empty"),
        (version_of(&dense(500)), "dense"),
        (version_of(&bigroot(500, 100)), "bigroot"),
        (version_of(&hugeleaf(4000)), "hugeleaf"),
    ] {
        let s = v.to_string();
        assert_honest_text(name, s.len(), v.encoded_bits() as u64);
    }
    let v = version_of(&dense(500));
    let padded = (TEXT_BYTES_PER_CONTENT_BIT * v.encoded_bits() as f64) as usize + 1;
    let trips =
        std::panic::catch_unwind(|| assert_honest_text("padded", padded, v.encoded_bits() as u64));
    assert!(
        trips.is_err(),
        "a text stream past the honesty ceiling must trip the assertion"
    );
}

/// The current digit-by-digit parser's limb work exceeds the text-row limb
/// ceiling κ: the amended criterion cannot be satisfied by re-denomination
/// alone.
///
/// Those cells stay red until a subquadratic chunked converter lands (flip
/// this pin to a `<= κ` envelope in the same change).
/// `R = n_io + Σ digits × limbs` is the schoolbook cost law itself, so the
/// parser scores ~1 limb per `R` unit on the magnitude families — about 4×
/// the pinned κ. This pin is the criterion's anti-softening tripwire: it
/// fails if κ drifts up to where schoolbook passes.
#[cfg(feature = "limb-meter")]
#[test]
fn schoolbook_parser_exceeds_the_text_limb_ceiling() {
    for (packed, name) in [
        (hugeleaf(16_000), "hugeleaf"),
        (bigroot(8_000, 2_000), "bigroot"),
    ] {
        let v = version_of(&packed);
        let s = v.to_string();
        let n_io = s.len() + version_output_bytes(&v);
        let r = n_io as u64 + radix_units_version(&v);
        crate::meter::reset_limb_ops();
        let parsed: Version = s.parse().expect("a displayed version parses back");
        let ops = crate::meter::limb_ops();
        assert_eq!(parsed, v, "the parse round-trips");
        let score = ops as f64 / r as f64;
        assert!(
            score > MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT,
            "{name}: the schoolbook parser scored {score:.3} limb/R, at or under the text \
             ceiling {MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT}: the criterion softened (either κ \
             drifted up or the denominator over-counts)"
        );
    }
}
