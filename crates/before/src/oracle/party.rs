//! The oracle id component: [`Party`] as the paper's plain recursive tree.

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Party {
    Leaf(bool),
    Node(Box<Party>, Box<Party>),
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
            _ => Party::Node(Box::new(l), Box::new(r)),
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
            (Party::Node(l1, r1), Party::Node(l2, r2)) => {
                Party::node((*l1).sum(*l2), (*r1).sum(*r2))
            }
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

    pub fn is_disjoint(&self, other: &Party) -> bool {
        match (self, other) {
            (Party::Leaf(false), _) | (_, Party::Leaf(false)) => true,
            (Party::Leaf(true), x) | (x, Party::Leaf(true)) => x.is_empty(),
            (Party::Node(a1, a2), Party::Node(b1, b2)) => a1.is_disjoint(b1) && a2.is_disjoint(b2),
        }
    }

    /// Whether `self`'s owned region contains all of `other`'s (`self ⊇
    /// other`). The asymmetric companion of [`is_disjoint`](Self::is_disjoint):
    /// where disjointness asks whether two regions *share nothing*, this asks
    /// whether one region *subsumes* the other.
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
    /// does not own. May be the empty `Leaf(false)` (when `other` covers
    /// `self`). The reference for [`Party::without`](crate::Party::without),
    /// which maps that empty result to `None`.
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
