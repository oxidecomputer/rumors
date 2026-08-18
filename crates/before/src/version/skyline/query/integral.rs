//! The anchored-segment integral: the machinery every rank-family fold runs on
//! — the height split, the freeze discipline, the promotion ledger, and the
//! settle tree — with the funding certificate for every charge.
//!
//! [`rank`](super::rank) runs this integral on one stream: its integrand is the
//! height itself. [`distance`](super::distance), [`lag`](super::lag), and
//! [`rank_cmp`](super::rank_cmp) run it on the pair co-sweep's integrand `h* =
//! σ·D`, for the measure's orientation `σ` (the σ table in the [`query`](super)
//! module doc). This doc derives the per-boundary algebra, the split, and the
//! funding argument the public cost claims rest on.
//!
//! # The per-boundary algebra
//!
//! The co-sweep maintains the running difference `D = h_a − h_b` exactly as the
//! comparison sweep does, and integrates `h* = σ·D` with `σ ∈ {−1, 0, +1}`
//! constant on every interval of constant `D`-sign. Per boundary, with `σ → σ′`
//! and net folded difference `dD`, the integrand moves by
//!
//! `dh* = (σ′ − σ) · D′ + σ · dD`
//!
//! Two terms, two prices. The `σ·dD` term re-folds the boundary's own codes:
//! each consumed delta enters `D` once and `h*` at most once, orientation being
//! a side swap. The `(σ′ − σ)·D′` term materializes the difference itself, but
//! only at orientation changes — and in every row of the σ table an orientation
//! change requires `D` to have crossed, left, or entered zero at this very
//! boundary (the rank order's constant σ never changes at all). So `|D′| ≤
//! |dD|`, and the read — after the sign fold's collapse — is priced by the
//! codes just folded, the same argument the emission sweep's side switch rests
//! on.
//!
//! # The height split
//!
//! Each elementary interval must add `h* · 2^(S − depth)` (`S` the overlay's
//! maximum depth), but a per-interval read of the full integrand re-imports the
//! quadratic the delta coding invites: on the boundary comb the height is a
//! `2^k`-scale value behind 3-bit stored deltas. The integral therefore splits
//! its running quantity into anchored components folded narrow.
//!
//! The anchoring is the point. A freeze must not settle evicted drift against
//! its *absolute* position: positions grow arbitrarily dense while the codes at
//! hand stay cheap. A single stream can alternate isolated wide drops with unit
//! drops down a spine (the freeze-position board family), firing a freeze per
//! block at ever-growing written position spans. The overlay is worse: one
//! operand's cheap boundaries fire freezes of drift the other operand's wide
//! codes deposited, at positions whose compacted density neither operand's
//! codes funded. (On the two-operand jump comb — the jump-pair board family's
//! shape: a shared descent spine planting isolated position bits, then an
//! `m`-level comb where one operand's wide teeth cross the other's near-flat
//! band — every crest of `|D|` would pay a drift-width × position-density
//! product, superlinear in the packed pair while each operand alone stays
//! flat.) The integral therefore works in *anchored segments*: no correction,
//! at any point of the sweep or its close, multiplies by an absolute position.
//!
//! The integrand splits `h* = B + P + L` (for rank, `h* = h` itself):
//!
//! - `L` (*live*): the drift since the last freeze. Each elementary
//!   interval adds `L · 2^(S − depth)` directly — O(`L`'s digits),
//!   bounded by the previous boundary's widest folded code plus the
//!   freeze allowance, and the trigger below empties `L` before a
//!   second unfunded interval could ride a stale width.
//! - `P` (*parked*): drift a freeze moved out of `L`, anchored at that
//!   freeze. A segment-mass accumulator sums the interval masses since
//!   `P`'s anchor — the feed opens at the first freeze, because the
//!   pre-freeze mass funds no settle (the promotion-ledger section
//!   below and the [`Integrator::frozen`] field doc derive why), so a
//!   sweep that never freezes deposits no interval mass
//!   at all; the next freeze (or the stream end) settles
//!   `P · segment` in one compacted product and re-anchors. The
//!   segment mass's nonzero span is the *depth variation inside the
//!   segment* — the dyadic positions' shared prefix never appears in
//!   it — so a crest settled one comb level later costs `P`'s width
//!   times O(1) digits however dense the absolute position is, and
//!   oscillating drift cancels digit-wise inside `P` instead of
//!   re-paying its width. The segment mass is read through the
//!   accumulator's write-watermark read (`sign_magnitude_shl`) and
//!   cleared by buffer replacement, so a segment parked deep in the
//!   stream costs its written span, never its scale.
//! - `B` (*base*): the opening `h*` plateau, anchored at position zero
//!   and closing in a single shifted add `B · 2^S`.
//!
//! # The freeze trigger
//!
//! A freeze fires once a folded delta leaves the live component wider than
//! that delta's own code by an allowance sized by [`FREEZE_ALLOWANCE_DIGITS`]:
//! stale wide drift is about to ride under cheaper codes, so the sweep evicts
//! it once — charged to the codes that built the drift, which the freeze
//! consumes and resets — and the cheap codes continue on an emptied live
//! component. The engagement boundary itself is a tuning choice inside the
//! deliberate cost allowance — settled values are identical on either side of
//! it. What holds at every tuning: bounded oscillation at *any* width keeps
//! the live component within its own codes' width and never freezes — every
//! wide-tooth fold is paid by the tooth's own code, on either side of any
//! fixed width.
//!
//! The pair co-sweep denominates the same trigger once per boundary, against
//! the *boundary's* widest folded code, not per folded delta. The behavior it
//! buys is the same: bounded oscillation at any width never freezes, and wide
//! drift riding under cheaper codes is parked at the first such code.
//!
//! # The promotion ledger
//!
//! `P` is *promoted* out of the per-freeze settle when incoming drift runs more
//! than the allowance narrower than `P`. Without promotion a wide `P` would
//! re-settle its full width at every later narrow-drift freeze; with it, every
//! settle's `P` is within the allowance of the drift the settling freeze itself
//! parks.
//!
//! A promotion performs two funded-width reads and no product: the parked
//! component, at the width its arming deposited, and the *position window* —
//! the interval mass banked since the previous promotion, one compacted segment
//! mass per freeze, read at its watermark span. The two are recorded together
//! as one ledger entry. Nothing is ever re-based against an absolute position:
//! the entry owes `P · (2^S − position)`, and the ledger settles once, at the
//! sweep's close.
//!
//! # The settle tree
//!
//! The ledger settles as one mass-balanced product tree over the entry
//! sequence. Each tree node contributes exactly one aggregate product: the left
//! half's summed parked masses times the right half's summed position windows.
//! Parked sums fold digit-wise, so opposing armings cancel inside the sum
//! before any product reads a width. Window sums are held as sparse balanced
//! signed digits, so an all-ones run — a long climb's consumed mass — compacts
//! to O(1) terms. Each node's product is delegated cluster-wise to the
//! backend's sub-quadratic integer multiplication.
//!
//! Every arming-window cross term of the debt rides exactly one aggregate
//! product, and no entry is re-read more times than its tree depth. The mass
//! balance keeps that depth logarithmic in the ledger's total settle mass
//! (parked digits plus window density) — not in the entry count alone:
//! exponentially spread masses chain one isolating split per level (the
//! committed split-depth witness in the [`query`](super) module's tests), still
//! within twice the total-mass logarithm ([`mass_split`] derives the constant
//! and states the pinned bound). Every unit of settle mass is a digit the
//! input funded, so the depth is `O(log |v|)`.
//!
//! # Funding: the potential function and its arity
//!
//! The certificate is a **two-ledger potential, one ledger per operand**:
//! `Φ = Φ_a + Φ_b`, where folding a code of `w` digits from operand `s`
//! deposits `Θ(w)` into `Φ_s`, and each topology bit deposits O(1). The arity
//! is the point: distance and lag are two-stream operations, and a per-stream
//! potential argument is sound only if no charge draws on the ledger of an
//! operand that did not deposit — the hole the composed form fell into, where
//! the meet's emission re-coded one operand's width into switch jumps that
//! the integral then evicted at the other operand's cheap codes, priced by a
//! position density neither had funded. The rank fold is the one-ledger,
//! single-stream instance of the same integral (its orientation is constantly
//! `+1`), so its certificate is this one with `Φ_b` empty.
//!
//! ## The charge inventory
//!
//! Every charge names its deposit:
//!
//! - folds into `D` and `L`, and the orientation-change read of `D′`:
//!   this boundary's own deposits (`|D′| ≤ |dD|` caps the read);
//! - the interval add of `L`: the deposit that last set `L`'s width —
//!   at most one interval rides between trigger checks;
//! - a settle `P · segment`: `P`'s width is within the allowance of
//!   the drift the settling freeze parks (else promotion fires first),
//!   so the product draws from the deposits that built that drift,
//!   times a segment span the segment's own topology deposits cover;
//! - a promotion's ledger entry: the parked read from the wide deposit
//!   that armed `P` past the allowance, the window read from the
//!   watermark span the banked segments' topology deposits cover —
//!   both once per arming, no product;
//! - the ledger settle: one aggregate product per tree node — the left
//!   half's parked sum times the right half's window sum, delegated
//!   cluster-wise to the backend's multiplication at its bound over
//!   (parked width, cluster span). Its funding splits three ways:
//!   - the width side: the deposits that armed the parked sum;
//!   - the span side: the position space — each digit position of a
//!     window is a depth the stream's topology paid at least one bit
//!     for;
//!   - the rewrites: every window's digits are rewritten at most once
//!     per tree level, each rewrite paid by the window's own read, and
//!     the mass balance keeps the level count logarithmic in the total
//!     settle mass, hence `O(log |v|)`.
//!
//! A cheap code from one operand can *fire* a freeze, but the work the freeze
//! performs is bounded by deposits from the codes that built the state being
//! moved — never by an absolute position the firing operand chose.
//!
//! ## The settle's bound
//!
//! The product tree charges every arming-window cross term of the exact debt
//! `Σ_{i<j} P_i · w_j` inside exactly one aggregate product, so no accounting
//! direction exists for an input to load — the two one-sided settle orders'
//! defects are both closed: a shared dense suffix cannot be re-walked once per
//! arming, a promoted prefix cannot be re-read once per window, and no width or
//! density is re-read more times than its node's depth.
//!
//! Streams whose parked masses stay `O(1)` digits wide — every committed board
//! family, and the dense-suffix adversaries of the `skyline_flatness`
//! dense-suffix bands (a gap spine whose turns puncture the trailing mass a
//! full digit apart, over `Θ(p)` re-arm blocks) — therefore settle in `O((n +
//! D) log n)` digit work over `n` armings, `D` the total window density, and
//! measure flat per byte. The `log n` is conditioned on exactly that
//! `O(1)`-wide premise, and it holds through the entropy bound, not through any
//! per-entry depth cap: a leaf of mass `m` sits at depth `O(log(total/m))`, so
//! heavy windows sit shallow and the mass-weighted traffic stays under the
//! total mass times the entry-count logarithm however unevenly the window
//! masses spread.
//!
//! The wide × dense settle products themselves ride one backend multiplication
//! per cluster, so each costs the multiplication bound `M` over its funded
//! factors instead of their schoolbook product. Two shapes reach them: one
//! arming as wide as the input ahead of a trailing mass as dense (the
//! wide-arming family), and the close-time `P · segment` settle the same shape
//! reaches with the ledger never armed (the plateau-puncture family, whose
//! exact rank numerator *is* the plateau times the punctured turn mass).
//!
//! Two facts make the whole settle `O(M(|v|))` under every power-law tier of
//! the backend's multiplication. Cluster splitting keeps every densified span
//! funded: gaps wider than the factor split, so separated products total
//! `O(span)` traffic, and bridged gaps cost less than the product a split would
//! add. And the mass balance makes node products shrink geometrically down the
//! tree, telescoping their costs into the root's.
//!
//! The shipped backend dispatches power-law tiers up to 4,000-word operand
//! sides (~32 KiB parked sums per side). Past that its quasilinear tier's
//! per-level costs stop telescoping, and the settle pays at most one extra
//! tree-depth factor, `O(M(|v|) · log |v|)` — and the log factor is tight there
//! [derived; a committed witness at this scale would need 65 KiB+ packed
//! operands]: `Θ(log |v|)` armings whose parked widths grow as `4,000 · 2^i`
//! words, each banked ahead of a trailing window span `Θ(|v|)`, keep `Θ(log
//! |v|)` tree levels' products in the quasilinear tier at `Θ(M(|v|))` each,
//! fully funded — the public worst case cannot tighten without a deeper
//! mechanism change.
//!
//! ## The multiplication floor
//!
//! The floor is matched by a reduction from arbitrary integer multiplication:
//! the same answer-embedding shape carries *arbitrary* factors. The
//! puncture-product construction stores `Θ(bits(x) + bits(y))` bits and its
//! exact rank numerator is `2·x·y + 1` (the committed proptest constructs it
//! for arbitrary positive factors and pins the stored size linear in their
//! widths, so the floor's denominator cannot drift). A fold that answers
//! exactly therefore multiplies two input-funded integers at linear overhead,
//! and `Ω(M(|v|))` digit work is mandatory for any fold that answers exactly:
//! no settle goes below the multiplication bound.
//!
//! The public API's `# Complexity` sections
//! ([`Version::rank`](crate::Version::rank),
//! [`Version::distance`](crate::Version::distance),
//! [`Version::lag`](crate::Version::lag) — one shared integrator) state the
//! resulting three-part claim. The `ledger_wide_arming` and
//! `answer_embedded_product` bands (`tests/meter.rs`) hold both wide × dense
//! families flat per byte in the deterministic counters (which price the fold's
//! own traffic; the multiplication runs inside the backend, below the limb
//! shim), and the committed schoolbook kernel beside the [`query`](super)
//! module's tests (`schoolbook_settle_reads_superlinear_on_wide_arming` and its
//! plateau-puncture twin) keeps the per-digit charge failing on both families,
//! so the bands are never decoration.

use core::cmp::Ordering;

use suanpan::{Accumulator, Limbs, UBig};

use crate::codec::{Base, Int};

use super::super::signed::{fold_signed, fold_signed_int, Sign};

/// The live accumulator's tolerated width overshoot, in base-2^32 digits, over
/// the just-folded delta's own width: a fold that leaves `L` wider than its
/// delta by more than this freezes the height split.
///
/// Relative to the delta, so bounded oscillation never freezes at any width — a
/// tooth's fold is paid by the tooth's own code — while stale drift under
/// cheaper codes is evicted at the first such code. 8 digits (256 bits) of
/// slack: reaching it from the codes' own widths would take more small folds
/// than any real stream holds, and it caps how far a per-leaf `L` add can
/// outgrow the code that last set `L`'s width.
pub(super) const FREEZE_ALLOWANCE_DIGITS: usize = 8;

#[cfg(test)]
thread_local! {
    /// Freezes parked on the current thread, counted at
    /// [`Integrator::freeze`]'s drift-parking arm: the liveness tap for the
    /// sibling tests' freeze-regime witnesses.
    ///
    /// A witness pinned "in the freeze regime" must prove its inputs actually
    /// park drift — a value pin whose input never freezes passes vacuously.
    pub(super) static FREEZE_HITS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
}

/// The settle tree's split rule: the first boundary at or past the mass
/// midpoint of `prefix[lo..=hi]`, clamped so both halves are nonempty.
///
/// `prefix` holds the running leaf-mass sums (`prefix[i]` is the mass before
/// leaf `i`, each leaf's mass floored at one), and `lo < hi − 1` names an
/// internal node's range; the returned `mid` satisfies `lo < mid < hi`. Each
/// half's mass stays within half the node's plus one leaf's: the right half is
/// at most half the node's mass, and the left half exceeds half only by mass
/// its own *last* leaf carries — the straddling leaf, which the left half's
/// next split pushes into a right half of its own. So along any root-to-leaf
/// path the mass at least halves every second level, and the deepest leaf sits
/// within `2·log₂(total mass) + 2` levels — the entropy denomination the
/// module doc's settle bound rests on, pinned size-generically (with the
/// nonempty-halves contract) by the sibling tests' split-rule family.
pub(super) fn mass_split(prefix: &[u64], lo: usize, hi: usize) -> usize {
    let target = (prefix[lo] + prefix[hi]).div_ceil(2);
    (lo + 1 + prefix[lo + 1..hi].partition_point(|&p| p < target)).min(hi - 1)
}

/// [`base_digits`] over the decoded-payload value form.
pub(super) fn int_digits(value: &Int) -> usize {
    match value {
        Int::Small(word) => {
            let digits = usize::try_from((u64::BITS - word.leading_zeros()).div_ceil(32))
                .expect("digit counts fit usize");
            digits.max(1)
        }
        Int::Wide(base) => base_digits(base),
    }
}

/// A stored magnitude's width in base-2^32 digits (minimum 1).
pub(super) fn base_digits(value: &Base) -> usize {
    let digits = usize::try_from(value.bits().div_ceil(32)).expect("digit counts fit usize");
    digits.max(1)
}

/// Split an ascending balanced-digit run into clusters whose interior zero gaps
/// never exceed `gap_limit` digit positions.
///
/// The cluster seam of the settle products: within a cluster the digits densify
/// into one integer for the backend's multiplication, across a split the
/// products stay separate. The threshold is a parameter so a caller can gate
/// the split point — the settle passes the factor's own width
/// ([`charge_digits`] derives why) — and the iterator borrows the run, so
/// clustering allocates nothing.
pub(super) fn clusters(
    digits: &[(u64, i64)],
    gap_limit: u64,
) -> impl Iterator<Item = &[(u64, i64)]> + '_ {
    let mut rest = digits;
    core::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let mut end = 1;
        while end < rest.len() && rest[end].0 - rest[end - 1].0 - 1 <= gap_limit {
            end += 1;
        }
        let (head, tail) = rest.split_at(end);
        rest = tail;
        Some(head)
    })
}

/// Record a settle product's limb-scale traffic: both operands and the
/// materialized product.
///
/// The multiplication itself is delegated whole to the backend, below the limb
/// shim — the `parse_decimal` convention — so the counters price the traffic
/// the fold moves (operand reads, the product's width) and stay linear when the
/// mechanism is honest: a settle that multiplied too often, or densified across
/// an unfunded gap, would push this very tap superlinear. The backend's
/// internal cost per product is its multiplication bound, which the public `#
/// Complexity` claims carry. Compiles to nothing without the `limb-meter`
/// feature. The tap's own liveness is pinned: the sibling tests' seam-window
/// floor (`settle_product_tap_is_alive_on_the_wide_arming_close`) holds the
/// recording to a per-boundary mechanism minimum, so a dark tap fails there
/// instead of letting every limb ceiling it feeds pass vacuously.
#[inline(always)]
fn meter_product(factor: &UBig, part: &UBig, product: &UBig) {
    #[cfg(feature = "limb-meter")]
    {
        crate::codec::limb_meter::record_wide(factor);
        crate::codec::limb_meter::record_wide(part);
        crate::codec::limb_meter::record_wide(product);
    }
    #[cfg(not(feature = "limb-meter"))]
    let _ = (factor, part, product);
}

/// Record `count` window digits' worth of merge work into the limb meter.
///
/// The window masses move digits as plain `i64` vector traffic — no `Base`
/// operation, no accumulator write — so without this tap a settle could re-walk
/// window digits arbitrarily often while every committed counter read zero: the
/// digit walk is width-scale work exactly as a `Base` operand walk is, and it
/// meters at the same one-count-per-digit rate. Compiles to nothing without the
/// `limb-meter` feature, so [`WindowMass::combine`] calls it unconditionally —
/// wherever the meters are, the tap is on by construction.
#[inline(always)]
fn meter_window_digits(count: u64) {
    #[cfg(feature = "limb-meter")]
    crate::codec::limb_meter::record(count);
    #[cfg(not(feature = "limb-meter"))]
    let _ = count;
}

/// Record the zero-filled capacity of a cluster's two densified byte images,
/// in base-2^32 digits: [`charge_digits`]' zero-fill tap.
///
/// The images are zero-filled at the cluster's span before any live digit
/// lands, and that fill is real width-scale work no other counter reads: a
/// zeroed byte no digit lands on enters no operand width [`meter_product`]
/// records, touches no accumulator digit, and raises no peak while the image
/// stays under the walk's own high-water mark. Without this tap a
/// densification could size its images by anything — the cluster's absolute
/// digit position included, O(position) fill per cluster — while every
/// committed counter read unchanged; with it, the images' own lengths are
/// the recorded quantity, so the count moves with the allocation itself (the
/// sibling tests' span pin, `densify_tap_prices_the_cluster_span`, holds the
/// rate to exactly two spans per multi-digit cluster). The fill is memory
/// work, not `Base` arithmetic, so it feeds its own meter column
/// (`crate::meter::densified_digits`) instead of a share of the limb count.
/// Compiles to nothing without the `limb-meter` feature, and
/// [`charge_digits`] calls it unconditionally — wherever the meters are, the
/// tap is on by construction.
#[inline(always)]
fn meter_densified_image(bytes: u64) {
    #[cfg(feature = "limb-meter")]
    crate::codec::limb_meter::record_densified(bytes / 4);
    #[cfg(not(feature = "limb-meter"))]
    let _ = bytes;
}

/// Debit (or, with a negative `sign`, credit) `factor × segment · 2^shift`
/// into the total: the segment settle move of [`charge_digits`].
///
/// The segment mass compacts into balanced signed digits first (the
/// [`WindowMass`] spelling, one metered pass over the read-out span), so an
/// all-ones run — a long climb's consumed mass — collapses to two far-apart
/// digits and splits into two word-scale products instead of densifying its
/// whole span, and the punctured dense runs that remain ride the backend's
/// multiplication cluster-wise.
fn charge_segment(total: &mut Accumulator, sign: Sign, factor: &Base, segment: &UBig, shift: u64) {
    let mut mass = WindowMass::new();
    mass.merge(segment, shift);
    mass.charge(total, sign, factor);
}

/// Debit (or, with a negative `sign`, credit) `factor × Σ digits ·
/// 2^(32·index)` into the total, cluster-wise through the backend's
/// sub-quadratic multiplication.
///
/// `digits` is an ascending balanced signed-digit run (a [`WindowMass`]'s
/// spelling). Each cluster of digits whose interior gaps stay within the
/// factor's own width densifies into at most two magnitudes (the positive and
/// negative digits separately, so no signed subtraction precedes the product)
/// and rides one backend multiplication each — the densification zero-fills
/// two images at the cluster's span, priced by their own meter column
/// ([`meter_densified_image`]); a single-digit cluster takes the one-word
/// product directly. The gap threshold is the factor's width because
/// that is where bridging stops paying: a zero run narrower than the factor
/// costs less to carry through the multiplication than the extra full-width
/// product a split would add, while a wider run would do work no code funded —
/// and splitting there caps the cluster count at the position span over the
/// factor width, so the separated products total O(span) digit traffic. Cost,
/// in digit work: one multiplication per cluster at the backend's bound over
/// (factor width, cluster span), with every span funded by the position space
/// the stream's own topology paid for — the module doc's settle bound builds on
/// exactly this shape.
pub(super) fn charge_digits(
    total: &mut Accumulator,
    sign: Sign,
    factor: &Base,
    digits: &[(u64, i64)],
) {
    // Total with no identity fast path: an empty digit run yields no
    // clusters, so the loop is the no-op it should be, and both callers
    // (the segment settles and the aggregate merges) already skip
    // zero-valued factors at the sign reads that price them. A guard here
    // would itself be metered width-scale work: `Base` equality records its
    // operands' limbs, so a per-charge zero test taxes every settle by the
    // factor's width.
    let gap_limit = base_digits(factor) as u64;
    for cluster in clusters(digits, gap_limit) {
        if let [(index, digit)] = *cluster {
            // The single-digit cluster: one word-scale product, no
            // densified image.
            let mut product = factor.clone();
            product *= u32::try_from(digit.unsigned_abs()).expect("balanced digits fit 32 bits");
            if sign.is_negative() == (digit < 0) {
                total.add_magnitude_shl(&product, 32 * index);
            } else {
                total.sub_magnitude_shl(&product, 32 * index);
            }
            continue;
        }
        let floor_index = cluster[0].0;
        let span = usize::try_from(cluster[cluster.len() - 1].0 - floor_index + 1)
            .expect("cluster spans are bounded by the stream's depth");
        // Densify the cluster into little-endian byte images, positive and
        // negative digits separately: balanced digits carry signs, and two
        // nonnegative products need no borrow machinery. The zero fill is
        // span-scale work no width or touch counter sees, so the tap prices
        // the images by their own lengths.
        let mut parts = [(vec![0u8; span * 4], false), (vec![0u8; span * 4], false)];
        meter_densified_image((parts[0].0.len() + parts[1].0.len()) as u64);
        for &(index, digit) in cluster {
            debug_assert!(
                digit != 0 && digit.unsigned_abs() <= 1 << 31,
                "window digits are nonzero and balanced"
            );
            let offset = usize::try_from(index - floor_index).expect("inside the cluster span") * 4;
            let (image, live) = &mut parts[usize::from(digit < 0)];
            image[offset..offset + 4].copy_from_slice(&(digit.unsigned_abs() as u32).to_le_bytes());
            *live = true;
        }
        for (side, (image, live)) in parts.iter().enumerate() {
            if !live {
                continue;
            }
            let part = UBig::from_le_bytes(image);
            let product = &factor.0 * &part;
            meter_product(&factor.0, &part, &product);
            if sign.is_negative() == (side == 1) {
                total.add_wide_shl(&product, 32 * floor_index);
            } else {
                total.sub_wide_shl(&product, 32 * floor_index);
            }
        }
    }
}

/// The anchored-segment integral of the co-sweep's integrand `h* = B + P + L`.
///
/// Nonnegative for the directed measures, signed for the rank order, every
/// component accumulator signed throughout (the module doc derives the split
/// and certifies its funding).
pub(super) struct Integrator {
    /// The running integral's raw numerator, at the overlay scale.
    pub(super) total: Accumulator,
    /// `L`: the integrand's drift since the last freeze. Written by the sweep's
    /// folds directly; every other component is this integrator's own
    /// bookkeeping.
    pub(super) live: Accumulator,
    /// `P`: drift parked by freezes, anchored at the last freeze.
    pub(super) parked: Accumulator,
    /// The interval mass accumulated since `parked`'s anchor.
    ///
    /// Fed only while [`frozen`](Self::frozen) holds: every consumer of segment
    /// mass settles drift some freeze parked, so a sweep that never freezes —
    /// every practical-regime input — deposits nothing here and pays nothing
    /// per interval for the settle machinery's existence.
    pub(super) segment_mass: Accumulator,
    /// Whether any freeze has parked drift: the gate on the segment and window
    /// feeds.
    ///
    /// Until the first freeze, no segment mass can ever be read against a
    /// parked width — the pre-freeze settle has no parked mass to cover, and
    /// the mass banked behind the first freeze would feed only the first
    /// promotion's window, which no arming precedes, so the ledger settle
    /// multiplies it into nothing (a window is charged exactly by the parked
    /// sums of entries before it, and the first entry is the reduction's
    /// leftmost leaf in every node that contains it). Skipping the feed until
    /// the gate opens is therefore value-identical on every input; it removes
    /// the one per-interval deposit — and the segment buffer's scale-deep
    /// growth — that benign sweeps paid without ever reading.
    frozen: bool,
    /// `B`: the opening plateau, anchored at position zero and closing as `B ·
    /// 2^S`.
    pub(super) base: Accumulator,
    /// The interval mass banked since the last promotion (or the sweep's
    /// start): the window the next promotion records.
    ///
    /// Fed one compacted segment mass per freeze — the same watermark read the
    /// settle already pays — never per interval, and read once per promotion,
    /// so a sweep that never freezes never touches it and a sweep that never
    /// promotes never reads it.
    pub(super) banked_window: Accumulator,
    /// The promotion ledger: one [`Arming`] per promotion, settled once at the
    /// sweep's close ([`settle_armings`](Self::settle_armings)).
    pub(super) promotions: Vec<Arming>,
    /// The unit mass every interval deposits at its own scale; a constant,
    /// held on the struct so the per-interval deposit borrows a ready
    /// `&Base` instead of building one per interval.
    one: Base,
}

/// One promotion, recorded at its freeze and settled once at the sweep's close:
/// the promoted parked component and the window of interval mass that separates
/// it from the previous promotion.
pub(super) struct Arming {
    /// The promoted parked component's sign.
    pub(super) sign: Sign,
    /// The promoted parked component, read once at its promotion.
    pub(super) parked: Base,
    /// The interval mass banked between the previous promotion (or the sweep's
    /// start) and this one: `window · 2^shift`, the watermark read of the
    /// position window, `shift` a multiple of 32.
    pub(super) window: UBig,
    /// The window's power-of-two scale (its never-written low prefix).
    pub(super) shift: u64,
}

/// A run of the sweep's interval mass as sparse balanced signed digits: the
/// window side of a settle [`Aggregate`].
///
/// Each entry is `(digit index, digit)` with `0 < |digit| ≤ 2^31`, ascending;
/// the denoted mass is `Σ digit · 2^(32·index)`. The balanced form is the
/// point: an all-ones run — the shape a long climb's consumed masses sum to —
/// compacts to O(1) entries, so a charge against the mass costs the parked
/// width times the mass's *density*, never its span, and a merge rewrites only
/// live entries.
pub(super) struct WindowMass {
    pub(super) digits: Vec<(u64, i64)>,
}

impl WindowMass {
    /// The empty mass.
    pub(super) fn new() -> WindowMass {
        WindowMass { digits: Vec::new() }
    }

    /// Add `mass · 2^shift` into this mass, re-balancing the digits it lands
    /// on: one pass over the live entries plus the operand's digits.
    ///
    /// Each window enters the settle through exactly one such merge — its
    /// 1-entry aggregate's — so the operand walk is paid by the watermark read
    /// that produced it.
    pub(super) fn merge(&mut self, mass: &UBig, shift: u64) {
        debug_assert_eq!(shift % 32, 0, "interval masses are digit-aligned");
        let start_index = shift / 32;
        let new = Limbs::new(mass)
            .enumerate()
            .flat_map(|(limb_index, limb)| {
                [
                    (
                        start_index + 2 * limb_index as u64,
                        (limb & 0xFFFF_FFFF) as i64,
                    ),
                    (start_index + 2 * limb_index as u64 + 1, (limb >> 32) as i64),
                ]
            })
            .filter(|&(_, digit)| digit != 0);
        self.combine(new);
    }

    /// Fold another balanced mass into this one, re-balancing: one pass over
    /// both operands' live entries.
    ///
    /// The product-tree merge: a mass's digits are rewritten once per tree
    /// level they survive, which is what bounds every window's digits to one
    /// rewrite per tree level — a depth the mass balance keeps logarithmic in
    /// the total settle mass, hence `O(log |v|)` — across the whole
    /// settle.
    pub(super) fn absorb(&mut self, other: WindowMass) {
        self.combine(other.digits.into_iter());
    }

    /// The shared re-balancing merge loop over an ascending sparse digit
    /// stream.
    ///
    /// Incoming digits may exceed the balanced range: [`merge`](Self::merge)
    /// feeds raw `u32` limb halves (up to `2^32 − 1`) and
    /// [`absorb`](Self::absorb) feeds balanced digits, so a position's sum of
    /// carry, live, and incoming digit stays under `2^33` — far inside `i64` —
    /// and the recentering below restores every output digit to the balanced
    /// range. Every merged position records one limb-meter count
    /// ([`meter_window_digits`]), the digit traffic's only meter.
    pub(super) fn combine<I: Iterator<Item = (u64, i64)>>(&mut self, new: I) {
        let mut old = core::mem::take(&mut self.digits).into_iter().peekable();
        let mut new = new.peekable();
        let mut out: Vec<(u64, i64)> = Vec::new();
        let mut carry: i64 = 0;
        let mut carry_index: u64 = 0;
        loop {
            let mut index = u64::MAX;
            if carry != 0 {
                index = carry_index;
            }
            if let Some(&(next_index, _)) = old.peek() {
                index = index.min(next_index);
            }
            if let Some(&(next_index, _)) = new.peek() {
                index = index.min(next_index);
            }
            if index == u64::MAX {
                break;
            }
            meter_window_digits(1);
            let mut sum: i64 = 0;
            if carry != 0 {
                // A pending carry is consumed at the very next merged
                // position: both operand streams ascend strictly past the
                // position that produced it, so the loop head's min lands
                // exactly on its index.
                debug_assert_eq!(
                    carry_index, index,
                    "a pending carry is consumed at its own index"
                );
                sum = carry;
                carry = 0;
            }
            if let Some((_, digit)) = old.next_if(|&(entry_index, _)| entry_index == index) {
                sum += digit;
            }
            if let Some((_, digit)) = new.next_if(|&(entry_index, _)| entry_index == index) {
                sum += digit;
            }
            // Recenter into the balanced range [−2^31, 2^31): an all-ones run
            // becomes one subtract at its floor and one carry past its top,
            // exactly the compaction the charge relies on.
            let carry_out = (sum + (1 << 31)) >> 32;
            let rem = sum - (carry_out << 32);
            if rem != 0 {
                out.push((index, rem));
            }
            if carry_out != 0 {
                debug_assert_eq!(carry, 0, "positions advance, so the carry was consumed");
                carry = carry_out;
                carry_index = index + 1;
            }
        }
        self.digits = out;
    }

    /// Debit (or, for a negative `parked`, credit) `parked × mass` into the
    /// total, cluster-wise ([`charge_digits`]).
    ///
    /// One backend multiplication per dense cluster of the mass's live digits,
    /// so the charge runs at the multiplication bound instead of the parked
    /// width times the mass's balanced density.
    pub(super) fn charge(&self, total: &mut Accumulator, sign: Sign, parked: &Base) {
        charge_digits(total, sign, parked, &self.digits);
    }
}

/// One node's worth of the mass-balanced product-tree settle: a contiguous run
/// of ledger entries, reduced to the signed sum of its parked components and
/// the balanced sum of its position windows.
///
/// The parked side lives on an [`Accumulator`] so opposing armings cancel
/// digit-wise inside the sum before any product reads a width; the window side
/// is a [`WindowMass`] so adjacent windows — contiguous interval runs — compact
/// as they combine.
pub(super) struct Aggregate {
    /// The signed sum of the run's parked components.
    pub(super) parked: Accumulator,
    /// The balanced sum of the run's position windows.
    pub(super) windows: WindowMass,
}

impl Aggregate {
    /// Charge this aggregate's parked sum against `mass`, then combine: exactly
    /// one product-tree node.
    ///
    /// `self` is the left (older) half and `right` the newer, so the node's
    /// product `(Σ parked_left) × (Σ windows_right)` covers every arming-window
    /// cross pair split by this node's seam — and no other node covers any of
    /// them, which is what makes the settle exact.
    fn merge(&mut self, right: Aggregate, total: &mut Accumulator) {
        let (parked_sign, parked_magnitude) = self.parked.sign_magnitude();
        if parked_magnitude != UBig::ZERO {
            right.windows.charge(
                total,
                Sign::from_is_negative(parked_sign == Ordering::Less),
                &Base::from(parked_magnitude),
            );
        }
        self.parked.add_accum(&right.parked);
        self.windows.absorb(right.windows);
    }
}

impl Integrator {
    pub(super) fn new() -> Integrator {
        Integrator {
            total: Accumulator::new(),
            live: Accumulator::new(),
            parked: Accumulator::new(),
            segment_mass: Accumulator::new(),
            frozen: false,
            base: Accumulator::new(),
            banked_window: Accumulator::new(),
            promotions: Vec::new(),
            one: Base::from(1u8),
        }
    }

    /// Anchor the opening plateau at position zero (signed: the signed pair
    /// measure's integrand carries `D`'s own sign, every other caller opens
    /// nonnegative).
    pub(super) fn open(&mut self, sign: Sign, opening: &Int) {
        fold_signed_int(&mut self.base, sign, opening);
    }

    /// Credit one elementary interval: the live component's contribution at the
    /// interval's mass, and — once a freeze has parked drift — the mass itself
    /// into the segment sum.
    pub(super) fn interval(&mut self, weight_shift: u64) {
        // The zero test is one-sided (true means zero, false means unknown),
        // which is all this skip needs: a redundantly spelled zero takes the
        // add and contributes nothing.
        if !self.live.is_literally_zero() {
            self.total.add_accum_shl(&self.live, weight_shift);
        }
        // Segment mass exists to settle parked drift, so the feed waits for the
        // gate ([`frozen`](Self::frozen) derives why the mass behind the first
        // freeze funds nothing).
        if self.frozen {
            self.segment_mass.add_magnitude_shl(&self.one, weight_shift);
        }
    }

    /// Fold the orientation-change term `(σ′ − σ) · D′` into the live
    /// component.
    ///
    /// Called only when the orientation moved at this boundary, which bounds
    /// `|D′|` by the deltas the boundary folded; the sign read that decided the
    /// new orientation has already collapsed the difference's spelling, so the
    /// read is priced by those same codes.
    ///
    /// The term is always a debit. The orientation contract (the σ table's
    /// monotonicity, [`pair_fold`](super::pair_fold)'s closure contract)
    /// derives it: σ non-decreasing in `sign(D)` means a nonzero coefficient
    /// `σ′ − σ` agrees in sign with the move `sign(D′) − sign(D)`, and a
    /// changed sign whose new value is nonzero agrees with `D′` itself — a
    /// zero `D′` returned above — so `(σ′ − σ) · D′ > 0` on every fold that
    /// reaches the add.
    pub(super) fn jump(&mut self, coefficient: i8, diff: &Accumulator) {
        let (sign, magnitude) = diff.sign_magnitude();
        if magnitude == UBig::ZERO {
            return;
        }
        let magnitude = Base::from(magnitude);
        // A hard assert, not a debug one: a non-monotone closure would
        // otherwise fold the term in the wrong direction silently, and the
        // check is one word compare per orientation change.
        assert_eq!(
            coefficient < 0,
            sign == Ordering::Less,
            "a monotone orientation's change term is a debit"
        );
        let shift = if coefficient.abs() == 2 { 1 } else { 0 };
        self.live.add_magnitude_shl(&magnitude, shift);
    }

    /// The end-of-boundary trigger: park the live drift when this boundary's
    /// folds left it more than the allowance wider than the widest code folded
    /// here.
    pub(super) fn boundary(&mut self, funded_digits: usize) {
        if self.live.digit_count() > funded_digits + FREEZE_ALLOWANCE_DIGITS {
            self.freeze();
        }
    }

    /// Park the live drift, closing the current segment.
    ///
    /// Settles the parked component over the segment (banking the segment's
    /// mass into the position window), promotes the parked component first if
    /// the incoming drift runs far narrower, then moves the drift in and
    /// re-anchors.
    fn freeze(&mut self) {
        let (drift_sign, drift) = self.live.sign_magnitude();
        if drift == UBig::ZERO {
            // A redundantly spelled zero tripped the width trigger: there is no
            // drift to park — empty the spelling and keep the current segment
            // open.
            self.live.reset();
            return;
        }
        let drift = Base::from(drift);
        // Open the gate on the segment and window feeds: from here on, parked
        // drift exists for segment mass to settle. At this first opening the
        // segment sum is empty — the mass behind it funds nothing
        // ([`frozen`](Self::frozen)) — so the settle below banks and charges
        // only from the second freeze onward.
        self.frozen = true;
        #[cfg(test)]
        FREEZE_HITS.with(|hits| hits.set(hits.get() + 1));
        // Segment pricing first, with the segment mass untouched by any
        // sign read (see settle_segment: a collapsing sign read could
        // lower the scaled read's shift).
        self.settle_segment();
        if self.parked.digit_count() > base_digits(&drift) + FREEZE_ALLOWANCE_DIGITS {
            self.promote();
        }
        match drift_sign {
            Ordering::Less => self.parked.sub_magnitude(&drift),
            _ => self.parked.add_magnitude(&drift),
        }
        self.live.reset();
        // A fresh buffer, not `reset()`: the segment's digits sit at the sweep
        // position's scale, and a clearing scan would pay the untouched zero
        // prefix below them; replacing the buffer opens the next segment in
        // O(1).
        self.segment_mass = Accumulator::new();
    }

    /// Close the current segment at a freeze: credit the parked component over
    /// it and bank the segment's mass.
    ///
    /// The credit is `total += P · segment`, as [`settle`](Self::settle); the
    /// banked mass joins the position window the next promotion records
    /// ([`banked_window`](Self::banked_window)) — one watermark read serving
    /// both consumers, priced by the segment's depth variation.
    fn settle_segment(&mut self) {
        // The segment mass is priced with no prior sign read: sign queries
        // count as writers to the accumulator's write watermark, so a
        // collapsing sign read can lower the returned shift and surrender
        // part of the never-written-prefix skip this pricing rests on —
        // suanpan's witness `collapsing_sign_read_lowers_the_scaled_read_shift`.
        let (segment_sign, segment_magnitude, segment_shift) =
            self.segment_mass.sign_magnitude_shl();
        debug_assert_ne!(
            segment_sign,
            Ordering::Less,
            "interval masses only accumulate"
        );
        if segment_magnitude == UBig::ZERO {
            return;
        }
        let segment = Base::from(segment_magnitude);
        self.banked_window
            .add_magnitude_shl(&segment, segment_shift);
        if self.parked.is_literally_zero() {
            return;
        }
        let (parked_sign, parked_magnitude) = self.parked.sign_magnitude();
        if parked_magnitude == UBig::ZERO {
            return;
        }
        charge_segment(
            &mut self.total,
            Sign::from_is_negative(parked_sign == Ordering::Less),
            &Base::from(parked_magnitude),
            &segment.0,
            segment_shift,
        );
    }

    /// Credit the parked component over the final segment at the sweep's close:
    /// `total += P · segment`.
    ///
    /// One clustered product ([`charge_segment`]) priced at the multiplication
    /// bound over `P`'s width and the segment's depth variation; the scaled
    /// read skips the never-written scale prefix under the segment. No banking
    /// here: only a promoting sweep needs the final window, and
    /// [`finish`](Self::finish) banks it exactly there.
    pub(super) fn settle(&mut self) {
        if self.parked.is_literally_zero() {
            return;
        }
        let (parked_sign, parked_magnitude) = self.parked.sign_magnitude();
        if parked_magnitude == UBig::ZERO {
            return;
        }
        // No sign read precedes this scaled read either, for the reason
        // stated at settle_segment: a collapsing sign read could lower the
        // shift and surrender the never-written-prefix skip (suanpan's
        // `collapsing_sign_read_lowers_the_scaled_read_shift`).
        let (segment_sign, segment_magnitude, segment_shift) =
            self.segment_mass.sign_magnitude_shl();
        debug_assert_ne!(
            segment_sign,
            Ordering::Less,
            "interval masses only accumulate"
        );
        charge_segment(
            &mut self.total,
            Sign::from_is_negative(parked_sign == Ordering::Less),
            &Base::from(parked_magnitude),
            &segment_magnitude,
            segment_shift,
        );
    }

    /// Promote the parked component out of the per-freeze settle: record it in
    /// the ledger with the position window it closes, and re-open both.
    ///
    /// The entry owes `P · (2^S − position)`, settled once at the sweep's close
    /// through the balanced product tree
    /// ([`settle_armings`](Self::settle_armings)).
    ///
    /// Sound only immediately after [`settle_segment`](Self::settle_segment):
    /// the segment credit covered `P` up to the current position — which the
    /// banking has just brought current — so its remaining tail is exactly the
    /// interval mass still ahead of this freeze. Both reads here are at funded
    /// widths: the parked read at the width its arming deposited, the window
    /// read at the watermark span the banked segments paid for; nothing is
    /// re-based against an absolute position.
    fn promote(&mut self) {
        let (parked_sign, parked_magnitude) = self.parked.sign_magnitude();
        if parked_magnitude != UBig::ZERO {
            let (window_sign, window_magnitude, window_shift) =
                self.banked_window.sign_magnitude_shl();
            debug_assert_eq!(
                window_sign,
                Ordering::Greater,
                "a freeze always follows at least one interval"
            );
            self.promotions.push(Arming {
                sign: Sign::from_is_negative(parked_sign == Ordering::Less),
                parked: Base::from(parked_magnitude),
                window: window_magnitude,
                shift: window_shift,
            });
            // A fresh buffer, not `reset()`: the window's digits sit at the
            // sweep position's scale, and a clearing scan would pay the
            // untouched zero prefix below them.
            self.banked_window = Accumulator::new();
        }
        self.parked.reset();
    }

    /// Settle the promotion ledger at the sweep's close: one mass-balanced
    /// product-tree reduction over the entry sequence, every cross term `P_i ·
    /// w_j` (`i < j`) riding exactly one aggregate product.
    ///
    /// The entry sequence is the armings in sweep order — entry `i` pairs `P_i`
    /// with the window *behind* it, the mass banked between the previous
    /// promotion and its own — closed by one virtual entry carrying no parked
    /// mass and the final window (the mass banked since the last promotion,
    /// which [`finish`](Self::finish) completed with the final segment). Entry
    /// `i`'s ledger debt `P_i · (2^S − position_i)` is then exactly `Σ_{j>i}
    /// P_i · w_j`, and the reduction computes the double sum as one aggregate
    /// product per merge ([`Aggregate::merge`]): `(Σ parked of the left half) ×
    /// (Σ windows of the right half)`, each cross pair covered by the one node
    /// whose seam splits it. No per-arming walk of the suffix and no per-window
    /// read of a promoted prefix exists for an input to load: a window's digits
    /// are rewritten once per tree level, and a parked width is read once per
    /// node where it is the left half's widest.
    ///
    /// The tree balances by **mass** (parked digits plus window density), not
    /// by entry count: the whole ledger is in hand at the close, so each node
    /// splits its run at the mass midpoint ([`mass_split`] states the split's
    /// contract), node masses shrink geometrically down the tree, and the
    /// per-node backend products telescope into the top node's under any
    /// power-law multiplication tier — where an entry-count
    /// split would let one wide arming meet an equal share of window mass at
    /// every level and stack a polylog on top of the multiplication bound (the
    /// module doc's settle bound carries the resulting cost). The reduction is
    /// iterative on explicit stacks per the crate's recursion rule, and it
    /// stays hand-rolled rather than routed through `crate::fold`'s binary
    /// counter: the counter is an online entry-count balancer, this is an
    /// offline mass balancer whose combiner charges the running total as a side
    /// effect of every merge.
    fn settle_armings(&mut self) {
        // The ledger must hold armings: an empty one would still push the
        // virtual closing entry and charge the final window against nobody's
        // debt. The one caller tests this immediately above the call, so the
        // check lives there rather than being re-taken here.
        debug_assert!(
            !self.promotions.is_empty(),
            "the ledger settles only behind a non-empty check"
        );
        let armings = core::mem::take(&mut self.promotions);
        let (final_window_sign, final_window_magnitude, final_window_shift) =
            self.banked_window.sign_magnitude_shl();
        debug_assert_ne!(
            final_window_sign,
            Ordering::Less,
            "interval masses only accumulate"
        );
        // The leaf aggregates, in sweep order; the virtual closing entry
        // carries the final window and no parked mass, so it is charged by
        // every arming and charges nothing itself.
        let mut leaves: Vec<Aggregate> = Vec::with_capacity(armings.len() + 1);
        for arming in armings {
            let mut parked = Accumulator::new();
            fold_signed(&mut parked, arming.sign, &arming.parked);
            let mut windows = WindowMass::new();
            windows.merge(&arming.window, arming.shift);
            leaves.push(Aggregate { parked, windows });
        }
        // The final window is nonzero — the drivers' loops credit an interval
        // after every boundary, so the segment banked behind the last freeze
        // carries mass — but nothing here rests on it: a zero mass merges as
        // the no-op its empty limb walk makes it.
        let mut windows = WindowMass::new();
        windows.merge(&final_window_magnitude, final_window_shift);
        leaves.push(Aggregate {
            parked: Accumulator::new(),
            windows,
        });
        // Prefix sums of the leaf masses: the split currency. A leaf's mass is
        // what its merges read — parked digits plus window density — floored at
        // one so empty leaves still take a slot.
        let mut prefix: Vec<u64> = Vec::with_capacity(leaves.len() + 1);
        let mut running = 0u64;
        prefix.push(0);
        for leaf in &leaves {
            running += (leaf.parked.digit_count() + leaf.windows.digits.len()).max(1) as u64;
            prefix.push(running);
        }
        // The reduction: expand ranges at their mass midpoint, merge on the way
        // back up. `Open`s and `Merge`s interleave exactly as the recursion
        // would, the left (older) half always reduced first — so every merge's
        // left operand precedes its right in sweep order, and the unit ranges
        // arrive in ascending order, draining the leaves front to back.
        enum Step {
            Open(usize, usize),
            Merge,
        }
        let leaf_count = leaves.len();
        let mut leaves = leaves.into_iter();
        let mut next_leaf = 0;
        let mut control = vec![Step::Open(0, leaf_count)];
        let mut reduced: Vec<Aggregate> = Vec::new();
        while let Some(step) = control.pop() {
            match step {
                Step::Open(lo, hi) => {
                    if hi - lo == 1 {
                        debug_assert_eq!(
                            next_leaf, lo,
                            "the left-first reduction reaches unit ranges in ascending order"
                        );
                        next_leaf += 1;
                        reduced.push(leaves.next().expect("one aggregate per unit range"));
                    } else {
                        let mid = mass_split(&prefix, lo, hi);
                        control.push(Step::Merge);
                        control.push(Step::Open(mid, hi));
                        control.push(Step::Open(lo, mid));
                    }
                }
                Step::Merge => {
                    let right = reduced.pop().expect("the right half reduced");
                    let mut left = reduced.pop().expect("the left half reduced");
                    left.merge(right, &mut self.total);
                    reduced.push(left);
                }
            }
        }
    }

    /// Close the sweep: the final segment settlement, the promotion ledger's
    /// one settle, then the base's whole-interval term `B · 2^S`.
    ///
    /// The parked component's final segment mass is exactly the tail from its
    /// anchor, because the interval masses tile the unit interval; when the
    /// ledger holds armings, the same tiling makes the banked windows plus the
    /// final segment exactly the interval mass behind every arming's debt, so
    /// the final segment is banked (one more watermark read, on promoting
    /// sweeps only) before the ledger settles. The live component owes nothing
    /// here: every interval already credited it directly.
    pub(super) fn finish(mut self, closing_shift: u64) -> (Ordering, UBig) {
        self.settle();
        if !self.promotions.is_empty() {
            let (segment_sign, segment_magnitude, segment_shift) =
                self.segment_mass.sign_magnitude_shl();
            debug_assert_ne!(
                segment_sign,
                Ordering::Less,
                "interval masses only accumulate"
            );
            // Nonzero by the drivers' loop shape (an interval always follows
            // the last freeze), and harmless when zero: a zero magnitude
            // banks nothing.
            self.banked_window
                .add_magnitude_shl(&Base::from(segment_magnitude), segment_shift);
            self.settle_armings();
        }
        if !self.base.is_literally_zero() {
            self.total.add_accum_shl(&self.base, closing_shift);
        }
        self.total.sign_magnitude()
    }
}
