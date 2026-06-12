//! A local stress rehearsal for rumormill: drive N real rumormill
//! processes (real binary, real iroh networking) through pseudo-terminals,
//! exactly as N humans at N keyboards would.
//!
//! The run mirrors the demo it rehearses: one seed node comes up first and
//! everyone else bootstraps from its endpoint id, then the room converges,
//! chats, floods, idles, and quits in a storm. Between those phases the
//! harness scrapes each node's terminal (a vt100 model of the live TUI) and
//! asserts the things a demo audience would notice:
//!
//! - **Convergence**: every roster reaches `peers (N)`; one universe id.
//! - **Consistency**: every node settles at the same live-entry count, and
//!   that count is exactly what was sent (N presence + 1 channel + chats).
//! - **Latency**: a chat line reaches all N screens within the probe
//!   timeout, before and after a full-room flood.
//! - **Steady state**: an idle window's session counters give the real
//!   background gossip rate, and failures do not keep climbing.
//! - **Departure**: a mass quit retires cleanly (no wedged processes), and
//!   the survivors' rosters shrink to match.
//!
//! Soft signals (RSS, quit latency, retirement outcome mix) are reported
//! rather than asserted. The process exits nonzero if any hard assertion
//! failed; run it through `just soak`.

mod fleet;
mod scrape;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail};
use clap::Parser;

use crate::fleet::{Fleet, Node};

#[derive(Debug, Parser)]
#[command(name = "rumormill-soak", version, about)]
struct Args {
    /// How many rumormill instances to run.
    #[arg(long, default_value_t = 100)]
    nodes: usize,

    /// Path to the rumormill binary (build it with
    /// `cargo build --release -p rumormill`).
    #[arg(long, default_value = "target/release/rumormill")]
    rumormill: PathBuf,

    /// Pause between spawns during the join storm, in milliseconds.
    #[arg(long, default_value_t = 100)]
    stagger_ms: u64,

    /// How long every roster gets to reach `peers (N)`, in seconds.
    #[arg(long, default_value_t = 240)]
    converge_secs: u64,

    /// How long the live-entry counts get to settle and agree after
    /// convergence, in seconds (must outlive the 15s join-notice TTL).
    #[arg(long, default_value_t = 90)]
    settle_secs: u64,

    /// Chat propagation probes to run before the flood.
    #[arg(long, default_value_t = 5)]
    probes: usize,

    /// How long one probe gets to reach every screen, in seconds.
    #[arg(long, default_value_t = 30)]
    probe_secs: u64,

    /// How long the all-nodes flood gets to fully propagate, in seconds.
    #[arg(long, default_value_t = 120)]
    flood_secs: u64,

    /// Idle window for measuring background session rates, in seconds.
    #[arg(long, default_value_t = 20)]
    quiet_secs: u64,

    /// How long each quit wave gets to exit cleanly, in seconds (retiring
    /// can walk dial timeouts during a quit storm).
    #[arg(long, default_value_t = 180)]
    quit_secs: u64,

    /// Directory for per-node connection-lifecycle traces
    /// (`RUMORMILL_TRACE`); created if missing.
    #[arg(long, default_value = "/tmp/rumormill-soak-trace")]
    trace_dir: PathBuf,
}

/// Spacing between sends during the all-nodes flood: fast enough to be a
/// burst, slow enough that the harness isn't the bottleneck.
const FLOOD_STAGGER: Duration = Duration::from_millis(5);

/// Spacing between Escape presses during a quit wave.
const QUIT_STAGGER: Duration = Duration::from_millis(50);

/// How often polls re-scrape screens.
const POLL: Duration = Duration::from_millis(500);

/// How often probe polls re-scrape (latency measurements want finer grain).
const PROBE_POLL: Duration = Duration::from_millis(200);

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("soak: fatal: {e:#}");
            ExitCode::FAILURE
        }
    }
}

/// Findings accumulated across phases: `failures` flip the exit code,
/// `warnings` and `notes` only shape the report.
#[derive(Default)]
struct Report {
    failures: Vec<String>,
    warnings: Vec<String>,
    notes: Vec<String>,
}

/// Elapsed-stamped progress lines.
struct Log {
    t0: Instant,
}

impl Log {
    fn say(&self, msg: &str) {
        println!("[{:7.1}s] {msg}", self.t0.elapsed().as_secs_f64());
    }
}

fn run(args: &Args) -> anyhow::Result<bool> {
    if args.nodes < 2 {
        bail!("--nodes must be at least 2");
    }
    if !args.rumormill.is_file() {
        bail!(
            "{} not found: build it with `cargo build --release -p rumormill` (or run `just soak`)",
            args.rumormill.display()
        );
    }
    // The PTY spawn resolves bare names through PATH, not the cwd; an
    // absolute path sidesteps that entirely.
    let rumormill = args
        .rumormill
        .canonicalize()
        .context("resolving the rumormill binary path")?;
    // PTY masters, readers, and writers cost several fds per node; make
    // sure the soft limit isn't the thing the soak ends up testing.
    let _ = rlimit::increase_nofile_limit(args.nodes as u64 * 8 + 256);

    let log = Log { t0: Instant::now() };
    let mut report = Report::default();
    let mut fleet = Fleet::default();
    let n = args.nodes;
    let nonce = std::process::id();

    // Fresh traces per run: stale logs from a previous run would mislead.
    let _ = std::fs::remove_dir_all(&args.trace_dir);
    std::fs::create_dir_all(&args.trace_dir).context("creating the trace dir")?;
    let trace_dir = Some(args.trace_dir.as_path());
    log.say(&format!("node traces: {}", args.trace_dir.display()));

    // ── seed ────────────────────────────────────────────────────────────
    log.say("spawning the seed node n000");
    fleet
        .nodes
        .push(Node::spawn(&rumormill, "n000", None, trace_dir)?);
    let seed_id = await_value(Duration::from_secs(30), POLL, || {
        scrape::endpoint_id(&fleet.nodes[0].raw())
    })
    .with_context(|| {
        format!(
            "the seed never announced its endpoint id; transcript:\n{}",
            fleet.nodes[0].raw()
        )
    })?;
    log.say(&format!("seed endpoint id: {seed_id}"));

    // The seed has no `--peer`, so it boots into the connect dialog —
    // which swallows keystrokes (a chat line types into the dialog buffer)
    // and remaps Esc (closes the dialog instead of quitting). Dismiss it
    // once, verified, so the seed behaves like every other node.
    await_value(Duration::from_secs(30), POLL, || {
        fleet.nodes[0]
            .screen()
            .contains("connect to a peer")
            .then_some(())
    })
    .context("the seed never showed the connect dialog")?;
    fleet.nodes[0].send_esc();
    await_value(Duration::from_secs(10), POLL, || {
        let gone = !fleet.nodes[0].screen().contains("connect to a peer");
        gone.then_some(())
    })
    .context("the seed's connect dialog did not dismiss on Esc")?;
    log.say("seed dialog dismissed");

    // ── join storm ──────────────────────────────────────────────────────
    log.say(&format!(
        "spawning {} joiners, all bootstrapping from the seed, {}ms apart",
        n - 1,
        args.stagger_ms
    ));
    for i in 1..n {
        std::thread::sleep(Duration::from_millis(args.stagger_ms));
        fleet.nodes.push(Node::spawn(
            &rumormill,
            &format!("n{i:03}"),
            Some(&seed_id),
            trace_dir,
        )?);
    }

    // ── convergence ─────────────────────────────────────────────────────
    log.say(&format!("waiting for `peers ({n})` on every roster"));
    let all: Vec<usize> = (0..n).collect();
    let latencies = await_each(
        &fleet.nodes,
        &all,
        Duration::from_secs(args.converge_secs),
        POLL,
        |node| scrape::roster_count(&node.screen()) == Some(n),
    );
    let converged = latencies.iter().flatten().count();
    if converged == n {
        let stats = dist(&latencies.iter().flatten().copied().collect::<Vec<_>>());
        log.say(&format!("all {n} rosters converged: {stats}"));
        report.notes.push(format!("roster convergence: {stats}"));
    } else {
        report.failures.push(format!(
            "convergence: only {converged}/{n} rosters reached peers ({n}) within {}s; \
             roster sizes: {}",
            args.converge_secs,
            roster_histogram(&fleet.nodes)
        ));
        diagnose_stragglers(&log, &mut fleet.nodes, &latencies, &all);
        // Every later phase assumes a formed room; without one they are
        // minutes of doomed timeouts. Report and stop here (the fleet's
        // drop kills the survivors).
        return Ok(print_report(&report, n));
    }

    // ── settle: one universe, one live count ────────────────────────────
    let mut sent_chats: u64 = 0;
    let expected_idle = n as u64 + 1; // N presence + #general; notices expire
    log.say(&format!(
        "waiting for every header to settle at {expected_idle} live in one universe"
    ));
    let settle_t0 = Instant::now();
    let settled = await_consistent(
        &fleet.nodes,
        Duration::from_secs(args.settle_secs),
        |stats| stats.live == expected_idle,
    );
    match settled {
        Some(net) => {
            log.say(&format!(
                "settled in {:.1}s: every node at {expected_idle} live in net {net}",
                settle_t0.elapsed().as_secs_f64()
            ));
            report.notes.push(format!(
                "settle after join: {:.1}s to {expected_idle} live everywhere, one universe ({net})",
                settle_t0.elapsed().as_secs_f64()
            ));
        }
        None => report.failures.push(format!(
            "settle: nodes did not agree on {expected_idle} live entries in one universe \
             within {}s; live counts: {}",
            args.settle_secs,
            live_histogram(&fleet.nodes)
        )),
    }

    // ── chat probes ─────────────────────────────────────────────────────
    let mut first_chat: Option<Instant> = None;
    for round in 0..args.probes {
        let sender = (round * 37 + 1) % n;
        let body = format!("probe-{nonce}-r{round}");
        probe(
            &log,
            &mut fleet,
            sender,
            &body,
            Duration::from_secs(args.probe_secs),
            &mut report,
        );
        first_chat.get_or_insert_with(Instant::now);
        sent_chats += 1;
    }

    // ── flood: everyone talks at once ───────────────────────────────────
    log.say(&format!("flood: all {n} nodes send one message each"));
    let flood_t0 = Instant::now();
    for i in 0..n {
        let body = format!("flood-{nonce}-{i:03}");
        fleet.nodes[i].send_line(&body);
        std::thread::sleep(FLOOD_STAGGER);
    }
    first_chat.get_or_insert_with(Instant::now);
    sent_chats += n as u64;
    let expected_flooded = expected_idle + sent_chats;
    let flooded = await_consistent(
        &fleet.nodes,
        Duration::from_secs(args.flood_secs),
        |stats| stats.live == expected_flooded,
    );
    match flooded {
        Some(_) => {
            log.say(&format!(
                "flood fully propagated in {:.1}s: every node at {expected_flooded} live",
                flood_t0.elapsed().as_secs_f64()
            ));
            report.notes.push(format!(
                "flood ({n} concurrent messages): {:.1}s to {expected_flooded} live everywhere",
                flood_t0.elapsed().as_secs_f64()
            ));
        }
        None => report.failures.push(format!(
            "flood: nodes did not reach {expected_flooded} live within {}s; live counts: {}",
            args.flood_secs,
            live_histogram(&fleet.nodes)
        )),
    }

    // One more probe with the room full: latency after the burst. (Its
    // body joins the live set too, but nothing asserts a live count after
    // this point, so `sent_chats` stops here.)
    let body = format!("probe-{nonce}-postflood");
    probe(
        &log,
        &mut fleet,
        n / 2,
        &body,
        Duration::from_secs(args.probe_secs),
        &mut report,
    );

    // ── quiet window: background rates and RSS ──────────────────────────
    log.say(&format!(
        "quiet window: idling {}s to measure background gossip",
        args.quiet_secs
    ));
    let before = session_totals(&fleet.nodes);
    std::thread::sleep(Duration::from_secs(args.quiet_secs));
    let after = session_totals(&fleet.nodes);
    let d_ok = after.0.saturating_sub(before.0);
    let d_failed = after.1.saturating_sub(before.1);
    report.notes.push(format!(
        "idle gossip: {d_ok} sessions over {}s room-wide ({:.0}/s); failures grew by {d_failed}",
        args.quiet_secs,
        d_ok as f64 / args.quiet_secs as f64
    ));
    if d_failed > n as u64 {
        report.warnings.push(format!(
            "session failures grew by {d_failed} during an idle {}s window: \
             something is redialing or flapping at steady state",
            args.quiet_secs
        ));
    }
    let rss = rss_kb(&fleet.nodes);
    if !rss.is_empty() {
        report.notes.push(format!(
            "RSS per node: min {} MB, p50 {} MB, max {} MB",
            rss.first().expect("nonempty") / 1024,
            rss[rss.len() / 2] / 1024,
            rss.last().expect("nonempty") / 1024
        ));
    }
    report.notes.push(format!(
        "final session counters room-wide: {}✓ {}✗",
        after.0, after.1
    ));
    if let Some(first) = first_chat
        && first.elapsed() > Duration::from_secs(240)
    {
        report.warnings.push(
            "the run outlived most of the 5-minute chat TTL: live-count assertions near the \
             end may have raced expiry"
                .into(),
        );
    }

    // ── quit: a storm, the rest, then the seed ──────────────────────────
    let storm: Vec<usize> = (1..n).step_by(2).collect();
    let calm: Vec<usize> = (2..n).step_by(2).collect();
    log.say(&format!(
        "quit storm: {} nodes press Esc nearly at once",
        storm.len()
    ));
    quit_wave(&log, &mut fleet, &storm, args, &mut report);

    let survivors = calm.len() + 1;
    log.say(&format!(
        "waiting for survivor rosters to shrink to peers ({survivors})"
    ));
    let shrink: Vec<usize> = std::iter::once(0).chain(calm.iter().copied()).collect();
    let shrunk = await_each(
        &fleet.nodes,
        &shrink,
        Duration::from_secs(60),
        POLL,
        |node| scrape::roster_count(&node.screen()) == Some(survivors),
    );
    if shrunk.iter().all(Option::is_some) {
        let stats = dist(&shrunk.iter().flatten().copied().collect::<Vec<_>>());
        report
            .notes
            .push(format!("survivor rosters shrank after the storm: {stats}"));
    } else {
        let stuck: Vec<&Node> = shrink
            .iter()
            .zip(&shrunk)
            .filter(|(_, lat)| lat.is_none())
            .map(|(&i, _)| &fleet.nodes[i])
            .collect();
        let histogram: BTreeMap<Option<usize>, usize> =
            stuck.iter().fold(BTreeMap::new(), |mut histogram, node| {
                *histogram
                    .entry(scrape::roster_count(&node.screen()))
                    .or_default() += 1;
                histogram
            });
        // Attribution data: a stuck node whose *live count* dropped and
        // whose screen shows a goodbye notice received the retire delta
        // (display problem); one with neither never got it (set problem).
        for node in stuck.iter().take(5) {
            let screen = node.screen();
            let live = scrape::header_stats(&screen).map(|s| s.live);
            let goodbye = screen.contains(" left");
            log.say(&format!(
                "stuck survivor {}: live={live:?} goodbye-visible={goodbye}",
                node.name
            ));
        }
        report.warnings.push(format!(
            "survivor rosters had not all shrunk to peers ({survivors}) within 60s of the quit \
             storm (stuck: {histogram:?}): leaked retirements age out only via the staleness \
             sweep (~{}s + beat age)",
            rumormill_presence_stale_secs()
        ));
    }

    log.say(&format!("quitting the remaining {} nodes", calm.len()));
    quit_wave(&log, &mut fleet, &calm, args, &mut report);
    log.say("quitting the seed last");
    quit_wave(&log, &mut fleet, &[0], args, &mut report);

    // ── report ──────────────────────────────────────────────────────────
    Ok(print_report(&report, n))
}

/// Print the accumulated findings and return whether the run passed.
fn print_report(report: &Report, n: usize) -> bool {
    println!("\n══ soak report: {n} nodes ══");
    for note in &report.notes {
        println!("  · {note}");
    }
    for warning in &report.warnings {
        println!("  ⚠ {warning}");
    }
    for failure in &report.failures {
        println!("  ✘ {failure}");
    }
    let passed = report.failures.is_empty();
    println!(
        "verdict: {}",
        if passed {
            "PASS".to_string()
        } else {
            format!("FAIL ({} hard findings)", report.failures.len())
        }
    );
    passed
}

/// The PRESENCE_STALE the survivors' sweep runs on, for the warning text.
/// (Kept here as a number, not a dependency: the soak drives the binary,
/// not the crate.)
fn rumormill_presence_stale_secs() -> u64 {
    90
}

/// Send one probe message and wait until every screen shows it.
fn probe(
    log: &Log,
    fleet: &mut Fleet,
    sender: usize,
    body: &str,
    timeout: Duration,
    report: &mut Report,
) {
    fleet.nodes[sender].send_line(body);
    let all: Vec<usize> = (0..fleet.nodes.len()).collect();
    let latencies = await_each(&fleet.nodes, &all, timeout, PROBE_POLL, |node| {
        node.screen().contains(body)
    });
    let seen: Vec<Duration> = latencies.iter().flatten().copied().collect();
    if seen.len() == fleet.nodes.len() {
        let stats = dist(&seen);
        log.say(&format!("probe `{body}` reached all screens: {stats}"));
        report.notes.push(format!("probe `{body}`: {stats}"));
    } else {
        let missing: Vec<&str> = latencies
            .iter()
            .zip(&fleet.nodes)
            .filter(|(lat, _)| lat.is_none())
            .map(|(_, node)| node.name.as_str())
            .take(10)
            .collect();
        report.failures.push(format!(
            "probe `{body}`: {}/{} screens never showed it within {}s (first missing: {})",
            fleet.nodes.len() - seen.len(),
            fleet.nodes.len(),
            timeout.as_secs(),
            missing.join(", ")
        ));
    }
}

/// Press Esc on each listed node (staggered), wait for the wave to exit,
/// and account exits, departures, and stragglers.
fn quit_wave(log: &Log, fleet: &mut Fleet, idxs: &[usize], args: &Args, report: &mut Report) {
    if idxs.is_empty() {
        return;
    }
    let esc_t0 = Instant::now();
    for &i in idxs {
        fleet.nodes[i].send_esc();
        std::thread::sleep(QUIT_STAGGER);
    }
    let deadline = esc_t0 + Duration::from_secs(args.quit_secs);
    let mut exited: Vec<Option<Duration>> = vec![None; idxs.len()];
    loop {
        for (slot, &i) in exited.iter_mut().zip(idxs) {
            if slot.is_none() && fleet.nodes[i].poll_exit().is_some() {
                *slot = Some(esc_t0.elapsed());
            }
        }
        if exited.iter().all(Option::is_some) || Instant::now() > deadline {
            break;
        }
        std::thread::sleep(POLL);
    }

    let mut departures: BTreeMap<String, usize> = BTreeMap::new();
    let mut dirty_exits = 0;
    let mut wedged: Vec<String> = Vec::new();
    for (slot, &i) in exited.iter().zip(idxs) {
        let node = &mut fleet.nodes[i];
        match slot {
            Some(_) => {
                if !node.poll_exit().expect("exit observed").success() {
                    dirty_exits += 1;
                }
                *departures
                    .entry(format!("{:?}", scrape::departure(&node.raw())))
                    .or_default() += 1;
            }
            None => {
                wedged.push(node.name.clone());
                node.kill();
            }
        }
    }
    let clean: Vec<Duration> = exited.iter().flatten().copied().collect();
    let summary = departures
        .iter()
        .map(|(k, v)| format!("{v} {k}"))
        .collect::<Vec<_>>()
        .join(", ");
    log.say(&format!(
        "quit wave of {}: {} exited ({}), departures: {summary}",
        idxs.len(),
        clean.len(),
        dist(&clean)
    ));
    report
        .notes
        .push(format!("quit wave of {}: {summary}", idxs.len()));
    if !wedged.is_empty() {
        report.failures.push(format!(
            "{} nodes never exited within {}s of Esc and were killed: {}",
            wedged.len(),
            args.quit_secs,
            wedged.join(", ")
        ));
    }
    if dirty_exits > 0 {
        report.failures.push(format!(
            "{dirty_exits} nodes exited nonzero during the wave"
        ));
    }
}

/// Poll until `f` yields a value or `timeout` passes.
fn await_value<T>(timeout: Duration, poll: Duration, f: impl Fn() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(value) = f() {
            return Some(value);
        }
        if Instant::now() > deadline {
            return None;
        }
        std::thread::sleep(poll);
    }
}

/// Poll until `pred` holds for every listed node or `timeout` passes;
/// returns each node's time-to-satisfied (`None`: never, within timeout).
/// A satisfied node stays satisfied: the first observation is recorded.
fn await_each(
    nodes: &[Node],
    idxs: &[usize],
    timeout: Duration,
    poll: Duration,
    pred: impl Fn(&Node) -> bool,
) -> Vec<Option<Duration>> {
    let t0 = Instant::now();
    let deadline = t0 + timeout;
    let mut done: Vec<Option<Duration>> = vec![None; idxs.len()];
    loop {
        for (slot, &i) in done.iter_mut().zip(idxs) {
            if slot.is_none() && pred(&nodes[i]) {
                *slot = Some(t0.elapsed());
            }
        }
        if done.iter().all(Option::is_some) || Instant::now() > deadline {
            return done;
        }
        std::thread::sleep(poll);
    }
}

/// Poll until every node's header parses, satisfies `pred`, and names the
/// same universe; returns that universe id, or `None` on timeout.
fn await_consistent(
    nodes: &[Node],
    timeout: Duration,
    pred: impl Fn(&scrape::HeaderStats) -> bool,
) -> Option<String> {
    await_value(timeout, POLL, || {
        let mut net: Option<String> = None;
        for node in nodes {
            let stats = scrape::header_stats(&node.screen())?;
            if !pred(&stats) {
                return None;
            }
            match &net {
                None => net = Some(stats.net),
                Some(seen) if *seen != stats.net => return None,
                Some(_) => {}
            }
        }
        net
    })
}

/// Sum of (sessions_ok, sessions_failed) across every parseable header.
fn session_totals(nodes: &[Node]) -> (u64, u64) {
    nodes
        .iter()
        .filter_map(|node| scrape::header_stats(&node.screen()))
        .fold((0, 0), |(ok, failed), stats| {
            (ok + stats.sessions_ok, failed + stats.sessions_failed)
        })
}

/// `min/p50/max` over a set of latencies.
fn dist(latencies: &[Duration]) -> String {
    if latencies.is_empty() {
        return "none".to_string();
    }
    let mut sorted = latencies.to_vec();
    sorted.sort();
    format!(
        "min {:.1}s, p50 {:.1}s, max {:.1}s",
        sorted.first().expect("nonempty").as_secs_f64(),
        sorted[sorted.len() / 2].as_secs_f64(),
        sorted.last().expect("nonempty").as_secs_f64()
    )
}

/// `count×size` histogram of roster sizes, for convergence diagnostics.
fn roster_histogram(nodes: &[Node]) -> String {
    let mut histogram: BTreeMap<Option<usize>, usize> = BTreeMap::new();
    for node in nodes {
        *histogram
            .entry(scrape::roster_count(&node.screen()))
            .or_default() += 1;
    }
    histogram
        .iter()
        .map(|(size, count)| match size {
            Some(size) => format!("{count} nodes at {size}"),
            None => format!("{count} nodes unreadable"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// `count×live` histogram of live-entry counts, for settle diagnostics.
fn live_histogram(nodes: &[Node]) -> String {
    let mut histogram: BTreeMap<Option<u64>, usize> = BTreeMap::new();
    for node in nodes {
        *histogram
            .entry(scrape::header_stats(&node.screen()).map(|s| s.live))
            .or_default() += 1;
    }
    histogram
        .iter()
        .map(|(live, count)| match live {
            Some(live) => format!("{count} nodes at {live}"),
            None => format!("{count} nodes unreadable"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Print a diagnosis line for every node that missed a deadline: its name,
/// whether the process is even alive, and its header line.
fn diagnose_stragglers(
    log: &Log,
    nodes: &mut [Node],
    latencies: &[Option<Duration>],
    idxs: &[usize],
) {
    for (slot, &i) in latencies.iter().zip(idxs) {
        if slot.is_some() {
            continue;
        }
        let node = &mut nodes[i];
        let alive = node.poll_exit().is_none() && !node.eof();
        let screen = node.screen();
        let header = screen.lines().next().unwrap_or("<blank>");
        log.say(&format!(
            "straggler {}: alive={alive} header=`{header}`",
            node.name
        ));
        if !alive {
            // The tail of a dead node's transcript usually names the error.
            let raw = node.raw();
            let tail: String = raw.chars().skip(raw.len().saturating_sub(500)).collect();
            log.say(&format!("  {} transcript tail: {tail:?}", node.name));
        }
    }
}

/// Sorted RSS samples in KB for every live node, via one `ps` call.
fn rss_kb(nodes: &[Node]) -> Vec<u64> {
    let pids: Vec<String> = nodes
        .iter()
        .filter_map(|node| node.pid())
        .map(|pid| pid.to_string())
        .collect();
    if pids.is_empty() {
        return Vec::new();
    }
    let Ok(output) = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pids.join(",")])
        .output()
    else {
        return Vec::new();
    };
    let mut rss: Vec<u64> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect();
    rss.sort_unstable();
    rss
}
