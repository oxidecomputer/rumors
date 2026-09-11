//! Type-level heights for the tree's 32-byte paths.
//!
//! [`Z`] marks leaves; [`S<H>`] adds one level above `H`. [`Height`] is
//! implemented only through [`Root`], so typed traversals cannot step past
//! either end. Numbered aliases let runtime dispatch select the same types.

use std::fmt::Debug;
use std::marker::PhantomData;

/// One level above `H`.
///
/// `PhantomData<fn() -> T>` keeps this marker independent of `T`'s auto
/// traits. Function pointers are always `Send + Sync`, so checking those
/// traits does not recurse through the height chain.
pub struct S<T>(
    /// The predecessor, recorded only in the type.
    PhantomData<fn() -> T>,
);

// Explicit implementations avoid derive's bounds on `T`; height markers
// carry no values whose traits need to be checked recursively.

/// Copy the marker without requiring `T: Copy`.
impl<T> Copy for S<T> {}

/// Clone the marker without requiring `T: Clone`.
impl<T> Clone for S<T> {
    /// Copy this zero-sized marker.
    fn clone(&self) -> Self {
        *self
    }
}

/// Construct the marker without constructing a predecessor value.
impl<T> Default for S<T> {
    /// Create the zero-sized successor marker.
    fn default() -> Self {
        S(PhantomData)
    }
}

/// Identify the marker without formatting its predecessor type.
impl<T> Debug for S<T> {
    /// Render the successor marker as `S`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S").finish()
    }
}

/// Equal successor markers contribute no distinguishing bytes.
impl<T> std::hash::Hash for S<T> {
    /// Leave the hasher unchanged: this type has only one value.
    fn hash<H: std::hash::Hasher>(&self, _state: &mut H) {}
}

/// All values of the same successor type are equal.
impl<T> PartialEq for S<T> {
    /// Compare the sole value of this marker type.
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

/// Marker equality is total.
impl<T> Eq for S<T> {}

/// Order successor markers by their sole value.
impl<T> PartialOrd for S<T> {
    /// Delegate to the total ordering.
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Every pair of values of this marker type compares equal.
impl<T> Ord for S<T> {
    /// Return equality without comparing predecessor types.
    fn cmp(&self, _other: &Self) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
    }
}

/// Height zero: the leaves.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Z;

/// A type-level height: how many levels sit between a node at this height
/// and the leaves.
///
/// Traversals specialize each step for its height and stop at [`Z`]. The
/// bound describes a type; it does not require constructing a marker value.
pub trait Height: Sized + sealed::Sealed + 'static {
    /// This height as a plain number (`Z` is 0; [`Root`] is 32).
    const HEIGHT: usize;
}

/// The predecessor of a nonzero height; [`Z`] has no predecessor.
pub trait Pred: sealed::Sealed {
    /// The height one level closer to the leaves.
    type Pred: Height;
}

/// A successor's predecessor is its type parameter.
impl<H: Height> Pred for S<H> {
    /// Remove one successor from the height.
    type Pred = H;
}

/// The height just under the root, i.e. 31.
pub type UnderRoot = <Root as Pred>::Pred;

/// The height two levels under the root, i.e. 30.
pub type UnderUnderRoot = <UnderRoot as Pred>::Pred;

/// Define numbered aliases and their numeric heights together, starting at zero.
macro_rules! heights {
    (@emit $t:ty, $n:expr;) => {};
    (@emit $t:ty, $n:expr; $name:ident $($rest:ident)*) => {
        #[doc = concat!("The `", stringify!($name), "` height used by runtime dispatch.")]
        pub(crate) type $name = $t;
        /// The numeric height of this marker type.
        impl Height for $t {
            /// The number of successor steps above a leaf.
            const HEIGHT: usize = $n;
        }
        heights!(@emit S<$t>, $n + 1; $($rest)*);
    };
    ($($name:ident)*) => {
        heights!(@emit Z, 0; $($name)*);
    };
}

/// The height of the root: 32 levels above the leaves, one per byte of a
/// leaf's 32-byte version-derived path.
pub type Root = H32;

// The erased traversal's dispatch tables paste these names with seq_macro.
#[rustfmt::skip]
heights!(
    H0  H1  H2  H3  H4  H5  H6  H7
    H8  H9  H10 H11 H12 H13 H14 H15
    H16 H17 H18 H19 H20 H21 H22 H23
    H24 H25 H26 H27 H28 H29 H30 H31
    H32
);

/// A root-to-leaf walk consumes exactly one 32-byte address.
const _: () = assert!(H0::HEIGHT == 0 && Root::HEIGHT == 32);

/// The leaf height is a zero-sized marker with no alignment requirement.
const _: () = assert!(size_of::<Z>() == 0 && align_of::<Z>() == 1);
/// A successor adds no storage or alignment requirement.
const _: () = assert!(size_of::<S<Z>>() == 0 && align_of::<S<Z>>() == 1);
/// The complete height chain remains a zero-sized marker.
const _: () = assert!(size_of::<Root>() == 0 && align_of::<Root>() == 1);

/// Keep height implementations within the successor chain defined here.
mod sealed {
    use super::{S, Z};

    /// A marker belonging to this module's height family.
    pub trait Sealed {}
    /// Zero starts the height family.
    impl Sealed for Z {}
    /// A successor stays within the same family.
    impl<H: Sealed> Sealed for S<H> {}
}
