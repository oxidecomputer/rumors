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

mod difference;
mod reader;

use crate::party::io::PartyReader;
use crate::party::Party;

use difference::Difference;

impl<'a> PartyReader<'a> {
    /// Return the canonical region owned by `self` but not `other`.
    ///
    /// Unlike [`join`](PartyReader::join), `without` is total: overlap is the
    /// point, not an error. Its result is `None` when `other` covers `self`,
    /// because a [`Party`] always owns a nonempty region.
    ///
    /// This internal walk only computes the remainder. The public operation
    /// consumes the source party, which is what preserves linear ownership.
    /// Each operand is read at most once.
    pub fn without<'b>(self, other: PartyReader<'b>) -> Option<Party> {
        Difference::between(self, other)
    }
}
