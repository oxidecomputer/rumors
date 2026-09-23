#!/usr/bin/env python3
"""Reproduce the modeled figures in the single-socket parking note.

The script transcribes the window model in
`src/tree/mirror/streaming/window.rs` (the integer quantiles `C`, `L`, `S`,
`occupied`, and the charge) and extends it with the note's design: the parking price, the set
statistic, and the shared window that exchanged budgets give both peers.
It also computes the root-comparison bound the design considered and
dropped (exposition section 6.8). It also
runs the duplex simulation of exposition section 10.3. Slot sizes are
estimates for the in-memory backend; the implementation derives them with
`size_of`. Run it with `python3 sizing-model.py`.
"""

import math
import random
import statistics

# Tree fan, key length in bytes (and so tree depth), and the per-statistic
# tail exponent: each quantile holds with probability 1 - 2^-48.
FAN, KEY, TAIL_BITS = 256, 32, 48

# Estimated byte sizes (in-memory backend).
REFERENCE_BYTES = 8 + 65      # one queued child reference: node handle + query/resolution/listing slots
SCOPE_FIXED_BYTES = 128       # per queued scope: inline Query + Resolution
LEAF_REQUEST_BYTES = 40       # one queued leaf request at the last level
SUPPLY_FANS = 17 * 257 * 48   # decode fans, one per reply stream; a fixed charge
REACTION_BYTES = 32           # one parked reaction
ENTRY_BYTES = 25              # one (radix, digest) listing entry
FRAME_BYTES = 1_830_000       # default supply-run budget, about one frame


# --- The window model's quantiles, transcribed ---------------------------

def pow256(j):
    """256^j, saturated at 2^128 - 1 as the Rust model's u128 saturates."""
    return (1 << 128) - 1 if j >= 16 else 1 << (8 * j)


def tail_exponent(j):
    """The Bernstein tail for a depth-j statistic: 48 + 8j bits, in nats.

    Converts bits to nats with ln 2 rounded up to 0.7, then rounds up.
    """
    return -(-(7 * (TAIL_BITS + 8 * j)) // 10)


def bernstein(mean_hi, t):
    """Upper quantile at tail e^-t for a count whose mean is at most `mean_hi`."""
    return mean_hi + math.isqrt(2 * mean_hi * t) + t


def small_mean_quantile(num, j, t):
    """Upper quantile at tail 2^-t for a count with small mean num / 256^j.

    Returns 0 when the mean is far below the tail, and None when the mean
    is too large for this bound, so that the caller falls back to
    `bernstein`.
    """
    if num == 0:
        return 0
    den_bits, num_bits = 8 * j + 1, num.bit_length()
    if num_bits + t + 2 < den_bits:
        return 0
    b = max(den_bits - num_bits, 0)
    return t // (b - 3) + 2 if b >= 5 else None


def occupied(n, j):
    """The deterministic bound on occupied depth-j prefixes: min(256^j, n)."""
    return int(n >= 1) if j == 0 else min(pow256(j), n)


def disputed(n, pair, j):
    """Depth-j nodes in dispute, where `pair` is the product of the set sizes.

    A node is in dispute only if both sides hold a message under it, so the
    count is bounded by the pairs sharing a j-byte prefix, by the smaller
    set, and by the number of depth-j nodes.
    """
    if j == 0:
        return int(pair >= 1)
    smaller = pair // n if n else 0
    q = small_mean_quantile(pair, j, TAIL_BITS)
    if q is None:
        q = bernstein(pair // pow256(j) + 1, tail_exponent(0))
    return min(pow256(j), smaller, q)


def leaves_quantile(n, j):
    """`L(j)`: the leaves under any single depth-j node, at the model's tail."""
    if j > 0:
        q = small_mean_quantile(n, j, TAIL_BITS + 8 * j)
        if q is not None:
            return q
    return bernstein((n // pow256(j) if j > 0 else n) + 1, tail_exponent(j))


def children_quantile(n, j):
    """`C(j)`: the occupied children of any single depth-j node, at the model's tail."""
    slots = min(pow256(j) * FAN, (1 << 128) - 1)
    mean_hi = min(FAN, 2 * n * FAN // (2 * slots + n) + 1)
    return min(FAN, leaves_quantile(n, j), min(FAN, bernstein(mean_hi, tail_exponent(j))))


def population(n, pair, d):
    """`S(d)`: the scopes (questions) that can be in dispute at depth d.

    Disputed parents times their children, capped by occupancy.
    """
    if d == 0 or n == 0:
        return 0
    if d == 1:
        return 1
    return min(occupied(n, d - 1), disputed(n, pair, d - 2) * children_quantile(n, d - 2))


# --- This note's additions ----------------------------------------------

def set_leaves_quantile(n, q, j):
    """Leaves under any q distinct depth-j nodes, at the model's tail."""
    if q >= pow256(j):
        return n
    if q == 1:
        return leaves_quantile(n, j)
    t = TAIL_BITS + q * (8 * j + 2 - (q.bit_length() - 1))
    quantile = small_mean_quantile(n * q, j, t)
    if quantile is None:
        quantile = bernstein(n * q // pow256(j) + 1, -(-(7 * t) // 10))
    return min(n, quantile)


def d_hi(k):
    """Smallest D with binom(256, k) * (k/256)^D <= 2^-48, exactly; None at 256.

    The root comparison's bound on differing messages (exposition 6.8).
    """
    if k == 0:
        return 0
    if k >= 256:
        return None
    ways = math.comb(256, k)
    d = 1
    while ways * k ** d * 2 ** TAIL_BITS > 256 ** d:
        d += 1
    return d


def charges(n_a, n_b, exchange=True, k=256, peer_k=None, node_bytes=8):
    """Return (max population, charge before, scope-only charge, charge after).

    Each charge is a function of the window K:
    - "before" is today's scope charge;
    - "scope-only" is the set-priced scope charge (exposition 6.5);
    - "after" adds parking (6.3).
    With `exchange`, parked listings are priced against the peer's window
    (6.6): the queries in replies parked at level d are the peer's
    questions still outstanding at level d + 1, so their listings number at
    most (K_peer(d + 1) + 257) * min(256, C(d)). `peer_k` is the peer's
    window bound; None prices against a peer holding the same K.
    `node_bytes` is the backend's price for one node handle.
    `k < 256` applies the dropped root-comparison bound (6.8).
    """
    n, pair, bound = max(n_a, n_b), n_a * n_b, d_hi(k)
    pop = [population(n, pair, d) for d in range(KEY + 1)]
    if bound is not None:
        pop = [p if d < 2 else min(p, k if d == 2 else bound) for d, p in enumerate(pop)]
    width = lambda K, d: max(1, min(K, pop[d]))

    def scope_before(d, K):
        return min(pop[d], K) * (children_quantile(n, d - 1) * REFERENCE_BYTES + SCOPE_FIXED_BYTES)

    def scope_after(d, K):
        q = min(pop[d], K)
        if q == 0:
            return 0
        refs = min(q * children_quantile(n, d - 1), set_leaves_quantile(n, q, d - 1), occupied(n, d))
        return refs * (REFERENCE_BYTES - 8 + node_bytes) + q * SCOPE_FIXED_BYTES

    def park(d, K):
        slots = min(width(K, d) + FAN + 1, pow256(d - 1))
        if bound is not None and d >= 2:
            slots = min(slots, k if d == 2 else bound)
        agg = set_leaves_quantile(n, slots, d - 1)
        c0, c1 = children_quantile(n, d - 1), children_quantile(n, d)
        reactions = min(slots * min(FAN, 2 * c0), 2 * occupied(n, d), 2 * agg)
        entries = min(slots * c0 * c1, occupied(n, d + 1), agg)
        if bound is not None:
            entries = min(entries, bound * c1)
        if exchange and d < KEY:
            peer = K if peer_k is None else peer_k
            entries = min(entries, (width(peer, d + 1) + FAN + 1) * min(FAN, c1))
        return reactions * REACTION_BYTES + entries * ENTRY_BYTES

    leaf = lambda K: min(pop[KEY], K) * LEAF_REQUEST_BYTES
    before = lambda K: SUPPLY_FANS + sum(scope_before(d, K) for d in range(1, KEY + 1)) + leaf(K)
    scopes = lambda K: SUPPLY_FANS + sum(scope_after(d, K) for d in range(1, KEY + 1)) + leaf(K)
    after = lambda K: scopes(K) + sum(park(d, K) for d in range(1, KEY + 1))
    after.level2 = lambda K: park(2, K)
    return max(pop), before, scopes, after


def design_window(n, budget, k=256):
    """The design's window for one side, both peers holding `budget` or more.

    `K_max` is the widest window the smaller budget affords at a node price
    of zero, so no peer on any backend holds a wider one. The side's own
    window is the widest the budget affords at its real node price, with
    parked listings priced against a peer holding `K_max` (exposition 6.6).
    """
    top, _, _, zero = charges(n, n, k=k, node_bytes=0)
    k_max = granted(zero, top, budget)
    _, _, _, own = charges(n, n, k=k, peer_k=k_max)
    return min(granted(own, top, budget), k_max)


def design_threshold(n, target, k=256):
    """The least budget whose design window reaches `target`, in bytes."""
    lo, hi = 1, 1 << 40
    while hi - lo > 1 << 16:
        mid = (lo + hi) // 2
        lo, hi = (lo, mid) if design_window(n, mid, k) >= target else (mid, hi)
    return hi


def granted(charge, top, budget):
    """The window: the largest K in [1, top] whose charge fits the budget."""
    lo, hi = 1, max(top, 1)
    while lo < hi:
        mid = lo + (hi - lo + 1) // 2
        lo, hi = (mid, hi) if charge(mid) <= budget else (lo, mid - 1)
    return lo


MIB, GIB = 2 ** 20, 2 ** 30
# Reference links by bandwidth-delay product in bytes: in-rack (100 Gb/s,
# 50 us), metro (10 Gb/s, 2 ms), long haul (1 Gb/s, 100 ms). A 100-byte
# message costs 43 bytes of protocol overhead on the wire, so the long-haul
# link holds W = 12,500,000 // 143 = 87,412 messages.
LINKS = [("in-rack", 625_000), ("metro", 2_500_000), ("long haul", 12_500_000)]
MESSAGE_WIRE_BYTES = 43 + 100
LONG_HAUL_W = 12_500_000 // MESSAGE_WIRE_BYTES
SIZES = (10**5, 10**6, 10**7, 10**8)


def level_walk(n=10**7, extra=200_000):
    """Exposition 6.4: the worked example, level by level."""
    children = lambda leaves: 256 * (1 - math.exp(-leaves / 256))
    differs = lambda depth: 1 - math.exp(-extra / 256 ** depth)
    print(f"\n[6.4] Level walk, n={n:.0e}, {extra} extra messages on one side:")
    for j in range(4):
        print(f"  depth {j}: {n / 256 ** j:,.1f} leaves, {children(n / 256 ** j):.0f} children"
              + (f", differs with probability {differs(j):.3f}" if j else ""))
    level1 = 256 * children(n / 256)
    print(f"  level 1: one reply, {level1:,.0f} entries, {level1 * ENTRY_BYTES / 1e6:.1f} MB")
    queries = 256 * differs(2)
    entries = queries * children(n / 256 ** 2)
    print(f"  level 2: 256 replies of {queries:.0f} queries and {entries:,.0f} entries"
          f" ({entries * ENTRY_BYTES / 1e3:.0f} KB); all {256 * entries / 1e6:.1f}M entries,"
          f" {256 * entries * ENTRY_BYTES / 1e6:.0f} MB")
    level3 = 65536 * differs(2)
    print(f"  level 3: {level3:,.0f} questions; replies of"
          f" {children(n / 256 ** 2):.0f} reactions, {children(n / 256 ** 2) * differs(3):.1f} queries")
    top, _, _, zero = charges(n, n, node_bytes=0)
    k_max = granted(zero, top, 512 * MIB)
    window = design_window(n, 512 * MIB)
    _, _, _, own = charges(n, n, peer_k=k_max)
    ahead = k_max / queries
    print(f"  default budget: window {window:,}, peer bound K_max {k_max:,}; at most about {ahead:.0f}"
          f" level-2 replies run ahead ({ahead * entries * ENTRY_BYTES / 1e6:.0f} MB); level-2 parking"
          f" priced at {own.level2(window) / MIB:.0f} MiB")
    print(f"  budget for a window holding all {level3:,.0f} level-3 questions:"
          f" {design_threshold(n, int(level3)) / GIB:.2f} GiB")
    for size in (10**6, 10**7, 10**8):
        prefixes = 2 ** 24 * (1 - math.exp(-size / 2 ** 24))
        print(f"  level-2 volume at n={size:.0e}: {prefixes / size:.2f} n prefixes,"
              f" {prefixes * ENTRY_BYTES / 1e6:.0f} MB")


def exchange_effect():
    """Exposition 6.6: what exchanging budgets buys over a private window."""
    print("\n[6.6] Exchange: private window -> design window (entirely different replicas, one budget)")
    for n in SIZES:
        top, _, _, private = charges(n, n, exchange=False)
        _, _, _, floor = charges(n, n, peer_k=1)
        row = [f"{b} MiB {granted(private, top, b * MIB)} -> {design_window(n, b * MIB)}"
               for b in (16, 64, 512)]
        K = min(top, LONG_HAUL_W)
        print(f"  n={n:.0e}: " + " | ".join(row)
              + f" | floor {private(1)/MIB:.1f} -> {floor(1)/MIB:.1f} MiB"
              + f" | long-haul threshold {private(K)/MIB:.0f} -> {design_threshold(n, K)/MIB:.0f} MiB")
    print("[6.6] Mismatched budgets: private (narrow, wide) -> design (both)")
    for n, small, big in ((10**6, 64, 512), (10**7, 64, 512), (10**7, 128, 2048)):
        top, _, _, private = charges(n, n, exchange=False)
        print(f"  n={n:.0e} {small} and {big} MiB: {granted(private, top, small * MIB)} and"
              f" {granted(private, top, big * MIB)} -> {design_window(n, small * MIB)}")
    print("[6.6] Cost of pricing against K_max rather than one's own window")
    for n in (10**6, 10**7):
        for b in (64, 128, 512):
            top, _, _, same = charges(n, n)
            k_same, k_design = granted(same, top, b * MIB), design_window(n, b * MIB)
            print(f"  n={n:.0e} {b} MiB: {k_same} -> {k_design} ({100 * (k_design / k_same - 1):+.1f}%)")


def thresholds():
    """Exposition 6.7: threshold budgets and windows, today -> the design."""
    print("\n[6.7] Threshold budgets (MiB), today -> the design:")
    for n in SIZES:
        top, before, _, _ = charges(n, n)
        cells = []
        for name, bdp in LINKS:
            K = min(top, bdp // MESSAGE_WIRE_BYTES)
            cells.append(f"{name}: {before(K)/MIB:.0f} -> {design_threshold(n, K)/MIB:.0f}")
        print(f"  n={n:.0e}  " + " | ".join(cells))
    print("[6.7] Windows granted, today | private | design:")
    for n in SIZES:
        top, before, _, _ = charges(n, n)
        _, _, _, private = charges(n, n, exchange=False)
        print(f"  n={n:.0e}: " + " | ".join(
            f"{b} MiB {granted(before, top, b * MIB)} | {granted(private, top, b * MIB)}"
            f" | {design_window(n, b * MIB)}" for b in (64, 128, 512)))
    top, _, scopes, _ = charges(10**6, 10**6)
    K = min(top, LONG_HAUL_W)
    t = design_threshold(10**6, K)
    print(f"[6.7] Reference point: design threshold {t/MIB:.0f} MiB (queued scopes {scopes(K)/MIB:.0f} MiB"
          f" of it); design window at 512 MiB {design_window(10**6, 512 * MIB):,}")


def root_comparison():
    """Exposition 6.8: the root-comparison bound, and why the design drops it."""
    print("\n[6.8] D_hi(k):", {k: d_hi(k) for k in (1, 8, 32, 83, 128, 177, 224, 251, 255)})
    for D in (1400, 2000):
        print(f"  P(all 256 root slots differ) at D={D}: {(1 - math.exp(-D / 256)) ** 256:.2f}")
    print("[6.8] Window: private -> private + root comparison -> design -> design + root comparison")
    for n in (10**6, 10**7, 10**8):
        for D in (100, 1000):
            k = round(256 * (1 - math.exp(-D / 256)))
            top, _, _, private = charges(n, n, exchange=False)
            top_k, _, _, private_k = charges(n, n, exchange=False, k=k)
            cells = [f"{b} MiB {granted(private, top, b * MIB)} -> {granted(private_k, top_k, b * MIB)}"
                     f" -> {design_window(n, b * MIB)} -> {design_window(n, b * MIB, k)}"
                     for b in (16, 64)]
            print(f"  n={n:.0e} D={D} (k={k}): " + " | ".join(cells))


def credits():
    """Exposition 9.2: latency-free budgets, parking against credits."""
    print("\n[9.2] Long haul, latency-free budgets (MiB): parking (design) | credits, 17 x BDP"
          " | credits, band-targeted")
    band_credits = 2 * 12_500_000 + 15 * FRAME_BYTES
    for n in SIZES:
        top, _, scopes, _ = charges(n, n)
        K = min(top, LONG_HAUL_W)
        print(f"  n={n:.0e}: {design_threshold(n, K)/MIB:.0f} | {(scopes(K)+17*12_500_000)/MIB:.0f}"
              f" | {(scopes(K)+band_credits)/MIB:.0f}")


def band_figures(n=10**6):
    """Exposition 7.2: where supplies fall, by the other side's prefix occupancy."""
    occupied_at = lambda j: 1 - math.exp(-n / 256 ** j)
    print(f"\n[7.2] Band at n={n:.0e}: occupancy by depth "
          + ", ".join(f"{j}: {occupied_at(j):.4f}" for j in (2, 3, 4))
          + f"; supplies at depth 3: {1 - occupied_at(3):.0%}")


def exact_deep_fans():
    """Exposition 10.1: fans of exactly 1 past the leaf depth, under the design."""
    global children_quantile, leaves_quantile
    base_c, base_l = children_quantile, leaves_quantile
    print("\n[10.1] Exact deep fans past the leaf depth h: floor, threshold, and window at 64 MiB")
    for n in (10**5, 10**6, 10**7):
        h = math.floor(math.log(n * n / 2, 256)) + 2
        rows = []
        for exact in (False, True):
            if exact:
                children_quantile = lambda nn, j: 1 if j >= h else base_c(nn, j)
                leaves_quantile = lambda nn, j: 1 if j >= h else base_l(nn, j)
            top, _, _, floor = charges(n, n, peer_k=1)
            rows.append((floor(1), design_threshold(n, min(top, LONG_HAUL_W)), design_window(n, 64 * MIB)))
            children_quantile, leaves_quantile = base_c, base_l
        (f0, t0, w0), (f1, t1, w1) = rows
        print(f"  n={n:.0e} h={h}: floor {f0/MIB:.1f} -> {f1/MIB:.1f} MiB ({1 - f1/f0:.1%} removed),"
              f" threshold {t0/MIB:.0f} -> {t1/MIB:.0f} MiB, window {w0} -> {w1}")


def duplex():
    """Exposition 10.2: reverse-wire idle time, and the duplex term."""
    print("\n[10.2] Duplex coupling: reverse-wire idle time, % of the larger transfer (mean, worst)")
    for N in (100, 1000, 10000):
        row = []
        for ratio in (0.5, 1.0, 2.0):
            runs = []
            for trial in range(100 if N >= 10000 else 300):
                rng = random.Random(trial)
                items = [("S", 1 + int(rng.expovariate(1))) for _ in range(N)]
                items += [("Q", 1 + int(rng.expovariate(1))) for _ in range(int(N * ratio))]
                rng.shuffle(items)
                forward = sum(s for kind, s in items if kind == "S")
                reverse = sum(s for kind, s in items if kind == "Q")
                clock = free = 0.0
                for kind, size in items:
                    if kind == "S":
                        clock += size
                    else:
                        free = max(free, clock) + size
                runs.append((max(clock, free) - max(forward, reverse)) / max(forward, reverse))
            row.append(f"{100*statistics.mean(runs):.2f}% ({100*max(runs):.2f}%)")
        print(f"  N={N}: " + " | ".join(row))
    wire = 125e6  # 1 Gb/s in bytes per second
    for N, m in ((100, 1e6), (200_000, 143)):
        print(f"  duplex term, {N} supplies of {m:.0f} B: {0.7 * math.sqrt(N) * m / wire * 1e3:.2f} ms"
              f" of {N * m / wire:.2f} s")


def main():
    """Print each group of figures the note cites, tagged by exposition section."""
    level_walk()
    exchange_effect()
    thresholds()
    root_comparison()
    band_figures()
    credits()
    exact_deep_fans()
    duplex()


if __name__ == "__main__":
    main()
