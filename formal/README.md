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
| radix order ("violates radix order") | the per-channel in-order program structure (always on) |

The sibling-contiguity check **exists because of this model**: the original
three ledgers admitted a "publish all wires, then all resolutions, then all
queries" implementation that passes `assert_valid` and deadlocks the cap-1
child-resolution queue at fan ≥ 3. The `ledgerGap` instance is the durable
witness; `assert_valid` was tightened the same day (2026-07-15).

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

Next: the canonical-order progress lemma (`Inv ∧ ¬terminal → canStep`),
`deadlock_free`, ITF-witness negative controls (incl. the
level-parameterized DropW existential), termination.

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
  remaining: the progress lemma → `deadlock_free`, ITF traces from the
  controls as constructive existentials.
- **D**: Lean termination (fairness-free corollary of safety + the ρ-by-1
  ranking).
- **E**: documentation closure; the findings above land as doc changes only
  when Finch signs off.
