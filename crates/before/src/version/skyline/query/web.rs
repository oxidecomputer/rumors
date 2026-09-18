//! Bookkeeping for the streaming [`min_ticks`](super::min_ticks) fold.
//!
//! The value is
//!
//! `Σ leaf heights − Σ internal-node subtree minima`.
//!
//! A skyline stores the first height and then the differences between adjacent
//! leaves. Reconstructing every absolute height and subtree minimum would make
//! a small difference repeatedly traverse an old, much wider value. This
//! module instead records recent differences and settles each older wide value
//! once.
//!
//! # Tracking heights
//!
//! The sweep treats the current height as `F + L`. `L` is the live drift held
//! in one accumulator. When it becomes much wider than the next stored
//! difference, [`EpochLedger`] moves it into a new epoch; `F` is the conceptual
//! sum of those frozen drifts and is never materialized.
//!
//! Each leaf contributes its live offset immediately and adds one reference to
//! its epoch. A subtree minimum contributes the negative of its offset and its
//! number of uses. At the end, summation by parts recovers every contribution
//! from `F` without rebuilding an absolute height:
//!
//! `Σ_e refs_e · F_e = Σ_f drift_f · Σ_{e ≥ f} refs_e`.
//!
//! Thus each frozen drift participates in one product with the suffix of the
//! reference counts. An offset remains attached to the epoch in which it was
//! observed, so freezing never requires updating existing records.
//!
//! # Tracking subtree minima
//!
//! Open subtrees are nested, so a closing node needs the minimum of the
//! innermost open range. [`ReignTracker`] uses the range-minimum machinery in
//! [`watermark`](crate::version::skyline::watermark) to track that value. A
//! *reign* is the interval during which one leaf value remains the relevant
//! minimum. Its record holds the leaf's live offset, its epoch, and the number
//! of nodes that closed while it was the minimum. Closing a node normally only
//! increments that count. When a lower leaf replaces the minimum, or the range
//! ends, the record is settled once as `offset × count`.
//!
//! [`StoredReign`] stores a signed 32-bit offset, about eight million epochs,
//! and 255 closes in one word. [`ReignStore`] preserves larger values exactly.
//! A spill adds one stable record. Its count still increments in constant time,
//! while a wide offset costs space and settlement work proportional to the
//! offset that the input encoded.
//!
//! # Why the cost remains bounded
//!
//! Each stored difference is folded once into the live height and the stack's
//! gap. Each close increments a count or moves one boundary. When a new
//! minimum propagates outward, every difference it consumes is removed from
//! the stack and folded once at the width previously stored for it. Each freeze
//! moves one live drift into the epoch ledger, and the final settlement visits
//! every epoch once. The wide work is therefore charged to the input that
//! introduced that width; later small differences do not repeatedly read it.

use suanpan::Accumulator;

use num_bigint::{BigInt, BigUint, Sign};

use crate::codec::accumulator;

use super::super::watermark::{Close, RangeMinima};

#[cfg(test)]
mod tests;

/// Add (or, with `subtract`, remove) `factor · digits · 2^shift` in the total:
/// one `factor`-wide product per nonzero signed digit of the compacted `digits`
/// operand.
///
/// The `digits` operand's base-2^32 digits are compacted greedily into balanced
/// signed digits, so an all-ones run — the usual shape of a dyadic mass — costs
/// one subtract at its floor and one carry past its top instead of a product
/// per digit. The `shift` carries a `digits` operand read out at a scale (a
/// segment mass located far along the stream) without ever materializing the
/// scaled value.
///
/// The cost is the factor's width times the operand's compacted density, so
/// this is the settle move for products whose `digits` side stays word-scale —
/// this module's ledgers' reference counts, where the density is O(1) by
/// construction. A product whose both sides the input can widen goes through
/// [`WindowMass::charge`](super::integral::WindowMass::charge) instead, which
/// delegates each dense cluster to the backend's sub-quadratic multiplication.
pub(super) fn mul_into(
    total: &mut Accumulator,
    factor: &BigUint,
    digits: &BigUint,
    shift: u64,
    subtract: bool,
) {
    // A zero digit sequence naturally does no work. Skip a zero factor here
    // because it would otherwise clone and multiply it for every nonzero
    // digit.
    if *factor == BigUint::ZERO {
        return;
    }
    // These shifts are positions within a stored value, whose bit length is
    // below 2^32. They therefore remain well below the accumulator's shift
    // bound even on 32-bit targets.
    let mut carry = 0u64;
    let mut add_term = |digit: u64, sign: Sign, shift: u64| {
        if digit == 0 {
            return;
        }
        let mut product = factor.clone();
        product *= u32::try_from(digit).expect("a compacted signed digit fits 32 bits");
        accumulator::fold(total, &product, shift, (sign == Sign::Minus) != subtract);
    };
    let mut shift = shift;
    for digit in digits
        .iter_u64_digits()
        .flat_map(|limb| [(limb & 0xFFFF_FFFF) as u32, (limb >> 32) as u32])
    {
        let digit_sum = u64::from(digit) + carry;
        if digit_sum > 1 << 31 {
            // Balanced arm: `digit_sum − 2^32` with a carry, so ones-runs
            // cancel.
            add_term((1u64 << 32) - digit_sum, Sign::Minus, shift);
            carry = 1;
        } else {
            add_term(digit_sum, Sign::Plus, shift);
            carry = 0;
        }
        shift += 32;
    }
    if carry == 1 {
        add_term(1, Sign::Plus, shift);
    }
}

/// One minimum's offset, epoch, and number of range closes.
struct Reign {
    /// The signed offset relative to its epoch's frozen component.
    offset: BigInt,
    /// The epoch whose frozen component anchors `offset`.
    epoch: usize,
    /// Closes folded at this record's value, unsettled.
    count: u64,
}

impl Reign {
    /// Construct an exact reign record.
    fn new(offset: BigInt, epoch: usize, count: u64) -> Reign {
        Reign {
            offset,
            epoch,
            count,
        }
    }

    /// Add this completed record's contribution to the total and epoch ledger.
    fn settle(self, total: &mut Accumulator, ledger: &mut EpochLedger) {
        if self.count == 0 {
            return;
        }
        ledger.minimum_refs(self.epoch, self.count);
        // Each counted close subtracted the value once: −(±offset) · count.
        mul_into(
            total,
            self.offset.magnitude(),
            &BigUint::from(self.count),
            0,
            self.offset.sign() != Sign::Minus,
        );
    }
}

/// One reign record, inline when its fields fit or indexed into `ReignStore`.
///
/// An inline record covers every signed 32-bit frozen-relative leaf offset,
/// about eight million freeze epochs, and 255 closes during one reign. In
/// input terms, spilling therefore requires a live offset outside `i32` (from
/// one large jump or more than two billion net ticks since the last freeze),
/// more than eight million nonzero freezes, or a leaf that remains the minimum
/// through more than 255 nested closes. Each epoch is created only when the
/// accumulated live drift is nonzero and more than eight 32-bit digits wider
/// than the next leaf's delta.
#[derive(Clone, Copy)]
struct StoredReign(u64);

/// Bits that hold any signed `i32` offset.
const REIGN_OFFSET_BITS: u32 = 32;
/// Bits that hold the first eight million epochs.
const REIGN_EPOCH_BITS: u32 = 23;
/// Bits that hold up to 255 closes.
const REIGN_COUNT_BITS: u32 = 8;
/// Marks a `StoredReign` as an index into the spill store.
const REIGN_SPILLED: u64 = 1 << 63;
/// Mask selecting the inline offset.
const REIGN_OFFSET_MASK: u64 = (1 << REIGN_OFFSET_BITS) - 1;
/// Mask selecting the inline epoch.
const REIGN_EPOCH_MASK: u64 = (1 << REIGN_EPOCH_BITS) - 1;
/// Mask selecting the inline count.
const REIGN_COUNT_MASK: u64 = (1 << REIGN_COUNT_BITS) - 1;
/// Bit position of the inline epoch.
const REIGN_EPOCH_SHIFT: u32 = REIGN_OFFSET_BITS;
/// Bit position of the inline count.
const REIGN_COUNT_SHIFT: u32 = REIGN_OFFSET_BITS + REIGN_EPOCH_BITS;

const _: () = assert!(REIGN_OFFSET_BITS + REIGN_EPOCH_BITS + REIGN_COUNT_BITS == 63);

impl StoredReign {
    /// Encode a reign in one word when its three fields fit.
    ///
    /// These are representation thresholds, not input limits. The spill path
    /// preserves every larger value exactly.
    fn inline(offset: &BigInt, epoch: usize) -> Option<Self> {
        let offset = i32::try_from(offset).ok()?;
        if epoch > REIGN_EPOCH_MASK as usize {
            return None;
        }
        let offset = u64::from(offset as u32);
        Some(Self(offset | (epoch as u64) << REIGN_EPOCH_SHIFT))
    }

    /// Whether this word names an out-of-line record.
    fn is_spilled(self) -> bool {
        self.0 & REIGN_SPILLED != 0
    }

    /// Decode an inline record.
    fn decode(self) -> Reign {
        debug_assert!(!self.is_spilled());
        let raw_offset = self.0 & REIGN_OFFSET_MASK;
        let offset = raw_offset as u32 as i32;
        let epoch = ((self.0 >> REIGN_EPOCH_SHIFT) & REIGN_EPOCH_MASK) as usize;
        let count = (self.0 >> REIGN_COUNT_SHIFT) & REIGN_COUNT_MASK;
        Reign::new(BigInt::from(offset), epoch, count)
    }

    /// Index of the exact record in the spill store.
    fn spill_index(self) -> usize {
        debug_assert!(self.is_spilled());
        (self.0 & !REIGN_SPILLED) as usize
    }
}

/// Exact storage for reigns that do not fit `StoredReign`'s inline form.
///
/// A spill allocates one stable slot and clones the offset once. Subsequent
/// count increments remain O(1), settlement uses the same arithmetic as an
/// inline record, and the slot can be reused. Wide offsets use space
/// proportional to their encoded magnitude; epoch and count overflows add one
/// fixed-size record to an input already containing the corresponding freezes
/// or nested closes.
struct ReignStore {
    /// Stable slots addressed by spilled records.
    slots: Vec<Option<Reign>>,
    /// Vacant slots available for reuse.
    free: Vec<usize>,
}

impl ReignStore {
    /// Construct an empty spill store.
    fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    /// Store a fresh reign inline when possible.
    fn store(&mut self, offset: &BigInt, epoch: usize) -> StoredReign {
        StoredReign::inline(offset, epoch)
            .unwrap_or_else(|| self.spill(Reign::new(offset.clone(), epoch, 0)))
    }

    /// Put an exact reign in a stable spill slot.
    fn spill(&mut self, reign: Reign) -> StoredReign {
        let index = if let Some(index) = self.free.pop() {
            debug_assert!(self.slots[index].is_none());
            self.slots[index] = Some(reign);
            index
        } else {
            let index = self.slots.len();
            self.slots.push(Some(reign));
            index
        };
        let index = u64::try_from(index).expect("reign spill index fits u64");
        assert!(index < REIGN_SPILLED, "reign spill index fits 63 bits");
        StoredReign(REIGN_SPILLED | index)
    }

    /// Count one close against a stored reign, spilling on inline overflow.
    fn increment(&mut self, reign: &mut StoredReign) {
        if reign.is_spilled() {
            self.slots[reign.spill_index()]
                .as_mut()
                .expect("a spilled reign owns its slot")
                .count += 1;
            return;
        }
        let count = (reign.0 >> REIGN_COUNT_SHIFT) & REIGN_COUNT_MASK;
        if count < REIGN_COUNT_MASK {
            reign.0 += 1 << REIGN_COUNT_SHIFT;
            return;
        }
        let mut exact = reign.decode();
        exact.count += 1;
        *reign = self.spill(exact);
    }

    /// Remove a stored reign, returning its exact values.
    fn take(&mut self, reign: StoredReign) -> Reign {
        if !reign.is_spilled() {
            return reign.decode();
        }
        let index = reign.spill_index();
        let exact = self.slots[index]
            .take()
            .expect("a spilled reign owns its slot");
        self.free.push(index);
        exact
    }

    /// Settle a stored reign and release any spill slot it occupied.
    fn settle(&mut self, reign: StoredReign, total: &mut Accumulator, ledger: &mut EpochLedger) {
        self.take(reign).settle(total, ledger);
    }
}

/// Mutable accounting used while arming a range.
struct ReignContext<'a> {
    /// The record for the current innermost minimum.
    winner: &'a mut Option<StoredReign>,
    /// Storage for exact out-of-line records.
    reigns: &'a mut ReignStore,
    /// Offset of the leaf that may begin a new reign.
    offset: &'a BigInt,
    /// Epoch of the leaf that may begin a new reign.
    epoch: usize,
    /// Running min-ticks total.
    total: &'a mut Accumulator,
    /// Frozen-height accounting by epoch.
    ledger: &'a mut EpochLedger,
}

/// State needed to settle records consumed by an undercut.
struct SettlementContext<'a> {
    /// Storage for exact out-of-line records.
    reigns: &'a mut ReignStore,
    /// Running min-ticks total.
    total: &'a mut Accumulator,
    /// Frozen-height accounting by epoch.
    ledger: &'a mut EpochLedger,
}

/// Tracks the current minimum and its accounting record for every open range.
pub(super) struct ReignTracker {
    /// Nested minima; each positive boundary carries the outer minimum's record.
    minima: RangeMinima<StoredReign>,
    /// The innermost minimum's record; present while a range is armed.
    winner: Option<StoredReign>,
    /// Exact records that exceed the common inline representation.
    reigns: ReignStore,
}

impl ReignTracker {
    /// Construct an empty tracker with no open ranges or current minimum.
    pub(super) fn new() -> ReignTracker {
        ReignTracker {
            minima: RangeMinima::new(),
            winner: None,
            reigns: ReignStore::new(),
        }
    }

    /// Open `count` ranges: the internal nodes a descent just entered.
    pub(super) fn open(&mut self, count: u64) {
        self.minima.open(count);
    }

    /// Move the running height by one consumed delta.
    pub(super) fn fold_height(&mut self, delta: &BigInt) {
        self.minima.fold_height(delta);
    }

    /// Close the innermost range and add its minimum to the total.
    ///
    /// If the parent has the same minimum, its current record continues. If
    /// the parent has a lower minimum, that record resumes and the inner one is
    /// settled. Closing the final range settles the final record.
    pub(super) fn close(&mut self, total: &mut Accumulator, ledger: &mut EpochLedger) {
        // Count the close on the record that supplied this range's minimum,
        // before that record continues, settles, or is replaced.
        match self.minima.close() {
            Close::Equal => {
                self.reigns.increment(
                    self.winner
                        .as_mut()
                        .expect("an armed range has a current record"),
                );
            }
            Close::Retired => {
                let mut reign = self.winner.take().expect("the reigning record was live");
                self.reigns.increment(&mut reign);
                self.reigns.settle(reign, total, ledger);
            }
            Close::Lower(interrupted) => {
                let mut dead = self
                    .winner
                    .replace(interrupted)
                    .expect("the reigning record was live");
                self.reigns.increment(&mut dead);
                self.reigns.settle(dead, total, ledger);
            }
        }
    }

    /// Record one leaf at the running height, with its narrow frozen-relative
    /// offset and epoch.
    ///
    /// Arms any pending ranges at the leaf; otherwise an amortized sign read
    /// decides whether the leaf undercuts the innermost minimum, and only a
    /// true undercut does more than O(1) work. It consumes each crossed
    /// boundary and settles that boundary's record once.
    pub(super) fn leaf(
        &mut self,
        offset: &BigInt,
        epoch: usize,
        total: &mut Accumulator,
        ledger: &mut EpochLedger,
    ) {
        if self.minima.has_pending() {
            if !self.minima.armed() {
                // The first minimum also establishes the anchor.
                self.winner = Some(self.reigns.store(offset, epoch));
            }
            // A higher minimum stores the prior record on its boundary. An
            // equal minimum keeps that record. A lower minimum settles it.
            let mut context = ReignContext {
                winner: &mut self.winner,
                reigns: &mut self.reigns,
                offset,
                epoch,
                total,
                ledger,
            };
            self.minima.arm_at_height(
                &mut context,
                |context| {
                    context
                        .winner
                        .replace(context.reigns.store(context.offset, context.epoch))
                        .expect("an armed range has a current record")
                },
                |reign, context| context.reigns.settle(reign, context.total, context.ledger),
            );
            return;
        }
        if !self.minima.armed() {
            // A single-leaf stream: no node will ever fold a minimum.
            return;
        }
        if !self.minima.undercuts_here() {
            return;
        }
        // The new leaf supplies the minimum. Settle the displaced record, then
        // settle every outer record whose boundary the drop consumes.
        let dead = self
            .winner
            .take()
            .expect("an armed range has a current record");
        self.reigns.settle(dead, total, ledger);
        self.winner = Some(self.reigns.store(offset, epoch));
        let mut context = SettlementContext {
            reigns: &mut self.reigns,
            total,
            ledger,
        };
        self.minima.undercut(&mut context, |reign, context| {
            context.reigns.settle(reign, context.total, context.ledger);
        });
    }

    /// Close every remaining range at the stream's end.
    pub(super) fn drain(&mut self, total: &mut Accumulator, ledger: &mut EpochLedger) {
        debug_assert!(
            !self.minima.has_pending(),
            "the final leaf armed every open range"
        );
        while self.minima.armed() {
            self.close(total, ledger);
        }
    }
}

/// Frozen height drifts and their reference counts, grouped by epoch.
pub(super) struct EpochLedger {
    /// One signed drift per epoch: entry 0 is the first leaf's absolute height,
    /// every later entry one freeze's evicted live drift.
    drifts: Vec<BigInt>,
    /// Per epoch, the signed count of events denominated in that epoch's frozen
    /// component: `+1` per leaf, `−count` per settled reign.
    refs: Vec<i128>,
}

impl EpochLedger {
    /// Open the ledger at epoch 0: the first leaf's absolute height is the
    /// opening frozen component.
    pub(super) fn new(first: BigUint) -> EpochLedger {
        EpochLedger {
            drifts: vec![BigInt::from(first)],
            refs: vec![0],
        }
    }

    /// The current epoch, equal to the number of stored nonzero drifts.
    pub(super) fn epoch(&self) -> usize {
        self.drifts.len() - 1
    }

    /// Count one leaf against the current epoch.
    pub(super) fn leaf_ref(&mut self) {
        *self.refs.last_mut().expect("epoch 0 always exists") += 1;
    }

    /// Count a settled reign's closes against its record's epoch.
    fn minimum_refs(&mut self, epoch: usize, count: u64) {
        self.refs[epoch] -= i128::from(count);
    }

    /// Evict the live drift into a new epoch, or keep the epoch when its terms
    /// cancel to zero, then reset the live component.
    pub(super) fn freeze(&mut self, live: &mut Accumulator) {
        let drift = accumulator::signed_value(live);
        if drift.sign() != Sign::NoSign {
            self.drifts.push(drift);
            self.refs.push(0);
        }
        live.reset();
    }

    /// Settle `Σ_e refs_e · F_e` by summation by parts, using one
    /// `drift × suffix-count` product per epoch.
    pub(super) fn settle(self, total: &mut Accumulator) {
        let mut suffix: i128 = 0;
        for (drift, refs) in self.drifts.iter().zip(&self.refs).rev() {
            suffix += refs;
            if suffix == 0 {
                continue;
            }
            let count = BigUint::from(suffix.unsigned_abs());
            mul_into(
                total,
                drift.magnitude(),
                &count,
                0,
                (drift.sign() == Sign::Minus) != (suffix < 0),
            );
        }
        debug_assert_eq!(
            suffix, 1,
            "leaves exceed closed nodes by exactly one, so the net reference is one"
        );
    }
}
