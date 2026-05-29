//! Iterative id operations on the packed form (plan §2.1: ids have no working form,
//! so `split`/`sum`/`is_disjoint`/`contains` run directly on the `enc_id` bit stream).
//!
//! `enc_id(Leaf v) = 0, v` (2 bits); `enc_id(Node l r) = 1, enc_id(l), enc_id(r)`.
//! Every traversal is iterative (explicit stack) and `O(n + m)` in its inputs — no
//! re-scan to find a right child. Two techniques achieve that:
//!
//! - **Threading.** A right child's position is *discovered* when the walk finishes
//!   the left subtree, not recomputed by skipping it. The DFS returns where each
//!   subtree ended; the sibling resumes there.
//! - **Bounded lazy-skip.** Where one side prunes early (a leaf dominates the other's
//!   whole subtree), the dominated subtree is skipped *once*, at the prune point, to
//!   resync the cursors. Each node is skipped at most once, so the total stays `O(n)`.
//!
//! Emptiness/fullness are `O(1)` leaf checks (see [`idbits`]), valid because every
//! `Party` — and every subtree of one — is in canonical normal form.

use crate::codec::{Bits, BitsSlice};
use crate::idbits::{header, skip};

/// A leaf id stream: `0, v`.
fn id_leaf(v: bool) -> Bits {
    let mut out = Bits::with_capacity(2);
    out.push(false);
    out.push(v);
    out
}

/// `norm((l, r))`: build a node, collapsing `(0,0) → 0` and `(1,1) → 1`. A 2-bit
/// stream is exactly a leaf, so equal-valued leaf children collapse.
fn id_node(l: &BitsSlice, r: &BitsSlice) -> Bits {
    if l.len() == 2 && r.len() == 2 && l[1] == r[1] {
        return id_leaf(l[1]);
    }
    let mut out = Bits::with_capacity(1 + l.len() + r.len());
    out.push(true);
    out.extend_from_bitslice(l);
    out.extend_from_bitslice(r);
    out
}

// ───────────────────────────── two-tree comparisons ─────────────────────────────

/// A step in a threaded two-tree DFS.
enum Pair {
    /// Compare the subtrees at these positions.
    Eval(usize, usize),
    /// The left child just finished; launch the right child from where it ended.
    Right,
}

/// Whether two (normal-form) ids share no owned region. `O(n + m)`: both cursors are
/// threaded, and a side is skipped only where the other's leaf dominates it.
pub(crate) fn is_disjoint(a: &BitsSlice, b: &BitsSlice) -> bool {
    // `ends` holds the (a, b) end positions of the most recently completed pair, so a
    // pending `Right` knows where its sibling begins without re-scanning.
    let mut ends = (0usize, 0usize);
    let mut stack = vec![Pair::Eval(0, 0)];
    while let Some(job) = stack.pop() {
        match job {
            Pair::Eval(ap, bp) => {
                let (a_node, a_val, a_next) = header(a, ap);
                let (b_node, b_val, b_next) = header(b, bp);
                if !a_node && !a_val {
                    ends = (a_next, skip(b, bp)); // a owns nothing here: disjoint
                } else if !b_node && !b_val {
                    ends = (skip(a, ap), b_next); // b owns nothing here: disjoint
                } else if !a_node {
                    return false; // a is full, b is nonempty: overlap
                } else if !b_node {
                    return false; // b is full, a is nonempty: overlap
                } else {
                    stack.push(Pair::Right);
                    stack.push(Pair::Eval(a_next, b_next)); // left
                }
            }
            Pair::Right => {
                let (a_left_end, b_left_end) = ends;
                stack.push(Pair::Eval(a_left_end, b_left_end));
            }
        }
    }
    true
}

/// Whether `a` owns every region `b` owns (reverse-inclusion: `a` contains `b`).
/// `O(n + m)`, by the same threading + bounded lazy-skip.
pub(crate) fn contains(a: &BitsSlice, b: &BitsSlice) -> bool {
    let mut ends = (0usize, 0usize);
    let mut stack = vec![Pair::Eval(0, 0)];
    while let Some(job) = stack.pop() {
        match job {
            Pair::Eval(ap, bp) => {
                let (a_node, a_val, a_next) = header(a, ap);
                let (b_node, b_val, b_next) = header(b, bp);
                if !b_node && !b_val {
                    ends = (skip(a, ap), b_next); // b owns nothing here: contained
                } else if !a_node && a_val {
                    ends = (a_next, skip(b, bp)); // a owns everything here: contains b
                } else if !a_node {
                    return false; // a empty, b nonempty: not contained
                } else if !b_node {
                    return false; // b full, a is a node (never full): not contained
                } else {
                    stack.push(Pair::Right);
                    stack.push(Pair::Eval(a_next, b_next)); // left
                }
            }
            Pair::Right => {
                let (a_left_end, b_left_end) = ends;
                stack.push(Pair::Eval(a_left_end, b_left_end));
            }
        }
    }
    true
}

// ───────────────────────────── sum (disjoint union) ─────────────────────────────

/// A step in the threaded `sum` build. Results carry `(bits, a_end, b_end)`.
enum SumJob {
    Eval(usize, usize),
    /// Left finished; launch the right child from its end.
    Right,
    /// Both children built; combine them into a normalized node.
    Combine,
}

/// Sum two (disjoint, normal-form) ids — the union of their regions — producing a
/// normalized id. `O(n + m)`: the both-internal case threads (no skip); a `0` child
/// copies the other subtree verbatim (work bounded by the output size). For disjoint
/// inputs the overlap fallback (`1`) is unreachable, matching the oracle.
pub(crate) fn sum(a: &BitsSlice, b: &BitsSlice) -> Bits {
    let mut results: Vec<(Bits, usize, usize)> = Vec::new();
    let mut stack = vec![SumJob::Eval(0, 0)];
    while let Some(job) = stack.pop() {
        match job {
            SumJob::Eval(ap, bp) => {
                let (a_node, a_val, a_next) = header(a, ap);
                let (b_node, b_val, b_next) = header(b, bp);
                if !a_node && !a_val {
                    let end = skip(b, bp); // sum(0, b) = b
                    results.push((b[bp..end].to_bitvec(), a_next, end));
                } else if !b_node && !b_val {
                    let end = skip(a, ap); // sum(a, 0) = a
                    results.push((a[ap..end].to_bitvec(), end, b_next));
                } else if a_node && b_node {
                    stack.push(SumJob::Combine);
                    stack.push(SumJob::Right);
                    stack.push(SumJob::Eval(a_next, b_next)); // left
                } else {
                    // Overlap (full vs nonempty): unreachable for disjoint inputs;
                    // mirror the oracle's saturating fallback.
                    results.push((id_leaf(true), skip(a, ap), skip(b, bp)));
                }
            }
            SumJob::Right => {
                let &(_, a_left_end, b_left_end) = results.last().expect("left result present");
                stack.push(SumJob::Eval(a_left_end, b_left_end));
            }
            SumJob::Combine => {
                let (right, a_end, b_end) = results.pop().expect("right result present");
                let (left, ..) = results.pop().expect("left result present");
                results.push((id_node(&left, &right), a_end, b_end));
            }
        }
    }
    results.pop().expect("one result remains").0
}

// ───────────────────────────── split ─────────────────────────────

/// Split an id into two non-overlapping ids that sum to it. `O(n)` in two passes:
/// locate the *branch* — the shallowest node along the (unique) nonempty spine whose
/// two children both own something, or the spine's terminal `1` leaf — then build both
/// halves by copying the input with one side of the branch zeroed.
///
/// The branch is the both-nonempty node of minimum start position (all shallower nodes
/// are spine wrappers, with one empty child), found by a single forward scan rather
/// than by descending and re-scanning to test each right child for emptiness.
pub(crate) fn split(bits: &BitsSlice) -> (Bits, Bits) {
    // A whole-tree leaf splits directly.
    let (root_node, root_val, _) = header(bits, 0);
    if !root_node {
        return if root_val {
            // split(1) = ((1, 0), (0, 1))
            (
                id_node(&id_leaf(true), &id_leaf(false)),
                id_node(&id_leaf(false), &id_leaf(true)),
            )
        } else {
            (id_leaf(false), id_leaf(false))
        };
    }

    // Pass 1: find the branch by a single forward preorder scan.
    enum Frame {
        NeedLeft {
            start: usize,
        },
        NeedRight {
            start: usize,
            left_empty: bool,
            right_start: usize,
        },
    }
    // The branch node `(start, left_start, right_start)`, and any `1` leaf (the branch
    // when the tree is a pure spine with no both-nonempty node).
    let mut branch: Option<(usize, usize, usize)> = None;
    let mut one_leaf: Option<usize> = None;
    let mut stack: Vec<Frame> = Vec::new();
    let mut pos = 0;
    loop {
        let (is_node, val, next) = header(bits, pos);
        let start = pos;
        pos = next;
        // What the just-parsed subtree reports to its parent: was it empty?
        let mut child_empty = if is_node {
            stack.push(Frame::NeedLeft { start });
            continue; // descend into the left child
        } else {
            if val {
                one_leaf.get_or_insert(start);
            }
            !val // a `0` leaf is empty; a `1` leaf is not
        };
        // Bubble the completed subtree up, completing ancestors as their turn comes.
        loop {
            match stack.pop() {
                None => return build_split(bits, branch, one_leaf),
                Some(Frame::NeedLeft { start }) => {
                    stack.push(Frame::NeedRight {
                        start,
                        left_empty: child_empty,
                        right_start: pos,
                    });
                    break; // parse the right child next
                }
                Some(Frame::NeedRight {
                    start,
                    left_empty,
                    right_start,
                }) => {
                    let both_nonempty = !left_empty && !child_empty;
                    if both_nonempty && branch.is_none_or(|(p, ..)| start < p) {
                        branch = Some((start, start + 1, right_start));
                    }
                    child_empty = false; // a normal-form node is never empty
                }
            }
        }
    }
}

/// Build the two split halves once the branch is located (see [`split`]). `a` keeps the
/// branch's left side (its right zeroed); `b` keeps the right side (its left zeroed).
fn build_split(
    bits: &BitsSlice,
    branch: Option<(usize, usize, usize)>,
    one_leaf: Option<usize>,
) -> (Bits, Bits) {
    let zero = id_leaf(false);
    if let Some((p, left_start, right_start)) = branch {
        // Branch is a node `(i1, i2)`: i1 = bits[left_start..right_start],
        // i2 = bits[right_start..branch_end], with the wrapper spine in the prefix
        // bits[0..p] and the trailing wrapper closings in bits[branch_end..].
        let branch_end = skip(bits, right_start);
        let prefix = &bits[0..p];
        let i1 = &bits[left_start..right_start];
        let i2 = &bits[right_start..branch_end];
        let suffix = &bits[branch_end..];

        let mut a = Bits::new();
        a.extend_from_bitslice(prefix);
        a.push(true); // the branch node, right child zeroed
        a.extend_from_bitslice(i1);
        a.extend_from_bitslice(&zero);
        a.extend_from_bitslice(suffix);

        let mut b = Bits::new();
        b.extend_from_bitslice(prefix);
        b.push(true); // the branch node, left child zeroed
        b.extend_from_bitslice(&zero);
        b.extend_from_bitslice(i2);
        b.extend_from_bitslice(suffix);

        (a, b)
    } else {
        // No both-nonempty node: the spine ends in a `1` leaf, split as (1,0)/(0,1).
        let p = one_leaf.expect("a nonempty id has a branch node or a 1 leaf");
        let prefix = &bits[0..p];
        let suffix = &bits[p + 2..]; // the `1` leaf occupies 2 bits
        let one = id_leaf(true);

        let mut a = Bits::new();
        a.extend_from_bitslice(prefix);
        a.extend_from_bitslice(&id_node(&one, &zero));
        a.extend_from_bitslice(suffix);

        let mut b = Bits::new();
        b.extend_from_bitslice(prefix);
        b.extend_from_bitslice(&id_node(&zero, &one));
        b.extend_from_bitslice(suffix);

        (a, b)
    }
}
