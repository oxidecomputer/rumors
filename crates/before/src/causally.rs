//! Composable causal queries over [`Version`]s.
//!
//! A range over totally ordered values is an interval. [`Version`]s are only
//! partially ordered, so a causal *query* generalizes the idea of an interval
//! into an optional *floor* and *ceiling*, minus a set of *holes*.
//!
//! Queries may be constructed by composing atomic queries, and resolved against
//! [`Version`]s and [`Span`]s to answer questions of
//! [`contains`](Query::contains) and [`coverage`](Query::coverage).
//!
//! # Constructing [`Query`]s
//!
//! Two elementary atoms bound one side each:
//!
//! | atom                  | keeps `v` iff |
//! |-----------------------|---------------|
//! | [`after(p)`](after)   | `p <= v`      |
//! | [`before(e)`](before) | `v <= e`      |
//!
//! Both atoms drop versions [`concurrent`](Version::concurrent) to their bound
//! (which we will write `∥` for brevity below).
//!
//! Concurrency enters through negation; the complement of "comparable, on this
//! side" is "comparable on the other side, or concurrent":
//!
//! | expression                                            | keeps `v` iff       | polarity |
//! |-------------------------------------------------------|---------------------|----------|
//! | [`!before(s)`](before), a.k.a. [`since(s)`](since)    | `v > s` or `v ∥ s`  | [`Down`] |
//! | [`after(s).or_concurrent()`](Floor::or_concurrent)    | `v >= s` or `v ∥ s` | [`Down`] |
//! | [`!after(s)`](after), a.k.a. [`until(s)`](until)      | `v < s` or `v ∥ s`  | [`Up`]   |
//! | [`before(s).or_concurrent()`](Ceiling::or_concurrent) | `v <= s` or `v ∥ s` | [`Up`]   |
//!
//! Expressing "strictly after" or "strictly before" requires excluding the
//! version exactly equal to the bound: [`strictly_after(v)`](strictly_after) is
//! equivalent to `after(v) & !before(v)`, and [`strictly_before`] is equivalent
//! to `before(v) & !after(v)`.
//!
//! Together these are all the atomic causal bounds: each keeps some subset of
//! the four relations `v` can have to the bound (`<`, `=`, `>`, `∥`).
//!
//! # Polarity
//!
//! While it might seem desirable to permit any queries to be composed using
//! `&`, permitting this carries a powerful hidden footgun: exactly deciding the
//! [`coverage`](Query::coverage) of a [`Span`] against a freely constructed
//! [`Query`] with arbitary negation is equivalent to the famously NP-complete
//! SAT problem: exposing this interface would make it easy to express silently
//! exponential queries.
//!
//! Instead, we restrict queries to only those whose verdicts can assuredly be
//! resolved in linear time: those with a uniform *polarity*. We say a [`Query`]
//! has a [`Neutral`] polarity if it is a pure causal range (optional lower
//! bound + optional upper bound) with no other holes; it has a [`Down`]
//! polarity if it excludes sets of versions each defined by their shared
//! *upper* bound; it has an [`Up`] polarity if it excludes sets of versions
//! each defined by their shared *lower* bound.
//!
//! Queries with opposing polarities statically are prohibited from conjunction,
//! which rules out compositions like `!after(v) & !before(w)`, which would
//! define something like "all versions not in the [`Span`] with lower-bound `v`
//! and upper-bound `w`". If you have need in your application of such queries,
//! observe that the verdict returned by such a hypothetical [`Query`] is the
//! same as the logical-`&&` of its expressible components; you can just ask
//! both questions in sequence and branch on both verdicts.
//!
//! # Relationship to [`Span`]s
//!
//! Two concrete versions `lo <= hi` form a [`Span`]: not a query but a value
//! proper, with further operations available on it and a stable wire form. A
//! span converts [`Into`] the query `after(lo) & before(hi)`, which admits
//! exactly the versions the span covers; a [`Version`] converts into the
//! singleton query admitting only itself.
//!
//! # Complexity
//!
//! Atoms and named constructors are `O(1)`.
//!
//! Each pass and walk is linear in its operands' sizes in bytes and
//! stops as soon as its verdict is decided.
//!
//! # Examples
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
//! // Everything `a1` does not already contain.
//! assert!(causally::since(&a1).contains(&a2));
//! assert!(causally::since(&a1).contains(&b1));
//! assert!(!causally::since(&a1).contains(&a1));
//!
//! // Elementary atoms demand a causal relationship; concurrent versions drop.
//! assert!(causally::before(&a2).contains(&a1));
//! assert!(!causally::before(&a2).contains(&b1));
//!
//! // Conjunction composes compatible bounds, in any order.
//! let delta = causally::since(&a1) & causally::before(&a2);
//! assert!(delta.contains(&a2));
//! assert!(!delta.contains(&b1));
//!
//! // Concurrent bound versions are a meaningful query:
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
