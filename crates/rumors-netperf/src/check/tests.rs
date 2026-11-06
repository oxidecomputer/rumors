use proptest::prelude::*;
use rumors::default;

use super::*;
use crate::grid::{Grid, Session, sessions};
use crate::record::{Placement, Writer};

/// The library's minimum budget, in the record's unit.
const MIN: u64 = MIN_SYNC_MEMORY_BUDGET as u64;

/// A grid with every feature the check judges: a point at a budget below
/// the minimum, points the wire-time limit skips at 10 Mbit/s, and a pair
/// whose in-flight bytes on the 250 ms link exceed [`SMALL_WINDOW`].
const GRID: &str = r#"
repetitions = 2

[[group]]
name = "g"
pairs = [
    { shared = 1000, insert_a = 100000 },
    { shared = 1000, insert_a = 10 },
]
rate = ["1Gbit", "10Mbit"]
rtt = ["250ms"]
budget = ["0", "default"]
max_wire_seconds = 1
"#;

/// A usable window of 1 MiB, which the grid's larger pair exceeds on its
/// 1 Gbit/s link.
const SMALL_WINDOW: u64 = 1 << 20;

/// A usable window no point of the grid exceeds.
const LARGE_WINDOW: u64 = 1 << 40;

/// Host limits with a usable window of `window` bytes.
fn limits(window: u64) -> TcpLimits {
    TcpLimits {
        payload_share: 0.5,
        usable_window: Some(window),
        ..TcpLimits::default()
    }
}

/// The budget the library runs a side at for `requested`.
fn effective(requested: Option<u64>) -> u64 {
    requested
        .unwrap_or(default::SYNC_MEMORY_BUDGET as u64)
        .max(MIN)
}

/// A row for `session` as a clean run records it.
fn clean_row(session: &Session) -> Row {
    let mut row = Row::new(session, Placement::default());
    row.wall_seconds = Some(1.0);
    row.a_effective_budget_bytes = Some(effective(row.budget_a_bytes));
    row.b_effective_budget_bytes = Some(effective(row.budget_b_bytes));
    row
}

/// The results of a clean run of [`GRID`] on a host whose usable window is
/// `window`, in `count` shards, or unsharded for a count of zero.
fn clean_run(count: u32, window: u64) -> Vec<Results> {
    let (points, skipped) = Grid::parse(GRID).unwrap().plan().unwrap();
    let tcp = limits(window);
    let all = sessions(&points, 2, Some((window, tcp.payload_share)));
    let shards: Vec<Option<Shard>> = if count == 0 {
        vec![None]
    } else {
        (1..=count)
            .map(|index| Some(Shard { index, count }))
            .collect()
    };
    shards
        .into_iter()
        .map(|shard| {
            let mine: Vec<Session> = all
                .iter()
                .filter(|s| shard.is_none_or(|shard| shard.owns(s)))
                .cloned()
                .collect();
            Results {
                dir: PathBuf::from(format!("shard-{}", shard.map_or(0, |shard| shard.index))),
                plan: Some(Plan::new(2, shard, points.clone(), skipped.clone(), &mine)),
                run: Some(RunRecord {
                    ended: Some(1),
                    ..RunRecord::default()
                }),
                tcp: Some(tcp.clone()),
                rows: mine.iter().map(clean_row).collect(),
            }
        })
        .collect()
}

/// The first row of `results` matching `select`, mutably.
fn row_where(results: &mut [Results], select: impl Fn(&Row) -> bool) -> &mut Row {
    results
        .iter_mut()
        .flat_map(|r| &mut r.rows)
        .find(|row| select(row))
        .expect("the clean run has such a row")
}

proptest! {
    /// A clean run passes, unsharded or in any number of shards; losing
    /// any one row names exactly that session as missing, and recording
    /// any one row twice names exactly that session as duplicated.
    ///
    /// A resumed run that ran a recorded session again would record it
    /// twice.
    #[test]
    fn clean_runs_pass_and_each_lost_or_doubled_row_is_named(
        count in 0u32..4,
        chosen in any::<prop::sample::Index>(),
    ) {
        let clean = clean_run(count, SMALL_WINDOW);
        prop_assert_eq!(verify(&clean), vec![]);
        let located: Vec<(usize, usize)> = clean
            .iter()
            .enumerate()
            .flat_map(|(r, result)| (0..result.rows.len()).map(move |i| (r, i)))
            .collect();
        let (r, i) = located[chosen.index(located.len())];
        let mut lost = clean.clone();
        let row = lost[r].rows.remove(i);
        let session = Planned::recorded(&row);
        prop_assert_eq!(verify(&lost), vec![Failure::Missing(session.clone())]);
        let mut doubled = clean;
        doubled[r].rows.push(row);
        prop_assert_eq!(verify(&doubled), vec![Failure::Duplicated { session, rows: 2 }]);
    }

    /// A row for a session no plan lists, beside a clean run's rows, names
    /// exactly that session as unplanned, unsharded or in any number of
    /// shards, whatever directory records it.
    ///
    /// The extra row copies a planned session's at the repetition after
    /// the plan's last, which no plan runs.
    #[test]
    fn each_unplanned_row_is_named(
        count in 0u32..4,
        chosen in any::<prop::sample::Index>(),
        recorder in any::<prop::sample::Index>(),
    ) {
        let mut results = clean_run(count, SMALL_WINDOW);
        let rows: Vec<&Row> = results.iter().flat_map(|r| &r.rows).collect();
        let mut extra = rows[chosen.index(rows.len())].clone();
        extra.repetition = results[0].plan.as_ref().unwrap().repetitions;
        let session = Planned::recorded(&extra);
        let index = recorder.index(results.len());
        results[index].rows.push(extra);
        prop_assert_eq!(verify(&results), vec![Failure::Unplanned(session)]);
    }
}

/// The clean run's grid exercises what the check judges: some points are
/// skipped, some sessions run below the minimum budget, and the small
/// window brings raised-limit reruns.
#[test]
fn clean_run_exercises_every_condition() {
    let results = clean_run(0, SMALL_WINDOW);
    let plan = results[0].plan.as_ref().unwrap();
    assert!(!plan.skipped.is_empty());
    assert!(
        results[0]
            .rows
            .iter()
            .any(|row| row.budget_a_bytes == Some(0))
    );
    assert!(results[0].rows.iter().any(|row| row.kind == "raised"));
}

/// A run missing one of its shards fails as incomplete, names every grid
/// session the lost shard held as missing, and names the raised-limit path
/// as not run exactly when the lost shard held every rerun.
#[test]
fn lost_shard_is_named() {
    let mut results = clean_run(2, SMALL_WINDOW);
    let lost = results.pop().unwrap();
    let failures = verify(&results);
    assert!(
        matches!(failures[0], Failure::Shards { .. }),
        "{failures:?}"
    );
    let mut missing: BTreeSet<&Planned> = BTreeSet::new();
    let mut not_raised = false;
    for failure in &failures[1..] {
        match failure {
            Failure::Missing(session) => {
                missing.insert(session);
            }
            Failure::NotRaised { .. } => not_raised = true,
            other => panic!("unexpected failure {other:?}"),
        }
    }
    assert_eq!(
        not_raised,
        results[0].rows.iter().all(|row| row.kind != "raised")
    );
    let lost_grid: BTreeSet<&Planned> = lost
        .plan
        .as_ref()
        .unwrap()
        .sessions
        .iter()
        .filter(|session| session.kind == "grid")
        .collect();
    assert_eq!(missing, lost_grid);
}

/// Two shards whose plans disagree are named as not one run's.
#[test]
fn mismatched_plans_are_named() {
    let mut results = clean_run(2, SMALL_WINDOW);
    results[1].plan.as_mut().unwrap().skipped.pop();
    assert_eq!(
        verify(&results),
        vec![Failure::Mismatched {
            dir: results[1].dir.clone(),
            first: results[0].dir.clone(),
        }]
    );
}

/// A run whose `run.json` is absent, or records no end, as a run cut
/// short leaves it, is named as unfinished.
#[test]
fn unfinished_run_is_named() {
    for run in [
        None,
        Some(RunRecord {
            ended: None,
            ..RunRecord::default()
        }),
    ] {
        let mut results = clean_run(0, SMALL_WINDOW);
        results[0].run = run;
        assert_eq!(
            verify(&results),
            vec![Failure::Unfinished {
                dir: results[0].dir.clone()
            }]
        );
    }
}

/// Sentinels are numbered from zero in every shard, so the same sentinel
/// in two shards' directories is not a duplicate; twice in one directory,
/// it is.
#[test]
fn sentinels_are_duplicates_only_within_a_directory() {
    let mut results = clean_run(2, SMALL_WINDOW);
    let template = results[0].rows[0].clone();
    let sentinel = Row {
        kind: Kind::Sentinel.name().into(),
        group: "sentinel".into(),
        ..template
    };
    results[0].rows.push(sentinel.clone());
    results[1].rows.push(sentinel.clone());
    assert_eq!(verify(&results), vec![]);
    results[1].rows.push(sentinel.clone());
    assert_eq!(
        verify(&results),
        vec![Failure::Duplicated {
            session: Planned::recorded(&sentinel),
            rows: 2
        }]
    );
}

/// A skip recorded without a reason is named.
#[test]
fn unexplained_skip_is_named() {
    let mut results = clean_run(0, SMALL_WINDOW);
    let skip = &mut results[0].plan.as_mut().unwrap().skipped[0];
    skip.reason = " ".into();
    let (group, point) = (skip.group.clone(), skip.point.clone());
    assert_eq!(
        verify(&results),
        vec![Failure::Unexplained { group, point }]
    );
}

/// A failed session is named with its error.
#[test]
fn errored_session_is_named() {
    let mut results = clean_run(0, SMALL_WINDOW);
    let row = row_where(&mut results, |row| row.kind == "grid");
    row.error = Some("side A: it broke".into());
    let session = Planned::recorded(row);
    assert_eq!(
        verify(&results),
        vec![Failure::Errored {
            session,
            error: "side A: it broke".into()
        }]
    );
}

/// A side at the default target that reported any budget but the one it
/// requested raised to the minimum, or none, is named with the budget it
/// should have run at.
///
/// That holds whether it requested less than the minimum or the library's
/// default, which exceeds it.
///
/// The reports tried are each budget's neighbors, the other request's
/// budget, the minimum, and no report at all. A library that raised a
/// budget below the minimum but ignored one above it would report the
/// minimum for the default request.
#[test]
fn every_budget_not_raised_exactly_to_its_request_is_named() {
    for requested in [Some(0), None] {
        let expected = effective(requested);
        for reported in [
            Some(expected - 1),
            Some(expected + 1),
            Some(effective(Some(0))),
            Some(effective(None)),
            Some(MIN),
            None,
        ]
        .into_iter()
        .filter(|&reported| reported != Some(expected))
        {
            let mut results = clean_run(0, SMALL_WINDOW);
            let row = row_where(&mut results, |row| row.budget_b_bytes == requested);
            row.b_effective_budget_bytes = reported;
            let session = Planned::recorded(row);
            assert_eq!(
                verify(&results),
                vec![Failure::Budget {
                    session,
                    side: 'B',
                    requested,
                    expected,
                    effective: reported,
                }],
                "requested {requested:?}, reported {reported:?}",
            );
        }
    }
}

/// A side at a target other than the default is not judged by the budget
/// condition, whatever it reports.
///
/// [`MIN_SYNC_MEMORY_BUDGET`] is the minimum only at the default target,
/// and a side that states the default target explicitly is still judged.
#[test]
fn sub_minimum_budget_at_another_target_is_not_judged() {
    let mut results = clean_run(0, SMALL_WINDOW);
    let row = row_where(&mut results, |row| row.budget_b_bytes == Some(0));
    row.target_message_size_b_bytes = Some(default::TARGET_MESSAGE_SIZE as u64 / 2);
    row.b_effective_budget_bytes = Some(MIN / 2);
    assert_eq!(verify(&results), vec![]);

    let row = row_where(&mut results, |row| row.budget_b_bytes == Some(0));
    row.target_message_size_b_bytes = Some(default::TARGET_MESSAGE_SIZE as u64);
    let session = Planned::recorded(row);
    assert_eq!(
        verify(&results),
        vec![Failure::Budget {
            session,
            side: 'B',
            requested: Some(0),
            expected: MIN,
            effective: Some(MIN / 2),
        }]
    );
}

/// When the host's window binds a planned point, a run with no finished
/// rerun at raised limits is named, whether it planned none or its reruns
/// all failed to record.
#[test]
fn missing_raised_rerun_is_named() {
    let mut results = clean_run(0, SMALL_WINDOW);
    let plan = results[0].plan.as_mut().unwrap();
    plan.sessions.retain(|session| session.kind != "raised");
    results[0].rows.retain(|row| row.kind != "raised");
    let failures = verify(&results);
    assert!(
        matches!(failures.as_slice(), [Failure::NotRaised { .. }]),
        "{failures:?}"
    );
}

/// When the host's window covers every planned point, a run with no
/// raised-limit rerun passes.
#[test]
fn no_rerun_needed_when_window_covers() {
    let results = clean_run(0, LARGE_WINDOW);
    assert!(results[0].rows.iter().all(|row| row.kind != "raised"));
    assert_eq!(verify(&results), vec![]);
}

/// An emulated run whose host recorded no usable window is named, since
/// the run could not tell which points need raised limits.
#[test]
fn unknown_window_is_named() {
    let mut results = clean_run(0, LARGE_WINDOW);
    results[0].tcp.as_mut().unwrap().usable_window = None;
    assert_eq!(
        verify(&results),
        vec![Failure::UnknownWindow {
            dir: results[0].dir.clone()
        }]
    );
}

/// Writes `results` as results directories under a fresh directory named
/// for `test`, returning their paths.
fn store(results: &[Results], test: &str) -> (PathBuf, Vec<PathBuf>) {
    let root = std::env::temp_dir().join(format!("netperf-check-{test}-{}", std::process::id()));
    let dirs = results
        .iter()
        .map(|result| {
            let dir = root.join(&result.dir);
            fs::create_dir_all(&dir).unwrap();
            let mut writer = Writer::create(&dir.join("sessions.csv")).unwrap();
            for row in &result.rows {
                writer.append(row).unwrap();
            }
            let write = |file: &str, json: String| fs::write(dir.join(file), json).unwrap();
            write("plan.json", serde_json::to_string(&result.plan).unwrap());
            write("run.json", serde_json::to_string(&result.run).unwrap());
            write(
                "environment.json",
                serde_json::json!({ "tcp": result.tcp }).to_string(),
            );
            dir
        })
        .collect();
    (root, dirs)
}

/// A clean sharded run, written to disk and read back, passes every check,
/// its merged report included, and the report lands where `out` names.
#[test]
fn clean_run_on_disk_passes_and_renders() {
    let (root, dirs) = store(&clean_run(2, SMALL_WINDOW), "clean");
    let out = root.join("report");
    let outcome = check(&dirs, &out);
    let rendered = out.join("report.md").is_file();
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(outcome.failures, vec![]);
    assert!(rendered);
}

/// A report that cannot be written, here because `out` is a file, is named
/// as the one failure of an otherwise clean run.
#[test]
fn unrenderable_report_is_named() {
    let (root, dirs) = store(&clean_run(0, SMALL_WINDOW), "unrenderable");
    let out = root.join("not-a-directory");
    fs::write(&out, "").unwrap();
    let outcome = check(&dirs, &out);
    fs::remove_dir_all(&root).unwrap();
    assert!(
        matches!(outcome.failures.as_slice(), [Failure::Report(_)]),
        "{:?}",
        outcome.failures
    );
}

/// One session's intervals as the phase tests draw them: its
/// preparation's start and length, and its timed session's, if it had one.
///
/// They are whole seconds, so that intervals often touch, and some have no
/// length, as the clock's resolution can leave a short one.
type Spans = ((u8, u8), Option<(u8, u8)>);

/// The results of one directory whose run times `jobs` sessions at once and
/// records how it prepares them, with a row per entry of `spans`: a clean
/// row where the entry has a timed session, a failed one where it has not.
fn phased(spans: &[Spans], jobs: usize) -> Results {
    let mut results = clean_run(0, LARGE_WINDOW).remove(0);
    let template = results.rows[0].clone();
    results.run = Some(RunRecord {
        ended: Some(1),
        jobs,
        preparation: Some(crate::run::Preparation::default()),
        ..RunRecord::default()
    });
    results.rows = spans
        .iter()
        .enumerate()
        .map(|(index, &((began, length), timed))| {
            let mut row = template.clone();
            row.repetition = index as u32;
            row.prepare_began_unix_seconds = Some(f64::from(began));
            row.prepare_ended_unix_seconds = Some(f64::from(began) + f64::from(length));
            row.timed_began_unix_seconds = timed.map(|(began, _)| f64::from(began));
            row.timed_ended_unix_seconds =
                timed.map(|(began, length)| f64::from(began) + f64::from(length));
            if timed.is_none() {
                row.error = Some("the preparation failed".into());
            }
            row
        })
        .collect();
    results
}

/// The half-open intervals `[began, ended)` of `spans` that hold any
/// moment: the preparations, and the timed sessions.
#[allow(clippy::type_complexity)]
fn intervals(spans: &[Spans]) -> (Vec<(u32, u32)>, Vec<(u32, u32)>) {
    let half_open = |(began, length): (u8, u8)| {
        (length > 0).then(|| (u32::from(began), u32::from(began) + u32::from(length)))
    };
    (
        spans
            .iter()
            .filter_map(|(prepare, _)| half_open(*prepare))
            .collect(),
        spans
            .iter()
            .filter_map(|(_, timed)| timed.and_then(half_open))
            .collect(),
    )
}

proptest! {
    /// The phase check names overlaps and crowding exactly where a
    /// brute-force comparison of every pair of intervals finds them.
    ///
    /// An overlap is a timed session's interval overlapping a
    /// preparation's; crowding is more timed sessions running at some
    /// timed session's start than the job count. Intervals that only touch
    /// do not overlap, and an interval of no length overlaps nothing.
    #[test]
    fn phases_agree_with_brute_force(
        spans in prop::collection::vec(((0u8..40, 0u8..8), prop::option::of((0u8..40, 0u8..8))), 0..12),
        jobs in 1usize..3,
    ) {
        let mut failures = Vec::new();
        phases(&phased(&spans, jobs), &mut failures);
        let (prepared, timed) = intervals(&spans);
        let overlap = |(b1, e1): (u32, u32), (b2, e2): (u32, u32)| b1 < e2 && b2 < e1;
        let overlapped = timed.iter().any(|t| prepared.iter().any(|p| overlap(*t, *p)));
        let crowded = timed.iter().any(|&(start, _)| {
            timed.iter().filter(|&&(began, ended)| began <= start && start < ended).count() > jobs
        });
        prop_assert_eq!(failures.iter().any(|f| matches!(f, Failure::Overlapped { .. })), overlapped);
        prop_assert_eq!(failures.iter().any(|f| matches!(f, Failure::Crowded { .. })), crowded);
        let unrecorded = failures.iter().any(|f| matches!(f, Failure::Unrecorded { .. }));
        prop_assert!(!unrecorded);
    }
}

/// In a run that records how it prepares sessions, every row whose
/// intervals are missing or garbled is named.
///
/// A measured row without a timed interval, a row without a preparation
/// interval, and a row whose timed interval ends before it begins each
/// fail, so the overlap check cannot pass because the intervals went
/// unrecorded.
#[test]
fn unrecorded_intervals_are_named() {
    let mut results = phased(
        &[
            ((0, 1), Some((2, 1))),
            ((0, 1), Some((4, 1))),
            ((0, 1), Some((6, 1))),
        ],
        1,
    );
    results.rows[0].timed_began_unix_seconds = None;
    results.rows[1].prepare_ended_unix_seconds = None;
    results.rows[2].timed_ended_unix_seconds = Some(5.0);
    let mut failures = Vec::new();
    phases(&results, &mut failures);
    let named: Vec<&'static str> = failures
        .iter()
        .filter_map(|f| match f {
            Failure::Unrecorded { interval, .. } => Some(*interval),
            _ => None,
        })
        .collect();
    assert_eq!(named, vec!["timed session", "preparation", "timed session"]);
}

/// A run from a tool that prepared each session just before timing it
/// records no preparation limits and no intervals, and the phase check
/// leaves it alone rather than naming every row.
#[test]
fn run_without_preparation_limits_is_not_judged() {
    let mut results = phased(&[((0, 5), Some((1, 1)))], 1);
    results.run.as_mut().unwrap().preparation = None;
    let mut failures = Vec::new();
    phases(&results, &mut failures);
    assert_eq!(failures, vec![]);
}
