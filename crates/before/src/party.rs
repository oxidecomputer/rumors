//! Disjoint parties who can emit events.
//!
//! A [`Party`] is a non-empty set of subintervals of `[0, 1)`, stored as a
//! canonical id-tree: the share of the identifier space its holder may
//! [`tick`](Party::tick) against. [`fork`](Party::fork) splits a share in two;
//! [`join`](Party::join) reunites disjoint shares and refuses overlapping ones,
//! because everything ITCs guarantee rests on the Law of Disjointness (see the
//! [crate docs](crate)' safety rules). Parties are deliberately `!Clone` and
//! their operations consume `self`: linearity in the type system, leaving only
//! serialization boundaries to the caller.

use core::fmt::Display;

use bitvec::prelude::*;

use crate::codec::{self, BitsSlice};
use crate::error::{Decode, Parse};
use crate::idbits::IdReader;
use crate::Version;

mod forks;
mod ops;

pub use forks::Forks;

#[cfg(test)]
mod tests;

/// A causal party: a disjoint share of the unit interval `[0, 1)`.
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
/// A [`Party`] is not ordered. Use [`is_disjoint`](Party::is_disjoint) to tell
/// whether two parties may [`join`](Party::join). There is likewise no `Party |
/// Party`: reuniting is the fallible [`join`](Party::join), which verifies
/// disjointness itself.
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
    /// Call this function (or [`Clock::seed`](crate::Clock::seed), which
    /// invokes it) once per system of parties. Every descendant of a single
    /// seed is disjoint from its peers, but descendants of two independent
    /// seeds need not be; if they ever interact, causal history is silently
    /// corrupted.
    ///
    /// ```
    /// assert_eq!(before::Party::seed().to_string(), "1");
    /// ```
    pub fn seed() -> Self {
        let mut bits = codec::Bits::with_capacity(2);
        bits.push(false); // terminal tag `00`: the whole interval, owned
        bits.push(false);
        Party::from_bits(bits)
    }

    /// Whether this party is the whole, undivided seed region: equal to
    /// [`Party::seed`].
    ///
    /// True only before any [`fork`](Party::fork) has split a region away, and
    /// again once every fork has been [`join`](Party::join)ed back. A
    /// bootstrapped descendant, holding a forked sub-region, is never the seed.
    ///
    /// ```
    /// use before::Party;
    /// let mut p = Party::seed();
    /// assert!(p.is_seed());
    /// let q = p.fork();
    /// assert!(!p.is_seed()); // a party that has forked no longer owns the whole
    /// assert!(!q.is_seed());
    /// p.join(q).unwrap();
    /// assert!(p.is_seed()); // ... until the whole is reunited
    /// ```
    pub fn is_seed(&self) -> bool {
        *self == Party::seed()
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
    /// # Warning
    ///
    /// Repeatedly forking the same [`Party`] produces an imbalanced internal
    /// tree, with worse memory use and performance. Prefer to vary which party
    /// is forked, or use [`forks`](Party::forks) to generate a fixed number of
    /// balanced forks.
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
        *self = Party::from_bits(keep);
        Party::from_bits(give)
    }

    /// Split `n` balanced shares off this [`Party`], as a lazy
    /// [`ExactSizeIterator`].
    ///
    /// A single balanced split: the region is divided into `n + 1` subregions
    /// whose id tree has minimal depth `⌈log₂(n + 1)⌉`. The iterator hands out
    /// `n` of them and `self` keeps the last, so unlike repeatedly calling
    /// [`fork`](Party::fork), which deepens one spine into a linear tree (see
    /// its warning), every share here stays shallow.
    ///
    /// A [`Party`] is never empty, so `self` retains its residual share even
    /// once the iterator is fully drained; shares not taken before the iterator
    /// drops are [`join`](Party::join)ed back into `self`. The handed-out
    /// shares together with `self` reconstruct the original region.
    ///
    /// For the consuming counterpart that splits into exactly `N` shares with no
    /// residual, see [`From<Party>`](Party) for `[Party; N]`.
    ///
    /// ```
    /// use before::Party;
    /// let mut p = Party::seed();
    /// let shares: Vec<Party> = p.forks(3).collect();
    /// assert_eq!(shares.len(), 3); // three shares handed out...
    /// for s in &shares {
    ///     assert!(p.is_disjoint(s)); // ...each disjoint from the keeper
    /// }
    /// // `self` kept the fourth; rejoining all four recovers the whole seed.
    /// p.join_all(shares).unwrap();
    /// assert!(p.is_seed());
    /// ```
    pub fn forks(&mut self, n: usize) -> Forks<'_> {
        Forks::new(self, n)
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
                *self = Party::from_bits(bits);
                Ok(())
            }
            None => Err(other),
        }
    }

    /// Reunite every disjoint [`Party`] in `iter` into `self`: the fold of the
    /// partial commutative monoid that [`join`](Party::join) generates.
    ///
    /// Total where a free function could not be — `self` seeds the fold, so an
    /// empty `iter` simply leaves `self` unchanged. (Contrast
    /// [`Version::join_all`](crate::Version::join_all), an associated function
    /// with the empty version for its identity; the [`Party`] monoid has none,
    /// since the empty region is not a party.) The natural "retire this whole
    /// set of peers" primitive.
    ///
    /// Best-effort: every party [disjoint](Party::is_disjoint) from the region
    /// accumulated so far is folded in, so `self` ends owning its original
    /// region plus all of them.
    ///
    /// # Errors
    ///
    /// Returns the parties that *overlapped* — those that intersect `self`'s
    /// growing region and so cannot be folded in — and drops nothing: each input
    /// is either joined into `self` or handed back. Overlap is tested against
    /// the running union, so for a malformed (aliased) input which parties come
    /// back can depend on iteration order. For parties descended from one
    /// [`seed`](Party::seed) the error is unreachable — they are pairwise
    /// disjoint — and the returned `Vec` is then never allocated.
    ///
    /// ```
    /// use before::Party;
    /// let mut p = Party::seed();
    /// let shares: Vec<Party> = p.forks(3).collect();
    /// p.join_all(shares).unwrap(); // the residual and three shares reunite
    /// assert!(p.is_seed());
    /// ```
    pub fn join_all<I: IntoIterator<Item = Party>>(&mut self, iter: I) -> Result<(), Vec<Party>> {
        let mut overlapping = Vec::new();
        for other in iter {
            if let Err(back) = self.join(other) {
                overlapping.push(back);
            }
        }
        if overlapping.is_empty() {
            Ok(())
        } else {
            Err(overlapping)
        }
    }

    /// Test whether `self` and `other` are *disjoint*: their owned regions
    /// share nothing.
    ///
    /// All live descendants of a single
    /// [`seed`](Party::seed), evolved by linear [`fork`](Party::fork) and
    /// [`join`](Party::join), are pairwise disjoint.
    ///
    /// Disjoint [`Party`]s may always be [`join`](Party::join)ed without
    /// error.
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

    /// Test whether `self`'s owned region contains all of `other`'s
    /// (`self ⊇ other`).
    ///
    /// This is the asymmetric companion of [`is_disjoint`](Party::is_disjoint).
    /// Two arbitrary parties are disjoint (they share nothing), nested (one
    /// covers the other), or partially overlapping (neither covers the other).
    /// For parties descended from one [`seed`](Party::seed) via
    /// [`fork`](Party::fork) and [`join`](Party::join), partial overlap cannot
    /// arise.
    ///
    /// Covering is reflexive and transitive, a partial order on regions with
    /// the whole [`seed`](Party::seed) on top:
    ///
    /// - `seed` covers every [`Party`];
    /// - a [`Party`] covers itself;
    /// - the parent of a [`fork`](Party::fork) covers both halves, and a
    ///   [`join`](Party::join) covers each of its parts.
    ///
    /// Covering a non-empty region implies the two are not
    /// [disjoint](Party::is_disjoint), so a party that has come to cover
    /// another's region can no longer [`join`](Party::join) it. This is how a
    /// caller recognizes an outstanding share as fully reabsorbed.
    ///
    /// ```
    /// use before::Party;
    /// let mut p = Party::seed();
    /// let q = p.fork();
    /// assert!(Party::seed().covers(&p)); // the whole covers a part
    /// assert!(p.covers(&p.dangerously_alias())); // a region covers itself
    /// assert!(!p.covers(&q)); // neither disjoint half covers the other
    /// assert!(!q.covers(&p));
    /// p.join(q).unwrap();
    /// assert!(p.covers(&Party::seed())); // rejoined to the whole again
    /// ```
    pub fn covers(&self, other: &Party) -> bool {
        self.view().covers(other.view())
    }

    /// Carve `other`'s region out of `self`: the region difference
    /// `self \ other`.
    ///
    /// Returns `None` when `other` [`covers`](Party::covers) `self` and
    /// nothing remains; the empty region is not a [`Party`]. Otherwise
    /// returns the remainder, which is always a subregion of `self`
    /// (`self \ other ⊆ self`).
    ///
    /// This is a partial inverse of [`join`](Party::join): where `join`
    /// folds a disjoint share in, `without` cuts a share back out. It
    /// consumes `self` and reads `other` only as a mask, so it introduces no
    /// new aliasing.
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

    /// Duplicate this party, producing a second handle to the same identity, in
    /// violation of linearity.
    ///
    /// # Warning
    ///
    /// [`Party`] is [`!Clone`](Clone) because two live handles to one region
    /// break the Law of Disjointness: the alias is not
    /// [disjoint](Party::is_disjoint) from the original, so if both copies (or
    /// any of their [`fork`](Party::fork)s) go on to [`tick`](Party::tick) or
    /// [`join`](Party::join), causal history can be corrupted arbitrarily. The
    /// caller must ensure that at most one of the two copies is ever treated as
    /// live; the other must be dropped without further use. The same rule
    /// applies to any [`Clock`](crate::Clock) built from such a party.
    ///
    /// This method exists for handing a party across a boundary where ownership
    /// transfers to exactly one side based on an outcome not known at the time
    /// of transfer.
    ///
    /// ```
    /// use before::Party;
    /// let p = Party::seed();
    /// let q = p.dangerously_alias();
    /// assert!(!p.is_disjoint(&q));
    /// ```
    pub fn dangerously_alias(&self) -> Self {
        Party::from_bits(self.0.clone())
    }

    /// Encode a [`Party`] to bytes.
    ///
    /// The byte encoding of a [`Clock`](crate::Clock) is not the
    /// concatenation of the encodings of its [`Party`] and
    /// [`Version`]; see
    /// [`Clock::encode`](crate::Clock::encode).
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
    /// // The seed is a single terminal: a 2-bit presence tag (`00`).
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
        Ok(Party::from_bits(id))
    }

    /// The anonymous (zero) id: the empty bit stream, since a `0` is structural
    /// absence in the pruned encoding.
    ///
    /// Internal and transient only (i.e. for use
    /// in `mem::swap`) and *never* a publicly constructible value (a `Party` is
    /// a nonzero share).
    ///
    /// Used as a placeholder when moving a party out of a `&mut` during `sync`,
    /// immediately overwritten by the re-split half.
    pub(crate) fn anonymous() -> Party {
        Party::from_bits(codec::Bits::new())
    }

    /// A read-only [`IdReader`] cursor at the root of this party's packed id bits.
    fn view(&self) -> IdReader<'_> {
        IdReader::root(&self.0)
    }

    /// The canonical packed bytes of this [`Party`]: what
    /// [`encode`](Self::encode) produces, borrowed without copying.
    ///
    /// The final
    /// partial byte is zero-padded in the stored form, so these bytes are a
    /// canonical identity: byte-equal if and only if the parties are equal, and
    /// consistent with [`hash`](core::hash::Hash).
    ///
    /// A [`Party`] is not ordered (see the type docs). The lexicographic order
    /// of these bytes is an arbitrary total order with no semantic meaning,
    /// useful only as a deterministic tiebreak. Use
    /// [`is_disjoint`](Self::is_disjoint) to reason about whether two parties
    /// may interact.
    ///
    /// ```
    /// use before::Party;
    /// let p = Party::seed();
    /// assert_eq!(p.as_bytes(), p.encode().as_slice());
    /// ```
    pub fn as_bytes(&self) -> &[u8] {
        let raw = self.0.as_raw_slice();
        debug_assert_eq!(
            raw,
            self.encode().as_slice(),
            "non-canonical Party storage: as_bytes must equal encode (dead bits not zeroed)",
        );
        raw
    }

    /// The packed preorder bit stream (no trailing padding). Internal.
    pub(crate) fn as_bits(&self) -> &BitsSlice {
        &self.0
    }

    /// Wrap a normal-form packed bit stream as a `Party`, canonicalizing its
    /// storage. The single gate every built/parsed `Party` passes through.
    ///
    /// Callers guarantee normal *tree* form (a nonempty, normalized id); this
    /// zeroes the dead bits past the live length so the stored bytes are
    /// canonical — see [`codec::zero_dead_bits`] for why a tree op can leave
    /// them non-zero, and what byte-canonicity underpins.
    pub(crate) fn from_bits(mut bits: codec::Bits) -> Self {
        codec::zero_dead_bits(&mut bits);
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
/// nested `(left, right)` tuples.
///
/// Sealed and hidden — an implementation detail enabling
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
