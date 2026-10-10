//! Checks min_ticks against the floors' dyadic-piece count, and its lattice behavior.
use before::shape::Rise;
use before::{Party, Version};

/// Canonical plateaus `(start, end, height)` over 2^64 (panics past depth 64).
fn plateaus(v: &Version) -> Vec<(u128, u128, u64)> {
    let (mut out, mut pos, mut h) = (Vec::new(), 0u128, 0u64);
    for p in v.shape() {
        match &p.rise {
            Some(Rise::Up(c)) => h += u64::try_from(c).unwrap(),
            Some(Rise::Down(c)) => h -= u64::try_from(c).unwrap(),
            None => {}
        }
        assert!(p.depth <= 64);
        let end = pos + (1u128 << (64 - p.depth));
        out.push((pos, end, h));
        pos = end;
    }
    out
}
fn max(v: &Version) -> u64 { plateaus(v).iter().map(|r| r.2).max().unwrap() }
fn area(v: &Version) -> u128 { plateaus(v).iter().map(|r| (r.1 - r.0) * r.2 as u128).sum() }
fn width(v: &Version) -> u128 { plateaus(v).iter().filter(|r| r.2 > 0).map(|r| r.1 - r.0).sum() }
fn mt(v: &Version) -> u64 { u64::try_from(&v.min_ticks()).unwrap() }

/// Maximal dyadic pieces of one interval: the range-to-prefix greedy.
fn pieces(mut s: u128, e: u128) -> u64 {
    let mut n = 0;
    while s < e {
        let align = if s == 0 { 1u128 << 64 } else { 1u128 << s.trailing_zeros() };
        let mut size = align;
        while s + size > e { size >>= 1; }
        s += size;
        n += 1;
    }
    n
}
/// Σ over floors {v ≥ k} of their maximal dyadic piece counts.
fn bricks(v: &Version) -> u64 {
    let ps = plateaus(v);
    let mut total = 0;
    for k in 1..=max(v) {
        let mut run: Option<(u128, u128)> = None;
        for &(s, e, h) in &ps {
            if h >= k {
                run = Some(match run { Some((rs, re)) if re == s => (rs, e), Some((rs, re)) => { total += pieces(rs, re); (s, e) } None => (s, e) });
            }
        }
        if let Some((rs, re)) = run { total += pieces(rs, re); }
    }
    total
}

struct Lcg(u64);
impl Lcg { fn next(&mut self, n: usize) -> usize { self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); ((self.0 >> 33) as usize) % n } }

fn main() {
    let mut rng = Lcg(0xb41c);
    let (mut versions, mut pairs, mut sub_strict, mut super_viol, mut nonmono, mut le_pairs) = (0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    for _ in 0..300 {
        let mut parties = vec![Party::seed()];
        for _ in 0..rng.next(8) { let i = rng.next(parties.len()); let f = parties[i].fork(); parties.push(f); }
        let mut pool = vec![Version::new()];
        for _ in 0..50 {
            let mut v = pool[rng.next(pool.len())].clone();
            if rng.next(3) == 0 { v = &v | &pool[rng.next(pool.len())]; } else { v.tick(&parties[rng.next(parties.len())]); }
            assert_eq!(bricks(&v), mt(&v), "min_ticks counts the floors' dyadic pieces");
            versions += 1;
            pool.push(v);
        }
        for _ in 0..200 {
            let (a, b) = (&pool[rng.next(pool.len())], &pool[rng.next(pool.len())]);
            let (j, m) = (a | b, a & b);
            assert_eq!(area(&j) + area(&m), area(a) + area(b), "rank is modular");
            assert_eq!(width(&j) + width(&m), width(a) + width(b), "width is modular");
            let (lhs, rhs) = (mt(&j) + mt(&m), mt(a) + mt(b));
            if lhs < rhs { sub_strict += 1 } else if lhs > rhs { super_viol += 1 }
            if a <= b { le_pairs += 1; if mt(a) > mt(b) { nonmono += 1 } }
            pairs += 1;
        }
    }
    println!("{versions} versions: min_ticks == floor brick count for all");
    println!("{pairs} pairs: rank and width modular for all; min_ticks(a|b)+min_ticks(a&b) < sum in {sub_strict}, > sum in {super_viol}");
    println!("{le_pairs} ordered pairs a <= b: min_ticks(a) > min_ticks(b) in {nonmono}");
    // The hand-built non-monotone case: two separate bumps lie below one flat tick.
    let mut p = Party::seed();
    let q = p.fork();
    let _p2 = p.fork();
    let mut v = Version::new();
    v.tick(&p);
    v.tick(&q);
    let mut w = Version::new();
    w.tick(&Party::seed());
    println!("v <= w: {}, min_ticks v = {}, w = {}", v <= w, mt(&v), mt(&w));
}
