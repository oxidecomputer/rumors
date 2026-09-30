//! Compact LIFO storage for tree traversal state.
//!
//! [`BitStack`] stores paths and phases one bit at a time.
//! [`PackedU64Stack`] stores nonnegative integers in proportion to their bit
//! width. Both keep deep iterative walks from paying a machine word per open
//! tree level.

mod bit;
mod packed_u64;

pub(crate) use bit::BitStack;
pub(crate) use packed_u64::PackedU64Stack;
