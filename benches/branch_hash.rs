//! Feeding-strategy microbenchmark behind `Hash::branch`'s one-shot form.
//!
//! A branch preimage is `kind ‖ prefix_len ‖ prefix ‖ count ‖ (radix ‖ hash)*`
//! — dominated by fixed-width child records, several to one of SHA3-256's
//! 136-byte rate blocks. The shipped `Hash::branch` assembles the whole
//! preimage in a contiguous buffer and hashes it in one shot; the
//! alternative feeds the sponge one `update` per field. This bench measures
//! exactly that comparison over the current preimage layout, across fan-outs
//! from the smallest representable branch to the saturated 256, so the
//! claim at `Hash::branch` stays re-measurable: run
//! `just bench branch_hash` and compare the `contiguous` and `streamed`
//! curves.
//!
//! The layout is restated locally because the tree's hashing internals are
//! not public API; it mirrors the preimage documented at `Hash::branch`,
//! which the hash tests pin byte-for-byte. `contiguous` reproduces the
//! shipped form including its per-call buffer allocation, so the measured
//! difference is the end-to-end cost a caller sees, not the hash core alone.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use sha3::{Digest, Sha3_256};

/// Kind byte leading a branch preimage, mirrored from the documented layout.
const BRANCH_TAG: u8 = 1;

/// Width of a truncated child hash inside a branch preimage.
const HASH_LEN: usize = rumors::MERKLE_HASH_LEN;

/// Bytes one child contributes to a branch preimage: its radix byte followed
/// by its hash.
const CHILD_RECORD_LEN: usize = 1 + HASH_LEN;

/// A hot node's compressed span: short, as path compression typically leaves
/// interior branches near the root.
const PREFIX: &[u8] = &[0xa5, 0x5a, 0x3c];

/// Fan-outs to sweep: 2 is the smallest representable branch under the
/// canonical-shape invariant, 256 a saturated one, the rest fill in between.
const FANOUTS: &[usize] = &[2, 4, 16, 64, 256];

/// A deterministic set of `k` (radix, hash) children in strictly ascending
/// radix order, as the convention requires. Only the byte content matters to
/// a hashing microbench, and only that it is fixed across runs.
fn children(k: usize) -> Vec<(u8, [u8; HASH_LEN])> {
    assert!(k <= 256, "branch fan-out is bounded by the 256-way radix");
    (0..k)
        .map(|i| {
            let radix = u8::try_from(i * 256 / k.max(1)).expect("index scaled into radix range");
            let hash = std::array::from_fn(|j| (i as u8) ^ (j as u8).wrapping_mul(31));
            (radix, hash)
        })
        .collect()
}

/// The shipped form: assemble the whole preimage contiguously (fresh buffer,
/// count backfilled after the records), then hash it in one shot.
fn contiguous(prefix: &[u8], children: &[(u8, [u8; HASH_LEN])]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(4 + prefix.len() + CHILD_RECORD_LEN * children.len());
    buf.push(BRANCH_TAG);
    buf.push(u8::try_from(prefix.len()).expect("a compressed span fits in one length byte"));
    buf.extend_from_slice(prefix);
    let count_at = buf.len();
    buf.extend_from_slice(&[0, 0]);
    for (radix, hash) in children {
        buf.push(*radix);
        buf.extend_from_slice(hash);
    }
    let count = u16::try_from(children.len()).expect("fan-out fits u16");
    buf[count_at..count_at + 2].copy_from_slice(&count.to_be_bytes());
    Sha3_256::digest(&buf).into()
}

/// The streamed form: one `update` call per field, so the sponge sees the
/// preimage in radix-byte and hash-width fragments.
fn streamed(prefix: &[u8], children: &[(u8, [u8; HASH_LEN])]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update([BRANCH_TAG]);
    hasher.update([u8::try_from(prefix.len()).expect("a compressed span fits in one length byte")]);
    hasher.update(prefix);
    let count = u16::try_from(children.len()).expect("fan-out fits u16");
    hasher.update(count.to_be_bytes());
    for (radix, hash) in children {
        hasher.update([*radix]);
        hasher.update(hash);
    }
    hasher.finalize().into()
}

fn branch_hash(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("branch_hash");
    for &fanout in FANOUTS {
        let kids = children(fanout);
        group.bench_with_input(BenchmarkId::new("contiguous", fanout), &kids, |b, kids| {
            b.iter(|| contiguous(black_box(PREFIX), black_box(kids)));
        });
        group.bench_with_input(BenchmarkId::new("streamed", fanout), &kids, |b, kids| {
            b.iter(|| streamed(black_box(PREFIX), black_box(kids)));
        });
    }
    group.finish();
}

criterion_group!(benches, branch_hash);
criterion_main!(benches);
