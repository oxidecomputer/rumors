"""Exhaustive checks of the signed brick norm on shallow dyadic trees.

A function is a list of 2**D integer heights on the finest cells. The norm is
the fewest signed unit bricks (dyadic interval indicators, added or
subtracted) summing to it, computed by dynamic programming over the tree:
cost(node, t) is the fewest bricks inside the node's subtree given that
ancestors already contribute t on it.
"""
import functools, itertools, random, sys

def norm(f):
    D = (len(f) - 1).bit_length()
    lo, hi = min(f) - 2, max(f) + 2
    @functools.lru_cache(None)
    def cost(depth, idx, t):
        if depth == D:
            return abs(f[idx] - t)
        return min(abs(b) + cost(depth + 1, 2 * idx, t + b) + cost(depth + 1, 2 * idx + 1, t + b)
                   for b in range(lo - t, hi - t + 1))
    return cost(0, 0, 0)

def positive_bricks(f):
    """Canonical (all-added) brick count: sum of node bases, for f >= 0."""
    D = (len(f) - 1).bit_length()
    def walk(depth, idx, parent_min):
        span = 1 << (D - depth)
        cells = f[idx * span:(idx + 1) * span]
        m = min(cells)
        here = m - parent_min
        if depth == D or len(set(cells)) == 1:
            return here
        return here + walk(depth + 1, 2 * idx, m) + walk(depth + 1, 2 * idx + 1, m)
    return walk(0, 0, 0)

def naf_weight(k):
    w = 0
    while k:
        if k & 1:
            k -= 2 - (k & 3)  # digit +1 if k = 1 mod 4, -1 if k = 3 mod 4
            w += 1
        k >>= 1
    return w

D = 6
n = 1 << D
# 1. Prefix indicators [0, k / 2^D): positive bricks are popcount, signed bricks the NAF weight.
for k in range(n + 1):
    f = [1] * k + [0] * (n - k)
    assert positive_bricks(f) == bin(k).count("1"), k
    assert norm(f) == naf_weight(k), (k, norm(f), naf_weight(k))
print(f"prefix indicators at depth {D}: positive = popcount, signed = NAF weight, for all {n + 1}")

# 2. Signed never exceeds positive, and the hole example.
rng = random.Random(7)
worse = 0
for _ in range(300):
    f = [rng.randrange(0, 3) for _ in range(n)]
    s, p = norm(f), positive_bricks(f)
    assert s <= p
    worse += s < p
print(f"random height-<=2 functions: signed < positive in {worse} of 300")
hole = [0] * (n >> 5) + [1] * (n - (n >> 5))
print("flat(1) less a depth-5 hole: positive", positive_bricks(hole), "signed", norm(hole))
