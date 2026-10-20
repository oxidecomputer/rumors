//! Scratch probe for the wide-boundary heap fix; never lands.

use num_bigint::BigUint;

use super::{ev_leaf, wide_step_pairs, Encoding};
use crate::bits::BitsWriter;
use crate::Party;

/// A right spine whose every nested minimum rises by `2^e`.
fn wide_steps(e: usize, d: usize) -> Encoding {
    let wide = BigUint::from(1u8) << e;
    let mut bits = BitsWriter::new();
    bits.push(true);
    bits.write_gamma(&BigUint::ZERO);
    for _ in 2..=d {
        ev_leaf(&mut bits, 0);
        bits.push(true);
        bits.write_gamma(&wide);
    }
    ev_leaf(&mut bits, 0);
    ev_leaf(&mut bits, 1);
    Encoding::from_bits(bits)
}

/// Print one line of readings for `min_ticks` and `tick` on `encoding`.
fn measure(name: &str, encoding: &Encoding) {
    let version = encoding.version();
    let bytes = version.encode().len();
    let (stats, _) = alloc_meter::measure(|| version.min_ticks());
    let mut ticked = version.clone();
    let (tick_stats, ()) = alloc_meter::measure(|| ticked.tick(&Party::seed()));
    println!(
        "RMW {name} bytes={bytes} min_ticks_peak={} per_byte={:.2} tick_peak={} tick_per_byte={:.2}",
        stats.peak_bytes,
        stats.peak_bytes as f64 / bytes as f64,
        tick_stats.peak_bytes,
        tick_stats.peak_bytes as f64 / bytes as f64,
    );
}

/// Prints the ladders; asserts nothing.
#[test]
fn rmw_probe_ladders() {
    println!(
        "RMW size_of Accumulator = {}",
        core::mem::size_of::<suanpan::Accumulator>()
    );
    for e in [
        63, 64, 65, 72, 80, 95, 96, 97, 112, 127, 128, 129, 140, 160, 192, 224, 256, 320, 512,
    ] {
        for d in [1_026, 1_100, 2_048, 4_400] {
            measure(&format!("steps e={e} d={d}"), &wide_steps(e, d));
        }
    }
    for d in [2_000, 2_100, 8_000, 8_400, 32_000] {
        measure(&format!("pairs e=65 d={d}"), &wide_step_pairs(65, d));
    }
}
