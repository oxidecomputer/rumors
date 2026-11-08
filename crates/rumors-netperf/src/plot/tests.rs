use proptest::prelude::*;

use super::*;

proptest! {
    /// A tick label is the value rounded to three significant figures, or
    /// to a whole number at 100 and above: it parses back to within half a
    /// unit of its last printed place.
    #[test]
    fn ticks_round_to_three_figures(mantissa in 1.0f64..10.0, exponent in -6i32..9) {
        let value = mantissa * 10f64.powi(exponent);
        let label = tick(value);
        let parsed: f64 = label.parse().unwrap();
        let place = 10f64.powi((exponent - 2).min(0));
        prop_assert!((parsed - value).abs() <= 0.5 * place + 1e-9 * value, "{value} -> {label}");
    }
}

/// The rounding noise a logarithmic axis produces prints cleanly.
#[test]
fn rounding_noise_prints_cleanly() {
    assert_eq!(tick(0.6000000000000001), "0.6");
    assert_eq!(tick(250.0), "250");
    assert_eq!(tick(0.1), "0.1");
}
