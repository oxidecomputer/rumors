//! The Tier 2 compactness envelope and the Euler-tour charge probe.
//!
//! The Tier 2 coding stores preorder topology plus delta-coded absolute leaf
//! values ([`crate::meter::tier2`]); the claim its adoption turns on is that
//! its coded size never exceeds ~2x today's size plus O(1) bits per node.
//! This module holds the shared scaffolding for the tests that pin that claim
//! over every input family: the envelope and charge checks with their
//! measured constants, and the alternating-comb builder — the shape whose
//! ratio approaches the factor-2 ceiling from below, tight for both bounds.
//!
//! The argument the probe validates executably: a consecutive-leaf delta
//! telescopes over the two path segments to the leaves' lowest common
//! ancestor, so its magnitude is at most the sum of today's stored bases on
//! those segments; each stored base lies on the exit path of exactly one
//! consecutive-leaf pair and the entry path of exactly one other (an Euler
//! tour of the tree), so it is charged at most twice; and
//! `gamma(x + y) <= gamma(x) + gamma(y) + 1` turns the value bound into a
//! code-length bound, spending O(1) bits per merge. The checks below assert
//! the code-length forms directly, with the per-node and per-leaf constants
//! pinned at their measured ceilings.

use proptest::prelude::*;

use crate::codec::{self, Base, BitsBuf};
use crate::meter::tier2::{tier2_size, Tier2Size};
use crate::Version;

#[cfg(test)]
mod tests;

/// Per-node envelope constant: `tier2_bits <= 2 * current_bits +
/// TIER2_NODE_ENVELOPE_BITS * nodes` on every measured sample.
///
/// Provenance: measured (~13k samples: 5000 arbitrary trees,
/// ~7800 organic-history versions at trace lengths up to 400, the
/// adversarial-shape and comb grids, and realistic gossip populations). The
/// derivation (each stored base charged at most twice, O(1) bits per gamma
/// merge and per zigzag sign) allows a small positive constant; the measured
/// per-node excess `(tier2 - 2 * current) / nodes` never exceeded -1.57 on
/// any family, so the pinned ceiling is 0 — Tier 2 never exceeded twice
/// today's size outright, with over a bit per node of headroom. Any sample
/// breaking this is a decision-critical finding: capture it as a regression
/// case and re-pin the honest constant.
pub(crate) const TIER2_NODE_ENVELOPE_BITS: f64 = 0.0;

/// Per-leaf charge constant for the Euler-tour probe: `delta_bits <=
/// 2 * stored_base_bits + EULER_LEAF_CHARGE_BITS * leaves` on every measured
/// sample.
///
/// Provenance: measured (the same sweep as
/// [`TIER2_NODE_ENVELOPE_BITS`]), with the delta stream priced at its
/// zigzag-gamma lengths (what the codec would emit; at most 2 bits per leaf
/// over `gamma(|delta|)`), so the constant covers the sign convention too.
/// The measured per-leaf excess `(delta_bits - 2 * stored) / leaves` never
/// exceeded -1.33 on any family, so the pinned ceiling is 0, with over a bit
/// per leaf of headroom.
pub(crate) const EULER_LEAF_CHARGE_BITS: f64 = 0.0;

/// One measured sample: the Tier 2 split, today's bit length, and the ratio.
#[derive(Debug, Clone)]
pub(crate) struct Sample {
    /// The Tier 2 size split of the sampled version.
    pub(crate) tier2: Tier2Size,
    /// Today's live encoded bit length of the same version.
    pub(crate) current_bits: u64,
    /// The compactness ratio `tier2.total_bits / current_bits`.
    pub(crate) ratio: f64,
}

/// Measure one version and assert both pinned bounds, returning the sample.
///
/// The two assertions are the suite's contract: the size envelope
/// (`tier2 <= 2 * current + C_node * nodes`, [`TIER2_NODE_ENVELOPE_BITS`])
/// and the Euler-tour charge (`delta_bits <= 2 * stored_base_bits +
/// C_leaf * leaves`, [`EULER_LEAF_CHARGE_BITS`], where the stored-base code
/// bits are exactly `current_bits - nodes`).
pub(crate) fn check_sample(version: &Version) -> Sample {
    // The decision-era "current" coding is the min-lifted packed preorder
    // stream (one gamma-coded base per node), re-derived through the
    // oracle lowering; the stored coding is Tier 2 itself.
    let packed =
        crate::testing::bridge::packed_bits_of(&crate::testing::bridge::to_oracle_version(version));
    let tier2 = tier2_size(crate::codec::built_view(&packed));
    let current_bits = packed.len();
    let ratio = tier2.total_bits as f64 / current_bits as f64;

    let envelope = 2.0 * current_bits as f64 + TIER2_NODE_ENVELOPE_BITS * tier2.nodes as f64;
    assert!(
        tier2.total_bits as f64 <= envelope,
        "Tier 2 size envelope violated: {} bits > 2 * {current_bits} + {} * {} \
         (ratio {ratio:.4}): decision-critical, pin this witness: {tier2:?}",
        tier2.total_bits,
        TIER2_NODE_ENVELOPE_BITS,
        tier2.nodes,
    );

    let stored_base_bits = current_bits - tier2.nodes;
    let charge = 2.0 * stored_base_bits as f64 + EULER_LEAF_CHARGE_BITS * tier2.leaves as f64;
    assert!(
        tier2.delta_bits as f64 <= charge,
        "Euler-tour charge violated: delta stream {} bits > 2 * {stored_base_bits} + {} * {}: \
         decision-critical, pin this witness: {tier2:?}",
        tier2.delta_bits,
        EULER_LEAF_CHARGE_BITS,
        tier2.leaves,
    );

    Sample {
        tier2,
        current_bits,
        ratio,
    }
}

/// Build the alternating comb: `pairs` subtrees `(0, 0, M)` with
/// `M = 2^m_bits - 1`, hung off a left-leaning zero-base spine.
///
/// Preorder leaf values alternate `0, M, 0, M, ...` — every consecutive-leaf
/// delta is a full `+M`/`-M` swing while today's form stores each `M` once —
/// the shape whose ratio approaches 2 from below as `m_bits` grows, tight for
/// both the size envelope and the charge bound. Exact sizes (pinned by this
/// module's tests): `4 * pairs - 1` nodes,
/// `pairs * (2 * m_bits + 8) - 2` bits today. Strict normal form (every
/// internal node has a zero-base child; the only sibling leaf pair is
/// `(0, M)` with `M >= 1`), asserted by round-trip here.
///
/// # Panics
///
/// Panics if `m_bits == 0` or `pairs == 0`, or if the built stream fails its
/// round-trip self-check.
pub(crate) fn comb(m_bits: usize, pairs: usize) -> Version {
    assert!(m_bits >= 1, "comb needs a nonzero tooth magnitude");
    assert!(pairs >= 1, "comb needs at least one tooth");
    let m_bits_u32 = u32::try_from(m_bits).expect("tooth magnitude bit count fits u32");
    let m = (Base::from(1u8) << m_bits_u32) - &Base::from(1u8);

    let pair_bits = 2 * m_bits + 8;
    let mut bits = BitsBuf::with_capacity((pairs * pair_bits - 2) as u64);
    // The spine: each node is `1 . gamma(0)`, its left child the next spine
    // node (the innermost left child is the first pair subtree).
    for _ in 0..pairs - 1 {
        bits.push(true);
        codec::encode_int(&mut bits, &Base::ZERO);
    }
    // The pair subtrees, innermost first: `(0, 0, M)` is
    // `1 . gamma(0) . 0 . gamma(0) . 0 . gamma(M)`.
    for _ in 0..pairs {
        bits.push(true);
        codec::encode_int(&mut bits, &Base::ZERO);
        bits.push(false);
        codec::encode_int(&mut bits, &Base::ZERO);
        bits.push(false);
        codec::encode_int(&mut bits, &m);
    }
    // The comb is hand-built in the min-lifted packed construction
    // language; the transcoding bridge lifts it into the stored coding.
    let version = Version::from_bits(crate::version::skyline::encode_bits(
        crate::codec::built_view(&bits),
    ));

    // Self-check: the built stream is canonical and round-trips the wire.
    let decoded = Version::decode(version.encode().as_slice())
        .expect("hand-built comb is strict normal form");
    assert_eq!(decoded, version, "comb round-trips canonically");
    version
}

/// Strategy over `(m_bits, pairs)` comb parameters spanning small teeth to
/// magnitudes far past one machine word, at several comb lengths.
pub(crate) fn arb_comb_params() -> impl Strategy<Value = (usize, usize)> {
    (
        prop_oneof![
            Just(1usize),
            1usize..=8,
            Just(32usize),
            Just(64usize),
            Just(200usize)
        ],
        prop_oneof![Just(1usize), 1usize..=8, Just(64usize), Just(256usize)],
    )
}
