//! Process-global counters classifying pair-hull traffic by the ladder
//! rung that answered it.
//!
//! Every pair-hull construction ([`Version::span`](crate::Version::span),
//! `span_all`'s leaf combines, and the span union's point-combines,
//! which derive their hull through the same kernel) descends a ladder
//! of fast paths —
//! byte-equal operands, an empty operand, a comparable pair handed back
//! as clones — before the one emitting case, a concurrent pair. Which
//! rung answers is a property of the *caller's traffic*, not of the
//! kernel: a consumer whose pairs are mostly comparable pays comparison
//! sweeps, one whose pairs are mostly concurrent pays emissions, and no
//! per-operation envelope can see the mix. These counters record it, so
//! a consumer workload (a tree's bounds memos, a reconciliation run)
//! can be measured at the door it actually exercises.
//!
//! The recording compiles to nothing without the `meter` feature — the
//! [`codec::scan`](crate::codec) counter's idiom — and the readings are
//! process-global with the same isolation requirement as every other
//! meter: meaningful one scenario per process (nextest's model) or
//! under a single-threaded caller. The read surface is
//! `meter::span_traffic` / `meter::reset_span_traffic`.

/// One pair-hull call's classification: the span ladder rung that
/// answered it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rung {
    /// Byte-equal operands: the coincident hull, two clones of one
    /// stream, no walk.
    Equal,
    /// An empty operand: the hull is the operands themselves (the
    /// empty version is the lattice bottom), no walk.
    Empty,
    /// A comparable pair: the hull is the pair reordered, handed back
    /// as clones at the cost of one comparison sweep, zero emission.
    Comparable,
    /// A concurrent pair: the one emitting case — the classifying
    /// sweep's early-exiting prefix, then the fused emission walk.
    Concurrent,
}

/// A snapshot of the four rung counters, in calls.
///
/// Read through `meter::span_traffic`; the fields sum to the pair-hull
/// call count since the last reset.
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
    pub(super) fn record(rung: super::Rung) {
        cell(rung).fetch_add(1, Ordering::Relaxed);
    }

    /// The counters since the last [`reset`].
    pub(crate) fn snapshot() -> super::SpanTraffic {
        super::SpanTraffic {
            equal: EQUAL.load(Ordering::Relaxed),
            empty: EMPTY.load(Ordering::Relaxed),
            comparable: COMPARABLE.load(Ordering::Relaxed),
            concurrent: CONCURRENT.load(Ordering::Relaxed),
        }
    }

    /// Reset every rung counter to zero.
    pub(crate) fn reset() {
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
pub(crate) use counter::{reset, snapshot};

/// Count one pair-hull call answered by `rung`.
///
/// Compiles to nothing without the `meter` feature, so the ladder can
/// call it unconditionally.
#[inline(always)]
pub(crate) fn record(rung: Rung) {
    #[cfg(feature = "meter")]
    counter::record(rung);
    #[cfg(not(feature = "meter"))]
    let _ = rung;
}
