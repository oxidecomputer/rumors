//! Disjoint parties who can emit events.

use core::cmp::Ordering;
use core::fmt::Display;

use bitvec::prelude::*;

use crate::codec::{self, BitsSlice};
use crate::error::{Decode, Parse};
use crate::idbits::IdView;

mod ops;

#[cfg(test)]
mod tests;

/// A disjoint party.
///
/// Parties are ordered by ancestry: [`seed`](Party::seed) is the minimum;
/// siblings and cousins are incomparable. For disjoint parties,
/// [`join`](Party::join) computes the meet under this order.
///
/// ```
/// use before::Party;
/// let mut whole = Party::seed();
/// let half = whole.fork();
/// assert!(whole.is_disjoint(&half));
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
        match self.view().sum(&other.view()) {
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
        self.view().is_disjoint(&other.view())
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

    /// A read-only [`IdView`] cursor over this party's packed id bits.
    fn view(&self) -> IdView<'_> {
        IdView(&self.0)
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

impl PartialOrd for Party {
    /// Descent: an ancestor (larger region) is *less than* its forked descendants;
    /// cousins are incomparable (`None`). `self < other` ⇔ `self` contains `other`.
    /// One pass tracks both containment directions (see `IdView::compare`); running
    /// the containment test once per direction would double the work.
    ///
    /// ```
    /// use before::Party;
    /// let whole: Party = "1".parse().unwrap();
    /// let part: Party = "(1, 0)".parse().unwrap();
    /// assert!(whole < part); // an ancestor region precedes its descendants
    /// let sibling: Party = "(0, 1)".parse().unwrap();
    /// assert!(part.partial_cmp(&sibling).is_none()); // siblings are incomparable
    /// ```
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.view().compare(&other.view())
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
