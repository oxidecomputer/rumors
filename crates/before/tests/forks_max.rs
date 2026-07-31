//! `forks(usize::MAX)`: the documented behavior at the split count's
//! overflow boundary, pinned in both profiles.
//!
//! The split carves `n + 1` shares, so `n == usize::MAX` has no headroom
//! for the residual. In builds with debug assertions the addition panics
//! (and a caught unwind leaves the caller's `Party` emptied — destroyed,
//! never duplicated); without them the count wraps and the returned
//! iterator yields nothing while reporting `usize::MAX` remaining. These
//! tests hold the `# Panics` sections on [`Party::forks`] and
//! [`Clock::forks`] to what actually happens, so any change at this
//! boundary moves a committed reading instead of drifting silently.

use before::Party;

#[cfg(debug_assertions)]
use before::Clock;

/// Debug profile: `Party::forks(usize::MAX)` panics on the unguarded
/// `n + 1`, exactly as its `# Panics` section states.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "attempt to add with overflow")]
fn party_forks_max_panics_in_debug() {
    let mut p = Party::seed();
    let _ = p.forks(usize::MAX);
}

/// Debug profile: `Clock::forks` inherits the same overflow panic
/// through the party split it drives.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "attempt to add with overflow")]
fn clock_forks_max_panics_in_debug() {
    let mut c = Clock::seed();
    let _ = c.forks(usize::MAX);
}

/// Debug profile: a caller that catches the unwind is left holding an
/// emptied `Party`.
///
/// The region was moved into the split before the overflow, so it is
/// dropped during unwind (destroyed, never duplicated: the Law of
/// Disjointness is safe, the handle is not).
#[cfg(debug_assertions)]
#[test]
fn party_forks_max_caught_unwind_leaves_an_emptied_party() {
    use std::panic;
    let mut p = Party::seed();
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let _ = p.forks(usize::MAX);
    }));
    assert!(result.is_err(), "the overflow must panic in debug");
    // The whole region was moved out and dropped during unwind; the
    // caller's binding holds the anonymous party.
    assert_eq!(p.to_string(), "0");
}

/// Release profile: the wrap yields an iterator that claims
/// `usize::MAX` shares and yields none.
///
/// The [`ExactSizeIterator`] contract is violated at exactly this
/// input, while the borrowed party soundly keeps the whole region.
#[cfg(not(debug_assertions))]
#[test]
fn party_forks_max_wraps_in_release() {
    let mut p = Party::seed();
    {
        let mut it = p.forks(usize::MAX);
        assert_eq!(it.size_hint(), (usize::MAX, Some(usize::MAX)));
        assert_eq!(it.len(), usize::MAX);
        assert!(it.next().is_none(), "claims usize::MAX shares, yields none");
    }
    // Soundness: the borrowed party still holds the entire region.
    assert!(p.is_seed());
}
