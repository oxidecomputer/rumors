//! The shape axis: every input family and the operand bundle it builds.
//!
//! Every shape comes from the [`meter`] generators; the
//! roster is [`FAMILIES`], and the bundle post-pass in
//! [`FamilyData::build`] derives uniformly every slot a shape does not
//! natively fill, so a shape reaches every operation its bundle supplies
//! (the board module doc's product section) without naming any.
//!
//! The carriers: the event shapes —
//! the dense spine, `bigroot`, `hugeleaf`, the boundary comb (`cliff`, at
//! `k = n` so its value content grows quadratically in its packed input),
//! `harmonic`, `freeze-pos` (the many-freezes spine, one query-fold
//! freeze per block), `promo-rearm` (the many-armings spine, one
//! query-fold promotion per block), `weight-comb` (the many-jumps
//! spine, one accumulator-top jump and settle per block pair),
//! `freeze-parade` (the deep-segment freeze spine, one scaled segment
//! read per block), `plateau-puncture` (the answer-embedded product:
//! the exact rank is
//! one wide × dense multiplication), and `lone-freeze` (the
//! first-freeze gate straddle, a never-freezing plateau prefix and a
//! frozen tail on one knob) — carry a version; the diverted id-spine
//! pair carries a
//! disjoint party pair; the eleven cross shapes (`comb-scatter` and the
//! ten tick-walk crosses) carry a version, a mounted party pair, and a
//! clock; the two version-pair shapes — `jump-pair` (wide
//! height-difference crests over a dense-position spine) and
//! `concurrent-pair` (the switch-density population) — carry a version
//! pair of their own construction, so
//! their comparison rows run the pairing the shape was built around
//! rather than the ticked counterpart, and `tooth-tail` (the
//! boundary-aligned exact-`top` pair) and `dense-suffix` (the
//! many-armings re-arm spine over its unit mate) carry their generator
//! pairs the same
//! way; the three fold populations —
//! `scatter`, `weave`, and `stagger` — carry fold operands alone, so
//! exactly the three
//! fold rows run on them; `benign` — a fixed-seed pseudo-random population of forked,
//! ticked clocks, the control row that keeps the ceilings honest on
//! organic inputs — carries everything. Where an operation needs a
//! `Party` and a `Version`, the board crosses adversarial party × small
//! version, small party × adversarial version, and — on the cross
//! shapes — the designated adversarial × adversarial pairing.
//!
//! The recurring carrier classes, named here because no single
//! declaration spells them out: the 27
//! version-carrying shapes (all but `id-pair` and the three fold
//! populations) run every version row; the party-pair carriers
//! (`id-pair`, `comb-scatter`, the ten tick crosses, `benign`) run the
//! party rows; every clock-carrying shape (the version carriers plus
//! `id-pair`) runs the clock rows; the projection rows add the
//! output-domination cross; and the three fold populations (`scatter`,
//! `weave`, `stagger`) plus the `benign` control carry fold operands,
//! so exactly the fold rows run on them.
//!
//! This list is deliberately narrower than the generator surface: a shape
//! earns a board column only as a whole-surface adversary, while
//! kernel-seam probes live in the envelope suite alone. The criterion and
//! the add-a-shape touch list sit on the `FAMILIES` roster below.
//!
//! Ten shapes carry a genre note beyond their variant docs:
//!
//! - `freeze-pos`, built against the linear-functional rows: `Θ(s)`
//!   query-fold freezes at ever-deeper stream positions where every
//!   comb fires O(1). The committed known-bad kernel (the query fold's
//!   adequacy tripwire) reads ×1.50 per byte across this family's
//!   doubling, so a green `version_rank × freeze-pos` cell is a live
//!   verdict, not decoration.
//!
//! - `promo-rearm`, built against the linear-functional rows: `Θ(s)`
//!   query-fold promotions at O(1) stored codes each, over a consumed
//!   mass whose written span the spine keeps growing — the coverage
//!   hole freeze-pos left, its parked drift being monotone (no
//!   committed family promoted at all). The committed known-bad kernel
//!   (the query fold's span-promotion tripwire) reads ×1.74 per byte
//!   across this family's doubling, so a green
//!   `version_rank × promo-rearm` cell is a live verdict, not
//!   decoration — and the class-binding seal that holds `Linear`
//!   claims against exponent-mechanism reds is live for the promotion
//!   mechanism exactly because this column exists.
//!
//! - `weight-comb` and `freeze-parade`, the accumulator skip-mechanism
//!   families (the zero-run certificate ledger's and the write
//!   watermark's, respectively): each is a public-API stream that
//!   stays flat only through its mechanism, and each mechanism's
//!   absence reads ~×2 per byte across the family's doubling (the
//!   committed probe-build measurements in the `skyline_flatness` band
//!   ceilings, `tests/meter.rs` — the enforcement stays there; the
//!   columns exist so the dashboard is never structurally blind to the
//!   genre, every cell a live verdict over the mechanism that holds it
//!   flat).
//!
//! - `tooth-tail`, the third skip mechanism's family (exact-`top`
//!   maintenance): the boundary-aligned pair whose cancelled spike
//!   leaves `Θ(m)` post-cancellation sign reads over a `g`-digit dead
//!   buffer — flat with the settled top, `Θ(m·g)` with a high-water
//!   bound (the `skyline_flatness` tooth-tail band carries both
//!   readings; enforcement stays there). The pair is also the
//!   committed demonstration behind the comparison rows' per-boundary
//!   touch floor: same-shape operands share every overlay boundary,
//!   so the fused sweep honestly folds ~once per boundary against two
//!   stored deltas, and its parse rows are the board's densest
//!   node-per-text-byte streams (the family-declared parse heap
//!   ceiling at [`TOOTH_TAIL_PARSE_HEAP_BYTES_PER_TEXT_BYTE`](super::ceilings::TOOTH_TAIL_PARSE_HEAP_BYTES_PER_TEXT_BYTE) carries
//!   the derivation).
//!
//! - `comb-scatter`: the projection cross (boundary-comb version ×
//!   scattered party) whose mandatory output dominates its input — the
//!   case the small-operand crosses cannot exhibit; its two projection
//!   cells are the board's only I/O-denominated non-text cells.
//! - `harmonic` (`meter::harmonic`, a 1-leaf at every depth), built
//!   against the linear-functional rows (`rank`/`distance`/`lag`/
//!   `min_ticks`) and the rank rows (`rank_pair_ops`, `rank_sum`): its
//!   rank's numerator is as
//!   wide as the depth already walked at every level, so a fold that
//!   re-shifts its accumulated numerator per level reads limb exponent ~2
//!   here while `dense` (a one-bit numerator) stays the linear control.
//!   The query kernels' rank fold telescopes through height deltas, and
//!   `version_rank × harmonic` reads the control's linear signature
//!   \[measured — limb exponent 1.00, constant within 2% of `dense`, both
//!   scales\]: the column is the tripwire that goes red under the
//!   re-shifting genre.
//! - `scatter`, whose bundle carries fold operands alone, for the three
//!   fold rows (`version_join_all`, `version_meet_all`,
//!   `party_join_all`; all also keep a `benign` control cell, folding the
//!   organic population in construction order): balanced-forked
//!   single-tick operands ordered evens before odds, so a sequential
//!   fold's accumulator holds every other leaf and never coalesces — the
//!   shape that reads exponent ~2 under a left fold. Both rows run the
//!   balanced binary-counter reduction (every input passes through
//!   O(log n) joins), and what the cells show is its log factor — on the
//!   version fold's limb and scan columns, and on the party fold's scan
//!   column alone (its walk allocates nothing, recurses nothing, and does
//!   no arithmetic, so scan is the only deterministic meter that sees
//!   it): exponents ~1.1 and constants that grow with scale, marginally
//!   over the amortized-linear bounds at some scales \[measured — both
//!   scales\]. The `benign` controls read the same
//!   signature as `scatter`, so the readings priced by the declared fold
//!   model are the reduction's own n·log n cost, not the adversarial
//!   ordering's.
//! - `weave`, the correlated fold population (the leaves of one balanced
//!   fork expansion dealt round-robin among 16 group parties,
//!   one tick each), also fold-rows-only: every operand pair is
//!   both-present at the whole shared upper skeleton while each operand
//!   alone is an organic region set, so the per-node fold costs that
//!   scale with the *other* operand — the overlap test against the
//!   accumulator above all — dominate at fixed arity. Scatter's
//!   single-leaf operands cannot reach the genre and benign reaches it
//!   only diluted.
//! - `stagger`, the reduction-loading fold population, also
//!   fold-rows-only: `n` operands of `m` unit teeth each, every
//!   operand's teeth in the gaps of every other's, fed in
//!   bit-reversed order so each binary-counter merge pairs operand
//!   groups whose slots diverge at the top address bit — every
//!   internal merge, at every level, swells to near the sum of its
//!   inputs' sizes, the declared `O(D log k)` model's intermediate-
//!   swell worst case with no coalescing until the last level.
//!   Scatter scales arity at single-leaf operands and weave scales
//!   operand size at fixed arity; this population is the joint axis
//!   \[measured — scan 8.6 bits/B per reduction level, constant
//!   across `n` and `m` doublings alike, under the declared 12\].

use crate::codec;
use crate::meter;
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
const HUGELEAF_BASE_MAGNITUDE_BITS: usize = 16_000;

/// Id spine depth at scale 1.0 (packed pair ~6 KiB).
const ID_BASE_DEPTH: usize = 12_000;

/// Boundary-comb tooth magnitude (bits) and tooth count at scale 1.0
/// (packed size ~4 KiB); one parameter drives both, mirroring the meter
/// suite's `k = n` convention.
///
/// Scaling `k` with `n` is the separating choice: it keeps the comb's
/// absolute value content growing quadratically in the packed input, so a
/// sweep that materializes running leaf values in a plain big integer
/// reads a superlinear exponent here instead of hiding a `k`-sized
/// constant under a fixed magnitude.
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
/// Deep enough that a per-level re-scan genre reads its exponent
/// across the level doubling, small enough that the quadratic pin
/// stays inside the board's runtime budget at the acceptance scale.
const NESTED_BASE_DEPTH: usize = 1_500;

/// Nested-wide depth and root-magnitude bits at scale 1.0 (equal, so
/// the doubling scales width and depth together — the cross's cost
/// genre is their product; packed pair ~1.5 KiB).
///
/// Small enough that even a width × depth kernel stays inside the
/// acceptance-scale runtime budget; the red reading rides the exponent
/// leg, not the constant ceiling.
const NESTED_WIDE_BASE: usize = 1_000;

/// Mirror-wide depth and tail-magnitude bits at scale 1.0 (equal, as
/// above; packed pair ~1 KiB). The memo arm's chains grow steeper than
/// the right-full arm's, so the base sits lower.
const MIRROR_WIDE_BASE: usize = 500;

/// Mirror-narrow depth at scale 1.0 (packed pair ~1.5 KiB): the
/// nested-full base, mirrored — the memo machinery at the same depth
/// the right-full cells walk.
const MIRROR_NARROW_BASE_DEPTH: usize = 1_500;

/// Staircase depth at scale 1.0 (packed pair ~2 KiB): deep enough that
/// per-level minimum bookkeeping would read its exponent across the
/// doubling, all values word-scale.
const STAIRCASE_BASE_DEPTH: usize = 1_500;

/// Reveal-comb site count and plateau-magnitude bits at scale 1.0
/// (equal; packed pair ~1 KiB).
///
/// One parameter drives both, so the doubling scales the site count
/// and the circulated width together — the cycle's cost genre is
/// their product. The close-reveal cycle's per-site cost is steeper
/// than the mirror families' chains, so the base sits at the
/// mirror-wide level.
const REVEAL_COMB_BASE: usize = 500;

/// Pure-comb level count and leaf-magnitude bits at scale 1.0 (equal,
/// as above; packed pair ~1 KiB).
///
/// The base watermark stack's own cycle runs at ~2 wide folds per
/// level — a tenth of the reveal comb's constant — so the base sits
/// higher for comparable work.
const PURE_COMB_BASE: usize = 1_000;

/// Ascending-cliff spine length and leaf-magnitude bits at scale 1.0
/// (equal, so the doubling scales the hop count and the residue width
/// together — the cascade's cost genre is their product; packed pair
/// ~1 KiB).
///
/// The cascade runs at ~4 touches per input byte on the cured fold
/// direction — the leveled control's constant — so the base sits at
/// the pure-comb level for comparable work.
const ASCEND_CLIFF_BASE: usize = 1_000;

/// Ticks behind the integer (exponent-zero) rank of the `rank_pair_ops`
/// row: small, so the pair's cost is carried entirely by the mismatch.
const RANK_PAIR_INTEGER_TICKS: u64 = 3;

/// Probes per accumulator byte (as a divisor) on the
/// `party_join_all_overlap` row.
///
/// The probe count scales with the accumulator so the row's exponent
/// judges the fold against a denominator both sides of which double
/// together — work scaling with the fixed accumulator per input reads
/// quadratic there — and the divisor keeps the row inside the board's
/// runtime budget.
pub(super) const OVERLAP_FOLD_INPUT_DIVISOR: usize = 64;

/// Two-operand jump-comb teeth at scale 1.0 (packed pair ~35 KiB, the
/// teeth operand's per-level wide codes dominating).
///
/// One knob drives the tooth count and, through
/// [`JUMP_PAIR_DIGIT_DIVISOR`], the isolated-position digit count, at
/// the fixed tooth magnitude [`JUMP_PAIR_MAGNITUDE_BITS`]: an
/// absolute-position freeze accounting pays teeth × digits × magnitude
/// here, so the doubling scales the crest count and the position
/// density together while the packed pair grows linearly — the
/// separating choice that makes any such accounting read on the
/// exponent leg rather than hide in a constant.
const JUMP_PAIR_BASE_TEETH: usize = 256;

/// Tooth magnitude (bits) of the two-operand jump comb, fixed across
/// scales: comfortably over the freeze allowance's 256-bit digit bound,
/// so every cheap fold behind a wide difference crest parks the drift.
const JUMP_PAIR_MAGNITUDE_BITS: usize = 512;

/// Isolated-position digits per tooth (as a divisor) on the two-operand
/// jump comb.
///
/// The digit count scales with the teeth at an eighth: deep enough that
/// any per-freeze absolute-position work reads its exponent across the
/// doubling, shallow enough that the shared spine stays a small
/// fraction of the packed pair.
const JUMP_PAIR_DIGIT_DIVISOR: usize = 8;

/// Freeze-position blocks at scale 1.0 (packed version ~74 KiB, the
/// per-block wide drop codes dominating).
///
/// The scale of the `skyline_flatness` freeze-position band's small
/// run: the committed known-bad accounting reads ×1.50 per-byte growth
/// across this regime's doubling (the adequacy tripwire's committed
/// measurement), so the board's default pair straddles exactly what the
/// family exists to catch. The base is a multiple of 16 deliberately:
/// the family's rank exponent is `2s − 1` (one trailing zero strips —
/// exactly one leaf term, the odd `2^L + 1` at weight `2^1`, has
/// 2-adic valuation one), and `rank_sum` lands each small summand at
/// bit remainder `exp mod 32`, where a remainder near the digit top
/// makes most landings span two digits instead of one — an honest
/// amortized-O(1) constant, but one that flips with the remainder, and
/// an exponent fitted across two scales with different remainders
/// reads the flip as growth (measured: e 1.65 from a 1.0 → 1.57
/// per-summand constant at remainders 15 → 31). `16 | s` keeps
/// `2s ≡ 0 (mod 32)`, so every doubling preserves the remainder and
/// the exponent leg compares like against like.
const FREEZE_POS_BASE_BLOCKS: usize = 1_024;

/// Promotion re-arm blocks at scale 1.0 (packed version ~128 KiB, the
/// per-block wide arming codes dominating).
///
/// Half the `skyline_flatness` promotion re-arm band's small run: the
/// committed span-reading promotion reads ×1.74 per-byte growth across
/// that regime's doubling (the span-promotion tripwire's committed
/// measurement), so the board's default pair straddles what the family
/// exists to catch. The base is a multiple of 8 deliberately: the
/// family's rank exponent is `36s`, and `rank_sum` lands its small
/// summands at bit remainder `exp mod 32` (an honest amortized-O(1)
/// constant that flips with the remainder — the freeze-position base's
/// derivation carries the mechanism); `8 | s` keeps `36s ≡ 0 (mod 32)`,
/// so every doubling compares like against like.
const PROMO_REARM_BASE_BLOCKS: usize = 512;

/// Weight-comb block pairs at scale 1.0 (packed version ~7 KiB, the
/// spine's unit codes dominating), rounded up to a power of two at
/// every scale.
///
/// The rounding is the complete-subtree relation's call-site repair,
/// and the level doubling then doubles the rounded count exactly.
/// The base is the scale of the `skyline_flatness` weight-comb band's small run:
/// with certificate consumption disabled, rank reads ×1.93 per-byte
/// growth across this regime's doubling (the band ceiling doc's
/// committed probe-build measurement), so the board's default pair
/// straddles exactly what the family exists to catch. Power-of-two `n`
/// keeps the spine depth `32n ≡ 0 (mod 32)`, so `rank_sum` lands its
/// small summands at the same bit remainder at both scales and the
/// exponent leg compares like against like (the freeze-position base's
/// derivation carries the mechanism).
const WEIGHT_COMB_BASE_BLOCKS: usize = 512;

/// Freeze-parade blocks at scale 1.0 (packed version ~49 KiB, the
/// blocks' wide drop codes dominating), rounded up to a power of two
/// at every scale (the parade block is one complete subtree).
///
/// The scale of the `skyline_flatness` freeze-parade band's small run:
/// with the write watermark disabled, rank reads ×1.91 per-byte growth
/// in the touch and limb currencies together across this regime's
/// doubling (the band ceiling doc's probe-build measurement of
/// record). Power-of-two `k` keeps the spine depth `64k ≡ 0 (mod 32)`,
/// the same `rank_sum` remainder alignment as the weight comb's.
const FREEZE_PARADE_BASE_BLOCKS: usize = 512;

/// Concurrent-pair forked-party count at scale 1.0, rounded up to a
/// power of two at every scale (the balanced fork and the alternating
/// dominance schedule both need it; the level doubling then doubles it
/// exactly).
const CONCURRENT_BASE_LEAVES: usize = 1_024;

/// Tooth-tail boundary count at scale 1.0 (the pair ~3.6 KiB per
/// operand); the spike width rides the same knob at
/// [`TOOTH_TAIL_SPIKE_DIVISOR`], the committed flatness band's ratio.
const TOOTH_TAIL_BASE_BOUNDARIES: usize = 4_096;

/// Dense-suffix blocks (and gap digits: one knob drives both, the
/// `DS(p, p)` diagonal) at scale 1.0 (packed version ~122 KiB, the
/// blocks' wide climb codes dominating).
///
/// The scale of the `skyline_flatness` dense-suffix bands' small run:
/// the committed per-arming suffix walk reads ×1.96 per-byte growth
/// across that regime's doubling (the query fold's committed
/// tripwire), so the board's default pair straddles what the family
/// exists to catch. The base is a multiple of 32 deliberately: the
/// family's rank exponent is linear in the knob, and `rank_sum` lands
/// its small summands at bit remainder `exp mod 32` (an honest
/// amortized-O(1) constant that flips with the remainder — the
/// freeze-position base's derivation carries the mechanism); `32 | s`
/// keeps any integer-linear exponent's remainder fixed across the
/// level doubling, so the exponent leg compares like against like.
const DENSE_SUFFIX_BASE_BLOCKS: usize = 512;

/// Plateau-puncture digits (plateau digits and turn count together:
/// the `PP(w, d)` diagonal at `w = d`) at scale 1.0 (packed version
/// ~21 KiB).
///
/// The scale of the `skyline_flatness` plateau-puncture band's small
/// run: the committed schoolbook settle reads ×1.90 per-byte growth
/// across that regime's doubling (the query fold's committed
/// tripwire), so the board's default pair straddles what the family
/// exists to catch. A multiple of 32 for the same `rank_sum`
/// remainder alignment as [`DENSE_SUFFIX_BASE_BLOCKS`]; the build arm
/// floors the knob at the generator's minimum width (the plunge must
/// trip the freeze allowance past a unit code), which binds only
/// under extreme scale-down.
const PLATEAU_PUNCTURE_BASE_DIGITS: usize = 512;

/// Lone-freeze oscillation pairs per axis (the `LF(s, s)` diagonal:
/// one knob drives the never-freezing plateau prefix and the frozen
/// tail together) at scale 1.0 (packed version ~2.6 KiB).
///
/// The scale of the `skyline_flatness` lone-freeze bands' small runs
/// (each band isolates one axis at the generator minimum; the board
/// column scales both, so a regression on either side reads on the
/// doubling). A multiple of 32 for the same `rank_sum` remainder
/// alignment as [`DENSE_SUFFIX_BASE_BLOCKS`], kept even at every
/// scale by the build arm (the generator counts whole oscillation
/// pairs).
const LONE_FREEZE_BASE_PAIRS: usize = 2_048;

/// Boundaries per spike digit in the tooth-tail bundle.
///
/// The committed flatness band's `g = m/64` ratio, so the board prices
/// the same spike-to-tail proportion the envelope band holds flat (a
/// spike a few wide digits under thousands of post-cancellation sign
/// reads — the exact-top genre needs the tail to dominate the spike).
const TOOTH_TAIL_SPIKE_DIVISOR: usize = 64;

/// Staggered fold population operand count at scale 1.0, rounded up to
/// the power of two the slot addressing needs.
///
/// 64 operands of [`STAGGER_BASE_BLOCKS`] teeth each keep the
/// default-scale fold cells seconds-fast while giving the balanced
/// reduction seven levels of intermediate swell; the level doubling
/// doubles both knobs, so the two board scales move the arity and the
/// per-operand size together and the declared fold model's exponent
/// ceiling is computed from the realized pair.
const STAGGER_BASE_OPERANDS: usize = 64;

/// Staggered fold population blocks (teeth) per operand at scale 1.0,
/// rounded up to a power of two (the complete top tree needs it).
const STAGGER_BASE_BLOCKS: usize = 64;

/// Benign clock population at scale 1.0.
const BENIGN_BASE_CLOCKS: usize = 256;

/// The weave fold population's leaf count at scale 1.0 (rounded up to a
/// power of two by construction).
///
/// 4096 leaves woven into [`WEAVE_GROUPS`] parties give each operand
/// ~256 scattered leaves under a fully shared upper skeleton — deep
/// enough that the both-present-rich cost terms (the indexed overlap
/// test's per-node table searches, the version joins' interleaved
/// merges) dominate each cell, small enough that the default-scale
/// board stays seconds-fast.
const WEAVE_BASE_LEAVES: usize = 4_096;

/// How many parties the weave population folds: fixed across scales, so
/// the family's scaling axis is operand *size* (the both-present
/// richness per merge), not arity — scatter and benign already own the
/// arity axis.
///
/// 16 groups keep every internal node of the shared skeleton above the
/// last four levels both-present in every operand while each operand
/// stays an organic, individually well-formed region set.
const WEAVE_GROUPS: usize = 16;

/// Floor on every scaled size parameter, so extreme scale-down (the smoke
/// test) still builds valid shapes and a nonempty benign population.
///
/// The floor preserves positivity, never a *relation* between two
/// parameters (an even count, a width strictly under a depth, an ascent
/// inside a code band). Shapes whose generator asserts a relation
/// therefore drive every related parameter from one knob — the
/// `ascend_cliff(s, s)` / `reveal_comb(s, s)` convention — or repair at
/// the call site as [`CONCURRENT_BASE_LEAVES`]'s power-of-two rounding
/// does; a two-knob shape with a floored relation panics in the smoke
/// test's extreme scale-down.
pub(super) const MIN_SIZE_PARAM: usize = 4;

/// Fixed seed for the benign family's pseudo-random construction: the
/// control row must be deterministic run to run.
const BENIGN_RNG_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

// ─── input families ─────────────────────────────────────────────────────────

/// The input families, one column group of the matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FamilyKind {
    /// The dense event spine `S(d)`: node count and depth maximizer.
    Dense,
    /// `bigroot(B, d)`: a huge root magnitude over a long spine.
    Bigroot,
    /// `hugeleaf(B)`: one node, maximal bits per node.
    Hugeleaf,
    /// The boundary comb `C(k, n)` at `k = n`: leaf values oscillating
    /// across a `2^k` carry cliff, every crossing paid by a stored code.
    Cliff,
    /// The diverted id-spine pair `I(d, ·)`: full-lockstep two-party walks.
    IdPair,
    /// The output-domination cross: boundary comb × scattered party.
    CombScatter,
    /// The harmonic spine `H(d)`: the rank fold's wide-numerator
    /// adversary, designed against the linear-functional rows and the
    /// rank pair.
    Harmonic,
    /// The scatter-ordered fold population: balanced-forked single-tick
    /// operands whose join accumulator never coalesces; its bundle
    /// carries fold operands alone, so only the fold rows apply.
    Scatter,
    /// The weave fold population: the leaves of one balanced fork tree
    /// dealt round-robin among [`WEAVE_GROUPS`] parties, one tick each.
    ///
    /// Every operand is individually benign — an organic region set any
    /// retire/reunite call site could hold — while every internal node
    /// of the shared upper skeleton is both-present in every operand
    /// pair, so the fold's per-node costs that scale with the *other*
    /// operand (the overlap test against the accumulator, the join
    /// merges over interleaved trees) dominate. Scatter cannot reach
    /// this genre (its operands are single leaves) and benign reaches
    /// it only diluted; the arity is fixed so the scaling axis is
    /// both-present richness alone. Its bundle carries fold operands
    /// alone, so only the fold rows apply.
    Weave,
    /// The staggered fold population `stagger_population(n, m)`: `n`
    /// operands of `m` unit teeth each, every operand's teeth landing
    /// in the gaps of every other's, fed in bit-reversed order.
    ///
    /// The correlated-population loading of the balanced reduction
    /// itself: the feed order pairs operands whose slot addresses
    /// diverge at the top bit, so every internal merge — at every
    /// level — joins region sets that interleave maximally and swell
    /// to near the sum of their sizes, the intermediate-swell worst
    /// case of the declared `O(D log k)` fold model, held until the
    /// last level (the full union collapses to the constant-1 skyline
    /// on the version side, the whole seed region on the id side).
    /// Scatter scales arity at single-leaf operands and weave scales
    /// operand size at fixed arity; this population scales both, and
    /// its bit-reversed feed forecloses the adjacent-slot coalescing
    /// luck index order would hand the counter. Its bundle carries
    /// fold operands alone, so only the fold rows apply.
    Stagger,
    /// The nested-full-sibling cross `N(d)` × the dense spine `S(d)`.
    ///
    /// Every level a right-full shortcut site, the deepest stacking of
    /// the walk's deferred right-full decisions and raise bookkeeping
    /// on narrow values — the designated cross of the two tick rows.
    NestedFull,
    /// The wide right-full cross: `bigroot(b, d)` × `N(d)`.
    ///
    /// The stream's first payload is coded absolute, so the deepest
    /// subtree's net movement carries the root's full magnitude and
    /// every level's bookkeeping meets it — width × depth through the
    /// right-full arm. The designated cross of the two tick rows.
    NestedWide,
    /// The wide left-full (memo) cross: `wide_tail(b, d)` × `M(d)`.
    ///
    /// Every proper subtree nets the tail's full magnitude while every
    /// level is a memoized pre-scan site — width × depth through the
    /// left-full arm and the pre-scan's own chains. The designated cross
    /// of the two tick rows.
    MirrorWide,
    /// The narrow left-full (memo) cross: `wide_tail(1, d)` × `M(d)`.
    ///
    /// The memoized pre-scan machinery itself, all values word-scale.
    /// The designated cross of the two tick rows.
    MirrorNarrow,
    /// The descending staircase `D(d)` × the unary id spine `I(d)`.
    ///
    /// Every consumed leaf undercuts every open range's minimum —
    /// full-penetration minimum updates at every level, all values
    /// word-scale. The designated cross of the two tick rows.
    Staircase,
    /// The reveal-comb cross: `reveal_comb(s, s)` × its own id.
    ///
    /// `s` sibling left-full sites share one `2^s`-wide minimum over a
    /// zero floor, and the left-leaning spine closes each site's frame
    /// back into the floor frame between consecutive consumes: the
    /// width-`s` boundary difference is minted at every consume and
    /// popped at every close — the unfunded width circulation, in the
    /// touch currency these columns do not carry (the gate pins in
    /// `tests/meter.rs` enforce it; the bench mirror's time leg sees
    /// it). The designated cross of the two tick rows.
    RevealComb,
    /// The reveal-comb control: `reveal_comb_hifloor(s, s)` × the
    /// reveal-comb id.
    ///
    /// Identical forest and close-reveal cycle with the floor raised
    /// to `2^s − 2`, so the circulated boundary difference is O(1)
    /// wide: the gap control. The designated cross of the two tick rows.
    RevealHifloor,
    /// The pure-comb cross: `pure_comb(s, s)` × its own id.
    ///
    /// The reveal comb's cycle with no left-full site anywhere — no
    /// memo, no pre-scan, no site consume: the base watermark stack's
    /// own arm-move + close-pop width circulation, isolated from the
    /// frame ledger. The designated cross of the two tick rows.
    PureComb,
    /// The ascending-cliff cross: `ascend_cliff(s, s)` × its own id.
    ///
    /// `s` ascending wide leaves stack `s − 1` nonzero unit boundary
    /// differences and a terminal 0-cliff drives one width-`s` undercut
    /// residue through all of them — the cascade whose per-hop fold
    /// direction the gate pins in `tests/meter.rs` price in the touch
    /// currency these columns do not carry. The designated cross of the
    /// two tick rows.
    AscendCliff,
    /// The ascending-cliff control: `ascend_cliff_plateau(s, s)` × the
    /// ascending-cliff id.
    ///
    /// Identical spine, arming schedule, and cliff undercut with every
    /// leaf leveled, so the difference stack is one compressed zero run
    /// the residue passes whole in O(1): the hop-schedule control.
    /// The designated cross of the two tick rows.
    AscendPlateau,
    /// The two-operand jump comb `jump_pair(k, m, d)`: wide
    /// height-difference crests over a dense-position spine.
    ///
    /// The overlay interleaves one operand's wide teeth with the
    /// other's cheap codes, so the pair rows park wide drift at the
    /// other operand's boundaries `2m` times while every absolute
    /// position stays `d` digits dense — the shape that separates
    /// segment-anchored freeze accounting (flat) from absolute-position
    /// accounting (superlinear), with each operand certified-linear
    /// alone (the generator doc carries the mechanism).
    JumpPair,
    /// The freeze-position spine `freeze_position(s)`: the
    /// many-freezes sentinel.
    ///
    /// `2s` descending wide leaves alternate a ten-digit drop and a
    /// unit drop down a right spine, so a query fold freezes `Θ(s)`
    /// times at ever-deeper stream positions — every comb fires O(1)
    /// freezes, which was exactly the coverage hole — and any freeze
    /// accounting that reads an absolute position (or any
    /// whole-history state) per freeze goes quadratic here while the
    /// family's positions compact to O(1) digits. The committed
    /// known-bad kernel reads ×1.50 per byte across the doubling on
    /// this shape (the query fold's adequacy tripwire); the
    /// anchored-segment discipline reads flat (the `skyline_flatness`
    /// freeze-position band). Designed against the linear-functional
    /// query rows.
    FreezePos,
    /// The promotion re-arm spine `promotion_rearm(s)`: the
    /// many-armings sentinel.
    ///
    /// `32s` span-building levels grow the consumed mass's written
    /// span, then `s` four-node blocks each park a wide drift and
    /// promote it at a narrow one — `Θ(s)` query-fold promotions at
    /// O(1) stored codes each, where every comb promotes never and the
    /// freeze-position spine's parked drift is monotone. Any promotion
    /// accounting that re-reads whole-history state per arming goes
    /// quadratic here while the family's suffix masses compact to O(1)
    /// balanced terms. The committed known-bad kernel reads ×1.74 per
    /// byte across the doubling on this shape (the query fold's
    /// span-promotion tripwire); the promotion ledger reads flat (the
    /// `skyline_flatness` promotion re-arm bands). Designed against
    /// the linear-functional query rows.
    PromoRearm,
    /// The weight-comb spine `weight_comb(n)`: the many-jumps
    /// sentinel.
    ///
    /// A depth-`32n` parked-unit spine, then `2n` shallow leaves
    /// oscillating heights 0 and 2: the rank integral deposits the
    /// oscillation at one digit position `Θ(n)` digits above the
    /// parked unit for O(1) stored bits per event — the position
    /// weight is topology, so no code funds the gap — and every
    /// cancellation makes the accumulator's top settle back across the
    /// never-written run. A settlement scan that steps the gap digit
    /// by digit goes quadratic here (×1.93 per byte across the
    /// doubling, measured under a probe build with certificate
    /// consumption disabled); consuming one zero-run certificate per
    /// jumped run reads flat (the `skyline_flatness` weight-comb
    /// band). Designed against the linear-functional query rows.
    WeightComb,
    /// The freeze-parade spine `freeze_parade(k)`: the deep-segment
    /// freeze sentinel.
    ///
    /// The parked-unit spine at depth `64k`, then `k` shallow freeze
    /// blocks whose wide in-pair drops each fire one query-fold freeze
    /// at the block's position weight, `Θ(k)` digits above digit 0, so
    /// every freeze's scaled segment read starts `Θ(k)` digits up. The
    /// accumulator's write watermark prices each read at the segment's
    /// written span; a scaled read that starts at digit 0 walks the
    /// never-written prefix per freeze and goes quadratic in the touch
    /// and limb currencies together (×1.91 per byte across the
    /// doubling, measured under a probe build whose scaled reads start
    /// at digit 0); the watermark reads flat (the `skyline_flatness`
    /// freeze-parade band). The freeze-position spine prices the query
    /// layer's per-freeze accounting; this family prices the
    /// accumulator's read side under the same schedule. Designed
    /// against the linear-functional query rows.
    FreezeParade,
    /// The concurrent pair `concurrent_pair(n)`: the emit side-switch
    /// density population.
    ///
    /// Organically forked and ticked so the sweep's side switch fires at
    /// every one of the `n − 1` overlay boundaries, join and meet alike
    /// — the pairing the ticked counterpart cannot reach.
    ConcurrentPair,
    /// The dense-suffix pair `dense_suffix(p, p)` against its unit
    /// mate `dense_suffix_mate(p, p)`: the many-armings ×
    /// dense-trailing-mass sentinel.
    ///
    /// A gap spine holds the trailing interval mass at `Θ(p)` balanced
    /// digits, then `p` re-arm blocks each park a wide drift and
    /// promote it at O(1) stored codes — `Θ(p)` ledger armings all
    /// owing their debt across the same `Θ(p)`-dense trailing mass, so
    /// a settle that walks the suffix once per arming (or re-reads a
    /// promoted prefix once per window) goes quadratic here while the
    /// mass-balanced product tree charges every arming-window cross
    /// term inside one aggregate product and reads flat. The committed
    /// tripwire beside the kernel
    /// (`suffix_walk_settle_reads_superlinear_on_dense_suffix`, the
    /// query fold's test suite) keeps the per-arming walk failing on
    /// this family. The mate is the same topology at unit bases, and
    /// the wide operand dominates it pointwise, so the pair rows run
    /// the co-sweep whose freezes and promotions fire on drift only
    /// the wide operand deposited (the `skyline_flatness` dense-suffix
    /// rank and distance bands carry the enforcement). Designed
    /// against the linear-functional query rows.
    DenseSuffix,
    /// The plateau-puncture family `plateau_puncture(s, s)`: the
    /// answer-embedded-product sentinel, and the floor under every
    /// settle.
    ///
    /// Every turn leaf sits on one incompressible pseudorandom plateau
    /// `x` of `Θ(s)` digits and the turn positions spell a jittered
    /// punctured mass `y` of `Θ(s)` isolated digits, so the exact rank
    /// embeds the integer product `2·x·y + 1` — bought with `Θ(s)`
    /// input bits, both factors' content beyond the settle's own
    /// balanced-digit compaction. No promotion ever fires; the cost is
    /// the close-time settle, one wide × dense multiplication run
    /// inside the backend at its bound `M(|v|)` — and because the same
    /// constructor embeds the product of arbitrary factors, any fold
    /// that answers exactly multiplies arbitrary input-funded
    /// integers, so `Ω(M(|v|))` floors every settle. The committed
    /// kernel
    /// (`schoolbook_settle_reads_superlinear_on_plateau_puncture`, the
    /// query fold's test suite) keeps the per-digit charge failing on
    /// this family (the `skyline_flatness` plateau-puncture band
    /// carries the enforcement). Designed against the
    /// linear-functional query rows.
    PlateauPuncture,
    /// The lone-freeze spine `lone_freeze(s, s)`: the first-freeze
    /// gate straddle, both sides on one knob.
    ///
    /// `s` unit-oscillation pairs ride a wide plateau strictly before
    /// the sweep's one freeze-firing drop, and `s` more run behind it
    /// with the gate open and a ten-digit drift parked — so any
    /// per-interval deposit toward the settle machinery made before
    /// drift exists to settle scales with the prefix, and a segment
    /// feed or close read that is not amortized O(1) per interval
    /// scales with the tail, while the family's funded wide codes stay
    /// O(1). Exactly one freeze and no promotion ever fires, so the
    /// column also prices the settle's smallest nonempty
    /// configuration. The `skyline_flatness` lone-freeze bands isolate
    /// each axis at the generator minimum and carry the enforcement;
    /// the column scales both together. Designed against the
    /// linear-functional query rows.
    LoneFreeze,
    /// The tooth-tail pair `tooth_tail(g, m)`: the boundary-aligned
    /// exact-`top` population.
    ///
    /// Two same-shape unit chains whose second leaves spike `2^(32g)`
    /// in both operands, `b` one tick above `a` everywhere except the
    /// shared terminal: the pair sweep folds both spikes into one
    /// cancelling difference at the same boundary, then reads
    /// `sign(D)` once per remaining boundary with no intervening
    /// write. Exact-`top` maintenance prices each read at the settled
    /// value's own width; a high-water bound re-walks the spike's `g`
    /// dead digits per read — `Θ(m·g)` on `Θ(m + g)` input (the
    /// `skyline_flatness` tooth-tail band carries both readings).
    /// Every overlay boundary is shared by both operands and almost
    /// every stored delta is zero, so the pair is also the touch
    /// floor's honest-less-work witness
    /// ([`touch_pair_fold`](super::floors::touch_pair_fold)): a
    /// conforming sweep is forced to fold only the three nonzero
    /// deltas per operand, and the measured per-boundary sign-read
    /// traffic sits far above that floor as implementation, never
    /// mandate.
    ToothTail,
    /// The fixed-seed organic control population.
    Benign,
}

/// Every family, in display order.
///
/// Adding a shape: the array length and the [`FamilyData::build`] and
/// [`designed`](super::ops::designed) match arms are compiler-forced from here.
/// What the compiler cannot force, in the order it is otherwise found by
/// luck: the shape's base-size constant (the block above, with its
/// derivation doc), the module doc's family prose and any cardinality
/// it carries, the cell-count pin and its derivation comment
/// (`tests/amp_board_smoke.rs`), the envelope rows in `tests/meter.rs`
/// (the enforced record), the ceiling-calibration witnesses (the
/// `ceilings` module's header comment), and — only if a cell needs a
/// declared model or turns up red — the declaration site (the
/// `ceilings` module's declared-models section), the red-triage buffer
/// ([`BOARD_EXPECTED_REDS`](super::coverage::BOARD_EXPECTED_REDS), with a live task), the rider list
/// ([`BOARD_DECLARED_BENCH_RIDERS`](super::export::BOARD_DECLARED_BENCH_RIDERS)), and the judge roster with its
/// membership pin (`tools/benchjudge-expected.json`,
/// `tests/bench_judge_roster.rs`). And not every shape belongs here: a
/// whole-surface adversary earns a board family, while a kernel-seam
/// shape lives in the envelope suite alone, as `wide_tooth_comb`,
/// `alt_spine`, and the `memo_*` shapes do.
pub(super) const FAMILIES: [FamilyKind; 31] = [
    FamilyKind::Dense,
    FamilyKind::Bigroot,
    FamilyKind::Hugeleaf,
    FamilyKind::Cliff,
    FamilyKind::IdPair,
    FamilyKind::CombScatter,
    FamilyKind::Harmonic,
    FamilyKind::Scatter,
    FamilyKind::Weave,
    FamilyKind::Stagger,
    FamilyKind::NestedFull,
    FamilyKind::NestedWide,
    FamilyKind::MirrorWide,
    FamilyKind::MirrorNarrow,
    FamilyKind::Staircase,
    FamilyKind::RevealComb,
    FamilyKind::RevealHifloor,
    FamilyKind::PureComb,
    FamilyKind::AscendCliff,
    FamilyKind::AscendPlateau,
    FamilyKind::JumpPair,
    FamilyKind::FreezePos,
    FamilyKind::PromoRearm,
    FamilyKind::WeightComb,
    FamilyKind::FreezeParade,
    FamilyKind::DenseSuffix,
    FamilyKind::PlateauPuncture,
    FamilyKind::LoneFreeze,
    FamilyKind::ConcurrentPair,
    FamilyKind::ToothTail,
    FamilyKind::Benign,
];

/// One shape instantiated at one scale: the operand bundle every row's
/// `prepare` decodes fresh (outside measurement).
///
/// The bundle is the shape axis's declaration: each slot a shape fills
/// flows to every operation whose signature consumes it, so a shape's
/// reach is structural — build a shape with a version and it appears on
/// every version row; give it an id side and it appears on every party
/// row through the disjoint-mount adapter. The derived slots (`version2`,
/// `rank_pair`, a cross shape's `version` and `parties`) are filled by
/// one uniform post-pass in [`FamilyData::build`], never per shape.
pub(super) struct FamilyData {
    pub(super) kind: FamilyKind,
    pub(super) name: &'static str,
    /// The shape's primary packed version (a cross shape's event side).
    pub(super) version: Option<Vec<u8>>,
    /// The comparison counterpart: `version` plus one seed tick, packed.
    ///
    /// Derived uniformly by the post-pass — except on the pair shapes
    /// (jump-pair, concurrent-pair), whose build arms fill it with the
    /// pairing the shape was constructed around and the post-pass leaves
    /// in place.
    pub(super) version2: Option<Vec<u8>>,
    /// A disjoint packed party pair within one universe: natural for the
    /// id pair and the benign halves, minted by the disjoint-mount
    /// adapter from a cross shape's id side.
    pub(super) parties: Option<(Vec<u8>, Vec<u8>)>,
    /// The designated packed (event version, id party) cross: the pairing
    /// the shape was built around, driving the tick rows' walk floors and
    /// the clock rows' operand choice.
    ///
    /// Each cross shape's variant doc states the arm and cost genre its
    /// cross drives.
    pub(super) cross: Option<(Vec<u8>, Vec<u8>)>,
    /// Whether the cross's mandatory projection output dominates its
    /// input (the comb-scatter and plateau-comb shapes): the projection
    /// rows I/O-denominate exactly these cells (the `cell` module doc's
    /// output-domination cross).
    pub(super) output_dominated: bool,
    /// The bundle's value content in bytes, `Some` only on the
    /// flat-denominator shape (comb-scatter).
    ///
    /// The denominator every input-denominated cell's *exponent* is
    /// fitted against; constants and floors stay per packed byte (the
    /// `cell` module doc derives the split).
    pub(super) content_bytes: Option<usize>,
    /// The packed fold operands (versions, parties), consumed by the two
    /// fold rows alone: the scatter, weave, and stagger populations'
    /// adversarial orderings and the benign shape's organic control.
    #[allow(clippy::type_complexity)]
    pub(super) fold: Option<(Vec<Vec<u8>>, Vec<Vec<u8>>)>,
    /// An overlapping packed party pair within one universe: the
    /// rejection rows' operands.
    ///
    /// Minted by the overlap-mount adapter from the same id source as
    /// `parties` (the post-pass); semantically void by design — see
    /// [`overlap_mounted_pair`].
    pub(super) overlap: Option<(Vec<u8>, Vec<u8>)>,
    /// The mismatched rank pair, derived from `version` in the post-pass.
    ///
    /// Precomputed here (shape-derived rank, small integer rank) so the
    /// `rank_pair_ops` and `rank_sum` prepares clone their operands instead
    /// of re-running the rank fold:
    /// the bench harness calls prepare once per timed iteration, and the
    /// fold costs orders of magnitude more than the pair operations it
    /// feeds.
    pub(super) rank_pair: Option<(Rank, Rank)>,
}

impl FamilyData {
    /// A bundle with every slot empty, for a build arm to fill with what
    /// the shape honestly has.
    fn bare(kind: FamilyKind, name: &'static str) -> FamilyData {
        FamilyData {
            kind,
            name,
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
    /// `level` 0 and 1 are the two measurement scales of every cell. The
    /// arm fills the slots the shape natively has; the post-pass below
    /// derives the rest uniformly (a cross shape's version is its event
    /// side, its party pair is the disjoint-mount adapter over its id
    /// side; every version gains its rank pair, and its ticked
    /// counterpart wherever the arm built no pairing of its own), so a
    /// new shape reaches every operation its bundle supplies without
    /// naming any.
    pub(super) fn build(kind: FamilyKind, scale: f64, level: u32) -> FamilyData {
        let size = |base: usize| -> usize {
            let scaled = ((base as f64) * scale).round() as usize;
            scaled.max(MIN_SIZE_PARAM) << level
        };
        let mut data = match kind {
            FamilyKind::Dense => Self::event(
                kind,
                "dense",
                meter::dense(size(DENSE_BASE_DEPTH)).version().encode(),
            ),
            FamilyKind::Bigroot => Self::event(
                kind,
                "bigroot",
                meter::bigroot(size(BIGROOT_BASE_MAGNITUDE_BITS), size(BIGROOT_BASE_DEPTH))
                    .version()
                    .encode(),
            ),
            FamilyKind::Hugeleaf => Self::event(
                kind,
                "hugeleaf",
                meter::hugeleaf(size(HUGELEAF_BASE_MAGNITUDE_BITS))
                    .version()
                    .encode(),
            ),
            FamilyKind::Cliff => {
                let scale = size(CLIFF_BASE_SCALE);
                Self::event(
                    kind,
                    "cliff",
                    meter::cliff_comb(scale, scale).version().encode(),
                )
            }
            FamilyKind::IdPair => {
                let mut data = Self::bare(kind, "id-pair");
                data.parties = Some((
                    meter::id_spine(size(ID_BASE_DEPTH), false).bytes,
                    meter::id_spine(size(ID_BASE_DEPTH), true).bytes,
                ));
                data
            }
            FamilyKind::CombScatter => {
                let teeth = size(CROSS_BASE_TEETH);
                let mut data = Self::bare(kind, "comb-scatter");
                data.cross = Some((
                    meter::cliff_comb(CROSS_TOOTH_MAGNITUDE_BITS, teeth)
                        .version()
                        .encode(),
                    meter::scattered_id(teeth / 2).bytes,
                ));
                data.output_dominated = true;
                let (v, p) = data.cross.as_ref().expect("just set");
                data.content_bytes = Some(value_content_bytes(&decode_version(v)) + p.len());
                data
            }
            FamilyKind::Harmonic => Self::event(
                kind,
                "harmonic",
                meter::harmonic(size(HARMONIC_BASE_DEPTH))
                    .version()
                    .encode(),
            ),
            FamilyKind::Scatter => Self::scatter(size(SCATTER_BASE_CLOCKS)),
            FamilyKind::Weave => Self::weave(size(WEAVE_BASE_LEAVES)),
            FamilyKind::Stagger => Self::stagger(
                size(STAGGER_BASE_OPERANDS).next_power_of_two(),
                size(STAGGER_BASE_BLOCKS).next_power_of_two(),
            ),
            FamilyKind::NestedFull => {
                let d = size(NESTED_BASE_DEPTH);
                Self::cross_family(
                    kind,
                    "nested-full",
                    meter::dense(d).version().encode(),
                    meter::nested_full_id(d).bytes,
                )
            }
            FamilyKind::NestedWide => {
                let s = size(NESTED_WIDE_BASE);
                Self::cross_family(
                    kind,
                    "nested-wide",
                    meter::bigroot(s, s).version().encode(),
                    meter::nested_full_id(s).bytes,
                )
            }
            FamilyKind::MirrorWide => {
                let s = size(MIRROR_WIDE_BASE);
                Self::cross_family(
                    kind,
                    "mirror-wide",
                    meter::wide_tail(s, s).version().encode(),
                    meter::nested_left_full_id(s).bytes,
                )
            }
            FamilyKind::MirrorNarrow => {
                let d = size(MIRROR_NARROW_BASE_DEPTH);
                Self::cross_family(
                    kind,
                    "mirror-narrow",
                    meter::wide_tail(1, d).version().encode(),
                    meter::nested_left_full_id(d).bytes,
                )
            }
            FamilyKind::Staircase => {
                let d = size(STAIRCASE_BASE_DEPTH);
                Self::cross_family(
                    kind,
                    "staircase",
                    meter::staircase(d).version().encode(),
                    meter::id_spine(d, false).bytes,
                )
            }
            FamilyKind::RevealComb => {
                let s = size(REVEAL_COMB_BASE);
                let mut data = Self::cross_family(
                    kind,
                    "reveal-comb",
                    meter::reveal_comb(s, s).version().encode(),
                    meter::reveal_comb_id(s).bytes,
                );
                // Projecting the shared-wide-plateau event through its
                // site-owning comb id re-materializes a wide absolute
                // value per kept site: mandatory output Theta(k*b) on a
                // Theta(k + b) input, the same output domination the
                // comb-scatter cross declares [measured: output x4 per
                // input doubling, every work column within x4 of it].
                data.output_dominated = true;
                data
            }
            FamilyKind::RevealHifloor => {
                let s = size(REVEAL_COMB_BASE);
                let mut data = Self::cross_family(
                    kind,
                    "reveal-hifloor",
                    meter::reveal_comb_hifloor(s, s).version().encode(),
                    meter::reveal_comb_id(s).bytes,
                );
                // The raised floor changes the consume-time gap, not the
                // projection's re-materialized wide sites: the same
                // output domination as reveal-comb.
                data.output_dominated = true;
                data
            }
            FamilyKind::PureComb => {
                let s = size(PURE_COMB_BASE);
                let mut data = Self::cross_family(
                    kind,
                    "pure-comb",
                    meter::pure_comb(s, s).version().encode(),
                    meter::pure_comb_id(s).bytes,
                );
                // Bare wide leaves under the site-owning id: the masked
                // skyline spells a wide code per owned site, the same
                // output domination as reveal-comb.
                data.output_dominated = true;
                data
            }
            FamilyKind::AscendCliff => {
                let s = size(ASCEND_CLIFF_BASE);
                Self::cross_family(
                    kind,
                    "ascend-cliff",
                    meter::ascend_cliff(s, s).version().encode(),
                    meter::ascend_cliff_id(s).bytes,
                )
            }
            FamilyKind::AscendPlateau => {
                let s = size(ASCEND_CLIFF_BASE);
                Self::cross_family(
                    kind,
                    "ascend-plateau",
                    meter::ascend_cliff_plateau(s, s).version().encode(),
                    meter::ascend_cliff_id(s).bytes,
                )
            }
            FamilyKind::JumpPair => {
                let m = size(JUMP_PAIR_BASE_TEETH);
                let d = (m / JUMP_PAIR_DIGIT_DIVISOR).max(1);
                let (a, b) = meter::jump_pair(JUMP_PAIR_MAGNITUDE_BITS, m, d);
                let mut data = Self::event(kind, "jump-pair", a.version().encode());
                data.version2 = Some(b.version().encode());
                data
            }
            FamilyKind::FreezePos => Self::event(
                kind,
                "freeze-pos",
                meter::freeze_position(size(FREEZE_POS_BASE_BLOCKS))
                    .version()
                    .encode(),
            ),
            FamilyKind::PromoRearm => Self::event(
                kind,
                "promo-rearm",
                meter::promotion_rearm(size(PROMO_REARM_BASE_BLOCKS))
                    .version()
                    .encode(),
            ),
            FamilyKind::WeightComb => Self::event(
                kind,
                "weight-comb",
                meter::weight_comb(size(WEIGHT_COMB_BASE_BLOCKS).next_power_of_two())
                    .version()
                    .encode(),
            ),
            FamilyKind::FreezeParade => Self::event(
                kind,
                "freeze-parade",
                meter::freeze_parade(size(FREEZE_PARADE_BASE_BLOCKS).next_power_of_two())
                    .version()
                    .encode(),
            ),
            FamilyKind::DenseSuffix => {
                // One knob drives the block count and the gap-digit
                // count (the bands' DS(p, p) diagonal); the mate is the
                // same topology at unit bases, the pair the distance
                // band prices.
                let p = size(DENSE_SUFFIX_BASE_BLOCKS);
                let mut data = Self::event(
                    kind,
                    "dense-suffix",
                    meter::dense_suffix(p, p).version().encode(),
                );
                data.version2 = Some(meter::dense_suffix_mate(p, p).version().encode());
                data
            }
            FamilyKind::PlateauPuncture => {
                // One knob drives the plateau width and the turn count
                // (the band's PP(s, s) diagonal), floored at the
                // generator's minimum width; the floor binds only under
                // extreme scale-down (the base constant's rustdoc).
                let s = size(PLATEAU_PUNCTURE_BASE_DIGITS).max(10);
                Self::event(
                    kind,
                    "plateau-puncture",
                    meter::plateau_puncture(s, s).version().encode(),
                )
            }
            FamilyKind::LoneFreeze => {
                // One knob drives the plateau prefix and the frozen
                // tail (the bands isolate each axis; the column scales
                // both), kept even at every scale — the generator
                // counts whole oscillation pairs, and MIN_SIZE_PARAM
                // keeps the masked value at least 4.
                let s = size(LONE_FREEZE_BASE_PAIRS) & !1;
                Self::event(
                    kind,
                    "lone-freeze",
                    meter::lone_freeze(s, s).version().encode(),
                )
            }
            FamilyKind::ConcurrentPair => {
                let n = size(CONCURRENT_BASE_LEAVES).next_power_of_two();
                let (v, w) = meter::concurrent_pair(n);
                let mut data = Self::event(kind, "concurrent-pair", v.encode());
                data.version2 = Some(w.encode());
                data
            }
            FamilyKind::ToothTail => {
                // One knob: the boundary count, with the spike width
                // riding it at the committed band ratio (the generator
                // needs g >= 1 and m >= 2; the size floor guarantees
                // both).
                let m = size(TOOTH_TAIL_BASE_BOUNDARIES);
                let (a, b) = meter::tooth_tail((m / TOOTH_TAIL_SPIKE_DIVISOR).max(1), m);
                let mut data = Self::event(kind, "tooth-tail", a.version().encode());
                data.version2 = Some(b.version().encode());
                data
            }
            FamilyKind::Benign => Self::benign(size(BENIGN_BASE_CLOCKS)),
        };
        // ── the bundle post-pass: the derived slots, uniform across shapes ──
        // A cross shape's primary version is its event side.
        if data.version.is_none() {
            data.version = data.cross.as_ref().map(|(v, _)| v.clone());
        }
        // Every version gains its ticked comparison counterpart (where
        // the shape did not build its own pairing) and its mismatched
        // rank pair (shape-derived rank against a small integer rank,
        // the pair whose exponent mismatch the rank rows price).
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
        // overlap-mount adapter, for the rejection rows: the cross id
        // where the shape has one, the first natural party otherwise.
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

    /// Build the scatter fold population: `n` balanced-forked parties, one
    /// tick each, ordered evens before odds so a sequential fold's
    /// accumulator holds every other leaf and never coalesces.
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
        // Dropping the tail keeps `n` honest at non-power-of-two scales;
        // a dropped party's region simply goes unowned.
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
        let mut data = Self::bare(FamilyKind::Scatter, "scatter");
        data.fold = Some((versions, parties));
        data
    }

    /// Build the weave fold population.
    ///
    /// The `leaves` (rounded up to a power of two) leaf parties of one
    /// balanced fork expansion are dealt round-robin into
    /// [`WEAVE_GROUPS`] group parties, each group carrying its own
    /// single-tick version.
    ///
    /// Dealing leaf `i` to group `i % WEAVE_GROUPS` puts leaves of every
    /// group under every skeleton node above the last `log2(WEAVE_GROUPS)`
    /// levels, so each operand pair is both-present at the whole shared
    /// skeleton — the correlated-population genre — while each group on
    /// its own is an ordinary scattered region set.
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
        // Deal the leaves round-robin: each group accumulates its party
        // by joining every WEAVE_GROUPS-th leaf, and its version by one
        // tick per dealt leaf — a single-leaf party forces the event onto
        // that leaf, so the group's version is height one exactly over
        // its scattered region, a deep tree sharing the whole upper
        // skeleton with every other group's.
        let mut group_parties: Vec<Option<Party>> = (0..WEAVE_GROUPS).map(|_| None).collect();
        let mut group_versions: Vec<Version> = (0..WEAVE_GROUPS).map(|_| Version::new()).collect();
        for (i, leaf) in parties.into_iter().enumerate() {
            let r = i % WEAVE_GROUPS;
            group_versions[r].tick(&leaf);
            match &mut group_parties[r] {
                slot @ None => *slot = Some(leaf),
                Some(group) => group
                    .join(leaf)
                    .expect("leaves of one fork expansion are pairwise disjoint"),
            }
        }
        let versions = group_versions.iter().map(Version::encode).collect();
        let parties = group_parties
            .into_iter()
            .map(|g| g.expect("every group received leaves").encode())
            .collect();
        let mut data = Self::bare(FamilyKind::Weave, "weave");
        data.fold = Some((versions, parties));
        data
    }

    /// Build the staggered fold population: `n` operands of `m` unit
    /// teeth each, teeth in the gaps of every other operand's.
    ///
    /// Fed in bit-reversed order (`meter::stagger_population` carries
    /// both the construction and the feed order's derivation).
    fn stagger(n: usize, m: usize) -> FamilyData {
        let (versions, ids) = meter::stagger_population(n, m);
        let mut data = Self::bare(FamilyKind::Stagger, "stagger");
        data.fold = Some((
            versions.iter().map(|p| p.version().encode()).collect(),
            ids.into_iter().map(|p| p.bytes).collect(),
        ));
        data
    }

    /// Wrap a cross shape: a packed (event, id) pair built as one
    /// adversarial pairing.
    ///
    /// The cross drives the tick rows' walk floors and the clock rows'
    /// operand choice directly; the post-pass derives the shape's version
    /// (the event side) and its disjoint party pair (the mounted id side),
    /// so the shape also reaches every version and party row.
    fn cross_family(
        kind: FamilyKind,
        name: &'static str,
        version: Vec<u8>,
        id: Vec<u8>,
    ) -> FamilyData {
        let mut data = Self::bare(kind, name);
        data.cross = Some((version, id));
        data
    }

    /// Wrap an event shape's wire bytes.
    fn event(kind: FamilyKind, name: &'static str, bytes: Vec<u8>) -> FamilyData {
        let mut data = Self::bare(kind, name);
        data.version = Some(bytes);
        data
    }

    /// Build the benign control: `n` clocks forked at random from a seed,
    /// each ticked one to three times, folded into one version and two
    /// disjoint half-population parties.
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
        let version = Version::join_all(clocks.iter().map(|c| c.version().clone()));
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
        for (i, p) in parties.enumerate() {
            // Alternate the halves so both operand parties scatter across
            // the whole id tree rather than owning one contiguous region.
            let half = if i % 2 == 0 { &mut a } else { &mut b };
            half.join(p).expect("forked parties are pairwise disjoint");
        }
        let mut data = Self::bare(FamilyKind::Benign, "benign");
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

    /// Two joinable clocks (disjoint parties), with combined operand
    /// bytes, from the bundle's slots.
    ///
    /// A shape with both a party pair and versions crosses them; a
    /// party-only pair rides empty versions; a version-only shape forks
    /// a seed pair around its version pair.
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

/// The disjoint-mount adapter: lift one packed id shape into a disjoint
/// party pair inside a single universe.
///
/// The pair mounts the shape under opposite children of a fresh root —
/// `(shape, ·)` and `(·, shape)` — so the halves are disjoint by
/// construction and joining them merely reunites the root's two subtrees:
/// two independently-generated id shapes are never asked to share a
/// universe (linearity of parties is the invariant everything rests on —
/// the crate docs' safety rules). Each half is the shape itself one level
/// deeper, so party cells on a mounted shape measure the shape plus one
/// root tag. Runs at bundle build, outside any measurement, and asserts
/// the disjointness it mints.
fn disjoint_mounted_pair(id: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let shape = decode_party(id);
    let mount = |left: bool| -> Vec<u8> {
        let mut bits = codec::Bits::with_capacity(shape.as_bits().len() + 2);
        bits.push(left);
        bits.push(!left);
        bits.extend_from_bitslice(shape.as_bits());
        codec::zero_dead_bits(&mut bits);
        bits.into_vec()
    };
    let (a, b) = (mount(true), mount(false));
    assert!(
        decode_party(&a).is_disjoint(&decode_party(&b)),
        "the disjoint-mount adapter must mint a disjoint pair"
    );
    (a, b)
}

/// The overlap-mount adapter: lift one packed id shape into an
/// *overlapping* party pair whose single shared region sits at both
/// operands' preorder ends — the disjoint-mount adapter's counterpart,
/// for the rejection rows.
///
/// `a` mounts the shape under a fresh root's left child and a marker
/// under its right; `b` mounts the shape under the right child alone.
/// The marker is a single-child chain along the shape's rightmost-present
/// path ending in a terminal at the shape's preorder-last owned position,
/// so the pair's one overlap is the last position a lockstep walk over
/// `b`'s side reaches, with every earlier region disjoint — rejection
/// consumes essentially both streams before the witnessing pair meets.
///
/// The outputs are **semantically void by design**: a well-formed pair
/// that no legal fork/join history produces (two claims on one region),
/// built on purpose because the crate's cost claims are total — the
/// rejection rows price what rejecting such a pair costs, and nothing
/// downstream treats the pair as meaningful. Runs at bundle build,
/// outside any measurement, and asserts the overlap it mints (both
/// halves decode canonically on the way).
pub(super) fn overlap_mounted_pair(id: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let shape = decode_party(id);
    let bits = shape.as_bits();
    let path = rightmost_terminal_path(bits);
    assert!(
        !path.is_empty(),
        "the overlap-mount adapter needs a non-terminal shape: a full shape's mount would \
         not be normal form"
    );
    let mut a = codec::Bits::with_capacity(bits.len() + 2 * path.len() + 4);
    a.push(true); // root: both children present
    a.push(true);
    a.extend_from_bitslice(bits); // left: the shape
    for &go_right in &path {
        // right: the marker chain, one single-child node per level
        a.push(!go_right);
        a.push(go_right);
    }
    a.push(false); // the marker's terminal, at the shape's last owned position
    a.push(false);
    codec::zero_dead_bits(&mut a);
    let mut b = codec::Bits::with_capacity(bits.len() + 2);
    b.push(false); // root: right child only
    b.push(true);
    b.extend_from_bitslice(bits); // right: the shape
    codec::zero_dead_bits(&mut b);
    let (a, b) = (a.into_vec(), b.into_vec());
    assert!(
        !decode_party(&a).is_disjoint(&decode_party(&b)),
        "the overlap-mount adapter must mint an overlapping pair"
    );
    (a, b)
}

/// The branch choices (`false` left, `true` right) from an id tree's root
/// to its preorder-last terminal: at every node, the last present child.
///
/// Preorder lays each subtree's bits contiguously, so the stream's final
/// tag belongs to the node reached by always taking the rightmost present
/// child; left subtrees along the way are skipped (each exactly once, so
/// the walk is linear). Runs at bundle build, outside any measurement.
fn rightmost_terminal_path(bits: &codec::BitsSlice) -> Vec<bool> {
    let mut pos = 0usize;
    let mut path = Vec::new();
    loop {
        let left = bits[pos];
        let right = bits[pos + 1];
        pos += 2;
        if !left && !right {
            return path; // the terminal
        }
        if right {
            if left {
                pos = crate::idbits::skip_subtree(pos, |at| {
                    let children = usize::from(bits[at]) + usize::from(bits[at + 1]);
                    (children, at + 2)
                });
            }
            path.push(true);
        } else {
            path.push(false);
        }
    }
}

/// The overlap fold's probe: a right-mounted full leaf — `(0, 1)`, one
/// packed byte — overlapping the a-mount's whole right half (the marker's
/// region).
///
/// The `party_join_all_overlap` row's per-input operand. The witnessing
/// pair sits in the right half, behind the accumulator's whole left
/// shape, so a per-input overlap test priced in the accumulator — a
/// cursor walk skip-scanning the left shape to reach the witness — reads
/// Θ(accumulator) scan per O(1)-byte input and turns the row quadratic;
/// the fold's per-call accumulator index answers the same test in
/// O(probe), which is the separation the row watches.
pub(super) fn overlap_fold_probe() -> Vec<u8> {
    let mut probe = codec::Bits::with_capacity(4);
    probe.push(false); // root: right child only
    probe.push(true);
    probe.push(false); // the right child: a full leaf
    probe.push(false);
    codec::zero_dead_bits(&mut probe);
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
