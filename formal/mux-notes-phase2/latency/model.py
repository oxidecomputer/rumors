"""Mechanical transcription of formal/lean/StreamingMirror/{Skel,Model}.lean.

Transcribed from the Lean source at worktree /Users/oxide/src/rumors-mux
(branch mux-conjectures), NOT re-derived from prose:

  - Skel.lean:  Party/Kind/Scope/Skel, asks, scope/scopesAt/wellFormed,
    walkKeys/asmKeys, stage accessors, prefix sums, schedulable, AxMode.
  - Model.lean: Chan, cap, wiring (wireIn/askedIn/wireOut/lowerOut/upperOut/
    askedOut/asmResChan/asmLevelChan/asmOutChan), Oblig, WalkSt/AsmSt/State,
    freshWalk/scopeComplete/normWalk, producerDone, init, iopenChoosable/
    ropenChoosable/wkChoosable, obligChan/fireOblig, the 23 Action
    constructors, apply, allActions, canStep, terminal, stuck, run.

State = channel occupancies + process program counters exactly as the Lean
has it. Committed-choice semantics: walkCommit parks an obligation that
walkFire must then complete. All Nat subtraction is clamped at 0 (Lean Nat).

The MUX EXTENSION lives in mux.py; this file is the base (independent
channels) model plus hooks: `apply` takes an optional MuxCfg and, when
present, reroutes exactly the wire-family sends through per-direction
bounded FIFO pipes (see mux.py for the model decisions).

LATENCY-HARNESS COPY (mux-latency branch, 2026-07-21). Copied verbatim
from the phase-2 probe at the session scratchpad, with exactly two marked
modifications for the RTT-cost validation (timed.py):

  [L1] `WIDE_INTERNAL` flag on `cap()`: widens every intra-party channel
       (asked/lower/upper/level/leafReqs) to effectively unbounded,
       emulating the shipped `Peer::max_in_flight_nodes` window
       (design/streaming-latency-serialization.md section 5.2). Wire
       channels stay cap-1 (they are the demux slots). Widening only
       removes send-blocking edges (capacity monotonicity), so every
       schedule live at the floor stays live.

  [L2] `MuxCfg` gains `lanes`: 'direction' is the phase-2 mux of record
       (one FIFO per direction); 'stream' keys one FIFO per wire channel,
       which with C large is the fully independent link-transport
       construction (per-stream windows). Pipes become a dict keyed by
       lane; `('demux', lane)` replaces `('demux', party)`.
"""

from __future__ import annotations
import itertools

I, R = 'I', 'R'


def other(p):
    return R if p == I else I


def asks(p, j):
    """Skel.lean `asks`: I asks even heights, R odd."""
    return (j % 2 == 0) if p == I else (j % 2 == 1)


# Degenerate scope: Lean's out-of-range default ⟨Kind.R, 0, [], 0⟩.
DEG = ('R', 0, (), 0)

# scope tuple fields: (kind, height, kids, leafReqs)
K, H, KIDS, LR = 0, 1, 2, 3


class Skel:
    def __init__(self, scopes, rootH, fan, capLevel):
        self.scopes = [(k, h, tuple(kids), lr) for (k, h, kids, lr) in scopes]
        self.rootH = rootH
        self.fan = fan
        self.capLevel = capLevel
        self._scopesAt = {}
        self._asmResList = {}
        self._walkKeys = None
        self._asmKeys = None
        self._totalLeafReqs = None

    # -- Skel.lean accessors --------------------------------------------

    def scope(self, i):
        return self.scopes[i] if 0 <= i < len(self.scopes) else DEG

    def scopesAt(self, h):
        if h not in self._scopesAt:
            self._scopesAt[h] = [i for i in range(len(self.scopes))
                                 if self.scope(i)[H] == h]
        return self._scopesAt[h]

    def wellFormed(self):
        n = len(self.scopes)
        if n == 0:
            return False
        for i in range(n):
            sc = self.scope(i)
            if not (sc[H] >= 1):
                return False
            if not (len(sc[KIDS]) <= self.fan):
                return False
            if not (sc[LR] <= self.fan):
                return False
            if not (sc[K] == 'D' or (len(sc[KIDS]) == 0 and sc[LR] == 0)):
                return False
            if not (sc[H] != 1 or len(sc[KIDS]) == 0):
                return False
            if not (sc[LR] == 0 or (sc[H] == 1 and sc[K] == 'D')):
                return False
            prev, ok = i, True
            for k in sc[KIDS]:
                ok = ok and k > prev and k < n and \
                    self.scope(k)[H] == max(0, sc[H] - 1)
                prev = k
            if not ok:
                return False
        kidCount = sum(len(sc[KIDS]) for sc in self.scopes)
        kidList = [k for sc in self.scopes for k in sc[KIDS]]
        dedup = []
        for k in kidList:
            if k not in dedup:
                dedup.append(k)
        if not (kidCount == n - 1 and len(dedup) == n - 1
                and 0 not in kidList):
            return False
        if not (self.scope(0)[H] == self.rootH and self.scope(0)[K] == 'D'
                and self.rootH % 2 == 0):
            return False
        if not (self.capLevel >= 1):
            return False
        # BFS alignment conjunct
        for h in range(self.rootH):
            flat = [k for s in self.scopesAt(h + 1)
                    for k in self.scope(s)[KIDS]]
            if flat != self.scopesAt(h):
                return False
        return True

    def walkKeys(self):
        if self._walkKeys is None:
            self._walkKeys = \
                [(I, self.rootH - 1 - 2 * k) for k in range(self.rootH // 2)] + \
                [(R, self.rootH - 2 - 2 * k) for k in range(self.rootH // 2)]
        return self._walkKeys

    def asmKeys(self):
        if self._asmKeys is None:
            self._asmKeys = [(I, j + 1) for j in range(self.rootH)] + \
                            [(R, j + 1) for j in range(self.rootH - 1)]
        return self._asmKeys

    def stageScopes(self, h):
        return self.scopesAt(h + 1)

    def stageLen(self, h):
        return len(self.stageScopes(h))

    def stageScope(self, h, k):
        ss = self.stageScopes(h)
        return ss[k] if 0 <= k < len(ss) else 0

    def nChildren(self, h, s):
        sc = self.scope(s)
        return sc[LR] if h == 0 else len(sc[KIDS])

    def childIsD(self, h, s, i):
        if h == 0:
            return False
        kids = self.scope(s)[KIDS]
        if 0 <= i < len(kids):
            return self.scope(kids[i])[K] == 'D'
        return False

    def qCount(self, h, s, i):
        if not self.childIsD(h, s, i):
            return 0
        kids = self.scope(s)[KIDS]
        if 0 <= i < len(kids):
            child = self.scope(kids[i])
            return child[LR] if child[H] == 1 else len(child[KIDS])
        return 0

    def dCount(self, s):
        return sum(1 for k in self.scope(s)[KIDS]
                   if self.scope(k)[K] == 'D')

    def dOf(self, h, s):
        return 0 if h == 0 else self.dCount(s)

    def qOf(self, h, s):
        return sum(self.qCount(h, s, i) for i in range(self.nChildren(h, s)))

    def totalLeafReqs(self):
        if self._totalLeafReqs is None:
            self._totalLeafReqs = sum(
                self.scope(s)[LR] for s in self.scopesAt(1)
                if self.scope(s)[K] == 'D')
        return self._totalLeafReqs

    def asmResList(self, p, j):
        key = (p, j)
        if key not in self._asmResList:
            if asks(p, j):
                lst = [self.dCount(s) for s in self.scopesAt(j)]
            else:
                lst = []
                for s in self.scopesAt(j):
                    sc = self.scope(s)
                    if sc[K] == 'D':
                        lst.append(sc[LR] if sc[H] == 1 else len(sc[KIDS]))
            self._asmResList[key] = lst
        return self._asmResList[key]

    def rootPending(self):
        return len(self.scope(0)[KIDS])

    def pendAt(self, p, j, i):
        lst = self.asmResList(p, j)
        return lst[i] if 0 <= i < len(lst) else 0

    def schedulable(self):
        return all(self.dCount(s) <= self.capLevel + 2
                   for s in range(len(self.scopes)))

    def maxDCount(self):
        return max((self.dCount(s) for s in range(len(self.scopes))),
                   default=0)

    def margin0(self):
        """dCount ≤ capLevel everywhere: the .impl flagship hypothesis."""
        return all(self.dCount(s) <= self.capLevel
                   for s in range(len(self.scopes)))

    def totalOps(self):
        """Rough ρ(init) upper bound, for run fuel."""
        t = 0
        for (p, h) in self.walkKeys():
            for s in self.stageScopes(h):
                t += 2 + self.nChildren(h, s) * 3 + self.qOf(h, s) + 1
            t += 2
        for (p, j) in self.asmKeys():
            t += sum(2 + pend for pend in self.asmResList(p, j)) + 1
        t += 6 + 3 * self.totalLeafReqs() + 4 + self.rootPending() + 4
        return t


# AxMode: (w, d1root, d1int, d2, d3, d4, d5, d6, wireFirst)
class AxMode:
    __slots__ = ('w', 'd1root', 'd1int', 'd2', 'd3', 'd4', 'd5', 'd6',
                 'wireFirst', 'name')

    def __init__(self, w, d1root, d1int, d2, d3, d4, d5, d6, wireFirst,
                 name=''):
        self.w, self.d1root, self.d1int, self.d2, self.d3 = \
            w, d1root, d1int, d2, d3
        self.d4, self.d5, self.d6, self.wireFirst = d4, d5, d6, wireFirst
        self.name = name

    def __repr__(self):
        return f'AxMode({self.name})'


FULL = AxMode(True, True, True, True, True, True, True, False, False, 'full')
IMPL = AxMode(True, True, True, True, True, True, False, True, False, 'impl')
FULL_NO_D4 = AxMode(True, True, True, True, True, False, False, False, False,
                    'fullNoD4')
FULL_NO_D5 = AxMode(True, True, True, True, True, True, False, False, False,
                    'fullNoD5')


# -- channels ------------------------------------------------------------
# ('w',p,h) wire | ('a',p,h) asked | ('lr',) leafRequests | ('u',p,h) upper
# ('l',p,h) lower | ('lv',p,j) level | ('rr',) rootret | ('rrs',) rootrets
# ('rres',) rootres

WIDE_INTERNAL = False    # [L1] emulate the shipped pipeline window
_WIDE = 10 ** 9


def cap(sk, c):
    if WIDE_INTERNAL and c[0] in ('a', 'l', 'u', 'lv', 'lr'):
        return _WIDE
    return sk.capLevel if c[0] == 'lv' else 1


def wireIn(pk):
    return ('w', other(pk[0]), pk[1] + 1)


def askedIn(pk):
    return ('a', pk[0], pk[1])


def wireOut(pk):
    return ('w', pk[0], pk[1])


def lowerOut(pk):
    return ('l', pk[0], pk[1])


def upperOut(pk):
    return ('u', pk[0], pk[1])


def askedOut(pk):
    return ('lr',) if pk[1] < 2 else ('a', pk[0], pk[1] - 2)


def asmResChan(pk):
    return ('u', pk[0], pk[1] - 1) if asks(pk[0], pk[1]) \
        else ('l', pk[0], pk[1])


def asmLevelChan(pk):
    return ('lv', pk[0], pk[1] - 1)


def asmOutChan(sk, pk):
    if pk[0] == I and pk[1] == sk.rootH:
        return ('rr',)
    if pk[0] == R and pk[1] == sk.rootH - 1:
        return ('rrs',)
    return ('lv', pk[0], pk[1])


def is_wire_chan(c):
    return c[0] == 'w'


# -- state ----------------------------------------------------------------

class WalkSt:
    __slots__ = ('scope', 'phase', 'wireDone', 'resDone', 'qSent',
                 'parentDone', 'committed')

    def __init__(self, scope, phase, wireDone, resDone, qSent, parentDone,
                 committed):
        self.scope = scope
        self.phase = phase
        self.wireDone = wireDone      # frozenset of child indices
        self.resDone = resDone        # frozenset
        self.qSent = qSent            # dict i -> count (missing = 0)
        self.parentDone = parentDone
        self.committed = committed    # None | ('wire',i)|('res',i)|('query',i)|('parent',)

    def clone(self):
        return WalkSt(self.scope, self.phase, self.wireDone, self.resDone,
                      dict(self.qSent), self.parentDone, self.committed)

    def key(self):
        return (self.scope, self.phase, self.wireDone, self.resDone,
                tuple(sorted(self.qSent.items())), self.parentDone,
                self.committed)


class State:
    __slots__ = ('walk', 'asm', 'chan', 'iopenWire', 'iopenQuery', 'iopenCh',
                 'ropenGotWire', 'ropenWire', 'ropenRes', 'ropenQ', 'ropenCh',
                 'absorbIdx', 'absorbPhase', 'ifin', 'rfinGotRes', 'rfinGot',
                 'pipe')

    def clone(self):
        s = State.__new__(State)
        s.walk = {k: v.clone() for k, v in self.walk.items()}
        s.asm = dict(self.asm)   # AsmSt as tuple (idx, phase, got)
        s.chan = dict(self.chan)
        s.iopenWire, s.iopenQuery, s.iopenCh = \
            self.iopenWire, self.iopenQuery, self.iopenCh
        s.ropenGotWire, s.ropenWire, s.ropenRes, s.ropenQ, s.ropenCh = \
            self.ropenGotWire, self.ropenWire, self.ropenRes, self.ropenQ, \
            self.ropenCh
        s.absorbIdx, s.absorbPhase = self.absorbIdx, self.absorbPhase
        s.ifin, s.rfinGotRes, s.rfinGot = \
            self.ifin, self.rfinGotRes, self.rfinGot
        s.pipe = {k: list(v) for k, v in self.pipe.items()}
        return s

    def ch(self, c):
        return self.chan.get(c, 0)


def freshWalk(sk, h, k):
    return WalkSt(k, 0 if k < sk.stageLen(h) else 3,
                  frozenset(), frozenset(), {}, False, None)


def scopeComplete(sk, h, ws):
    if ws.scope >= sk.stageLen(h):
        return True
    s = sk.stageScope(h, ws.scope)
    if not ws.parentDone:
        return False
    for i in range(sk.nChildren(h, s)):
        if i not in ws.wireDone:
            return False
        if sk.childIsD(h, s, i):
            if i not in ws.resDone or ws.qSent.get(i, 0) != sk.qCount(h, s, i):
                return False
    return True


def normWalk(sk, h, ws):
    if ws.phase == 2 and scopeComplete(sk, h, ws):
        return freshWalk(sk, h, ws.scope + 1)
    return ws


def doneWalk(ws):
    return ws.phase == 5


def doneAsm(a):
    return a[1] == 4


def doneIOpen(s):
    return s.iopenWire and s.iopenQuery


def doneROpen(sk, s):
    return s.ropenGotWire and s.ropenWire and s.ropenRes and \
        s.ropenQ == len(sk.scope(0)[KIDS])


def producerDone(sk, s, c):
    kind = c[0]
    if kind == 'w':
        p, h = c[1], c[2]
        if h == sk.rootH:
            return doneIOpen(s) if p == I else doneROpen(sk, s)
        return doneWalk(s.walk[(p, h)])
    if kind == 'a':
        p, h = c[1], c[2]
        if p == I and h == sk.rootH - 1:
            return doneIOpen(s)
        if p == R and h == sk.rootH - 2:
            return doneROpen(sk, s)
        return doneWalk(s.walk[(p, h + 2)])
    if kind == 'lr':
        return doneWalk(s.walk[(I, 1)])
    if kind in ('u', 'l'):
        return doneWalk(s.walk[(c[1], c[2])])
    return False   # level/root channels: never close-recv'd


def init(sk):
    s = State.__new__(State)
    s.walk = {pk: freshWalk(sk, pk[1], 0) for pk in sk.walkKeys()}
    s.asm = {pk: (0, 0 if len(sk.asmResList(pk[0], pk[1])) > 0 else 3, 0)
             for pk in sk.asmKeys()}
    s.chan = {}
    s.iopenWire = s.iopenQuery = False
    s.iopenCh = None
    s.ropenGotWire = s.ropenWire = s.ropenRes = False
    s.ropenQ = 0
    s.ropenCh = None
    s.absorbIdx = 0
    s.absorbPhase = 0 if sk.totalLeafReqs() > 0 else 3
    s.ifin = s.rfinGotRes = False
    s.rfinGot = 0
    s.pipe = {}
    return s


# -- choosability guards --------------------------------------------------

def iopenChoosable(ax, s, o):
    if o == 'wire':
        return not s.iopenWire
    # query
    return (not s.iopenQuery) and ((not ax.w) or s.iopenWire)


def ropenChoosable(sk, ax, s, o):
    if o == 'wire':
        return s.ropenGotWire and not s.ropenWire
    if o == 'res':
        return s.ropenGotWire and not s.ropenRes and \
            ((not ax.w) or s.ropenWire)
    # query
    return s.ropenGotWire and s.ropenQ < len(sk.scope(0)[KIDS]) and \
        ((not ax.d1root) or s.ropenRes) and \
        ((not ax.wireFirst) or s.ropenWire)


def wkChoosable(sk, ax, pk, ws, o):
    if ws.phase != 2 or ws.committed is not None:
        return False
    h = pk[1]
    s = sk.stageScope(h, ws.scope)
    n = sk.nChildren(h, s)
    kind = o[0]
    if kind == 'wire':
        i = o[1]
        if not (i < n and i not in ws.wireDone):
            return False
        if not all(j in ws.wireDone for j in range(i)):
            return False
        if ax.d4:
            for j in range(i):
                if sk.childIsD(h, s, j) and not (
                        j in ws.resDone
                        and ws.qSent.get(j, 0) == sk.qCount(h, s, j)):
                    return False
        if ax.d5 and not ws.parentDone:
            if all((not sk.childIsD(h, s, j)) or j in ws.resDone
                   for j in range(n)):
                return False
        return True
    if kind == 'res':
        i = o[1]
        if not (i < n and sk.childIsD(h, s, i) and i not in ws.resDone):
            return False
        if not all((not sk.childIsD(h, s, j)) or j in ws.resDone
                   for j in range(i)):
            return False
        if ax.w and i not in ws.wireDone:
            return False
        if ax.d3:
            for j in range(n):
                if j in ws.resDone and \
                        ws.qSent.get(j, 0) != sk.qCount(h, s, j):
                    return False
        return True
    if kind == 'query':
        i = o[1]
        if not (i < n and sk.childIsD(h, s, i)
                and ws.qSent.get(i, 0) < sk.qCount(h, s, i)):
            return False
        if not all(ws.qSent.get(j, 0) == sk.qCount(h, s, j)
                   for j in range(i)):
            return False
        if ax.d1int and i not in ws.resDone:
            return False
        if ax.wireFirst and i not in ws.wireDone:
            return False
        if ax.d5 and not ws.parentDone:
            if all((not sk.childIsD(h, s, j)) or j in ws.resDone
                   for j in range(n)):
                return False
        return True
    # parent
    if ws.parentDone:
        return False
    if ax.d2:
        if not all((not sk.childIsD(h, s, j)) or j in ws.resDone
                   for j in range(n)):
            return False
    if ax.d6:
        for j in range(n):
            if j not in ws.wireDone:
                return False
            if sk.childIsD(h, s, j) and not (
                    j in ws.resDone
                    and ws.qSent.get(j, 0) == sk.qCount(h, s, j)):
                return False
    return True


def obligChan(sk, pk, o):
    kind = o[0]
    if kind == 'wire':
        return wireOut(pk)
    if kind == 'res':
        return lowerOut(pk)
    if kind == 'query':
        return askedOut(pk)
    return upperOut(pk)


def fireOblig(ws, o):
    ws = ws.clone()
    kind = o[0]
    if kind == 'wire':
        ws.wireDone = ws.wireDone | {o[1]}
    elif kind == 'res':
        ws.resDone = ws.resDone | {o[1]}
    elif kind == 'query':
        ws.qSent[o[1]] = ws.qSent.get(o[1], 0) + 1
    else:
        ws.parentDone = True
    ws.committed = None
    return ws


# -- mux configuration ----------------------------------------------------

class MuxCfg:
    """Per MUX-PROGRESS.md §2: per direction one bounded FIFO of capacity C
    (message-counted). Only wire-family channels are muxed; a wire send by
    party p enters pipe[p] (guard: len < C) instead of the wire channel; the
    demux action delivers the pipe HEAD into the target wire channel's
    existing cap-1 slot, blocking (head-of-line) while the slot is full.
    Close-recv of a wire channel additionally requires no in-flight message
    for it in the producer's pipe.

    [L2] `lanes='direction'` is the mux of record; `lanes='stream'` keys one
    FIFO per wire channel (per-stream windows = independent links)."""
    __slots__ = ('C', 'lanes')

    def __init__(self, C, lanes='direction'):
        self.C = C
        self.lanes = lanes


def lane_of(mux, c):
    """[L2] the pipe key a wire channel rides."""
    return c[1] if mux.lanes == 'direction' else c


def lane_list(sk, mux):
    """[L2] every lane the configuration can populate."""
    if mux.lanes == 'direction':
        return [I, R]
    return [('w', I, sk.rootH), ('w', R, sk.rootH)] + \
        [wireOut(pk) for pk in sk.walkKeys()]


def pipe_ref(s, lane):
    return s.pipe.setdefault(lane, [])


def pipe_get(s, lane):
    return s.pipe.get(lane, [])


def _bump(chan, c, d):
    v = chan.get(c, 0) + d
    if v:
        chan[c] = v
    else:
        chan.pop(c, None)


def _pipe_count(s, c, mux):
    return pipe_get(s, lane_of(mux, c)).count(c)


# -- apply ----------------------------------------------------------------

def apply(sk, ax, a, s, mux=None):
    """Guarded transition: returns new State or None (guard failed).
    Mirrors Model.lean `apply` branch-for-branch; `mux` reroutes wire sends."""
    tag = a[0]

    if tag == 'demux':
        if mux is None:
            return None
        lane = a[1]
        pipe = pipe_get(s, lane)
        if pipe and s.ch(pipe[0]) == 0:
            s2 = s.clone()
            c = s2.pipe[lane].pop(0)
            _bump(s2.chan, c, 1)
            return s2
        return None

    if tag == 'iopenChoose':
        if s.iopenCh is None and iopenChoosable(ax, s, a[1]):
            s2 = s.clone()
            s2.iopenCh = a[1]
            return s2
        return None

    if tag == 'iopenFire':
        if s.iopenCh == 'wire':
            c = ('w', I, sk.rootH)
            if mux is not None:
                lane = lane_of(mux, c)
                if len(pipe_get(s, lane)) < mux.C:
                    s2 = s.clone()
                    pipe_ref(s2, lane).append(c)
                    s2.iopenWire = True
                    s2.iopenCh = None
                    return s2
                return None
            if s.ch(c) < 1:
                s2 = s.clone()
                _bump(s2.chan, c, 1)
                s2.iopenWire = True
                s2.iopenCh = None
                return s2
            return None
        if s.iopenCh == 'query':
            c = ('a', I, sk.rootH - 1)
            if s.ch(c) < cap(sk, c):    # [L1] was the base model's cap-1
                s2 = s.clone()
                _bump(s2.chan, c, 1)
                s2.iopenQuery = True
                s2.iopenCh = None
                return s2
            return None
        return None

    if tag == 'ropenRecv':
        c = ('w', I, sk.rootH)
        if (not s.ropenGotWire) and s.ch(c) > 0:
            s2 = s.clone()
            _bump(s2.chan, c, -1)
            s2.ropenGotWire = True
            return s2
        return None

    if tag == 'ropenChoose':
        if s.ropenCh is None and ropenChoosable(sk, ax, s, a[1]):
            s2 = s.clone()
            s2.ropenCh = a[1]
            return s2
        return None

    if tag == 'ropenFire':
        if s.ropenCh == 'wire':
            c = ('w', R, sk.rootH)
            if mux is not None:
                lane = lane_of(mux, c)
                if len(pipe_get(s, lane)) < mux.C:
                    s2 = s.clone()
                    pipe_ref(s2, lane).append(c)
                    s2.ropenWire = True
                    s2.ropenCh = None
                    return s2
                return None
            if s.ch(c) < 1:
                s2 = s.clone()
                _bump(s2.chan, c, 1)
                s2.ropenWire = True
                s2.ropenCh = None
                return s2
            return None
        if s.ropenCh == 'res':
            if s.ch(('rres',)) < 1:
                s2 = s.clone()
                _bump(s2.chan, ('rres',), 1)
                s2.ropenRes = True
                s2.ropenCh = None
                return s2
            return None
        if s.ropenCh == 'query':
            c = ('a', R, sk.rootH - 2)
            if s.ch(c) < cap(sk, c):    # [L1] was the base model's cap-1
                s2 = s.clone()
                _bump(s2.chan, c, 1)
                s2.ropenQ += 1
                s2.ropenCh = None
                return s2
            return None
        return None

    if tag == 'walkRecvWire':
        pk = a[1]
        if pk not in s.walk:
            return None
        ws = s.walk[pk]
        c = wireIn(pk)
        if ws.phase == 0 and s.ch(c) > 0:
            s2 = s.clone()
            _bump(s2.chan, c, -1)
            w2 = s2.walk[pk]
            w2.phase = 1
            w2.committed = None
            return s2
        return None

    if tag == 'walkRecvAsked':
        pk = a[1]
        if pk not in s.walk:
            return None
        ws = s.walk[pk]
        c = askedIn(pk)
        if ws.phase == 1 and s.ch(c) > 0:
            s2 = s.clone()
            _bump(s2.chan, c, -1)
            w2 = s2.walk[pk].clone()
            w2.phase = 2
            w2.committed = None
            s2.walk[pk] = normWalk(sk, pk[1], w2)
            return s2
        return None

    if tag == 'walkCommit':
        pk, o = a[1], a[2]
        if pk not in s.walk:
            return None
        ws = s.walk[pk]
        if wkChoosable(sk, ax, pk, ws, o):
            s2 = s.clone()
            s2.walk[pk].committed = o
            return s2
        return None

    if tag == 'walkFire':
        pk = a[1]
        if pk not in s.walk:
            return None
        ws = s.walk[pk]
        if ws.committed is None or ws.phase != 2:
            return None
        o = ws.committed
        c = obligChan(sk, pk, o)
        if mux is not None and o[0] == 'wire':
            lane = lane_of(mux, c)
            if len(pipe_get(s, lane)) < mux.C:
                s2 = s.clone()
                pipe_ref(s2, lane).append(c)
                s2.walk[pk] = normWalk(sk, pk[1], fireOblig(s2.walk[pk], o))
                return s2
            return None
        if s.ch(c) < cap(sk, c):        # [L1] was the base model's cap-1
            s2 = s.clone()
            _bump(s2.chan, c, 1)
            s2.walk[pk] = normWalk(sk, pk[1], fireOblig(s2.walk[pk], o))
            return s2
        return None

    if tag == 'walkCloseWire':
        pk = a[1]
        if pk not in s.walk:
            return None
        ws = s.walk[pk]
        c = wireIn(pk)
        if ws.phase == 3 and producerDone(sk, s, c) and s.ch(c) == 0 and \
                (mux is None or _pipe_count(s, c, mux) == 0):
            s2 = s.clone()
            s2.walk[pk].phase = 4
            return s2
        return None

    if tag == 'walkCloseAsked':
        pk = a[1]
        if pk not in s.walk:
            return None
        ws = s.walk[pk]
        c = askedIn(pk)
        if ws.phase == 4 and producerDone(sk, s, c) and s.ch(c) == 0:
            s2 = s.clone()
            s2.walk[pk].phase = 5
            return s2
        return None

    if tag == 'asmRecvRes':
        pk = a[1]
        if pk not in s.asm:
            return None
        idx, phase, got = s.asm[pk]
        c = asmResChan(pk)
        if phase == 0 and s.ch(c) > 0:
            s2 = s.clone()
            _bump(s2.chan, c, -1)
            s2.asm[pk] = (idx, 1 if sk.pendAt(pk[0], pk[1], idx) > 0 else 2, 0)
            return s2
        return None

    if tag == 'asmRecvLevel':
        pk = a[1]
        if pk not in s.asm:
            return None
        idx, phase, got = s.asm[pk]
        c = asmLevelChan(pk)
        if phase == 1 and s.ch(c) > 0:
            s2 = s.clone()
            _bump(s2.chan, c, -1)
            s2.asm[pk] = (idx,
                          2 if got + 1 == sk.pendAt(pk[0], pk[1], idx) else 1,
                          got + 1)
            return s2
        return None

    if tag == 'asmSend':
        pk = a[1]
        if pk not in s.asm:
            return None
        idx, phase, got = s.asm[pk]
        c = asmOutChan(sk, pk)
        if phase == 2 and s.ch(c) < cap(sk, c):
            s2 = s.clone()
            _bump(s2.chan, c, 1)
            nxt = 0 if idx + 1 < len(sk.asmResList(pk[0], pk[1])) else 3
            s2.asm[pk] = (idx + 1, nxt, 0)
            return s2
        return None

    if tag == 'asmClose':
        pk = a[1]
        if pk not in s.asm:
            return None
        idx, phase, got = s.asm[pk]
        c = asmResChan(pk)
        if phase == 3 and producerDone(sk, s, c) and s.ch(c) == 0:
            s2 = s.clone()
            s2.asm[pk] = (idx, 4, got)
            return s2
        return None

    if tag == 'absorbRecvWire':
        c = ('w', R, 0)
        if s.absorbPhase == 0 and s.ch(c) > 0:
            s2 = s.clone()
            _bump(s2.chan, c, -1)
            s2.absorbPhase = 1
            return s2
        return None

    if tag == 'absorbRecvAsked':
        if s.absorbPhase == 1 and s.ch(('lr',)) > 0:
            s2 = s.clone()
            _bump(s2.chan, ('lr',), -1)
            s2.absorbPhase = 2
            return s2
        return None

    if tag == 'absorbSend':
        c = ('lv', I, 0)
        if s.absorbPhase == 2 and s.ch(c) < cap(sk, c):
            s2 = s.clone()
            _bump(s2.chan, c, 1)
            s2.absorbIdx += 1
            s2.absorbPhase = 0 if s2.absorbIdx < sk.totalLeafReqs() else 3
            return s2
        return None

    if tag == 'absorbCloseWire':
        c = ('w', R, 0)
        if s.absorbPhase == 3 and producerDone(sk, s, c) and s.ch(c) == 0 and \
                (mux is None or _pipe_count(s, c, mux) == 0):
            s2 = s.clone()
            s2.absorbPhase = 4
            return s2
        return None

    if tag == 'absorbCloseAsked':
        if s.absorbPhase == 4 and producerDone(sk, s, ('lr',)) and \
                s.ch(('lr',)) == 0:
            s2 = s.clone()
            s2.absorbPhase = 5
            return s2
        return None

    if tag == 'finRet':
        if (not s.ifin) and s.ch(('rr',)) > 0:
            s2 = s.clone()
            _bump(s2.chan, ('rr',), -1)
            s2.ifin = True
            return s2
        return None

    if tag == 'finRes':
        if (not s.rfinGotRes) and s.ch(('rres',)) > 0:
            s2 = s.clone()
            _bump(s2.chan, ('rres',), -1)
            s2.rfinGotRes = True
            return s2
        return None

    if tag == 'finRets':
        if s.rfinGotRes and s.rfinGot < sk.rootPending() and \
                s.ch(('rrs',)) > 0:
            s2 = s.clone()
            _bump(s2.chan, ('rrs',), -1)
            s2.rfinGot += 1
            return s2
        return None

    raise ValueError(f'unknown action {a}')


# -- allActions in the Lean's exact enumeration order ---------------------

def allActions(sk, mux=None):
    acts = [('iopenChoose', 'wire'), ('iopenChoose', 'query'), ('iopenFire',),
            ('ropenRecv',), ('ropenChoose', 'wire'), ('ropenChoose', 'res'),
            ('ropenChoose', 'query'), ('ropenFire',),
            ('absorbRecvWire',), ('absorbRecvAsked',), ('absorbSend',),
            ('absorbCloseWire',), ('absorbCloseAsked',),
            ('finRet',), ('finRes',), ('finRets',)]
    for pk in sk.walkKeys():
        acts += [('walkRecvWire', pk), ('walkRecvAsked', pk),
                 ('walkFire', pk), ('walkCloseWire', pk),
                 ('walkCloseAsked', pk), ('walkCommit', pk, ('parent',))]
        for i in range(sk.fan):
            acts += [('walkCommit', pk, ('wire', i)),
                     ('walkCommit', pk, ('res', i)),
                     ('walkCommit', pk, ('query', i))]
    for pk in sk.asmKeys():
        acts += [('asmRecvRes', pk), ('asmRecvLevel', pk),
                 ('asmSend', pk), ('asmClose', pk)]
    if mux is not None:
        acts += [('demux', lane) for lane in lane_list(sk, mux)]
    return acts


def terminal(sk, s, mux=None):
    if mux is not None and any(s.pipe.values()):
        return False
    return (all(doneWalk(s.walk[pk]) for pk in sk.walkKeys())
            and all(doneAsm(s.asm[pk]) for pk in sk.asmKeys())
            and doneIOpen(s) and doneROpen(sk, s)
            and s.absorbPhase == 5 and s.ifin and s.rfinGotRes
            and s.rfinGot == sk.rootPending())


def canStep(sk, ax, s, mux=None, acts=None):
    for a in (acts if acts is not None else allActions(sk, mux)):
        if apply(sk, ax, a, s, mux) is not None:
            return True
    return False


def stuck(sk, ax, s, mux=None, acts=None):
    return (not terminal(sk, s, mux)) and not canStep(sk, ax, s, mux, acts)


def run(sk, ax, s, acts_list, mux=None):
    """Replay an action list; None on the first disabled action."""
    for a in acts_list:
        s = apply(sk, ax, a, s, mux)
        if s is None:
            return None
    return s


def drain(sk, ax, fuel, s, mux=None, acts=None):
    """Greedy firstM scheduler (Lean `drive`/`drain`): fire the first
    enabled action in allActions order until quiescent or out of fuel."""
    if acts is None:
        acts = allActions(sk, mux)
    for _ in range(fuel):
        nxt = None
        for a in acts:
            nxt = apply(sk, ax, a, s, mux)
            if nxt is not None:
                break
        if nxt is None:
            return s
        s = nxt
    return s


def all_drained(sk, s):
    """Instances.lean `allActionsDrained`: every touched channel empty."""
    for pk in sk.walkKeys():
        for c in (wireIn(pk), askedIn(pk), wireOut(pk), lowerOut(pk),
                  upperOut(pk)):
            if s.ch(c) != 0:
                return False
    for pk in sk.asmKeys():
        for c in (asmResChan(pk), asmLevelChan(pk), asmOutChan(sk, pk)):
            if s.ch(c) != 0:
                return False
    return s.ch(('lr',)) == 0 and s.ch(('lv', I, 0)) == 0 and \
        s.ch(('rr',)) == 0 and s.ch(('rrs',)) == 0 and s.ch(('rres',)) == 0


def completes(sk, fuel=2000):
    s = drain(sk, FULL, fuel, init(sk))
    return terminal(sk, s) and all_drained(sk, s)
