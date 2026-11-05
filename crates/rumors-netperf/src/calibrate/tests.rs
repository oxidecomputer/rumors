use proptest::prelude::*;

use super::*;

/// Solo readings as calibration takes them, or any count from three up,
/// each between 1 and 2.
fn solo_readings() -> impl Strategy<Value = Vec<f64>> {
    prop_oneof![
        proptest::collection::vec(1.0f64..2.0, SOLO_RUNS),
        proptest::collection::vec(1.0f64..2.0, 3..24),
    ]
}

/// A reading anywhere from far below to far above every solo reading.
fn stray_reading() -> impl Strategy<Value = f64> {
    prop_oneof![-1e9f64..1e9, Just(0.0), Just(1e300), Just(-1e300)]
}

/// The smallest and largest of `values`.
fn bounds(values: &[f64]) -> (f64, f64) {
    let low = values.iter().copied().fold(f64::INFINITY, f64::min);
    let high = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (low, high)
}

proptest! {
    /// A band admits concurrent readings exactly when their median lies
    /// within one spread of the solo median: the solo readings themselves
    /// always pass, and a shift beyond the spread fails while a shift
    /// within it passes.
    #[test]
    fn band_admits_within_one_spread(solo in solo_readings(), shift in 0.0f64..3.0) {
        let band = Band::of(&solo);
        prop_assert!(band.admits(&solo));
        let shifted: Vec<f64> = solo.iter().map(|x| x + shift).collect();
        if shift < band.spread - 1e-9 {
            prop_assert!(band.admits(&shifted));
        } else if shift > band.spread + 1e-9 {
            prop_assert!(!band.admits(&shifted));
        }
    }

    /// One solo reading, however far it strays, cannot widen the band past
    /// the full range of the other readings, nor carry its median outside
    /// them.
    ///
    /// The stray reading replaces any one of the readings, so the band a
    /// single slow session or dipped transfer leaves is never wider than the
    /// readings without it span.
    #[test]
    fn one_solo_reading_cannot_widen_the_band(
        solo in solo_readings(),
        chosen in any::<prop::sample::Index>(),
        stray in stray_reading(),
    ) {
        let index = chosen.index(solo.len());
        let mut others = solo.clone();
        others.remove(index);
        let (low, high) = bounds(&others);
        let mut strayed = solo;
        strayed[index] = stray;
        let band = Band::of(&strayed);
        prop_assert!(band.spread <= high - low, "{band:?} beyond {low}..={high}");
        prop_assert!((low..=high).contains(&band.median), "{band:?} beyond {low}..={high}");
    }

    /// One concurrent reading, however far it strays, cannot fail a level
    /// whose other readings all lie within the band.
    ///
    /// Levels are drawn with any count of readings from three up, the
    /// fewest [`LEVEL_READINGS`] may be, so the property covers every level
    /// calibration runs.
    #[test]
    fn one_level_reading_cannot_fail_a_level(
        solo in solo_readings(),
        offsets in proptest::collection::vec(-0.99f64..=0.99, 3..24),
        chosen in any::<prop::sample::Index>(),
        stray in stray_reading(),
    ) {
        let band = Band::of(&solo);
        let mut level: Vec<f64> = offsets
            .iter()
            .map(|offset| band.median + offset * band.spread)
            .collect();
        prop_assert!(band.admits(&level));
        let index = chosen.index(level.len());
        level[index] = stray;
        prop_assert!(band.admits(&level), "{band:?} refused {level:?}");
    }
}

/// Each job count from 1 to 256 runs the fewest rounds that give its level
/// at least [`LEVEL_READINGS`] readings.
#[test]
fn rounds_reach_the_level_readings_and_no_more() {
    for jobs in 1..=256 {
        let rounds = rounds(jobs);
        assert!(
            rounds * jobs >= LEVEL_READINGS,
            "{jobs} jobs, {rounds} rounds"
        );
        assert!(
            (rounds - 1) * jobs < LEVEL_READINGS,
            "{jobs} jobs, {rounds} rounds"
        );
    }
}
