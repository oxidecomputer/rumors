//! Compares two versions directly from their canonical streams.
//!
//! A version is a step function. To compare two versions, two leaf cursors walk
//! their constant-height regions from left to right. Their current regions
//! overlap and therefore nest; the smaller region supplies the next boundary.
//! On every region where both heights are constant, the walk reads the sign of
//! `D = height_a - height_b` from an [`Accumulator`](suanpan::Accumulator).
//!
//! The signs rule out either possible ordering. If `D > 0` anywhere, `a <= b`
//! is false. If `D < 0` anywhere, `b <= a` is false. If neither ordering
//! survives, the versions are concurrent; if both survive, they are equal.
//! The same state answers the narrower equality and domination questions.
//!
//! # Cost
//!
//! Each boundary and payload is decoded once. The accumulator handles small
//! changes to wide heights without repeatedly copying those heights, so work is
//! linear in the combined stream size. Transient state is two compact cursor
//! paths and one accumulator; traversal is iterative.
//!
//! # Early exit
//!
//! Each entry point stops when its answer is irreversible. Equality stops at
//! the first nonzero difference, domination at the first violating region, and
//! causal comparison once both orderings have been ruled out.

#![allow(rustdoc::private_intra_doc_links)]

use core::cmp::Ordering;
use core::ops::ControlFlow;

use super::overlay::{advance_diff, OpenedPair};
use crate::version::io::regions::RegionReader;
use crate::Version;

impl Version {
    /// Test equality through the tree walk rather than canonical bytes.
    #[cfg(any(test, feature = "meter"))]
    pub(crate) fn walk_eq(&self, other: &Version) -> bool {
        compare(self, other, OrderState::exit_equality, |state| {
            debug_assert!(
                state.is_equal(),
                "exhaustion after no refutation is equality"
            );
            true
        })
    }

    /// Test concurrency through the comparison walk.
    #[cfg(test)]
    pub(crate) fn walk_concurrent(&self, other: &Version) -> bool {
        self.partial_cmp(other).is_none()
    }

    /// Test causal domination with the single-direction early exit.
    #[cfg(test)]
    pub(crate) fn walk_le(&self, other: &Version) -> bool {
        compare(
            self,
            other,
            |state| {
                if state.allows_le() {
                    ControlFlow::Continue(())
                } else {
                    ControlFlow::Break(false)
                }
            },
            OrderState::allows_le,
        )
    }
}

/// Compares two versions in causal order by walking their canonical streams.
///
/// Shared storage proves equality without a walk. Otherwise traversal stops as
/// soon as both possible orderings have been refuted.
impl PartialOrd<Version> for Version {
    fn partial_cmp(&self, other: &Version) -> Option<Ordering> {
        // A full `==` would scan every unequal pair before this walk scanned it
        // again; shared storage preserves the useful constant-time fast path.
        if self.ptr_eq(other) {
            return Some(Ordering::Equal);
        }
        compare(self, other, OrderState::exit_order, OrderState::relation)
    }
}

/// Which of `a <= b` and `b <= a` the visited regions have not refuted.
///
/// Each region's height difference may refute one direction. Refutation is
/// permanent, which makes early exit sound.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OrderState {
    /// Every visited region is equal, so both directions remain possible.
    Equal,
    /// Only `a <= b` remains possible.
    LessOrEqual,
    /// Only `a >= b` remains possible.
    GreaterOrEqual,
    /// Both directions have been refuted.
    Concurrent,
}

impl OrderState {
    /// Both directions open: nothing folded yet.
    pub fn new() -> OrderState {
        OrderState::Equal
    }

    /// Fold one region's sign of `D = height_a − height_b` into
    /// the surviving directions: a positive interval refutes `a <= b`, a
    /// negative one `b <= a`.
    pub fn fold(&mut self, sign: Ordering) {
        *self = match (*self, sign) {
            (state, Ordering::Equal) => state,
            (OrderState::Concurrent, _) => OrderState::Concurrent,
            (OrderState::Equal | OrderState::GreaterOrEqual, Ordering::Greater) => {
                OrderState::GreaterOrEqual
            }
            (OrderState::Equal | OrderState::LessOrEqual, Ordering::Less) => {
                OrderState::LessOrEqual
            }
            (OrderState::LessOrEqual, Ordering::Greater)
            | (OrderState::GreaterOrEqual, Ordering::Less) => OrderState::Concurrent,
        };
    }

    /// The relation the folded directions decide, as the causal order: both
    /// surviving is equality, one is the strict order, neither is concurrent
    /// (`None`).
    pub fn relation(self) -> Option<Ordering> {
        match self {
            OrderState::Equal => Some(Ordering::Equal),
            OrderState::LessOrEqual => Some(Ordering::Less),
            OrderState::GreaterOrEqual => Some(Ordering::Greater),
            OrderState::Concurrent => None,
        }
    }

    /// Whether no visited region has refuted `a <= b`.
    pub fn allows_le(self) -> bool {
        matches!(self, OrderState::Equal | OrderState::LessOrEqual)
    }

    /// Whether no visited region has refuted `a >= b`.
    pub fn allows_ge(self) -> bool {
        matches!(self, OrderState::Equal | OrderState::GreaterOrEqual)
    }

    /// Whether every visited region has equal heights.
    pub fn is_equal(self) -> bool {
        self == OrderState::Equal
    }

    /// Whether visited regions have refuted both causal directions.
    pub fn is_concurrent(self) -> bool {
        self == OrderState::Concurrent
    }

    /// Stop a full comparison once both causal directions have been refuted.
    pub fn exit_order(self) -> ControlFlow<Option<Ordering>> {
        if self.is_concurrent() {
            ControlFlow::Break(None)
        } else {
            ControlFlow::Continue(())
        }
    }

    /// Stop an equality comparison at its first nonzero difference.
    pub fn exit_equality(self) -> ControlFlow<bool> {
        if self.is_equal() {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(false)
        }
    }
}

/// Run the merge, generic over the question asked of the surviving
/// [`OrderState`].
///
/// After each interval's sign fold, `exit` sees the surviving directions and
/// may declare the question decided — the `Break` payload carries the verdict,
/// so the earliest stop and its answer are one value. A break carries `V`
/// rather than the directions because the direction the question ignores may
/// be stale at an early exit: only the fully-swept directions `finish` maps
/// at exhaustion are all decided.
fn compare<V>(
    a_bits: &Version,
    b_bits: &Version,
    exit: impl Fn(OrderState) -> ControlFlow<V>,
    finish: impl FnOnce(OrderState) -> V,
) -> V {
    let OpenedPair {
        mut a,
        mut b,
        mut diff,
        ..
    } = OpenedPair::open(a_bits, b_bits);
    let mut directions = OrderState::new();
    loop {
        // The current region ends at the earlier leaf boundary, and D is
        // constant throughout it.
        directions.fold(diff.cmp_zero());
        if let ControlFlow::Break(verdict) = exit(directions) {
            return verdict;
        }
        if a.done() && b.done() {
            return finish(directions);
        }
        advance_diff(&mut a, &mut b, &mut diff);
    }
}

#[cfg(test)]
mod tests;
