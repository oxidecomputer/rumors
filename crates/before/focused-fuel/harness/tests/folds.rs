//! The balanced folds' logarithmic factor, held alive at the version, span,
//! and clock fold entry points.
//!
//! Each entry point's `# Complexity` section documents `O(D log k)` for a
//! fold of `k` operands totalling `D` bytes. On the staggered populations,
//! whose balanced merges stay as large as the inputs they carry, the factor
//! shows; if a linear fold landed behind an entry point, these claims fail,
//! and the rustdoc and the claim move together.

use focused_fuel_harness::{assert_grows, measure_per, Probe};

/// The population sizes, a fourfold step apart.
const SIZES: [u64; 2] = [256, 1_024];

/// Each version, span, and clock fold entry point's fuel grows faster than
/// its input across a fourfold growth of the population, by at least half
/// the documented logarithmic factor's excess over linear growth.
///
/// A fold costing `D log k` grows by the population's byte growth times
/// `log 1024 / log 256 = 1.25`; a linear fold by the byte growth alone. The
/// floor sits midway, at the byte growth times 1.125, derived from the two
/// models rather than from any reading. Form: growth floor (F-grow).
#[test]
fn version_span_and_clock_folds_keep_their_log_factor() {
    let [small, large] = SIZES;
    let log_ratio = (large as f64).ln() / (small as f64).ln();
    for probe in [
        Probe::StaggerJoinAll,
        Probe::StaggerMeetAll,
        Probe::StaggerSpanAll,
        Probe::StaggerClockJoinAll,
    ] {
        let (fuel_small, bytes_small) = measure_per(probe, small, 0);
        let (fuel_large, bytes_large) = measure_per(probe, large, 0);
        let byte_growth = bytes_large as f64 / bytes_small as f64;
        assert_grows(
            &format!("{probe:?} across a fourfold population"),
            fuel_small,
            fuel_large,
            byte_growth * (1.0 + (log_ratio - 1.0) / 2.0),
        );
    }
}
