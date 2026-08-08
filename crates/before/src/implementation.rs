//! How [`before`](crate) works (documentation only).
//!
//! End-user documentation lives in the [crate docs](crate) and on the
//! public items; here we discuss the design.
//!
//! ## The skyline
//!
//! Start with what a version *is*. The original paper defines a
//! [`Version`](crate::Version) as a function from the unit id interval `[0, 1)`
//! to the naturals: how many events each point of the id space has seen. Plot
//! that function and you get plateaus of differing heights over subintervals,
//! visually akin to a city *skyline*. Here is the version written `(0, 1, (0,
//! 0, 2))` in the paper's tree notation, drawn as the function it denotes:
//!
//! ```text
//! 2 │                  ┌────────
//! 1 │────────┐         │
//! 0 │        └─────────┘
//!   0       1/2       3/4      1
//! ```
//!
//! The skyline is the semantics; a tree is just one way to spell it (in `(n, l,
//! r)` notation, a node's number lifts its whole subtree and each child covers
//! half its parent's interval). Every operation is a pointwise statement about
//! the skyline: causal comparison is pointwise `<=` — one history contains
//! another exactly when the containing skyline is nowhere lower — join `|` is
//! pointwise max, meet `&` is pointwise min, and [`rank`](crate::Version::rank)
//! is the **area under the skyline**: for the drawing above, `1·½ + 0·¼ + 2·¼ =
//! 1`. (An area over dyadic intervals is a dyadic rational, `num · 2⁻ᵉˣᵖ`,
//! which [`Rank`](crate::Rank) keeps exact at any magnitude — the exactness
//! behind its strict-monotonicity guarantee.)
//!
//! A [`Party`](crate::Party) has a skyline reading too, one step simpler: a
//! 0-or-1 landscape over the same interval — 1 where the party owns the id
//! space, 0 where it does not. Disjoint parties are landscapes whose owned
//! regions never overlap, and a tick raises the version's skyline somewhere
//! over the party's owned region: anywhere there will do, since a successor
//! timestamp only has to dominate its predecessor and no other party ever
//! writes that region.
//!
//! ## The packed representation
//!
//! At rest, a party, version, or clock is exactly its wire encoding: one packed
//! bit stream in one heap buffer. There is no node graph behind the API — the
//! tree exists only as the order of bits in the stream — so
//! [`encode`](crate::Version::encode) is a copy of the stored bytes,
//! [`decode`](crate::Version::decode) is one validating pass over bytes read
//! straight into the new value's own storage (nothing is re-encoded or
//! rebuilt), and a value's memory footprint is its wire footprint.
//!
//! **Ids.** The paper writes a party's tree with `1` for an owned leaf, `0` for
//! an unowned one, and `(l, r)` for a node — `(1, 0)` owns exactly the left
//! half. The stream writes that tree in preorder, two bits per node, answering
//! "does a left child follow?" and "does a right child follow?". An unowned
//! region is simply *absent* — its parent's tag already said so, and no bits
//! follow — while the childless tag is a terminal, a wholly owned region. So
//! the seed, owning everything, is one terminal: two bits; and `(1, 0)` is a
//! left-only node and then its terminal: four bits. (An owns-nothing party has
//! no spelling: parties are non-empty by construction, and
//! [`without`](crate::Party::without) returns `None` sooner than spell one.)
//!
//! ```
//! use before::Party;
//! assert_eq!(Party::seed().encoded_bits(), 2);
//! assert_eq!("(1, 0)".parse::<Party>().unwrap().encoded_bits(), 4);
//! ```
//!
//! **Versions.** A version's tree is one topology flag per preorder node — `0`
//! for an internal node, `1` for a leaf — with each leaf's plateau height
//! following its flag in the stream. Unlike the paper's trees, interior nodes
//! carry no numbers: heights are absolute at the leaves, which read left to
//! right *are* the skyline. And because an interior node contributes only its
//! `0`, a descent to a leaf is a single run of `0`s ended by the leaf's `1` —
//! one unary read. The first height is stored outright; each later one as the
//! difference from its predecessor, because neighboring plateaus tend to sit
//! close in height even when both stand very tall. A difference can be
//! negative, so it is first folded onto the naturals (*zigzag*: `+k → 2k`, `−k
//! → 2k−1`) and then written as a variable-length integer code (*Elias gamma*,
//! applied to the number plus one so that zero stays codable) that spends bits
//! in proportion to the number's width, not its magnitude — so a run of similar
//! heights costs a few bits per leaf no matter how tall it stands. Our example
//! `(0, 1, (0, 0, 2))` becomes five topology flags and the payload sequence `1,
//! −1, +2` (the absolute `1`, then zigzags `1` and `4`): sixteen bits in all.
//!
//! ```
//! use before::Version;
//! let v: Version = "(0, 1, (0, 0, 2))".parse().unwrap();
//! assert_eq!(v.encoded_bits(), 16);
//! ```
//!
//! **Canonical form.** Each skyline has exactly one spelling. The topology must
//! be minimal — `(0, (0, 1, 1), 0)` draws the same function as `(0, 1, 0)`, so
//! equal sibling leaf plateaus always merge — and no delta may drive a height
//! below zero. (Heights being absolute, the paper's other normalization,
//! lifting a common minimum into the parent, has no analogue here.) Unique
//! spelling is what buys the cheap guarantees: byte equality *is* value
//! equality, so `==` and hashing are byte operations, and decode can afford to
//! reject rather than repair — every valid value has exactly one acceptable
//! input.
//!
//! ## The sweep kernels
//!
//! Every operation is one left-to-right sweep over its operands'
//! streams, and no sweep ever materializes a node tree.
//!
//! Take the join of `a = (0, 1, 0)` and `b = (0, 0, (0, 0, 2))`. Their plateau
//! boundaries differ — `a` steps at ½, `b` at ¾ — so the sweep overlays the two
//! partitions and walks the pieces on which both are flat: `[0, ½)`, `[½, ¾)`,
//! `[¾, 1)`. On each piece it takes the pointwise max — `max(1, 0)`, `max(0,
//! 0)`, `max(0, 2)` — and what it carries between pieces is not two absolute
//! heights but one running *difference* between the sides, updated from the
//! deltas the streams themselves supply. (The output is delta-coded like its
//! inputs, so each emitted step falls out of the two input steps and that
//! running difference — no absolute height is ever reconstructed.) Comparison
//! is the same walk with no output: the sign of that difference, watched across
//! the whole sweep, settles `<`, `>`, `==`, or concurrent. The two-operand
//! measures ride the same aligned walk: [`distance`](crate::Version::distance)
//! accumulates the area the operands don't share, [`lag`](crate::Version::lag)
//! its one-way half (what the other side holds that `self` does not). The
//! one-operand measures are sweeps of a single stream —
//! [`rank`](crate::Version::rank) accumulates area,
//! [`min_ticks`](crate::Version::min_ticks) the fewest events that could have
//! built the skyline — and the projection view `&v / &p`
//! ([`OwnVersion`](crate::OwnVersion)) defers its sweep: comparing the view
//! runs one fused walk over the operands' streams, `p`'s ownership landscape
//! gating `v`'s heights in flight, while the explicit
//! [`to_version`](crate::OwnVersion::to_version) materialization — the one
//! output that can outgrow its operands — sweeps `v`'s stream against `p`'s,
//! keeping the skyline where `p` owns and zeroing it elsewhere.
//!
//! ```
//! use before::Version;
//! let a: Version = "(0, 1, 0)".parse().unwrap();
//! let b: Version = "(0, 0, (0, 0, 2))".parse().unwrap();
//! assert_eq!((&a | &b).to_string(), "(0, 1, (0, 0, 2))");
//! ```
//!
//! The join's result must itself be canonical, and the emitting sweep makes it
//! so *while streaming*: output plateaus feed a collapsing builder that derives
//! the result's topology from their depths and merges equal sibling leaves the
//! moment the second of a pair completes. A merge can cascade upward, but only
//! through the ancestors still open on the right edge of the tree, so the
//! builder holds just that pending spine — state bounded by depth — and the
//! result is born in normal form, never normalized after the fact.
//!
//! A tick is the one asymmetric sweep: it pairs the party's id stream against
//! the version's and first plays the paper's `fill`, collapsing every subtree
//! the party wholly owns to a single plateau at that subtree's maximum height.
//! If filling changed the stream anywhere, the flattening itself recorded the
//! event. If it changed nothing, the same walk has already scored every point
//! where the party could grow instead — cheapest by fewest added nodes, then by
//! shallowest depth — and one splice rebuilds exactly the winning root-to-leaf
//! path, copying everything off that path as verbatim bit ranges.
//!
//! ```
//! use before::{Party, Version};
//! let p: Party = "(1, 0)".parse().unwrap();
//! let mut v: Version = "(0, 1, (0, 0, 2))".parse().unwrap();
//! v.tick(&p); // p's half is already flat: fill changes nothing, so grow
//! assert_eq!(v.to_string(), "(0, 2, (0, 0, 2))");
//! ```
//!
//! Even strict decoding is a sweep: validation replays the topology on a couple
//! of bits per open ancestor and one running height for the nonnegativity
//! check, never a parsed tree.
//!
//! Working this way, the kernels touch memory the way caches like — forward,
//! densely, once — and transient state beyond the output being built stays a
//! couple of bits per open ancestor, however the operands are shaped. Heights
//! are arbitrary-precision (a tick can always raise a plateau past any fixed
//! width): in the stream a height is just its code, however wide, and during a
//! sweep a decoded value lives inline in machine words until it outgrows two of
//! them, only then spilling to a heap integer. And no operand shape can
//! overflow the call stack: traversals either run iteratively or move the stack
//! onto the heap before descending (recursion resumes on a freshly allocated
//! segment when the native stack runs low).
//!
//! ## The trades
//!
//! **Compactness over random access.** A packed stream has no `O(1)` subtree
//! access: every question is answered from the front. That is the right trade
//! here because the API asks no random-access questions — comparison, join,
//! meet, tick, and the measures are whole-value operations, and the packed form
//! makes each a linear scan over a few dozen to a few thousand contiguous
//! bytes. What the trade rules out is cheap point queries ("the height at this
//! id"), which the public API deliberately does not offer.
//!
//! **Small values over large.** Elias gamma is one member of a family of
//! integer codes, so the pick deserves its argument. The stream demands two
//! things of any candidate: exactly one prefix-free spelling per natural,
//! because unique spelling is what canonical form rests on; and cost
//! proportional to a value's *width* (its bit count), never its magnitude,
//! because arbitrarily tall plateaus are legitimate values. The second demand
//! excludes the Rice codes outright — the classic pick for delta streams, but a
//! unary quotient makes their length linear in the value — and leaves the
//! universal codes: Elias gamma, delta, and omega, and the zeta family beside
//! them.
//!
//! What decides among the survivors is where the stored values actually fall,
//! and that is a measurement, committed as the `code_study` example: re-running
//! the space-consumption experiment behind the crate docs'
//! [Efficiency](crate#efficiency) figures and histogramming every value handed
//! to the coder — the first absolute height and every zigzagged delta — in both
//! of the paper's workload regimes, data causality under membership churn and
//! process causality among a fixed set. The figures quoted below are from a
//! reduced-parameter run of that instrument — ten runs per regime over
//! populations of 4 to 128, at 20,000 iterations in the churning regime and
//! 10,000 in the fixed, a fraction of the full experiment's schedule; the
//! example's constants are those parameters, and its exact per-version
//! reconciliation check ties the histograms to the encoder's own output. The
//! distribution that emerges is small-valued but not zero-heavy: zeros are only
//! 27% of the churning regime's values (10.5% of the fixed regime's), so the
//! one-bit zero is not the whole story — the code must be cheap across the
//! small band, not just at zero — and 85–93% of values are 15 or less. The
//! pointwise comparison is then arithmetic: gamma is better than or tied with
//! delta and omega on every value below 31 — deltas in `[−15, +15]` — and loses
//! above the band, to delta immediately, to omega only from 127 up: out where
//! the mass never goes. Where the mass sits, gamma wins. Its price is two bits
//! per doubling — `2·⌊log2(v + 1)⌋ + 1` bits for value `v` — visible on
//! single-plateau versions, whose stream is one topology flag and one absolute
//! height:
//!
//! ```
//! use before::Version;
//! let bits = |s: &str| s.parse::<Version>().unwrap().encoded_bits();
//! assert_eq!(bits("15"), bits("7") + 2); // one doubling: two bits
//! assert_eq!(bits("1000000"), bits("1000") + 20); // ten doublings: twenty
//! ```
//!
//! There is a tidy frame for how narrowly the shape picks gamma. The *zeta*
//! codes ζₖ make the small-versus-large trade a dial: raising `k` cheapens wide
//! values at the expense of the narrowest ones — ζ₂ spends two bits on a zero
//! where gamma spends one, and about a bit and a half per doubling where gamma
//! spends two — and gamma *is* ζ₁, the member that bets hardest on small. The
//! two regimes bracket the dial's low end — the churning regime's histogram
//! fits ζ₁, the fixed regime's ζ₂ — and the churning regime, where gamma wins
//! outright, produces about nine tenths of the experiment's bytes. Over the
//! combined corpus the rivalry is a hair's width: ζ₂ would eke back 0.17% of
//! bytes, far too small a saving to buy a wire-format change, while delta and
//! omega cost 6–9% more.
//!
//! The worst-case metric — distance above the information-theoretic floor —
//! reads against gamma, and bounds what any rival could buy. Count the versions
//! whose canonical stream fits in `n` bits: any injective coding must spend at
//! least `log2` of that count on some member, and this stream's worst member
//! spends `n`. Derived from the coding grammar and cross-checked by exact
//! census: 1.043 asymptotically, about 1.067 at 100-byte versions, where delta
//! or omega would reach exactly 1 in the limit. The gap is not slack in the
//! code itself — topology-plus-gamma spends the whole code space, so every
//! input decode rejects is a spelling canonical form excludes, not a wasted
//! pattern — and what that exclusion costs depends on where a code puts its
//! weight: gamma's canonical family is dominated by many-leaf, small-delta
//! streams, where the sibling-merge rule bites at every node, while delta and
//! omega would shift the family onto few-leaf, giant-valued streams the rule
//! barely touches. Their asymptotic tightness is bought with cheap giant values
//! — exactly what the measured workload does not produce; hence the 6–9% on
//! real traffic. (And the floor is relative to the set a coding covers: against
//! families of uniformly tall plateaus the ratio would instead approach 2 — the
//! price of betting the cheap spellings on the small steps organic histories
//! actually produce.)
//!
//! **Strictness over tolerance.** [`decode`](crate::Version::decode) rejects
//! every non-canonical spelling rather than normalizing it. Repair would be
//! friendlier to bytes built by hand — a foreign implementation, a debugging
//! human — but it would break the identity that equality, hashing, and
//! cross-replica agreement rest on: byte equality *is* value equality.
//!
//! How we convince ourselves all of this is correct is the [crate docs](crate)'
//! Testing section: every kernel is pinned differentially against the paper's
//! recursive trees, which stay in-tree as the oracle.
