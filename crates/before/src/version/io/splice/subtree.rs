//! A scanned subtree that can be copied without decoding its interior.

use crate::version::io::regions::VersionSubtreeReader;
use crate::version::io::tree::VersionTreeReader;
use crate::version::io::writer::VersionWriter;
use crate::version::io::PayloadRange;
use crate::Version;

/// Source ranges and edge depths of one complete Version subtree.
pub struct VersionSubtree<'a> {
    /// Source Version.
    source: &'a Version,
    /// Position after the subtree.
    end: u64,
    /// First leaf's payload.
    first_payload: PayloadRange<'a>,
    /// First leaf's depth below the subtree root.
    first_depth: u64,
    /// Last leaf's payload length.
    last_payload_len: u64,
    /// Last leaf's depth below the subtree root.
    last_depth: u64,
}

impl<'a> VersionSubtree<'a> {
    /// The first leaf's payload range.
    pub fn first_payload(&self) -> PayloadRange<'a> {
        self.first_payload.clone()
    }

    /// The first leaf's depth below the subtree root.
    pub fn first_depth(&self) -> u64 {
        self.first_depth
    }

    /// Copy everything after the first leaf into `output`.
    ///
    /// A zero first depth means the subtree has only that leaf.
    pub fn copy_remainder(&self, output: &mut VersionWriter, root_depth: u64) {
        if self.first_depth == 0 {
            return;
        }
        let (_, first_range) = self.first_payload.clone().storage();
        output.copy_subtree_remainder(
            self.source,
            first_range.end,
            self.end,
            root_depth,
            self.first_depth,
            self.last_depth,
            self.last_payload_len,
        );
    }

    /// Locate one complete subtree and its boundary leaves.
    pub fn scan(source: &'a Version, start: u64) -> Self {
        let mut cursor = VersionTreeReader::at(source, start);
        let mut first = None;
        let mut last_payload_len = 0;
        let mut last_depth = 0;
        let mut walk = VersionSubtreeReader::new();

        while let Some(depth) = walk.descend(&mut cursor) {
            let payload = cursor.payload_range();
            last_payload_len = payload.len();
            last_depth = depth;
            first.get_or_insert((payload, depth));
        }

        let (first_payload, first_depth) = first.expect("a subtree has at least one leaf");
        Self {
            source,
            end: cursor.position(),
            first_payload,
            first_depth,
            last_payload_len,
            last_depth,
        }
    }

    /// Position immediately after the subtree.
    pub fn end(&self) -> u64 {
        self.end
    }
}
