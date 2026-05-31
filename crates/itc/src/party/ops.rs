//! Iterative id operations on the packed form (plan §2.1: ids have no working form,
//! so `split`/`sum`/`is_disjoint`/`compare` run directly on the `enc_id` bit stream).
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
//! Emptiness/fullness are `O(1)` leaf checks (see [`idbits`](crate::idbits)), valid because every
//! `Party` — and every subtree of one — is in canonical normal form.

use core::cmp::Ordering;

use crate::codec::{Bits, BitsSlice};
use crate::idbits::{header, is_empty, is_full, skip, IdHeader};

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

/// A step in a threaded two-tree DFS. `a_pos`/`b_pos` are bit offsets into the two
/// packed id streams.
enum Pair {
    /// Compare the subtrees rooted at these positions.
    Eval { a_pos: usize, b_pos: usize },
    /// The left child just finished; launch the right child from where it ended (both
    /// positions read from the `Ends` register).
    Right,
}

/// The thread register for the predicate walks ([`is_disjoint`], [`compare`]): the
/// position just past the most-recently-finished subtree in each input. An `Eval` arm
/// *writes* it when it decides a branch locally; a deferred `Right` frame *reads* it to
/// resume the sibling. (No payload — the predicates accumulate into a `bool` or the
/// `le`/`ge` pair, not here.)
#[derive(Clone, Copy, Default)]
struct Ends {
    /// Position just past the finished subtree in `a`.
    a_end: usize,
    /// Position just past the finished subtree in `b`.
    b_end: usize,
}

/// Whether two (normal-form) ids share no owned region. `O(n + m)`: both cursors are
/// threaded, and a side is skipped only where the other's leaf dominates it.
pub(crate) fn is_disjoint(a: &BitsSlice, b: &BitsSlice) -> bool {
    // A pending `Right` reads where its sibling begins from `ret`, without re-scanning.
    let mut ret = Ends::default();
    let mut stack = vec![Pair::Eval { a_pos: 0, b_pos: 0 }];
    while let Some(job) = stack.pop() {
        match job {
            Pair::Eval { a_pos, b_pos } => {
                let a_next = header(a, a_pos).next;
                let b_next = header(b, b_pos).next;
                if is_empty(a, a_pos) {
                    // a owns nothing here: disjoint
                    ret = Ends {
                        a_end: a_next,
                        b_end: skip(b, b_pos),
                    };
                } else if is_empty(b, b_pos) {
                    // b owns nothing here: disjoint
                    ret = Ends {
                        a_end: skip(a, a_pos),
                        b_end: b_next,
                    };
                } else if is_full(a, a_pos) {
                    return false; // a is full, b is nonempty: overlap
                } else if is_full(b, b_pos) {
                    return false; // b is full, a is nonempty: overlap
                } else {
                    stack.push(Pair::Right);
                    stack.push(Pair::Eval {
                        a_pos: a_next,
                        b_pos: b_next,
                    }); // left
                }
            }
            Pair::Right => {
                stack.push(Pair::Eval {
                    a_pos: ret.a_end,
                    b_pos: ret.b_end,
                });
            }
        }
    }
    true
}

/// The descent order on two normal-form ids, in a single `O(n + m)` pass. `Some(Less)`
/// means `a` is an ancestor of (contains) `b`; `Some(Greater)` the reverse; `Some(Equal)`
/// equal regions; `None` incomparable (cousins).
///
/// Tracks both containment directions together — `a ⊇ b` as `le` and `b ⊇ a` as `ge` —
/// so the two reverse-inclusion scans share one traversal instead of running `contains`
/// twice; the walk stops early once both are excluded. Only a both-node pair descends:
/// wherever at least one side is a leaf, that region's value (empty / full) settles both
/// directions locally, and the other side is skipped once to resync (bounded lazy-skip),
/// so each node is still visited at most once.
pub(crate) fn compare(a: &BitsSlice, b: &BitsSlice) -> Option<Ordering> {
    // Both ids are canonical normal form, so bit-equality is semantic equality: settle
    // `Equal` with one length-checked memcmp before allocating the traversal stack.
    // Differing lengths fail in O(1); only equal-length inputs pay the scan.
    if a == b {
        return Some(Ordering::Equal);
    }
    let mut le = true; // `a ⊇ b` (a is an ancestor of b) still possible
    let mut ge = true; // `b ⊇ a` still possible
    let mut ret = Ends::default();
    let mut stack = vec![Pair::Eval { a_pos: 0, b_pos: 0 }];
    while let Some(job) = stack.pop() {
        match job {
            Pair::Eval { a_pos, b_pos } => {
                let a_hdr = header(a, a_pos);
                let b_hdr = header(b, b_pos);
                let (a_node, a_next) = (a_hdr.node, a_hdr.next);
                let (b_node, b_next) = (b_hdr.node, b_hdr.next);
                if a_node && b_node {
                    // Both internal: descend in lockstep (left now, right threaded after).
                    stack.push(Pair::Right);
                    stack.push(Pair::Eval {
                        a_pos: a_next,
                        b_pos: b_next,
                    });
                    continue;
                }
                // At least one leaf: this region is decided. `a ⊇ b` holds iff `b` owns
                // nothing here or `a` owns everything; `b ⊇ a` is the mirror.
                le &= is_empty(b, b_pos) || is_full(a, a_pos);
                ge &= is_empty(a, a_pos) || is_full(b, b_pos);
                if !le && !ge {
                    return None; // incomparable: neither containment can recover
                }
                // Resync: advance the leaf side past its header, skip the node side once.
                ret = Ends {
                    a_end: if a_node { skip(a, a_pos) } else { a_next },
                    b_end: if b_node { skip(b, b_pos) } else { b_next },
                };
            }
            Pair::Right => {
                stack.push(Pair::Eval {
                    a_pos: ret.a_end,
                    b_pos: ret.b_end,
                });
            }
        }
    }
    match (le, ge) {
        (true, true) => Some(Ordering::Equal),
        (true, false) => Some(Ordering::Less),
        (false, true) => Some(Ordering::Greater),
        // Unreachable: both-false returns `None` inside the loop above.
        (false, false) => None,
    }
}

// ───────────────────────────── sum (disjoint union) ─────────────────────────────

/// A step in the threaded `sum` build. `a_pos`/`b_pos` are bit offsets into the two id
/// streams.
enum SumJob {
    /// Sum the subtrees rooted at these positions.
    Eval { a_pos: usize, b_pos: usize },
    /// Left finished; launch the right child from its end (read off the `results` stack).
    Right,
    /// Both children built; combine them into a normalized node.
    Combine,
}

/// A built `sum` subtree on the `results` stack — the register analogue for `sum` (see
/// the module doc, which notes `sum` needs an explicit stack because it folds child
/// *outputs*, not just positions): the subtree's `bits`, plus where it ended in each
/// input. `Eval` pushes one for a copied side; `Combine` pops two and pushes their
/// joined node; `Right` reads the top one's ends to launch the sibling.
struct Summed {
    /// The built (normalized) subtree's bits.
    bits: Bits,
    /// Position just past the subtree in `a`.
    a_end: usize,
    /// Position just past the subtree in `b`.
    b_end: usize,
}

/// Sum two normal-form ids — the union of their regions — producing a normalized id, or
/// `None` if they overlap (share a region, so no disjoint union exists). This is the
/// single point of overlap detection: callers (`Party::join`) need not pre-check
/// [`is_disjoint`], since a successful `sum` *is* the disjointness proof. `O(n + m)`:
/// the both-internal case threads (no skip); a `0` child copies the other subtree
/// verbatim (work bounded by the output size).
pub(crate) fn sum(a: &BitsSlice, b: &BitsSlice) -> Option<Bits> {
    let mut results: Vec<Summed> = Vec::new();
    let mut stack = vec![SumJob::Eval { a_pos: 0, b_pos: 0 }];
    while let Some(job) = stack.pop() {
        match job {
            SumJob::Eval { a_pos, b_pos } => {
                let a_hdr = header(a, a_pos);
                let b_hdr = header(b, b_pos);
                let (a_node, a_next) = (a_hdr.node, a_hdr.next);
                let (b_node, b_next) = (b_hdr.node, b_hdr.next);
                if is_empty(a, a_pos) {
                    let end = skip(b, b_pos); // sum(0, b) = b
                    results.push(Summed {
                        bits: b[b_pos..end].to_bitvec(),
                        a_end: a_next,
                        b_end: end,
                    });
                } else if is_empty(b, b_pos) {
                    let end = skip(a, a_pos); // sum(a, 0) = a
                    results.push(Summed {
                        bits: a[a_pos..end].to_bitvec(),
                        a_end: end,
                        b_end: b_next,
                    });
                } else if a_node && b_node {
                    stack.push(SumJob::Combine);
                    stack.push(SumJob::Right);
                    stack.push(SumJob::Eval {
                        a_pos: a_next,
                        b_pos: b_next,
                    }); // left
                } else {
                    // A `1` (full) leaf meets a nonempty subtree on the other side: the
                    // two ids share a region, so there is no disjoint union.
                    return None;
                }
            }
            SumJob::Right => {
                let left = results.last().expect("left result present");
                stack.push(SumJob::Eval {
                    a_pos: left.a_end,
                    b_pos: left.b_end,
                });
            }
            SumJob::Combine => {
                let right = results.pop().expect("right result present");
                let left = results.pop().expect("left result present");
                // The node's ends are the right child's (it was threaded last).
                results.push(Summed {
                    bits: id_node(&left.bits, &right.bits),
                    a_end: right.a_end,
                    b_end: right.b_end,
                });
            }
        }
    }
    Some(results.pop().expect("one result remains").bits)
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
    let root = header(bits, 0);
    if !root.node {
        return if root.val {
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
        /// An opened node whose left child is being parsed; `start` is its bit position.
        NeedLeft { start: usize },
        /// An opened node whose left child is done and right child is being parsed.
        NeedRight {
            /// The node's bit position.
            start: usize,
            /// Whether the (now-parsed) left child owned nothing.
            left_empty: bool,
            /// Bit position where the right child begins.
            right_start: usize,
        },
    }
    // The branch node `(start, left_start, right_start)`, and any `1` leaf (the branch
    // when the tree is a pure spine with no both-nonempty node).
    let mut branch: Option<(usize, usize, usize)> = None;
    let mut one_leaf: Option<usize> = None;
    let mut stack: Vec<Frame> = Vec::new();
    let mut pos = 0;
    // Two interleaved phases per outer iteration: phase A descends left to a leaf
    // (pushing `NeedLeft` frames); phase B (the inner `loop`) pops completed ancestors,
    // recording the shallowest both-nonempty node as the branch, until one still needs
    // its right child (then resume phase A there) or the stack empties (then build).
    loop {
        let IdHeader {
            node: is_node,
            val,
            next,
        } = header(bits, pos);
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
