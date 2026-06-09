//! Disjoint parties who can emit events.

use core::fmt::Display;

use bitvec::prelude::*;

use crate::codec::{self, BitsSlice};
use crate::error::{Decode, Parse};
use crate::idbits::IdReader;
use crate::Version;

mod ops;

#[cfg(test)]
mod tests;

/// A disjoint party: a share of the unit interval `[0, 1)`, identified by its
/// place in the fork tree.
///
/// A party is primarily manipulated by these operations:
///
/// | Operation                                 | Meaning                                                                   |
/// |-------------------------------------------|---------------------------------------------------------------------------|
/// | [`a.tick(v)`](Party::tick)                | advance the [`Version`] for this [`Party`]                                |
/// | [`a.fork()`](Party::fork)                 | split `a` into two disjoint children                                      |
/// | [`a.join(b)`](Party::join)                | reunite two *disjoint* parties into the one owning both regions; fallible |
/// | [`a.is_disjoint(&b)`](Party::is_disjoint) | whether `a` and `b` share no region, hence may safely interact            |
/// | `a == b`                                  | whether `a` is exactly the same [`Party`] as `b`                          |
///
/// A [`Party`] is **not ordered**. Use [`is_disjoint`](Party::is_disjoint) to
/// tell whether two parties may [`join`](Party::join). There is likewise no
/// `Party | Party`: reuniting is the fallible [`join`](Party::join), which
/// internally verifies disjointness.
///
/// Like [`Clock`](crate::Clock), [`Party`] is [`!Clone`](Clone): duplicating a
/// live party would violate the linearity which interval tree clocks require.
///
/// ```
/// use before::Party;
/// let mut whole = Party::seed();
/// let half = whole.fork();
/// assert!(whole.is_disjoint(&half)); // the two halves share no region
/// whole.join(half).unwrap();         // ... and reunite into the whole
/// assert_eq!(whole.to_string(), "1");
/// ```
#[derive(PartialEq, Eq, Hash)]
pub struct Party(BitVec<u8, Msb0>);

impl Party {
    /// The initial [`Party`] in the system.
    ///
    /// In any given system of [`Party`]s, this function (or
    /// [`Clock::seed`](crate::Clock::seed), which invokes it) should only be
    /// called by one party in the entire system, and only once: all its
    /// descendents are necessarily disjoint, but the descendents of parallel
    /// seeds need not be; if ever the twain meet, invariants and expectations
    /// will be violated.
    ///
    /// ```
    /// assert_eq!(before::Party::seed().to_string(), "1");
    /// ```
    pub fn seed() -> Self {
        let mut bits = codec::Bits::with_capacity(2);
        bits.push(false); // leaf flag
        bits.push(true); // value 1
        Party(bits)
    }

    /// Advance the [`Version`] from the perspective of [`Party`].
    ///
    /// ```
    /// use before::{Party, Version};
    /// let mut v = Version::new();
    /// Party::seed().tick(&mut v);
    /// assert_eq!(v.to_string(), "1");
    /// ```
    pub fn tick(&self, version: &mut Version) {
        version.tick(self)
    }

    /// Split off a new disjoint [`Party`] from this one.
    ///
    /// # ⚠️ Warning
    ///
    /// Repeatedly calling [`fork`](Party::fork) on solely the same [`Party`]
    /// will lead to imbalanced internal tree representations and worse memory
    /// usage and performance; it's recommended to randomize which [`Party`]s
    /// are [`fork`](Party::fork)ed.
    ///
    /// ```
    /// use before::Party;
    /// let mut p = Party::seed();
    /// let q = p.fork();
    /// assert_eq!(p.to_string(), "(1, 0)");
    /// assert_eq!(q.to_string(), "(0, 1)");
    /// ```
    pub fn fork(&mut self) -> Party {
        let (keep, give) = self.view().split();
        self.0 = keep;
        Party(give)
    }

    /// Reunite two disjoint [`Party`]s.
    ///
    /// # Errors
    ///
    /// If the parties are not disjoint, `self` is unmodified, and `Err(other)`
    /// is returned.
    ///
    /// ```
    /// use before::Party;
    /// let mut p = Party::seed();
    /// let q = p.fork();
    /// p.join(q).unwrap(); // the two halves reunite into the whole
    /// assert_eq!(p.to_string(), "1");
    /// ```
    pub fn join(&mut self, other: Party) -> Result<(), Party> {
        match self.view().sum(other.view()) {
            Some(bits) => {
                self.0 = bits;
                Ok(())
            }
            None => Err(other),
        }
    }

    /// Test whether `self` and `other` are *disjoint* (i.e. descend from linear
    /// [`fork`](Party::fork)-[`join`](Party::join) operations starting from a
    /// singular [`seed`](Party::seed)).
    ///
    /// Disjoint [`Party`]s may always be [`join`](Party::join)ed without error.
    ///
    /// ```
    /// use before::Party;
    /// let mut p = Party::seed();
    /// let q = p.fork();
    /// assert!(p.is_disjoint(&q));
    /// ```
    pub fn is_disjoint(&self, other: &Party) -> bool {
        self.view().is_disjoint(other.view())
    }

    /// Test whether `self`'s owned id-region *contains* all of `other`'s — i.e.
    /// `self ⊇ other`, every region `other` owns is also owned by `self`.
    ///
    /// The asymmetric companion of [`is_disjoint`](Party::is_disjoint): two
    /// [`Party`]s are either disjoint (share nothing), or one covers the other
    /// (their regions are nested), or — for arbitrary unrelated ids — neither
    /// (they partially overlap). For any two [`Party`]s descended from the same
    /// [`seed`](Party::seed) via [`fork`](Party::fork)/[`join`](Party::join),
    /// the partial-overlap case cannot arise, so covering is exactly the
    /// negation of disjointness once equal regions are set aside.
    ///
    /// Covering is reflexive and transitive (a partial order on regions), with
    /// the whole [`seed`](Party::seed) on top:
    ///
    /// - `seed` covers every [`Party`];
    /// - a [`Party`] covers itself (and any [`dangerously_alias`] of it);
    /// - the parent of a [`fork`](Party::fork) covers both resulting halves,
    ///   and a [`join`](Party::join) covers each of its parts.
    ///
    /// Covering a *non-empty* region implies the two are **not**
    /// [disjoint](Party::is_disjoint): a reclaiming party that has come to
    /// cover another's region can therefore no longer [`join`](Party::join) it
    /// (the region is already held), which is exactly how a caller recognizes a
    /// once-outstanding share as fully reabsorbed.
    ///
    /// [`dangerously_alias`]: Party::dangerously_alias
    ///
    /// ```
    /// use before::Party;
    /// let mut p = Party::seed();
    /// let q = p.fork();
    /// assert!(Party::seed().covers(&p)); // the whole covers a part
    /// assert!(p.covers(&p.dangerously_alias())); // a region covers itself
    /// assert!(!p.covers(&q)); // disjoint halves cover neither other
    /// assert!(!q.covers(&p));
    /// p.join(q).unwrap();
    /// assert!(p.covers(&Party::seed())); // rejoined to the whole again
    /// ```
    pub fn covers(&self, other: &Party) -> bool {
        self.view().covers(other.view())
    }

    /// Carve `other`'s region out of `self`, yielding the share of `self` that
    /// `other` does **not** own — the region difference `self \ other`.
    ///
    /// Returns `None` exactly when `other` [`covers`](Party::covers) `self`, so
    /// nothing remains: a [`Party`] is a *nonzero* share, and the empty region
    /// is not a [`Party`]. Otherwise returns `Some` of the remainder, which is
    /// always a subregion of `self` (`self \ other ⊆ self`).
    ///
    /// This is a partial inverse of [`join`](Party::join): where `join` folds a
    /// disjoint share *in*, `without` cuts a share back *out*. It consumes
    /// `self` by value and reads `other` only as a mask, shrinking `self`,
    /// which means that it does not introduce any more non-linearity than
    /// already exists.
    ///
    /// ```
    /// use before::Party;
    /// let mut p = Party::seed();
    /// let q = p.fork(); // p and q are disjoint halves of the seed
    ///
    /// // Removing a disjoint share leaves `self` untouched.
    /// let keep = p.dangerously_alias();
    /// assert_eq!(p.without(&q).unwrap().to_string(), keep.to_string());
    ///
    /// // Removing a covering share (here, itself) leaves nothing.
    /// assert!(Party::seed().without(&Party::seed()).is_none());
    /// ```
    pub fn without(self, other: &Party) -> Option<Party> {
        let bits = self.view().diff(other.view());
        if codec::id_is_empty(&bits) {
            None
        } else {
            Some(Party::from_bits(bits))
        }
    }

    /// Dangerously duplicate this party, violating linearity to produce a
    /// second handle to the **same** party identity.
    ///
    /// # ⚠️ You probably don't want to call this method because it can **corrupt
    /// causal history**
    ///
    /// [`Party`] is [`!Clone`](Clone) precisely because two live handles to one
    /// region break the Law of Disjointness upon which interval tree clocks
    /// rely: the copy produced by this method is *not*
    /// [disjoint](Party::is_disjoint), so if the original and the alias (or any
    /// of their [`fork`](Party::fork)s) both [`join`](Party::join) or
    /// [`tick`](Party::tick), it can corrupt causal history arbitrarily.
    ///
    /// Because of this, duplicating a [`Party`] is almost always a footgun;
    /// this method invites you to shoot yourself in the foot. The caller is
    /// **solely responsible** for ensuring at most one copy of a [`Party`] is
    /// ever treated as "live"; the other must be dropped without ever being
    /// used again. It is only causality-safe to call this method if you can
    /// ensure that *at most one* of the copies (or any of its
    /// [`fork`](Party::fork)s) will *ever* call [`tick`](Party::tick) again.
    ///
    /// Keep in mind that a [`Clock`](crate::Clock) is merely the convenient
    /// pairing of a [`Party`] and a [`Version`], so all these warnings apply
    /// equally to [`Clock`](crate::Clock)s constructed from such a [`Party`]:
    /// *at most one* of such a [`Clock`](crate::Clock) (or any of its
    /// [`fork`](crate::Clock::fork)s) must *ever* call
    /// [`tick`](crate::Clock::tick) again.
    ///
    /// ## When might you want to do this?
    ///
    /// You might reach for this when handing a party across a boundary where
    /// ownership transfers to exactly one side based on a subsequent
    /// determination not known at the time of transfer.
    ///
    /// ```
    /// use before::Party;
    /// let p = Party::seed();
    /// let q = p.dangerously_alias();
    /// assert!(!p.is_disjoint(&q));
    /// ```
    pub fn dangerously_alias(&self) -> Self {
        Party(self.0.clone())
    }

    /// Encode a [`Party`] to bytes.
    ///
    /// **Note:** The byte-encoding of a [`Clock`](crate::Clock) is **not the
    /// same** as the concatenation of the byte-encoding of a [`Party`] and a
    /// [`Version`](crate::Version).
    ///
    /// ```
    /// use before::Party;
    /// let p = Party::seed();
    /// assert_eq!(Party::decode(&p.encode()[..]).unwrap(), p);
    /// ```
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.encode_to(&mut bytes)
            .expect("writing to a Vec is infallible");
        bytes
    }

    /// Encode a [`Party`] to an arbitrary writer.
    ///
    /// ```
    /// use before::Party;
    /// let mut buf = Vec::new();
    /// Party::seed().encode_to(&mut buf).unwrap();
    /// assert_eq!(buf, Party::seed().encode());
    /// ```
    pub fn encode_to<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        codec::pack_to_writer(&self.0, writer)
    }

    /// The exact length in bits of [`encode`](Self::encode) before its zero-pad
    /// to a byte boundary.
    ///
    /// ```
    /// // The seed is a single `1` leaf: a flag bit plus a value bit.
    /// assert_eq!(before::Party::seed().encoded_bits(), 2);
    /// ```
    pub fn encoded_bits(&self) -> usize {
        self.as_bits().len()
    }

    /// Decode a [`Party`] from a reader of canonical bytes, strictly rejecting
    /// non-canonical representations.
    ///
    /// ```
    /// use before::Party;
    /// let bytes = Party::seed().encode();
    /// assert_eq!(Party::decode(&bytes[..]).unwrap(), Party::seed());
    /// ```
    pub fn decode<R: std::io::Read>(mut reader: R) -> Result<Self, Decode> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).map_err(Decode::Io)?;
        let end = {
            let bits = codec::bytes_as_bits(&buf);
            let end = codec::parse_id(bits, 0)?;
            codec::require_zero_padding(bits, end)?;
            end
        };
        // Reuse the read buffer as the result's backing store (it is offset-0
        // and canonical up to `end`), so decoding allocates no more than before.
        let mut id = codec::Bits::from_vec(buf);
        id.truncate(end);
        if codec::id_is_empty(&id) {
            return Err(Decode::Anonymous);
        }
        Ok(Party(id))
    }

    /// The anonymous (zero) id, `Leaf(false)`. Internal and transient only
    /// (i.e. for use in `mem::swap`) and *never* a publicly constructible value
    /// (a `Party` is a nonzero share).
    ///
    /// Used as a placeholder when moving a party out of a `&mut` during `sync`,
    /// immediately overwritten by the re-split half.
    pub(crate) fn anonymous() -> Party {
        let mut bits = codec::Bits::with_capacity(2);
        bits.push(false); // leaf flag
        bits.push(false); // value 0
        Party(bits)
    }

    /// A read-only [`IdReader`] cursor at the root of this party's packed id bits.
    fn view(&self) -> IdReader<'_> {
        IdReader::root(&self.0)
    }

    /// The canonical packed bytes of this [`Party`]: exactly what
    /// [`encode`](Self::encode) produces, but borrowed without copying. The
    /// final partial byte is zero-padded (an invariant of the stored form), so
    /// these bytes are a *canonical* identity — byte-equal if and only if the
    /// [`Party`]s are equal, and stable to [`hash`](core::hash::Hash).
    ///
    /// A [`Party`] is **not ordered** (see the type docs); the lexicographic
    /// order of these bytes is an arbitrary total order with no semantic
    /// meaning, useful only as a deterministic tiebreak. Use
    /// [`is_disjoint`](Self::is_disjoint) to reason about whether two parties
    /// may interact.
    ///
    /// ```
    /// use before::Party;
    /// let p = Party::seed();
    /// assert_eq!(p.as_bytes(), p.encode().as_slice());
    /// ```
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_raw_slice()
    }

    /// The packed preorder bit stream (no trailing padding). Internal.
    pub(crate) fn as_bits(&self) -> &BitsSlice {
        &self.0
    }

    /// Wrap a canonical packed bit stream. Internal; callers guarantee normal form.
    pub(crate) fn from_bits(bits: codec::Bits) -> Self {
        Party(bits)
    }
}

/// Paper notation: `0` / `1` leaves, `(l, r)` nodes. E.g. `(1, (0, 1))`.
///
/// ```
/// use before::Party;
/// let p: Party = "(1, (0, 1))".parse().unwrap();
/// assert_eq!(p.to_string(), "(1, (0, 1))");
/// ```
impl core::fmt::Display for Party {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        codec::write_id(&self.0, f, ", ")
    }
}

/// Same as `Display`.
impl core::fmt::Debug for Party {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <Self as Display>::fmt(self, f)
    }
}

/// Parse paper notation (`0 | 1 | (i1, i2)`), strictly rejecting non-normal-form input
/// and the anonymous identity `0` (a standalone `Party` must be a nonzero share).
///
/// ```
/// use before::Party;
/// let p: Party = "(1, 0)".parse().unwrap();
/// assert_eq!(p.to_string(), "(1, 0)");
/// assert!("0".parse::<Party>().is_err()); // the anonymous identity is rejected
/// ```
impl core::str::FromStr for Party {
    type Err = Parse;
    fn from_str(s: &str) -> Result<Self, Parse> {
        finish_id(codec::parse_id_str(s)?)
    }
}

/// Wrap validated id bits as a `Party`, rejecting the anonymous (empty) identity. The
/// single gate through which every parsed/built top-level `Party` passes.
fn finish_id(bits: codec::Bits) -> Result<Party, Parse> {
    if codec::id_is_empty(&bits) {
        Err(Parse::Anonymous)
    } else {
        Ok(Party::from_bits(bits))
    }
}

/// An id literal that can ground out a [`Party`] tuple: the `u8` leaves `0`/`1` and
/// nested `(left, right)` tuples. Sealed and hidden — an implementation detail enabling
/// `Party::try_from(..)` literals. Unlike the public `TryFrom`, an `IdLit` leaf of `0`
/// is allowed (it is a valid *sub-tree*); the anonymous check happens only once the
/// whole id is assembled (see [`finish_id`]).
mod sealed {
    pub trait Sealed {}
    impl Sealed for u8 {}
    impl Sealed for bool {}
    impl<T, S> Sealed for (T, S) {}
}

#[doc(hidden)]
pub trait PartyLiteral: sealed::Sealed {
    #[doc(hidden)]
    fn into_id_bits(self) -> Result<codec::Bits, Parse>;
}

impl PartyLiteral for u8 {
    fn into_id_bits(self) -> Result<codec::Bits, Parse> {
        match self {
            0 => Ok(codec::id_leaf(false)),
            1 => Ok(codec::id_leaf(true)),
            _ => Err(Parse::Syntax),
        }
    }
}

impl PartyLiteral for bool {
    fn into_id_bits(self) -> Result<codec::Bits, Parse> {
        Ok(codec::id_leaf(self))
    }
}

impl<T: PartyLiteral, S: PartyLiteral> PartyLiteral for (T, S) {
    fn into_id_bits(self) -> Result<codec::Bits, Parse> {
        let l = self.0.into_id_bits()?;
        let r = self.1.into_id_bits()?;
        codec::id_node(&l, &r) // assembles + validates normal form
    }
}

/// An id leaf from a single bit: `1` (full) is a valid `Party`; `0` is the anonymous
/// identity and is rejected here, though it is allowed as a sub-tree in the tuple form.
///
/// ```
/// use before::Party;
/// assert_eq!(Party::try_from(1).unwrap().to_string(), "1");
/// assert!(Party::try_from(0).is_err());
/// ```
impl TryFrom<u8> for Party {
    type Error = Parse;
    fn try_from(v: u8) -> Result<Self, Parse> {
        finish_id(v.into_id_bits()?)
    }
}

/// An id leaf from a single boolean: `true` = `1`, `false` = `0`.
///
/// ```
/// use before::Party;
/// assert_eq!(Party::try_from(true).unwrap().to_string(), "1");
/// assert!(Party::try_from(false).is_err()); // `0` is anonymous
/// ```
impl TryFrom<bool> for Party {
    type Error = Parse;
    fn try_from(v: bool) -> Result<Self, Parse> {
        finish_id(v.into_id_bits()?)
    }
}

/// An id node from a `(left, right)` literal, e.g. `Party::try_from((1u8, (0u8, 1u8)))`.
/// Rejects a collapsible `(v, v)` (non-canonical) and an all-`0` (anonymous) result.
///
/// ```
/// use before::Party;
/// let p = Party::try_from((1, (0, 1))).unwrap();
/// assert_eq!(p.to_string(), "(1, (0, 1))");
/// ```
impl<T: PartyLiteral, S: PartyLiteral> TryFrom<(T, S)> for Party {
    type Error = Parse;
    fn try_from(t: (T, S)) -> Result<Self, Parse> {
        finish_id(t.into_id_bits()?)
    }
}
