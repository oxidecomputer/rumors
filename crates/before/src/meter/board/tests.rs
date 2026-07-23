//! Pins for the board's denomination criterion: the radix-work term, the
//! output-honesty ceiling, and the text limb column's two anti-softening
//! tripwires.
//!
//! The tripwires: κ against the digit-by-digit parser, and the `n_io`
//! exponent leg against a chunked schoolbook converter.

use crate::meter::{bigroot, dense, hugeleaf, Packed};
use crate::{Party, Version};

use super::{
    assert_honest_text, radix_units_party, radix_units_version, version_output_bytes,
    MAX_SCALING_EXPONENT, MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT, TEXT_BYTES_PER_CONTENT_BIT,
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
/// the pinned κ. This pin is the constant leg's anti-softening tripwire: it
/// fails if κ drifts up to where the digit-by-digit parser passes.
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

/// Decimal digits one `mul; add` accumulator pair folds at a time: the
/// widest chunk a `u32` multiplier covers (`10^9 < 2^32`).
///
/// Off-the-shelf chunked parsers use this strategy at `u32` or `u64` chunk
/// width; wider chunks only shrink the constant further, so this width is
/// the conservative choice for a tripwire pinning that the constant sits
/// under κ.
#[cfg(feature = "limb-meter")]
const SCHOOLBOOK_CHUNK_DIGITS: usize = 9;

/// Fold every decimal run of `text` through the crate's metered `Base`
/// arithmetic in [`SCHOOLBOOK_CHUNK_DIGITS`]-digit chunks and return the
/// recorded limb operations.
///
/// This is chunked schoolbook conversion: per run, `acc = acc·10^len + chunk`
/// left to right — still `Θ(digits × limbs)`, quadratic in the value's bits,
/// only with a ~`SCHOOLBOOK_CHUNK_DIGITS`× smaller constant than the
/// digit-by-digit parser. Each accumulated value is checked exact against
/// its run (outside the metered window), so the probe's cost is the cost of
/// the whole conversion, never of a sampled fraction.
#[cfg(feature = "limb-meter")]
fn chunked_schoolbook_limb_ops(text: &str) -> u64 {
    use crate::codec::Base;
    let mut values = Vec::new();
    crate::meter::reset_limb_ops();
    for run in text.split(|c: char| !c.is_ascii_digit()) {
        if run.is_empty() {
            continue;
        }
        let mut acc = Base::from(0u32);
        for chunk in run.as_bytes().chunks(SCHOOLBOOK_CHUNK_DIGITS) {
            let chunk = std::str::from_utf8(chunk).expect("a decimal run is ASCII");
            acc *= 10u32.pow(chunk.len() as u32);
            acc += chunk.parse::<u32>().expect("a 9-digit chunk fits u32");
        }
        values.push((run, acc));
    }
    let ops = crate::meter::limb_ops();
    for (run, acc) in values {
        assert_eq!(
            acc.to_string(),
            run,
            "the probe converts every decimal run exactly"
        );
    }
    ops
}

/// A chunked schoolbook converter reads red on the text limb criterion, and
/// only on its exponent leg: chunking slips the constant under κ.
///
/// κ alone does not enforce subquadratic conversion: `u32` chunking shrinks
/// the schoolbook constant ~9× to well under κ (measured 0.11 limb/`R` on
/// hugeleaf, 0.15 on bigroot) while leaving the complexity class untouched
/// (`Θ(digits × limbs)`, limb work quadratic in the value's bits — recorded
/// here through the same metered ops the board reads). The criterion's
/// teeth against it are entirely in the limb exponent judged against
/// `n_io`: quadratic limb work reads ~2 there, over
/// [`MAX_SCALING_EXPONENT`], where a subquadratic converter's near-linear
/// recorded work stays under. (Against `R` the exponent is toothless on any
/// schoolbook converter: `R` is its own cost law, so it reads a flat ~1.)
/// This pin is the exponent leg's anti-softening tripwire: the probe's
/// measured samples go through [`evaluate`] itself, so it fails if the leg
/// re-denominates to `R` or its ceiling drifts up to where quadratic
/// conversion passes; the κ assertions pin the premise that the constant
/// leg cannot exclude this converter.
#[cfg(feature = "limb-meter")]
#[test]
fn chunked_schoolbook_slips_under_kappa_and_trips_the_exponent_leg() {
    use std::time::Duration;

    use super::{evaluate, Sample};

    let measure = |packed: &Packed| -> Sample {
        let v = version_of(packed);
        let s = v.to_string();
        let n_io = s.len() + version_output_bytes(&v);
        let r = n_io as u64 + radix_units_version(&v);
        let ops = chunked_schoolbook_limb_ops(&s);
        let score = ops as f64 / r as f64;
        assert!(
            score < MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT,
            "chunked schoolbook scored {score:.3} limb/R, over κ \
             {MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT}: the constant leg now excludes it, so this \
             tripwire's premise changed and the criterion prose needs re-derivation"
        );
        Sample {
            denom_bytes: n_io,
            limb_denom: r,
            text_row: true,
            peak_heap: 0,
            segments: 0,
            limb: Some(ops),
            wall: Duration::ZERO,
        }
    };
    // The spine-over-magnitude family is also constant-blind to chunking.
    let _ = measure(&bigroot(8_000, 2_000));
    let cell = evaluate(
        "chunked_schoolbook_probe",
        "hugeleaf",
        measure(&hugeleaf(16_000)),
        measure(&hugeleaf(32_000)),
    );
    assert_eq!(
        cell.red,
        vec!["limb exponent"],
        "the criterion must read the chunked schoolbook probe red on exactly the limb \
         exponent leg (limb exponent {:?} vs {MAX_SCALING_EXPONENT}, constant {:?} limb/R \
         vs κ {MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT}): quadratic conversion under κ is excluded \
         by that leg alone",
        cell.limb_exp,
        cell.limb_per_byte
    );
}
