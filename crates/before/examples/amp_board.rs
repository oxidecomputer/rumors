//! Runs the amplification board and prints the red-green matrix to stdout.
//!
//! The board itself — the operation × input-family sweep, the meters, the
//! green/red criterion, and the not-applicable coverage list — lives in
//! `before::testing::meter::board`; this binary installs the counting allocator
//! the board's peak-heap column reads (a global allocator is per-binary
//! state the library cannot own), parses the size knob, and orchestrates
//! the process sharding below.
//!
//! Usage: `just amp-board` (release, the profile of record — dev runs are
//! a debugging view whose readings are never pinned), or directly
//! `cargo run -p before --example amp_board --features touch-meter,scan-meter
//! -- [scale]` where the optional `scale` (a positive number, default
//! `board::DEFAULT_SCALE`) multiplies every input family's base size and
//! renders a single-scale debugging view whose verdicts never bind. The counter
//! features are `required-features`: a build without them would render scan
//! and touch unjudged while still printing verdict colors, so cargo refuses it
//! outright.
//!
//! # The verdict of record
//!
//! `acceptance [--capture <file>]` (`just amp-board-acceptance`) sweeps the
//! measurement ladder once (two sizes at each of the two growth sampling
//! scales, `board::DEFAULT_SCALE` and `board::LADDER_TOP_SCALE`, plus a
//! small-input pair at `board::SMALL_INPUT_SCALE`) and judges that one sweep
//! twice:
//!
//! 1. The acceptance judgment renders one matrix per sweep (the ladder's
//!    base, its top, and the small inputs), fitting each exponent as one trend
//!    over the four growth points and judging every constant, model band, and
//!    liveness floor at every size.
//! 2. The worst-case ranking pin entry-compares the argmax family per
//!    operation × currency at both growth scales against the committed table,
//!    printing one drift line per disagreement.
//!
//! Both judgments always run, so a red cell never hides ranking drift. Red
//! rows and drift lines print to stdout with the matrices; stderr carries a
//! one-line summary of each failed judgment (the red-cell count, and a notice
//! that the pin drifted). The run exits nonzero if any cell is red anywhere on
//! the ladder or any ranking drifted, so every gate leg that runs the board
//! consumes the exit code.
//!
//! With `--capture <file>`, the run also writes every cell's exact readings at
//! every size to `<file>` (replacing it only once the whole capture is
//! written), whatever the verdict. `compare <before> <after>` reads two
//! captures and prints, per cell and currency, the exact difference at each
//! size, classified as unchanged, constant, growing (with its slope), or
//! uneven; it judges neither side. Use the pair where a rendered board's
//! rounded exponent moves and the question is whether a cost grows with input
//! or only shifts: a fixed-cost change alone moves a fitted exponent. Take both
//! captures with `just amp-board-acceptance --capture <file>`, so each is a
//! release reading, and on one host: a capture records its target and
//! debug-assertion setting, and the comparison refuses captures that differ in
//! either. `board::compare` documents the capture format, the classification
//! rule, and what the rule cannot distinguish.
//!
//! # The worst-case map
//!
//! `worst-cases` (`just worst-cases`) renders the worst-case map itself, with
//! runner-ups and margins, at each of the ladder's growth sampling scales
//! (`board::WORST_MAP_SCALES`). It is a reading aid; the pin above is its
//! verdict.
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
//! judged quantity is, for a given build target, a deterministic counter
//! over state a child owns privately, so the shard count is a throughput
//! setting and never an input to a reading: `AMP_BOARD_SHARDS` overrides
//! it (default: available parallelism).

use std::fs;
use std::io::{self, Write as _};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use before::testing::meter::board::{self, HeapMeter};
use peak_alloc::PeakAlloc;
use tempfile::NamedTempFile;

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

/// The shard-count override. Unset, the runner uses available
/// parallelism.
const SHARDS_ENV: &str = "AMP_BOARD_SHARDS";

/// Live bytes at the board's last peak reset: the level its peak readings rise
/// from.
static PEAK_ORIGIN: AtomicUsize = AtomicUsize::new(0);

/// The peak-heap readers over this binary's global allocator.
///
/// The allocator counts process-wide, which the board's single-threaded sweep
/// makes exact: no other thread allocates while a cell is measured.
fn heap_meter() -> HeapMeter {
    HeapMeter {
        reset_peak: || {
            HEAP.reset_peak_usage();
            PEAK_ORIGIN.store(HEAP.current_usage(), Ordering::Relaxed);
        },
        peak: || {
            HEAP.peak_usage()
                .saturating_sub(PEAK_ORIGIN.load(Ordering::Relaxed))
        },
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

/// Unwrap a sweep's, a capture file's, or a capture format's result, or report
/// its error and exit nonzero.
fn exit_on_error<T>(result: io::Result<T>, doing: &str) -> T {
    result.unwrap_or_else(|error| {
        eprintln!("amp-board: {doing}: {error}");
        std::process::exit(1)
    })
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
        Some("acceptance") => {
            let capture_path = match &args[1..] {
                [] => None,
                [flag, path] if flag == "--capture" => Some(Path::new(path)),
                _ => panic!("amp-board: acceptance takes no arguments or `--capture <file>`"),
            };
            let ladder = exit_on_error(
                board::Ladder::sweep(shards, &spawner(shards)),
                "sweeping the board",
            );
            let verdict = ladder.judge(&mut out).expect("stdout stays writable");
            // The verdict's summary prints before the capture is attempted,
            // so a failed capture write cannot leave a red board's stderr
            // without it.
            if verdict.summary.red != 0 {
                eprintln!(
                    "amp-board: {red} red cells across the measurement ladder",
                    red = verdict.summary.red
                );
            }
            if !verdict.pin_clean {
                eprintln!("amp-board: the worst-case ranking pin drifted (drift lines above)");
            }
            // A capture records readings, not a verdict, so it is written
            // whatever the verdict: a red board's capture is the evidence a
            // `compare` against the last green one needs. It is written to a
            // temporary file beside `path` and renamed into place, so a
            // failed write leaves any existing capture intact and no
            // temporary file behind.
            if let Some(path) = capture_path {
                let mut text = Vec::new();
                board::capture(&ladder, &mut text).expect("writing to a Vec succeeds");
                let written = write_atomically(path, &text);
                exit_on_error(written, &format!("writing capture {path:?}"));
            }
            if !verdict.passed() {
                std::process::exit(1);
            }
        }
        Some("compare") => {
            let [_, before, after] = args.as_slice() else {
                panic!("amp-board: compare takes exactly two capture paths, before then after");
            };
            let read = |path: &String| {
                let text = fs::read_to_string(path);
                exit_on_error(text, &format!("reading capture {path:?}"))
            };
            let (before, after) = (read(before), read(after));
            exit_on_error(
                board::compare(&before, &after, &mut out),
                "comparing captures",
            );
        }
        // A single-scale render is a debugging view: its verdict colors
        // ride per-window fits and never bind (the acceptance mode above
        // is the one verdict of record).
        arg => {
            let scale = match arg {
                None => board::DEFAULT_SCALE,
                Some(arg) => arg.parse::<f64>().unwrap_or_else(|_| {
                    panic!("amp-board: scale must be a positive number, got {arg:?}")
                }),
            };
            board::run(scale, shards, &spawner(shards), &mut out).expect("stdout stays writable");
        }
    }
}

/// Replace `path` with `bytes` in one rename, or leave it untouched.
///
/// The temporary file lives in `path`'s directory so the rename stays on one
/// filesystem, and it deletes itself if the write or rename fails.
fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut file = NamedTempFile::new_in(dir)?;
    file.write_all(bytes)?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}
