use std::fmt::Debug;
use std::marker::PhantomData;

/// Peano successor: `S<H>` represents one level above `H`.
///
/// The inner marker is `PhantomData<fn() -> T>` rather than `T` (or
/// `PhantomData<T>`) so the auto-trait check on `S<T>` does not descend
/// into `T`. Without this, proving `S<S<…S<Z>…>>: Sync` recurses 32
/// levels deep every time a downstream crate asks an auto-trait question
/// about a type that names `Root`. Function pointers are unconditionally
/// `Send + Sync` regardless of their return type, so this marker
/// short-circuits the recursion without unsafe `Send` / `Sync` impls.
#[repr(C)]
pub struct S<T>(PhantomData<fn() -> T>);

// Hand-rolled trait impls below rather than `#[derive(...)]` because each
// derived impl would carry an inherited `T: Trait` bound (e.g.
// `#[derive(Clone)]` expands to `impl<T: Clone> Clone for S<T>`), which
// would defeat the whole point of the `fn() -> T` phantom — that bound
// forces the recursive auto-trait walk on every `Clone` / `Debug` query
// against the deep height chain.

impl<T> Copy for S<T> {}

impl<T> Clone for S<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Default for S<T> {
    fn default() -> Self {
        S(PhantomData)
    }
}

impl<T> Debug for S<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S").finish()
    }
}

impl<T> std::hash::Hash for S<T> {
    fn hash<H: std::hash::Hasher>(&self, _state: &mut H) {}
}

impl<T> PartialEq for S<T> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl<T> Eq for S<T> {}

impl<T> PartialOrd for S<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for S<T> {
    fn cmp(&self, _other: &Self) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
    }
}

/// Peano zero: the height of the leaves.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(C)]
pub struct Z;

/// A type-level height: how many levels sit between a node at this height
/// and the leaves.
///
/// Carrying the height in the type is what lets every traversal recurse
/// *polymorphically* — each inductive step is a separate monomorphization
/// that the compiler proves terminates at [`Z`] — instead of trusting a
/// runtime depth counter.
pub trait Height: Debug + Clone + Default + sealed::Sealed + 'static {
    /// This height as a plain number (`Z` is 0; [`Root`] is 32).
    const HEIGHT: usize;
}

/// The predecessor of a nonzero height: `S<T>` has predecessor `T`. Deliberately
/// not implemented for `Z`, so a `Pred` bound witnesses that a height is nonzero.
pub trait Pred: sealed::Sealed {
    type Pred: Height;
}

impl<H: Height> Pred for S<H> {
    type Pred = H;
}

/// The height just under the root, i.e. 31.
pub type UnderRoot = <Root as Pred>::Pred;

/// The height two levels under the root, i.e. 30.
pub type UnderUnderRoot = <UnderRoot as Pred>::Pred;

/// Enumerate `Height` (and its `Sealed` supertrait) for the Peano numbers
/// `Z`, `S<Z>`, ..., `S^N<Z>`, where `N` is the count of `_` tokens passed.
/// Each successor also gets a `Pred` impl pointing back at its predecessor.
macro_rules! impl_heights {
    (@emit $t:ty, $n:expr;) => {
        impl Height for $t { const HEIGHT: usize = $n; }
    };
    (@emit $t:ty, $n:expr; $head:tt $($tail:tt)*) => {
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

/// The height of the root: 32 levels above the leaves, one per byte of a
/// leaf's 32-byte version-derived path.
#[rustfmt::skip]
pub type Root =
// Laid out for your counting convenience in two rows of 16:
    S<S<S<S<S<S<S<S<S<S<S<S<S<S<S<S< // 0
    S<S<S<S<S<S<S<S<S<S<S<S<S<S<S<S< // 1
//  0 1 2 3 4 5 6 7 8 9 a b c d e f
    Z>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>;

/// Numbered aliases `H0` (= [`Z`]) through `H32` (= [`Root`]), one per
/// height.
///
/// The vocabulary a runtime-to-type dispatch table indexes with
/// `seq_macro` ident pasting, which can splice a numbered name but cannot
/// build an `S<…>` chain (see the erased plumbing's `at_parent_height!`).
/// Each alias is one successor over its predecessor by construction of
/// the emitting macro, so a name always denotes its own number's height.
macro_rules! alias_heights {
    (@emit $t:ty; $name:ident $($rest:ident)*) => {
        pub(crate) type $name = $t;
        alias_heights!(@emit S<$t>; $($rest)*);
    };
    (@emit $t:ty;) => {};
    ($($name:ident)*) => {
        alias_heights!(@emit Z; $($name)*);
    };
}

#[rustfmt::skip]
alias_heights!(
    H0  H1  H2  H3  H4  H5  H6  H7
    H8  H9  H10 H11 H12 H13 H14 H15
    H16 H17 H18 H19 H20 H21 H22 H23
    H24 H25 H26 H27 H28 H29 H30 H31
    H32
);

/// The table's endpoints are what their names claim; each alias being one
/// successor over its predecessor by construction, every name between
/// denotes its own number's height.
const _: () = assert!(H0::HEIGHT == 0 && H32::HEIGHT == 32);

mod sealed {
    use super::*;
    pub trait Sealed {}
    impl Sealed for Z {}
    impl<H: Sealed> Sealed for S<H> {}
}

#[cfg(test)]
mod tests;
