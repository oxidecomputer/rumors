//! The report: `report.md` and its SVG plots, rendered from a results
//! directory's files alone, so it can be rendered again, or merged across
//! shards, at any time.
//!
//! # How the report reads each point
//!
//! For every point, the report takes medians over its finished sessions and
//! splits the median wall time *T* into what could explain it:
//!
//! - *wire time*: the busier direction's socket bytes over its rate (the
//!   emulated rate, or in pair mode the goodput the network probe measured);
//! - *latency*: the one-way delays the protocol needs, from the model of
//!   uniform hashing ([`crate::point::Model::hops`]), times half the round
//!   trip, plus one round trip to connect on a fresh connection;
//! - *computation*: the busier peer process's CPU time during the session.
//!
//! The largest of the three names what bound the point. The *effective hops*
//! are what is left of *T* after the connection, the wire, and the computation,
//! in one-way delays: set beside the model's minimum, they show how close the
//! session came to the fewest round trips the protocol needs. *Link use* is
//! each direction's bytes over *T* times its rate.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rumors::default;
use serde::{Deserialize, Serialize};

use crate::calibrate::Calibration;
use crate::env::{Build, Environment};
use crate::plot::{Chart, Series, line_chart};
use crate::point::{self, Model, Replicas, Transport, UNLIMITED_BUDGET, minimum_budget};
use crate::probe::ProbeResult;
use crate::record::{Row, read};
use crate::run::RunRecord;
use crate::stats::{CONFIDENCE, Summary, summarize};
use crate::units::{format_bytes, human_bytes, human_seconds};

/// What pair mode measured of the network before its grid.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Network {
    /// The serving host's environment.
    pub server: Option<Environment>,
    /// Probes from this host to the server.
    pub probes: Vec<ProbeResult>,
}

impl Network {
    /// The median probed round trip, in seconds.
    pub fn rtt(&self) -> Option<f64> {
        summarize(
            &self
                .probes
                .iter()
                .map(|p| p.rtt_seconds)
                .collect::<Vec<_>>(),
            0.0,
        )
        .map(|s| s.median)
    }

    /// The median probed goodput, in bytes per second.
    pub fn goodput(&self) -> Option<f64> {
        summarize(
            &self
                .probes
                .iter()
                .map(|p| p.goodput_bytes_per_second)
                .collect::<Vec<_>>(),
            0.0,
        )
        .map(|s| s.median)
    }
}

/// One point's medians and their reading.
#[derive(Clone, Debug)]
struct PointSummary {
    /// A finished session of the point, for its settings.
    row: Row,
    /// Sessions that finished.
    finished: usize,
    /// Sessions that failed.
    failed: usize,
    /// The wall time.
    wall: Summary,
    /// Median connect seconds.
    connect: f64,
    /// Median bytes A sent.
    bytes_a: f64,
    /// Median bytes B sent.
    bytes_b: f64,
    /// Median CPU seconds of the busier process.
    cpu: f64,
    /// Median of the larger side's heap above both its start and its end.
    transient_heap: f64,
    /// Median of the larger side's resident peak.
    rss_peak: Option<f64>,
    /// Median retransmitted segments, both sides.
    retransmits: Option<f64>,
    /// Median budget waits, both sides.
    budget_waits: f64,
    /// Size draws redrawn over all its sessions.
    redrawn: u64,
    /// The A-to-B rate in bytes per second, if known.
    rate_a: Option<f64>,
    /// The B-to-A rate in bytes per second, if known.
    rate_b: Option<f64>,
    /// The round trip in seconds, if known.
    rtt: Option<f64>,
}

impl PointSummary {
    /// The replica counts.
    fn replicas(&self) -> Replicas {
        Replicas {
            shared: self.row.shared,
            insert_a: self.row.insert_a,
            insert_b: self.row.insert_b,
            redact_a: self.row.redact_a,
            redact_b: self.row.redact_b,
        }
    }

    /// The model's predictions.
    fn model(&self) -> Model {
        Model::of(&self.replicas(), self.row.size_mean_bytes)
    }

    /// Wire seconds of the busier direction.
    fn wire(&self) -> Option<f64> {
        let a = self.rate_a.map(|r| self.bytes_a / r);
        let b = self.rate_b.map(|r| self.bytes_b / r);
        match (a, b) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        }
    }

    /// The latency the protocol needs, in seconds, connection included.
    fn latency(&self) -> Option<f64> {
        let rtt = self.rtt?;
        let connect = if self.row.connection == "fresh" {
            rtt
        } else {
            0.0
        };
        Some(self.model().hops * rtt / 2.0 + connect)
    }

    /// What bound the point: the largest of wire, latency, and computation.
    fn bound(&self) -> &'static str {
        let wire = self.wire().unwrap_or(0.0);
        let latency = self.latency().unwrap_or(0.0);
        if self.cpu >= wire && self.cpu >= latency {
            "computation"
        } else if wire >= latency {
            "bandwidth"
        } else {
            "latency"
        }
    }

    /// One-way delays left after connection, wire, and computation.
    fn effective_hops(&self) -> Option<f64> {
        let rtt = self.rtt.filter(|r| *r > 0.0)?;
        let rest = self.wall.median - self.connect - self.wire().unwrap_or(0.0).max(self.cpu);
        Some(rest / (rtt / 2.0))
    }

    /// Each direction's use of its link.
    fn utilization(&self) -> (Option<f64>, Option<f64>) {
        let t = self.wall.median;
        (
            self.rate_a.map(|r| self.bytes_a / (t * r)),
            self.rate_b.map(|r| self.bytes_b / (t * r)),
        )
    }

    /// The point's settings as a key, leaving out its budgets or its TCP buffer
    /// limit, so points that differ only there compare.
    fn family(&self, with_budget: bool, with_buffers: bool) -> String {
        let r = &self.row;
        let mut key = format!(
            "{} {} {} {} {} | {}~{}<={} | t{:?}/{:?} d{:?} s{:?} | {} {:?} {:?} {:?} | {} {}",
            r.shared,
            r.insert_a,
            r.insert_b,
            r.redact_a,
            r.redact_b,
            r.size_mean_bytes,
            r.size_cv,
            r.max_message_size_bytes,
            r.target_message_size_a_bytes,
            r.target_message_size_b_bytes,
            r.payload_depth_limit,
            r.session_deadline_seconds,
            r.link,
            r.rate_a_to_b_bits,
            r.rate_b_to_a_bits,
            r.rtt_us,
            r.connection,
            stack_text(r),
        );
        if with_budget {
            write!(key, " | b{:?}/{:?}", r.budget_a_bytes, r.budget_b_bytes)
                .expect("writing to a string");
        }
        if with_buffers {
            write!(key, " | tcp{:?}", r.tcp_buffer_bytes).expect("writing to a string");
        }
        key
    }

    /// The smaller side's effective budget in bytes: the budget the session ran
    /// at ([`effective_budget`]).
    fn budget(&self) -> u64 {
        let row = &self.row;
        let a = effective_budget(row.budget_a_bytes, row.a_effective_budget_bytes);
        let b = effective_budget(row.budget_b_bytes, row.b_effective_budget_bytes);
        a.min(b)
    }

    /// The session heap over the smaller side's effective budget, or `None`
    /// when both sides ran unlimited, with no bound to compare against.
    fn heap_ratio(&self) -> Option<f64> {
        let budget = self.budget();
        (budget != UNLIMITED_BUDGET).then(|| self.transient_heap / budget as f64)
    }
}

/// The budget a side ran at: the effective budget its peer `reported`, or for a
/// row that records none, the side's request (`None` for the library's
/// default).
///
/// The library raises a request below the minimum for the side's message-size
/// target to that minimum, which is [`minimum_budget`] at the default target.
/// It doesn't publish the minimum at other targets, so the report reads
/// the raised budget the library reports
/// ([`SynchronizationSettings`](rumors::SynchronizationSettings)) rather than
/// recomputing it.
fn effective_budget(requested: Option<u64>, reported: Option<u64>) -> u64 {
    reported.unwrap_or_else(|| requested.unwrap_or(default::SYNC_MEMORY_BUDGET as u64))
}

/// The median of `values`, or NaN for none.
fn median(values: impl Iterator<Item = f64>) -> f64 {
    summarize(&values.collect::<Vec<_>>(), 0.0).map_or(f64::NAN, |s| s.median)
}

/// The median of the present `values`, if any are.
fn median_some(values: impl Iterator<Item = Option<f64>>) -> Option<f64> {
    let present: Vec<f64> = values.flatten().collect();
    summarize(&present, 0.0).map(|s| s.median)
}

/// Summarizes the sessions of one point.
fn summarize_point(rows: &[&Row], network: Option<&Network>) -> Option<PointSummary> {
    let ok: Vec<&&Row> = rows.iter().filter(|r| r.ok()).collect();
    let row = (**ok.first()?).clone();
    let wall = summarize(
        &ok.iter().filter_map(|r| r.wall_seconds).collect::<Vec<_>>(),
        CONFIDENCE,
    )?;
    let side_heap = |before: Option<u64>, peak: Option<u64>, after: Option<u64>| {
        let base = before?.max(after?);
        Some(peak?.saturating_sub(base) as f64)
    };
    let (rate_a, rate_b, rtt) = match row.link.as_str() {
        "emulated" => (
            row.rate_a_to_b_bits.map(|b| b as f64 / 8.0),
            row.rate_b_to_a_bits.map(|b| b as f64 / 8.0),
            row.rtt_us.map(|us| us as f64 / 1e6),
        ),
        _ => (
            network.and_then(Network::goodput),
            network.and_then(Network::goodput),
            network.and_then(Network::rtt),
        ),
    };
    Some(PointSummary {
        finished: ok.len(),
        failed: rows.len() - ok.len(),
        wall,
        connect: median(ok.iter().filter_map(|r| r.connect_seconds)),
        bytes_a: median(
            ok.iter()
                .filter_map(|r| r.a_socket_sent_bytes.map(|b| b as f64)),
        ),
        bytes_b: median(
            ok.iter()
                .filter_map(|r| r.b_socket_sent_bytes.map(|b| b as f64)),
        ),
        cpu: median(ok.iter().map(|r| {
            let a = r.a_user_seconds.unwrap_or(0.0) + r.a_system_seconds.unwrap_or(0.0);
            let b = r.b_user_seconds.unwrap_or(0.0) + r.b_system_seconds.unwrap_or(0.0);
            a.max(b)
        })),
        transient_heap: median(ok.iter().map(|r| {
            let a = side_heap(
                r.a_heap_before_bytes,
                r.a_heap_peak_bytes,
                r.a_heap_after_bytes,
            );
            let b = side_heap(
                r.b_heap_before_bytes,
                r.b_heap_peak_bytes,
                r.b_heap_after_bytes,
            );
            a.unwrap_or(0.0).max(b.unwrap_or(0.0))
        })),
        rss_peak: median_some(ok.iter().map(|r| {
            r.a_rss_peak_bytes
                .zip(r.b_rss_peak_bytes)
                .map(|(a, b)| a.max(b) as f64)
        })),
        retransmits: median_some(ok.iter().map(|r| {
            r.a_retransmits
                .zip(r.b_retransmits)
                .map(|(a, b)| (a + b) as f64)
        })),
        budget_waits: median(
            ok.iter()
                .map(|r| (r.a_budget_waits.unwrap_or(0) + r.b_budget_waits.unwrap_or(0)) as f64),
        ),
        redrawn: rows
            .iter()
            .map(|r| r.redrawn_above_cap.unwrap_or(0) + r.redrawn_below_one_byte.unwrap_or(0))
            .sum(),
        rate_a,
        rate_b,
        rtt,
        row,
    })
}

/// Formats an optional value, or a dash.
fn or_dash<T>(value: Option<T>, format: impl Fn(T) -> String) -> String {
    value.map_or_else(|| "–".to_string(), format)
}

/// A rate in bits per second, readably.
fn human_rate(bits: Option<u64>) -> String {
    match bits {
        None => "uncapped".to_string(),
        Some(bits) if bits.is_multiple_of(1_000_000_000) => {
            format!("{} Gbit/s", bits / 1_000_000_000)
        }
        Some(bits) if bits.is_multiple_of(1_000_000) => format!("{} Mbit/s", bits / 1_000_000),
        Some(bits) => format!("{:.3} Mbit/s", bits as f64 / 1e6),
    }
}

/// The link column of a row.
fn link_text(row: &Row) -> String {
    if row.link != "emulated" {
        return "network".to_string();
    }
    let rate = if row.rate_a_to_b_bits == row.rate_b_to_a_bits {
        human_rate(row.rate_a_to_b_bits)
    } else {
        format!(
            "{} / {}",
            human_rate(row.rate_a_to_b_bits),
            human_rate(row.rate_b_to_a_bits)
        )
    };
    format!(
        "{rate}, {}",
        or_dash(row.rtt_us, |us| human_seconds(us as f64 / 1e6))
    )
}

/// One side's budget as the report prints it: the requested value, and, when
/// the side ran at more than it requested, that it ran at the minimum for its
/// target instead (see [`effective_budget`]).
fn side_budget_text(requested: Option<u64>, reported: Option<u64>) -> String {
    let asked = requested.unwrap_or(default::SYNC_MEMORY_BUDGET as u64);
    if effective_budget(requested, reported) > asked {
        format!("{} → minimum", point::budget_text(requested))
    } else {
        point::budget_text(requested)
    }
}

/// The budget column of a row: one entry when both sides requested and ran at
/// the same budgets, else one per side.
fn budget_text(row: &Row) -> String {
    let a = side_budget_text(row.budget_a_bytes, row.a_effective_budget_bytes);
    let b = side_budget_text(row.budget_b_bytes, row.b_effective_budget_bytes);
    if row.budget_a_bytes == row.budget_b_bytes
        && row.a_effective_budget_bytes == row.b_effective_budget_bytes
    {
        a
    } else {
        format!("A {a} / B {b}")
    }
}

/// The pair column of a row.
fn pair_text(row: &Row) -> String {
    let r = Replicas {
        shared: row.shared,
        insert_a: row.insert_a,
        insert_b: row.insert_b,
        redact_a: row.redact_a,
        redact_b: row.redact_b,
    };
    let size = if row.size_cv == 0.0 {
        format!("{} B", row.size_mean_bytes)
    } else {
        format!("{} B CV {}", row.size_mean_bytes, row.size_cv)
    };
    format!("{r}, {size}")
}

/// The stack column of a row: `tls` or `tcp`, as [`Transport`] prints it.
fn stack_text(row: &Row) -> String {
    Transport { tls: row.tls }.to_string()
}

/// A file name made of `text`'s letters and digits.
fn slug(text: &str) -> String {
    let mut slug: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    slug.trim_matches('-').chars().take(80).collect()
}

/// Reads an optional JSON file.
fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

/// One results directory's files, as the report reads them.
struct Source {
    /// The directory.
    dir: PathBuf,
    /// Its `environment.json`, if present and readable.
    environment: Option<Environment>,
    /// Its `run.json`, if present and readable.
    run: Option<RunRecord>,
    /// Its `calibration.json`, if present and readable.
    calibration: Option<Calibration>,
    /// Its `sessions.csv`.
    rows: Vec<Row>,
}

impl Source {
    /// Reads `dir`.
    ///
    /// # Errors
    ///
    /// If `dir` has no `sessions.csv`, or it cannot be read.
    fn read(dir: &Path) -> Result<Self> {
        Ok(Source {
            dir: dir.to_path_buf(),
            environment: read_json(&dir.join("environment.json")),
            run: read_json(&dir.join("run.json")),
            calibration: read_json(&dir.join("calibration.json")),
            rows: read(&dir.join("sessions.csv"))?,
        })
    }

    /// How the report names this directory among several: by its shard, if it
    /// holds one, and by its path.
    fn label(&self) -> String {
        match self.run.as_ref().and_then(|run| run.shard) {
            Some(shard) => format!(
                "shard {} of {} (`{}`)",
                shard.index,
                shard.count,
                self.dir.display()
            ),
            None => format!("`{}`", self.dir.display()),
        }
    }
}

/// The host names of `sources`, each once, in their order, joined for a
/// sentence.
fn hosts(sources: &[Source]) -> String {
    let mut names: Vec<&str> = Vec::new();
    for name in sources
        .iter()
        .filter_map(|s| s.environment.as_ref())
        .map(|e| e.hostname.as_str())
    {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    match names.as_slice() {
        [] => "unknown host".to_string(),
        [one] => (*one).to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// Renders the report for the results in `dirs` (one directory, or the shards
/// of one run, which may have run on several hosts) into `out`, returning the
/// report's path.
///
/// The points' sections read every directory's rows together. What belongs to
/// one directory, its run's summary, sentinels, calibration, and host, the
/// report gives for each directory.
///
/// # Errors
///
/// If a directory has no `sessions.csv` or a file cannot be written.
pub fn render(dirs: &[PathBuf], out: &Path) -> Result<PathBuf> {
    let sources = dirs
        .iter()
        .map(|dir| Source::read(dir))
        .collect::<Result<Vec<_>>>()?;
    let first = sources.first().context("no results directory given")?;
    let rows: Vec<Row> = sources
        .iter()
        .flat_map(|s| s.rows.iter().cloned())
        .collect();
    let environment = first.environment.as_ref();
    let network: Option<Network> = read_json(&first.dir.join("network.json"));
    fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;

    let mut by_point: BTreeMap<(String, String, String), Vec<&Row>> = BTreeMap::new();
    for row in rows.iter().filter(|r| r.kind != "sentinel") {
        by_point
            .entry((row.group.clone(), row.point.clone(), row.kind.clone()))
            .or_default()
            .push(row);
    }
    let failed_points: Vec<(String, usize, String)> = by_point
        .iter()
        .filter(|(_, rows)| rows.iter().all(|r| !r.ok()))
        .map(|((_, key, _), rows)| {
            (
                key.clone(),
                rows.len(),
                rows[0]
                    .error
                    .clone()
                    .unwrap_or_else(|| "no measurement".into()),
            )
        })
        .collect();
    let summaries: Vec<(String, PointSummary)> = by_point
        .iter()
        .filter_map(|((group, _, _), rows)| {
            Some((group.clone(), summarize_point(rows, network.as_ref())?))
        })
        .collect();

    let mut md = String::new();
    writeln!(md, "# Rumors network benchmark on {}\n", hosts(&sources))?;
    write_summary(&mut md, &sources, &rows)?;
    write_reading_guide(&mut md)?;
    if let Some(network) = &network {
        write_network(&mut md, network, environment)?;
    }
    write_results(&mut md, &summaries)?;
    write_budgets(&mut md, &summaries)?;
    write_buffers(&mut md, &summaries, environment)?;
    write_memory(&mut md, &summaries)?;
    write_charts(&mut md, &summaries, out)?;
    if !failed_points.is_empty() {
        writeln!(md, "## Points with no finished session\n")?;
        writeln!(md, "| point | sessions | first error |\n|---|---|---|")?;
        for (key, count, error) in &failed_points {
            writeln!(md, "| `{key}` | {count} | {} |", error.replace('|', "\\|"))?;
        }
        writeln!(md)?;
    }
    write_sentinels(&mut md, &sources)?;
    write_calibrations(&mut md, &sources)?;
    write_environments(&mut md, &sources, network.as_ref())?;
    let path = out.join("report.md");
    fs::write(&path, md).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

/// The opening summary: one paragraph for one directory, or a line for each of
/// several, then the build or builds that measured them.
fn write_summary(md: &mut String, sources: &[Source], rows: &[Row]) -> Result<()> {
    match sources {
        [source] => writeln!(md, "{}", part_summary(&source.rows, source.run.as_ref()))?,
        _ => {
            let finished = rows.iter().filter(|r| r.ok()).count();
            writeln!(
                md,
                "{} sessions ({finished} finished, {} failed) from {} results directories, each run \
                 apart:\n",
                rows.len(),
                rows.len() - finished,
                sources.len()
            )?;
            for source in sources {
                writeln!(
                    md,
                    "- `{}` on {}: {}",
                    source.dir.display(),
                    source
                        .environment
                        .as_ref()
                        .map_or("an unknown host", |e| e.hostname.as_str()),
                    part_summary(&source.rows, source.run.as_ref()).replace('\n', " ")
                )?;
            }
        }
    }
    let mut builds: Vec<&Build> = Vec::new();
    for build in sources
        .iter()
        .filter_map(|s| s.environment.as_ref())
        .map(|e| &e.build)
    {
        if !builds.contains(&build) {
            builds.push(build);
        }
    }
    match builds.as_slice() {
        [] => {}
        [build] => writeln!(md, "\nBuild: {build}.")?,
        _ => {
            writeln!(
                md,
                "\n**The directories come from different builds**, so their sessions measured different code:\n"
            )?;
            for source in sources {
                if let Some(environment) = &source.environment {
                    writeln!(md, "- {}: {}.", source.label(), environment.build)?;
                }
            }
        }
    }
    if sources
        .iter()
        .any(|s| s.run.as_ref().is_some_and(|r| !r.drift.is_empty()))
    {
        writeln!(
            md,
            "\n**The sentinel flagged drift during this run**; see [Sentinels](#sentinels) before relying on the sessions after it."
        )?;
    }
    writeln!(md)?;
    Ok(())
}

/// What one directory's run did, in sentences, from its `rows` and its
/// `run.json`.
fn part_summary(rows: &[Row], run: Option<&RunRecord>) -> String {
    let finished = rows.iter().filter(|r| r.ok()).count();
    let Some(run) = run else {
        return format!("{} sessions, {finished} finished.", rows.len());
    };
    let mut text = format!(
        "{} mode, {} sessions ({} finished, {} failed) {}, {} at once ({}).",
        run.mode,
        rows.len(),
        finished,
        rows.len() - finished,
        match run.ended {
            Some(_) => format!("in {}", human_seconds(run.seconds)),
            None => "so far, in a run that has not finished".into(),
        },
        run.jobs,
        run.jobs_source
    );
    if let Some(preparation) = &run.preparation {
        text.push_str(&format!(
            "\nIt prepared up to {} sessions at once ({}), only while no session was timed, with at most \
             {} of prepared sessions waiting by the memory model's prediction, and at most {} sessions \
             in one window.",
            preparation.jobs,
            preparation.jobs_source,
            human_bytes(preparation.memory_bytes as f64),
            preparation.window
        ));
    }
    match run.resumptions.len() {
        0 => {}
        1 => text.push_str("\nThe run was cut short and resumed once."),
        count => text.push_str(&format!(
            "\nThe run was cut short and resumed {count} times."
        )),
    }
    if let Some(shard) = run.shard {
        text.push_str(&format!(
            "\nThis directory holds shard {} of {}.",
            shard.index, shard.count
        ));
    }
    text
}

/// The guide to the columns.
fn write_reading_guide(md: &mut String) -> Result<()> {
    writeln!(
        md,
        "## How to read this report\n\n\
         Each row is one point: a replica pair (*s* shared messages, *i* inserted and *r* redacted on each side), \
         its message sizes, the `Peer` settings, and the link. Times are medians over the point's finished sessions, \
         with an interval between two order statistics that covers the true median with the probability shown \
         (exact for any distribution; with five sessions, the full range at 93.75%).\n\n\
         - **Stack** is the transport the session ran over: `tls` for TLS 1.3 on its TCP connection, `tcp` \
           for plain TCP. Socket bytes are counted beneath TLS, so over TLS they include each record's framing.\n\
         - **T** is the session's wall time from side B's connect to its completion.\n\
         - **Bound** names the largest of three explanations of *T*: *bandwidth* (the busier direction's bytes \
           over its rate), *latency* (the one-way delays the protocol needs, from the uniform-hashing model, \
           plus the connection's round trip), and *computation* (the busier process's CPU time).\n\
         - **Hops** compares the one-way delays left in *T* after the connection, the wire time, and the \
           computation, against the model's minimum for the pair. Close to the minimum means the session \
           pipelined well; far above it means it waited. Small round trips make the ratio noisy.\n\
         - **Use** is each direction's bytes over *T* times its rate: how busy the session kept the link.\n\
         - **Heap** is the largest live heap either process held during the session beyond both its \
           start and its end, which excludes the replica's own growth; the budget bounds the session's \
           pipelining memory, and `Peer::sync_memory_budget` lists what lies outside it.\n"
    )?;
    Ok(())
}

/// The network section of a pair-mode report.
fn write_network(md: &mut String, network: &Network, env: Option<&Environment>) -> Result<()> {
    writeln!(md, "## The network\n")?;
    if let (Some(rtt), Some(goodput)) = (network.rtt(), network.goodput()) {
        writeln!(
            md,
            "The probe measured a round trip of {} and a goodput of {:.1} Mbit/s from this host to the server \
             (median of {} probes): a bandwidth-delay product of {}.\n",
            human_seconds(rtt),
            goodput * 8e-6,
            network.probes.len(),
            human_bytes(goodput * rtt)
        )?;
        let windows: Vec<(String, u64)> = env
            .into_iter()
            .chain(network.server.as_ref())
            .filter_map(|e| Some((e.hostname.clone(), e.tcp.usable_window?)))
            .collect();
        for (host, window) in windows {
            let verdict = if (window as f64) < goodput * rtt {
                "smaller than the bandwidth-delay product: one connection cannot keep this link busy, and raising the host's TCP buffer limits would help"
            } else {
                "enough for this link"
            };
            writeln!(
                md,
                "- {host}: usable TCP window {}, {verdict}.",
                human_bytes(window as f64)
            )?;
        }
        writeln!(md)?;
    }
    Ok(())
}

/// The results tables, one per group.
fn write_results(md: &mut String, summaries: &[(String, PointSummary)]) -> Result<()> {
    writeln!(md, "## Results\n")?;
    let mut groups: BTreeMap<&str, Vec<&PointSummary>> = BTreeMap::new();
    for (group, summary) in summaries {
        groups.entry(group.as_str()).or_default().push(summary);
    }
    for (group, points) in groups {
        writeln!(md, "### {group}\n")?;
        writeln!(
            md,
            "| pair | link | budget | conn | stack | TCP buffers | n | T (interval, coverage) | bytes A→B / B→A | bound | hops (model) | use A→B / B→A | heap | waits | retrans |"
        )?;
        writeln!(
            md,
            "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
        )?;
        for p in points {
            let (use_a, use_b) = p.utilization();
            let percent = |u: Option<f64>| or_dash(u, |u| format!("{:.0}%", u * 100.0));
            writeln!(
                md,
                "| {} | {} | {} | {} | {} | {} | {}{} | {} ({}–{}, {:.0}%) | {} / {} | {} | {} ({:.1}) | {} / {} | {} | {:.0} | {} |",
                pair_text(&p.row),
                link_text(&p.row),
                budget_text(&p.row),
                p.row.connection,
                stack_text(&p.row),
                or_dash(p.row.tcp_buffer_bytes, |b| format!(
                    "raised to {}",
                    format_bytes(b)
                )),
                p.finished,
                if p.failed > 0 {
                    format!(" ({} failed)", p.failed)
                } else {
                    String::new()
                },
                human_seconds(p.wall.median),
                human_seconds(p.wall.low),
                human_seconds(p.wall.high),
                p.wall.coverage * 100.0,
                human_bytes(p.bytes_a),
                human_bytes(p.bytes_b),
                p.bound(),
                or_dash(p.effective_hops(), |h| format!("{h:.1}")),
                p.model().hops,
                percent(use_a),
                percent(use_b),
                human_bytes(p.transient_heap),
                p.budget_waits,
                or_dash(p.retransmits, |r| format!("{r:.0}")),
            )?;
        }
        writeln!(md)?;
    }
    let redrawn: u64 = summaries.iter().map(|(_, p)| p.redrawn).sum();
    if redrawn > 0 {
        writeln!(
            md,
            "Message-size draws redrawn because they fell outside 1 byte to the cap: {redrawn} over the run \
             (each session's count is in `sessions.csv`).\n"
        )?;
    }
    Ok(())
}

/// Slowdown against budget.
fn write_budgets(md: &mut String, summaries: &[(String, PointSummary)]) -> Result<()> {
    let mut families: BTreeMap<String, Vec<&PointSummary>> = BTreeMap::new();
    for (_, p) in summaries {
        families.entry(p.family(false, true)).or_default().push(p);
    }
    let families: Vec<Vec<&PointSummary>> =
        families.into_values().filter(|f| f.len() > 1).collect();
    if families.is_empty() {
        return Ok(());
    }
    writeln!(md, "## Duration against budget\n")?;
    writeln!(
        md,
        "Each row compares one pair and link across budgets: the slowdown is *T* at that budget over *T* at the \
         largest budget measured for it. A slowdown near 1 means the budget did not limit the session.\n"
    )?;
    writeln!(
        md,
        "| pair | link | budget | T | slowdown | waits |\n|---|---|---|---|---|---|"
    )?;
    for mut family in families {
        family.sort_by_key(|p| std::cmp::Reverse(p.budget()));
        let reference = family[0].wall.median;
        for p in family {
            writeln!(
                md,
                "| {} | {} | {} | {} | {:.3} | {:.0} |",
                pair_text(&p.row),
                link_text(&p.row),
                budget_text(&p.row),
                human_seconds(p.wall.median),
                p.wall.median / reference,
                p.budget_waits
            )?;
        }
    }
    writeln!(md)?;
    Ok(())
}

/// Raised TCP buffers against the host's.
fn write_buffers(
    md: &mut String,
    summaries: &[(String, PointSummary)],
    env: Option<&Environment>,
) -> Result<()> {
    let host: BTreeMap<String, &PointSummary> = summaries
        .iter()
        .filter(|(_, p)| p.row.tcp_buffer_bytes.is_none())
        .map(|(_, p)| (p.family(true, false), p))
        .collect();
    let raised: Vec<&PointSummary> = summaries
        .iter()
        .map(|(_, p)| p)
        .filter(|p| p.row.tcp_buffer_bytes.is_some())
        .collect();
    writeln!(md, "## TCP buffer limits\n")?;
    if let Some(window) = env.and_then(|e| e.tcp.usable_window) {
        writeln!(
            md,
            "From the host's limits, one connection can keep at most about {} in flight. A point whose \
             bandwidth-delay product and busier direction both exceed that was rerun with the limits raised \
             inside its own network namespaces (the host's were not changed).\n",
            human_bytes(window as f64)
        )?;
    }
    if raised.is_empty() {
        writeln!(md, "No point needed raised limits.\n")?;
        return Ok(());
    }
    writeln!(
        md,
        "| pair | link | budget | T at host limits | T raised | raised to | ratio |\n|---|---|---|---|---|---|---|"
    )?;
    for p in raised {
        let base = host.get(&p.family(true, false));
        writeln!(
            md,
            "| {} | {} | {} | {} | {} | {} | {} |",
            pair_text(&p.row),
            link_text(&p.row),
            budget_text(&p.row),
            or_dash(base, |b| human_seconds(b.wall.median)),
            human_seconds(p.wall.median),
            or_dash(p.row.tcp_buffer_bytes, format_bytes),
            or_dash(base, |b| format!("{:.3}", p.wall.median / b.wall.median)),
        )?;
    }
    writeln!(
        md,
        "\nA ratio well below 1 means the host's TCP limits, not the protocol, held the session back on that link: \
         raise `net.ipv4.tcp_rmem` and `net.ipv4.tcp_wmem`'s maxima (or your platform's equivalents) on hosts \
         that gossip over such links.\n"
    )?;
    Ok(())
}

/// Memory against budget.
fn write_memory(md: &mut String, summaries: &[(String, PointSummary)]) -> Result<()> {
    writeln!(md, "## Memory against budget\n")?;
    writeln!(
        md,
        "The heap column is the larger process's session heap beyond both its start and its end. The \
         budget bounds the memory a session spends on pipelining; beside it a session keeps a fixed amount \
         per level of its comparison and wire buffers, which `Peer::sync_memory_budget` documents. The \
         resident peak is what a memory limit meets, allocator slack included.\n\n\
         A budget below the minimum, {} in this build, runs at the minimum: the budget column then shows \
         the request and \"→ minimum\", and the ratio is against the minimum. An unlimited budget has \
         no ratio.\n",
        human_bytes(minimum_budget() as f64)
    )?;
    writeln!(
        md,
        "| pair | link | budget | heap | heap ÷ budget | resident peak |\n|---|---|---|---|---|---|"
    )?;
    for (_, p) in summaries {
        writeln!(
            md,
            "| {} | {} | {} | {} | {} | {} |",
            pair_text(&p.row),
            link_text(&p.row),
            budget_text(&p.row),
            human_bytes(p.transient_heap),
            or_dash(p.heap_ratio(), |r| format!("{r:.3}")),
            or_dash(p.rss_peak, human_bytes)
        )?;
    }
    writeln!(md)?;
    Ok(())
}

/// The plots, one per group and pair: duration against round trip, one series
/// per rate, at the largest budget measured.
fn write_charts(md: &mut String, summaries: &[(String, PointSummary)], out: &Path) -> Result<()> {
    #[allow(clippy::type_complexity)]
    let mut panels: BTreeMap<(String, String), BTreeMap<String, Vec<(u64, &PointSummary)>>> =
        BTreeMap::new();
    for (group, p) in summaries {
        if p.row.link != "emulated" || p.row.tcp_buffer_bytes.is_some() {
            continue;
        }
        let rate = if p.row.rate_a_to_b_bits == p.row.rate_b_to_a_bits {
            human_rate(p.row.rate_a_to_b_bits)
        } else {
            format!(
                "{} / {}",
                human_rate(p.row.rate_a_to_b_bits),
                human_rate(p.row.rate_b_to_a_bits)
            )
        };
        let series = format!(
            "{rate}, {}, {}, {}",
            budget_text(&p.row),
            p.row.connection,
            stack_text(&p.row)
        );
        panels
            .entry((group.clone(), pair_text(&p.row)))
            .or_default()
            .entry(series)
            .or_default()
            .push((p.row.rtt_us.unwrap_or(0), p));
    }
    let mut wrote_heading = false;
    for ((group, pair), series) in panels {
        let series: Vec<Series> = series
            .into_iter()
            .filter(|(_, points)| points.len() > 1)
            .map(|(label, mut points)| {
                points.sort_by_key(|(rtt, _)| *rtt);
                (
                    label,
                    points
                        .into_iter()
                        .map(|(rtt, p)| (rtt as f64 / 1e3, p.wall.median))
                        .collect(),
                )
            })
            .collect();
        if series.is_empty() {
            continue;
        }
        let name = format!("duration-{}.svg", slug(&format!("{group} {pair}")));
        let title = format!("{group}: {pair}");
        let chart = Chart {
            title: &title,
            x_label: "round-trip time (ms)",
            y_label: "session wall time T (s)",
            log_x: true,
            log_y: true,
        };
        if line_chart(&out.join(&name), &chart, &series)? {
            if !wrote_heading {
                writeln!(md, "## Duration across links\n")?;
                wrote_heading = true;
            }
            writeln!(md, "![{title}]({name})\n")?;
        }
    }
    Ok(())
}

/// The sentinels' section: for each directory whose run interleaved sentinels,
/// their times and any drift they flagged, under the directory's label when
/// there are several.
fn write_sentinels(md: &mut String, sources: &[Source]) -> Result<()> {
    let with_sentinels: Vec<&Source> = sources
        .iter()
        .filter(|s| s.rows.iter().any(|r| r.kind == "sentinel"))
        .collect();
    if with_sentinels.is_empty() {
        return Ok(());
    }
    writeln!(md, "## Sentinels\n")?;
    for source in with_sentinels {
        if sources.len() > 1 {
            writeln!(md, "### {}\n", source.label())?;
        }
        write_part_sentinels(md, &source.rows, source.run.as_ref())?;
    }
    Ok(())
}

/// One directory's sentinels: their times in run order, and any drift.
fn write_part_sentinels(md: &mut String, rows: &[Row], run: Option<&RunRecord>) -> Result<()> {
    let sentinels: Vec<&Row> = rows.iter().filter(|r| r.kind == "sentinel").collect();
    let bound = run.and_then(|r| r.sentinel_bound_seconds);
    writeln!(
        md,
        "The calibration's noise probe ran {} times through the run{}.",
        sentinels.len(),
        or_dash(bound, |b| format!(
            ", held to the slowest session calibration saw at this job count, {}",
            human_seconds(b)
        ))
    )?;
    let walls: Vec<String> = sentinels
        .iter()
        .map(|r| format!("{}: {}", r.sequence, or_dash(r.wall_seconds, human_seconds)))
        .collect();
    writeln!(
        md,
        "In run order (session number: T): {}.\n",
        walls.join(", ")
    )?;
    if let Some(run) = run
        && !run.drift.is_empty()
    {
        for (sequence, median) in &run.drift {
            writeln!(
                md,
                "- **Drift** at session {sequence}: the median of the last three sentinels was {}.",
                human_seconds(*median)
            )?;
        }
        writeln!(md)?;
    }
    Ok(())
}

/// One calibration: its verdict, its levels, and its preparation probe.
fn write_calibration(md: &mut String, calibration: &Calibration) -> Result<()> {
    writeln!(
        md,
        "The run used {} jobs. A job count passes when, for the noise probe's wall time (the heaviest pair on \
         an uncapped link) and for the fidelity probe's goodput and round trip (raw TCP on the fastest shaped \
         link), the median of its readings lies within the solo spread of the median of {} solo readings: \
         their range once the most extreme reading at each end is set aside. Verdict: {}.\n",
        calibration.jobs,
        calibration.solo.session_seconds.len(),
        calibration.verdict
    )?;
    writeln!(
        md,
        "| jobs | readings | noise T median | goodput median | round trip median | busy CPU | noise | fidelity |\n\
         |---|---|---|---|---|---|---|---|"
    )?;
    for level in std::iter::once(&calibration.solo).chain(&calibration.levels) {
        writeln!(
            md,
            "| {} | {} | {} | {:.1} Mbit/s | {} | {:.0}% | {} | {} |",
            level.jobs,
            level.session_seconds.len(),
            human_seconds(median(level.session_seconds.iter().copied())),
            median(level.goodput.iter().copied()) * 8e-6,
            human_seconds(median(level.rtt.iter().copied())),
            level.busy_share * 100.0,
            if level.noise_passed { "pass" } else { "fail" },
            if level.fidelity_passed {
                "pass"
            } else {
                "fail"
            },
        )?;
    }
    writeln!(md)?;
    let probe = &calibration.preparation;
    if probe.prepare_jobs > 0 {
        writeln!(
            md,
            "Preparation: each noise pair's preparation alone kept {:.2} processors busy, so the run \
             prepares {} sessions at once. {} noise pairs prepared together took a median of {} to build \
             and {} to join; an empty pair prepares alone in {}. Prepared sessions are predicted to hold \
             {} plus {} per message beyond its payload (the noise pair peaked at {}, the empty pair at {}).\n",
            probe.cores_per_preparation,
            probe.prepare_jobs,
            probe.copies,
            human_seconds(median(probe.build_seconds.iter().copied())),
            human_seconds(median(probe.fetch_seconds.iter().copied())),
            human_seconds(median(probe.empty_seconds.iter().copied())),
            human_bytes(calibration.memory.fixed_bytes),
            human_bytes(calibration.memory.overhead_bytes_per_message),
            human_bytes(probe.noise_bytes as f64),
            human_bytes(probe.empty_bytes as f64),
        )?;
    }
    Ok(())
}

/// The calibration section: each directory's calibration, under the directory's
/// label when there are several.
fn write_calibrations(md: &mut String, sources: &[Source]) -> Result<()> {
    let calibrated: Vec<&Source> = sources.iter().filter(|s| s.calibration.is_some()).collect();
    if calibrated.is_empty() {
        return Ok(());
    }
    writeln!(md, "## Calibration\n")?;
    for source in calibrated {
        if sources.len() > 1 {
            writeln!(md, "### {}\n", source.label())?;
        }
        if let Some(calibration) = &source.calibration {
            write_calibration(md, calibration)?;
        }
    }
    Ok(())
}

/// The environment section: the host each directory ran on, labelled by the
/// directory when there are several, and in pair mode the server.
fn write_environments(
    md: &mut String,
    sources: &[Source],
    network: Option<&Network>,
) -> Result<()> {
    let mut hosts: Vec<(String, &Environment)> = match sources {
        [source] => source
            .environment
            .iter()
            .map(|e| ("this host".to_string(), e))
            .collect(),
        _ => sources
            .iter()
            .filter_map(|s| Some((s.label(), s.environment.as_ref()?)))
            .collect(),
    };
    if hosts.is_empty() {
        return Ok(());
    }
    hosts.extend(
        network
            .and_then(|n| n.server.as_ref())
            .map(|s| ("server".to_string(), s)),
    );
    writeln!(md, "## Environment\n")?;
    for (role, e) in hosts {
        writeln!(
            md,
            "- **{role}**: {} ({} {}), {}, {} processors, {} memory; load {}.",
            e.hostname,
            e.os,
            e.kernel_release,
            e.cpu_model,
            e.logical_cpus,
            or_dash(e.memory_bytes, |b| human_bytes(b as f64)),
            or_dash(e.load_average, |l| format!(
                "{:.2} {:.2} {:.2}",
                l[0], l[1], l[2]
            )),
        )?;
        for (name, value) in &e.tcp.settings {
            writeln!(md, "  - `{name}` = `{value}`")?;
        }
        if let Some(cc) = &e.congestion_control {
            writeln!(md, "  - congestion control: `{cc}`")?;
        }
    }
    writeln!(md)?;
    Ok(())
}

#[cfg(test)]
mod tests;
