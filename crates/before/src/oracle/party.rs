//! The oracle id component: [`Party`] as the paper's plain recursive tree.

use std::sync::Arc;

/// An id tree, exactly as the paper defines it.
///
/// Children sit behind [`Arc`] so the derived [`Clone`] is a refcount bump:
/// the paper's subtree-preserving cases (`split` handing each half of a
/// two-sided node to one fork, whole) share structure instead of
/// deep-copying, which keeps every oracle walk linear in the tree it visits.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Party {
    Leaf(bool),
    Node(Arc<Party>, Arc<Party>),
}

impl Party {
    pub fn seed() -> Self {
        Party::Leaf(true)
    }

    // `pub(crate)` so the test-support shape builders can construct normal-form ids.
    pub(crate) fn node(l: Party, r: Party) -> Party {
        match (&l, &r) {
            (Party::Leaf(false), Party::Leaf(false)) => Party::Leaf(false),
            (Party::Leaf(true), Party::Leaf(true)) => Party::Leaf(true),
            _ => Party::Node(Arc::new(l), Arc::new(r)),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        match self {
            Party::Leaf(b) => !*b,
            Party::Node(l, r) => l.is_empty() && r.is_empty(),
        }
    }

    pub(super) fn is_full(&self) -> bool {
        match self {
            Party::Leaf(b) => *b,
            Party::Node(l, r) => l.is_full() && r.is_full(),
        }
    }

    pub(super) fn split(&self) -> (Party, Party) {
        match self {
            Party::Leaf(false) => (Party::Leaf(false), Party::Leaf(false)),
            Party::Leaf(true) => (
                Party::node(Party::Leaf(true), Party::Leaf(false)),
                Party::node(Party::Leaf(false), Party::Leaf(true)),
            ),
            Party::Node(l, r) => {
                if l.is_empty() {
                    let (a, b) = r.split();
                    (
                        Party::node(Party::Leaf(false), a),
                        Party::node(Party::Leaf(false), b),
                    )
                } else if r.is_empty() {
                    let (a, b) = l.split();
                    (
                        Party::node(a, Party::Leaf(false)),
                        Party::node(b, Party::Leaf(false)),
                    )
                } else {
                    (
                        Party::node((**l).clone(), Party::Leaf(false)),
                        Party::node(Party::Leaf(false), (**r).clone()),
                    )
                }
            }
        }
    }

    pub(super) fn sum(self, other: Party) -> Party {
        match (self, other) {
            (Party::Leaf(false), b) => b,
            (a, Party::Leaf(false)) => a,
            (Party::Node(l1, r1), Party::Node(l2, r2)) => Party::node(
                Arc::unwrap_or_clone(l1).sum(Arc::unwrap_or_clone(l2)),
                Arc::unwrap_or_clone(r1).sum(Arc::unwrap_or_clone(r2)),
            ),
            _ => Party::Leaf(true), // overlap: unreachable (callers check disjointness)
        }
    }

    pub fn fork(&mut self) -> Party {
        let (a, b) = self.split();
        *self = a;
        b
    }

    pub fn join(&mut self, other: Party) -> Result<(), Party> {
        if !self.is_disjoint(&other) {
            return Err(other);
        }
        let mine = std::mem::replace(self, Party::Leaf(false));
        *self = mine.sum(other);
        Ok(())
    }

    /// Fold every disjoint [`Party`] in `inputs` into `self` — the reference
    /// for [`Party::join_all`](crate::Party::join_all), hand-back vector
    /// (contents *and* order) and final accumulator value included.
    ///
    /// The contract's granularity, spelled exactly: each input is tested up
    /// front against the **fixed** `self` — never against the running union;
    /// `self` does not change until the final joins — and an overlapping
    /// input is handed back untouched. Accepted inputs coalesce in
    /// binary-counter groups (an incoming operand merges upward while the
    /// top group holds as many inputs as it does); a collision at a merge
    /// hands back a lone incoming input and leaves an already-coalesced
    /// group on the stack unmerged. The surviving groups join into `self`
    /// at the end, any group colliding there handed back as one party.
    pub fn join_all(&mut self, inputs: impl IntoIterator<Item = Party>) -> Result<(), Vec<Party>> {
        let mut overlapping = Vec::new();
        let mut stack: Vec<(Party, u32)> = Vec::new();
        for other in inputs {
            if !self.is_disjoint(&other) {
                overlapping.push(other);
                continue;
            }
            let mut merged = Some(other);
            let mut weight = 0u32;
            while stack.last().is_some_and(|(_, w)| *w == weight) {
                let (mut top, _) = stack.pop().expect("the loop condition saw a top entry");
                match top.join(merged.take().expect("the operand is held while merging up")) {
                    Ok(()) => {
                        merged = Some(top);
                        weight += 1;
                    }
                    Err(back) => {
                        stack.push((top, weight));
                        if weight == 0 {
                            overlapping.push(back);
                        } else {
                            stack.push((back, weight));
                        }
                        break;
                    }
                }
            }
            if let Some(merged) = merged {
                stack.push((merged, weight));
            }
        }
        for (group, _) in stack {
            if let Err(back) = self.join(group) {
                overlapping.push(back);
            }
        }
        if overlapping.is_empty() {
            Ok(())
        } else {
            Err(overlapping)
        }
    }

    pub fn is_disjoint(&self, other: &Party) -> bool {
        match (self, other) {
            (Party::Leaf(false), _) | (_, Party::Leaf(false)) => true,
            (Party::Leaf(true), x) | (x, Party::Leaf(true)) => x.is_empty(),
            (Party::Node(a1, a2), Party::Node(b1, b2)) => a1.is_disjoint(b1) && a2.is_disjoint(b2),
        }
    }

    /// Whether `self`'s owned region contains all of `other`'s (`self ⊇
    /// other`).
    ///
    /// The asymmetric companion of [`is_disjoint`](Self::is_disjoint): where
    /// disjointness asks whether two regions *share nothing*, this asks whether
    /// one region *subsumes* the other.
    pub fn covers(&self, other: &Party) -> bool {
        match (self, other) {
            // Nothing to cover: every region contains the empty region.
            (_, Party::Leaf(false)) => true,
            // Owns everything: the full region contains any other.
            (Party::Leaf(true), _) => true,
            // Owns nothing yet `other` owns something: not covered.
            (Party::Leaf(false), x) => x.is_empty(),
            // `other` owns the whole region here; `self` must own it all too.
            (x, Party::Leaf(true)) => x.is_full(),
            // Both internal: cover holds iff it holds on both halves.
            (Party::Node(a1, a2), Party::Node(b1, b2)) => a1.covers(b1) && a2.covers(b2),
        }
    }

    /// The region complement `1 \ self`: the share `self` does *not* own. Flips
    /// each leaf and recurses; `node` renormalizes (a complemented normal tree
    /// is already normal).
    fn complement(&self) -> Party {
        match self {
            Party::Leaf(b) => Party::Leaf(!*b),
            Party::Node(l, r) => Party::node(l.complement(), r.complement()),
        }
    }

    /// The region difference `self \ other`: the part of `self` that `other`
    /// does not own.
    ///
    /// May be the empty `Leaf(false)` (when `other` covers `self`). The reference
    /// for [`Party::without`](crate::Party::without), which maps that empty result
    /// to `None`.
    pub fn without(&self, other: &Party) -> Party {
        match (self, other) {
            // diff(0, _) = 0 and diff(_, 1) = 0: nothing of `self` survives.
            (Party::Leaf(false), _) | (_, Party::Leaf(true)) => Party::Leaf(false),
            // diff(a, 0) = a: `other` owns nothing here.
            (a, Party::Leaf(false)) => a.clone(),
            // diff(1, b) = complement(b): `self` owns everything `b` lacks.
            (Party::Leaf(true), b) => b.complement(),
            (Party::Node(a1, a2), Party::Node(b1, b2)) => {
                Party::node(a1.without(b1), a2.without(b2))
            }
        }
    }

    pub fn is_normal(&self) -> bool {
        match self {
            Party::Leaf(_) => true,
            Party::Node(l, r) => {
                let collapsible =
                    matches!((&**l, &**r), (Party::Leaf(a), Party::Leaf(b)) if a == b);
                !collapsible && l.is_normal() && r.is_normal()
            }
        }
    }
}
