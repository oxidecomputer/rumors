//! Exact-uniform samplers over the packed input spaces, by exact byte size.
//!
//! The sampling mechanism is counting-guided generation: at every node the
//! sampler draws one uniform integer below the exact number of canonical
//! completions ([`crate::count`]) and decodes it into a choice by walking
//! the choice weights in a fixed order — every member of the space gets
//! probability exactly `1 / |space|`, with no backtracking anywhere (a
//! backtracking generator's retry order would bias the measure).
//!
//! One constraint lives outside the tables: the version grammar's
//! nonnegative-height rule couples every leaf value in the stream, and no
//! polynomial-state exact count exists for it (the count module's doc
//! carries the argument). The version sampler therefore draws exact-uniform
//! members of the sibling-rule family and rejects the whole draw whenever a
//! running height goes negative. Restricting a uniform measure to a subset
//! by independent whole-sample rejection is still exactly uniform on the
//! subset — unlike backtracking, rejection never conditions one part of a
//! sample on another. The acceptance rate decays only polynomially (the
//! height walk is a sign-symmetric random walk, so staying nonnegative
//! costs `Θ(1/sqrt(leaves))`); [`VersionSampler::sample_bytes`] reports the
//! tries so runs can quote the measured rate. The party grammar has no
//! cross-cutting constraint: its sampler is pure counting-guided generation
//! with no rejection at all.
//!
//! Uniformity, membership, and counts are pinned in `tests.rs` against
//! exhaustive enumeration and the real decoders.

use num_bigint::{BigUint, RandBigInt};
use num_traits::Zero;
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;

use crate::count::{
    bit_window, version_leaf_count, PartyCounts, VersionCounts, MIN_PARTY_BITS, MIN_VERSION_BITS,
};

#[cfg(test)]
mod tests;

/// A packed bit stream under construction, MSB-first within each byte —
/// the codec's storage order, so the packed bytes are exactly what
/// `decode` reads.
#[derive(Default)]
pub struct BitSink {
    bytes: Vec<u8>,
    len: usize,
}

impl BitSink {
    /// Append one bit.
    pub fn push(&mut self, bit: bool) {
        let byte = self.len / 8;
        if byte == self.bytes.len() {
            self.bytes.push(0);
        }
        if bit {
            self.bytes[byte] |= 0x80 >> (self.len % 8);
        }
        self.len += 1;
    }

    /// The number of live bits.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether no bit has been pushed yet.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The packed bytes, final partial byte zero-padded (the canonical
    /// stored form).
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// Split one gamma code value out of its bucket: append `k` zero bits,
/// then `v + 1 = 2^k + mantissa` in `k + 1` bits, most significant first.
fn push_gamma(sink: &mut BitSink, k: usize, mantissa: &BigUint) {
    for _ in 0..k {
        sink.push(false);
    }
    sink.push(true); // the mantissa's leading 1
    for i in (0..k).rev() {
        sink.push(mantissa.bit(i as u64));
    }
}

/// What the enclosing context forbids at a subtree position.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Unconstrained (a left child, or the root).
    Free,
    /// A right sibling of a bare leaf: the bare zero-code leaf (the
    /// collapsible pair's right half) is excluded.
    NoZeroLeaf,
    /// Conditioned internal: every bare leaf is excluded (the sampler
    /// reaches this branch only after the leaf mass was drawn away).
    NoLeaf,
}

/// The deterministic RNG for one sampling cell.
///
/// The 32-byte ChaCha key mixes the base seed with the cell coordinates
/// through splitmix64 (a fixed, documented expansion — no entropy from
/// time or the OS anywhere), so any cell replays exactly and cells are
/// independent of execution order.
pub fn cell_rng(base_seed: u64, op: &str, size: usize, index: usize) -> ChaCha12Rng {
    let mut h = base_seed ^ 0x9e37_79b9_7f4a_7c15;
    let mut mix = |v: u64| {
        h ^= v;
        h = splitmix64(h);
    };
    for b in op.bytes() {
        mix(u64::from(b));
    }
    mix(0xff); // domain separator: name vs coordinates
    mix(size as u64);
    mix(index as u64);
    let mut seed = [0u8; 32];
    let mut s = h;
    for chunk in seed.chunks_exact_mut(8) {
        s = splitmix64(s);
        chunk.copy_from_slice(&s.to_le_bytes());
    }
    ChaCha12Rng::from_seed(seed)
}

/// One splitmix64 round: the standard finalizer-quality mixer.
fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// The interleaved split order `lo, hi, lo+1, hi-1, …` over `lo..=hi`.
///
/// Purely an efficiency choice (any fixed order is uniform): the counting
/// measure puts most split mass at the extreme splits, so scanning from
/// both ends reaches the drawn index after O(1) expected weights instead
/// of O(range).
fn interleave(lo: usize, hi: usize) -> impl Iterator<Item = usize> {
    let mut a = lo;
    let mut b = hi;
    let mut from_lo = true;
    std::iter::from_fn(move || {
        if a > b {
            return None;
        }
        let next = if from_lo {
            let v = a;
            a += 1;
            v
        } else {
            let v = b;
            b -= 1;
            v
        };
        from_lo = !from_lo;
        Some(next)
    })
}

/// An exact-uniform sampler over packed versions of an exact byte size.
pub struct VersionSampler {
    counts: VersionCounts,
}

/// One version draw: the canonical packed bytes and the live bit length.
pub struct VersionDraw {
    /// Canonical packed bytes (`Version::decode` accepts them; pinned).
    pub bytes: Vec<u8>,
    /// The live bit length before the zero pad.
    pub bits: usize,
    /// Whole-sample rejection draws spent before the accepted one.
    pub rejected: u64,
}

impl VersionSampler {
    /// Build a sampler covering byte sizes up to `max_bytes`.
    pub fn new(max_bytes: usize) -> VersionSampler {
        VersionSampler {
            counts: VersionCounts::build(8 * max_bytes),
        }
    }

    /// The count table (for tests and plan-time feasibility checks).
    pub fn counts(&self) -> &VersionCounts {
        &self.counts
    }

    /// Draw one version uniformly from the canonical versions whose packed
    /// encoding is exactly `bytes` bytes. `None` if the space is empty
    /// (it is not, for any `bytes >= 1` within the table).
    pub fn sample_bytes(&self, bytes: usize, rng: &mut ChaCha12Rng) -> Option<VersionDraw> {
        let window = bit_window(bytes, MIN_VERSION_BITS);
        let total: BigUint = window.clone().map(|m| self.counts.whole(m)).sum();
        if total.is_zero() {
            return None;
        }
        let mut rejected = 0u64;
        loop {
            // The bit length within the byte's window, weighted by the
            // sibling-rule family counts; the rejection below restricts
            // the joint (length, member) draw to the real family, which
            // reweights the lengths to exactly their canonical share.
            let mut u = rng.gen_biguint_below(&total);
            let mut bits = 0;
            for m in window.clone() {
                let w = self.counts.whole(m);
                if &u < w {
                    bits = m;
                    break;
                }
                u -= w;
            }
            let mut sink = BitSink::default();
            let mut walk = HeightWalk::new();
            if self.subtree(bits, Mode::Free, &mut sink, &mut walk, rng) {
                return Some(VersionDraw {
                    bytes: sink.into_bytes(),
                    bits,
                    rejected,
                });
            }
            rejected += 1;
        }
    }

    /// Emit one uniform subtree of exactly `bits` bits into `sink`,
    /// tracking leaf heights in `walk`. Returns `false` (abandon the whole
    /// draw) as soon as a height goes negative.
    fn subtree(
        &self,
        bits: usize,
        mode: Mode,
        sink: &mut BitSink,
        walk: &mut HeightWalk,
        rng: &mut ChaCha12Rng,
    ) -> bool {
        let leaf_w = match mode {
            Mode::NoLeaf => BigUint::zero(),
            Mode::NoZeroLeaf if bits == 2 => BigUint::zero(),
            Mode::Free | Mode::NoZeroLeaf => version_leaf_count(bits),
        };
        let total = match mode {
            Mode::Free => self.counts.subtree(bits).clone(),
            Mode::NoZeroLeaf | Mode::NoLeaf => {
                self.counts.subtree(bits) - version_leaf_count(bits) + &leaf_w
            }
        };
        debug_assert!(!total.is_zero(), "sampled a size with no members");
        let mut u = rng.gen_biguint_below(&total);

        // Option 1: a bare leaf (one flag bit plus one whole gamma bucket;
        // the drawn index's low bits are the uniform mantissa).
        if u < leaf_w {
            let k = (bits - 2) / 2;
            sink.push(true);
            push_gamma(sink, k, &u);
            return walk.leaf(k, &u);
        }
        u -= leaf_w;

        // Option 2: an internal node over a split `(a, b)`, `a + b = bits - 1`
        // (one flag bit, two subtrees of at least 2 bits each).
        assert!(bits >= 5, "no internal option below 5 bits");
        for a in interleave(2, bits - 3) {
            let b = bits - 1 - a;
            // Left a bare leaf forces the right into no-zero-code mode;
            // the two legs partition the split's pairs.
            let bare = version_leaf_count(a)
                * (self.counts.subtree(b) - version_leaf_count(if b == 2 { 2 } else { 0 }));
            let internal =
                (self.counts.subtree(a) - version_leaf_count(a)) * self.counts.subtree(b);
            if u < bare {
                sink.push(false);
                let k = (a - 2) / 2;
                // `u` is uniform below `leaf(a) * count(b, NoZeroLeaf)`:
                // its quotient by the leaf mass is a fresh uniform for the
                // right subtree, its remainder the leaf mantissa — but the
                // right subtree redraws from the rng for simplicity; only
                // the mantissa reuses `u`.
                let mantissa = u % version_leaf_count(a);
                sink.push(true);
                push_gamma(sink, k, &mantissa);
                if !walk.leaf(k, &mantissa) {
                    return false;
                }
                return self.subtree(b, Mode::NoZeroLeaf, sink, walk, rng);
            }
            u -= bare;
            if u < internal {
                sink.push(false);
                if !self.subtree(a, Mode::NoLeaf, sink, walk, rng) {
                    return false;
                }
                return self.subtree(b, Mode::Free, sink, walk, rng);
            }
            u -= internal;
        }
        unreachable!("choice index exceeded the total weight");
    }
}

/// The running leaf-height walk the nonnegativity rule constrains.
struct HeightWalk {
    /// The current height, kept as (nonnegative magnitude); a subtraction
    /// below zero is the rejection event, so a signed value never exists.
    height: BigUint,
    /// Whether the first leaf (absolute height) has been seen.
    seen_leaf: bool,
}

impl HeightWalk {
    fn new() -> HeightWalk {
        HeightWalk {
            height: BigUint::zero(),
            seen_leaf: false,
        }
    }

    /// Account one leaf: bucket `k`, mantissa, value `v = 2^k - 1 + mantissa`.
    ///
    /// The first leaf is the absolute height; every later leaf is a
    /// zigzag delta (`2m -> +m`, `2m - 1 -> -m`). Returns `false` when
    /// the height would go negative.
    fn leaf(&mut self, k: usize, mantissa: &BigUint) -> bool {
        let v = (BigUint::from(1u32) << k) - 1u32 + mantissa;
        if !self.seen_leaf {
            self.seen_leaf = true;
            self.height = v;
            return true;
        }
        if v.bit(0) {
            // Odd zigzag: a descent of magnitude (v + 1) / 2.
            let mag = (v + 1u32) >> 1;
            if self.height < mag {
                return false;
            }
            self.height -= mag;
        } else {
            self.height += v >> 1;
        }
        true
    }
}

/// An exact-uniform sampler over packed parties of an exact byte size:
/// pure counting-guided generation, no rejection anywhere.
pub struct PartySampler {
    counts: PartyCounts,
}

/// One party draw: the canonical packed bytes and the live bit length.
pub struct PartyDraw {
    /// Canonical packed bytes (`Party::decode` accepts them; pinned).
    pub bytes: Vec<u8>,
    /// The live bit length before the zero pad.
    pub bits: usize,
}

impl PartySampler {
    /// Build a sampler covering byte sizes up to `max_bytes`.
    pub fn new(max_bytes: usize) -> PartySampler {
        PartySampler {
            counts: PartyCounts::build(8 * max_bytes),
        }
    }

    /// The count table (for tests and plan-time feasibility checks).
    pub fn counts(&self) -> &PartyCounts {
        &self.counts
    }

    /// Draw one party uniformly from the canonical parties whose packed
    /// encoding is exactly `bytes` bytes.
    pub fn sample_bytes(&self, bytes: usize, rng: &mut ChaCha12Rng) -> Option<PartyDraw> {
        let window = bit_window(bytes, MIN_PARTY_BITS);
        let total: BigUint = window.clone().map(|m| self.counts.whole(m)).sum();
        if total.is_zero() {
            return None;
        }
        let mut u = rng.gen_biguint_below(&total);
        let mut bits = 0;
        for m in window {
            let w = self.counts.whole(m);
            if &u < w {
                bits = m;
                break;
            }
            u -= w;
        }
        let mut sink = BitSink::default();
        self.subtree(bits, false, &mut sink, rng);
        Some(PartyDraw {
            bytes: sink.into_bytes(),
            bits,
        })
    }

    /// Emit one uniform id subtree of exactly `bits` bits into `sink`.
    /// `no_terminal` excludes the bare terminal (the sibling of a terminal,
    /// or a position whose terminal mass was already drawn away).
    fn subtree(&self, bits: usize, no_terminal: bool, sink: &mut BitSink, rng: &mut ChaCha12Rng) {
        let terminal_w = if bits == 2 && !no_terminal {
            BigUint::from(1u32)
        } else {
            BigUint::zero()
        };
        // The terminal is the unique 2-bit subtree, so excluding it only
        // changes the total at `bits == 2` — a position the weights never
        // select (its weight is zero wherever `no_terminal` holds).
        let total = if bits == 2 {
            terminal_w.clone()
        } else {
            self.counts.subtree(bits).clone()
        };
        assert!(!total.is_zero(), "sampled a size with no members");
        let mut u = rng.gen_biguint_below(&total);

        // Option 1: the terminal, tag `00`.
        if u < terminal_w {
            sink.push(false);
            sink.push(false);
            return;
        }
        u -= terminal_w;

        // Option 2: a one-child node, tag `10` (left) or `01` (right).
        if bits >= 4 {
            let unary = self.counts.subtree(bits - 2);
            for &left_side in &[true, false] {
                if &u < unary {
                    sink.push(left_side);
                    sink.push(!left_side);
                    return self.subtree(bits - 2, false, sink, rng);
                }
                u -= unary;
            }
        }

        // Option 3: a both-children node, tag `11`, split `(a, b)` with
        // `a + b = bits - 2`; the terminal-terminal pair is excluded.
        for a in interleave(2, bits.saturating_sub(4)) {
            let b = bits - 2 - a;
            let term_left = if a == 2 {
                self.counts.subtree(b)
                    - if b == 2 {
                        BigUint::from(1u32)
                    } else {
                        BigUint::zero()
                    }
            } else {
                BigUint::zero()
            };
            let deep_left = (self.counts.subtree(a)
                - if a == 2 {
                    BigUint::from(1u32)
                } else {
                    BigUint::zero()
                })
                * self.counts.subtree(b);
            if u < term_left {
                sink.push(true);
                sink.push(true);
                sink.push(false);
                sink.push(false);
                return self.subtree(b, true, sink, rng);
            }
            u -= term_left;
            if u < deep_left {
                sink.push(true);
                sink.push(true);
                self.subtree(a, true, sink, rng);
                return self.subtree(b, false, sink, rng);
            }
            u -= deep_left;
        }
        unreachable!("choice index exceeded the total weight");
    }
}
