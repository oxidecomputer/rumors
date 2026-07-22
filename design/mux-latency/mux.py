"""The mux layer per MUX-PROGRESS.md §2, plus the scheduler strategies.

Model decisions implemented here (all flagged in the report):
  - Per direction one bounded FIFO `pipe[p]` of capacity C, message-counted.
    Only the wire family is muxed (`wire p h`, incl. the two opening wires);
    every intra-party channel keeps its Model.lean semantics untouched.
  - A wire send by party p enters pipe[p] when len < C (the send REPLACES
    the base model's cap-1 wire-channel send; the cap-1 wire channel itself
    becomes the receiver-side demux slot).
  - Demux (one per direction, non-strategic): delivers the pipe HEAD into
    the head's target wire channel slot when that slot is empty; blocks
    (head-of-line) while it is full. This is the shipped discipline
    (wire-order delivery into per-stream one-slot handoffs, sole reader).
  - Close-recv of a wire channel additionally requires no in-flight
    message for it in the producer's pipe (EOS after final bytes).
  - Deadlock = reachable non-terminal state where no process, mux or
    demux can move (for sigma*: ... no move the strategy permits).

Strategies:
  - EAGER (work-conserving): whenever the pipe has room and some wire send
    is enabled, one may enter; the policy picks WHICH. Policies: 'bottom'
    (bottom-most-ready: deepest stream first = the shipped discipline),
    'top', 'rr' (round-robin over heights), 'rand' (seeded).
  - SIGMA* (demand-lockstep): a wire send may enter the pipe only when its
    eventual EXIT from the pipe is provable from traffic already committed:
    clone the state, perform the push, then close under every action EXCEPT
    further wire pushes/commits (both sides), and require the pushed
    channel's pipe occupancy to return to zero ('exit' certificate; the
    'consume' variant additionally requires the slot to drain). Idle
    otherwise. Commit-to-wire is fused with the push so a walk never parks
    committed on an unproven wire.
"""

import random
from model import (I, R, other, apply, allActions, terminal, init,
                   iopenChoosable, ropenChoosable, wkChoosable, wireOut,
                   MuxCfg, lane_of, pipe_get)

# LATENCY-HARNESS COPY: adapted to model.py's [L2] lane generalization
# (sigma_push_candidates takes the MuxCfg; capacity checks are per lane).


# -- action classification -------------------------------------------------

def is_wire_commit(a):
    return (a[0] == 'walkCommit' and a[2][0] == 'wire') or \
        (a[0] == 'iopenChoose' and a[1] == 'wire') or \
        (a[0] == 'ropenChoose' and a[1] == 'wire')


def is_wire_fire(s, a):
    """Does this fire action push into a pipe, in state s?"""
    if a[0] == 'iopenFire':
        return s.iopenCh == 'wire'
    if a[0] == 'ropenFire':
        return s.ropenCh == 'wire'
    if a[0] == 'walkFire':
        ws = s.walk.get(a[1])
        return ws is not None and ws.committed is not None \
            and ws.committed[0] == 'wire'
    return False


def push_channel(sk, s, a):
    """The wire channel a push action would append (assumes is_wire_fire)."""
    if a[0] == 'iopenFire':
        return ('w', I, sk.rootH)
    if a[0] == 'ropenFire':
        return ('w', R, sk.rootH)
    pk = a[1]
    return wireOut(pk)


# -- enabled-action enumeration -------------------------------------------

class Runner:
    def __init__(self, sk, ax, C):
        self.sk = sk
        self.ax = ax
        self.mux = MuxCfg(C)
        self.acts = allActions(sk, self.mux)

    def enabled(self, s):
        out = []
        for a in self.acts:
            s2 = apply(self.sk, self.ax, a, s, self.mux)
            if s2 is not None:
                out.append((a, s2))
        return out


# -- eager (work-conserving) runs -------------------------------------------

def _policy_key(policy, rng):
    """Order key over push items (a, s2, chan) (smaller = preferred)."""
    def key(item):
        h = item[2][2]
        if policy == 'bottom':
            return (h,)          # deepest (lowest height) first
        if policy == 'top':
            return (-h,)
        if policy == 'rand':
            return (rng.random(),)
        raise ValueError(policy)
    return key


def eager_run(sk, ax, C, policy='bottom', interleave='greedy', seed=0,
              max_steps=None, trace=False):
    """Work-conserving mux run.

    interleave='greedy': deterministic; each step fire, in priority order:
      demux moves, then the policy-preferred enabled push, then the first
      other enabled action (allActions order). Push-preferred variant of
      the shipped flush-paced sender ("sender runs ahead").
    interleave='random': uniformly random among enabled moves, except that
      when a push is selected it is the policy's choice among enabled
      pushes (the strategy stays fixed; the interleaving varies).

    Returns dict(outcome, steps, trace?, state).
    """
    sk_ops = sk.totalOps()
    fuel = max_steps or (6 * sk_ops + 400)
    rng = random.Random(seed)
    rr_next = 0
    runner = Runner(sk, ax, C)
    s = init(sk)
    tr = []
    for n in range(fuel):
        if terminal(sk, s, runner.mux):
            return dict(outcome='terminal', steps=n, trace=tr, state=s)
        en = runner.enabled(s)
        if not en:
            return dict(outcome='stuck', steps=n, trace=tr, state=s)
        demuxes = [e for e in en if e[0][0] == 'demux']
        pushes = [(a, s2, push_channel(sk, s, a)) for (a, s2) in en
                  if is_wire_fire(s, a)]
        others = [e for e in en
                  if e[0][0] != 'demux' and not is_wire_fire(s, e[0])]
        # pick the policy's preferred push
        chosen_push = None
        if pushes:
            if policy == 'cert':
                # Smartest work-conserving policy: prefer a push whose
                # pipe-exit is certified (sigma*'s certificate); if NONE
                # is certified, work-conservation forces a push anyway
                # (fall back to bottom-most). Idling is not allowed.
                cl_fuel = 4 * sk_ops + 200
                certified = [e for e in pushes
                             if _certificate(sk, ax, runner.mux, s,
                                             [e[0]], 'exit', cl_fuel)]
                pool2 = certified if certified else pushes
                chosen_push = min(pool2, key=_policy_key('bottom', rng))
            elif policy == 'rr':
                tagged = sorted(pushes, key=lambda e: e[2][2])
                pick = 0
                for k, (_, _, c) in enumerate(tagged):
                    if c[2] >= rr_next:
                        pick = k
                        break
                chosen_push = tagged[pick]
            else:
                chosen_push = min(pushes, key=_policy_key(policy, rng))
        if interleave == 'greedy':
            if demuxes:
                a, s2 = demuxes[0]
            elif chosen_push is not None:
                a, s2 = chosen_push[0], chosen_push[1]
            elif others:
                a, s2 = others[0]
            else:
                return dict(outcome='stuck', steps=n, trace=tr, state=s)
        elif interleave == 'push_first':
            # flush-paced sender running ahead: pushes beat everything;
            # demux only when no push; consumers last.
            if chosen_push is not None:
                a, s2 = chosen_push[0], chosen_push[1]
            elif demuxes:
                a, s2 = demuxes[0]
            elif others:
                a, s2 = others[0]
            else:
                return dict(outcome='stuck', steps=n, trace=tr, state=s)
        else:
            pool = demuxes + others + \
                ([chosen_push[:2]] if chosen_push is not None else [])
            a, s2 = pool[rng.randrange(len(pool))]
        if policy == 'rr' and is_wire_fire(s, a):
            rr_next = push_channel(sk, s, a)[2] + 1
        if trace:
            tr.append(a)
        s = s2
    return dict(outcome='fuel', steps=fuel, trace=tr, state=s)


# -- sigma* (demand-lockstep) ------------------------------------------------

def _closure(sk, ax, mux, s, fuel):
    """Greedy fixpoint under every action EXCEPT wire pushes and wire
    commits (both sides). Demux and all intra-party actions run freely."""
    acts = [a for a in allActions(sk, mux) if not is_wire_commit(a)]
    for _ in range(fuel):
        progressed = False
        for a in acts:
            if is_wire_fire(s, a):
                continue
            s2 = apply(sk, ax, a, s, mux)
            if s2 is not None:
                s = s2
                progressed = True
                break
        if not progressed:
            return s
    return s


def _certificate(sk, ax, mux, s, push, cert, fuel):
    """push = (composite) list of actions realizing one wire push.
    Proven iff after the push the closure empties the pushed channel out
    of the pipe ('exit'), or also out of the slot ('consume')."""
    s2 = s
    for a in push:
        s2 = apply(sk, ax, a, s2, mux)
        if s2 is None:
            return False
    c = push_channel(sk, s, push[-1]) if push[-1][0] != 'walkFire' \
        else None
    # channel of the composite's fire:
    last = push[-1]
    if last[0] == 'iopenFire':
        c = ('w', I, sk.rootH)
    elif last[0] == 'ropenFire':
        c = ('w', R, sk.rootH)
    else:
        c = wireOut(last[1])
    s3 = _closure(sk, ax, mux, s2, fuel)
    if pipe_get(s3, lane_of(mux, c)).count(c) != 0:
        return False
    if cert == 'consume' and s3.ch(c) != 0:
        return False
    return True


def sigma_push_candidates(sk, ax, s, mux):
    """Composite push moves currently mechanically available: commit(+fire)
    fused. Returns list of (label, [actions], channel)."""
    C = mux.C

    def room(c):
        return len(pipe_get(s, lane_of(mux, c))) < C

    out = []
    # iopen
    ic = ('w', I, sk.rootH)
    if s.iopenCh is None and iopenChoosable(ax, s, 'wire') and room(ic):
        out.append((('iopen',), [('iopenChoose', 'wire'), ('iopenFire',)],
                    ic))
    elif s.iopenCh == 'wire' and room(ic):
        out.append((('iopen',), [('iopenFire',)], ic))
    # ropen
    rc = ('w', R, sk.rootH)
    if s.ropenCh is None and ropenChoosable(sk, ax, s, 'wire') and room(rc):
        out.append((('ropen',), [('ropenChoose', 'wire'), ('ropenFire',)],
                    rc))
    elif s.ropenCh == 'wire' and room(rc):
        out.append((('ropen',), [('ropenFire',)], rc))
    # walks
    for pk, ws in s.walk.items():
        if not room(wireOut(pk)):
            continue
        if ws.committed is not None and ws.committed[0] == 'wire' \
                and ws.phase == 2:
            out.append((('walk', pk), [('walkFire', pk)], wireOut(pk)))
        elif ws.committed is None and ws.phase == 2:
            # at most one wire index is choosable (in-order conjunct)
            h = pk[1]
            sc = sk.stageScope(h, ws.scope)
            n = sk.nChildren(h, sc)
            for i in range(n):
                if i not in ws.wireDone:
                    if wkChoosable(sk, ax, pk, ws, ('wire', i)):
                        out.append((('walk', pk),
                                    [('walkCommit', pk, ('wire', i)),
                                     ('walkFire', pk)], wireOut(pk)))
                    break
    return out


def sigma_star_run(sk, ax, C, seed=0, interleave='random', cert='exit',
                   max_steps=None, trace=False):
    """Demand-lockstep run. Free (non-push) actions fire freely; wire
    pushes fire only when certified. Wire COMMITS are fused into pushes
    (never parked unproven).

    Outcomes:
      terminal     -- session complete
      sigma_stuck  -- no free action, no PROVEN push, but >= 1 mechanically
                      available push (the strategy idles forever): the
                      demand-proof bottom-out state H-b asks about
      hard_stuck   -- nothing mechanically available at all (a true
                      structural deadlock: even eager could not move)
      fuel         -- step budget exceeded
    """
    mux = MuxCfg(C)
    rng = random.Random(seed)
    sk_ops = sk.totalOps()
    fuel = max_steps or (8 * sk_ops + 400)
    cl_fuel = 4 * sk_ops + 200
    base_acts = [a for a in allActions(sk, mux) if not is_wire_commit(a)]
    s = init(sk)
    tr = []
    unproven_events = 0
    for n in range(fuel):
        if terminal(sk, s, mux):
            return dict(outcome='terminal', steps=n, trace=tr, state=s,
                        unproven_events=unproven_events)
        free = []
        for a in base_acts:
            if is_wire_fire(s, a):
                continue
            s2 = apply(sk, ax, a, s, mux)
            if s2 is not None:
                free.append((a, s2))
        cands = sigma_push_candidates(sk, ax, s, mux)
        proven = []
        for label, push, c in cands:
            if _certificate(sk, ax, mux, s, push, cert, cl_fuel):
                proven.append((label, push, c))
        if cands and not proven:
            unproven_events += 1
        if not free and not proven:
            if cands:
                return dict(outcome='sigma_stuck', steps=n, trace=tr,
                            state=s, cands=cands,
                            unproven_events=unproven_events)
            return dict(outcome='hard_stuck', steps=n, trace=tr, state=s,
                        unproven_events=unproven_events)
        # choose a move
        moves = [('free', a, s2) for a, s2 in free] + \
                [('push', push, None) for _, push, _ in proven]
        if interleave == 'random':
            kind, x, s2 = moves[rng.randrange(len(moves))]
        elif interleave == 'push_first':
            pushes = [m for m in moves if m[0] == 'push']
            kind, x, s2 = pushes[0] if pushes else moves[0]
        else:  # greedy: free first (allActions order), else first push
            frees = [m for m in moves if m[0] == 'free']
            kind, x, s2 = frees[0] if frees else moves[0]
        if kind == 'free':
            if trace:
                tr.append(x)
            s = s2
        else:
            for a in x:
                if trace:
                    tr.append(a)
                s = apply(sk, ax, a, s, mux)
                assert s is not None
    return dict(outcome='fuel', steps=fuel, trace=tr, state=s,
                unproven_events=unproven_events)


# -- diagnostics -------------------------------------------------------------

def describe_stuck(sk, ax, s, C):
    """Human-readable wait analysis of a stuck mux state."""
    mux = MuxCfg(C)
    lines = []
    for lane, entries in s.pipe.items():
        if entries:
            lines.append(f'pipe[{lane}]: {entries}')
    occ = {c: v for c, v in s.chan.items() if v}
    lines.append(f'occupied channels: {occ}')
    if s.iopenCh or not (s.iopenWire and s.iopenQuery):
        lines.append(f'iopen: wire={s.iopenWire} query={s.iopenQuery} '
                     f'committed={s.iopenCh}')
    if s.ropenCh or not s.ropenRes:
        lines.append(f'ropen: got={s.ropenGotWire} wire={s.ropenWire} '
                     f'res={s.ropenRes} q={s.ropenQ} committed={s.ropenCh}')
    for pk in sk.walkKeys():
        ws = s.walk[pk]
        if ws.phase != 5:
            h = pk[1]
            sc = sk.stageScope(h, ws.scope) if ws.scope < sk.stageLen(h) \
                else None
            lines.append(
                f'walk{pk}: scope#{ws.scope}(id {sc}) phase={ws.phase} '
                f'committed={ws.committed} wireDone={sorted(ws.wireDone)} '
                f'resDone={sorted(ws.resDone)} qSent={ws.qSent} '
                f'parent={ws.parentDone}')
    for pk in sk.asmKeys():
        a = s.asm[pk]
        if a[1] != 4:
            lines.append(f'asm{pk}: idx={a[0]} phase={a[1]} got={a[2]}')
    if s.absorbPhase != 5 and sk.totalLeafReqs() > 0:
        lines.append(f'absorb: idx={s.absorbIdx} phase={s.absorbPhase}')
    return '\n'.join(lines)
