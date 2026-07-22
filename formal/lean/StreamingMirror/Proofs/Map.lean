/-
The proof map: how the flagship theorems are assembled, file by file.

This module contains no code. It is the navigation document for the
proof stack — read it before reading any `Proofs/` file. The audit
surface (what the theorems CLAIM) is Statement.lean and deliberately
does not depend on anything here; this map is for the reader who wants
to know how the claims are DISCHARGED.

# The two theorems

Both corners of the parent-placement design space
(design/parent-placement.md) carry a kernel-checked deadlock-freedom
theorem, on the three standard axioms each:

- **The flagship** (the shipping encoder's order):
  `Sched.deadlock_free : wellFormed → (∀ s, dCount s ≤ capLevel) →
  DeadlockFree sk AxMode.impl` — Proofs/EndgameE.lean.
- **The d5 counterpart** (the weave's order, any capacity):
  `Sched.deadlock_free_d5 : wellFormed → schedulable →
  DeadlockFree sk AxMode.full` — Proofs/Endgame.lean.

Each is `progress` (every reachable non-terminal state can step) fed
into the reachability closure. The two proofs share every layer below
the trace family and differ exactly where the two encoder orders do:
the placement of each scope's parent summary.

The flagship additionally holds at every pointwise-widened capacity
vector: `Sched.deadlock_free_wide` (Proofs/Wide.lean) — `applyW κ`
beside the untouched `apply`, the track-G `InvPW` progress engine
consumed verbatim, the preservation sweep assembled from the mux
campaign's Steps extraction via chan-doctored companions, termination
transferred because ρ is chan-blind. This resolved the audit's
capacity-monotonicity quarantine by theorem.

# The shared foundation (order of reading)

1. **Model layer** — Basic, Skel, Model, Invariant, Instances,
   Controls, Statement (not under `Proofs/`): the skeleton vocabulary,
   the guarded step relation, the inductive invariant `Inv`, the
   pinned instances, the kernel-checked negative controls, and the
   audit surface. Everything below is scaffolding for theorems ABOUT
   this layer.
2. **Proofs/Lemmas** — the shared algebra: channel-occupancy `bump`,
   state-update projections, prefix counting.
3. **Proofs/Wiring** — which channel each count is observed on; the
   frame lemma (a walk update is invisible to other channels).
4. **Proofs/Init, Proofs/Preserve, Proofs/Preserve/**
   (Top/Walk/WalkFire/Asm/AbsorbFin) — the invariant induction:
   `Inv` at `init`, preserved by every action ⟹ `inv_reachable`.
5. **Proofs/Counting** — full-sweep supply totals per channel, from
   `wellFormed` alone: how much each producer owes over a whole
   session.
6. **Proofs/Progress** — the pillar: at a CHOICE point the
   committed-choice publisher always has a choosable obligation, in
   every axiom mode (`hmode` selects the corner). Blocking therefore
   only happens at channel operations — which is what the per-corner
   argmin arguments discharge.
7. **Proofs/Sched, Proofs/Sched/Numbering** — the canonical schedule
   (the E3-linear per-process traces and their priority merge,
   transcribed for proof; the executable oracle is EventDag.lean) and
   the per-channel numbering layer: on every channel-side the family's
   events carry seqs 0, 1, 2, … in order. τ (schedule position) is
   the potential both argmin arguments minimize.

# The per-corner chain

Each corner instantiates the same five-stage argument at its own trace
family (`procs` for d5, `procsE` for `.impl`); the invariant cores are
family-parameterized (`WCountP`/`WEdgeP`/`EmitOKOnP` + the `FamOK`/
`ManRows` fact bundles) with the d5 names kept as thin abbreviations.

Stage A, **the witness schedule**: a concrete completion order for the
whole session. Sched/Weave.lean (the weave; parent immediately after
the final D resolution) / Sched/WeaveE.lean (the eweave; parent as the
scope's last send). Both are executable-validated in EventDag before
anything is proven about them.

Stage B, **edge-respect**: the witness never sends into a full channel
nor receives ahead of supply — `weave_wedge` (Weave/Master.lean) /
`weaveE_wedge` (Weave/MasterE.lean). This is the bulk of the proof
stack:

- Weave/Count — the `WCountP` counting invariant of the interpreter;
- Weave/Expand, ExpandE — the fuel-free expansion ghost (what an op
  will emit, expansions included);
- Weave/Edge — `WEdgeP`: counting plus guard history, preserved
  generically;
- Weave/Prec, PrecE — dep-closure of the future (`DepOK`): every
  manual event's dependency is already behind it;
- Weave/Pump, Window, Ctx — the pump case tree: what a pump-facing
  emission demands of the tower above and below (the four windows);
- Weave/Emit, Site — the d5 counting route: `futLen` forms (what the
  remaining future holds, per channel), count pins, the
  ascent/descent telescopes and their site packages;
- Weave/SiteE, TeleE — the E counterparts (see the mirror table);
- Weave/Align, AlignE — the initial alignment: the opening worklist's
  per-owner filters ARE the traces;
- Weave/Master, MasterE — the consumption induction (`EmitOKOnP`:
  pointwise emission-readiness of the ghost future) and the master
  induction that produces it, site by site, with the rolling ancestor
  context (`AncTele`/`AncTeleE`).

Stage C, **merge completeness**: the witness's merge drains — every
trace embeds in the canonical schedule, making τ total along traces.
Weave/Final.lean (`merge_complete`) / Weave/FinalE.lean
(`merge_completeE`), by the drained-weave argument plus the blame-head
argmin.

Stage D, **the decode layer**: every determined process of a reachable
state sits AT a position of its trace — prefix performed, pending
event carrying the channel's current count. Proofs/Pending.lean /
Proofs/PendingE.lean. No reachability induction: the committed-arm
guard mirrors (d5's resp. d6's conjuncts) pin the performed prefix
statically.

Stage E, **the argmin endgame**: rank all pending events by τ and take
the least, `e*`. Its E1/E2 predecessors are performed (they sit
τ-below), so flow puts data (resp. room) on its channel and its owner
can fire; an empty pending pool close-cascades to `terminal`. Hence
`progress`, hence `deadlock_free`. Proofs/Endgame.lean /
Proofs/EndgameE.lean.

# The E/d5 mirror table

| d5 file | E file | the delta |
|---|---|---|
| Sched/Weave | Sched/WeaveE | parent moves to the scope tail |
| Weave/Expand | Weave/ExpandE | same ghost over the E ops |
| Weave/Align | Weave/AlignE | own-walk filter arms only; the upper-splice case splits vanish |
| Weave/Prec | Weave/PrecE | transferred (filter-preservation + Nodup), not re-derived |
| Weave/Emit + Site | Weave/SiteE | `childChunk_spliced`: an E kid chunk IS `splicedChunk … none`; whole-block projections equal |
| `AncTele` (in Master) | Weave/TeleE | no σ discriminant; ladders are base-rungs-only (`cases`, not induction) |
| Weave/Master | Weave/MasterE | per-kid chunks then ONE tail parent site; the U-site discharged by margin 0 + tower drainage instead of the telescopes |
| Weave/Final | Weave/FinalE | drain machinery family-generic; `ManRows` bridges the rows |
| Proofs/Pending | Proofs/PendingE | walk decode: `.wire`/`.query` arms lose `d5`, `.parent` gains the d6 everything-done mirror; the ~275-line splice analysis is gone |
| Proofs/Endgame | Proofs/EndgameE | margin 0 replaces `schedulable` (`margin0_schedulable`) |

The recurring shape: everywhere the d5 proof pays for the parent being
EARLY (splice discriminants, mid-chunk cursors, conditional ancestor
counts), the E proof gets a refund because the parent is LAST — the
two proofs differ exactly where the two encoders do.

# Epistemic frame

- **Kernel-checked**: every theorem in this tree, and the negative
  controls (Controls.lean) — `decide`-reduced, no `native_decide`.
- **Executable, gate-pinned** (EventDag.lean, `lake exe eventdag`):
  the schedulable ⟺ DAG-acyclicity conjecture (both directions, per
  sweep), transcription equality of `schedule`/`scheduleE` and both
  weave orders against the independent imperative model, replay to
  terminal under both modes, the adversarial drains (margin-0 `.impl`
  must complete; sub-margin stalls must reproduce), and the pinned
  skeletons and capLevel boundary matrix.
- **Assumed** (named, not proven): capacity monotonicity for the `d5`
  corner only (the `.impl` flagship's widening is
  `Sched.deadlock_free_wide`, Proofs/Wide.lean), and the
  modeled-world premises — see Statement.lean's "Assumed, not proven"
  section.
-/

namespace StreamingMirror.ProofMap
-- Documentation-only module (the sharded-slab pattern): a stable
-- rustdoc/olean anchor for the map above, with no definitions.
end StreamingMirror.ProofMap
