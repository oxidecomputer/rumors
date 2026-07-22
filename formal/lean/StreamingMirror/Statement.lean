/-
The statement of record: what this artifact claims, in definitions small
enough to audit by reading them.

# The two theorems

One statement shape, two theorems — one per corner of the
parent-placement design space (design/parent-placement.md), each on the
three standard axioms only (`propext`, `Classical.choice`,
`Quot.sound`; no `sorry`, no `native_decide`):

- **The flagship** (the shipping implementation's discipline):
  `Sched.deadlock_free : sk.wellFormed = true →
  (∀ s, sk.dCount s ≤ sk.capLevel) → DeadlockFree sk AxMode.impl`
  (Proofs/EndgameE.lean, via `Sched.progress`). The mode is the
  encoder's real send order (parent summary LAST in its scope — `d6`);
  the second hypothesis is the margin-0 capacity discipline the
  shipping code enforces (assembler capacity ≥ every scope's dispute
  count: `FAN ≥ kids`).
- **The counterpart** (the priced alternative design point):
  `Sched.deadlock_free_d5 : sk.wellFormed = true →
  sk.schedulable = true → DeadlockFree sk AxMode.full`
  (Proofs/Endgame.lean, via `Sched.progress_d5`). The mode is the
  parent-EARLY discipline (`d5` — no shipping encoder follows it);
  deadlock freedom holds at ANY assembler capacity, at the price of
  descent/assembly serialization (design/parent-placement.md §4–5).

# What a skeptical reader must read, in full

- `Skel.wellFormed` (Skel.lean, ~25 lines) — which dispute skeletons
  the claim covers: a real session shape (BFS-aligned scope stages,
  consistent child/dispute/query tables, `capLevel ≥ 1`);
- the capacity hypotheses. The flagship's `∀ s, dCount s ≤ capLevel`
  is the shipping regime read off the Rust (`FAN = 256 ≥` children per
  scope `≥` disputes per scope; `queues.rs`); it strictly implies
  `Skel.schedulable` (`dCount ≤ capLevel + 2`, Skel.lean, 2 lines),
  the counterpart's weaker bound, which is exactly the
  no-interleaving-can-finish frontier: `pyramid1_not_schedulable`
  below exhibits a well-formed skeleton one past it whose greedy run
  `Control.pyramid1_not_deadlockFree` kernel-checks stuck. That
  `schedulable` excludes ONLY sessions no interleaving whatsoever can
  finish is the event-DAG analysis's checked (not kernel-proven)
  equivalence — formal/PROGRESS.md §5. The margin between the two
  bounds is real and sharp: at `dCount = capLevel + 2` the
  parent-late order deadlocks under an adversarial interleaving
  (`Control.parentTrap`) even though the encoder's own poll schedules
  complete — the −2 boundary is poll-schedule-specific, margin 0 is
  the interleaving-robust line (design/parent-placement.md §2);
- `AxMode`, `AxMode.full`, `AxMode.impl` (Skel.lean, ~40 lines) — the
  send-order ledgers. In one sentence each: `w` — a child's wire
  precedes its internal publications; `d1root`/`d1int` — a resolution
  precedes its dependent queries (root/internal); `d2` — a parent
  resolution follows all its D-child resolutions; `d3` — sibling
  contiguity (child i's dependent work before child i+1's
  resolution); `d4` — wire contiguity (no wire departs over an
  earlier sibling's unresolved/unqueried debt); `d5` — parent-early
  (once every D child is resolved, the parent summary precedes any
  further wire or query); `d6` — parent-last (the parent summary is
  its scope's final send); `wireFirst` — control scaffolding, never
  asserted. `.full` = the first seven with `d5`; `.impl` = the first
  seven with `d6`. `d5` and `d6` contradict and are never combined;
- `Model.apply` (Model.lean, ~150 lines) — the protocol model itself:
  every guard and every state delta, quantifying over COMMITTED
  choice (a chosen action parks until it fires — the model of a task
  awaiting a bounded send) under arbitrary cross-process
  interleaving. This is the irreducible core; it is trusted not by
  inspection alone but by cross-pinning (the Phase A matrix runs to
  completion inside Lean: `Pin.positives_complete`), by the
  adversarial transcription review (formal/README.md, Phase C), and
  by the must-fail regression `Pin.phantom_walk_rejected`;
- `Model.Reachable`, `Model.stuck`, `Model.terminal`, `Model.canStep`
  (Model.lean, a few lines each) — reachability is init plus closure
  under `apply`; stuck is "not finished and nobody can move".

The reader need NOT read: `Model.Inv` (the inductive invariant) or
anything under `Proofs/` — those are proof scaffolding, absent from
the statement. (The map of that scaffolding, for readers of the
proofs, is Proofs/Map.lean.)

# The chain to the Rust implementation

The theorems are about any system obeying the ledgers; the Rust is
tied to them by checks at both ends (branch `parent-first`,
`src/tree/mirror/streaming/`):

- **Ledgers ⇐ traces.** `Trace::assert_valid`
  (`materialized/progress.rs`) checks every ledger of `AxMode.impl`
  on every encoder trace the streaming proptests produce: the
  wire/dependent/lower ledgers, sibling contiguity (`d3`), wire
  contiguity (`d4`), radix order, and — since finding #7 —
  `Trace::assert_parent_last` (= `d6`, mirrored verbatim from the
  model's guard). The `d5` corner deliberately has NO Rust check:
  `Trace::assert_parent_early` exists unwired, with a `should_panic`
  pin documenting that the real encoder violates parent-early — the
  design-space record, not a gap. The row-by-row mapping is the table
  in formal/README.md.
- **Margin 0 ⇐ configuration.** The capacity hypothesis is discharged
  by the shipping constant `FAN = 256` (`materialized/work/queues.rs`,
  the assembler channel's capacity) against the radix bound
  `kids ≤ 256`, pinned from both sides by
  `capacity_stress_witness_requires_inter_level_fan` (assembler
  high-water ≥ 254 on the [32,256] pyramid: the slack is consumed)
  and the `parent_delay_*` boundary probes (`tests/capacity.rs`).
- **Transcription boundary.** That the Lean definitions mean what the
  proofs need them to mean is the executable gate's job
  (EventDag.lean, `lake exe eventdag`): transcription equality of the
  proof-side schedules and both weave orders against an independent
  imperative model, the schedulable ⟺ DAG-acyclicity conjecture
  checked both directions, replay to terminal under both modes, and
  the margin-0 adversarial drains asserted — per 300-seed sweep, on
  every def-touching commit.

# Assumed, not proven

- **Capacity monotonicity (`d5` corner only)**: for the `.impl`
  flagship, widening is now a THEOREM — `Sched.deadlock_free_wide`
  (Proofs/Wide.lean): deadlock freedom plus the ρ(init) run bound at
  every pointwise capacity vector κ ≥ `sk.cap`, with `applyW_cap`
  pinning that κ = κ₀ recovers `apply` definitionally (the audit's
  capacity-monotonicity item, resolved by theorem 2026-07-21). What
  remains assumed is the `d5` corner's
  wire-widening: `deadlock_free_d5`'s chain still consumes the full
  `InvP`, so widened wire cells under the parent-early discipline rest
  on the informal Kahn argument (design/parent-placement.md §6) until
  Endgame.lean is re-typed over `InvPW`.
- **Modeled-world premises** (MODEL.md §1/§5): error-free conforming
  peers, SPSC channels, sequential scopes per walk, per-channel
  in-order delivery (the last is now also `assert_valid`'s radix-order
  rule).

# Conservativity notes

- `canStep` enumerates `allActions`. An accidental omission from that
  list makes `stuck` easier to satisfy, so `DeadlockFree` only gets
  HARDER to prove — the enumeration cannot silently weaken the claim.
- `terminal` is the definition that could weaken the claim if it held
  too early. The Phase A pins check conservation (all channels drained)
  at terminal executably; a planned corollary of `inv_reachable` makes
  that a theorem (see formal/README.md, Phase C).

# Non-vacuity

`wellFormed_satisfiable` and `reachable_init` below witness that the
hypotheses of the claim are inhabited: there are well-formed skeletons
(kernel-`decide`d, no native trust), and every skeleton has a reachable
state.
-/
import StreamingMirror.Model
import StreamingMirror.Instances

namespace StreamingMirror

open Model

/-- Deadlock-freedom, the Phase C target: under axiom mode `ax`, no
reachable state of the session is stuck — every interleaving either can
still move or has completed. PROVEN at both corners of
the parent-placement design space: the implementation-facing flagship
`sk.wellFormed → (∀ s, sk.dCount s ≤ sk.capLevel) → DeadlockFree sk
AxMode.impl` (`Sched.deadlock_free`, Proofs/EndgameE.lean, via
`Sched.progress`) — the shipping encoder's epilogue order at the
shipping margin-0 capacity discipline, `schedulable` subsumed by the
capacity hypothesis — and the capacity-universal counterpart
`sk.wellFormed → sk.schedulable → DeadlockFree sk AxMode.full`
(`Sched.deadlock_free_d5`, Proofs/Endgame.lean, via
`Sched.progress_d5`) — the weave's parent-early discipline, at any
capacity. The mode index and
the `schedulable` hypothesis are each load-bearing, and each is a
THEOREM, not a promise: `Control.jam_not_deadlockFree` refutes this
very statement for the pre-finding-#6 interface (`Control.fullNoD4` —
everything but wire contiguity) by a kernel-checked stuck run on a
well-formed skeleton; `Control.parentTrap_not_deadlockFree` refutes it
for the pre-finding-#7 interface (`Control.fullNoD5` — everything but
parent placement, in either corner: the capacity hypothesis of the
`d6` flagship is exactly what defuses its trap) on a well-formed AND
schedulable skeleton; and
`Control.pyramid1_not_deadlockFree` refutes it under `.full` for
`Pin.pyramid 1` — well-formed, one D child past the `schedulable`
bound (`pyramid1_not_schedulable` below), greedy run kernel-checked
stuck. -/
def DeadlockFree (sk : Skel) (ax : AxMode) : Prop :=
  ∀ s : State, Reachable sk ax s → stuck sk ax s = false

/-- The smallest Phase A skeleton is well-formed, by kernel reduction
(no `native_decide` trust): the claim's skeleton class is inhabited. -/
theorem smokeChain_wellFormed : Pin.smokeChain.wellFormed = true := by
  decide

/-- Non-vacuity of the skeleton class. -/
theorem wellFormed_satisfiable : ∃ sk : Skel, sk.wellFormed = true :=
  ⟨Pin.smokeChain, smokeChain_wellFormed⟩

/-- The `schedulable` hypothesis is not redundant: a well-formed
skeleton can violate it. `pyramid 1` is the witness (4 D kids under one
parent, `capLevel = 1`, one past the bound) — the Phase A negative whose
greedy jam the event-DAG analysis upgraded to "no schedule completes
it". Kernel-`decide`d, like the anchors around it. -/
theorem pyramid1_not_schedulable :
    (Pin.pyramid 1).wellFormed = true ∧ (Pin.pyramid 1).schedulable = false := by
  decide

/-- The positive Phase A matrix sits inside the progress lemma's
hypothesis class: every pinned skeleton that completes is schedulable
(the boundary case, `Control.jam`, is pinned in Controls.lean, which
this file cannot import). -/
theorem positives_schedulable :
    Pin.smokeChain.schedulable && Pin.rMix.schedulable &&
    Pin.comb6.schedulable && (Pin.pyramid 4).schedulable &&
    (Pin.pyramid 2).schedulable = true := by
  decide

/-- Non-vacuity of reachability: the initial state is always reachable,
so `DeadlockFree` quantifies over an inhabited set. -/
theorem reachable_init (sk : Skel) (ax : AxMode) :
    Reachable sk ax (init sk) :=
  .init

end StreamingMirror
