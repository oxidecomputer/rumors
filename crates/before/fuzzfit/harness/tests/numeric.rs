//! Fuel checks for public paths whose work does not reach the board counters.
//!
//! These deterministic width and arity sweeps use the same guest, fuel meter,
//! and regression fitter as the generated-program suite. Their limits are
//! growth limits, not fitted bands: linear paths allow the board's usual slope
//! tolerance, decimal conversion must stay below quadratic, and fixed rank
//! precision must stay independent of the unprinted suffix. These finite
//! sweeps detect regressions; they do not prove the bounds for every size.

use before::testing::meter::board::MAX_SCALING_EXPONENT;
use before::testing::meter::registry::Shape;
use before::{Rank, Ticks, Version};
use fuzzfit_harness::fit::fit;
use fuzzfit_harness::wasm::Guest;

/// Judge a geometric sweep with the harness's existing bucket-median fit.
fn check_growth(name: &str, samples: &[(u64, u64)], ceiling: f64) {
    let fit = fit(samples).expect("a width sweep has several samples");
    eprintln!("{name}: slope {:.4}, samples {samples:?}", fit.slope);
    assert!(
        !fit.constant,
        "{name}: the sweep must span enough sizes to fit"
    );
    assert!(
        fit.slope < ceiling,
        "{name}: slope {} exceeds {ceiling}",
        fit.slope
    );
}

/// Call a successful kernel and return its measured fuel.
fn measure(guest: &mut Guest, kernel: &str, args: &[u32]) -> u64 {
    let result = guest.call(kernel, args);
    assert_eq!(result.ret, 0, "{kernel}");
    result.fuel
}

/// Load one canonical version without charging its decode to another path.
fn load_version(guest: &mut Guest, dst: u32, version: &Version) {
    guest.stage_write(&version.encode());
    measure(guest, "ff_version_decode", &[dst]);
}

/// Load `2^bits - 1` as a count through the existing wide-leaf fixture.
fn load_wide_count(guest: &mut Guest, dst: u32, bits: usize) -> Ticks {
    let version = Shape::Hugeleaf.build1(bits).version();
    load_version(guest, dst, &version);
    measure(guest, "ff_ticks_from_version", &[dst, dst]);
    version.min_ticks()
}

/// Read a count through decimal rendering for the native/wasm differential.
fn assert_count(guest: &mut Guest, src: u32, expected: &Ticks) {
    measure(guest, "ff_ticks_display", &[src]);
    assert_eq!(guest.stage_read(), expected.to_string().as_bytes());
}

/// Parsing scales linearly for wide integers, wide fractions, and compact
/// numerators with long zero prefixes; rejecting a last-byte error does too.
#[test]
fn rank_parsing_scales_with_text_length() {
    for form in 0..3 {
        let mut accepted = Vec::new();
        let mut rejected = Vec::new();
        for power in 7..=17 {
            let width = 1usize << power;
            let text = match form {
                0 => "10".repeat(width / 2),
                1 => format!("0.{}1", "10".repeat(width / 2)),
                _ => format!("0.{}1", "0".repeat(width)),
            };
            let expected: Rank = text.parse().unwrap();
            let mut guest = Guest::new();
            guest.stage_write(text.as_bytes());
            let fuel = measure(&mut guest, "ff_rank_parse", &[0]);
            measure(&mut guest, "ff_rank_encode", &[0]);
            assert_eq!(guest.stage_read(), expected.encode());
            accepted.push((text.len() as u64, fuel));

            let mut invalid = text.into_bytes();
            *invalid.last_mut().unwrap() = b'x';
            guest.stage_write(&invalid);
            let result = guest.call("ff_rank_parse", &[1]);
            assert_eq!(result.ret, -3);
            rejected.push((invalid.len() as u64, result.fuel));
        }
        check_growth(
            &format!("rank parse form {form}"),
            &accepted,
            MAX_SCALING_EXPONENT,
        );
        check_growth(
            &format!("rank reject form {form}"),
            &rejected,
            MAX_SCALING_EXPONENT,
        );
    }
}

/// Rank formatting prices the retained prefix and padding, while holding
/// precision fixed makes its fuel independent of a growing input suffix.
#[test]
fn rank_precision_prices_only_the_text_written() {
    let mut prefix = Vec::new();
    let mut padding = Vec::new();
    let mut fixed_precision = Vec::new();
    // Keep formatter arguments within the range accepted by std while the
    // unprinted input grows through 128 KiB.
    for power in 7..=15 {
        let width = 1u32 << power;
        let text = format!("0.{}1", "10".repeat(width as usize * 2));
        let rank: Rank = text.parse().unwrap();
        let mut guest = Guest::new();
        guest.stage_write(text.as_bytes());
        measure(&mut guest, "ff_rank_parse", &[0]);

        for (precision, field_width, samples) in [
            (width, 0, &mut prefix),
            (8, width, &mut padding),
            (8, 0, &mut fixed_precision),
        ] {
            let fuel = measure(&mut guest, "ff_rank_format", &[0, precision, field_width]);
            let expected = format!(
                "{rank:width$.precision$}",
                width = field_width as usize,
                precision = precision as usize
            );
            assert_eq!(guest.stage_read(), expected.as_bytes());
            samples.push((width as u64, fuel));
        }
    }
    check_growth("rank retained prefix", &prefix, MAX_SCALING_EXPONENT);
    check_growth("rank padding", &padding, MAX_SCALING_EXPONENT);
    check_growth(
        "rank fixed precision",
        &fixed_precision,
        MAX_SCALING_EXPONENT - 1.0,
    );
}

/// Borrowed addition and in-place addition remain linear as the count width
/// grows, including a carry through every limb of an all-ones count.
#[test]
fn ticks_addition_scales_with_numeric_width() {
    for kernel in ["ff_ticks_add", "ff_ticks_add_assign"] {
        for carry_only in [false, true] {
            let mut samples = Vec::new();
            for power in 7..=17 {
                let width = 1usize << power;
                let mut guest = Guest::new();
                let left = load_wide_count(&mut guest, 0, width);
                let right = if carry_only {
                    measure(&mut guest, "ff_ticks_from_u32", &[1, 1]);
                    Ticks::from(1u8)
                } else {
                    load_wide_count(&mut guest, 1, width)
                };
                let (args, result) = if kernel == "ff_ticks_add" {
                    (vec![2, 0, 1], 2)
                } else {
                    (vec![0, 1], 0)
                };
                let fuel = measure(&mut guest, kernel, &args);
                assert_count(&mut guest, result, &(&left + &right));
                samples.push((width as u64, fuel));
            }
            check_growth(
                &format!("{kernel}, carry only {carry_only}"),
                &samples,
                MAX_SCALING_EXPONENT,
            );
        }
    }
}

/// Sums price every summand, including zeroes, and avoid rescanning a wide
/// accumulator for each following narrow count. Both ownership forms run.
#[test]
fn ticks_sum_scales_with_total_content_and_arity() {
    for kernel in ["ff_ticks_sum", "ff_ticks_sum_owned"] {
        for wide_first in [None, Some(false), Some(true)] {
            let mut samples = Vec::new();
            for power in 7..=12 {
                let n = 1u32 << power;
                let mut guest = Guest::new();
                for reg in 0..n {
                    measure(
                        &mut guest,
                        "ff_ticks_from_u32",
                        &[reg, u32::from(wide_first.is_some())],
                    );
                }
                let (expected, size) = if let Some(first) = wide_first {
                    let width = n as usize * 64;
                    let wide = load_wide_count(&mut guest, if first { 0 } else { n - 1 }, width);
                    (&wide + Ticks::from(n - 1), width as u64 + n as u64)
                } else {
                    (Ticks::ZERO, n as u64)
                };
                let fuel = measure(&mut guest, kernel, &[n, 0, n]);
                assert_count(&mut guest, n, &expected);
                samples.push((size, fuel));
            }
            check_growth(
                &format!("{kernel}, wide first {wide_first:?}"),
                &samples,
                MAX_SCALING_EXPONENT,
            );
        }
    }
}

/// Decimal rendering stays below quadratic growth across wide counts and
/// agrees byte-for-byte with native rendering at every measured width.
#[test]
fn ticks_decimal_rendering_stays_subquadratic() {
    let mut samples = Vec::new();
    for power in 10..=18 {
        let width = 1usize << power;
        let mut guest = Guest::new();
        let ticks = load_wide_count(&mut guest, 0, width);
        let fuel = measure(&mut guest, "ff_ticks_display", &[0]);
        assert_eq!(guest.stage_read(), ticks.to_string().as_bytes());
        samples.push((width as u64, fuel));
    }
    check_growth("ticks decimal", &samples, 2.0);
}

/// Shape combination prices independently growing arity and operand size.
/// The registered staggered population gives the combiner distinct boundaries
/// to refine as each dimension grows.
#[test]
fn shape_combine_scales_in_both_dimensions() {
    for grow_arity in [false, true] {
        let mut samples = Vec::new();
        for power in 1..=8 {
            let (arity, blocks) = if grow_arity {
                (1 << power, 4)
            } else {
                (8, 1 << power)
            };
            let (encodings, _) = Shape::StaggerPopulation.population(arity, blocks);
            let versions: Vec<Version> = encodings.iter().map(|input| input.version()).collect();
            let input_bits: usize = versions
                .iter()
                .map(|version| version.encode().len() * 8)
                .sum();
            let mut guest = Guest::new();
            for (reg, version) in versions.iter().enumerate() {
                load_version(&mut guest, reg as u32, version);
            }
            let result = guest.call("ff_shape_combine", &[0, arity as u32]);
            assert!(result.ret >= 0, "shape combine rejected arity {arity}");
            samples.push(((arity * input_bits) as u64, result.fuel));
        }
        check_growth(
            &format!("shape combine, growing arity {grow_arity}"),
            &samples,
            MAX_SCALING_EXPONENT,
        );
    }
}
