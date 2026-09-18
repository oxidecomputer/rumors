// `criterion_group!` generates a public registrar with no documentation.
#![allow(missing_docs)]

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
//! The contiguous series calls the production implementation through its
//! test-only entry point. The streamed alternative spells out the same input,
//! and an untimed assertion keeps the two forms equivalent.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rumors::testing::MERKLE_HASH_LEN;
use sha3::{Digest, Sha3_256};

/// Kind byte leading a branch preimage, mirrored from the documented layout.
const BRANCH_TAG: u8 = 1;

/// A hot node's compressed span: short, as path compression typically leaves
/// interior branches near the root.
const PREFIX: &[u8] = &[0xa5, 0x5a, 0x3c];

/// Fan-outs to sweep: 2 is the smallest representable branch under the
/// canonical-shape invariant, 256 a saturated one, the rest fill in between.
const FANOUTS: &[usize] = &[2, 4, 16, 64, 256];

/// A deterministic set of `k` (radix, hash) children in strictly ascending
/// radix order, as the convention requires. Only the byte content matters to
/// a hashing microbench, and only that it is fixed across runs.
fn children(k: usize) -> Vec<(u8, [u8; MERKLE_HASH_LEN])> {
    assert!(k <= 256, "branch fan-out is bounded by the 256-way radix");
    (0..k)
        .map(|i| {
            let radix = u8::try_from(i * 256 / k.max(1)).expect("index scaled into radix range");
            let hash = std::array::from_fn(|j| (i as u8) ^ (j as u8).wrapping_mul(31));
            (radix, hash)
        })
        .collect()
}

/// Hash a branch with the production contiguous preimage builder.
fn contiguous(prefix: &[u8], children: &[(u8, [u8; MERKLE_HASH_LEN])]) -> [u8; MERKLE_HASH_LEN] {
    rumors::testing::branch_hash(prefix, children.iter().copied())
}

/// The streamed form: one `update` call per field, so the sponge sees the
/// preimage in radix-byte and hash-width fragments.
fn streamed(prefix: &[u8], children: &[(u8, [u8; MERKLE_HASH_LEN])]) -> [u8; MERKLE_HASH_LEN] {
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
    hasher.finalize()[..MERKLE_HASH_LEN]
        .try_into()
        .expect("digest prefix has the comparison width")
}

/// Compare the production contiguous hash with incremental sponge updates.
fn branch_hash(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("branch_hash");
    for &fanout in FANOUTS {
        let kids = children(fanout);
        assert_eq!(
            contiguous(PREFIX, &kids),
            streamed(PREFIX, &kids),
            "feeding strategies must hash the same branch input",
        );
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
