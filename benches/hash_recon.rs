//! Worst-case hash reconstruction benchmark.
//!
//! Reconstructs the root hash of a tree containing a single non-zero leaf at
//! depth 32. The leaf's "hash" is the sentinel `[0xff; 32]`; the root hash is
//! then 32 iterated branch-wrap hashes, each BLAKE3 over an 8192-byte buffer
//! (256 slots * 32 bytes) with exactly one slot filled. This is the cost a
//! "cache only the topmost hash" strategy would pay to recompute a
//! maximally-compressed path from scratch — the depth bound (32) makes it the
//! worst case.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

const DEPTH: usize = 32;
const LEAF_SENTINEL: [u8; 32] = [0xff; 32];

/// One full reconstruction: 32 iterated branch-wrap hashes from the leaf
/// sentinel up to the root.
fn reconstruct(indices: &[u8; DEPTH]) -> [u8; 32] {
    let mut cur = LEAF_SENTINEL;
    for &index in indices {
        let mut buf = [0u8; 256 * 32];
        buf[index as usize * 32..][..32].copy_from_slice(&cur);
        cur = *blake3::hash(&buf).as_bytes();
    }
    cur
}

fn bench(c: &mut Criterion) {
    // A fixed path; the slot indices don't affect timing materially.
    let indices: [u8; DEPTH] = std::array::from_fn(|i| (i as u8).wrapping_mul(7).wrapping_add(3));

    c.bench_function("reconstruct_root_from_single_leaf_depth32", |b| {
        b.iter(|| reconstruct(black_box(&indices)))
    });

    // Single-level wrap, for reference: one 8 KiB BLAKE3 hash.
    c.bench_function("single_level_wrap_8kib", |b| {
        let mut buf = [0u8; 256 * 32];
        buf[3 * 32..][..32].copy_from_slice(&LEAF_SENTINEL);
        b.iter(|| blake3::hash(black_box(&buf)))
    });

    // Cheap single-child commitment: domain tag (1) + slot byte (1) + child
    // hash (32) = 34 bytes, one BLAKE3 block.
    c.bench_function("single_child_commit_34b", |b| {
        let mut buf = [0u8; 34];
        buf[0] = 0x01; // domain tag: compressed/single-child level
        buf[1] = 3; // slot byte
        buf[2..].copy_from_slice(&LEAF_SENTINEL);
        b.iter(|| blake3::hash(black_box(&buf)))
    });

    // Worst-case reconstruction under the cheap convention: 32 iterated
    // 34-byte commitments instead of 32 iterated 8 KiB hashes.
    c.bench_function("reconstruct_root_cheap_convention_depth32", |b| {
        b.iter(|| {
            let mut cur = LEAF_SENTINEL;
            for &index in black_box(&indices) {
                let mut buf = [0u8; 34];
                buf[0] = 0x01;
                buf[1] = index;
                buf[2..].copy_from_slice(&cur);
                cur = *blake3::hash(&buf).as_bytes();
            }
            cur
        })
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
