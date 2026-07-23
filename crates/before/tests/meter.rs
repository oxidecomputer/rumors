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
//!   operand limbs per `Base` operation plus one accumulator-width record
//!   per wide-gamma decode step. Arithmetic-width cost is invisible to the
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
}

// ─── measurement harness ────────────────────────────────────────────────────

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
        "{name}: peak heap {peak_heap} B exceeds the pinned envelope {} B (input {input_bytes} B)",
        env.peak_heap,
    );
    assert!(
        segments <= env.segments,
        "{name}: {segments} grown stack segments exceed the pinned envelope {}",
        env.segments,
    );
    #[cfg(feature = "limb-meter")]
    assert!(
        limb_ops <= env.limb_ops,
        "{name}: {limb_ops} limb operations exceed the pinned envelope {}",
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
