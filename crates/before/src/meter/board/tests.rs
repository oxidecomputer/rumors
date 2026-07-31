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
//! target. The join_all fold's scan-flatness pin (growth across a joint
//! doubling, over a build-liveness floor) lives here too, beside the board
//! row that carries it.

use crate::meter::{bigroot, dense, hugeleaf, reveal_comb, Packed};
use crate::{Party, Version};

use super::cell::assert_honest_text;
use super::operand::{
    mandatory_limbs_stream, mandatory_limbs_version, radix_units_party, radix_units_version,
};
use super::TEXT_BYTES_PER_RADIX_UNIT;
// The limb-priced tripwires read the touch counter, so they compile only
// with the `limb-meter` feature; these names have no ungated user.
#[cfg(feature = "limb-meter")]
use super::judge::exponent;
#[cfg(feature = "limb-meter")]
use super::operand::{stored_bases, value_content_bytes, version_output_bytes};
#[cfg(feature = "limb-meter")]
use super::{
    MAX_SCALING_EXPONENT, MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT, TEXT_PIPELINE_LIMB_OPS_PER_VALUE,
};
#[cfg(feature = "limb-meter")]
use crate::meter::cliff_comb;

/// Lift a meter-generated packed event shape into a [`Version`].
fn version_of(p: &Packed) -> Version {
    p.version()
}

/// The two limb-floor derivations split exactly where their rustdoc says.
///
/// On a plateau shape (equal wide leaves behind unit deltas) the
/// stream-derived floor counts the stored width once — the wide boundary
/// codes alone — while the tree-derived floor demands it per site, work no
/// conforming walk does; on a single wide leaf the two coincide (the one
/// stored code is the value), and on a small-value tree both are zero.
///
/// This is the derivation behind the walk rows (decode, rank, distance,
/// lag, tick) flooring at [`mandatory_limbs_stream`] while the
/// value-materializing parse rows keep [`mandatory_limbs_version`].
#[test]
fn limb_floor_derivations_split_on_plateaus_and_coincide_on_a_leaf() {
    // A single wide leaf: one stored code, which is the value itself.
    let huge = version_of(&hugeleaf(256));
    assert_eq!(mandatory_limbs_stream(&huge), 4, "256 bits = 4 limbs, once");
    assert_eq!(
        mandatory_limbs_version(&huge),
        4,
        "the tree stores the same single value"
    );
    // A plateau: 8 sites sharing one 2^256-scale minimum. The stream pays
    // wide codes only where the walk crosses the plateau boundary; the
    // decoded tree holds a wide base per site.
    let plateau = version_of(&reveal_comb(8, 256));
    let stream = mandatory_limbs_stream(&plateau);
    let tree = mandatory_limbs_version(&plateau);
    assert!(
        stream >= 4,
        "at least one wide boundary code is stored: got {stream}"
    );
    assert!(
        stream < tree,
        "the plateau's width is stored once but materialized per site:          stream {stream} must undercut tree {tree}"
    );
    assert!(
        tree >= 8 * 4,
        "the decoded tree holds a wide base per site: got {tree}"
    );
    // A small-value tree: no code and no base exceeds machine words.
    let small = version_of(&dense(64));
    assert_eq!(mandatory_limbs_stream(&small), 0);
    assert_eq!(mandatory_limbs_version(&small), 0);
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
/// site alone contributes ~33% on hugeleaf and ~17% on bigroot \[measured
/// 2026-07-31 — pipeline totals 752 and 24_399; 502 and 20_272 with the
/// radix recording deleted. The radix side's own contribution (250 and
/// 4_127) is unmoved since the 2026-07-24 record; the other sites' bigroot
/// share grew under the standing ceiling\]. A floor at ×0.85 (639 and
/// 20_739) sits above what the other two sites can reach, so a parse path
/// that stops recording trips it; the values' mandatory limbs alone cannot
/// separate that bypass (the encode-side arithmetic already covers them).
/// The bigroot separation margin is thin — a 20_739 floor over the
/// bypass's 20_272 reading — so a re-measure that moves the other sites
/// up must re-derive both readings before trusting the floor separates.
#[cfg(feature = "limb-meter")]
const DELEGATING_PARSE_LIMB_FLOORS: [(&str, u64); 2] = [("hugeleaf", 639), ("bigroot", 20_739)];

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
        let r = n_io as u64
            + radix_units_version(&v)
            + TEXT_PIPELINE_LIMB_OPS_PER_VALUE * stored_bases(&v).len() as u64;
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
/// scores ~1 limb per `R` unit on the magnitude families, over the pinned
/// κ (the pipeline term is negligible where conversion dominates, so `R`
/// is the schoolbook cost law there). This pin is the constant leg's
/// anti-softening tripwire: it fails if κ drifts up to where digit-by-digit
/// conversion passes.
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
        let r = n_io as u64
            + radix_units_version(&v)
            + TEXT_PIPELINE_LIMB_OPS_PER_VALUE * stored_bases(&v).len() as u64;
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
    use super::floors::na;
    use super::judge::evaluate;
    use super::measure::Sample;
    use super::{ByCurrency, Floors};

    let measure = |packed: &Packed| -> Sample {
        let v = version_of(packed);
        let s = v.to_string();
        let n_io = s.len() + version_output_bytes(&v);
        let r = n_io as u64
            + radix_units_version(&v)
            + TEXT_PIPELINE_LIMB_OPS_PER_VALUE * stored_bases(&v).len() as u64;
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
            exp_denom_bytes: n_io,
            limb_denom: r,
            text_row: true,
            floors: Floors {
                heap: na(PROBE_NA),
                segments: na(PROBE_NA),
                limb: na(PROBE_NA),
                scan: na(PROBE_NA),
                touch: na(PROBE_NA),
            },
            fold_arity: None,
            fold_search_bits: 0,
            heap_model: None,
            declared_heap: None,
            declared_limb: None,
            readings: ByCurrency {
                heap: Some(0),
                segments: Some(0),
                limb: Some(ops),
                scan: None,
                touch: None,
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
    use super::floors::{na, walk_floors};
    use super::judge::{evaluate, SCAN_FLOOR_TRIP};
    use super::measure::Sample;
    use super::{ByCurrency, Floors};

    const PROBE_NA: &str = "probe: the ceilings-alone leg declares no floors";
    fn na_floors(_packed_bytes: usize) -> Floors {
        Floors {
            heap: na(PROBE_NA),
            segments: na(PROBE_NA),
            limb: na(PROBE_NA),
            scan: na(PROBE_NA),
            touch: na(PROBE_NA),
        }
    }
    /// The committed walk convention with the touch column honestly
    /// undeclared: the probe folds no accumulator, and the leg under test
    /// is the scan floor.
    fn probe_walk_floors(packed_bytes: usize) -> Floors {
        walk_floors(packed_bytes, na(PROBE_NA))
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
            exp_denom_bytes: n,
            limb_denom: n as u64,
            text_row: false,
            floors: floors_of(n),
            fold_arity: None,
            fold_search_bits: 0,
            heap_model: None,
            declared_heap: None,
            declared_limb: None,
            readings: ByCurrency {
                heap: Some(0),
                segments: Some(0),
                limb: None,
                scan: Some(scanned),
                touch: None,
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
        sample(1_000, probe_walk_floors),
        sample(2_000, probe_walk_floors),
    );
    assert_eq!(
        floored.red,
        vec![SCAN_FLOOR_TRIP],
        "under the committed floors the bypass walk must read red on exactly the scan \
         floor: the meter is not watching its traversal"
    );
}

/// The join_all up-front overlap test reads scan-flat: a joint doubling
/// of accumulator and input count grows the fold's scan bits ≤ ×2.05,
/// over a liveness floor proving the meter watches the discipline.
///
/// \[Measured ×2.00, 33,036 → 66,060 bits — the re-pin landed with the
/// per-call index.\]
///
/// `Party::join_all` tests every input against the *fixed* accumulator up
/// front (the hand-back granularity the contract documents), through a
/// per-call `IdIndex` of the accumulator: the index build scans the
/// accumulator's tags twice, and each per-input test then costs O(input)
/// recorded reads, so a population of one-byte probes overlapping the
/// accumulator's right half behind its whole left shape prices linear in
/// the joint operands. A discipline that instead cursor-walks the fixed
/// accumulator per input reads ~×4 across the joint doubling — the packed
/// coding has no random access, so each such test skip-scans the whole
/// left shape — and trips this ceiling. The floor closes the vacuous
/// pass: a counter that stops watching the up-front discipline reads
/// under one full pass of the accumulator's stored bits. The board's
/// `party_join_all_overlap` row carries the same reading at the scales of
/// record; the cure's decision record lives in the design doc's §3 entry.
#[cfg(feature = "scan-meter")]
#[test]
fn join_all_overlap_upfront_test_reads_flat() {
    use super::family::{decode_party, overlap_fold_probe, overlap_mounted_pair};
    /// Ceiling on scan growth across the joint doubling: measured ×2.00
    /// (deterministic meter), with headroom for rounding only — a
    /// per-input accumulator re-walk reads ~×4.
    const MAX_SCAN_GROWTH: f64 = 2.05;
    let scan_at = |depth: usize| -> (u64, u64) {
        let shape = crate::meter::id_spine(depth, false);
        let (a_bytes, _) = overlap_mounted_pair(&shape.bytes);
        let mut acc = decode_party(&a_bytes);
        let probe = overlap_fold_probe();
        let count = a_bytes.len() / 64;
        let inputs: Vec<Party> = (0..count).map(|_| decode_party(&probe)).collect();
        crate::meter::reset_scan_bits();
        let back = acc
            .join_all(inputs)
            .expect_err("every probe overlaps the accumulator");
        assert_eq!(back.len(), count, "every probe is handed back");
        (crate::meter::scan_bits(), 8 * a_bytes.len() as u64)
    };
    let ((lo, lo_floor), (hi, _)) = (scan_at(4_096), scan_at(8_192));
    assert!(
        lo >= lo_floor,
        "the fold recorded {lo} scan bits under its {lo_floor}-bit liveness floor (one full \
         pass of the accumulator's stored bits): the scan meter is not watching the up-front \
         discipline"
    );
    let growth = hi as f64 / lo as f64;
    assert!(
        growth <= MAX_SCAN_GROWTH,
        "join_all's up-front overlap test reads x{growth:.2} scan growth across the joint \
         doubling ({lo} -> {hi} bits), over the pinned x{MAX_SCAN_GROWTH}: work scaling with \
         the fixed accumulator re-entered the per-input path"
    );
}

/// The flat-denominator shape's packed-byte fit manufactures a superlinear
/// exponent out of measured flat per-tooth work.
///
/// The value-content fit reads the same measurements linear. This is the
/// tripwire the comb-scatter exponent re-denomination rests on: the
/// comparison sweep's limb work per tooth is flat across a tooth-count
/// doubling (the honest linear witness), the packed denominator grows
/// under x1.5 because the fixed 1000-bit magnitude dominates it (the
/// intercept premise), and the two fits disagree by an exponent class on
/// identical readings.
#[cfg(feature = "limb-meter")]
#[test]
fn flat_denominator_packed_fit_manufactures_an_exponent() {
    let measure = |teeth: usize| -> (usize, usize, u64) {
        let v = version_of(&cliff_comb(1_000, teeth));
        let mut w = v.clone();
        w.tick(&Party::seed());
        let packed = v.encode().len() + w.encode().len();
        let content = value_content_bytes(&v) + value_content_bytes(&w);
        crate::meter::reset_limb_ops();
        let ord = v.partial_cmp(&w);
        let ops = crate::meter::limb_ops();
        assert!(ord.is_some(), "a ticked counterpart stays comparable");
        (packed, content, ops)
    };
    let (packed1, content1, ops1) = measure(128);
    let (packed2, content2, ops2) = measure(256);
    // The intercept premise and the content axis's liveness.
    let packed_growth = packed2 as f64 / packed1 as f64;
    let content_growth = content2 as f64 / content1 as f64;
    assert!(
        packed_growth < 1.5,
        "the packed denominator must be intercept-dominated: grew x{packed_growth:.2}"
    );
    assert!(
        (1.9..=2.1).contains(&content_growth),
        "the value content must track the doubled tooth count: grew x{content_growth:.2}"
    );
    // The honest linear witness: per-tooth limb work is flat.
    let per_tooth = (ops1 as f64 / 128.0, ops2 as f64 / 256.0);
    assert!(
        per_tooth.1 <= per_tooth.0 * 1.25,
        "per-tooth limb work must be flat across the doubling: {per_tooth:?}"
    );
    // The same readings, two fits, an exponent class apart.
    let packed_fit = exponent(ops1, ops2, packed1, packed2);
    let content_fit = exponent(ops1, ops2, content1, content2);
    assert!(
        packed_fit > 2.0 * MAX_SCALING_EXPONENT,
        "the packed fit must manufacture a superlinear exponent from flat marginal work: \
         read {packed_fit:.2}"
    );
    assert!(
        content_fit <= MAX_SCALING_EXPONENT,
        "the content fit must read the same measurements linear: read {content_fit:.2}"
    );
}

/// A genuinely quadratic-in-teeth walk still reads red against the value
/// content denominator: the re-denomination corrects the fit's axis,
/// never the criterion's teeth.
///
/// The probe does one metered accumulator pass per tooth over all earlier
/// teeth — Theta(teeth^2) limb ops on a content-linear operand — and its
/// content fit lands a full exponent class over the ceiling.
#[cfg(feature = "limb-meter")]
#[test]
fn quadratic_in_teeth_work_reads_red_against_the_content_denominator() {
    use crate::codec::Base;
    let measure = |teeth: usize| -> (usize, u64) {
        let v = version_of(&cliff_comb(1_000, teeth));
        let content = value_content_bytes(&v);
        crate::meter::reset_limb_ops();
        let mut acc = Base::from(0u32);
        for i in 0..teeth {
            for _ in 0..i {
                acc += 1u32;
            }
        }
        let ops = crate::meter::limb_ops();
        assert!(acc > Base::from(0u32), "the probe's fold is live");
        (content, ops)
    };
    let (content1, ops1) = measure(128);
    let (content2, ops2) = measure(256);
    let content_fit = exponent(ops1, ops2, content1, content2);
    assert!(
        content_fit > MAX_SCALING_EXPONENT,
        "a quadratic-in-teeth probe must read red against the content denominator: \
         read {content_fit:.2}"
    );
}

/// The exponent guards judge the denominator's ability to scale and the
/// heap reading's materiality, never the reading's growth.
///
/// The same amplifier-shaped readings read green where the operand pair
/// cannot scale (6 -> 7 bytes: the fit divides by a vanishing log) or
/// where both heap readings sit inside the flat allowance the constant
/// leg already forgives, and read red the moment the denominator honestly
/// doubles or the readings clear the allowance. Both directions pinned so
/// neither guard can silently widen into an exemption hole.
#[test]
fn exponent_guards_skip_noise_and_keep_real_amplifiers_red() {
    use super::floors::na;
    use super::judge::evaluate;
    use super::measure::Sample;
    use super::{ByCurrency, Floors, HEAP_FLAT_ALLOWANCE_BYTES};
    const PROBE_NA: &str = "probe: the exponent guards alone are under test";
    let sample = |denom: usize, heap: u64, limb: u64| -> Sample {
        Sample {
            denom_bytes: denom,
            exp_denom_bytes: denom,
            limb_denom: denom as u64,
            text_row: false,
            floors: Floors {
                heap: na(PROBE_NA),
                segments: na(PROBE_NA),
                limb: na(PROBE_NA),
                scan: na(PROBE_NA),
                touch: na(PROBE_NA),
            },
            fold_arity: None,
            fold_search_bits: 0,
            heap_model: None,
            declared_heap: None,
            declared_limb: None,
            readings: ByCurrency {
                heap: Some(heap),
                segments: Some(0),
                limb: Some(limb),
                scan: None,
                touch: None,
            },
        }
    };
    // A x5 limb growth over a denominator pair that cannot scale: the fit
    // is noise amplification, unjudged; the identical readings over an
    // honestly doubling pair are a real amplifier, red.
    let sub_scaling = evaluate(
        "guard_probe",
        "sub-scaling",
        sample(6, 0, 12),
        sample(7, 0, 60),
    );
    assert!(
        !sub_scaling.red.iter().any(|r| r.contains("limb exponent")),
        "a non-scaling denominator pair must leave the exponent unjudged: {:?}",
        sub_scaling.red
    );
    let scaling = evaluate(
        "guard_probe",
        "scaling",
        sample(6, 0, 12),
        sample(12, 0, 60),
    );
    assert!(
        scaling.red.contains(&"limb exponent"),
        "the same readings over an honestly doubling denominator must stay red: {:?}",
        scaling.red
    );
    // A cubic-shaped heap growth entirely inside the flat allowance is
    // size-class noise, unjudged; the same shape clearing the allowance
    // is judged and red.
    let sub_allowance = evaluate(
        "guard_probe",
        "sub-allowance",
        sample(100, 100, 0),
        sample(200, 800, 0),
    );
    assert!(
        !sub_allowance
            .red
            .iter()
            .any(|r| r.contains("heap exponent")),
        "sub-allowance heap readings must leave the exponent unjudged: {:?}",
        sub_allowance.red
    );
    let over_allowance = evaluate(
        "guard_probe",
        "over-allowance",
        sample(100_000, HEAP_FLAT_ALLOWANCE_BYTES as u64 + 1_000, 0),
        sample(200_000, 8 * (HEAP_FLAT_ALLOWANCE_BYTES as u64 + 1_000), 0),
    );
    assert!(
        over_allowance.red.contains(&"heap exponent"),
        "heap readings clearing the allowance must be judged and red: {:?}",
        over_allowance.red
    );
    // A probe pair straddling the allowance boundary manufactures an
    // exponent: the flat term the constant leg forgives deflates the
    // base reading and releases at the large one, so the fit measures
    // the boundary, not a scaling class. The straddling pair stays
    // unjudged; the class is judged at the next doubling, where both
    // probes sit in the scaling regime (the over-allowance probe above).
    let straddling = evaluate(
        "guard_probe",
        "straddle-allowance",
        sample(100_000, HEAP_FLAT_ALLOWANCE_BYTES as u64 / 2, 0),
        sample(200_000, 3 * HEAP_FLAT_ALLOWANCE_BYTES as u64, 0),
    );
    assert!(
        !straddling.red.iter().any(|r| r.contains("heap exponent")),
        "a probe pair straddling the flat allowance must leave the heap \
         exponent unjudged: {:?}",
        straddling.red
    );
}

/// The declared fold model admits the balanced reduction's log factor
/// and nothing steeper.
///
/// Three probes through [`evaluate`], all at the benign control's
/// committed arity pair (k 256 -> 512 over a x2.19 denominator): the
/// pre-declaration honest readings (scan exponent ~1.17, constant
/// ~114 bits/B — the readings that were red under the flat ceilings and
/// are exactly the reduction's own log factor) read green under the
/// model; a quadratic fold (a left fold re-walking its accumulator,
/// exponent ~2 — the cheapest wrong artifact the model could bless)
/// stays exponent-red; and a fold whose per-level scan constant
/// regresses past the model's allowance reads constant-red even at an
/// admissible exponent. The ceiling-tightness leg pins the formula
/// itself: at every committed arity pair the declared exponent ceiling
/// stays under 1.5, so a quadratic's ~2 can never fit however the
/// populations scale.
#[cfg(feature = "scan-meter")]
#[test]
fn declared_fold_model_admits_the_log_factor_and_rejects_quadratic() {
    use super::ceilings::fold_exponent_ceiling;
    use super::floors::na;
    use super::judge::evaluate;
    use super::measure::Sample;
    use super::{ByCurrency, Floors, FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL};
    const PROBE_NA: &str = "probe: the declared fold model alone is under test";
    let sample = |denom: usize, arity: u64, scan: u64| -> Sample {
        Sample {
            denom_bytes: denom,
            exp_denom_bytes: denom,
            limb_denom: denom as u64,
            text_row: false,
            floors: Floors {
                heap: na(PROBE_NA),
                segments: na(PROBE_NA),
                limb: na(PROBE_NA),
                scan: na(PROBE_NA),
                touch: na(PROBE_NA),
            },
            fold_arity: Some(arity),
            fold_search_bits: 0,
            heap_model: None,
            declared_heap: None,
            declared_limb: None,
            readings: ByCurrency {
                heap: Some(0),
                segments: Some(0),
                limb: None,
                scan: Some(scan),
                touch: None,
            },
        }
    };
    // The benign control's committed pair: k 256 -> 512, denominators
    // 1322 -> 2897 bytes.
    let (n1, n2, k1, k2) = (1_322usize, 2_897usize, 256u64, 512u64);
    let honest = evaluate(
        "fold_probe",
        "log-factor",
        sample(n1, k1, 132_200),
        sample(n2, k2, 330_500), // e ~1.17, ~114 bits/B: the reduction's own signature
    );
    assert!(
        !honest.red.iter().any(|r| r.starts_with("scan")),
        "the reduction's log factor must read green under its declared model: {:?}",
        honest.red
    );
    let quadratic = evaluate(
        "fold_probe",
        "quadratic",
        sample(n1, k1, 132_200),
        sample(n2, k2, 634_600), // e ~2.0: a left fold re-walking its accumulator
    );
    assert!(
        quadratic.red.contains(&"scan exponent"),
        "a quadratic fold must stay exponent-red under the declared model: {:?}",
        quadratic.red
    );
    let fat_constant = evaluate(
        "fold_probe",
        "fat-constant",
        sample(n1, k1, 150_900),
        sample(n2, k2, 362_125), // e ~1.12, but 125 bits/B over the 12/level model
    );
    assert!(
        fat_constant.red.contains(&"scan constant"),
        "a per-level constant regression must read constant-red: {:?}",
        fat_constant.red
    );
    // The party fold's search allowance admits the binary search's own
    // probe bound and nothing looser: readings at the weave family's
    // committed proportions (fold model + allowance, ~7% under) are
    // green, and a search discipline paying twice the bound — linear
    // re-probing where the partition search is logarithmic — reads red.
    let search = |denom: usize, arity: u64, search_bits: u64, scan: u64| -> Sample {
        let mut s = sample(denom, arity, scan);
        s.fold_search_bits = search_bits;
        s
    };
    let searched = evaluate(
        "fold_probe",
        "searched",
        search(6_144, 16, 979_200, 1_370_000),
        search(12_288, 16, 2_207_520, 2_740_000), // ~223 bits/B: the weave reading
    );
    assert!(
        !searched.red.iter().any(|r| r.starts_with("scan")),
        "the indexed fold's searches must read green under their declared allowance: {:?}",
        searched.red
    );
    let over_searched = evaluate(
        "fold_probe",
        "over-searched",
        search(6_144, 16, 979_200, 2_400_000),
        search(12_288, 16, 2_207_520, 5_150_000), // ~2x the allowance: a regressed search
    );
    assert!(
        over_searched.red.contains(&"scan constant"),
        "a search paying past its declared allowance must read red: {:?}",
        over_searched.red
    );
    // Ceiling tightness: at every committed arity pair (scatter and
    // benign, both scales, doubling denominators and beyond) the
    // declared exponent ceiling leaves no room for a quadratic.
    for (k1, k2, n1, n2) in [
        (256u64, 512u64, 1_322usize, 2_897usize),
        (1_024, 2_048, 5_120, 10_240),
        (1_024, 2_048, 3_825, 8_363),
        (4_096, 8_192, 20_480, 40_960),
    ] {
        let ceiling = fold_exponent_ceiling(k1, k2, n1, n2);
        assert!(
            ceiling < 1.5,
            "the declared fold exponent ceiling must stay far under a quadratic's ~2: \
             read {ceiling:.3} at k {k1}->{k2}, n {n1}->{n2}"
        );
        assert!(
            FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL * (2.0 * k2 as f64).log2()
                < super::MAX_SCAN_BITS_PER_INPUT_BYTE * 2.0,
            "the declared scan model must stay within the flat ceiling's own order at \
             committed arities"
        );
    }
}

/// The declared capacity model bands the projection's peak on both sides
/// and retires the unjudgeable exponent fit.
///
/// Probes through [`evaluate`] at the comb-scatter cross's committed
/// geometry (the model reads 68 160 -> 176 256 B there): the ratified
/// profile (measured/model 1.005–1.017 at every probed point) is green;
/// a regressed builder — an unanchored doubling chain or an extra buffer
/// copy, reading 2x the model — is red on the model ceiling; an improved
/// builder reading half the model trips the stale-model floor, forcing a
/// deliberate re-declaration; and the heap exponent stays unjudged (the
/// chain's power-of-two quantization is why the fit lies), so no k-step
/// straddle can re-manufacture the old exponent red.
#[test]
fn declared_capacity_model_bands_the_projection_peak() {
    use super::floors::na;
    use super::judge::evaluate;
    use super::measure::Sample;
    use super::{ByCurrency, Floors};
    const PROBE_NA: &str = "probe: the declared capacity model alone is under test";
    let sample = |denom: usize, model: f64, heap: u64| -> Sample {
        Sample {
            denom_bytes: denom,
            exp_denom_bytes: denom,
            limb_denom: denom as u64,
            text_row: false,
            floors: Floors {
                heap: na(PROBE_NA),
                segments: na(PROBE_NA),
                limb: na(PROBE_NA),
                scan: na(PROBE_NA),
                touch: na(PROBE_NA),
            },
            fold_arity: None,
            fold_search_bits: 0,
            heap_model: Some(model),
            declared_heap: None,
            declared_limb: None,
            readings: ByCurrency {
                heap: Some(heap),
                segments: Some(0),
                limb: None,
                scan: None,
                touch: None,
            },
        }
    };
    // The committed geometry: models 68 160 and 176 256 B at denominators
    // 32 814 and 65 126.
    let (n1, n2, m1, m2) = (32_814usize, 65_126usize, 68_160.0, 176_256.0);
    let ratified = evaluate(
        "capacity_probe",
        "ratified",
        sample(n1, m1, 68_952),  // measured/model 1.012: the ratified profile
        sample(n2, m2, 177_824), // 1.009
    );
    assert!(
        ratified.red.is_empty(),
        "the ratified capacity profile must read green: {:?}",
        ratified.red
    );
    assert!(
        !ratified.scores.heap.exp_judged,
        "a capacity-model cell's heap exponent fit must stay unjudged"
    );
    let regressed = evaluate(
        "capacity_probe",
        "regressed",
        sample(n1, m1, (m1 * 2.0) as u64),
        sample(n2, m2, (m2 * 2.0) as u64),
    );
    assert!(
        regressed.red.contains(&"heap capacity-model ceiling"),
        "a builder at twice the chain's peak must read red on the model ceiling: {:?}",
        regressed.red
    );
    let improved = evaluate(
        "capacity_probe",
        "improved",
        sample(n1, m1, (m1 * 0.5) as u64),
        sample(n2, m2, (m2 * 0.5) as u64),
    );
    assert!(
        improved
            .red
            .contains(&"heap capacity-model floor (stale model)"),
        "a builder under half the model must trip the stale-model floor: {:?}",
        improved.red
    );
}

/// Every declared-model bench rider names a live board cell that
/// actually carries a declared model, so the rider census cannot go
/// stale.
///
/// A cure that dissolves a cell's model must retire its rider in
/// the same change, and a rider can never point at an undeclared cell.
#[test]
fn bench_riders_name_declared_model_cells() {
    use super::family::FamilyData;
    use super::{bench_cells, BenchMode, BOARD_DECLARED_BENCH_RIDERS};
    use crate::meter::registry::FamilyId;
    let cells: std::collections::BTreeSet<(String, String)> = bench_cells(0.02, BenchMode::Full)
        .into_iter()
        .map(|cell| (cell.op.to_owned(), cell.family.to_owned()))
        .collect();
    for (op_name, family_name) in BOARD_DECLARED_BENCH_RIDERS {
        assert!(
            cells.contains(&((*op_name).to_owned(), (*family_name).to_owned())),
            "rider {op_name}/{family_name} names no live board cell"
        );
        let kind = FamilyId::board()
            .find(|&kind| kind.name() == *family_name)
            .unwrap_or_else(|| {
                panic!("rider family {family_name} is not on the registry's board roster")
            });
        let family = FamilyData::build(kind, 0.02, 0);
        let op = super::ops::ops()
            .into_iter()
            .find(|op| op.name == *op_name)
            .unwrap_or_else(|| panic!("rider op {op_name} is not a board row"));
        let cell = (op.prepare)(&family)
            .unwrap_or_else(|| panic!("rider {op_name}/{family_name} does not prepare"));
        assert!(
            cell.declared_heap.is_some() || cell.declared_limb.is_some(),
            "rider {op_name}/{family_name} carries no declared model: a rider exists to \
             keep a modeled cell's wall leg judged, so either declare the model at the \
             cell or retire the rider"
        );
    }
}

// ─── the worst-case map ─────────────────────────────────────────────────────

/// An [`Entry`](super::worst::Entry) candidate with the modeled flag off,
/// for the argmax-kernel tests.
fn candidate(family: &'static str, value: f64) -> super::worst::Entry {
    super::worst::Entry {
        family,
        value,
        modeled: false,
    }
}

/// The argmax kernel records every exactly-tied family at the top, sorted
/// by name, and picks the runner-up strictly below the maximum.
///
/// A tie is recorded whole, so it can never make the ranking pin flappy;
/// a runner-up tie reports the name-order first entry, and a runner-up
/// never shadows a tied worst.
#[test]
fn worst_rank_records_ties_whole_and_runner_up_strictly_below() {
    let (worst, runner_up) = super::worst::rank(vec![
        candidate("beta", 4.0),
        candidate("alpha", 4.0),
        candidate("delta", 2.0),
        candidate("gamma", 2.0),
        candidate("zeta", 1.0),
    ]);
    let names: Vec<&str> = worst.iter().map(|e| e.family).collect();
    assert_eq!(
        names,
        ["alpha", "beta"],
        "tied maxima, in family-name order"
    );
    let runner_up = runner_up.expect("entries exist strictly below the maximum");
    assert_eq!(
        runner_up.family, "delta",
        "the runner-up is the best entry strictly below the maximum, name-order first on a tie"
    );
    assert_eq!(runner_up.value, 2.0);
}

/// Zero readings never place in the argmax.
///
/// A currency every shape reads zero on folds to an empty worst set
/// (rendered `-`), and a currency only one shape drives has a worst but
/// no runner-up — a shape that does none of the work is not a runner-up
/// at zero.
#[test]
fn worst_rank_excludes_zero_readings() {
    let (worst, runner_up) =
        super::worst::rank(vec![candidate("alpha", 0.0), candidate("beta", 0.0)]);
    assert!(worst.is_empty(), "an all-zero currency is dead on the row");
    assert!(runner_up.is_none());
    let (worst, runner_up) =
        super::worst::rank(vec![candidate("alpha", 1.5), candidate("beta", 0.0)]);
    let names: Vec<&str> = worst.iter().map(|e| e.family).collect();
    assert_eq!(names, ["alpha"]);
    assert!(
        runner_up.is_none(),
        "a zero reading must not surface as a runner-up"
    );
}

/// Render one worst-map row for a two-family column at the given margin.
fn rendered_row(worst_value: f64, runner_up_value: f64) -> String {
    let column = super::worst::CurrencyWorst {
        currency: super::Currency::Touch,
        off: false,
        per_r: false,
        worst: vec![candidate("alpha", worst_value)],
        runner_up: Some(candidate("beta", runner_up_value)),
    };
    let mut out = Vec::new();
    super::worst::row(&mut out, "probe_op", &column).expect("writing to a Vec succeeds");
    String::from_utf8(out).expect("the map renders UTF-8")
}

/// The `~near-tie` flag fires exactly under [`NEAR_TIE_RATIO`](super::NEAR_TIE_RATIO).
///
/// A margin strictly inside the band is flagged, and a margin at the
/// boundary or beyond is not: the flag marks rank orders a reader must
/// not over-read, and its boundary is the pinned constant, not a
/// formatting accident.
#[test]
fn worst_row_flags_near_ties_strictly_under_the_ratio() {
    assert!(
        rendered_row(1.2, 1.0).contains("~near-tie"),
        "a margin inside the band must be flagged"
    );
    assert!(
        !rendered_row(super::NEAR_TIE_RATIO, 1.0).contains("~near-tie"),
        "a margin exactly at the ratio is outside the band"
    );
    assert!(
        !rendered_row(2.0, 1.0).contains("~near-tie"),
        "a clear margin must not be flagged"
    );
}

/// The committed ranking pin stays well-formed against the live axes
/// without a board run.
///
/// Per scale of record it names exactly the board's operation rows, in
/// board row order, and every pinned worst set is name-sorted,
/// duplicate-free rostered family names (or the dead-row `-`).
///
/// The cheap structural half of the pin's tamper evidence; the readings
/// half — the argmax itself — is the release-profile entry-compare
/// (`just worst-cases-pin`), because rankings derive from readings and
/// dev readings are never pinned.
#[test]
fn worst_rankings_pin_is_well_formed() {
    use super::worst::WORST_RANKINGS;
    use crate::meter::registry::FamilyId;
    let ops: Vec<&str> = super::ops::ops().into_iter().map(|op| op.name).collect();
    let families: std::collections::BTreeSet<&str> =
        FamilyId::board().map(|kind| kind.name()).collect();
    for (label, _) in super::worst::WORST_MAP_SCALES {
        let pinned: Vec<&str> = WORST_RANKINGS
            .iter()
            .filter(|(scale, _, _)| *scale == label)
            .map(|(_, op, _)| *op)
            .collect();
        assert_eq!(
            pinned, ops,
            "the {label}-scale pin must name exactly the board's operation rows, in board \
             row order"
        );
    }
    assert_eq!(
        WORST_RANKINGS.len(),
        ops.len() * super::worst::WORST_MAP_SCALES.len(),
        "the pin carries exactly one entry per operation per scale of record"
    );
    for (scale, op, columns) in WORST_RANKINGS {
        for worst in columns {
            if *worst == "-" {
                continue;
            }
            let names: Vec<&str> = worst.split(',').collect();
            let mut sorted = names.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(
                names, sorted,
                "{op} at the {scale} scale: a pinned worst set is name-sorted and \
                 duplicate-free"
            );
            for name in names {
                assert!(
                    families.contains(name),
                    "{op} at the {scale} scale pins {name}, which is not on the registry's \
                     board roster"
                );
            }
        }
    }
}
