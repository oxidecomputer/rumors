//! Reference oracle — the paper's trees as plain recursive enums.
//!
//! `Party` and `Version` *are* the trees; every operation is a method, so there is no
//! second representation to keep in sync. Deliberately simple, suboptimal, and
//! recursive: its only job is to be obviously correct, so it can serve as differential
//! ground truth. It mirrors the target's **semantic** surface (construction,
//! operations, ordering, operators) and omits the two purely *representational*
//! concerns that carry no semantics: the byte codec (`encode`/`decode`) and the batch
//! optimization (a batch only ever equals its value-level ops). Bounded-depth use only
//! — the deep-tree stack-safety test runs against the impl, never the oracle.
//!
//! All three types derive `Clone`: a reference oracle needs cheap snapshots of "before"
//! states for the property checks, and linearity (`!Clone` on `Party`/`Clock`) is a
//! *type-level* guarantee checked against `itc` by compile-fail tests — not a runtime
//! semantic the differential harness exercises.

#![allow(dead_code)] // Full semantic surface; some methods are used only by later phases.

use std::cmp::Ordering;
use std::ops::{BitOr, BitOrAssign};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct OverlapError;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Party {
    Leaf(bool),
    Node(Box<Party>, Box<Party>),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Version {
    Leaf(u64),
    Node(u64, Box<Version>, Box<Version>),
}

#[derive(Clone, Debug)]
pub struct Clock {
    party: Party,
    version: Version,
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

    fn is_empty(&self) -> bool {
        match self {
            Party::Leaf(b) => !*b,
            Party::Node(l, r) => l.is_empty() && r.is_empty(),
        }
    }

    fn is_full(&self) -> bool {
        match self {
            Party::Leaf(b) => *b,
            Party::Node(l, r) => l.is_full() && r.is_full(),
        }
    }

    fn split(&self) -> (Party, Party) {
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

    fn sum(self, other: Party) -> Party {
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

    fn contains(&self, other: &Party) -> bool {
        match (self, other) {
            (_, Party::Leaf(false)) => true,
            (Party::Leaf(true), _) => true,
            (Party::Leaf(false), x) => x.is_empty(),
            (x, Party::Leaf(true)) => x.is_full(),
            (Party::Node(a1, a2), Party::Node(b1, b2)) => a1.contains(b1) && a2.contains(b2),
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

impl PartialOrd for Party {
    /// Descent: an ancestor (larger region) is *less than* its forked descendants.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self.contains(other), other.contains(self)) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            (false, false) => None,
        }
    }
}

type Cost = (u32, u32); // (#expansions, depth), lexicographic

impl Version {
    pub fn new() -> Self {
        Version::Leaf(0)
    }

    fn base(&self) -> u64 {
        match self {
            Version::Leaf(n) | Version::Node(n, ..) => *n,
        }
    }

    fn max_ev(&self) -> u64 {
        match self {
            Version::Leaf(n) => *n,
            Version::Node(n, l, r) => *n + l.max_ev().max(r.max_ev()),
        }
    }

    fn debase(self, m: u64) -> Version {
        match self {
            Version::Leaf(n) => Version::Leaf(n - m),
            Version::Node(n, l, r) => Version::Node(n - m, l, r),
        }
    }

    /// `norm((n,l,r))`, assuming `l`,`r` already normal. `pub(crate)` so the
    /// test-support shape builders can construct normal-form event trees.
    pub(crate) fn node(n: u64, l: Version, r: Version) -> Version {
        let m = l.base().min(r.base());
        let l = l.debase(m);
        let r = r.debase(m);
        match (&l, &r) {
            (Version::Leaf(a), Version::Leaf(b)) if a == b => Version::Leaf(n + m + *a),
            _ => Version::Node(n + m, Box::new(l), Box::new(r)),
        }
    }

    fn normalized(&self) -> Version {
        match self {
            Version::Leaf(n) => Version::Leaf(*n),
            Version::Node(n, l, r) => Version::node(*n, l.normalized(), r.normalized()),
        }
    }

    /// `self+so <= other+oo` pointwise (offset-threaded).
    fn leq(&self, so: u64, other: &Version, oo: u64) -> bool {
        let sn = so + self.base();
        let on = oo + other.base();
        if sn > on {
            return false;
        }
        match self {
            Version::Leaf(_) => true,
            Version::Node(_, sl, sr) => match other {
                Version::Leaf(_) => sl.leq(sn, other, oo) && sr.leq(sn, other, oo),
                Version::Node(_, ol, or) => sl.leq(sn, ol, on) && sr.leq(sn, or, on),
            },
        }
    }

    /// Join (LUB) of two event trees, offset-threaded.
    fn join_off(&self, so: u64, other: &Version, oo: u64) -> Version {
        if let (Version::Leaf(sn), Version::Leaf(on)) = (self, other) {
            return Version::Leaf((so + *sn).max(oo + *on));
        }
        let sb = so + self.base();
        let ob = oo + other.base();
        let z = Version::Leaf(0);
        let (sl, sr) = match self {
            Version::Node(_, l, r) => (l.as_ref(), r.as_ref()),
            _ => (&z, &z),
        };
        let (ol, or) = match other {
            Version::Node(_, l, r) => (l.as_ref(), r.as_ref()),
            _ => (&z, &z),
        };
        let l = sl.join_off(sb, ol, ob);
        let r = sr.join_off(sb, or, ob);
        Version::node(0, l, r)
    }

    /// `fill(id, self)`.
    fn fill(&self, id: &Party) -> Version {
        match (id, self) {
            (Party::Leaf(false), _) => self.clone(),
            (Party::Leaf(true), _) => Version::Leaf(self.max_ev()),
            (Party::Node(..), Version::Leaf(n)) => Version::Leaf(*n),
            (Party::Node(il, ir), Version::Node(n, el, er)) => {
                if il.is_full() {
                    let er2 = er.fill(ir);
                    let x = el.max_ev().max(er2.base());
                    Version::node(*n, Version::Leaf(x), er2)
                } else if ir.is_full() {
                    let el2 = el.fill(il);
                    let x = er.max_ev().max(el2.base());
                    Version::node(*n, el2, Version::Leaf(x))
                } else {
                    Version::node(*n, el.fill(il), er.fill(ir))
                }
            }
        }
    }

    /// `grow(id, self)` → (tree, cost).
    fn grow(&self, id: &Party) -> (Version, Cost) {
        match (id, self) {
            (Party::Leaf(true), Version::Leaf(n)) => (Version::Leaf(*n + 1), (0, 0)),
            (Party::Leaf(true), Version::Node(n, el, er)) => {
                let (el2, cl) = el.grow(&Party::Leaf(true));
                let (er2, cr) = er.grow(&Party::Leaf(true));
                if cl < cr {
                    (
                        Version::Node(*n, Box::new(el2), er.clone()),
                        (cl.0, cl.1 + 1),
                    )
                } else {
                    (
                        Version::Node(*n, el.clone(), Box::new(er2)),
                        (cr.0, cr.1 + 1),
                    )
                }
            }
            (Party::Leaf(false), _) => (self.clone(), (u32::MAX, u32::MAX)),
            (Party::Node(..), Version::Leaf(n)) => {
                let expanded =
                    Version::Node(*n, Box::new(Version::Leaf(0)), Box::new(Version::Leaf(0)));
                let (e2, c) = expanded.grow(id);
                (e2, (c.0 + 1, c.1))
            }
            (Party::Node(il, ir), Version::Node(n, el, er)) => {
                if il.is_empty() {
                    let (er2, cr) = er.grow(ir);
                    (
                        Version::Node(*n, el.clone(), Box::new(er2)),
                        (cr.0, cr.1 + 1),
                    )
                } else if ir.is_empty() {
                    let (el2, cl) = el.grow(il);
                    (
                        Version::Node(*n, Box::new(el2), er.clone()),
                        (cl.0, cl.1 + 1),
                    )
                } else {
                    let (el2, cl) = el.grow(il);
                    let (er2, cr) = er.grow(ir);
                    if cl < cr {
                        (
                            Version::Node(*n, Box::new(el2), er.clone()),
                            (cl.0, cl.1 + 1),
                        )
                    } else {
                        (
                            Version::Node(*n, el.clone(), Box::new(er2)),
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

    pub fn tick(&mut self, party: &Party) {
        *self = self.event(party);
    }

    pub fn is_normal(&self) -> bool {
        match self {
            Version::Leaf(_) => true,
            Version::Node(_, l, r) => {
                let one_zero = l.base() == 0 || r.base() == 0;
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
        match (self.leq(0, other, 0), other.leq(0, self, 0)) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            (false, false) => None,
        }
    }
}

impl Clock {
    pub fn seed() -> Self {
        Self::from_parts(Party::seed(), Version::new())
    }

    pub fn from_parts(party: Party, version: Version) -> Self {
        Clock { party, version }
    }

    pub fn into_parts(self) -> (Party, Version) {
        (self.party, self.version)
    }

    pub fn party(&self) -> &Party {
        &self.party
    }

    pub fn version(&self) -> Version {
        self.version.clone()
    }

    pub fn tick(&mut self) {
        self.version.tick(&self.party);
    }

    pub fn fork(&mut self) -> Clock {
        let child = self.party.fork();
        Clock {
            party: child,
            version: self.version.clone(),
        }
    }

    pub fn join(&mut self, other: Clock) -> Result<(), Clock> {
        let (op, ov) = other.into_parts();
        match self.party.join(op) {
            Ok(()) => {
                self.version |= ov;
                Ok(())
            }
            Err(op) => Err(Clock::from_parts(op, ov)),
        }
    }

    pub fn sync(&mut self, other: &mut Clock) -> Result<(), OverlapError> {
        if !self.party.is_disjoint(&other.party) {
            return Err(OverlapError);
        }
        let theirs = std::mem::replace(&mut other.party, Party::Leaf(false));
        self.party.join(theirs).expect("disjoint, just checked");
        other.party = self.party.fork();
        let merged = self.version.clone() | other.version.clone();
        self.version = merged.clone();
        other.version = merged;
        Ok(())
    }

    pub fn has_seen(&self, msg: &Version) -> bool {
        msg.leq(0, &self.version, 0)
    }

    pub fn happens_before(&self, other: &Clock) -> bool {
        self.version < other.version
    }

    pub fn concurrent_with(&self, other: &Clock) -> bool {
        self.version.partial_cmp(&other.version).is_none()
    }

    pub fn send(&mut self) -> Version {
        self.tick();
        self.version()
    }

    pub fn receive(&mut self, msg: Version) {
        self.version |= msg;
        self.tick();
    }

    pub fn trees(&self) -> (&Party, &Version) {
        (&self.party, &self.version)
    }
}

impl BitOr<Version> for Version {
    type Output = Version;
    fn bitor(self, rhs: Version) -> Version {
        self.join_off(0, &rhs, 0)
    }
}

impl BitOrAssign<Version> for Version {
    fn bitor_assign(&mut self, rhs: Version) {
        *self = self.join_off(0, &rhs, 0);
    }
}

impl BitOr<Version> for Clock {
    type Output = Clock;
    fn bitor(mut self, rhs: Version) -> Clock {
        self.version |= rhs;
        self
    }
}

impl BitOr<Clock> for Version {
    type Output = Clock;
    fn bitor(self, mut rhs: Clock) -> Clock {
        rhs.version |= self;
        rhs
    }
}

impl BitOrAssign<Version> for Clock {
    fn bitor_assign(&mut self, rhs: Version) {
        self.version |= rhs;
    }
}
