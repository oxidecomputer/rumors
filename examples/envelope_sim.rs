//! Uniform-occupancy envelope simulator for the session memory model.
//!
//! Certifies the analytic machinery behind dividing a session memory
//! budget across per-stream receive windows under the model of record
//! (uniform-hash, authenticated-honest-peer): exact binomial occupancy
//! envelopes at 2⁻⁴⁸ per statistic (union ≤ 2⁻⁴⁰ per session), the
//! integer-honest adoptable forms, and the proof obligations tying them
//! together. Concretely it
//!
//!   1. replicates a flat per-scope window solve (a constant scope price
//!      against a geometric declared-capacity ladder) and pins its
//!      reference figures, as the baseline the sharpened envelopes are
//!      compared against;
//!   2. computes the sharpened per-stage envelope (exact binomial
//!      occupancy, Chernoff quantiles) and solves the widest window it
//!      admits;
//!   3. computes the integer-honest envelope and verifies, over a dense
//!      sampled sweep of `(N, depth)`, that every integer bound
//!      dominates its exact-Chernoff counterpart — so the integer
//!      envelope inherits the 2⁻⁴⁰ guarantee;
//!   4. derives `L(N)`, the simultaneously-heavy-stage count that a
//!      per-stream budget division consumes, with its threshold band;
//!   5. under `--full`, validates the analytic formulas against
//!      brute-force path-compressed tries over real uniform keys and a
//!      conditional layer sampler (Monte Carlo; the envelope assertions
//!      are the check, seeds are arbitrary).
//!
//! The shipped window derivation (`src/tree/mirror/streaming/window.rs`)
//! implements the pair-based `A·B` adaptation of the same integer
//! family; this tool certifies the one-corpus `N` forms and the
//! integer-over-exact dominance those adaptations rest on.
//!
//! Usage:
//!   cargo run --release --example envelope_sim              # certify (~seconds)
//!   cargo run --release --example envelope_sim -- --full    # + Monte Carlo tiers
//!   cargo run --release --example envelope_sim -- --fast    # fewer seeds
//!   cargo run --release --example envelope_sim -- --manifest # machine-readable dump
//!
//! Numeric representation: every operand that outgrows `u128` is a
//! power of 256, so slot counts are carried as exponents (`j` in
//! `256^j`) and each crossover is guarded: quantile searches run in
//! `u128` where `256^j` fits and collapse to their deterministic caps
//! where it does not (proved where used). Float paths call the system
//! libm (`powf`, `ln_1p`, `exp_m1`), and the `--manifest` mode dumps
//! every deterministic quantity so an independent implementation —
//! arbitrary-precision, say — can be diffed value-for-value.

use std::cell::RefCell;
use std::collections::HashMap;
use std::env;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

// ---------------------------------------------------------------------
// Model constants. FAN/DEPTH/LISTING_ENTRY_BYTES/STAGES mirror the
// crate (radix-256 tries over 32-byte content addresses, 17-byte
// `(u8, Hash)` listing entries, `Stream::COUNT` = 17 streams);
// DECODE_SLACK_BYTES mirrors `DEFAULT_TARGET_MESSAGE_SIZE`. The rest
// parameterize the reference flat solve and the reply containers.
// ---------------------------------------------------------------------

const FAN: u128 = 256;
const DEPTH: i32 = 32;
const LISTING_ENTRY_BYTES: u128 = 17;
const PARKED_REPLY_SKELETON_BYTES: u128 = FAN * FAN * LISTING_ENTRY_BYTES;
const CHILD_CONTAINER_BYTES: u128 = 32;
const STAGES: u128 = 17;
const DECODE_SLACK_BYTES: u128 = 1_114_624;
const DEFAULT_BUDGET: u128 = 16 * (1 << 30);
const DEFAULT_N: u64 = 1 << 40;

/// The flat solve's per-node price: the unique integer consistent with
/// the reference solve's pinned default window (asserted in
/// [`check_landed_replication`]).
const NODE_BYTES: u128 = 340;

/// One `Query(Vec<(u8, Hash)>)` reaction slot: 24-byte Vec header plus
/// tag/radix, derived.
const C_REACTION_DEFAULT: u128 = 32;
/// A reply's own Vec header plus channel-slot bookkeeping: estimated,
/// generous; the sensitivity section reports how little it matters.
const C_REPLY_DEFAULT: u128 = 64;

/// The synthetic opening the initiator-parking side holds: one reply
/// whose single Query carries the root's listing, ≤ 256 depth-1 entries.
const OPENING_SKELETON_BYTES: u128 =
    C_REPLY_DEFAULT + C_REACTION_DEFAULT + FAN * LISTING_ENTRY_BYTES;

/// Per-(stage, statistic) tail level: 2⁻⁴⁸, unioning to < 2⁻⁴⁰ per
/// session (≤ ~24 active depths × ≤ 9 allocations per stage).
const LOG2_EPS_STAGE: f64 = -48.0;

/// B2's per-parent threshold level in the exact aggregate.
const LOG2_EPS_TYPICAL: f64 = -20.0;

const TABULATED_N: [u64; 5] = [
    256u64.pow(3),
    256u64.pow(4),
    256u64.pow(5),
    10u64.pow(10),
    10u64.pow(12),
];

const STEADY: u128 = DEFAULT_BUDGET - STAGES * DECODE_SLACK_BYTES;

/// The equal-split share: the heaviness unit for `L(N)`.
const SHARE: f64 = STEADY as f64 / STAGES as f64;

/// Stands in for an unbounded window in saturation computations.
const UNBOUNDED_K: u128 = 1 << 100;

/// Reply container sizes, threaded (rather than global) so the
/// sensitivity section can vary them without mutable statics.
#[derive(Clone, Copy)]
struct Containers {
    c_reply: u128,
    c_reaction: u128,
}

const CTR: Containers = Containers {
    c_reply: C_REPLY_DEFAULT,
    c_reaction: C_REACTION_DEFAULT,
};

/// `256^j` when it fits in `u128` (8·j ≤ 120 kept conservative), else
/// `None`, meaning "astronomically larger than any corpus bound".
fn pow256(j: i32) -> Option<u128> {
    if !(0..=15).contains(&j) {
        return None;
    }
    Some(1u128 << (8 * j))
}

// ---------------------------------------------------------------------
// The flat reference charge (pure integer arithmetic).
// ---------------------------------------------------------------------

fn per_scope_flat(node_bytes: u128) -> u128 {
    PARKED_REPLY_SKELETON_BYTES + FAN * (CHILD_CONTAINER_BYTES + node_bytes)
}

/// Flat ladder: `K + Σ_{j≥2} min(K, N ∕ 256^j)`.
fn charged_scopes(scopes: u128, n: u64) -> u128 {
    let mut total = scopes;
    let mut divisor: u128 = FAN * FAN;
    loop {
        let capacity = u128::from(n) / divisor;
        if capacity == 0 {
            break;
        }
        total += capacity.min(scopes);
        divisor = match divisor.checked_mul(FAN) {
            Some(d) => d,
            None => break, // next capacity would be 0 anyway
        };
    }
    total
}

/// Flat solve: the largest `K` with `charged(K) × per_scope ≤ steady`.
fn k_flat(n: u64, budget: u128, node_bytes: u128) -> u128 {
    let per = per_scope_flat(node_bytes);
    let steady = budget.saturating_sub(STAGES * DECODE_SLACK_BYTES);
    let fits = |k: u128| charged_scopes(k, n) * per <= steady;
    let (mut lo, mut hi) = (1u128, steady / per);
    if hi < lo || !fits(lo) {
        return 1;
    }
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        if fits(mid) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo
}

/// Pins the flat replication: the reference figures (default window
/// 4 644 at N = 256⁵; NODE_BYTES = 340 the unique consistent price;
/// the ~3× widening at small declarations) must reproduce exactly.
fn check_landed_replication() {
    let consistent: Vec<u128> = (0..2048)
        .filter(|&nb| k_flat(DEFAULT_N, DEFAULT_BUDGET, nb) == 4_644)
        .collect();
    assert_eq!(consistent, vec![340], "NODE_BYTES back-out failed");
    assert_eq!(k_flat(DEFAULT_N, DEFAULT_BUDGET, NODE_BYTES), 4_644);
    let ratio = k_flat(256u64.pow(3), DEFAULT_BUDGET, NODE_BYTES) as f64
        / k_flat(DEFAULT_N, DEFAULT_BUDGET, NODE_BYTES) as f64;
    assert!(
        (2.8..=3.2).contains(&ratio),
        "small-N widening ratio {ratio}"
    );
}

// ---------------------------------------------------------------------
// Exact occupancy under uniform keys (binomial, no Poissonization).
// For a fixed depth-j prefix the leaf count is Binomial(N, 256⁻ʲ);
// conditional on the count, continuations are iid uniform. Two
// disjoint honest corpora are independent draws, so a slot is jointly
// occupied with probability exactly p_occ².
// ---------------------------------------------------------------------

/// `P(a fixed depth-j slot holds ≥ 1 leaf) = 1 − (1−q)^N`, exact.
fn p_occ(n: u64, j: i32) -> f64 {
    if j == 0 {
        return if n >= 1 { 1.0 } else { 0.0 };
    }
    let q = 256.0f64.powf(-f64::from(j));
    -((n as f64) * (-q).ln_1p()).exp_m1()
}

/// Log of the Chernoff–Hoeffding bound on `P(Binomial(n, p) ≥ a)`:
/// `−n·KL(a∕n ‖ p)`, valid for `a∕n ≥ p` and verbatim for sums of
/// negatively associated indicators (multinomial slot occupancies are
/// NA; joint-occupancy indicators are products of independent NA
/// families, hence NA).
fn binom_tail_log(n: u128, p: f64, a: u128) -> f64 {
    let (nf, af) = (n as f64, a as f64);
    if af <= nf * p {
        return 0.0;
    }
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    let x = af / nf;
    if x >= 1.0 {
        return nf * p.ln();
    }
    let kl = x * (x / p).ln() + (1.0 - x) * ((-x).ln_1p() - (-p).ln_1p());
    -nf * kl
}

thread_local! {
    static QUANTILE_CACHE: RefCell<HashMap<(u128, u64, u64), u128>> =
        RefCell::new(HashMap::new());
}

/// Largest value a Binomial(n, p) exceeds only with probability
/// ≤ 2^log2_eps: (the smallest `a` whose Chernoff tail certifies) − 1.
fn chernoff_quantile(n: u128, p: f64, log2_eps: f64) -> u128 {
    let key = (n, p.to_bits(), log2_eps.to_bits());
    if let Some(v) = QUANTILE_CACHE.with(|c| c.borrow().get(&key).copied()) {
        return v;
    }
    let target = log2_eps * 2.0f64.ln();
    let out = if p >= 1.0 {
        n
    } else {
        let mut lo = (n as f64 * p) as u128; // tail bound is 1 here
        let mut hi = n;
        if binom_tail_log(n, p, hi) > target {
            n // even the deterministic max cannot certify: cap.
        } else {
            while lo + 1 < hi {
                let mid = lo + (hi - lo) / 2;
                if binom_tail_log(n, p, mid) <= target {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            hi.saturating_sub(1)
        }
    };
    QUANTILE_CACHE.with(|c| c.borrow_mut().insert(key, out));
    out
}

/// High-probability bound on occupied depth-j slots of one N-corpus:
/// deterministic caps (slot count, corpus) plus the Chernoff quantile.
///
/// Where `256^j` outgrows `u128` (j ≥ 16) the corpus cap binds: the
/// binomial mean is ≥ N − N²∕2^(8j+1) ≥ N − ½ for N ≤ 2⁶⁰, and no
/// tail level certifies below the mean, so the quantile ≥ N and
/// `min` collapses to the cap — returned directly.
fn occ_hi(n: u64, j: i32) -> u128 {
    if j <= 0 {
        return u128::from(n >= 1);
    }
    let p = p_occ(n, j);
    if p <= 0.0 {
        return 0;
    }
    match pow256(j) {
        Some(slots) => {
            let cap = slots.min(u128::from(n));
            cap.min(chernoff_quantile(slots, p, LOG2_EPS_STAGE))
        }
        None => {
            assert!(n < (1 << 60), "cap-collapse argument needs N < 2^60");
            u128::from(n)
        }
    }
}

/// High-probability bound on jointly occupied depth-j slots of two
/// disjoint N-corpora (per-slot probability exactly p_occ²).
///
/// Past `u128` range the slot count is still an exact power of two in
/// `f64`, the pair mean is sub-unit, and the search boundary sits at
/// small values where every `f64` conversion is exact — so the
/// clamped-domain search returns what the unbounded-integer search
/// would (verified value-for-value in the manifest comparison).
fn joint_hi(n: u64, j: i32) -> u128 {
    if j <= 0 {
        return u128::from(n >= 1);
    }
    let p = p_occ(n, j).powi(2);
    if p <= 0.0 {
        return 0;
    }
    match pow256(j) {
        Some(slots) => slots
            .min(u128::from(n))
            .min(chernoff_quantile(slots, p, LOG2_EPS_STAGE)),
        None => {
            let nf = 2.0f64.powi(8 * j);
            u128::from(n).min(chernoff_quantile_huge(nf, p, LOG2_EPS_STAGE))
        }
    }
}

/// [`chernoff_quantile`] for slot counts beyond `u128`, passed as an
/// exact power-of-two `f64`. The sub-unit mean keeps the boundary at
/// small `a`; mids above 2⁵³ evaluate deep inside the certified region
/// where rounding cannot move the boundary.
fn chernoff_quantile_huge(nf: f64, p: f64, log2_eps: f64) -> u128 {
    let target = log2_eps * 2.0f64.ln();
    let tail = |a: u128| {
        let x = a as f64 / nf;
        if x <= p {
            return 0.0;
        }
        if p <= 0.0 {
            return f64::NEG_INFINITY;
        }
        if x >= 1.0 {
            return nf * p.ln();
        }
        let kl = x * (x / p).ln() + (1.0 - x) * ((-x).ln_1p() - (-p).ln_1p());
        -nf * kl
    };
    let (mut lo, mut hi) = ((nf * p) as u128, 1u128 << 100);
    if tail(hi) > target {
        return hi;
    }
    while lo + 1 < hi {
        let mid = lo + (hi - lo) / 2;
        if tail(mid) <= target {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    hi.saturating_sub(1)
}

/// Per-scope tail level for a max-over-parents union bound at parent
/// depth j: 2⁻⁴⁸ split over the `256^j` candidate prefixes.
fn per_slot_eps(j: i32) -> f64 {
    LOG2_EPS_STAGE - 8.0 * f64::from(j.min(40))
}

/// Quantile of the occupied sub-slot count under one depth-j parent at
/// `2^log2_eps`: the sub-slot route and (for j > 0) the leaves-under
/// cap, both Chernoff, min.
fn occ_quantile(n: u64, j: i32, sub_slots: u128, log2_eps: f64) -> u128 {
    if n == 0 {
        return 0;
    }
    let levels = levels_of(sub_slots);
    let by_slots = chernoff_quantile(sub_slots, p_occ(n, j + levels), log2_eps);
    if j == 0 {
        return sub_slots.min(by_slots);
    }
    let by_leaves = chernoff_quantile(u128::from(n), 256.0f64.powf(-f64::from(j)), log2_eps);
    sub_slots.min(by_slots).min(by_leaves)
}

/// Sub-slot spaces are powers of 256; their level count.
fn levels_of(sub_slots: u128) -> i32 {
    (sub_slots.trailing_zeros() / 8) as i32
}

/// `S(d)`: hp bound on parked replies at the depth-d stage — the
/// queried-listing-entry count at depth d−1: min of the occupied-slot
/// cap and the listed-under-disputed-parents route (joint parents at
/// d−2 × the per-parent children quantile at the union level).
fn stage_pop(n: u64, d: i32) -> u128 {
    if d < 1 {
        return 0;
    }
    if d == 1 {
        return 1; // the opening reply (or the synthetic opening question)
    }
    let listed = joint_hi(n, d - 2) * occ_quantile(n, d - 2, FAN, per_slot_eps(d - 2));
    occ_hi(n, d - 1).min(listed)
}

/// High-probability bound on the summed occupied sub-slot counts under
/// the `min(k, s)` parked depth-j parents. Three routes, min:
/// B1 (max bound), B2 (typical + exceeders), B3 (corpus total,
/// deterministic).
fn aggregate_occ(n: u64, j: i32, sub_slots: u128, k: u128, s: u128) -> u128 {
    let m = k.min(s);
    if m == 0 || n == 0 {
        return 0;
    }
    let levels = levels_of(sub_slots);
    let b1 = m * occ_quantile(n, j, sub_slots, per_slot_eps(j));
    // B2's exceeder count runs over 256^min(j,40) candidate parents; past
    // u128 range its mean alone (slots × 2⁻²⁰) dwarfs B3's corpus cap,
    // so B2 cannot bind and is skipped (saturation keeps `min` honest).
    let b2 = match pow256(j.min(40)) {
        Some(slots) => {
            let q_typ = occ_quantile(n, j, sub_slots, LOG2_EPS_TYPICAL);
            let n_over = chernoff_quantile(slots, 2.0f64.powf(LOG2_EPS_TYPICAL), LOG2_EPS_STAGE);
            m.saturating_mul(q_typ)
                .saturating_add(n_over.saturating_mul(sub_slots))
        }
        None => u128::MAX,
    };
    let b3 = match pow256(j + levels) {
        Some(lev_slots) => u128::from(n).min(lev_slots),
        None => u128::from(n),
    };
    b1.min(b2).min(b3)
}

/// Stream stages by the speaking role: heights descend by two per
/// stream; depth = 32 − height. Initiator replies land at even depths
/// 2..=32, responder at odd depths 1..=31 plus the leaf stage 32.
fn skeleton_depths(initiator: bool) -> Vec<i32> {
    if initiator {
        (2..=DEPTH).step_by(2).collect()
    } else {
        let mut v: Vec<i32> = (1..DEPTH).step_by(2).collect();
        v.push(DEPTH);
        v
    }
}

/// The walk holds in-flight questions/scopes about every level's
/// parents regardless of reply parity.
fn refs_depths() -> Vec<i32> {
    (1..=DEPTH).collect()
}

type AggFn = fn(u64, i32, u128, u128, u128) -> u128;
type PopFn = fn(u64, i32) -> u128;

/// Simultaneous parked-bytes envelope at window `k`, declaration `n`,
/// peer speaking the given role: parked containers + reactions (union
/// corpus) + listing entries per skeleton stage, the synthetic opening
/// when the peer initiates, and question/scope queues at every level.
fn envelope_bytes(
    k: u128,
    n: u64,
    peer_initiator: bool,
    ctr: Containers,
    agg: AggFn,
    pop: PopFn,
) -> u128 {
    let mut total: u128 = 0;
    if peer_initiator {
        total += OPENING_SKELETON_BYTES;
    }
    for d in skeleton_depths(peer_initiator) {
        let s = pop(n, d);
        if s == 0 {
            continue;
        }
        total += k.min(s) * ctr.c_reply;
        // Reactions span the union of both sides' children; disjoint
        // corpora of N each give the occupancy of a 2N corpus.
        total += ctr.c_reaction * agg(2 * n, d - 1, FAN, k, s);
        if d < DEPTH {
            total += LISTING_ENTRY_BYTES * agg(n, d - 1, FAN * FAN, k, s);
        }
    }
    for d in refs_depths() {
        let s = pop(n, d);
        if s == 0 {
            continue;
        }
        total += (CHILD_CONTAINER_BYTES + NODE_BYTES) * agg(n, d - 1, FAN, k, s);
    }
    total
}

/// Widest `K` whose envelope fits the post-slack budget, worst peer
/// role. `capped` means the envelope saturates below the budget at
/// every `K` (the declared corpus cannot bind it); the returned value
/// is then the widest stage population.
fn k_solve(n: u64, budget: u128, ctr: Containers, agg: AggFn, pop: PopFn) -> (u128, bool) {
    let steady = budget.saturating_sub(STAGES * DECODE_SLACK_BYTES);
    let fits = |k: u128| {
        [true, false]
            .iter()
            .all(|&init| envelope_bytes(k, n, init, ctr, agg, pop) <= steady)
    };
    let cap = (1..=DEPTH).map(|d| pop(n, d)).max().unwrap_or(0).max(1);
    if fits(cap) {
        return (cap, true);
    }
    let (mut lo, mut hi) = (1u128, cap);
    if !fits(lo) {
        return (1, false);
    }
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        if fits(mid) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    (lo, false)
}

fn k_sharp(n: u64, budget: u128, ctr: Containers) -> (u128, bool) {
    k_solve(n, budget, ctr, aggregate_occ, stage_pop)
}

// ---------------------------------------------------------------------
// The integer-honest adoptable envelope: the same structure with pure
// integer bounds in place of the Chernoff quantiles. The dominance
// sweep verifies each integer bound is ≥ its exact counterpart across
// the sampled range, so the integer envelope inherits the 2⁻⁴⁰
// guarantee (a min over fewer, individually larger bounds can only be
// larger).
// ---------------------------------------------------------------------

/// Integer upper bound on ln 2 × the union tail bits at parent depth
/// j: ⌈0.7 × (48 + 8·min(j, 40))⌉.
fn t_int(j: i32) -> u128 {
    let bits = 48 + 8 * u128::try_from(j.clamp(0, 40)).unwrap();
    (7 * bits).div_ceil(10)
}

/// Integer quantile from the multiplicative Chernoff tail
/// `P(X ≥ μ + x) ≤ exp(−x²∕(2μ + x))`: `x = ⌊√(2μT)⌋ + T` gives tail
/// ≤ e⁻ᵀ.
fn bernstein(mu_hi: u128, t: u128) -> u128 {
    mu_hi + (2 * mu_hi * t).isqrt() + t
}

/// Integer occupied-slot envelope: both caps are deterministic.
fn occ_int(n: u64, j: i32) -> u128 {
    if j <= 0 {
        return u128::from(n >= 1);
    }
    match pow256(j) {
        Some(slots) => slots.min(u128::from(n)),
        None => u128::from(n),
    }
}

/// Integer quantile for the sub-unit-mean regime with mean
/// `num ∕ 2^den_exp`, or `None` when the mean is not clearly
/// sub-unit. Poisson-type tail `P(X ≥ a) ≤ (eμ)^a`; with
/// `b = den_bits − num_bits ≥ 5`, `a = t∕(b−3) + 2` reaches 2⁻ᵗ, and
/// if `eμ ≤ 2⁻ᵗ` already the quantile is 0.
fn small_mean_quantile(num: u128, den_exp: u32, t: u128) -> Option<u128> {
    // num × 2^(t+2) < 2^den_exp  ⇔  num < 2^(den_exp − t − 2).
    let num_bits = u128::from(128 - num.leading_zeros());
    if u128::from(den_exp) > t + 2 && num_bits <= u128::from(den_exp) - t - 2 {
        return Some(0);
    }
    let den_bits = u128::from(den_exp) + 1;
    if den_bits >= num_bits + 5 {
        let b = den_bits - num_bits;
        return Some(t / (b - 3) + 2);
    }
    None
}

/// Integer jointly-occupied-slot envelope: slot and corpus caps plus a
/// quantile at the pair mean N²∕256^j — Bernstein in the bulk (flat
/// 2⁻⁴⁸ level, t = 34 ≥ 48·ln 2), Poisson-type past the joint
/// frontier.
fn joint_int(n: u64, j: i32) -> u128 {
    if j <= 0 {
        return u128::from(n >= 1);
    }
    let nn = u128::from(n) * u128::from(n);
    let jexp = 8 * u32::try_from(j).unwrap();
    if let Some(small) = small_mean_quantile(nn, jexp, 48) {
        let cap = pow256(j).unwrap_or(u128::MAX).min(u128::from(n));
        return cap.min(small);
    }
    // Non-sub-unit mean ⇒ 8j is within nn's bit length ⇒ 256^j fits.
    let slots = pow256(j).expect("bulk regime keeps 256^j within u128");
    slots.min(u128::from(n)).min(bernstein(nn / slots + 1, 34))
}

/// Integer per-parent quantile, leaves route: occupied sub-slots ≤
/// leaves under the prefix ~ Binomial(N, 256⁻ʲ).
fn q_leaves_int(n: u64, j: i32) -> u128 {
    if j > 0 {
        let t = 48 + 8 * u128::try_from(j.min(40)).unwrap();
        if let Some(small) = small_mean_quantile(u128::from(n), 8 * j as u32, t) {
            return small;
        }
    }
    let mu_hi = if j > 0 {
        // Sub-unit fallback failed ⇒ 8j < 64+5 ⇒ the shift is exact.
        (u128::from(n) >> (8 * j)) + 1
    } else {
        u128::from(n) + 1
    };
    bernstein(mu_hi, t_int(j))
}

/// Integer per-parent quantile, slots route: the integer mean envelope
/// `2·N·sub ∕ (2·256^(j+levels) + N)` (from `1 − e⁻ˣ ≤ 2x∕(2+x)`) plus
/// Bernstein slack at the union level.
fn q_slots_int(n: u64, j: i32, sub_slots: u128) -> u128 {
    let levels = levels_of(sub_slots);
    let mean_hi = match pow256(j + levels) {
        Some(lev_slots) => {
            let num = 2 * u128::from(n) * sub_slots;
            sub_slots.min(num / (2 * lev_slots + u128::from(n)) + 1)
        }
        // 256^(j+levels) ≥ 2^128 dwarfs the numerator (≤ 2^81): mean 0.
        None => 1,
    };
    sub_slots.min(bernstein(mean_hi, t_int(j)))
}

/// Integer per-parent children quantile (fan-slot and leaves routes).
fn c_q_int(n: u64, j: i32) -> u128 {
    FAN.min(q_leaves_int(n, j)).min(q_slots_int(n, j, FAN))
}

/// Integer stage population.
fn stage_pop_int(n: u64, d: i32) -> u128 {
    if d < 1 {
        return 0;
    }
    if d == 1 {
        return 1;
    }
    let listed = joint_int(n, d - 2) * c_q_int(n, d - 2);
    occ_int(n, d - 1).min(listed)
}

fn aggregate_occ_int(n: u64, j: i32, sub_slots: u128, k: u128, s: u128) -> u128 {
    let m = k.min(s);
    if m == 0 || n == 0 {
        return 0;
    }
    let levels = levels_of(sub_slots);
    let q = sub_slots
        .min(q_leaves_int(n, j))
        .min(q_slots_int(n, j, sub_slots));
    let b1 = m.saturating_mul(q);
    let b3 = match pow256(j + levels) {
        Some(lev_slots) => u128::from(n).min(lev_slots),
        None => u128::from(n),
    };
    b1.min(b3)
}

fn k_int(n: u64, budget: u128, ctr: Containers) -> (u128, bool) {
    k_solve(n, budget, ctr, aggregate_occ_int, stage_pop_int)
}

/// The dominance sweep: every integer bound must be ≥ its exact
/// counterpart over the sampled `(N, j)` grid — 13 corpus sizes ×
/// depths 0..=32 (sampled, not exhaustive; the structural argument
/// `e^{−n·KL} ≤ (eμ∕a)^a` closes the gaps between samples).
fn check_integer_dominates() {
    let ns: [u64; 13] = [
        2,
        10,
        100,
        10u64.pow(4),
        10u64.pow(6),
        256u64.pow(3),
        10u64.pow(8),
        256u64.pow(4),
        10u64.pow(10),
        256u64.pow(5),
        10u64.pow(12),
        10u64.pow(13),
        1 << 50,
    ];
    for &n in &ns {
        for j in 0..=DEPTH {
            assert!(occ_int(n, j) >= occ_hi(n, j), "occ dominance at ({n}, {j})");
            assert!(
                joint_int(n, j) >= joint_hi(n, j),
                "joint dominance at ({n}, {j})"
            );
            for sub in [FAN, FAN * FAN] {
                let exact_q = occ_quantile(n, j, sub, per_slot_eps(j));
                let q_i = sub.min(q_leaves_int(n, j)).min(q_slots_int(n, j, sub));
                assert!(q_i >= exact_q, "quantile dominance at ({n}, {j}, {sub})");
            }
        }
        for d in 1..=DEPTH {
            assert!(
                stage_pop_int(n, d) >= stage_pop(n, d),
                "population dominance at ({n}, {d})"
            );
        }
    }
}

// ---------------------------------------------------------------------
// L(N): the simultaneously-heavy-stage count (the per-stream budget
// division's divisor). A_d(N) is a stage's achievable parked bytes at
// unbounded window; L is the clamped fractional heavy count
// max(1, Σ_d min(1, A_d ∕ share)), which makes the operating promise a
// theorem: with per-stream advertisement adv = steady∕L, total parked
// bytes are ≤ Σ_d min(A_d, adv) ≤ adv·L = steady.
// ---------------------------------------------------------------------

/// `A_d(N)`: the depth-d stage's achievable parked skeleton bytes.
fn stage_saturation_bytes(n: u64, d: i32, peer_initiator: bool) -> u128 {
    if peer_initiator && d == 1 {
        return OPENING_SKELETON_BYTES;
    }
    let s = stage_pop(n, d);
    if s == 0 {
        return 0;
    }
    let mut total = s * CTR.c_reply;
    total += CTR.c_reaction * aggregate_occ(2 * n, d - 1, FAN, UNBOUNDED_K, s);
    if d < DEPTH {
        total += LISTING_ENTRY_BYTES * aggregate_occ(n, d - 1, FAN * FAN, UNBOUNDED_K, s);
    }
    total
}

fn role_depths(initiator: bool) -> Vec<i32> {
    let mut v = skeleton_depths(initiator);
    if initiator {
        v.push(1);
    }
    v
}

/// `L(N)`: clamped fractional simultaneously-heavy-stage count, worst
/// peer role.
fn l_of_n(n: u64) -> f64 {
    let mut best = 0.0f64;
    for init in [true, false] {
        let frac: f64 = role_depths(init)
            .iter()
            .map(|&d| (stage_saturation_bytes(n, d, init) as f64 / SHARE).min(1.0))
            .sum();
        best = best.max(frac);
    }
    best.max(1.0)
}

/// Integer heavy-stage count at threshold `theta × share`, worst role.
fn heavy_count(n: u64, theta: f64) -> u128 {
    let mut best = 0u128;
    for init in [true, false] {
        let count = role_depths(init)
            .iter()
            .filter(|&&d| stage_saturation_bytes(n, d, init) as f64 >= theta * SHARE)
            .count() as u128;
        best = best.max(count);
    }
    best
}

// ---------------------------------------------------------------------
// Monte Carlo tiers (--full): brute-force tries over uniform keys and
// the conditional layer sampler. The envelope assertions carry the
// validation, not the exact samples, so the RNG streams are arbitrary.
// ---------------------------------------------------------------------

const BRUTE_MAX_D: i32 = 6;

/// Uniform u64 keys, sorted and deduplicated: statistics below depend
/// only on the leading 8 bytes of the modeled 32-byte addresses, and a
/// u64-level collision (probability ~N²∕2⁶⁵) only merges one prefix.
fn draw_keys(n: usize, rng: &mut SmallRng) -> Vec<u64> {
    let mut keys: Vec<u64> = (0..n).map(|_| rng.r#gen::<u64>()).collect();
    keys.sort_unstable();
    keys.dedup();
    keys
}

fn prefixes(keys: &[u64], j: i32) -> Vec<u64> {
    let shift = 8 * (8 - j as u32);
    let mut v: Vec<u64> = keys.iter().map(|&k| k >> shift).collect();
    v.dedup(); // input sorted ⇒ prefixes sorted
    v
}

/// Grouped counts of `items` (sorted) under their `>> 8` parents.
fn group_counts(items: &[u64]) -> Vec<(u64, u64)> {
    let mut out: Vec<(u64, u64)> = Vec::new();
    for &it in items {
        let parent = it >> 8;
        if let Some((p, c)) = out.last_mut()
            && *p == parent
        {
            *c += 1;
            continue;
        }
        out.push((parent, 1));
    }
    out
}

struct TrieDepth {
    occupied: usize,
    branch: usize,
    children: Vec<u64>,
    entries: Vec<u64>,
}

/// Exact per-depth statistics of one corpus (per-occupied-parent child
/// and grandchild-entry counts; branch = parents with ≥ 2 children).
fn trie_stats(keys: &[u64], max_j: i32) -> HashMap<i32, TrieDepth> {
    let uniq: HashMap<i32, Vec<u64>> = (0..=max_j + 2).map(|j| (j, prefixes(keys, j))).collect();
    let mut out = HashMap::new();
    for j in 1..=max_j {
        let child_counts: Vec<u64> = group_counts(&uniq[&(j + 1)])
            .into_iter()
            .map(|(_, c)| c)
            .collect();
        let gc: Vec<u64> = uniq[&(j + 2)].iter().map(|&x| x >> 16).collect();
        let mut entries: Vec<u64> = Vec::new();
        let mut last = None;
        for &p in &gc {
            match (last, entries.last_mut()) {
                (Some(l), Some(c)) if l == p => *c += 1,
                _ => entries.push(1),
            }
            last = Some(p);
        }
        out.insert(
            j,
            TrieDepth {
                occupied: uniq[&j].len(),
                branch: child_counts.iter().filter(|&&c| c >= 2).count(),
                children: child_counts,
                entries,
            },
        );
    }
    out
}

struct TwoCorpusDepth {
    joint: usize,
    listed: usize,
    branch: usize,
    entries_all: Vec<u64>,
}

/// The population object measured on two disjoint uniform corpora: A
/// parks the replies B speaks; `listed` counts B's depth-j prefixes
/// whose depth-(j−1) parent is jointly occupied (the stage-(j+1)
/// population).
fn two_corpus_stats(n: usize, rng: &mut SmallRng, max_j: i32) -> HashMap<i32, TwoCorpusDepth> {
    let a = draw_keys(n, rng);
    let mut b = draw_keys(n, rng);
    b.retain(|k| a.binary_search(k).is_err());
    let tb = trie_stats(&b, max_j);
    let mut out = HashMap::new();
    for j in 1..=max_j {
        let a_j = prefixes(&a, j);
        let b_j = prefixes(&b, j);
        let joint = b_j.iter().filter(|p| a_j.binary_search(p).is_ok()).count();
        let a_up = prefixes(&a, j - 1);
        let listed_mask: Vec<bool> = b_j
            .iter()
            .map(|&p| a_up.binary_search(&(p >> 8)).is_ok())
            .collect();
        let listed = listed_mask.iter().filter(|&&m| m).count();
        let entries_all: Vec<u64> = tb[&j]
            .entries
            .iter()
            .zip(&listed_mask)
            .filter(|(_, m)| **m)
            .map(|(&e, _)| e)
            .collect();
        out.insert(
            j,
            TwoCorpusDepth {
                joint,
                listed,
                branch: tb[&j].branch,
                entries_all,
            },
        );
    }
    out
}

/// Binomial(n, p) sampler in the two regimes this tier reaches: exact
/// CDF inversion for mean ≤ 30 (the stable ratio recurrence from
/// `P(0) = (1−p)^n`), Gaussian with continuity correction above,
/// clamped to `[0, n]`. The Gaussian tail error (relative O(1∕√np))
/// sits far inside the envelopes' 2⁻⁴⁸-vs-measured slack; every
/// consumer is a mean- or quantile-level statistic under an asserted
/// envelope.
fn binomial_sample(n: u64, p: f64, rng: &mut SmallRng) -> u64 {
    let mean = (n as f64) * p;
    if mean <= 30.0 {
        let mut cdf = ((n as f64) * (-p).ln_1p()).exp(); // P(0)
        let mut pk = cdf;
        let u: f64 = rng.r#gen();
        let mut k = 0u64;
        while u > cdf && k < n {
            pk *= ((n - k) as f64) * p / (((k + 1) as f64) * (1.0 - p));
            cdf += pk;
            k += 1;
            if pk < 1e-320 {
                break; // exhausted double precision; the tail is negligible
            }
        }
        return k;
    }
    let sd = (mean * (1.0 - p)).sqrt();
    // Box-Muller from two uniforms in (0, 1].
    let u1: f64 = 1.0 - rng.r#gen::<f64>();
    let u2: f64 = rng.r#gen();
    let z = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
    (mean + sd * z + 0.5).clamp(0.0, n as f64) as u64
}

/// Per-parent occupancy sampled under a depth-j prefix: leaf count from
/// the binomial marginal, then leaves thrown uniformly into the
/// child (256) and grandchild (256²) slot spaces. Past the
/// coupon-collector bound the occupancy clamps to full (errs toward
/// the envelope by < 2⁻³⁰ per sample).
struct ParentSample {
    children: u64,
    entries: u64,
    occupied: bool,
}

fn sample_parent(n: u64, j: i32, rng: &mut SmallRng) -> ParentSample {
    let q = 256.0f64.powf(-f64::from(j));
    let l: u64 = if j == 0 {
        n
    } else if (n as f64) * q < 1e17 {
        binomial_sample(n, q, rng)
    } else {
        ((n as f64) * q) as u64
    };
    if l == 0 {
        return ParentSample {
            children: 0,
            entries: 0,
            occupied: false,
        };
    }
    let full_g = (4.0 * 65536.0 * 65536f64.ln()) as u64;
    if l >= full_g {
        return ParentSample {
            children: 256,
            entries: 65536,
            occupied: true,
        };
    }
    let mut occupied = [0u64; 1024]; // 65536-slot bitset
    for _ in 0..l {
        let ball: u16 = rng.r#gen();
        occupied[usize::from(ball >> 6)] |= 1u64 << (ball & 63);
    }
    let entries: u64 = occupied.iter().map(|w| u64::from(w.count_ones())).sum();
    let children = occupied
        .chunks(4)
        .filter(|c| c.iter().any(|&w| w != 0))
        .count() as u64;
    ParentSample {
        children,
        entries,
        occupied: true,
    }
}

/// Measured per-parent (children, entries) means conditional on the
/// parent being occupied; the analytic 1 + O(λ) limit below λ = 0.01.
fn cond_child_mean(n: u64, j: i32, rng: &mut SmallRng, m: usize) -> (f64, f64) {
    let lam = if j > 0 {
        (n as f64) * 256.0f64.powf(-f64::from(j))
    } else {
        n as f64
    };
    if lam < 0.01 {
        return (1.0, 1.0);
    }
    let m = if lam > 4096.0 {
        (4e6 / lam).max(200.0).min(m as f64) as usize
    } else {
        m
    };
    let (mut cs, mut es, mut cnt) = (0.0f64, 0.0f64, 0u64);
    for _ in 0..m {
        let s = sample_parent(n, j, rng);
        if s.occupied {
            cs += s.children as f64;
            es += s.entries as f64;
            cnt += 1;
        }
    }
    if cnt == 0 {
        return (1.0, 1.0);
    }
    (cs / cnt as f64, es / cnt as f64)
}

/// Mean-level measured achievable bytes of stage d (sampled conditional
/// statistics × exact slot marginals, capped by measured-mean corpus
/// totals).
fn measured_stage_bytes(n: u64, d: i32, rng: &mut SmallRng, m: usize) -> f64 {
    if d < 1 {
        return 0.0;
    }
    let j = d - 1;
    let s_meas = if d == 1 {
        1.0
    } else {
        let (c_up, _) = cond_child_mean(n, j - 1, rng, m);
        let p_up = p_occ(n, j - 1);
        let s = 256.0f64.powf(f64::from(j - 1)) * p_up * p_up * c_up;
        s.min(256.0f64.powf(f64::from(j)) * p_occ(n, j))
            .min(n as f64)
    };
    let (r_cond_u, _) = cond_child_mean(2 * n, j, rng, m);
    let (_, mut g_cond) = cond_child_mean(n, j, rng, m);
    if d >= DEPTH {
        g_cond = 0.0;
    }
    let (c_reply, c_reaction) = (CTR.c_reply as f64, CTR.c_reaction as f64);
    let b1 = s_meas * (c_reply + c_reaction * r_cond_u + LISTING_ENTRY_BYTES as f64 * g_cond);
    let listing = if d < DEPTH {
        (n as f64).min(256.0f64.powf(f64::from(d + 1)) * p_occ(n, d + 1))
    } else {
        0.0
    };
    let b3 = s_meas * c_reply
        + c_reaction * ((2 * n) as f64).min(256.0f64.powf(f64::from(d)) * p_occ(2 * n, d))
        + LISTING_ENTRY_BYTES as f64 * listing;
    b1.min(b3)
}

// ---------------------------------------------------------------------
// Reporting.
// ---------------------------------------------------------------------

fn fmt_bytes(b: f64) -> String {
    for (unit, div) in [
        ("GiB", (1u64 << 30) as f64),
        ("MiB", (1u64 << 20) as f64),
        ("KiB", 1024.0),
    ] {
        if b >= div {
            return format!("{:.2} {unit}", b / div);
        }
    }
    format!("{b:.0} B")
}

fn fmt_n(n: u64) -> String {
    for (name, val) in [
        ("256^2", 256u64.pow(2)),
        ("256^3", 256u64.pow(3)),
        ("256^4", 256u64.pow(4)),
        ("256^5", 256u64.pow(5)),
    ] {
        if n == val {
            return name.to_string();
        }
    }
    format!("{:.0e}", n as f64)
}

fn section(title: &str) {
    println!("\n{}\n{title}\n{}", "=".repeat(74), "=".repeat(74));
}

/// Machine-readable dump of every deterministic quantity, for
/// value-for-value comparison against an independent implementation.
fn manifest() {
    let sweep_ns: [u64; 13] = [
        2,
        10,
        100,
        10u64.pow(4),
        10u64.pow(6),
        256u64.pow(3),
        10u64.pow(8),
        256u64.pow(4),
        10u64.pow(10),
        256u64.pow(5),
        10u64.pow(12),
        10u64.pow(13),
        1 << 50,
    ];
    for &n in &sweep_ns {
        println!("flat,{n},{}", k_flat(n, DEFAULT_BUDGET, NODE_BYTES));
        for j in 0..=DEPTH {
            println!("occ_hi,{n},{j},{}", occ_hi(n, j));
            println!("joint_hi,{n},{j},{}", joint_hi(n, j));
            println!("occ_int,{n},{j},{}", occ_int(n, j));
            println!("joint_int,{n},{j},{}", joint_int(n, j));
            println!("cq_int,{n},{j},{}", c_q_int(n, j));
            for sub in [FAN, FAN * FAN] {
                println!(
                    "occq,{n},{j},{sub},{}",
                    occ_quantile(n, j, sub, per_slot_eps(j))
                );
                let qi = sub.min(q_leaves_int(n, j)).min(q_slots_int(n, j, sub));
                println!("qint,{n},{j},{sub},{qi}");
            }
        }
        for d in 1..=DEPTH {
            println!("pop,{n},{d},{}", stage_pop(n, d));
            println!("pop_int,{n},{d},{}", stage_pop_int(n, d));
        }
    }
    for &n in &TABULATED_N {
        let (ks, cs) = k_sharp(n, DEFAULT_BUDGET, CTR);
        let (ki, ci) = k_int(n, DEFAULT_BUDGET, CTR);
        println!("ksharp,{n},{ks},{}", u8::from(cs));
        println!("kint,{n},{ki},{}", u8::from(ci));
        for init in [true, false] {
            let role = if init { "initiator" } else { "responder" };
            println!(
                "env,{n},{role},{}",
                envelope_bytes(ks, n, init, CTR, aggregate_occ, stage_pop)
            );
            for d in 1..=DEPTH {
                println!("sat,{n},{d},{role},{}", stage_saturation_bytes(n, d, init));
            }
        }
        println!("l,{n},{:.12e}", l_of_n(n));
        for theta in [0.25f64, 0.5, 1.0, 2.0, 4.0] {
            println!("heavy,{n},{theta:?},{}", heavy_count(n, theta));
        }
    }
}

fn certification() {
    check_landed_replication();
    check_integer_dominates();
    println!("flat-solve replication: OK (K_flat(256^5) = 4644; NODE_BYTES = 340 unique)");
    println!("integer envelope dominates exact Chernoff quantiles: OK (sweep)");

    section("K_sharp vs K_flat at the default 16 GiB budget");
    println!(
        "{:>8} {:>8} {:>10} {:>10} {:>10} {:>9}",
        "N", "K_flat", "K_sharp", "K_int", "sharp/flat", "int/flat"
    );
    for &n in &TABULATED_N {
        let kf = k_flat(n, DEFAULT_BUDGET, NODE_BYTES);
        let (ks, cs) = k_sharp(n, DEFAULT_BUDGET, CTR);
        let (ki, ci) = k_int(n, DEFAULT_BUDGET, CTR);
        let star = |k: u128, c: bool| if c { format!("{k}*") } else { format!("{k}") };
        let ratio = |k: u128, c: bool| {
            if c {
                "inf".to_string()
            } else {
                format!("{:.2}", k as f64 / kf as f64)
            }
        };
        println!(
            "{:>8} {kf:>8} {:>10} {:>10} {:>10} {:>9}",
            fmt_n(n),
            star(ks, cs),
            star(ki, ci),
            ratio(ks, cs),
            ratio(ki, ci)
        );
    }
    println!("  * corpus-capped: the envelope saturates below the budget at every");
    println!("    K; the printed value is the widest stage population.");
    for &n in &TABULATED_N {
        let (ks, _) = k_sharp(n, DEFAULT_BUDGET, CTR);
        let worst = [true, false]
            .iter()
            .map(|&i| envelope_bytes(ks, n, i, CTR, aggregate_occ, stage_pop))
            .max()
            .unwrap();
        let sat = [true, false]
            .iter()
            .map(|&i| envelope_bytes(UNBOUNDED_K, n, i, CTR, aggregate_occ, stage_pop))
            .max()
            .unwrap();
        println!(
            "      N = {:>7}: envelope at K_sharp {}, saturation {} (post-slack budget {})",
            fmt_n(n),
            fmt_bytes(worst as f64),
            fmt_bytes(sat as f64),
            fmt_bytes(STEADY as f64)
        );
    }

    section("L(N): simultaneously-heavy-stage count (per-stream divisor)");
    println!("share = steady/17 = {}", fmt_bytes(SHARE));
    let thetas = [0.25f64, 0.5, 1.0, 2.0, 4.0];
    println!(
        "{:>8} {:>6} {:>6} {}   adv = steady/L   ceiling = 17 x adv",
        "N",
        "L(N)",
        "17/L",
        thetas.map(|t| format!("H(x{t})")).join(" ")
    );
    for &n in &TABULATED_N {
        let l = l_of_n(n);
        let adv = STEADY as f64 / l;
        let hs: Vec<String> = thetas
            .iter()
            .map(|&t| format!("{:>6}", heavy_count(n, t)))
            .collect();
        println!(
            "{:>8} {l:>6.2} {:>6.2} {}   {:>12}   {:>12}",
            fmt_n(n),
            17.0 / l,
            hs.join(" "),
            fmt_bytes(adv),
            fmt_bytes(17.0 * adv)
        );
    }
    println!("  H(x theta): integer heavy count at threshold theta x share, worst");
    println!("  role; a 16x threshold band moves H by at most ~1 (membership");
    println!("  decays ~256x per stage past the joint frontier).");

    section("Smoothness and monotonicity of K_sharp(N) and L(N)");
    let mut grid: Vec<u64> = Vec::new();
    for kk in 2..=6u32 {
        for m in -2i64..=2 {
            grid.push((256u64.pow(kk) as i64 + m) as u64);
        }
    }
    let mut e = 6.0f64;
    while e <= 13.01 {
        grid.push(10f64.powf(e) as u64);
        e += 0.25;
    }
    grid.sort_unstable();
    grid.dedup();
    grid.retain(|&g| g >= 2);
    let vals: Vec<f64> = grid
        .iter()
        .map(|&n| {
            let (k, capped) = k_sharp(n, DEFAULT_BUDGET, CTR);
            if capped { f64::INFINITY } else { k as f64 }
        })
        .collect();
    let lvals: Vec<f64> = grid.iter().map(|&n| l_of_n(n)).collect();
    let monotone = vals.windows(2).all(|w| w[0] >= w[1]);
    let l_monotone = lvals.windows(2).all(|w| w[0] <= w[1] + 1e-12);
    let mut worst_step = 0.0f64;
    let mut worst_l_step = 0.0f64;
    for i in 1..grid.len() {
        if grid[i] - grid[i - 1] <= 4 {
            if vals[i - 1].is_finite() && vals[i].is_finite() && vals[i - 1] > 0.0 {
                worst_step = worst_step.max((vals[i] - vals[i - 1]).abs() / vals[i - 1]);
            }
            worst_l_step =
                worst_l_step.max((lvals[i] - lvals[i - 1]).abs() / lvals[i - 1].max(1.0));
        }
    }
    println!(
        "  K_sharp monotone nonincreasing over {}-point grid (capped = +inf): {monotone}",
        grid.len()
    );
    println!("  L(N) monotone nondecreasing over the same grid: {l_monotone}");
    println!("  worst relative K step across adjacent declarations: {worst_step:.2e}");
    println!("  worst relative L step across the same crossings: {worst_l_step:.2e}");
    let finite_from = grid
        .iter()
        .zip(&vals)
        .find(|(_, v)| v.is_finite())
        .map(|(n, _)| *n);
    println!("  budget first binds at declared N = {finite_from:?} on this grid");

    section("Sensitivity: container constants (C_REPLY, C_REACTION)");
    let quad = Containers {
        c_reply: 256,
        c_reaction: 64,
    };
    for &n in &TABULATED_N {
        let (kb, cb) = k_sharp(n, DEFAULT_BUDGET, CTR);
        let (kq, cq) = k_sharp(n, DEFAULT_BUDGET, quad);
        if cb || cq {
            if cb && cq {
                println!("  N = {:>7}: corpus-capped in both settings", fmt_n(n));
            } else {
                println!("  N = {:>7}: capped state changed -- inspect", fmt_n(n));
            }
            continue;
        }
        let delta = 100.0 * (kb as f64 - kq as f64) / kb as f64;
        println!(
            "  N = {:>7}: K_sharp {kb:>7} -> {kq:>7} at 4x/2x containers ({delta:+.1}%)",
            fmt_n(n)
        );
    }
}

fn monte_carlo(fast: bool) {
    let seeds: Vec<u64> = (0..if fast { 20 } else { 100 }).collect();
    let n_brute: usize = 100_000;

    section(&format!(
        "Brute-force trie validation (N = {n_brute}, {} seeds)",
        seeds.len()
    ));
    #[allow(clippy::type_complexity)] // (occupied, children, entries) per depth
    let mut per_depth: HashMap<i32, (Vec<usize>, Vec<u64>, Vec<u64>)> = HashMap::new();
    for &seed in &seeds {
        let mut rng = SmallRng::seed_from_u64(seed);
        let keys = draw_keys(n_brute, &mut rng);
        let st = trie_stats(&keys, BRUTE_MAX_D);
        for j in 1..=BRUTE_MAX_D {
            let slot = per_depth.entry(j).or_default();
            slot.0.push(st[&j].occupied);
            slot.1.extend(&st[&j].children);
            slot.2.extend(&st[&j].entries);
        }
    }
    println!(
        "{:>2} {:>13} {:>11} {:>8} {:>13} {:>15} {:>10} {:>6}",
        "j",
        "E[occ_j] pred",
        "occ_j meas",
        "occ_hi",
        "c mean p/m",
        "g mean p/m",
        "g max meas",
        "g_hi"
    );
    let n = n_brute as u64;
    for j in 1..=BRUTE_MAX_D {
        let (occ, ch, en) = &per_depth[&j];
        let pred_o = 256.0f64.powf(f64::from(j)) * p_occ(n, j);
        let meas_o = occ.iter().sum::<usize>() as f64 / occ.len() as f64;
        let max_o = *occ.iter().max().unwrap() as u128;
        let pred_c = 256.0 * p_occ(n, j + 1);
        let pred_g = 65536.0 * p_occ(n, j + 2);
        let mean = |v: &Vec<u64>| v.iter().sum::<u64>() as f64 / v.len() as f64;
        let g_hi_v = occ_quantile(n, j, FAN * FAN, per_slot_eps(j));
        let occ_hi_v = occ_hi(n, j);
        assert!(max_o <= occ_hi_v, "occ_hi violated at j={j}");
        assert!(
            u128::from(*en.iter().max().unwrap()) <= g_hi_v,
            "g_hi violated at j={j}"
        );
        println!(
            "{j:>2} {pred_o:>13.1} {meas_o:>11.1} {occ_hi_v:>8} {:>6.2}/{:<6.2} {:>7.2}/{:<7.2} {:>10} {g_hi_v:>6}",
            pred_c,
            mean(ch),
            pred_g,
            mean(en),
            en.iter().max().unwrap()
        );
    }
    println!("  (pred c/g are unconditional occupancy means; measured are conditional");
    println!("   on parent occupancy -- the quantile, not the mean, is the bound.)");

    section(&format!(
        "Two-corpus (disjoint, N = {n_brute} each): the population object"
    ));
    #[allow(clippy::type_complexity)] // (joint, listed, branch, entries) per depth
    let mut agg2: HashMap<i32, (Vec<usize>, Vec<usize>, Vec<usize>, Vec<u64>)> = HashMap::new();
    for &seed in &seeds {
        let mut rng = SmallRng::seed_from_u64(10_000 + seed);
        let tc = two_corpus_stats(n_brute, &mut rng, BRUTE_MAX_D);
        for j in 1..=BRUTE_MAX_D {
            let slot = agg2.entry(j).or_default();
            slot.0.push(tc[&j].joint);
            slot.1.push(tc[&j].listed);
            slot.2.push(tc[&j].branch);
            slot.3.extend(&tc[&j].entries_all);
        }
    }
    println!(
        "{:>2} {:>10} {:>10} {:>11} {:>10} {:>8} {:>13} {:>8}",
        "j",
        "joint pred",
        "joint meas",
        "listed meas",
        "S(j+1)",
        "branch",
        "listed/branch",
        "E e_all"
    );
    for j in 1..=BRUTE_MAX_D {
        let (joint, listed, branch, e_all) = &agg2[&j];
        let pred_joint = 256.0f64.powf(f64::from(j)) * p_occ(n, j).powi(2);
        let meanu = |v: &Vec<usize>| v.iter().sum::<usize>() as f64 / v.len() as f64;
        let max_listed = *listed.iter().max().unwrap() as u128;
        let s_env = stage_pop(n, j + 1);
        assert!(max_listed <= s_env, "population envelope violated at j={j}");
        let e_mean = if e_all.is_empty() {
            0.0
        } else {
            e_all.iter().sum::<u64>() as f64 / e_all.len() as f64
        };
        let br = meanu(branch);
        let ratio = if br > 0.0 {
            meanu(listed) / br
        } else {
            f64::INFINITY
        };
        println!(
            "{j:>2} {pred_joint:>10.1} {:>10.1} {:>11.1} {s_env:>10} {br:>8.1} {ratio:>13.1} {e_mean:>8.2}",
            meanu(joint),
            meanu(listed)
        );
    }
    println!("  listed = stage-(j+1) population; S(j+1) is its 2^-48 envelope");
    println!("  (asserted, all seeds); listed/branch contrasts the corrected");
    println!("  population object against branch-node counts.");

    section("Predicted vs measured per stage at the tabulated N (sampler)");
    let m = if fast { 200 } else { 1_000 };
    for &n in &TABULATED_N {
        println!(
            "\nN = {} ({} seeds x {m} parent samples/depth):",
            fmt_n(n),
            seeds.len()
        );
        println!(
            "{:>2} {:>13} {:>13} {:>8} {:>8} {:>12} {:>7} {:>6}",
            "d", "S env", "S meas (mean)", "g pred", "g meas", "g q99.9 meas", "g_hi", "flag"
        );
        let depths: Vec<i32> = (1..13).filter(|&d| stage_pop(n, d) > 0).collect();
        for d in depths {
            let j = d - 1;
            let lam = if j > 0 {
                (n as f64) * 256.0f64.powf(-f64::from(j))
            } else {
                n as f64
            };
            let mut en_all: Vec<u64> = Vec::new();
            if lam >= 0.01 {
                let mean_l = lam.max(1.0);
                let m_eff = if mean_l > 4096.0 {
                    (2e6 / mean_l).max(200.0).min(m as f64) as usize
                } else {
                    m
                };
                for &seed in &seeds {
                    let mut rng = SmallRng::seed_from_u64(seed * 1_000 + d as u64);
                    for _ in 0..m_eff {
                        let s = sample_parent(n, j, &mut rng);
                        if s.occupied {
                            en_all.push(s.entries);
                        }
                    }
                }
            } else {
                en_all.push(1);
            }
            let mut rng = SmallRng::seed_from_u64(777 * d as u64 + 1);
            let s_meas = if d == 1 {
                1.0
            } else {
                let (c_up, _) =
                    cond_child_mean(n, j - 1, &mut rng, if fast { 2_000 } else { 20_000 });
                let s = 256.0f64.powf(f64::from(j - 1)) * p_occ(n, j - 1).powi(2) * c_up;
                s.min(256.0f64.powf(f64::from(j)) * p_occ(n, j))
                    .min(n as f64)
            };
            let g_pred = 65536.0 * p_occ(n, d + 1);
            let g_hi_v = occ_quantile(n, j, FAN * FAN, per_slot_eps(j));
            en_all.sort_unstable();
            let q999 = if en_all.len() > 100 {
                en_all[((en_all.len() as f64) * 0.999) as usize] as f64
            } else {
                *en_all.last().unwrap() as f64
            };
            let g_mean = en_all.iter().sum::<u64>() as f64 / en_all.len() as f64;
            let flag = if q999 > g_hi_v as f64 {
                "VIOL"
            } else if q999 > 0.0 && (g_hi_v as f64) / q999.max(1.0) > 2.0 {
                ">2x"
            } else {
                ""
            };
            let s_env = stage_pop(n, d);
            assert!(
                s_meas <= s_env as f64 * 1.001,
                "population mean exceeds envelope at d={d}"
            );
            println!(
                "{d:>2} {s_env:>13} {s_meas:>13.3e} {g_pred:>8.1} {g_mean:>8.1} {q999:>12.1} {g_hi_v:>7} {flag:>6}"
            );
        }
        println!("  flags: VIOL = measured 99.9% exceeds the 2^-48 envelope (must");
        println!("  never happen); >2x = envelope over 2x the measured 99.9%");
        println!("  (expected at thin stages: it holds at 2^-48, not 10^-3).");
    }

    section("Measured simultaneous heavy-stage counts vs L(N)");
    let m_l = if fast { 2_000 } else { 20_000 };
    let l_seeds = &seeds[..seeds.len().min(20)];
    println!(
        "{:>8} {:>6} {:>6} {:>11} {:>9}",
        "N", "L(N)", "H(x1)", "H_meas mode", "min..max"
    );
    for &n in &TABULATED_N {
        let mut counts: Vec<usize> = Vec::new();
        for &seed in l_seeds {
            let mut rng = SmallRng::seed_from_u64(50_000 + seed);
            let mut best = 0usize;
            for init in [true, false] {
                let c = role_depths(init)
                    .iter()
                    .filter(|&&d| {
                        let a = if d == 1 && init {
                            OPENING_SKELETON_BYTES as f64
                        } else if stage_pop(n, d) == 0 {
                            0.0
                        } else {
                            measured_stage_bytes(n, d, &mut rng, m_l)
                        };
                        a >= SHARE
                    })
                    .count();
                best = best.max(c);
            }
            counts.push(best);
        }
        let mode = {
            let mut freq: HashMap<usize, usize> = HashMap::new();
            for &c in &counts {
                *freq.entry(c).or_default() += 1;
            }
            *freq.iter().max_by_key(|(_, f)| **f).unwrap().0
        };
        println!(
            "{:>8} {:>6.2} {:>6} {mode:>11} {:>4}..{}",
            fmt_n(n),
            l_of_n(n),
            heavy_count(n, 1.0),
            counts.iter().min().unwrap(),
            counts.iter().max().unwrap()
        );
    }
    println!("  H_meas: per-seed heavy count from sampled conditional statistics");
    println!("  at mean level (worst role); L(N) upper-bounds both by construction.");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let flag = |f: &str| args.iter().any(|a| a == f);
    if flag("--manifest") {
        manifest();
        return;
    }
    certification();
    if flag("--full") {
        monte_carlo(flag("--fast"));
    } else {
        println!("\n(analytic certification only; run with --full for the Monte Carlo tiers)");
    }
    println!("\ndone.");
}
