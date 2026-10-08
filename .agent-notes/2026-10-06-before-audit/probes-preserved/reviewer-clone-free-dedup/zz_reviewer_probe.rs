//! Reviewer probe (scratch, never committed): deterministic scan and heap
//! readings for families that exercise the fold duplicate filter, the
//! adapter's one-item lookahead, and the span folds' merged-with-merged arm.
//!
//! Output lines: `family<TAB>param<TAB>scan_bits<TAB>peak_heap<TAB>retained_heap`.

use before::testing::meter::{reset_scan_bits, scan_bits};
use before::{Party, Span, Version};
use peak_alloc::PeakAlloc;

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

/// Run `f`, returning (scan bits, peak heap above entry, heap retained above entry
/// while the result is still alive).
fn measure<T>(f: impl FnOnce() -> T) -> (u64, usize, usize) {
    reset_scan_bits();
    HEAP.reset_peak_usage();
    let base = HEAP.current_usage();
    let out = f();
    let peak = HEAP.peak_usage().saturating_sub(base);
    let retained = HEAP.current_usage().saturating_sub(base);
    let scan = scan_bits();
    drop(out);
    (scan, peak, retained)
}

fn report<T>(family: &str, param: impl std::fmt::Display, f: impl FnOnce() -> T) {
    let (s, p, r) = measure(f);
    println!("{family}\t{param}\t{s}\t{p}\t{r}");
}

/// `n` disjoint parties (n a power of two) from one seed.
fn parties(n: usize) -> Vec<Party> {
    let mut ps = vec![Party::seed()];
    while ps.len() < n {
        let mut next = Vec::with_capacity(ps.len() * 2);
        for mut p in ps {
            let q = p.fork();
            next.push(p);
            next.push(q);
        }
        ps = next;
    }
    ps
}

/// A wide shared history (one tick per party) and `m` pairwise-concurrent
/// versions, each the history plus one more tick on a distinct party.
/// Every returned version is decoded into its own exactly-filled buffer.
fn concurrent(width: usize, m: usize) -> (Version, Vec<Version>) {
    let ps = parties(width.max(m).next_power_of_two());
    // Varied tick counts per party, so the history does not normalize to a
    // uniform leaf: its stored width grows with the party count.
    let base: Version = ps
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let mut v = Version::new();
            p.ticks(&mut v, (i % 13 + 1) as u64);
            v
        })
        .sum();
    let vs: Vec<Version> = (0..m)
        .map(|i| {
            let mut v = base.clone();
            ps[i].tick(&mut v);
            Version::decode(&v.encode()[..]).unwrap()
        })
        .collect();
    eprintln!("fixture width={width} m={m} base_bytes={} item_bytes={}", base.encode().len(),
        vs.first().map_or(0, |v: &Version| v.encode().len()));
    (Version::decode(&base.encode()[..]).unwrap(), vs)
}

fn main() {
    // Warm-up outside any measurement: initializes the scratch flags.
    {
        let (_, vs) = concurrent(4, 2);
        let _ = vs[0].join_all([&vs[1]]);
        let _ = Span::at(&vs[0]).join_all([&vs[1], &vs[0], &vs[1]]);
    }
    // A. A run of k shared-buffer duplicates `d` after a distinct receiver
    // `x`. With the filter, the fold sees [x, d]; without it, the binary
    // counter pairs (x, d) once and then meets the merged result against
    // a `d` at every further level, where no buffer is shared.
    let (base, vs) = concurrent(256, 2);
    let (x, d) = (&vs[0], &vs[1]);
    let sx = Span::new(&base, x).unwrap();
    let sd = Span::new(&base, d).unwrap();
    for k in [1usize, 2, 4, 16, 64, 256, 1024] {
        report("A-version_join_all", k, || x.join_all(std::iter::repeat(d).take(k)));
        report("A-version_meet_all", k, || x.meet_all(std::iter::repeat(d).take(k)));
        report("A-version_span_all", k, || x.span_all(std::iter::repeat(d).take(k)));
        report("A-sum_refs", k, || {
            std::iter::once(x).chain(std::iter::repeat(d).take(k)).sum::<Version>()
        });
        report("A-span_union_all", k, || sx.union_all(std::iter::repeat(&sd).take(k)));
        report("A-span_join_all_points", k, || {
            Span::at(x).join_all(std::iter::repeat(d).take(k))
        });
        report("A-span_intersect_all", k, || {
            sx.intersect_all(std::iter::repeat(&sd).take(k))
        });
    }
    // A-width: the same run at k = 64 across widths: the filter-free
    // excess should scale with width (a kernel per level), not stay constant.
    for w in [16usize, 64, 256, 1024] {
        let (_, vs) = concurrent(w, 2);
        let (x, d) = (&vs[0], &vs[1]);
        report("Aw-version_join_all_k64", w, || x.join_all(std::iter::repeat(d).take(64)));
    }

    // B. Pairs offset by one: [x0, d1, d1, d2, d2, ..., dm, dm]. With the
    // filter every duplicate vanishes; without it the counter pairs each
    // duplicate with a different neighbor, so each one costs a kernel.
    for m in [4usize, 16, 64, 128] {
        let (_, vs) = concurrent(256, m + 1);
        let items: Vec<&Version> = vs[1..].iter().flat_map(|v| [v, v]).collect();
        report("B-version_join_all", m, || vs[0].join_all(items.iter().copied()));
        report("B-version_meet_all", m, || vs[0].meet_all(items.iter().copied()));
    }

    // C. A lazily decoded owned source: receiver a, then b and c = a | b
    // decoded only when pulled. The adapter's lookahead holds c's buffer
    // while (a, b) combines; a pull-on-demand fold would not yet hold it.
    for w in [64usize, 256, 1024, 4096] {
        let (_, vs) = concurrent(w, 2);
        let (a, b) = (&vs[0], &vs[1]);
        let bytes = [b.encode(), (a | b).encode()];
        report("C-lazy_join_all", w, || {
            a.join_all(bytes.iter().map(|e| Version::decode(&e[..]).unwrap()))
        });
        let all = [a.encode(), b.encode(), (a | b).encode()];
        report("C-lazy_sum_owned", w, || {
            all.iter()
                .map(|e| Version::decode(&e[..]).unwrap())
                .sum::<Version>()
        });
    }

    // D. Span::join_all whose left group has an empty lower endpoint and
    // whose right group's lower endpoint the kernel emitted: the
    // merged-with-merged arm's join yields its owned right operand.
    for w in [64usize, 256, 1024, 4096] {
        let (_, vs) = concurrent(w, 4);
        let (a, b, x, d) = (&vs[0], &vs[1], &vs[2], &vs[3]);
        let xd = x | d;
        let s0 = Span::new(Version::new(), a).unwrap();
        let items = [
            Span::new(Version::new(), b).unwrap(),
            Span::new(x, &xd).unwrap(),
            Span::new(d, &xd).unwrap(),
        ];
        report("D-span_join_all_empty_lo", w, || s0.join_all(&items));
        report("D-span_intersect_all_empty_lo", w, || s0.intersect_all(&items));
    }

    // E. Two separately decoded empty versions do not share a buffer, so the
    // filter keeps both; their merged group is empty, and the join side of
    // the merged-with-merged arm then yields its owned right operand, a
    // kernel-emitted group.
    for w in [64usize, 256, 1024, 4096] {
        let (_, vs) = concurrent(w, 2);
        let (x, y) = (&vs[0], &vs[1]);
        let empty = Version::new().encode();
        let e1 = Version::decode(&empty[..]).unwrap();
        let e2 = Version::decode(&empty[..]).unwrap();
        report("E-version_join_all_empty_group", w, || e1.join_all([&e2, x, y]));
        report("E-version_span_all_empty_group", w, || e1.span_all([&e2, x, y]));
    }
}
