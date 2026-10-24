//! Directed comparisons stop once their requested ordering is impossible.

use focused_fuel_harness::{assert_equal, assert_less, measure, Probe};

/// Tooth counts of the comb `C(1024, teeth)`, an eightfold range of tail
/// lengths behind a first interval that decides every directed question.
const TEETH: [u64; 3] = [512, 1_024, 4_096];

/// A directed comparison, and the point-span and query forms that ask one
/// direction, stop at the first interval once that ordering is impossible,
/// as does the directed comparison through a projection.
///
/// Form: equal fuel (F-eq) over the comb's tooth count, for each directed
/// form.
#[test]
fn directed_comparisons_stop_at_the_deciding_interval() {
    for probe in [
        Probe::DirectedLe,
        Probe::DirectedPointDominance,
        Probe::DirectedFloorContains,
        Probe::DirectedProjectedLe,
    ] {
        assert_equal(probe, &TEETH);
    }
}

/// The full comparison continues past the deciding interval, far enough to
/// tell `Greater` from concurrent, so it costs more than the directed form
/// and grows with the comb's tail, directly and through a projection.
///
/// Form: strict saving (F-lt) of the directed form under the full relation
/// at every tooth count, and of each tooth count's full relation under the
/// next.
#[test]
fn full_comparisons_walk_the_whole_tail() {
    for (directed, full) in [
        (Probe::DirectedLe, Probe::DirectedFullCompare),
        (Probe::DirectedProjectedLe, Probe::DirectedProjectedCompare),
    ] {
        let mut previous: Option<(u64, u64)> = None;
        for teeth in TEETH {
            let reading = measure(full, teeth, 0);
            assert_less(
                &format!("{directed:?} vs {full:?} at {teeth} teeth"),
                measure(directed, teeth, 0),
                reading,
            );
            if let Some((fewer, earlier)) = previous {
                assert_less(
                    &format!("{full:?} at {fewer} vs {teeth} teeth"),
                    earlier,
                    reading,
                );
            }
            previous = Some((teeth, reading));
        }
    }
}
