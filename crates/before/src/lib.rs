//! [`before`](crate) implements [*Interval Tree Clocks* (Almeida, Baquero &
//! Fonte, 2008)](https://gsd.di.uminho.pt/members/cbm/ps/itc2008.pdf) (ITCs)
//! using a compact representation which is approximately 100× more
//! space-efficient than a naïve transcription of the original paper, while
//! maintaining asymptotically linear and practically quick performance even
//! over the most adversarially pessimal inputs.
//!
//! A causal clock answers the question wall-clock time cannot: given two
//! updates made on different machines, did one *know about* the other, or did
//! they happen concurrently? The classic solutions to the problem — [*version
//! vectors*](https://en.wikipedia.org/wiki/Version_vector) and [*vector
//! clocks*](https://en.wikipedia.org/wiki/Vector_clock) — pay a cost linear in
//! the number of total participants who have *ever* registered an event,
//! regardless of whether they are presently part of the system. Without global
//! coordination, these causal clocks continue to grow unboundedly as
//! participants join and leave.
//!
//! Interval tree clocks solve this problem using much less space, often by more
//! than one or two orders of magnitude, and in dynamic settings they *recycle
//! identity*: a departing participant [`join`](Clock::join)s its clock into a
//! surviving peer's, returning its share of the *id space* — the range of
//! identity the participants partition among themselves — and its history, so
//! the clocks avoid unbounded growth.
//!
//! ## The types
//!
//! | Type                                    | Is                                                                                          | Core operations                                                                                                                                                                                                                                                     |
//! |-----------------------------------------|---------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
//! | [`Party`]                               | a distinct entity which may emit events                                                     | [`tick`](Party::tick), [`fork`](Party::fork)/[`forks`](Party::forks), [`join`](Party::join), [`is_disjoint`](Party::is_disjoint)                                                                                                                                    |
//! | [`Version`]                             | a causal timestamp; a history of known events                                               | [`tick`](Version::tick), [`PartialOrd`] (`<`, `<=`, [`concurrent`](Version::concurrent)), [`join`](Version::join) (`\|`), [`meet`](Version::meet) (`&`), [`span`](Version::span) (`^`), [`project`](Version::project) (`/`), [`rank`](Version::rank)                |
//! | [`Clock`]                               | a [`Party`] paired with its current [`Version`]                                             | [`tick`](Clock::tick), [`fork`](Clock::fork)/[`forks`](Clock::forks), [`join`](Clock::join), [`send`](Clock::send), [`recv`](Clock::recv), [`absorb`](Clock::absorb) (`\|`, `\|=`) a [`Version`]                                                                    |
//! | [`Rank`]/[`Ranked`]                     | a total order extending the causal order, with a lexicographically concordant serialization | [`Ord`] (`<`, `==`, `>`, etc.), summation (`+`), [`checked_sub`](Rank::checked_sub)/[`saturating_sub`](Rank::saturating_sub), [`encode`](Rank::encode), [`decode`](Rank::decode)                                                                                    |
//! | [`Span`]                                | an ordered pair of versions `lo <= hi`                                                      | [`union`](Span::union) (`+`), [`intersect`](Span::intersect) (`*`), pointwise [`join`](Span::join)/[`meet`](Span::meet) (`\|`, `&`), [`project`](Span::project), queries ([`place`](Span::place), [`dominance`](Span::dominance), [`precedence`](Span::precedence)) |
//! | [`causally`]/[`Query`](causally::Query) | a query language for causal range membership                                                | [`after`](causally::after), [`before`](causally::before), [`strictly_after`](causally::strictly_after), [`strictly_before`](causally::strictly_before), [`since`](causally::since), [`until`](causally::until), [`delta`](causally::delta), [`toward`](causally::toward), negation (`!`), conjunction (`&`), [`contains`](causally::Query::contains), [`coverage`](causally::Query::coverage)|
//!
//! ## Quickstart
//!
//! The *seed* is the unique first clock in a causal universe, initially owning
//! the whole space of possible future clocks; a *fork* splits a clock in two,
//! each half a valid clock that acts independently from then on, until it is
//! potentially *joined* in the future.
//!
//! ```
//! use before::Clock;
//!
//! // Every system of clocks descends from one seed (see the safety rules).
//! let mut alice = Clock::seed();
//!
//! // New participants fork off a live clock, never mint themselves.
//! let mut bob = alice.fork();
//!
//! // Each participant ticks its own clock to record a local event.
//! alice.tick();
//! bob.tick();
//!
//! // Neither history contains the other: the two events are concurrent.
//! assert!(alice.version().concurrent(bob.version()));
//!
//! // Versions are the timestamps that travel. Joining a received version
//! // (`|=`) merges history only; bob then knows strictly more than alice —
//! // her whole history, plus his own tick that she has not seen.
//! bob |= alice.version();
//! assert!(bob.version() > alice.version());
//!
//! // Joining a whole *clock* merges identity too: it retires bob, recycling
//! // his share of the id space into alice. It is fallible — the two parties
//! // must be disjoint — hence the `unwrap`.
//! alice.join(bob).unwrap();
//!
//! // Clocks and versions cross process boundaries as canonical bytes: one
//! // byte spelling per value, so equal bytes mean equal values.
//! let bytes = alice.encode();
//! let restored = Clock::decode(&bytes[..]).unwrap();
//! assert_eq!(restored, alice);
//! // `restored` duplicates `alice`: treat encode-then-send as a move, and
//! // let exactly one of the two handles stay live (see the safety rules).
//! ```
//!
//! ## Version vector or vector clock?
//!
//! [`before`](crate) can play either classic role; the difference is entirely
//! in how you use it.
//!
//! **In brief:** reach for a version vector when reconciling replicated state;
//! reach for a vector clock when protocol messages themselves are among the
//! events you wish to causally order.
//!
//! Usually, you should pick one discipline per protocol and keep to it; mixing
//! them is not unsafe, but mixing the two disciplines answers neither of their
//! questions clearly.
//!
//! ### [Version vectors](https://en.wikipedia.org/wiki/Version_vector) order data: "has this replica seen that write?"
//!
//! A version vector does not record any local events when sending and receiving
//! messages; only when modifying data.
//!
//! To use [`Clock`] in this manner:
//!
//! - Record changes to data using [`tick`](Clock::tick).
//! - Transmit your [`Version`] to your counterparties using [`clock.version()`](Clock::version), **not** [`Clock::send`].
//! - Upon receiving a version from a counterparty, [`absorb`](Clock::absorb) it (a.k.a. the operator forms `|` or `|=`), **not** [`Clock::recv`].
//!
//! Consistently eschewing [`Clock::send`] and [`Clock::recv`] ensures that your
//! clock's [`Version`] tracks only *data causality*, and not the ordering of
//! sends and receives.
//!
//! <details>
//! <summary>Worked example: as a version vector</summary>
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
//! // Bob's clock now dominates or equals the message and Alice's version.
//! // (`>=`, not `>`: incorporating a version records no event, so a Bob
//! // with no history of his own would end exactly equal to the message —
//! // contrast the vector-clock example below.)
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
//! </details>
//!
//! ### [Vector clocks](https://en.wikipedia.org/wiki/Vector_clock) order processes: "could this event have influenced that one?"
//!
//! A vector clock records a local event when sending and receiving as well as
//! when modifying data.
//!
//! To use [`Clock`] in this manner:
//!
//! - Record changes to data using [`tick`](Clock::tick).
//! - Transmit [`Version`]s to your counterparties using [`clock.send()`](Clock::send), and **not**
//!   [`clock.version()`](Clock::version). Unlike [`clock.version()`](Clock::version), [`send`](Clock::send)
//!   increments the inner [`Version`] prior to producing it for transmission.
//! - Upon receiving a version from a counterparty, [`recv`](Clock::recv) it; **do not** [`absorb`](Clock::absorb) it.
//!   Unlike [`absorb`](Clock::absorb) (and its operator forms `|` and `|=`), [`recv`](Clock::recv) increments the clock's
//!   inner [`Version`] after absorbing the received one.
//!
//! Consistently using [`send`](Clock::send) and [`recv`](Clock::recv`) in this
//! manner ensures that the [`Clock`] tracks *process causality*, by following
//! the ordering of sends and receives.
//!
//! <details>
//! <summary>Worked example: as a vector clock</summary>
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
//! // Bob incorporates Alice's version, then marks a "recv" event locally
//! bob.recv(&msg);
//!
//! // Bob's clock now strictly dominates the message, and also Alice's
//! // version: the receive itself was an event.
//! assert!(bob.version() > msg);
//! assert!(bob.version() > alice.version());
//!
//! // But if Alice now records another local event unknown to Bob ...
//! alice.tick();
//! // ... then their versions are now incomparable (i.e. concurrent)
//! assert!(bob.version().concurrent(alice.version()));
//!
//! // Unlike with version vectors, there is no way to re-synchronize two
//! // versions to become equal by sending or receiving messages,
//! // because receiving a message records a local event unknown to the
//! // sender by definition -- so if Bob sends to Alice, then vice-versa,
//! // then Bob's version will strictly dominate Alice's, because he knows
//! // about one more event than her (his own local receive)
//! alice.recv(bob.send());
//! bob.recv(alice.send());
//! assert!(bob.version() > alice.version());
//! ```
//!
//! </details>
//!
//! ## Safety rules
//!
//! Interval tree clocks only track a meaningful notion of causality if you
//! respect both of these rules, which together ensure that *identities are
//! always disjoint*:
//!
//! 1. **Causal Singularity.** A system of clocks has one [`Clock::seed`] (or
//!    [`Party::seed`]), created once, from which every [`Clock`] and
//!    [`Party`] in the system descends. Multiple "universes" of [`Clock`]s
//!    and/or [`Party`]s may coexist, each descended from its own
//!    [`seed`](Clock::seed), as long as parties, versions, and clocks from
//!    different seeds never intermingle.
//!
//! 2. **Identity Linearity.** Advancing a [`Clock`] or [`Party`] (by
//!    [`tick`](Clock::tick), [`fork`](Clock::fork),
//!    [`join`](Clock::join), etc.) retires every earlier state of it: from then
//!    on, only the latest state of the identity may act. Within a process, the
//!    compiler enforces this, as [`Party`] and [`Clock`] are
//!    [`!Clone`](Clone). This leaves exactly one hole: bytes. A
//!    serialized state sidesteps the type system, and
//!    [`decode`](Clock::decode) cannot tell the latest state from a stale
//!    one. Restore a [`Clock`] or [`Party`] persisted prior to its latest
//!    elsewhere-observed state, and there are now two non-disjoint copies of
//!    its identity: the restored [`Party`] may overlap its own descendant, an
//!    otherwise inexpressible violation of linearity.
//!
//! Comparing or combining [`Version`]s originating from different seeds will
//! yield arbitrary, meaningless results. It is not possible to detect if two
//! [`Version`]s originate from different seeds by inspecting the [`Version`]s
//! themselves; they may be compared or combined regardless of provenance, to
//! nonsensical effect.
//!
//! By contrast, attempts to [`join`](Clock::join) or [`sync`](Clock::sync)
//! parties or clocks originating from different seeds or those which have been
//! handled non-linearly *may* return an error *at some point*, but this is not
//! guaranteed to happen promptly, at any particular callsite or time
//! thereafter. However, if and when such an error does arise, indicating that
//! parties were not [disjoint](Party::is_disjoint), the diagnosis is always
//! definitive: the programmer violated one of the above rules.
//!
//! ## Replicating clocks between processes
//!
//! Everything crosses process boundaries as canonical bytes:
//! [`encode`](Clock::encode)/[`encode_to`](Clock::encode_to) emit them, and
//! [`decode`](Clock::decode) validates strictly, so an accepted value is always
//! in canonical normal form. The `serde` and `borsh` features serialize through
//! the same encodings (see [Crate features](#crate-features)).
//!
//! Most protocols built on [`before`](crate) will end up primarily moving
//! [`Version`]s across process boundaries, keeping each [`Clock`] pinned to its
//! process. A [`Clock`] or [`Party`] need transit the wire only when the
//! process identity itself must move. At that boundary, linearity becomes your
//! job (the second safety rule): the type system cannot see that a clock whose
//! bytes left the process now exists twice, so treat an inter-process
//! transmission of a [`Clock`] or [`Party`] as a *move* and stop using the
//! local handle immediately.
//!
//! ## Comparing and ordering versions
//!
//! [`Version`]'s [`PartialEq`] describes a causal ordering: `a <= b` tests
//! containment of history, and two versions with no containing order are
//! [`concurrent`](Version::concurrent). Three tools extend it:
//!
//! - **Filtering**: the [`causally`] module composes causal queries, e.g.
//!   [`since(a)`](causally::since) is everything `a` does not already
//!   contain, [`before(b)`](causally::before) everything `b` contains, and
//!   [`delta(a, b)`](causally::delta) everything `b` contains but `a` does
//!   not — and any compatible bounds conjoin with `&` into a
//!   [`causally::Query`], whose membership predicate is causal
//!   containment and which classifies whole version regions at once
//!   ([`coverage`](causally::Query::coverage)). For questions about a
//!   concrete pair of bounding [`Version`]s, [`Span`] interrogates the
//!   pair simultaneously and efficiently.
//! - **Sorting**: where a *total* order over versions is needed, [`Rank`]
//!   measures a version by a quantity that strictly grows with every tick,
//!   so `v < w` implies `v.rank() < w.rank()`: causes always sort before
//!   their effects. Only concurrent versions can tie, so any deterministic
//!   tiebreak then yields the same total order on every replica. [`Ranked`]
//!   builds that total order in: it views a [`Version`] by its rank, with the
//!   version's own canonical bytes as an arbitrary tiebreak, and its
//!   [`encode`](Ranked::encode) emits a canonical encoding whose plain
//!   lexicographic order *is* the total order on [`Ranked`], allowing an
//!   ordinary byte-oriented key-value store to sort by causal ordering.
//! - **Simultaneous comparison**: in some cases, it is desirable to compare
//!   a [`Version`] against both an upper and a lower bound at the same time,
//!   establishing its relationship in the causal interval `lo <= v <= hi`.
//!   [`Span`] represents precisely such ordered pairs of [`Version`]s, with
//!   a host of algebraic operations and queries of varying granularity to
//!   establish the relative ordering of [`Version`]s against the represented
//!   interval. These operations are more efficient than the individual comparisons
//!   which would otherwise be required.
//!
//! ## Space Efficiency
//!
//! At 100 parties and 1,000,000 events, the expected size of a [`Party`] is
//! about 3 bytes and the expected size of a [`Version`] is about 100 bytes
//! (measured: the space-consumption experiment that draws the figure below).
//! These figures assume static membership; continually [`fork`](Clock::fork)ing
//! and [`join`](Clock::join)ing causes these to grow, but with reasonable
//! bounds. Under sustained random membership churn, those same 100 parties will
//! each stabilize at around 50 bytes (growing linearly in the steady-state
//! number of parties `N`) and their corresponding versions at around 2,000
//! bytes (roughly `N²`).
//!
// The space-consumption figure, inlined so it inherits the page's theme
// (build.rs derives this theme-reactive form from the measurement
// artifact in results/). The HTML comment renders as nothing here; it is
// the derived README's image in reference form, which tools/readme
// unwraps and resolves to an absolute URL (rustdoc and GitHub cannot
// share one image mechanism: an inline SVG follows the reader's theme
// but GitHub strips it from a README, and an image URL renders on
// GitHub but cannot react to rustdoc's theme picker).
#![doc = include_str!(concat!(env!("OUT_DIR"), "/space_consumption.svg"))]
//!
//! <!-- ![Space consumption of `before`'s interval-tree versions][space-consumption] -->
//!
//! The operations in this crate are also designed to mitigate the possibility
//! of adversarial *transient* memory amplification. Even on pathologically
//! shaped inputs, the auxiliary space required to compute any operation is at
//! most a small constant multiple of the input size. In many cases, no scratch
//! space is needed at all; where possible, operations read their input(s) in
//! one single fused pass, writing their output simultaneously. Any operation
//! which requires more than a small amount of temporary memory (relative to its
//! input size) is a bug: please report it!
//!
//! ## Asymptotic Complexity
//!
//! Every operation in the crate is documented with its
//! [Big-O](https://en.wikipedia.org/wiki/Big_O_notation) time complexity,
//! noting space complexity where this is non-trivial. Unless otherwise
//! documented, the *size* of an argument `|x|` means that argument's *size in
//! encoded bytes*.
//!
//! The operations in this crate have been very carefully hardened against
//! pathological input shapes. Any asymptotic claim is a hard guarantee that the
//! operation will perform in time proportionate to that bound, for all input
//! sizes, no matter how unlikely and contorted the shape of the input.
//! Moreover, the operations in this crate are optimized through a variety of
//! techniques to ensure that they have low constant factors, especially on
//! organically reachable (i.e. non-adversarial) inputs, with the asymptotic
//! guarantee acting as a backstop in case of pathologically unlikely shapes.
//! Any violation of these guarantees is a bug: please report it!
//!
//! Most operations also carry an interactive **measured-growth chart** in these
//! API docs; the one below is [`Version::tick`]'s (it may not render if you are
//! viewing this on a third-party site such as GitHub or crates.io).
//!
#![doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_tick.open.html"))]
//!
//! Each chart shows a heatmap of instruction-count required to evaluate the
//! function, plotted against the total input size in bytes, and sampled over
//! thousands of inputs drawn uniformly from inputs of exact sizes. The bands in
//! the heatmap are the cost distribution for each input size column; the dotted
//! trace follows your chosen quantile of that same distribution (median by
//! default); the solid curves are hypotheses about the growth of the function,
//! with the documented bound pre-selected and *compensated*. Compensating by a
//! hypothesis divides it out from the graph, magnifying divergence from the
//! hypothesis, so that the heatmap's bands will display flat exactly where cost
//! grows at the selected hypothesis.
//!
//! These charts are meant to be read in terms of shape, not absolute magnitude,
//! because the instruction counts are computed via
//! [`wasmtime`](https://github.com/bytecodealliance/wasmtime)'s idiosyncratic
//! deterministic "fuel" metric, and therefore do not directly translate to
//! absolute instruction counts in native builds. Consequentially, the cost axis
//! carries no absolute label. Note also that uniform sampling shows the
//! *distributional bulk* of the input space, so an operation with a rare worst
//! case may read below its bound. The bound still holds, even though the
//! pathological input shapes which exercise the worst cases of some functions
//! are effectively impossible to hit under uniform sampling.
//!
//! ## Crate features
//!
//! Every feature is off by default.
//!
//! - **`serde`:** `Serialize`/`Deserialize` for [`Party`], [`Version`],
//!   [`Clock`], [`Rank`], [`Ranked`], and [`Span`].
//! - **`borsh`:** `BorshSerialize`/`BorshDeserialize`, likewise as the
//!   canonical encodings. The encodings are *prefix-free* — no value's
//!   encoding is a prefix of another's — and values therefore compose
//!   inside larger borsh messages without a length prefix.
//! - **`oracle`**, **`meter`** (plus the meter's counter switches
//!   `limb-meter` and `scan-meter`), and **`laws`:** expose the crate's own
//!   verification instruments (the reference implementation,
//!   the input generators and resource meters behind the performance tests,
//!   and the named algebraic-law predicates) to its bench, metering,
//!   and fuzz suites. These are unnecessary in production.
//!
//! ## Testing
//!
//! Every operation is verified differentially against the paper's naive
//! recursive implementation as well as a nondeterministic function-space
//! semantics, alongside exhaustive small-scope enumeration of clock shapes,
//! algebraic-law property suites, and fuzzed codecs.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod auto_traits;
mod clock;
mod codec;
mod fold;
mod idbits;
mod party;
mod recurse;
mod version;

// The whole public API:
pub mod span;
pub use clock::Clock;
pub mod causally;
pub mod error;
pub use party::Party;
pub use span::{OwnSpan, Span};
pub use version::{OwnVersion, Rank, Ranked, Ticks, Version};
pub mod iter;

// Tutorial-only documentation:
pub mod implementation;

#[cfg(any(test, feature = "oracle"))]
pub mod oracle;

#[cfg(any(test, feature = "meter"))]
pub mod meter;

#[cfg(any(test, feature = "meter"))]
pub mod surface;

#[cfg(any(test, feature = "laws"))]
pub mod laws;

#[cfg(feature = "serde")]
mod serde_impls;

#[cfg(feature = "borsh")]
mod borsh_impls;

#[cfg(test)]
mod testing;
