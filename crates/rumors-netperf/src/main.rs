//! Measure how fast [Rumors](https://github.com/oxidecomputer/rumors) replicas
//! synchronize over network links, on your own hardware.
//!
//! `rumors-netperf` builds pairs of replicas that differ as you describe, runs
//! one gossip session between them over TCP, and records how long it took, how
//! many bytes crossed in each direction, how much CPU and memory each side
//! used, and why the session took as long as it did: whether the link's
//! bandwidth, its round trip, or the computation set the pace. It measures the
//! library as an application uses it, a production build with no test hooks,
//! and writes a report you can read on its own.
//!
//! Use it to size a deployment: to learn how long replicas of your size,
//! diverged as yours diverge, take to converge over your links, and what memory
//! budget and TCP settings let them do it as fast as the link allows.
//!
//! # Quick start
//!
//! ```text
//! cargo build --release -p rumors-netperf
//! target/release/rumors-netperf doctor
//! target/release/rumors-netperf run --grid crates/rumors-netperf/grids/quick.toml
//! ```
//!
//! `doctor` says what this host can do and, for anything it cannot, what to
//! change. `run` measures the grid file it is given, here the quick grid of a
//! few minutes that ships beside this crate, prints progress as it goes, and
//! leaves a results directory (`netperf-<time>/` unless you pass `--out`)
//! holding `report.md`.
//!
//! Build the tool on its own, as above. It refuses to measure with a debug
//! build, and with a library that carries its test-only `test-internals`
//! feature, which cargo compiles in when one command builds the tool beside a
//! crate that enables it.
//!
//! # Two modes
//!
//! - **Emulation** (`run`, Linux only) measures across emulated links on one
//!   host. Each session's two peers run as separate processes in network
//!   namespaces of their own, joined by a virtual link that `netem` shapes to a
//!   rate and a round trip in each direction. Many links, from an intra-rack
//!   backplane to an intercontinental path, fit in one run. It needs no root:
//!   it runs in an unprivileged user namespace and never touches the host's
//!   network.
//! - **Pair** (`serve` on one host, `connect` on another) measures over the
//!   network between two hosts, on any platform Rumors supports, with nothing
//!   shaped. It first probes the network's round trip and goodput, and reads
//!   every session against them.
//!
//! Both modes run the same sessions and write the same report.
//!
//! A third mode, **in-process** (`duplex`, Linux only), answers a narrower
//! question: what a session costs each side's processor, with no link and no
//! kernel TCP stack. It builds each point's pair as emulation does, runs both
//! sides in one process over an in-memory duplex stream, each on its own
//! thread, and prints each side's thread CPU time, user and system separately,
//! with the session's counts, one line per session. It writes no report, and
//! ignores the link and connection axes.
//!
//! # Describing what to measure
//!
//! A *point* is one replica pair, its message sizes, the settings each
//! [`Peer`](rumors::Peer) runs with, and the link. A pair is described by five
//! counts:
//!
//! | count                                 | meaning                                               |
//! |---------------------------------------|-------------------------------------------------------|
//! | `shared` (*s*)                        | messages live on both sides                           |
//! | `insert_a`, `insert_b` (*i_A*, *i_B*) | messages one side inserted that the other never saw   |
//! | `redact_a`, `redact_b` (*r_A*, *r_B*) | messages one side redacted that the other still holds |
//!
//! Message sizes follow a lognormal distribution set by its mean and its
//! coefficient of variation (standard deviation over mean); a coefficient of
//! zero is a fixed size. Draws above the cap, 64 KiB unless you raise it, are
//! redrawn rather than clamped, and the results count them.
//!
//! Every setting `Peer` offers that can change a session is a parameter too:
//! each side's memory budget and target message size, the payload depth limit,
//! and a session deadline. So is the transport: by default TLS 1.3 over TCP, as
//! deployments run it, and plain TCP with `tls = off`, each set up as the
//! library advises. Every session's row records the values it ran with, and
//! counts the bytes and the write and read operations each side's socket saw
//! beneath every buffer and TLS: over TLS the bytes include each record's
//! framing, and on plain TCP the operations are its system calls.
//!
//! By default each session runs on a *warm* connection: one already open, which
//! has carried a bulk transfer each way so its congestion window is open, and
//! the session's clock starts after that setup. That is the session an
//! application sees when it keeps a connection to each partner with
//! `Rumors::gossip`, as it should. Choose `fresh` instead if your application
//! connects in order to synchronize: the session then opens its connection, and
//! its time includes connecting, with the TLS handshake over TLS, and slow
//! start.
//!
//! On the command line, each parameter is a flag that takes one value or a
//! comma-separated list, and the run measures every combination:
//!
//! ```text
//! rumors-netperf run --shared 1e5 --insert-a 1000 --insert-b 1000 \
//!     --rate 100Mbit,1Gbit,uncapped --rtt 1ms,10ms,100ms --budget default,32MiB
//! ```
//!
//! For anything larger, write a grid file: groups of axes, each group measuring
//! every combination of its axes, and a `pairs` array for counts that move
//! together. Each `pairs` entry describes its replica pair whole: any count it
//! leaves out is zero. `rumors-netperf run --help` lists every axis.
//!
//! ```toml
//! repetitions = 5
//!
//! [[group]]
//! name = "a peer that fell behind"
//! pairs = [
//!     { shared = 100000, insert_a = 1000 },
//!     { shared = 1000000, insert_a = 100000 },
//! ]
//! size_mean = 200
//! size_cv = 1.0
//! rate = ["100Mbit", "1Gbit", "uncapped"]
//! rtt = ["0.1ms", "10ms", "100ms", "250ms"]
//! budget = ["default", "128MiB", "32MiB"]
//! ```
//!
//! ```text
//! rumors-netperf run --grid my-grid.toml
//! ```
//!
//! The grid the Rumors project publishes its own figures from ships beside this
//! crate as `grids/reference.toml`; running it is how you check your hardware
//! against those figures.
//!
//! # Reading the report
//!
//! For each point, the report gives the median session time with an exact
//! confidence interval, and names what bound it: *bandwidth* when the busier
//! direction's bytes over the link's rate explain the time, *latency* when the
//! round trips the protocol needs do, and *computation* when the busier side's
//! CPU time does. It compares the round trips the session spent against the
//! fewest the protocol needs, shows how busy each direction of the link was,
//! how much slower each smaller budget made the session, how much memory the
//! session held against its budget, and, for links whose bandwidth-delay
//! product exceeds what the host's TCP limits let one connection keep in
//! flight, how much raising those limits changes.
//!
//! # TCP buffer limits
//!
//! A TCP connection keeps at most one window in flight per round trip, and the
//! host's buffer limits cap the window. From those limits the tool computes the
//! largest window a connection can use, and reruns, with the limits raised,
//! exactly the points whose bandwidth-delay product and busier direction both
//! exceed it. It raises them inside the session's own network namespaces only:
//! the host keeps its settings, and the report tells you whether yours need
//! raising.
//!
//! # Parallelism
//!
//! Each session has two parts. Its *preparation* builds the pair and moves side
//! B's replica to its process, which is most of a run's work and measures
//! nothing; its *timed session* is the gossip the row measures. The tool
//! prepares many sessions at once and times them apart from any preparation,
//! alternating between the two: it prepares a window of the next sessions to
//! time, then times them in the run's order, then prepares the next window. A
//! prepared session waits with its replicas in its two processes, idle, until
//! its turn, so a window holds only as many sessions as fit the memory bound:
//! half of physical memory by default, by a memory model calibration fits, or
//! `--prepare-memory`.
//!
//! Timed sessions that share a host can slow each other's computation or
//! distort the emulated links. Before a run, the tool calibrates: it runs the
//! run's heaviest session on its fastest link alone several times to measure
//! its natural spread, then 2, 4, 8, … at once, and times as many sessions at
//! once as the largest count whose sessions stay within that spread. It checks
//! the emulator with raw TCP transfers on the fastest shaped link,
//! independently of Rumors, and without TLS even when the sessions run over it:
//! that check judges the link alone, while TLS's cost belongs to the sessions,
//! which measure it. It also measures how many processors one preparation keeps
//! busy, and prepares as many sessions at once as fill the host's processors
//! (or `--prepare-jobs`). During the run it repeats the heaviest session as a
//! sentinel and flags drift if the host slows down. Calibrations are cached per
//! host and build, a build being identified by a digest of its executable, so a
//! change to the library or to the tool calibrates afresh; `--jobs N` skips
//! calibration, and `--recalibrate` discards the cache.
//! `rumors-netperf calibrate --grid` calibrates without running the grid and
//! predicts how long a run of it takes.
//!
//! # Splitting a run across hosts
//!
//! When the emulator itself saturates while processors sit idle, the kernel's
//! shaping is the limit, and the tool says so. Split a large run across hosts
//! with `--shard k/n`: the same grid on every host, `k` from 1 to `n`. Each
//! shard calibrates on its own host and writes its own results directory, and
//! `check` and `report` read the directories as one run.
//!
//! A point's repetitions fall to different shards, so the hosts must measure
//! alike: they must agree in everything `environment.json` records but their
//! names and their load, which is the operating system and kernel, the
//! processors, the memory, the TCP settings and congestion control, and the
//! build of the tool, by its executable's digest. Two builds of one revision
//! can differ, so build once and copy the executable. For two hosts, `host-1`
//! with a checkout and `host-2` with none:
//!
//! ```text
//! host-1$ cargo build --release --locked -p rumors-netperf
//! host-1$ scp target/release/rumors-netperf crates/rumors-netperf/grids/reference.toml host-2:
//! host-1$ target/release/rumors-netperf doctor
//! host-2$ ./rumors-netperf doctor
//! host-1$ target/release/rumors-netperf run --grid crates/rumors-netperf/grids/reference.toml --shard 1/2 --out reference-1
//! host-2$ ./rumors-netperf run --grid reference.toml --shard 2/2 --out reference-2
//! host-1$ scp -r host-2:reference-2 .
//! host-1$ target/release/rumors-netperf check reference-1 reference-2
//! ```
//!
//! Each `doctor` prints its host's build, whose executable digests must match.
//! `check` judges the two directories as one run and renders their one report
//! into `reference-1/check/`; `report` renders it anywhere else:
//!
//! ```text
//! rumors-netperf report reference-1 reference-2 --out reference
//! ```
//!
//! # Checking a run
//!
//! `check` says whether a run, or every shard of one, finished cleanly. It
//! judges facts any clean run satisfies on any host, never thresholds on
//! timings, and exits non-zero, naming each failure, unless all of them hold:
//!
//! - the results are whole: the shards given make up one run exactly once, and
//!   each finished;
//! - the shards measured alike: every directory's environment matches the
//!   first's in all but the host's name, the build's executable digest
//!   included;
//! - every session the run planned has exactly one row, across every part of a
//!   run that was cut short and resumed, and every point the grid skips is
//!   recorded with its reason;
//! - no session failed;
//! - every side that requested a budget below the library's minimum,
//!   [`MIN_SYNC_MEMORY_BUDGET`](rumors::MIN_SYNC_MEMORY_BUDGET), reports that
//!   it ran at that minimum;
//! - if the host's TCP limits leave a window smaller than some point keeps in
//!   flight, at least one rerun with raised limits finished;
//! - every row records when its preparation and its timed session ran, no timed
//!   session overlapped any preparation, and no more timed sessions overlapped
//!   than the run's job count;
//! - the report renders; `check` renders it into `check/` inside the first
//!   directory unless you pass `--out`.
//!
//! ```text
//! rumors-netperf check netperf-shard-1 netperf-shard-2
//! ```
//!
//! # Finishing a run cut short
//!
//! A run records each session as it finishes, appending its row to
//! `sessions.csv` and syncing it to the disk, so a run cut short, by a killed
//! process or a crash of the host, keeps every session it finished. To finish
//! it, resume it in its results directory:
//!
//! ```text
//! rumors-netperf run --resume netperf-1767225600
//! ```
//!
//! A resumed run takes no grid, shard, or job flags: it runs the directory's
//! plan as the run began it, with the same job counts, memory bound, and
//! calibration, and runs every session the directory has no row for, appending
//! to `sessions.csv`. A session that failed has a row, so it is not run again.
//! If the run was cut short while writing a row, the resumed run removes that
//! torn row, says so, and runs its session again. It refuses to resume with
//! another build of the tool, on another host, or under another kernel, naming
//! what differs, since those rows would not measure the same thing as the rest.
//! A build is identified by a SHA-256 of its executable, so any change to the
//! library, the tool, or the compiler makes another build, uncommitted edits
//! included. `run.json` records each resumption, and the report and `check`
//! read the parts as one run.
//!
//! The smoke grid that ships beside this crate, `grids/smoke.toml`, runs every
//! kind of point the reference grid does, each group at its smallest, beside
//! the reference run's heaviest session, and calibrates as a reference run
//! does: run it and check it before a long run.
//!
//! # Setting up a Linux host
//!
//! Emulation needs `unshare` and `nsenter` (util-linux), `ip` and `tc`
//! (iproute2), `ethtool`, and `sysctl` (procps); unprivileged user namespaces;
//! and the `sch_netem` kernel module. `doctor` checks each and names the fix.
//! In Docker, run the tool as an ordinary user in a container started with
//! `--security-opt seccomp=unconfined --security-opt systempaths=unconfined`:
//! `NET_ADMIN` alone does not allow creating network namespaces or raising TCP
//! limits inside them.
//!
//! # Pair mode
//!
//! ```text
//! server$ rumors-netperf serve
//! client$ rumors-netperf connect server.example:7979 --grid my-grid.toml
//! ```
//!
//! Both hosts must run the tool built from the same revision; the server
//! refuses a client whose version or revision differs. Side A of every session
//! runs on the server, side B on the client, and the client writes the results.
//! The server only needs its port reachable; each session also opens one
//! ephemeral port on it. Rate and round-trip axes do not apply, since the
//! network sets both. The server starts one process at a time, so pair mode
//! prepares each session just before timing it.
//!
//! # What the measurements are
//!
//! - Every session builds a fresh, independent pair: the trees two sessions
//!   compare are separate random draws.
//! - A timed session runs with no preparation beside it, and with no other
//!   timed session beyond calibration's job count. Its replicas were built a
//!   while before, while other sessions were prepared, so it starts with
//!   processor caches that hold nothing of its own. Each row's `build_seconds`
//!   and `fetch_seconds` were measured with other preparations sharing the
//!   host; the rows' intervals say which.
//! - Timings come from an optimized build of the tool and a default-feature
//!   build of the library. The tool counts heap through a thin wrapper around
//!   the system allocator, which adds two uncontended atomic operations per
//!   allocation.
//! - Emulated links add delay and a rate, not loss; the queue in front of each
//!   rate holds one bandwidth-delay product of packets, as a router's buffer
//!   would.
//! - `environment.json` records the host, its kernel and TCP limits, and the
//!   build; every row of `sessions.csv` records its settings, so results from
//!   different hosts or builds can be compared outside the tool. `plan.json`
//!   records what the run set out to measure, the points its grid skips among
//!   them, with their reasons.

mod calibrate;
mod check;
mod doctor;
mod driver;
mod duplex;
mod emulate;
mod env;
mod fixture;
mod grid;
mod meter;
mod netns;
mod pair;
mod peer;
mod plot;
mod point;
mod predict;
mod probe;
mod record;
mod report;
mod run;
mod schedule;
mod sizes;
mod stats;
mod transport;
mod units;

use std::collections::BTreeMap;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use anyhow::{Context, Result, bail, ensure};
use clap::{Args, Parser, Subcommand};

use crate::calibrate::Calibration;
use crate::grid::{AXES, Grid, Group, Shard};
use crate::point::{Link, Point};
use crate::report::Network;
use crate::run::{Part, Plan, Preparation, RunRecord};
use crate::units::{human_bytes, human_seconds};

/// Repetitions per point when neither the grid nor the command line says.
const DEFAULT_REPETITIONS: u32 = 5;

/// The command line.
#[derive(Parser, Debug)]
#[command(
    name = "rumors-netperf",
    version,
    about = "Measure how fast Rumors replicas synchronize over emulated links and between hosts",
    long_about = "Measure how fast Rumors replicas synchronize over emulated links and between hosts.\n\n\
        Start with `rumors-netperf doctor`, then `rumors-netperf run --grid grids/quick.toml`, the quick grid \
        that ships in the tool's crate. Each run writes a results directory with a per-session CSV, the \
        environment, and a report (report.md)."
)]
struct Cli {
    /// What to do.
    #[command(subcommand)]
    command: Command,
}

/// The commands.
#[derive(Subcommand, Debug)]
enum Command {
    /// Check what this host can do, and say how to fix what it cannot.
    Doctor,
    /// Measure a grid of points over emulated links on this host (Linux).
    #[command(after_long_help = axes_help())]
    Run(RunArgs),
    /// Calibrate parallelism for a grid without running it.
    Calibrate(CalibrateArgs),
    /// Measure each point's session CPU in this process, over an in-memory
    /// duplex stream, each side on its own thread (Linux).
    #[command(after_long_help = axes_help())]
    Duplex(DuplexArgs),
    /// Serve the other side of pair-mode sessions to `connect` on another host.
    Serve {
        /// The address to listen on.
        #[arg(long, default_value_t = SocketAddr::from(([0, 0, 0, 0], pair::DEFAULT_PORT)))]
        listen: SocketAddr,
    },
    /// Measure a grid over the network to a host running `serve`.
    #[command(after_long_help = axes_help())]
    Connect(ConnectArgs),
    /// Check that a run, or every shard of one, finished cleanly; exits
    /// non-zero, naming each failure, unless it did.
    Check {
        /// The results directories: one run's, or every shard of one run.
        #[arg(required = true)]
        dirs: Vec<PathBuf>,
        /// Where to render the report the check renders; `check` inside the
        /// first directory by default.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Render the report again from one results directory, or merge shards.
    Report {
        /// The results directories: one run's, or every shard of one run.
        #[arg(required = true)]
        dirs: Vec<PathBuf>,
        /// Where to write the report; the first directory by default.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// One side of one session, driven over standard input and output.
    #[command(hide = true)]
    Peer,
    /// One side of one link probe, driven over standard input and output.
    #[command(hide = true)]
    ProbePeer,
}

/// The `--help` text listing every axis.
fn axes_help() -> String {
    let mut text = String::from(
        "Axes (each a flag taking a comma-separated list, or a key in a grid file's [[group]]):\n",
    );
    for (name, meaning) in AXES {
        text.push_str(&format!("  {:<24}{meaning}\n", name.replace('_', "-")));
    }
    text.push_str(
        "\nValues: counts as integers or `1e5`; sizes as bytes or with units (`32MiB`, `4MB`); rates as `100Mbit`, \
         `1Gbit`, `uncapped`, or `A/B`; times as `250us`, `10ms`, `2s`.\n\
         Defaults: shared 1e5, insert-a and insert-b 1000, fixed 100-byte messages capped at 64KiB, the library's \
         Peer defaults, 1Gbit at 1ms, warm connections, plain TCP.",
    );
    text
}

/// Where the grid comes from.
#[derive(Args, Debug, Default)]
struct GridArgs {
    /// A grid file (TOML): groups of axes, each measured in every combination.
    #[arg(long)]
    grid: Option<PathBuf>,
    /// Sessions per point; overrides the grid file's `repetitions`.
    #[arg(long)]
    repetitions: Option<u32>,
    /// Messages live on both sides.
    #[arg(long)]
    shared: Option<String>,
    /// Messages only A inserted.
    #[arg(long)]
    insert_a: Option<String>,
    /// Messages only B inserted.
    #[arg(long)]
    insert_b: Option<String>,
    /// Sets insert-a and insert-b alike.
    #[arg(long)]
    insert: Option<String>,
    /// Messages A redacted that B still holds.
    #[arg(long)]
    redact_a: Option<String>,
    /// Messages B redacted that A still holds.
    #[arg(long)]
    redact_b: Option<String>,
    /// Sets redact-a and redact-b alike.
    #[arg(long)]
    redact: Option<String>,
    /// Mean encoded message size in bytes.
    #[arg(long)]
    size_mean: Option<String>,
    /// Message-size coefficient of variation; 0 for a fixed size.
    #[arg(long)]
    size_cv: Option<String>,
    /// Message-size cap; larger draws are redrawn (default 64KiB).
    #[arg(long)]
    max_message_size: Option<String>,
    /// Peer::sync_memory_budget for both sides, `default`, or `unlimited`.
    #[arg(long)]
    budget: Option<String>,
    /// Side A's sync_memory_budget.
    #[arg(long)]
    budget_a: Option<String>,
    /// Side B's sync_memory_budget.
    #[arg(long)]
    budget_b: Option<String>,
    /// Peer::target_message_size for both sides, or `default`.
    #[arg(long)]
    target_message_size: Option<String>,
    /// Side A's target_message_size.
    #[arg(long)]
    target_message_size_a: Option<String>,
    /// Side B's target_message_size.
    #[arg(long)]
    target_message_size_b: Option<String>,
    /// Peer::payload_depth_limit, or `default`.
    #[arg(long)]
    payload_depth_limit: Option<String>,
    /// Peer::session_deadline, such as `60s`, or `none`.
    #[arg(long)]
    session_deadline: Option<String>,
    /// Link rate per direction: `100Mbit`, `uncapped`, or `A/B`.
    #[arg(long)]
    rate: Option<String>,
    /// Link round-trip time, such as `10ms`.
    #[arg(long)]
    rtt: Option<String>,
    /// `warm` (the default) or `fresh` TCP connections.
    #[arg(long)]
    connection: Option<String>,
    /// `on` (the default) to run each session over TLS 1.3, `off` for plain
    /// TCP.
    #[arg(long)]
    tls: Option<String>,
}

impl GridArgs {
    /// The axis flags given, in [`AXES`] order.
    fn axes(&self) -> Vec<(String, Vec<String>)> {
        let flags: BTreeMap<&str, &Option<String>> = [
            ("shared", &self.shared),
            ("insert_a", &self.insert_a),
            ("insert_b", &self.insert_b),
            ("insert", &self.insert),
            ("redact_a", &self.redact_a),
            ("redact_b", &self.redact_b),
            ("redact", &self.redact),
            ("size_mean", &self.size_mean),
            ("size_cv", &self.size_cv),
            ("max_message_size", &self.max_message_size),
            ("budget", &self.budget),
            ("budget_a", &self.budget_a),
            ("budget_b", &self.budget_b),
            ("target_message_size", &self.target_message_size),
            ("target_message_size_a", &self.target_message_size_a),
            ("target_message_size_b", &self.target_message_size_b),
            ("payload_depth_limit", &self.payload_depth_limit),
            ("session_deadline", &self.session_deadline),
            ("rate", &self.rate),
            ("rtt", &self.rtt),
            ("connection", &self.connection),
            ("tls", &self.tls),
        ]
        .into_iter()
        .collect();
        AXES.iter()
            .filter_map(|(name, _)| {
                let value = flags.get(name)?.as_ref()?;
                Some((
                    name.to_string(),
                    value.split(',').map(|v| v.trim().to_string()).collect(),
                ))
            })
            .collect()
    }

    /// The grid these arguments describe, and its repetitions.
    fn grid(&self) -> Result<(Grid, u32)> {
        let axes = self.axes();
        let grid = match &self.grid {
            Some(_) if !axes.is_empty() => bail!(
                "set axes either in the grid file (--grid) or with flags, not both: the flags given were {}",
                axes.iter()
                    .map(|(n, _)| format!("--{}", n.replace('_', "-")))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Some(path) => Grid::parse(
                &fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?,
            )
            .with_context(|| format!("in {}", path.display()))?,
            None if axes.is_empty() => bail!(
                "say what to measure: a grid file with --grid, such as the quick grid in the tool's crate \
                 (`--grid grids/quick.toml`), or axes with flags such as --shared and --rtt"
            ),
            None => Grid {
                groups: vec![Group {
                    name: "command line".into(),
                    axes,
                    ..Group::default()
                }],
                ..Grid::default()
            },
        };
        let repetitions = self
            .repetitions
            .or(grid.repetitions)
            .unwrap_or(DEFAULT_REPETITIONS);
        ensure!(repetitions > 0, "--repetitions must be at least 1");
        grid.points()?;
        Ok((grid, repetitions))
    }
}

/// `duplex`'s arguments.
#[derive(Args, Debug)]
struct DuplexArgs {
    /// The grid; its link and connection axes do not apply.
    #[command(flatten)]
    grid: GridArgs,
    /// Bytes each direction of the duplex stream holds.
    #[arg(long, default_value_t = DUPLEX_CAPACITY)]
    capacity: usize,
    /// Run a debug build anyway (its timings say nothing about production).
    #[arg(long)]
    allow_debug_build: bool,
}

/// The bytes each direction of `duplex`'s stream holds by default: on the order
/// of a socket's default send buffer.
const DUPLEX_CAPACITY: usize = 1 << 20;

/// Measures each point's session in this process ([`duplex`]).
fn run_duplex(args: &DuplexArgs) -> Result<()> {
    run::checked_environment(args.allow_debug_build)?;
    let (grid, repetitions) = args.grid.grid()?;
    duplex::run(&grid.points()?, repetitions, args.capacity)
}

/// `run`'s arguments.
#[derive(Args, Debug)]
struct RunArgs {
    /// The grid.
    #[command(flatten)]
    grid: GridArgs,
    /// Finish the run cut short in this results directory, running every
    /// session it has no row for with the run's own grid, shard, job count, and
    /// calibration.
    #[arg(long, value_name = "DIR", conflicts_with_all = ["GridArgs", "out", "jobs", "prepare_jobs", "prepare_memory", "recalibrate", "shard"])]
    resume: Option<PathBuf>,
    /// The results directory to create (default `netperf-<unix time>`).
    #[arg(long)]
    out: Option<PathBuf>,
    /// Time this many sessions at once instead of calibrating.
    #[arg(long)]
    jobs: Option<usize>,
    /// Prepare this many sessions at once, instead of as many as keep this
    /// host's processors busy by calibration's measure (or one per two
    /// processors without calibration).
    #[arg(long, value_name = "N")]
    prepare_jobs: Option<usize>,
    /// Let prepared sessions waiting for their timed session hold at most this
    /// much memory by the memory model's prediction, such as `64GiB` (default
    /// half of physical memory).
    #[arg(long, value_name = "SIZE")]
    prepare_memory: Option<String>,
    /// Calibrate again even if a calibration for this host and grid is cached.
    #[arg(long)]
    recalibrate: bool,
    /// Run only shard k of n, a deterministic share of the sessions, such as
    /// `2/4`.
    #[arg(long)]
    shard: Option<Shard>,
    /// Measure with a debug build anyway (its timings mean nothing).
    #[arg(long)]
    allow_debug_build: bool,
}

/// `calibrate`'s arguments.
#[derive(Args, Debug)]
struct CalibrateArgs {
    /// The grid whose heaviest session and fastest link to probe.
    #[command(flatten)]
    grid: GridArgs,
    /// Calibrate again even if a calibration is cached.
    #[arg(long)]
    recalibrate: bool,
    /// Where to write calibration.json besides the cache.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Measure with a debug build anyway.
    #[arg(long)]
    allow_debug_build: bool,
}

/// `connect`'s arguments.
#[derive(Args, Debug)]
struct ConnectArgs {
    /// The host running `rumors-netperf serve`, as HOST or HOST:PORT.
    server: String,
    /// The grid.
    #[command(flatten)]
    grid: GridArgs,
    /// The results directory to create (default `netperf-<unix time>`).
    #[arg(long)]
    out: Option<PathBuf>,
    /// Measure with a debug build anyway.
    #[arg(long)]
    allow_debug_build: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Doctor => {
            doctor::doctor();
            Ok(())
        }
        Command::Run(args) => run_emulation(&args),
        Command::Calibrate(args) => calibrate_only(&args),
        Command::Duplex(args) => run_duplex(&args),
        // The server's peer processes measure their side of each session,
        // so the server refuses the builds `connect` refuses.
        Command::Serve { listen } => run::refuse_test_internals()
            .and_then(|()| current_exe())
            .and_then(|program| pair::serve(&program, listen, |line| eprintln!("{line}"))),
        Command::Connect(args) => run_pair(&args),
        Command::Check { dirs, out } => {
            let out = out.unwrap_or_else(|| dirs[0].join("check"));
            check_results(&dirs, &out)
        }
        Command::Report { dirs, out } => {
            let out = out.unwrap_or_else(|| dirs[0].clone());
            report::render(&dirs, &out).map(|path| println!("{}", path.display()))
        }
        Command::Peer => {
            peer::serve_one(&mut std::io::stdin().lock(), &mut std::io::stdout().lock())
        }
        Command::ProbePeer => {
            probe::serve_one(&mut std::io::stdin().lock(), &mut std::io::stdout().lock())
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

/// This executable's path, which peer processes run.
fn current_exe() -> Result<PathBuf> {
    std::env::current_exe().context("finding this executable to start peer processes")
}

/// Re-runs the command inside the emulation sandbox unless already in one,
/// exiting with its status; returns only inside the sandbox.
fn enter_sandbox() -> Result<()> {
    ensure!(
        cfg!(target_os = "linux"),
        "emulation mode needs Linux network namespaces; on this platform use pair mode (`serve` and `connect`)"
    );
    if netns::in_sandbox() {
        return Ok(());
    }
    match netns::sandbox_command(Path::new("true"), &[]).output() {
        Ok(output) if output.status.success() => {}
        Ok(output) => bail!(
            "could not create the emulation sandbox ({}); `rumors-netperf doctor` says what to change",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(error) => bail!("could not run `unshare` ({error}); install util-linux"),
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let status = netns::sandbox_command(&current_exe()?, &args)
        .status()
        .context("re-running inside the emulation sandbox")?;
    std::process::exit(status.code().unwrap_or(1));
}

/// The emulated sessions of a grid, with raised-buffer reruns, sharded, and the
/// plan they make.
fn emulated_sessions(
    args: &GridArgs,
    environment: &env::Environment,
    shard: Option<Shard>,
) -> Result<(Vec<grid::Session>, Plan)> {
    let (grid, repetitions) = args.grid()?;
    let (points, skipped) = grid.plan()?;
    let window = environment
        .tcp
        .usable_window
        .map(|w| (w, environment.tcp.payload_share));
    let sessions = run::shard(grid::sessions(&points, repetitions, window), shard);
    ensure!(
        !sessions.is_empty(),
        "the grid has no sessions{}",
        if shard.is_some() {
            " in this shard"
        } else {
            ""
        }
    );
    let plan = Plan::new(repetitions, shard, points, skipped, &sessions);
    Ok((sessions, plan))
}

/// The calibration for `sessions`, from the cache or measured now.
fn calibration_for(
    sessions: &[grid::Session],
    environment: &env::Environment,
    recalibrate: bool,
) -> Result<(Calibration, String)> {
    let (noise, fidelity) = calibrate::probe_points(sessions)?;
    let cache = calibrate::cache_path(&environment.fingerprint(), &noise, &fidelity);
    if !recalibrate && let Some(cached) = cache.as_deref().and_then(calibrate::load) {
        eprintln!(
            "using the cached calibration: {} jobs ({})",
            cached.jobs, cached.verdict
        );
        return Ok((cached, "calibration, cached".into()));
    }
    let program = current_exe()?;
    let setup = calibrate::Setup {
        program: &program,
        window: environment.tcp.usable_window.unwrap_or(4 << 20),
        payload_share: environment.tcp.payload_share,
        max_jobs: (environment.logical_cpus / 2).max(1),
        logical_cpus: environment.logical_cpus,
        memory_bytes: memory_bound(environment, None)?,
    };
    eprintln!(
        "calibrating: noise probe {} on an uncapped link, fidelity probe on {}",
        noise.replicas, fidelity.link
    );
    let began = Instant::now();
    let calibration = calibrate::calibrate(&setup, environment, &noise, &fidelity, &mut |step| {
        eprintln!(
            "[{}] calibration: {step}",
            human_seconds(began.elapsed().as_secs_f64())
        );
    })?;
    eprintln!(
        "calibration: {} jobs; {}; {} preparations at once",
        calibration.jobs, calibration.verdict, calibration.preparation.prepare_jobs
    );
    if let Some(path) = &cache {
        calibrate::store(path, &calibration)?;
    }
    Ok((calibration, "calibration".into()))
}

/// `rumors-netperf run`.
fn run_emulation(args: &RunArgs) -> Result<()> {
    if args.resume.is_none() {
        args.grid.grid()?;
    }
    enter_sandbox()?;
    let started = run::now();
    let environment = run::checked_environment(args.allow_debug_build)?;
    let (out, part) = match &args.resume {
        Some(dir) => {
            let part = Part::resume(dir, &environment)?;
            let resumption = part
                .resumption()
                .expect("a resumed part records its resumption");
            let limits = part.limits();
            ensure!(
                emulate::descriptor_window(limits.prepare_jobs) >= limits.window,
                "this process's file descriptor limit no longer allows the run's window of {} \
                 prepared sessions; raise `ulimit -n` to resume it",
                limits.window
            );
            eprintln!(
                "resuming the run in {}: {} sessions recorded, {} to run, timing {} and preparing up \
                 to {} at once",
                dir.display(),
                resumption.found,
                resumption.ran,
                limits.timed_jobs,
                limits.prepare_jobs
            );
            (dir.clone(), part)
        }
        None => {
            let (sessions, plan) = emulated_sessions(&args.grid, &environment, args.shard)?;
            let (calibration, jobs, jobs_source) = match args.jobs {
                Some(jobs) => {
                    ensure!(jobs > 0, "--jobs must be at least 1");
                    (None, jobs, "--jobs".to_string())
                }
                None => {
                    let (calibration, source) =
                        calibration_for(&sessions, &environment, args.recalibrate)?;
                    let jobs = calibration.jobs;
                    (Some(calibration), jobs, source)
                }
            };
            let preparation = preparation(args, &environment, calibration.as_ref())?;
            let sentinels = calibration.as_ref().map_or_else(Vec::new, |calibration| {
                run::sentinels(calibration, run::sentinel_count(&sessions, calibration))
            });
            let record = RunRecord {
                mode: run::EMULATION.into(),
                command: std::env::args().collect(),
                started,
                jobs,
                jobs_source,
                preparation: Some(preparation),
                shard: args.shard,
                sentinels: sentinels.len(),
                sessions: sessions.len() + sentinels.len(),
                sentinel_bound_seconds: calibration.as_ref().map(Calibration::sentinel_bound),
                usable_window: environment.tcp.usable_window,
                ..RunRecord::default()
            };
            let out = args.out.clone().unwrap_or_else(run::default_out);
            let skipped = plan.skipped.len();
            let part = Part::begin(
                &out,
                &environment,
                &plan,
                calibration,
                run::order(sessions, sentinels),
                record,
            )?;
            let limits = part.limits();
            eprintln!(
                "running {} sessions ({}), timing {} and preparing up to {} at once within {} of \
                 prepared sessions: {}; {skipped} points skipped for their wire time (plan.json lists \
                 them); results in {}",
                part.sessions().len(),
                format_groups(part.sessions()),
                limits.timed_jobs,
                limits.prepare_jobs,
                human_bytes(limits.memory_bytes as f64),
                part.calibration().map_or_else(
                    || "no calibration, so no prediction of its wall time".to_string(),
                    |calibration| describe(&predict_run(
                        part.sessions(),
                        calibration,
                        &environment,
                        limits
                    )),
                ),
                out.display()
            );
            (out, part)
        }
    };
    let program = current_exe()?;
    let executor = emulate::Emulation {
        program: &program,
        window: environment.tcp.usable_window.unwrap_or(4 << 20),
        payload_share: environment.tcp.payload_share,
    };
    let record = part.run(&executor)?;
    let report = report::render(std::slice::from_ref(&out), &out)?;
    eprintln!(
        "done in {}: {} sessions, {} failed; report: {}",
        human_seconds(record.seconds),
        record.sessions,
        record.failures,
        report.display()
    );
    Ok(())
}

/// `rumors-netperf check`: prints every failure, or a summary when there is
/// none.
fn check_results(dirs: &[PathBuf], out: &Path) -> Result<()> {
    let outcome = check::check(dirs, out);
    for failure in &outcome.failures {
        println!("FAIL {failure}");
    }
    ensure!(
        outcome.failures.is_empty(),
        "{} of the run's checks failed",
        outcome.failures.len()
    );
    println!(
        "passed: {} sessions recorded, {} points skipped with their reasons, {} raised-limit sessions; report: {}",
        outcome.sessions,
        outcome.skipped,
        outcome.raised,
        out.join("report.md").display()
    );
    Ok(())
}

/// Sessions per group, for the plan's one-line summary.
fn format_groups(sessions: &[grid::Session]) -> String {
    grid::by_group(sessions)
        .into_iter()
        .map(|(group, count)| format!("{group}: {count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `rumors-netperf calibrate`.
fn calibrate_only(args: &CalibrateArgs) -> Result<()> {
    args.grid.grid()?;
    enter_sandbox()?;
    let environment = run::checked_environment(args.allow_debug_build)?;
    let (sessions, _) = emulated_sessions(&args.grid, &environment, None)?;
    let (calibration, _) = calibration_for(&sessions, &environment, args.recalibrate)?;
    if let Some(out) = &args.out {
        fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;
        run::write_json(out, "calibration.json", &calibration)?;
        run::write_json(out, "environment.json", &environment)?;
    }
    let preparation = Preparation {
        jobs: calibration.preparation.prepare_jobs,
        jobs_source: "calibration".into(),
        memory_bytes: memory_bound(&environment, None)?,
        window: emulate::descriptor_window(calibration.preparation.prepare_jobs),
    };
    let limits = RunRecord {
        jobs: calibration.jobs,
        preparation: Some(preparation),
        ..RunRecord::default()
    }
    .limits()
    .expect("the record holds its preparation");
    let sentinels = run::sentinels(&calibration, run::sentinel_count(&sessions, &calibration));
    let order = run::order(sessions, sentinels);
    println!("jobs: {}", calibration.jobs);
    println!("verdict: {}", calibration.verdict);
    println!("preparations at once: {}", limits.prepare_jobs);
    println!(
        "a run of this grid on this host: {}",
        describe(&predict_run(&order, &calibration, &environment, limits))
    );
    Ok(())
}

/// The memory a run's prepared sessions may hold: `flag`, a size such as
/// `64GiB`, or half of physical memory.
///
/// # Errors
///
/// If the flag does not parse as a size, or neither it nor the host's physical
/// memory is known.
fn memory_bound(environment: &env::Environment, flag: Option<&str>) -> Result<u64> {
    match flag {
        Some(text) => units::parse_bytes(text).context("--prepare-memory"),
        None => environment.memory_bytes.map(|bytes| bytes / 2).context(
            "this host does not report its physical memory; bound prepared sessions with \
             --prepare-memory",
        ),
    }
}

/// How a fresh run prepares its sessions: the flags given, else the
/// calibration's preparation count and half of physical memory, within the
/// window the descriptor limit allows.
fn preparation(
    args: &RunArgs,
    environment: &env::Environment,
    calibration: Option<&Calibration>,
) -> Result<Preparation> {
    let (jobs, jobs_source) = match (args.prepare_jobs, calibration) {
        (Some(jobs), _) => {
            ensure!(jobs > 0, "--prepare-jobs must be at least 1");
            (jobs, "--prepare-jobs".to_string())
        }
        (None, Some(calibration)) => (calibration.preparation.prepare_jobs, "calibration".into()),
        (None, None) => (
            (environment.logical_cpus / 2).max(1),
            "one per two processors, without calibration".into(),
        ),
    };
    Ok(Preparation {
        jobs,
        jobs_source,
        memory_bytes: memory_bound(environment, args.prepare_memory.as_deref())?,
        window: emulate::descriptor_window(jobs),
    })
}

/// The schedule of `sessions`, in their order, within `limits`, played through
/// with `calibration`'s predictions on a host whose usable TCP window and
/// payload share `environment` records.
///
/// A timed session lasts its session and, on a warm connection, the setup and
/// warm-up before the session's clock starts.
fn predict_run(
    sessions: &[grid::Session],
    calibration: &Calibration,
    environment: &env::Environment,
    limits: schedule::Limits,
) -> schedule::Simulation {
    let costs = &calibration.costs;
    let jobs = run::jobs(sessions, costs, &calibration.memory);
    let host_window = environment.tcp.usable_window.unwrap_or(4 << 20);
    let timed: Vec<f64> = sessions
        .iter()
        .map(|s| {
            let window = emulate::window(&s.point, host_window, environment.tcp.payload_share);
            costs.session_seconds(&s.point)
                + costs.warm_seconds(&s.point, emulate::warm_bytes(&s.point, window), window)
        })
        .collect();
    schedule::simulate(jobs, &timed, limits)
}

/// A simulated run in words: its wall time, and how it divides.
fn describe(simulation: &schedule::Simulation) -> String {
    format!(
        "predicted about {} of wall time: {} preparing in {} windows (all workers busy would take {}), \
         {} timing",
        human_seconds(simulation.wall_seconds),
        human_seconds(simulation.preparing_seconds),
        simulation.windows,
        human_seconds(simulation.ideal_preparing_seconds),
        human_seconds(simulation.timing_seconds)
    )
}

/// Probes the network to the server: one small transfer to size the probes,
/// then three probes of about two seconds each.
fn probe_network(program: &Path, server: &pair::Server) -> Result<Network> {
    const PINGS: u32 = 20;
    const PROBES: usize = 3;
    let sizing = pair::probe(program, server, 1 << 20, 5)?;
    let bulk = ((sizing.goodput_bytes_per_second * 2.0) as u64).clamp(1 << 20, 1 << 30);
    let mut probes = Vec::with_capacity(PROBES);
    for _ in 0..PROBES {
        probes.push(pair::probe(program, server, bulk, PINGS)?);
    }
    Ok(Network {
        server: Some(server.environment()?),
        probes,
    })
}

/// `rumors-netperf connect`.
fn run_pair(args: &ConnectArgs) -> Result<()> {
    let started = run::now();
    let environment = run::checked_environment(args.allow_debug_build)?;
    let server = pair::Server::resolve(&args.server)?;
    let (grid, repetitions) = args.grid.grid()?;
    if grid.groups.iter().any(|g| {
        g.axes
            .iter()
            .any(|(axis, _)| axis == "rate" || axis == "rtt")
    }) {
        eprintln!(
            "note: pair mode measures the network between the hosts, so the grid's `rate` and `rtt` axes are ignored"
        );
    }
    let (grid_points, skipped) = grid.plan()?;
    let mut points: Vec<(String, Point)> = Vec::new();
    for (group, point) in grid_points {
        let point = Point {
            link: Link::Network,
            ..point
        };
        if !points
            .iter()
            .any(|(g, p)| *g == group && p.key() == point.key())
        {
            points.push((group, point));
        }
    }
    let program = current_exe()?;
    eprintln!("probing the network to {}", server.address);
    let network = probe_network(&program, &server)?;
    eprintln!(
        "network: round trip {}, goodput {:.1} Mbit/s",
        network.rtt().map_or("-".into(), human_seconds),
        network.goodput().unwrap_or(0.0) * 8e-6
    );
    let sessions = grid::sessions(&points, repetitions, None);
    let plan = Plan::new(repetitions, None, points, skipped, &sessions);
    let out = args.out.clone().unwrap_or_else(run::default_out);
    let count = sessions.len();
    let record = RunRecord {
        mode: run::PAIR.into(),
        command: std::env::args().collect(),
        started,
        jobs: 1,
        jobs_source: "pair mode runs one session at a time".into(),
        preparation: Some(Preparation {
            jobs: 1,
            jobs_source: "pair mode prepares one session at a time".into(),
            memory_bytes: u64::MAX,
            window: 1,
        }),
        sessions: count,
        usable_window: environment.tcp.usable_window,
        ..RunRecord::default()
    };
    let part = Part::begin(
        &out,
        &environment,
        &plan,
        None,
        run::order(sessions, Vec::new()),
        record,
    )?;
    run::write_json(&out, "network.json", &network)?;
    eprintln!(
        "running {count} sessions against {}; results in {}",
        server.address,
        out.display()
    );
    let record = part.run(&pair::Pair {
        program: &program,
        server: &server,
    })?;
    let report = report::render(std::slice::from_ref(&out), &out)?;
    eprintln!(
        "done: {count} sessions, {} failed; report: {}",
        record.failures,
        report.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests;
