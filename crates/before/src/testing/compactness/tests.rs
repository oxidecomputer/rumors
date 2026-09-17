//! Checks the Tier 2 size envelope and Euler-tour charge bound.

use crate::meter::registry::Shape;
use proptest::prelude::*;

use crate::testing::bridge::from_oracle_version;
use crate::testing::{generators, optrace};
use crate::{meter, Clock, Version};

use super::{arb_comb_params, check_sample, comb};

/// Decode a meter-generated encoded shape into a `Version`.
fn decode(encoded: &meter::Encoding) -> Version {
    encoded.version()
}

proptest! {
    /// Arbitrary normal-form event trees hold the size envelope and the
    /// Euler-tour charge bound.
    ///
    /// The generator's base magnitudes span small values to past `u64::MAX`.
    #[test]
    fn arbitrary_versions_hold_the_envelope(t in generators::arb_oracle_version()) {
        check_sample(&from_oracle_version(&t));
    }

    /// Every version produced by an organic fork/tick/send/sync/join history
    /// from one seed holds the size envelope and the Euler-tour charge bound.
    ///
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
    #[test]
    fn alternating_combs_hold_the_envelope((m_bits, pairs) in arb_comb_params()) {
        check_sample(&comb(m_bits, pairs));
    }
}

/// Deep-spine and wide-magnitude shapes hold both bounds across a deterministic
/// size grid.
#[test]
fn registered_shapes_hold_the_envelope() {
    for d in [1, 2, 3, 8, 64, 512, 4096] {
        check_sample(&decode(&Shape::Dense.build1(d)));
    }
    for b in [1, 2, 8, 64, 512, 4096] {
        check_sample(&decode(&Shape::Hugeleaf.build1(b)));
    }
    for b in [1, 8, 64, 512] {
        for d in [1, 8, 64, 512] {
            check_sample(&decode(&Shape::Bigroot.build2(b, d)));
        }
    }
}

/// The comb's exact sizes at the tightness point: at `m_bits = pairs = 1024`
/// the ratio is pinned above 1.994 — within 0.6% of the factor-2 ceiling —
/// demonstrating that the factor cannot be reduced materially.
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
