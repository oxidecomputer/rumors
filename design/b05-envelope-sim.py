#!/usr/bin/env python3
"""B0.5 uniformity-envelope spike: analytic bounds + simulation.

CONTEXT ON THIS BRANCH (2026-07-22): analysis artifact from the declined
single-socket campaign, kept for its transport-independent content --
the uniform-occupancy population/reply-size analysis and the L(N)
simultaneity divisor (the basis for a future division of a session
memory budget across the seventeen per-stream transport receive
windows). The "landed window solve" it replicates (charged_scopes /
from_bytes, K = 4,644) is the campaign branch's window.rs, not this
branch's: here the window is node-denominated (max_in_flight_nodes).
See the context header in design/b05-uniformity-envelope.md.

Companion to design/b05-uniformity-envelope.md (the spike note). This
script is the source of every number in that note's tables: it

  1. replicates the landed window solve (window.rs: charged_scopes /
     from_bytes) and pins K_flat against the module tests' figure;
  2. computes the sharpened per-stage envelope under the uniform-hash
     model (exact binomial occupancy + Chernoff quantiles at 2^-48 per
     statistic, union <= 2^-40 per session) and solves K_sharp;
  3. computes the integer-honest adoptable envelope and solves K_int;
  4. derives L(N), the simultaneously-heavy-stage count that B0.7's
     budget division consumes, and its threshold-insensitivity band;
  5. validates the analytic occupancy formulas against brute-force
     path-compressed tries built from real uniform 32-byte keys
     (N = 10^5, 100 seeds), including a two-corpus measurement of the
     stage-population object (queried listing entries);
  6. validates the large-N conditional sampler against the brute force,
     then measures per-stage statistics at the tabulated N values
     (100 seeds) and compares predicted vs measured, including measured
     simultaneous heavy-stage counts vs L(N).

Model of record (design/byte-window-plan.md section 0): uniform-hash,
authenticated-honest-peer. Keys are 32-byte content addresses (one radix
byte per level, depth 32); listing entries are (u8, 16-byte Merkle hash)
pairs, 17 bytes each. No pricing argument rests on adversary economics.

Stage anatomy (remote/adapter/scope.rs, remote/adapter/decode.rs,
remote/proxy/state.rs): a reply on the stream labeled depth d has its
scope parent at depth d-1, reactions at depth d, and listing entries at
depth d+1. Stage population (materialized/work/answer.rs): the answering
walk asks one question for EVERY peer-listed disputed-or-absent entry --
Query(empty) supplies included -- so the population of stage d is the
queried-listing-entry count at depth d-1, NOT the branch count at d.

Determinism: all sampling uses numpy default_rng with the recorded seeds
below. Running `python3 design/b05-envelope-sim.py` reproduces the
note's tables byte-for-byte modulo float printing.

Usage (needs numpy; `uv run --with numpy design/b05-envelope-sim.py`
works from a bare checkout):
  python3 design/b05-envelope-sim.py           # full run (~10 min)
  python3 design/b05-envelope-sim.py --fast    # ~1 min, fewer seeds
"""

import functools
import math
import sys

import numpy as np

# --------------------------------------------------------------------------
# Constants mirrored from src/tree/mirror/streaming/window.rs (verified
# against the module's pinned tests, see check_landed_replication).
# --------------------------------------------------------------------------

FAN = 256
DEPTH = 32  # 32-byte content addresses: one radix byte per level.
LISTING_ENTRY_BYTES = 17  # size_of::<(u8, Hash)>(), Hash = [u8; 16], align 1.
PARKED_REPLY_SKELETON_BYTES = FAN * FAN * LISTING_ENTRY_BYTES  # 1_114_112
CHILD_CONTAINER_BYTES = 32  # 2 * size_of::<(u8, *const ())>() on 64-bit.
STAGES = 17  # Stream::COUNT (remote/codec/signal.rs).
DECODE_SLACK_BYTES = 1_114_624  # DEFAULT_TARGET_MESSAGE_SIZE.
DEFAULT_BUDGET = 16 * (1 << 30)  # 16 GiB.
DEFAULT_N = 1 << 40  # 256^5, DEFAULT_EXPECTED_MAX_MESSAGES.

# NODE_APPROX_MAX_BYTES is private to the crate; its value is backed out
# from window.rs's pinned default K = 4_644 (module test
# default_budget_derives_the_documented_window). check_landed_replication
# asserts 340 is the unique integer consistent with that pin.
NODE_BYTES = 340

# Decoded-reply container terms the landed skeleton omits (they are
# <1% at fat stages but dominate thin-stage prices, so the sharpened
# form must carry them):
#   C_REACTION [derived]: one Reaction enum slot in the reply's Vec --
#     Query(Vec<(u8, Hash)>) is a 24-byte Vec header + tag/radix, 32 B.
#   C_REPLY [estimated, generous]: the reply's own Vec header plus its
#     channel-slot bookkeeping. Sensitivity is reported in the output.
C_REACTION = 32
C_REPLY = 64

# The synthetic opening the initiator-parking side holds (decode.rs
# opening_reply): one reply whose single Query carries the root's
# listing, <= 256 depth-1 entries.
OPENING_SKELETON_BYTES = C_REPLY + C_REACTION + FAN * LISTING_ENTRY_BYTES

# Concentration budget: per-session failure probability 2^-40, split
# 2^-48 per (stage, statistic). Per active stage the envelope consumes
# at most 9 allocations (the occupied-slot, joint-slot, and per-parent
# children quantiles behind S, plus a B1 quantile and a B2 exceeder
# count for each of the three occupancy aggregates; B2's 2^-20
# threshold is a quantile property, not an event), and a stage is
# active (S > 0) only while the joint-occupancy mean N^2/256^(d-2)
# reaches 2^-48 -- at most ~24 depths for any N <= 2^64 -- so the
# union stays under 24 x 9 x 2^-48 < 2^-40.
LOG2_EPS_STAGE = -48

TABULATED_N = [256**3, 256**4, 256**5, 10**10, 10**12]

STEADY = DEFAULT_BUDGET - STAGES * DECODE_SLACK_BYTES

# B0.7's equal-split share: the heaviness unit for L(N).
SHARE = STEADY / STAGES


# --------------------------------------------------------------------------
# The landed charge, replicated exactly (integer arithmetic).
# --------------------------------------------------------------------------

def per_scope_flat(node_bytes: int = NODE_BYTES) -> int:
    return PARKED_REPLY_SKELETON_BYTES + FAN * (CHILD_CONTAINER_BYTES + node_bytes)


def charged_scopes(scopes: int, n: int) -> int:
    """window.rs charged_scopes: K + sum_{j>=2} min(K, N // 256^j)."""
    total = scopes
    divisor = FAN * FAN
    while True:
        capacity = n // divisor
        if capacity == 0:
            break
        total += min(capacity, scopes)
        divisor *= FAN
    return total


def k_flat(n: int, budget: int = DEFAULT_BUDGET, node_bytes: int = NODE_BYTES) -> int:
    """window.rs from_bytes: max K with charged(K) * per_scope <= steady."""
    per = per_scope_flat(node_bytes)
    steady = max(0, budget - STAGES * DECODE_SLACK_BYTES)

    def fits(k: int) -> bool:
        return charged_scopes(k, n) * per <= steady

    lo, hi = 1, steady // per
    if hi < lo or not fits(lo):
        return 1
    while lo < hi:
        mid = lo + (hi - lo + 1) // 2
        if fits(mid):
            lo = mid
        else:
            hi = mid - 1
    return lo


def check_landed_replication() -> None:
    """Pin the replication against window.rs's module tests, and show
    NODE_BYTES = 340 is the unique value consistent with the pinned
    default K = 4_644."""
    consistent = [nb for nb in range(0, 2048) if k_flat(DEFAULT_N, node_bytes=nb) == 4_644]
    assert consistent == [340], f"NODE_BYTES back-out failed: {consistent}"
    assert k_flat(DEFAULT_N) == 4_644
    # small_declarations_approach_the_single_boundary_form: ~3x at 256^3.
    assert 2.8 <= k_flat(256**3) / k_flat(DEFAULT_N) <= 3.2


# --------------------------------------------------------------------------
# Exact occupancy formulas under uniform keys (binomial, no Poissonization).
#
# For a fixed depth-j prefix, the number of corpus leaves beneath it is
# L ~ Binomial(N, q) with q = 256^-j (keys are uniform and independent).
# Conditional on L, the leaves' continuation bytes are iid uniform --
# exact, which is what makes the layer-by-layer sampler exact. Two
# disjoint honest corpora of N each are independent draws, so a slot is
# jointly occupied with probability exactly p_occ^2.
# --------------------------------------------------------------------------

def p_occ(n: int, j: int) -> float:
    """P(a fixed depth-j slot holds >= 1 leaf) = 1 - (1-q)^N, exact."""
    if j == 0:
        return 1.0 if n >= 1 else 0.0
    q = 256.0 ** -j
    return -math.expm1(n * math.log1p(-q))


def p_node(n: int, j: int) -> float:
    """P(a branch node exists at a fixed depth-j prefix), exact.

    No longer an envelope input (the population object is occupied and
    jointly occupied slots, not branch nodes); kept for the measured
    population-vs-branch-count contrast the note reports. Derivation:
    P(split | L) = 1 - 256^(1-L) for L >= 2 summed against the
    Binomial(N, q) pmf collapses to
    1 + 255 (1-q)^N - 256 (1 - 255q/256)^N, with the (255/512) lam^2
    series below lam < 1e-4 where the closed form cancels.
    """
    q = 256.0 ** -j
    lam = n * q
    if lam < 1e-4:
        return (255.0 / 512.0) * lam * lam * max(0.0, 1.0 - lam)
    return 1.0 + 255.0 * math.exp(n * math.log1p(-q)) \
        - 256.0 * math.exp(n * math.log1p(-q * 255.0 / 256.0))


def binom_tail_log(n: int, p: float, a: int) -> float:
    """log of the Chernoff-Hoeffding bound on P(Binomial(n, p) >= a):
    -n * KL(a/n || p), valid for a/n >= p. Applies verbatim to sums of
    negatively associated indicators (Dubhashi-Ranjan 1998): multinomial
    slot occupancies are NA, and joint-occupancy indicators are products
    of two independent NA families (increasing functions of disjoint
    coordinate blocks of the concatenated NA vector), hence NA too."""
    if a <= n * p:
        return 0.0
    if p <= 0.0:
        return -math.inf
    x = a / n
    if x >= 1.0:
        return n * math.log(p)
    kl = x * math.log(x / p) + (1.0 - x) * (math.log1p(-x) - math.log1p(-p))
    return -n * kl


@functools.lru_cache(maxsize=None)
def chernoff_quantile(n: int, p: float, log2_eps: float) -> int:
    """Largest value X exceeds only with probability <= 2^log2_eps:
    (smallest a with the Chernoff bound on P(X >= a) <= eps) - 1, so
    P(X > returned) <= eps."""
    target = log2_eps * math.log(2.0)
    if p >= 1.0:
        return n
    lo = int(n * p)  # tail bound is 1 here
    hi = n
    if binom_tail_log(n, p, hi) > target:
        return n  # even the deterministic max cannot certify: cap.
    while lo + 1 < hi:
        mid = (lo + hi) // 2
        if binom_tail_log(n, p, mid) <= target:
            hi = mid
        else:
            lo = mid
    return max(0, hi - 1)


# --------------------------------------------------------------------------
# The corrected population object and per-stage envelopes.
#
# Anatomy [verified against code]: the reply parked on the depth-d
# stream discusses the children of one depth-(d-1) scope parent
# (adapter/scope.rs: Scope<H> holds parent: Prefix<S<H>>): reactions at
# depth d, Query listing entries at depth d+1 (decode.rs). The
# answering walk (materialized/work/answer.rs) merge-joins the peer's
# listing against its own children and asks one question per peer entry
# that is disputed (Both, hashes differ -> Query(listing)) or absent
# (Right -> Query(empty)); own-only children become Supply reactions
# and ask nothing. Under the fully divergent worst case every peer
# entry is queried, so:
#
#   S(d) = queried listing entries at depth d-1
#        <= occupied depth-(d-1) slots of the peer corpus     [each slot
#           is listed at most once], and
#        <= 256 x jointly occupied depth-(d-2) slots          [an entry is
#           listed only inside a Query reaction, which requires its
#           parent to be disputed, i.e. occupied by BOTH corpora].
#
# The second bound is what turns the population back off past the joint
# frontier: 256^(d-2) >> N^2 kills S(d), which is also why the leaf
# stages (d = 32) are population-empty at every realistic declaration.
#
# Divergence worst case [derived]: every stage quantity is monotone in
# each side's corpus occupancy, and honest replicas each hold <= N
# elements, so evaluating with BOTH sides at N uniform leaves, fully
# divergent, pointwise dominates every honest configuration. D_max = N
# per side.
# --------------------------------------------------------------------------

def occ_hi(n: int, j: int) -> int:
    """High-probability bound on occupied depth-j slots of one N-corpus.

    Deterministic caps: the slot count 256^j and the corpus N (each
    leaf occupies exactly one slot per level); Chernoff quantile of the
    negatively associated occupancy indicators at 2^-48."""
    if j <= 0:
        return 1 if n >= 1 else 0
    slots = 256 ** j
    cap = min(slots, n)
    p = p_occ(n, j)
    if p <= 0.0:
        return 0
    return min(cap, chernoff_quantile(slots, p, LOG2_EPS_STAGE))


def joint_hi(n: int, j: int) -> int:
    """High-probability bound on jointly occupied depth-j slots of two
    disjoint N-corpora (slot indicators: product of independent NA
    families, per-slot probability exactly p_occ^2)."""
    if j <= 0:
        return 1 if n >= 1 else 0
    slots = 256 ** j
    p = p_occ(n, j) ** 2
    if p <= 0.0:
        return 0
    return min(slots, n, chernoff_quantile(slots, p, LOG2_EPS_STAGE))


def stage_pop(n: int, d: int) -> int:
    """S(d): hp bound on parked replies at the depth-d stage -- the
    queried-listing-entry count at depth d-1 (see the block comment).

    The listed route is a B1-style aggregate: entries are the replier's
    children of jointly occupied depth-(d-2) parents, at most the joint
    parent count times the per-parent children quantile (union-bounded
    over all candidate parents). Past the joint frontier the quantile
    sits near 1, not 256 -- the flat fan factor there is pure slack."""
    if d < 1:
        return 0
    if d == 1:
        return 1  # the opening reply (or the synthetic opening question)
    listed = joint_hi(n, d - 2) * _occ_quantile(n, d - 2, FAN, _per_slot_eps(d - 2))
    return min(occ_hi(n, d - 1), listed)


def _per_slot_eps(j: int) -> float:
    """Per-scope tail level for a max-over-parents union bound at parent
    depth j: 2^-48 split over the 256^j candidate prefixes. Using the
    slot count rather than S keeps the union valid without conditioning
    on 'is a queried parent' (P(queried and X >= a) <= P(X >= a))."""
    return LOG2_EPS_STAGE - 8.0 * min(j, 40)


def _occ_quantile(n: int, j: int, sub_slots: int, log2_eps: float) -> int:
    """Quantile of the occupied sub-slot count under one depth-j parent
    prefix at 2^log2_eps.

    Two Chernoff routes, take the min: `sub_slots` slots each occupied
    with the exact per-slot probability, and the leaves-under cap
    (occupied sub-slots <= leaves beneath the prefix ~ Binomial(N,
    256^-j)); both routes are sums of negatively associated indicators."""
    if n <= 0:
        return 0
    levels = round(math.log2(sub_slots) / 8)
    by_slots = chernoff_quantile(sub_slots, p_occ(n, j + levels), log2_eps)
    if j == 0:
        return min(sub_slots, by_slots)
    by_leaves = chernoff_quantile(n, 256.0 ** -j, log2_eps)
    return min(sub_slots, by_slots, by_leaves)


# Aggregate (per-stage sum) bounds. The parked set at a stage is an
# adversarially scheduled subset of at most min(K, S) of the stage's
# replies; each parked reply owns one distinct depth-(d-1) parent, and
# distinct parents own disjoint sub-slot ranges. Three independently
# valid bounds, charged at their minimum:
#
#   B1 (max bound):        min(K, S) x per-parent quantile at the
#       union level over all 256^(d-1) candidate parents;
#   B2 (threshold bound):  min(K, S) x per-parent quantile at 2^-20,
#       plus (hp count of parents exceeding that quantile) x the
#       deterministic per-parent cap -- valid because every parked reply
#       is either below the threshold or one of the counted exceeders;
#   B3 (corpus total):     disjoint parent sub-slot ranges make the
#       stage total at sub-slot level d-1+levels at most the corpus's
#       occupied slot count there: min(corpus, 256^(d-1+levels)) --
#       deterministic (each leaf occupies exactly one slot per level;
#       this is the identity behind byte-window-plan section B0.3's
#       rejected-global-budget note).

LOG2_EPS_TYPICAL = -20  # B2's per-parent threshold level.


def _aggregate_occ(n: int, j: int, sub_slots: int, k: int, s: int) -> int:
    """High-probability bound on the summed occupied sub-slot counts
    under the min(k, s) parked depth-j parents (corpus size n)."""
    m = min(k, s)
    if m == 0 or n <= 0:
        return 0
    levels = round(math.log2(sub_slots) / 8)
    b1 = m * _occ_quantile(n, j, sub_slots, _per_slot_eps(j))
    q_typ = _occ_quantile(n, j, sub_slots, LOG2_EPS_TYPICAL)
    slots = 256 ** min(j, 40)
    n_over = chernoff_quantile(slots, 2.0 ** LOG2_EPS_TYPICAL, LOG2_EPS_STAGE)
    b2 = m * q_typ + n_over * sub_slots
    b3 = min(n, 256 ** (j + levels))
    return min(b1, b2, b3)


# Stage sets [derived from remote/codec/signal.rs]: heights 0..=31 are
# streamed; stream 0 is height 31 for both speakers (the initiator's is
# the opening question, replayed locally as the synthetic opening);
# successive streams descend two heights. Initiator reply heights:
# {30, 28, ..., 2, 0}; responder: {31, 29, ..., 1, 0}. Depth = 32 - h.

def skeleton_depths(peer_role: str) -> list[int]:
    if peer_role == "initiator":
        return list(range(2, DEPTH + 1, 2))
    return list(range(1, DEPTH, 2)) + [DEPTH]


def refs_depths() -> list[int]:
    # The walk holds in-flight questions/scopes about every level's
    # parents regardless of reply parity; the question answered at the
    # depth-d stage holds up to a fan of depth-d children under its
    # depth-(d-1) parent (materialized Query { prefix, ours }).
    return list(range(1, DEPTH + 1))


def envelope_bytes(k: int, n: int, peer_role: str, agg=_aggregate_occ, pop=stage_pop) -> int:
    """Simultaneous parked-bytes envelope at window K, declaration N,
    with the peer speaking `peer_role`'s streams:

      sum over the peer's reply stages (depth d, parent depth d-1) of
          min(K, S(d)) x C_REPLY                 (parked containers)
        + C_REACTION x Agg(2N, d-1, 256)         (reactions, union corpus)
        + 17 B x Agg(N, d-1, 256^2)              (listing entries; the
                                                  leaf stages d = 32
                                                  carry no listings)
      + [peer is initiator] x OPENING_SKELETON_BYTES
      + sum over every level of
          (32 + 340) B x Agg(N, d-1, 256)        (question/scope queues)

    There is no unconditional full-K term: under the model of record,
    stage populations ARE corpus-bounded at 2^-40 -- that is exactly
    what S(d) certifies.
    """
    total = 0
    if peer_role == "initiator":
        total += OPENING_SKELETON_BYTES
    for d in skeleton_depths(peer_role):
        s = pop(n, d)
        if s == 0:
            continue
        total += min(k, s) * C_REPLY
        # Reactions span the union of both sides' children; disjoint
        # corpora of N each give the occupancy of a 2N corpus.
        total += C_REACTION * agg(2 * n, d - 1, FAN, k, s)
        if d < DEPTH:
            total += LISTING_ENTRY_BYTES * agg(n, d - 1, FAN * FAN, k, s)
    for d in refs_depths():
        s = pop(n, d)
        if s == 0:
            continue
        total += (CHILD_CONTAINER_BYTES + NODE_BYTES) * agg(n, d - 1, FAN, k, s)
    return total


def k_solve(n: int, budget: int = DEFAULT_BUDGET, agg=_aggregate_occ, pop=stage_pop):
    """Widest K whose envelope fits the post-slack budget, worst peer
    role. Returns (K, capped): capped means the envelope saturates below
    the budget at every K -- the declared corpus cannot bind it, and K
    is reported as the widest stage population (beyond which additional
    capacity is physically idle: min(K, S) saturates at every stage)."""
    steady = max(0, budget - STAGES * DECODE_SLACK_BYTES)

    def fits(k: int) -> bool:
        return all(
            envelope_bytes(k, n, role, agg, pop) <= steady
            for role in ("initiator", "responder")
        )

    cap = max((pop(n, d) for d in range(1, DEPTH + 1)), default=0)
    cap = max(cap, 1)
    if fits(cap):
        return cap, True
    lo, hi = 1, cap
    if not fits(lo):
        return 1, False
    while lo < hi:
        mid = lo + (hi - lo + 1) // 2
        if fits(mid):
            lo = mid
        else:
            hi = mid - 1
    return lo, False


def k_sharp(n: int, budget: int = DEFAULT_BUDGET):
    return k_solve(n, budget)


# --------------------------------------------------------------------------
# The integer-honest adoptable envelope: the same structure with pure
# u128-friendly integer bounds in place of the Chernoff quantiles. The
# sweep in check_integer_dominates verifies each integer bound is >= its
# exact-Chernoff counterpart across the full parameter range, so the
# integer envelope inherits the 2^-40 guarantee (a min over FEWER,
# individually larger bounds can only be larger).
# --------------------------------------------------------------------------

def _t_int(j: int) -> int:
    """Integer upper bound on ln(2) x the union tail bits at parent
    depth j: ceil(0.7 x (48 + 8 min(j, 40)))."""
    return (7 * (48 + 8 * min(j, 40)) + 9) // 10


def _bernstein(mu_hi: int, t: int) -> int:
    """Integer quantile from the multiplicative Chernoff tail
    P(X >= mu + x) <= exp(-x^2 / (2 mu + x)) (valid for sums of NA
    indicators; weaker than the KL form the exact envelope uses, so
    dominance is structural): x = isqrt(2 mu T) + T suffices for tail
    <= e^-T, and T = _t_int covers the 2^-(48+8j) union level."""
    return mu_hi + math.isqrt(2 * mu_hi * t) + t


def occ_int(n: int, j: int) -> int:
    """Integer occupied-slot envelope: both caps are deterministic (one
    slot per level per leaf), so no concentration term is needed."""
    if j <= 0:
        return 1 if n >= 1 else 0
    return min(256 ** j, n)


def _small_mean_quantile(num: int, den: int, t: int) -> int | None:
    """Integer quantile for the sub-unit-mean regime, or None if the
    mean mu = num/den is not clearly sub-unit. Uses the Poisson-type
    tail P(X >= a) <= (e mu)^a: with b = floor(log2(den)) -
    bit_length(num) >= 5 (so mu <= 2^-(b-1) and e mu <= 2^(2.45-b)),
    a = t // (b - 3) + 2 gives tail <= 2^-t; and if e mu <= 2^-t even
    a = 1 exceeds the level, so the quantile is 0 (the extra 2^2 > e
    keeps this aligned with the exact side's KL bound, which is what
    dominance is asserted against). Dominance over the exact KL
    quantile is structural (e^-n.KL <= (e mu / a)^a) and
    sweep-verified."""
    if num * (1 << (t + 2)) < den:
        return 0
    b = den.bit_length() - num.bit_length()
    if b >= 5:
        return t // (b - 3) + 2
    return None


def joint_int(n: int, j: int) -> int:
    """Integer jointly-occupied-slot envelope: slot and corpus caps plus
    a quantile at the pair mean N^2 / 256^j (the exact mean is
    256^j p_occ^2 <= N^2/256^j): Bernstein in the bulk (flat 2^-48
    level, t = 34 >= 48 ln 2), Poisson-type past the joint frontier
    where the mean is sub-unit (sweep-verified against joint_hi)."""
    if j <= 0:
        return 1 if n >= 1 else 0
    slots = 256 ** j
    small = _small_mean_quantile(n * n, slots, 48)
    if small is not None:
        return min(slots, n, small)
    return min(slots, n, _bernstein(n * n // slots + 1, 34))


def q_leaves_int(n: int, j: int) -> int:
    """Integer per-parent quantile, leaves route: occupied sub-slots <=
    leaves under the prefix ~ Binomial(N, 256^-j), Bernstein-bounded at
    the union level in the bulk and Poisson-type below unit mean
    (sweep-verified against the exact Chernoff quantile)."""
    if j > 0:
        small = _small_mean_quantile(n, 256 ** j, 48 + 8 * min(j, 40))
        if small is not None:
            return small
    mu_hi = (n // 256 ** j if j > 0 else n) + 1
    return _bernstein(mu_hi, _t_int(j))


def q_slots_int(n: int, j: int, sub_slots: int) -> int:
    """Integer per-parent quantile, slots route. The mean occupied
    sub-slot count is sub_slots x (1 - (1 - 256^-(j+levels))^N), and
    1 - e^-x <= 2x/(2+x) gives the integer mean envelope
    mean_hi = 2 N sub_slots // (2 x 256^(j+levels) + N); Bernstein
    slack at the union level on top (sweep-verified)."""
    levels = round(math.log2(sub_slots) / 8)
    lev_slots = 256 ** (j + levels)
    mean_hi = min(sub_slots, 2 * n * sub_slots // (2 * lev_slots + n) + 1)
    return min(sub_slots, _bernstein(mean_hi, _t_int(j)))


def c_q_int(n: int, j: int) -> int:
    """Integer per-parent children quantile (the fan-slot route and the
    leaves route, min)."""
    return min(FAN, q_leaves_int(n, j), q_slots_int(n, j, FAN))


def stage_pop_int(n: int, d: int) -> int:
    """Integer stage population: min of the occupied-slot cap at d-1 and
    the listed-under-disputed-parents aggregate joint(d-2) x per-parent
    children quantile."""
    if d < 1:
        return 0
    if d == 1:
        return 1
    listed = joint_int(n, d - 2) * c_q_int(n, d - 2)
    return min(occ_int(n, d - 1), listed)


def _aggregate_occ_int(n: int, j: int, sub_slots: int, k: int, s: int) -> int:
    m = min(k, s)
    if m == 0 or n <= 0:
        return 0
    levels = round(math.log2(sub_slots) / 8)
    q = min(sub_slots, q_leaves_int(n, j), q_slots_int(n, j, sub_slots))
    b1 = m * q
    b3 = min(n, 256 ** (j + levels))
    return min(b1, b3)


def k_int(n: int, budget: int = DEFAULT_BUDGET):
    return k_solve(n, budget, agg=_aggregate_occ_int, pop=stage_pop_int)


def check_integer_dominates() -> None:
    """Verify the integer bounds dominate their exact counterparts over
    a dense sweep of (N, j) -- the integer envelope must never be
    tighter than the certified one. (B3 is shared verbatim; B1's factors
    dominate pointwise; the integer form omits B2, which only raises it.)
    """
    ns = [2, 10, 100, 10**4, 10**6, 256**3, 10**8, 256**4, 10**10,
          256**5, 10**12, 10**13, 2**50]
    for n in ns:
        for j in range(0, DEPTH + 1):
            assert occ_int(n, j) >= occ_hi(n, j), (n, j, "occ")
            assert joint_int(n, j) >= joint_hi(n, j), (n, j, "joint")
            for sub in (FAN, FAN * FAN):
                exact_q = _occ_quantile(n, j, sub, _per_slot_eps(j))
                q_i = min(sub, q_leaves_int(n, j), q_slots_int(n, j, sub))
                assert q_i >= exact_q, (n, j, sub, q_i, exact_q)
        for d in range(1, DEPTH + 1):
            assert stage_pop_int(n, d) >= stage_pop(n, d), (n, d, "S")


# --------------------------------------------------------------------------
# L(N): the simultaneously-heavy-stage count (B0.7's budget divisor).
#
# A_d(N) is a stage's ACHIEVABLE parked bytes: the skeleton envelope at
# unbounded window (min(K, S) -> S), i.e. what the stage could hold if
# nothing but its own population and the corpus identities limited it.
# The reference edges are excluded: B0.7 gives them their own meter,
# and the per-stream advertisement denominates parked replies.
#
# The heaviness unit is the equal split share = steady/17, and L is the
# CLAMPED FRACTIONAL count L(N) = max(1, max_role sum_d min(1, A_d /
# share)) -- smooth and monotone in N because every A_d is. The choice
# is not cosmetic; it makes the operating promise a theorem:
#
#   With per-stream advertisement adv = steady/L, meter-enforced, the
#   total parked bytes are at most sum_d min(A_d, adv) <= steady.
#   Proof: L <= 17 so adv >= share. Split stages into big (A_d >= adv),
#   mid (share <= A_d < adv), small (A_d < share): big and mid each
#   contribute <= adv to the sum and >= 1 to L; small contribute A_d =
#   share x (A_d/share) <= adv x (A_d/share) and exactly A_d/share to
#   L. So sum_d min(A_d, adv) <= adv x L = steady.
#
# The hard ceiling 17 x adv = steady x 17/L is what the meters enforce
# regardless of declaration error; the operating promise above is the
# uniformity consequence, holding whenever the A_d envelopes hold --
# i.e. except with probability < 2^-40 per session.
# --------------------------------------------------------------------------

UNBOUNDED_K = 1 << 100


def stage_saturation_bytes(n: int, d: int, peer_role: str) -> int:
    """A_d(N): the depth-d stage's achievable parked skeleton bytes."""
    if peer_role == "initiator" and d == 1:
        return OPENING_SKELETON_BYTES
    s = stage_pop(n, d)
    if s == 0:
        return 0
    total = s * C_REPLY
    total += C_REACTION * _aggregate_occ(2 * n, d - 1, FAN, UNBOUNDED_K, s)
    if d < DEPTH:
        total += LISTING_ENTRY_BYTES * _aggregate_occ(n, d - 1, FAN * FAN, UNBOUNDED_K, s)
    return total


def l_of_n(n: int, share: float = SHARE) -> float:
    """L(N): clamped fractional simultaneously-heavy-stage count, worst
    peer role."""
    best = 0.0
    for role in ("initiator", "responder"):
        depths = skeleton_depths(role) + ([1] if role == "initiator" else [])
        frac = sum(
            min(1.0, stage_saturation_bytes(n, d, role) / share) for d in depths
        )
        best = max(best, frac)
    return max(1.0, best)


def heavy_count(n: int, theta: float) -> int:
    """Integer heavy-stage count at threshold theta x share, worst role
    (the insensitivity probe for L's threshold choice)."""
    best = 0
    for role in ("initiator", "responder"):
        depths = skeleton_depths(role) + ([1] if role == "initiator" else [])
        count = sum(
            1 for d in depths
            if stage_saturation_bytes(n, d, role) >= theta * SHARE
        )
        best = max(best, count)
    return best


# --------------------------------------------------------------------------
# Brute-force trie statistics from real uniform 32-byte keys.
#
# We draw full 32-byte keys, then compute per-depth statistics from the
# leading 8 bytes as uint64 prefixes: every statistic below depends only
# on prefixes of length <= j + 2 <= 8. Keys are deduplicated on the
# full 32 bytes.
# --------------------------------------------------------------------------

BRUTE_MAX_D = 6


def draw_keys_u64(n: int, rng: np.random.Generator) -> np.ndarray:
    raw = rng.integers(0, 256, size=(n, 32), dtype=np.uint8)
    raw = np.unique(raw, axis=0)  # dedupe full 32-byte keys (rarely fires)
    hi = raw[:, :8].astype(np.uint64)
    shifts = np.uint64(8) * np.arange(7, -1, -1, dtype=np.uint64)
    return np.sort((hi << shifts).sum(axis=1, dtype=np.uint64))


def prefix(keys_u64: np.ndarray, j: int) -> np.ndarray:
    return keys_u64 >> np.uint64(8 * (8 - j))


def trie_stats(keys_u64: np.ndarray, max_j: int = BRUTE_MAX_D) -> dict:
    """Exact per-depth statistics of one corpus.

    Returns, per depth j: occupied prefix count G_j; branch-node count
    B_j (occupied prefixes with >= 2 distinct continuation bytes, the
    OLD population object, kept for the contrast column); per-OCCUPIED-
    parent children counts (occupied depth-(j+1) slots beneath) and
    grandchild entry counts (occupied depth-(j+2) slots beneath) -- the
    per-reply reaction and listing-entry shapes of a reply whose scope
    parent sits at depth j."""
    uniq = {j: np.unique(prefix(keys_u64, j)) for j in range(0, max_j + 3)}
    out = {}
    for j in range(1, max_j + 1):
        # Every occupied depth-j prefix has >= 1 child and >= 1
        # grandchild, so the grouped counts align with uniq[j] sorted.
        _, child_counts = np.unique(uniq[j + 1] >> np.uint64(8), return_counts=True)
        _, gc_counts = np.unique(uniq[j + 2] >> np.uint64(16), return_counts=True)
        out[j] = dict(
            occupied=len(uniq[j]),
            branch=int((child_counts >= 2).sum()),
            children=child_counts,
            entries=gc_counts,
        )
    return out


def two_corpus_stats(n: int, rng: np.random.Generator, max_j: int = BRUTE_MAX_D) -> dict:
    """The corrected population object, measured on two disjoint uniform
    corpora. A parks the replies B speaks. Per depth j we measure:

      joint    = depth-j slots occupied by both corpora (the disputed
                 parents whose Query reactions carry listings);
      listed   = B's depth-j prefixes whose depth-(j-1) parent is
                 jointly occupied -- the queried-listing-entry count,
                 i.e. the measured population of stage d = j + 1;
      branch   = B's branch nodes at depth j (the OLD object, for the
                 contrast ratio);
      entries_all    = B's depth-(j+2) slots under each listed parent
                 (the envelope's per-reply g for stage d = j + 1);
      entries_shared = the same restricted to depth-(j+1) children both
                 sides occupy (Query(empty) reactions carry no listing),
                 the true per-reply skeleton load.
    """
    a = draw_keys_u64(n, rng)
    b = draw_keys_u64(n, rng)
    # Disjointness on the 8-byte reduction is automatic w.h.p.; enforce.
    b = b[~np.isin(b, a)]
    tb = trie_stats(b, max_j)
    out = {}
    for j in range(1, max_j + 1):
        a_j = np.unique(prefix(a, j))
        b_j = np.unique(prefix(b, j))
        joint_j = b_j[np.isin(b_j, a_j, assume_unique=True)]
        a_up = np.unique(prefix(a, j - 1))
        listed_mask = np.isin(b_j >> np.uint64(8), a_up, assume_unique=False)
        # A depth-(j-1) parent is disputed iff both corpora occupy it;
        # B occupies it by construction (it has a B child).
        listed = b_j[listed_mask]
        # Per-listed-parent entries: B's grandchild counts, aligned with
        # b_j via the same grouping trie_stats used at depth j.
        entries_all = tb[j]["entries"][listed_mask]
        # Shared-children refinement: B's grandchild slots under listed
        # parents, counting only children (depth j+1) that A occupies.
        b1 = np.unique(prefix(b, j + 1))
        a1 = np.unique(prefix(a, j + 1))
        shared_children = b1[np.isin(b1, a1, assume_unique=True)]
        b2 = np.unique(prefix(b, j + 2))
        under_shared = b2[np.isin(b2 >> np.uint64(8), shared_children, assume_unique=True)]
        parents2, counts2 = np.unique(under_shared >> np.uint64(16), return_counts=True)
        idx = np.searchsorted(parents2, listed)
        entries_shared = np.zeros(len(listed), dtype=np.int64)
        inside = idx < len(parents2)
        hit = np.zeros(len(listed), dtype=bool)
        hit[inside] = parents2[idx[inside]] == listed[inside]
        entries_shared[hit] = counts2[idx[hit]]
        out[j] = dict(
            occupied=len(b_j),
            branch=tb[j]["branch"],
            joint=len(joint_j),
            listed=len(listed),
            entries_all=entries_all,
            entries_shared=entries_shared,
        )
    return out


# --------------------------------------------------------------------------
# The large-N conditional sampler.
#
# Justification [derived]: for a fixed depth-j prefix, L ~ Binomial(N,
# 256^-j) exactly (uniform independent keys), and conditional on L the
# continuations are iid uniform, so child occupancy is an exact
# multinomial layer. We sample L, then place L balls uniformly into the
# child (256) and grandchild (256^2) slot spaces and count distinct
# occupied slots. When L is past the coupon-collector bound 4 M ln M
# for a slot space of size M, every slot is occupied except with
# probability <= M^-3 and we clamp to full (errs toward the envelope,
# by < 2^-30 per sample). The sampler is validated against the
# brute-force trie at N = 10^5 before use.
# --------------------------------------------------------------------------

def sample_parent(n: int, j: int, rng: np.random.Generator, n_samples: int) -> dict:
    """Per-parent occupancy under depth-j prefixes: leaves, occupied
    child slots, occupied grandchild slots, and the occupied mask."""
    q = 256.0 ** -j
    if j == 0:
        ls = np.full(n_samples, n)
    elif n * q < 1e17:
        ls = rng.binomial(n, q, size=n_samples)
    else:
        ls = np.full(n_samples, int(n * q))
    children = np.zeros(n_samples, dtype=np.int64)
    entries = np.zeros(n_samples, dtype=np.int64)
    full_g = int(4 * FAN * FAN * math.log(FAN * FAN))
    for i, l in enumerate(ls):
        l = int(l)
        if l == 0:
            continue
        if l >= full_g:
            children[i], entries[i] = FAN, FAN * FAN
            continue
        balls = rng.integers(0, FAN * FAN, size=l)
        if l > 4096:
            occupied = np.bincount(balls, minlength=FAN * FAN) > 0
            entries[i] = int(occupied.sum())
            children[i] = int(occupied.reshape(FAN, FAN).any(axis=1).sum())
        else:
            uniq = np.unique(balls)
            entries[i] = len(uniq)
            children[i] = len(np.unique(uniq >> 8))
    return dict(leaves=ls, children=children, entries=entries, occupied=ls > 0)


def cond_child_mean(n: int, j: int, rng: np.random.Generator, m: int) -> tuple[float, float]:
    """Measured per-parent (children, entries) means conditional on the
    parent being occupied. For lam = N/256^j < 0.01 an occupied parent
    holds a single leaf except with probability O(lam), so the
    conditional means are 1 + O(lam) and we return the analytic limit
    rather than rejection-sampling a 100x-waste event."""
    lam = n * (256.0 ** -j) if j > 0 else float(n)
    if lam < 0.01:
        return 1.0, 1.0
    # Adapt the sample count to the expected subtree size (as the
    # per-stage table does): the conditional means at ball-heavy depths
    # are sharply concentrated, so a few hundred samples suffice and
    # the total ball work stays bounded.
    if lam > 4096:
        m = max(200, int(min(m, 4e6 / lam)))
    sc = sample_parent(n, j, rng, m)
    mask = sc["occupied"]
    if not mask.any():
        return 1.0, 1.0
    return float(sc["children"][mask].mean()), float(sc["entries"][mask].mean())


def measured_stage_bytes(n: int, d: int, rng: np.random.Generator, m: int) -> float:
    """Mean-level measured achievable bytes of stage d: sampled
    conditional per-parent statistics x exact binomial slot marginals
    (the marginals are exact by the model's construction and are
    validated against the brute force in tier 1/2), capped by the
    measured-mean B3 corpus totals."""
    if d < 1:
        return 0.0
    j = d - 1
    if d == 1:
        s_meas = 1.0
    else:
        # E[listed] = 256^(j-1) x p_occ(A)^2-free product form:
        # P(parent jointly occupied) x E[B children | B occupies] uses
        # independence of the corpora: E[1_A 1_B c_B] = p_occ x E[1_B c_B].
        c_up, _ = cond_child_mean(n, j - 1, rng, m)
        p_up = p_occ(n, j - 1)
        s_meas = (256.0 ** (j - 1)) * p_up * p_up * c_up
        s_meas = min(s_meas, (256.0 ** j) * p_occ(n, j), float(n))
    r_cond_u, _ = cond_child_mean(2 * n, j, rng, m)
    _, g_cond = cond_child_mean(n, j, rng, m)
    if d >= DEPTH:
        g_cond = 0.0
    b1 = s_meas * (C_REPLY + C_REACTION * r_cond_u + LISTING_ENTRY_BYTES * g_cond)
    b3 = (
        s_meas * C_REPLY
        + C_REACTION * min(2 * n, (256.0 ** d) * p_occ(2 * n, d))
        + LISTING_ENTRY_BYTES
        * (min(n, (256.0 ** (d + 1)) * p_occ(n, d + 1)) if d < DEPTH else 0.0)
    )
    return min(b1, b3)


# --------------------------------------------------------------------------
# Reporting helpers.
# --------------------------------------------------------------------------

def fmt_bytes(b: float) -> str:
    for unit, div in [("GiB", 2**30), ("MiB", 2**20), ("KiB", 2**10)]:
        if b >= div:
            return f"{b / div:.2f} {unit}"
    return f"{b:.0f} B"


def fmt_n(n: int) -> str:
    for name, val in [("256^2", 256**2), ("256^3", 256**3), ("256^4", 256**4),
                      ("256^5", 256**5)]:
        if n == val:
            return name
    return f"{n:.0e}".replace("e+", "e")


def section(title: str) -> None:
    print(f"\n{'=' * 74}\n{title}\n{'=' * 74}")


def main() -> None:
    global C_REPLY, C_REACTION
    fast = "--fast" in sys.argv
    seeds = list(range(20 if fast else 100))
    n_brute = 10**5

    check_landed_replication()
    check_integer_dominates()
    print("landed-solve replication: OK (K_flat(256^5) = 4644; NODE_BYTES = 340 unique)")
    print("integer envelope dominates exact Chernoff quantiles: OK (sweep)")

    # ---- headline comparison table -------------------------------------
    section("K_sharp vs K_flat at the default 16 GiB budget")
    print(f"{'N':>8} {'K_flat':>8} {'K_sharp':>10} {'K_int':>10} "
          f"{'sharp/flat':>10} {'int/flat':>9}")
    for n in TABULATED_N:
        kf = k_flat(n)
        ks, cs = k_sharp(n)
        ki, ci = k_int(n)
        ks_s = f"{ks}*" if cs else f"{ks}"
        ki_s = f"{ki}*" if ci else f"{ki}"
        rs = "inf" if cs else f"{ks / kf:.2f}"
        ri = "inf" if ci else f"{ki / kf:.2f}"
        print(f"{fmt_n(n):>8} {kf:>8} {ks_s:>10} {ki_s:>10} {rs:>10} {ri:>9}")
    print("  * corpus-capped: the envelope saturates below the budget at every")
    print("    K; the printed value is the widest stage population.")
    for n in TABULATED_N:
        ks, _ = k_sharp(n)
        worst = max(
            envelope_bytes(ks, n, role) for role in ("initiator", "responder")
        )
        sat = max(
            envelope_bytes(UNBOUNDED_K, n, role) for role in ("initiator", "responder")
        )
        print(f"      N = {fmt_n(n):>7}: envelope at K_sharp {fmt_bytes(worst)}, "
              f"saturation {fmt_bytes(sat)} "
              f"(post-slack budget {fmt_bytes(STEADY)})")

    # ---- per-stage breakdown at the default declaration -----------------
    section("Per-stage envelope at N = 256^5 (default declaration)")
    ks, _ = k_sharp(DEFAULT_N)
    n = DEFAULT_N
    for role in ("initiator", "responder"):
        print(f"\npeer speaks as {role} (K = {ks}):")
        print(f"{'depth':>5} {'height':>6} {'S':>13} {'r_hi':>5} {'g_hi':>6} "
              f"{'skel bytes':>11} {'refs bytes':>11}")
        shown = 0
        for d in sorted(set(skeleton_depths(role)) | set(refs_depths())):
            s = stage_pop(n, d)
            if s == 0:
                continue
            in_skel = d in skeleton_depths(role)
            skel = 0
            if in_skel:
                skel = (
                    min(ks, s) * C_REPLY
                    + C_REACTION * _aggregate_occ(2 * n, d - 1, FAN, ks, s)
                    + (LISTING_ENTRY_BYTES
                       * _aggregate_occ(n, d - 1, FAN * FAN, ks, s)
                       if d < DEPTH else 0)
                )
            refs = (CHILD_CONTAINER_BYTES + NODE_BYTES) * _aggregate_occ(n, d - 1, FAN, ks, s)
            if skel < 1024 and refs < 1024 and shown > 8:
                continue  # elide sub-KiB tail stages from the report
            shown += 1
            g_hi = _occ_quantile(n, d - 1, FAN * FAN, _per_slot_eps(d - 1)) if d < DEPTH else 0
            print(f"{d:>5} {DEPTH - d:>6} {s:>13} "
                  f"{_occ_quantile(2 * n, d - 1, FAN, _per_slot_eps(d - 1)):>5} "
                  f"{g_hi:>6} "
                  f"{fmt_bytes(skel) if in_skel else '-':>11} {fmt_bytes(refs):>11}")
        env = envelope_bytes(ks, n, role)
        print(f"  total envelope at K = {ks}: {fmt_bytes(env)} "
              f"(budget after slack: {fmt_bytes(STEADY)})")

    # ---- L(N): the simultaneously-heavy-stage count ----------------------
    section("L(N): simultaneously-heavy-stage count (B0.7's divisor)")
    print(f"share = steady/17 = {fmt_bytes(SHARE)}")
    thetas = [0.25, 0.5, 1.0, 2.0, 4.0]
    print(f"{'N':>8} {'L(N)':>6} {'17/L':>6} "
          + " ".join(f"H(x{t:g})" for t in thetas)
          + "   adv = steady/L   ceiling = 17 x adv")
    for n in TABULATED_N:
        l = l_of_n(n)
        hs = [heavy_count(n, t) for t in thetas]
        adv = STEADY / l
        print(f"{fmt_n(n):>8} {l:>6.2f} {17 / l:>6.2f} "
              + " ".join(f"{h:>6}" for h in hs)
              + f"   {fmt_bytes(adv):>12}   {fmt_bytes(17 * adv):>12}")
    print("  H(x theta): integer heavy count at threshold theta x share, worst")
    print("  role. Insensitivity: membership decays ~256x per stage past the")
    print("  joint frontier, so a 16x threshold band moves H by at most ~1.")
    for n in TABULATED_N:
        for role in ("initiator", "responder"):
            depths = skeleton_depths(role) + ([1] if role == "initiator" else [])
            heavy = [d for d in sorted(depths)
                     if stage_saturation_bytes(n, d, role) >= SHARE]
            frac = sum(min(1.0, stage_saturation_bytes(n, d, role) / SHARE)
                       for d in depths)
            print(f"    N = {fmt_n(n):>7} {role:>9}: heavy depths {heavy}, "
                  f"fractional {frac:.2f}")

    # ---- L(N) smoothness / monotonicity ----------------------------------
    section("Smoothness and monotonicity of K_sharp(N) and L(N)")
    grid = []
    for kk in range(2, 7):
        for m in [-2, -1, 0, 1, 2]:
            grid.append(256**kk + m)
    grid += [int(10**e) for e in np.arange(6.0, 13.01, 0.25)]
    grid = sorted(set(g for g in grid if g >= 2))
    vals, lvals = [], []
    for n in grid:
        kk, capped = k_sharp(n)
        vals.append(math.inf if capped else float(kk))
        lvals.append(l_of_n(n))
    monotone = all(a >= b for a, b in zip(vals, vals[1:]))
    l_monotone = all(a <= b + 1e-12 for a, b in zip(lvals, lvals[1:]))
    worst_step = 0.0
    worst_l_step = 0.0
    for (n0, v0, l0), (n1, v1, l1) in zip(
        zip(grid, vals, lvals), zip(grid[1:], vals[1:], lvals[1:])
    ):
        if n1 - n0 <= 4:
            if math.isfinite(v0) and math.isfinite(v1) and v0 > 0:
                worst_step = max(worst_step, abs(v1 - v0) / v0)
            worst_l_step = max(worst_l_step, abs(l1 - l0) / max(l0, 1.0))
    print(f"  K_sharp monotone nonincreasing over {len(grid)}-point grid "
          f"(corpus-capped treated as +inf): {monotone}")
    print(f"  L(N) monotone nondecreasing over the same grid: {l_monotone}")
    print(f"  worst relative K step across adjacent (+-2) declarations at "
          f"256^k crossings: {worst_step:.2e}")
    print(f"  worst relative L step across the same crossings: {worst_l_step:.2e}")
    finite_from = next((n for n, v in zip(grid, vals) if math.isfinite(v)), None)
    print(f"  budget first binds at declared N = {finite_from:.4e} on this grid")

    # ---- container-constant sensitivity ---------------------------------
    section("Sensitivity: container constants (C_REPLY, C_REACTION)")
    base = {n: k_sharp(n) for n in TABULATED_N}
    C_REPLY, C_REACTION = 256, 64
    quad = {n: k_sharp(n) for n in TABULATED_N}
    C_REPLY, C_REACTION = 64, 32
    for n in TABULATED_N:
        (kb, cb), (kq, cq) = base[n], quad[n]
        if cb or cq:
            print(f"  N = {fmt_n(n):>7}: corpus-capped in both settings"
                  if cb and cq else
                  f"  N = {fmt_n(n):>7}: capped state changed -- inspect")
            continue
        delta = 100.0 * (kb - kq) / kb
        print(f"  N = {fmt_n(n):>7}: K_sharp {kb:>7} -> {kq:>7} "
              f"at 4x/2x containers ({delta:+.1f}%)")

    # ---- brute-force validation at N = 1e5 ------------------------------
    section(f"Brute-force trie validation (N = {n_brute}, {len(seeds)} seeds)")
    per_depth = {j: dict(occ=[], children=[], entries=[]) for j in range(1, BRUTE_MAX_D + 1)}
    for seed in seeds:
        rng = np.random.default_rng(seed)
        keys = draw_keys_u64(n_brute, rng)
        st = trie_stats(keys)
        for j in range(1, BRUTE_MAX_D + 1):
            per_depth[j]["occ"].append(st[j]["occupied"])
            per_depth[j]["children"].append(st[j]["children"])
            per_depth[j]["entries"].append(st[j]["entries"])
    print(f"{'j':>2} {'E[occ_j] pred':>13} {'occ_j meas':>11} {'occ_hi':>8} "
          f"{'c mean p/m':>13} {'g mean p/m':>15} {'g max meas':>10} {'g_hi':>6}")
    n = n_brute
    for j in range(1, BRUTE_MAX_D + 1):
        pred_o = 256**j * p_occ(n, j)
        meas_o = float(np.mean(per_depth[j]["occ"]))
        max_o = int(np.max(per_depth[j]["occ"]))
        ch = np.concatenate(per_depth[j]["children"])
        en = np.concatenate(per_depth[j]["entries"])
        # The envelope's per-parent c/g are built from unconditional
        # occupancy; measurements are conditioned on the parent being
        # occupied, so at thin depths measured means exceed the
        # unconditional predictions -- the quantile (union-bounded
        # without conditioning) is the bound.
        pred_c_uncond = FAN * p_occ(n, j + 1)
        pred_g_uncond = FAN * FAN * p_occ(n, j + 2)
        g_hi_v = _occ_quantile(n, j, FAN * FAN, _per_slot_eps(j))
        occ_hi_v = occ_hi(n, j)
        assert max_o <= occ_hi_v, f"occ_hi violated at j={j}: {max_o} > {occ_hi_v}"
        assert int(en.max()) <= g_hi_v, f"g_hi violated at j={j}"
        print(f"{j:>2} {pred_o:>13.1f} {meas_o:>11.1f} {occ_hi_v:>8} "
              f"{pred_c_uncond:>6.2f}/{ch.mean():<6.2f} "
              f"{pred_g_uncond:>7.2f}/{en.mean():<7.2f} "
              f"{int(en.max()):>10} {g_hi_v:>6}")
    print("  (pred c/g are the unconditional occupancy means the envelope uses;")
    print("   measured are conditioned on parent occupancy, so measured >= pred")
    print("   at thin depths is expected -- the quantile, not the mean, is the bound.)")

    # ---- sampler validation against brute force -------------------------
    section("Conditional sampler vs brute force (N = 1e5)")
    rng = np.random.default_rng(12345)
    print(f"{'j':>2} {'p_occ exact':>12} {'p_occ sampled':>14} {'brute':>12} "
          f"{'g|occ sampled':>14} {'brute':>10}")
    for j in range(2, 5):
        m = 200_000 if not fast else 40_000
        sc = sample_parent(n_brute, j, rng, m)
        p_hat = sc["occupied"].mean()
        brute_p = float(np.mean(per_depth[j]["occ"])) / 256**j
        en = np.concatenate(per_depth[j]["entries"])
        g_hat = sc["entries"][sc["occupied"]].mean() if sc["occupied"].any() else 0.0
        print(f"{j:>2} {p_occ(n_brute, j):>12.3e} {p_hat:>14.3e} {brute_p:>12.3e} "
              f"{g_hat:>14.3f} {en.mean():>10.3f}")

    # ---- two-corpus population measurement --------------------------------
    section(f"Two-corpus (disjoint, N = {n_brute} each): the population object")
    agg2 = {j: dict(occ=[], branch=[], joint=[], listed=[], e_all=[], e_sh=[])
            for j in range(1, BRUTE_MAX_D + 1)}
    for seed in seeds:
        rng = np.random.default_rng(10_000 + seed)
        tc = two_corpus_stats(n_brute, rng)
        for j in tc:
            agg2[j]["occ"].append(tc[j]["occupied"])
            agg2[j]["branch"].append(tc[j]["branch"])
            agg2[j]["joint"].append(tc[j]["joint"])
            agg2[j]["listed"].append(tc[j]["listed"])
            agg2[j]["e_all"].append(tc[j]["entries_all"])
            agg2[j]["e_sh"].append(tc[j]["entries_shared"])
    print(f"{'j':>2} {'joint pred':>10} {'joint meas':>10} {'listed meas':>11} "
          f"{'S(j+1)':>10} {'branch':>8} {'listed/branch':>13} "
          f"{'E e_all':>8} {'E e_sh':>7}")
    for j in range(1, BRUTE_MAX_D + 1):
        pred_joint = 256**j * p_occ(n_brute, j) ** 2
        meas_joint = float(np.mean(agg2[j]["joint"]))
        meas_listed = float(np.mean(agg2[j]["listed"]))
        max_listed = int(np.max(agg2[j]["listed"]))
        br = float(np.mean(agg2[j]["branch"]))
        s_env = stage_pop(n_brute, j + 1)
        assert max_listed <= s_env, f"population envelope violated at j={j}"
        ea = np.concatenate(agg2[j]["e_all"]) if meas_listed else np.array([0])
        es = np.concatenate(agg2[j]["e_sh"]) if meas_listed else np.array([0])
        ratio = meas_listed / br if br else float("inf")
        print(f"{j:>2} {pred_joint:>10.1f} {meas_joint:>10.1f} {meas_listed:>11.1f} "
              f"{s_env:>10} {br:>8.1f} {ratio:>13.1f} "
              f"{ea.mean():>8.2f} {es.mean():>7.2f}")
    print("  listed = queried listing entries at depth j = population of stage")
    print("  d = j+1; S(j+1) is its 2^-48 envelope (asserted, all seeds).")
    print("  listed/branch is the corrected-object vs old-object ratio; e_sh")
    print("  (entries under mutually occupied children only) is the true")
    print("  skeleton load -- the envelope's one-sided e_all stays generous.")

    # ---- per-stage predicted vs measured at tabulated N ------------------
    section("Predicted vs measured per stage at the tabulated N (sampler)")
    m = 200 if fast else 1_000  # per seed; every seed participates.
    for n in TABULATED_N:
        print(f"\nN = {fmt_n(n)} ({len(seeds)} seeds x {m} parent samples/depth):")
        print(f"{'d':>2} {'S env':>13} {'S meas (mean)':>13} "
              f"{'r meas':>7} {'g pred':>8} {'g meas':>8} "
              f"{'g q99.9 meas':>12} {'g_hi':>7} {'flag':>6}")
        depths = [d for d in range(1, 13) if stage_pop(n, d) > 0]
        for d in depths:
            j = d - 1
            ch_all, en_all = [], []
            lam = n * 256.0 ** -j if j > 0 else float(n)
            if lam < 0.01:
                s_meas_mean = None
                ch = en = np.array([1.0])
            else:
                mean_l = max(1.0, lam)
                m_eff = int(min(m, max(200, 2e6 / mean_l))) if mean_l > 4096 else m
                for seed in seeds:
                    rng = np.random.default_rng(seed * 1_000 + d)
                    sc = sample_parent(n, j, rng, m_eff)
                    mask = sc["occupied"]
                    if mask.any():
                        ch_all.append(sc["children"][mask])
                        en_all.append(sc["entries"][mask])
                ch = np.concatenate(ch_all) if ch_all else np.array([0])
                en = np.concatenate(en_all) if en_all else np.array([0])
            # Measured population: independence-product of exact
            # marginals and the sampled conditional child mean (tier-2
            # validates the product form at N = 1e5).
            rng = np.random.default_rng(777 * d + 1)
            if d == 1:
                s_meas = 1.0
            else:
                c_up, _ = cond_child_mean(n, j - 1, rng, 2_000 if fast else 20_000)
                s_meas = (256.0 ** (j - 1)) * p_occ(n, j - 1) ** 2 * c_up
                s_meas = min(s_meas, (256.0 ** j) * p_occ(n, j), float(n))
            g_pred = FAN * FAN * p_occ(n, d + 1)
            g_hi_v = _occ_quantile(n, j, FAN * FAN, _per_slot_eps(j))
            q999 = float(np.quantile(en, 0.999)) if len(en) > 100 else float(en.max())
            flag = ""
            if q999 > g_hi_v:
                flag = "VIOL"
            elif q999 > 0 and g_hi_v / max(q999, 1) > 2:
                flag = ">2x"
            s_env = stage_pop(n, d)
            assert s_meas <= s_env * 1.001, f"population mean exceeds envelope at d={d}"
            print(f"{d:>2} {s_env:>13} {s_meas:>13.3e} "
                  f"{ch.mean():>7.1f} {g_pred:>8.1f} {en.mean():>8.1f} "
                  f"{q999:>12.1f} {g_hi_v:>7} {flag:>6}")
        print("  flags: VIOL = measured 99.9% exceeds the 2^-48 envelope "
              "(must never happen); >2x = envelope more than 2x the measured "
              "99.9% (expected at thin stages: the envelope holds at 2^-48, "
              "not 10^-3).")

    # ---- measured heavy-stage counts vs L(N) ------------------------------
    section("Measured simultaneous heavy-stage counts vs L(N)")
    m_l = 2_000 if fast else 20_000
    l_seeds = seeds[: min(20, len(seeds))]
    print(f"{'N':>8} {'L(N)':>6} {'H(x1)':>6} {'H_meas mode':>11} {'min..max':>9}")
    for n in TABULATED_N:
        counts = []
        for seed in l_seeds:
            rng = np.random.default_rng(50_000 + seed)
            best = 0
            for role in ("initiator", "responder"):
                depths = skeleton_depths(role) + ([1] if role == "initiator" else [])
                c = 0
                for d in depths:
                    if d == 1 and role == "initiator":
                        a_meas = float(OPENING_SKELETON_BYTES)
                    elif stage_pop(n, d) == 0:
                        a_meas = 0.0
                    else:
                        a_meas = measured_stage_bytes(n, d, rng, m_l)
                    if a_meas >= SHARE:
                        c += 1
                best = max(best, c)
            counts.append(best)
        counts = np.array(counts)
        mode = int(np.bincount(counts).argmax())
        print(f"{fmt_n(n):>8} {l_of_n(n):>6.2f} {heavy_count(n, 1.0):>6} "
              f"{mode:>11} {counts.min():>4}..{counts.max()}")
    print("  H_meas: per-seed heavy count from sampled conditional statistics")
    print("  x exact slot marginals at mean level (worst role). Agreement with")
    print("  H(x1) shows heavy membership is not a quantile artifact; L(N)")
    print("  upper-bounds both by construction (hp envelope >= mean).")

    print("\ndone.")


if __name__ == "__main__":
    main()
