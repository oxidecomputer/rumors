//! Primitive-cost probe for the emission machinery.
//!
//! Measures candidate output-side primitive shapes (per-bit pushes into a
//! general bit vector — the external `bitvec` baseline — per-leaf
//! heap-allocated code buffers, bit-addressed splices) against the
//! word-buffered equivalents the crate's own builder ships, on the
//! join/tick sweeps' workload shape (~5k leaves, 3-9 bit codes, 75k
//! output bits).
//!
//! Usage: cargo run -p before --profile bench --example emit_probe

use bitvec::prelude::*;
use std::hint::black_box;
use std::time::Instant;

type BaselineBits = BitVec<u8, Msb0>;

const LEAVES: usize = 5000;

/// Deterministic pseudo-random small gamma-shaped codes (1-9 bits).
fn codes() -> Vec<(u64, u32)> {
    let mut state = 0x1737_C10C_C0DEu64;
    (0..LEAVES)
        .map(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let len = 1 + (state >> 60) % 9;
            (state & ((1 << len) - 1), len as u32)
        })
        .collect()
}

fn bench(name: &str, iters: u32, mut f: impl FnMut() -> usize) {
    // Warmup + measure.
    let mut sink = 0usize;
    for _ in 0..3 {
        sink ^= f();
    }
    let start = Instant::now();
    for _ in 0..iters {
        sink ^= f();
    }
    let per = start.elapsed().as_nanos() as f64 / iters as f64;
    black_box(sink);
    println!(
        "{name:<44} {per:>12.0} ns/iter  ({:.2} ns/leaf)",
        per / LEAVES as f64
    );
}

/// A minimal word-buffered MSB-first bit writer: the shape a
/// `PackedBuilder` replacement would have.
struct WordWriter {
    words: Vec<u64>,
    /// Bits already committed to `words` (multiple of 64).
    staged: u64,
    /// Live bits in `staged`, < 64.
    staged_len: u32,
}

impl WordWriter {
    fn with_capacity(bits: usize) -> Self {
        WordWriter {
            words: Vec::with_capacity(bits / 64 + 1),
            staged: 0,
            staged_len: 0,
        }
    }
    #[inline]
    fn push_bits(&mut self, value: u64, len: u32) {
        debug_assert!(len <= 63, "codes wider than 63 bits take the spill path");
        let total = self.staged_len + len;
        if total >= 64 {
            // Commit one word MSB-first: staged prefix high, value head low.
            let spill = total - 64;
            let committed = if self.staged_len == 0 {
                value >> spill
            } else {
                (self.staged << (64 - self.staged_len)) | (value >> spill)
            };
            self.words.push(committed);
            self.staged = value & ((1u64 << spill) - 1);
            self.staged_len = spill;
        } else {
            self.staged = (self.staged << len) | value;
            self.staged_len = total;
        }
    }
    fn len_bits(&self) -> usize {
        self.words.len() * 64 + self.staged_len as usize
    }
}

fn main() {
    let codes = codes();
    let total_bits: usize = codes.iter().map(|(_, l)| 2 + *l as usize).sum();
    println!("leaves={LEAVES} total output bits≈{total_bits}");

    // 1. Per-bit bitvec push: the current PackedBuilder discipline
    //    (1 flag push + per-bit code pushes per leaf).
    bench("bitvec push per bit", 2000, || {
        let mut out: BaselineBits = BitVec::with_capacity(total_bits);
        for &(code, len) in &codes {
            out.push(true);
            for i in (0..len).rev() {
                out.push((code >> i) & 1 == 1);
            }
            out.push(false);
        }
        out.len()
    });

    // 2. Current per-leaf heap code + extend_from_bitslice splice: what
    //    gamma_code + SkylineBuilder::leaf actually do.
    bench("bitvec alloc code + splice per leaf", 2000, || {
        let mut out: BaselineBits = BitVec::with_capacity(total_bits);
        for &(code, len) in &codes {
            let mut c: BaselineBits = BaselineBits::new();
            for i in (0..len).rev() {
                c.push((code >> i) & 1 == 1);
            }
            out.push(true);
            out.extend_from_bitslice(&c);
            out.push(false);
        }
        out.len()
    });

    // 3. Word-buffered writer, code stays a (u64, len) value.
    bench("word writer, inline codes", 2000, || {
        let mut out = WordWriter::with_capacity(total_bits);
        for &(code, len) in &codes {
            out.push_bits(1, 1);
            out.push_bits(code, len);
            out.push_bits(0, 1);
        }
        out.len_bits()
    });

    // 4. Bulk copy comparison: one 37.5k-bit misaligned splice, bitvec vs
    //    word shifts (the tick grow-path verbatim copy).
    let src: BaselineBits = {
        let mut v = BaselineBits::new();
        let mut state = 7u64;
        for _ in 0..37_500 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            v.push(state >> 63 == 1);
        }
        v
    };
    bench(
        "bitvec extend_from_bitslice 37.5kbit misaligned",
        2000,
        || {
            let mut out: BaselineBits = BitVec::with_capacity(38_000);
            out.push(true); // force misalignment
            out.extend_from_bitslice(&src[3..]);
            out.len()
        },
    );
    let src_words: Vec<u64> = (0..600)
        .map(|i| (i as u64).wrapping_mul(0x9E3779B97F4A7C15))
        .collect();
    bench("word-shift copy 37.5kbit misaligned", 2000, || {
        let mut out = WordWriter::with_capacity(38_400);
        out.push_bits(1, 1);
        out.push_bits(src_words[0] & 7, 3);
        for &w in &src_words[1..] {
            out.push_bits(w >> 32, 32);
            out.push_bits(w & 0xFFFF_FFFF, 32);
        }
        out.len_bits()
    });
}
