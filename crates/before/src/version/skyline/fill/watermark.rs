//! Range-minimum watermarks over one running height: the fill walk's
//! shared-anchor bookkeeping.
//!
//! One [`MinStack`] tracks, for every open range of the walk, the
//! minimum *emitted* value in that range — without materializing any of
//! them. The representation is one signed accumulator `t = h − m`
//! (`h` the walk's running input height, `m` the innermost armed
//! range's minimum) plus a stack of nonnegative differences
//! `min(inner) − min(outer)` between adjacent armed ranges, with runs
//! of zero differences compressed to one counted entry. Ranges nest
//! LIFO and minima are monotone outward (an outer range's emissions
//! include its inner ranges'), so the differences are nonnegative by
//! construction and one `t` serves every frame.
//!
//! The cost discipline, enforced by shape rather than convention:
//!
//! - Each consumed input delta folds into `t` once (a uniform shift of
//!   `h` against a fixed `m`) — never once per open range.
//! - An emission compares against `m` by one amortized sign read; when
//!   the comparison must fold, it folds the *priced* side (the
//!   emission's own offset, paid by the scan or code that produced it)
//!   and restores it, or it is the dying side's single terminal fold.
//!   A wide `t` is never folded into anything while it survives; a
//!   word-scale offset against a dominating `t` is decided post-sign
//!   with no fold at all.
//! - An undercut (an emission below `m`) replaces `t` and propagates
//!   the drop outward as a residue: whole zero runs pass in O(1) (their
//!   frames' minima track the innermost implicitly), each nonzero
//!   difference the drop consumes dies by one fold into the residue,
//!   and the stopping frame absorbs exactly one surviving fold, bounded
//!   by the residue the input or the emission already paid for. The
//!   residue is never folded into frames it passes.
//! - Dying accumulators return to a pool and are re-armed cleared, so
//!   range churn allocates nothing in steady state.
//!
//! *Followers* ride the stack: accumulators tracking `m − X` for a
//! caller-fixed reference `X` (the emit path's previous output value;
//! the memoized pre-scan's last recorded minimum). Every event that
//! moves `m` — arming, popping, an undercut — folds the same operand it
//! already prices into each active follower, so a follower is exact at
//! all times for one extra O(operand) fold per event.

use core::cmp::Ordering;

use crate::codec::accum::Accum;
use crate::codec::Base;

/// A signed relative quantity: sign and magnitude, the module's
/// exchange currency with the zigzag coding and the scans.
pub(super) type Signed = (bool, Base);

/// Fold a signed quantity into an accumulator.
pub(super) fn fold(acc: &mut Accum, neg: bool, mag: &Base) {
    if neg {
        acc.sub_base(mag);
    } else {
        acc.add_base(mag);
    }
}

/// One record of the difference stack.
enum DiffEntry {
    /// A nonzero `min(inner) − min(outer)` between adjacent armed
    /// frames.
    Diff(Accum),
    /// `count` consecutive frames whose minima equal the next inner
    /// frame's.
    ZeroRun(usize),
}

/// The LIFO web of range-minimum watermarks (module doc).
pub(super) struct MinStack {
    /// `h − m` for the innermost armed frame; zero-valued while
    /// `armed == 0`.
    t: Accum,
    /// Adjacent-frame differences outward from the innermost armed
    /// frame, zero runs compressed; last entry = nearest the innermost.
    diffs: Vec<DiffEntry>,
    /// Open frames with no emission yet, all inner of every armed one.
    pending: usize,
    /// Armed frames (the difference stack carries `armed − 1` frame
    /// records).
    armed: usize,
    /// Active followers (module doc), tracking `m − X`.
    followers: [Option<Accum>; 2],
    /// Cleared accumulators awaiting reuse.
    pool: Vec<Accum>,
}

impl MinStack {
    pub(super) fn new() -> Self {
        MinStack {
            t: Accum::new(),
            diffs: Vec::new(),
            pending: 0,
            armed: 0,
            followers: [None, None],
            pool: Vec::new(),
        }
    }

    /// Whether any frame is armed (an emission has occurred inside an
    /// open range).
    pub(super) fn armed(&self) -> bool {
        self.armed > 0
    }

    /// Open a range: one more frame, unarmed until the next emission.
    pub(super) fn open(&mut self) {
        self.pending += 1;
    }

    /// Fold one consumed input step into the height side of `t`.
    ///
    /// `h` moved while every `m` stayed: exactly the innermost frame's
    /// `t` shifts; the differences and followers are height-free.
    pub(super) fn fold_height(&mut self, neg: bool, mag: &Base) {
        if self.armed > 0 {
            fold(&mut self.t, neg, mag);
        }
    }

    /// Close the innermost range, merging its minimum into its parent.
    ///
    /// Monotone nesting makes the merge free: the parent's minimum
    /// already reflects every inner emission (propagation kept it
    /// live), so closing is one dying-difference fold into `t` (or a
    /// run decrement), plus the mirrored follower folds.
    pub(super) fn close(&mut self) {
        if self.pending > 0 {
            self.pending -= 1;
            return;
        }
        debug_assert!(self.armed > 0, "closing an armed frame");
        self.armed -= 1;
        if self.armed == 0 {
            debug_assert!(self.diffs.is_empty(), "no differences without frames");
            debug_assert!(
                self.followers.iter().all(Option::is_none),
                "followers die before their anchor web does"
            );
            let t = core::mem::take(&mut self.t);
            self.retire(t);
            return;
        }
        match self.diffs.pop().expect("armed > 1 has a difference record") {
            DiffEntry::ZeroRun(n) => {
                if n > 1 {
                    self.diffs.push(DiffEntry::ZeroRun(n - 1));
                }
            }
            DiffEntry::Diff(d) => {
                // m widens from the child's to the parent's, d lower.
                for follower in self.followers.iter_mut().flatten() {
                    follower.sub_accum(&d);
                }
                self.t.add_accum(&d);
                self.retire(d);
            }
        }
    }

    /// Record an emission at the current height (`v = h`).
    pub(super) fn emit_here(&mut self) {
        if self.pending > 0 {
            let below = self.lease();
            self.arm(below);
            return;
        }
        // v − m = t: one amortized sign read; at or above the minimum
        // costs nothing further.
        if self.t.sign() != Ordering::Less {
            return;
        }
        // Undercut: m drops to v, t dies into the residue.
        let mut residue = core::mem::take(&mut self.t);
        residue.negate();
        for follower in self.followers.iter_mut().flatten() {
            follower.sub_accum(&residue);
        }
        self.t = self.lease();
        self.propagate(residue);
    }

    /// Record an emission at `v = h + off` for a signed, priced offset
    /// (a consuming scan's extremum, or a raise decided against it).
    pub(super) fn emit_offset(&mut self, off: &Signed) {
        if off.1 == Base::ZERO {
            self.emit_here();
            return;
        }
        if self.pending > 0 {
            // below = h − v = −off.
            let mut below = self.lease();
            fold(&mut below, !off.0, &off.1);
            self.arm(below);
            return;
        }
        // v − m = t + off. Post-sign domination decides against a
        // word-scale offset with no fold; otherwise fold the priced
        // side and restore it (or let it fund the undercut's residue).
        if off.1.to_u64().is_some() {
            let (sign, decided) = self.t.sign_dominates_word();
            if decided {
                if sign == Ordering::Greater {
                    return;
                }
                // t wide-negative: v sits far below m; the drop dwarfs
                // the offset. Residue = m − v = −t − off.
                let mut residue = core::mem::take(&mut self.t);
                residue.negate();
                fold(&mut residue, off.0, &off.1);
                for follower in self.followers.iter_mut().flatten() {
                    follower.sub_accum(&residue);
                }
                let mut t = self.lease();
                fold(&mut t, !off.0, &off.1);
                self.t = t;
                self.propagate(residue);
                return;
            }
        }
        fold(&mut self.t, off.0, &off.1);
        if self.t.sign() != Ordering::Less {
            // No undercut: restore the folded offset.
            fold(&mut self.t, !off.0, &off.1);
            return;
        }
        // Undercut: t holds v − m = −(the drop).
        let mut residue = core::mem::take(&mut self.t);
        residue.negate();
        for follower in self.followers.iter_mut().flatten() {
            follower.sub_accum(&residue);
        }
        let mut t = self.lease();
        fold(&mut t, !off.0, &off.1);
        self.t = t;
        self.propagate(residue);
    }

    /// Record an emission at `v = h − below` where `below` arrives as a
    /// funded accumulator (a resolved memoized minimum), arming the
    /// pending frame that must exist for it.
    ///
    /// The accumulator moves into the web — it becomes the new `t` —
    /// so wide content is stored once and read only at the arming
    /// boundary it prices.
    pub(super) fn emit_below_accum(&mut self, below: Accum) {
        debug_assert!(self.pending > 0, "a raise arms its own node's frame");
        self.arm(below);
    }

    /// Whether `h + above` reaches the innermost armed minimum:
    /// `Ordering::Less` means strictly below `m`.
    ///
    /// The raise arms' decision read. Post-sign domination answers a
    /// word-scale offset with no fold; otherwise the priced offset is
    /// folded and restored.
    pub(super) fn compare_above(&mut self, above: &Signed) -> Ordering {
        debug_assert!(self.armed > 0, "a raise compares against an armed frame");
        if above.1.to_u64().is_some() {
            let (sign, decided) = self.t.sign_dominates_word();
            if decided {
                return sign;
            }
        }
        fold(&mut self.t, above.0, &above.1);
        let sign = self.t.sign();
        fold(&mut self.t, !above.0, &above.1);
        sign
    }

    /// Whether `h + above` reaches a minimum sitting `d_arm` above the
    /// innermost armed one (`m + d_arm`, `d_arm` signed):
    /// `Ordering::Less` means strictly below it.
    ///
    /// The memo consumer's decision read: `(h + above) − (m + d_arm) =
    /// t − d_arm + above`, folded and restored — `above` is priced and
    /// `d_arm` funded-dying, so the surviving `t` is only ever touched
    /// across their widths.
    pub(super) fn compare_above_vs(&mut self, above: &Signed, d_arm: &Accum) -> Ordering {
        debug_assert!(self.armed > 0, "a raise compares against an armed frame");
        self.t.sub_accum(d_arm);
        fold(&mut self.t, above.0, &above.1);
        let sign = self.t.sign();
        fold(&mut self.t, !above.0, &above.1);
        self.t.add_accum(d_arm);
        sign
    }

    /// Arm the pending frame at a minimum `d_arm` above the innermost
    /// armed one (`v = m + d_arm`, `d_arm` signed and dying here).
    ///
    /// The memo consumer's arming: the new `t = t_old − d_arm` needs no
    /// read of the old web beyond `d_arm`'s own width, and `d_arm`
    /// itself becomes the boundary difference (or, negated, the
    /// undercut's residue).
    pub(super) fn arm_relative(&mut self, d_arm: Accum) {
        debug_assert!(self.pending > 0, "a raise arms its own node's frame");
        debug_assert!(self.armed > 0, "a relative arming needs an armed anchor");
        let pending = core::mem::replace(&mut self.pending, 0);
        self.t.sub_accum(&d_arm);
        self.armed += pending;
        let mut d = d_arm;
        match d.sign() {
            Ordering::Greater => {
                for follower in self.followers.iter_mut().flatten() {
                    follower.add_accum(&d);
                }
                self.diffs.push(DiffEntry::Diff(d));
                self.push_zeros(pending - 1);
            }
            Ordering::Equal => {
                self.retire(d);
                self.push_zeros(pending);
            }
            Ordering::Less => {
                for follower in self.followers.iter_mut().flatten() {
                    follower.add_accum(&d);
                }
                d.negate();
                self.propagate(d);
                self.push_zeros(pending);
            }
        }
    }

    /// Install follower `slot` tracking `m − X`, where `X` is whatever
    /// reference the caller's accumulator currently encodes.
    pub(super) fn follower_set(&mut self, slot: usize, acc: Accum) {
        debug_assert!(self.followers[slot].is_none(), "one follower per slot");
        debug_assert!(self.armed > 0, "a follower needs an armed anchor");
        self.followers[slot] = Some(acc);
    }

    /// Remove and return follower `slot`.
    pub(super) fn follower_take(&mut self, slot: usize) -> Accum {
        self.followers[slot].take().expect("the follower is active")
    }

    /// A cleared accumulator, pooled when one is available.
    pub(super) fn lease(&mut self) -> Accum {
        self.pool.pop().unwrap_or_default()
    }

    /// Retire a dying accumulator into the pool, clearing it.
    pub(super) fn retire(&mut self, mut acc: Accum) {
        acc.reset();
        self.pool.push(acc);
    }

    /// Materialize a dying accumulator: collapse, then read the sign
    /// and magnitude (held digits exceed the value's width by at most
    /// the collapse slack), retiring the buffer.
    pub(super) fn materialize(&mut self, mut acc: Accum) -> Signed {
        acc.sign();
        let (sign, magnitude) = acc.sign_magnitude();
        self.retire(acc);
        (sign == Ordering::Less, Base::from(magnitude))
    }

    /// Fold the innermost `t` into `acc` (`acc += h − m`): a bridge
    /// read of the surviving web, priced by the code emitted at the
    /// switch that needs it.
    pub(super) fn bridge_add_t(&mut self, acc: &mut Accum) {
        acc.add_accum(&self.t);
    }

    /// Fold the innermost `t` out of `acc` (`acc −= h − m`): the
    /// subtractive bridge read.
    pub(super) fn bridge_sub_t(&mut self, acc: &mut Accum) {
        acc.sub_accum(&self.t);
    }

    /// Arm every pending frame at the emission `v = h − below`,
    /// moving `below` in as the new `t`.
    fn arm(&mut self, below: Accum) {
        debug_assert!(self.pending > 0, "arming consumes pending frames");
        let pending = core::mem::replace(&mut self.pending, 0);
        if self.armed == 0 {
            debug_assert!(
                self.followers.iter().all(Option::is_none),
                "followers attach after the first arming"
            );
            self.armed = pending;
            let old = core::mem::replace(&mut self.t, below);
            self.retire(old);
            self.push_zeros(pending - 1);
            return;
        }
        // The boundary difference to the previously armed frame:
        // d = v − m_old = t_old − below. t_old dies into it.
        let mut d = core::mem::replace(&mut self.t, below);
        d.sub_accum(&self.t);
        self.armed += pending;
        match d.sign() {
            Ordering::Greater => {
                for follower in self.followers.iter_mut().flatten() {
                    follower.add_accum(&d);
                }
                self.diffs.push(DiffEntry::Diff(d));
                self.push_zeros(pending - 1);
            }
            Ordering::Equal => {
                self.retire(d);
                self.push_zeros(pending);
            }
            Ordering::Less => {
                // The arming emission undercuts: the old frame's
                // minimum drops to v (a zero boundary), the residue
                // continues outward.
                for follower in self.followers.iter_mut().flatten() {
                    follower.add_accum(&d);
                }
                d.negate();
                self.propagate(d);
                self.push_zeros(pending);
            }
        }
    }

    /// Drive an undercut's residue (`residue > 0`, the drop below the
    /// old innermost minimum) outward through the difference stack.
    ///
    /// Zero runs pass whole in O(1); each nonzero difference the drop
    /// exceeds dies by one fold; the stopping frame absorbs the one
    /// surviving fold. The caller has already adjusted `t` and the
    /// followers.
    fn propagate(&mut self, residue: Accum) {
        let mut residue = residue;
        let mut zeros = 0usize;
        loop {
            match self.diffs.pop() {
                None => {
                    // The outermost armed frame dropped; nothing is
                    // outward of it.
                    self.retire(residue);
                    break;
                }
                Some(DiffEntry::ZeroRun(n)) => zeros += n,
                Some(DiffEntry::Diff(mut d)) => {
                    d.sub_accum(&residue);
                    self.retire(residue);
                    match d.sign() {
                        Ordering::Greater => {
                            // The drop stops here: d survives, shrunk.
                            self.diffs.push(DiffEntry::Diff(d));
                            break;
                        }
                        Ordering::Equal => {
                            // Exact meet: this frame's minimum now
                            // equals the new innermost one's.
                            self.retire(d);
                            zeros += 1;
                            break;
                        }
                        Ordering::Less => {
                            // d dies; the remainder keeps dropping.
                            d.negate();
                            residue = d;
                            zeros += 1;
                        }
                    }
                }
            }
        }
        self.push_zeros(zeros);
    }

    /// Push `n` zero-difference frames, merging with a top run.
    fn push_zeros(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        if let Some(DiffEntry::ZeroRun(m)) = self.diffs.last_mut() {
            *m += n;
        } else {
            self.diffs.push(DiffEntry::ZeroRun(n));
        }
    }
}
