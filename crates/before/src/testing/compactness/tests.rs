//! The Tier 2 ratio meter: every input family holds the size envelope and
//! the Euler-tour charge bound (see the module doc for both statements).
//!
//! Ratio record of the 2026-07-23 measurement sweep (thousands of samples
//! per random family; the deterministic grids in full): the global maximum
//! ratio is 1.9966 at the alternating comb with 2048-bit teeth and 1024
//! pairs, and the maximum off-comb ratio is 1.9633 on an arbitrary-generator
//! tree of 7 nodes whose ~124-bit bases alternate high/low across leaf
//! order — the comb mechanism arising at random. No sample in any family
//! reached ratio 2.

use proptest::prelude::*;

use crate::testing::bridge::from_oracle_version;
use crate::testing::{generators, optrace};
use crate::{meter, Clock, Version};

use super::{arb_comb_params, check_sample, comb};

/// Decode a meter-generated packed shape into a `Version`.
fn decode(packed: &meter::Packed) -> Version {
    Version::decode(&packed.bytes[..]).expect("meter shapes are strict normal form")
}

proptest! {
    /// Arbitrary normal-form event trees hold the size envelope and the
    /// Euler-tour charge bound.
    ///
    /// The generator's base magnitudes span small values to past `u64::MAX`.
    /// Measured maximum ratio in this family: 1.9633, on a 7-node tree with
    /// ~124-bit bases alternating across leaf order.
    #[test]
    fn arbitrary_versions_hold_the_envelope(t in generators::arb_oracle_version()) {
        check_sample(&from_oracle_version(&t));
    }

    /// Every version produced by an organic fork/tick/send/sync/join history
    /// from one seed holds the size envelope and the Euler-tour charge bound.
    ///
    /// Measured maximum ratio in this family: 1.50 (median 1.00; Tier 2 was
    /// strictly smaller on 30-40% of organic versions).
    #[test]
    fn organic_histories_hold_the_envelope(ops in optrace::world_strategy_up_to(120)) {
        let mut clocks = vec![Clock::seed()];
        for op in &ops {
            optrace::step_impl(&mut clocks, op);
        }
        for clock in &clocks {
            check_sample(clock.version());
        }
    }

    /// The alternating comb — the shape whose ratio approaches the factor-2
    /// ceiling — holds the size envelope and the Euler-tour charge bound at
    /// every sampled tooth magnitude and length.
    ///
    /// Measured maximum ratio: 1.9966 at 2048-bit teeth, 1024 pairs — the
    /// global maximum across all families.
    #[test]
    fn alternating_combs_hold_the_envelope((m_bits, pairs) in arb_comb_params()) {
        check_sample(&comb(m_bits, pairs));
    }
}

/// The adversarial event shapes of record (dense spine, bigroot, hugeleaf)
/// hold the size envelope and the Euler-tour charge bound across a size grid
/// from minimal to deep/wide.
///
/// Measured ratios here never exceed 1: hugeleaf is exactly 1 at every
/// magnitude, and the spine shapes are where Tier 2 is smallest (dense
/// reaches 0.75, bigroot 0.7502 — near-zero deltas replace stored codes).
#[test]
fn adversarial_shapes_hold_the_envelope() {
    for d in [1, 2, 3, 8, 64, 512, 4096] {
        check_sample(&decode(&meter::dense(d)));
    }
    for b in [1, 2, 8, 64, 512, 4096] {
        check_sample(&decode(&meter::hugeleaf(b)));
    }
    for b in [1, 8, 64, 512] {
        for d in [1, 8, 64, 512] {
            check_sample(&decode(&meter::bigroot(b, d)));
        }
    }
}

/// The comb's exact sizes at the tightness point: at `m_bits = pairs = 1024`
/// the ratio is pinned above 1.994 — within 0.6% of the factor-2 ceiling —
/// confirming the envelope's factor is tight, not loose.
#[test]
fn comb_ratio_is_tight_against_the_factor_two_ceiling() {
    let sample = check_sample(&comb(1024, 1024));
    assert_eq!(sample.tier2.total_bits, 4_198_399);
    assert_eq!(sample.current_bits, 2_105_342);
    assert!(
        sample.ratio > 1.994 && sample.ratio < 2.0,
        "comb tightness drifted: ratio {:.6}",
        sample.ratio,
    );
}
