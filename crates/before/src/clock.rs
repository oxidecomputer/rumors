//! A [`Clock`] is a [`Party`] paired with a [`Version`].

use core::borrow::Borrow;
use core::ops::{BitOr, BitOrAssign};

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
/// # Complexity
///
/// A clock's *packed size* `|c|` is the length of [`encode`](Clock::encode)'s
/// bytes (its [`Party`]'s and [`Version`]'s packed forms concatenated). Every
/// _Complexity_ section on this type's operations is denominated in packed
/// sizes.
///
/// Joining a [`Version`] into a clock (`|`, `|=`, either operand order) costs
/// the two operands' packed sizes; `==` and hashing read the packed clock once;
/// each remaining cost is on its operation.
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
    /// assert_eq!(before::Clock::seed().to_string(), "(1, 0)");
    /// ```
    pub fn seed() -> Self {
        Self::from_parts(Party::seed(), Version::new())
    }

    /// Advances this [`Clock`] by one event for its own [`Party`], returning
    /// the new [`Version`].
    ///
    /// # Complexity
    ///
    /// `O(|self|)`.
    ///
    /// # Example
    ///
    /// ```
    /// let mut clock = before::Clock::seed();
    /// assert_eq!(clock.tick().to_string(), "1");
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
    /// The count `n` is any unsigned number, since all can be converted into
    /// [`Ticks`].
    ///
    /// # Complexity
    ///
    /// `O(|self| + log n)`.
    ///
    /// # Example
    ///
    /// ```
    /// let mut clock = before::Clock::seed();
    /// assert_eq!(clock.ticks(1_000_000u64).to_string(), "1000000");
    /// ```
    pub fn ticks(&mut self, n: impl Into<Ticks>) -> &Version {
        self.version.ticks(&self.party, n);
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
    /// `O(|self|)`. The party splits in linear time and the version is
    /// `O(1)`-cloned into the child.
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
    /// The clock analogue of [`Party::forks`]: one balanced split of the
    /// underlying [`Party`] into `n + 1` shares of minimal-depth (`⌈log₂(n +
    /// 1)⌉`) id tree, each child carrying a clone of this clock's [`Version`]
    /// (as [`fork`](Clock::fork) does).
    ///
    /// The iterator yields `n` children and `self` keeps the last share, so it
    /// stays a valid clock even once the iterator is fully drained; children
    /// not taken before the iterator drops have their party shares rejoined
    /// into `self`, so no `Party` is lost. Prefer this to repeated
    /// [`fork`](Clock::fork), which deepens one spine into a linear tree
    /// structure.
    ///
    /// For the consuming counterpart that splits into exactly `N` clocks, see
    /// [`From<Clock>`](Clock) for `[Clock; N]`.
    ///
    /// # Complexity
    ///
    /// `O(S + n)`: the party split plus one `O(1)` version clone per child. For
    /// a full drain, `S` is the total packed size of the party shares and each
    /// of the `n` children clones the version by sharing its stored buffer;
    /// children are built on demand (see [`Forks`]).
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
    pub fn forks(&mut self, n: u64) -> Forks<'_> {
        Forks::new(self, n)
    }

    /// Absorbs a *disjoint* [`Clock`]'s [`Party`] and [`Version`], returning
    /// the new [`Version`] of `self`.
    ///
    /// # Errors
    ///
    /// If the two clocks' [`Party`]s overlap, `self` is unmodified and
    /// `other` is handed back in the error.
    ///
    /// # Complexity
    ///
    /// `O(a + b)`, regardless of whether or not an error is returned.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let mut a = Clock::seed();
    /// let b = a.fork();
    /// // `a` and `b` are disjoint halves, so they rejoin into the whole.
    /// a.join(b).unwrap();
    /// assert_eq!(a.party().to_string(), "1");
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
    /// The collective form of [`join`](Clock::join): `self` seeds the fold, so
    /// an empty `iter` is a no-op returning `self`'s current version. The
    /// "reabsorb this whole set of retired peers" primitive.
    ///
    /// Best-effort: every clock whose [`Party`] is disjoint from the region
    /// accumulated so far has its party reunited and its [`Version`] merged
    /// into `self`.
    ///
    /// # Errors
    ///
    /// Returns the clocks whose parties *overlapped* and so could not be folded
    /// in, dropping nothing: every input's party region and version are either
    /// merged into `self` or handed back. In case of partial error, which
    /// [`Clock`]s are absorbed vs. handed back is unspecified.
    ///
    /// Unreachable for clocks descended from one [`seed`](Clock::seed): their
    /// parties are pairwise disjoint.
    ///
    /// # Complexity
    ///
    /// `O((|c| + |i|) log n)` time, `O(|c| + |i|)` auxiliary space, where `|i|`
    /// is the total packed size of the inputs, and `n` the number of inputs.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let mut parent = Clock::seed();
    /// let children: Vec<Clock> = parent.forks(3).collect();
    /// parent.join_all(children).unwrap(); // reabsorb the three children
    /// assert_eq!(parent.party().to_string(), "1"); // the whole seed region again
    /// ```
    //
    // The combine closure's `Err` is the hand-back pair itself — both operands
    // returned to the caller on an aliased input, the fold's drop-nothing
    // policy — so its size is the two clocks it preserves. Boxing would spend
    // an allocation on every refusal to dodge a by-value move, a trade against
    // the policy the error exists for.
    #[allow(clippy::result_large_err)]
    pub fn join_all<I: IntoIterator<Item = Clock>>(
        &mut self,
        iter: I,
    ) -> Result<&Version, Vec<Clock>> {
        // The shared balanced binary counter (`crate::fold`), one join into
        // `self` per surviving group at the end — the same discipline as
        // [`Party::join_all`], because both of this fold's halves (the party
        // union and the version join) pay per-input scans of the whole
        // accumulated value under a left fold. Inputs overlapping `self` are
        // handed back by the `accept` test against the *fixed* `self` up front,
        // through a per-call index of `self`'s party (O(input) node visits plus
        // the table searches per input, as in [`Party::join_all`]); parties
        // disjoint from `self` stay disjoint from it however they coalesce, so
        // the final joins cannot fail on well-formed input. A failed combine is
        // aliased input; the counter's hand-back policy (`crate::fold`) drops
        // nothing.
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
    /// `O(|c| + |d|)`, regardless of whether or not an error is returned.
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
        // directly — byte-identical to joining them and forking the union (the
        // `sync_is_join_then_fork` law pins the equality) — and is also the
        // overlap check: on overlap it emits nothing and neither clock moves.
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

    /// Reconciles this [`Clock`] with every clock in `others`, keeping all
    /// alive.
    ///
    /// The collective form of [`sync`](Clock::sync): every participant —
    /// `self` and each of the `others` — ends holding the join of all their
    /// [`Version`]s over one share of the union of all their [`Party`]s,
    /// re-shared as [`forks`](Clock::forks) re-shares: `self` keeps the
    /// residual share and each of the `others` receives one balanced,
    /// minimal-depth share in iteration order, however imbalanced the
    /// inputs were. An empty `others` is a no-op returning `self`'s current
    /// version.
    ///
    /// # Errors
    ///
    /// If any two participants' [`Party`]s overlap, an error is returned
    /// and every clock — `self` and all of `others` — is left unmodified.
    ///
    /// Unreachable for clocks descended from one [`seed`](Clock::seed):
    /// their parties are pairwise disjoint.
    ///
    /// # Complexity
    ///
    /// `O(D log k)` time, `O(D)` space, plus the re-share's `O(S + k)`.
    /// `D` is the participants' total packed size and `k` their count;
    /// the re-share is the balanced split, `S` its shares' total packed
    /// size.
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
    pub fn sync_all<'a>(
        &mut self,
        others: impl IntoIterator<Item = &'a mut Clock>,
    ) -> Result<&Version, Overlap> {
        let others: Vec<&'a mut Clock> = others.into_iter().collect();

        // The `join_all` counter discipline run over aliases, byte-identical
        // to joining everything and re-forking the union (the
        // `sync_all_is_join_all_then_forks` law pins the equality to that
        // composed spelling). The fold consumes its operands, but on overlap
        // every participant must be left untouched, so the originals stay in
        // their slots while `O(1)` aliases carry the merge —
        // `dangerously_alias`'s boundary case, ownership resolving to exactly
        // one side per outcome: on success the commit below overwrites every
        // original handle with a share of the union (the merged alias
        // supersedes them all at once), and on overlap the merged aliases
        // drop with nothing observable moved. No up-front accept test
        // against `self`: any overlap anywhere is a whole-call error, and
        // each one surfaces either as a lone rejected input or as a failed
        // join (in the counter, or against `self` in the closing drain), so
        // the per-input index `join_all` builds for its hand-back accounting
        // would buy nothing here.
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

    /// Equivalent to [`tick`](Clock::tick), named for the case where another
    /// party will [`recv`](Clock::recv) the resulting [`Version`].
    ///
    /// When using [`Clock`]s as *vector clocks* rather than *version vectors*,
    /// mark communication by `send`ing a [`Version`] from the sender to the
    /// recipient, who [`recv`](Clock::recv)s it into their own [`Clock`].
    ///
    /// # Complexity
    ///
    /// `O(n)`, exacctly as [`tick`](Clock::tick).
    ///
    /// # Example
    ///
    /// ```
    /// let mut clock = before::Clock::seed();
    /// let msg = clock.send().clone(); // tick, then hand the version to a peer
    /// assert_eq!(msg.to_string(), "1");
    /// ```
    pub fn send(&mut self) -> &Version {
        self.tick()
    }

    /// Merges a received [`Version`] into this [`Clock`]'s version, then
    /// [`tick`](Clock::tick)s the [`Clock`].
    ///
    /// Equivalent to `self |= version; self.tick()`. The receiving half of
    /// the vector-clock communication pattern described on
    /// [`send`](Clock::send).
    ///
    /// # Complexity
    ///
    /// `O(a + b)`. One join, then one tick.
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
        self.version |= version;
        self.tick()
    }

    /// Merges any number of received [`Version`]s into this [`Clock`]'s
    /// version, then [`tick`](Clock::tick)s the [`Clock`] (once).
    ///
    /// Equivalent to `clock |= Version::join_all(versions); clock.tick()`:
    /// the batch is absorbed whole and recorded as one local event. The
    /// n-ary half of the vector-clock communication pattern described on
    /// [`send`](Clock::send).
    ///
    /// When possible, prefer this to iteratively [`recv`](Clock::recv)-ing
    /// [`Version`]s one-at-a-time, which is less efficient than this method.
    /// That is to say:
    ///
    /// ```ignore
    /// // Don't do this!
    /// for v in versions {
    ///     clock.recv(v);
    /// }
    ///
    /// // Instead, do this:
    /// clock.recv_all(versions);
    /// ```
    ///
    /// # Complexity
    ///
    /// `O(n + D log k)` time, `O(n + D)` space. `n` is the clock's packed
    /// size, `D` the messages' total packed size, and `k` their number:
    /// the messages ride the balanced [`Version::join_all`] fold, then one
    /// join and one tick land the result.
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
    pub fn recv_all<I>(&mut self, versions: I) -> &Version
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        self.version |= Version::join_all(versions);
        self.tick()
    }

    /// Pairs a [`Party`] with a [`Version`] to form a [`Clock`].
    ///
    /// Any version pairs with the party: a clock rebuilt over an earlier
    /// version re-mints successors the party's later ticks already
    /// produced, and that is valid — a version records causal knowledge,
    /// not event identity. [`into_parts`](Clock::into_parts) and back is
    /// always safe.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// ```
    /// use before::{Clock, Party, Version};
    /// let clock = Clock::from_parts(Party::seed(), Version::new());
    /// assert_eq!(clock.to_string(), "(1, 0)");
    /// ```
    pub fn from_parts(party: Party, version: Version) -> Self {
        Clock { party, version }
    }

    /// Decomposes a [`Clock`] into its [`Party`] and [`Version`].
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// ```
    /// use before::Clock;
    /// let (party, version) = Clock::seed().into_parts();
    /// assert_eq!(party.to_string(), "1");
    /// assert_eq!(version.to_string(), "0");
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
    /// ```
    /// assert_eq!(before::Clock::seed().party().to_string(), "1");
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
    /// ```
    /// assert_eq!(before::Clock::seed().version().to_string(), "0");
    /// ```
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// The *slice* of this clock's [`Version`] owned by its own [`Party`],
    /// as the lazy [`OwnVersion`] view.
    ///
    /// This is short for `self.version() / self.party()`. The view
    /// compares directly (against a [`Version`] or another view); the
    /// projected [`Version`] itself exists only through the explicit
    /// [`OwnVersion::to_version`], whose result can outgrow the clock.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    /// The view borrows the clock's parts. Every cost lives on the view's
    /// operations ([`OwnVersion`]'s doc carries them).
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
    /// The bytes are the [`Party`]'s encoding followed by the [`Version`]'s:
    /// each part is byte-aligned and independently canonical, and the party
    /// is self-delimiting, so the two concatenate with no length prefix.
    ///
    /// # Complexity
    ///
    /// `O(n)`.
    /// One copy of each part's stored bytes.
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
    /// `O(n)`.
    /// One write of each part's stored bytes, plus whatever the writer
    /// itself costs.
    ///
    /// ```
    /// use before::Clock;
    /// let mut buf = Vec::new();
    /// Clock::seed().encode_to(&mut buf).unwrap();
    /// assert_eq!(buf, Clock::seed().encode());
    /// ```
    pub fn encode_to<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // The clock's bytes are the byte-aligned [`Party`] encoding followed by
        // the byte-aligned [`Version`] encoding. Each part is independently
        // canonical and the party is self-delimiting (a decoder parses its id to
        // find the split), so the two concatenate with no bit-level packing —
        // at the cost of at most one padding byte between them. Decoding then
        // reuses `Party::decode`/`Version::decode` on the two byte ranges.
        self.party.encode_to(writer)?;
        self.version.encode_to(writer)
    }

    /// Decodes a [`Clock`] from a reader of canonical bytes, strictly
    /// rejecting malformed or non-canonical input.
    ///
    /// A decoded clock is a *second holder* of whatever identity the
    /// bytes spell: nothing ties bytes to their source, so decoding
    /// while the original handle — or any other copy of the bytes — can
    /// still act violates linearity. Treat encode-then-decode as a
    /// move, and restore an identity only from its latest persisted
    /// state ([Safety rules](crate#safety-rules)).
    ///
    /// # Complexity
    ///
    /// `O(n)`.
    /// `n` is the bytes read, accepted or rejected: strict validation is
    /// one pass over each part's stream.
    ///
    /// ```
    /// use before::Clock;
    /// let bytes = Clock::seed().encode();
    /// assert_eq!(Clock::decode(&bytes[..]).unwrap().to_string(), "(1, 0)");
    /// ```
    pub fn decode<R: std::io::Read>(mut reader: R) -> Result<Self, Decode> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).map_err(Decode::Io)?;
        // The party is the byte-aligned prefix: parse its id once to
        // find the split, then validate both components against the
        // borrowed buffer — the party's padding first, then the
        // version's stream and padding, the order the component
        // decoders check. The id grammar has no empty production, so
        // the party is a nonzero share (paper §3: a standalone share is
        // `i ≠ 0`) and empty input is exhausted input. Both parts then
        // adopt slices of the ONE read buffer as their storage: no
        // per-component copy, and the id is parsed once where handing
        // byte ranges to the component decoders re-parsed it.
        let (id_end, id_bytes, v_end) = {
            let bits = codec::bytes_as_bits(&buf);
            let id_end = codec::parse_id(bits, 0)?;
            let id_bytes = id_end.div_ceil(8);
            codec::require_zero_padding(&bits[..8 * id_bytes], id_end)?;
            let tail = &bits[8 * id_bytes..];
            let v_end = crate::version::skyline::validate_prefix(tail)?;
            codec::require_zero_padding(tail, v_end)?;
            (id_end, id_bytes, v_end)
        };
        let buf = bytes::Bytes::from(buf);
        let party = Party::from_frozen(codec::Bits::from_canonical(buf.slice(..id_bytes), id_end));
        let version =
            Version::from_frozen(codec::Bits::from_canonical(buf.slice(id_bytes..), v_end));
        Ok(Clock::from_parts(party, version))
    }

    /// The exact length in bits of [`encode`](Self::encode), not counting the
    /// final byte's zero-pad.
    ///
    /// The encoding byte-concatenates the [`Party`] and [`Version`] (see
    /// [`encode`](Self::encode)), so the party occupies whole bytes and only the
    /// version's last byte is padded: this is the byte-aligned party length plus
    /// the version's own bit length.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// ```
    /// use before::Clock;
    /// let clock = Clock::seed();
    /// assert_eq!(clock.encode().len(), clock.encoded_bits().div_ceil(8));
    /// ```
    pub fn encoded_bits(&self) -> usize {
        8 * self.party().encoded_bits().div_ceil(8) + self.version().encoded_bits()
    }

    /// Duplicates this clock, producing a second handle to the same clock, in
    /// violation of linearity.
    ///
    /// # Warning
    ///
    /// [`Clock`] is [`!Clone`](Clone) because two live handles to one region
    /// break the Law of Disjointness: the alias's [`Party`] is not
    /// [disjoint](Party::is_disjoint) from the original, so if both copies (or
    /// any of their [`fork`](Clock::fork)s) go on to [`tick`](Clock::tick) or
    /// [`join`](Clock::join), causal history can be corrupted arbitrarily. The
    /// caller must ensure that at most one of the two copies is ever treated as
    /// live; the other must be dropped without further use. The same rule
    /// applies to any [`Party`] built from such a clock.
    ///
    /// This method exists for handing a clock across a boundary where ownership
    /// transfers to exactly one side based on an outcome not known at the time
    /// of transfer.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    /// Each part's alias shares its stored buffer (the at-rest form is
    /// refcounted), which is safe *for storage* because no operation
    /// mutates a stored stream in place — the linearity hazard above is
    /// about causal identity, not bytes.
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

/// Paper notation: `(<id>, <event>)`, e.g. `(1, 0)` for [`Clock::seed`].
///
/// # Complexity
///
/// `O(n + t)` space; time superlinear on the version side (as `Version`'s
/// `Display`).
/// The version side costs as [`Version`]'s `Display` (value conversion plus
/// the renderer's summary merge); the party side is linear.
///
/// ```
/// assert_eq!(before::Clock::seed().to_string(), "(1, 0)");
/// ```
impl core::fmt::Display for Clock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "({}, {})", self.party, self.version)
    }
}

impl core::fmt::Debug for Clock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Clock")
            .field("party", &self.party)
            .field("version", &self.version)
            .finish()
    }
}

/// Parses a stamp `(i, e)` in paper notation, strictly rejecting
/// non-normal-form
/// input and any anonymous (id `0`) party.
///
/// Parsing *mints* the clock's party, tied to no existing handle:
/// `"(1, 0)".parse()` yields a clock whose party overlaps every seed's
/// whole region. Text mints belong in tests and fresh universes; inside
/// a live system, identity comes only from
/// [`fork`](Clock::fork)/[`join`](Clock::join)
/// ([Safety rules](crate#safety-rules)).
///
/// # Complexity
///
/// `O(t + n)` space; time superlinear in the spelled value widths
/// (decimal-to-binary conversion).
/// The bound holds accepted or rejected, except that each spelled value
/// wider than a machine word pays decimal-to-binary conversion, superlinear
/// (though subquadratic) in that value's width.
///
/// ```
/// use before::Clock;
/// let clock: Clock = "(1, 0)".parse().unwrap();
/// assert_eq!(clock.to_string(), "(1, 0)");
/// ```
impl core::str::FromStr for Clock {
    type Err = Parse;
    fn from_str(s: &str) -> Result<Self, Parse> {
        let (id, ev) = codec::parse_clock_str(s)?;
        let version: Version = ev.parse()?;
        if codec::id_is_empty(&id) {
            return Err(Parse::Anonymous);
        }
        Ok(Clock::from_parts(Party::from_bits(id), version))
    }
}

/// A clock from a `(party, version)` literal, e.g.
/// `Clock::try_from(((1, 0), 5))`.
///
/// Mints the clock's party tied to no existing handle, like the text
/// door — a test and fresh-universe door
/// ([Safety rules](crate#safety-rules)).
///
/// # Complexity
///
/// `O(n)`.
/// `n` is the packed clock built.
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

// The join operators for `Clock` over {Clock, Version}: `|` merges a
// `Version` into a clock (on either side, since a `Version` carries no
// party) and returns the clock; `|=` merges in place. There is no
// `Clock | Clock`: a borrowing form would duplicate the clock's party, and
// reuniting two whole clocks is the fallible `Clock::join`. Every cell folds
// the version operand into the clock's `version` through the `Version`
// join-assign; `Borrow::borrow` coerces an owned or borrowed operand
// uniformly to `&Version`, so one `@cell` arm per position covers both
// forms.

/// Generates the `Clock` join matrix.
///
/// A `|` cell owns its clock operand (whichever side it is on) and returns it;
/// a `|=` cell merges into the receiver in place. Each position — `op_l`/`op_r`
/// for the clock as the left or right `|` operand, `as_clock` for the `|=`
/// receiver — has its own `@cell` arm so the receiver `self` is written in
/// the same expansion as the method it belongs to (`self` cannot cross a
/// macro-invocation boundary).
macro_rules! clock_join_matrix {
    ($($kind:tt $lhs:ty, $rhs:ty);* $(;)?) => {
        $( clock_join_matrix!(@cell $kind $lhs, $rhs); )*
    };
    (@cell op_l $lhs:ty, $rhs:ty) => {
        impl BitOr<$rhs> for $lhs {
            type Output = Clock;
            fn bitor(mut self, r: $rhs) -> Clock {
                self.version |= r.borrow();
                self
            }
        }
    };
    (@cell op_r $lhs:ty, $rhs:ty) => {
        impl BitOr<$rhs> for $lhs {
            type Output = Clock;
            fn bitor(self, mut r: $rhs) -> Clock {
                r.version |= self.borrow();
                r
            }
        }
    };
    (@cell as_clock $lhs:ty, $rhs:ty) => {
        impl BitOrAssign<$rhs> for $lhs {
            fn bitor_assign(&mut self, r: $rhs) {
                self.version |= r.borrow();
            }
        }
    };
}

clock_join_matrix! {
    op_l     Clock,    Version;
    op_l     Clock,    &Version;
    op_r     Version,  Clock;
    op_r     &Version, Clock;
    as_clock Clock,    Version;
    as_clock Clock,    &Version;
}
