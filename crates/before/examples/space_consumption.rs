//! Empirical ITC space-consumption simulation, reproducing the experiment in
//! §6 ("Exercising ITCs") and Figure 1 of the ITC 2008 paper, but measuring
//! *this crate's* packed encoding rather than the paper's Appendix A encoding.
//!
//! # What the paper measured
//!
//! Figure 1 plots the mean encoded size (in bytes) of an ITC stamp against the
//! number of operations performed, for entity populations of 4, 8, 16, 32, 64,
//! and 128, averaged across 100 runs, in two regimes:
//!
//! * **Data causality, dynamic setting** — ids churn constantly. Each iteration
//!   performs a `fork`, an `event`, and a `join`, each on randomly chosen
//!   replicas (paper §6). `fork` grows the population by one, `join` shrinks it
//!   by one, so the population is held constant while the id tree is reshaped
//!   continuously. Run to 100,000 iterations.
//!
//! * **Process causality, static setting** — a fixed set of processes exchange
//!   messages (via `peek` + `join`) and record internal `event`s. Because
//!   messages are anonymous stamps `(0, e)` (paper §3, `peek`), ids never
//!   change; only the event component grows. Run to 25,000 iterations.
//!
//! # Mapping to this crate's API
//!
//! | Paper operation        | This crate                                   |
//! |------------------------|----------------------------------------------|
//! | `fork`                 | [`Clock::fork`]                              |
//! | `event`                | [`Clock::tick`]                              |
//! | `join`                 | [`Clock::join`]                              |
//! | `peek` then `join`     | snapshot [`Clock::version`], then `clock |= v` |
//! | binary encoding        | [`Clock::encode`] (see note below)           |
//!
//! # Encoding note
//!
//! The paper sizes stamps with the Appendix A bit encoding. This crate uses its
//! own canonical packed encoding ([`Clock::encode`]), which is slightly more
//! compact. We deliberately measure *ours*: the absolute byte counts here run a
//! little below the paper's, but the curve shapes — rapid early growth followed
//! by stabilization with only a faint logarithmic creep — reproduce the paper's
//! qualitative result, which is the point of the experiment.
//!
//! # Output
//!
//! A CSV is written to stdout; a progress bar (weighted by total iterations) is
//! drawn on stderr. Columns:
//!
//! ```text
//! scenario,entities,iteration,mean_bits,std_bits,mean_bytes,std_bytes,runs
//! ```
//!
//! `mean_bits`/`mean_bytes` are the grand mean, across runs, of the per-run mean
//! stamp size at that iteration checkpoint; the `std_` columns are the standard
//! deviation of the per-run means (the spread the paper averages away). Sampling
//! is log-spaced so a handful of checkpoints traces the whole curve.
//!
//! Bits are the exact pre-pad encoded length ([`Clock::encoded_bits`]); bytes are
//! `encode().len()` = `⌈bits/8⌉` per stamp. The bit column is the smooth quantity
//! to fit — averaging per-stamp byte ceilings biases small stamps upward and adds
//! quantization noise; the byte column is what a real deployment stores.
//!
//! # Running
//!
//! ```sh
//! cargo run --release --example space_consumption > space.csv
//! ```
//!
//! With the paper's parameters (100 runs) this is a long simulation — the
//! 128-replica dynamic case builds multi-kilobyte stamps and reshapes them
//! 100,000 times per run. Runs are parallelized across cores with rayon. For a
//! quick look, reduce the work:
//!
//! ```sh
//! cargo run --release --example space_consumption -- --runs 10
//! cargo run --release --example space_consumption -- --runs 4 --data-iters 5000 --process-iters 2000
//! ```
//!
//! Flags: `--runs N`, `--data-iters N`, `--process-iters N`,
//! `--entities 4,8,16,...`.

use std::io::{self, Write};

use before::Clock;
use indicatif::{ProgressBar, ProgressStyle};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;

/// Paper defaults (§6, Figure 1).
const DEFAULT_RUNS: u64 = 100;
const DEFAULT_DATA_ITERS: u64 = 100_000;
const DEFAULT_PROCESS_ITERS: u64 = 25_000;
const DEFAULT_ENTITIES: &[usize] = &[4, 8, 16, 32, 64, 128];

/// Log-spaced checkpoints per decade. Eight points per decade is dense enough to
/// trace the curve smoothly on a log x-axis without sampling every iteration.
const POINTS_PER_DECADE: f64 = 8.0;

/// The two regimes of Figure 1.
#[derive(Clone, Copy)]
enum Scenario {
    /// Data causality, dynamic setting: fork + event + join per iteration.
    Data,
    /// Process causality, static setting: internal event + anonymous message.
    Process,
}

impl Scenario {
    fn label(self) -> &'static str {
        match self {
            Scenario::Data => "data",
            Scenario::Process => "process",
        }
    }

    /// A distinct tag so per-run seeds don't collide across scenarios.
    fn tag(self) -> u64 {
        match self {
            Scenario::Data => 0,
            Scenario::Process => 1,
        }
    }
}

/// Simulation parameters, after applying any CLI overrides.
struct Config {
    runs: u64,
    data_iters: u64,
    process_iters: u64,
    entities: Vec<usize>,
}

impl Config {
    fn iters(&self, scenario: Scenario) -> u64 {
        match scenario {
            Scenario::Data => self.data_iters,
            Scenario::Process => self.process_iters,
        }
    }
}

fn main() {
    let config = parse_args();

    // Total iterations across every (scenario, entity-count, run) drives a
    // work-weighted progress bar: a 128-replica run counts far more than a
    // 4-replica one, so the ETA reflects real remaining work rather than a flat
    // count of runs.
    let total_iters: u64 = [Scenario::Data, Scenario::Process]
        .iter()
        .map(|&s| config.iters(s) * config.runs * config.entities.len() as u64)
        .sum();

    let pb = ProgressBar::new(total_iters);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner} {msg:<14} [{bar:30}] {percent:>3}%  {pos}/{len} iters  ETA {eta}",
        )
        .expect("static template is valid")
        .progress_chars("=>-"),
    );

    let stdout = io::stdout();
    let mut out = stdout.lock();
    writeln!(
        out,
        "scenario,entities,iteration,mean_bits,std_bits,mean_bytes,std_bytes,runs"
    )
    .expect("writing CSV header to stdout");

    // Heaviest populations first (descending), so the progress bar's early
    // throughput reflects the slow large-stamp work: the ETA then *over*estimates
    // the lighter work still to come rather than under estimating it.
    let mut entities = config.entities.clone();
    entities.sort_unstable_by(|a, b| b.cmp(a));

    for scenario in [Scenario::Data, Scenario::Process] {
        let max_iters = config.iters(scenario);
        let checkpoints = checkpoints(max_iters);

        for &n in &entities {
            pb.set_message(format!("{} N={}", scenario.label(), n));

            // Each run is independent; parallelize across them. `collect`
            // preserves run order, so results stay deterministic regardless of
            // how rayon schedules the threads.
            let per_run: Vec<Vec<(f64, f64)>> = (0..config.runs)
                .into_par_iter()
                .map(|run| {
                    let seed = seed_for(scenario, n, run);
                    simulate(scenario, n, max_iters, &checkpoints, seed, &pb)
                })
                .collect();

            // Aggregate across runs at each checkpoint: the figure's y-value is
            // the mean of the per-run mean stamp sizes, reported in both bits
            // (exact) and bytes (rounded, as stored).
            for (ci, &iteration) in checkpoints.iter().enumerate() {
                let bits: Vec<f64> = per_run.iter().map(|run| run[ci].0).collect();
                let bytes: Vec<f64> = per_run.iter().map(|run| run[ci].1).collect();
                let (mean_bits, std_bits) = mean_std(&bits);
                let (mean_bytes, std_bytes) = mean_std(&bytes);
                writeln!(
                    out,
                    "{},{},{},{:.4},{:.4},{:.4},{:.4},{}",
                    scenario.label(),
                    n,
                    iteration,
                    mean_bits,
                    std_bits,
                    mean_bytes,
                    std_bytes,
                    config.runs,
                )
                .expect("writing CSV row to stdout");
            }
        }
    }

    pb.finish_with_message("done");
}

/// Run one simulation: build a population of `n` balanced stamps, then exercise
/// it for `max_iters` iterations, recording the mean stamp size (as `(bits, bytes)`,
/// under this crate's encoding) at each checkpoint. Advances `pb` by the number of
/// iterations completed between checkpoints.
fn simulate(
    scenario: Scenario,
    n: usize,
    max_iters: u64,
    checkpoints: &[u64],
    seed: u64,
    pb: &ProgressBar,
) -> Vec<(f64, f64)> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut clocks = build_population(n);

    let mut sizes = Vec::with_capacity(checkpoints.len());
    let mut next_cp = 0usize;
    let mut last_reported = 0u64;

    for iteration in 1..=max_iters {
        match scenario {
            Scenario::Data => step_data(&mut clocks, &mut rng),
            Scenario::Process => step_process(&mut clocks, n, &mut rng),
        }

        if next_cp < checkpoints.len() && iteration == checkpoints[next_cp] {
            sizes.push(mean_stamp_sizes(&clocks));
            next_cp += 1;
            // Report progress in checkpoint-sized chunks: ~8 atomic increments
            // per decade per run, negligible against the work itself.
            pb.inc(iteration - last_reported);
            last_reported = iteration;
        }
    }

    // Account for any iterations after the final checkpoint (none, since the
    // last checkpoint is always `max_iters`, but keeps the accounting exact).
    pb.inc(max_iters - last_reported);
    sizes
}

/// Data causality, dynamic setting (paper §6): fork a random replica (+1), record
/// an event on a random replica, then join a random pair (−1). Population is
/// constant across the iteration; ids churn.
fn step_data(clocks: &mut Vec<Clock>, rng: &mut StdRng) {
    // fork: clone a random replica's causal past into a fresh id.
    let parent = rng.gen_range(0..clocks.len());
    let child = clocks[parent].fork();
    clocks.push(child);

    // event: advance a random replica.
    let who = rng.gen_range(0..clocks.len());
    clocks[who].tick();

    // join: merge a random donor into a different random target, retiring the
    // donor's id. Forked clocks are always disjoint, so the join never fails.
    let donor = clocks.swap_remove(rng.gen_range(0..clocks.len()));
    let target = rng.gen_range(0..clocks.len());
    clocks[target]
        .join(donor)
        .expect("clocks forked from one seed are disjoint");
}

/// Process causality, static setting (paper §6): record one internal event, then
/// exchange one anonymous message. `peek` yields an anonymous stamp `(0, e)`
/// (the snapshot `Version`); the receiver joins it, leaving its own id intact.
fn step_process(clocks: &mut [Clock], n: usize, rng: &mut StdRng) {
    // internal event on a random process.
    let who = rng.gen_range(0..n);
    clocks[who].tick();

    // message: a random sender's peek is joined by a different random receiver.
    let sender = rng.gen_range(0..n);
    let mut receiver = rng.gen_range(0..n);
    if receiver == sender {
        receiver = (receiver + 1) % n;
    }
    let peeked = clocks[sender].version().clone();
    clocks[receiver] |= peeked;
}

/// Build `n` stamps from a single seed by balanced forking (paper §3: "starting
/// from an initial seed stamp and forking several times"). For power-of-two `n`
/// this doubles the population each round, yielding a perfectly balanced id tree;
/// other counts get a slightly uneven tail, which is harmless.
fn build_population(n: usize) -> Vec<Clock> {
    let mut clocks = vec![Clock::seed()];
    while clocks.len() < n {
        let round = clocks.len();
        for i in 0..round {
            if clocks.len() >= n {
                break;
            }
            let child = clocks[i].fork();
            clocks.push(child);
        }
    }
    clocks
}

/// Mean encoded size over all live stamps, as `(bits, bytes)` — this crate's
/// packed encoding, not the paper's Appendix A encoding. Bits are the exact
/// pre-pad length ([`Clock::encoded_bits`]); bytes are `encode().len()`, i.e.
/// `⌈bits/8⌉` per stamp (the per-stamp ceiling is what biases the byte mean).
fn mean_stamp_sizes(clocks: &[Clock]) -> (f64, f64) {
    let n = clocks.len() as f64;
    let bits: usize = clocks.iter().map(Clock::encoded_bits).sum();
    let bytes: usize = clocks.iter().map(|c| c.encode().len()).sum();
    (bits as f64 / n, bytes as f64 / n)
}

/// Mean and (population) standard deviation of a sample.
fn mean_std(xs: &[f64]) -> (f64, f64) {
    let n = xs.len() as f64;
    let mean = xs.iter().sum::<f64>() / n;
    let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    (mean, var.sqrt())
}

/// Log-spaced checkpoint iterations in `1..=max`, always including `max` so the
/// final stabilized size is recorded.
fn checkpoints(max: u64) -> Vec<u64> {
    let mut points = Vec::new();
    let mut last = 0u64;
    let k_max = ((max as f64).log10() * POINTS_PER_DECADE).ceil() as i64;
    for k in 0..=k_max {
        let x = (10f64.powf(k as f64 / POINTS_PER_DECADE).round() as u64).min(max);
        if x != last {
            points.push(x);
            last = x;
        }
    }
    if points.last() != Some(&max) {
        points.push(max);
    }
    points
}

/// A reproducible per-run seed. Mixing the scenario, population, and run index
/// keeps every simulation independent yet deterministic across invocations.
fn seed_for(scenario: Scenario, n: usize, run: u64) -> u64 {
    const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;
    GOLDEN ^ (scenario.tag() << 56) ^ ((n as u64) << 32) ^ run
}

/// Minimal flag parser for the paper-default overrides.
fn parse_args() -> Config {
    let mut config = Config {
        runs: DEFAULT_RUNS,
        data_iters: DEFAULT_DATA_ITERS,
        process_iters: DEFAULT_PROCESS_ITERS,
        entities: DEFAULT_ENTITIES.to_vec(),
    };

    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut value = || {
            args.next()
                .unwrap_or_else(|| fail(&format!("{flag} needs a value")))
        };
        match flag.as_str() {
            "--runs" => config.runs = parse_u64(&flag, &value()),
            "--data-iters" => config.data_iters = parse_u64(&flag, &value()),
            "--process-iters" => config.process_iters = parse_u64(&flag, &value()),
            "--entities" => {
                config.entities = value()
                    .split(',')
                    .map(|s| {
                        s.trim().parse().unwrap_or_else(|_| {
                            fail("--entities wants a comma-separated list of integers, e.g. 4,8,16")
                        })
                    })
                    .collect();
            }
            "-h" | "--help" => {
                eprintln!(
                    "usage: space_consumption [--runs N] [--data-iters N] [--process-iters N] [--entities 4,8,16,...]"
                );
                std::process::exit(0);
            }
            other => fail(&format!("unknown flag: {other}")),
        }
    }

    if config.entities.is_empty() || config.entities.iter().any(|&n| n < 2) {
        fail("--entities must be non-empty and every count must be >= 2 (a join needs two distinct stamps)");
    }
    config
}

fn parse_u64(flag: &str, value: &str) -> u64 {
    value.parse().unwrap_or_else(|_| {
        fail(&format!(
            "{flag} expects a non-negative integer, got {value:?}"
        ))
    })
}

fn fail(message: &str) -> ! {
    eprintln!("error: {message}");
    std::process::exit(2);
}
