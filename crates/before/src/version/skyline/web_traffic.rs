//! Counters for offset emissions decided from the anchor gap's leading digits.
//!
//! With no deferred distance and a word-sized offset,
//! [`RangeMinima::emit_offset`](super::watermark::RangeMinima::emit_offset)
//! can sometimes decide the ordering without folding the offset into a wide
//! gap. These counters distinguish a value above the minimum, an undercut, and
//! a comparison that needed the ordinary folded path. Resource tests use them
//! to confirm that their scale-separated inputs exercise the intended branch.
//!
//! Recording compiles to nothing without the `meter` feature. The counters are
//! process-global, so each measurement must run in an isolated process or on a
//! single thread.

/// The outcome of one leading-digit comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Decision {
    /// The emission is at or above the minimum.
    DominatedAbove,
    /// The emission sets a new minimum.
    DominatedUndercut,
    /// Leading digits did not decide, so the offset was folded into the gap.
    Undecided,
}

/// Offset-emission decisions since the last reset.
#[cfg(feature = "meter")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmitTraffic {
    /// Emissions known to be at or above the minimum without a fold.
    pub dominated_above: u64,
    /// Undercuts known without a fold.
    pub dominated_undercut: u64,
    /// Emissions that required a folded comparison.
    pub undecided: u64,
}

#[cfg(feature = "meter")]
mod counter {
    use core::sync::atomic::{AtomicU64, Ordering};

    static ABOVE: AtomicU64 = AtomicU64::new(0);
    static UNDERCUT: AtomicU64 = AtomicU64::new(0);
    static UNDECIDED: AtomicU64 = AtomicU64::new(0);

    /// The counter cell for one decision.
    fn cell(decision: super::Decision) -> &'static AtomicU64 {
        match decision {
            super::Decision::DominatedAbove => &ABOVE,
            super::Decision::DominatedUndercut => &UNDERCUT,
            super::Decision::Undecided => &UNDECIDED,
        }
    }

    /// Count one decision.
    pub(super) fn record(decision: super::Decision) {
        cell(decision).fetch_add(1, Ordering::Relaxed);
    }

    /// The counters since the last [`reset`].
    pub(crate) fn snapshot() -> super::EmitTraffic {
        super::EmitTraffic {
            dominated_above: ABOVE.load(Ordering::Relaxed),
            dominated_undercut: UNDERCUT.load(Ordering::Relaxed),
            undecided: UNDECIDED.load(Ordering::Relaxed),
        }
    }

    /// Reset every decision counter to zero.
    pub(crate) fn reset() {
        for decision in [
            super::Decision::DominatedAbove,
            super::Decision::DominatedUndercut,
            super::Decision::Undecided,
        ] {
            cell(decision).store(0, Ordering::Relaxed);
        }
    }
}

#[cfg(feature = "meter")]
pub(crate) use counter::{reset, snapshot};

/// Count one decision.
///
/// Compiles to nothing without the `meter` feature, so the emission path
/// can call it unconditionally.
#[inline(always)]
pub(crate) fn record(decision: Decision) {
    #[cfg(feature = "meter")]
    counter::record(decision);
    #[cfg(not(feature = "meter"))]
    let _ = decision;
}
