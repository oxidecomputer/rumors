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
//! documented escape hatches: decoding can recreate a party from bytes, and
//! [`dangerously_alias`](Party::dangerously_alias) deliberately duplicates one
//! in memory.

use crate::codec::{self, BitsView};
use crate::error::Decode;
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
/// | Operation                                              | Meaning                                                                   |
/// |--------------------------------------------------------|---------------------------------------------------------------------------|
/// | [`Party::seed()`]                                      | create the initial [`Party`] which owns all of `[0, 1)`                   |
/// | [`p.tick(v)`](Party::tick)                             | advance the [`Version`] `v` for this [`Party`]                            |
/// | [`p.ticks(v, n)`](Party::ticks)                        | advance the [`Version`] `v` by `n` events for this [`Party`], in one pass |
/// | [`p.fork()`](Party::fork)/[`p.forks(n)`](Party::forks) | split off one disjoint child from `p` (or `n` disjoint children)          |
/// | [`p.join(b)`](Party::join)                             | reunite two *disjoint* parties into the one owning both regions; fallible |
/// | [`p.is_disjoint(&b)`](Party::is_disjoint)              | whether `p` and `q` share no region, hence may safely interact            |
/// | `p == q`                                               | whether `p` is exactly the same [`Party`] as `q`                          |
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
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscape-assets.html")))]
pub struct Party(codec::Bits);

// Identity Linearity (the crate docs' second safety rule) is compiler-enforced
// within a process precisely because `Party` is `!Clone`: every operation that
// redistributes identity moves it, so no share is ever in two hands. A
// `Clone`/`Copy` impl would break the rule for every holder at once, so its
// absence is pinned here at the definition, where a tempting `derive` would
// land.
static_assertions::assert_not_impl_any!(Party: Clone, Copy);

// Equality and hashing are byte-level over the stored stream's raw bytes plus
// its live length, resting on the canonical-raw-slice invariant: the build
// buffer keeps its dead bits zero and `from_bits` seals the marker at every
// storage seam, so raw-byte equality is exactly bit equality (see
// `codec::canonical_eq` for the argument and the measurement). The two impls
// read the same pair, so `Eq`/`Hash` consistency holds by construction.
impl PartialEq for Party {
    fn eq(&self, other: &Self) -> bool {
        codec::canonical_eq(&self.0, &other.0)
    }
}

impl Eq for Party {}

/// Hashes the canonical bytes, consistently with `Eq`'s byte compare.
///
/// # Complexity
///
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/party_hash.html")))]
#[cfg_attr(
    not(doc),
    doc = "`O(n)` in total input bytes; `O(|self|)`: one pass over the canonical bytes"
)]
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
    /// [`Clock`](crate::Clock), which binds the two together.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_tick.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |party|)`")]
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
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_ticks.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n)` in total input bytes; `O(|self| + |party| + log k)`"
    )]
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
    pub fn ticks(&self, version: &mut Version, k: impl Into<Ticks>) {
        version.ticks(self, k)
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
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/party_fork.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self|)`")]
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

    /// Splits `k` balanced shares off this [`Party`] as a lazy iterator.
    ///
    /// Unlike repeatedly calling [`fork`](Party::fork), which deepens its
    /// representation into a biased linear tree (see its warning), every
    /// resultant [`Party`] produced here increases in size by only a
    /// logarithmic factor.
    ///
    /// A [`Party`] is never empty, so `self` retains one residual share. It also
    /// retains every share the iterator has not returned.
    ///
    /// `k` may be a [`Ticks`] count or any standard unsigned integer type.
    /// Suffix integer literals to select an unsigned type, as in `3u64`. The
    /// iterator type is exported as [`iter::Party`](crate::iter::Party).
    ///
    /// To split a [`Party`] into exactly `N` shares with no residual, see
    /// [`From<Party>`](Party) for `[Party; N]`.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/party_forks.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n)` in total input bytes; a full drain costs `O(|self| + k (|self| + log k))`"
    )]
    ///
    /// Shares are built on demand; see [`Forks`] for the per-step and early-drop costs.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Party;
    /// let mut p = Party::seed();
    /// let shares: Vec<Party> = p.forks(3u64).collect();
    /// assert_eq!(shares.len(), 3); // three shares handed out...
    /// for s in &shares {
    ///     assert!(p.is_disjoint(s)); // ...each disjoint from the keeper
    /// }
    /// // `self` kept the fourth; rejoining all four recovers the whole seed.
    /// p.join_all(shares).unwrap();
    /// assert!(p.is_seed());
    /// ```
    pub fn forks(&mut self, k: impl Into<Ticks>) -> Forks<'_> {
        Forks::new(self, k.into())
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
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/party_join.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n)` in total input bytes; `O(|self| + |other|)`, accepted or rejected"
    )]
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
    /// Returns every input region not absorbed into `self`, without dropping
    /// any region. Returned parties may be unions of inputs. Once an overlap is
    /// found, later inputs may be returned without being tested.
    ///
    /// Unreachable for parties descended from one [`seed`](Party::seed): they
    /// are definitionally pairwise-disjoint.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/party_join_all.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n log n)` in total input bytes; `O((|self| + |iter|) log k + (|self| + |iter|) log |self|)` time, `k` the operand count"
    )]
    ///
    /// Auxiliary space is `O(|self| + |iter|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Party;
    /// let mut p = Party::seed();
    /// let shares: Vec<Party> = p.forks(3u64).collect();
    /// p.join_all(shares).unwrap(); // the residual and three shares reunite
    /// assert!(p.is_seed());
    /// ```
    pub fn join_all<I: IntoIterator<Item = Party>>(&mut self, iter: I) -> Result<(), Vec<Party>> {
        let groups =
            crate::fold::balanced_try_fold(iter, |mut top, incoming| match top.join(incoming) {
                Ok(()) => Ok(top),
                Err(back) => Err((top, back)),
            })?;
        let mut groups = groups.into_iter();
        while let Some(group) = groups.next() {
            if let Err(back) = self.join(group) {
                let mut uncombined = vec![back];
                uncombined.extend(groups);
                return Err(uncombined);
            }
        }
        Ok(())
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
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/party_is_disjoint.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n)` in total input bytes; `O(|self| + |other|)`, no allocation"
    )]
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
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/party_covers.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n)` in total input bytes; `O(|self| + |other|)`, no allocation"
    )]
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
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/party_without.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |other|)`")]
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
    /// assert_eq!(p.without(&q).unwrap(), keep);
    ///
    /// // Removing a covering share (here, itself) leaves nothing.
    /// assert!(Party::seed().without(&Party::seed()).is_none());
    /// ```
    pub fn without(self, other: &Party) -> Option<Party> {
        let bits = self.view().diff(other.view());
        if bits.is_empty() {
            None
        } else {
            Some(Party::from_bits(bits))
        }
    }

    /// The party's shape.
    ///
    /// This iterator renders its 0/1-valued membership function over the
    /// interval `[0, 1)` as an iterator of [`Region`](crate::shape::Region)s,
    /// left to right.
    ///
    /// One item per maximal constant run: whether the party owns it and
    /// the dyadic interval it spans.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/party_shape.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self|)` to drain")]
    ///
    /// Draining the iterator is linear in the party's encoded size: each
    /// region costs `O(1)`, and nothing allocates.
    ///
    /// # Example
    ///
    /// ```
    /// use before::shape::Region;
    /// use before::Party;
    ///
    /// let mut party = Party::seed();
    /// let right = party.fork();
    /// let _second_quarter = party.fork();
    /// party.join(right).unwrap();
    /// let regions: Vec<Region> = party.shape().collect();
    /// assert_eq!(
    ///     regions,
    ///     vec![
    ///         Region { owned: true, depth: 2 },  // the first quarter...
    ///         Region { owned: false, depth: 2 }, // ...but not the second...
    ///         Region { owned: true, depth: 1 },  // ...and the whole right half.
    ///     ],
    /// );
    /// ```
    pub fn shape(&self) -> crate::shape::Regions<'_> {
        crate::shape::Regions::of_party(self)
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
    /// rule applies to any [`Clock`](crate::Clock) built from such a party.
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
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/party_encode.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self|)`")]
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
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/party_encode.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self|)`")]
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
    /// Instrument surface, public under the `meter` feature: the resource
    /// meters, coverage suites, and boundary pins denominate readings in
    /// exact encoded bit lengths. Applications measure wire cost as
    /// `encode().len()` or [`as_bytes`](Self::as_bytes)`.len()` — the byte
    /// length actually shipped.
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
    #[cfg(any(test, feature = "meter"))]
    pub fn encoded_bits(&self) -> u64 {
        // The stored form's O(1) length: exact at every size memory holds.
        self.0.len()
    }

    /// Decodes one [`Party`] from a reader.
    ///
    /// A successful decode requires exactly one canonical encoding; bytes after
    /// its padding are an error.
    ///
    /// # Warning
    ///
    /// Decoding can recreate a party that is still live elsewhere, bypassing
    /// its `!Clone` linearity. Treat transfer through bytes as a move: never let
    /// the source and decoded party participate in the same system.
    ///
    /// # Errors
    ///
    /// - [`Decode::Truncated`] if the tree or its padding is incomplete;
    /// - [`Decode::TrailingBits`] if the padding is malformed or followed by
    ///   more bytes;
    /// - [`Decode::NotCanonical`] if the tree contains a collapsible node;
    /// - [`Decode::Io`] if the reader fails.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/party_decode.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n)` in total input bytes; `O(n)` with `n` the size of the input, accepted or rejected"
    )]
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
        Self::decode_bytes(buf.into())
    }

    /// Validates and adopts an owned canonical encoding.
    pub(crate) fn decode_bytes(buf: bytes::Bytes) -> Result<Self, Decode> {
        {
            let end = codec::parse_id(codec::BitsView::whole(&buf), 0)?;
            codec::require_marker_padding(&buf, end)?;
        }
        Ok(Party(codec::Bits::from_canonical(buf)))
    }

    /// A read-only [`IdReader`] cursor at the root of this party's party bits.
    pub(crate) fn view(&self) -> IdReader<'_> {
        IdReader::root(self.0.live())
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

    /// The preorder bit stream, live bits only (the padding stays
    /// behind the view). Internal.
    pub(crate) fn as_bits(&self) -> BitsView<'_> {
        self.0.live()
    }

    /// Freeze a normal-form encoded bit stream as a `Party`, canonicalizing its
    /// storage. The single build-side gate every built/parsed `Party` passes
    /// through.
    ///
    /// Callers guarantee normal *tree* form (a nonempty, normalized id);
    /// the freeze seals the marker padding so the stored bytes are
    /// canonical — see [`codec::Bits::freeze`] for the seam's contract and
    /// what the padding underpins.
    pub(crate) fn from_bits(bits: codec::BitsBuf) -> Self {
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

/// Shows the party as `Party(0b…)` using its binary encoding.
impl core::fmt::Debug for Party {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Party({:#b})", self.0)
    }
}
