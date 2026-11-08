//! Scheduling a run's sessions: many preparations at once, and the timed
//! sessions apart from them, in the run's order.
//!
//! A session has two parts ([`crate::predict`]): its *preparation*, which
//! starts its processes, builds its pair, and moves side B's replica, and
//! its *timed session*, the gossip its row measures. Preparation is most
//! of a run's work and measures nothing, so the run prepares many sessions
//! at once. Timed sessions are what the run measures, and calibration
//! judges them only beside each other ([`crate::calibrate`]): it times as
//! many as the job count at once, with nothing else running. So the run
//! never times a session while any preparation runs.
//!
//! # The schedule
//!
//! The run alternates between two phases:
//!
//! 1. **Preparing.** The run chooses a *window*: the next sessions in its
//!    order that have not been timed, as many as fit the limits
//!    ([`Limits`]). It prepares every session of the window, up to
//!    [`Limits::prepare_jobs`] at once, longest first, and waits for the
//!    last to finish. A prepared session waits with its replicas in its
//!    processes, idle, until its turn.
//! 2. **Timing.** The run times the window's sessions in its order, up to
//!    [`Limits::timed_jobs`] at once, and waits for the last to finish.
//!    Each session's processes exit as its timed session ends, which frees
//!    their memory for the next window.
//!
//! The order the sessions are timed in is the order the scheduler is
//! given: the run shuffles it so that drift spreads across points
//! ([`crate::run`]), and preparation, which measures nothing, may run in
//! any order within a window.
//!
//! # Choosing a window
//!
//! A window holds the sessions of an unbroken stretch of the order from
//! the next session to time, so its sessions are timed back to back, and
//! the stretch is bounded two ways:
//!
//! - **Memory.** A prepared session holds its replicas until it is timed,
//!   and a large pair holds gigabytes, so the predicted peak bytes of a
//!   window's sessions ([`crate::predict::MemoryModel`]) stay within
//!   [`Limits::memory_bytes`]. A session whose prediction alone exceeds
//!   the bound forms a window of its own: it must run, and nothing waits
//!   beside it.
//! - **Count.** Each prepared session holds two processes and their file
//!   descriptors, so a window holds at most [`Limits::window`] sessions.
//!
//! Within those bounds the window's length is a choice. The preparing
//! phase lasts until its longest-running worker finishes, so its cost is
//! not the window's work divided among the workers but the *makespan* of
//! scheduling it longest first (Graham's LPT rule): a window of 46 equal
//! jobs on 32 workers takes two rounds for less than one and a half
//! rounds of work. The scheduler therefore takes, among the admissible
//! lengths, the one whose window keeps the workers busiest: the largest
//! ratio of predicted work to workers times predicted makespan, the
//! longest such window on a tie. What idle workers still cost shows in a
//! run's prediction ([`Simulation`]), which sets its preparing time beside
//! the time it would take with every worker busy.
//!
//! # Why phases rather than overlap
//!
//! Preparing the next window while the current one is timed would hide
//! preparation entirely, but a preparation computes and allocates hard,
//! more than a second timed session does, and calibration has not
//! measured that a timed session beside preparation measures what it
//! measures alone. So the phases never overlap, and the run records each
//! session's preparing and timed intervals so that `check` can confirm
//! they did not ([`crate::check`]).
//!
//! The [`Scheduler`] is a state machine that decides only what to start,
//! so the run drives it with processes and threads, [`simulate`] drives
//! it with predicted durations to estimate a run's wall time, and the
//! tests drive it with every interleaving of completions.

use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

/// One session as the scheduler sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Job {
    /// The predicted wall seconds of its preparation.
    pub prepare_seconds: f64,
    /// The predicted peak bytes it holds from the start of its preparation
    /// to the end of its timed session.
    pub bytes: u64,
}

/// The bounds a run schedules within.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Preparations that run at once.
    pub prepare_jobs: usize,
    /// Timed sessions that run at once: the calibrated job count.
    pub timed_jobs: usize,
    /// The most predicted bytes the sessions of one window hold together.
    pub memory_bytes: u64,
    /// The most sessions one window holds.
    pub window: usize,
}

/// What the scheduler starts next.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Step {
    /// Prepare the session at this position in the order.
    Prepare(usize),
    /// Time the session at this position, which is prepared.
    Time(usize),
}

/// The phase a run is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Preparing the current window.
    Preparing,
    /// Timing the current window.
    Timing,
}

/// The schedule of one part of a run, as [this module](self) describes it.
///
/// The caller asks for [`steps`](Self::steps), starts each one, and
/// reports each as it finishes ([`prepared`](Self::prepared),
/// [`timed`](Self::timed)), until [`finished`](Self::finished). A
/// preparation that fails is still reported as prepared: its session's
/// turn then records the failure.
#[derive(Clone, Debug)]
pub struct Scheduler {
    /// Every session, in the order they are timed.
    jobs: Vec<Job>,
    /// The bounds.
    limits: Limits,
    /// The next session to time; every earlier one has started its timed
    /// session.
    head: usize,
    /// The end of the current window, exclusive: the window is
    /// `head..end` while timing, and its preparations' sessions while
    /// preparing.
    end: usize,
    /// The window's sessions not yet started preparing, ordered so that
    /// the longest pops first.
    unprepared: Vec<usize>,
    /// Preparations running.
    preparing: usize,
    /// Timed sessions running.
    timing: usize,
    /// The current phase.
    phase: Phase,
    /// Windows opened so far.
    windows: usize,
}

impl Scheduler {
    /// The schedule of `jobs`, timed in their order, within `limits`.
    ///
    /// Limits of zero preparations, timed sessions, or window count as one,
    /// since no schedule can run within them.
    pub fn new(jobs: Vec<Job>, limits: Limits) -> Self {
        let limits = Limits {
            prepare_jobs: limits.prepare_jobs.max(1),
            timed_jobs: limits.timed_jobs.max(1),
            window: limits.window.max(1),
            ..limits
        };
        let mut scheduler = Self {
            jobs,
            limits,
            head: 0,
            end: 0,
            unprepared: Vec::new(),
            preparing: 0,
            timing: 0,
            phase: Phase::Timing,
            windows: 0,
        };
        scheduler.open_window();
        scheduler
    }

    /// Every step that may start now, in the order to start them; the
    /// scheduler counts each as running until it is reported finished.
    pub fn steps(&mut self) -> Vec<Step> {
        let mut steps = Vec::new();
        loop {
            match self.phase {
                Phase::Preparing => {
                    while self.preparing < self.limits.prepare_jobs
                        && let Some(index) = self.unprepared.pop()
                    {
                        self.preparing += 1;
                        steps.push(Step::Prepare(index));
                    }
                    if self.preparing > 0 || !self.unprepared.is_empty() {
                        return steps;
                    }
                    self.phase = Phase::Timing;
                }
                Phase::Timing => {
                    while self.timing < self.limits.timed_jobs && self.head < self.end {
                        self.timing += 1;
                        steps.push(Step::Time(self.head));
                        self.head += 1;
                    }
                    if self.timing > 0 || self.head < self.end || self.end == self.jobs.len() {
                        return steps;
                    }
                    self.open_window();
                }
            }
        }
    }

    /// Records that the preparation of the session at `index` finished.
    ///
    /// # Panics
    ///
    /// If no preparation is running: a defect in the caller.
    pub fn prepared(&mut self, index: usize) {
        debug_assert!(index < self.end, "only a window's sessions prepare");
        self.preparing = self
            .preparing
            .checked_sub(1)
            .expect("a preparation finishes only after it starts");
    }

    /// Records that the timed session of the session at `index` finished.
    ///
    /// # Panics
    ///
    /// If no timed session is running: a defect in the caller.
    pub fn timed(&mut self, index: usize) {
        debug_assert!(index < self.head, "only a started session finishes");
        self.timing = self
            .timing
            .checked_sub(1)
            .expect("a timed session finishes only after it starts");
    }

    /// Whether every session has been prepared and timed.
    pub fn finished(&self) -> bool {
        self.head == self.jobs.len() && self.timing == 0 && self.preparing == 0
    }

    /// The windows opened so far.
    pub fn windows(&self) -> usize {
        self.windows
    }

    /// Opens the next window from the head and enters the preparing
    /// phase, as [the module docs](self) describe.
    fn open_window(&mut self) {
        let rest = &self.jobs[self.head..];
        let mut held = 0u64;
        let mut admissible = 0;
        for job in rest.iter().take(self.limits.window) {
            let next = held.saturating_add(job.bytes);
            if admissible > 0 && next > self.limits.memory_bytes {
                break;
            }
            held = next;
            admissible += 1;
        }
        let mut best = (0.0, admissible);
        let mut seconds: Vec<f64> = Vec::with_capacity(admissible);
        for len in 1..=admissible {
            seconds.clear();
            seconds.extend(rest[..len].iter().map(|job| job.prepare_seconds));
            let utilization = utilization(&mut seconds, self.limits.prepare_jobs);
            if utilization >= best.0 {
                best = (utilization, len);
            }
        }
        self.end = self.head + best.1;
        let jobs = &self.jobs;
        let mut window: Vec<usize> = (self.head..self.end).collect();
        // Shortest first, and among equals the latest first, so that
        // popping takes the longest, earliest session.
        window.sort_by(|&x, &y| {
            jobs[x]
                .prepare_seconds
                .total_cmp(&jobs[y].prepare_seconds)
                .then(y.cmp(&x))
        });
        self.unprepared = window;
        self.phase = Phase::Preparing;
        if self.end > self.head {
            self.windows += 1;
        }
    }
}

/// A time that orders totally, for the heaps below.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Seconds(f64);

impl Eq for Seconds {}

impl PartialOrd for Seconds {
    /// As [`Ord`].
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Seconds {
    /// IEEE 754's total order.
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// The makespan of `seconds` on `workers` under Graham's longest-first
/// rule: each job, longest first, goes to the worker that frees first.
/// Sorts `seconds` longest first.
pub fn makespan(seconds: &mut [f64], workers: usize) -> f64 {
    seconds.sort_by(|x, y| y.total_cmp(x));
    let mut loads: BinaryHeap<Reverse<Seconds>> = (0..workers.max(1).min(seconds.len()))
        .map(|_| Reverse(Seconds(0.0)))
        .collect();
    for &job in seconds.iter() {
        let Reverse(Seconds(load)) = loads.pop().expect("there is at least one worker");
        loads.push(Reverse(Seconds(load + job)));
    }
    loads
        .into_iter()
        .map(|Reverse(Seconds(load))| load)
        .fold(0.0, f64::max)
}

/// The share of `workers`' time the jobs `seconds` keep busy over their
/// [`makespan`]: one when nothing idles, and one for jobs that take no
/// time at all.
fn utilization(seconds: &mut [f64], workers: usize) -> f64 {
    let work: f64 = seconds.iter().sum();
    let span = makespan(seconds, workers);
    if span > 0.0 {
        work / (workers.max(1) as f64 * span)
    } else {
        1.0
    }
}

/// What [`simulate`] predicts of a run.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Simulation {
    /// Wall seconds from the first preparation to the last timed session's
    /// end.
    pub wall_seconds: f64,
    /// Wall seconds during which some preparation runs.
    pub preparing_seconds: f64,
    /// Wall seconds during which some timed session runs.
    pub timing_seconds: f64,
    /// The preparing time if every worker stayed busy: the predicted
    /// preparation seconds divided among the workers.
    pub ideal_preparing_seconds: f64,
    /// The windows the run prepares.
    pub windows: usize,
}

/// Plays the schedule of `jobs` within `limits` through on a simulated
/// clock, each preparation taking its [`Job::prepare_seconds`] and each
/// timed session its entry in `timed_seconds`.
///
/// # Panics
///
/// If `timed_seconds` and `jobs` differ in length: a defect in the caller.
pub fn simulate(jobs: Vec<Job>, timed_seconds: &[f64], limits: Limits) -> Simulation {
    assert_eq!(
        jobs.len(),
        timed_seconds.len(),
        "every job has a timed duration"
    );
    let ideal =
        jobs.iter().map(|job| job.prepare_seconds).sum::<f64>() / limits.prepare_jobs.max(1) as f64;
    let prepare: Vec<f64> = jobs.iter().map(|job| job.prepare_seconds).collect();
    let mut scheduler = Scheduler::new(jobs, limits);
    let mut running: BinaryHeap<Reverse<(Seconds, Step)>> = BinaryHeap::new();
    let mut simulation = Simulation {
        ideal_preparing_seconds: ideal,
        ..Simulation::default()
    };
    let mut clock = 0.0;
    let mut preparing = 0usize;
    loop {
        for step in scheduler.steps() {
            let seconds = match step {
                Step::Prepare(index) => {
                    preparing += 1;
                    prepare[index]
                }
                Step::Time(index) => timed_seconds[index],
            };
            running.push(Reverse((Seconds(clock + seconds.max(0.0)), step)));
        }
        let Some(Reverse((Seconds(at), step))) = running.pop() else {
            break;
        };
        let elapsed = at - clock;
        if preparing > 0 {
            simulation.preparing_seconds += elapsed;
        } else {
            simulation.timing_seconds += elapsed;
        }
        clock = at;
        match step {
            Step::Prepare(index) => {
                preparing -= 1;
                scheduler.prepared(index);
            }
            Step::Time(index) => scheduler.timed(index),
        }
    }
    debug_assert!(scheduler.finished(), "the simulated schedule ran dry");
    simulation.wall_seconds = clock;
    simulation.windows = scheduler.windows();
    simulation
}

#[cfg(test)]
mod tests;
