//! Resource envelopes: peak transient heap, grown stack segments, and
//! big-integer limb work per operation on the adversarial input families.
//!
//! The contract this suite is driving toward: no operation materializes
//! transient state asymptotically larger than its packed operands, and every
//! operation is amortized O(n + m) in the packed input bits — with no bound
//! on value magnitude, tree depth, or encoded size. Today's implementation
//! is far from that — several operations amplify their input by large
//! constants or worse — so every scenario here pins the *current* measured
//! cost, with ×1.25 slack, as a ceiling. A regression fails loudly now; each
//! improvement tightens a committed number.
//!
//! Three deterministic meters, asserted together per scenario:
//!
//! - **Peak heap bytes**: the binary-wide counting allocator
//!   ([`PeakAlloc`]), read as a delta over the scenario body. One global
//!   allocator exists per test binary, and the counters are process-global,
//!   so per-scenario peaks are meaningful **only under nextest's
//!   process-per-test isolation** — this workspace's runner. Under a runner
//!   that shares one process across tests, concurrent allocation would bleed
//!   between scenarios.
//! - **Grown stack segments** ([`meter::stack_segments`]): the deep
//!   traversals grow the stack onto the heap in fixed-size segments that
//!   bypass any allocator meter; the segment counter is the honest stand-in
//!   for recursion-driven stack cost. Process-global, same isolation
//!   requirement.
//! - **Big-integer limb operations** ([`meter::limb_ops`], only when the
//!   `limb-meter` feature compiles the counter into the arithmetic):
//!   operand limbs per `Base` operation plus one value-width record per
//!   decoded wide-gamma value. Arithmetic-width cost is invisible to the
//!   other two meters — the work is wider, not more frequent — so this is
//!   the only column that sees a magnitude-quadratic regression. Without
//!   the feature the scenarios still run and assert the other two columns.
//!
//! Wall time is deliberately never asserted *in this suite*: it is the one
//! number here that is not deterministic (the amplification board judges the
//! wall-time *exponent* above a minimum-time threshold — see
//! `meter::board`'s module docs — and the display canary asserts a
//! wall-clock ratio in a reserved runner; those two are the wall legs of
//! record). The envelope constants are **measured** on the
//! development target (aarch64-apple-darwin, dev profile); heap byte counts
//! and limb counts are deterministic and portable across 64-bit targets
//! (limb counts shrink under release, where `debug_assert!` comparisons
//! vanish, so the dev-profile pin is the binding one), while segment counts
//! track per-target frame sizes, and the slack absorbs modest variation.

use std::cmp::Ordering;
use std::fmt::Debug;

use before::{meter, Party, Version};
use peak_alloc::PeakAlloc;

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

// ─── scenario sizes ─────────────────────────────────────────────────────────

/// Depth of the dense event spine `S(d)` scenarios.
const DENSE_DEPTH: usize = 125_000;

/// Root magnitude (bits) of the bigroot scenarios.
const BIGROOT_MAGNITUDE_BITS: usize = 40_000;

/// Spine depth of the bigroot scenarios.
const BIGROOT_DEPTH: usize = 10_000;

/// Leaf magnitude (bits) of the hugeleaf scenarios: one gamma code as wide
/// as the whole input, the shape where any cost superlinear in a single
/// code's width shows up undiluted.
const HUGELEAF_MAGNITUDE_BITS: usize = 125_000;

/// Depth of the id spine `I(d, divert)` pair scenarios.
const ID_DEPTH: usize = 250_000;

/// Tooth magnitude (bits) of the boundary comb scenarios; also its tooth
/// count, so crossing work `n·k` grows quadratically while each crossing
/// stays paid for by a `2k + 1`-bit stored code.
const CLIFF_SCALE: usize = 1_024;

/// Tooth width (bits) of the wide-tooth comb scenarios: wider than any
/// machine word, so every skyline delta is a genuinely wide operand while
/// still oscillating across the `2^k` cliff.
const WIDE_TOOTH_WIDTH_BITS: usize = 192;

/// Depth of the harmonic spine `H(d)` rank scenario: deep enough that the
/// fold's per-level numerator re-shifts dominate every constant.
const RANK_HARMONIC_DEPTH: usize = 65_536;

/// Spine depth behind the max-exponent rank of the pair-mismatch scenario.
const RANK_PAIR_DEPTH: usize = 500_000;

/// Integer ranks folded by the mixed-sum scenario.
const RANK_SUM_COUNT: usize = 10_000;

/// Spine depth behind the mixed-sum scenario's one high-exponent rank.
const RANK_SUM_EXP_DEPTH: usize = 250_000;

// ─── pinned envelopes ───────────────────────────────────────────────────────

/// One scenario's pinned ceilings: the measured value ×1.25, rounded up.
struct Envelope {
    /// Peak heap delta over the scenario body, in bytes.
    peak_heap: usize,
    /// Stack segments grown during the scenario body.
    segments: u64,
    /// Big-integer limb operations counted during the scenario body.
    #[cfg(feature = "limb-meter")]
    limb_ops: u64,
}

/// Build an [`Envelope`] from the three pinned columns.
///
/// The limb column is carried only when the `limb-meter` feature compiles
/// the counter into the arithmetic; the leading underscore keeps the
/// parameter warning-free in the other configuration.
const fn envelope(peak_heap: usize, segments: u64, _limb_ops: u64) -> Envelope {
    Envelope {
        peak_heap,
        segments,
        #[cfg(feature = "limb-meter")]
        limb_ops: _limb_ops,
    }
}

// The envelope table: pinned ceiling = measured ×1.25, rounded up, and
// only ever tightened: where a remeasure rises while staying inside an
// existing ceiling (the spilled-magnitude heap cells, which carry the
// backend's `len/8 + 2` words of growth headroom per heap allocation),
// the older, tighter ceiling stands over the recorded movement. The
// trailing comment on each line is the measurement of record (2026-07-22,
// aarch64-apple-darwin, dev profile, three identical runs) the ceiling
// derives from; a row re-pinned after an improvement records the movement
// as `old -> new` with the re-pin date, and its ceiling derives from the
// new value. Re-pin by rerunning this binary under `--no-capture` and
// reading the MEASURED lines (the limb column needs `--all-features` or
// `--features limb-meter`).
#[rustfmt::skip]
mod envelope {
    use super::{envelope, Envelope};
    //                                              peak heap,  segments, limb ops                      measured: peak heap, segments, limb ops
    pub const DECODE_DENSE: Envelope    = envelope(13_840_687,        0,       312_503); // 11_072_549,   0, 0 -> 250_002 (2026-07-23, metered Base equality)
    pub const CMP_DENSE: Envelope       = envelope(        10,      240,     2_500_013); //          8, 192,   2_000_010
    pub const JOIN_DENSE: Envelope      = envelope( 5_996_847,      300,     3_750_010); //  6_093_856 -> 4_797_477, 240, 2_750_008 -> 3_000_008 (2026-07-23, push-grow Builder; metered Base equality)
    pub const TICK_DENSE: Envelope      = envelope(11_837_440,      165,     1_250_005); // 12_355_499 -> 9_469_952, 132, 1_000_004 (2026-07-23, push-grow Builder)
    pub const DECODE_BIGROOT: Envelope  = envelope( 1_745_332,        0,        25_788); //  1_399_449 -> 1_396_265 -> 1_396_905 (2026-07-24, dashu-int backend), 0, 12_520_000 -> 626 -> 20_630 (2026-07-23, limb-wise wide-gamma decode; metered Base equality)
    pub const CMP_BIGROOT: Envelope     = envelope(62_531_270,       15,    47_007_838); // 50_028_232 -> 50_025_016 -> 56_416_936 (2026-07-24, dashu-int backend), 12, 50_125_644 -> 37_606_270 (2026-07-23, limb-wise wide-gamma decode + clone-free mixed add)
    pub const JOIN_BIGROOT: Envelope    = envelope(63_075_342,       20,   109_539_093); // 51_515_838 -> 50_460_273 -> 56_849_753 (2026-07-24, dashu-int backend), 16, 100_150_646 -> 87_631_272 -> 87_631_274 (2026-07-23, limb-wise wide-gamma decode + push-grow Builder; metered Base equality)
    pub const DECODE_HUGELEAF: Envelope = envelope(    58_604,        0,         2_443); //     55_827 -> 46_883 -> 48_851 (2026-07-24, dashu-int backend), 0, 122_132_816 -> 1_954 (2026-07-23, limb-wise wide-gamma decode)
    pub const JOIN_HUGELEAF: Envelope   = envelope(   139_714,        0,         9_777); //  3_127_365 -> 111_771 -> 115_707 (2026-07-24, dashu-int backend), 0, 122_138_683 -> 7_821 (2026-07-23, limb-wise wide-gamma decode + push-grow Builder)
    pub const ID_JOIN: Envelope         = envelope(   279_132,        0,             0); //    125_001 -> 223_305, 202 -> 0, 0 (2026-07-24, iterative id walks: frame bits on the heap, no grown segments)
    pub const ID_COVERS: Envelope       = envelope(        10,        0,             0); //          0 -> 8,  85 -> 0, 0 (2026-07-24, iterative id walks)
    pub const ID_DISJOINT: Envelope     = envelope(        10,        0,             0); //          0 -> 8, 170 -> 0, 0 (2026-07-24, iterative id walks)
    pub const ID_WITHOUT: Envelope      = envelope(   647_774,        0,             0); //    518_219, 138 -> 0, 0 (2026-07-23, iterative complement)
    pub const DECODE_CLIFF: Envelope    = envelope(   718_402,        0,        51_200); //    574_721 -> 607_489 (2026-07-24, dashu-int backend),   0,        40_960 (2026-07-23, new scenario)
    pub const CMP_CLIFF: Envelope       = envelope(       620,        0,       238_093); //        656 -> 496 (2026-07-24, dashu-int backend),   0,       190_474 (2026-07-23, new scenario)
    pub const JOIN_CLIFF: Envelope      = envelope( 1_723_362,        0,       480_010); //  1_378_689 -> 1_411_489 (2026-07-24, dashu-int backend),   0,       384_008 (2026-07-23, new scenario)
    // Skyline validator rows (2026-07-23, new scenarios): the V5
    // replacement's transient, achieved — the dense row's 49 KB peak over
    // 125k levels is ~3.1 bits per open ancestor (bit stack plus
    // reallocation growth) against DECODE_DENSE's 11 MB parse frames on
    // the same tree, ~56 B per level.
    pub const SKYLINE_VALIDATE_DENSE: Envelope      = envelope(    61_450,        0,       625_003); //     49_160, 0,   500_002
    pub const SKYLINE_VALIDATE_CLIFF: Envelope      = envelope(     1_770,        0,        12_903); //      1_416 -> 1_448 (2026-07-24, dashu-int backend), 0,    10_322
    pub const SKYLINE_VALIDATE_WIDE_TOOTH: Envelope = envelope(     1_520,        0,        42_325); //      1_216, 0,    33_860
    pub const SKYLINE_VALIDATE_HUGELEAF: Envelope   = envelope(    80_980,        0,         2_443); //     64_784 -> 66_752 (2026-07-24, dashu-int backend), 0,     1_954
    pub const SKYLINE_VALIDATE_ALT_SPINE: Envelope  = envelope(    61_450,        0,       625_003); //     49_160, 0,   500_002
    // Skyline decoder rows (2026-07-23, new scenarios): validate plus the
    // transcode back to the packed form, whose materialized heights and
    // floors price these against the packed output rather than the skyline
    // input (the module doc's cost section).
    pub const SKYLINE_DECODE_DENSE: Envelope        = envelope(22_672_865,        0,     3_437_515); // 18_138_292, 0, 2_750_012
    pub const SKYLINE_DECODE_CLIFF: Envelope        = envelope( 2_193_750,        0,       397_033); //  1_755_000 -> 1_787_704 (2026-07-24, dashu-int backend), 0,   317_626
    pub const SKYLINE_DECODE_WIDE_TOOTH: Envelope   = envelope( 2_083_300,        0,       463_554); //  1_666_640 -> 1_699_472 (2026-07-24, dashu-int backend), 0,   370_843
    pub const SKYLINE_DECODE_HUGELEAF: Envelope     = envelope(   117_414,        0,        12_217); //     93_931 -> 101_803 (2026-07-24, dashu-int backend), 0,     9_773
    pub const SKYLINE_DECODE_ALT_SPINE: Envelope    = envelope(19_723_745,        0,     3_437_515); // 15_778_996, 0, 2_750_012
}

// ─── meter liveness canaries ────────────────────────────────────────────────

/// Size of the canary allocation that proves the heap meter is live.
const CANARY_ALLOC_BYTES: usize = 1 << 20;

/// The heap meter registers a known allocation: a canary buffer reads back
/// a peak delta at least its own size, so a lost `#[global_allocator]` line
/// or a broken peak reader (either of which would pass every upper-bound
/// envelope vacuously at zero) fails loudly here instead.
#[test]
fn heap_meter_registers_known_allocation() {
    HEAP.reset_peak_usage();
    let baseline = HEAP.current_usage();
    let buf = std::hint::black_box(vec![0u8; CANARY_ALLOC_BYTES]);
    let peak = HEAP.peak_usage().saturating_sub(baseline);
    assert!(
        peak >= CANARY_ALLOC_BYTES,
        "heap meter read {peak} B for a {CANARY_ALLOC_BYTES} B canary allocation: \
         the counting allocator is not measuring this binary"
    );
    drop(buf);
}

/// The dense-spine decode registers at least its packed input size: the
/// decoded version owns a copy of the packed bits, so the one big scenario
/// here has a floor as well as a ceiling, and a dead heap meter cannot
/// slide a big scenario under its envelope at zero.
#[test]
fn heap_meter_floor_on_decode_dense() {
    let p = meter::dense(DENSE_DEPTH);
    HEAP.reset_peak_usage();
    let baseline = HEAP.current_usage();
    let v = version_of(&p);
    let peak = HEAP.peak_usage().saturating_sub(baseline);
    assert!(
        peak >= p.bytes.len(),
        "decode_dense peak {peak} B is under its {} B packed input: \
         the decoded version alone must allocate at least that",
        p.bytes.len(),
    );
    drop(v);
}

// ─── measurement harness ────────────────────────────────────────────────────

/// Appended to every envelope failure: the first cause to rule out is a
/// shared-process test runner, under which the process-global meters bleed
/// other tests' work into the scenario being measured.
const ISOLATION_NOTE: &str = "note: the meters are process-global and meaningful only one \
     scenario per process: run under cargo nextest, not a shared-process cargo test";

/// Run one scenario body under both meters and assert its envelope.
///
/// Prints the measured numbers (visible under `--no-capture` or on failure)
/// so re-pinning an envelope never requires editing the harness. The
/// scenario's result is returned alive, so the peak includes the fully
/// materialized output, and is dropped by the caller after measurement.
fn metered<R>(name: &str, input_bytes: usize, env: &Envelope, f: impl FnOnce() -> R) -> R {
    meter::reset_stack_segments();
    #[cfg(feature = "limb-meter")]
    meter::reset_limb_ops();
    HEAP.reset_peak_usage();
    let baseline = HEAP.current_usage();
    let r = f();
    let peak_heap = HEAP.peak_usage().saturating_sub(baseline);
    let segments = meter::stack_segments();
    #[cfg(feature = "limb-meter")]
    let limb_ops = meter::limb_ops();
    #[cfg(feature = "limb-meter")]
    eprintln!(
        "MEASURED {name}: input_bytes={input_bytes} peak_heap={peak_heap} segments={segments} limb_ops={limb_ops}"
    );
    #[cfg(not(feature = "limb-meter"))]
    eprintln!(
        "MEASURED {name}: input_bytes={input_bytes} peak_heap={peak_heap} segments={segments}"
    );
    assert!(
        peak_heap <= env.peak_heap,
        "{name}: peak heap {peak_heap} B exceeds the pinned envelope {} B (input {input_bytes} B): {ISOLATION_NOTE}",
        env.peak_heap,
    );
    assert!(
        segments <= env.segments,
        "{name}: {segments} grown stack segments exceed the pinned envelope {}: {ISOLATION_NOTE}",
        env.segments,
    );
    #[cfg(feature = "limb-meter")]
    assert!(
        limb_ops <= env.limb_ops,
        "{name}: {limb_ops} limb operations exceed the pinned envelope {}: {ISOLATION_NOTE}",
        env.limb_ops,
    );
    r
}

/// Decode a generated shape as a [`Version`], outside any measurement.
fn version_of(p: &meter::Packed) -> Version {
    Version::decode(&p.bytes[..]).expect("generated shape is strict normal form")
}

/// Decode a generated shape as a [`Party`], outside any measurement.
fn party_of(p: &meter::Packed) -> Party {
    Party::decode(&p.bytes[..]).expect("generated shape is strict normal form")
}

/// Assert a scenario result is consumed, so the operation cannot be
/// dead-code-eliminated and the walk provably ran to completion.
fn consumed<T: Debug>(v: T) -> String {
    format!("{v:?}")
}

// ─── dense spine scenarios ──────────────────────────────────────────────────

/// Decoding the dense spine stays within its pinned peak-heap and
/// stack-segment envelope (the parse-stack cost, linear in depth today).
#[test]
fn decode_dense_envelope() {
    let p = meter::dense(DENSE_DEPTH);
    let v = metered(
        "decode_dense",
        p.bytes.len(),
        &envelope::DECODE_DENSE,
        || version_of(&p),
    );
    drop(v);
}

/// Comparing the dense spine against the empty version stays within its
/// envelope (the recursion-frame cost: heap stays flat, segments do not).
#[test]
fn cmp_dense_envelope() {
    let p = meter::dense(DENSE_DEPTH);
    let v = version_of(&p);
    let r = metered("cmp_dense", p.bytes.len(), &envelope::CMP_DENSE, || {
        v.partial_cmp(&Version::new())
    });
    consumed(r);
}

/// Joining the dense spine with a one-tick version stays within its envelope
/// (the working-form and emit-path cost, linear in nodes today).
#[test]
fn join_dense_envelope() {
    let p = meter::dense(DENSE_DEPTH);
    let v = version_of(&p);
    let one = Version::try_from(1u64).expect("a one-tick version is valid");
    let joined = metered("join_dense", p.bytes.len(), &envelope::JOIN_DENSE, || {
        &v | &one
    });
    drop(joined);
}

/// Ticking the dense spine stays within its envelope (the working-form
/// round-trip cost, linear in nodes today).
#[test]
fn tick_dense_envelope() {
    let p = meter::dense(DENSE_DEPTH);
    let mut v = version_of(&p);
    let seed = Party::seed();
    metered("tick_dense", p.bytes.len(), &envelope::TICK_DENSE, || {
        v.tick(&seed)
    });
    drop(v);
}

// ─── bigroot scenarios ──────────────────────────────────────────────────────

/// Decoding bigroot stays within its envelope (one big-integer base plus the
/// parse stack).
#[test]
fn decode_bigroot_envelope() {
    let p = meter::bigroot(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
    let v = metered(
        "decode_bigroot",
        p.bytes.len(),
        &envelope::DECODE_BIGROOT,
        || version_of(&p),
    );
    drop(v);
}

/// Comparing bigroot against the empty version stays within its envelope
/// (today the worst amplifier: per-frame owned path sums, quadratic in the
/// root magnitude × depth).
#[test]
fn cmp_bigroot_envelope() {
    let p = meter::bigroot(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
    let v = version_of(&p);
    let r = metered("cmp_bigroot", p.bytes.len(), &envelope::CMP_BIGROOT, || {
        v.partial_cmp(&Version::new())
    });
    consumed(r);
}

/// Joining bigroot with a one-tick version stays within its envelope (the
/// same per-frame path-sum amplification on the combine path).
#[test]
fn join_bigroot_envelope() {
    let p = meter::bigroot(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
    let v = version_of(&p);
    let one = Version::try_from(1u64).expect("a one-tick version is valid");
    let joined = metered(
        "join_bigroot",
        p.bytes.len(),
        &envelope::JOIN_BIGROOT,
        || &v | &one,
    );
    drop(joined);
}

// ─── hugeleaf scenarios ─────────────────────────────────────────────────────

/// Decoding hugeleaf stays within its envelope (one gamma code as wide as
/// the whole input; the limb column pins the wide decode's linear limb
/// work, so a magnitude-superlinear regression fails this row first).
#[test]
fn decode_hugeleaf_envelope() {
    let p = meter::hugeleaf(HUGELEAF_MAGNITUDE_BITS);
    let v = metered(
        "decode_hugeleaf",
        p.bytes.len(),
        &envelope::DECODE_HUGELEAF,
        || version_of(&p),
    );
    drop(v);
}

/// Joining hugeleaf with a one-tick version stays within its envelope (the
/// emit path grows by push, so the peak tracks the result's node count;
/// the limb column tracks decode's, because reading the stored spilled
/// base runs the same linear wide-gamma decode).
#[test]
fn join_hugeleaf_envelope() {
    let p = meter::hugeleaf(HUGELEAF_MAGNITUDE_BITS);
    let v = version_of(&p);
    let one = Version::try_from(1u64).expect("a one-tick version is valid");
    let joined = metered(
        "join_hugeleaf",
        p.bytes.len(),
        &envelope::JOIN_HUGELEAF,
        || &v | &one,
    );
    drop(joined);
}

// ─── boundary comb scenarios ────────────────────────────────────────────────

/// Decoding the boundary comb stays within its envelope (every carry-cliff
/// crossing in the leaf values is paid for by a `2k + 1`-bit stored code, so
/// the parse's limb work stays linear per input bit).
#[test]
fn decode_cliff_envelope() {
    let p = meter::cliff_comb(CLIFF_SCALE, CLIFF_SCALE);
    let v = metered(
        "decode_cliff",
        p.bytes.len(),
        &envelope::DECODE_CLIFF,
        || version_of(&p),
    );
    drop(v);
}

/// Comparing the boundary comb against the empty version stays within its
/// envelope (each tooth's cliff excursion costs `Θ(k)` limb work bought by
/// its own `2k + 1`-bit stored magnitude, so the walk stays linear per
/// input bit — the property the comb exists to separate from codings that
/// store 3-bit deltas per crossing).
#[test]
fn cmp_cliff_envelope() {
    let p = meter::cliff_comb(CLIFF_SCALE, CLIFF_SCALE);
    let v = version_of(&p);
    let r = metered("cmp_cliff", p.bytes.len(), &envelope::CMP_CLIFF, || {
        v.partial_cmp(&Version::new())
    });
    consumed(r);
}

/// Joining the boundary comb with a one-tick version stays within its
/// envelope (the emit path re-codes every tooth magnitude, each paid for by
/// a comparably-wide input code).
#[test]
fn join_cliff_envelope() {
    let p = meter::cliff_comb(CLIFF_SCALE, CLIFF_SCALE);
    let v = version_of(&p);
    let one = Version::try_from(1u64).expect("a one-tick version is valid");
    let joined = metered("join_cliff", p.bytes.len(), &envelope::JOIN_CLIFF, || {
        &v | &one
    });
    drop(joined);
}

// ─── rank scenarios ─────────────────────────────────────────────────────────
//
// The rank fold and the Rank operations. The fold rows run on the
// digit-routed merge fold (child numerators land in their sibling's
// accumulator at the exponent gap, never through a materialized shift of
// the accumulated value), so their arithmetic shows up in the accumulator
// touch column; the limb column keeps the `Base` work (decode, the final
// conversion) honest, and the touch column is the liveness floor for the
// fold itself. RANK_HARMONIC is the fold's separating family — a
// numerator as wide as the depth already walked at every level — pinned
// linear (its limb column moved 134,740,995 -> 1,025 when the digit-routed
// fold landed). RANK_DENSE and RANK_BIGROOT are the controls: one-bit and
// root-heavy numerators respectively.
//
// RANK_PAIR_MISMATCH pins the class-first comparison's honest remainder
// (the subtraction and addition outputs' own content), and RANK_SUM_MIXED
// the raw-accumulator Sum (one normalization at the end; its limb column
// moved 156,312,196 -> 3,908 when the fold landed).

/// One rank scenario's pinned ceilings: [`Envelope`]'s three columns plus
/// accumulator digit touches, asserted when the `limb-meter` feature is
/// lit.
struct RankEnvelope {
    /// Peak heap delta over the scenario body, in bytes.
    peak_heap: usize,
    /// Stack segments grown during the scenario body.
    segments: u64,
    /// Big-integer limb operations counted during the scenario body.
    #[cfg(feature = "limb-meter")]
    limb_ops: u64,
    /// Accumulator digit touches counted during the scenario body.
    #[cfg(feature = "limb-meter")]
    touches: u64,
}

/// Build a [`RankEnvelope`] from the four pinned columns.
///
/// The limb and touch columns are carried only when the `limb-meter`
/// feature compiles their counters in; the leading underscores keep the
/// parameters warning-free in the other configuration.
const fn rank_envelope(
    peak_heap: usize,
    segments: u64,
    _limb_ops: u64,
    _touches: u64,
) -> RankEnvelope {
    RankEnvelope {
        peak_heap,
        segments,
        #[cfg(feature = "limb-meter")]
        limb_ops: _limb_ops,
        #[cfg(feature = "limb-meter")]
        touches: _touches,
    }
}

// The rank envelope table: pinned ceiling = measured ×1.25, rounded up,
// and only ever tightened: where a remeasure rises while staying inside
// an existing ceiling (the spilled-numerator heap cells, which carry the
// backend's `len/8 + 2` words of growth headroom per heap allocation),
// the older, tighter ceiling stands over the recorded movement. The
// trailing comment on each line is the measurement of record
// (2026-07-24, aarch64-apple-darwin, dev profile, three identical runs)
// the ceiling derives from; a re-pinned column records the movement as
// `old -> new`. Re-pin by rerunning under `--no-capture` with
// `--all-features` and reading the MEASURED lines.
#[rustfmt::skip]
mod rank_env {
    use super::{rank_envelope, RankEnvelope};
    //                                                            peak heap, segments,    limb ops, touches       measured: peak heap, segments, limb ops (movement), touches
    pub const RANK_DENSE: RankEnvelope         = rank_envelope(           0,      300,           3,      0); //          0, 240, 1_250_002 -> 2, 0
    pub const RANK_BIGROOT: RankEnvelope       = rank_envelope(      50_110,       20,       1_762,  5_862); //     40_088 -> 41_368 (2026-07-24, dashu-int backend),  16, 102_824 -> 1_409, 4_689
    pub const RANK_HARMONIC: RankEnvelope      = rank_envelope(      41_005,      155,       1_282, 84_403); //     32_804 -> 33_840 (2026-07-24, dashu-int backend), 124, 134_740_995 -> 1_025, 67_522
    pub const RANK_PAIR_MISMATCH: RankEnvelope = rank_envelope(     234_400,        0,      48_848,      0); //    187_520 -> 211_016 (2026-07-24, dashu-int backend),   0, 54_710 -> 39_078 (class-first cmp; the rest is checked_sub's and add's mandatory output), 0
    pub const RANK_SUM_MIXED: RankEnvelope     = rank_envelope(      78_140,        0,       4_885, 22_268); //     62_512,   0, 156_312_196 -> 3_908 (raw accumulator, one normalization), 17_814
}

/// Run one rank scenario body under all four meters and assert its
/// envelope.
///
/// [`metered`]'s harness plus the accumulator touch column; prints the
/// measured numbers so re-pinning never requires editing the harness.
fn rank_metered<R>(name: &str, input_bytes: usize, env: &RankEnvelope, f: impl FnOnce() -> R) -> R {
    meter::reset_stack_segments();
    #[cfg(feature = "limb-meter")]
    meter::reset_limb_ops();
    #[cfg(feature = "limb-meter")]
    meter::accum::touch_meter::reset();
    HEAP.reset_peak_usage();
    let baseline = HEAP.current_usage();
    let r = f();
    let peak_heap = HEAP.peak_usage().saturating_sub(baseline);
    let segments = meter::stack_segments();
    #[cfg(feature = "limb-meter")]
    let limb_ops = meter::limb_ops();
    #[cfg(feature = "limb-meter")]
    let touches = meter::accum::touch_meter::touches();
    #[cfg(feature = "limb-meter")]
    eprintln!(
        "MEASURED {name}: input_bytes={input_bytes} peak_heap={peak_heap} segments={segments} limb_ops={limb_ops} touches={touches}"
    );
    #[cfg(not(feature = "limb-meter"))]
    eprintln!(
        "MEASURED {name}: input_bytes={input_bytes} peak_heap={peak_heap} segments={segments}"
    );
    assert!(
        peak_heap <= env.peak_heap,
        "{name}: peak heap {peak_heap} B exceeds the pinned envelope {} B (input {input_bytes} B): {ISOLATION_NOTE}",
        env.peak_heap,
    );
    assert!(
        segments <= env.segments,
        "{name}: {segments} grown stack segments exceed the pinned envelope {}: {ISOLATION_NOTE}",
        env.segments,
    );
    #[cfg(feature = "limb-meter")]
    assert!(
        limb_ops <= env.limb_ops,
        "{name}: {limb_ops} limb operations exceed the pinned envelope {}: {ISOLATION_NOTE}",
        env.limb_ops,
    );
    #[cfg(feature = "limb-meter")]
    assert!(
        touches <= env.touches,
        "{name}: {touches} accumulator digit touches exceed the pinned envelope {}: {ISOLATION_NOTE}",
        env.touches,
    );
    r
}

/// The rank fold on the dense spine stays within its envelope (the
/// control: the spine's numerator stays one bit wide, so the fold's
/// per-level shifts are word-scale and the walk is linear).
#[test]
fn rank_dense_envelope() {
    let p = meter::dense(DENSE_DEPTH);
    let v = version_of(&p);
    let r = rank_metered("rank_dense", p.bytes.len(), &rank_env::RANK_DENSE, || {
        v.rank()
    });
    consumed(r);
}

/// The rank fold on the bigroot spine stays within its envelope (the
/// wide-magnitude control: one root-wide shift, then word-scale work).
#[test]
fn rank_bigroot_envelope() {
    let p = meter::bigroot(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
    let v = version_of(&p);
    let r = rank_metered(
        "rank_bigroot",
        p.bytes.len(),
        &rank_env::RANK_BIGROOT,
        || v.rank(),
    );
    consumed(r);
}

/// The rank fold on the harmonic spine stays within its envelope — the
/// fold's separating family, pinned linear: the accumulated numerator is
/// as wide as the depth already walked at every level, and the
/// digit-routed merge folds each level's one-leaf sibling into it at the
/// exponent gap instead of re-shifting it.
#[test]
fn rank_harmonic_envelope() {
    let p = meter::harmonic(RANK_HARMONIC_DEPTH);
    let v = version_of(&p);
    let r = rank_metered(
        "rank_harmonic",
        p.bytes.len(),
        &rank_env::RANK_HARMONIC,
        || v.rank(),
    );
    consumed(r);
}

/// `Rank::cmp` + `checked_sub` + `+` on the mismatched-exponent pair stay
/// within their envelope: the class-first comparison decides the order
/// and the pre-check in O(1), so the pinned cost is the `Some`-arm
/// subtraction and the addition — transients that are the outputs' own
/// value content, not amplification.
///
/// The pair is built through the public API outside measurement: the
/// dense spine's rank is the maximal-exponent operand (`1/2^d`, a
/// one-bit numerator, so the pinned cost is pure mismatch), against a
/// small integer rank at exponent zero.
#[test]
fn rank_pair_mismatch_envelope() {
    let a = version_of(&meter::dense(RANK_PAIR_DEPTH)).rank();
    let b = Version::try_from(3u64)
        .expect("a small integer version is valid")
        .rank();
    // Informational denominator: the pair's value content in bytes
    // (numerator bits + exponent, over eight).
    let content_bytes = RANK_PAIR_DEPTH / 8 + 1;
    let r = rank_metered(
        "rank_pair_mismatch",
        content_bytes,
        &rank_env::RANK_PAIR_MISMATCH,
        || {
            let ord = a.cmp(&b);
            let diff = b.checked_sub(&a);
            let sum = &a + &b;
            (ord, diff, sum)
        },
    );
    let (ord, diff, sum) = r;
    assert_eq!(ord, std::cmp::Ordering::Less, "1/2^d is under 3");
    assert!(
        diff.is_some(),
        "3 dominates 1/2^d, so the difference exists"
    );
    consumed((ord, diff, sum));
}

/// `Sum` over one high-exponent rank followed by many integer ranks stays
/// within its envelope: the raw accumulator anchors at the largest
/// exponent seen and digit-routes each summand in at its exponent gap,
/// normalizing once at the end, so the high-exponent operand costs its
/// own width once instead of once per later element.
///
/// High-first ordering was the adversarial arm of the fold's
/// order-dependence (`Sum` accepts arbitrary order, so the worst order is
/// the honest pin); under the raw accumulator it is the order that makes
/// every later add a shifted word, which is why the pin stays the
/// scenario of record.
#[test]
fn rank_sum_mixed_envelope() {
    let high = version_of(&meter::dense(RANK_SUM_EXP_DEPTH)).rank();
    let ones: Vec<before::Rank> = (0..RANK_SUM_COUNT)
        .map(|i| {
            Version::try_from(i as u64 % 7 + 1)
                .expect("a small integer version is valid")
                .rank()
        })
        .collect();
    let content_bytes = RANK_SUM_EXP_DEPTH / 8 + RANK_SUM_COUNT;
    let ranks: Vec<before::Rank> = std::iter::once(high).chain(ones).collect();
    let r = rank_metered(
        "rank_sum_mixed",
        content_bytes,
        &rank_env::RANK_SUM_MIXED,
        || ranks.into_iter().sum::<before::Rank>(),
    );
    consumed(r);
}

// ─── skyline codec scenarios ────────────────────────────────────────────────
//
// The skyline validator and decoder over the adversarial event families,
// with the stream transcoded outside measurement. The validator rows pin
// the V5 replacement's transient — ~2 bits of open-ancestor stack per
// level plus the cliff-immune accumulator — denominated against skyline
// input bytes; the decoder rows add the transcode back to the packed
// form, whose materialized heights and floors are priced by that packed
// output (on the comb it is quadratically larger than the skyline input,
// so no transcode can be skyline-linear; the validator is the piece that
// carries the wire-bit-linear claim).

/// The skyline stream of a packed family shape, built outside measurement.
fn skyline_of(p: &meter::Packed) -> meter::skyline::Encoded {
    meter::skyline::encode(&version_of(p))
}

/// The skyline validator on the dense spine stays within its envelope:
/// ~2 bits of transient per open ancestor (measured ~3.1 bits per level
/// including reallocation growth, against the old parse stack's ~56 bytes
/// per level on the same tree), zero grown segments.
#[test]
fn skyline_validate_dense_envelope() {
    let enc = skyline_of(&meter::dense(DENSE_DEPTH));
    let r = metered(
        "skyline_validate_dense",
        enc.bytes.len(),
        &envelope::SKYLINE_VALIDATE_DENSE,
        || meter::skyline::validate(&enc.bytes, enc.bits),
    );
    assert!(r.is_ok(), "the transcoded dense spine is canonical");
}

/// The skyline validator on the boundary comb stays within its envelope:
/// every 3-bit `±1` delta sits on the `2^k` carry boundary, and the
/// accumulator's redundant representation keeps the nonnegativity check
/// amortized O(1) per delta (the flatness pin below is the cross-scale
/// witness; a plain big-integer accumulator is quadratic here).
#[test]
fn skyline_validate_cliff_envelope() {
    let enc = skyline_of(&meter::cliff_comb(CLIFF_SCALE, CLIFF_SCALE));
    let r = metered(
        "skyline_validate_cliff",
        enc.bytes.len(),
        &envelope::SKYLINE_VALIDATE_CLIFF,
        || meter::skyline::validate(&enc.bytes, enc.bits),
    );
    assert!(r.is_ok(), "the transcoded boundary comb is canonical");
}

/// The skyline validator on the wide-tooth comb stays within its envelope:
/// each `±2^w` delta is a wide operand paid for by its own zigzag code, so
/// limb work stays linear per input bit at every tooth width.
#[test]
fn skyline_validate_wide_tooth_envelope() {
    let enc = skyline_of(&meter::wide_tooth_comb(
        CLIFF_SCALE,
        WIDE_TOOTH_WIDTH_BITS,
        CLIFF_SCALE,
    ));
    let r = metered(
        "skyline_validate_wide_tooth",
        enc.bytes.len(),
        &envelope::SKYLINE_VALIDATE_WIDE_TOOTH,
        || meter::skyline::validate(&enc.bytes, enc.bits),
    );
    assert!(r.is_ok(), "the transcoded wide-tooth comb is canonical");
}

/// The skyline validator on the hugeleaf analog — a single huge first
/// leaf, the whole stream one absolute gamma code — stays within its
/// envelope: one wide decode plus one wide accumulator load, both linear
/// in the code's own width.
#[test]
fn skyline_validate_hugeleaf_envelope() {
    let enc = skyline_of(&meter::hugeleaf(HUGELEAF_MAGNITUDE_BITS));
    let r = metered(
        "skyline_validate_hugeleaf",
        enc.bytes.len(),
        &envelope::SKYLINE_VALIDATE_HUGELEAF,
        || meter::skyline::validate(&enc.bytes, enc.bits),
    );
    assert!(r.is_ok(), "the transcoded hugeleaf is canonical");
}

/// The skyline validator on the alternating-binary spine stays within its
/// envelope: the direction of descent flips every level, so per-level
/// state is maximally non-uniform — and still costs 2 bits per level, not
/// a frame.
#[test]
fn skyline_validate_alt_spine_envelope() {
    let enc = skyline_of(&meter::alt_spine(DENSE_DEPTH));
    let r = metered(
        "skyline_validate_alt_spine",
        enc.bytes.len(),
        &envelope::SKYLINE_VALIDATE_ALT_SPINE,
        || meter::skyline::validate(&enc.bytes, enc.bits),
    );
    assert!(r.is_ok(), "the transcoded alternating spine is canonical");
}

/// The skyline decoder on the dense spine stays within its envelope (the
/// transcode materializes per-node floors, priced by the packed output).
#[test]
fn skyline_decode_dense_envelope() {
    let p = meter::dense(DENSE_DEPTH);
    let enc = skyline_of(&p);
    let v = metered(
        "skyline_decode_dense",
        enc.bytes.len(),
        &envelope::SKYLINE_DECODE_DENSE,
        || meter::skyline::decode(&enc.bytes, enc.bits).expect("canonical"),
    );
    assert_eq!(v, version_of(&p), "the transcode round-trips");
}

/// The skyline decoder on the boundary comb stays within its envelope:
/// the packed output stores a fresh `gamma(2^k − 1)` per tooth, so the
/// materialized heights and floors are output-sized — quadratically above
/// the skyline input, linearly within the packed form being rebuilt.
#[test]
fn skyline_decode_cliff_envelope() {
    let p = meter::cliff_comb(CLIFF_SCALE, CLIFF_SCALE);
    let enc = skyline_of(&p);
    let v = metered(
        "skyline_decode_cliff",
        enc.bytes.len(),
        &envelope::SKYLINE_DECODE_CLIFF,
        || meter::skyline::decode(&enc.bytes, enc.bits).expect("canonical"),
    );
    assert_eq!(v, version_of(&p), "the transcode round-trips");
}

/// The skyline decoder on the wide-tooth comb stays within its envelope
/// (wide heights and floors, output-priced like the boundary comb's).
#[test]
fn skyline_decode_wide_tooth_envelope() {
    let p = meter::wide_tooth_comb(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
    let enc = skyline_of(&p);
    let v = metered(
        "skyline_decode_wide_tooth",
        enc.bytes.len(),
        &envelope::SKYLINE_DECODE_WIDE_TOOTH,
        || meter::skyline::decode(&enc.bytes, enc.bits).expect("canonical"),
    );
    assert_eq!(v, version_of(&p), "the transcode round-trips");
}

/// The skyline decoder on the hugeleaf analog stays within its envelope
/// (one wide height, one wide floor, one wide re-emitted gamma code).
#[test]
fn skyline_decode_hugeleaf_envelope() {
    let p = meter::hugeleaf(HUGELEAF_MAGNITUDE_BITS);
    let enc = skyline_of(&p);
    let v = metered(
        "skyline_decode_hugeleaf",
        enc.bytes.len(),
        &envelope::SKYLINE_DECODE_HUGELEAF,
        || meter::skyline::decode(&enc.bytes, enc.bits).expect("canonical"),
    );
    assert_eq!(v, version_of(&p), "the transcode round-trips");
}

/// The skyline decoder on the alternating-binary spine stays within its
/// envelope (small heights and floors, one per node, output-priced).
#[test]
fn skyline_decode_alt_spine_envelope() {
    let p = meter::alt_spine(DENSE_DEPTH);
    let enc = skyline_of(&p);
    let v = metered(
        "skyline_decode_alt_spine",
        enc.bytes.len(),
        &envelope::SKYLINE_DECODE_ALT_SPINE,
        || meter::skyline::decode(&enc.bytes, enc.bits).expect("canonical"),
    );
    assert_eq!(v, version_of(&p), "the transcode round-trips");
}

// ─── skyline comparison sweep scenarios ─────────────────────────────────────
//
// The comparison sweep over skyline streams: one merge of the two leaf
// sequences on two path-bit stacks and one cliff-immune accumulator, no
// recursion anywhere. Streams are transcoded outside measurement. Each
// family scenario compares the shape against the empty version — the
// shallow-operand shape, where the whole deep side is consumed
// iteratively against a single depth-0 plateau — and the self scenario
// compares identical dense streams, so every boundary is an aligned tie
// and both cursors advance in lockstep to full depth. These rows carry a
// fourth column, packed-stream bits scanned, because the sweep's work is
// dominated by stream reads that allocate nothing, recurse nothing, and
// (off the cliff families) do almost no arithmetic — the scan column is
// the one that sees it.

/// One sweep scenario's pinned ceilings: [`Envelope`]'s three columns
/// plus scanned bits, asserted when the `scan-meter` feature is lit.
struct SweepEnvelope {
    /// Peak heap delta over the scenario body, in bytes.
    peak_heap: usize,
    /// Stack segments grown during the scenario body.
    segments: u64,
    /// Big-integer limb operations counted during the scenario body.
    #[cfg(feature = "limb-meter")]
    limb_ops: u64,
    /// Packed-stream bits scanned during the scenario body.
    #[cfg(feature = "scan-meter")]
    scan_bits: u64,
}

/// Build a [`SweepEnvelope`] from the four pinned columns.
///
/// The limb and scan columns are carried only when their features
/// compile the counters in; the leading underscores keep the parameters
/// warning-free in the other configurations.
const fn sweep_envelope(
    peak_heap: usize,
    segments: u64,
    _limb_ops: u64,
    _scan_bits: u64,
) -> SweepEnvelope {
    SweepEnvelope {
        peak_heap,
        segments,
        #[cfg(feature = "limb-meter")]
        limb_ops: _limb_ops,
        #[cfg(feature = "scan-meter")]
        scan_bits: _scan_bits,
    }
}

// The sweep envelope table: pinned ceiling = measured ×1.25, rounded
// up, and only ever tightened: where a remeasure rises while staying
// inside an existing ceiling (spilled-magnitude heap cells and their
// backend growth headroom), the older, tighter ceiling stands over the
// recorded movement. The trailing comment on each line is the measurement of record
// (2026-07-23, aarch64-apple-darwin, dev profile, three identical runs)
// the ceiling derives from. Re-pin by rerunning under `--no-capture`
// with `--all-features` and reading the MEASURED lines.
#[rustfmt::skip]
mod sweep_env {
    use super::{sweep_envelope, SweepEnvelope};
    //                                                               peak heap, segments, limb ops,  scan bits            measured: peak heap, segments, limb ops, scan bits
    pub const SKYLINE_CMP_DENSE: SweepEnvelope      = sweep_envelope(   30_730,        0,   312_503,   468_760); //   24_584, 0, 250_002, 375_008
    pub const SKYLINE_CMP_DENSE_SELF: SweepEnvelope = sweep_envelope(   51_210,        0,   625_005,   937_515); //   40_968, 0, 500_004, 750_012
    pub const SKYLINE_CMP_BIGROOT: SweepEnvelope    = sweep_envelope(   39_540,        0,    25_788,   137_514); //   31_632 -> 32_272 (2026-07-24, dashu-int backend), 0,  20_630, 110_011
    pub const SKYLINE_CMP_CLIFF: SweepEnvelope      = sweep_envelope(    1_450,        0,     7_763,    17_925); //    1_160 -> 1_296 (2026-07-23, emission-sweep shared step holds each consumed delta) -> 1_360 (2026-07-24, dashu-int backend), 0,   6_210,  14_340
    // SKYLINE_CMP_WIDE_TOOTH's 1_032-under-1_050 margin is a deliberate
    // change-detector on the backend's allocation policy: the committed
    // Cargo.lock (dashu-int 0.5.0 exact) is what makes the measurement
    // deterministic, and a cargo update to any other 0.5.x is a deliberate
    // re-measure event, not noise.
    pub const SKYLINE_CMP_WIDE_TOOTH: SweepEnvelope = sweep_envelope(    1_050,        0,    29_509, 1_000_483); //      840 -> 968 (2026-07-23, emission-sweep shared step holds each consumed delta) -> 1_032 (2026-07-24, dashu-int backend), 0,  23_607, 800_386
}

/// Run one sweep scenario body under all four meters and assert its
/// envelope.
///
/// [`metered`]'s harness plus the scan column; prints the measured
/// numbers so re-pinning never requires editing the harness.
fn sweep_metered<R>(
    name: &str,
    input_bytes: usize,
    env: &SweepEnvelope,
    f: impl FnOnce() -> R,
) -> R {
    meter::reset_stack_segments();
    #[cfg(feature = "limb-meter")]
    meter::reset_limb_ops();
    #[cfg(feature = "scan-meter")]
    meter::reset_scan_bits();
    HEAP.reset_peak_usage();
    let baseline = HEAP.current_usage();
    let r = f();
    let peak_heap = HEAP.peak_usage().saturating_sub(baseline);
    let segments = meter::stack_segments();
    #[cfg(feature = "limb-meter")]
    let limb_ops = meter::limb_ops();
    #[cfg(feature = "scan-meter")]
    let scan_bits = meter::scan_bits();
    #[cfg(feature = "limb-meter")]
    let limb_col = format!(" limb_ops={limb_ops}");
    #[cfg(not(feature = "limb-meter"))]
    let limb_col = "";
    #[cfg(feature = "scan-meter")]
    let scan_col = format!(" scan_bits={scan_bits}");
    #[cfg(not(feature = "scan-meter"))]
    let scan_col = "";
    eprintln!(
        "MEASURED {name}: input_bytes={input_bytes} peak_heap={peak_heap} segments={segments}{limb_col}{scan_col}"
    );
    assert!(
        peak_heap <= env.peak_heap,
        "{name}: peak heap {peak_heap} B exceeds the pinned envelope {} B (input {input_bytes} B): {ISOLATION_NOTE}",
        env.peak_heap,
    );
    assert!(
        segments <= env.segments,
        "{name}: {segments} grown stack segments exceed the pinned envelope {}: {ISOLATION_NOTE}",
        env.segments,
    );
    #[cfg(feature = "limb-meter")]
    assert!(
        limb_ops <= env.limb_ops,
        "{name}: {limb_ops} limb operations exceed the pinned envelope {}: {ISOLATION_NOTE}",
        env.limb_ops,
    );
    #[cfg(feature = "scan-meter")]
    assert!(
        scan_bits <= env.scan_bits,
        "{name}: {scan_bits} scanned bits exceed the pinned envelope {}: {ISOLATION_NOTE}",
        env.scan_bits,
    );
    r
}

/// The empty version's two-bit skyline stream: the shallow operand of
/// the family cmp scenarios.
fn skyline_empty() -> meter::skyline::Encoded {
    meter::skyline::encode(&Version::new())
}

/// The combined operand bytes of a sweep scenario.
fn sweep_input_bytes(a: &meter::skyline::Encoded, b: &meter::skyline::Encoded) -> usize {
    a.bytes.len() + b.bytes.len()
}

/// The sweep on the dense spine against the empty version stays within
/// its envelope: the deep side's 125k levels cost path *bits* (no grown
/// segments, heap in the path stack), consumed iteratively against one
/// depth-0 plateau.
#[test]
fn skyline_cmp_dense_envelope() {
    let a = skyline_of(&meter::dense(DENSE_DEPTH));
    let b = skyline_empty();
    let r = sweep_metered(
        "skyline_cmp_dense",
        sweep_input_bytes(&a, &b),
        &sweep_env::SKYLINE_CMP_DENSE,
        || meter::skyline::sweep::causal_cmp(&a, &b),
    );
    assert_eq!(
        r,
        Some(Ordering::Greater),
        "the dense spine strictly dominates the empty version"
    );
}

/// The sweep on two identical dense streams stays within its envelope:
/// every boundary is an aligned tie, both cursors advance in lockstep to
/// full depth, and the verdict is Equal only after both streams are
/// wholly consumed (no early exit anywhere).
#[test]
fn skyline_cmp_dense_self_envelope() {
    let a = skyline_of(&meter::dense(DENSE_DEPTH));
    let b = a.clone();
    let r = sweep_metered(
        "skyline_cmp_dense_self",
        sweep_input_bytes(&a, &b),
        &sweep_env::SKYLINE_CMP_DENSE_SELF,
        || meter::skyline::sweep::causal_cmp(&a, &b),
    );
    assert_eq!(r, Some(Ordering::Equal), "identical streams read equal");
}

/// The sweep on bigroot against the empty version stays within its
/// envelope: the difference accumulator absorbs the wide first height
/// once (paid by its own code) and every later delta is small.
#[test]
fn skyline_cmp_bigroot_envelope() {
    let a = skyline_of(&meter::bigroot(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH));
    let b = skyline_empty();
    let r = sweep_metered(
        "skyline_cmp_bigroot",
        sweep_input_bytes(&a, &b),
        &sweep_env::SKYLINE_CMP_BIGROOT,
        || meter::skyline::sweep::causal_cmp(&a, &b),
    );
    assert_eq!(
        r,
        Some(Ordering::Greater),
        "bigroot strictly dominates the empty version"
    );
}

/// The sweep on the boundary comb against the empty version stays within
/// its envelope: every 3-bit `±1` delta drives the running difference
/// across the `2^k` carry boundary, and the accumulator keeps each
/// crossing amortized O(1) (the flatness pin below is the cross-scale
/// witness).
#[test]
fn skyline_cmp_cliff_envelope() {
    let a = skyline_of(&meter::cliff_comb(CLIFF_SCALE, CLIFF_SCALE));
    let b = skyline_empty();
    let r = sweep_metered(
        "skyline_cmp_cliff",
        sweep_input_bytes(&a, &b),
        &sweep_env::SKYLINE_CMP_CLIFF,
        || meter::skyline::sweep::causal_cmp(&a, &b),
    );
    assert_eq!(
        r,
        Some(Ordering::Greater),
        "the comb strictly dominates the empty version"
    );
}

/// The sweep on the wide-tooth comb against the empty version stays
/// within its envelope: each `±2^w` delta is a genuinely wide operand
/// paid by its own zigzag code, so limb work stays linear per input bit
/// at every tooth width.
#[test]
fn skyline_cmp_wide_tooth_envelope() {
    let a = skyline_of(&meter::wide_tooth_comb(
        CLIFF_SCALE,
        WIDE_TOOTH_WIDTH_BITS,
        CLIFF_SCALE,
    ));
    let b = skyline_empty();
    let r = sweep_metered(
        "skyline_cmp_wide_tooth",
        sweep_input_bytes(&a, &b),
        &sweep_env::SKYLINE_CMP_WIDE_TOOTH,
        || meter::skyline::sweep::causal_cmp(&a, &b),
    );
    assert_eq!(
        r,
        Some(Ordering::Greater),
        "the wide-tooth comb strictly dominates the empty version"
    );
}

// ─── skyline join/meet emission scenarios ───────────────────────────────────
//
// The emission sweep over the same adversarial families: pointwise
// max/min re-delta-coded through the collapsing output builder, the
// measured result held alive so every peak includes the emitted stream.
// The columns carry the emission contract — zero grown segments (nothing
// recurses), heap in the cursor paths, the builder's bit stacks, and the
// output itself, limb work linear per input bit through the accumulator,
// and scanned-plus-written bits linear in the streams. The absorb row is
// the builder's collapse-heavy extreme: a flat dominating operand over
// the dense spine collapses the whole output to one leaf, one truncation
// per level around a held wide code — the shape whose cost a re-copying
// collapse discipline would make quadratic in depth times code width.

// The emission envelope table: pinned ceiling = measured ×1.25, rounded
// up, and only ever tightened: where a remeasure rises while staying
// inside an existing ceiling (spilled-magnitude heap cells and their
// backend growth headroom), the older, tighter ceiling stands over the
// recorded movement. The trailing comment on each line is the measurement of record
// (2026-07-23, aarch64-apple-darwin, dev profile, three identical runs)
// the ceiling derives from. Re-pin by rerunning under `--no-capture`
// with `--all-features` and reading the MEASURED lines.
#[rustfmt::skip]
mod emit_env {
    use super::{sweep_envelope, SweepEnvelope};
    //                                                                peak heap, segments, limb ops,  scan bits            measured: peak heap, segments, limb ops, scan bits
    pub const SKYLINE_JOIN_DENSE: SweepEnvelope      = sweep_envelope(  130_297,        0,   937_505,   625_018); //  104_237, 0, 750_004,   500_014
    pub const SKYLINE_JOIN_ABSORB: SweepEnvelope     = sweep_envelope(  270_798,        0,   942_389, 1_250_013); //  216_638 -> 218_606 (2026-07-24, dashu-int backend), 0, 753_911, 1_000_010
    pub const SKYLINE_JOIN_BIGROOT: SweepEnvelope    = sweep_envelope(   85_060,        0,    76_583,   275_028); //   71_768 -> 68_048 (2026-07-24, dashu-int backend), 0,  61_266,   220_022
    pub const SKYLINE_JOIN_CLIFF: SweepEnvelope      = sweep_envelope(    5_512,        0,    25_869,    35_848); //    4_409 -> 4_537 (2026-07-24, dashu-int backend), 0,  20_695,    28_678
    pub const SKYLINE_JOIN_WIDE_TOOTH: SweepEnvelope = sweep_envelope(  128_312,        0,    74_477, 2_000_963); //  102_649 -> 102_777 (2026-07-24, dashu-int backend), 0,  59_581, 1_600_770
    pub const SKYLINE_MEET_CLIFF: SweepEnvelope      = sweep_envelope(    5_002,        0,    18_020,    23_055); //    4_001 -> 4_065 (2026-07-24, dashu-int backend), 0,  14_416,    18_444
    pub const SKYLINE_MEET_WIDE_TOOTH: SweepEnvelope = sweep_envelope(  127_732,        0,    39_767, 1_005_613); //  102_185 -> 102_249 (2026-07-24, dashu-int backend), 0,  31_813,   804_490
}

/// The one-tick version's skyline stream: the shallow operand of the
/// family join/meet scenarios, mirroring the packed-form join rows.
fn skyline_one_tick() -> meter::skyline::Encoded {
    let one = Version::try_from(1u64).expect("a one-tick version is valid");
    meter::skyline::encode(&one)
}

/// One family shape and the packed-form oracle's answer against the
/// one-tick version, both as skyline streams built outside measurement,
/// so every scenario asserts byte-identity after its sweep.
fn skyline_oracle(
    p: &meter::Packed,
    join: bool,
) -> (meter::skyline::Encoded, meter::skyline::Encoded) {
    let v = version_of(p);
    let one = Version::try_from(1u64).expect("a one-tick version is valid");
    let out = if join { &v | &one } else { &v & &one };
    (meter::skyline::encode(&v), meter::skyline::encode(&out))
}

/// Joining the dense spine's skyline with a one-tick stream stays within
/// its envelope: the 125k-level walk emits and collapses on path-bit
/// stacks and one accumulator, with zero grown segments and the peak in
/// the emitted stream itself.
#[test]
fn skyline_join_dense_envelope() {
    let p = meter::dense(DENSE_DEPTH);
    let (a, expected) = skyline_oracle(&p, true);
    let b = skyline_one_tick();
    let out = sweep_metered(
        "skyline_join_dense",
        sweep_input_bytes(&a, &b),
        &emit_env::SKYLINE_JOIN_DENSE,
        || meter::skyline::emit::join(&a, &b),
    );
    assert_eq!(out, expected, "the emitted join must match the oracle");
}

/// Joining the dense spine's skyline with a dominating flat operand
/// stays within its envelope: the whole output collapses to one leaf
/// through 125k absorb steps around a held 125k-bit code, so this row
/// is linear only because absorb never moves the held code.
#[test]
fn skyline_join_absorb_envelope() {
    let p = meter::dense(DENSE_DEPTH);
    let flat = version_of(&meter::hugeleaf(HUGELEAF_MAGNITUDE_BITS));
    let a = skyline_of(&p);
    let b = meter::skyline::encode(&flat);
    let expected = b.clone();
    let out = sweep_metered(
        "skyline_join_absorb",
        sweep_input_bytes(&a, &b),
        &emit_env::SKYLINE_JOIN_ABSORB,
        || meter::skyline::emit::join(&a, &b),
    );
    assert_eq!(out, expected, "a dominating flat operand is the whole join");
}

/// Joining bigroot's skyline with a one-tick stream stays within its
/// envelope: the wide first height is absorbed once, paid by its own
/// code, and every later delta is small.
#[test]
fn skyline_join_bigroot_envelope() {
    let p = meter::bigroot(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
    let (a, expected) = skyline_oracle(&p, true);
    let b = skyline_one_tick();
    let out = sweep_metered(
        "skyline_join_bigroot",
        sweep_input_bytes(&a, &b),
        &emit_env::SKYLINE_JOIN_BIGROOT,
        || meter::skyline::emit::join(&a, &b),
    );
    assert_eq!(out, expected, "the emitted join must match the oracle");
}

/// Joining the boundary comb's skyline with a one-tick stream stays
/// within its envelope: every 3-bit `±1` delta re-emits across the
/// `2^k` carry boundary, and the accumulator keeps each crossing
/// amortized O(1).
#[test]
fn skyline_join_cliff_envelope() {
    let p = meter::cliff_comb(CLIFF_SCALE, CLIFF_SCALE);
    let (a, expected) = skyline_oracle(&p, true);
    let b = skyline_one_tick();
    let out = sweep_metered(
        "skyline_join_cliff",
        sweep_input_bytes(&a, &b),
        &emit_env::SKYLINE_JOIN_CLIFF,
        || meter::skyline::emit::join(&a, &b),
    );
    assert_eq!(out, expected, "the emitted join must match the oracle");
}

/// Joining the wide-tooth comb's skyline with a one-tick stream stays
/// within its envelope: each `±2^w` delta is a genuinely wide operand
/// re-coded into the output, paid by its own zigzag code.
#[test]
fn skyline_join_wide_tooth_envelope() {
    let p = meter::wide_tooth_comb(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
    let (a, expected) = skyline_oracle(&p, true);
    let b = skyline_one_tick();
    let out = sweep_metered(
        "skyline_join_wide_tooth",
        sweep_input_bytes(&a, &b),
        &emit_env::SKYLINE_JOIN_WIDE_TOOTH,
        || meter::skyline::emit::join(&a, &b),
    );
    assert_eq!(out, expected, "the emitted join must match the oracle");
}

/// Meeting the boundary comb's skyline with a one-tick stream stays
/// within its envelope: the output collapses to the flat one-tick leaf
/// through the absorb cascade while every comb delta still crosses the
/// carry boundary in the accumulator.
#[test]
fn skyline_meet_cliff_envelope() {
    let p = meter::cliff_comb(CLIFF_SCALE, CLIFF_SCALE);
    let (a, expected) = skyline_oracle(&p, false);
    let b = skyline_one_tick();
    let out = sweep_metered(
        "skyline_meet_cliff",
        sweep_input_bytes(&a, &b),
        &emit_env::SKYLINE_MEET_CLIFF,
        || meter::skyline::emit::meet(&a, &b),
    );
    assert_eq!(out, expected, "the emitted meet must match the oracle");
}

/// Meeting the wide-tooth comb's skyline with a one-tick stream stays
/// within its envelope: wide deltas are folded but never re-emitted
/// (the flat side wins everywhere), so the collapse discipline runs at
/// spilled operand widths.
#[test]
fn skyline_meet_wide_tooth_envelope() {
    let p = meter::wide_tooth_comb(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
    let (a, expected) = skyline_oracle(&p, false);
    let b = skyline_one_tick();
    let out = sweep_metered(
        "skyline_meet_wide_tooth",
        sweep_input_bytes(&a, &b),
        &emit_env::SKYLINE_MEET_WIDE_TOOTH,
        || meter::skyline::emit::meet(&a, &b),
    );
    assert_eq!(out, expected, "the emitted meet must match the oracle");
}

// The grow envelope table: pinned ceiling = measured ×1.25, rounded up.
// The trailing comment on each line is the measurement of record
// (2026-07-23, aarch64-apple-darwin, dev profile, three identical runs)
// the ceiling derives from. Re-pin by rerunning under `--no-capture`
// with `--all-features` and reading the MEASURED lines.
#[rustfmt::skip]
mod grow_env {
    use super::{sweep_envelope, SweepEnvelope};
    //                                                                peak heap, segments, limb ops,  scan bits            measured: peak heap, segments, limb ops, scan bits
    pub const SKYLINE_GROW_ALT_SPINE: SweepEnvelope  = sweep_envelope(  345_315,        0,        10, 1_406_282); //  276_252, 0,       8, 1_125_025
    pub const SKYLINE_GROW_PROBE_ALT_SPINE: SweepEnvelope = sweep_envelope(286_720,      0,         0,   468_760); //  229_376, 0,       0,   375_008
    pub const SKYLINE_GROW_ID_SPINE: SweepEnvelope   = sweep_envelope(  487_727,        0,   625_015, 2_812_519); //  390_181, 0, 500_012, 2_250_015
    pub const SKYLINE_GROW_CROSS: SweepEnvelope      = sweep_envelope(  669_830,        0,   625_010, 4_218_777); //  535_864, 0, 500_008, 3_375_021
}

/// One grow scenario's operand bytes: the skyline event stream plus the
/// packed id.
fn grow_input_bytes(ev: &meter::skyline::Encoded, id: &Party) -> usize {
    ev.bytes.len() + id.encoded_bits().div_ceil(8)
}

/// The probe alone on the frame-count adversary stays within its
/// envelope: the alternating spine packs one branch node into ~4 stream
/// bits, so this row's heap ceiling is the direct pin on the probe's
/// per-level frame state (the route is pre-allocated outside the
/// measurement) — bits per level, about one byte of stack per input
/// byte, where machine-word frames would cost ~32.
#[test]
fn skyline_grow_probe_alt_spine_envelope() {
    let v = version_of(&meter::alt_spine(DENSE_DEPTH));
    let party = Party::seed();
    let a = meter::skyline::encode(&v);
    let mut probe = meter::skyline::grow::Probe::for_operands(&a, &party);
    sweep_metered(
        "skyline_grow_probe_alt_spine",
        grow_input_bytes(&a, &party),
        &grow_env::SKYLINE_GROW_PROBE_ALT_SPINE,
        || probe.run(&a, &party),
    );
}

/// Growing the alternating spine's skyline under the seed party stays
/// within its envelope: the full id puts every one of the ~125k branch
/// nodes on the probe's frame stack at peak — the shape where one
/// machine-word frame per level would dwarf the ~4-bit-per-level input —
/// with zero grown segments and the frames held in bit stacks.
#[test]
fn skyline_grow_alt_spine_envelope() {
    let v = version_of(&meter::alt_spine(DENSE_DEPTH));
    let party = Party::seed();
    let a = meter::skyline::encode(&v);
    let expected = meter::skyline::encode(&meter::packed_grow(&v, &party));
    let out = sweep_metered(
        "skyline_grow_alt_spine",
        grow_input_bytes(&a, &party),
        &grow_env::SKYLINE_GROW_ALT_SPINE,
        || meter::skyline::grow::grow(&a, &party),
    );
    assert_eq!(out, expected, "the grown stream must match the oracle");
}

/// Growing the empty version under a 250k-deep unary id spine stays
/// within its envelope: the probe degenerates to the iterative id scan
/// (one Expand frame per level), the emit codes the whole expansion
/// chain as fresh one-bit deltas, and nothing recurses.
#[test]
fn skyline_grow_id_spine_envelope() {
    let v = Version::new();
    let party = party_of(&meter::id_spine(ID_DEPTH, false));
    let a = meter::skyline::encode(&v);
    let expected = meter::skyline::encode(&meter::packed_grow(&v, &party));
    let out = sweep_metered(
        "skyline_grow_id_spine",
        grow_input_bytes(&a, &party),
        &grow_env::SKYLINE_GROW_ID_SPINE,
        || meter::skyline::grow::grow(&a, &party),
    );
    assert_eq!(out, expected, "the grown stream must match the oracle");
}

/// Growing the alternating spine under a deep unary id spine stays
/// within its envelope: mixed regimes — two-cursor branch frames down
/// the shared spine, an id-only expansion where the id outruns the
/// event — with the same bit-stack ceilings as the pure shapes.
#[test]
fn skyline_grow_cross_envelope() {
    let v = version_of(&meter::alt_spine(DENSE_DEPTH));
    let party = party_of(&meter::id_spine(ID_DEPTH, false));
    let a = meter::skyline::encode(&v);
    let expected = meter::skyline::encode(&meter::packed_grow(&v, &party));
    let out = sweep_metered(
        "skyline_grow_cross",
        grow_input_bytes(&a, &party),
        &grow_env::SKYLINE_GROW_CROSS,
        || meter::skyline::grow::grow(&a, &party),
    );
    assert_eq!(out, expected, "the grown stream must match the oracle");
}

// ─── skyline cliff-immunity flatness ────────────────────────────────────────
//
// The cross-scale witness that the validator's nonnegativity state is
// cliff-immune on the boundary comb: per-delta accumulator digit touches
// and per-input-byte limb work both stay flat (×1.25) across a size
// doubling of `k = n`. A plain big-integer running height roughly doubles
// its per-unit cost per doubling here (the `meter/tier2` plain-sweep pin),
// so this is the row that separates the two representations — provided
// the height state actually runs on the metered accumulator. The touch
// column carries a liveness floor of one digit touch per delta (the
// counterpart of the heap meter's canaries): an unmetered height
// representation registers zero touches, and a flatness ratio over zeros
// holds vacuously, so the floor is what makes the flatness column a
// witness rather than a tautology.
#[cfg(feature = "limb-meter")]
mod skyline_flatness {
    use before::meter::{self, accum::touch_meter};

    /// Slack numerator over the small-scale cost (denominator
    /// [`SLACK_DEN`]): the ×1.25 flatness convention.
    const SLACK_NUM: u64 = 5;

    /// Slack denominator for the flatness bound.
    const SLACK_DEN: u64 = 4;

    /// One comb validation run: the two per-unit denominators (deltas for
    /// touches, skyline bytes for limb ops) and both counters.
    struct Run {
        deltas: u64,
        bytes: u64,
        touches: u64,
        limb_ops: u64,
    }

    /// Validate the `k = n = scale` boundary comb's skyline stream and
    /// record both counters over the validation body alone.
    ///
    /// Enforces the touch-meter liveness floor before returning: every
    /// delta code writes at least one accumulator digit, so a validator
    /// whose height state runs on anything but the metered accumulator
    /// (under which the flatness ratio holds vacuously at zero touches)
    /// fails loudly here instead. The metered accumulator measures about
    /// 1.6 touches per delta on this comb, so the one-touch floor is
    /// comfortable.
    fn comb_run(scale: usize) -> Run {
        let packed = meter::cliff_comb(scale, scale);
        let v = before::Version::decode(&packed.bytes[..]).expect("comb is strict normal form");
        let enc = meter::skyline::encode(&v);
        touch_meter::reset();
        meter::reset_limb_ops();
        meter::skyline::validate(&enc.bytes, enc.bits).expect("the comb stream is canonical");
        let run = Run {
            // 2n + 1 leaves: 2n delta codes follow the first leaf.
            deltas: 2 * scale as u64,
            bytes: enc.bytes.len() as u64,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert!(
            run.touches >= run.deltas,
            "skyline_comb scale {scale}: {} digit touches under the {}-delta floor: \
             the validator's height state is not running on the metered accumulator",
            run.touches,
            run.deltas,
        );
        run
    }

    /// Assert one per-unit cost stays flat (×1.25) across the doubling.
    fn assert_flat(name: &str, unit: &str, small: (u64, u64), large: (u64, u64)) {
        let (m1, n1) = small;
        let (m2, n2) = large;
        eprintln!(
            "MEASURED skyline_comb_{name}: small={m1}/{n1} large={m2}/{n2} \
             milli_per_{unit}={} -> {}",
            m1 * 1000 / n1,
            m2 * 1000 / n2,
        );
        assert!(
            u128::from(m2) * u128::from(n1) * u128::from(SLACK_DEN)
                <= u128::from(m1) * u128::from(n2) * u128::from(SLACK_NUM),
            "skyline_comb_{name}: per-{unit} cost grew more than x1.25 across the \
             size doubling: {m1}/{n1} -> {m2}/{n2}"
        );
    }

    /// The validator's per-delta accumulator touches and per-byte limb
    /// work stay flat across a `k = n` doubling of the boundary comb: the
    /// nonnegativity check is cliff-immune, achieved rather than promised.
    ///
    /// Each run also carries the one-touch-per-delta liveness floor (in
    /// [`comb_run`]), so flatness is asserted over a meter proven live.
    #[test]
    fn skyline_validate_cliff_cost_is_flat_per_unit() {
        let small = comb_run(512);
        let large = comb_run(1_024);
        assert_flat(
            "touches",
            "delta",
            (small.touches, small.deltas),
            (large.touches, large.deltas),
        );
        assert_flat(
            "limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
    }

    /// Compare the `k = n = scale` boundary comb's skyline stream against
    /// the empty version's and record both counters over the sweep body
    /// alone.
    ///
    /// Enforces the same touch-meter liveness floor as [`comb_run`]:
    /// every comb delta lands in the running difference, so a sweep whose
    /// difference state is not the metered accumulator fails loudly here
    /// instead of passing the flatness ratio vacuously at zero touches.
    fn comb_cmp_run(scale: usize) -> Run {
        let packed = meter::cliff_comb(scale, scale);
        let v = before::Version::decode(&packed.bytes[..]).expect("comb is strict normal form");
        let a = meter::skyline::encode(&v);
        let b = meter::skyline::encode(&before::Version::new());
        touch_meter::reset();
        meter::reset_limb_ops();
        let verdict = meter::skyline::sweep::causal_cmp(&a, &b);
        assert_eq!(
            verdict,
            Some(std::cmp::Ordering::Greater),
            "the comb strictly dominates the empty version"
        );
        let run = Run {
            // 2n + 1 leaves: 2n delta codes follow the first leaf.
            deltas: 2 * scale as u64,
            bytes: (a.bytes.len() + b.bytes.len()) as u64,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert!(
            run.touches >= run.deltas,
            "skyline_comb_cmp scale {scale}: {} digit touches under the {}-delta floor: \
             the sweep's difference state is not running on the metered accumulator",
            run.touches,
            run.deltas,
        );
        run
    }

    /// The sweep's per-delta accumulator touches and per-byte limb work
    /// stay flat across a `k = n` doubling of the boundary comb compared
    /// against the empty version: the running difference crosses the
    /// `2^k` carry boundary at every delta and each crossing stays
    /// amortized O(1) — the comparison-side cliff-immunity witness.
    ///
    /// Each run also carries the one-touch-per-delta liveness floor (in
    /// [`comb_cmp_run`]), so flatness is asserted over a meter proven
    /// live.
    #[test]
    fn skyline_cmp_cliff_cost_is_flat_per_unit() {
        let small = comb_cmp_run(512);
        let large = comb_cmp_run(1_024);
        assert_flat(
            "cmp_touches",
            "delta",
            (small.touches, small.deltas),
            (large.touches, large.deltas),
        );
        assert_flat(
            "cmp_limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
    }

    /// Tooth width (bits) one notch under the rank freeze threshold's
    /// 256-bit digit bound: the band's flat side.
    const FREEZE_BAND_UNDER_BITS: usize = 192;

    /// Tooth width (bits) one notch over the rank freeze threshold's
    /// 256-bit digit bound: every fold evicts the live component.
    const FREEZE_BAND_OVER_BITS: usize = 300;

    /// Cliff magnitude (bits) of the small freeze-band run; the frozen
    /// component's width, which the over-threshold regime re-reads per
    /// tooth.
    const FREEZE_BAND_SMALL_K: usize = 9_600;

    /// Tooth count of the small freeze-band run.
    const FREEZE_BAND_SMALL_N: usize = 128;

    /// One rank run over the wide-tooth comb `W(k, w, n)`'s skyline
    /// stream: the per-unit denominators (deltas, skyline bytes) and
    /// both counters over the rank body alone.
    ///
    /// Enforces the touch-meter liveness floor before returning — every
    /// delta code writes at least one accumulator digit, so a rank whose
    /// height state runs on anything but the metered accumulator fails
    /// loudly instead of passing a per-unit bound vacuously at zero
    /// touches — and pins the result against the packed rank, so the
    /// measured body is proven to compute the right answer.
    fn rank_wide_tooth_run(k: usize, w: usize, n: usize) -> Run {
        let packed = meter::wide_tooth_comb(k, w, n);
        let v = before::Version::decode(&packed.bytes[..]).expect("the comb is strict normal form");
        let enc = meter::skyline::encode(&v);
        touch_meter::reset();
        meter::reset_limb_ops();
        let r = meter::skyline::query::rank(&enc);
        let run = Run {
            // Each tooth's two leaves follow the first leaf as deltas.
            deltas: 2 * n as u64,
            bytes: enc.bytes.len() as u64,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert_eq!(r, v.rank(), "the kernel must match the packed rank");
        assert!(
            run.touches >= run.deltas,
            "skyline_rank_wide_tooth w={w}: {} digit touches under the {}-delta floor: \
             the rank height state is not running on the metered accumulator",
            run.touches,
            run.deltas,
        );
        run
    }

    /// Absolute over-threshold ceilings, measured 2026-07-24 ×1.25
    /// (three identical runs): the cured freeze discipline's numbers,
    /// tightened from the retired quadratic baseline of record
    /// (101,716 → 396,126 touches, 145,680 → 564,784 limbs at these
    /// scales) in the commit that landed the cure, per the ratchet
    /// convention. Measured: small 6,182 touches / 4,326 limbs on
    /// 24,085 skyline bytes; large 12,390 touches / 8,662 limbs on
    /// 48,245 bytes.
    const FREEZE_BAND_OVER_TOUCH_CEILINGS: (u64, u64) = (7_728, 15_488);

    /// The over-threshold limb ceilings paired with
    /// [`FREEZE_BAND_OVER_TOUCH_CEILINGS`].
    const FREEZE_BAND_OVER_LIMB_CEILINGS: (u64, u64) = (5_408, 10_828);

    /// The rank kernel's freeze band on the wide-tooth comb, both sides
    /// flat: bounded oscillation never freezes at any tooth width — a
    /// fold's cost rides the live component, paid by the tooth's own
    /// code — so per-byte cost stays flat (×1.25) across a doubling of
    /// `k` and `n` one notch under the freeze allowance's 256-bit digit
    /// bound (192-bit teeth) and one notch over it (300-bit teeth)
    /// alike, with the over side's absolute ceilings pinned as the
    /// tightened record that retired the frozen-width-per-tooth
    /// quadratic baseline.
    #[test]
    fn skyline_rank_wide_tooth_freeze_band() {
        let under_small = rank_wide_tooth_run(
            FREEZE_BAND_SMALL_K,
            FREEZE_BAND_UNDER_BITS,
            FREEZE_BAND_SMALL_N,
        );
        let under_large = rank_wide_tooth_run(
            2 * FREEZE_BAND_SMALL_K,
            FREEZE_BAND_UNDER_BITS,
            2 * FREEZE_BAND_SMALL_N,
        );
        assert_flat(
            "rank_under_threshold_touches",
            "byte",
            (under_small.touches, under_small.bytes),
            (under_large.touches, under_large.bytes),
        );
        let over_small = rank_wide_tooth_run(
            FREEZE_BAND_SMALL_K,
            FREEZE_BAND_OVER_BITS,
            FREEZE_BAND_SMALL_N,
        );
        let over_large = rank_wide_tooth_run(
            2 * FREEZE_BAND_SMALL_K,
            FREEZE_BAND_OVER_BITS,
            2 * FREEZE_BAND_SMALL_N,
        );
        for (run, (touch_ceiling, limb_ceiling), scale) in [
            (
                &over_small,
                (
                    FREEZE_BAND_OVER_TOUCH_CEILINGS.0,
                    FREEZE_BAND_OVER_LIMB_CEILINGS.0,
                ),
                "small",
            ),
            (
                &over_large,
                (
                    FREEZE_BAND_OVER_TOUCH_CEILINGS.1,
                    FREEZE_BAND_OVER_LIMB_CEILINGS.1,
                ),
                "large",
            ),
        ] {
            eprintln!(
                "MEASURED skyline_rank_over_threshold_{scale}: bytes={} touches={} limb_ops={}",
                run.bytes, run.touches, run.limb_ops,
            );
            assert!(
                run.touches <= touch_ceiling,
                "skyline_rank_over_threshold_{scale}: {} touches exceed the pinned \
                 ceiling {touch_ceiling}",
                run.touches,
            );
            assert!(
                run.limb_ops <= limb_ceiling,
                "skyline_rank_over_threshold_{scale}: {} limb ops exceed the pinned \
                 ceiling {limb_ceiling}",
                run.limb_ops,
            );
        }
        assert_flat(
            "rank_over_threshold_touches",
            "byte",
            (over_small.touches, over_small.bytes),
            (over_large.touches, over_large.bytes),
        );
        assert_flat(
            "rank_over_threshold_limb_ops",
            "byte",
            (over_small.limb_ops, over_small.bytes),
            (over_large.limb_ops, over_large.bytes),
        );
    }

    /// One rank run over the jump comb `J(k, n)`'s skyline stream: the
    /// per-unit denominators and both counters over the rank body alone.
    ///
    /// Carries the same liveness floor and packed-rank agreement as
    /// [`rank_wide_tooth_run`].
    fn rank_jump_run(k: usize, n: usize) -> Run {
        let packed = meter::jump_comb(k, n);
        let v = before::Version::decode(&packed.bytes[..]).expect("the comb is strict normal form");
        let enc = meter::skyline::encode(&v);
        touch_meter::reset();
        meter::reset_limb_ops();
        let r = meter::skyline::query::rank(&enc);
        let run = Run {
            // Each tooth's two leaves follow the first leaf as deltas.
            deltas: 2 * n as u64,
            bytes: enc.bytes.len() as u64,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert_eq!(r, v.rank(), "the kernel must match the packed rank");
        assert!(
            run.touches >= run.deltas,
            "skyline_rank_jump k={k}: {} digit touches under the {}-delta floor: \
             the rank height state is not running on the metered accumulator",
            run.touches,
            run.deltas,
        );
        run
    }

    /// Absolute jump-comb ceilings, measured 2026-07-24 ×1.25 (three
    /// identical runs): one eviction of the `k`-bit jump plus flat
    /// 3-bit-delta work — the un-evicted alternative reads the jump's
    /// width again on every following delta, ~15× these numbers at the
    /// small scale alone. Measured: small 5,138 touches / 2,128 limbs
    /// on 4,961 skyline bytes; large 10,272 touches / 4,250 limbs on
    /// 9,921 bytes.
    const RANK_JUMP_TOUCH_CEILINGS: (u64, u64) = (6_423, 12_840);

    /// The jump-comb limb ceilings paired with
    /// [`RANK_JUMP_TOUCH_CEILINGS`].
    const RANK_JUMP_LIMB_CEILINGS: (u64, u64) = (2_660, 5_313);

    /// The rank kernel's freeze eviction on the jump comb is funded and
    /// flat: the mid-stream `k`-bit jump lands in the live component,
    /// the first cheap delta behind it fires the one freeze — priced by
    /// the drift the jump's own code paid for, never by the frozen
    /// width — and every later 3-bit delta rides an emptied live
    /// component, so per-byte cost stays flat (×1.25) across a doubling
    /// of `k` and `n` under absolute ceilings a stale-drift regression
    /// (the jump re-read per delta) exceeds ~15-fold.
    #[test]
    fn skyline_rank_jump_eviction_is_flat_per_unit() {
        let small = rank_jump_run(FREEZE_BAND_SMALL_K, FREEZE_BAND_SMALL_N);
        let large = rank_jump_run(2 * FREEZE_BAND_SMALL_K, 2 * FREEZE_BAND_SMALL_N);
        for (run, (touch_ceiling, limb_ceiling), scale) in [
            (
                &small,
                (RANK_JUMP_TOUCH_CEILINGS.0, RANK_JUMP_LIMB_CEILINGS.0),
                "small",
            ),
            (
                &large,
                (RANK_JUMP_TOUCH_CEILINGS.1, RANK_JUMP_LIMB_CEILINGS.1),
                "large",
            ),
        ] {
            eprintln!(
                "MEASURED skyline_rank_jump_{scale}: bytes={} touches={} limb_ops={}",
                run.bytes, run.touches, run.limb_ops,
            );
            assert!(
                run.touches <= touch_ceiling,
                "skyline_rank_jump_{scale}: {} touches exceed the pinned ceiling \
                 {touch_ceiling}: the jump's drift is not being evicted once",
                run.touches,
            );
            assert!(
                run.limb_ops <= limb_ceiling,
                "skyline_rank_jump_{scale}: {} limb ops exceed the pinned ceiling \
                 {limb_ceiling}: the jump's drift is not being evicted once",
                run.limb_ops,
            );
        }
        assert_flat(
            "rank_jump_touches",
            "byte",
            (small.touches, small.bytes),
            (large.touches, large.bytes),
        );
        assert_flat(
            "rank_jump_limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
    }
}

// ─── id spine pair scenarios ────────────────────────────────────────────────

/// The combined operand bytes of an id-spine pair scenario.
fn id_pair_input_bytes(a: &meter::Packed, b: &meter::Packed) -> usize {
    a.bytes.len() + b.bytes.len()
}

/// Joining the diverted id-spine pair stays within its envelope (the
/// two-tree walk runs to full lockstep depth; its iterative frames must
/// grow no stack segments) and produces the construction-known bytes.
///
/// The output pin: the divert arms are the two children of the node at
/// depth `d − 1`, so their union collapses that node to a terminal — the
/// joined party is exactly the spine one level shorter, byte for byte.
#[test]
fn id_join_envelope() {
    let pa = meter::id_spine(ID_DEPTH, false);
    let pb = meter::id_spine(ID_DEPTH, true);
    let input = id_pair_input_bytes(&pa, &pb);
    let mut a = party_of(&pa);
    let b = party_of(&pb);
    let r = metered("id_join", input, &envelope::ID_JOIN, || a.join(b));
    assert!(
        r.is_ok(),
        "the divert arms are disjoint, so join must succeed"
    );
    assert_eq!(
        a.encode(),
        meter::id_spine(ID_DEPTH - 1, false).bytes,
        "the divert arms rejoin into the spine one level shorter"
    );
}

/// `covers` over the diverted id-spine pair stays within its envelope (the
/// two-tree walk runs to full lockstep depth; its iterative frames must
/// grow no stack segments).
#[test]
fn id_covers_envelope() {
    let pa = meter::id_spine(ID_DEPTH, false);
    let pb = meter::id_spine(ID_DEPTH, true);
    let input = id_pair_input_bytes(&pa, &pb);
    let a = party_of(&pa);
    let b = party_of(&pb);
    let r = metered("id_covers", input, &envelope::ID_COVERS, || a.covers(&b));
    assert!(
        !r,
        "the divert arms are disjoint, so neither covers the other"
    );
}

/// `is_disjoint` over the diverted id-spine pair stays within its envelope
/// (the walk runs to completion: the pair is disjoint, so no early exit).
#[test]
fn id_disjoint_envelope() {
    let pa = meter::id_spine(ID_DEPTH, false);
    let pb = meter::id_spine(ID_DEPTH, true);
    let input = id_pair_input_bytes(&pa, &pb);
    let a = party_of(&pa);
    let b = party_of(&pb);
    let r = metered("id_disjoint", input, &envelope::ID_DISJOINT, || {
        a.is_disjoint(&b)
    });
    assert!(r, "the divert arms own disjoint regions");
}

/// Joining an id spine with its exact complement yields the seed, byte for
/// byte.
///
/// A region and its complement union to the full owner. On the spine pair
/// the collapse cascades: the deepest output node closes as two terminals,
/// which turns its parent into two terminals, and so on for all `d` levels
/// up to the root — the deep-shape pin on the join emitter's close-time
/// `(1, 1) → 1` repair, whose output is the seed's single-terminal encoding.
#[test]
fn id_join_spine_with_complement_collapses_to_seed() {
    let pa = meter::id_spine(ID_DEPTH, false);
    let mut a = party_of(&pa);
    let complement = Party::seed()
        .without(&a)
        .expect("the seed strictly covers a spine, so the complement is non-empty");
    a.join(complement)
        .expect("a region and its complement are disjoint");
    assert_eq!(
        a.encode(),
        Party::seed().encode(),
        "a spine and its complement union to the full seed region"
    );
}

/// Joining spines that lean into opposite root halves splices both operands
/// verbatim under a root fork, byte for byte.
///
/// The operands share no level below the root, so no output node collapses:
/// the joined encoding is the both-children root tag followed by each
/// operand's subtree bits unchanged — the deep-shape pin that the join
/// emitter copies non-overlapping subtrees without rewriting them.
#[test]
fn id_join_opposite_spines_splice_verbatim() {
    let pa = meter::id_spine(ID_DEPTH, false);
    let pb_tags = right_spine_tags(ID_DEPTH);
    let mut a = party_of(&pa);
    let b = Party::decode(&pack_bits(&pb_tags)[..])
        .expect("the right-leaning spine is strict normal form");
    a.join(b)
        .expect("spines in opposite root halves are disjoint");
    // The expected bytes, assembled from the constructions: a both-children
    // root tag, then each operand's bits below its root node, verbatim.
    let mut expected = vec![true, true];
    expected.extend((2..pa.bits).map(|i| packed_bit(&pa.bytes, i)));
    expected.extend_from_slice(&pb_tags[2..]);
    assert_eq!(
        a.encode(),
        pack_bits(&expected),
        "opposite spines splice verbatim under a root fork"
    );
}

/// The right-leaning id spine's tag stream: `d` right-only tags ending in a
/// terminal, the mirror of [`meter::id_spine`]'s unary chain.
fn right_spine_tags(d: usize) -> Vec<bool> {
    let mut tags = Vec::with_capacity(2 * d + 2);
    for _ in 0..d {
        tags.push(false); // left child absent ...
        tags.push(true); // ... right child present
    }
    tags.push(false); // terminal tag "00": the single owned tip
    tags.push(false);
    tags
}

/// Pack a most-significant-bit-first bit stream into zero-padded bytes, the
/// generators' packing convention.
fn pack_bits(bits: &[bool]) -> Vec<u8> {
    let mut bytes = vec![0u8; bits.len().div_ceil(8)];
    for (i, &bit) in bits.iter().enumerate() {
        if bit {
            bytes[i / 8] |= 0x80 >> (i % 8);
        }
    }
    bytes
}

/// Read live bit `i` of a packed stream (most significant bit first).
fn packed_bit(bytes: &[u8], i: usize) -> bool {
    bytes[i / 8] & (0x80 >> (i % 8)) != 0
}

/// `without` subtracting an id spine from the seed stays within its envelope
/// (the complement emitter is iterative, so the subtrahend's depth alone
/// must grow no stack segments).
#[test]
fn id_without_envelope() {
    let pb = meter::id_spine(ID_DEPTH, true);
    let input = pb.bytes.len();
    let b = party_of(&pb);
    let r = metered("id_without", input, &envelope::ID_WITHOUT, || {
        Party::seed().without(&b)
    });
    assert!(
        r.is_some(),
        "the seed strictly covers a spine, so the complement is non-empty"
    );
}

// ─── accumulator stream scenarios ───────────────────────────────────────────
//
// The digit-touch cost of the cliff-immune accumulator on the adversarial
// families' delta streams, with the sign read after every delta (the read
// the sweeps depend on), plus the read-heavy stream where the sign folds
// outnumber the writes. Each scenario runs at a base scale and its
// doubling under the same per-stream ceiling — pinning the per-unit cost
// (per delta, per coded byte where the deltas themselves widen, or per
// sign read where reads dominate) *flat* across the doubling is the
// linearity claim — plus an explicit cross-scale ratio bound. Touch
// counts are deterministic, so the ceilings are exact-measured ×1.25 like
// every other column; the counter exists only under the `limb-meter`
// feature, which is the whole scenario's gate.
#[cfg(feature = "limb-meter")]
mod accum_streams {
    use std::cmp::Ordering;

    use before::meter::accum::{touch_meter, Accum};
    use dashu_int::UBig;

    /// Slack numerator over the measured value, matching the ×1.25 envelope
    /// convention (denominator [`SLACK_DEN`]).
    const SLACK_NUM: u64 = 5;

    /// Slack denominator: ceilings and flatness bounds are measured ×5/4.
    const SLACK_DEN: u64 = 4;

    /// One accumulator stream measurement: the linearity denominator
    /// (delta count, coded bytes where deltas widen, or sign reads where
    /// reads dominate) and the digit touches counted over the stream body
    /// (setup excluded).
    struct Run {
        denominator: u64,
        touches: u64,
    }

    /// Assert a two-scale stream family stays under its pinned per-unit
    /// ceiling at both scales and flat (×1.25) across the doubling.
    fn assert_flat(name: &str, small: &Run, large: &Run, ceiling_milli_per_unit: u64) {
        for run in [small, large] {
            eprintln!(
                "MEASURED accum_{name}: denominator={} touches={} milli_per_unit={}",
                run.denominator,
                run.touches,
                run.touches * 1000 / run.denominator,
            );
            assert!(
                u128::from(run.touches) * 1000
                    <= u128::from(ceiling_milli_per_unit) * u128::from(run.denominator),
                "accum_{name}: {} touches over {} units exceed the pinned \
                 {ceiling_milli_per_unit} milli-touches per unit",
                run.touches,
                run.denominator,
            );
        }
        assert!(
            u128::from(large.touches) * u128::from(small.denominator) * u128::from(SLACK_DEN)
                <= u128::from(small.touches)
                    * u128::from(large.denominator)
                    * u128::from(SLACK_NUM),
            "accum_{name}: per-unit touch cost grew more than ×1.25 across the \
             size doubling: {}/{} -> {}/{}",
            small.touches,
            small.denominator,
            large.touches,
            large.denominator,
        );
    }

    /// The boundary-comb delta stream: setup `2^k − 1`, then `2n` deltas of
    /// `±1` oscillating across the `2^k` cliff, sign read after each.
    fn comb_run(k: u32, n: usize) -> Run {
        let mut acc = Accum::new();
        acc.add_wide(&((UBig::from(1u8) << k as usize) - 1u8));
        touch_meter::reset();
        for _ in 0..n {
            acc.add_small(1);
            assert_eq!(acc.sign(), Ordering::Greater, "at 2^k");
            acc.sub_small(1);
            assert_eq!(acc.sign(), Ordering::Greater, "back at 2^k - 1");
        }
        Run {
            denominator: 2 * n as u64,
            touches: touch_meter::touches(),
        }
    }

    /// The wide-tooth delta stream: setup `2^k`, then `2n` deltas of `±2^w`
    /// oscillating across the `2^k` cliff, sign read after each.
    fn wide_tooth_run(k: u32, w: u32, n: usize) -> Run {
        let tooth = UBig::from(1u8) << w as usize;
        let mut acc = Accum::new();
        acc.add_wide(&(UBig::from(1u8) << k as usize));
        touch_meter::reset();
        for _ in 0..n {
            acc.sub_wide(&tooth);
            assert_eq!(acc.sign(), Ordering::Greater, "below the cliff");
            acc.add_wide(&tooth);
            assert_eq!(acc.sign(), Ordering::Greater, "back at the cliff");
        }
        Run {
            denominator: 2 * n as u64,
            touches: touch_meter::touches(),
        }
    }

    /// The cancelling-prefix chain stream: setup `2^k`, then `2n` deltas of
    /// `∓(2^k − 1)` dropping to 1 and back, sign read after each.
    ///
    /// The deltas themselves are `k` bits wide, so the linearity
    /// denominator is the stream's own coded size — `2n` zigzag-gamma codes
    /// of `2k + 3` bits each — in bytes, not the delta count.
    fn cancelling_run(k: u32, n: usize) -> Run {
        let drop = (UBig::from(1u8) << k as usize) - 1u8;
        let mut acc = Accum::new();
        acc.add_wide(&(UBig::from(1u8) << k as usize));
        touch_meter::reset();
        for _ in 0..n {
            acc.sub_wide(&drop);
            assert_eq!(acc.sign(), Ordering::Greater, "down at 1");
            acc.add_wide(&drop);
            assert_eq!(acc.sign(), Ordering::Greater, "back at the peak");
        }
        Run {
            denominator: (2 * n as u64) * (2 * u64::from(k) + 3) / 8,
            touches: touch_meter::touches(),
        }
    }

    /// The static-prefix read stream: a cancelling prefix built once
    /// (`+2^k` then `−(2^k − 1)`, leaving value 1 spelled across `k/32`
    /// wide digits), then `n` cycles of `add_small(1)` / sign /
    /// `sub_small(1)` / sign. Setup is excluded from the count; the
    /// linearity denominator is the `2n` sign reads.
    ///
    /// Unlike [`cancelling_run`], no wide write precedes the reads: the
    /// first sign fold must scan the whole prefix, and only its collapse
    /// keeps every later read from re-scanning it. A no-collapse
    /// implementation reads the full `k/32`-digit prefix on every sign
    /// here, so its per-read cost grows linearly with `k` instead of
    /// staying flat.
    fn static_prefix_run(k: u32, n: usize) -> Run {
        let drop = (UBig::from(1u8) << k as usize) - 1u8;
        let mut acc = Accum::new();
        acc.add_wide(&(UBig::from(1u8) << k as usize));
        acc.sub_wide(&drop);
        touch_meter::reset();
        for _ in 0..n {
            acc.add_small(1);
            assert_eq!(acc.sign(), Ordering::Greater, "up at 2");
            acc.sub_small(1);
            assert_eq!(acc.sign(), Ordering::Greater, "back at 1");
        }
        Run {
            denominator: 2 * n as u64,
            touches: touch_meter::touches(),
        }
    }

    /// The boundary-comb stream's per-delta digit-touch cost stays under
    /// its pinned ceiling at both scales and flat across the `k`, `n`
    /// doubling (the shape where a normalized representation is quadratic).
    #[test]
    fn accum_comb_touches_flat() {
        let small = comb_run(4_096, 50_000);
        let large = comb_run(8_192, 100_000);
        assert_flat("comb", &small, &large, envelope::COMB_MILLI_PER_DELTA);
    }

    /// The unpaid-crossing fan's entry/exit stream — the root magnitude
    /// paid once, then `±1` path-sum crossings per tooth — stays under the
    /// same pinned per-delta ceiling, flat across the doubling.
    ///
    /// The fan prices Dyck-walk accumulation where the boundary comb prices
    /// consecutive-leaf deltas; per delta the two streams are the same
    /// arithmetic, and the pinned ceiling says so.
    #[test]
    fn accum_fan_touches_flat() {
        let small = comb_run(4_096, 50_000);
        let large = comb_run(8_192, 100_000);
        assert_flat("fan", &small, &large, envelope::COMB_MILLI_PER_DELTA);
    }

    /// The wide-tooth stream's per-delta digit-touch cost (tooth width
    /// fixed, cliff height and tooth count doubling) stays under its pinned
    /// ceiling at both scales and flat across the doubling — the stream on
    /// which any normalized-prefix-plus-window form is quadratic.
    #[test]
    fn accum_wide_tooth_touches_flat() {
        let small = wide_tooth_run(4_096, 192, 25_000);
        let large = wide_tooth_run(8_192, 192, 50_000);
        assert_flat(
            "wide_tooth",
            &small,
            &large,
            envelope::WIDE_TOOTH_MILLI_PER_DELTA,
        );
    }

    /// The cancelling-prefix chain's digit-touch cost per coded byte of its
    /// own wide deltas stays under its pinned ceiling at both scales and
    /// flat across the doubling: every deep sign scan here is funded by the
    /// wide delta immediately preceding it, so the stream's cost tracks its
    /// own coded size (the collapse itself is priced by
    /// `accum_static_prefix_touches_flat`, where no adjacent write funds
    /// the scans).
    #[test]
    fn accum_cancelling_touches_flat() {
        let small = cancelling_run(2_048, 4_096);
        let large = cancelling_run(4_096, 8_192);
        assert_flat(
            "cancelling",
            &small,
            &large,
            envelope::CANCELLING_MILLI_PER_CODED_BYTE,
        );
    }

    /// The static-prefix read stream's digit-touch cost per sign read
    /// stays under its pinned ceiling at both scales and flat across the
    /// `k`, `n` doubling: the sign fold's collapse pays for the deep scan
    /// exactly once, so a cancelling prefix built once and then read many
    /// times costs O(1) digit touches per read.
    ///
    /// This is the pin that makes the collapse load-bearing: the other
    /// three streams fund every deep scan with an immediately preceding
    /// wide write, so they stay flat even with the collapse deleted; only
    /// this stream's ceiling breaks (by a factor growing linearly in `k`)
    /// when a sign fold leaves the scanned prefix in place.
    #[test]
    fn accum_static_prefix_touches_flat() {
        let small = static_prefix_run(2_048, 10_000);
        let large = static_prefix_run(4_096, 20_000);
        assert_flat(
            "static_prefix",
            &small,
            &large,
            envelope::STATIC_PREFIX_MILLI_PER_READ,
        );
    }

    // The pinned per-unit ceilings: measured ×1.25, rounded up, in
    // milli-touches per unit (per delta, per coded byte for the chain, per
    // sign read for the static prefix).
    // The trailing comment on each line is the measurement of record the
    // ceiling derives from; re-pin from the MEASURED lines under
    // `--no-capture` with `--all-features`.
    #[rustfmt::skip]
    mod envelope {
        //                                                     ceiling         measured (2026-07-23, aarch64-apple-darwin, dev profile, three identical runs)
        pub const COMB_MILLI_PER_DELTA: u64            = 2_500; // 2_000.00 at k=4096/n=50_000 and k=8192/n=100_000 (also the fan row)
        pub const WIDE_TOOTH_MILLI_PER_DELTA: u64      = 7_501; // 6_000.08 at w=192, k=4096/n=25_000; 6_000.04 at k=8192/n=50_000
        pub const CANCELLING_MILLI_PER_CODED_BYTE: u64 =   314; // 250.81 at k=2048/n=4096; 250.40 at k=4096/n=8192
        pub const STATIC_PREFIX_MILLI_PER_READ: u64    = 2_509; // 2_006.50 at k=2048/n=10_000; 2_006.45 at k=4096/n=20_000
    }
}

// ─── skyline query-fold scenarios ───────────────────────────────────────────
//
// The query kernels over skyline streams: rank on the frozen/live height
// split, min_ticks on the word stack with its early saturation exit, and
// projection against a packed id. Streams are transcoded outside
// measurement. These rows carry all five columns — heap, segments, limbs,
// scanned bits, and accumulator touches — because the kernels' arithmetic
// lives in digit touches (the limb column alone would read a vacuous
// near-zero), while their stream work lives in the scan column. The cliff
// and wide-tooth rank rows are load-bearing live-path pins: wide deltas
// ride the live component without freezing — the comb's terminal borrow
// and every 192-bit tooth are each paid by their own codes — and the
// `skyline_flatness` module's freeze-band and jump rows pin the freeze
// discipline itself (bounded oscillation never freezes at any width;
// stale drift is evicted once, at the drift's own width). The projection
// row is I/O-denominated per the board's criterion: its output is
// mandatory and dominates its input, so the pinned ceilings price
// input + output bytes (the MEASURED line prints both).

/// One query scenario's pinned ceilings: [`Envelope`]'s three columns
/// plus scanned bits and accumulator touches, asserted when their
/// features are lit.
struct QueryEnvelope {
    /// Peak heap delta over the scenario body, in bytes.
    peak_heap: usize,
    /// Stack segments grown during the scenario body.
    segments: u64,
    /// Big-integer limb operations counted during the scenario body.
    #[cfg(feature = "limb-meter")]
    limb_ops: u64,
    /// Packed-stream bits scanned during the scenario body.
    #[cfg(feature = "scan-meter")]
    scan_bits: u64,
    /// Accumulator digit touches counted during the scenario body.
    #[cfg(feature = "limb-meter")]
    touches: u64,
}

/// Build a [`QueryEnvelope`] from the five pinned columns.
///
/// The limb, scan, and touch columns are carried only when their features
/// compile the counters in; the leading underscores keep the parameters
/// warning-free in the other configurations.
const fn query_envelope(
    peak_heap: usize,
    segments: u64,
    _limb_ops: u64,
    _scan_bits: u64,
    _touches: u64,
) -> QueryEnvelope {
    QueryEnvelope {
        peak_heap,
        segments,
        #[cfg(feature = "limb-meter")]
        limb_ops: _limb_ops,
        #[cfg(feature = "scan-meter")]
        scan_bits: _scan_bits,
        #[cfg(feature = "limb-meter")]
        touches: _touches,
    }
}

// The query envelope table: pinned ceiling = measured ×1.25, rounded up,
// and only ever tightened: where a remeasure rises while staying inside
// an existing ceiling (the bigroot heap and touch cells, whose frozen
// component now lives on the accumulator), the older, tighter ceiling
// stands. The trailing comment on each line is the measurement of record
// (2026-07-24, aarch64-apple-darwin, dev profile, three identical runs)
// the ceiling derives from. Re-pin by rerunning under `--no-capture`
// with `--all-features` and reading the MEASURED lines.
#[rustfmt::skip]
mod query_env {
    use super::{query_envelope, QueryEnvelope};
    //                                                                        peak heap, segments,  limb ops, scan bits,   touches       measured: heap, seg, limb, scan, touches
    pub const SKYLINE_RANK_DENSE: QueryEnvelope           = query_envelope(    81_950,        0,   312_505, 1_093_772,   156_259); // 65_560, 0, 250_004, 875_017, 125_007
    pub const SKYLINE_RANK_BIGROOT: QueryEnvelope         = query_envelope(    67_145,        0,    26_767,   387_530,    17_199); // 60_088 -> 61_516 (2026-07-24, dashu-int backend), 0, 21_413, 310_024, 17_194
    pub const SKYLINE_RANK_HARMONIC: QueryEnvelope        = query_envelope(    71_705,        0,   165_122,   573_454,   248_324); // 57_364 -> 58_400 (2026-07-24, dashu-int backend), 0, 132_097, 458_763, 198_659
    pub const SKYLINE_RANK_CLIFF: QueryEnvelope           = query_envelope(     3_075,        0,     7_805,    48_647,     8_008); // 2_460 -> 2_540 (2026-07-24, dashu-int backend), 0, 6_244, 38_917, 6_406
    pub const SKYLINE_RANK_WIDE_TOOTH: QueryEnvelope      = query_envelope(     3_095,        0,    29_552, 2_996_319,    33_580); // 2_740 -> 2_820 (2026-07-24, dashu-int backend), 0, 23_641, 2_397_055, 26_864
    pub const SKYLINE_MIN_TICKS_DENSE: QueryEnvelope      = query_envelope(    30_720,        0,   312_503,   468_758,   156_255); // 24_576, 0, 250_002, 375_006, 125_004
    pub const SKYLINE_MIN_TICKS_CLIFF: QueryEnvelope      = query_envelope(       660,        0,        22,     2_565,        62); // 528 -> 560 (2026-07-24, dashu-int backend), 0, 17, 2_052, 49
    pub const SKYLINE_PROJECT_COMB_SCATTER: QueryEnvelope = query_envelope(   525_700,        0,   115_265, 2_656_008,    44_924); // 420_560 -> 420_592 (2026-07-24, dashu-int backend), 0, 92_212, 2_124_806, 35_939
    pub const FOLD_VERSION_SCATTER: QueryEnvelope        = query_envelope(    91_520,        0,   862_888,   204_833,         0); // 73_216, 0, 690_310 (sequential 14_281_732), 163_866, 0
    pub const FOLD_PARTY_SCATTER: QueryEnvelope          = query_envelope(       420,        0,         0,   365_540,         0); // 336, 0, 0, 292_432 (sequential 3_284_952), 0
}

/// Run one query scenario body under all five meters and assert its
/// envelope.
///
/// [`sweep_metered`]'s harness plus the accumulator touch column; prints
/// the measured numbers so re-pinning never requires editing the harness.
fn query_metered<R>(
    name: &str,
    input_bytes: usize,
    env: &QueryEnvelope,
    f: impl FnOnce() -> R,
) -> R {
    meter::reset_stack_segments();
    #[cfg(feature = "limb-meter")]
    meter::reset_limb_ops();
    #[cfg(feature = "limb-meter")]
    meter::accum::touch_meter::reset();
    #[cfg(feature = "scan-meter")]
    meter::reset_scan_bits();
    HEAP.reset_peak_usage();
    let baseline = HEAP.current_usage();
    let r = f();
    let peak_heap = HEAP.peak_usage().saturating_sub(baseline);
    let segments = meter::stack_segments();
    #[cfg(feature = "limb-meter")]
    let limb_ops = meter::limb_ops();
    #[cfg(feature = "limb-meter")]
    let touches = meter::accum::touch_meter::touches();
    #[cfg(feature = "scan-meter")]
    let scan_bits = meter::scan_bits();
    #[cfg(feature = "limb-meter")]
    let limb_col = format!(" limb_ops={limb_ops} touches={touches}");
    #[cfg(not(feature = "limb-meter"))]
    let limb_col = "";
    #[cfg(feature = "scan-meter")]
    let scan_col = format!(" scan_bits={scan_bits}");
    #[cfg(not(feature = "scan-meter"))]
    let scan_col = "";
    eprintln!(
        "MEASURED {name}: input_bytes={input_bytes} peak_heap={peak_heap} segments={segments}{limb_col}{scan_col}"
    );
    assert!(
        peak_heap <= env.peak_heap,
        "{name}: peak heap {peak_heap} B exceeds the pinned envelope {} B (input {input_bytes} B): {ISOLATION_NOTE}",
        env.peak_heap,
    );
    assert!(
        segments <= env.segments,
        "{name}: {segments} grown stack segments exceed the pinned envelope {}: {ISOLATION_NOTE}",
        env.segments,
    );
    #[cfg(feature = "limb-meter")]
    assert!(
        limb_ops <= env.limb_ops,
        "{name}: {limb_ops} limb operations exceed the pinned envelope {}: {ISOLATION_NOTE}",
        env.limb_ops,
    );
    #[cfg(feature = "scan-meter")]
    assert!(
        scan_bits <= env.scan_bits,
        "{name}: {scan_bits} scanned bits exceed the pinned envelope {}: {ISOLATION_NOTE}",
        env.scan_bits,
    );
    #[cfg(feature = "limb-meter")]
    assert!(
        touches <= env.touches,
        "{name}: {touches} accumulator digit touches exceed the pinned envelope {}: {ISOLATION_NOTE}",
        env.touches,
    );
    r
}

/// The rank kernel on the dense spine's skyline stays within its
/// envelope (the depth control: 125k levels of path bits, near-zero
/// arithmetic).
#[test]
fn skyline_rank_dense_envelope() {
    let p = meter::dense(DENSE_DEPTH);
    let v = version_of(&p);
    let enc = skyline_of(&p);
    let r = query_metered(
        "skyline_rank_dense",
        enc.bytes.len(),
        &query_env::SKYLINE_RANK_DENSE,
        || meter::skyline::query::rank(&enc),
    );
    assert_eq!(r, v.rank(), "the kernel must match the packed rank");
}

/// The rank kernel on the bigroot skyline stays within its envelope (the
/// wide-magnitude control: the first leaf's magnitude seeds the frozen
/// component and is read exactly once, in the closing shifted add
/// against the whole interval).
#[test]
fn skyline_rank_bigroot_envelope() {
    let p = meter::bigroot(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
    let v = version_of(&p);
    let enc = skyline_of(&p);
    let r = query_metered(
        "skyline_rank_bigroot",
        enc.bytes.len(),
        &query_env::SKYLINE_RANK_BIGROOT,
        || meter::skyline::query::rank(&enc),
    );
    assert_eq!(r, v.rank(), "the kernel must match the packed rank");
}

/// The rank kernel on the harmonic spine stays within its envelope — the
/// rank fold's separating family, linear here because each level's
/// one-leaf delta lands in the accumulator at its own weight instead of
/// re-shifting an accumulated numerator.
#[test]
fn skyline_rank_harmonic_envelope() {
    let p = meter::harmonic(RANK_HARMONIC_DEPTH);
    let v = version_of(&p);
    let enc = skyline_of(&p);
    let r = query_metered(
        "skyline_rank_harmonic",
        enc.bytes.len(),
        &query_env::SKYLINE_RANK_HARMONIC,
        || meter::skyline::query::rank(&enc),
    );
    assert_eq!(r, v.rank(), "the kernel must match the packed rank");
}

/// The rank kernel on the boundary comb's skyline stays within its
/// envelope: the heights are `2^k`-scale behind 3-bit deltas, the live
/// component absorbs the oscillation at O(1) digits per fold, and the
/// terminal borrow — as wide as its own code — rides the live component
/// into the last leaf's single wide add, no freeze anywhere.
#[test]
fn skyline_rank_cliff_envelope() {
    let p = meter::cliff_comb(CLIFF_SCALE, CLIFF_SCALE);
    let v = version_of(&p);
    let enc = skyline_of(&p);
    let r = query_metered(
        "skyline_rank_cliff",
        enc.bytes.len(),
        &query_env::SKYLINE_RANK_CLIFF,
        || meter::skyline::query::rank(&enc),
    );
    assert_eq!(r, v.rank(), "the kernel must match the packed rank");
}

/// The rank kernel on the wide-tooth comb's skyline stays within its
/// envelope — the no-freeze pin: bounded 192-bit oscillation keeps the
/// live component exactly as wide as each tooth's own code, so every
/// fold and every per-leaf add is paid by that code and the frozen
/// component never churns (the `skyline_flatness` freeze-band row pins
/// the same shape above the freeze allowance).
#[test]
fn skyline_rank_wide_tooth_envelope() {
    let p = meter::wide_tooth_comb(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
    let v = version_of(&p);
    let enc = skyline_of(&p);
    let r = query_metered(
        "skyline_rank_wide_tooth",
        enc.bytes.len(),
        &query_env::SKYLINE_RANK_WIDE_TOOTH,
        || meter::skyline::query::rank(&enc),
    );
    assert_eq!(r, v.rank(), "the kernel must match the packed rank");
}

/// The min_ticks kernel on the dense spine stays within its envelope:
/// one `u64` min-merge per node, heights on the accumulator, zero grown
/// segments at 125k levels.
#[test]
fn skyline_min_ticks_dense_envelope() {
    let p = meter::dense(DENSE_DEPTH);
    let v = version_of(&p);
    let enc = skyline_of(&p);
    let r = query_metered(
        "skyline_min_ticks_dense",
        enc.bytes.len(),
        &query_env::SKYLINE_MIN_TICKS_DENSE,
        || meter::skyline::query::min_ticks(&enc),
    );
    assert_eq!(r, v.min_ticks(), "the kernel must match the packed fold");
}

/// The min_ticks kernel on the boundary comb stays within its envelope:
/// the first `2^k`-scale height saturates the answer immediately, so the
/// early exit reads one leaf and no wide arithmetic ever reaches the
/// sums.
#[test]
fn skyline_min_ticks_cliff_envelope() {
    let p = meter::cliff_comb(CLIFF_SCALE, CLIFF_SCALE);
    let v = version_of(&p);
    let enc = skyline_of(&p);
    let r = query_metered(
        "skyline_min_ticks_cliff",
        enc.bytes.len(),
        &query_env::SKYLINE_MIN_TICKS_CLIFF,
        || meter::skyline::query::min_ticks(&enc),
    );
    assert_eq!(r, u64::MAX, "a comb height saturates the tick floor");
    assert_eq!(r, v.min_ticks(), "the kernel must match the packed fold");
}

/// The projection kernel on the comb × scattered-party cross stays
/// within its envelope — the output-dominated case: every kept tooth
/// boundary forces a fresh `2^k`-scale magnitude into the output, so the
/// mandatory output dominates the linear input and the pinned ceilings
/// price input + output bytes (the denomination the board's criterion
/// records for exactly this cross).
#[test]
fn skyline_project_comb_scatter_envelope() {
    let p = meter::cliff_comb(CLIFF_SCALE, CLIFF_SCALE);
    let v = version_of(&p);
    let party = before::Party::decode(&meter::scattered_id(CLIFF_SCALE / 2).bytes[..])
        .expect("scattered id is strict normal form");
    let enc = skyline_of(&p);
    let io_bytes_in = enc.bytes.len() + meter::scattered_id(CLIFF_SCALE / 2).bytes.len();
    let out = query_metered(
        "skyline_project_comb_scatter",
        io_bytes_in,
        &query_env::SKYLINE_PROJECT_COMB_SCATTER,
        || meter::skyline::query::project(&enc, &party),
    );
    eprintln!(
        "MEASURED skyline_project_comb_scatter: output_bytes={}",
        out.bytes.len()
    );
    let expected = meter::skyline::encode(&(&v / &party));
    assert_eq!(out, expected, "the kernel must match the packed quotient");
}

// ─── join-fold scenarios ────────────────────────────────────────────────────
//
// The public join folds on the scatter population: 1,024 balanced-forked
// parties, one tick each, ordered evens before odds so a left fold's
// accumulator would hold every other leaf and never coalesce — the shape
// on which a sequential fold reads quadratic (the board's two `scatter`
// cells were its red pins). The balanced binary-counter reduction gives
// every input O(log n) joins against similarly-sized partners, and these
// rows pin that as the enforced record: the version fold on the limb
// column (sequential reads 14,281,732 limb ops on this population — 20.7×
// the pinned fold), the party fold on the scan column (sequential reads
// 3,284,952 scanned bits — 11.2× the pinned fold; the id walk allocates
// nothing and does no `Base` arithmetic, so scanned bits are the only
// deterministic meter that sees it).

/// The board's scatter population at the enforced-suite scale.
const FOLD_SCATTER_CLOCKS: usize = 1_024;

/// Build the scatter fold population: balanced-forked parties, one tick
/// each, evens before odds (the board's `scatter` family recipe).
fn scatter_population() -> (Vec<Version>, Vec<before::Party>) {
    let mut parties = vec![before::Party::seed()];
    while parties.len() < FOLD_SCATTER_CLOCKS {
        let mut next = Vec::with_capacity(parties.len() * 2);
        for mut p in parties {
            let q = p.fork();
            next.push(p);
            next.push(q);
        }
        parties = next;
    }
    let versions: Vec<Version> = parties
        .iter()
        .map(|p| {
            let mut v = Version::new();
            v.tick(p);
            v
        })
        .collect();
    let scatter = |len: usize| (0..len).step_by(2).chain((1..len).step_by(2));
    let versions = scatter(versions.len())
        .map(|i| versions[i].clone())
        .collect();
    let mut scattered_parties = Vec::with_capacity(parties.len());
    let mut slots: Vec<Option<before::Party>> = parties.into_iter().map(Some).collect();
    for i in scatter(slots.len()) {
        scattered_parties.push(slots[i].take().expect("each index is visited once"));
    }
    (versions, scattered_parties)
}

/// `Version::join_all` over the scatter population stays within its
/// envelope: the balanced reduction keeps every join's operands
/// comparably sized, so the fold is near-linear in the population's
/// packed bytes where the left fold re-scanned its whole accumulator per
/// input.
#[test]
fn fold_version_scatter_envelope() {
    let (versions, _) = scatter_population();
    let input_bytes: usize = versions.iter().map(|v| v.encode().len()).sum();
    let reference = versions.iter().fold(Version::new(), |acc, v| acc | v);
    let out = query_metered(
        "fold_version_scatter",
        input_bytes,
        &query_env::FOLD_VERSION_SCATTER,
        || Version::join_all(versions),
    );
    assert_eq!(out, reference, "the balanced fold equals the left fold");
}

/// `Party::join_all` over the scatter population stays within its
/// envelope: the id-side fold's work is pure stream scanning, and the
/// balanced reduction keeps the scanned bits near-linear in the
/// population's packed bytes where the left fold re-walked its whole
/// accumulated region per input.
#[test]
fn fold_party_scatter_envelope() {
    let (_, mut parties) = scatter_population();
    let input_bytes: usize = parties.iter().map(|p| p.encode().len()).sum();
    let rest = parties.split_off(1);
    let mut acc = parties.remove(0);
    let acc = query_metered(
        "fold_party_scatter",
        input_bytes,
        &query_env::FOLD_PARTY_SCATTER,
        move || {
            acc.join_all(rest)
                .expect("balanced forks are pairwise disjoint");
            acc
        },
    );
    assert!(acc.is_seed(), "the scattered forks reunite the seed region");
}
