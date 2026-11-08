//! Byte sets against an ordered set of bytes as the oracle, and their encoding
//! against its schema.

use std::collections::BTreeSet;

use proptest::prelude::*;

use super::{BITMAP_LEN, ByteSet, ByteSetError, MAX_ENCODED_LEN};

/// Any set, from empty to full, with every size likely enough to reach both
/// spellings: a uniform count of elements, then that many distinct values.
fn any_set() -> impl Strategy<Value = BTreeSet<u8>> {
    (0..=256usize).prop_flat_map(|len| {
        proptest::sample::subsequence((0..=u8::MAX).collect::<Vec<_>>(), len)
            .prop_map(|values| values.into_iter().collect())
    })
}

/// The schema's encoding of `set`, written from the module docs' table rather
/// than from the implementation: the ascending elements below 32 of them, the
/// bitmap (value `v` at bit `v % 8` of byte `v / 8`) from 32 on.
fn schema(set: &BTreeSet<u8>) -> Vec<u8> {
    if set.len() < 32 {
        set.iter().copied().collect()
    } else {
        let mut bitmap = vec![0u8; 32];
        for &value in set {
            bitmap[usize::from(value / 8)] |= 1 << (value % 8);
        }
        bitmap
    }
}

/// One operation on a set, for the model comparison.
#[derive(Clone, Debug)]
enum Operation {
    Insert(u8),
    Remove(u8),
}

fn operation() -> impl Strategy<Value = Operation> {
    prop_oneof![
        any::<u8>().prop_map(Operation::Insert),
        any::<u8>().prop_map(Operation::Remove),
    ]
}

proptest! {
    /// Every set encodes to exactly the schema's bytes, at most a bitmap's
    /// 32, and decodes back to itself.
    #[test]
    fn every_set_round_trips_through_its_schema_encoding(set in any_set()) {
        let byte_set: ByteSet = set.iter().copied().collect();
        let encoded = byte_set.encoded();
        let expected_encoding = schema(&set);
        prop_assert_eq!(encoded.as_bytes(), expected_encoding.as_slice());
        prop_assert!(encoded.as_bytes().len() <= MAX_ENCODED_LEN);
        prop_assert_eq!(ByteSet::decode(encoded.as_bytes()), Ok(byte_set));
    }

    /// Decoding accepts exactly the canonical encodings: any byte string up
    /// to a few bytes past a bitmap decodes if and only if it is the
    /// encoding of the set it decodes to.
    #[test]
    fn decoding_accepts_exactly_the_canonical_encodings(
        bytes in proptest::collection::vec(any::<u8>(), 0..=BITMAP_LEN + 4),
    ) {
        match ByteSet::decode(&bytes) {
            Ok(set) => {
                let encoded = set.encoded();
                prop_assert_eq!(encoded.as_bytes(), bytes.as_slice());
            }
            Err(error) => {
                let canonical = match bytes.len() {
                    len if len < BITMAP_LEN => bytes.windows(2).all(|pair| pair[0] < pair[1]),
                    BITMAP_LEN => bytes.iter().map(|byte| byte.count_ones()).sum::<u32>() >= 32,
                    _ => false,
                };
                prop_assert!(!canonical, "{:?} rejected {:?}", error, bytes);
            }
        }
    }

    /// Every other spelling of a set is rejected with the error naming
    /// what is wrong with it: a list with two elements swapped or one
    /// repeated does not ascend, and a set below 32 elements spelled as its
    /// bitmap is sparse.
    #[test]
    fn every_other_spelling_of_a_set_is_rejected(
        set in any_set(),
        index in any::<prop::sample::Index>(),
    ) {
        let byte_set: ByteSet = set.iter().copied().collect();
        if set.len() < 32 {
            let list: Vec<u8> = set.iter().copied().collect();
            let mut bitmap = [0u8; BITMAP_LEN];
            bitmap.copy_from_slice(byte_set.0.as_raw_slice());
            prop_assert_eq!(
                ByteSet::decode(&bitmap),
                Err(ByteSetError::SparseBitmap { len: set.len() })
            );
            if list.len() >= 2 {
                let at = index.index(list.len() - 1);
                let mut swapped = list.clone();
                swapped.swap(at, at + 1);
                prop_assert_eq!(
                    ByteSet::decode(&swapped),
                    Err(ByteSetError::Unordered {
                        index: at + 1,
                        previous: list[at + 1],
                        value: list[at],
                    })
                );
            }
            if !list.is_empty() && list.len() < 31 {
                let at = index.index(list.len());
                let mut repeated = list.clone();
                repeated.insert(at, list[at]);
                prop_assert_eq!(
                    ByteSet::decode(&repeated),
                    Err(ByteSetError::Unordered {
                        index: at + 1,
                        previous: list[at],
                        value: list[at],
                    })
                );
            }
        } else {
            // The ascending list of a set of 33 or more elements is longer
            // than any encoding; the list of exactly 32 has a bitmap's
            // length and is read as one.
            let list: Vec<u8> = set.iter().copied().collect();
            if list.len() > BITMAP_LEN {
                prop_assert_eq!(
                    ByteSet::decode(&list),
                    Err(ByteSetError::TooLong { len: list.len() })
                );
            }
        }
    }

    /// Any sequence of insertions and removals leaves the set holding what
    /// an ordered set of bytes holds.
    ///
    /// The two agree on every byte's membership, the count, the count above
    /// every byte, the ascending elements, and each insertion's and
    /// removal's answer.
    #[test]
    fn operations_agree_with_an_ordered_set(
        operations in proptest::collection::vec(operation(), 0..600),
    ) {
        let mut set = ByteSet::EMPTY;
        let mut model = BTreeSet::new();
        for operation in operations {
            match operation {
                Operation::Insert(value) => prop_assert_eq!(set.insert(value), model.insert(value)),
                Operation::Remove(value) => prop_assert_eq!(set.remove(value), model.remove(&value)),
            }
            prop_assert_eq!(set.len(), model.len());
            prop_assert_eq!(set.is_empty(), model.is_empty());
        }
        for value in 0..=u8::MAX {
            prop_assert_eq!(set.contains(value), model.contains(&value), "value {}", value);
        }
        prop_assert!(set.iter().eq(model.iter().copied()));
        prop_assert!(set.iter().rev().eq(model.iter().rev().copied()));
        for value in 0..=u8::MAX {
            prop_assert_eq!(set.len_above(value), model.iter().filter(|&&element| element > value).count(), "value {}", value);
        }
        prop_assert_eq!(set, model.iter().copied().collect::<ByteSet>());
    }

    /// The union of two sets holds exactly the values either holds, so its
    /// count is the sum of theirs less the values they share.
    #[test]
    fn a_union_holds_exactly_what_either_set_holds(ours in any_set(), theirs in any_set()) {
        let union = ours
            .iter()
            .copied()
            .collect::<ByteSet>()
            .union(theirs.iter().copied().collect());
        prop_assert!(union.iter().eq(ours.union(&theirs).copied()));
        let shared = ours.intersection(&theirs).count();
        prop_assert_eq!(union.len(), ours.len() + theirs.len() - shared);
    }
}

/// A set encodes as a list exactly below 32 elements and as the bitmap from 32
/// on, round-trips, and has its other spelling rejected.
///
/// The sets are every one of 30 to 33 elements that is a run of consecutive
/// values, or of values nine apart (each in the bitmap's next byte, one bit
/// further along), from every starting value with wraparound.
#[test]
fn the_spelling_switches_exactly_at_32_elements() {
    for len in 30..=33usize {
        for stride in [1usize, 9] {
            for start in 0..=255usize {
                let set: BTreeSet<u8> = (0..len)
                    .map(|i| u8::try_from((start + i * stride) % 256).expect("reduced mod 256"))
                    .collect();
                assert_eq!(
                    set.len(),
                    len,
                    "stride {stride} from {start} repeats no value"
                );
                let byte_set: ByteSet = set.iter().copied().collect();
                let encoded = byte_set.encoded();
                assert_eq!(encoded.as_bytes().len(), len.min(BITMAP_LEN));
                assert_eq!(encoded.as_bytes(), schema(&set).as_slice());
                assert_eq!(ByteSet::decode(encoded.as_bytes()), Ok(byte_set));
                if len < 32 {
                    let mut bitmap = [0u8; BITMAP_LEN];
                    bitmap.copy_from_slice(byte_set.0.as_raw_slice());
                    assert_eq!(
                        ByteSet::decode(&bitmap),
                        Err(ByteSetError::SparseBitmap { len })
                    );
                } else if len > 32 {
                    let list: Vec<u8> = set.iter().copied().collect();
                    assert_eq!(ByteSet::decode(&list), Err(ByteSetError::TooLong { len }));
                }
            }
        }
    }
}

/// The empty set is the empty byte string, and the full set is the bitmap of
/// all ones: the two ends of the size range, each in its spelling.
#[test]
fn the_empty_and_full_sets_take_their_spellings() {
    assert_eq!(ByteSet::EMPTY.encoded().as_bytes(), &[] as &[u8]);
    assert_eq!(ByteSet::decode(&[]), Ok(ByteSet::EMPTY));
    let full: ByteSet = (0..=u8::MAX).collect();
    assert_eq!(full.len(), 256);
    assert_eq!(full.encoded().as_bytes(), &[u8::MAX; BITMAP_LEN]);
    assert_eq!(ByteSet::decode(&[u8::MAX; BITMAP_LEN]), Ok(full));
}
