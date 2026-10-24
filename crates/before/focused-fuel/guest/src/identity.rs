//! Probes of the identity fast paths: operands that are clones of one
//! another, byte-equal in distinct buffers, or empty.
//!
//! Every operand is a dense spine (`Shape::Dense`) of the probe's depth, so
//! the probes vary the operands' size while their structure stays alike.

use std::cmp::Ordering;

use before::testing::meter::registry::Shape;
use before::{Rank, Version};

use crate::measured;

/// Builds the dense spine of `depth` levels.
fn spine(depth: usize) -> Version {
    Shape::Dense.build1(depth).version()
}

/// Builds a dense spine and a clone of it, which shares its buffer.
fn spine_and_clone(depth: usize) -> (Version, Version) {
    let spine = spine(depth);
    let clone = spine.clone();
    (spine, clone)
}

/// Builds a dense spine and a byte-equal copy decoded from its encoding,
/// which holds a buffer of its own.
fn spine_and_redecoded(depth: usize) -> (Version, Version) {
    let spine = spine(depth);
    let redecoded =
        Version::decode(&spine.encode()[..]).expect("a stored version's encoding decodes");
    (spine, redecoded)
}

/// Measures `partial_cmp` of a spine with its clone.
pub fn clone_compare(depth: usize) -> u64 {
    let (v, c) = spine_and_clone(depth);
    measured(
        || v.partial_cmp(&c),
        |order| assert_eq!(order, Some(Ordering::Equal), "a version equals its clone"),
    )
}

/// Measures `|` of a spine with its clone.
pub fn clone_join(depth: usize) -> u64 {
    let (v, c) = spine_and_clone(depth);
    measured(|| &v | &c, |join| assert_eq!(join, v, "join is idempotent"))
}

/// Measures `&` of a spine with its clone.
pub fn clone_meet(depth: usize) -> u64 {
    let (v, c) = spine_and_clone(depth);
    measured(|| &v & &c, |meet| assert_eq!(meet, v, "meet is idempotent"))
}

/// Measures `span` of a spine with its clone.
pub fn clone_span(depth: usize) -> u64 {
    let (v, c) = spine_and_clone(depth);
    measured(
        || v.span(&c),
        |span| assert_eq!(span.lo(), &v, "a version's span with itself is coincident"),
    )
}

/// Measures `join_all` of a spine over its clone and itself.
pub fn clone_join_all(depth: usize) -> u64 {
    let (v, c) = spine_and_clone(depth);
    measured(
        || v.join_all([&c, &v]),
        |join| assert_eq!(join, v, "join is idempotent"),
    )
}

/// Measures `meet_all` of a spine over its clone and itself.
pub fn clone_meet_all(depth: usize) -> u64 {
    let (v, c) = spine_and_clone(depth);
    measured(
        || v.meet_all([&c, &v]),
        |meet| assert_eq!(meet, v, "meet is idempotent"),
    )
}

/// Measures `span_all` of a spine over its clone and itself.
pub fn clone_span_all(depth: usize) -> u64 {
    let (v, c) = spine_and_clone(depth);
    measured(
        || v.span_all([&c, &v]),
        |span| assert_eq!(span.hi(), &v, "a version's hull with itself is coincident"),
    )
}

/// Measures `distance` of a spine to its clone.
pub fn clone_distance(depth: usize) -> u64 {
    let (v, c) = spine_and_clone(depth);
    measured(
        || v.distance(&c),
        |distance| assert_eq!(distance, Rank::ZERO, "a version is no distance from itself"),
    )
}

/// Measures `lag` of a spine behind its clone.
pub fn clone_lag(depth: usize) -> u64 {
    let (v, c) = spine_and_clone(depth);
    measured(
        || v.lag(&c),
        |lag| assert_eq!(lag, Rank::ZERO, "a version does not lag itself"),
    )
}

/// Measures `partial_cmp` of a spine with its byte-equal re-decode.
pub fn redecoded_compare(depth: usize) -> u64 {
    let (v, r) = spine_and_redecoded(depth);
    measured(
        || v.partial_cmp(&r),
        |order| {
            assert_eq!(
                order,
                Some(Ordering::Equal),
                "byte-equal versions are equal"
            )
        },
    )
}

/// Measures `|` of a spine with its byte-equal re-decode.
pub fn redecoded_join(depth: usize) -> u64 {
    let (v, r) = spine_and_redecoded(depth);
    measured(|| &v | &r, |join| assert_eq!(join, v, "join is idempotent"))
}

/// Measures `&` of a spine with its byte-equal re-decode.
pub fn redecoded_meet(depth: usize) -> u64 {
    let (v, r) = spine_and_redecoded(depth);
    measured(|| &v & &r, |meet| assert_eq!(meet, v, "meet is idempotent"))
}

/// Measures `span` of a spine with its byte-equal re-decode.
pub fn redecoded_span(depth: usize) -> u64 {
    let (v, r) = spine_and_redecoded(depth);
    measured(
        || v.span(&r),
        |span| assert_eq!(span.lo(), &v, "a version's span with itself is coincident"),
    )
}

/// Measures `distance` of a spine to its byte-equal re-decode.
pub fn redecoded_distance(depth: usize) -> u64 {
    let (v, r) = spine_and_redecoded(depth);
    measured(
        || v.distance(&r),
        |distance| assert_eq!(distance, Rank::ZERO, "equal versions are no distance apart"),
    )
}

/// Measures `lag` of a spine behind its byte-equal re-decode.
pub fn redecoded_lag(depth: usize) -> u64 {
    let (v, r) = spine_and_redecoded(depth);
    measured(
        || v.lag(&r),
        |lag| assert_eq!(lag, Rank::ZERO, "equal versions do not lag each other"),
    )
}

/// Measures `empty | v`.
pub fn empty_join_left(depth: usize) -> u64 {
    let (v, empty) = (spine(depth), Version::new());
    measured(
        || &empty | &v,
        |join| assert_eq!(join, v, "the empty version is join's identity"),
    )
}

/// Measures `v | empty`.
pub fn empty_join_right(depth: usize) -> u64 {
    let (v, empty) = (spine(depth), Version::new());
    measured(
        || &v | &empty,
        |join| assert_eq!(join, v, "the empty version is join's identity"),
    )
}

/// Measures `empty & v`.
pub fn empty_meet_left(depth: usize) -> u64 {
    let (v, empty) = (spine(depth), Version::new());
    measured(
        || &empty & &v,
        |meet| assert_eq!(meet, empty, "the empty version absorbs meet"),
    )
}

/// Measures `v & empty`.
pub fn empty_meet_right(depth: usize) -> u64 {
    let (v, empty) = (spine(depth), Version::new());
    measured(
        || &v & &empty,
        |meet| assert_eq!(meet, empty, "the empty version absorbs meet"),
    )
}

/// Measures `empty.span(v)`.
pub fn empty_span_left(depth: usize) -> u64 {
    let (v, empty) = (spine(depth), Version::new());
    measured(
        || empty.span(&v),
        |span| {
            assert_eq!(
                span.hi(),
                &v,
                "the hull of the empty version and v reaches v"
            )
        },
    )
}

/// Measures `v.span(empty)`.
pub fn empty_span_right(depth: usize) -> u64 {
    let (v, empty) = (spine(depth), Version::new());
    measured(
        || v.span(&empty),
        |span| {
            assert_eq!(
                span.hi(),
                &v,
                "the hull of v and the empty version reaches v"
            )
        },
    )
}

/// Measures `acc |= v` on an empty accumulator, the first join a fold makes.
pub fn empty_join_assign_left(depth: usize) -> u64 {
    let v = spine(depth);
    let mut acc = Version::new();
    measured(
        || {
            acc |= &v;
            acc
        },
        |acc| assert_eq!(acc, v, "the empty version is join's identity"),
    )
}

/// Measures `acc |= empty` on an accumulator holding `v`.
pub fn empty_join_assign_right(depth: usize) -> u64 {
    let (v, empty) = (spine(depth), Version::new());
    let mut acc = v.clone();
    measured(
        || {
            acc |= &empty;
            acc
        },
        |acc| assert_eq!(acc, v, "the empty version is join's identity"),
    )
}

/// Measures `acc &= v` on an empty accumulator.
pub fn empty_meet_assign_left(depth: usize) -> u64 {
    let (v, empty) = (spine(depth), Version::new());
    let mut acc = Version::new();
    measured(
        || {
            acc &= &v;
            acc
        },
        |acc| assert_eq!(acc, empty, "the empty version absorbs meet"),
    )
}

/// Measures `acc &= empty` on an accumulator holding `v`.
pub fn empty_meet_assign_right(depth: usize) -> u64 {
    let (v, empty) = (spine(depth), Version::new());
    let mut acc = v.clone();
    measured(
        || {
            acc &= &empty;
            acc
        },
        |acc| assert_eq!(acc, empty, "the empty version absorbs meet"),
    )
}

/// Measures `v.join_all([empty])`.
pub fn empty_join_all_right(depth: usize) -> u64 {
    let (v, empty) = (spine(depth), Version::new());
    measured(
        || v.join_all([&empty]),
        |join| assert_eq!(join, v, "the empty version is join's identity"),
    )
}

/// Measures `empty.join_all([v])`.
pub fn empty_join_all_left(depth: usize) -> u64 {
    let (v, empty) = (spine(depth), Version::new());
    measured(
        || empty.join_all([&v]),
        |join| assert_eq!(join, v, "the empty version is join's identity"),
    )
}

/// Measures `v.meet_all([empty])`.
pub fn empty_meet_all_right(depth: usize) -> u64 {
    let (v, empty) = (spine(depth), Version::new());
    measured(
        || v.meet_all([&empty]),
        |meet| assert_eq!(meet, empty, "the empty version absorbs meet"),
    )
}

/// Measures `empty.meet_all([v])`.
pub fn empty_meet_all_left(depth: usize) -> u64 {
    let (v, empty) = (spine(depth), Version::new());
    measured(
        || empty.meet_all([&v]),
        |meet| assert_eq!(meet, empty, "the empty version absorbs meet"),
    )
}
