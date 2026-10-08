//! REVIEWER SCRATCH: heap of min_ticks in the prefix-spill regime; never committed.
#![cfg(feature = "meter")]

use before::testing::meter::reviewer_probe::{min_ticks_inline, stepped_spine};
use before::Count;
use num_bigint::BigUint;
use peak_alloc::PeakAlloc;

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

fn peak_heap<T>(f: impl FnOnce() -> T) -> (usize, T) {
    HEAP.reset_peak_usage();
    let baseline = HEAP.current_usage();
    let value = f();
    let peak = HEAP.peak_usage().saturating_sub(baseline);
    (peak, value)
}

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

fn report(label: &str, b: usize, wide: usize, unit: usize, inline: Option<usize>) {
    let encoded = stepped_spine(b, wide, unit);
    let v = encoded.version();
    let bytes = v.encode().len();
    let expected = count_of(&((BigUint::from(wide) << b) + BigUint::from(unit + 1)));
    let (peak, got) = match inline {
        None => peak_heap(|| v.min_ticks()),
        Some(n) => peak_heap(|| min_ticks_inline(&v, n)),
    };
    assert_eq!(got, expected, "{label} value");
    let ceiling = 1024 + 20 * bytes;
    println!(
        "PROBE {label} b={b} wide={wide} unit={unit} inline={inline:?} bytes={bytes} peak={peak} \
         per_byte={:.2} ceiling={ceiling} {}",
        peak as f64 / bytes as f64,
        if peak > ceiling { "RED" } else { "green" }
    );
}

#[test]
fn reviewer_spill_regime_small() {
    for d in [600usize, 2_400, 9_600, 38_400, 153_600] {
        report("WS", 31, d - 1, 0, None);
        report("WS", 31, d - 1, 0, Some(1));
    }
    for d in [8_000usize, 32_000, 128_000, 512_000] {
        report("JR", 40, 1, d, None);
        report("JR", 40, 1, d, Some(1));
        report("JR-after-64", 31, 64, d, Some(64));
    }
}

#[test]
#[ignore]
fn reviewer_spill_regime_real_scale() {
    let wide = (1usize << 23) + 16;
    report("REAL", 31, wide, 0, None);
    report("REAL", 31, wide, 1 << 25, None);
}

#[cfg(feature = "touch-meter")]
#[test]
fn reviewer_phase_attribution() {
    use before::testing::meter::registry::Shape;
    use before::testing::meter::reviewer_probe::{phases, reset_phases, REPLACE_LIVE};
    use std::sync::atomic::Ordering::Relaxed;
    let shapes: Vec<(&str, before::testing::meter::Encoding)> = vec![
        (
            "hoisted-window",
            Shape::HoistedWindow.build3(12, 40, 81_920),
        ),
        ("plateau-puncture", Shape::PlateauPuncture.build2(384, 384)),
        ("reveal-hifloor", Shape::RevealCombHifloor.build2(500, 500)),
        ("reveal-comb", Shape::RevealComb.build2(500, 500)),
        ("seam_plunge k1024", Shape::SeamPlunge.build2(1024, 5)),
        (
            "seam_plunge_control k1024",
            Shape::SeamPlungeControl.build2(1024, 5),
        ),
        ("seam_stop k1024", Shape::SeamStop.build1(1024)),
        (
            "seam_stop_control k1024",
            Shape::SeamStopControl.build1(1024),
        ),
        ("wide-step-spine", Shape::WideStepSpine.build2(31, 600)),
        ("jump-rising-spine", Shape::JumpRisingSpine.build2(40, 8000)),
    ];
    for (name, encoded) in shapes {
        let v = encoded.version();
        let bytes = v.encode().len();
        for replace in [false, true] {
            REPLACE_LIVE.store(replace, Relaxed);
            reset_phases();
            suanpan::touch_meter::reset();
            let ticks = v.min_ticks();
            let total = suanpan::touch_meter::touches();
            let [re, comps, read, add, adv, wf, reset, settle] = phases();
            let rest = total - read - add - adv - wf - reset - settle;
            println!(
                "PHASE {name} replace_live={replace} bytes={bytes} total={total} ({:.2}/B) reanchors={re} components={comps} \
                 leaf_read={read} offset_add={add} advance={adv} width_freeze={wf} reanchor_reset={reset} settle={settle} \
                 other(minima)={rest} ticks_bits={}",
                total as f64 / bytes as f64,
                ticks.to_string().len(),
            );
        }
    }
    REPLACE_LIVE.store(false, Relaxed);
}
