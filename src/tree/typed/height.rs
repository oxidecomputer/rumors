use std::fmt::Debug;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(C)]
pub struct S<T>(pub T);

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(C)]
pub struct Z;

pub trait Height: Send + Sync + Debug + Clone + Default + sealed::Sealed {
    const HEIGHT: usize;
}

/// Enumerate `Height` (and its `Sealed` supertrait) for the Peano numbers
/// `Z`, `S<Z>`, ..., `S^N<Z>`, where `N` is the count of `_` tokens passed.
/// Each successor also gets a `Pred` impl pointing back at its predecessor.
macro_rules! impl_heights {
    (@emit $t:ty, $n:expr;) => {
        impl sealed::Sealed for $t {}
        impl Height for $t { const HEIGHT: usize = $n; }
    };
    (@emit $t:ty, $n:expr; $head:tt $($tail:tt)*) => {
        impl sealed::Sealed for $t {}
        impl Height for $t { const HEIGHT: usize = $n; }
        impl_heights!(@emit S<$t>, $n + 1; $($tail)*);
    };
    ($($tok:tt)*) => {
        impl_heights!(@emit Z, 0; $($tok)*);
    };
}

// 32 `_` tokens => heights 0..=32 inclusive (33 impls).
#[rustfmt::skip]
impl_heights!(
    _ _ _ _ _ _ _ _
    _ _ _ _ _ _ _ _
    _ _ _ _ _ _ _ _
    _ _ _ _ _ _ _ _
);

/// The height of the root of the tree: 32 bytes.
#[rustfmt::skip]
pub type Root =
// Laid out for your counting convenience in two rows of 16:
    S<S<S<S<S<S<S<S<S<S<S<S<S<S<S<S< // 0
    S<S<S<S<S<S<S<S<S<S<S<S<S<S<S<S< // 1
//  0 1 2 3 4 5 6 7 8 9 a b c d e f
    Z>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>;

mod sealed {
    pub trait Sealed {}
}

#[cfg(test)]
mod test;
