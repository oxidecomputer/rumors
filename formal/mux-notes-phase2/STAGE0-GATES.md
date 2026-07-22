# STAGE 0 — pre-Lean probe gates P1–P4 (mux-conjectures phase 3)

**Frozen record (2026-07-22 estate audit):** kept deliberately — stage 0's evidence record (the causal σ* sweep, P2's π-eligibility refutation, P4's no-peek reversal). Claims herein carry the epistemic status they had when written; where later work superseded one, the supersession lives in MUX-PROGRESS §4 or an in-place marker, and this file is otherwise not updated.


Stage-0 agent, 2026-07-21. Code: `../probe/` (`causal.py`, `oracle.py`,
`families.py`, `wedgecheck.py`, `stage0.py` added; `model.py`/`mux.py`
untouched; `python3 selftest.py` re-run: **21/21 calibration gates pass**).
Raw numbers: `results-stage0.json`, `extension-runs.json`; full driver log
`../probe/stage0_run.log`. Epistemic tier throughout: **[checked]**
(probe/transcription tier, deterministic given seeds).

## Verdict table

| gate | verdict | headline |
|---|---|---|
| **P1** causal A_p-limited σ*×σ* | **PASS — GREEN** | 4,970/4,970 runs Terminal (397 + 100 skeletons × C∈{1,2} × 5 interleavings); zero σ-stuck, zero hard-stuck, zero fuel; zero unsound verdicts in the omniscient cross-check. **C1-literal stays FALSE; T4 is unblocked.** |
| **P2** ofSchedule(π_d) | **FAIL (informative)** | π_d transcribed from `scheduleE` (primary form, not the proxy): 1,010/1,090 Terminal; **8/100 random skeletons wedge at both C=1 and C=2, all 10 configs each**; all pins pass. π-eligibility is FALSE as stated — minimal 11-scope counterexample in hand. Per the gate rule, **T5 falls back to the state-feedback oracle**, which (with causal σ*) is Terminal on every wedge skeleton. |
| **P3** wedge singleton / forced run | **PASS** | Mechanized prove-c1 §4.3 adversary (R→I deliveries withheld): wedge stuck at C∈{1,2,3}; **every strategy consultation singleton-enabled** — in fact per-party enabled-push sets never exceed 1 anywhere on the forced run. bottomMostReady×bottomMostReady kill matrix reconfirmed. Decide-anchor traces dumped. |
| **P4** no-peek σ* on the F2 family | **PASS with a reversed expectation** | The no-peek causal σ* did **NOT** wedge: 3,470/3,470 runs Terminal, including the whole F2 gadget family and 250 randoms. **Slot-peek was not observed to be load-bearing.** The panel's F2 prediction is unconfirmed at probe tier. |

**The single headline: the causal σ\* survived P1.** Every run of the
symmetric causal composition terminated, on every pin, all 88 adversarial
family skeletons (alternating-parity fresh-dispute chains, all-M tails, the
F2 pattern family), and 400 random well-formed margin-0 skeletons, at C ∈
{1, 2}, under 5 interleavings each. The one place a C1-true reversal could
hide (SYNTHESIS Condition B) produced nothing. Secondary headline: **the
demand-order oracle's π-eligibility lemma is refuted** — T5 must take its
recorded fallback.

---

## P1 — the causal σ* (the blocking gate)

### What was implemented

`causal.py::CausalSigma` — the formulation of record (refute-c1 §1–§2 as
ratified by MUX-ADJUDICATION, F1 keystone repair discipline, F6 positional
guards):

- **Observation**: p's own pushes (flush-paced, recorded at the push) +
  frames **delivered to p's demux slots** (slot-peek, recorded at the
  `demux` action; `peek=False` switches to consumed-only for P4). Excluded
  by construction: consumption receipts, the peer's components, in-flight
  reverse frames, own-pipe drain.
- **A_p**: the two minting rules. Received minting: frame n on `wire(q,h)`
  decodes (B5) to the scope it is about and announces that scope's record.
  Own minting: the same arrival announces the records of that scope's
  children (the receiver answers height h−1 and can merge). The opening
  listing announces record(root) to R; R's opening reply announces
  record(root) + kids to I.
- **Certified ∪ Inevitable** as a forward derivation: a Kahn-confluent
  fixpoint from `init` over the **announced sub-skeleton only**, under all
  non-push actions, with each side's wire fires capped per-channel at
  evidence (own actual pushes / observed arrivals). I-step never cites an
  unperformed push by either side (self-containment); deliveries enter only
  for capped-in (= performed) sends with the slot E2 guard — the F1
  discipline; guards are evaluated on the derived state = positional form
  (F6). Demand rule: push (c, k) iff k = 1 or the derived state has
  consumed ≥ k−1 frames of c.
- **Causality boundary, structural**: the strategy object holds the true
  `Skel` privately and touches it in exactly two B5-justified places
  (frame→scope decode; record read of an already-announced scope). All
  derivation goes through `KnownSkel`, which **raises** outside A_p;
  quantities that only steer are-we-done transitions (stage lengths, list
  lengths, totalLeafReqs) report a large sentinel instead, which
  under-derives closes only, never consumption — the most-restrictive
  sound reading. The strategy never reads the run state; mechanical
  enabledness (committed obligation + pipe room) is the harness's job.
- **F8**: the strengthened `recvClose` (slot empty ∧ producer done ∧ no
  in-flight frames in the producer's pipe) was already in `model.py`
  (`walkCloseWire`/`absorbCloseWire` `_pipe_count == 0` conjunct) — nothing
  to add; noted as implemented.
- **Step-4 per-state invariant**: asserted at every step of every run —
  whenever the system quiesces to pushes-only (no free action), some
  withheld push must be causally proven, else the run records `sigma_stuck`
  and the gate fails. Zero occurrences. Additionally, the **soundness
  cross-check** (causal-proven ⊆ omniscient-exit-certified) ran on every
  consultation of every pin, every family skeleton, every 10th random, and
  the P2-wedge skeletons: **0 violations** — the causal closure never
  over-derives.

### Sweep and results

| pool | skeletons | runs (×C∈{1,2}×5 interleavings) | outcome |
|---|---|---|---|
| pins (incl. `wedge`, regression ×2, jam+m0, pdelay+m0) | 9 | 90 | all Terminal |
| families: alt-parity chains (12), all-M tails (10), F2 gadgets (66) | 88 | 880 | all Terminal |
| random margin-0 (seeds 1000–1299, rootH ∈ {4,6}, fan ≤ 7) | 300 | 3,000 | all Terminal |
| extension: P2's pool (seeds 5000–5099) | 100 | 1,000 | all Terminal |
| **total** | **497** | **4,970** | **4,970 Terminal** |

Idle telemetry confirms σ* genuinely exercises the right to idle where
work-conserving schedulers die: e.g. `regression8` C=2 greedy idles at 104
states before completing; the wedge family completes with 25–78 idle
states per run. The causal σ* is live exactly where H-a's class is dead.

### The one implementation artifact found (and why it matters for Lean)

The first cut of the derivation reused the true mux's **head-only FIFO
delivery** inside the fixpoint. Because the sim re-derives pushes in its
own cross-channel interleaving, this manufactured spurious head-of-line
blocks and produced fake wedges (two F2 skeletons, since re-verified
Terminal in 20/20 configs). The fix: in the derivation, `del(c,n)` is
gated **only** by the slot E2 edge (send performed, rcv(c,n−1) performed) —
delivery skip-scans the derived pipe. This is precisely attack-refute F1's
point that the DAG has no cross-stream pipe-order edges, observed as a
live/dead difference. **Lean note for T2/T4: the I-step closure must be
defined channel-positionally (E2-membership), never over a simulated
shared-FIFO state, or the coverage induction will be proving the wrong
(and false-completeness) object.**

### Ambiguities resolved (least-information readings, flagged for the panel)

1. Own-minting latency = arrival of the parent's frame (refute-c1 §1.2's
   words), not reply-production time. (More restrictive alternatives would
   only delay knowledge σ* provably doesn't need before that point.)
2. BFS census positions are parent-record-driven only; a delivered frame
   does not pin its own level position beyond what announced parents imply.
3. The strategy was made a pure function of (pushes, arrivals) alone — it
   does not even consult p's own component snapshot, a strict restriction
   of the licensed observation. It still never idles into a wedge.

## P2 — ofSchedule(π_d)

**Which π was implemented: BOTH, labeled.** Primary (form of record):
`oracle.py::scheduleE` is a mechanical transcription of
`Proofs/Sched.lean`'s `scheduleE` — the per-process encoder-order (D6)
traces as structural folds (childChunk / scopeSendsE with parent-last /
prologues / openers / absorb / asm blocks / fins) merged by the priority
scan with per-channel counters and cap windows; `demand_order(sk, d)` is
T5's filterMap (receive events on direction d's wire channels). Transcription
checks: the merge drains every trace on all 147 skeletons tried (assertion),
and the projection agrees with the run-derived receive order as a multiset
and per-channel subsequence on 47 skeletons. Secondary (fallback): the
receive order of a completed unmuxed greedy drain.

**Result: FAIL on the random class.** scheduleE-π: 1,010/1,090 Terminal;
wedges on 8/100 randoms (5001, 5016, 5061, 5079, 5085, 5088, 5093, 5094) —
at **both** C=1 and C=2, all 5 interleavings, deterministic. All 9 pins and
all 88 family skeletons pass. The run-derived fallback π wedges on 5/100
(subset of the same class). A 400-seed rootH=4 scan found 11 more wedges;
the smallest is **11 scopes** (`gen_random(20063, rootH=4)`, recorded with
anatomy and both π's in `p2-counterexample-minimal.txt`).

**Mechanism (diagnosed on rand5016, step-level).** The ledger guards force
`snd(wire(I,3), f)` — a trailing provision wire of a scope whose D children
are already resolved — **before** `snd(wire(I,1), m)`: m's commit needs the
walk's earlier D sibling fully queried, the queries need the cap-1
`leafRequests` cell, the cell needs absorb, absorb needs the next
`wire(R,0)` supply, the supply needs R's pipe (blocked head-of-line behind
a `wire(R,4)` frame), whose slot drains only when Walk(I,3) completes its
scope — by firing f. Meanwhile τ **receives** m before f (legal
cross-stream skew: f waits in its cap-1 cell while the deep exchange
completes). A receive-order pusher therefore withholds f waiting for m
while m transitively requires f. **π-eligibility ("when the τ-least
unperformed event is a wire push, all π-earlier frames of its direction
are already pushed") is FALSE** — the adjudication's named risk item for
the oracle module, now with executable counterexamples.

**Gate consequence (per MUX-ADJUDICATION §4):** T5 falls back to the
**state-feedback oracle** (the probe's 'exit' certificate, omniscient
grounding). Checked: the state-feedback oracle AND the causal σ* are
Terminal on all 8 wedge skeletons (16/16 and 16/16 configs; plus the 50/50
sweep on the first five). Nothing else in the suite consumed π-eligibility;
T3/T4/T6 are unaffected. Note the irony for the docs: on these skeletons
the *local* σ* is live where the *nonlocal* demand-order oracle deadlocks —
the oracle's failure is scheduling rigidity, not information.

## P3 — the wedge singleton assertion

`wedge` = regression shape, rootH=6, root fan 5, first radix child
deep-disputed to full depth, w=4 provisions behind it (margin-0,
wellFormed, schedulable — asserted). `wedgecheck.py::forced_run` mechanizes
the prove-c1 §4.3 adversary: flush-paced pushes first (bottom-most-ready
choice), all non-demux actions next, I→R deliveries, and **R→I deliveries
dead last** (withheld through the wall).

| C | outcome | strategy consultations | all singleton? | max per-party enabled-push set anywhere on the run |
|---|---|---|---|---|
| 1 | stuck @68 | 8 | **yes** | 1 |
| 2 | stuck @77 | 9 | **yes** | 1 |
| 3 | stuck @77 | 9 | **yes** | 1 |

The forced-run technique's premise holds in the strongest form: not only is
every consultation singleton-enabled, no state on the run ever offers
either party more than one enabled push — WC + singleton ⇒ the script
replays for any pair. Decide anchors (full action lists + stuck anatomy):
`trace_wedge_bottom_C{1,2,3}.txt`.

Kill-matrix reconfirmation (generic interleavings, eager bottom×bottom):
stuck under all 5 interleavings at C∈{2,3}; at C=1 under the 3 random
interleavings (greedy/push_first escape at C=1 — the withholding adversary
above closes that gap deterministically). Consistent with probe §3's
"deadlock = some tested interleaving sticks".

## P4 — the no-peek variant

Observation = **consumed** frames only (arrivals recorded at
`walkRecvWire`/`ropenRecv`/`absorbRecvWire` instead of at demux delivery).
Swept: pins + all 88 family skeletons (including every F2 gadget, the
attack-refute F2 alternating-D shapes among them) + 250 randoms, C∈{1,2},
5 interleavings: **3,470/3,470 Terminal.**

The panel expected a wedge ("slot-peek is load-bearing... the no-peek
variant is plausibly FALSE", attack-refute F2; SYNTHESIS §2.3). **Not
observed.** Two honest readings:

1. attack-refute's gadget was never fully constructed ("[open, not fully
   constructed]"); my F2 family (4^4 rootH-4 exhaustive + 50 rootH-6
   pattern trees incl. the named alternating shapes) may not realize the
   exact mutual-parking configuration. The needed configuration — a
   label-carrying frame parked delivered-but-unconsumed while its
   consumer's walk idles on a τ-above withheld push — is self-limiting
   under σ*: walks park only when σ* withholds, and σ* withholds only when
   derivation lags, which the forward closure kept short everywhere tested.
2. refute-c1 §6.5's original position ("no-peek believed to hold", via the
   scope-completion-strengthened demand rule) may simply be right.

Recorded per the gate text ("either way, record it"): **no executable
evidence that peek is load-bearing**; the observation ruling (§2.3
slot-peek IN) stands as a modeling decision, not a liveness necessity, on
current evidence. T7's "mint the starvation control" should wait for an
actual starving instance; none exists yet.

## Files

- `../probe/causal.py` — KnownSkel (A_p view), CausalSigma, causal_run,
  wedge anatomizer. `../probe/oracle.py` — scheduleE transcription,
  demand_order, ofSchedule runner (+ run-derived fallback).
  `../probe/families.py` — the three adversarial families.
  `../probe/wedgecheck.py` — wedge + forced run. `../probe/stage0.py` —
  driver. `../probe/stage0_run.log` — full log.
- `results-stage0.json` — all counts (incl. the superseded run-derived P2
  as `p2_oracle_runderived_fallback`); `extension-runs.json`.
- `trace_wedge_bottom_C{1,2,3}.txt` — P3 decide anchors.
- `p2-counterexample-minimal.txt` — the 11-scope π-eligibility refuter.

## Caveats

Transcription tier, not kernel tier; sampling (5 interleavings/config, 400
randoms at rootH ≤ 6, fan ≤ 7), no BMC; ρ-boundedness means no livelocks
hide behind the zero fuel-exhaustions. The scheduleE transcription is
validated by merge-completeness + per-channel projection agreement, not by
the eventdag gate itself. The P1 verdict is as strong as the A_p
formalization implemented here; the three least-information ambiguity
resolutions above are the places a Lean definition could still diverge.
