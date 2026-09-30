//! Streams the pointwise minimum or maximum of two versions into canonical
//! output.
//!
//! Join takes the greater height at every point; meet takes the lesser. Two
//! leaf cursors walk the inputs from left to right. Their current leaves cover
//! the same point and are dyadic regions, so one contains the other. The
//! smaller region is therefore the next region on which both heights are
//! constant. After emitting it, the cursor whose region ends there advances.
//!
//! The walk maintains the signed difference `D = height_a - height_b` in an
//! [`Accumulator`]. The sign of `D` selects the input that supplies the next
//! output height. Equal heights keep the previous selection. A
//! [`VersionWriter`] reconstructs topology from the emitted region depths and
//! collapses equal sibling leaves, producing canonical output without an
//! intermediate tree.
//!
//! # The side-switch algebra
//!
//! On each output region the height equals one input — the *side*, `a` when
//! `sign(D)` favors it, `b` when it favors the other, sticky at ties (`D = 0`
//! keeps the current side, which both inputs then agree on). Join and meet are
//! this one sweep with the side selection reversed — pointwise max follows the
//! higher side, pointwise min the lower — and the selection is everything that
//! distinguishes them: each entry point passes its own picking closure and the
//! sweep never consults which operation it is running. Because the crossing
//! sequence and the running difference are selection-independent, [`Version::hull_bits`]
//! emits both outputs from one sweep, each operand decoded once. The output's
//! delta across a boundary needs no absolute heights:
//!
//! - **Same side**: the output moves with its side, so the delta is
//!   that side's own step delta — zero when the boundary belonged to
//!   the other stream alone.
//! - **Switch**: the output jumps from the old side's plateau to the
//!   new side's. With `D′` the difference *after* the boundary's folds
//!   and `δ` the old side's step delta at this boundary (zero if it
//!   did not step), the jump is `+D′ + δ` switching to `a`, `−D′ + δ`
//!   switching to `b` — both from one sign-and-magnitude read of the
//!   accumulator plus one signed sum. A switch means `D` crossed or
//!   left zero at this boundary, so `|D′|` is bounded by the deltas
//!   just folded, and the read is priced by the codes that carried
//!   them (the accumulator's sign fold has already collapsed any
//!   cancelling prefix by the time the side is picked).
//!
//! Each input boundary is visited once. Arithmetic work is charged to the
//! payload codes that changed `D`, and writer collapse is amortized over the
//! output it removes or retains. Transient state consists of two compact cursor
//! paths, the accumulator, the writer's compact path state, and the output.

#![allow(rustdoc::private_intra_doc_links)]

use core::cmp::Ordering;

use suanpan::Accumulator;

use num_bigint::{BigInt, Sign};

use crate::accumulator;

use super::order::OrderState;
use super::overlay::{advance_diff, OpenedPair, Side};
use crate::version::io::regions::{HeightChange, RegionReader};
use crate::version::io::writer::VersionWriter;
use crate::Version;

impl Side {
    /// The output delta when switching to this side.
    fn switch_delta(self, diff: &Accumulator, old_step: Option<&HeightChange>) -> BigInt {
        let (diff_sign, magnitude) = accumulator::value(diff);
        debug_assert_ne!(diff_sign, Ordering::Equal, "a tie never switches sides");
        let negative = match self {
            Side::A => diff_sign == Ordering::Less,
            Side::B => diff_sign == Ordering::Greater,
        };
        let switched =
            BigInt::from_biguint(if negative { Sign::Minus } else { Sign::Plus }, magnitude);
        match old_step {
            Some(step) => switched + step,
            None => switched,
        }
    }

    /// Write the output delta after a boundary, switching sides when needed.
    fn write_delta(
        self,
        out: &mut VersionWriter,
        depth: u64,
        diff: &Accumulator,
        next: Side,
        step_a: Option<&HeightChange>,
        step_b: Option<&HeightChange>,
    ) {
        let step = match self {
            Side::A => step_a,
            Side::B => step_b,
        };
        if next == self {
            match step {
                Some(step) => out.change(depth, step),
                None => out.unchanged(depth),
            }
            return;
        }
        let delta = next.switch_delta(diff, step);
        out.change(depth, &delta);
    }
}

/// Which pointwise extreme an emission follows.
#[derive(Clone, Copy)]
pub enum Extreme {
    /// The lower height.
    Lower,
    /// The higher height.
    Higher,
}

impl Extreme {
    /// Select a side from the height difference, staying put at ties.
    fn pick(self, sign: Ordering, current: Side) -> Side {
        match (self, sign) {
            (_, Ordering::Equal) => current,
            (Extreme::Higher, Ordering::Greater) | (Extreme::Lower, Ordering::Less) => Side::A,
            (Extreme::Higher, Ordering::Less) | (Extreme::Lower, Ordering::Greater) => Side::B,
        }
    }
}

/// The meet, join, and causal relation produced by one traversal.
pub struct Hull {
    /// The operands' causal order; `None` means concurrent.
    pub relation: Option<Ordering>,
    /// The meet (pointwise min) stream.
    pub lo: Version,
    /// The join (pointwise max) stream.
    pub hi: Version,
}

/// Write both pointwise extremes and determine the operands' causal order in
/// one traversal.
///
/// Meet and join visit the same input boundaries and differ only in which
/// input height they select. Producing both together decodes each input once.
/// The sign already used for selection also determines whether either operand
/// dominates the other.
///
/// # Panics
///
/// [`Version::join`]'s contract exactly: canonical operands required, structural
/// violations panic, the rest yield an unspecified output triple.
impl Version {
    pub(crate) fn hull_bits(&self, other: &Version) -> Hull {
        /// State for one of the two outputs.
        struct Emission {
            extreme: Extreme,
            side: Side,
            out: VersionWriter,
        }

        let OpenedPair {
            a: mut cursor_a,
            b: mut cursor_b,
            mut diff,
            a_first,
            b_first,
        } = OpenedPair::open(self, other);

        // Both outputs and the relation use the same sign on each region.
        let mut directions = OrderState::new();
        let sign = diff.sign();
        directions.fold(sign);

        // Open each output with the selected first height. The side fields are
        // initialized below once the first sign has selected them.
        let mut outputs = [
            Emission {
                extreme: Extreme::Lower,
                side: Side::A,
                out: VersionWriter::with_capacity(self.stored_len() + other.stored_len()),
            },
            Emission {
                extreme: Extreme::Higher,
                side: Side::A,
                out: VersionWriter::with_capacity(self.stored_len() + other.stored_len()),
            },
        ];
        for emission in &mut outputs {
            // At a tie both first heights are equal, so either side is equivalent.
            emission.side = emission.extreme.pick(sign, Side::A);
            let first = match emission.side {
                Side::A => &a_first,
                Side::B => &b_first,
            };
            emission
                .out
                .height(cursor_a.depth().max(cursor_b.depth()), first);
        }
        // The accumulator and outputs now own everything derived from the
        // opening heights. Release wide decoded integers before the walk grows
        // its cursor paths and output buffers.
        drop(a_first);
        drop(b_first);

        while !(cursor_a.done() && cursor_b.done()) {
            // Decode this boundary once, then write both resulting deltas.
            let (step_a, step_b) = advance_diff(&mut cursor_a, &mut cursor_b, &mut diff);
            let sign = diff.sign();
            directions.fold(sign);
            let depth = cursor_a.depth().max(cursor_b.depth());
            for emission in &mut outputs {
                let new_side = emission.extreme.pick(sign, emission.side);
                let old_side = emission.side;
                emission.side = new_side;
                old_side.write_delta(
                    &mut emission.out,
                    depth,
                    &diff,
                    new_side,
                    step_a.as_ref(),
                    step_b.as_ref(),
                );
            }
        }

        let [lo, hi] = outputs;
        Hull {
            relation: directions.relation(),
            lo: lo.out.finish(),
            hi: hi.out.finish(),
        }
    }
}

impl Extreme {
    /// Emit this pointwise extreme in one overlay walk.
    pub fn emit(self, a_bits: &Version, b_bits: &Version) -> Version {
        let OpenedPair {
            a: mut cursor_a,
            b: mut cursor_b,
            mut diff,
            a_first,
            b_first,
        } = OpenedPair::open(a_bits, b_bits);

        // The first region's selected height opens the output. The combined
        // input length is sufficient capacity: output boundaries come from the
        // union of the inputs' boundaries, and each output delta lies between
        // the two input deltas at that boundary. Signed gamma length depends
        // only on magnitude, so the output code is no wider than the wider
        // input code. The opening height comes from one input, and canonical
        // collapse only removes topology and a zero delta. Thus the output is
        // no longer than both inputs together.
        //
        // Equal first heights encode identically, so A may break the tie.
        let mut side = self.pick(diff.sign(), Side::A);
        let mut out = VersionWriter::with_capacity(a_bits.stored_len() + b_bits.stored_len());
        let first = match side {
            Side::A => &a_first,
            Side::B => &b_first,
        };
        out.height(cursor_a.depth().max(cursor_b.depth()), first);
        // The running difference and encoded output retain the opening value;
        // the decoded integers need not overlap the walk's growing state.
        drop(a_first);
        drop(b_first);

        while !(cursor_a.done() && cursor_b.done()) {
            let (step_a, step_b) = advance_diff(&mut cursor_a, &mut cursor_b, &mut diff);
            let new_side = self.pick(diff.sign(), side);
            let old_side = side;
            side = new_side;
            old_side.write_delta(
                &mut out,
                cursor_a.depth().max(cursor_b.depth()),
                &diff,
                new_side,
                step_a.as_ref(),
                step_b.as_ref(),
            );
        }

        out.finish()
    }
}

#[cfg(test)]
mod tests;
