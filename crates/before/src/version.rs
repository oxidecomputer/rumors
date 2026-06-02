//! [`Version`] — an interval-tree-clock event tree / message, and its working-form
//! mutation [`Batch`].

use core::cmp::Ordering;
use core::fmt::Display;
use core::ops::{BitOr, BitOrAssign};

use bitvec::prelude::*;

use crate::codec::{self, BitsSlice};
use crate::error::{Decode, Parse};
use crate::Party;

use self::compare::EvView;
use self::working::WorkingVersion;

mod compare;
mod event;
mod working;

#[cfg(test)]
mod tests;

/// A causal version.
///
/// ```
/// use before::{Party, Version};
/// let party = Party::seed();
/// let mut v = Version::new();
/// v.tick(&party);
/// assert!(v > Version::new()); // a tick advances the causal order
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Version(BitVec<u8, Msb0>);

impl Version {
    /// The empty [`Version`], representing no [`tick`](Version::tick)s.
    ///
    /// ```
    /// assert_eq!(before::Version::new().to_string(), "0");
    /// ```
    pub fn new() -> Self {
        let mut bits = codec::Bits::new();
        bits.push(false); // leaf flag
        codec::encode_int(&mut bits, &codec::Base::ZERO);
        Version(bits)
    }

    /// Advance the [`Version`] from the perspective of [`Party`].
    ///
    /// ```
    /// use before::{Party, Version};
    /// let mut v = Version::new();
    /// v.tick(&Party::seed());
    /// assert_eq!(v.to_string(), "1");
    /// ```
    pub fn tick(&mut self, party: &Party) {
        self.batch().tick(party);
    }

    /// Determine if two [`Version`]s are concurrent, i.e. incomparable.
    ///
    /// ```
    /// use before::Clock;
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let vb = b.tick().clone();
    /// assert!(va.concurrent(&vb)); // ticks on disjoint parties are concurrent
    /// ```
    pub fn concurrent<V: PartialOrd<Self>>(&self, version: &V) -> bool {
        version.partial_cmp(self).is_none()
    }

    /// Begin a batch of operations on this [`Version`].
    ///
    /// Sequential operations within a [`Batch`] are more efficient.
    ///
    /// ```
    /// use before::{Party, Version};
    /// let party = Party::seed();
    /// let mut v = Version::new();
    /// v.batch().tick(&party).tick(&party);
    /// assert_eq!(v.to_string(), "2");
    /// ```
    pub fn batch(&mut self) -> Batch<'_> {
        Batch {
            version: self,
            work: None,
        }
    }

    /// A read-only view of this version's event tree.
    fn view(&self) -> EvView<'_> {
        EvView::Packed(&self.0)
    }

    /// Encode this [`Version`] to bytes.
    ///
    /// **Note:** The byte-encoding of a [`Clock`](crate::Clock) is **not the
    /// same** as the concatenation of the byte-encoding of a [`Party`] and a
    /// [`Version`].
    ///
    /// ```
    /// use before::Version;
    /// let v = Version::new();
    /// assert_eq!(Version::decode(&v.encode()[..]).unwrap(), v);
    /// ```
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.encode_to(&mut bytes)
            .expect("writing to a Vec is infallible");
        bytes
    }

    /// Encode a [`Version`] to an arbitrary writer.
    ///
    /// ```
    /// use before::Version;
    /// let mut buf = Vec::new();
    /// Version::new().encode_to(&mut buf).unwrap();
    /// assert_eq!(buf, Version::new().encode());
    /// ```
    pub fn encode_to<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        codec::pack_to_writer(&self.0, writer)
    }

    /// Decode a [`Version`] from a reader of canonical bytes.
    ///
    /// ```
    /// use before::Version;
    /// let bytes = Version::new().encode();
    /// assert_eq!(Version::decode(&bytes[..]).unwrap(), Version::new());
    /// ```
    pub fn decode<R: std::io::Read>(mut reader: R) -> Result<Self, Decode> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).map_err(Decode::Io)?;
        let end = {
            let bits = codec::bytes_as_bits(&buf);
            let end = codec::parse_ev(bits, 0)?;
            codec::require_zero_padding(bits, end)?;
            end
        };
        // Reuse the read buffer as the result's backing store (offset-0,
        // canonical up to `end`), so decoding allocates no more than before.
        let mut bits = codec::Bits::from_vec(buf);
        bits.truncate(end);
        Ok(Version(bits))
    }

    /// The exact length in bits of [`encode`](Self::encode) before its zero-pad
    /// to a byte boundary.
    ///
    /// ```
    /// use before::Version;
    /// // The empty version is a single `0` leaf: a flag bit plus a value bit.
    /// assert_eq!(Version::new().encoded_bits(), 2);
    /// ```
    pub fn encoded_bits(&self) -> usize {
        self.as_bits().len()
    }

    /// The packed preorder bit stream (no trailing padding). Internal.
    pub(crate) fn as_bits(&self) -> &BitsSlice {
        &self.0
    }

    /// Wrap a canonical packed bit stream. Internal; callers guarantee normal form.
    pub(crate) fn from_bits(bits: codec::Bits) -> Self {
        Version(bits)
    }
}

/// The empty [`Version`] (same as [`Version::new`]).
///
/// ```
/// assert_eq!(before::Version::default(), before::Version::new());
/// ```
impl Default for Version {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialOrd for Version {
    /// The causal order; `None` means the two versions are concurrent.
    ///
    /// ```
    /// use before::Version;
    /// let a: Version = "1".parse().unwrap();
    /// let b: Version = "2".parse().unwrap();
    /// assert!(a < b);
    /// ```
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.view().causal_cmp(&other.view())
    }
}

/// Paper notation: `n` leaves, `(n, e1, e2)` nodes. E.g. `(1, 2, (0, (1, 0, 2), 0))`.
///
/// ```
/// use before::Version;
/// let v: Version = "(1, 2, (0, (1, 0, 2), 0))".parse().unwrap();
/// assert_eq!(v.to_string(), "(1, 2, (0, (1, 0, 2), 0))");
/// ```
impl core::fmt::Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        codec::write_ev(&self.0, f, ", ")
    }
}

/// The same format as `Display`.
///
/// ```
/// assert_eq!(format!("{:?}", before::Version::new()), "0");
/// ```
impl core::fmt::Debug for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <Self as Display>::fmt(self, f)
    }
}

/// Parse paper notation (`n` or `(n, e1, e2)`), strictly rejecting non-normal-form input.
///
/// ```
/// use before::Version;
/// let v: Version = "(1, 0, 1)".parse().unwrap();
/// assert_eq!(v.to_string(), "(1, 0, 1)");
/// ```
impl core::str::FromStr for Version {
    type Err = Parse;
    fn from_str(s: &str) -> Result<Self, Parse> {
        Ok(Version::from_bits(codec::parse_ev_str(s)?))
    }
}

/// An event leaf from its base value, e.g. `Version::try_from(3u64)`.
///
/// ```
/// use before::Version;
/// assert_eq!(Version::try_from(3).unwrap().to_string(), "3");
/// ```
impl TryFrom<u64> for Version {
    type Error = Parse;
    fn try_from(n: u64) -> Result<Self, Parse> {
        Ok(Version::from_bits(codec::ev_leaf(n)))
    }
}

/// An event node from an `(n, left, right)` literal, e.g.
/// `Version::try_from((1u64, 0u64, (2u64, 0u64, 1u64)))`. Rejects non-normal-form nodes
/// (no zero-base child, or a collapsible `(n, m, m)`).
///
/// ```
/// use before::Version;
/// let v = Version::try_from((1, 0, 1)).unwrap();
/// assert_eq!(v.to_string(), "(1, 0, 1)");
/// ```
impl<T, S> TryFrom<(u64, T, S)> for Version
where
    Version: TryFrom<T, Error = Parse> + TryFrom<S, Error = Parse>,
{
    type Error = Parse;
    fn try_from((n, l, r): (u64, T, S)) -> Result<Self, Parse> {
        let l = Version::try_from(l)?;
        let r = Version::try_from(r)?;
        Ok(Version::from_bits(codec::ev_node(n, &l.0, &r.0)?))
    }
}

/// A batch for a [`Version`], providing a similar API, but faster for multiple
/// operations.
///
/// ```
/// use before::{Party, Version};
/// let party = Party::seed();
/// let mut v = Version::new();
/// v.batch().tick(&party).tick(&party); // amortized; repacked when the batch drops
/// assert_eq!(v.to_string(), "2");
/// ```
pub struct Batch<'v> {
    version: &'v mut Version,
    work: Option<WorkingVersion>,
}

impl Batch<'_> {
    /// Like [`tick`](Version::tick), but chainable.
    ///
    /// ```
    /// use before::{Party, Version};
    /// let mut v = Version::new();
    /// v.batch().tick(&Party::seed());
    /// assert_eq!(v.to_string(), "1");
    /// ```
    pub fn tick(&mut self, party: &Party) -> &mut Self {
        let work = self
            .work
            .take()
            .unwrap_or_else(|| WorkingVersion::unpack(self.version.as_bits()));
        self.work = Some(event::tick(party.as_bits(), &work));
        self
    }

    /// Like [`concurrent`](Version::concurrent).
    ///
    /// ```
    /// use before::{Party, Version};
    /// let party = Party::seed();
    /// let mut later = Version::new();
    /// later.tick(&party);
    /// let mut v = Version::new();
    /// let batch = v.batch();
    /// // an empty version and a later one on the same line are comparable
    /// assert!(!batch.concurrent(&later));
    /// ```
    pub fn concurrent<V: PartialOrd<Self>>(&self, version: &V) -> bool {
        version.partial_cmp(self).is_none()
    }

    /// Like `|=`, but chainable.
    pub(crate) fn merge(&mut self, other: &Version) -> &mut Self {
        let current = self.view();
        let incoming = other.view();
        if current.trivially_eq(&incoming) {
            return self;
        }
        let work = current.join(&incoming);
        self.work = Some(work);
        self
    }

    /// Replace the in-progress history with an already-canonical owned version.
    /// Used by `clock::Batch::sync` after it computes the merged history once.
    pub(crate) fn replace_with(&mut self, version: Version) {
        self.work = None;
        *self.version = version;
    }

    /// Snapshot the in-progress history as an owned, canonical [`Version`] — without
    /// committing or forcing materialization. Used by `clock::Batch` for `fork`/`sync`,
    /// which must hand a concrete version to another clock mid-session.
    pub(crate) fn snapshot(&self) -> Version {
        match &self.work {
            Some(work) => Version::from_bits(work.repack()),
            None => self.version.clone(),
        }
    }

    /// A read-only view of the in-progress event tree (working form if
    /// materialized, otherwise the borrowed version's packed bits).
    fn view(&self) -> EvView<'_> {
        match &self.work {
            Some(work) => EvView::Working(work),
            None => EvView::Packed(self.version.as_bits()),
        }
    }
}

impl Drop for Batch<'_> {
    fn drop(&mut self) {
        if let Some(work) = self.work.take() {
            *self.version = Version::from_bits(work.repack());
        }
    }
}

/// Borrow a [`Version`] as a [`Batch`]; equivalent to [`Version::batch`].
///
/// ```
/// use before::{batch, Version};
/// let mut v = Version::new();
/// let _batch: batch::Version = (&mut v).into();
/// ```
impl<'a> From<&'a mut Version> for Batch<'a> {
    fn from(v: &'a mut Version) -> Self {
        v.batch()
    }
}

/// `a | b` is the causal join (least upper bound) of two [`Version`]s.
///
/// ```
/// use before::Clock;
/// let mut a = Clock::seed();
/// let mut b = a.fork();
/// let va = a.tick().clone();
/// let vb = b.tick().clone();
/// let joined = va.clone() | vb.clone();
/// assert!(joined >= va && joined >= vb); // the join dominates both inputs
/// ```
impl BitOr<Version> for Version {
    type Output = Version;
    fn bitor(self, r: Version) -> Version {
        if self == r {
            return self;
        }
        let work = self.view().join(&r.view());
        Version::from_bits(work.repack())
    }
}

/// `a |= b` joins `b` into `a` in place.
///
/// ```
/// use before::Clock;
/// let mut a = Clock::seed();
/// let mut b = a.fork();
/// let mut va = a.tick().clone();
/// let vb = b.tick().clone();
/// va |= vb.clone();
/// assert!(va >= vb); // `a` now dominates what it absorbed
/// ```
impl BitOrAssign<Version> for Version {
    fn bitor_assign(&mut self, r: Version) {
        if *self == r {
            return;
        }
        let work = self.view().join(&r.view());
        *self = Version::from_bits(work.repack());
    }
}

/// Merge a [`Version`] into a version [`Batch`] in place.
///
/// ```
/// use before::{Party, Version};
/// let mut base = Version::new();
/// base.tick(&Party::seed());
/// let mut v = Version::new();
/// {
///     let mut batch = v.batch();
///     batch |= &base;
/// }
/// assert_eq!(v.to_string(), "1");
/// ```
impl BitOrAssign<&Version> for Batch<'_> {
    fn bitor_assign(&mut self, r: &Version) {
        self.merge(r);
    }
}

// Causal comparison across {Version, Batch}², reading current state in place.
// `Version`/`Version` lives separately (derived `PartialEq` + the `PartialOrd`
// above); this macro fills in the remaining three off-diagonal/`Batch` cells so
// the comparison matrix reads as a matrix. Each cell delegates to `causal_cmp`,
// with `eq` defined as `partial_cmp == Some(Equal)`.
macro_rules! causal_cmp_impls {
    ($($lhs:ty, $rhs:ty);* $(;)?) => {
        $(
            impl PartialEq<$rhs> for $lhs {
                fn eq(&self, o: &$rhs) -> bool {
                    self.view().causal_cmp(&o.view()) == Some(Ordering::Equal)
                }
            }
            impl PartialOrd<$rhs> for $lhs {
                fn partial_cmp(&self, o: &$rhs) -> Option<Ordering> {
                    self.view().causal_cmp(&o.view())
                }
            }
            impl PartialEq<$rhs> for &$lhs {
                fn eq(&self, o: &$rhs) -> bool {
                    self.view().causal_cmp(&o.view()) == Some(Ordering::Equal)
                }
            }
            impl PartialOrd<$rhs> for &$lhs {
                fn partial_cmp(&self, o: &$rhs) -> Option<Ordering> {
                    self.view().causal_cmp(&o.view())
                }
            }
            impl PartialEq<&$rhs> for $lhs {
                fn eq(&self, o: &&$rhs) -> bool {
                    self.view().causal_cmp(&o.view()) == Some(Ordering::Equal)
                }
            }
            impl PartialOrd<&$rhs> for $lhs {
                fn partial_cmp(&self, o: &&$rhs) -> Option<Ordering> {
                    self.view().causal_cmp(&o.view())
                }
            }
        )*
    };
}

causal_cmp_impls! {
    Version, Batch<'_>;
    Batch<'_>, Version;
    Batch<'_>, Batch<'_>;
}
