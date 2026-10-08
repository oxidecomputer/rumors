//! Stack-frame probe for the deep-test depth argument (reviewer scratch, never committed).
use std::hint::black_box;

struct State<'a> {
    bits: &'a [u8],
    pos: usize,
}

impl State<'_> {
    #[inline(always)]
    fn next(&mut self) -> bool {
        let b = self.bits[self.pos] != 0;
        self.pos += 1;
        b
    }
}

/// Pre-order walk: a 1 is a branch with two children, a 0 a leaf. The first
/// call is non-tail (the state is live across it); the second is a tail call.
#[inline(never)]
fn walk(s: &mut State) {
    if s.next() {
        walk(s);
        walk(s);
    }
}

/// Source-level non-tail recursion in accumulator form.
#[inline(never)]
fn acc(n: u64) -> u64 {
    if n == 0 { 0 } else { 1 + acc(black_box(n) - 1) }
}

/// Non-tail recursion that records the stack address of a local at each level.
#[inline(never)]
fn addrs(n: usize, out: &mut Vec<usize>) {
    let local = 0u8;
    out.push(black_box(&local) as *const u8 as usize);
    if n > 0 {
        addrs(n - 1, out);
    }
    black_box(&local);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let kind = args[1].clone();
    let depth: usize = args[2].parse().unwrap();
    let h = std::thread::Builder::new()
        .spawn(move || match kind.as_str() {
            "walk" => {
                let mut bits = vec![1u8; depth];
                bits.extend(std::iter::repeat(0u8).take(depth + 1));
                let mut s = State { bits: &bits, pos: 0 };
                walk(&mut s);
                println!("walk depth={depth} ok pos={}", s.pos);
            }
            "acc" => println!("acc depth={depth} ok {}", acc(depth as u64)),
            "addrs" => {
                let mut v = Vec::with_capacity(depth + 1);
                addrs(depth, &mut v);
                println!("addrs frame bytes = {}", v[0] - v[1]);
            }
            _ => unreachable!(),
        })
        .unwrap();
    h.join().unwrap();
}
