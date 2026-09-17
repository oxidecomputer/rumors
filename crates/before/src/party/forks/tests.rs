//! Checks for the arbitrary-precision count inside the balanced split.

use super::{Forks, Party};
use crate::Ticks;

/// Reserving the residual does not narrow or saturate a count wider than
/// `u128`, and each yielded share decrements that exact count once.
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
