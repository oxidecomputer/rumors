//! Tests for population-relative trace operands.

use proptest::prelude::*;
use proptest::sample::Index;

use crate::{Clock, Version};

use super::{step_impl, Op, MAX_TRACE_OPS};

proptest! {
    /// A trace operand can select and mutate any clock appended beyond the
    /// generator's initially small population.
    #[test]
    fn population_selectors_reach_appended_members(
        (population, selector) in (9usize..MAX_TRACE_OPS, any::<Index>())
            .prop_filter("selector addresses an appended member", |(n, i)| i.index(*n) >= 8)
    ) {
        let mut clocks = vec![Clock::seed()];
        while clocks.len() < population {
            let child = clocks[0].fork();
            clocks.push(child);
        }

        let selected = selector.index(population);
        let before: Vec<Version> = clocks.iter().map(|clock| clock.version().clone()).collect();
        step_impl(&mut clocks, &Op::Tick(selector));

        for (index, (old, clock)) in before.iter().zip(&clocks).enumerate() {
            if index == selected {
                prop_assert_ne!(old, clock.version());
            } else {
                prop_assert_eq!(old, clock.version());
            }
        }
    }
}
