//! Classifies how pair-hull construction was resolved.
//!
//! Equal, empty, and causally comparable operands can return existing values;
//! concurrent operands require a new Version. The mixture is a property of a
//! workload rather than one operation in isolation, so resource tests inspect
//! these counters to prove which path their inputs exercise.
//!
//! Recording compiles to nothing without the `meter` feature. Counts are
//! process-global, so a measurement must run alone or on one thread.

/// How one pair-hull call was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rung {
    /// Byte-equal operands: the coincident hull, two clones of one stream, no
    /// walk.
    Equal,
    /// An empty operand: the hull is the operands themselves (the empty version
    /// is the lattice bottom), no walk.
    Empty,
    /// A comparable pair: the hull is the pair reordered, handed back as clones
    /// at the cost of one comparison sweep, zero emission.
    Comparable,
    /// A concurrent pair: the one emitting case — the classifying sweep's
    /// early-exiting prefix, then the fused emission walk.
    Concurrent,
}

/// A snapshot of the four rung counters, in calls.
///
/// Read through `meter::span_traffic`; the fields sum to the pair-hull call
/// count since the last reset.
#[cfg(feature = "meter")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpanTraffic {
    /// Calls the byte-equal rung answered.
    pub equal: u64,
    /// Calls the empty-operand rungs answered.
    pub empty: u64,
    /// Calls the comparable rung answered (hand-back, no emission).
    pub comparable: u64,
    /// Calls that reached the emitting walk (concurrent pairs).
    pub concurrent: u64,
}

#[cfg(feature = "meter")]
mod counter {
    use core::sync::atomic::{AtomicU64, Ordering};

    static EQUAL: AtomicU64 = AtomicU64::new(0);
    static EMPTY: AtomicU64 = AtomicU64::new(0);
    static COMPARABLE: AtomicU64 = AtomicU64::new(0);
    static CONCURRENT: AtomicU64 = AtomicU64::new(0);

    /// The counter cell for one rung.
    fn cell(rung: super::Rung) -> &'static AtomicU64 {
        match rung {
            super::Rung::Equal => &EQUAL,
            super::Rung::Empty => &EMPTY,
            super::Rung::Comparable => &COMPARABLE,
            super::Rung::Concurrent => &CONCURRENT,
        }
    }

    /// Count one call answered by `rung`.
    pub fn record(rung: super::Rung) {
        cell(rung).fetch_add(1, Ordering::Relaxed);
    }

    /// The counters since the last [`reset`].
    pub fn snapshot() -> super::SpanTraffic {
        super::SpanTraffic {
            equal: EQUAL.load(Ordering::Relaxed),
            empty: EMPTY.load(Ordering::Relaxed),
            comparable: COMPARABLE.load(Ordering::Relaxed),
            concurrent: CONCURRENT.load(Ordering::Relaxed),
        }
    }

    /// Reset every rung counter to zero.
    pub fn reset() {
        for rung in [
            super::Rung::Equal,
            super::Rung::Empty,
            super::Rung::Comparable,
            super::Rung::Concurrent,
        ] {
            cell(rung).store(0, Ordering::Relaxed);
        }
    }
}

#[cfg(feature = "meter")]
pub use counter::{reset, snapshot};

/// Count one pair-hull call answered by `rung`.
///
/// Compiles to nothing without the `meter` feature, so the ladder can call it
/// unconditionally.
#[inline(always)]
pub fn record(rung: Rung) {
    #[cfg(feature = "meter")]
    counter::record(rung);
    #[cfg(not(feature = "meter"))]
    let _ = rung;
}
