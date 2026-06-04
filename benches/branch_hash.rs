//! Scratch microbenchmark: how should `Hash::branch` feed Blake3?
//!
//! The branch preimage is `BRANCH_TAG ‖ (radix ‖ 32-byte child hash)*` — each
//! child is a 33-byte record that straddles Blake3's 64-byte block boundary.
//! The current implementation streams two `update` calls per child (the radix
//! byte alone, then the 32-byte hash). This bench asks whether assembling a
//! contiguous buffer first is faster, across realistic fan-outs.
//!
//! Strategies:
//!   - `stream2`: current — `update(&[radix]); update(&hash)` per child.
//!   - `stream1`: one `update` per child of a 33-byte stack record.
//!   - `buffer_oneshot`: fill a reused contiguous buffer, then `blake3::hash`.
//!
//! `buffer_oneshot` is the only one that hands Blake3 a single contiguous slice,
//! which is what lets its SIMD path compress multiple blocks at once. The
//! question is whether that beats the per-call overhead + buffer fill at the
//! fan-outs the tree actually produces (1 for compressed singletons, up to 256
//! for a saturated branch).

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

const BRANCH_TAG: u8 = 1;

/// Fan-outs to sweep: 1 is the path-compressed singleton (worst-case
/// reconstruction), 256 is a saturated branch, the rest fill in between.
const FANOUTS: &[usize] = &[1, 2, 4, 8, 16, 64, 256];

/// A deterministic set of `k` (radix, hash) children. Radixes need not be
/// distinct for a hashing microbench; the byte content only has to be fixed.
fn children(k: usize) -> Vec<(u8, [u8; 32])> {
    (0..k)
        .map(|i| {
            let r = (i as u8).wrapping_mul(7).wrapping_add(3);
            let h = std::array::from_fn(|j| (i as u8) ^ (j as u8).wrapping_mul(31));
            (r, h)
        })
        .collect()
}

/// Current implementation: two `update` calls per child.
fn stream2(children: &[(u8, [u8; 32])]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[BRANCH_TAG]);
    for (radix, hash) in children {
        hasher.update(&[*radix]);
        hasher.update(hash);
    }
    *hasher.finalize().as_bytes()
}

/// One `update` per child of a 33-byte stack record.
fn stream1(children: &[(u8, [u8; 32])]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[BRANCH_TAG]);
    let mut record = [0u8; 33];
    for (radix, hash) in children {
        record[0] = *radix;
        record[1..].copy_from_slice(hash);
        hasher.update(&record);
    }
    *hasher.finalize().as_bytes()
}

/// Assemble a contiguous buffer, then a single one-shot hash. `buf` is reused
/// across calls so the cost measured is the fill + hash, not allocation.
fn buffer_oneshot(children: &[(u8, [u8; 32])], buf: &mut Vec<u8>) -> [u8; 32] {
    buf.clear();
    buf.push(BRANCH_TAG);
    for (radix, hash) in children {
        buf.push(*radix);
        buf.extend_from_slice(hash);
    }
    *blake3::hash(buf).as_bytes()
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("branch_hash");
    for &k in FANOUTS {
        let kids = children(k);
        group.bench_with_input(BenchmarkId::new("stream2", k), &kids, |b, kids| {
            b.iter(|| stream2(black_box(kids)))
        });
        group.bench_with_input(BenchmarkId::new("stream1", k), &kids, |b, kids| {
            b.iter(|| stream1(black_box(kids)))
        });
        group.bench_with_input(BenchmarkId::new("buffer_oneshot", k), &kids, |b, kids| {
            let mut buf = Vec::with_capacity(1 + 33 * k);
            b.iter(|| buffer_oneshot(black_box(kids), &mut buf))
        });
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
