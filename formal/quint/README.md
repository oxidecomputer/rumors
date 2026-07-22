# The Quint tier: runner's manual

The base campaign's model-checking artifacts: a Quint model of the
streaming mirror protocol ([`streamingMirror.qnt`](streamingMirror.qnt))
plus a validation matrix ([`instances.qnt`](instances.qnt)) that
reproduces the Rust suite's known completions, the capacity-tightness
threshold, and — under relaxed axioms — the deadlocks that prove each
axiom is load-bearing. The documentation of record is the Lean artifact
([`../lean/StreamingMirror/Statement.lean`](../lean/StreamingMirror/Statement.lean));
the protocol model both tiers transcribe is [`../MODEL.md`](../MODEL.md).
This file is the operational manual for running the Quint tier.

## Toolchain (pinned)

- Quint **0.32.0**, pinned in [`package.json`](package.json); run
  everything via `npx quint` from `formal/quint/`.
- JDK for Apalache runs: `/opt/homebrew/opt/openjdk/bin/java` (the system
  `/usr/bin/java` is a stub); [`check.sh`](check.sh) sets `PATH` itself.
- `tla2tools.jar` (sha256 pinned in
  [`../tla/tla2tools.jar.sha256`](../tla/tla2tools.jar.sha256)) is
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
                      # dumps per pinned skeleton (../PROGRESS.md §3);
                      # nonzero exit on any failed check
```

[`check.sh`](check.sh) encodes every expectation; a control instance
passes only when the checker **finds** a stuck state. Per-instance BMC
depth is the spec's own `totalSteps` bound, computed via the REPL —
every action draws on a finite skeleton-derived budget, so runs are
bounded and checking to that depth is exhaustive
([`../MODEL.md`](../MODEL.md) §7).

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
tier exists in [`check.sh`](check.sh) and is honest about its cost, but
exhaustiveness was never achieved on any instance. Finding #6 (the
wire-contiguity ledger gap, [`../MODEL.md`](../MODEL.md) §6, Axiom D4)
is the consequence made concrete: its trap needs ~60 steps and a
skeleton shape (uneven fan ≥ 3 with an early D child owing ≥ 2 queries)
that no matrix instance has — `fanDepthPositive` misses it by exactly
one query. Deadlock-freedom claims rest on the Lean artifact
([`../lean/StreamingMirror/Statement.lean`](../lean/StreamingMirror/Statement.lean)),
nowhere else.

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
in [`../MODEL.md`](../MODEL.md) §4), all positive instances reach
`Terminal` in 100% of sampled schedules, and the pyramid family exercises
backpressure (`blockedSend`).
