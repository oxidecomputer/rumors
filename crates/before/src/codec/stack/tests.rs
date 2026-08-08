use proptest::prelude::*;

use super::{BitStack, PopStack};

/// The pop-able bit stack round-trips arbitrary pushes through pops in LIFO
/// order — zero included (it stores one real value bit) and the full word width
/// — so every consumer's coordinate comes back exactly as pushed.
#[test]
fn pop_stack_round_trips_lifo() {
    let mut stack = PopStack::new();
    let vals = [0u64, 1, 7, 64, 3, 1, 100_000, 2, u64::MAX];
    for &v in &vals {
        stack.push(v);
    }
    for &v in vals.iter().rev() {
        assert_eq!(stack.pop(), v);
    }
}

proptest! {
    /// The word-backed bit stack agrees with a plain `Vec<bool>` on any
    /// interleaving of pushes and pops — `last`, `len`, `is_empty`, and
    /// `all_set` included — across the word-spill boundary at 64 bits.
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
            prop_assert_eq!(stack.len(), model.len());
            prop_assert_eq!(stack.all_set(), model.iter().all(|&b| b));
        }
    }
}
