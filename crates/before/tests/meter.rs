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
//! Every row whose measured limb count is nonzero also carries a limb
//! *liveness floor* — the measured value ×0.75, rounded down, a column in
//! the same tables as the ceilings. A limb ceiling passes vacuously when
//! the counter stops counting (a meter hook deleted from one `Base`
//! operation reads a near-zero column with every ceiling green), and the
//! floor is what fails instead. Like the board's floors, these detect
//! *total* bypass, not partial rerouting: an implementation that routes
//! some width-scale work through metered operations and the rest around
//! them still reads green, so a floor is a bypass tripwire, never a
//! full-liveness proof.
//!
//! Wall time is deliberately never asserted *in this suite*: it is the one
//! number here that is not deterministic (the bench judge fits the time
//! *exponent* over criterion medians across two bench scales — see
//! `meter::board`'s module docs and `tools/benchjudge`; its wide-display
//! pair judges the conversion class no counter column can see — that is
//! the wall leg of record). The envelope constants are **measured** on the
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

/// The wide × deep tick crosses' shared scale: magnitude bits and
/// shortcut depth together, deep enough that a per-level re-touch of
/// the wide content would overshoot the envelopes by orders of
/// magnitude.
const TICK_CROSS_SCALE: usize = 4_000;

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

/// One scenario's pinned ceilings (the measured value ×1.25, rounded up)
/// and its limb liveness floor (measured ×0.75, rounded down).
struct Envelope {
    /// Peak heap delta over the scenario body, in bytes.
    peak_heap: usize,
    /// Stack segments grown during the scenario body.
    segments: u64,
    /// Big-integer limb operations counted during the scenario body.
    #[cfg(feature = "limb-meter")]
    limb_ops: u64,
    /// Liveness floor under the limb column: a reading below it means the
    /// meter is not watching this work (zero where the measured count is
    /// zero, under which the floor asserts nothing).
    #[cfg(feature = "limb-meter")]
    limb_floor: u64,
}

/// Build an [`Envelope`] from the three pinned columns and the limb floor.
///
/// The limb columns are carried only when the `limb-meter` feature
/// compiles the counter into the arithmetic; the leading underscores keep
/// the parameters warning-free in the other configuration.
const fn envelope(peak_heap: usize, segments: u64, _limb_ops: u64, _limb_floor: u64) -> Envelope {
    Envelope {
        peak_heap,
        segments,
        #[cfg(feature = "limb-meter")]
        limb_ops: _limb_ops,
        #[cfg(feature = "limb-meter")]
        limb_floor: _limb_floor,
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
// The limb floor column is the measured value ×0.75, rounded down (the
// module doc's liveness-floor convention).
#[rustfmt::skip]
mod envelope {
    use super::{envelope, Envelope};
    //                                              peak heap,  segments, limb ops, limb floor           measured: peak heap, segments, limb ops
    pub const DECODE_DENSE: Envelope    = envelope(   120_045,        0,       625_003, 375_001); // 11_072_549 -> 96_036 (2026-07-25, C2: operations route to the skyline kernels: wire decode is validate + wrap), 0, 250_002 -> 500_002 (2026-07-25, C2: operations route to the skyline kernels)
    pub const CMP_DENSE: Envelope       = envelope(    30_740,        0,       312_505, 187_503); //          8 -> 24_592, 192 -> 0, 2_000_010 -> 250_004 (2026-07-25, C2: operations route to the skyline kernels: the iterative sweep)
    pub const JOIN_DENSE: Envelope      = envelope(   188_892,        0,       937_505, 562_503); //  4_797_477 -> 151_113, 240 -> 0, 3_000_008 -> 750_004 (2026-07-25, C2: operations route to the skyline kernels: the emit kernel)
    // The tick rows live in `query_env`: the tick walk's cost currency
    // is accumulator digit touches (with scanned bits beside it), which
    // this four-column table never watched.
    pub const DECODE_BIGROOT: Envelope  = envelope(    60_090,        0,        50_790, 30_474); //  1_396_905 -> 48_072 (2026-07-25, C2: operations route to the skyline kernels: wire decode is validate + wrap), 0, 20_630 -> 40_632 (2026-07-25, C2: operations route to the skyline kernels)
    pub const CMP_BIGROOT: Envelope     = envelope(    40_350,        0,        25_790, 15_474); // 56_416_936 -> 32_280, 12 -> 0, 37_606_270 -> 20_632 (2026-07-25, C2: operations route to the skyline kernels: the iterative sweep; the V1 kill realized)
    pub const JOIN_BIGROOT: Envelope    = envelope(   102_250,        0,        76_583, 45_949); // 56_849_753 -> 81_800, 16 -> 0, 87_631_274 -> 61_266 (2026-07-25, C2: operations route to the skyline kernels: the emit kernel; the V1 kill realized)
    pub const DECODE_HUGELEAF: Envelope = envelope(   122_504,        0,         2_443, 1_465); //     48_851 -> 98_003 (2026-07-25, C2: operations route to the skyline kernels: the validating wire decode holds the running height), 0, 1_954
    pub const JOIN_HUGELEAF: Envelope   = envelope(   224_528,        0,         4_887, 2_931); //    115_707 -> 179_622 (2026-07-25, C2: operations route to the skyline kernels: the emit kernel holds both payload buffers), 0, 7_821 -> 3_909 (2026-07-25, C2: operations route to the skyline kernels)
    pub const ID_JOIN: Envelope         = envelope(   279_132,        0,             0, 0); //    125_001 -> 223_305, 202 -> 0, 0 (2026-07-24, iterative id walks: frame bits on the heap, no grown segments)
    pub const ID_COVERS: Envelope       = envelope(        10,        0,             0, 0); //          0 -> 8,  85 -> 0, 0 (2026-07-24, iterative id walks)
    pub const ID_DISJOINT: Envelope     = envelope(        10,        0,             0, 0); //          0 -> 8, 170 -> 0, 0 (2026-07-24, iterative id walks)
    pub const ID_WITHOUT: Envelope      = envelope(   647_774,        0,             0, 0); //    518_219, 138 -> 0, 0 (2026-07-23, iterative complement)
    pub const DECODE_CLIFF: Envelope    = envelope(     4_052,        0,        12_903, 7_741); //    607_489 -> 3_241 (2026-07-25, C2: operations route to the skyline kernels: wire decode is validate + wrap), 0, 40_960 -> 10_322 (2026-07-25, C2: operations route to the skyline kernels)
    pub const CMP_CLIFF: Envelope       = envelope(     1_710,        0,         7_765, 4_659); //        496 -> 1_368 (2026-07-25, C2: operations route to the skyline kernels: the sweep holds two accumulators), 0, 190_474 -> 6_212 (2026-07-25, C2: operations route to the skyline kernels: the cliff-immune sweep)
    pub const JOIN_CLIFF: Envelope      = envelope(     7_913,        0,        25_869, 15_521); //  1_411_489 -> 6_330, 0, 384_008 -> 20_695 (2026-07-25, C2: operations route to the skyline kernels: the emit kernel)
    // Skyline validator rows (2026-07-23, new scenarios): the V5
    // replacement's transient, achieved — the dense row's 49 KB peak over
    // 125k levels is ~3.1 bits per open ancestor (bit stack plus
    // reallocation growth) against DECODE_DENSE's 11 MB parse frames on
    // the same tree, ~56 B per level.
    pub const SKYLINE_VALIDATE_DENSE: Envelope      = envelope(    61_450,        0,       625_003, 375_001); //     49_160, 0,   500_002
    pub const SKYLINE_VALIDATE_CLIFF: Envelope      = envelope(     1_770,        0,        12_903, 7_741); //      1_416 -> 1_448 (2026-07-24, dashu-int backend), 0,    10_322
    pub const SKYLINE_VALIDATE_WIDE_TOOTH: Envelope = envelope(     1_520,        0,        42_325, 25_395); //      1_216, 0,    33_860
    pub const SKYLINE_VALIDATE_HUGELEAF: Envelope   = envelope(    80_980,        0,         2_443, 1_465); //     64_784 -> 66_752 (2026-07-24, dashu-int backend), 0,     1_954
    pub const SKYLINE_VALIDATE_ALT_SPINE: Envelope  = envelope(    61_450,        0,       625_003, 375_001); //     49_160, 0,   500_002
    // Skyline decoder rows: validation plus the wrap into storage — the
    // stored coding is the skyline stream itself, so decode materializes
    // nothing beyond the copy and stays priced by the wire input.
    pub const SKYLINE_DECODE_DENSE: Envelope        = envelope(   122_880,        0,       625_003, 375_001); // 18_138_292 -> 98_304, 0, 2_750_012 -> 500_002 (2026-07-25, C2: operations route to the skyline kernels: decode is validate + wrap)
    pub const SKYLINE_DECODE_CLIFF: Envelope        = envelope(     3_840,        0,        12_903, 7_741); //  1_787_704 -> 3_072, 0, 317_626 -> 10_322 (2026-07-25, C2: operations route to the skyline kernels: decode is validate + wrap)
    pub const SKYLINE_DECODE_WIDE_TOOTH: Envelope   = envelope(   245_760,        0,        42_325, 25_395); //  1_699_472 -> 196_608, 0, 370_843 -> 33_860 (2026-07-25, C2: operations route to the skyline kernels: decode is validate + wrap)
    pub const SKYLINE_DECODE_HUGELEAF: Envelope     = envelope(    83_440,        0,         2_443, 1_465); //    101_803 -> 66_752, 0, 9_773 -> 1_954 (2026-07-25, C2: operations route to the skyline kernels: decode is validate + wrap)
    pub const SKYLINE_DECODE_ALT_SPINE: Envelope    = envelope(   122_880,        0,       625_003, 375_001); // 15_778_996 -> 98_304, 0, 2_750_012 -> 500_002 (2026-07-25, C2: operations route to the skyline kernels: decode is validate + wrap)
}

// ─── meter liveness canaries ────────────────────────────────────────────────

/// Size of the canary allocation that proves the heap meter is live.
const CANARY_ALLOC_BYTES: usize = 1 << 20;

/// The heap meter registers a known allocation.
///
/// A canary buffer reads back a peak delta at least its own size, so a lost
/// `#[global_allocator]` line or a broken peak reader (either of which
/// would pass every upper-bound envelope vacuously at zero) fails loudly
/// here instead.
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

/// The dense-spine decode registers at least its packed input size.
///
/// The decoded version owns a copy of the packed bits, so the one big
/// scenario here has a floor as well as a ceiling, and a dead heap meter
/// cannot slide a big scenario under its envelope at zero.
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
    #[cfg(feature = "limb-meter")]
    assert!(
        limb_ops >= env.limb_floor,
        "{name}: limb counter reads {limb_ops}, below the {} liveness floor: \
         the meter is not watching this work",
        env.limb_floor,
    );
    r
}

/// Lift a generated shape into a [`Version`], outside any measurement.
fn version_of(p: &meter::Packed) -> Version {
    p.version()
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
    let wire = version_of(&p).encode();
    let v = metered("decode_dense", wire.len(), &envelope::DECODE_DENSE, || {
        Version::decode(&wire[..]).expect("a stored version's wire bytes decode")
    });
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
/// (the emit-path cost, linear in nodes).
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

/// Ticking the dense spine stays within its envelope (the fill-splice
/// round-trip cost, linear in nodes today).
#[test]
fn tick_dense_envelope() {
    let p = meter::dense(DENSE_DEPTH);
    let mut v = version_of(&p);
    let seed = Party::seed();
    query_metered("tick_dense", p.bytes.len(), &query_env::TICK_DENSE, || {
        v.tick(&seed)
    });
    drop(v);
}

/// Ticking the wide right-full chain (a bigroot magnitude over the
/// nested-full id) stays within its envelope: the anchor-web walk
/// touches the wide first payload O(1) times, never once per shortcut
/// level.
#[test]
fn tick_nested_wide_envelope() {
    let ev = meter::bigroot(TICK_CROSS_SCALE, TICK_CROSS_SCALE);
    let id = meter::nested_full_id(TICK_CROSS_SCALE);
    let mut v = version_of(&ev);
    let p = party_of(&id);
    let input = ev.bytes.len() + id.bytes.len();
    query_metered(
        "tick_nested_wide",
        input,
        &query_env::TICK_NESTED_WIDE,
        || v.tick(&p),
    );
    drop(v);
}

/// Ticking the wide memo chain (a wide-tail spine under the
/// nested-left-full id) stays within its envelope: the pre-scan's
/// frame ledger stores no link for the shared wide minimum, so
/// nothing is materialized per site.
#[test]
fn tick_mirror_wide_envelope() {
    let ev = meter::wide_tail(TICK_CROSS_SCALE, TICK_CROSS_SCALE);
    let id = meter::nested_left_full_id(TICK_CROSS_SCALE);
    let mut v = version_of(&ev);
    let p = party_of(&id);
    let input = ev.bytes.len() + id.bytes.len();
    query_metered(
        "tick_mirror_wide",
        input,
        &query_env::TICK_MIRROR_WIDE,
        || v.tick(&p),
    );
    drop(v);
}

// ─── bigroot scenarios ──────────────────────────────────────────────────────

/// Decoding bigroot stays within its envelope (one big-integer base plus the
/// parse stack).
#[test]
fn decode_bigroot_envelope() {
    let p = meter::bigroot(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
    let wire = version_of(&p).encode();
    let v = metered(
        "decode_bigroot",
        wire.len(),
        &envelope::DECODE_BIGROOT,
        || Version::decode(&wire[..]).expect("a stored version's wire bytes decode"),
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
    let wire = version_of(&p).encode();
    let v = metered(
        "decode_hugeleaf",
        wire.len(),
        &envelope::DECODE_HUGELEAF,
        || Version::decode(&wire[..]).expect("a stored version's wire bytes decode"),
    );
    drop(v);
}

/// Joining hugeleaf with a one-tick version stays within its envelope.
///
/// The emit path grows by push, so the peak tracks the result's node
/// count; the limb column tracks decode's, because reading the stored
/// spilled base runs the same linear wide-gamma decode.
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
    let wire = version_of(&p).encode();
    let v = metered("decode_cliff", wire.len(), &envelope::DECODE_CLIFF, || {
        Version::decode(&wire[..]).expect("a stored version's wire bytes decode")
    });
    drop(v);
}

/// Comparing the boundary comb against the empty version stays within its
/// envelope.
///
/// Each tooth's cliff excursion costs `Θ(k)` limb work bought by its own
/// `2k + 1`-bit stored magnitude, so the walk stays linear per input bit —
/// the property the comb exists to separate from codings that store 3-bit
/// deltas per crossing.
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

/// One touch-priced scenario's pinned ceilings, asserted when the
/// `limb-meter` feature is lit.
///
/// [`Envelope`]'s three columns plus accumulator digit touches — the
/// rank folds' and the tick walk's own cost currency: wide content
/// moves through `Accum`s that the heap and limb columns cannot see.
struct TouchEnvelope {
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
    /// Liveness floor under the limb column: measured ×0.75, per the
    /// module doc's floor convention.
    #[cfg(feature = "limb-meter")]
    limb_floor: u64,
    /// Liveness floor under the touch column: measured ×0.75.
    ///
    /// A touch reading below it means the scenario's accumulator work
    /// left the metered representation, and every touch ceiling above
    /// would hold vacuously.
    #[cfg(feature = "limb-meter")]
    touch_floor: u64,
}

/// Build a [`TouchEnvelope`] from the four pinned columns and the two
/// liveness floors.
///
/// The limb and touch columns are carried only when the `limb-meter`
/// feature compiles their counters in; the leading underscores keep the
/// parameters warning-free in the other configuration.
const fn touch_envelope(
    peak_heap: usize,
    segments: u64,
    _limb_ops: u64,
    _touches: u64,
    _limb_floor: u64,
    _touch_floor: u64,
) -> TouchEnvelope {
    TouchEnvelope {
        peak_heap,
        segments,
        #[cfg(feature = "limb-meter")]
        limb_ops: _limb_ops,
        #[cfg(feature = "limb-meter")]
        touches: _touches,
        #[cfg(feature = "limb-meter")]
        limb_floor: _limb_floor,
        #[cfg(feature = "limb-meter")]
        touch_floor: _touch_floor,
    }
}

// The touch-priced envelope table (the rank rows): pinned ceiling =
// measured ×1.25, rounded up,
// and only ever tightened: where a remeasure rises while staying inside
// an existing ceiling (the spilled-numerator heap cells, which carry the
// backend's `len/8 + 2` words of growth headroom per heap allocation),
// the older, tighter ceiling stands over the recorded movement. The
// trailing comment on each line is the measurement of record
// (2026-07-24, aarch64-apple-darwin, dev profile, three identical runs)
// the ceiling derives from; a re-pinned column records the movement as
// `old -> new`. Re-pin by rerunning under `--no-capture` with
// `--all-features` and reading the MEASURED lines.
// The limb floor column is the measured value ×0.75, rounded down (the
// module doc's liveness-floor convention).
// One rise is recorded against the tightening rule: every limb ceiling
// here rose 2026-07-24 when `Base::trailing_zeros` joined the metered
// seam (rank normalization strips factors of two through it, so these
// are the rows that gained counts) — a re-denomination of the column,
// the same work newly counted, not a weakening.
#[rustfmt::skip]
mod rank_env {
    use super::{touch_envelope, TouchEnvelope};
    //                                                             peak heap, segments,    limb ops, touches, limb floor, touch floor       measured: peak heap, segments, limb ops (movement), touches
    pub const RANK_DENSE: TouchEnvelope         = touch_envelope(      81_950,        0,     312_507, 156_259, 187_503, 93_755); //          0 -> 65_560, 240 -> 0, 3 -> 250_005, 0 -> 125_007 (2026-07-25, C2: operations route to the skyline kernels: the query fold reads delta payloads)
    pub const RANK_BIGROOT: TouchEnvelope       = touch_envelope(      76_895,        0,      27_744, 21_493, 16_646, 12_895); //     41_368 -> 61_516, 16 -> 0, 2_191 -> 22_195, 4_689 -> 17_194 (2026-07-25, C2: operations route to the skyline kernels: the query fold reads delta payloads)
    pub const RANK_HARMONIC: TouchEnvelope      = touch_envelope(      73_000,        0,     166_402, 248_324, 99_840, 148_994); //     33_840 -> 58_400, 124 -> 0, 2_049 -> 133_121, 67_522 -> 198_659 (2026-07-25, C2: operations route to the skyline kernels: the query fold reads delta payloads)
    pub const RANK_PAIR_MISMATCH: TouchEnvelope = touch_envelope(     234_400,        0,      87_910,      0, 52_746, 0); //    187_520 -> 211_016 (2026-07-24, dashu-int backend),   0, 54_710 -> 39_078 (class-first cmp; the rest is checked_sub's and add's mandatory output) -> 54_704 (2026-07-24, metered trailing_zeros) -> 70_328 (2026-07-26, widening shifts record output width: a re-denomination, the exponent-alignment work newly counted), 0
    pub const RANK_SUM_MIXED: TouchEnvelope     = touch_envelope(      78_140,        0,       9_769, 22_268, 5_861, 13_360); //     62_512,   0, 156_312_196 -> 3_908 (raw accumulator, one normalization) -> 7_815 (2026-07-24, metered trailing_zeros), 17_814
}

/// Run one touch-priced scenario body under all four meters and assert
/// its envelope, both liveness floors included.
///
/// [`metered`]'s harness plus the accumulator touch column; prints the
/// measured numbers so re-pinning never requires editing the harness.
fn touch_metered<R>(
    name: &str,
    input_bytes: usize,
    env: &TouchEnvelope,
    f: impl FnOnce() -> R,
) -> R {
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
    #[cfg(feature = "limb-meter")]
    assert!(
        limb_ops >= env.limb_floor,
        "{name}: limb counter reads {limb_ops}, below the {} liveness floor: \
         the meter is not watching this work",
        env.limb_floor,
    );
    #[cfg(feature = "limb-meter")]
    assert!(
        touches >= env.touch_floor,
        "{name}: touch counter reads {touches}, below the {} liveness floor: \
         the accumulator work left the metered representation",
        env.touch_floor,
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
    let r = touch_metered("rank_dense", p.bytes.len(), &rank_env::RANK_DENSE, || {
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
    let r = touch_metered(
        "rank_bigroot",
        p.bytes.len(),
        &rank_env::RANK_BIGROOT,
        || v.rank(),
    );
    consumed(r);
}

/// The rank fold on the harmonic spine stays within its envelope — the
/// fold's separating family, pinned linear.
///
/// The accumulated numerator is as wide as the depth already walked at
/// every level, and the digit-routed merge folds each level's one-leaf
/// sibling into it at the exponent gap instead of re-shifting it.
#[test]
fn rank_harmonic_envelope() {
    let p = meter::harmonic(RANK_HARMONIC_DEPTH);
    let v = version_of(&p);
    let r = touch_metered(
        "rank_harmonic",
        p.bytes.len(),
        &rank_env::RANK_HARMONIC,
        || v.rank(),
    );
    consumed(r);
}

/// `Rank::cmp` + `checked_sub` + `+` on the mismatched-exponent pair stay
/// within their envelope.
///
/// The class-first comparison decides the order and the pre-check in O(1),
/// so the pinned cost is the `Some`-arm subtraction and the addition —
/// transients that are the outputs' own value content, not amplification.
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
    let r = touch_metered(
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
/// within its envelope.
///
/// The raw accumulator anchors at the largest exponent seen and
/// digit-routes each summand in at its exponent gap, normalizing once at
/// the end, so the high-exponent operand costs its own width once instead
/// of once per later element.
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
    let r = touch_metered(
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
fn skyline_of(p: &meter::Packed) -> meter::skyline::Bits {
    meter::skyline::encode(&version_of(p))
}

/// The skyline validator on the dense spine stays within its envelope.
///
/// The transient is ~2 bits per open ancestor (measured ~3.1 bits per
/// level including reallocation growth, against the old parse stack's ~56
/// bytes per level on the same tree), with zero grown segments.
#[test]
fn skyline_validate_dense_envelope() {
    let enc = skyline_of(&meter::dense(DENSE_DEPTH));
    let r = metered(
        "skyline_validate_dense",
        enc.as_raw_slice().len(),
        &envelope::SKYLINE_VALIDATE_DENSE,
        || meter::skyline::validate(&enc),
    );
    assert!(r.is_ok(), "the transcoded dense spine is canonical");
}

/// The skyline validator on the boundary comb stays within its envelope.
///
/// Every 3-bit `±1` delta sits on the `2^k` carry boundary, and the
/// accumulator's redundant representation keeps the nonnegativity check
/// amortized O(1) per delta (the flatness pin below is the cross-scale
/// witness; a plain big-integer accumulator is quadratic here).
#[test]
fn skyline_validate_cliff_envelope() {
    let enc = skyline_of(&meter::cliff_comb(CLIFF_SCALE, CLIFF_SCALE));
    let r = metered(
        "skyline_validate_cliff",
        enc.as_raw_slice().len(),
        &envelope::SKYLINE_VALIDATE_CLIFF,
        || meter::skyline::validate(&enc),
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
        enc.as_raw_slice().len(),
        &envelope::SKYLINE_VALIDATE_WIDE_TOOTH,
        || meter::skyline::validate(&enc),
    );
    assert!(r.is_ok(), "the transcoded wide-tooth comb is canonical");
}

/// The skyline validator on the hugeleaf analog — a single huge first
/// leaf, the whole stream one absolute gamma code — stays within its
/// envelope.
///
/// The cost is one wide decode plus one wide accumulator load, both
/// linear in the code's own width.
#[test]
fn skyline_validate_hugeleaf_envelope() {
    let enc = skyline_of(&meter::hugeleaf(HUGELEAF_MAGNITUDE_BITS));
    let r = metered(
        "skyline_validate_hugeleaf",
        enc.as_raw_slice().len(),
        &envelope::SKYLINE_VALIDATE_HUGELEAF,
        || meter::skyline::validate(&enc),
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
        enc.as_raw_slice().len(),
        &envelope::SKYLINE_VALIDATE_ALT_SPINE,
        || meter::skyline::validate(&enc),
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
        enc.as_raw_slice().len(),
        &envelope::SKYLINE_DECODE_DENSE,
        || meter::skyline::decode(&enc).expect("canonical"),
    );
    assert_eq!(v, version_of(&p), "the transcode round-trips");
}

/// The skyline decoder on the boundary comb stays within its envelope.
///
/// The packed output stores a fresh `gamma(2^k − 1)` per tooth, so the
/// materialized heights and floors are output-sized — quadratically above
/// the skyline input, linearly within the packed form being rebuilt.
#[test]
fn skyline_decode_cliff_envelope() {
    let p = meter::cliff_comb(CLIFF_SCALE, CLIFF_SCALE);
    let enc = skyline_of(&p);
    let v = metered(
        "skyline_decode_cliff",
        enc.as_raw_slice().len(),
        &envelope::SKYLINE_DECODE_CLIFF,
        || meter::skyline::decode(&enc).expect("canonical"),
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
        enc.as_raw_slice().len(),
        &envelope::SKYLINE_DECODE_WIDE_TOOTH,
        || meter::skyline::decode(&enc).expect("canonical"),
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
        enc.as_raw_slice().len(),
        &envelope::SKYLINE_DECODE_HUGELEAF,
        || meter::skyline::decode(&enc).expect("canonical"),
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
        enc.as_raw_slice().len(),
        &envelope::SKYLINE_DECODE_ALT_SPINE,
        || meter::skyline::decode(&enc).expect("canonical"),
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
    /// Liveness floor under the limb column: measured ×0.75, per the
    /// module doc's floor convention.
    #[cfg(feature = "limb-meter")]
    limb_floor: u64,
}

/// Build a [`SweepEnvelope`] from the four pinned columns and the limb
/// floor.
///
/// The limb and scan columns are carried only when their features
/// compile the counters in; the leading underscores keep the parameters
/// warning-free in the other configurations.
const fn sweep_envelope(
    peak_heap: usize,
    segments: u64,
    _limb_ops: u64,
    _scan_bits: u64,
    _limb_floor: u64,
) -> SweepEnvelope {
    SweepEnvelope {
        peak_heap,
        segments,
        #[cfg(feature = "limb-meter")]
        limb_ops: _limb_ops,
        #[cfg(feature = "scan-meter")]
        scan_bits: _scan_bits,
        #[cfg(feature = "limb-meter")]
        limb_floor: _limb_floor,
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
// The limb floor column is the measured value ×0.75, rounded down (the
// module doc's liveness-floor convention).
#[rustfmt::skip]
mod sweep_env {
    use super::{sweep_envelope, SweepEnvelope};
    //                                                               peak heap, segments, limb ops,  scan bits, limb floor            measured: peak heap, segments, limb ops, scan bits
    pub const SKYLINE_CMP_DENSE: SweepEnvelope      = sweep_envelope(   30_730,        0,   312_503,   468_760, 187_501); //   24_584, 0, 250_002, 375_008
    pub const SKYLINE_CMP_DENSE_SELF: SweepEnvelope = sweep_envelope(   51_210,        0,   625_005,   937_515, 375_003); //   40_968, 0, 500_004, 750_012
    pub const SKYLINE_CMP_BIGROOT: SweepEnvelope    = sweep_envelope(   39_540,        0,    25_788,   137_514, 15_472); //   31_632 -> 32_272 (2026-07-24, dashu-int backend), 0,  20_630, 110_011
    pub const SKYLINE_CMP_CLIFF: SweepEnvelope      = sweep_envelope(    1_450,        0,     7_763,    17_925, 4_657); //    1_160 -> 1_296 (2026-07-23, emission-sweep shared step holds each consumed delta) -> 1_360 (2026-07-24, dashu-int backend), 0,   6_210,  14_340
    // SKYLINE_CMP_WIDE_TOOTH's 1_032-under-1_050 margin is a deliberate
    // change-detector on the backend's allocation policy: the committed
    // Cargo.lock (dashu-int 0.5.0 exact) is what makes the measurement
    // deterministic, and a cargo update to any other 0.5.x is a deliberate
    // re-measure event, not noise.
    pub const SKYLINE_CMP_WIDE_TOOTH: SweepEnvelope = sweep_envelope(    1_050,        0,    29_509, 1_000_483, 17_705); //      840 -> 968 (2026-07-23, emission-sweep shared step holds each consumed delta) -> 1_032 (2026-07-24, dashu-int backend), 0,  23_607, 800_386
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
    #[cfg(feature = "limb-meter")]
    assert!(
        limb_ops >= env.limb_floor,
        "{name}: limb counter reads {limb_ops}, below the {} liveness floor: \
         the meter is not watching this work",
        env.limb_floor,
    );
    r
}

/// The empty version's two-bit skyline stream: the shallow operand of
/// the family cmp scenarios.
fn skyline_empty() -> meter::skyline::Bits {
    meter::skyline::encode(&Version::new())
}

/// The combined operand bytes of a sweep scenario.
fn sweep_input_bytes(a: &meter::skyline::Bits, b: &meter::skyline::Bits) -> usize {
    a.as_raw_slice().len() + b.as_raw_slice().len()
}

/// The sweep on the dense spine against the empty version stays within
/// its envelope.
///
/// The deep side's 125k levels cost path *bits* (no grown segments, heap
/// in the path stack), consumed iteratively against one depth-0 plateau.
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

/// The sweep on two identical dense streams stays within its envelope.
///
/// Every boundary is an aligned tie, both cursors advance in lockstep to
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
/// its envelope.
///
/// Every 3-bit `±1` delta drives the running difference across the `2^k`
/// carry boundary, and the accumulator keeps each crossing amortized O(1)
/// (the flatness pin below is the cross-scale witness).
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
/// within its envelope.
///
/// Each `±2^w` delta is a genuinely wide operand paid by its own zigzag
/// code, so limb work stays linear per input bit at every tooth width.
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
// The limb floor column is the measured value ×0.75, rounded down (the
// module doc's liveness-floor convention).
#[rustfmt::skip]
mod emit_env {
    use super::{sweep_envelope, SweepEnvelope};
    //                                                                peak heap, segments, limb ops,  scan bits, limb floor            measured: peak heap, segments, limb ops, scan bits
    pub const SKYLINE_JOIN_DENSE: SweepEnvelope      = sweep_envelope(  130_297,        0,   937_505,   625_018, 562_503); //  104_237, 0, 750_004,   500_014
    pub const SKYLINE_JOIN_ABSORB: SweepEnvelope     = sweep_envelope(  270_798,        0,   942_389, 1_250_013, 565_433); //  216_638 -> 218_606 (2026-07-24, dashu-int backend), 0, 753_911, 1_000_010
    pub const SKYLINE_JOIN_BIGROOT: SweepEnvelope    = sweep_envelope(   85_060,        0,    76_583,   275_028, 45_949); //   71_768 -> 68_048 (2026-07-24, dashu-int backend), 0,  61_266,   220_022
    pub const SKYLINE_JOIN_CLIFF: SweepEnvelope      = sweep_envelope(    5_512,        0,    25_869,    35_848, 15_521); //    4_409 -> 4_537 (2026-07-24, dashu-int backend), 0,  20_695,    28_678
    pub const SKYLINE_JOIN_WIDE_TOOTH: SweepEnvelope = sweep_envelope(  128_312,        0,    74_477, 2_000_963, 44_685); //  102_649 -> 102_777 (2026-07-24, dashu-int backend), 0,  59_581, 1_600_770
    pub const SKYLINE_MEET_CLIFF: SweepEnvelope      = sweep_envelope(    5_002,        0,    18_020,    23_055, 10_812); //    4_001 -> 4_065 (2026-07-24, dashu-int backend), 0,  14_416,    18_444
    pub const SKYLINE_MEET_WIDE_TOOTH: SweepEnvelope = sweep_envelope(  127_732,        0,    39_767, 1_005_613, 23_859); //  102_185 -> 102_249 (2026-07-24, dashu-int backend), 0,  31_813,   804_490
}

/// The one-tick version's skyline stream: the shallow operand of the
/// family join/meet scenarios, mirroring the packed-form join rows.
fn skyline_one_tick() -> meter::skyline::Bits {
    let one = Version::try_from(1u64).expect("a one-tick version is valid");
    meter::skyline::encode(&one)
}

/// One family shape and the packed-form oracle's answer against the
/// one-tick version, both as skyline streams built outside measurement,
/// so every scenario asserts byte-identity after its sweep.
fn skyline_oracle(p: &meter::Packed, join: bool) -> (meter::skyline::Bits, meter::skyline::Bits) {
    let v = version_of(p);
    let one = Version::try_from(1u64).expect("a one-tick version is valid");
    let out = if join { &v | &one } else { &v & &one };
    (meter::skyline::encode(&v), meter::skyline::encode(&out))
}

/// Joining the dense spine's skyline with a one-tick stream stays within
/// its envelope.
///
/// The 125k-level walk emits and collapses on path-bit stacks and one
/// accumulator, with zero grown segments and the peak in the emitted
/// stream itself.
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
/// stays within its envelope.
///
/// The whole output collapses to one leaf through 125k absorb steps
/// around a held 125k-bit code, so this row is linear only because absorb
/// never moves the held code.
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
/// within its envelope.
///
/// The output collapses to the flat one-tick leaf through the absorb
/// cascade while every comb delta still crosses the carry boundary in the
/// accumulator.
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
/// within its envelope.
///
/// Wide deltas are folded but never re-emitted (the flat side wins
/// everywhere), so the collapse discipline runs at spilled operand
/// widths.
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

// ─── grow-branch tick scenarios ─────────────────────────────────────────────
//
// The deep expansion shapes: pairs whose fill is the identity, so the
// fused tick records the inflation route on its one walk and replays it
// through the splice emit. These rows measure the whole public tick —
// walk, route fold, and splice — on the shapes whose id side dominates
// (their envelopes live in `query_env` with the other tick rows).

/// The version that is `1` on the leftmost `2^-depth` interval and `0`
/// everywhere else: `depth` nested nodes, all bases zero, the single
/// 1-leaf at the bottom left.
///
/// This is what ticking a version that is zero over the owned region
/// registers for a depth-`depth` unary id spine. Built as a text
/// literal (the parser is iterative), so the expected tree shares no
/// walk with the tick under measurement.
fn left_spike(depth: usize) -> Version {
    let mut text = "(0, ".repeat(depth - 1);
    text.push_str("(0, 1, 0)");
    text.push_str(&", 0)".repeat(depth - 1));
    text.parse().expect("the spike literal is normal form")
}

/// Ticking the empty version under a 250k-deep unary id spine stays
/// within its envelope.
///
/// The walk is one event leaf whose route fold is the iterative id
/// scan (bit-stack frames, nothing recurses), and the emit codes the
/// whole expansion chain as fresh one-bit deltas. The value witness is
/// closed-form: the expansion chain to the owned tip is exactly the
/// left spike literal.
#[test]
fn tick_expand_spine_envelope() {
    let mut v = Version::new();
    let party = party_of(&meter::id_spine(ID_DEPTH, false));
    let input = meter::skyline::encode(&v).len() / 8 + party.encoded_bits().div_ceil(8);
    query_metered(
        "tick_expand_spine",
        input,
        &query_env::TICK_EXPAND_SPINE,
        || v.tick(&party),
    );
    assert_eq!(
        v,
        left_spike(ID_DEPTH),
        "the ticked version must be the derived closed form"
    );
}

/// Ticking the alternating spine under a deep unary id spine stays
/// within its envelope.
///
/// The regimes mix — the two-cursor fused walk down the shared spine,
/// an id-only expansion fold where the id outruns the event — and the
/// splice replays the recorded route. The value witness is closed-form:
/// the unary id turns left into the spine's depth-2 zero leaf, so the
/// forced route raises exactly the owned region from 0 to 1 — the
/// pointwise max with the left spike, realized through the
/// independently-tested join, byte-exact by canonical uniqueness.
#[test]
fn tick_expand_cross_envelope() {
    let ev = meter::alt_spine(DENSE_DEPTH);
    let mut v = version_of(&ev);
    let party = party_of(&meter::id_spine(ID_DEPTH, false));
    let expected = &v | &left_spike(ID_DEPTH);
    let input = ev.bytes.len() + party.encoded_bits().div_ceil(8);
    query_metered(
        "tick_expand_cross",
        input,
        &query_env::TICK_EXPAND_CROSS,
        || v.tick(&party),
    );
    assert_eq!(
        v, expected,
        "the ticked version must be the derived closed form"
    );
}

// ─── skyline text kernel scenarios ──────────────────────────────────────────
//
// The skyline-first text kernels (`meter::skyline::text`): rendering
// derives every printed base in delta-sized relative coordinates and
// sizes its output exactly before writing; parsing turns path-sum
// movement into skyline payloads through the per-leaf delta accumulator
// and the collapsing output builder. The columns carry the kernels'
// contract — zero grown segments (nothing recurses at any depth), heap
// in the frame vectors, the digit arena, and the output itself, limb
// work linear per I/O byte (radix conversion runs inside the backend;
// the recorded ops are the delta algebra), and scan linear in the
// skyline stream. The bigroot rows are the width separator: the 40k-bit
// heights never materialize, so no summary or accumulator state carries
// a copy of the wide magnitude per level.

// The text envelope table: pinned ceiling = measured ×1.25, rounded up,
// and only ever tightened. The trailing comment on each line is the
// measurement of record (2026-07-24, aarch64-apple-darwin, dev profile,
// three identical runs) the ceiling derives from. Re-pin by rerunning
// under `--no-capture` with `--all-features` and reading the MEASURED
// lines.
// The limb floor column is the measured value ×0.75, rounded down (the
// module doc's liveness-floor convention).
#[rustfmt::skip]
mod text_env {
    use super::{sweep_envelope, SweepEnvelope};
    //                                                                 peak heap, segments, limb ops,  scan bits, limb floor            measured: peak heap, segments, limb ops, scan bits
    pub const SKYLINE_RENDER_DENSE: SweepEnvelope    = sweep_envelope(29_511_680,        0, 1_562_513,   468_758, 937_507); // 23_609_344, 0, 1_250_010, 375_006
    pub const SKYLINE_RENDER_BIGROOT: SweepEnvelope  = sweep_envelope( 3_688_960,        0,   127_368,   137_512, 76_420); //  2_951_168, 0,   101_894, 110_009
    pub const SKYLINE_RENDER_HUGELEAF: SweepEnvelope = sweep_envelope(   171_370,        0,     7_330,   312_503, 4_398); //    137_096, 0,     5_864, 250_002
    pub const SKYLINE_RENDER_CLIFF: SweepEnvelope    = sweep_envelope( 1_850_502,        0,   243_385,    17_923, 146_031); //  1_480_401, 0,   194_708, 14_338
    pub const SKYLINE_PARSE_DENSE: SweepEnvelope     = sweep_envelope(13_918_822,        0, 1_875_017,   937_515, 1_125_009); // 11_135_057, 0, 1_500_013, 750_012
    pub const SKYLINE_PARSE_BIGROOT: SweepEnvelope   = sweep_envelope( 1_762_244,        0,   152_374,   275_023, 91_424); //  1_409_795, 0,   121_899, 220_018
    pub const SKYLINE_PARSE_HUGELEAF: SweepEnvelope  = sweep_envelope(   196_280,        0,     7_329,   625_005, 4_397); //    157_024, 0,     5_863, 500_004
    pub const SKYLINE_PARSE_CLIFF: SweepEnvelope     = sweep_envelope(   482_592,        0,    84_705,    35_845, 50_823); //    386_073, 0,    67_764, 28_676
}

/// Rendering the dense spine's skyline stays within its envelope.
///
/// 125k levels of frames and ~250k single-digit printed bases finalize
/// through word-sized summaries, the output is sized exactly before one
/// byte is written, and nothing recurses.
#[test]
fn skyline_render_dense_envelope() {
    let v = version_of(&meter::dense(DENSE_DEPTH));
    let a = meter::skyline::encode(&v);
    let expected = v.to_string();
    let out = sweep_metered(
        "skyline_render_dense",
        a.as_raw_slice().len(),
        &text_env::SKYLINE_RENDER_DENSE,
        || meter::skyline::text::render(&a),
    );
    assert_eq!(out, expected, "the kernel must render Display's bytes");
}

/// Rendering bigroot's skyline stays within its envelope — the width
/// separator.
///
/// Every leaf height carries the 40k-bit root magnitude, but the finalize
/// pass's summaries are leaf-delta-sized, so the deep spine's transient
/// holds no per-level copy of the wide value and the one wide printed
/// base is paid by its own rendered digits.
#[test]
fn skyline_render_bigroot_envelope() {
    let v = version_of(&meter::bigroot(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH));
    let a = meter::skyline::encode(&v);
    let expected = v.to_string();
    let out = sweep_metered(
        "skyline_render_bigroot",
        a.as_raw_slice().len(),
        &text_env::SKYLINE_RENDER_BIGROOT,
        || meter::skyline::text::render(&a),
    );
    assert_eq!(out, expected, "the kernel must render Display's bytes");
}

/// Rendering hugeleaf's skyline stays within its envelope: one node, one
/// 125k-bit magnitude, so the whole cost is the delegated decimal
/// rendering plus the exact-sized output — no tree state at all.
#[test]
fn skyline_render_hugeleaf_envelope() {
    let v = version_of(&meter::hugeleaf(HUGELEAF_MAGNITUDE_BITS));
    let a = meter::skyline::encode(&v);
    let expected = v.to_string();
    let out = sweep_metered(
        "skyline_render_hugeleaf",
        a.as_raw_slice().len(),
        &text_env::SKYLINE_RENDER_HUGELEAF,
        || meter::skyline::text::render(&a),
    );
    assert_eq!(out, expected, "the kernel must render Display's bytes");
}

/// Rendering the boundary comb's skyline stays within its envelope:
/// every tooth's wide printed base re-derives from 3-bit `±1` deltas
/// against the running relative floor, each merge paid by the tooth's
/// own rendered digits.
#[test]
fn skyline_render_cliff_envelope() {
    let v = version_of(&meter::cliff_comb(CLIFF_SCALE, CLIFF_SCALE));
    let a = meter::skyline::encode(&v);
    let expected = v.to_string();
    let out = sweep_metered(
        "skyline_render_cliff",
        a.as_raw_slice().len(),
        &text_env::SKYLINE_RENDER_CLIFF,
        || meter::skyline::text::render(&a),
    );
    assert_eq!(out, expected, "the kernel must render Display's bytes");
}

/// Parsing the dense spine's text stays within its envelope: one frame
/// per open node, single-digit bases through the delegated reader, and
/// the per-leaf delta accumulator staying word-sized throughout.
#[test]
fn skyline_parse_dense_envelope() {
    let v = version_of(&meter::dense(DENSE_DEPTH));
    let s = v.to_string();
    let expected = meter::skyline::encode(&v);
    let out = sweep_metered(
        "skyline_parse_dense",
        s.len(),
        &text_env::SKYLINE_PARSE_DENSE,
        || meter::skyline::text::parse(&s).expect("canonical text parses"),
    );
    assert_eq!(
        out, expected,
        "the kernel must build the transcoder's stream"
    );
}

/// Parsing bigroot's text stays within its envelope.
///
/// The 12k-digit root base converts once through the backend's
/// divide-and-conquer parser, joins and leaves the delta accumulator
/// exactly twice, and every spine base is word-sized — no per-level copy
/// of the wide value.
#[test]
fn skyline_parse_bigroot_envelope() {
    let v = version_of(&meter::bigroot(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH));
    let s = v.to_string();
    let expected = meter::skyline::encode(&v);
    let out = sweep_metered(
        "skyline_parse_bigroot",
        s.len(),
        &text_env::SKYLINE_PARSE_BIGROOT,
        || meter::skyline::text::parse(&s).expect("canonical text parses"),
    );
    assert_eq!(
        out, expected,
        "the kernel must build the transcoder's stream"
    );
}

/// Parsing hugeleaf's text stays within its envelope: one ~37k-digit
/// run through the delegated conversion, one absolute payload out — the
/// shape where any superlinear parse-side arithmetic shows undiluted.
#[test]
fn skyline_parse_hugeleaf_envelope() {
    let v = version_of(&meter::hugeleaf(HUGELEAF_MAGNITUDE_BITS));
    let s = v.to_string();
    let expected = meter::skyline::encode(&v);
    let out = sweep_metered(
        "skyline_parse_hugeleaf",
        s.len(),
        &text_env::SKYLINE_PARSE_HUGELEAF,
        || meter::skyline::text::parse(&s).expect("canonical text parses"),
    );
    assert_eq!(
        out, expected,
        "the kernel must build the transcoder's stream"
    );
}

/// Parsing the boundary comb's text stays within its envelope.
///
/// Every tooth's wide base enters and leaves the cliff-immune accumulator
/// paid by its own digit run, so the `2^k` carry boundary costs amortized
/// O(1) digit touches per crossing.
#[test]
fn skyline_parse_cliff_envelope() {
    let v = version_of(&meter::cliff_comb(CLIFF_SCALE, CLIFF_SCALE));
    let s = v.to_string();
    let expected = meter::skyline::encode(&v);
    let out = sweep_metered(
        "skyline_parse_cliff",
        s.len(),
        &text_env::SKYLINE_PARSE_CLIFF,
        || meter::skyline::text::parse(&s).expect("canonical text parses"),
    );
    assert_eq!(
        out, expected,
        "the kernel must build the transcoder's stream"
    );
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
        let v = packed.version();
        let enc = meter::skyline::encode(&v);
        touch_meter::reset();
        meter::reset_limb_ops();
        meter::skyline::validate(&enc).expect("the comb stream is canonical");
        let run = Run {
            // 2n + 1 leaves: 2n delta codes follow the first leaf.
            deltas: 2 * scale as u64,
            bytes: enc.as_raw_slice().len() as u64,
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
        let v = packed.version();
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
            bytes: (a.as_raw_slice().len() + b.as_raw_slice().len()) as u64,
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
    /// against the empty version.
    ///
    /// The running difference crosses the `2^k` carry boundary at every
    /// delta and each crossing stays amortized O(1) — the comparison-side
    /// cliff-immunity witness.
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

    /// Join the `k = n = scale` boundary comb's skyline stream with a
    /// one-tick stream and record both counters over the emission body
    /// alone.
    ///
    /// Enforces the same touch-meter liveness floor as [`comb_cmp_run`]:
    /// the emitter's running difference lands every comb delta, so an
    /// emission whose difference state is not the metered accumulator
    /// fails loudly here instead of passing the flatness ratio vacuously
    /// at zero touches.
    fn comb_join_run(scale: usize) -> Run {
        let packed = meter::cliff_comb(scale, scale);
        let v = packed.version();
        let a = meter::skyline::encode(&v);
        let mut one = before::Version::new();
        one.tick(&before::Party::seed());
        let b = meter::skyline::encode(&one);
        let expected = meter::skyline::encode(&(&v | &one));
        touch_meter::reset();
        meter::reset_limb_ops();
        let out = meter::skyline::emit::join(&a, &b);
        let run = Run {
            // 2n + 1 leaves: 2n delta codes follow the first leaf.
            deltas: 2 * scale as u64,
            bytes: (a.as_raw_slice().len() + b.as_raw_slice().len()) as u64,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert_eq!(out, expected, "the emitted join must match the packed join");
        assert!(
            run.touches >= run.deltas,
            "skyline_comb_join scale {scale}: {} digit touches under the {}-delta floor: \
             the emitter's difference state is not running on the metered accumulator",
            run.touches,
            run.deltas,
        );
        run
    }

    /// The join emitter's per-delta accumulator touches and per-byte limb
    /// work stay flat across a `k = n` doubling of the boundary comb
    /// joined with a one-tick stream.
    ///
    /// The running difference crosses the `2^k` carry boundary at every
    /// delta and each crossing stays amortized O(1) — the emission-side
    /// cliff-immunity witness, the merge counterpart of the comparison
    /// pin above (join, meet, `recv`, `sync`, and the fold operators all
    /// ride this emitter). Each run also carries the one-touch-per-delta
    /// liveness floor (in [`comb_join_run`]), so flatness is asserted
    /// over a meter proven live.
    #[test]
    fn skyline_join_cliff_cost_is_flat_per_unit() {
        let small = comb_join_run(512);
        let large = comb_join_run(1_024);
        assert_flat(
            "join_touches",
            "delta",
            (small.touches, small.deltas),
            (large.touches, large.deltas),
        );
        assert_flat(
            "join_limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
    }

    /// Parse the `k = n = scale` boundary comb's rendered text and record
    /// the touch counter over the parse body alone (the text is rendered
    /// outside the metered window).
    ///
    /// Enforces the same touch-meter liveness floor as [`comb_run`]: the
    /// parse extracts each leaf's delta from the running path-sum
    /// accumulator, so a parse whose accumulator left the metered
    /// representation fails loudly here instead of passing the flatness
    /// ratio vacuously at zero touches.
    fn comb_parse_run(scale: usize) -> Run {
        let packed = meter::cliff_comb(scale, scale);
        let v = packed.version();
        let s = v.to_string();
        let expected = meter::skyline::encode(&v);
        touch_meter::reset();
        meter::reset_limb_ops();
        let out = meter::skyline::text::parse(&s).expect("rendered text parses back");
        let run = Run {
            // 2n + 1 leaves: 2n delta codes follow the first leaf.
            deltas: 2 * scale as u64,
            bytes: s.len() as u64,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert_eq!(
            out, expected,
            "the parse must build the transcoder's stream"
        );
        assert!(
            run.touches >= run.deltas,
            "skyline_comb_parse scale {scale}: {} digit touches under the {}-delta floor: \
             the parse's path-sum accumulator is not running on the metered representation",
            run.touches,
            run.deltas,
        );
        run
    }

    /// The text parse's per-text-byte accumulator touches stay flat
    /// across a `k = n` doubling of the boundary comb's rendered text:
    /// the path-sum accumulator is cliff-immune on the crate's canonical
    /// untrusted-input surface.
    ///
    /// The parse is the touch-heaviest public surface measured, and its
    /// per-base ≤2× accumulator charge is what this pin holds in the
    /// aggregate (the `SKYLINE_PARSE_*` envelopes carry the other four
    /// columns). Text bytes are the row's honest denominator: each
    /// parsed base's join costs digit touches proportional to the base's
    /// own spelled width, so the comb's quadratically growing text pays
    /// for its own accumulator work — per delta the same reading grows
    /// with the tooth width and would misread as amplification. Each run
    /// also carries the one-touch-per-delta liveness floor (in
    /// [`comb_parse_run`]), so flatness is asserted over a meter proven
    /// live.
    #[test]
    fn skyline_parse_cliff_touch_cost_is_flat_per_unit() {
        let small = comb_parse_run(512);
        let large = comb_parse_run(1_024);
        assert_flat(
            "parse_touches",
            "text_byte",
            (small.touches, small.bytes),
            (large.touches, large.bytes),
        );
    }

    /// Rendering records exactly zero accumulator digit touches: the
    /// renderer's relative-coordinate summaries carry no running
    /// accumulator, and this pin is the conservation tripwire on the
    /// text seam.
    ///
    /// The parse direction carries the touch floor and flatness pins
    /// above; the render direction pins the measured zero, so render
    /// adopting an accumulator — or parse-side accumulator work leaking
    /// into the render path — moves a pinned constant instead of
    /// arriving silently. Re-pin only with the derivation that prices
    /// the new accumulator work.
    #[test]
    fn skyline_render_records_zero_touches() {
        for (packed, name) in [
            (meter::dense(4_096), "dense"),
            (meter::cliff_comb(512, 512), "cliff"),
            (meter::bigroot(8_000, 2_000), "bigroot"),
        ] {
            let v = packed.version();
            let enc = meter::skyline::encode(&v);
            touch_meter::reset();
            let out = meter::skyline::text::render(&enc);
            assert!(!out.is_empty(), "the render does real work");
            assert_eq!(
                touch_meter::touches(),
                0,
                "skyline_render_{name}: the renderer touched the accumulator; its \
                 delta-sized summaries are priced by the limb and heap columns, so new \
                 accumulator work here needs its own pin, not a silent arrival"
            );
        }
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
        let v = packed.version();
        let enc = meter::skyline::encode(&v);
        touch_meter::reset();
        meter::reset_limb_ops();
        let r = meter::skyline::query::rank(&enc);
        let run = Run {
            // Each tooth's two leaves follow the first leaf as deltas.
            deltas: 2 * n as u64,
            bytes: enc.as_raw_slice().len() as u64,
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
    /// (three identical runs).
    ///
    /// These are the cured freeze discipline's numbers, tightened from
    /// the retired quadratic baseline of record (101,716 → 396,126
    /// touches, 145,680 → 564,784 limbs at these scales) in the commit
    /// that landed the cure, per the ratchet convention. Measured: small
    /// 6,182 touches / 4,326 → 4,479 limbs on 24,085 skyline bytes;
    /// large 12,390 touches / 8,662 → 8,967 limbs on 48,245 bytes (limb
    /// movement 2026-07-24, metered `trailing_zeros`, under the standing
    /// ceilings).
    const FREEZE_BAND_OVER_TOUCH_CEILINGS: (u64, u64) = (7_728, 15_488);

    /// The over-threshold limb ceilings paired with
    /// [`FREEZE_BAND_OVER_TOUCH_CEILINGS`].
    const FREEZE_BAND_OVER_LIMB_CEILINGS: (u64, u64) = (5_408, 10_828);

    /// The rank kernel's freeze band on the wide-tooth comb, both sides
    /// flat: bounded oscillation never freezes at any tooth width.
    ///
    /// A fold's cost rides the live component, paid by the tooth's own
    /// code, so per-byte cost stays flat (×1.25) across a doubling of
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
        let v = packed.version();
        let enc = meter::skyline::encode(&v);
        touch_meter::reset();
        meter::reset_limb_ops();
        let r = meter::skyline::query::rank(&enc);
        let run = Run {
            // Each tooth's two leaves follow the first leaf as deltas.
            deltas: 2 * n as u64,
            bytes: enc.as_raw_slice().len() as u64,
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
    /// identical runs).
    ///
    /// The ceilings price one eviction of the `k`-bit jump plus flat
    /// 3-bit-delta work — the un-evicted alternative reads the jump's
    /// width again on every following delta, ~15× these numbers at the
    /// small scale alone. Measured: small 5,138 touches / 2,128 → 2,280
    /// limbs on 4,961 skyline bytes; large 10,272 touches / 4,250 →
    /// 4,554 limbs on 9,921 bytes (limb movement 2026-07-24, metered
    /// `trailing_zeros`, under the standing ceilings).
    const RANK_JUMP_TOUCH_CEILINGS: (u64, u64) = (6_423, 12_840);

    /// The jump-comb limb ceilings paired with
    /// [`RANK_JUMP_TOUCH_CEILINGS`].
    const RANK_JUMP_LIMB_CEILINGS: (u64, u64) = (2_660, 5_313);

    /// The rank kernel's freeze eviction on the jump comb is funded and
    /// flat.
    ///
    /// The mid-stream `k`-bit jump lands in the live component, the
    /// first cheap delta behind it fires the one freeze — priced by the
    /// drift the jump's own code paid for, never by the frozen width —
    /// and every later 3-bit delta rides an emptied live component, so
    /// per-byte cost stays flat (×1.25) across a doubling of `k` and `n`
    /// under absolute ceilings a stale-drift regression (the jump
    /// re-read per delta) exceeds ~15-fold.
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

// ─── id walk scan cost (the covers/disjoint scan pins) ──────────────────────
//
// The id walks' entire cost is scan bits: they allocate nothing, recurse
// nothing, and do no arithmetic, so the `ID_COVERS`/`ID_DISJOINT` envelope
// columns are all structurally near-zero and this counter is the one
// deterministic meter that sees the work. These pins hold each walk's scan
// reading to an absolute ceiling (measured ×1.25) over a full-examination
// liveness floor (one bit per packed operand byte — the diverted pair
// forces both walks to full lockstep depth), and to per-byte flatness
// (×1.25) across a depth doubling, so a re-scanning walk or a walk that
// leaves the metered primitives moves a committed number instead of
// passing every near-zero column unchanged.
#[cfg(feature = "scan-meter")]
mod id_walk_scan_cost {
    use super::{id_pair_input_bytes, party_of, ID_DEPTH};
    use before::meter;

    /// One walk run: packed operand bytes and the bits scanned by the
    /// walk body alone.
    struct Run {
        bytes: u64,
        bits: u64,
    }

    /// Absolute scan ceilings at the [`ID_DEPTH`] pair, measured
    /// 2026-07-26 ×1.25: covers 1,000,004 bits on 125,002 packed bytes
    /// (8 bits per byte: every stored tag read once, both operands).
    const COVERS_SCAN_CEILING_BITS: u64 = 1_250_005;

    /// The disjoint walk's ceiling paired with
    /// [`COVERS_SCAN_CEILING_BITS`] (measured 1,000,004 bits, the same
    /// full lockstep walk).
    const DISJOINT_SCAN_CEILING_BITS: u64 = 1_250_005;

    /// Run one id-pair walk at `depth` and read the scan counter over
    /// the body alone, enforcing the full-examination liveness floor.
    fn walk_run(
        name: &str,
        depth: usize,
        body: impl FnOnce(&before::Party, &before::Party),
    ) -> Run {
        let pa = meter::id_spine(depth, false);
        let pb = meter::id_spine(depth, true);
        let bytes = id_pair_input_bytes(&pa, &pb) as u64;
        let a = party_of(&pa);
        let b = party_of(&pb);
        meter::reset_scan_bits();
        body(&a, &b);
        let bits = meter::scan_bits();
        eprintln!("MEASURED id_walk_scan_{name}: depth={depth} bytes={bytes} scan_bits={bits}");
        assert!(
            bits >= bytes,
            "id_walk_scan_{name}: {bits} scanned bits under the one-bit-per-byte floor over \
             {bytes} packed bytes: the walk left the metered primitives"
        );
        Run { bytes, bits }
    }

    /// Per-byte scan cost stays flat (×1.25) across the depth doubling.
    fn assert_flat(name: &str, small: &Run, large: &Run) {
        assert!(
            u128::from(large.bits) * u128::from(small.bytes) * 4
                <= u128::from(small.bits) * u128::from(large.bytes) * 5,
            "id_walk_scan_{name}: per-byte scan cost grew more than x1.25 across the depth \
             doubling: {}/{} -> {}/{}",
            small.bits,
            small.bytes,
            large.bits,
            large.bytes,
        );
    }

    /// The covers walk's scan bits are absolute-pinned, floored, and flat
    /// per byte across a depth doubling of the diverted spine pair (which
    /// admits no early exit).
    ///
    /// The walk's cost is invisible to every other deterministic meter, so
    /// this pin is what a re-scanning `covers` (quadratic restarts) or an
    /// unmetered raw-indexing walk moves.
    #[test]
    fn id_covers_scan_cost_is_pinned_and_flat() {
        let small = walk_run("covers_small", ID_DEPTH / 2, |a, b| {
            assert!(!a.covers(b), "the divert arms are disjoint");
        });
        let large = walk_run("covers", ID_DEPTH, |a, b| {
            assert!(!a.covers(b), "the divert arms are disjoint");
        });
        assert_flat("covers", &small, &large);
        assert!(
            large.bits <= COVERS_SCAN_CEILING_BITS,
            "id_covers: {} scanned bits exceed the pinned ceiling {COVERS_SCAN_CEILING_BITS}",
            large.bits,
        );
    }

    /// The disjoint walk's scan bits are absolute-pinned, floored, and
    /// flat per byte across a depth doubling of the diverted spine pair
    /// (disjoint operands, so the walk runs to completion).
    ///
    /// Same rationale as the covers pin: scan is the one live column on
    /// this walk.
    #[test]
    fn id_disjoint_scan_cost_is_pinned_and_flat() {
        let small = walk_run("disjoint_small", ID_DEPTH / 2, |a, b| {
            assert!(a.is_disjoint(b), "the divert arms own disjoint regions");
        });
        let large = walk_run("disjoint", ID_DEPTH, |a, b| {
            assert!(a.is_disjoint(b), "the divert arms own disjoint regions");
        });
        assert_flat("disjoint", &small, &large);
        assert!(
            large.bits <= DISJOINT_SCAN_CEILING_BITS,
            "id_disjoint: {} scanned bits exceed the pinned ceiling \
             {DISJOINT_SCAN_CEILING_BITS}",
            large.bits,
        );
    }
}

// ─── fork envelope (the split kernel's committed cost record) ───────────────

/// The fork envelope: measured 2026-07-26 ×1.25 (dev profile, the envelope
/// suite's convention).
///
/// The split kernel builds both halves by raw bit-slice writes and walks
/// the spine by raw indexing — deliberately outside the scan primitives —
/// so the scan column pins the raw path's near-zero reading: routing the
/// walk or the writes through the metered primitives is a deliberate
/// re-pin (the number moving is the point), and until then the heap
/// column is the one that prices the halves' materialization.
#[rustfmt::skip]
mod fork_env {
    use super::{sweep_envelope, SweepEnvelope};
    //                                                    peak heap, segments, limb ops, scan bits, limb floor      measured: peak heap, segments, limb ops, scan bits
    pub const ID_FORK: SweepEnvelope = sweep_envelope(      156_253,        0,        0,         3, 0); //   125_002 (both halves materialize, ~2x the packed input), 0, 0, 2 (the raw split path; 2026-07-26)
}

/// Forking the deep id spine stays within its envelope, and the halves
/// rejoin into the original party byte for byte.
///
/// Fork is the one id operation with no committed cost record: its halves
/// materialize (the heap column prices them), its spine walk is iterative
/// (zero segments), and its writes are raw (the scan pin above). The
/// rejoin closes the semantic leg: fork then join is the identity.
#[test]
fn id_fork_envelope() {
    let pa = meter::id_spine(ID_DEPTH, false);
    let input = pa.bytes.len();
    let original = pa.bytes.clone();
    let mut a = party_of(&pa);
    let child = sweep_metered("id_fork", input, &fork_env::ID_FORK, || a.fork());
    a.join(child).expect("a fork's halves are disjoint");
    assert_eq!(
        a.encode(),
        original,
        "fork then join must reconstruct the original party"
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

    /// The static-prefix read stream: a cancelling prefix built once,
    /// then `n` cycles of `add_small(1)` / sign / `sub_small(1)` / sign.
    ///
    /// The prefix is `+2^k` then `−(2^k − 1)`, leaving value 1 spelled
    /// across `k/32` wide digits. Setup is excluded from the count; the
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
    /// ceiling at both scales and flat across the doubling.
    ///
    /// This is the stream on which any normalized-prefix-plus-window form
    /// is quadratic.
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
    /// flat across the doubling.
    ///
    /// Every deep sign scan here is funded by the wide delta immediately
    /// preceding it, so the stream's cost tracks its own coded size (the
    /// collapse itself is priced by `accum_static_prefix_touches_flat`,
    /// where no adjacent write funds the scans).
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
    /// `k`, `n` doubling.
    ///
    /// The sign fold's collapse pays for the deep scan exactly once, so a
    /// cancelling prefix built once and then read many times costs O(1)
    /// digit touches per read.
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
    /// Liveness floor under the limb column: measured ×0.75, per the
    /// module doc's floor convention.
    #[cfg(feature = "limb-meter")]
    limb_floor: u64,
    /// Liveness floor under the touch column: measured ×0.75.
    ///
    /// A touch reading below it means the scenario's accumulator work
    /// left the metered representation, and every touch ceiling above
    /// would hold vacuously (zero where the measured count is zero,
    /// under which the floor asserts nothing).
    #[cfg(feature = "limb-meter")]
    touch_floor: u64,
}

/// Build a [`QueryEnvelope`] from the five pinned columns and the limb
/// and touch floors.
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
    _limb_floor: u64,
    _touch_floor: u64,
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
        #[cfg(feature = "limb-meter")]
        limb_floor: _limb_floor,
        #[cfg(feature = "limb-meter")]
        touch_floor: _touch_floor,
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
// The limb floor column is the measured value ×0.75, rounded down (the
// module doc's liveness-floor convention).
#[rustfmt::skip]
mod query_env {
    use super::{query_envelope, QueryEnvelope};
    //                                                                        peak heap, segments,  limb ops, scan bits,   touches, limb floor, touch floor       measured: heap, seg, limb, scan, touches
    pub const SKYLINE_RANK_DENSE: QueryEnvelope           = query_envelope(    81_950,        0,   312_505, 1_093_772,   156_259, 187_503, 93_755); // 65_560, 0, 250_004 -> 250_005 (2026-07-24, metered trailing_zeros), 875_017, 125_007
    pub const SKYLINE_RANK_BIGROOT: QueryEnvelope         = query_envelope(    67_145,        0,    26_767,   387_530,    17_199, 16_646, 12_895); // 60_088 -> 61_516 (2026-07-24, dashu-int backend), 0, 21_413 -> 22_195 (2026-07-24, metered trailing_zeros), 310_024, 17_194
    pub const SKYLINE_RANK_HARMONIC: QueryEnvelope        = query_envelope(    71_705,        0,   165_122,   573_454,   248_324, 99_840, 148_994); // 57_364 -> 58_400 (2026-07-24, dashu-int backend), 0, 132_097 -> 133_121 (2026-07-24, metered trailing_zeros), 458_763, 198_659
    pub const SKYLINE_RANK_CLIFF: QueryEnvelope           = query_envelope(     3_075,        0,     7_805,    48_647,     8_008, 4_707, 4_804); // 2_460 -> 2_540 (2026-07-24, dashu-int backend), 0, 6_244 -> 6_277 (2026-07-24, metered trailing_zeros), 38_917, 6_406
    pub const SKYLINE_RANK_WIDE_TOOTH: QueryEnvelope      = query_envelope(     3_095,        0,    29_552, 2_996_319,    33_580, 17_755, 20_148); // 2_740 -> 2_820 (2026-07-24, dashu-int backend), 0, 23_641 -> 23_674 (2026-07-24, metered trailing_zeros), 2_397_055, 26_864
    pub const SKYLINE_MIN_TICKS_DENSE: QueryEnvelope      = query_envelope(    30_720,        0,   312_503,   468_758,   156_255, 187_501, 93_753); // 24_576, 0, 250_002, 375_006, 125_004
    pub const SKYLINE_MIN_TICKS_CLIFF: QueryEnvelope      = query_envelope(       660,        0,        22,     2_565,        62, 12, 36); // 528 -> 560 (2026-07-24, dashu-int backend), 0, 17, 2_052, 49
    pub const SKYLINE_PROJECT_COMB_SCATTER: QueryEnvelope = query_envelope(   525_700,        0,   115_265, 2_652_165,    44_924, 69_159, 26_954); // 420_560 -> 420_592 (2026-07-24, dashu-int backend), 0, 92_212, 2_124_806 -> 2_121_732 (2026-07-25, single-record id tags), 35_939
    pub const FOLD_VERSION_SCATTER: QueryEnvelope        = query_envelope(       488,        0,   317_380,   330_913,    63_347, 190_428, 38_007); // 73_216 -> 390, 0, 690_310 -> 253_904 (sequential 14_281_732), 163_866 -> 264_730, 0 -> 50_677 (2026-07-25, C2: operations route to the skyline kernels)
    pub const FOLD_PARTY_SCATTER: QueryEnvelope          = query_envelope(       420,        0,         0,   365_540,         0, 0, 0); // 336, 0, 0, 292_432 (sequential 3_284_952), 0
    // Tick rows (2026-07-25, the #34 currency round): moved onto the
    // five-meter harness — the tick walk's cost currency is accumulator
    // digit touches, which the four-column table these rows came from
    // never watched (nor scanned bits). Ceilings ×1.25 and floors ×0.75
    // over the measurements of record below.
    pub const TICK_DENSE: QueryEnvelope                   = query_envelope(    58_815,        0,   312_508,   468_765,   156_272, 187_504, 93_762); // 71_484 -> 47_052 (2026-07-26, the fused tick: copy-on-first-divergence defers the output buffer past the collapse scan, so the scan path and the builder no longer coexist at peak), 0, 250_008, 375_012, 125_017 (work columns byte-identical across the fusion)
    pub const TICK_NESTED_WIDE: QueryEnvelope             = query_envelope(     9_628,        7,    30_259,    80_028,    45_815, 18_155, 27_489); // 7_702, 5 -> 6 (2026-07-26, the fused walk's wider frames cross one more segment boundary; the ceiling stands), 24_207 -> 24_209 (2026-07-26, the fused tick; the ceiling stands), 64_022, 36_652 (the anchor web reads the wide first payload O(1) times)
    pub const TICK_MIRROR_WIDE: QueryEnvelope             = query_envelope(    32_467,       10,    50_390,   160_003,   131_948, 30_234, 79_169); // 27_397 -> 25_973 (2026-07-26, the fused tick's deferred output buffer), 8, 40_312, 128_002, 105_559 -> 109_560 (2026-07-26, the latent boundary register's O(1) tag work per close; the older ceiling stands) (the frame ledger stores no link for the shared wide minimum; heap parity with one queue word per site)
    // The expansion rows (2026-07-26, the fused tick): grow-branch deep
    // ticks measuring the whole public tick — walk, route fold, splice.
    // Baselines at the fusion's landing; the composed path's grow-only
    // measurements of record (splice passes alone, 2026-07-23) were
    // 390_181 heap / 500_012 limb / 2_250_015 scan on the spine and
    // 535_864 / 500_008 / 3_375_021 on the cross, without the fill
    // walk the tick also paid.
    pub const TICK_EXPAND_SPINE: QueryEnvelope            = query_envelope(   435_455,        0,   625_015, 2_187_519,         4, 375_009, 2); // 348_364, 0, 500_012, 1_750_015, 3 (an empty version's tick folds one word-scale payload: near-zero accumulator work)
    pub const TICK_EXPAND_CROSS: QueryEnvelope            = query_envelope(   611_237,        0,   937_513, 3_593_782,   312_517, 562_507, 187_509); // 488_989, 0, 750_010, 2_875_025, 250_013
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
    #[cfg(feature = "limb-meter")]
    assert!(
        limb_ops >= env.limb_floor,
        "{name}: limb counter reads {limb_ops}, below the {} liveness floor: \
         the meter is not watching this work",
        env.limb_floor,
    );
    #[cfg(feature = "limb-meter")]
    assert!(
        touches >= env.touch_floor,
        "{name}: touch counter reads {touches}, below the {} liveness floor: \
         the accumulator work left the metered representation",
        env.touch_floor,
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
        enc.as_raw_slice().len(),
        &query_env::SKYLINE_RANK_DENSE,
        || meter::skyline::query::rank(&enc),
    );
    assert_eq!(r, v.rank(), "the kernel must match the packed rank");
}

/// The rank kernel on the bigroot skyline stays within its envelope (the
/// wide-magnitude control).
///
/// The first leaf's magnitude seeds the frozen component and is read
/// exactly once, in the closing shifted add against the whole interval.
#[test]
fn skyline_rank_bigroot_envelope() {
    let p = meter::bigroot(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
    let v = version_of(&p);
    let enc = skyline_of(&p);
    let r = query_metered(
        "skyline_rank_bigroot",
        enc.as_raw_slice().len(),
        &query_env::SKYLINE_RANK_BIGROOT,
        || meter::skyline::query::rank(&enc),
    );
    assert_eq!(r, v.rank(), "the kernel must match the packed rank");
}

/// The rank kernel on the harmonic spine stays within its envelope — the
/// rank fold's separating family.
///
/// The fold is linear here because each level's one-leaf delta lands in
/// the accumulator at its own weight instead of re-shifting an
/// accumulated numerator.
#[test]
fn skyline_rank_harmonic_envelope() {
    let p = meter::harmonic(RANK_HARMONIC_DEPTH);
    let v = version_of(&p);
    let enc = skyline_of(&p);
    let r = query_metered(
        "skyline_rank_harmonic",
        enc.as_raw_slice().len(),
        &query_env::SKYLINE_RANK_HARMONIC,
        || meter::skyline::query::rank(&enc),
    );
    assert_eq!(r, v.rank(), "the kernel must match the packed rank");
}

/// The rank kernel on the boundary comb's skyline stays within its
/// envelope.
///
/// The heights are `2^k`-scale behind 3-bit deltas, the live component
/// absorbs the oscillation at O(1) digits per fold, and the terminal
/// borrow — as wide as its own code — rides the live component into the
/// last leaf's single wide add, no freeze anywhere.
#[test]
fn skyline_rank_cliff_envelope() {
    let p = meter::cliff_comb(CLIFF_SCALE, CLIFF_SCALE);
    let v = version_of(&p);
    let enc = skyline_of(&p);
    let r = query_metered(
        "skyline_rank_cliff",
        enc.as_raw_slice().len(),
        &query_env::SKYLINE_RANK_CLIFF,
        || meter::skyline::query::rank(&enc),
    );
    assert_eq!(r, v.rank(), "the kernel must match the packed rank");
}

/// The rank kernel on the wide-tooth comb's skyline stays within its
/// envelope — the no-freeze pin.
///
/// Bounded 192-bit oscillation keeps the live component exactly as wide
/// as each tooth's own code, so every fold and every per-leaf add is paid
/// by that code and the frozen component never churns (the
/// `skyline_flatness` freeze-band row pins the same shape above the
/// freeze allowance).
#[test]
fn skyline_rank_wide_tooth_envelope() {
    let p = meter::wide_tooth_comb(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
    let v = version_of(&p);
    let enc = skyline_of(&p);
    let r = query_metered(
        "skyline_rank_wide_tooth",
        enc.as_raw_slice().len(),
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
        enc.as_raw_slice().len(),
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
        enc.as_raw_slice().len(),
        &query_env::SKYLINE_MIN_TICKS_CLIFF,
        || meter::skyline::query::min_ticks(&enc),
    );
    assert_eq!(r, u64::MAX, "a comb height saturates the tick floor");
    assert_eq!(r, v.min_ticks(), "the kernel must match the packed fold");
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
    let p = meter::cliff_comb(CLIFF_SCALE, CLIFF_SCALE);
    let v = version_of(&p);
    let party = before::Party::decode(&meter::scattered_id(CLIFF_SCALE / 2).bytes[..])
        .expect("scattered id is strict normal form");
    let enc = skyline_of(&p);
    let io_bytes_in = enc.as_raw_slice().len() + meter::scattered_id(CLIFF_SCALE / 2).bytes.len();
    let out = query_metered(
        "skyline_project_comb_scatter",
        io_bytes_in,
        &query_env::SKYLINE_PROJECT_COMB_SCATTER,
        || meter::skyline::query::project(&enc, &party),
    );
    eprintln!(
        "MEASURED skyline_project_comb_scatter: output_bytes={}",
        out.as_raw_slice().len()
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
/// envelope.
///
/// The balanced reduction keeps every join's operands comparably sized,
/// so the fold is near-linear in the population's packed bytes where the
/// left fold re-scanned its whole accumulator per input.
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
/// envelope.
///
/// The id-side fold's work is pure stream scanning, and the balanced
/// reduction keeps the scanned bits near-linear in the population's
/// packed bytes where the left fold re-walked its whole accumulated
/// region per input.
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

// ─── the memo resolution's touch cost (the frame ledger's pins) ─────────────
//
// The committed witnesses that the memoized pre-scan's site resolution is
// LINEAR in accumulator digit touches on consumption-order adversaries —
// every ledger link read exactly once, dying into the raise decision it
// serves — while the shared-minimum control shows the records themselves
// cost nothing (zero links are unstored). Each family separates the
// frame ledger from a refuted resolution, so a regression re-admitting
// one reads over the x2.5 doubling ceiling: the chain family defeats any
// resolution that re-reads recorded differences per crossing interval
// (consumption order permutes recording order, so interval folds pay
// Θ(k) links per site); the comb family additionally defeats anchoring
// to the previously consumed site (its interleaved shallow/covering
// sites keep consecutive consumptions Θ(d) apart in recording order
// under that anchoring too) — a conforming resolution reads sites
// against the walk's own live relation, one link fold each.
#[cfg(feature = "limb-meter")]
mod memo_resolution_cost {
    use before::meter::{self, accum::touch_meter};
    use before::Party;

    /// One tick run over a memo family cross: the tick's packed
    /// input bytes and the accumulator digit touches of its body.
    ///
    /// The input is the version's own stored stream — the skyline
    /// coding, not the generator's construction language, whose
    /// per-leaf absolute codes overstate a plateau family's input by
    /// orders of magnitude — plus the id.
    struct Run {
        input: u64,
        touches: u64,
    }

    /// Tick the event × id cross and read the touch counter over the
    /// tick body alone.
    ///
    /// Enforces a one-touch-per-input-byte liveness floor before
    /// returning: the walk folds every consumed delta into the height
    /// accumulator, so a reading below the floor means the walk's
    /// accumulator work left the metered representation and any ratio
    /// over it would hold vacuously.
    fn tick_run(ev: meter::Packed, id: meter::Packed) -> Run {
        let mut v = ev.version();
        let p = Party::decode(&*id.bytes).expect("the generator's id is canonical");
        let input = (v.encode().len() + id.bytes.len()) as u64;
        touch_meter::reset();
        v.tick(&p);
        let run = Run {
            input,
            touches: touch_meter::touches(),
        };
        assert!(
            run.touches >= run.input,
            "memo family at {input} input bytes: {} digit touches under the \
             one-per-byte floor: the walk's accumulator work is not metered",
            run.touches,
        );
        run
    }

    /// Assert the linear signature: touches grow by at most ×2.5
    /// across a size doubling.
    ///
    /// Measured ×2.0 under the frame ledger; a resolution that
    /// re-reads links once per crossing reads ~×3.9 here. A reading
    /// over the ceiling means a link is being read more than once —
    /// re-pin only with a cure, never by deleting the family.
    fn assert_flat(name: &str, small: &Run, large: &Run) {
        eprintln!(
            "MEASURED {name}: small={}/{}B large={}/{}B",
            small.touches, small.input, large.touches, large.input,
        );
        assert!(
            u128::from(large.touches) * 2 <= u128::from(small.touches) * 5,
            "{name}: touch growth across the doubling exceeds x2.5 \
             ({} -> {}): a ledger link is being read more than once",
            small.touches,
            large.touches,
        );
    }

    /// Resolving the flat memo chain's distinct-minimum sites is
    /// linear in digit touches.
    ///
    /// `k` consumption-sibling sites' links each die into their own
    /// raise decision — one fold per link across the whole walk
    /// [measured: ×2.00 across the doubling, 62,021 → 124,021 at the
    /// pinned sizes (62,023 → 124,023 before the fused tick,
    /// 2026-07-26; 60,023 → 120,023 before the latent boundary
    /// register's O(1) tag work per close); ×3.94 under the refuted
    /// recording-chain interval resolution].
    #[test]
    fn memo_chain_distinct_resolution_reads_linear() {
        let small = tick_run(meter::memo_chain(1_000, true), meter::memo_chain_id(1_000));
        let large = tick_run(meter::memo_chain(2_000, true), meter::memo_chain_id(2_000));
        assert_flat("memo_chain_distinct", &small, &large);
    }

    /// The shared-minimum control stays flat per input byte (×1.25
    /// across the doubling).
    ///
    /// Zero recorded differences are unstored, so the same site
    /// structure with nothing to resolve costs linear touches — the
    /// quadratic lives in the resolution, not the records.
    #[test]
    fn memo_chain_shared_control_is_flat_per_unit() {
        let small = tick_run(meter::memo_chain(1_000, false), meter::memo_chain_id(1_000));
        let large = tick_run(meter::memo_chain(2_000, false), meter::memo_chain_id(2_000));
        eprintln!(
            "MEASURED memo_chain_shared: small={}/{}B large={}/{}B",
            small.touches, small.input, large.touches, large.input,
        );
        assert!(
            u128::from(large.touches) * u128::from(small.input) * 4
                <= u128::from(small.touches) * u128::from(large.input) * 5,
            "memo_chain_shared: per-byte touch cost grew more than x1.25 across \
             the size doubling: {}/{}B -> {}/{}B",
            small.touches,
            small.input,
            large.touches,
            large.input,
        );
    }

    /// Resolving the memo comb's interleaved sites is linear in digit
    /// touches.
    ///
    /// Consecutive consumptions sit Θ(d) apart in recording order —
    /// the shape that defeated the recording-chain and the
    /// previously-consumed-site resolutions alike — but every ledger
    /// link is a sibling or first-child difference read exactly once
    /// [measured: ×2.00 across the doubling, 44,032 → 88,032 at the
    /// pinned sizes (43,532 → 87,032 before the latent boundary
    /// register's O(1) tag work per close, 2026-07-26); ×3.92 under
    /// the refuted interval resolution].
    #[test]
    fn memo_comb_resolution_reads_linear() {
        let small = tick_run(meter::memo_comb(500), meter::memo_comb_id(500));
        let large = tick_run(meter::memo_comb(1_000), meter::memo_comb_id(1_000));
        assert_flat("memo_comb", &small, &large);
    }

    /// The wide fan-out's ledger cost is independent of the site
    /// count.
    ///
    /// `k` sibling sites sharing one wide minimum record zero links,
    /// and exactly one deferred link carries the width.
    ///
    /// The absolute ceiling is the k-independence assert — a
    /// discipline that materializes one wide record per site (the
    /// refuted floor-anchored recording) adds the width once per
    /// site and blows it [measured: 94,723 touches at k = 2,000,
    /// b = 2,048 (94,725 before the fused tick, 2026-07-26; 88,726,
    /// from which the pinned band derives, before the latent boundary
    /// register's O(1) tag work per close; the older ceiling stands
    /// over the rise) — a
    /// per-site fan-out at that width would add ~64 touches per site
    /// on top of the ~43-touch linear slope].
    #[test]
    fn memo_fanout_wide_cost_is_site_count_independent() {
        let small = tick_run(
            meter::memo_fanout(1_000, 2_048),
            meter::memo_chain_id(1_000),
        );
        let large = tick_run(
            meter::memo_fanout(2_000, 2_048),
            meter::memo_chain_id(2_000),
        );
        assert_flat("memo_fanout", &small, &large);
        assert!(
            large.touches <= 110_907,
            "memo_fanout: {} touches at k = 2,000 exceed the pinned absolute              ceiling 110,907 (measured 88,726 x1.25): a wide ledger quantity              is being materialized per site",
            large.touches,
        );
        assert!(
            large.touches >= 66_544,
            "memo_fanout: {} touches read below the 66,544 liveness floor              (measured 88,726 x0.75): the ledger's work left the metered              representation",
            large.touches,
        );
    }

    /// Oscillating sibling minima cost flat touches per input byte.
    ///
    /// Every sibling link is wide, and every one is funded
    /// one-for-one by the input code that stores its site's minimum
    /// — the funding control for the ledger's cost argument.
    #[test]
    fn memo_oscillating_links_are_input_funded() {
        let small = tick_run(
            meter::memo_oscillating(1_000, 512),
            meter::memo_chain_id(1_000),
        );
        let large = tick_run(
            meter::memo_oscillating(2_000, 512),
            meter::memo_chain_id(2_000),
        );
        eprintln!(
            "MEASURED memo_oscillating: small={}/{}B large={}/{}B",
            small.touches, small.input, large.touches, large.input,
        );
        assert!(
            u128::from(large.touches) * u128::from(small.input) * 4
                <= u128::from(small.touches) * u128::from(large.input) * 5,
            "memo_oscillating: per-byte touch cost grew more than x1.25 across              the size doubling: {}/{}B -> {}/{}B",
            small.touches,
            small.input,
            large.touches,
            large.input,
        );
    }

    /// Full-penetration minimum drops with recorded minima in flight
    /// cost one fold each.
    ///
    /// The descending run undercuts every open range while `d`
    /// sibling records ride the one live ledger head [measured:
    /// ×2.00 across the doubling, 84,817 → 169,617 (84,819 → 169,619
    /// before the fused tick, 2026-07-26; 80,019 → 160,019 before the
    /// latent boundary register landed)]. A discipline
    /// keeping one live record per open level folds all `d` per
    /// drop — the refuted live-anchored followers' tombstone.
    #[test]
    fn memo_churn_undercuts_fold_one_follower() {
        let small = tick_run(meter::memo_churn(800), meter::memo_churn_id(800));
        let large = tick_run(meter::memo_churn(1_600), meter::memo_churn_id(1_600));
        assert_flat("memo_churn", &small, &large);
    }

    /// Raises landing below the frame's minimum at every consume
    /// stay linear — and, foremost, semantically exact.
    ///
    /// The family's every raise moves the tracked minimum between
    /// the ledger relation's install and its next read, so a
    /// decide-then-emit ordering violation (a relation installed
    /// after the raise's arm) produces wrong values its oracle
    /// differential catches; this pin carries the cost leg
    /// [measured: ×2.00 across the doubling, 48,048 → 96,048
    /// (48,052 → 96,052 before the fused tick, 2026-07-26; 46,452 →
    /// 92,852 before the latent boundary register landed)].
    #[test]
    fn descending_raises_stay_linear_under_min_movement() {
        let small = tick_run(
            meter::descending_raises(800),
            meter::descending_raises_id(800),
        );
        let large = tick_run(
            meter::descending_raises(1_600),
            meter::descending_raises_id(1_600),
        );
        assert_flat("descending_raises", &small, &large);
    }
}

// ─── the width-circulation cycle's touch cost (the reveal-comb pins) ────────
//
// The committed witnesses that the tick walk's close-reveal cycle pays
// the wide consume-time GAP at most once total — never once per site.
// The families: k sibling sites sharing one wide minimum over a low
// floor, each site's node frame closing back into the floor frame
// between consecutive consumes, so a per-site width leak reads Θ(k·b)
// touches on a Θ(k + b) input whose output is Θ(k + b) too (each
// site's fill collapses to the shared plateau leaf; the per-site
// output deltas are unit codes) — an amplification the input+output
// denominator cannot excuse. The watermark stack's latent boundary
// register is what these pins hold: a close MOVES the popped wide
// boundary into the register and the next consume's arm recycles it
// by a narrow anchor-relative fold, with the relation follower going
// anchor-relative under a one-bit tag — so the cycle's marginal cost
// is the unit inter-site movement, and touches read ×2 across the
// joint (k, b) doubling. A regression re-introducing any per-site
// width read (a close that folds the boundary into the stack or a
// follower, a consume that grosses the anchor-to-floor gap into its
// decision) drives these families back toward ×4 and the ceilings
// below catch it. Semantics are exact on every family here — the
// oracle differential pools carry the full crossing, and each pin
// below asserts its shape's closed-form tick — so these pins carry
// the cost leg alone. The high-floor control (identical forest,
// identical deferral and close-reveal cycle, consume-time gap 2)
// separates the wide gap from the shape; the pure comb (no left-full
// site anywhere: no memo, no pre-scan) pins the base watermark
// stack's own arm-move + close-pop cycle in isolation.
#[cfg(feature = "limb-meter")]
mod width_circulation_cost {
    use before::meter::{self, accum::touch_meter};
    use before::{Party, Version};
    use dashu_int::UBig;

    /// One tick run over a family cross: the tick's packed input bytes
    /// (the version's own stored stream plus the id), the accumulator
    /// digit touches of its body, and the ticked version for the
    /// closed-form semantic leg.
    struct Run {
        input: u64,
        touches: u64,
        ticked: Version,
    }

    /// Tick the event × id cross and read the touch counter over the
    /// tick body alone.
    ///
    /// Enforces a one-touch-per-input-byte liveness floor before
    /// returning: the walk folds every consumed delta into the height
    /// accumulator, so a reading below the floor means the walk's
    /// accumulator work left the metered representation and any ratio
    /// over it would hold vacuously.
    fn tick_run(ev: meter::Packed, id: meter::Packed) -> Run {
        let mut v = ev.version();
        let p = Party::decode(&*id.bytes).expect("the generator's id is canonical");
        let input = (v.encode().len() + id.bytes.len()) as u64;
        touch_meter::reset();
        v.tick(&p);
        let run = Run {
            input,
            touches: touch_meter::touches(),
            ticked: v,
        };
        assert!(
            run.touches >= run.input,
            "reveal family at {input} input bytes: {} digit touches under the \
             one-per-byte floor: the walk's accumulator work is not metered",
            run.touches,
        );
        run
    }

    /// The shared plateau value `2^b` as decimal text, for the
    /// closed-form expected trees.
    fn plateau(b: usize) -> String {
        (UBig::ONE << b).to_string()
    }

    /// The reveal comb's close-reveal cycle is gap-funded flat —
    /// touches grow by at most ×2.5 across the joint (k, b) doubling
    /// on a ×2 input, under an absolute band on the larger run.
    ///
    /// Semantics first: the tick is the closed form (every site
    /// collapses to the shared plateau leaf; the covering raise stays
    /// at the floor), so this pin carries the cost leg alone. The
    /// signature [measured: 48,853 → 97,701 touches across
    /// (k, b) = (1,000, 1,024) → (2,000, 2,048), ×2.00 on a ×2.00
    /// input (48,857 → 97,705 before the fused tick, 2026-07-26, the
    /// pinned band's derivation; 738,449 → 2,884,881 (×3.91) before
    /// the latent boundary register landed)]: the consume-minted width-b
    /// boundary difference parks in the latent register at the site's
    /// close and the next consume's arm recycles it by a narrow
    /// anchor-relative fold, so no hop re-reads the width. A reading
    /// over the growth ceiling means a per-site width read is back —
    /// re-pin only with a cure, never by deleting the family.
    #[test]
    fn reveal_comb_close_reveal_cycle_reads_width_quadratic() {
        let expected = |k: usize, b: usize| -> Version {
            let w = plateau(b);
            let mut text = format!("(0, 0, {}(0, 0, {w})", "(0, ".repeat(k - 1));
            text.push_str(&format!(", {w})").repeat(k - 1));
            text.push(')');
            text.parse().expect("the reveal-comb literal parses")
        };
        let small = tick_run(
            meter::reveal_comb(1_000, 1_024),
            meter::reveal_comb_id(1_000),
        );
        assert_eq!(
            small.ticked,
            expected(1_000, 1_024),
            "reveal_comb ticks to its closed form: the failure is cost-only"
        );
        let large = tick_run(
            meter::reveal_comb(2_000, 2_048),
            meter::reveal_comb_id(2_000),
        );
        assert_eq!(
            large.ticked,
            expected(2_000, 2_048),
            "reveal_comb ticks to its closed form: the failure is cost-only"
        );
        eprintln!(
            "MEASURED reveal_comb: small={}/{}B large={}/{}B",
            small.touches, small.input, large.touches, large.input,
        );
        assert!(
            u128::from(large.touches) * 2 <= u128::from(small.touches) * 5,
            "reveal_comb: touch growth across the joint doubling exceeds x2.5 \
             ({} -> {}): a per-site width read is back in the close-reveal cycle",
            small.touches,
            large.touches,
        );
        assert!(
            large.touches <= 122_131,
            "reveal_comb: {} touches at (k, b) = (2,000, 2,048) exceed the pinned \
             ceiling 122,131 (measured 97,705 x1.25, 2026-07-26)",
            large.touches,
        );
        assert!(
            large.touches >= 73_278,
            "reveal_comb: {} touches read below the 73,278 liveness floor \
             (measured 97,705 x0.75, 2026-07-26): the cycle's work left the \
             metered representation",
            large.touches,
        );
    }

    /// The pure comb's arm-move + close-pop cycle is flat in the base
    /// watermark stack alone — at most ×1.15 per-byte touch growth
    /// across a width doubling at fixed site count, under an absolute
    /// band on the larger run.
    ///
    /// Semantics first: fill is the identity here (no left-full site
    /// exists), so the tick is grow's closed form — the shallowest
    /// owned leaf expands, ties right. The signature [measured:
    /// per-byte 3.71 → 3.20 across b = 1,024 → 2,048 at k = 1,000
    /// (the widening input divides a flat count; 5.18 → 4.46 before
    /// the fused tick skipped the matched pass-through emissions'
    /// output materializations, 2026-07-26; 50.8 → 82.0 (×1.61)
    /// before the latent boundary register landed)]: each
    /// wide leaf's frame closes its width-`b` boundary into the latent
    /// register by move and the next arm recycles it at the zero
    /// inter-site offset — no memo, no pre-scan, and no site consume
    /// anywhere, so this family pins the base stack's own cycle in
    /// isolation from the frame ledger.
    #[test]
    fn pure_comb_width_cycle_reads_width_scaled() {
        let expected = |k: usize, b: usize| -> Version {
            let w = plateau(b);
            let mut text = format!("{}(0, 0, {w})", "(0, ".repeat(k - 1));
            text.push_str(&format!(", {w})").repeat(k - 2));
            text.push_str(&format!(", ({w}, 1, 0))"));
            text.parse().expect("the pure-comb literal parses")
        };
        let small = tick_run(meter::pure_comb(1_000, 1_024), meter::pure_comb_id(1_000));
        assert_eq!(
            small.ticked,
            expected(1_000, 1_024),
            "pure_comb ticks to grow's closed form: the failure is cost-only"
        );
        let large = tick_run(meter::pure_comb(1_000, 2_048), meter::pure_comb_id(1_000));
        assert_eq!(
            large.ticked,
            expected(1_000, 2_048),
            "pure_comb ticks to grow's closed form: the failure is cost-only"
        );
        eprintln!(
            "MEASURED pure_comb: small={}/{}B large={}/{}B",
            small.touches, small.input, large.touches, large.input,
        );
        assert!(
            u128::from(large.touches) * u128::from(small.input) * 100
                <= u128::from(small.touches) * u128::from(large.input) * 115,
            "pure_comb: per-byte touch growth across the width doubling exceeds \
             x1.15 ({}/{}B -> {}/{}B): the base stack's arm-move + close-pop cycle \
             has picked up a width term",
            small.touches,
            small.input,
            large.touches,
            large.input,
        );
        assert!(
            large.touches <= 6_544,
            "pure_comb: {} touches at (k, b) = (1,000, 2,048) exceed the pinned \
             ceiling 6,544 (measured 5,235 x1.25, 2026-07-26, the fused tick)",
            large.touches,
        );
        assert!(
            large.touches >= 3_926,
            "pure_comb: {} touches read below the 3,926 liveness floor \
             (measured 5,235 x0.75, 2026-07-26, the fused tick): the cycle's \
             work left the metered representation",
            large.touches,
        );
    }

    /// Absolute touch ceiling on the high-floor control's larger run,
    /// measured 50,837 ×1.25 (2026-07-26, three identical runs).
    ///
    /// Movement: 56,831 → 50,837 when the latent boundary register
    /// landed — the deleted close and consume folds this family paid
    /// narrow.
    const HIFLOOR_TOUCH_CEILING: u64 = 63_547;

    /// Touch liveness floor paired with [`HIFLOOR_TOUCH_CEILING`]:
    /// measured ×0.75.
    const HIFLOOR_TOUCH_FLOOR: u64 = 38_127;

    /// GREEN PIN: the high-floor control is flat and width-independent
    /// — identical forest, identical deferral and close-reveal cycle,
    /// consume-time gap 2.
    ///
    /// Per-byte touches stay flat (×1.25) across the width QUADRUPLING
    /// the wide family scales with [measured: 19.1 → 16.9 per byte
    /// across b = 512 → 2,048 at k = 1,000; 21.4 → 18.9 before the
    /// latent boundary register landed, 2026-07-26], under an absolute
    /// band on the larger run. The wide GAP is the cycle's cost driver
    /// — not the site forest, not the deferral, not the close-reveal
    /// schedule, all of which this family shares with the wide one.
    #[test]
    fn reveal_comb_hifloor_control_is_flat_per_unit() {
        let expected = |k: usize, b: usize| -> Version {
            // The raised floor 2^b − 2 lifts to the root: it is the
            // filled tree's minimum (the covering raise meets it).
            let floor = (UBig::ONE << b) - UBig::from(2u8);
            let mut text = format!("({floor}, 0, {}(0, 0, 2)", "(0, ".repeat(k - 1));
            text.push_str(&", 2)".repeat(k - 1));
            text.push(')');
            text.parse().expect("the high-floor literal parses")
        };
        let small = tick_run(
            meter::reveal_comb_hifloor(1_000, 512),
            meter::reveal_comb_id(1_000),
        );
        assert_eq!(
            small.ticked,
            expected(1_000, 512),
            "the high-floor control ticks to its closed form"
        );
        let large = tick_run(
            meter::reveal_comb_hifloor(1_000, 2_048),
            meter::reveal_comb_id(1_000),
        );
        assert_eq!(
            large.ticked,
            expected(1_000, 2_048),
            "the high-floor control ticks to its closed form"
        );
        eprintln!(
            "MEASURED reveal_comb_hifloor: small={}/{}B large={}/{}B",
            small.touches, small.input, large.touches, large.input,
        );
        assert!(
            u128::from(large.touches) * u128::from(small.input) * 4
                <= u128::from(small.touches) * u128::from(large.input) * 5,
            "reveal_comb_hifloor: per-byte touch cost grew more than x1.25 across \
             the width quadrupling: {}/{}B -> {}/{}B — the narrow-gap cycle has \
             picked up a width term",
            small.touches,
            small.input,
            large.touches,
            large.input,
        );
        assert!(
            large.touches <= HIFLOOR_TOUCH_CEILING,
            "reveal_comb_hifloor: {} touches exceed the pinned ceiling \
             {HIFLOOR_TOUCH_CEILING} (measured 50,837 x1.25)",
            large.touches,
        );
        assert!(
            large.touches >= HIFLOOR_TOUCH_FLOOR,
            "reveal_comb_hifloor: {} touches read below the {HIFLOOR_TOUCH_FLOOR} \
             liveness floor (measured 50,837 x0.75): the cycle's work left the \
             metered representation",
            large.touches,
        );
    }

    /// The grown ascending-cliff closed form: the input spine with the
    /// cliff grown to `(0, 1, 0)`.
    fn ascend_cliff_ticked(k: usize, b: usize) -> Version {
        let w = UBig::ONE << b;
        let mut text = String::new();
        for i in 1..=k {
            text.push_str(&format!("(0, {}, ", &w + UBig::from(i)));
        }
        text.push_str("(0, 1, 0)");
        text.push_str(&")".repeat(k));
        text.parse()
            .expect("the grown ascending-cliff literal parses")
    }

    /// The undercut cascade is dying-digit-funded flat — touches grow
    /// by at most ×2.5 across the joint (k, b) doubling on a ×2 input,
    /// under an absolute band on the larger run.
    ///
    /// Semantics first: fill is the identity (no id region covers a
    /// subdividable subtree at its minimum), so the tick is grow's
    /// closed form — the owned cliff leaf expands to `(0, 1, 0)` — and
    /// this pin carries the cost leg alone. The signature [measured:
    /// 10,495 → 20,975 touches across (k, b) =
    /// (1,000, 2,048) → (2,000, 4,096), ×2.00 on a ×2.00 input
    /// (12,626 → 25,234 before the fused tick skipped the matched
    /// pass-through emissions' output materializations, 2026-07-26;
    /// 203,435 → 790,851 (×3.89) before the cascade's fold direction
    /// inverted)]: the cliff's single wide undercut
    /// penetrates k − 1 nonzero unit boundary differences, each dying
    /// by one fold into the surviving residue at the difference's own
    /// width, top-index domination deciding every hop in O(1). A
    /// reading over the growth ceiling means a per-hop residue-width
    /// read is back — re-pin only with a cure, never by deleting the
    /// family.
    #[test]
    fn ascend_cliff_undercut_cascade_reads_residue_width() {
        let small = tick_run(
            meter::ascend_cliff(1_000, 2_048),
            meter::ascend_cliff_id(1_000),
        );
        assert_eq!(
            small.ticked,
            ascend_cliff_ticked(1_000, 2_048),
            "ascend_cliff ticks to grow's closed form: the failure is cost-only"
        );
        let large = tick_run(
            meter::ascend_cliff(2_000, 4_096),
            meter::ascend_cliff_id(2_000),
        );
        assert_eq!(
            large.ticked,
            ascend_cliff_ticked(2_000, 4_096),
            "ascend_cliff ticks to grow's closed form: the failure is cost-only"
        );
        eprintln!(
            "MEASURED ascend_cliff: small={}/{}B large={}/{}B",
            small.touches, small.input, large.touches, large.input,
        );
        assert!(
            u128::from(large.touches) * 2 <= u128::from(small.touches) * 5,
            "ascend_cliff: touch growth across the joint doubling exceeds x2.5 \
             ({} -> {}): a per-hop residue-width read is back in the undercut \
             cascade",
            small.touches,
            large.touches,
        );
        assert!(
            large.touches <= 26_219,
            "ascend_cliff: {} touches at (k, b) = (2,000, 4,096) exceed the pinned \
             ceiling 26,219 (measured 20,975 x1.25, 2026-07-26, the fused tick)",
            large.touches,
        );
        assert!(
            large.touches >= 15_731,
            "ascend_cliff: {} touches read below the 15,731 liveness floor \
             (measured 20,975 x0.75, 2026-07-26, the fused tick): the cascade's \
             work left the metered representation",
            large.touches,
        );
    }

    /// Absolute touch ceiling on the leveled control's larger run,
    /// measured 8,981 ×1.25 (2026-07-26, three identical runs of the
    /// fused tick; 13,240 before it skipped the matched pass-through
    /// emissions' output materializations).
    const PLATEAU_TOUCH_CEILING: u64 = 11_227;

    /// Touch liveness floor paired with [`PLATEAU_TOUCH_CEILING`]:
    /// measured ×0.75.
    const PLATEAU_TOUCH_FLOOR: u64 = 6_735;

    /// GREEN PIN: the leveled control is flat — identical spine,
    /// identical arming schedule, identical cliff undercut, all
    /// boundary differences zero.
    ///
    /// Per-byte touches stay flat (×1.25) across the joint (k, b)
    /// doubling the ascending family scales with [measured: 4.02 →
    /// 4.01 per byte across (1,000, 2,048) → (2,000, 4,096),
    /// 2026-07-26], under an absolute band on the larger run. The
    /// nonzero differences are the cascade's cost driver — with the
    /// stack one compressed zero run, the same wide undercut passes it
    /// whole in O(1) — so the hop schedule, not the undercut or the
    /// spine, carries the red family's growth.
    #[test]
    fn ascend_cliff_plateau_control_is_flat_per_unit() {
        let expected = |k: usize, b: usize| -> Version {
            let w = (UBig::ONE << b) + UBig::from(1u8);
            let mut text = String::new();
            for _ in 0..k {
                text.push_str(&format!("(0, {w}, "));
            }
            text.push_str("(0, 1, 0)");
            text.push_str(&")".repeat(k));
            text.parse()
                .expect("the grown leveled-cliff literal parses")
        };
        let small = tick_run(
            meter::ascend_cliff_plateau(1_000, 2_048),
            meter::ascend_cliff_id(1_000),
        );
        assert_eq!(
            small.ticked,
            expected(1_000, 2_048),
            "the leveled control ticks to grow's closed form"
        );
        let large = tick_run(
            meter::ascend_cliff_plateau(2_000, 4_096),
            meter::ascend_cliff_id(2_000),
        );
        assert_eq!(
            large.ticked,
            expected(2_000, 4_096),
            "the leveled control ticks to grow's closed form"
        );
        eprintln!(
            "MEASURED ascend_cliff_plateau: small={}/{}B large={}/{}B",
            small.touches, small.input, large.touches, large.input,
        );
        assert!(
            u128::from(large.touches) * u128::from(small.input) * 4
                <= u128::from(small.touches) * u128::from(large.input) * 5,
            "ascend_cliff_plateau: per-byte touch cost grew more than x1.25 across \
             the joint doubling: {}/{}B -> {}/{}B — the zero-run cascade has \
             picked up a width term",
            small.touches,
            small.input,
            large.touches,
            large.input,
        );
        assert!(
            large.touches <= PLATEAU_TOUCH_CEILING,
            "ascend_cliff_plateau: {} touches exceed the pinned ceiling \
             {PLATEAU_TOUCH_CEILING} (measured 8,981 x1.25)",
            large.touches,
        );
        assert!(
            large.touches >= PLATEAU_TOUCH_FLOOR,
            "ascend_cliff_plateau: {} touches read below the {PLATEAU_TOUCH_FLOOR} \
             liveness floor (measured 8,981 x0.75): the cascade's work left the \
             metered representation",
            large.touches,
        );
    }
}
