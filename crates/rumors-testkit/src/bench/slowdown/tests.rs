//! The profile estimate against the two simpler quantities it refines: the
//! unlimited duration and the budget area over the budget.

use proptest::prelude::*;

use super::Profile;
use crate::bench::session::Step;

/// A profile of up to 40 steps, each holding up to 2²⁰ bytes and lasting 1 to
/// 25,000 byte-times.
///
/// The first starts at the session's start or within 1,000 byte-times of it, so
/// no two steps share a time, and some profiles cover the whole session.
fn profile() -> impl Strategy<Value = Profile> {
    (
        prop_oneof![Just(0_u64), 1..1_000_u64],
        prop::collection::vec((1..=25_000_u64, 0..=1_usize << 20), 0..40),
    )
        .prop_map(|(start, spans)| {
            let mut at = start;
            let mut steps = Vec::with_capacity(spans.len());
            for (span, bytes) in spans {
                steps.push(Step { at, bytes });
                at += span;
            }
            Profile {
                duration: at.max(1),
                steps,
            }
        })
}

proptest! {
    /// At or above the profile's peak no moment is stretched, so the
    /// estimate is exactly the unlimited duration; below it the estimate
    /// never falls as the budget falls.
    #[test]
    fn the_estimate_is_the_unlimited_duration_from_the_peak_and_rises_below(
        profile in profile(),
        budgets in prop::collection::vec(1..=2_u64 << 20, 2),
    ) {
        let peak = profile.peak().max(1);
        prop_assert_eq!(profile.duration_at(peak), profile.duration as f64);
        prop_assert_eq!(profile.duration_at(peak + 1), profile.duration as f64);
        let (small, large) = (budgets[0].min(budgets[1]), budgets[0].max(budgets[1]));
        prop_assert!(profile.duration_at(small) >= profile.duration_at(large));
    }

    /// The estimate is never below the unlimited duration nor below the
    /// budget area over the budget, and equals the latter, up to rounding,
    /// when the bytes stay at or above the budget for the whole session.
    #[test]
    fn the_estimate_bounds_the_area_estimate_and_meets_it_above_the_budget(
        profile in profile(),
        budget in 1..=2_u64 << 20,
    ) {
        let estimate = profile.duration_at(budget);
        let area = profile.area() as f64 / budget as f64;
        let rounding = 1e-9 * estimate.max(1.0);
        prop_assert!(estimate >= profile.duration as f64);
        prop_assert!(estimate + rounding >= area);
        let covered = profile.steps.first().is_some_and(|step| step.at == 0)
            && profile.steps.iter().all(|step| step.bytes as u64 >= budget);
        if covered {
            prop_assert!((estimate - area).abs() <= rounding, "{estimate} vs {area}");
        }
    }

    /// The two sides combine into the larger at every moment: the combined
    /// profile's estimate is at least either side's own, and a side
    /// combined with itself is unchanged.
    #[test]
    fn the_larger_side_sets_the_combined_profile(
        a in profile(),
        b in profile(),
        budget in 1..=2_u64 << 20,
    ) {
        let duration = a.duration.max(b.duration);
        let one = |steps: &[Step]| Profile::of_sides(duration, &[steps]);
        let both = Profile::of_sides(duration, &[&a.steps, &b.steps]);
        prop_assert!(both.duration_at(budget) >= one(&a.steps).duration_at(budget));
        prop_assert!(both.duration_at(budget) >= one(&b.steps).duration_at(budget));
        prop_assert_eq!(Profile::of_sides(duration, &[&a.steps, &a.steps]), one(&a.steps));
        prop_assert_eq!(one(&a.steps).area(), Profile { duration, steps: a.steps.clone() }.area());
    }
}
