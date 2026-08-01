//! The min-ticks fold's bookkeeping: subtree minima as a range-minimum
//! anchor web, and the frozen component as an epoch ledger settled once.
//!
//! [`min_ticks`](super::min_ticks) folds `Σ leaf heights − Σ internal-node
//! subtree minima` over one leaf sweep. Both sums are event streams over
//! evolving wide quantities — each leaf folds the running height, each
//! closing node folds the minimum of its completed span — and this module
//! is the accounting that keeps every event narrow: no event ever reads a
//! width its own boundary's codes did not pay for.
//!
//! # The minima side: reigns over a range-minimum web
//!
//! Subtree spans nest LIFO along the sweep, so "the closing node's
//! subtree minimum" is always *the innermost open range's minimum* at the
//! close — the range-minimum problem the fill walk's watermark stack
//! already solves, re-derived here for the fold's needs. [`MinWeb`]
//! keeps one signed accumulator `gap = h − A` (`h` the running height,
//! `A` an anchor at or above the innermost minimum `m`), one optional
//! latent boundary `Λ = A − m` (strictly positive when present), and a
//! stack of nonnegative differences `min(inner) − min(outer)` between
//! adjacent open ranges, zero runs compressed. Each consumed delta folds
//! into `gap` once; a leaf's undercut test is an amortized sign read; a
//! close never folds a difference — a popped nonzero boundary MOVES into
//! the latent register — and an undercut's residue drives outward through
//! dying differences, each hop funded by the operand it kills.
//!
//! What the closes *fold into the total* rides the same web as value
//! identity: a [`Reign`]. The innermost minimum's value is a leaf height
//! the sweep already paid for — recorded once, at the boundary that made
//! it the minimum, as a narrow frozen-relative offset (below) — and every
//! close while that value reigns just counts. The record settles into the
//! total exactly once, at its death (a lower leaf dethrones it, a
//! propagating drop annihilates its difference, or the stream ends), as
//! one compacted `offset × count` product priced by the offset's width.
//! A record whose reign is interrupted — an inner range arms above it —
//! moves into the difference record and returns at the pop with its count
//! intact, so an interruption never re-reads the offset.
//!
//! # The heights side: the epoch ledger
//!
//! The sweep splits the running height `h = F + L`: `L` (*live*) the
//! drift since the last freeze on one accumulator, `F` (*frozen*) the
//! rest — never materialized anywhere. [`EpochLedger`] holds one signed
//! drift per freeze (epoch 0's "drift" is the first leaf's absolute) and
//! one signed reference count per epoch: a leaf event folds its narrow
//! `L`-offset into the total directly and counts `+1` against its epoch;
//! a settling reign counts `−count` against the epoch its offset was
//! recorded under. The frozen component reaches the total once, at the
//! end, by summation by parts:
//!
//! `Σ_e refs_e · F_e = Σ_f drift_f · Σ_{e ≥ f} refs_e`
//!
//! — one compacted `drift × suffix-count` product per freeze, priced by
//! the drift's own width (which the codes that built the drift funded)
//! times the count's O(1) compacted digits. No event is ever re-based
//! across a freeze: an offset recorded under epoch `e` keeps its epoch,
//! and the ledger's settle carries the frozen difference for it.
//!
//! # Funding: the potential function and its arity
//!
//! The certificate is a **one-ledger potential over the single operand**:
//! folding a code of `w` digits deposits `Θ(w)` into `Φ`, and each
//! topology bit deposits O(1). Every charge names its deposit:
//!
//! - folds into `L` and `gap`, and each leaf's offset fold and undercut
//!   sign read: the boundary's own code (the freeze trigger keeps `L`
//!   within the allowance of the last code; `gap`'s sign reads amortize
//!   against the folds that widened it);
//! - a reign record's mint and its one death settle: the code funding
//!   the leaf offset it snapshots (records move, and counts add, at
//!   O(1) between mint and death);
//! - a close: O(1) — a count bump plus a boundary *move* into the
//!   latent register;
//! - an undercut's propagation: each consumed difference dies by one
//!   fold into the residue at the dying side's width (the width a
//!   previous arm deposited), decided by top-index domination before
//!   any fold, with zero runs passing whole;
//! - a freeze: the drift's one eviction read, funded by the codes that
//!   built the drift, which the eviction consumes and resets;
//! - the ledger settle: one product per freeze at the evicted drift's
//!   own width.
//!
//! The arity is one: min_ticks is a single-stream fold, so no charge can
//! draw on a ledger its own operand did not fund — the two-operand
//! co-sweep's per-operand split (the module doc's pair section) is not
//! needed here.

use core::cmp::Ordering;

use suanpan::{Accumulator, UBig};

use crate::codec::Base;

use super::{fold_signed, mul_into};

/// The value the innermost minimum currently holds, as the sweep folds
/// it: a frozen-relative offset, its epoch, and the closes counted at it
/// since the record's mint (module doc: the minima side).
struct Reign {
    /// Whether the offset is negative.
    neg: bool,
    /// The offset's magnitude, relative to its epoch's frozen component.
    off: Base,
    /// The epoch whose frozen component anchors `off`.
    epoch: u32,
    /// Closes folded at this record's value, unsettled.
    count: u64,
}

/// A stacked boundary `min(inner) − min(outer)`, held at machine width
/// whenever the value fits.
///
/// Most boundaries on the deep committed shapes are unit-scale (an
/// ascending staircase arms one small difference per level), so the
/// inline arm keeps the web's transient at one machine word per open
/// range instead of one heap buffer.
enum Boundary {
    /// A machine-word difference (strictly positive).
    Word(u64),
    /// A wide difference on its own accumulator (strictly positive).
    Wide(Accumulator),
}

/// One record of the difference stack.
enum Frame {
    /// `count` consecutive ranges whose minima equal the next-inner
    /// range's.
    ZeroRun(usize),
    /// A range whose minimum sits `boundary` below the next-inner one,
    /// with the record realizing that minimum.
    Diff { boundary: Boundary, rec: Reign },
}

/// The LIFO web of range minima with reign records (module doc).
pub(super) struct MinWeb {
    /// `h − A` for the anchor `A` of the innermost armed range
    /// (`A = m + Λ` for the latent `Λ`); zero-valued while `armed == 0`.
    gap: Accumulator,
    /// The latent boundary `Λ = A − m`: the anchor's stale excess over
    /// the innermost minimum. Strictly positive when present.
    latent: Option<Accumulator>,
    /// Adjacent-range differences outward from the innermost armed
    /// range, zero runs compressed; last entry = nearest the innermost.
    frames: Vec<Frame>,
    /// The innermost minimum's record; `Some` exactly while `armed > 0`.
    winner: Option<Reign>,
    /// Open ranges with no leaf yet, all inner of every armed one.
    pending: usize,
    /// Armed ranges (the difference stack carries `armed − 1` records).
    armed: usize,
    /// Cleared accumulators awaiting reuse.
    pool: Vec<Accumulator>,
}

impl MinWeb {
    pub(super) fn new() -> MinWeb {
        MinWeb {
            gap: Accumulator::new(),
            latent: None,
            frames: Vec::new(),
            winner: None,
            pending: 0,
            armed: 0,
            pool: Vec::new(),
        }
    }

    /// Open `n` ranges: the internal nodes a descent just entered.
    pub(super) fn open(&mut self, n: usize) {
        self.pending += n;
    }

    /// Fold one consumed delta into the height side of `gap`.
    ///
    /// `h` moved while every minimum stayed: exactly the innermost
    /// range's `gap` shifts; the differences are height-free.
    pub(super) fn fold_height(&mut self, negative: bool, magnitude: &Base) {
        if self.armed > 0 {
            fold_signed(&mut self.gap, negative, magnitude);
        }
    }

    /// Close the innermost range: fold its minimum into the total (one
    /// count on the reigning record) and merge it into its parent.
    ///
    /// The merge is free — nesting keeps the parent's minimum current —
    /// and O(1): a popped zero run decrements; a popped nonzero boundary
    /// MOVES into the latent register, the interrupted outer record
    /// resuming with its count intact while the inner record dies by its
    /// one settle.
    pub(super) fn close(&mut self, total: &mut Accumulator, ledger: &mut EpochLedger) {
        debug_assert_eq!(self.pending, 0, "a closing range's leaves have all arrived");
        debug_assert!(self.armed > 0, "closing an armed range");
        self.winner
            .as_mut()
            .expect("an armed web has a reigning record")
            .count += 1;
        self.armed -= 1;
        if self.armed == 0 {
            debug_assert!(self.frames.is_empty(), "no differences without ranges");
            let rec = self.winner.take().expect("the reigning record was live");
            self.settle(rec, total, ledger);
            if let Some(latent) = self.latent.take() {
                self.retire(latent);
            }
            let gap = core::mem::take(&mut self.gap);
            self.retire(gap);
            return;
        }
        match self
            .frames
            .pop()
            .expect("armed > 1 has a difference record")
        {
            Frame::ZeroRun(n) => {
                if n > 1 {
                    self.frames.push(Frame::ZeroRun(n - 1));
                }
            }
            Frame::Diff { boundary, rec } => {
                // The minimum widens to the parent's: the anchor stays
                // and the boundary parks in the latent; the inner record
                // will never be folded again, so it settles here.
                let dead = self
                    .winner
                    .replace(rec)
                    .expect("the reigning record was live");
                self.settle(dead, total, ledger);
                self.park(boundary);
            }
        }
    }

    /// Record one leaf at the running height, with its narrow
    /// frozen-relative offset (`neg`, `off`) and epoch.
    ///
    /// Arms any pending ranges at the leaf; otherwise an amortized sign
    /// read decides whether the leaf undercuts the innermost minimum,
    /// and only a true undercut does more than O(1) work — funded by the
    /// differences it consumes.
    pub(super) fn leaf(
        &mut self,
        neg: bool,
        off: &Base,
        epoch: u32,
        total: &mut Accumulator,
        ledger: &mut EpochLedger,
    ) {
        if self.pending > 0 {
            self.arm(neg, off, epoch, total, ledger);
            return;
        }
        if self.armed == 0 {
            // A single-leaf stream: no node will ever fold a minimum.
            return;
        }
        // v − A = gap: at or above the anchor is at or above the minimum.
        if self.gap.sign() != Ordering::Less {
            return;
        }
        // v < A: only a drop past the latent too is a true undercut.
        if self.latent.is_some() && !self.decide_undercut_through_latent() {
            return;
        }
        if self.gap.sign() != Ordering::Less {
            // A collapse re-based the anchor to m and v is not below it.
            return;
        }
        self.undercut(neg, off, epoch, total, ledger);
    }

    /// Close every remaining range at the stream's end.
    pub(super) fn drain(&mut self, total: &mut Accumulator, ledger: &mut EpochLedger) {
        debug_assert_eq!(self.pending, 0, "the final leaf armed every open range");
        while self.armed > 0 {
            self.close(total, ledger);
        }
    }

    /// Arm every pending range at the leaf `v`: the new innermost minima.
    fn arm(
        &mut self,
        neg: bool,
        off: &Base,
        epoch: u32,
        total: &mut Accumulator,
        ledger: &mut EpochLedger,
    ) {
        let pending = core::mem::replace(&mut self.pending, 0);
        if self.armed == 0 {
            // The first arming: the web seats its anchor at v.
            debug_assert!(self.latent.is_none(), "the latent dies with the web");
            self.armed = pending;
            self.winner = Some(Reign {
                neg,
                off: off.clone(),
                epoch,
                count: 0,
            });
            let fresh = self.lease();
            let old = core::mem::replace(&mut self.gap, fresh);
            self.retire(old);
            self.push_zeros(pending - 1);
            return;
        }
        // The anchor-relative offset d = v − A_old: the gap's buffer
        // moves out whole and a fresh zero seats the new anchor A = v.
        let fresh = self.lease();
        let mut d = core::mem::replace(&mut self.gap, fresh);
        self.armed += pending;
        // The true boundary v − m_old recycles any parked latent.
        if let Some(latent) = self.latent.take() {
            let drained = d.merge_into_wider(latent);
            self.retire(drained);
        }
        match d.sign() {
            Ordering::Greater => {
                // The new minima sit above the old: the interrupted
                // record moves into the difference, count intact, and a
                // fresh record reigns at the arming leaf.
                let rec = self
                    .winner
                    .replace(Reign {
                        neg,
                        off: off.clone(),
                        epoch,
                        count: 0,
                    })
                    .expect("an armed web has a reigning record");
                let boundary = self.compact(d);
                self.frames.push(Frame::Diff { boundary, rec });
                self.push_zeros(pending - 1);
            }
            Ordering::Equal => {
                // An exact meet: same minimum value, the record continues.
                self.retire(d);
                self.push_zeros(pending);
            }
            Ordering::Less => {
                // An arming undercut: the old record dies and the drop
                // propagates outward.
                let dead = self
                    .winner
                    .replace(Reign {
                        neg,
                        off: off.clone(),
                        epoch,
                        count: 0,
                    })
                    .expect("an armed web has a reigning record");
                self.settle(dead, total, ledger);
                d.negate();
                self.propagate(d, total, ledger);
                self.push_zeros(pending);
            }
        }
    }

    /// Drop the innermost minimum to the current leaf `v` (`gap < 0`,
    /// the true-undercut decision already made).
    ///
    /// The old record dies by its settle, `gap` dies into the residue, a
    /// live latent annihilates into it, and the drop propagates outward.
    fn undercut(
        &mut self,
        neg: bool,
        off: &Base,
        epoch: u32,
        total: &mut Accumulator,
        ledger: &mut EpochLedger,
    ) {
        let dead = self
            .winner
            .replace(Reign {
                neg,
                off: off.clone(),
                epoch,
                count: 0,
            })
            .expect("an armed web has a reigning record");
        self.settle(dead, total, ledger);
        let fresh = self.lease();
        let mut residue = core::mem::replace(&mut self.gap, fresh);
        residue.negate();
        if let Some(latent) = self.latent.take() {
            // The annihilation: residue = (A − v) − Λ = m − v > 0.
            residue.sub_accum(&latent);
            self.retire(latent);
        }
        self.propagate(residue, total, ledger);
    }

    /// Decide a drop below the anchor (`gap < 0` holding `v − A`)
    /// against the true minimum `m = A − Λ` while a latent lives.
    ///
    /// Top-index domination answers scale-disparate cases in O(1): a
    /// dominating latent means `m < v < A` — return false, nothing
    /// changes; a dominated one means a true undercut — return true with
    /// the latent left live for [`undercut`](Self::undercut)'s residue
    /// to annihilate. Comparable tops retire the latent (the
    /// near-cancellation funds the merge, and re-widening it costs the
    /// input a fresh climb) and return true for the caller's plain
    /// re-test against the re-based anchor.
    fn decide_undercut_through_latent(&mut self) -> bool {
        let gap_floor = self.gap.digit_count() - 1;
        let latent = self.latent.as_mut().expect("the caller saw a live latent");
        // Collapse for an honest top before the domination reads.
        let _sign = latent.sign();
        debug_assert_eq!(_sign, Ordering::Greater, "the latent is strictly positive");
        if latent.sign_dominates_at(gap_floor).1 {
            return false;
        }
        let lat_floor = latent.digit_count() - 1;
        if self.gap.sign_dominates_at(lat_floor).1 {
            return true;
        }
        let latent = self.latent.take().expect("the latent is live");
        let drained = self.gap.merge_into_wider(latent);
        self.retire(drained);
        true
    }

    /// Drive an undercut's residue (`residue > 0`, the drop below the
    /// old innermost minimum) outward through the difference stack.
    ///
    /// Zero runs pass whole in O(1); each nonzero difference the drop
    /// exceeds dies by one fold *into the residue* at the difference's
    /// own width — its record dying by its settle, the range's minimum
    /// now the new leaf's — and the stopping range absorbs the one
    /// surviving fold. Top-index domination decides each wide hop's
    /// direction before any fold, so the dying side always funds the
    /// fold that consumes it; word-scale boundaries fold in O(1)
    /// outright.
    fn propagate(
        &mut self,
        residue: Accumulator,
        total: &mut Accumulator,
        ledger: &mut EpochLedger,
    ) {
        let mut residue = residue;
        let mut zeros = 0usize;
        loop {
            match self.frames.pop() {
                None => {
                    // The outermost armed range dropped; nothing is
                    // outward of it.
                    self.retire(residue);
                    break;
                }
                Some(Frame::ZeroRun(n)) => zeros += n,
                Some(Frame::Diff {
                    boundary: Boundary::Word(w),
                    rec,
                }) => {
                    // A word-scale boundary folds outright: O(1) against
                    // any residue.
                    residue.sub_u64(w);
                    match residue.sign() {
                        Ordering::Greater => {
                            // The boundary died; the drop keeps going.
                            self.settle(rec, total, ledger);
                            zeros += 1;
                        }
                        Ordering::Equal => {
                            // Exact meet: this range's minimum now equals
                            // the new innermost one's.
                            self.settle(rec, total, ledger);
                            self.retire(residue);
                            zeros += 1;
                            break;
                        }
                        Ordering::Less => {
                            // The boundary survives, shrunk: the dying
                            // residue's terminal fold already happened.
                            residue.negate();
                            let boundary = self.compact(residue);
                            self.frames.push(Frame::Diff { boundary, rec });
                            break;
                        }
                    }
                }
                Some(Frame::Diff {
                    boundary: Boundary::Wide(mut d),
                    rec,
                }) => {
                    // The width guards skip domination reads a top index
                    // could never decide; tops are honest (a pushed
                    // difference had its sign read at push, and the
                    // residue collapses under its own reads here). Both
                    // sides are strictly positive, so a decided
                    // domination is always `Greater`.
                    if residue.digit_count() >= d.digit_count() + 2 {
                        let (sign, decided) = residue.sign_dominates_at(d.digit_count() - 1);
                        debug_assert!(
                            !decided || sign == Ordering::Greater,
                            "the residue is strictly positive"
                        );
                        if decided && sign == Ordering::Greater {
                            // The residue dwarfs the difference: d dies
                            // by its one fold into the surviving residue.
                            residue.sub_accum(&d);
                            self.retire(d);
                            self.settle(rec, total, ledger);
                            zeros += 1;
                            continue;
                        }
                    }
                    if d.digit_count() >= residue.digit_count() + 2 {
                        let (sign, decided) = d.sign_dominates_at(residue.digit_count() - 1);
                        debug_assert!(
                            !decided || sign == Ordering::Greater,
                            "stacked differences are strictly positive"
                        );
                        if decided && sign == Ordering::Greater {
                            // The difference dwarfs the residue: the drop
                            // stops here, the dying residue's terminal
                            // fold shrinking the survivor.
                            d.sub_accum(&residue);
                            self.retire(residue);
                            self.frames.push(Frame::Diff {
                                boundary: Boundary::Wide(d),
                                rec,
                            });
                            break;
                        }
                    }
                    // Comparable scales: the near-cancellation prices the
                    // fold — the dying side's digits within a constant,
                    // whichever side dies.
                    d.sub_accum(&residue);
                    self.retire(residue);
                    match d.sign() {
                        Ordering::Greater => {
                            // The drop stops here: d survives, shrunk.
                            let boundary = self.compact(d);
                            self.frames.push(Frame::Diff { boundary, rec });
                            break;
                        }
                        Ordering::Equal => {
                            // Exact meet.
                            self.settle(rec, total, ledger);
                            self.retire(d);
                            zeros += 1;
                            break;
                        }
                        Ordering::Less => {
                            // d dies; the remainder keeps dropping.
                            self.settle(rec, total, ledger);
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

    /// Park a popped boundary in the latent register: a move, never a
    /// fold of the wide side.
    fn park(&mut self, boundary: Boundary) {
        match (self.latent.take(), boundary) {
            (None, Boundary::Word(w)) => {
                let mut latent = self.lease();
                latent.add_u64(w);
                self.latent = Some(latent);
            }
            (None, Boundary::Wide(acc)) => self.latent = Some(acc),
            (Some(mut latent), Boundary::Word(w)) => {
                latent.add_u64(w);
                self.latent = Some(latent);
            }
            (Some(mut latent), Boundary::Wide(acc)) => {
                let drained = latent.merge_into_wider(acc);
                self.retire(drained);
                self.latent = Some(latent);
            }
        }
    }

    /// Store a strictly positive difference at machine width when it
    /// fits, retiring its buffer; keep the accumulator when it does not.
    ///
    /// The width test reads the digit count alone, so a wide difference
    /// is never normalized just to learn it would not fit.
    fn compact(&mut self, acc: Accumulator) -> Boundary {
        if acc.digit_count() <= 2 {
            let (sign, magnitude) = acc.sign_magnitude();
            debug_assert_eq!(sign, Ordering::Greater, "boundaries are strictly positive");
            if let Ok(word) = u64::try_from(&magnitude) {
                self.retire(acc);
                return Boundary::Word(word);
            }
        }
        Boundary::Wide(acc)
    }

    /// Settle a dying record: one compacted `offset × count` product
    /// into the narrow total, and the count against the record's epoch.
    fn settle(&mut self, rec: Reign, total: &mut Accumulator, ledger: &mut EpochLedger) {
        if rec.count == 0 {
            return;
        }
        ledger.minimum_refs(rec.epoch, rec.count);
        // Each counted close subtracted the value once: −(±off) · count.
        mul_into(total, &rec.off, &Base::from(rec.count), 0, !rec.neg);
    }

    /// Push `n` zero-difference ranges, merging with a top run.
    fn push_zeros(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        if let Some(Frame::ZeroRun(m)) = self.frames.last_mut() {
            *m += n;
        } else {
            self.frames.push(Frame::ZeroRun(n));
        }
    }

    /// A cleared accumulator, pooled when one is available.
    fn lease(&mut self) -> Accumulator {
        self.pool.pop().unwrap_or_default()
    }

    /// Retire a dying accumulator into the pool, clearing it.
    fn retire(&mut self, mut acc: Accumulator) {
        acc.reset();
        self.pool.push(acc);
    }
}

/// The frozen component as per-epoch drifts and reference counts,
/// settled by summation by parts once, at the end (module doc: the
/// heights side).
pub(super) struct EpochLedger {
    /// One signed drift per epoch: entry 0 is the first leaf's absolute
    /// height, every later entry one freeze's evicted live drift.
    drifts: Vec<(bool, Base)>,
    /// Per epoch, the signed count of events denominated in that epoch's
    /// frozen component: `+1` per leaf, `−count` per settled reign.
    refs: Vec<i128>,
}

impl EpochLedger {
    /// Open the ledger at epoch 0: the first leaf's absolute height is
    /// the opening frozen component.
    pub(super) fn new(first: Base) -> EpochLedger {
        EpochLedger {
            drifts: vec![(false, first)],
            refs: vec![0],
        }
    }

    /// The current epoch: the freezes so far.
    pub(super) fn epoch(&self) -> u32 {
        u32::try_from(self.drifts.len() - 1).expect("freeze count fits u32")
    }

    /// Count one leaf against the current epoch.
    pub(super) fn leaf_ref(&mut self) {
        *self.refs.last_mut().expect("epoch 0 always exists") += 1;
    }

    /// Count a settled reign's closes against its record's epoch.
    fn minimum_refs(&mut self, epoch: u32, count: u64) {
        self.refs[epoch as usize] -= i128::from(count);
    }

    /// Evict the live drift into a new epoch (or discard a redundantly
    /// spelled zero, keeping the epoch), resetting the live component.
    pub(super) fn freeze(&mut self, live: &mut Accumulator) {
        let (sign, drift) = live.sign_magnitude();
        if drift != UBig::ZERO {
            self.drifts
                .push((sign == Ordering::Less, Base::from(drift)));
            self.refs.push(0);
        }
        live.reset();
    }

    /// Settle the frozen component: `Σ_e refs_e · F_e` by summation by
    /// parts — one `drift × suffix-count` product per epoch, each priced
    /// by the drift's own width.
    pub(super) fn settle(self, total: &mut Accumulator) {
        let mut suffix: i128 = 0;
        for ((neg, drift), refs) in self.drifts.iter().zip(&self.refs).rev() {
            suffix += refs;
            if suffix == 0 {
                continue;
            }
            let count = Base::from(suffix.unsigned_abs());
            mul_into(total, drift, &count, 0, *neg != (suffix < 0));
        }
        debug_assert_eq!(
            suffix, 1,
            "leaves exceed closed nodes by exactly one, so the net reference is one"
        );
    }
}
