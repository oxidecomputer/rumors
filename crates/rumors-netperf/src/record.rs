//! The per-session record: one row of `sessions.csv` for every session a run
//! attempted, finished or failed.
//!
//! Every column is flat, so the file reads directly into a spreadsheet, pandas,
//! or R, and the report reads it back. Units are in the column names:
//! `_seconds`, `_bytes`, `_bits`, `_us`.

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Seek, SeekFrom};
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::driver::Measured;
use crate::grid::Session;
use crate::peer::Outcome;
use crate::point::{Connection, Link, TcpBuffers};
use crate::units::Rate;

/// One session's row.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Row {
    /// The session's place in the run's execution order, from zero.
    pub sequence: u64,
    /// `grid`, `raised` (a rerun with raised TCP buffers), or `sentinel`.
    pub kind: String,
    /// The point's group.
    pub group: String,
    /// The point's stable key.
    pub point: String,
    /// Which repetition of the point, from zero.
    pub repetition: u32,
    /// The timed slot that ran it.
    pub slot: usize,
    /// How many sessions the run timed at once.
    pub jobs: usize,
    /// The host's one-minute load average when the timed session started.
    pub load_1m: Option<f64>,

    /// Shared messages, *s*.
    pub shared: u64,
    /// A's insertions, *i_A*.
    pub insert_a: u64,
    /// B's insertions, *i_B*.
    pub insert_b: u64,
    /// A's redactions, *r_A*.
    pub redact_a: u64,
    /// B's redactions, *r_B*.
    pub redact_b: u64,
    /// The requested mean message size.
    pub size_mean_bytes: f64,
    /// The requested coefficient of variation.
    pub size_cv: f64,
    /// The message-size cap.
    pub max_message_size_bytes: u64,
    /// Side A's memory budget; empty for the library's default.
    pub budget_a_bytes: Option<u64>,
    /// Side B's memory budget; empty for the library's default.
    pub budget_b_bytes: Option<u64>,
    /// The memory budget side A's peer reported it runs at, after the library
    /// raised a budget below its minimum and filled in its default; empty when
    /// the session failed before reporting it.
    pub a_effective_budget_bytes: Option<u64>,
    /// The memory budget side B's peer reported it runs at.
    pub b_effective_budget_bytes: Option<u64>,
    /// Side A's target message size; empty for the default.
    pub target_message_size_a_bytes: Option<u64>,
    /// Side B's target message size; empty for the default.
    pub target_message_size_b_bytes: Option<u64>,
    /// The payload depth limit; empty for the default.
    pub payload_depth_limit: Option<u64>,
    /// The session deadline; empty for none.
    pub session_deadline_seconds: Option<f64>,
    /// `emulated` or `network`.
    pub link: String,
    /// The A-to-B rate; empty when uncapped or between hosts.
    pub rate_a_to_b_bits: Option<u64>,
    /// The B-to-A rate; empty when uncapped or between hosts.
    pub rate_b_to_a_bits: Option<u64>,
    /// The emulated round trip; empty between hosts.
    pub rtt_us: Option<u64>,
    /// `fresh` or `warm`.
    pub connection: String,
    /// Whether the session ran over TLS.
    pub tls: bool,
    /// The raised TCP buffer limit; empty at the host's limits.
    pub tcp_buffer_bytes: Option<u64>,
    /// The A-to-B `netem` queue limit, in packets.
    pub queue_a_to_b_packets: Option<u64>,
    /// The B-to-A `netem` queue limit, in packets.
    pub queue_b_to_a_packets: Option<u64>,

    /// The offset drawn for the history.
    pub offset: Option<u64>,
    /// Messages built into the pair.
    pub built_messages: Option<u64>,
    /// Their mean encoded size: the realized mean, after redraws.
    pub built_mean_bytes: Option<f64>,
    /// Size draws redrawn above the cap.
    pub redrawn_above_cap: Option<u64>,
    /// Size draws redrawn below one byte.
    pub redrawn_below_one_byte: Option<u64>,
    /// Wall seconds side A took to build the pair. Other sessions' preparations
    /// run beside it, up to the run's preparation count at once (the intervals
    /// below say which), so it measures the build on a shared host, not alone.
    pub build_seconds: Option<f64>,
    /// Wall seconds side B took to join its replica, on a shared host as
    /// `build_seconds` is.
    pub fetch_seconds: Option<f64>,
    /// Side A's resident high-water mark over the preparation (Linux).
    #[serde(default)]
    pub a_prepare_rss_peak_bytes: Option<u64>,
    /// Side B's resident high-water mark over the preparation (Linux).
    #[serde(default)]
    pub b_prepare_rss_peak_bytes: Option<u64>,
    /// Side A's CPU seconds over the preparation (Linux), which other
    /// preparations sharing the host change less than its wall time.
    #[serde(default)]
    pub a_prepare_cpu_seconds: Option<f64>,
    /// Side B's CPU seconds over the preparation (Linux).
    #[serde(default)]
    pub b_prepare_cpu_seconds: Option<f64>,
    /// When the preparation began, in seconds since the Unix epoch, as the
    /// orchestrator's monotonic clock places it: from starting the processes.
    #[serde(default)]
    pub prepare_began_unix_seconds: Option<f64>,
    /// When the preparation ended: with both processes idle, the pair in place
    /// and the link shaped, or with the preparation failed and its processes
    /// stopped.
    #[serde(default)]
    pub prepare_ended_unix_seconds: Option<f64>,
    /// When the timed session began: from the first command of the measured
    /// exchange. Empty when the preparation failed.
    #[serde(default)]
    pub timed_began_unix_seconds: Option<f64>,
    /// When the timed session ended: once both processes had exited.
    #[serde(default)]
    pub timed_ended_unix_seconds: Option<f64>,

    /// The session's wall time as side B saw it: from the start of its connect
    /// (fresh) or the end of the warm-up (warm) until its `gossip_once`
    /// returned.
    pub wall_seconds: Option<f64>,
    /// Side B's connect time; zero on a warm connection's session.
    pub connect_seconds: Option<f64>,
    /// Side A's wall time, from accepting the connection.
    pub a_wall_seconds: Option<f64>,
    /// Side A's session latency as the library measured it.
    pub a_library_seconds: Option<f64>,
    /// Side B's session latency as the library measured it.
    pub b_library_seconds: Option<f64>,

    /// Side A's user CPU seconds over the session.
    pub a_user_seconds: Option<f64>,
    /// Side A's system CPU seconds.
    pub a_system_seconds: Option<f64>,
    /// Side B's user CPU seconds.
    pub b_user_seconds: Option<f64>,
    /// Side B's system CPU seconds.
    pub b_system_seconds: Option<f64>,

    /// Every byte A wrote to the socket during the session, counted beneath
    /// TLS: over TLS, each record's framing included.
    pub a_socket_sent_bytes: Option<u64>,
    /// Every byte B wrote to the socket during the session.
    pub b_socket_sent_bytes: Option<u64>,
    /// Writes A's socket accepted bytes from, beneath every buffer and TLS: on
    /// plain TCP, one system call each.
    pub a_socket_writes: Option<u64>,
    /// Writes B's socket accepted bytes from.
    pub b_socket_writes: Option<u64>,
    /// Reads A's socket completed: on plain TCP, one system call each.
    pub a_socket_reads: Option<u64>,
    /// Reads B's socket completed.
    pub b_socket_reads: Option<u64>,
    /// A's data-frame bytes, by the library's count.
    pub a_data_sent_bytes: Option<u64>,
    /// B's data-frame bytes, by the library's count.
    pub b_data_sent_bytes: Option<u64>,
    /// A's data frames.
    pub a_frames_sent: Option<u64>,
    /// B's data frames.
    pub b_frames_sent: Option<u64>,
    /// Disputed scopes, both sides together.
    pub disputed_scopes: Option<u64>,
    /// Messages A gained.
    pub a_gained: Option<u64>,
    /// Messages B gained.
    pub b_gained: Option<u64>,
    /// Messages A shed.
    pub a_shed: Option<u64>,
    /// Messages B shed.
    pub b_shed: Option<u64>,
    /// Units A held back for budget.
    pub a_budget_waits: Option<u64>,
    /// Units B held back for budget.
    pub b_budget_waits: Option<u64>,
    /// Live messages A holds after the session.
    pub a_live_after: Option<u64>,
    /// Live messages B holds after the session.
    pub b_live_after: Option<u64>,

    /// A's live heap when the session started.
    pub a_heap_before_bytes: Option<u64>,
    /// A's peak live heap during the session.
    pub a_heap_peak_bytes: Option<u64>,
    /// A's live heap when the session ended.
    pub a_heap_after_bytes: Option<u64>,
    /// B's live heap when the session started.
    pub b_heap_before_bytes: Option<u64>,
    /// B's peak live heap during the session.
    pub b_heap_peak_bytes: Option<u64>,
    /// B's live heap when the session ended.
    pub b_heap_after_bytes: Option<u64>,
    /// A's resident high-water mark during the session (Linux).
    pub a_rss_peak_bytes: Option<u64>,
    /// B's resident high-water mark during the session (Linux).
    pub b_rss_peak_bytes: Option<u64>,
    /// Segments A's namespace retransmitted during the session (Linux).
    pub a_retransmits: Option<u64>,
    /// Segments B's namespace retransmitted during the session (Linux).
    pub b_retransmits: Option<u64>,

    /// Why the session failed; empty when it finished.
    pub error: Option<String>,
}

/// Where and how a session ran, beyond its point.
#[derive(Clone, Copy, Debug, Default)]
pub struct Placement {
    /// The session's place in execution order.
    pub sequence: u64,
    /// Its timed slot.
    pub slot: usize,
    /// Sessions the run times at once.
    pub jobs: usize,
    /// The one-minute load average at its timed session's start.
    pub load_1m: Option<f64>,
    /// The two directions' queue limits, in emulation.
    pub queues: Option<(u64, u64)>,
    /// When the session's preparation ran.
    pub prepared: Option<Interval>,
    /// When its timed session ran.
    pub timed: Option<Interval>,
}

/// A stretch of time, in seconds since the Unix epoch.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Interval {
    /// When it began.
    pub began: f64,
    /// When it ended.
    pub ended: f64,
}

/// A rate's bits, or `None` when uncapped.
fn bits(rate: Rate) -> Option<u64> {
    match rate {
        Rate::Bits(bits) => Some(bits),
        Rate::Uncapped => None,
    }
}

impl Row {
    /// A row naming the session and its context, with no measurements.
    pub fn new(session: &Session, context: Placement) -> Self {
        let p = &session.point;
        let (link, rate_a_to_b_bits, rate_b_to_a_bits, rtt_us) = match p.link {
            Link::Emulated { rate, rtt } => (
                "emulated",
                bits(rate.a_to_b),
                bits(rate.b_to_a),
                Some(rtt.duration().as_micros() as u64),
            ),
            Link::Network => ("network", None, None, None),
        };
        Row {
            sequence: context.sequence,
            kind: session.kind.name().to_string(),
            group: session.group.clone(),
            point: p.key(),
            repetition: session.repetition,
            slot: context.slot,
            jobs: context.jobs,
            load_1m: context.load_1m,
            shared: p.replicas.shared,
            insert_a: p.replicas.insert_a,
            insert_b: p.replicas.insert_b,
            redact_a: p.replicas.redact_a,
            redact_b: p.replicas.redact_b,
            size_mean_bytes: p.sizes.mean,
            size_cv: p.sizes.cv,
            max_message_size_bytes: p.sizes.max,
            budget_a_bytes: p.settings.a.budget,
            budget_b_bytes: p.settings.b.budget,
            target_message_size_a_bytes: p.settings.a.target_message_size,
            target_message_size_b_bytes: p.settings.b.target_message_size,
            payload_depth_limit: p.settings.payload_depth_limit,
            session_deadline_seconds: p.settings.session_deadline.map(|d| d.as_secs_f64()),
            link: link.to_string(),
            rate_a_to_b_bits,
            rate_b_to_a_bits,
            rtt_us,
            connection: match p.connection {
                Connection::Fresh => "fresh",
                Connection::Warm => "warm",
            }
            .to_string(),
            tls: p.transport.tls,
            tcp_buffer_bytes: match p.tcp_buffers {
                TcpBuffers::Host => None,
                TcpBuffers::Raised(bytes) => Some(bytes),
            },
            queue_a_to_b_packets: context.queues.map(|q| q.0),
            queue_b_to_a_packets: context.queues.map(|q| q.1),
            prepare_began_unix_seconds: context.prepared.map(|i| i.began),
            prepare_ended_unix_seconds: context.prepared.map(|i| i.ended),
            timed_began_unix_seconds: context.timed.map(|i| i.began),
            timed_ended_unix_seconds: context.timed.map(|i| i.ended),
            ..Row::default()
        }
    }

    /// A row for a session that failed before it measured anything.
    pub fn failed(session: &Session, context: Placement, error: &anyhow::Error) -> Self {
        Row {
            error: Some(format!("{error:#}")),
            ..Row::new(session, context)
        }
    }

    /// A row for a measured session.
    pub fn measured(session: &Session, context: Placement, measured: &Measured) -> Self {
        let mut row = Row::new(session, context);
        let build = &measured.prepared.build;
        row.offset = Some(build.offset);
        row.built_messages = Some(build.messages);
        row.built_mean_bytes = Some(build.mean_message_bytes());
        row.redrawn_above_cap = Some(build.redrawn_above_cap);
        row.redrawn_below_one_byte = Some(build.redrawn_below_one_byte);
        row.build_seconds = Some(build.seconds);
        row.fetch_seconds = Some(measured.prepared.fetch_seconds);
        if let Some([a, b]) = measured.prepared.usage {
            row.a_prepare_rss_peak_bytes = Some(a.rss_peak);
            row.b_prepare_rss_peak_bytes = Some(b.rss_peak);
            row.a_prepare_cpu_seconds = Some(a.cpu_seconds);
            row.b_prepare_cpu_seconds = Some(b.cpu_seconds);
        }
        let (a, b): (&Outcome, &Outcome) = (&measured.a, &measured.b);
        row.wall_seconds = Some(b.wall_seconds);
        row.connect_seconds = Some(b.connect_seconds);
        row.a_wall_seconds = Some(a.wall_seconds);
        row.a_library_seconds = Some(a.stats.elapsed_seconds);
        row.b_library_seconds = Some(b.stats.elapsed_seconds);
        row.a_user_seconds = Some(a.usage.user_seconds);
        row.a_system_seconds = Some(a.usage.system_seconds);
        row.b_user_seconds = Some(b.usage.user_seconds);
        row.b_system_seconds = Some(b.usage.system_seconds);
        row.a_socket_sent_bytes = Some(a.socket.sent_bytes);
        row.b_socket_sent_bytes = Some(b.socket.sent_bytes);
        row.a_socket_writes = Some(a.socket.writes);
        row.b_socket_writes = Some(b.socket.writes);
        row.a_socket_reads = Some(a.socket.reads);
        row.b_socket_reads = Some(b.socket.reads);
        row.a_data_sent_bytes = Some(a.stats.bytes_sent);
        row.b_data_sent_bytes = Some(b.stats.bytes_sent);
        row.a_frames_sent = Some(a.stats.frames_sent);
        row.b_frames_sent = Some(b.stats.frames_sent);
        row.disputed_scopes = Some(a.stats.disputed_scopes + b.stats.disputed_scopes);
        row.a_gained = Some(a.stats.messages_gained);
        row.b_gained = Some(b.stats.messages_gained);
        row.a_shed = Some(a.stats.messages_shed);
        row.b_shed = Some(b.stats.messages_shed);
        row.a_budget_waits = Some(a.stats.budget_waits);
        row.b_budget_waits = Some(b.stats.budget_waits);
        row.a_effective_budget_bytes = Some(a.effective_budget);
        row.b_effective_budget_bytes = Some(b.effective_budget);
        row.a_live_after = Some(a.live_after);
        row.b_live_after = Some(b.live_after);
        row.a_heap_before_bytes = Some(a.usage.heap_before);
        row.a_heap_peak_bytes = Some(a.usage.heap_peak);
        row.a_heap_after_bytes = Some(a.usage.heap_after);
        row.b_heap_before_bytes = Some(b.usage.heap_before);
        row.b_heap_peak_bytes = Some(b.usage.heap_peak);
        row.b_heap_after_bytes = Some(b.usage.heap_after);
        row.a_rss_peak_bytes = a.usage.rss_peak;
        row.b_rss_peak_bytes = b.usage.rss_peak;
        row.a_retransmits = a.usage.retransmits;
        row.b_retransmits = b.usage.retransmits;
        let expected = session.point.replicas.shared
            + session.point.replicas.insert_a
            + session.point.replicas.insert_b;
        row.error = a
            .error
            .as_ref()
            .map(|e| format!("side A: {e}"))
            .or_else(|| b.error.as_ref().map(|e| format!("side B: {e}")))
            .or_else(|| {
                (a.live_after != expected || b.live_after != expected).then(|| {
                    format!(
                        "the replicas did not converge on {expected} messages: A holds {}, B holds {}",
                        a.live_after, b.live_after
                    )
                })
            });
        row
    }

    /// Whether the session finished and converged.
    pub fn ok(&self) -> bool {
        self.error.is_none() && self.wall_seconds.is_some()
    }
}

/// Appends rows to a run's `sessions.csv`, syncing each to the disk as it is
/// written, so a run cut short keeps every session it finished, even when the
/// host crashes or loses power.
#[derive(Debug)]
pub struct Writer {
    /// The CSV writer.
    csv: csv::Writer<File>,
}

impl Writer {
    /// Creates `path`, which the first row begins with the header.
    pub fn create(path: &Path) -> Result<Self> {
        let file = File::create(path).with_context(|| format!("creating {}", path.display()))?;
        sync_parent(path)?;
        Ok(Self {
            csv: csv::Writer::from_writer(file),
        })
    }

    /// Opens `path` to append after the first `whole` bytes, discarding the
    /// rest, which [`recover`] found torn.
    ///
    /// The first row written begins with the header when `whole` is zero, so
    /// the file reads as one written by [`Writer::create`].
    pub fn resume(path: &Path, whole: u64) -> Result<Self> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(path)
            .with_context(|| format!("opening {}", path.display()))?;
        file.set_len(whole)
            .and_then(|()| file.sync_data())
            .with_context(|| format!("truncating {} to its whole rows", path.display()))?;
        file.seek(SeekFrom::End(0))?;
        sync_parent(path)?;
        Ok(Self {
            csv: csv::WriterBuilder::new()
                .has_headers(whole == 0)
                .from_writer(file),
        })
    }

    /// Appends one row, returning once it is on the disk.
    ///
    /// A flush hands the row to the kernel, which would keep it through a
    /// killed process but not through a crash of the host; the sync makes it
    /// survive both, at one sync per session.
    pub fn append(&mut self, row: &Row) -> Result<()> {
        self.csv.serialize(row)?;
        self.csv.flush()?;
        self.csv.get_ref().sync_data()?;
        Ok(())
    }
}

/// Syncs the directory holding `path`, so a file just created in it, or renamed
/// into it, keeps its name through a crash of the host.
///
/// Only Unix platforms sync a directory through a handle opened on it;
/// elsewhere the file's own sync is all we can ask for.
pub fn sync_parent(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let parent = match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        File::open(parent)
            .and_then(|dir| dir.sync_all())
            .with_context(|| format!("syncing {}", parent.display()))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Reads every row of a `sessions.csv`.
pub fn read(path: &Path) -> Result<Vec<Row>> {
    let mut reader =
        csv::Reader::from_path(path).with_context(|| format!("reading {}", path.display()))?;
    reader
        .deserialize()
        .map(|row| row.with_context(|| format!("parsing a row of {}", path.display())))
        .collect()
}

/// The rows of a `sessions.csv` a run cut short left, as [`recover`] finds
/// them.
#[derive(Debug)]
pub struct Recovered {
    /// Every whole row, in the order the run wrote them.
    pub rows: Vec<Row>,
    /// The length in bytes of the file's whole part: the header and the whole
    /// rows, or zero when not even the header is whole.
    pub whole: u64,
    /// The length in bytes of what follows the whole part: a row, or the
    /// header, that the run was writing when it stopped.
    pub torn: u64,
}

/// The bytes this build writes for `row`, preceded by the header when `header`
/// is set.
fn encode(row: &Row, header: bool) -> Result<Vec<u8>> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(header)
        .from_writer(Vec::new());
    writer.serialize(row)?;
    writer
        .into_inner()
        .map_err(|error| anyhow::anyhow!("encoding a row: {}", error.error()))
}

/// The header line this build writes, its terminator included.
fn header() -> Result<Vec<u8>> {
    let mut bytes = encode(&Row::default(), true)?;
    let end = bytes
        .iter()
        .position(|&byte| byte == b'\n')
        .context("a header ends its line")?;
    bytes.truncate(end + 1);
    Ok(bytes)
}

/// Reads the `sessions.csv` at `path` for a run to continue, finding its whole
/// rows and any torn final row; an absent file holds no rows.
///
/// Each row is synced as it is written, so a crash of the host can tear only
/// the row being written when it struck, which is the file's last. A row is
/// *whole* when it parses and the bytes this build writes for it are exactly
/// the bytes the file holds for it, its line terminator included. That is the
/// only test both strict and complete: a prefix of a row can parse, as a row
/// whose last field is cut short, and a field that holds a quoted line break
/// can end a prefix at a line break, but neither reproduces its bytes. A
/// resumed run comes from the same build as the run it continues, so a whole
/// row always does.
///
/// # Errors
///
/// If the file cannot be read, does not begin with this build's header, or
/// holds a row that is not whole anywhere but at its end: none of those is what
/// a run cut short leaves.
pub fn recover(path: &Path) -> Result<Recovered> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(error).with_context(|| format!("reading {}", path.display())),
    };
    let header = header()?;
    if !bytes.starts_with(&header) {
        anyhow::ensure!(
            header.starts_with(&bytes),
            "{} does not begin with the header this build writes",
            path.display()
        );
        // The run stopped before its first row reached the disk whole.
        return Ok(Recovered {
            rows: Vec::new(),
            whole: 0,
            torn: bytes.len() as u64,
        });
    }
    let body = &bytes[header.len()..];
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(body);
    let mut rows = Vec::new();
    let mut whole = 0;
    let mut record = csv::ByteRecord::new();
    loop {
        let start = reader.position().byte() as usize;
        if !reader.read_byte_record(&mut record)? {
            break;
        }
        let end = reader.position().byte() as usize;
        let row = record
            .deserialize::<Row>(None)
            .ok()
            .filter(|row| encode(row, false).is_ok_and(|encoded| encoded == body[start..end]));
        match row {
            Some(row) => {
                rows.push(row);
                whole = end;
            }
            None if end == body.len() => break,
            None => anyhow::bail!(
                "{}: row {} is malformed, yet rows follow it; a run cut short tears only its last row",
                path.display(),
                rows.len() + 1
            ),
        }
    }
    Ok(Recovered {
        rows,
        whole: (header.len() + whole) as u64,
        torn: (body.len() - whole) as u64,
    })
}

#[cfg(test)]
mod tests;
