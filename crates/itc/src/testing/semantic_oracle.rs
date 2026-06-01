//! A third, semantically independent reference: the paper's §4 function-space construction,
//! realized *literally as functions*.
//!
//! The recursive [`oracle`] and the packed impl both represent a stamp as a *tree* and
//! realize each operation as the same tree recursion, so "impl == oracle" is blind to a bug
//! the two share. This module shares no code and no structure with that recursion: a stamp's
//! id *is* its characteristic function `⟦i⟧: [0,1) → {0,1}` and its event *is* its step
//! function `⟦e⟧: [0,1) → ℕ₀` ([`Id`]/[`Event`], boxed closures over dyadic rationals), and
//! every ITC operation is a closure combinator (§2-3). It is a deliberately inefficient,
//! one-to-one transcription of §4.
//!
//! ## Cross-check by replay
//!
//! [`tests`] plays one (single-seed) op trace against all three references — impl, tree
//! oracle, and this function space — then over the Cartesian product of each final clock
//! population computes a comparison descriptor (version causal order, party containment, party
//! disjointness) and requires all three to agree. This is ITC's defining guarantee: the
//! observable partial order is fixed by the operation sequence, independent of the (valid)
//! fork/inflation policy each implementation chooses — so the function space's *easy* choices
//! (`add-one` `event`, paper-`split` `fork`) yield the same order as the impl's minimal `grow`.
//!
//! That invariance holds only for a *proper* ITC system — one seed, so ids partition a single
//! space and all live ids stay disjoint. Several seeds would each own all of `[0,1)` (not a
//! valid configuration), and a cross-lineage `receive` then entangles events on the
//! overlapping region, where `add-one` and minimal `grow` genuinely disagree. Hence single
//! seed here; overlap and the join/sync-`Err` paths are id-algorithm behavior, checked against
//! the oracle elsewhere.

#[cfg(test)]
mod tests;

use std::cmp::Ordering;
use std::rc::Rc;

use crate::codec::Base;
use crate::oracle;

/// Grid exponent ceiling for the comparison scans: a comparison samples `2^g` points with
/// `g` the depth of the trees in hand, and this caps `g`. Set well above the depths the tests
/// build (arbitrary generators cap at 4, the op-trace tops out near 7), so it never bites; the
/// [`tests::grid_cap_is_never_reached`] guard pins that headroom.
pub(crate) const GRID_N: u32 = 10;

// ───────────────────────────── dyadic points ─────────────────────────────

/// A point `num / 2^exp` in `[0, 1)` (`0 ≤ num < 2^exp`; `exp == 0` is the point `0`).
/// Halving the enclosing interval — the §4 descent — just consumes the top bit of `num`, so
/// no general-rational arithmetic is needed.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Dyadic {
    num: u64,
    exp: u32,
}

impl Dyadic {
    /// The `k`-th of `2^g` equally spaced grid points, `k / 2^g`.
    fn grid(k: u64, g: u32) -> Self {
        Dyadic { num: k, exp: g }
    }

    /// The center of cell `k` at level `g`: `(2k + 1) / 2^{g+1}`.
    fn center(k: u64, g: u32) -> Self {
        Dyadic {
            num: 2 * k + 1,
            exp: g + 1,
        }
    }
}

impl PartialEq for Dyadic {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Dyadic {}
impl PartialOrd for Dyadic {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Dyadic {
    /// Compare `a/2^p` and `b/2^q` by cross-multiplication: `a·2^q` vs `b·2^p` (exponents are
    /// small in tests, so `u128` never overflows).
    fn cmp(&self, other: &Self) -> Ordering {
        let lhs = (self.num as u128) << other.exp;
        let rhs = (other.num as u128) << self.exp;
        lhs.cmp(&rhs)
    }
}

// ───────────────────────────── the function space ─────────────────────────────

/// A party's characteristic function `⟦i⟧: [0,1) → {0,1}`.
pub(crate) type Id = Rc<dyn Fn(Dyadic) -> bool>;
/// A version's step function `⟦e⟧: [0,1) → ℕ₀`.
pub(crate) type Event = Rc<dyn Fn(Dyadic) -> Base>;

/// `⟦seed⟧`: owns all of `[0,1)`.
pub(crate) fn seed_id() -> Id {
    Rc::new(|_| true)
}

/// `⟦Version::new()⟧`: the zero function.
pub(crate) fn new_ev() -> Event {
    Rc::new(|_| Base::ZERO)
}

/// Id union `⟦i1⟧ + ⟦i2⟧` (used by `join`/`sum`; operands must be disjoint for a valid id).
pub(crate) fn sum(a: Id, b: Id) -> Id {
    Rc::new(move |x| a(x) || b(x))
}

/// Event least-upper-bound `⟦e1⟧ ⊔ ⟦e2⟧`: pointwise max.
pub(crate) fn join(a: Event, b: Event) -> Event {
    Rc::new(move |x| {
        let (va, vb) = (a(x), b(x));
        if va >= vb {
            va
        } else {
            vb
        }
    })
}

/// `event`: the easiest §4-valid inflation — add one wherever the id owns. Dominates `e`,
/// is local to the owned region, and strictly advances on a nonempty id. (The impl's `grow`
/// is *minimal*; that is a §5.3 refinement, not a §4 law — see the module doc on why the
/// difference does not affect the causal-order comparison.)
pub(crate) fn event(i: &Id, e: Event) -> Event {
    let i = i.clone();
    Rc::new(move |x| {
        let v = e(x);
        if i(x) {
            v + Base::from(1u32)
        } else {
            v
        }
    })
}

/// `fork`/`split`: partition the owned region into two nonempty disjoint halves, reproducing
/// the paper's `split` exactly so the result matches the impl/oracle region-for-region.
///
/// Returns `(left, right)` where the cut `b` is the midpoint of the smallest dyadic interval
/// containing the support — the paper's split point (descend through singly-occupied halves;
/// split the first both-occupied node). Returns threshold-gated closures; only [`find_split`]
/// samples.
pub(crate) fn fork(i: Id) -> (Id, Id) {
    let b = find_split(&i);
    let il = i.clone();
    (
        Rc::new(move |x| il(x) && x < b),
        Rc::new(move |x| i(x) && x >= b),
    )
}

/// The split boundary for [`fork`]: the midpoint of the smallest dyadic interval enclosing the
/// support. Detected at a fixed granularity [`GRID_N`] — which the depth cap guarantees is
/// `≥ depth`, so every owned cell is hit (an *adaptive* search cannot reproduce the paper
/// split: a thin, deep part of the support is undetectable until sampled at its own depth, and
/// the search has no way to know that depth, so it can stop early at a different valid cut).
/// At full resolution the min and max owned cells bound the support, so flipping their
/// most-significant differing bit gives the enclosing interval's midpoint. Panics only if `i`
/// owns nothing (never for a valid id).
fn find_split(i: &Id) -> Dyadic {
    let g = GRID_N;
    let mut first = None;
    let mut last = None;
    for k in 0..(1u64 << g) {
        if i(Dyadic::center(k, g)) {
            first.get_or_insert(k);
            last = Some(k);
        }
    }
    let (f, l) = (first.expect("fork of an empty id"), last.unwrap());
    if f == l {
        // The whole support fits one level-g cell (only if depth ≥ GRID_N, which the cap
        // rules out): bisect that cell.
        return Dyadic::center(f, g);
    }
    let p = 63 - (f ^ l).leading_zeros();
    let boundary = ((f >> (p + 1)) << (p + 1)) + (1 << p);
    Dyadic::grid(boundary, g)
}

// ───────────────────────────── embedding (tree → function) ─────────────────────────────

/// `⟦i⟧` for an oracle id tree: the only place a tree is read. Descends by the §4 recursion —
/// at a node the left child owns `[0,½)` (argument `2x`), the right `[½,1)` (argument `2x−1`).
pub(crate) fn lift_id(t: oracle::Party) -> Id {
    Rc::new(move |x| eval_id(&t, x))
}

/// `⟦e⟧` for an oracle event tree: base values accumulate down the path.
pub(crate) fn lift_ev(t: oracle::Version) -> Event {
    Rc::new(move |x| eval_ev(&t, x))
}

fn eval_id(t: &oracle::Party, mut x: Dyadic) -> bool {
    use oracle::Party as P;
    let mut node = t;
    loop {
        match node {
            P::Leaf(b) => return *b,
            P::Node(l, r) => {
                let (right, nx) = descend(x);
                node = if right { r } else { l };
                x = nx;
            }
        }
    }
}

fn eval_ev(t: &oracle::Version, mut x: Dyadic) -> Base {
    use oracle::Version as V;
    let mut node = t;
    let mut acc = Base::ZERO;
    loop {
        match node {
            V::Leaf(n) => return acc + n,
            V::Node(n, l, r) => {
                acc += n;
                let (right, nx) = descend(x);
                node = if right { r } else { l };
                x = nx;
            }
        }
    }
}

/// One step of the §4 descent: which half of `[0,1)` the point lies in, and the point
/// rescaled into that half (`2x` on the left, `2x − 1` on the right). A point coarser than
/// the tree (`exp == 0`) is the left endpoint `0`, so it descends left and stays `0`.
fn descend(x: Dyadic) -> (bool, Dyadic) {
    if x.exp == 0 {
        return (false, x);
    }
    let half = 1u64 << (x.exp - 1);
    if x.num < half {
        (
            false,
            Dyadic {
                num: x.num,
                exp: x.exp - 1,
            },
        )
    } else {
        (
            true,
            Dyadic {
                num: x.num - half,
                exp: x.exp - 1,
            },
        )
    }
}

// ───────────────────────────── scan comparison (no materialization) ─────────────────────────────

/// Event causal order: pointwise `≤` over the `2^g` grid points, `None` if incomparable
/// (concurrent). Scans with an early-out once both directions are ruled out.
pub(crate) fn ev_order(a: &Event, b: &Event, g: u32) -> Option<Ordering> {
    let (mut le, mut ge) = (true, true);
    for k in 0..(1u64 << g) {
        let x = Dyadic::grid(k, g);
        match a(x).cmp(&b(x)) {
            Ordering::Less => ge = false,
            Ordering::Greater => le = false,
            Ordering::Equal => {}
        }
        if !le && !ge {
            break;
        }
    }
    order_of(le, ge)
}

/// Party containment order, matching `Party::partial_cmp`: an ancestor (larger owned region)
/// reads as `Less`. `le` is `a ⊇ b`, `ge` is `b ⊇ a`.
pub(crate) fn id_order(a: &Id, b: &Id, g: u32) -> Option<Ordering> {
    let (mut le, mut ge) = (true, true);
    for k in 0..(1u64 << g) {
        let x = Dyadic::grid(k, g);
        let (oa, ob) = (a(x), b(x));
        if ob && !oa {
            le = false; // b owns a point a does not: ¬(a ⊇ b)
        }
        if oa && !ob {
            ge = false;
        }
        if !le && !ge {
            break;
        }
    }
    order_of(le, ge)
}

/// Party disjointness `i1 · i2 = 0`: no grid point owned by both.
pub(crate) fn disjoint(a: &Id, b: &Id, g: u32) -> bool {
    !(0..(1u64 << g)).any(|k| {
        let x = Dyadic::grid(k, g);
        a(x) && b(x)
    })
}

fn order_of(le: bool, ge: bool) -> Option<Ordering> {
    match (le, ge) {
        (true, true) => Some(Ordering::Equal),
        (true, false) => Some(Ordering::Less),
        (false, true) => Some(Ordering::Greater),
        (false, false) => None,
    }
}

// ───────────────────────────── the function-space clock ─────────────────────────────

/// A clock in the function space: an owned-region characteristic function and an event step
/// function. Operations mirror the crate's [`Clock`](crate::Clock) semantics.
pub(crate) struct SemClock {
    pub(crate) id: Id,
    pub(crate) ev: Event,
}

impl SemClock {
    pub(crate) fn seed() -> Self {
        SemClock {
            id: seed_id(),
            ev: new_ev(),
        }
    }

    pub(crate) fn tick(&mut self) {
        self.ev = event(&self.id, self.ev.clone());
    }

    /// Split off a child; `self` keeps the left half, the child takes the right (mirroring the
    /// crate's fork, which returns the child and keeps `self`).
    pub(crate) fn fork(&mut self) -> SemClock {
        let (left, right) = fork(self.id.clone());
        self.id = left;
        SemClock {
            id: right,
            ev: self.ev.clone(),
        }
    }

    /// Absorb a disjoint clock; on overlap return it unchanged. `g` resolves the disjointness
    /// scan.
    pub(crate) fn join(&mut self, other: SemClock, g: u32) -> Result<(), SemClock> {
        if disjoint(&self.id, &other.id, g) {
            self.id = sum(self.id.clone(), other.id);
            self.ev = join(self.ev.clone(), other.ev);
            Ok(())
        } else {
            Err(other)
        }
    }

    /// Reconcile two clocks: merge events to their LUB, union ids, re-split the union.
    pub(crate) fn sync(&mut self, other: &mut SemClock, g: u32) -> Result<(), ()> {
        if disjoint(&self.id, &other.id, g) {
            let merged = join(self.ev.clone(), other.ev.clone());
            let (left, right) = fork(sum(self.id.clone(), other.id.clone()));
            self.id = left;
            self.ev = merged.clone();
            other.id = right;
            other.ev = merged;
            Ok(())
        } else {
            Err(())
        }
    }

    /// Advance, then snapshot the event to transmit.
    pub(crate) fn send(&mut self) -> Event {
        self.tick();
        self.ev.clone()
    }

    /// Merge a received event, then advance.
    pub(crate) fn receive(&mut self, msg: Event) {
        self.ev = event(&self.id, join(self.ev.clone(), msg));
    }
}

// ───────────────────────────── structural depth (grid sizing) ─────────────────────────────

/// Structural depth of an event tree (0 for a leaf), iterative.
pub(crate) fn ev_depth(e: &oracle::Version) -> u32 {
    use oracle::Version as V;
    let mut max = 0;
    let mut stack = vec![(e, 0u32)];
    while let Some((node, d)) = stack.pop() {
        max = max.max(d);
        if let V::Node(_, l, r) = node {
            stack.push((l, d + 1));
            stack.push((r, d + 1));
        }
    }
    max
}

/// Structural depth of an id tree (0 for a leaf), iterative.
pub(crate) fn id_depth(i: &oracle::Party) -> u32 {
    use oracle::Party as P;
    let mut max = 0;
    let mut stack = vec![(i, 0u32)];
    while let Some((node, d)) = stack.pop() {
        max = max.max(d);
        if let P::Node(l, r) = node {
            stack.push((l, d + 1));
            stack.push((r, d + 1));
        }
    }
    max
}
