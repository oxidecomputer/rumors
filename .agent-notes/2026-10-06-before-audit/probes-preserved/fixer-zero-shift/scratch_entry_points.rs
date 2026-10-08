//! Scratch (uncommitted): every operand entry point keeps a cancelled zero from growing a fresh receiver.

use super::{assert_value, Accumulator};
use num_bigint::BigInt as IBig;

/// Zero stored as -2^32 at digit k - 1 and 1 at digit k.
fn wide_zero(k: u64) -> Accumulator {
    let mut zero = Accumulator::new();
    zero.add_shifted_limbs(32 * k, [1, 0]);
    zero.sub_shifted_limbs(32 * (k - 1), [1 << 32]);
    assert!(!zero.is_known_zero());
    zero
}

/// Report retained digit positions and the value check for one result.
fn check(name: &str, acc: Accumulator) {
    let retained = acc.digits.retained_len();
    let stored = acc.stored_digit_count();
    assert_value(&acc, &IBig::ZERO);
    println!("SCRATCH {name}: stored {stored}, retained {retained}");
    assert!(retained <= 2, "{name}: retained {retained}");
    assert!(stored <= 2, "{name}: stored {stored}");
}

#[test]
fn scratch_entry_points_skip_cancelled_zeros() {
    const K: u64 = 1 << 16;
    let zero = wide_zero(K);

    let mut acc = Accumulator::new();
    acc += &zero;
    check("borrowed +=", acc);

    let mut acc = Accumulator::new();
    acc -= &zero;
    check("borrowed -=", acc);

    let mut acc = Accumulator::new();
    acc -= zero.clone();
    check("owned -=", acc);

    check("owned -", Accumulator::new() - zero.clone());
    check("borrowed +", Accumulator::new() + &zero);
    check("borrowed -", Accumulator::new() - &zero);

    check("Sum<&>", [&zero, &zero, &zero].into_iter().sum());

    // Owned addition returns whichever operand is not a known zero, so it
    // hands back the cancelled zero itself: its own space, not new space.
    let owned = Accumulator::new() + zero.clone();
    println!(
        "SCRATCH owned + (returns rhs): stored {}, retained {}",
        owned.stored_digit_count(),
        owned.digits.retained_len()
    );

    // Shifting each result again must stay constant too.
    let mut acc = zero.clone();
    acc <<= 32 * K;
    check("<<= by the zero's own width", acc);
}
