//! [`before`](crate) implements [*Interval Tree Clocks* (Almeida, Baquero &
//! Fonte, 2008)](https://gsd.di.uminho.pt/members/cbm/ps/itc2008.pdf) (ITCs)
//! using an efficient and compact representation.
//!
//! A causal clock answers the question wall-clock time cannot: given two
//! updates made on different machines, did one *know about* the other, or did
//! they happen concurrently? The classic answers — [*version
//! vectors*](https://en.wikipedia.org/wiki/Version_vector) and [*vector
//! clocks*](https://en.wikipedia.org/wiki/Vector_clock) — pay one counter per
//! participant, forever: an entry can never be safely removed once its
//! participant leaves, so under churn those clocks only grow.
//!
//! Interval tree clocks give the same causal answers in much less space,
//! often by more than an order of magnitude, and in dynamic settings they
//! *recycle identity*: a departing participant [`join`](Clock::join)s its
//! clock into a surviving peer's, returning its share of the *id space* —
//! the range of identity the participants partition among themselves — and
//! its history, so the clocks avoid unbounded growth.
//!
//! ## Quickstart
//!
//! A first session. Two informal definitions up front: the *seed* is the
//! first clock, owning the whole id space; a *fork* splits a clock in two,
//! each half a valid clock that acts independently from then on.
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
//! ## The types
//!
//! | Type                | Is                                              | Core operations                                                                                                                                                   |
//! |---------------------|-------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------|
//! | [`Party`]           | a distinct entity which may emit events         | [`tick`](Party::tick), [`fork`](Party::fork)([`s`](Party::forks)), [`join`](Party::join), [`is_disjoint`](Party::is_disjoint)                                     |
//! | [`Version`]         | a causal timestamp (history of known events)    | [`tick`](Version::tick), [`PartialOrd`] (`<`, `<=`, [`concurrent`](Version::concurrent)), join (`\|`), meet (`&`), [`rank`](Version::rank)                        |
//! | [`Clock`]           | a [`Party`] paired with its current [`Version`] | [`tick`](Clock::tick), [`fork`](Clock::fork)([`s`](Clock::forks)), [`join`](Clock::join), [`send`](Clock::send), [`recv`](Clock::recv), join (`\|`, `\|=`) a [`Version`] |
//! | [`Rank`]/[`Ranked`] | a total order extending the causal order       | [`Ord`] (`<`, `==`, `>`, etc.), summation (`+`), [`checked_sub`](Rank::checked_sub)                                                                               |
//!
//! [`Party`]s and [`Clock`]s are linear ([`!Clone`](Clone)): moved, never
//! duplicated, because duplicating identity is exactly what breaks a causal
//! clock. [`Version`]s are freely [`Clone`]able. A tick pairs the two
//! halves — `party.tick(&mut version)` and `version.tick(&party)` are the
//! same act from either receiver ([`Party::tick`], [`Version::tick`]) — and
//! [`Clock::tick`] performs it on its own pair.
//! The [`batch`] module chains several operations through one borrow; the
//! [`iter`] module forks `n` peers in one balanced split.
//!
//! ## Version vector or vector clock?
//!
//! [`before`](crate) can play either classic role; the difference is
//! entirely in how you use it. [*Version
//! vectors*](https://en.wikipedia.org/wiki/Version_vector) order **data** —
//! they answer "has this replica seen that write?" — so participants record
//! an event only when the data changes. [*Vector
//! clocks*](https://en.wikipedia.org/wiki/Vector_clock) order **processes** —
//! they answer "could this event have influenced that one?" — so
//! participants record an event on every send and receive as well. Reach for
//! a version vector when reconciling replicated state; reach for a vector
//! clock when the messages themselves are the events. Pick one discipline
//! per protocol and keep to it — mixing them is not unsafe, but the ordering
//! then answers neither question exactly.
//!
//! ### As a version vector
//!
//! Participants **do not** record a local event when sending and receiving
//! messages; only when modifying data.
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
//! ### As a vector clock
//!
//! Participants **do** record a local event when sending and receiving
//! messages, *as well as* when modifying data.
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
//! ## Safety rules
//!
//! Interval tree clocks are correct only under the Law of Disjointness: no
//! [`Party`] may ever interact with another [`Party`] that is not
//! [*disjoint*](Party::is_disjoint) from it. (A party is a set of id-space
//! intervals — see [How it works](#how-it-works); disjoint parties share
//! none.) Two parties *interact* when one is [`join`](Clock::join)ed or
//! [`sync`](Clock::sync)ed (a mutual join) with the other, and whenever versions they tick
//! meet in a comparison or a join. Only the first kind is fenced: join and
//! sync do verify their operands, and refuse overlapping parties. The
//! second kind is where corruption lives — a version carries no record of
//! who ticked it, so a version written through a duplicated identity is
//! indistinguishable from a healthy one, and comparisons simply begin
//! reporting causal order that never happened. Nothing panics; the answers
//! are just wrong.
//!
//! The two rules below are what make the Law hold: obey them and every two
//! live parties in a system are disjoint by construction. An overlap error
//! from a join or sync is therefore always a symptom, never the disease —
//! some rule was already broken upstream (a pre-fork copy came back into
//! play, or a second seed leaked in), and the fence caught one visible
//! consequence of it; the rest of the damage may already be silent. The
//! caller must ensure both:
//!
//! 1. **Singularity.** A system of clocks has one [`Clock::seed`] (or
//!    [`Party::seed`]), created once, from which every [`Clock`] and
//!    [`Party`] in the system descends. One [`Party`] may be reused with
//!    multiple [`Version`]s ([`Party::tick`] borrows the party, so one
//!    identity can stamp many histories), and multiple "universes" may
//!    coexist, each descended from its own [`seed`](Clock::seed), as long
//!    as clocks from different seeds never interact. Nothing in a value or
//!    its encoding identifies its universe, so that boundary is the
//!    caller's to police.
//!
//!    *What about a universe tag, so mixups could be detected?* There is
//!    deliberately no mechanism for this. Naming a universe is the concern
//!    of the library that embeds [`before`](crate), which already has the
//!    protocol and structures a universe lives in — while a tag stamped
//!    into every value (a UUID, say) would add 128 bits to values that
//!    are often a few bytes long, paid across an entire corpus.
//!
//! 2. **Linearity.** Operations on [`Clock`]s and [`Party`]s are strictly
//!    linear: once a [`Clock`] or [`Party`] has been
//!    [`fork`](Clock::fork)ed, a copy of the pre-fork value must not come
//!    back into play. The crate helps by making [`Party`] and [`Clock`]
//!    [`!Clone`](Clone), but at serialization boundaries and across
//!    processes, linearity is the caller's responsibility.
//!
//! ## Replicating clocks between processes
//!
//! Everything crosses process boundaries as canonical bytes:
//! [`encode`](Clock::encode)/[`encode_to`](Clock::encode_to) emit them, and
//! [`decode`](Clock::decode) validates strictly — malformed or non-canonical
//! input is rejected, never repaired — so an accepted value is always in
//! normal form, and byte equality on encodings is exactly value equality.
//! The `serde` and `borsh` features serialize through the same encodings
//! (see [Crate features](#crate-features)).
//!
//! Most protocols move [`Version`]s — the freely [`Clone`]able timestamps —
//! and keep each [`Clock`] pinned to its process. A whole [`Clock`] or
//! [`Party`] crosses the wire only when the identity itself must move: a
//! hand-off, or a retiring peer sending its clock to a survivor — any live
//! peer can absorb it, since [`join`](Clock::join) needs only disjointness.
//! At that boundary, linearity becomes your job (the second safety rule):
//! the type system cannot see that a clock whose bytes left the process now
//! exists twice, so treat encode-then-send as a *move* and stop using the
//! local handle.
//!
//! ## Comparing and ordering versions
//!
//! Causality itself is the partial order on [`Version`]: `a <= b` tests
//! containment of history, and two versions with no containing order are
//! [`concurrent`](Version::concurrent). Two tools extend it:
//!
//! - **Filtering**: the [`causally`] module names causal ranges —
//!   [`since(a)`](causally::since) is everything `a` does not already
//!   contain, [`before(b)`](causally::before) everything strictly contained
//!   in `b`, and [`delta(a, b)`](causally::delta) everything known at `b`
//!   but not at `a`: the shape of "what my peer has not seen yet". Each is
//!   a [`causally::Range`], a
//!   [`RangeBounds<Version>`](std::ops::RangeBounds) whose membership
//!   predicate is causal containment.
//! - **Sorting**: where a *total* order over versions is needed, [`Rank`]
//!   measures a version by a quantity that strictly grows with every tick
//!   — the exact area under the version's history function, drawn out in
//!   [`implementation`] — so `v < w` implies
//!   `v.rank() < w.rank()`: causes always sort before their effects. Only
//!   concurrent versions can tie, and any deterministic tiebreak then
//!   yields the same total order on every replica. [`Ranked`] packages a
//!   version with its rank as a ready-made totally ordered key, tiebroken
//!   by canonical bytes.
//!
//! ## How it works
//!
//! The insight of the original ITC paper is that a [`Party`] can be
//! represented as a *tree* denoting a non-empty set of subintervals of
//! `[0, 1)` — each node splits its interval in half, and a leaf marks its
//! whole subinterval owned or not — giving both compact representation and
//! dynamic membership. The initial [`Party`], [`Party::seed`], is
//! `{ [0, 1) }`; a [`fork`](Party::fork) splits an interval in half, so the
//! first fork yields `{ [0, 1/2) }` and `{ [1/2, 1) }`. Disjoint interval
//! sets are [`join`](Party::join)ed by set union, merging adjacent
//! intervals: `{ [0, 1/2), [5/8, 3/4) }` ∪ `{ [3/4, 1) }` = `{ [0, 1/2),
//! [5/8, 1) }`. Parties can therefore be minted and recycled freely while
//! their representations stay small.
//!
//! A [`Version`] is then a function from `[0, 1)` to the natural numbers,
//! also represented as a tree, with the initial [`Version`] the
//! constantly-zero function. To register an event for a [`Party`], it
//! suffices to increment the function over any non-empty region owned by
//! that party. Any such choice is valid because a successor timestamp only
//! has to dominate its predecessor — everywhere at least as high, somewhere
//! strictly higher — and disjointness guarantees no other party ever
//! increments the same region. That freedom lets the implementation prefer
//! the increment that *simplifies* the tree, flattening a party's region to
//! one plateau rather than stacking detail; ticks flatten and joins merge
//! fragments, which is why typical sizes stay small — hundreds to low
//! thousands of bytes even for hundreds of communicating processes and
//! millions of events (see [Efficiency](#efficiency)).
//!
//! These functions form a *lattice* — a partial order in which any two
//! elements have a least combined upper bound — and that lattice is the
//! [`Version`] API: the partial order `<=` tests whether one version's
//! history is contained in another's, the join `|` combines two histories
//! into their least upper bound, the meet `&` keeps exactly the history two
//! versions share, and [`tick`](Version::tick) moves strictly upward. Two
//! histories with no containing order are
//! *[`concurrent`](Version::concurrent)*. Packaging a [`Version`] and a
//! [`Party`] together into a [`Clock`] gives a causal clock which may be
//! [`tick`](Clock::tick)ed, [`fork`](Clock::fork)ed, and
//! [`join`](Clock::join)ed, in addition to derived operations like
//! [`send`](Clock::send), [`recv`](Clock::recv), and [`sync`](Clock::sync)
//! (a mutual exchange: both clocks end holding the joined history).
//!
//! How the crate makes all of this compact and fast — the packed
//! representation, the coding, and the sweeps every operation runs as — is
//! the [`implementation`] module's essay, with a worked example in hand.
//!
//! ## Efficiency
//!
//! At 100 parties and 1,000,000 events, the expected size of a [`Party`] is
//! about 3 bytes and the expected size of a [`Version`] is about 100 bytes
//! (measured: the space-consumption experiment that draws the figure below).
//! These figures assume static membership; continually [`fork`](Clock::fork)ing
//! and [`join`](Clock::join)ing causes these to grow, but with reasonable
//! bounds. Under sustained membership churn, those same 100 parties will each
//! stabilize at around 50 bytes (growing linearly in the number of parties
//! `N`) and their corresponding versions at around 2,000 bytes (roughly
//! `N²`).
//!
//! ![Space consumption of `before`'s interval-tree versions][space-consumption]
//!
//! This crate implements cache-friendly, optimized versions of the operations
//! in the original paper, in addition to a host of useful operations not
//! described therein. Compared to a 1-to-1 transliteration of the paper into
//! Rust, [`before`](crate) is between 2–20× faster (measured: the workspace
//! bench suite times every operation against that transliteration, kept
//! in-tree as the differential-testing oracle).
//!
//! ## Crate features
//!
//! Every feature is off by default.
//!
//! - **`serde`** — `Serialize`/`Deserialize` for [`Party`], [`Version`], and
//!   [`Clock`], each as its canonical byte encoding; deserializing runs the
//!   same strict validation as [`decode`](Clock::decode).
//! - **`borsh`** — `BorshSerialize`/`BorshDeserialize`, likewise as the
//!   canonical encodings. The encodings are *prefix-free* — no value's
//!   encoding is a prefix of another's, so a decoder knows where each value
//!   ends — and values therefore compose inside larger borsh messages
//!   without a length prefix.
//! - **`doc-images`** — embeds the space-consumption diagram above into the
//!   rendered docs (`cargo doc --all-features`).
//! - **`oracle`**, **`meter`** (plus the meter's counter switches
//!   `limb-meter` and `scan-meter`), and **`laws`** — expose the crate's own
//!   verification instruments — the paper-faithful reference implementation,
//!   the input generators and resource meters behind the performance tests,
//!   and the named algebraic-law predicates the law proptests and fuzz
//!   target share — to its bench, metering, and fuzz suites. Never for
//!   production use.
//!
//! ## Testing
//!
//! Every operation is verified differentially against the paper's naive
//! recursive implementation as well as a nondeterministic function-space
//! semantics (versions modeled as literal mathematical functions, with
//! tick's freedom of where to increment exercised nondeterministically),
//! alongside exhaustive small-scope enumeration of clock shapes,
//! algebraic-law property suites, and fuzzed codecs (`decode`'s strict
//! canonicality is asserted inline in the fuzz targets).

// Define the `[space-consumption]` image reference above as a base64 data URI.
// A relative path would resolve against the rustdoc HTML output tree, where the
// source asset is never copied, so the image must be embedded inline. The
// `cfg_attr(all(), …)` wrapper is the stable-Rust idiom for a macro call in
// doc-attribute position; the fallback note keeps the link defined when the
// `doc-images` feature is off (a plain `cargo build` never pulls the dep).
#![cfg_attr(
    feature = "doc-images",
    cfg_attr(
        all(),
        doc = ::embed_doc_image::embed_image!(
            "space-consumption",
            "results/space_consumption/itc_space_consumption.svg"
        )
    )
)]
#![cfg_attr(
    not(feature = "doc-images"),
    doc = "[space-consumption]: # \"build with the `doc-images` feature \
           (enabled by `cargo doc --all-features`) to render this diagram\""
)]
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
    //! [`batch::Clock`](Clock) and [`batch::Version`](Version) apply a run
    //! of operations through one mutable borrow, each committing as it
    //! runs, with a chainable API.
    //!
    //! ```
    //! use before::{batch, Clock};
    //! let mut clock = Clock::seed();
    //! clock.batch().tick().tick().tick().tick();  // four ticks, one borrow
    //! assert_eq!(clock.version().to_string(), "4");
    //! ```
    pub use crate::{clock::Batch as Clock, version::Batch as Version};
}
pub mod iter {
    //! Lazy balanced-fork iterators: [`iter::Party`](Party) and
    //! [`iter::Clock`](Clock).
    //!
    //! They hand out `n` shallow shares of a [`Party`](crate::Party) (or
    //! [`Clock`](crate::Clock)) in one balanced split — see
    //! [`Party::forks`](crate::Party::forks) — generating each share on demand
    //! and folding any unconsumed shares back when dropped.
    //!
    //! ```
    //! use before::{iter, Party};
    //! let mut p = Party::seed();
    //! let forks: iter::Party<'_> = p.forks(3);
    //! assert_eq!(forks.len(), 3); // an ExactSizeIterator of three shares
    //! let shares: Vec<Party> = forks.collect();
    //! assert_eq!(shares.len(), 3);
    //! ```
    pub use crate::{clock::Forks as Clock, party::Forks as Party};
}
pub mod implementation {
    //! How [`before`](crate) works inside: the skyline, the packed
    //! representation, and the sweep kernels.
    //!
    //! End-user documentation lives in the [crate docs](crate) and on the
    //! public items; here we discuss the design. Nothing on this page adds
    //! to the public contract — it explains why the crate's behavior, and
    //! its costs, are what the contracts say. Where the crate docs' [How
    //! it works](crate#how-it-works) section sketches the paper's model,
    //! this page walks the machinery, one small example in hand.
    //!
    //! ## The skyline
    //!
    //! Start with what a version *is*. The crate docs define a
    //! [`Version`](crate::Version) as a function from the unit id interval
    //! `[0, 1)` to the naturals — how many events each point of the id
    //! space has seen. Plot that function and you get plateaus of differing
    //! heights over subintervals: a city *skyline*. Here is the version
    //! written `(0, 1, (0, 0, 2))` in the paper's tree notation, drawn as
    //! the function it denotes:
    //!
    //! ```text
    //! 2 │                  ┌────────
    //! 1 │────────┐         │
    //! 0 │        └─────────┘
    //!   0       1/2       3/4      1
    //! ```
    //!
    //! The skyline is the semantics; a tree is just one way to spell it (in
    //! `(n, l, r)` notation, a node's number lifts its whole subtree and
    //! each child covers half its parent's interval). Every operation is a
    //! pointwise statement about the skyline: causal comparison is
    //! pointwise `<=` — one history contains another exactly when the
    //! containing skyline is nowhere lower — join `|` is pointwise max,
    //! meet `&` is
    //! pointwise min, and [`rank`](crate::Version::rank) is the **area
    //! under the skyline**: for the drawing above, `1·½ + 0·¼ + 2·¼ = 1`.
    //! (An area over dyadic intervals is a dyadic rational, `num · 2⁻ᵉˣᵖ`,
    //! which [`Rank`](crate::Rank) keeps exact at any magnitude — the
    //! exactness behind its strict-monotonicity guarantee.)
    //!
    //! A [`Party`](crate::Party) has a skyline reading too, one step
    //! simpler: a 0-or-1 landscape over the same interval — 1 where the
    //! party owns the id space, 0 where it does not. Disjoint parties are
    //! landscapes whose owned regions never overlap, and a tick raises the
    //! version's skyline somewhere over the party's owned region: anywhere
    //! there will do, since a successor timestamp only has to dominate its
    //! predecessor and no other party ever writes that region.
    //!
    //! ## The packed representation
    //!
    //! At rest, a party, version, or clock is exactly its wire encoding:
    //! one packed bit stream in one heap buffer. There is no node graph
    //! behind the API — the tree exists only as the order of bits in the
    //! stream — so [`encode`](crate::Version::encode) is a copy of the
    //! stored bytes, [`decode`](crate::Version::decode) is one validating
    //! pass over bytes read straight into the new value's own storage
    //! (nothing is re-encoded or rebuilt), and a value's memory footprint
    //! is its wire footprint.
    //!
    //! **Ids.** The paper writes a party's tree with `1` for an owned
    //! leaf, `0` for an unowned one, and `(l, r)` for a node — `(1, 0)`
    //! owns exactly the left half. The stream writes that tree in
    //! preorder, two bits per node, answering "does a left child follow?"
    //! and "does a right child follow?". An unowned region is simply
    //! *absent* — its parent's tag already said so, and no bits follow —
    //! while the childless tag is a terminal, a wholly owned region. So
    //! the seed, owning everything, is one terminal: two bits; and
    //! `(1, 0)` is a left-only node and then its terminal: four bits.
    //! (An owns-nothing party has no spelling: parties are non-empty by
    //! construction, and [`without`](crate::Party::without) returns
    //! `None` sooner than spell one.)
    //!
    //! ```
    //! use before::Party;
    //! assert_eq!(Party::seed().encoded_bits(), 2);
    //! assert_eq!("(1, 0)".parse::<Party>().unwrap().encoded_bits(), 4);
    //! ```
    //!
    //! **Versions.** A version's tree is one topology flag per preorder
    //! node — internal or leaf — with each leaf's plateau height following
    //! its flag in the stream. Unlike the paper's trees, interior nodes
    //! carry no numbers: heights are absolute at the leaves, which read
    //! left to right *are* the skyline. The first height is stored
    //! outright; each later one as the difference from its predecessor,
    //! because neighboring plateaus tend to sit close in height even when
    //! both stand very tall. A difference can be negative, so it is first
    //! folded onto the naturals (*zigzag*: `+k → 2k`, `−k → 2k−1`) and then
    //! written as a variable-length integer code (*Elias gamma*, applied to
    //! the number plus one so that zero stays codable) that
    //! spends bits in proportion to the number's width, not its
    //! magnitude — so a run of
    //! similar heights costs a few bits per leaf no matter how tall it
    //! stands. Our example `(0, 1, (0, 0, 2))` becomes five topology flags
    //! and the payload sequence `1, −1, +2` (the absolute `1`, then zigzags
    //! `1` and `4`): sixteen bits in all.
    //!
    //! ```
    //! use before::Version;
    //! let v: Version = "(0, 1, (0, 0, 2))".parse().unwrap();
    //! assert_eq!(v.encoded_bits(), 16);
    //! ```
    //!
    //! **Canonical form.** Each skyline has exactly one spelling. The
    //! topology must be minimal — `(0, (0, 1, 1), 0)` draws the same
    //! function as `(0, 1, 0)`, so equal sibling leaf plateaus always
    //! merge — and no delta may drive a height below zero. (Heights being
    //! absolute, the paper's other normalization, lifting a common minimum
    //! into the parent, has no analogue here.) Unique spelling is what buys
    //! the cheap guarantees: byte equality *is* value equality, so `==` and
    //! hashing are byte operations, and decode can afford to reject rather
    //! than repair — every valid value has exactly one acceptable input.
    //!
    //! ## The sweep kernels
    //!
    //! Every operation is one left-to-right sweep over its operands'
    //! streams, and no sweep ever materializes a node tree.
    //!
    //! Take the join of `a = (0, 1, 0)` and `b = (0, 0, (0, 0, 2))`. Their
    //! plateau boundaries differ — `a` steps at ½, `b` at ¾ — so the sweep
    //! overlays the two partitions and walks the pieces on which both are
    //! flat: `[0, ½)`, `[½, ¾)`, `[¾, 1)`. On each piece it takes the
    //! pointwise max — `max(1, 0)`, `max(0, 0)`, `max(0, 2)` — and what it
    //! carries between pieces is not two absolute heights but one running
    //! *difference* between the sides, updated from the deltas the streams
    //! themselves supply. (The output is delta-coded like its inputs, so
    //! each emitted step falls out of the two input steps and that running
    //! difference — no absolute height is ever reconstructed.) Comparison
    //! is the same walk with no output: the sign of that difference,
    //! watched across the whole sweep, settles `<`, `>`, `==`, or
    //! concurrent. The two-operand measures ride the same aligned walk:
    //! [`distance`](crate::Version::distance) accumulates the area the
    //! operands don't share, [`lag`](crate::Version::lag) its one-way half
    //! (what the other side holds that `self` does not). The one-operand
    //! measures are sweeps of a single stream —
    //! [`rank`](crate::Version::rank) accumulates area,
    //! [`min_ticks`](crate::Version::min_ticks) the fewest events that
    //! could have built the skyline — and the projection `v / &p` sweeps
    //! `v`'s stream against party `p`'s, keeping the skyline where `p`
    //! owns and zeroing it elsewhere.
    //!
    //! ```
    //! use before::Version;
    //! let a: Version = "(0, 1, 0)".parse().unwrap();
    //! let b: Version = "(0, 0, (0, 0, 2))".parse().unwrap();
    //! assert_eq!((&a | &b).to_string(), "(0, 1, (0, 0, 2))");
    //! ```
    //!
    //! The join's result must itself be canonical, and the emitting sweep
    //! makes it so *while streaming*: output plateaus feed a collapsing
    //! builder that derives the result's topology from their depths and
    //! merges equal sibling leaves the moment the second of a pair
    //! completes. A merge can cascade upward, but only through the
    //! ancestors still open on the right edge of the tree, so the builder
    //! holds just that pending spine — state bounded by depth — and the
    //! result is born in normal form, never normalized after the fact.
    //!
    //! A tick is the one asymmetric sweep: it pairs the party's id stream
    //! against the version's and first plays the paper's `fill`, collapsing
    //! every subtree the party wholly owns to a single plateau at that
    //! subtree's maximum height. If filling changed the stream
    //! anywhere, the flattening itself recorded the event. If it changed
    //! nothing, the same walk has already scored every point where the
    //! party could grow instead — cheapest by fewest added nodes, then by
    //! shallowest depth — and one splice rebuilds exactly the winning
    //! root-to-leaf path, copying everything off that path as verbatim bit
    //! ranges.
    //!
    //! ```
    //! use before::{Party, Version};
    //! let p: Party = "(1, 0)".parse().unwrap();
    //! let mut v: Version = "(0, 1, (0, 0, 2))".parse().unwrap();
    //! v.tick(&p); // p's half is already flat: fill changes nothing, so grow
    //! assert_eq!(v.to_string(), "(0, 2, (0, 0, 2))");
    //! ```
    //!
    //! Even strict decoding is a sweep: validation replays the topology on
    //! a couple of bits per open ancestor and one running height for the
    //! nonnegativity check, never a parsed tree.
    //!
    //! Working this way, the kernels touch memory the way caches like —
    //! forward, densely, once — and transient state beyond the output
    //! being built stays a couple of bits per open ancestor, however the
    //! operands are shaped. Heights are
    //! arbitrary-precision (a tick can always raise a plateau past any
    //! fixed width): in the stream a height is just its code, however wide,
    //! and during a sweep a decoded value lives inline in machine words
    //! until it outgrows two of them, only then spilling to a heap integer.
    //! And no operand shape can overflow the call stack: traversals either
    //! run iteratively or move the stack onto the heap before descending
    //! (recursion resumes on a freshly allocated segment when the native
    //! stack runs low).
    //!
    //! ## The trades
    //!
    //! **Compactness over random access.** A packed stream has no `O(1)`
    //! subtree access: every question is answered from the front. That is
    //! the right trade here because the API asks no random-access
    //! questions — comparison, join, meet, tick, and the measures are
    //! whole-value operations, and the packed form makes each a linear scan
    //! over a few dozen to a few thousand contiguous bytes. What the trade
    //! rules out is cheap point queries ("the height at this id"), which
    //! the public API deliberately does not offer.
    //!
    //! **Small values over large.** Elias gamma is one member of a family
    //! of integer codes, so the pick deserves its argument. The stream
    //! demands two things of any candidate: exactly one prefix-free
    //! spelling per natural, because unique spelling is what canonical
    //! form rests on; and cost proportional to a value's *width* (its bit
    //! count), never its magnitude, because arbitrarily tall plateaus are
    //! legitimate values. The second demand excludes the Rice codes
    //! outright — the classic pick for delta streams, but a unary quotient
    //! makes their length linear in the value — and leaves the universal
    //! codes: Elias gamma, delta, and omega, and the zeta family beside
    //! them.
    //!
    //! What decides among the survivors is where the stored values
    //! actually fall, and that is a measurement: re-running the
    //! space-consumption experiment behind the crate docs'
    //! [Efficiency](crate#efficiency) figures and histogramming every
    //! value handed to the coder — the first absolute height and every
    //! zigzagged delta — in both of the paper's workload regimes, data
    //! causality under membership churn and process causality among a
    //! fixed set. The distribution that emerges is small-valued but not
    //! zero-heavy: zeros are only 27% of the churning regime's values
    //! (10.5% of the fixed regime's), so the one-bit zero is not the whole
    //! story — the code must be cheap across the small band, not just at
    //! zero — and 85–93% of values are 15 or less. The pointwise
    //! comparison is then arithmetic: gamma is better than or tied with
    //! delta and omega on every value below 31 — deltas in `[−15, +15]` —
    //! and loses above the band, to delta immediately, to omega only from
    //! 127 up: out where the mass never goes. Where the mass sits, gamma
    //! wins. Its price is two bits per doubling — `2·⌊log2(v + 1)⌋ + 1`
    //! bits for value `v` — visible on single-plateau versions, whose
    //! stream is one topology flag and one absolute height:
    //!
    //! ```
    //! use before::Version;
    //! let bits = |s: &str| s.parse::<Version>().unwrap().encoded_bits();
    //! assert_eq!(bits("15"), bits("7") + 2); // one doubling: two bits
    //! assert_eq!(bits("1000000"), bits("1000") + 20); // ten doublings: twenty
    //! ```
    //!
    //! There is a tidy frame for how narrowly the shape picks gamma. The
    //! *zeta* codes ζₖ make the small-versus-large trade a dial: raising
    //! `k` cheapens wide values at the expense of the narrowest ones — ζ₂
    //! spends two bits on a zero where gamma spends one, and about a bit
    //! and a half per doubling where gamma spends two — and gamma *is* ζ₁,
    //! the member that bets hardest on small. The two regimes bracket the
    //! dial's low end — the churning regime's histogram fits ζ₁, the fixed
    //! regime's ζ₂ — and the churning regime, where gamma wins outright,
    //! produces about nine tenths of the experiment's bytes. Over the
    //! combined corpus the rivalry is a hair's width: ζ₂ would eke back
    //! 0.17% of bytes, far too small a saving to buy a wire-format change,
    //! while delta and omega cost 6–9% more.
    //!
    //! The worst-case metric — distance above the information-theoretic
    //! floor — reads against gamma, and bounds what any rival could buy.
    //! Count the versions whose canonical stream fits in `n` bits: any
    //! injective coding must spend at least `log2` of that count on some
    //! member, and this stream's worst member spends `n`. Derived from the
    //! coding grammar and cross-checked by exact census: 1.043
    //! asymptotically, about 1.067 at 100-byte versions, where delta or
    //! omega would reach exactly 1 in the limit. The gap is not slack in
    //! the code itself — topology-plus-gamma spends the whole code space,
    //! so every input decode rejects is a spelling canonical form
    //! excludes, not a wasted pattern — and what that exclusion costs
    //! depends on where a code puts its weight: gamma's canonical family
    //! is dominated by many-leaf, small-delta streams, where the
    //! sibling-merge rule bites at every node, while delta and omega would
    //! shift the family onto few-leaf, giant-valued streams the rule
    //! barely touches. Their
    //! asymptotic tightness is bought with cheap giant values — exactly
    //! what the measured workload does not produce; hence the 6–9% on real
    //! traffic. (And the floor is relative to the set a coding covers:
    //! against families of uniformly tall plateaus the ratio would instead
    //! approach 2 — the price of betting the cheap spellings on the small
    //! steps organic histories actually produce.)
    //!
    //! What would reopen the choice is the histogram moving, not a better
    //! argument: a deployment dominated by fixed-membership histories
    //! pushes the dial toward ζ₂ and turns that hair's width into a real
    //! margin. The decision would be taken the way this one was — by
    //! re-measuring.
    //!
    //! **Strictness over tolerance.** [`decode`](crate::Version::decode)
    //! rejects every non-canonical spelling rather than normalizing it.
    //! Repair would be friendlier to bytes built by hand — a foreign
    //! implementation, a debugging human — but it would break the identity
    //! that equality, hashing, and cross-replica agreement rest on: byte
    //! equality *is* value equality.
    //!
    //! How we convince ourselves all of this is correct is the [crate
    //! docs](crate)' Testing section: every kernel is pinned differentially
    //! against the paper's recursive trees, which stay in-tree as the
    //! oracle.
}

// No outer doc comment: one here would merge with the module's inner docs
// and shift their link resolution to this scope, where the module's
// test-gated items don't resolve. The module documents itself.
mod recurse;

/// Reference oracle: the paper's recursive trees; ground truth for the
/// differential tests. Public under the `oracle` feature so the benchmark suite
/// can time it against the optimized implementation.
#[cfg(any(test, feature = "oracle"))]
pub mod oracle;

/// Adversarial input generators and deterministic resource meters, the
/// instruments behind the resource-proportionality envelopes. Public under
/// the `meter` feature so the metering test binaries can drive them.
#[cfg(any(test, feature = "meter"))]
pub mod meter;

/// The algebraic and representational laws of the public API, as named
/// predicates. Public under the `laws` feature so the fuzz workspace can
/// drive the same collection the in-tree proptests assert.
#[cfg(any(test, feature = "laws"))]
pub mod laws;

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
