"""The CAUSAL (A_p-limited) sigma* -- stage 0 gate P1/P4 of the phase-3 plan.

Implements the formulation of record (MUX-ADJUDICATION.md sigma* = refute-c1
section 1-2 with the F1 keystone repair, F6 positional guards, F7 wording,
F8 close conjunct already in model.py):

  A party p's strategy is a pure function of
    - the announced sub-skeleton A_p (two minting rules, refute-c1 1.2), and
    - the observed trace: p's own pushes (flush-paced) + frames DELIVERED to
      p's demux slots (slot-peek; `peek=False` gives the P4 no-peek variant
      where observation = consumed frames only).
  It does NOT see: consumption receipts for its own pushes, the peer's
  component state, in-flight reverse-direction frames, its own pipe's drain.

  Demand rule: push frame k on stream c iff k == 1 or
  rcv(c, k-1) in Certified_p U Inevitable_p.

Implementation of the closures: Certified U Inevitable is computed as a
forward derivation (Kahn-confluent fixpoint) from init over the announced
sub-skeleton, under every action EXCEPT wire pushes beyond evidence:
  - p's own wire fires are capped per channel at p's ACTUAL push counts
    (C-own: performed pushes only -- self-containment);
  - the peer's wire fires are capped per channel at the OBSERVED arrival
    counts (C-arr; per-channel FIFO makes the first n frames exactly these);
  - every other action (recvs, internal sends, non-wire commits, closes,
    demux deliveries) runs freely = the I-step closure over non-push events,
    with guards evaluated positionally on the derived state (attack-refute
    F6: at the real capacities, state-occupancy == E2-predecessor
    membership).  Deliveries enter only for frames whose snd is already
    capped-in, i.e. performed -- the F1 keystone repair's discipline (no
    forward-delivery citation of an unperformed push is possible by
    construction).
  Then rcv(c, k-1) in Certified U Inevitable  <=>  the derived state has
  consumed >= k-1 frames of c.

Causality boundary (structural): CausalSigma holds the true Skel PRIVATELY
and touches it in exactly two places, both justified by bridge axiom B5
(frames are decoded at delivery, payload-erased model):
  (1) decoding an observed frame (c, n) to the scope it is about
      (positional: frame n on wire(q,h) is about scopesAt(h)[n-1]);
  (2) reading the RECORD of a scope already announced by rule (1)/(2).
Everything else -- the fixpoint, the guards, the census -- goes through
KnownSkel, which raises Unknown outside A_p.  Unknown quantities that only
control "are we done" transitions (stage lengths, list lengths, total leaf
requests) report BIG instead of raising, which UNDER-derives closes only
(never consumption) -- the most-restrictive sound reading.

Ambiguities resolved to the least-information side, noted for the report:
  - own-minting latency: p knows kids(u) for scopes u it answers at the
    ARRIVAL of the frame about parent(u) (refute-c1 1.2's "as soon as the
    parent's listing has arrived"), not earlier;
  - census (BFS positions) is parent-record-driven only; a delivered frame
    does not by itself pin its scope's level position for census purposes
    beyond what announced parent records imply;
  - the strategy is implemented as a function of (pushes, arrivals) only --
    it does not even consult p's own component snapshot (a deterministic
    restriction of the allowed observation, hence still causal).
"""

import random
from model import (I, R, other, apply, allActions, terminal, init, asks,
                   MuxCfg, State, WalkSt, wireOut, wireIn, IMPL,
                   K, H, KIDS, LR)
from mux import (is_wire_commit, is_wire_fire, push_channel,
                 sigma_push_candidates, _certificate, describe_stuck)

BIG = 10 ** 9


class Unknown(Exception):
    """Raised by KnownSkel when a guard needs an unannounced scope record."""
    pass


class _GScope:
    """Guarded scope tuple: fields readable only within A_p."""
    __slots__ = ('_ks', '_i')

    def __init__(self, ks, i):
        self._ks = ks
        self._i = i

    def __getitem__(self, idx):
        ks, i = self._ks, self._i
        if idx == K or idx == H:
            kind, h = ks._kindH(i)
            return kind if idx == K else h
        if i not in ks.known:
            raise Unknown()
        return ks._sk.scope(i)[idx]


class _AsmList:
    """Lazy asmResList stand-in: len() only (BIG when census incomplete)."""
    __slots__ = ('_ks', '_p', '_j')

    def __init__(self, ks, p, j):
        self._ks, self._p, self._j = ks, p, j

    def __len__(self):
        ks = self._ks
        items, comp, kinds = ks._level(self._j)
        if not comp:
            return BIG
        if asks(self._p, self._j):
            return len(items)
        return sum(1 for s in items if kinds[s] == 'D')


class KnownSkel:
    """A_p-restricted skeleton view.  Same accessor interface as Skel where
    model.apply needs it; raises Unknown outside the announced set, or
    returns BIG for pure are-we-done counters (see module docstring)."""

    def __init__(self, sk):
        self._sk = sk
        self.rootH = sk.rootH
        self.fan = sk.fan
        self.capLevel = sk.capLevel
        self.known = set()          # scope ids with announced records
        self._parent = {}
        for i in range(len(sk.scopes)):
            for kd in sk.scope(i)[KIDS]:
                self._parent[kd] = i
        self._version = 0
        self._census_v = -1
        self._census = None

    def add(self, t):
        if t not in self.known:
            self.known.add(t)
            self._version += 1

    # -- kind/height: derivable from the parent's record (or the root) ----

    def _kindH(self, i):
        if i == 0:
            return ('D', self.rootH)
        par = self._parent.get(i)
        if par is None or (par not in self.known and par != 0):
            raise Unknown()
        if par not in self.known:
            raise Unknown()
        sc = self._sk.scope(i)
        return (sc[K], sc[H])

    def scope(self, i):
        return _GScope(self, i)

    # -- census: BFS level prefixes from announced records ----------------

    def _levels(self):
        if self._census_v == self._version:
            return self._census
        levels = {self.rootH: ([0], True)}
        kinds = {0: 'D'}
        for h in range(self.rootH - 1, 0, -1):
            up, upc = levels[h + 1]
            items, comp = [], upc
            for sid in up:
                if kinds[sid] != 'D':
                    continue
                if sid not in self.known:
                    comp = False
                    break
                for kd in self._sk.scope(sid)[KIDS]:
                    items.append(kd)
                    kinds[kd] = self._sk.scope(kd)[K]
            levels[h] = (items, comp)
        self._census = (levels, kinds)
        self._census_v = self._version
        return self._census

    def _level(self, h):
        levels, kinds = self._levels()
        if h not in levels:
            return ([], True, kinds)
        items, comp = levels[h]
        return (items, comp, kinds)

    # -- Skel accessor interface ------------------------------------------

    def stageScopes(self, h):
        items, comp, _ = self._level(h + 1)
        if not comp:
            raise Unknown()
        return items

    def stageLen(self, h):
        items, comp, _ = self._level(h + 1)
        return len(items) if comp else BIG

    def stageScope(self, h, k):
        items, comp, _ = self._level(h + 1)
        if 0 <= k < len(items):
            return items[k]
        if comp:
            return 0
        raise Unknown()

    def _rec(self, s):
        if s not in self.known:
            raise Unknown()
        return self._sk.scope(s)

    def nChildren(self, h, s):
        sc = self._rec(s)
        return sc[LR] if h == 0 else len(sc[KIDS])

    def childIsD(self, h, s, i):
        if h == 0:
            return False
        kids = self._rec(s)[KIDS]
        if 0 <= i < len(kids):
            return self._sk.scope(kids[i])[K] == 'D'
        return False

    def qCount(self, h, s, i):
        if not self.childIsD(h, s, i):
            return 0
        kids = self._rec(s)[KIDS]
        child = self._rec(kids[i])
        return child[LR] if child[H] == 1 else len(child[KIDS])

    def dCount(self, s):
        sc = self._rec(s)
        return sum(1 for kd in sc[KIDS] if self._sk.scope(kd)[K] == 'D')

    def asmResList(self, p, j):
        return _AsmList(self, p, j)

    def pendAt(self, p, j, i):
        items, comp, kinds = self._level(j)
        if asks(p, j):
            if 0 <= i < len(items):
                s = items[i]
                return self.dCount(s)
            if comp:
                return 0
            raise Unknown()
        seen = 0
        for s in items:
            if kinds[s] == 'D':
                if seen == i:
                    rec = self._rec(s)
                    return rec[LR] if rec[H] == 1 else len(rec[KIDS])
                seen += 1
        if comp:
            return 0
        raise Unknown()

    def totalLeafReqs(self):
        items, comp, kinds = self._level(1)
        if not comp:
            return BIG
        t = 0
        for s in items:
            if kinds[s] == 'D':
                if s not in self.known:
                    return BIG
                t += self._sk.scope(s)[LR]
        return t

    def rootPending(self):
        return len(self._rec(0)[KIDS])

    def walkKeys(self):
        return self._sk.walkKeys()

    def asmKeys(self):
        return self._sk.asmKeys()


def sim_init(ks, sk):
    """init(sk) with unknown census tolerated (waiting phases, never done)."""
    s = State.__new__(State)
    s.walk = {}
    for pk in sk.walkKeys():
        sl = ks.stageLen(pk[1])
        s.walk[pk] = WalkSt(0, 0 if 0 < sl else 3,
                            frozenset(), frozenset(), {}, False, None)
    s.asm = {}
    for pk in sk.asmKeys():
        n = len(ks.asmResList(pk[0], pk[1]))
        s.asm[pk] = (0, 0 if n > 0 else 3, 0)
    s.chan = {}
    s.iopenWire = s.iopenQuery = False
    s.iopenCh = None
    s.ropenGotWire = s.ropenWire = s.ropenRes = False
    s.ropenQ = 0
    s.ropenCh = None
    s.absorbIdx = 0
    s.absorbPhase = 0 if ks.totalLeafReqs() > 0 else 3
    s.ifin = s.rfinGotRes = False
    s.rfinGot = 0
    s.pipe = {I: [], R: []}
    return s


def _commit_channel(sk, a):
    """Channel a wire COMMIT action reserves (None if not a wire commit)."""
    if a[0] == 'walkCommit' and a[2][0] == 'wire':
        return wireOut(a[1])
    if a[0] == 'iopenChoose' and a[1] == 'wire':
        return ('w', I, sk.rootH)
    if a[0] == 'ropenChoose' and a[1] == 'wire':
        return ('w', R, sk.rootH)
    return None


class CausalSigma:
    """Causal sigma* for one party: demand verdicts from (pushes, arrivals).

    The true Skel is held privately and consulted ONLY to (a) decode an
    observed frame to its scope (B5) and (b) read records of scopes already
    announced.  All derivation runs on the KnownSkel view."""

    def __init__(self, sk, party, peek=True):
        self._sk = sk
        self.party = party
        self.peek = peek
        self.ks = KnownSkel(sk)
        self.simmux = MuxCfg(BIG)
        self.acts = allActions(sk, self.simmux)
        self.caps = {}          # channel -> evidence cap on sim pushes
        self.arr = {}           # channel -> observed arrival count
        self.spush = {}         # channel -> sim push count
        self.srcv = {}          # channel -> sim rcv count
        self.sim = sim_init(self.ks, sk)
        self.dirty = True
        self.sim_steps = 0

    # -- knowledge (A_p) ---------------------------------------------------

    def _announce(self, c, n):
        """Frame n on wire channel c delivered to this party: update A_p."""
        _, q, h = c
        sk = self._sk
        if h == sk.rootH:
            # q == I: the opening listing; its receiver R mints record(0).
            # q == R: the frame ABOUT scope 0; its receiver I reads
            # record(0) and mints the records of kids(0) (I answers rootH-1).
            self.ks.add(0)
            if q == R:
                for v in sk.scope(0)[KIDS]:
                    self.ks.add(v)
            return
        if h == 0:
            return          # supplies announce nothing
        lst = sk.scopesAt(h)
        if n - 1 < len(lst):
            u = lst[n - 1]
            self.ks.add(u)                       # record(u), received minting
            for v in sk.scope(u)[KIDS]:          # own minting: receiver
                self.ks.add(v)                   # answers height h-1

    def on_arrival(self, c):
        self.arr[c] = self.arr.get(c, 0) + 1
        self.caps[c] = self.arr[c]
        self._announce(c, self.arr[c])
        self.dirty = True

    def on_push(self, c):
        self.caps[c] = self.caps.get(c, 0) + 1
        self.dirty = True

    # -- the fixpoint derivation -------------------------------------------

    def _try(self, a):
        s = self.sim
        if a[0] == 'demux':
            # In the DERIVATION, del(c, n) is gated only by the slot E2 edge
            # (snd performed, rcv(c, n-1) performed) -- the DAG has NO
            # cross-stream pipe-order edges (attack-refute F1).  The sim
            # re-derives pushes in its own cross-channel interleaving, so
            # head-only delivery would manufacture spurious HOL blocks:
            # deliver the first pipe entry whose slot is free instead
            # (per-channel FIFO is preserved; the slot is cap-1).
            d = a[1]
            pipe = s.pipe[d]
            for ix, c in enumerate(pipe):
                if s.chan.get(c, 0) == 0:
                    s2 = s.clone()
                    s2.pipe[d].pop(ix)
                    s2.chan[c] = 1
                    return s2
            return None
        cc = _commit_channel(self._sk, a)
        try:
            if cc is not None and \
                    self.spush.get(cc, 0) >= self.caps.get(cc, 0):
                return None
            fire = is_wire_fire(s, a)
            if fire:
                c = push_channel(self._sk, s, a)
                if self.spush.get(c, 0) >= self.caps.get(c, 0):
                    return None
            s2 = apply(self.ks, IMPL, a, s, self.simmux)
        except Unknown:
            return None
        if s2 is None:
            return None
        if fire:
            self.spush[c] = self.spush.get(c, 0) + 1
        t = a[0]
        if t == 'walkRecvWire':
            c2 = wireIn(a[1])
            self.srcv[c2] = self.srcv.get(c2, 0) + 1
        elif t == 'ropenRecv':
            c2 = ('w', I, self._sk.rootH)
            self.srcv[c2] = self.srcv.get(c2, 0) + 1
        elif t == 'absorbRecvWire':
            c2 = ('w', R, 0)
            self.srcv[c2] = self.srcv.get(c2, 0) + 1
        return s2

    def resume(self):
        if not self.dirty:
            return
        progress = True
        while progress:
            progress = False
            for a in self.acts:
                s2 = self._try(a)
                if s2 is not None:
                    self.sim = s2
                    self.sim_steps += 1
                    progress = True
        self.dirty = False

    def demands(self, c, k):
        """The demand rule: k == 1, or rcv(c, k-1) in Certified U Inevitable."""
        if k == 1:
            return True
        self.resume()
        return self.srcv.get(c, 0) >= k - 1


# -- the causal sigma* x sigma* harness -------------------------------------

def _cand_party(label):
    if label[0] == 'iopen':
        return I
    if label[0] == 'ropen':
        return R
    return label[1][0]


def causal_run(sk, C, seed=0, interleave='random', peek=True,
               max_steps=None, trace=False, crosscheck=False):
    """Symmetric causal-sigma* composition over the true muxed system.

    Outcomes: terminal | sigma_stuck (>=1 mechanically-available push, none
    causally proven, no free action: THE C1-flipping wedge) | hard_stuck |
    fuel.  Also reports idle telemetry and (crosscheck) soundness of every
    causal verdict against the omniscient 'exit' certificate."""
    mux = MuxCfg(C)
    rng = random.Random(seed)
    sk_ops = sk.totalOps()
    fuel = max_steps or (10 * sk_ops + 600)
    cl_fuel = 4 * sk_ops + 200
    base_acts = [a for a in allActions(sk, mux) if not is_wire_commit(a)]
    strats = {I: CausalSigma(sk, I, peek), R: CausalSigma(sk, R, peek)}
    pushed = {}                     # channel -> count of performed pushes
    s = init(sk)
    tr = []
    idle_states = 0
    unsound = 0
    for n in range(fuel):
        if terminal(sk, s, mux):
            return dict(outcome='terminal', steps=n, trace=tr, state=s,
                        idle_states=idle_states, unsound=unsound,
                        sim_steps=strats[I].sim_steps + strats[R].sim_steps)
        free = []
        for a in base_acts:
            if is_wire_fire(s, a):
                continue
            s2 = apply(sk, IMPL, a, s, mux)
            if s2 is not None:
                free.append((a, s2))
        cands = sigma_push_candidates(sk, IMPL, s, C)
        proven = []
        for label, push, c in cands:
            p = _cand_party(label)
            k = pushed.get(c, 0) + 1
            if strats[p].demands(c, k):
                proven.append((label, push, c))
                if crosscheck and not _certificate(sk, IMPL, mux, s, push,
                                                   'exit', cl_fuel):
                    unsound += 1
        if cands and not proven:
            idle_states += 1
        if not free and not proven:
            if cands:
                return dict(outcome='sigma_stuck', steps=n, trace=tr,
                            state=s, cands=cands, idle_states=idle_states,
                            unsound=unsound, strats=strats, pushed=pushed)
            return dict(outcome='hard_stuck', steps=n, trace=tr, state=s,
                        idle_states=idle_states, unsound=unsound)
        moves = [('free', a, s2) for a, s2 in free] + \
                [('push', (label, push, c), None) for label, push, c in proven]
        if interleave == 'random':
            kind, x, s2 = moves[rng.randrange(len(moves))]
        elif interleave == 'push_first':
            ps = [m for m in moves if m[0] == 'push']
            kind, x, s2 = ps[0] if ps else moves[0]
        else:   # greedy: free first
            fs = [m for m in moves if m[0] == 'free']
            kind, x, s2 = fs[0] if fs else moves[0]
        if kind == 'free':
            a = x
            # observation updates BEFORE state change (need pipe head)
            if a[0] == 'demux':
                d = a[1]
                c = s.pipe[d][0]
                if peek:
                    strats[other(d)].on_arrival(c)
            elif not peek:
                if a[0] == 'walkRecvWire':
                    strats[a[1][0]].on_arrival(wireIn(a[1]))
                elif a[0] == 'ropenRecv':
                    strats[R].on_arrival(('w', I, sk.rootH))
                elif a[0] == 'absorbRecvWire':
                    strats[I].on_arrival(('w', R, 0))
            if trace:
                tr.append(a)
            s = s2
        else:
            label, push, c = x
            p = _cand_party(label)
            for a in push:
                if trace:
                    tr.append(a)
                s = apply(sk, IMPL, a, s, mux)
                assert s is not None
            pushed[c] = pushed.get(c, 0) + 1
            strats[p].on_push(c)
    return dict(outcome='fuel', steps=fuel, trace=tr, state=s,
                idle_states=idle_states, unsound=unsound)


def anatomize_wedge(sk, C, res):
    """Diagnostic dump for a sigma_stuck outcome of causal_run."""
    s = res['state']
    lines = ['== CAUSAL WEDGE ==',
             f'steps={res["steps"]} idle_states={res["idle_states"]}',
             describe_stuck(sk, IMPL, s, C), '']
    mux = MuxCfg(C)
    cl_fuel = 4 * sk.totalOps() + 200
    for label, push, c in res['cands']:
        p = _cand_party(label)
        k = res['pushed'].get(c, 0) + 1
        st = res['strats'][p]
        omn = _certificate(sk, IMPL, mux, s, push, 'exit', cl_fuel)
        lines.append(
            f'withheld: party={p} chan={c} k={k} '
            f'sim_rcv={st.srcv.get(c, 0)} (need {k - 1}) '
            f'omniscient_exit={omn} known={len(st.ks.known)}/'
            f'{len(sk.scopes)} arrivals={sum(st.arr.values())}')
    return '\n'.join(lines)
