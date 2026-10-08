
/// Scratch (uncommitted): builds a pseudo-random accumulator history from `seed`.
///
/// Shifts stay below 24 digits; one update in three is a cancelling pair (one
/// unit at digit `k + 1` against `-2^32` at digit `k`), so uncompacted tops are
/// common.
fn scratch_history(seed: u64) -> Accumulator {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut accumulator = Accumulator::new();
    let updates = 1 + next() % 12;
    for _ in 0..updates {
        let digit = next() % 24;
        let offset = if next() % 4 == 0 { next() % 32 } else { 0 };
        let shift = digit * 32 + offset;
        let low = next() >> (next() % 64);
        let high = if next() % 2 == 0 { 0 } else { next() >> (next() % 64) };
        match next() % 3 {
            0 => accumulator.add_shifted_limbs(shift, [low, high]),
            1 => accumulator.sub_shifted_limbs(shift, [low, high]),
            _ => {
                accumulator.add_shifted_limbs(digit * 32 + 32, [1, 0]);
                accumulator.sub_shifted_limbs(digit * 32, [1 << 32]);
            }
        }
    }
    accumulator
}

/// Scratch (uncommitted): a stability query at width `bits` leaves the stored
/// form `cmp_zero` leaves, and any decided answer agrees with `cmp_zero`.
fn scratch_stability_family(seed: u64, bits: u64) -> Result<(), Failure> {
    let mut queried = scratch_history(seed);
    let mut reference = queried.clone();
    let sign = reference.cmp_zero();
    if let Some(answer) = queried.cmp_zero_stable_under(bits) {
        if answer != sign {
            return Err(Failure::WrongValue);
        }
    }
    if queried.stored_digit_count() != reference.stored_digit_count() {
        return Err(Failure::WrongLength);
    }
    let (queried_sign, queried_magnitude) = queried.signed_magnitude();
    let (reference_sign, reference_magnitude) = reference.signed_magnitude();
    if queried_sign != reference_sign || queried_magnitude.as_ref() != reference_magnitude.as_ref() {
        return Err(Failure::WrongBytes);
    }
    Ok(())
}

/// Scratch (uncommitted): passes when `seed`'s history has something to compact.
fn scratch_stability_coverage(seed: u64) -> Result<(), Failure> {
    let original = scratch_history(seed);
    let mut compacted = original.clone();
    compacted.cmp_zero();
    if compacted.stored_digit_count() < original.stored_digit_count() {
        Ok(())
    } else {
        Err(Failure::Synthesis)
    }
}
