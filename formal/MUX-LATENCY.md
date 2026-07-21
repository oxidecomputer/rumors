# The latency price of σ*: round trips vs independent links

Latency analyst, mux-conjectures campaign (2026-07-21). Question under
analysis, verbatim from the charter: *what is the real latency cost of the
σ\* scheduler, if it were to be implemented — the expected number of round
trips compared with the fully independent link construction?* This is an
analytic derivation with a step-counted simulation check, not a benchmark
of real code. Inputs: MUX-ADJUDICATION.md (σ* of record, the H-c demotion,
`ofSchedule`), mux-notes-phase2/refute-c1.md §2/§4 (claims re-examined
here, not assumed), attack-refute.md F5/F9 (both resolved below), MODEL.md
§2–§6, Mux/Basic.lean's module doc, and
`design/streaming-latency-serialization.md`, whose vocabulary (one-way
*hops*, `hops = (T(delay) − T(0)) / delay`, V1-vs-V2 framing) this
document adopts so the two compose.

Epistemic key as in PROGRESS.md: **[derived]** paper argument with stated
assumptions; **[checked]** validated by the RTT-costed probe in
`mux-notes-phase2/latency/` (step-counted simulation of the phase-2 model,
§5 — algebra-checking, not benchmarking); **[checked, in-repo]** measured
numbers already committed in `design/streaming-latency-serialization.md`;
**[open]** known unknown.

---

## 0. Verdict up front

Let L be the dispute depth (ladder levels below the root), W\* the width
of the widest *fresh-dispute frontier* (the maximum, over heights, of the
number of child-bearing scopes at that height), N the total scope count,
F the per-direction frame count, δ = RTT/2 one one-way hop.

1. **Baseline (independent links, shipped windows): (L + 2)·δ** — depth
   only, no width term [derived; checked exactly on every probe shape].
2. **σ\* at C = 1: ≈ max-direction F·δ ≈ Θ(N)·δ** — the single
   message-counted in-flight slot is stop-and-wait; this cost is
   *scheduler-independent* (the oracle pays it too, probe-identical).
   "C₀ = 1 suffices" is a liveness fact with a Θ(N/L)× latency price
   [derived + checked].
3. **σ\* at C = ∞ (capacity removed, informational lag isolated):
   ≈ (L + 2 + W\*)·δ** — the causal demand proof paces every
   fresh-dispute stream at ~2 frames per RTT, and levels pipeline, so the
   widest frontier level is paid once, nearly in full [derived; checked
   to ±1 hop on every shape]. σ\* therefore does **not** stay
   Θ(depth·RTT) with a worse constant: completion degrades to
   Θ(W\*·RTT), i.e. Θ(#scopes·RTT) on breadth-heavy skeletons.
4. **Multiplier vs the baseline: 1 + W\*/(L + 2), unbounded in width.**
   Measured on the campaign's own random-skeleton distribution: mean
   **1.84×**, max 4.8× at C = ∞; mean **4.6×**, max 21× at C = 1
   [checked]. Projected onto the benchmark divergence shape
   (I = 5000: full 256-slot dispute frontier, L ≈ 5): ≈ **260 hops vs 8**,
   ≈ 33× — ≈ 26 s vs 0.85 s at 100 ms one-way [derived from the checked
   law]. σ\* would reintroduce the same hops ∝ disputed-scopes pathology
   (§2 of the latency doc: 246–647 hops, 65 s sessions) that
   `Peer::max_in_flight_nodes` was landed to remove.
5. **Where σ\* is free:** chains, provision walls, and the historical
   wedge cost σ\* *nothing* beyond the baseline at C ≥ 3–4 in the
   message-counted model (wedge: 8 hops = baseline 8 at C = 4; 9 at
   C = 2) [checked] — plus a
   bandwidth head-of-line term B the message-counted model erases
   [derived, §3.3]. The shape that motivated the deadlock campaign is
   exactly the shape σ\* handles gracefully; the price lives in the
   opposite corner (wide fresh-dispute frontiers), which is the *typical*
   content-hash gossip shape.
6. **H-c ("the price is steep"): survives, relocated, and made
   shape-conditional.** Not uniform per-stream lockstep (that intuition
   dies: silent runs pipeline unboundedly, F5 confirmed); not a constant
   factor on depth·RTT (that claim dies: F9 resolved as a
   descent-vs-completion conflation, §3.1); but a width-proportional term
   that on realistic divergence is one to two orders of magnitude.

---

## 1. The latency model

**RTT-dominant, computation-free, message-counted, with bandwidth as an
explicit secondary parameter.** Rules, applied identically to every
construction:

- A frame **arrives** exactly δ = RTT/2 after its push; pushes are
  instantaneous; every intra-party action is free. Completion time is the
  clock at Terminal, denominated in **hops** (multiples of δ) — the same
  quantity `streaming-latency-serialization.md` measures as the slope of
  session time in one-way delay [model definition].
- **"A round trip" per construction.** Baseline/V1: 2 hops on the wire.
  σ\*: additionally the unit of the *demand-proof cycle* — a pushed frame,
  its consumer's reverse-direction publication, and that publication's
  arrival back at the sender, which is the cycle the causal proof rides
  (§3.1). Oracle: 2 hops; it has no proof cycle.
- **Capacity C is the in-flight message window per pipe** (per direction
  for the mux, per stream for independent links): a pipe entry occupies
  its lane from push to delivery, so a frame holds a lane slot for ≥ δ
  and C = 1 is stop-and-wait at message granularity. This is the honest
  timing reading of the harness of record's `push`/`deliver` actions
  (Mux/Basic.lean: the pipe is the transport; `deliver` moves head to
  slot) [model decision; the alternative reading — C as a costless
  sender-side queue — would make `deliver` free and the pipe fiction].
- **Bandwidth.** Message counting is primary, per the charter. Bytes
  enter as an additive head-of-line term: B(x) = (bytes queued ahead of
  frame x in its pipe)/bandwidth. This is exactly the unit-mismatch
  (MUX-ADJUDICATION §2.5: one "message" = one reply of unbounded bytes,
  §5A's W = 1 argument), so every mux number below is a **lower bound**
  on the byte-real cost; independent links bound B(x) by x's own stream's
  backlog instead of the whole direction's [derived]. §3.3 locates where
  B is large.
- **What the model erases** and why it is safe here: compute (measured
  in-repo at 9–30 ms/session vs multi-second stall terms at the delays of
  interest — it moves totals, not multipliers [checked, in-repo]); the
  handshake prefix (identical constant for every construction; the Rust
  ledger's `hops = 2 + L + 1` and the model's L + 2 differ by that
  constant only).
- **Internal channel regime.** The base model's intra-party channels are
  cap-1 — the K = 1 floor whose serialization the latency doc diagnosed
  and the shipped `Peer::max_in_flight_nodes` window removed. All
  headline numbers use the **wide-window regime** (internal channels
  non-blocking), because that is the shipped configuration and the only
  regime in which the baseline actually achieves Θ(depth) — i.e. the only
  regime where the *transport* delta is visible. The floor regime is
  reported once (§4, second table) because it explains the panel's
  fair-rounds 0.99× artifact: at the floor, the protocol's own internal
  serialization (Θ(N) hops for everything, including the baseline) masks
  the mux delta entirely. A message-counted, latency-free, floor-regime
  metric was structurally incapable of pricing σ\* — the H-c demotion,
  made quantitative [checked: floor pyr3 row, base = σ\* = 58].

Parameters carried throughout: depth L (= rootH − deepest populated
height), per-height scope counts n_h, fresh-dispute frontier widths W_h
(child-bearing scopes at height h; W\* = max_h W_h), per-direction frame
counts F_d (opening + Σ n_h over that parity), dispute density q and fan
f for the expected case, provision volume V for B-terms.

---

## 2. Completion-time laws

### 2.1 Fully independent links (the link-transport baseline)

    T_base = (L + 2)·δ  +  B_stream                                [derived]

Opening hop, root reply, then one hop per descended level: consecutive
levels' replies flow in opposite directions (parity alternation, MODEL.md
§3), so each level adds exactly one crossing; per-stream windows ≥ BDP
and the shipped internal window make everything else overlap. No width
term, no dispute-count term. Assumptions: per-stream in-flight window ≥
the level's frame volume; internal pipeline window ≥ frontier (the
shipped default). B_stream = only the frames' own stream backlog.

[checked]: chain6 = 8 = L+2, combW8/16 = 5, pyr2/pyr3 = 6, wedge = 8 —
exact on all nine shapes. Corresponds to the in-repo measured
`hops = 2 + L + 1` (same ladder term, handshake constant differs)
[checked, in-repo].

### 2.2 σ\* (demand-lockstep, causal) at capacity C

**The pacing law** [derived]. For stream c = wire(p, h) with consumer
W′ = Walk(¬p, h−1): σ\* pushes frame (c, k) only when rcv(c, k−1) ∈
Certified ∪ Inevitable. For k ≥ 3 that derivation must carry W′ through
scope k−2's complete publication set (E3: the scope-(k−1) prologue recv
sits after it). Two independent reasons pin that to a *physical reverse
arrival* at p:

1. **Self-containment** (attack-refute §4.5): the closures never cite an
   unperformed push by either side, and W′'s scope-(k−2) publications
   include its wire frames (whenever scope k−2 has children) — peer
   pushes, admissible only via C-arr, i.e. after they arrive at p.
2. **Label transport** (attack-refute §2): the asked-quota counts the
   derivation needs for scope k−2 are minted by ¬p and ride exactly those
   same frames.

W′ publishes scope k−2 no earlier than δ after p pushed (c, k−2), and
the publication takes another δ to return, so:

    T_push(c, k)  ≥  T_push(c, k−2) + RTT      whenever scope k−2 is
                                               child-bearing            (†)

— **two frames per RTT per fresh-dispute stream**, with frames whose
scope-(k−2) predecessor is childless (provision runs, all-M/childless-D
scopes, leaf boundaries) exempt: those consumptions are I-step-derivable
with zero reverse evidence, and σ\* streams them at pipe speed. This
replaces refute-c1 §3's "per-stream in-flight ≤ 2" side-claim with the
correct statement (attack-refute F5, resolved): in-flight is bounded by
the slot plus the *forward-derivable silent horizon*, which is unbounded
on silent runs and exactly 2 on fresh-dispute runs.

**Completion** [derived; checked]. Define the paced-frame count of a
stream, P_h = #{k : 3 ≤ k ≤ n_h, scope k−2 at height h is
child-bearing} (≈ W_h − 2, clamped at 0). Levels trickle concurrently
(the wavefront pipelines: level h's frame k enables level h−1's frames
one hop later), so pacing is paid at the *widest* level, nearly alone:

    T_σ*(C = ∞)  ≈  (L + 2)·δ  +  (max_h P_h)·δ  +  ε        ε ≤ O(L)·δ
    T_σ*(C)      ≈  max( T_σ*(∞),  (max_d F_d)·δ / C  +  overlap slack )
                  +  B_pipe                                     [derived]

[checked, exact to ±1 hop]: combW8 = 5+6 = 11, combW16 = 5+14 = 19,
pyr2 = 6+6 = 12, pyr3 = 6+25 = 31 predicted, 32 measured (ε = +1, the
cross-level coupling); chain/dfan/comb6: max P_h = 0 ⇒ = base, measured
= base; wedge/provwall: max P_h = 1 (the first provision behind the
dispute head), law +1, measured +0 — an isolated paced frame's wait runs
concurrently with the descent ladder and is absorbed. The width term is
tight when the frontier is the bottleneck and an over-estimate by up to
O(L) when it is not; both signs of slack are ≤ 1 hop on every probe
shape. C-dependence [checked]: C = 4 already ≈ C = ∞ on every standard
shape (σ\* never uses more window than its proof frontier — refute-c1
§3's flatness claim survives *for C ≥ 2·active streams*); C = 1 is the
stop-and-wait floor: pyr3 = 93 ≈ F_R = 91.

Worst case over skeletons at fixed N: an all-D two-level broom
(one frontier level of width Θ(N)): T = Θ(N)·δ against baseline
Θ(1)·δ — the maximizing shape (§3.1). Best case: any skeleton with
max P_h = 0 (chains, provision walls, invisible scopes): T = T_base
exactly [checked].

**Caution on the sign of the check.** The probe's causal gate is a
*necessary* condition on causal-σ\* pushes conjoined with the omniscient
exit certificate (§5), i.e. an optimistic over-approximation: true causal
σ\* can only be **slower** than the [checked] numbers, and the (†)
recurrence bounds it above at 2× the paced term (1 RTT per paced frame,
reached on the alternating-parity family where the reverse frames are
themselves proof-lagged). All multipliers below are therefore lower
bounds with a ×2-on-the-width-term ceiling [derived].

### 2.3 The oracle (`ofSchedule (π_d)`) at capacity C

    T_oracle  ≈  (L + 2)·δ  +  (max_d F_d)·δ / C  (throughput)
              +  S_π  (linearization slack)  +  B_pipe            [derived]

No proof cycle: the oracle pushes when committed, π-front, and in-window.
Two structural costs remain. (i) **The window**: at C = 1 it pays the
same per-direction stop-and-wait as σ\* — [checked]: oracle C=1 within
±1 hop of σ\* C=1 on all nine shapes (pyr3: 94 vs 93). Nonlocal
information does not buy back the window; C₀ = 1 is liveness-only.
(ii) **Linearization rigidity S_π**: `ofSchedule` enforces one fixed
total order on pushes; where the true DAG leaves receive events
unordered across streams, a π-late frame that is production-*early*
waits for π-earlier frames that are production-late. S_π = 0 on
narrow/serial shapes [checked: = base on chain, wedge, provwall, combW];
real on dense trees [checked with the proxy π: pyr2 = 12, pyr3 = 22 vs
base 6]. Magnitude is π-dependent — the probe's π is the greedy drain's
receive order, not `scheduleE`'s projection — so S_π's exact value for
the π_d of record is [open]; its existence for *any* fixed linearization
of an under-constrained DAG is structural [derived]. Bounded below by
T_base.

At C = ∞ and byte-blind, the oracle is the only muxed construction that
matches the baseline on the standard shapes — the "oracle-grade overlap"
of the adjudication — but it still shares one byte pipe: B_pipe stands.

### 2.4 V1 alternating (the serialized calibration floor)

    T_V1  ≈  2·(L + 1)·δ  +  Σ_levels bytes_ℓ / bw                [derived]

One whole-frontier exchange per level: 2 hops each, no width term ever.
Consistent with the in-repo measurement (8.0–9.0 hops at effective depth
3–4, invariant in divergence) [checked, in-repo]. The calibration it
provides: **σ\* on wide frontiers is worse than V1** — pyr3: σ\* 32 vs
V1 ≈ 10; the benchmark shape: σ\* ≈ 260 vs V1 = 9 [derived from the
checked laws] — the muxed streaming protocol under demand-lockstep loses
to the protocol streaming was built to replace, on exactly the workloads
that motivated streaming.

### 2.5 Expected case

Distribution: the campaign's own random-skeleton generator (gen.py,
unchanged): rootH ∈ {4, 4, 6}, fan ~ U[2, 7], interior children D w.p.
0.55 (0.45 at height 2, leafReqs ~ U[0, fan]), kid counts U[0, fan] —
a subcritical-to-critical dispute branching process, i.e. small trees
(N ≈ 5–60). First-moment law [derived]:

    E[multiplier]  ≈  1 + E[max_h P_h] / (L + 2)
    with n_{h−1} = n_h · f̄·q̄  and  W_h ≈ n_h · (1 − (1 − q̄)^f̄)

Measured over 40 seeds [checked]:

| construction | mean ×base | max ×base |
|---|---|---|
| σ\*-causal, C = ∞ | **1.84** | 4.83 |
| σ\*-causal, C = 1 | **4.57** | 21.3 |

The pool's smallness is load-bearing for reading these: the multiplier
law is linear in frontier width, and this distribution rarely mints
W\* > 15. Extrapolated to the benchmark fixture's geometry (all 256 root
slots disputed and child-bearing, L ≈ 5, order 10³ scopes):
σ\*(C = ∞) ≈ (8 + ~250)·δ ≈ 260 hops ≈ 130 RTT vs baseline 4 RTT
(≈ 33×; ≈ 26 s vs 0.85 s at 100 ms one-way); σ\*(C = 1) ≈ F_d·δ, order
700 hops (≈ 70 s) [derived from the checked laws, order-of-magnitude;
the same class as the measured K = 1 pathology of 647 hops / 64.8 s that
motivated the window fix — [checked, in-repo]].

---

## 3. Where σ\* waits: the penalty, located

Three mechanisms, in decreasing order of blame.

### 3.1 The fresh-dispute frontier (informational: the only σ\*-specific term)

**Situation:** consecutive child-bearing scopes on one stream — the
demand proof for frame k cannot close until the consumer's scope-(k−2)
publications have physically returned (both the push-evidence and the
labels ride them, §2.2).
**Cost:** 1 RTT per 2 paced frames (best case, reverse frames prompt) to
1 RTT per paced frame (worst case, alternating-parity chains where the
reverse frames are themselves lagged) [derived; lower edge checked].
**Frequency:** every frame k ≥ 3 whose (k−2)-predecessor scope is
child-bearing — on uniform content-hash divergence, essentially every
frame of every interior level.
**Compounding — the crux, answered:** levels do *not* compound
additively; the wavefronts pipeline and the *widest* level is paid ≈ once
[checked: pyr3 pays 25 of its Σ P_h = 38, within 1 hop of the
single-widest-level prediction]. But that is no comfort: the widest level
of a geometric dispute tree carries Θ(N) scopes, so completion is
**Θ(#scopes·RTT), not Θ(depth·RTT)** — σ\* degrades in class, not in
constant. The critical *descent* (time-to-deepest-leaf) does stay
Θ(depth·RTT) with constant ≤ 2; refute-c1 §4.4's "constant factor on
depth·RTT" conflated that descent with session completion, which is what
attack-refute F9 flagged as unmetered. F9 is resolved: the claim was
wrong for completion, right for the descent [derived + checked].
**Maximizing shape:** the wide shallow comb/broom — one level of m
child-bearing D scopes (combW16: 3.8× at m = 16; multiplier
≈ 1 + (m−2)/(L+2), unbounded) [checked]. **Matching shape:** chains
(every frame is seq ≤ 2 on its stream: zero paced frames; σ\* = baseline
at every C ≥ 1, exactly) [checked].
**Decomposition** [checked]: the probe's *omniscient* σ\* (global-state
certificate, no locality) splits the width term almost exactly in half —
combW8: base 5 → omniscient 8 → causal 11; pyr3: 6 → 19 → 32. Half the
price is demand-gating itself (even a scheduler that *sees* the peer's
state must wait ~½ RTT per paced frame for the consumer's real
progress); the other half is locality (waiting for the proof's carrier
frames to physically arrive). So free consumption *evidence* — credits
at window 1, the adjudication's "third thing" — buys back only the
locality half; removing the whole term takes a consumption-independent
push license: a credit window wider than the frontier, or independent
streams, or work-conservation plus the deadlock it buys.

### 3.2 The shared window (structural: shared with every single-pipe scheduler)

**Situation:** all streams of a direction share one C-message in-flight
window; each frame occupies a slot for the full transit δ.
**Cost:** (F_d/C − concurrency)·δ; at C = 1, ≈ F_d·δ — stop-and-wait.
**Frequency:** every frame, C-independent of shape.
**Compounding:** linear in total traffic, by construction.
This term is *scheduler-independent* — the omniscient σ\*, the causal
σ\*, and the oracle land within ±1 hop of each other at C = 1 on every
shape [checked] — so it is a property of the single-pipe design point,
not of demand-lockstep. Any real σ\* deployment would need C ≈ the
frontier width in *messages*, which in bytes is unbounded (the
unit-mismatch): the window sizing problem credits/independence solve
reappears untouched [derived].

### 3.3 Byte head-of-line and the wedge (bandwidth: the erased term)

**Situation:** supplies queued behind — or ahead of — an unresolved
dispute in the one byte-serial pipe. In the message-counted model the
wedge is *free* for σ\* (positional inevitability of the provisions is
derivable with zero reverse evidence; the walk consumes them; measured =
baseline at C ≥ 4, +5δ at C = 1) [checked]. What the model erases: the w
provision frames are byte-unbounded, and every later frame of that
direction — including the deep descent's — waits B ≈ w·V/bw behind them.
Independent links delay only the provisions' own stream and let the
descent's (tiny) frames interleave at fair share [derived; not
probe-checkable in a message-counted model — this is the H-c
unit-mismatch, inherited, not resolved].
**Frequency:** wherever provision volume shares a direction with live
dispute traffic — the wedge shape, bootstrap-shaped syncs.
**Note the inversion:** the shape that historically deadlocked the
work-conserving mux is the shape where σ\*'s *RTT* price is zero and the
whole residual price is B. σ\* genuinely fixes the wedge; it loses
elsewhere.

The positional-inevitability *blocking* variant of the wedge (provisions
unprovable until an earlier sibling dispute's publications complete)
exists only in the internal-floor regime, where the walk parks mid-scope;
at the shipped window it does not occur [checked: wedge floor rows σ\*
C1 = 12 vs wide 13, both dominated by the window term, neither by
blocking].

---

## 4. Standard shapes: closed forms and probe values

Hops (δ = 1), wide-window regime, `.impl`, margin-0. "law" = the §2
closed form; probe values from `latency_results.json` [checked]. V1 =
2(L+1) [derived]. Multipliers in parens are ×baseline.

| shape | N | L | W\*/paced | base (law=probe) | V1 | σ\* C=1 | σ\* C=∞ (law) | oracle C=1 | oracle C=∞ |
|---|---|---|---|---|---|---|---|---|---|
| chain6 | 6 | 6 | 0 | 8 | 14 | 8 (1.0×) | 8 = 8 | 8 | 8 |
| wedge (fan 7, 6 prov) | 12 | 6 | 1† | 8 | 14 | 13 (1.6×) | 9 → 8 (+B) | 12 | 8 (+B) |
| provwall8 | 12 | 4 | 1† | 6 | 10 | 13 (2.2×) | 7 → 6 (+B) | 12 | 6 (+B) |
| dfan8 | 10 | 2 | 0 | 4 | 6 | 11 (2.8×) | 4 = 4 | 11 | 4 |
| comb6 | 8 | 6 | 0 | 8 | 14 | 9 (1.1×) | 8 = 8 | 9 | 8 |
| combW8 | 18 | 3 | 6 | 5 | 8 | 12 (2.4×) | 11 = 11 (2.2×) | 12 | 5 |
| combW16 | 34 | 3 | 14 | 5 | 8 | 20 (4.0×) | 19 = 19 (3.8×) | 20 | 5 |
| pyr2 | 31 | 4 | 6 | 6 | 10 | 23 (3.8×) | 12 = 12 (2.0×) | 24 | 12* |
| pyr3 | 121 | 4 | 25 | 6 | 10 | 93 (15.5×) | 31 ≈ 32 (5.3×) | 94 | 22* |

\* oracle C = ∞ on dense trees: S_π under the proxy linearization; π_d of
record may differ (§2.3, [open]).
† law +1 (one paced frame), absorbed by the ladder in the probe (§2.2):
"law → probe".

The floor regime (internal channels at the model of record's cap-1) shows
why no message-fair metric saw this coming [checked, same sweep]:

| shape (floor regime) | base | σ\* C=1 | σ\* C=∞ |
|---|---|---|---|
| combW8 | 11 | 12 | 11 |
| combW16 | 19 | 20 | 19 |
| pyr3 | 58 | 94 | 58 |

The baseline itself degrades to Θ(N) hops (the K = 1 internal
serialization the latency doc diagnosed in the Rust) and σ\* at C = ∞
lands *exactly equal* to it: the protocol's own floor saturates the
critical path and hides the transport delta entirely. The fair-rounds
0.99× [checked, probe §5] was measured in that blind spot.

Reading the C = 1 column downward: every shape pays ≈ max(F_d)·δ
regardless of scheduler — wedge 13/12, dfan 11/11, pyr3 93/94 for
σ\*/oracle. Reading the C = ∞ column: only the frontier shapes pay, and
they pay the paced-frame law exactly.

---

## 5. The [checked] tier: harness and its honest limits

`formal/mux-notes-phase2/latency/`: `model.py`, `gen.py`, `instances.py`,
`mux.py` are the phase-2 probe copied verbatim with two marked
modifications ([L1] widen-internal flag, fixing three transcription sites
that hardcoded the base model's cap-1 in fire guards; [L2] pipes keyed
per direction or per stream); `timed.py` adds the §1 clock; `run_latency.py`
is the sweep (`python3 run_latency.py`, deterministic,
writes `latency_results.json`). This validates *algebra*: the same model
the panel argued over, plus a clock — no Rust was run.

- **Calibration gate:** the chain completes in exactly rootH + 2 hops
  under every construction at every C [checked, asserted in the sweep].
- **The causal σ\* proxy.** The probe's σ\* certificate is omniscient
  (probe caveat 3 / F4 — the causal closure remains unimplemented,
  stage-0 P1). This harness gates it with the *label-arrival* condition:
  frame k on (p, h) needs every reverse frame covering the consumer's
  scopes ≤ k−2 delivered to p (`prefix_kids`, exact by wf_bfs_aligned).
  That condition is provably necessary for the causal σ\* (self-
  containment + label transport, §2.2), so measured times are **lower
  bounds** on causal σ\*; the (†) recurrence caps the true value at
  2× the width term. The headline multipliers survive both ends of the
  band. If stage-0's real causal σ\* lands *outside* [measured,
  measured + width-term], §2.2's derivation is wrong and this document
  must be revised — that is the falsifiable handle.
- **The oracle proxy.** π_d = the unmuxed greedy drain's receive order,
  not `scheduleE`'s projection. Per-channel it is the same sequence
  (consumption order is positional); across channels it may differ, so
  S_π values are indicative, its existence argument is §2.3's [derived].
- **Discrepancies found and disposition:** pyr3 σ\* C=∞ measured 32 vs
  law 31 — cross-level coupling inside the stated ε ≤ O(L) slack; no
  other shape deviates. An earlier draft of the law charged Σ_h P_h
  (additive compounding); the probe refuted it (pyr3: 38 predicted, 32
  measured) and the pipelined form replaced it. That is the process
  working as intended.
- **Not validated here:** B-terms (message-counted model, §3.3);
  byte-denominated C; compute; the Rust transport. A hop-metered Rust
  measurement (the latency doc's §1 instrument pointed at a
  single-stream link) is the natural next rung if anyone proposes
  actually shipping a mux [open].

---

## 6. Conclusion

**The sentence Finch asked for:** against the fully independent link
construction, σ\* costs — in round trips, computation-free, bytes set
aside — **nothing on chains, provision walls, and the historical wedge;
an expected ≈ 1.8× (max ≈ 5×) on the campaign's small random skeletons;
and Θ(frontier-width/depth)× on fresh-dispute breadth, which is
unbounded and lands at roughly 30× (≈ 130 RTT vs 4) on the benchmark's
uniform-divergence shape even at infinite pipe capacity — and at the
adjudicated C = 1 it is stop-and-wait for *every* scheduler, ≈ Θ(N/L)×,
oracle included** [derived; lower-bound-checked].

**H-c adjudication.** "The price is steep" **survives shape-conditionally
and dies as stated.** It dies on the wedge — the empirically motivating
shape — where σ\* is RTT-free and only the bandwidth head-of-line term
remains; it survives, sharpened, on fresh-dispute breadth, where the
price is not a constant factor but a class change (depth·RTT →
scopes·RTT). The panel's instinct that the model *couldn't* price this
(H-c demoted to executable tier) was correct twice over: message-fair
rounds at the internal floor sit in a measured blind spot where baseline
and σ\* coincide (§4), and the byte term is erased by construction. The
"mysterious third thing" reading also gains a quantitative face: what
credits/independence buy over σ\* is exactly one reverse-arrival per two
frontier scopes — W\*/2 round trips a session — plus byte-HOL isolation.

**What this says about link-transport.** The independent-streams contract
is not a convenience the right scheduler could replace: among all
constructions over one FIFO per direction, the *omniscient* scheduler
with *unbounded* window and a production-respecting linearization is the
minimum needed to match independent links in the RTT dimension — and it
still shares one byte-serial pipe (§3.3). σ\* — the best *local*
scheduler the campaign found, and a genuine refutation of C1-literal —
recovers liveness but reintroduces the hops ∝ disputed-scopes latency
class that `max_in_flight_nodes` was shipped to eliminate, losing even to
V1 on wide frontiers. Deadlock-freedom was never the expensive part of
the mux; overlap is. The link contract (independent, lazily opened,
per-stream flow control) buys exactly the three terms σ\* cannot:
zero proof-lag, per-stream windows, and per-stream byte ordering — which
is the deadlock doc's design argument, now with its latency half
quantified [derived + checked].

**Residual items:** stage-0 P1's real causal σ\* should be run under this
harness's clock to collapse the [measured, +width]-band to a point;
`scheduleE`'s actual S_π on dense trees ([open], affects only how much
worse the oracle is than the baseline, not any σ\* claim); byte-metered
extension if a mux is ever seriously proposed.
