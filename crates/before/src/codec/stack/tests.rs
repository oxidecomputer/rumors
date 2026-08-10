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

    /// The pop-able integer stack agrees with a plain `Vec<u64>` model on
    /// arbitrary interleavings of pushes and pops, with entry widths drawn
    /// uniformly across the whole 1..=64 range.
    ///
    /// Uniform widths put mass on both sides of the pop-side width scan's
    /// discrimination band around the 62-continuation cap — the widest
    /// register-scanned entries and the narrowest capped ones — and on the
    /// full-word split-value case, at varying unary-register fill; a drain to
    /// empty closes every case.
    #[test]
    fn pop_stack_matches_a_vec_model_across_all_widths(
        ops in proptest::collection::vec(
            (any::<bool>(), 0u32..64, any::<u64>()),
            1..200,
        ),
    ) {
        let mut stack = PopStack::new();
        let mut model: Vec<u64> = Vec::new();
        for (push, shift, raw) in ops {
            if push || model.is_empty() {
                // Width `shift + 1` exactly: a set top bit over `shift` raw
                // low bits (shift 63 gives the full-word split-value case;
                // shift 0 admits zero, the other one-bit value).
                let v = if shift == 0 {
                    raw & 1
                } else {
                    (1u64 << shift) | (raw & ((1u64 << shift) - 1))
                };
                stack.push(v);
                model.push(v);
            } else {
                prop_assert_eq!(stack.pop(), model.pop().expect("guarded nonempty"));
            }
        }
        for v in model.into_iter().rev() {
            prop_assert_eq!(stack.pop(), v);
        }
    }
}
