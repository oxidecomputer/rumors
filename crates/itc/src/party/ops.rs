//! Iterative id operations on the packed form (plan §2.1: ids have no working form,
//! so `split`/`sum`/`is_disjoint`/`contains` run directly on the `enc_id` bit stream).
//!
//! `enc_id(Leaf v) = 0, v` (2 bits); `enc_id(Node l r) = 1, l, r`. Every traversal
//! uses an explicit stack — no recursion on tree depth.

use crate::codec::{Bits, BitsSlice};

/// `(is_internal, leaf_value, position-just-past-this-node's-header)`. For a node the
/// header is the single flag bit and the left child begins at the returned position;
/// for a leaf the header is the flag plus its value bit.
fn header(bits: &BitsSlice, at: usize) -> (bool, bool, usize) {
    let is_node = bits[at];
    if is_node {
        (true, false, at + 1)
    } else {
        (false, bits[at + 1], at + 2)
    }
}

/// Position just past the whole subtree rooted at `at`. Iterative.
fn skip(bits: &BitsSlice, mut at: usize) -> usize {
    let mut pending: i64 = 1;
    while pending > 0 {
        let (is_node, _, next) = header(bits, at);
        at = next;
        pending += if is_node { 1 } else { -1 };
    }
    at
}

/// Whether the subtree at `pos` owns nothing (all leaves are `0`).
fn is_empty(bits: &BitsSlice, pos: usize) -> bool {
    let mut at = pos;
    let mut pending: i64 = 1;
    while pending > 0 {
        let (is_node, val, next) = header(bits, at);
        if !is_node && val {
            return false;
        }
        at = next;
        pending += if is_node { 1 } else { -1 };
    }
    true
}

/// Whether the subtree at `pos` owns everything (all leaves are `1`).
fn is_full(bits: &BitsSlice, pos: usize) -> bool {
    let mut at = pos;
    let mut pending: i64 = 1;
    while pending > 0 {
        let (is_node, val, next) = header(bits, at);
        if !is_node && !val {
            return false;
        }
        at = next;
        pending += if is_node { 1 } else { -1 };
    }
    true
}

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

/// Whether two ids share no owned region.
pub(crate) fn is_disjoint(a: &BitsSlice, b: &BitsSlice) -> bool {
    let mut stack: Vec<(usize, usize)> = vec![(0, 0)];
    while let Some((ap, bp)) = stack.pop() {
        let (a_node, a_val, a_next) = header(a, ap);
        let (b_node, b_val, b_next) = header(b, bp);
        if (!a_node && !a_val) || (!b_node && !b_val) {
            continue; // a `0` leaf is disjoint from anything
        } else if !a_node && a_val {
            if !is_empty(b, bp) {
                return false; // a is full here; b must own nothing
            }
        } else if !b_node && b_val {
            if !is_empty(a, ap) {
                return false;
            }
        } else {
            let (a_l, b_l) = (a_next, b_next);
            stack.push((a_l, b_l));
            stack.push((skip(a, a_l), skip(b, b_l)));
        }
    }
    true
}

/// Whether `a` owns every region `b` owns (reverse-inclusion: `a` contains `b`).
pub(crate) fn contains(a: &BitsSlice, b: &BitsSlice) -> bool {
    let mut stack: Vec<(usize, usize)> = vec![(0, 0)];
    while let Some((ap, bp)) = stack.pop() {
        let (a_node, a_val, a_next) = header(a, ap);
        let (b_node, b_val, b_next) = header(b, bp);
        if !b_node && !b_val {
            continue; // b owns nothing here
        } else if !a_node && a_val {
            continue; // a owns everything here
        } else if !a_node && !a_val {
            if !is_empty(b, bp) {
                return false; // a empty but b owns something
            }
        } else if !b_node && b_val {
            if !is_full(a, ap) {
                return false; // b full but a does not own all of it
            }
        } else {
            let (a_l, b_l) = (a_next, b_next);
            stack.push((a_l, b_l));
            stack.push((skip(a, a_l), skip(b, b_l)));
        }
    }
    true
}

/// Split an id into two non-overlapping ids that sum to it. Descends a single spine
/// (following the non-empty child when one side is empty) to the base case, then
/// rebuilds both halves on the way up — one descent, one ascent, both iterative.
pub(crate) fn split(bits: &BitsSlice) -> (Bits, Bits) {
    enum Wrap {
        /// Descended right (left was empty): rebuild as `node(0, child)`.
        Right,
        /// Descended left (right was empty): rebuild as `node(child, 0)`.
        Left,
    }
    let mut wraps: Vec<Wrap> = Vec::new();
    let mut pos = 0;
    let (mut a, mut b) = loop {
        let (is_node, val, next) = header(bits, pos);
        if !is_node {
            break if val {
                // split(1) = ((1,0), (0,1))
                (
                    id_node(&id_leaf(true), &id_leaf(false)),
                    id_node(&id_leaf(false), &id_leaf(true)),
                )
            } else {
                (id_leaf(false), id_leaf(false))
            };
        }
        let l = next;
        let r = skip(bits, l);
        let l_empty = is_empty(bits, l);
        let r_empty = is_empty(bits, r);
        if l_empty && r_empty {
            // A canonical node never has both children empty (it would collapse),
            // but treat it as the empty split for total coverage.
            break (id_leaf(false), id_leaf(false));
        } else if l_empty {
            wraps.push(Wrap::Right);
            pos = r;
        } else if r_empty {
            wraps.push(Wrap::Left);
            pos = l;
        } else {
            let l_sub = &bits[l..r];
            let r_sub = &bits[r..skip(bits, r)];
            break (
                id_node(l_sub, &id_leaf(false)),
                id_node(&id_leaf(false), r_sub),
            );
        }
    };
    while let Some(w) = wraps.pop() {
        match w {
            Wrap::Right => {
                a = id_node(&id_leaf(false), &a);
                b = id_node(&id_leaf(false), &b);
            }
            Wrap::Left => {
                a = id_node(&a, &id_leaf(false));
                b = id_node(&b, &id_leaf(false));
            }
        }
    }
    (a, b)
}

/// Sum two ids (the disjoint union of their regions), producing a normalized id.
/// Postorder evaluation on explicit task/result stacks. For disjoint inputs the
/// overlap fallback (`1`) is unreachable, matching the oracle.
pub(crate) fn sum(a: &BitsSlice, b: &BitsSlice) -> Bits {
    enum Task {
        /// Sum the subtrees at these positions.
        Sum(usize, usize),
        /// Pop two results and combine them with `id_node`.
        Combine,
    }
    let mut tasks: Vec<Task> = vec![Task::Sum(0, 0)];
    let mut results: Vec<Bits> = Vec::new();
    while let Some(task) = tasks.pop() {
        match task {
            Task::Sum(ap, bp) => {
                let (a_node, a_val, a_next) = header(a, ap);
                let (b_node, b_val, b_next) = header(b, bp);
                if !a_node && !a_val {
                    results.push(b[bp..skip(b, bp)].to_bitvec()); // sum(0, b) = b
                } else if !b_node && !b_val {
                    results.push(a[ap..skip(a, ap)].to_bitvec()); // sum(a, 0) = a
                } else if a_node && b_node {
                    let (a_l, b_l) = (a_next, b_next);
                    let (a_r, b_r) = (skip(a, a_l), skip(b, b_l));
                    tasks.push(Task::Combine);
                    tasks.push(Task::Sum(a_r, b_r));
                    tasks.push(Task::Sum(a_l, b_l));
                } else {
                    // Overlap (full vs anything non-empty): unreachable for disjoint
                    // inputs; mirror the oracle's saturating fallback.
                    results.push(id_leaf(true));
                }
            }
            Task::Combine => {
                // The left subtree was pushed first, so the right result is on top.
                let right = results.pop().expect("right result present");
                let left = results.pop().expect("left result present");
                results.push(id_node(&left, &right));
            }
        }
    }
    results.pop().expect("one result remains")
}
