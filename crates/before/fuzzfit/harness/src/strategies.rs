//! Shape-biased program generators: the family roster, the budgets, and the
//! coupled/independent operand regimes.
//!
//! Each family translates one of the meter board's adversarial shapes into a
//! *program generator*: the archetype with randomized dimensions and
//! structural jitter, built exclusively through public operations (seed,
//! fork, tick, join, …), so every constructed value is API-reachable by
//! definition. The families bias exploration toward the regions the chosen
//! adversarial shapes mark as interesting; the [`Family::Combination`]
//! programs then explore the composition space between them, and
//! [`Family::Independent`] crosses operands from separately seeded
//! universes.
//!
//! # The two n-ary regimes
//!
//! Multi-operand operations are exercised under two deliberate regimes:
//!
//! - **Coupled**: operands constructed together in one universe, valid by
//!   construction (linearity respected, one seed). Every family below except
//!   `Independent` generates this regime.
//! - **Independent**: operands from separately seeded universes — inputs on
//!   which the *result* is meaningless but the *cost claim still binds*: an
//!   operation must stay amortized linear whether or not the caller honored
//!   the safety rules. Each universe is still API-constructed internally;
//!   only the measured multi-operand calls cross them. Operations that
//!   reject such operands (`Party::join`, `Clock::join`/`sync` on overlap)
//!   have the rejection arm measured as its own legitimate outcome.
//!
//! # Budgets
//!
//! Every generator runs under a [`Budget`]: hard caps on emitted ops, ticks,
//! forks, and fold width, enforced by the builder no matter what parameters
//! the strategy draws. The caps bound total constructed size a priori
//! (packed growth per public op is amortized constant per tick/fork), which
//! keeps iterated joins from compounding exponentially and doubles as the
//! honesty bound for composed cases: a program's total denominated work is
//! within a constant of its op budget. Most families run under [`BUDGET`];
//! the reach family ([`Family::Escalation`]) runs under
//! [`ESCALATION_BUDGET`] — see [`budget_for`].
//!
//! # Scope: what the generators deliberately never construct
//!
//! - **Wide magnitude.** Operands are built exclusively through
//!   value-producing operations, so a `2^b`-wide leaf costs `2^b` ticks
//!   here — even though `Version::decode` reaches one from `O(b)` bits of
//!   crafted input. Crafted codec input, and with it the wide-magnitude
//!   regime, deliberately remains the meter board's hand-built territory;
//!   this instrument's envelope is the region reachable by paying for
//!   values one operation at a time.
//! - **Codec rejection.** The staged bytes are always the canonical
//!   encoding (or rendering) the immediately preceding encode/display step
//!   produced, so `decode`/`FromStr` rejection paths are never measured
//!   here; malformed-input cost is the decode fuzz targets' and the meter
//!   board's territory. The rejection arms this harness *does* measure are
//!   the operation-level ones (`join`/`sync`/`without`/`checked_sub` on
//!   overlap, emptiness, or underflow), predicted per case by the mirror.
//! - **Empty folds.** `join_all`/`meet_all` operands come from clock
//!   populations the families build, never from an empty range, so
//!   `meet_all([])`'s `None` outcome is not sampled. It stays unpriced
//!   deliberately: the outcome is production-reachable but structurally
//!   constant — no operand exists, so there is nothing for a regression
//!   to scale with and nothing to denominate a cost against.

use proptest::prelude::*;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::ops::{Op, Reg};

/// Hard caps one generated program may not exceed, whatever its parameters.
#[derive(Debug, Clone, Copy)]
pub struct Budget {
    /// Maximum emitted ops (equivalently, measured guest calls).
    pub max_ops: usize,
    /// Maximum total `tick`s (clock or version) across the program.
    pub max_ticks: u32,
    /// Maximum total forks (single and balanced shares combined).
    pub max_forks: u32,
    /// Maximum operand count for one fold (`join_all`/`meet_all`).
    pub max_fold: u32,
}

/// The budget of record for both legs.
///
/// Sized for suite time: one program stays well under ~10⁹ fuel and a few
/// thousand guest calls, so the enforcement sentry's case count (not the
/// per-case size) is the knob that scales suite duration.
pub const BUDGET: Budget = Budget {
    max_ops: 6_000,
    max_ticks: 3_000,
    max_forks: 1_536,
    max_fold: 1_024,
};

/// The reach family's budget.
///
/// Sized to admit the family's full depth draw (256..=1792 spine forks)
/// plus the cadence battery riding on it, so every kernel row — pair,
/// fold, query, and the single-operand ticks/sends/recvs/splits — sees
/// denominators decades past the rest of the roster and the fitted
/// *slope*, not the band's width, carries the asymptotic judgment there.
/// Construction cost is quadratic in the reach (every op pays the
/// current size), which is why this budget belongs to one low-weighted
/// family instead of the whole roster.
pub const ESCALATION_BUDGET: Budget = Budget {
    max_ops: 9_000,
    max_ticks: 3_000,
    max_forks: 2_048,
    max_fold: 1_024,
};

/// Shift-overflow guard on the comb families' magnitude exponents.
///
/// The drawn `magnitude` becomes a shift count (`1 << magnitude`), so a
/// draw-range widening past 31 would otherwise overflow the shift. The
/// cap never binds today — the magnitude draws top out at 8 — it makes
/// the shift's definedness local to the construction instead of resting
/// on the draw ranges, and bounds a capped tooth's tick count (2¹⁰ − 1)
/// far below the tick budgets.
const MAGNITUDE_SHIFT_CAP: u32 = 10;

/// The escalation family's depth cap: the top of [`any_family`]'s draw
/// range.
///
/// Also the depth of the enforcement suite's fixed depth-cap replay
/// ([`ESCALATION_REPLAYS`]) — one constant, so widening the family's
/// range cannot silently leave the replay pinned below the family's
/// true far end.
pub const ESCALATION_MAX_DEPTH: u32 = 1792;

/// The enforcement suite's fixed escalation replays, as (depth, seed).
///
/// One mid-reach rung and the depth cap, on distinct seeds: the
/// deterministic reach proof the enforcement suite replays every run.
/// `bin/calibrate` replays the same two programs — enforcement-context
/// executions outside the calibration corpus — to measure the ceiling
/// excess [`crate::bands::ENFORCE_MARGIN`] must absorb, so the two
/// consumers must agree on the (depth, seed) pins.
pub const ESCALATION_REPLAYS: [(u32, u64); 2] = [(1024, 0xE5CA), (ESCALATION_MAX_DEPTH, 0x1792)];

/// The small-operand corpus size: how many [`Family::Bootstrap`] programs
/// the deterministic bootstrap stream enumerates (seed = case index,
/// rounds cycling `1..=BOOTSTRAP_MAX_ROUNDS`).
///
/// Sized so every rounds rung repeats under several seeds: the jitter
/// across seeds populates the sub-floor span densely instead of pinning
/// one fuel point per size.
pub const BOOTSTRAP_PROGRAMS: usize = 96;

/// The bootstrap family's growth cap, in rounds.
///
/// Measured against the mirror's denominators: at this cap the family's
/// clock tops out just under the fit floor (`crate::fit::FIT_FLOOR_BITS`),
/// so the corpus covers the sub-floor span end to end while the
/// single-operand kernels stay inside the constant-overhead regime the
/// small bands price. Re-derive by sweeping the printed denominator
/// ranges in `bin/calibrate` when the growth-per-round changes.
pub const BOOTSTRAP_MAX_ROUNDS: u32 = 12;

/// The budget a family's programs run under (the builder enforces it
/// unconditionally, whatever the drawn dimensions).
pub fn budget_for(family: &Family) -> Budget {
    match family {
        Family::Escalation { .. } => ESCALATION_BUDGET,
        _ => BUDGET,
    }
}

/// One family instantiation: a named archetype with its dimensions.
///
/// The names map onto the meter board's family roster; the board's control
/// variants ride as parameters (`hifloor`, `plateau`, `tail_ticks = 1` for
/// the narrow mirror cross).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Family {
    /// The dense event spine: a deep fork chain, ticks at every depth.
    DenseSpine {
        /// Fork-chain depth.
        depth: u32,
        /// Ticks per level (0..=3 adds jitter).
        ticks_per_level: u32,
    },
    /// A large root magnitude over a spine.
    BigRoot {
        /// Ticks paid at the seed before any fork.
        root_ticks: u32,
        /// Fork-chain depth below the root.
        depth: u32,
    },
    /// One party, maximal single-leaf magnitude within budget.
    HugeLeaf {
        /// Ticks paid at the seed.
        ticks: u32,
    },
    /// Sibling teeth oscillating across a power-of-two carry boundary.
    CliffComb {
        /// Number of balanced-forked teeth.
        teeth: u32,
        /// High teeth tick to `2^magnitude ± 1`.
        magnitude: u32,
    },
    /// Two parties forked in deep lockstep (the id-walk pair).
    IdPairLockstep {
        /// Lockstep fork depth.
        depth: u32,
        /// Keep the child (true) or the parent (false) at each level.
        divert: bool,
    },
    /// A carry-boundary comb crossed with a scattered party (the
    /// output-domination shape: projection rows).
    CombScatter {
        /// Comb teeth.
        teeth: u32,
        /// High-tooth magnitude exponent.
        magnitude: u32,
    },
    /// Tick counts decaying harmonically down the spine (the rank fold's
    /// wide-numerator shape).
    Harmonic {
        /// Spine depth.
        depth: u32,
        /// Ticks at the top level (level `i` gets `total / (i + 1)`).
        total_ticks: u32,
    },
    /// A balanced-forked, once-ticked population folded in adversarial
    /// order (the fold rows' scatter shape).
    ScatterFold {
        /// Population size (capped by the fold budget).
        clocks: u32,
    },
    /// Full sibling pairs at every level: both sides of each fork ticked
    /// equally (the right-full walk shape).
    NestedFull {
        /// Fork-chain depth.
        depth: u32,
        /// Ticks per side per level.
        ticks: u32,
    },
    /// A deep spine with ticks concentrated at the tail (`tail_ticks = 1`
    /// is the narrow mirror cross; large values the wide one).
    WideTail {
        /// Ticks at the deepest level.
        tail_ticks: u32,
        /// Spine depth.
        depth: u32,
    },
    /// Descending tick counts down the spine (every consumed leaf undercuts
    /// the open minima).
    Staircase {
        /// Spine depth; level `i` gets `depth - i` ticks.
        depth: u32,
    },
    /// Sibling sites sharing one wide minimum over a floor, joined in a
    /// close-reveal cycle (`hifloor` is the O(1)-circulation control).
    RevealComb {
        /// Sites.
        teeth: u32,
        /// Site magnitude exponent.
        magnitude: u32,
        /// Raise the floor to within O(1) of the sites (the control).
        hifloor: bool,
    },
    /// The reveal cycle with no equal-sibling site anywhere.
    PureComb {
        /// Teeth (distinct values `1..=teeth`).
        teeth: u32,
    },
    /// Ascending teeth with a terminal zero cliff (`plateau` levels every
    /// tooth: the hop-schedule control).
    AscendCliff {
        /// Teeth.
        teeth: u32,
        /// Level all teeth instead of ascending (the control).
        plateau: bool,
    },
    /// The organic control: a mild random gossip walk.
    Benign {
        /// Random-walk length.
        ops: u32,
    },
    /// The composition explorer: a weighted random walk over the whole
    /// vocabulary, operands drawn from everything constructed so far.
    Combination {
        /// Walk length.
        ops: u32,
    },
    /// The cross-universe regime: several independently seeded universes,
    /// each constructed by a random reduced family, measured ops drawing
    /// operands across them.
    Independent {
        /// Universe count (2..=4).
        universes: u32,
        /// Cross-battery size.
        ops: u32,
    },
    /// The small-operand family: one universe held at seed scale,
    /// cycling rumors' bootstrap hot path.
    ///
    /// The path is tick, a fork rejoined (the success join), and the
    /// clock codec round-trip — so the four small-band kernels
    /// ([`crate::bands::SMALL_BAND_KERNELS`]) sample densely below the
    /// fit floor, where every size-law leg is out of range by design.
    ///
    /// Not a roster draw: its region is judged deterministically (the
    /// corpus [`for_each_bootstrap_program`] enumerates feeds both the
    /// small-band calibration and the enforcement replay), and keeping it
    /// off [`any_family`] keeps the pinned calibration stream's draws
    /// unchanged.
    ///
    /// [`for_each_bootstrap_program`]: crate::drive::for_each_bootstrap_program
    Bootstrap {
        /// Growth rounds; the corpus cycles `1..=BOOTSTRAP_MAX_ROUNDS` so
        /// programs stop at staggered sizes across the sub-floor span.
        rounds: u32,
    },
    /// The reach family: one universe grown far past the roster's fork
    /// cap, under [`ESCALATION_BUDGET`].
    ///
    /// A size ladder of snapshot clocks lets the pair, fold, and query
    /// rows sample every half-decade bucket up to the escalated top —
    /// slope leverage for the wide-cloud kernels, and the large coupled
    /// `Party::join` regime. A cadence battery rides the ladder so the
    /// single-operand rows (ticks, sends, recvs, part splits and
    /// reassembly, party splits and differences) sample the same reach,
    /// and codec duplicates place aliased operands so every rejection arm
    /// (`join`/`sync` on overlap, `without` on emptiness) is priced at
    /// escalated size too — including one maximally-deferred overlap
    /// (detectable only deep in the id tree) between the two finished
    /// party halves.
    Escalation {
        /// Spine depth in forks: the reach knob.
        depth: u32,
    },
}

/// Slot state the builder tracks (mirrors the guest/mirror register file).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ty {
    V,
    P,
    C,
    R,
    /// Consumed, or of uncertain liveness after a possibly-rejecting op;
    /// never used again either way.
    Dead,
}

/// The principal values a family construction hands to the battery.
#[derive(Debug, Default, Clone)]
struct Pools {
    clocks: Vec<Reg>,
    versions: Vec<Reg>,
    parties: Vec<Reg>,
    ranks: Vec<Reg>,
}

/// The type-tracked program builder: emits ops, models liveness the way the
/// mirror will, and enforces the budget unconditionally (an out-of-budget
/// request is skipped, so any parameter draw stays within [`BUDGET`]).
struct B {
    ops: Vec<Op>,
    slots: Vec<Ty>,
    ticks: u32,
    forks: u32,
    rng: ChaCha8Rng,
    budget: Budget,
}

impl B {
    fn new(seed: u64, budget: Budget) -> B {
        B {
            ops: Vec::new(),
            slots: Vec::new(),
            ticks: 0,
            forks: 0,
            rng: ChaCha8Rng::seed_from_u64(seed),
            budget,
        }
    }

    fn alloc(&mut self, ty: Ty) -> Reg {
        self.slots.push(ty);
        (self.slots.len() - 1) as Reg
    }

    fn room(&self) -> bool {
        self.ops.len() < self.budget.max_ops
    }

    fn push(&mut self, op: Op) {
        debug_assert!(self.room(), "callers check room() before emitting");
        self.ops.push(op);
    }

    // ── constructors ──

    fn clock_seed(&mut self) -> Option<Reg> {
        if !self.room() {
            return None;
        }
        let dst = self.alloc(Ty::C);
        self.push(Op::ClockSeed { dst });
        Some(dst)
    }

    // ── clock ops ──

    fn tick(&mut self, c: Reg) -> bool {
        if !self.room() || self.ticks >= self.budget.max_ticks {
            return false;
        }
        self.ticks += 1;
        self.push(Op::ClockTick { c });
        true
    }

    fn tick_n(&mut self, c: Reg, n: u32) {
        for _ in 0..n {
            if !self.tick(c) {
                return;
            }
        }
    }

    fn fork(&mut self, src: Reg) -> Option<Reg> {
        if !self.room() || self.forks >= self.budget.max_forks {
            return None;
        }
        self.forks += 1;
        let dst = self.alloc(Ty::C);
        self.push(Op::ClockFork { dst, src });
        Some(dst)
    }

    /// Balanced share splitting by repeated doubling: `n` clocks of roughly
    /// equal interval depth (log₂ n), the population shape the fold
    /// families want.
    fn fork_balanced(&mut self, src: Reg, n: u32) -> Vec<Reg> {
        let mut pool = vec![src];
        while (pool.len() as u32) < n {
            let mut next = Vec::with_capacity(pool.len() * 2);
            for &c in &pool {
                next.push(c);
                if (next.len() as u32) < n || pool.len() * 2 <= n as usize {
                    match self.fork(c) {
                        Some(f) => next.push(f),
                        None => return pool,
                    }
                }
                if next.len() as u32 >= n {
                    break;
                }
            }
            if next.len() == pool.len() {
                return pool;
            }
            pool = next;
            if pool.len() as u32 >= n {
                break;
            }
        }
        pool.truncate(n as usize);
        pool
    }

    /// `Clock::join`, which may reject cross-universe operands: `b` is
    /// treated as dead under either outcome.
    fn clock_join(&mut self, a: Reg, b: Reg) {
        if self.room() {
            self.slots[b as usize] = Ty::Dead;
            self.push(Op::ClockJoin { a, b });
        }
    }

    /// Duplicate a clock through its canonical codec (encode, then
    /// decode the staged bytes).
    ///
    /// The API-reachable route to an aliased id inside one universe,
    /// which is how the reach family constructs overlap — and with it
    /// `join`/`sync` rejection arms at escalated size.
    fn clock_dup(&mut self, src: Reg) -> Option<Reg> {
        if self.ops.len() + 2 > self.budget.max_ops {
            return None;
        }
        self.push(Op::ClockEncode { src });
        let dst = self.alloc(Ty::C);
        self.push(Op::ClockDecode { dst });
        Some(dst)
    }

    fn send(&mut self, c: Reg) {
        if self.room() {
            self.push(Op::ClockSend { c });
        }
    }

    fn recv(&mut self, c: Reg, v: Reg) {
        if self.room() {
            self.push(Op::ClockRecv { c, v });
        }
    }

    fn sync(&mut self, a: Reg, b: Reg) {
        if self.room() {
            self.push(Op::ClockSync { a, b });
        }
    }

    fn version_of(&mut self, c: Reg) -> Option<Reg> {
        if !self.room() {
            return None;
        }
        let dst = self.alloc(Ty::V);
        self.push(Op::ClockVersion { dst, src: c });
        Some(dst)
    }

    fn own_version(&mut self, c: Reg) -> Option<Reg> {
        if !self.room() {
            return None;
        }
        let dst = self.alloc(Ty::V);
        self.push(Op::ClockOwnVersion { dst, src: c });
        Some(dst)
    }

    fn split_parts(&mut self, c: Reg) -> Option<(Reg, Reg)> {
        if !self.room() {
            return None;
        }
        self.slots[c as usize] = Ty::Dead;
        let dst_p = self.alloc(Ty::P);
        let dst_v = self.alloc(Ty::V);
        self.push(Op::ClockIntoParts {
            dst_p,
            dst_v,
            src: c,
        });
        Some((dst_p, dst_v))
    }

    fn assemble_parts(&mut self, p: Reg, v: Reg) -> Option<Reg> {
        if !self.room() {
            return None;
        }
        self.slots[p as usize] = Ty::Dead;
        self.slots[v as usize] = Ty::Dead;
        let dst = self.alloc(Ty::C);
        self.push(Op::ClockFromParts { dst, p, v });
        Some(dst)
    }

    // ── party ops ──

    fn party_fork(&mut self, src: Reg) -> Option<Reg> {
        if !self.room() || self.forks >= self.budget.max_forks {
            return None;
        }
        self.forks += 1;
        let dst = self.alloc(Ty::P);
        self.push(Op::PartyFork { dst, src });
        Some(dst)
    }

    /// `Party::join`: `b` is treated as dead under either outcome.
    fn party_join(&mut self, a: Reg, b: Reg) {
        if self.room() {
            self.slots[b as usize] = Ty::Dead;
            self.push(Op::PartyJoin { a, b });
        }
    }

    /// Duplicate a party through its canonical codec (see [`B::clock_dup`]
    /// for why: aliased regions are how one universe reaches the rejection
    /// arms at scale).
    fn party_dup(&mut self, src: Reg) -> Option<Reg> {
        if self.ops.len() + 2 > self.budget.max_ops {
            return None;
        }
        self.push(Op::PartyEncode { src });
        let dst = self.alloc(Ty::P);
        self.push(Op::PartyDecode { dst });
        Some(dst)
    }

    /// `Party::forks(n)`: balanced shares into `n` fresh contiguous slots,
    /// debiting the fork budget by `n`. Returns the base register.
    fn party_forks(&mut self, src: Reg, n: u32) -> Option<Reg> {
        if !self.room() || self.forks + n > self.budget.max_forks {
            return None;
        }
        self.forks += n;
        let dst = self.slots.len() as Reg;
        for _ in 0..n {
            self.alloc(Ty::P);
        }
        self.push(Op::PartyForks { dst, src, n });
        Some(dst)
    }

    /// `Party::without` into a fresh slot (`a` is consumed).
    ///
    /// When the difference can be empty (`may_reject`), the result slot
    /// is dead to later draws: the mirror reports emptiness as a
    /// rejection and the guest writes nothing.
    fn party_without(&mut self, a: Reg, b: Reg, may_reject: bool) -> Option<Reg> {
        if !self.room() {
            return None;
        }
        self.slots[a as usize] = Ty::Dead;
        let dst = self.alloc(Ty::P);
        if may_reject {
            self.slots[dst as usize] = Ty::Dead;
        }
        self.push(Op::PartyWithout { dst, a, b });
        Some(dst)
    }

    // ── version ops ──

    fn version_tick(&mut self, v: Reg, p: Reg) -> bool {
        if !self.room() || self.ticks >= self.budget.max_ticks {
            return false;
        }
        self.ticks += 1;
        self.push(Op::VersionTick { v, p });
        true
    }

    fn version_join(&mut self, a: Reg, b: Reg) -> Option<Reg> {
        if !self.room() {
            return None;
        }
        self.slots[a as usize] = Ty::Dead;
        let dst = self.alloc(Ty::V);
        self.push(Op::VersionJoin { dst, a, b });
        Some(dst)
    }

    fn version_meet(&mut self, a: Reg, b: Reg) -> Option<Reg> {
        if !self.room() {
            return None;
        }
        self.slots[a as usize] = Ty::Dead;
        let dst = self.alloc(Ty::V);
        self.push(Op::VersionMeet { dst, a, b });
        Some(dst)
    }

    fn project(&mut self, v: Reg, p: Reg) -> Option<Reg> {
        if !self.room() {
            return None;
        }
        let dst = self.alloc(Ty::V);
        self.push(Op::VersionProject { dst, v, p });
        Some(dst)
    }

    fn rank(&mut self, v: Reg) -> Option<Reg> {
        if !self.room() {
            return None;
        }
        let dst = self.alloc(Ty::R);
        self.push(Op::VersionRank { dst, src: v });
        Some(dst)
    }

    /// Extract versions from `clocks` in the given order into a fresh
    /// contiguous range and fold them (`join_all`, or occasionally
    /// `meet_all`, so both fold rows sample), respecting the fold budget.
    fn join_all_versions(&mut self, clocks: &[Reg]) -> Option<Reg> {
        let n = (clocks.len() as u32).min(self.budget.max_fold);
        if n == 0 || self.ops.len() + n as usize + 1 > self.budget.max_ops {
            return None;
        }
        let meet = self.rng.gen_bool(0.25);
        let base = self.slots.len() as Reg;
        for &c in &clocks[..n as usize] {
            let dst = self.alloc(Ty::V);
            self.push(Op::ClockVersion { dst, src: c });
            self.slots[dst as usize] = Ty::Dead; // consumed by the fold below
        }
        let dst = self.alloc(Ty::V);
        if meet {
            self.push(Op::VersionMeetAll { dst, src: base, n });
        } else {
            self.push(Op::VersionJoinAll { dst, src: base, n });
        }
        Some(dst)
    }

    // ── the measurement battery ──

    /// Append a randomized battery of query/codec/text/rank ops over the
    /// pools: where most per-operation samples come from. Consuming ops use
    /// spares re-extracted from clocks, never the pools' principals.
    ///
    /// The battery also *feeds* the rank pool: every distance/lag pair it
    /// emits joins `pools.ranks`, so the pool holds ranks of distinct
    /// magnitudes and `Rank::checked_sub` draws unequal operands in both
    /// orders — the `Greater` alignment-shift-subtract arm and the
    /// underflow rejection arm both sample, instead of the degenerate
    /// equal-operand `Rank::ZERO` path a single-rank pool would pin every
    /// draw to.
    fn battery(&mut self, pools: &mut Pools, rounds: u32) {
        for _ in 0..rounds {
            if !self.room() {
                return;
            }
            match self.rng.gen_range(0..12u32) {
                0 => {
                    // Codec round-trip on a random version.
                    if let Some(&v) = pick(&mut self.rng, &pools.versions) {
                        self.push(Op::VersionEncode { src: v });
                        if self.room() {
                            let dst = self.alloc(Ty::V);
                            self.push(Op::VersionDecode { dst });
                        }
                    }
                }
                1 => {
                    // Text round-trip on a random version.
                    if let Some(&v) = pick(&mut self.rng, &pools.versions) {
                        self.push(Op::VersionDisplay { src: v });
                        if self.room() {
                            let dst = self.alloc(Ty::V);
                            self.push(Op::VersionFromstr { dst });
                        }
                    }
                }
                2 => {
                    if let (Some(&a), Some(&b)) = (
                        pick(&mut self.rng, &pools.versions),
                        pick(&mut self.rng, &pools.versions),
                    ) {
                        self.push(Op::VersionCmp { a, b });
                        if self.room() {
                            self.push(Op::VersionConcurrent { a, b });
                        }
                    }
                }
                3 => {
                    if let (Some(&a), Some(&b)) = (
                        pick(&mut self.rng, &pools.versions),
                        pick(&mut self.rng, &pools.versions),
                    ) {
                        if self.ops.len() + 2 <= self.budget.max_ops {
                            let d1 = self.alloc(Ty::R);
                            self.push(Op::VersionDistance { dst: d1, a, b });
                            let d2 = self.alloc(Ty::R);
                            self.push(Op::VersionLag { dst: d2, a, b });
                            // Pool both outputs: distance dominates lag on
                            // the same operand pair, so the rank pool gains
                            // unequal values and checked_sub below samples
                            // both its subtraction and its underflow arm.
                            pools.ranks.push(d1);
                            pools.ranks.push(d2);
                        }
                    }
                }
                4 => {
                    if let Some(&v) = pick(&mut self.rng, &pools.versions) {
                        self.push(Op::VersionMinTicks { src: v });
                    }
                }
                5 => {
                    if let Some(&v) = pick(&mut self.rng, &pools.versions) {
                        let _ = self.rank(v);
                    }
                }
                6 => {
                    // Join/meet on spare versions extracted for the purpose.
                    if let (Some(&ca), Some(&vb)) = (
                        pick(&mut self.rng, &pools.clocks),
                        pick(&mut self.rng, &pools.versions),
                    ) {
                        if let Some(spare) = self.version_of(ca) {
                            if self.rng.gen_bool(0.5) {
                                self.version_join(spare, vb);
                            } else {
                                self.version_meet(spare, vb);
                            }
                        }
                    }
                }
                7 => {
                    if let (Some(&v), Some(&p)) = (
                        pick(&mut self.rng, &pools.versions),
                        pick(&mut self.rng, &pools.parties),
                    ) {
                        self.project(v, p);
                    }
                }
                8 => {
                    // Party queries and codecs.
                    if let (Some(&a), Some(&b)) = (
                        pick(&mut self.rng, &pools.parties),
                        pick(&mut self.rng, &pools.parties),
                    ) {
                        self.push(Op::PartyIsDisjoint { a, b });
                        if self.room() {
                            self.push(Op::PartyCovers { a, b });
                        }
                    }
                }
                9 => {
                    if let Some(&p) = pick(&mut self.rng, &pools.parties) {
                        self.push(Op::PartyEncode { src: p });
                        if self.room() {
                            let dst = self.alloc(Ty::P);
                            self.push(Op::PartyDecode { dst });
                        }
                        if self.room() {
                            self.push(Op::PartyDisplay { src: p });
                            if self.room() {
                                let dst = self.alloc(Ty::P);
                                self.push(Op::PartyFromstr { dst });
                            }
                        }
                    }
                }
                10 => {
                    if let Some(&c) = pick(&mut self.rng, &pools.clocks) {
                        self.push(Op::ClockEncode { src: c });
                        if self.room() {
                            let dst = self.alloc(Ty::C);
                            self.push(Op::ClockDecode { dst });
                        }
                        if self.room() {
                            // The clock-side output-dominated row.
                            let dst = self.alloc(Ty::V);
                            self.push(Op::ClockOwnVersion { dst, src: c });
                        }
                    }
                }
                _ => {
                    if let (Some(&a), Some(&b)) = (
                        pick(&mut self.rng, &pools.ranks),
                        pick(&mut self.rng, &pools.ranks),
                    ) {
                        self.push(Op::RankCmp { a, b });
                        if self.room() {
                            self.push(Op::RankDisplay { src: a });
                        }
                        if self.room() {
                            let dst = self.alloc(Ty::R);
                            self.slots[dst as usize] = Ty::Dead; // may underflow
                            self.push(Op::RankCheckedSub { dst, a, b });
                        }
                        if self.room() {
                            // The reverse order: whenever the operands
                            // differ, exactly one direction underflows, so
                            // the rejection arm samples at every size the
                            // rank pool reaches.
                            let dst = self.alloc(Ty::R);
                            self.slots[dst as usize] = Ty::Dead; // may underflow
                            self.push(Op::RankCheckedSub { dst, a: b, b: a });
                        }
                        if self.room() {
                            // RankAdd consumes `a`: re-derive a spare first.
                            if let Some(&v) = pick(&mut self.rng, &pools.versions) {
                                if let Some(spare) = self.rank(v) {
                                    if self.room() {
                                        self.slots[spare as usize] = Ty::Dead;
                                        let dst = self.alloc(Ty::R);
                                        self.push(Op::RankAdd { dst, a: spare, b });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Uniformly pick from a slice (None when empty).
fn pick<'s, T>(rng: &mut ChaCha8Rng, xs: &'s [T]) -> Option<&'s T> {
    xs.choose(rng)
}

/// Construct one family's values into `b`, returning the principal pools.
fn construct(b: &mut B, family: &Family) -> Pools {
    let mut pools = Pools::default();
    match *family {
        Family::DenseSpine {
            depth,
            ticks_per_level,
        } => {
            let Some(seed) = b.clock_seed() else {
                return pools;
            };
            let mut sink = None;
            for _ in 0..depth {
                let Some(child) = b.fork(seed) else { break };
                let jitter = b.rng.gen_range(0..=ticks_per_level);
                b.tick_n(seed, 1 + jitter);
                match sink {
                    None => sink = Some(child),
                    Some(s) => b.clock_join(s, child),
                }
            }
            if let Some(v) = b.version_of(seed) {
                pools.versions.push(v);
            }
            if let Some(r) = pools.versions.first().and_then(|&v| b.rank(v)) {
                pools.ranks.push(r);
            }
            pools.clocks.push(seed);
            if let Some(s) = sink {
                pools.clocks.push(s);
                if let Some((p, v)) = b.split_parts(s) {
                    pools.parties.push(p);
                    pools.versions.push(v);
                    pools.clocks.pop();
                }
            }
        }
        Family::BigRoot { root_ticks, depth } => {
            let Some(seed) = b.clock_seed() else {
                return pools;
            };
            b.tick_n(seed, root_ticks);
            let mut deepest = seed;
            for _ in 0..depth {
                let Some(child) = b.fork(deepest) else { break };
                b.tick_n(child, 1);
                deepest = child;
            }
            pools.clocks.push(seed);
            if let Some(v) = b.version_of(deepest) {
                pools.versions.push(v);
            }
            if deepest != seed {
                if let Some((p, v)) = b.split_parts(deepest) {
                    pools.parties.push(p);
                    pools.versions.push(v);
                }
            }
        }
        Family::HugeLeaf { ticks } => {
            let Some(seed) = b.clock_seed() else {
                return pools;
            };
            b.tick_n(seed, ticks);
            pools.clocks.push(seed);
            if let Some(v) = b.version_of(seed) {
                pools.versions.push(v);
                if let Some(r) = b.rank(v) {
                    pools.ranks.push(r);
                }
            }
        }
        Family::CliffComb { teeth, magnitude } => {
            let Some(seed) = b.clock_seed() else {
                return pools;
            };
            let shares = b.fork_balanced(seed, teeth);
            let high = (1u32 << magnitude.min(MAGNITUDE_SHIFT_CAP)).saturating_sub(1);
            for (i, &tooth) in shares.iter().enumerate() {
                let n = if i % 2 == 0 {
                    high + b.rng.gen_range(0..=2)
                } else {
                    b.rng.gen_range(0..=1)
                };
                b.tick_n(tooth, n);
            }
            if let Some(comb) = b.join_all_versions(&shares) {
                pools.versions.push(comb);
                if let Some(r) = b.rank(comb) {
                    pools.ranks.push(r);
                }
            }
            if let Some((p, v)) = shares.first().copied().and_then(|c| b.split_parts(c)) {
                pools.parties.push(p);
                pools.versions.push(v);
            }
            pools.clocks.extend(shares.iter().skip(1).copied());
        }
        Family::IdPairLockstep { depth, divert } => {
            let Some(seed) = b.clock_seed() else {
                return pools;
            };
            let Some(other) = b.fork(seed) else {
                pools.clocks.push(seed);
                return pools;
            };
            let (mut a, mut bb) = (seed, other);
            for _ in 0..depth {
                let (Some(fa), Some(fb)) = (b.fork(a), b.fork(bb)) else {
                    break;
                };
                // The diverted walk keeps opposite sides in the two lanes.
                if divert {
                    a = fa;
                } else {
                    bb = fb;
                }
                let (ja, jb) = (b.rng.gen_range(0..=1), b.rng.gen_range(0..=1));
                b.tick_n(a, ja);
                b.tick_n(bb, jb);
            }
            pools.clocks.push(a);
            pools.clocks.push(bb);
            if let Some((p, v)) = b.split_parts(a) {
                pools.parties.push(p);
                pools.versions.push(v);
                pools.clocks.remove(0);
            }
            if let Some((p, v)) = b.split_parts(bb) {
                pools.parties.push(p);
                pools.versions.push(v);
                pools.clocks.pop();
            }
            // The pair walk: tick the version of one lane with the party of
            // the other (the id-walk rows' operand shape).
            if let (Some(&v), Some(&p)) = (pools.versions.first(), pools.parties.last()) {
                b.version_tick(v, p);
            }
        }
        Family::CombScatter { teeth, magnitude } => {
            let Some(seed) = b.clock_seed() else {
                return pools;
            };
            let shares = b.fork_balanced(seed, teeth);
            let high = (1u32 << magnitude.min(MAGNITUDE_SHIFT_CAP)).saturating_sub(1);
            for (i, &tooth) in shares.iter().enumerate() {
                b.tick_n(tooth, if i % 2 == 0 { high } else { 0 });
            }
            let comb = b.join_all_versions(&shares);
            // The scattered party: every other share's party joined into
            // one (adjacent intervals never merge).
            let mut scattered: Option<Reg> = None;
            for &c in shares.iter().step_by(2) {
                if let Some((p, _v)) = b.split_parts(c) {
                    match scattered {
                        None => scattered = Some(p),
                        Some(acc) => {
                            if b.room() {
                                b.slots[p as usize] = Ty::Dead;
                                b.push(Op::PartyJoin { a: acc, b: p });
                            }
                        }
                    }
                }
            }
            if let (Some(comb), Some(scattered)) = (comb, scattered) {
                pools.versions.push(comb);
                pools.parties.push(scattered);
                // The output-dominated rows, on their designed operands.
                b.project(comb, scattered);
                if let Some(c) = shares.iter().skip(1).step_by(2).next().copied() {
                    b.own_version(c);
                    pools.clocks.push(c);
                }
            }
        }
        Family::Harmonic { depth, total_ticks } => {
            let Some(seed) = b.clock_seed() else {
                return pools;
            };
            let mut cur = seed;
            for i in 0..depth {
                b.tick_n(cur, total_ticks / (i + 1));
                let Some(child) = b.fork(cur) else { break };
                cur = child;
            }
            pools.clocks.push(seed);
            if let Some(v) = b.version_of(cur) {
                pools.versions.push(v);
                if let Some(r) = b.rank(v) {
                    pools.ranks.push(r);
                }
            }
            if cur != seed {
                if let Some((p, v)) = b.split_parts(cur) {
                    pools.parties.push(p);
                    pools.versions.push(v);
                }
            }
        }
        Family::ScatterFold { clocks } => {
            let Some(seed) = b.clock_seed() else {
                return pools;
            };
            let mut shares = b.fork_balanced(seed, clocks);
            for &c in &shares {
                b.tick_n(c, 1);
            }
            // Adversarial fold order: shuffled, so the accumulator never
            // coalesces.
            let mut order = std::mem::take(&mut shares);
            order.shuffle(&mut b.rng);
            // The width ladder: shuffled folds at doubling widths below
            // the full population, so every case samples the fold rows
            // along the *width* axis — the axis a degenerate (left-fold)
            // reduction is quadratic in, and one that byte-size reach
            // cannot stand in for.
            let mut width = 8usize;
            while width < order.len() {
                let mut subset = order.clone();
                subset.shuffle(&mut b.rng);
                subset.truncate(width);
                b.join_all_versions(&subset);
                width *= 2;
            }
            if let Some(folded) = b.join_all_versions(&order) {
                pools.versions.push(folded);
            }
            // A party fold in the same shuffled order.
            let mut parts: Vec<Reg> = Vec::new();
            for &c in order.iter().take(16) {
                if let Some((p, _v)) = b.split_parts(c) {
                    parts.push(p);
                }
            }
            if let Some((&acc, rest)) = parts.split_first() {
                for &p in rest {
                    if b.room() {
                        b.slots[p as usize] = Ty::Dead;
                        b.push(Op::PartyJoin { a: acc, b: p });
                    }
                }
                pools.parties.push(acc);
            }
            pools.clocks.extend(order.into_iter().skip(16));
        }
        Family::NestedFull { depth, ticks } => {
            let Some(seed) = b.clock_seed() else {
                return pools;
            };
            let mut cur = seed;
            for _ in 0..depth {
                let Some(child) = b.fork(cur) else { break };
                // Full siblings: both sides get the same count.
                b.tick_n(cur, ticks);
                b.tick_n(child, ticks);
                cur = child;
            }
            pools.clocks.push(seed);
            if let Some(v) = b.version_of(cur) {
                pools.versions.push(v);
            }
            if cur != seed {
                if let Some((p, v)) = b.split_parts(cur) {
                    pools.parties.push(p);
                    pools.versions.push(v);
                    // The designated cross: the deep id walking the full tree.
                    if let Some(&ver) = pools.versions.first() {
                        b.version_tick(ver, p);
                    }
                }
            }
        }
        Family::WideTail { tail_ticks, depth } => {
            let Some(seed) = b.clock_seed() else {
                return pools;
            };
            let mut cur = seed;
            for _ in 0..depth {
                b.tick_n(cur, 1);
                let Some(child) = b.fork(cur) else { break };
                cur = child;
            }
            b.tick_n(cur, tail_ticks);
            pools.clocks.push(seed);
            if let Some(v) = b.version_of(cur) {
                pools.versions.push(v);
                if let Some(r) = b.rank(v) {
                    pools.ranks.push(r);
                }
            }
            if cur != seed {
                if let Some((p, v)) = b.split_parts(cur) {
                    pools.parties.push(p);
                    pools.versions.push(v);
                    if let Some(&ver) = pools.versions.first() {
                        b.version_tick(ver, p);
                    }
                }
            }
        }
        Family::Staircase { depth } => {
            let Some(seed) = b.clock_seed() else {
                return pools;
            };
            let mut cur = seed;
            for i in 0..depth {
                b.tick_n(cur, depth - i);
                let Some(child) = b.fork(cur) else { break };
                cur = child;
            }
            pools.clocks.push(seed);
            if let Some(v) = b.version_of(cur) {
                pools.versions.push(v);
            }
            if cur != seed {
                if let Some((p, v)) = b.split_parts(cur) {
                    pools.parties.push(p);
                    pools.versions.push(v);
                    if let Some(&ver) = pools.versions.first() {
                        b.version_tick(ver, p);
                    }
                }
            }
        }
        Family::RevealComb {
            teeth,
            magnitude,
            hifloor,
        } => {
            let Some(seed) = b.clock_seed() else {
                return pools;
            };
            let shares = b.fork_balanced(seed, teeth);
            let site = (1u32 << magnitude.min(MAGNITUDE_SHIFT_CAP)).saturating_sub(1);
            let floor = if hifloor { site.saturating_sub(2) } else { 0 };
            for (i, &tooth) in shares.iter().enumerate() {
                // Equal-sibling sites over the floor.
                b.tick_n(tooth, if i % 2 == 0 { site } else { floor });
            }
            // The close-reveal cycle: adjacent joins, one at a time, each
            // consume revealing the shared minimum to the floor frame.
            let mut acc = match shares.first().copied().and_then(|c| b.version_of(c)) {
                Some(v) => v,
                None => return pools,
            };
            for &c in shares.iter().skip(1) {
                let Some(next) = b.version_of(c) else { break };
                match b.version_join(acc, next) {
                    Some(joined) => acc = joined,
                    None => break,
                }
            }
            pools.versions.push(acc);
            pools.clocks.extend(shares);
        }
        Family::PureComb { teeth } => {
            let Some(seed) = b.clock_seed() else {
                return pools;
            };
            let shares = b.fork_balanced(seed, teeth);
            for (i, &tooth) in shares.iter().enumerate() {
                // Distinct values everywhere: no equal-sibling site.
                b.tick_n(tooth, i as u32 + 1);
            }
            let mut acc = match shares.first().copied().and_then(|c| b.version_of(c)) {
                Some(v) => v,
                None => return pools,
            };
            for &c in shares.iter().skip(1) {
                let Some(next) = b.version_of(c) else { break };
                match b.version_join(acc, next) {
                    Some(joined) => acc = joined,
                    None => break,
                }
            }
            pools.versions.push(acc);
            pools.clocks.extend(shares);
        }
        Family::AscendCliff { teeth, plateau } => {
            let Some(seed) = b.clock_seed() else {
                return pools;
            };
            let shares = b.fork_balanced(seed, teeth);
            let n = shares.len() as u32;
            for (i, &tooth) in shares.iter().enumerate() {
                // Ascending stack (or the leveled control), terminal zero
                // cliff: the last tooth stays at zero.
                let val = if i + 1 == shares.len() {
                    0
                } else if plateau {
                    n
                } else {
                    i as u32 + 1
                };
                b.tick_n(tooth, val);
            }
            // Fold right-to-left so the zero cliff undercuts every stacked
            // difference.
            let mut rev: Vec<Reg> = shares.iter().rev().copied().collect();
            if let Some(first) = rev.pop() {
                rev.insert(0, first);
            }
            if let Some(folded) = b.join_all_versions(&rev) {
                pools.versions.push(folded);
            }
            pools.clocks.extend(shares);
        }
        Family::Benign { ops } => {
            let Some(seed) = b.clock_seed() else {
                return pools;
            };
            let mut clocks = vec![seed];
            for _ in 0..ops {
                match b.rng.gen_range(0..10u32) {
                    0..=4 => {
                        if let Some(&c) = pick(&mut b.rng, &clocks) {
                            b.tick(c);
                        }
                    }
                    5..=6 => {
                        if clocks.len() < 24 {
                            if let Some(&c) = pick(&mut b.rng, &clocks) {
                                if let Some(f) = b.fork(c) {
                                    clocks.push(f);
                                }
                            }
                        }
                    }
                    7..=8 => {
                        if clocks.len() >= 2 {
                            let i = b.rng.gen_range(0..clocks.len());
                            let j = b.rng.gen_range(0..clocks.len());
                            if i != j {
                                b.send(clocks[i]);
                                if let Some(v) = b.version_of(clocks[i]) {
                                    b.recv(clocks[j], v);
                                }
                            }
                        }
                    }
                    _ => {
                        if clocks.len() >= 3 {
                            let bidx = b.rng.gen_range(1..clocks.len());
                            let removed = clocks.swap_remove(bidx);
                            b.clock_join(clocks[0], removed);
                        }
                    }
                }
            }
            if let Some(&c) = clocks.first() {
                if let Some(v) = b.version_of(c) {
                    pools.versions.push(v);
                    if let Some(r) = b.rank(v) {
                        pools.ranks.push(r);
                    }
                }
            }
            if clocks.len() >= 2 {
                if let Some((p, v)) = b.split_parts(clocks.pop().expect("len checked")) {
                    pools.parties.push(p);
                    pools.versions.push(v);
                }
            }
            pools.clocks.extend(clocks);
        }
        Family::Combination { ops } => {
            let Some(seed) = b.clock_seed() else {
                return pools;
            };
            pools.clocks.push(seed);
            for _ in 0..ops {
                if !b.room() {
                    break;
                }
                match b.rng.gen_range(0..20u32) {
                    0..=5 => {
                        if let Some(&c) = pick(&mut b.rng, &pools.clocks) {
                            b.tick(c);
                        }
                    }
                    6..=7 => {
                        if pools.clocks.len() < 48 {
                            if let Some(&c) = pick(&mut b.rng, &pools.clocks) {
                                if let Some(f) = b.fork(c) {
                                    pools.clocks.push(f);
                                }
                            }
                        }
                    }
                    8 => {
                        if pools.clocks.len() >= 3 {
                            let i = b.rng.gen_range(1..pools.clocks.len());
                            let removed = pools.clocks.swap_remove(i);
                            b.clock_join(pools.clocks[0], removed);
                        }
                    }
                    9 => {
                        if pools.clocks.len() >= 2 {
                            let i = b.rng.gen_range(0..pools.clocks.len());
                            let j = b.rng.gen_range(0..pools.clocks.len());
                            if i != j {
                                b.sync(pools.clocks[i], pools.clocks[j]);
                            }
                        }
                    }
                    10..=11 => {
                        if let Some(&c) = pick(&mut b.rng, &pools.clocks) {
                            b.send(c);
                            if let Some(v) = b.version_of(c) {
                                pools.versions.push(v);
                            }
                        }
                    }
                    12 => {
                        if let (Some(&c), Some(&v)) = (
                            pick(&mut b.rng, &pools.clocks),
                            pick(&mut b.rng, &pools.versions),
                        ) {
                            b.recv(c, v);
                        }
                    }
                    13 => {
                        if let (Some(&v), Some(&p)) = (
                            pick(&mut b.rng, &pools.versions),
                            pick(&mut b.rng, &pools.parties),
                        ) {
                            b.version_tick(v, p);
                        }
                    }
                    14 => {
                        // Split a clock out into parts; rebuild sometimes.
                        if pools.clocks.len() >= 2 {
                            let c = pools.clocks.pop().expect("len checked");
                            if let Some((p, v)) = b.split_parts(c) {
                                if b.rng.gen_bool(0.4) {
                                    if let Some(rebuilt) = b.assemble_parts(p, v) {
                                        pools.clocks.push(rebuilt);
                                    }
                                } else {
                                    pools.parties.push(p);
                                    pools.versions.push(v);
                                }
                            }
                        }
                    }
                    15 => {
                        // A small shuffled fold.
                        if pools.clocks.len() >= 4 {
                            let mut sample = pools.clocks.clone();
                            sample.shuffle(&mut b.rng);
                            sample.truncate(b.rng.gen_range(2..=8.min(sample.len())));
                            if let Some(folded) = b.join_all_versions(&sample) {
                                pools.versions.push(folded);
                            }
                        }
                    }
                    16 => {
                        if let Some(&p) = pick(&mut b.rng, &pools.parties) {
                            if let Some(dst) = b.party_fork(p) {
                                pools.parties.push(dst);
                            }
                        }
                    }
                    17 => {
                        // A balanced party split through the forks iterator.
                        if let Some(&p) = pick(&mut b.rng, &pools.parties) {
                            let n = b.rng.gen_range(2..=6u32);
                            if let Some(dst) = b.party_forks(p, n) {
                                pools.parties.extend(dst..dst + n);
                            }
                        }
                    }
                    18 => {
                        // Party difference on a spare fork (the difference
                        // may be empty).
                        if let Some(&p) = pick(&mut b.rng, &pools.parties) {
                            if let Some(spare) = b.party_fork(p) {
                                b.party_without(spare, p, true);
                            }
                        }
                    }
                    _ => {
                        b.battery(&mut pools, 1);
                    }
                }
            }
        }
        Family::Bootstrap { rounds } => {
            let Some(seed) = b.clock_seed() else {
                return pools;
            };
            // The dense-spine shape at seed scale: each round forks a
            // child (halving the seed's interval, so the seed's next
            // tick lands at a fresh position and its packed bits grow
            // with the rounds) and folds the child into a sink — a
            // *success* join whose operands grow round over round. A
            // fork-and-rejoin schedule would not do: rejoining restores
            // the interval, later events lift into counters, and the
            // whole corpus stays pinned at the very bottom of the span.
            let mut sink: Option<Reg> = None;
            for _ in 0..rounds {
                if let Some(child) = b.fork(seed) {
                    b.tick(seed);
                    let jitter = b.rng.gen_range(1..=2);
                    b.tick_n(child, jitter);
                    match sink {
                        None => sink = Some(child),
                        Some(s) => b.clock_join(s, child),
                    }
                }
                // The clock codec round-trip at the current size.
                b.clock_dup(seed);
                b.tick(seed);
            }
            pools.clocks.push(seed);
            if let Some(s) = sink {
                pools.clocks.push(s);
            }
        }
        Family::Escalation { depth } => {
            let Some(seed) = b.clock_seed() else {
                return pools;
            };
            // Two scattered party accumulators over alternating spine
            // children: disjoint by linearity (one universe, no id sliver
            // duplicated — until the one deliberate poison below), each a
            // scatter of O(depth) never-coalescing intervals — so every
            // accumulation step below is a *successful* coupled
            // `Party::join` at a strictly growing size, and the two
            // finished halves are the large coupled query operands.
            let mut acc = [None::<Reg>, None::<Reg>];
            // Snapshot cadence: ~40 kept clocks, enough to populate every
            // half-decade fit bucket across the reach while staying far
            // under the fold budget, so the escalated fold below can take
            // the whole ladder.
            let keep_every = (depth / 40).max(4);
            // The deep-overlap poison: one deep child's party rides into
            // *both* lanes (via its codec duplicate), so the finished
            // halves overlap in exactly one interval far down the id tree
            // and the top-size rejections below do detection work that
            // scales with the operands — the maximally-deferred overlap.
            let poison_at = {
                let i = 3 * depth / 4;
                if i % keep_every == 0 {
                    i + 1
                } else {
                    i
                }
            };
            let mut cadence = 0u32;
            // The previous rung's version: the meet ladder's second
            // operand.
            let mut prev_v: Option<Reg> = None;
            for i in 0..depth {
                let Some(child) = b.fork(seed) else { break };
                // One tick per level: the seed's id deepens every fork, so
                // each tick lands at a fresh position and the event tree's
                // packed bits grow linearly with depth (repeat ticks at one
                // position would only bump a counter).
                if !b.tick(seed) {
                    break;
                }
                if i % keep_every == 0 {
                    // The size ladder: each kept child freezes the seed's
                    // event tree at this depth, so the kept clocks span
                    // the whole reach rather than just its endpoints. The
                    // cadence battery rides here: the single-operand rows
                    // (send/recv, version ticks, part splits and
                    // reassembly, party splits and differences) each
                    // sample this depth's operand size, so their reach is
                    // the ladder's reach.
                    b.send(child);
                    if let Some(v) = b.version_of(child) {
                        b.recv(seed, v);
                        if let Some(lane) = acc[0] {
                            b.version_tick(v, lane);
                        }
                        // The meet ladder: this rung's version against the
                        // previous rung's, on a spare — so the widest
                        // thin-per-case row (`version_meet`) carries
                        // within-case shape evidence across the reach in
                        // every escalated draw (the shape leg abstains
                        // without it, leaving mild smooth superlinearity
                        // to the point leg's much looser ceiling).
                        if let Some(prev) = prev_v {
                            if let Some(spare) = b.version_of(child) {
                                b.version_meet(spare, prev);
                            }
                        }
                        prev_v = Some(v);
                    }
                    // The party ladder, rotating arms over the lane so no
                    // single cadence point pays them all: a fork rejoined
                    // (split and reunite at size), balanced shares
                    // rejoined, a difference that succeeds (a fresh share
                    // minus the disjoint remainder is the share itself),
                    // and a difference that rejects (the lane's codec
                    // duplicate minus the lane is empty). Every arm leaves
                    // the lane exactly as it found it, so the
                    // accumulation's growth is untouched.
                    if let Some(lane) = acc[(cadence % 2) as usize] {
                        match cadence % 4 {
                            0 => {
                                if let Some(spare) = b.party_fork(lane) {
                                    b.party_join(lane, spare);
                                }
                                // The join rejection at this size: the
                                // lane's codec duplicate overlaps it
                                // everywhere, so the rejection arm
                                // samples every ladder rung, not just
                                // the deferred-overlap top.
                                if let Some(dup) = b.party_dup(lane) {
                                    b.party_join(lane, dup);
                                }
                            }
                            1 => {
                                // Reserve op room for the n rejoins riding
                                // on the split before emitting it (the
                                // split itself is one op).
                                let n = 3u32;
                                if b.ops.len() + (n as usize) < b.budget.max_ops {
                                    if let Some(dst) = b.party_forks(lane, n) {
                                        for k in 0..n {
                                            b.party_join(lane, dst + k);
                                        }
                                    }
                                }
                            }
                            2 => {
                                if let Some(spare) = b.party_fork(lane) {
                                    if let Some(dst) = b.party_without(spare, lane, false) {
                                        b.party_join(lane, dst);
                                    }
                                }
                            }
                            _ => {
                                // Empty difference (equal regions): the
                                // rejection arm at this size, both
                                // operands lane-sized; the duplicate is
                                // consumed and the lane never touched.
                                if let Some(dup) = b.party_dup(lane) {
                                    b.party_without(dup, lane, true);
                                }
                            }
                        }
                    }
                    // The from_parts ladder: split the snapshot and
                    // reassemble it, so both part conversions sample this
                    // size; the rebuilt clock is byte-identical to the
                    // child and takes its ladder slot. (A split whose
                    // reassembly ran out of room leaves no whole clock to
                    // keep, so that ladder slot is skipped.)
                    let snap = match b.split_parts(child) {
                        Some((p, v)) => b.assemble_parts(p, v),
                        None => Some(child),
                    };
                    if let Some(snap) = snap {
                        // The clock rejection ladder: a codec duplicate
                        // shares the snapshot's id — a single interval
                        // deep in the tree — so `join`/`sync` must
                        // descend to it to find the overlap: rejection
                        // arms whose detection work scales with this
                        // size.
                        if cadence.is_multiple_of(2) {
                            if let Some(dup) = b.clock_dup(snap) {
                                if cadence.is_multiple_of(4) {
                                    b.clock_join(snap, dup);
                                } else {
                                    b.sync(snap, dup);
                                }
                            }
                        }
                        pools.clocks.push(snap);
                    }
                    cadence += 1;
                } else if let Some((p, _v)) = b.split_parts(child) {
                    let lane_idx = (i % 2) as usize;
                    if i == poison_at {
                        if let (Some(dup), Some(other)) = (b.party_dup(p), acc[1 - lane_idx]) {
                            b.party_join(other, dup);
                        }
                    }
                    let lane = &mut acc[lane_idx];
                    match *lane {
                        None => *lane = Some(p),
                        Some(a) => b.party_join(a, p),
                    }
                }
            }
            // The coupled large-party rows: queries between the two
            // halves at top size — each scan runs deep before the poison's
            // single shared interval settles the answer — then the largest
            // `Party::join` the harness constructs, the rejection arm at
            // full scale with its overlap maximally deferred.
            if let [Some(pa), Some(pb)] = acc {
                if b.room() {
                    b.push(Op::PartyIsDisjoint { a: pa, b: pb });
                }
                if b.room() {
                    b.push(Op::PartyCovers { a: pa, b: pb });
                }
                b.party_join(pa, pb);
                pools.parties.push(pa);
            }
            // Clock rows at top size: sync the seed against the deepest
            // snapshot (same universe, disjoint: the success arm at
            // scale).
            if let Some(&snap) = pools.clocks.last() {
                b.sync(seed, snap);
            }
            if let Some(v) = b.version_of(seed) {
                pools.versions.push(v);
                if let Some(r) = b.rank(v) {
                    pools.ranks.push(r);
                }
            }
            pools.clocks.push(seed);
            // The escalated fold: versions extracted from the whole
            // ladder, seed included — fold denominators sum the ladder.
            let ladder = pools.clocks.clone();
            if let Some(folded) = b.join_all_versions(&ladder) {
                pools.versions.push(folded);
            }
            // The coupled `Clock::join` success arm at scale: the two
            // deepest snapshots are disjoint shares of one universe, so
            // their join succeeds at top size (the consumed snapshot
            // leaves the pools).
            if pools.clocks.len() >= 3 {
                let a = pools.clocks[pools.clocks.len() - 3];
                let consumed = pools.clocks.remove(pools.clocks.len() - 2);
                b.clock_join(a, consumed);
            }
        }
        Family::Independent { .. } => {
            unreachable!("Independent is expanded by build(), not construct()")
        }
    }
    pools
}

/// A reduced-parameter family for one universe of the independent regime.
fn reduced_family(rng: &mut ChaCha8Rng) -> Family {
    match rng.gen_range(0..6u32) {
        0 => Family::DenseSpine {
            depth: rng.gen_range(2..=48),
            ticks_per_level: rng.gen_range(0..=2),
        },
        1 => Family::HugeLeaf {
            ticks: rng.gen_range(1..=300),
        },
        2 => Family::CliffComb {
            teeth: rng.gen_range(2..=12),
            magnitude: rng.gen_range(1..=6),
        },
        3 => Family::Harmonic {
            depth: rng.gen_range(2..=24),
            total_ticks: rng.gen_range(4..=120),
        },
        4 => Family::Staircase {
            depth: rng.gen_range(2..=24),
        },
        _ => Family::Benign {
            ops: rng.gen_range(8..=80),
        },
    }
}

/// Build the full program for one family draw: construction, then the
/// measurement battery (cross-universe for the independent regime).
pub fn build(family: &Family, seed: u64) -> Vec<Op> {
    let mut b = B::new(seed, budget_for(family));
    match *family {
        Family::Independent { universes, ops } => {
            let mut per_universe: Vec<Pools> = Vec::new();
            for _ in 0..universes.clamp(2, 4) {
                let sub = reduced_family(&mut b.rng);
                per_universe.push(construct(&mut b, &sub));
            }
            // The cross battery: operands deliberately drawn from different
            // universes. Results are meaningless; costs are the claim.
            for _ in 0..ops {
                if !b.room() || per_universe.len() < 2 {
                    break;
                }
                let i = b.rng.gen_range(0..per_universe.len());
                let j = (i + b.rng.gen_range(1..per_universe.len())) % per_universe.len();
                let (ui, uj) = (per_universe[i].clone(), per_universe[j].clone());
                match b.rng.gen_range(0..13u32) {
                    0 => {
                        if let (Some(&a), Some(&bb)) = (
                            pick(&mut b.rng, &ui.versions),
                            pick(&mut b.rng, &uj.versions),
                        ) {
                            b.push(Op::VersionCmp { a, b: bb });
                            if b.room() {
                                b.push(Op::VersionConcurrent { a, b: bb });
                            }
                        }
                    }
                    1 => {
                        if let (Some(&a), Some(&bb)) = (
                            pick(&mut b.rng, &ui.versions),
                            pick(&mut b.rng, &uj.versions),
                        ) {
                            if b.ops.len() + 2 <= b.budget.max_ops {
                                let d1 = b.alloc(Ty::R);
                                b.push(Op::VersionDistance { dst: d1, a, b: bb });
                                let d2 = b.alloc(Ty::R);
                                b.push(Op::VersionLag { dst: d2, a, b: bb });
                            }
                        }
                    }
                    2 => {
                        // Cross join/meet on a spare extracted copy.
                        if let (Some(&ca), Some(&vb)) =
                            (pick(&mut b.rng, &ui.clocks), pick(&mut b.rng, &uj.versions))
                        {
                            if let Some(spare) = b.version_of(ca) {
                                if b.rng.gen_bool(0.5) {
                                    b.version_join(spare, vb);
                                } else {
                                    b.version_meet(spare, vb);
                                }
                            }
                        }
                    }
                    3 => {
                        // Cross tick and cross projection: a version walked
                        // by a foreign party.
                        if let (Some(&v), Some(&p)) = (
                            pick(&mut b.rng, &ui.versions),
                            pick(&mut b.rng, &uj.parties),
                        ) {
                            b.version_tick(v, p);
                            if b.room() {
                                b.project(v, p);
                            }
                        }
                    }
                    4 => {
                        if let (Some(&a), Some(&bb)) =
                            (pick(&mut b.rng, &ui.parties), pick(&mut b.rng, &uj.parties))
                        {
                            b.push(Op::PartyIsDisjoint { a, b: bb });
                            if b.room() {
                                b.push(Op::PartyCovers { a, b: bb });
                            }
                        }
                    }
                    5 => {
                        // Cross party join: overlap likely, the rejection
                        // arm's cost is the sample.
                        if let (Some(&a), Some(&bb)) =
                            (pick(&mut b.rng, &ui.parties), pick(&mut b.rng, &uj.parties))
                        {
                            if b.room() {
                                b.slots[bb as usize] = Ty::Dead;
                                b.push(Op::PartyJoin { a, b: bb });
                                // The pool copy still lists `bb`; drop it
                                // from the source of truth as well.
                                per_universe[j].parties.retain(|&r| r != bb);
                            }
                        }
                    }
                    6 => {
                        if let (Some(&ca), Some(&cb)) =
                            (pick(&mut b.rng, &ui.clocks), pick(&mut b.rng, &uj.clocks))
                        {
                            if ca != cb {
                                b.sync(ca, cb);
                            }
                        }
                    }
                    7 => {
                        if let (Some(&c), Some(&v)) =
                            (pick(&mut b.rng, &ui.clocks), pick(&mut b.rng, &uj.versions))
                        {
                            b.recv(c, v);
                        }
                    }
                    8 => {
                        // Cross clock join: separately seeded universes
                        // always overlap, so this is `Clock::join`'s
                        // rejection arm, priced as its own outcome (the
                        // mirror predicts it per case).
                        if let (Some(&ca), Some(&cb)) =
                            (pick(&mut b.rng, &ui.clocks), pick(&mut b.rng, &uj.clocks))
                        {
                            b.clock_join(ca, cb);
                            per_universe[j].clocks.retain(|&r| r != cb);
                        }
                    }
                    9 => {
                        // Cross reassembly: `from_parts` composes a party
                        // and a version from different universes into a
                        // mongrel clock — meaningless as a value, but the
                        // cost claim binds on it. Spare halves, so the
                        // universes' principals survive.
                        if let (Some(&p), Some(&c)) =
                            (pick(&mut b.rng, &ui.parties), pick(&mut b.rng, &uj.clocks))
                        {
                            if let Some(spare_p) = b.party_fork(p) {
                                if let Some(spare_v) = b.version_of(c) {
                                    b.assemble_parts(spare_p, spare_v);
                                }
                            }
                        }
                    }
                    10 => {
                        // Cross fold: operands drawn from every universe
                        // at once, shuffled, so the fold rows see
                        // interleaved foreign shapes.
                        let mut mixed: Vec<Reg> = per_universe
                            .iter()
                            .flat_map(|u| u.clocks.iter().copied())
                            .collect();
                        if mixed.len() >= 2 {
                            mixed.shuffle(&mut b.rng);
                            mixed.truncate(b.rng.gen_range(2..=16.min(mixed.len())));
                            b.join_all_versions(&mixed);
                        }
                    }
                    11 => {
                        // Cross difference: a spare share minus a foreign
                        // party. Foreign coverage makes the empty
                        // difference likely, so `Party::without`'s
                        // rejection arm is priced beside its success arm,
                        // per the mirror's prediction.
                        if let (Some(&pi), Some(&pj)) =
                            (pick(&mut b.rng, &ui.parties), pick(&mut b.rng, &uj.parties))
                        {
                            if let Some(spare) = b.party_fork(pi) {
                                b.party_without(spare, pj, true);
                            }
                        }
                    }
                    _ => {
                        // A bare seed party as the extreme foreign operand:
                        // it overlaps every other universe's party (never
                        // disjoint, join always rejects) and covers them.
                        if let Some(&p) = pick(&mut b.rng, &uj.parties) {
                            if b.room() {
                                let fresh = b.alloc(Ty::P);
                                b.push(Op::PartySeed { dst: fresh });
                                if b.room() {
                                    b.push(Op::PartyIsDisjoint { a: fresh, b: p });
                                }
                                if b.room() {
                                    b.push(Op::PartyCovers { a: fresh, b: p });
                                }
                                if b.room() {
                                    b.slots[p as usize] = Ty::Dead;
                                    b.push(Op::PartyJoin { a: fresh, b: p });
                                    per_universe[j].parties.retain(|&r| r != p);
                                }
                            }
                        }
                    }
                }
            }
        }
        // The bootstrap family is its construction: the battery's mixed
        // draws would dilute the four-kernel schedule the small bands
        // calibrate on without adding reach (everything here is sub-floor
        // by design).
        Family::Bootstrap { .. } => {
            construct(&mut b, family);
        }
        ref coupled => {
            let mut pools = construct(&mut b, coupled);
            let rounds = b.rng.gen_range(8..=32);
            b.battery(&mut pools, rounds);
        }
    }
    b.ops
}

/// The proptest strategy over every family: what the enforcement suite
/// draws from. Sizes are biased upward (the fits are about slopes, and
/// small programs mostly sample the intercept).
pub fn any_program() -> impl Strategy<Value = Vec<Op>> {
    any_family()
        .prop_flat_map(|family| {
            (Just(family), any::<u64>()).prop_map(|(family, seed)| build(&family, seed))
        })
        .prop_filter("non-empty program", |p| !p.is_empty())
}

/// A strategy over [`Family`] draws (dimensions included).
///
/// The roster is uniform except [`Family::Escalation`], weighted at one
/// draw in ~137: its programs cost quadratically in their reach (every
/// construction op pays the current size), so a handful per corpus buys
/// the reach without the corpus paying escalated prices everywhere.
pub fn any_family() -> impl Strategy<Value = Family> {
    prop_oneof![
        8 => (2u32..=192, 0u32..=3).prop_map(|(depth, ticks_per_level)| Family::DenseSpine {
            depth,
            ticks_per_level
        }),
        8 => (16u32..=1024, 2u32..=64)
            .prop_map(|(root_ticks, depth)| Family::BigRoot { root_ticks, depth }),
        8 => (16u32..=2048).prop_map(|ticks| Family::HugeLeaf { ticks }),
        8 => (2u32..=32, 1u32..=8)
            .prop_map(|(teeth, magnitude)| Family::CliffComb { teeth, magnitude }),
        8 => (2u32..=96, any::<bool>())
            .prop_map(|(depth, divert)| Family::IdPairLockstep { depth, divert }),
        8 => (2u32..=32, 1u32..=8)
            .prop_map(|(teeth, magnitude)| Family::CombScatter { teeth, magnitude }),
        8 => (2u32..=64, 8u32..=512)
            .prop_map(|(depth, total_ticks)| Family::Harmonic { depth, total_ticks }),
        8 => (8u32..=1024).prop_map(|clocks| Family::ScatterFold { clocks }),
        8 => (2u32..=64, 1u32..=6).prop_map(|(depth, ticks)| Family::NestedFull { depth, ticks }),
        8 => (1u32..=512, 2u32..=64)
            .prop_map(|(tail_ticks, depth)| Family::WideTail { tail_ticks, depth }),
        8 => (2u32..=48).prop_map(|depth| Family::Staircase { depth }),
        8 => (2u32..=24, 1u32..=7, any::<bool>()).prop_map(|(teeth, magnitude, hifloor)| {
            Family::RevealComb {
                teeth,
                magnitude,
                hifloor,
            }
        }),
        8 => (2u32..=48).prop_map(|teeth| Family::PureComb { teeth }),
        8 => (2u32..=32, any::<bool>())
            .prop_map(|(teeth, plateau)| Family::AscendCliff { teeth, plateau }),
        8 => (16u32..=256).prop_map(|ops| Family::Benign { ops }),
        8 => (32u32..=512).prop_map(|ops| Family::Combination { ops }),
        8 => (2u32..=4, 16u32..=128)
            .prop_map(|(universes, ops)| Family::Independent { universes, ops }),
        1 => (256u32..=ESCALATION_MAX_DEPTH).prop_map(|depth| Family::Escalation { depth }),
    ]
}
