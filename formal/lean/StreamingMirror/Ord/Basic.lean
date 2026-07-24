/-
The dequeue-order class (MODEL.md §5's order-indifference amendment,
2026-07-23): the ord-parameterized transition function and the
metatheorem's target predicates.

Each pairing loop of the session — each walk stage, and the absorber —
dequeues one wire reply and one queued query per scope (per leaf
request, for the absorber), and the class quantified over is the
TWO-POINT per-loop choice of which comes first: reply-first (the
baseline transcription, `Model.apply`'s order) or query-first (the
shipping Rust's order since fd36bb65). The end-of-stream close order
is TIED to the loop's prologue choice — the Rust loop exits on
whichever queue it dequeues first going closed — so a `PairOrder`
fixes both. Everything else (`walkCommit`/`walkFire`, the publication
axiom guards, assemblers, openers, finishes) is shared with
`Model.apply` arm-for-arm.

This is per-loop dequeue-order indifference over a two-point class,
NOT arbitrary-order indifference: loop shapes outside the class
(racing both inputs, cross-scope prefetching, prologues interleaved
with sends) are named and excluded in MODEL.md's amendment.

Non-destructive by construction: this module family (`Ord/`) only
imports the base model; no existing proof file changes. The
reply-first instance recovers `Model.apply` definitionally
(`applyO_rf`, `rfl` per arm), which is what transports the baseline
flagships into the quantified statements' `ord = .rf` corner.

Chain (ord, stage 0): the transition layer; consumed by every Ord/
module. Map: Proofs/Map.lean (base campaign), PROGRESS.md §13 (this
campaign).
-/
import StreamingMirror.Model
import StreamingMirror.Statement

namespace StreamingMirror.Ord

open Model

/-- Which end of a pairing loop dequeues first (fd36bb65's flip):
reply-first awaits the wire reply then dequeues the paired query;
query-first dequeues the query then awaits the reply, with the
end-of-stream checks flipped to match. One value per pairing loop
(each walk stage, and the absorber). -/
inductive PairOrder | replyFirst | queryFirst
  deriving DecidableEq, Repr

/-- Per-loop order assignment: each walk stage and the absorber
choose independently. -/
structure OrdMap where
  walk : Party × Nat → PairOrder
  absorb : PairOrder

/-- The all-reply-first assignment: the baseline transcription. -/
def OrdMap.rf : OrdMap := ⟨fun _ => .replyFirst, .replyFirst⟩

/-- The all-query-first assignment: the shipping Rust's order. -/
def OrdMap.qf : OrdMap := ⟨fun _ => .queryFirst, .queryFirst⟩

/-- The phase at which a loop with assignment `o` performs its wire
receive (`0` or `1`); the paired query receive takes the other slot. -/
def PairOrder.wirePhase : PairOrder → Nat
  | .replyFirst => 0
  | .queryFirst => 1

/-- The phase at which a loop with assignment `o` performs its wire
close wait (`3` or `4`); the query close takes the other slot. -/
def PairOrder.wireClosePhase : PairOrder → Nat
  | .replyFirst => 3
  | .queryFirst => 4

variable (sk : Skel) (ax : AxMode) (ord : OrdMap)

/-- The ord-parameterized transition function: `Model.apply` with the
prologue and close arms of each pairing loop reading the loop's
assigned order. All other arms are shared bodies (the final catch-all
delegates to `Model.apply`), so the publication machinery — commit
guards, fire, assemblers, openers, finishes — is the base model's by
definitional equality. -/
def applyO (a : Action) (s : State) : Option State :=
  match a with
  | .walkRecvWire pk =>
      let ws := s.walk pk
      let c := wireIn pk
      let ph : Nat := match ord.walk pk with | .replyFirst => 0 | .queryFirst => 1
      if sk.walkKeys.contains pk && ws.phase == ph && s.chan c > 0 then
        some (setWalk { s with chan := bump s.chan c (-1) } pk
          (if ph == 1 then normWalk sk pk.2 { ws with phase := 2, committed := none }
           else { ws with phase := 1, committed := none }))
      else none
  | .walkRecvAsked pk =>
      let ws := s.walk pk
      let c := askedIn pk
      let ph : Nat := match ord.walk pk with | .replyFirst => 1 | .queryFirst => 0
      if sk.walkKeys.contains pk && ws.phase == ph && s.chan c > 0 then
        some (setWalk { s with chan := bump s.chan c (-1) } pk
          (if ph == 1 then normWalk sk pk.2 { ws with phase := 2, committed := none }
           else { ws with phase := 1, committed := none }))
      else none
  | .walkCloseWire pk =>
      let ws := s.walk pk
      let ph : Nat := match ord.walk pk with | .replyFirst => 3 | .queryFirst => 4
      if sk.walkKeys.contains pk && ws.phase == ph
          && producerDone sk s (wireIn pk) && s.chan (wireIn pk) == 0 then
        some (setWalk s pk { ws with phase := ph + 1 })
      else none
  | .walkCloseAsked pk =>
      let ws := s.walk pk
      let ph : Nat := match ord.walk pk with | .replyFirst => 4 | .queryFirst => 3
      if sk.walkKeys.contains pk && ws.phase == ph
          && producerDone sk s (askedIn pk) && s.chan (askedIn pk) == 0 then
        some (setWalk s pk { ws with phase := ph + 1 })
      else none
  | .absorbRecvWire =>
      let c := Chan.wire Party.R 0
      let ph : Nat := match ord.absorb with | .replyFirst => 0 | .queryFirst => 1
      if s.absorbPhase == ph && s.chan c > 0 then
        some { s with chan := bump s.chan c (-1), absorbPhase := ph + 1 }
      else none
  | .absorbRecvAsked =>
      let ph : Nat := match ord.absorb with | .replyFirst => 1 | .queryFirst => 0
      if s.absorbPhase == ph && s.chan Chan.leafRequests > 0 then
        some { s with chan := bump s.chan Chan.leafRequests (-1), absorbPhase := ph + 1 }
      else none
  | .absorbCloseWire =>
      let c := Chan.wire Party.R 0
      let ph : Nat := match ord.absorb with | .replyFirst => 3 | .queryFirst => 4
      if s.absorbPhase == ph && producerDone sk s c && s.chan c == 0 then
        some { s with absorbPhase := ph + 1 }
      else none
  | .absorbCloseAsked =>
      let ph : Nat := match ord.absorb with | .replyFirst => 4 | .queryFirst => 3
      if s.absorbPhase == ph && producerDone sk s Chan.leafRequests &&
          s.chan Chan.leafRequests == 0 then
        some { s with absorbPhase := ph + 1 }
      else none
  | a => Model.apply sk ax a s

/-- The reply-first instance IS the model of record, pointwise: the
baseline theorems are the `ord = .rf` corner of every quantified
statement. `rfl` per arm — no reasoning, a definitional identity. -/
theorem applyO_rf (a : Action) (s : State) :
    applyO sk ax .rf a s = Model.apply sk ax a s := by
  cases a <;> rfl

/-- Reachability under an order assignment (cf. `Model.Reachable`). -/
inductive ReachableO : State → Prop
  | init : ReachableO (Model.init sk)
  | step {s s' : State} (a : Action) :
      ReachableO s → applyO sk ax ord a s = some s' → ReachableO s'

/-- Some process can act under the assignment (cf. `Model.canStep`;
the action alphabet is the base model's). -/
def canStepO (s : State) : Bool :=
  (allActions sk).any fun a => (applyO sk ax ord a s).isSome

/-- The deadlock predicate under the assignment (cf. `Model.stuck`). -/
def stuckO (s : State) : Bool := !terminal sk s && !canStepO sk ax ord s

/-- The metatheorem's target shape: no reachable state of the session
is stuck, at THIS per-loop dequeue-order assignment. The flagship
quantifies it over every assignment of the two-point class — and over
nothing else (MODEL.md's exclusions). -/
def DeadlockFreeO : Prop :=
  ∀ s : State, ReachableO sk ax ord s → stuckO sk ax ord s = false

/-- Run a list of actions under the assignment, failing on the first
disabled action (cf. `Model.run`) — the executable spine of any
ord-indexed witness or anchor. -/
def runO (s : State) : List Action → Option State
  | [] => some s
  | a :: rest =>
      match applyO sk ax ord a s with
      | some s' => runO s' rest
      | none => none

-- ================================================ reply-first transport

/-- Reply-first reachability is the base model's, right to left. -/
theorem reachable_of_reachableO_rf {s : State}
    (h : ReachableO sk ax .rf s) : Reachable sk ax s := by
  induction h with
  | init => exact .init
  | step a _ hstep ih =>
      exact .step a ih (by rwa [applyO_rf] at hstep)

/-- Reply-first reachability is the base model's, left to right. -/
theorem reachableO_rf_of_reachable {s : State}
    (h : Reachable sk ax s) : ReachableO sk ax .rf s := by
  induction h with
  | init => exact .init
  | step a _ hstep ih =>
      exact .step a ih (by rwa [applyO_rf])

/-- Reply-first enabledness is the base model's. -/
theorem canStepO_rf (s : State) : canStepO sk ax .rf s = canStep sk ax s := by
  unfold canStepO canStep
  congr 1
  funext a
  rw [applyO_rf]

/-- Reply-first stuckness is the base model's. -/
theorem stuckO_rf (s : State) : stuckO sk ax .rf s = stuck sk ax s := by
  unfold stuckO stuck
  rw [canStepO_rf]

/-- The base flagships transport into the quantified statement's
reply-first corner: `DeadlockFree` and `DeadlockFreeO · .rf` are one
claim. -/
theorem deadlockFreeO_rf_iff :
    DeadlockFreeO sk ax .rf ↔ DeadlockFree sk ax := by
  constructor
  · intro h s hr
    rw [← stuckO_rf]
    exact h s (reachableO_rf_of_reachable sk ax hr)
  · intro h s hr
    rw [stuckO_rf]
    exact h s (reachable_of_reachableO_rf sk ax hr)

end StreamingMirror.Ord
