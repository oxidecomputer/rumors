//! Heap and allocation ceilings for `min_ticks`' constant factors.
//!
//! The board judges `min_ticks` against the heap ceiling every operation
//! shares, which admits a several-fold regression in the bytes each suspended
//! minimum costs. These checks pin `min_ticks`' own readings on the shapes
//! that exercise each of its representation choices: the record slack, the
//! working width up to which a buried reference's chunk is copied, and the
//! reused buffers that keep allocation counts independent of depth.

use super::*;
use before::testing::meter::Encoding;

/// Depth of the jump-entered rising spines' small runs (the large runs double
/// it, doubling the operand's size).
const RISING_SPINE_SMALL_DEPTH: usize = 8_000;

/// Entry width of the rising spine whose every level stays within the record
/// slack once re-anchored: the board's `jump-rising-spine` sample.
const RISING_SPINE_ENTRY_BITS: usize = 40;

/// Entry width of the rising spine whose outermost suspended minimum keeps an
/// offset wider than `min_ticks` packs inline, so that it is stored on its own
/// beneath every narrower record pushed after it.
const WIDE_ENTRY_BITS: usize = 320;

/// Absolute peak-heap ceilings in bytes at two depths for `min_ticks` on
/// `JR(40, d)`, measured ×1.25 (the record and every re-pin's movement live
/// in the pin commits).
///
/// Every level after the entry jump is one tick above the last, so a record
/// trigger keeps each suspended minimum's offset a few bits wide. Widening the
/// slack, disabling the trigger, or moving every buried chunk whole multiplies
/// the bytes each level holds and exceeds these ceilings.
const RISING_SPINE_HEAP_CEILINGS: [usize; 2] = [29_160, 58_600];

/// Depth of the one-tick wide-step pair spines' small runs (the large runs
/// double it).
const WIDE_STEP_PAIRS_SMALL_DEPTH: usize = 2_048;

/// Step width of the wide-step pairs that exercise the copied chunk regime:
/// each frozen height is `2^65 + 1`, past the 64 bits one limb holds and within
/// the working width up to which `min_ticks` copies a buried chunk.
const COPIED_STEP_BITS: usize = 65;

/// Absolute peak-heap ceilings in bytes at two depths for `min_ticks` on
/// `WP(65, d)`, measured ×1.25 (the record and every re-pin's movement live in
/// the pin commits).
///
/// Every second level suspends a minimum on two two-limb values: the frozen
/// height `2^65 + 1`, whose chunk `min_ticks` buries, and the range boundary
/// `2^65` beneath the level's minimum. Both are at most 256 bits wide, so
/// `min_ticks` copies the chunk into its packed stack and the range minima
/// copy the boundary once a deeper level covers it: each wide level costs
/// packed limbs, in proportion to the values' width.
const WIDE_STEP_PAIRS_HEAP_CEILINGS: [usize; 2] = [65_120, 130_080];

/// The closed-form `min_ticks` of the jump-entered rising spine `JR(b, d)`.
///
/// Every internal node's minimum is its left leaf, so the identity's sum of
/// leaves minus internal minima leaves only the last leaf, `2^b + d`.
fn rising_spine_ticks(b: usize, d: usize) -> BigUint {
    (BigUint::ONE << b) + BigUint::from(d)
}

/// The closed-form `min_ticks` of the wide-step pair spine `WP(e, d)`.
///
/// Every internal node's minimum is its left leaf, so only the last leaf
/// remains: `⌊d/2⌋` wide rises of `2^e` and `d − ⌊d/2⌋` one-tick rises.
fn wide_step_pairs_ticks(e: usize, d: usize) -> BigUint {
    let wide = d / 2;
    (BigUint::ONE << e) * BigUint::from(wide) + BigUint::from(d - wide)
}

/// One `Version::min_ticks` run with its heap and allocation readings.
struct HeapRun {
    bytes: usize,
    peak: usize,
    allocations: usize,
}

/// Run `min_ticks` on a generated shape under the allocation meter, checking
/// the closed form and a liveness floor on the peak.
///
/// The floor is one bit per level the spine suspends: every level's minimum
/// is held at once at the spine's bottom, so a fold that holds none of them in
/// the heap is not being metered.
fn min_ticks_heap_run(encoded: Encoding, expected: &BigUint, levels: usize) -> HeapRun {
    let version = encoded.version();
    let bytes = version.encode().len();
    let (stats, ticks) = alloc_meter::measure(|| version.min_ticks());
    assert_eq!(
        ticks,
        min_ticks_from_big(expected),
        "min_ticks disagrees with the shape's closed form"
    );
    assert!(
        stats.peak_bytes >= levels / 8,
        "min_ticks at {bytes} operand bytes peaked at {} heap bytes, under the \
         one-bit-per-suspended-level floor of {}: the allocation meter is not live",
        stats.peak_bytes,
        levels / 8,
    );
    HeapRun {
        bytes,
        peak: stats.peak_bytes,
        allocations: stats.allocations,
    }
}

/// Assert two peak-heap readings against their pinned ceilings, printing the
/// measured lines re-pins read from.
fn assert_heap_ceilings(name: &str, runs: [&HeapRun; 2], ceilings: [usize; 2]) {
    for ((run, ceiling), scale) in runs.into_iter().zip(ceilings).zip(["small", "large"]) {
        eprintln!(
            "MEASURED {name}_{scale}: bytes={} peak={} allocations={}",
            run.bytes, run.peak, run.allocations,
        );
        assert!(
            run.peak <= ceiling,
            "{name}_{scale}: a peak of {} heap bytes exceeds the pinned ceiling {ceiling}",
            run.peak,
        );
    }
}

/// `min_ticks`' peak heap on `JR(40, d)` stays within its measured ceilings at
/// two depths.
///
/// The spine suspends one minimum per level, each one tick above the last
/// after a 40-bit entry jump; this pins the bytes each suspended level costs.
#[test]
fn min_ticks_peak_heap_on_the_rising_spine_stays_within_measured_ceilings() {
    let runs = [RISING_SPINE_SMALL_DEPTH, 2 * RISING_SPINE_SMALL_DEPTH].map(|d| {
        min_ticks_heap_run(
            Shape::JumpRisingSpine.build2(RISING_SPINE_ENTRY_BITS, d),
            &rising_spine_ticks(RISING_SPINE_ENTRY_BITS, d),
            d,
        )
    });
    assert_heap_ceilings(
        "min_ticks_rising_spine_heap",
        [&runs[0], &runs[1]],
        RISING_SPINE_HEAP_CEILINGS,
    );
}

/// `min_ticks`' peak heap on `WP(65, d)` stays within its measured ceilings at
/// two depths.
///
/// Every second level freezes a `2^65 + 1` height that a suspended minimum
/// still uses, above a `2^65` range boundary, so this pins the bytes a buried
/// two-limb chunk and a covered two-limb boundary cost.
#[test]
fn min_ticks_peak_heap_on_copied_wide_step_pairs_stays_within_measured_ceilings() {
    let runs = [WIDE_STEP_PAIRS_SMALL_DEPTH, 2 * WIDE_STEP_PAIRS_SMALL_DEPTH].map(|d| {
        min_ticks_heap_run(
            Shape::WideStepPairs.build2(COPIED_STEP_BITS, d),
            &wide_step_pairs_ticks(COPIED_STEP_BITS, d),
            d,
        )
    });
    assert_heap_ceilings(
        "min_ticks_copied_wide_step_pairs_heap",
        [&runs[0], &runs[1]],
        WIDE_STEP_PAIRS_HEAP_CEILINGS,
    );
}

/// `min_ticks`' allocation count on the rising spine does not grow with its
/// depth, whether the outermost suspended offset is packed inline or stored on
/// its own.
///
/// The fold's stacks grow by reallocation, and a settlement multiplies into
/// the answer in place, so a doubled depth adds no allocation. With a 320-bit
/// entry, every narrow record is pushed directly above the separately stored
/// wide one, so a push that copies the entry below it into a big integer
/// allocates once per level and fails here.
#[test]
fn min_ticks_allocation_count_does_not_grow_with_spine_depth() {
    for entry_bits in [RISING_SPINE_ENTRY_BITS, WIDE_ENTRY_BITS] {
        let [small, large] = [RISING_SPINE_SMALL_DEPTH, 2 * RISING_SPINE_SMALL_DEPTH].map(|d| {
            min_ticks_heap_run(
                Shape::JumpRisingSpine.build2(entry_bits, d),
                &rising_spine_ticks(entry_bits, d),
                d,
            )
        });
        eprintln!(
            "MEASURED min_ticks_rising_spine_allocations_b{entry_bits}: small={} large={}",
            small.allocations, large.allocations,
        );
        assert!(
            small.allocations >= 1,
            "JR({entry_bits}, d): the fold's stacks hold every level, so they \
             allocate at least once; a zero count means the meter is not live"
        );
        assert!(
            large.allocations <= small.allocations,
            "JR({entry_bits}, d): min_ticks made {} allocations at depth {} and {} \
             at depth {}: some per-level step allocates",
            small.allocations,
            RISING_SPINE_SMALL_DEPTH,
            large.allocations,
            2 * RISING_SPINE_SMALL_DEPTH,
        );
    }
}
