//! Process-global counters classifying the fill walk's priced-offset
//! emissions by the outcome of the watermark web's post-sign domination
//! read.
//!
//! A priced-offset emission with no latent boundary and a word-scale
//! offset reads the anchor gap's sign domination instead of folding
//! (`MinWeb::emit_offset`'s no-fold paths): a decided dominating-positive
//! gap returns with nothing to do, a decided wide-negative gap is the
//! scale-disparate undercut whose residue moves out whole, and an
//! undecided read falls back to the fold-and-restore path. Which outcome
//! answers is a property of the *input's* shape — a family built to drive
//! the dominated-undercut arm certifies the arm fires by reading this
//! counter, so a routing change that silently re-routes its emissions
//! onto the fold path (the arm going dead while every value stays
//! correct) trips a committed floor instead of passing unread. The
//! `dominated-undercut` family's band in `tests/meter.rs` pins exactly
//! that floor.
//!
//! The recording compiles to nothing without the `meter` feature — the
//! [`codec::scan`](crate::codec) counter's idiom — and the readings are
//! process-global with the same isolation requirement as every other
//! meter: meaningful one scenario per process (nextest's model) or under
//! a single-threaded caller. The read surface is `meter::emit_traffic` /
//! `meter::reset_emit_traffic`.

/// One domination read's classification: the emission arm that answered
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Decision {
    /// The gap read decided dominating-positive: the emission sits at or
    /// above the minimum, answered with no fold and no state change.
    DominatedAbove,
    /// The gap read decided wide-negative: the scale-disparate undercut,
    /// its residue moved out whole with the offset folded at the
    /// documented polarity.
    DominatedUndercut,
    /// The read could not certify domination: the emission fell back to
    /// the fold-and-restore path.
    Undecided,
}

/// A snapshot of the three decision counters, in emissions.
///
/// Read through `meter::emit_traffic`; the fields sum to the domination
/// reads performed since the last reset.
#[cfg(feature = "meter")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmitTraffic {
    /// Emissions the dominating-positive arm answered.
    pub dominated_above: u64,
    /// Emissions the dominated-undercut arm answered.
    pub dominated_undercut: u64,
    /// Emissions that fell back to the fold-and-restore path.
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

    /// Count one domination read answered by `decision`.
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

/// Count one domination read answered by `decision`.
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
