//! Checks that memo slots return the values stored in them, across block
//! boundaries and out of insertion order.

use super::{Memo, StoredAccumulator};

/// Values inserted nonsequentially at both edges of three memo blocks are
/// read back from those same six slots.
#[test]
fn selected_out_of_order_values_cross_block_boundaries() {
    let mut memo = Memo::new();
    let mut expected = [None; 130];
    for _ in &expected {
        memo.reserve();
    }

    for (slot, value) in [(129, -7), (64, 5), (0, 1), (63, -2), (65, 3), (128, 4)] {
        expected[slot] = Some(value);
        memo.set_link(slot, StoredAccumulator::Small(value));
    }

    for (slot, expected) in expected.into_iter().enumerate() {
        let actual = memo.take_link(slot).map(|stored| match stored {
            StoredAccumulator::Small(value) => value,
            StoredAccumulator::Wide(_) => panic!("the fixture stores only machine words"),
        });
        assert_eq!(actual, expected, "slot {slot}");
    }
}
