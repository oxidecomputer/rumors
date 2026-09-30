//! Locates Version leaves and subtrees for zero-copy rewrites.
//!
//! Most Version algorithms need decoded heights. A structural rewrite instead
//! needs exact payload and subtree ranges so unchanged storage can be copied.
//! [`VersionSpliceReader`] provides that narrower interface without exposing bit
//! positions or codec readers to the rewriting algorithm.

use crate::version::io::tree::{VersionNode, VersionTreeReader};
use crate::version::io::PayloadRange;
use crate::Version;

mod subtree;

pub use subtree::VersionSubtree;

/// A forward reader over Version topology and payload ranges.
pub struct VersionSpliceReader<'a> {
    /// Source Version and current structural position.
    tree: VersionTreeReader<'a>,
}

impl<'a> VersionSpliceReader<'a> {
    /// Start at the Version root.
    pub fn new(version: &'a Version) -> Self {
        Self {
            tree: VersionTreeReader::new(version),
        }
    }

    /// Read one node, returning a leaf's payload range.
    pub fn leaf(&mut self) -> Option<PayloadRange<'a>> {
        match self.tree.node() {
            VersionNode::Branch => None,
            VersionNode::Leaf => Some(self.tree.payload_range()),
        }
    }

    /// Advance past one complete subtree.
    #[cfg(test)]
    pub fn skip_subtree(&mut self) {
        let mut pending = 1u64;
        while pending > 0 {
            let branches = self.tree.descend_left();
            self.tree.skip_payload();
            pending = pending + branches - 1;
        }
    }

    /// Consume and describe the complete subtree at the reader's position.
    pub fn subtree(&mut self) -> VersionSubtree<'a> {
        let subtree = VersionSubtree::scan(self.tree.source(), self.tree.position());
        self.tree.seek(subtree.end());
        subtree
    }

    /// Position immediately after the last consumed bit.
    pub fn position(&self) -> u64 {
        self.tree.position()
    }
}
