//! Pins for the board's judging criterion: the radix-work term, the
//! output-honesty ceiling, and the tripwires that prove each judgment leg
//! catches what the others cannot.
//!
//! The tripwires: κ against the digit-by-digit parser; the `n_io` exponent
//! leg against a chunked schoolbook converter; the liveness floors against
//! a meter-bypassing walk; the wall-exponent leg against an unmetered
//! quadratic.

use crate::meter::{bigroot, dense, hugeleaf, Packed};
use crate::{Party, Version};

use super::{
    assert_honest_text, mandatory_limbs_version, radix_units_party, radix_units_version,
    TEXT_BYTES_PER_CONTENT_BIT,
};
// The limb-priced tripwires read the touch counter, so they compile only
// with the `limb-meter` feature; these names have no ungated user.
#[cfg(feature = "limb-meter")]
use super::{version_output_bytes, MAX_SCALING_EXPONENT, MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT};

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

    use super::{evaluate, na, Floors, Sample};

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
        const PROBE_NA: &str = "probe: the limb exponent leg alone is under test";
        Sample {
            denom_bytes: n_io,
            limb_denom: r,
            text_row: true,
            floors: Floors {
                heap: na(PROBE_NA),
                limb: na(PROBE_NA),
                scan: na(PROBE_NA),
            },
            peak_heap: 0,
            segments: 0,
            limb: Some(ops),
            scan: None,
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
    let bits = v.as_bits();
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
    use std::time::Duration;

    use super::{evaluate, na, walk_floors, Floors, Sample, SCAN_FLOOR_TRIP};

    const PROBE_NA: &str = "probe: the ceilings-alone leg declares no floors";
    fn na_floors(_packed_bytes: usize) -> Floors {
        Floors {
            heap: na(PROBE_NA),
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
            peak_heap: 0,
            segments: 0,
            limb: None,
            scan: Some(scanned),
            wall: Duration::ZERO,
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

/// The size parameter of the wall-exponent probe's smaller scale; the
/// larger runs at its double.
///
/// Sized so the larger scale's quadratic pass comfortably outlasts
/// [`MIN_JUDGED_WALL_MILLIS`](super::MIN_JUDGED_WALL_MILLIS) under the dev
/// profile while the whole probe stays under about a second.
#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
const QUAD_PROBE_BASE: usize = 6_144;

/// One quadratic pass of plain machine-word arithmetic keyed to `n`: no
/// allocation, no recursion, no metered reads.
#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
fn unmetered_quadratic(n: usize) -> u64 {
    let mut acc = 0u64;
    for i in 0..n {
        for j in 0..n {
            acc = acc
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add((i ^ j) as u64);
        }
    }
    acc
}

/// An unmetered quadratic reads red on the wall-exponent leg and green on
/// all four counter columns: the time leg sees what no counter does.
///
/// The probe is plain machine-word arithmetic — no allocation, no
/// recursion, no big-integer ops, no stream reads — so heap, segments,
/// limb, and scan all record zero and every ceiling and floor passes; only
/// the measured wall, fitted across the doubling through [`evaluate`]
/// itself, exposes the ~2.0 exponent. Wall measurement wants a quiet
/// machine: the runner configuration reserves every test thread for this
/// test (the display canary's idiom), and the quadratic-versus-1.3 margin
/// absorbs what scheduling noise remains.
#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
#[test]
fn unmetered_quadratic_reads_red_on_the_wall_exponent_leg_alone() {
    use std::hint::black_box;
    use std::time::{Duration, Instant};

    use super::{evaluate, na, Floors, Sample, MIN_JUDGED_WALL_MILLIS};

    const PROBE_NA: &str = "probe: plain machine-word arithmetic declares no floors";
    let sample = |n: usize| -> Sample {
        crate::meter::reset_stack_segments();
        crate::meter::reset_limb_ops();
        crate::meter::reset_scan_bits();
        let start = Instant::now();
        let acc = unmetered_quadratic(black_box(n));
        let wall = start.elapsed();
        black_box(acc);
        Sample {
            denom_bytes: n,
            limb_denom: n as u64,
            text_row: false,
            floors: Floors {
                heap: na(PROBE_NA),
                limb: na(PROBE_NA),
                scan: na(PROBE_NA),
            },
            // The probe allocates nothing; this binary installs no counting
            // allocator, so zero is also the only honest reading.
            peak_heap: 0,
            segments: crate::meter::stack_segments(),
            limb: Some(crate::meter::limb_ops()),
            scan: Some(crate::meter::scan_bits()),
            wall,
        }
    };
    let s1 = sample(QUAD_PROBE_BASE);
    let s2 = sample(QUAD_PROBE_BASE * 2);
    assert!(
        s2.wall >= Duration::from_millis(MIN_JUDGED_WALL_MILLIS),
        "the probe's larger scale must outlast the wall judgment threshold \
         ({MIN_JUDGED_WALL_MILLIS} ms): measured {:?}; grow QUAD_PROBE_BASE",
        s2.wall
    );
    let cell = evaluate("unmetered_quadratic_probe", "probe", s1, s2);
    assert_eq!(
        cell.red,
        vec!["wall exponent"],
        "the criterion must read the unmetered quadratic red on exactly the wall exponent \
         (measured exponent {:.2}, walls {:?} -> {:?}): every counter column is blind to it",
        cell.wall_exp,
        cell.s1.wall,
        cell.s2.wall
    );
}
