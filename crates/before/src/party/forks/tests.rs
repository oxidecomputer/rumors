//! Checks for balanced splitting above machine-sized counts.

use super::{Forks, Party};
use crate::Ticks;

/// The internal remaining count stays exact above `u128::MAX`.
///
/// Constructing the iterator preserves the requested count, and yielding one
/// share subtracts exactly one.
#[test]
fn unbounded_remainder_stays_exact() {
    let count = Ticks::from(u128::MAX) + Ticks::from(1u8);
    let after_one = Ticks::from(u128::MAX);
    let mut keeper = Party::seed();
    let mut forks = Forks::new(&mut keeper, count.clone());

    assert_eq!(forks.split.remaining, count);
    assert!(forks.next().is_some());
    assert_eq!(forks.split.remaining, after_one);
}
