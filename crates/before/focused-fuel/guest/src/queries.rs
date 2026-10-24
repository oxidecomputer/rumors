//! Probes of comparisons, causal queries, and span verdicts over the history
//! fixture.
//!
//! Every answer is checked outside the measured window against the
//! documented meaning of the operation, computed from the directed
//! comparisons `<=` and `>=`, which take a separate path from the walks
//! these probes measure.

use std::cmp::Ordering;

use before::causally::{self, Query};
use before::{Dominance, Endpoint, Placement, Precedence, Span, Version};

use crate::history::{redecoded, History};
use crate::measured;

/// Whether `x <= y` in the causal order.
pub(crate) fn le(x: &Version, y: &Version) -> bool {
    x <= y
}

/// The causal relation of `x` to `y`, from the directed comparisons.
fn relation(x: &Version, y: &Version) -> Option<Ordering> {
    match (le(x, y), le(y, x)) {
        (true, true) => Some(Ordering::Equal),
        (true, false) => Some(Ordering::Less),
        (false, true) => Some(Ordering::Greater),
        (false, false) => None,
    }
}

/// The placement [`Placement`] documents for `probe` against `[lo, hi]`.
pub(crate) fn placement(lo: &Version, hi: &Version, probe: &Version) -> Placement {
    match (relation(probe, lo), relation(probe, hi)) {
        (None, None) => Placement::Concurrent(Endpoint::Both),
        (None, _) => Placement::Concurrent(Endpoint::Start),
        (_, None) => Placement::Concurrent(Endpoint::End),
        (Some(Ordering::Less), _) => Placement::Before,
        (_, Some(Ordering::Greater)) => Placement::After,
        (Some(Ordering::Equal), Some(Ordering::Equal)) => Placement::At(Endpoint::Both),
        (Some(Ordering::Equal), _) => Placement::At(Endpoint::Start),
        (_, Some(Ordering::Equal)) => Placement::At(Endpoint::End),
        _ => Placement::Between,
    }
}

/// The dominance [`Dominance`] documents for `probe` against `[lo, hi]`.
pub(crate) fn dominance(lo: &Version, hi: &Version, probe: &Version) -> Dominance {
    if le(hi, probe) {
        Dominance::After
    } else if le(lo, probe) {
        Dominance::Between
    } else {
        Dominance::Before
    }
}

/// The precedence [`Precedence`] documents for `probe` against `[lo, hi]`.
pub(crate) fn precedence(lo: &Version, hi: &Version, probe: &Version) -> Precedence {
    if le(probe, lo) {
        Precedence::Before
    } else if le(probe, hi) {
        Precedence::Between
    } else {
        Precedence::After
    }
}

/// Measures `x.partial_cmp(y)`.
pub fn compare(senders: usize, packed: u64) -> u64 {
    let [x, y] = History::new(senders).snapshots(packed);
    measured(
        || x.partial_cmp(&y),
        |order| assert_eq!(order, relation(&x, &y), "partial_cmp agrees with <= and >="),
    )
}

/// Measures `x.partial_cmp(x')`, where `x'` is the byte-equal re-decode of
/// `x`.
pub fn redecoded_compare(senders: usize, packed: u64) -> u64 {
    let [x] = History::new(senders).snapshots(packed);
    let copy = redecoded(&x);
    measured(
        || x.partial_cmp(&copy),
        |order| {
            assert_eq!(
                order,
                Some(Ordering::Equal),
                "byte-equal versions are equal"
            )
        },
    )
}

/// Measures `Query::from(causally::before(x)).contains(y)`.
pub fn before(senders: usize, packed: u64) -> u64 {
    let [x, y] = History::new(senders).snapshots(packed);
    let query = Query::from(causally::before(&x));
    measured(
        || query.contains(&y),
        |held| assert_eq!(held, le(&y, &x), "before(x) holds exactly y <= x"),
    )
}

/// Measures `causally::before(x).contains(y)`.
pub fn before_bound(senders: usize, packed: u64) -> u64 {
    let [x, y] = History::new(senders).snapshots(packed);
    let ceiling = causally::before(&x);
    measured(
        || ceiling.contains(&y),
        |held| assert_eq!(held, le(&y, &x), "before(x) holds exactly y <= x"),
    )
}

/// Measures `causally::after(x).contains(y)`.
pub fn after_bound(senders: usize, packed: u64) -> u64 {
    let [x, y] = History::new(senders).snapshots(packed);
    let floor = causally::after(&x);
    measured(
        || floor.contains(&y),
        |held| assert_eq!(held, le(&x, &y), "after(x) holds exactly x <= y"),
    )
}

/// Measures `causally::since(x).contains(y)`.
pub fn since(senders: usize, packed: u64) -> u64 {
    let [x, y] = History::new(senders).snapshots(packed);
    let query = causally::since(&x);
    measured(
        || query.contains(&y),
        |held| assert_eq!(held, !le(&y, &x), "since(x) holds exactly y not <= x"),
    )
}

/// Measures `(causally::since(x) & causally::before(y)).contains(z)`.
pub fn since_before(senders: usize, packed: u64) -> u64 {
    let [x, y, z] = History::new(senders).snapshots(packed);
    let query = causally::since(&x) & causally::before(&y);
    measured(
        || query.contains(&z),
        |held| {
            assert_eq!(
                held,
                !le(&z, &x) && le(&z, &y),
                "since(x) & before(y) holds exactly z not <= x and z <= y"
            );
        },
    )
}

/// Builds the span `[lo, hi]` and the probe that parameter `packed` names.
fn span_and_probe(senders: usize, packed: u64) -> (Version, Version, Version) {
    let [lo, hi, probe] = History::new(senders).snapshots(packed);
    (lo, hi, probe)
}

/// Builds the span `[lo, hi]`.
fn span<'a>(lo: &'a Version, hi: &'a Version) -> Span<'a> {
    Span::new(lo, hi).expect("the probe's span endpoints are ordered")
}

/// Measures `Span::new(lo, hi).place(probe)`.
pub fn span_place(senders: usize, packed: u64) -> u64 {
    let (lo, hi, probe) = span_and_probe(senders, packed);
    let span = span(&lo, &hi);
    measured(
        || span.place(&probe),
        |verdict| {
            assert_eq!(
                verdict,
                placement(&lo, &hi, &probe),
                "the documented placement"
            )
        },
    )
}

/// Measures `Span::new(lo, hi).dominance(probe)`.
pub fn span_dominance(senders: usize, packed: u64) -> u64 {
    let (lo, hi, probe) = span_and_probe(senders, packed);
    let span = span(&lo, &hi);
    measured(
        || span.dominance(&probe),
        |verdict| {
            assert_eq!(
                verdict,
                dominance(&lo, &hi, &probe),
                "the documented dominance"
            )
        },
    )
}

/// Measures `Span::new(lo, hi).precedence(probe)`.
pub fn span_precedence(senders: usize, packed: u64) -> u64 {
    let (lo, hi, probe) = span_and_probe(senders, packed);
    let span = span(&lo, &hi);
    measured(
        || span.precedence(&probe),
        |verdict| {
            assert_eq!(
                verdict,
                precedence(&lo, &hi, &probe),
                "the documented precedence"
            );
        },
    )
}

/// The membership [`Span::contains`] documents: `lo <= probe <= hi`.
pub(crate) fn contained(lo: &Version, hi: &Version, probe: &Version) -> bool {
    le(lo, probe) && le(probe, hi)
}

/// Measures `Span::new(lo, hi).contains(&probe)`.
pub fn span_contains(senders: usize, packed: u64) -> u64 {
    let (lo, hi, probe) = span_and_probe(senders, packed);
    let span = span(&lo, &hi);
    measured(
        || span.contains(&probe),
        |held| assert_eq!(held, contained(&lo, &hi, &probe), "lo <= probe <= hi"),
    )
}

/// Measures `Span::new(lo, hi).contains(probe.clone())`.
pub fn span_contains_owned(senders: usize, packed: u64) -> u64 {
    let (lo, hi, probe) = span_and_probe(senders, packed);
    let span = span(&lo, &hi);
    measured(
        || span.contains(probe.clone()),
        |held| assert_eq!(held, contained(&lo, &hi, &probe), "lo <= probe <= hi"),
    )
}

/// Measures `Span::new(lo, hi).contains(Span::at(&probe))`.
pub fn span_contains_point(senders: usize, packed: u64) -> u64 {
    let (lo, hi, probe) = span_and_probe(senders, packed);
    let span = span(&lo, &hi);
    measured(
        || span.contains(Span::at(&probe)),
        |held| assert_eq!(held, contained(&lo, &hi, &probe), "lo <= probe <= hi"),
    )
}

/// Measures `Span::new(lo, hi).contains(&coincident)`, where `coincident`
/// spans the probe and its byte-equal re-decode.
pub fn span_contains_distinct_point(senders: usize, packed: u64) -> u64 {
    let (lo, hi, probe) = span_and_probe(senders, packed);
    let span = span(&lo, &hi);
    let copy = redecoded(&probe);
    let coincident = Span::new(&probe, &copy).expect("equal endpoints are a valid span");
    measured(
        || span.contains(&coincident),
        |held| assert_eq!(held, contained(&lo, &hi, &probe), "lo <= probe <= hi"),
    )
}

/// Builds the endpoint and probe that parameter `packed` names.
fn endpoint_and_probe(senders: usize, packed: u64) -> (Version, Version) {
    let [endpoint, probe] = History::new(senders).snapshots(packed);
    (endpoint, probe)
}

/// Measures `endpoint.span(&endpoint.clone()).place(probe)`.
pub fn coincident_place(senders: usize, packed: u64) -> u64 {
    let (endpoint, probe) = endpoint_and_probe(senders, packed);
    let span = endpoint.span(&endpoint.clone());
    measured(
        || span.place(&probe),
        |verdict| {
            assert_eq!(
                verdict,
                placement(&endpoint, &endpoint, &probe),
                "the documented placement"
            );
        },
    )
}

/// Measures `endpoint.span(&endpoint.clone()).dominance(probe)`.
pub fn coincident_dominance(senders: usize, packed: u64) -> u64 {
    let (endpoint, probe) = endpoint_and_probe(senders, packed);
    let span = endpoint.span(&endpoint.clone());
    measured(
        || span.dominance(&probe),
        |verdict| {
            assert_eq!(
                verdict,
                dominance(&endpoint, &endpoint, &probe),
                "the documented dominance"
            );
        },
    )
}

/// Measures `Span::new(endpoint, redecoded).place(probe)`.
pub fn distinct_place(senders: usize, packed: u64) -> u64 {
    let (endpoint, probe) = endpoint_and_probe(senders, packed);
    let copy = redecoded(&endpoint);
    let span = span(&endpoint, &copy);
    measured(
        || span.place(&probe),
        |verdict| {
            assert_eq!(
                verdict,
                placement(&endpoint, &endpoint, &probe),
                "the documented placement"
            );
        },
    )
}

/// Measures `Span::new(endpoint, redecoded).dominance(probe)`.
pub fn distinct_dominance(senders: usize, packed: u64) -> u64 {
    let (endpoint, probe) = endpoint_and_probe(senders, packed);
    let copy = redecoded(&endpoint);
    let span = span(&endpoint, &copy);
    measured(
        || span.dominance(&probe),
        |verdict| {
            assert_eq!(
                verdict,
                dominance(&endpoint, &endpoint, &probe),
                "the documented dominance"
            );
        },
    )
}

/// Measures `x & y`.
pub fn meet(senders: usize, packed: u64) -> u64 {
    let [x, y] = History::new(senders).snapshots(packed);
    measured(
        || &x & &y,
        |meet| assert!(meet <= x && meet <= y, "the meet lies below both operands"),
    )
}

/// Measures `x | y`.
pub fn join(senders: usize, packed: u64) -> u64 {
    let [x, y] = History::new(senders).snapshots(packed);
    measured(
        || &x | &y,
        |join| assert!(x <= join && y <= join, "the join lies above both operands"),
    )
}

/// Measures `x.span(y)`.
pub fn pair_span(senders: usize, packed: u64) -> u64 {
    let [x, y] = History::new(senders).snapshots(packed);
    let (meet, join) = (&x & &y, &x | &y);
    measured(
        || x.span(&y),
        |span| {
            assert_eq!(span.lo(), &meet, "the hull's start is the meet");
            assert_eq!(span.hi(), &join, "the hull's end is the join");
        },
    )
}

/// Measures `x.span_all([y])`.
pub fn pair_span_all(senders: usize, packed: u64) -> u64 {
    let [x, y] = History::new(senders).snapshots(packed);
    let (meet, join) = (&x & &y, &x | &y);
    measured(
        || x.span_all([&y]),
        |span| {
            assert_eq!(span.lo(), &meet, "the hull's start is the meet");
            assert_eq!(span.hi(), &join, "the hull's end is the join");
        },
    )
}

/// Measures `start.span_all(population)`.
pub fn population_span_all(senders: usize) -> u64 {
    let history = History::new(senders);
    let (start, population) = history.population();
    let (meet, join) = (start.meet_all(population), start.join_all(population));
    measured(
        || start.span_all(population),
        |span| {
            assert_eq!(
                span.lo(),
                &meet,
                "the hull's start is the meet of every input"
            );
            assert_eq!(
                span.hi(),
                &join,
                "the hull's end is the join of every input"
            );
        },
    )
}

/// Measures `start.meet_all(population)`.
pub fn population_meet_all(senders: usize) -> u64 {
    let history = History::new(senders);
    let (start, population) = history.population();
    measured(
        || start.meet_all(population),
        |meet| {
            assert!(
                population.iter().all(|member| meet <= *member) && meet <= *start,
                "the meet lies below every input"
            );
        },
    )
}

/// Measures `start.join_all(population)`.
pub fn population_join_all(senders: usize) -> u64 {
    let history = History::new(senders);
    let (start, population) = history.population();
    measured(
        || start.join_all(population),
        |join| {
            assert!(
                population.iter().all(|member| *member <= join) && *start <= join,
                "the join lies above every input"
            );
        },
    )
}

/// Builds the span `[lo, hi]` that parameter `packed` names, its wire bytes,
/// and the byte where its second component begins.
fn span_wire(senders: usize, packed: u64) -> (Version, Version, Vec<u8>, usize) {
    let [lo, hi] = History::new(senders).snapshots(packed);
    let bytes = span(&lo, &hi).encode();
    let boundary = lo.encode().len();
    (lo, hi, bytes, boundary)
}

/// Measures `Span::decode` of the span's wire bytes.
pub fn span_decode(senders: usize, packed: u64) -> u64 {
    let (lo, hi, bytes, _) = span_wire(senders, packed);
    measured(
        || Span::decode(&bytes[..]),
        |decoded| {
            let decoded = decoded.expect("a canonical span decodes");
            assert!(
                *decoded.lo() == lo && *decoded.hi() == hi,
                "the decoded endpoints"
            );
        },
    )
}

/// Measures `Version::decode` of the span's first component.
pub fn span_decode_lower(senders: usize, packed: u64) -> u64 {
    let (lo, _, bytes, boundary) = span_wire(senders, packed);
    measured(
        || Version::decode(&bytes[..boundary]),
        |decoded| assert_eq!(decoded.ok(), Some(lo), "the first component decodes"),
    )
}

/// Measures `Version::decode` of the span's second component.
pub fn span_decode_upper(senders: usize, packed: u64) -> u64 {
    let (_, hi, bytes, boundary) = span_wire(senders, packed);
    measured(
        || Version::decode(&bytes[boundary..]),
        |decoded| assert_eq!(decoded.ok(), Some(hi), "the second component decodes"),
    )
}
