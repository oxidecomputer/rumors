"""Brute-force check of LP duality for the signed brick norm at depth 3.

Dual: maximize sum f(x) g(x) over integer charges g on the finest cells such
that every dyadic interval (cells included) holds net charge in [-1, 1].
"""
import itertools, random
from norm import norm

D = 3
n = 1 << D
intervals = [(d, i) for d in range(D + 1) for i in range(1 << d)]
def feasible(g):
    for d, i in intervals:
        span = n >> d
        if abs(sum(g[i * span:(i + 1) * span])) > 1:
            return False
    return True
charges = [g for g in itertools.product((-1, 0, 1), repeat=n) if feasible(g)]
rng = random.Random(11)
for trial in range(400):
    f = [rng.randrange(-2, 3) for _ in range(n)]
    best = max(sum(a * b for a, b in zip(f, g)) for g in charges)
    assert best == norm(f), (f, best, norm(f))
print(f"depth {D}: norm == best charge certificate for 400 random signed functions ({len(charges)} feasible charges)")
