//! Divides a party and selects or removes descendants of that division.
//!
//! A one-child branch locates ownership within the unit interval; it does not
//! offer two shares. A fork therefore preserves the initial one-child chain in
//! both results, then divides at the first two-child branch or owned terminal.
//! At a branch, each result retains one child. At a terminal, each result owns
//! one half of that terminal's interval.
//!
//! Repeated fork choices can be applied in one descent: the walk copies each
//! shared one-child chain and consumes a choice only where ownership divides.
//! Removing the resulting share instead takes a spatial path, with one
//! direction for every tree level, including those one-child chains. Both
//! walks avoid building intermediate parties; removal retains compact ancestor
//! state to rebuild the remainder.

mod remove;

use crate::codec::{extend_from_view, BitBuilder, BitsBuf, BitsView};
use crate::party::tree::PartyCursor;

use crate::party::Party;

impl PartyCursor<'_> {
    /// Select one descendant by a sequence of forks.
    ///
    /// Each `false` retains the original party after a fork; each `true`
    /// retains the returned party. These are the left and right shares at the
    /// first place ownership divides, which may lie below one-child branches.
    /// This produces the same bits as repeated [`Party::fork`] calls, but
    /// descends through the source once and builds only the final descendant.
    pub(crate) fn select_path(self, path: impl IntoIterator<Item = bool>) -> BitsBuf {
        let bits = self.bits();
        let pos = self.offset();
        let mut out = BitBuilder::with_capacity(0);
        let mut pos = pos;
        let mut path = path.into_iter();

        while let Some(right) = path.next() {
            loop {
                // A one-child tag is copied without consuming a fork choice;
                // a two-child tag or terminal consumes the current choice.
                crate::codec::scan::record_bits(2);
                match (bits.bit(pos), bits.bit(pos + 1)) {
                    (false, false) => {
                        // The selected path continues below an owned terminal.
                        // Each remaining choice creates one unary node, and one
                        // terminal closes the final selected region.
                        push_unary(&mut out, right);
                        for right in path {
                            push_unary(&mut out, right);
                        }
                        out.push_bit(false);
                        out.push_bit(false);
                        return out.finish();
                    }
                    (true, true) => {
                        let left = pos + 2;
                        push_unary(&mut out, right);
                        pos = if right {
                            // Preorder stores the complete left subtree before
                            // the right subtree, so the left subtree's end is
                            // the right child's root.
                            let mut left_subtree = PartyCursor::at(bits, left);
                            left_subtree.skip();
                            left_subtree.offset()
                        } else {
                            left
                        };
                        break;
                    }
                    (left_present, right_present) => {
                        // Both fork results inherit a unary prefix. Copy the
                        // tag and continue at its only child.
                        out.push_bit(left_present);
                        out.push_bit(right_present);
                        pos += 2;
                    }
                }
            }
        }

        // The requested path ends here, so the complete selected subtree is
        // already the desired result and can be copied unchanged.
        let mut selected = PartyCursor::at(bits, pos);
        selected.skip();
        out.splice(bits, pos, selected.offset());
        out.finish()
    }

    /// Remove all ownership in the subtree selected by a spatial path.
    ///
    /// Each direction crosses one tree level, including one-child branches.
    /// The path must follow present children, but may end at any subtree or
    /// continue below an owned terminal, where each direction divides its
    /// interval in half. Retained siblings are copied once, and the party
    /// builder normalizes the reconstructed ancestors. Compact frames keep
    /// auxiliary space proportional to the path and copied output.
    pub(crate) fn remove_path(
        self,
        path: impl IntoIterator<Item = bool>,
        output_capacity: u64,
    ) -> BitsBuf {
        remove::Removal::run(self.bits(), self.offset(), path, output_capacity)
    }
}

impl Party {
    /// Build the two halves of one complete canonical party tree.
    ///
    /// The common one-child prefix belongs in both halves, so the walk follows
    /// it to the first two-child node or terminal. A two-child node contributes
    /// one child to each result; a terminal is divided into a left terminal and
    /// a right terminal. Since every ancestor has only one child, this first
    /// fork point occupies the rest of the complete input tree. Its right
    /// child's end is therefore the end of the input.
    pub(in crate::party) fn fork_tree(bits: BitsView<'_>) -> (BitsBuf, BitsBuf) {
        if bits.is_empty() {
            return (BitsBuf::new(), BitsBuf::new());
        }
        /// What ends the initial one-child chain.
        enum ForkPoint {
            /// A two-child node, whose children become the two results.
            Branch,
            /// A terminal, which is divided into a left and right terminal.
            Terminal,
        }

        let mut pos = 0;
        let (prefix_end, fork_point) = loop {
            crate::codec::scan::record_bits(2);
            match (bits.bit(pos), bits.bit(pos + 1)) {
                (false, false) => break (pos, ForkPoint::Terminal),
                (true, true) => break (pos, ForkPoint::Branch),
                _ => pos += 2,
            }
        };

        if matches!(fork_point, ForkPoint::Branch) {
            // The left child occupies `left_child..right_child`; the right
            // child occupies the rest of this complete party.
            let left_child = prefix_end + 2;
            let mut left = PartyCursor::at(bits, left_child);
            left.skip();
            let right_child = left.offset();

            let mut keep = BitsBuf::with_capacity(prefix_end + 2 + (right_child - left_child));
            extend_from_view(&mut keep, bits, 0, prefix_end);
            keep.push(true);
            keep.push(false);
            extend_from_view(&mut keep, bits, left_child, right_child);

            let mut give = BitsBuf::with_capacity(prefix_end + 2 + (bits.len() - right_child));
            extend_from_view(&mut give, bits, 0, prefix_end);
            give.push(false);
            give.push(true);
            extend_from_view(&mut give, bits, right_child, bits.len());
            return (keep, give);
        }

        // Dividing a terminal creates one terminal in each half.
        let mut keep = BitsBuf::with_capacity(prefix_end + 4);
        extend_from_view(&mut keep, bits, 0, prefix_end);
        keep.push(true);
        keep.push(false);
        keep.push(false);
        keep.push(false);

        let mut give = BitsBuf::with_capacity(prefix_end + 4);
        extend_from_view(&mut give, bits, 0, prefix_end);
        give.push(false);
        give.push(true);
        give.push(false);
        give.push(false);

        (keep, give)
    }
}

/// Append a unary node retaining the selected child.
fn push_unary(out: &mut BitBuilder, right: bool) {
    out.push_bit(!right);
    out.push_bit(right);
}
