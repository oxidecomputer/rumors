//! Divides a party and selects or removes descendants of that division.
//!
//! A one-child branch locates ownership within the unit interval; it does not
//! offer two shares. A fork therefore preserves the initial one-child chain in
//! both results, then divides at the first two-child branch or wholly owned
//! region. At a branch, each result retains one child. A wholly owned region is
//! divided into owned left and right halves.
//!
//! Repeated fork choices can be applied in one descent: the walk copies each
//! shared one-child chain and consumes a choice only where ownership divides.
//! Removing the resulting share instead takes a spatial path, with one
//! direction for every tree level, including those one-child chains. Both
//! walks avoid building intermediate parties; removal retains compact ancestor
//! state to rebuild the remainder.

mod remove;

use crate::party::io::writer::PartyWriter;
use crate::party::io::{BranchChoice, ForkPoint, PartyBranch, PartyReader};

use crate::party::Party;

impl PartyReader<'_> {
    /// Select one descendant by a sequence of forks.
    ///
    /// Each `false` retains the original party after a fork; each `true`
    /// retains the returned party. These are the left and right shares at the
    /// first place ownership divides, which may lie below one-child branches.
    /// This produces the same party as repeated [`Party::fork`] calls, but
    /// descends through the source once and builds only the final descendant.
    pub fn select_path(mut self, path: impl IntoIterator<Item = bool>) -> Party {
        let mut out = PartyWriter::new();
        let mut path = path.into_iter();

        while let Some(right) = path.next() {
            let choice = BranchChoice::from_right(right);
            // One-child branches locate ownership but do not consume a fork
            // choice. Copy that whole path at once, then apply the choice at
            // the owned region or two-child branch where ownership divides.
            match self.next_fork() {
                ForkPoint::Owned(common_path) => {
                    out.copy_unary_path(common_path);
                    out.branch(PartyBranch::with_only_child(choice));
                    for right in path {
                        out.branch(PartyBranch::with_only_child(BranchChoice::from_right(
                            right,
                        )));
                    }
                    out.owned();
                    return out.finish();
                }
                ForkPoint::Children(common_path) => {
                    out.copy_unary_path(common_path);
                    out.branch(PartyBranch::with_only_child(choice));
                    if choice.is_right() {
                        // Preorder stores the complete left subtree before the
                        // right subtree, so skipping left lands on the selected
                        // right child.
                        self.skip();
                    }
                }
            }
        }

        // The requested path ends here, so the complete selected subtree is
        // already the desired result and can be copied unchanged.
        out.copy_next_subtree(&mut self);
        out.finish()
    }

    /// Remove all ownership in the subtree selected by a spatial path.
    ///
    /// This cursor must cover the complete source party. Each direction crosses
    /// one tree level, including one-child branches. The path must follow
    /// present children and leave some ownership outside the selected region.
    /// It may continue below an owned region, where each direction divides that
    /// region in half. Retained siblings are copied once, and the party builder
    /// normalizes the reconstructed ancestors. Compact frames keep auxiliary
    /// space proportional to the path and copied output.
    pub fn remove_path(self, path: impl IntoIterator<Item = bool>, size_hint: &Party) -> Party {
        remove::Removal::run(self, path, size_hint)
    }
}

impl Party {
    /// Build the two halves of one complete canonical party tree.
    ///
    /// The common one-child prefix belongs in both halves, so the walk follows
    /// it to the first two-child node or wholly owned region. A two-child node
    /// contributes one child to each result; a wholly owned region is divided
    /// into owned left and right halves. Every ancestor above that first fork
    /// point has only one child, so the fork point occupies the rest of the
    /// complete input tree. Its right child's end is therefore the end of the
    /// input.
    pub(crate) fn fork_tree(mut source: PartyReader<'_>) -> (Party, Party) {
        match source.next_fork() {
            ForkPoint::Owned(common_path) => {
                // The shared unary path and the newly divided terminal have
                // known exact sizes. Sizing each result here prevents a small
                // half from retaining storage proportional to the source.
                let output_bits = common_path.stored_len() + 4;
                let mut keep = PartyWriter::with_capacity(output_bits);
                let mut give = PartyWriter::with_capacity(output_bits);
                keep.copy_shared_unary_path(common_path, &mut give);
                // Splitting a fully owned region gives one half to each
                // result.
                keep.branch(PartyBranch::Left);
                keep.owned();
                give.branch(PartyBranch::Right);
                give.owned();
                (keep.finish(), give.finish())
            }
            ForkPoint::Children(common_path) => {
                // Delimit the left child once. The right child is the source
                // suffix, so both output sizes are then known without another
                // traversal.
                let (_, left) = source.take_subtree();
                let right = source.remainder();
                let prefix_bits = common_path.stored_len() + 2;
                let mut keep = PartyWriter::with_capacity(prefix_bits + left.stored_len());
                let mut give = PartyWriter::with_capacity(prefix_bits + right.stored_len());
                keep.copy_shared_unary_path(common_path, &mut give);
                // Each child is already one complete half of the union.
                keep.branch(PartyBranch::Left);
                keep.copy_subtree(left);
                give.branch(PartyBranch::Right);
                give.copy_subtree(right);
                (keep.finish(), give.finish())
            }
        }
    }
}
