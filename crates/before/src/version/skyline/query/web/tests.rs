//! Tests for the compact reign representation.

use num_bigint::BigInt;

use super::{ReignStore, REIGN_COUNT_MASK, REIGN_EPOCH_MASK, REIGN_OFFSET_BITS};

/// Inline reign fields round-trip at every boundary and spill exactly beyond
/// them, without changing the retained offset, epoch, or close count.
#[test]
fn reign_storage_preserves_every_representation_boundary() {
    let min_offset = -(1i64 << (REIGN_OFFSET_BITS - 1));
    let max_offset = (1i64 << (REIGN_OFFSET_BITS - 1)) - 1;
    let cases = [
        (min_offset - 1, 0),
        (min_offset, 0),
        (-1, 0),
        (0, 0),
        (1, REIGN_EPOCH_MASK as usize),
        (max_offset, 0),
        (max_offset + 1, 0),
        (0, REIGN_EPOCH_MASK as usize + 1),
    ];

    for (offset, epoch) in cases {
        let mut store = ReignStore::new();
        let mut stored = store.store(&BigInt::from(offset), epoch);
        let fits_inline =
            (min_offset..=max_offset).contains(&offset) && epoch <= REIGN_EPOCH_MASK as usize;
        assert_eq!(stored.is_spilled(), !fits_inline);
        for _ in 0..REIGN_COUNT_MASK {
            store.increment(&mut stored);
        }
        assert_eq!(stored.is_spilled(), !fits_inline);
        store.increment(&mut stored);
        assert!(stored.is_spilled());
        let reign = store.take(stored);
        assert_eq!(reign.offset, BigInt::from(offset));
        assert_eq!(reign.epoch, epoch);
        assert_eq!(reign.count, REIGN_COUNT_MASK + 1);
    }
}
