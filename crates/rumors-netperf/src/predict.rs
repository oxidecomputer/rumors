//! Predicting what a session costs, for progress, scheduling, and sizing
//! the sentinels.
//!
//! Sessions differ in cost by orders of magnitude, so progress counted in
//! sessions says little. The run counts progress in predicted seconds
//! instead: each session contributes its prediction when it finishes, and
//! the estimated time remaining is the remaining prediction at the rate the
//! run has been retiring it. A prediction that is off by a constant factor
//! still yields a sound fraction and estimate.
//!
//! A session's prediction has two parts. Its *preparation* starts the two
//! processes and joins their link, builds the pair, and moves side B's
//! replica ([`CostModel::prepare_seconds`]); its *timed session* is the
//! gossip the row measures ([`CostModel::session_seconds`]). A run
//! prepares many sessions at once and times them apart from any
//! preparation ([`crate::schedule`]), so the run's wall time is not the
//! sum of its sessions' predictions: [`crate::schedule::simulate`] plays
//! the run's schedule through with these predictions to estimate it.
//!
//! Calibration measures the rates on the host ([`CostModel::fit`] and
//! [`MemoryModel::fit`]); before it has, the defaults below stand in, and
//! the tool predicts no wall time from them.

use serde::{Deserialize, Serialize};

use crate::fixture::OFFSET_RANGE;
use crate::point::{Connection, Link, Point};

/// The payload of one full-sized TCP segment.
const SEGMENT_BYTES: f64 = 1448.0;

/// Segments in Linux's initial congestion window.
const INITIAL_WINDOW_SEGMENTS: f64 = 10.0;

/// Per-message rates of the work a session's processes do on this host.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CostModel {
    /// Seconds side A spends per message it builds into the pair, while as
    /// many preparations run at once as the run allows.
    pub build_seconds_per_message: f64,
    /// Seconds side B spends per message it joins, while as many
    /// preparations run at once as the run allows.
    pub fetch_seconds_per_message: f64,
    /// Seconds the session spends per message held by one side only.
    pub session_seconds_per_message: f64,
    /// Seconds a preparation spends beyond its messages: starting two
    /// processes and setting up their link.
    pub overhead_seconds: f64,
}

impl Default for CostModel {
    /// Rates of the order measured on a current laptop processor, for use
    /// until calibration measures this host's.
    fn default() -> Self {
        Self {
            build_seconds_per_message: 6e-6,
            fetch_seconds_per_message: 6e-6,
            session_seconds_per_message: 10e-6,
            overhead_seconds: 0.1,
        }
    }
}

/// The messages side A sends while building `point`'s pair: the offset,
/// the common messages, the insertions on both sides, and the common
/// messages again as B joins.
fn build_messages(point: &Point) -> f64 {
    let r = &point.replicas;
    (OFFSET_RANGE.end + 2 * r.common() + r.insert_a + r.insert_b) as f64
}

impl CostModel {
    /// The predicted wall seconds of the measured session alone.
    pub fn session_seconds(&self, point: &Point) -> f64 {
        let model = point.model();
        let compute = self.session_seconds_per_message
            * (point.replicas.only_a() + point.replicas.only_b()) as f64;
        let Link::Emulated { rate, rtt } = point.link else {
            return compute;
        };
        let wire = |bytes: f64, rate: crate::units::Rate| {
            rate.bytes_per_second().map_or(0.0, |r| bytes / r)
        };
        let wire = wire(model.bytes_a, rate.a_to_b).max(wire(model.bytes_b, rate.b_to_a));
        let rtt = rtt.seconds();
        let mut seconds = wire.max(compute) + model.hops * rtt / 2.0;
        if point.connection == Connection::Fresh {
            let bdp = rate
                .fastest_bytes_per_second()
                .map_or(f64::INFINITY, |r| r * rtt);
            let in_flight = model.busier_bytes().min(bdp);
            let slow_start_rtts =
                (1.0 + in_flight / (INITIAL_WINDOW_SEGMENTS * SEGMENT_BYTES)).log2();
            seconds += rtt * (1.0 + slow_start_rtts);
        }
        seconds
    }

    /// The predicted wall seconds a timed session on a warm connection
    /// spends before its clock starts.
    ///
    /// That is connecting, the TLS handshake if it runs over TLS, and
    /// carrying `warm_bytes` each way, one direction after the other,
    /// through a window of at most `window` bytes. It is zero on a fresh
    /// connection, whose session time already includes its setup, and
    /// between hosts.
    ///
    /// Each direction takes the slow-start round trips that open the
    /// window to what one round trip carries, then its bytes at the rate a
    /// window that size sustains, or the link's rate if that is lower.
    pub fn warm_seconds(&self, point: &Point, warm_bytes: u64, window: u64) -> f64 {
        let Link::Emulated { rate, rtt } = point.link else {
            return 0.0;
        };
        if point.connection == Connection::Fresh {
            return 0.0;
        }
        let rtt = rtt.seconds();
        let handshakes = if point.transport.tls { 2.0 } else { 1.0 };
        let direction = |rate: crate::units::Rate| {
            let windowed = window as f64 / rtt.max(f64::MIN_POSITIVE);
            let throughput = rate
                .bytes_per_second()
                .map_or(windowed, |r| r.min(windowed));
            let in_flight = (throughput * rtt).min(window as f64);
            let slow_start_rtts =
                (1.0 + in_flight / (INITIAL_WINDOW_SEGMENTS * SEGMENT_BYTES)).log2();
            slow_start_rtts * rtt + warm_bytes as f64 / throughput.max(f64::MIN_POSITIVE)
        };
        handshakes * rtt + direction(rate.b_to_a) + direction(rate.a_to_b)
    }

    /// The predicted wall seconds of a session's preparation, while as many
    /// preparations run at once as the run allows.
    pub fn prepare_seconds(&self, point: &Point) -> f64 {
        self.overhead_seconds
            + self.build_seconds_per_message * build_messages(point)
            + self.fetch_seconds_per_message * point.replicas.live_b() as f64
    }

    /// The predicted wall seconds of a whole session, preparation included.
    pub fn total_seconds(&self, point: &Point) -> f64 {
        self.prepare_seconds(point) + self.session_seconds(point)
    }

    /// Rates fitted to calibration's measurements of the compute-bound
    /// `point` and of an empty pair.
    ///
    /// `build_seconds` and `fetch_seconds` come from preparations of
    /// `point` run as many at once as the run allows, `session_seconds`
    /// from its session on an uncapped link with a short round trip, and
    /// `empty_seconds` from the preparation of an empty pair alone.
    ///
    /// The overhead is the empty pair's preparation less the offset it
    /// builds at the fitted rate. Any rate the measurements cannot
    /// determine keeps its default.
    pub fn fit(
        point: &Point,
        build_seconds: f64,
        fetch_seconds: f64,
        session_seconds: f64,
        empty_seconds: f64,
    ) -> Self {
        let defaults = Self::default();
        let rate = |seconds: f64, messages: f64, default: f64| {
            if messages > 0.0 && seconds.is_finite() && seconds > 0.0 {
                seconds / messages
            } else {
                default
            }
        };
        let r = &point.replicas;
        let build_seconds_per_message = rate(
            build_seconds,
            build_messages(point),
            defaults.build_seconds_per_message,
        );
        let offset = build_seconds_per_message * OFFSET_RANGE.end as f64;
        Self {
            build_seconds_per_message,
            fetch_seconds_per_message: rate(
                fetch_seconds,
                r.live_b() as f64,
                defaults.fetch_seconds_per_message,
            ),
            session_seconds_per_message: rate(
                session_seconds,
                (r.only_a() + r.only_b()) as f64,
                defaults.session_seconds_per_message,
            ),
            overhead_seconds: if empty_seconds.is_finite() && empty_seconds > 0.0 {
                (empty_seconds - offset).max(0.0)
            } else {
                defaults.overhead_seconds
            },
        }
    }
}

/// The messages a prepared session's two processes hold at their peak:
/// side A holds both replicas while it builds them, and side B its own
/// once it has joined.
fn held_messages(point: &Point) -> f64 {
    build_messages(point) + point.replicas.live_b() as f64
}

/// How many bytes of memory a session's two processes hold at most while
/// it is prepared and while it waits for its timed session: the quantity
/// the run bounds when it schedules preparation ([`crate::schedule`]).
///
/// The model is a line in the messages the processes hold
/// ([`held_messages`]): a fixed part for two processes and their runtimes,
/// and a part per message, which is a fixed overhead beyond the message's
/// payload plus the payload itself, so points whose messages are larger
/// than calibration's cost proportionally more.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryModel {
    /// Bytes two processes hold whatever their pair.
    pub fixed_bytes: f64,
    /// Bytes each held message costs beyond its payload's mean size.
    pub overhead_bytes_per_message: f64,
}

impl Default for MemoryModel {
    /// A model above resident memory measured on Linux x86-64, for use
    /// until calibration measures this host's.
    ///
    /// Sessions of 10,000 and 1,000,000 messages a side at 100-byte
    /// messages measured about 18 MiB, and 790 bytes a message beyond its
    /// payload.
    fn default() -> Self {
        Self {
            fixed_bytes: (32u64 << 20) as f64,
            overhead_bytes_per_message: (1u64 << 10) as f64,
        }
    }
}

impl MemoryModel {
    /// The predicted peak bytes of `point`'s prepared session.
    pub fn bytes(&self, point: &Point) -> u64 {
        let per_message = self.overhead_bytes_per_message + point.sizes.mean;
        (self.fixed_bytes + held_messages(point) * per_message) as u64
    }

    /// The line through two measured preparations: `empty`, a pair with no
    /// messages beyond the offset, and `point`, a large one, each with the
    /// peak resident bytes its two processes reached.
    ///
    /// Both parts are clamped to be at least zero, and measurements that
    /// cannot determine the slope keep the defaults.
    pub fn fit(empty: (&Point, u64), point: (&Point, u64)) -> Self {
        let (empty_point, empty_bytes) = (empty.0, empty.1 as f64);
        let (point, bytes) = (point.0, point.1 as f64);
        let messages = held_messages(point) - held_messages(empty_point);
        if messages <= 0.0 || bytes <= empty_bytes {
            return Self::default();
        }
        let slope = (bytes - empty_bytes) / messages;
        Self {
            fixed_bytes: (empty_bytes - slope * held_messages(empty_point)).max(0.0),
            overhead_bytes_per_message: (slope - point.sizes.mean).max(0.0),
        }
    }
}

#[cfg(test)]
mod tests;
