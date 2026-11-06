use proptest::prelude::*;

use super::*;

proptest! {
    /// A band admits concurrent readings exactly when their median lies
    /// within one solo range of the solo median: the solo readings
    /// themselves always pass, and a shift beyond the range fails.
    #[test]
    fn band_admits_within_one_range(
        solo in proptest::collection::vec(1.0f64..2.0, 2..10),
        shift in 0.0f64..3.0,
    ) {
        let band = Band::of(&solo);
        prop_assert!(band.admits(&solo));
        let shifted: Vec<f64> = solo.iter().map(|x| x + shift).collect();
        if shift < band.range - 1e-9 {
            prop_assert!(band.admits(&shifted));
        } else if shift > band.range + 1e-9 {
            prop_assert!(!band.admits(&shifted));
        }
    }
}
