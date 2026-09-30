//! Removes ownership along a spatial path and rebuilds the remaining party.
//!
//! A path crosses one tree level per direction. It may stop at a branch, in
//! which case all ownership below that branch is removed, or continue below an
//! owned terminal, whose interval is then divided as needed. Every direction
//! within the stored tree must select a present child.
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
//! If the path extends below a terminal, its remaining levels have no source
//! bytes. Each such level retains an owned sibling beside the selected half.
//! Rebuilding treats these siblings exactly like stored terminals.
//!
//! Each retained or removed subtree is traversed once. Three bits describe
//! each ancestor; reserved output positions are stored as runs and distances.
//! This keeps deep paths iterative, with storage proportional to the encoded
//! path and output rather than a machine-word frame for every level.

use crate::codec::{BitStack, BitsBuf, BitsView};
use crate::party::ops::build::{Builder, Built, Open, Positions};
use crate::party::tree::PartyCursor;

/// The retained sibling of a selected child.
#[derive(Clone, Copy)]
enum Sibling {
    /// No sibling is owned.
    Unowned,
    /// The sibling is an owned terminal.
    Terminal,
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
    open: Open,
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
    fn push(&mut self, open: Open, selected: Selected, sibling: Sibling) {
        let code = match sibling {
            Sibling::Unowned => 0,
            Sibling::Terminal => 1,
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
            (false, true) => Sibling::Terminal,
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
pub(super) struct Removal<'a> {
    /// The complete canonical source tree.
    source: BitsView<'a>,
    /// The next unread source tag.
    source_pos: u64,
    /// Canonical output assembled in preorder.
    output: Builder,
    /// Parents awaiting the result of their selected child.
    ancestors: Frames,
    /// Whether the path has descended past a source terminal.
    below_terminal: bool,
}

impl<'a> Removal<'a> {
    /// Remove `path` from `source` and return the canonical remainder.
    ///
    /// The source subtree begins at `start` and ends at `source.len()`. The
    /// path selects only present children until it reaches an owned terminal.
    /// Work is linear in source, path, and output size: copied and skipped
    /// source ranges never overlap.
    pub(super) fn run(
        source: BitsView<'a>,
        start: u64,
        path: impl IntoIterator<Item = bool>,
        output_capacity: u64,
    ) -> BitsBuf {
        let mut removal = Self {
            source,
            source_pos: start,
            output: Builder::with_capacity(output_capacity),
            ancestors: Frames::new(),
            below_terminal: false,
        };

        // At the start of every iteration:
        //
        // - `source_pos` is the root of the selected source region, unless the
        //   path has continued below a terminal;
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
        let open = self.output.open();

        if self.below_terminal {
            self.retain_terminal_sibling(open, selected);
            return;
        }

        crate::codec::scan::record_bits(2);
        let left_present = self.source.bit(self.source_pos);
        let right_present = self.source.bit(self.source_pos + 1);
        self.source_pos += 2;

        if !left_present && !right_present {
            // A fork below an owned terminal creates two conceptual owned
            // children. They have no source tags; all later path choices stay
            // in this conceptual tree.
            self.below_terminal = true;
            self.retain_terminal_sibling(open, selected);
            return;
        }

        match selected {
            Selected::Right => {
                assert!(right_present, "the removed region is owned");

                // Preorder places the left sibling before the selected right
                // child. Copying it now advances the source cursor to the right
                // child and leaves `output` complete up to that child.
                let sibling = if left_present {
                    let mut cursor = PartyCursor::at(self.source, self.source_pos);
                    let built = self.output.copy_cursor(&mut cursor);
                    self.source_pos = cursor.offset();
                    match built {
                        Built::Unowned => Sibling::Unowned,
                        Built::Owned => Sibling::Terminal,
                        Built::Branch => Sibling::Branch,
                    }
                } else {
                    Sibling::Unowned
                };
                self.ancestors.push(open, selected, sibling);
            }
            Selected::Left => {
                assert!(left_present, "the removed region is owned");

                // The selected left child starts at `source_pos`. Its right
                // sibling follows the whole selected subtree, so it cannot be
                // read yet. Marking it deferred avoids storing a second source
                // position: omitting the selected subtree lands on it exactly.
                let sibling = if right_present {
                    Sibling::Deferred
                } else {
                    Sibling::Unowned
                };
                self.ancestors.push(open, selected, sibling);
            }
        }
    }

    /// Retain the other half of a conceptual fork below an owned terminal.
    fn retain_terminal_sibling(&mut self, open: Open, selected: Selected) {
        if matches!(selected, Selected::Right) {
            // The retained left terminal precedes a selected right child.
            self.output.terminal();
        }
        self.ancestors.push(open, selected, Sibling::Terminal);
    }

    /// Advance past the selected source subtree without emitting it.
    fn omit_selected_region(&mut self) {
        if self.below_terminal {
            // A conceptual descendant has no source encoding to skip.
            return;
        }

        let mut removed = PartyCursor::at(self.source, self.source_pos);
        removed.skip();
        self.source_pos = removed.offset();
        // The cursor now points at the innermost deferred right sibling, if
        // one exists. Further deferred siblings follow in unwind order.
    }

    /// Close every open parent around the now-unowned selected region.
    fn rebuild_ancestors(mut self) -> BitsBuf {
        // `selected_result` describes what remains in the child on the path.
        // At the path's end nothing remains; each rebuilt parent then becomes
        // the selected child's result for the next ancestor.
        let mut selected_result = Built::Unowned;

        while let Some(ancestor) = self.ancestors.pop() {
            let sibling = self.materialize_sibling(ancestor.selected, ancestor.sibling);
            selected_result = match ancestor.selected {
                Selected::Right => self
                    .output
                    .close_node(ancestor.open, sibling, selected_result),
                Selected::Left => self
                    .output
                    .close_node(ancestor.open, selected_result, sibling),
            };
        }

        debug_assert_eq!(
            self.source_pos,
            self.source.len(),
            "removing one region consumes the source"
        );
        self.output.finish()
    }

    /// Obtain the retained sibling while unwinding one ancestor.
    fn materialize_sibling(&mut self, selected: Selected, sibling: Sibling) -> Built {
        match (selected, sibling) {
            (Selected::Left, Sibling::Deferred) => {
                // Deferred right siblings are contiguous in the source and
                // encountered from inner to outer, just like the frame stack.
                let mut cursor = PartyCursor::at(self.source, self.source_pos);
                let built = self.output.copy_cursor(&mut cursor);
                self.source_pos = cursor.offset();
                built
            }
            (Selected::Left, Sibling::Terminal) => self.output.terminal(),
            (_, Sibling::Unowned) => Built::Unowned,
            (Selected::Right, Sibling::Terminal) => Built::Owned,
            (Selected::Right, Sibling::Branch) => Built::Branch,
            (Selected::Right, Sibling::Deferred) | (Selected::Left, Sibling::Branch) => {
                unreachable!("the sibling kind matches its selected direction")
            }
        }
    }
}
