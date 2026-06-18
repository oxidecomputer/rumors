//! The oracle event component: [`Version`] and its causal-order operators.

use std::cmp::Ordering;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Div, DivAssign};

use crate::codec::Base;

use super::Party;

type Cost = (u32, u32); // (#expansions, depth), lexicographic

/// Event component.
///
/// Bases are arbitrary-precision `Base` (`num_bigint::BigUint`), matching the
/// implementation's working form, so large-base differentials lower losslessly
/// — there is no `u64` truncation point. Literal/`u64` construction still works
/// via `Version::leaf`/`Version::node` (both take `impl Into<Base>`) and the
/// [`From<u64>`](Version) conversion.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Version {
    Leaf(Base),
    Node(Base, Box<Version>, Box<Version>),
}

impl From<u64> for Version {
    fn from(n: u64) -> Self {
        Version::Leaf(Base::from(n))
    }
}

impl Version {
    pub fn new() -> Self {
        Version::Leaf(Base::ZERO)
    }

    /// Build a leaf from any `u64`/`Base`. Keeps literal construction ergonomic now
    /// that the base is arbitrary-precision.
    pub(crate) fn leaf(n: impl Into<Base>) -> Version {
        Version::Leaf(n.into())
    }

    fn base(&self) -> &Base {
        match self {
            Version::Leaf(n) | Version::Node(n, ..) => n,
        }
    }

    fn max_ev(&self) -> Base {
        match self {
            Version::Leaf(n) => n.clone(),
            Version::Node(n, l, r) => n + l.max_ev().max(r.max_ev()),
        }
    }

    fn debase(self, m: &Base) -> Version {
        match self {
            Version::Leaf(n) => Version::Leaf(n - m),
            Version::Node(n, l, r) => Version::Node(n - m, l, r),
        }
    }

    /// `norm((n,l,r))`, assuming `l`,`r` already normal. `pub(crate)` so the
    /// test-support shape builders can construct normal-form event trees. Takes
    /// `impl Into<Base>` so callers can pass a `u64` literal.
    pub(crate) fn node(n: impl Into<Base>, l: Version, r: Version) -> Version {
        let n = n.into();
        let m = l.base().min(r.base()).clone();
        let l = l.debase(&m);
        let r = r.debase(&m);
        match (&l, &r) {
            (Version::Leaf(a), Version::Leaf(b)) if a == b => Version::Leaf(n + m + a),
            _ => Version::Node(n + m, Box::new(l), Box::new(r)),
        }
    }

    fn normalized(&self) -> Version {
        match self {
            Version::Leaf(n) => Version::Leaf(n.clone()),
            Version::Node(n, l, r) => Version::node(n.clone(), l.normalized(), r.normalized()),
        }
    }

    /// `self+so <= other+oo` pointwise (offset-threaded).
    pub(super) fn leq(&self, so: &Base, other: &Version, oo: &Base) -> bool {
        let sn = so + self.base();
        let on = oo + other.base();
        if sn > on {
            return false;
        }
        match self {
            Version::Leaf(_) => true,
            Version::Node(_, sl, sr) => match other {
                Version::Leaf(_) => sl.leq(&sn, other, oo) && sr.leq(&sn, other, oo),
                Version::Node(_, ol, or) => sl.leq(&sn, ol, &on) && sr.leq(&sn, or, &on),
            },
        }
    }

    /// Join (LUB) of two event trees, offset-threaded.
    fn join_off(&self, so: &Base, other: &Version, oo: &Base) -> Version {
        if let (Version::Leaf(sn), Version::Leaf(on)) = (self, other) {
            return Version::Leaf((so + sn).max(oo + on));
        }
        let sb = so + self.base();
        let ob = oo + other.base();
        let z = Version::new();
        let (sl, sr) = match self {
            Version::Node(_, l, r) => (l.as_ref(), r.as_ref()),
            _ => (&z, &z),
        };
        let (ol, or) = match other {
            Version::Node(_, l, r) => (l.as_ref(), r.as_ref()),
            _ => (&z, &z),
        };
        let l = sl.join_off(&sb, ol, &ob);
        let r = sr.join_off(&sb, or, &ob);
        Version::node(0u64, l, r)
    }

    /// Meet (GLB) of two event trees, offset-threaded.
    ///
    /// The order-theoretic dual of [`join_off`](Self::join_off): pointwise
    /// *minimum* in place of maximum, identical structure otherwise (a leaf
    /// still broadcasts as the constant `(n, 0, 0)` to both of the other side's
    /// children).
    fn meet_off(&self, so: &Base, other: &Version, oo: &Base) -> Version {
        if let (Version::Leaf(sn), Version::Leaf(on)) = (self, other) {
            return Version::Leaf((so + sn).min(oo + on));
        }
        let sb = so + self.base();
        let ob = oo + other.base();
        let z = Version::new();
        let (sl, sr) = match self {
            Version::Node(_, l, r) => (l.as_ref(), r.as_ref()),
            _ => (&z, &z),
        };
        let (ol, or) = match other {
            Version::Node(_, l, r) => (l.as_ref(), r.as_ref()),
            _ => (&z, &z),
        };
        let l = sl.meet_off(&sb, ol, &ob);
        let r = sr.meet_off(&sb, or, &ob);
        Version::node(0u64, l, r)
    }

    /// Lift this event tree into absolute value by adding `off` to its root base
    /// (every value in the subtree rises by `off`).
    ///
    /// Normal form is preserved — the children are untouched, so neither the
    /// one-zero-base nor the non-collapsible invariant can break.
    fn shift(&self, off: &Base) -> Version {
        match self {
            Version::Leaf(n) => Version::Leaf(n + off),
            Version::Node(n, l, r) => Version::Node(n + off, l.clone(), r.clone()),
        }
    }

    /// `project(id, self)` carrying the accumulated ancestor base `off`: keep
    /// the value where `id` owns the region (as an absolute value lifted by
    /// `off`), and zero it everywhere `id` does not own.
    ///
    /// Masking rebuilds from **absolute** values because a non-negative event
    /// base cannot be undone below a split — the same reason the impl's
    /// `project` threads its offset.
    fn project_off(&self, id: &Party, off: &Base) -> Version {
        match id {
            // Owned outright: keep the value, lifted to absolute by `off`.
            Party::Leaf(true) => self.shift(off),
            // Unowned: masked to zero, whatever the value is here.
            Party::Leaf(false) => Version::new(),
            // The id subdivides this region: push any event base down (into
            // `off`) so the masked side can still reach zero, then recurse and
            // let `node` renormalize.
            Party::Node(il, ir) => match self {
                // A constant `off + n` over a subdivided region: broadcast it to
                // both id children as a fresh zero event carrying the constant.
                Version::Leaf(n) => {
                    let val = off + n;
                    let z = Version::new();
                    Version::node(0u64, z.project_off(il, &val), z.project_off(ir, &val))
                }
                Version::Node(n, el, er) => {
                    let off2 = off + n;
                    Version::node(0u64, el.project_off(il, &off2), er.project_off(ir, &off2))
                }
            },
        }
    }

    /// `self / id` — the part of the version contributed within `id`'s region,
    /// zero everywhere `id` does not own. The reference for the impl's quotient
    /// [`Version / &Party`](crate::Version).
    fn project(&self, id: &Party) -> Version {
        self.project_off(id, &Base::ZERO)
    }

    /// The minimum number of [`tick`](Self::tick)s that could have produced this
    /// version: the sum of every base in the event tree, saturating at
    /// [`u64::MAX`]. The reference for
    /// [`Version::min_ticks`](crate::Version::min_ticks).
    pub fn min_ticks(&self) -> u64 {
        self.base_total().to_u64_saturating()
    }

    /// The sum of every base in the event tree (node bases plus leaf values).
    fn base_total(&self) -> Base {
        match self {
            Version::Leaf(n) => n.clone(),
            Version::Node(n, l, r) => n.clone() + l.base_total() + r.base_total(),
        }
    }

    /// The causal rank — the exact area under the event tree: a leaf is its
    /// base, a node is its base plus half the sum of its children. The
    /// reference for [`Version::rank`](crate::Version::rank).
    pub fn rank(&self) -> crate::Rank {
        let (num, exp) = self.rank_raw();
        crate::Rank::from_raw(num, exp)
    }

    /// The raw `(numerator, exponent)` area fold, in subtree-relative units
    /// (this subtree's interval has width 1).
    fn rank_raw(&self) -> (Base, u32) {
        match self {
            Version::Leaf(n) => (n.clone(), 0),
            Version::Node(n, l, r) => {
                let (l_num, l_exp) = l.rank_raw();
                let (r_num, r_exp) = r.rank_raw();
                let exp = l_exp.max(r_exp);
                let sum = (l_num << (exp - l_exp)) + (r_num << (exp - r_exp));
                ((n.clone() << (exp + 1)) + sum, exp + 1)
            }
        }
    }

    /// `fill(id, self)`.
    fn fill(&self, id: &Party) -> Version {
        match (id, self) {
            (Party::Leaf(false), _) => self.clone(),
            (Party::Leaf(true), _) => Version::Leaf(self.max_ev()),
            (Party::Node(..), Version::Leaf(n)) => Version::Leaf(n.clone()),
            (Party::Node(il, ir), Version::Node(n, el, er)) => {
                if il.is_full() {
                    let er2 = er.fill(ir);
                    let x = el.max_ev().max(er2.base().clone());
                    Version::node(n.clone(), Version::Leaf(x), er2)
                } else if ir.is_full() {
                    let el2 = el.fill(il);
                    let x = er.max_ev().max(el2.base().clone());
                    Version::node(n.clone(), el2, Version::Leaf(x))
                } else {
                    Version::node(n.clone(), el.fill(il), er.fill(ir))
                }
            }
        }
    }

    /// `grow(id, self)` → (tree, cost).
    fn grow(&self, id: &Party) -> (Version, Cost) {
        match (id, self) {
            (Party::Leaf(true), Version::Leaf(n)) => (Version::Leaf(n + 1u64), (0, 0)),
            (Party::Leaf(true), Version::Node(n, el, er)) => {
                let (el2, cl) = el.grow(&Party::Leaf(true));
                let (er2, cr) = er.grow(&Party::Leaf(true));
                if cl < cr {
                    (
                        Version::Node(n.clone(), Box::new(el2), er.clone()),
                        (cl.0, cl.1 + 1),
                    )
                } else {
                    (
                        Version::Node(n.clone(), el.clone(), Box::new(er2)),
                        (cr.0, cr.1 + 1),
                    )
                }
            }
            (Party::Leaf(false), _) => (self.clone(), (u32::MAX, u32::MAX)),
            (Party::Node(..), Version::Leaf(n)) => {
                let expanded = Version::Node(
                    n.clone(),
                    Box::new(Version::leaf(0u64)),
                    Box::new(Version::leaf(0u64)),
                );
                let (e2, c) = expanded.grow(id);
                (e2, (c.0 + 1, c.1))
            }
            (Party::Node(il, ir), Version::Node(n, el, er)) => {
                if il.is_empty() {
                    let (er2, cr) = er.grow(ir);
                    (
                        Version::Node(n.clone(), el.clone(), Box::new(er2)),
                        (cr.0, cr.1 + 1),
                    )
                } else if ir.is_empty() {
                    let (el2, cl) = el.grow(il);
                    (
                        Version::Node(n.clone(), Box::new(el2), er.clone()),
                        (cl.0, cl.1 + 1),
                    )
                } else {
                    let (el2, cl) = el.grow(il);
                    let (er2, cr) = er.grow(ir);
                    if cl < cr {
                        (
                            Version::Node(n.clone(), Box::new(el2), er.clone()),
                            (cl.0, cl.1 + 1),
                        )
                    } else {
                        (
                            Version::Node(n.clone(), el.clone(), Box::new(er2)),
                            (cr.0, cr.1 + 1),
                        )
                    }
                }
            }
        }
    }

    fn event(&self, id: &Party) -> Version {
        let filled = self.fill(id);
        if filled != *self {
            filled
        } else {
            let (grown, _) = self.grow(id);
            grown.normalized()
        }
    }

    /// `fill(id, self)` — `pub(crate)` so tests can detect when `event` takes
    /// the `grow` branch (`fill` left the tree unchanged) versus the `fill`
    /// branch.
    #[cfg(test)]
    pub(crate) fn fill_for_test(&self, id: &Party) -> Version {
        self.fill(id)
    }

    /// `grow(id, self)` → (raw tree, cost).
    ///
    /// `pub(crate)` so the grow-optimality tests can compare the DP's chosen
    /// inflation and its reported cost against the brute-force search
    /// (`testing::grow_brute_force::best_inflation`/`min_inflation_cost`).
    #[cfg(test)]
    pub(crate) fn grow_for_test(&self, id: &Party) -> (Version, (u32, u32)) {
        self.grow(id)
    }

    /// `norm(self)` — `pub(crate)` so tests can normalize a raw `grow` output
    /// before comparing it to `event`'s (normalized) result.
    #[cfg(test)]
    pub(crate) fn normalized_for_test(&self) -> Version {
        self.normalized()
    }

    pub fn tick(&mut self, party: &Party) {
        *self = self.event(party);
    }

    pub fn is_normal(&self) -> bool {
        match self {
            Version::Leaf(_) => true,
            Version::Node(_, l, r) => {
                let one_zero = *l.base() == Base::ZERO || *r.base() == Base::ZERO;
                let collapsible =
                    matches!((&**l, &**r), (Version::Leaf(a), Version::Leaf(b)) if a == b);
                one_zero && !collapsible && l.is_normal() && r.is_normal()
            }
        }
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialOrd for Version {
    /// Causal order; `None` means concurrent.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (
            self.leq(&Base::ZERO, other, &Base::ZERO),
            other.leq(&Base::ZERO, self, &Base::ZERO),
        ) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            (false, false) => None,
        }
    }
}

impl BitOr<Version> for Version {
    type Output = Version;
    fn bitor(self, rhs: Version) -> Version {
        self.join_off(&Base::ZERO, &rhs, &Base::ZERO)
    }
}

impl BitOrAssign<Version> for Version {
    fn bitor_assign(&mut self, rhs: Version) {
        *self = self.join_off(&Base::ZERO, &rhs, &Base::ZERO);
    }
}

// `&`/`&=` are the meet (GLB), the dual of `|`/`|=`, on `Version` only. They
// are deliberately *not* offered on `Clock`: the id (`Party`) component has no
// safe meet — intersecting two small disjoint shares could synthesize an
// ancestor they share with a third live party, violating disjoint linearity —
// so the meet lives purely on the event component.
impl BitAnd<Version> for Version {
    type Output = Version;
    fn bitand(self, rhs: Version) -> Version {
        self.meet_off(&Base::ZERO, &rhs, &Base::ZERO)
    }
}

impl BitAndAssign<Version> for Version {
    fn bitand_assign(&mut self, rhs: Version) {
        *self = self.meet_off(&Base::ZERO, &rhs, &Base::ZERO);
    }
}

// `/`/`/=` project a `Version` onto a `Party`'s region (the quotient),
// mirroring the impl's `Div<&Party> for Version`. Unlike the impl, the oracle
// `Party` is `Clone`, so the borrow is incidental — but the surface is kept
// identical (a borrowed party) so the operator reads the same in differential
// tests.
impl Div<&Party> for &Version {
    type Output = Version;
    fn div(self, party: &Party) -> Version {
        self.project(party)
    }
}

impl Div<&Party> for Version {
    type Output = Version;
    fn div(self, party: &Party) -> Version {
        self.project(party)
    }
}

impl DivAssign<&Party> for Version {
    fn div_assign(&mut self, party: &Party) {
        *self = self.project(party);
    }
}
