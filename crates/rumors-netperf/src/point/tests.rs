use proptest::prelude::*;

use super::*;

proptest! {
    /// The key pairs satisfy *K = s·(k_A + k_B) + k_A·k_B*: the pairs that
    /// are not two copies of one shared key are those involving at least
    /// one unshared key.
    #[test]
    fn key_pairs_count_unshared_pairs(
        shared in 0u64..1 << 20,
        insert_a in 0u64..1 << 20,
        insert_b in 0u64..1 << 20,
        redact_a in 0u64..1 << 20,
        redact_b in 0u64..1 << 20,
    ) {
        let replicas = Replicas { shared, insert_a, insert_b, redact_a, redact_b };
        let (s, k_a, k_b) = (shared as f64, replicas.only_a() as f64, replicas.only_b() as f64);
        let model = Model::of(&replicas, 100.0);
        prop_assert_eq!(model.key_pairs, s * (k_a + k_b) + k_a * k_b);
        prop_assert_eq!(replicas.live_a(), shared + replicas.only_a());
        prop_assert_eq!(replicas.live_b(), shared + replicas.only_b());
    }

    /// The expected deepest depth grows with the key pairs and stays within
    /// one level of `log₂₅₆ K` once K is large.
    #[test]
    fn expected_depth_tracks_log_256(exponent in 2.0f64..30.0, step in 1.0f64..10.0) {
        let depth = |k: f64| {
            (1..=32).map(|level| 1.0 - (-k / 256f64.powi(level)).exp()).sum::<f64>()
        };
        let k = 2f64.powf(exponent * 2.0);
        prop_assert!(depth(k) <= depth(k * step) + 1e-12);
        prop_assert!((depth(k) - k.log(256.0)).abs() <= 1.0);
    }
}

/// Counts print in scientific shorthand exactly when they are one digit times a
/// power of ten of at least a thousand.
#[test]
fn compact_counts() {
    assert_eq!(compact_count(100_000), "1e5");
    assert_eq!(compact_count(9_000), "9e3");
    assert_eq!(compact_count(9_900), "9900");
    assert_eq!(compact_count(10), "10");
}
