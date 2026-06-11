//! The totally ordered version key: [`Ranked`], a [`Version`] packaged
//! with its [`Rank`], ordered by `(rank, canonical bytes)`. The public
//! contract lives on the type; this module is private.

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

use super::{Rank, Version};

/// A [`Version`] packaged with its [`Rank`]: a totally ordered causal key.
///
/// The order is `(rank, canonical bytes)`. Rank first, so the total order
/// linearly extends causality — `v < w` as versions implies
/// `Ranked::from(v) < Ranked::from(w)` — and the byte tiebreak only ever
/// separates *concurrent* versions (equal ranks are never causally
/// ordered, see [`Rank`]), where any fixed deterministic order is causally
/// safe. Because the encoding is canonical, byte equality *is* version
/// equality: the tiebreak never declares two distinct versions equal, so
/// [`Ord`] is consistent with [`Eq`] ([`PartialEq`] and [`Hash`] delegate
/// to the version), and the order is the same on every replica.
///
/// The rank is computed once, at construction ([`From<Version>`]), and
/// carried: comparisons never re-fold the event tree. A rank comparison
/// cannot short-circuit — a later subtree can always flip a running
/// difference — so deriving the order on demand would pay an `O(n)` fold
/// per comparison, every time a sorted container probes a key. Carrying
/// the rank, sorting `n` versions folds `n` trees, not `O(n log n)`.
///
/// ```
/// use before::{Clock, Ranked};
/// let mut a = Clock::seed();
/// let mut b = a.fork();
/// a.tick();
/// b.tick();
/// let joined = a.version() | b.version(); // dominates both sides
/// let mut keys: Vec<Ranked> =
///     [joined.clone(), a.version().clone(), b.version().clone()]
///         .map(Ranked::from)
///         .into();
/// keys.sort(); // total order: a plain sort, no sidecar ranks
/// assert_eq!(keys[2].version(), &joined); // causes sort before effects
/// ```
#[derive(Clone, Debug)]
pub struct Ranked {
    /// The version itself, the sole witness for equality and hashing.
    version: Version,
    /// The version's rank, computed at construction. An invariant, not a
    /// field: always exactly `version.rank()`.
    rank: Rank,
}

impl Ranked {
    /// The version itself.
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// The version's causal [`Rank`], computed at construction.
    pub fn rank(&self) -> &Rank {
        &self.rank
    }

    /// Unwrap into the version and its rank.
    pub fn into_parts(self) -> (Version, Rank) {
        (self.version, self.rank)
    }
}

impl From<Version> for Ranked {
    /// Compute and carry the version's rank: one `O(n)` fold, here and
    /// never again.
    fn from(version: Version) -> Self {
        let rank = version.rank();
        Ranked { version, rank }
    }
}

// Equality and hashing are the version's: the rank is a function of the
// version, so delegating keeps both consistent without comparing it.
impl PartialEq for Ranked {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
    }
}

impl Eq for Ranked {}

impl Hash for Ranked {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.version.hash(state);
    }
}

impl Ord for Ranked {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank
            .cmp(&other.rank)
            .then_with(|| self.version.as_bytes().cmp(other.version.as_bytes()))
    }
}

impl PartialOrd for Ranked {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
