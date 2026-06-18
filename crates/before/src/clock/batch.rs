//! The amortizing mutation handle for a [`Clock`].
//!
//! [`Batch`] borrows the clock's party directly and wraps its version in a
//! [`version::Batch`], repacking once when dropped.

use crate::error::Overlap;
use crate::{version, Party, Version};

use super::Clock;

/// A batch for a [`Clock`], providing a similar API, but faster for multiple
/// operations.
///
/// ```
/// use before::Clock;
/// let mut clock = Clock::seed();
/// clock.batch().tick().tick().tick(); // three ticks, one repack on drop
/// assert_eq!(clock.version().to_string(), "3");
/// ```
pub struct Batch<'c> {
    party: &'c mut Party,
    version: version::Batch<'c>,
}

impl<'c> Batch<'c> {
    /// Begin a batch over `clock`, borrowing its party and wrapping its
    /// version. The public entry point is [`Clock::batch`].
    pub(super) fn new(clock: &'c mut Clock) -> Self {
        let Clock { party, version } = clock;
        Batch {
            party,
            version: version.batch(),
        }
    }
}

impl Batch<'_> {
    /// Like [`tick`](Clock::tick), but chainable.
    ///
    /// ```
    /// use before::Clock;
    /// let mut clock = Clock::seed();
    /// clock.batch().tick().tick();
    /// assert_eq!(clock.version().to_string(), "2");
    /// ```
    pub fn tick(&mut self) -> &mut Self {
        self.version.tick(&*self.party);
        self
    }

    /// Like `|=`, but chainable.
    pub(crate) fn join_version(&mut self, version: &Version) -> &mut Self {
        self.version.join(version);
        self
    }

    /// Like [`fork`](Clock::fork).
    ///
    /// ```
    /// use before::Clock;
    /// let mut parent = Clock::seed();
    /// let child = parent.batch().fork();
    /// assert!(parent.party().is_disjoint(child.party()));
    /// ```
    pub fn fork(&mut self) -> Clock {
        let child_party = self.party.fork();
        let child_version = self.version.snapshot();
        Clock::from_parts(child_party, child_version)
    }

    /// Like [`join`](Clock::join).
    ///
    /// ```
    /// use before::Clock;
    /// let mut a = Clock::seed();
    /// let b = a.fork();
    /// assert!(a.batch().join(b).is_ok());
    /// ```
    pub fn join(&mut self, other: Clock) -> Result<&version::Batch<'_>, Clock> {
        let (other_party, other_version) = other.into_parts();
        match self.party.join(other_party) {
            Ok(()) => {
                self.version.join(&other_version);
                Ok(self.version())
            }
            Err(other_party) => Err(Clock::from_parts(other_party, other_version)),
        }
    }

    /// Like [`sync`](Clock::sync).
    ///
    /// ```
    /// use before::Clock;
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// assert!(a.batch().sync(&mut b.batch()).is_ok());
    /// ```
    pub fn sync(&mut self, other: &mut Batch<'_>) -> Result<&version::Batch<'_>, Overlap> {
        // Merge both parties into self, then re-split: self keeps one half, other the
        // other. `join` is the overlap check — on failure it hands the party back and
        // leaves `self` unchanged, so we restore `other` and report the overlap.
        let theirs = core::mem::replace(other.party, Party::anonymous());
        if let Err(theirs) = self.party.join(theirs) {
            *other.party = theirs;
            return Err(Overlap);
        }
        *other.party = self.party.fork();

        // Both histories become the join of the two.
        let other_version = other.version.snapshot();
        self.version.join(&other_version);
        let merged = self.version.snapshot();
        self.version.replace_with(merged.clone());
        other.version.replace_with(merged);
        Ok(self.version())
    }

    /// The in-progress version, for comparison (no repack).
    ///
    /// ```
    /// use before::{Clock, Version};
    /// let mut clock = Clock::seed();
    /// let mut batch = clock.batch();
    /// batch.tick();
    /// assert!(batch.version() > Version::new());
    /// ```
    pub fn version(&self) -> &version::Batch<'_> {
        &self.version
    }

    /// The current party (may have changed via fork/join/sync).
    ///
    /// ```
    /// use before::Clock;
    /// let mut clock = Clock::seed();
    /// assert_eq!(clock.batch().party().to_string(), "1");
    /// ```
    pub fn party(&self) -> &Party {
        &*self.party
    }
}

/// Borrow a [`Clock`] as a [`Batch`]; equivalent to [`Clock::batch`].
///
/// ```
/// use before::{batch, Clock};
/// let mut clock = Clock::seed();
/// let _batch: batch::Clock = (&mut clock).into();
/// ```
impl<'a> From<&'a mut Clock> for Batch<'a> {
    fn from(c: &'a mut Clock) -> Self {
        c.batch()
    }
}
