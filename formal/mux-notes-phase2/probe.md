# Probe report: executable adjudication of the mux trichotomy

Role: executable probe (phase 2 adjudication panel). Date: 2026-07-21.
Code: `/private/tmp/claude-506/-Users-oxide-src-rumors/fba95cb0-7415-4790-ae13-976998d6ac3c/scratchpad/probe/`
(Python 3.14, deterministic given seeds; `python3 selftest.py` runs the
calibration gates; `python3 experiments.py all` reproduces everything and
writes `results.json`).

Provenance: the simulator is a **mechanical transcription** of
`formal/lean/StreamingMirror/Skel.lean` and `Model.lean` (worktree
`/Users/oxide/src/rumors-mux`, branch `mux-conjectures`, HEAD `8c0a8e25`),
guard-for-guard and action-for-action: the 23 `Action` constructors,
committed-choice commit/fire split, occupancy-counter channels, the AxMode
ledgers (W, D1root, D1int, D2, D3, D4, D5, D6, wireFirst), `producerDone`
close semantics, `allActions` in the Lean's exact enumeration order (so the
greedy `drain` replicates the Lean's `firstM` scheduler bit-for-bit). The
pinned skeletons and both control schedules (`jam`/`trap`,
`pdelay`/`parentTrap`) are transcribed from `Instances.lean`/`Controls.lean`
verbatim. AxMode `.impl` (the shipping encoder, MODEL.md §6 D6) is used for
every mux experiment.

Epistemic tags as in PROGRESS.md: **[checked]** = observed in this
simulator's runs; **[derived]** = argument from the model/spec; **[open]** =
known unknown.

---

## 1. Calibration — 21/21 gates pass [checked]

Every kernel-checked or gate-checked fact I could re-execute was re-executed
before any mux experiment. All pass on the first complete run (no gate was
ever observed failing after the transcription compiled):

| # | gate | Lean anchor | result |
|---|---|---|---|
| 1 | wellFormed: smokeChain, rMix, comb6, pyramid 4/2/1, jam, pdelay | Instances/Controls | PASS |
| 2 | greedy `.full` completion + all-channels-drained: the 5 positives | `positives_complete` | PASS |
| 3 | pyramid 1 greedy `.full` drain(600) stuck | `pyramid1_stuck` | PASS |
| 4 | pyramid 1 not schedulable; positives schedulable | `pyramid1_not_schedulable`, `positives_schedulable` | PASS |
| 5 | jam and pdelay wellFormed ∧ schedulable ∧ dCount = capLevel+2 | `jam_on_boundary`, `pdelay_on_boundary` | PASS |
| 6 | `trap` replay under fullNoD4 → stuck ∧ ¬terminal | `trap_stuck` | PASS |
| 7 | `trap` refused under fullNoD5 AND under `.full` | `d4_rejects_trap` | PASS |
| 8 | `parentTrap` replay under fullNoD5 → stuck ∧ ¬terminal | `parentTrap_stuck` | PASS |
| 9 | `parentTrap` refused under `.full` | `d5_rejects_parentTrap` | PASS |
| 10 | jam & pdelay greedy `.full` completion | `jam_completes_full`, `pdelay_completes_full` | PASS |
| 11 | phantom walk (R,3) steals opening → rejected | `phantom_walk_rejected` | PASS |
| 12 | random-fair scheduling ×20 seeds: positives terminal under `.full`; margin-0 pins terminal under `.impl` | the two flagship theorems, exercised beyond the greedy schedule | PASS |

Gates 6–9 are exact replays of the ~60- and ~100-action kernel-checked
schedules — the strongest transcription check available: they exercise the
committed-choice split, every ledger guard, `normWalk` scope advancement,
the assembler pending arithmetic, and the close cascade, and they must
succeed/fail at exactly the recorded actions. They do.

**Verdict: the core is calibrated; mux results below are trustworthy at the
transcription tier** (not kernel tier — see caveats).

## 2. The mux model as implemented (decisions of record)

Per MUX-PROGRESS.md §2, confirmed against the channel structure of
MODEL.md §4 and the old Rust proxy discipline (`session/incoming.rs`,
`outgoing.rs`):

- **What is muxed**: exactly the wire family — `wire p h` for all h,
  including the two opening wires (`iopenFire`/`ropenFire` wire arms).
  Everything else (asked, leafRequests, upper, lower, level, root channels)
  keeps its Model.lean semantics untouched. (MODEL.md §4: "the pump's
  capacity-1 channel is the wire"; exposition: only wires cross parties.)
- **Sender side**: a wire send by party p appends a channel tag to
  `pipe[p]` (bounded FIFO, capacity C, message-counted); guard
  `len(pipe[p]) < C` replaces the base cap-1 wire-channel guard. The cap-1
  wire channel itself becomes the **receiver-side demux slot**.
- **Demux** (one per direction, non-strategic, always willing): delivers
  the pipe **head** into its target wire channel's slot when the slot is
  empty; head-of-line blocks while it is full. This is the shipped
  discipline (wire-order delivery, one-slot per-stream handoffs, sole
  reader — deadlock doc §1.1).
- **Close semantics**: close-recv of a wire channel additionally requires
  zero in-flight messages for it in the producer's pipe (EOS after final
  bytes).
- **Terminal** additionally requires both pipes empty. **Deadlock** =
  reachable non-terminal state where no process, demux, or (strategy-
  permitted) push can move; for work-conserving runs this is literally
  "no action enabled". Every non-terminal outcome reported below is such a
  state — fuel exhaustion never occurred in any experiment ([checked];
  1920/1920 eager runs classify as terminal or stuck, 0 fuel).
- **Committed choice meets the mux**: a walk that commits a wire obligation
  holds it in hand until the pipe has room — the model twin of "a frame
  serialized into the FIFO cannot be retracted, and a sender mid-frame
  cannot switch streams". σ* additionally *fuses* wire commits with their
  fire so it never parks committed on a frame it is unwilling to push
  (choice of commit order is the sender's local strategy; the axiom guards
  are respected by construction since the fused pair replays
  `walkCommit`+`walkFire`).
- Skeleton class: every mux experiment uses **margin-0 skeletons**
  (`dCount ≤ capLevel`), so the UNMUXED `.impl` system is inside the
  kernel theorem `Sched.deadlock_free` — any mux deadlock is attributable
  to the mux, not to capacity or ledger effects. (The σ* sweep additionally
  covers `.full` on schedulable non-margin-0 skeletons, including jam,
  pdelay, pyramid 2 at their original capLevels.)

## 3. H-a: work-conserving schedulers deadlock — SUPPORTED [checked]

Pool: 8 pinned shapes (smokeChain, rMix, comb6, pyramid 4, jam and pdelay
with capLevel lifted to margin 0, the regression shape at two sizes) + 120
random well-formed margin-0 skeletons (rootH ∈ {4,6}, fan ≤ 7). Schedulers:
bottom-most-ready (the shipped discipline), top-most, round-robin, random;
interleavings per configuration: push-first (flush-paced sender running
ahead), greedy, and 3 random seeds; deadlock = any tested interleaving
reaches a no-move state.

| policy | C=1 | C=2 | C=4 | C=8 | C=16 |
|---|---|---|---|---|---|
| bottom-most | 39/128 | 37 | 38 | 38 | 38 |
| top-most | 40/128 | 39 | 40 | 40 | 40 |
| round-robin | 40/128 | 39 | 39 | 39 | 39 |
| random | 39/128 | 38 | 38 | 38 | 38 |
| **cert-aware** (below) | 39/128 | — | 38 (C=4) | — | — |

(cells = skeletons with a reachable deadlock)

Findings:

1. **~30% of random margin-0 skeletons deadlock every work-conserving
   scheduler tested, at every capacity C ∈ {1,2,4,8,16}.** The deadlocking
   sets nearly coincide across policies (±2 skeletons): the wedge is
   skeleton-intrinsic, not policy-specific. [checked]
2. **Capacity-independence.** Deadlock counts are flat in C. The minimal
   deadlocking width of the regression family (root disputes its first
   child to full depth, w whole-subtree provisions behind it, rootH 6) is
   **w = 4 for every C in {1..16} and both extreme policies** — width does
   NOT scale with C. [checked] This confirms, in-model, the deadlock doc's
   empirical §1.4 finding (stall identical at 64 B and 16 MiB): the
   load-bearing bound is the per-stream demux slot (cap 1), not the pipe.
   **Consequence for C1's statement: no pigeonhole over pipe capacity is
   needed or appropriate; the impossibility should be stated against
   bounded per-stream demux state, with the pipe's boundedness only
   forcing the sender's commit-no-retract.** [derived]
3. **The wedge is the Rust six-link cycle, exactly.** Deterministic witness
   (regression shape, bottom-most, C=2, 94 steps —
   `probe/trace_regression_bottom_C2.txt`; minimal w=4 C=1 witness in
   `probe/trace_minimal_w4_C1.txt`): walk(I,5) flushes the disputed child's
   reply and then its provision run onto `wire(I,5)`; the pipe head +
   slot fill with provisions; the consumer walk(R,4) is parked on its
   parent-summary send into cap-1 `upper(R,4)` (positional assembly:
   asm(R,5) is filling the FIRST, disputed, Pending slot, which
   transitively requires the deep reply); the deep answer — walk(I,1)'s
   committed `wire(I,1)` — cannot enter the full pipe / cannot overtake the
   pipe head. Demux blocked on the full slot; nothing moves. Links 1–6 of
   design/streaming-wire-deadlock.md §2 map one-to-one. [checked]
4. **Even the smartest work-conserving policy dies.** Policy 'cert' picks,
   among enabled frames, one whose pipe-exit is PROVEN (σ*'s certificate,
   §4) whenever one exists, falling back only when work-conservation forces
   an unproven push. It deadlocks the same ~30% of skeletons (39/128 at
   C=1, 38/128 at C=4). The runs wedge precisely at states where the pipe
   has room, at least one frame is enabled, and NO enabled frame is
   certified — a state in which every work-conserving scheduler, whatever
   its tie-break, must push a frame that can wedge. **Frame CHOICE is not
   the missing ingredient; the right to IDLE is.** [checked]
5. Scope of the support: five policies plus the certificate-aware one were
   tested, under 5 interleavings each — not a quantification over all
   deterministic local strategies. A strategy could in principle shape its
   entire history to avoid ever reaching a forced-uncertified state. The
   theorists' C1 burden is exactly a forcing lemma: *every* work-conserving
   history on the witness family reaches a state where all enabled pushes
   are uncertified. The probe's evidence (identical deadlocking sets across
   very different policies; wedge states reached from both cooperative and
   adversarial interleavings) makes that lemma plausible but does not
   prove it. [open]

## 4. H-b: σ* demand-lockstep is live at C=1 — SUPPORTED at the structural level [checked], with the locality question sharpened, not settled [open]

**σ* as implemented.** A wire push is permitted only when its eventual
pipe-exit is PROVEN from traffic already committed: clone the state,
perform the push, close the system under every action EXCEPT further wire
pushes on either side (demux and all intra-party actions run freely), and
require the pushed channel's in-pipe count to return to zero. Two variants:
'exit' (message provably leaves the pipe — no-HOL) and 'consume' (also
leaves the slot). Otherwise the sender idles that frame. Both sides run σ*
(symmetric composition). This is a sound demand-proof: by induction every
pipe prefix drains, so the demux never blocks persistently — the mux
becomes transparent and the composed system embeds in the base model with
wire capacity ≤ 2 (slot + head), which is covered by the flagship theorem
plus the capacity-monotonicity assumption (Statement.lean "assumed, not
proven"). [derived]

**Results.**

- **2,150/2,150 runs terminal**: 215 skeletons (all pins incl. both
  regression sizes, the regression-family and D-fan width sweeps, 200
  random margin-0) × C ∈ {1,2} × 5 interleavings/seeds each, `.impl`,
  'exit' certificate. Zero `sigma_stuck` (demands all unproven yet session
  incomplete), zero `hard_stuck`, zero fuel. [checked]
- **174/174 terminal** with the strict 'consume' certificate (pins + 50
  random, C=1). [checked]
- **81/81 terminal** under `.full` on 27 schedulable NON-margin-0
  skeletons including jam, pdelay, pyramid 2 at original capLevels — H-b's
  "all skeletons the unmuxed protocol completes" leg, beyond the margin-0
  class. [checked]
- **The symmetric composition never bottoms out**: no reachable state had
  every mechanically-available push unproven with nothing else to do. The
  worry that withholding is self-blocking (axiom W chains a walk's
  resolutions/queries behind its own wire, so idling a wire stalls the
  whole walk and its assembler tower) did not materialize: the other
  side's free actions always eventually made some certificate true.
  [checked, ~2,400 runs]
- **σ* idles exactly where eager dies.** On 68 skeletons: 24 had σ*
  idle at least once (up to 199 idle events); the 20 eager-deadlocking
  skeletons are all among them; zero skeletons deadlocked eager without σ*
  idling. Idling is both necessary (H-a) and sufficient (H-b) on this
  pool. [checked]
- **Demand-proofs are confluent**: at 1,739 certificate decision points,
  recomputing each certificate under 5 randomized closure schedules never
  changed a verdict (0/8,695 disagreements). The certificate is a function
  of (skeleton, committed traffic), not of the closure's schedule — the
  per-side Kahn-determinism the docs assume, observed. [checked]

**What this does and does not say about C1.** The probe's σ* reads the
current global state (including remote internals) when computing
certificates. At model level this is unavoidable: **the model has no
per-party private trees — both parties' processes are derived from the one
shared `Skel`** (MODEL.md §2; MUX-PROGRESS §4 names per-party knowledge as
the genuinely new definition). So the probe settles the STRUCTURAL half of
the hinge: *the shared-capacity coupling of one bounded FIFO per direction
is NOT by itself fatal — an idling scheduler with full knowledge is live at
C=1 per direction, on every skeleton tested, with the shipped demux
discipline unchanged.* Any true C1 must therefore be an INFORMATIONAL
impossibility, and its fooling wedge must be built at the tree/knowledge
level (two remote trees indistinguishable from the local trace demanding
incompatible push orders), not at the channel-structure level. [derived
from checked results]

Confluence collapses the residual gap usefully: since the closure verdict
is a deterministic function of (skeleton, committed traffic), and committed
traffic is causally known to the sender (its own pushes; its own receives;
FIFO delivery), **a sender that KNEW the skeleton could compute every
certificate by simulation with zero communication** — σ*-with-skeleton ≈
σ*-omniscient. The entire locality question for C1 therefore reduces to:
*can each side learn enough of the dispute skeleton, in time, from its own
tree plus announced questions/reactions?* That is exactly the coordinator's
hinge, now with the channel-structure side eliminated. [derived]

## 5. H-c: the serialization price — NOT SUPPORTED at model granularity [checked], and the probe explains why the model cannot price it [derived]

Fair parallel-time metric: rounds-to-terminal, one action per agent per
round, identical loop for all configurations (σ*'s fused commit+fire is
charged two agent-rounds to match the baseline). Over the 33 skeletons of
the cost pool (pins + 25 random):

| configuration | rounds / unmuxed baseline (mean, max, n) |
|---|---|
| σ* mux, C=1 | **0.99**, 1.03, 33 |
| σ* mux, C=8 | 0.99, 1.03, 33 |
| eager mux, C=8 (only where live) | 1.11, 1.18, 21 |

σ* at C=1 matches the un-muxed independent-channel baseline within noise on
every skeleton, including both regression shapes (where eager deadlocks).
There is NO steep serialization price in this model. [checked]

Why this does not refute H-c so much as relocate it: the model is
message-counted, payload-erased, and latency-free — three erasures that are
precisely where §5A's cost analysis lives (deadlock doc: W=1 lockstep costs
~1 RTT per consecutive same-stream reply; provision runs are subtree-sized
in bytes; sizing `extra rounds ≈ Σ_ℓ max(0, ⌈frontier_bytes_ℓ/W⌉ − 1)`).
Moreover our σ*'s proofs are computed with zero informational lag; a causal
σ* would wait for announcements, paying round trips the model does not
meter. **Conclusion for the panel: H-c's "steep price" cannot be exhibited
or refuted inside the current Lean model. Pricing it needs either a
hop/latency-metered extension (the Rust `tests/hop_trace.rs` style) or a
Rust-level measurement. If C2 is to claim "credits are needed for
liveness+performance jointly", the performance half of that statement is
currently unformalizable in the artifact's vocabulary.** [derived]

## 6. The hinge sub-questions, answered as far as the model allows

- **Does every receiver-side branching that affects consumption order get
  announced?** In-model this question cannot be posed: there are no
  private branchings — consumption order is a function of the shared
  skeleton plus interleaving, and the confluence result shows the
  interleaving component is irrelevant to demand (verdicts are functions of
  delivered traffic). [checked] The unannounced residue is exactly the
  skeleton itself (whose D/R/M labels depend on both trees). At the Rust
  level the M-children are dropped with zero channel ops on both sides
  (MODEL.md §2), but the parent scope's reply frame always flows and the
  M-set is inferable once that reply is decoded — by the RECEIVER. Whether
  the SENDER can infer the part it needs before its push decisions is the
  skeleton-learning question above. [open — tree-level, out of this
  model's reach]
- **Are provision-run (R-child) absorptions order-known though silent?**
  Yes: R-children are consumed positionally exactly like D scopes (wire k
  then asked k per scope, MODEL.md §5), and the certificates never needed
  more than positional counting. [checked]
- **Does the symmetric composition bottom out?** Never observed in ~2,400
  σ* runs spanning every pinned adversarial shape, the regression family,
  and 250+ random skeletons, all interleavings tested, both certificate
  strengths, both AxModes. [checked]
- **Is a same-stream head-block always finite under sound demand-proofs?**
  Yes by construction (every pushed message provably exits the pipe given
  already-committed traffic) and empirically (zero HOL wedges under σ*).
  [checked]

## 7. Read of the evidence (probe's verdict for the panel)

The trichotomy, as tested, comes out **H-a supported / H-b supported
structurally with the locality burden isolated / H-c unpriceable in this
model**:

1. **C1 for the natural (work-conserving) class is real and
   capacity-independent.** The witness family is in hand (regression shape,
   w ≥ 4, deep-disputed first child), the wedge is the Rust six-link cycle,
   and the Lean route is exactly the Controls.lean stuck-run technique on a
   mux-extended transition system: concrete skeleton + concrete scheduler +
   `decide`. The statement should bound per-stream demux state (slot = 1),
   not lean on pipe capacity. [checked → Lean-ready]
2. **C1 as literally stated (all local strategies, idling allowed) is in
   grave danger.** A strategically-idling scheduler is live at C=1 on
   everything we could throw at it — but our σ* consumes global state, which
   the model cannot distinguish from local knowledge because it has no
   private-knowledge notion. If the panel wants C1 true, the fooling wedge
   MUST come from skeleton-indistinguishability at the tree level; no
   channel-structural obstruction exists. If the panel wants C1 false, the
   missing lemma is skeleton-learnability: the sender can compute each
   certificate at the time it first becomes decision-relevant from (own
   tree, announced questions/reactions, own delivered FIFO prefix).
   [derived; the deciding construction is out of the probe's reach]
3. **C2's positive half is stronger than conjectured**: capacity 1 per
   direction suffices (not merely "small constant"), with the shipped demux
   discipline, on every skeleton tested — and the oracle can be
   state-feedback (push when certified) rather than a precomputed τ
   serialization. The τ-projection route (MUX-PROGRESS §4) remains the
   kernel-proof route, but the target constant should be C=1. [checked]
4. Genest–Kuske–Muscholl framing, as invited: the session's event family is
   **existentially 1-bounded** (σ* exhibits a 1-bounded linearization per
   direction, computable by per-side simulation) but **not universally
   bounded even with smart frame choice** (work-conserving = forced
   linearization progress; ~30% of skeletons refuse every such
   linearization). C1's remaining content is precisely "no
   locally-computable existential linearization" — the GKM gap between
   ∃-bounded and locally-schedulable. [derived]

## 8. Caveats (all of them)

1. **Transcription tier, not kernel tier.** Python, calibrated by 21 gates
   including exact replays of both kernel-checked stuck schedules; still
   one transcription removed from the Lean.
2. **Sampling, not exhaustion.** 5 interleavings per configuration
   (2 deterministic + 3 seeds), 120–250 random skeletons; no BMC pass.
   ρ-boundedness means no livelocks are being missed (every run terminates
   in bounded steps; zero fuel-exhaustions observed), but rare-interleaving
   deadlocks under σ* could in principle be missed. The eager deadlocks
   need no such hedge — each is a concrete witnessed stuck state.
3. **σ* omniscience.** Certificates read the true global state, including
   reverse-direction in-flight messages the local side has not yet
   received. Confluence + FIFO reduce the gap to skeleton knowledge
   [checked/derived], but a causal-simulation σ* (fed only by own
   trace + skeleton) was not implemented, and a tree-local σ* (fed only by
   own tree + trace) cannot even be expressed in this model.
4. **Demux discipline fixed** to the shipped one (wire-order, one slot per
   stream, sole blocking reader). Robustness across demux variants (e.g.
   consumer-readiness demux) is argued in MUX-PROGRESS §2 as a phase-2
   obligation and was not swept here.
5. **Message-counted capacity.** Payload bytes (the §5A unit-mismatch
   discontinuity, provision runs) are erased; H-c's verdict is strictly
   model-granularity; C=1 "messages" hides unbounded bytes per message.
6. **Random-skeleton distribution** (rootH ≤ 6, fan ≤ 7) is small-world;
   the Rust constants (rootH 32, fan 256) are far larger. Width/depth
   scaling beyond the tested range is extrapolation — though the
   capacity-flatness across C ∈ {1..16} at fixed width-4 witnesses needed
   no scaling at all.
7. **Eager 'deadlock' classification** is per-skeleton "some tested
   interleaving sticks"; per-run rates are lower (e.g. 489/1920 stuck for
   bottom-most across C ∈ {1,4,16}).
8. jam+m0/pdelay+m0 lift capLevel to margin 0 to stay inside the `.impl`
   theorem's class; the originals were additionally exercised under
   `.full` in the σ* sweep.

## 9. File inventory

- `probe/model.py` — the transcription (Skel + Model + run/drain).
- `probe/instances.py` — pins + `trap`/`parentTrap` schedules.
- `probe/selftest.py` — the 21 calibration gates.
- `probe/mux.py` — mux layer, eager policies (incl. 'cert'), σ*,
  certificates, stuck-state anatomizer.
- `probe/gen.py` — random skeletons, regression family, D-fan family.
- `probe/rounds.py` — fair parallel-time metric.
- `probe/experiments.py` — matrix driver (`all` ≈ 10 min single-threaded).
- `probe/confluence.py` — certificate order-independence probe.
- `probe/results.json`, `probe/run_all.log`, `probe/run_all2.log` — outputs.
- `probe/trace_regression_bottom_C2.txt` — deterministic 94-step eager
  deadlock (bottom-most, C=2, push-first) with stuck-state anatomy.
- `probe/trace_regression_bottom_C1_rand.txt` — C=1 random-interleave
  witness (seed 0, 92 steps).
- `probe/trace_minimal_w4_C1.txt` — minimal-width (w=4) witness at C=1,
  10-scope skeleton, with anatomy.
