//! One measurement point: the two replicas, their message sizes, the `Peer`
//! settings each side runs with, and the link between them.
//!
//! # The replicas
//!
//! Two replicas, A and B, reconcile in one session. Five counts describe them:
//!
//! - `shared`, written *s*: messages live on both sides;
//! - `insert_a` and `insert_b` (*i_A*, *i_B*): messages one side inserted that
//!   the other has never seen;
//! - `redact_a` and `redact_b` (*r_A*, *r_B*): messages one side redacted that
//!   the other still holds live. Rumors keeps no tombstones, so the redacting
//!   side simply no longer holds them.
//!
//! So A holds *n_A = s + i_A + r_B* live messages and B holds *n_B = s + i_B +
//! r_A*; the keys only A holds number *k_A = i_A + r_B*, and those only B holds
//! *k_B = i_B + r_A*.
//!
//! # What the model predicts
//!
//! Under uniform hashing each message's position in the 256-ary tree is random,
//! and a few quantities follow from the counts alone. The tool uses them to
//! predict costs (for progress and scheduling) and to read each session (which
//! resource bound it); [`Point::model`] computes them, and each field's
//! documentation states its formula.

use std::fmt;
use std::sync::LazyLock;
use std::time::Duration;

use rumors::Peer;
use serde::{Deserialize, Serialize};

use crate::sizes::SizeDistribution;
use crate::units::{LinkRate, Rtt, format_bytes};

/// The message counts that describe a replica pair.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Replicas {
    /// Messages live on both sides.
    pub shared: u64,
    /// Messages only A inserted.
    pub insert_a: u64,
    /// Messages only B inserted.
    pub insert_b: u64,
    /// Messages A redacted that B still holds.
    pub redact_a: u64,
    /// Messages B redacted that A still holds.
    pub redact_b: u64,
}

impl Replicas {
    /// The live messages A holds: *s + i_A + r_B*.
    pub fn live_a(&self) -> u64 {
        self.shared + self.insert_a + self.redact_b
    }

    /// The live messages B holds: *s + i_B + r_A*.
    pub fn live_b(&self) -> u64 {
        self.shared + self.insert_b + self.redact_a
    }

    /// The keys only A holds: *i_A + r_B*.
    pub fn only_a(&self) -> u64 {
        self.insert_a + self.redact_b
    }

    /// The keys only B holds: *i_B + r_A*.
    pub fn only_b(&self) -> u64 {
        self.insert_b + self.redact_a
    }

    /// The messages A sends before the two replicas part: every shared message
    /// and every message one side later redacts.
    pub fn common(&self) -> u64 {
        self.shared + self.redact_a + self.redact_b
    }
}

impl fmt::Display for Replicas {
    /// Prints the counts compactly: `s=1e5 iA=1e3 iB=0 rA=0 rB=0`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "s={} iA={} iB={} rA={} rB={}",
            compact_count(self.shared),
            compact_count(self.insert_a),
            compact_count(self.insert_b),
            compact_count(self.redact_a),
            compact_count(self.redact_b)
        )
    }
}

/// Prints a count as `NeK` when it is a single digit times a power of ten of at
/// least a thousand, and in full otherwise.
pub fn compact_count(count: u64) -> String {
    if count >= 1_000 {
        let exponent = count.ilog10();
        let scale = 10u64.pow(exponent);
        if count.is_multiple_of(scale) {
            return format!("{}e{exponent}", count / scale);
        }
    }
    count.to_string()
}

/// The budget a grid's `unlimited` spelling requests: the largest byte count.
///
/// The run passes it to
/// [`Peer::sync_memory_budget`](rumors::Peer::sync_memory_budget) as
/// `usize::MAX`, so the session's pipelining memory has no bound below what the
/// host can allocate.
pub const UNLIMITED_BUDGET: u64 = u64::MAX;

/// The least budget the library runs a session on at the default message-size
/// target, to which it raises any smaller request.
///
/// The library publishes the minimum only as the budget a peer reports
/// ([`SynchronizationSettings`](rumors::SynchronizationSettings)), so this
/// reads it, once, from a peer that requests a budget of zero.
pub fn minimum_budget() -> u64 {
    static MINIMUM: LazyLock<u64> = LazyLock::new(|| {
        let budget = Peer::<()>::seed()
            .sync_memory_budget(0)
            .synchronization_settings()
            .sync_memory_budget;
        u64::try_from(budget).unwrap_or(u64::MAX)
    });
    *MINIMUM
}

/// A side's requested budget as text the grid parser reads back to the same
/// request: `default`, `unlimited`, or a byte count.
pub fn budget_text(budget: Option<u64>) -> String {
    match budget {
        None => "default".into(),
        Some(UNLIMITED_BUDGET) => "unlimited".into(),
        Some(bytes) => format_bytes(bytes),
    }
}

/// The settings one side's `Peer` runs with; `None` keeps the library's
/// default, which the record names as such.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SideSettings {
    /// [`Peer::sync_memory_budget`](rumors::Peer::sync_memory_budget), in
    /// bytes.
    pub budget: Option<u64>,
    /// [`Peer::target_message_size`](rumors::Peer::target_message_size), in
    /// bytes.
    pub target_message_size: Option<u64>,
}

/// Every setting `Peer` offers that can change a session, for both sides.
///
/// The payload depth limit and the session deadline apply to both sides: the
/// library requires the depth limit to agree across a network, and the deadline
/// is a property of the measurement, not of one side.
/// [`Peer::bookmark_size_limit`](rumors::Peer::bookmark_size_limit) is not here
/// because the tool's peers keep no bookmark, and the gossip policy and
/// observers are not here because a measured session is one explicit exchange
/// with nothing observing it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Settings {
    /// Side A's settings.
    pub a: SideSettings,
    /// Side B's settings.
    pub b: SideSettings,
    /// [`Peer::payload_depth_limit`](rumors::Peer::payload_depth_limit).
    pub payload_depth_limit: Option<u64>,
    /// [`Peer::session_deadline`](rumors::Peer::session_deadline): a session
    /// running longer fails and is recorded as failed.
    pub session_deadline: Option<Duration>,
}

/// The link between the two peers.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Link {
    /// An emulated link: each direction shaped to its rate and delayed by half
    /// the round trip.
    Emulated {
        /// The rate in each direction.
        rate: LinkRate,
        /// The round-trip time the link adds.
        rtt: Rtt,
    },
    /// The network between two hosts, unshaped.
    Network,
}

impl fmt::Display for Link {
    /// Prints `100Mbit 10ms`, or `network`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Link::Emulated { rate, rtt } => write!(f, "{rate} {rtt}"),
            Link::Network => f.write_str("network"),
        }
    }
}

/// Whether a session runs on a connection already open and warm, or on one it
/// opens.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Connection {
    /// A new TCP connection: the session pays the handshake and slow start, as
    /// for an application that connects in order to synchronize.
    Fresh,
    /// A TCP connection already open and warm, as on the long-lived connection
    /// an application keeps with `Rumors::gossip`.
    ///
    /// It has carried a bulk transfer each way, so its congestion window is
    /// open when the session starts, and the session's clock starts after setup
    /// and warm-up.
    #[default]
    Warm,
}

impl std::str::FromStr for Connection {
    type Err = anyhow::Error;

    /// Parses `fresh` or `warm`.
    fn from_str(text: &str) -> anyhow::Result<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "fresh" => Ok(Connection::Fresh),
            "warm" => Ok(Connection::Warm),
            _ => anyhow::bail!("`{text}` is not a connection mode: write `fresh` or `warm`"),
        }
    }
}

impl fmt::Display for Connection {
    /// Prints `fresh` or `warm`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Connection::Fresh => "fresh",
            Connection::Warm => "warm",
        })
    }
}

/// The stack each side runs a session over, from the TCP socket up to the
/// halves it hands the library.
///
/// Every stack follows the library's transport advice: Nagle's algorithm off on
/// both ends, and the read half wrapped in a `BufReader`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Transport {
    /// Whether the session runs over TLS 1.3 (rustls) on the connection.
    pub tls: bool,
}

impl Default for Transport {
    /// TLS 1.3 over TCP, the stack deployments run sessions over.
    fn default() -> Self {
        Transport { tls: true }
    }
}

impl fmt::Display for Transport {
    /// Prints the stack: `tcp` or `tls`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.tls { "tls" } else { "tcp" })
    }
}

/// Parses an `on` or `off` axis value.
pub fn switch(text: &str) -> anyhow::Result<bool> {
    match text.trim().to_ascii_lowercase().as_str() {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => anyhow::bail!("`{text}` is not a switch: write `on` or `off`"),
    }
}

/// The TCP buffer limits a session's two network namespaces run with.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum TcpBuffers {
    /// The host's limits, as the kernel gives every new namespace.
    #[default]
    Host,
    /// The largest autotuned send and receive buffer raised to this many bytes
    /// inside the session's namespaces only.
    Raised(u64),
}

impl fmt::Display for TcpBuffers {
    /// Prints `host`, or `raised to N`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TcpBuffers::Host => f.write_str("host"),
            TcpBuffers::Raised(bytes) => write!(f, "raised to {}", format_bytes(*bytes)),
        }
    }
}

/// One measurement point.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    /// The replica pair.
    pub replicas: Replicas,
    /// The message sizes.
    pub sizes: SizeDistribution,
    /// The `Peer` settings.
    pub settings: Settings,
    /// The link.
    pub link: Link,
    /// Fresh or warm connection.
    pub connection: Connection,
    /// The stack over the connection.
    pub transport: Transport,
    /// The TCP buffer limits.
    pub tcp_buffers: TcpBuffers,
}

impl Point {
    /// A stable text key naming the point, the same in every run: the report
    /// groups sessions by it and `--shard` hashes it.
    pub fn key(&self) -> String {
        let side = |s: &SideSettings| {
            format!(
                "{}/{}",
                budget_text(s.budget),
                s.target_message_size.map_or("default".into(), format_bytes)
            )
        };
        format!(
            "{} | m={}~{}<={} | A {} B {} depth {} deadline {} | {} | {} | {} | tcp {}",
            self.replicas,
            self.sizes.mean,
            self.sizes.cv,
            self.sizes.max,
            side(&self.settings.a),
            side(&self.settings.b),
            self.settings
                .payload_depth_limit
                .map_or("default".into(), |d| d.to_string()),
            self.settings
                .session_deadline
                .map_or("none".into(), |d| format!("{}ms", d.as_millis())),
            self.link,
            self.connection,
            self.transport,
            self.tcp_buffers
        )
    }

    /// The model's predictions for this point.
    pub fn model(&self) -> Model {
        Model::of(&self.replicas, self.sizes.mean)
    }
}

/// One-way delays a session spends on its handshake and completion beyond its
/// descent of the tree: three, as measured on the library's model of a session
/// over a latency-only link.
pub const PROTOCOL_HOPS: f64 = 3.0;

/// Bytes a supplied message adds on the wire beyond its own encoding: its
/// version and the record's framing. An estimate for scheduling only; the
/// report reads measured bytes.
const SUPPLY_OVERHEAD_BYTES: f64 = 40.0;

/// Bytes the comparison spends per differing key, in each direction: the
/// questions and listings that find it. An estimate for scheduling only.
const COMPARISON_BYTES_PER_KEY: f64 = 24.0;

/// What uniform hashing predicts for a replica pair.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Model {
    /// *K = n_A·n_B − s²*: the pairs of keys, one on each side, that are not
    /// two copies of one shared key. The comparison's depth depends on the
    /// replicas only through it.
    pub key_pairs: f64,
    /// The expected depth of the deepest disputed node: with
    /// `P(D ≥ ℓ) = 1 − exp(−K / 256^ℓ)`, the sum of those probabilities over ℓ
    /// ≥ 1.
    pub expected_depth: f64,
    /// The one-way delays the session needs on a link with no bandwidth limit:
    /// one per generation of the descent, which reaches one or two levels below
    /// the deepest disputed node, and [`PROTOCOL_HOPS`] more.
    ///
    /// A join from an empty replica needs one generation; identical replicas
    /// none.
    pub hops: f64,
    /// Estimated bytes A sends.
    pub bytes_a: f64,
    /// Estimated bytes B sends.
    pub bytes_b: f64,
}

impl Model {
    /// The predictions for `replicas` with messages of `mean_size` bytes.
    pub fn of(replicas: &Replicas, mean_size: f64) -> Self {
        let n_a = replicas.live_a() as f64;
        let n_b = replicas.live_b() as f64;
        let s = replicas.shared as f64;
        let key_pairs = n_a * n_b - s * s;
        let expected_depth = (1..=32)
            .map(|level| 1.0 - (-key_pairs / 256f64.powi(level)).exp())
            .sum::<f64>();
        let generations = if key_pairs == 0.0 && replicas.only_a() + replicas.only_b() == 0 {
            0.0
        } else if replicas.live_a() == 0 || replicas.live_b() == 0 {
            1.0
        } else {
            expected_depth + 1.5
        };
        let differing = (replicas.only_a() + replicas.only_b()) as f64;
        let comparison = differing * COMPARISON_BYTES_PER_KEY;
        let supply = mean_size + SUPPLY_OVERHEAD_BYTES;
        Self {
            key_pairs,
            expected_depth,
            hops: generations + PROTOCOL_HOPS,
            bytes_a: replicas.insert_a as f64 * supply + comparison,
            bytes_b: replicas.insert_b as f64 * supply + comparison,
        }
    }

    /// The busier direction's estimated bytes.
    pub fn busier_bytes(&self) -> f64 {
        self.bytes_a.max(self.bytes_b)
    }
}

#[cfg(test)]
mod tests;
