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
  recursion returns before the issuer's next chunk begins.
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

# Relation to the schedule

The weave is NOT the schedule: τ and the blame lemmas stay with the
merge (`Proofs/Sched.lean`). The weave only witnesses that a valid
completion exists — the potential for the stall-refutation argmin.
The eventdag gate pins this definition event-for-event to the tool's
`weaveOrder` on every pin and every acyclic fuzz seed, and validates
it (permutation + every E1/E2/E3 edge) by the same `validateSchedule`
that checks the merge candidate.
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

/-- Emit a feed entry if present (`none` past the end is inert; feeds
are sized one query per kid by the BFS alignment). -/
def wFeed (st : MState) (feed : List Ev) (i : Nat) : MState :=
  match feed[i]? with
  | some q => wEmitP sk st q
  | none => st

mutual

/-- The descent weave for scope `k` of stage `h`: the two-receive
prologue, the parent summary when nothing disputes (the §5 splice puts
it first), then the kids.

`feed[i]` is the query event for kid `i`, owned by this scope's
PARENT's trace and emitted here one per kid, in order. -/
def weaveScope (h k : Nat) (feed : List Ev) (st : MState) : MState :=
  let pk : Party × Nat := (if h % 2 == 1 then Party.I else Party.R, h)
  let st := wEmitP sk st (wireIn pk, false, k)
  let st := wEmitP sk st (askedIn pk, false, k)
  let s := sk.stageScope h k
  let n := sk.nChildren h s
  let lastD := ((List.range n).filter (fun i => sk.childIsD h s i)).getLast?
  let st := if lastD == none then wEmitP sk st (upperOut pk, true, k) else st
  let kidBase := (List.range k).foldl
    (fun a k' => a + sk.nChildren h (sk.stageScope h k')) 0
  weaveKids h k (List.range n) feed st (s := s) (lastD := lastD)
    (kidBase := kidBase)
termination_by (h, 1, 0)

/-- One kid at a time: the wire; for a D kid the resolution, the
parent summary when this kid closes the dispute list, the kid's feed
query, and the recursive descent with this scope's chunk queries as
the kid's feed; for a W kid (or a leaf slot at `h = 0`) the feed query
and — off the leaf stage — the descent of an undisputed subtree. -/
def weaveKids (h k : Nat) (kids : List Nat) (feed : List Ev) (st : MState)
    (s : Nat) (lastD : Option Nat) (kidBase : Nat) : MState :=
  match kids with
  | [] => st
  | i :: rest =>
      let pk : Party × Nat := (if h % 2 == 1 then Party.I else Party.R, h)
      let st := wEmitP sk st (wireOut pk, true, sk.wiresBefore h k + i)
      let st :=
        if sk.childIsD h s i then
          let dRank := ((List.range i).filter (fun i' => sk.childIsD h s i')).length
          let st := wEmitP sk st (lowerOut pk, true, sk.dsBefore h k + dRank)
          let st := if lastD == some i then
            wEmitP sk st (upperOut pk, true, k) else st
          if h = 0 then st  -- childIsD is hard-false at the leaf stage
          else
            let qBase := sk.qsBefore h k
              + ((List.range i).map (fun i' => sk.qCount h s i')).sum
            let myQ := (List.range (sk.qCount h s i)).map fun t =>
              ((askedOut pk, true, qBase + t) : Ev)
            let st := wFeed sk st feed i
            weaveScope (h - 1) (kidBase + i) myQ st
        else
          let st := wFeed sk st feed i
          if h = 0 then st
          else weaveScope (h - 1) (kidBase + i) [] st
      weaveKids h k rest feed st (s := s) (lastD := lastD) (kidBase := kidBase)
termination_by (h, 0, kids.length)

end

/-- The pump traces, in the merge's priority order: absorb, the asm
towers bottom-up, the floating `rootret` receive, fins. -/
def weavePumps : List (List Ev) :=
  [absorbEvents sk]
    ++ sk.asmKeys.map (asmEvents sk)
    ++ [[(Chan.rootret, false, 0)], finEvents sk]

/-- The weave: openers, then the root scope's descent (ropen's root
queries as its feed), then a final pump — a full linearization of the
event set, kept event-for-event equal to `EventDag.weaveOrder` by the
tool's gate. -/
def weave : List Ev :=
  let st : MState := ⟨[], fun _ => 0, fun _ => 0, weavePumps sk⟩
  let st := (iopenEvents sk).foldl (wEmitP sk) st
  let st := ((ropenEvents sk).take 3).foldl (wEmitP sk) st
  let st := weaveScope sk (sk.rootH - 1) 0 ((ropenEvents sk).drop 3) st
  (wPump sk st).out

end StreamingMirror.Sched
