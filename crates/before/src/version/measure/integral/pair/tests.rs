//! The integrands' coefficients, over every sign of the difference.

use core::cmp::Ordering;

use super::{Absolute, Integrand, NegativePart, Signed};

/// Every sign change of every integrand calls for a nonnegative correction
/// `(sigma' - sigma) * D'`.
///
/// The coefficient is zero or has the sign of the new difference `D'`, and
/// magnitude at most two, as `Integrator::jump` requires when it applies the
/// correction as a shift of zero or one bit. Equivalently, each `sigma` lies
/// in `{-1, 0, 1}` and is monotone over `Less`, `Equal`, `Greater`, which the
/// test checks directly too.
#[test]
#[allow(clippy::type_complexity)]
fn every_sign_change_corrects_nonnegatively() {
    let signs = [Ordering::Less, Ordering::Equal, Ordering::Greater];
    let integrands: [(&str, fn(Ordering) -> i8); 3] = [
        ("Absolute", Absolute::sigma),
        ("NegativePart", NegativePart::sigma),
        ("Signed", Signed::sigma),
    ];
    for (integrand, sigma) in integrands {
        for pair in signs.windows(2) {
            assert!(sigma(pair[0]) <= sigma(pair[1]), "{integrand} is monotone");
        }
        for old in signs {
            assert!((-1..=1).contains(&sigma(old)));
            // A new difference of zero needs no correction: `D'` is zero.
            for new in [Ordering::Less, Ordering::Greater] {
                let coefficient = sigma(new) - sigma(old);
                assert!(coefficient.abs() <= 2);
                assert!(
                    coefficient == 0 || (coefficient < 0) == (new == Ordering::Less),
                    "{integrand} from {old:?} to {new:?}: coefficient {coefficient}"
                );
            }
        }
    }
}
