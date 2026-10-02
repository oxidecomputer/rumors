//! The semantic checks run by the wasm guest.
//!
//! Each function performs the smallest operation that exposes one 32-bit-only
//! seam. Construction stays in [`crate::synthesis`]; this module begins only
//! once canonical operands exist, making a returned synthesis failure distinct
//! from a panic in the operation under test.

use core::cmp::Ordering;

use before::{Clock, Count, Party, Rank, Version};
use suanpan::Accumulator;
use wasm32_pins_protocol::{Check, Failure};

use crate::synthesis;

/// Dispatches one check and gives every check the same typed failure channel.
pub fn run(check: Check, a: u64, b: u64) -> Result<(), Failure> {
    match check {
        Check::Liveness => liveness(),
        Check::HarnessTrap => panic!("deliberate harness trap"),
        Check::Forks => forks(a),
        Check::VersionDecode => version_decode(a),
        Check::RankDecode => rank_decode(a),
        Check::VersionCompare => version_compare(a),
        Check::VersionJoinEmitted => version_join_emitted(a, b),
        Check::RankArithmetic => rank_arithmetic(a),
        Check::SuanpanLanding => suanpan_landing(a),
    }
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

/// Checks that a rank retains the bit at the 32-bit exponent seam.
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

/// Checks one arithmetic path on either side of wasm32's alignment limit.
fn rank_arithmetic(case: u64) -> Result<(), Failure> {
    let deep =
        Rank::decode(&synthesis::rank(1u64 << 32)?[..]).map_err(|_| Failure::DecodeRejected)?;

    // Half has exponent 1, so aligning it to `deep` shifts by `usize::MAX`:
    // the largest distance accepted by the contiguous big-integer path.
    // One has exponent 0, so its 2^32-bit distance must instead use the
    // accumulator path without narrowing the shift.
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
        _ => return Err(Failure::InvalidArguments),
    }

    // Correct code panics before reaching this observation. Wrapping the
    // landing position returns a small digit count, so a normal return is red.
    Err(Failure::WrongValue)
}
