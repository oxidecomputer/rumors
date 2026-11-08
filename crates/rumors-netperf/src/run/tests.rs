use std::sync::atomic::{AtomicUsize, Ordering};

use proptest::prelude::*;
use rumors::testing::property::{SESSION_DIVISOR, divided_proptest};

use super::*;
use crate::calibrate::{Level, PreparationProbe};
use crate::check;
use crate::driver::Prepared;
use crate::env::{Build, TcpLimits};
use crate::fixture::BuildReport;
use crate::grid::{Grid, Kind};
use crate::peer::Outcome;
use crate::point::Point;

/// A session of kind `kind` whose repetition is `repetition`, as a marker.
fn session(kind: Kind, repetition: u32) -> Session {
    let point: Point = crate::grid::Grid::parse(include_str!("../../grids/quick.toml"))
        .unwrap()
        .points()
        .unwrap()[0]
        .1;
    Session {
        point,
        group: "g".into(),
        kind,
        repetition,
    }
}

proptest! {
    /// Interleaving sentinels keeps every grid session in its order, adds
    /// exactly the requested sentinels numbered in order, and spaces them
    /// so no stretch between two consecutive sentinels exceeds the stride.
    #[test]
    fn sentinels_interleave_evenly(len in 0usize..200, count in 0usize..30) {
        let sessions: Vec<Session> = (0..len as u32).map(|i| session(Kind::Grid, i)).collect();
        let sentinels: Vec<Session> = (0..count as u32).map(|i| session(Kind::Sentinel, i)).collect();
        let out = with_sentinels(sessions, sentinels);
        prop_assert_eq!(out.len(), len + count);
        let grid: Vec<u32> = out.iter().filter(|s| s.kind == Kind::Grid).map(|s| s.repetition).collect();
        prop_assert_eq!(grid, (0..len as u32).collect::<Vec<_>>());
        let sentinels: Vec<u32> = out.iter().filter(|s| s.kind == Kind::Sentinel).map(|s| s.repetition).collect();
        prop_assert_eq!(sentinels, (0..count as u32).collect::<Vec<_>>());
        if count > 0 {
            let stride = len.div_ceil(count).max(1);
            let mut gap = 0;
            for s in &out {
                if s.kind == Kind::Sentinel {
                    gap = 0;
                } else {
                    gap += 1;
                    prop_assert!(gap <= stride);
                }
            }
        }
    }
}

/// A grid of two points, each run twice, on a 250 ms link whose bandwidth-delay
/// product the larger pair's transfer exceeds under [`WINDOW`], so the run
/// plans raised-limit reruns of it beside its grid sessions.
const GRID: &str = r#"
repetitions = 2

[[group]]
name = "g"
pairs = [
    { shared = 1000, insert_a = 100000 },
    { shared = 1000, insert_a = 10 },
]
rate = ["1Gbit"]
rtt = ["250ms"]
"#;

/// The fixture host's usable TCP window: 1 MiB.
const WINDOW: u64 = 1 << 20;

/// Sentinels the fixture run interleaves.
const SENTINELS: usize = 3;

/// Sessions the fixture run times at once.
const JOBS: usize = 2;

/// Sessions the fixture run prepares at once.
const PREPARE_JOBS: usize = 3;

/// This host's environment, with the fixture's TCP window.
fn environment() -> Environment {
    let mut environment = Environment::capture().unwrap();
    environment.tcp = TcpLimits {
        payload_share: 0.5,
        usable_window: Some(WINDOW),
        ..environment.tcp
    };
    environment
}

/// A calibration whose noise point is `noise`, accepting sessions of up to one
/// second.
fn calibration(noise: Point) -> Calibration {
    Calibration {
        fingerprint: String::new(),
        noise_point: noise,
        fidelity_point: noise,
        solo: Level {
            jobs: 1,
            session_seconds: vec![1.0],
            ..Level::default()
        },
        levels: Vec::new(),
        jobs: JOBS,
        max_jobs: JOBS,
        verdict: "a fixture".into(),
        preparation: PreparationProbe::default(),
        costs: CostModel::default(),
        memory: MemoryModel::default(),
    }
}

/// A fresh, empty directory for one test case, named for `test`.
fn scratch(test: &str) -> PathBuf {
    static CASES: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "netperf-run-{test}-{}-{}",
        std::process::id(),
        CASES.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

/// The fixture executor: every session prepares at once and measures one
/// second, its replicas converged and each side at the budget the library runs
/// its request at.
struct Fixture;

impl Executor for Fixture {
    type Prepared = ();

    /// Prepares nothing.
    fn prepare(&self, _session: &Session, _slot: usize) -> Result<()> {
        Ok(())
    }

    /// Measures the session as converged in one second.
    fn finish(&self, session: &Session, (): ()) -> Result<Measured> {
        let r = &session.point.replicas;
        let outcome = Outcome {
            wall_seconds: 1.0,
            live_after: r.shared + r.insert_a + r.insert_b,
            ..Outcome::default()
        };
        let settings = &session.point.settings;
        let at = |budget| Outcome {
            effective_budget: check::expected_budget(budget),
            ..outcome.clone()
        };
        Ok(Measured {
            prepared: Prepared {
                build: BuildReport::default(),
                fetch_seconds: 0.0,
                usage: None,
            },
            a: at(settings.a.budget),
            b: at(settings.b.budget),
        })
    }
}

/// A row for `session` as the fixture executor's run records it.
fn row(session: &Session) -> Row {
    let measured = Fixture.finish(session, ()).unwrap();
    Row::measured(session, Placement::default(), &measured)
}

/// Begins the fixture run of [`GRID`] in `dir`, returning its part.
fn begin(dir: &Path, environment: &Environment) -> Part {
    try_begin(dir, environment).unwrap()
}

/// Begins the fixture run of [`GRID`] in `dir`, returning its part, or why it
/// could not begin.
fn try_begin(dir: &Path, environment: &Environment) -> Result<Part> {
    let (points, skipped) = Grid::parse(GRID).unwrap().plan().unwrap();
    let sessions = grid::sessions(&points, 2, Some((WINDOW, 0.5)));
    let plan = Plan::new(2, None, points.clone(), skipped, &sessions);
    let calibration = calibration(points[1].1);
    let sentinels = sentinels(&calibration, SENTINELS);
    let record = RunRecord {
        mode: EMULATION.into(),
        started: now(),
        jobs: JOBS,
        jobs_source: "a fixture".into(),
        preparation: Some(Preparation {
            jobs: PREPARE_JOBS,
            jobs_source: "a fixture".into(),
            memory_bytes: u64::MAX,
            window: 4,
        }),
        sentinels: sentinels.len(),
        sessions: sessions.len() + sentinels.len(),
        sentinel_bound_seconds: Some(calibration.sentinel_bound()),
        ..RunRecord::default()
    };
    Part::begin(
        dir,
        environment,
        &plan,
        Some(calibration),
        order(sessions, sentinels),
        record,
    )
}

/// Runs the first `cut` of `part`'s sessions and stops, as a run does when its
/// process is killed: `run.json` keeps the record written before the first
/// session. Returns the session that would have run next, if any.
fn run_cut_short(mut part: Part, cut: usize) -> Option<Session> {
    let mut sessions = std::mem::take(&mut part.sessions);
    let next = sessions.get(cut).cloned();
    sessions.truncate(cut);
    part.execute(sessions, &Fixture).unwrap();
    next
}

/// The bytes a writer appends to `sessions.csv` for `row`, preceded by the
/// header when `first` is set, as for a file's first row.
fn row_bytes(row: &Row, first: bool) -> Vec<u8> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(first)
        .from_writer(Vec::new());
    writer.serialize(row).unwrap();
    writer.into_inner().unwrap()
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// A run cut short after any number of sessions, with or without a
    /// torn final row, and then resumed, is one clean run by `check`.
    ///
    /// It records every planned session and sentinel exactly once, its
    /// rows record their intervals, and no timed session overlaps a
    /// preparation, across both parts.
    ///
    /// Its rows carry distinct sequence numbers, and its resumption records
    /// what it found, removed, and ran, under the run's own limits.
    #[test]
    fn resumed_run_records_each_session_once(
        cut in any::<prop::sample::Index>(),
        torn in any::<prop::sample::Index>(),
        tear in any::<bool>(),
    ) {
        let dir = scratch("resume");
        let environment = environment();
        let part = begin(&dir, &environment);
        let total = part.sessions.len();
        let cut = cut.index(total + 1);
        let next = run_cut_short(part, cut);
        let mut torn_bytes = 0;
        if let Some(next) = next.filter(|_| tear) {
            let row = row(&next);
            let bytes = row_bytes(&row, cut == 0);
            let prefix = &bytes[..1 + torn.index(bytes.len() - 1)];
            let mut file = fs::OpenOptions::new().append(true).open(dir.join("sessions.csv")).unwrap();
            file.write_all(prefix).unwrap();
            // A first row begins with the header, and a prefix that holds
            // the whole header leaves it in the file's whole part: only
            // the bytes past it are torn.
            let header = bytes.len() - row_bytes(&row, false).len();
            torn_bytes = prefix.len().checked_sub(header).unwrap_or(prefix.len()) as u64;
        }
        let mut cut_short = Vec::new();
        let results = check::load(&dir, &mut cut_short);
        assert!(
            check::verify(std::slice::from_ref(&results)).contains(&check::Failure::Unfinished { dir: dir.clone() }),
        );

        let resumed = Part::resume(&dir, &environment).unwrap();
        assert_eq!(resumed.record.jobs, JOBS);
        assert_eq!(resumed.limits().prepare_jobs, PREPARE_JOBS);
        assert_eq!(resumed.first_sequence, cut as u64);
        let record = resumed.run(&Fixture).unwrap();
        assert_eq!(
            record.resumptions,
            vec![Resumption {
                started: record.resumptions[0].started,
                found: cut,
                ran: total - cut,
                torn_bytes,
            }]
        );

        let mut failures = Vec::new();
        let results = check::load(&dir, &mut failures);
        failures.extend(check::verify(std::slice::from_ref(&results)));
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(failures, vec![]);
        assert_eq!(results.rows.len(), total);
        let mut sequences: Vec<u64> = results.rows.iter().map(|row| row.sequence).collect();
        sequences.sort_unstable();
        assert_eq!(sequences, (0..total as u64).collect::<Vec<_>>());
        let sentinels = results.rows.iter().filter(|row| row.kind == Kind::Sentinel.name()).count();
        assert_eq!(sentinels, SENTINELS);
    }
}

/// A resume under an environment that differs from the run's is refused before
/// it touches the directory, and the refusal names the part that differs with
/// both values.
///
/// The differences tried are another host, and another build whose executable
/// alone differs, as a change to the library alone makes: every fact the build
/// records at compile time, its revision included, matches the run's.
#[test]
fn resume_under_another_environment_is_refused() {
    let environment = environment();
    let elsewhere = Environment {
        hostname: format!("{}-elsewhere", environment.hostname),
        ..environment.clone()
    };
    let rebuilt = Environment {
        build: Build {
            executable: Some("0".repeat(64)),
            ..environment.build.clone()
        },
        ..environment.clone()
    };
    for (case, other, part, value) in [
        ("host", &elsewhere, "host name", elsewhere.hostname.clone()),
        ("build", &rebuilt, "build", "0".repeat(64)),
    ] {
        let dir = scratch(&format!("environment-{case}"));
        run_cut_short(begin(&dir, &environment), 1);
        let before = fs::read(dir.join("sessions.csv")).unwrap();
        let error = format!("{:#}", Part::resume(&dir, other).unwrap_err());
        let after = fs::read(dir.join("sessions.csv")).unwrap();
        let record: RunRecord = read_json(&dir, "run.json").unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert!(error.contains(&format!("the {part} is")), "{case}: {error}");
        assert!(error.contains(&value), "{case}: {error}");
        assert_eq!(before, after, "{case}");
        assert!(record.resumptions.is_empty(), "{case}");
    }
}

/// A new run refuses a directory that already holds a run, finished or cut
/// short, and leaves that run's files as they were.
///
/// The rows of a run cut short are what `--resume` finishes it from, and a
/// finished run's are its results, so a new run beginning there would destroy
/// them. The refusal names `--resume`.
#[test]
fn a_new_run_refuses_a_directory_holding_a_run() {
    let environment = environment();
    let cut_short = scratch("refuse-cut-short");
    run_cut_short(begin(&cut_short, &environment), 1);
    let finished = scratch("refuse-finished");
    begin(&finished, &environment).run(&Fixture).unwrap();
    for dir in [&cut_short, &finished] {
        let files = |dir: &Path| {
            ["sessions.csv", "run.json", "plan.json", "environment.json"]
                .map(|file| fs::read(dir.join(file)).unwrap())
        };
        let before = files(dir);
        let refusal = try_begin(dir, &environment).map(drop);
        let after = files(dir);
        fs::remove_dir_all(dir).unwrap();
        let error = format!("{:#}", refusal.unwrap_err());
        assert!(error.contains("--resume"), "{error}");
        assert_eq!(before, after, "{}", dir.display());
    }
}

/// A run that finished is not resumed: there is nothing left to run, and its
/// record already counts its parts.
#[test]
fn finished_run_is_not_resumed() {
    let dir = scratch("finished");
    let environment = environment();
    begin(&dir, &environment).run(&Fixture).unwrap();
    let error = format!("{:#}", Part::resume(&dir, &environment).unwrap_err());
    fs::remove_dir_all(&dir).unwrap();
    assert!(error.contains("finished"), "{error}");
}
