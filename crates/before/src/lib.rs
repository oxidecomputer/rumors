//! [`before`](crate) implements [*Interval Tree Clocks* (Almeida, Baquero &
//! Fonte, 2008)](https://gsd.di.uminho.pt/members/cbm/ps/itc2008.pdf) (ITCs)
//! using an efficient and compact representation.
//!
//! Interval tree clocks use much less space than traditional representations of
//! version vectors and vector clocks, often by more than an order of magnitude.
//! In dynamic settings where participants join and leave, they can also
//! *recycle identifiers* without violating causality, so they avoid the
//! unbounded growth that affects naïve sparse clocks and vectors.
//!
//! ## The types
//!
//! | Type        | Is                                              | Core operations                                                                                                                                                           |
//! |-------------|-------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
//! | [`Party`]   | a distinct entity which may emit events         | [`tick`](Party::tick), [`fork`](Party::fork), [`join`](Party::join), [`is_disjoint`](Party::is_disjoint)                                                                  |
//! | [`Version`] | a causal timestamp (a history of seen events)   | [`tick`](Version::tick), compare (`<`, `<=`, [`concurrent`](Version::concurrent)), merge (`\|`)                                                                           |
//! | [`Clock`]   | a [`Party`] paired with its current [`Version`] | [`tick`](Clock::tick), [`fork`](Clock::fork), [`join`](Clock::join), [`sync`](Clock::sync), [`send`](Clock::send), [`recv`](Clock::recv), merge (`\|`) with [`Version`]   |
//!
//! [`Party`]s and [`Clock`]s are linear ([`!Clone`](Clone)); [`Version`]s are
//! freely [`Clone`]able.
//!
//! ## A conceptual sketch
//!
//! The insight of the original ITC paper is that a [`Party`] can be
//! represented as a *tree* denoting a non-empty set of subintervals of
//! `[0, 1)`, giving both compact representation and dynamic membership. The
//! initial [`Party`], [`Party::seed`], is `{ [0, 1) }`; a
//! [`fork`](Party::fork) splits an interval in half, so the first fork yields
//! `{ [0, 1/2) }` and `{ [1/2, 1) }`. Disjoint interval sets are
//! [`join`](Party::join)ed by set union, merging adjacent intervals:
//! `{ [0, 1/2), [5/8, 3/4) }` ∪ `{ [3/4, 1) }` = `{ [0, 1/2), [5/8, 1) }`.
//! Parties can therefore be minted and recycled freely while their
//! representations stay small.
//!
//! A [`Version`] is then a function from `[0, 1)` to the natural numbers,
//! also represented as a tree, with the initial [`Version`] the
//! constantly-zero function. To register an event for a [`Party`], it
//! suffices to increment the function over any non-empty region owned by that
//! party. Any such choice yields a valid causal timestamp, and the freedom of
//! choice lets the implementation simplify a [`Version`]'s tree on
//! [`tick`](Version::tick). As parties and versions are forked and joined
//! over the lifetime of a distributed system, their typical size therefore
//! stays small: hundreds to low thousands of bytes, even for hundreds of
//! communicating processes and millions of events.
//!
//! That lattice is the [`Version`] API: the partial order `<=` tests whether
//! one version's history is contained in another's, the join `|` combines two
//! histories into their least upper bound, and [`tick`](Version::tick) moves
//! strictly upward. Two histories with no containing order are
//! *[`concurrent`](Version::concurrent)*.
//!
//! The [`causally`] module names that order's down-sets: composable
//! constructors — [`since`](causally::since), [`before`](causally::before),
//! [`delta`](causally::delta), and friends — build a [`causally::Range`]
//! (a [`RangeBounds<Version>`](std::ops::RangeBounds) whose membership
//! predicate is causal containment) for any API that filters by where
//! versions stand in history. Where a *total* order over versions is
//! needed, [`Rank`] is the exact, strictly-monotone causal rank: ordering
//! by `(Rank, tiebreak)` linearly extends causality, so concurrent
//! versions can be sequenced deterministically without ever inverting a
//! causal dependency. [`Ranked`] packages a version with its rank as a
//! ready-made totally ordered key, tiebroken by canonical bytes.
//!
//! By packaging a [`Version`] and a [`Party`] together into a [`Clock`], we get
//! a causal clock which may be [`tick`](Clock::tick)ed,
//! [`fork`](Clock::fork)ed, and [`join`](Clock::join)ed, in addition to derived
//! operations like [`send`](Clock::send), [`recv`](Clock::recv), and
//! [`sync`](Clock::sync). This is sufficient to implement *both* [*version
//! vectors*](https://en.wikipedia.org/wiki/Version_vector) and [*vector
//! clocks*](https://en.wikipedia.org/wiki/Vector_clock), depending on how you
//! use it.
//!
//! ## Example
//!
//! Depending on whether you want the semantics of [*version
//! vectors*](https://en.wikipedia.org/wiki/Version_vector) or [*vector
//! clocks*](https://en.wikipedia.org/wiki/Vector_clock), you use
//! [`before`](crate) slightly differently.
//!
//! ### ... as a Version Vector
//!
//! [*Version vectors*](https://en.wikipedia.org/wiki/Version_vector) give a
//! causal ordering to **data**. Participants **do not** record a local event
//! when sending and receiving messages; only when modifying data.
//!
//! ```
//! use before::Clock;
//!
//! // Alice is the distinguished first party who creates the first clock
//! let mut alice = Clock::seed();
//!
//! // Alice hands Bob a clock of his own
//! let mut bob = alice.fork();
//!
//! // Alice marks an event locally
//! alice.tick();
//!
//! // Bob marks an event locally
//! bob.tick();
//!
//! // Alice sends her *current* version *without* recording another event locally
//! let msg = alice.version();
//!
//! // Bob incorporates Alice's version *without* recording another event locally
//! bob |= msg;
//!
//! // Bob's clock now dominates or is equal to the message, and also Alice's version
//! assert!(bob.version() >= msg);
//! assert!(bob.version() >= alice.version());
//!
//! // But if Alice now records another local event unknown to Bob ...
//! alice.tick();
//! // ... then their versions are now incomparable (i.e. concurrent)
//! assert!(bob.version().concurrent(alice.version()));
//!
//! // Bob can send his version back to Alice, and vice-versa,
//! // for their versions to become equal again.
//! alice |= bob.version();
//! bob |= alice.version();
//! assert!(bob.version() == alice.version());
//! ```
//!
//! ### ... as a Vector Clock
//!
//! [*Vector clocks*](https://en.wikipedia.org/wiki/Vector_clock) give a causal
//! ordering to **processes**. Participants **do** record a local event when
//! sending and receiving messages, *as well as* when modifying data.
//!
//! ```
//! use before::Clock;
//!
//! // Alice is the distinguished first party who creates the first clock
//! let mut alice = Clock::seed();
//!
//! // Alice hands Bob a clock of his own
//! let mut bob = alice.fork();
//!
//! // Alice marks an event locally
//! alice.tick();
//!
//! // Bob marks an event locally
//! bob.tick();
//!
//! // Alice marks a "send" event locally and then sends her version to Bob
//! let msg = alice.send();
//!
//! // Bob incorporates Alice's version, then marking a "recv" event locally
//! bob.recv(&msg);
//!
//! // Bob's clock now dominates the message, and also Alice's version
//! assert!(bob.version() > msg);
//! assert!(bob.version() > alice.version());
//!
//! // But if Alice now records another local event unknown to Bob ...
//! alice.tick();
//! // ... then their versions are now incomparable (i.e. concurrent)
//! assert!(bob.version().concurrent(alice.version()));
//!
//! // Unlike with version vectors, there is no way to re-synchronize two
//! // versions to become strictly equal by sending or receiving messages,
//! // because receiving a message records a local event unknown to the
//! // sender by definition -- so if Bob sends to Alice, then vice-versa,
//! // then Bob's version will strictly dominate Alice's, because he knows
//! // about one more event than her (his own local receive)
//! alice.recv(bob.send());
//! bob.recv(alice.send());
//! assert!(bob.version() > alice.version());
//! ```
//!
//! ## Safety rules
//!
//! Interval tree clocks are correct only under the Law of Disjointness: no
//! [`Party`] may ever interact with another [`Party`] that is not
//! [*disjoint*](Party::is_disjoint) from it. The caller must ensure both:
//!
//! 1. **Singularity.** A system of clocks has one [`Clock::seed`] (or
//!    [`Party::seed`]), created once, from which every [`Clock`] and
//!    [`Party`] in the system descends. One [`Party`] may be reused with
//!    multiple [`Version`]s, and multiple "universes" may coexist, each
//!    descended from its own [`seed`](Clock::seed), as long as clocks from
//!    different seeds never interact.
//!
//! 2. **Linearity.** Operations on [`Clock`]s and [`Party`]s are strictly
//!    linear: once a [`Clock`] or [`Party`] has been
//!    [`fork`](Clock::fork)ed, a copy of the pre-fork value must not come
//!    back into play. The crate helps by making [`Party`] and [`Clock`]
//!    [`!Clone`](Clone), but at serialization boundaries and across
//!    processes, linearity is the caller's responsibility.
//!
//! ## Testing
//!
//! Every operation is verified differentially against the paper's naive
//! recursive implementation — the [`oracle`] module, public under the
//! `oracle` feature so the benchmarks can time the two against each
//! other — alongside exhaustive small-scope enumeration of clock shapes,
//! algebraic-law property suites, and fuzzed codecs (`decode`'s strict
//! canonicality is asserted inline in the fuzz targets). Every discovered
//! counterexample's seed is committed under `proptest-regressions/`, so
//! failures reproduce deterministically forever.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod clock;
mod codec;
mod idbits;
mod party;
mod version;

// The whole public API:
pub use clock::Clock;
pub mod causally;
pub mod error;
pub use party::Party;
pub use version::{Rank, Ranked, Version};
pub mod batch {
    //! [`batch::Clock`](Clock) and [`batch::Version`](Version) amortize costs
    //! to improve performance on [`Clock`](crate::Clock)s and
    //! [`Version`](crate::Version)s.
    //!
    //! ```
    //! use before::{batch, Clock};
    //! let mut clock = Clock::seed();
    //! clock.batch().tick().tick().tick().tick();  // faster in a batch
    //! assert_eq!(clock.version().to_string(), "4");
    //! ```
    pub use crate::{clock::Batch as Clock, version::Batch as Version};
}

// No outer doc comment: one here would merge with the module's inner docs
// and shift their link resolution to this scope, where `grow`/`descend!`/
// `STRIDE` don't resolve. The module documents itself.
mod recurse;

/// Reference oracle: the paper's recursive trees; ground truth for the
/// differential tests. Public under the `oracle` feature so the benchmark suite
/// can time it against the optimized implementation.
#[cfg(any(test, feature = "oracle"))]
pub mod oracle;

#[cfg(feature = "serde")]
mod serde_impls;

#[cfg(feature = "borsh")]
mod borsh_impls;

#[cfg(test)]
mod testing;

/// Record one traversal step. Expands to a counter bump under `cfg(test)` (see the
/// test-only [`metrics`](crate::testing::metrics) module) and to nothing otherwise.
///
/// This is used to deterministically test asymptotic traversal cost to prevent
/// accidental quadraticity.
#[cfg(test)]
macro_rules! step {
    () => {
        $crate::testing::metrics::bump()
    };
}
#[cfg(not(test))]
macro_rules! step {
    () => {};
}
pub(crate) use step;
