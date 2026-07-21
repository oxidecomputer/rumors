"""Skeleton generators: random well-formed schedulable skeletons (BFS ids
by construction) and the parametric adversarial families, including the
Rust regression shape (root fan >= 7, first radix child deep-disputed
depth >= 2, >= 6 whole-subtree provisions behind it)."""

import random
from model import Skel


def gen_random(seed, rootH=None, fan=None, margin0=True):
    """Random well-formed skeleton, BFS ids by level-order construction.
    capLevel = max dCount (margin 0) when margin0, else capped at
    schedulable (dCount <= capLevel + 2)."""
    rng = random.Random(seed)
    rootH = rootH or rng.choice([4, 4, 6])
    fan = fan or rng.randint(2, 7)
    scopes = []          # (kind, height, kids(list to fill), leafReqs)
    scopes.append(['D', rootH, [], 0])
    level = [0]          # ids at current height
    h = rootH
    while h >= 2:
        nxt = []
        for sid in level:
            kind = scopes[sid][0]
            if kind != 'D':
                continue
            if h == rootH:
                nk = rng.randint(1, fan)
            else:
                nk = rng.randint(0, fan)
            for _ in range(nk):
                if h - 1 == 1:
                    ck = 'D' if rng.random() < 0.45 else 'R'
                    lr = rng.randint(0, fan) if ck == 'D' else 0
                    scopes.append([ck, 1, [], lr])
                else:
                    ck = 'D' if rng.random() < 0.55 else 'R'
                    scopes.append([ck, h - 1, [], 0])
                cid = len(scopes) - 1
                scopes[sid][2].append(cid)
                nxt.append(cid)
        level = nxt
        h -= 1
    sk0 = Skel(scopes, rootH, fan, 1)
    md = max(1, sk0.maxDCount())
    capLevel = md if margin0 else max(1, md - 2)
    sk = Skel(scopes, rootH, fan, capLevel)
    assert sk.wellFormed(), f'generator bug: seed {seed} not wellFormed'
    assert sk.schedulable(), f'generator bug: seed {seed} not schedulable'
    if margin0:
        assert sk.margin0()
    return sk


def regression_shape(provisions=6, depth=2, rootH=6, capLevel=None):
    """The Rust wide-tree regression shape in skeleton terms: the root
    disputes its FIRST child (a chain descending `depth` more disputed
    levels, ending in a leaf request) and takes `provisions` whole-subtree
    R children behind it. Root fan = provisions + 1 (>= 7 at the Rust
    trigger)."""
    assert depth >= 1 and rootH - 1 - depth >= 0
    scopes = [['D', rootH, [], 0]]
    # level rootH-1: first the D chain head, then the provisions
    chain = len(scopes)
    scopes.append(['D', rootH - 1, [], 0])
    scopes[0][2].append(chain)
    for _ in range(provisions):
        rid = len(scopes)
        scopes.append(['R', rootH - 1, [], 0])
        scopes[0][2].append(rid)
    # descend the chain: each level one D child
    cur = chain
    h = rootH - 1
    while h > 1:
        nid = len(scopes)
        if h - 1 == 1:
            scopes.append(['D', 1, [], 1])
        else:
            scopes.append(['D', h - 1, [], 0])
        scopes[cur][2].append(nid)
        cur = nid
        h -= 1
    fan = provisions + 1
    sk0 = Skel(scopes, rootH, fan, 1)
    cl = capLevel or max(1, sk0.maxDCount())
    sk = Skel(scopes, rootH, fan, cl)
    assert sk.wellFormed() and sk.schedulable()
    return sk


def prov_family(width, rootH=4, capLevel=None):
    """Width-parametric provisions-behind-dispute family: root disputes
    child A (which descends one more level to a leaf-request scope) and
    takes `width` R siblings after it. Margin-0 capLevel by default."""
    scopes = [['D', rootH, [], 0]]
    aid = len(scopes)
    scopes.append(['D', rootH - 1, [], 0])
    scopes[0][2].append(aid)
    rids = []
    for _ in range(width):
        rid = len(scopes)
        scopes.append(['R', rootH - 1, [], 0])
        scopes[0][2].append(rid)
        rids.append(rid)
    cur = aid
    h = rootH - 1
    while h > 1:
        nid = len(scopes)
        if h - 1 == 1:
            scopes.append(['D', 1, [], 1])
        else:
            scopes.append(['D', h - 1, [], 0])
        scopes[cur][2].append(nid)
        cur = nid
        h -= 1
    fan = width + 1
    sk0 = Skel(scopes, rootH, fan, 1)
    cl = capLevel or max(1, sk0.maxDCount())
    sk = Skel(scopes, rootH, fan, cl)
    assert sk.wellFormed() and sk.schedulable()
    return sk


def dfan_family(width, rootH=4):
    """Width-parametric D-fan family at margin 0: root child A disputes
    `width` childless D scopes (capLevel = width, the shipping FAN >= kids
    stance), so the unmuxed .impl system is inside the flagship theorem."""
    scopes = [['D', rootH, [], 0]]
    aid = len(scopes)
    scopes.append(['D', rootH - 1, [], 0])
    scopes[0][2].append(aid)
    for _ in range(width):
        nid = len(scopes)
        scopes.append(['D', rootH - 2, [], 0])
        scopes[aid][2].append(nid)
    fan = max(1, width)
    sk = Skel(scopes, rootH, fan, max(1, width))
    assert sk.wellFormed() and sk.schedulable() and sk.margin0()
    return sk
