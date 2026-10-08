//! Second stack-frame probe (reviewer scratch, never committed).
use std::cell::Cell;

thread_local! {
    static LEFT: Cell<usize> = const { Cell::new(0) };
}

/// A left spine walked with no value live across the recursive call: the
/// remaining depth lives in a thread-local, so the frame holds only the
/// return address and the saved frame pointer.
#[inline(never)]
fn bare() {
    let left = LEFT.with(Cell::get);
    if left > 0 {
        LEFT.with(|c| c.set(left - 1));
        bare();
        LEFT.with(|c| c.set(c.get() + 1));
    }
}

/// Source-level non-tail recursion in accumulator form, unobstructed.
#[inline(never)]
fn acc(n: u64) -> u64 {
    if n == 0 { 0 } else { 1 + acc(n - 1) }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let kind = args[1].clone();
    let depth: usize = args[2].parse().unwrap();
    let h = std::thread::Builder::new()
        .spawn(move || match kind.as_str() {
            "bare" => {
                LEFT.with(|c| c.set(depth));
                bare();
                println!("bare depth={depth} ok");
            }
            "acc" => println!("acc depth={depth} ok {}", acc(std::hint::black_box(depth as u64))),
            _ => unreachable!(),
        })
        .unwrap();
    h.join().unwrap();
}
