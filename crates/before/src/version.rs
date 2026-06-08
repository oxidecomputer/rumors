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

    /// The minimum number of [`tick`](Self::tick)s that could have produced this
    /// [`Version`]: the sum of every base in its event tree, saturating at
    /// [`u64::MAX`].
    ///
    /// This is a true **floor** over *all* causal histories — every sequence of
    /// [`fork`](crate::Clock::fork)/`tick`/[`join`](crate::Clock::join) that
    /// yields this version performs at least this many ticks — and it is tight:
    /// some history hits it exactly (a single [`Party`] ticking in a line hits
    /// it for a leaf). It exceeds the tallest root-to-leaf path sum whenever
    /// the history forked: `(0, (0,1,0), (0,0,1))` is two height-`1` peaks over
    /// disjoint regions, so although no single path is taller than `1`, two
    /// independent ticks are forced.
    ///
    /// There is no corresponding *maximum*. For any nonempty version the tick
    /// count is unbounded above: one height-`1` increment over an interval can
    /// always be refined into two concurrent height-`1` increments over its
    /// halves (forked, ticked, rejoined) — the same version, one more tick —
    /// without limit.
    ///
    /// ```
    /// use before::Version;
    /// assert_eq!(Version::new().min_ticks(), 0);
    /// assert_eq!(Version::try_from(5).unwrap().min_ticks(), 5);
    /// // Concurrency forces more ticks than the tallest path (1) suggests:
    /// let peaks: Version = "(0, (0, 1, 0), (0, 0, 1))".parse().unwrap();
    /// assert_eq!(peaks.min_ticks(), 2);
    /// ```
    pub fn min_ticks(&self) -> u64 {
        self.view().min_ticks()
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
    pub(crate) fn join(&mut self, other: &Version) -> &mut Self {
        self.join_view(other.view())
    }

    /// The view-taking core of [`join`](Self::join): join an arbitrary
    /// event-tree view into this batch's in-progress history. Any operand with
    /// a [`view`](Self::view) — a [`Version`] or another [`Batch`], owned or
    /// borrowed — joins through here, which is what lets the `|`/`|=` matrix
    /// (below) accept a [`Batch`] on either side without transcoding.
    fn join_view(&mut self, incoming: EvReader<'_>) -> &mut Self {
        let current = self.view();
        if current.trivially_eq(&incoming) {
            return self;
        }
        let work = current.join(incoming);
        self.work = Some(work);
        self
    }

    /// The view-taking meet core, the dual of [`join_view`](Self::join_view):
    /// meet an arbitrary event-tree view into this batch's in-progress history.
    /// The `&`/`&=` matrix routes through here exactly as the `|`/`|=` matrix
    /// routes through [`join_view`](Self::join_view), which is what lets it
    /// accept a [`Batch`] on either side without transcoding.
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

// The join (`|`, `|=`) and meet (`&`, `&=`) matrices across {Version, Batch}²,
// duals of each other and mirroring the comparison matrix below. The
// `binop_matrix!` macro generates every cell of both — all 16 value-operator
// cells (lhs × rhs over {Version, &Version, Batch, &Batch}) and all 8 assign
// cells (lhs over {Version, Batch}) — so each operator reads as a single matrix.
//
// A value-operator cell (`|`/`&`) turns its left operand into a fresh owned
// `Version` — `own` (move, an owned `Version`), `clone` (a borrowed `Version`),
// or `snapshot` (a `Batch`, owned or borrowed) — then folds the right operand's
// view into it; a `Batch` read this way is never mutated and commits its own
// pending state on drop as usual. An assign cell (`|=`/`&=`) folds the right
// operand's view into the left operand in place: through a transient `batch()`
// for a `Version` left operand (`batch`), or directly for a `Batch` (`direct`).
// The only thing that distinguishes the two families is the view-folding method
// each cell routes through: `Batch::join_view` for `|`/`|=`, `Batch::meet_view`
// for `&`/`&=` (each folds any `.view()` into a batch).

/// Generates one binary-operator family's full matrix across {Version, Batch}².
/// Parameterized over the value operator `$Op::$op` (e.g. `BitOr::bitor`), its
/// assigning form `$Assign::$assign` (e.g. `BitOrAssign::bitor_assign`), and the
/// view-folding method `$view` every cell routes through (`join_view` or
/// `meet_view`). Each strategy — `own`/`clone`/`snapshot` for value cells,
/// `batch`/`direct` for assign cells — has its own `@cell` arm so the receiver
/// `self` is written in the same expansion as the method it belongs to (`self`
/// cannot cross a macro-invocation boundary).
macro_rules! binop_matrix {
    ($Op:ident::$op:ident, $Assign:ident::$assign:ident, $view:ident;
     $($lhs:ty, $rhs:ty, $strat:tt);* $(;)?
    ) => {
        $( binop_matrix!(@cell $Op::$op, $Assign::$assign, $view, $lhs, $rhs, $strat); )*
    };
    (@cell $Op:ident::$op:ident, $Assign:ident::$assign:ident, $view:ident, $lhs:ty, $rhs:ty, own) => {
        impl $Op<$rhs> for $lhs {
            type Output = Version;
            fn $op(self, r: $rhs) -> Version {
                let mut out: Version = self;
                out.batch().$view(r.view());
                out
            }
        }
    };
    (@cell $Op:ident::$op:ident, $Assign:ident::$assign:ident, $view:ident, $lhs:ty, $rhs:ty, clone) => {
        impl $Op<$rhs> for $lhs {
            type Output = Version;
            fn $op(self, r: $rhs) -> Version {
                let mut out: Version = self.clone();
                out.batch().$view(r.view());
                out
            }
        }
    };
    (@cell $Op:ident::$op:ident, $Assign:ident::$assign:ident, $view:ident, $lhs:ty, $rhs:ty, snapshot) => {
        impl $Op<$rhs> for $lhs {
            type Output = Version;
            fn $op(self, r: $rhs) -> Version {
                let mut out: Version = self.snapshot();
                out.batch().$view(r.view());
                out
            }
        }
    };
    (@cell $Op:ident::$op:ident, $Assign:ident::$assign:ident, $view:ident, $lhs:ty, $rhs:ty, batch) => {
        impl $Assign<$rhs> for $lhs {
            fn $assign(&mut self, r: $rhs) {
                self.batch().$view(r.view());
            }
        }
    };
    (@cell $Op:ident::$op:ident, $Assign:ident::$assign:ident, $view:ident, $lhs:ty, $rhs:ty, direct) => {
        impl $Assign<$rhs> for $lhs {
            fn $assign(&mut self, r: $rhs) {
                self.$view(r.view());
            }
        }
    };
}

// The join (`|`, `|=`) family. Routes through `Batch::join_view`.
binop_matrix! {
    BitOr::bitor, BitOrAssign::bitor_assign, join_view;
    // value operator: left operand becomes a fresh owned `Version`
    Version,    Version,    own;
    Version,    &Version,   own;
    Version,    Batch<'_>,  own;
    Version,    &Batch<'_>, own;
    &Version,   Version,    clone;
    &Version,   &Version,   clone;
    &Version,   Batch<'_>,  clone;
    &Version,   &Batch<'_>, clone;
    Batch<'_>,  Version,    snapshot;
    Batch<'_>,  &Version,   snapshot;
    Batch<'_>,  Batch<'_>,  snapshot;
    Batch<'_>,  &Batch<'_>, snapshot;
    &Batch<'_>, Version,    snapshot;
    &Batch<'_>, &Version,   snapshot;
    &Batch<'_>, Batch<'_>,  snapshot;
    &Batch<'_>, &Batch<'_>, snapshot;
    // assign: right operand folded into the left operand in place
    Version,    Version,    batch;
    Version,    &Version,   batch;
    Version,    Batch<'_>,  batch;
    Version,    &Batch<'_>, batch;
    Batch<'_>,  Version,    direct;
    Batch<'_>,  &Version,   direct;
    Batch<'_>,  Batch<'_>,  direct;
    Batch<'_>,  &Batch<'_>, direct;
}

// The meet (`&`, `&=`) family: the exact dual of the join matrix above — same
// cells, same ownership/snapshot strategy, same `binop_matrix!` macro — routing
// through `Batch::meet_view` (which meets any `.view()` into a batch) instead of
// `join_view`.
binop_matrix! {
    BitAnd::bitand, BitAndAssign::bitand_assign, meet_view;
    // value operator: left operand becomes a fresh owned `Version`
    Version,    Version,    own;
    Version,    &Version,   own;
    Version,    Batch<'_>,  own;
    Version,    &Batch<'_>, own;
    &Version,   Version,    clone;
    &Version,   &Version,   clone;
    &Version,   Batch<'_>,  clone;
    &Version,   &Batch<'_>, clone;
    Batch<'_>,  Version,    snapshot;
    Batch<'_>,  &Version,   snapshot;
    Batch<'_>,  Batch<'_>,  snapshot;
    Batch<'_>,  &Batch<'_>, snapshot;
    &Batch<'_>, Version,    snapshot;
    &Batch<'_>, &Version,   snapshot;
    &Batch<'_>, Batch<'_>,  snapshot;
    &Batch<'_>, &Batch<'_>, snapshot;
    // assign: right operand folded into the left operand in place
    Version,    Version,    batch;
    Version,    &Version,   batch;
    Version,    Batch<'_>,  batch;
    Version,    &Batch<'_>, batch;
    Batch<'_>,  Version,    direct;
    Batch<'_>,  &Version,   direct;
    Batch<'_>,  Batch<'_>,  direct;
    Batch<'_>,  &Batch<'_>, direct;
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
