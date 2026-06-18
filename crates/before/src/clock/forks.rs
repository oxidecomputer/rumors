//! Balanced n-way fork for [`Clock`]: [`Clock::forks`] and its [`Forks`]
//! iterator, plus the consuming [`From<Clock>`](From) for `[Clock; N]` static
//! split.
//!
//! A clock splits exactly as its [`Party`] does — see [`party::Forks`] for the
//! lazy, minimal-depth partition — with every share carrying a clone of the
//! clock's [`Version`], the same rule as [`Clock::fork`].

use crate::{party, Clock, Party, Version};

/// A lazy iterator of balanced child [`Clock`]s, returned by [`Clock::forks`].
///
/// Yields exactly `n` disjoint clocks, each pairing one balanced [`Party`]
/// share with a clone of the parent's [`Version`]. The clock it borrows keeps
/// the residual party share and its version, and is never left empty; party
/// shares not taken before the iterator drops are rejoined into it (its version
/// untouched).
pub struct Forks<'a> {
    /// The lazy partition of party shares; its [`Drop`] folds unconsumed shares
    /// back into the borrowed clock's party.
    parties: party::Forks<'a>,
    /// The parent version, cloned into every child clock.
    version: &'a Version,
}

impl<'a> Forks<'a> {
    /// Borrow `clock` and reserve `n` balanced child clocks. The public entry
    /// point is [`Clock::forks`].
    pub(super) fn new(clock: &'a mut Clock, n: usize) -> Self {
        let Clock { party, version } = clock;
        let version: &Version = version; // the children only read it, to clone
        Forks {
            parties: party::Forks::new(party, n),
            version,
        }
    }

    /// Pair a party share with a clone of the parent version.
    fn clock(&self, party: Party) -> Clock {
        Clock::from_parts(party, self.version.clone())
    }
}

impl Iterator for Forks<'_> {
    type Item = Clock;
    fn next(&mut self) -> Option<Clock> {
        let party = self.parties.next()?;
        Some(self.clock(party))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.parties.size_hint()
    }
}

impl ExactSizeIterator for Forks<'_> {}

/// Splits a [`Clock`] into exactly `N` balanced child clocks, consuming it.
///
/// The static counterpart of [`forks`](Clock::forks). Each child pairs one
/// balanced [`Party`] share (see [`From<Party>`](Party) for `[Party; N]`) with
/// a clone of the clock's [`Version`].
///
/// `N` must be at least 1, for the same reason as the [`Party`] split: a clock
/// owns a nonempty party and cannot vanish into zero shares.
///
/// ```
/// use before::Clock;
/// let [a, b]: [Clock; 2] = Clock::seed().into();
/// assert!(a.party().is_disjoint(b.party()));
/// assert_eq!(a.version(), b.version()); // both carry the seed's version
/// ```
impl<const N: usize> From<Clock> for [Clock; N] {
    fn from(clock: Clock) -> [Clock; N] {
        const { assert!(N >= 1, "a `Clock` cannot split into zero shares") }
        let (party, version) = clock.into_parts();
        let parties: [Party; N] = party.into();
        parties.map(|party| Clock::from_parts(party, version.clone()))
    }
}
