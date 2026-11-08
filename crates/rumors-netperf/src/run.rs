//! Running a grid: the sessions prepared many at once and timed in random
//! order, with progress, sentinels, and the results directory.
//!
//! # The results directory
//!
//! - `sessions.csv`: one row per session ([`crate::record::Row`]),
//!   appended and synced to the disk as each finishes, so a run cut short,
//!   by a killed process or a crash of the host, keeps what it finished;
//! - `environment.json`: the host, kernel, TCP limits, and build;
//! - `plan.json`: what the run set out to measure ([`Plan`]): the grid's
//!   points, the points it skipped and why, and the sessions of this run;
//! - `run.json`: how the run was invoked and what happened to it
//!   ([`RunRecord`]), written before the first session and again when it
//!   ends, which is how a reader tells a finished run from one cut short;
//! - `calibration.json`: the calibration the job count came from, if any;
//! - `report.md` and its plots, rendered from the files above when the run
//!   ends, and again by `rumors-netperf report`.
//!
//! # Preparing and timing
//!
//! Each session is prepared, then timed ([`Executor`]). The run prepares
//! many sessions at once and times them only while no preparation runs,
//! as many at once as calibration's job count, in windows that bound the
//! memory prepared sessions hold while they wait ([`crate::schedule`]).
//! Each row records when its session's preparation and timed session ran,
//! on one monotonic clock per part, so `check` can confirm that no timed
//! session overlapped a preparation ([`crate::check`]).
//!
//! # Order and sentinels
//!
//! Sessions are timed in an order shuffled afresh for every run, so a
//! drift in the host's state over the run spreads across points instead
//! of landing on whichever points happened to run last. Calibration's noise
//! probe recurs through the run as a *sentinel*: when the median of the
//! last three sentinels exceeds the slowest session calibration saw at the
//! chosen job count, the host has drifted (its load changed, or the
//! calibration was optimistic). The run says so as it happens and the
//! report flags it; it never drops or adjusts a measurement.
//!
//! # Resuming a run cut short
//!
//! A run is a sequence of *parts*: the part [`Part::begin`] starts, and a
//! part [`Part::resume`] starts for each time the run is continued after
//! it was cut short. A resumed part reads the directory back and runs
//! exactly the sessions without a row, so the parts together record every
//! planned session once:
//!
//! - It refuses unless this build, host, and kernel match the run's
//!   ([`Environment::differences`]): rows measured under two of them are
//!   not one run's.
//! - It runs as the first part did: the same plan, shard, job count,
//!   preparation limits, calibration, and sentinels, from the directory's
//!   files.
//! - It identifies a session by its group, point, kind, and repetition
//!   ([`Planned`]), and skips each session `sessions.csv` records, failed
//!   ones included.
//! - It removes a torn final row before appending ([`record::recover`]),
//!   and runs that session again.
//! - Its rows continue the run's sequence numbers, and the sentinels'
//!   drift check continues from the sentinels already recorded.
//! - It adds a [`Resumption`] to `run.json`.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Write;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::calibrate::Calibration;
use crate::driver::Measured;
use crate::env::{Environment, load_average};
use crate::grid::{self, Kind, Session, Shard, Skip};
use crate::point::Point;
use crate::predict::{CostModel, MemoryModel};
use crate::record::{self, Interval, Placement, Row, Writer, sync_parent};
use crate::schedule::{Job, Limits, Scheduler, Step};
use crate::stats::summarize;
use crate::units::human_seconds;

/// The share of a run's predicted time its sentinels may take.
const SENTINEL_SHARE: f64 = 0.05;

/// The fewest sentinels a calibrated run includes.
const MIN_SENTINELS: usize = 3;

/// The most sentinels a run includes.
const MAX_SENTINELS: usize = 30;

/// Sentinels whose median the drift check compares.
const DRIFT_WINDOW: usize = 3;

/// The [`RunRecord::mode`] of a run on emulated links.
pub const EMULATION: &str = "emulation";

/// The [`RunRecord::mode`] of a run between two hosts.
pub const PAIR: &str = "pair";

/// What `run.json` records: how the run was set up, written before its
/// first session, and what happened to it, written again when it ends.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RunRecord {
    /// [`EMULATION`] or [`PAIR`].
    pub mode: String,
    /// The command line that started the run.
    pub command: Vec<String>,
    /// Seconds since the Unix epoch when the run started.
    pub started: u64,
    /// Seconds since the Unix epoch when the run's last part ended; empty
    /// while a part runs, and in a run cut short.
    #[serde(default)]
    pub ended: Option<u64>,
    /// Wall seconds from the run's start to its end, including any time it
    /// spent cut short before it resumed; zero until it ends.
    pub seconds: f64,
    /// Timed sessions run at once.
    pub jobs: usize,
    /// Where the job count came from.
    pub jobs_source: String,
    /// How the run prepares its sessions; empty in the record of a run
    /// that prepared each session just before timing it.
    #[serde(default)]
    pub preparation: Option<Preparation>,
    /// The shard this run covered, if any.
    pub shard: Option<Shard>,
    /// Sentinels the run interleaves through its sessions.
    #[serde(default)]
    pub sentinels: usize,
    /// Sessions the run sets out to run, sentinels included.
    pub sessions: usize,
    /// Sessions that failed, across every part; counted when the run ends.
    pub failures: usize,
    /// The sentinel bound, in seconds, if the run had sentinels.
    pub sentinel_bound_seconds: Option<f64>,
    /// Each moment the sentinels' recent median exceeded the bound: the
    /// session sequence number and the median.
    pub drift: Vec<(u64, f64)>,
    /// The host's usable TCP window, in bytes, as the run computed it.
    pub usable_window: Option<u64>,
    /// Each time the run resumed after it was cut short, in order.
    #[serde(default)]
    pub resumptions: Vec<Resumption>,
}

/// How a run prepares its sessions, as `run.json` records it: the bounds
/// [`crate::schedule`] keeps besides the timed job count.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preparation {
    /// Preparations run at once.
    pub jobs: usize,
    /// Where the preparation count came from.
    pub jobs_source: String,
    /// The most predicted bytes the sessions of one window hold.
    pub memory_bytes: u64,
    /// The most sessions one window holds.
    pub window: usize,
}

impl RunRecord {
    /// The bounds the run schedules within, if it records them.
    pub fn limits(&self) -> Option<Limits> {
        self.preparation.as_ref().map(|preparation| Limits {
            prepare_jobs: preparation.jobs,
            timed_jobs: self.jobs,
            memory_bytes: preparation.memory_bytes,
            window: preparation.window,
        })
    }
}

/// One time a run resumed after it was cut short, as `run.json` records it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Resumption {
    /// Seconds since the Unix epoch when it resumed.
    pub started: u64,
    /// Sessions `sessions.csv` already recorded whole, which it skipped.
    pub found: usize,
    /// Sessions it ran: every session of the run `sessions.csv` did not
    /// record whole. A part that was itself cut short ran only some of
    /// them, and the next resumption finds the rest.
    pub ran: usize,
    /// Bytes of a torn final row it removed before appending; zero when
    /// the file ended on a whole row.
    pub torn_bytes: u64,
}

/// What `plan.json` records: what a run set out to measure, written
/// before its first session, so a run cut short still says what it lacks.
///
/// Every shard of one run records the same points and skips, the whole
/// grid's; each records only its own sessions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    /// Sessions per point.
    pub repetitions: u32,
    /// The shard this run covered, if any.
    pub shard: Option<Shard>,
    /// Every point the grid plans, with its group, in grid order: in pair
    /// mode, with the link the network between the hosts.
    pub points: Vec<(String, Point)>,
    /// The points the grid skips, with their reasons, keyed as the grid
    /// describes them.
    pub skipped: Vec<Skip>,
    /// This run's sessions, raised-limit reruns included and sentinels
    /// aside.
    pub sessions: Vec<Planned>,
}

/// One session a run plans, by the fields of its row in `sessions.csv`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Planned {
    /// The point's group.
    pub group: String,
    /// The point's key.
    pub point: String,
    /// What the session is for, as [`Kind::name`] spells it.
    pub kind: String,
    /// Which repetition of its point it is, from zero.
    pub repetition: u32,
}

impl Planned {
    /// The planned session naming `session`.
    pub fn of(session: &Session) -> Self {
        Planned {
            group: session.group.clone(),
            point: session.point.key(),
            kind: session.kind.name().to_string(),
            repetition: session.repetition,
        }
    }

    /// The session `row` records.
    pub fn recorded(row: &Row) -> Self {
        Planned {
            group: row.group.clone(),
            point: row.point.clone(),
            kind: row.kind.clone(),
            repetition: row.repetition,
        }
    }
}

impl Plan {
    /// The plan of a run of `points` and `skipped`, `repetitions` times
    /// each, whose sessions are `sessions`.
    pub fn new(
        repetitions: u32,
        shard: Option<Shard>,
        points: Vec<(String, Point)>,
        skipped: Vec<Skip>,
        sessions: &[Session],
    ) -> Self {
        Plan {
            repetitions,
            shard,
            points,
            skipped,
            sessions: sessions
                .iter()
                .filter(|s| s.kind != Kind::Sentinel)
                .map(Planned::of)
                .collect(),
        }
    }
}

/// Seconds since the Unix epoch.
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Writes `value` as pretty JSON to `dir/name`, replacing any file there
/// whole, and returns once the file is on the disk.
///
/// The JSON goes to a temporary file beside the target, synced, which is
/// then renamed over it, so a reader sees either the old file or the new
/// one, never a mixture, even after a crash of the host.
pub fn write_json<T: Serialize>(dir: &Path, name: &str, value: &T) -> Result<()> {
    let path = dir.join(name);
    let partial = dir.join(format!(".{name}.partial"));
    let write = || -> std::io::Result<()> {
        let mut file = File::create(&partial)?;
        file.write_all(serde_json::to_string_pretty(value)?.as_bytes())?;
        file.sync_data()?;
        fs::rename(&partial, &path)
    };
    write().with_context(|| format!("writing {}", path.display()))?;
    sync_parent(&path)
}

/// Reads `dir/name` as JSON.
fn read_json<T: for<'de> Deserialize<'de>>(dir: &Path, name: &str) -> Result<T> {
    let path = dir.join(name);
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

/// The default results directory: `netperf-<unix seconds>` in the current
/// directory.
pub fn default_out() -> PathBuf {
    PathBuf::from(format!("netperf-{}", now()))
}

/// Interleaves `sentinels` evenly through `sessions`, in their order: one
/// before each stretch of sessions, and any left over at the end.
pub fn with_sentinels(sessions: Vec<Session>, sentinels: Vec<Session>) -> Vec<Session> {
    if sentinels.is_empty() {
        return sessions;
    }
    let stride = sessions.len().div_ceil(sentinels.len()).max(1);
    let mut out = Vec::with_capacity(sessions.len() + sentinels.len());
    let mut sentinels = sentinels.into_iter();
    for (index, session) in sessions.into_iter().enumerate() {
        if index.is_multiple_of(stride)
            && let Some(sentinel) = sentinels.next()
        {
            out.push(sentinel);
        }
        out.push(session);
    }
    out.extend(sentinels);
    out
}

/// The order a part runs its sessions in: `sessions` shuffled, with
/// `sentinels` interleaved through them.
pub fn order(mut sessions: Vec<Session>, sentinels: Vec<Session>) -> Vec<Session> {
    shuffle(&mut sessions);
    with_sentinels(sessions, sentinels)
}

/// How many sentinels of `calibration`'s noise point a run of `sessions`
/// gets: enough to take [`SENTINEL_SHARE`] of the predicted time, within
/// [`MIN_SENTINELS`] and [`MAX_SENTINELS`].
///
/// A run records the count in `run.json` rather than have a resumed part
/// count again: the sum of predicted times depends on the order it adds
/// them in, by rounding, so a count near a boundary could come out
/// differently.
pub fn sentinel_count(sessions: &[Session], calibration: &Calibration) -> usize {
    let costs = &calibration.costs;
    let total: f64 = sessions.iter().map(|s| costs.total_seconds(&s.point)).sum();
    let each = costs
        .total_seconds(&calibration.noise_point)
        .max(f64::MIN_POSITIVE);
    ((SENTINEL_SHARE * total / each) as usize).clamp(MIN_SENTINELS, MAX_SENTINELS)
}

/// Shuffles the order sessions run in.
pub fn shuffle(sessions: &mut [Session]) {
    sessions.shuffle(&mut rand::rng());
}

/// Keeps only the sessions `shard` owns.
pub fn shard(sessions: Vec<Session>, shard: Option<Shard>) -> Vec<Session> {
    match shard {
        None => sessions,
        Some(shard) => sessions.into_iter().filter(|s| shard.owns(s)).collect(),
    }
}

/// How a run prepares and times its sessions: in emulation, peer
/// processes in network namespaces on this host ([`crate::emulate`]); in
/// pair mode, one peer here and one behind `serve` ([`crate::pair`]).
pub trait Executor: Sync {
    /// A prepared session: its processes running and idle, holding the
    /// pair, waiting for its timed session.
    type Prepared: Send;

    /// Prepares `session` in preparation slot `slot`, which no other
    /// preparation running at the same time holds.
    ///
    /// # Errors
    ///
    /// If the preparation fails; the session's row records the error.
    fn prepare(&self, session: &Session, slot: usize) -> Result<Self::Prepared>;

    /// Runs the timed session of `prepared`, and returns once its
    /// processes on this host have exited.
    ///
    /// # Errors
    ///
    /// If a peer fails or answers out of order; the session's row records
    /// the error. A session that fails is not an error: its outcomes say
    /// so.
    fn finish(&self, session: &Session, prepared: Self::Prepared) -> Result<Measured>;

    /// The `netem` queue limits `session` runs with, if it runs on an
    /// emulated link.
    fn queues(&self, _session: &Session) -> Option<(u64, u64)> {
        None
    }
}

/// A part's clock: monotonic, read in seconds since the Unix epoch, so
/// intervals from one part compare exactly and parts order by when they
/// ran.
#[derive(Clone, Copy, Debug)]
struct Clock {
    /// The instant the part's clock started.
    start: Instant,
    /// The Unix time it started at, in seconds.
    epoch: f64,
}

impl Clock {
    /// A clock starting now.
    fn new() -> Self {
        Self {
            start: Instant::now(),
            epoch: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0.0, |d| d.as_secs_f64()),
        }
    }

    /// The time now, in seconds since the Unix epoch.
    fn now(&self) -> f64 {
        self.epoch + self.start.elapsed().as_secs_f64()
    }
}

/// What a step running on its own thread reports when it finishes.
enum Event<P> {
    /// A preparation finished, or failed.
    Prepared {
        /// The session's position in the order.
        index: usize,
        /// The preparation slot it held.
        slot: usize,
        /// The prepared session, or why preparing it failed.
        prepared: Result<P>,
        /// When the preparation ran.
        interval: Interval,
    },
    /// A timed session finished, and its row records it.
    Timed {
        /// The session's position in the order.
        index: usize,
        /// The timed slot it held.
        slot: usize,
        /// Its row.
        row: Box<Row>,
    },
    /// A step's thread panicked: a defect, which the run propagates.
    Panicked(Box<dyn std::any::Any + Send>),
}

/// The jobs [`Scheduler`] sees for `sessions`: each one's predicted
/// preparation and memory.
pub fn jobs(sessions: &[Session], costs: &CostModel, memory: &MemoryModel) -> Vec<Job> {
    sessions
        .iter()
        .map(|session| Job {
            prepare_seconds: costs.prepare_seconds(&session.point),
            bytes: memory.bytes(&session.point),
        })
        .collect()
}

/// The sentinels' drift check: whether the median of the last
/// [`DRIFT_WINDOW`] sentinels exceeds the calibration's bound.
#[derive(Clone, Debug, Default)]
pub struct Drift {
    /// The bound, when the run has a calibration.
    bound: Option<f64>,
    /// The wall seconds of every sentinel observed, in order.
    sentinels: Vec<f64>,
    /// Each moment the median exceeded the bound: the sequence number of
    /// the sentinel that completed the window, and the median.
    events: Vec<(u64, f64)>,
}

impl Drift {
    /// A drift check against `bound`, or one that never fires without it.
    pub fn new(bound: Option<f64>) -> Self {
        Drift {
            bound,
            ..Drift::default()
        }
    }

    /// Observes the next row to finish, returning the recent median and the
    /// bound when this row is a sentinel that brings the median above it.
    pub fn observe(&mut self, row: &Row) -> Option<(f64, f64)> {
        if row.kind != Kind::Sentinel.name() {
            return None;
        }
        let (wall, bound) = (row.wall_seconds?, self.bound?);
        self.sentinels.push(wall);
        let recent = &self.sentinels[self.sentinels.len().saturating_sub(DRIFT_WINDOW)..];
        if recent.len() < DRIFT_WINDOW {
            return None;
        }
        let median = summarize(recent, 0.0).map_or(0.0, |s| s.median);
        (median > bound).then(|| {
            self.events.push((row.sequence, median));
            (median, bound)
        })
    }

    /// Every moment the median exceeded the bound, in order.
    pub fn events(&self) -> &[(u64, f64)] {
        &self.events
    }
}

/// One part of a run, ready to run its sessions: the whole run, or what a
/// run cut short has left.
#[derive(Debug)]
pub struct Part {
    /// The results directory.
    dir: PathBuf,
    /// The run's record, as `run.json` holds it while this part runs.
    record: RunRecord,
    /// The calibration the run's job count and models came from, if any.
    calibration: Option<Calibration>,
    /// The bounds the part schedules within.
    limits: Limits,
    /// This part's sessions, in the order it runs them.
    sessions: Vec<Session>,
    /// The sequence number of this part's first session.
    first_sequence: u64,
    /// Failures among the rows earlier parts recorded.
    failures: usize,
    /// The drift check, having observed the rows earlier parts recorded.
    drift: Drift,
    /// This part's resumption, if it continues a run.
    resumption: Option<Resumption>,
    /// The writer appending to `sessions.csv`.
    writer: Writer,
}

impl Part {
    /// Begins a run in `dir`, creating it, and writing every file that
    /// describes the run before its first session: the environment, the
    /// plan, the calibration, and `record`, which [`Part::run`] completes.
    ///
    /// `sessions` are the run's, in the order it times them, sentinels
    /// included; `record` counts them, names the sentinels, and records the
    /// preparation limits.
    ///
    /// # Errors
    ///
    /// If `dir` already holds a run's `run.json` or `sessions.csv`, finished
    /// or cut short, before writing anything: beginning there would
    /// overwrite that run's results, or the rows `run --resume` finishes it
    /// from. Also if `record` has no preparation limits, or a file cannot be
    /// written.
    pub fn begin(
        dir: &Path,
        environment: &Environment,
        plan: &Plan,
        calibration: Option<Calibration>,
        sessions: Vec<Session>,
        record: RunRecord,
    ) -> Result<Self> {
        for file in ["run.json", "sessions.csv"] {
            let path = dir.join(file);
            let held = path
                .try_exists()
                .with_context(|| format!("looking for {}", path.display()))?;
            anyhow::ensure!(
                !held,
                "{} already holds a run's {file}; `rumors-netperf run --resume {}` finishes a run \
                 cut short, and a new run needs a directory of its own",
                dir.display(),
                dir.display()
            );
        }
        let limits = record
            .limits()
            .context("a run records how it prepares its sessions")?;
        fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        write_json(dir, "environment.json", environment)?;
        write_json(dir, "plan.json", plan)?;
        if let Some(calibration) = &calibration {
            write_json(dir, "calibration.json", calibration)?;
        }
        write_json(dir, "run.json", &record)?;
        let writer = Writer::create(&dir.join("sessions.csv"))?;
        Ok(Part {
            dir: dir.to_path_buf(),
            drift: Drift::new(record.sentinel_bound_seconds),
            record,
            calibration,
            limits,
            sessions,
            first_sequence: 0,
            failures: 0,
            resumption: None,
            writer,
        })
    }

    /// Resumes the emulated run in `dir`, which was cut short, as described
    /// in [this module's docs](self): its sessions are the run's sessions
    /// that `sessions.csv` does not record whole.
    ///
    /// # Errors
    ///
    /// If `environment` differs from the run's, naming each difference; if
    /// a file the run writes before its first session is absent or cannot
    /// be read; if the run already finished, or ran in pair mode; or if the
    /// recorded rows are not the run's.
    pub fn resume(dir: &Path, environment: &Environment) -> Result<Self> {
        let recorded: Environment = read_json(dir, "environment.json")?;
        let differences: Vec<String> = environment
            .differences(&recorded)
            .into_iter()
            .map(|d| {
                format!(
                    "the {} is `{}` here but `{}` in the run",
                    d.part, d.ours, d.theirs
                )
            })
            .collect();
        anyhow::ensure!(
            differences.is_empty(),
            "{} holds a run measured under another environment, and rows measured under two \
             environments are not one run's: {}",
            dir.display(),
            differences.join("; ")
        );
        let plan: Plan = read_json(dir, "plan.json")?;
        let mut record: RunRecord = read_json(dir, "run.json")?;
        anyhow::ensure!(
            record.mode == EMULATION,
            "{} holds a {} run; `run --resume` continues emulated runs only",
            dir.display(),
            record.mode
        );
        anyhow::ensure!(
            record.ended.is_none(),
            "{} holds a run that finished; there is nothing to resume",
            dir.display()
        );
        let limits = record.limits().with_context(|| {
            format!(
                "{}: run.json records no preparation limits, so another build began the run",
                dir.display()
            )
        })?;
        let calibration: Option<Calibration> = dir
            .join("calibration.json")
            .exists()
            .then(|| read_json(dir, "calibration.json"))
            .transpose()?;
        let sentinels = match &calibration {
            Some(calibration) => sentinels(calibration, record.sentinels),
            None => {
                anyhow::ensure!(
                    record.sentinels == 0,
                    "{}: run.json records {} sentinels, but no calibration.json says what they run",
                    dir.display(),
                    record.sentinels
                );
                Vec::new()
            }
        };
        let window = environment
            .tcp
            .usable_window
            .map(|window| (window, environment.tcp.payload_share));
        let planned = shard(
            grid::sessions(&plan.points, plan.repetitions, window),
            plan.shard,
        );
        let mut made: Vec<Planned> = planned.iter().map(Planned::of).collect();
        let mut listed = plan.sessions.clone();
        made.sort();
        listed.sort();
        anyhow::ensure!(
            made == listed,
            "{}: the sessions plan.json lists are not the ones its points make on this host",
            dir.display()
        );

        let path = dir.join("sessions.csv");
        let recovered = record::recover(&path)?;
        if recovered.torn > 0 {
            eprintln!(
                "{}: removing a torn final row of {} bytes, left when the run was cut short; \
                 its session runs again",
                path.display(),
                recovered.torn
            );
        }
        let done: BTreeSet<Planned> = recovered.rows.iter().map(Planned::recorded).collect();
        let known: BTreeSet<Planned> = planned.iter().chain(&sentinels).map(Planned::of).collect();
        if let Some(stray) = done.iter().find(|session| !known.contains(*session)) {
            anyhow::bail!(
                "{} records the {stray}, which the run does not plan",
                path.display()
            );
        }
        let undone = |sessions: Vec<Session>| -> Vec<Session> {
            sessions
                .into_iter()
                .filter(|session| !done.contains(&Planned::of(session)))
                .collect()
        };
        let sessions = order(undone(planned), undone(sentinels));
        let mut drift = Drift::new(record.sentinel_bound_seconds);
        for row in &recovered.rows {
            drift.observe(row);
        }
        record.resumptions.push(Resumption {
            started: now(),
            found: recovered.rows.len(),
            ran: sessions.len(),
            torn_bytes: recovered.torn,
        });
        let resumption = record.resumptions.last().cloned();
        let writer = Writer::resume(&path, recovered.whole)?;
        write_json(dir, "run.json", &record)?;
        Ok(Part {
            dir: dir.to_path_buf(),
            first_sequence: recovered
                .rows
                .iter()
                .map(|row| row.sequence + 1)
                .max()
                .unwrap_or(0),
            failures: recovered.rows.iter().filter(|row| !row.ok()).count(),
            writer,
            record,
            calibration,
            limits,
            sessions,
            drift,
            resumption,
        })
    }

    /// This part's sessions, in the order it runs them.
    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    /// The resumption this part records, if it continues a run.
    pub fn resumption(&self) -> Option<&Resumption> {
        self.resumption.as_ref()
    }

    /// The per-message costs the schedule and the progress bar predict
    /// with: the calibration's, or the defaults.
    pub fn costs(&self) -> CostModel {
        self.calibration
            .as_ref()
            .map_or_else(CostModel::default, |c| c.costs)
    }

    /// The memory model the schedule bounds prepared sessions with: the
    /// calibration's, or the default.
    pub fn memory(&self) -> MemoryModel {
        self.calibration
            .as_ref()
            .map_or_else(MemoryModel::default, |c| c.memory)
    }

    /// The bounds this part schedules within.
    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// The calibration the run's job count and models came from, if any.
    pub fn calibration(&self) -> Option<&Calibration> {
        self.calibration.as_ref()
    }

    /// Prepares and times `sessions`, in their order, as [`Scheduler`]
    /// directs.
    ///
    /// It numbers them from this part's first sequence number, appends
    /// each row as its timed session ends, and observes it for drift, with
    /// a progress bar counted in predicted seconds.
    ///
    /// Each step runs on a thread of its own. The run appends a row, and
    /// syncs it, before it starts the next step, so with one timed job the
    /// sync never overlaps a timed session.
    ///
    /// Returns the rows' failure count.
    fn execute<E: Executor>(&mut self, sessions: Vec<Session>, executor: &E) -> Result<usize> {
        let (costs, limits) = (self.costs(), self.limits);
        let predicted: Vec<f64> = sessions
            .iter()
            .map(|s| costs.total_seconds(&s.point))
            .collect();
        let total_millis: u64 = predicted.iter().map(|s| (s * 1000.0) as u64).sum();
        let count = sessions.len();
        let bar = ProgressBar::new(total_millis.max(1));
        bar.set_style(
            ProgressStyle::with_template(
                "{elapsed_precise} [{wide_bar}] {percent:>3}% of predicted work, ETA {eta_precise}\n{msg}",
            )
            .expect("the progress template is valid")
            .progress_chars("=> "),
        );
        bar.enable_steady_tick(Duration::from_millis(500));
        let mut scheduler = Scheduler::new(jobs(&sessions, &costs, &self.memory()), limits);
        let mut free_prepare: Vec<usize> = (0..limits.prepare_jobs).rev().collect();
        let mut free_timed: Vec<usize> = (0..limits.timed_jobs).rev().collect();
        let mut waiting: Vec<Option<(Result<E::Prepared>, Interval)>> =
            sessions.iter().map(|_| None).collect();
        let (sender, receiver) = mpsc::channel::<Event<E::Prepared>>();
        let clock = Clock::new();
        let began = Instant::now();
        let first_sequence = self.first_sequence;
        let (writer, drift) = (&mut self.writer, &mut self.drift);
        let mut failures = 0;
        let mut done = 0;
        let sessions = &sessions;
        thread::scope(|scope| -> Result<()> {
            loop {
                for step in scheduler.steps() {
                    let sender = sender.clone();
                    match step {
                        Step::Prepare(index) => {
                            let slot = free_prepare
                                .pop()
                                .expect("the scheduler runs no more preparations than slots");
                            scope.spawn(move || {
                                let event = panic::catch_unwind(AssertUnwindSafe(|| {
                                    let began = clock.now();
                                    let prepared = executor.prepare(&sessions[index], slot);
                                    let interval = Interval {
                                        began,
                                        ended: clock.now(),
                                    };
                                    Event::Prepared {
                                        index,
                                        slot,
                                        prepared,
                                        interval,
                                    }
                                }));
                                let _ = sender.send(event.unwrap_or_else(Event::Panicked));
                            });
                        }
                        Step::Time(index) => {
                            let slot = free_timed
                                .pop()
                                .expect("the scheduler runs no more timed sessions than slots");
                            let (prepared, interval) = waiting[index]
                                .take()
                                .expect("the scheduler times only prepared sessions");
                            scope.spawn(move || {
                                let event = panic::catch_unwind(AssertUnwindSafe(|| {
                                    let session = &sessions[index];
                                    let mut context = Placement {
                                        sequence: first_sequence + index as u64,
                                        slot,
                                        jobs: limits.timed_jobs,
                                        load_1m: load_average().map(|l| l[0]),
                                        queues: executor.queues(session),
                                        prepared: Some(interval),
                                        timed: None,
                                    };
                                    let row = match prepared {
                                        Ok(prepared) => {
                                            let began = clock.now();
                                            let measured = executor.finish(session, prepared);
                                            context.timed = Some(Interval {
                                                began,
                                                ended: clock.now(),
                                            });
                                            match measured {
                                                Ok(measured) => {
                                                    Row::measured(session, context, &measured)
                                                }
                                                Err(error) => Row::failed(session, context, &error),
                                            }
                                        }
                                        Err(error) => Row::failed(session, context, &error),
                                    };
                                    Event::Timed {
                                        index,
                                        slot,
                                        row: Box::new(row),
                                    }
                                }));
                                let _ = sender.send(event.unwrap_or_else(Event::Panicked));
                            });
                        }
                    }
                }
                if scheduler.finished() {
                    return Ok(());
                }
                match receiver.recv().expect("the run holds a sender") {
                    Event::Prepared {
                        index,
                        slot,
                        prepared,
                        interval,
                    } => {
                        free_prepare.push(slot);
                        waiting[index] = Some((prepared, interval));
                        scheduler.prepared(index);
                    }
                    Event::Timed { index, slot, row } => {
                        free_timed.push(slot);
                        writer.append(&row)?;
                        done += 1;
                        if !row.ok() {
                            failures += 1;
                            bar.println(format!(
                                "session {} failed: {}",
                                row.sequence,
                                row.error.as_deref().unwrap_or("no measurement")
                            ));
                        }
                        if let Some((median, bound)) = drift.observe(&row) {
                            bar.println(format!(
                                "drift: the last {DRIFT_WINDOW} sentinels' median, {}, exceeds calibration's slowest, {}; \
                                 the host's load may have changed (the report flags the sessions after this point)",
                                human_seconds(median),
                                human_seconds(bound)
                            ));
                        }
                        bar.inc((predicted[index] * 1000.0) as u64);
                        let flagged = drift.events().len();
                        let message = format!(
                            "{done}/{count} sessions, {failures} failed{}; last: {} in {}",
                            if flagged == 0 {
                                String::new()
                            } else {
                                format!(", drift flagged {flagged} times")
                            },
                            row.point,
                            row.wall_seconds.map_or("-".into(), human_seconds),
                        );
                        if bar.is_hidden() {
                            eprintln!(
                                "[{}] {message}",
                                human_seconds(began.elapsed().as_secs_f64())
                            );
                        }
                        bar.set_message(message);
                        scheduler.timed(index);
                    }
                    Event::Panicked(payload) => panic::resume_unwind(payload),
                }
            }
        })?;
        bar.finish_and_clear();
        Ok(failures)
    }

    /// Runs this part's sessions with `executor`, then completes `run.json`
    /// and returns the record it holds.
    pub fn run<E: Executor>(mut self, executor: &E) -> Result<RunRecord> {
        let sessions = std::mem::take(&mut self.sessions);
        let failures = self.execute(sessions, executor)?;
        let ended = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        self.record.ended = Some(ended.as_secs());
        self.record.seconds = (ended.as_secs_f64() - self.record.started as f64).max(0.0);
        self.record.failures = self.failures + failures;
        self.record.drift = self.drift.events().to_vec();
        write_json(&self.dir, "run.json", &self.record)?;
        Ok(self.record)
    }
}

/// The `count` sentinel sessions of a calibration's noise point, numbered
/// from zero.
pub fn sentinels(calibration: &Calibration, count: usize) -> Vec<Session> {
    (0..count as u32)
        .map(|repetition| Session {
            point: calibration.noise_point,
            group: "sentinel".to_string(),
            kind: Kind::Sentinel,
            repetition,
        })
        .collect()
}

/// Refuses a build of the tool whose library carries `test-internals`.
///
/// Cargo unifies the library's features across everything one invocation
/// builds, so building this tool beside a crate that enables
/// `test-internals` compiles the feature into its copy of the library,
/// and the feature's hooks cost time on the paths the tool measures
/// ([`rumors::TEST_INTERNALS`]). We refuse at run time rather than fail
/// the build: a compile-time assertion would also break builds that only
/// check or lint the tool beside those crates, such as
/// `cargo check --workspace` and editors that check the whole workspace,
/// where nothing is measured.
///
/// # Errors
///
/// If the library carries the feature, with how to build the tool instead.
pub fn refuse_test_internals() -> Result<()> {
    anyhow::ensure!(
        !rumors::TEST_INTERNALS,
        "this build of rumors-netperf compiled the library with its `test-internals` feature, \
         whose hooks cost time on the paths the tool measures; build the tool on its own, with \
         `cargo build --release -p rumors-netperf` (or `just netperf`), never in one cargo \
         invocation with a crate that enables the feature"
    );
    Ok(())
}

/// The environment, refusing a build whose library carries
/// `test-internals` ([`refuse_test_internals`]), and a debug build of the
/// tool unless allowed.
pub fn checked_environment(allow_debug: bool) -> Result<Environment> {
    refuse_test_internals()?;
    let environment = Environment::capture()?;
    if environment.build.profile != "release" && !allow_debug {
        anyhow::bail!(
            "this is a debug build of rumors-netperf, whose timings say nothing about production; \
             build it with `cargo build --release -p rumors-netperf` (or pass --allow-debug-build to try the tool itself)"
        );
    }
    Ok(environment)
}

#[cfg(test)]
mod tests;
