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

/// The first stability width whose top adjustment digit index cannot fit a
/// wasm32 `usize`.
///
/// A width of `w` bits spans `ceil(w / 32)` digits, so its top index is
/// `ceil(w / 32) - 1`. At `32 * 2^32` bits that index is `2^32 - 1`, the
/// largest `usize`; one more bit makes it `2^32`.
const FIRST_UNINDEXABLE_STABILITY_WIDTH: u64 = 32 * (1 << 32) + 1;

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

/// Rank addition and checked subtraction are exact when alignment shifts a
/// numerator by `usize::MAX` bits and by `usize::MAX + 1` bits.
///
/// The alignment gap is a `u64` bit count. Narrowing it to `usize` would keep
/// the first shift and wrap the second to zero, which either breaks the guest's
/// exact inverse identities or traps on a negative difference.
#[test]
fn rank_arithmetic_crosses_the_usize_gap_boundary() {
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

/// `reserve_bits` ignores requests that wasm32 cannot honor, in both
/// accumulator representations.
///
/// `u64::MAX` bits needs more 32-bit positions than `usize` can count, and
/// `2^33 - 31` bits is the smallest request whose `i64` digit storage exceeds
/// `isize::MAX` bytes; both fail the capacity computation. `2^33 - 32` bits is
/// the largest valid layout, `2^31 - 8` bytes. The scalar accumulator's
/// request of that size is granted and kept, so the digit accumulator's cannot
/// fit beside it in the 4 GiB linear memory, and only the allocator can refuse
/// it. A panicking narrowing of the position count, or an infallible
/// reservation, traps instead of returning.
#[test]
fn suanpan_ignores_unsatisfiable_reservations() {
    for bits in [u64::MAX, (1 << 33) - 31, (1 << 33) - 32] {
        assert_passes(Check::SuanpanReserve, bits, 0);
    }
}

/// Shifting zero onto digit position `2^32 - 1` returns zero whatever digits
/// store the zero.
///
/// A known zero deposits nothing. A zero stored as the cancelling digits
/// `[-2^32, 1]` must not deposit either: its digits would land at index
/// `usize::MAX`, which the accumulator rejects with a panic, trapping the guest.
#[test]
fn suanpan_shifts_zero_onto_an_unaddressable_digit_position() {
    for case in 0..=1 {
        assert_eq!(
            run(Check::SuanpanZeroShift, case, 0),
            Outcome::Passed,
            "zero-shift case {case} did not return zero"
        );
    }
}

/// A stability query compacts and answers identically on every pointer width,
/// however wide the adjustment it is asked about.
///
/// The guest stores the value `5 * 2^64` at digit 2, below a two-digit
/// cancelling top at digits 9 and 10, eleven digits in all. At the last width
/// whose top digit index fits a 32-bit `usize`, at the first that does not, and
/// at `u64::MAX`, the documented scan must compact the value to three digits,
/// deciding its sign at digit 2, and must answer `None`, because a value below
/// `2^67` cannot dominate such a width.
///
/// Each half rejects a wrong implementation on wasm32. One that returned early
/// when the top index did not fit a `usize` would leave all eleven digits
/// stored. One that truncated the index to a `usize` would turn position
/// `2^32` into 0 at the first unindexable width, so its threshold of 2 would
/// let the decision at digit 2 claim stability.
#[test]
fn suanpan_stability_query_compacts_and_declines_past_the_usize_digit_index() {
    for width in [
        FIRST_UNINDEXABLE_STABILITY_WIDTH - 1,
        FIRST_UNINDEXABLE_STABILITY_WIDTH,
        u64::MAX,
    ] {
        assert_passes(Check::SuanpanStabilityWidth, width, 0);
    }
}
