//! Operations that could meet the same accumulator twice.
//!
//! An accumulator's value lives in a `RefCell`, so a second borrow while one
//! is held panics. That can happen in two ways, and these tests cover both:
//!
//! - The same accumulator as both operands. Only operations that borrow both
//!   operands shared can receive it twice; an assignment or shifted merge
//!   borrows its receiver mutably, which the compile-fail examples on those
//!   operations show the borrow checker rejects.
//! - Caller code that reads the accumulator while an operation runs. Caller
//!   code enters only through a formatting sink, a hasher, or an iterator.
//!
//! Each value is built with a cancelled top, so the operation's reads compact
//! it, and each test checks the result as well as the absence of a panic.

use core::cmp::Ordering;
use core::fmt::{self, Write as _};
use core::hash::{BuildHasher, Hash, Hasher};
use std::collections::hash_map::RandomState;

use num_bigint::BigInt as IBig;

use super::{assert_value, cancelled_top, Accumulator};

/// A value of 12345 under a top cancelled at position 20.
fn shared_value() -> (Accumulator, IBig) {
    let mut value = cancelled_top(20);
    value += 12_345;
    (value, IBig::from(12_345))
}

/// An accumulator equals itself.
#[test]
#[allow(
    clippy::eq_op,
    reason = "comparing a value with itself is the case under test"
)]
fn equals_itself() {
    let (value, _) = shared_value();
    assert!(value == value);
}

/// An accumulator compares equal to itself under every ordering form.
#[test]
#[allow(
    clippy::eq_op,
    reason = "comparing a value with itself is the case under test"
)]
fn orders_equal_to_itself() {
    let (value, _) = shared_value();
    assert_eq!(value.cmp(&value), Ordering::Equal);
    assert_eq!(value.partial_cmp(&value), Some(Ordering::Equal));
    assert!(value <= value);
    assert!(value >= value);
    let less = value < value;
    let greater = value > value;
    assert!(!less && !greater);
}

/// Adding an accumulator to itself through two borrows doubles it.
#[test]
fn borrowed_sum_with_itself_doubles() {
    let (value, oracle) = shared_value();
    assert_value(&(&value + &value), &(&oracle * 2));
    assert_value(&value, &oracle);
}

/// Subtracting an accumulator from itself through two borrows is zero.
#[test]
fn borrowed_difference_with_itself_is_zero() {
    let (value, oracle) = shared_value();
    assert_value(&(&value - &value), &IBig::ZERO);
    assert_value(&value, &oracle);
}

/// Summing the same borrowed accumulator three times triples it.
#[test]
fn sum_of_repeated_borrows_multiplies() {
    let (value, oracle) = shared_value();
    let total: Accumulator = [&value, &value, &value].into_iter().sum();
    assert_value(&total, &(&oracle * 3));
}

/// Hashing an accumulator twice gives one hash, though the first hash
/// compacts its stored form.
#[test]
fn hashes_itself_consistently() {
    let (value, _) = shared_value();
    let hasher = RandomState::new();
    let first = hasher.hash_one(&value);
    assert_eq!(hasher.hash_one(&value), first);
}

/// An accumulator can move to another thread: its cell holds owned data only.
#[test]
fn moves_between_threads() {
    let (value, oracle) = shared_value();
    let moved = std::thread::spawn(move || value)
        .join()
        .expect("the thread returns");
    assert_value(&moved, &oracle);
}

/// A formatting sink that reads the accumulator being formatted on every write.
struct ReadingSink<'a> {
    /// The accumulator the sink reads.
    watched: &'a Accumulator,
    /// The text written so far.
    text: String,
}

impl fmt::Write for ReadingSink<'_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let _ = (
            self.watched.sign(),
            self.watched.signed_magnitude(),
            self.watched.is_known_zero(),
        );
        self.text.push_str(text);
        Ok(())
    }
}

/// Every formatting trait writes to a sink that reads the same accumulator,
/// and produces the same text as formatting into a `String`.
#[test]
fn formatting_sinks_may_read_the_accumulator() {
    let (value, _) = shared_value();
    let expected = [
        format!("{value}"),
        format!("{value:+}"),
        format!("{value:>12}"),
        format!("{value:?}"),
        format!("{value:#?}"),
    ];
    let mut sink = ReadingSink {
        watched: &value,
        text: String::new(),
    };
    write!(sink, "{value}").unwrap();
    write!(sink, "{value:+}").unwrap();
    write!(sink, "{value:>12}").unwrap();
    write!(sink, "{value:?}").unwrap();
    write!(sink, "{value:#?}").unwrap();
    assert_eq!(sink.text, expected.concat());
}

/// A hasher that reads the accumulator being hashed on every write.
struct ReadingHasher<'a> {
    /// The accumulator the hasher reads.
    watched: &'a Accumulator,
    /// The bytes written so far.
    bytes: Vec<u8>,
}

impl Hasher for ReadingHasher<'_> {
    fn write(&mut self, bytes: &[u8]) {
        let _ = (self.watched.sign(), self.watched.signed_magnitude());
        self.bytes.extend_from_slice(bytes);
    }

    fn finish(&self) -> u64 {
        self.bytes.len() as u64
    }
}

/// Hashing writes to a hasher that reads the same accumulator, and feeds it
/// the same bytes as hashing an equal value in another stored form.
#[test]
fn hashers_may_read_the_accumulator() {
    let (value, _) = shared_value();
    let mut reading = ReadingHasher {
        watched: &value,
        bytes: Vec::new(),
    };
    value.hash(&mut reading);
    let plain = Accumulator::from(12_345);
    let mut plain_hasher = ReadingHasher {
        watched: &plain,
        bytes: Vec::new(),
    };
    plain.hash(&mut plain_hasher);
    assert_eq!(reading.bytes, plain_hasher.bytes);
}

/// A sum's iterator may read every accumulator it yields, and another, between
/// yields.
#[test]
fn sum_iterators_may_read_their_operands() {
    let (value, oracle) = shared_value();
    let other = Accumulator::from(-5);
    let total: Accumulator = [&value, &other, &value]
        .into_iter()
        .inspect(|operand| {
            let _ = (operand.sign(), value.signed_magnitude(), other.is_zero());
        })
        .sum();
    assert_value(&total, &(&oracle * 2 - 5));
}
