//! Grids: which points a run measures, and how often.
//!
//! A grid is a list of *groups*, and each group is the Cartesian product
//! of its *axes*: every combination of one value from each axis is a
//! point. Any setting can be an axis, the replica counts, message sizes,
//! `Peer` settings, and the link alike; [`AXES`] lists them. An axis left
//! out of a group takes its value from the file's `[defaults]` table, or
//! else the tool's default.
//!
//! Some settings move together, such as the counts that describe "a
//! million messages, 90% shared". A group's `pairs` array holds such
//! combinations: each entry is a table of settings applied together, and
//! the array is one axis of the product. An entry describes its replica
//! pair whole: any of the five counts it leaves out is zero, so
//! `{ shared = 0, insert_a = 20000 }` is a peer joining from empty, not
//! one that also inherits the default insertions on side B.
//!
//! The command line describes one group, with each axis flag taking a
//! comma-separated list (`--rtt 1ms,10ms,100ms`).
//!
//! ```toml
//! repetitions = 5
//!
//! [defaults]
//! size_mean = 100
//!
//! [[group]]
//! name = "catch-up"
//! pairs = [
//!     { shared = 100000, insert_a = 1000 },
//!     { shared = 1000000, insert_a = 100000 },
//! ]
//! rate = ["100Mbit", "1Gbit"]
//! rtt = ["1ms", "100ms"]
//! budget = ["default", "32MiB"]
//! ```

use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail, ensure};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

use crate::point::{
    Connection, Link, Point, Replicas, Settings, TcpBuffers, Transport, UNLIMITED_BUDGET, switch,
};
use crate::sizes::SizeDistribution;
use crate::units::{LinkRate, Rate, Rtt, parse_bytes, parse_count};

/// Every axis a grid can sweep, with what it sets.
pub const AXES: &[(&str, &str)] = &[
    ("shared", "messages live on both sides (s)"),
    ("insert_a", "messages only A inserted (i_A)"),
    ("insert_b", "messages only B inserted (i_B)"),
    ("insert", "sets insert_a and insert_b alike"),
    ("redact_a", "messages A redacted that B still holds (r_A)"),
    ("redact_b", "messages B redacted that A still holds (r_B)"),
    ("redact", "sets redact_a and redact_b alike"),
    ("size_mean", "mean encoded message size, bytes"),
    (
        "size_cv",
        "message-size coefficient of variation; 0 for a fixed size",
    ),
    (
        "max_message_size",
        "message-size cap, bytes; larger draws are redrawn",
    ),
    (
        "budget",
        "Peer::sync_memory_budget for both sides, bytes, `default`, or `unlimited`",
    ),
    ("budget_a", "side A's sync_memory_budget"),
    ("budget_b", "side B's sync_memory_budget"),
    (
        "target_message_size",
        "Peer::target_message_size for both sides, bytes or `default`",
    ),
    ("target_message_size_a", "side A's target_message_size"),
    ("target_message_size_b", "side B's target_message_size"),
    (
        "payload_depth_limit",
        "Peer::payload_depth_limit, or `default`",
    ),
    (
        "session_deadline",
        "Peer::session_deadline, a duration such as `60s`, or `none`",
    ),
    (
        "rate",
        "link rate per direction: `100Mbit`, `uncapped`, or `A/B` such as `100Mbit/10Mbit`",
    ),
    ("rtt", "link round-trip time, such as `10ms`"),
    (
        "connection",
        "`warm` (the default): a connection already open and past slow start, as `gossip` keeps; `fresh`: the session opens its connection",
    ),
    (
        "tls",
        "`on` (the default) to run each session over TLS 1.3 on its TCP connection, `off` for plain TCP",
    ),
];

/// A point under construction from axis values.
#[derive(Clone, Copy, Debug)]
struct Draft(Point);

impl Default for Draft {
    /// The tool's defaults: 100,000 shared messages, 1,000 inserted on each
    /// side, fixed 100-byte messages, the library's settings, and a warm
    /// connection over 1 Gbit/s with a 1 ms round trip.
    fn default() -> Self {
        Draft(Point {
            replicas: Replicas {
                shared: 100_000,
                insert_a: 1_000,
                insert_b: 1_000,
                redact_a: 0,
                redact_b: 0,
            },
            sizes: SizeDistribution::default(),
            settings: Settings::default(),
            link: Link::Emulated {
                rate: LinkRate::symmetric(Rate::Bits(1_000_000_000)),
                rtt: Rtt::from_duration(Duration::from_millis(1)),
            },
            connection: Connection::Warm,
            transport: Transport::default(),
            tcp_buffers: TcpBuffers::Host,
        })
    }
}

/// Parses an optional byte count: `default` for the library's default.
fn optional_bytes(text: &str) -> Result<Option<u64>> {
    if text.trim().eq_ignore_ascii_case("default") {
        Ok(None)
    } else {
        parse_bytes(text).map(Some)
    }
}

/// Parses a requested budget: `default` for the library's default,
/// `unlimited` for [`UNLIMITED_BUDGET`], or a byte count.
fn budget(text: &str) -> Result<Option<u64>> {
    if text.trim().eq_ignore_ascii_case("unlimited") {
        Ok(Some(UNLIMITED_BUDGET))
    } else {
        optional_bytes(text)
    }
}

/// Parses a finite, non-negative number.
fn number(text: &str) -> Result<f64> {
    let value: f64 = text
        .trim()
        .parse()
        .with_context(|| format!("`{text}` is not a number"))?;
    ensure!(
        value.is_finite() && value >= 0.0,
        "`{text}` must be a finite, non-negative number"
    );
    Ok(value)
}

impl Draft {
    /// Sets one axis from its text.
    fn set(&mut self, axis: &str, text: &str) -> Result<()> {
        let p = &mut self.0;
        match axis {
            "shared" => p.replicas.shared = parse_count(text)?,
            "insert_a" => p.replicas.insert_a = parse_count(text)?,
            "insert_b" => p.replicas.insert_b = parse_count(text)?,
            "insert" => {
                p.replicas.insert_a = parse_count(text)?;
                p.replicas.insert_b = p.replicas.insert_a;
            }
            "redact_a" => p.replicas.redact_a = parse_count(text)?,
            "redact_b" => p.replicas.redact_b = parse_count(text)?,
            "redact" => {
                p.replicas.redact_a = parse_count(text)?;
                p.replicas.redact_b = p.replicas.redact_a;
            }
            "size_mean" => p.sizes.mean = number(text)?,
            "size_cv" => p.sizes.cv = number(text)?,
            "max_message_size" => p.sizes.max = parse_bytes(text)?,
            "budget" => {
                p.settings.a.budget = budget(text)?;
                p.settings.b.budget = p.settings.a.budget;
            }
            "budget_a" => p.settings.a.budget = budget(text)?,
            "budget_b" => p.settings.b.budget = budget(text)?,
            "target_message_size" => {
                p.settings.a.target_message_size = optional_bytes(text)?;
                p.settings.b.target_message_size = p.settings.a.target_message_size;
            }
            "target_message_size_a" => p.settings.a.target_message_size = optional_bytes(text)?,
            "target_message_size_b" => p.settings.b.target_message_size = optional_bytes(text)?,
            "payload_depth_limit" => {
                p.settings.payload_depth_limit = if text.trim().eq_ignore_ascii_case("default") {
                    None
                } else {
                    Some(parse_count(text)?)
                }
            }
            "session_deadline" => {
                p.settings.session_deadline = if text.trim().eq_ignore_ascii_case("none") {
                    None
                } else {
                    Some(text.parse::<Rtt>()?.duration())
                }
            }
            "rate" | "rtt" => {
                let Link::Emulated { rate, rtt } = &mut p.link else {
                    unreachable!("drafts are emulated until pair mode rewrites them");
                };
                if axis == "rate" {
                    *rate = text.parse()?;
                } else {
                    *rtt = text.parse()?;
                }
            }
            "connection" => p.connection = text.parse()?,
            "tls" => p.transport.tls = switch(text)?,
            _ => bail!(
                "`{axis}` is not an axis; the axes are: {}",
                AXES.iter()
                    .map(|(name, _)| *name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
        Ok(())
    }
}

/// One axis: its name and the values it takes, as text.
pub type Axis = (String, Vec<String>);

/// One group: its name, its axes, its combined settings, and its filter.
#[derive(Clone, Debug, Default)]
pub struct Group {
    /// The group's name in the report.
    pub name: String,
    /// The axes, each a name and its values.
    pub axes: Vec<Axis>,
    /// Combinations of settings applied together, as one more axis.
    pub pairs: Vec<Vec<(String, String)>>,
    /// Points whose busier direction the model predicts to take longer
    /// than this many seconds on the wire alone are left out.
    pub max_wire_seconds: Option<f64>,
}

/// A grid: its groups and the defaults beneath them.
#[derive(Clone, Debug, Default)]
pub struct Grid {
    /// Sessions per point.
    pub repetitions: Option<u32>,
    /// Settings every group starts from.
    pub defaults: Vec<(String, String)>,
    /// The groups.
    pub groups: Vec<Group>,
}

/// A grid value's text: strings as written, numbers in full.
fn value_text(value: &toml::Value) -> Result<String> {
    Ok(match value {
        toml::Value::String(text) => text.clone(),
        toml::Value::Integer(integer) => integer.to_string(),
        toml::Value::Float(float) => float.to_string(),
        other => bail!("`{other}` is not a setting: write a string or a number"),
    })
}

/// A table of settings as name and text pairs.
fn settings(table: &toml::Table) -> Result<Vec<(String, String)>> {
    table
        .iter()
        .map(|(key, value)| {
            Ok((
                key.clone(),
                value_text(value).with_context(|| format!("setting `{key}`"))?,
            ))
        })
        .collect()
}

impl Grid {
    /// Parses a grid file's text.
    ///
    /// # Errors
    ///
    /// If the text is not TOML, a key is not an axis, or a value does not
    /// parse for its axis; the error names the place.
    pub fn parse(text: &str) -> Result<Self> {
        let table: toml::Table = text.parse().context("the grid file is not valid TOML")?;
        let mut grid = Grid::default();
        for (key, value) in &table {
            match (key.as_str(), value) {
                ("repetitions", toml::Value::Integer(n)) => {
                    grid.repetitions = Some(
                        u32::try_from(*n)
                            .ok()
                            .filter(|n| *n > 0)
                            .ok_or_else(|| anyhow!("`repetitions` must be a positive integer"))?,
                    )
                }
                ("defaults", toml::Value::Table(defaults)) => grid.defaults = settings(defaults)?,
                ("group", toml::Value::Array(groups)) => {
                    for (index, group) in groups.iter().enumerate() {
                        let toml::Value::Table(group) = group else {
                            bail!("each `[[group]]` must be a table");
                        };
                        grid.groups.push(
                            Group::parse(group)
                                .with_context(|| format!("in group {}", index + 1))?,
                        );
                    }
                }
                _ => bail!(
                    "unexpected top-level key `{key}`: a grid file has `repetitions`, `[defaults]`, and `[[group]]` tables"
                ),
            }
        }
        ensure!(!grid.groups.is_empty(), "the grid file has no `[[group]]`");
        grid.points()?;
        Ok(grid)
    }

    /// Every point of every group, each with its group's name, in grid
    /// order, less the points [`plan`](Self::plan) skips.
    ///
    /// # Errors
    ///
    /// As [`plan`](Self::plan).
    pub fn points(&self) -> Result<Vec<(String, Point)>> {
        Ok(self.plan()?.0)
    }

    /// Every point of every group, each with its group's name, in grid
    /// order, and apart from them the points their group's
    /// `max_wire_seconds` skips, each with its reason.
    ///
    /// Together the two lists hold every combination of every group's axes
    /// exactly once.
    ///
    /// # Errors
    ///
    /// If a value does not parse for its axis, or a point's message sizes
    /// are impossible.
    #[allow(clippy::type_complexity)]
    pub fn plan(&self) -> Result<(Vec<(String, Point)>, Vec<Skip>)> {
        let mut base = Draft::default();
        for (axis, text) in &self.defaults {
            base.set(axis, text)
                .with_context(|| format!("in `[defaults]`, `{axis}`"))?;
        }
        let mut points = Vec::new();
        let mut skipped = Vec::new();
        for group in &self.groups {
            let mut drafts = vec![base];
            for (axis, values) in &group.axes {
                let mut next = Vec::with_capacity(drafts.len() * values.len());
                for draft in &drafts {
                    for value in values {
                        let mut draft = *draft;
                        draft
                            .set(axis, value)
                            .with_context(|| format!("in group `{}`, axis `{axis}`", group.name))?;
                        next.push(draft);
                    }
                }
                drafts = next;
            }
            if !group.pairs.is_empty() {
                let mut next = Vec::with_capacity(drafts.len() * group.pairs.len());
                for draft in &drafts {
                    for pair in &group.pairs {
                        let mut draft = *draft;
                        draft.0.replicas = Replicas::default();
                        for (axis, value) in pair {
                            draft.set(axis, value).with_context(|| {
                                format!("in group `{}`, a `pairs` entry's `{axis}`", group.name)
                            })?;
                        }
                        next.push(draft);
                    }
                }
                drafts = next;
            }
            for Draft(point) in drafts {
                point
                    .sizes
                    .validate()
                    .with_context(|| format!("in group `{}`", group.name))?;
                if let Some(limit) = group.max_wire_seconds {
                    let seconds = predicted_wire_seconds(&point);
                    if seconds > limit {
                        skipped.push(Skip {
                            group: group.name.clone(),
                            point: point.key(),
                            reason: format!(
                                "the busier direction needs an estimated {seconds:.1} s on the wire alone, \
                                 above the group's max_wire_seconds of {limit}"
                            ),
                        });
                        continue;
                    }
                }
                points.push((group.name.clone(), point));
            }
        }
        Ok((points, skipped))
    }
}

/// A point its group's `max_wire_seconds` leaves out of a run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Skip {
    /// The point's group.
    pub group: String,
    /// The point's key.
    pub point: String,
    /// Why the point is skipped, in words.
    pub reason: String,
}

impl Group {
    /// Parses one `[[group]]` table.
    fn parse(table: &toml::Table) -> Result<Self> {
        let mut group = Group {
            name: "group".to_string(),
            ..Group::default()
        };
        for (key, value) in table {
            match key.as_str() {
                "name" => group.name = value_text(value)?,
                "max_wire_seconds" => {
                    group.max_wire_seconds = Some(number(&value_text(value)?)?);
                }
                "pairs" => {
                    let toml::Value::Array(entries) = value else {
                        bail!("`pairs` must be an array of tables");
                    };
                    for entry in entries {
                        let toml::Value::Table(entry) = entry else {
                            bail!(
                                "each `pairs` entry must be a table, such as {{ shared = 1000, insert_a = 10 }}"
                            );
                        };
                        group.pairs.push(settings(entry)?);
                    }
                }
                axis => {
                    let values = match value {
                        toml::Value::Array(values) => {
                            values.iter().map(value_text).collect::<Result<Vec<_>>>()?
                        }
                        scalar => vec![value_text(scalar)?],
                    };
                    ensure!(!values.is_empty(), "axis `{axis}` has no values");
                    group.axes.push((axis.to_string(), values));
                }
            }
        }
        Ok(group)
    }
}

/// The seconds the model predicts the busier direction spends on the wire
/// alone: its bytes over its rate, or zero on an uncapped link or between two hosts.
pub fn predicted_wire_seconds(point: &Point) -> f64 {
    let Link::Emulated { rate, .. } = point.link else {
        return 0.0;
    };
    let model = point.model();
    let seconds = |bytes: f64, rate: Rate| rate.bytes_per_second().map_or(0.0, |r| bytes / r);
    seconds(model.bytes_a, rate.a_to_b).max(seconds(model.bytes_b, rate.b_to_a))
}

/// What a session is for.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// A point of the grid.
    Grid,
    /// A grid point rerun with raised TCP buffer limits.
    Raised,
    /// The calibration's sentinel point, repeated through the run to detect
    /// drift.
    Sentinel,
}

impl Kind {
    /// The kind's name in `sessions.csv` and `plan.json`.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Grid => "grid",
            Kind::Raised => "raised",
            Kind::Sentinel => "sentinel",
        }
    }
}

/// One session to run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    /// The point.
    pub point: Point,
    /// Its group.
    pub group: String,
    /// What it is for.
    pub kind: Kind,
    /// Which repetition of its point it is, from zero.
    pub repetition: u32,
}

/// The largest TCP buffer limit a rerun raises to: 1 GiB. Linux caps one
/// socket buffer at `INT_MAX`, and a window of half this size covers a
/// 16 Gbit/s link at 250 ms.
pub const MAX_RAISED_BUFFER: u64 = 1 << 30;

/// The buffer limit a rerun of `point` needs, if `usable_window` cannot
/// hold what the session could keep in flight.
///
/// A connection can keep at most one window in flight per round trip, and
/// a session has no more to send than its busier direction's bytes, so the
/// window binds exactly when it is smaller than both the link's
/// bandwidth-delay product and those bytes. The raised limit holds twice
/// the larger of the two over the payload share, rounded up to a power of
/// two, so the window that limit allows covers it with room to spare.
pub fn raised_buffer(point: &Point, usable_window: u64, payload_share: f64) -> Option<u64> {
    let Link::Emulated { rate, rtt } = point.link else {
        return None;
    };
    let busier = point.model().busier_bytes();
    let in_flight = match rate.fastest_bytes_per_second() {
        Some(bytes_per_second) => (bytes_per_second * rtt.seconds()).min(busier),
        None => busier,
    };
    if in_flight <= usable_window as f64 {
        return None;
    }
    let needed = (2.0 * in_flight / payload_share.max(f64::MIN_POSITIVE)).ceil() as u64;
    Some(needed.max(1).next_power_of_two().min(MAX_RAISED_BUFFER))
}

/// The sessions of a grid: every point `repetitions` times, and every point
/// that needs raised TCP buffers once more at the raised limit, as many
/// times.
pub fn sessions(
    points: &[(String, Point)],
    repetitions: u32,
    tcp_window: Option<(u64, f64)>,
) -> Vec<Session> {
    let mut sessions = Vec::new();
    for (group, point) in points {
        let raised = tcp_window.and_then(|(window, share)| raised_buffer(point, window, share));
        let variants = std::iter::once((Kind::Grid, *point)).chain(raised.map(|bytes| {
            (
                Kind::Raised,
                Point {
                    tcp_buffers: TcpBuffers::Raised(bytes),
                    ..*point
                },
            )
        }));
        for (kind, point) in variants {
            for repetition in 0..repetitions {
                sessions.push(Session {
                    point,
                    group: group.clone(),
                    kind,
                    repetition,
                });
            }
        }
    }
    sessions
}

/// A deterministic share of a run, `index` of `count` (from one).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Shard {
    /// This shard, from one.
    pub index: u32,
    /// The number of shards.
    pub count: u32,
}

impl std::str::FromStr for Shard {
    type Err = anyhow::Error;

    /// Parses `k/n`, with 1 ≤ k ≤ n.
    fn from_str(text: &str) -> Result<Self> {
        let (index, count) = text
            .split_once('/')
            .ok_or_else(|| anyhow!("`{text}` is not a shard: write `k/n`, such as `2/4`"))?;
        let index: u32 = index
            .trim()
            .parse()
            .context("the shard index is not a number")?;
        let count: u32 = count
            .trim()
            .parse()
            .context("the shard count is not a number")?;
        ensure!(
            count >= 1 && (1..=count).contains(&index),
            "a shard `k/n` needs 1 ≤ k ≤ n"
        );
        Ok(Shard { index, count })
    }
}

impl Shard {
    /// Whether `session` belongs to this shard: a hash of its point's key,
    /// kind, and repetition, the same on every host, picks one shard of
    /// `count`.
    pub fn owns(&self, session: &Session) -> bool {
        let mut hasher = Sha3_256::new();
        hasher.update(session.point.key().as_bytes());
        hasher.update(format!("{:?}/{}", session.kind, session.repetition).as_bytes());
        let digest = hasher.finalize();
        let value = u64::from_le_bytes(digest[..8].try_into().expect("a digest has eight bytes"));
        value % u64::from(self.count) == u64::from(self.index - 1)
    }
}

/// Counts points by group, for a plan's summary.
pub fn by_group(sessions: &[Session]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for session in sessions {
        *counts.entry(session.group.clone()).or_insert(0) += 1;
    }
    counts
}

#[cfg(test)]
mod tests;
