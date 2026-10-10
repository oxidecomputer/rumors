//! Focused resource checks for claims the amplification board cannot express.
//!
//! The global board owns ordinary input-scaling and peak-heap coverage across
//! the public operation surface. This suite retains only independent axes: an
//! operation argument varied while operands stay fixed, a control-paired
//! marginal, an implementation counter with no board currency, an exact
//! early-exit relationship, or an exact heap parity between entry points that
//! compute the same value.

use before::testing::meter;
use before::testing::meter::registry::Shape;
use before::{Count, Party, Version};
use num_bigint::BigUint;

/// Measure peak heap above the storage already live at entry.
///
/// The reading counts only this thread's allocator requests, so allocations
/// the test harness makes on its own threads cannot enter it.
fn peak_heap<T>(f: impl FnOnce() -> T) -> (usize, T) {
    let (stats, value) = alloc_meter::measure(f);
    (stats.peak_bytes, value)
}

/// Decode a generated party.
fn party_of(encoded: &meter::Encoding) -> Party {
    Party::decode(&encoded.bytes[..]).expect("the generated party is canonical")
}

/// Convert a generated event tree into its stored version representation.
fn version_of(encoded: &meter::Encoding) -> Version {
    encoded.version()
}

/// Build an expected minimum tick count from most-significant limb to least.
///
/// `Count` deliberately exposes no big-integer backend. Repeated doubling and
/// addition reconstruct the same value through its public arithmetic surface;
/// resource tests call this helper only while preparing their operands.
fn min_ticks_from_big(value: &BigUint) -> Count {
    let mut digits = value.iter_u64_digits().rev();
    let Some(first) = digits.next() else {
        return Count::ZERO;
    };
    let mut ticks = Count::from(first);
    for digit in digits {
        for _ in 0..64 {
            ticks += ticks.clone();
        }
        ticks += Count::from(digit);
    }
    ticks
}

/// Construct `2^exponent` as an unbounded tick count.
fn power_of_two(exponent: usize) -> Count {
    let mut ticks = Count::from(1u8);
    for _ in 0..exponent {
        ticks += ticks.clone();
    }
    ticks
}

/// Dense-spine depth used while varying only the `ticks` count.
const DENSE_DEPTH: usize = 125_000;
/// Ordinary count at the low end of the `ticks` argument band.
const TICKS_POINT_LO: u64 = 512;
/// Shared magnitude and depth of the wide tick operands.
const TICK_CROSS_SCALE: usize = 4_000;
/// Magnitude of the paired jump-comb family.
const JUMP_PAIR_MAGNITUDE_BITS: usize = 512;

#[path = "meter/answer_embedded_product.rs"]
mod answer_embedded_product;
#[cfg(feature = "touch-meter")]
#[path = "meter/deferred_wide_arming.rs"]
mod deferred_wide_arming;
#[cfg(feature = "scan-meter")]
#[path = "meter/duplicate_runs.rs"]
mod duplicate_runs;
#[cfg(feature = "touch-meter")]
#[path = "meter/eq_early_exit.rs"]
mod eq_early_exit;
#[cfg(feature = "touch-meter")]
#[path = "meter/hoisted_window.rs"]
mod hoisted_window;
#[cfg(feature = "scan-meter")]
#[path = "meter/identity_fast_paths.rs"]
mod identity_fast_paths;
#[path = "meter/lattice_clones.rs"]
mod lattice_clones;
#[cfg(feature = "scan-meter")]
#[path = "meter/placement.rs"]
mod placement;
#[cfg(feature = "touch-meter")]
#[path = "meter/settle_flatness.rs"]
mod settle_flatness;
#[cfg(feature = "scan-meter")]
#[path = "meter/span.rs"]
mod span;
#[cfg(feature = "scan-meter")]
#[path = "meter/span_codec.rs"]
mod span_codec;
#[path = "meter/tick_counts.rs"]
mod tick_counts;
#[cfg(feature = "touch-meter")]
#[path = "meter/version_scaling.rs"]
mod version_scaling;
#[cfg(feature = "touch-meter")]
#[path = "meter/width_circulation_cost.rs"]
mod width_circulation_cost;
