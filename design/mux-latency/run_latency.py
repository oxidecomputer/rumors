"""Latency validation sweep for formal/MUX-LATENCY.md.

Runs every construction (baseline / stream-W1 / sigma*-causal /
sigma*-omniscient / oracle) over the campaign's standard shapes plus the
latency-adversarial families, under the RTT-cost semantics of timed.py,
and writes latency_results.json. Deterministic. AxMode = .impl throughout;
every skeleton is margin-0 (dCount <= capLevel), so the unmuxed system is
inside `Sched.deadlock_free` and any timing difference is attributable to
the transport construction.

Regimes: wide=True (internal channels at the shipped pipeline window, the
regime where the baseline is Theta(depth) and the transport delta is fully
exposed) is the headline; wide=False (the model of record's cap-1 floor)
is reported for the standard shapes as the secondary column.
"""

import json

from model import Skel, IMPL, I, R
import gen
import instances as X
from timed import timed_run, consumption_order, hops

BIG = 10 ** 9


# ------------------------------------------------------------------ shapes

def chain(rootH=6):
    """Pure dispute chain: one scope per height, leaf request at the foot.
    Every frame is seq <= 1 on its stream: the shape where sigma* should
    exactly match independent links."""
    scopes = []
    for h in range(rootH, 1, -1):
        scopes.append(['D', h, [len(scopes) + 1], 0])
    scopes.append(['D', 1, [], 1])
    sk = Skel(scopes, rootH, 2, 1)
    assert sk.wellFormed() and sk.margin0()
    return sk


def comb_wide(m, rootH=6):
    """The fresh-dispute maximizer: one level of m D scopes, each with one
    childless-D child, so every one of the m frames at the frontier level
    needs peer-minted labels two scopes back."""
    scopes = [['D', rootH, [1], 0], ['D', rootH - 1, [], 0]]
    heads = []
    for _ in range(m):
        nid = len(scopes)
        scopes.append(['D', rootH - 2, [], 0])
        scopes[1][2].append(nid)
        heads.append(nid)
    for hid in heads:
        nid = len(scopes)
        scopes.append(['D', rootH - 3, [], 0])
        scopes[hid][2].append(nid)
    sk0 = Skel(scopes, rootH, max(2, m), 1)
    sk = Skel(scopes, rootH, max(2, m), max(1, sk0.maxDCount()))
    assert sk.wellFormed() and sk.margin0()
    return sk


def pyramid_dense(f=2, rootH=6):
    """All-D f-ary tree, childless at height 2: fresh disputes at every
    level, the compounding-across-levels probe."""
    scopes = [['D', rootH, [], 0]]
    level = [0]
    for h in range(rootH - 1, 1, -1):
        nxt = []
        for sid in level:
            for _ in range(f):
                nid = len(scopes)
                scopes.append(['D', h, [], 0])
                scopes[sid][2].append(nid)
                nxt.append(nid)
        level = nxt
    sk0 = Skel(scopes, rootH, f, 1)
    sk = Skel(scopes, rootH, f, max(1, sk0.maxDCount()))
    assert sk.wellFormed() and sk.margin0()
    return sk


def shapes():
    return [
        ('chain6', chain(6)),
        ('wedge', gen.regression_shape(provisions=6, rootH=6)),
        ('provwall8', gen.prov_family(8, rootH=4)),
        ('dfan8', gen.dfan_family(8)),
        ('comb6', X.comb6),
        ('combW8', comb_wide(8)),
        ('combW16', comb_wide(16)),
        ('pyr2', pyramid_dense(2, 6)),
        ('pyr3', pyramid_dense(3, 6)),
    ]


# ------------------------------------------------------------------- sweep

def row(name, sk, wide):
    ax = IMPL
    pi = consumption_order(sk, ax)
    r = {
        'shape': name,
        'scopes': len(sk.scopes),
        'wide': wide,
        'base': hops(sk, ax, 'eager', BIG, 'stream', wide),
        'w1': hops(sk, ax, 'eager', 1, 'stream', wide),
        'sigC1': hops(sk, ax, 'sigma_causal', 1, 'direction', wide),
        'sigC4': hops(sk, ax, 'sigma_causal', 4, 'direction', wide),
        'sigCinf': hops(sk, ax, 'sigma_causal', BIG, 'direction', wide),
        'omniC1': hops(sk, ax, 'sigma_omni', 1, 'direction', wide),
        'omniCinf': hops(sk, ax, 'sigma_omni', BIG, 'direction', wide),
        'oraC1': hops(sk, ax, 'oracle', 1, 'direction', wide,
                      oracle_order=pi),
        'oraCinf': hops(sk, ax, 'oracle', BIG, 'direction', wide,
                        oracle_order=pi),
    }
    return r


def fmt(r):
    def f(v):
        return str(v)
    return (f"{r['shape']:10s} N={r['scopes']:3d} wide={int(r['wide'])} "
            f"base={f(r['base']):>4} w1={f(r['w1']):>4} "
            f"sig(C1/C4/Cinf)={f(r['sigC1']):>4}/{f(r['sigC4']):>4}/"
            f"{f(r['sigCinf']):>4} omni(C1/Cinf)={f(r['omniC1']):>4}/"
            f"{f(r['omniCinf']):>4} ora(C1/Cinf)={f(r['oraC1']):>4}/"
            f"{f(r['oraCinf']):>4}")


def calibrate():
    """Gates: the chain must complete at rootH+2 hops for EVERY
    construction (the ladder is the whole critical path), and every
    standard-shape run must be terminal."""
    sk = chain(6)
    ax = IMPL
    pi = consumption_order(sk, ax)
    want = sk.rootH + 2
    got = {
        'base': hops(sk, ax, 'eager', BIG, 'stream', True),
        'w1': hops(sk, ax, 'eager', 1, 'stream', True),
        'sigC1': hops(sk, ax, 'sigma_causal', 1, 'direction', True),
        'oraC1': hops(sk, ax, 'oracle', 1, 'direction', True,
                      oracle_order=pi),
    }
    ok = all(v == want for v in got.values())
    print(f'calibration chain6: want {want} hops everywhere -> {got} '
          f'{"PASS" if ok else "FAIL"}')
    return ok


def random_expected(n=40, seed0=1000):
    """Expected-case multipliers over the campaign's own random-skeleton
    distribution (gen.gen_random: rootH in {4,4,6}, fan U[2,7], interior
    child D w.p. 0.55, height-1 child D w.p. 0.45, kid counts U[0,fan])."""
    ax = IMPL
    out = []
    for k in range(n):
        sk = gen.gen_random(seed0 + k)
        base = hops(sk, ax, 'eager', BIG, 'stream', True)
        s1 = hops(sk, ax, 'sigma_causal', 1, 'direction', True)
        sinf = hops(sk, ax, 'sigma_causal', BIG, 'direction', True)
        if isinstance(base, tuple) or isinstance(s1, tuple) \
                or isinstance(sinf, tuple):
            out.append(dict(seed=seed0 + k, error=str((base, s1, sinf))))
            continue
        out.append(dict(seed=seed0 + k, scopes=len(sk.scopes), base=base,
                        sigC1=s1, sigCinf=sinf,
                        m1=s1 / base, minf=sinf / base))
    ms1 = [r['m1'] for r in out if 'm1' in r]
    msinf = [r['minf'] for r in out if 'minf' in r]
    summ = dict(n=len(ms1),
                sigC1_mean=sum(ms1) / len(ms1), sigC1_max=max(ms1),
                sigCinf_mean=sum(msinf) / len(msinf),
                sigCinf_max=max(msinf))
    print(f'random pool (n={len(ms1)}): sigma*/base multipliers '
          f'C=1 mean {summ["sigC1_mean"]:.2f} max {summ["sigC1_max"]:.2f}; '
          f'C=inf mean {summ["sigCinf_mean"]:.2f} '
          f'max {summ["sigCinf_max"]:.2f}')
    return out, summ


# ------------------------------------------------------- sigma*_K (parking)

def paced_star(sk):
    """P* = max over streams of the K=1 paced-frame count: frames k >= 3
    whose scope k-2 at the stream's height is child-bearing."""
    best = 0
    for h in range(sk.rootH):
        ss = sk.scopesAt(h)
        if not ss:
            continue
        p = sum(1 for k in range(3, len(ss) + 1)
                if sk.nChildren(h - 1, ss[k - 3]) > 0)
        best = max(best, p)
    return best


def k_law(base, pstar, K):
    """MUX-LATENCY.md section 7 law: width term = 2*ceil((P*-K+1)/(K+1)),
    clamped at 0 (exact on single-frontier combs; +-eps elsewhere)."""
    import math
    return base + 2 * max(0, math.ceil((pstar - K + 1) / (K + 1)))


def k_sweep(Ks=(1, 2, 4, 8, 16, 32)):
    """sigma*_K validation: causal sigma* with parking depth K, C = inf,
    wide regime, against the section 7 law. K=1 must reproduce the sigma*
    rows; K >= n*-1 must reproduce the baseline."""
    ax = IMPL
    rows = []
    for name, sk in shapes():
        base = hops(sk, ax, 'eager', BIG, 'stream', True)
        ps = paced_star(sk)
        cells = {}
        for K in Ks:
            got = hops(sk, ax, 'sigma_causal', BIG, 'direction', True,
                       park=K)
            cells[K] = dict(probe=got, law=k_law(base, ps, K))
        rows.append(dict(shape=name, scopes=len(sk.scopes), base=base,
                         pstar=ps, cells=cells))
        cell_s = ' '.join(f'K{K}={v["probe"]}(law {v["law"]})'
                          for K, v in cells.items())
        print(f'  ksweep {name:10s} base={base} P*={ps:2d}  {cell_s}')
    return rows


def main():
    results = {'rows': [], 'random': None}
    if not calibrate():
        print('CALIBRATION FAILED')
        results['calibration'] = 'FAIL'
    else:
        results['calibration'] = 'PASS'
    for wide in (True, False):
        for name, sk in shapes():
            r = row(name, sk, wide)
            print(fmt(r))
            results['rows'].append(r)
    pool, summ = random_expected()
    results['random'] = dict(pool=pool, summary=summ)
    print('== sigma*_K parking sweep ==')
    results['ksweep'] = k_sweep()
    with open('latency_results.json', 'w') as f:
        json.dump(results, f, indent=1, default=str)
    print('wrote latency_results.json')


if __name__ == '__main__':
    main()
