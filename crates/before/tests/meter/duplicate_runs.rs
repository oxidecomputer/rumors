//! Scan checks for the n-ary folds' duplicate filter.
//!
//! Every n-ary join, meet, hull, union, and intersection fold drops an input
//! that shares its predecessor's buffer before the balanced counter reads it.
//! Without that filter the result is unchanged, but a run of copies costs
//! kernel work: the counter merges the run's first copy with its neighbor, then
//! meets that merged result against another copy at every further level, where
//! no buffer is shared. These checks hold the filter at every fold entry point.

use before::testing::meter;
use before::{Party, Span, Version};

/// Scan bits of one fold, on a fresh counter.
fn scanned<T>(fold: impl FnOnce() -> T) -> u64 {
    meter::reset_scan_bits();
    let result = fold();
    let bits = meter::scan_bits();
    drop(result);
    bits
}

/// Build a shared history and two concurrent versions over it, each one more
/// tick on a different share.
///
/// The shares tick different counts, so the history keeps a stored width that
/// grows with the share count rather than normalizing to one uniform leaf.
fn history_and_concurrent_pair() -> (Version, Version, Version) {
    let shares: [Party; 16] = Party::seed().into();
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

/// Each fold entry point scans a run of copies of one input exactly as it
/// scans that input once.
///
/// Every fold reads `x` and then `k` clones of `d`, a version concurrent with
/// `x`, all sharing one buffer. With the duplicate filter, the counter sees
/// `x` and one `d` at every `k`, so the scan equals the single-copy scan.
/// Without it, every further counter level runs the kernel again. The span
/// folds read spans from a shared history to `x` and to `d`, whose run shares
/// both endpoint buffers.
///
/// A floor first proves that the single-copy fold scans; at zero, every
/// comparison passes trivially.
#[test]
fn duplicate_runs_scan_like_one_copy() {
    let (history, x, d) = history_and_concurrent_pair();
    let to_x = Span::new(&history, &x).expect("x extends the history");
    let to_d = Span::new(&history, &d).expect("d extends the history");
    let copies = |k: usize| std::iter::repeat_n(&d, k);
    let owned_copies = |k: usize| std::iter::repeat_n(d.clone(), k);
    let span_copies = |k: usize| std::iter::repeat_n(&to_d, k);
    let owned_span_copies = |k: usize| std::iter::repeat_n(to_d.clone(), k);
    #[allow(clippy::type_complexity)]
    let folds: [(&str, &dyn Fn(usize) -> u64); 22] = [
        ("Version::join_all", &|k| scanned(|| x.join_all(copies(k)))),
        ("Version::meet_all", &|k| scanned(|| x.meet_all(copies(k)))),
        ("Version::span_all", &|k| scanned(|| x.span_all(copies(k)))),
        ("Sum<&Version>", &|k| {
            scanned(|| std::iter::once(&x).chain(copies(k)).sum::<Version>())
        }),
        ("Sum<Version>", &|k| {
            scanned(|| {
                std::iter::once(x.clone())
                    .chain(owned_copies(k))
                    .sum::<Version>()
            })
        }),
        ("FromIterator<Version>", &|k| {
            scanned(|| {
                std::iter::once(x.clone())
                    .chain(owned_copies(k))
                    .collect::<Version>()
            })
        }),
        ("Span::union_all", &|k| {
            scanned(|| to_x.union_all(span_copies(k)))
        }),
        ("Span::intersect_all", &|k| {
            scanned(|| to_x.intersect_all(span_copies(k)))
        }),
        ("Span::join_all", &|k| {
            scanned(|| to_x.join_all(span_copies(k)))
        }),
        ("Span::meet_all", &|k| {
            scanned(|| to_x.meet_all(span_copies(k)))
        }),
        ("Span::join_all of points", &|k| {
            scanned(|| Span::at(&x).join_all(copies(k)))
        }),
        ("FromIterator<&Version>", &|k| {
            scanned(|| std::iter::once(&x).chain(copies(k)).collect::<Version>())
        }),
        ("Option<Span>: Sum<&Span>", &|k| {
            scanned(|| {
                std::iter::once(&to_x)
                    .chain(span_copies(k))
                    .sum::<Option<Span<'static>>>()
            })
        }),
        ("Option<Span>: Sum<Span>", &|k| {
            scanned(|| {
                std::iter::once(to_x.clone())
                    .chain(owned_span_copies(k))
                    .sum::<Option<Span<'static>>>()
            })
        }),
        ("Option<Span>: Sum<&Version>", &|k| {
            scanned(|| {
                std::iter::once(&x)
                    .chain(copies(k))
                    .sum::<Option<Span<'static>>>()
            })
        }),
        ("Option<Span>: Sum<Version>", &|k| {
            scanned(|| {
                std::iter::once(x.clone())
                    .chain(owned_copies(k))
                    .sum::<Option<Span<'static>>>()
            })
        }),
        ("Option<Span>: FromIterator<&Span>", &|k| {
            scanned(|| {
                std::iter::once(&to_x)
                    .chain(span_copies(k))
                    .collect::<Option<Span<'static>>>()
            })
        }),
        ("Option<Span>: FromIterator<Span>", &|k| {
            scanned(|| {
                std::iter::once(to_x.clone())
                    .chain(owned_span_copies(k))
                    .collect::<Option<Span<'static>>>()
            })
        }),
        ("Option<Span>: FromIterator<&Version>", &|k| {
            scanned(|| {
                std::iter::once(&x)
                    .chain(copies(k))
                    .collect::<Option<Span<'static>>>()
            })
        }),
        ("Option<Span>: FromIterator<Version>", &|k| {
            scanned(|| {
                std::iter::once(x.clone())
                    .chain(owned_copies(k))
                    .collect::<Option<Span<'static>>>()
            })
        }),
        ("Option<Span>: Product<&Span>", &|k| {
            scanned(|| {
                std::iter::once(&to_x)
                    .chain(span_copies(k))
                    .product::<Option<Span<'static>>>()
            })
        }),
        ("Option<Span>: Product<Span>", &|k| {
            scanned(|| {
                std::iter::once(to_x.clone())
                    .chain(owned_span_copies(k))
                    .product::<Option<Span<'static>>>()
            })
        }),
    ];
    let mut violations = Vec::new();
    for (name, fold) in folds {
        let once = fold(1);
        assert!(
            once > 0,
            "{name}: folding x with one copy of d must scan; at zero, every \
             comparison below passes trivially"
        );
        for k in [2, 3, 8] {
            let run = fold(k);
            if run != once {
                violations.push(format!(
                    "{name}: {k} copies of d scanned {run} bits, one copy {once}"
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "a run of shared-buffer copies must fold like one copy; a larger \
         reading means the duplicate filter let a copy reach the counter:\n{}",
        violations.join("\n"),
    );
}
