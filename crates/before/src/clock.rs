//! A [`Clock`] is a [`Party`] paired with a [`Version`].
//!
//! A [`clock::Batch`](Batch) is a borrow of a `Clock` affording the same
//! interface but faster for bulk operations.

use core::ops::{BitOr, BitOrAssign};

use crate::{codec, version, DecodeError, OverlapError, ParseError, Party, Version};

#[cfg(test)]
mod tests;

/// A [`Party`] and its [`Version`].
///
/// This type is `!Clone` to discourage non-linear usage: while using a
/// [`Clock`] non-linearly is "safe" from the perspective of Rust, it is invalid
/// in the setting of interval tree clocks, which requires that all live clocks
/// in the system **must** be disjoint.
pub struct Clock {
    party: Party,
    version: Version,
}

impl Clock {
    /// The initial clock of the distinguished [`Party::seed`]; the only
    /// [`Clock`] which is not derived from some prior clock.
    ///
    /// In any given system of clocks, this function should only be called by
    /// one party in the entire system, and only once: all its descendents are
    /// necessarily disjoint, but the descendents of parallel seeds need not be;
    /// if ever the twain meet, invariants and expectations will be violated.
    pub fn seed() -> Self {
        Self::from_parts(Party::seed(), Version::new())
    }

    /// A [`Clock`] is merely the pair of a [`Version`] and its [`Party`], for
    /// convenience.
    pub fn from_parts(party: Party, version: Version) -> Self {
        Clock { party, version }
    }

    /// Decompose into the owned party and version.
    pub fn into_parts(self) -> (Party, Version) {
        (self.party, self.version)
    }

    /// The party whose causal history this clock tracks.
    pub fn party(&self) -> &Party {
        &self.party
    }

    /// Snapshot the current state of the [`Clock`] as a transmittable [`Version`].
    ///
    /// This does not advance the clock.
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// Advance this [`Clock`] by one event for its own [`Party`], returning the
    /// new [`Version`].
    pub fn tick(&mut self) -> &Version {
        self.batch().tick();
        self.version()
    }

    /// Split off a child clock by forking the underlying [`Party`].
    ///
    /// Both resultant clocks carry the current [`Version`].
    ///
    /// Repeatedly calling [`fork`](Clock::fork) on the same [`Clock`] will lead
    /// to imbalanced internal tree representations and worse memory usage and
    /// performance; it's recommended to randomize which [`Clock`]s are
    /// [`fork`](Clock::fork)ed.
    pub fn fork(&mut self) -> Clock {
        self.batch().fork()
    }

    /// Absorb a *disjoint* [`Clock`]'s [`Party`] and [`Version`], returning the
    /// new [`Version`].
    ///
    /// # Errors
    ///
    /// If the [`Clock`]s' [`Party`]s overlap, `self` is unmodified and
    /// `Err(other)` is returned unmodified.
    pub fn join(&mut self, other: Clock) -> Result<&Version, Clock> {
        self.batch().join(other)?;
        Ok(self.version())
    }

    /// Reconcile two *disjoint* [`Clock`]s: join their [`Version`]s and
    /// re-[`fork`](Clock::fork) the [`join`](Clock::join) of their [`Party`]s.
    ///
    /// # Errors
    ///
    /// If the [`Clock`]s' [`Party`]s overlap, an error is returned and `self`
    /// and `other` are left unmodified.
    pub fn sync(&mut self, other: &mut Clock) -> Result<&Version, OverlapError> {
        self.batch().sync(&mut other.batch())?;
        Ok(self.version())
    }

    /// Equivalent to `self.tick()`, but with a more illustrative name when
    /// versions are [`receive`](Version::receive)d.
    ///
    /// If you are using [`Clock`]s as *vector clock*s rather than *version
    /// vector*s, you should mark communication between [`Party`]s by
    /// [`send`](Clock::send)ing a [`Version`] from the sender to the recipient,
    /// who should dually [`receive`](Clock::receive) that [`Version`] to
    /// incorporate it into their own [`Clock`].
    pub fn send(&mut self) -> &Version {
        self.tick()
    }

    /// Merge a received [`Version`] into this [`Clock`]'s version, then
    /// [`tick`](Clock::tick) the [`Clock`].
    ///
    /// Equivalent to `self |= version; self.tick()`.
    ///
    /// If you are using [`Clock`]s as *vector clock*s rather than *version
    /// vector*s, you should mark communication between [`Party`]s by sending a
    /// [`Version`] from the sender to the recipient, who should dually
    /// [`receive`](Clock::receive) that [`Version`] to incorporate it into
    /// their own [`Clock`].
    pub fn receive(&mut self, version: &Version) -> &Version {
        self.batch().merge(version).tick();
        self.version()
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
        let mut bits =
            codec::Bits::with_capacity(self.party.as_bits().len() + self.version.as_bits().len());
        bits.extend_from_bitslice(self.party.as_bits());
        bits.extend_from_bitslice(self.version.as_bits());
        codec::pack_to_bytes(&bits)
    }

    /// Decode a byte string, strictly rejecting malformed or non-canonical input.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let bits = codec::bytes_as_bits(bytes);
        let after_id = codec::parse_id(bits, 0)?;
        let after_ev = codec::parse_ev(bits, after_id)?;
        codec::require_zero_padding(bits, after_ev)?;
        // The party begins at bit 0, so its slice is already byte-aligned. The
        // version begins at `after_id`, a generally non-byte-aligned offset:
        // `to_bitvec` on such a slice copies the backing region and *preserves*
        // the head bit-offset rather than shifting to bit 0, which would leave the
        // stored stream non-canonical and make `Version::encode` mis-pack it. Copy
        // the bits logically into a fresh, offset-0 stream to restore canonicity.
        let party_bits = bits[..after_id].to_bitvec();
        if codec::id_is_empty(&party_bits) {
            // A standalone `Clock` carries a nonzero share (paper §3: `event` requires
            // `i ≠ 0`); the anonymous id `0` is not a decodable top-level party.
            return Err(DecodeError::Anonymous);
        }
        let party = Party::from_bits(party_bits);
        let mut version_bits = codec::Bits::new();
        version_bits.extend_from_bitslice(&bits[after_id..after_ev]);
        let version = Version::from_bits(version_bits);
        Ok(Clock::from_parts(party, version))
    }
}

/// Format a [`Clock`] using the notation in the original paper: `(<id>,
/// <event>)`, e.g. `(1, 0)` for [`Clock::seed`].
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

/// Parse a stamp `(i, e)` in paper notation, strictly rejecting non-normal-form
/// input and any anonymous (id `0`) party.
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

/// A clock from a `(party, version)` literal, e.g. `((1, 0), 5).into()`.
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

/// A session over a [`Clock`], providing the same API, but with faster
/// performance for batches of operations.
pub struct Batch<'c> {
    party: &'c mut Party,
    version: version::Batch<'c>,
}

impl Batch<'_> {
    /// Like [`tick`](Clock::tick), but chainable.
    pub fn tick(&mut self) -> &mut Self {
        self.version.tick(&*self.party);
        self
    }

    /// Like `self |= version`, but chainable.
    pub fn merge(&mut self, version: &Version) -> &mut Self {
        self.version.merge(version);
        self
    }

    /// Like [`fork`](Clock::fork).
    pub fn fork(&mut self) -> Clock {
        let child_party = self.party.fork();
        let child_version = self.version.snapshot();
        Clock::from_parts(child_party, child_version)
    }

    /// Like [`join`](Clock::join).
    pub fn join(&mut self, other: Clock) -> Result<&version::Batch<'_>, Clock> {
        let (other_party, other_version) = other.into_parts();
        match self.party.join(other_party) {
            Ok(()) => {
                self.version.merge(&other_version);
                Ok(self.version())
            }
            Err(other_party) => Err(Clock::from_parts(other_party, other_version)),
        }
    }

    /// Like [`sync`](Clock::sync).
    pub fn sync(&mut self, other: &mut Batch<'_>) -> Result<&version::Batch<'_>, OverlapError> {
        // Merge both parties into self, then re-split: self keeps one half, other the
        // other. `join` is the overlap check — on failure it hands the party back and
        // leaves `self` unchanged, so we restore `other` and report the overlap.
        let theirs = core::mem::replace(other.party, Party::anonymous());
        if let Err(theirs) = self.party.join(theirs) {
            *other.party = theirs;
            return Err(OverlapError);
        }
        *other.party = self.party.fork();

        // Both histories become the join of the two.
        let other_version = other.version.snapshot();
        self.version.merge(&other_version);
        let merged = self.version.snapshot();
        self.version.replace_with(merged.clone());
        other.version.replace_with(merged);
        Ok(self.version())
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
