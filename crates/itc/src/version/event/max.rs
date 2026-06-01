use crate::codec::Base;

use crate::version::compare::{EvHeader, EvView};

/// A step in the [`EvView::max`] descent, in the dominant job-stack idiom. The thread
/// register is just the end position of the last-finished subtree (`end` in the loop):
/// this fold has no per-node output to combine, only a global maximum, so it needs no
/// register struct and `Eval`/`Right` alone (no `Close`) suffice.
enum MaxJob {
    /// Accumulate the subtree at `pos`, whose root-to-parent path sum is `off`.
    Eval { pos: usize, off: Base },
    /// Left child finished (its end is in the thread register); launch the right child
    /// under this node's path sum `off`.
    Right { off: Base },
}

impl EvView<'_> {
    /// The maximum value of the event function over the subtree at `root` (the paper's
    /// `max`: `base + max(child maxes)`), and the position just past the subtree.
    /// Iterative `O(n)` pass in the crate's dominant job-stack idiom: the threaded `end`
    /// reports where each subtree finished so a right sibling resumes there without
    /// re-scanning, while the running `max` accumulates every node's path sum.
    pub(super) fn max(&self, root: usize) -> (Base, usize) {
        let mut max = Base::ZERO;
        let mut end = root;
        let mut stack = vec![MaxJob::Eval {
            pos: root,
            off: Base::ZERO,
        }];
        while let Some(job) = stack.pop() {
            match job {
                MaxJob::Eval { pos, off } => {
                    let EvHeader {
                        internal,
                        base,
                        next,
                    } = self.header(pos);
                    let cumulative = off + base;
                    max = max.max(cumulative.clone());
                    if internal {
                        // Descend the left child; defer the right under this node's sum.
                        // (LIFO: when `Right` pops, `end` is exactly the left subtree's end.)
                        stack.push(MaxJob::Right {
                            off: cumulative.clone(),
                        });
                        stack.push(MaxJob::Eval {
                            pos: next,
                            off: cumulative,
                        });
                    } else {
                        end = next;
                    }
                }
                MaxJob::Right { off } => {
                    stack.push(MaxJob::Eval { pos: end, off });
                }
            }
        }
        (max, end)
    }
}
