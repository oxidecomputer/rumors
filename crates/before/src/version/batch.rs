//! The amortizing mutation handle for a [`Version`]: [`Batch`] materializes
//! the packed event tree into working form once, accumulates operations, and
//! repacks when dropped.

use crate::Party;

use super::compare::EvReader;
use super::working::WorkingVersion;
use super::{event, Version};

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

impl<'v> Batch<'v> {
    /// Begin a batch over `version`: no working form is materialized until
    /// the first mutation. The public entry point is [`Version::batch`].
    pub(super) fn new(version: &'v mut Version) -> Self {
        Batch {
            version,
            work: None,
        }
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
    /// event-tree view into this batch's in-progress history.
    ///
    /// Any operand with a [`view`](Self::view) (a [`Version`] or another
    /// [`Batch`], owned or borrowed) joins through here, so the `|`/`|=` matrix
    /// below accepts a [`Batch`] on either side without transcoding.
    ///
    /// Before the combine walk, two `O(1)` short-circuits settle the cases
    /// canonical form makes immediate: trivial equality (`a ∨ a = a`, a no-op)
    /// and the lattice identity `0 ∨ v = v` — an empty incoming leaves the
    /// current tree untouched, and an empty current adopts the incoming tree
    /// wholesale (a copy, byte-identical to what the combine + repack would
    /// produce). The identity path is the common seed pattern: folds seeded
    /// with [`Version::new`] (`join_all`, `Sum`) hit it on their first join.
    pub(super) fn join_view(&mut self, incoming: EvReader<'_>) -> &mut Self {
        let current = self.view();
        if current.trivially_eq(&incoming) {
            return self;
        }
        if incoming.is_empty() {
            return self; // v ∨ 0 = v: nothing to fold in
        }
        if current.is_empty() {
            // 0 ∨ v = v: adopt the incoming tree wholesale. Both forms are
            // canonical normal form, so the copy equals the combine + repack
            // byte for byte.
            match incoming {
                EvReader::Packed { bits, .. } => {
                    self.replace_with(Version::from_bits(bits.to_bitvec()));
                }
                EvReader::Working { work, .. } => self.work = Some(work.clone()),
                // Only real operands reach here (`Version::view`/`Batch::view`
                // never yield `Zero`), and `Zero` is empty, so the incoming
                // check above already returned.
                EvReader::Zero => unreachable!("Zero is empty and short-circuited above"),
            }
            return self;
        }
        let work = current.join(incoming);
        self.work = Some(work);
        self
    }

    /// The view-taking meet core, the dual of
    /// [`join_view`](Self::join_view): meet an arbitrary event-tree view
    /// into this batch's in-progress history.
    ///
    /// The `&`/`&=` matrix routes through here just as the `|`/`|=` matrix
    /// routes through `join_view`, and accepts a [`Batch`] on either side
    /// without transcoding.
    ///
    /// The dual short-circuits apply: trivial equality (`a ∧ a = a`), and the
    /// empty version as the *absorbing* element, `0 ∧ v = 0` — an empty
    /// current is already the answer, and an empty incoming makes the result
    /// the empty version outright, no combine walk either way.
    pub(super) fn meet_view(&mut self, incoming: EvReader<'_>) -> &mut Self {
        let current = self.view();
        if current.trivially_eq(&incoming) {
            return self; // a & a == a
        }
        if current.is_empty() {
            return self; // 0 ∧ v = 0: already empty, nothing can shrink it
        }
        if incoming.is_empty() {
            // v ∧ 0 = 0: the result is the empty version, whatever `v` was.
            self.replace_with(Version::new());
            return self;
        }
        let work = current.meet(incoming);
        self.work = Some(work);
        self
    }

    /// Force the working form to materialize, leaving the value unchanged.
    /// Test-only.
    ///
    /// The parity tests use this to drive Working-form views through the
    /// comparison and combine surfaces. The identity-join idiom they once
    /// used (`join(&Version::new())`) no longer materializes: it is exactly
    /// the lattice short-circuit in [`join_view`](Self::join_view).
    #[cfg(test)]
    pub(super) fn materialize(&mut self) -> &mut Self {
        if self.work.is_none() {
            self.work = Some(WorkingVersion::unpack(self.version.as_bits()));
        }
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
    /// dropped now, but the batch stays open and further
    /// [`tick`](Self::tick)s and joins continue to accumulate in the
    /// materialized working form. A caller can therefore read a
    /// per-operation version mid-batch (for example, to key each insert in a
    /// run) while paying the unpack cost once for the whole batch.
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
    pub(super) fn view(&self) -> EvReader<'_> {
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
