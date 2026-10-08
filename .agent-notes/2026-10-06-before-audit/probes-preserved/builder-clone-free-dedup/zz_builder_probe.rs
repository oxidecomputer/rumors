//! Builder probe (scratch, never committed): peak heap of folds over owned
//! versions decoded one at a time as the fold pulls them.
use before::{Party, Version};
use peak_alloc::PeakAlloc;

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

fn peak<T>(f: impl FnOnce() -> T) -> usize {
    HEAP.reset_peak_usage();
    let base = HEAP.current_usage();
    let out = f();
    let p = HEAP.peak_usage().saturating_sub(base);
    drop(out);
    p
}

/// `n` disjoint parties from one seed, by balanced forking.
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

/// A history over `width` parties with varied counts, and two concurrent
/// versions: the history plus one tick on party 0, and on party 1.
fn pair(width: usize) -> (Version, Version) {
    let ps = parties(width);
    let mut history = Version::new();
    for (i, p) in ps.iter().enumerate() {
        p.ticks(&mut history, (i % 13 + 1) as u64);
    }
    let one_more = |p: &Party| {
        let mut v = history.clone();
        p.tick(&mut v);
        Version::decode(&v.encode()[..]).unwrap()
    };
    (one_more(&ps[0]), one_more(&ps[1]))
}

fn main() {
    for width in [64usize, 256, 1024, 4096] {
        let (a, b) = pair(width);
        let all = [a.encode(), b.encode(), (&a | &b).encode()];
        let lazy_sum = peak(|| {
            all.iter()
                .map(|e| Version::decode(&e[..]).unwrap())
                .sum::<Version>()
        });
        let lazy_join_all = peak(|| {
            a.join_all(all[1..].iter().map(|e| Version::decode(&e[..]).unwrap()))
        });
        println!(
            "width={width} input_bytes={} lazy_sum_owned={lazy_sum} lazy_join_all={lazy_join_all}",
            all[2].len()
        );
    }
}
