//! Operations whose behavior changes at a 32-bit `usize` boundary.
//!
//! Every large operand is synthesized inside wasm and every check runs in a
//! fresh instance. The host supplies only the exact boundary coordinate.

use wasm32_pins_harness::{run, run_raw, Check, Failure, Outcome, Trap};

/// The first byte count whose live bit length cannot fit a wasm32 `usize`.
///
/// The synthesized stream has `8n - 8` live bits. At this byte count it has
/// exactly `2^32` live bits; the preceding byte has `2^32 - 8`.
const FIRST_WIDE_VERSION_BYTES: u64 = (1 << 29) + 1;

/// A left payload width small enough that both join inputs remain addressable.
const JOIN_LEFT_WIDTH: u64 = 100_000_000;

/// Produces `2^32 - 1` output bits under `2k + 2j + 5`.
const JOIN_RIGHT_BELOW: u64 = 2_047_483_645;

/// Produces `2^32 + 1` output bits, the next possible odd output length.
const JOIN_RIGHT_ABOVE: u64 = 2_047_483_646;

/// Runs a check and reports its parameters if it fails.
fn assert_passes(check: Check, a: u64, b: u64) {
    assert_eq!(
        run(check, a, b),
        Outcome::Passed,
        "{check:?} failed for ({a}, {b})"
    );
}

/// The protocol distinguishes a pass, an in-band failure, and a wasm trap.
#[test]
fn harness_outcomes_are_live() {
    assert_passes(Check::Liveness, 0, 0);
    assert_eq!(
        run_raw(u32::MAX, 0, 0),
        Outcome::Failed(Failure::UnknownCheck)
    );
    assert_eq!(
        run(Check::HarnessTrap, 0, 0),
        Outcome::Trapped(Trap::UnreachableCodeReached)
    );
}

/// `Count` remains wider than `usize`, and both borrowing fork iterators keep
/// exact ownership and sound size hints at the first unrepresentable count.
///
/// A narrowing conversion would accept `2^32` as zero. Narrowing the iterator's
/// remaining count would exhaust it or report `(0, Some(0))` before one child.
#[test]
fn forks_accept_the_first_count_past_usize() {
    assert_passes(Check::Forks, 1u64 << 32, 0);
}

/// Version validation and stored-byte adoption preserve the live length on
/// both sides of wasm32's bit-position limit.
///
/// Computing the padded buffer's `bytes.len() * 8` in `usize` already wraps at
/// the lower witness and remains short by `2^32` at the upper witness, causing
/// rejection, a trap, or an incorrect stored length.
#[test]
fn version_decode_crosses_the_usize_position_boundary() {
    for size in [FIRST_WIDE_VERSION_BYTES - 1, FIRST_WIDE_VERSION_BYTES] {
        assert_passes(Check::VersionDecode, size, 0);
    }
}

/// Causal comparison can traverse a stored payload whose final bit position
/// does not fit `usize`.
///
/// Narrowing a cursor position before dividing it into a byte index would wrap
/// the second input and misorder it against the smaller version.
#[test]
fn version_compare_crosses_the_usize_position_boundary() {
    for size in [FIRST_WIDE_VERSION_BYTES - 1, FIRST_WIDE_VERSION_BYTES] {
        assert_passes(Check::VersionCompare, size, 0);
    }
}

/// Join constructs the closest representable output lengths below and above
/// `2^32` bits while each input remains within wasm32's address space.
///
/// The joined stream has `2k + 2j + 5` bits, hence the two asserted constants.
/// A `usize` output length would wrap while appending or finalizing the second
/// result.
#[test]
fn version_join_output_crosses_the_usize_position_boundary() {
    assert_eq!(
        2 * JOIN_LEFT_WIDTH + 2 * JOIN_RIGHT_BELOW + 5,
        u64::from(u32::MAX)
    );
    assert_eq!(
        2 * JOIN_LEFT_WIDTH + 2 * JOIN_RIGHT_ABOVE + 5,
        u64::from(u32::MAX) + 2
    );
    for right in [JOIN_RIGHT_BELOW, JOIN_RIGHT_ABOVE] {
        assert_passes(Check::VersionJoinEmitted, JOIN_LEFT_WIDTH, right);
    }
}

/// Rank decoding retains a fractional bit at exponent `2^32`, the first
/// exponent a wasm32 `usize` cannot represent.
///
/// Narrowing the exponent or a decoded bit position would drop or relocate the
/// final bit; the guest proves it with an exact adjacent-value identity.
#[test]
fn rank_decode_accepts_the_first_exponent_past_usize() {
    assert_passes(Check::RankDecode, 1u64 << 32, 0);
}

/// Rank addition and checked subtraction are exact when alignment shifts by
/// `usize::MAX` bits and by `usize::MAX + 1` bits.
///
/// The former exercises the contiguous-big-integer path at its limit; the
/// latter must use the shifted accumulator path. Wrapping or using the wrong
/// route breaks the guest's exact inverse identities.
#[test]
fn rank_arithmetic_straddles_the_usize_alignment_limit() {
    for case in 1..=4 {
        assert_passes(Check::RankArithmetic, case, 0);
    }
}

/// Every public shifted-accumulator route rejects a nonzero digit whose
/// required buffer length cannot fit wasm32's `usize`.
///
/// The cases cover a limb stream after leading zero limbs, a shifted stored
/// accumulator, a contribution at the last index, and a limb stream longer
/// than `usize::MAX` limbs, whose landing position must not come from a
/// wrapped limb counter. Wrapping any landing would return normally after
/// writing near the start of the buffer.
#[test]
fn suanpan_rejects_unaddressable_digit_landings() {
    for case in 1..=4 {
        assert_eq!(
            run(Check::SuanpanLanding, case, 0),
            Outcome::Trapped(Trap::UnreachableCodeReached),
            "landing case {case} returned instead of panicking"
        );
    }
}
