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
mod forks;

pub use batch::Batch;
pub use forks::Forks;

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
    /// tree, with worse memory use and performance. Prefer to vary which clock
    /// is forked, or use [`forks`](Clock::forks) to generate a fixed number of
    /// balanced forks.
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

    /// Split `n` balanced child clocks off this [`Clock`], as a lazy
    /// [`ExactSizeIterator`].
    ///
    /// The clock analogue of [`Party::forks`]: one balanced split of the
    /// underlying [`Party`] into `n + 1` shares of minimal-depth (`⌈log₂(n +
    /// 1)⌉`) id tree, each child carrying a clone of this clock's [`Version`]
    /// (as [`fork`](Clock::fork) does).
    ///
    /// The iterator yields `n` children and `self` keeps the last share, so it
    /// stays a valid clock even once the iterator is fully drained; children
    /// not taken before the iterator drops have their party shares rejoined
    /// into `self`, so no `Party` is lost. Prefer this to repeated
    /// [`fork`](Clock::fork), which deepens one spine into a linear tree.
    ///
    /// For the consuming counterpart that splits into exactly `N` clocks, see
    /// [`From<Clock>`](Clock) for `[Clock; N]`.
    ///
    /// ```
    /// use before::Clock;
    /// let mut parent = Clock::seed();
    /// let children: Vec<Clock> = parent.forks(3).collect();
    /// assert_eq!(children.len(), 3);
    /// for child in &children {
    ///     assert!(parent.party().is_disjoint(child.party()));
    ///     assert_eq!(child.version(), parent.version()); // every child copies the version
    /// }
    /// ```
    pub fn forks(&mut self, n: usize) -> Forks<'_> {
        Forks::new(self, n)
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

    /// Absorb every disjoint [`Clock`] in `iter` into `self`, returning the
    /// merged [`Version`].
    ///
    /// The collective form of [`join`](Clock::join): `self` seeds the fold, so
    /// an empty `iter` is a no-op returning `self`'s current version. The
    /// "reabsorb this whole set of retired peers" primitive.
    ///
    /// Best-effort: every clock whose [`Party`] is disjoint from the region
    /// accumulated so far has its party reunited and its [`Version`] merged into
    /// `self`.
    ///
    /// # Errors
    ///
    /// Returns the clocks whose parties *overlapped* `self`'s growing region and
    /// so could not be folded in, dropping nothing: each input is either merged
    /// into `self` or handed back. Overlap is tested against the running union,
    /// so for malformed (aliased) input which clocks come back can depend on
    /// iteration order. Unreachable for clocks descended from one
    /// [`seed`](Clock::seed): their parties are pairwise disjoint.
    ///
    /// ```
    /// use before::Clock;
    /// let mut parent = Clock::seed();
    /// let children: Vec<Clock> = parent.forks(3).collect();
    /// parent.join_all(children).unwrap(); // reabsorb the three children
    /// assert_eq!(parent.party().to_string(), "1"); // the whole seed region again
    /// ```
    pub fn join_all<I: IntoIterator<Item = Clock>>(
        &mut self,
        iter: I,
    ) -> Result<&Version, Vec<Clock>> {
        let mut overlapping = Vec::new();
        for other in iter {
            if let Err(back) = self.join(other) {
                overlapping.push(back);
            }
        }
        if overlapping.is_empty() {
            Ok(self.version())
        } else {
            Err(overlapping)
        }
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
        // The clock's bytes are the byte-aligned [`Party`] encoding followed by
        // the byte-aligned [`Version`] encoding. Each part is independently
        // canonical and the party is self-delimiting (a decoder parses its id to
        // find the split), so the two concatenate with no bit-level packing —
        // at the cost of at most one padding byte between them. Decoding then
        // reuses `Party::decode`/`Version::decode` on the two byte ranges.
        self.party.encode_to(writer)?;
        self.version.encode_to(writer)
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
        // The party is the byte-aligned prefix: parse its id to find its bit
        // length, round up to the byte boundary the version starts on, then
        // decode each part independently. `Party::decode` checks the party's
        // canonicity, padding, and the anonymous-id rejection (paper §3: a
        // standalone share is `i ≠ 0`); `Version::decode` checks the version.
        let id_bytes = {
            let bits = codec::bytes_as_bits(&buf);
            codec::parse_id(bits, 0)?.div_ceil(8)
        };
        let party = Party::decode(&buf[..id_bytes])?;
        let version = Version::decode(&buf[id_bytes..])?;
        Ok(Clock::from_parts(party, version))
    }

    /// Count the number of bits in the encoding of this [`Clock`], not including
    /// the final byte's padding.
    ///
    /// The encoding byte-concatenates the [`Party`] and [`Version`] (see
    /// [`encode`](Self::encode)), so the party occupies whole bytes and only the
    /// version's last byte is padded: this is the byte-aligned party length plus
    /// the version's own bit length.
    ///
    /// ```
    /// use before::Clock;
    /// let clock = Clock::seed();
    /// assert_eq!(clock.encode().len(), clock.encoded_bits().div_ceil(8));
    /// ```
    pub fn encoded_bits(&self) -> usize {
        8 * self.party().encoded_bits().div_ceil(8) + self.version().encoded_bits()
    }

    /// Duplicate this clock, producing a second handle to the same clock, in
    /// violation of linearity.
    ///
    /// # Warning
    ///
    /// [`Clock`] is [`!Clone`](Clone) because two live handles to one region
    /// break the Law of Disjointness: the alias's [`Party`] is not
    /// [disjoint](Party::is_disjoint) from the original, so if both copies (or
    /// any of their [`fork`](Clock::fork)s) go on to [`tick`](Clock::tick) or
    /// [`join`](Clock::join), causal history can be corrupted arbitrarily. The
    /// caller must ensure that at most one of the two copies is ever treated as
    /// live; the other must be dropped without further use. The same rule
    /// applies to any [`Party`] built from such a clock.
    ///
    /// This method exists for handing a clock across a boundary where ownership
    /// transfers to exactly one side based on an outcome not known at the time
    /// of transfer.
    ///
    /// ```
    /// use before::Clock;
    /// let c = Clock::seed();
    /// let d = c.dangerously_alias();
    /// assert!(!c.party().is_disjoint(d.party()));
    /// ```
    pub fn dangerously_alias(&self) -> Self {
        Self {
            party: self.party.dangerously_alias(),
            version: self.version.clone(),
        }
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

/// Generates the `Clock` join matrix.
///
/// A `|` cell owns its clock operand (whichever side it is on) and returns it;
/// a `|=` cell merges into the receiver in place. Each position — `op_l`/`op_r`
/// for the clock as the left or right `|` operand, `as_clock`/`as_batch` for
/// the `|=` receiver — has its own `@cell` arm so the receiver `self` is
/// written in the same expansion as the method it belongs to (`self` cannot
/// cross a macro-invocation boundary).
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
