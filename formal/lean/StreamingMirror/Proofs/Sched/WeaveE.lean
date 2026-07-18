/-
The `.impl` completeness witness (PROGRESS.md §9): the EWEAVE — the
encoder-order analog of `Weave.lean`'s d5 weave. One delta, per scope:
the parent summary emits at the scope TAIL, after every kid op — and
therefore after the whole subtree, whose descent carries the scope's
last-chunk queries — instead of spliced after the final resolution.
The per-walk projection is exactly `walkEventsE`'s epilogue order
(`scopeSendsE`: every child chunk, parent last), the d6 ledger's
placement.

Where the d5 weave is valid at every capacity (parent-early is the
token-release discipline), the eweave is valid only under the margin-0
capacity hypothesis (`∀ s, dCount s ≤ capLevel`): at the scope tail
the upper window needs the level channel to have drained everything
below, and margin 0 is what bounds the in-flight uppers of the parent
scope's dispute group (design/parent-placement.md §6). Sub-margin the
eweave emits through a closed guard — `Control.pdelay` pins this
executably (`weaveOrderE` reports the disabled emission), the
capacity hypothesis shown load-bearing at the witness itself.

The interpreter (`weaveGo`), emission/pump primitives (`wEmitP`), op
type (`WOp`), fuel, pumps, and starting state are `Weave.lean`'s,
reused verbatim — only the two expansion functions differ, so
`weaveGoE` is the same worklist recursion dispatching to the E
expanders. `WOp.kid`'s `lastD` field is dead weight here (no splice
decision exists); the E expanders pass and ignore `none`.

Validated the same way the d5 weave is: the eventdag gate pins this
definition event-for-event to the tool's `weaveOrderE` and runs the
full `validateSchedule` acceptance (permutation + every E1/E2/E3
edge — the DAG's edge set is placement-agnostic, so the same oracle
serves both corners) on every pin and every acyclic fuzz seed, at
margin 0. Kernel anchors below keep the recursion honest against
silent degeneracy.
-/
import StreamingMirror.Proofs.Sched.Weave

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

/-- Expand a `.scope` op, encoder order: the two-receive prologue, the
kids, and the parent summary LAST — after every kid, hence after the
whole subtree (whose descent emits this scope's last-chunk queries).
No early-parent case: undisputed scopes also close with the parent. -/
def wScopeOpsE (h k : Nat) (feed : List Ev) : List WOp :=
  let pk : Party × Nat := (if h % 2 == 1 then Party.I else Party.R, h)
  let s := sk.stageScope h k
  let n := sk.nChildren h s
  let kidBase := (List.range k).foldl
    (fun a k' => a + sk.nChildren h (sk.stageScope h k')) 0
  [WOp.emit (wireIn pk, false, k), WOp.emit (askedIn pk, false, k)]
    ++ (List.range n).map (fun i => WOp.kid h k s none kidBase i feed)
    ++ [WOp.emit (upperOut pk, true, k)]

/-- Expand a `.kid` op, encoder order: `wKidOps` minus the splice —
the wire; for a D kid the resolution, the kid's feed query, and the
kid's `.scope` op with this scope's chunk queries as its feed; for a
W kid the feed query and (off the leaf stage) the undisputed subtree.
The parent emission lives in `wScopeOpsE`'s tail, never here. -/
def wKidOpsE (h k s : Nat) (kidBase i : Nat) (feed : List Ev) : List WOp :=
  let pk : Party × Nat := (if h % 2 == 1 then Party.I else Party.R, h)
  let feedOp := match feed[i]? with
    | some q => [WOp.emit q]
    | none => []
  [WOp.emit (wireOut pk, true, sk.wiresBefore h k + i)]
    ++ if sk.childIsD h s i then
        let dRank := ((List.range i).filter (fun i' => sk.childIsD h s i')).length
        let qBase := sk.qsBefore h k
          + ((List.range i).map (fun i' => sk.qCount h s i')).sum
        let myQ := (List.range (sk.qCount h s i)).map fun t =>
          ((askedOut pk, true, qBase + t) : Ev)
        [WOp.emit (lowerOut pk, true, sk.dsBefore h k + dRank)]
          -- childIsD is hard-false at the leaf stage, so h ≥ 1 here
          ++ feedOp ++ [WOp.scope (h - 1) (kidBase + i) myQ]
      else
        feedOp ++ if h == 0 then [] else [WOp.scope (h - 1) (kidBase + i) []]

/-- The encoder-order worklist interpreter: `weaveGo`'s recursion,
dispatching to the E expanders. -/
def weaveGoE : Nat → List WOp → MState → MState
  | 0, _, st => st
  | _ + 1, [], st => st
  | fuel + 1, op :: rest, st =>
      match op with
      | .emit e => weaveGoE fuel rest (wEmitP sk st e)
      | .scope h k feed => weaveGoE fuel (wScopeOpsE sk h k feed ++ rest) st
      | .kid h k s _lastD kidBase i feed =>
          weaveGoE fuel (wKidOpsE sk h k s kidBase i feed ++ rest) st

/-- The eweave's final state: `weaveState`'s shape over the E
interpreter — same opening worklist, same pumps, same fuel, one last
pump. -/
def weaveStateE : MState :=
  wPump sk (weaveGoE sk (weaveFuel sk) (weaveOps sk) (weaveInit sk))

/-- The eweave: the `.impl` witness linearization, kept event-for-event
equal to `EventDag.weaveOrderE` by the tool's gate. -/
def weaveE : List Ev := (weaveStateE sk).out

-- ===================================================== kernel anchors

set_option maxRecDepth 16000 in
/-- Kernel anchor: the smokeChain eweave drains every event. -/
theorem smokeChain_weaveE_length :
    (weaveE Pin.smokeChain).length = totalEvents Pin.smokeChain := by decide

set_option maxRecDepth 16000 in
/-- Kernel anchor: the smokeChain eweave never repeats an event. -/
theorem smokeChain_weaveE_nodup : (weaveE Pin.smokeChain).Nodup := by decide

end StreamingMirror.Sched
