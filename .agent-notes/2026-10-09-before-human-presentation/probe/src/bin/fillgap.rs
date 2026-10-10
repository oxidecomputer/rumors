//! One missing tick can leave a lag-height of 10.
#[path = "../main.rs"]
#[allow(dead_code)]
mod probe;
use before::{Party, Version};

fn main() {
    let mut p = Party::seed();
    let _q = p.fork(); // p keeps a half
    let p2 = p.fork(); // p and p2 split that half into quarters
    let mut a = Version::new();
    a.ticks(&p2, 10u8);
    p.join(p2).unwrap(); // p owns its whole half again
    let mut b = a.clone();
    b.tick(&p); // one tick a has not seen
    println!("a = {}", probe::show(&a));
    println!("b = {}", probe::show(&b));
}
