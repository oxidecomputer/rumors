//! [`Clock`] — a [`Party`] paired with a [`Version`], and its working-form [`Batch`].

use core::marker::PhantomData;
use core::ops::{BitOr, BitOrAssign};

use crate::{version, DecodeError, OverlapError, Party, Version};

/// A `Party` paired with a `Version`. Not `Clone`. Implements no comparison
/// traits — compare the party and version separately with any lexicography.
pub struct Clock {
    party: Party,
    version: Version,
}

impl Clock {
    /// A fresh clock owning the whole id space with empty history.
    pub fn seed() -> Self {
        Self::from_parts(Party::seed(), Version::new())
    }

    /// Pair an existing party and version into a clock.
    pub fn from_parts(party: Party, version: Version) -> Self {
        Clock { party, version }
    }

    /// Decompose into the owned party and version.
    pub fn into_parts(self) -> (Party, Version) {
        (self.party, self.version)
    }

    /// The clock's party (its share of the id space).
    pub fn party(&self) -> &Party {
        &self.party
    }

    /// Snapshot the history as a transmittable `Version`. Does not advance.
    pub fn version(&self) -> Version {
        self.version.clone()
    }

    /// Advance this clock's own component by one event.
    pub fn tick(&mut self) {
        self.batch().tick();
    }

    /// Split off a child clock; `self` keeps half the id space, the child the
    /// other half. Both carry the current version.
    pub fn fork(&mut self) -> Clock {
        self.batch().fork()
    }

    /// Absorb a disjoint clock's party and history; on overlap, hand it back.
    pub fn join(&mut self, other: Clock) -> Result<(), Clock> {
        self.batch().join(other)
    }

    /// Reconcile two clocks: merge histories and re-split the merged party.
    pub fn sync(&mut self, other: &mut Clock) -> Result<(), OverlapError> {
        self.batch().sync(&mut other.batch())
    }

    /// Whether this clock's history already dominates `msg`.
    pub fn has_seen(&self, msg: &Version) -> bool {
        let _ = msg;
        todo!()
    }

    /// Whether this clock's history strictly precedes `other`'s.
    pub fn happens_before(&self, other: &Clock) -> bool {
        let _ = other;
        todo!()
    }

    /// Whether this clock's history is concurrent with `other`'s.
    pub fn concurrent_with(&self, other: &Clock) -> bool {
        let _ = other;
        todo!()
    }

    /// Advance, then snapshot the history to transmit.
    pub fn send(&mut self) -> Version {
        self.tick();
        self.version()
    }

    /// Merge a received message, then advance this clock's own component.
    pub fn receive(&mut self, msg: Version) {
        self.batch().merge(&msg).tick();
    }

    /// Begin a working-form session over this clock.
    pub fn batch(&mut self) -> Batch<'_> {
        todo!()
    }

    /// The canonical packed byte encoding: `enc_id(party)` then `enc_ev(version)`,
    /// zero-padded to a byte boundary.
    pub fn encode(&self) -> Vec<u8> {
        todo!()
    }

    /// Decode a byte string, strictly rejecting malformed or non-canonical input.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let _ = bytes;
        todo!()
    }
}

impl core::fmt::Debug for Clock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let _ = f;
        todo!()
    }
}

/// A session over a [`Clock`], built on [`version::Batch`]. Repacks on drop.
pub struct Batch<'c> {
    _p: PhantomData<&'c mut Clock>,
}

impl Batch<'_> {
    /// Advance the clock's own component. Chainable.
    pub fn tick(&mut self) -> &mut Self {
        todo!()
    }

    /// Merge a received message in place. Chainable.
    pub fn merge(&mut self, msg: &Version) -> &mut Self {
        let _ = msg;
        todo!()
    }

    /// Split off a child clock; the child gets the current version.
    pub fn fork(&mut self) -> Clock {
        todo!()
    }

    /// Absorb a disjoint clock; on overlap, hand it back.
    pub fn join(&mut self, other: Clock) -> Result<(), Clock> {
        let _ = other;
        todo!()
    }

    /// Reconcile with another live batch (keeps both live).
    pub fn sync(&mut self, other: &mut Batch<'_>) -> Result<(), OverlapError> {
        let _ = other;
        todo!()
    }

    /// The in-progress version, for comparison (no repack).
    pub fn version(&self) -> &version::Batch<'_> {
        todo!()
    }

    /// The current party (may have changed via fork/join/sync).
    pub fn party(&self) -> &Party {
        todo!()
    }
}

impl Drop for Batch<'_> {
    fn drop(&mut self) {
        // Repack version into *clock if materialized.
    }
}

impl<'a> From<&'a mut Clock> for Batch<'a> {
    fn from(c: &'a mut Clock) -> Self {
        c.batch()
    }
}

// Join operators. The `Clock` operand is consumed (a borrowing form would
// duplicate its party). No `Clock | Clock` — that is the fallible `Clock::join`.

impl BitOr<Version> for Clock {
    type Output = Clock;
    fn bitor(self, r: Version) -> Clock {
        let _ = r;
        todo!()
    }
}

impl BitOr<Clock> for Version {
    type Output = Clock;
    fn bitor(self, r: Clock) -> Clock {
        let _ = r;
        todo!()
    }
}

impl BitOrAssign<Version> for Clock {
    fn bitor_assign(&mut self, r: Version) {
        let _ = r;
        todo!()
    }
}

impl BitOrAssign<&Version> for Batch<'_> {
    fn bitor_assign(&mut self, r: &Version) {
        self.merge(r);
    }
}
