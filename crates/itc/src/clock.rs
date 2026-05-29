//! [`Clock`] — a [`Party`] paired with a [`Version`], and its working-form [`Batch`].
//!
//! A [`clock::Batch`](Batch) is a split borrow of a `Clock`: the party (which has no
//! working form — id ops run on the packed bits directly) plus a [`version::Batch`]
//! over the version. Each `Clock` method is a single-op batch; the version repacks once
//! when the inner `version::Batch` drops.

use core::ops::{BitOr, BitOrAssign};

use crate::{codec, version, DecodeError, OverlapError, ParseError, Party, Version};

#[cfg(test)]
mod tests;

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

    /// Whether this clock's history already dominates `msg` (`msg <= version`).
    pub fn has_seen(&self, msg: &Version) -> bool {
        msg <= &self.version
    }

    /// Whether this clock's history strictly precedes `other`'s.
    pub fn happens_before(&self, other: &Clock) -> bool {
        self.version < other.version
    }

    /// Whether this clock's history is concurrent with `other`'s.
    pub fn concurrent_with(&self, other: &Clock) -> bool {
        self.version.partial_cmp(&other.version).is_none()
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

    /// Begin a batch of operations on this clock.
    ///
    /// The same operations are available on a [`Batch`] as on a [`Clock`], but
    /// sequential operations within a batch are more efficient.
    pub fn batch(&mut self) -> Batch<'_> {
        let Clock { party, version } = self;
        Batch {
            party,
            version: version.batch(),
        }
    }

    /// The canonical packed byte encoding: `enc_id(party)` then `enc_ev(version)`,
    /// bit-concatenated with no padding between, then zero-padded to a byte boundary.
    pub fn encode(&self) -> Vec<u8> {
        let mut bits = self.party.as_bits().to_bitvec();
        bits.extend_from_bitslice(self.version.as_bits());
        codec::pack_to_bytes(&bits)
    }

    /// Decode a byte string, strictly rejecting malformed or non-canonical input.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let bits = codec::Bits::from_slice(bytes);
        let after_id = codec::parse_id(&bits, 0)?;
        let after_ev = codec::parse_ev(&bits, after_id)?;
        codec::require_zero_padding(&bits, after_ev)?;
        let party = Party::from_bits(bits[..after_id].to_bitvec());
        let version = Version::from_bits(bits[after_id..after_ev].to_bitvec());
        Ok(Clock::from_parts(party, version))
    }
}

/// Paper stamp notation: `(<id>, <event>)`, e.g. `(1, 0)` for [`Clock::seed`].
impl core::fmt::Display for Clock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "({}, {})", self.party, self.version)
    }
}

impl core::fmt::Debug for Clock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Clock")
            .field("party", &self.party)
            .field("version", &self.version)
            .finish()
    }
}

/// Parse a stamp `(i, e)` in paper notation, strictly rejecting non-normal-form input
/// and an anonymous (id `0`) party.
impl core::str::FromStr for Clock {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, ParseError> {
        let (id, ev) = codec::parse_clock_str(s)?;
        if codec::id_is_empty(&id) {
            return Err(ParseError::Anonymous);
        }
        Ok(Clock::from_parts(
            Party::from_bits(id),
            Version::from_bits(ev),
        ))
    }
}

/// A clock from a `(party, version)` literal, e.g. `Clock::try_from(((1u8, 0u8), 5u64))`,
/// grounding on the recursive [`Party`]/[`Version`] literal forms.
impl<I, E> TryFrom<(I, E)> for Clock
where
    Party: TryFrom<I, Error = ParseError>,
    Version: TryFrom<E, Error = ParseError>,
{
    type Error = ParseError;
    fn try_from((i, e): (I, E)) -> Result<Self, ParseError> {
        Ok(Clock::from_parts(
            Party::try_from(i)?,
            Version::try_from(e)?,
        ))
    }
}

/// A session over a [`Clock`], built on [`version::Batch`]. The version repacks when
/// the inner `version::Batch` drops; the party is mutated in place (it has no working
/// form).
pub struct Batch<'c> {
    party: &'c mut Party,
    version: version::Batch<'c>,
}

impl Batch<'_> {
    /// Advance the clock's own component. Chainable.
    pub fn tick(&mut self) -> &mut Self {
        self.version.tick(&*self.party);
        self
    }

    /// Merge a received message in place. Chainable.
    pub fn merge(&mut self, msg: &Version) -> &mut Self {
        self.version.merge(msg);
        self
    }

    /// Split off a child clock; the child gets the current version.
    pub fn fork(&mut self) -> Clock {
        let child_party = self.party.fork();
        let child_version = self.version.snapshot();
        Clock::from_parts(child_party, child_version)
    }

    /// Absorb a disjoint clock; on overlap, hand it back.
    pub fn join(&mut self, other: Clock) -> Result<(), Clock> {
        let (other_party, other_version) = other.into_parts();
        match self.party.join(other_party) {
            Ok(()) => {
                self.version.merge(&other_version);
                Ok(())
            }
            Err(other_party) => Err(Clock::from_parts(other_party, other_version)),
        }
    }

    /// Reconcile with another live batch (keeps both live): merge the two parties and
    /// re-split them, and bring both versions to the join of the two.
    pub fn sync(&mut self, other: &mut Batch<'_>) -> Result<(), OverlapError> {
        // Merge both parties into self, then re-split: self keeps one half, other the
        // other. `join` is the overlap check — on failure it hands the party back and
        // leaves `self` unchanged, so we restore `other` and report the overlap.
        let theirs = core::mem::replace(other.party, Party::empty());
        if let Err(theirs) = self.party.join(theirs) {
            *other.party = theirs;
            return Err(OverlapError);
        }
        *other.party = self.party.fork();

        // Both histories become the join of the two.
        let other_version = other.version.snapshot();
        self.version.merge(&other_version);
        let merged = self.version.snapshot();
        other.version.merge(&merged);
        Ok(())
    }

    /// The in-progress version, for comparison (no repack).
    pub fn version(&self) -> &version::Batch<'_> {
        &self.version
    }

    /// The current party (may have changed via fork/join/sync).
    pub fn party(&self) -> &Party {
        &*self.party
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
    fn bitor(mut self, r: Version) -> Clock {
        self.batch().merge(&r);
        self
    }
}

impl BitOr<Clock> for Version {
    type Output = Clock;
    fn bitor(self, mut r: Clock) -> Clock {
        r.batch().merge(&self);
        r
    }
}

impl BitOrAssign<Version> for Clock {
    fn bitor_assign(&mut self, r: Version) {
        self.batch().merge(&r);
    }
}

impl BitOrAssign<&Version> for Batch<'_> {
    fn bitor_assign(&mut self, r: &Version) {
        self.merge(r);
    }
}
