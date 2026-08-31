//! Deterministic tree-shape staging for the wire-pin fixtures.
//!
//! A leaf's tree path is a pure function of its version, and a staged
//! universe's versions are a pure function of the staging script (the
//! seeded network, the fork points, the send order) — payload bytes steer
//! nothing. A fixture therefore stages a required shape in three steps:
//! send a pool of messages, search the created versions for ones whose
//! paths satisfy the shape, and redact the rest. Same script, same
//! versions, same winners, every run: the searches here are deterministic,
//! and each fixture's self-checks still verify the landed shape.

use rumors::{Rumors, Version};

/// A leaf's tree path: the full-width BLAKE3 hash of its version's
/// canonical bytes.
pub fn leaf_path(version: &Version) -> [u8; 32] {
    *blake3::hash(version.as_bytes()).as_bytes()
}

/// The root radix of a leaf's path: its first byte.
pub fn path_radix(version: &Version) -> u8 {
    leaf_path(version)[0]
}

/// Send `count` messages carrying the payloads `from..from + count`, as
/// one batch: one fresh version per payload, in payload order.
pub fn send_pool(rumors: &Rumors<u64>, from: u64, count: u64) {
    rumors
        .batch(|batch| {
            for value in from..from + count {
                batch.send(value)?;
            }
            Ok::<(), rumors::EncodeError>(())
        })
        .expect("flat test payloads are within any depth limit");
}

/// The live pool as `(payload, version)` in ascending payload order,
/// restricted to payloads in `from..from + count`: the deterministic
/// search order for the shape searches.
pub fn pool(rumors: &Rumors<u64>, from: u64, count: u64) -> Vec<(u64, Version)> {
    let mut pool: Vec<(u64, Version)> = rumors
        .snapshot()
        .iter()
        .filter(|(_, m)| (from..from + count).contains(m))
        .map(|(v, m)| (*m, v.clone()))
        .collect();
    pool.sort_by_key(|(value, _)| *value);
    pool
}

/// Redact every live message whose payload lies in `from..from + count`
/// but is not listed in `keep`: the pool cleanup after a shape search.
pub fn keep_only(rumors: &Rumors<u64>, from: u64, count: u64, keep: &[u64]) {
    let losers: Vec<Version> = rumors
        .snapshot()
        .iter()
        .filter(|(_, m)| (from..from + count).contains(m) && !keep.contains(m))
        .map(|(v, _)| v.clone())
        .collect();
    rumors
        .batch(|batch| {
            for version in &losers {
                batch.redact(version);
            }
            Ok::<(), rumors::EncodeError>(())
        })
        .expect("flat test payloads are within any depth limit");
}

/// The first pool pair (in payload order) whose paths agree on the
/// leading `shared` bytes; `distinct_next` additionally requires the byte
/// after the shared span to differ (a split exactly one level below).
///
/// Panics if the pool holds no such pair: enlarge the pool — the verdict
/// is deterministic, never flaky.
pub fn shaped_pair(pool: &[(u64, Version)], shared: usize, distinct_next: bool) -> (u64, u64) {
    for (i, (first, v1)) in pool.iter().enumerate() {
        let p1 = leaf_path(v1);
        for (second, v2) in &pool[i + 1..] {
            let p2 = leaf_path(v2);
            if p1[..shared] == p2[..shared] && (!distinct_next || p1[shared] != p2[shared]) {
                return (*first, *second);
            }
        }
    }
    panic!("no pool pair shares a {shared}-byte path prefix: enlarge the pool");
}

/// The first `count` pool payloads whose root radix is not `avoid`:
/// ballast that stays out of a fixture's disputed subtree.
///
/// Panics if the pool runs dry: enlarge it — the verdict is
/// deterministic, never flaky.
pub fn ballast_avoiding(pool: &[(u64, Version)], avoid: u8, count: usize) -> Vec<u64> {
    let picked: Vec<u64> = pool
        .iter()
        .filter(|(_, v)| path_radix(v) != avoid)
        .take(count)
        .map(|(value, _)| *value)
        .collect();
    assert_eq!(
        picked.len(),
        count,
        "the pool cannot fill the ballast quota: enlarge it"
    );
    picked
}
