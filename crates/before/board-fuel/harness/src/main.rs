//! Runs the fuel board: the amplification board's cells in wasm guests,
//! judged in Wasmtime fuel.
//!
//! Usage (the `just amp-board-fuel*` recipes pass the guest's path in
//! `BOARD_FUEL_GUEST_WASM`, and name the source tree in
//! `AMP_BOARD_SOURCE_TREE`):
//!
//! - `acceptance [--capture <file>]` sweeps the board's measurement ladder,
//!   one fresh guest per cell, and renders the board's acceptance judgment of
//!   the fuel readings. It exits nonzero if any cell is red, and refuses to
//!   judge at all if the guest was built under a compiler, Wasmtime, or cost
//!   table other than the fuel ceilings' calibration, or embeds a path under
//!   the home directory. With `--capture`, it also writes the sweep's exact
//!   readings, which `just amp-board compare` compares like any board capture.
//! - `calibrate [<capture>]` judges nothing and rewrites the fuel ceiling
//!   table, recording the guest's provenance and printing every cell whose
//!   ceiling moved. Without a capture it sweeps the same ladder for the
//!   readings. With the capture an `acceptance --capture` run of this source
//!   tree wrote, it derives the table from that capture's readings instead,
//!   refusing a capture of another tree or build, and requires the guest to
//!   match the table's recorded provenance, as acceptance required of the
//!   guest that took the capture.
//!
//! `AMP_BOARD_FUEL_THREADS` sets the number of worker threads (default: the
//! available parallelism). It moves no reading: every cell runs in its own
//! instance.

use std::io;
use std::path::{Path, PathBuf};

use before::testing::meter::board::{self, Currency, Ladder};
use board_fuel_harness::{check_guest, check_paths, provenance, FuelBoard};

/// The environment variable naming the compiled guest module.
const GUEST_ENV: &str = "BOARD_FUEL_GUEST_WASM";

/// The environment variable overriding the worker count.
const THREADS_ENV: &str = "AMP_BOARD_FUEL_THREADS";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["acceptance"] => acceptance(None),
        ["acceptance", "--capture", path] => acceptance(Some(Path::new(path))),
        ["calibrate"] => calibrate(None),
        ["calibrate", path] => calibrate(Some(Path::new(path))),
        _ => fail(
            "usage: board-fuel-harness (acceptance [--capture <file>] | calibrate [<capture>])",
        ),
    }
}

/// Reports `message` and exits nonzero.
fn fail(message: &str) -> ! {
    eprintln!("amp-board-fuel: {message}");
    std::process::exit(1)
}

/// Unwraps `result`, or reports its error while `doing` and exits nonzero.
fn exit_on_error<T>(result: io::Result<T>, doing: &str) -> T {
    result.unwrap_or_else(|error| fail(&format!("{doing}: {error}")))
}

/// Reads the compiled guest that `BOARD_FUEL_GUEST_WASM` names.
fn guest() -> Vec<u8> {
    let path = std::env::var_os(GUEST_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| fail(&format!("{GUEST_ENV} must name the compiled guest")));
    exit_on_error(std::fs::read(&path), &format!("reading {path:?}"))
}

/// Returns the home directory, under which no path may appear in the guest.
fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| fail("HOME must name the home directory"))
}

/// Sweeps the board's whole ladder in fresh instances of guest `wasm`.
fn sweep(wasm: &[u8]) -> Ladder {
    let threads = match std::env::var(THREADS_ENV) {
        Ok(text) => text
            .parse::<usize>()
            .ok()
            .filter(|&threads| threads > 0)
            .unwrap_or_else(|| fail(&format!("{THREADS_ENV} must be a positive integer"))),
        Err(_) => std::thread::available_parallelism().map_or(1, usize::from),
    };
    let fuel_board = exit_on_error(FuelBoard::new(wasm, threads), "compiling the guest");
    exit_on_error(
        Ladder::sweep_jointly(board::max_useful_shards(), &|samplings| {
            fuel_board.spawn(samplings)
        }),
        "sweeping the board",
    )
}

/// The acceptance mode: judge the fuel readings, and capture them on request.
fn acceptance(capture: Option<&Path>) {
    let wasm = guest();
    if let Err(message) = check_guest(&wasm, &home()) {
        fail(&message);
    }
    // Named before the sweep, so a run that cannot write its capture fails
    // before it spends one.
    let source_tree = capture.map(|_| exit_on_error(board::source_tree(), "naming the tree"));
    let ladder = sweep(&wasm);
    let summary = exit_on_error(
        ladder.accept(&[Currency::Fuel], &mut io::stdout().lock()),
        "judging the board",
    );
    if summary.red != 0 {
        eprintln!(
            "amp-board-fuel: {red} red cells across the measurement ladder",
            red = summary.red
        );
    }
    // A capture records readings, not a verdict, so it is written whatever
    // the verdict: a red board's capture is what a comparison needs.
    if let Some((path, tree)) = capture.zip(source_tree) {
        exit_on_error(
            board::capture_file(&ladder, &tree, path),
            &format!("writing capture {path:?}"),
        );
    }
    if summary.red != 0 {
        std::process::exit(1);
    }
}

/// The calibrate mode: rewrite the fuel ceiling table from a fresh sweep, or
/// from `capture`, a capture of this tree's fuel board.
///
/// The guest's paths are checked as for acceptance. After a sweep, its
/// provenance is recorded, not compared. A capture does not name the guest
/// that took it, but acceptance writes one only under the table's
/// provenance, so a calibration from a capture requires this guest to match
/// that provenance as well, and the table keeps it.
fn calibrate(capture: Option<&Path>) {
    let source_tree = exit_on_error(board::source_tree(), "naming the tree");
    let wasm = guest();
    let provenance = provenance(&wasm).unwrap_or_else(|message| fail(&message));
    let checked = match capture {
        None => check_paths(&wasm, &home()),
        Some(_) => check_guest(&wasm, &home()),
    };
    if let Err(message) = checked {
        fail(&message);
    }
    let recalibrations = match capture {
        None => {
            let ladder = sweep(&wasm);
            // Fuel readings do not depend on the host, so no other capture
            // is merged.
            board::recalibrate(&ladder, &source_tree, &[], Some(&provenance))
        }
        Some(path) => {
            let text = exit_on_error(
                std::fs::read_to_string(path),
                &format!("reading capture {path:?}"),
            );
            let name = path.display().to_string();
            board::recalibrate_from_captures(
                Currency::Fuel,
                &source_tree,
                &[(&name, &text)],
                Some(&provenance),
            )
            .map(|recalibration| vec![recalibration])
        }
    };
    let recalibrations = exit_on_error(recalibrations, "recalibrating");
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    exit_on_error(
        board::write_recalibrations(&recalibrations, &crate_root, &mut io::stdout().lock()),
        "writing the table",
    );
}
