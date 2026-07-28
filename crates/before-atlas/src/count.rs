//! Exact counting tables over the packed codec grammars.
//!
//! Uniform sampling from an exact-size input space needs, at every choice
//! point, the exact number of canonical completions each choice admits.
//! This module builds those numbers: for each exact bit length, the count
//! of packed subtrees the strict decoders accept, as arbitrary-precision
//! integers (the counts grow like `2^(0.96 n)`, so machine words overflow
//! within one cache line of stream).
//!
//! The two grammars, transcribed from the decoders' accept sets:
//!
//! - **Version** (the skyline coding): a preorder full binary tree, one
//!   flag bit per node (`0` internal, `1` leaf), each leaf followed by an
//!   Elias-gamma code (`k` zero bits, then `v + 1` in `k + 1` bits; bucket
//!   `k` holds exactly `2^k` values at cost `2k + 1`). Canonical form
//!   excludes an internal node whose two children are both leaves with a
//!   zero right code (the collapsible sibling pair), requires every
//!   running leaf height to stay nonnegative, and requires exactly one
//!   complete tree. The table here counts the family with the sibling
//!   rule and exactness but **without** the nonnegativity constraint:
//!   nonnegativity couples every leaf's decoded value across the whole
//!   stream (the reachable-height state space at `r` remaining bits grows
//!   like `2^(r/2)`), so no polynomial-state exact table exists for it;
//!   the sampler restores exactness-of-measure by whole-sample rejection
//!   (see [`crate::sample`]).
//! - **Party** (the id coding): a 2-bit child-presence tag per node
//!   (`00` terminal, `10`/`01` one child on the named side, `11` both),
//!   canonical form excluding a node with two terminal children. No
//!   payloads, so the table is the whole canonical family, exactly.
//!
//! Every count here is pinned against ground truth in two independent
//! ways (`tests.rs`): exhaustive enumeration of the grammar, and the real
//! decoders' accept sets over all short byte strings.

use num_bigint::BigUint;
use num_traits::{One, Zero};

#[cfg(test)]
mod tests;

/// A version leaf costs one flag bit plus a gamma code of `2k + 1` bits:
/// the smallest leaf (`k = 0`) is 2 bits.
pub const MIN_VERSION_BITS: usize = 2;

/// The smallest party is a single terminal: one 2-bit presence tag.
pub const MIN_PARTY_BITS: usize = 2;

/// The number of gamma codes of exactly `2k + 1` bits: bucket `k` holds
/// `2^k` values (`v + 1` ranges over `[2^k, 2^(k+1))`).
fn gamma_bucket(k: usize) -> BigUint {
    BigUint::one() << k
}

/// The number of bare version leaves of exactly `bits` bits: `2^k` for
/// `bits = 2k + 2` (flag plus code), zero for every other length.
pub fn version_leaf_count(bits: usize) -> BigUint {
    if bits >= 2 && bits.is_multiple_of(2) {
        gamma_bucket((bits - 2) / 2)
    } else {
        BigUint::zero()
    }
}

/// Per-bit-length subtree counts for the version grammar (sibling rule
/// and exactness enforced; nonnegativity deliberately not — the module
/// doc carries the argument).
pub struct VersionCounts {
    /// `subtree[j]` = the number of subtrees of exactly `j` bits.
    subtree: Vec<BigUint>,
}

impl VersionCounts {
    /// Build the table for subtree sizes up to `max_bits` inclusive.
    ///
    /// One quadratic pass of big-integer convolutions: `subtree[j]` sums
    /// the bare leaves of `j` bits plus, per split `(a, b)` with
    /// `a + b = j - 1`, the pairs `subtree[a] * subtree[b]` minus the
    /// excluded (bare left leaf, bare zero right leaf) pairs — the right
    /// zero leaf is the unique 2-bit subtree, so the exclusion at `j` is
    /// exactly the bare-leaf count at `j - 3`.
    pub fn build(max_bits: usize) -> VersionCounts {
        let mut subtree: Vec<BigUint> = Vec::with_capacity(max_bits + 1);
        for j in 0..=max_bits {
            let mut total = version_leaf_count(j);
            // Internal: 1 flag bit, then two subtrees of at least 2 bits.
            if j >= 5 {
                for a in 2..=(j - 3) {
                    let b = j - 1 - a;
                    total += &subtree[a] * &subtree[b];
                }
                total -= version_leaf_count(j - 3);
            }
            subtree.push(total);
        }
        VersionCounts { subtree }
    }

    /// The number of subtrees of exactly `bits` bits (any context).
    pub fn subtree(&self, bits: usize) -> &BigUint {
        &self.subtree[bits]
    }

    /// The number of whole packed versions of exactly `bits` bits: the
    /// root is an unconstrained subtree position.
    pub fn whole(&self, bits: usize) -> &BigUint {
        self.subtree(bits)
    }

    /// The largest subtree size the table covers.
    pub fn max_bits(&self) -> usize {
        self.subtree.len() - 1
    }
}

/// Per-bit-length subtree counts for the party (id) grammar — the whole
/// canonical family, exactly (no payloads, so no rejection anywhere).
pub struct PartyCounts {
    /// `subtree[j]` = the number of canonical id subtrees of exactly `j` bits.
    subtree: Vec<BigUint>,
}

impl PartyCounts {
    /// Build the table for subtree sizes up to `max_bits` inclusive.
    ///
    /// `subtree[j]` sums the terminal (`j = 2`), the two one-child tags
    /// over a child of `j - 2` bits, and per split `(a, b)` with
    /// `a + b = j - 2` the pairs `subtree[a] * subtree[b]` minus the one
    /// excluded terminal-terminal pair (which exists only at `j = 6`).
    pub fn build(max_bits: usize) -> PartyCounts {
        let mut subtree: Vec<BigUint> = Vec::with_capacity(max_bits + 1);
        for j in 0..=max_bits {
            let mut total = if j == 2 {
                BigUint::one()
            } else {
                BigUint::zero()
            };
            if j >= 4 {
                total += 2u32 * &subtree[j - 2];
            }
            if j >= 6 {
                for a in 2..=(j - 4) {
                    let b = j - 2 - a;
                    total += &subtree[a] * &subtree[b];
                }
                if j == 6 {
                    total -= BigUint::one();
                }
            }
            subtree.push(total);
        }
        PartyCounts { subtree }
    }

    /// The number of canonical id subtrees of exactly `bits` bits.
    pub fn subtree(&self, bits: usize) -> &BigUint {
        &self.subtree[bits]
    }

    /// The number of whole packed parties of exactly `bits` bits. The
    /// empty (anonymous) id is rejected by `Party::decode`, so the root
    /// is any nonzero subtree: the same count.
    pub fn whole(&self, bits: usize) -> &BigUint {
        self.subtree(bits)
    }

    /// The largest subtree size the table covers.
    pub fn max_bits(&self) -> usize {
        self.subtree.len() - 1
    }
}

/// The exact bit lengths a packed encoding of exactly `bytes` bytes can
/// carry: `decode` requires fewer than 8 zero pad bits, so the window is
/// `(8 (bytes - 1), 8 bytes]`, floored at the grammar's minimum.
pub fn bit_window(bytes: usize, min_bits: usize) -> std::ops::RangeInclusive<usize> {
    let hi = 8 * bytes;
    let lo = (8 * (bytes - 1) + 1).max(min_bits);
    lo..=hi
}
