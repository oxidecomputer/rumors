use proptest::prelude::*;

use super::*;

/// Where one session is in its life, as the test driver tracks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// Not yet started.
    Unprepared,
    /// Its preparation runs.
    Preparing,
    /// Prepared, waiting for its turn.
    Prepared,
    /// Its timed session runs.
    Timing,
    /// Timed.
    Done,
}

/// One moment of a driven schedule: what had just started or finished, and
/// the state of every session after it.
#[derive(Clone, Debug)]
struct Moment {
    /// The step that started, or finished when `finished` is set.
    step: Step,
    /// Whether the step finished rather than started.
    finished: bool,
    /// Every session's state just after.
    states: Vec<State>,
}

/// A schedule driven to its end, completing running steps in the order
/// `choices` picks, with every moment recorded.
struct Driven {
    /// Every start and finish, in order.
    moments: Vec<Moment>,
    /// Whether the scheduler reported itself finished at the end.
    finished: bool,
}

/// Drives a scheduler of `jobs` within `limits`, finishing at each turn
/// the running step `choices` picks, so that every interleaving of
/// completions is reachable.
///
/// A step the scheduler hands out for a session in the wrong state panics
/// the driver, which fails the case: preparing a session twice, or timing
/// one that is not prepared.
fn drive(jobs: &[Job], limits: Limits, choices: &[prop::sample::Index]) -> Driven {
    let mut scheduler = Scheduler::new(jobs.to_vec(), limits);
    let mut states = vec![State::Unprepared; jobs.len()];
    let mut running: Vec<Step> = Vec::new();
    let mut moments = Vec::new();
    let mut choices = choices.iter().cycle();
    loop {
        for step in scheduler.steps() {
            match step {
                Step::Prepare(index) => {
                    assert_eq!(states[index], State::Unprepared, "{step:?} prepares twice");
                    states[index] = State::Preparing;
                }
                Step::Time(index) => {
                    assert_eq!(states[index], State::Prepared, "{step:?} is not prepared");
                    states[index] = State::Timing;
                }
            }
            running.push(step);
            moments.push(Moment {
                step,
                finished: false,
                states: states.clone(),
            });
        }
        if running.is_empty() {
            break;
        }
        let pick = choices
            .next()
            .map_or(0, |choice| choice.index(running.len()));
        let step = running.swap_remove(pick);
        match step {
            Step::Prepare(index) => {
                states[index] = State::Prepared;
                scheduler.prepared(index);
            }
            Step::Time(index) => {
                states[index] = State::Done;
                scheduler.timed(index);
            }
        }
        moments.push(Moment {
            step,
            finished: true,
            states: states.clone(),
        });
    }
    Driven {
        moments,
        finished: scheduler.finished(),
    }
}

/// Sessions with predicted preparation seconds and bytes, some of them
/// larger than any memory bound the limits draw.
fn jobs() -> impl Strategy<Value = Vec<Job>> {
    prop::collection::vec(
        (0.0..10.0f64, 0u64..120).prop_map(|(prepare_seconds, bytes)| Job {
            prepare_seconds,
            bytes,
        }),
        0..48,
    )
}

/// Limits small enough that windows close on every bound.
fn limits() -> impl Strategy<Value = Limits> {
    (1usize..5, 1usize..4, 0u64..300, 1usize..12).prop_map(
        |(prepare_jobs, timed_jobs, memory_bytes, window)| Limits {
            prepare_jobs,
            timed_jobs,
            memory_bytes,
            window,
        },
    )
}

/// The completion order: which running step finishes at each turn.
fn choices() -> impl Strategy<Value = Vec<prop::sample::Index>> {
    prop::collection::vec(any::<prop::sample::Index>(), 1..64)
}

proptest! {
    /// Under any interleaving of completions, the scheduler times every
    /// session exactly once, in the order it was given.
    ///
    /// It prepares each session exactly once and times it only after, and
    /// reports itself finished exactly when nothing is left: it never
    /// stalls with work remaining.
    #[test]
    fn every_session_is_timed_once_in_order(jobs in jobs(), limits in limits(), choices in choices()) {
        let driven = drive(&jobs, limits, &choices);
        prop_assert!(driven.finished);
        let started = |prepare: bool| -> Vec<usize> {
            driven
                .moments
                .iter()
                .filter(|moment| !moment.finished)
                .filter_map(|moment| match (moment.step, prepare) {
                    (Step::Prepare(index), true) | (Step::Time(index), false) => Some(index),
                    _ => None,
                })
                .collect()
        };
        let timed = started(false);
        prop_assert_eq!(timed, (0..jobs.len()).collect::<Vec<_>>());
        let mut prepared = started(true);
        prepared.sort_unstable();
        prop_assert_eq!(prepared, (0..jobs.len()).collect::<Vec<_>>());
        let last = driven.moments.last().map_or_else(Vec::new, |moment| moment.states.clone());
        prop_assert!(last.iter().all(|state| *state == State::Done));
    }

    /// No preparation ever runs while a timed session runs, and at most
    /// `prepare_jobs` preparations and `timed_jobs` timed sessions run at
    /// once: the strict phases calibration's conditions require.
    #[test]
    fn preparation_never_runs_beside_a_timed_session(jobs in jobs(), limits in limits(), choices in choices()) {
        let driven = drive(&jobs, limits, &choices);
        for moment in &driven.moments {
            let count = |state| moment.states.iter().filter(|s| **s == state).count();
            let (preparing, timing) = (count(State::Preparing), count(State::Timing));
            prop_assert!(preparing == 0 || timing == 0, "{preparing} preparing beside {timing} timed");
            prop_assert!(preparing <= limits.prepare_jobs);
            prop_assert!(timing <= limits.timed_jobs);
        }
    }

    /// The sessions held at any moment stay within the window's count and
    /// the memory bound.
    ///
    /// A session is held from the start of its preparation to the end of
    /// its timed session. Only a single session that alone exceeds the
    /// memory bound may exceed it, and it then waits alone.
    #[test]
    fn held_sessions_stay_within_the_bounds(jobs in jobs(), limits in limits(), choices in choices()) {
        let driven = drive(&jobs, limits, &choices);
        for moment in &driven.moments {
            let held: Vec<usize> = moment
                .states
                .iter()
                .enumerate()
                .filter(|(_, state)| matches!(state, State::Preparing | State::Prepared | State::Timing))
                .map(|(index, _)| index)
                .collect();
            let bytes: u64 = held.iter().map(|&index| jobs[index].bytes).sum();
            prop_assert!(held.len() <= limits.window);
            prop_assert!(
                bytes <= limits.memory_bytes || held.len() == 1,
                "{} sessions hold {bytes} bytes against a bound of {}",
                held.len(),
                limits.memory_bytes
            );
        }
    }

    /// Graham's longest-first makespan lies within the bounds every list
    /// schedule meets.
    ///
    /// It is at least the longest job and the work divided among the
    /// workers, and at most the divided work plus the longest job.
    #[test]
    fn makespan_lies_within_its_bounds(
        mut seconds in prop::collection::vec(0.0..100.0f64, 1..40),
        workers in 1usize..9,
    ) {
        let longest = seconds.iter().copied().fold(0.0, f64::max);
        let divided = seconds.iter().sum::<f64>() / workers as f64;
        let span = makespan(&mut seconds, workers);
        let slack = 1e-9 * (1.0 + divided + longest);
        prop_assert!(span + slack >= longest.max(divided));
        prop_assert!(span <= divided + longest + slack);
    }

    /// A simulated run accounts for its whole wall time as preparing or
    /// timing, never both, with no idle gap.
    ///
    /// Its preparing time is never less than the work divided among the
    /// workers, and with one timed session at a time its timing time is
    /// exactly the sum of the timed sessions.
    #[test]
    fn simulation_accounts_for_every_second(
        jobs in jobs(),
        limits in limits(),
        timed in prop::collection::vec(0.0..10.0f64, 48),
    ) {
        let timed = &timed[..jobs.len()];
        let one_at_a_time = Limits { timed_jobs: 1, ..limits };
        let simulation = simulate(jobs.clone(), timed, one_at_a_time);
        let slack = 1e-6 * (1.0 + simulation.wall_seconds);
        prop_assert!(
            (simulation.wall_seconds - simulation.preparing_seconds - simulation.timing_seconds).abs() <= slack
        );
        prop_assert!(simulation.preparing_seconds + slack >= simulation.ideal_preparing_seconds);
        prop_assert!((simulation.timing_seconds - timed.iter().sum::<f64>()).abs() <= slack);
    }

    /// With one preparation and one timed session at a time, the simulated
    /// wall time is the sum of every preparation and every timed session:
    /// the serial run, computed a second way.
    #[test]
    fn serial_simulation_is_the_sum(
        jobs in jobs(),
        memory_bytes in 0u64..300,
        window in 1usize..12,
        timed in prop::collection::vec(0.0..10.0f64, 48),
    ) {
        let timed = &timed[..jobs.len()];
        let serial = Limits { prepare_jobs: 1, timed_jobs: 1, memory_bytes, window };
        let simulation = simulate(jobs.clone(), timed, serial);
        let sum = jobs.iter().map(|job| job.prepare_seconds).sum::<f64>() + timed.iter().sum::<f64>();
        prop_assert!((simulation.wall_seconds - sum).abs() <= 1e-6 * (1.0 + sum));
    }
}

/// A window of 46 equal preparations on 32 workers is cut to the 32 that
/// fill one round.
///
/// Longest-first scheduling would finish all 46 in two rounds for less
/// than one and a half rounds of work; the other 14 open the next window.
#[test]
fn window_is_cut_where_workers_stay_busy() {
    let jobs = vec![
        Job {
            prepare_seconds: 1.0,
            bytes: 1,
        };
        46
    ];
    let limits = Limits {
        prepare_jobs: 32,
        timed_jobs: 1,
        memory_bytes: u64::MAX,
        window: 64,
    };
    let mut scheduler = Scheduler::new(jobs, limits);
    let steps = scheduler.steps();
    assert_eq!(steps.len(), 32);
    assert!(
        steps
            .iter()
            .all(|step| matches!(step, Step::Prepare(index) if *index < 32))
    );
}
