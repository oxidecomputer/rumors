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
//! Wall time is deliberately never asserted: it is the one number here that
//! is not deterministic. The envelope constants are **measured** on the
//! development target (aarch64-apple-darwin, dev profile); heap byte counts
//! and limb counts are deterministic and portable across 64-bit targets
//! (limb counts shrink under release, where `debug_assert!` comparisons
//! vanish, so the dev-profile pin is the binding one), while segment counts
//! track per-target frame sizes, and the slack absorbs modest variation.

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

// The envelope table: pinned ceiling = measured ×1.25, rounded up. The
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
    pub const DECODE_BIGROOT: Envelope  = envelope( 1_745_332,        0,        25_788); //  1_399_449 -> 1_396_265, 0, 12_520_000 -> 626 -> 20_630 (2026-07-23, limb-wise wide-gamma decode; metered Base equality)
    pub const CMP_BIGROOT: Envelope     = envelope(62_531_270,       15,    47_007_838); // 50_028_232 -> 50_025_016, 12, 50_125_644 -> 37_606_270 (2026-07-23, limb-wise wide-gamma decode + clone-free mixed add)
    pub const JOIN_BIGROOT: Envelope    = envelope(63_075_342,       20,   109_539_093); // 51_515_838 -> 50_460_273, 16, 100_150_646 -> 87_631_272 -> 87_631_274 (2026-07-23, limb-wise wide-gamma decode + push-grow Builder; metered Base equality)
    pub const DECODE_HUGELEAF: Envelope = envelope(    58_604,        0,         2_443); //     55_827 -> 46_883, 0, 122_132_816 -> 1_954 (2026-07-23, limb-wise wide-gamma decode)
    pub const JOIN_HUGELEAF: Envelope   = envelope(   139_714,        0,         9_777); //  3_127_365 -> 111_771, 0, 122_138_683 -> 7_821 (2026-07-23, limb-wise wide-gamma decode + push-grow Builder)
    pub const ID_JOIN: Envelope         = envelope(   156_252,      253,             0); //    125_001, 202,           0
    pub const ID_COVERS: Envelope       = envelope(         0,      107,             0); //          0,  85,           0
    pub const ID_DISJOINT: Envelope     = envelope(         0,      213,             0); //          0, 170,           0
    pub const ID_WITHOUT: Envelope      = envelope(   647_774,        0,             0); //    518_219, 138 -> 0, 0 (2026-07-23, iterative complement)
    pub const DECODE_CLIFF: Envelope    = envelope(   718_402,        0,        51_200); //    574_721,   0,        40_960 (2026-07-23, new scenario)
    pub const CMP_CLIFF: Envelope       = envelope(       820,        0,       238_093); //        656,   0,       190_474 (2026-07-23, new scenario)
    pub const JOIN_CLIFF: Envelope      = envelope( 1_723_362,        0,       480_010); //  1_378_689,   0,       384_008 (2026-07-23, new scenario)
    // Skyline validator rows (2026-07-23, new scenarios): the V5
    // replacement's transient, achieved — the dense row's 49 KB peak over
    // 125k levels is ~3.1 bits per open ancestor (bit stack plus
    // reallocation growth) against DECODE_DENSE's 11 MB parse frames on
    // the same tree, ~56 B per level.
    pub const SKYLINE_VALIDATE_DENSE: Envelope      = envelope(    61_450,        0,       625_003); //     49_160, 0,   500_002
    pub const SKYLINE_VALIDATE_CLIFF: Envelope      = envelope(     1_770,        0,        12_903); //      1_416, 0,    10_322
    pub const SKYLINE_VALIDATE_WIDE_TOOTH: Envelope = envelope(     1_520,        0,        42_325); //      1_216, 0,    33_860
    pub const SKYLINE_VALIDATE_HUGELEAF: Envelope   = envelope(    80_980,        0,         2_443); //     64_784, 0,     1_954
    pub const SKYLINE_VALIDATE_ALT_SPINE: Envelope  = envelope(    61_450,        0,       625_003); //     49_160, 0,   500_002
    // Skyline decoder rows (2026-07-23, new scenarios): validate plus the
    // transcode back to the packed form, whose materialized heights and
    // floors price these against the packed output rather than the skyline
    // input (the module doc's cost section).
    pub const SKYLINE_DECODE_DENSE: Envelope        = envelope(22_672_865,        0,     3_437_515); // 18_138_292, 0, 2_750_012
    pub const SKYLINE_DECODE_CLIFF: Envelope        = envelope( 2_193_750,        0,       397_033); //  1_755_000, 0,   317_626
    pub const SKYLINE_DECODE_WIDE_TOOTH: Envelope   = envelope( 2_083_300,        0,       463_554); //  1_666_640, 0,   370_843
    pub const SKYLINE_DECODE_HUGELEAF: Envelope     = envelope(   117_414,        0,        12_217); //     93_931, 0,     9_773
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

// ─── skyline cliff-immunity flatness ────────────────────────────────────────
//
// The cross-scale witness that the validator's nonnegativity state is
// cliff-immune on the boundary comb: per-delta accumulator digit touches
// and per-input-byte limb work both stay flat (×1.25) across a size
// doubling of `k = n`. A plain big-integer running height roughly doubles
// its per-unit cost per doubling here (the `meter/tier2` plain-sweep pin),
// so this is the row that separates the two representations.
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
    fn comb_run(scale: usize) -> Run {
        let packed = meter::cliff_comb(scale, scale);
        let v = before::Version::decode(&packed.bytes[..]).expect("comb is strict normal form");
        let enc = meter::skyline::encode(&v);
        touch_meter::reset();
        meter::reset_limb_ops();
        meter::skyline::validate(&enc.bytes, enc.bits).expect("the comb stream is canonical");
        Run {
            // 2n + 1 leaves: 2n delta codes follow the first leaf.
            deltas: 2 * scale as u64,
            bytes: enc.bytes.len() as u64,
            touches: touch_meter::touches(),
            limb_ops: meter::limb_ops(),
        }
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
}

// ─── id spine pair scenarios ────────────────────────────────────────────────

/// The combined operand bytes of an id-spine pair scenario.
fn id_pair_input_bytes(a: &meter::Packed, b: &meter::Packed) -> usize {
    a.bytes.len() + b.bytes.len()
}

/// Joining the diverted id-spine pair stays within its envelope (the
/// two-tree walk recurses to full lockstep depth).
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
    drop(a);
}

/// `covers` over the diverted id-spine pair stays within its envelope (the
/// two-tree walk recurses to full lockstep depth).
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
    use num_bigint::BigUint;

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
        acc.add_wide(&((BigUint::from(1u8) << k) - 1u8));
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
        let tooth = BigUint::from(1u8) << w;
        let mut acc = Accum::new();
        acc.add_wide(&(BigUint::from(1u8) << k));
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
        let drop = (BigUint::from(1u8) << k) - 1u8;
        let mut acc = Accum::new();
        acc.add_wide(&(BigUint::from(1u8) << k));
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
        let drop = (BigUint::from(1u8) << k) - 1u8;
        let mut acc = Accum::new();
        acc.add_wide(&(BigUint::from(1u8) << k));
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
