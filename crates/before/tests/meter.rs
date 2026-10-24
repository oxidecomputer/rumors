//! Focused resource checks for claims the amplification board cannot express.
//!
//! The global board owns ordinary input-scaling and peak-heap coverage across
//! the public operation surface, and the `focused-fuel` harness judges the
//! time claims the board cannot vary. This suite retains the memory and
//! counter claims beside them: exact heap parities between entry points that
//! compute the same value, heap and allocation ceilings at the shapes that
//! exercise a representation choice, and the retained densification counter.

use before::testing::meter;
use before::{Count, Version};
use num_bigint::BigUint;

/// Measure peak heap above the storage already live at entry.
///
/// The reading counts only this thread's allocator requests, so allocations
/// the test harness makes on its own threads cannot enter it.
fn peak_heap<T>(f: impl FnOnce() -> T) -> (usize, T) {
    let (stats, value) = alloc_meter::measure(f);
    (stats.peak_bytes, value)
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

#[path = "meter/answer_embedded_product.rs"]
mod answer_embedded_product;
#[path = "meter/deferred_wide_arming.rs"]
mod deferred_wide_arming;
#[path = "meter/encode_allocation.rs"]
mod encode_allocation;
#[path = "meter/hoisted_window.rs"]
mod hoisted_window;
#[path = "meter/identity_fast_paths.rs"]
mod identity_fast_paths;
#[path = "meter/lattice_clones.rs"]
mod lattice_clones;
#[path = "meter/version_scaling.rs"]
mod version_scaling;
