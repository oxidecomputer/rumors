//! Model checks for compact id-builder state.

use proptest::prelude::*;

use super::{Open, PosStack};

proptest! {
    /// Interleaved pushes and pops preserve every open tag position across
    /// adjacent runs and explicit gaps.
    #[test]
    fn position_stack_matches_a_vec(
        actions in prop::collection::vec(
            (any::<bool>(), prop_oneof![4 => Just(2u64), 1 => 0u64..64]),
            0..512,
        ),
    ) {
        let mut stack = PosStack::new();
        let mut expected = Vec::with_capacity(actions.len());

        for (pop, delta) in actions {
            if pop && !expected.is_empty() {
                let position = expected.pop().expect("the model is nonempty");
                let Open(actual) = stack.pop();
                prop_assert_eq!(actual, position);
            } else {
                let position = expected.last().copied().unwrap_or(0) + delta;
                stack.push(Open(position));
                expected.push(position);
            }
        }

        while let Some(position) = expected.pop() {
            let Open(actual) = stack.pop();
            prop_assert_eq!(actual, position);
        }
    }
}
