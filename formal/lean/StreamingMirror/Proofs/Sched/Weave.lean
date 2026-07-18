/-
The completeness witness (PROGRESS.md §7 3b): the tree-recursive
WEAVE — a full topological order of the event DAG, built by structural
recursion over the scope tree. Position in the weave is the potential
the completeness argmin consumes: strict across every E1/E2 edge and
along every trace, which is stronger than the weak potential the
argument needs.

# Shape

Two mechanisms carry the whole design (both tool-validated before this
transcription; see `EventDag.weaveOrder`):

- **Query feeds.** A scope's chunk-`i` queries (for kid `i`'s kids)
  are passed down as kid `i`'s `feed` and emitted one per kid-chunk.
  That matches the cap-1 asked channel's E2 exactly — a query fires
  only after the previous scope of the consuming stage has received
  its own — and preserves the ISSUER's trace order, because the
  subtree is woven before the issuer's next chunk begins.
- **Greedy pumps.** The linear traces (absorb, the asm towers, the
  floating `rootret` receive, fins) live in the weave state's `rem`
  and drain by `mergeN` — the SAME priority merge the schedule uses,
  here restricted to the pump traces — after every descent emission.
  Pump emissions only raise counts, so greedy pumping is confluent.

The weave state IS `MState`: manual emissions push an event and bump a
counter (`wEmit`, no enabledness check — on a schedulable skeleton the
emission points are proven open, which is precisely where
`Skel.schedulable` enters the completeness proof), and the pump is a
`mergeN` run, so the whole `MInv` layer (provenance, canon, trace
monotonicity) applies to weave states unchanged.

The recursion itself is a fuel-indexed WORKLIST interpreter
(`weaveGo`), not a well-founded mutual recursion: structural fuel
keeps the definition kernel-reducible (the `decide` anchors below need
iota, which `WellFounded.fix` does not provide) and gives the validity
proofs one induction principle, `mergeN`-style. A `.scope` op expands
to its prologue emissions and per-kid ops; a `.kid` op expands to the
chunk emissions, the kid's feed query, and the kid's `.scope` op —
worklist order IS emission order.

# Relation to the schedule

The weave is NOT the schedule: τ and the blame lemmas stay with the
merge (`Proofs/Sched.lean`). The weave only witnesses that a valid
completion exists — the potential for the stall-refutation argmin.
The eventdag gate pins this definition event-for-event to the tool's
`weaveOrder` on every pin and every acyclic fuzz seed, and validates
it (permutation + every E1/E2/E3 edge) by the same `validateSchedule`
that checks the merge candidate.

Chain (d5, stage A): the witness schedule, executable-validated in
EventDag; consumed by Align.lean and Master.lean. E mirror: WeaveE.lean.
Map: Proofs/Map.lean.
-/
import StreamingMirror.Proofs.Sched

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

/-- Emit one event unconditionally: push it and bump its counter.

The weave's manual emissions go through this — enabledness at each
emission point is a THEOREM (under `Skel.schedulable`), not a check;
the eventdag tool checks it executably at every position. -/
def wEmit (st : MState) (e : Ev) : MState :=
  match e with
  | (c, true, _) =>
      { st with out := st.out ++ [e]
                sent := fun c' => if c' = c then st.sent c + 1 else st.sent c' }
  | (c, false, _) =>
      { st with out := st.out ++ [e]
                rcvd := fun c' => if c' = c then st.rcvd c + 1 else st.rcvd c' }

/-- Drain the pump traces greedily: run the priority merge over the
state's `rem` to its fixpoint (total-remaining-count fuel suffices —
each step emits one event). -/
def wPump (st : MState) : MState :=
  mergeN sk ((st.rem.map List.length).sum) st

/-- Emit, then pump: every manual emission may open pump windows. -/
def wEmitP (st : MState) (e : Ev) : MState :=
  wPump sk (wEmit st e)

/-- One weave instruction: emit an event, weave a scope, or weave one
kid of a scope (`s`/`lastD`/`kidBase` are the scope's data, computed
once at `.scope` expansion). -/
inductive WOp
  | emit (e : Ev)
  | scope (h k : Nat) (feed : List Ev)
  | kid (h k s : Nat) (lastD : Option Nat) (kidBase i : Nat) (feed : List Ev)

/-- Expand a `.scope` op: the two-receive prologue, the parent summary
when nothing disputes (the §5 splice puts it first), then the kids. -/
def wScopeOps (h k : Nat) (feed : List Ev) : List WOp :=
  let pk : Party × Nat := (if h % 2 == 1 then Party.I else Party.R, h)
  let s := sk.stageScope h k
  let n := sk.nChildren h s
  let lastD := ((List.range n).filter (fun i => sk.childIsD h s i)).getLast?
  let kidBase := (List.range k).foldl
    (fun a k' => a + sk.nChildren h (sk.stageScope h k')) 0
  [WOp.emit (wireIn pk, false, k), WOp.emit (askedIn pk, false, k)]
    ++ (if lastD == none then [WOp.emit (upperOut pk, true, k)] else [])
    ++ (List.range n).map fun i => WOp.kid h k s lastD kidBase i feed

/-- Expand a `.kid` op: the wire; for a D kid the resolution, the
parent summary when this kid closes the dispute list, the kid's feed
query, and the kid's `.scope` op with this scope's chunk queries as
its feed; for a W kid (or a leaf slot at `h = 0`) the feed query and —
off the leaf stage — the `.scope` op of an undisputed subtree. -/
def wKidOps (h k s : Nat) (lastD : Option Nat) (kidBase i : Nat)
    (feed : List Ev) : List WOp :=
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
          ++ (if lastD == some i then [WOp.emit (upperOut pk, true, k)] else [])
          -- childIsD is hard-false at the leaf stage, so h ≥ 1 here
          ++ feedOp ++ [WOp.scope (h - 1) (kidBase + i) myQ]
      else
        feedOp ++ if h == 0 then [] else [WOp.scope (h - 1) (kidBase + i) []]

/-- The worklist interpreter: emits pump after every emission, expands
scope/kid ops in place (worklist order is emission order). Structural
on fuel, so the kernel can evaluate it — and each op expands to a
bounded list, so `weaveFuel` below always suffices. -/
def weaveGo : Nat → List WOp → MState → MState
  | 0, _, st => st
  | _ + 1, [], st => st
  | fuel + 1, op :: rest, st =>
      match op with
      | .emit e => weaveGo fuel rest (wEmitP sk st e)
      | .scope h k feed => weaveGo fuel (wScopeOps sk h k feed ++ rest) st
      | .kid h k s lastD kidBase i feed =>
          weaveGo fuel (wKidOps sk h k s lastD kidBase i feed ++ rest) st

/-- Sufficient interpreter fuel: one step per emission plus one per
scope/kid expansion, bounded generously by the event count. -/
def weaveFuel : Nat := 4 * totalEvents sk + 8

/-- The pump traces, in the merge's priority order: absorb, the asm
towers bottom-up, the floating `rootret` receive, fins. -/
def weavePumps : List (List Ev) :=
  [absorbEvents sk]
    ++ sk.asmKeys.map (asmEvents sk)
    ++ [[(Chan.rootret, false, 0)], finEvents sk]

/-- The weave's opening worklist: the openers as plain emits, then
the root scope with ropen's root queries as its feed. -/
def weaveOps : List WOp :=
  ((iopenEvents sk) ++ (ropenEvents sk).take 3).map WOp.emit
    ++ [WOp.scope (sk.rootH - 1) 0 ((ropenEvents sk).drop 3)]

/-- The weave's starting state: nothing emitted, zero counters, the
pump traces racked in `rem`. -/
def weaveInit : MState :=
  ⟨[], fun _ => 0, fun _ => 0, weavePumps sk⟩

/-- The weave's final state: run the worklist to the fuel's end, then
pump once more — the validity lemmas speak about this state. -/
def weaveState : MState :=
  wPump sk (weaveGo sk (weaveFuel sk) (weaveOps sk) (weaveInit sk))

/-- The weave: openers, then the root scope's descent (ropen's root
queries as its feed), then a final pump — a full linearization of the
event set, kept event-for-event equal to `EventDag.weaveOrder` by the
tool's gate. -/
def weave : List Ev := (weaveState sk).out

-- ===================================================== kernel anchors
-- Non-vacuity for the definition above, in the kernel: on the
-- smallest pin the weave emits the whole event set, exactly once.
-- (The full validity claims are gated executably on every pin and
-- acyclic fuzz seed; these anchors keep the Lean definition itself
-- honest against a silently-degenerate recursion.)

set_option maxRecDepth 16000 in
/-- Kernel anchor: the smokeChain weave drains every event. -/
theorem smokeChain_weave_length :
    (weave Pin.smokeChain).length = totalEvents Pin.smokeChain := by decide

set_option maxRecDepth 16000 in
/-- Kernel anchor: the smokeChain weave never repeats an event. -/
theorem smokeChain_weave_nodup : (weave Pin.smokeChain).Nodup := by decide

end StreamingMirror.Sched
