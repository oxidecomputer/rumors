/-
The mux proof map: how the campaign's statements of record are
discharged, file by file — the Mux/ extension of Proofs/Map.lean (read
that first for the base artifact's layers; everything here builds on
them). This module contains no code. The audit surface — what the
theorems CLAIM — is Mux/Statement.lean and deliberately depends on
nothing here; this map is for the reader of the proofs.

# The transport dial: one harness, three deliver arms

Every composition in the suite is the SAME harness — Mux/Basic.lean's
`MState`/`MAction`/`apply`, with the base arms and the strategy-gated
`push` shared definitionally — differing only in the deliver arm's
parking bound:

| semantics | deliver arm | parking bound | home |
|---|---|---|---|
| record | `deliverStep` | cap-1 demux slot | Mux/Basic.lean (`apply`; the arm named in Proofs/WcImpossibility.lean) |
| K-deep | `deliverStepK KI KR` | the RECEIVING party's advertised depth | Proofs/WcImpossibilityK.lean (`applyK`) |
| elastic | `deliverStepE` | unbounded | Mux/Elastic.lean (`applyE`) |

The relations are kernel-pinned: `deliverStepK_one` (depths (1, 1) ARE
the record slot, definitionally), and `applyU_eq_applyE`
(Proofs/Twins.lean: the Mux/Controls.lean unbounded-slot control
`applyU` IS the elastic semantics). Reading the dial: the record
harness is the K = (1, 1) point, the elastic composition the K = ∞
endpoint, and T8's positive half — when it lands — quantifies over
`applyK`'s two depths between them (T8-SPEC.md).

# The invariant family: the canonical map

Base-model invariants (Proofs/Lemmas.lean; states of the UN-muxed
system):

- `InvP` — the full reachable-state package: the local cursor
  consistency conjuncts plus flow conservation plus the capacity
  bound. What `inv_reachable` maintains.
- `InvPW` — `InvP` minus the capacity half (conservation without
  `chan ≤ cap`). What the progress engine actually consumes
  (`Sched.progress_of_inv`), minted because parked elastic states
  over-fill wire cells by design. Projection: `InvP.weak`.
- `InvL` — the flow-free local fragment (cursors only). What the muxed
  decode layer runs on: a muxed state with frames in flight satisfies
  NO unmuxed conservation law. Projection: `InvP.local`.

Muxed-system invariants (states of a composition; each is preserved
along its own reachability and consumed as a ground-fact interface):

- `MuxInv` (Proofs/Chase/Ground.lean) — the record transport's ground
  facts: `InvL`, the slot bound, internal-channel flow, the hist/pipe
  FIFO correspondence, and the two `RealWire`-guarded count equations
  (`pushed_eq`, `delivered_eq`; the guard discipline and the phantom
  trap it ends are documented there). With both pipes drained it
  collapses to `InvP` (`MuxInv.invP`).
- `HistInv` (Proofs/Inhabitation.lean) — the hand ledger:
  commit-vs-flush counts decode `holdsWire`. Mode- and shape-generic
  (no `wellFormed`), which is what lets the class-inhabitation
  certificates quantify over `MReachableAny` and its K/E twins.
- `SInv` (Proofs/SigmaStarInv.lean) = `MuxInv` + `HistInv`, preserved
  in ONE strategy-generic sweep (`sinv_reachable`) — the record
  transport's single preservation induction; `muxInv_reachable` is its
  `MuxInv` projection and T5's assembly consumes it verbatim.
- `EMuxInv` (Mux/Elastic.lean) — the elastic twin: `InvL`, internal
  flow, `RealWire`-guarded wire flow through the pipe, and pipe
  content — minus every occupancy bound (parking is unbounded by
  design) and minus the history ledger. With pipes drained it
  collapses to `InvPW` (`EMuxInv.invPW`), deliberately NOT to `InvP`.
  Preserved by `eMuxInv_reachable` (assembled from the same Steps
  deltas as `sinv_reachable`).
- `RecvLedger` (Proofs/CausalCoverage.lean) — recorded wire receives
  never outrun the base consumer counts and name real channels: the
  ground fact that makes the causal closure's own-receive evidence arm
  sound. Strategy-generic; `recvLedger_reachable`.
- `OracleInv` (Proofs/Oracle/Order.lean) — oracle-run-specific: a
  machine's pushes are exactly the τ-prefix of its send projection.
  History-only preservation, disjoint from the ground-fact sweep;
  turns FIFO pipe positions into τ positions at a stuck state.

Implication skeleton (arrows are the named projections; the starred
ones need both pipes drained):

    SInv ─→ MuxInv ─*→ InvP ─→ InvPW ─→ (local) InvL
                EMuxInv ─*→ InvPW
    SInv ─→ HistInv        RecvLedger, OracleInv: side ledgers

# The statement-by-statement discharge map

- **T1 `commit_totality`** (Proofs/CommitTotality.lean): unique
  choosable obligation per walk at `.impl`-reachable states, over the
  `InvL` fragment (`commit_unique`).
- **T2 `keystone` + `chase`** (Proofs/Chase/, over `MuxInv`): at a
  stuck drained state the τ-least unperformed event is a withheld wire
  push with its DAG past performed. The decode layer
  (Chase/Decode.lean) and closure theory (Chase/Closure.lean) feed it.
- **T3 `wc_impossibility`** (Proofs/WcImpossibility.lean): the σ-free
  forced-run executor + the b-bounded replay lift + four kernel
  anchors (pipe-full parks at C ∈ {1,2,3}, the capacity-blind
  `noHands` burial for C ≥ 4). Controls in Mux/Controls.lean.
- **T8's impossibility half `wc_impossibility_K`**
  (Proofs/WcImpossibilityK.lean): the same machinery with the deliver
  arm re-gated; 12 anchors across KR ∈ {1,2,3}.
- **T4 `sigmaStar_deadlock_free`** (Proofs/SigmaStarLive.lean, over
  `sinv_reachable`): push certificates drain the pipes, the chase
  names the withheld push, the τ-staged `closure_coverage` proves it
  demanded, σ* names it.
- **The charter-grain re-run** (Mux/Causal.lean the strategy and
  locality; Proofs/CausalCoverage.lean Steps 1–3 groundwork +
  `announcedProcs_prefix` + `RecvLedger` + `keystoneA`;
  Proofs/CausalLive.lean the conditional assembly;
  Proofs/CausalMint.lean the minting ladder + `causalStuckCoverage` +
  the unconditional `sigmaStarCausal_deadlock_free`). The C1 verdicts
  assemble in Proofs/C1.lean; the grain controls in Proofs/Grains.lean.
- **T5 `oracle_deadlock_free`** (Proofs/Oracle/Order.lean the
  projection and `OracleInv`; Proofs/Oracle.lean the τ-argmin and
  head-cycle argument; Proofs/Oracle/Controls.lean the T9 controls and
  the refuted receive projection). Assembled unconditionally in
  Proofs/Necessity.lean, which also holds **T6 `necessity`**.
- **The elastic endpoint `elastic_deadlock_free`** (Mux/Elastic.lean):
  reduction to the base progress engine through `InvPW`; no new
  liveness induction.
- **Termination** (Proofs/Termination.lean): `mrho = 2·ρ(base) + Σ
  |pipe|` strictly decreases on every arm of all three deliver
  variants; `mux_terminating` bounds every run, and the greedy/maximal
  run corollaries ground every "completes".
- **Class inhabitation** (Proofs/Inhabitation.lean): the `HistInv`
  sweep certifies the shipped policy work-conserving on all three
  universes and legacy-local; the idler pins `LocalStrategy` inhabited.
- **Drift guards** (Proofs/Twins.lean): the executable tier
  (Mux/Gen.lean, muxprobe) runs definitionally the kernel objects.

# Epistemic frame

- **Kernel-checked**: every theorem named above, `decide`-only (no
  `native_decide` anywhere in the Mux suite).
- **Executable, gate-pinned**: the muxprobe golden matrix
  (`just muxprobe`) over the same definitions; the stage-0 probe
  validated the PYTHON counterpart of σ*-causal (divergence axes:
  Mux/Causal.lean's module doc).
- **Assumed / open**: the byte-denomination boundary, the KR ≥ 4
  tail, the `.full` port, the legacy-grain hypotheses of
  `c1_literal_false` — Mux/Statement.lean's "Assumed, not proven".
-/

namespace StreamingMirror.MuxProofMap
-- Documentation-only module (the sharded-slab pattern, as
-- Proofs/Map.lean): a stable anchor for the map above, no definitions.
end StreamingMirror.MuxProofMap
