//! Pins for the board's judging criterion: the radix-work term, the
//! output-honesty ceiling, and the tripwires that prove each judgment leg
//! catches what the others cannot.
//!
//! The tripwires: κ against a digit-by-digit schoolbook probe (and the
//! production delegating parser pinned under κ with a live counter); the
//! `n_io` exponent leg against a chunked schoolbook converter; the
//! liveness floors against a meter-bypassing walk. The time leg's tripwire
//! (an unmetered quadratic reading red on its fitted exponent) lives with
//! the leg, in `tools/benchjudge`'s self-test and the `tripwire` bench
//! target.

use crate::meter::{bigroot, dense, hugeleaf, Packed};
use crate::{Party, Version};

use super::{
    assert_honest_text, mandatory_limbs_version, radix_units_party, radix_units_version,
    TEXT_BYTES_PER_RADIX_UNIT,
};
// The limb-priced tripwires read the touch counter, so they compile only
// with the `limb-meter` feature; these names have no ungated user.
#[cfg(feature = "limb-meter")]
use super::{version_output_bytes, MAX_SCALING_EXPONENT, MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT};

/// Lift a meter-generated packed event shape into a [`Version`].
fn version_of(p: &Packed) -> Version {
    p.version()
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
/// [`TEXT_BYTES_PER_RADIX_UNIT`] bytes per radix unit trips it.
#[test]
fn rendered_text_is_honest_and_padding_trips() {
    for (v, name) in [
        (Version::new(), "empty"),
        (version_of(&dense(500)), "dense"),
        (version_of(&bigroot(500, 100)), "bigroot"),
        (version_of(&hugeleaf(4000)), "hugeleaf"),
    ] {
        let s = v.to_string();
        assert_honest_text(name, s.len(), radix_units_version(&v));
    }
    let v = version_of(&dense(500));
    let units = radix_units_version(&v);
    let padded = (TEXT_BYTES_PER_RADIX_UNIT * units as f64) as usize + 1;
    let trips = std::panic::catch_unwind(|| assert_honest_text("padded", padded, units));
    assert!(
        trips.is_err(),
        "a text stream past the honesty ceiling must trip the assertion"
    );
}

/// Pinned liveness floors for the delegating-parser pin, one per family:
/// the whole `FromStr` pipeline's measured limb records × 0.85.
///
/// The pipeline records from three sites — the delegated radix conversion
/// (one width-proportional count per materialized value), the gamma
/// encoder's arithmetic, and the validator's wide decodes — and the radix
/// site alone contributes ~33% on hugeleaf and ~25% on bigroot \[measured
/// — pipeline totals 752 and 16_387; 502 and 12_260 with the radix
/// recording deleted\]. A floor at ×0.85 (639 and 13_928) therefore sits
/// above what the other two sites can reach, so a parse path that stops
/// recording trips it; the values' mandatory limbs alone cannot separate
/// that bypass (the encode-side arithmetic already covers them).
#[cfg(feature = "limb-meter")]
const DELEGATING_PARSE_LIMB_FLOORS: [(&str, u64); 2] = [("hugeleaf", 639), ("bigroot", 13_928)];

/// The production parser's recorded limb work sits under the text-row limb
/// ceiling κ on the wide-magnitude families, with a live counter.
///
/// The parse delegates radix conversion to the backend's subquadratic
/// divide-and-conquer parser and records one width-proportional limb count
/// per materialized value (the wide-gamma decode's convention), so its
/// score against `R = n_io + Σ digits × limbs` is far under κ where
/// conversion work dominates. The ceiling leg alone would pass vacuously
/// if the parse path stopped recording, so the pin pairs it with two
/// liveness floors: the derived one (the recorded ops cover the values'
/// mandatory limbs) and the pinned per-family
/// [`DELEGATING_PARSE_LIMB_FLOORS`], set above what the pipeline's
/// non-radix recording sites reach so the radix delegation's own
/// recording is required to pass.
#[cfg(feature = "limb-meter")]
#[test]
fn delegating_parser_stays_under_the_text_limb_ceiling() {
    for ((packed, name), (floor_name, pinned_floor)) in [
        (hugeleaf(16_000), "hugeleaf"),
        (bigroot(8_000, 2_000), "bigroot"),
    ]
    .into_iter()
    .zip(DELEGATING_PARSE_LIMB_FLOORS)
    {
        assert_eq!(name, floor_name, "the pinned floors mirror the family list");
        let v = version_of(&packed);
        let s = v.to_string();
        let n_io = s.len() + version_output_bytes(&v);
        let r = n_io as u64 + radix_units_version(&v);
        crate::meter::reset_limb_ops();
        let parsed: Version = s.parse().expect("a displayed version parses back");
        let ops = crate::meter::limb_ops();
        assert_eq!(parsed, v, "the parse round-trips");
        let floor = mandatory_limbs_version(&v);
        assert!(
            ops >= floor,
            "{name}: the parse recorded {ops} limb ops under its {floor}-limb liveness \
             floor: the limb meter is not watching the parse path"
        );
        assert!(
            ops >= pinned_floor,
            "{name}: the parse recorded {ops} limb ops under the pinned {pinned_floor}-op \
             floor: the radix delegation is not recording its materialized values"
        );
        let score = ops as f64 / r as f64;
        assert!(
            score <= MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT,
            "{name}: the delegating parser scored {score:.3} limb/R, over the text ceiling \
             {MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT}: width-scale work re-entered the parse path"
        );
    }
}

/// A schoolbook conversion probe exceeds the text-row limb ceiling κ: the
/// constant leg still excludes the digit-by-digit strategy.
///
/// The probe folds each digit through one metered `mul; add` pair —
/// `Θ(digits × limbs)` at the largest constant the strategy admits — and
/// scores ~1 limb per `R` unit on the magnitude families, about 4× the
/// pinned κ (`R` is the schoolbook cost law itself). This pin is the
/// constant leg's anti-softening tripwire: it fails if κ drifts up to
/// where digit-by-digit conversion passes.
#[cfg(feature = "limb-meter")]
#[test]
fn schoolbook_probe_exceeds_the_text_limb_ceiling() {
    for (packed, name) in [
        (hugeleaf(16_000), "hugeleaf"),
        (bigroot(8_000, 2_000), "bigroot"),
    ] {
        let v = version_of(&packed);
        let s = v.to_string();
        let n_io = s.len() + version_output_bytes(&v);
        let r = n_io as u64 + radix_units_version(&v);
        let ops = schoolbook_limb_ops(&s, 1);
        let score = ops as f64 / r as f64;
        assert!(
            score > MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT,
            "{name}: the schoolbook probe scored {score:.3} limb/R, at or under the text \
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
/// arithmetic in `chunk_digits`-digit chunks and return the recorded limb
/// operations.
///
/// This is chunked schoolbook conversion: per run, `acc = acc·10^len + chunk`
/// left to right — `Θ(digits × limbs)`, quadratic in the value's bits, with
/// a constant that shrinks as the chunks widen (`chunk_digits` 1 is the
/// digit-by-digit strategy at the genre's largest constant; at most
/// [`SCHOOLBOOK_CHUNK_DIGITS`], so every chunk fits a `u32`). Each
/// accumulated value is checked exact against its run (outside the metered
/// window), so the probe's cost is the cost of the whole conversion, never
/// of a sampled fraction.
#[cfg(feature = "limb-meter")]
fn schoolbook_limb_ops(text: &str, chunk_digits: usize) -> u64 {
    use crate::codec::Base;
    assert!(
        (1..=SCHOOLBOOK_CHUNK_DIGITS).contains(&chunk_digits),
        "a schoolbook chunk is one to nine digits: a u32 covers it"
    );
    let mut values = Vec::new();
    crate::meter::reset_limb_ops();
    for run in text.split(|c: char| !c.is_ascii_digit()) {
        if run.is_empty() {
            continue;
        }
        let mut acc = Base::from(0u32);
        for chunk in run.as_bytes().chunks(chunk_digits) {
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
    use super::{evaluate, na, ByCurrency, Floors, Sample};

    let measure = |packed: &Packed| -> Sample {
        let v = version_of(packed);
        let s = v.to_string();
        let n_io = s.len() + version_output_bytes(&v);
        let r = n_io as u64 + radix_units_version(&v);
        let ops = schoolbook_limb_ops(&s, SCHOOLBOOK_CHUNK_DIGITS);
        let score = ops as f64 / r as f64;
        assert!(
            score < MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT,
            "chunked schoolbook scored {score:.3} limb/R, over κ \
             {MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT}: the constant leg now excludes it, so this \
             tripwire's premise changed and the criterion prose needs re-derivation"
        );
        const PROBE_NA: &str = "probe: the limb exponent leg alone is under test";
        Sample {
            denom_bytes: n_io,
            limb_denom: r,
            text_row: true,
            floors: Floors {
                heap: na(PROBE_NA),
                segments: na(PROBE_NA),
                limb: na(PROBE_NA),
                scan: na(PROBE_NA),
            },
            readings: ByCurrency {
                heap: Some(0),
                segments: Some(0),
                limb: Some(ops),
                scan: None,
            },
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
        cell.scores.limb.exp,
        cell.scores.limb.per_unit
    );
}

/// The mandatory-limb term is exact on shapes whose magnitude widths are
/// hand-derivable.
///
/// An empty version and a small dense spine hold only machine-word-scale
/// values (no limb work is mandatory); `hugeleaf(200)` stores one 200-bit
/// value, whose four limbs any materialization must touch.
#[test]
fn mandatory_limbs_match_hand_counts() {
    assert_eq!(mandatory_limbs_version(&Version::new()), 0);
    assert_eq!(mandatory_limbs_version(&version_of(&dense(2))), 0);
    assert_eq!(mandatory_limbs_version(&version_of(&hugeleaf(200))), 4);
}

/// Sum the stored bits of `v` by direct slice indexing: real linear
/// traversal work that touches no metered primitive (no cursor, no builder,
/// no arithmetic, no allocation).
#[cfg(feature = "scan-meter")]
fn bypass_walk(v: &Version) -> usize {
    let enc = v.as_encoded();
    let all = crate::codec::bytes_as_bits(&enc.bytes);
    let bits = &all[..enc.bits];
    (0..bits.len()).filter(|&i| bits[i]).count()
}

/// A body that does its traversal outside the metered primitives reads
/// green under ceilings alone and red under the committed liveness floors.
///
/// A criterion of ceilings alone is vacuous against exactly this bypass;
/// the floors close it.
///
/// The probe walks a decoded dense spine by direct indexing, so every
/// counter column records ~nothing while real linear work runs. Both legs
/// go through [`evaluate`] with the probe's real counter readings; the only
/// difference is the declarations — all-NA (ceilings alone) versus the
/// committed walk convention (scan floored at one bit per packed byte).
#[cfg(feature = "scan-meter")]
#[test]
fn bypassing_walk_is_green_under_ceilings_alone_and_red_under_floors() {
    use super::{evaluate, na, walk_floors, ByCurrency, Floors, Sample, SCAN_FLOOR_TRIP};

    const PROBE_NA: &str = "probe: the ceilings-alone leg declares no floors";
    fn na_floors(_packed_bytes: usize) -> Floors {
        Floors {
            heap: na(PROBE_NA),
            segments: na(PROBE_NA),
            limb: na(PROBE_NA),
            scan: na(PROBE_NA),
        }
    }

    let sample = |depth: usize, floors_of: fn(usize) -> Floors| -> Sample {
        let packed = dense(depth);
        let v = version_of(&packed);
        let n = packed.bytes.len();
        crate::meter::reset_scan_bits();
        let ones = bypass_walk(&v);
        let scanned = crate::meter::scan_bits();
        assert!(ones > 0, "the bypass walk does real work over real bits");
        Sample {
            denom_bytes: n,
            limb_denom: n as u64,
            text_row: false,
            floors: floors_of(n),
            readings: ByCurrency {
                heap: Some(0),
                segments: Some(0),
                limb: None,
                scan: Some(scanned),
            },
        }
    };

    let ceilings_only = evaluate(
        "bypass_probe",
        "dense",
        sample(1_000, na_floors),
        sample(2_000, na_floors),
    );
    assert!(
        ceilings_only.red.is_empty(),
        "under ceilings alone the bypass walk must read green (every counter near zero): \
         got {:?}",
        ceilings_only.red
    );

    let floored = evaluate(
        "bypass_probe",
        "dense",
        sample(1_000, walk_floors),
        sample(2_000, walk_floors),
    );
    assert_eq!(
        floored.red,
        vec![SCAN_FLOOR_TRIP],
        "under the committed floors the bypass walk must read red on exactly the scan \
         floor: the meter is not watching its traversal"
    );
}
