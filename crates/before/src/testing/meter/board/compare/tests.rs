//! Checks that the board comparison reads captures exactly and classifies each
//! difference by how it changes with input size.

use proptest::prelude::*;

use super::{
    compare, compare_captures, currencies, parse, write_capture, BuildStamp, Capture, Outcome,
    Point, Row, LADDER_SIZES,
};

/// Render `capture` in the capture format.
fn text(capture: &Capture) -> String {
    let mut out = Vec::new();
    write_capture(capture, &mut out).expect("writing to a Vec cannot fail");
    String::from_utf8(out).expect("a capture is UTF-8")
}

/// Write both captures to text, parse them back, and compare them: the
/// outcomes, in the after capture's row order.
fn outcomes(before: &Capture, after: &Capture) -> Vec<Outcome> {
    let before = parse("before", &text(before)).expect("a written capture parses");
    let after = parse("after", &text(after)).expect("a written capture parses");
    compare_captures(before, after)
        .into_iter()
        .map(|compared| compared.outcome)
        .collect()
}

/// A capture of `rows`, each a distinct cell and currency with the given axes
/// and readings, under this build's stamp and a fixed source tree.
///
/// Each point's units are three times its axis, as under a model of three
/// constant units per trend unit, so that a round trip that confused the two
/// would not pass.
fn capture_of(rows: &[([u64; LADDER_SIZES], [u64; LADDER_SIZES])]) -> Capture {
    let currencies = currencies();
    Capture {
        measured_on: BuildStamp::current(),
        source_tree: "tree-a".to_owned(),
        rows: rows
            .iter()
            .enumerate()
            .map(|(index, (axes, readings))| Row {
                op: format!("op{}", index / currencies.len()),
                family: "family".to_owned(),
                currency: currencies[index % currencies.len()],
                points: std::array::from_fn(|size| Point {
                    axis: axes[size],
                    units: 3 * axes[size],
                    reading: readings[size],
                }),
            })
            .collect(),
    }
}

/// The largest proportional change the property applies, in reading units per
/// axis unit; base readings leave room for a saving this large.
const MAX_RATE: i64 = 8;

/// The largest fixed shift the property applies; base readings leave room for
/// a saving this large.
const MAX_SHIFT: i64 = 1_000;

/// One row's axes, shaped like the board's ladder, and base readings large
/// enough to absorb any applied saving.
///
/// The axes are a small-input pair at or below the base size, a past-power
/// size between the base size and its double and that size's double, then a
/// size, its double, four times it, and eight times it.
fn ladder_row() -> impl Strategy<Value = ([u64; LADDER_SIZES], [u64; LADDER_SIZES])> {
    (1u64..1_000_000)
        .prop_flat_map(|base| (Just(base), 1..=base, 1..=base, base..=2 * base))
        .prop_flat_map(|(base, small, double, past)| {
            let (small, double) = (small.min(double), small.max(double));
            let axes = [
                small,
                double,
                past,
                2 * past,
                base,
                2 * base,
                4 * base,
                8 * base,
            ];
            let readings = axes.map(|axis| {
                let floor = MAX_RATE as u64 * axis + MAX_SHIFT as u64;
                floor..floor + 1_000_000
            });
            (Just(axes), readings)
        })
}

/// Apply `change` to every reading of every row: `change(axis)` is the
/// difference added at that size.
fn changed(capture: &Capture, change: impl Fn(u64) -> i64) -> Capture {
    let mut after = capture.clone();
    for row in &mut after.rows {
        for point in &mut row.points {
            point.reading = point
                .reading
                .checked_add_signed(change(point.axis))
                .expect("base readings absorb every applied saving");
        }
    }
    after
}

proptest! {
    /// Through the capture text, identical captures compare unchanged, a fixed
    /// shift compares constant, and a proportional change compares growing.
    ///
    /// The constant row's shift is the one applied, and the growing row's slope
    /// is exactly the applied rate.
    ///
    /// These are the three readings a rounded comparison of rendered boards
    /// confuses: a fixed shift moves a fitted exponent, and a proportional
    /// change can leave it in place. The capture round trip is asserted too,
    /// since a lossy writer would make every classification meaningless.
    #[test]
    fn differences_classify_by_how_they_scale(
        rows in prop::collection::vec(ladder_row(), 1..8),
        shift in prop_oneof![-MAX_SHIFT..=-1, 1..=MAX_SHIFT],
        rate in prop_oneof![-MAX_RATE..=-1, 1..=MAX_RATE],
    ) {
        let before = capture_of(&rows);
        prop_assert_eq!(&parse("before", &text(&before)).expect("a written capture parses"), &before);

        let unchanged = outcomes(&before, &before);
        prop_assert!(unchanged.iter().all(|&outcome| outcome == Outcome::Unchanged), "{:?}", unchanged);

        let constant = outcomes(&before, &changed(&before, |_| shift));
        let expected = Outcome::Constant { shift: i128::from(shift) };
        prop_assert!(constant.iter().all(|&outcome| outcome == expected), "{:?}", constant);

        let growing = outcomes(&before, &changed(&before, |axis| rate * axis as i64));
        let expected = Outcome::Growing { slope: Some(rate as f64) };
        prop_assert!(growing.iter().all(|&outcome| outcome == expected), "{:?}", growing);
    }

    /// Every proper prefix of a capture is refused, so a capture cut short by
    /// a failed run can never be read as a smaller board or a smaller reading.
    #[test]
    fn a_truncated_capture_is_refused(rows in prop::collection::vec(ladder_row(), 1..4)) {
        let whole = text(&capture_of(&rows));
        for cut in 0..whole.len() {
            prop_assert!(parse("cut", &whole[..cut]).is_err(), "prefix of {} bytes parsed", cut);
        }
    }
}

/// A fixed 112-byte block that appears only at the largest size reads
/// uneven, never constant or growing: four sizes cannot tell such a step from
/// growth that registers only there, so the comparison names neither.
#[test]
fn a_step_at_the_largest_size_reads_uneven() {
    let axes = [10, 19, 600, 1_200, 546, 1_090, 2_178, 4_354];
    let before = capture_of(&[(axes, [34, 43, 1_180, 2_340, 1_073, 2_129, 4_241, 8_465])]);
    let after = capture_of(&[(axes, [34, 43, 1_180, 2_340, 1_073, 2_129, 4_241, 8_577])]);
    assert_eq!(outcomes(&before, &after), [Outcome::Uneven]);
}

/// A fixed allocation that the heap peak absorbs as inputs grow reads uneven,
/// never growing: its difference moves the same way at every step, but toward
/// zero, so its magnitude is bounded by its value at the smallest size.
#[test]
fn a_fixed_change_the_peak_absorbs_reads_uneven() {
    let axes = [30, 60, 6_000, 12_000, 5_120, 10_240, 24_576, 49_152];
    let before = capture_of(&[(axes, [10_000; LADDER_SIZES])]);
    let after = capture_of(&[(
        axes,
        [5_904, 5_904, 5_907, 5_908, 5_907, 5_908, 5_909, 5_910],
    )]);
    assert_eq!(outcomes(&before, &after), [Outcome::Uneven]);
}

/// A growing row's slope prints in scientific notation, so a slope far below
/// one unit per axis unit, here a rise of 3 across 70,000 axis units, never
/// rounds to zero.
#[test]
fn a_small_growing_slope_prints_without_rounding_to_zero() {
    let axes = [1, 2, 12_000, 24_000, 10_000, 20_000, 40_000, 80_000];
    let before = capture_of(&[(axes, [100; LADDER_SIZES])]);
    let after = capture_of(&[(axes, [100, 100, 100, 100, 101, 102, 103, 104])]);
    let mut out = Vec::new();
    compare(&text(&before), &text(&after), &mut out).expect("matching captures compare");
    let printed = String::from_utf8(out).expect("the comparison prints UTF-8");
    assert!(printed.contains("slope +4.286e-5"), "{printed}");
}

/// A row measured at different sizes in the two captures is resized, and a
/// row in one capture only is only-before or only-after; none is classified,
/// since no difference at equal size exists.
#[test]
fn rows_without_a_counterpart_at_equal_size_are_not_classified() {
    let axes = [1, 2, 12, 24, 10, 20, 40, 80];
    let readings = [100; LADDER_SIZES];
    let mut grown = axes;
    grown[LADDER_SIZES - 1] += 1;
    let before = capture_of(&[(axes, readings), (axes, readings)]);
    let mut after = capture_of(&[(grown, readings), (axes, readings)]);
    after.rows[1].family = "other".to_owned();
    assert_eq!(
        outcomes(&before, &after),
        [Outcome::Resized, Outcome::OnlyAfter, Outcome::OnlyBefore]
    );
}

/// Captures from different builds are refused rather than compared, while
/// captures from different source trees of one build are compared.
///
/// Debug assertions add metered work, and some operations allocate
/// differently by target, so a difference between builds would not be the
/// change's; a difference between trees is the change, which is what a
/// comparison is for.
#[test]
fn captures_from_different_builds_are_refused_and_different_trees_compared() {
    let before = capture_of(&[([1, 2, 12, 24, 10, 20, 40, 80], [100; LADDER_SIZES])]);
    let mut other_build = before.clone();
    other_build.measured_on.debug_assertions = !other_build.measured_on.debug_assertions;
    let refused = compare(&text(&before), &text(&other_build), &mut Vec::new())
        .expect_err("captures from different builds are refused");
    assert_eq!(refused.kind(), std::io::ErrorKind::InvalidData);

    let mut other_target = before.clone();
    other_target.measured_on.arch = "wasm32".to_owned();
    compare(&text(&before), &text(&other_target), &mut Vec::new())
        .expect_err("captures from different targets are refused");

    let mut other_tree = before.clone();
    other_tree.source_tree = "tree-b".to_owned();
    compare(&text(&before), &text(&other_tree), &mut Vec::new())
        .expect("captures of different trees from one build are compared");
}

/// A build stamp renders as `debug_assertions=<bool>` and
/// `target=<arch>-<os>`, tab-separated, and parses back to itself, including
/// a wasm32 guest's, whose operating system is empty.
///
/// A shard child's header and a capture's header both carry the stamp as
/// text, and the merge and the calibration compare parsed stamps, so a stamp
/// that did not survive the round trip would make two identical builds read
/// as different, or two different ones as the same.
#[test]
fn build_stamps_render_and_parse_back() {
    let guest = BuildStamp {
        debug_assertions: false,
        arch: "wasm32".to_owned(),
        os: String::new(),
    };
    assert_eq!(guest.to_string(), "debug_assertions=false\ttarget=wasm32-");
    for stamp in [BuildStamp::current(), guest] {
        assert_eq!(BuildStamp::parse(&stamp.to_string()), Some(stamp));
    }
    for malformed in [
        "",
        "debug_assertions=false",
        "debug_assertions=maybe\ttarget=x86_64-linux",
        "debug_assertions=false\ttarget=x86_64",
        "target=x86_64-linux\tdebug_assertions=false",
    ] {
        assert_eq!(BuildStamp::parse(malformed), None, "{malformed:?}");
    }
}
