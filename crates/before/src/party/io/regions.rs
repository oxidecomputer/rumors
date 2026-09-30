//! Traverses a Party as consecutive constant-ownership regions.
//!
//! Stored Party trees omit unowned children. This reader presents those absent
//! children as ordinary unowned regions, so overlay algorithms can advance a
//! Party and a Version by the same dyadic-boundary rule.

use crate::bits::stack::BitStack;
use crate::bits::{BitRead, BitsReader};
use crate::Party;

/// A forward reader over a Party's constant-ownership regions.
pub struct PartyRegionReader<'a> {
    /// The next stored Party node.
    bits: BitsReader<'a>,
    /// Root-to-current-region directions.
    path: BitStack,
    /// Whether each open branch has a stored right child.
    right_present: BitStack,
    /// Open left branches; zero exactly on the final region.
    open_lefts: u64,
    /// Whether the current region is owned.
    owned: bool,
}

impl<'a> PartyRegionReader<'a> {
    /// Open a Party at its leftmost region.
    pub fn new(party: &'a Party) -> Self {
        let mut reader = PartyRegionReader {
            bits: party.0.reader(),
            path: BitStack::new(),
            right_present: BitStack::new(),
            open_lefts: 0,
            owned: false,
        };
        reader.descend();
        reader
    }

    /// Whether the current region is owned.
    pub fn owned(&self) -> bool {
        self.owned
    }

    /// The current region's depth.
    pub fn depth(&self) -> u64 {
        self.path.len()
    }

    /// Whether the current region reaches the right edge of the Party space.
    pub fn done(&self) -> bool {
        self.open_lefts == 0
    }

    /// Advance to the next region and return their shared boundary depth.
    pub fn advance(&mut self) -> u64 {
        loop {
            match self.path.pop() {
                Some(true) => {
                    self.right_present.pop();
                }
                Some(false) => break,
                None => unreachable!("the final Party region is never advanced"),
            }
        }
        self.open_lefts -= 1;
        self.path.push(true);
        let boundary = self.path.len();
        if self
            .right_present
            .last()
            .expect("an open branch records its right child")
        {
            self.descend();
        } else {
            self.owned = false;
        }
        boundary
    }

    /// Descend through stored left children to the next region.
    fn descend(&mut self) {
        loop {
            let left = self.bits.read_bit().expect("canonical Party");
            let right = self.bits.read_bit().expect("canonical Party");
            if !left && !right {
                self.owned = true;
                return;
            }
            self.path.push(false);
            self.open_lefts += 1;
            self.right_present.push(right);
            if !left {
                self.owned = false;
                return;
            }
        }
    }
}
