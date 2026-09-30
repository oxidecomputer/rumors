//! Removes ownership along a spatial path and rebuilds the remaining party.
//!
//! A path crosses one tree level per direction. It may stop at a branch, in
//! which case all ownership below that branch is removed, or continue below a
//! wholly owned region, whose interval is then divided as needed. Every direction
//! within the stored tree must select a present child, and the removal must
//! leave some ownership behind.
//!
//! A party tree is stored in preorder, so the two sides of the selected path
//! require different treatment. A retained left sibling precedes the selected
//! child and must be copied during descent. A retained right sibling follows
//! the selected child and cannot be reached until that child has been skipped.
//! The removal therefore has two phases:
//!
//! 1. Descend to the selected region, emitting preceding siblings and retaining
//!    enough information to reconstruct each parent.
//! 2. Omit the selected region, then unwind the parents while consuming the
//!    deferred right siblings now reached by the source cursor.
//!
//! If the path extends below an owned region, its remaining levels have no
//! stored nodes. Each such level retains an owned sibling beside the selected
//! half. Rebuilding treats these siblings exactly like stored owned regions.
//!
//! Each retained or removed subtree is traversed once. Compact frames retain
//! three bits per ancestor, and the writer compresses consecutive reserved
//! branches. Deep paths therefore remain iterative without allocating a Rust
//! frame for every level.

use crate::bits::stack::BitStack;
use crate::party::io::writer::{OpenBranch, PartyWriter, Positions, RegionKind};
use crate::party::io::{PartyNode, PartyReader};
use crate::party::Party;

/// The retained sibling of a selected child.
#[derive(Clone, Copy)]
enum Sibling {
    /// No sibling is owned.
    Unowned,
    /// The sibling is an owned region.
    Owned,
    /// The sibling is an already-emitted branch.
    Branch,
    /// The sibling follows the selected subtree in the source stream.
    Deferred,
}

/// Which child of an ancestor lies on the removal path.
#[derive(Clone, Copy)]
enum Selected {
    /// The selected region is the left child.
    Left,
    /// The selected region is the right child.
    Right,
}

impl Selected {
    /// Interpret `false` as left and `true` as right.
    fn from_right(right: bool) -> Self {
        if right {
            Self::Right
        } else {
            Self::Left
        }
    }

    /// Whether the selected region is the right child.
    fn is_right(self) -> bool {
        matches!(self, Self::Right)
    }
}

/// One ancestor restored while the removal unwinds.
struct Ancestor {
    /// The parent's reserved output tag.
    open: OpenBranch,
    /// The child containing the removed region.
    selected: Selected,
    /// The retained sibling's representation or location.
    sibling: Sibling,
}

/// Compact state for ancestors awaiting reconstruction.
struct Frames {
    /// Selected direction and sibling kind, three bits per ancestor.
    bits: BitStack,
    /// Reserved output tags, compressed across adjacent positions.
    opens: Positions,
}

impl Frames {
    /// Create an empty removal stack.
    fn new() -> Self {
        Self {
            bits: BitStack::new(),
            opens: Positions::new(),
        }
    }

    /// Retain one ancestor until its selected child has been removed.
    fn push(&mut self, open: OpenBranch, selected: Selected, sibling: Sibling) {
        let code = match sibling {
            Sibling::Unowned => 0,
            Sibling::Owned => 1,
            Sibling::Branch => 2,
            Sibling::Deferred => 3,
        };
        self.bits.push(selected.is_right());
        self.bits.push(code & 2 != 0);
        self.bits.push(code & 1 != 0);
        self.opens.push(open);
    }

    /// Restore the innermost ancestor.
    fn pop(&mut self) -> Option<Ancestor> {
        let low = self.bits.pop()?;
        let high = self
            .bits
            .pop()
            .expect("each removal frame carries three bits");
        let selected = self
            .bits
            .pop()
            .expect("each removal frame carries three bits");
        let sibling = match (high, low) {
            (false, false) => Sibling::Unowned,
            (false, true) => Sibling::Owned,
            (true, false) => Sibling::Branch,
            (true, true) => Sibling::Deferred,
        };
        Some(Ancestor {
            open: self.opens.pop(),
            selected: Selected::from_right(selected),
            sibling,
        })
    }
}

/// The state of one path removal.
pub struct Removal<'a> {
    /// The next unread source node.
    source: PartyReader<'a>,
    /// Canonical output assembled in preorder.
    output: PartyWriter,
    /// Parents awaiting the result of their selected child.
    ancestors: Frames,
    /// Whether the path has descended past a source owned region.
    below_owned_region: bool,
}

impl<'a> Removal<'a> {
    /// Remove `path` from `source` and return the canonical remainder.
    ///
    /// `source` covers the complete party tree. The path follows present
    /// children until it reaches an owned region; it may continue below that
    /// region by conceptually dividing it. It must leave some source ownership
    /// outside the selected region. Copied and skipped subtrees never overlap,
    /// so work is linear in the source, path, and result.
    pub fn run(
        source: PartyReader<'a>,
        path: impl IntoIterator<Item = bool>,
        size_hint: &Party,
    ) -> Party {
        let mut removal = Self {
            source,
            output: PartyWriter::sized_like(size_hint),
            ancestors: Frames::new(),
            below_owned_region: false,
        };

        // At the start of every iteration:
        //
        // - `source` is at the selected region, unless the path has continued
        //   below an owned region;
        // - `output` contains the retained siblings before that region, with
        //   reserved tags for ancestors whose final shape is not yet known;
        // - `ancestors` can reconstruct every open parent once the selected
        //   child has been removed.
        for right in path {
            removal.descend(Selected::from_right(right));
        }

        removal.omit_selected_region();
        removal.rebuild_ancestors()
    }

    /// Descend one spatial level, retaining the unselected sibling.
    fn descend(&mut self, selected: Selected) {
        // The selected child is not known until removal and normalization are
        // complete. Reserve its parent's tag and close that tag during unwind.
        let open = self.output.open_branch();

        if self.below_owned_region {
            self.retain_owned_sibling(open, selected);
            return;
        }

        let node = self.source.read();
        if matches!(node, PartyNode::Owned) {
            // Dividing an owned region creates two conceptual owned
            // children. They have no source tags; all later path choices stay
            // in this conceptual tree.
            self.below_owned_region = true;
            self.retain_owned_sibling(open, selected);
            return;
        }

        let PartyNode::Branch(branch) = node else {
            unreachable!("the owned-region case returned above")
        };
        match selected {
            Selected::Right => {
                assert!(branch.has_right_child(), "the removed region is owned");

                // Preorder places the left sibling before the selected right
                // child. Copying it now advances the source cursor to the right
                // child and leaves `output` complete up to that child.
                let sibling = if branch.has_left_child() {
                    let built = self.output.copy_next_subtree(&mut self.source);
                    match built {
                        RegionKind::Unowned => Sibling::Unowned,
                        RegionKind::Owned => Sibling::Owned,
                        RegionKind::Branch => Sibling::Branch,
                    }
                } else {
                    Sibling::Unowned
                };
                self.ancestors.push(open, selected, sibling);
            }
            Selected::Left => {
                assert!(branch.has_left_child(), "the removed region is owned");

                // The right sibling follows the selected left subtree in
                // preorder. Mark it deferred; omitting the selected subtree
                // will place the cursor at that sibling without another saved
                // position.
                let sibling = if branch.has_right_child() {
                    Sibling::Deferred
                } else {
                    Sibling::Unowned
                };
                self.ancestors.push(open, selected, sibling);
            }
        }
    }

    /// Retain the other half of a conceptual division below an owned region.
    fn retain_owned_sibling(&mut self, open: OpenBranch, selected: Selected) {
        if matches!(selected, Selected::Right) {
            // The retained left region precedes a selected right child.
            self.output.owned();
        }
        self.ancestors.push(open, selected, Sibling::Owned);
    }

    /// Advance past the selected source subtree without emitting it.
    fn omit_selected_region(&mut self) {
        if self.below_owned_region {
            // A conceptual descendant has no stored node to skip.
            return;
        }

        self.source.skip();
        // The cursor now points at the innermost deferred right sibling, if
        // one exists. Further deferred siblings follow in unwind order.
    }

    /// Close every open parent around the now-unowned selected region.
    fn rebuild_ancestors(mut self) -> Party {
        // `selected_result` describes what remains in the child on the path.
        // At the path's end nothing remains; each rebuilt parent then becomes
        // the selected child's result for the next ancestor.
        let mut selected_result = RegionKind::Unowned;

        while let Some(ancestor) = self.ancestors.pop() {
            let sibling = self.materialize_sibling(ancestor.selected, ancestor.sibling);
            selected_result = match ancestor.selected {
                Selected::Right => {
                    self.output
                        .close_branch(ancestor.open, sibling, selected_result)
                }
                Selected::Left => self
                    .output
                    .close_branch(ancestor.open, selected_result, sibling),
            };
        }

        debug_assert!(
            self.source.at_end(),
            "removing one region consumes the source"
        );
        self.output.finish()
    }

    /// Obtain the retained sibling while unwinding one ancestor.
    fn materialize_sibling(&mut self, selected: Selected, sibling: Sibling) -> RegionKind {
        match (selected, sibling) {
            (Selected::Left, Sibling::Deferred) => {
                // Deferred right siblings are contiguous in the source and
                // encountered from inner to outer, just like the frame stack.
                self.output.copy_next_subtree(&mut self.source)
            }
            (Selected::Left, Sibling::Owned) => self.output.owned(),
            (_, Sibling::Unowned) => RegionKind::Unowned,
            (Selected::Right, Sibling::Owned) => RegionKind::Owned,
            (Selected::Right, Sibling::Branch) => RegionKind::Branch,
            (Selected::Right, Sibling::Deferred) | (Selected::Left, Sibling::Branch) => {
                unreachable!("the sibling kind matches its selected direction")
            }
        }
    }
}
