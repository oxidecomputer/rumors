//! A [`Clock`] is a [`Party`] paired with a [`Version`].
//!
//! A [`clock::Batch`](Batch) is a borrow of a `Clock` affording the same
//! interface but faster for bulk operations.

use core::ops::{BitOr, BitOrAssign};

use crate::{
    codec,
    error::{Decode, Overlap, Parse},
    version, Party, Version,
};

#[cfg(test)]
mod tests;

/// A [`Party`] and its [`Version`].
///
/// This type is `!Clone` to discourage non-linear usage: while using a
/// [`Clock`] non-linearly is "safe" from the perspective of Rust, it is invalid
/// in the setting of interval tree clocks, which requires that all live clocks
/// in the system **must** be disjoint.
///
/// Causal comparison and merge happen through the [`Version`]; `Clock` is not
/// itself ordered:
///
/// | Operation                                                                                                                           | Meaning                                                  |
/// |-------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------|
/// | `a.version()` (`<`, `<=`, `==`) `b.version()`                                                                                       | compare causal histories (the order lives on [`Version`])|
/// | [`a.version().concurrent(b.version())`](Version::concurrent)                                                                        | the two clocks' histories are incomparable               |
/// | `clock \| v`, `clock \|= v`                                                                                                         | join a received [`Version`] `v` into this clock          |
/// | [`tick`](Clock::tick)/[`fork`](Clock::fork)/[`join`](Clock::join)/[`sync`](Clock::sync)/[`send`](Clock::send)/[`recv`](Clock::recv) | advance, split, and reunite clocks                       |
///
/// There is deliberately no `Clock | Clock`: merging two whole clocks is the
/// fallible [`join`](Clock::join), which must verify the parties are disjoint.
///
/// ```
/// use before::Clock;
/// let mut a = Clock::seed();
/// let mut b = a.fork(); // two disjoint clocks
/// a.tick();
/// b.tick();
/// assert!(a.version().concurrent(b.version()));
/// ```
#[derive(PartialEq, Eq, Hash)]
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
    ///
    /// ```
    /// assert_eq!(before::Clock::seed().to_string(), "(1, 0)");
    /// ```
    pub fn seed() -> Self {
        Self::from_parts(Party::seed(), Version::new())
    }

    /// Advance this [`Clock`] by one event for its own [`Party`], returning the
    /// new [`Version`].
    ///
    /// ```
    /// let mut clock = before::Clock::seed();
    /// assert_eq!(clock.tick().to_string(), "1");
    /// ```
    pub fn tick(&mut self) -> &Version {
        self.batch().tick();
        self.version()
    }

    /// Split off a child clock by [`fork`](Party::fork)ing the underlying
    /// [`Party`] and copying the underlying [`Version`].
    ///
    /// # ⚠️ Warning
    ///
    /// Repeatedly calling [`fork`](Clock::fork) on the same [`Clock`] will lead
    /// to imbalanced internal tree representations and worse memory usage and
    /// performance; it's recommended to randomize which [`Clock`]s are
    /// [`fork`](Clock::fork)ed.
    ///
    /// ```
    /// use before::Clock;
    /// let mut parent = Clock::seed();
    /// let child = parent.fork();
    /// assert!(parent.party().is_disjoint(child.party()));
    /// ```
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
    ///
    /// ```
    /// use before::Clock;
    /// let mut a = Clock::seed();
    /// let b = a.fork();
    /// // `a` and `b` are disjoint halves, so they rejoin into the whole.
    /// a.join(b).unwrap();
    /// assert_eq!(a.party().to_string(), "1");
    /// ```
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
    ///
    /// ```
    /// use before::Clock;
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// a.tick();
    /// a.sync(&mut b).unwrap(); // both clocks learn each other's history
    /// assert_eq!(a.version(), b.version());
    /// ```
    pub fn sync(&mut self, other: &mut Clock) -> Result<&Version, Overlap> {
        self.batch().sync(&mut other.batch())?;
        Ok(self.version())
    }

    /// Equivalent to `self.tick()`, but with a more illustrative name when
    /// another party is to [`recv`](Clock::recv) the resultant new [`Version`].
    ///
    /// If you are using [`Clock`]s as *vector clock*s rather than *version
    /// vector*s, you should mark communication between [`Party`]s by
    /// [`send`](Clock::send)ing a [`Version`] from the sender to the recipient,
    /// who should dually [`recv`](Clock::recv) that [`Version`] to incorporate
    /// it into their own [`Clock`].
    ///
    /// ```
    /// let mut clock = before::Clock::seed();
    /// let msg = clock.send().clone(); // tick, then hand the version to a peer
    /// assert_eq!(msg.to_string(), "1");
    /// ```
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
    /// [`recv`](Clock::recv) that [`Version`] to incorporate it into their own
    /// [`Clock`].
    ///
    /// ```
    /// use before::Clock;
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let msg = a.send().clone();
    /// b.recv(&msg); // absorb a's history, then tick
    /// assert!(*b.version() > msg);
    /// ```
    pub fn recv(&mut self, version: &Version) -> &Version {
        self.batch().merge(version).tick();
        self.version()
    }

    /// Begin a batch of operations on this clock.
    ///
    /// Sequential operations within a batch are more efficient.
    ///
    /// ```
    /// use before::Clock;
    /// let mut clock = Clock::seed();
    /// clock.batch().tick().tick();
    /// assert_eq!(clock.version().to_string(), "2");
    /// ```
    pub fn batch(&mut self) -> Batch<'_> {
        let Clock { party, version } = self;
        Batch {
            party,
            version: version.batch(),
        }
    }

    /// Pair a [`Party`] with a [`Version`] to form a [`Clock`].
    ///
    /// ```
    /// use before::{Clock, Party, Version};
    /// let clock = Clock::from_parts(Party::seed(), Version::new());
    /// assert_eq!(clock.to_string(), "(1, 0)");
    /// ```
    pub fn from_parts(party: Party, version: Version) -> Self {
        Clock { party, version }
    }

    /// Decompose a [`Clock`] into its [`Party`] and [`Version`].
    ///
    /// ```
    /// use before::Clock;
    /// let (party, version) = Clock::seed().into_parts();
    /// assert_eq!(party.to_string(), "1");
    /// assert_eq!(version.to_string(), "0");
    /// ```
    pub fn into_parts(self) -> (Party, Version) {
        (self.party, self.version)
    }

    /// The [`Party`] whose causal history this clock tracks.
    ///
    /// ```
    /// assert_eq!(before::Clock::seed().party().to_string(), "1");
    /// ```
    pub fn party(&self) -> &Party {
        &self.party
    }

    /// Get the current state of the [`Clock`] as a [`Version`].
    ///
    /// ```
    /// assert_eq!(before::Clock::seed().version().to_string(), "0");
    /// ```
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// Encode a [`Clock`] as canonical bytes.
    ///
    /// ```
    /// use before::Clock;
    /// let bytes = Clock::seed().encode();
    /// assert_eq!(Clock::decode(&bytes[..]).unwrap().encode(), bytes);
    /// ```
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.encode_to(&mut bytes)
            .expect("writing to a Vec is infallible");
        bytes
    }

    /// Encode a [`Clock`]'s canonical bytes to an arbitrary writer.
    ///
    /// ```
    /// use before::Clock;
    /// let mut buf = Vec::new();
    /// Clock::seed().encode_to(&mut buf).unwrap();
    /// assert_eq!(buf, Clock::seed().encode());
    /// ```
    pub fn encode_to<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let mut writer = codec::BitWriter::new(writer);
        writer.write(self.party.as_bits())?;
        writer.write(self.version.as_bits())?;
        writer.finish()
    }

    /// Decode from a reader of canonical bytes, strictly rejecting malformed or
    /// non-canonical input.
    ///
    /// ```
    /// use before::Clock;
    /// let bytes = Clock::seed().encode();
    /// assert_eq!(Clock::decode(&bytes[..]).unwrap().to_string(), "(1, 0)");
    /// ```
    pub fn decode<R: std::io::Read>(mut reader: R) -> Result<Self, Decode> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).map_err(Decode::Io)?;
        // The party begins at bit 0 (byte-aligned); the version begins at
        // `after_id`, a generally non-byte-aligned offset, so it is copied
        // logically into a fresh offset-0 stream to restore canonicity (a
        // byte-offset copy would leave it non-canonical and mis-pack on
        // re-encode). The party then reuses the read buffer as its backing
        // store, so decoding allocates no more than before.
        let (after_id, version) = {
            let bits = codec::bytes_as_bits(&buf);
            let after_id = codec::parse_id(bits, 0)?;
            let after_ev = codec::parse_ev(bits, after_id)?;
            codec::require_zero_padding(bits, after_ev)?;
            if codec::id_is_empty(&bits[..after_id]) {
                // A standalone `Clock` carries a nonzero share (paper §3: `event`
                // requires `i ≠ 0`); the anonymous id `0` is not a decodable
                // top-level party.
                return Err(Decode::Anonymous);
            }
            let mut version_bits = codec::Bits::new();
            version_bits.extend_from_bitslice(&bits[after_id..after_ev]);
            (after_id, Version::from_bits(version_bits))
        };
        let mut party_bits = codec::Bits::from_vec(buf);
        party_bits.truncate(after_id);
        let party = Party::from_bits(party_bits);
        Ok(Clock::from_parts(party, version))
    }

    /// Count the number of bits in the encoding of this [`Clock`], not
    /// including padding to the nearest byte.
    ///
    /// ```
    /// use before::Clock;
    /// let clock = Clock::seed();
    /// assert_eq!(
    ///     clock.encoded_bits(),
    ///     clock.party().encoded_bits() + clock.version().encoded_bits(),
    /// );
    /// ```
    pub fn encoded_bits(&self) -> usize {
        self.party().encoded_bits() + self.version().encoded_bits()
    }
}

/// Format a [`Clock`] using the notation in the original paper: `(<id>,
/// <event>)`, e.g. `(1, 0)` for [`Clock::seed`].
///
/// ```
/// assert_eq!(before::Clock::seed().to_string(), "(1, 0)");
/// ```
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
///
/// ```
/// use before::Clock;
/// let clock: Clock = "(1, 0)".parse().unwrap();
/// assert_eq!(clock.to_string(), "(1, 0)");
/// ```
impl core::str::FromStr for Clock {
    type Err = Parse;
    fn from_str(s: &str) -> Result<Self, Parse> {
        let (id, ev) = codec::parse_clock_str(s)?;
        if codec::id_is_empty(&id) {
            return Err(Parse::Anonymous);
        }
        Ok(Clock::from_parts(
            Party::from_bits(id),
            Version::from_bits(ev),
        ))
    }
}

/// A clock from a `(party, version)` literal, e.g. `((1, 0), 5).into()`.
///
/// ```
/// use before::Clock;
/// let clock = Clock::try_from((1, 0)).unwrap();
/// assert_eq!(clock.to_string(), "(1, 0)");
/// ```
impl<I, E> TryFrom<(I, E)> for Clock
where
    Party: TryFrom<I, Error = Parse>,
    Version: TryFrom<E, Error = Parse>,
{
    type Error = Parse;
    fn try_from((i, e): (I, E)) -> Result<Self, Parse> {
        Ok(Clock::from_parts(
            Party::try_from(i)?,
            Version::try_from(e)?,
        ))
    }
}

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
    pub(crate) fn merge(&mut self, version: &Version) -> &mut Self {
        self.version.merge(version);
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
                self.version.merge(&other_version);
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
        self.version.merge(&other_version);
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

// Join operators. The `Clock` operand is consumed (a borrowing form would
// duplicate its party). No `Clock | Clock` — that is the fallible `Clock::join`.

/// Merge a [`Version`] into a [`Clock`], consuming the clock.
///
/// ```
/// use before::{Clock, Version};
/// let merged = Clock::seed() | "1".parse::<Version>().unwrap();
/// assert_eq!(merged.version().to_string(), "1");
/// ```
impl BitOr<Version> for Clock {
    type Output = Clock;
    fn bitor(mut self, r: Version) -> Clock {
        self.batch().merge(&r);
        self
    }
}

impl BitOr<&Version> for Clock {
    type Output = Clock;
    fn bitor(mut self, r: &Version) -> Clock {
        self.batch().merge(r);
        self
    }
}

/// Merge a [`Version`] into a [`Clock`], consuming the clock.
///
/// ```
/// use before::{Clock, Version};
/// let merged = "1".parse::<Version>().unwrap() | Clock::seed();
/// assert_eq!(merged.version().to_string(), "1");
/// ```
impl BitOr<Clock> for Version {
    type Output = Clock;
    fn bitor(self, mut r: Clock) -> Clock {
        r.batch().merge(&self);
        r
    }
}

impl BitOr<Clock> for &Version {
    type Output = Clock;
    fn bitor(self, mut r: Clock) -> Clock {
        r.batch().merge(self);
        r
    }
}

/// Merge a [`Version`] into a [`Clock`] in place.
///
/// ```
/// use before::{Clock, Version};
/// let mut clock = Clock::seed();
/// clock |= "1".parse::<Version>().unwrap();
/// assert_eq!(clock.version().to_string(), "1");
/// ```
impl BitOrAssign<Version> for Clock {
    fn bitor_assign(&mut self, r: Version) {
        self.batch().merge(&r);
    }
}

/// Merge a [`&Version`](Version) into a [`Clock`] in place.
///
/// ```
/// use before::{Clock, Version};
/// let mut clock = Clock::seed();
/// clock |= "1".parse::<Version>().unwrap();
/// assert_eq!(clock.version().to_string(), "1");
/// ```
impl BitOrAssign<&Version> for Clock {
    fn bitor_assign(&mut self, r: &Version) {
        self.batch().merge(r);
    }
}

/// Merge a [`Version`] into a clock [`Batch`] in place.
///
/// ```
/// use before::{Clock, Version};
/// let mut clock = Clock::seed();
/// {
///     let mut batch = clock.batch();
///     batch |= &"1".parse::<Version>().unwrap();
/// }
/// assert_eq!(clock.version().to_string(), "1");
/// ```
impl BitOrAssign<&Version> for Batch<'_> {
    fn bitor_assign(&mut self, r: &Version) {
        self.merge(r);
    }
}

impl BitOrAssign<Version> for Batch<'_> {
    fn bitor_assign(&mut self, r: Version) {
        self.merge(&r);
    }
}
