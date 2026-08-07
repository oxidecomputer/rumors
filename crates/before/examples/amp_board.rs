//! Runs the amplification board and prints the red-green matrix to stdout.
//!
//! The board itself — the operation × input-family sweep, the meters, the
//! green/red criterion, and the not-applicable coverage list — lives in
//! `before::meter::board`; this binary installs the counting allocator
//! the board's peak-heap column reads (a global allocator is per-binary
//! state the library cannot own), parses the size knob, and orchestrates
//! the process sharding below.
//!
//! Usage: `just amp-board` (release, the profile of record — dev runs are
//! a debugging view whose readings are never pinned), or directly
//! `cargo run -p before --example amp_board --features limb-meter,scan-meter
//! -- [scale]` where the optional `scale` (a positive number, default 1)
//! multiplies every input family's base size; the literal `acceptance`
//! selects the acceptance scale (`board::ACCEPTANCE_SCALE`, `just
//! amp-board-acceptance`). The default sizes keep the whole board at
//! seconds of runtime; acceptance requires all green at both the default
//! and acceptance scales, one run each under the board's determinism
//! tripwire — and the exit code carries that verdict: at those two
//! scales of record, any red cell exits nonzero, so every gate leg that
//! runs a board of record consumes its verdicts. The counter features are
//! `required-features`: a build without them would render limb, scan,
//! and touch unjudged while still printing verdict colors, so cargo
//! refuses it outright.
//!
//! Two further modes consume the same sweep instead of rendering the
//! matrix: `worst-cases` renders the worst-case map (the argmax family
//! per operation × currency) at both scales of record
//! (`board::WORST_MAP_SCALES`, `just worst-cases`), and
//! `worst-cases-check` entry-compares the live fold against the
//! committed ranking pin, exiting nonzero on any drift (`just
//! worst-cases-pin`).
//!
//! # Process sharding
//!
//! The sweep is single-threaded within a process by design — the
//! peak-heap column reads the process-global allocator, so concurrent
//! threads would blend live sets — and parallelizes by process instead:
//! every mode spawns one copy of this binary per slice of the operation
//! × family cell grid (`--shard i/N`, an internal protocol;
//! `board::shard` documents it), each child measuring its slice
//! single-threaded, and the parent merges, judges, and renders. Every
//! judged quantity is a deterministic counter over state a child owns
//! privately, so the shard count is a throughput knob and never an input
//! to a reading: `AMP_BOARD_SHARDS` overrides it (default: available
//! parallelism).

use std::io;
use std::process::{Command, Stdio};

use before::meter::board::{self, HeapMeter};
use peak_alloc::PeakAlloc;

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

/// The default size multiplier when no argument is given.
const DEFAULT_SCALE: f64 = 1.0;

/// The shard-count override. Unset, the runner uses available
/// parallelism.
const SHARDS_ENV: &str = "AMP_BOARD_SHARDS";

/// The peak-heap readers over this binary's global allocator.
fn heap_meter() -> HeapMeter {
    HeapMeter {
        reset_peak: || HEAP.reset_peak_usage(),
        peak: || HEAP.peak_usage(),
        current: || HEAP.current_usage(),
    }
}

/// The shard count for this run: the override if set, else available
/// parallelism, capped at the cell-grid size (more children than cells
/// would only spawn empty sweeps).
fn shard_count() -> usize {
    let requested = match std::env::var_os(SHARDS_ENV) {
        Some(value) => value
            .to_str()
            .and_then(|text| text.parse::<usize>().ok())
            .filter(|&count| count >= 1)
            .unwrap_or_else(|| {
                panic!("amp-board: {SHARDS_ENV} must be a positive integer, got {value:?}")
            }),
        None => std::thread::available_parallelism().map_or(1, usize::from),
    };
    requested.min(board::max_useful_shards())
}

/// Build the child spawner: launch `count` copies of this binary
/// concurrently, one cell-grid slice each at the given scale, and
/// return their stdout captures in shard order (the merge validates
/// each child's stamps).
fn spawner(count: usize) -> impl Fn(f64) -> io::Result<Vec<Vec<u8>>> {
    move |scale: f64| {
        let exe = std::env::current_exe()?;
        let children = (0..count)
            .map(|index| {
                Command::new(&exe)
                    .arg("--shard")
                    .arg(format!("{index}/{count}"))
                    .arg("--scale-bits")
                    .arg(format!("{:016x}", scale.to_bits()))
                    .stdout(Stdio::piped())
                    .spawn()
            })
            .collect::<io::Result<Vec<_>>>()?;
        children
            .into_iter()
            .enumerate()
            .map(|(index, child)| {
                let output = child.wait_with_output()?;
                // The child's stderr is inherited, so its own panic
                // message precedes this refusal on the terminal.
                assert!(
                    output.status.success(),
                    "amp-board: shard child {index}/{count} failed: {status}",
                    status = output.status
                );
                Ok(output.stdout)
            })
            .collect()
    }
}

/// Parse the child-mode shard argument `i/N`.
fn parse_shard_spec(spec: &str) -> (usize, usize) {
    let parsed = spec
        .split_once('/')
        .and_then(|(index, count)| Some((index.parse().ok()?, count.parse().ok()?)));
    parsed.unwrap_or_else(|| panic!("amp-board: malformed --shard argument {spec:?}"))
}

/// Child mode: sweep one family slice and emit samples on stdout in the
/// shard wire form (internal; spawned by the parent run below).
fn run_child(args: &[String]) {
    let (index, count) = parse_shard_spec(
        args.get(1)
            .unwrap_or_else(|| panic!("amp-board: --shard requires an i/N argument")),
    );
    let bits = match (args.get(2).map(String::as_str), args.get(3)) {
        (Some("--scale-bits"), Some(bits)) if args.len() == 4 => bits,
        _ => panic!("amp-board: --shard takes exactly `i/N --scale-bits <hex>`"),
    };
    let scale = f64::from_bits(
        u64::from_str_radix(bits, 16)
            .unwrap_or_else(|_| panic!("amp-board: malformed --scale-bits argument {bits:?}")),
    );
    let mut out = std::io::stdout().lock();
    board::emit_shard(scale, index, count, &heap_meter(), &mut out).expect("stdout stays writable");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--shard") {
        run_child(&args);
        return;
    }

    let shards = shard_count();
    let mut out = std::io::stdout().lock();
    match args.first().map(String::as_str) {
        Some("worst-cases") => {
            for (label, scale) in board::WORST_MAP_SCALES {
                board::worst_map(label, scale, shards, &spawner(shards), &mut out)
                    .expect("stdout stays writable");
            }
        }
        Some("worst-cases-check") => {
            let clean = board::check_worst_map(shards, &spawner(shards), &mut out)
                .expect("stdout stays writable");
            if !clean {
                std::process::exit(1);
            }
        }
        arg => {
            let scale = match arg {
                None => DEFAULT_SCALE,
                Some("acceptance") => board::ACCEPTANCE_SCALE,
                Some(arg) => arg.parse::<f64>().unwrap_or_else(|_| {
                    panic!("amp-board: scale must be a positive number, got {arg:?}")
                }),
            };
            let summary = board::run(scale, shards, &spawner(shards), &mut out)
                .expect("stdout stays writable");
            // The verdicts are consumed, not just rendered: at the
            // scales of record (default and acceptance, the two the
            // all-green acceptance criterion is stated over), any red
            // cell exits nonzero, so a red board cannot pass a gate
            // leg that runs it — a red is an untriaged contradiction,
            // resolved only by a cure or an owner-declared model at
            // the cell. Other scales stay debugging views whose
            // verdicts are not of record and never bind.
            let of_record = scale == DEFAULT_SCALE || scale == board::ACCEPTANCE_SCALE;
            if of_record && summary.red != 0 {
                eprintln!(
                    "amp-board: {red} red cells at scale {scale}",
                    red = summary.red
                );
                std::process::exit(1);
            }
        }
    }
}
