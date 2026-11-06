//! Checking a run's results against facts every clean run satisfies.
//!
//! A run's timings vary with its host, so no threshold on them can say
//! whether the run worked. What a clean run guarantees instead are facts
//! that hold on any host, and `rumors-netperf check` verifies them for one
//! run's results directory, or for the directories of every shard of one
//! run taken together:
//!
//! - **The results are whole.** Every directory holds its `plan.json`,
//!   `environment.json`, and a `run.json` that records the run's end; the
//!   directories' plans are one grid's, and their shards make up the whole
//!   run exactly once.
//! - **Every planned session has exactly one row, and every row a plan:**
//!   each point the plan lists at every repetition, and each raised-limit
//!   rerun a shard planned, has one row, and no row but a sentinel's
//!   records a session no plan lists. A run cut short and resumed is one
//!   run, its parts' rows in one `sessions.csv`, so this holds across
//!   every part. Every point the grid skips is recorded with its reason.
//! - **No session failed.**
//! - **Every side runs at the budget it requested, raised to the
//!   minimum:** every side at the default message-size target reports,
//!   from the library, that it ran at exactly the larger of the budget it
//!   requested (the library's default when it requested none) and
//!   [`MIN_SYNC_MEMORY_BUDGET`].
//! - **The raised-limit path ran where the grid needs it:** when the
//!   host's usable TCP window is smaller than what some planned point
//!   keeps in flight ([`raised_buffer`]), at least one rerun with raised
//!   limits finished.
//! - **Timed sessions ran under calibration's conditions:** in a run that
//!   records how it prepares sessions, every row records when its
//!   preparation ran and, if it was timed, when its timed session ran;
//!   no timed session overlaps any preparation; and no more timed sessions
//!   overlap at any moment than the run's job count ([`phases`]).
//! - **The report renders.**
//!
//! [`verify`] judges results already read, which is how the tests feed it
//! synthetic runs; [`check`] reads the directories, verifies them, and
//! renders the report.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use rumors::{MIN_SYNC_MEMORY_BUDGET, default};
use serde::Deserialize;

use crate::env::TcpLimits;
use crate::grid::{Kind, Shard, raised_buffer};
use crate::point::Link;
use crate::record::{self, Row};
use crate::report;
use crate::run::{Plan, Planned, RunRecord};

/// One results directory's files, as `check` reads them.
#[derive(Clone, Debug, Default)]
pub struct Results {
    /// The directory.
    pub dir: PathBuf,
    /// Its `plan.json`, if present.
    pub plan: Option<Plan>,
    /// Its `run.json`, which records the run's end once it finished.
    pub run: Option<RunRecord>,
    /// The TCP limits its `environment.json` records, if present.
    pub tcp: Option<TcpLimits>,
    /// Its `sessions.csv`.
    pub rows: Vec<Row>,
}

/// The part of `environment.json` the check reads.
#[derive(Deserialize)]
struct EnvironmentTcp {
    /// The host's TCP limits.
    tcp: TcpLimits,
}

/// One fact a run's results fail to satisfy.
#[derive(Clone, Debug, PartialEq)]
pub enum Failure {
    /// A results file exists but could not be read.
    Unreadable {
        /// The directory.
        dir: PathBuf,
        /// The file.
        file: &'static str,
        /// Why, with its causes.
        error: String,
    },
    /// A results file a run writes before its first session is absent.
    Absent {
        /// The directory.
        dir: PathBuf,
        /// The file.
        file: &'static str,
    },
    /// A directory has no `run.json`, or one that records no end: its run
    /// was cut short, or still runs.
    Unfinished {
        /// The directory.
        dir: PathBuf,
    },
    /// A directory's plan differs from the first directory's in its points,
    /// skips, or repetitions, so the two are not shards of one run.
    Mismatched {
        /// The directory.
        dir: PathBuf,
        /// The first directory, whose plan it was compared against.
        first: PathBuf,
    },
    /// The directories' shards do not make up one whole run exactly once.
    Shards {
        /// The shards found, in words.
        found: String,
    },
    /// A planned session has no row.
    Missing(Planned),
    /// A row records a session no plan lists, and is not a sentinel's.
    Unplanned(Planned),
    /// A session has more than one row in one run.
    Duplicated {
        /// The session.
        session: Planned,
        /// How many rows record it.
        rows: usize,
    },
    /// A skipped point is recorded without a reason.
    Unexplained {
        /// The point's group.
        group: String,
        /// The point's key.
        point: String,
    },
    /// A session failed or never measured.
    Errored {
        /// The session.
        session: Planned,
        /// Why, as its row records it.
        error: String,
    },
    /// A side reported running at another budget than the one it
    /// requested, raised to the minimum.
    Budget {
        /// The session.
        session: Planned,
        /// `A` or `B`.
        side: char,
        /// The budget the side requested; `None` for the library's default.
        requested: Option<u64>,
        /// The budget the library runs the side at for that request.
        expected: u64,
        /// The budget the side reported, if it reported one.
        effective: Option<u64>,
    },
    /// The host's window binds a planned point, yet no rerun with raised
    /// limits finished.
    NotRaised {
        /// A point the window binds.
        point: String,
    },
    /// A directory's host recorded no usable TCP window, so the run could
    /// not tell which emulated points to rerun with raised limits.
    UnknownWindow {
        /// The directory.
        dir: PathBuf,
    },
    /// A row of a run that records how it prepares sessions lacks an
    /// interval it must record, or records one that ends before it begins:
    /// its preparation's, or, for a session that measured, its timed
    /// session's.
    Unrecorded {
        /// The directory.
        dir: PathBuf,
        /// The session.
        session: Planned,
        /// `preparation` or `timed session`.
        interval: &'static str,
    },
    /// A timed session overlapped another session's preparation.
    Overlapped {
        /// The directory.
        dir: PathBuf,
        /// The timed session.
        timed: Planned,
        /// The session whose preparation overlapped it.
        preparing: Planned,
    },
    /// More timed sessions overlapped at one moment than the run's job
    /// count.
    Crowded {
        /// The directory.
        dir: PathBuf,
        /// The timed session whose start exceeded the count.
        session: Planned,
        /// Timed sessions running once it started, itself included.
        running: usize,
        /// The job count.
        jobs: usize,
    },
    /// The report failed to render.
    Report(String),
}

impl fmt::Display for Planned {
    /// The session as its row names it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "group `{}`, point `{}`, {} session, repetition {}",
            self.group, self.point, self.kind, self.repetition
        )
    }
}

impl fmt::Display for Failure {
    /// The failure in one line, naming where it lies.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Unreadable { dir, file, error } => {
                write!(f, "{}: cannot read {file}: {error}", dir.display())
            }
            Failure::Absent { dir, file } => {
                write!(f, "{}: no {file}", dir.display())
            }
            Failure::Unfinished { dir } => write!(
                f,
                "{}: run.json records no end, so the run was cut short or still runs; \
                 `rumors-netperf run --resume {}` finishes a run cut short",
                dir.display(),
                dir.display()
            ),
            Failure::Mismatched { dir, first } => write!(
                f,
                "{}: its plan differs from {}'s, so the two are not shards of one run",
                dir.display(),
                first.display()
            ),
            Failure::Shards { found } => write!(f, "the shards do not make up one run: {found}"),
            Failure::Missing(session) => write!(f, "no row for the {session}"),
            Failure::Unplanned(session) => {
                write!(f, "a row for the {session}, which no plan lists")
            }
            Failure::Duplicated { session, rows } => {
                write!(f, "{rows} rows for the {session}, which runs once")
            }
            Failure::Unexplained { group, point } => write!(
                f,
                "group `{group}`, point `{point}`: skipped without a reason"
            ),
            Failure::Errored { session, error } => write!(f, "the {session} failed: {error}"),
            Failure::Budget {
                session,
                side,
                requested,
                expected,
                effective,
            } => write!(
                f,
                "the {session}: side {side} requested {}, so the library runs it at {expected} bytes, \
                 but it reported running at {}",
                requested.map_or("the default budget".into(), |bytes| format!(
                    "a budget of {bytes} bytes"
                )),
                effective.map_or("no budget".into(), |bytes| format!("{bytes} bytes"))
            ),
            Failure::NotRaised { point } => write!(
                f,
                "the host's usable TCP window binds point `{point}`, yet no rerun with raised limits finished"
            ),
            Failure::UnknownWindow { dir } => write!(
                f,
                "{}: the host recorded no usable TCP window, so the run could not tell which points to \
                 rerun with raised limits",
                dir.display()
            ),
            Failure::Unrecorded {
                dir,
                session,
                interval,
            } => write!(
                f,
                "{}: the {session} records no {interval} interval, or one that ends before it begins",
                dir.display()
            ),
            Failure::Overlapped {
                dir,
                timed,
                preparing,
            } => write!(
                f,
                "{}: the timed session of the {timed} overlapped the preparation of the {preparing}",
                dir.display()
            ),
            Failure::Crowded {
                dir,
                session,
                running,
                jobs,
            } => write!(
                f,
                "{}: the timed session of the {session} ran beside others, {running} at once, \
                 beyond the run's {jobs} jobs",
                dir.display()
            ),
            Failure::Report(error) => write!(f, "the report does not render: {error}"),
        }
    }
}

/// What [`check`] found.
#[derive(Clone, Debug, Default)]
pub struct Outcome {
    /// Every failure, in the order the checks found them.
    pub failures: Vec<Failure>,
    /// Rows read, sentinels included.
    pub sessions: usize,
    /// Points the grid skipped.
    pub skipped: usize,
    /// Rows of raised-limit reruns.
    pub raised: usize,
}

/// Reads an optional JSON file of `dir`: `None` when it is absent.
fn read_json<T: for<'de> Deserialize<'de>>(
    dir: &Path,
    file: &'static str,
    failures: &mut Vec<Failure>,
) -> Option<T> {
    let unreadable = |error: String| Failure::Unreadable {
        dir: dir.to_path_buf(),
        file,
        error,
    };
    match fs::read_to_string(dir.join(file)) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|error| failures.push(unreadable(error.to_string())))
            .ok(),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => {
            failures.push(unreadable(error.to_string()));
            None
        }
    }
}

/// Reads one results directory, adding a failure for each file that exists
/// but cannot be read.
pub fn load(dir: &Path, failures: &mut Vec<Failure>) -> Results {
    let rows = record::read(&dir.join("sessions.csv")).unwrap_or_else(|error| {
        failures.push(Failure::Unreadable {
            dir: dir.to_path_buf(),
            file: "sessions.csv",
            error: format!("{error:#}"),
        });
        Vec::new()
    });
    Results {
        dir: dir.to_path_buf(),
        plan: read_json(dir, "plan.json", failures),
        run: read_json(dir, "run.json", failures),
        tcp: read_json::<EnvironmentTcp>(dir, "environment.json", failures).map(|e| e.tcp),
        rows,
    }
}

/// Every failure among `results`, one run's directory or every shard's,
/// apart from the report's.
pub fn verify(results: &[Results]) -> Vec<Failure> {
    let mut failures = Vec::new();
    for result in results {
        let dir = result.dir.clone();
        if result.plan.is_none() {
            failures.push(Failure::Absent {
                dir: dir.clone(),
                file: "plan.json",
            });
        }
        if result.tcp.is_none() {
            failures.push(Failure::Absent {
                dir: dir.clone(),
                file: "environment.json",
            });
        }
        if result.run.as_ref().is_none_or(|run| run.ended.is_none()) {
            failures.push(Failure::Unfinished { dir });
        }
    }
    let plans: Vec<(&Path, &Plan)> = results
        .iter()
        .filter_map(|r| Some((r.dir.as_path(), r.plan.as_ref()?)))
        .collect();
    let rows: Vec<&Row> = results.iter().flat_map(|r| &r.rows).collect();
    wholeness(&plans, &mut failures);
    coverage(&plans, &rows, &mut failures);
    duplicates(results, &mut failures);
    for row in rows.iter().filter(|row| !row.ok()) {
        failures.push(Failure::Errored {
            session: Planned::recorded(row),
            error: row
                .error
                .clone()
                .unwrap_or_else(|| "it recorded no measurement".into()),
        });
    }
    budgets(&rows, &mut failures);
    raised(results, &rows, &mut failures);
    for result in results {
        phases(result, &mut failures);
    }
    failures
}

/// Whether, in a directory whose run records how it prepares sessions,
/// every row records its intervals, no timed session overlaps any
/// preparation, and no more timed sessions overlap than the job count.
///
/// The intervals of one part share its monotonic clock, and the parts of
/// a run follow one another, so every row of the directory compares
/// against every other. Each interval is half-open, holding its start but
/// not its end, so intervals that only touch do not overlap, and an
/// interval the clock's resolution shrank to nothing holds no moment at
/// all. A directory from a tool that ran each session whole records no
/// preparation limits and is left out: it never scheduled preparation.
///
/// A sweep over the intervals' ends, in time order with every end before
/// any start at the same moment, sees each start with the set of
/// intervals open at that moment, which is every interval it overlaps
/// that started no later; an overlap is therefore found when its later
/// interval starts.
pub fn phases(result: &Results, failures: &mut Vec<Failure>) {
    let Some(jobs) = result
        .run
        .as_ref()
        .filter(|run| run.preparation.is_some())
        .map(|run| run.jobs)
    else {
        return;
    };
    let dir = &result.dir;
    let mut ends: Vec<(f64, bool, bool, usize)> = Vec::new();
    for (index, row) in result.rows.iter().enumerate() {
        let unrecorded = |interval| Failure::Unrecorded {
            dir: dir.clone(),
            session: Planned::recorded(row),
            interval,
        };
        match (
            row.prepare_began_unix_seconds,
            row.prepare_ended_unix_seconds,
        ) {
            (Some(began), Some(ended)) if began < ended => {
                ends.push((began, true, false, index));
                ends.push((ended, false, false, index));
            }
            (Some(began), Some(ended)) if began == ended => {}
            _ => failures.push(unrecorded("preparation")),
        }
        match (row.timed_began_unix_seconds, row.timed_ended_unix_seconds) {
            (Some(began), Some(ended)) if began < ended => {
                ends.push((began, true, true, index));
                ends.push((ended, false, true, index));
            }
            (Some(began), Some(ended)) if began == ended => {}
            (None, None) if !row.ok() => {}
            _ => failures.push(unrecorded("timed session")),
        }
    }
    // Time order; at one moment, ends before starts.
    ends.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
    let mut preparing: BTreeSet<usize> = BTreeSet::new();
    let mut timing: BTreeSet<usize> = BTreeSet::new();
    let name = |index: usize| Planned::recorded(&result.rows[index]);
    for (_, start, timed, index) in ends {
        let (own, other) = if timed {
            (&mut timing, &mut preparing)
        } else {
            (&mut preparing, &mut timing)
        };
        if !start {
            own.remove(&index);
            continue;
        }
        own.insert(index);
        if let Some(&first) = other.first() {
            let (timed, preparing) = if timed {
                (index, first)
            } else {
                (first, index)
            };
            failures.push(Failure::Overlapped {
                dir: dir.clone(),
                timed: name(timed),
                preparing: name(preparing),
            });
        }
        if timed && timing.len() > jobs {
            failures.push(Failure::Crowded {
                dir: dir.clone(),
                session: name(index),
                running: timing.len(),
                jobs,
            });
        }
    }
}

/// Whether `plans` are one run's: the same points, skips, and repetitions,
/// and either one unsharded plan or shards 1 to n of n, each once.
fn wholeness(plans: &[(&Path, &Plan)], failures: &mut Vec<Failure>) {
    let Some((first_dir, first)) = plans.first() else {
        return;
    };
    for (dir, plan) in &plans[1..] {
        if (plan.repetitions, &plan.points, &plan.skipped)
            != (first.repetitions, &first.points, &first.skipped)
        {
            failures.push(Failure::Mismatched {
                dir: dir.to_path_buf(),
                first: first_dir.to_path_buf(),
            });
        }
    }
    let shards: Vec<Option<Shard>> = plans.iter().map(|(_, plan)| plan.shard).collect();
    let found = || {
        shards
            .iter()
            .map(|shard| shard.map_or("unsharded".into(), |s| format!("{}/{}", s.index, s.count)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let whole = match shards.as_slice() {
        [None] => true,
        _ => {
            let sharded: Vec<Shard> = shards.iter().flatten().copied().collect();
            let count = sharded.first().map_or(0, |s| s.count);
            let indices: Vec<u32> = {
                let mut indices: Vec<u32> = sharded.iter().map(|s| s.index).collect();
                indices.sort_unstable();
                indices
            };
            sharded.len() == shards.len()
                && sharded.iter().all(|s| s.count == count)
                && indices == (1..=count).collect::<Vec<_>>()
        }
    };
    if !whole {
        failures.push(Failure::Shards {
            found: format!(
                "{}; give one unsharded run, or shards 1 to n of n, each once",
                found()
            ),
        });
    }
}

/// Whether every planned session has a row, every row a planned session,
/// and every skip a reason.
///
/// A session is planned when a plan lists it, or when it is a grid session
/// of a planned point at a repetition the plan runs: the second covers
/// every shard's share of the grid together, so a lost shard shows here
/// as well as in [`wholeness`]. Plans leave sentinels out, so their rows
/// are never unplanned.
fn coverage(plans: &[(&Path, &Plan)], rows: &[&Row], failures: &mut Vec<Failure>) {
    let recorded: BTreeSet<Planned> = rows.iter().map(|row| Planned::recorded(row)).collect();
    let mut expected: BTreeSet<Planned> = BTreeSet::new();
    let mut skipped = BTreeSet::new();
    for (_, plan) in plans {
        expected.extend(plan.sessions.iter().cloned());
        for (group, point) in &plan.points {
            for repetition in 0..plan.repetitions {
                expected.insert(Planned {
                    group: group.clone(),
                    point: point.key(),
                    kind: Kind::Grid.name().to_string(),
                    repetition,
                });
            }
        }
        for skip in &plan.skipped {
            if skip.reason.trim().is_empty() && skipped.insert((&skip.group, &skip.point)) {
                failures.push(Failure::Unexplained {
                    group: skip.group.clone(),
                    point: skip.point.clone(),
                });
            }
        }
    }
    failures.extend(
        expected
            .iter()
            .filter(|session| !recorded.contains(session))
            .cloned()
            .map(Failure::Missing),
    );
    let unplanned: BTreeSet<Planned> = rows
        .iter()
        .filter(|row| row.kind != Kind::Sentinel.name())
        .map(|row| Planned::recorded(row))
        .filter(|session| !expected.contains(session))
        .collect();
    failures.extend(unplanned.into_iter().map(Failure::Unplanned));
}

/// Whether any session has more than one row.
///
/// Every shard runs sentinels numbered from zero, so a sentinel's identity
/// is scoped to its directory; every other session belongs to one shard,
/// so its identity spans them all.
fn duplicates(results: &[Results], failures: &mut Vec<Failure>) {
    let mut counts: BTreeMap<(Option<usize>, Planned), usize> = BTreeMap::new();
    for (index, result) in results.iter().enumerate() {
        for row in &result.rows {
            let scope = (row.kind == Kind::Sentinel.name()).then_some(index);
            *counts.entry((scope, Planned::recorded(row))).or_default() += 1;
        }
    }
    failures.extend(
        counts
            .into_iter()
            .filter(|&(_, rows)| rows > 1)
            .map(|((_, session), rows)| Failure::Duplicated { session, rows }),
    );
}

/// The budget the library runs a side at the default message-size target
/// at, for its `requested` budget, `None` for the library's default.
///
/// The run hands the library the request as a `usize`, saturating, and the
/// library raises it to [`MIN_SYNC_MEMORY_BUDGET`], the minimum at the
/// default target.
pub(crate) fn expected_budget(requested: Option<u64>) -> u64 {
    let requested = requested.map_or(default::SYNC_MEMORY_BUDGET, |bytes| {
        usize::try_from(bytes).unwrap_or(usize::MAX)
    });
    requested.max(MIN_SYNC_MEMORY_BUDGET) as u64
}

/// Whether every finished side at the default message-size target reports
/// running at exactly the budget it requested, raised to the minimum
/// ([`expected_budget`]).
///
/// The library computes the minimum for each side's own target and
/// publishes it only at the default, so sides at other targets are left
/// out. Failed sessions are left to the error check: a session that never
/// reached its peers has no effective budget to report.
fn budgets(rows: &[&Row], failures: &mut Vec<Failure>) {
    let default_target = |target: Option<u64>| {
        target.is_none_or(|bytes| bytes == default::TARGET_MESSAGE_SIZE as u64)
    };
    for row in rows.iter().filter(|row| row.ok()) {
        for (side, requested, target, effective) in [
            (
                'A',
                row.budget_a_bytes,
                row.target_message_size_a_bytes,
                row.a_effective_budget_bytes,
            ),
            (
                'B',
                row.budget_b_bytes,
                row.target_message_size_b_bytes,
                row.b_effective_budget_bytes,
            ),
        ] {
            let expected = expected_budget(requested);
            if default_target(target) && effective != Some(expected) {
                failures.push(Failure::Budget {
                    session: Planned::recorded(row),
                    side,
                    requested,
                    expected,
                    effective,
                });
            }
        }
    }
}

/// Whether a rerun with raised limits finished, if any directory's host
/// window binds one of its planned points.
fn raised(results: &[Results], rows: &[&Row], failures: &mut Vec<Failure>) {
    let mut bound = None;
    for result in results {
        let (Some(plan), Some(tcp)) = (&result.plan, &result.tcp) else {
            continue;
        };
        match tcp.usable_window {
            Some(window) => {
                bound = bound.or_else(|| {
                    plan.points
                        .iter()
                        .find(|(_, point)| {
                            raised_buffer(point, window, tcp.payload_share).is_some()
                        })
                        .map(|(_, point)| point.key())
                });
            }
            None if plan
                .points
                .iter()
                .any(|(_, point)| matches!(point.link, Link::Emulated { .. })) =>
            {
                failures.push(Failure::UnknownWindow {
                    dir: result.dir.clone(),
                });
            }
            None => {}
        }
    }
    let reran = rows
        .iter()
        .any(|row| row.kind == Kind::Raised.name() && row.ok());
    if let Some(point) = bound
        && !reran
    {
        failures.push(Failure::NotRaised { point });
    }
}

/// Checks the results in `dirs`, one run's directory or every shard's,
/// rendering their report into `out`.
pub fn check(dirs: &[PathBuf], out: &Path) -> Outcome {
    let mut failures = Vec::new();
    let results: Vec<Results> = dirs.iter().map(|dir| load(dir, &mut failures)).collect();
    failures.extend(verify(&results));
    if let Err(error) = report::render(dirs, out) {
        failures.push(Failure::Report(format!("{error:#}")));
    }
    let rows = || results.iter().flat_map(|r| &r.rows);
    Outcome {
        failures,
        sessions: rows().count(),
        skipped: results
            .iter()
            .find_map(|r| r.plan.as_ref())
            .map_or(0, |plan| plan.skipped.len()),
        raised: rows().filter(|row| row.kind == Kind::Raised.name()).count(),
    }
}

#[cfg(test)]
mod tests;
