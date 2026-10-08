
/// Probes for contract questions (explore branch only).
mod probes {
    use super::super::Accumulator;
    use core::cmp::Ordering;

    /// A mathematically zero accumulator stored as digits `[-2^32, 1]`.
    fn redundant_zero() -> Accumulator {
        let mut acc = Accumulator::new();
        acc.add_shifted_limbs(32, [1, 0]);
        acc -= 1_u64 << 32;
        assert!(!acc.is_known_zero(), "the probe needs a redundant zero");
        let (sign, limbs) = acc.signed_magnitude();
        assert_eq!(sign, Ordering::Equal);
        assert!(limbs.as_ref().is_empty());
        acc
    }

    /// Outcome of a closure: Ok or the panic message.
    fn outcome(f: impl FnOnce() + std::panic::UnwindSafe) -> String {
        match std::panic::catch_unwind(f) {
            Ok(()) => "returned".to_string(),
            Err(payload) => {
                if let Some(s) = payload.downcast_ref::<&str>() {
                    format!("panicked: {s}")
                } else if let Some(s) = payload.downcast_ref::<String>() {
                    format!("panicked: {s}")
                } else {
                    "panicked: <non-string>".to_string()
                }
            }
        }
    }

    /// Observe whether shifting zero depends on the stored representation.
    #[test]
    fn l7_probe_zero_shift_history() {
        let huge = 32 * (usize::MAX as u64 - 1);
        let known = outcome(|| {
            let mut acc = Accumulator::new();
            acc += 0_i64;
            acc <<= huge;
        });
        let redundant_assign = outcome(|| {
            let mut acc = redundant_zero();
            acc <<= huge;
        });
        let redundant_shl = outcome(|| {
            let acc = redundant_zero();
            let _ = acc << huge;
        });
        let redundant_operand = outcome(|| {
            let mut acc = Accumulator::new();
            acc.add_shifted(huge, &redundant_zero());
        });
        let normalized_first = outcome(|| {
            let mut acc = redundant_zero();
            acc.normalize();
            acc <<= huge;
        });
        eprintln!("PROBE zero-shift known-zero <<= huge: {known}");
        eprintln!("PROBE zero-shift redundant-zero <<= huge: {redundant_assign}");
        eprintln!("PROBE zero-shift redundant-zero << huge: {redundant_shl}");
        eprintln!("PROBE zero-shift add_shifted(huge, redundant-zero): {redundant_operand}");
        eprintln!("PROBE zero-shift normalized redundant-zero <<= huge: {normalized_first}");
    }

    /// Observe `reserve_digits` at the largest request.
    #[test]
    fn l7_probe_reserve_digits_extreme() {
        let result = outcome(|| {
            let mut acc = Accumulator::new();
            acc.reserve_digits(usize::MAX);
        });
        eprintln!("PROBE reserve_digits(usize::MAX): {result}");
    }
}
