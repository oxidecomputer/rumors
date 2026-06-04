//! [`Version`] — an interval-tree-clock event tree / message, and its working-form
//! mutation [`Batch`].

use core::cmp::Ordering;
use core::fmt::Display;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

use bitvec::prelude::*;

use crate::codec::{self, BitsSlice};
use crate::error::{Decode, Parse};
use crate::Party;

use self::compare::EvReader;
use self::working::WorkingVersion;

mod compare;
mod event;
mod working;

#[cfg(test)]
mod tests;

/// A causal version: an event tree timestamping a [`Party`]'s history.
///
/// Comparison and **join** (`|`) are what give a version meaning;
/// [`tick`](Version::tick) is the only way to change one:
///
/// | Operation                                 | Meaning                                                        |
/// |-------------------------------------------|----------------------------------------------------------------|
/// | `a == b`                                  | identical causal history                                       |
/// | `a < b`, `a <= b`                         | `a` is causally dominated by `b`: every event in `a` is in `b` |
/// | [`a.concurrent(b)`](Version::concurrent)  | incomparable: neither dominates the other                      |
/// | `a \| b`, `a \|= b`                       | the *join* (least upper bound): the combined history of both   |
/// | `a & b`, `a &= b`                         | the *meet* (greatest lower bound): the history common to both  |
/// | [`a.tick(&p)`](Version::tick)             | record one new event for [`Party`] `p`                         |
///
/// Comparison is **partial** ([`PartialOrd`], not [`Ord`]): two distinct
/// versions can be [`concurrent`](Version::concurrent), and then `a < b`,
/// `a == b`, and `a > b` are all false.
///
/// ```
/// use before::Clock;
/// let mut a = Clock::seed();
/// let mut b = a.fork();
/// let va = a.tick();
/// let vb = b.tick();
/// assert!(va.concurrent(vb));  // ticking two forks makes them concurrent
/// let merged = va | vb;
/// assert!(merged > va && merged > vb);  // the join dominates both inputs
/// ```
// `PartialEq` is the macro's `causal_cmp == Equal` (see `causal_cmp_impls!`);
// for canonical normal form that *is* byte-equality, so the derived `Hash` over
// the packed bits stays consistent with it. clippy can't see the invariant.
#[allow(clippy::derived_hash_with_manual_eq)]
#[derive(Clone, Eq, Hash)]
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
    fn view(&self) -> EvReader<'_> {
        EvReader::packed(&self.0)
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

    /// The canonical packed bytes of this [`Version`]: exactly what
    /// [`encode`](Self::encode) produces, but borrowed without copying. The
    /// final partial byte is zero-padded (an invariant of the stored form), so
    /// these bytes are a *canonical* identity — byte-equal if and only if the
    /// [`Version`]s are equal, and stable to [`hash`](core::hash::Hash).
    ///
    /// Their lexicographic order is an arbitrary total order with **no causal
    /// meaning**: use it only where a deterministic tiebreak between distinct
    /// versions is wanted. For causal comparison, use [`PartialOrd`] (`<=`) or
    /// [`concurrent`](Self::concurrent).
    ///
    /// ```
    /// use before::Version;
    /// let v = Version::try_from((1, 0, 1)).unwrap();
    /// assert_eq!(v.as_bytes(), v.encode().as_slice());
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
        self.merge_view(other.view())
    }

    /// The view-taking core of [`merge`](Self::merge): join an arbitrary
    /// event-tree view into this batch's in-progress history. Any operand with a
    /// [`view`](Self::view) — a [`Version`] or another [`Batch`], owned or
    /// borrowed — joins through here, which is what lets the `|`/`|=` matrix
    /// (below) accept a [`Batch`] on either side without transcoding.
    fn merge_view(&mut self, incoming: EvReader<'_>) -> &mut Self {
        let current = self.view();
        if current.trivially_eq(&incoming) {
            return self;
        }
        let work = current.join(incoming);
        self.work = Some(work);
        self
    }

    /// Like `&=`, but chainable: the greatest-lower-bound dual of
    /// [`merge`](Self::merge).
    fn meet(&mut self, other: &Version) -> &mut Self {
        self.meet_view(other.view())
    }

    /// The view-taking core of [`meet`](Self::meet), the dual of
    /// [`merge_view`](Self::merge_view): meet an arbitrary event-tree view into
    /// this batch's in-progress history. The `&`/`&=` matrix routes through here
    /// exactly as the `|`/`|=` matrix routes through `merge_view`, which is what
    /// lets it accept a [`Batch`] on either side without transcoding.
    fn meet_view(&mut self, incoming: EvReader<'_>) -> &mut Self {
        let current = self.view();
        if current.trivially_eq(&incoming) {
            return self; // a & a == a
        }
        let work = current.meet(incoming);
        self.work = Some(work);
        self
    }

    /// Replace the in-progress history with an already-canonical owned version.
    /// Used by `clock::Batch::sync` after it computes the merged history once.
    pub(crate) fn replace_with(&mut self, version: Version) {
        self.work = None;
        *self.version = version;
    }

    /// Snapshot the in-progress history as an owned, canonical [`Version`]
    /// without ending the batch.
    ///
    /// Equivalent to the [`Version`] that would result if the batch were
    /// dropped now, but leaves the batch open so further
    /// [`tick`](Self::tick)s/joins continue to accumulate in the materialized
    /// working form. This is what lets a caller read a per-operation version
    /// mid-batch (e.g. to key each insert in a run) while still paying the
    /// unpack cost only once for the whole batch.
    ///
    /// ```
    /// use before::{Party, Version};
    /// let party = Party::seed();
    /// let mut v = Version::new();
    /// let mut batch = v.batch();
    /// let one = batch.tick(&party).snapshot();
    /// let two = batch.tick(&party).snapshot();
    /// assert_eq!(one.to_string(), "1");
    /// assert_eq!(two.to_string(), "2");
    /// assert!(one < two);
    /// ```
    pub fn snapshot(&self) -> Version {
        match &self.work {
            Some(work) => Version::from_bits(work.repack()),
            None => self.version.clone(),
        }
    }

    /// A read-only view of the in-progress event tree (working form if
    /// materialized, otherwise the borrowed version's packed bits).
    fn view(&self) -> EvReader<'_> {
        match &self.work {
            Some(work) => EvReader::working(work),
            None => EvReader::packed(self.version.as_bits()),
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
        &self | r
    }
}

impl BitOr<&Version> for Version {
    type Output = Version;
    fn bitor(mut self, r: &Version) -> Version {
        self.batch().merge(r);
        self
    }
}

impl BitOr<Version> for &Version {
    type Output = Version;
    fn bitor(self, r: Version) -> Version {
        r | self
    }
}

impl BitOr<&Version> for &Version {
    type Output = Version;
    fn bitor(self, r: &Version) -> Version {
        self.clone() | r
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
        *self = &*self | r;
    }
}

impl BitOrAssign<&Version> for Version {
    fn bitor_assign(&mut self, r: &Version) {
        *self = &*self | r;
    }
}

impl BitOrAssign<Version> for Batch<'_> {
    fn bitor_assign(&mut self, r: Version) {
        self.merge(&r);
    }
}

impl BitOrAssign<&Version> for Batch<'_> {
    fn bitor_assign(&mut self, r: &Version) {
        self.merge(r);
    }
}

// The join (`|`) and assigning join (`|=`) across {Version, Batch}², mirroring
// the comparison matrix below. The `Version`/`Version` cells (and their three
// reference forms) are the documented exemplars hand-written above; these two
// macros fill in every remaining cell — the ones with a `Batch` on one or both
// sides — so the join matrix reads as a matrix.
//
// Both route through `Batch::merge_view`, which joins any `.view()` into a
// batch. `|` snapshots its left operand to a fresh owned `Version` (a `Batch`
// left operand via `snapshot`, a borrowed `Version` via `clone`, an owned
// `Version` moved in place) and joins the right operand's view into it; a
// `Batch` read for `|` is never mutated by the join and commits its own pending
// state on drop as usual. `|=` joins the right operand's view into the left
// operand in place (a `Version` through a transient `batch()`, a `Batch`
// directly).

/// Fills the non-`Version`/`Version` cells of the `|` matrix. Each cell owns its
/// left operand as a fresh `Version` — `own` (move, for an owned `Version`),
/// `clone` (a borrowed `Version`), or `snapshot` (a `Batch`, owned or borrowed)
/// — then joins the right operand's view into it. Each kind has its own `@impl`
/// arm so the receiver `self` is written in the same expansion as the method it
/// belongs to (`self` cannot cross a macro-invocation boundary).
macro_rules! version_join_impls {
    ($($lhs:ty, $rhs:ty, $own:tt);* $(;)?) => {
        $( version_join_impls!(@impl $lhs, $rhs, $own); )*
    };
    (@impl $lhs:ty, $rhs:ty, own) => {
        impl BitOr<$rhs> for $lhs {
            type Output = Version;
            fn bitor(self, r: $rhs) -> Version {
                let mut out: Version = self;
                out.batch().merge_view(r.view());
                out
            }
        }
    };
    (@impl $lhs:ty, $rhs:ty, clone) => {
        impl BitOr<$rhs> for $lhs {
            type Output = Version;
            fn bitor(self, r: $rhs) -> Version {
                let mut out: Version = self.clone();
                out.batch().merge_view(r.view());
                out
            }
        }
    };
    (@impl $lhs:ty, $rhs:ty, snapshot) => {
        impl BitOr<$rhs> for $lhs {
            type Output = Version;
            fn bitor(self, r: $rhs) -> Version {
                let mut out: Version = self.snapshot();
                out.batch().merge_view(r.view());
                out
            }
        }
    };
}

version_join_impls! {
    Version,    Batch<'_>,  own;
    Version,    &Batch<'_>, own;
    &Version,   Batch<'_>,  clone;
    &Version,   &Batch<'_>, clone;
    Batch<'_>,  Version,    snapshot;
    Batch<'_>,  &Version,   snapshot;
    &Batch<'_>, Version,    snapshot;
    &Batch<'_>, &Version,   snapshot;
    Batch<'_>,  Batch<'_>,  snapshot;
    Batch<'_>,  &Batch<'_>, snapshot;
    &Batch<'_>, Batch<'_>,  snapshot;
    &Batch<'_>, &Batch<'_>, snapshot;
}

/// Fills the `Batch`-valued right-operand cells of the `|=` matrix (the
/// `Version`/`Batch` right operands are hand-written above). The right operand's
/// view is joined into the left operand in place: through a transient `batch()`
/// for a `Version` left operand (`batch`), or directly for a `Batch` left
/// operand (`direct`). Split into per-kind `@impl` arms for the same reason as
/// the `|` macro: `self` must be written alongside its method.
macro_rules! version_join_assign_impls {
    ($($lhs:ty, $rhs:ty, $into:tt);* $(;)?) => {
        $( version_join_assign_impls!(@impl $lhs, $rhs, $into); )*
    };
    (@impl $lhs:ty, $rhs:ty, batch) => {
        impl BitOrAssign<$rhs> for $lhs {
            fn bitor_assign(&mut self, r: $rhs) {
                self.batch().merge_view(r.view());
            }
        }
    };
    (@impl $lhs:ty, $rhs:ty, direct) => {
        impl BitOrAssign<$rhs> for $lhs {
            fn bitor_assign(&mut self, r: $rhs) {
                self.merge_view(r.view());
            }
        }
    };
}

version_join_assign_impls! {
    Version,   Batch<'_>,  batch;
    Version,   &Batch<'_>, batch;
    Batch<'_>, Batch<'_>,  direct;
    Batch<'_>, &Batch<'_>, direct;
}

/// `a & b` is the causal meet (greatest lower bound) of two [`Version`]s: the
/// history common to both, dual to the join `|`.
///
/// ```
/// use before::Clock;
/// let mut a = Clock::seed();
/// let mut b = a.fork();
/// let va = a.tick().clone();
/// let vb = b.tick().clone();
/// let met = va.clone() & vb.clone();
/// assert!(met <= va && met <= vb); // the meet is dominated by both inputs
/// ```
impl BitAnd<Version> for Version {
    type Output = Version;
    fn bitand(self, r: Version) -> Version {
        &self & r
    }
}

impl BitAnd<&Version> for Version {
    type Output = Version;
    fn bitand(mut self, r: &Version) -> Version {
        self.batch().meet(r);
        self
    }
}

impl BitAnd<Version> for &Version {
    type Output = Version;
    fn bitand(self, r: Version) -> Version {
        r & self // meet is commutative
    }
}

impl BitAnd<&Version> for &Version {
    type Output = Version;
    fn bitand(self, r: &Version) -> Version {
        self.clone() & r
    }
}

/// `a &= b` meets `b` into `a` in place.
///
/// ```
/// use before::Clock;
/// let mut a = Clock::seed();
/// let mut b = a.fork();
/// let mut va = a.tick().clone();
/// let vb = b.tick().clone();
/// let before = va.clone();
/// va &= vb;
/// assert!(va <= before); // `a` is narrowed to what it shares
/// ```
impl BitAndAssign<Version> for Version {
    fn bitand_assign(&mut self, r: Version) {
        *self = &*self & r;
    }
}

impl BitAndAssign<&Version> for Version {
    fn bitand_assign(&mut self, r: &Version) {
        *self = &*self & r;
    }
}

impl BitAndAssign<Version> for Batch<'_> {
    fn bitand_assign(&mut self, r: Version) {
        self.meet(&r);
    }
}

impl BitAndAssign<&Version> for Batch<'_> {
    fn bitand_assign(&mut self, r: &Version) {
        self.meet(r);
    }
}

// The meet (`&`) and assigning meet (`&=`) across {Version, Batch}², the exact
// dual of the join matrix above: same cells, same ownership/snapshot strategy,
// routing through `Batch::meet_view` (which meets any `.view()` into a batch)
// instead of `merge_view`. The `Version`/`Version` cells are hand-written above;
// these two macros fill in every remaining cell with a `Batch` on one or both
// sides, so the meet matrix reads as a matrix exactly as the join one does.

/// Fills the non-`Version`/`Version` cells of the `&` matrix, the dual of
/// [`version_join_impls`]. Each cell owns its left operand as a fresh `Version`
/// — `own`, `clone`, or `snapshot` — then meets the right operand's view into
/// it.
macro_rules! version_meet_impls {
    ($($lhs:ty, $rhs:ty, $own:tt);* $(;)?) => {
        $( version_meet_impls!(@impl $lhs, $rhs, $own); )*
    };
    (@impl $lhs:ty, $rhs:ty, own) => {
        impl BitAnd<$rhs> for $lhs {
            type Output = Version;
            fn bitand(self, r: $rhs) -> Version {
                let mut out: Version = self;
                out.batch().meet_view(r.view());
                out
            }
        }
    };
    (@impl $lhs:ty, $rhs:ty, clone) => {
        impl BitAnd<$rhs> for $lhs {
            type Output = Version;
            fn bitand(self, r: $rhs) -> Version {
                let mut out: Version = self.clone();
                out.batch().meet_view(r.view());
                out
            }
        }
    };
    (@impl $lhs:ty, $rhs:ty, snapshot) => {
        impl BitAnd<$rhs> for $lhs {
            type Output = Version;
            fn bitand(self, r: $rhs) -> Version {
                let mut out: Version = self.snapshot();
                out.batch().meet_view(r.view());
                out
            }
        }
    };
}

version_meet_impls! {
    Version,    Batch<'_>,  own;
    Version,    &Batch<'_>, own;
    &Version,   Batch<'_>,  clone;
    &Version,   &Batch<'_>, clone;
    Batch<'_>,  Version,    snapshot;
    Batch<'_>,  &Version,   snapshot;
    &Batch<'_>, Version,    snapshot;
    &Batch<'_>, &Version,   snapshot;
    Batch<'_>,  Batch<'_>,  snapshot;
    Batch<'_>,  &Batch<'_>, snapshot;
    &Batch<'_>, Batch<'_>,  snapshot;
    &Batch<'_>, &Batch<'_>, snapshot;
}

/// Fills the `Batch`-valued right-operand cells of the `&=` matrix, the dual of
/// [`version_join_assign_impls`]. The right operand's view is met into the left
/// operand in place: through a transient `batch()` for a `Version` left operand
/// (`batch`), or directly for a `Batch` left operand (`direct`).
macro_rules! version_meet_assign_impls {
    ($($lhs:ty, $rhs:ty, $into:tt);* $(;)?) => {
        $( version_meet_assign_impls!(@impl $lhs, $rhs, $into); )*
    };
    (@impl $lhs:ty, $rhs:ty, batch) => {
        impl BitAndAssign<$rhs> for $lhs {
            fn bitand_assign(&mut self, r: $rhs) {
                self.batch().meet_view(r.view());
            }
        }
    };
    (@impl $lhs:ty, $rhs:ty, direct) => {
        impl BitAndAssign<$rhs> for $lhs {
            fn bitand_assign(&mut self, r: $rhs) {
                self.meet_view(r.view());
            }
        }
    };
}

version_meet_assign_impls! {
    Version,   Batch<'_>,  batch;
    Version,   &Batch<'_>, batch;
    Batch<'_>, Batch<'_>,  direct;
    Batch<'_>, &Batch<'_>, direct;
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
                    self.view().causal_cmp(o.view()) == Some(Ordering::Equal)
                }
            }
            impl PartialOrd<$rhs> for $lhs {
                fn partial_cmp(&self, o: &$rhs) -> Option<Ordering> {
                    self.view().causal_cmp(o.view())
                }
            }
            impl PartialEq<$rhs> for &$lhs {
                fn eq(&self, o: &$rhs) -> bool {
                    self.view().causal_cmp(o.view()) == Some(Ordering::Equal)
                }
            }
            impl PartialOrd<$rhs> for &$lhs {
                fn partial_cmp(&self, o: &$rhs) -> Option<Ordering> {
                    self.view().causal_cmp(o.view())
                }
            }
            impl PartialEq<&$rhs> for $lhs {
                fn eq(&self, o: &&$rhs) -> bool {
                    self.view().causal_cmp(o.view()) == Some(Ordering::Equal)
                }
            }
            impl PartialOrd<&$rhs> for $lhs {
                fn partial_cmp(&self, o: &&$rhs) -> Option<Ordering> {
                    self.view().causal_cmp(o.view())
                }
            }
        )*
    };
}

causal_cmp_impls! {
    Version, Version;
    Version, Batch<'_>;
    Batch<'_>, Version;
    Batch<'_>, Batch<'_>;
}
