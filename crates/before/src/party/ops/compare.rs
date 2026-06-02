use core::cmp::Ordering;

use crate::idbits::IdView;

/// A step in the threaded [`is_disjoint`](IdView::is_disjoint) walk.
/// `a_pos`/`b_pos` are bit offsets into the two packed id streams.
enum DisjointJob {
    /// Test the subtrees rooted at these positions for shared ownership.
    Eval { a_pos: usize, b_pos: usize },
    /// The left child just finished; launch the right child from where it ended (both
    /// positions read from the `Ends` register).
    Right,
}

/// A step in the threaded [`compare`](IdView::compare) walk, the
/// containment-order analogue of [`DisjointJob`]. `a_pos`/`b_pos` are bit
/// offsets into the two packed id streams.
enum CompareJob {
    /// Compare the subtrees rooted at these positions.
    Eval { a_pos: usize, b_pos: usize },
    /// The left child just finished; launch the right child from where it ended (both
    /// positions read from the `Ends` register).
    Right,
}

/// The thread register for the predicate walks
/// ([`is_disjoint`](IdView::is_disjoint), [`compare`](IdView::compare)): the
/// position just past the most-recently-finished subtree in each input. An
/// `Eval` arm *writes* it when it decides a branch locally; a deferred `Right`
/// frame *reads* it to resume the sibling. (No payload — the predicates
/// accumulate into a `bool` or the `le`/`ge` pair, not here.)
#[derive(Clone, Copy, Default)]
struct Ends {
    /// Position just past the finished subtree in `a`.
    a_end: usize,
    /// Position just past the finished subtree in `b`.
    b_end: usize,
}

impl IdView<'_> {
    /// Whether `self` and `other` (normal-form ids) share no owned region. `O(n + m)`: both
    /// cursors are threaded, and a side is skipped only where the other's leaf dominates it.
    pub(crate) fn is_disjoint(&self, other: &IdView) -> bool {
        let (a, b) = (*self, *other);
        // A pending `Right` reads where its sibling begins from `ret`, without re-scanning.
        let mut ret = Ends::default();
        let mut stack = vec![DisjointJob::Eval { a_pos: 0, b_pos: 0 }];
        while let Some(job) = stack.pop() {
            match job {
                DisjointJob::Eval { a_pos, b_pos } => {
                    let a_hdr = a.header(a_pos);
                    let b_hdr = b.header(b_pos);
                    let a_next = a_hdr.next;
                    let b_next = b_hdr.next;
                    if a_hdr.is_empty() {
                        // a owns nothing here: disjoint
                        ret = Ends {
                            a_end: a_next,
                            b_end: b.skip(b_pos),
                        };
                    } else if b_hdr.is_empty() {
                        // b owns nothing here: disjoint
                        ret = Ends {
                            a_end: a.skip(a_pos),
                            b_end: b_next,
                        };
                    } else if a_hdr.is_full() {
                        return false; // a is full, b is nonempty: overlap
                    } else if b_hdr.is_full() {
                        return false; // b is full, a is nonempty: overlap
                    } else {
                        stack.push(DisjointJob::Right);
                        stack.push(DisjointJob::Eval {
                            a_pos: a_next,
                            b_pos: b_next,
                        }); // left
                    }
                }
                DisjointJob::Right => {
                    stack.push(DisjointJob::Eval {
                        a_pos: ret.a_end,
                        b_pos: ret.b_end,
                    });
                }
            }
        }
        true
    }

    /// The descent order on `self` and `other` (normal-form ids), in a single
    /// `O(n + m)` pass. `Some(Less)` means `self` is an ancestor of (its region
    /// contains) `other`; `Some(Greater)` the reverse; `Some(Equal)` equal
    /// regions; `None` incomparable (cousins).
    ///
    /// The iterative form of the recursive `oracle::Party::contains`, run in
    /// both directions at once. Tracks both containment directions together —
    /// `a ⊇ b` as `le` and `b ⊇ a` as `ge` — so the two reverse-inclusion scans
    /// share one traversal instead of running `contains` twice; the walk stops
    /// early once both are excluded. Only a both-node pair descends: wherever
    /// at least one side is a leaf, that region's value (empty / full) settles
    /// both directions locally, and the other side is skipped once to resync
    /// (bounded lazy-skip), so each node is still visited at most once.
    pub(crate) fn compare(&self, other: &IdView) -> Option<Ordering> {
        let (a, b) = (*self, *other);
        // Both ids are canonical normal form, so bit-equality is semantic
        // equality: settle `Equal` with one length-checked memcmp before
        // allocating the traversal stack. Differing lengths fail in O(1); only
        // equal-length inputs pay the scan.
        if a.bits() == b.bits() {
            return Some(Ordering::Equal);
        }
        let mut le = true; // `a ⊇ b` (a is an ancestor of b) still possible
        let mut ge = true; // `b ⊇ a` still possible
        let mut ret = Ends::default();
        let mut stack = vec![CompareJob::Eval { a_pos: 0, b_pos: 0 }];
        while let Some(job) = stack.pop() {
            match job {
                CompareJob::Eval { a_pos, b_pos } => {
                    let a_hdr = a.header(a_pos);
                    let b_hdr = b.header(b_pos);
                    let (a_node, a_next) = (a_hdr.node, a_hdr.next);
                    let (b_node, b_next) = (b_hdr.node, b_hdr.next);
                    if a_node && b_node {
                        // Both internal: descend in lockstep (left now, right threaded after).
                        stack.push(CompareJob::Right);
                        stack.push(CompareJob::Eval {
                            a_pos: a_next,
                            b_pos: b_next,
                        });
                        continue;
                    }
                    // At least one leaf: this region is decided. `a ⊇ b` holds iff `b` owns
                    // nothing here or `a` owns everything; `b ⊇ a` is the mirror.
                    le &= b_hdr.is_empty() || a_hdr.is_full();
                    ge &= a_hdr.is_empty() || b_hdr.is_full();
                    if !le && !ge {
                        return None; // incomparable: neither containment can recover
                    }
                    // Resync: advance the leaf side past its header, skip the node side once.
                    ret = Ends {
                        a_end: if a_node { a.skip(a_pos) } else { a_next },
                        b_end: if b_node { b.skip(b_pos) } else { b_next },
                    };
                }
                CompareJob::Right => {
                    stack.push(CompareJob::Eval {
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
            (false, false) => unreachable!("both-false returns `None` inside the loop above"),
        }
    }
}
