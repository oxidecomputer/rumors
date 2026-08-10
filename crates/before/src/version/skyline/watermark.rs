//! Range-minimum watermarks over one running height: the anchored-minimum
//! web both skyline sweeps share.
//!
//! One [`MinWeb`] tracks, for every open range of a LIFO sweep, the minimum
//! *emitted* value in that range — without materializing any of them. A *range*
//! is the client's own bracket, opened and closed by the client's sweep and
//! never interpreted by the web; the web holds one record per open range. Two
//! clients drive it: the fill walk (payload `()`, followers installed) and the
//! min-ticks fold (`query`'s `web` module, riding its reign records as the
//! payload). The representation, the emission decisions, and the cost
//! discipline live here once; each client contributes only its own semantics
//! through the payload seam below.
//!
//! # The representation
//!
//! - One signed accumulator `gap = h − A` (`h` the sweep's running input
//!   height, `A` an *anchor* at or above the innermost armed range's
//!   minimum `m`).
//! - One optional latent boundary `Λ = A − m` (strictly positive when
//!   present; absent means `A = m` exactly).
//! - A stack of nonnegative differences `min(inner) − min(outer)` between
//!   adjacent armed ranges, with runs of zero differences compressed to
//!   one counted entry and — where the instantiation elects it
//!   ([`MinWeb::compacting`]) — each nonzero difference held at machine
//!   width whenever it fits ([`Boundary`]).
//!
//! Ranges nest LIFO and minima are monotone outward (an outer range's emissions
//! include its inner ranges'), so the differences are nonnegative by
//! construction and one `gap` serves every range.
//!
//! # The cost discipline: width conservation
//!
//! (An amortization argument, self-contained: a first read can skip to the
//! operations and return here.)
//!
//! Every digit touch is paid by a consumed input code, an emitted output code,
//! or the death of the digits it reads — so wide content can shuttle between
//! the difference stack and the latent register by moves alone, and no schedule
//! of arms and closes re-reads a width the input paid for only once. Enforced
//! by shape:
//!
//! - Each consumed input delta folds into `gap` once (a uniform shift of
//!   `h` against a fixed anchor) — never once per open range.
//! - A close never folds: a popped nonzero boundary MOVES into the latent
//!   register (merged with one already parked by
//!   [`Accumulator::merge_into_wider`]: the narrower buffer folds into the
//!   wider, costing the dying narrow side's width), leaving
//!   `gap` and the followers untouched — the anchor goes stale by exactly
//!   the parked width.
//! - An arm recycles: the arming offset `v − A` is narrow whenever the
//!   input moved little since the anchor was seated, and the true boundary
//!   `v − m` is that offset merged into the latent's buffer
//!   (`merge_into_wider` again, the narrow side dying) —
//!   pushed back as the new difference by move, the register drained.
//! - An emission compares against `m` by one amortized sign read against
//!   the anchor; a drop landing under the anchor is decided against the
//!   latent by top-index domination (a clear gap between the two
//!   operands' top digit indices decides the sign with no fold) in O(1),
//!   and only comparable scales
//!   fold — a narrower-into-wider collapse whose near-cancellation funds
//!   it,
//!   after which re-widening the latent costs the input a fresh climb.
//!   When a comparison must fold, it folds the *priced* side (the
//!   emission's own offset, paid by the scan or code that produced it —
//!   *priced by* is the cost convention the [`overlay`](super::overlay)
//!   module's Cost section mints) and
//!   restores it, or it is the dying side's single terminal fold. A wide
//!   `gap` is never folded into anything while it survives; a word-scale
//!   offset against a dominating `gap` is decided post-sign with no fold
//!   at all.
//! - An undercut (an emission below `m`) replaces `gap`, annihilates any
//!   latent into the residue (the drop dominated it), and propagates the
//!   drop outward:
//!   - whole zero runs pass in O(1) (their ranges' minima track the
//!     innermost implicitly);
//!   - word-scale boundaries fold in O(1) outright;
//!   - each wide difference the drop consumes dies by one fold into the
//!     residue;
//!   - the stopping range absorbs exactly one surviving fold, bounded by
//!     the residue the input or the emission already paid for;
//!   - the residue is never folded into ranges it passes.
//! - Dying accumulators return to a pool and are re-armed cleared, so
//!   range churn allocates nothing in steady state.
//!
//! # Followers
//!
//! *Followers* ride the stack: accumulators tracking `m − X` for a caller-fixed
//! reference `X`. What `X` means is the client's business — the web maintains
//! the relation and never reads it. Only the fill walk installs any (two, for
//! relations named in `fill.rs`; the min-ticks fold installs none and pays two
//! `None` checks per event). Arms, undercuts, and collapses fold the same
//! operand they already price into each active follower; closes touch no
//! follower at all — each active slot goes *anchor-relative* under a one-bit
//! tag (`f_true = f_stored − Λ`), resolved at the follower's own death:
//! symbolically where the consumer is itself anchor-relative or the switch's
//! terms cancel, by one latent fold where an emitted code prices it, and by the
//! death-event fan-out at undercuts and collapses. A set tag never outlives its
//! latent. ([`park`](MinWeb::park) is where the tag is set; its doc discharges
//! the value-preservation of both cases.)
//!
//! # The arming paths
//!
//! Three entry points arm pending ranges, split by where the emission's value
//! comes from:
//!
//! - [`arm_at_height`](MinWeb::arm_at_height): `v = h` exactly. Handles the
//!   first arming (it seats the anchor); the dying `gap` is itself the
//!   anchor-relative offset, moved out whole with no fold.
//! - [`arm_below`](MinWeb::arm_below): `v = h − below`, the accumulator
//!   moving in as the new `gap`. Handles the first arming too; the offset
//!   `gap_old − below` costs one fold of the narrow dying side.
//! - [`arm_relative`](MinWeb::arm_relative): `v = A + arm_offset`,
//!   anchor-relative from the start. Requires an armed anchor — it cannot
//!   first-arm; the memo consumer's arming, its offset recycling any parked
//!   latent.
//!
//! All three converge on the shared boundary bookkeeping
//! ([`push_boundary`](MinWeb::push_boundary)): fold the offset into the active
//! followers, merge it with any latent, then push it as the new difference,
//! count it as an exact meet, or propagate it as an arming undercut's residue.
//!
//! # The payload seam
//!
//! The payload `P` is per-boundary client freight, moved — never read — by the
//! web: a pushed-above arm stacks the payload its caller mints lazily beside
//! the new difference, a close hands the popped boundary's payload back
//! ([`Close::Parked`]) as it parks the boundary, and every operation that kills
//! a difference (an undercut's propagation, an arming undercut) surrenders the
//! dying entry's payload to the caller's `on_die` at exactly the moment the
//! difference dies — so a client can account per-range state (min-ticks' reign
//! records) with the web's own move-only lifetimes, and a payload-free client
//! (`P = ()`) pays nothing.

use core::cmp::Ordering;

use suanpan::Accumulator;

use crate::codec::Int;

use super::signed::{fold_signed_int, Sign, Signed};
use super::web_traffic;

/// Follower slots the web carries (the fill walk's two relations; a const
/// assert beside the fill walk's slot constants binds the two rosters).
pub(super) const FOLLOWER_SLOTS: usize = 2;

/// A stacked boundary `min(inner) − min(outer)`, held at machine width when
/// the instantiation compacts and the value fits
/// ([`MinWeb::compacting`]).
enum Boundary {
    /// A machine-word difference (strictly positive).
    Word(u64),
    /// A wide difference on its own accumulator (strictly positive).
    Wide(Accumulator),
}

/// One record of the difference stack.
enum Entry<P> {
    /// `count` consecutive ranges whose minima equal the next inner range's.
    ZeroRun(usize),
    /// A range whose minimum sits `boundary` below the next-inner one, with
    /// the payload its client rode on that boundary.
    Diff { boundary: Boundary, payload: P },
}

/// What [`MinWeb::close`] popped, for the client to dispatch on.
pub(super) enum Close<P> {
    /// The last armed range closed and the web retired.
    Retired,
    /// The popped record was a zero run: the parent range's minimum equals
    /// the closed one's.
    ZeroRun,
    /// The popped boundary parked in the latent register; its payload
    /// resumes at the parent range.
    Parked(P),
}

/// The LIFO web of range-minimum watermarks (module doc).
pub(super) struct MinWeb<P> {
    /// `h − A` for the anchor `A` of the innermost armed range (`A = m + Λ` for
    /// the latent `Λ`, so `A = m` exactly while no latent lives); zero-valued
    /// while `armed == 0`.
    gap: Accumulator,
    /// The latent boundary `Λ = A − m`: the anchor's stale excess over the
    /// innermost armed range's true minimum.
    ///
    /// Strictly positive when present; at most one lives, conceptually at the
    /// top of the difference stack; it holds no height content (heights fold
    /// into `gap` only) and dies with the last armed range.
    latent: Option<Accumulator>,
    /// Per follower slot: whether the stored content is anchor-relative
    /// (`f_true = f_stored − Λ`). Set only while the latent lives; a set tag
    /// never outlives it.
    anchor_relative: [bool; FOLLOWER_SLOTS],
    /// Adjacent-range differences outward from the innermost armed range, zero
    /// runs compressed; last entry = nearest the innermost.
    diffs: Vec<Entry<P>>,
    /// Open ranges with no emission yet, all inner of every armed one.
    pending: usize,
    /// Armed ranges (the difference stack carries `armed − 1` range
    /// records).
    armed: usize,
    /// Active followers (module doc), tracking `m − X` (anchor-relative while
    /// the slot's tag is set). The fill walk installs them; the min-ticks
    /// fold leaves both slots empty.
    followers: [Option<Accumulator>; FOLLOWER_SLOTS],
    /// Whether pushed boundaries compact to [`Boundary::Word`] when the
    /// value fits — each instantiation's constructor states its client's
    /// measured basis ([`new`](Self::new), [`compacting`](Self::compacting)).
    compact_words: bool,
    /// Cleared accumulators awaiting reuse.
    pool: Vec<Accumulator>,
}

impl<P> MinWeb<P> {
    /// A fresh web whose pushed boundaries stay on their own accumulators.
    ///
    /// The fill walk's instantiation. The walk's boundary buffers live in
    /// the pool either way, so per-push word compaction would improve no
    /// committed transient — and it reads one extra touch per word-scale
    /// site (the compacting read-out), measured on the walk's committed
    /// families (the `width_circulation_cost` and memo modules of
    /// `tests/meter.rs`).
    pub(super) fn new() -> Self {
        MinWeb {
            gap: Accumulator::new(),
            latent: None,
            anchor_relative: [false; FOLLOWER_SLOTS],
            diffs: Vec::new(),
            pending: 0,
            armed: 0,
            followers: [None, None],
            compact_words: false,
            pool: Vec::new(),
        }
    }

    /// A fresh web that stores each pushed boundary at machine width when
    /// the value fits ([`Boundary::Word`]).
    ///
    /// The min-ticks instantiation. Compaction pays exactly where nonzero
    /// boundaries stack, in two currencies: per-boundary transient storage
    /// (an inline word per stacked difference instead of an accumulator
    /// entry) and undercut propagation (a residue consumes each word
    /// boundary by one O(1) fold instead of an accumulator-width hop). The
    /// measured basis is the boundary-stacking row — the
    /// `skyline_min_ticks_ascend` envelope of `tests/meter.rs`, whose shape
    /// holds one nonzero unit difference per open range simultaneously:
    /// un-compacted storage reads ×1.41 that row's pinned peak heap and
    /// ×2.0 its pinned touches, over both ceilings. Shapes whose stacked
    /// differences are all zero runs (the committed dense and comb rows)
    /// never store a boundary, so compaction is invisible there.
    pub(super) fn compacting() -> Self {
        MinWeb {
            compact_words: true,
            ..Self::new()
        }
    }

    /// Whether any range is armed (an emission has occurred inside an
    /// open range).
    pub(super) fn armed(&self) -> bool {
        self.armed > 0
    }

    /// Whether any open range is still pending (no emission has armed it):
    /// the next emission will arm.
    pub(super) fn has_pending(&self) -> bool {
        self.pending > 0
    }

    /// Open `count` ranges: `count` more ranges, each unarmed until the
    /// next emission.
    pub(super) fn open(&mut self, count: usize) {
        self.pending += count;
    }

    /// Fold one consumed input step into the height side of `gap`.
    ///
    /// `h` moved while every `m` stayed: exactly the innermost range's
    /// `gap` shifts; the differences and followers are height-free.
    pub(super) fn fold_height(&mut self, sign: Sign, magnitude: &Int) {
        if self.armed > 0 {
            fold_signed_int(&mut self.gap, sign, magnitude);
        }
    }

    /// Close the innermost range, merging its minimum into its parent.
    ///
    /// **Callers close only armed ranges**: every range a client opens is
    /// armed by an emission before its close arrives — both sweeps emit
    /// inside every range they open, since a closing node's leaves have all
    /// been consumed — so no pending range is ever open at a close
    /// (debug-asserted below).
    ///
    /// Monotone nesting makes the merge free — the parent's minimum already
    /// reflects every inner emission (propagation kept it live) — and the
    /// latent makes it O(1): a popped zero run decrements; a popped nonzero
    /// boundary MOVES into the latent register (minting it, or dying by
    /// `merge_into_wider` into a live one), leaving `gap` and the followers
    /// untouched. Each active follower goes anchor-relative by its one-bit tag
    /// instead of absorbing a fold, so a close never touches a follower digit.
    /// The last armed range's close retires the web: `gap` and any latent drop
    /// unread (followers are already dead, so no surviving relation needs
    /// re-anchoring).
    ///
    /// The outcome carries the popped payload where one was stacked
    /// ([`Close::Parked`]). The fill walk discards the whole outcome — not
    /// just the payload — while the min-ticks fold dispatches on all three
    /// arms.
    pub(super) fn close(&mut self) -> Close<P> {
        debug_assert_eq!(
            self.pending, 0,
            "callers close only armed ranges: an emission armed every open range before its close"
        );
        debug_assert!(
            self.armed > 0,
            "a close finds an armed range: every open range was armed before it closes"
        );
        self.armed -= 1;
        if self.armed == 0 {
            debug_assert!(self.diffs.is_empty(), "no differences without ranges");
            debug_assert!(
                self.followers.iter().all(Option::is_none),
                "followers die before their anchor web does"
            );
            self.anchor_relative = [false; FOLLOWER_SLOTS];
            if let Some(latent) = self.latent.take() {
                self.retire(latent);
            }
            let gap = core::mem::take(&mut self.gap);
            self.retire(gap);
            return Close::Retired;
        }
        match self.diffs.pop().expect("armed > 1 has a difference record") {
            Entry::ZeroRun(count) => {
                if count > 1 {
                    self.diffs.push(Entry::ZeroRun(count - 1));
                }
                Close::ZeroRun
            }
            Entry::Diff { boundary, payload } => {
                // m widens from the child's to the parent's, the boundary
                // lower; the anchor stays where it is and the boundary parks in
                // the latent (`Λ += boundary` by `merge_into_wider`, or the
                // mint move).
                self.park(boundary);
                Close::Parked(payload)
            }
        }
    }

    /// Park a popped boundary in the latent register: a move, never a fold of
    /// the wide side.
    ///
    /// Active followers were exact against the old anchor state, so tagging
    /// them anchor-relative is value-preserving: a mint finds them `m`-exact
    /// with `A = m_old`, a merge finds them already tagged (a live latent keeps
    /// every active follower tagged).
    fn park(&mut self, boundary: Boundary) {
        match self.latent.take() {
            None => {
                for slot in 0..self.followers.len() {
                    debug_assert!(
                        !self.anchor_relative[slot],
                        "a set tag never outlives its latent"
                    );
                    if self.followers[slot].is_some() {
                        self.anchor_relative[slot] = true;
                    }
                }
                let latent = match boundary {
                    Boundary::Word(word) => {
                        let mut latent = self.lease();
                        latent.add_u64(word);
                        latent
                    }
                    Boundary::Wide(latent) => latent,
                };
                self.latent = Some(latent);
            }
            Some(mut latent) => {
                debug_assert!(
                    (0..self.followers.len())
                        .all(|slot| self.followers[slot].is_none() || self.anchor_relative[slot]),
                    "a live latent keeps every active follower tagged"
                );
                match boundary {
                    Boundary::Word(word) => latent.add_u64(word),
                    Boundary::Wide(wide) => {
                        let drained = latent.merge_into_wider(wide);
                        self.retire(drained);
                    }
                }
                self.latent = Some(latent);
            }
        }
    }

    /// Retire the latent into the true minimum.
    ///
    /// The anchor re-bases to `m` (`gap += Λ` by `merge_into_wider`), each
    /// tagged follower resolves by one fold of the dying latent (its
    /// death-event fan-out), and the tags clear. A no-op while no latent lives.
    ///
    /// Callers fund the death: a comparable-scale decision (the merge's
    /// near-cancellation), an emission whose output code the latent's width
    /// widens, or a re-anchor riding one — every caller resolves at a point
    /// where an emission or a comparable-scale collapse prices the fold.
    pub(super) fn resolve_latent(&mut self) {
        let Some(latent) = self.latent.take() else {
            return;
        };
        for slot in 0..self.followers.len() {
            if self.anchor_relative[slot] {
                self.followers[slot]
                    .as_mut()
                    .expect("a set tag rides an active follower")
                    .sub_accum(&latent);
                self.anchor_relative[slot] = false;
            }
        }
        let drained = self.gap.merge_into_wider(latent);
        self.retire(drained);
    }

    /// Whether a latent boundary is live (the anchor sits above the
    /// true minimum).
    pub(super) fn latent_live(&self) -> bool {
        self.latent.is_some()
    }

    /// Whether an emission at the current height (`v = h`) strictly
    /// undercuts the innermost tracked minimum.
    ///
    /// `v − A = gap`: one amortized sign read answers at or above the anchor —
    /// at or above the minimum, nothing changes and nothing further is read. A
    /// drop below the anchor is decided against the latent by the domination
    /// ladder ([`Self::decide_undercut_through_latent`]); a comparable-scales
    /// collapse re-bases the anchor to `m` and the final re-test reads the
    /// plain sign. A `true` return leaves `gap` still holding `v − A`
    /// (negative) for the undercut that must follow.
    ///
    /// May retire the latent (a funded collapse): the web's *value* is
    /// unchanged, its representation is not — unlike the fold-and-restore
    /// readers ([`compare_above_vs`](Self::compare_above_vs),
    /// [`bridge_add_gap`](Self::bridge_add_gap)), which restore exactly.
    pub(super) fn undercuts_here(&mut self) -> bool {
        // v − A = gap: at or above the anchor is at or above the minimum.
        if self.gap.sign() != Ordering::Less {
            return false;
        }
        // v < A: only a drop past the latent too is a true undercut.
        if self.latent.is_some() && !self.decide_undercut_through_latent() {
            return false;
        }
        // A collapse may have re-based the anchor to m; re-test plainly.
        self.gap.sign() == Ordering::Less
    }

    /// Decide a drop below the anchor (`gap < 0` holding `v − A`) against the
    /// true minimum `m = A − Λ` while a latent lives.
    ///
    /// Top-index domination answers scale-disparate cases in O(1): a dominating
    /// latent means `m < v < A` — return false, nothing changes; a dominated
    /// one means a true undercut — return true with the latent left live for
    /// the undercut's residue to annihilate. Comparable tops retire the latent
    /// (the near-cancellation funds the merge, and re-widening it costs the
    /// input a fresh climb) and return true for the caller's plain re-test
    /// against the re-based anchor.
    fn decide_undercut_through_latent(&mut self) -> bool {
        let gap_floor = self.gap.digit_count() - 1;
        let latent = self.latent.as_mut().expect("the caller saw a live latent");
        // Collapse for an honest top before the domination reads.
        let _sign = latent.sign();
        debug_assert_eq!(_sign, Ordering::Greater, "the latent is strictly positive");
        if latent.sign_dominates_at(gap_floor).1 {
            return false;
        }
        let latent_floor = latent.digit_count() - 1;
        if self.gap.sign_dominates_at(latent_floor).1 {
            return true;
        }
        self.resolve_latent();
        true
    }

    /// Drop the innermost minimum to the current height (`gap < 0` holding `v −
    /// A`, the true-undercut decision already made —
    /// [`undercuts_here`](Self::undercuts_here)).
    ///
    /// `gap` dies into the residue and a fresh zero seats the new anchor `A =
    /// v`; the drop then drives outward ([`drop_below`](Self::drop_below)).
    pub(super) fn undercut(&mut self, on_die: impl FnMut(P)) {
        let fresh = self.lease();
        let mut residue = core::mem::replace(&mut self.gap, fresh);
        residue.negate();
        self.drop_below(residue, on_die);
    }

    /// Drive a drop below the old anchor outward, `residue = A − v > 0` already
    /// negated positive and `gap` already re-seated by the caller.
    ///
    /// Each active follower absorbs the anchor-relative drop `A − v` — one fold
    /// that also resolves a set tag, since a tagged follower's content is
    /// anchor-relative and the new anchor is `v` itself — a live latent
    /// annihilates into the residue (the drop dominated it), and the drop
    /// propagates through the difference stack.
    fn drop_below(&mut self, mut residue: Accumulator, on_die: impl FnMut(P)) {
        for slot in 0..self.followers.len() {
            if let Some(follower) = &mut self.followers[slot] {
                debug_assert_eq!(
                    self.anchor_relative[slot],
                    self.latent.is_some(),
                    "a live latent keeps every active follower tagged"
                );
                follower.sub_accum(&residue);
                self.anchor_relative[slot] = false;
            }
        }
        if let Some(latent) = self.latent.take() {
            // The annihilation: residue = (A − v) − Λ = m − v > 0.
            residue.sub_accum(&latent);
            self.retire(latent);
        }
        self.propagate(residue, on_die);
    }

    /// Arm every pending range at an emission `v = h` exactly: the
    /// anchor-relative offset is the dying `gap` itself, moved out whole with
    /// no fold at all, and a fresh zero seats the new anchor `A = v`.
    ///
    /// `payload` mints lazily: only the arms that store or kill a boundary read
    /// it (the pushed-above arm stacks it; an arming undercut hands it straight
    /// to `on_die` — [`push_boundary`](Self::push_boundary)).
    pub(super) fn arm_at_height(&mut self, payload: impl FnOnce() -> P, on_die: impl FnMut(P)) {
        debug_assert!(
            self.pending > 0,
            "an arm fires only while a pending range awaits it"
        );
        let pending = core::mem::replace(&mut self.pending, 0);
        if self.armed == 0 {
            debug_assert!(
                self.followers.iter().all(Option::is_none),
                "followers attach after the first arming"
            );
            debug_assert!(self.latent.is_none(), "the latent dies with the web");
            self.armed = pending;
            let fresh = self.lease();
            let old = core::mem::replace(&mut self.gap, fresh);
            self.retire(old);
            self.push_zeros(pending - 1);
            return;
        }
        let fresh = self.lease();
        let offset = core::mem::replace(&mut self.gap, fresh);
        self.armed += pending;
        self.push_boundary(offset, pending, payload, on_die);
    }

    /// Arm every pending range at the emission `v = h − below`, moving `below`
    /// in as the new `gap`.
    ///
    /// The accumulator moves into the web — wide content is stored once and
    /// read only at the arming boundary it prices. The anchor-relative offset
    /// `v − A_old = gap_old − below` costs one fold of the narrow dying side.
    pub(super) fn arm_below(
        &mut self,
        below: Accumulator,
        payload: impl FnOnce() -> P,
        on_die: impl FnMut(P),
    ) {
        debug_assert!(
            self.pending > 0,
            "an arm fires only while a pending range awaits it"
        );
        let pending = core::mem::replace(&mut self.pending, 0);
        if self.armed == 0 {
            debug_assert!(
                self.followers.iter().all(Option::is_none),
                "followers attach after the first arming"
            );
            debug_assert!(self.latent.is_none(), "the latent dies with the web");
            self.armed = pending;
            let old = core::mem::replace(&mut self.gap, below);
            self.retire(old);
            self.push_zeros(pending - 1);
            return;
        }
        // The anchor-relative offset: offset = v − A_old = gap_old − below.
        // gap_old dies into it (a move of the buffer, one narrow fold), and
        // the boundary bookkeeping recycles any parked latent.
        let mut offset = core::mem::replace(&mut self.gap, below);
        offset.sub_accum(&self.gap);
        self.armed += pending;
        self.push_boundary(offset, pending, payload, on_die);
    }

    /// Arm bookkeeping shared by the arming paths, after `gap` is seated for
    /// the new anchor `A = v` and `armed` counts the new ranges.
    ///
    /// Folds the anchor-relative offset `offset = v − A_old` into each active
    /// follower (resolving set tags — the offset is exactly the tagged
    /// content's shift to the new anchor, where the latent is spent), merges
    /// the offset with any latent into the true boundary `v − m_old`, and
    /// pushes it (a positive difference, compacted to machine width when it
    /// fits), counts it (an exact meet), or propagates it (an arming undercut's
    /// residue). Only the pushed-above arm mints the payload; an arming
    /// undercut's payload dies by `on_die` before the residue drives outward,
    /// and an exact meet touches no payload at all — the reigning state
    /// continues.
    fn push_boundary(
        &mut self,
        offset: Accumulator,
        pending: usize,
        payload: impl FnOnce() -> P,
        mut on_die: impl FnMut(P),
    ) {
        for slot in 0..self.followers.len() {
            if let Some(follower) = &mut self.followers[slot] {
                follower.add_accum(&offset);
                self.anchor_relative[slot] = false;
            }
        }
        let mut offset = offset;
        if let Some(latent) = self.latent.take() {
            let drained = offset.merge_into_wider(latent);
            self.retire(drained);
        }
        match offset.sign() {
            Ordering::Greater => {
                let boundary = self.compact(offset);
                self.diffs.push(Entry::Diff {
                    boundary,
                    payload: payload(),
                });
                self.push_zeros(pending - 1);
            }
            Ordering::Equal => {
                self.retire(offset);
                self.push_zeros(pending);
            }
            Ordering::Less => {
                on_die(payload());
                let mut residue = offset;
                residue.negate();
                self.propagate(residue, on_die);
                self.push_zeros(pending);
            }
        }
    }

    /// Drive an undercut's residue (`residue > 0`, the drop below the old
    /// innermost minimum) outward through the difference stack.
    ///
    /// Zero runs pass whole in O(1); word-scale boundaries fold in O(1)
    /// outright; each wide difference the drop exceeds dies by one fold *into
    /// the residue* at the difference's own width — its payload surrendered to
    /// `on_die` — and the stopping range absorbs the one surviving fold: the
    /// residue's terminal death into the difference that outlasts it. Top-index
    /// domination decides each wide hop's direction before any fold, so the
    /// dying side always funds the fold that consumes it and the wide side of a
    /// scale-disparate hop is never read across its width while it survives;
    /// only comparable scales fold undecided, the near-cancellation pricing
    /// either direction. The caller has already adjusted `gap` and the
    /// followers.
    fn propagate(&mut self, residue: Accumulator, mut on_die: impl FnMut(P)) {
        let mut residue = residue;
        // Deferred zero-run bookkeeping: every consumed entry whose range's
        // minimum now equals the new innermost one's counts here, and one flush
        // after the loop pushes the merged run — every escape path below
        // reaches that flush.
        let mut zeros = 0usize;
        // Loop invariant: `residue > 0` is always the drop still to apply at
        // the current stack position. Every arm either kills it (the stopping
        // range absorbs it, or the stack empties — break), consumes a
        // difference whole and keeps it going, or replaces it with the
        // surviving remainder of a comparable-scale fold.
        loop {
            match self.diffs.pop() {
                None => {
                    // The outermost armed range dropped; nothing is
                    // outward of it.
                    self.retire(residue);
                    break;
                }
                Some(Entry::ZeroRun(count)) => zeros += count,
                Some(Entry::Diff {
                    boundary: Boundary::Word(word),
                    payload,
                }) => {
                    // A word-scale boundary folds outright: O(1) against any
                    // residue.
                    residue.sub_u64(word);
                    match residue.sign() {
                        Ordering::Greater => {
                            // The boundary died; the drop keeps going.
                            on_die(payload);
                            zeros += 1;
                        }
                        Ordering::Equal => {
                            // Exact meet: this range's minimum now equals
                            // the new innermost one's.
                            on_die(payload);
                            self.retire(residue);
                            zeros += 1;
                            break;
                        }
                        Ordering::Less => {
                            // The boundary survives, shrunk: the dying
                            // residue's terminal fold already happened.
                            residue.negate();
                            let boundary = self.compact(residue);
                            self.diffs.push(Entry::Diff { boundary, payload });
                            break;
                        }
                    }
                }
                Some(Entry::Diff {
                    boundary: Boundary::Wide(mut diff),
                    payload,
                }) => {
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
                    if residue.digit_count() >= diff.digit_count() + 2 {
                        let (sign, decided) = residue.sign_dominates_at(diff.digit_count() - 1);
                        debug_assert!(
                            !decided || sign == Ordering::Greater,
                            "the residue is strictly positive"
                        );
                        if decided && sign == Ordering::Greater {
                            // The residue dwarfs the difference: it dies by
                            // its one fold into the surviving residue, which
                            // stays positive and keeps dropping.
                            residue.sub_accum(&diff);
                            self.retire(diff);
                            on_die(payload);
                            zeros += 1;
                            continue;
                        }
                    }
                    if diff.digit_count() >= residue.digit_count() + 2 {
                        let (sign, decided) = diff.sign_dominates_at(residue.digit_count() - 1);
                        debug_assert!(
                            !decided || sign == Ordering::Greater,
                            "stacked differences are strictly positive"
                        );
                        if decided && sign == Ordering::Greater {
                            // The difference dwarfs the residue: the drop stops
                            // here, and the dying residue's terminal fold
                            // shrinks the survivor.
                            diff.sub_accum(&residue);
                            self.retire(residue);
                            self.diffs.push(Entry::Diff {
                                boundary: Boundary::Wide(diff),
                                payload,
                            });
                            break;
                        }
                    }
                    // Comparable scales: the near-cancellation prices the fold
                    // — the dying side's digits within a constant, whichever
                    // side dies.
                    diff.sub_accum(&residue);
                    self.retire(residue);
                    match diff.sign() {
                        Ordering::Greater => {
                            // The drop stops here: the difference survives,
                            // shrunk.
                            let boundary = self.compact(diff);
                            self.diffs.push(Entry::Diff { boundary, payload });
                            break;
                        }
                        Ordering::Equal => {
                            // Exact meet: this range's minimum now equals
                            // the new innermost one's.
                            on_die(payload);
                            self.retire(diff);
                            zeros += 1;
                            break;
                        }
                        Ordering::Less => {
                            // The difference dies; the remainder keeps
                            // dropping.
                            on_die(payload);
                            diff.negate();
                            residue = diff;
                            zeros += 1;
                        }
                    }
                }
            }
        }
        self.push_zeros(zeros);
    }

    /// Store a strictly positive difference at machine width when the
    /// instantiation compacts and the value fits, retiring its buffer; keep the
    /// accumulator otherwise.
    ///
    /// The width test reads the digit count alone — two digits cover a `u64` at
    /// the accumulator's base-2^32 digit width, so anything wider can never fit
    /// — and a wide difference is therefore never normalized just to learn it
    /// would not fit.
    fn compact(&mut self, difference: Accumulator) -> Boundary {
        if self.compact_words && difference.digit_count() <= 2 {
            let (sign, magnitude) = difference.sign_magnitude();
            debug_assert_eq!(sign, Ordering::Greater, "boundaries are strictly positive");
            if let Ok(word) = u64::try_from(&magnitude) {
                self.retire(difference);
                return Boundary::Word(word);
            }
        }
        Boundary::Wide(difference)
    }

    /// Push `count` zero-difference ranges, merging with a top run.
    fn push_zeros(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        if let Some(Entry::ZeroRun(run)) = self.diffs.last_mut() {
            *run += count;
        } else {
            self.diffs.push(Entry::ZeroRun(count));
        }
    }

    /// A cleared accumulator, pooled when one is available.
    pub(super) fn lease(&mut self) -> Accumulator {
        self.pool.pop().unwrap_or_default()
    }

    /// Retire a dying accumulator into the pool, clearing it.
    pub(super) fn retire(&mut self, mut dying: Accumulator) {
        dying.reset();
        self.pool.push(dying);
    }
}

/// The payload-free surface the fill walk and its pre-scan drive.
///
/// Emissions at, above, and below the running height, the raise-decision reads,
/// the follower slots, and the anchor-switch bridges. Every method forwards to
/// the shared discipline above with trivial hooks; only the emission vocabulary
/// and the priced-offset handling live here.
impl MinWeb<()> {
    /// Record an emission at the current height (`v = h`).
    pub(super) fn emit_here(&mut self) {
        if self.pending > 0 {
            // v = h exactly: the dying gap is itself the anchor-relative
            // offset, moved out whole with no fold
            // ([`arm_at_height`](Self::arm_at_height)).
            self.arm_at_height(|| (), |()| ());
            return;
        }
        if !self.undercuts_here() {
            return;
        }
        // Undercut: m drops to v, gap dies into the residue and re-seats
        // at zero.
        let mut residue = core::mem::take(&mut self.gap);
        residue.negate();
        self.drop_below(residue, |()| ());
        self.gap = self.lease();
    }

    /// Record an emission at `v = h + offset` for a signed, priced offset (a
    /// consuming scan's extremum, or a raise decided against it).
    ///
    /// Five paths, in order:
    ///
    /// 1. A zero offset delegates to [`emit_here`](Self::emit_here).
    /// 2. Pending ranges arm at `v` ([`arm_below`](Self::arm_below) with
    ///    `below = −offset`).
    /// 3. No latent and a word-scale offset: post-sign domination reads the
    ///    answer with no fold — a dominating-positive `gap` returns; a
    ///    wide-negative `gap` is an undercut whose residue dwarfs the
    ///    offset.
    /// 4. Otherwise the priced offset folds into `gap`, and three gates each
    ///    restore the fold on exit: `v` at or above the anchor; a drop that
    ///    does not carry through the latent; a collapse that re-based the
    ///    anchor with `v` not below it.
    /// 5. A true undercut. The fold is deliberately *not* restored — the
    ///    folded offset funds the residue (`gap` holds `v − A`, negated
    ///    into `m − v`) — and the re-seated `gap` is `h − v = −offset`
    ///    exactly.
    pub(super) fn emit_offset(&mut self, offset: &Signed) {
        if offset.is_zero() {
            self.emit_here();
            return;
        }
        if self.pending > 0 {
            // below = h − v = −offset.
            let mut below = self.lease();
            fold_signed_int(&mut below, offset.sign.negate(), &offset.magnitude);
            self.arm_below(below, || (), |()| ());
            return;
        }
        // v − A = gap + offset. With no latent, post-sign domination decides
        // against a word-scale offset with no fold (with one live, the
        // O(offset) fold below is cheap for a word and the ladder decides).
        if self.latent.is_none() && offset.magnitude.to_u64().is_some() {
            let (sign, decided) = self.gap.sign_dominates_word();
            if decided {
                if sign == Ordering::Greater {
                    web_traffic::record(web_traffic::Decision::DominatedAbove);
                    return;
                }
                web_traffic::record(web_traffic::Decision::DominatedUndercut);
                // gap wide-negative: v sits far below the minimum; the
                // drop dwarfs the offset. Residue = m − v = −gap − offset.
                let mut residue = core::mem::take(&mut self.gap);
                residue.negate();
                fold_signed_int(&mut residue, offset.sign.negate(), &offset.magnitude);
                for follower in self.followers.iter_mut().flatten() {
                    follower.sub_accum(&residue);
                }
                let mut gap = self.lease();
                fold_signed_int(&mut gap, offset.sign.negate(), &offset.magnitude);
                self.gap = gap;
                self.propagate(residue, |()| ());
                return;
            }
            web_traffic::record(web_traffic::Decision::Undecided);
        }
        // Fold the priced side; restore it unless it funds the residue.
        fold_signed_int(&mut self.gap, offset.sign, &offset.magnitude);
        if self.gap.sign() != Ordering::Less {
            // v at or above the anchor, hence at or above the minimum.
            fold_signed_int(&mut self.gap, offset.sign.negate(), &offset.magnitude);
            return;
        }
        // v < A: only a drop past the latent too is a true undercut.
        if self.latent.is_some() && !self.decide_undercut_through_latent() {
            fold_signed_int(&mut self.gap, offset.sign.negate(), &offset.magnitude);
            return;
        }
        if self.gap.sign() != Ordering::Less {
            // A collapse re-based the anchor to m and v is not below it.
            fold_signed_int(&mut self.gap, offset.sign.negate(), &offset.magnitude);
            return;
        }
        // Undercut: gap holds v − A, offset stays folded to fund the
        // residue; the re-seated gap is h − v = −offset exactly.
        let mut residue = core::mem::take(&mut self.gap);
        residue.negate();
        self.drop_below(residue, |()| ());
        let mut gap = self.lease();
        fold_signed_int(&mut gap, offset.sign.negate(), &offset.magnitude);
        self.gap = gap;
    }

    /// Record an emission at `v = h − below` where `below` arrives as a funded
    /// accumulator (a resolved memoized minimum), arming the pending range that
    /// must exist for it.
    ///
    /// The accumulator moves into the web — it becomes the new `gap` — so wide
    /// content is stored once and read only at the arming boundary it prices.
    pub(super) fn emit_below_accum(&mut self, below: Accumulator) {
        debug_assert!(self.pending > 0, "a raise arms its own node's range");
        self.arm_below(below, || (), |()| ());
    }

    /// Whether `h + above` reaches the innermost armed minimum:
    /// `Ordering::Less` means strictly below `m`.
    ///
    /// The raise arms' decision read. Post-sign domination answers a word-scale
    /// offset with no fold; otherwise the priced offset is folded and restored,
    /// with the latent ladder deciding drops that land between the true minimum
    /// and the anchor (domination in O(1), or a funded collapse at comparable
    /// scales).
    ///
    /// May retire the latent (a funded collapse): the web's *value* is
    /// unchanged, its representation is not — unlike the fold-and-restore
    /// readers, which restore exactly.
    pub(super) fn compare_above(&mut self, above: &Signed) -> Ordering {
        debug_assert!(self.armed > 0, "a raise compares against an armed range");
        if self.latent.is_none() && above.magnitude.to_u64().is_some() {
            let (sign, decided) = self.gap.sign_dominates_word();
            if decided {
                return sign;
            }
        }
        fold_signed_int(&mut self.gap, above.sign, &above.magnitude);
        let mut sign = self.gap.sign();
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
                        self.gap.sign()
                    }
                }
            };
        }
        fold_signed_int(&mut self.gap, above.sign.negate(), &above.magnitude);
        sign
    }

    /// Whether `h + above` reaches a minimum sitting `arm_offset` above the
    /// anchor (`A + arm_offset`, signed): `Ordering::Less` means strictly below
    /// it.
    ///
    /// The memo consumer's decision read: `(h + above) − (A + arm_offset) = gap
    /// − arm_offset + above`, folded and restored — `above` is priced,
    /// `arm_offset` is anchor-relative dying content (a ledger link net of the
    /// taken relation, narrow whenever the reference minima agree), and the
    /// latent never participates: the anchor-relative target cancels it
    /// exactly, so the read costs the operands' own widths no matter how wide
    /// the parked boundary is.
    pub(super) fn compare_above_vs(
        &mut self,
        above: &Signed,
        arm_offset: &Accumulator,
    ) -> Ordering {
        debug_assert!(self.armed > 0, "a raise compares against an armed range");
        self.gap.sub_accum(arm_offset);
        fold_signed_int(&mut self.gap, above.sign, &above.magnitude);
        let sign = self.gap.sign();
        fold_signed_int(&mut self.gap, above.sign.negate(), &above.magnitude);
        self.gap.add_accum(arm_offset);
        sign
    }

    /// Arm the pending range at a minimum `arm_offset` above the anchor
    /// (`v = A + arm_offset`, signed and dying here).
    ///
    /// The memo consumer's arming: the new `gap = gap_old − arm_offset` needs
    /// no read of the old web beyond `arm_offset`'s own width, and `arm_offset`
    /// recycles any parked latent — the true boundary `v − m` is `arm_offset +
    /// Λ`, realized by folding the narrow dying offset into the latent's buffer
    /// (`merge_into_wider`) and pushing the merged buffer (or, negated,
    /// propagating it as the undercut's residue).
    pub(super) fn arm_relative(&mut self, arm_offset: Accumulator) {
        debug_assert!(self.pending > 0, "a raise arms its own node's range");
        debug_assert!(self.armed > 0, "a relative arming needs an armed anchor");
        let pending = core::mem::replace(&mut self.pending, 0);
        self.gap.sub_accum(&arm_offset);
        self.armed += pending;
        self.push_boundary(arm_offset, pending, || (), |()| ());
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
    pub(super) fn follower_set(&mut self, slot: usize, follower: Accumulator) {
        debug_assert!(self.followers[slot].is_none(), "one follower per slot");
        debug_assert!(self.armed > 0, "a follower needs an armed anchor");
        self.anchor_relative[slot] = self.latent.is_some();
        self.followers[slot] = Some(follower);
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
        self.anchor_relative[slot] = false;
        self.followers[slot].take().expect("the follower is active")
    }

    /// Materialize a dying accumulator: collapse, then read the sign and
    /// magnitude (held digits exceed the value's width by at most the collapse
    /// slack), retiring the buffer.
    pub(super) fn materialize(&mut self, mut dying: Accumulator) -> Signed {
        // Collapse for an honest width before the read-out: `sign()` is
        // called for its compaction side effect, the value unread.
        let _sign = dying.sign();
        let (sign, magnitude) = dying.sign_magnitude();
        self.retire(dying);
        Signed::from_sign_magnitude(sign, magnitude)
    }

    /// Fold the stored `gap` into `delta` (`delta += h − A`): the
    /// watermark-to-height anchor switch's bridge read, priced by the code
    /// emitted at the switch that needs it.
    ///
    /// Deliberately anchor-relative: the caller's `delta` is a follower taken
    /// raw, so a live latent cancels symbolically — `(f_true) + (h − m) =
    /// (f_stored − Λ) + (gap + Λ) = f_stored + gap` — and no latent digit is
    /// ever touched by this switch.
    pub(super) fn bridge_add_gap(&mut self, delta: &mut Accumulator) {
        delta.add_accum(&self.gap);
    }

    /// Fold the innermost `gap` out of `delta` (`delta −= h − m`): the
    /// height-to-watermark anchor switch's bridge read.
    ///
    /// The caller resolves any latent first
    /// ([`resolve_latent`](Self::resolve_latent) — the switch's emission
    /// re-anchors to the true minimum, which retires the latent anyway), so
    /// `gap` is exact here.
    pub(super) fn bridge_sub_gap(&mut self, delta: &mut Accumulator) {
        debug_assert!(
            self.latent.is_none(),
            "the height-to-watermark switch resolves the latent first"
        );
        delta.sub_accum(&self.gap);
    }
}

#[cfg(test)]
mod tests;
