//! FIXER SCRATCH: the reviewer's real-scale min_ticks heap probe; never committed.
#![cfg(feature = "meter")]

use before::testing::meter::fixer_probe::stepped_spine;
use before::Count;
use num_bigint::BigUint;
use peak_alloc::PeakAlloc;

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

fn count_of(value: &BigUint) -> Count {
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

fn report(label: &str, b: usize, wide: usize, unit: usize) {
    let encoded = stepped_spine(b, wide, unit);
    let v = encoded.version();
    let bytes = v.encode().len();
    let expected = count_of(&((BigUint::from(wide) << b) + BigUint::from(unit + 1)));
    HEAP.reset_peak_usage();
    let baseline = HEAP.current_usage();
    let got = v.min_ticks();
    let peak = HEAP.peak_usage().saturating_sub(baseline);
    assert_eq!(got, expected, "{label} value");
    let ceiling = 1024 + 20 * bytes;
    println!(
        "PROBE {label} b={b} wide={wide} unit={unit} bytes={bytes} peak={peak} per_byte={:.2} ceiling={ceiling} {}",
        peak as f64 / bytes as f64,
        if peak > ceiling { "RED" } else { "green" }
    );
}

#[test]
fn fixer_probe_small() {
    for d in [600usize, 2_400, 9_600, 38_400, 153_600] {
        report("WS", 31, d - 1, 0);
    }
    for d in [8_000usize, 32_000, 128_000, 512_000] {
        report("JR", 40, 1, d);
        report("JR-after-64", 31, 64, d);
    }
}

#[test]
#[ignore]
fn fixer_probe_real_scale() {
    let wide = (1usize << 23) + 16;
    report("REAL", 31, wide, 0);
    report("REAL", 31, wide, 1 << 25);
}
