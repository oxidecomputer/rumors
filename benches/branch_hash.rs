// `criterion_group!` generates a public registrar with no documentation.
#![allow(missing_docs)]

//! Feeding-strategy microbenchmark behind `HashKey::branch`'s one-shot form.
//!
//! A branch preimage is `kind ‖ height ‖ prefix_len ‖ prefix ‖ count ‖
//! (radix ‖ hash)*`, hashed by HMAC-SHA-256 under the tree hash key — dominated
//! by fixed-width child records, several to one of SHA-256's 64-byte blocks.
//! The shipped `HashKey::branch` assembles the whole preimage in a contiguous
//! buffer and hashes it in one shot; the alternative feeds the keyed hasher one
//! `update` per field. This bench measures exactly that comparison over the
//! current preimage layout, across fan-outs from the smallest representable
//! branch to the saturated 256, so the claim at `HashKey::branch` stays
//! re-measurable: run `just bench branch_hash` and compare the `contiguous` and
//! `streamed` curves.
//!
//! The contiguous series calls the production implementation through its
//! test-only entry point. The streamed alternative spells out the same input,
//! and an untimed assertion keeps the two forms equivalent. Both key the hash
//! once, outside the timed loop, and clone the keyed state per branch, as the
//! tree does.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use hmac::{Hmac, KeyInit, Mac};
use rumors::testing::{BranchHasher, HASH_KEY_LEN, MERKLE_HASH_LEN};
use sha2::Sha256;

/// Kind byte leading a branch preimage, mirrored from the documented layout.
const BRANCH_TAG: u8 = 1;

/// The height the benched branch sits at: one below the root, as a hot interior
/// branch near the root does.
const HEIGHT: u8 = 31;

/// A fixed raw key: only that both series use the same one matters.
const KEY: [u8; HASH_KEY_LEN] = [0x5e; HASH_KEY_LEN];

/// A hot node's compressed span: short, as path compression typically leaves
/// interior branches near the root.
const PREFIX: &[u8] = &[0xa5, 0x5a, 0x3c];

/// Fan-outs to sweep: 2 is the smallest representable branch under the
/// canonical-shape invariant, 256 a saturated one, the rest fill in between.
const FANOUTS: &[usize] = &[2, 4, 16, 64, 256];

/// A deterministic set of `k` (radix, hash) children in strictly ascending
/// radix order, as the convention requires. Only the byte content matters to a
/// hashing microbench, and only that it is fixed across runs.
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
fn contiguous(
    key: &BranchHasher,
    prefix: &[u8],
    children: &[(u8, [u8; MERKLE_HASH_LEN])],
) -> [u8; MERKLE_HASH_LEN] {
    key.branch_hash(usize::from(HEIGHT), prefix, children.iter().copied())
}

/// The streamed form: one `update` call per field, so the keyed hasher sees the
/// preimage in radix-byte and hash-width fragments.
fn streamed(
    key: &Hmac<Sha256>,
    prefix: &[u8],
    children: &[(u8, [u8; MERKLE_HASH_LEN])],
) -> [u8; MERKLE_HASH_LEN] {
    let mut hasher = key.clone();
    hasher.update(&[BRANCH_TAG, HEIGHT]);
    hasher
        .update(&[u8::try_from(prefix.len()).expect("a compressed span fits in one length byte")]);
    hasher.update(prefix);
    let count = u16::try_from(children.len()).expect("fan-out fits u16");
    hasher.update(&count.to_be_bytes());
    for (radix, hash) in children {
        hasher.update(&[*radix]);
        hasher.update(hash);
    }
    hasher.finalize().into_bytes()[..MERKLE_HASH_LEN]
        .try_into()
        .expect("digest prefix has the comparison width")
}

/// Compare the production contiguous hash with incremental hasher updates.
fn branch_hash(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("branch_hash");
    let production = BranchHasher::new(&KEY);
    let keyed = <Hmac<Sha256>>::new_from_slice(&KEY).expect("HMAC accepts a key of any length");
    for &fanout in FANOUTS {
        let kids = children(fanout);
        assert_eq!(
            contiguous(&production, PREFIX, &kids),
            streamed(&keyed, PREFIX, &kids),
            "feeding strategies must hash the same branch input",
        );
        group.bench_with_input(BenchmarkId::new("contiguous", fanout), &kids, |b, kids| {
            b.iter(|| contiguous(&production, black_box(PREFIX), black_box(kids)));
        });
        group.bench_with_input(BenchmarkId::new("streamed", fanout), &kids, |b, kids| {
            b.iter(|| streamed(&keyed, black_box(PREFIX), black_box(kids)));
        });
    }
    group.finish();
}

criterion_group!(benches, branch_hash);
criterion_main!(benches);
