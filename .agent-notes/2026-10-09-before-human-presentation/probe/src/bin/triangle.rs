//! Counting bricks of |a − b| breaks the triangle inequality.
#[path = "../main.rs"]
#[allow(dead_code)]
mod probe;
use before::{Party, Version};

fn main() {
    let d = 5u32;
    // `forks` splits off 2^d shares and the seed keeps a residual share; every
    // share but the first ticks once, leaving a dyadic hole of depth d.
    let mut seed = Party::seed();
    let shares: Vec<Party> = seed.forks(1u64 << d).collect();
    let mut c = Version::new();
    for p in &shares[1..] {
        c.tick(p);
    }
    let mut hole = Version::new();
    hole.tick(&shares[0]);
    hole.tick(&seed);
    let mut flat = Version::new();
    flat.tick(&Party::seed());
    let a = Version::new();
    assert!(c <= flat && hole <= flat);
    assert_eq!(&c | &hole, flat, "c and the hole tile the flat version");
    assert_eq!(&c & &hole, a, "c and the hole are disjoint");
    println!("c    = {}", probe::show(&c));
    println!("hole = {}", probe::show(&hole));
    // |a - c| = c, |a - flat| = flat, |flat - c| = hole, so brick counts of the
    // absolute differences are min_ticks of these three versions.
    println!("bricks|a - c| = {}, bricks|a - flat| = {}, bricks|flat - c| = {}",
        c.min_ticks(), flat.min_ticks(), hole.min_ticks());
}
