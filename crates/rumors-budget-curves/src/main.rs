//! The budget curves: how a synchronization slows as its memory budget shrinks
//! below what keeps a link busy, measured on the library's own session code
//! over the test kit's link model, and plotted against the estimates in
//! [`rumors_testkit::bench::slowdown`]: the profile estimate, which stretches
//! each moment of the unlimited session by how far its memory exceeds the
//! budget, where the unlimited sessions recorded their profiles, and the
//! analytic lower estimate otherwise.
//!
//! Every session is deterministic: the link model's clock counts byte-times and
//! advances only when both peers wait, so on one platform each point is an
//! exact function of its pair, link, and budget, unmoved by machine load.
//! Across platforms the points can differ slightly: the same sessions measured
//! on Linux and on illumos differ in their heap figures and admission counts,
//! and occasionally in duration.
//!
//! - `simulate` runs the grid and appends one JSON document per session to its
//!   output file. For each pair and link it first runs the session with an
//!   unlimited budget, which reads the budget no smaller one beats (`Λ`, the
//!   largest excess either side's ledger holds: what its kinds and depths of
//!   unit hold beyond their bounds), then a ladder of budgets from `Λ` down to
//!   [`MIN_SYNC_MEMORY_BUDGET`](rumors::MIN_SYNC_MEMORY_BUDGET), the least
//!   budget the public setter runs a session on, each run beside the floor and
//!   session allowances every memory budget sets aside, since `Λ` and the rungs
//!   below it are what a ledger admits work against. Explicit budgets run as
//!   given, below the minimum included, so analysis can probe the regime the
//!   setter rules out. A rerun with the same output file skips every session
//!   the file already holds. Every unlimited session records each side's
//!   budget-counted bytes over time, its profile, which the profile estimate
//!   reads; with `--detail`, every session records its profile, each wait for
//!   admission, and each admission, dated, to show when and where a budget held
//!   work back.
//! - `render` reads such a file and writes the plots (SVG) and the tables
//!   (Markdown) beside them.
//! - `compare` overlays several simulations' curves, one figure per pair.
//! - `model` runs each shape's unlimited sessions, over a latency-only link and
//!   over each link with bandwidth, and writes the budget model's quantities:
//!   the hop count, the bytes each side sent, the unlimited duration, and each
//!   side's `Λ` and share peak with where and when each occurred. Shapes name
//!   insertions and redactions on each side.
//! - `count` prints the grants of the depth-1 questions a session's root answer
//!   asks, the budget that opens every top-level subtree at once, stopping each
//!   session after its first round trip, so it reaches replica sizes whole
//!   sessions cannot.
//!
//! `simulate`, `model`, and `count` run each session in a child process of its
//! own ([`child`]), because a large session leaves the allocator in a state
//! that slows the next large session in the same process. The parent writes
//! each session's document exactly as its child printed it.

mod child;
mod grid;
mod model;
mod plot;
mod record;
mod report;
mod simulate;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use std::time::Instant;

use rumors_testkit::bench::session::{self, MESSAGE_BYTES, Shape};

/// The command line.
#[derive(Parser)]
#[command(about = "Measure and plot synchronization slowdown against memory budget")]
struct Cli {
    /// What to do.
    #[command(subcommand)]
    command: Command,
}

/// The modes.
#[derive(Subcommand)]
enum Command {
    /// Run the sessions, appending each to a JSON-lines file.
    Simulate {
        /// The file to append sessions to; sessions it holds are skipped.
        #[arg(long)]
        out: PathBuf,
        /// Messages each replica holds, per pair.
        #[arg(long, value_delimiter = ',', default_values_t = [100_000u64, 1_000_000])]
        sizes: Vec<u64>,
        /// The share of each replica the other holds too, in percent.
        #[arg(long, value_delimiter = ',', default_values_t = [0u64, 90, 99])]
        overlaps: Vec<u64>,
        /// Links by bandwidth-delay product, in bytes.
        #[arg(long, value_delimiter = ',', default_values_t = grid::LINKS)]
        links: Vec<u64>,
        /// Sessions to run at once.
        #[arg(long, default_value_t = 16)]
        jobs: usize,
        /// Run only each pair's unlimited session, which reads `Λ`.
        #[arg(long)]
        unlimited_only: bool,
        /// Rungs per halving in the ladder below `Λ`; zero runs only the
        /// ladder's candidates, the powers of two below `Λ`.
        #[arg(long, default_value_t = 4)]
        steps_per_octave: u32,
        /// Halvings the ladder spans below `Λ`.
        #[arg(long, default_value_t = 10)]
        octaves: u32,
        /// Leave out the minimum's rung, `MIN_SYNC_MEMORY_BUDGET`, where the
        /// ladder starts.
        #[arg(long)]
        no_minimum: bool,
        /// The smallest power of two to run among the ladder's candidates, in
        /// bytes beside the floor and session allowances.
        #[arg(long, default_value_t = grid::SMALLEST_CANDIDATE)]
        smallest_candidate: u64,
        /// Run exactly these memory budgets, in bytes, instead of the ladder,
        /// below the minimum included; the unlimited session runs first unless
        /// the output holds it.
        #[arg(long, value_delimiter = ',')]
        budgets: Vec<u64>,
        /// The bytes each fixture message encodes in, its CBOR head included.
        #[arg(long, default_value_t = MESSAGE_BYTES)]
        message_bytes: usize,
        /// Record every session in detail: each side's profile, every wait, and
        /// every admission, dated. Without it only unlimited sessions record
        /// their profiles, which the profile estimate reads.
        #[arg(long)]
        detail: bool,
    },
    /// Overlay several simulations' curves, one figure per pair, each series
    /// given as `label=path`.
    Compare {
        /// The series, `label=path`, at most three plotted per figure.
        #[arg(long, value_delimiter = ',')]
        series: Vec<String>,
        /// The directory to write the SVGs into.
        #[arg(long)]
        out: PathBuf,
    },
    /// Print the grants of the depth-1 questions the responder's root answer
    /// asks, and their total, one JSON line per size: the budget a session
    /// needs to open every top-level subtree at once.
    Count {
        /// Messages each replica holds.
        #[arg(long, value_delimiter = ',')]
        sizes: Vec<u64>,
        /// The share of each replica the other holds too, in percent.
        #[arg(long, default_value_t = 99)]
        overlap: u64,
        /// The bytes each fixture message encodes in, its CBOR head included.
        #[arg(long, default_value_t = MESSAGE_BYTES)]
        message_bytes: usize,
    },
    /// Run each shape's unlimited sessions over a latency-only link and over
    /// each link, appending the model's quantities, one JSON line per session;
    /// sessions the file holds are skipped.
    Model {
        /// The file to append sessions to.
        #[arg(long)]
        out: PathBuf,
        /// The shapes, each `s:iA:iB` or `s:iA:iB:rA:rB`: messages both hold,
        /// messages each side inserted that the other never saw, and messages
        /// both received that each side redacted.
        ///
        /// A suffix `@draw` builds another tree for the same counts.
        #[arg(long, value_delimiter = ',', value_parser = model::ShapeSpec::parse, required = true)]
        shapes: Vec<model::ShapeSpec>,
        /// Links with bandwidth, by bandwidth-delay product, in bytes.
        #[arg(long, value_delimiter = ',', default_values_t = grid::LINKS)]
        links: Vec<u64>,
        /// The latency-only link's bandwidth-delay product, twice its one-way
        /// delay, in bytes.
        #[arg(long, default_value_t = rumors_testkit::bench::session::LONG_HAUL_BDP)]
        latency_bdp: u64,
        /// The bytes each fixture message encodes in, its CBOR head included.
        #[arg(long, default_value_t = MESSAGE_BYTES)]
        message_bytes: usize,
        /// Sessions to run at once.
        #[arg(long, default_value_t = 4)]
        jobs: usize,
    },
    /// Render the plots and tables from a simulation's file.
    Render {
        /// The JSON-lines file `simulate` wrote.
        #[arg(long)]
        data: PathBuf,
        /// The directory to write the SVGs and `tables.md` into.
        #[arg(long)]
        out: PathBuf,
    },
    /// Run one session, given as JSON, and print its document: the other modes
    /// run each session in a child process through this command.
    #[command(hide = true)]
    Session {
        /// The session, as JSON.
        job: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().command {
        Command::Simulate {
            out,
            sizes,
            overlaps,
            links,
            jobs,
            unlimited_only,
            steps_per_octave,
            octaves,
            no_minimum,
            smallest_candidate,
            budgets,
            message_bytes,
            detail,
        } => {
            let ladder = (!unlimited_only).then_some(grid::Ladder {
                steps_per_octave,
                octaves,
                minimum: !no_minimum,
                smallest_candidate,
                explicit: budgets,
            });
            simulate::run(
                &out,
                &grid::pairs(&sizes, &overlaps, &links),
                ladder,
                message_bytes,
                detail,
                jobs,
            )
        }
        Command::Render { data, out } => report::render(&data, &out),
        Command::Model {
            out,
            shapes,
            links,
            latency_bdp,
            message_bytes,
            jobs,
        } => model::run(&out, &shapes, &links, latency_bdp, message_bytes, jobs),
        Command::Compare { series, out } => {
            let series: Vec<(String, PathBuf)> = series
                .iter()
                .map(|spec| {
                    let (label, path) = spec.split_once('=').ok_or("a series is label=path")?;
                    Ok((label.to_owned(), PathBuf::from(path)))
                })
                .collect::<Result<_, Box<dyn std::error::Error>>>()?;
            report::compare(&series, &out)
        }
        Command::Count {
            sizes,
            overlap,
            message_bytes,
        } => {
            // One size at a time, in order, so the lines print in the order
            // the sizes were given.
            for messages in sizes {
                let job = child::Job::Count {
                    messages,
                    overlap,
                    message_bytes,
                };
                println!("{}", job.run_in_child()?);
            }
            Ok(())
        }
        Command::Session { job } => {
            let job: child::Job = serde_json::from_str(&job)?;
            println!("{}", job.run_here());
            Ok(())
        }
    }
}

/// Counts the grants of the depth-1 questions the responder's root answer asks
/// in the pair of `messages` per replica, `overlap` percent shared, every
/// message encoding in `message_bytes`, and returns them as a JSON document.
fn count(messages: u64, overlap: u64, message_bytes: usize) -> String {
    let started = Instant::now();
    let shape = Shape::overlapping(messages, overlap).with_message_bytes(message_bytes);
    let grants = session::root_question_grants(shape);
    serde_json::json!({
        "messages": messages,
        "shared_percent": overlap,
        "message_bytes": message_bytes,
        "questions": grants.len(),
        "total": grants.iter().sum::<usize>(),
        "min": grants.iter().min(),
        "max": grants.iter().max(),
        "grants": grants,
        "seconds": started.elapsed().as_secs_f64(),
    })
    .to_string()
}
