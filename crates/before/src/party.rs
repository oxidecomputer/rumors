//! Disjoint parties who can emit events.
//!
//! A [`Party`] is a non-empty set of subintervals of `[0, 1)`, stored as a
//! canonical id-tree: the share of the identifier space its holder may
//! [`tick`](Party::tick) against. [`fork`](Party::fork) splits a share in two;
//! [`join`](Party::join) reunites disjoint shares and refuses overlapping ones,
//! because everything ITCs guarantee rests on disjointness (see the [crate
//! docs](crate)' safety rules).
//!
//! Parties are deliberately `!Clone`, and the operations that redistribute
//! identity move it linearly: `fork` and `join` mutate their receiver and
//! `join` consumes its operand, so no share is ever in two hands, while `tick`
//! merely (mutably) borrows. The type system enforces that linearity up to the
//! documented escape hatches: the serialization and text/literal doors, which
//! mint a second holder from bytes or notation, and
//! [`dangerously_alias`](Party::dangerously_alias), the deliberate in-memory
//! duplication.

use core::fmt::Display;

use crate::codec::{self, BitsSlice};
use crate::error::{Decode, Parse};
use crate::idbits::IdReader;
use crate::{Ticks, Version};

mod forks;
pub(crate) mod ops;

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
/// | [`a.ticks(v, n)`](Party::ticks)           | advance the [`Version`] by `n` events, in one pass                        |
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
/// # Example
///
/// ```
/// use before::Party;
/// let mut whole = Party::seed();
/// let half = whole.fork();
/// assert!(whole.is_disjoint(&half)); // the two halves share no region
/// whole.join(half).unwrap();         // ... and reunite into the whole
/// assert!(whole.is_seed());
/// ```
pub struct Party(codec::Bits);

// Equality and hashing are byte-level over the stored stream's raw bytes plus
// its live length, resting on the canonical-raw-slice invariant: `from_bits`
// zeroes the dead pad bits at every storage seam, so raw-byte equality is
// exactly bit equality (see `codec::canonical_eq` for the argument and the
// measurement). The two impls read the same pair, so `Eq`/`Hash` consistency
// holds by construction.
impl PartialEq for Party {
    fn eq(&self, other: &Self) -> bool {
        codec::canonical_eq(&self.0, &other.0)
    }
}

impl Eq for Party {}

impl core::hash::Hash for Party {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        codec::canonical_hash(&self.0, state);
    }
}

impl Party {
    /// The initial [`Party`] in the system.
    ///
    /// Call this function (or [`Clock::seed`](crate::Clock::seed), which
    /// invokes it internally) **exactly once** per interacting system of
    /// parties.
    ///
    /// Every descendant of a single seed is disjoint from its peers, but
    /// descendants of two independent seeds need not be; if they ever interact,
    /// causal history is silently corrupted.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// assert!(before::Party::seed().is_seed()); // the whole region, undivided
    /// ```
    pub fn seed() -> Self {
        // The seed id is exactly the 2-bit terminal tag `00` (the whole
        // interval, owned), marker-padded to the one static byte
        // `0b0010_0000`: construction allocates nothing, and every seed
        // shares the one static buffer. A `static`, not a `const`: a
        // const's promoted allocation has no guaranteed unique address,
        // and the cross-call sharing claim rests on one. The codec
        // round-trip and text laws pin the constant against the parsed
        // form.
        static SEED_STREAM: &[u8] = &[0b0010_0000];
        Party(codec::Bits::from_canonical(bytes::Bytes::from_static(
            SEED_STREAM,
        )))
    }

    /// Whether this party is equal to [`Party::seed`].
    ///
    /// True only before any [`fork`](Party::fork) has split a region away, and
    /// again only once every fork has been [`join`](Party::join)ed back.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
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

    /// Advances `version` by one event for this party.
    ///
    /// Dealing directly with a [`Party`] and a [`Version`] permits one version
    /// to be [`tick`](Version::tick)ed by many parties, or one [`Party`] to
    /// [`tick`](Party::tick) many [`Version`]s; this is in contrast to a
    /// [`Clock`], which binds the two together.
    ///
    /// # Complexity
    ///
    /// `O(|self| + |version|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Party, Version};
    /// let mut v = Version::new();
    /// Party::seed().tick(&mut v);
    /// assert!(v > Version::new()); // one event: strictly after the empty history
    /// ```
    pub fn tick(&self, version: &mut Version) {
        version.tick(self)
    }

    /// Advances `version` by `n` events for this [`Party`]
    ///
    /// The result is identical to `n` sequential [`tick`](Self::tick)s, but
    /// computed much more efficiently.
    ///
    /// # Complexity
    ///
    /// `O(|self| + |version| + log n)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Party, Version};
    /// let p = Party::seed();
    /// let mut v = Version::new();
    /// p.ticks(&mut v, 3u64);
    /// let mut w = Version::new();
    /// for _ in 0..3 {
    ///     p.tick(&mut w);
    /// }
    /// assert_eq!(v, w); // one call, same version as three sequential ticks
    /// ```
    pub fn ticks(&self, version: &mut Version, n: impl Into<Ticks>) {
        version.ticks(self, n)
    }

    /// Splits off a new disjoint [`Party`] from this one.
    ///
    /// # Warning
    ///
    /// Repeatedly [`fork`](Party::fork)ing the same [`Party`] produces an
    /// imbalanced internal representation, with memory use linear in the number
    /// of sequential forks, and worse performance. Whenever possible, prefer to
    /// vary which party is forked, or use [`forks`](Party::forks) to generate a
    /// fixed number of balanced forks.
    ///
    /// # Complexity
    ///
    /// `O(|self|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Party;
    /// let mut p = Party::seed();
    /// let q = p.fork();
    /// assert!(p.is_disjoint(&q)); // the halves share no region...
    /// assert!(!p.is_seed() && !q.is_seed()); // ...and neither is the whole
    /// ```
    pub fn fork(&mut self) -> Party {
        let (keep, give) = self.view().split();
        *self = Party::from_bits(keep);
        Party::from_bits(give)
    }

    /// Splits `n` balanced shares off this [`Party`], as a lazy
    /// [`ExactSizeIterator`].
    ///
    /// Unlike repeatedly calling [`fork`](Party::fork), which deepens its
    /// representation into a biased linear tree (see its warning), every
    /// resultant [`Party`] produced here increases in size by only a
    /// logarithmic factor.
    ///
    /// A [`Party`] is never empty, so `self` retains its residual share even
    /// once the iterator is fully drained; shares not taken before the iterator
    /// drops are [`join`](Party::join)ed back into `self`.
    ///
    /// To split a [`Party`] into exactly `N` shares with no residual, see
    /// [`From<Party>`](Party) for `[Party; N]`.
    ///
    /// # Complexity
    ///
    /// A full drain costs `O(|self| + n (|self| + log n))` at worst. Shares are
    /// built on demand (see [`Forks`] for the per-step and early-drop costs).
    ///
    /// # Example
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
    pub fn forks(&mut self, n: u64) -> Forks<'_> {
        Forks::new(self, n)
    }

    /// Reunites two disjoint [`Party`]s.
    ///
    /// # Errors
    ///
    /// If the parties are not disjoint, `self` is unmodified, and `Err(other)`
    /// is returned.
    ///
    /// # Complexity
    ///
    /// `O(|self| + |other|)`, regardless of acceptance or rejection.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Party;
    /// let mut p = Party::seed();
    /// let q = p.fork();
    /// p.join(q).unwrap(); // the two halves reunite into the whole
    /// assert!(p.is_seed());
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

    /// Reunites every disjoint [`Party`] in `iter` into `self`.
    ///
    /// # Errors
    ///
    /// Returns the parties which *overlapped* and so could not be folded in,
    /// dropping nothing: every input [`Party`] is either merged into `self` or
    /// handed back. In case of partial error, the set of parties which are
    /// absorbed vs. handed back is unspecified.
    ///
    /// Unreachable for parties descended from one [`seed`](Party::seed): they
    /// are definitionally pairwise-disjoint.
    ///
    /// # Complexity
    ///
    /// `O((|self| + |iter|) log k + (|self| + |iter|) log |self|)` time, where
    /// `k` is the count of `iter`, `O(|self| + |iter|)` auxiliary space.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Party;
    /// let mut p = Party::seed();
    /// let shares: Vec<Party> = p.forks(3).collect();
    /// p.join_all(shares).unwrap(); // the residual and three shares reunite
    /// assert!(p.is_seed());
    /// ```
    pub fn join_all<I: IntoIterator<Item = Party>>(&mut self, iter: I) -> Result<(), Vec<Party>> {
        // The shared balanced binary counter (`crate::fold`), one join into
        // `self` per surviving group at the end — a left fold into `self` would
        // re-walk the whole growing union per input, quadratic scan work on
        // scattered populations. Inputs overlapping `self` can never merge (the
        // union only grows), so the up-front `accept` test against the *fixed*
        // `self` hands them back exactly as the growing-union fold would;
        // regions disjoint from `self` stay disjoint from it however they
        // coalesce, so the final joins cannot fail on well-formed input. The
        // up-front test runs against a per-call [`ops::IdIndex`] of `self` —
        // O(input) node visits plus the table searches per input, instead of a
        // cursor re-walk of the fixed `self` per input, which would make the
        // fold quadratic on populations of many small inputs against a large
        // accumulator (the index module doc carries the trade). A failed
        // combine is aliased input; the counter's hand-back policy
        // (`crate::fold`) drops nothing.
        let mut overlapping = Vec::new();
        let index = ops::IdIndex::build(self.as_bits());
        let groups = crate::fold::balanced_try_fold(
            iter,
            |other| index.is_disjoint(other.view()),
            |mut top, incoming| match top.join(incoming) {
                Ok(()) => Ok(top),
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
            Ok(())
        } else {
            Err(overlapping)
        }
    }

    /// Tests whether `self` and `other` are *disjoint*.
    ///
    /// All live descendants of a single [`seed`](Party::seed), evolved by
    /// linear [`fork`](Party::fork) and [`join`](Party::join), are pairwise
    /// disjoint. The converse *does not hold*: just because two parties are
    /// disjoint, it does not mean they descended from the same seed, or that
    /// they evolved linearly!
    ///
    /// Disjoint [`Party`]s may always be [`join`](Party::join)ed without error.
    ///
    /// # Complexity
    ///
    /// `O(|self| + |other|)`, no allocations.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Party;
    /// let mut p = Party::seed();
    /// let q = p.fork();
    /// assert!(p.is_disjoint(&q));
    /// ```
    // Deliberately no clone-identity fast path here (or on `covers`):
    // parties are linear, so a live clone-shared pair has no production
    // witness — `dangerously_alias` is a boundary hand-off, not a live
    // operand pair — and the lockstep walk stays the one mechanism the
    // fuel bands price.
    pub fn is_disjoint(&self, other: &Party) -> bool {
        self.view().is_disjoint(other.view())
    }

    /// Tests whether `self`'s owned region contains all of `other`'s
    /// (`self ⊇ other`).
    ///
    /// # Complexity
    ///
    /// `O(|self| + |other|)`, no allocations.
    ///
    /// # Example
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
    // No clone-identity fast path, as `is_disjoint`: linearity leaves a
    // live clone-shared party pair no production witness.
    pub fn covers(&self, other: &Party) -> bool {
        self.view().covers(other.view())
    }

    /// Carves `other`'s region out of `self`, forcing the parties to become
    /// disjoint.
    ///
    /// Returns `None` when `other` [`covers`](Party::covers) `self` and nothing
    /// remains. Otherwise, returns the remainder.
    ///
    /// This is a partial inverse of [`join`](Party::join): where `join`
    /// folds a disjoint share in, `without` cuts a share back out.
    ///
    /// # Complexity
    ///
    /// `O(|self| + |other|)`.
    ///
    /// # Example
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

    /// Duplicates this party, producing a second handle to the same identity,
    /// **intentionally violating linearity**.
    ///
    /// # Warning
    ///
    /// [`Party`] is [`!Clone`](Clone) because two live handles to one [`Party`]
    /// break disjointness, so if both copies (or any of their
    /// [`fork`](Party::fork)s) go on to [`tick`](Party::tick) or
    /// [`join`](Party::join), causal history can be corrupted arbitrarily.
    ///
    /// The caller must ensure that at most one of the two copies is ever
    /// treated as live; the other must be dropped without further use. The same
    /// rule applies to any [`Clock`] built from such a party.
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
    /// use before::Party;
    /// let p = Party::seed();
    /// let q = p.dangerously_alias();
    /// assert!(!p.is_disjoint(&q));
    /// ```
    pub fn dangerously_alias(&self) -> Self {
        Party(self.0.clone())
    }

    /// Encodes this [`Party`] to bytes.
    ///
    /// Prefer [`as_bytes`](Party::as_bytes) to get a reference to the
    /// underlying encoding without cloning it.
    ///
    /// # Complexity
    ///
    /// `O(|self|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Party;
    /// let p = Party::seed();
    /// assert_eq!(Party::decode(&p.encode()[..]).unwrap(), p);
    /// ```
    pub fn encode(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    /// Encodes this [`Party`] to an arbitrary writer.
    ///
    /// # Complexity
    ///
    /// `O(|self|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Party;
    /// let mut buf = Vec::new();
    /// Party::seed().encode_to(&mut buf).unwrap();
    /// assert_eq!(buf, Party::seed().encode());
    /// ```
    pub fn encode_to<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(self.as_bytes())
    }

    /// The exact length in bits of [`encode`](Self::encode) before its
    /// padding — the marker bit and zero-pad to the byte boundary, so
    /// `encode().len()` is `(encoded_bits() + 1).div_ceil(8)`.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// // The seed is a single terminal: a 2-bit presence tag (`00`).
    /// assert_eq!(before::Party::seed().encoded_bits(), 2);
    /// ```
    pub fn encoded_bits(&self) -> usize {
        self.as_bits().len()
    }

    /// Decodes a [`Party`] from a reader of canonical bytes, strictly rejecting
    /// non-canonical representations.
    ///
    /// # Warning
    ///
    /// Serializing a [`Clock`] circumvents its otherwise compiler-enforced
    /// `!Clone` linearity. Deserializing one can violate causality. Treat
    /// serialization/deserialization boundaries as *moves* of the [`Clock`].
    ///
    /// # Complexity
    ///
    /// `O(n)` with `n` the size of the input, regardless of whether accepted or
    /// rejected.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Party;
    /// let bytes = Party::seed().encode();
    /// assert_eq!(Party::decode(&bytes[..]).unwrap(), Party::seed());
    /// ```
    pub fn decode<R: std::io::Read>(mut reader: R) -> Result<Self, Decode> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).map_err(Decode::Io)?;
        {
            let bits = codec::bytes_as_bits(&buf);
            let end = codec::parse_id(bits, 0)?;
            codec::require_marker_padding(bits, end)?;
        }
        // Adopt the read buffer as the result's backing store without
        // copying: the padding check proved the buffer is the stream's one
        // marker-padded spelling — the canonical form the at-rest
        // container stores. The id grammar has no empty production
        // (exhausted input rejects as `Truncated` above), so the parsed
        // id is a nonzero share — the standalone-party invariant (paper
        // §3: `i ≠ 0`) holds structurally.
        Ok(Party(codec::Bits::from_canonical(buf.into())))
    }

    /// The anonymous (zero) id: the empty bit stream, since a `0` is structural
    /// absence in the pruned encoding.
    ///
    /// Internal and transient only (i.e. for use in `mem::swap`) and *never* a
    /// publicly constructible value (a `Party` is a nonzero share).
    ///
    /// Used as a placeholder when moving a party out of a `&mut` (the splitting
    /// iterator in [`forks`](Party::forks)), immediately overwritten by a real
    /// share.
    pub(crate) fn anonymous() -> Party {
        Party(codec::Bits::empty())
    }

    /// A read-only [`IdReader`] cursor at the root of this party's packed id bits.
    pub(crate) fn view(&self) -> IdReader<'_> {
        IdReader::root(&self.0)
    }

    /// Reunites this party with `other` and re-splits the union, in one fused
    /// walk: the `(keep, give)` halves of [`join`](Party::join) followed by
    /// [`fork`](Party::fork), or `None` if the parties overlap.
    ///
    /// Byte-identical to that composition (`IdReader::sum_split` carries the
    /// argument; the `sync_is_join_then_fork` law and the `sum_split`
    /// differentials pin it), without building the joined party. Neither
    /// operand is moved, accepted or refused.
    ///
    /// `O(|self| + |other|)` worst case, and sublinear where the regions do not
    /// interleave — a subtree owned by one side alone is spliced into its half
    /// without a walk.
    pub(crate) fn sum_split(&self, other: &Party) -> Option<(Party, Party)> {
        let (keep, give) = self.view().sum_split(other.view())?;
        Some((Party::from_bits(keep), Party::from_bits(give)))
    }

    /// The canonical bytes of this [`Party`], borrowed.
    ///
    /// These bytes are a canonical identity: byte-equal if and only if the
    /// parties are equal, and consistent with [`hash`](core::hash::Hash).
    ///
    /// A [`Party`] is not ordered (see the type docs). The lexicographic order
    /// of these bytes is an arbitrary total order with no meaning, useful only
    /// as a deterministic tiebreak.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Party;
    /// let p = Party::seed();
    /// assert_eq!(p.as_bytes(), p.encode().as_slice());
    /// ```
    pub fn as_bytes(&self) -> &[u8] {
        debug_assert!(
            codec::padding_is_canonical(&self.0),
            "non-canonical Party storage: the bytes must end in the `1 0*` padding",
        );
        self.0.as_raw_slice()
    }

    /// The packed preorder bit stream, live bits only (the padding stays
    /// behind the view). Internal.
    pub(crate) fn as_bits(&self) -> &BitsSlice {
        &self.0
    }

    /// Freeze a normal-form packed bit stream as a `Party`, canonicalizing its
    /// storage. The single build-side gate every built/parsed `Party` passes
    /// through.
    ///
    /// Callers guarantee normal *tree* form (a nonempty, normalized id);
    /// the freeze seals the marker padding so the stored bytes are
    /// canonical — see [`codec::Bits::freeze`] for why a tree op can leave
    /// the tail dirty, and what the padding underpins.
    pub(crate) fn from_bits(bits: codec::BitsMut) -> Self {
        Party(codec::Bits::freeze(bits))
    }

    /// Adopt an already-frozen canonical id stream as a `Party`: the
    /// decode-side gate, dual to the build-side [`from_bits`](Self::from_bits).
    ///
    /// Callers guarantee the stream is a nonempty normal-form id in canonical
    /// storage — what a validated decode slice already is — so no
    /// re-canonicalization runs and adoption is `O(1)`.
    pub(crate) fn from_frozen(bits: codec::Bits) -> Self {
        Party(bits)
    }
}

/// Paper notation: `0` / `1` leaves, `(l, r)` nodes. E.g. `(1, (0, 1))`.
///
/// # Complexity
///
/// `O(|self|)` time and space: the text spells `O(1)` bytes per id-tree node,
/// so it is itself `O(|self|)` bytes.
///
/// # Example
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

/// Parses paper notation (`0 | 1 | (i1, i2)`), strictly rejecting
/// non-normal-form input and the anonymous identity `0` (a standalone `Party`
/// must be a nonzero share).
///
/// Parsing *creates* the party its text names, tied to no existing handle:
/// `"1".parse()` yields a party overlapping every seed's whole region.
///
/// # Complexity
///
/// `O(|s|)` time and space, accepted or rejected; the parsed party is itself
/// `O(|s|)` bytes.
///
/// # Example
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

/// Wrap validated id bits as a `Party`, rejecting the anonymous (empty)
/// identity. The single gate through which every parsed/built top-level `Party`
/// passes.
fn finish_id(bits: codec::BitsMut) -> Result<Party, Parse> {
    if codec::id_is_empty(&bits) {
        Err(Parse::Anonymous)
    } else {
        Ok(Party::from_bits(bits))
    }
}

/// An id literal that can ground out a [`Party`] tuple: the `u8` leaves `0`/`1`
/// and nested `(left, right)` tuples.
///
/// Sealed and hidden — an implementation detail enabling `Party::try_from(..)`
/// literals. Unlike the public `TryFrom`, an `IdLit` leaf of `0` is allowed (it
/// is a valid *sub-tree*); the anonymous check happens only once the whole id
/// is assembled (see [`finish_id`]).
mod sealed {
    pub trait Sealed {}
    impl Sealed for u8 {}
    impl Sealed for bool {}
    impl<T, S> Sealed for (T, S) {}
}

#[doc(hidden)]
pub trait PartyLiteral: sealed::Sealed {
    #[doc(hidden)]
    fn into_id_bits(self) -> Result<codec::BitsMut, Parse>;
}

impl PartyLiteral for u8 {
    fn into_id_bits(self) -> Result<codec::BitsMut, Parse> {
        match self {
            0 => Ok(codec::id_leaf(false)),
            1 => Ok(codec::id_leaf(true)),
            _ => Err(Parse::Syntax),
        }
    }
}

impl PartyLiteral for bool {
    fn into_id_bits(self) -> Result<codec::BitsMut, Parse> {
        Ok(codec::id_leaf(self))
    }
}

impl<T: PartyLiteral, S: PartyLiteral> PartyLiteral for (T, S) {
    fn into_id_bits(self) -> Result<codec::BitsMut, Parse> {
        let l = self.0.into_id_bits()?;
        let r = self.1.into_id_bits()?;
        codec::id_node(&l, &r) // assembles + validates normal form
    }
}

/// An id leaf from a single bit: `1` (full) is a valid `Party`; `0` is the
/// anonymous identity and is rejected here, though it is allowed as a sub-tree
/// in the tuple form.
///
/// Like every literal door, this *creates* identity tied to no existing handle.
///
/// # Complexity
///
/// `O(1)`.
///
/// # Example
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
/// Mints identity exactly as the `u8` literal door does — a test and
/// fresh-universe door ([Safety rules](crate#safety-rules)).
///
/// # Complexity
///
/// `O(1)`.
///
/// # Example
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

/// An id node from a `(left, right)` literal, e.g. `Party::try_from((1u8, (0u8,
/// 1u8)))`. Rejects a collapsible `(v, v)` (non-canonical) and an all-`0`
/// (anonymous) result.
///
/// # Complexity
///
/// `O(n)`, `n` the built party's size in bytes.
///
/// # Example
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
