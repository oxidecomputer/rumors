//! Resource envelopes for rank, minimum-tick, and projection kernels.

use super::*;

// ─── Version query-fold scenarios ───────────────────────────────────────────
//
// The query kernels over version streams: rank integration, `min_ticks`
// subtree-minimum accounting, and projection against a party. Streams are transcoded
// outside measurement. The kernels' arithmetic lives in digit touches and
// their stream work in the scan column. The cliff
// and wide-tooth rank rows are load-bearing live-path pins: wide deltas
// ride the live component without freezing — the comb's terminal borrow
// and every 192-bit tooth are each paid by their own codes — and the
// `skyline_flatness` module's freeze-band and jump rows pin the freeze
// discipline itself (bounded oscillation never freezes at any width;
// stale drift is evicted once, at the drift's own width). The projection
// row is I/O-denominated per the board's criterion: its output is
// mandatory and dominates its input, so the pinned ceilings price
// input + output bytes (the MEASURED line prints both).

// Pins per the file doc's convention; each row's trailing comment states
// the mechanism that prices it.
#[rustfmt::skip]
pub mod query_env {
    use super::{band, envelope, Envelope};
    pub const SKYLINE_RANK_CLIFF: Envelope           = envelope(  2_855,     band(6_688, 4_012),       band(35_845, 21_507)); // the live component absorbs the oscillation at O(1) digits per fold; the terminal borrow rides it into one wide add, no freeze
    pub const SKYLINE_RANK_WIDE_TOOTH: Envelope      = envelope(  3_635,   band(24_585, 14_751), band(2_000_960, 1_200_576)); // the no-freeze pin: every fold paid by its tooth's own code; certificate skips replace zero-run walks, and the pre-scan records each payload skip once on this payload-dominated comb
    // The practical-regime gauge: `Version::rank`
    // on one concurrent-pair operand — word-scale heights over organic
    // forks, no freeze, no arming. The row pins the benign path's
    // constants so the worst-case machinery's price on common inputs is
    // a committed number, not a vibe.
    pub const RANK_CONCURRENT: Envelope              = envelope(      0,    band(11_099, 6_659),       band(61_448, 36_868)); // word-scale heights: zero heap, one walk's scan and touches
    pub const TICKS_DENSE: Envelope                  = envelope( 58_815,  band(156_270, 93_762),     band(468_809, 281_285)); // the tick row's cost plus the count's gamma codes
    pub const TICKS_NESTED_WIDE: Envelope            = envelope( 14_107,   band(31_125, 18_675),      band(205_085, 90_042)); // the fill branch's second walk plus the split builder's linear separation and final interleaving
    pub const TICKS_MIRROR_WIDE: Envelope            = envelope( 63_520,   band(72_582, 43_548),     band(220_048, 132_028)); // second-walk fill branch, as the nested-wide row; the frame ledger uses one usize queue cell per site, and the pre-scan records minima only, so per-site collapse re-reads and raise-mirror folds stay out of the scan and touch columns
    pub const SKYLINE_MIN_TICKS_DENSE: Envelope      = envelope( 30_720, band(312_508, 187_504),     band(468_758, 281_254)); // every delta updates the live height and the nested-minimum tracker, so touches run ~2x the rank row's with no minimum displacement
    pub const SKYLINE_MIN_TICKS_CLIFF: Envelope      = envelope(  3_530,    band(12_000, 7_200),       band(17_923, 10_753)); // frozen-prefix coefficients count the comb's repeated wide height, so that height enters the exact total once
    pub const SKYLINE_MIN_TICKS_ASCEND: Envelope     = envelope(54_000,    band(20_044, 12_026),        band(12_823, 7_693)); // the boundary-stacking row: dense stack columns keep both heap and touch work within these ceilings
    pub const SKYLINE_PROJECT_COMB_SCATTER: Envelope = envelope(988_890,   band(44_924, 26_954), band(7_922_599, 1_591_299)); // output-dominated: the split builder holds at most two stream forms and scans the output once to separate it and once to interleave it
    pub const FOLD_VERSION_SCATTER: Envelope         = envelope(    323,   band(61_429, 36_857),     band(330_913, 198_547)); // the balanced reduction: near-linear in the population's encoded bytes where a left fold re-scans its whole accumulator per input; the at-rest form is a length-carrying container of the wire bytes, cloned by refcount in the fold's lone-group settle and adoption arms, and the counter stack's entries carry the operand-form tag (~8 B per level)
    pub const FOLD_PARTY_SCATTER: Envelope           = envelope(    780,             band(0, 0),     band(322_068, 193_240)); // pure stream scanning through the balanced merges; one refcount control block per frozen stream lives in the fold's groups
    // The tick rows: the tick walk's cost currency is accumulator digit
    // touches, with scanned bits beside it.
    pub const TICK_DENSE: Envelope                   = envelope( 58_815,  band(156_265, 93_759),     band(468_765, 281_259)); // the fused tick: copy-on-first-divergence defers the output buffer past the collapse scan, so the scan path and the builder never coexist at peak
    pub const TICK_NESTED_WIDE: Envelope             = envelope( 14_108,   band(30_808, 18_484),      band(135_042, 48_016)); // the explicit-stack walk plus the split builder's linear separation and final interleaving
    pub const TICK_MIRROR_WIDE: Envelope             = envelope( 63_520,   band(71_955, 43_173),      band(160_003, 96_001)); // the frame ledger stores one usize queue cell per site and no link for the shared wide minimum; the pre-scan records minima only, so per-site collapse re-reads and raise-mirror folds stay out of the scan and touch columns
    // The expansion rows: grow-branch deep
    // ticks measuring the whole public tick — walk, route fold, and
    // splice — in one fused pass.
    pub const TICK_OWNERSHIP_HOLE: Envelope          = envelope(  3_647,     band(7_563, 4_537),       band(37_585, 22_551)); // the ownership-gated block scan: unowned staircase runs fold as one net-and-minimum summary each; the touch ceiling sits below the leaf-by-leaf mechanism's reading, so the skip must engage for the pin to hold, and the scan column holds every skipped bit still read
    pub const TICK_OWNERSHIP_COMB: Envelope          = envelope( 59_575,  band(156_275, 93_765),     band(498_774, 299_264)); // readings identical to the ungated per-leaf walk's on this family (single-leaf regions everywhere, so the block gate never opens and may cost nothing when closed)
    pub const TICK_COLLAPSE_HOLE: Envelope           = envelope(  2_748,     band(8_125, 4_875),        band(14_368, 8_620)); // the descend-arm consuming max scan rides the block summary over each deep collapse range, its only crossing; rerouting either lead's ranges to the per-leaf fold reads touches over the ceiling, and the scan column holds every folded bit still read
    pub const TICK_COPY_HOLE: Envelope               = envelope(  1_733,    band(15_615, 9_369),       band(53_302, 31_980)); // the pre-scan copies each untouched range as one net movement and one watermark emission; rerouting either lead's ranges to per-leaf virtual emissions reads touches over the ceiling, and the scan column holds every folded bit still read
    pub const TICK_RAISE_HOLE: Envelope              = envelope(  2_660,     band(8_030, 4_818),        band(13_543, 8_125)); // the ascend-arm consuming max scan rides the block summary over each deep raised range, its only crossing; rerouting either lead's ranges to the per-leaf fold reads touches over the ceiling, and the scan column holds every folded bit still read
    pub const TICK_SITE_HOLE: Envelope               = envelope(  2_768,    band(10_779, 6_467),       band(27_962, 16_776)); // the pre-scan's collapse skip and the walk's consuming max scan each cross every deep range once as one block fold, and the collapse skip's fold accumulates the net movement alone; a block fold that also streams the range's unread minimum reads touches over the ceiling, and the scan column holds every folded bit still read
    pub const MASKED_CMP_HOLE: Envelope              = envelope(    480,           band(18, 10),         band(7_535, 4_521)); // the block skip consumes the spine's unowned continuation whole: the touch reading is a function of the mask depth alone; a per-boundary walk reads ~one touch per spine boundary, orders over the ceiling — the depth band beside this row holds the reading flat across a spine-depth doubling
    pub const TICK_EXPAND_SPINE: Envelope            = envelope(435_435,             band(0, 0), band(2_187_519, 1_312_511)); // an empty version's tick folds one word-scale payload: near-zero accumulator work; the emit codes the whole expansion chain as fresh one-bit deltas
    pub const TICK_EXPAND_CROSS: Envelope            = envelope(611_210,  band(156_260, 93_756), band(3_593_782, 2_156_268)); // the mixed regimes: the fused walk down the shared spine plus the id-only expansion fold, spliced in one pass
    // The version-pair rows: the public
    // two-operand queries on the pair families (the corpus pairing
    // `w = v + one seed tick` collapses the second operand onto a
    // dominating plateau, so the co-sweep's orientation switches and its
    // freeze paths would go unpriced without these rows). All four rows
    // are linear records of the fused co-sweep, which reads flat where
    // the composed emit-then-re-rank shape reads superlinear (the
    // `skyline_flatness` band test holds the jump-pair rows flat across
    // a scale doubling). Lag walks both operands' full overlay on the
    // accumulator instead of skipping the meet leg, which is what buys
    // its heap, scan, and touch readings down to the distance row's
    // neighborhood.
    pub const DISTANCE_JUMP_PAIR: Envelope           = envelope(  5_750,  band(158_194, 94_916), band(2_694_095, 1_616_457)); // the fused co-sweep applies signed differences directly and settles each frozen segment once; the pre-scan still records each payload skip once per operand
    pub const LAG_JUMP_PAIR: Envelope                = envelope(  5_750,  band(145_950, 87_570), band(2_694_095, 1_616_457)); // the one-sided functional over the same fused co-sweep as the distance row
    pub const DISTANCE_CONCURRENT: Envelope          = envelope(      0,   band(32_429, 19_457),      band(117_753, 70_651)); // orientation-switch density on word-scale heights: the pair never freezes, so no segment feed deposits
    pub const LAG_CONCURRENT: Envelope               = envelope(      0,   band(33_278, 19_966),      band(117_753, 70_651)); // the one-sided functional over the same switch-dense overlay
    // The masked-comparison rows:
    // the fused projected comparisons on the correlated mask-drift
    // families, priced input-only on shapes whose *materialization* is
    // product-growth — the laziness the view exists for.
    pub const MASKED_CMP_DRIFT_TRIPLE: Envelope      = envelope(  1_570,     band(5_240, 3_144),       band(20_488, 12_292)); // one pass over the overlay, ~2 touches per stored delta
    pub const MASKED_CMP_DRIFT_QUAD: Envelope        = envelope(  2_720,   band(83_946, 50_367),   band(1_342_092, 805_255)); // the sparse comb's wide climb/drop codes dominate the input; scan ~8 bits per input byte
}

/// The rank kernel on the boundary comb's Version stays within its
/// envelope.
///
/// The heights are `2^k`-scale behind 3-bit deltas, the live component
/// absorbs the oscillation at O(1) digits per fold, and the terminal
/// borrow — as wide as its own code — rides the live component into the
/// last leaf's single wide add, no freeze anywhere.
#[test]
fn skyline_rank_cliff_envelope() {
    let p = Shape::CliffComb.build2(CLIFF_SCALE, CLIFF_SCALE);
    let v = version_of(&p);
    let enc = version_of(&p);
    let r = metered(
        "skyline_rank_cliff",
        enc.as_bytes().len(),
        &query_env::SKYLINE_RANK_CLIFF,
        || meter::version::rank(&enc),
    );
    assert_eq!(r, v.rank(), "the kernel must match the encoded rank");
}

/// The rank kernel on the wide-tooth comb's Version stays within its
/// envelope — the no-freeze pin.
///
/// Bounded 192-bit oscillation keeps the live component exactly as wide
/// as each tooth's own code, so every fold and every per-leaf add is paid
/// by that code and the frozen component never churns (the
/// `skyline_flatness` freeze-band row pins the same shape above the
/// freeze allowance).
#[test]
fn skyline_rank_wide_tooth_envelope() {
    let p = Shape::WideToothComb.build3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
    let v = version_of(&p);
    let enc = version_of(&p);
    let r = metered(
        "skyline_rank_wide_tooth",
        enc.as_bytes().len(),
        &query_env::SKYLINE_RANK_WIDE_TOOTH,
        || meter::version::rank(&enc),
    );
    assert_eq!(r, v.rank(), "the kernel must match the encoded rank");
}

/// The min_ticks kernel on the dense spine stays within its envelope:
/// one narrow offset min-merge per node, heights on the accumulator,
/// across 125k levels.
#[test]
fn skyline_min_ticks_dense_envelope() {
    let p = Shape::Dense.build1(DENSE_DEPTH);
    let v = version_of(&p);
    let enc = version_of(&p);
    let r = metered(
        "skyline_min_ticks_dense",
        enc.as_bytes().len(),
        &query_env::SKYLINE_MIN_TICKS_DENSE,
        || meter::version::min_ticks(&enc),
    );
    assert_eq!(
        r.to_string(),
        v.min_ticks().to_string(),
        "the kernel must match the encoded fold"
    );
}

/// The min_ticks kernel on the boundary comb stays within its envelope.
///
/// The `2^k`-scale first height rides the frozen component and enters
/// the exact total once, through the counting term — never per leaf —
/// so the comb's teeth cost narrow offsets only.
#[test]
fn skyline_min_ticks_cliff_envelope() {
    let p = Shape::CliffComb.build2(CLIFF_SCALE, CLIFF_SCALE);
    let v = version_of(&p);
    let enc = version_of(&p);
    let r = metered(
        "skyline_min_ticks_cliff",
        enc.as_bytes().len(),
        &query_env::SKYLINE_MIN_TICKS_CLIFF,
        || meter::version::min_ticks(&enc),
    );
    assert!(
        r.to_string().len() > 20,
        "the comb's floor exceeds any machine word: the wide arm is live"
    );
    assert_eq!(
        r.to_string(),
        v.min_ticks().to_string(),
        "the kernel must match the encoded fold"
    );
}

/// The min_ticks kernel on the ascending cliff stays within its envelope.
///
/// The ascending spine arms every open range one above its parent's
/// minimum, so the tracker holds `ASCEND_STACK_DEPTH − 1` nonzero unit
/// boundary differences simultaneously at the terminal cliff — the one
/// committed min_ticks shape where boundary storage determines the transient
/// heap. Each unit difference occupies the word arm of the compact boundary
/// stack, and the terminal cliff consumes it with an O(1) word fold.
#[test]
fn skyline_min_ticks_ascend_envelope() {
    let p = Shape::AscendCliff.build2(ASCEND_STACK_DEPTH, ASCEND_STACK_MAGNITUDE_BITS);
    let v = version_of(&p);
    let enc = version_of(&p);
    let r = metered(
        "skyline_min_ticks_ascend",
        enc.as_bytes().len(),
        &query_env::SKYLINE_MIN_TICKS_ASCEND,
        || meter::version::min_ticks(&enc),
    );
    // The family's closed form: k leaves at 2^b + i over spine minima
    // all zero (the terminal cliff), so min_ticks = k·2^b + k(k+1)/2.
    let k = ASCEND_STACK_DEPTH;
    let expected = BigUint::from(k as u64) * (BigUint::ONE << ASCEND_STACK_MAGNITUDE_BITS)
        + BigUint::from((k * (k + 1) / 2) as u64);
    assert_eq!(
        r.to_string(),
        expected.to_string(),
        "min_ticks disagrees with the ascending cliff's closed form"
    );
    assert_eq!(
        r.to_string(),
        v.min_ticks().to_string(),
        "the kernel must match the encoded fold"
    );
}

/// The projection kernel on the comb × scattered-party cross stays
/// within its envelope — the output-dominated case.
///
/// Every kept tooth boundary forces a fresh `2^k`-scale magnitude into
/// the output, so the mandatory output dominates the linear input and the
/// pinned ceilings price input + output bytes (the denomination the
/// board's criterion records for exactly this cross).
#[test]
fn skyline_project_comb_scatter_envelope() {
    let p = Shape::CliffComb.build2(CLIFF_SCALE, CLIFF_SCALE);
    let v = version_of(&p);
    let party = before::Party::decode(&Shape::ScatteredId.build1(CLIFF_SCALE / 2).bytes[..])
        .expect("scattered id is strict normal form");
    let enc = version_of(&p);
    let io_bytes_in = enc.as_bytes().len() + Shape::ScatteredId.build1(CLIFF_SCALE / 2).bytes.len();
    let out = metered(
        "skyline_project_comb_scatter",
        io_bytes_in,
        &query_env::SKYLINE_PROJECT_COMB_SCATTER,
        || meter::version::project(&enc, &party),
    );
    eprintln!(
        "MEASURED skyline_project_comb_scatter: output_bytes={}",
        out.as_bytes().len()
    );
    let expected = (&v / &party).to_version();
    assert_eq!(out, expected, "the kernel must match the encoded quotient");
}
