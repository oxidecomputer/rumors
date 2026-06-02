//! [`before`](crate) implements [*Interval Tree Clocks* (Almeida, Baquero &
//! Fonte, 2008)](https://gsd.di.uminho.pt/members/cbm/ps/itc2008.pdf) (ITCs)
//! using an efficient and compact representation.
//!
//! Interval tree clocks use much less space than traditional representations of
//! version vectors and vector clocks, often by more than an order of magnitude.
//! Uniquely, in dynamic settings where participants may join or leave, they can
//! *recycle identifiers* without violating causality, thereby avoiding the
//! unbounded linear inflation to which naïve sparse clocks/vectors fall victim.
//!
//! ## A conceptual sketch
//!
//! The insight of the original ITC paper is that we can get *both* compact
//! representation *and also* dynamic membership by representing a [`Party`] as
//! a *tree* which denotes a non-empty set of subintervals of `[0, 1)`. The
//! initial [`Party`], [`Party::seed`], is `{ [0, 1) }`; subsequent
//! [`fork`](Party::fork)s split that interval into `{ [0, 1/2) }` and
//! `{ [1/2, 1) }`, etc. These sets of disjoint intervals can then be
//! [`join`](Party::join)ed together by taking their disjoint set union
//! (concretely a merge of trees), so that `{ [0, 1/2), [5/8, 3/4) }` ∪
//! `{ [3/4, 1) }` = `{ [0, 1/2), [5/8, 1) }` (note: merging adjacent intervals).
//! This gives us a lattice algebra of [`Party`]s which can be dynamically
//! generated and recycled, with parsimonious inner structure.
//!
//! Atop this, we can implement a second lattice algebra of [`Version`]s, so
//! that a [`Version`] is a function from `[0, 1)` to natural numbers (also
//! represented concretely as a tree), with the initial [`Version`] being the
//! constantly-zero function. To register an event in a [`Version`] for a given
//! [`Party`], we need only increment an *arbitrary* non-empty part of the
//! [`Version`]'s domain so that the incremented portion lies entirely within
//! the [`Party`]'s owned set of subintervals. Any such arbitrary choice will
//! let [`Version`]s behave like causal timestamps for [`Party`]s, and the
//! implementation freedom left by this conceptual nondeterminism allows our
//! concrete representation to *simplify [`Version`]s on
//! [`tick`](Version::tick)*. This opportunistic structural compacting means
//! that even as [`Party`]s and [`Version`]s are dynamically forked and joined
//! over the lifetime of a distributed system, their average representational
//! size remains quite small (in the hundreds or low thousands of bytes, even
//! for hundreds of communicating processes and millions of iterations).
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
//! ## ⚠️ Safety rules
//!
//! In order to reap the rewards of interval tree clocks, one must always heed
//! the Law of Disjointness: no [`Party`] may ever interact with another
//! [`Party`] which is not [*disjoint*](Party::is_disjoint) from it. In other
//! words, the programmer must ensure both:
//!
//! 1. **Singularity:** There must be one singular [`Clock::seed`]
//!    (alternatively, [`Party::seed`]) *ever* created *anywhere* in any given
//!    system of clocks, and *all* [`Clock`]s or [`Party`]s *everywhere* must
//!    descend from it. Note that it is acceptable to reuse one [`Party`] with
//!    multiple [`Version`]s; one also may create multiple "universes" of
//!    [`Clock`]s (or [`Party`]s), each descended from a different
//!    [`seed`](Clock::seed), *so long as [`Clock`]s (or [`Party`]s) from different
//!    [`seed`](Clock::seed)s never interact*.
//!
//! 2. **Linearity:** Operations on [`Clock`]s and [`Party`]s must be *strictly
//!    linear*; once a [`Clock`] or [`Party`] has been [`fork`](Clock::fork)ed, a
//!    copy of the pre-`fork` entity *must not* come back into play. This crate
//!    assists by making [`Party`] and [`Clock`] [!`Clone`](Clone), but enforcing
//!    linearity becomes the *programmer's responsibility* at serialization
//!    boundaries and in the presence of multiple processes.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod clock;
mod codec;
mod idbits;
mod party;
mod version;

// The whole public API:
pub use clock::Clock;
pub mod error;
pub use party::Party;
pub use version::Version;
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

/// Stack-growth guard shared by the recursive traversals.
mod recurse;

/// Reference oracle: the paper's recursive trees; ground truth for the
/// differential tests. Public under the `oracle` feature so the benchmark suite
/// can time it against the optimized implementation.
#[cfg(any(test, feature = "oracle"))]
pub mod oracle;

#[cfg(feature = "serde")]
mod serde_impls;

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
