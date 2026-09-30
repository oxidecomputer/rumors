//! Removes one party's region from another.
//!
//! Each party partitions the unit interval into owned and unowned regions. A
//! [`Difference`] walks both partitions from left to right and emits a region
//! exactly where the first party owns it and the second does not.
//!
//! When one current region covers an entire subtree of the other party, the
//! walk settles that subtree at once: it either copies the first party's
//! subtree or skips a region that cannot survive. Every input tag is therefore
//! read at most once.

mod cursor;
mod difference;

use crate::codec::BitsBuf;
use crate::party::tree::PartyCursor;

use difference::Difference;

impl<'a> PartyCursor<'a> {
    /// Return the canonical region owned by `self` but not `other`.
    ///
    /// Unlike [`join`](PartyCursor::join), `without` is total: overlap is the
    /// point, not an error. Its result is empty when `other` covers `self`. The
    /// caller maps that empty result to `None`, because a [`Party`](crate::Party)
    /// always owns a nonempty region.
    ///
    /// The result is a subregion of `self`, so consuming `self` and returning
    /// its remainder cannot duplicate ownership. Each operand is read at most
    /// once.
    pub(crate) fn without<'b>(self, other: PartyCursor<'b>) -> BitsBuf {
        Difference::new(self, other).finish()
    }
}
