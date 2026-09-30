//! Structural reading of a canonical Version tree.
//!
//! This is the low-level reader shared by algorithms that must inspect tree
//! topology rather than only visit constant-height regions. It contains the
//! bit reader and exposes Version concepts: nodes, payloads, and source ranges.

use num_bigint::BigUint;

use crate::bits::{BitRead, BitsReader};

use super::PayloadRange;
use crate::Version;

/// A node header in the Version tree.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VersionNode {
    /// An internal node followed by its left and right subtrees.
    Branch,
    /// A leaf followed by one height payload.
    Leaf,
}

/// A forward structural reader over one canonical Version.
pub struct VersionTreeReader<'a> {
    /// Source retained for ranges and constant-time seeking.
    source: &'a Version,
    /// Next unread topology bit or payload.
    bits: BitsReader<'a>,
}

impl<'a> VersionTreeReader<'a> {
    /// Start at the Version root.
    pub fn new(source: &'a Version) -> Self {
        Self {
            source,
            bits: source.0.reader(),
        }
    }

    /// Start at a known node position.
    pub fn at(source: &'a Version, position: u64) -> Self {
        Self {
            source,
            bits: BitsReader::at(&source.0, position),
        }
    }

    /// Position immediately after the last consumed bit.
    pub fn position(&self) -> u64 {
        self.bits.position()
    }

    /// Move to a known node position in the same Version.
    pub fn seek(&mut self, position: u64) {
        self.bits = BitsReader::at(&self.source.0, position);
    }

    /// Read one node header, leaving a leaf payload unread.
    pub fn node(&mut self) -> VersionNode {
        if self.bits.read_bit().expect("canonical Version") {
            VersionNode::Leaf
        } else {
            VersionNode::Branch
        }
    }

    /// Read a run of branches and its terminating leaf.
    pub fn descend_left(&mut self) -> u64 {
        self.bits.read_unary().expect("canonical Version")
    }

    /// Decode the payload at the current position.
    pub fn payload(&mut self) -> BigUint {
        self.bits.read_gamma().expect("canonical Version")
    }

    /// Skip the payload at the current position.
    pub fn skip_payload(&mut self) {
        self.bits.skip_gamma().expect("canonical Version");
    }

    /// Consume one payload and return its exact source range.
    pub fn payload_range(&mut self) -> PayloadRange<'a> {
        let start = self.position();
        self.skip_payload();
        PayloadRange::new(self.source, start..self.position())
    }

    /// The immutable source read by this reader.
    pub fn source(&self) -> &'a Version {
        self.source
    }
}
