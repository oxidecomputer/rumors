//! The shape axis: every input family's operand bundle.
//!
//! The roster is the registry's ([`FamilyId::board`], the render-order filter
//! over [`FamilyId::ALL`]); every shape is built through the registry's
//! [`Shape`] door, and the bundle post-pass in [`FamilyData::build`] derives
//! uniformly every slot a shape does not natively fill, so a shape reaches
//! every operation its bundle supplies (the board module doc's product section)
//! without naming any.

use crate::codec;
use crate::meter::registry::{FamilyId, Shape};
use crate::{Clock, Party, Rank, Version};

use super::operand::value_content_bytes;

// ─── family sizes at scale 1.0 ──────────────────────────────────────────────

/// Dense event spine depth at scale 1.0 (packed size ~4 KiB).
const DENSE_BASE_DEPTH: usize = 8_000;

/// Bigroot root magnitude in bits at scale 1.0.
const BIGROOT_BASE_MAGNITUDE_BITS: usize = 8_000;

/// Bigroot spine depth at scale 1.0 (packed size ~3 KiB with the magnitude).
const BIGROOT_BASE_DEPTH: usize = 2_000;

/// Hugeleaf magnitude in bits at scale 1.0 (packed size ~4 KiB).
///
/// Sized so the level doubling stays inside one backend decimal-conversion
/// regime: the backend's divide-and-conquer parser switches algorithm between
/// 16,000 and 20,000 value bits (its parse transient steps from ~1× to ~4× the
/// value bytes there, by measurement), and a probe pair straddling that switch
/// reads the step as a heap exponent — a 16,000-bit base fits e 1.41 on the
/// noncanon parse cell from a flat 2 B/B constant, while this base's pair and
/// every deeper one fit e ≤ 1.0. A two-point fit prices scaling only with both
/// probes on one side of the backend's own threshold, and the larger base is
/// strictly more adversarial for the shape's purpose (maximal bits per node).
const HUGELEAF_BASE_MAGNITUDE_BITS: usize = 32_000;

/// Id spine depth at scale 1.0 (packed pair ~6 KiB).
const ID_BASE_DEPTH: usize = 12_000;

/// Boundary-comb tooth magnitude (bits) and tooth count at scale 1.0 (packed
/// size ~4 KiB); one parameter drives both, mirroring the meter suite's `k = n`
/// convention.
///
/// Scaling `k` with `n` is the separating choice: it keeps the comb's absolute
/// value content growing quadratically in the packed input, so a sweep that
/// materializes running leaf values in a plain big integer reads a superlinear
/// exponent here instead of hiding a `k`-sized constant under a fixed
/// magnitude.
const CLIFF_BASE_SCALE: usize = 128;

/// Comb-scatter tooth count at scale 1.0 (packed cross ~32 KiB).
///
/// Scale drives the tooth count (and with it the scattered party's fragment
/// count, half the teeth); the tooth magnitude stays at
/// [`CROSS_TOOTH_MAGNITUDE_BITS`], so the operands grow linearly and the
/// output-domination ratio holds at every scale.
const CROSS_BASE_TEETH: usize = 128;

/// Comb-scatter tooth magnitude in bits (fixed across scales).
const CROSS_TOOTH_MAGNITUDE_BITS: usize = 1_000;

/// Harmonic spine depth at scale 1.0 (packed size ~6 KiB, matching the
/// dense spine's depth).
const HARMONIC_BASE_DEPTH: usize = 8_000;

/// Scatter population at scale 1.0: balanced-forked parties, one tick
/// each (~10 KiB of packed single-tick versions).
const SCATTER_BASE_CLOCKS: usize = 1_024;

/// Nested-full-sibling depth at scale 1.0 (packed pair ~1.5 KiB).
///
/// Deep enough that a per-level re-scan genre reads its exponent across the
/// level doubling, small enough that the quadratic pin stays inside the board's
/// runtime budget at the acceptance scale.
const NESTED_BASE_DEPTH: usize = 1_500;

/// Nested-wide depth and root-magnitude bits at scale 1.0 (equal, so the
/// doubling scales width and depth together — the cross's cost genre is their
/// product; packed pair ~1.5 KiB).
///
/// Small enough that even a width × depth kernel stays inside the
/// acceptance-scale runtime budget; the red reading rides the exponent leg, not
/// the constant ceiling.
const NESTED_WIDE_BASE: usize = 1_000;

/// Mirror-wide depth and tail-magnitude bits at scale 1.0 (equal, as above;
/// packed pair ~1 KiB). The memo arm's chains grow steeper than the right-full
/// arm's, so the base sits lower.
const MIRROR_WIDE_BASE: usize = 500;

/// Mirror-narrow depth at scale 1.0 (packed pair ~1.5 KiB): the nested-full
/// base, mirrored — the memo machinery at the same depth the right-full cells
/// walk.
const MIRROR_NARROW_BASE_DEPTH: usize = 1_500;

/// Staircase depth at scale 1.0 (packed pair ~2 KiB): deep enough that
/// per-level minimum bookkeeping would read its exponent across the doubling,
/// all values word-scale.
const STAIRCASE_BASE_DEPTH: usize = 1_500;

/// Reveal-comb site count and plateau-magnitude bits at scale 1.0
/// (equal; packed pair ~1 KiB).
///
/// One parameter drives both, so the doubling scales the site count and the
/// circulated width together — the cycle's cost genre is their product. The
/// close-reveal cycle's per-site cost is steeper than the mirror families'
/// chains, so the base sits at the mirror-wide level.
const REVEAL_COMB_BASE: usize = 500;

/// Pure-comb level count and leaf-magnitude bits at scale 1.0 (equal,
/// as above; packed pair ~1 KiB).
///
/// The watermark web's own cycle runs at ~2 wide folds per level — a
/// tenth of the reveal comb's constant — so the base sits higher for comparable
/// work.
const PURE_COMB_BASE: usize = 1_000;

/// Ascending-cliff spine length and leaf-magnitude bits at scale 1.0 (equal, so
/// the doubling scales the hop count and the residue width together — the
/// cascade's cost genre is their product; packed pair ~1 KiB).
///
/// The cascade runs at ~4 touches per input byte on the cured fold direction —
/// the leveled control's constant — so the base sits at the pure-comb level for
/// comparable work. The base is a multiple of 32 deliberately (992 = 31 ×
/// 32): the family's rank exponent is `s − 1`, and `rank_sum` lands its small
/// summands at bit remainder `exp mod 32` (an honest amortized-O(1) constant
/// that flips with the remainder — the freeze-position base's derivation
/// carries the mechanism); `32 | s` pins the remainder at 31 at every ladder
/// point, so the exponent trend compares like against like across the whole
/// ladder. Among the multiples of 32 the base sits just below the 1024-bit
/// magnitude boundary: the tick walk's certificate buffers round their
/// capacity at powers of two, the ×2 ladder preserves the base's position
/// inside that period at every point, and the position just below a boundary
/// samples the rounding at its efficient edge — a base just above one
/// samples the same work at nearly double the held capacity.
const ASCEND_CLIFF_BASE: usize = 992;

/// Dominated-undercut site count and wide-width bits at scale 1.0 (equal;
/// packed pair ~13 KiB).
///
/// One knob drives both, so the doubling scales the emission count and the
/// per-site climb width together — every site's climb is its own input-funded
/// wide code, so the packed pair grows with their product.
///
/// The base is a multiple of 32 deliberately: the family's dominant rank
/// summand rides the `5 · 2^s` climb, and `rank_sum` lands its small summands
/// at bit remainder `exp mod 32` (an honest amortized-O(1) constant that
/// flips with the remainder — the freeze-position base's derivation carries
/// the mechanism); `32 | s` keeps the remainder fixed across the level
/// doubling, so the exponent leg compares like against like. The build arm
/// floors the knob at the generator's minimum width (the domination read must
/// decide the word bound from the wide side), which binds only under extreme
/// scale-down.
const DOMINATED_UNDERCUT_BASE: usize = 160;

/// Ticks behind the integer (exponent-zero) rank of the `rank_pair_ops` row:
/// small, so the pair's cost is carried entirely by the mismatch.
const RANK_PAIR_INTEGER_TICKS: u64 = 3;

/// Probes per accumulator byte (as a divisor) on the `party_join_all_overlap`
/// row.
///
/// The probe count scales with the accumulator so the row's exponent judges the
/// fold against a denominator both sides of which double together — work
/// scaling with the fixed accumulator per input reads quadratic there — and the
/// divisor keeps the row inside the board's runtime budget.
pub(super) const OVERLAP_FOLD_INPUT_DIVISOR: usize = 64;

/// Two-operand jump-comb teeth at scale 1.0 (packed pair ~35 KiB, the teeth
/// operand's per-level wide codes dominating).
///
/// One knob drives the tooth count and, through [`JUMP_PAIR_DIGIT_DIVISOR`],
/// the isolated-position digit count, at the fixed tooth magnitude
/// [`JUMP_PAIR_MAGNITUDE_BITS`]: an absolute-position freeze accounting pays
/// teeth × digits × magnitude here, so the doubling scales the crest count and
/// the position density together while the packed pair grows linearly — the
/// separating choice that makes any such accounting read on the exponent leg
/// rather than hide in a constant.
const JUMP_PAIR_BASE_TEETH: usize = 256;

/// Tooth magnitude (bits) of the two-operand jump comb, fixed across scales:
/// comfortably over the freeze allowance's 256-bit digit bound, so every cheap
/// fold behind a wide difference crest parks the drift.
const JUMP_PAIR_MAGNITUDE_BITS: usize = 512;

/// Isolated-position digits per tooth (as a divisor) on the two-operand jump
/// comb.
///
/// The digit count scales with the teeth at an eighth: deep enough that any
/// per-freeze absolute-position work reads its exponent across the doubling,
/// shallow enough that the shared spine stays a small fraction of the packed
/// pair.
const JUMP_PAIR_DIGIT_DIVISOR: usize = 8;

/// Freeze-position blocks at scale 1.0 (packed version ~74 KiB, the per-block
/// wide drop codes dominating).
///
/// The scale of the `skyline_flatness` freeze-position band's small run: the
/// committed known-bad accounting reads superlinear per-byte growth across
/// this regime's doubling (the committed adequacy tripwire keeps it failing
/// there), so the
/// board's default pair straddles exactly what the family exists to catch. The
/// base is a multiple of 16 deliberately: the family's rank exponent is `2s −
/// 1` (one trailing zero strips — exactly one leaf term, the odd `2^L + 1` at
/// weight `2^1`, has 2-adic valuation one), and `rank_sum` lands each small
/// summand at bit remainder `exp mod 32`, where a remainder near the digit top
/// makes most landings span two digits instead of one — an honest
/// amortized-O(1) constant, but one that flips with the remainder, and an
/// exponent fitted across two scales with different remainders reads the flip
/// as growth. `16 | s` keeps `2s ≡ 0 (mod 32)`, so every doubling
/// preserves the remainder and the exponent leg compares like against like.
const FREEZE_POS_BASE_BLOCKS: usize = 1_024;

/// Promotion re-arm blocks at scale 1.0 (packed version ~128 KiB, the per-block
/// wide arming codes dominating).
///
/// Half the `skyline_flatness` promotion re-arm band's small run: the committed
/// span-reading promotion reads superlinear per-byte growth across that
/// regime's doubling (the committed span-promotion tripwire keeps it failing
/// there), so the
/// board's default pair straddles what the family exists to catch. The base is
/// a multiple of 8 deliberately: the family's rank exponent is `36s`, and
/// `rank_sum` lands its small summands at bit remainder `exp mod 32` (an honest
/// amortized-O(1) constant that flips with the remainder — the freeze-position
/// base's derivation carries the mechanism); `8 | s` keeps `36s ≡ 0 (mod 32)`,
/// so every doubling compares like against like.
const PROMO_REARM_BASE_BLOCKS: usize = 512;

/// Weight-comb block pairs at scale 1.0 (packed version ~7 KiB, the spine's
/// unit codes dominating), rounded up to a power of two at every scale.
///
/// The rounding is the complete-subtree relation's call-site repair, and the
/// level doubling then doubles the rounded count exactly. The base is the scale
/// of the `skyline_flatness` weight-comb band's small run: with certificate
/// consumption disabled, rank reads ×1.93 per-byte growth across this regime's
/// doubling (the band ceiling doc's committed probe-build measurement), so the
/// board's default pair straddles exactly what the family exists to catch.
/// Power-of-two `n` keeps the spine depth `32n ≡ 0 (mod 32)`, so `rank_sum`
/// lands its small summands at the same bit remainder at both scales and the
/// exponent leg compares like against like (the freeze-position base's
/// derivation carries the mechanism).
const WEIGHT_COMB_BASE_BLOCKS: usize = 512;

/// Freeze-parade blocks at scale 1.0 (packed version ~49 KiB, the blocks' wide
/// drop codes dominating), rounded up to a power of two at every scale (the
/// parade block is one complete subtree).
///
/// The scale of the `skyline_flatness` freeze-parade band's small run: with the
/// write watermark disabled, rank reads ×1.91 per-byte growth in the touch and
/// limb currencies together across this regime's doubling (the band ceiling
/// doc's probe-build measurement of record). Power-of-two `k` keeps the spine
/// depth `64k ≡ 0 (mod 32)`, the same `rank_sum` remainder alignment as the
/// weight comb's.
const FREEZE_PARADE_BASE_BLOCKS: usize = 512;

/// Concurrent-pair forked-party count at scale 1.0, rounded up to a power of
/// two at every scale (the balanced fork and the alternating dominance schedule
/// both need it; the level doubling then doubles it exactly).
const CONCURRENT_BASE_LEAVES: usize = 1_024;

/// Tooth-tail boundary count at scale 1.0 (the pair ~3.6 KiB per operand); the
/// spike width rides the same knob at [`TOOTH_TAIL_SPIKE_DIVISOR`], the
/// committed flatness band's ratio.
const TOOTH_TAIL_BASE_BOUNDARIES: usize = 4_096;

/// Dense-suffix blocks (and gap digits: one knob drives both, the `DS(p, p)`
/// diagonal) at scale 1.0 (packed version ~122 KiB, the blocks' wide climb
/// codes dominating).
///
/// The scale of the `skyline_flatness` dense-suffix bands' small run: the
/// committed per-arming suffix walk reads ×1.96 per-byte growth across that
/// regime's doubling (the query fold's committed tripwire), so the board's
/// default pair straddles what the family exists to catch. The base is a
/// multiple of 32 deliberately: the family's rank exponent is linear in the
/// knob, and `rank_sum` lands its small summands at bit remainder `exp mod 32`
/// (an honest amortized-O(1) constant that flips with the remainder — the
/// freeze-position base's derivation carries the mechanism); `32 | s` keeps any
/// integer-linear exponent's remainder fixed across the level doubling, so the
/// exponent leg compares like against like.
const DENSE_SUFFIX_BASE_BLOCKS: usize = 512;

/// Wide-arming digits (arming digits and gap digits together: the `WA(w, d)`
/// diagonal at `w = d`) at scale 1.0 (packed version ~13 KiB).
///
/// The scale sits beside the two committed bands that price the family's seams
/// (`ledger_wide_arming`'s small run is 500, the `parse_wide_arming` band's
/// 256): the committed schoolbook settle reads ~×1.9 per byte and the committed
/// schoolbook parse ×2.00 per byte across those regimes' doublings (the query
/// fold's and the text kernel's committed tripwires), so the board's default
/// pair straddles what the family exists to catch on both seams. No remainder
/// alignment is needed: the family's rank exponent is `32s`, a multiple of 32
/// at every knob, so `rank_sum` lands its small summands at bit remainder 0 at
/// both scales (the freeze-position base's derivation carries the mechanism).
/// The build arm floors the knob at the generator's minimum width (the parked
/// component must clear the settling drift's ten digits), which binds only
/// under extreme scale-down.
const WIDE_ARMING_BASE_DIGITS: usize = 512;

/// Plateau-puncture digits (plateau digits and turn count together: the `PP(w,
/// d)` diagonal at `w = d`) at scale 1.0 (packed version ~15 KiB).
///
/// The smallest knob at which the board's default pair still separates the
/// family's genre from a conforming fold, at the board's own cost: the family's
/// packed construction spells the plateau once per turn, so every bundle build
/// pays `Θ(s²)` packed bits, and this knob owns the board's dominant build
/// cost. Calibration (dev profile, exact counters, the query fold's committed
/// schoolbook kernel): across the level doubling PP(384, 384) → PP(768, 768)
/// the known-bad settle reads ×1.879 touch and ×1.579 limb per byte — above the
/// shipped kernels' ×1.25 flatness ceiling by more than the board's ×1.25
/// one-reading band ([`NEAR_TIE_RATIO`](super::worst::NEAR_TIE_RATIO)) in both
/// width currencies (≥ ×1.5625), the margin policy; the next smaller multiple
/// of 32 fails it (×1.555 limb at 352), and the margin only grows toward the
/// acceptance scale (the growth is monotone in the knob — ×1.91 touch and ×1.65
/// limb at 512, the committed tripwire's own regime). A multiple of 32 for the
/// same `rank_sum` remainder alignment as [`DENSE_SUFFIX_BASE_BLOCKS`]; the
/// build arm floors the knob at the generator's minimum width (the plunge must
/// trip the freeze allowance past a unit code), which binds only under extreme
/// scale-down.
const PLATEAU_PUNCTURE_BASE_DIGITS: usize = 384;

/// Lone-freeze oscillation pairs per axis (the `LF(s, s)` diagonal: one knob
/// drives the never-freezing plateau prefix and the frozen tail together) at
/// scale 1.0 (packed version ~2.6 KiB).
///
/// The scale of the `skyline_flatness` lone-freeze bands' small runs (each band
/// isolates one axis at the generator minimum; the board column scales both, so
/// a regression on either side reads on the doubling). A multiple of 32 for the
/// same `rank_sum` remainder alignment as [`DENSE_SUFFIX_BASE_BLOCKS`], kept
/// even at every scale by the build arm (the generator counts whole oscillation
/// pairs).
const LONE_FREEZE_BASE_PAIRS: usize = 2_048;

/// Boundaries per spike digit in the tooth-tail bundle.
///
/// The committed flatness band's `g = m/64` ratio, so the board prices the same
/// spike-to-tail proportion the envelope band holds flat (a spike a few wide
/// digits under thousands of post-cancellation sign reads — the exact-top genre
/// needs the tail to dominate the spike).
const TOOTH_TAIL_SPIKE_DIVISOR: usize = 64;

/// Staggered fold population operand count at scale 1.0, rounded up to the
/// power of two the slot addressing needs.
///
/// 64 operands of [`STAGGER_BASE_BLOCKS`] teeth each keep the default-scale
/// fold cells seconds-fast while giving the balanced reduction seven levels of
/// intermediate swell; the level doubling doubles both knobs, so the two board
/// scales move the arity and the per-operand size together and the declared
/// fold model's exponent ceiling is computed from the realized pair.
const STAGGER_BASE_OPERANDS: usize = 64;

/// Staggered fold population blocks (teeth) per operand at scale 1.0, rounded
/// up to a power of two (the complete top tree needs it).
const STAGGER_BASE_BLOCKS: usize = 64;

/// Benign clock population at scale 1.0.
const BENIGN_BASE_CLOCKS: usize = 256;

/// The weave fold population's leaf count at scale 1.0 (rounded up to a power
/// of two by construction).
///
/// 4096 leaves woven into [`WEAVE_GROUPS`] parties give each operand ~256
/// scattered leaves under a fully shared upper skeleton — deep enough that the
/// both-present-rich cost terms (the indexed overlap test's per-node table
/// searches, the version joins' interleaved merges) dominate each cell, small
/// enough that the default-scale board stays seconds-fast.
const WEAVE_BASE_LEAVES: usize = 4_096;

/// How many parties the weave population folds: fixed across scales, so the
/// family's scaling axis is operand *size* (the both-present richness per
/// merge), not arity — scatter and benign already own the arity axis.
///
/// 16 groups keep every internal node of the shared skeleton above the last
/// four levels both-present in every operand while each operand stays an
/// organic, individually well-formed region set.
const WEAVE_GROUPS: usize = 16;

/// Floor on every scaled size parameter, so extreme scale-down (the smoke test)
/// still builds valid shapes and a nonempty benign population.
///
/// The floor preserves positivity, never a *relation* between two parameters
/// (an even count, a width strictly under a depth, an ascent inside a code
/// band). Shapes whose generator asserts a relation therefore drive every
/// related parameter from one knob — the `ascend_cliff(s, s)` / `reveal_comb(s,
/// s)` convention — or repair at the call site as [`CONCURRENT_BASE_LEAVES`]'s
/// power-of-two rounding does; a two-knob shape with a floored relation panics
/// in the smoke test's extreme scale-down.
pub(super) const MIN_SIZE_PARAM: usize = 4;

/// Fixed seed for the benign family's pseudo-random construction: the control
/// row must be deterministic run to run.
const BENIGN_RNG_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// One shape instantiated at one scale: the operand bundle every row's
/// `prepare` decodes fresh (outside measurement).
///
/// The bundle is the shape axis's declaration: each slot a shape fills flows to
/// every operation whose signature consumes it, so a shape's reach is
/// structural — build a shape with a version and it appears on every version
/// row; give it an id side and it appears on every party row through the
/// disjoint-mount adapter. The derived slots (`version2`, `rank_pair`, a cross
/// shape's `version` and `parties`) are filled by one uniform post-pass in
/// [`FamilyData::build`], never per shape.
pub(super) struct FamilyData {
    pub(super) kind: FamilyId,
    pub(super) name: &'static str,
    /// The shape's primary packed version (a cross shape's event side).
    pub(super) version: Option<Vec<u8>>,
    /// The comparison counterpart: `version` plus one seed tick, packed.
    ///
    /// Derived uniformly by the post-pass — except on the pair shapes
    /// (jump-pair, concurrent-pair), whose build arms fill it with the pairing
    /// the shape was constructed around and the post-pass leaves in place.
    pub(super) version2: Option<Vec<u8>>,
    /// A disjoint packed party pair within one universe: natural for the id
    /// pair and the benign halves, minted by the disjoint-mount adapter from a
    /// cross shape's id side.
    pub(super) parties: Option<(Vec<u8>, Vec<u8>)>,
    /// The designated packed (event version, id party) cross: the pairing the
    /// shape was built around, driving the tick rows' walk floors and the clock
    /// rows' operand choice.
    ///
    /// Each cross shape's variant doc states the arm and cost genre its cross
    /// drives.
    pub(super) cross: Option<(Vec<u8>, Vec<u8>)>,
    /// Whether the cross's mandatory projection output dominates its input (the
    /// comb-scatter and plateau-comb shapes): the projection rows
    /// I/O-denominate exactly these cells (the `cell` module doc's
    /// output-domination cross).
    pub(super) output_dominated: bool,
    /// The bundle's value content in bytes, `Some` only on the flat-denominator
    /// shape (comb-scatter).
    ///
    /// The denominator every input-denominated cell's *exponent* is fitted
    /// against; constants and floors stay per packed byte (the `cell` module
    /// doc derives the split).
    pub(super) content_bytes: Option<usize>,
    /// The packed fold operands (versions, parties), consumed by the two fold
    /// rows alone: the scatter, weave, and stagger populations' adversarial
    /// orderings and the benign shape's organic control.
    #[allow(clippy::type_complexity)]
    pub(super) fold: Option<(Vec<Vec<u8>>, Vec<Vec<u8>>)>,
    /// An overlapping packed party pair within one universe: the rejection
    /// rows' operands.
    ///
    /// Minted by the overlap-mount adapter from the same id source as
    /// `parties` (the post-pass); semantically void by design — see
    /// [`overlap_mounted_pair`].
    pub(super) overlap: Option<(Vec<u8>, Vec<u8>)>,
    /// The mismatched rank pair, derived from `version` in the post-pass.
    ///
    /// Precomputed here (shape-derived rank, small integer rank) so the
    /// `rank_pair_ops` and `rank_sum` prepares clone their operands instead of
    /// re-running the rank fold: the bench harness calls prepare once per timed
    /// iteration, and the fold costs orders of magnitude more than the pair
    /// operations it feeds.
    pub(super) rank_pair: Option<(Rank, Rank)>,
}

impl FamilyData {
    /// A bundle with every slot empty, for a build arm to fill with what the
    /// shape honestly has; the name is the registry's name of record.
    fn bare(kind: FamilyId) -> FamilyData {
        FamilyData {
            kind,
            name: kind.name(),
            version: None,
            version2: None,
            parties: None,
            cross: None,
            output_dominated: false,
            content_bytes: None,
            fold: None,
            overlap: None,
            rank_pair: None,
        }
    }

    /// Build a shape's operand bundle at `scale`, doubled `level` times.
    ///
    /// `level` 0 and 1 are the two measurement scales of every cell. The arm
    /// fills the slots the shape natively has; the post-pass below derives the
    /// rest uniformly (a cross shape's version is its event side, its party
    /// pair is the disjoint-mount adapter over its id side; every version gains
    /// its rank pair, and its ticked counterpart wherever the arm built no
    /// pairing of its own), so a new shape reaches every operation its bundle
    /// supplies without naming any.
    pub(super) fn build(kind: FamilyId, scale: f64, level: u32) -> FamilyData {
        let size = |base: usize| -> usize {
            let scaled = ((base as f64) * scale).round() as usize;
            scaled.max(MIN_SIZE_PARAM) << level
        };
        let mut data = match kind {
            FamilyId::Dense => Self::event(
                kind,
                Shape::Dense
                    .packed1(size(DENSE_BASE_DEPTH))
                    .version()
                    .encode(),
            ),
            FamilyId::Bigroot => Self::event(
                kind,
                Shape::Bigroot
                    .packed2(size(BIGROOT_BASE_MAGNITUDE_BITS), size(BIGROOT_BASE_DEPTH))
                    .version()
                    .encode(),
            ),
            FamilyId::Hugeleaf => Self::event(
                kind,
                Shape::Hugeleaf
                    .packed1(size(HUGELEAF_BASE_MAGNITUDE_BITS))
                    .version()
                    .encode(),
            ),
            FamilyId::Cliff => {
                let scale = size(CLIFF_BASE_SCALE);
                Self::event(
                    kind,
                    Shape::CliffComb.packed2(scale, scale).version().encode(),
                )
            }
            FamilyId::IdPair => {
                let mut data = Self::bare(kind);
                data.parties = Some((
                    Shape::IdSpine
                        .packed_flagged(size(ID_BASE_DEPTH), false)
                        .bytes,
                    Shape::IdSpine
                        .packed_flagged(size(ID_BASE_DEPTH), true)
                        .bytes,
                ));
                data
            }
            FamilyId::CombScatter => {
                let teeth = size(CROSS_BASE_TEETH);
                let mut data = Self::bare(kind);
                data.cross = Some((
                    Shape::CliffComb
                        .packed2(CROSS_TOOTH_MAGNITUDE_BITS, teeth)
                        .version()
                        .encode(),
                    Shape::ScatteredId.packed1(teeth / 2).bytes,
                ));
                data.output_dominated = true;
                let (v, p) = data.cross.as_ref().expect("just set");
                data.content_bytes = Some(value_content_bytes(&decode_version(v)) + p.len());
                data
            }
            FamilyId::Harmonic => Self::event(
                kind,
                Shape::Harmonic
                    .packed1(size(HARMONIC_BASE_DEPTH))
                    .version()
                    .encode(),
            ),
            FamilyId::Scatter => Self::scatter(size(SCATTER_BASE_CLOCKS)),
            FamilyId::Weave => Self::weave(size(WEAVE_BASE_LEAVES)),
            FamilyId::Stagger => Self::stagger(
                size(STAGGER_BASE_OPERANDS).next_power_of_two(),
                size(STAGGER_BASE_BLOCKS).next_power_of_two(),
            ),
            FamilyId::NestedFull => {
                let d = size(NESTED_BASE_DEPTH);
                Self::cross_family(
                    kind,
                    Shape::Dense.packed1(d).version().encode(),
                    Shape::NestedFullId.packed1(d).bytes,
                )
            }
            FamilyId::NestedWide => {
                let s = size(NESTED_WIDE_BASE);
                Self::cross_family(
                    kind,
                    Shape::Bigroot.packed2(s, s).version().encode(),
                    Shape::NestedFullId.packed1(s).bytes,
                )
            }
            FamilyId::MirrorWide => {
                let s = size(MIRROR_WIDE_BASE);
                Self::cross_family(
                    kind,
                    Shape::WideTail.packed2(s, s).version().encode(),
                    Shape::NestedLeftFullId.packed1(s).bytes,
                )
            }
            FamilyId::MirrorNarrow => {
                let d = size(MIRROR_NARROW_BASE_DEPTH);
                Self::cross_family(
                    kind,
                    Shape::WideTail.packed2(1, d).version().encode(),
                    Shape::NestedLeftFullId.packed1(d).bytes,
                )
            }
            FamilyId::Staircase => {
                let d = size(STAIRCASE_BASE_DEPTH);
                Self::cross_family(
                    kind,
                    Shape::Staircase.packed1(d).version().encode(),
                    Shape::IdSpine.packed_flagged(d, false).bytes,
                )
            }
            FamilyId::RevealComb => {
                let s = size(REVEAL_COMB_BASE);
                let mut data = Self::cross_family(
                    kind,
                    Shape::RevealComb.packed2(s, s).version().encode(),
                    Shape::RevealCombId.packed1(s).bytes,
                );
                // Projecting the shared-wide-plateau event through its
                // site-owning comb id re-materializes a wide absolute value per
                // kept site: mandatory output Theta(k*b) on a Theta(k + b)
                // input — output ~x4 per joint input doubling by construction
                // — the same output domination the comb-scatter cross
                // declares.
                data.output_dominated = true;
                data
            }
            FamilyId::RevealHifloor => {
                let s = size(REVEAL_COMB_BASE);
                let mut data = Self::cross_family(
                    kind,
                    Shape::RevealCombHifloor.packed2(s, s).version().encode(),
                    Shape::RevealCombId.packed1(s).bytes,
                );
                // The raised floor changes the consume-time gap, not the
                // projection's re-materialized wide sites: the same output
                // domination as reveal-comb.
                data.output_dominated = true;
                data
            }
            FamilyId::PureComb => {
                let s = size(PURE_COMB_BASE);
                let mut data = Self::cross_family(
                    kind,
                    Shape::PureComb.packed2(s, s).version().encode(),
                    Shape::PureCombId.packed1(s).bytes,
                );
                // Bare wide leaves under the site-owning id: the masked skyline
                // spells a wide code per owned site, the same output domination
                // as reveal-comb.
                data.output_dominated = true;
                data
            }
            FamilyId::AscendCliff => {
                let s = size(ASCEND_CLIFF_BASE);
                Self::cross_family(
                    kind,
                    Shape::AscendCliff.packed2(s, s).version().encode(),
                    Shape::AscendCliffId.packed1(s).bytes,
                )
            }
            FamilyId::AscendPlateau => {
                let s = size(ASCEND_CLIFF_BASE);
                Self::cross_family(
                    kind,
                    Shape::AscendCliffPlateau.packed2(s, s).version().encode(),
                    Shape::AscendCliffId.packed1(s).bytes,
                )
            }
            FamilyId::DominatedUndercut => {
                // One knob drives the site count and the wide width (the
                // band's DU(s, s) diagonal), floored at the generator's
                // minimum width; the floor binds only under extreme
                // scale-down (the base constant's rustdoc).
                let s = size(DOMINATED_UNDERCUT_BASE).max(128);
                Self::cross_family(
                    kind,
                    Shape::DominatedUndercut.packed2(s, s).version().encode(),
                    Shape::DominatedUndercutId.packed1(s).bytes,
                )
            }
            FamilyId::JumpPair => {
                let m = size(JUMP_PAIR_BASE_TEETH);
                let d = (m / JUMP_PAIR_DIGIT_DIVISOR).max(1);
                let (a, b) = Shape::JumpPair.packed_pair3(JUMP_PAIR_MAGNITUDE_BITS, m, d);
                let mut data = Self::event(kind, a.version().encode());
                data.version2 = Some(b.version().encode());
                data
            }
            FamilyId::FreezePos => Self::event(
                kind,
                Shape::FreezePosition
                    .packed1(size(FREEZE_POS_BASE_BLOCKS))
                    .version()
                    .encode(),
            ),
            FamilyId::PromoRearm => Self::event(
                kind,
                Shape::PromotionRearm
                    .packed1(size(PROMO_REARM_BASE_BLOCKS))
                    .version()
                    .encode(),
            ),
            FamilyId::WeightComb => Self::event(
                kind,
                Shape::WeightComb
                    .packed1(size(WEIGHT_COMB_BASE_BLOCKS).next_power_of_two())
                    .version()
                    .encode(),
            ),
            FamilyId::FreezeParade => Self::event(
                kind,
                Shape::FreezeParade
                    .packed1(size(FREEZE_PARADE_BASE_BLOCKS).next_power_of_two())
                    .version()
                    .encode(),
            ),
            FamilyId::DenseSuffix => {
                // One knob drives the block count and the gap-digit count (the
                // bands' DS(p, p) diagonal); the mate is the same topology at
                // unit bases, the pair the distance band prices.
                let p = size(DENSE_SUFFIX_BASE_BLOCKS);
                let mut data =
                    Self::event(kind, Shape::DenseSuffix.packed2(p, p).version().encode());
                data.version2 = Some(Shape::DenseSuffixMate.packed2(p, p).version().encode());
                data
            }
            FamilyId::WideArming => {
                // One knob drives the arming width and the gap-digit count (the
                // bands' WA(s, s) diagonal), floored at the generator's minimum
                // width; the floor binds only under extreme scale-down (the
                // base constant's rustdoc).
                let s = size(WIDE_ARMING_BASE_DIGITS).max(10);
                Self::event(kind, Shape::WideArming.packed2(s, s).version().encode())
            }
            FamilyId::PlateauPuncture => {
                // One knob drives the plateau width and the turn count (the
                // band's PP(s, s) diagonal), floored at the generator's minimum
                // width; the floor binds only under extreme scale-down (the
                // base constant's rustdoc).
                let s = size(PLATEAU_PUNCTURE_BASE_DIGITS).max(10);
                Self::event(
                    kind,
                    Shape::PlateauPuncture.packed2(s, s).version().encode(),
                )
            }
            FamilyId::LoneFreeze => {
                // One knob drives the plateau prefix and the frozen tail (the
                // bands isolate each axis; the column scales both), kept even
                // at every scale — the generator counts whole oscillation
                // pairs, and MIN_SIZE_PARAM keeps the masked value at least 4.
                let s = size(LONE_FREEZE_BASE_PAIRS) & !1;
                Self::event(kind, Shape::LoneFreeze.packed2(s, s).version().encode())
            }
            FamilyId::ConcurrentPair => {
                let n = size(CONCURRENT_BASE_LEAVES).next_power_of_two();
                let (v, w) = Shape::ConcurrentPair.version_pair(n);
                let mut data = Self::event(kind, v.encode());
                data.version2 = Some(w.encode());
                data
            }
            FamilyId::ToothTail => {
                // One knob: the boundary count, with the spike width riding it
                // at the committed band ratio (the generator needs g >= 1 and m
                // >= 2; the size floor guarantees both).
                let m = size(TOOTH_TAIL_BASE_BOUNDARIES);
                let (a, b) = Shape::ToothTail.packed_pair((m / TOOTH_TAIL_SPIKE_DIVISOR).max(1), m);
                let mut data = Self::event(kind, a.version().encode());
                data.version2 = Some(b.version().encode());
                data
            }
            FamilyId::Benign => Self::benign(size(BENIGN_BASE_CLOCKS)),
            FamilyId::WideToothComb
            | FamilyId::JumpComb
            | FamilyId::CliffFan
            | FamilyId::CancellingChain
            | FamilyId::AltSpine
            | FamilyId::MemoChain
            | FamilyId::MemoComb
            | FamilyId::MemoFanout
            | FamilyId::MemoOscillating
            | FamilyId::MemoChurn
            | FamilyId::DescendingRaises
            | FamilyId::MaskDrift
            | FamilyId::MeetShade
            | FamilyId::ArmingTrain
            | FamilyId::ScanHole
            | FamilyId::MaskedHole
            | FamilyId::HoistedWindow
            | FamilyId::PropagateSeam
            | FamilyId::LatentLadder => unreachable!(
                "{kind:?} is envelope-only in the registry: it has no operand bundle, \
                 and the board sweeps FamilyId::board() alone"
            ),
        };
        // ── the bundle post-pass: the derived slots, uniform across shapes ──
        // A cross shape's primary version is its event side.
        if data.version.is_none() {
            data.version = data.cross.as_ref().map(|(v, _)| v.clone());
        }
        // Every version gains its ticked comparison counterpart (where the
        // shape did not build its own pairing) and its mismatched rank pair
        // (shape-derived rank against a small integer rank, the pair whose
        // exponent mismatch the rank rows price).
        if let Some(bytes) = &data.version {
            let v = decode_version(bytes);
            if data.version2.is_none() {
                let mut w = v.clone();
                w.tick(&Party::seed());
                data.version2 = Some(w.encode());
            }
            let b = Version::try_from(RANK_PAIR_INTEGER_TICKS)
                .expect("a small integer version is valid")
                .rank();
            data.rank_pair = Some((v.rank(), b));
        }
        // A cross shape's id side becomes a disjoint party pair through
        // the mount adapter.
        if data.parties.is_none() {
            if let Some((_, id)) = &data.cross {
                data.parties = Some(disjoint_mounted_pair(id));
            }
        }
        // Every id source also mints an overlapping pair through the
        // overlap-mount adapter, for the rejection rows: the cross id where the
        // shape has one, the first natural party otherwise.
        if data.overlap.is_none() {
            let id = data
                .cross
                .as_ref()
                .map(|(_, id)| id)
                .or_else(|| data.parties.as_ref().map(|(a, _)| a));
            if let Some(id) = id {
                data.overlap = Some(overlap_mounted_pair(id));
            }
        }
        data
    }

    /// Build the scatter fold population: `n` balanced-forked parties, one tick
    /// each, ordered evens before odds so a sequential fold's accumulator holds
    /// every other leaf and never coalesces.
    fn scatter(n: usize) -> FamilyData {
        let mut parties = vec![Party::seed()];
        while parties.len() < n {
            let mut next = Vec::with_capacity(parties.len() * 2);
            for mut p in parties {
                let q = p.fork();
                next.push(p);
                next.push(q);
            }
            parties = next;
        }
        // Dropping the tail keeps `n` honest at non-power-of-two scales; a
        // dropped party's region simply goes unowned.
        parties.truncate(n);
        let scatter_order = |v: Vec<Vec<u8>>| -> Vec<Vec<u8>> {
            let (evens, odds): (Vec<_>, Vec<_>) =
                v.into_iter().enumerate().partition(|(i, _)| i % 2 == 0);
            evens
                .into_iter()
                .chain(odds)
                .map(|(_, bytes)| bytes)
                .collect()
        };
        let versions = scatter_order(
            parties
                .iter()
                .map(|p| {
                    let mut v = Version::new();
                    v.tick(p);
                    v.encode()
                })
                .collect(),
        );
        let parties = scatter_order(parties.iter().map(Party::encode).collect());
        let mut data = Self::bare(FamilyId::Scatter);
        data.fold = Some((versions, parties));
        data
    }

    /// Build the weave fold population.
    ///
    /// The `leaves` (rounded up to a power of two) leaf parties of one balanced
    /// fork expansion are dealt round-robin into [`WEAVE_GROUPS`] group
    /// parties, each group carrying its own single-tick version.
    ///
    /// Dealing leaf `i` to group `i % WEAVE_GROUPS` puts leaves of every group
    /// under every skeleton node above the last `log2(WEAVE_GROUPS)` levels, so
    /// each operand pair is both-present at the whole shared skeleton — the
    /// correlated-population genre — while each group on its own is an ordinary
    /// scattered region set.
    fn weave(leaves: usize) -> FamilyData {
        let leaves = leaves.next_power_of_two().max(WEAVE_GROUPS * 2);
        let mut parties = vec![Party::seed()];
        while parties.len() < leaves {
            let mut next = Vec::with_capacity(parties.len() * 2);
            for mut p in parties {
                let q = p.fork();
                next.push(p);
                next.push(q);
            }
            parties = next;
        }
        // Deal the leaves round-robin: group `r` takes every WEAVE_GROUPS-th
        // leaf, carrying one tick per dealt leaf — a single-leaf party forces
        // the event onto that leaf, so the group's version is height one
        // exactly over its scattered region, a deep tree sharing the whole
        // upper skeleton with every other group's.
        let mut dealt: Vec<Vec<Party>> = (0..WEAVE_GROUPS).map(|_| Vec::new()).collect();
        for (i, leaf) in parties.into_iter().enumerate() {
            dealt[i % WEAVE_GROUPS].push(leaf);
        }
        // Both sides reduce through the balanced folds rather than a left fold
        // into one accumulator. A group's operands are single leaves of one
        // expansion, so a left fold re-walks the whole growing union and the
        // whole growing event tree once per leaf: quadratic in the group's leaf
        // count, which is the size axis this family scales. The balanced folds
        // pass each operand through `O(log k)` merges of similarly sized
        // operands instead. The built value is the same either way — a group's
        // version is one event at each of its leaves, its party their disjoint
        // union — and both are canonical, so the packed bytes are too.
        let mut versions = Vec::with_capacity(WEAVE_GROUPS);
        let mut parties = Vec::with_capacity(WEAVE_GROUPS);
        for leaves in dealt {
            let version: Version = leaves
                .iter()
                .map(|leaf| {
                    let mut v = Version::new();
                    v.tick(leaf);
                    v
                })
                .sum();
            let mut rest = leaves.into_iter();
            let mut group = rest.next().expect("every group received leaves");
            group
                .join_all(rest)
                .expect("leaves of one fork expansion are pairwise disjoint");
            versions.push(version.encode());
            parties.push(group.encode());
        }
        let mut data = Self::bare(FamilyId::Weave);
        data.fold = Some((versions, parties));
        data
    }

    /// Build the staggered fold population: `n` operands of `m` unit teeth
    /// each, teeth in the gaps of every other operand's.
    ///
    /// Fed in bit-reversed order ([`Shape::StaggerPopulation`]'s constructor
    /// carries both the construction and the feed order's derivation).
    fn stagger(n: usize, m: usize) -> FamilyData {
        let (versions, ids) = Shape::StaggerPopulation.population(n, m);
        let mut data = Self::bare(FamilyId::Stagger);
        data.fold = Some((
            versions.iter().map(|p| p.version().encode()).collect(),
            ids.into_iter().map(|p| p.bytes).collect(),
        ));
        data
    }

    /// Wrap a cross shape: a packed (event, id) pair built as one
    /// adversarial pairing.
    ///
    /// The cross drives the tick rows' walk floors and the clock rows' operand
    /// choice directly; the post-pass derives the shape's version (the event
    /// side) and its disjoint party pair (the mounted id side), so the shape
    /// also reaches every version and party row.
    fn cross_family(kind: FamilyId, version: Vec<u8>, id: Vec<u8>) -> FamilyData {
        let mut data = Self::bare(kind);
        data.cross = Some((version, id));
        data
    }

    /// Wrap an event shape's wire bytes.
    fn event(kind: FamilyId, bytes: Vec<u8>) -> FamilyData {
        let mut data = Self::bare(kind);
        data.version = Some(bytes);
        data
    }

    /// Build the benign control: `n` clocks forked at random from a seed, each
    /// ticked one to three times, folded into one version and two disjoint
    /// half-population parties.
    fn benign(n: usize) -> FamilyData {
        let mut rng = XorShift(BENIGN_RNG_SEED);
        let mut clocks = vec![Clock::seed()];
        while clocks.len() < n {
            let i = (rng.next() as usize) % clocks.len();
            let child = clocks[i].fork();
            clocks.push(child);
        }
        for clock in &mut clocks {
            for _ in 0..(rng.next() % 3 + 1) {
                clock.tick();
            }
        }
        let version: Version = clocks.iter().map(|c| c.version()).sum();
        // The fold rows' organic control: the population's own versions and
        // parties in construction order (the adversarial ordering belongs to
        // the scatter family alone).
        let fold = Some((
            clocks.iter().map(|c| c.version().encode()).collect(),
            clocks.iter().map(|c| c.party().encode()).collect(),
        ));
        let mut parties = clocks.into_iter().map(|c| c.into_parts().0);
        let mut a = parties.next().expect("the population is nonempty");
        let mut b = parties
            .next()
            .expect("MIN_SIZE_PARAM keeps at least two clocks in the population");
        // Alternate the halves so both operand parties scatter across the whole
        // id tree rather than owning one contiguous region, and reduce each
        // half through the balanced fold: a left fold re-walks the growing
        // union once per party, quadratic in the population size this family
        // scales.
        let (to_a, to_b): (Vec<_>, Vec<_>) = parties.enumerate().partition(|(i, _)| i % 2 == 0);
        let disjoint = |half: &mut Party, dealt: Vec<(usize, Party)>| {
            half.join_all(dealt.into_iter().map(|(_, p)| p))
                .expect("forked parties are pairwise disjoint");
        };
        disjoint(&mut a, to_a);
        disjoint(&mut b, to_b);
        let mut data = Self::bare(FamilyId::Benign);
        data.version = Some(version.encode());
        data.parties = Some((a.encode(), b.encode()));
        data.fold = fold;
        data
    }

    /// The primary version, decoded fresh, with its packed byte length.
    pub(super) fn version(&self) -> Option<(Version, usize)> {
        let bytes = self.version.as_ref()?;
        Some((decode_version(bytes), bytes.len()))
    }

    /// Both versions decoded fresh, with their combined packed byte length.
    pub(super) fn version_pair(&self) -> Option<(Version, Version, usize)> {
        let (v, n) = self.version()?;
        let bytes2 = self.version2.as_ref()?;
        Some((v, decode_version(bytes2), n + bytes2.len()))
    }

    /// The disjoint party pair decoded fresh, with combined byte length.
    pub(super) fn party_pair(&self) -> Option<(Party, Party, usize)> {
        let (a, b) = self.parties.as_ref()?;
        Some((decode_party(a), decode_party(b), a.len() + b.len()))
    }

    /// The designated cross decoded fresh (event version, id party), with
    /// combined packed byte length.
    pub(super) fn cross(&self) -> Option<(Version, Party, usize)> {
        let (v, p) = self.cross.as_ref()?;
        Some((decode_version(v), decode_party(p), v.len() + p.len()))
    }

    /// One clock per shape, from the bundle's slots.
    ///
    /// A cross shape pairs its own id and event sides; a version-bearing
    /// shape pairs the seed party with the adversarial version; a
    /// party-only shape pairs the adversarial party with the empty
    /// version.
    pub(super) fn clock(&self) -> Option<(Clock, usize)> {
        if let Some((v, p, n)) = self.cross() {
            return Some((Clock::from_parts(p, v), n));
        }
        if let Some((v, n)) = self.version() {
            return Some((Clock::from_parts(Party::seed(), v), n + 1));
        }
        let (a, _, _) = self.party_pair()?;
        let n = self.parties.as_ref().map(|(a, _)| a.len())?;
        Some((Clock::from_parts(a, Version::new()), n + 1))
    }

    /// Two joinable clocks (disjoint parties), with combined operand bytes,
    /// from the bundle's slots.
    ///
    /// A shape with both a party pair and versions crosses them; a party-only
    /// pair rides empty versions; a version-only shape forks a seed pair around
    /// its version pair.
    pub(super) fn clock_pair(&self) -> Option<(Clock, Clock, usize)> {
        match (self.parties.is_some(), self.version.is_some()) {
            (true, true) => {
                let (a, b, np) = self.party_pair()?;
                let (v, w, nv) = self.version_pair()?;
                Some((Clock::from_parts(a, v), Clock::from_parts(b, w), np + nv))
            }
            (true, false) => {
                let (a, b, n) = self.party_pair()?;
                Some((
                    Clock::from_parts(a, Version::new()),
                    Clock::from_parts(b, Version::new()),
                    n + 2,
                ))
            }
            (false, true) => {
                let (v, w, n) = self.version_pair()?;
                let mut p = Party::seed();
                let q = p.fork();
                Some((Clock::from_parts(p, v), Clock::from_parts(q, w), n + 2))
            }
            (false, false) => None,
        }
    }
}

/// The disjoint-mount adapter: lift one packed id shape into a disjoint party
/// pair inside a single universe.
///
/// The pair mounts the shape under opposite children of a fresh root — `(shape,
/// ·)` and `(·, shape)` — so the halves are disjoint by construction and
/// joining them merely reunites the root's two subtrees: two
/// independently-generated id shapes are never asked to share a universe
/// (linearity of parties is the invariant everything rests on — the crate docs'
/// safety rules). Each half is the shape itself one level deeper, so party
/// cells on a mounted shape measure the shape plus one root tag. Runs at bundle
/// build, outside any measurement, and asserts the disjointness it mints.
fn disjoint_mounted_pair(id: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let shape = decode_party(id);
    let mount = |left: bool| -> Vec<u8> {
        let view = shape.as_bits();
        let mut bits = codec::BitsMut::with_capacity(view.len() as usize + 2);
        bits.push(left);
        bits.push(!left);
        codec::extend_from_view(&mut bits, view, 0, view.len());
        codec::seal_padding(&mut bits);
        bits.into_vec()
    };
    let (a, b) = (mount(true), mount(false));
    assert!(
        decode_party(&a).is_disjoint(&decode_party(&b)),
        "the disjoint-mount adapter must mint a disjoint pair"
    );
    (a, b)
}

/// The overlap-mount adapter: lift one packed id shape into an *overlapping*
/// party pair whose single shared region sits at both operands' preorder ends —
/// the disjoint-mount adapter's counterpart, for the rejection rows.
///
/// `a` mounts the shape under a fresh root's left child and a marker under its
/// right; `b` mounts the shape under the right child alone. The marker is a
/// single-child chain along the shape's rightmost-present path ending in a
/// terminal at the shape's preorder-last owned position, so the pair's one
/// overlap is the last position a lockstep walk over `b`'s side reaches, with
/// every earlier region disjoint — rejection consumes essentially both streams
/// before the witnessing pair meets.
///
/// The outputs are **semantically void by design**: a well-formed pair that no
/// legal fork/join history produces (two claims on one region), built on
/// purpose because the crate's cost claims are total — the rejection rows price
/// what rejecting such a pair costs, and nothing downstream treats the pair as
/// meaningful. Runs at bundle build, outside any measurement, and asserts the
/// overlap it mints (both halves decode canonically on the way).
pub(super) fn overlap_mounted_pair(id: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let shape = decode_party(id);
    let bits = shape.as_bits();
    let path = rightmost_terminal_path(bits);
    assert!(
        !path.is_empty(),
        "the overlap-mount adapter needs a non-terminal shape: a full shape's mount would \
         not be normal form"
    );
    let mut a = codec::BitsMut::with_capacity(bits.len() as usize + 2 * path.len() + 4);
    a.push(true); // root: both children present
    a.push(true);
    codec::extend_from_view(&mut a, bits, 0, bits.len()); // left: the shape
    for &go_right in &path {
        // right: the marker chain, one single-child node per level
        a.push(!go_right);
        a.push(go_right);
    }
    a.push(false); // the marker's terminal, at the shape's last owned position
    a.push(false);
    codec::seal_padding(&mut a);
    let mut b = codec::BitsMut::with_capacity(bits.len() as usize + 2);
    b.push(false); // root: right child only
    b.push(true);
    codec::extend_from_view(&mut b, bits, 0, bits.len()); // right: the shape
    codec::seal_padding(&mut b);
    let (a, b) = (a.into_vec(), b.into_vec());
    assert!(
        !decode_party(&a).is_disjoint(&decode_party(&b)),
        "the overlap-mount adapter must mint an overlapping pair"
    );
    (a, b)
}

/// The branch choices (`false` left, `true` right) from an id tree's root to
/// its preorder-last terminal: at every node, the last present child.
///
/// Preorder lays each subtree's bits contiguously, so the stream's final tag
/// belongs to the node reached by always taking the rightmost present child;
/// left subtrees along the way are skipped (each exactly once, so the walk is
/// linear). Runs at bundle build, outside any measurement.
fn rightmost_terminal_path(bits: codec::BitsView<'_>) -> Vec<bool> {
    let mut pos = 0u64;
    let mut path = Vec::new();
    loop {
        let left = bits.bit(pos);
        let right = bits.bit(pos + 1);
        pos += 2;
        if !left && !right {
            return path; // the terminal
        }
        if right {
            if left {
                pos = crate::idbits::skip_subtree(pos, |at| {
                    let children = u64::from(bits.bit(at)) + u64::from(bits.bit(at + 1));
                    (children, at + 2)
                });
            }
            path.push(true);
        } else {
            path.push(false);
        }
    }
}

/// The overlap fold's probe: a right-mounted full leaf — `(0, 1)`, one packed
/// byte — overlapping the a-mount's whole right half (the marker's region).
///
/// The `party_join_all_overlap` row's per-input operand. The witnessing pair
/// sits in the right half, behind the accumulator's whole left shape, so a
/// per-input overlap test priced in the accumulator — a cursor walk
/// skip-scanning the left shape to reach the witness — reads Θ(accumulator)
/// scan per O(1)-byte input and turns the row quadratic; the fold's per-call
/// accumulator index answers the same test in O(probe), which is the separation
/// the row watches.
pub(super) fn overlap_fold_probe() -> Vec<u8> {
    let mut probe = codec::BitsMut::with_capacity(4);
    probe.push(false); // root: right child only
    probe.push(true);
    probe.push(false); // the right child: a full leaf
    probe.push(false);
    codec::seal_padding(&mut probe);
    probe.into_vec()
}

/// Decode packed bytes the board itself generated.
pub(super) fn decode_version(bytes: &[u8]) -> Version {
    Version::decode(bytes).expect("board-generated version bytes are canonical")
}

/// Decode packed party bytes the board itself generated.
pub(super) fn decode_party(bytes: &[u8]) -> Party {
    Party::decode(bytes).expect("board-generated party bytes are canonical")
}

/// A tiny xorshift64 generator: deterministic, dependency-free randomness
/// for the benign control family.
struct XorShift(u64);

impl XorShift {
    /// The next pseudo-random word.
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// Every packed version in each board family's operand bundle at `scale`, level
/// 0, named per family: the shape corpus exactly as the board builds it.
///
/// A measure-only study surface for offline payload analysis — the `code_study`
/// example re-parses these stored streams into per-class integer histograms to
/// price candidate integer codes on the adversarial corpus. Nothing on the
/// board consumes it.
pub fn study_family_versions(scale: f64) -> Vec<(&'static str, Vec<Vec<u8>>)> {
    FamilyId::board()
        .map(|kind| {
            let data = FamilyData::build(kind, scale, 0);
            let mut versions = Vec::new();
            versions.extend(data.version.clone());
            versions.extend(data.version2.clone());
            if let Some((vs, _)) = &data.fold {
                versions.extend(vs.iter().cloned());
            }
            (data.name, versions)
        })
        .collect()
}
