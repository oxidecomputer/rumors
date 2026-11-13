use proptest::prelude::*;

use super::*;

/// The library's minimum budget, in the report's unit.
const MIN: u64 = rumors::testing::MIN_SYNC_MEMORY_BUDGET as u64;

/// A finished session of a point whose sides request `budget` and report
/// running at `effective`, and whose larger side's heap rose `heap` bytes above
/// both its start and its end.
fn session(budget: Option<u64>, effective: u64, heap: u64) -> Row {
    Row {
        link: "emulated".into(),
        connection: "warm".into(),
        wall_seconds: Some(1.0),
        budget_a_bytes: budget,
        budget_b_bytes: budget,
        a_effective_budget_bytes: Some(effective),
        b_effective_budget_bytes: Some(effective),
        a_heap_before_bytes: Some(0),
        a_heap_peak_bytes: Some(heap),
        a_heap_after_bytes: Some(0),
        b_heap_before_bytes: Some(0),
        b_heap_peak_bytes: Some(0),
        b_heap_after_bytes: Some(0),
        ..Row::default()
    }
}

/// A point at budget 0 whose sides report running at the minimum: the report
/// takes the minimum as its effective budget, divides its heap by the minimum,
/// and prints the request beside the minimum it ran at.
#[test]
fn zero_budget_point_runs_at_the_minimum() {
    let heap = 3 << 20;
    let row = session(Some(0), MIN, heap);
    let point = summarize_point(&[&row], None).unwrap();
    assert_eq!(point.budget(), MIN);
    assert_eq!(point.heap_ratio(), Some(heap as f64 / MIN as f64));
    assert_eq!(budget_text(&point.row), "0 → minimum");
}

/// A point whose sides run unlimited has no bound to compare its heap against,
/// so it has no heap ratio.
///
/// Its effective budget is the largest, so the budget table measures the other
/// budgets against it, and its budget column reads `unlimited`.
#[test]
fn unlimited_point_has_no_heap_ratio() {
    let row = session(Some(UNLIMITED_BUDGET), UNLIMITED_BUDGET, 3 << 20);
    let point = summarize_point(&[&row], None).unwrap();
    assert_eq!(point.budget(), UNLIMITED_BUDGET);
    assert_eq!(point.heap_ratio(), None);
    assert_eq!(budget_text(&point.row), "unlimited");
}

/// Points compare across budgets, and across TCP buffer limits, only within one
/// transport.
///
/// For each comparison the report makes, two points that differ only in what it
/// leaves out (the budget, or the buffer limit) share a family exactly when
/// they ran over the same stack, over every pair of stacks.
#[test]
fn families_never_mix_transports() {
    for (with_budget, with_buffers) in [(false, true), (true, false)] {
        for (tls_x, tls_y) in [(false, false), (false, true), (true, false), (true, true)] {
            let x = Row {
                tls: tls_x,
                ..session(Some(MIN), MIN, 0)
            };
            let budget = if with_budget { MIN } else { 2 * MIN };
            let y = Row {
                tls: tls_y,
                tcp_buffer_bytes: (!with_buffers).then_some(64 << 20),
                ..session(Some(budget), budget, 0)
            };
            let family = |row: &Row| {
                summarize_point(&[row], None)
                    .unwrap()
                    .family(with_budget, with_buffers)
            };
            assert_eq!(
                family(&x) == family(&y),
                tls_x == tls_y,
                "with_budget {with_budget}, with_buffers {with_buffers}, tls {tls_x} and {tls_y}"
            );
        }
    }
}

proptest! {
    /// A point runs at the effective budget its sides report, whatever
    /// minimum their target sets.
    ///
    /// Around a reported minimum, a request runs at the larger of itself
    /// and that minimum, and the budget column marks exactly the requests
    /// below it as running at the minimum.
    ///
    /// Minimums are drawn at and away from [`MIN`], the minimum at the
    /// default target.
    #[test]
    fn requests_below_the_minimum_run_at_it(
        (minimum, requested) in prop_oneof![Just(MIN), 1..UNLIMITED_BUDGET / 2].prop_flat_map(
            |minimum| (
                Just(minimum),
                prop_oneof![
                    0..minimum,
                    minimum.saturating_sub(2)..minimum + 2,
                    minimum..UNLIMITED_BUDGET,
                ],
            )
        ),
        heap in 0u64..1 << 40,
    ) {
        let effective = requested.max(minimum);
        let row = session(Some(requested), effective, heap);
        let point = summarize_point(&[&row], None).unwrap();
        prop_assert_eq!(point.budget(), effective);
        prop_assert_eq!(point.heap_ratio(), Some(heap as f64 / effective as f64));
        prop_assert_eq!(
            budget_text(&point.row).ends_with("→ minimum"),
            requested < minimum
        );
    }
}

/// Every row of the calibration section's table has a cell for each header
/// column, and each cell holds its column's quantity.
///
/// The columns are the job count, the readings taken, a time, a goodput in
/// Mbit/s, a time, a busy share in percent, and the two verdicts. The
/// calibration has a solo level and two concurrent levels of different reading
/// counts, one failing on fidelity.
#[test]
fn calibration_table_cells_follow_their_header() {
    let (_, point) = crate::grid::Grid::parse("[[group]]\nname = \"g\"\n")
        .unwrap()
        .points()
        .unwrap()
        .remove(0);
    let level = |jobs: usize, readings: usize, fidelity_passed: bool| crate::calibrate::Level {
        jobs,
        session_seconds: vec![2.5; readings],
        goodput: vec![1.25e8; readings],
        rtt: vec![0.25; readings],
        busy_share: 0.5,
        noise_passed: true,
        fidelity_passed,
    };
    let calibration = Calibration {
        fingerprint: String::new(),
        noise_point: point,
        fidelity_point: point,
        solo: level(1, 11, true),
        levels: vec![level(2, 8, true), level(16, 16, false)],
        jobs: 2,
        max_jobs: 16,
        verdict: "a fixture".into(),
        preparation: crate::calibrate::PreparationProbe::default(),
        costs: crate::predict::CostModel::default(),
        memory: crate::predict::MemoryModel::default(),
    };
    let mut md = String::new();
    write_calibration(&mut md, &calibration).unwrap();
    let cells = |line: &str| -> Vec<String> {
        line.trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().to_string())
            .collect()
    };
    let mut table = md.lines().filter(|line| line.starts_with('|'));
    let header = cells(table.next().unwrap());
    assert_eq!(header[1], "readings");
    let rows: Vec<Vec<String>> = table.skip(1).map(cells).collect();
    let expected = [(1, 11, "pass"), (2, 8, "pass"), (16, 16, "fail")];
    assert_eq!(rows.len(), expected.len(), "{md}");
    for (row, (jobs, readings, fidelity)) in rows.iter().zip(expected) {
        assert_eq!(row.len(), header.len(), "{md}");
        assert_eq!(row[0], jobs.to_string(), "{md}");
        assert_eq!(row[1], readings.to_string(), "{md}");
        assert_eq!(row[2], human_seconds(2.5), "{md}");
        assert_eq!(row[3], "1000.0 Mbit/s", "{md}");
        assert_eq!(row[4], human_seconds(0.25), "{md}");
        assert_eq!(row[5], "50%", "{md}");
        assert_eq!(row[6], "pass", "{md}");
        assert_eq!(row[7], fidelity, "{md}");
    }
}
