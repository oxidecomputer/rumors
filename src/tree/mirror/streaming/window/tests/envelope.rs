//! Check the integer window bounds against a floating-point Chernoff calculation.
//!
//! The reference uses occupancy probabilities and a numerical tail search,
//! independent of production's integer approximations. It checks the stated
//! uniform-hash model; sampled comparisons do not prove that model.

use proptest::prelude::*;

use super::super::{FAN, KEY_DEPTH, children_quantile, disputed, occupied, stage_population};

/// Tail probability promised for each statistic, independently of its implementation.
const TAIL_BITS: f64 = 48.0;

/// Negative log of the Chernoff upper bound at `count`, given an upper mean.
///
/// For `count > mean`, this is `count * ln(count / mean) - count + mean`.
/// The same bound applies to independent Bernoulli trials and negatively
/// associated occupancy indicators. See the [Chernoff derivation][chernoff].
///
/// [chernoff]: https://courses.csail.mit.edu/6.856/21/Notes/n6-chernoff.html
fn tail_exponent(mean: f64, count: f64) -> f64 {
    if count <= mean {
        return 0.0;
    }
    if mean == 0.0 {
        return f64::INFINITY;
    }
    let relative = (count - mean) / mean;
    if relative < 0.001 {
        // Near the mean, subtracting the two leading terms loses precision.
        // Expand (1+r) ln(1+r) - r; the omitted term is below f64 precision
        // relative to r² at this threshold.
        mean * relative
            * relative
            * (0.5
                + relative
                    * (-1.0 / 6.0
                        + relative * (1.0 / 12.0 + relative * (-1.0 / 20.0 + relative / 30.0))))
    } else {
        count * relative.ln_1p() - (count - mean)
    }
}

/// Smallest limit whose exceedance has the requested tail bound, or the hard cap.
fn quantile(mean: f64, cap: u128, bits: f64) -> u128 {
    let mut low = 0;
    let mut high = cap;
    while low < high {
        let limit = low + (high - low) / 2;
        // Exceeding an integer limit means reaching at least limit + 1.
        if tail_exponent(mean, (limit + 1) as f64) >= bits * std::f64::consts::LN_2 {
            high = limit;
        } else {
            low = limit + 1;
        }
    }
    low
}

/// Probability that one depth-specific prefix contains a leaf of a corpus.
fn occupied_probability(messages: u64, depth: usize) -> f64 {
    let probability = 256f64.powi(-(depth as i32));
    if depth == 0 {
        return f64::from(messages > 0);
    }
    -((messages as f64) * (-probability).ln_1p()).exp_m1()
}

/// Maximum occupied prefixes: at most one per leaf and at most every prefix.
fn occupied_cap(messages: u64, depth: usize) -> u128 {
    let prefixes = 1u128.checked_shl(8 * depth as u32).unwrap_or(u128::MAX);
    u128::from(messages).min(prefixes)
}

/// Independently calculate occupied, disputed, child, and stage limits.
fn reference(a: u64, b: u64, depth: usize) -> [u128; 4] {
    let larger = a.max(b);
    let disputed = |depth| {
        let slots = 256f64.powi(depth as i32);
        let mean = slots * occupied_probability(a, depth) * occupied_probability(b, depth);
        quantile(mean, occupied_cap(a.min(b), depth), TAIL_BITS)
    };
    let children = |depth| {
        let bits = TAIL_BITS + 8.0 * depth as f64;
        let slots_mean = FAN as f64 * occupied_probability(larger, depth + 1);
        let leaves_mean = larger as f64 * 256f64.powi(-(depth as i32));
        quantile(slots_mean, FAN as u128, bits).min(quantile(
            leaves_mean,
            u128::from(larger).min(FAN as u128),
            bits,
        ))
    };
    let stage = match depth {
        _ if larger == 0 => 0,
        0 => 0,
        1 => 1,
        _ => occupied_cap(larger, depth - 1).min(disputed(depth - 2) * children(depth - 2)),
    };
    [
        occupied_cap(larger, depth),
        disputed(depth),
        children(depth),
        stage,
    ]
}

/// Read the actual production bounds at the same corpus sizes and depth.
fn shipped(a: u64, b: u64, depth: usize) -> [u128; 4] {
    let larger = u128::from(a.max(b));
    let pair = u128::from(a) * u128::from(b);
    [
        occupied(larger, depth),
        disputed(larger, pair, depth),
        children_quantile(larger, depth),
        stage_population(larger, pair, depth),
    ]
}

/// Compare limits with the numerical reference, reporting the first understatement.
fn compare(a: u64, b: u64, depth: usize, actual: [u128; 4]) -> Result<(), TestCaseError> {
    let names = ["occupied", "disputed", "children", "stage"];
    for ((name, actual), expected) in names.into_iter().zip(actual).zip(reference(a, b, depth)) {
        prop_assert!(
            actual >= expected,
            "{name} at ({a}, {b}), depth {depth}: shipped {actual}, reference {expected}"
        );
    }
    Ok(())
}

/// Draw small and large corpus sizes without concentrating almost every case near u64::MAX.
fn corpus_size() -> impl Strategy<Value = u64> {
    (0u32..=64, any::<u64>()).prop_map(|(bits, fill)| fill.checked_shr(64 - bits).unwrap_or(0))
}

/// Tail arithmetic retains precision near large means and for very small means.
#[test]
fn numerical_tail_retains_precision() {
    // Evaluated independently with 100-digit decimal arithmetic, starting
    // from these exact f64 inputs. The near-mean cases exercise cancellation.
    for (mean, count, expected) in [
        (1.0, 18.0, 35.02669164213096),
        (1e18, 1.0000000082462113e18, 34.0000005395607),
        (1e18, 1.0005e18, 124979171873.43802),
        (1e-24, 1.0, 54.262042231857095),
    ] {
        let actual = tail_exponent(mean, count);
        assert!((actual - expected).abs() <= expected * 1e-12);
    }
}

/// Boundaries of integer arithmetic remain conservative at every tree depth.
#[test]
fn envelope_boundaries_match_the_numerical_reference() {
    for bits in 0..=64 {
        let power = 1u64.checked_shl(bits).unwrap_or(u64::MAX);
        for a in [power - 1, power, power.saturating_add(1)] {
            for b in [0, 1, a / 256, a, u64::MAX] {
                for depth in 0..=KEY_DEPTH {
                    compare(a, b, depth, shipped(a, b, depth)).unwrap();
                }
            }
        }
    }
}

proptest! {
    /// Production limits cover the reference across corpus scales, asymmetry, and depth.
    #[test]
    fn envelopes_cover_the_numerical_reference(
        a in corpus_size(), b in corpus_size(), depth in 0usize..=KEY_DEPTH,
    ) {
        compare(a, b, depth, shipped(a, b, depth))?;
    }
}

/// An understated disputed population is rejected by the same comparison as production.
#[test]
fn an_understated_envelope_fails_the_comparison() {
    let (a, b, depth) = (2047, 2047, 2);
    let mut limits = shipped(a, b, depth);
    // Reducing the production Bernstein exponent from 34 to 20 yields 134
    // here; the independent calculation requires 136. A modest understatement
    // must fail, not just an obviously empty window.
    limits[1] = 134;
    assert!(compare(a, b, depth, limits).is_err());
}
