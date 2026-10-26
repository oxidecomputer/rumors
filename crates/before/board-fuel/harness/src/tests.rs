//! Checks the fuel board's work queue: its order across the ladder's
//! samplings, and the pool that runs it.

use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};

use before::testing::meter::board::{Sampling, DEFAULT_SCALE, LADDER_TOP_SCALE, SMALL_INPUT_SCALE};

use super::{queue, run_queue};

/// For the ladder's samplings, given in sweep order, [`queue`] lists every
/// shard of the top scale, then of the past-power sampling, then of the
/// base scale, then of the small inputs, each sampling's shards in shard
/// order.
#[test]
fn queue_orders_the_ladder_top_past_power_base_small() {
    let samplings = [
        Sampling::Scaled(SMALL_INPUT_SCALE),
        Sampling::PastPowerOfTwo,
        Sampling::Scaled(DEFAULT_SCALE),
        Sampling::Scaled(LADDER_TOP_SCALE),
    ];
    let shards = 3;
    let expected: Vec<(usize, usize)> = [3, 1, 2, 0]
        .into_iter()
        .flat_map(|sampling| (0..shards).map(move |index| (sampling, index)))
        .collect();
    assert_eq!(queue(&samplings, shards), expected);
}

/// [`run_queue`] returns every task's result in queue order when no task
/// fails, and otherwise the error of the first failing task in queue order.
///
/// The claim is checked for every queue of up to six tasks, every set of
/// failing tasks, and one to eight workers; no task may run twice, and
/// every task must run when none fails.
///
/// The queue lists the tasks in reverse, so results in task order, or the
/// first failure in task order, would not pass. Each run's interleaving is
/// whatever the scheduler produces; the claim holds under every one.
#[test]
fn run_queue_returns_results_in_queue_order_or_the_first_failure() {
    for len in 0..=6usize {
        let queue: Vec<usize> = (0..len).rev().collect();
        for failing in 0u32..(1 << len) {
            let fails = |task: usize| failing >> task & 1 == 1;
            for threads in 1..=8 {
                let runs: Vec<AtomicUsize> = (0..len).map(|_| AtomicUsize::new(0)).collect();
                let result = run_queue(
                    &queue,
                    threads,
                    || (),
                    |(), &task| {
                        runs[task].fetch_add(1, Ordering::Relaxed);
                        if fails(task) {
                            Err(io::Error::other(format!("task {task}")))
                        } else {
                            Ok(10 * task)
                        }
                    },
                );
                let case = format!("{len} tasks, failing set {failing:#b}, {threads} threads");
                match queue.iter().find(|&&task| fails(task)) {
                    None => {
                        let expected: Vec<usize> = queue.iter().map(|task| 10 * task).collect();
                        assert_eq!(result.expect(&case), expected, "{case}");
                        assert!(
                            runs.iter().all(|runs| runs.load(Ordering::Relaxed) == 1),
                            "{case}: every task runs once"
                        );
                    }
                    Some(first) => {
                        let error = result.expect_err(&case);
                        assert_eq!(error.to_string(), format!("task {first}"), "{case}");
                        assert!(
                            runs.iter().all(|runs| runs.load(Ordering::Relaxed) <= 1),
                            "{case}: no task runs twice"
                        );
                    }
                }
            }
        }
    }
}
