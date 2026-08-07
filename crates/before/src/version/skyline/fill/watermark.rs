//! Range-minimum watermarks over one running height: the fill walk's
//! shared-anchor bookkeeping.
//!
//! One [`MinStack`] tracks, for every open range of the walk, the minimum
//! *emitted* value in that range — without materializing any of them. The
//! representation is one signed accumulator `t = h − A` (`h` the walk's running
//! input height, `A` an *anchor* at or above the innermost armed range's
//! minimum `m`), one optional latent boundary `Λ = A − m` (strictly positive
//! when present; absent means `A = m` exactly), and a stack of nonnegative
//! differences `min(inner) − min(outer)` between adjacent armed ranges, with
//! runs of zero differences compressed to one counted entry. Ranges nest LIFO
//! and minima are monotone outward (an outer range's emissions include its
//! inner ranges'), so the differences are nonnegative by construction and one
//! `t` serves every frame.
//!
//! The cost discipline conserves *width*, not just object lifetimes: every
//! digit touch is paid by a consumed input code, an emitted output code, or the
//! death of the digits it reads — so wide content can shuttle between the
//! difference stack and the latent register by moves alone, and no schedule of
//! arms and closes re-reads a width the input paid for only once. Enforced by
//! shape:
//!
//! - Each consumed input delta folds into `t` once (a uniform shift of
//!   `h` against a fixed anchor) — never once per open range.
//! - A close never folds: a popped nonzero boundary MOVES into the
//!   latent register (merging min-into-max with one already parked),
//!   leaving `t` and the followers untouched — the anchor goes stale by
//!   exactly the parked width.
//! - An arm recycles: the arming offset `v − A` is narrow whenever the
//!   input moved little since the anchor was seated, and the true
//!   boundary `v − m` is that offset merged min-into-max with the
//!   latent's buffer — pushed back as the new difference by move, the
//!   register drained.
//! - An emission compares against `m` by one amortized sign read
//!   against the anchor; a drop landing under the anchor is decided
//!   against the latent by top-index domination in O(1), and only
//!   comparable scales fold — a min-into-max collapse whose
//!   near-cancellation funds it, after which re-widening the latent
//!   costs the input a fresh climb. When a comparison must fold, it
//!   folds the *priced* side (the emission's own offset, paid by the
//!   scan or code that produced it) and restores it, or it is the dying
//!   side's single terminal fold. A wide `t` is never folded into
//!   anything while it survives; a word-scale offset against a
//!   dominating `t` is decided post-sign with no fold at all.
//! - An undercut (an emission below `m`) replaces `t`, annihilates any
//!   latent into the residue (the drop dominated it), and propagates
//!   the drop outward: whole zero runs pass in O(1) (their frames'
//!   minima track the innermost implicitly), each nonzero difference
//!   the drop consumes dies by one fold into the residue, and the
//!   stopping frame absorbs exactly one surviving fold, bounded by the
//!   residue the input or the emission already paid for. The residue is
//!   never folded into frames it passes.
//! - Dying accumulators return to a pool and are re-armed cleared, so
//!   range churn allocates nothing in steady state.
//!
//! *Followers* ride the stack: accumulators tracking `m − X` for a caller-fixed
//! reference `X` (the emit path's previous output value; the memoized
//! pre-scan's last recorded minimum). Arms, undercuts, and collapses fold the
//! same operand they already price into each active follower; closes touch no
//! follower at all — each active slot goes *anchor-relative* under a one-bit
//! tag (`f_true = f_stored − Λ`), resolved at the follower's own death:
//! symbolically where the consumer is itself anchor-relative or the switch's
//! terms cancel, by one latent fold where an emitted code prices it, and by the
//! death-event fan-out at undercuts and collapses. A set tag never outlives its
//! latent.

use core::cmp::Ordering;

use suanpan::Accumulator;

use crate::codec::Int;
use crate::version::skyline::signed::{fold_signed_int, Signed};

/// One record of the difference stack.
enum DiffEntry {
    /// A nonzero `min(inner) − min(outer)` between adjacent armed frames.
    Diff(Accumulator),
    /// `count` consecutive frames whose minima equal the next inner frame's.
    ZeroRun(usize),
}

/// The LIFO web of range-minimum watermarks (module doc).
pub(super) struct MinStack {
    /// `h − A` for the anchor `A` of the innermost armed frame (`A = m + Λ` for
    /// the latent `Λ`, so `A = m` exactly while no latent lives); zero-valued
    /// while `armed == 0`.
    t: Accumulator,
    /// The latent boundary `Λ = A − m`: the anchor's stale excess over the
    /// innermost armed frame's true minimum.
    ///
    /// Strictly positive when present; at most one lives, conceptually at the
    /// top of the difference stack; it holds no height content (heights fold
    /// into `t` only) and dies with the last armed frame.
    latent: Option<Accumulator>,
    /// Per follower slot: whether the stored content is anchor-relative
    /// (`f_true = f_stored − Λ`). Set only while the latent lives; a set tag
    /// never outlives it.
    sig: [bool; 2],
    /// Adjacent-frame differences outward from the innermost armed frame, zero
    /// runs compressed; last entry = nearest the innermost.
    diffs: Vec<DiffEntry>,
    /// Open frames with no emission yet, all inner of every armed one.
    pending: usize,
    /// Armed frames (the difference stack carries `armed − 1` frame
    /// records).
    armed: usize,
    /// Active followers (module doc), tracking `m − X` (anchor-relative while
    /// the slot's `sig` tag is set).
    followers: [Option<Accumulator>; 2],
    /// Cleared accumulators awaiting reuse.
    pool: Vec<Accumulator>,
}

impl MinStack {
    pub(super) fn new() -> Self {
        MinStack {
            t: Accumulator::new(),
            latent: None,
            sig: [false, false],
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
    pub(super) fn fold_height(&mut self, neg: bool, mag: &Int) {
        if self.armed > 0 {
            fold_signed_int(&mut self.t, neg, mag);
        }
    }

    /// Close the innermost range, merging its minimum into its parent.
    ///
    /// Monotone nesting makes the merge free — the parent's minimum already
    /// reflects every inner emission (propagation kept it live) — and the
    /// latent makes it O(1): a popped zero run decrements; a popped nonzero
    /// boundary MOVES into the latent register (minting it, or dying
    /// min-into-max into a live one), leaving `t` and the followers untouched.
    /// Each active follower goes anchor-relative by its one-bit tag instead of
    /// absorbing a fold, so a close never touches a follower digit. The last
    /// armed frame's close retires the web: `t` and any latent drop unread
    /// (followers are already dead, so no surviving relation needs
    /// re-anchoring).
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
            self.sig = [false, false];
            if let Some(latent) = self.latent.take() {
                self.retire(latent);
            }
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
                // m widens from the child's to the parent's, d lower; the
                // anchor stays where it is and the boundary parks in the latent
                // (`Λ += d` by min-into-max merge, or the mint move). Active
                // followers were exact against the old anchor state, so tagging
                // them anchor-relative is value-preserving: a mint finds them
                // `m`-exact with `A = m_old`, a merge finds them already tagged
                // (a live latent keeps every active follower tagged).
                match self.latent.take() {
                    None => {
                        for slot in 0..self.followers.len() {
                            debug_assert!(!self.sig[slot], "a set tag never outlives its latent");
                            if self.followers[slot].is_some() {
                                self.sig[slot] = true;
                            }
                        }
                        self.latent = Some(d);
                    }
                    Some(mut latent) => {
                        debug_assert!(
                            (0..self.followers.len())
                                .all(|s| self.followers[s].is_none() || self.sig[s]),
                            "a live latent keeps every active follower tagged"
                        );
                        let drained = latent.merge_into_wider(d);
                        self.retire(drained);
                        self.latent = Some(latent);
                    }
                }
            }
        }
    }

    /// Retire the latent into the true minimum.
    ///
    /// The anchor re-bases to `m` (`t += Λ` by min-into-max buffer merge), each
    /// tagged follower resolves by one fold of the dying latent (its
    /// death-event fan-out), and the tags clear. A no-op while no latent lives.
    ///
    /// Callers fund the death: a comparable-scale decision (the merge's
    /// near-cancellation), an emission whose output code the latent's width
    /// widens, or a re-anchor riding one (the site close and recorder paths,
    /// whose in-cycle case finds the latent already drained by the arm).
    pub(super) fn resolve_latent(&mut self) {
        let Some(latent) = self.latent.take() else {
            return;
        };
        for slot in 0..self.followers.len() {
            if self.sig[slot] {
                self.followers[slot]
                    .as_mut()
                    .expect("a set tag rides an active follower")
                    .sub_accum(&latent);
                self.sig[slot] = false;
            }
        }
        let drained = self.t.merge_into_wider(latent);
        self.retire(drained);
    }

    /// Whether a latent boundary is live (the anchor sits above the
    /// true minimum).
    pub(super) fn latent_live(&self) -> bool {
        self.latent.is_some()
    }

    /// Record an emission at the current height (`v = h`).
    pub(super) fn emit_here(&mut self) {
        if self.pending > 0 {
            let below = self.lease();
            self.arm(below);
            return;
        }
        // v − A = t: one amortized sign read; at or above the anchor is at or
        // above the minimum and costs nothing further.
        if self.t.sign() != Ordering::Less {
            return;
        }
        // v < A: only a drop past the latent too is a true undercut.
        if self.latent.is_some() && !self.decide_undercut_through_latent() {
            return;
        }
        if self.t.sign() != Ordering::Less {
            // A collapse re-based the anchor to m and v is not below it.
            return;
        }
        // Undercut: m drops to v, t dies into the residue.
        self.undercut();
        self.t = self.lease();
    }

    /// Record an emission at `v = h + off` for a signed, priced offset (a
    /// consuming scan's extremum, or a raise decided against it).
    pub(super) fn emit_offset(&mut self, off: &Signed) {
        if off.is_zero() {
            self.emit_here();
            return;
        }
        if self.pending > 0 {
            // below = h − v = −off.
            let mut below = self.lease();
            fold_signed_int(&mut below, !off.negative, &off.magnitude);
            self.arm(below);
            return;
        }
        // v − A = t + off. With no latent, post-sign domination decides against
        // a word-scale offset with no fold (with one live, the O(off) fold
        // below is cheap for a word and the ladder decides).
        if self.latent.is_none() && off.magnitude.to_u64().is_some() {
            let (sign, decided) = self.t.sign_dominates_word();
            if decided {
                if sign == Ordering::Greater {
                    return;
                }
                // t wide-negative: v sits far below the minimum; the
                // drop dwarfs the offset. Residue = m − v = −t − off.
                let mut residue = core::mem::take(&mut self.t);
                residue.negate();
                fold_signed_int(&mut residue, off.negative, &off.magnitude);
                for follower in self.followers.iter_mut().flatten() {
                    follower.sub_accum(&residue);
                }
                let mut t = self.lease();
                fold_signed_int(&mut t, !off.negative, &off.magnitude);
                self.t = t;
                self.propagate(residue);
                return;
            }
        }
        // Fold the priced side; restore it unless it funds the residue.
        fold_signed_int(&mut self.t, off.negative, &off.magnitude);
        if self.t.sign() != Ordering::Less {
            // v at or above the anchor, hence at or above the minimum.
            fold_signed_int(&mut self.t, !off.negative, &off.magnitude);
            return;
        }
        // v < A: only a drop past the latent too is a true undercut.
        if self.latent.is_some() && !self.decide_undercut_through_latent() {
            fold_signed_int(&mut self.t, !off.negative, &off.magnitude);
            return;
        }
        if self.t.sign() != Ordering::Less {
            // A collapse re-based the anchor to m and v is not below it.
            fold_signed_int(&mut self.t, !off.negative, &off.magnitude);
            return;
        }
        // Undercut: t holds v − A, off stays folded to fund the residue.
        self.undercut();
        let mut t = self.lease();
        fold_signed_int(&mut t, !off.negative, &off.magnitude);
        self.t = t;
    }

    /// Decide a drop below the anchor (`t < 0` holding `v − A`) against the
    /// true minimum `m = A − Λ` while a latent lives.
    ///
    /// Top-index domination answers scale-disparate cases in O(1): a dominating
    /// latent means `m < v < A` — return false, nothing changes; a dominated
    /// one means a true undercut — return true with the latent left live for
    /// [`undercut`](Self::undercut)'s residue to annihilate. Comparable tops
    /// retire the latent (the near-cancellation funds the merge, and
    /// re-widening it costs the input a fresh climb) and return true for the
    /// caller's plain re-test against the re-based anchor.
    fn decide_undercut_through_latent(&mut self) -> bool {
        let t_floor = self.t.digit_count() - 1;
        let latent = self.latent.as_mut().expect("the caller saw a live latent");
        // Collapse for an honest top before the domination reads.
        let _sign = latent.sign();
        debug_assert_eq!(_sign, Ordering::Greater, "the latent is strictly positive");
        if latent.sign_dominates_at(t_floor).1 {
            return false;
        }
        let lat_floor = latent.digit_count() - 1;
        if self.t.sign_dominates_at(lat_floor).1 {
            return true;
        }
        self.resolve_latent();
        true
    }

    /// Drop the true minimum to `v = A + t` (`t < 0`, the true-undercut
    /// decision already made).
    ///
    /// `t` dies into the residue, a live latent annihilates into it, and each
    /// active follower absorbs the anchor-relative drop `A − v` — one fold that
    /// also resolves a set tag, since a tagged follower's content is
    /// anchor-relative and the new anchor is `v` itself. The caller re-seats
    /// `t` for the new anchor.
    fn undercut(&mut self) {
        let mut residue = core::mem::take(&mut self.t);
        residue.negate();
        for slot in 0..self.followers.len() {
            if let Some(follower) = &mut self.followers[slot] {
                debug_assert_eq!(
                    self.sig[slot],
                    self.latent.is_some(),
                    "a live latent keeps every active follower tagged"
                );
                follower.sub_accum(&residue);
                self.sig[slot] = false;
            }
        }
        if let Some(latent) = self.latent.take() {
            // The annihilation: residue = (A − v) − Λ = m − v > 0.
            residue.sub_accum(&latent);
            self.retire(latent);
        }
        self.propagate(residue);
    }

    /// Record an emission at `v = h − below` where `below` arrives as a funded
    /// accumulator (a resolved memoized minimum), arming the pending frame that
    /// must exist for it.
    ///
    /// The accumulator moves into the web — it becomes the new `t` — so wide
    /// content is stored once and read only at the arming boundary it prices.
    pub(super) fn emit_below_accum(&mut self, below: Accumulator) {
        debug_assert!(self.pending > 0, "a raise arms its own node's frame");
        self.arm(below);
    }

    /// Whether `h + above` reaches the innermost armed minimum:
    /// `Ordering::Less` means strictly below `m`.
    ///
    /// The raise arms' decision read. Post-sign domination answers a word-scale
    /// offset with no fold; otherwise the priced offset is folded and restored,
    /// with the latent ladder deciding drops that land between the true minimum
    /// and the anchor (domination in O(1), or a funded collapse at comparable
    /// scales).
    pub(super) fn compare_above(&mut self, above: &Signed) -> Ordering {
        debug_assert!(self.armed > 0, "a raise compares against an armed frame");
        if self.latent.is_none() && above.magnitude.to_u64().is_some() {
            let (sign, decided) = self.t.sign_dominates_word();
            if decided {
                return sign;
            }
        }
        fold_signed_int(&mut self.t, above.negative, &above.magnitude);
        let mut sign = self.t.sign();
        if self.latent.is_some() {
            sign = match sign {
                // v = A > m exactly when a latent lives.
                Ordering::Equal | Ordering::Greater => Ordering::Greater,
                Ordering::Less => {
                    if !self.decide_undercut_through_latent() {
                        Ordering::Greater
                    } else if self.latent.is_some() {
                        // The drop dominates the latent: v < m.
                        Ordering::Less
                    } else {
                        // Collapsed at comparable scales: re-test
                        // plainly against the re-based anchor A = m.
                        self.t.sign()
                    }
                }
            };
        }
        fold_signed_int(&mut self.t, !above.negative, &above.magnitude);
        sign
    }

    /// Whether `h + above` reaches a minimum sitting `d_arm` above the anchor
    /// (`A + d_arm`, `d_arm` signed): `Ordering::Less` means strictly below it.
    ///
    /// The memo consumer's decision read: `(h + above) − (A + d_arm) = t −
    /// d_arm + above`, folded and restored — `above` is priced, `d_arm` is
    /// anchor-relative dying content (a ledger link net of the taken relation,
    /// narrow whenever the reference minima agree), and the latent never
    /// participates: the anchor-relative target cancels it exactly, so the read
    /// costs the operands' own widths no matter how wide the parked boundary
    /// is.
    pub(super) fn compare_above_vs(&mut self, above: &Signed, d_arm: &Accumulator) -> Ordering {
        debug_assert!(self.armed > 0, "a raise compares against an armed frame");
        self.t.sub_accum(d_arm);
        fold_signed_int(&mut self.t, above.negative, &above.magnitude);
        let sign = self.t.sign();
        fold_signed_int(&mut self.t, !above.negative, &above.magnitude);
        self.t.add_accum(d_arm);
        sign
    }

    /// Arm the pending frame at a minimum `d_arm` above the anchor (`v = A +
    /// d_arm`, `d_arm` signed and dying here).
    ///
    /// The memo consumer's arming: the new `t = t_old − d_arm` needs no read of
    /// the old web beyond `d_arm`'s own width, and `d_arm` recycles any parked
    /// latent — the true boundary `v − m` is `d_arm + Λ`, realized by folding
    /// the narrow dying offset min-into-max with the latent's buffer and
    /// pushing the merged buffer (or, negated, propagating it as the undercut's
    /// residue).
    pub(super) fn arm_relative(&mut self, d_arm: Accumulator) {
        debug_assert!(self.pending > 0, "a raise arms its own node's frame");
        debug_assert!(self.armed > 0, "a relative arming needs an armed anchor");
        let pending = core::mem::replace(&mut self.pending, 0);
        self.t.sub_accum(&d_arm);
        self.armed += pending;
        self.push_boundary(d_arm, pending);
    }

    /// Arm bookkeeping shared by the arming paths, after `t` is seated for the
    /// new anchor `A = v` and `armed` counts the new frames.
    ///
    /// Folds the anchor-relative offset `d = v − A_old` into each active
    /// follower (resolving set tags — the offset is exactly the tagged
    /// content's shift to the new anchor, where the latent is spent), merges
    /// the offset with any latent into the true boundary `v − m_old`, and
    /// pushes it (a positive difference), counts it (an exact meet), or
    /// propagates it (an arming undercut's residue).
    fn push_boundary(&mut self, d: Accumulator, pending: usize) {
        for slot in 0..self.followers.len() {
            if let Some(follower) = &mut self.followers[slot] {
                follower.add_accum(&d);
                self.sig[slot] = false;
            }
        }
        let mut d = d;
        if let Some(latent) = self.latent.take() {
            let drained = d.merge_into_wider(latent);
            self.retire(drained);
        }
        match d.sign() {
            Ordering::Greater => {
                self.diffs.push(DiffEntry::Diff(d));
                self.push_zeros(pending - 1);
            }
            Ordering::Equal => {
                self.retire(d);
                self.push_zeros(pending);
            }
            Ordering::Less => {
                d.negate();
                self.propagate(d);
                self.push_zeros(pending);
            }
        }
    }

    /// Install follower `slot` tracking `m − X`, where `X` is whatever
    /// reference the caller's accumulator currently encodes.
    ///
    /// While a latent lives the caller's content must be anchor-relative (`A −
    /// X`) — the slot is tagged and reads resolve through the latent. Every
    /// install site either derives its content from the anchor web itself
    /// (already anchor-relative) or runs where no latent can live (after an
    /// arm's recycle or a [`resolve_latent`](Self::resolve_latent)), so no fold
    /// is ever needed to install.
    pub(super) fn follower_set(&mut self, slot: usize, acc: Accumulator) {
        debug_assert!(self.followers[slot].is_none(), "one follower per slot");
        debug_assert!(self.armed > 0, "a follower needs an armed anchor");
        self.sig[slot] = self.latent.is_some();
        self.followers[slot] = Some(acc);
    }

    /// Remove and return follower `slot`, as stored.
    ///
    /// The content is anchor-relative when the slot was tagged (a latent
    /// lives): `f_true = f_stored − Λ`. Callers either consume it against
    /// another anchor-relative quantity (the tag cancels symbolically), retire
    /// it unread, or run under a preceding
    /// [`resolve_latent`](Self::resolve_latent) that made it exact — never
    /// store it raw into state that outlives the latent.
    pub(super) fn follower_take(&mut self, slot: usize) -> Accumulator {
        self.sig[slot] = false;
        self.followers[slot].take().expect("the follower is active")
    }

    /// A cleared accumulator, pooled when one is available.
    pub(super) fn lease(&mut self) -> Accumulator {
        self.pool.pop().unwrap_or_default()
    }

    /// Retire a dying accumulator into the pool, clearing it.
    pub(super) fn retire(&mut self, mut acc: Accumulator) {
        acc.reset();
        self.pool.push(acc);
    }

    /// Materialize a dying accumulator: collapse, then read the sign and
    /// magnitude (held digits exceed the value's width by at most the collapse
    /// slack), retiring the buffer.
    pub(super) fn materialize(&mut self, mut acc: Accumulator) -> Signed {
        acc.sign();
        let (sign, magnitude) = acc.sign_magnitude();
        self.retire(acc);
        Signed::from_sign_magnitude(sign, magnitude)
    }

    /// Fold the stored `t` into `acc` (`acc += h − A`): the watermark-to-height
    /// anchor switch's bridge read, priced by the code emitted at the switch
    /// that needs it.
    ///
    /// Deliberately anchor-relative: the caller's `acc` is a follower taken
    /// raw, so a live latent cancels symbolically — `(f_true) + (h − m) =
    /// (f_stored − Λ) + (t + Λ) = f_stored + t` — and no latent digit is ever
    /// touched by this switch.
    pub(super) fn bridge_add_t(&mut self, acc: &mut Accumulator) {
        acc.add_accum(&self.t);
    }

    /// Fold the innermost `t` out of `acc` (`acc −= h − m`): the
    /// height-to-watermark anchor switch's bridge read.
    ///
    /// The caller resolves any latent first
    /// ([`resolve_latent`](Self::resolve_latent) — the switch's emission
    /// re-anchors to the true minimum, which retires the latent anyway), so `t`
    /// is exact here.
    pub(super) fn bridge_sub_t(&mut self, acc: &mut Accumulator) {
        debug_assert!(
            self.latent.is_none(),
            "the height-to-watermark switch resolves the latent first"
        );
        acc.sub_accum(&self.t);
    }

    /// Arm every pending frame at the emission `v = h − below`, moving `below`
    /// in as the new `t`.
    fn arm(&mut self, below: Accumulator) {
        debug_assert!(self.pending > 0, "arming consumes pending frames");
        let pending = core::mem::replace(&mut self.pending, 0);
        if self.armed == 0 {
            debug_assert!(
                self.followers.iter().all(Option::is_none),
                "followers attach after the first arming"
            );
            debug_assert!(self.latent.is_none(), "the latent dies with the web");
            self.armed = pending;
            let old = core::mem::replace(&mut self.t, below);
            self.retire(old);
            self.push_zeros(pending - 1);
            return;
        }
        // The anchor-relative offset: d = v − A_old = t_old − below. t_old dies
        // into it (a move of the buffer, one narrow fold), and the boundary
        // bookkeeping recycles any parked latent.
        let mut d = core::mem::replace(&mut self.t, below);
        d.sub_accum(&self.t);
        self.armed += pending;
        self.push_boundary(d, pending);
    }

    /// Drive an undercut's residue (`residue > 0`, the drop below the old
    /// innermost minimum) outward through the difference stack.
    ///
    /// Zero runs pass whole in O(1); each nonzero difference the drop exceeds
    /// dies by one fold *into the residue* at the difference's own width; the
    /// stopping frame absorbs the one surviving fold — the residue's terminal
    /// death into the difference that outlasts it. Top-index domination decides
    /// each hop's direction before any fold, so the dying side always funds the
    /// fold that consumes it and the wide side of a scale-disparate hop is
    /// never read across its width while it survives; only comparable scales
    /// fold undecided, the near-cancellation pricing either direction. The
    /// caller has already adjusted `t` and the followers.
    fn propagate(&mut self, residue: Accumulator) {
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
                    // The width guards skip domination reads a top index could
                    // never decide (`sign_dominates_at` needs two digits of
                    // clearance), so a comparable-scale hop pays no extra read.
                    // Tops are honest: a pushed difference had its sign read at
                    // push, and the residue collapses under its own reads here.
                    // Both sides are strictly positive, so a decided domination
                    // is always `Greater`; each arm requires the sign anyway so
                    // that a violated positivity invariant falls through to the
                    // total fold-then-sign path (the debug asserts keep the
                    // violation loud) instead of folding in the wrong
                    // direction.
                    if residue.digit_count() >= d.digit_count() + 2 {
                        let (sign, decided) = residue.sign_dominates_at(d.digit_count() - 1);
                        debug_assert!(
                            !decided || sign == Ordering::Greater,
                            "the residue is strictly positive"
                        );
                        if decided && sign == Ordering::Greater {
                            // The residue dwarfs the difference: d dies by its
                            // one fold into the surviving residue, which stays
                            // positive and keeps dropping.
                            residue.sub_accum(&d);
                            self.retire(d);
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
                            // The difference dwarfs the residue: the drop stops
                            // here, and the dying residue's terminal fold
                            // shrinks the survivor.
                            d.sub_accum(&residue);
                            self.retire(residue);
                            self.diffs.push(DiffEntry::Diff(d));
                            break;
                        }
                    }
                    // Comparable scales: the near-cancellation prices the fold
                    // — the dying side's digits within a constant, whichever
                    // side dies.
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
