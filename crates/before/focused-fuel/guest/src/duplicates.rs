//! Probes of the n-ary folds' duplicate filter.
//!
//! Every n-ary join, meet, hull, union, and intersection fold drops an input
//! that shares its predecessor's buffer before its balanced counter reads it.
//! Each probe folds `x` and then `copies` clones of `d`, a version concurrent
//! with `x`, all sharing one buffer, and checks that the result equals the
//! fold of `x` with one copy.

use std::fmt::Debug;
use std::iter::{once, repeat_n};

use before::{Party, Span, Version};
use focused_fuel_protocol::Probe;

use crate::measured;

/// Builds the duplicate fixture over `N` shares of the seed: a shared
/// history, and `x` and `d`, each one tick on a different share past it.
///
/// The shares tick different counts, so the history keeps a stored width that
/// grows with the share count rather than normalizing to one uniform leaf.
fn fixture<const N: usize>() -> (Version, Version, Version) {
    let shares: [Party; N] = Party::seed().into();
    let mut history = Version::new();
    for (count, share) in (1u64..).zip(&shares) {
        share.ticks(&mut history, count);
    }
    let one_more = |share: &Party| {
        let mut version = history.clone();
        share.tick(&mut version);
        version
    };
    let (x, d) = (one_more(&shares[0]), one_more(&shares[1]));
    assert!(x.concurrent(&d), "folding x with d must run the kernel");
    (history, x, d)
}

/// Builds the duplicate fixture over `shares` shares.
///
/// # Panics
///
/// Panics unless `shares` is 16 or 64, the two widths the claims compare.
fn fixture_of(shares: usize) -> (Version, Version, Version) {
    match shares {
        16 => fixture::<16>(),
        64 => fixture::<64>(),
        _ => panic!("the duplicate fixture has 16 or 64 shares, not {shares}"),
    }
}

/// Measures `fold` over `x` and `copies` copies of `d`, checking the result
/// against the fold over one copy.
fn versions<R: PartialEq + Debug>(
    shares: usize,
    copies: usize,
    fold: impl Fn(&Version, &Version, usize) -> R,
) -> u64 {
    let (_, x, d) = fixture_of(shares);
    let once = fold(&x, &d, 1);
    measured(
        || fold(&x, &d, copies),
        |run| assert_eq!(run, once, "a run of copies folds like one copy"),
    )
}

/// Measures `fold` over the span from the history to `x` and `copies`
/// copies of the span from the history to `d`, checking the result against
/// the fold over one copy.
fn spans<R: PartialEq + Debug>(
    shares: usize,
    copies: usize,
    fold: impl for<'a> Fn(&Span<'a>, &Span<'a>, usize) -> R,
) -> u64 {
    let (history, x, d) = fixture_of(shares);
    let to_x = Span::new(&history, &x).expect("x extends the history");
    let to_d = Span::new(&history, &d).expect("d extends the history");
    let once = fold(&to_x, &to_d, 1);
    measured(
        || fold(&to_x, &to_d, copies),
        |run| assert_eq!(run, once, "a run of copies folds like one copy"),
    )
}

/// The result type every span sum, collection, and product folds into.
type SpanFold = Option<Span<'static>>;

/// Measures the duplicate probe `probe` over `shares` shares and `copies`
/// copies.
///
/// # Panics
///
/// Panics if `probe` is not a duplicate probe.
pub fn measure(probe: Probe, shares: usize, copies: usize) -> u64 {
    match probe {
        Probe::DuplicateVersionJoinAll => {
            versions(shares, copies, |x, d, k| x.join_all(repeat_n(d, k)))
        }
        Probe::DuplicateVersionMeetAll => {
            versions(shares, copies, |x, d, k| x.meet_all(repeat_n(d, k)))
        }
        Probe::DuplicateVersionSpanAll => versions(shares, copies, |x, d, k| {
            x.span_all(repeat_n(d, k)).into_owned()
        }),
        Probe::DuplicateSumBorrowed => versions(shares, copies, |x, d, k| {
            once(x).chain(repeat_n(d, k)).sum::<Version>()
        }),
        Probe::DuplicateSumOwned => versions(shares, copies, |x, d, k| {
            once(x.clone())
                .chain(repeat_n(d.clone(), k))
                .sum::<Version>()
        }),
        Probe::DuplicateCollectOwned => versions(shares, copies, |x, d, k| {
            once(x.clone())
                .chain(repeat_n(d.clone(), k))
                .collect::<Version>()
        }),
        Probe::DuplicateCollectBorrowed => versions(shares, copies, |x, d, k| {
            once(x).chain(repeat_n(d, k)).collect::<Version>()
        }),
        Probe::DuplicateSpanUnionAll => {
            spans(shares, copies, |x, d, k| x.union_all(repeat_n(d, k)))
        }
        Probe::DuplicateSpanIntersectAll => spans(shares, copies, |x, d, k| {
            x.intersect_all(repeat_n(d, k)).map(Span::into_owned)
        }),
        Probe::DuplicateSpanJoinAll => spans(shares, copies, |x, d, k| {
            x.join_all(repeat_n(d, k)).into_owned()
        }),
        Probe::DuplicateSpanMeetAll => spans(shares, copies, |x, d, k| {
            x.meet_all(repeat_n(d, k)).into_owned()
        }),
        Probe::DuplicatePointJoinAll => versions(shares, copies, |x, d, k| {
            Span::at(x).join_all(repeat_n(d, k)).into_owned()
        }),
        Probe::DuplicateOptionSpanSumBorrowedSpans => spans(shares, copies, |x, d, k| {
            once(x).chain(repeat_n(d, k)).sum::<SpanFold>()
        }),
        Probe::DuplicateOptionSpanSumOwnedSpans => spans(shares, copies, |x, d, k| {
            once(x.clone())
                .chain(repeat_n(d.clone(), k))
                .sum::<SpanFold>()
        }),
        Probe::DuplicateOptionSpanSumBorrowedVersions => versions(shares, copies, |x, d, k| {
            once(x).chain(repeat_n(d, k)).sum::<SpanFold>()
        }),
        Probe::DuplicateOptionSpanSumOwnedVersions => versions(shares, copies, |x, d, k| {
            once(x.clone())
                .chain(repeat_n(d.clone(), k))
                .sum::<SpanFold>()
        }),
        Probe::DuplicateOptionSpanCollectBorrowedSpans => spans(shares, copies, |x, d, k| {
            once(x).chain(repeat_n(d, k)).collect::<SpanFold>()
        }),
        Probe::DuplicateOptionSpanCollectOwnedSpans => spans(shares, copies, |x, d, k| {
            once(x.clone())
                .chain(repeat_n(d.clone(), k))
                .collect::<SpanFold>()
        }),
        Probe::DuplicateOptionSpanCollectBorrowedVersions => versions(shares, copies, |x, d, k| {
            once(x).chain(repeat_n(d, k)).collect::<SpanFold>()
        }),
        Probe::DuplicateOptionSpanCollectOwnedVersions => versions(shares, copies, |x, d, k| {
            once(x.clone())
                .chain(repeat_n(d.clone(), k))
                .collect::<SpanFold>()
        }),
        Probe::DuplicateOptionSpanProductBorrowedSpans => spans(shares, copies, |x, d, k| {
            once(x).chain(repeat_n(d, k)).product::<SpanFold>()
        }),
        Probe::DuplicateOptionSpanProductOwnedSpans => spans(shares, copies, |x, d, k| {
            once(x.clone())
                .chain(repeat_n(d.clone(), k))
                .product::<SpanFold>()
        }),
        _ => panic!("{probe:?} is not a duplicate probe"),
    }
}
