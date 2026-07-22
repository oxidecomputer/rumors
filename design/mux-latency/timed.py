"""RTT-cost semantics for the phase-2 probe: critical-path completion time.

This is ALGEBRA-CHECKING for formal/MUX-LATENCY.md, not benchmarking: the
model is the phase-2 probe's transcription of StreamingMirror (model.py,
copied with the [L1]/[L2] marks), and the only addition is a clock.

Timing rules (MUX-LATENCY.md section 1):
  - a push is instantaneous; the pushed frame ARRIVES (becomes deliverable
    at the receiver's demux) exactly `delta` later (delta = 1/2 RTT = 1
    time unit);
  - a pipe/lane entry occupies its lane from push until delivery, so lane
    capacity C is the in-flight message window (C = 1 is per-lane
    stop-and-wait);
  - every intra-party action is free (fires at the current instant);
  - completion time = the clock value at Terminal, in units of delta
    (i.e. in one-way hops, the vocabulary of
    design/streaming-latency-serialization.md).

Constructions (strategy x lanes):
  - baseline  : lanes='stream', C large, eager pushes — the fully
    independent link-transport construction (per-stream windows).
  - stream_w1 : lanes='stream', C=1 — the per-stream stop-and-wait floor
    (the W = 1 credit model), for calibration.
  - sigma_omni: lanes='direction', the probe's omniscient exit-certificate
    sigma* (no informational lag; validates the capacity/structural terms
    only).
  - sigma_causal: sigma_omni AND the label-arrival gate — the causal
    A_p-limited sigma* proxy: a push of frame k on stream (p,h) is
    permitted only when every reverse-direction frame covering the
    consumer's scopes 1..k-2 has been DELIVERED to p. Soundness of the
    gate as a proxy: sigma*'s closures never cite an unperformed push
    (self-containment, attack-refute.md section 4.5), so the consumer's
    scope-j wire publications enter a demand proof only via C-arr =
    physical arrival; and the labels the derivation needs (asked-quota
    counts) ride exactly those frames (attack-refute.md section 2). The
    gate is therefore a NECESSARY condition on the causal sigma*'s pushes;
    conjoined with the omniscient certificate it is the tightest
    over-approximation available without implementing the full closure
    (stage-0 P1). Where the gate wedges, causal sigma* wedges; where it
    flows, causal sigma* flows no faster.
  - oracle    : lanes='direction', pushes in a fixed per-direction order
    pi_d = the wire-receive order of the unmuxed greedy drain (a proxy for
    demandOrder/scheduleE's receive projection: any valid consumption
    order, per-channel-sequential by construction).
"""

import model
from model import (I, R, other, apply, allActions, terminal, init,
                   MuxCfg, lane_of, lane_list, pipe_get, wireIn)
from mux import (is_wire_commit, is_wire_fire, sigma_push_candidates,
                 _certificate)

BIG = 10 ** 9


def prefix_kids(sk, h, j):
    """Frames on the reverse stream ('w', other(p), h-1) that cover the
    consumer's scopes 1..j on stream (p, h): the flattened child count of
    the first j scopes at height h (wf_bfs_aligned makes the flattening
    exact). At h = 1 the children are leaf requests."""
    return sum(sk.nChildren(h - 1, s) for s in sk.scopesAt(h)[:j])


def consumption_order(sk, ax):
    """pi_d proxy: per-direction wire-channel receive order of the unmuxed
    greedy drain (first enabled action in allActions order)."""
    s = init(sk)
    acts = allActions(sk, None)
    order = {I: [], R: []}
    fuel = 6 * sk.totalOps() + 400
    for _ in range(fuel):
        fired = None
        for a in acts:
            s2 = apply(sk, ax, a, s, None)
            if s2 is not None:
                fired = a
                s = s2
                break
        if fired is None:
            break
        c = None
        if fired[0] == 'ropenRecv':
            c = ('w', I, sk.rootH)
        elif fired[0] == 'walkRecvWire':
            c = wireIn(fired[1])
        elif fired[0] == 'absorbRecvWire':
            c = ('w', R, 0)
        if c is not None:
            order[c[1]].append(c)
    assert terminal(sk, s, None), 'unmuxed drain did not complete'
    return order


def timed_run(sk, ax, strategy='eager', C=1, lanes='direction', delta=1,
              wide=True, oracle_order=None, max_iter=200000, park=1):
    """Event-driven critical-path run. Returns dict(outcome, t, pushes).

    `park` is sigma*_K's receiver parking depth K ([L3] in model.py): the
    wire slots hold up to K decoded replies per stream, and the causal
    gate's arrears distance generalizes from k-2 to k-K-1 (the sender may
    run K unproven replies past demand proof per stream,
    design/single-socket.md section 3.2). park=1 is the sigma* of record."""
    model.WIDE_INTERNAL = wide
    model.PARK = park
    try:
        return _timed_run(sk, ax, strategy, C, lanes, delta, oracle_order,
                          max_iter, park)
    finally:
        model.WIDE_INTERNAL = False
        model.PARK = 1


def _timed_run(sk, ax, strategy, C, lanes, delta, oracle_order, max_iter,
               park):
    mux = MuxCfg(C, lanes)
    s = init(sk)
    arr = {}                 # lane -> arrival times, parallel to s.pipe
    pushed = {}              # chan -> frames pushed so far
    delivered = {}           # chan -> frames delivered so far
    dir_pushed = {I: 0, R: 0}
    t = 0
    npush = 0
    free_acts = [a for a in allActions(sk, mux)
                 if a[0] != 'demux' and not is_wire_commit(a)]
    all_lanes = lane_list(sk, mux)
    cl_fuel = 4 * sk.totalOps() + 200

    def permitted(push_acts, c):
        if strategy == 'eager':
            return True
        if strategy == 'oracle':
            d = c[1]
            seq = dir_pushed[d]
            return seq < len(oracle_order[d]) and oracle_order[d][seq] == c
        if strategy == 'sigma_causal':
            h = c[2]
            k = pushed.get(c, 0) + 1
            if 1 <= h <= sk.rootH - 1 and k >= park + 2:
                rev = ('w', other(c[1]), h - 1)
                if delivered.get(rev, 0) < prefix_kids(sk, h, k - park - 1):
                    return False
        # sigma_omni and sigma_causal both require the exit certificate
        return _certificate(sk, ax, mux, s, push_acts, 'exit', cl_fuel)

    for _ in range(max_iter):
        progressed = True
        while progressed:
            progressed = False
            # ripe deliveries (head-of-line per lane)
            for lane in all_lanes:
                pl = s.pipe.get(lane)
                while pl and arr[lane] and arr[lane][0] <= t:
                    head = pl[0]
                    s2 = apply(sk, ax, ('demux', lane), s, mux)
                    if s2 is None:
                        break        # slot full: HOL wait
                    s = s2
                    pl = s.pipe.get(lane)
                    arr[lane].pop(0)
                    delivered[head] = delivered.get(head, 0) + 1
                    progressed = True
            # free intra-party actions
            for a in free_acts:
                if is_wire_fire(s, a):
                    continue
                s2 = apply(sk, ax, a, s, mux)
                if s2 is not None:
                    s = s2
                    progressed = True
            # strategy-permitted pushes
            for _label, push_acts, c in \
                    sigma_push_candidates(sk, ax, s, mux):
                if not permitted(push_acts, c):
                    continue
                ok = True
                for a in push_acts:
                    s2 = apply(sk, ax, a, s, mux)
                    if s2 is None:
                        ok = False
                        break
                    s = s2
                if ok:
                    arr.setdefault(lane_of(mux, c), []).append(t + delta)
                    pushed[c] = pushed.get(c, 0) + 1
                    dir_pushed[c[1]] += 1
                    npush += 1
                    progressed = True
        if terminal(sk, s, mux):
            return dict(outcome='terminal', t=t, pushes=npush)
        future = [q[0] for lane, q in arr.items()
                  if q and s.pipe.get(lane) and q[0] > t]
        if not future:
            return dict(outcome='stuck', t=t, pushes=npush, state=s)
        t = min(future)
    return dict(outcome='iter', t=t, pushes=npush)


def hops(sk, ax, strategy, C, lanes, wide=True, oracle_order=None, park=1):
    """Completion time in one-way hops (delta = 1); asserts termination."""
    r = timed_run(sk, ax, strategy=strategy, C=C, lanes=lanes, wide=wide,
                  oracle_order=oracle_order, park=park)
    if r['outcome'] != 'terminal':
        return ('!' + r['outcome'], r['t'])
    return r['t']
