//! Composable causal queries over [`Version`]s.
//!
//! A range over totally ordered values is an interval. [`Version`]s
//! are only partially ordered, so a causal *query* generalizes the
//! interval: an optional *floor* and *ceiling*, minus a set of
//! *holes*. Two elementary atoms bound one side each:
//!
//! | atom | keeps `v` iff |
//! |---|---|
//! | [`after(p)`](after) | `p <= v` |
//! | [`before(e)`](before) | `v <= e` |
//!
//! An atom demands an order relation, and order relations fail under
//! incomparability, so both atoms drop versions concurrent to their
//! bound. Concurrency enters through negation: the complement of
//! "comparable, on this side" is "comparable on the other side, or
//! concurrent". Each negated form subtracts a hole:
//!
//! | expression | keeps `v` iff | polarity |
//! |---|---|---|
//! | [`!before(s)`](before), a.k.a. [`since(s)`](since) | `v > s` or `v ∥ s` | [`Down`] |
//! | [`after(s).or_concurrent()`](Floor::or_concurrent) | `v >= s` or `v ∥ s` | [`Down`] |
//! | [`!after(s)`](after), a.k.a. [`until(s)`](until) | `v < s` or `v ∥ s` | [`Up`] |
//! | [`before(s).or_concurrent()`](Ceiling::or_concurrent) | `v <= s` or `v ∥ s` | [`Up`] |
//!
//! The strict forms subtract their own bound, so they carry a hole
//! too: [`strictly_after(p)`](strictly_after) is
//! `after(p) & !before(p)`, and [`strictly_before`] dually. Together
//! these are all the atomic causal bounds: each keeps some subset of
//! the four relations `v` can have to the bound (`<`, `=`, `>`, `∥`).
//!
//! # Polarity
//!
//! A hole subtracts either a down-set ([`Down`], the [`since`]
//! family) or an up-set ([`Up`], the [`until`] family). A query's type
//! carries which kind it may hold; the hole-free default [`Neutral`]
//! conjoins into either. Mixing the polarities —
//! `since(&a) & !after(&b)` — does not compile, and the refusal is
//! deliberate: within one polarity every verdict is exact and cheap,
//! while across both, deciding whether a query empties a span is
//! combinatorial.
//!
//! # Conjunction
//!
//! `&` intersects atoms and queries of compatible polarity. It is
//! total: no pairing that compiles fails at runtime. Floors join,
//! ceilings meet, and redundant holes disappear, so
//! `after(a) & after(b)` is `after(a | b)`. Concurrent bounds are
//! meaningful, not an error: [`delta(&mine, &yours)`](delta) with
//! `mine ∥ yours` asks what you know that I don't — the anti-entropy
//! question.
//!
//! # Membership and coverage
//!
//! [`contains`](Query::contains) answers membership for one version.
//! [`coverage`](Query::coverage) answers it for every version a
//! [`Span`] covers at once, and its [`Coverage`] verdict is exact for
//! every constructible query.
//!
//! # Deliberately absent
//!
//! - **Mixed-polarity conjunction.** `since(&s) & !after(&t)` denotes
//!   a fine predicate, but deciding whether it empties a span encodes
//!   satisfiability. [`coverage`](Query::coverage) could stay exact
//!   there, or cheap, not both; the language keeps both by refusing
//!   the mix. Evaluate two queries when you need it.
//! - **The half-open delta** `since(s) & strictly_before(e)`, for the
//!   same reason: [`strictly_before`] carries an [`Up`] hole. Query
//!   [`delta(s, e)`](delta) and skip the one version equal to `e`.
//! - **Disjunction, and `!` on composites.** `!(a & b)` is
//!   `!a | !b`, a union, and unions would surrender the one-pass
//!   verdicts. Evaluate several queries instead.
//! - **`Eq`, `Hash`, and a wire form.** A query is an ephemeral
//!   filter, not a value: two queries built differently may denote
//!   the same predicate. Observe queries behaviorally, through
//!   [`contains`](Query::contains) and [`coverage`](Query::coverage).
//! - **[`RangeBounds<Version>`](std::ops::RangeBounds).** `a..=b` has
//!   two causal readings — keep or drop versions concurrent to `a` —
//!   and picking one silently is a trap. Every query names its
//!   concurrency treatment through its atoms.
//!
//! # [`Span`]s
//!
//! Two concrete versions `lo <= hi` form a [`Span`]: not a query but a value —
//! the ordered pair itself, with operators, a party quotient, and a wire form.
//! A span converts [`Into`] the query `after(lo) & before(hi)`, which admits
//! exactly the versions the span covers; a [`Version`] converts into the
//! singleton query admitting only itself.
//!
//! # Complexity
//!
//! Atoms and named constructors are `O(1)`; conjunction pays one lattice
//! walk per floor/ceiling merge and one causal comparison per hole pair;
//! membership and coverage are one pass over the probe and every stored
//! bound.
//! Each pass and walk is linear in its operands' packed sizes and stops
//! as soon as its verdict is decided; `coverage` may pay two further
//! lattice walks to close a verdict the pass alone cannot.
//!
//! ```
//! use before::{Clock, causally};
//!
//! let mut alice = Clock::seed();
//! let mut bob = alice.fork();
//! let a1 = alice.tick().clone();
//! let b1 = bob.tick().clone(); // concurrent to a1
//! let a2 = alice.tick().clone(); // a1 < a2
//!
//! // The resume shape: everything `a1` does not already contain.
//! // Versions concurrent to the bound pass — negation is where "or
//! // concurrent" enters.
//! assert!(causally::since(&a1).contains(&a2));
//! assert!(causally::since(&a1).contains(&b1));
//! assert!(!causally::since(&a1).contains(&a1));
//!
//! // Elementary atoms demand the relation: concurrent versions drop.
//! assert!(causally::before(&a2).contains(&a1));
//! assert!(!causally::before(&a2).contains(&b1));
//!
//! // Conjunction composes compatible bounds, in any order, totally.
//! let delta = causally::since(&a1) & causally::before(&a2);
//! assert!(delta.contains(&a2));
//! assert!(!delta.contains(&b1));
//!
//! // Concurrent bound versions are a meaningful query, not an error:
//! // what `b1` knows that `a1` doesn't.
//! let anti_entropy = causally::delta(&a1, &b1);
//! assert!(anti_entropy.contains(&b1));
//! assert!(!anti_entropy.contains(&a1));
//! ```

use std::cmp::Ordering;

mod conjunction;
mod convert;
mod forms;
mod polarity;
mod query;

#[cfg(test)]
mod tests;

pub use crate::error::Crossed;
pub use crate::span::{Dominance, Endpoint, OwnSpan, Placement, Span};

pub use forms::{
    after, all, before, delta, since, strictly_after, strictly_before, toward, until, Ceiling,
    Floor,
};
pub use polarity::{Down, Neutral, Polarity, Up};
pub use query::{Coverage, Query};

use crate::Version;

/// `a <= b` under the causal order.
fn le(a: &Version, b: &Version) -> bool {
    matches!(a.partial_cmp(b), Some(Ordering::Less | Ordering::Equal))
}

/// `a < b` under the causal order.
fn lt(a: &Version, b: &Version) -> bool {
    a.partial_cmp(b) == Some(Ordering::Less)
}
