//! Allocation bands for the rank integrator's width buffers.
//!
//! A freeze closes the integrator's segment width, and a deferral closes its
//! deferred width. Both widths are written next at the following region's
//! weight shift, which can sit `Θ(n)` digits above digit 0. Rank's allocation
//! is linear only if each width buffer grows to such a shift once per sweep,
//! not once per freeze or deferral. These bands read the bytes requested from
//! the allocator, because zero-filling a grown buffer writes no digit through
//! the arithmetic, and peak heap holds only the buffer that is live.

use super::*;

/// Blocks of the freeze-parade allocation band's small run (the large run
/// doubles the block count).
const RANK_FREEZE_PARADE_SMALL: usize = 512;

/// One `Version::rank` call's allocation reading.
struct AllocationRun {
    /// The operand's encoded size: the per-byte denominator.
    bytes: u64,
    /// Bytes the call requested from the allocator, whether or not it freed
    /// them before returning.
    allocated: u64,
}

/// The bytes needed to store a rank's numerator, recovered from its public
/// rendering.
///
/// `Display` renders the integer part without leading zeroes, and a fraction
/// that never ends in zero. The binary digits from the first `1` onward, with
/// the point removed, therefore spell the numerator `r · 2^exp` exactly.
fn numerator_bytes(rank: &before::Rank) -> u64 {
    let text = rank.to_string();
    let bits = text
        .trim_start_matches(['0', '.'])
        .bytes()
        .filter(|&c| c != b'.')
        .count();
    (bits as u64).div_ceil(8)
}

/// Measure the bytes one `rank` call requests from the allocator, above a
/// liveness floor of the returned numerator's own storage.
///
/// The floor is the operation's irreducible allocation, not its typical one:
/// the rank it returns owns its numerator, which the call has to allocate.
fn rank_allocation_run(v: &before::Version) -> AllocationRun {
    let bytes = v.encode().len() as u64;
    let (stats, rank) = alloc_meter::measure(|| v.rank());
    let floor = numerator_bytes(&rank);
    assert!(
        floor > 0,
        "the family's rank is nonzero, so the floor is live"
    );
    let run = AllocationRun {
        bytes,
        allocated: stats.bytes_allocated as u64,
    };
    assert!(
        run.allocated >= floor,
        "rank at {bytes} operand bytes requested {} bytes, under the \
         {floor}-byte numerator it returns: the allocation meter is not live",
        run.allocated,
    );
    run
}

/// Assert two allocation readings against their pinned ceilings, printing
/// the measured lines re-pins read from.
fn assert_allocation_ceilings(
    name: &str,
    small: &AllocationRun,
    large: &AllocationRun,
    ceilings: [u64; 2],
) {
    for (run, ceiling, scale) in [(small, ceilings[0], "small"), (large, ceilings[1], "large")] {
        eprintln!(
            "MEASURED {name}_{scale}: bytes={} allocated={}",
            run.bytes, run.allocated,
        );
        assert!(
            run.allocated <= ceiling,
            "{name}_{scale}: {} allocated bytes exceed the pinned ceiling {ceiling}",
            run.allocated,
        );
    }
}

/// Absolute allocation ceilings in bytes at two scales for rank on the
/// freeze parade, measured ×1.25 (the record and every re-pin's movement
/// live in the pin commits).
///
/// The integrator keeps one segment-width buffer across the sweep and grows
/// it to the segment scale once, so the reading stays near four bytes per
/// encoded byte at both scales.
const RANK_FREEZE_PARADE_ALLOCATION_CEILINGS: [u64; 2] = [261_310, 522_430];

/// rank's allocation on the freeze parade is linear: the bytes it requests
/// per encoded byte stay flat (×1.25) across a block doubling, under
/// absolute two-scale ceilings.
///
/// `FZ(k)` fires `k − 1` freezes in its shallow block. The deep spine sets
/// the common scale at about `64k` levels, so each new segment's width lands
/// at a weight shift of nearly `64k` bits, `Θ(k)` digits above digit 0.
/// Allocation stays linear only if growing the segment-width buffer to that
/// shift is paid once per sweep: paid once per freeze, it totals `Θ(k²)`
/// bytes on `Θ(k)` input. The family's closed form is checked in the
/// `family_answers` suite.
#[test]
fn skyline_rank_freeze_parade_allocation_is_flat_per_unit() {
    let small = rank_allocation_run(
        &Shape::FreezeParade
            .build1(RANK_FREEZE_PARADE_SMALL)
            .version(),
    );
    let large = rank_allocation_run(
        &Shape::FreezeParade
            .build1(2 * RANK_FREEZE_PARADE_SMALL)
            .version(),
    );
    assert_flat(
        "rank_freeze_parade_allocated",
        "byte",
        (small.allocated, small.bytes),
        (large.allocated, large.bytes),
    );
    assert_allocation_ceilings(
        "skyline_rank_freeze_parade_allocation",
        &small,
        &large,
        RANK_FREEZE_PARADE_ALLOCATION_CEILINGS,
    );
}

/// Blocks (and suffix digits) of the dense-suffix allocation band's small run
/// (the large run doubles both).
///
/// Large enough that the width buffers are a large share of the reading, so
/// a doubling resolves how the bytes each freeze and deferral allocates
/// scale with the block count.
const RANK_DENSE_SUFFIX_ALLOCATION_SMALL: usize = 1_024;

/// Absolute allocation ceilings in bytes at two scales for rank on the
/// dense-suffix family, measured ×1.25 (the record and every re-pin's
/// movement live in the pin commits).
///
/// The integrator keeps its segment-width and deferred-width buffers across
/// the sweep, so each grows to its highest position once and the reading
/// stays near ten bytes per encoded byte at both scales.
const RANK_DENSE_SUFFIX_ALLOCATION_CEILINGS: [u64; 2] = [2_955_600, 5_905_660];

/// rank's allocation on the dense-suffix family is linear: the bytes it
/// requests per encoded byte stay flat (×1.25) across a block doubling, under
/// absolute two-scale ceilings.
///
/// `DS(p, p)` chains its `p` re-arm blocks one level per node below the gap
/// spine, so the sweep meets block `i` about `4(p − i)` levels above the
/// deepest leaf. Each block fires two freezes and one deferral there, and
/// each opens a width at a weight shift of about `4(p − i)` bits, about
/// `p − i` bytes of eight-byte digits above digit 0. Allocation stays linear
/// only if growing the segment-width and deferred-width buffers to those
/// shifts is paid once per sweep: paid once per freeze or deferral, it totals
/// `Θ(p²)` bytes on `Θ(p)` input.
#[test]
fn skyline_rank_dense_suffix_allocation_is_flat_per_unit() {
    let p = RANK_DENSE_SUFFIX_ALLOCATION_SMALL;
    let small = rank_allocation_run(&Shape::DenseSuffix.build2(p, p).version());
    let large = rank_allocation_run(&Shape::DenseSuffix.build2(2 * p, 2 * p).version());
    assert_flat(
        "rank_dense_suffix_allocated",
        "byte",
        (small.allocated, small.bytes),
        (large.allocated, large.bytes),
    );
    assert_allocation_ceilings(
        "skyline_rank_dense_suffix_allocation",
        &small,
        &large,
        RANK_DENSE_SUFFIX_ALLOCATION_CEILINGS,
    );
}
