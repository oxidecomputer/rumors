//! A [`Clock`] is a [`Party`] paired with a [`Version`].

use core::borrow::Borrow;
use core::fmt::{Debug, Display};
use core::ops::{BitOr, BitOrAssign};
use core::str::FromStr;
use std::io::{Read, Write};

use crate::{
    codec,
    error::{Decode, Overlap, Parse},
    OwnVersion, Party, Ticks, Version,
};

mod forks;

pub use forks::Forks;

#[cfg(test)]
mod tests;

/// A [`Party`] and its [`Version`].
///
/// This type is `!Clone` to strongly discourage non-linear usage: duplicating a
/// [`Clock`] is memory-safe but semantically invalid for interval tree clocks,
/// which require all live clocks in a system to be disjoint.
///
/// Causal comparison and merge happen through the [`Version`]; `Clock` is not
/// itself ordered:
///
/// | Operation                                                                                                                           | Meaning                                                  |
/// |-------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------|
/// | `a.version()` (`<`, `<=`, `==`) `b.version()`                                                                                       | compare causal histories (the order lives on [`Version`])|
/// | [`a.version().concurrent(b.version())`](Version::concurrent)                                                                        | the two clocks' histories are incomparable               |
/// | `clock \| v`, `clock \|= v`                                                                                                         | join a received [`Version`] `v` into this clock          |
/// | [`tick`](Clock::tick)/[`ticks`](Clock::ticks)/[`fork`](Clock::fork)/[`join`](Clock::join)/[`sync`](Clock::sync)/[`send`](Clock::send)/[`recv`](Clock::recv) | advance, split, and reunite clocks             |
///
/// There is deliberately no `Clock | Clock`: merging two whole clocks is the
/// fallible [`join`](Clock::join), which must verify the parties are disjoint.
///
/// # Example
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

// Identity Linearity (the crate docs' second safety rule) is compiler-enforced
// within a process because `Clock` — like the [`Party`] it owns — is `!Clone`:
// duplicating a live clock would put its party's share in two hands. The
// absence is pinned here at the definition, where a tempting `derive` would
// land.
static_assertions::assert_not_impl_any!(Clock: Clone, Copy);

impl Clock {
    /// The initial clock of the distinguished [`Party::seed`]; the only
    /// [`Clock`] not derived from some prior clock.
    ///
    /// Call this function once per system of clocks. Every descendant of a
    /// single seed is disjoint from its peers, but descendants of two
    /// independent seeds need not be; if they ever interact, causal history is
    /// silently corrupted.
    ///
    /// # Complexity
    ///
    /// `O(1)`, and no allocation.
    ///
    /// # Example
    ///
    /// ```
    /// let clock = before::Clock::seed();
    /// assert!(clock.party().is_seed());    // the whole region...
    /// assert!(clock.version().is_empty()); // ...with no events yet
    /// ```
    pub fn seed() -> Self {
        Self::from_parts(Party::seed(), Version::new())
    }

    /// Advances this [`Clock`] by one event for its own [`Party`], returning
    /// the new [`Version`].
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/clock_tick.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// let mut clock = before::Clock::seed();
    /// let start = clock.version().clone();
    /// assert!(*clock.tick() > start); // one event: strictly later
    /// ```
    pub fn tick(&mut self) -> &Version {
        self.version.tick(&self.party);
        self.version()
    }

    /// Advances this [`Clock`] by `n` events for its own [`Party`], returning
    /// the new [`Version`]: byte-identical to `n` sequential
    /// [`tick`](Self::tick)s, computed in a bounded number of passes rather
    /// than `n`.
    ///
    /// The count `k` is any unsigned number, since all can be converted into
    /// [`Ticks`].
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_ticks.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::Ticks;
    /// let mut clock = before::Clock::seed();
    /// assert_eq!(clock.ticks(1_000_000u64).min_ticks(), Ticks::from(1_000_000u64));
    /// ```
    pub fn ticks(&mut self, k: impl Into<Ticks>) -> &Version {
        self.version.ticks(&self.party, k);
        &self.version
    }

    /// Splits off a child clock by [`fork`](Party::fork)ing the underlying
    /// [`Party`] and copying the underlying [`Version`].
    ///
    /// # Warning
    ///
    /// Repeatedly forking the same [`Clock`] produces an imbalanced internal
    /// representation, with worse memory use and performance. Prefer to vary
    /// which clock is forked, or use [`forks`](Clock::forks) to generate a
    /// fixed number of balanced forks.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/clock_fork.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let mut parent = Clock::seed();
    /// let child = parent.fork();
    /// assert!(parent.party().is_disjoint(child.party()));
    /// ```
    pub fn fork(&mut self) -> Clock {
        let child_party = self.party.fork();
        let child_version = self.version.clone();
        Clock::from_parts(child_party, child_version)
    }

    /// Splits `n` balanced child clocks off this [`Clock`], as a lazy
    /// [`ExactSizeIterator`].
    ///
    /// Prefer this to iterated [`fork`](Clock::fork), which would generate
    /// linearly growing [`Clock`] sizes, as opposed to the balanced,
    /// logarithmic sizes generated by this method.
    ///
    /// The iterator yields `n` children and `self` keeps the last share, so it
    /// stays a valid clock even once the iterator is fully drained. Forks not
    /// taken from the iterator before it drops have their party shares rejoined
    /// into `self`, so no `Party` is lost.
    ///
    /// For the consuming counterpart that splits into exactly `N` clocks, see
    /// [`From<Clock>`](Clock) for `[Clock; N]`.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/clock_forks.html"))]
    ///
    /// Children are built on demand; see [`Forks`] for the per-step and early-drop costs.
    ///
    /// # Example
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
    pub fn forks(&mut self, k: u64) -> Forks<'_> {
        Forks::new(self, k)
    }

    /// Absorbs a *disjoint* [`Clock`]'s [`Party`] and [`Version`], returning
    /// the new [`Version`] of `self`.
    ///
    /// # Errors
    ///
    /// If the two clocks' [`Party`]s overlap, `self` is unmodified and `other`
    /// is handed back in the error.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/clock_join.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let mut a = Clock::seed();
    /// let b = a.fork();
    /// // `a` and `b` are disjoint halves, so they rejoin into the whole.
    /// a.join(b).unwrap();
    /// assert!(a.party().is_seed());
    /// ```
    pub fn join(&mut self, other: Clock) -> Result<&Version, Clock> {
        let (other_party, other_version) = other.into_parts();
        match self.party.join(other_party) {
            Ok(()) => {
                self.version |= &other_version;
                Ok(self.version())
            }
            Err(other_party) => Err(Clock::from_parts(other_party, other_version)),
        }
    }

    /// Absorbs every disjoint [`Clock`] in `iter` into `self`, returning the
    /// merged [`Version`].
    ///
    /// # Errors
    ///
    /// Returns the clocks whose parties *overlapped* and so could not be folded
    /// in, dropping nothing: every input's party region and version are either
    /// merged into `self` or handed back. In case of partial error, the set of
    /// [`Clock`]s which are absorbed vs. handed back is unspecified.
    ///
    /// Unreachable for clocks descended from one [`seed`](Clock::seed): their
    /// parties are pairwise disjoint.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/clock_join_all.html"))]
    ///
    /// Auxiliary space is `O(|self| + |iter|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let mut parent = Clock::seed();
    /// let children: Vec<Clock> = parent.forks(3).collect();
    /// parent.join_all(children).unwrap(); // reabsorb the three children
    /// assert!(parent.party().is_seed()); // the whole seed region again
    /// ```
    #[allow(clippy::result_large_err)]
    pub fn join_all<I: IntoIterator<Item = Clock>>(
        &mut self,
        iter: I,
    ) -> Result<&Version, Vec<Clock>> {
        // The shared balanced binary counter (`crate::fold`), one join into
        // `self` per surviving group at the end: the same discipline as
        // [`Party::join_all`], because both of this fold's halves (the party
        // union and the version join) pay per-input scans of the whole
        // accumulated value under a left fold.
        //
        // Inputs overlapping `self` are handed back by the `accept` test
        // against the *fixed* `self` up front, through a per-call index of
        // `self`'s party (O(input) node visits plus the table searches per
        // input, as in [`Party::join_all`]); parties disjoint from `self` stay
        // disjoint from it however they coalesce, so the final joins cannot
        // fail on well-formed input. A failed combine is aliased input; the
        // counter's hand-back policy (`crate::fold`) drops nothing.
        let mut overlapping = Vec::new();
        let index = crate::party::ops::IdIndex::build(self.party.as_bits());
        let groups = crate::fold::balanced_try_fold(
            iter,
            |other| index.is_disjoint(other.party().view()),
            |mut top, incoming| match top.join(incoming) {
                Ok(_) => Ok(top),
                Err(back) => Err((top, back)),
            },
            &mut overlapping,
        );
        for group in groups {
            if let Err(back) = self.join(group) {
                overlapping.push(back);
            }
        }
        if overlapping.is_empty() {
            Ok(self.version())
        } else {
            Err(overlapping)
        }
    }

    /// Reconciles two *disjoint* [`Clock`]s, keeping both alive.
    ///
    /// This operation joins their [`Version`]s and re-[`fork`](Clock::fork)s
    /// the [`join`](Clock::join) of their [`Party`]s.
    ///
    /// # Errors
    ///
    /// If the [`Clock`]s' [`Party`]s overlap, an error is returned and `self`
    /// and `other` are left unmodified.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/clock_sync.html"))]
    ///
    /// # Example
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
        // One fused walk over the two parties emits both re-split halves
        // directly, and is also the overlap check: on overlap it emits nothing
        // and neither clock moves.
        let Some((keep, give)) = self.party.sum_split(&other.party) else {
            return Err(Overlap);
        };
        self.party = keep;
        other.party = give;

        // Both histories become the join of the two.
        self.version |= &other.version;
        other.version = self.version.clone();
        Ok(self.version())
    }

    /// Reconciles this [`Clock`] with every *disjoint* clock in `others`,
    /// keeping all alive.
    ///
    /// Prefer this to iteratatively calling [`sync`](Clock::sync), as this is
    /// more efficient, and re-splits the inner [`Party`] of each [`Clock`] to
    /// be maximally balanced, and therefore minimally large.
    ///
    /// # Errors
    ///
    /// If any two participants' [`Party`]s overlap, an error is returned and
    /// every clock is left unmodified.
    ///
    /// Unreachable for clocks descended from one [`seed`](Clock::seed):
    /// their parties are pairwise disjoint.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/clock_sync_all.html"))]
    ///
    /// Auxiliary space is `O(|self| + |iter|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let mut c = b.fork();
    /// a.tick();
    /// b.tick();
    /// a.sync_all([&mut b, &mut c]).unwrap(); // everyone learns everything
    /// assert!(a.version() == b.version() && b.version() == c.version());
    /// assert!(a.party().is_disjoint(b.party()));
    /// ```
    //
    // The combine closure's `Err` is the operand pair the counter keeps (the
    // fold's drop-nothing policy, as in `join_all`); boxing it would spend an
    // allocation on every refusal to dodge a by-value move.
    #[allow(clippy::result_large_err)]
    pub fn sync_all<'a, I>(&mut self, iter: I) -> Result<&Version, Overlap>
    where
        I: IntoIterator<Item = &'a mut Clock>,
    {
        let others: Vec<&'a mut Clock> = iter.into_iter().collect();

        // The `join_all` counter discipline run over aliases, byte-identical to
        // joining everything and re-forking the union.
        //
        // The fold consumes its operands, but on overlap every participant must
        // be left untouched, so the originals stay in their slots while `O(1)`
        // aliases carry the merge: on success the commit below overwrites every
        // original handle with a share of the union, and on overlap the merged
        // aliases drop with nothing observable moved.
        //
        // No up-front accept test against `self`: any overlap anywhere is a
        // whole-call error, and each one surfaces either as a lone rejected
        // input or as a failed join (in the counter, or against `self` in the
        // closing drain), so the per-input index `join_all` builds for its
        // hand-back accounting would buy nothing here.
        let mut rejected = Vec::new();
        let groups = crate::fold::balanced_try_fold(
            others.iter().map(|other| other.dangerously_alias()),
            |_| true,
            |mut top, incoming| match top.join(incoming) {
                Ok(_) => Ok(top),
                Err(back) => Err((top, back)),
            },
            &mut rejected,
        );
        if !rejected.is_empty() {
            return Err(Overlap);
        }
        let mut whole = self.dangerously_alias();
        for group in groups {
            if whole.join(group).is_err() {
                return Err(Overlap);
            }
        }

        // Commit: every handle becomes a balanced share of the union,
        // carrying the merged version.
        let shares = others.len() as u64;
        *self = whole;
        for (slot, child) in others.into_iter().zip(self.forks(shares)) {
            *slot = child;
        }
        Ok(self.version())
    }

    /// Exactly equivalent to [`tick`](Clock::tick), named for the case where
    /// another party will [`recv`](Clock::recv) the resulting [`Version`].
    ///
    /// When using [`Clock`]s as *vector clocks* rather than *version vectors*,
    /// mark communication by `send`ing a [`Version`] from the sender to the
    /// recipient, who [`recv`](Clock::recv)s it into their own [`Clock`].
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/clock_send.html"))]
    ///
    /// Exactly as [`tick`](Clock::tick).
    ///
    /// # Example
    ///
    /// ```
    /// let mut clock = before::Clock::seed();
    /// let start = clock.version().clone();
    /// let msg = clock.send().clone(); // tick, then hand the version to a peer
    /// assert!(msg > start); // the send is itself an event
    /// ```
    pub fn send(&mut self) -> &Version {
        self.tick()
    }

    /// Merges a received [`Version`] into this [`Clock`]'s version, then
    /// [`tick`](Clock::tick)s the [`Clock`].
    ///
    /// Equivalent to [`absorb`](Clock::absorb)ing the version and then
    /// [`tick`](Clock::tick)ing. The receiving half of the vector-clock
    /// communication pattern described on [`send`](Clock::send).
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/clock_recv.html"))]
    ///
    /// # Example
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
        self.absorb(version);
        self.tick()
    }

    /// Merges a received [`Version`] into this [`Clock`]'s version, without
    /// marking an event, and returns the new version.
    ///
    /// Identical to the operator form `self |= version`.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_join.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let msg = a.send().clone();
    /// b.absorb(&msg); // learn a's history without marking an event
    /// assert!(*b.version() >= msg);
    /// // Already seen: absorbing the same message again changes nothing.
    /// let seen = b.version().clone();
    /// b.absorb(&msg);
    /// assert_eq!(*b.version(), seen);
    /// ```
    pub fn absorb(&mut self, version: &Version) -> &Version {
        self.version |= version;
        &self.version
    }

    /// Merges any number of received [`Version`]s into this [`Clock`]'s
    /// version, then [`tick`](Clock::tick)s the [`Clock`] (once).
    ///
    /// Prefer this to iteratively [`recv`](Clock::recv)-ing [`Version`]s
    /// one-at-a-time, which is less efficient than this method.
    ///
    /// Equivalent to [`absorb_all`](Clock::absorb_all) and then
    /// [`tick`](Clock::tick)ing once. The n-ary half of the vector-clock
    /// communication pattern described on [`send`](Clock::send).
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/clock_recv_all.html"))]
    ///
    /// Auxiliary space is `O(|self| + |iter|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let mut c = b.fork();
    /// let (m1, m2) = (b.send().clone(), c.send().clone());
    /// a.recv_all([&m1, &m2]); // absorb both histories, then tick once
    /// assert!(*a.version() > m1 && *a.version() > m2);
    /// ```
    pub fn recv_all<I>(&mut self, iter: I) -> &Version
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        self.absorb_all(iter);
        self.tick()
    }

    /// Merges any number of received [`Version`]s into this [`Clock`]'s
    /// version, without marking an event, and returning the new version.
    ///
    /// Prefer this to iteratively [`absorb`](Clock::absorb)ing [`Version`]s
    /// one-at-a-time, which is less efficient than this method.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_join_all.html"))]
    ///
    /// Auxiliary space is `O(|self| + |iter|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let mut c = b.fork();
    /// let (m1, m2) = (b.send().clone(), c.send().clone());
    /// a.absorb_all([&m1, &m2]); // learn both histories; no event of a's own
    /// assert!(*a.version() >= m1 && *a.version() >= m2);
    /// ```
    pub fn absorb_all<I>(&mut self, iter: I) -> &Version
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        self.version = self.version.join_all(iter);
        &self.version
    }

    /// Pairs a [`Party`] with a [`Version`] to form a [`Clock`].
    ///
    /// Inverse to [`into_parts`](Clock::into_parts).
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/clock_from_parts.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Party, Version};
    /// let clock = Clock::from_parts(Party::seed(), Version::new());
    /// assert!(clock.party().is_seed());
    /// assert!(clock.version().is_empty());
    /// ```
    pub fn from_parts(party: Party, version: Version) -> Self {
        Clock { party, version }
    }

    /// Decomposes a [`Clock`] into its [`Party`] and [`Version`].
    ///
    /// Inverse to [`Clock::from_parts`].
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/clock_into_parts.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let (party, version) = Clock::seed().into_parts();
    /// assert!(party.is_seed());
    /// assert!(version.is_empty());
    /// ```
    pub fn into_parts(self) -> (Party, Version) {
        (self.party, self.version)
    }

    /// The [`Party`] whose causal history this clock tracks.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// assert!(before::Clock::seed().party().is_seed());
    /// ```
    pub fn party(&self) -> &Party {
        &self.party
    }

    /// The current state of the [`Clock`], as a [`Version`].
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// assert!(before::Clock::seed().version().is_empty());
    /// ```
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// The *slice* of this clock's [`Version`] which is owned by its own
    /// [`Party`].
    ///
    /// Equivalent to `self.version() / self.party()`.
    ///
    /// The returned view compares directly (against a [`Version`] or another
    /// [`OwnVersion`]).
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/clock_own_version.html"))]
    ///
    /// # Example
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
    /// // But the meet of the two projected versions (materialized: the
    /// // meet needs the objects) is not:
    /// let (own_a, own_b) = (a.own_version().to_version(), b.own_version().to_version());
    /// assert!(own_a & own_b == Version::new());
    /// ```
    pub fn own_version(&self) -> OwnVersion<'_> {
        self.version() / self.party()
    }

    /// Encodes this [`Clock`] as canonical bytes.
    ///
    /// The bytes are the [`Party`]'s encoding followed by the [`Version`]'s.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/clock_encode.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let clock = Clock::seed();
    /// let bytes = clock.encode();
    /// assert_eq!(Clock::decode(&bytes[..]).unwrap(), clock);
    /// // The framing: the party's bytes, then the version's.
    /// assert_eq!(bytes, [clock.party().encode(), clock.version().encode()].concat());
    /// ```
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.encode_to(&mut bytes)
            .expect("writing to a Vec is infallible");
        bytes
    }

    /// Encodes this [`Clock`]'s canonical bytes to an arbitrary writer.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/clock_encode.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let mut buf = Vec::new();
    /// Clock::seed().encode_to(&mut buf).unwrap();
    /// assert_eq!(buf, Clock::seed().encode());
    /// ```
    pub fn encode_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // The clock's bytes are the byte-aligned [`Party`] encoding followed by
        // the byte-aligned [`Version`] encoding. Each part is independently
        // canonical and the party is self-delimiting (a decoder parses its id
        // to find the split), so the two concatenate with no bit-level packing.
        self.party.encode_to(writer)?;
        self.version.encode_to(writer)
    }

    /// Decodes a [`Clock`] from a reader of canonical bytes, strictly rejecting
    /// malformed or non-canonical input.
    ///
    /// # Warning
    ///
    /// Serializing a [`Clock`] circumvents its otherwise compiler-enforced
    /// `!Clone` linearity. Deserializing one can violate causality. Treat
    /// serialization/deserialization boundaries as *moves* of the [`Clock`].
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/clock_decode.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let bytes = Clock::seed().encode();
    /// let clock = Clock::decode(&bytes[..]).unwrap();
    /// assert!(clock.party().is_seed());
    /// assert!(clock.version().is_empty());
    /// ```
    pub fn decode<R: Read>(mut reader: R) -> Result<Self, Decode> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).map_err(Decode::Io)?;
        // The party is the byte-aligned prefix: parse its id once to find the
        // split, then validate both components against the borrowed buffer —
        // the party's padding first, then the version's stream and padding, the
        // order the component decoders check. The id grammar has no empty
        // production, so the party is a nonzero share and empty input is
        // exhausted input. Both parts then adopt slices of the ONE read buffer
        // as their storage: no per-component copy, and the id is parsed once
        // where handing byte ranges to the component decoders re-parsed it.
        // Each walk's input is its component's whole byte range as bits,
        // padding included, judged by its marker check.
        let id_bytes = {
            let id_end = codec::parse_id(codec::BitsView::whole(&buf), 0)?;
            // The party's padding marker rides in its final byte — which an
            // input cut right after a flush id tree lacks. That cut is
            // missing required data (the marker byte, and the whole version
            // after it): the truncation genre, exactly as a byte-starved
            // reader reports the same boundary.
            let id_bytes = (id_end + 1).div_ceil(8);
            if id_bytes > buf.len() as u64 {
                return Err(Decode::Truncated);
            }
            let id_bytes =
                usize::try_from(id_bytes).expect("the id prefix ends within the read buffer");
            codec::require_marker_padding(&buf[..id_bytes], id_end)?;
            let tail = &buf[id_bytes..];
            let v_end = crate::version::skyline::validate_prefix(codec::BitsView::whole(tail))?;
            codec::require_marker_padding(tail, v_end)?;
            id_bytes
        };
        let buf = bytes::Bytes::from(buf);
        let party = Party::from_frozen(codec::Bits::from_canonical(buf.slice(..id_bytes)));
        let version = Version::from_frozen(codec::Bits::from_canonical(buf.slice(id_bytes..)));
        Ok(Clock::from_parts(party, version))
    }

    /// The exact length in bits of [`encode`](Self::encode) before the final
    /// byte's padding — the marker bit and zero-pad to the byte boundary, so
    /// `encode().len()` is `(encoded_bits() + 1).div_ceil(8)`.
    ///
    /// The encoding byte-concatenates the [`Party`] and [`Version`] (see
    /// [`encode`](Self::encode)), so the party occupies whole bytes — its own
    /// padding included — and only the version's final byte is left to pad:
    /// this is the byte-aligned party length plus the version's own bit
    /// length.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let clock = Clock::seed();
    /// assert_eq!(clock.encode().len(), (clock.encoded_bits() + 1).div_ceil(8));
    /// ```
    pub fn encoded_bits(&self) -> usize {
        // u64 width: on a 32-bit target each component's bit length fits
        // usize (the storable bound), but the byte-rounded party plus the
        // version can exceed it — a clock is two independently bounded
        // streams. The conversion is checked, so a composite too long for
        // this target's usize fails loudly by name instead of wrapping.
        let party = 8 * (self.party().encoded_bits() as u64 + 1).div_ceil(8);
        let bits = party + self.version().encoded_bits() as u64;
        usize::try_from(bits).expect("the clock's combined bit length fits usize")
    }

    /// Duplicates this clock, producing a second handle to the same clock: an
    /// **intentional violation of linearity**.
    ///
    /// # Warning
    ///
    /// [`Clock`] is [`!Clone`](Clone) because two live handles to one [`Clock`]
    /// break disjointness of the underlying [`Party`], so if both copies (or
    /// any of their [`fork`](Clock::fork)s) go on to [`tick`](Clock::tick) or
    /// [`join`](Clock::join), causal history can be corrupted arbitrarily.
    ///
    /// The caller must ensure that at most one of the two copies is ever
    /// treated as live; the other must be dropped without further use. The same
    /// rule applies to any [`Party`] extracted from such a clock.
    ///
    /// This method exists for handing a clock across a boundary where ownership
    /// transfers to exactly one side based on an outcome not known at the time
    /// of transfer.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
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

/// Notation from the original paper: `(<id>, <event>)`, e.g. `(1, 0)` for
/// [`Clock::seed`].
///
/// # Complexity
///
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/clock_display.html"))]
///
/// # Example
///
/// ```
/// assert_eq!(before::Clock::seed().to_string(), "(1, 0)");
/// ```
impl Display for Clock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "({}, {})", self.party, self.version)
    }
}

impl Debug for Clock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Clock")
            .field("party", &self.party)
            .field("version", &self.version)
            .finish()
    }
}

/// Parses a stamp `(i, e)` in the notation from the original paper, strictly
/// rejecting non-normal-form input and any anonymous (id `0`) party.
///
/// Parsing *creates* the clock's party, tied to no existing handle: `"(1,
/// 0)".parse()` yields a clock whose party overlaps every seed's whole region.
///
/// # Complexity
///
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/clock_fromstr.html"))]
///
/// # Example
///
/// ```
/// use before::Clock;
/// let clock: Clock = "(1, 0)".parse().unwrap();
/// assert_eq!(clock.to_string(), "(1, 0)");
/// ```
impl FromStr for Clock {
    type Err = Parse;
    fn from_str(s: &str) -> Result<Self, Parse> {
        let (id, ev) = codec::parse_clock_str(s)?;
        let version: Version = ev.parse()?;
        if codec::id_is_empty(codec::built_view(&id)) {
            return Err(Parse::Anonymous);
        }
        Ok(Clock::from_parts(Party::from_bits(id), version))
    }
}

/// A clock from a `(party, version)` literal, e.g. `Clock::try_from(((1, 0),
/// 5))`.
///
/// Creates the clock's party tied to no existing handle, like parsing it from
/// text via [`FromStr`].
///
/// # Complexity
///
/// `O(n)`, `n` the built clock's size in bytes.
///
/// # Example
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

// The join operators for `Clock` over {Clock, Version}: `|` merges a `Version`
// into a clock (on either side, since a `Version` carries no party) and returns
// the clock; `|=` merges in place (`Clock::absorb` is the named spelling of
// both). There is no `Clock | Clock`: a borrowing
// form would duplicate the clock's party, and reuniting two whole clocks is the
// fallible `Clock::join`. Every cell folds the version operand into the clock's
// `version` through the `Version` join-assign; `Borrow::borrow` coerces an
// owned or borrowed operand uniformly to `&Version`, so one `@cell` arm per
// position covers both forms.

/// Generates the `Clock` join matrix.
///
/// A `|` cell owns its clock operand (whichever side it is on) and returns it;
/// a `|=` cell merges into the receiver in place. Each position — `op_l`/`op_r`
/// for the clock as the left or right `|` operand, `as_clock` for the `|=`
/// receiver — has its own `@cell` arm so the receiver `self` is written in the
/// same expansion as the method it belongs to (`self` cannot cross a
/// macro-invocation boundary).
macro_rules! clock_join_matrix {
    ($island:literal, $opdoc:literal, $($kind:tt $lhs:ty, $rhs:ty);* $(;)?) => {
        $( clock_join_matrix!(@cell $island, $opdoc, $kind $lhs, $rhs); )*
    };
    (@cell $island:literal, $opdoc:literal, op_l $lhs:ty, $rhs:ty) => {
        #[doc = $opdoc]
        #[doc = ""]
        #[doc = "# Complexity"]
        #[doc = ""]
        #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/", $island, ".html"))]
        impl BitOr<$rhs> for $lhs {
            type Output = Clock;
            fn bitor(mut self, r: $rhs) -> Clock {
                self.version |= r.borrow();
                self
            }
        }
    };
    (@cell $island:literal, $opdoc:literal, op_r $lhs:ty, $rhs:ty) => {
        #[doc = $opdoc]
        #[doc = ""]
        #[doc = "# Complexity"]
        #[doc = ""]
        #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/", $island, ".html"))]
        impl BitOr<$rhs> for $lhs {
            type Output = Clock;
            fn bitor(self, mut r: $rhs) -> Clock {
                r.version |= self.borrow();
                r
            }
        }
    };
    (@cell $island:literal, $opdoc:literal, as_clock $lhs:ty, $rhs:ty) => {
        #[doc = $opdoc]
        #[doc = ""]
        #[doc = "# Complexity"]
        #[doc = ""]
        #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/", $island, ".html"))]
        impl BitOrAssign<$rhs> for $lhs {
            fn bitor_assign(&mut self, r: $rhs) {
                self.version |= r.borrow();
            }
        }
    };
}

clock_join_matrix! {
    "version_join",
    "`clock | version` (in either operand order) and `clock |= version`: merge a received [`Version`] without marking an event, as [`Clock::absorb`].",
    op_l     Clock,    Version;
    op_l     Clock,    &Version;
    op_r     Version,  Clock;
    op_r     &Version, Clock;
    as_clock Clock,    Version;
    as_clock Clock,    &Version;
}

// The matrix above is the entire `|` surface a `Clock` participates in, and
// its shape is load-bearing:
//
// - No cell reunites two clocks, in any borrow shape: `Clock::join` is the
//   only reunion, fallible because it must verify the parties are disjoint,
//   and an infallible `|` would silently merge overlapping shares.
// - No cell borrows its clock operand: the `clock | version` impls are
//   carried by `Clock` itself, `BitOr::bitor` takes its receiver by value,
//   and `Clock` is `!Copy` (pinned at the definition) — so `|` moves the
//   clock, and any later use of it is rejected by the compiler as a
//   use-after-move. An impl carried by `&Clock` would instead hand back a
//   merged clock while the operand stayed live: two holders of one share.
static_assertions::assert_impl_all!(
    Clock: BitOr<Version, Output = Clock>,
    BitOr<&'static Version, Output = Clock>
);
static_assertions::assert_not_impl_any!(Clock: BitOr<Clock>, BitOr<&'static Clock>);
static_assertions::assert_not_impl_any!(
    &'static Clock: BitOr<Clock>,
    BitOr<&'static Clock>,
    BitOr<Version>,
    BitOr<&'static Version>
);
