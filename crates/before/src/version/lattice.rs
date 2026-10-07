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
//! one [`sweep`] with the side selection reversed — pointwise max follows the
//! higher side, pointwise min the lower — and the selection is everything that
//! distinguishes them: each entry point passes one [`Extreme`] per output it
//! requests, and the sweep reads those values only through [`Extreme::pick`],
//! never asking which operation it is running. Because the crossing sequence
//! and the running difference are selection-independent, one sweep can emit
//! several extremes: [`Extreme::emit`] requests one, and [`Version::hull_bits`]
//! requests both, decoding each operand once. The output's delta across a
//! boundary needs no absolute heights:
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
//! paths, the accumulator, and, for each emitted extreme, a writer's compact
//! path state and its output.

#![allow(rustdoc::private_intra_doc_links)]

use core::cmp::Ordering;

use suanpan::Accumulator;

use num_bigint::{BigInt, Sign};

use crate::accumulator::BigIntAccumulator as _;

use super::order::OrderState;
use super::overlay::{advance_diff, OpenedPair, Side};
use crate::version::io::regions::{HeightChange, RegionReader};
use crate::version::io::writer::VersionWriter;
use crate::Version;

impl Side {
    /// The output delta when switching to this side.
    fn switch_delta(self, diff: &Accumulator, old_step: Option<&HeightChange>) -> BigInt {
        let (diff_sign, magnitude) = diff.biguint_parts();
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

    /// Emit this pointwise extreme in one overlay walk.
    ///
    /// # Panics
    ///
    /// Panics as [`sweep`] does.
    pub fn emit(self, a_bits: &Version, b_bits: &Version) -> Version {
        let ([out], _) = sweep(a_bits, b_bits, [self]);
        out
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

impl Version {
    /// Write both pointwise extremes and determine the operands' causal order
    /// in one traversal.
    ///
    /// Meet and join visit the same input boundaries and differ only in which
    /// input height they select. Producing both together decodes each input
    /// once. The sign already used for selection also determines whether
    /// either operand dominates the other.
    ///
    /// # Panics
    ///
    /// Panics as [`sweep`] does.
    pub(crate) fn hull_bits(&self, other: &Version) -> Hull {
        let ([lo, hi], relation) = sweep(self, other, [Extreme::Lower, Extreme::Higher]);
        Hull { relation, lo, hi }
    }
}

/// One output of [`sweep`]: the extreme it follows, the input whose height it
/// currently takes, and the stream written so far.
struct Emission {
    /// The extreme this output follows.
    extreme: Extreme,
    /// The input supplying the output's height on the current region.
    side: Side,
    /// The output stream.
    out: VersionWriter,
}

/// Walk both operands once, emitting each requested extreme and the operands'
/// causal relation.
///
/// The `i`th output follows `extremes[i]`. Each boundary is decoded once and
/// its sign is shared: every output picks its side from it, and an
/// [`OrderState`] folds it into the relation, which is `None` when the operands
/// are concurrent.
///
/// # Panics
///
/// Panics if either operand's stream is structurally malformed; other
/// non-canonical streams yield unspecified outputs and relation. Every
/// [`Version`] holds one canonical stream, so neither case arises from a value
/// the crate built.
fn sweep<const N: usize>(
    a: &Version,
    b: &Version,
    extremes: [Extreme; N],
) -> ([Version; N], Option<Ordering>) {
    let OpenedPair {
        a: mut cursor_a,
        b: mut cursor_b,
        mut diff,
        a_first,
        b_first,
    } = OpenedPair::open(a, b);

    let mut directions = OrderState::new();
    let sign = diff.cmp_zero();
    directions.fold(sign);

    // The first region's selected height opens each output. The combined input
    // length is sufficient capacity: output boundaries come from the union of
    // the inputs' boundaries, and each output delta lies between the two input
    // deltas at that boundary. Signed gamma length depends only on magnitude,
    // so the output code is no wider than the wider input code. The opening
    // height comes from one input, and canonical collapse only removes topology
    // and a zero delta. Thus each output is no longer than both inputs
    // together.
    let depth = cursor_a.depth().max(cursor_b.depth());
    let mut outputs = extremes.map(|extreme| {
        // Equal first heights encode identically, so A may break the tie.
        let side = extreme.pick(sign, Side::A);
        let first = match side {
            Side::A => &a_first,
            Side::B => &b_first,
        };
        let mut out = VersionWriter::with_capacity(a.stored_len() + b.stored_len());
        out.height(depth, first);
        Emission { extreme, side, out }
    });

    // The running difference and encoded outputs retain the opening values;
    // the decoded integers need not overlap the walk's growing state.
    drop(a_first);
    drop(b_first);

    while !(cursor_a.done() && cursor_b.done()) {
        let (step_a, step_b) = advance_diff(&mut cursor_a, &mut cursor_b, &mut diff);
        let sign = diff.cmp_zero();
        directions.fold(sign);
        let depth = cursor_a.depth().max(cursor_b.depth());
        for emission in &mut outputs {
            let old_side = emission.side;
            emission.side = emission.extreme.pick(sign, old_side);
            old_side.write_delta(
                &mut emission.out,
                depth,
                &diff,
                emission.side,
                step_a.as_ref(),
                step_b.as_ref(),
            );
        }
    }

    (
        outputs.map(|emission| emission.out.finish()),
        directions.relation(),
    )
}

#[cfg(test)]
mod tests;
