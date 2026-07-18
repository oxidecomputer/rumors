# Formal verification of the streaming mirror protocol

**CAVEAT LECTOR:** This development was *entirely* "vibe-verified" using Claude
Fable 5, and has not been checked by a human expert in any of these verification
tools for correctness. It exists primarily as an experiment.

Machine-checked deadlock-freedom for `src/tree/mirror/streaming/`, built in
phases (see the design doc [`MODEL.md`](MODEL.md) for the model itself and
its soundness argument). Phase A (this directory's current content): a Quint
model plus a validation matrix that reproduces the Rust suite's known
completions, the capacity-tightness threshold, and — under relaxed axioms —
the deadlocks that prove each axiom is load-bearing.

## The assumption/theorem interface

The artifact **assumes** the send-order invariants exactly as
`materialized/progress.rs::Trace::assert_valid` checks them (they remain
proptest-verified in Rust on every scheduled run — that is the bridge), and
**checks** that under every scheduler interleaving and every
axiom-consistent committed publication order, no session reaches a stuck
state and every maximal run terminates.

| Rust (`assert_valid` check) | Model axiom (`streamingMirror.qnt` const) |
|---|---|
| wire ledger ("preceded its wire action") | `AX_W` |
| dependent ledger ("preceded its resolution", exact count) | `AX_D1_ROOT` / `AX_D1_INT` |
| lower ledger ("preceded its N lower resolutions") | `AX_D2` |
| sibling contiguity ("still owes N dependent work items") | `AX_D3` |
| wire contiguity ("departed while an earlier sibling was unresolved / owed dependent work") | `AxMode.d4` (Lean only — postdates the frozen Quint spec) |
| parent placement ("wire or query departed after the final resolution with the parent summary unsent") | `AxMode.d5` (Lean only — postdates the frozen Quint spec) |
| radix order ("violates radix order") | the per-channel in-order program structure (always on) |

The sibling-contiguity check **exists because of this model**: the original
three ledgers admitted a "publish all wires, then all resolutions, then all
queries" implementation that passes `assert_valid` and deadlocks the cap-1
child-resolution queue at fan ≥ 3. The `ledgerGap` instance is the durable
witness; `assert_valid` was tightened the same day (2026-07-15).

The wire-contiguity check exists because of this model **twice over**
(finding #6, Phase C, 2026-07-16): D3 polices the resolution stream, but
nothing ordered child i's queries before child i+1's *wire*, so a
publisher whose wire stream outruns its query stream satisfies all four
ledgers and deadlocks a three-walk wait cycle at uneven fan ≥ 3. The
durable witness is `lean/StreamingMirror/Controls.lean` — a kernel-checked
(`decide`, no native trust) stuck run on a well-formed skeleton, packaged
as `Control.jam_not_deadlockFree : ¬ DeadlockFree jam fullNoD4`. The Rust
publisher was never exposed (`yield_resolve_query!` publishes each child
wire→resolution→queries contiguously, calling it "progress-critical
order"), but the *checked* interface did not say so; `assert_valid` was
tightened the same day (2026-07-16).

The parent-placement check is finding #7 (Phase C, 2026-07-17, the
parent-delay finding): the six-ledger interface left exactly one
out-of-trace-order freedom — a walk whose D children are all resolved
could commit a last-chunk query or trailing wire with its floating
parent summary unsent, and that delay closes a commit/back-pressure
cycle through the level towers (the parent starves the assembler two
heights up; the backed-up tower stops draining the walk's own `upper`
channel below). The durable witness is
`Control.parentTrap_not_deadlockFree : ¬ DeadlockFree pdelay fullNoD5`
(kernel-checked stuck run on a well-formed, *schedulable* skeleton —
the refutation sits inside the target theorem's hypothesis class); the
fuzz sweep pins that the parent-delaying adversarial driver stalls
under `fullNoD5` and drains to terminal under today's `.full`. The Rust
publisher was again never exposed (the encoder emits the parent summary
immediately after the final resolution — the weave's §5 placement);
`assert_valid` gains the matching seventh check alongside this change.

Modeled-world premises (assumed, argued in MODEL.md §1/§5, not checked):
error-free conforming peers, SPSC channels, sequential scopes per walk.
The per-channel in-order premise (positional pairing is the protocol's
identity carrier; reordering within a channel is functional incorrectness)
started as an unchecked premise and is now `assert_valid`'s radix-order
rule, added alongside sibling contiguity.

## Toolchain (pinned)

- Quint **0.32.0**, pinned in [`quint/package.json`](quint/package.json);
  run everything via `npx quint` from `formal/quint/`.
- JDK for Apalache runs: `/opt/homebrew/opt/openjdk/bin/java` (the system
  `/usr/bin/java` is a stub); `check.sh` sets `PATH` itself.
- `tla/tla2tools.jar` (sha256 pinned in `tla/tla2tools.jar.sha256`) is
  **retired from the required path** — kept only as the escape hatch
  (`quint compile --target tlaplus` + TLC) if an Apalache defect is ever
  suspected. Note the upstream `v1.8.0` GitHub tag is a rolling nightly;
  the checksum is the real pin.

## Running the checks

```sh
cd formal/quint
./check.sh            # simulator tier: ~2 min, run on every model edit
./check.sh verify     # exhaustive tier: Apalache BMC at full depth (slow)
SAMPLES=5000 ./check.sh   # deeper random sweep

cd formal/lean
lake build            # the theorem artifact: all proofs, pins, controls
lake exe eventdag     # event-DAG control: acyclicity + totals + depth
                      # dumps per pinned skeleton (PROGRESS.md §3);
                      # nonzero exit on any failed check
```

`check.sh` encodes every expectation; a control instance passes only when
the checker **finds** a stuck state. Per-instance BMC depth is the spec's
own `totalSteps` bound, computed via the REPL — every action draws on a
finite skeleton-derived budget, so runs are bounded and checking to that
depth is exhaustive (MODEL.md §7).

**Apalache status:** `quint verify` works on this spec after the
constant-bound rewrites, but the `stuck` invariant (a large disjunction) is
SMT-expensive: ~30 s/step on the smallest instance, so full-depth
exhaustive runs are hours-scale. The simulator tier (hundreds of schedules
per instance, seconds) is the Phase A workhorse; Phase B replaces
reachability-of-stuck with an *inductive* invariant — the workload Apalache
is actually built for — and Phase C's Lean `decide` re-checks the small
instances exhaustively inside the theorem artifact.

**To be explicit about what the verify tier delivered:** no full-depth
exhaustive stuck-freedom run ever completed. The `_apalache-out` artifacts
show the deepest symbolic exploration anywhere in the campaign reached
step 18 (of per-instance bounds 106–224) before the client died; the
tier exists in `check.sh` and is honest about its cost, but exhaustiveness
was never achieved on any instance. Finding #6 (below) is the consequence
made concrete: its trap needs ~60 steps and a skeleton shape (uneven
fan ≥ 3 with an early D child owing ≥ 2 queries) that no matrix instance
has — `fanDepthPositive` misses it by exactly one query. Deadlock-freedom
claims rest on Phase C's Lean artifact, nowhere else.

## The instance matrix (all currently passing)

| Instance | Mode | Expectation | What it validates |
|---|---|---|---|
| `smokeChain` | all axioms | safe | every structurally distinct stage at D=3 |
| `rMix` | all axioms | safe | R (one-sided request) children at every legal height |
| `comb6` | all axioms | safe | D=5, internal→internal handoff both parties |
| `fanDepthPositive` | all axioms | safe | the `ledgerGap` shape, healed by D3 |
| `pyramidFull` | all axioms | safe | production stance C = F |
| `pyramidC2` | all axioms | safe | tightness law, positive side (C = N−2) |
| `pyramidC1` | all axioms | **stuck** | tightness law, stall side (C = N−3); scaled twin of Rust's stall-at-253/complete-at-254 witness |
| `n1DropW` | drop W | **stuck** | wire-before-publication is load-bearing (cross-party starvation cycle at F=2) |
| `n2fan6` / `n2fan5` | drop D1 at root (+`WIRE_FIRST`) | **stuck** / safe | the responder root return boundary: deadlock at fan 6, completion at fan 5 — the plan's sharp prediction, reproduced |
| `n2unrestricted` | drop D1 at root | **stuck** | finding: the wire ledger never constrains `DependentWork`, so a bare D1 drop already deadlocks at fan 2 |
| `n3Internal` / `n3Reduced` | drop D1 internally (+`WIRE_FIRST`) | safe / safe | D1 is *not* individually load-bearing internally once D3 holds — masked at documented capacity as predicted, and (beyond the prediction) at reduced capacity too; conjecture for Phase B/C |
| `n4DropD2` | drop D2 | safe | as predicted: slack to report against the docs, not forced into a counterexample |
| `ledgerGap` | drop D3 | **stuck** | the three original ledgers do not imply deadlock-freedom; why `assert_valid` gained sibling contiguity |

Coverage checkpoints (simulator witnesses): every channel family carries
traffic in every positive instance (`usedWire/Asked/Upper/Lower/Level/Roots`
— the model's channel tags partition the 14 Rust `QueueKind`s per the table
in MODEL.md §4), all positive instances reach `Terminal` in 100% of sampled
schedules, and the pyramid family exercises backpressure (`blockedSend`).

## Findings fed back to the Rust (Phase E ledger)

1. **`assert_valid` sibling-contiguity gap** — found, and already fixed in
   `progress.rs` (with a `should_panic` regression test).
2. **N2 threshold**: with dependent ordering dropped at the root, the
   session deadlocks at root fan 6 and completes at fan 5 — candidate doc
   note for `queues.rs::responder_root_returns`.
3. **N3/N4 slack**: internally, D1's and D2's progress roles are fully
   covered by D3 + the fan capacity at every instance tested; the docs'
   one-slot arguments are defense-in-depth there. Candidate doc nuance for
   `queues.rs`; Phase B/C should prove the parametric version.
4. **Tightness mechanism**: the +2 slack decomposes as blocked-sender hand
   + the cap-1 `lowerRes` slot (MODEL.md §8) — candidate doc improvement
   for `assembly_level_returns`.
5. **Wire-first is load-bearing per level, not just at the opening**
   (Phase B, openStage; adjudicated by Finch): a stage that withholds a
   wire reply while jammed on an internal send starves its own assembler
   through the braid, at every interior height. The "Why this is
   deadlock-free" prose in `materialized.rs` and the wire-adjacent
   constructor docs in `queues.rs` currently argue the root/opening case
   explicitly; candidate doc improvement is the per-level statement.
6. **`assert_valid` wire-contiguity gap** (Phase C, 2026-07-16; the D3
   finding's wire-stream twin) — the four ledgers admitted a publisher
   whose wire stream runs ahead of its sibling queries, which deadlocks a
   three-walk cycle at uneven fan (kernel-checked witness:
   `lean/StreamingMirror/Controls.lean`). Found while constructing the
   progress lemma: the blame-graph acyclicity argument has exactly one
   cycle the axioms failed to cut, and the witness realizes it. Fixed the
   same day: `AxMode.d4` in the model (on in `.full`; invariant shadow +
   preservation re-proven) and the wire-contiguity rule in
   `progress.rs::assert_valid` (with a `should_panic` regression). The
   Rust publisher itself was never exposed — `yield_resolve_query!`
   already enforces the order syntactically.

## Phase B: the inductive invariant and the open stage

Phase B was re-scoped mid-flight (2026-07-15, Finch): once the invariant's
architecture was validated on small instances, effort pivoted to the Lean
parametric proof rather than grinding Apalache to full CTI convergence.
What stands:

- **The invariant is in the spec** (`streamingMirror.qnt`: `indInv =
  indTypeOk and occupancyOk and indLocal and indFlow`). Its layers:
  assignment-style domain bindings (Apalache's inductive mode havocs a
  state from the invariant, so every variable needs a first-occurrence
  `x.in(S)` over constant bounds — the reason the spec's state is split
  into 25 per-field variables and the reason `NSC` exists); per-process
  local consistency (committed-choice stability, phase/cursor coherence,
  in-order prefix closure, the D3 at-most-one-deficient-resolved-sibling
  shadow); and per-channel **flow equations** (occupancy = producer sends
  − consumer receives, both derived from process-local state) — the layer
  that carries the counting argument.
- **Checked on smokeChain**: `init ⇒ indInv` passes; **`indInv ⇒
  phaseAInvariant` passes** (the acyclicity crux, discharged by Apalache);
  consecution is checked per process family (`./check.sh induction`,
  ~30–60 min per round, all families parallel). The first CTI taught two
  transcription rules now baked into the invariant and carried into Lean:
  **mirror the guards exactly** (a "strengthened" shadow — claiming W
  orders queries after wires — was falsified: the wire ledger never
  constrains dependent work, the n2unrestricted lesson again), and
  **every fired fact needs its own shadow lemma** (`AX_W ⇒ ropenRes ⇒
  ropenWire`). Final consecution tally on the post-CTI invariant
  (smokeChain): implication, `finStep`, and `stepIOpenF` discharged by
  Apalache; `absorbStep`, `stepAsmF`, `stepROpenF`, `stepWalkF` runs were
  abandoned mid-flight at the Lean pivot (clients died after ~2 h with no
  violation found; not re-run — the Lean per-action preservation lemmas
  subsume smokeChain-sized consecution parametrically).
- **`openStage.qnt` + `stageInstances.qnt` + `checkStage.sh`**: the
  level-generic assume-guarantee module — one walk plus its two
  assemblers, scope structure havoc'd at init, environment as explicit
  contract-bounded actions (level returns entitled only after the stage's
  own wire+query sends for that child: the braid contract). Results:
  stageAll safe (sim 500 schedules + symbolic to step 14); **stageNoD1
  safe** — the strongest evidence yet for the Phase A conjecture that D1
  is not individually load-bearing internally once D3 holds; stageNoD3
  jams (`stageNoD3.itf.json`, the ledgerGap shape level-generically);
  **stageNoW jams** (`stageNoW.itf.json`) — a Phase B finding upgrading
  Phase A's picture: under the braid contract (a level return for child
  c is entitled only after the stage's own wire reply and query for c —
  the returns transit the counterparty's descent), **Axiom W is
  load-bearing at every interior level**, not just at the root opening.
  Adjudicated 2026-07-15 (Finch): the braid contract is the faithful
  transcription; the finding stands. Phase C carries it as the
  level-parameterized DropW existential; Phase E owes queues.rs the
  per-level phrasing (findings ledger #5).
- **Production constants are out of Apalache's reach — measured, not
  assumed.** `production.qnt` (generated by `gen-production.mjs`; ROOT_H
  = 32, F = 256): n2Prod6 BMC produced **no verdict in 15 minutes at 32
  GiB** (default 4 GiB heap OOMs in ~3 min; a fresh 32 GiB server spent
  14.8 min in BoundedChecker without logging one step). openStage's
  F=256 symbolic run exhausted 32 GiB at depth 30 on three 512-entry
  havoc'd maps. The parametric claim moves to Lean.
- Tooling notes for re-runners: Apalache servers persist across quint
  invocations — a stale server on the default port 8822 silently absorbs
  later runs with its original heap; `JVM_ARGS` affects only freshly
  spawned servers; `check.sh induction` therefore uses dedicated ports
  (8872+). Known upstream issue quint#1989 (inductive mode can hang
  between obligations) — if a run wedges after "[1/3] … NoError", it's
  the tooling.

## Phase C: the Lean 4 artifact (`lean/`)

Toolchain: `leanprover/lean4:v4.32.0`, Batteries `v4.32.0`, mathlib-free
(pins in `lean/lean-toolchain`, `lean/lakefile.toml`). Layout:

- `StreamingMirror/Skel.lean` — skeletons + derived structure, name-for-
  name with the Quint spec (no `NSC`: Lean folds lengths).
- `StreamingMirror/Model.lean` — the executable transition system:
  `apply : Action → State → Option State` transcribing every Quint action
  branch; `stuck`/`terminal`; `Reachable`; `run_reachable` (action-list
  replays are reachability proofs — the ITF-witness bridge).
- `StreamingMirror/Instances.lean` — cross-pinning: the Phase A positive
  skeletons executed to `terminal` with conservation by a greedy
  scheduler, pinned by the `positives_complete` theorem (`native_decide`;
  kernel `decide` is impractical at trace length — a trust tradeoff to
  revisit).
- `StreamingMirror/Invariant.lean` — the Phase B invariant as an
  executable `Inv`, validated along entire executions of the pinned
  matrix (`inv_along_positives`).
- `StreamingMirror/Statement.lean` — **the statement of record**: the
  `DeadlockFree` target as a definition, the audit surface a skeptical
  reader must read (and what they need not: `Inv` and `Proofs/` are
  scaffolding, absent from the claim), the conservativity notes
  (`canStep` under-enumeration only strengthens the claim; `terminal` is
  the definition to scrutinize), and kernel-`decide`d non-vacuity
  witnesses for the skeleton class and reachability.

The port survived a six-lane adversarial transcription review (12 agents,
findings execution-verified). One HIGH finding, confirmed and fixed the
same hour: the walk/asm actions originally accepted arbitrary
`Party × Nat` keys while `State.walk` is a total function, and the
phantom walk `(R, rootH−1)`'s input channel aliases the real opening
channel — a phantom step could steal the opening message and deadlock a
positive instance. Quint was never exposed (`oneOf(walkKeys)` scopes its
keys); the Lean fix adds key membership to every walk/asm guard, and the
exploit trace is pinned as the must-fail `phantom_walk_rejected`
theorem.

**The parametric induction is proven** (`Proofs/`, ~4500 lines, zero
sorries): `inv_init` (every skeleton, no well-formedness needed — `init`
is self-consistent by construction), all 23 per-action preservation
lemmas, and the assembled `inv_preserved`/`inv_reachable` — the
invariant holds at every reachable state of EVERY well-formed skeleton,
in every axiom mode, with kernel-checked proofs resting only on
`[propext, Classical.choice, Quot.sound]` (no `native_decide` in the
proof chain; that trust lives only in the cross-validation pins). This
subsumes, parametrically, everything Apalache's consecution could check
on fixed small instances. Layout:

- `Proofs/Lemmas.lean` — conventions (the `InvP` Prop-level restatement,
  the no-`subst` successor-state discipline), occupancy algebra,
  prefix/frontier counting, `wellFormed` extraction.
- `Proofs/Wiring.lean` — channel↔count alignment and the setWalk flow
  frames: which channels can see a walk update, membership-relativized
  (the `wire I 0` Nat-subtraction phantom is excluded by `allChans`).
- `Proofs/Init.lean`, `Proofs/Preserve/{Top,Walk,WalkFire,Asm,
  AbsorbFin}.lean`, `Proofs/Preserve.lean` (the 23-way assembly).

Two transcription findings surfaced DURING the preservation proofs, both
executably countermodeled before being fixed by threading `wellFormed`:
the `askedOut`/`leafRequests` channel alias mis-routes for odd `rootH`
(responder stage 1 would exist and collide with the initiator leaf
stage), and the consumer-side flow frame is false at `rootH = 0`. Both
are exactly the class of wiring subtlety the flow layer exists to police;
neither is reachable for well-formed skeletons, and the Rust constants
(`rootH = 32`) are far inside the safe region. The guard-mirroring
transcription rule paid off structurally: in `walkCommit`, three of the
four committed-obligation arms close by definitional equality with the
guard, and only `.wire` needs real counting.

**Finding #6 (2026-07-16), surfaced by the progress-lemma design.** While
constructing the blame-graph acyclicity argument (blocked send ⇔ channel
full, blocked recv ⇔ channel empty; every blame edge must decrease a
potential), exactly one wait cycle refused to die: producer jammed on a
wire, consumer starving for an asked, asked's owner committed to the
jammed wire's sibling. No axiom cut it, and the cycle is realizable: the
kernel-checked witness in `StreamingMirror/Controls.lean` runs a
60-action schedule on a well-formed uneven-fan skeleton to a stuck state
under the pre-finding interface (`Control.jam_not_deadlockFree`), while
`Inv` holds throughout (safety was never at issue). The stuck state
satisfies every ledger `assert_valid` then checked — the wire stream was
simply never told to stay contiguous with its siblings' dependent work.
Resolution: `AxMode.d4` (wire sibling contiguity), guard + invariant
shadow + preservation re-proven; `Control.d4_rejects_trap` pins that the
strengthened interface refuses the schedule, and
`Control.jam_completes_full` pins that the skeleton still completes
greedily under it. Why three verification tiers missed it: the trap shape
is in no matrix instance (nearest miss `fanDepthPositive`, one query
short), the exhaustive tier never ran past depth 18 (trap depth ~60),
and 800-sample random simulation demonstrably misses narrow
committed-choice linearizations (`checkStage.sh`'s own stageNoW note).
Only the parametric progress *proof* was positioned to find it — and did,
before a line of it was formalized.

**The progress lemma is in flight; its design of record is
[PROGRESS.md](PROGRESS.md)** — read that before touching it. Landed so
far (2026-07-16): the enabledness pillar (`Proofs/Progress.lean`:
phase-2 uncommitted walks always have a choosable obligation, every
axiom mode — blocking is confined to channel operations); the counting
layer (`Proofs/Counting.lean`: whole-sweep supply = demand for every
channel family); the BFS-alignment conjunct in `wellFormed` (the
docstring's promise made checkable — a proof-method requirement for
positional timestamps, explicitly NOT a protocol finding: crossed-kid
skeletons stay count-consistent and complete); and the `eventdag`
control (`lake exe eventdag`), which checked the forced-order event DAG
acyclic on the full pinned matrix + jam with totals cross-validated
against `sentOf`/`recvdOf`. The refuted design alternatives live in
PROGRESS.md §4: not a closed-form lex formula, and not static
DFS positions either — stalls relocate walk-side events, so τ is
merge-emergent.

**The §5 schedule candidate has landed at the executable tier**
(`EventDag.schedCandidate`): the deterministic priority merge of the
per-process event traces, validated four ways in the tool's gate —
edge-check + permutation on the pins, greedy-trace coherence, replay
of the schedule as a real model run to `terminal` (each schedule is an
explicit termination witness), and a 300-seed random-skeleton sweep
with self-testing negative controls. The sweep also confirmed, both
directions with zero mismatches, the session's central finding:
**`wellFormed` alone does not imply schedulability** — the event DAG
is acyclic iff every scope has `dCount ≤ capLevel + 2`
(`pyramidC1` violates it, `jam` sits exactly on the boundary; Rust's
`capLevel = FAN` has margin 2), so the progress theorem must carry a
capLevel hypothesis. That hypothesis is now on the statement layer as
**`Skel.schedulable`** (the tight form — see PROGRESS.md §5 for the
decision rationale), with kernel-checked anchors: the hypothesis is
not implied by `wellFormed` (`pyramid1_not_schedulable`), the positive
matrix satisfies it (`positives_schedulable`), and the bound is exact
from both sides (`jam_on_boundary`). The Phase C target reads
`sk.wellFormed → sk.schedulable → DeadlockFree sk .full`.

The merge is transcribed (`Proofs/Sched.lean`: traces as prefix-sum
folds, the merge as a fuel-indexed fixpoint, pinned event-for-event to
the tool's `schedCandidate` by the gate), and the two by-construction
properties are kernel-checked theorems, generic over any trace list:
`trace_monotone` (each trace = an in-order-subsequence prefix of the
schedule + its actual unemitted remainder) and `schedule_e1`/`_e2`
(counted guard history at every emission index). An adversarial
review pass hardened the layer: `MInv.out_count` (output provenance,
added while the merge induction was open), the
`smokeChain_merge_complete` kernel anchor (non-vacuity — and the first
kernel-checked completeness instance), `Control.pyramid1_not_deadlockFree`
(the `schedulable` hypothesis's load-bearing-ness as a theorem, via a
kernel-decided greedy stuck run), and the tool's capLevel-parametric
boundary matrix (the ⟺ conjecture's exactness at capLevels the fuzz
envelope could not reach).

The canonical numbering layer is kernel-checked
(`Proofs/Sched/Numbering.lean`, its claims first validated
executably by the tool's `numberingErrs` gate): every trace projects,
on every channel-side, to consecutive seqs from zero (`procs_canon` —
the parent splice is proven projection-invisible), and ownership
(`sndOwner`/`rcvOwner`, one trace index per channel-side) makes the
producer unique, so the SCHEDULE's own projections are canon
(`schedule_proj_canon`). That upgrades counted E1 to positional —
"`snd(c,n)` precedes `rcv(c,n)`" (`schedule_e1_pos`) — and gives τ
its injectivity (`schedule_inj`), with `decide` anchors on the
smallest pin (`smokeChain_schedule_nodup`, `smokeChain_level_canon`).

The completeness invariant is decided and tool-validated
(PROGRESS.md §7 3b): a stalled merge state gives every blocked head a
unique blame target (canon + ownership + totals), and a *weak
potential* φ — strict across E1/E2 edges, weak along traces — makes
the argmin head a contradiction, with `Skel.schedulable` entering in
the level-channel E2 arithmetic. The tool's `blameProbe` checks the
whole reduction at every reachable merge state (owner unique, φ
drops, chains terminate) on all pins and acyclic fuzz seeds;
`pyramid 1` pins the negative (its probe finds the blame cycle, its
potential does not exist).

The potential itself is the *weave* (`EventDag.weaveOrder`,
tool-validated): a full topological order of the event DAG built by
structural recursion over the scope tree — query feeds thread each
scope's chunk queries down to its kids' descent, and the linear
assembly traces pump greedily after every emission. Position in the
weave is the potential; it validates (permutation + every edge) on
all pins, all 300 acyclic fuzz seeds, and the capLevel boundary
matrix, completing exactly ON `dCount = capLevel + 2` and failing one
past.

The weave is transcribed to the proof layer
(`Proofs/Sched/Weave.lean`, a fuel-indexed worklist interpreter whose
state IS the merge's `MState` and whose pump IS `mergeN`, pinned
event-for-event to the tool and kernel-anchored on the smallest pin),
and its permutation half is closed (`Weave/Count.lean` +
`Weave/Align.lean`): the `WCount` invariant recovers each manual
trace's unemitted remainder from the worklist by 3a's ownership
functions and rides the interpreter with no enabledness hypothesis,
and the alignment master induction (per-owner filters of a subtree op
are the manual traces' contiguous segments, the kid feeds resplicing
the chunk queries) discharges its hypotheses at the root scope op —
`weave_wcount` holds under `wellFormed` alone.

Next: weave edge-respect — `Skel.schedulable`
enters only in the pump-progress lemmas — then the blame-reduction
lemmas + argmin assembly closing merge completeness, the blame lemmas
(§6), and `deadlock_free`; then ITF-witness negative controls (incl.
the level-parameterized DropW existential) and termination — whose
witness the schedule construction already supplies executably.

## Phase map

- **A (done)**: this matrix.
- **B (done, re-scoped)**: invariant architecture validated on small
  instances (init + implication discharged; per-family consecution
  runner in `check.sh induction`); `openStage` assume-guarantee module
  with its two jam findings; production-constant BMC measured infeasible.
  The full-tower induction at FAN=256/DEPTH=32 is explicitly NOT claimed
  — the parametric result is Phase C's.
- **C (in progress)**: Lean 4 parametric safety theorem; core model built
  and cross-pinned to the Phase A matrix; **the inductive invariant is
  proven at every reachable state, parametrically (`inv_reachable`)**;
  **finding #6** (the wire-contiguity ledger gap) surfaced by the
  progress-lemma design, kernel-checked in `Controls.lean`, healed by
  `AxMode.d4` on both sides of the interface; progress lemma in flight
  per [PROGRESS.md](PROGRESS.md) — enabledness pillar and counting
  layer proven, event DAG checked acyclic on the pinned matrix,
  schedule construction open; then `deadlock_free` under the
  strengthened `.full`, ITF traces from the controls as constructive
  existentials.
- **D**: Lean termination (fairness-free corollary of safety + the ρ-by-1
  ranking).
- **E**: documentation closure; the findings above land as doc changes only
  when Finch signs off.
