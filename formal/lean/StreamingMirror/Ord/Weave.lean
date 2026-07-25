/-
The O weave: the completeness witness at an arbitrary assignment of
the two-point dequeue-order class — the eweave (`WeaveE.lean`) with
each scope's two-receive prologue in ITS walk's assigned order and the
absorber pump row in ITS assigned order.

Only the prologue moves. Each scope still expands as prologue, kids in
slot order, parent summary LAST (`wScopeOpsE`'s tail placement — the
`.impl` encoder order this campaign quantifies over), so the O weave's
per-walk projection is `walkEventsO`'s epilogue order and the send
suffixes are the E weave's verbatim. Kid ops carry no prologue —
`wKidOpsE` is reused as-is; the kid's own prologue dispatches when the
interpreter's `.scope` arm expands the kid's subtree through
`wScopeOpsO`. The pump side swaps `weaveInit`'s racked rows for
`weavePumpsO`: the absorber row in `ord.absorb`'s order, everything
else shared.

The interpreter (`weaveGoO`) is `weaveGo`'s recursion dispatching to
the O scope expander; emission/pump primitives (`wEmitP`), op type
(`WOp`), fuel, and opening worklist are `Weave.lean`'s, reused
verbatim (`wPump` is family-agnostic — it pumps whatever `rem`
holds). The reply-first assignment collapses the expanders to the E
expanders definitionally (`wScopeOpsO_rf`, `rfl`).

The eventdag oracle validates this construction executably
(`EventDag.weaveOrderEO`, absorber pump in its own assignment order)
on every pin, every acyclic fuzz seed, and the boundary matrix, at
margin 0 — this file transcribes a validated design. Kernel anchors
below keep the recursion honest against silent degeneracy at the
all-query-first corner.

Chain (ord, stage D): the witness and its alignment; consumed by the
O master induction. Base mirror: WeaveE.lean. Map: PROGRESS.md §13.
-/
import StreamingMirror.Proofs.Sched.WeaveE
import StreamingMirror.Ord.Pumps

namespace StreamingMirror.Ord

open Model
open Sched

variable (sk : Skel) (ord : OrdMap)

/-- The ordered prologue always holds exactly its two receives. -/
theorem prologueO_length (pk : Party × Nat) (k : Nat) :
    (prologueO ord pk k).length = 2 := by
  unfold prologueO
  cases ord.walk pk <;> rfl

/-- Expand a `.scope` op at the assignment: the two-receive prologue
in the scope's walk's assigned order, the kids, and the parent summary
LAST — `wScopeOpsE` with the prologue dispatched through `prologueO`.
No early-parent case: the whole class closes scopes with the parent. -/
def wScopeOpsO (h k : Nat) (feed : List Ev) : List WOp :=
  let pk : Party × Nat := (if h % 2 == 1 then Party.I else Party.R, h)
  let s := sk.stageScope h k
  let n := sk.nChildren h s
  let kidBase := (List.range k).foldl
    (fun a k' => a + sk.nChildren h (sk.stageScope h k')) 0
  (prologueO ord pk k).map WOp.emit
    ++ (List.range n).map (fun i => WOp.kid h k s none kidBase i feed)
    ++ [WOp.emit (upperOut pk, true, k)]

/-- The reply-first scope expander IS the E scope expander: the
prologue dispatch collapses definitionally. -/
theorem wScopeOpsO_rf (h k : Nat) (feed : List Ev) :
    wScopeOpsO sk .rf h k feed = wScopeOpsE sk h k feed := rfl

/-- The O worklist interpreter: `weaveGo`'s recursion, dispatching
`.scope` ops to the O expander. Kid ops are `wKidOpsE`'s — kids carry
no prologue of their own; a kid's subtree prologue dispatches when its
`.scope` op reaches this arm. -/
def weaveGoO : Nat → List WOp → MState → MState
  | 0, _, st => st
  | _ + 1, [], st => st
  | fuel + 1, op :: rest, st =>
      match op with
      | .emit e => weaveGoO fuel rest (wEmitP sk st e)
      | .scope h k feed =>
          weaveGoO fuel (wScopeOpsO sk ord h k feed ++ rest) st
      | .kid h k s _lastD kidBase i feed =>
          weaveGoO fuel (wKidOpsE sk h k s kidBase i feed ++ rest) st

/-- The events an O worklist will emit by hand, in order: `goEvents`'
twin over the O expanders — same fuel, same expansion, no state — so
the counting induction can walk `weaveGoO` and its futures in
lockstep. -/
def goEventsO : Nat → List WOp → List Ev
  | 0, _ => []
  | _ + 1, [] => []
  | fuel + 1, op :: rest =>
      match op with
      | .emit e => e :: goEventsO fuel rest
      | .scope h k feed => goEventsO fuel (wScopeOpsO sk ord h k feed ++ rest)
      | .kid h k s _lastD kidBase i feed =>
          goEventsO fuel (wKidOpsE sk h k s kidBase i feed ++ rest)

/-- The O weave's starting state: nothing emitted, zero counters, the
O pump traces — absorber row in its assigned order — racked in
`rem`. -/
def weaveInitO : MState :=
  ⟨[], fun _ => 0, fun _ => 0, weavePumpsO sk ord⟩

/-- The O weave's final state: `weaveState`'s shape over the O
interpreter — same opening worklist, same fuel, the O pumps, one last
pump. -/
def weaveStateO : MState :=
  wPump sk (weaveGoO sk ord (weaveFuel sk) (weaveOps sk) (weaveInitO sk ord))

/-- The O weave: the class's witness linearization at this assignment,
kept event-for-event equal to `EventDag.weaveOrderEO` by the tool's
gate. -/
def weaveO : List Ev := (weaveStateO sk ord).out

-- ===================================================== kernel anchors
-- Non-vacuity at the all-query-first corner (the shipping Rust's
-- assignment — the corner the reply-first collapse does NOT cover):
-- on the smallest pin the O weave emits the whole event set, exactly
-- once. The full validity claims are gated executably per assignment;
-- these anchors keep the Lean recursion itself honest.

set_option maxRecDepth 16000 in
/-- Kernel anchor: the smokeChain all-QF O weave drains every event. -/
theorem smokeChain_weaveO_length :
    (weaveO Pin.smokeChain .qf).length = totalEvents Pin.smokeChain := by
  decide

set_option maxRecDepth 16000 in
/-- Kernel anchor: the smokeChain all-QF O weave never repeats an
event. -/
theorem smokeChain_weaveO_nodup :
    (weaveO Pin.smokeChain .qf).Nodup := by decide

end StreamingMirror.Ord
