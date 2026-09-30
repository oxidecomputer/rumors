use proptest::prelude::*;

use super::BitStack;

proptest! {
    /// The word-backed bit stack agrees with a plain `Vec<bool>` on arbitrary
    /// pushes and pops, including paths that cross machine-word boundaries.
    #[test]
    fn bit_stack_matches_a_vec_of_bools(
        ops in proptest::collection::vec((any::<bool>(), any::<bool>()), 1..300),
    ) {
        let mut stack = BitStack::new();
        let mut model: Vec<bool> = Vec::new();
        for (push, bit) in ops {
            if push {
                stack.push(bit);
                model.push(bit);
            } else {
                prop_assert_eq!(stack.pop(), model.pop());
            }
            prop_assert_eq!(stack.last(), model.last().copied());
            prop_assert_eq!(stack.len(), model.len() as u64);
            prop_assert_eq!(stack.all_set(), model.iter().all(|&b| b));
        }
    }
}
