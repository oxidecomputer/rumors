//! Randomized checks of the height and width claims over organic histories.
use before::shape::Rise;
use before::{Party, Version};

/// Runs `(start, end, height)` with positions over 2^64 (panics past depth 64).
fn runs(v: &Version) -> Vec<(u128, u128, u64)> {
    let mut out: Vec<(u128, u128, u64)> = Vec::new();
    let (mut pos, mut h) = (0u128, 0u64);
    for p in v.shape() {
        match &p.rise {
            Some(Rise::Up(c)) => h += u64::try_from(c).unwrap(),
            Some(Rise::Down(c)) => h -= u64::try_from(c).unwrap(),
            None => {}
        }
        assert!(p.depth <= 64, "probe limited to depth 64");
        let end = pos + (1u128 << (64 - p.depth));
        out.push((pos, end, h));
        pos = end;
    }
    out
}
fn max(v: &Version) -> u64 { runs(v).iter().map(|r| r.2).max().unwrap() }
fn min(v: &Version) -> u64 { runs(v).iter().map(|r| r.2).min().unwrap() }
fn width(v: &Version) -> u128 { runs(v).iter().filter(|r| r.2 > 0).map(|r| r.1 - r.0).sum() }
fn area(v: &Version) -> u128 { runs(v).iter().map(|r| (r.1 - r.0) * r.2 as u128).sum() }

struct Lcg(u64);
impl Lcg { fn next(&mut self, n: usize) -> usize { self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); ((self.0 >> 33) as usize) % n } }

fn main() {
    let mut rng = Lcg(0x5eed);
    let (mut checks, mut tick_zero, mut tick_one) = (0u64, 0u64, 0u64);
    for _trial in 0..400 {
        // Up to 8 disjoint parties by repeated forking of random parties.
        let mut parties = vec![Party::seed()];
        for _ in 0..rng.next(8) { let i = rng.next(parties.len()); let f = parties[i].fork(); parties.push(f); }
        let mut pool = vec![Version::new()];
        for _step in 0..60 {
            let i = rng.next(pool.len());
            if rng.next(3) == 0 && pool.len() > 1 {
                let j = rng.next(pool.len());
                let (a, b) = (pool[i].clone(), pool[j].clone());
                let (jn, mt) = (&a | &b, &a & &b);
                assert_eq!(max(&jn), max(&a).max(max(&b)), "max preserves joins");
                assert_eq!(min(&mt), min(&a).min(min(&b)), "min preserves meets");
                pool.push(jn);
            } else {
                let mut v = pool[i].clone();
                let before = max(&v);
                v.tick(&parties[rng.next(parties.len())]);
                match max(&v) - before { 0 => tick_zero += 1, 1 => tick_one += 1, d => panic!("tick raised max by {d}") }
                pool.push(v);
            }
            let v = pool.last().unwrap();
            let (w, a, m) = (width(v), area(v), max(v));
            assert!(w <= a && a <= m as u128 * w, "width <= rank <= max * width");
            assert!(u64::try_from(&v.min_ticks()).unwrap() >= m, "min_ticks >= max");
            checks += 1;
        }
    }
    println!("{checks} versions checked; ticks raising max by 0: {tick_zero}, by 1: {tick_one}");
}
