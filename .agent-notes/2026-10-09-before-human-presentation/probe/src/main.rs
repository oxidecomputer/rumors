//! Probes how ticks move a version's global maximum height.
use before::shape::Rise;
use before::{Party, Version};

/// Maximal runs `(start numerator, end numerator, height)` over a common denominator 2^64.
pub fn runs(v: &Version) -> Vec<(u128, u128, u64)> {
    let mut out: Vec<(u128, u128, u64)> = Vec::new();
    let (mut pos, mut h) = (0u128, 0u64);
    for p in v.shape() {
        match &p.rise {
            Some(Rise::Up(c)) => h += u64::try_from(c).unwrap(),
            Some(Rise::Down(c)) => h -= u64::try_from(c).unwrap(),
            None => {}
        }
        let end = pos + (1u128 << (64 - p.depth));
        match out.last_mut() {
            Some(last) if last.2 == h => last.1 = end,
            _ => out.push((pos, end, h)),
        }
        pos = end;
    }
    out
}

/// Spells a position k / 2^64 as `0`, `1`, or a binary fraction.
pub fn pos(k: u128) -> String {
    if k == 0 { return "0".into() }
    if k == 1u128 << 64 { return "1".into() }
    let bits = format!("{:064b}", k);
    format!("0.{}", bits.trim_end_matches('0'))
}

pub fn show(v: &Version) -> String {
    let arms: Vec<String> = runs(v).iter().map(|&(s, e, h)| format!("{}..{} => {}", pos(s), pos(e), h)).collect();
    format!("Version {{ {} }}", arms.join(", "))
}

fn max(v: &Version) -> u64 { runs(v).iter().map(|r| r.2).max().unwrap() }

fn main() {
    // A keeps one half; B and C split the other half into quarters.
    let mut a = Party::seed();
    let mut b = a.fork();
    let c = b.fork();
    let mut v = Version::new();
    v.ticks(&b, 5u8);
    v.ticks(&c, 10u8);
    println!("start          {}  max {}", show(&v), max(&v));
    for i in 1..=4 {
        let before = max(&v);
        v.tick(&a);
        println!("A tick {i}       {}  max {} (+{})", show(&v), max(&v), max(&v) - before);
    }
    // The seed owns everything: its first tick fills, then each grows.
    let seed = Party::seed();
    let mut w = Version::new();
    w.ticks(&b, 5u8);
    let before = max(&w);
    w.ticks(&seed, 3u8);
    println!("seed ticks(3)  {}  max {} (+{})", show(&w), max(&w), max(&w) - before);
    println!("start version: min_ticks {} >= max {}", { let mut s = Version::new(); s.ticks(&b, 5u8); s.ticks(&c, 10u8); s.min_ticks() }, 14);
}
