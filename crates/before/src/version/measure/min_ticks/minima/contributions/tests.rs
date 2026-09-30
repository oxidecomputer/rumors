//! Tests for the compact contribution representation.

use num_bigint::BigInt;

use super::{ContributionStore, CLOSE_MASK, OFFSET_BITS, PREFIX_MASK};

/// Inline contribution fields round-trip at every boundary and spill beyond
/// them, without changing the retained offset, prefix, or close count.
#[test]
fn contribution_storage_preserves_every_representation_boundary() {
    let min_offset = -(1i64 << (OFFSET_BITS - 1));
    let max_offset = (1i64 << (OFFSET_BITS - 1)) - 1;
    let cases = [
        (min_offset - 1, 0),
        (min_offset, 0),
        (-1, 0),
        (0, 0),
        (1, PREFIX_MASK as usize),
        (max_offset, 0),
        (max_offset + 1, 0),
        (0, PREFIX_MASK as usize + 1),
    ];

    for (offset, prefix) in cases {
        let mut store = ContributionStore::new();
        let mut stored = store.store_parts(&BigInt::from(offset), prefix);
        let fits_inline =
            (min_offset..=max_offset).contains(&offset) && prefix <= PREFIX_MASK as usize;
        assert_eq!(stored.is_spilled(), !fits_inline);
        for _ in 0..CLOSE_MASK {
            store.increment(&mut stored);
        }
        assert_eq!(stored.is_spilled(), !fits_inline);
        store.increment(&mut stored);
        assert!(stored.is_spilled());
        let contribution = store.take(stored);
        assert_eq!(contribution.offset, BigInt::from(offset));
        assert_eq!(contribution.prefix, prefix);
        assert_eq!(contribution.closes, CLOSE_MASK + 1);
    }
}
