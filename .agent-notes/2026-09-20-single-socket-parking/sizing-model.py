#!/usr/bin/env python3
"""Reproduce the note's modeled figures.

A transcription of the window model in `src/tree/mirror/streaming/window.rs`
(the integer quantiles `C`, `L`, `S`, `occupied`, and the charge), extended
with this note's parking price, set statistic, and root-comparison bound.
Slot sizes are estimates for the in-memory backend; the implementation
derives them with `size_of`. Run: `python3 sizing-model.py`.
"""

import math
import random
import statistics

FAN, KEY, TAIL_BITS = 256, 32, 48

# Estimated byte sizes (in-memory backend).
REFERENCE_BYTES = 8 + 65      # node handle + query/resolution/listing slots
SCOPE_FIXED_BYTES = 128       # inline Query + Resolution
LEAF_REQUEST_BYTES = 40
SUPPLY_FANS = 17 * 257 * 48   # decode fans, one per reply stream
REACTION_BYTES = 32           # one parked reaction
ENTRY_BYTES = 25              # one (radix, digest) listing entry
FRAME_BYTES = 1_830_000       # default supply-run budget, about one frame


# --- The window model's quantiles, transcribed ---------------------------

def pow256(j):
    return (1 << 128) - 1 if j >= 16 else 1 << (8 * j)


def tail_exponent(j):
    return -(-(7 * (TAIL_BITS + 8 * j)) // 10)


def bernstein(mean_hi, t):
    return mean_hi + math.isqrt(2 * mean_hi * t) + t


def small_mean_quantile(num, j, t):
    if num == 0:
        return 0
    den_bits, num_bits = 8 * j + 1, num.bit_length()
    if num_bits + t + 2 < den_bits:
        return 0
    b = max(den_bits - num_bits, 0)
    return t // (b - 3) + 2 if b >= 5 else None


def occupied(n, j):
    return int(n >= 1) if j == 0 else min(pow256(j), n)


def disputed(n, pair, j):
    if j == 0:
        return int(pair >= 1)
    smaller = pair // n if n else 0
    q = small_mean_quantile(pair, j, TAIL_BITS)
    if q is None:
        q = bernstein(pair // pow256(j) + 1, tail_exponent(0))
    return min(pow256(j), smaller, q)


def leaves_quantile(n, j):
    if j > 0:
        q = small_mean_quantile(n, j, TAIL_BITS + 8 * j)
        if q is not None:
            return q
    return bernstein((n // pow256(j) if j > 0 else n) + 1, tail_exponent(j))


def children_quantile(n, j):
    slots = min(pow256(j) * FAN, (1 << 128) - 1)
    mean_hi = min(FAN, 2 * n * FAN // (2 * slots + n) + 1)
    return min(FAN, leaves_quantile(n, j), min(FAN, bernstein(mean_hi, tail_exponent(j))))


def population(n, pair, d):
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
    """Smallest D with binom(256, k) * (k/256)^D <= 2^-48, exactly; None at 256."""
    if k == 0:
        return 0
    if k >= 256:
        return None
    ways = math.comb(256, k)
    d = 1
    while ways * k ** d * 2 ** TAIL_BITS > 256 ** d:
        d += 1
    return d


def charges(n_a, n_b, k=256, peer_window=None):
    """Return (max population, charge before, scope-only charge, charge after).

    `peer_window(level)` is the peer's question-queue capacity at a level,
    when the greeting carries it. The queries in replies parked at level d
    are peer questions still outstanding at level d + 1, so they number at
    most `peer_window(d + 1) + 257`, and so do their listings' owners.
    """
    n, pair, bound = max(n_a, n_b), n_a * n_b, d_hi(k)
    pop = [population(n, pair, d) for d in range(KEY + 1)]
    if bound is not None:
        pop = [p if d < 2 else min(p, k if d == 2 else bound) for d, p in enumerate(pop)]

    def scope_before(d, K):
        return min(pop[d], K) * (children_quantile(n, d - 1) * REFERENCE_BYTES + SCOPE_FIXED_BYTES)

    def scope_after(d, K):
        q = min(pop[d], K)
        if q == 0:
            return 0
        refs = min(q * children_quantile(n, d - 1), set_leaves_quantile(n, q, d - 1), occupied(n, d))
        return refs * REFERENCE_BYTES + q * SCOPE_FIXED_BYTES

    def park(d, K):
        slots = min(max(1, min(K, pop[d])) + FAN + 1, pow256(d - 1))
        if bound is not None and d >= 2:
            slots = min(slots, k if d == 2 else bound)
        agg = set_leaves_quantile(n, slots, d - 1)
        c0, c1 = children_quantile(n, d - 1), children_quantile(n, d)
        reactions = min(slots * min(FAN, 2 * c0), 2 * occupied(n, d), 2 * agg)
        entries = min(slots * c0 * c1, occupied(n, d + 1), agg)
        if bound is not None:
            entries = min(entries, bound * c1)
        if peer_window is not None and d < KEY:
            entries = min(entries, (peer_window(d + 1) + FAN + 1) * min(FAN, c1))
        return reactions * REACTION_BYTES + entries * ENTRY_BYTES

    leaf = lambda K: min(pop[KEY], K) * LEAF_REQUEST_BYTES
    before = lambda K: SUPPLY_FANS + sum(scope_before(d, K) for d in range(1, KEY + 1)) + leaf(K)
    scopes = lambda K: SUPPLY_FANS + sum(scope_after(d, K) for d in range(1, KEY + 1)) + leaf(K)
    after = lambda K: scopes(K) + sum(park(d, K) for d in range(1, KEY + 1))
    return max(pop), before, scopes, after


def shared_window(n, budget):
    """Both peers on one budget, each pricing parking by the other's window.

    Solves for the largest common K whose charge, priced against a peer
    holding the same K at every level, fits the budget.
    """
    pair = n * n
    pop = [population(n, pair, d) for d in range(KEY + 1)]
    lo, hi = 1, max(max(pop), 1)
    while lo < hi:
        mid = lo + (hi - lo + 1) // 2
        _, _, _, after = charges(n, n, peer_window=lambda l: max(1, min(mid, pop[l])))
        lo, hi = (mid, hi) if after(mid) <= budget else (lo, mid - 1)
    return lo, pop


def granted(charge, top, budget):
    lo, hi = 1, max(top, 1)
    while lo < hi:
        mid = lo + (hi - lo + 1) // 2
        lo, hi = (mid, hi) if charge(mid) <= budget else (lo, mid - 1)
    return lo


def level_walk(n=10**7, extra=200_000):
    """The worked example of exposition section 6.8, level by level."""
    children = lambda leaves: 256 * (1 - math.exp(-leaves / 256))
    differs = lambda depth: 1 - math.exp(-extra / 256 ** depth)
    print(f"\nLevel walk, n={n:.0e}, {extra} extra messages on one side:")
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
    print(f"  level 3: {65536 * differs(2):,.0f} questions; replies of"
          f" {children(n / 256 ** 2):.0f} reactions, {children(n / 256 ** 2) * differs(3):.1f} queries")
    top, _, _, after = charges(n, n)
    window = granted(after, top, 512 * 2 ** 20)
    ahead = window / queries
    print(f"  sender pacing at the default budget: level-3 window {window:,},"
          f" so about {ahead:.0f} level-2 replies ahead ({ahead * entries * ENTRY_BYTES / 1e6:.0f} MB)")
    for size in (10**6, 10**7, 10**8):
        prefixes = 2 ** 24 * (1 - math.exp(-size / 2 ** 24))
        print(f"  level-2 volume at n={size:.0e}: {prefixes / size:.2f} n prefixes,"
              f" {prefixes * ENTRY_BYTES / 1e6:.0f} MB")


MIB, GIB = 2 ** 20, 2 ** 30
LINKS = [("in-rack", 625_000), ("metro", 2_500_000), ("long haul", 12_500_000)]
MESSAGE_WIRE_BYTES = 43 + 100


def main():
    level_walk()
    print("\nD_hi(k):", {k: d_hi(k) for k in (1, 8, 32, 83, 128, 177, 224, 251, 255)})

    print("\nThreshold budgets (MiB), entirely different replicas, before -> after:")
    for n in (10**5, 10**6, 10**7, 10**8):
        top, before, _, after = charges(n, n)
        cells = []
        for name, bdp in LINKS:
            K = min(top, bdp // MESSAGE_WIRE_BYTES)
            cells.append(f"{name}: {before(K)/MIB:.0f} -> {after(K)/MIB:.0f}")
        print(f"  n={n:.0e}  " + " | ".join(cells))

    print("\nOrdinary sessions, n=1e6, long haul:")
    for D in (10, 100, 1000):
        k = round(256 * (1 - math.exp(-D / 256)))
        top, _, _, after = charges(10**6, 10**6, k)
        print(f"  D={D}: k={k}, D_hi={d_hi(k)}, threshold {after(min(top, 87412))/MIB:.1f} MiB")

    print("\nWindows granted:")
    for n in (10**5, 10**6, 10**7):
        top, before, _, after = charges(n, n)
        for budget in (64 * MIB, 512 * MIB):
            print(f"  n={n:.0e} {budget//MIB} MiB: {granted(before, top, budget)} -> {granted(after, top, budget)}")

    print("\nLong haul, latency-free budgets (MiB): parking | credits, 17 x BDP | credits, band-targeted")
    band_credits = 2 * 12_500_000 + 15 * FRAME_BYTES
    for n in (10**5, 10**6, 10**7, 10**8):
        top, _, scopes, after = charges(n, n)
        K = min(top, 87412)
        print(f"  n={n:.0e}: {after(K)/MIB:.0f} | {(scopes(K)+17*12_500_000)/MIB:.0f} | {(scopes(K)+band_credits)/MIB:.0f}")

    print("\nExchanged windows, both peers on one budget: window today -> priced by the peer's")
    for n in (10**5, 10**6, 10**7, 10**8):
        top, _, _, after = charges(n, n)
        row = []
        for budget in (16 * MIB, 64 * MIB, 512 * MIB):
            row.append(f"{budget // MIB} MiB {granted(after, top, budget)} -> {shared_window(n, budget)[0]}")
        pop = [population(n, n * n, d) for d in range(KEY + 1)]
        _, _, _, floor = charges(n, n, peer_window=lambda l: 1)
        K = min(top, 87412)
        _, _, _, wide = charges(n, n, peer_window=lambda l: max(1, min(K, pop[l])))
        print(f"  n={n:.0e}: " + " | ".join(row)
              + f" | floor {after(1)/MIB:.1f} -> {floor(1)/MIB:.1f} MiB"
              + f" | long-haul threshold {after(K)/MIB:.0f} -> {wide(K)/MIB:.0f} MiB")

    print("\nDuplex coupling: reverse-wire idle time, % of the larger transfer (mean, worst)")
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


if __name__ == "__main__":
    main()
