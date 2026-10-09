//! The semantic checks run by the wasm guest.
//!
//! Each function performs the smallest operation that exposes one 32-bit-only
//! boundary. Construction stays in [`crate::synthesis`]; this module begins
//! only once canonical operands exist, making a returned synthesis failure
//! distinct from a panic in the operation under test.

use core::cmp::Ordering;
use core::hint::black_box;
use core::iter;

use before::{Clock, Count, Party, Rank, Version};
use suanpan::Accumulator;
use wasm32_pins_protocol::{Check, Failure};

use crate::synthesis;

/// Dispatches one check and gives every check the same typed failure channel.
pub fn run(check: Check, a: u64, b: u64) -> Result<(), Failure> {
    match check {
        Check::Liveness => liveness(),
        Check::HarnessPanic => panic!("deliberate harness panic"),
        Check::Forks => forks(a),
        Check::VersionDecode => version_decode(a),
        Check::RankDecode => rank_decode(a),
        Check::VersionCompare => version_compare(a),
        Check::VersionJoinEmitted => version_join_emitted(a, b),
        Check::RankArithmetic => rank_arithmetic(a),
        Check::SuanpanLanding => suanpan_landing(a),
        Check::SuanpanReserve => suanpan_reserve(a),
        Check::SuanpanZeroShift => suanpan_zero_shift(a),
        Check::SuanpanStabilityWidth => suanpan_stability_width(a),
        Check::HarnessAllocationFailure => exhaust_memory(),
    }
}

/// The size of each block [`exhaust_memory`] reserves: 1 GiB.
const EXHAUSTING_BLOCK_BYTES: usize = 1 << 30;

/// The number of blocks [`exhaust_memory`] reserves.
///
/// Together they span exactly wasm32's 4 GiB address space, before counting
/// the allocator's headers and the guest's own data, so they cannot all fit.
const EXHAUSTING_BLOCKS: usize = 4;

/// Aborts on allocation failure without panicking.
///
/// Each block is a valid `Vec` capacity, so no capacity check panics; the
/// allocator itself runs out and the guest aborts.
fn exhaust_memory() -> Result<(), Failure> {
    let blocks: Vec<Vec<u8>> = (0..EXHAUSTING_BLOCKS)
        .map(|_| Vec::with_capacity(EXHAUSTING_BLOCK_BYTES))
        .collect();
    // Observing the blocks keeps the optimizer from deleting the reservations.
    black_box(&blocks);

    // The address space cannot hold every block, so a normal return fails
    // the pin.
    Err(Failure::WrongValue)
}

/// Creates a flat version with the given event count.
fn uniform(count: impl Into<Count>) -> Version {
    let mut version = Version::new();
    Party::seed().ticks(&mut version, count);
    version
}

/// Creates rank one half without relying on text or wire parsing.
fn half() -> Rank {
    let mut keeper = Party::seed();
    let child = keeper.fork();
    let mut version = Version::new();
    child.tick(&mut version);
    version.rank()
}

/// Decodes a synthesized version, preserving the distinction from synthesis.
fn decoded_version(bytes: &[u8]) -> Result<Version, Failure> {
    Version::decode(bytes).map_err(|_| Failure::DecodeRejected)
}

/// Proves that the guest, decoder, and typed reject path are alive.
fn liveness() -> Result<(), Failure> {
    let bytes = synthesis::version(64)?;
    let version = decoded_version(&bytes)?;
    if version.encoded_bits() != 8 * 64 - 8 {
        return Err(Failure::WrongLength);
    }
    if version.as_bytes() != bytes {
        return Err(Failure::WrongBytes);
    }
    if Version::decode(&bytes[..63]).is_ok() {
        return Err(Failure::DecodeAccepted);
    }
    let mut unmarked = bytes;
    unmarked[63] = 0;
    if Version::decode(&unmarked[..]).is_ok() {
        return Err(Failure::DecodeAccepted);
    }
    Ok(())
}

/// Checks iterator bounds and drop recovery beyond a 32-bit `usize`.
fn forks(count: u64) -> Result<(), Failure> {
    if count <= u64::from(u32::MAX) {
        return Err(Failure::InvalidArguments);
    }
    let expected = count;
    let count = Count::from(count);
    if usize::try_from(&count).is_ok() || u64::try_from(&count).ok() != Some(expected) {
        return Err(Failure::WrongValue);
    }

    let mut party = Party::seed();
    let share = {
        let mut forks = party.forks(count.clone());
        if forks.size_hint() != (usize::MAX, None) {
            return Err(Failure::WrongValue);
        }
        let share = forks.next().ok_or(Failure::Exhausted)?;
        if forks.size_hint() != (usize::MAX, Some(usize::MAX)) {
            return Err(Failure::WrongValue);
        }
        share
    };
    if !party.is_disjoint(&share) || party.join(share).is_err() || !party.is_seed() {
        return Err(Failure::WrongValue);
    }

    let mut clock = Clock::seed();
    let child = {
        let mut forks = clock.forks(count);
        if forks.size_hint() != (usize::MAX, None) {
            return Err(Failure::WrongValue);
        }
        let child = forks.next().ok_or(Failure::Exhausted)?;
        if forks.size_hint() != (usize::MAX, Some(usize::MAX)) {
            return Err(Failure::WrongValue);
        }
        child
    };
    if !clock.party().is_disjoint(child.party())
        || clock.join(child).is_err()
        || !clock.party().is_seed()
    {
        return Err(Failure::WrongValue);
    }
    Ok(())
}

/// Checks exact decode and storage at the supplied byte length.
fn version_decode(size: u64) -> Result<(), Failure> {
    let bytes = synthesis::version(size)?;
    let version = decoded_version(&bytes)?;
    if version.as_bytes() != bytes {
        return Err(Failure::WrongBytes);
    }
    let expected = size
        .checked_mul(8)
        .and_then(|bits| bits.checked_sub(8))
        .ok_or(Failure::Synthesis)?;
    if version.encoded_bits() != expected {
        return Err(Failure::WrongLength);
    }
    Ok(())
}

/// Checks that a rank retains the bit at the 32-bit exponent boundary.
fn rank_decode(exp: u64) -> Result<(), Failure> {
    let bytes = synthesis::rank(exp)?;
    let rank = Rank::decode(&bytes[..]).map_err(|_| Failure::DecodeRejected)?;
    drop(bytes);
    let adjacent = Rank::decode(&synthesis::rank_with_penultimate(exp)?[..])
        .map_err(|_| Failure::DecodeRejected)?;
    let unit =
        Rank::decode(&synthesis::unit_fraction(exp)?[..]).map_err(|_| Failure::DecodeRejected)?;
    if &rank + &unit != adjacent {
        return Err(Failure::WrongValue);
    }
    Ok(())
}

/// Checks a comparison that walks a large stored stream without decoding it.
fn version_compare(size: u64) -> Result<(), Failure> {
    let large = decoded_version(&synthesis::version(size)?)?;
    let small = decoded_version(&synthesis::version(18)?)?;
    if large.partial_cmp(&small) != Some(Ordering::Greater)
        || small.partial_cmp(&large) != Some(Ordering::Less)
    {
        return Err(Failure::WrongValue);
    }
    Ok(())
}

/// Checks a join whose output crosses a boundary independently of its inputs.
fn version_join_emitted(k: u64, j: u64) -> Result<(), Failure> {
    let left = decoded_version(&synthesis::two_leaf_left(k)?)?;
    let right = decoded_version(&synthesis::two_leaf_right(k, j)?)?;
    let joined = left.join(&right);
    let expected = k
        .checked_mul(2)
        .and_then(|n| j.checked_mul(2).and_then(|m| n.checked_add(m)))
        .and_then(|n| n.checked_add(5))
        .ok_or(Failure::Synthesis)?;
    if joined.encoded_bits() != expected {
        return Err(Failure::WrongLength);
    }
    if joined.as_bytes() != synthesis::joined(k, j)? {
        return Err(Failure::WrongBytes);
    }
    Ok(())
}

/// Checks exact rank arithmetic at one of two alignment gaps, on either side
/// of the largest gap a wasm32 `usize` can hold.
fn rank_arithmetic(case: u64) -> Result<(), Failure> {
    let deep =
        Rank::decode(&synthesis::rank(1u64 << 32)?[..]).map_err(|_| Failure::DecodeRejected)?;

    // Half has exponent 1, so aligning it to `deep` shifts by `usize::MAX`
    // bits. One has exponent 0, so its shift is `2^32` bits, the first gap
    // that narrowing to a wasm32 `usize` would wrap.
    let (small, add) = match case {
        1 => (half(), true),
        2 => (uniform(1u8).rank(), true),
        3 => (half(), false),
        4 => (uniform(1u8).rank(), false),
        _ => return Err(Failure::InvalidArguments),
    };
    if add {
        let sum = &deep + &small;
        if sum.checked_sub(&small).as_ref() != Some(&deep) {
            return Err(Failure::WrongValue);
        }
    } else {
        let difference = small.checked_sub(&deep).ok_or(Failure::WrongValue)?;
        if &difference + &deep != small {
            return Err(Failure::WrongValue);
        }
    }
    Ok(())
}

/// Reaches each checked digit-landing panic that is specific to 32-bit indices.
fn suanpan_landing(case: u64) -> Result<(), Failure> {
    let max = u64::from(u32::MAX);
    let mut accumulator = Accumulator::new();
    match case {
        1 => accumulator.add_shifted_limbs(32 * (max - 3), [0, 0, 5]),
        2 => {
            accumulator += 1_u64;
            accumulator <<= 64;
            accumulator <<= 32 * (max - 1);
        }
        3 => accumulator.add_shifted_limbs(32 * max, [1]),
        // Zero limbs fill indices 0 through 2^32 - 1, and the 1 sits at index
        // 2^32, one past the largest index a 32-bit `usize` counter can hold.
        // The stream is lazy, so the guest allocates nothing for it.
        4 => accumulator.add_shifted_limbs(0, iter::repeat_n(0, usize::MAX).chain([0, 1])),
        _ => return Err(Failure::InvalidArguments),
    }

    // Correct code panics before reaching this observation. A wrapped landing
    // position falls near the start of the buffer and the call returns
    // normally, so any return is a failure.
    Err(Failure::WrongValue)
}

/// Ignores unsatisfiable reservations in both accumulator representations.
///
/// The scalar accumulator keeps its storage while the digit accumulator
/// reserves, so a request for `2^31 - 8` bytes can be granted at most once in
/// wasm32's 4 GiB linear memory. Correct code returns from every
/// `reserve_bits` call and leaves each value exact, so undoing the setup
/// update reads zero.
fn suanpan_reserve(bits: u64) -> Result<(), Failure> {
    let mut scalar = Accumulator::new();
    scalar += 7_u64;
    scalar.reserve_bits(bits);
    scalar -= 7_u64;

    let mut digits = Accumulator::new();
    digits.add_shifted_limbs(3_200, [1]);
    digits.reserve_bits(bits);
    digits.sub_shifted_limbs(3_200, [1]);

    for mut accumulator in [scalar, digits] {
        if accumulator.cmp_zero() != Ordering::Equal {
            return Err(Failure::WrongValue);
        }
    }
    Ok(())
}

/// Shifts zero onto digit position `2^32 - 1`, which no 32-bit buffer can address.
///
/// Case 0 holds a known zero. Case 1 stores zero as the cancelling digits
/// `-2^32` at position 0 and `1` at position 1. The result is zero either
/// way, so neither case may trap; depositing case 1's digits at the shifted
/// position would panic on the unaddressable landing.
fn suanpan_zero_shift(case: u64) -> Result<(), Failure> {
    let mut accumulator = Accumulator::new();
    match case {
        0 => {}
        1 => {
            // Two limbs select the digit representation, so the cancelling
            // digits stay stored instead of combining in the small value.
            accumulator.add_shifted_limbs(32, [1, 0]);
            accumulator.sub_shifted_limbs(0, [1 << 32]);
            if accumulator.is_known_zero() {
                return Err(Failure::Synthesis);
            }
        }
        _ => return Err(Failure::InvalidArguments),
    }
    accumulator <<= 32 * u64::from(u32::MAX);
    if accumulator.cmp_zero() == Ordering::Equal {
        Ok(())
    } else {
        Err(Failure::WrongValue)
    }
}

/// Checks that a stability query at adjustment width `bits` compacts the stored
/// form and declines to decide.
///
/// The documented scan compacts what it reads whatever answer it gives, so the
/// stored form afterward must not depend on whether the adjustment's top digit
/// position fits a 32-bit `usize`. The scan decides at digit 2, so the answer
/// also depends on comparing that position with the full threshold: a threshold
/// narrowed to a 32-bit `usize` could fall to 2 or below and claim stability.
fn suanpan_stability_width(bits: u64) -> Result<(), Failure> {
    // The value 5 * 2^64 at digit 2, below a cancelling top: digit 10 holds 1
    // and digit 9 holds -2^32. The one-limb subtraction deposits -2^32 at digit
    // 9 without carrying into digit 10, so eleven digits stay stored.
    let mut accumulator = Accumulator::new();
    accumulator.add_shifted_limbs(64, [5]);
    accumulator.add_shifted_limbs(32 * 10, [1, 0]);
    accumulator.sub_shifted_limbs(32 * 9, [1 << 32]);
    // Without the cancelling top there is nothing to compact, and the length
    // check below would pass vacuously.
    if accumulator.stored_digit_count() != 11 {
        return Err(Failure::Synthesis);
    }

    // A value below 2^67 cannot dominate an adjustment billions of digits wide,
    // so the correct answer is `None` on every target.
    if accumulator.cmp_zero_stable_under(bits).is_some() {
        return Err(Failure::WrongValue);
    }
    // The scan folds digits 10 and 9 into a zero partial, descends through the
    // zero digits to digit 2, and decides there, leaving three stored digits.
    if accumulator.stored_digit_count() != 3 {
        return Err(Failure::WrongLength);
    }
    if !matches!(i128::try_from(accumulator), Ok(value) if value == 5 << 64) {
        return Err(Failure::WrongValue);
    }
    Ok(())
}
