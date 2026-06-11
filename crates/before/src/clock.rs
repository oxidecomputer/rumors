//! A [`Clock`] is a [`Party`] paired with a [`Version`].
//!
//! A [`clock::Batch`](Batch) is a borrow of a `Clock` affording the same
//! interface but faster for bulk operations.

use core::borrow::Borrow;
use core::ops::{BitOr, BitOrAssign};

use crate::{
    codec,
    error::{Decode, Overlap, Parse},
    Party, Version,
};

mod batch;

pub use batch::Batch;

#[cfg(test)]
mod tests;

/// A [`Party`] and its [`Version`].
///
/// This type is `!Clone` to discourage non-linear usage: duplicating a
/// [`Clock`] is memory-safe but invalid for interval tree clocks, which
/// require all live clocks in a system to be disjoint.
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
    /// [`Clock`] not derived from some prior clock.
    ///
    /// Call this function once per system of clocks. Every descendant of a
    /// single seed is disjoint from its peers, but descendants of two
    /// independent seeds need not be; if they ever interact, causal history
    /// is silently corrupted.
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
    /// # Warning
    ///
    /// Repeatedly forking the same [`Clock`] produces an imbalanced internal
    /// tree, with worse memory use and performance. Prefer to vary which
    /// clock is forked.
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
    /// If the two clocks' [`Party`]s overlap, `self` is unmodified and
    /// `other` is handed back in the error.
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

    /// Equivalent to [`tick`](Clock::tick), named for the case where another
    /// party will [`recv`](Clock::recv) the resulting [`Version`].
    ///
    /// When using [`Clock`]s as *vector clocks* rather than *version
    /// vectors*, mark communication by `send`ing a [`Version`] from the
    /// sender to the recipient, who [`recv`](Clock::recv)s it into their own
    /// [`Clock`].
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
    /// Equivalent to `self |= version; self.tick()`. The receiving half of
    /// the vector-clock communication pattern described on
    /// [`send`](Clock::send).
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
        self.batch().join_version(version).tick();
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
        Batch::new(self)
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

    /// Get the *slice* of the [`Version`] of the [`Clock`] *which is owned by
    /// its own [`Party`].
    ///
    /// This is short for `self.version() / self.party()`.
    ///
    /// ```
    /// use before::{Clock, Version};
    /// let mut a = Clock::seed();
    /// a.tick();
    /// let mut b = a.fork();
    /// a.tick();
    /// b.tick();
    /// // The meet (greatest lower bound) of the two versions is more than
    /// // the initial version:
    /// assert!(a.version() & b.version() > Version::new());
    /// // But the meet of the two projected versions is not:
    /// assert!(a.own_version() & b.own_version() == Version::new());
    /// ```
    pub fn own_version(&self) -> Version {
        self.version() / self.party()
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

// The join operators for `Clock` over {Clock, Version}: `|` merges a
// `Version` into a clock (on either side, since a `Version` carries no
// party) and returns the clock; `|=` merges in place. There is no
// `Clock | Clock`: a borrowing form would duplicate the clock's party, and
// reuniting two whole clocks is the fallible `Clock::join`. Every cell folds
// the version operand into the clock's `version` batch through
// `Batch::join_version`; `Borrow::borrow` coerces an owned or borrowed
// operand uniformly to `&Version`, so one `@cell` arm per position covers
// both forms.

/// Generates the `Clock` join matrix. A `|` cell owns its clock operand
/// (whichever side it is on) and returns it; a `|=` cell merges into the
/// receiver in place. Each position — `op_l`/`op_r` for the clock as the left or
/// right `|` operand, `as_clock`/`as_batch` for the `|=` receiver — has its own
/// `@cell` arm so the receiver `self` is written in the same expansion as the
/// method it belongs to (`self` cannot cross a macro-invocation boundary).
macro_rules! clock_join_matrix {
    ($($kind:tt $lhs:ty, $rhs:ty);* $(;)?) => {
        $( clock_join_matrix!(@cell $kind $lhs, $rhs); )*
    };
    (@cell op_l $lhs:ty, $rhs:ty) => {
        impl BitOr<$rhs> for $lhs {
            type Output = Clock;
            fn bitor(mut self, r: $rhs) -> Clock {
                self.batch().join_version(r.borrow());
                self
            }
        }
    };
    (@cell op_r $lhs:ty, $rhs:ty) => {
        impl BitOr<$rhs> for $lhs {
            type Output = Clock;
            fn bitor(self, mut r: $rhs) -> Clock {
                r.batch().join_version(self.borrow());
                r
            }
        }
    };
    (@cell as_clock $lhs:ty, $rhs:ty) => {
        impl BitOrAssign<$rhs> for $lhs {
            fn bitor_assign(&mut self, r: $rhs) {
                self.batch().join_version(r.borrow());
            }
        }
    };
    (@cell as_batch $lhs:ty, $rhs:ty) => {
        impl BitOrAssign<$rhs> for $lhs {
            fn bitor_assign(&mut self, r: $rhs) {
                self.join_version(r.borrow());
            }
        }
    };
}

clock_join_matrix! {
    op_l     Clock,     Version;
    op_l     Clock,     &Version;
    op_r     Version,   Clock;
    op_r     &Version,  Clock;
    as_clock Clock,     Version;
    as_clock Clock,     &Version;
    as_batch Batch<'_>, Version;
    as_batch Batch<'_>, &Version;
}
