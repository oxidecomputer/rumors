//! The balanced folds' logarithmic factor, held alive at every public fold
//! entry point.
//!
//! Each entry point's `# Complexity` section documents `O(D log k)` for a
//! fold of `k` operands totalling `D` bytes. On the staggered populations,
//! whose balanced merges stay as large as the inputs they carry, the factor
//! shows; if a linear fold landed behind an entry point, these claims fail,
//! and the rustdoc and the claim move together.

use focused_fuel_harness::{assert_flat, assert_grows, measure_per, Probe};

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

/// `span_all` over a population sharing one deep skeleton, every operand
/// full-size so meets never shrink, stays flat per byte at a fixed arity of
/// sixteen when the skeleton doubles.
///
/// This is the `D` leg of the folds' `O(D log k)` model, on the one
/// population shape the board's fold families leave out.
///
/// Form: flat per unit (F-lin) per encoded byte across the skeleton's
/// doubling.
#[test]
fn span_all_is_flat_on_a_shared_deep_skeleton() {
    assert_flat(
        "span_all over a shared skeleton per byte",
        measure_per(Probe::SharedSkeletonSpanAll, 0, 0),
        measure_per(Probe::SharedSkeletonSpanAll, 1, 0),
    );
}

/// `Party::join_all`'s fuel grows faster than its input across a fourfold
/// growth of the staggered ids, by at least half the excess over linear
/// growth that its own joins express.
///
/// The fold joins the ids through a balanced binary counter, and each join
/// walks both operands once, so its work is the bytes its joins read: over
/// every join, both operands' encoded sizes. A union of interleaved disjoint
/// ids spells fewer bytes than its operands together, because adjacent
/// blocks share their ancestors, so each level of the counter reads less than
/// the population's bytes and the factor shows more weakly than on versions,
/// whose merges stay as large as their inputs. The probe replays the
/// counter's joins outside the measured window to count the bytes they read.
/// A fold linear in its input grows by the population's byte growth; the
/// balanced fold grows by the growth of the bytes its joins read. The floor
/// sits midway between the two, derived from the ids' own joins rather than
/// from any reading. Its premise is that at least half of the fold's fuel
/// growth above linear is proportional to the bytes its joins read, so a
/// join kernel faster per byte read, with a larger fixed cost per join, could
/// fail it without any change in the fold's order. Form: growth floor
/// (F-grow).
#[test]
fn party_fold_keeps_its_log_factor() {
    let [small, large] = SIZES;
    let (fuel_small, bytes_small) = measure_per(Probe::StaggerPartyJoinAll, small, 0);
    let (fuel_large, bytes_large) = measure_per(Probe::StaggerPartyJoinAll, large, 0);
    let (_, read_small) = measure_per(Probe::StaggerPartyJoinAllMergeBytes, small, 0);
    let (_, read_large) = measure_per(Probe::StaggerPartyJoinAllMergeBytes, large, 0);
    let byte_growth = bytes_large as f64 / bytes_small as f64;
    let read_growth = read_large as f64 / read_small as f64;
    eprintln!(
        "MEASURED party fold: bytes {bytes_small} -> {bytes_large} (x{byte_growth:.3}), \
         joins read {read_small} -> {read_large} (x{read_growth:.3})"
    );
    assert!(
        read_growth > byte_growth,
        "the ids' joins read x{read_growth:.3} across the population's x{byte_growth:.3}: \
         they express no logarithmic factor to hold"
    );
    assert_grows(
        "Party::join_all across a fourfold population",
        fuel_small,
        fuel_large,
        byte_growth + (read_growth - byte_growth) / 2.0,
    );
}
