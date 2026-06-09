//! A third, semantically independent reference: the paper's §4 function-space
//! construction, realized *literally as functions*.
//!
//! The recursive [`oracle`] and the packed impl both represent a stamp as a
//! *tree* and realize each operation as the same tree recursion, so "impl ==
//! oracle" is blind to a bug the two share. This module shares no code and no
//! structure with that recursion: a stamp's id *is* its characteristic function
//! `⟦i⟧: [0,1) → {0,1}` and its event *is* its step function `⟦e⟧: [0,1) → ℕ₀`
//! ([`Id`]/[`Event`], boxed closures over dyadic rationals), and every ITC
//! operation is a closure combinator (§2-3). It is a deliberately inefficient,
//! one-to-one transcription of §4.
//!
//! ## Cross-check by replay
//!
//! [`tests`] plays one (single-seed) op trace against all three references —
//! impl, tree oracle, and this function space — then over the Cartesian product
//! of each final clock population computes a comparison descriptor (version
//! causal order, party containment, party disjointness) and requires all three
//! to agree. This is ITC's defining guarantee: the observable partial order is
//! fixed by the operation sequence, independent of the (valid) fork/inflation
//! policy each implementation chooses.
//!
//! To exercise that independence directly, the two under-determined
//! operations make a fresh random, law-abiding choice on every call (seeded
//! from the proptest input, so failures replay): [`event`] draws an
//! arbitrary §4-valid inflation (a partial, random-amount bump of the owned
//! region) and [`fork`] draws an arbitrary §4-valid split (dealing the owned
//! region's pieces out at random). The cross-check therefore asserts
//! agreement across a random sample of the valid policy space, not merely
//! between one fixed instantiation and the impl's minimal `grow`. The one
//! concession to the finite grid: an arbitrary cut of an indivisible
//! interval needs unbounded resolution, so such a piece is bisected at its
//! midpoint (see [`fork`]); every other split, and every inflation, is
//! unconstrained.
//!
//! That invariance holds only for a *proper* ITC system — one seed, so ids
//! partition a single space and all live ids stay disjoint. Several seeds would
//! each own all of `[0,1)` (not a valid configuration), and a cross-lineage
//! `receive` then entangles events on the overlapping region, where the random
//! policy and minimal `grow` genuinely disagree. Hence single seed here;
//! overlap and the join/sync-`Err` paths are id-algorithm behavior, checked
//! against the oracle elsewhere. (Under the random policy multiple seeds also
//! disagree on *structural* vs. *geometric* party disjointness, a second reason
//! they cannot be replayed in lockstep.)

#[cfg(test)]
mod tests;

use std::cmp::Ordering;
use std::rc::Rc;

use rand::rngs::StdRng;
use rand::Rng;

use crate::codec::Base;
use crate::oracle;

/// Grid exponent ceiling for the comparison and resolution scans: a scan
/// samples `2^g` points with `g` the resolution actually in hand
/// ([`id_res`]/[`ev_res`]), and this caps `g`. Set well above the resolution
/// the tests reach (arbitrary generators cap at 4; a single-seed op trace
/// tops out near 7, since the random `fork` only deepens at the paper's
/// rate), so it never bites. The headroom is required for correctness:
/// `fork` bisects an indivisible piece one level finer, so a piece at
/// resolution `GRID_N` could not be split.
/// [`tests::grid_cap_is_never_reached`] pins it.
pub(crate) const GRID_N: u32 = 10;

// ───────────────────────────── dyadic points ─────────────────────────────

/// A point `num / 2^exp` in `[0, 1)` (`0 ≤ num < 2^exp`; `exp == 0` is the
/// point `0`). Halving the enclosing interval — the §4 descent — just consumes
/// the top bit of `num`, so no general-rational arithmetic is needed.
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
    /// Compare `a/2^p` and `b/2^q` by cross-multiplication: `a·2^q` vs `b·2^p`
    /// (exponents are small in tests, so `u128` never overflows).
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

/// Id union `⟦i1⟧ + ⟦i2⟧` (used by `join`/`sum`; operands must be disjoint for
/// a valid id).
pub(crate) fn sum(a: Id, b: Id) -> Id {
    Rc::new(move |x| a(x) || b(x))
}

/// Id difference `⟦i1⟧ \ ⟦i2⟧`: the region `a` owns that `b` does not, pointwise
/// `a ∧ ¬b`. The function-space realization of [`Party::without`](crate::Party::without)
/// — total (overlap is the point) and possibly empty (the all-`false` function,
/// when `b` covers `a`).
pub(crate) fn diff(a: Id, b: Id) -> Id {
    Rc::new(move |x| a(x) && !b(x))
}

/// Event projection `⟦e⟧ / ⟦i⟧`: keep the value where `i` owns the region, zero
/// it everywhere else (pointwise `if i(x) { e(x) } else { 0 }`). The
/// function-space realization of the quotient [`Version / &Party`](crate::Version).
pub(crate) fn project(e: Event, i: Id) -> Event {
    Rc::new(move |x| if i(x) { e(x) } else { Base::ZERO })
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

/// Event greatest-lower-bound `⟦e1⟧ ⊓ ⟦e2⟧`: pointwise min. The dual of
/// [`join`]; the meet that the crate exposes as `Version::&`.
pub(crate) fn meet(a: Event, b: Event) -> Event {
    Rc::new(move |x| {
        let (va, vb) = (a(x), b(x));
        if va <= vb {
            va
        } else {
            vb
        }
    })
}

// ───────────────────────────── cell indexing ─────────────────────────────

/// The index, at resolution `level`, of the cell containing `x` — the top
/// `level` bits of `x` (always `< 2^level`). The random operations draw a
/// per-cell decision into a table indexed this way, so a closure maps a sampled
/// point back to its cell to look that decision up.
fn cell_at(x: Dyadic, level: u32) -> usize {
    let cell = if x.exp >= level {
        x.num >> (x.exp - level)
    } else {
        x.num << (level - x.exp)
    };
    cell as usize
}

// ───────────────────────────── the under-determined operations ─────────────────────────────

/// `event`: a *random* §4-valid inflation, freshly drawn each call. §4 pins
/// only `⟦e'⟧ = ⟦e⟧ + f·⟦i⟧` for some `f` with `f·⟦i⟧ ▷ 0` (strictly positive
/// somewhere the id owns); the old fixed `add-one` and the impl's minimal
/// `grow` are two particular `f`. Here `f` is arbitrary: each owned cell (at
/// the id's own resolution) is bumped by an independent amount in `0..=3`, with
/// one owned cell forced positive so the advance is real (and others may be `0`
/// — a *partial* inflation). In a proper single-seed system every such `f`
/// still tracks happens-before — it meets the §3 event condition (the result is
/// fresh, `e' ≰` any other live stamp, and dominates nothing new, because the
/// id owns its region exclusively) — so the causal order is identical to
/// `add-one`'s and `grow`'s. That invariance is what the replay exercises.
pub(crate) fn event(i: &Id, e: Event, rng: &mut StdRng) -> Event {
    let level = id_res(i);
    let owned = owned_cells(i, level);
    // Per-cell bump, indexed by cell at the id's resolution: each owned cell
    // `0..=3`, with the first owned cell forced `1..=3` so `f·i ▷ 0`. Non-owned
    // cells stay `0` (and are gated out).
    let mut bump = vec![0u64; 1 << level];
    for (n, &c) in owned.iter().enumerate() {
        bump[c] = if n == 0 {
            rng.gen_range(1..=3) // first owned cell: strictly positive
        } else {
            rng.gen_range(0..=3) // any other owned cell: 0 leaves it untouched (partial inflation)
        };
    }
    let bump = Rc::new(bump);
    let i = i.clone();
    Rc::new(move |x| {
        let v = e(x);
        if i(x) {
            v + Base::from(bump[cell_at(x, level)])
        } else {
            v
        }
    })
}

/// `fork`/`split`: a *random* §4-valid partition, freshly drawn each call. §4
/// pins only `⟦i₁⟧ + ⟦i₂⟧ = ⟦i⟧` and `⟦i₁⟧ · ⟦i₂⟧ = 0` (a disjoint cover of the
/// owned region); the paper's `split` is one particular choice. Here the
/// region's maximal constant pieces — its owned cells at its *own* resolution —
/// are dealt independently to the two sides (an arbitrary, possibly
/// interleaved, partition), with the lowest and highest pinned to opposite
/// sides so both halves are nonempty. When the region is a single indivisible
/// piece there is nothing to deal out, so it is bisected at its midpoint (the
/// one forced cut).
///
/// Dealing out *existing* pieces adds no new boundary, so resolution grows only
/// on the bisection of an indivisible piece — exactly the paper's rate (≤ 1
/// level per fork). That is the one concession to a *finite* comparison grid:
/// an arbitrary cut of an indivisible interval would need ever-finer dyadic
/// points, which a fixed grid cannot resolve over a long trace.
/// [`tests::grid_cap_is_never_reached`] guards the headroom. (Both halves
/// nonempty is stronger than §4 — which permits the empty `peek` split — but a
/// child handed an empty id could never advance, diverging from the impl; the
/// replay needs both children live.)
pub(crate) fn fork(i: &Id, rng: &mut StdRng) -> (Id, Id) {
    let res = id_res(i);
    // Deal out the region's pieces at its own resolution; if it is a single
    // piece, bisect it.
    let level = if owned_cells(i, res).len() >= 2 {
        res
    } else {
        (res + 1).min(GRID_N)
    };
    let owned = owned_cells(i, level);
    let (lo, hi) = (owned[0], *owned.last().expect("fork of an empty id"));
    // `left[c]` decides each owned piece independently, then `lo` is pinned
    // left and `hi` right so both halves are nonempty (`lo != hi`, the region
    // having ≥ 2 pieces at `level`).
    let mut left = vec![false; 1 << level];
    for &c in &owned {
        left[c] = rng.gen();
    }
    left[lo] = true;
    left[hi] = false;
    let left = Rc::new(left);
    let il = i.clone();
    let right_mask = left.clone();
    let ir = i.clone();
    (
        Rc::new(move |x| il(x) && left[cell_at(x, level)]),
        Rc::new(move |x| ir(x) && !right_mask[cell_at(x, level)]),
    )
}

// ───────────────────────────── resolution probing ─────────────────────────────

/// The owned cells of `i` at resolution `level`, by index. `level` is `≥` the
/// id's own resolution, so `⟦i⟧` is constant within each cell and one interior
/// sample per cell decides it.
fn owned_cells(i: &Id, level: u32) -> Vec<usize> {
    (0..(1u64 << level))
        .filter(|&k| i(Dyadic::center(k, level)))
        .map(|k| k as usize)
        .collect()
}

/// The resolution of an id: the finest dyadic level at which `⟦i⟧` actually
/// changes value (`0` if constant). Probed from the function, not tracked — so
/// a `sum` that recombines into a coarser region (e.g. two halves back to the
/// whole `[0,1)`) reports its *true*, collapsed resolution, keeping the
/// comparison grid no finer than necessary.
pub(crate) fn id_res(i: &Id) -> u32 {
    let samples: Vec<bool> = (0..(1u64 << GRID_N))
        .map(|k| i(Dyadic::center(k, GRID_N)))
        .collect();
    resolution(&samples)
}

/// The resolution of an event step function (see [`id_res`]).
pub(crate) fn ev_res(e: &Event) -> u32 {
    let samples: Vec<Base> = (0..(1u64 << GRID_N))
        .map(|k| e(Dyadic::center(k, GRID_N)))
        .collect();
    resolution(&samples)
}

/// Finest boundary level present in a row of `2^GRID_N` cell samples: the
/// deepest level at which two adjacent cells disagree (`0` if all equal). The
/// boundary between cells `k-1` and `k` sits at level `GRID_N − v₂(k)`, so the
/// finest disagreement is the resolution.
fn resolution<T: PartialEq>(samples: &[T]) -> u32 {
    let mut res = 0;
    for k in 1..samples.len() {
        if samples[k] != samples[k - 1] {
            res = res.max(GRID_N - (k as u32).trailing_zeros());
        }
    }
    res
}

// ───────────────────────────── embedding (tree → function) ─────────────────────────────

/// `⟦i⟧` for an oracle id tree (with [`lift_ev`], the only places a tree is
/// read). Descends by the §4 recursion: at a node the left child owns
/// `[0,½)` (argument `2x`), the right `[½,1)` (argument `2x−1`).
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

/// One step of the §4 descent: which half of `[0,1)` the point lies in, and the
/// point rescaled into that half (`2x` on the left, `2x − 1` on the right). A
/// point coarser than the tree (`exp == 0`) is the left endpoint `0`, so it
/// descends left and stays `0`.
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

/// Event causal order: pointwise `≤` over the `2^g` grid points, `None` if
/// incomparable (concurrent). Scans with an early-out once both directions are
/// ruled out.
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

/// Party containment order, matching `Party::partial_cmp`: an ancestor (larger
/// owned region) reads as `Less`. `le` is `a ⊇ b`, `ge` is `b ⊇ a`.
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

/// `min_ticks` recovered from the step function `⟦e⟧`: the sum of the per-node
/// floors pulled up the dyadic subdivision — the *geometric* mirror of event-tree
/// normalization, sharing no code with the tree (it samples the closure and
/// recurses on halves). A cell already constant is a leaf: its value (relative to
/// the floor pulled up above it) is its only base; otherwise the cell's minimum
/// is pulled up and the two halves recurse with it subtracted, so the total is
/// `local + left + right`. The result is the sum of every base in the normal-form
/// tree — exactly [`oracle::Version::min_ticks`](crate::oracle::Version). `g` must
/// resolve `e` (every real boundary at level `≤ g`), so each level-`g` cell is a
/// single constant point.
pub(crate) fn min_ticks(e: &Event, g: u32) -> Base {
    fn rec(e: &Event, k: u64, level: u32, g: u32, off: &Base) -> Base {
        let span = 1u64 << (g - level);
        let start = k << (g - level);
        // The cell's values relative to the floor already pulled up above it.
        // Every value here is `≥ off` (a containing cell's running minimum), so
        // the `Base` subtraction never underflows.
        let vals: Vec<Base> = (start..start + span)
            .map(|j| e(Dyadic::grid(j, g)) - off)
            .collect();
        let local = vals
            .iter()
            .min()
            .expect("a cell samples at least one point")
            .clone();
        // A constant cell (or a single point at the finest level) is a leaf.
        if level == g || vals.iter().all(|v| *v == local) {
            return local;
        }
        let off2 = off.clone() + &local;
        let l = rec(e, 2 * k, level + 1, g, &off2);
        let r = rec(e, 2 * k + 1, level + 1, g, &off2);
        local + l + r
    }
    rec(e, 0, 0, g, &Base::ZERO)
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

/// A clock in the function space: an owned-region characteristic function and
/// an event step function. Operations mirror the crate's
/// [`Clock`](crate::Clock) semantics.
pub(crate) struct FunctionClock {
    pub(crate) id: Id,
    pub(crate) ev: Event,
}

impl FunctionClock {
    pub(crate) fn seed() -> Self {
        FunctionClock {
            id: seed_id(),
            ev: new_ev(),
        }
    }

    pub(crate) fn tick(&mut self, rng: &mut StdRng) {
        self.ev = event(&self.id, self.ev.clone(), rng);
    }

    /// Split off a child; `self` keeps the left half, the child takes the right
    /// (mirroring the crate's fork, which returns the child and keeps `self`).
    pub(crate) fn fork(&mut self, rng: &mut StdRng) -> FunctionClock {
        let (left, right) = fork(&self.id, rng);
        self.id = left;
        FunctionClock {
            id: right,
            ev: self.ev.clone(),
        }
    }

    /// Absorb a disjoint clock; on overlap return it unchanged. `g` resolves the disjointness
    /// scan.
    pub(crate) fn join(&mut self, other: FunctionClock, g: u32) -> Result<(), FunctionClock> {
        if disjoint(&self.id, &other.id, g) {
            self.id = sum(self.id.clone(), other.id);
            self.ev = join(self.ev.clone(), other.ev);
            Ok(())
        } else {
            Err(other)
        }
    }

    /// Reconcile two clocks: merge events to their LUB, union ids, re-split the union.
    pub(crate) fn sync(
        &mut self,
        other: &mut FunctionClock,
        g: u32,
        rng: &mut StdRng,
    ) -> Result<(), ()> {
        if disjoint(&self.id, &other.id, g) {
            let merged = join(self.ev.clone(), other.ev.clone());
            let (left, right) = fork(&sum(self.id.clone(), other.id.clone()), rng);
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
    pub(crate) fn send(&mut self, rng: &mut StdRng) -> Event {
        self.tick(rng);
        self.ev.clone()
    }

    /// Merge a received event, then advance.
    pub(crate) fn receive(&mut self, msg: Event, rng: &mut StdRng) {
        self.ev = event(&self.id, join(self.ev.clone(), msg), rng);
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
