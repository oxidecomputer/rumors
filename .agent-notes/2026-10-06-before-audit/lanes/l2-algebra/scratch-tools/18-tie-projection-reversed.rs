//! Restricts a version to one party's ownership.
//!
//! A [`Version`] assigns an event height to every region of party space. A
//! [`Party`] identifies the regions one participant owns. Projecting `v` onto
//! `p` keeps `v`'s height where `p` owns the region and replaces every other
//! height with zero.
//!
//! [`OwnVersion`] represents that result lazily. This module provides its two
//! implementations:
//!
//! - [`VersionWriter::project`] materializes the result when a caller asks for
//!   [`OwnVersion::to_version`].
//! - [`Comparison`] compares materialized or projected versions directly,
//!   without constructing either projection.
//!
//! # Walking the common partition
//!
//! A version divides party space wherever its height changes; a party divides
//! it wherever ownership changes. The algorithms walk those divisions
//! together. The next boundary is whichever current region ends first. After
//! advancing every input that ends there, all inputs are again constant until
//! the following boundary. Each such interval therefore has one height and one
//! ownership answer per operand.
//!
//! Materialization emits the source version's changes while the party owns the
//! interval and emits zero elsewhere. When an entire version subtree lies in
//! an unowned region, the walk skips it as a unit: every height in that subtree
//! has the same projected value, zero. [`VersionWriter`] then performs the
//! ordinary sibling collapses needed to leave one canonical version.
//!
//! # Comparing lazy projections
//!
//! Comparison needs only the sign of the two projected heights' difference on
//! each interval. There are four ownership cases:
//!
//! - both owned: compare `h_a - h_b`;
//! - only `a` owned: compare `h_a` with zero;
//! - only `b` owned: compare zero with `h_b`;
//! - neither owned: the projected heights are equal.
//!
//! One [`Accumulator`] always tracks `h_a - h_b`. An absolute-height
//! accumulator for `a` is needed only when `b` is projected, and the converse
//! holds for `b`; without the other mask, the single-owner case cannot occur.
//! Thus each version change is decoded once and updates only the quantities a
//! later ownership case can inspect.
//!
//! Full ordering stops after finding evidence against both causal directions.
//! A directional operator stops as soon as its requested direction is
//! impossible, and equality stops at the first unequal interval.
//!
//! # Cost
//!
//! Lazy comparison visits each input boundary a constant number of times, so
//! its work is linear in the combined inputs. Its transient state is the input
//! cursors and at most three accumulators; it never allocates a projected
//! version. Materialization is linear in the inputs plus the required output.
//! That distinction matters because interleaving ownership and height
//! boundaries can make the canonical result as large as the product of the
//! operands' sizes.
//!

use core::cmp::Ordering;
use core::ops::ControlFlow;

use num_bigint::{BigInt, BigUint, Sign};
use suanpan::Accumulator;

use crate::accumulator::BigIntAccumulator as _;
use crate::party::io::PartyRegionReader;
use crate::{OwnVersion, Party};

use super::order::OrderState;
use super::overlay::{advance, advance_set, Crossed, CursorSet, OpenedPair, Side};
use crate::version::io::regions::{RegionReader, VersionRegionReader};
use crate::version::io::writer::VersionWriter;
use crate::Version;

/// One operand of a comparison, optionally restricted to a party.
pub trait Operand {
    /// The stored version and the party whose region remains visible.
    fn comparison_parts(&self) -> (&Version, Option<&Party>);
}

impl Operand for Version {
    fn comparison_parts(&self) -> (&Version, Option<&Party>) {
        (self, None)
    }
}

impl Operand for OwnVersion<'_> {
    fn comparison_parts(&self) -> (&Version, Option<&Party>) {
        (self.version, Some(self.party))
    }
}

impl VersionWriter {
    /// Materialize a lazy projection as one canonical Version stream.
    pub fn project(view: &OwnVersion<'_>) -> Version {
        let source = view.version;
        let party = view.party;
        let (mut version, first) = VersionRegionReader::open(source);
        let mut ownership = PartyRegionReader::new(party);
        let mut height = Accumulator::new();
        height.add_shifted_limbs(0, first.iter_u64_digits());
        let mut owned = ownership.owned();

        // Most projections fit within the combined input size. Larger outputs
        // grow normally; this reservation only avoids reallocating the common
        // case.
        let capacity = source.stored_len() + party.stored_len();
        let mut out = VersionWriter::with_capacity(capacity);
        let opening = if owned { first } else { BigUint::ZERO };
        out.height(version.depth().max(ownership.depth()), &opening);

        while !(version.done() && ownership.done()) {
            // Inside an unowned Party region, every Version height projects to
            // zero. Consume a nested Version subtree in bulk and emit the one
            // zero leaf to which all of its regions collapse.
            if !owned {
                loop {
                    let flip = version.peek_flip();
                    if flip <= ownership.depth() {
                        break;
                    }
                    let (stepped_flip, step) = version.step();
                    debug_assert_eq!(stepped_flip, flip, "the peeked flip is the next step");
                    Side::A.fold(&mut height, &step);
                    version.skip_deeper(flip, &mut height);
                    out.unchanged(flip);
                }
                if version.done() && ownership.done() {
                    break;
                }
            }

            // Move to the next boundary in either input. Version boundaries
            // update the absolute height; Party boundaries only change whether
            // that height belongs in the output.
            let (version_step, _) = advance(&mut version, &mut ownership, |crossing| {
                if let Crossed::A(step) = crossing {
                    Side::A.fold(&mut height, step);
                }
            });
            let now_owned = ownership.owned();
            let delta = match (owned, now_owned) {
                // Within an owned run, preserve the Version's own change. A
                // Party-only boundary leaves the output unchanged.
                (true, true) => version_step.unwrap_or_else(|| BigInt::from(0u8)),
                (false, false) => BigInt::from(0u8),
                // Entering ownership raises the output from zero to the current
                // absolute height.
                (false, true) => {
                    height.cmp_zero();
                    let height = height.to_bigint();
                    debug_assert!(height.sign() != Sign::Minus, "heights are nonnegative");
                    height
                }
                // Leaving ownership drops from the height immediately before
                // this boundary. If the Version stepped here too, remove that
                // just-applied change to recover the preceding height.
                (true, false) => {
                    height.cmp_zero();
                    let now = height.to_bigint();
                    debug_assert!(now.sign() != Sign::Minus, "heights are nonnegative");
                    let before = match version_step {
                        Some(step) => now - step,
                        None => now,
                    };
                    debug_assert!(before.sign() != Sign::Minus, "heights are nonnegative");
                    -before
                }
            };
            owned = now_owned;
            out.change(version.depth().max(ownership.depth()), &delta);
        }

        out.finish()
    }
}

/// The cursor set and integrators of one masked comparison.
pub struct Comparison<'a> {
    /// The left version's current region.
    a: VersionRegionReader<'a>,
    /// The left side's ownership regions; `None` means the side is unrestricted
    /// (owned everywhere).
    a_party: Option<PartyRegionReader<'a>>,
    /// The right version's current region.
    b: VersionRegionReader<'a>,
    /// The right side's ownership regions, as for `a_party`.
    b_party: Option<PartyRegionReader<'a>>,
    /// `D = h_a − h_b`, the both-owned intervals' sign source.
    diff: Accumulator,
    /// `h_a`, maintained only when `b` is masked (the only case that reads it:
    /// `a` owned alone compares `h_a` against zero).
    height_a: Option<Accumulator>,
    /// `h_b`, maintained only when `a` is masked, dually.
    height_b: Option<Accumulator>,
}

/// Slot numbers used by [`CursorSet`].
impl Comparison<'_> {
    /// The left version's slot.
    const A: usize = 0;
    /// The left mask's slot.
    const A_PARTY: usize = 1;
    /// The right version's slot.
    const B: usize = 2;
    /// The right mask's slot.
    const B_PARTY: usize = 3;
}

impl<'a> Comparison<'a> {
    /// Return the causal order after restricting either version to a party.
    pub fn order(a: &'a impl Operand, b: &'a impl Operand) -> Option<Ordering> {
        let ((a, a_party), (b, b_party)) = (a.comparison_parts(), b.comparison_parts());
        Self::open(a, a_party, b, b_party).resolve(OrderState::exit_order, OrderState::relation)
    }

    /// Test equality after restricting either version to a party.
    pub fn equal(a: &'a impl Operand, b: &'a impl Operand) -> bool {
        let ((a, a_party), (b, b_party)) = (a.comparison_parts(), b.comparison_parts());
        Self::open(a, a_party, b, b_party).resolve(OrderState::exit_equality, |state| {
            debug_assert!(
                state.is_equal(),
                "the equality exit breaks on refutation, so exhaustion is equality"
            );
            true
        })
    }

    /// Test `a <= b` after restricting either version to a party.
    pub fn le(a: &'a impl Operand, b: &'a impl Operand) -> bool {
        let ((a, a_party), (b, b_party)) = (a.comparison_parts(), b.comparison_parts());
        Self::open(a, a_party, b, b_party).resolve(OrderState::exit_le, OrderState::allows_le)
    }

    /// Test `a < b` after restricting either version to a party.
    pub fn lt(a: &'a impl Operand, b: &'a impl Operand) -> bool {
        let ((a, a_party), (b, b_party)) = (a.comparison_parts(), b.comparison_parts());
        Self::open(a, a_party, b, b_party).resolve(OrderState::exit_le, OrderState::is_lt)
    }

    /// Open every operand stream at its first leaf or region and seed the
    /// integrators with the two absolute first heights.
    fn open(
        a_bits: &'a Version,
        a_party: Option<&'a Party>,
        b_bits: &'a Version,
        b_party: Option<&'a Party>,
    ) -> Comparison<'a> {
        let OpenedPair {
            a,
            b,
            diff,
            a_first,
            b_first,
        } = OpenedPair::open(a_bits, b_bits);
        // Each height integrator exists only if the *other* side is masked: no
        // ownership case reads it otherwise, so feeding it would be pure waste.
        let height_a = b_party.map(|_| {
            let mut height_a = Accumulator::new();
            height_a.add_shifted_limbs(0, a_first.iter_u64_digits());
            height_a
        });
        let height_b = a_party.map(|_| {
            let mut height_b = Accumulator::new();
            height_b.add_shifted_limbs(0, b_first.iter_u64_digits());
            height_b
        });
        Comparison {
            a,
            a_party: a_party.map(PartyRegionReader::new),
            b,
            b_party: b_party.map(PartyRegionReader::new),
            diff,
            height_a,
            height_b,
        }
    }

    /// Run the merge over the projected skylines, generic over the question
    /// asked of the surviving [`OrderState`] (here `a′ <= b′`, `b′ <= a′`: the
    /// projected heights).
    ///
    /// After each interval's sign fold, `exit` sees the surviving directions
    /// and may declare the question decided — the `Break` payload carries the
    /// verdict, so the earliest stop and its answer are one value (an early
    /// exit leaves the direction the question does not need wherever the folded
    /// prefix left it, which is why directions are never handed back early). At
    /// exhaustion `finish` maps the fully-swept directions.
    fn resolve<V>(
        mut self,
        exit: impl Fn(OrderState) -> ControlFlow<V>,
        finish: impl FnOnce(OrderState) -> V,
    ) -> V {
        let mut directions = OrderState::new();
        loop {
            // Ownership selects the accumulator that represents the projected
            // difference on this constant region.
            let owned_a = self.a_party.as_ref().is_none_or(PartyRegionReader::owned);
            let owned_b = self.b_party.as_ref().is_none_or(PartyRegionReader::owned);
            let sign = match (owned_a, owned_b) {
                (true, true) => self.diff.cmp_zero(),
                (true, false) => {
                    // `h′_b = 0`: the interval's sign is `sign(h_a)`, the
                    // trichotomy's zero-check on the unmasked side.
                    let height_sign = self
                        .height_a
                        .as_mut()
                        .expect("a restricted `b` maintains h_a")
                        .cmp_zero();
                    debug_assert_ne!(height_sign, Ordering::Less, "heights are nonnegative");
                    height_sign
                }
                (false, true) => {
                    let height_sign = self
                        .height_b
                        .as_mut()
                        .expect("a restricted `a` maintains h_b")
                        .cmp_zero();
                    debug_assert_ne!(height_sign, Ordering::Less, "heights are nonnegative");
                    height_sign.reverse()
                }
                (false, false) => Ordering::Equal, // 0 vs 0
            };
            directions.fold(sign);
            if let ControlFlow::Break(verdict) = exit(directions) {
                return verdict;
            }
            if self.done() {
                return finish(directions);
            }
            self.advance();
        }
    }

    /// Whether every operand stream is at its final leaf or region.
    ///
    /// Canonical streams all tile the unit interval, so they exhaust together,
    /// exactly as in the pair sweep.
    fn done(&self) -> bool {
        self.a.done()
            && self.b.done()
            && self.a_party.as_ref().is_none_or(RegionReader::done)
            && self.b_party.as_ref().is_none_or(RegionReader::done)
    }

    /// The deepest current region among the other cursors.
    ///
    /// A boundary deeper than this belongs only to `slot`, so it can be
    /// consumed without advancing another cursor.
    fn others_deepest(&self, slot: usize) -> u64 {
        self.priority()
            .filter(|&other| other != slot)
            .map(|other| self.depth(other))
            .max()
            .expect("the walk has more than one cursor slot")
    }

    /// Consume runs of version boundaries hidden by an unowned region.
    ///
    /// If a mask does not own the current interval, event boundaries deeper
    /// than every other cursor stay inside that interval and cannot affect the
    /// result. Consume those boundaries together and fold their net height
    /// change once.
    ///
    /// Every iteration consumes a boundary, so the shortcut remains linear in
    /// the input it crosses.
    ///
    /// A block can consume a side to exhaustion, so the caller re-checks
    /// [`done`](Self::done) before applying the advance law.
    ///
    fn block_skip(&mut self) {
        loop {
            let a_bound = self.others_deepest(Self::A);
            if self.a_party.as_ref().is_some_and(|party| !party.owned())
                && self.a.peek_flip() > a_bound
            {
                let mut net = Accumulator::new();
                self.a.skip_deeper(a_bound, &mut net);
                self.diff += &net;
                if let Some(height_a) = &mut self.height_a {
                    *height_a += &net;
                }
                continue;
            }
            let b_bound = self.others_deepest(Self::B);
            if self.b_party.as_ref().is_some_and(|party| !party.owned())
                && self.b.peek_flip() > b_bound
            {
                let mut net = Accumulator::new();
                self.b.skip_deeper(b_bound, &mut net);
                self.diff -= &net;
                if let Some(height_b) = &mut self.height_b {
                    *height_b += &net;
                }
                continue;
            }
            return;
        }
    }

    /// Advance to the next relevant boundary after skipping hidden runs.
    fn advance(&mut self) {
        self.block_skip();
        if self.done() {
            // A block consumed the last unowned run; the final interval's sign
            // folds in the caller's next round.
            return;
        }
        advance_set(self);
    }
}

/// Makes the four cursors advance under the shared boundary rule.
///
/// Ties use the stable order `[B_PARTY, B, A_PARTY, A]`. Every fold is a
/// commutative sum, so this order cannot affect the verdict.
impl CursorSet for Comparison<'_> {
    fn priority(&self) -> impl Iterator<Item = usize> + Clone + 'static {
        [Self::A, Self::A_PARTY, Self::B, Self::B_PARTY].into_iter()
    }

    /// An absent mask reads zero: one all-owned region over the whole
    /// interval, which never steps.
    fn depth(&self, slot: usize) -> u64 {
        match slot {
            Self::A => self.a.depth(),
            Self::A_PARTY => self.a_party.as_ref().map_or(0, RegionReader::depth),
            Self::B => self.b.depth(),
            Self::B_PARTY => self.b_party.as_ref().map_or(0, RegionReader::depth),
            _ => unreachable!("four cursor slots"),
        }
    }

    /// A Version slot folds its height change into the integrators that watch
    /// that side; a Party slot changes ownership but no height.
    ///
    /// The watchers are `diff` always, plus the side's height integrator when
    /// present. A mask crossing carries no delta — ownership is per-region
    /// state read between boundaries.
    fn step(&mut self, slot: usize) -> u64 {
        match slot {
            Self::A => {
                let (flip, step) = self.a.step();
                Side::A.fold(&mut self.diff, &step);
                if let Some(height_a) = &mut self.height_a {
                    // A height integrator accumulates its own side plainly:
                    // the side orientation belongs to `D` alone.
                    height_a.add_bigint(&step);
                }
                flip
            }
            Self::A_PARTY => {
                self.a_party
                    .as_mut()
                    .expect("an absent mask reads depth zero and never steps")
                    .step()
                    .0
            }
            Self::B => {
                let (flip, step) = self.b.step();
                Side::B.fold(&mut self.diff, &step);
                if let Some(height_b) = &mut self.height_b {
                    // A height integrator accumulates its own side plainly:
                    // the side orientation belongs to `D` alone.
                    height_b.add_bigint(&step);
                }
                flip
            }
            Self::B_PARTY => {
                self.b_party
                    .as_mut()
                    .expect("an absent mask reads depth zero and never steps")
                    .step()
                    .0
            }
            _ => unreachable!("four cursor slots"),
        }
    }
}

#[cfg(test)]
mod tests;
