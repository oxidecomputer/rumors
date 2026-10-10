//! Measures how the readable notation's length grows against encoded size.
//!
//! Positions and heights print in absolute form while the encoding stores
//! depth and height deltas, so adversarial versions can make the text
//! superlinear in the encoding.
#[path = "../main.rs"]
#[allow(dead_code)]
mod probe;
use before::shape::Rise;
use before::{Count, Party, Version};

/// A version whose plateau boundaries follow an alternating left/right spine,
/// so boundary positions look like 0.0101…, which no run-length form shortens.
///
/// Heights decrease with depth so that no tick fills a leaf from its
/// sibling: a shallow leaf's sibling holds only lower heights.
fn alternating_spine(levels: usize) -> Version {
    let mut current = Party::seed();
    let mut leaves = Vec::new();
    for level in 0..levels {
        let other = current.fork();
        let (leaf, next) = if level % 2 == 0 { (other, current) } else { (current, other) };
        leaves.push(leaf);
        current = next;
    }
    let mut v = Version::new();
    for (level, leaf) in leaves.iter().enumerate() {
        v.ticks(leaf, (levels - level) as u64);
    }
    v
}

fn main() {
    println!("alternating spine: levels, encoded bytes, notation chars");
    for levels in [8, 16, 32, 60] {
        let v = alternating_spine(levels);
        let text = probe::show(&v);
        println!("  {levels:>3} {:>6} {:>7}", v.as_bytes().len(), text.len());
    }
    let v = alternating_spine(8);
    println!("  e.g. {}", probe::show(&v));

    println!("wide base, narrow oscillations: base digits, runs, encoded bytes, digits printed for heights");
    for digits in [20usize, 80, 320, 1280] {
        let mut seed = Party::seed();
        let shares: Vec<Party> = seed.forks(64u64).collect();
        let mut v = Version::new();
        let wide: Count = format!("1{}", "0".repeat(digits - 1)).parse().unwrap();
        v.ticks(&Party::seed(), wide);
        for p in shares.iter().step_by(2) {
            v.tick(p);
        }
        let runs = v.shape().filter(|p| p.rise.is_some()).count();
        println!("  {digits:>5} {runs:>5} {:>6} {:>8}", v.as_bytes().len(), runs * digits);
    }
    let _ = Rise::Up(Count::from(1u8));
}
