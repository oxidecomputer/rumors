//! Self-calibrated parallelism: the largest number of sessions this host
//! can run at once without changing what they measure.
//!
//! Running sessions side by side shortens a run, but each session then
//! shares processors, memory bandwidth, and the kernel's network stack
//! with the others, which can slow its computation (noise) or distort the
//! emulated link (fidelity). Calibration measures both effects directly
//! and picks the largest job count at which neither is distinguishable
//! from the variation of sessions run alone.
//!
//! # The probes
//!
//! - **Noise:** the run's heaviest replica pair, with that point's
//!   settings and transport, on an uncapped link at the run's shortest
//!   round trip, where the session is bound by computation, so any
//!   slowdown from sharing the host shows in its wall time.
//! - **Fidelity:** a raw TCP bulk transfer and ping-pong
//!   ([`crate::probe`]) on the run's fastest shaped link, which stresses
//!   the kernel's shaping hardest. It judges the emulator alone,
//!   independently of the library and of this tool's fixtures. It runs
//!   without TLS whatever the sessions' transport: TLS adds computation and
//!   per-record bytes to a session, not to the link, and the noise probe
//!   and the sessions themselves measure that cost.
//!
//! # The criterion
//!
//! First each probe runs [`SOLO_RUNS`] times alone, which measures its own
//! *spread*: the range of those readings once the most extreme reading at
//! each end is set aside. Then, at 2, 4, 8, … jobs, as many copies of each
//! probe run at once, all starting their measured part together, in as
//! many rounds as give the level at least [`LEVEL_READINGS`] readings. A
//! level *passes* when, for the noise probe's wall time and for the
//! fidelity probe's goodput and round trip alike, the median of the level's
//! readings lies within one spread of the solo median. That is: concurrency
//! may move a reading by no more than the host already varies it, a
//! criterion relative to measured noise rather than a fixed percentage.
//! The ladder stops at the first level that fails, and the run uses the
//! last level that passed.
//!
//! The sample counts keep any one reading from deciding the verdict. A host
//! now and then gives a reading far from the rest, alone as at any job
//! count: a slow session, or a transfer whose goodput dips. Such a solo
//! reading lands at an end of the solo readings, where the spread sets it
//! aside, so it cannot widen the band. A median of three or more readings,
//! with every other reading inside the band, stays inside it however far
//! the one reading strays, and a level's median leaves the band only when
//! at least half its readings do. The constants' docs give the reason for
//! each count.
//!
//! When a level fails on fidelity while at least one job's worth of
//! processors sat idle, the shaping, not the processors, is the limit: the
//! kernel serializes it. Then more jobs on this host cannot help, and
//! `--shard` spreads the run across hosts instead.
//!
//! The job count governs only timed sessions, which is all the criterion
//! judges: the run never times a session while it prepares others
//! ([`crate::schedule`]), just as each level here prepares its copies
//! first and then starts them together.
//!
//! # Preparation
//!
//! Preparation measures nothing, so how many preparations run at once is
//! a question of throughput, not fidelity, and calibration answers it from
//! three measurements ([`PreparationProbe`]):
//!
//! - **Cores per preparation.** The solo noise preparations' processes
//!   report their CPU time, and both sides together spend some number of
//!   processors' worth of it per second of building and joining: side A
//!   alone computes while it builds, and both while B joins. The run
//!   prepares as many sessions at once as fill the host's processors at
//!   that rate.
//! - **Memory.** An empty pair's preparation and the noise pair's each
//!   report their processes' peak resident bytes, and the line through the
//!   two ([`MemoryModel::fit`]) predicts every session's, which bounds what
//!   the run's prepared sessions hold.
//! - **Rates at that count.** As many noise preparations as the run will
//!   run at once, within the memory bound, run together, and their build
//!   and join times give the cost model's preparation rates, so the
//!   predicted wall time accounts for preparations sharing the host. The
//!   empty pair's preparation gives the overhead every preparation pays.
//!
//! # The cache
//!
//! A calibration is kept per environment fingerprint and probe pair in the
//! user's cache directory, so later runs of the same build on the same
//! host and grid shape reuse it. `--recalibrate` discards it, and
//! `--jobs N` skips calibration altogether.

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

use crate::driver::{Measured, Prepared};
use crate::emulate;
use crate::env::{Environment, hex};
use crate::grid::Session;
use crate::netns::CpuTimes;
use crate::point::{Connection, Link, Point, Replicas, TcpBuffers};
use crate::predict::{CostModel, MemoryModel};
use crate::probe::ProbeResult;
use crate::stats::summarize;
use crate::units::{LinkRate, Rate, Rtt};

/// Readings of each probe taken alone, whose spread sets the band, as
/// [the module docs](self#the-criterion) explain.
///
/// With eleven readings, the spread (the second largest less the second
/// smallest) averages about 2.1 standard deviations of a normally
/// distributed reading: twice the expected second largest of eleven
/// standard normal draws, 1.06. So a level with no effect seldom fails on
/// its sampling alone, and a level that moves readings by more than about
/// two of the host's own standard deviations fails. Each solo noise
/// reading costs one preparation and session of the run's heaviest pair.
pub const SOLO_RUNS: usize = 11;

/// The fewest readings of each probe a concurrent level takes: it runs
/// rounds of its job count until it has at least this many.
///
/// A level's median leaves the band only when at least half its readings
/// do, so with eight a verdict that a level fails rests on four or more
/// readings, never on one slow session or one dipped transfer. A level of
/// at least that many jobs takes one round.
pub const LEVEL_READINGS: usize = 8;

// A median of at least three readings stays within the range of all but
// one of them, whatever the one reading is; with two, one reading moves
// the median halfway to itself.
const _: () = assert!(LEVEL_READINGS >= 3);

/// Preparations of an empty pair alone, whose median is the overhead every
/// preparation pays.
const EMPTY_PREPARATIONS: usize = 5;

/// Round trips in each fidelity probe's ping-pong.
const PINGS: u32 = 20;

/// Seconds of the fidelity link's rate in each probe's bulk transfer.
const BULK_SECONDS: f64 = 2.0;

/// The bulk transfer on an uncapped link.
const UNCAPPED_BULK_BYTES: u64 = 256 << 20;

/// What the host's two probes measured at one job count.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Level {
    /// Sessions run at once.
    pub jobs: usize,
    /// The noise probe's session wall seconds.
    pub session_seconds: Vec<f64>,
    /// The fidelity probe's goodput, in bytes per second.
    pub goodput: Vec<f64>,
    /// The fidelity probe's round trips, in seconds.
    pub rtt: Vec<f64>,
    /// The host's busy share of processor time during the fidelity probes:
    /// the largest of its rounds'.
    pub busy_share: f64,
    /// Whether concurrency stayed within the solo spread for the noise
    /// probe.
    pub noise_passed: bool,
    /// Whether it did for the fidelity probe.
    pub fidelity_passed: bool,
}

/// What calibration measured of preparation, as [the module
/// docs](self#preparation) describe.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PreparationProbe {
    /// Wall seconds of each preparation of an empty pair, alone.
    pub empty_seconds: Vec<f64>,
    /// The most resident bytes an empty pair's two processes reached.
    pub empty_bytes: u64,
    /// The most resident bytes the noise pair's two processes reached in
    /// a preparation alone.
    pub noise_bytes: u64,
    /// The processors' worth of CPU time both processes of a solo noise
    /// preparation spent per second of building and joining: the median
    /// over the solo preparations.
    pub cores_per_preparation: f64,
    /// Preparations the run runs at once: the host's processors divided by
    /// the cores each preparation keeps busy, at least one.
    pub prepare_jobs: usize,
    /// Noise preparations run together to measure the rates: the
    /// preparation count, or fewer if the memory bound holds fewer.
    pub copies: usize,
    /// Each copy's build seconds.
    pub build_seconds: Vec<f64>,
    /// Each copy's join seconds.
    pub fetch_seconds: Vec<f64>,
}

/// A calibration's result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Calibration {
    /// The environment fingerprint it was measured under.
    pub fingerprint: String,
    /// The noise probe's point.
    pub noise_point: Point,
    /// The fidelity probe's link, as a point.
    pub fidelity_point: Point,
    /// The probes run alone.
    pub solo: Level,
    /// Each concurrent level, in ladder order, up to the first that failed.
    pub levels: Vec<Level>,
    /// The job count the run uses.
    pub jobs: usize,
    /// The largest job count the ladder could try: one per two processors.
    pub max_jobs: usize,
    /// Why the ladder stopped, in words.
    pub verdict: String,
    /// What calibration measured of preparation; empty in a calibration
    /// from a tool that prepared each session just before timing it.
    #[serde(default)]
    pub preparation: PreparationProbe,
    /// Per-message rates: preparation's from the copies prepared together
    /// and the empty pair, the session's from the solo noise sessions.
    pub costs: CostModel,
    /// The memory model fitted to the empty and noise preparations; the
    /// default in a calibration from a tool that prepared each session
    /// just before timing it.
    #[serde(default)]
    pub memory: MemoryModel,
}

impl Calibration {
    /// The slowest session the calibration saw at the chosen job count or
    /// alone: the bound the run's sentinels are held to.
    ///
    /// Calibration accepted every one of those sessions as undisturbed, so
    /// a sentinel median above all of them means the host now runs slower
    /// than anything calibration accepted.
    pub fn sentinel_bound(&self) -> f64 {
        self.solo
            .session_seconds
            .iter()
            .chain(
                self.levels
                    .iter()
                    .filter(|level| level.jobs == self.jobs)
                    .flat_map(|level| &level.session_seconds),
            )
            .copied()
            .fold(0.0, f64::max)
    }
}

/// The two probe points for a run of `sessions`: the heaviest pair on an
/// uncapped link at the shortest round trip, and the fastest shaped link
/// at its shortest round trip.
pub fn probe_points(sessions: &[Session]) -> Result<(Point, Point)> {
    let emulated: Vec<(&Point, LinkRate, Rtt)> = sessions
        .iter()
        .filter_map(|s| match s.point.link {
            Link::Emulated { rate, rtt } => Some((&s.point, rate, rtt)),
            Link::Network => None,
        })
        .collect();
    let Some(heaviest) = emulated.iter().max_by_key(|(p, _, _)| {
        let r = &p.replicas;
        (r.only_a() + r.only_b(), r.live_a() + r.live_b())
    }) else {
        bail!("calibration needs at least one emulated point");
    };
    let shortest = emulated
        .iter()
        .map(|(_, _, rtt)| *rtt)
        .min()
        .expect("emulated is not empty");
    let noise_point = Point {
        link: Link::Emulated {
            rate: LinkRate::symmetric(Rate::Uncapped),
            rtt: shortest,
        },
        connection: Connection::Fresh,
        tcp_buffers: TcpBuffers::Host,
        ..*heaviest.0
    };
    let fastest = |rate: LinkRate| match (rate.a_to_b, rate.b_to_a) {
        (Rate::Bits(a), Rate::Bits(b)) => Some(a.max(b)),
        _ => None,
    };
    let fidelity_link = emulated
        .iter()
        .filter_map(|(_, rate, rtt)| {
            fastest(*rate).map(|bits| (bits, std::cmp::Reverse(*rtt), *rate, *rtt))
        })
        .max()
        .map(|(_, _, rate, rtt)| (rate, rtt))
        .unwrap_or((LinkRate::symmetric(Rate::Uncapped), shortest));
    let fidelity_point = Point {
        link: Link::Emulated {
            rate: fidelity_link.0,
            rtt: fidelity_link.1,
        },
        connection: Connection::Fresh,
        tcp_buffers: TcpBuffers::Host,
        ..*heaviest.0
    };
    Ok((noise_point, fidelity_point))
}

/// Where a calibration for `fingerprint` and these probes is cached.
pub fn cache_path(fingerprint: &str, noise: &Point, fidelity: &Point) -> Option<PathBuf> {
    let mut hasher = Sha3_256::new();
    hasher.update(noise.key().as_bytes());
    hasher.update(fidelity.key().as_bytes());
    let probes = hex(&hasher.finalize()[..6]);
    Some(
        dirs::cache_dir()?
            .join("rumors-netperf")
            .join(format!("calibration-{fingerprint}-{probes}.json")),
    )
}

/// Reads a cached calibration, if one exists and parses.
pub fn load(path: &Path) -> Option<Calibration> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

/// Writes a calibration to the cache.
pub fn store(path: &Path, calibration: &Calibration) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(path, serde_json::to_string_pretty(calibration)?)
        .with_context(|| format!("writing {}", path.display()))
}

/// The settings a calibration runs with.
#[derive(Clone, Copy, Debug)]
pub struct Setup<'a> {
    /// The peer executable.
    pub program: &'a Path,
    /// The host's usable TCP window.
    pub window: u64,
    /// The host's buffer payload share.
    pub payload_share: f64,
    /// The largest job count to try.
    pub max_jobs: usize,
    /// The host's logical processors, which preparations fill.
    pub logical_cpus: usize,
    /// The most predicted bytes the run's prepared sessions may hold,
    /// which bounds the noise copies prepared together.
    pub memory_bytes: u64,
}

/// The fidelity probe's bulk transfer for `point`'s link.
fn bulk_bytes(point: &Point) -> u64 {
    match point.link {
        Link::Emulated { rate, .. } => rate
            .fastest_bytes_per_second()
            .map_or(UNCAPPED_BULK_BYTES, |r| {
                ((r * BULK_SECONDS) as u64).clamp(1 << 20, 1 << 30)
            }),
        Link::Network => UNCAPPED_BULK_BYTES,
    }
}

/// Runs `jobs` noise sessions at once, all gossiping together, and
/// returns each one's measurements.
fn noise_level(setup: &Setup, point: &Point, jobs: usize) -> Result<Vec<Measured>> {
    let window = emulate::window(point, setup.window, setup.payload_share);
    let ready: Vec<emulate::Ready> = thread::scope(|scope| {
        let handles: Vec<_> = (0..jobs)
            .map(|slot| scope.spawn(move || emulate::prepare(setup.program, slot, point, window)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("a preparing thread panicked"))
            .collect::<Vec<_>>()
    })
    .into_iter()
    .collect::<Result<_>>()?;
    thread::scope(|scope| {
        let handles: Vec<_> = ready
            .into_iter()
            .map(|r| scope.spawn(move || emulate::finish(r)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("a session thread panicked"))
            .collect()
    })
}

/// Runs `jobs` fidelity probes at once and returns each one's result, with
/// the host's busy share while they ran.
fn fidelity_level(setup: &Setup, point: &Point, jobs: usize) -> Result<(Vec<ProbeResult>, f64)> {
    let window = emulate::window(point, setup.window, setup.payload_share);
    let bulk = bulk_bytes(point);
    let ready: Vec<emulate::ReadyProbe> = thread::scope(|scope| {
        let handles: Vec<_> = (0..jobs)
            .map(|slot| {
                scope.spawn(move || emulate::prepare_probe(setup.program, slot, point, window))
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("a preparing thread panicked"))
            .collect::<Vec<_>>()
    })
    .into_iter()
    .collect::<Result<_>>()?;
    let before = CpuTimes::read();
    let results: Result<Vec<ProbeResult>> = thread::scope(|scope| {
        let handles: Vec<_> = ready
            .into_iter()
            .map(|r| scope.spawn(move || emulate::finish_probe(r, bulk, PINGS)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("a probe thread panicked"))
            .collect()
    });
    let busy = before
        .zip(CpuTimes::read())
        .map_or(0.0, |(b, a)| a.busy_share_since(&b));
    Ok((results?, busy))
}

/// Prepares `copies` sessions of `point` at once and returns each one's
/// preparation, its processes stopped once measured.
fn prepare_level(setup: &Setup, point: &Point, copies: usize) -> Result<Vec<Prepared>> {
    let window = emulate::window(point, setup.window, setup.payload_share);
    thread::scope(|scope| {
        let handles: Vec<_> = (0..copies)
            .map(|slot| {
                scope.spawn(move || {
                    emulate::prepare(setup.program, slot, point, window).map(|r| *r.prepared())
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("a preparing thread panicked"))
            .collect()
    })
}

/// Both processes' peak resident bytes over a preparation, if read.
fn prepared_bytes(prepared: &Prepared) -> Option<u64> {
    prepared.usage.map(|[a, b]| a.rss_peak + b.rss_peak)
}

/// Measures preparation as [the module docs](self#preparation) describe,
/// from the solo noise sessions' preparations and new preparations of an
/// empty pair and of copies of `noise_point`.
fn probe_preparation(
    setup: &Setup,
    noise_point: &Point,
    solo: &[Measured],
    progress: &mut dyn FnMut(&str),
) -> Result<PreparationProbe> {
    let empty_point = Point {
        replicas: Replicas::default(),
        ..*noise_point
    };
    let window = emulate::window(&empty_point, setup.window, setup.payload_share);
    let mut probe = PreparationProbe::default();
    for run in 0..EMPTY_PREPARATIONS {
        progress(&format!(
            "empty pair prepared alone, {}/{EMPTY_PREPARATIONS}",
            run + 1
        ));
        let began = Instant::now();
        let ready = emulate::prepare(setup.program, 0, &empty_point, window)?;
        probe.empty_seconds.push(began.elapsed().as_secs_f64());
        probe.empty_bytes = probe
            .empty_bytes
            .max(prepared_bytes(ready.prepared()).unwrap_or(0));
    }
    probe.noise_bytes = solo
        .iter()
        .filter_map(|m| prepared_bytes(&m.prepared))
        .max()
        .unwrap_or(0);
    let cores: Vec<f64> = solo
        .iter()
        .filter_map(|m| {
            let [a, b] = m.prepared.usage?;
            let wall = m.prepared.build.seconds + m.prepared.fetch_seconds;
            (wall > 0.0).then(|| (a.cpu_seconds + b.cpu_seconds) / wall)
        })
        .collect();
    probe.cores_per_preparation = median(&cores);
    probe.prepare_jobs = if probe.cores_per_preparation > 0.0 {
        ((setup.logical_cpus as f64 / probe.cores_per_preparation) as usize)
            .clamp(1, setup.logical_cpus.max(1))
    } else {
        (setup.logical_cpus / 2).max(1)
    };
    let fits = match probe.noise_bytes {
        0 => probe.prepare_jobs,
        bytes => usize::try_from(setup.memory_bytes / bytes).unwrap_or(usize::MAX),
    };
    probe.copies = probe.prepare_jobs.min(fits).max(1);
    progress(&format!(
        "{} noise pairs prepared at once, for {} preparations at once",
        probe.copies, probe.prepare_jobs
    ));
    let prepared = prepare_level(setup, noise_point, probe.copies)?;
    probe.build_seconds = prepared.iter().map(|p| p.build.seconds).collect();
    probe.fetch_seconds = prepared.iter().map(|p| p.fetch_seconds).collect();
    Ok(probe)
}

/// The wall seconds of finished sessions, failing on any that failed.
fn session_seconds(measured: &[Measured]) -> Result<Vec<f64>> {
    measured
        .iter()
        .map(|m| match m.a.error.as_ref().or(m.b.error.as_ref()) {
            Some(error) => bail!("a calibration session failed: {error}"),
            None => Ok(m.b.wall_seconds),
        })
        .collect()
}

/// A probe's solo readings: their median and their spread.
#[derive(Clone, Copy, Debug)]
struct Band {
    /// The solo median.
    median: f64,
    /// The solo readings' range once the most extreme at each end is set
    /// aside: the second largest less the second smallest. With fewer than
    /// three readings nothing can be set aside, and it is the full range.
    spread: f64,
}

impl Band {
    /// The band of `solo` readings; readings that are not finite are left
    /// out.
    fn of(solo: &[f64]) -> Self {
        let mut sorted: Vec<f64> = solo.iter().copied().filter(|x| x.is_finite()).collect();
        sorted.sort_by(f64::total_cmp);
        let spread = match sorted.as_slice() {
            [] | [_] => 0.0,
            [low, high] => high - low,
            [_, inner @ .., _] => inner[inner.len() - 1] - inner[0],
        };
        Self {
            median: median(&sorted),
            spread,
        }
    }

    /// Whether the median of `concurrent` readings lies within one spread
    /// of the solo median.
    fn admits(&self, concurrent: &[f64]) -> bool {
        (median(concurrent) - self.median).abs() <= self.spread
    }
}

/// Rounds a level of `jobs` runs: the fewest whose readings, `jobs` a
/// round, number at least [`LEVEL_READINGS`].
fn rounds(jobs: usize) -> usize {
    LEVEL_READINGS.div_ceil(jobs.max(1))
}

/// The median of `values`.
fn median(values: &[f64]) -> f64 {
    summarize(values, 0.0).map_or(f64::NAN, |s| s.median)
}

/// Calibrates, reporting each step through `progress`.
///
/// # Errors
///
/// If any probe session fails.
pub fn calibrate(
    setup: &Setup,
    environment: &Environment,
    noise_point: &Point,
    fidelity_point: &Point,
    progress: &mut dyn FnMut(&str),
) -> Result<Calibration> {
    let mut solo = Level {
        jobs: 1,
        noise_passed: true,
        fidelity_passed: true,
        ..Level::default()
    };
    let mut measured_solo = Vec::new();
    for run in 0..SOLO_RUNS {
        progress(&format!("noise probe alone, {}/{SOLO_RUNS}", run + 1));
        measured_solo.extend(noise_level(setup, noise_point, 1)?);
    }
    solo.session_seconds = session_seconds(&measured_solo)?;
    for run in 0..SOLO_RUNS {
        progress(&format!("fidelity probe alone, {}/{SOLO_RUNS}", run + 1));
        let (results, busy) = fidelity_level(setup, fidelity_point, 1)?;
        solo.goodput
            .extend(results.iter().map(|r| r.goodput_bytes_per_second));
        solo.rtt.extend(results.iter().map(|r| r.rtt_seconds));
        solo.busy_share = busy;
    }
    let session_band = Band::of(&solo.session_seconds);
    let goodput_band = Band::of(&solo.goodput);
    let rtt_band = Band::of(&solo.rtt);

    let preparation = probe_preparation(setup, noise_point, &measured_solo, progress)?;
    let empty_point = Point {
        replicas: Replicas::default(),
        ..*noise_point
    };
    let costs = CostModel::fit(
        noise_point,
        median(&preparation.build_seconds),
        median(&preparation.fetch_seconds),
        median(&solo.session_seconds),
        median(&preparation.empty_seconds),
    );
    let memory = MemoryModel::fit(
        (&empty_point, preparation.empty_bytes),
        (noise_point, preparation.noise_bytes),
    );

    let mut levels = Vec::new();
    let mut jobs = 1;
    let mut verdict = format!(
        "every level up to {} jobs, one per two processors, stayed within the solo spread",
        setup.max_jobs
    );
    let mut level_jobs = 2;
    while level_jobs <= setup.max_jobs {
        let rounds = rounds(level_jobs);
        let mut session = Vec::new();
        for round in 0..rounds {
            progress(&format!(
                "{level_jobs} noise probes at once, {}/{rounds}",
                round + 1
            ));
            session.extend(session_seconds(&noise_level(
                setup,
                noise_point,
                level_jobs,
            )?)?);
        }
        let (mut goodput, mut rtt, mut busy_share) = (Vec::new(), Vec::new(), 0.0f64);
        for round in 0..rounds {
            progress(&format!(
                "{level_jobs} fidelity probes at once, {}/{rounds}",
                round + 1
            ));
            let (results, busy) = fidelity_level(setup, fidelity_point, level_jobs)?;
            goodput.extend(results.iter().map(|r| r.goodput_bytes_per_second));
            rtt.extend(results.iter().map(|r| r.rtt_seconds));
            busy_share = busy_share.max(busy);
        }
        let level = Level {
            jobs: level_jobs,
            noise_passed: session_band.admits(&session),
            fidelity_passed: goodput_band.admits(&goodput) && rtt_band.admits(&rtt),
            session_seconds: session,
            goodput,
            rtt,
            busy_share,
        };
        let passed = level.noise_passed && level.fidelity_passed;
        if !passed {
            let idle_cpus = environment.logical_cpus as f64 * (1.0 - level.busy_share);
            verdict = match (level.noise_passed, level.fidelity_passed) {
                (false, true) => format!(
                    "at {level_jobs} jobs the median session took {:.4} s, more than the solo spread ({:.4} s) from the solo median ({:.4} s): \
                     sessions slow each other's computation",
                    median(&level.session_seconds),
                    session_band.spread,
                    session_band.median
                ),
                (_, false) if idle_cpus >= 2.0 => format!(
                    "at {level_jobs} jobs the emulated link moved outside its solo spread while {idle_cpus:.0} processors sat idle: \
                     the kernel's shaping is the bottleneck, so more jobs on this host cannot help; \
                     spread a large run across hosts with --shard k/n"
                ),
                _ => format!(
                    "at {level_jobs} jobs the emulated link moved outside its solo spread \
                     (median goodput {:.2} Mbit/s against {:.2} ± {:.2} alone; median round trip {:.3} ms against {:.3} ± {:.3} alone)",
                    median(&level.goodput) * 8e-6,
                    goodput_band.median * 8e-6,
                    goodput_band.spread * 8e-6,
                    median(&level.rtt) * 1e3,
                    rtt_band.median * 1e3,
                    rtt_band.spread * 1e3
                ),
            };
            levels.push(level);
            break;
        }
        jobs = level_jobs;
        levels.push(level);
        level_jobs *= 2;
    }
    Ok(Calibration {
        fingerprint: environment.fingerprint(),
        noise_point: *noise_point,
        fidelity_point: *fidelity_point,
        solo,
        levels,
        jobs,
        max_jobs: setup.max_jobs,
        verdict,
        preparation,
        costs,
        memory,
    })
}

#[cfg(test)]
mod tests;
