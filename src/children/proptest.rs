//! [`Arbitrary`](::proptest::arbitrary::Arbitrary) impl for [`Children`].

use ::proptest::prelude::*;
use ::proptest::strategy::BoxedStrategy;

use super::Children;

impl<T> Arbitrary for Children<T>
where
    T: Arbitrary + 'static,
{
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_: ()) -> Self::Strategy {
        ::proptest::collection::btree_map(any::<u8>(), any::<T>(), 0..=256)
            .prop_map(|map| {
                let mut c = Children::new();
                for (k, v) in map {
                    c.insert(k, v);
                }
                c
            })
            .boxed()
    }
}
