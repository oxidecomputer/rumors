/-
The statement of record: what this artifact claims, in definitions small
enough to audit by reading them.

# The audit surface

To believe `DeadlockFree sk AxMode.full` (once proven), a skeptical
reader must read, in full:

- `Skel.wellFormed` (Skel.lean, ~25 lines) — which dispute skeletons the
  claim covers;
- `Skel.schedulable` (Skel.lean, 2 lines) — the fan/back-pressure bound
  the progress lemma additionally assumes (`dCount ≤ capLevel + 2` per
  scope). It is not implied by `wellFormed`, and dropping it makes the
  target false: `pyramid1_not_schedulable` below exhibits a well-formed
  skeleton past the bound, and `Control.pyramid1_not_deadlockFree`
  kernel-checks that its greedy run under `.full` jams. That the
  hypothesis excludes ONLY sessions no interleaving whatsoever can
  finish is the event-DAG analysis's checked (not kernel-proven)
  equivalence — formal/PROGRESS.md §5;
- `AxMode` and `AxMode.full` (Skel.lean, 10 lines) — which send-order
  axioms are assumed; the mapping to the Rust `Trace::assert_valid`
  ledgers is the table in formal/README.md;
- `Model.apply` (Model.lean, ~150 lines) — the protocol model itself:
  every guard and every state delta. This is the irreducible core; it is
  trusted not by inspection alone but by cross-pinning (the Phase A
  matrix runs to completion inside Lean: `Pin.positives_complete`), by
  the adversarial transcription review (formal/README.md, Phase C), and
  by the must-fail regression `Pin.phantom_walk_rejected`;
- `Model.Reachable`, `Model.stuck`, `Model.terminal`, `Model.canStep`
  (Model.lean, a few lines each) — reachability is init plus closure
  under `apply`; stuck is "not finished and nobody can move".

The reader need NOT read: `Model.Inv` (the inductive invariant) or
anything under `Proofs/` — those are proof scaffolding, absent from the
statement.

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
still move or has completed. The target theorem is
`sk.wellFormed → sk.schedulable → DeadlockFree sk AxMode.full` (the
seven-ledger interface), pending the progress lemma. The mode index and
the `schedulable` hypothesis are each load-bearing, and each is a
THEOREM, not a promise: `Control.jam_not_deadlockFree` refutes this
very statement for the pre-finding-#6 interface (`Control.fullNoD4` —
everything but wire contiguity) by a kernel-checked stuck run on a
well-formed skeleton; `Control.parentTrap_not_deadlockFree` refutes it
for the pre-finding-#7 interface (`Control.fullNoD5` — everything but
parent placement) on a well-formed AND schedulable skeleton; and
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
