//! Lazy balanced child clocks.
//!
//! The party iterator performs the balanced partition. This layer pairs each
//! returned share with a clone of the parent's version; the consuming array
//! conversion applies the same pairing to a fixed-size party split.

use core::iter::FusedIterator;

use crate::{party, Clock, Count, Party, Version};

/// A lazy iterator of balanced child [`Clock`]s, returned by [`Clock::forks`].
///
/// Yields exactly `k` disjoint clocks, each pairing one structurally balanced
/// [`Party`] with a clone of the parent's [`Version`]. The clock it borrows
/// keeps the residual and every party share not yet returned.
///
/// [`Iterator::size_hint`] is exact whenever the number of clocks still to
/// yield fits `usize`. When more than `usize::MAX` remain, it reports
/// `(usize::MAX, None)`.
///
/// # Complexity
///
/// Construction costs `O(log k)` for the count representation. Each `next`
/// costs `O(|p| + log k)`, with `|p|` the size of the borrowed clock's party
/// when the iterator was created; the child's version shares the parent's
/// stored bytes through an `O(1)` clone. A full drain takes at most
/// `O(|c| + k (|c| + log k))` time, with `|c|` the borrowed clock's size;
/// measured on [`Clock::forks`].
#[must_use = "`ClockForks` produces no child clocks unless consumed"]
pub struct ClockForks<'a> {
    /// The lazy partition of party shares.
    parties: party::PartyForks<'a>,
    /// The parent version, cloned into every child clock.
    version: &'a Version,
}

impl<'a> ClockForks<'a> {
    /// Borrow `clock` and reserve `k` balanced child clocks. The public entry
    /// point is [`Clock::forks`].
    pub(super) fn new(clock: &'a mut Clock, k: Count) -> Self {
        let Clock { party, version } = clock;
        let version: &Version = version; // the children only read it, to clone
        ClockForks {
            parties: party::PartyForks::new(party, k),
            version,
        }
    }

    /// Pair a party share with a clone of the parent version.
    fn clock(&self, party: Party) -> Clock {
        Clock::from_parts(party, self.version.clone())
    }
}

impl Iterator for ClockForks<'_> {
    type Item = Clock;
    fn next(&mut self) -> Option<Clock> {
        let party = self.parties.next()?;
        Some(self.clock(party))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.parties.size_hint()
    }
}

/// Once `next` returns `None`, it returns `None` forever.
impl FusedIterator for ClockForks<'_> {}

/// Splits a [`Clock`] into exactly `N` balanced child clocks, consuming it.
///
/// The static counterpart of [`forks`](Clock::forks). Each child pairs one
/// balanced [`Party`] share (see [`From<Party>`](Party) for `[Party; N]`) with
/// a clone of the clock's [`Version`].
///
/// # The `N >= 1` bound
///
/// A clock owns a nonempty party and cannot vanish into zero shares, so `N`
/// must be at least 1 — the same bound as the [`Party`] split, enforced the
/// same way at compile time: the zero-length split is rejected when the
/// conversion is built, while the same spelling at any nonzero arity compiles
/// and runs.
///
/// ```
/// use before::Clock;
/// let _children: [Clock; 1] = Clock::seed().into();
/// ```
///
/// ```compile_fail,E0080
/// use before::Clock;
/// let _children: [Clock; 0] = Clock::seed().into();
/// ```
///
/// # Complexity
///
/// Takes `O(|clock| + N (|clock| + log N))` time; measured on
/// [`Clock::forks`].
///
/// # Example
///
/// ```
/// use before::Clock;
/// let [a, b]: [Clock; 2] = Clock::seed().into();
/// assert!(a.party().is_disjoint(b.party()));
/// assert_eq!(a.version(), b.version()); // both carry the seed's version
/// ```
impl<const N: usize> From<Clock> for [Clock; N] {
    fn from(clock: Clock) -> [Clock; N] {
        // Fires at monomorphization, making `N == 0` a build error — before the
        // delegated party split's own `N >= 1` assert would. The paired
        // doctests above pin it: the `compile_fail` twin must be rejected while
        // its identical-but-for-arity sibling compiles.
        const { assert!(N >= 1, "a `Clock` cannot split into zero shares") }
        let (party, version) = clock.into_parts();
        let parties: [Party; N] = party.into();
        parties.map(|party| Clock::from_parts(party, version.clone()))
    }
}
