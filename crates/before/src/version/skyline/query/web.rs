//! The min-ticks fold's bookkeeping: subtree minima as reigns over the
//! anchored-minimum web, and the frozen component as an epoch ledger settled
//! once.
//!
//! [`min_ticks`](super::min_ticks) folds `Σ leaf heights − Σ internal-node
//! subtree minima` over one leaf sweep. Both sums are event streams over
//! evolving wide quantities — each leaf folds the running height, each closing
//! node folds the minimum of its completed span — and this module is the
//! accounting that keeps every event narrow: no event ever reads a width its
//! own boundary's codes did not pay for.
//!
//! # The minima side: reigns over the anchored-minimum web
//!
//! Subtree spans nest LIFO along the sweep, so "the closing node's subtree
//! minimum" is always *the innermost open range's minimum* at the close — the
//! range-minimum problem the anchored-minimum web solves. The web itself — the
//! one `gap` register against a shared anchor, the latent boundary a close
//! parks by move, the compressed difference stack, and the propagation each
//! undercut funds — is [`watermark`](crate::version::skyline::watermark)'s,
//! held once for both its clients; this module drives it through [`ReignWeb`]
//! at payload [`Reign`] and contributes only the fold's own semantics.
//!
//! What the closes *fold into the total* rides the web as value identity: a
//! [`Reign`]. The innermost minimum's value is a leaf height the sweep already
//! paid for — recorded once, at the boundary that made it the minimum, as a
//! narrow frozen-relative offset (below) — and every close while that value
//! reigns just counts. The record settles into the total exactly once, at its
//! death (a lower leaf dethrones it, a propagating drop annihilates its
//! difference, or the stream ends), as one compacted `offset × count` product
//! priced by the offset's width. A record whose reign is interrupted — an inner
//! range arms above it — rides the interrupting boundary as its payload and
//! returns at the pop with its count intact, so an interruption never re-reads
//! the offset.
//!
//! # The heights side: the epoch ledger
//!
//! The sweep splits the running height `h = F + L`: `L` (*live*) the drift
//! since the last freeze on one accumulator, `F` (*frozen*) the rest — never
//! materialized anywhere. [`EpochLedger`] holds one signed drift per freeze
//! (epoch 0's "drift" is the first leaf's absolute) and one signed reference
//! count per epoch: a leaf event folds its narrow `L`-offset into the total
//! directly and counts `+1` against its epoch; a settling reign counts `−count`
//! against the epoch its offset was recorded under. The frozen component
//! reaches the total once, at the end, by summation by parts:
//!
//! `Σ_e refs_e · F_e = Σ_f drift_f · Σ_{e ≥ f} refs_e`
//!
//! — one compacted `drift × suffix-count` product per freeze, priced by the
//! drift's own width (which the codes that built the drift funded) times the
//! count's O(1) compacted digits. No event is ever re-based across a freeze: an
//! offset recorded under epoch `e` keeps its epoch, and the ledger's settle
//! carries the frozen difference for it.
//!
//! # Funding: the potential function and its arity
//!
//! The certificate is a **one-ledger potential over the single operand**:
//! folding a code of `w` digits deposits `Θ(w)` into `Φ`, and each topology bit
//! deposits O(1). Every charge names its deposit:
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
//!   any fold, with zero runs passing whole (the `watermark` module's
//!   width-conservation discipline);
//! - a freeze: the drift's one eviction read, funded by the codes that
//!   built the drift, which the eviction consumes and resets;
//! - the ledger settle: one product per freeze at the evicted drift's
//!   own width.
//!
//! The arity is one: min_ticks is a single-stream fold, so no charge can draw
//! on a ledger its own operand did not fund — the two-operand co-sweep's
//! per-operand split (the funding section of [`integral`](super::integral)) is
//! not needed here.

use core::cmp::Ordering;

use suanpan::{Accumulator, Limbs, UBig};

use crate::codec::{Base, Int};

use super::super::signed::Sign;
use super::super::watermark::{Close, MinWeb};

/// A magnitude's little-endian base-2^32 digits: [`mul_into`]'s read of its
/// `digits` operand.
///
/// The top digit of the top limb may be zero (the compaction loop skips
/// zero digits, so the padding is free).
fn u32_digits(value: &Base) -> Vec<u32> {
    Limbs::new(&value.0)
        .flat_map(|limb| [(limb & 0xFFFF_FFFF) as u32, (limb >> 32) as u32])
        .collect()
}

/// Add (or, with `subtract`, remove) `factor · digits · 2^shift` in the total:
/// one `factor`-wide product per nonzero signed digit of the compacted `digits`
/// operand.
///
/// The `digits` operand's base-2^32 digits are compacted greedily into balanced
/// signed digits, so an all-ones run — the usual shape of a dyadic mass — costs
/// one subtract at its floor and one carry past its top instead of a product
/// per digit. The `shift` carries a `digits` operand read out at a scale (a
/// segment mass parked deep in the stream) without ever materializing the
/// scaled value.
///
/// The cost is the factor's width times the operand's compacted density, so
/// this is the settle move for products whose `digits` side stays word-scale —
/// this module's ledgers' reference counts, where the density is O(1) by
/// construction. A product whose both sides the input can widen goes through
/// [`charge_digits`](super::integral::charge_digits) instead, which delegates
/// each dense cluster to the backend's sub-quadratic multiplication.
pub(super) fn mul_into(
    total: &mut Accumulator,
    factor: &Base,
    digits: &Base,
    shift: u64,
    subtract: bool,
) {
    if *factor == Base::ZERO || *digits == Base::ZERO {
        return;
    }
    let mut carry = 0u64;
    let mut add_term = |digit: u64, sign: Sign, shift: u64| {
        if digit == 0 {
            return;
        }
        let mut product = factor.clone();
        product *= u32::try_from(digit).expect("a compacted signed digit fits 32 bits");
        if sign.is_negative() == subtract {
            total.add_magnitude_shl(&product, shift);
        } else {
            total.sub_magnitude_shl(&product, shift);
        }
    };
    let mut shift = shift;
    for digit in u32_digits(digits) {
        let digit_sum = u64::from(digit) + carry;
        if digit_sum > 1 << 31 {
            // Balanced arm: `digit_sum − 2^32` with a carry, so ones-runs
            // cancel.
            add_term((1u64 << 32) - digit_sum, Sign::Negative, shift);
            carry = 1;
        } else {
            add_term(digit_sum, Sign::Positive, shift);
            carry = 0;
        }
        shift += 32;
    }
    if carry == 1 {
        add_term(1, Sign::Positive, shift);
    }
}

/// The value the innermost minimum currently holds, as the sweep folds it: a
/// frozen-relative offset, its epoch, and the closes counted at it since the
/// record's mint (module doc: the minima side).
struct Reign {
    /// The offset's sign.
    sign: Sign,
    /// The offset's magnitude, relative to its epoch's frozen component.
    offset: Base,
    /// The epoch whose frozen component anchors `offset`.
    epoch: u32,
    /// Closes folded at this record's value, unsettled.
    count: u64,
}

impl Reign {
    /// A fresh record at a leaf's value, no closes counted yet.
    fn mint(sign: Sign, offset: &Base, epoch: u32) -> Reign {
        Reign {
            sign,
            offset: offset.clone(),
            epoch,
            count: 0,
        }
    }
}

/// Settle a dying record: one compacted `offset × count` product into the
/// narrow total, and the count against the record's epoch.
fn settle(reign: Reign, total: &mut Accumulator, ledger: &mut EpochLedger) {
    if reign.count == 0 {
        return;
    }
    ledger.minimum_refs(reign.epoch, reign.count);
    // Each counted close subtracted the value once: −(±offset) · count.
    mul_into(
        total,
        &reign.offset,
        &Base::from(reign.count),
        0,
        !reign.sign.is_negative(),
    );
}

/// The min-ticks fold's view of the anchored-minimum web: the shared core at
/// payload [`Reign`], plus the innermost minimum's own record (module doc).
pub(super) struct ReignWeb {
    /// The anchored-minimum web, each stacked boundary carrying the record
    /// whose reign that arming interrupted.
    web: MinWeb<Reign>,
    /// The innermost minimum's record; `Some` exactly while the web is
    /// armed.
    winner: Option<Reign>,
}

impl ReignWeb {
    pub(super) fn new() -> ReignWeb {
        ReignWeb {
            web: MinWeb::compacting(),
            winner: None,
        }
    }

    /// Open `count` ranges: the internal nodes a descent just entered.
    pub(super) fn open(&mut self, count: usize) {
        self.web.open(count);
    }

    /// Fold one consumed delta into the height side of the web's `gap`.
    pub(super) fn fold_height(&mut self, sign: Sign, magnitude: &Int) {
        self.web.fold_height(sign, magnitude);
    }

    /// Close the innermost range: fold its minimum into the total (one count on
    /// the reigning record) and merge it into its parent.
    ///
    /// The web's close is O(1) — a zero-run decrement, or a boundary move into
    /// the latent register — and its outcome carries the reign bookkeeping: a
    /// parked boundary's record resumes reigning with its count intact while
    /// the inner record dies by its one settle, and the last close settles the
    /// final record as the web retires.
    pub(super) fn close(&mut self, total: &mut Accumulator, ledger: &mut EpochLedger) {
        // The reigning record counts this close on every outcome — the
        // increment rides each arm, after the dispatch, because each arm
        // hands the record off differently (kept, taken, replaced) and the
        // count must land on the record that reigned over this close.
        match self.web.close() {
            Close::ZeroRun => {
                self.winner
                    .as_mut()
                    .expect("an armed web has a reigning record")
                    .count += 1;
            }
            Close::Retired => {
                let mut reign = self.winner.take().expect("the reigning record was live");
                reign.count += 1;
                settle(reign, total, ledger);
            }
            Close::Parked(interrupted) => {
                let mut dead = self
                    .winner
                    .replace(interrupted)
                    .expect("the reigning record was live");
                dead.count += 1;
                settle(dead, total, ledger);
            }
        }
    }

    /// Record one leaf at the running height, with its narrow frozen-relative
    /// offset (`sign`, `offset`) and epoch.
    ///
    /// Arms any pending ranges at the leaf; otherwise an amortized sign read
    /// decides whether the leaf undercuts the innermost minimum, and only a
    /// true undercut does more than O(1) work — funded by the differences it
    /// consumes, each dying record settling as its difference dies.
    pub(super) fn leaf(
        &mut self,
        sign: Sign,
        offset: &Base,
        epoch: u32,
        total: &mut Accumulator,
        ledger: &mut EpochLedger,
    ) {
        if self.web.has_pending() {
            if !self.web.armed() {
                // The first arming: the web seats its anchor at the leaf.
                self.winner = Some(Reign::mint(sign, offset, epoch));
            }
            // The trichotomy through the hooks: an arming above the old
            // minimum stacks the interrupted record as the boundary's
            // payload, an exact meet leaves the old record reigning
            // untouched, and an arming undercut settles it dead — each
            // record minted or moved only on the arm that needs it.
            let winner = &mut self.winner;
            self.web.arm_at_height(
                || {
                    winner
                        .replace(Reign::mint(sign, offset, epoch))
                        .expect("an armed web has a reigning record")
                },
                |reign| settle(reign, total, ledger),
            );
            return;
        }
        if !self.web.armed() {
            // A single-leaf stream: no node will ever fold a minimum.
            return;
        }
        if !self.web.undercuts_here() {
            return;
        }
        // A true undercut: the old record dies by its settle, the new leaf
        // reigns, and the drop propagates outward, settling each record
        // whose difference it consumes.
        let dead = self
            .winner
            .replace(Reign::mint(sign, offset, epoch))
            .expect("an armed web has a reigning record");
        settle(dead, total, ledger);
        self.web.undercut(|reign| settle(reign, total, ledger));
    }

    /// Close every remaining range at the stream's end.
    pub(super) fn drain(&mut self, total: &mut Accumulator, ledger: &mut EpochLedger) {
        debug_assert!(
            !self.web.has_pending(),
            "the final leaf armed every open range"
        );
        while self.web.armed() {
            self.close(total, ledger);
        }
    }
}

/// The frozen component as per-epoch drifts and reference counts, settled by
/// summation by parts once, at the end (module doc: the heights side).
pub(super) struct EpochLedger {
    /// One signed drift per epoch: entry 0 is the first leaf's absolute height,
    /// every later entry one freeze's evicted live drift.
    drifts: Vec<(Sign, Base)>,
    /// Per epoch, the signed count of events denominated in that epoch's frozen
    /// component: `+1` per leaf, `−count` per settled reign.
    refs: Vec<i128>,
}

impl EpochLedger {
    /// Open the ledger at epoch 0: the first leaf's absolute height is the
    /// opening frozen component.
    pub(super) fn new(first: Base) -> EpochLedger {
        EpochLedger {
            drifts: vec![(Sign::Positive, first)],
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

    /// Evict the live drift into a new epoch (or discard a redundantly spelled
    /// zero, keeping the epoch), resetting the live component.
    pub(super) fn freeze(&mut self, live: &mut Accumulator) {
        let (sign, drift) = live.sign_magnitude();
        if drift != UBig::ZERO {
            self.drifts.push((
                Sign::from_is_negative(sign == Ordering::Less),
                Base::from(drift),
            ));
            self.refs.push(0);
        }
        live.reset();
    }

    /// Settle the frozen component: `Σ_e refs_e · F_e` by summation by parts —
    /// one `drift × suffix-count` product per epoch, each priced by the drift's
    /// own width.
    pub(super) fn settle(self, total: &mut Accumulator) {
        let mut suffix: i128 = 0;
        for ((drift_sign, drift), refs) in self.drifts.iter().zip(&self.refs).rev() {
            suffix += refs;
            if suffix == 0 {
                continue;
            }
            let count = Base::from(suffix.unsigned_abs());
            mul_into(
                total,
                drift,
                &count,
                0,
                drift_sign.is_negative() != (suffix < 0),
            );
        }
        debug_assert_eq!(
            suffix, 1,
            "leaves exceed closed nodes by exactly one, so the net reference is one"
        );
    }
}
