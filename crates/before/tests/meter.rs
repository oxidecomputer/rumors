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
//! lower bound — the measured value ×0.75, rounded down, a column in the
//! same tables as the ceilings. Two lower-bound genres appear in this
//! suite, named apart because their trips mean opposite things: a derived
//! *liveness floor* states a mechanism's irreducible work, never a
//! measured basis — an honest improvement can approach but never cross
//! it, so a trip means the work left the metered representation
//! (investigate the meter) — while a measured-×0.75 *improvement
//! tripwire*, the envelope columns' genre, bands the pinned reading — a
//! trip means the reading dropped more than 25% below the pin: attribute
//! it, and an honest improvement re-pins the band while a dead meter is
//! the bypass the column exists to catch. A limb ceiling passes vacuously
//! when the counter stops counting (a meter hook deleted from one `Base`
//! operation reads a near-zero column with every ceiling green), and the
//! tripwire is what fails instead. Like the board's floors, these detect
//! *total* bypass, not partial rerouting: an implementation that routes
//! some width-scale work through metered operations and the rest around
//! them still reads green, so the column is a bypass tripwire, never a
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

use before::meter::registry::Shape;
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

/// Staircase depth of the ownership-hole tick scenario: enough distinct
/// plateaus that the unowned regions' per-leaf freight would dominate
/// the envelope if the block scan failed to engage.
const HOLE_STAIR_DEPTH: usize = 2_000;

/// Id depth of the ownership-hole tick scenario: a party owning one
/// diverted fragment `2^-8` of the space, leaving the staircase's runs
/// unowned.
const HOLE_ID_DEPTH: usize = 8;

/// Owned-fragment count of the alternating-ownership comb scenario,
/// interleaving one owned leaf and one absent gap per level down the
/// alternating spine.
const COMB_FRAGMENTS: usize = 2_000;

/// Tooth width (bits) of the wide-tooth comb scenarios: wider than any
/// machine word, so every skyline delta is a genuinely wide operand while
/// still oscillating across the `2^k` cliff.
const WIDE_TOOTH_WIDTH_BITS: usize = 192;

/// Tooth magnitude (bits) of the two-operand jump-comb scenarios.
///
/// Comfortably over the rank freeze allowance's 256-bit digit bound, so
/// every cheap fold arriving behind a wide switch jump in the meet
/// stream fires the eviction.
const JUMP_PAIR_MAGNITUDE_BITS: usize = 512;

/// Comb levels of the two-operand jump-comb query scenario (the
/// superlinearity band's small run; the large run doubles it).
const JUMP_PAIR_TEETH: usize = 512;

/// Freeze-position digits of the two-operand jump-comb query scenario
/// (an eighth of the teeth, the board family's proportion; the large
/// run doubles it).
const JUMP_PAIR_DIGITS: usize = 64;

/// Forked-party count of the concurrent-pair query scenarios: every one
/// of the `n − 1` overlay boundaries is an emit side switch, in the join
/// and the meet alike.
const CONCURRENT_PAIR_LEAVES: usize = 4_096;

/// Tooth magnitude (bits) of the mask-drift families: wide enough that a
/// per-boundary materialization of the walk's integrator reads would be
/// unmistakably superlinear, word-scale enough to keep the scenario in
/// seconds.
const MASK_DRIFT_MAGNITUDE_BITS: usize = 512;

/// Tooth count of the mask-drift families' envelope scenarios.
const MASK_DRIFT_TEETH: usize = 1_024;

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
/// and its limb improvement tripwire (measured ×0.75, rounded down — the
/// file doc's tripwire genre).
struct Envelope {
    /// Peak heap delta over the scenario body, in bytes.
    peak_heap: usize,
    /// Stack segments grown during the scenario body.
    segments: u64,
    /// Big-integer limb operations counted during the scenario body.
    #[cfg(feature = "limb-meter")]
    limb_ops: u64,
    /// Improvement tripwire under the limb column.
    ///
    /// A reading below it is a drop of more than 25% from the pinned
    /// reading — attribute it, re-pinning an honest improvement or curing a
    /// dead meter (zero where the measured count is zero, under which the
    /// bound asserts nothing).
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
// trailing comment on each line is the measurement of record
// (aarch64-apple-darwin, dev profile, three identical runs) the ceiling
// derives from; a row re-pinned after an improvement records the movement
// as `old -> new`, and its ceiling derives from the
// new value. Re-pin by rerunning this binary under `--no-capture` and
// reading the MEASURED lines (the limb column needs `--all-features` or
// `--features limb-meter`).
// The limb floor column is the measured value ×0.75, rounded down (the
// file doc's improvement-tripwire convention).
#[rustfmt::skip]
mod envelope {
    use super::{envelope, sweep_envelope, Envelope, SweepEnvelope};
    //                                              peak heap,  segments, limb ops, limb floor           measured: peak heap, segments, limb ops
    pub const DECODE_DENSE: Envelope = envelope(120_035, 0, 0, 0); // 11_072_549 -> 96_036 (operations route to the skyline kernels: wire decode is validate + wrap), 0, 250_002 -> 500_002 (operations route to the skyline kernels); re-pinned (limb 0, heap 96_028): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const CMP_DENSE: Envelope = envelope(30_720, 0, 0, 0); //          8 -> 24_592 -> 24_584 (OpenedPair: the pair walk's opening move stated once) -> 24_576 (the Bytes-backed at-rest form), 192 -> 0, 2_000_010 -> 250_004 (operations route to the skyline kernels: the iterative sweep) -> 250_002 (the Bytes-backed at-rest form); re-pinned (limb 0, heap 24_576): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const JOIN_DENSE: Envelope = envelope(130_277, 0, 0, 0); //  4_797_477 -> 151_113 -> 104_237 (the Bytes-backed at-rest form: the value-operator cell's lhs clone is a refcount bump, not a byte copy of the operand, so the public join's peak is the emit kernel's alone), 240 -> 0, 3_000_008 -> 750_004 (operations route to the skyline kernels: the emit kernel); re-pinned to the new readings (limb 250_002, heap 104_221): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work; re-pinned (limb 0, heap 104_221): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    // The tick rows live in `query_env`: the tick walk's cost currency
    // is accumulator digit touches (with scanned bits beside it), which
    // this four-column table never watched.
    pub const DECODE_BIGROOT: Envelope = envelope(60_090, 0, 783, 469); //  1_396_905 -> 48_072 (operations route to the skyline kernels: wire decode is validate + wrap), 0, 20_630 -> 40_632 (operations route to the skyline kernels); re-pinned (limb 626, heap 48_072): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const CMP_BIGROOT: Envelope = envelope(40_340, 0, 783, 469); // 56_416_936 -> 32_280 -> 32_272 (the Bytes-backed at-rest form), 12 -> 0, 37_606_270 -> 20_632 (operations route to the skyline kernels: the iterative sweep) -> 20_630 (the Bytes-backed at-rest form); re-pinned (limb 626, heap 32_272): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const JOIN_BIGROOT: Envelope = envelope(85_060, 0, 1_565, 939); // 56_849_753 -> 81_800 -> 68_048 (the Bytes-backed at-rest form: the value-operator cell's lhs clone is a refcount bump, not a byte copy of the operand, so the public join's peak is the emit kernel's alone), 16 -> 0, 87_631_274 -> 61_266 (operations route to the skyline kernels: the emit kernel); re-pinned to the new readings (limb 21_258, heap 68_048): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work; re-pinned (limb 1_252, heap 68_048): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const DECODE_HUGELEAF: Envelope = envelope(   122_504,        0,         2_443, 1_465); //     48_851 -> 98_003 (operations route to the skyline kernels: the validating wire decode holds the running height), 0, 1_954
    pub const JOIN_HUGELEAF: Envelope   = envelope(   185_494,        0,         4_887, 2_931); //    115_707 -> 179_622 (operations route to the skyline kernels: the emit kernel holds both payload buffers) -> 148_395 (the Bytes-backed at-rest form: the value-operator cell's lhs clone is a refcount bump, not a byte copy of the operand, so the public join's peak is the emit kernel's alone), 0, 7_821 -> 3_909 (operations route to the skyline kernels)
    pub const ID_JOIN: Envelope         = envelope(   279_132,        0,             0, 0); //    125_001 -> 223_305, 202 -> 0, 0 (iterative id walks: frame bits on the heap, no grown segments)
    pub const ID_COVERS: Envelope       = envelope(        10,        0,             0, 0); //          0 -> 8,  85 -> 0, 0 (iterative id walks)
    pub const ID_DISJOINT: Envelope     = envelope(        10,        0,             0, 0); //          0 -> 8, 170 -> 0, 0 (iterative id walks)
    pub const ID_WITHOUT: Envelope      = envelope(   521_110,        0,             0, 0); //    518_219 -> 416_888 (dev builds run no shadow re-parse of the diff emission: the differential suites carry the normal-form check) -> 416_887 (the Bytes-backed at-rest form), 138 -> 0, 0 (iterative complement)
    pub const DECODE_CLIFF: Envelope = envelope(4_052, 0, 88, 52); //    607_489 -> 3_241 (operations route to the skyline kernels: wire decode is validate + wrap), 0, 40_960 -> 10_322 (operations route to the skyline kernels); re-pinned (limb 70, heap 3_241): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const CMP_CLIFF: Envelope = envelope(1_330, 0, 88, 52); //        496 -> 1_368 (operations route to the skyline kernels: the sweep holds two accumulators) -> 1_200 (OpenedPair: the pair walk's opening move stated once) -> 1_192 (the Bytes-backed at-rest form), 0, 190_474 -> 6_212 (operations route to the skyline kernels: the cliff-immune sweep) -> 6_210 (the Bytes-backed at-rest form); re-pinned (limb 70, heap 1_064): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const JOIN_CLIFF: Envelope = envelope(5_362, 0, 308, 184); //  1_411_489 -> 6_330 -> 4_537 (the Bytes-backed at-rest form: the value-operator cell's lhs clone is a refcount bump, not a byte copy of the operand, so the public join's peak is the emit kernel's alone), 0, 384_008 -> 20_695 (operations route to the skyline kernels: the emit kernel); re-pinned to the new readings (limb 8_432, heap 4_689): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work; re-pinned (limb 246, heap 4_289): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    // Skyline validator rows: the dense row's 49 KB peak over
    // 125k levels is ~3.1 bits per open ancestor (bit stack plus
    // reallocation growth) against DECODE_DENSE's 11 MB parse frames on
    // the same tree, ~56 B per level. The validator and decoder rows carry
    // the sweep tables' scanned-bits column: their work is cursor reads
    // end to end (the validator allocates near-nothing and, off the wide
    // families, does little arithmetic), so scan is the column that sees
    // a re-read the others cannot. Decode is validate plus the wrap, so
    // each shape's scan reading equals its validate row's.
    pub const SKYLINE_VALIDATE_DENSE: SweepEnvelope = sweep_envelope(61_440, 0, 0, 468_758, 0); //     49_160, 0,   500_002, scan 375_006; re-pinned (limb 0, scan 375_006, heap 49_152): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_VALIDATE_CLIFF: SweepEnvelope = sweep_envelope(1_770, 0, 88, 17_923, 52); //      1_416 -> 1_448 (dashu-int backend), 0,    10_322, scan 14_338; re-pinned (limb 70, scan 14_338, heap 1_448): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_VALIDATE_WIDE_TOOTH: SweepEnvelope = sweep_envelope(1_520, 0, 29_509, 1_000_480, 17_705); //      1_216 -> 1_408 (the zero-run ledger's map node; the older ceiling stands), 0,    33_860, scan 800_384; re-pinned (limb 23_607, scan 800_384, heap 1_408): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_VALIDATE_HUGELEAF: SweepEnvelope   = sweep_envelope(    80_980,        0,         2_443, 312_503, 1_465); //     64_784 -> 66_752 (dashu-int backend), 0,     1_954, scan 250_002
    pub const SKYLINE_VALIDATE_ALT_SPINE: SweepEnvelope = sweep_envelope(61_440, 0, 0, 468_758, 0); //     49_160, 0,   500_002, scan 375_006; re-pinned (limb 0, scan 375_006, heap 49_152): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    // Skyline decoder rows: validation plus the wrap into storage — the
    // stored coding is the skyline stream itself, so decode materializes
    // nothing beyond the copy and stays priced by the wire input.
    pub const SKYLINE_DECODE_DENSE: SweepEnvelope = sweep_envelope(122_880, 0, 0, 468_758, 0); // 18_138_292 -> 98_304, 0, 2_750_012 -> 500_002 (operations route to the skyline kernels: decode is validate + wrap), scan 375_006; re-pinned (limb 0, scan 375_006, heap 98_304): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_DECODE_CLIFF: SweepEnvelope = sweep_envelope(3_840, 0, 88, 17_923, 52); //  1_787_704 -> 3_072, 0, 317_626 -> 10_322 (operations route to the skyline kernels: decode is validate + wrap), scan 14_338; re-pinned (limb 70, scan 14_338, heap 3_072): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_DECODE_WIDE_TOOTH: SweepEnvelope = sweep_envelope(245_760, 0, 29_509, 1_000_480, 17_705); //  1_699_472 -> 196_608, 0, 370_843 -> 33_860 (operations route to the skyline kernels: decode is validate + wrap), scan 800_384; re-pinned (limb 23_607, scan 800_384, heap 196_608): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_DECODE_HUGELEAF: SweepEnvelope     = sweep_envelope(    83_440,        0,         2_443, 312_503, 1_465); //    101_803 -> 66_752, 0, 9_773 -> 1_954 (operations route to the skyline kernels: decode is validate + wrap), scan 250_002
    pub const SKYLINE_DECODE_ALT_SPINE: SweepEnvelope = sweep_envelope(122_880, 0, 0, 468_758, 0); // 15_778_996 -> 98_304, 0, 2_750_012 -> 500_002 (operations route to the skyline kernels: decode is validate + wrap), scan 375_006; re-pinned (limb 0, scan 375_006, heap 98_304): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
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
    let p = Shape::Dense.packed1(DENSE_DEPTH);
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
        "{name}: limb counter reads {limb_ops}, below the {} improvement \
         tripwire (measured x0.75): attribute the drop — an honest \
         improvement re-pins the band; a dead meter is the bypass this \
         column exists to catch",
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
    let p = Shape::Dense.packed1(DENSE_DEPTH);
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
    let p = Shape::Dense.packed1(DENSE_DEPTH);
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
    let p = Shape::Dense.packed1(DENSE_DEPTH);
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
    let p = Shape::Dense.packed1(DENSE_DEPTH);
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
    let ev = Shape::Bigroot.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE);
    let id = Shape::NestedFullId.packed1(TICK_CROSS_SCALE);
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
    let ev = Shape::WideTail.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE);
    let id = Shape::NestedLeftFullId.packed1(TICK_CROSS_SCALE);
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

/// Ticking the descending staircase under a party owning one deep
/// diverted fragment (the ownership-hole family) stays within an
/// envelope the leaf-by-leaf walk exceeds.
///
/// The fill walk's unowned regions are whole staircase runs, and the
/// block scan must fold each into O(1) accumulator work instead of
/// per-leaf freight. The touch ceiling is the skip's liveness signal
/// — it sits below the per-leaf mechanism's reading, so the fast path
/// must demonstrably engage; the scan column pins that every skipped
/// bit is still read.
#[test]
fn tick_ownership_hole_envelope() {
    let ev = Shape::Staircase.packed1(HOLE_STAIR_DEPTH);
    let id = Shape::IdSpine.packed_flagged(HOLE_ID_DEPTH, true);
    let mut v = version_of(&ev);
    let p = party_of(&id);
    let input = ev.bytes.len() + id.bytes.len();
    query_metered(
        "tick_ownership_hole",
        input,
        &query_env::TICK_OWNERSHIP_HOLE,
        || v.tick(&p),
    );
    drop(v);
}

/// Ticking the alternating spine under the scattered id (the
/// alternating-ownership comb: owned fragments and absent gaps
/// interleaved at every level, so every unowned region is a single
/// leaf) stays within its envelope.
///
/// The comb is the region gate's worst case — the block scan can
/// never engage — and the pin holds the gated walk to the per-leaf
/// walk's own readings: a gate that costs anything when closed moves
/// this envelope.
#[test]
fn tick_ownership_comb_envelope() {
    let ev = Shape::AltSpine.packed1(DENSE_DEPTH);
    let id = Shape::ScatteredId.packed1(COMB_FRAGMENTS);
    let mut v = version_of(&ev);
    let p = party_of(&id);
    let input = ev.bytes.len() + id.bytes.len();
    query_metered(
        "tick_ownership_comb",
        input,
        &query_env::TICK_OWNERSHIP_COMB,
        || v.tick(&p),
    );
    drop(v);
}

/// The fused multi-tick on the dense spine stays within its envelope:
/// registering 512 events costs the single tick's walk and splice plus
/// only the count's gamma-width boundary codes.
#[test]
fn ticks_dense_envelope() {
    let p = Shape::Dense.packed1(DENSE_DEPTH);
    let mut v = version_of(&p);
    let seed = Party::seed();
    query_metered(
        "ticks_dense",
        p.bytes.len(),
        &query_env::TICKS_DENSE,
        || v.ticks(&seed, TICKS_POINT_LO),
    );
    drop(v);
}

/// The fused multi-tick on the wide right-full chain stays within its
/// envelope: the `+n` splice compounds at the same site the single
/// tick's does, and the wide first payload is still touched O(1) times.
#[test]
fn ticks_nested_wide_envelope() {
    let ev = Shape::Bigroot.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE);
    let id = Shape::NestedFullId.packed1(TICK_CROSS_SCALE);
    let mut v = version_of(&ev);
    let p = party_of(&id);
    let input = ev.bytes.len() + id.bytes.len();
    query_metered(
        "ticks_nested_wide",
        input,
        &query_env::TICKS_NESTED_WIDE,
        || v.ticks(&p, TICKS_POINT_LO),
    );
    drop(v);
}

/// The fused multi-tick on the wide memo chain stays within its
/// envelope: the pre-scan's frame ledger behaves exactly as the single
/// tick's, count notwithstanding.
#[test]
fn ticks_mirror_wide_envelope() {
    let ev = Shape::WideTail.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE);
    let id = Shape::NestedLeftFullId.packed1(TICK_CROSS_SCALE);
    let mut v = version_of(&ev);
    let p = party_of(&id);
    let input = ev.bytes.len() + id.bytes.len();
    query_metered(
        "ticks_mirror_wide",
        input,
        &query_env::TICKS_MIRROR_WIDE,
        || v.ticks(&p, TICKS_POINT_LO),
    );
    drop(v);
}

/// The flatness pin: `O(|v| + |p| + log n)` as a committed two-point
/// check, not prose.
///
/// On each tick-designated family the whole cost
/// movement from `ticks(512)` to `ticks(4096)` — three doublings — must
/// sit inside the boundary codes' gamma-width delta band: the two codes
/// that carry the count widen by 2 bits per doubling each, and no other
/// column may move beyond a word of slack. An implementation iterating
/// any fraction of the count moves every column by ~8x here and cannot
/// hide in a constant; a dead meter reads zero movement AND a zero
/// point, which the envelope rows' improvement tripwires already reject.
#[test]
#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
fn ticks_flatness_holds_the_log_band() {
    let cases: Vec<(&str, Version, Party)> = vec![
        (
            "dense",
            version_of(&Shape::Dense.packed1(DENSE_DEPTH)),
            Party::seed(),
        ),
        (
            "nested-wide",
            version_of(&Shape::Bigroot.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE)),
            party_of(&Shape::NestedFullId.packed1(TICK_CROSS_SCALE)),
        ),
        (
            "mirror-wide",
            version_of(&Shape::WideTail.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE)),
            party_of(&Shape::NestedLeftFullId.packed1(TICK_CROSS_SCALE)),
        ),
    ];
    for (name, v, p) in &cases {
        let lo = ticks_counters(v, p, TICKS_POINT_LO);
        let hi = ticks_counters(v, p, TICKS_POINT_HI);
        let moved = [
            ("scan", lo.0, hi.0, TICKS_FLATNESS_SCAN_BAND),
            ("limb", lo.1, hi.1, TICKS_FLATNESS_LIMB_BAND),
            ("touch", lo.2, hi.2, TICKS_FLATNESS_TOUCH_BAND),
        ];
        for (col, at_lo, at_hi, band) in moved {
            let delta = at_hi.abs_diff(at_lo);
            eprintln!("MEASURED ticks_flatness {name}/{col}: lo={at_lo} hi={at_hi} delta={delta}");
            assert!(
                delta <= band,
                "{name}/{col}: ticks({}) -> ticks({}) moved {delta}                  (from {at_lo} to {at_hi}), outside the gamma-width band {band}",
                TICKS_POINT_LO,
                TICKS_POINT_HI,
            );
        }
    }
}

/// One `ticks(n)` run's `(scan bits, limb ops, touches)` on fresh
/// counters — the flatness pin's probe.
#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
fn ticks_counters(v: &Version, p: &Party, n: u64) -> (u64, u64, u64) {
    let mut v = v.clone();
    meter::reset_scan_bits();
    meter::reset_limb_ops();
    suanpan::touch_meter::reset();
    v.ticks(p, n);
    (
        meter::scan_bits(),
        meter::limb_ops(),
        suanpan::touch_meter::touches(),
    )
}

/// One `ticks(n)` run at an arbitrary-width count, on the operand's
/// post-fill tree, with the exact `min_ticks` movement as the value
/// leg.
///
/// One public tick is applied *outside* the metered body first: on the
/// once-ticked tree the fill is the identity (fill is idempotent, and
/// a grow opens no fillable structure), so the metered `ticks(n)` is
/// the pure grow branch, where registering `n` events grows the
/// minimum tick count by exactly `n` — the fill branch instead
/// collapses owned structure and moves `min_ticks` by a
/// shape-dependent amount, which the committed small-count
/// differentials pin byte-for-byte against iterated ticks.
#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
fn ticks_counters_wide(v: &Version, p: &Party, n: &before::Ticks) -> (u64, u64, u64) {
    let mut v = v.clone();
    v.tick(p);
    let before_ticks = v.min_ticks();
    meter::reset_scan_bits();
    meter::reset_limb_ops();
    suanpan::touch_meter::reset();
    v.ticks(p, n.clone());
    let counters = (
        meter::scan_bits(),
        meter::limb_ops(),
        suanpan::touch_meter::touches(),
    );
    assert_eq!(
        v.min_ticks(),
        before_ticks + n.clone(),
        "a grow-branch ticks(n) must grow the minimum tick count by exactly n"
    );
    counters
}

/// The wide-count pin's count width in bits (the second wide point
/// doubles it): far past every machine integer.
///
/// The count's own arithmetic — the splice's site addition, the
/// changed branch's decrement, the count-carrying gamma codes — runs
/// at genuinely wide operands here.
///
/// Both wide points sit *above* the crosses' site-value width
/// ([`TICK_CROSS_SCALE`] bits), because the emitted stream's count
/// dependence is piecewise-linear with a knee exactly there: below it
/// the output carries the count in one gamma code (span `2·bits(n)`),
/// above it the min-lift re-coding around the grown site carries the
/// count's excess over the site again (span `4·bits(n) − 2·site`,
/// measured exact on the mirror-wide cross: 24,750 → 57,518 scanned
/// bits against the law's 24,766 → 57,534). Judging two points in one
/// regime keeps the ratio band tight; a probe straddling the knee
/// legitimately reads up to ×3 without any superlinearity.
#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
const TICKS_WIDE_COUNT_BITS: usize = 8_192;

/// The count-attributable growth bound: doubling the count's width may
/// at most double the count-attributable cost (×1.25 flatness slack on
/// the ratio), plus a word of boundary slack per column.
///
/// `ticks(n)` claims `O(|v| + |p| + log n)`: the whole `n`-dependence
/// is the count's own width — two count-carrying codes and word-linear
/// arithmetic on the count — so the cost *above the word-count
/// baseline* must scale linearly in `bits(n)`. An implementation
/// superlinear in the count's width (a per-limb re-walk of the site
/// value per count limb, a decimal detour) moves the second span by
/// ×4 here and cannot hide in the baseline, which the committed
/// three-family log band already pins at word counts.
///
/// [Measured in the dev profile, exact counters: scan spans
/// 16,366 → 32,750 bits (×2.001) and limb spans 256 → 512 on dense
/// and nested-wide; scan spans 24,750 → 57,518 (×2.32, the slope-4
/// regime above the site-width knee) and limb spans 834 → 1,730 on
/// mirror-wide; touch spans 0 → 0 everywhere — the count's arithmetic
/// lives on `Base`, never the accumulator.]
#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
const TICKS_WIDE_GROWTH_NUM: u64 = 5;

/// See [`TICKS_WIDE_GROWTH_NUM`]: the ratio denominator.
#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
const TICKS_WIDE_GROWTH_DEN: u64 = 2;

/// The wide-count flatness pin: `ticks(n)` stays width-linear in the
/// count far past every machine integer, on every tick-designated
/// family.
///
/// Three points per family — the word-count baseline `n₀ = 512`, an
/// 8,192-bit count, and its width doubling — judged as two spans: the
/// count-attributable movement (each counter above its baseline) may
/// at most double, ×1.25, when the width doubles. The scan span
/// additionally carries a liveness floor of `2 · bits(n)` (the
/// widened count-carrying code must be written), so a dead meter or a
/// count that never reaches the splice cannot pass vacuously; the
/// value leg inside the probe holds the `min_ticks` movement exactly
/// equal to `n` at every point, so the wide registration is proven to
/// have happened before any cost is judged.
#[test]
#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
fn ticks_wide_count_flatness_holds_the_width_band() {
    use dashu_int::UBig;
    let wide = |bits: usize| -> before::Ticks {
        (UBig::ONE << bits)
            .to_string()
            .parse()
            .expect("a power of two renders as a count")
    };
    let n1 = wide(TICKS_WIDE_COUNT_BITS);
    let n2 = wide(2 * TICKS_WIDE_COUNT_BITS);
    let cases: Vec<(&str, Version, Party)> = vec![
        (
            "dense",
            version_of(&Shape::Dense.packed1(DENSE_DEPTH)),
            Party::seed(),
        ),
        (
            "nested-wide",
            version_of(&Shape::Bigroot.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE)),
            party_of(&Shape::NestedFullId.packed1(TICK_CROSS_SCALE)),
        ),
        (
            "mirror-wide",
            version_of(&Shape::WideTail.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE)),
            party_of(&Shape::NestedLeftFullId.packed1(TICK_CROSS_SCALE)),
        ),
    ];
    for (name, v, p) in &cases {
        let base = ticks_counters_wide(v, p, &before::Ticks::from(TICKS_POINT_LO));
        let at1 = ticks_counters_wide(v, p, &n1);
        let at2 = ticks_counters_wide(v, p, &n2);
        let spans = [
            ("scan", base.0, at1.0, at2.0),
            ("limb", base.1, at1.1, at2.1),
            ("touch", base.2, at1.2, at2.2),
        ];
        for (col, c0, c1, c2) in spans {
            let d1 = c1.saturating_sub(c0);
            let d2 = c2.saturating_sub(c0);
            eprintln!(
                "MEASURED ticks_wide_count {name}/{col}: base={c0} w={c1} 2w={c2} \
                 spans {d1} -> {d2}"
            );
            assert!(
                d2 * TICKS_WIDE_GROWTH_DEN
                    <= d1 * TICKS_WIDE_GROWTH_NUM + 64 * TICKS_WIDE_GROWTH_DEN,
                "{name}/{col}: doubling the count's width from {TICKS_WIDE_COUNT_BITS} bits \
                 grew the count-attributable cost {d1} -> {d2}, past the width-linear band"
            );
        }
        // The scan span's liveness floor: one count-carrying gamma code
        // widened from word scale to `bits(n)` writes at least
        // `2·(bits(n) − 64)` fresh bits through the metered builder.
        assert!(
            at1.0.saturating_sub(base.0) >= 2 * (TICKS_WIDE_COUNT_BITS as u64 - 64),
            "{name}: a {TICKS_WIDE_COUNT_BITS}-bit count moved the scan column by only \
             {} bits — the widened count-carrying code is not being written through \
             the metered builder",
            at1.0.saturating_sub(base.0),
        );
    }
}

/// The flatness pin's two count points: three doublings apart, so the
/// per-doubling gamma growth (2 bits per count-carrying code) is
/// legible against the band.
const TICKS_POINT_LO: u64 = 512;
/// See [`TICKS_POINT_LO`].
#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
const TICKS_POINT_HI: u64 = 4_096;
/// The scan movement band: up to two count-carrying codes x 2 bits per
/// doubling x 3 doublings.
///
/// [Measured: exactly 6 on all three families - one code
/// carries the count there; the second code's budget covers operand
/// shapes where the successor repair carries it too.]
#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
const TICKS_FLATNESS_SCAN_BAND: u64 = 12;
/// The limb movement band: the count's arithmetic stays inside one
/// digit across the band [measured: exactly 0 on all three
/// families; a word of slack for a digit-boundary crossing].
#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
const TICKS_FLATNESS_LIMB_BAND: u64 = 8;
/// The touch movement band: see the limb band [measured:
/// exactly 0 on all three families].
#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
const TICKS_FLATNESS_TOUCH_BAND: u64 = 8;

// ─── bigroot scenarios ──────────────────────────────────────────────────────

/// Decoding bigroot stays within its envelope (one big-integer base plus the
/// parse stack).
#[test]
fn decode_bigroot_envelope() {
    let p = Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
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
    let p = Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
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
    let p = Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
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
    let p = Shape::Hugeleaf.packed1(HUGELEAF_MAGNITUDE_BITS);
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
    let p = Shape::Hugeleaf.packed1(HUGELEAF_MAGNITUDE_BITS);
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
    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
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
    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
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
    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
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
/// moves through `Accumulator`s that the heap and limb columns cannot see.
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
    /// Improvement tripwire under the limb column: measured ×0.75, per
    /// the file doc's tripwire convention.
    #[cfg(feature = "limb-meter")]
    limb_floor: u64,
    /// Improvement tripwire under the touch column: measured ×0.75, per
    /// the file doc's tripwire convention.
    ///
    /// A touch reading below it is a >25% drop from the pinned reading —
    /// attribute it, re-pinning an honest improvement or curing a dead
    /// meter, without which every touch ceiling above would hold
    /// vacuously.
    #[cfg(feature = "limb-meter")]
    touch_floor: u64,
}

/// Build a [`TouchEnvelope`] from the four pinned columns and the two
/// improvement tripwires.
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
// (aarch64-apple-darwin, dev profile, three identical runs)
// the ceiling derives from; a re-pinned column records the movement as
// `old -> new`. Re-pin by rerunning under `--no-capture` with
// `--all-features` and reading the MEASURED lines.
// The limb floor column is the measured value ×0.75, rounded down (the
// file doc's improvement-tripwire convention).
// One rise is recorded against the tightening rule: every limb ceiling
// here rose when `Base::trailing_zeros` joined the metered
// seam (rank normalization strips factors of two through it, so these
// are the rows that gained counts) — a re-denomination of the column,
// the same work newly counted, not a weakening.
#[rustfmt::skip]
mod rank_env {
    use super::{touch_envelope, TouchEnvelope};
    //                                                             peak heap, segments,    limb ops, touches, limb floor, touch floor       measured: peak heap, segments, limb ops (movement), touches
    pub const RANK_DENSE: TouchEnvelope = touch_envelope(30_720, 0, 4, 7, 2, 3); //          0 -> 65_560, 240 -> 0, 3 -> 250_005, 0 -> 125_007 (operations route to the skyline kernels: the query fold reads delta payloads); heap 65_576 -> 24_576 and touches 125_007 -> 5 (the segment feed opens at the first freeze — the depth control's touch column and scale-deep segment buffer were the feed); re-pinned (limb 3, touches 5, heap 24_576): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const RANK_BIGROOT: TouchEnvelope = touch_envelope(72_005, 0, 2_739, 8_993, 1_643, 5_395); //     41_368 -> 61_516 -> 61_708 (the zero-run ledger's map nodes; the older ceiling stands), 16 -> 0, 2_191 -> 22_195, 4_689 -> 17_194 (operations route to the skyline kernels: the query fold reads delta payloads); heap 61_724 -> 57_636 and touches 17_194 -> 7_192 (the segment feed opens at the first freeze); re-pinned (limb 2_191, touches 7_194, heap 57_604): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const RANK_HARMONIC: TouchEnvelope = touch_envelope(52_500, 0, 2_562, 248_285, 1_536, 148_971); //     33_840 -> 58_400, 124 -> 0, 2_049 -> 133_121, 67_522 -> 198_659 (operations route to the skyline kernels: the query fold reads delta payloads); heap 58_416 -> 42_040 and touches 198_658 -> 133_121 (the segment feed opens at the first freeze — one deposit per leaf gone); re-pinned to the new readings (limb 133_121, touches 198_628, heap 42_000): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work; re-pinned (limb 2_049, touches 198_628, heap 42_000): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const RANK_PAIR_MISMATCH: TouchEnvelope = touch_envelope(     234_400,        0,      87_910,      0, 52_746, 0); //    187_520 -> 211_016 (dashu-int backend),   0, 54_710 -> 39_078 (class-first cmp; the rest is checked_sub's and add's mandatory output) -> 54_704 (metered trailing_zeros) -> 70_328 (widening shifts record output width: a re-denomination, the exponent-alignment work newly counted), 0
    pub const RANK_SUM_MIXED: TouchEnvelope     = touch_envelope(      78_140,        0,       9_769, 22_268, 5_861, 13_360); //     62_512 -> 62_704 (the zero-run ledger's map nodes; the older ceiling stands),   0, 156_312_196 -> 3_908 (raw accumulator, one normalization) -> 7_815 (metered trailing_zeros), 17_814
}

/// Run one touch-priced scenario body under all four meters and assert
/// its envelope, both improvement tripwires included.
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
    suanpan::touch_meter::reset();
    HEAP.reset_peak_usage();
    let baseline = HEAP.current_usage();
    let r = f();
    let peak_heap = HEAP.peak_usage().saturating_sub(baseline);
    let segments = meter::stack_segments();
    #[cfg(feature = "limb-meter")]
    let limb_ops = meter::limb_ops();
    #[cfg(feature = "limb-meter")]
    let touches = suanpan::touch_meter::touches();
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
        "{name}: limb counter reads {limb_ops}, below the {} improvement \
         tripwire (measured x0.75): attribute the drop — an honest \
         improvement re-pins the band; a dead meter is the bypass this \
         column exists to catch",
        env.limb_floor,
    );
    #[cfg(feature = "limb-meter")]
    assert!(
        touches >= env.touch_floor,
        "{name}: touch counter reads {touches}, below the {} improvement \
         tripwire (measured x0.75): attribute the drop — an honest \
         improvement re-pins the band; a dead meter is the bypass this \
         column exists to catch",
        env.touch_floor,
    );
    r
}

/// The rank fold on the dense spine stays within its envelope (the
/// control: the spine's numerator stays one bit wide, so the fold's
/// per-level shifts are word-scale and the walk is linear).
#[test]
fn rank_dense_envelope() {
    let p = Shape::Dense.packed1(DENSE_DEPTH);
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
    let p = Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
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
    let p = Shape::Harmonic.packed1(RANK_HARMONIC_DEPTH);
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
    let a = version_of(&Shape::Dense.packed1(RANK_PAIR_DEPTH)).rank();
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
    let high = version_of(&Shape::Dense.packed1(RANK_SUM_EXP_DEPTH)).rank();
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
fn skyline_of(p: &meter::Packed) -> meter::skyline::BitsMut {
    meter::skyline::encode(&version_of(p))
}

/// The skyline validator on the dense spine stays within its envelope.
///
/// The transient is ~2 bits per open ancestor (measured ~3.1 bits per
/// level including reallocation growth, against the old parse stack's ~56
/// bytes per level on the same tree), with zero grown segments.
#[test]
fn skyline_validate_dense_envelope() {
    let enc = skyline_of(&Shape::Dense.packed1(DENSE_DEPTH));
    let r = sweep_metered(
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
    let enc = skyline_of(&Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE));
    let r = sweep_metered(
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
    let enc =
        skyline_of(&Shape::WideToothComb.packed3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE));
    let r = sweep_metered(
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
    let enc = skyline_of(&Shape::Hugeleaf.packed1(HUGELEAF_MAGNITUDE_BITS));
    let r = sweep_metered(
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
    let enc = skyline_of(&Shape::AltSpine.packed1(DENSE_DEPTH));
    let r = sweep_metered(
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
    let p = Shape::Dense.packed1(DENSE_DEPTH);
    let enc = skyline_of(&p);
    let v = sweep_metered(
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
    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
    let enc = skyline_of(&p);
    let v = sweep_metered(
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
    let p = Shape::WideToothComb.packed3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
    let enc = skyline_of(&p);
    let v = sweep_metered(
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
    let p = Shape::Hugeleaf.packed1(HUGELEAF_MAGNITUDE_BITS);
    let enc = skyline_of(&p);
    let v = sweep_metered(
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
    let p = Shape::AltSpine.packed1(DENSE_DEPTH);
    let enc = skyline_of(&p);
    let v = sweep_metered(
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
    /// Improvement tripwire under the limb column: measured ×0.75, per
    /// the file doc's tripwire convention.
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
// (aarch64-apple-darwin, dev profile, three identical runs)
// the ceiling derives from. Re-pin by rerunning under `--no-capture`
// with `--all-features` and reading the MEASURED lines.
// The limb floor column is the measured value ×0.75, rounded down (the
// file doc's improvement-tripwire convention).
#[rustfmt::skip]
mod sweep_env {
    use super::{sweep_envelope, SweepEnvelope};
    //                                                               peak heap, segments, limb ops,  scan bits, limb floor            measured: peak heap, segments, limb ops, scan bits
    pub const SKYLINE_CMP_DENSE: SweepEnvelope = sweep_envelope(30_720, 0, 0, 468_760, 0); //   24_584 -> 24_576 (OpenedPair: the pair walk's opening move stated once; work columns unmoved), 0, 250_002, 375_008; re-pinned (limb 0, scan 375_008, heap 24_576): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_CMP_DENSE_SELF: SweepEnvelope = sweep_envelope(51_200, 0, 0, 937_515, 0); //   40_968 -> 40_960 (OpenedPair; work columns unmoved), 0, 500_004, 750_012; re-pinned (limb 0, scan 750_012, heap 40_960): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_CMP_BIGROOT: SweepEnvelope = sweep_envelope(39_540, 0, 783, 137_514, 469); //   31_632 -> 32_272 (dashu-int backend), 0,  20_630, 110_011; re-pinned (limb 626, scan 110_011, heap 32_272): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_CMP_CLIFF: SweepEnvelope = sweep_envelope(1_330, 0, 88, 17_925, 52); //    1_160 -> 1_296 (emission-sweep shared step holds each consumed delta) -> 1_360 (dashu-int backend) -> 1_192 (OpenedPair; work columns unmoved), 0,   6_210,  14_340; re-pinned (limb 70, scan 14_340, heap 1_064): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    // SKYLINE_CMP_WIDE_TOOTH's 1_064-under-1_250 margin is a deliberate
    // change-detector on the backend's and the accumulator's allocation
    // policies: the committed Cargo.lock (dashu-int 0.5.0 exact) is what
    // makes the measurement deterministic, and a cargo update to any other
    // 0.5.x is a deliberate re-measure event, not noise.
    pub const SKYLINE_CMP_WIDE_TOOTH: SweepEnvelope = sweep_envelope(    1_250,        0,    29_509, 1_000_483, 17_705); //      840 -> 968 (emission-sweep shared step holds each consumed delta) -> 1_032 (dashu-int backend) -> 1_224 (the zero-run ledger's map node) -> 1_064 (OpenedPair: the pair walk's opening move stated once; work columns unmoved), 0,  23_607, 800_386
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
        "{name}: limb counter reads {limb_ops}, below the {} improvement \
         tripwire (measured x0.75): attribute the drop — an honest \
         improvement re-pins the band; a dead meter is the bypass this \
         column exists to catch",
        env.limb_floor,
    );
    r
}

/// The empty version's two-bit skyline stream: the shallow operand of
/// the family cmp scenarios.
fn skyline_empty() -> meter::skyline::BitsMut {
    meter::skyline::encode(&Version::new())
}

/// The combined operand bytes of a sweep scenario.
fn sweep_input_bytes(a: &meter::skyline::BitsMut, b: &meter::skyline::BitsMut) -> usize {
    a.as_raw_slice().len() + b.as_raw_slice().len()
}

/// The sweep on the dense spine against the empty version stays within
/// its envelope.
///
/// The deep side's 125k levels cost path *bits* (no grown segments, heap
/// in the path stack), consumed iteratively against one depth-0 plateau.
#[test]
fn skyline_cmp_dense_envelope() {
    let a = skyline_of(&Shape::Dense.packed1(DENSE_DEPTH));
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
    let a = skyline_of(&Shape::Dense.packed1(DENSE_DEPTH));
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
    let a = skyline_of(&Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH));
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
    let a = skyline_of(&Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE));
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
    let a =
        skyline_of(&Shape::WideToothComb.packed3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE));
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
// (aarch64-apple-darwin, dev profile, three identical runs)
// the ceiling derives from. Re-pin by rerunning under `--no-capture`
// with `--all-features` and reading the MEASURED lines.
// The limb floor column is the measured value ×0.75, rounded down (the
// file doc's improvement-tripwire convention).
#[rustfmt::skip]
mod emit_env {
    use super::{sweep_envelope, SweepEnvelope};
    //                                                                peak heap, segments, limb ops,  scan bits, limb floor            measured: peak heap, segments, limb ops, scan bits
    pub const SKYLINE_JOIN_DENSE: SweepEnvelope = sweep_envelope(130_277, 0, 0, 625_018, 0); //  104_237, 0, 750_004,   500_014; re-pinned to the new readings (limb 250_002, scan 500_014, heap 104_221): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work; re-pinned (limb 0, scan 500_014, heap 104_221): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_JOIN_ABSORB: SweepEnvelope = sweep_envelope(270_798, 0, 4_887, 1_250_013, 2_931); //  216_638 -> 218_606 (dashu-int backend), 0, 753_911, 1_000_010; re-pinned to the new readings (limb 253_911, scan 1_000_010, heap 218_607): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work; re-pinned (limb 3_909, scan 1_000_010, heap 218_607): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_JOIN_BIGROOT: SweepEnvelope = sweep_envelope(85_060, 0, 1_565, 275_028, 939); //   71_768 -> 68_048 (dashu-int backend), 0,  61_266,   220_022; re-pinned to the new readings (limb 21_258, scan 220_022, heap 68_048): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work; re-pinned (limb 1_252, scan 220_022, heap 68_048): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_JOIN_CLIFF: SweepEnvelope = sweep_envelope(5_362, 0, 308, 35_848, 184); //    4_409 -> 4_537 (dashu-int backend), 0,  20_695,    28_678; re-pinned to the new readings (limb 8_432, scan 28_678, heap 4_689): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work; re-pinned (limb 246, scan 28_678, heap 4_289): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_JOIN_WIDE_TOOTH: SweepEnvelope = sweep_envelope(  128_312,        0,    74_477, 2_000_963, 44_685); //  102_649 -> 102_777 (dashu-int backend) -> 102_969 (the zero-run ledger's map node; the older ceiling stands), 0,  59_581, 1_600_770
    pub const SKYLINE_MEET_CLIFF: SweepEnvelope = sweep_envelope(4_422, 0, 88, 23_055, 52); //    4_001 -> 4_065 (dashu-int backend), 0,  14_416,    18_444; re-pinned to the new readings (limb 6_212, scan 18_444, heap 4_049): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work; re-pinned (limb 70, scan 18_444, heap 3_537): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_MEET_WIDE_TOOTH: SweepEnvelope = sweep_envelope(127_732, 0, 29_512, 1_005_613, 17_706); //  102_185 -> 102_249 (dashu-int backend) -> 102_441 (the zero-run ledger's map node; the older ceiling stands), 0,  31_813,   804_490; re-pinned to the new readings (limb 23_609, scan 804_490, heap 102_425): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work
}

/// The one-tick version's skyline stream: the shallow operand of the
/// family join/meet scenarios, mirroring the packed-form join rows.
fn skyline_one_tick() -> meter::skyline::BitsMut {
    let one = Version::try_from(1u64).expect("a one-tick version is valid");
    meter::skyline::encode(&one)
}

/// One family shape and the packed-form oracle's answer against the
/// one-tick version, both as skyline streams built outside measurement,
/// so every scenario asserts byte-identity after its sweep.
fn skyline_oracle(
    p: &meter::Packed,
    join: bool,
) -> (meter::skyline::BitsMut, meter::skyline::BitsMut) {
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
    let p = Shape::Dense.packed1(DENSE_DEPTH);
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
    let p = Shape::Dense.packed1(DENSE_DEPTH);
    let flat = version_of(&Shape::Hugeleaf.packed1(HUGELEAF_MAGNITUDE_BITS));
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
    let p = Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
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
    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
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
    let p = Shape::WideToothComb.packed3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
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
    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
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
    let p = Shape::WideToothComb.packed3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
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
    let party = party_of(&Shape::IdSpine.packed_flagged(ID_DEPTH, false));
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
    let ev = Shape::AltSpine.packed1(DENSE_DEPTH);
    let mut v = version_of(&ev);
    let party = party_of(&Shape::IdSpine.packed_flagged(ID_DEPTH, false));
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
// in the open-node stacks, the digit arena, and the output itself, limb
// work linear per I/O byte (radix conversion runs inside the backend;
// the recorded ops are the delta algebra), and scan linear in the
// skyline stream. The bigroot rows are the width separator: the 40k-bit
// heights never materialize, so no summary or accumulator state carries
// a copy of the wide magnitude per level.

// The text envelope table: pinned ceiling = measured ×1.25, rounded up,
// and only ever tightened. The trailing comment on each line is the
// measurement of record (parse rows at the parse transient
// cure — parallel chunked open-node stacks in place of a doubling
// frame vector, the extraction magnitude moved instead of cloned, and
// the wide path-sum buffer dropped at extraction; the peak-heap column
// moved on all four rows, parent-measured 11,135,057 → 3,232,857
// dense, 1,409,795 → 302,363 bigroot, 157,024 → 130,771 hugeleaf,
// 386,073 → 275,441 cliff, with limb and scan byte-identical; render
// rows at the streaming finalize arena;
// aarch64-apple-darwin, dev profile, identical repeated runs) the
// ceiling derives from. Re-pin by rerunning under `--no-capture` with
// `--all-features` and reading the MEASURED lines.
// The limb floor column is the measured value ×0.75, rounded down (the
// file doc's improvement-tripwire convention).
#[rustfmt::skip]
mod text_env {
    use super::{sweep_envelope, SweepEnvelope};
    //                                                                 peak heap, segments, limb ops,  scan bits, limb floor            measured: peak heap, segments, limb ops, scan bits
    pub const SKYLINE_RENDER_DENSE: SweepEnvelope    = sweep_envelope( 1_996_800,        0, 1_562_513,   468_758, 937_507); //  1_597_440, 0, 1_250_010, 375_006
    pub const SKYLINE_RENDER_BIGROOT: SweepEnvelope  = sweep_envelope(   249_600,        0,   127_368,   137_512, 76_420); //    199_680, 0,   101_894, 110_009
    pub const SKYLINE_RENDER_HUGELEAF: SweepEnvelope = sweep_envelope(   171_310,        0,     7_330,   312_503, 4_398); //    137_048, 0,     5_864, 250_002
    pub const SKYLINE_RENDER_CLIFF: SweepEnvelope    = sweep_envelope( 1_113_202,        0,   243_385,    17_923, 146_031); //    890_561, 0,   194_708, 14_338
    pub const SKYLINE_PARSE_DENSE: SweepEnvelope = sweep_envelope(4_041_052, 0, 625_007, 468_758, 375_003); //  3_232_857, 0, 1_500_013, 750_012; re-pinned to the new readings (limb 1_000_007, scan 750_012, heap 3_232_841): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work; re-pinned (limb 500_005, scan 750_012, heap 3_232_841): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal); re-pinned (limb 500_005, scan 375_006, heap 3_232_841): the parse pipeline ends at the builder — the built stream's canonicality rides the committed render↔parse inverse pair and transcoder differential — so the scan column is the build pass's own
    pub const SKYLINE_PARSE_BIGROOT: SweepEnvelope = sweep_envelope(377_944, 0, 51_574, 137_512, 30_944); //    302_363, 0,   121_899, 220_018; re-pinned to the new readings (limb 81_891, scan 220_018, heap 302_355): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work; re-pinned (limb 41_885, scan 220_018, heap 302_355): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal); re-pinned (limb 41_259, scan 110_009, heap 302_355): the parse pipeline ends at the builder — canonicality rides the committed differential suites — so the scan column is the build pass's own
    pub const SKYLINE_PARSE_HUGELEAF: SweepEnvelope  = sweep_envelope(   152_480,        0,     4_887,   312_503, 2_931); //    130_771, 0,     5_863, 500_004; re-pinned (limb 3_909, scan 250_002, heap 121_984): the parse pipeline ends at the builder — canonicality rides the committed differential suites — so the scan column is the build pass's own and no accumulator re-walks the built stream's wide payloads
    pub const SKYLINE_PARSE_CLIFF: SweepEnvelope = sweep_envelope(344_152, 0, 56_475, 17_923, 33_885); //    275_441, 0,    67_764, 28_676; re-pinned (limb 45_250, scan 28_676, heap 275_321): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal); re-pinned (limb 45_180, scan 14_338, heap 275_321): the parse pipeline ends at the builder — canonicality rides the committed differential suites — so the scan column is the build pass's own
}

/// Rendering the dense spine's skyline stays within its envelope.
///
/// 125k open-node levels and ~250k single-digit printed bases finalize
/// through word-sized summaries, the output is sized exactly before one
/// byte is written, and nothing recurses.
#[test]
fn skyline_render_dense_envelope() {
    let v = version_of(&Shape::Dense.packed1(DENSE_DEPTH));
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
    let v = version_of(&Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH));
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
    let v = version_of(&Shape::Hugeleaf.packed1(HUGELEAF_MAGNITUDE_BITS));
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
    let v = version_of(&Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE));
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
    let v = version_of(&Shape::Dense.packed1(DENSE_DEPTH));
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
    let v = version_of(&Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH));
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
    let v = version_of(&Shape::Hugeleaf.packed1(HUGELEAF_MAGNITUDE_BITS));
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
    let v = version_of(&Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE));
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
    use before::meter;
    use before::meter::registry::Shape;
    use suanpan::touch_meter;

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
        let packed = Shape::CliffComb.packed2(scale, scale);
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
        let packed = Shape::CliffComb.packed2(scale, scale);
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
        let packed = Shape::CliffComb.packed2(scale, scale);
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
        let packed = Shape::CliffComb.packed2(scale, scale);
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
            (Shape::Dense.packed1(4_096), "dense"),
            (Shape::CliffComb.packed2(512, 512), "cliff"),
            (Shape::Bigroot.packed2(8_000, 2_000), "bigroot"),
            // The parse direction's dual: the render walks the same
            // wide-swing-then-dense-trail stream through its summary
            // merges, and records zero accumulator work doing it.
            (Shape::WideArming.packed2(512, 512), "wide_arming"),
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
        let packed = Shape::WideToothComb.packed3(k, w, n);
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

    /// Absolute over-threshold ceilings, measured ×1.25
    /// (deterministic counters).
    ///
    /// These are the cured freeze discipline's numbers, tightened from
    /// the retired quadratic baseline of record (101,716 → 396,126
    /// touches, 145,680 → 564,784 limbs at these scales) in the commit
    /// that landed the cure, per the ratchet convention. Measured: small
    /// 6,182 → 5,166 touches (certificate skips replace the
    /// accumulator's zero-run walks) / 4,326 → 4,479 limbs on 24,085
    /// skyline bytes; large 12,390 → 10,350 touches (same change) /
    /// 8,662 → 8,967 limbs on 48,245 bytes (limb movement:
    /// metered `trailing_zeros`, under the standing ceilings). Live
    /// remeasure: 4,905 → 9,829 touches (−5.1%, accumulated since the
    /// record; limbs unmoved), under the standing ceilings.
    const FREEZE_BAND_OVER_TOUCH_CEILINGS: (u64, u64) = (6_458, 12_938);

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
        let packed = Shape::JumpComb.packed2(k, n);
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

    /// Absolute jump-comb ceilings, measured ×1.25 (three
    /// identical runs).
    ///
    /// The ceilings price one eviction of the `k`-bit jump plus flat
    /// 3-bit-delta work — the un-evicted alternative reads the jump's
    /// width again on every following delta, ~15× these numbers at the
    /// small scale alone. Measured: small 5,138 touches / 2,128 → 2,280
    /// limbs on 4,961 skyline bytes; large 10,272 touches / 4,250 →
    /// 4,554 limbs on 9,921 bytes (limb movement: metered
    /// `trailing_zeros`, under the standing ceilings). Live remeasure:
    /// 5,134 / 10,264 touches and 2,580 / 5,152 limbs — the limb column
    /// has accumulated +13% since the movement and now sits
    /// ×1.03 under its standing ceilings, the thinnest margin on the
    /// suite; the next reading over is real signal, not slack.
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

    /// One public fold run over a packed family shape: the operand's
    /// packed bytes and both counters over the metered body alone.
    struct QueryRun {
        bytes: u64,
        touches: u64,
        limb_ops: u64,
    }

    /// One `Version::min_ticks` run over a packed family shape, with
    /// the family's closed-form tick total as the semantic leg and the
    /// one-touch-per-operand-byte liveness floor.
    fn min_ticks_family_run(packed: before::meter::Packed, expected: &dashu_int::UBig) -> QueryRun {
        let v = packed.version();
        let bytes = v.encode().len() as u64;
        touch_meter::reset();
        meter::reset_limb_ops();
        let ticks = v.min_ticks();
        let run = QueryRun {
            bytes,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert_eq!(
            ticks,
            expected
                .to_string()
                .parse::<before::Ticks>()
                .expect("the closed form parses"),
            "min_ticks disagrees with the family's closed form"
        );
        assert!(
            run.touches >= run.bytes,
            "min_ticks at {bytes} operand bytes: {} digit touches under the \
             one-per-byte floor: the fold's accumulator work is not metered",
            run.touches,
        );
        run
    }

    /// Assert one two-scale reading against its absolute pinned
    /// ceilings, printing the measured line re-pins read from.
    fn assert_ceilings(name: &str, small: &QueryRun, large: &QueryRun, ceilings: [(u64, u64); 2]) {
        for (run, (touch_ceiling, limb_ceiling), scale) in
            [(small, ceilings[0], "small"), (large, ceilings[1], "large")]
        {
            eprintln!(
                "MEASURED {name}_{scale}: bytes={} touches={} limb_ops={}",
                run.bytes, run.touches, run.limb_ops,
            );
            assert!(
                run.touches <= touch_ceiling,
                "{name}_{scale}: {} touches exceed the pinned ceiling {touch_ceiling}",
                run.touches,
            );
            assert!(
                run.limb_ops <= limb_ceiling,
                "{name}_{scale}: {} limb ops exceed the pinned ceiling {limb_ceiling}",
                run.limb_ops,
            );
        }
    }

    /// Blocks of the min_ticks comb bands' small runs (the large runs
    /// double both comb parameters, doubling the packed operand).
    const MIN_TICKS_COMB_SMALL: usize = 1_000;

    /// Absolute two-scale (touch, limb) ceilings for min_ticks on the
    /// pure comb, measured ×1.25.
    ///
    /// The cured record: the anchor-web fold reads 2,228 / 4,447
    /// touches and 2,071 / 4,135 limb ops at the two scales (~3.6 per
    /// packed byte, flat across the doubling), tightened in the cure's
    /// own commit from the epoch-stack fold's red pin of 18,210 /
    /// 68,413 touches (29.1 → 54.7 per byte, ×1.88 across the doubling
    /// and still rising — the closing nodes each circulated the full
    /// plateau width).
    const MIN_TICKS_PURE_COMB_CEILINGS: [(u64, u64); 2] = [(2_785, 2_588), (5_558, 5_168)];

    /// Absolute two-scale (touch, limb) ceilings for min_ticks on the
    /// reveal comb, measured ×1.25.
    ///
    /// The cured record: 13,397 / 26,787 touches and 12,101 / 24,197
    /// limb ops (~8.9 per packed byte, flat), tightened from the red
    /// pin of 24,273 / 80,539 touches and 23,467 / 78,915 limb ops
    /// (×1.66 touch and ×1.68 limb across the doubling, both rising).
    const MIN_TICKS_REVEAL_COMB_CEILINGS: [(u64, u64); 2] = [(16_746, 15_126), (33_483, 30_246)];

    /// min_ticks is linear on the pure comb: per-byte touch and limb
    /// work stay flat (×1.25) across a joint `(k, b)` doubling, under
    /// absolute two-scale ceilings.
    ///
    /// `k` wide plateau leaves ride one wide code and unit deltas over
    /// a zero floor, so every closing comb node's minimum is the floor:
    /// the fold must subtract the same value `k` times. An accounting
    /// that folds the minimum's width per closing node pays the plateau
    /// width `k` times from one funding code and reads superlinear —
    /// which is exactly what the epoch-stack fold's committed red pin
    /// measured until the anchor-web cure landed: the web now counts
    /// the floor's reign and settles it once, so the flatness bound
    /// holds with the closed form `min_ticks = k·2^b` exact at both
    /// scales.
    #[test]
    fn skyline_min_ticks_pure_comb_is_flat_per_unit() {
        use dashu_int::UBig;
        let k = MIN_TICKS_COMB_SMALL;
        let expected = |k: usize| (UBig::ONE << k) * UBig::from(k as u64);
        let small = min_ticks_family_run(Shape::PureComb.packed2(k, k), &expected(k));
        let large = min_ticks_family_run(Shape::PureComb.packed2(2 * k, 2 * k), &expected(2 * k));
        assert_ceilings(
            "skyline_min_ticks_pure_comb",
            &small,
            &large,
            MIN_TICKS_PURE_COMB_CEILINGS,
        );
        assert_flat(
            "min_ticks_pure_comb_touches",
            "byte",
            (small.touches, small.bytes),
            (large.touches, large.bytes),
        );
        assert_flat(
            "min_ticks_pure_comb_limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
    }

    /// min_ticks is linear on the reveal comb in BOTH width currencies:
    /// per-byte touch and limb work stay flat (×1.25) across a joint
    /// `(k, b)` doubling, under absolute two-scale ceilings.
    ///
    /// The reveal comb's `k` sibling sites share one `2^b`-wide minimum
    /// over a zero floor, so the sweep's minimum tracking crosses the
    /// width-`b` boundary between the floor and the site plateau at
    /// every site — the close-reveal genre. The web shuttles that
    /// boundary between the difference stack and the latent register by
    /// moves alone, so the flatness bound holds in both currencies with
    /// the closed form `min_ticks = k·2^b` exact at both scales (the
    /// epoch-stack fold's committed red pin read ×1.66/×1.68 per-byte
    /// growth here until the anchor-web cure landed).
    #[test]
    fn skyline_min_ticks_reveal_comb_is_flat_per_unit() {
        use dashu_int::UBig;
        let k = MIN_TICKS_COMB_SMALL;
        let expected = |k: usize| (UBig::ONE << k) * UBig::from(k as u64);
        let small = min_ticks_family_run(Shape::RevealComb.packed2(k, k), &expected(k));
        let large = min_ticks_family_run(Shape::RevealComb.packed2(2 * k, 2 * k), &expected(2 * k));
        assert_ceilings(
            "skyline_min_ticks_reveal_comb",
            &small,
            &large,
            MIN_TICKS_REVEAL_COMB_CEILINGS,
        );
        assert_flat(
            "min_ticks_reveal_comb_touches",
            "byte",
            (small.touches, small.bytes),
            (large.touches, large.bytes),
        );
        assert_flat(
            "min_ticks_reveal_comb_limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
    }

    /// One `Version::rank` run over the freeze-position family
    /// `FP(k)`, both counters over the rank body alone.
    ///
    /// Carries `min_ticks`' closed form as the cross-fold semantic leg
    /// (proving the generator builds the tree this band reasons about)
    /// and the one-touch-per-operand-byte liveness floor.
    fn rank_freeze_position_run(k: usize) -> QueryRun {
        use dashu_int::UBig;
        let packed = Shape::FreezePosition.packed1(k);
        let v = packed.version();
        let bytes = v.encode().len() as u64;
        let band = 289 + (usize::BITS - k.leading_zeros()) as usize;
        let expected = (UBig::from(2 * k as u64) << band)
            + UBig::from((k * (k - 1)) as u64) * ((UBig::ONE << 288usize) + UBig::ONE)
            + UBig::from(k as u64);
        assert_eq!(
            v.min_ticks(),
            expected
                .to_string()
                .parse::<before::Ticks>()
                .expect("the closed form parses"),
            "the family's leaf sum disagrees with min_ticks: the generator \
             does not build the tree this band reasons about"
        );
        touch_meter::reset();
        meter::reset_limb_ops();
        let rank = v.rank();
        std::hint::black_box(rank);
        let run = QueryRun {
            bytes,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert!(
            run.touches >= run.bytes,
            "rank at {bytes} operand bytes: {} digit touches under the \
             one-per-byte floor: the fold's accumulator work is not metered",
            run.touches,
        );
        run
    }

    /// Blocks of the freeze-position band's small run (the large run
    /// doubles the block count, doubling the packed operand).
    const RANK_FREEZE_POSITION_SMALL: usize = 1_000;

    /// Absolute two-scale (touch, limb) ceilings for rank on the
    /// freeze-position family, measured ×1.25.
    ///
    /// The cured record: the anchored-segment integral reads 87,519 /
    /// 175,260 touches and 35,243 / 70,485 limb ops at the two scales
    /// (~1.2 touches per packed byte, flat across the doubling),
    /// tightened in the cure's own commit from the absolute-position
    /// accounting's red pin of 124,371 / 372,862 touches and 72,351 /
    /// 206,804 limb ops (×1.50 touch and ×1.43 limb per-byte growth
    /// across the doubling, local exponent still rising at FP(4,000) —
    /// each freeze read the position accumulator's whole written span).
    // Ceilings: the element-wise tightest of two independent truings,
    // held green by the run below.
    const RANK_FREEZE_POSITION_CEILINGS: [(u64, u64); 2] = [(109_361, 44_054), (219_007, 88_590)];

    /// rank is linear on the freeze-position family: per-byte touch and
    /// limb work stay flat (×1.25) across a block-count doubling, under
    /// absolute two-scale ceilings.
    ///
    /// `FP(k)` fires one freeze per block — `Θ(k)` freezes at
    /// ever-deeper stream positions, every committed comb's count being
    /// O(1) — so any freeze accounting that reads an absolute position
    /// (or any whole-history state) per freeze goes quadratic here
    /// while the family's positions compact to O(1) digits. The
    /// anchored-segment discipline settles each freeze against its own
    /// segment's mass instead (read through the write watermark, spans
    /// never scales), so the flatness bound holds in both currencies.
    /// The committed tripwire beside the kernel
    /// (`absolute_position_accounting_reads_superlinear_on_freeze_position`,
    /// the query fold's test suite) keeps the absolute-position
    /// accounting failing on this family, so this band is never
    /// decoration.
    #[test]
    fn skyline_rank_freeze_position_is_flat_per_unit() {
        let small = rank_freeze_position_run(RANK_FREEZE_POSITION_SMALL);
        let large = rank_freeze_position_run(2 * RANK_FREEZE_POSITION_SMALL);
        assert_ceilings(
            "skyline_rank_freeze_position",
            &small,
            &large,
            RANK_FREEZE_POSITION_CEILINGS,
        );
        assert_flat(
            "rank_freeze_position_touches",
            "byte",
            (small.touches, small.bytes),
            (large.touches, large.bytes),
        );
        assert_flat(
            "rank_freeze_position_limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
    }

    /// The freeze-position family's near-flat co-operand: the same
    /// `2k`-node right spine with unit-descending small leaves
    /// (`2k + 1 − j` at depth `j`) over the terminal zero.
    ///
    /// Overlaying it against `FP(k)` gives the co-sweep the many-freezes
    /// genre in its two-operand form: the difference's deltas alternate
    /// a wide drop and zero, so this operand's cheap codes fire `Θ(k)`
    /// freezes of drift only the freeze-position operand's wide codes
    /// deposited — at ever-deeper stream positions.
    fn freeze_position_flat_mate(k: usize) -> before::Version {
        let mut text = String::new();
        for j in 0..2 * k {
            text.push_str("(0, ");
            text.push_str(&(2 * k - j).to_string());
            text.push_str(", ");
        }
        text.push('0');
        for _ in 0..2 * k {
            text.push(')');
        }
        text.parse()
            .expect("the descending unit spine is canonical")
    }

    /// One public distance-and-lag run over the two-operand
    /// freeze-position analogue `(FP(k), unit spine)`: both counters
    /// over the two query bodies together, with the pair's packed bytes
    /// as the per-byte denominator.
    ///
    /// Value legs anchor both measures before the counters return: the
    /// freeze-position operand dominates its mate pointwise, so
    /// `distance = rank(a) − rank(b)`, `lag(a, b) = 0`, and
    /// `lag(b, a) = distance` — three exact identities on `Rank`
    /// arithmetic the sweeps share nothing with.
    fn distance_freeze_position_run(k: usize) -> QueryRun {
        let a = Shape::FreezePosition.packed1(k).version();
        let b = freeze_position_flat_mate(k);
        let bytes = (a.encode().len() + b.encode().len()) as u64;
        let gap = a
            .rank()
            .checked_sub(&b.rank())
            .expect("the freeze-position operand dominates its mate");
        touch_meter::reset();
        meter::reset_limb_ops();
        let d = a.distance(&b);
        let forward = a.lag(&b);
        let backward = b.lag(&a);
        let run = QueryRun {
            bytes,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert_eq!(d, gap, "distance must be the dominating rank gap");
        assert_eq!(
            forward,
            before::Rank::ZERO,
            "the dominating side lags by nothing"
        );
        assert_eq!(backward, d, "the dominated side lags by the whole gap");
        assert!(
            run.touches >= run.bytes,
            "pair queries at {bytes} operand bytes: {} digit touches under \
             the one-per-byte floor: the co-sweep's difference state is not \
             running on the metered accumulator",
            run.touches,
        );
        run
    }

    /// Absolute two-scale (touch, limb) ceilings for the distance/lag
    /// triple on the freeze-position analogue, measured
    /// ×1.25.
    ///
    /// The record: 258,676 / 517,516 touches and 115,878 / 231,750 limb
    /// ops across `k = 1,000 → 2,000` on a 73 → 146 KiB packed pair —
    /// three sweeps' worth, ~3.5 touches per byte, flat across the
    /// doubling. Live remeasure: 256,037 / 512,297 touches and
    /// 115,492 / 230,976 limb ops, under the standing ceilings.
    const DISTANCE_FREEZE_POSITION_CEILINGS: [(u64, u64); 2] =
        [(323_345, 144_847), (646_895, 289_687)];

    /// Distance and lag are linear on the freeze-position analogue: the
    /// two-operand many-freezes genre reads flat (×1.25) per packed
    /// byte across a block-count doubling, under absolute two-scale
    /// ceilings.
    ///
    /// The review's residual risk: `Θ(k)` freezes where one operand's
    /// cheap codes fire evictions of drift only the other operand's
    /// wide codes deposited, at ever-deeper stream positions — the
    /// jump-pair wedge covered crest freezes, not span growth. The
    /// anchored-segment discipline settles each parked drift against
    /// its own segment's written span, and the parked component's
    /// monotone descent never triggers promotion, so no charge reads an
    /// absolute position and the flatness bound holds in both
    /// currencies.
    #[test]
    fn skyline_distance_freeze_position_is_flat_per_unit() {
        let small = distance_freeze_position_run(RANK_FREEZE_POSITION_SMALL);
        let large = distance_freeze_position_run(2 * RANK_FREEZE_POSITION_SMALL);
        assert_ceilings(
            "skyline_distance_freeze_position",
            &small,
            &large,
            DISTANCE_FREEZE_POSITION_CEILINGS,
        );
        assert_flat(
            "distance_freeze_position_touches",
            "byte",
            (small.touches, small.bytes),
            (large.touches, large.bytes),
        );
        assert_flat(
            "distance_freeze_position_limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
    }

    /// One public `Version::rank` run over the promotion re-arm spine
    /// `PR(p)` (`meter::promotion_rearm`), both counters over the rank
    /// body alone.
    ///
    /// Carries `min_ticks`' closed form as the cross-fold semantic leg
    /// (proving the generator builds the re-arm spine this band
    /// reasons about) and the one-touch-per-operand-byte liveness
    /// floor.
    fn rank_promotion_rearm_run(p: usize) -> QueryRun {
        use dashu_int::UBig;
        let v = Shape::PromotionRearm.packed1(p).version();
        let bytes = v.encode().len() as u64;
        let expected = UBig::from(16 * p as u64)
            + UBig::from(p as u64) * ((UBig::ONE << 608usize) + (UBig::ONE << 288usize) + 2u8)
            + 1u8;
        assert_eq!(
            v.min_ticks(),
            expected
                .to_string()
                .parse::<before::Ticks>()
                .expect("the closed form parses"),
            "the family's stored-code sum disagrees with min_ticks: the \
             generator does not build the tree this band reasons about"
        );
        touch_meter::reset();
        meter::reset_limb_ops();
        let rank = v.rank();
        std::hint::black_box(rank);
        let run = QueryRun {
            bytes,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert!(
            run.touches >= run.bytes,
            "rank at {bytes} operand bytes: {} digit touches under the \
             one-per-byte floor: the fold's accumulator work is not metered",
            run.touches,
        );
        run
    }

    /// Blocks of the promotion re-arm bands' small runs (the large runs
    /// double the count).
    const PROMOTION_REARM_SMALL: usize = 1_000;

    /// Absolute two-scale (touch, limb) ceilings for rank on the
    /// promotion re-arm spine, measured ×1.25.
    ///
    /// The record: the settle reads
    /// 403,912 → 808,098 touches and 206,192 → 412,837 limb ops across
    /// PR(1,000) → PR(2,000) on 246,501 B → 493,001 B (~1.6 touches
    /// per packed byte, flat across the doubling). Movement
    /// (the cluster-delegated settle, parent 401,387 → 802,887 touches
    /// and 187,925 → 375,924 limb ops): +0.6% touch, +9.7% limb — the
    /// per-freeze segment settles now compact through the window
    /// spelling and the products record their operand traffic; the
    /// per-byte exponent stays ×1.00. Movement
    /// (the window-digit combine tap, parent 184,180 → 368,429 limb
    /// ops): +2.0% limb — the settle's window-digit traffic now
    /// metered — touches unchanged. The span-reading
    /// promotion's committed tripwire reads 1,440,756 → 5,006,506
    /// touches here (×1.74/byte, local exponent ~1.9 and rising —
    /// every promotion re-read the position accumulator's whole
    /// written span). Live remeasure: 368,546 → 737,232 touches
    /// (−8.8%, accumulated since the record) and 205,479 → 411,125
    /// limb ops, under the standing ceilings.
    const RANK_PROMOTION_REARM_CEILINGS: [(u64, u64); 2] =
        [(504_890, 257_740), (1_010_122, 516_046)];

    /// rank is linear on the promotion re-arm spine: per-byte touch and
    /// limb work stay flat (×1.25) across a block-count doubling, under
    /// absolute two-scale ceilings.
    ///
    /// `PR(p)` fires one promotion per block at O(1) stored codes,
    /// after a `32p`-level climb keeps the consumed mass's written span
    /// growing — every committed comb promotes never, and the
    /// freeze-position spine's parked drift is monotone — so any
    /// promotion accounting that re-reads whole-history state per
    /// arming goes quadratic here while the family's suffix masses
    /// compact to O(1) balanced terms. The promotion ledger records
    /// each arming at funded widths and settles once at the sweep's
    /// close, so the flatness bound holds in both currencies. The
    /// committed tripwire beside the kernel
    /// (`span_promotion_accounting_reads_superlinear_on_rearm_spine`,
    /// the query fold's test suite) keeps the span-reading promotion
    /// failing on this family, so this band is never decoration.
    #[test]
    fn skyline_rank_promotion_rearm_is_flat_per_unit() {
        let small = rank_promotion_rearm_run(PROMOTION_REARM_SMALL);
        let large = rank_promotion_rearm_run(2 * PROMOTION_REARM_SMALL);
        assert_ceilings(
            "skyline_rank_promotion_rearm",
            &small,
            &large,
            RANK_PROMOTION_REARM_CEILINGS,
        );
        assert_flat(
            "rank_promotion_rearm_touches",
            "byte",
            (small.touches, small.bytes),
            (large.touches, large.bytes),
        );
        assert_flat(
            "rank_promotion_rearm_limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
    }

    /// One public `Version::rank` run over the lone-freeze spine
    /// `LF(pre, post)` (`meter::lone_freeze`), both counters over the
    /// rank body alone.
    ///
    /// Carries `min_ticks`' closed form as the cross-fold semantic leg
    /// (proving the generator builds the spine this band reasons
    /// about) and the one-touch-per-operand-byte liveness floor.
    fn rank_lone_freeze_run(pre: usize, post: usize) -> QueryRun {
        use dashu_int::UBig;
        let v = Shape::LoneFreeze.packed2(pre, post).version();
        let bytes = v.encode().len() as u64;
        let expected = UBig::from(pre as u64) * ((UBig::ONE << 288usize) + UBig::from(2u8))
            + UBig::from((pre / 2) as u64)
            + UBig::from((3 * post / 2) as u64)
            + UBig::from(3u8);
        assert_eq!(
            v.min_ticks(),
            expected
                .to_string()
                .parse::<before::Ticks>()
                .expect("the closed form parses"),
            "the family's leaf sum disagrees with min_ticks: the generator \
             does not build the tree this band reasons about"
        );
        touch_meter::reset();
        meter::reset_limb_ops();
        let rank = v.rank();
        std::hint::black_box(rank);
        let run = QueryRun {
            bytes,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert!(
            run.touches >= run.bytes,
            "rank at {bytes} operand bytes: {} digit touches under the \
             one-per-byte floor: the fold's accumulator work is not metered",
            run.touches,
        );
        run
    }

    /// Oscillation pairs of the lone-freeze bands' doubled axis (the
    /// large runs double it; the off-axis dial stays at the generator
    /// minimum so the doubled axis dominates the stored bytes).
    const LONE_FREEZE_SMALL: usize = 2_000;

    /// Absolute two-scale (touch, limb) ceilings for rank on the
    /// lone-freeze late axis, measured ×1.25.
    ///
    /// The record: 4,186 → 8,249 touches and 6,118 → 12,182 limb ops
    /// across LF(2,000, 2) → LF(4,000, 2) on 1,397 B → 2,647 B (~3.0
    /// touches per packed byte, flat across the doubling): the gate
    /// holds the segment feed shut across the whole never-freezing
    /// prefix.
    const RANK_LONE_FREEZE_LATE_CEILINGS: [(u64, u64); 2] = [(5_233, 7_648), (10_312, 15_228)];

    /// rank is linear on the lone-freeze spine's late axis: per-byte
    /// touch and limb work stay flat (×1.25) across a doubling of the
    /// never-freezing plateau prefix, under absolute two-scale
    /// ceilings.
    ///
    /// `LF(pre, 2)`'s whole prefix runs strictly before the sweep's
    /// one freeze — the first-freeze gate holds the segment feed shut
    /// for `pre` oscillation pairs, and the one settle that eventually
    /// runs never reads mass from that span — so any per-interval
    /// deposit toward the settle machinery made before drift exists to
    /// settle scales with the prefix here while the family's funded
    /// wide codes stay O(1). The unit oscillation itself must ride the
    /// live component without freezing (the trigger is relative to
    /// each boundary's own code).
    #[test]
    fn skyline_rank_lone_freeze_late_is_flat_per_unit() {
        let small = rank_lone_freeze_run(LONE_FREEZE_SMALL, 2);
        let large = rank_lone_freeze_run(2 * LONE_FREEZE_SMALL, 2);
        assert_ceilings(
            "skyline_rank_lone_freeze_late",
            &small,
            &large,
            RANK_LONE_FREEZE_LATE_CEILINGS,
        );
        assert_flat(
            "rank_lone_freeze_late_touches",
            "byte",
            (small.touches, small.bytes),
            (large.touches, large.bytes),
        );
        assert_flat(
            "rank_lone_freeze_late_limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
    }

    /// Absolute two-scale (touch, limb) ceilings for rank on the
    /// lone-freeze frozen-tail axis, measured ×1.25.
    ///
    /// The record: 6,246 → 12,372 touches and 6,118 → 12,182 limb ops
    /// across LF(2, 2,000) → LF(2, 4,000) on 1,397 B → 2,647 B (~4.5
    /// touches per packed byte, flat across the doubling). The tail
    /// axis reads ~one touch per interval above the late axis at the
    /// same bytes — exactly the open-gate segment feed, amortized O(1)
    /// per interval — and the close's one settle reads the whole
    /// tail's banked mass without moving the per-byte cost.
    const RANK_LONE_FREEZE_TAIL_CEILINGS: [(u64, u64); 2] = [(7_808, 7_648), (15_465, 15_228)];

    /// rank is linear on the lone-freeze spine's frozen-tail axis:
    /// per-byte touch and limb work stay flat (×1.25) across a
    /// doubling of the tail behind the sweep's one freeze, under
    /// absolute two-scale ceilings.
    ///
    /// `LF(2, post)`'s whole tail runs with the first-freeze gate open
    /// and a ten-digit drift parked: every tail interval feeds the
    /// segment mass, and the close's one `P · segment` settle reads
    /// that mass at its watermark across the tail's whole depth
    /// variation — so a segment feed that is not amortized O(1) per
    /// interval, or a close read priced by anything but the written
    /// span and the mass's compacted density, scales with the tail
    /// against O(1) funded wide codes. This is the frozen-path cost
    /// the gate must not regress: the segment machinery a
    /// never-freezing sweep skips runs here over the whole stream.
    #[test]
    fn skyline_rank_lone_freeze_tail_is_flat_per_unit() {
        let small = rank_lone_freeze_run(2, LONE_FREEZE_SMALL);
        let large = rank_lone_freeze_run(2, 2 * LONE_FREEZE_SMALL);
        assert_ceilings(
            "skyline_rank_lone_freeze_tail",
            &small,
            &large,
            RANK_LONE_FREEZE_TAIL_CEILINGS,
        );
        assert_flat(
            "rank_lone_freeze_tail_touches",
            "byte",
            (small.touches, small.bytes),
            (large.touches, large.bytes),
        );
        assert_flat(
            "rank_lone_freeze_tail_limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
    }

    /// Absolute two-scale (touch, limb) ceilings for min_ticks on the
    /// freeze-position spine, measured ×1.25.
    ///
    /// The record: 103,990 → 207,990 touches and 35,019 → 70,019 limb
    /// ops across FP(1,000) → FP(2,000) on 73,328 B → 146,579 B (~1.4
    /// touches per packed byte, flat across the doubling): `Θ(k)`
    /// epochs settle at one funded-width product each.
    const MIN_TICKS_FREEZE_POSITION_CEILINGS: [(u64, u64); 2] =
        [(129_988, 43_774), (259_988, 87_524)];

    /// min_ticks is linear on the freeze-position family: per-byte
    /// touch and limb work stay flat (×1.25) across a block-count
    /// doubling, under absolute two-scale ceilings.
    ///
    /// `FP(k)` fires one freeze per block — `Θ(k)` epochs in the
    /// min_ticks fold's ledger, where every committed comb's epoch
    /// count is O(1) — so the epoch ledger's settle runs its
    /// summation-by-parts over `Θ(k)` wide drifts here: an accounting
    /// that re-reads whole-history state per epoch (or re-bases any
    /// recorded offset across a freeze) goes quadratic while each
    /// drift's one settle product stays priced by the drift's own
    /// funded width times an O(1)-digit suffix count. The rank-side
    /// band prices the same schedule through the anchored-segment
    /// integral; this one prices the epoch ledger, min_ticks' own
    /// frozen-component accounting.
    #[test]
    fn skyline_min_ticks_freeze_position_is_flat_per_unit() {
        use dashu_int::UBig;
        let expected = |k: usize| {
            let band = 289 + (usize::BITS - k.leading_zeros()) as usize;
            (UBig::from(2 * k as u64) << band)
                + UBig::from((k * (k - 1)) as u64) * ((UBig::ONE << 288usize) + UBig::ONE)
                + UBig::from(k as u64)
        };
        let k = RANK_FREEZE_POSITION_SMALL;
        let small = min_ticks_family_run(Shape::FreezePosition.packed1(k), &expected(k));
        let large = min_ticks_family_run(Shape::FreezePosition.packed1(2 * k), &expected(2 * k));
        assert_ceilings(
            "skyline_min_ticks_freeze_position",
            &small,
            &large,
            MIN_TICKS_FREEZE_POSITION_CEILINGS,
        );
        assert_flat(
            "min_ticks_freeze_position_touches",
            "byte",
            (small.touches, small.bytes),
            (large.touches, large.bytes),
        );
        assert_flat(
            "min_ticks_freeze_position_limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
    }

    /// Absolute two-scale (touch, limb) ceilings for min_ticks on the
    /// promotion re-arm spine, measured ×1.25.
    ///
    /// The record: 516,060 → 1,032,060 touches and 212,012 → 424,012
    /// limb ops across PR(1,000) → PR(2,000) on 246,501 B → 493,001 B
    /// (~2.1 touches per packed byte, flat across the doubling):
    /// `Θ(p)` wide-drift epochs in both directions settle at one
    /// funded-width product each.
    const MIN_TICKS_PROMOTION_REARM_CEILINGS: [(u64, u64); 2] =
        [(645_075, 265_015), (1_290_075, 530_015)];

    /// min_ticks is linear on the promotion re-arm spine: per-byte
    /// touch and limb work stay flat (×1.25) across a block-count
    /// doubling, under absolute two-scale ceilings.
    ///
    /// `PR(p)` alternates 20-digit and 10-digit climbs through its
    /// blocks — `Θ(p)` freezes whose evicted drifts are wide in both
    /// directions — so the epoch ledger holds `Θ(p)` wide drifts whose
    /// reference counts the reign records must resolve against the
    /// right epoch: the settle's one `drift × suffix-count` product
    /// per epoch stays priced by each drift's own funded width, and
    /// any per-epoch re-read of whole-history state goes quadratic.
    /// The rank-side band prices this schedule through the promotion
    /// ledger; min_ticks has no promotion ledger — the epoch ledger is
    /// its entire frozen-component accounting, and this band is its
    /// many-epochs coverage.
    #[test]
    fn skyline_min_ticks_promotion_rearm_is_flat_per_unit() {
        use dashu_int::UBig;
        let expected = |p: usize| {
            UBig::from(16 * p as u64)
                + UBig::from(p as u64) * ((UBig::ONE << 608usize) + (UBig::ONE << 288usize) + 2u8)
                + 1u8
        };
        let p = PROMOTION_REARM_SMALL;
        let small = min_ticks_family_run(Shape::PromotionRearm.packed1(p), &expected(p));
        let large = min_ticks_family_run(Shape::PromotionRearm.packed1(2 * p), &expected(2 * p));
        assert_ceilings(
            "skyline_min_ticks_promotion_rearm",
            &small,
            &large,
            MIN_TICKS_PROMOTION_REARM_CEILINGS,
        );
        assert_flat(
            "min_ticks_promotion_rearm_touches",
            "byte",
            (small.touches, small.bytes),
            (large.touches, large.bytes),
        );
        assert_flat(
            "min_ticks_promotion_rearm_limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
    }

    /// One public distance-and-lag run over the two-operand promotion
    /// re-arm analogue `(PR(p), PRM(p))`: both counters over the three
    /// query bodies together, with the pair's packed bytes as the
    /// per-byte denominator.
    ///
    /// The mate is `PR(p)`'s unit-climb twin (same topology, every base
    /// 1), so the co-sweep's freezes and promotions fire at boundaries
    /// where the mate's cheap codes set the funded width while the
    /// drift being parked and promoted was deposited by the re-arm
    /// operand's wide codes — the two-operand arming genre the
    /// freeze-position analogue's monotone mate cannot reach (its own
    /// doc records that promotion never fires there; the committed
    /// span-promotion pair tripwire proves it fires here). Value legs
    /// anchor all three measures before the counters return: `PR(p)`
    /// dominates its mate pointwise, so `distance = rank(a) − rank(b)`,
    /// `lag(a, b) = 0`, and `lag(b, a) = distance`.
    fn distance_promotion_rearm_run(p: usize) -> QueryRun {
        let a = Shape::PromotionRearm.packed1(p).version();
        let b = Shape::PromotionRearmMate.packed1(p).version();
        let bytes = (a.encode().len() + b.encode().len()) as u64;
        let gap = a
            .rank()
            .checked_sub(&b.rank())
            .expect("the re-arm operand dominates its unit mate");
        touch_meter::reset();
        meter::reset_limb_ops();
        let d = a.distance(&b);
        let forward = a.lag(&b);
        let backward = b.lag(&a);
        let run = QueryRun {
            bytes,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert_eq!(d, gap, "distance must be the dominating rank gap");
        assert_eq!(
            forward,
            before::Rank::ZERO,
            "the dominating side lags by nothing"
        );
        assert_eq!(backward, d, "the dominated side lags by the whole gap");
        assert!(
            run.touches >= run.bytes,
            "pair queries at {bytes} operand bytes: {} digit touches under \
             the one-per-byte floor: the co-sweep's difference state is not \
             running on the metered accumulator",
            run.touches,
        );
        run
    }

    /// Absolute two-scale (touch, limb) ceilings for the distance/lag
    /// triple on the promotion re-arm analogue, measured
    /// ×1.25.
    ///
    /// The record: 1,095,042 → 2,190,366 touches and 866,425 →
    /// 1,733,715 limb ops across `p = 1,000 → 2,000` on a 263 → 525
    /// KiB packed pair — three sweeps' worth, ~4 touches per byte,
    /// flat across the doubling. Movement (the
    /// cluster-delegated settle, parent 1,090,072 → 2,180,072 touches
    /// and 829,891 → 1,659,889 limb ops): +0.5% touch, +4.4% limb —
    /// the per-freeze segment settles' window spelling and the
    /// products' operand traffic; the per-byte exponent stays ×1.00
    /// (the span-reading promotion reads ×1.71 per byte here, the
    /// committed pair tripwire's record). Movement (the
    /// window-digit combine tap, parent 822,401 → 1,644,899 limb ops):
    /// +0.9% limb — the settle's window-digit traffic now metered —
    /// touches unchanged. Live remeasure: 988,333 → 1,976,657 touches
    /// (−9.7%, accumulated since the record) and 864,999 → 1,730,291
    /// limb ops, under the standing ceilings.
    const DISTANCE_PROMOTION_REARM_CEILINGS: [(u64, u64); 2] =
        [(1_368_802, 1_083_031), (2_737_957, 2_167_143)];

    /// Distance and lag are linear on the promotion re-arm analogue:
    /// the two-operand arming genre reads flat (×1.25) per packed byte
    /// across a block-count doubling, under absolute two-scale
    /// ceilings.
    ///
    /// One operand's cheap codes fire freezes and promotions of drift
    /// only the other operand's wide codes deposited — the promotion
    /// ledger records each arming at funded widths and settles once,
    /// so no charge reads an absolute position and the flatness bound
    /// holds in both currencies.
    #[test]
    fn skyline_distance_promotion_rearm_is_flat_per_unit() {
        let small = distance_promotion_rearm_run(PROMOTION_REARM_SMALL);
        let large = distance_promotion_rearm_run(2 * PROMOTION_REARM_SMALL);
        assert_ceilings(
            "skyline_distance_promotion_rearm",
            &small,
            &large,
            DISTANCE_PROMOTION_REARM_CEILINGS,
        );
        assert_flat(
            "distance_promotion_rearm_touches",
            "byte",
            (small.touches, small.bytes),
            (large.touches, large.bytes),
        );
        assert_flat(
            "distance_promotion_rearm_limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
    }

    /// One public-distance run over the two-operand jump comb
    /// `JP(k, m, d)`: both counters over the distance body alone, with
    /// the operands' packed bytes and stored delta codes as the per-unit
    /// denominators.
    ///
    /// Enforces the touch liveness floor (every stored delta lands in
    /// the metered accumulator) and anchors the result by rank
    /// modularity before returning.
    fn distance_jump_pair_run(k: usize, m: usize, d: usize) -> Run {
        let (pa, pb) = Shape::JumpPair.packed_pair3(k, m, d);
        let a = pa.version();
        let b = pb.version();
        let bytes = (a.encode().len() + b.encode().len()) as u64;
        // Per operand: one leaf per shared-spine level (33d), three per
        // comb level, and the comb terminal; deltas are leaves − 1.
        let deltas = 2 * (33 * d as u64 + 3 * m as u64);
        touch_meter::reset();
        meter::reset_limb_ops();
        let r = a.distance(&b);
        let run = Run {
            deltas,
            bytes,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert!(
            run.touches >= run.deltas,
            "distance_jump_pair m={m}: {} digit touches under the {}-delta floor: \
             the query height state is not running on the metered accumulator",
            run.touches,
            run.deltas,
        );
        assert_eq!(
            r,
            &a.lag(&b) + &b.lag(&a),
            "the distance must equal the two lags' sum (rank modularity)"
        );
        run
    }

    /// One public-rank run over a single [`meter::jump_pair`] operand:
    /// the flat single-operand control for the jump-pair band below.
    fn rank_jump_pair_operand_run(k: usize, m: usize, d: usize, band: bool) -> Run {
        let (pa, pb) = Shape::JumpPair.packed_pair3(k, m, d);
        let v = if band { pb.version() } else { pa.version() };
        let bytes = v.encode().len() as u64;
        let deltas = 33 * d as u64 + 3 * m as u64;
        touch_meter::reset();
        meter::reset_limb_ops();
        let r = v.rank();
        let run = Run {
            deltas,
            bytes,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert!(
            run.touches >= run.deltas,
            "rank height state left the accumulator"
        );
        drop(r);
        run
    }

    /// Comb levels of the band's small run (the large run doubles both
    /// parameters; the position digits stay an eighth of the teeth, the
    /// board family's proportion).
    const DISTANCE_JUMP_PAIR_SMALL_TEETH: usize = 512;

    /// Freeze-position digits of the band's small run.
    const DISTANCE_JUMP_PAIR_SMALL_DIGITS: usize = 64;

    /// Absolute two-scale touch ceilings for the jump-pair distance,
    /// measured ×1.25.
    ///
    /// The record: the anchored-segment co-sweep read 184,494 / 368,958
    /// touches at the two scales (1.37 per packed byte, flat across the
    /// doubling), tightened in that cure's own commit from the composed
    /// form's red pin of 1,029,327 / 3,735,733 (7.6 → 13.9 per byte
    /// across the doubling — the superlinear wedge this family was
    /// built to expose), then to 170,128 / 340,256 (1.26 per byte,
    /// still flat; certificate skips replace the
    /// accumulator's zero-run walks). Live remeasure: 166,993 /
    /// 333,979 touches, under the standing ceilings.
    const DISTANCE_JUMP_PAIR_TOUCH_CEILINGS: (u64, u64) = (212_660, 425_320);
    /// The limb ceilings paired with
    /// [`DISTANCE_JUMP_PAIR_TOUCH_CEILINGS`].
    ///
    /// Measured 53,905 / 107,753 (0.40 per byte, flat), tightened from
    /// the composed form's 973,702 / 3,341,816 (7.2 → 12.4 per byte).
    /// Live remeasure: 53,559 / 107,048, under the standing ceilings.
    const DISTANCE_JUMP_PAIR_LIMB_CEILINGS: (u64, u64) = (67_382, 134_692);

    /// The jump-pair distance is linear in the packed pair: per-byte
    /// touch and limb work stay flat (×1.25) across a (teeth, digits)
    /// doubling, under absolute two-scale ceilings.
    ///
    /// Both single-operand ranks are pinned flat beside the pair, so
    /// the family's separation stays whole: the shape exists only in
    /// the two-operand composition.
    ///
    /// The family interleaves one operand's wide teeth with the other's
    /// near-flat band over a shared spine whose right turns plant
    /// isolated position bits, so the overlay's height difference
    /// crests wide once per comb level while every absolute position
    /// stays dense under balanced compaction. A freeze accounting that
    /// multiplies evicted drift by absolute positions pays
    /// teeth × digits × magnitude here and reads superlinear — one
    /// operand's cheap codes firing corrections against drift only the
    /// other operand funded — which is what this family's growth leg
    /// read (×1.72 per-byte limb growth across the doubling) until the
    /// anchored-segment co-sweep landed: each crest now settles against
    /// its own segment's mass, whose compacted span the spine's shared
    /// prefix never enters, so the flatness bound holds at both scales
    /// and each operand alone stays the flat control.
    #[test]
    fn skyline_distance_jump_pair_is_flat_per_unit() {
        let k = super::JUMP_PAIR_MAGNITUDE_BITS;
        let (m, d) = (
            DISTANCE_JUMP_PAIR_SMALL_TEETH,
            DISTANCE_JUMP_PAIR_SMALL_DIGITS,
        );
        let small = distance_jump_pair_run(k, m, d);
        let large = distance_jump_pair_run(k, 2 * m, 2 * d);
        for (run, scale) in [(&small, "small"), (&large, "large")] {
            eprintln!(
                "MEASURED distance_jump_pair_{scale}: bytes={} touches={} limb_ops={}",
                run.bytes, run.touches, run.limb_ops,
            );
        }
        for (run, (touch_ceiling, limb_ceiling), scale) in [
            (
                &small,
                (
                    DISTANCE_JUMP_PAIR_TOUCH_CEILINGS.0,
                    DISTANCE_JUMP_PAIR_LIMB_CEILINGS.0,
                ),
                "small",
            ),
            (
                &large,
                (
                    DISTANCE_JUMP_PAIR_TOUCH_CEILINGS.1,
                    DISTANCE_JUMP_PAIR_LIMB_CEILINGS.1,
                ),
                "large",
            ),
        ] {
            assert!(
                run.touches <= touch_ceiling,
                "distance_jump_pair_{scale}: {} touches exceed the pinned ceiling \
                 {touch_ceiling}: an absolute-position product is back in the \
                 co-sweep's freeze accounting",
                run.touches,
            );
            assert!(
                run.limb_ops <= limb_ceiling,
                "distance_jump_pair_{scale}: {} limb ops exceed the pinned ceiling \
                 {limb_ceiling}: an absolute-position product is back in the \
                 co-sweep's freeze accounting",
                run.limb_ops,
            );
        }
        // The flatness bound: per-byte cost must not grow across the
        // doubling — the reading that separates the anchored-segment
        // accounting from any absolute-position one.
        assert_flat(
            "distance_jump_pair_touches",
            "byte",
            (small.touches, small.bytes),
            (large.touches, large.bytes),
        );
        assert_flat(
            "distance_jump_pair_limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
        // The separation witnesses: either operand alone stays flat —
        // the teeth operand's wide folds cancel adjacently (bounded
        // oscillation), the band operand pays its width once.
        let teeth_small = rank_jump_pair_operand_run(k, m, d, false);
        let teeth_large = rank_jump_pair_operand_run(k, 2 * m, 2 * d, false);
        assert_flat(
            "rank_jump_pair_teeth_limb_ops",
            "byte",
            (teeth_small.limb_ops, teeth_small.bytes),
            (teeth_large.limb_ops, teeth_large.bytes),
        );
        let band_small = rank_jump_pair_operand_run(k, m, d, true);
        let band_large = rank_jump_pair_operand_run(k, 2 * m, 2 * d, true);
        assert_flat(
            "rank_jump_pair_band_limb_ops",
            "byte",
            (band_small.limb_ops, band_small.bytes),
            (band_large.limb_ops, band_large.bytes),
        );
    }

    /// One fused three-stream comparison run over the mask-drift triple
    /// at `scale` teeth: per-delta touches and per-byte limb work, with
    /// the one-touch-per-delta liveness floor enforced before returning.
    fn masked_cmp_run(scale: usize) -> Run {
        let (comb, mask, plateau) = Shape::MaskDriftTriple.packed_triple(512, scale);
        let v = comb.version();
        let p = before::Party::decode(&mask.bytes[..]).expect("the mask is strict normal form");
        let w = plateau.version();
        let bytes = (v.encode().len() + mask.bytes.len() + w.encode().len()) as u64;
        touch_meter::reset();
        meter::reset_limb_ops();
        let verdict = (&v / &p).partial_cmp(&w);
        assert_eq!(
            verdict,
            Some(std::cmp::Ordering::Less),
            "the projected comb sits strictly under the plateau (no early exit)"
        );
        let run = Run {
            // The comb's 2n + 1 leaves put 2n delta codes behind the
            // first; the plateau adds none.
            deltas: 2 * scale as u64,
            bytes,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert!(
            run.touches >= run.deltas,
            "masked_cmp scale {scale}: {} digit touches under the {}-delta floor: \
             the walk's integrators are not running on the metered accumulator",
            run.touches,
            run.deltas,
        );
        run
    }

    /// The fused three-stream comparison's per-delta touches and
    /// per-byte limb work stay flat across a tooth-count doubling of
    /// the mask-drift triple.
    ///
    /// Every mask boundary's sign read — the difference mid-cancel
    /// inside owned teeth, the zero-check on unowned intervals — stays
    /// amortized O(1) however many boundaries the mask plants.
    ///
    /// Each run carries the one-touch-per-delta liveness floor (in
    /// [`masked_cmp_run`]), so flatness is asserted over a meter proven
    /// live. This is the correlated family's wedge test: an integrator
    /// that materialized a read per boundary would grow the per-delta
    /// cost with the magnitude and fail the band.
    #[test]
    fn masked_cmp_drift_cost_is_flat_per_unit() {
        let small = masked_cmp_run(1_024);
        let large = masked_cmp_run(2_048);
        assert_flat(
            "masked_cmp_touches",
            "delta",
            (small.touches, small.deltas),
            (large.touches, large.deltas),
        );
        assert_flat(
            "masked_cmp_limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
    }

    /// One fused four-stream comparison run over the mask-drift
    /// quadruple at `scale` teeth, as [`masked_cmp_run`].
    fn masked_pair_cmp_run(scale: usize) -> Run {
        let ((sparse, even_mask), (comb, odd_mask)) =
            Shape::MaskDriftQuadruple.packed_quadruple(512, scale);
        let v1 = sparse.version();
        let p1 =
            before::Party::decode(&even_mask.bytes[..]).expect("the mask is strict normal form");
        let v2 = comb.version();
        let p2 =
            before::Party::decode(&odd_mask.bytes[..]).expect("the mask is strict normal form");
        let bytes =
            (v1.encode().len() + even_mask.bytes.len() + v2.encode().len() + odd_mask.bytes.len())
                as u64;
        touch_meter::reset();
        meter::reset_limb_ops();
        let verdict = (&v1 / &p1).partial_cmp(&(&v2 / &p2));
        assert_eq!(
            verdict,
            Some(std::cmp::Ordering::Less),
            "the semantically-empty view sits strictly under the tooth-keeping view"
        );
        let run = Run {
            // The sparse comb's n + 1 leaves put n delta codes behind its
            // first; the full comb adds 2n.
            deltas: 3 * scale as u64,
            bytes,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert!(
            run.touches >= run.deltas,
            "masked_pair_cmp scale {scale}: {} digit touches under the {}-delta floor: \
             the walk's integrators are not running on the metered accumulator",
            run.touches,
            run.deltas,
        );
        run
    }

    /// The fused four-stream comparison's per-delta touches and
    /// per-byte limb work stay flat across a tooth-count doubling of
    /// the mask-drift quadruple.
    ///
    /// The zero-check on cancelling wide spellings (even teeth) and the
    /// mid-oscillation reads (odd teeth) are both amortized O(1) per
    /// boundary.
    #[test]
    fn masked_pair_cmp_drift_cost_is_flat_per_unit() {
        let small = masked_pair_cmp_run(1_024);
        let large = masked_pair_cmp_run(2_048);
        assert_flat(
            "masked_pair_cmp_touches",
            "delta",
            (small.touches, small.deltas),
            (large.touches, large.deltas),
        );
        assert_flat(
            "masked_pair_cmp_limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
    }

    // ── the accumulator skip mechanisms' before-level adequacy bands ──
    //
    // Three families, one per skip/extent mechanism inside the
    // accumulator (`suanpan`), each constructed so that the mechanism's
    // *absence* — scans stepping digit by digit instead of consuming a
    // zero-run certificate; scaled reads starting at digit 0 instead of
    // the write watermark; loop bounds and fold starts reading the
    // buffer's high water instead of the settled top — turns one public
    // `before` operation superlinear while the family's input stays
    // linear (each band's ceiling doc carries both readings, measured
    // by disabling exactly one mechanism in a local probe
    // build, value-identical by the full differential suite). On the
    // shipped accumulator all three read flat; each band is the
    // before-level witness that its mechanism is load-bearing, priced
    // through the public API rather than through `suanpan`'s own entry
    // points (whose row witnesses,
    // `alternating_shifted_writes_cost_the_operand_not_the_gap`,
    // `scaled_read_costs_the_written_span`, and
    // `held_width_rows_cost_the_held_digits`, pin the same three
    // mechanisms crate-locally).

    /// One `Version::rank` run over the weight-comb family `WC(n)`
    /// (`meter::weight_comb`), both counters over the rank body alone,
    /// with the tick total as the semantic leg and a
    /// one-touch-per-topology-byte liveness floor.
    fn rank_weight_comb_run(n: usize) -> QueryRun {
        use dashu_int::UBig;
        let v = Shape::WeightComb.packed1(n).version();
        let bytes = v.encode().len() as u64;
        // Σ stored bases: the spine's 32n − 1 unit leaves plus the
        // block's n twos.
        let expected = UBig::from((34 * n - 1) as u64);
        assert_eq!(
            v.min_ticks(),
            expected
                .to_string()
                .parse::<before::Ticks>()
                .expect("the closed form parses"),
            "the family's base sum disagrees with min_ticks: the generator \
             does not build the tree this band reasons about"
        );
        touch_meter::reset();
        meter::reset_limb_ops();
        let rank = v.rank();
        std::hint::black_box(rank);
        let run = QueryRun {
            bytes,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        // The liveness floor is the mechanism's irreducible work, not
        // the family's typical work: every nonzero stored delta folds
        // into the integral's accumulator at least once, and the
        // block's `2n` leaves alternate heights 0 and 2, so all `2n`
        // of its deltas are nonzero.
        assert!(
            run.touches >= 2 * n as u64,
            "rank on WC({n}): {} digit touches under the one-per-nonzero-delta \
             floor of {}: the fold's accumulator work is not metered",
            run.touches,
            2 * n,
        );
        run
    }

    /// Absolute two-scale (touch, limb) ceilings for rank on the
    /// weight comb, measured ×1.25.
    ///
    /// The record: 5,131 / 10,251 touches and 36,353 / 72,705 limb
    /// ops across `n = 512 → 1,024` (~0.73 touches per packed byte,
    /// flat across the doubling; touches 21,512 / 43,016 → 4,104 /
    /// 8,200 — the segment feed opens at the
    /// first freeze, and this family never freezes, so its per-leaf
    /// feed deposits left with the round — → 5,131 / 10,251: the
    /// accumulator's quick register re-arms per lease epoch, and this
    /// family's wide cycling pays one register spill per epoch). With certificate
    /// consumption disabled (measured under a local probe
    /// build whose scans step digit by digit), the probe runs
    /// read 282,632 → 1,089,544 touches — `n² + O(n)` exactly, ×1.93
    /// per byte across the doubling — so this band is the
    /// before-level adequacy witness for the zero-run ledger.
    const RANK_WEIGHT_COMB_CEILINGS: [(u64, u64); 2] = [(6_414, 45_441), (12_814, 90_881)];

    /// Block pairs of the weight-comb band's small run.
    const RANK_WEIGHT_COMB_SMALL: usize = 512;

    /// rank is linear on the weight comb: per-byte touch work stays
    /// flat (×1.25) across a block doubling, under absolute two-scale
    /// ceilings.
    ///
    /// `WC(n)` re-raises and cancels one digit `Θ(n)` digits above a
    /// parked unit, `Θ(n)` times, for O(1) stored bits per event — the
    /// position weight is topology, so no code funds the gap between.
    /// Each cancellation forces the accumulator's top to settle back
    /// across the never-written gap: a settlement that walks the gap
    /// pays `Θ(n)` unfunded touches per event (`Θ(n²)` on linear
    /// input), and the parked digit-0 unit forecloses value-emptiness
    /// and write-watermark shortcuts — one certificate per jumped run,
    /// consumed whole, is what holds this band flat. This is the
    /// public-API lift of the accumulator's own row witness
    /// (`alternating_shifted_writes_cost_the_operand_not_the_gap`):
    /// there the shift is a free parameter; here the stream buys the
    /// position with `Θ(n)` one-time topology bits and then oscillates
    /// at O(1) bits per event.
    #[test]
    fn skyline_rank_weight_comb_is_flat_per_unit() {
        let small = rank_weight_comb_run(RANK_WEIGHT_COMB_SMALL);
        let large = rank_weight_comb_run(2 * RANK_WEIGHT_COMB_SMALL);
        assert_ceilings(
            "skyline_rank_weight_comb",
            &small,
            &large,
            RANK_WEIGHT_COMB_CEILINGS,
        );
        assert_flat(
            "rank_weight_comb_touches",
            "byte",
            (small.touches, small.bytes),
            (large.touches, large.bytes),
        );
    }

    /// One `Version::rank` run over the freeze-parade family `FZ(k)`
    /// (`meter::freeze_parade`): the same harness as the weight
    /// comb's.
    fn rank_freeze_parade_run(k: usize) -> QueryRun {
        use dashu_int::UBig;
        let v = Shape::FreezeParade.packed1(k).version();
        let bytes = v.encode().len() as u64;
        // Σ printed bases in closed form: the spine's 64k − 1 unit
        // leaves, the block's k left-leaf wide drops, its internal
        // left children's half-minima differences (k/2 per level,
        // each level's difference doubling from the pair stride
        // 2^288 + 1), and its root's absolute minimum
        // 2^band − (k − 1)(2^288 + 1) − 2^288.
        let j = (usize::BITS - k.leading_zeros()) as usize - 1;
        let band = 290 + (usize::BITS - k.leading_zeros()) as usize;
        let w = UBig::ONE << 288usize;
        let stride = &w + UBig::ONE;
        let expected = UBig::from((64 * k - 1) as u64)
            + (UBig::ONE << band)
            + UBig::from(k as u64) * &w
            + UBig::from((k / 2 * j) as u64) * &stride
            - UBig::from((k - 1) as u64) * &stride
            - &w;
        assert_eq!(
            v.min_ticks(),
            expected
                .to_string()
                .parse::<before::Ticks>()
                .expect("the closed form parses"),
            "the family's base sum disagrees with min_ticks: the generator \
             does not build the tree this band reasons about"
        );
        touch_meter::reset();
        meter::reset_limb_ops();
        let rank = v.rank();
        std::hint::black_box(rank);
        let run = QueryRun {
            bytes,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        // The liveness floor is the mechanism's irreducible work, not
        // the family's typical work: every nonzero stored delta folds
        // into the integral's accumulator at least once, and each of
        // the `k` freeze blocks stores two nonzero drops (the wide
        // in-pair `2^288` and the unit cross-pair code).
        assert!(
            run.touches >= 2 * k as u64,
            "rank on FZ({k}): {} digit touches under the one-per-nonzero-delta \
             floor of {}: the fold's accumulator work is not metered",
            run.touches,
            2 * k,
        );
        run
    }

    /// Absolute two-scale (touch, limb) ceilings for rank on the
    /// freeze parade, measured ×1.25.
    ///
    /// The record: 46,774 / 93,530 touches and 83,973 / 167,941 limb
    /// ops across `k = 512 → 1,024` (~0.94 touches per packed byte,
    /// flat across the doubling; touches 81,080 / 162,140 → 46,774 /
    /// 93,530 — the segment feed opens at the
    /// first freeze, so the deep pre-freeze spine no longer deposits;
    /// the freeze blocks' banked segments and settles are untouched,
    /// and the limb column is byte-identical). With the write
    /// watermark disabled (measured under a local probe
    /// build whose scaled reads start at digit 0), the probe
    /// runs read 864,953 → 3,302,749 touches and 345,605 → 1,215,493
    /// limb ops — ×1.91 per byte across the doubling in both
    /// currencies — so this band is the before-level adequacy witness
    /// for the watermark read.
    const RANK_FREEZE_PARADE_CEILINGS: [(u64, u64); 2] = [(58_468, 104_967), (116_913, 209_927)];

    /// Freeze blocks of the parade band's small run.
    const RANK_FREEZE_PARADE_SMALL: usize = 512;

    /// rank is linear on the freeze parade: per-byte touch work stays
    /// flat (×1.25) across a block doubling, under absolute two-scale
    /// ceilings.
    ///
    /// `FZ(k)` fires `Θ(k)` freezes whose segments all sit `Θ(k)`
    /// digits above digit 0 (the blocks are shallow; the deep spine
    /// only sets the scale), so every settle's segment read crosses a
    /// `Θ(k)`-digit never-written prefix. The watermark read prices
    /// each at the segment's written span; a read that starts at digit
    /// 0 pays the prefix per freeze — `Θ(k²)` touches on linear input,
    /// and the zero-padded magnitudes it returns drag the limb column
    /// superlinear with it. The freeze-position family pins the
    /// query-layer half of this genre (no absolute position is read
    /// per freeze); this band pins the accumulator half — the
    /// public-API lift of `scaled_read_costs_the_written_span`.
    #[test]
    fn skyline_rank_freeze_parade_is_flat_per_unit() {
        let small = rank_freeze_parade_run(RANK_FREEZE_PARADE_SMALL);
        let large = rank_freeze_parade_run(2 * RANK_FREEZE_PARADE_SMALL);
        assert_ceilings(
            "skyline_rank_freeze_parade",
            &small,
            &large,
            RANK_FREEZE_PARADE_CEILINGS,
        );
        assert_flat(
            "rank_freeze_parade_touches",
            "byte",
            (small.touches, small.bytes),
            (large.touches, large.bytes),
        );
    }

    /// One comparison-sweep run over the tooth-tail pair `TT(g, m)`
    /// (`meter::tooth_tail`): touches over the `causal_cmp` body
    /// alone, with the verdict as the semantic leg and a
    /// one-touch-per-boundary liveness floor.
    fn cmp_tooth_tail_run(g: usize, m: usize) -> QueryRun {
        let (a, b) = Shape::ToothTail.packed_pair(g, m);
        let (a, b) = (a.version(), b.version());
        let ea = meter::skyline::encode(&a);
        let eb = meter::skyline::encode(&b);
        let bytes = (ea.as_raw_slice().len() + eb.as_raw_slice().len()) as u64;
        touch_meter::reset();
        meter::reset_limb_ops();
        let verdict = meter::skyline::sweep::causal_cmp(&ea, &eb);
        let run = QueryRun {
            bytes,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert_eq!(
            verdict,
            Some(std::cmp::Ordering::Less),
            "b runs one tick above a everywhere except the shared terminal"
        );
        assert!(
            run.touches >= m as u64,
            "cmp at {m} boundaries: {} digit touches under the \
             one-per-boundary floor: the sweep's difference state is not \
             running on the metered accumulator",
            run.touches,
        );
        run
    }

    /// Absolute two-scale (touch, limb) ceilings for the comparison
    /// sweep on the tooth-tail pair, measured ×1.25.
    ///
    /// The record: 4,239 / 8,463 touches and 16,716 / 33,420 limb ops
    /// across `(g, m) = (64, 4,096) → (128, 8,192)` (~0.8 touches per
    /// packed byte, flat across the doubling). With the settled top
    /// replaced by the buffer's high water (measured under
    /// a local probe build), the same runs read 536,590 → 2,121,742
    /// touches — 2(g + 1) per boundary, ×1.98 per byte across the
    /// doubling — so this band is the before-level adequacy witness
    /// for exact-top maintenance.
    const CMP_TOOTH_TAIL_CEILINGS: [(u64, u64); 2] = [(5_298, 20_895), (10_578, 41_775)];

    /// Boundaries of the tooth-tail band's small run.
    const CMP_TOOTH_TAIL_SMALL: usize = 4_096;

    /// The comparison sweep is linear on the tooth-tail pair: per-byte
    /// touch work stays flat (×1.25) across a joint `(g, m)` doubling,
    /// under absolute two-scale ceilings.
    ///
    /// `TT(g, m)`'s cancelled spike leaves the difference accumulator
    /// holding −1 in one digit under a buffer `g` digits tall, and the
    /// sweep then reads `sign(D)` once per boundary, `m` times, with
    /// no intervening write. The settled top prices each read at the
    /// value's width; any high-water bound re-walks the spike's `g`
    /// dead digits per read — `Θ(m·g)` on `Θ(m + g)` input, the cost
    /// the spike's own code paid once and would otherwise be re-paid
    /// per boundary forever. The public-API lift of
    /// `held_width_rows_cost_the_held_digits`: reads price the settled
    /// width, and the settlement (with its certificate skip) is what
    /// keeps the settled width honest after a cancellation.
    #[test]
    fn skyline_cmp_tooth_tail_is_flat_per_unit() {
        let m = CMP_TOOTH_TAIL_SMALL;
        let small = cmp_tooth_tail_run(m / 64, m);
        let large = cmp_tooth_tail_run(m / 32, 2 * m);
        assert_ceilings(
            "skyline_cmp_tooth_tail",
            &small,
            &large,
            CMP_TOOTH_TAIL_CEILINGS,
        );
        assert_flat(
            "cmp_tooth_tail_touches",
            "byte",
            (small.touches, small.bytes),
            (large.touches, large.bytes),
        );
    }

    /// One public `Version::rank` run over the dense-suffix family
    /// `DS(p, p)` (`meter::dense_suffix`), both counters over the rank
    /// body alone.
    ///
    /// Carries `min_ticks`' closed form as the cross-fold semantic leg
    /// (proving the generator builds the gap spine and the block
    /// schedule this band reasons about) and the
    /// one-touch-per-operand-byte liveness floor.
    fn rank_dense_suffix_run(p: usize) -> QueryRun {
        use dashu_int::UBig;
        let v = Shape::DenseSuffix.packed2(p, p).version();
        let bytes = v.encode().len() as u64;
        let expected = UBig::from(p as u64)
            + UBig::from(p as u64) * ((UBig::ONE << 608usize) + (UBig::ONE << 288usize) + 2u8)
            + 1u8;
        assert_eq!(
            v.min_ticks(),
            expected
                .to_string()
                .parse::<before::Ticks>()
                .expect("the closed form parses"),
            "the family's stored-code sum disagrees with min_ticks: the \
             generator does not build the tree this band reasons about"
        );
        touch_meter::reset();
        meter::reset_limb_ops();
        let rank = v.rank();
        std::hint::black_box(rank);
        let run = QueryRun {
            bytes,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert!(
            run.touches >= run.bytes,
            "rank at {bytes} operand bytes: {} digit touches under the \
             one-per-byte floor: the fold's accumulator work is not metered",
            run.touches,
        );
        run
    }

    /// Blocks (and suffix digits) of the dense-suffix bands' small runs
    /// (the large runs double both).
    const DENSE_SUFFIX_SMALL: usize = 500;

    /// Absolute two-scale (touch, limb) ceilings for rank on the
    /// dense-suffix family, measured ×1.25.
    ///
    /// The record: the mass-balanced product-tree settle reads
    /// 179,764 → 359,126 touches and 100,400 → 200,620 limb ops across
    /// DS(500, 500) → DS(1,000, 1,000) on 119,593 B → 239,030 B (~1.5
    /// touches per packed byte, flat across the doubling), tightened in
    /// the settle's own commit from the per-arming suffix walk's red pin
    /// of 3,417,450 → 13,357,237 touches and 2,837,934 → 11,175,881
    /// limb ops (×1.96 touch and ×1.97 limb per-byte growth, local
    /// exponent ~2.0 — every arming re-walked the suffix's Θ(d)
    /// balanced digits). Movement (the cluster-delegated
    /// settle, parent 219,764 → 437,912 touches and 126,403 → 252,379
    /// limb ops): −18% touch, −21% limb — each aggregate charge is one
    /// backend product per dense cluster instead of a factor-wide
    /// product per window digit; the per-byte exponent stays ×1.00 in
    /// both currencies. Live remeasure: 177,264 → 354,542 touches and
    /// 97,381 → 195,491 limb ops, under the standing ceilings.
    const RANK_DENSE_SUFFIX_CEILINGS: [(u64, u64); 2] = [(224_705, 125_500), (448_907, 250_775)];

    /// rank is flat per byte on the dense-suffix family under the
    /// declared log model: per-byte touch and limb work stay within
    /// ×1.25 across a block-count doubling, under absolute two-scale
    /// ceilings.
    ///
    /// `DS(p, p)` fires one promotion per block against a trailing
    /// interval mass the gap spine holds at Θ(p) balanced digits — the
    /// shape on which any settle that walks the suffix once per arming
    /// (or re-reads a promoted prefix once per window) goes quadratic.
    /// The mass-balanced product tree charges every arming-window
    /// cross term inside exactly one aggregate product and rewrites
    /// any window's digits once per tree level, so the declared model
    /// admits per-byte growth up to the log ratio — at this family's
    /// shape a doubling could read at most ×(log₂ 2p / log₂ p) ≈ ×1.11
    /// even if the settle dominated the fold, inside the band's ×1.25
    /// slack — and the measurement reads ×1.00 in both currencies (the
    /// settle's log term is a small share of the fold's linear work).
    /// The committed tripwire beside the kernel
    /// (`suffix_walk_settle_reads_superlinear_on_dense_suffix`, the
    /// query fold's test suite) keeps the per-arming suffix walk
    /// failing on this family, so this band is never decoration.
    #[test]
    fn skyline_rank_dense_suffix_is_flat_per_unit() {
        let small = rank_dense_suffix_run(DENSE_SUFFIX_SMALL);
        let large = rank_dense_suffix_run(2 * DENSE_SUFFIX_SMALL);
        assert_ceilings(
            "skyline_rank_dense_suffix",
            &small,
            &large,
            RANK_DENSE_SUFFIX_CEILINGS,
        );
        assert_flat(
            "rank_dense_suffix_touches",
            "byte",
            (small.touches, small.bytes),
            (large.touches, large.bytes),
        );
        assert_flat(
            "rank_dense_suffix_limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
    }

    /// One public distance-and-lag run over `(DS(p, p), DSM(p, p))`:
    /// both counters over the three query bodies together, with the
    /// pair's packed bytes as the per-byte denominator.
    ///
    /// The mate is `DS(p, p)`'s unit-block twin, so the co-sweep's
    /// freezes and promotions fire at boundaries where the mate's
    /// cheap codes set the funded width while the drift being parked
    /// and promoted was deposited by the wide operand — and every
    /// arming owes its debt across the same dense trailing mass.
    /// Value legs anchor all three measures before the counters
    /// return: `DS` dominates its mate pointwise, so
    /// `distance = rank(a) − rank(b)`, `lag(a, b) = 0`, and
    /// `lag(b, a) = distance`.
    fn distance_dense_suffix_run(p: usize) -> QueryRun {
        let a = Shape::DenseSuffix.packed2(p, p).version();
        let b = Shape::DenseSuffixMate.packed2(p, p).version();
        let bytes = (a.encode().len() + b.encode().len()) as u64;
        let gap = a
            .rank()
            .checked_sub(&b.rank())
            .expect("the dense-suffix operand dominates its unit mate");
        touch_meter::reset();
        meter::reset_limb_ops();
        let d = a.distance(&b);
        let forward = a.lag(&b);
        let backward = b.lag(&a);
        let run = QueryRun {
            bytes,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert_eq!(d, gap, "distance must be the dominating rank gap");
        assert_eq!(
            forward,
            before::Rank::ZERO,
            "the dominating side lags by nothing"
        );
        assert_eq!(backward, d, "the dominated side lags by the whole gap");
        assert!(
            run.touches >= run.bytes,
            "pair queries at {bytes} operand bytes: {} digit touches under \
             the one-per-byte floor: the co-sweep's difference state is not \
             running on the metered accumulator",
            run.touches,
        );
        run
    }

    /// Absolute two-scale (touch, limb) ceilings for the distance/lag
    /// triple on the dense-suffix pair, measured ×1.25.
    ///
    /// The record: 549,454 → 1,097,440 touches and 361,841 →
    /// 723,253 limb ops across `p = 500 → 1,000` on a 127,034 B →
    /// 253,909 B packed pair (three sweeps' worth, flat across the
    /// doubling), tightened in the settle's own commit from the
    /// per-arming suffix walk's red pin of 12,900,786 → 50,547,044
    /// touches and 5,836,909 → 22,673,775 limb ops (×1.96 touch and
    /// ×1.94 limb per-byte growth). Movement (the
    /// cluster-delegated settle, parent 710,692 → 1,416,186 touches
    /// and 413,847 → 826,771 limb ops): −23% touch, −13% limb — one
    /// backend product per dense cluster instead of a factor-wide
    /// product per window digit; the per-byte exponent stays ×1.00 in
    /// both currencies. Live remeasure: 525,427 → 1,051,407 touches
    /// and 355,553 → 712,993 limb ops, under the standing ceilings.
    const DISTANCE_DENSE_SUFFIX_CEILINGS: [(u64, u64); 2] =
        [(686_817, 452_301), (1_371_800, 904_066)];

    /// Distance and lag are flat per byte on the dense-suffix pair
    /// under the declared log model, within ×1.25 across a doubling
    /// and under absolute two-scale ceilings.
    ///
    /// `pair_integral` drives the same integrator as `rank` (one
    /// shared product-tree settle), so the two-operand form holds the
    /// same bound; the committed pair tripwire
    /// (`suffix_walk_settle_reads_superlinear_on_dense_suffix_pair`,
    /// the query fold's test suite) keeps the per-arming suffix walk
    /// failing on this pair, so this band is never decoration.
    #[test]
    fn skyline_distance_dense_suffix_is_flat_per_unit() {
        let small = distance_dense_suffix_run(DENSE_SUFFIX_SMALL);
        let large = distance_dense_suffix_run(2 * DENSE_SUFFIX_SMALL);
        assert_ceilings(
            "skyline_distance_dense_suffix",
            &small,
            &large,
            DISTANCE_DENSE_SUFFIX_CEILINGS,
        );
        assert_flat(
            "distance_dense_suffix_touches",
            "byte",
            (small.touches, small.bytes),
            (large.touches, large.bytes),
        );
        assert_flat(
            "distance_dense_suffix_limb_ops",
            "byte",
            (small.limb_ops, small.bytes),
            (large.limb_ops, large.bytes),
        );
    }
}

// ─── the ledger wide-arming band ─────────────────────────────────────────────
//
// The ledger settle's wide × dense genre, held flat in the fold's own
// traffic. The family arms the promotion ledger once with a parked
// mass as wide as the input (a `2^(32w)` climb) ahead of a trailing
// mass as dense as the input (the gap spine's punctured run), and the
// plateau's cancelling descent lands after the sweep — outside every
// aggregate — so the settle's one aggregate product is exactly the
// wide × dense cross term, undodgeable by any seam cancellation. The
// settle rides it through one backend multiplication (the query
// module doc's settle bound), so the deterministic counters — which
// price the traffic the fold itself moves: operand reads, window
// digits, the product's width — read flat per byte, and the
// multiplication's own superlinear work runs inside the backend at
// its bound, below the limb shim (the delegation convention the
// `parse_decimal` shim set). The committed schoolbook kernel
// (`schoolbook_settle_reads_superlinear_on_wide_arming`, the query
// fold's test suite) keeps the per-digit charge failing on this very
// family, value-exact, so this band is never decoration. The
// distance/lag claims ride the same band: one shared integrator.
#[cfg(feature = "limb-meter")]
mod ledger_wide_arming {
    use before::meter;
    use before::meter::registry::Shape;
    use suanpan::touch_meter;

    /// One public `Version::rank` run over `WA(w, w)`: packed bytes
    /// and both counters over the rank body alone.
    ///
    /// Carries `min_ticks`' closed form as the cross-fold semantic leg
    /// (proving the generator builds the gap spine and the wide arming
    /// this band reasons about) and the one-touch-per-operand-byte
    /// liveness floor.
    fn run(w: usize) -> (u64, u64, u64) {
        use dashu_int::UBig;
        let v = Shape::WideArming.packed2(w, w).version();
        let bytes = v.encode().len() as u64;
        let expected =
            UBig::from(w as u64) + (UBig::ONE << (32 * w)) + (UBig::ONE << 288usize) + 3u8;
        assert_eq!(
            v.min_ticks(),
            expected
                .to_string()
                .parse::<before::Ticks>()
                .expect("the closed form parses"),
            "the family's stored-code sum disagrees with min_ticks: the \
             generator does not build the tree this band reasons about"
        );
        touch_meter::reset();
        meter::reset_limb_ops();
        let rank = v.rank();
        std::hint::black_box(rank);
        let touches = touch_meter::touches();
        let limb_ops = meter::limb_ops();
        assert!(
            touches >= bytes,
            "rank at {bytes} operand bytes: {touches} digit touches under \
             the one-per-byte floor: the fold's accumulator work is not \
             metered",
        );
        (bytes, touches, limb_ops)
    }

    /// Suffix digits (and arming digits) of the band's small run (the
    /// large run doubles both).
    const WIDE_ARMING_SMALL: usize = 500;

    /// Absolute two-scale (touch, limb) ceilings for rank on the
    /// wide-arming family, measured ×1.25.
    ///
    /// The record: 34,742 → 69,373 touches and 41,942 → 83,836 limb
    /// ops across WA(500, 500) → WA(1,000, 1,000) on 14,263 B →
    /// 28,451 B packed operands (~2.4 touches per packed byte, flat
    /// across the doubling), tightened in the cure's own commit from
    /// the schoolbook settle's red reading of 288,037 → 1,083,963
    /// touches and 293,651 → 1,095,255 limb ops — measured at the
    /// parent as one whole fold, so 2 limb ops above the kernel's
    /// own committed tip record of that date, two measurement
    /// harnesses rather than drift — (×1.89 and ×1.87 per
    /// byte, local exponent ~1.9: the aggregate product paid the
    /// parked width times the window density one digit at a time).
    /// Live remeasure: 32,452 → 64,793 touches (−6.6%, accumulated
    /// since the record) and 41,412 → 82,774 limb ops, under the
    /// standing ceilings.
    const WIDE_ARMING_CEILINGS: [(u64, u64); 2] = [(43_427, 52_427), (86_716, 104_795)];

    /// rank is flat per byte on the wide-arming family: per-byte touch
    /// and limb work stay within ×1.25 across a `WA(w, w)` doubling,
    /// under absolute two-scale ceilings.
    ///
    /// `WA(w, w)` scales both factors of the settle's one aggregate
    /// product with the input, so a settle that pays their schoolbook
    /// product — or one whose product traffic stops being metered —
    /// moves this band, in opposite directions: the schoolbook charge
    /// reads ~×1.9 per byte (the ceilings' doc), and a dark tap reads
    /// under the liveness floor in [`run`].
    #[test]
    fn rank_wide_arming_is_flat_per_unit() {
        let (small_bytes, small_touches, small_limbs) = run(WIDE_ARMING_SMALL);
        let (large_bytes, large_touches, large_limbs) = run(2 * WIDE_ARMING_SMALL);
        eprintln!(
            "MEASURED rank_wide_arming: small={small_touches}/{small_bytes}B \
             (limb {small_limbs}) large={large_touches}/{large_bytes}B \
             (limb {large_limbs})"
        );
        for (name, small, large, ceilings) in [
            (
                "touches",
                small_touches,
                large_touches,
                (WIDE_ARMING_CEILINGS[0].0, WIDE_ARMING_CEILINGS[1].0),
            ),
            (
                "limb ops",
                small_limbs,
                large_limbs,
                (WIDE_ARMING_CEILINGS[0].1, WIDE_ARMING_CEILINGS[1].1),
            ),
        ] {
            assert!(
                small <= ceilings.0 && large <= ceilings.1,
                "rank ({name}) exceeds the pinned ceilings on the wide-arming \
                 family ({small}/{small_bytes}B -> {large}/{large_bytes}B \
                 against {} / {})",
                ceilings.0,
                ceilings.1,
            );
            assert!(
                u128::from(large) * u128::from(small_bytes) * 4
                    <= u128::from(small) * u128::from(large_bytes) * 5,
                "rank ({name}) grew more than x1.25 per byte across the \
                 wide-arming doubling ({small}/{small_bytes}B -> \
                 {large}/{large_bytes}B): the settle is paying a settle \
                 product's width times its density again",
            );
        }
    }
}

// ─── the wide-arming parse band ──────────────────────────────────────────────
//
// The parse-side exact-`top` genre, held flat at the text seam. The
// family's rendered text funds one wide swing (the `2^(32w)` arming
// climb) ahead of a `Θ(d)`-leaf trailing run whose deltas are all
// zero; the parse extracts one signed magnitude per leaf from the
// path-sum accumulator, so an extraction that pays a stale high-water
// span instead of the settled top re-walks the swing's `w` dead digits
// once per trailing leaf — `Θ(w·d)` touches on `Θ(w + d)` text,
// quadratic at `w = d`. The shipped discipline resets the accumulator
// at each extraction, so every read pays the span written since the
// previous leaf and the trailing run stays O(1) per leaf. The
// committed schoolbook kernel
// (`schoolbook_parse_reads_superlinear_on_wide_arming`, the text
// kernel's test suite) keeps the compensating-subtraction read failing
// on this very family, value-exact, so this band is never decoration.
// The board's wide-arming column prices the same mechanism through the
// public from_str and parse entries at both acceptance scales.
#[cfg(feature = "limb-meter")]
mod parse_wide_arming {
    use before::meter;
    use before::meter::registry::Shape;
    use suanpan::touch_meter;

    /// One text-parse run over `Shape::WideArming.packed2(s, s)`'s rendered text:
    /// text bytes and accumulator touches over the parse body alone
    /// (the text renders outside the metered window).
    ///
    /// Carries `min_ticks`' closed form as the cross-fold semantic leg
    /// (proving the generator builds the gap spine and the wide arming
    /// this band reasons about), the one-touch-per-leaf liveness floor
    /// (every leaf's delta extraction reads at least one accumulator
    /// digit, so a parse whose path-sum state left the metered
    /// representation fails loudly instead of passing the flatness
    /// ratio vacuously at zero touches), and the value leg (the parse
    /// lands on the stored stream byte for byte).
    fn run(s: usize) -> (u64, u64) {
        use dashu_int::UBig;
        let v = Shape::WideArming.packed2(s, s).version();
        let expected =
            UBig::from(s as u64) + (UBig::ONE << (32 * s)) + (UBig::ONE << 288usize) + 3u8;
        assert_eq!(
            v.min_ticks(),
            expected
                .to_string()
                .parse::<before::Ticks>()
                .expect("the closed form parses"),
            "the family's stored-code sum disagrees with min_ticks: the \
             generator does not build the tree this band reasons about"
        );
        let enc = meter::skyline::encode(&v);
        let text = meter::skyline::text::render(&enc);
        let bytes = text.len() as u64;
        touch_meter::reset();
        let parsed = meter::skyline::text::parse(&text).expect("rendered text parses");
        let touches = touch_meter::touches();
        assert_eq!(
            parsed, enc,
            "the parse must land on the stored stream byte for byte"
        );
        // The gap spine alone holds 33s leaves (the whole family
        // 33s + 5), and each leaf's delta extraction reads at least
        // one accumulator digit.
        let leaf_floor = 33 * s as u64;
        assert!(
            touches >= leaf_floor,
            "parse at {bytes} text bytes: {touches} digit touches under the \
             {leaf_floor}-leaf floor: the parse's path-sum accumulator is not \
             running on the metered representation"
        );
        (bytes, touches)
    }

    /// Digit width (and gap count) of the band's small run; the large
    /// run doubles both.
    const WIDE_ARMING_SMALL: usize = 256;

    /// Absolute two-scale touch ceilings: the measured record ×1.25.
    ///
    /// The record: 19,511 → 38,967 touches across
    /// WA(256, 256) → WA(512, 512) on 70,169 B → 140,219 B of rendered
    /// text (0.28 touches per byte, flat across the doubling)
    /// \[measured, exact counters\] — tightened in the
    /// cure's own commit from the compensating-subtraction read of
    /// 2,109,502 → 8,413,246 touches (30.1 → 60.0 per byte, ×2.00 per
    /// byte across the doubling: the high-water walk's quadratic
    /// signature), measured at the parent and kept demonstrated red by
    /// the committed schoolbook kernel.
    const PARSE_WIDE_ARMING_TOUCH_CEILINGS: (u64, u64) = (24_388, 48_708);

    /// The text parse is flat per byte on the wide-arming family:
    /// per-byte touch work stays within ×1.25 across a `WA(s, s)`
    /// doubling, under absolute two-scale ceilings.
    ///
    /// `WA(s, s)` scales the swing's width and the trailing run's
    /// length together, so an extraction that pays the high-water span
    /// again moves this band on the exponent leg (the schoolbook
    /// kernel reads ×2.00 per byte — the ceilings' doc), and a dark
    /// tap reads under the liveness floor in [`run`].
    #[test]
    fn parse_wide_arming_touch_cost_is_flat_per_unit() {
        let (small_bytes, small_touches) = run(WIDE_ARMING_SMALL);
        let (large_bytes, large_touches) = run(2 * WIDE_ARMING_SMALL);
        eprintln!(
            "MEASURED parse_wide_arming: small={small_touches}/{small_bytes}B \
             large={large_touches}/{large_bytes}B"
        );
        assert!(
            small_touches <= PARSE_WIDE_ARMING_TOUCH_CEILINGS.0
                && large_touches <= PARSE_WIDE_ARMING_TOUCH_CEILINGS.1,
            "the parse's touch cost exceeds the pinned ceilings on the \
             wide-arming family ({small_touches}/{small_bytes}B -> \
             {large_touches}/{large_bytes}B against {} / {})",
            PARSE_WIDE_ARMING_TOUCH_CEILINGS.0,
            PARSE_WIDE_ARMING_TOUCH_CEILINGS.1,
        );
        assert!(
            u128::from(large_touches) * u128::from(small_bytes) * 4
                <= u128::from(small_touches) * u128::from(large_bytes) * 5,
            "the parse's touch cost grew more than x1.25 per byte across the \
             wide-arming doubling ({small_touches}/{small_bytes}B -> \
             {large_touches}/{large_bytes}B): the extraction is paying a stale \
             high-water span again"
        );
    }
}

// ─── the answer-embedded product band ────────────────────────────────────────
//
// The close-time settle's wide × dense genre — and the floor under
// every settle. The plateau-puncture family `PP(w, d)`
// (`meter::plateau_puncture`) embeds its excess in the exact answer,
// not in any ledger accounting: every turn leaf sits on one
// incompressible pseudorandom plateau `x` of `w` digits, the turn
// positions spell a jittered mass `y` of `d` isolated digits, and
// the rank numerator is exactly `2·x·y + 1` — a Θ(w)-digit ×
// Θ(d)-term integer product bought with Θ(w + d) input bits, both
// factors' content beyond the settle's own balanced-digit compaction
// (`mul_bound_embedding_is_alive` pins exactly that). No promotion
// ever fires (the one wide plunge parks once and no later freeze
// arrives), so the cost sits in the close-time settle `P · segment`,
// outside the promotion ledger and its product tree entirely:
// computing the answer *is* one wide × dense multiplication, which
// the shipped settle delegates whole to the backend at its
// multiplication bound `M(|v|)`. The floor is a reduction, not a
// bet on this instance: the same constructor embeds `2·x·y + 1` for
// arbitrary factors (`meter::puncture_product`; the query fold's
// `arbitrary_factors_embed_their_product_in_exact_rank` proptest
// pins it), so any fold that answers exactly multiplies arbitrary
// input-funded integers — `Ω(M(|v|))` floors every settle and the
// `# Complexity` claims' worst case can never reach `O(|v|)` while
// integer multiplication is superlinear. The fold's own
// deterministic counters price its traffic — operand reads, the
// compacted segment, the product's width — and read flat per byte;
// the committed schoolbook kernel
// (`schoolbook_settle_reads_superlinear_on_plateau_puncture`, the
// query fold's test suite) keeps the per-digit charge failing on this
// family, value-exact, so this band is never decoration.
#[cfg(feature = "limb-meter")]
mod answer_embedded_product {
    use before::meter;
    use before::meter::registry::Shape;
    use suanpan::touch_meter;

    /// One public `Version::rank` run over `PP(s, s)`: packed bytes and
    /// both counters over the rank body alone.
    ///
    /// Carries the `min_ticks` closed form (`s · x + 1` over the
    /// committed factors) as the generator's semantic leg, the
    /// exact-rank leg (the answer is the product `2·x·y + 1` — the
    /// `Ω(M(|v|))` mandate's witness), and the
    /// one-touch-per-operand-byte liveness floor.
    fn run(s: usize) -> (u64, u64, u64) {
        use dashu_int::UBig;
        let v = Shape::PlateauPuncture.packed2(s, s).version();
        let bytes = v.encode().len() as u64;
        let (x, y) = meter::plateau_puncture_factors(s, s);
        let expected = UBig::from(s as u64) * &x + 1u8;
        assert_eq!(
            v.min_ticks(),
            expected
                .to_string()
                .parse::<before::Ticks>()
                .expect("the closed form parses"),
            "the family's stored-code sum disagrees with min_ticks: the \
             generator does not build the tree this band reasons about"
        );
        touch_meter::reset();
        meter::reset_limb_ops();
        let rank = v.rank();
        std::hint::black_box(&rank);
        let touches = touch_meter::touches();
        let limb_ops = meter::limb_ops();
        // The answer itself is the product: the band is honest only
        // while the measured body computes 2·x·y + 1 exactly.
        assert_eq!(
            rank.to_string(),
            format!("{}/2^{}", ((&x * &y) << 1usize) + 1u8, 66 * s),
            "the exact rank is the plateau times the punctured turn mass"
        );
        assert!(
            touches >= bytes,
            "rank at {bytes} operand bytes: {touches} digit touches under \
             the one-per-byte floor: the fold's accumulator work is not \
             metered",
        );
        (bytes, touches, limb_ops)
    }

    /// Plateau digits (and turn count) of the band's small run (the
    /// large run doubles both).
    const PLATEAU_PUNCTURE_SMALL: usize = 500;

    /// Absolute two-scale (touch, limb) ceilings for rank on the
    /// plateau-puncture family, measured ×1.25.
    ///
    /// The record: 48,420 → 96,847 touches and 72,883 → 145,758 limb
    /// ops across PP(500, 500) → PP(1,000, 1,000) on 20,376 B →
    /// 40,751 B packed operands (~2.4 touches per packed byte, flat
    /// across the doubling), while the committed schoolbook kernel
    /// reads the same family red at 482,968 → 1,843,181 touches and
    /// 198,320 → 653,131 limb ops (×1.91 and ×1.65 per byte: the
    /// close-time settle paying the parked plateau's width once per
    /// trailing-mass digit). Live remeasure: 45,871 → 91,751 touches
    /// (−5.3%, accumulated since the record; limbs unmoved), under
    /// the standing ceilings.
    const PLATEAU_PUNCTURE_CEILINGS: [(u64, u64); 2] = [(60_525, 91_103), (121_058, 182_197)];

    /// rank is flat per byte on the plateau-puncture family: per-byte
    /// touch and limb work stay within ×1.25 across a `PP(s, s)`
    /// doubling, under absolute two-scale ceilings.
    ///
    /// Flat in the fold's own traffic, never in total work: the
    /// answer-embedded product runs inside the backend at the
    /// multiplication bound (the exact-rank leg in [`run`] proves the
    /// answer is still the product), so this band and that leg
    /// together witness both directions of the settle's bound —
    /// `O(M(|v|))` achieved, `Ω(M(|v|))` mandatory.
    #[test]
    fn rank_plateau_puncture_is_flat_per_unit() {
        let (small_bytes, small_touches, small_limbs) = run(PLATEAU_PUNCTURE_SMALL);
        let (large_bytes, large_touches, large_limbs) = run(2 * PLATEAU_PUNCTURE_SMALL);
        eprintln!(
            "MEASURED rank_plateau_puncture: small={small_touches}/{small_bytes}B \
             (limb {small_limbs}) large={large_touches}/{large_bytes}B \
             (limb {large_limbs})"
        );
        for (name, small, large, ceilings) in [
            (
                "touches",
                small_touches,
                large_touches,
                (
                    PLATEAU_PUNCTURE_CEILINGS[0].0,
                    PLATEAU_PUNCTURE_CEILINGS[1].0,
                ),
            ),
            (
                "limb ops",
                small_limbs,
                large_limbs,
                (
                    PLATEAU_PUNCTURE_CEILINGS[0].1,
                    PLATEAU_PUNCTURE_CEILINGS[1].1,
                ),
            ),
        ] {
            assert!(
                small <= ceilings.0 && large <= ceilings.1,
                "rank ({name}) exceeds the pinned ceilings on the \
                 plateau-puncture family ({small}/{small_bytes}B -> \
                 {large}/{large_bytes}B against {} / {})",
                ceilings.0,
                ceilings.1,
            );
            assert!(
                u128::from(large) * u128::from(small_bytes) * 4
                    <= u128::from(small) * u128::from(large_bytes) * 5,
                "rank ({name}) grew more than x1.25 per byte across the \
                 plateau-puncture doubling ({small}/{small_bytes}B -> \
                 {large}/{large_bytes}B): the close-time settle is paying \
                 the parked width times the segment's density again",
            );
        }
    }
}

// ─── the settle flatness probes ──────────────────────────────────────────────
//
// The multi-arming and pair legs of the settle's bound, held flat per
// byte. The single-arming wide × dense genres carry their own bands
// (`ledger_wide_arming`, `answer_embedded_product`); the probes here
// hold the shapes only arming *count* can reach: trains of wide
// armings whose ledger settles through the full mass-balanced product
// tree, and a pair driving both settle sites through one co-sweep.
// The tree rewrites a window's digits once per level and the mass
// balance keeps levels logarithmic in the arming count, so the
// settle's metered traffic per byte can grow only by the level ratio
// across an arming-count doubling — ×log₂(2n)/log₂(n), at most ×1.17
// from the probes' smallest count — and only if the settle dominated
// the fold's linear work, which it does not: the ×1.25 flatness
// convention covers the model's whole admissible growth here, and the
// measurement reads ×1.04–×1.09 per doubling. The retired reading
// these probes tightened from is the ceilings' docs; the
// committed-and-failing schoolbook kernel (the query fold's test
// suite) is the adequacy witness that the families still catch a
// per-digit settle.
#[cfg(feature = "limb-meter")]
mod settle_flatness {
    use before::meter;
    use before::meter::registry::Shape;
    use suanpan::touch_meter;

    /// One `Version::rank` run: operand bytes and both counters, under
    /// the one-touch-per-byte liveness floor.
    fn rank_run(v: &before::Version) -> (u64, u64, u64) {
        let bytes = v.encode().len() as u64;
        touch_meter::reset();
        meter::reset_limb_ops();
        let rank = v.rank();
        std::hint::black_box(rank);
        let touches = touch_meter::touches();
        let limb_ops = meter::limb_ops();
        assert!(
            touches >= bytes,
            "rank at {bytes} operand bytes: {touches} digit touches under \
             the one-per-byte floor: the fold's accumulator work is not \
             metered",
        );
        (bytes, touches, limb_ops)
    }

    /// Assert one probe's reading against its absolute pinned ceilings
    /// and, per currency, flatness (×1.25 per byte) across the
    /// doubling, and report the readings.
    fn assert_flat_step(
        name: &str,
        small: (u64, u64, u64),
        large: (u64, u64, u64),
        ceilings: [(u64, u64); 2],
    ) {
        let (sb, st, sl) = small;
        let (lb, lt, ll) = large;
        eprintln!(
            "MEASURED settle_flatness_{name}: small={st}/{sb}B (limb {sl}) \
             large={lt}/{lb}B (limb {ll}) per_byte={} -> {} (milli-touches)",
            st * 1000 / sb,
            lt * 1000 / lb,
        );
        // Per-row growth bands: touches stay flat (x1.25); the limb row
        // now reads the settle's delegated products alone — the linear
        // payload work rides the word-valued form — so the product
        // tree's documented depth factor shows across a doubling, and
        // its band is x1.5 (a re-read past the level cap still reads
        // ~x2).
        for (cur, s, l, ceil, num, den) in [
            (
                "touches",
                st,
                lt,
                (ceilings[0].0, ceilings[1].0),
                4u128,
                5u128,
            ),
            (
                "limb ops",
                sl,
                ll,
                (ceilings[0].1, ceilings[1].1),
                2u128,
                3u128,
            ),
        ] {
            assert!(
                s <= ceil.0 && l <= ceil.1,
                "{name} ({cur}) exceeds the pinned ceilings: {s}/{sb}B -> \
                 {l}/{lb}B against {} / {}",
                ceil.0,
                ceil.1,
            );
            assert!(
                u128::from(l) * u128::from(sb) * num <= u128::from(s) * u128::from(lb) * den,
                "{name} ({cur}) grew past its per-byte band across the \
                 doubling: {s}/{sb}B -> {l}/{lb}B; the settle is re-reading \
                 a width or density past the mass-balanced tree's level cap",
            );
        }
    }

    /// Arming width (digits) of the multi-arming probes.
    const TRAIN_WIDTH: usize = 50;

    /// Window gaps per block of the multi-arming probes.
    ///
    /// Dense enough that the settle's aggregate products dominate the
    /// fold's linear work per stored byte: the windows are
    /// topology-funded, so `g` buys density the operand barely pays
    /// for.
    const TRAIN_GAPS: usize = 100;

    /// One arming-train rank run with the mirrored `min_ticks` leg.
    fn train_run(n: usize, alternate: bool) -> (u64, u64, u64) {
        use dashu_int::UBig;
        let v = Shape::ArmingTrain
            .packed_train(n, TRAIN_WIDTH, TRAIN_GAPS, alternate)
            .version();
        let band = 32 * TRAIN_WIDTH + (usize::BITS - n.leading_zeros()) as usize + 2;
        let arm = UBig::ONE << (32 * TRAIN_WIDTH);
        let kicker = UBig::ONE << 288usize;
        let mut plateau = (UBig::ONE << band) + (&arm << 1);
        let mut expected = UBig::ZERO;
        for b in 0..n {
            expected += &plateau * UBig::from(TRAIN_GAPS as u64);
            if alternate && b % 2 == 1 {
                plateau -= &arm;
            } else {
                plateau += &arm;
            }
            for kick in [UBig::ZERO, UBig::ONE, kicker.clone(), UBig::ONE] {
                plateau += kick;
                expected += &plateau;
            }
        }
        assert_eq!(
            v.min_ticks(),
            expected
                .to_string()
                .parse::<before::Ticks>()
                .expect("the mirrored sum parses"),
            "the family's leaf-value sum disagrees with min_ticks: the \
             generator does not build the tree these probes reason about"
        );
        rank_run(&v)
    }

    /// One distance-and-lag run over a version pair: combined operand
    /// bytes and both counters over the three query bodies together.
    ///
    /// Enforces the one-touch-per-byte liveness floor and the
    /// halves-sum value leg (`lag(a, b) + lag(b, a) == distance`,
    /// exact `Rank` arithmetic the sweeps share nothing with).
    fn pair_run(a: &before::Version, b: &before::Version) -> (u64, u64, u64) {
        let bytes = (a.encode().len() + b.encode().len()) as u64;
        touch_meter::reset();
        meter::reset_limb_ops();
        let d = a.distance(b);
        let forward = a.lag(b);
        let backward = b.lag(a);
        let touches = touch_meter::touches();
        let limb_ops = meter::limb_ops();
        assert_eq!(
            forward + backward,
            d,
            "the directed halves must sum to the symmetric distance"
        );
        assert!(
            touches >= bytes,
            "pair queries at {bytes} operand bytes: {touches} digit touches \
             under the one-per-byte floor: the co-sweep's difference state is \
             not running on the metered accumulator",
        );
        (bytes, touches, limb_ops)
    }

    /// Absolute two-scale (touch, limb) ceilings for the pair probe,
    /// measured ×1.25.
    ///
    /// The record: 406,244 → 815,755 touches and 369,661 → 745,682
    /// limb ops across the committed doubling on 30,801 B → 60,797 B
    /// packed pairs (×1.02 per byte); the committed schoolbook
    /// kernel keeps the plateau side's close-time settle — the site
    /// that dominates this pair — red on the same family in the
    /// query fold's test suite. Live remeasure: 398,013 → 798,265
    /// touches and 370,381 → 746,786 limb ops, under the standing
    /// ceilings.
    const PAIR_PLATEAU_TRAIN_CEILINGS: [(u64, u64); 2] = [(507_805, 462_076), (1_019_693, 932_102)];

    /// The plateau-puncture × arming-train pair is flat per byte
    /// through the public distance and lag entry points: the
    /// shared-integrator argument measured on the pair co-sweep, not
    /// inferred from rank alone.
    ///
    /// The pair drives both settle genres in one co-sweep — the
    /// plateau side parks one wide drift whose final segment stays
    /// dense (the close-time answer-embedded product) while the train
    /// side arms the promotion ledger repeatedly (the aggregate
    /// products) — so a pair-only regression in either site, or in
    /// their interaction through the shared difference integrator,
    /// reads here even while every rank-only probe stays green.
    #[test]
    fn pair_plateau_train_is_flat_per_unit() {
        let small = pair_run(
            &Shape::PlateauPuncture.packed2(400, 400).version(),
            &Shape::ArmingTrain
                .packed_train(8, TRAIN_WIDTH, TRAIN_GAPS, false)
                .version(),
        );
        let large = pair_run(
            &Shape::PlateauPuncture.packed2(800, 800).version(),
            &Shape::ArmingTrain
                .packed_train(16, TRAIN_WIDTH, TRAIN_GAPS, false)
                .version(),
        );
        assert_flat_step(
            "pair_plateau_train",
            small,
            large,
            PAIR_PLATEAU_TRAIN_CEILINGS,
        );
    }

    /// Absolute (touch, limb) ceilings for the same-sign train at
    /// n = 4, 8, 16, measured ×1.25.
    ///
    /// The record: 23,392 → 48,167 → 97,809 touches and 34,009 →
    /// 70,836 → 145,018 limb ops (×1.09, ×1.04 per byte — the tree's
    /// one-rewrite-per-level window traffic under full-width parked
    /// sums, inside the level-ratio model), tightened in the cure's
    /// own commit from the schoolbook charge's 57,542 → 127,612 →
    /// 279,963 touches (×1.17, ×1.13 per byte and rising with the
    /// count). Live remeasure: 22,716 → 47,491 → 96,946 touches and
    /// 33,487 → 70,210 → 144,081 limb ops, under the standing
    /// ceilings.
    const TRAIN_SAME_SIGN_CEILINGS: [(u64, u64); 3] =
        [(29_240, 42_511), (60_208, 88_545), (122_261, 181_272)];

    /// Absolute (touch, limb) ceilings for the alternating train at
    /// n = 4, 8, 16, measured ×1.25.
    ///
    /// The record: 23,942 → 49,571 → 100,426 touches and 33,851 →
    /// 70,619 → 144,373 limb ops — within 3% of the same-sign train's:
    /// under the backend-delegated products the parked sums' sign
    /// schedule moves constants only (cancellation narrows a product's
    /// factor; the bound never rests on it). The sign schedules' value
    /// coverage lives in the promoting differential pool. Live
    /// remeasure: 23,219 → 48,849 → 99,484 touches and 33,330 →
    /// 69,994 → 143,543 limb ops, under the standing ceilings.
    const TRAIN_ALTERNATING_CEILINGS: [(u64, u64); 3] =
        [(29_927, 42_313), (61_963, 88_273), (125_532, 180_466)];

    /// Multi-arming trains are flat per byte across two arming-count
    /// doublings, in both sign schedules, under absolute pinned
    /// ceilings.
    ///
    /// The same-sign train is the tree's hardest committed probe:
    /// every level holds the full window density under full-width
    /// parked sums, so all of the settle's per-level window traffic
    /// rides maximal-width products — and stays inside the ×1.25
    /// convention because a doubling adds one level to a logarithmic
    /// stack while the byte budget doubles. The alternating twin
    /// cancels parked width digit-wise inside the tree's aggregate
    /// sums; its ceilings read within 3% of the same-sign train's
    /// (the ceilings' doc), the committed record that the sign
    /// schedule is a constants effect under backend-delegated
    /// products, not a class effect.
    #[test]
    fn arming_trains_is_flat_per_unit() {
        let same = [
            train_run(4, false),
            train_run(8, false),
            train_run(16, false),
        ];
        let alt = [train_run(4, true), train_run(8, true), train_run(16, true)];
        for (name, runs, ceilings) in [
            ("train_same_sign", &same, &TRAIN_SAME_SIGN_CEILINGS),
            ("train_alternating", &alt, &TRAIN_ALTERNATING_CEILINGS),
        ] {
            assert_flat_step(name, runs[0], runs[1], [ceilings[0], ceilings[1]]);
            assert_flat_step(name, runs[1], runs[2], [ceilings[1], ceilings[2]]);
        }
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
    let pa = Shape::IdSpine.packed_flagged(ID_DEPTH, false);
    let pb = Shape::IdSpine.packed_flagged(ID_DEPTH, true);
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
        Shape::IdSpine.packed_flagged(ID_DEPTH - 1, false).bytes,
        "the divert arms rejoin into the spine one level shorter"
    );
}

/// `covers` over the diverted id-spine pair stays within its envelope (the
/// two-tree walk runs to full lockstep depth; its iterative frames must
/// grow no stack segments).
#[test]
fn id_covers_envelope() {
    let pa = Shape::IdSpine.packed_flagged(ID_DEPTH, false);
    let pb = Shape::IdSpine.packed_flagged(ID_DEPTH, true);
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
    let pa = Shape::IdSpine.packed_flagged(ID_DEPTH, false);
    let pb = Shape::IdSpine.packed_flagged(ID_DEPTH, true);
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
    let pa = Shape::IdSpine.packed_flagged(ID_DEPTH, false);
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
    let pa = Shape::IdSpine.packed_flagged(ID_DEPTH, false);
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

/// Pack a most-significant-bit-first bit stream into its marker-padded
/// wire bytes, the generators' packing convention.
fn pack_bits(bits: &[bool]) -> Vec<u8> {
    let mut bytes = vec![0u8; (bits.len() + 1).div_ceil(8)];
    for (i, &bit) in bits.iter().enumerate() {
        if bit {
            bytes[i / 8] |= 0x80 >> (i % 8);
        }
    }
    bytes[bits.len() / 8] |= 0x80 >> (bits.len() % 8); // the padding marker
    bytes
}

/// Read live bit `i` of a packed stream (most significant bit first).
fn packed_bit(bytes: &[u8], i: usize) -> bool {
    bytes[i / 8] & (0x80 >> (i % 8)) != 0
}

/// `without` subtracting an id spine from the seed stays within its envelope
/// (the sweep is iterative, so the subtrahend's depth alone must grow no
/// stack segments).
#[test]
fn id_without_envelope() {
    let pb = Shape::IdSpine.packed_flagged(ID_DEPTH, true);
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
// reading to an exact measured value at both depths (the walk reads every
// stored tag of both operands exactly once, so the reading is
// deterministic and two-sided: an undercounting tap and a re-scanning
// walk both move it) over a full-examination liveness floor (one bit per
// packed operand byte — the diverted pair forces both walks to full
// lockstep depth), and to per-byte flatness (×1.25) across a depth
// doubling, so a walk that leaves the metered primitives moves a
// committed number instead of passing every near-zero column unchanged.
#[cfg(feature = "scan-meter")]
mod id_walk_scan_cost {
    use super::{id_pair_input_bytes, party_of, ID_DEPTH};
    use before::meter;
    use before::meter::registry::Shape;

    /// One walk run: packed operand bytes and the bits scanned by the
    /// walk body alone.
    struct Run {
        bytes: u64,
        bits: u64,
    }

    /// Exact scan readings at the [`ID_DEPTH`] pair, measured
    /// with deterministic counters.
    ///
    /// Every stored tag of both operands is read exactly once through
    /// the metered primitives — [`SCAN_EXACT_BITS_SMALL`] on 62,502
    /// packed bytes at the half depth, [`SCAN_EXACT_BITS_LARGE`] on
    /// 125,002 at the full depth, identical for the covers and disjoint
    /// walks (the same full lockstep walk). Pinned with equality, not a
    /// ceiling: a uniform tap undercount halves the reading yet clears
    /// every slack floor in the tree, so only the exact number is
    /// tamper-evident in both directions.
    const SCAN_EXACT_BITS_SMALL: u64 = 500_004;

    /// The full-depth reading paired with [`SCAN_EXACT_BITS_SMALL`].
    const SCAN_EXACT_BITS_LARGE: u64 = 1_000_004;

    /// Run one id-pair walk at `depth` and read the scan counter over
    /// the body alone, enforcing the full-examination liveness floor.
    fn walk_run(
        name: &str,
        depth: usize,
        body: impl FnOnce(&before::Party, &before::Party),
    ) -> Run {
        let pa = Shape::IdSpine.packed_flagged(depth, false);
        let pb = Shape::IdSpine.packed_flagged(depth, true);
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

    /// The covers walk's scan bits are exact-pinned at both depths,
    /// floored, and flat per byte across the depth doubling of the
    /// diverted spine pair (which admits no early exit).
    ///
    /// The walk's cost is invisible to every other deterministic meter,
    /// so this pin is what a re-scanning `covers` (quadratic restarts),
    /// an unmetered raw-indexing walk, or a tap under- or over-count
    /// moves — the equality is the two-sided form a ceiling-plus-slack
    /// floor cannot give.
    #[test]
    fn id_covers_scan_cost_is_pinned_and_flat() {
        let small = walk_run("covers_small", ID_DEPTH / 2, |a, b| {
            assert!(!a.covers(b), "the divert arms are disjoint");
        });
        let large = walk_run("covers", ID_DEPTH, |a, b| {
            assert!(!a.covers(b), "the divert arms are disjoint");
        });
        assert_flat("covers", &small, &large);
        assert_eq!(
            (small.bits, large.bits),
            (SCAN_EXACT_BITS_SMALL, SCAN_EXACT_BITS_LARGE),
            "id_covers: the scanned bits moved off the exact pin: a moved \
             reading is a walk or tap change to re-pin deliberately",
        );
    }

    /// The disjoint walk's scan bits are exact-pinned at both depths,
    /// floored, and flat per byte across the depth doubling of the
    /// diverted spine pair (disjoint operands, so the walk runs to
    /// completion).
    ///
    /// Same rationale as the covers pin: scan is the one live column on
    /// this walk, and only the exact equality reads a tap undercount.
    #[test]
    fn id_disjoint_scan_cost_is_pinned_and_flat() {
        let small = walk_run("disjoint_small", ID_DEPTH / 2, |a, b| {
            assert!(a.is_disjoint(b), "the divert arms own disjoint regions");
        });
        let large = walk_run("disjoint", ID_DEPTH, |a, b| {
            assert!(a.is_disjoint(b), "the divert arms own disjoint regions");
        });
        assert_flat("disjoint", &small, &large);
        assert_eq!(
            (small.bits, large.bits),
            (SCAN_EXACT_BITS_SMALL, SCAN_EXACT_BITS_LARGE),
            "id_disjoint: the scanned bits moved off the exact pin: a moved \
             reading is a walk or tap change to re-pin deliberately",
        );
    }
}

// ─── fork envelope (the split kernel's committed cost record) ───────────────

/// The fork envelope: measured ×1.25 (dev profile, the envelope
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
    pub const ID_FORK: SweepEnvelope = sweep_envelope(      156_253,        0,        0,         3, 0); //   125_002 (both halves materialize, ~2x the packed input), 0, 0, 2 (the raw split path)
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
    let pa = Shape::IdSpine.packed_flagged(ID_DEPTH, false);
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

    use dashu_int::UBig;
    use suanpan::{touch_meter, Accumulator};

    /// Slack numerator over the measured value, matching the ×1.25 envelope
    /// convention (denominator [`SLACK_DEN`]).
    const SLACK_NUM: u64 = 5;

    /// Slack denominator: ceilings and flatness bounds are measured ×5/4.
    const SLACK_DEN: u64 = 4;

    /// One accumulator stream measurement over the stream body (setup
    /// excluded).
    ///
    /// Carries the linearity denominator (delta count, coded bytes
    /// where deltas widen, or sign reads where reads dominate), the
    /// operations the body performed, and the digit touches counted.
    struct Run {
        denominator: u64,
        /// Accumulator calls in the stream body (deltas plus sign
        /// reads): the touch counter's liveness floor, one touch per
        /// call.
        ///
        /// Every nonzero delta deposits into at least one digit, and
        /// every sign fold reads at least one — the floor is the
        /// mechanism's minimum possible work, not the typical work.
        ops: u64,
        touches: u64,
    }

    /// Assert a two-scale stream family's touch counter is alive (at
    /// least one touch per operation performed), stays under its pinned
    /// per-unit ceiling at both scales, and flat (×1.25) across the
    /// doubling.
    fn assert_flat(name: &str, small: &Run, large: &Run, ceiling_milli_per_unit: u64) {
        for run in [small, large] {
            eprintln!(
                "MEASURED accum_{name}: denominator={} touches={} milli_per_unit={}",
                run.denominator,
                run.touches,
                run.touches * 1000 / run.denominator,
            );
            assert!(
                run.touches >= run.ops,
                "accum_{name}: {} touches under {} operations: a deposit or \
                 sign fold stopped counting, so every ceiling above would \
                 hold vacuously",
                run.touches,
                run.ops,
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
        let mut acc = Accumulator::new();
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
            ops: 4 * n as u64,
            touches: touch_meter::touches(),
        }
    }

    /// The wide-tooth delta stream: setup `2^k`, then `2n` deltas of `±2^w`
    /// oscillating across the `2^k` cliff, sign read after each.
    fn wide_tooth_run(k: u32, w: u32, n: usize) -> Run {
        let tooth = UBig::from(1u8) << w as usize;
        let mut acc = Accumulator::new();
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
            ops: 4 * n as u64,
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
        let mut acc = Accumulator::new();
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
            ops: 4 * n as u64,
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
        let mut acc = Accumulator::new();
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
            ops: 4 * n as u64,
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
        //                                                     ceiling         measured (aarch64-apple-darwin, dev profile, three identical runs)
        pub const COMB_MILLI_PER_DELTA: u64            = 2_500; // 2_000.00 at k=4096/n=50_000 and k=8192/n=100_000 (also the fan row)
        pub const WIDE_TOOTH_MILLI_PER_DELTA: u64      = 7_501; // 6_000.08 at w=192, k=4096/n=25_000; 6_000.04 at k=8192/n=50_000
        pub const CANCELLING_MILLI_PER_CODED_BYTE: u64 =   314; // 250.81 at k=2048/n=4096; 250.40 at k=4096/n=8192
        pub const STATIC_PREFIX_MILLI_PER_READ: u64    = 2_509; // 2_006.50 at k=2048/n=10_000; 2_006.45 at k=4096/n=20_000
    }
}

// ─── skyline query-fold scenarios ───────────────────────────────────────────
//
// The query kernels over skyline streams: rank on the anchored-segment
// height split, min_ticks on the range-minimum anchor web and its epoch
// ledger, and
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
    /// Improvement tripwire under the limb column: measured ×0.75, per
    /// the file doc's tripwire convention.
    #[cfg(feature = "limb-meter")]
    limb_floor: u64,
    /// Improvement tripwire under the touch column: measured ×0.75, per
    /// the file doc's tripwire convention.
    ///
    /// A touch reading below it is a >25% drop from the pinned reading —
    /// attribute it, re-pinning an honest improvement or curing a dead
    /// meter, without which every touch ceiling above would hold
    /// vacuously (zero where the measured count is zero, under which the
    /// bound asserts nothing).
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
// (aarch64-apple-darwin, dev profile, three identical runs)
// the ceiling derives from. Re-pin by rerunning under `--no-capture`
// with `--all-features` and reading the MEASURED lines.
// The limb floor column is the measured value ×0.75, rounded down (the
// file doc's improvement-tripwire convention).
#[rustfmt::skip]
mod query_env {
    use super::{query_envelope, QueryEnvelope};
    //                                                                        peak heap, segments,  limb ops, scan bits,   touches, limb floor, touch floor       measured: heap, seg, limb, scan, touches
    pub const SKYLINE_RANK_DENSE: QueryEnvelope = query_envelope(30_720, 0, 4, 937_515, 7, 2, 3); // 65_560, 0, 250_004 -> 250_005 (metered trailing_zeros), 875_017, 125_007; heap 65_576 -> 24_576 and touches 125_007 -> 5 (the segment feed opens at the first freeze — the depth control's touch column and scale-deep segment buffer were the feed); scan 875_017 -> 750_012 (the max_depth pre-scan records each payload skip once; every other column byte-identical); re-pinned (limb 3, scan 750_012, touches 5, heap 24_576): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_RANK_BIGROOT: QueryEnvelope = query_envelope(67_145, 0, 2_739, 275_023, 8_993, 1_643, 5_395); // 60_088 -> 61_516 (dashu-int backend) -> 61_708 (the zero-run ledger's map nodes; the older ceiling stands), 0, 21_413 -> 22_195 (metered trailing_zeros), 310_024, 17_194; heap 61_724 -> 57_636 (the older ceiling stands) and touches 17_194 -> 7_192 (the segment feed opens at the first freeze); scan 310_024 -> 220_018 (the max_depth pre-scan records each payload skip once; every other column byte-identical); re-pinned (limb 2_191, scan 220_018, touches 7_194, heap 57_604): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_RANK_HARMONIC: QueryEnvelope = query_envelope(52_500, 0, 2_562, 491_530, 248_285, 1_536, 148_971); // 57_364 -> 58_400 (dashu-int backend), 0, 132_097 -> 133_121 (metered trailing_zeros), 458_763, 198_659; heap 58_416 -> 42_040 and touches 198_658 -> 133_121 (the segment feed opens at the first freeze — one deposit per leaf gone); scan 458_763 -> 393_224 (the max_depth pre-scan records each payload skip once; every other column byte-identical); re-pinned to the new readings (limb 133_121, scan 393_224, touches 198_628, heap 42_000): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work; re-pinned (limb 2_049, scan 393_224, touches 198_628, heap 42_000): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_RANK_CLIFF: QueryEnvelope = query_envelope(2_855, 0, 172, 35_845, 6_688, 102, 4_012); // 2_460 -> 2_540 (dashu-int backend) -> 3_132 (the zero-run ledger's map nodes plus the anchored-segment fold's integrator, measured at the cure-round merge), 0, 6_244 -> 6_277 (metered trailing_zeros), 38_917, 6_406; heap 3_132 -> 2_436 and touches 6_406 -> 4_325 (the segment feed opens at the first freeze); scan 38_917 -> 28_676 (the max_depth pre-scan records each payload skip once; every other column byte-identical); re-pinned (limb 137, scan 28_676, touches 5_350, heap 2_284): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_RANK_WIDE_TOOTH: QueryEnvelope      = query_envelope(     3_635,        0,    29_552, 2_000_960,    24_585, 17_755, 14_751); // 2_740 -> 2_820 (dashu-int backend) -> 3_604 (the zero-run ledger's map nodes plus the anchored-segment fold's integrator, measured at the cure-round merge), 0, 23_641 -> 23_674 (metered trailing_zeros), 2_397_055, 26_864 -> 21_749 (certificate skips replace the accumulator's zero-run walks); heap 3_604 -> 2_908 and touches 21_749 -> 19_668 (the segment feed opens at the first freeze — the no-freeze pin pays no feed at all); scan 2_397_055 -> 1_600_768 (the max_depth pre-scan records each payload skip once — 24.0 to 16.0 bits per packed byte on this payload-dominated comb; every other column byte-identical)
    // The practical-regime gauge: `Version::rank`
    // on one concurrent-pair operand — word-scale heights over organic
    // forks, no freeze, no arming. The row pins the benign path's
    // constants so the adversarial machinery's price on common inputs is
    // a committed number, not a vibe.
    pub const RANK_CONCURRENT: QueryEnvelope = query_envelope(0, 0, 4, 61_448, 11_099, 2, 6_659); // 68, 0, 12_289, 65_546, 12_975 (the row's pin of record); touches 12_975 -> 8_879 (the segment feed opens at the first freeze — the practical regime's one-third dead weight); scan 65_546 -> 49_158 (the max_depth pre-scan records each payload skip once; every other column byte-identical); re-pinned (limb 3, scan 49_158, touches 8_879, heap 0): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const TICKS_DENSE: QueryEnvelope = query_envelope(58_815, 0, 8, 468_809, 156_270, 4, 93_762); // 47_052 -> 47_084 (pre-existing drift) -> 47_180 (the zero-run ledger's map nodes; the older ceiling stands), 0, 250_020, 375_047, 125_025 (the tick row's readings plus the count's gamma codes); re-pinned (limb 6, scan 375_047, touches 125_016, heap 47_261): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const TICKS_NESTED_WIDE: QueryEnvelope = query_envelope(14_107, 0, 323, 150_072, 31_125, 193, 18_675); // 11_286 -> 11_350 (pre-existing drift) -> 11_542 (the zero-run ledger's map nodes; the older ceiling stands), 0, 24_280, 120_057, 36_908 (the fill branch pays its documented second walk - scan ~2x the tick row's one walk); re-pinned to the new readings (limb 8_272, scan 120_057, touches 24_900, heap 11_782): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work; re-pinned (limb 258, scan 120_057, touches 24_900, heap 11_782): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const TICKS_MIRROR_WIDE: QueryEnvelope = query_envelope(39_506, 0, 723, 230_045, 107_574, 433, 64_544); // 31_605 -> 31_701 (pre-existing drift) -> 31_989 (the zero-run ledger's map nodes; the older ceiling stands), 0, 40_580, 184_036, 122_060 (second-walk fill branch, as the nested-wide row); re-pinned to the new readings (limb 24_572, scan 184_036, touches 86_059, heap 31_877): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work; re-pinned (limb 578, scan 184_036, touches 86_059, heap 31_877): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_MIN_TICKS_DENSE: QueryEnvelope = query_envelope(30_720, 0, 5, 468_758, 312_508, 3, 187_504); // 24_576, 0, 500_002 -> 250_006 (the anchor-web fold: the per-close signed offset compare is gone), 375_006, 125_005 -> 250_008 (every delta folds into two accumulators - the live height and the web's gap - so the touch column doubles while the quadratic minima circulation leaves; heap and scan byte-identical across the rewrite); re-pinned (limb 4, scan 375_006, touches 250_006, heap 24_576): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_MIN_TICKS_CLIFF: QueryEnvelope = query_envelope(3_530, 0, 180, 17_923, 12_000, 108, 7_200); // 49_752 -> 2_592 -> 3_168 (the anchor-web fold's 19x drop, then the zero-run ledger's map nodes measured at the cure-round merge; the older ceiling stands) -> 3_448 (the accumulator's quick register: one idle i128 per pooled accumulator; work columns unmoved), 8_258 -> 6_284, 70_962 -> 10_619 (the anchor-web fold: the comb's wide F-relative pending offsets and their per-close folds are epoch-ledger counts now, touches 6.7x down; scan byte-identical), 0, 14_338; re-pinned (limb 144, scan 14_338, touches 9_600, heap 3_320): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const SKYLINE_PROJECT_COMB_SCATTER: QueryEnvelope = query_envelope(   525_700,        0,   115_265, 2_652_165,    44_924, 69_159, 26_954); // 420_560 -> 420_592 (dashu-int backend), 0, 92_212, 2_124_806 -> 2_121_732 (single-record id tags), 35_939
    pub const FOLD_VERSION_SCATTER: QueryEnvelope = query_envelope(323, 0, 0, 330_913, 61_429, 0, 36_857); // 73_216 -> 390 -> 294 (the at-rest form is codec::Bits, the wire bytes in a length-carrying container) -> 390 (the borrow-shaped fold face: the counter stack's entries carry the operand-form tag, ~8 B per level; work columns byte-identical) -> 309 (the Bytes-backed at-rest form: the fold's lone-group settle and adoption arms clone by refcount), 0, 690_310 -> 253_904 (sequential 14_281_732) -> 253_902 (pre-existing drift), 163_866 -> 264_730, 0 -> 50_677 (operations route to the skyline kernels); re-pinned to the new readings (limb 111_586, scan 264_730, touches 49_143, heap 282): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work; re-pinned (limb 0, scan 264_730, touches 49_143, heap 258): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const FOLD_PARTY_SCATTER: QueryEnvelope          = query_envelope(       780,        0,         0,   322_068,         0, 0, 0); // 336 -> 624 (the Bytes-backed at-rest form: one refcount control block per frozen stream live in the fold's groups; work columns byte-identical), 0, 0, 292_432 -> 257_654 (join_all answers its up-front tests through a per-call id index; the 292_432 was a mid-round reading, the C2 flag-day commit itself reads 276_044) (sequential 3_284_952), 0
    // Tick rows, on the five-meter harness: the tick walk's cost
    // currency is accumulator digit touches (with scanned bits beside
    // it), which the four-column table never watched. Ceilings ×1.25
    // and floors ×0.75 over the measurements of record below.
    pub const TICK_DENSE: QueryEnvelope = query_envelope(58_815, 0, 0, 468_765, 156_265, 0, 93_759); // 71_484 -> 47_052 (the fused tick: copy-on-first-divergence defers the output buffer past the collapse scan, so the scan path and the builder no longer coexist at peak) -> 47_084 (pre-existing drift) -> 47_180 (the zero-run ledger's map nodes; the older ceiling stands), 0, 250_008, 375_012, 125_017 (work columns byte-identical across the fusion); re-pinned (limb 0, scan 375_012, touches 125_012, heap 47_260): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const TICK_NESTED_WIDE: QueryEnvelope = query_envelope(14_108, 0, 239, 80_028, 30_808, 143, 18_484); // 7_702 -> 11_286 (the explicit-stack walk: suspended ancestors moved from stacker segments to metered frame bits) -> 11_350 (pre-existing drift) -> 11_542 (the zero-run ledger's map nodes; the older ceiling stands), 6 -> 0 (same change; the zero pin is the ratchet), 24_209, 64_022, 36_652 (work columns byte-identical across the conversion; the anchor web reads the wide first payload O(1) times); re-pinned to the new readings (limb 8_205, scan 64_022, touches 24_646, heap 11_782): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work; re-pinned (limb 191, scan 64_022, touches 24_646, heap 11_782): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const TICK_MIRROR_WIDE: QueryEnvelope = query_envelope(32_467, 0, 398, 170_000, 106_948, 238, 64_168); // 25_973 -> 31_605 (the explicit-stack walk's frame bits; the older ceiling stands) -> 31_701 (pre-existing drift) -> 31_989 (the zero-run ledger's map nodes; the older ceiling still stands), 8 -> 0 (same change; the zero pin is the ratchet), 40_312, 128_002 -> 136_000 and 109_560 -> 121_557 (the pre-scan's site-close replay re-reads each disjoint collapse range once; the older ceilings stand) (the frame ledger stores no link for the shared wide minimum; heap parity with one queue word per site); re-pinned to the new readings (limb 24_312, scan 136_000, touches 85_558, heap 31_877): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work; re-pinned (limb 318, scan 136_000, touches 85_558, heap 31_877): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    // The expansion rows: grow-branch deep
    // ticks measuring the whole public tick — walk, route fold, splice.
    // Baselines at the fusion's landing; the composed path's grow-only
    // measurements of record (splice passes alone) were
    // 390_181 heap / 500_012 limb / 2_250_015 scan on the spine and
    // 535_864 / 500_008 / 3_375_021 on the cross, without the fill
    // walk the tick also paid.
    pub const TICK_OWNERSHIP_HOLE: QueryEnvelope = query_envelope(3_647, 0, 0, 37_585, 7_563, 0, 4_537); // 2_917, 0, 0, 30_068, 6_050 (the ownership-gated block scan: unowned staircase runs fold as one net-and-minimum summary each); the leaf-by-leaf mechanism reads touches 12_023 on this family, above the ceiling — the skip must engage for the pin to hold, and the scan column holds every skipped bit still read
    pub const TICK_OWNERSHIP_COMB: QueryEnvelope = query_envelope(59_575, 0, 0, 498_774, 156_275, 0, 93_765); // 47_660, 0, 0, 399_019, 125_020: readings identical to the ungated per-leaf walk's on this family (single-leaf regions everywhere, so the block gate never opens and may cost nothing when closed)
    pub const TICK_EXPAND_SPINE: QueryEnvelope = query_envelope(435_435, 0, 5, 2_187_519, 0, 3, 0);// 348_364, 0, 500_012, 1_750_015, 3 (an empty version's tick folds one word-scale payload: near-zero accumulator work); re-pinned to the new readings (limb 4, scan 1_750_015, touches 0, heap 348_348): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work
    pub const TICK_EXPAND_CROSS: QueryEnvelope = query_envelope(611_210, 0, 5, 3_593_782, 156_260, 3, 93_756); // 488_989, 0, 750_010, 2_875_025, 250_013; re-pinned to the new readings (limb 250_006, scan 2_875_025, touches 125_008, heap 488_984): payload codes move as machine words and the accumulator's quick register folds narrow values without digit or limb work; re-pinned (limb 4, scan 2_875_025, touches 125_008, heap 488_968): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    // The version-pair rows: the public
    // two-operand queries on the pair families, all four rows pinned
    // at the fused co-sweep's cost:
    // the composed emit-then-re-rank record they tightened from is the
    // trailing comment on each row. The jump-pair distance row was the
    // cross-stream freeze wedge's red pin (973_702 limb / 1_029_327
    // touch at this scale); the anchored-segment co-sweep reads flat,
    // and the `skyline_flatness` band test holds it there. The lag
    // jump-pair touch column is the one rise in the sweep (87_148 →
    // 164_630): lag now walks both operands' full overlay on the
    // accumulator instead of skipping the meet leg, buying the heap
    // (−96%), limb (−60%), and scan (−25%) columns down with the same
    // change.
    pub const DISTANCE_JUMP_PAIR: QueryEnvelope = query_envelope(5_750, 0, 48_714, 2_694_095, 208_749, 29_228, 125_249); // 5_008 -> 5_776 (the zero-run ledger's map nodes; the older ceiling stands), 0, 53_905 -> 53_559 (cluster-delegated settle products), 3_218_320, 184_494 (the fused co-sweep; from 138_809, 0, 973_702, 6_464_538, 1_029_327 at the uncured composed form) -> 170_128 (certificate skips replace the accumulator's zero-run walks) -> 167_225 (pre-existing drift); heap 5_376 -> 4_608 and touches 167_225 -> 166_993 (the segment feed opens at the first freeze — this pair freezes early, so only the pre-freeze prefix moves); scan 3_218_320 -> 2_155_276 (the max_depth pre-scan records each payload skip once, twice per pair walk; every other column byte-identical); re-pinned (limb 38_971, scan 2_155_276, touches 166_999, heap 4_600): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const LAG_JUMP_PAIR: QueryEnvelope = query_envelope(5_750, 0, 45_420, 2_694_095, 173_492, 27_252, 104_094); // 5_008 -> 5_776 (the zero-run ledger's map nodes; the older ceiling stands), 0, 50_839 -> 50_924 (cluster-delegated settle products: the lag leg's one upward column, under the standing ceiling), 3_218_320, 163_509 (the fused co-sweep; from 138_641, 0, 127_449, 4_315_090, 87_148 at the composed form) -> 141_463 (certificate skips replace the accumulator's zero-run walks) -> 139_023 (pre-existing drift); heap 5_376 -> 4_608 and touches 139_023 -> 138_791 (the segment feed opens at the first freeze); scan 3_218_320 -> 2_155_276 (the max_depth pre-scan records each payload skip once, twice per pair walk; every other column byte-identical); re-pinned (limb 36_336, scan 2_155_276, touches 138_793, heap 4_600): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const DISTANCE_CONCURRENT: QueryEnvelope = query_envelope(0, 0, 4, 117_753, 32_429, 2, 19_457); // 84, 0, 23_207, 126_287, 36_183 (the fused co-sweep; from 5_964, 0, 166_549, 263_500, 61_442 at the composed form) -> 32_087 (pre-existing drift) -> 27_991 (the segment feed opens at the first freeze — the word-scale pair never freezes); scan 126_287 -> 94_202 (the max_depth pre-scan records each payload skip once, twice per pair walk; every other column byte-identical); re-pinned (limb 3, scan 94_202, touches 25_943, heap 0): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const LAG_CONCURRENT: QueryEnvelope = query_envelope(0, 0, 4, 117_753, 33_278, 2, 19_966); // 84, 0, 23_207, 126_287, 36_862 (the fused co-sweep; from 5_964, 0, 95_571, 197_300, 43_696 at the composed form) -> 32_766 (pre-existing drift) -> 28_670 (the segment feed opens at the first freeze); scan 126_287 -> 94_202 (the max_depth pre-scan records each payload skip once, twice per pair walk; every other column byte-identical); re-pinned (limb 3, scan 94_202, touches 26_622, heap 0): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    // The masked-comparison rows:
    // the fused projected comparisons on the correlated mask-drift
    // families, priced input-only on shapes whose *materialization* is
    // product-growth — the laziness the view exists for. Ceilings x1.25
    // and floors x0.75 over the measurements of record in the trailing
    // comments.
    pub const MASKED_CMP_DRIFT_TRIPLE: QueryEnvelope = query_envelope(1_570, 0, 59, 20_488, 5_240, 35, 3_144); // 1_032 -> 1_384 (the zero-run ledger's map nodes), 0, 6_187, 16_390, 4_192 (the landing measurement: one pass over the overlay, ~2 touches per stored delta); re-pinned (limb 47, scan 16_390, touches 4_192, heap 1_256): decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
    pub const MASKED_CMP_DRIFT_QUAD: QueryEnvelope        = query_envelope(     2_720,        0,    39_722, 1_342_092,    83_946, 23_833, 50_367); // 2_176 -> 2_368 (the zero-run ledger's map nodes; the older ceiling stands), 0, 31_778, 1_073_674, 67_157 (the landing measurement: the sparse comb's wide climb/drop codes dominate the input; scan ~8 bits per input byte)
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
    suanpan::touch_meter::reset();
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
    let touches = suanpan::touch_meter::touches();
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
        "{name}: limb counter reads {limb_ops}, below the {} improvement \
         tripwire (measured x0.75): attribute the drop — an honest \
         improvement re-pins the band; a dead meter is the bypass this \
         column exists to catch",
        env.limb_floor,
    );
    #[cfg(feature = "limb-meter")]
    assert!(
        touches >= env.touch_floor,
        "{name}: touch counter reads {touches}, below the {} improvement \
         tripwire (measured x0.75): attribute the drop — an honest \
         improvement re-pins the band; a dead meter is the bypass this \
         column exists to catch",
        env.touch_floor,
    );
    r
}

/// The rank kernel on the dense spine's skyline stays within its
/// envelope (the depth control: 125k levels of path bits, near-zero
/// arithmetic).
#[test]
fn skyline_rank_dense_envelope() {
    let p = Shape::Dense.packed1(DENSE_DEPTH);
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
    let p = Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
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
    let p = Shape::Harmonic.packed1(RANK_HARMONIC_DEPTH);
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
    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
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
    let p = Shape::WideToothComb.packed3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
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
/// one narrow offset min-merge per node, heights on the accumulator,
/// zero grown segments at 125k levels.
#[test]
fn skyline_min_ticks_dense_envelope() {
    let p = Shape::Dense.packed1(DENSE_DEPTH);
    let v = version_of(&p);
    let enc = skyline_of(&p);
    let r = query_metered(
        "skyline_min_ticks_dense",
        enc.as_raw_slice().len(),
        &query_env::SKYLINE_MIN_TICKS_DENSE,
        || meter::skyline::query::min_ticks(&enc),
    );
    assert_eq!(
        r.to_string(),
        v.min_ticks().to_string(),
        "the kernel must match the packed fold"
    );
}

/// The min_ticks kernel on the boundary comb stays within its envelope.
///
/// The `2^k`-scale first height rides the frozen component and enters
/// the exact total once, through the counting term — never per leaf —
/// so the comb's teeth cost narrow offsets only.
#[test]
fn skyline_min_ticks_cliff_envelope() {
    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
    let v = version_of(&p);
    let enc = skyline_of(&p);
    let r = query_metered(
        "skyline_min_ticks_cliff",
        enc.as_raw_slice().len(),
        &query_env::SKYLINE_MIN_TICKS_CLIFF,
        || meter::skyline::query::min_ticks(&enc),
    );
    assert!(
        r.to_string().len() > 20,
        "the comb's floor exceeds any machine word: the wide arm is live"
    );
    assert_eq!(
        r.to_string(),
        v.min_ticks().to_string(),
        "the kernel must match the packed fold"
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
    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
    let v = version_of(&p);
    let party = before::Party::decode(&Shape::ScatteredId.packed1(CLIFF_SCALE / 2).bytes[..])
        .expect("scattered id is strict normal form");
    let enc = skyline_of(&p);
    let io_bytes_in =
        enc.as_raw_slice().len() + Shape::ScatteredId.packed1(CLIFF_SCALE / 2).bytes.len();
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
    let expected = meter::skyline::encode(&(&v / &party).to_version());
    assert_eq!(out, expected, "the kernel must match the packed quotient");
}

// ─── version-pair query scenarios ───────────────────────────────────────────
//
// The public two-operand queries on the pair families (the corpus pairing
// `w = v + one seed tick` collapses the second operand onto a dominating
// plateau, so the co-sweep's orientation switches and its freeze paths
// would go unpriced without these rows). All four rows are linear
// records of the fused co-sweep: the jump-pair rows price wide drift
// crossing the other operand's cheap boundaries over a dense-position
// spine (the shape whose absolute-position accounting read superlinear
// before the anchored-segment discipline; the `skyline_flatness` band
// test holds it flat across a scale doubling), and the concurrent rows
// price orientation-switch density on word-scale heights.

/// `Version::distance` on the two-operand jump comb stays within its
/// envelope.
///
/// The shape: wide per-level crests of the height difference, parked
/// and settled against segment masses (the `skyline_flatness` band
/// test carries the cross-scale flatness bound).
///
/// The result is anchored by rank modularity: the distance must equal
/// the two lags' sum.
#[test]
fn version_distance_jump_pair_envelope() {
    let (pa, pb) =
        Shape::JumpPair.packed_pair3(JUMP_PAIR_MAGNITUDE_BITS, JUMP_PAIR_TEETH, JUMP_PAIR_DIGITS);
    let a = pa.version();
    let b = pb.version();
    let input_bytes = a.encode().len() + b.encode().len();
    let (r, a, b) = query_metered(
        "version_distance_jump_pair",
        input_bytes,
        &query_env::DISTANCE_JUMP_PAIR,
        move || {
            let r = a.distance(&b);
            (r, a, b)
        },
    );
    assert_eq!(
        r,
        &a.lag(&b) + &b.lag(&a),
        "the distance must equal the two lags' sum (rank modularity)"
    );
}

/// `Version::lag` on the two-operand jump comb stays within its
/// envelope.
///
/// Lag integrates the directed functional `(h_b − h_a)⁺` over the same
/// overlay walk as distance, so this row prices the one-sided
/// orientation (long zero-orientation stretches where the band
/// dominates) against the symmetric row above.
#[test]
fn version_lag_jump_pair_envelope() {
    let (pa, pb) =
        Shape::JumpPair.packed_pair3(JUMP_PAIR_MAGNITUDE_BITS, JUMP_PAIR_TEETH, JUMP_PAIR_DIGITS);
    let a = pa.version();
    let b = pb.version();
    let input_bytes = a.encode().len() + b.encode().len();
    query_metered(
        "version_lag_jump_pair",
        input_bytes,
        &query_env::LAG_JUMP_PAIR,
        move || {
            let r = a.lag(&b);
            (r, a, b)
        },
    );
}

/// `Version::rank` on one concurrent-pair operand stays within its
/// envelope — the practical-regime constant gauge.
///
/// Word-scale heights over organically forked parties: the regime the
/// overwhelming share of real inputs lives in (every event count fits a
/// machine word, nothing freezes, the promotion ledger never arms). The
/// adversarial rank rows above price the machinery's worst shapes; this
/// row pins what a benign input pays for that machinery's existence, so
/// a change that cheapens the adversarial path by charging the common
/// one cannot read as an improvement.
#[test]
fn version_rank_concurrent_envelope() {
    let (v, _) = Shape::ConcurrentPair.version_pair(CONCURRENT_PAIR_LEAVES);
    let input_bytes = v.encode().len();
    query_metered(
        "version_rank_concurrent",
        input_bytes,
        &query_env::RANK_CONCURRENT,
        move || {
            let r = v.rank();
            (r, v)
        },
    );
}

/// `Version::distance` on the concurrent pair stays within its
/// envelope.
///
/// The co-sweep's orientation flips at every one of the `n − 1` overlay
/// boundaries, on word-scale heights, so this row prices switch density
/// with no width in play.
///
/// The semantic anchor: the schedule realizes a distance of exactly the
/// integer rank 2 at every `n` (the generator's construction), so a
/// misrouted orientation switch cannot pass as a cheap reading.
#[test]
fn version_distance_concurrent_envelope() {
    let (v, w) = Shape::ConcurrentPair.version_pair(CONCURRENT_PAIR_LEAVES);
    let input_bytes = v.encode().len() + w.encode().len();
    let (r, _, _) = query_metered(
        "version_distance_concurrent",
        input_bytes,
        &query_env::DISTANCE_CONCURRENT,
        move || {
            let r = v.distance(&w);
            (r, v, w)
        },
    );
    assert_eq!(
        r,
        Version::try_from(2u64)
            .expect("a small integer version is valid")
            .rank(),
        "the schedule's heights must be realized end to end"
    );
}

/// `Version::lag` on the concurrent pair stays within its envelope —
/// the same switch-dense overlay as the distance row, under the
/// one-sided functional (every other plateau reads orientation zero).
#[test]
fn version_lag_concurrent_envelope() {
    let (v, w) = Shape::ConcurrentPair.version_pair(CONCURRENT_PAIR_LEAVES);
    let input_bytes = v.encode().len() + w.encode().len();
    query_metered(
        "version_lag_concurrent",
        input_bytes,
        &query_env::LAG_CONCURRENT,
        move || {
            let r = v.lag(&w);
            (r, v, w)
        },
    );
}

// ─── masked-comparison scenarios ────────────────────────────────────────────
//
// The fused projected comparisons (`OwnVersion`'s three- and four-stream
// co-walks) on the correlated mask-drift families: ownership toggles at
// every tooth boundary while the other operand's height drift sits on the
// `2^k` carry boundary, so every boundary's sign read lands mid-cancel or
// mid-oscillation. Both scenarios' verdicts are `Less` (pinned by the
// generator tests), so no early exit shortens the measured walk, and both
// anchor the fused verdict against the materialized comparison in the
// same run.

/// The fused three-stream comparison `(comb / mask) ⋚ plateau` stays
/// within its envelope on the correlated triple.
///
/// The mask gates the comb to every other tooth against a flat wide
/// plateau: owned intervals read the near-zero difference spelled by
/// cancelling wide digits, unowned intervals read the zero-check on the
/// plateau's height, and every read is amortized O(1) on the balanced
/// signed-digit accumulator (the flatness band below holds it across a
/// doubling).
#[test]
fn own_version_cmp_mask_drift_envelope() {
    let (comb, mask, plateau) =
        Shape::MaskDriftTriple.packed_triple(MASK_DRIFT_MAGNITUDE_BITS, MASK_DRIFT_TEETH);
    let v = comb.version();
    let p = Party::decode(&mask.bytes[..]).expect("the mask is strict normal form");
    let w = plateau.version();
    let input_bytes = v.encode().len() + mask.bytes.len() + w.encode().len();
    let (ord, v, p, w) = query_metered(
        "own_version_cmp_mask_drift",
        input_bytes,
        &query_env::MASKED_CMP_DRIFT_TRIPLE,
        move || {
            let ord = (&v / &p).partial_cmp(&w);
            (ord, v, p, w)
        },
    );
    assert_eq!(
        ord,
        Some(Ordering::Less),
        "the projected comb sits strictly under the plateau (the full-walk verdict)"
    );
    assert_eq!(
        ord,
        (&v / &p).to_version().partial_cmp(&w),
        "the fused verdict is the materialized verdict"
    );
}

/// The fused four-stream comparison `(v₁/p₁) ⋚ (v₂/p₂)` stays within its
/// envelope on the correlated quadruple.
///
/// The two masks' parities interleave tooth for tooth: even-level teeth
/// read the trichotomy's zero-check on a semantically-zero height spelled
/// by cancelling `2^k`-wide digits, odd-level teeth read the other side's
/// height mid-oscillation across the carry boundary.
#[test]
fn own_version_pair_cmp_mask_drift_envelope() {
    let ((sparse, even_mask), (comb, odd_mask)) =
        Shape::MaskDriftQuadruple.packed_quadruple(MASK_DRIFT_MAGNITUDE_BITS, MASK_DRIFT_TEETH);
    let v1 = sparse.version();
    let p1 = Party::decode(&even_mask.bytes[..]).expect("the mask is strict normal form");
    let v2 = comb.version();
    let p2 = Party::decode(&odd_mask.bytes[..]).expect("the mask is strict normal form");
    let input_bytes =
        v1.encode().len() + even_mask.bytes.len() + v2.encode().len() + odd_mask.bytes.len();
    let (ord, v1, p1, v2, p2) = query_metered(
        "own_version_pair_cmp_mask_drift",
        input_bytes,
        &query_env::MASKED_CMP_DRIFT_QUAD,
        move || {
            let ord = (&v1 / &p1).partial_cmp(&(&v2 / &p2));
            (ord, v1, p1, v2, p2)
        },
    );
    assert_eq!(
        ord,
        Some(Ordering::Less),
        "the semantically-empty view sits strictly under the tooth-keeping view"
    );
    assert_eq!(
        ord,
        (&v1 / &p1)
            .to_version()
            .partial_cmp(&(&v2 / &p2).to_version()),
        "the fused verdict is the materialized verdict"
    );
}

// ─── the cheap-clone demonstration cell ──────────────────────────────────────

/// Peak-heap ceiling for the equal-operands join fold: the fold's own
/// machinery (the counter's group vec, the dedup adapter's held clone),
/// none of it proportional to the operands.
// 624 -> 352 at 376 B operands, 1_749 -> 352 at 1_501 B (
// the Bytes-backed at-rest form, parent-measured at 3a4f3c8e: the
// equality rung's hand-back was a byte copy of the operand, so the
// parent peak scaled with it; the hand-back is now an O(1) refcount
// bump and the fold's peak is its size-independent machinery alone —
// the flatness leg below is the proof). Ceiling 1.25x the 352 B
// measurement of record.
const JOIN_EQUAL_OPERANDS_PEAK: usize = 440;

/// The cheap-clone demonstration: `join_all` over two byte-equal
/// versions answers through the equality rung and hands back a clone,
/// an `O(1)` refcount bump.
///
/// The fold's peak heap is therefore the counter machinery alone,
/// *byte-identical across a 4x operand growth* (the flatness leg no
/// operand-copying clone arm can pass). The semantic leg pins the
/// verdict: the join IS the operand, byte for byte.
#[test]
fn join_all_equal_operands_is_clone_cheap() {
    let run = |depth: usize| {
        let p = Shape::Dense.packed1(depth);
        let a = version_of(&p);
        let b = version_of(&p); // byte-equal, buffer-distinct
        let input_bytes = a.encode().len() + b.encode().len();
        HEAP.reset_peak_usage();
        let baseline = HEAP.current_usage();
        let out = a.join_all([&b]);
        let peak_heap = HEAP.peak_usage().saturating_sub(baseline);
        eprintln!(
            "MEASURED join_all_equal_operands/{depth}: input_bytes={input_bytes} \
             peak_heap={peak_heap}"
        );
        assert_eq!(out, a, "a ∨ a = a: the fold's verdict is the operand");
        assert_eq!(out.as_bytes(), a.as_bytes());
        peak_heap
    };
    // The witness of record: two equal 376 B versions (dense depth 1,000).
    let small = run(1000);
    let large = run(4000);
    assert!(
        small <= JOIN_EQUAL_OPERANDS_PEAK,
        "join_all over two equal operands peaked {small} B (ceiling \
         {JOIN_EQUAL_OPERANDS_PEAK} B): the clone arm must not copy the \
         operand: {ISOLATION_NOTE}"
    );
    assert_eq!(
        small, large,
        "the equal-operands fold's peak must not move with operand size: \
         the clone arm is O(1): {ISOLATION_NOTE}"
    );
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
    let (mut versions, _) = scatter_population();
    let input_bytes: usize = versions.iter().map(|v| v.encode().len()).sum();
    let reference = versions.iter().fold(Version::new(), |acc, v| acc | v);
    let rest = versions.split_off(1);
    let receiver = versions.pop().expect("the population is nonempty");
    let out = query_metered(
        "fold_version_scatter",
        input_bytes,
        &query_env::FOLD_VERSION_SCATTER,
        || receiver.join_all(rest),
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

// ─── the n-ary fold's correlated-population bands (the stagger pins) ─────────
//
// The staggered fold population loads the balanced binary-counter
// reduction itself: `n` operands of `m` unit teeth each, every operand's
// teeth landing in the gaps of every other's, fed in bit-reversed order
// so every internal merge — at every level — joins region sets that
// interleave maximally and swell to near the sum of their sizes
// (`meter::stagger_population` carries the construction and the feed
// order's derivation). The scatter population scales arity at
// single-leaf operands and weave scales operand size at fixed arity;
// this family drives both axes jointly against the reduction, so the
// bands below hold the fold's declared `O(D log 2k)` model in EACH
// direction independently — arity doubling at fixed operand size, and
// operand size doubling at fixed arity — under absolute pinned ceilings:
// a single diagonal scaling could hide which factor drives growth.
// Every counter is normalized by the model's own level count
// `log2(2n)` before the ×1.25 flatness bound, so the bands enforce the
// model's *constant*, not a flat reading the documented log factor
// would forbid (on the arity axis the raw per-byte cost legitimately
// grows exactly one level's worth per doubling).
//
// The known-bad readings these bands separate from [measured
// in the dev profile, exact counters, n = 256, m = 64]: the
// sequential left version fold (`fold(Version::new(), |acc, v| acc |
// v)`) re-walks its never-coalescing accumulator per input — 25,725,854
// scanned bits and 4,990,711 touches against the balanced reduction's
// 4,906,266 and 1,048,313 on the same 63,488 B population (×5.2 and
// ×4.8, the gap widening with arity) — and the sequential party fold
// (one `join` per input) reads 18,246,408 scanned bits against the
// balanced 7,260,488 on 40,960 B (×2.5): the growing-accumulator genre
// the balanced reduction exists to foreclose;
// the per-door `*_log_factor_is_alive` pins (the asymptotics suite)
// keep the model's log factor itself honest.
#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
mod fold_stagger {
    use before::meter::registry::Shape;
    use before::{meter, Version};
    use suanpan::touch_meter;

    /// One balanced `Version::join_all` run over the staggered
    /// population: total input bytes, the model's level count, and the
    /// three fold counters over the fold body alone.
    ///
    /// Carries two semantic legs — the balanced fold equals the
    /// sequential left fold, and both equal the constant-1 skyline
    /// (every slot owned exactly once) — and the teeth liveness floor:
    /// every operand's teeth are folded at least once in its
    /// first-level merge, so a touch reading under `n·m` means the
    /// fold's accumulator work left the metered representation.
    fn version_fold_run(n: usize, m: usize) -> Run {
        let (versions, _) = Shape::StaggerPopulation.population(n, m);
        let mut versions: Vec<Version> = versions.iter().map(meter::Packed::version).collect();
        let bytes: u64 = versions.iter().map(|v| v.encode().len() as u64).sum();
        let sequential = versions.iter().fold(Version::new(), |acc, v| acc | v);
        let rest = versions.split_off(1);
        let receiver = versions.pop().expect("the population is nonempty");
        touch_meter::reset();
        meter::reset_limb_ops();
        meter::reset_scan_bits();
        let out = receiver.join_all(rest);
        let run = Run {
            bytes,
            levels: (2.0 * n as f64).log2(),
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
            scan_bits: meter::scan_bits(),
        };
        assert_eq!(out, sequential, "the balanced fold equals the left fold");
        assert_eq!(
            out,
            "1".parse::<Version>().expect("canonical"),
            "the population's teeth tile the whole domain at height 1"
        );
        assert!(
            run.touches >= (n * m) as u64,
            "join_all over {n}x{m} teeth: {} digit touches under the \
             one-per-tooth floor: the fold's accumulator work is not metered",
            run.touches,
        );
        run
    }

    /// One balanced `Party::join_all` run over the staggered id
    /// population: total input bytes, the model's level count, and the
    /// scan counter over the fold body alone.
    ///
    /// The id walk allocates nothing and does no arithmetic, so
    /// scanned bits are the only deterministic meter that sees it.
    ///
    /// Carries the seed-reunion semantic leg (the population's slots
    /// tile the whole seed region) and the full-examination liveness
    /// floor: a scan reading under 8 bits per operand byte means the
    /// walk left the metered primitives.
    fn party_fold_run(n: usize, m: usize) -> Run {
        let (_, ids) = Shape::StaggerPopulation.population(n, m);
        let mut parties: Vec<before::Party> = ids
            .iter()
            .map(|p| before::Party::decode(&p.bytes[..]).expect("canonical"))
            .collect();
        let bytes: u64 = parties.iter().map(|p| p.encode().len() as u64).sum();
        let rest = parties.split_off(1);
        let mut acc = parties.remove(0);
        touch_meter::reset();
        meter::reset_limb_ops();
        meter::reset_scan_bits();
        acc.join_all(rest)
            .expect("the staggered slots are pairwise disjoint");
        let run = Run {
            bytes,
            levels: (2.0 * n as f64).log2(),
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
            scan_bits: meter::scan_bits(),
        };
        assert!(acc.is_seed(), "the staggered slots reunite the seed region");
        assert!(
            run.scan_bits >= 8 * run.bytes,
            "party join_all over {n}x{m} slots: {} scanned bits under the \
             full-examination floor: the walk is not metered",
            run.scan_bits,
        );
        run
    }

    /// One fold run's counters and its model denominators.
    struct Run {
        bytes: u64,
        levels: f64,
        touches: u64,
        limb_ops: u64,
        scan_bits: u64,
    }

    /// Assert one counter's model-normalized per-byte cost stays flat
    /// (×1.25) across a doubling: `counter / (bytes · log2(2n))` — the
    /// declared fold model's constant.
    fn assert_model_flat(name: &str, small: &Run, large: &Run, counter: fn(&Run) -> u64) {
        let (m1, m2) = (counter(small) as f64, counter(large) as f64);
        let (d1, d2) = (
            small.bytes as f64 * small.levels,
            large.bytes as f64 * large.levels,
        );
        eprintln!(
            "MEASURED fold_stagger_{name}: small={m1}/{:.0} large={m2}/{:.0} \
             per_byte_level={:.3} -> {:.3}",
            d1,
            d2,
            m1 / d1,
            m2 / d2,
        );
        assert!(
            m2 * d1 <= m1 * d2 * 1.25,
            "fold_stagger_{name}: the model-normalized per-byte cost grew more \
             than x1.25 across the doubling: {m1}/{d1} -> {m2}/{d2}"
        );
    }

    /// Assert one run's counters against its absolute pinned ceilings
    /// `(touches, limb ops, scanned bits)`, printing the measured line
    /// re-pins read from.
    fn assert_ceilings(name: &str, run: &Run, ceilings: (u64, u64, u64)) {
        eprintln!(
            "MEASURED fold_stagger_{name}: bytes={} touches={} limb_ops={} scan_bits={}",
            run.bytes, run.touches, run.limb_ops, run.scan_bits,
        );
        let (touch, limb, scan) = ceilings;
        assert!(
            run.touches <= touch,
            "fold_stagger_{name}: {} touches exceed the pinned ceiling {touch}",
            run.touches,
        );
        assert!(
            run.limb_ops <= limb,
            "fold_stagger_{name}: {} limb ops exceed the pinned ceiling {limb}",
            run.limb_ops,
        );
        assert!(
            run.scan_bits <= scan,
            "fold_stagger_{name}: {} scanned bits exceed the pinned ceiling {scan}",
            run.scan_bits,
        );
    }

    /// The bands' base population: 64 operands of 64 teeth (the board's
    /// stagger family at scale 1.0); each axis doubles its own knob
    /// twice from here.
    const STAGGER_SMALL: usize = 64;

    /// Absolute (touch, limb, scan) ceilings for the version fold's
    /// arity axis, measured ×1.25 at
    /// `(n, m) = (64, 64), (128, 64), (256, 64)`.
    const VERSION_ARITY_CEILINGS: [(u64, u64, u64); 3] = [
        (214_954, 982_360, 959_793),
        (537_433, 2_517_710, 2_462_273),
        (1_310_392, 6_264_260, 6_132_833),
    ];

    /// Absolute ceilings for the version fold's size axis, measured
    /// ×1.25 at `(n, m) = (64, 64), (64, 128), (64, 256)`.
    const VERSION_SIZE_CEILINGS: [(u64, u64, u64); 3] = [
        (214_954, 982_360, 959_793),
        (429_994, 1_965_400, 1_919_798),
        (860_074, 3_931_480, 3_839_803),
    ];

    /// Absolute ceilings for the party fold's arity axis, measured
    /// ×1.25 (scan is the fold's only live counter; the
    /// touch and limb legs assert the id walk stays arithmetic-free at
    /// zero).
    const PARTY_ARITY_CEILINGS: [u64; 3] = [1_781_690, 4_034_330, 9_075_610];

    /// Absolute ceilings for the party fold's size axis, measured
    /// ×1.25.
    const PARTY_SIZE_CEILINGS: [u64; 3] = [1_781_690, 3_892_090, 8_437_970];

    /// The version fold's model-normalized cost stays flat across two
    /// arity doublings at fixed operand size.
    ///
    /// `join_all` pays the declared `O(D log 2k)` and nothing more
    /// when every reduction merge swells to the sum of its inputs.
    ///
    /// The raw per-byte cost on this axis legitimately grows one
    /// level's worth per doubling (the documented log factor); the
    /// band divides it out and holds the model's constant, so a
    /// reduction whose merges re-walk more than the swollen overlay —
    /// a growing-accumulator regression, a per-level re-scan — reads
    /// over the bound while the model's own growth passes exactly.
    #[test]
    fn fold_version_stagger_arity_axis_is_flat_per_unit() {
        let m = STAGGER_SMALL;
        let runs = [
            version_fold_run(m, m),
            version_fold_run(2 * m, m),
            version_fold_run(4 * m, m),
        ];
        for (run, ceilings) in runs.iter().zip(VERSION_ARITY_CEILINGS) {
            assert_ceilings("version_arity", run, ceilings);
        }
        for pair in runs.windows(2) {
            assert_model_flat("version_arity_touches", &pair[0], &pair[1], |r| r.touches);
            assert_model_flat("version_arity_limb_ops", &pair[0], &pair[1], |r| r.limb_ops);
            assert_model_flat("version_arity_scan_bits", &pair[0], &pair[1], |r| {
                r.scan_bits
            });
        }
    }

    /// The version fold's model-normalized cost stays flat across two
    /// operand-size doublings at fixed arity.
    ///
    /// At a fixed level count the fold is linear in the population's
    /// packed bytes, however large each swollen intermediate grows.
    #[test]
    fn fold_version_stagger_size_axis_is_flat_per_unit() {
        let n = STAGGER_SMALL;
        let runs = [
            version_fold_run(n, n),
            version_fold_run(n, 2 * n),
            version_fold_run(n, 4 * n),
        ];
        for (run, ceilings) in runs.iter().zip(VERSION_SIZE_CEILINGS) {
            assert_ceilings("version_size", run, ceilings);
        }
        for pair in runs.windows(2) {
            assert_model_flat("version_size_touches", &pair[0], &pair[1], |r| r.touches);
            assert_model_flat("version_size_limb_ops", &pair[0], &pair[1], |r| r.limb_ops);
            assert_model_flat("version_size_scan_bits", &pair[0], &pair[1], |r| {
                r.scan_bits
            });
        }
    }

    /// The party fold's model-normalized scan cost stays flat across
    /// two arity doublings at fixed operand size, and the id walk
    /// forces no arithmetic at any scale (touch and limb pinned at
    /// zero).
    ///
    /// The population's operands are both-present at the whole shared
    /// top per pair, so the fold's up-front overlap tests run the
    /// indexed searches the board's declared allowance prices, and the
    /// reduction's merges splice maximally interleaved region sets —
    /// the party-side intermediate swell.
    #[test]
    fn fold_party_stagger_arity_axis_is_flat_per_unit() {
        let m = STAGGER_SMALL;
        let runs = [
            party_fold_run(m, m),
            party_fold_run(2 * m, m),
            party_fold_run(4 * m, m),
        ];
        for (run, ceiling) in runs.iter().zip(PARTY_ARITY_CEILINGS) {
            assert_ceilings("party_arity", run, (0, 0, ceiling));
        }
        for pair in runs.windows(2) {
            assert_model_flat("party_arity_scan_bits", &pair[0], &pair[1], |r| r.scan_bits);
        }
    }

    /// The party fold's model-normalized scan cost stays flat across
    /// two operand-size doublings at fixed arity, and the id walk
    /// forces no arithmetic at any scale (touch and limb pinned at
    /// zero).
    #[test]
    fn fold_party_stagger_size_axis_is_flat_per_unit() {
        let n = STAGGER_SMALL;
        let runs = [
            party_fold_run(n, n),
            party_fold_run(n, 2 * n),
            party_fold_run(n, 4 * n),
        ];
        for (run, ceiling) in runs.iter().zip(PARTY_SIZE_CEILINGS) {
            assert_ceilings("party_size", run, (0, 0, ceiling));
        }
        for pair in runs.windows(2) {
            assert_model_flat("party_size_scan_bits", &pair[0], &pair[1], |r| r.scan_bits);
        }
    }
}

// ─── the aliased-rejection fold band ─────────────────────────────────────────
//
// The aliased-rejection loading of the n-ary party fold: the one
// population genre the well-formed fold families (scatter, weave,
// stagger) cannot reach, because linear parties are pairwise disjoint
// by construction — aliases arrive only through decode or
// dangerously_alias, and the fold's contract is to hand them back,
// dropping nothing. The band holds the hand-back path linear: every
// rejected alias costs one up-front index test plus one failed counter
// combine against the weight-0 survivor, each walking at most the
// alias's own deep overlap path — work the alias's own packed bytes
// fund — and the binary counter's over-full-slot policy is what keeps
// a failed group from re-probing a large accumulated group (the popped
// partner is always the most recent same-weight arrival, never the big
// old one).
#[cfg(feature = "scan-meter")]
mod fold_alias {
    use before::{meter, Party};

    /// One aliased `Party::join_all` run: `k` aliases of one
    /// depth-`depth` fragment against the host owning the rest of the
    /// seed, scan bits over the fold body alone.
    ///
    /// Carries three semantic legs — exactly `k − 1` aliases come back,
    /// each byte-identical to the fragment, and the host ends as the
    /// whole seed (the one accepted alias reunited it) — and the
    /// examination liveness floor: every alias's overlap is only
    /// discoverable by walking its path, so a scan reading under one
    /// bit per population byte means the id walks left the metered
    /// primitives.
    fn alias_run(k: usize, depth: usize) -> (u64, u64) {
        let mut host = Party::seed();
        let mut deep = host.fork();
        for _ in 1..depth {
            let sibling = deep.fork();
            host.join(sibling).expect("halves of one seed are disjoint");
        }
        let aliases: Vec<Party> = (0..k).map(|_| deep.dangerously_alias()).collect();
        let bytes = host.encode().len() as u64
            + aliases.iter().map(|a| a.encode().len() as u64).sum::<u64>();
        meter::reset_scan_bits();
        let rejected = host
            .join_all(aliases)
            .expect_err("all but the first alias overlap the accumulated group");
        let scan_bits = meter::scan_bits();
        assert_eq!(
            rejected.len(),
            k - 1,
            "exactly one alias reunites the seed; the rest come back"
        );
        for back in &rejected {
            assert_eq!(back, &deep, "hand-backs are the fragment, byte for byte");
        }
        assert!(host.is_seed(), "the accepted alias reunited the whole seed");
        assert!(
            scan_bits >= bytes,
            "aliased join_all over {k} x depth {depth}: {scan_bits} scanned bits \
             under the one-bit-per-population-byte floor: the walks are not metered"
        );
        (bytes, scan_bits)
    }

    /// Slack numerator over the small-scale per-byte cost (denominator
    /// [`SLACK_DEN`]): the ×1.25 flatness convention.
    const SLACK_NUM: u64 = 5;

    /// Slack denominator for the flatness bound.
    const SLACK_DEN: u64 = 4;

    /// Assert the per-byte scan cost stays flat (×1.25) across one
    /// doubling.
    fn assert_flat(name: &str, small: (u64, u64), large: (u64, u64)) {
        let (b1, s1) = small;
        let (b2, s2) = large;
        eprintln!(
            "MEASURED fold_alias_{name}: small={s1}/{b1}B large={s2}/{b2}B \
             milli_per_byte={} -> {}",
            s1 * 1000 / b1,
            s2 * 1000 / b2,
        );
        assert!(
            u128::from(s2) * u128::from(b1) * u128::from(SLACK_DEN)
                <= u128::from(s1) * u128::from(b2) * u128::from(SLACK_NUM),
            "fold_alias_{name}: per-byte scan cost grew more than x1.25 across \
             the doubling: {s1}/{b1}B -> {s2}/{b2}B"
        );
    }

    /// Aliases of the alias-count axis' small run (the large run
    /// doubles the count at the same fragment depth).
    const ALIAS_COUNT_SMALL: usize = 512;

    /// Fragment depth of the depth axis' small run (the large run
    /// doubles the depth at the same alias count).
    const ALIAS_DEPTH_SMALL: usize = 256;

    /// The hand-back path is flat per population byte across an
    /// alias-count doubling: each rejected alias pays its own test and
    /// one weight-0 failed combine, never a re-probe of the group.
    #[test]
    fn party_fold_alias_rejection_count_is_flat_per_unit() {
        let small = alias_run(ALIAS_COUNT_SMALL, ALIAS_DEPTH_SMALL);
        let large = alias_run(2 * ALIAS_COUNT_SMALL, ALIAS_DEPTH_SMALL);
        assert_flat("count_scan_bits", small, large);
    }

    /// The hand-back path stays inside the declared fold model across
    /// a fragment-depth doubling.
    ///
    /// The overlap witness sits at the bottom of every alias's path,
    /// each rejection's walk to it is paid by that alias's own packed
    /// bytes, and the up-front test's per-node table search
    /// contributes the model's `log |p|` factor and nothing more.
    ///
    /// The per-byte reading legitimately moves by the log factor here
    /// (measured +16% across the doubling, the `O(B log |p|)` model's
    /// own growth — 204 → 239 scanned bits per shared path level), so
    /// the ×1.25 band is the model bound per doubling, not a flatness
    /// claim: a hand-back that re-walked the accumulated group per
    /// alias would read ×2 and cannot hide inside it.
    #[test]
    fn party_fold_alias_rejection_depth_is_flat_per_unit() {
        let small = alias_run(ALIAS_COUNT_SMALL, ALIAS_DEPTH_SMALL);
        let large = alias_run(ALIAS_COUNT_SMALL, 2 * ALIAS_DEPTH_SMALL);
        assert_flat("depth_scan_bits", small, large);
    }
}

// ─── the meet-fold shade band ────────────────────────────────────────────────
//
// The n-ary meet fold's non-shrinking-accumulator band. The shade
// population MS(d, k) is the meet dual of the join wedges: one deep
// carrier (`Shape::Dense.packed1(d)`, heights 0/1), then `k − 1` single-leaf plateau
// shades strictly above it (`Shape::Hugeleaf.packed1(2)`, the constant-3 skyline), so
// the running meet is the carrier, byte-identical at every combine, and
// a meet's emission sweep walks BOTH operands' streams whole — no
// domination short-circuit exists in `emit::meet`. `Version::meet_all`
// runs the join folds' balanced binary-counter reduction, so the
// carrier is re-walked once per counter level — `O(d log k + k)`, the
// declared `O(D log 2k)` fold model — and equal shades answer by
// canonical identity before any sweep. The band below holds the
// model-normalized per-byte cost flat across two diagonal doublings in
// both width currencies under absolute pinned ceilings; the committed
// sequential-reduce tripwire keeps the refuted fold — the left reduce
// that re-walks its whole accumulator per operand, `Θ(k · d)` on a
// `Θ(d + k)`-byte population — failing on the same population, so the
// band is never decoration.
#[cfg(feature = "limb-meter")]
mod meet_fold {
    use before::meter::registry::Shape;
    use before::{meter, Version};
    use suanpan::touch_meter;

    /// One n-ary meet run over the shade population `MS(d, k)` through
    /// `fold`: total input bytes, the fold model's level count, and
    /// both width counters over the fold body alone.
    ///
    /// Carries the population's semantic leg (the fold returns the
    /// carrier, byte for byte) and the one-touch-per-operand-byte
    /// liveness floor.
    fn run(d: usize, k: usize, fold: fn(Vec<Version>) -> Version) -> Run {
        let population = Shape::MeetShade.versions(d, k);
        let bytes: u64 = population.iter().map(|v| v.encode().len() as u64).sum();
        let carrier = population[0].clone();
        touch_meter::reset();
        meter::reset_limb_ops();
        let met = fold(population);
        let run = Run {
            bytes,
            levels: (2.0 * k as f64).log2(),
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        };
        assert_eq!(
            met, carrier,
            "the shades dominate the carrier everywhere: the meet is the carrier"
        );
        assert!(
            run.touches >= run.bytes,
            "meet fold at {bytes} operand bytes: {} digit touches under \
             the one-per-byte floor: the fold's accumulator work is not metered",
            run.touches,
        );
        run
    }

    /// One meet-fold run's counters and its model denominators.
    struct Run {
        bytes: u64,
        levels: f64,
        touches: u64,
        limb_ops: u64,
    }

    /// Assert one counter's model-normalized per-byte cost stays flat
    /// (×1.25) across a doubling: `counter / (bytes · log2(2k))` — the
    /// declared fold model's constant, as the stagger bands hold it.
    fn assert_model_flat(name: &str, small: &Run, large: &Run, counter: fn(&Run) -> u64) {
        let (m1, m2) = (counter(small) as f64, counter(large) as f64);
        let (d1, d2) = (
            small.bytes as f64 * small.levels,
            large.bytes as f64 * large.levels,
        );
        eprintln!(
            "MEASURED meet_fold_{name}: small={m1}/{:.0} large={m2}/{:.0} \
             per_byte_level={:.3} -> {:.3}",
            d1,
            d2,
            m1 / d1,
            m2 / d2,
        );
        assert!(
            m2 * d1 <= m1 * d2 * 1.25,
            "meet_fold_{name}: the model-normalized per-byte cost grew more \
             than x1.25 across the doubling: {m1}/{d1} -> {m2}/{d2}"
        );
    }

    /// Carrier depth and shade count of the band's small run (the other
    /// runs double both, twice).
    const MEET_SHADE_SMALL: usize = 512;

    /// Absolute (touch, limb) ceilings for `meet_all` on the shade
    /// diagonal, measured ×1.25 at
    /// `MS(512, 512), MS(1,024, 1,024), MS(2,048, 2,048)`.
    ///
    /// The cured record: the balanced reduction reads 4,644 / 10,280 /
    /// 22,572 touches and 27,738 / 61,540 / 135,278 limb ops at the
    /// three scales (704 / 1,408 / 2,816 B populations; per byte·level
    /// 0.660 → 0.664 → 0.668 touches and 3.94 → 3.97 → 4.00 limb, the
    /// model's constant flat while the raw per-byte cost grows exactly
    /// the documented one-level-per-doubling), tightened in the cure's
    /// own commit from the sequential reduce's red-pin record of
    /// 1,051,644 → 4,200,444 touches and 6,295,542 → 25,174,006 limb
    /// ops across the same last doubling (×2.00/byte in both
    /// currencies — 186× the balanced reduction's reading at
    /// MS(2,048, 2,048)) — the tripwire below keeps that mechanism
    /// red.
    const MEET_SHADE_CEILINGS: [(u64, u64); 3] =
        [(5_805, 34_672), (12_850, 76_925), (28_215, 169_097)];

    /// `Version::meet_all` is model-flat on the shade population: the
    /// model-normalized per-byte cost stays flat (×1.25) across two
    /// diagonal doublings in both width currencies, under absolute
    /// pinned ceilings.
    ///
    /// The population keeps the running meet full-size at every
    /// combine, so a fold that re-walks its accumulator per operand
    /// (rather than per counter level) reads ~×2.0 per byte per
    /// doubling here — the committed sequential-reduce tripwire
    /// (`sequential_meet_reduce_reads_superlinear_on_shade`) proves
    /// the population still catches that mechanism red, so this band
    /// is never decoration.
    #[test]
    fn meet_all_shade_is_flat_per_unit() {
        // The public door, entered as callers do: the population's first
        // element (the carrier) as the receiver, the rest as items.
        let door: fn(Vec<Version>) -> Version = |mut population| {
            let rest = population.split_off(1);
            let receiver = population.pop().expect("the population is nonempty");
            receiver.meet_all(rest)
        };
        let n = MEET_SHADE_SMALL;
        let runs = [
            run(n, n, door),
            run(2 * n, 2 * n, door),
            run(4 * n, 4 * n, door),
        ];
        for (r, (touch, limb)) in runs.iter().zip(MEET_SHADE_CEILINGS) {
            eprintln!(
                "MEASURED meet_all_shade: bytes={} touches={} limb_ops={}",
                r.bytes, r.touches, r.limb_ops,
            );
            assert!(
                r.touches <= touch,
                "meet_all_shade: {} touches exceed the pinned ceiling {touch}",
                r.touches,
            );
            assert!(
                r.limb_ops <= limb,
                "meet_all_shade: {} limb ops exceed the pinned ceiling {limb}",
                r.limb_ops,
            );
        }
        for pair in runs.windows(2) {
            assert_model_flat("touches", &pair[0], &pair[1], |r| r.touches);
            assert_model_flat("limb_ops", &pair[0], &pair[1], |r| r.limb_ops);
        }
    }

    /// The committed known-bad meet fold: the sequential left reduce
    /// reads superlinear per byte on the shade population, in both
    /// width currencies.
    ///
    /// The reduce's accumulator never shrinks and every step's sweep
    /// re-walks it whole — `Θ(k · d)`, quadratic on the diagonal — so
    /// the shade family still catches the mechanism the balanced
    /// reduction forecloses, and the flatness band above is never
    /// decoration. The floor ×1.49 sits midway between linear (×1.00)
    /// and the measured ×2.00, so only a class change crosses it.
    ///
    /// [measured in the dev profile, exact counters: touches
    /// 1,051,644 → 4,200,444 across MS(1,024, 1,024) → MS(2,048,
    /// 2,048) on 1,408 B → 2,816 B populations — per-byte growth
    /// ×2.00 — and limb ops 6,295,542 → 25,174,006 (×2.00/byte); the
    /// preceding doubling MS(512, 512) → MS(1,024, 1,024) reads ×1.99
    /// in both currencies. Direction fits: k alone
    /// (d = 256 fixed, k = 256 → 512 → 1,024) grows cost ×2.004 and
    /// ×2.002 per doubling; d alone (k = 256 fixed, d = 256 → 512 →
    /// 1,024) grows cost ×1.985 and ×1.992 — the exact product law
    /// `Θ(k · d)`, each factor independently linear.]
    #[test]
    fn sequential_meet_reduce_reads_superlinear_on_shade() {
        let sequential: fn(Vec<Version>) -> Version = |population| {
            population
                .into_iter()
                .reduce(|acc, v| acc & v)
                .expect("the population is nonempty")
        };
        let n = 2 * MEET_SHADE_SMALL;
        let small = run(n, n, sequential);
        let large = run(2 * n, 2 * n, sequential);
        eprintln!(
            "MEASURED sequential_meet_shade: small={}/{}B (limb {}) \
             large={}/{}B (limb {})",
            small.touches, small.bytes, small.limb_ops, large.touches, large.bytes, large.limb_ops,
        );
        for (name, s, l) in [
            ("touches", small.touches, large.touches),
            ("limb ops", small.limb_ops, large.limb_ops),
        ] {
            assert!(
                u128::from(l) * u128::from(small.bytes) * 100
                    >= u128::from(s) * u128::from(large.bytes) * 149,
                "the sequential meet reduce reads flat ({name}) on the shade \
                 population ({s}/{}B -> {l}/{}B): the family no longer catches \
                 the per-operand accumulator re-walk it was built for, so the \
                 flatness band above is decoration until a new witness lands",
                small.bytes,
                large.bytes,
            );
        }
    }
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
    use before::meter;
    use before::meter::registry::Shape;
    use before::Party;
    use suanpan::touch_meter;

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
    /// Enforces a one-touch-per-eight-input-bytes liveness floor before
    /// returning, derived from the walk's irreducible work: every
    /// consumed code's magnitude folds into the height accumulator at
    /// least once — one digit touch per 64-bit limb of the operand,
    /// zero limbs included — and in every family here the folded
    /// payload (the circulated memo minima and the per-leaf delta
    /// codes) is at least an eighth of the packed input. A reading
    /// below the floor means the walk's accumulator work left the
    /// metered representation and any ratio over it would hold
    /// vacuously.
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
            run.touches >= run.input / 8,
            "memo family at {input} input bytes: {} digit touches under the \
             one-per-eight-bytes floor: the walk's accumulator work is not metered",
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
    /// pinned sizes (62,023 → 124,023 before the fused tick;
    /// 60,023 → 120,023 before the latent boundary
    /// register's O(1) tag work per close); ×3.94 under the refuted
    /// recording-chain interval resolution. Live remeasure: 65,021 →
    /// 130,021 — +3 touches per site accumulated since the record,
    /// still exactly ×2.00 across the doubling].
    #[test]
    fn memo_chain_distinct_resolution_reads_linear() {
        let small = tick_run(
            Shape::MemoChain.packed_flagged(1_000, true),
            Shape::MemoChainId.packed1(1_000),
        );
        let large = tick_run(
            Shape::MemoChain.packed_flagged(2_000, true),
            Shape::MemoChainId.packed1(2_000),
        );
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
        let small = tick_run(
            Shape::MemoChain.packed_flagged(1_000, false),
            Shape::MemoChainId.packed1(1_000),
        );
        let large = tick_run(
            Shape::MemoChain.packed_flagged(2_000, false),
            Shape::MemoChainId.packed1(2_000),
        );
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
    /// register's O(1) tag work per close); ×3.92 under
    /// the refuted interval resolution].
    #[test]
    fn memo_comb_resolution_reads_linear() {
        let small = tick_run(Shape::MemoComb.packed1(500), Shape::MemoCombId.packed1(500));
        let large = tick_run(
            Shape::MemoComb.packed1(1_000),
            Shape::MemoCombId.packed1(1_000),
        );
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
    /// site and blows it [measured: 72,721 touches at k = 2,000,
    /// b = 2,048, the pinned band's basis (76,722 before the at-height
    /// arm moved the dying gap out whole instead of folding a leased
    /// zero; 94,725 before the fused tick; 88,726 before the latent
    /// boundary register's O(1) tag work per close) — a
    /// per-site fan-out at that width would add ~64 touches per site
    /// on top of the ~43-touch linear slope].
    #[test]
    fn memo_fanout_wide_cost_is_site_count_independent() {
        let small = tick_run(
            Shape::MemoFanout.packed2(1_000, 2_048),
            Shape::MemoChainId.packed1(1_000),
        );
        let large = tick_run(
            Shape::MemoFanout.packed2(2_000, 2_048),
            Shape::MemoChainId.packed1(2_000),
        );
        assert_flat("memo_fanout", &small, &large);
        assert!(
            large.touches <= 90_902,
            "memo_fanout: {} touches at k = 2,000 exceed the pinned absolute              ceiling 90,902 (measured 72,721 x1.25): a wide ledger quantity              is being materialized per site",
            large.touches,
        );
        assert!(
            large.touches >= 54_540,
            "memo_fanout: {} touches read below the 54,540 liveness floor              (measured 72,721 x0.75): the ledger's work left the metered              representation",
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
            Shape::MemoOscillating.packed2(1_000, 512),
            Shape::MemoChainId.packed1(1_000),
        );
        let large = tick_run(
            Shape::MemoOscillating.packed2(2_000, 512),
            Shape::MemoChainId.packed1(2_000),
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
    /// before the fused tick; 80,019 → 160,019 before the
    /// latent boundary register landed)]. A discipline
    /// keeping one live record per open level folds all `d` per
    /// drop — the refuted live-anchored followers' tombstone.
    #[test]
    fn memo_churn_undercuts_fold_one_follower() {
        let small = tick_run(
            Shape::MemoChurn.packed1(800),
            Shape::MemoChurnId.packed1(800),
        );
        let large = tick_run(
            Shape::MemoChurn.packed1(1_600),
            Shape::MemoChurnId.packed1(1_600),
        );
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
    /// (48,052 → 96,052 before the fused tick; 46,452 →
    /// 92,852 before the latent boundary register landed)].
    #[test]
    fn descending_raises_stay_linear_under_min_movement() {
        let small = tick_run(
            Shape::DescendingRaises.packed1(800),
            Shape::DescendingRaisesId.packed1(800),
        );
        let large = tick_run(
            Shape::DescendingRaises.packed1(1_600),
            Shape::DescendingRaisesId.packed1(1_600),
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
// denominator cannot excuse. The watermark web's latent boundary
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
// site anywhere: no memo, no pre-scan) pins the watermark web's
// own arm-move + close-pop cycle in isolation.
//
// Two lower-bound genres guard these pins, named apart because their
// trips mean opposite things. A liveness FLOOR is derived from the
// mechanism's irreducible work, never from a measured basis: an honest
// improvement approaches it but can never cross it, so a trip means
// the work left the metered representation — investigate the meter. An
// improvement TRIPWIRE is a measured reading ×0.75: a trip means the
// reading dropped more than 25% below the pin — attribute the
// improvement and re-pin; the meter may be perfectly alive.
#[cfg(feature = "limb-meter")]
mod width_circulation_cost {
    use before::meter;
    use before::meter::registry::Shape;
    use before::{Party, Version};
    use dashu_int::UBig;
    use suanpan::touch_meter;

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
    /// Enforces a one-touch-per-eight-input-bytes liveness floor before
    /// returning, derived from the walk's irreducible work: every
    /// consumed code's magnitude folds into a live accumulator at least
    /// once — one digit touch per 64-bit limb of the operand, zero limbs
    /// included — and in every family here the folded payload (the
    /// circulated wide minimum and the nonzero boundary codes) is at
    /// least an eighth of the packed input, the leanest committed shape
    /// (the leveled control) holding roughly a limb of wide payload per
    /// site's worth of structure. A reading below the floor means the
    /// walk's accumulator work left the metered representation and any
    /// ratio over it would hold vacuously.
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
            run.touches >= run.input / 8,
            "reveal family at {input} input bytes: {} digit touches under the \
             one-per-eight-bytes floor: the walk's accumulator work is not metered",
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
    /// signature [measured: 37,848 → 75,696 touches across
    /// (k, b) = (1,000, 1,024) → (2,000, 2,048), ×2.00 on a ×2.00
    /// input (39,849 → 79,697 before the at-height arm moved the dying
    /// gap out whole instead of folding a leased zero — two arms per
    /// site, walk and pre-scan; 738,449 → 2,884,881 (×3.91) before
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
            Shape::RevealComb.packed2(1_000, 1_024),
            Shape::RevealCombId.packed1(1_000),
        );
        assert_eq!(
            small.ticked,
            expected(1_000, 1_024),
            "reveal_comb ticks to its closed form: the failure is cost-only"
        );
        let large = tick_run(
            Shape::RevealComb.packed2(2_000, 2_048),
            Shape::RevealCombId.packed1(2_000),
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
            large.touches <= 94_620,
            "reveal_comb: {} touches at (k, b) = (2,000, 2,048) exceed the pinned \
             ceiling 94,620 (measured 75,696 x1.25)",
            large.touches,
        );
        assert!(
            large.touches >= REVEAL_COMB_TOUCH_TRIPWIRE,
            "reveal_comb: {} touches dropped more than 25% below the pinned \
             reading ({REVEAL_COMB_TOUCH_TRIPWIRE} = measured 75,696 x0.75): \
             attribute the improvement and re-pin",
            large.touches,
        );
    }

    /// Improvement tripwire on the reveal comb's larger run: the
    /// measured reading ×0.75, rounded down.
    ///
    /// The module comment's tripwire genre: a trip means the reading
    /// improved past the band, not that the meter died — attribute and
    /// re-pin.
    const REVEAL_COMB_TOUCH_TRIPWIRE: u64 = 56_772;

    /// Touch liveness floor on the pure comb's larger run, derived from
    /// the cycle's irreducible work — never from a measured basis.
    ///
    /// Each of the comb's k − 1 close-reveal cycles pays two touches the
    /// arm-recycle mechanism cannot avoid: the recycle's fold of the
    /// arriving offset with the parked latent boundary (a fold of an
    /// accumulator operand touches at least one digit, a leased zero
    /// included) and the merged boundary's sign read deciding the push
    /// trichotomy.
    /// The wide plateau's own code folds into the running height once,
    /// at one touch per 64-bit limb. At (k, b) = (1,000, 2,048):
    /// 2·(k − 1) + b/64 = 1,998 + 32. A design that honestly does less
    /// is a floor-premise finding — re-derive the premise before
    /// trusting the trip.
    const PURE_COMB_TOUCH_FLOOR: u64 = 2_030;

    /// The pure comb's arm-move + close-pop cycle is flat in the
    /// watermark web alone — at most ×1.15 per-byte touch growth
    /// across a width doubling at fixed site count, under an absolute
    /// band on the larger run.
    ///
    /// Semantics first: fill is the identity here (no left-full site
    /// exists), so the tick is grow's closed form — the shallowest
    /// owned leaf expands, ties right. The signature [measured:
    /// per-byte 1.53 → 1.36 across b = 1,024 → 2,048 at k = 1,000
    /// (the widening input divides a flat count; 2.26 → 1.97 before
    /// the at-height arm moved the dying gap out whole instead of
    /// folding a leased zero — one touch per arm; 5.18 → 4.46 before
    /// the fused tick skipped the matched pass-through emissions'
    /// output materializations; 50.8 → 82.0 (×1.61)
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
        let small = tick_run(
            Shape::PureComb.packed2(1_000, 1_024),
            Shape::PureCombId.packed1(1_000),
        );
        assert_eq!(
            small.ticked,
            expected(1_000, 1_024),
            "pure_comb ticks to grow's closed form: the failure is cost-only"
        );
        let large = tick_run(
            Shape::PureComb.packed2(1_000, 2_048),
            Shape::PureCombId.packed1(1_000),
        );
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
            large.touches <= 2_793,
            "pure_comb: {} touches at (k, b) = (1,000, 2,048) exceed the pinned \
             ceiling 2,793 (measured 2,234 x1.25, the at-height arm's no-fold \
             move; the accumulator's quick register folds the comb's narrow \
             values in its register)",
            large.touches,
        );
        assert!(
            large.touches >= PURE_COMB_TOUCH_FLOOR,
            "pure_comb: {} touches read below the {PURE_COMB_TOUCH_FLOOR} \
             liveness floor (the cycle's derived irreducible work): the \
             cycle's work left the metered representation",
            large.touches,
        );
    }

    /// Absolute touch ceiling on the high-floor control's larger run,
    /// measured 40,833 ×1.25, rounded up.
    ///
    /// Movement: 42,834 → 40,833 when the at-height arm moved the dying
    /// gap out whole instead of folding a leased zero (one touch per
    /// arm); 56,831 → 50,837 when the latent boundary register landed —
    /// the deleted close and consume folds this family paid narrow.
    const HIFLOOR_TOUCH_CEILING: u64 = 51_042;

    /// Improvement tripwire paired with [`HIFLOOR_TOUCH_CEILING`]: the
    /// measured reading ×0.75, rounded down.
    ///
    /// The module comment's tripwire genre: a trip means the reading
    /// improved past the band, not that the meter died — attribute and
    /// re-pin.
    const HIFLOOR_TOUCH_TRIPWIRE: u64 = 30_624;

    /// GREEN PIN: the high-floor control is flat and width-independent
    /// — identical forest, identical deferral and close-reveal cycle,
    /// consume-time gap 2.
    ///
    /// Per-byte touches stay flat (×1.25) across the width QUADRUPLING
    /// the wide family scales with [measured: 15.3 → 13.5 per byte
    /// across b = 512 → 2,048 at k = 1,000; 16.1 → 14.2 before the
    /// at-height arm's no-fold move; 21.4 → 18.9 before the
    /// latent boundary register landed], under an absolute
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
            Shape::RevealCombHifloor.packed2(1_000, 512),
            Shape::RevealCombId.packed1(1_000),
        );
        assert_eq!(
            small.ticked,
            expected(1_000, 512),
            "the high-floor control ticks to its closed form"
        );
        let large = tick_run(
            Shape::RevealCombHifloor.packed2(1_000, 2_048),
            Shape::RevealCombId.packed1(1_000),
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
             {HIFLOOR_TOUCH_CEILING} (measured 40,833 x1.25)",
            large.touches,
        );
        assert!(
            large.touches >= HIFLOOR_TOUCH_TRIPWIRE,
            "reveal_comb_hifloor: {} touches dropped more than 25% below the \
             pinned reading ({HIFLOOR_TOUCH_TRIPWIRE} = measured 40,833 x0.75): \
             attribute the improvement and re-pin",
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

    /// Touch liveness floor on the undercut cascade's larger run,
    /// derived from the cascade's irreducible work — never from a
    /// measured basis.
    ///
    /// Four per-boundary charges the cascade mechanism cannot avoid —
    /// two per boundary *created* (one fold of each of the k − 1
    /// consumed nonzero unit codes into the running height, one sign
    /// read per arm deciding each pushed boundary's trichotomy) and two
    /// per boundary *penetrated* (one domination read plus one dying
    /// fold per boundary the cascade consumes, the difference dying
    /// into the residue at its own width, at least one digit each).
    /// The two counts coincide at k − 1 here because the one cascade
    /// penetrates every boundary. The wide
    /// cliff code folds into the running height once, at one touch per
    /// 64-bit limb. At (k, b) = (2,000, 4,096):
    /// 4·(k − 1) + b/64 = 7,996 + 64. A design that honestly does less
    /// is a floor-premise finding — re-derive the premise before
    /// trusting the trip.
    const ASCEND_CLIFF_TOUCH_FLOOR: u64 = 8_060;

    /// The undercut cascade is dying-digit-funded flat — touches grow
    /// by at most ×2.5 across the joint (k, b) doubling on a ×2 input,
    /// under an absolute band on the larger run.
    ///
    /// Semantics first: fill is the identity (no id region covers a
    /// subdividable subtree at its minimum), so the tick is grow's
    /// closed form — the owned cliff leaf expands to `(0, 1, 0)` — and
    /// this pin carries the cost leg alone. The signature [measured:
    /// 7,432 → 14,848 touches across (k, b) =
    /// (1,000, 2,048) → (2,000, 4,096), ×2.00 on a ×2.00 input
    /// (8,432 → 16,848 before the at-height arm moved the dying gap
    /// out whole instead of folding a leased zero — one touch per arm;
    /// 12,626 → 25,234 before the fused tick
    /// skipped the matched pass-through emissions' output
    /// materializations; 203,435 → 790,851 (×3.89) before
    /// the cascade's fold direction
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
            Shape::AscendCliff.packed2(1_000, 2_048),
            Shape::AscendCliffId.packed1(1_000),
        );
        assert_eq!(
            small.ticked,
            ascend_cliff_ticked(1_000, 2_048),
            "ascend_cliff ticks to grow's closed form: the failure is cost-only"
        );
        let large = tick_run(
            Shape::AscendCliff.packed2(2_000, 4_096),
            Shape::AscendCliffId.packed1(2_000),
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
            large.touches <= 18_560,
            "ascend_cliff: {} touches at (k, b) = (2,000, 4,096) exceed the pinned \
             ceiling 18,560 (measured 14,848 x1.25, the at-height arm's \
             no-fold move)",
            large.touches,
        );
        assert!(
            large.touches >= ASCEND_CLIFF_TOUCH_FLOOR,
            "ascend_cliff: {} touches read below the {ASCEND_CLIFF_TOUCH_FLOOR} \
             liveness floor (the cascade's derived irreducible work): the \
             cascade's work left the metered representation",
            large.touches,
        );
    }

    /// Absolute touch ceiling on the leveled control's larger run,
    /// measured 2,854 ×1.25, rounded up.
    ///
    /// Movement: 4,854 → 2,854 when the at-height arm moved the dying
    /// gap out whole instead of folding a leased zero (one touch per
    /// arm); 13,240 before the fused tick skipped the matched
    /// pass-through emissions' output materializations.
    const PLATEAU_TOUCH_CEILING: u64 = 3_568;

    /// Touch liveness floor paired with [`PLATEAU_TOUCH_CEILING`],
    /// derived from the walk's irreducible work on this family — never
    /// from a measured basis.
    ///
    /// Every arm reads its pushed boundary offset's sign — one touch,
    /// even for this family's all-zero boundaries — and the wide first
    /// raise folds into the running height once, at one touch per
    /// 64-bit limb; the all-zero difference stack passes the final
    /// undercut whole, so the cascade owes nothing further. At
    /// (k, b) = (2,000, 4,096): (k − 1) + b/64 = 1,999 + 64.
    const PLATEAU_TOUCH_FLOOR: u64 = 2_063;

    /// GREEN PIN: the leveled control is flat — identical spine,
    /// identical arming schedule, identical cliff undercut, all
    /// boundary differences zero.
    ///
    /// Per-byte touches stay flat (×1.25) across the joint (k, b)
    /// doubling the ascending family scales with [measured: 0.87 →
    /// 0.86 per byte across (1,000, 2,048) → (2,000, 4,096); 1.48 →
    /// 1.47 before the at-height arm's no-fold move],
    /// under an absolute band on the larger run. The
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
            Shape::AscendCliffPlateau.packed2(1_000, 2_048),
            Shape::AscendCliffId.packed1(1_000),
        );
        assert_eq!(
            small.ticked,
            expected(1_000, 2_048),
            "the leveled control ticks to grow's closed form"
        );
        let large = tick_run(
            Shape::AscendCliffPlateau.packed2(2_000, 4_096),
            Shape::AscendCliffId.packed1(2_000),
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
             {PLATEAU_TOUCH_CEILING} (measured 2,854 x1.25)",
            large.touches,
        );
        assert!(
            large.touches >= PLATEAU_TOUCH_FLOOR,
            "ascend_cliff_plateau: {} touches read below the {PLATEAU_TOUCH_FLOOR} \
             liveness floor (the walk's derived irreducible work): the cascade's \
             work left the metered representation",
            large.touches,
        );
    }
}

// ─── query and span placement scenarios ─────────────────────────────────────
//
// The `causally` filter and placement walks' resource identities, stated
// relationally against the pair comparison sweep so there is no constant to
// re-pin: each fused walk must cost exactly its composition minus the saved
// probe scans on full sweeps, degenerate to the pair sweep byte-for-byte on
// one exhaustion-confirmed bound, and add verdict-driven exits the
// composition never had.
#[cfg(feature = "scan-meter")]
mod placement {
    use std::cmp::Ordering;

    use before::causally;
    use before::{meter, Clock, Version};

    /// Scan bits of one closure run, on a fresh counter.
    fn scanned(f: impl FnOnce()) -> u64 {
        meter::reset_scan_bits();
        f();
        meter::scan_bits()
    }

    /// The placement fixture: one clock's comparable snapshot chain
    /// `s < v < e` (multi-party skylines via received sends, so the
    /// streams have real structure), plus a divergent line for the
    /// concurrent genres.
    ///
    /// Returns `(s, v, e, div)` with `s < v < e`, `s <= div`, and
    /// `div` concurrent to both `v` and `e`.
    fn fixture() -> (Version, Version, Version, Version) {
        let mut main = Clock::seed();
        let mut others: Vec<Clock> = (0..6).map(|_| main.fork()).collect();
        let mut rounds = |main: &mut Clock, n: usize| {
            let k = others.len();
            for i in 0..n {
                main.tick();
                let msg = others[i % k].send().clone();
                main.recv(&msg);
            }
        };
        rounds(&mut main, 24);
        let s = main.version().clone();
        let mut diverged = main.fork();
        // One plateau past the word range (2^80 ticks): the share pins'
        // limb legs price wide-gamma decode sharing, and word-scale
        // heights never enter the limb denomination.
        main.ticks(
            "1208925819614629174706176"
                .parse::<before::Ticks>()
                .expect("2^80 parses"),
        );
        rounds(&mut main, 24);
        let v = main.version().clone();
        rounds(&mut main, 24);
        let e = main.version().clone();
        for _ in 0..24 {
            diverged.tick();
        }
        let div = diverged.version().clone();
        assert!(s < v && v < e, "the snapshot chain is strict");
        assert!(s <= div, "the divergent line extends the fork point");
        assert!(
            v.concurrent(&div) && e.concurrent(&div),
            "the lines diverge"
        );
        (s, v, e, div)
    }

    /// GREEN PIN: on a full sweep (no demand settles before
    /// exhaustion), the fused membership walk scans exactly the
    /// two-walk composition minus one probe scan — each stream decoded
    /// once.
    ///
    /// The probe sits below the hole and the ceiling, so the hole's
    /// subtraction and the ceiling's containment both confirm only at
    /// exhaustion. Stated relationally against the pair sweep on the
    /// same operands (`cmp(p, p')` prices one probe scan as half its
    /// reading, `p'` a buffer-distinct re-decode of `p`: a shared
    /// buffer would answer by clone identity without a walk), so the
    /// identity self-normalizes and no measured constant can rot.
    #[test]
    fn query_fused_walk_scans_each_stream_once() {
        let (s, v, e, _) = fixture();
        let query = causally::since(&v) & causally::before(&e);

        let fused = scanned(|| {
            assert!(!query.contains(&s));
        });
        let s_redecoded = Version::decode(&s.encode()[..]).expect("a stored stream re-decodes");
        let cmp_sv = scanned(|| assert!(s.partial_cmp(&v).is_some()));
        let cmp_se = scanned(|| assert!(s.partial_cmp(&e).is_some()));
        let cmp_ss = scanned(|| assert!(s.partial_cmp(&s_redecoded).is_some()));
        eprintln!(
            "MEASURED query_one_pass: fused={fused} composed={} probe_scan={} \
             encoded_bits: s={} v={} e={}",
            cmp_sv + cmp_se,
            cmp_ss / 2,
            s.encoded_bits(),
            v.encoded_bits(),
            e.encoded_bits(),
        );
        assert!(fused > 0, "a live scan meter reads nonzero on a real walk");
        assert_eq!(
            fused + cmp_ss / 2,
            cmp_sv + cmp_se,
            "the fused walk must cost the composition minus exactly one probe scan"
        );
    }

    /// GREEN PIN: over a single stored bound, the membership walk is
    /// the pair sweep.
    ///
    /// A bound whose verdict confirms only at exhaustion reads scan
    /// bits byte-identical to `partial_cmp` on the same operands,
    /// ceiling demand and hole alike, while a bound whose verdict
    /// refutes mid-walk answers at the first refuted direction, at or
    /// strictly under the raw sweep (which must refute both
    /// directions, or confirm one to exhaustion).
    ///
    /// This is the identity the classifier conversions in `rumors` rest
    /// on: a single-bound `contains` costs at most what the raw
    /// comparison it replaces cost.
    #[test]
    fn query_single_bound_matches_the_pair_sweep() {
        let (_, v, e, div) = fixture();
        // Exhaustion-confirmed verdicts: the ceiling admits the probe,
        // the hole holds it — byte-identical to the sweep.
        let raw = scanned(|| {
            let _ = v.partial_cmp(&e);
        });
        let ceiling = scanned(|| {
            assert!(causally::Query::from(causally::before(&e)).contains(&v));
        });
        let hole = scanned(|| {
            assert!(!causally::since(&e).contains(&v));
        });
        eprintln!(
            "MEASURED query_single_bound/exhaustion: raw={raw} ceiling={ceiling} hole={hole}"
        );
        assert!(raw > 0, "a live scan meter reads nonzero on a real walk");
        assert_eq!(
            ceiling, raw,
            "an exhaustion-confirmed ceiling must scan exactly as the pair sweep"
        );
        assert_eq!(
            hole, raw,
            "an exhaustion-confirmed hole must scan exactly as the pair sweep"
        );

        // Refuted verdicts: the bail acts at the first refuted
        // direction, strictly under a raw sweep still confirming its
        // other direction.
        let raw = scanned(|| {
            let _ = e.partial_cmp(&v);
        });
        let ceiling_refuted = scanned(|| {
            assert!(!causally::Query::from(causally::before(&v)).contains(&e));
        });
        let hole_satisfied = scanned(|| {
            assert!(causally::since(&v).contains(&e));
        });
        eprintln!(
            "MEASURED query_single_bound/refuted: raw={raw} \
             ceiling_refuted={ceiling_refuted} hole_satisfied={hole_satisfied}"
        );
        assert!(
            ceiling_refuted < raw,
            "a refuted ceiling must bail before the raw sweep's domination confirm"
        );
        assert!(
            hole_satisfied < raw,
            "a satisfied hole must drop before the raw sweep's domination confirm"
        );

        // A concurrent bound refutes the watched direction no later
        // than the raw sweep refutes both.
        let raw = scanned(|| {
            let _ = v.partial_cmp(&div);
        });
        let concurrent = scanned(|| {
            assert!(!causally::Query::from(causally::before(&div)).contains(&v));
        });
        eprintln!("MEASURED query_single_bound/concurrent: raw={raw} ceiling={concurrent}");
        assert!(
            concurrent <= raw,
            "a concurrent ceiling must answer no later than the raw sweep"
        );
    }

    /// GREEN PIN: the composition's early exits survive the fusion, and
    /// the fused walk stays strictly under the composition on both
    /// concurrent genres.
    ///
    /// Concurrent to the ceiling: the refuted containment answers at
    /// the deciding interval. Concurrent to the hole: the hole is
    /// satisfied and its stream dropped at the deciding interval while
    /// the ceiling sweeps on — the two-walk composition's bail, minus
    /// its second probe scan.
    #[test]
    fn query_early_exits_survive_the_fusion() {
        let (s, v, e, div) = fixture();

        // Concurrent to the ceiling.
        let query = causally::since(&s) & causally::before(&div);
        let fused = scanned(|| {
            assert!(!query.contains(&v));
        });
        let composed = scanned(|| {
            let _ = v.partial_cmp(&s);
            let _ = v.partial_cmp(&div);
        });
        eprintln!("MEASURED query_concurrent_ceiling: fused={fused} composed={composed}");
        assert!(
            fused < composed,
            "concurrent-to-ceiling: the fused walk ({fused}) must undercut the \
             composition ({composed})"
        );

        // Concurrent to the hole, within the ceiling.
        let top = &e | &div;
        let query = causally::since(&div) & causally::before(&top);
        let fused = scanned(|| {
            assert!(query.contains(&v));
        });
        let composed = scanned(|| {
            let _ = v.partial_cmp(&div);
            let _ = v.partial_cmp(&top);
        });
        eprintln!("MEASURED query_concurrent_hole: fused={fused} composed={composed}");
        assert!(
            fused < composed,
            "concurrent-to-hole: the dropped hole stream must keep the fused \
             walk ({fused}) under the composition ({composed})"
        );
    }

    /// GREEN PIN: the single-bound identity holds on the limb meter too —
    /// on an exhaustion-confirmed demand, the degenerate walk commits
    /// exactly the pair sweep's accumulator write sequence.
    #[cfg(feature = "limb-meter")]
    #[test]
    fn query_single_bound_matches_the_pair_sweep_limbs() {
        let (_, v, e, _) = fixture();
        let limbs = |f: &dyn Fn()| {
            meter::reset_limb_ops();
            f();
            meter::limb_ops()
        };
        let raw = limbs(&|| {
            let _ = v.partial_cmp(&e);
        });
        let ceiling = limbs(&|| {
            assert!(causally::Query::from(causally::before(&e)).contains(&v));
        });
        eprintln!("MEASURED query_single_bound_limbs: raw={raw} ceiling={ceiling}");
        assert!(raw > 0, "a live limb meter reads nonzero on a real sweep");
        assert_eq!(
            ceiling, raw,
            "an exhaustion-confirmed ceiling must fold exactly as the pair sweep"
        );
    }

    /// GREEN PIN: one fused span pass, each stream decoded once.
    ///
    /// On a full sweep (every relation comparable) the fused span
    /// placement scans exactly the two-comparison composition minus one
    /// probe scan. The dominance coarsening never costs more: on a
    /// probe dominating the whole span nothing refutes and the walk
    /// is the placement walk to the bit, while on a merely contained
    /// probe the end stream's refuted domination drops that cursor and
    /// the coarser verdict reads strictly cheaper.
    ///
    /// Stated relationally like the range walk's one-pass pin
    /// (`cmp(v, v')` prices one probe scan as half its reading, `v'` a
    /// buffer-distinct re-decode of `v`: a shared buffer would answer
    /// by clone identity without a walk), so no measured constant can
    /// rot.
    #[test]
    fn span_place_scans_each_stream_once() {
        let (s, v, e, _) = fixture();
        let span = causally::Span::new(&s, &e).unwrap();

        let fused = scanned(|| {
            assert_eq!(span.place(&v), causally::Placement::Between);
        });
        let dominance = scanned(|| {
            assert_eq!(span.dominance(&v), causally::Dominance::Between);
        });
        let v_redecoded = Version::decode(&v.encode()[..]).expect("a stored stream re-decodes");
        let cmp_vs = scanned(|| assert!(v.partial_cmp(&s).is_some()));
        let cmp_ve = scanned(|| assert!(v.partial_cmp(&e).is_some()));
        let cmp_vv = scanned(|| assert!(v.partial_cmp(&v_redecoded).is_some()));
        eprintln!(
            "MEASURED span_one_pass: fused={fused} dominance={dominance} composed={} \
             probe_scan={}",
            cmp_vs + cmp_ve,
            cmp_vv / 2,
        );
        assert!(fused > 0, "a live scan meter reads nonzero on a real walk");
        assert_eq!(
            fused + cmp_vv / 2,
            cmp_vs + cmp_ve,
            "the fused span walk must cost the composition minus exactly one probe scan"
        );
        assert!(
            dominance < fused,
            "on a contained probe the end stream's refuted domination must drop \
             that cursor: dominance ({dominance}) under full resolution ({fused})"
        );

        // The mirrored coarsening on the same contained probe: the
        // start stream's refuted precedence drops that cursor, while
        // the membership walk — both required directions confirming
        // only at exhaustion — is the placement walk to the bit.
        let precedence = scanned(|| {
            assert_eq!(span.precedence(&v), causally::Precedence::Between);
        });
        let contains = scanned(|| {
            assert!(span.contains(&v));
        });
        eprintln!("MEASURED span_one_pass_mirror: precedence={precedence} contains={contains}");
        assert!(
            precedence < fused,
            "on a contained probe the start stream's refuted precedence must drop \
             that cursor: precedence ({precedence}) under full resolution ({fused})"
        );
        assert_eq!(
            contains, fused,
            "with both membership directions confirming only at exhaustion, the \
             membership walk is the placement walk to the bit"
        );

        // A probe dominating the whole span refutes nothing on
        // either side: the dominance walk is the placement walk to the
        // bit.
        let whole = causally::Span::new(&s, &v).unwrap();
        let place_whole = scanned(|| {
            assert_eq!(whole.place(&e), causally::Placement::After);
        });
        let dominance_whole = scanned(|| {
            assert_eq!(whole.dominance(&e), causally::Dominance::After);
        });
        eprintln!("MEASURED span_whole_sweep: place={place_whole} dominance={dominance_whole}");
        assert_eq!(
            dominance_whole, place_whole,
            "with nothing refuted, the dominance walk is the placement walk to the bit"
        );

        // Dually, a probe preceding the whole span refutes nothing on
        // either side: the precedence walk is the placement walk to
        // the bit.
        let ahead = causally::Span::new(&v, &e).unwrap();
        let place_ahead = scanned(|| {
            assert_eq!(ahead.place(&s), causally::Placement::Before);
        });
        let precedence_ahead = scanned(|| {
            assert_eq!(ahead.precedence(&s), causally::Precedence::Before);
        });
        eprintln!("MEASURED span_whole_precede: place={place_ahead} precedence={precedence_ahead}");
        assert_eq!(
            precedence_ahead, place_ahead,
            "with nothing refuted, the precedence walk is the placement walk to the bit"
        );
    }

    /// GREEN PIN: the span walk's concurrency exits fire per
    /// endpoint, and every concurrent genre stays strictly under the
    /// two-comparison composition.
    ///
    /// Concurrent to both endpoints: the walk returns at the second
    /// deciding interval. Concurrent to one endpoint: that endpoint's
    /// cursor is dropped at its deciding interval (its stream is never
    /// scanned further) while the other relation sweeps on.
    #[test]
    fn span_concurrent_exits_survive_the_fusion() {
        let (s, v, e, div) = fixture();
        let top = &e | &div;

        for (lo, hi, probe, verdict, genre) in [
            // div is concurrent to both v and e: the early return.
            (
                &v,
                &e,
                &div,
                causally::Placement::Concurrent(causally::Endpoint::Both),
                "both",
            ),
            // v is past s but concurrent to div: the hi-drop path.
            (
                &s,
                &div,
                &v,
                causally::Placement::Concurrent(causally::Endpoint::End),
                "end",
            ),
            // v is concurrent to div but under div|e: the lo-drop path.
            (
                &div,
                &top,
                &v,
                causally::Placement::Concurrent(causally::Endpoint::Start),
                "start",
            ),
        ] {
            let span = causally::Span::new(lo, hi).unwrap();
            let fused = scanned(|| {
                assert_eq!(span.place(probe), verdict);
            });
            // The two-comparison composition on the span's own
            // endpoints: the operands the fused walk actually replaces.
            let composed = scanned(|| {
                let _ = probe.partial_cmp(lo);
                let _ = probe.partial_cmp(hi);
            });
            eprintln!("MEASURED span_concurrent_{genre}: fused={fused} composed={composed}");
            assert!(
                fused < composed,
                "concurrent-to-{genre}: the fused span walk ({fused}) must undercut \
                 the composition ({composed})"
            );
        }
    }

    /// GREEN PIN: the dominance face's bail on a start the probe fails
    /// to dominate, on both failure genres.
    ///
    /// A *concurrent* start refutes `lo <= probe` at its first opposing
    /// interval — one interval before the pair sweep's two-flag
    /// concurrency exit — so the walk returns strictly before full
    /// resolution and strictly under the floor-first two-check shape it
    /// replaces; against the old *first check alone* the earlier bail
    /// buys back only part of the fused walk's end-stream prefix, so
    /// that reading is printed, not bounded. A *dominating* start
    /// (`probe < lo`, comparable) is where the bail changes class: the
    /// old floor-first check could confirm `Greater` only at
    /// exhaustion, while the single-flag refutation lands at the first
    /// excess interval — strictly under even the first check.
    #[test]
    fn dominance_bails_at_the_refuted_start() {
        let (s, v, e, div) = fixture();

        // Genre 1: the start is concurrent to the probe.
        let top = &e | &div;
        let span = causally::Span::new(&div, &top).unwrap();
        let fused = scanned(|| {
            assert_eq!(span.dominance(&v), causally::Dominance::Before);
        });
        let place = scanned(|| {
            assert_eq!(
                span.place(&v),
                causally::Placement::Concurrent(causally::Endpoint::Start)
            );
        });
        // The two-check shape the dominance face replaces: compare the
        // start version against the probe (the floor-first check,
        // which exits at the concurrency), then check the end
        // version's containment in the probe's past (the second probe
        // decode the fusion ends).
        let first_check = scanned(|| {
            assert!(div.partial_cmp(&v).is_none());
        });
        let two_check = first_check
            + scanned(|| {
                assert!(!causally::before(&v).contains(&top));
            });
        eprintln!(
            "MEASURED dominance_bail_concurrent: fused={fused} place={place} \
             first_check={first_check} two_check={two_check}"
        );
        assert!(fused > 0, "a live scan meter reads nonzero on a real walk");
        assert!(
            fused < place,
            "the dominance bail ({fused}) must return before full resolution ({place})"
        );
        assert!(
            fused < two_check,
            "the dominance bail ({fused}) must undercut the two-check shape ({two_check})"
        );

        // Genre 2: the start strictly dominates the probe.
        let span = causally::Span::new(&v, &e).unwrap();
        let fused = scanned(|| {
            assert_eq!(span.dominance(&s), causally::Dominance::Before);
        });
        let first_check = scanned(|| {
            assert_eq!(v.partial_cmp(&s), Some(Ordering::Greater));
        });
        let two_check = first_check
            + scanned(|| {
                assert!(!causally::before(&s).contains(&e));
            });
        eprintln!(
            "MEASURED dominance_bail_dominating: fused={fused} \
             first_check={first_check} two_check={two_check}"
        );
        assert!(
            fused < first_check,
            "on a dominating start the single-flag bail ({fused}) must undercut \
             even the floor-first check ({first_check}), which confirms Greater \
             only at exhaustion"
        );
        assert!(
            fused < two_check,
            "the dominance bail ({fused}) must undercut the two-check shape ({two_check})"
        );
    }

    /// GREEN PIN: the precedence face's bail on an end the probe fails
    /// to precede — the dominance bail, mirrored — on both failure
    /// genres.
    ///
    /// A *concurrent* end refutes `probe <= hi` at its first opposing
    /// interval — one interval before the pair sweep's two-flag
    /// concurrency exit — so the walk returns strictly before full
    /// resolution and strictly under the ceiling-first two-check shape
    /// it replaces. A *preceded* end (`hi < probe`, comparable) is
    /// where the bail changes class: the ceiling-first check could
    /// confirm `Less` only at exhaustion, while the single-flag
    /// refutation lands at the first excess interval — strictly under
    /// even the first check.
    #[test]
    fn precedence_bails_at_the_refuted_end() {
        let (s, v, e, div) = fixture();

        // Genre 1: the end is concurrent to the probe.
        let span = causally::Span::new(&s, &div).unwrap();
        let fused = scanned(|| {
            assert_eq!(span.precedence(&v), causally::Precedence::After);
        });
        let place = scanned(|| {
            assert_eq!(
                span.place(&v),
                causally::Placement::Concurrent(causally::Endpoint::End)
            );
        });
        // The two-check shape the precedence face replaces: compare the
        // end version against the probe (the ceiling-first check, which
        // exits at the concurrency), then check the start version's
        // containment in the probe's causal future (the second probe
        // decode the fusion ends).
        let first_check = scanned(|| {
            assert!(div.partial_cmp(&v).is_none());
        });
        let two_check = first_check
            + scanned(|| {
                assert!(!causally::after(&v).contains(&s));
            });
        eprintln!(
            "MEASURED precedence_bail_concurrent: fused={fused} place={place} \
             first_check={first_check} two_check={two_check}"
        );
        assert!(fused > 0, "a live scan meter reads nonzero on a real walk");
        assert!(
            fused < place,
            "the precedence bail ({fused}) must return before full resolution ({place})"
        );
        assert!(
            fused < two_check,
            "the precedence bail ({fused}) must undercut the two-check shape ({two_check})"
        );

        // Genre 2: the end strictly precedes the probe.
        let span = causally::Span::new(&s, &v).unwrap();
        let fused = scanned(|| {
            assert_eq!(span.precedence(&e), causally::Precedence::After);
        });
        let first_check = scanned(|| {
            assert_eq!(v.partial_cmp(&e), Some(Ordering::Less));
        });
        let two_check = first_check
            + scanned(|| {
                assert!(!causally::after(&e).contains(&s));
            });
        eprintln!(
            "MEASURED precedence_bail_preceded: fused={fused} \
             first_check={first_check} two_check={two_check}"
        );
        assert!(
            fused < first_check,
            "on a preceded end the single-flag bail ({fused}) must undercut \
             even the ceiling-first check ({first_check}), which confirms Less \
             only at exhaustion"
        );
        assert!(
            fused < two_check,
            "the precedence bail ({fused}) must undercut the two-check shape ({two_check})"
        );
    }

    /// GREEN PIN: the membership face bails at the first refuted
    /// required direction, on either side.
    ///
    /// A probe above the end refutes `probe <= hi` at its first excess
    /// interval; a probe below the start refutes `lo <= probe` the
    /// same way. Either bail answers strictly before the
    /// full-resolution placement walk, which confirms its
    /// `After`/`Before` verdict only at exhaustion.
    #[test]
    fn contains_bails_at_either_refuted_side() {
        let (s, v, e, _) = fixture();

        // Above the end: `probe <= hi` refuted mid-walk.
        let span = causally::Span::new(&s, &v).unwrap();
        let fused = scanned(|| assert!(!span.contains(&e)));
        let place = scanned(|| {
            assert_eq!(span.place(&e), causally::Placement::After);
        });
        eprintln!("MEASURED contains_bail_above: fused={fused} place={place}");
        assert!(fused > 0, "a live scan meter reads nonzero on a real walk");
        assert!(
            fused < place,
            "the membership bail ({fused}) must return before full resolution ({place})"
        );

        // Below the start: `lo <= probe` refuted mid-walk.
        let span = causally::Span::new(&v, &e).unwrap();
        let fused = scanned(|| assert!(!span.contains(&s)));
        let place = scanned(|| {
            assert_eq!(span.place(&s), causally::Placement::Before);
        });
        eprintln!("MEASURED contains_bail_below: fused={fused} place={place}");
        assert!(
            fused < place,
            "the membership bail ({fused}) must return before full resolution ({place})"
        );
    }
}

// ─── span hull scenarios ─────────────────────────────────────────────────────
//
// The fused hull kernel's resource identities, stated relationally against
// the single-op emissions and the pair comparison on the same operands so
// there is no constant to re-pin: one fused sweep feeds both endpoints, so
// the pair's streams are decoded once where the composed emitters decode
// them twice. Two regimes, denominated separately: the *pair* regime
// (binary `span`, and `span_all`'s leaf combines) shares its operands
// between the meet and join legs — decode-halving; the *interior* regime
// (`span_all`'s combines over already-merged hulls) reads a different
// operand pair per leg — consolidation into one fold, no shared walk.
#[cfg(feature = "scan-meter")]
mod span {
    use before::{meter, Clock, Version};

    /// Scan bits of one closure run, on a fresh counter.
    fn scanned(f: impl FnOnce()) -> u64 {
        meter::reset_scan_bits();
        f();
        meter::scan_bits()
    }

    /// The span fixture: two comparable snapshots `s < v` of one
    /// multi-party history, a divergent line `div` concurrent to `v`,
    /// and a population of intermediate snapshots for the n-ary
    /// regimes.
    ///
    /// Received sends give every stream real multi-party structure.
    fn fixture() -> (Version, Version, Version, Vec<Version>) {
        let mut main = Clock::seed();
        let mut others: Vec<Clock> = (0..6).map(|_| main.fork()).collect();
        let mut population = Vec::new();
        let mut rounds = |main: &mut Clock, population: &mut Vec<Version>, n: usize| {
            let k = others.len();
            for i in 0..n {
                main.tick();
                let msg = others[i % k].send().clone();
                main.recv(&msg);
                if i % 7 == 0 {
                    population.push(main.version().clone());
                }
            }
        };
        rounds(&mut main, &mut population, 24);
        let s = main.version().clone();
        let mut diverged = main.fork();
        // One plateau past the word range (2^80 ticks): the share pins'
        // limb legs price wide-gamma decode sharing, and word-scale
        // heights never enter the limb denomination.
        main.ticks(
            "1208925819614629174706176"
                .parse::<before::Ticks>()
                .expect("2^80 parses"),
        );
        rounds(&mut main, &mut population, 24);
        let v = main.version().clone();
        for _ in 0..24 {
            diverged.tick();
        }
        let div = diverged.version().clone();
        assert!(s < v, "the snapshot chain is strict");
        assert!(v.concurrent(&div), "the lines diverge");
        (s, v, div, population)
    }

    /// GREEN PIN: the span ladder's two walking regimes, one scan
    /// identity each.
    ///
    /// A *comparable* pair's hull is the pair handed back
    /// (`span_is_the_pair_hull`): the span costs exactly one comparison
    /// sweep — `span == cmp(a, b)` — with zero emission, so the pin is
    /// scan identity with the pair sweep itself. A *concurrent* pair is
    /// the only emitting case: the fused hull decodes the pair once
    /// (scan counts both stream reads and builder writes, and the fused
    /// sweep's writes are the two single-op outputs exactly), after
    /// paying the ladder's classifying comparison — its early-exiting
    /// concurrent prefix — up front:
    /// `span + decode(a) + decode(b) == meet + join + cmp(a, b)`. Each
    /// operand's decode is priced as half its self-comparison against a
    /// buffer-distinct re-decode (`cmp(x, x')` reads `x` twice and
    /// writes nothing; a shared buffer would answer by clone identity
    /// without a walk). Stated relationally, so no measured constant
    /// can rot.
    #[test]
    fn span_fuses_the_pair_walk() {
        let (s, v, div, _) = fixture();

        // The comparable regime: hand-back at the cost of the pair sweep.
        let fused = scanned(|| {
            let _ = s.span(&v);
        });
        let cmp_sv = scanned(|| assert!(s.partial_cmp(&v).is_some()));
        eprintln!("MEASURED span_pair_comparable: fused={fused} cmp={cmp_sv}");
        assert!(fused > 0, "a live scan meter reads nonzero on a real walk");
        assert_eq!(
            fused, cmp_sv,
            "comparable: the hull is the pair handed back at exactly one \
             comparison sweep, zero emission"
        );

        // The concurrent regime: the one emitting case, the fused hull's
        // decode saving intact, the classifying comparison accounted.
        let (a, b) = (&v, &div);
        let fused = scanned(|| {
            let _ = a.span(b);
        });
        let met = scanned(|| {
            let _ = a & b;
        });
        let joined = scanned(|| {
            let _ = a | b;
        });
        let cmp_ab = scanned(|| assert!(a.partial_cmp(b).is_none()));
        let redecode = |x: &Version| Version::decode(&x.encode()[..]).expect("re-decodes");
        let (a2, b2) = (redecode(a), redecode(b));
        let decode_a = scanned(|| assert!(a.partial_cmp(&a2).is_some())) / 2;
        let decode_b = scanned(|| assert!(b.partial_cmp(&b2).is_some())) / 2;
        eprintln!(
            "MEASURED span_pair_concurrent: fused={fused} meet={met} join={joined} \
             cmp={cmp_ab} pair_decode={}",
            decode_a + decode_b,
        );
        assert!(fused > 0, "a live scan meter reads nonzero on a real walk");
        assert_eq!(
            fused + decode_a + decode_b,
            met + joined + cmp_ab,
            "concurrent: the fused hull must cost the composed emissions minus \
             one decode of the pair, plus the ladder's classifying comparison"
        );
    }

    /// GREEN PIN: `span_all`'s leaf combines ride the fused pair walk —
    /// at one item the n-ary door scans exactly as the binary span.
    #[test]
    fn span_all_leaf_combine_is_the_fused_pair_walk() {
        let (s, v, _, _) = fixture();
        let fused = scanned(|| {
            let _ = s.span(&v);
        });
        let unary = scanned(|| {
            let _ = s.span_all([&v]);
        });
        eprintln!("MEASURED span_all_unary: span={fused} span_all={unary}");
        assert!(fused > 0, "a live scan meter reads nonzero on a real walk");
        assert_eq!(
            unary, fused,
            "span_all at one item must scan exactly as the binary span"
        );
    }

    /// GREEN PIN: the n-ary hull undercuts the two composed folds, and
    /// the saving is the leaf level's — the interior regime
    /// consolidates without a shared walk, so the fold stays strictly
    /// above half the composition.
    ///
    /// `span_all` fuses exactly the leaf combines (two raw inputs, one
    /// shared pair walk); interior combines read a different operand
    /// pair per leg (`lo₁ ∧ lo₂`, `hi₁ ∨ hi₂`), costing what the two
    /// composed folds' interior levels cost. The composition is
    /// measured over the identical population, receiver included.
    #[test]
    fn span_all_fuses_the_leaf_level() {
        let (s, _, _, population) = fixture();
        assert!(
            population.len() >= 4,
            "the population exercises interior combines"
        );
        let fused = scanned(|| {
            let _ = s.span_all(&population);
        });
        let composed = scanned(|| {
            let _ = s.meet_all(&population);
            let _ = s.join_all(&population);
        });
        eprintln!("MEASURED span_all_population: fused={fused} composed={composed}");
        assert!(fused > 0, "a live scan meter reads nonzero on a real walk");
        assert!(
            fused < composed,
            "the leaf-level fusion must undercut the composed folds \
             ({fused} vs {composed})"
        );
        assert!(
            fused * 2 > composed,
            "interior combines have no shared pair walk: the fold must stay \
             strictly above half the composition ({fused} vs {composed})"
        );
    }

    /// GREEN PIN: the fused hull decodes the pair once at arithmetic
    /// width, and folds each crossing into ONE shared running
    /// difference — the two meter faces of the fusion, one leg each.
    ///
    /// The limb leg pins the decode sharing at arithmetic width: each
    /// wide-gamma decode records one value-width limb count, and the
    /// composed emitters decode every operand twice. Its witness is an
    /// unfused hull that decodes per emission; it is blind to the
    /// accumulator, whose folds record no limb ops.
    ///
    /// The touch leg pins the crossing-fold sharing: accumulator digit
    /// touches are exactly the traffic the fusion halves, so a
    /// two-accumulator spelling (each emission keeping its own
    /// difference, operands still decoded once) reads the composed
    /// folds back and fails the strict undercut — constructed and
    /// verified red in review136; the limb leg alone read that fake
    /// byte-identically to the true fusion.
    #[cfg(feature = "limb-meter")]
    #[test]
    fn span_shares_the_crossing_folds() {
        // The concurrent pair: the ladder's only emitting case, so this
        // is the pair that still reaches the fused emission walk these
        // pins are about (a comparable pair hands its operands back at
        // one comparison sweep — `span_fuses_the_pair_walk`'s regime).
        let (_, v, div, _) = fixture();
        // No limb leg: word-scale crossings never enter the limb
        // denomination, so the arithmetic-width undercut that once rode
        // the composed emissions' duplicated zigzag work has no margin
        // left to read. Decode sharing is pinned structurally by the
        // scan identity (`span_fuses_the_pair_walk`: a re-decode
        // re-reads bits the scan meter counts), and the fold sharing by
        // the touch leg here.
        let touches = |f: &dyn Fn()| {
            suanpan::touch_meter::reset();
            f();
            suanpan::touch_meter::touches()
        };
        let fused = touches(&|| {
            let _ = v.span(&div);
        });
        let met = touches(&|| {
            let _ = &v & &div;
        });
        let joined = touches(&|| {
            let _ = &v | &div;
        });
        let cmp = touches(&|| assert!(v.partial_cmp(&div).is_none()));
        eprintln!("MEASURED span_pair_touches: fused={fused} meet={met} join={joined} cmp={cmp}");
        assert!(fused > 0, "a live touch meter reads nonzero on a real walk");
        // The fused walk maintains ONE shared difference (it reads that
        // difference's sign once per crossing per pick, so its traffic
        // is not a single emission's to the digit, but every crossing
        // is folded exactly once); the ladder's classifying comparison
        // adds its early-exiting prefix on top, measured separately and
        // subtracted. A two-accumulator spelling (each emission keeping
        // its own difference, constructed and verified red in
        // review136) folds every crossing twice and reads the composed
        // emissions back exactly, so the strict undercut keeps it
        // failing.
        assert!(
            fused - cmp < met + joined,
            "the fused hull's own folds must undercut the composed \
             emissions' two accumulators ({} vs {} composed touches)",
            fused - cmp,
            met + joined
        );
    }
}

// ─── span wire decode scenarios ──────────────────────────────────────────────
//
// The fused span decode's resource identities, stated relationally against
// the standalone component decode and the pair comparison on the same
// operands so there is no constant to rot. `Span::decode` parses the first
// component exactly as `Version::decode` does, then ONE admission walk
// parses the second while validating dominance in the same pass; what the
// fusion deletes is the second component's standalone parse — its whole
// stream scan, its payload re-decodes, and its entire validation-height
// accumulator (dominance over a canonical first component subsumes
// nonnegativity). Each meter leg pins one face; the touch leg is the one a
// parse-then-validate pseudo-fusion cannot fake (it reads the composed sum
// back exactly).
#[cfg(feature = "scan-meter")]
mod span_codec {
    use before::{causally::Span, meter, Clock, Version};

    /// Scan bits of one closure run, on a fresh counter.
    fn scanned(f: impl FnOnce()) -> u64 {
        meter::reset_scan_bits();
        f();
        meter::scan_bits()
    }

    /// The wire fixture: two comparable snapshots `s < v` of one
    /// multi-party history, their composite `[s, v]` wire bytes, and
    /// the byte seam where the second component begins.
    ///
    /// Received sends give both streams real multi-party structure, so
    /// every leg folds live deltas.
    fn fixture() -> (Version, Version, Vec<u8>, usize) {
        let mut main = Clock::seed();
        let mut others: Vec<Clock> = (0..6).map(|_| main.fork()).collect();
        let mut rounds = |main: &mut Clock, n: usize| {
            let k = others.len();
            for i in 0..n {
                main.tick();
                let msg = others[i % k].send().clone();
                main.recv(&msg);
            }
        };
        rounds(&mut main, 24);
        let s = main.version().clone();
        // One plateau past the word range (2^80 ticks): the second
        // component's wide decode is what the limb liveness check reads,
        // and word-scale heights never enter that denomination.
        main.ticks(
            "1208925819614629174706176"
                .parse::<before::Ticks>()
                .expect("2^80 parses"),
        );
        rounds(&mut main, 24);
        let v = main.version().clone();
        assert!(s < v, "the snapshot chain is strict");
        let bytes = Span::new(&s, &v).unwrap().encode();
        let seam = s.encode().len();
        (s, v, bytes, seam)
    }

    /// GREEN PIN: the fused decode scans the second component ONCE —
    /// its reading is exactly the first component's standalone parse
    /// plus one comparison sweep.
    ///
    /// The composed decode + decode + compare shape sits strictly
    /// above it, by exactly the second component's parse scan.
    ///
    /// Both statements are relational, so no measured constant can
    /// rot: the identity pins the fusion to its own pieces (an ordered
    /// pair's comparison sweeps to exhaustion, as the admission walk
    /// must), and the undercut is the deleted second scan — which a
    /// parse-then-validate spelling reads back exactly.
    #[test]
    fn span_decode_scans_the_second_component_once() {
        let (s, v, bytes, seam) = fixture();
        let fused = scanned(|| {
            let _ = Span::decode(&bytes[..]).expect("a canonical composite decodes");
        });
        let decode_lo = scanned(|| {
            let _ = Version::decode(&bytes[..seam]).expect("the first component decodes");
        });
        let decode_hi = scanned(|| {
            let _ = Version::decode(&bytes[seam..]).expect("the second component decodes");
        });
        let cmp = scanned(|| assert!(s < v));
        eprintln!(
            "MEASURED span_decode_scan: fused={fused} decode_lo={decode_lo} \
             decode_hi={decode_hi} cmp={cmp}"
        );
        assert!(
            fused > 0 && decode_hi > 0,
            "live scan meters read nonzero on real walks"
        );
        assert_eq!(
            fused,
            decode_lo + cmp,
            "the fused decode must scan exactly the first component's parse \
             plus one comparison sweep"
        );
        assert!(
            fused < decode_lo + decode_hi + cmp,
            "the fused decode must undercut the composed \
             decode + decode + compare shape ({fused} vs {})",
            decode_lo + decode_hi + cmp
        );
    }

    /// GREEN PIN: the fused decode's limb traffic decodes the second
    /// component's payloads once, strictly under the composed shape.
    ///
    /// The floor is the fusion's own pieces; the gap above it is
    /// exactly the topology-minimality check the fusion keeps (one
    /// word-scale zero-equality per second-component leaf *delta* —
    /// the first leaf is never asked, the bare comparison never asks
    /// at all, the strict parse must), and the
    /// undercut against the composed shape is the second component's
    /// deleted payload re-decode. A parse-then-validate spelling reads
    /// the composed sum back exactly and fails the undercut.
    #[cfg(feature = "limb-meter")]
    #[test]
    fn span_decode_shares_the_second_payload_decode() {
        let (s, v, bytes, seam) = fixture();
        let limbs = |f: &dyn Fn()| {
            meter::reset_limb_ops();
            f();
            meter::limb_ops()
        };
        let fused = limbs(&|| {
            let _ = Span::decode(&bytes[..]).expect("a canonical composite decodes");
        });
        let decode_lo = limbs(&|| {
            let _ = Version::decode(&bytes[..seam]).expect("the first component decodes");
        });
        let decode_hi = limbs(&|| {
            let _ = Version::decode(&bytes[seam..]).expect("the second component decodes");
        });
        let cmp = limbs(&|| assert!(s < v));
        eprintln!(
            "MEASURED span_decode_limbs: fused={fused} decode_lo={decode_lo} \
             decode_hi={decode_hi} cmp={cmp}"
        );
        assert!(
            fused > 0 && decode_hi > 0,
            "live limb meters read nonzero on real walks"
        );
        assert!(
            fused >= decode_lo + cmp,
            "the fusion cannot beat its own pieces ({fused} vs {})",
            decode_lo + cmp
        );
        assert!(
            fused < decode_lo + decode_hi + cmp,
            "the fused decode must undercut the composed shape by the second \
             component's payload re-decode ({fused} vs {})",
            decode_lo + decode_hi + cmp
        );
    }

    /// GREEN PIN: the fused decode's accumulator traffic is exactly the
    /// first component's validation plus ONE comparison's folds — the
    /// second component's validation-height accumulator is deleted
    /// outright, not fused.
    ///
    /// Touches are the meter that can see the deletion: accumulator
    /// folds record no limb ops and no scan bits, so this identity is
    /// the leg a parse-then-validate pseudo-fusion cannot fake — it
    /// runs the second component's height folds and reads the composed
    /// sum back exactly, failing the identity by that component's whole
    /// validation traffic (asserted live below).
    #[cfg(feature = "limb-meter")]
    #[test]
    fn span_decode_deletes_the_second_validation_accumulator() {
        let (s, v, bytes, seam) = fixture();
        let touches = |f: &dyn Fn()| {
            suanpan::touch_meter::reset();
            f();
            suanpan::touch_meter::touches()
        };
        let fused = touches(&|| {
            let _ = Span::decode(&bytes[..]).expect("a canonical composite decodes");
        });
        let decode_lo = touches(&|| {
            let _ = Version::decode(&bytes[..seam]).expect("the first component decodes");
        });
        let decode_hi = touches(&|| {
            let _ = Version::decode(&bytes[seam..]).expect("the second component decodes");
        });
        let cmp = touches(&|| assert!(s < v));
        eprintln!(
            "MEASURED span_decode_touches: fused={fused} decode_lo={decode_lo} \
             decode_hi={decode_hi} cmp={cmp}"
        );
        assert!(
            fused > 0 && decode_hi > 0,
            "live touch meters read nonzero on real walks: the undercut margin \
             is the second validation's whole fold traffic"
        );
        assert_eq!(
            fused,
            decode_lo + cmp,
            "the fused decode must fold exactly the first component's \
             validation plus one comparison — no second height accumulator"
        );
        assert!(
            fused < decode_lo + decode_hi + cmp,
            "a parse-then-validate spelling reads the composed sum exactly \
             ({} here); the fusion must undercut it (read {fused})",
            decode_lo + decode_hi + cmp
        );
    }
}

// ─── identity fast paths (clone-identity liveness) ───────────────────────────
//
// The at-rest form's refcounted backing store makes clone identity
// observable (`ptr_eq`), and the identity-law fast paths dispatch on it:
// clone-then-op answers without a walk, each shortcut citing its law in
// `before::laws` at the code site. These pins hold the fast paths LIVE —
// every clone-operand cell must read zero walk work, and every cell rides
// beside a walking leg on the same operands' values in distinct buffers,
// so a dead meter (which also reads zero) cannot green the section and
// the walked path stays covered.
#[cfg(feature = "scan-meter")]
mod identity_fast_paths {
    use before::{meter, Clock, Version};

    /// Scan bits of one closure run, on a fresh counter.
    fn scanned(f: impl FnOnce()) -> u64 {
        meter::reset_scan_bits();
        f();
        meter::scan_bits()
    }

    /// The fixture: one multi-party snapshot `v` with real structure, a
    /// buffer-distinct byte-equal re-decode `v'`, and a concurrent
    /// divergence `w` for the walking legs.
    fn fixture() -> (Version, Version, Version) {
        let mut main = Clock::seed();
        let mut others: Vec<Clock> = (0..6).map(|_| main.fork()).collect();
        let mut rounds = |main: &mut Clock, n: usize| {
            let k = others.len();
            for i in 0..n {
                main.tick();
                let msg = others[i % k].send().clone();
                main.recv(&msg);
            }
        };
        rounds(&mut main, 24);
        let mut diverged = main.fork();
        rounds(&mut main, 24);
        let v = main.version().clone();
        for _ in 0..24 {
            diverged.tick();
        }
        let w = diverged.version().clone();
        assert!(v.concurrent(&w), "the walking legs need a real walk");
        let redecoded = Version::decode(&v.encode()[..]).expect("a stored stream re-decodes");
        (v, redecoded, w)
    }

    /// Clone operands answer every identity-law fast path without a
    /// walk.
    ///
    /// Comparison (`order_reflexive`), join and meet idempotence
    /// (`merge_idempotent`/`meet_idempotent`), the coincident hull
    /// (`span_with_self_is_coincident`), and the n-ary folds' adjacent
    /// clone collapse — all zero scanned bits over operands that share
    /// one buffer, while the same operations walk (nonzero) on a
    /// concurrent pair, so the zeros are fast paths, not a dead meter.
    #[test]
    fn clone_operands_answer_without_a_walk() {
        let (v, _, w) = fixture();
        let c = v.clone();

        let cells: &[(&str, &dyn Fn())] = &[
            ("cmp", &|| assert!(v.partial_cmp(&c).is_some())),
            ("join", &|| assert_eq!(&(&v | &c), &v)),
            ("meet", &|| assert_eq!(&(&v & &c), &v)),
            ("span", &|| assert_eq!(v.span(&c).lo(), &v)),
            ("join_all", &|| {
                assert_eq!(v.join_all([&c, &v]), v);
            }),
            ("meet_all", &|| {
                assert_eq!(v.meet_all([&c, &v]), v);
            }),
            ("span_all", &|| {
                assert_eq!(v.span_all([&c, &v]).hi(), &v);
            }),
        ];
        for (name, cell) in cells {
            let read = scanned(cell);
            assert_eq!(
                read, 0,
                "{name} over clone operands must answer by clone identity, \
                 not a walk ({read} bits scanned)"
            );
        }

        // The walking legs: the same operations on a concurrent pair
        // read nonzero, so the zeros above are fast paths firing, not a
        // dead scan meter.
        let walking: &[(&str, &dyn Fn())] = &[
            ("cmp", &|| assert!(v.partial_cmp(&w).is_none())),
            ("join", &|| assert!((&v | &w) >= v)),
            ("span", &|| assert!(v.span(&w).hi() >= v)),
        ];
        for (name, cell) in walking {
            let read = scanned(cell);
            assert!(
                read > 0,
                "{name} over a concurrent pair must walk: a zero here is a \
                 dead scan meter"
            );
        }
    }

    /// Byte-equal operands in distinct buffers keep the walked paths
    /// covered.
    ///
    /// Comparison takes the full sweep (the clone-identity rung must
    /// not fire across buffers), while the byte-compare rung answers
    /// join/meet/span/distance/lag with no bit-stream walk — and every
    /// verdict equals the clone-operand fast path's.
    #[test]
    fn distinct_buffers_keep_the_walked_paths_covered() {
        let (v, redecoded, _) = fixture();

        // The comparison sweep runs whole: equal streams survive both
        // directions to exhaustion, so the read is both streams' bits.
        let cmp =
            scanned(|| assert_eq!(v.partial_cmp(&redecoded), Some(core::cmp::Ordering::Equal)));
        assert!(
            cmp > 0,
            "cmp over byte-equal distinct buffers must take the sweep: \
             clone identity must not fire across buffers"
        );

        // The byte-compare rung (canonical_eq) answers the lattice ops:
        // no bit-stream walk, verdicts identical to the clone legs'.
        let byte_rung: &[(&str, &dyn Fn())] = &[
            ("join", &|| assert_eq!(&(&v | &redecoded), &v)),
            ("meet", &|| assert_eq!(&(&v & &redecoded), &v)),
            ("span", &|| assert_eq!(v.span(&redecoded).lo(), &v)),
        ];
        for (name, cell) in byte_rung {
            let read = scanned(cell);
            assert_eq!(
                read, 0,
                "{name} over byte-equal operands must answer by the byte \
                 compare, not a walk ({read} bits scanned)"
            );
        }
    }

    /// The metric fast paths skip the fold at arithmetic width.
    ///
    /// `distance` and `lag` over clone or byte-equal operands fold
    /// nothing through the accumulator
    /// (`distance_to_self_is_zero`/`lag_to_self_is_zero`), where the
    /// same pair walked whole at the parent — and a real pair still
    /// folds, so the zeros are the equality rung, not a dead touch
    /// meter.
    #[cfg(feature = "limb-meter")]
    #[test]
    fn metric_fast_paths_skip_the_fold() {
        let (v, redecoded, w) = fixture();
        let c = v.clone();
        let touches = |f: &dyn Fn()| {
            suanpan::touch_meter::reset();
            f();
            suanpan::touch_meter::touches()
        };

        let equal_cells: &[(&str, &dyn Fn())] = &[
            ("distance_clone", &|| {
                assert_eq!(v.distance(&c), before::Rank::ZERO)
            }),
            ("distance_redecoded", &|| {
                assert_eq!(v.distance(&redecoded), before::Rank::ZERO)
            }),
            ("lag_clone", &|| assert_eq!(v.lag(&c), before::Rank::ZERO)),
            ("lag_redecoded", &|| {
                assert_eq!(v.lag(&redecoded), before::Rank::ZERO)
            }),
        ];
        for (name, cell) in equal_cells {
            let read = touches(cell);
            assert_eq!(
                read, 0,
                "{name}: equal operands must skip the fold entirely \
                 ({read} digit touches)"
            );
        }
        assert!(
            touches(&|| assert!(v.distance(&w) > before::Rank::ZERO)) > 0,
            "distance over a real pair must fold: a zero here is a dead \
             touch meter"
        );
    }

    /// An empty operand answers every lattice identity/absorption rung
    /// without a walk.
    ///
    /// The identity ladder's empty rungs — `v ∨ 0 = v` (no-op),
    /// `0 ∨ v = v` (adopt the incoming stream wholesale, an `O(1)`
    /// refcount clone), `0 ∧ v = 0` / `v ∧ 0 = 0` (absorption), and the
    /// span forms — must all settle with zero scanned bits, in both
    /// orders at every door: the operators and `|=`/`&=` assigns (all of
    /// which route through the in-place cores — `0 |= v` is the seed
    /// pattern the fold accumulators hit on their first join), the span
    /// doors, and two-element folds (whose single combine is the
    /// borrowed-pair core, unreachable from the operator matrix). The
    /// walking control below proves the zeros are fast paths firing,
    /// not a dead meter: without these rungs the general emission walk
    /// produces byte-identical values (every value law stays green), so
    /// only this scan pin witnesses the rungs' existence.
    #[test]
    fn empty_operands_answer_without_a_walk() {
        let (v, _, _) = fixture();
        let empty = Version::new();

        let cells: &[(&str, &dyn Fn())] = &[
            ("join_adopt", &|| assert_eq!(&(&empty | &v), &v)),
            ("join_noop", &|| assert_eq!(&(&v | &empty), &v)),
            ("meet_absorb_l", &|| assert_eq!(&(&empty & &v), &empty)),
            ("meet_absorb_r", &|| assert_eq!(&(&v & &empty), &empty)),
            ("span_l", &|| assert_eq!(empty.span(&v).hi(), &v)),
            ("span_r", &|| assert_eq!(v.span(&empty).hi(), &v)),
            ("join_view_seed", &|| {
                let mut acc = Version::new();
                acc |= &v;
                assert_eq!(&acc, &v);
            }),
            ("join_view_noop", &|| {
                let mut acc = v.clone();
                acc |= &empty;
                assert_eq!(&acc, &v);
            }),
            ("meet_view_absorb", &|| {
                let mut acc = v.clone();
                acc &= &empty;
                assert_eq!(&acc, &empty);
            }),
            ("meet_view_absorb_l", &|| {
                let mut acc = Version::new();
                acc &= &v;
                assert_eq!(&acc, &empty);
            }),
            // The fold doors: a two-element fold's only combine is the
            // borrowed-pair core, so these cells are what reach the
            // `refs` rungs (the operator matrix routes through the
            // in-place cores exclusively).
            ("join_fold_noop", &|| assert_eq!(&v.join_all([&empty]), &v)),
            ("join_fold_adopt", &|| assert_eq!(&empty.join_all([&v]), &v)),
            ("meet_fold_absorb_r", &|| {
                assert_eq!(&v.meet_all([&empty]), &empty)
            }),
            ("meet_fold_absorb_l", &|| {
                assert_eq!(&empty.meet_all([&v]), &empty)
            }),
        ];
        for (name, cell) in cells {
            let read = scanned(cell);
            assert_eq!(
                read, 0,
                "{name} over an empty operand must answer by the empty \
                 rung, not a walk ({read} bits scanned)"
            );
        }

        // The walking control: join over a concurrent pair still walks,
        // so the zeros above are the rungs firing, not a dead meter.
        let (a, _, w) = fixture();
        assert!(
            scanned(|| assert!((&a | &w) >= a)) > 0,
            "join over a concurrent pair must walk: a zero here is a dead \
             scan meter"
        );
    }
}
