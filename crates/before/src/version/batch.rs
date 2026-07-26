//! The chaining mutation handle for a [`Version`]: [`Batch`] borrows one
//! version mutably and applies operations in place, each committing as it
//! runs.

use crate::codec::{self, Bits};
use crate::Party;

use super::skyline;
use super::Version;

/// A batch of operations on one [`Version`], applied through a single
/// mutable borrow with a chainable API.
///
/// Every operation commits to the underlying version as it runs: a `Batch`
/// holds no divergent state, so reads through it (comparison,
/// [`snapshot`](Batch::snapshot)) always see the latest committed value.
///
/// ```
/// use before::{Party, Version};
/// let party = Party::seed();
/// let mut v = Version::new();
/// v.batch().tick(&party).tick(&party);
/// assert_eq!(v.to_string(), "2");
/// ```
pub struct Batch<'v> {
    version: &'v mut Version,
}

impl<'v> Batch<'v> {
    /// Begin a batch over `version`. The public entry point is
    /// [`Version::batch`].
    pub(super) fn new(version: &'v mut Version) -> Self {
        Batch { version }
    }
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
        *self.version = Version::from_bits(skyline::fill::tick(&self.version.0, party));
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
    /// skyline stream into this batch's version.
    ///
    /// Any operand with a [`view`](Self::view) (a [`Version`] or another
    /// [`Batch`], owned or borrowed) joins through here, so the `|`/`|=`
    /// matrix accepts a [`Batch`] on either side without transcoding.
    ///
    /// Before the merge sweep, two `O(1)` short-circuits settle the cases
    /// canonical form makes immediate: trivial equality (`a ∨ a = a`, a
    /// no-op, decided by a byte compare of the two unique streams) and the
    /// lattice identity `0 ∨ v = v` — an empty incoming leaves the current
    /// tree untouched, and an empty current adopts the incoming stream
    /// wholesale (a copy, byte-identical to what the merge would emit). The
    /// identity path is the common seed pattern: folds seeded with
    /// [`Version::new`] (`join_all`, `Sum`) hit it on their first join.
    pub(super) fn join_view(&mut self, incoming: &Bits) -> &mut Self {
        if codec::canonical_eq(&self.version.0, incoming) {
            return self; // a ∨ a = a
        }
        if skyline::is_empty_stream(incoming) {
            return self; // v ∨ 0 = v: nothing to fold in
        }
        if skyline::is_empty_stream(&self.version.0) {
            // 0 ∨ v = v: adopt the incoming stream wholesale. Both streams
            // are canonical, so the copy equals the merge byte for byte.
            *self.version = Version::from_bits(incoming.clone());
            return self;
        }
        *self.version = Version::from_bits(skyline::emit::join(&self.version.0, incoming));
        self
    }

    /// The view-taking meet core, the dual of
    /// [`join_view`](Self::join_view): meet an arbitrary skyline stream
    /// into this batch's version.
    ///
    /// The `&`/`&=` matrix routes through here just as the `|`/`|=` matrix
    /// routes through `join_view`, and accepts a [`Batch`] on either side
    /// without transcoding.
    ///
    /// The dual short-circuits apply: trivial equality (`a ∧ a = a`), and the
    /// empty version as the *absorbing* element, `0 ∧ v = 0` — an empty
    /// current is already the answer, and an empty incoming makes the result
    /// the empty version outright, no merge sweep either way.
    pub(super) fn meet_view(&mut self, incoming: &Bits) -> &mut Self {
        if codec::canonical_eq(&self.version.0, incoming) {
            return self; // a ∧ a == a
        }
        if skyline::is_empty_stream(&self.version.0) {
            return self; // 0 ∧ v = 0: already empty, nothing can shrink it
        }
        if skyline::is_empty_stream(incoming) {
            // v ∧ 0 = 0: the result is the empty version, whatever `v` was.
            self.replace_with(Version::new());
            return self;
        }
        *self.version = Version::from_bits(skyline::emit::meet(&self.version.0, incoming));
        self
    }

    /// Replace the version with an already-canonical owned value.
    /// Used by `clock::Batch::sync` after it computes the merged history once.
    pub(crate) fn replace_with(&mut self, version: Version) {
        *self.version = version;
    }

    /// The current value as an owned [`Version`] without ending the batch.
    ///
    /// Every operation commits as it runs, so this is a clone of the
    /// underlying version at this point in the chain.
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
        self.version.clone()
    }

    /// A read-only view of the version's stored skyline stream.
    pub(super) fn view(&self) -> &Bits {
        &self.version.0
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
