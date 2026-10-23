use std::collections::BTreeSet;

use proptest::prelude::*;

use super::super::ceilings::{HEAP_INTERCEPT_BYTES, MAX_HEAP_BYTES_PER_INPUT_BYTE};
use super::super::compare::{capture, readings, BuildStamp};
use super::super::currency::{ByCurrency, Currency, Liveness};
use super::super::judge::evaluate_acceptance;
use super::super::ladder::Ladder;
use super::super::measure::{Model, Sample};
use super::super::render::ceiling_text;
use super::{
    archs_of_record, calibration, ceiling_thousandths, derive_ceilings, recalibrate, Calibration,
    CellCeilings, CEILING_RESOLUTION, CELL_CEILING_HEADROOM, TABLES,
};

/// The operation and family of the synthetic cell these tests calibrate.
const CELL: (&str, &str) = ("probe_op", "probe_family");

/// The input bytes of the synthetic cell's samples, in ladder order.
const DENOMS: [usize; 8] = [10, 20, 1_200, 2_400, 1_000, 2_000, 4_000, 8_000];

/// A sample of `denom` input bytes reading `heap` heap bytes and no other
/// currency, with no liveness obligation and no model.
fn heap_sample(denom: usize, heap: u64) -> Sample {
    Sample {
        denom_bytes: denom,
        exp_denom_bytes: denom,
        floors: ByCurrency::from_fn(|_| Liveness::NotApplicable {
            reason: "no liveness obligation in this probe",
        }),
        models: ByCurrency::from_fn(|_| None),
        readings: ByCurrency {
            heap: Some(heap),
            scan: None,
            touch: None,
            fuel: None,
        },
    }
}

/// A one-cell ladder of [`CELL`], measured on `arch`, whose samples read
/// `heap` at [`DENOMS`].
fn ladder(arch: &str, heap: [u64; 8]) -> Ladder {
    let sweep = |first: usize| {
        let (op, family) = CELL;
        vec![(
            op,
            family,
            heap_sample(DENOMS[first], heap[first]),
            heap_sample(DENOMS[first + 1], heap[first + 1]),
        )]
    };
    Ladder {
        measured_on: BuildStamp {
            debug_assertions: false,
            arch: arch.to_owned(),
            os: "test".to_owned(),
        },
        small: sweep(0),
        past_power: sweep(2),
        base: sweep(4),
        top: sweep(6),
    }
}

/// The heap reading over `units` units that sits `per_unit` per unit above
/// the intercept.
fn reading(units: usize, per_unit: f64) -> u64 {
    (HEAP_INTERCEPT_BYTES as f64 + per_unit * units as f64).round() as u64
}

/// Heap units per input byte under [`modeled`]'s model, unequal to one so
/// that the model's units differ from both of a sample's default
/// denominators, which are its input bytes here.
const MODEL_UNITS_PER_BYTE: usize = 3;

/// `ladder` with every sample's heap measured against a model of
/// [`MODEL_UNITS_PER_BYTE`] units per input byte, its trend axis unchanged.
fn modeled(mut ladder: Ladder) -> Ladder {
    let cells = ladder
        .small
        .iter_mut()
        .chain(&mut ladder.past_power)
        .chain(&mut ladder.base)
        .chain(&mut ladder.top);
    for (_, _, first, second) in cells {
        for sample in [first, second] {
            sample.models.heap = Some(Model {
                trend_units: sample.exp_denom_bytes as u64,
                constant_units: (MODEL_UNITS_PER_BYTE * sample.denom_bytes) as u64,
                ceiling: None,
            });
        }
    }
    ladder
}

/// `ladder`'s capture under source tree `tree`.
fn capture_text(ladder: &Ladder, tree: &str) -> String {
    let mut out = Vec::new();
    capture(ladder, tree, &mut out).expect("writing to a Vec succeeds");
    String::from_utf8(out).expect("a capture is UTF-8")
}

/// The ceiling a derivation gives [`CELL`], as a reading per unit.
fn derived(ladder: &Ladder, captures: &[(&str, String)]) -> f64 {
    let others: Vec<_> = captures
        .iter()
        .map(|(name, text)| (*name, readings(name, text).expect("the capture parses")))
        .collect();
    let entries =
        derive_ceilings(ladder, &others, Currency::Heap).expect("the derivation succeeds");
    assert_eq!(entries.len(), 1, "the synthetic ladder has one cell");
    entries[0].2 as f64 / CEILING_RESOLUTION as f64
}

/// Every per-cell table is strictly sorted by operation and then family, and
/// no two tables share a currency.
///
/// A ceiling lookup binary-searches its table, which finds every entry only
/// in a strictly sorted table; a duplicate cell would also be a second,
/// conflicting ceiling. [`super::cell_ceilings`] returns the first table for
/// a currency, so a second one would be silently ignored.
#[test]
fn tables_are_sorted_and_distinct() {
    let mut currencies = BTreeSet::new();
    for table in TABLES {
        for pair in table.entries.windows(2) {
            let ((op_a, family_a, _), (op_b, family_b, _)) = (pair[0], pair[1]);
            assert!(
                (op_a, family_a) < (op_b, family_b),
                "{}: {op_a} x {family_a} must sort strictly before {op_b} x {family_b}",
                table.currency.label(),
            );
        }
        assert!(
            currencies.insert(table.currency),
            "{}: two tables share a currency",
            table.currency.label(),
        );
    }
}

/// Every table names exactly its currency's architectures of record.
///
/// A table calibrated from fewer architectures stops holding for the omitted
/// one, whose runs then face the board-wide ceiling alone (or, for fuel,
/// which has none, are refused).
#[test]
fn every_table_names_its_architectures_of_record() {
    for table in TABLES {
        let named: BTreeSet<&str> = table.archs.iter().copied().collect();
        let of_record: BTreeSet<&str> = archs_of_record(table.currency).iter().copied().collect();
        assert_eq!(named, of_record, "the {} table", table.currency.label());
    }
}

/// A derived ceiling is the cell's worst reading per unit after the
/// intercept, times the headroom, rounded up to a thousandth, and clamped at
/// zero.
///
/// The ceiling must admit the worst sample with at least the stated headroom,
/// and with no more than one thousandth beyond it; a cell whose every reading
/// lies within the intercept gets a ceiling of zero, judged by the intercept
/// alone.
#[test]
fn ceilings_add_headroom_to_the_worst_sample_and_round_up() {
    let per_unit = [0.5, 1.0, 2.0, 2.5, 3.0, 2.5, 3.333, 1.0];
    let heap = std::array::from_fn(|i| reading(DENOMS[i], per_unit[i]));
    let worst = (0..DENOMS.len())
        .map(|i| (heap[i] as f64 - HEAP_INTERCEPT_BYTES as f64) / DENOMS[i] as f64)
        .fold(f64::NEG_INFINITY, f64::max);
    let ceiling = derived(&ladder("aarch64", heap), &[]);
    assert!(
        ceiling >= CELL_CEILING_HEADROOM * worst,
        "{ceiling} under the headroom above {worst}"
    );
    assert!(
        ceiling <= CELL_CEILING_HEADROOM * worst + 1.0 / CEILING_RESOLUTION as f64 + 1e-9,
        "{ceiling} rounds up by more than a thousandth"
    );

    let within_intercept = [0, 10, 700, 1_020, 512, 1_000, 1_024, 900];
    assert_eq!(derived(&ladder("aarch64", within_intercept), &[]), 0.0);
    assert_eq!(
        ceiling_thousandths(-3.7),
        0,
        "a negative worst clamps to zero"
    );
}

/// Merging another architecture's capture derives the ceiling from the
/// larger of the two readings at every sample.
///
/// The table must hold on every architecture of record, so a sample that
/// reads more on the other architecture sets the ceiling, and one that reads
/// less there changes nothing.
#[test]
fn merged_ceilings_take_the_larger_reading_per_sample() {
    let low = std::array::from_fn(|i| reading(DENOMS[i], 1.0));
    let mut high = low;
    high[3] = reading(DENOMS[3], 4.0);
    let here = ladder("aarch64", low);
    let there = ladder("x86_64", high);
    let raised = derived(&here, &[("x86", capture_text(&there, "tree-a"))]);
    assert!(raised >= CELL_CEILING_HEADROOM * 4.0, "{raised}");
    let unraised = derived(&there, &[("arm", capture_text(&here, "tree-a"))]);
    assert_eq!(
        raised, unraised,
        "the larger reading wins whichever side holds it"
    );
    let alone = derived(&here, &[]);
    assert!(
        alone < raised,
        "{alone} must stay below the merged {raised}"
    );
}

/// Recalibration refuses every capture it cannot merge soundly.
///
/// Each refusal names a way a merged table would be wrong: readings of other
/// code (another tree), of debug-assertion builds, or of another cell set or
/// other sizes.
#[test]
fn recalibration_refuses_unmergeable_captures() {
    let flat = std::array::from_fn(|i| reading(DENOMS[i], 1.0));
    let here = ladder("aarch64", flat);
    let there = ladder("x86_64", flat);
    let good = capture_text(&there, "tree-a");
    let refused = |captures: &[(&str, &str)]| {
        recalibrate(&here, "tree-a", captures, None)
            .expect_err("the calibration must refuse")
            .kind()
    };
    recalibrate(&here, "tree-a", &[("x86", &good)], None).expect("a sound merge succeeds");

    let other_tree = capture_text(&there, "tree-b");
    assert_eq!(
        refused(&[("x86", &other_tree)]),
        std::io::ErrorKind::InvalidData
    );
    let mut debug = ladder("x86_64", flat);
    debug.measured_on.debug_assertions = true;
    let debug = capture_text(&debug, "tree-a");
    assert_eq!(refused(&[("x86", &debug)]), std::io::ErrorKind::InvalidData);
    let derived = |text: &str| {
        let others = [("x86", readings("x86", text).expect("the capture parses"))];
        derive_ceilings(&here, &others, Currency::Heap).map(|_| ())
    };
    derived(&good).expect("a capture of the same cells and sizes merges");
    let resized = good.replace("\t8000\t", "\t8001\t");
    assert_ne!(resized, good, "the probe must move one sample's size");
    let refused = derived(&resized).expect_err("a capture at other sizes is refused");
    assert_eq!(refused.kind(), std::io::ErrorKind::InvalidData);
    let renamed = good.replace(CELL.1, "other_family");
    let refused = derived(&renamed).expect_err("a capture lacking the cell is refused");
    assert_eq!(refused.kind(), std::io::ErrorKind::InvalidData);
}

/// Recalibration refuses to write a heap table from any set of
/// architectures but the heap architectures of record.
///
/// A table calibrated from fewer architectures stops holding for the omitted
/// one, whose runs then face the board-wide ceiling alone. One calibrated
/// from more claims an architecture that the next calibration, taken from
/// the architectures of record, drops without notice.
#[test]
fn recalibration_refuses_architectures_off_the_record() {
    let flat = std::array::from_fn(|i| reading(DENOMS[i], 1.0));
    let here = ladder("aarch64", flat);
    let good = capture_text(&ladder("x86_64", flat), "tree-a");
    let third = capture_text(&ladder("riscv64", flat), "tree-a");
    for captures in [
        vec![],
        vec![("x86", good.as_str()), ("riscv", third.as_str())],
    ] {
        let refused =
            recalibrate(&here, "tree-a", &captures, None).expect_err("the calibration must refuse");
        assert_eq!(refused.kind(), std::io::ErrorKind::InvalidData);
    }
}

/// The heap table judged by every test below: [`CELL`] at a ceiling of 2.0
/// bytes per input byte.
static OWN_CEILING: CellCeilings = CellCeilings {
    currency: Currency::Heap,
    archs: &["aarch64"],
    provenance: None,
    entries: &[(CELL.0, CELL.1, 2_000)],
};

/// The red labels the acceptance judgment gives [`CELL`] when every sample
/// reads `per_unit` heap bytes per input byte above the intercept, judged
/// under `table`.
fn labels(per_unit: f64, table: Option<&'static CellCeilings>) -> Vec<&'static str> {
    let heap: [u64; 8] = std::array::from_fn(|i| reading(DENOMS[i], per_unit));
    let ladder = ladder("aarch64", heap);
    let calibration = Calibration {
        arch: "aarch64".to_owned(),
        tables: ByCurrency::from_fn(|currency| table.filter(|_| currency == Currency::Heap)),
    };
    let (_, _, b1, b2) = ladder.base[0].clone();
    let (_, _, t1, t2) = ladder.top[0].clone();
    let (base, top) = evaluate_acceptance(&calibration, CELL.0, CELL.1, (b1, b2), (t1, t2));
    let mut labels: Vec<&'static str> = base.red.into_iter().chain(top.red).collect();
    labels.sort_unstable();
    labels.dedup();
    labels
}

// The tighter-ceiling probe reads 3.0 bytes per byte, between the cell's own
// ceiling of 2.0 and the board-wide one.
const _: () = assert!(
    MAX_HEAP_BYTES_PER_INPUT_BYTE > 3.0,
    "the probe sits between the two ceilings"
);

/// The judge holds every sample to the tighter of the cell's own ceiling and
/// its outer bound, and names which one a sample exceeded.
///
/// A sample within both is green; one over the cell's own ceiling alone reads
/// `heap cell constant`, a move from its calibrated reading; one over the
/// board-wide ceiling reads `heap constant`. Without a table, only the outer
/// bound binds.
#[test]
fn judge_applies_the_tighter_ceiling_and_names_it() {
    assert!(labels(1.5, Some(&OWN_CEILING)).is_empty());
    assert_eq!(labels(3.0, Some(&OWN_CEILING)), ["heap cell constant"]);
    assert_eq!(
        labels(MAX_HEAP_BYTES_PER_INPUT_BYTE + 1.0, Some(&OWN_CEILING)),
        ["heap constant"]
    );
    assert!(
        labels(3.0, None).is_empty(),
        "without a table the outer bound alone binds"
    );
}

/// A table that holds no ceiling for a cell, because the cell is new since
/// the calibration, judges it red as uncalibrated rather than refusing to
/// judge, and its outer bound still binds.
///
/// A refusal would end the run before it writes its capture, and a
/// recalibration needs that capture from the other architecture of record.
#[test]
fn a_cell_its_table_omits_is_red_as_uncalibrated() {
    const OTHER_CELL: CellCeilings = CellCeilings {
        currency: Currency::Heap,
        archs: &["aarch64"],
        provenance: None,
        entries: &[("another_op", CELL.1, 2_000)],
    };
    assert_eq!(labels(1.5, Some(&OTHER_CELL)), ["heap cell uncalibrated"]);
    assert_eq!(
        labels(MAX_HEAP_BYTES_PER_INPUT_BYTE + 1.0, Some(&OTHER_CELL)),
        ["heap cell uncalibrated", "heap constant"]
    );
}

/// The rendered ceiling of a currency with a per-cell table names that table
/// on every architecture; a currency with no table anywhere names none.
///
/// On an architecture of record the text names the table's architectures;
/// on any other, where the outer bound judges alone, it names the table's
/// absence.
///
/// A reader of a run judged without its table would otherwise take the
/// board-wide ceiling for the whole judgment, or the table for one that
/// applied.
#[test]
fn ceiling_text_names_the_table_or_its_absence() {
    let elsewhere = calibration("no-such-arch");
    for (currency, _) in ByCurrency::from_fn(|_| ()).each() {
        let archs = archs_of_record(currency);
        let text = ceiling_text(currency, &elsewhere);
        let names_absence = text.contains(&format!(
            "no per-cell {} table for no-such-arch; the table is calibrated from {}",
            currency.label(),
            archs.join(" and ")
        ));
        assert_eq!(names_absence, !archs.is_empty(), "{text}");
        for arch in archs {
            let text = ceiling_text(currency, &calibration(arch));
            assert!(
                text.contains(&format!("(calibrated from {})", archs.join(" and "))),
                "{text}"
            );
        }
    }
}

proptest! {
    /// A ladder judges green under the table recalibrated from it, and any
    /// sample raised past its ceiling turns the cell red.
    ///
    /// This ties calibration and judgment together: the calibration's rule
    /// and the judge's comparison must agree on units, intercept, and
    /// rounding, or the board would be red the moment it is recalibrated, or
    /// green past its own ceilings. The samples carry a heap model whose
    /// units differ from their input bytes, so the two agree on units only by
    /// both reading the model's.
    #[test]
    fn a_recalibrated_ladder_judges_green_and_red_past_its_ceiling(
        per_unit in proptest::array::uniform8(0.0f64..10.0),
        raised in 0usize..8,
    ) {
        let units: [usize; 8] = DENOMS.map(|denom| MODEL_UNITS_PER_BYTE * denom);
        let heap: [u64; 8] = std::array::from_fn(|i| reading(units[i], per_unit[i]));
        let ladder = modeled(ladder("aarch64", heap));
        let entries = derive_ceilings(&ladder, &[], Currency::Heap).expect("no captures to refuse");
        let ceiling = entries[0].2;
        let table: &'static CellCeilings = Box::leak(Box::new(CellCeilings {
            currency: Currency::Heap,
            archs: &["aarch64"],
            provenance: None,
            entries: Box::leak(vec![(CELL.0, CELL.1, ceiling)].into_boxed_slice()),
        }));
        let calibration = Calibration {
            arch: "aarch64".to_owned(),
            tables: ByCurrency::from_fn(|c| (c == Currency::Heap).then_some(table)),
        };
        let judge = |heap: [u64; 8]| -> Vec<&'static str> {
            let ladder = modeled(self::ladder("aarch64", heap));
            let (_, _, b1, b2) = ladder.base[0].clone();
            let (_, _, t1, t2) = ladder.top[0].clone();
            let (_, _, s1, s2) = ladder.small[0].clone();
            let (_, _, p1, p2) = ladder.past_power[0].clone();
            let (base, top) = evaluate_acceptance(&calibration, CELL.0, CELL.1, (b1, b2), (t1, t2));
            let small = super::super::judge::evaluate_without_fit(&calibration, CELL.0, CELL.1, s1, s2);
            let past_power =
                super::super::judge::evaluate_without_fit(&calibration, CELL.0, CELL.1, p1, p2);
            base.red.into_iter().chain(top.red).chain(small.red).chain(past_power.red).collect()
        };
        let green = judge(heap);
        prop_assert!(
            !green.iter().any(|label| label.ends_with("constant")),
            "the calibrated ladder reads {green:?}"
        );
        let mut over = heap;
        let allowed = HEAP_INTERCEPT_BYTES as f64
            + ceiling as f64 / CEILING_RESOLUTION as f64 * units[raised] as f64;
        over[raised] = allowed.floor() as u64 + 1;
        let red = judge(over);
        prop_assert!(red.contains(&"heap cell constant"), "a sample past its ceiling reads {red:?}");
    }
}
