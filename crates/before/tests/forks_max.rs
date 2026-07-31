//! `forks(u64::MAX)`: the documented behavior at the split count's
//! saturation boundary, pinned in both profiles.
//!
//! The split carves `n + 1` shares and the count saturates, so
//! `n == u64::MAX` is total: no panic in any profile, the caller's
//! handle keeps a valid region throughout, and the iterator claims (and
//! would yield) `u64::MAX − 1` shares — one fewer than asked, at the
//! single input where the asked-for count has no headroom for the
//! residual. These tests hold [`Party::forks`] and [`Clock::forks`] to
//! exactly that contract, so any change at this boundary moves a
//! committed reading instead of drifting silently.

use before::{Clock, Party};

/// `Party::forks(u64::MAX)` is total: no panic, the iterator reports
/// `u64::MAX - 1` shares (the saturated count minus the residual), the
/// first shares are genuinely produced and disjoint from the keeper,
/// and dropping the iterator folds everything back into the caller's
/// party.
#[test]
fn party_forks_max_saturates_without_panic() {
    let mut p = Party::seed();
    let share = {
        let mut it = p.forks(u64::MAX);
        let expected = usize::try_from(u64::MAX - 1).expect("64-bit test host");
        assert_eq!(it.size_hint(), (expected, Some(expected)));
        assert_eq!(it.len(), expected);
        it.next().expect("a saturated split still yields shares")
    };
    // Drop folded the untaken shares back; only the taken share is
    // still out, disjoint from everything the keeper holds.
    assert!(
        p.is_disjoint(&share),
        "a yielded share is disjoint from the keeper's holdings"
    );
    p.join(share).expect("disjoint shares rejoin");
    assert!(p.is_seed(), "the borrowed party recovers the entire region");
}

/// `Clock::forks(u64::MAX)` rides the same saturating split: no panic,
/// one child fewer than asked, and the borrowed clock stays a valid
/// identity holding its whole region after the drop.
#[test]
fn clock_forks_max_saturates_without_panic() {
    let mut c = Clock::seed();
    let child = {
        let mut it = c.forks(u64::MAX);
        let expected = usize::try_from(u64::MAX - 1).expect("64-bit test host");
        assert_eq!(it.len(), expected);
        it.next().expect("a saturated split still yields children")
    };
    // Drop rejoined the untaken party shares; only the taken child is
    // still out, disjoint and carrying a clone of the parent version.
    assert!(c.party().is_disjoint(child.party()));
    assert_eq!(child.version(), c.version(), "children clone the version");
    c.join(child).expect("disjoint children rejoin");
    assert_eq!(
        c.party().to_string(),
        "1",
        "the borrowed clock recovers the entire region"
    );
}
