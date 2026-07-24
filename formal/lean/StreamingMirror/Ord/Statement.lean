/-
The audit surface for the order-indifference metatheorem: what
`Sched.deadlock_free_anyOrder` claims, in definitions small enough to
audit by reading them, plus the executable anchors that pin the O
transition function against the pinned skeleton matrix.

# The theorem

**`Sched.deadlock_free_anyOrder : ∀ (ord : OrdMap),
sk.wellFormed = true → (∀ sc, sk.dCount sc ≤ sk.capLevel) →
DeadlockFreeO sk AxMode.impl ord`** (Ord/Endgame.lean, via
`Ord.progressO`): under the shipping encoder's send-order ledgers
(`AxMode.impl` — parent summary last in its scope, `d6`) and the
shipping margin-0 capacity discipline (assembler capacity at least
every scope's dispute count, `FAN ≥ kids`), no reachable state of the
session is stuck under ANY assignment of the two-point per-loop
dequeue-order class. Termination rides along at every assignment
(`Ord.terminatingO`: every `applyO` run from `init` is ρ(init)-bounded;
`Sched.maximal_run_terminal_anyOrder` / `greedy_run_terminal_anyOrder`
close maximal and greedy runs to `terminal`).

# The operational reading (what "any order" means, exactly)

Each pairing loop of the session — each walk stage, and the absorber —
dequeues, per scope (per leaf request, for the absorber), one wire
reply and one queued query. The class quantified over is the TWO-POINT
per-loop choice of which comes first (`PairOrder`, Ord/Basic.lean):

- **reply-first**: recv wire at phase 0, recv asked at phase 1; the
  end-of-stream closes take wire at phase 3, asked at phase 4 — the
  baseline transcription. `applyO sk ax .rf = Model.apply sk ax`
  definitionally (`applyO_rf`, `rfl` per arm), so the base flagships
  ARE the `ord = .rf` instances of the quantified statements.
- **query-first**: recv asked at phase 0, recv wire at phase 1; closes
  asked at phase 3, wire at phase 4 — the shipping Rust's loop order.
  The close order is TIED to the loop's prologue choice (the Rust loop
  exits on whichever queue it dequeues first going closed); the model
  does not treat closes as a separate choice point.

An `OrdMap` assigns one `PairOrder` to every walk stage and one to the
absorber, each independently — 2^(#stages + 1) assignments per
skeleton. Sends never move in the class: every scope's publication
suffix is the encoder order at every assignment.

The margin-0 hypothesis is the shipping regime read off the Rust
(`FAN = 256 ≥` children per scope `≥` disputes per scope;
`materialized/work/queues.rs`); it subsumes `Skel.schedulable` and is
load-bearing at every assignment — `pdelay_qf_greedy_stuck` below
pins the sub-margin failure at the query-first corner.

# The exclusions (named honestly)

This is per-loop dequeue-order indifference over the two-point class,
NOT arbitrary-order indifference. Explicitly outside the class, per
MODEL.md §5's dequeue-order amendment ("loop shapes that are not a
two-point dequeue choice; none is the shipping Rust, and none is
covered"):

- racing both inputs (a `select!` over reply queue and query queue,
  order resolved per message at runtime);
- cross-scope prefetching (dequeuing scope k+1's inputs before scope
  k's obligations complete — the sequential-scope premise stays in
  force);
- a prologue interleaved with the scope's sends (both receives always
  precede every publication of the scope).

Nothing is claimed for these shapes.

# The reply-first residue (deliberate scope, per MODEL.md)

`Sched.deadlock_free_d5` (the parent-early design-space corner) and
the mux flagships (`wc_impossibility`, `sigmaStar_deadlock_free` and
kin, `elastic_deadlock_free`, `mux_terminating`) stay certifying the
baseline reply-first order only — dated scope decisions recorded in
MODEL.md's residue table, not gaps discovered later. No claim is made
about a query-first `d5` corner or a query-first mux.

# What a skeptical reader must read, in full

`Skel.wellFormed`, `AxMode.impl`, `Model.Reachable`/`stuck`/`terminal`
exactly as for the base flagship (Statement.lean) — plus, new here:
`PairOrder`/`OrdMap`/`applyO` (Ord/Basic.lean, ~100 lines): the eight
re-phased prologue/close arms, every other arm delegating to
`Model.apply` by a definitional catch-all, and the target predicates
`ReachableO`/`canStepO`/`stuckO`/`DeadlockFreeO` (each a few lines,
shaped exactly like the base predicates over `applyO`). The proof
scaffolding under Ord/ is absent from the statement.

# Trust note on the anchors below

The metatheorem's own chain is kernel-checked (the lead audits its
axioms; `native_decide` appears nowhere in it). The anchors in THIS
file are `native_decide` executables — replay pins in the Phase A
tradition (`Pin.invAlong`), trusted like the rest of the executable
tier, deliberately separate from the theorem's trust base.

Chain (ord, stage E exit): the audit surface. Base mirror:
Statement.lean. Map: PROGRESS.md §13.
-/
import StreamingMirror.Ord.Endgame

namespace StreamingMirror.Ord

open Model

/-- Drive `fuel` greedy `applyO` steps, checking the O invariant at
every state along the way — `Pin.invAlong` re-targeted at the
assignment (the executable transcription check for the O transition
layer). -/
def invAlongO (sk : Skel) (ax : AxMode) (ord : OrdMap) :
    Nat → State → Bool
  | 0, s => InvO sk ax ord s
  | fuel + 1, s =>
      InvO sk ax ord s &&
      match (allActions sk).firstM (fun a => applyO sk ax ord a s) with
      | some s' => invAlongO sk ax ord fuel s'
      | none => InvO sk ax ord s

/-- A concrete mixed assignment: even-height walks (the R stages, and
the absorber's leaf feeders) query-first, odd-height walks (the I
stages) reply-first, the absorber query-first — a genuine per-loop mix
exercising both dispatch arms in one session. -/
def mixSample : OrdMap :=
  ⟨fun pk => if pk.2 % 2 == 0 then .queryFirst else .replyFirst,
    .queryFirst⟩

/-- The O invariant holds along entire greedy `.impl` executions of
the pinned positive matrix at the all-query-first assignment — the
shipping Rust's corner of the class, replayed against `InvO`. -/
theorem invO_along_positives_qf :
    (invAlongO Pin.smokeChain .impl .qf 300 (init Pin.smokeChain)) &&
    (invAlongO Pin.rMix .impl .qf 500 (init Pin.rMix)) &&
    (invAlongO Pin.comb6 .impl .qf 600 (init Pin.comb6)) &&
    (invAlongO (Pin.pyramid 4) .impl .qf 700 (init (Pin.pyramid 4))) &&
    (invAlongO (Pin.pyramid 2) .impl .qf 700 (init (Pin.pyramid 2)))
      = true := by
  native_decide

/-- The O invariant holds along the same greedy executions at the
mixed assignment: the per-loop dispatch is exercised in both
directions inside one session, not only at the uniform corners. -/
theorem invO_along_positives_mix :
    (invAlongO Pin.smokeChain .impl mixSample 300 (init Pin.smokeChain)) &&
    (invAlongO Pin.rMix .impl mixSample 500 (init Pin.rMix)) &&
    (invAlongO Pin.comb6 .impl mixSample 600 (init Pin.comb6)) &&
    (invAlongO (Pin.pyramid 4) .impl mixSample 700
      (init (Pin.pyramid 4))) &&
    (invAlongO (Pin.pyramid 2) .impl mixSample 700
      (init (Pin.pyramid 2)))
      = true := by
  native_decide

/-- Non-vacuity of the query-first corner: greedy `applyO` drains run
the pinned sessions to `terminal` at the shipping order — the
executable face of `greedy_run_terminal_anyOrder` on the matrix. -/
theorem qf_drain_completes :
    terminal Pin.smokeChain
      (drainO Pin.smokeChain .impl .qf 300 (init Pin.smokeChain)) &&
    terminal (Pin.pyramid 2)
      (drainO (Pin.pyramid 2) .impl .qf 700 (init (Pin.pyramid 2)))
      = true := by
  native_decide

/-- The NEGATIVE control: the capacity hypothesis is load-bearing at
query-first too. `Control.pdelay` sits sub-margin
(`dCount 1 = capLevel + 2`, `Control.pdelay_on_boundary`) and its
greedy query-first `.impl` drain ends stuck — dropping the margin-0
hypothesis makes the metatheorem's statement false at the shipping
corner of the class, exactly as `Control.parentTrap` falsifies it at
reply-first. -/
theorem pdelay_qf_greedy_stuck :
    stuckO Control.pdelay .impl .qf
      (drainO Control.pdelay .impl .qf 600 (init Control.pdelay))
      = true := by
  native_decide

/-- The negative control at the mixed assignment: the sub-margin stick
is not an artifact of the uniform query-first corner. -/
theorem pdelay_mix_greedy_stuck :
    stuckO Control.pdelay .impl mixSample
      (drainO Control.pdelay .impl mixSample 600 (init Control.pdelay))
      = true := by
  native_decide

/-- Non-vacuity of O reachability: the initial state is `ReachableO`
at every assignment, so `DeadlockFreeO` quantifies over an inhabited
set (cf. `reachable_init`). -/
theorem reachableO_init (sk : Skel) (ax : AxMode) (ord : OrdMap) :
    ReachableO sk ax ord (init sk) :=
  .init

end StreamingMirror.Ord
