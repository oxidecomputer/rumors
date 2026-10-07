//! Remembers reserved output tags until their branches can be closed.
//!
//! Positions increase during descent and are restored in reverse order during
//! ascent. Consecutive two-bit tags form runs; copied siblings create gaps.
//! Encoding runs and gaps avoids keeping a machine word for every ancestor.

use crate::bits::stack::{BitStack, PackedU64Stack};

use super::{OpenBranch, TAG_BITS};

/// Reserved tag positions in last-in-first-out order.
///
/// Descending through fresh branches reserves adjacent two-bit tags. One run
/// length represents that common case. When emitted output creates a gap, the
/// stack stores the distance to the preceding run. This avoids one machine
/// word per open branch while still recovering every tag position on unwind.
pub struct Positions {
    /// The newest absolute position, or zero when empty.
    top: u64,
    /// Number of positions held.
    len: u64,
    /// Older positions adjacent to `top`.
    adjacent: u64,
    /// Whether each stored value is a run length or a distance.
    records: BitStack,
    /// Run lengths and distances, in stack order.
    values: PackedU64Stack,
}

impl Positions {
    /// Create an empty position stack.
    pub fn new() -> Self {
        Self {
            top: 0,
            len: 0,
            adjacent: 0,
            records: BitStack::new(),
            values: PackedU64Stack::new(),
        }
    }

    /// Retain a newly reserved tag position.
    pub fn push(&mut self, OpenBranch(pos): OpenBranch) {
        debug_assert!(pos >= self.top, "reserved tag positions never move left");
        let distance = pos - self.top;
        if self.len > 0 && distance == TAG_BITS {
            self.adjacent += 1;
        } else {
            self.flush_run();
            self.records.push(false);
            // The packed stack stores positive integers; the first position may be 0.
            self.values.push(distance + 1);
        }
        self.top = pos;
        self.len += 1;
    }

    /// Restore the newest tag position.
    pub fn pop(&mut self) -> OpenBranch {
        assert!(self.len > 0, "position stack underflow");
        let pos = self.top;
        self.len -= 1;
        if self.adjacent > 0 {
            self.adjacent -= 1;
            self.top -= TAG_BITS;
            return OpenBranch(pos);
        }

        let is_run = self.records.pop().expect("position stack underflow");
        let value = self.values.pop();
        if is_run {
            debug_assert!(value > 0, "an adjacent run is nonempty");
            self.adjacent = value - 1;
            self.top -= TAG_BITS;
        } else {
            self.top -= value - 1;
        }
        OpenBranch(pos)
    }

    /// Store the current adjacent run before a gap begins.
    fn flush_run(&mut self) {
        if self.adjacent > 0 {
            self.records.push(true);
            self.values.push(self.adjacent);
            self.adjacent = 0;
        }
    }
}
