//! Nested range minima for a streaming skyline walk.
//!
//! # Purpose and constraint
//!
//! A skyline walk encounters values in leaf order while its tree ranges open
//! and close in properly nested order. For every such range, this module
//! computes
//!
//! `minimum(values emitted between open and close)`.
//!
//! It also supports comparing a prospective value with the innermost minimum
//! and carrying caller-owned values relative to that minimum. These are one
//! family of operations over the same nested minima, not separate indexes.
//!
//! Correctness alone is not enough. Skyline heights are delta-coded and
//! arbitrary precision. A short input can build a wide running height and then
//! contain many tiny deltas or deeply nested ranges. A direct implementation
//! could make every tiny event copy, scan, or update that old wide value,
//! allowing a pathological input to cause far more work than its bytes account
//! for. The earlier wide delta pays for processing its digits once; it must not
//! grant every later one-byte event another traversal. The tracker is designed
//! around the stronger rule that topology costs
//! constant work, while arbitrary-precision work is proportional to the input
//! differences or stored state whose digits it consumes. A wide value may
//! survive many cheap operations, but those operations must not traverse it.
//!
//! [`Accumulator`] provides the arithmetic needed to enforce that rule: folds
//! read the operand rather than the receiver, sign reads are amortized constant
//! time, leading digits can decide comparisons between well-separated widths,
//! and `merge_into_wider` reads the narrower operand while retaining the wider
//! buffer. The representation below arranges the algorithm so every call uses
//! one of those bounded operations.
//!
//! # Abstract model
//!
//! [`RangeMinima`] is a stack of open ranges. A client opens ranges, advances
//! one running height `h`, reports values emitted inside the ranges, and closes
//! them in reverse order. A range is *pending* until its first emission gives
//! it a minimum, then *armed* until it closes.
//!
//! [`RangeMinima::new`] creates the empty stack. [`RangeMinima::open`] adds
//! pending ranges, and [`RangeMinima::fold_height`] advances `h`. A client with
//! payloads reports a first emission through [`RangeMinima::arm_at_height`] or
//! [`RangeMinima::arm_below`], then uses [`RangeMinima::undercut`] when a later
//! emission lowers the minimum. The payload-free form provides
//! [`RangeMinima::emit_here`] and [`RangeMinima::emit_offset`] for those steps.
//! [`RangeMinima::close`] consumes ranges from the inside out; closing the final
//! armed range makes the stack empty again.
//!
//! # Minima as differences
//!
//! An outer range contains every value of its inner range, so armed minima form
//! a monotone chain:
//!
//! `outer_min <= ... <= inner_min`.
//!
//! Storing every absolute minimum would duplicate any wide prefix shared by
//! many nested ranges. The stack instead stores each adjacent difference:
//!
//! `boundary = inner_min - outer_min >= 0`.
//!
//! Wide storage is therefore proportional to the differences present in the
//! input, not to their depth times a shared absolute value. Equal minima become
//! zero boundaries, compressed into counted runs.
//!
//! Each positive boundary also owns one opaque payload `P`. The minimum
//! algorithm only moves or returns it. The payload lets a client suspend state
//! for the outer minimum while a different inner minimum is current. For
//! example, if the outer minimum is `10` and the inner minimum is `17`, their
//! boundary is `7`. Closing the inner range returns the state for `10`; an
//! emission below `10` removes the boundary and sends the state to the
//! retirement callback. Equal minima need no separate suspended state. Moving
//! the payload with its boundary avoids copying or reconstructing client state.
//! Payload construction and retirement are client work, outside this module's
//! arithmetic bound. A constructed payload is either stored once and later
//! returned or retired, or retired immediately when a new minimum displaces
//! its state.
//!
//! # One height relative to one anchor
//!
//! Let `m` be the innermost armed minimum. The stack keeps one accumulator
//! `gap = h - A`, where the anchor `A` normally equals `m`. Every height change
//! folds into this single value, independent of the number of open ranges.
//! [`Accumulator`] makes a fold proportional to the input operand rather than
//! the possibly much wider running value, and answers sign queries in
//! amortized constant time.
//!
//! Closing the innermost range exposes its parent. If their boundary is `b`,
//! the parent's minimum is `m - b`. Immediately changing `gap` would make a
//! sequence of closes repeatedly modify the same wide accumulator. Instead the
//! anchor stays fixed and the boundary moves into the deferred distance
//!
//! `Λ = A - m > 0`.
//!
//! The true distance is `h - m = gap + Λ`. Further closes merge their
//! boundaries into `Λ`; [`Accumulator::merge_into_wider`] reads only the
//! narrower operand and keeps the wider buffer. An operation resolves `Λ` into
//! `gap` only when it needs the true minimum. Thus each closed boundary is
//! moved or folded once, while a wide surviving value is not revisited at
//! every nesting level.
//!
//! # Emissions and undercuts
//!
//! The first emission inside pending ranges arms all of them. Its distance
//! above the old minimum becomes a boundary; equality extends a zero run. If
//! the emission is below the old minimum, define
//!
//! `drop = old_min - emitted_value > 0`.
//!
//! The drop moves outward through the boundary stack. At each boundary `b`:
//!
//! - `drop < b`: the outer minimum stays put, `b` shrinks by `drop`, and the
//!   walk stops;
//! - `drop = b`: the two minima meet, so the boundary becomes zero and the
//!   walk stops;
//! - `drop > b`: the outer minimum also falls, the boundary becomes zero, and
//!   `drop - b` continues outward.
//!
//! [`RangeMinima::propagate_drop`] implements these cases explicitly. It skips
//! a zero run as one record. For wide values,
//! [`Accumulator::sign_dominates_at`] orders values with well-separated widths
//! from their leading digits. Subtraction then reads the smaller value, not the
//! wide survivor. Values close enough in width for that test to be inconclusive
//! are subtracted once at comparable cost. A boundary crossed by a drop is
//! permanently removed, so no later emission can charge its width again.
//!
//! # Followers
//!
//! A client may attach a *follower* holding `m - X` for one of its own values
//! `X`. While `Λ` exists, the stored follower remains relative to `A`; a bit
//! records that subtracting `Λ` is still owed. A close can therefore defer the
//! same shift for `gap` and every follower without traversing any of them.
//!
//! # Use in `before`
//!
//! The fill walk needs subtree minima to decide the value of a collapsed or
//! raised child. It uses `RangeMinima<()>`: the stack supplies comparisons and
//! emitted minima. One follower tracks the next output delta relative to the
//! minimum; another tracks the difference from a memoized range minimum. Both
//! move automatically when the tracked minimum moves, so the walk need not
//! reconstruct either endpoint as an absolute height. Its pre-scan uses the
//! same abstraction to obtain minima needed before their ranges are emitted.
//!
//! [`min_ticks`](super::query::min_ticks) computes
//! `sum(leaf heights) - sum(internal subtree minima)`. Each internal subtree is
//! one range, so closing the range contributes its tracked minimum. That fold
//! uses a payload for the accounting record attached to each distinct minimum.
//! If an inner range has a higher minimum, the outer record waits on their
//! boundary; a close resumes it, while an undercut that removes the boundary
//! settles it permanently.
//!
//! # Invariants
//!
//! - Pending ranges are inside every armed range.
//! - Expanding zero runs yields exactly `armed - 1` boundaries.
//! - Every boundary is nonnegative, and every positive boundary has one
//!   payload.
//! - `gap = h - A`. If `Λ` exists, `Λ = A - m > 0`; otherwise `A = m`.
//! - A follower is either `m - X`, or is tagged as the deferred form `A - X`.
//! - With no armed range, the boundary stack and deferred state are empty.
//!
//! Together these rules ensure that each input difference and stored boundary
//! pays for at most a constant number of folds over its own width. Depth alone
//! adds compact stack records and constant-time operations; it does not
//! multiply the cost of a wide height or minimum. For a validated skyline
//! stream, the tracker therefore performs amortized arithmetic work linear in
//! the encoded value bits plus the topology events, and retains state linear in
//! those same inputs. The surrounding algorithms add their own work, but they
//! cannot use this tracker to make a small later input repeatedly traverse a
//! wide earlier value.

use core::cmp::Ordering;

use suanpan::Accumulator;

use num_bigint::{BigInt, Sign};

use crate::codec::accumulator;

use super::web_traffic;

/// Number of client values that can follow the innermost minimum.
pub(super) const FOLLOWER_SLOTS: usize = 2;

/// One positive difference between adjacent range minima.
///
/// Common differences stay in a machine word. Only differences wider than a
/// `u64` retain an accumulator, so ordinary nesting has small resident cost.
enum Boundary {
    /// A positive difference that fits in a machine word.
    Word(u64),
    /// A positive difference that requires arbitrary precision.
    Wide(Accumulator),
}

/// The result of lowering an inner minimum across one stored boundary.
///
/// If the inner minimum falls by `drop` and the boundary is
/// `inner_min - outer_min`, exactly one of three things happens:
///
/// - `drop > boundary`: the outer minimum falls too, by the remainder;
/// - `drop < boundary`: the outer minimum is unchanged and the boundary
///   shrinks;
/// - equality: the two minima meet.
enum DropOutcome<P> {
    /// The boundary vanished; continue outward with the remaining drop.
    BoundaryConsumed {
        /// `drop - boundary`, strictly positive.
        remaining_drop: Accumulator,
        /// Payload whose boundary vanished.
        retired: P,
    },
    /// The drop stopped here; retain the smaller boundary.
    DropConsumed {
        /// `boundary - drop`, strictly positive.
        remaining_boundary: Boundary,
        /// Payload that remains attached to the boundary.
        payload: P,
    },
    /// The drop and boundary were equal; both vanished.
    Equal {
        /// Payload whose boundary vanished.
        retired: P,
    },
}

/// Result of comparing a value below the anchor with the true minimum.
///
/// The third case records a representation change: comparable wide values are
/// combined, making the anchor equal to the true minimum. The caller must then
/// read `gap` again to distinguish below, equal, and above.
enum DeferredComparison {
    /// The value lies between the true minimum and the anchor.
    AboveMinimum,
    /// The value lies below the true minimum.
    BelowMinimum,
    /// The deferred distance was resolved; compare against the new anchor.
    Reanchored,
}

/// One logical record popped from [`DifferenceStack`].
enum Entry<P> {
    /// `count` consecutive boundaries of zero.
    ZeroRun(u64),
    /// One positive boundary and the client state suspended on it.
    Diff { boundary: Boundary, payload: P },
}

/// Storage class of one logical [`DifferenceStack`] record.
#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
enum EntryKind {
    /// A run of equal minima.
    ZeroRun,
    /// A difference held in one machine word.
    Word,
    /// A difference held in the wide-value stack.
    Wide,
}

/// Compact LIFO storage for the boundaries between armed ranges.
///
/// A direct `Vec<Entry<P>>` would give every zero run enough space and alignment
/// for a payload and a wide accumulator. Instead, `kinds` and `words` describe
/// every logical record, while `payloads` contains only positive boundaries and
/// `wide` contains only arbitrary-precision boundaries. Because every column is
/// popped in LIFO order, sparse columns need no per-record indices.
struct DifferenceStack<P> {
    /// The representation of each logical record.
    kinds: Vec<EntryKind>,
    /// A zero-run count or word-sized difference for each logical record.
    words: Vec<u64>,
    /// Payloads for nonzero differences.
    payloads: Vec<P>,
    /// Differences too wide for `words`.
    wide: Vec<Accumulator>,
}

impl<P> DifferenceStack<P> {
    /// Construct an empty set of synchronized storage columns.
    fn new() -> Self {
        Self {
            kinds: Vec::new(),
            words: Vec::new(),
            payloads: Vec::new(),
            wide: Vec::new(),
        }
    }

    /// Whether there are no logical boundary records.
    fn is_empty(&self) -> bool {
        debug_assert_eq!(self.kinds.len(), self.words.len());
        self.kinds.is_empty()
    }

    /// Push one positive boundary.
    ///
    /// The kind selects either the inline word or the next entry on `wide`.
    /// Every positive boundary also appends exactly one payload.
    fn push_diff(&mut self, boundary: Boundary, payload: P) {
        match boundary {
            Boundary::Word(word) => {
                self.kinds.push(EntryKind::Word);
                self.words.push(word);
            }
            Boundary::Wide(wide) => {
                self.kinds.push(EntryKind::Wide);
                self.words.push(0);
                self.wide.push(wide);
            }
        }
        self.payloads.push(payload);
    }

    /// Push `count` zero boundaries.
    ///
    /// Adjacent runs are merged, so any number of nested ranges sharing one
    /// minimum needs a single logical record.
    fn push_zeros(&mut self, count: u64) {
        if count == 0 {
            return;
        }
        if self.kinds.last() == Some(&EntryKind::ZeroRun) {
            *self.words.last_mut().expect("each kind has a value") += count;
        } else {
            self.kinds.push(EntryKind::ZeroRun);
            self.words.push(count);
        }
    }

    /// Pop the boundary nearest the innermost armed range.
    ///
    /// The kind determines which sparse columns also have a top entry. Keeping
    /// all columns in lockstep here preserves the one-payload-per-boundary
    /// invariant.
    fn pop(&mut self) -> Option<Entry<P>> {
        let kind = self.kinds.pop()?;
        let word = self.words.pop().expect("each kind has a value");
        Some(match kind {
            EntryKind::ZeroRun => Entry::ZeroRun(word),
            EntryKind::Word => Entry::Diff {
                boundary: Boundary::Word(word),
                payload: self.payloads.pop().expect("each difference has a payload"),
            },
            EntryKind::Wide => Entry::Diff {
                boundary: Boundary::Wide(self.wide.pop().expect("each wide kind has a value")),
                payload: self.payloads.pop().expect("each difference has a payload"),
            },
        })
    }
}

/// What becomes visible after closing the innermost range.
pub(super) enum Close<P> {
    /// No armed range remains.
    Retired,
    /// The parent has the same minimum, so no client state changes.
    Equal,
    /// The parent has a lower minimum; its suspended state becomes current.
    Lower(P),
}

/// A stack of nested range minima relative to one running height.
///
/// The module documentation defines the abstract stack and its equations. The
/// fields below are its compact representation: one height gap, one optional
/// deferred anchor shift, a difference stack, and a fixed number of followers.
pub(super) struct RangeMinima<P> {
    /// `h - A`, where `A` is the current anchor; zero when no range is armed.
    gap: Accumulator,
    /// Deferred distance `A - m` from the anchor to the true minimum.
    ///
    /// Present only when positive. Closing more ranges adds their boundaries
    /// here instead of repeatedly updating `gap`.
    deferred: Option<Accumulator>,
    /// Whether each follower is relative to `A` and still needs `deferred`
    /// subtracted to become relative to `m`.
    anchor_relative: [bool; FOLLOWER_SLOTS],
    /// Boundaries between armed ranges, with the innermost boundary last.
    ///
    /// Expanding zero runs produces exactly `armed - 1` boundaries.
    diffs: DifferenceStack<P>,
    /// Innermost open ranges that have not yet seen an emission.
    pending: u64,
    /// Open ranges whose minima are represented by `gap` and `diffs`.
    armed: u64,
    /// Optional client values tracking `m - X`.
    followers: [Option<Accumulator>; FOLLOWER_SLOTS],
}

impl<P> RangeMinima<P> {
    /// Construct an empty range-minimum stack.
    ///
    /// No anchor exists until the first emission arms a range. Until then,
    /// height changes are irrelevant and are deliberately not accumulated.
    pub(super) fn new() -> Self {
        RangeMinima {
            gap: Accumulator::new(),
            deferred: None,
            anchor_relative: [false; FOLLOWER_SLOTS],
            diffs: DifferenceStack::new(),
            pending: 0,
            armed: 0,
            followers: [None, None],
        }
    }

    /// Whether the stack has an innermost minimum to query or update.
    pub(super) fn armed(&self) -> bool {
        self.armed > 0
    }

    /// Whether one or more innermost ranges still need their first minimum.
    pub(super) fn has_pending(&self) -> bool {
        self.pending > 0
    }

    /// Open `count` nested ranges inside every existing range.
    ///
    /// They remain pending because an empty range has no minimum. Their first
    /// emission arms all of them at the same value, so opening is `O(1)` and
    /// allocates no boundary records.
    pub(super) fn open(&mut self, count: u64) {
        self.pending += count;
    }

    /// Move the running height by `delta` without changing any minimum.
    ///
    /// For an armed stack, `A` stays fixed, so `gap = h - A` changes by exactly
    /// `delta`. For an empty stack there is no anchor and nothing is stored.
    /// The fold reads `delta`, not the potentially wider accumulated gap.
    pub(super) fn fold_height(&mut self, delta: &BigInt) {
        if self.armed > 0 {
            accumulator::fold_signed(&mut self.gap, delta);
        }
    }

    /// Give every pending range its first minimum at the current height `v = h`.
    ///
    /// If no older range is armed, the emission establishes the first anchor:
    /// all pending minima equal `h`, `gap` becomes zero, and their internal
    /// boundaries form one zero run.
    ///
    /// Otherwise, the old `gap = h - A` is also `v - A`. Move it out, replace
    /// the gap with zero for the new anchor `v`, and pass the offset to
    /// [`finish_arming`](Self::finish_arming). The payload closure is lazy: an
    /// equal minimum needs no payload, so that case never constructs one.
    pub(super) fn arm_at_height<C>(
        &mut self,
        context: &mut C,
        payload: impl FnOnce(&mut C) -> P,
        on_die: impl FnMut(P, &mut C),
    ) {
        debug_assert!(self.pending > 0, "arming requires a pending range");
        let pending = core::mem::replace(&mut self.pending, 0);
        if self.armed == 0 {
            debug_assert!(
                self.followers.iter().all(Option::is_none),
                "followers attach after the first arming"
            );
            debug_assert!(
                self.deferred.is_none(),
                "an empty stack has no deferred distance"
            );
            self.armed = pending;
            let fresh = Accumulator::new();
            let old = core::mem::replace(&mut self.gap, fresh);
            drop(old);
            self.diffs.push_zeros(pending - 1);
            return;
        }
        let fresh = Accumulator::new();
        let offset = core::mem::replace(&mut self.gap, fresh);
        self.armed += pending;
        self.finish_arming(offset, pending, context, payload, on_die);
    }

    /// Give every pending range its first minimum at `v = h - below`.
    ///
    /// If this is the first armed range, `below` becomes `gap = h - v` and the
    /// pending ranges contribute only zero boundaries.
    ///
    /// With an existing anchor, move `below` into `gap`, then compute the new
    /// value relative to the old anchor from the displaced gap:
    ///
    /// `v - A = (h - A) - (h - v) = old_gap - below`.
    ///
    /// Only the displaced accumulator is folded; the new gap stays in place.
    pub(super) fn arm_below<C>(
        &mut self,
        below: Accumulator,
        context: &mut C,
        payload: impl FnOnce(&mut C) -> P,
        on_die: impl FnMut(P, &mut C),
    ) {
        debug_assert!(self.pending > 0, "arming requires a pending range");
        let pending = core::mem::replace(&mut self.pending, 0);
        if self.armed == 0 {
            debug_assert!(
                self.followers.iter().all(Option::is_none),
                "followers attach after the first arming"
            );
            debug_assert!(
                self.deferred.is_none(),
                "an empty stack has no deferred distance"
            );
            self.armed = pending;
            let old = core::mem::replace(&mut self.gap, below);
            drop(old);
            self.diffs.push_zeros(pending - 1);
            return;
        }
        // v - A = (h - A) - (h - v).
        let mut offset = core::mem::replace(&mut self.gap, below);
        offset.sub_accum(&self.gap);
        self.armed += pending;
        self.finish_arming(offset, pending, context, payload, on_die);
    }

    /// Join newly armed ranges at value `v` to an existing armed stack.
    ///
    /// On entry, `gap` already uses `v` as its anchor and `offset = v - A_old`.
    /// The method then:
    ///
    /// 1. adds `offset` to each follower, changing `A_old - X` into `v - X`;
    /// 2. combines `offset` with `deferred = A_old - m`, yielding `v - m`;
    /// 3. interprets the sign of `v - m`.
    ///
    /// If positive, the old minimum is lower: store one boundary and its
    /// payload, followed by `pending - 1` zero boundaries among the new ranges.
    /// If zero, all `pending` new boundaries are zero. If negative, `v` lowers
    /// the old minimum: retire its payload, propagate `m - v` outward, and add
    /// `pending` zero boundaries because the new ranges and old innermost range
    /// now share `v`.
    ///
    /// Combining `offset` and `deferred` uses `merge_into_wider`, so the
    /// narrower value is read once and the wider buffer survives.
    fn finish_arming<C>(
        &mut self,
        offset: Accumulator,
        pending: u64,
        context: &mut C,
        payload: impl FnOnce(&mut C) -> P,
        mut on_die: impl FnMut(P, &mut C),
    ) {
        for (follower, anchor_relative) in self.followers.iter_mut().zip(&mut self.anchor_relative)
        {
            if let Some(follower) = follower {
                follower.add_accum(&offset);
            }
            *anchor_relative = false;
        }
        let mut offset = offset;
        if let Some(deferred) = self.deferred.take() {
            let drained = offset.merge_into_wider(deferred);
            drop(drained);
        }
        match offset.sign() {
            Ordering::Greater => {
                let boundary = Self::compact_boundary(offset);
                self.diffs.push_diff(boundary, payload(context));
                self.diffs.push_zeros(pending - 1);
            }
            Ordering::Equal => {
                drop(offset);
                self.diffs.push_zeros(pending);
            }
            Ordering::Less => {
                on_die(payload(context), context);
                let mut drop = offset;
                drop.negate();
                self.propagate_drop(drop, context, on_die);
                self.diffs.push_zeros(pending);
            }
        }
    }

    /// Whether an emission at the current height would lower the minimum.
    ///
    /// The decision proceeds from cheapest to most expensive:
    ///
    /// 1. If `gap = h - A >= 0`, then `h >= A >= m`; the minimum cannot fall.
    /// 2. If `gap < 0` and no distance is deferred, then `A = m` and `h < m`.
    /// 3. Otherwise compare `A - h = -gap` with `A - m = deferred`. Only the
    ///    larger first distance places `h` below `m`.
    ///
    /// Accumulator sign reads are amortized constant time. Well-separated
    /// widths are ordered from leading digits; comparable widths are resolved
    /// once into a new anchor at `m`.
    pub(super) fn undercuts_here(&mut self) -> bool {
        if self.gap.sign() != Ordering::Less {
            return false;
        }
        if self.deferred.is_none() {
            return true;
        }
        match self.compare_below_anchor_to_minimum() {
            DeferredComparison::AboveMinimum => false,
            DeferredComparison::BelowMinimum => true,
            DeferredComparison::Reanchored => self.gap.sign() == Ordering::Less,
        }
    }

    /// Compare a value `v < A` with `m = A - deferred`.
    ///
    /// `gap` holds `v - A`, so its magnitude is the candidate drop `A - v`.
    /// The leading-digit checks first ask whether either positive distance is
    /// too wide for the other to overtake:
    ///
    /// - if `deferred` dominates, then `A - m > A - v`, hence `v > m`;
    /// - if `-gap` dominates, then `A - v > A - m`, hence `v < m`;
    /// - otherwise the widths are comparable, so resolve `deferred` and let the
    ///   caller compare `v - m` directly.
    ///
    /// The first two paths inspect only leading digits. The last path performs
    /// work proportional to values of comparable width and consumes the
    /// deferred state, so the same wide distance cannot be charged again.
    fn compare_below_anchor_to_minimum(&mut self) -> DeferredComparison {
        let gap_floor = self.gap.digit_count() - 1;
        let deferred = self
            .deferred
            .as_mut()
            .expect("this comparison requires a deferred distance");
        // Reading the sign also removes cancelling leading digits, so the
        // digit counts used by the constant-time comparisons are current.
        let deferred_sign = deferred.sign();
        debug_assert_eq!(
            deferred_sign,
            Ordering::Greater,
            "the deferred distance is strictly positive"
        );
        if deferred.sign_dominates_at(gap_floor).1 {
            return DeferredComparison::AboveMinimum;
        }
        let deferred_floor = deferred.digit_count() - 1;
        if self.gap.sign_dominates_at(deferred_floor).1 {
            return DeferredComparison::BelowMinimum;
        }
        self.resolve_deferred();
        DeferredComparison::Reanchored
    }

    /// Make the current height the innermost minimum after a confirmed undercut.
    ///
    /// [`undercuts_here`](Self::undercuts_here) has established `h < m`. This
    /// method performs the state change in two steps:
    ///
    /// 1. Move the negative `gap = h - A` out and replace it with zero. The new
    ///    anchor is therefore the emitted value `h`.
    /// 2. Negate the old gap to obtain `A - h`, then update followers,
    ///    deferred state, and outer boundaries through [`Self::apply_undercut`].
    ///
    /// Moving the old accumulator avoids copying its possibly wide contents.
    pub(super) fn undercut<C>(&mut self, context: &mut C, on_die: impl FnMut(P, &mut C)) {
        let fresh = Accumulator::new();
        let mut drop = core::mem::replace(&mut self.gap, fresh);
        drop.negate();
        self.apply_undercut(drop, context, on_die);
    }

    /// Complete an undercut after the anchor has moved to the emitted value `v`.
    ///
    /// The input `drop` initially equals `A - v` for the old anchor. Processing
    /// proceeds in dependency order:
    ///
    /// 1. Every follower also changes anchors from `A` to `v`, so subtract
    ///    `A - v` and clear its anchor-relative tag.
    /// 2. If a distance was deferred, subtract `A - m` from the drop. What
    ///    remains is `(A - v) - (A - m) = m - v`.
    /// 3. Apply that true decrease in the old minimum to enclosing ranges.
    ///
    /// The deferred accumulator is consumed once. Propagation consumes or
    /// shrinks boundaries without reconstructing an absolute minimum.
    fn apply_undercut<C>(
        &mut self,
        mut drop: Accumulator,
        context: &mut C,
        on_die: impl FnMut(P, &mut C),
    ) {
        for (follower, anchor_relative) in self.followers.iter_mut().zip(&mut self.anchor_relative)
        {
            if let Some(follower) = follower {
                debug_assert_eq!(
                    *anchor_relative,
                    self.deferred.is_some(),
                    "a deferred distance keeps every active follower tagged"
                );
                follower.sub_accum(&drop);
                *anchor_relative = false;
            }
        }
        if let Some(deferred) = self.deferred.take() {
            // (A - v) - (A - m) = m - v.
            drop.sub_accum(&deferred);
        }
        self.propagate_drop(drop, context, on_die);
    }

    /// Carry a decrease in the innermost minimum through enclosing ranges.
    ///
    /// On entry, `drop = old_inner_min - new_inner_min > 0`; `gap`,
    /// `deferred`, and followers already describe the new inner minimum. The
    /// stack is consumed from inner to outer:
    ///
    /// 1. A zero boundary means the outer range shared the old minimum, so it
    ///    necessarily shares the new one. Consume the whole zero run and keep
    ///    the same drop.
    /// 2. At a positive boundary `b`, [`Self::cross_boundary`] compares `drop`
    ///    with `b`.
    /// 3. If the drop crosses `b`, retire that boundary's payload, turn the
    ///    boundary into zero, and continue with `drop - b`.
    /// 4. If `b` stops the drop, restore the surviving boundary `b - drop` and
    ///    stop. Equality turns the boundary into zero and also stops.
    /// 5. Push all newly created zero boundaries as one merged run.
    ///
    /// A crossed boundary is permanently consumed, so its width is charged
    /// only once. A much wider surviving boundary is compared from leading
    /// digits and updated by reading the smaller drop; comparable values pay
    /// for one ordinary subtraction.
    fn propagate_drop<C>(
        &mut self,
        drop: Accumulator,
        context: &mut C,
        mut on_die: impl FnMut(P, &mut C),
    ) {
        let mut drop = drop;
        // Every crossed boundary now separates equal minima. Defer one merged
        // zero run until the walk stops.
        let mut zeros = 0u64;
        while let Some(entry) = self.diffs.pop() {
            match entry {
                Entry::ZeroRun(count) => zeros += count,
                Entry::Diff { boundary, payload } => {
                    match Self::cross_boundary(drop, boundary, payload) {
                        DropOutcome::BoundaryConsumed {
                            remaining_drop,
                            retired,
                        } => {
                            on_die(retired, context);
                            zeros += 1;
                            drop = remaining_drop;
                        }
                        DropOutcome::DropConsumed {
                            remaining_boundary,
                            payload,
                        } => {
                            self.diffs.push_diff(remaining_boundary, payload);
                            break;
                        }
                        DropOutcome::Equal { retired } => {
                            on_die(retired, context);
                            zeros += 1;
                            break;
                        }
                    }
                }
            }
        }
        self.diffs.push_zeros(zeros);
    }

    /// Route a positive drop through either representation of a boundary.
    ///
    /// The returned outcome owns every input: it either carries the remaining
    /// drop outward, restores the surviving boundary and payload, or retires
    /// the payload at equality. This makes it impossible for propagation to
    /// accidentally use a consumed boundary again.
    fn cross_boundary(drop: Accumulator, boundary: Boundary, payload: P) -> DropOutcome<P> {
        match boundary {
            Boundary::Word(word) => Self::cross_word_boundary(drop, word, payload),
            Boundary::Wide(boundary) => Self::cross_wide_boundary(drop, boundary, payload),
        }
    }

    /// Subtract a word-sized boundary from a positive drop and classify the sign.
    ///
    /// The word fold is amortized constant time. A negative result is negated
    /// to recover the positive surviving boundary `boundary - drop`.
    fn cross_word_boundary(mut drop: Accumulator, boundary: u64, payload: P) -> DropOutcome<P> {
        drop.sub_u64(boundary);
        match drop.sign() {
            Ordering::Greater => DropOutcome::BoundaryConsumed {
                remaining_drop: drop,
                retired: payload,
            },
            Ordering::Equal => DropOutcome::Equal { retired: payload },
            Ordering::Less => {
                drop.negate();
                DropOutcome::DropConsumed {
                    remaining_boundary: Self::compact_boundary(drop),
                    payload,
                }
            }
        }
    }

    /// Compare two positive wide values without traversing a much larger survivor.
    ///
    /// The method deliberately avoids first subtracting one arbitrary-precision
    /// value from the other. It proceeds as follows:
    ///
    /// 1. If `drop` is at least two base-2^32 digits wider, ask whether its
    ///    leading digits prove that it dominates `boundary`. When they do,
    ///    subtract the smaller boundary and continue outward.
    /// 2. Apply the symmetric test when `boundary` is wider. When it succeeds,
    ///    subtract the smaller drop and restore the boundary.
    /// 3. Otherwise subtract once and use the sign to classify the result.
    ///
    /// Two digits are the first separation at which Accumulator's redundant
    /// representation can certify domination. A failed certificate is safe:
    /// the values are then close enough in stored width that one subtraction is
    /// proportional to both. In every path, arithmetic reads the value that is
    /// consumed or a value comparable in width to its survivor.
    fn cross_wide_boundary(
        mut drop: Accumulator,
        mut boundary: Accumulator,
        payload: P,
    ) -> DropOutcome<P> {
        if drop.digit_count() >= boundary.digit_count() + 2 {
            match drop.sign_dominates_at(boundary.digit_count() - 1) {
                (Ordering::Greater, true) => {
                    drop.sub_accum(&boundary);
                    return DropOutcome::BoundaryConsumed {
                        remaining_drop: drop,
                        retired: payload,
                    };
                }
                (_, true) => unreachable!("the drop is strictly positive"),
                (_, false) => {}
            }
        }
        if boundary.digit_count() >= drop.digit_count() + 2 {
            match boundary.sign_dominates_at(drop.digit_count() - 1) {
                (Ordering::Greater, true) => {
                    boundary.sub_accum(&drop);
                    return DropOutcome::DropConsumed {
                        remaining_boundary: Self::compact_boundary(boundary),
                        payload,
                    };
                }
                (_, true) => unreachable!("stored boundaries are strictly positive"),
                (_, false) => {}
            }
        }

        boundary.sub_accum(&drop);
        match boundary.sign() {
            Ordering::Greater => DropOutcome::DropConsumed {
                remaining_boundary: Self::compact_boundary(boundary),
                payload,
            },
            Ordering::Equal => DropOutcome::Equal { retired: payload },
            Ordering::Less => {
                boundary.negate();
                DropOutcome::BoundaryConsumed {
                    remaining_drop: boundary,
                    retired: payload,
                }
            }
        }
    }

    /// Choose the smallest representation for a positive boundary.
    ///
    /// At base `2^32`, any `u64` occupies at most two digits. A wider
    /// accumulator therefore cannot fit and is retained without normalization.
    /// For at most two digits, materialize once and store the value inline when
    /// conversion succeeds. This keeps common boundaries small without
    /// scanning every wide boundary merely to reject it.
    fn compact_boundary(difference: Accumulator) -> Boundary {
        if difference.digit_count() <= 2 {
            let (sign, magnitude) = accumulator::value(&difference);
            debug_assert_eq!(sign, Ordering::Greater, "boundaries are strictly positive");
            if let Ok(word) = u64::try_from(&magnitude) {
                return Boundary::Word(word);
            }
        }
        Boundary::Wide(difference)
    }

    /// Close the innermost armed range and expose its parent.
    ///
    /// Before the close, let `m` be the inner minimum. There are three cases:
    ///
    /// 1. If this is the final armed range, clear `gap` and `deferred`; no
    ///    minimum remains.
    /// 2. If the next boundary is zero, the parent also has minimum `m`.
    ///    Consume one zero boundary and leave the anchor unchanged.
    /// 3. If the boundary is `b > 0`, the parent minimum is `m - b`. Keep the
    ///    anchor fixed, add `b` to `deferred`, and return the payload belonging
    ///    to the parent minimum.
    ///
    /// This operation never reconstructs either absolute minimum. A zero close
    /// is constant time; a positive boundary is moved and, when necessary,
    /// merged at the cost of the narrower accumulator.
    pub(super) fn close(&mut self) -> Close<P> {
        debug_assert_eq!(
            self.pending, 0,
            "every closing range must first see an emission"
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
                "followers are removed before their ranges close"
            );
            self.anchor_relative = [false; FOLLOWER_SLOTS];
            if let Some(deferred) = self.deferred.take() {
                drop(deferred);
            }
            let gap = core::mem::take(&mut self.gap);
            drop(gap);
            return Close::Retired;
        }
        match self.diffs.pop().expect("armed > 1 has a difference record") {
            Entry::ZeroRun(count) => {
                if count > 1 {
                    self.diffs.push_zeros(count - 1);
                }
                Close::Equal
            }
            Entry::Diff { boundary, payload } => {
                // The parent minimum is lower by this boundary. Keep the
                // anchor fixed and defer that change in Λ.
                self.defer_boundary(boundary);
                Close::Lower(payload)
            }
        }
    }

    /// Add a closed range's positive boundary to the deferred anchor shift.
    ///
    /// Suppose the old innermost minimum is `m` and the exposed parent's
    /// minimum is `m - b`. The anchor `A` does not move, so the new deferred
    /// distance is `(A - m) + b`.
    ///
    /// On the first such close, each follower still contains `A - X` even
    /// though its abstract value is now `(m - b) - X`; its tag records that the
    /// deferred distance must later be subtracted. Further closes leave the tag
    /// unchanged and add only their boundary. Word boundaries update in
    /// amortized constant time; wide boundaries use `merge_into_wider`, which
    /// preserves the wider buffer and reads the narrower value once.
    fn defer_boundary(&mut self, boundary: Boundary) {
        match self.deferred.take() {
            None => {
                for (follower, anchor_relative) in
                    self.followers.iter().zip(&mut self.anchor_relative)
                {
                    debug_assert!(!*anchor_relative, "a tag requires a deferred distance");
                    *anchor_relative = follower.is_some();
                }
                let deferred = match boundary {
                    Boundary::Word(word) => {
                        let mut deferred = Accumulator::new();
                        deferred.add_u64(word);
                        deferred
                    }
                    Boundary::Wide(deferred) => deferred,
                };
                self.deferred = Some(deferred);
            }
            Some(mut deferred) => {
                debug_assert!(
                    self.followers.iter().zip(self.anchor_relative).all(
                        |(follower, anchor_relative)| { follower.is_none() || anchor_relative }
                    ),
                    "a deferred distance keeps every active follower tagged"
                );
                match boundary {
                    Boundary::Word(word) => deferred.add_u64(word),
                    Boundary::Wide(wide) => {
                        let drained = deferred.merge_into_wider(wide);
                        drop(drained);
                    }
                }
                self.deferred = Some(deferred);
            }
        }
    }

    /// Move the anchor from `A` to the true minimum `m`.
    ///
    /// Before resolution:
    ///
    /// - `gap = h - A`;
    /// - `deferred = A - m`;
    /// - each tagged follower contains `A - X`.
    ///
    /// Adding `deferred` makes the gap `h - m`. Subtracting it from tagged
    /// followers makes them `m - X`. The deferred value is then consumed and
    /// every tag clears. `merge_into_wider` ensures the gap/deferred merge reads
    /// only the narrower buffer; the fixed number of followers adds only a
    /// constant multiple of the deferred value's width.
    pub(super) fn resolve_deferred(&mut self) {
        let Some(deferred) = self.deferred.take() else {
            return;
        };
        for (follower, anchor_relative) in self.followers.iter_mut().zip(&mut self.anchor_relative)
        {
            if *anchor_relative {
                follower
                    .as_mut()
                    .expect("a set tag rides an active follower")
                    .sub_accum(&deferred);
                *anchor_relative = false;
            }
        }
        let drained = self.gap.merge_into_wider(deferred);
        drop(drained);
    }

    /// Whether a close has left the anchor above the true minimum.
    pub(super) fn deferred_live(&self) -> bool {
        self.deferred.is_some()
    }
}

/// Convenience operations when boundaries carry no client state.
///
/// They translate skyline emissions and comparisons into the generic arming
/// and undercut transitions above, using `()` as the payload and retirement
/// context.
impl RangeMinima<()> {
    /// Record an emission at the current height `v = h`.
    ///
    /// Pending ranges receive their first minimum through `arm_at_height`.
    /// Otherwise the emission changes state only if `h` is below the current
    /// minimum, in which case `undercut` lowers every enclosing minimum it
    /// reaches.
    pub(super) fn emit_here(&mut self) {
        if self.pending > 0 {
            self.arm_at_height(&mut (), |_| (), |(), _| ());
            return;
        }
        if self.undercuts_here() {
            self.undercut(&mut (), |(), _| ());
        }
    }

    /// Record an emission at `v = h + offset`.
    ///
    /// The method follows the state machine in this order:
    ///
    /// 1. A zero offset is the `emit_here` case.
    /// 2. If ranges are pending, convert the offset to `h - v = -offset` and
    ///    arm them through `arm_below`.
    /// 3. Otherwise compare `v` with the minimum. Since
    ///    `v - A = gap + offset`, a word-sized offset may be decided from the
    ///    wide gap's leading digits without a fold.
    /// 4. If that shortcut cannot decide, temporarily add `offset` to `gap`.
    ///    Restore it before returning unless `v` is a new minimum.
    /// 5. For an undercut, negate the folded `v - A` to obtain the drop, apply
    ///    it, and set the new gap to `h - v = -offset`.
    ///
    /// The shortcut prevents a stream of small offsets from repeatedly walking
    /// a wide gap. The general path reads the offset once; a comparable
    /// deferred distance is consumed rather than repeatedly compared.
    pub(super) fn emit_offset(&mut self, offset: &BigInt) {
        if offset.sign() == Sign::NoSign {
            self.emit_here();
            return;
        }
        if self.pending > 0 {
            // below = h − v = −offset.
            let mut below = Accumulator::new();
            accumulator::subtract_signed(&mut below, offset);
            self.arm_below(below, &mut (), |_| (), |(), _| ());
            return;
        }
        if self.emit_offset_when_gap_dominates(offset) {
            return;
        }

        accumulator::fold_signed(&mut self.gap, offset);
        if self.gap.sign() != Ordering::Less {
            accumulator::subtract_signed(&mut self.gap, offset);
            return;
        }
        if self.deferred.is_some() {
            let undercuts = match self.compare_below_anchor_to_minimum() {
                DeferredComparison::AboveMinimum => false,
                DeferredComparison::BelowMinimum => true,
                DeferredComparison::Reanchored => self.gap.sign() == Ordering::Less,
            };
            if !undercuts {
                accumulator::subtract_signed(&mut self.gap, offset);
                return;
            }
        }

        let mut drop = core::mem::take(&mut self.gap);
        drop.negate();
        self.apply_undercut(drop, &mut (), |(), _| ());
        let mut gap = Accumulator::new();
        accumulator::subtract_signed(&mut gap, offset);
        self.gap = gap;
    }

    /// Try to decide a word-sized offset from a much wider gap.
    ///
    /// This shortcut applies only when the anchor already equals the minimum.
    /// If the gap's leading digits prove that a word cannot change its sign,
    /// then that sign also orders `v = h + offset` against the minimum:
    ///
    /// - a positive gap means `v` remains above the minimum and no state moves;
    /// - a negative gap means `v` is an undercut. The drop is
    ///   `-gap - offset`, and the new height gap is `-offset`.
    ///
    /// Returns `false` when the offset is wide, a distance is deferred, or the
    /// leading digits do not prove the ordering. Those cases use the ordinary
    /// folded comparison.
    fn emit_offset_when_gap_dominates(&mut self, offset: &BigInt) -> bool {
        if self.deferred.is_some() || u64::try_from(offset.magnitude()).is_err() {
            return false;
        }
        let (sign, decided) = self.gap.sign_dominates_word();
        if !decided {
            web_traffic::record(web_traffic::Decision::Undecided);
            return false;
        }
        match sign {
            Ordering::Greater => {
                web_traffic::record(web_traffic::Decision::DominatedAbove);
            }
            Ordering::Less => {
                web_traffic::record(web_traffic::Decision::DominatedUndercut);
                // Since gap dominates offset, -gap-offset is positive.
                let mut drop = core::mem::take(&mut self.gap);
                drop.negate();
                accumulator::subtract_signed(&mut drop, offset);
                self.apply_undercut(drop, &mut (), |(), _| ());

                let mut gap = Accumulator::new();
                accumulator::subtract_signed(&mut gap, offset);
                self.gap = gap;
            }
            Ordering::Equal => unreachable!("a decisive gap is nonzero"),
        }
        true
    }

    /// Record the first emission of pending ranges at `v = h - below`.
    ///
    /// This is the ownership-taking form used when another calculation already
    /// produced `below` as an accumulator. Passing it to `arm_below` moves that
    /// allocation into the stack as the new gap instead of materializing or
    /// copying it.
    pub(super) fn emit_below_accum(&mut self, below: Accumulator) {
        debug_assert!(self.pending > 0, "a raise arms its own node's range");
        self.arm_below(below, &mut (), |_| (), |(), _| ());
    }

    /// Compare `v = h + above` with the innermost minimum `m`.
    ///
    /// `Less` means the candidate would lower the minimum. A word-sized
    /// `above` is first compared from the wide gap's leading digits when no
    /// distance is deferred. Otherwise:
    ///
    /// 1. add `above` to `gap`, obtaining `v - A`;
    /// 2. if `A > m`, compare a negative result with the deferred distance;
    /// 3. subtract `above` again, restoring the original height gap.
    ///
    /// A comparable deferred distance may be resolved into the anchor during
    /// step 2. That changes representation but not the tracked minimum.
    pub(super) fn compare_above(&mut self, above: &BigInt) -> Ordering {
        debug_assert!(self.armed > 0, "a raise compares against an armed range");
        if self.deferred.is_none() && u64::try_from(above.magnitude()).is_ok() {
            let (sign, decided) = self.gap.sign_dominates_word();
            if decided {
                return sign;
            }
        }
        accumulator::fold_signed(&mut self.gap, above);
        let sign = match (self.gap.sign(), self.deferred.is_some()) {
            (sign, false) => sign,
            (Ordering::Equal | Ordering::Greater, true) => Ordering::Greater,
            (Ordering::Less, true) => match self.compare_below_anchor_to_minimum() {
                DeferredComparison::AboveMinimum => Ordering::Greater,
                DeferredComparison::BelowMinimum => Ordering::Less,
                DeferredComparison::Reanchored => self.gap.sign(),
            },
        };
        accumulator::subtract_signed(&mut self.gap, above);
        sign
    }

    /// Compare `h + above` with an anchor-relative candidate minimum.
    ///
    /// The candidate is `A + arm_offset`, so their difference is
    ///
    /// `(h + above) - (A + arm_offset) = gap + above - arm_offset`.
    ///
    /// The method folds those two operands into `gap`, reads the sign, then
    /// reverses both folds. Because the anchor appears on both sides, any
    /// deferred distance to the true minimum cancels and is never read.
    pub(super) fn compare_above_vs(
        &mut self,
        above: &BigInt,
        arm_offset: &Accumulator,
    ) -> Ordering {
        debug_assert!(self.armed > 0, "a raise compares against an armed range");
        self.gap.sub_accum(arm_offset);
        accumulator::fold_signed(&mut self.gap, above);
        let sign = self.gap.sign();
        accumulator::subtract_signed(&mut self.gap, above);
        self.gap.add_accum(arm_offset);
        sign
    }

    /// Give pending ranges their first minimum at `v = A + arm_offset`.
    ///
    /// Subtracting `arm_offset` changes the height gap from `h - A` to
    /// `h - v`. The same owned accumulator then passes to `finish_arming`,
    /// which combines it with the deferred `A - m` to obtain the boundary
    /// `v - m`. One value therefore serves both calculations without a clone.
    pub(super) fn arm_relative(&mut self, arm_offset: Accumulator) {
        debug_assert!(self.pending > 0, "a raise arms its own node's range");
        debug_assert!(self.armed > 0, "a relative arming needs an armed anchor");
        let pending = core::mem::replace(&mut self.pending, 0);
        self.gap.sub_accum(&arm_offset);
        self.armed += pending;
        self.finish_arming(arm_offset, pending, &mut (), |_| (), |(), _| ());
    }

    /// Attach one client value to changes in the innermost minimum.
    ///
    /// Abstractly the follower is `m - X`. When `A = m`, the supplied
    /// accumulator already has that form. When a distance is deferred, the
    /// supplied value is `A - X`; `anchor_relative[slot]` records that
    /// `deferred = A - m` must be subtracted later. Storing the one-bit debt
    /// avoids immediately traversing either wide accumulator.
    pub(super) fn follower_set(&mut self, slot: usize, follower: Accumulator) {
        debug_assert!(self.followers[slot].is_none(), "one follower per slot");
        debug_assert!(self.armed > 0, "a follower needs an armed anchor");
        self.anchor_relative[slot] = self.deferred.is_some();
        self.followers[slot] = Some(follower);
    }

    /// Detach a follower and return the accumulator exactly as stored.
    ///
    /// An untagged result is `m - X`. A tagged result is `A - X`; its consumer
    /// must either combine it with another anchor-relative value, where the
    /// deferred term cancels, or call `resolve_deferred` before taking it. The
    /// tag clears because the stack no longer owns the returned value.
    pub(super) fn follower_take(&mut self, slot: usize) -> Accumulator {
        self.anchor_relative[slot] = false;
        self.followers[slot].take().expect("the follower is active")
    }

    /// Change an anchor-relative follower into a height-relative value.
    ///
    /// The stored follower is `A - X` and `gap = h - A`. Adding them gives
    /// `h - X`. If the follower is tagged, this is exactly the useful shortcut:
    /// the abstract forms contain opposite deferred terms,
    ///
    /// `(A - X - deferred) + (h - A + deferred) = h - X`.
    ///
    /// The terms cancel symbolically, so this conversion never reads the
    /// deferred accumulator.
    pub(super) fn bridge_add_gap(&mut self, delta: &mut Accumulator) {
        delta.add_accum(&self.gap);
    }

    /// Change a height-relative value into one relative to the true minimum.
    ///
    /// On entry, `delta` is relative to `h`. The anchor must already be
    /// resolved, making `gap = h - m`. Subtracting the gap therefore replaces
    /// the height reference with `m`. The assertion prevents an accidental use
    /// of `h - A`, which would leave the result off by the deferred distance.
    pub(super) fn bridge_sub_gap(&mut self, delta: &mut Accumulator) {
        debug_assert!(
            self.deferred.is_none(),
            "the height-to-watermark switch resolves the deferred distance first"
        );
        delta.sub_accum(&self.gap);
    }
}

#[cfg(test)]
mod tests;
