//! Tests for the board's judgment and liveness floors.

#[cfg(feature = "scan-meter")]
use crate::meter::dense;
#[cfg(any(feature = "scan-meter", feature = "touch-meter"))]
use crate::meter::Encoding;
#[cfg(feature = "touch-meter")]
use crate::Party;
#[cfg(any(feature = "scan-meter", feature = "touch-meter"))]
use crate::Version;

#[cfg(feature = "touch-meter")]
use super::judge::trend;
#[cfg(feature = "touch-meter")]
use super::{operand::value_content_bytes, MAX_SCALING_EXPONENT};
#[cfg(feature = "touch-meter")]
use crate::meter::cliff_comb;

/// Lift a meter-generated encoded event shape into a [`Version`].
#[cfg(any(feature = "scan-meter", feature = "touch-meter"))]
fn version_of(p: &Encoding) -> Version {
    p.version()
}

/// Sum the stored bits of `v` by direct slice indexing: real linear traversal
/// work that touches no metered primitive (no cursor, no builder, no
/// arithmetic, no allocation).
#[cfg(feature = "scan-meter")]
fn bypass_walk(v: &Version) -> usize {
    let bits = v.as_bits();
    (0..bits.len()).filter(|&i| bits.bit(i)).count()
}

/// A body that does its traversal outside the metered primitives reads green
/// under ceilings alone and red under the committed liveness floors.
///
/// A criterion of ceilings alone is vacuous against exactly this bypass; the
/// floors close it.
///
/// The probe walks a decoded dense spine by direct indexing, so every counter
/// column records ~nothing while real linear work runs. Both legs go through
/// [`evaluate`] with the probe's real counter readings; the only difference is
/// the declarations — all-NA (ceilings alone) versus the committed walk
/// convention (scan floored at one bit per encoded byte).
#[cfg(feature = "scan-meter")]
#[test]
fn bypassing_walk_is_green_under_ceilings_alone_and_red_under_floors() {
    use super::floors::{na, walk_floors};
    use super::judge::{evaluate, SCAN_FLOOR_TRIP};
    use super::measure::Sample;
    use super::{ByCurrency, Floors};

    const PROBE_NA: &str = "probe: the ceilings-alone leg declares no floors";
    fn na_floors(_encoded_bytes: usize) -> Floors {
        Floors {
            heap: na(PROBE_NA),
            segments: na(PROBE_NA),
            scan: na(PROBE_NA),
            touch: na(PROBE_NA),
        }
    }
    /// The committed walk convention with the touch column honestly undeclared:
    /// the probe folds no accumulator, and the leg under test is the scan
    /// floor.
    fn probe_walk_floors(encoded_bytes: usize) -> Floors {
        walk_floors(encoded_bytes, na(PROBE_NA))
    }

    let sample = |depth: usize, floors_of: fn(usize) -> Floors| -> Sample {
        let encoded = dense(depth);
        let v = version_of(&encoded);
        let n = encoded.bytes.len();
        crate::meter::reset_scan_bits();
        let ones = bypass_walk(&v);
        let scanned = crate::meter::scan_bits();
        assert!(ones > 0, "the bypass walk does real work over real bits");
        Sample {
            denom_bytes: n,
            exp_denom_bytes: n,
            floors: floors_of(n),
            fold_arity: None,
            declared_heap: None,
            readings: ByCurrency {
                heap: Some(0),
                segments: Some(0),
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

/// The flat-denominator shape's encoded-byte fit manufactures a superlinear
/// exponent out of measured flat per-tooth work.
///
/// The value-content fit reads the same measurements linear. This is the
/// tripwire the comb-scatter exponent re-denomination rests on: the comparison
/// sweep's accumulator work per tooth is flat across a tooth-count doubling (the
/// linear witness), the encoded denominator grows under x1.5 because the
/// fixed 1000-bit magnitude dominates it (the intercept premise), and the two
/// fits disagree by an exponent class on identical readings.
#[cfg(feature = "touch-meter")]
#[test]
fn flat_denominator_encoded_fit_manufactures_an_exponent() {
    let measure = |teeth: usize| -> (usize, usize, u64) {
        let v = version_of(&cliff_comb(1_000, teeth));
        let mut w = v.clone();
        w.tick(&Party::seed());
        let encoded = v.encode().len() + w.encode().len();
        let content = value_content_bytes(&v) + value_content_bytes(&w);
        // The sweep's per-tooth work is accumulator folds, so digit touches
        // provide the flat marginal quantity this tripwire needs.
        suanpan::touch_meter::reset();
        let ord = v.partial_cmp(&w);
        let ops = suanpan::touch_meter::touches();
        assert!(ord.is_some(), "a ticked counterpart stays comparable");
        (encoded, content, ops)
    };
    let (build1, content1, ops1) = measure(128);
    let (build2, content2, ops2) = measure(256);
    // The intercept premise and the content axis's liveness.
    let encoded_growth = build2 as f64 / build1 as f64;
    let content_growth = content2 as f64 / content1 as f64;
    assert!(
        encoded_growth < 1.5,
        "the encoded denominator must be intercept-dominated: grew x{encoded_growth:.2}"
    );
    assert!(
        (1.9..=2.1).contains(&content_growth),
        "the value content must track the doubled tooth count: grew x{content_growth:.2}"
    );
    // Per-tooth touch work stays flat under linear scaling.
    let per_tooth = (ops1 as f64 / 128.0, ops2 as f64 / 256.0);
    assert!(
        per_tooth.1 <= per_tooth.0 * 1.25,
        "per-tooth touch work must be flat across the doubling: {per_tooth:?}"
    );
    // The same readings, two fits, an exponent class apart.
    let encoded_fit = trend(&[(build1, ops1), (build2, ops2)]);
    let content_fit = trend(&[(content1, ops1), (content2, ops2)]);
    assert!(
        encoded_fit > 2.0 * MAX_SCALING_EXPONENT,
        "the encoded fit must manufacture a superlinear exponent from flat marginal work: \
         read {encoded_fit:.2}"
    );
    assert!(
        content_fit <= MAX_SCALING_EXPONENT,
        "the content fit must read the same measurements linear: read {content_fit:.2}"
    );
}

/// The exponent guards judge the denominator's ability to scale and the heap
/// reading's materiality, never the reading's growth.
///
/// The same amplifier-shaped readings read green where the operand pair cannot
/// scale (6 -> 7 bytes: the fit divides by a vanishing log) or where both heap
/// readings sit inside the flat allowance the constant leg already forgives,
/// and read red the moment the denominator honestly doubles or the readings
/// clear the allowance. Both directions pinned so neither guard can silently
/// widen into an exemption hole.
#[test]
fn exponent_guards_skip_noise_and_keep_real_amplifiers_red() {
    use super::floors::na;
    use super::judge::evaluate;
    use super::measure::Sample;
    use super::{ByCurrency, Floors, HEAP_FLAT_ALLOWANCE_BYTES};
    const PROBE_NA: &str = "probe: the exponent guards alone are under test";
    let sample = |denom: usize, heap: u64, scan: u64| -> Sample {
        Sample {
            denom_bytes: denom,
            exp_denom_bytes: denom,
            floors: Floors {
                heap: na(PROBE_NA),
                segments: na(PROBE_NA),
                scan: na(PROBE_NA),
                touch: na(PROBE_NA),
            },
            fold_arity: None,
            declared_heap: None,
            readings: ByCurrency {
                heap: Some(heap),
                segments: Some(0),
                scan: Some(scan),
                touch: None,
            },
        }
    };
    // A x5 scan growth over a denominator pair that cannot scale: the fit is
    // noise amplification, unjudged; the identical readings over an honestly
    // doubling pair are a real amplifier, red.
    let sub_scaling = evaluate(
        "guard_probe",
        "sub-scaling",
        sample(6, 0, 12),
        sample(7, 0, 60),
    );
    assert!(
        !sub_scaling.red.iter().any(|r| r.contains("scan exponent")),
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
        scaling.red.contains(&"scan exponent"),
        "the same readings over an honestly doubling denominator must stay red: {:?}",
        scaling.red
    );
    // A cubic-shaped heap growth entirely inside the flat allowance is
    // size-class noise, unjudged; the same shape clearing the allowance is
    // judged and red.
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
    // A probe pair straddling the allowance boundary manufactures an exponent:
    // the flat term the constant leg forgives deflates the base reading and
    // releases at the large one, so the fit measures the boundary, not a
    // scaling class. The straddling pair stays unjudged; the class is judged at
    // the next doubling, where both probes sit in the scaling regime (the
    // over-allowance probe above).
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

/// The acceptance judgment fits one exponent trend over all four measured
/// points, so a single generator lump cannot define the estimate, while a
/// genuine super-linearity bends every point and still reads red.
///
/// The exponent-policy tripwire, in both directions. The lump ladder is a
/// linear counter with one bumped point (the shape of a sparse counter over
/// an organic control rebuilt per scale): its first window's own two-point
/// fit reads over the ceiling — the pinned proof that the per-window ratio
/// would have manufactured a red — while the four-point trend reads it
/// linear, in both windows. The quadratic ladder grows as the square of the
/// denominator at every point: the trend reads it a full class over the
/// ceiling, red in both windows, so the policy cannot be softened into an
/// exemption for real amplifiers.
#[test]
fn acceptance_trend_absorbs_lumps_and_keeps_amplifiers_red() {
    use super::floors::na;
    use super::judge::{evaluate_acceptance, trend};
    use super::measure::Sample;
    use super::{ByCurrency, Floors, MAX_SCALING_EXPONENT};
    const PROBE_NA: &str = "probe: the exponent trend alone is under test";
    let sample = |denom: usize, scan: u64| -> Sample {
        Sample {
            denom_bytes: denom,
            exp_denom_bytes: denom,
            floors: Floors {
                heap: na(PROBE_NA),
                segments: na(PROBE_NA),
                scan: na(PROBE_NA),
                touch: na(PROBE_NA),
            },
            fold_arity: None,
            declared_heap: None,
            readings: ByCurrency {
                heap: Some(0),
                segments: Some(0),
                scan: Some(scan),
                touch: None,
            },
        }
    };
    // A linear counter (~2 per denominator byte) with the second point
    // bumped: the lump an organic family rebuilt per scale can hand any
    // sparse counter.
    let ladder = [(134usize, 226u64), (288, 708), (549, 860), (1117, 1720)];
    let window_fit = trend(&ladder[..2]);
    assert!(
        window_fit > MAX_SCALING_EXPONENT,
        "the tripwire's premise: the lumped window's own two-point fit must \
         read over the ceiling (read {window_fit:.2})"
    );
    let (lo, hi) = evaluate_acceptance(
        "trend_probe",
        "lump-ladder",
        (
            sample(ladder[0].0, ladder[0].1),
            sample(ladder[1].0, ladder[1].1),
        ),
        (
            sample(ladder[2].0, ladder[2].1),
            sample(ladder[3].0, ladder[3].1),
        ),
    );
    for (label, cell) in [("lo", &lo), ("hi", &hi)] {
        assert!(
            !cell.red.iter().any(|r| r.contains("scan exponent")),
            "a single lump must not define the four-point trend ({label}: {:?})",
            cell.red
        );
    }
    // The same denominators carrying genuinely quadratic work: every point
    // bends, and the trend stays red in both windows.
    let quadratic = |n: usize| (n * n / 100) as u64;
    let (lo, hi) = evaluate_acceptance(
        "trend_probe",
        "quadratic-ladder",
        (
            sample(ladder[0].0, quadratic(ladder[0].0)),
            sample(ladder[1].0, quadratic(ladder[1].0)),
        ),
        (
            sample(ladder[2].0, quadratic(ladder[2].0)),
            sample(ladder[3].0, quadratic(ladder[3].0)),
        ),
    );
    for (label, cell) in [("lo", &lo), ("hi", &hi)] {
        assert!(
            cell.red.contains(&"scan exponent"),
            "a genuine super-linearity must stay red through the trend \
             ({label}: {:?})",
            cell.red
        );
    }
}

/// The declared fold model admits the balanced reduction's log factor and
/// nothing steeper.
///
/// Three probes through [`evaluate`], all at the benign control's committed
/// arity pair (k 256 -> 512 over a x2.19 denominator): the pre-declaration
/// expected readings (scan exponent ~1.17, constant ~114 bits/B — the readings
/// that were red under the flat ceilings and are exactly the reduction's own
/// log factor) read green under the model; a quadratic fold (a left fold
/// re-walking its accumulator, exponent ~2 — the cheapest wrong artifact the
/// model could bless) stays exponent-red; and a fold whose per-level scan
/// constant regresses past the model's allowance reads constant-red even at an
/// admissible exponent. The ceiling-tightness leg pins the formula itself: at
/// every committed arity pair the declared exponent ceiling stays under 1.5, so
/// a quadratic's ~2 can never fit however the populations scale.
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
            floors: Floors {
                heap: na(PROBE_NA),
                segments: na(PROBE_NA),
                scan: na(PROBE_NA),
                touch: na(PROBE_NA),
            },
            fold_arity: Some(arity),
            declared_heap: None,
            readings: ByCurrency {
                heap: Some(0),
                segments: Some(0),
                scan: Some(scan),
                touch: None,
            },
        }
    };
    // The benign control's committed pair: k 256 -> 512, denominators 1322 ->
    // 2897 bytes.
    let (n1, n2, k1, k2) = (1_322usize, 2_897usize, 256u64, 512u64);
    let expected = evaluate(
        "fold_probe",
        "log-factor",
        sample(n1, k1, 132_200),
        sample(n2, k2, 330_500), // e ~1.17, ~114 bits/B: the reduction's own signature
    );
    assert!(
        !expected.red.iter().any(|r| r.starts_with("scan")),
        "the reduction's log factor must read green under its declared model: {:?}",
        expected.red
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
    // Ceiling tightness: at every committed arity pair (scatter and benign,
    // both scales, doubling denominators and beyond) the declared exponent
    // ceiling leaves no room for a quadratic.
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

/// A family-stated heap ceiling replaces the global constant without disabling
/// exponent judgment.
#[test]
fn family_stated_heap_ceiling_tightens_only_the_constant() {
    use super::floors::na;
    use super::judge::evaluate;
    use super::measure::Sample;
    use super::{ByCurrency, Floors};
    const PROBE_NA: &str = "probe: the family-stated heap ceiling alone is under test";
    let sample = |denom: usize, heap: u64| -> Sample {
        Sample {
            denom_bytes: denom,
            exp_denom_bytes: denom,
            floors: Floors {
                heap: na(PROBE_NA),
                segments: na(PROBE_NA),
                scan: na(PROBE_NA),
                touch: na(PROBE_NA),
            },
            fold_arity: None,
            declared_heap: Some(3.0),
            readings: ByCurrency {
                heap: Some(heap),
                segments: Some(0),
                scan: None,
                touch: None,
            },
        }
    };
    let within = evaluate(
        "heap_probe",
        "within",
        sample(10_000, 28_192),
        sample(20_000, 58_192),
    );
    assert!(
        within.red.is_empty(),
        "a flat 2.5 B/B reading is within the 3 B/B ceiling: {:?}",
        within.red
    );
    assert!(
        within.scores.heap.exp_judged,
        "declaring a constant must not disable exponent judgment"
    );
    let over = evaluate(
        "heap_probe",
        "over",
        sample(10_000, 38_193),
        sample(20_000, 68_193),
    );
    assert!(
        over.red.contains(&"heap constant"),
        "a reading above the family-stated ceiling must be red: {:?}",
        over.red
    );
}

// ─── the worst-case map ─────────────────────────────────────────────────────

/// An [`Entry`](super::worst::Entry) candidate with the modeled flag off, for
/// the argmax-kernel tests.
fn candidate(family: &'static str, value: f64) -> super::worst::Entry {
    super::worst::Entry {
        family,
        value,
        modeled: false,
    }
}

/// The argmax kernel records every exactly-tied family at the top, sorted by
/// name, and picks the runner-up strictly below the maximum.
///
/// A tie is recorded whole, so it can never make the ranking pin flappy; a
/// runner-up tie reports the name-order first entry, and a runner-up never
/// shadows a tied worst.
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
/// A currency every shape reads zero on folds to an empty worst set (rendered
/// `-`), and a currency only one shape drives has a worst but no runner-up — a
/// shape that does none of the work is not a runner-up at zero.
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
        worst: vec![candidate("alpha", worst_value)],
        runner_up: Some(candidate("beta", runner_up_value)),
    };
    let mut out = Vec::new();
    super::worst::row(&mut out, "probe_op", &column).expect("writing to a Vec succeeds");
    String::from_utf8(out).expect("the map renders UTF-8")
}

/// The `~near-tie` flag fires exactly under [`NEAR_TIE_RATIO`](super::NEAR_TIE_RATIO).
///
/// A margin strictly inside the band is flagged, and a margin at the boundary
/// or beyond is not: the flag marks rank orders a reader must not over-read,
/// and its boundary is the pinned constant, not a formatting accident.
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

/// The committed ranking pin stays well-formed against the live axes without a
/// board run.
///
/// Per sampling scale it names exactly the board's operation rows, in board
/// row order, and every pinned worst set is name-sorted, duplicate-free
/// rostered family names (or the dead-row `-`).
///
/// The cheap structural half of the pin's tamper evidence; the readings half —
/// the argmax itself — is the release-profile entry-compare (`just
/// worst-cases-pin`), because rankings derive from readings and dev readings
/// are never pinned.
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
        "the pin carries exactly one entry per operation per sampling scale"
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
