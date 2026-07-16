/-
Dispute skeletons and their derived structure (MODEL.md §2), transcribed
from `formal/quint/streamingMirror.qnt` (the split-variable Phase B
revision). Every definition here is a `def` computing over lists, so the
whole layer is executable; theorems assume `wf : sk.wellFormed = true`.

Correspondence: Quint's stringly `(kindTag, party, height)` tuples become
the `Party`/`Kind` inductives; `NSC` disappears (Lean folds lengths);
everything else is name-for-name.
-/

namespace StreamingMirror

/-- The two endpoints. Quint: `"I"` / `"R"`. -/
inductive Party | I | R
  deriving DecidableEq, Repr

/-- The other endpoint. Quint: `other`. -/
def Party.other : Party → Party
  | .I => .R
  | .R => .I

/-- A scope's dispute kind: two-sided dispute (recursive) or one-sided
request (degenerate, childless). Matches (M scopes are erased). -/
inductive Kind | D | R
  deriving DecidableEq, Repr

/-- One scope of a dispute skeleton, in the flattened BFS encoding. -/
structure Scope where
  kind : Kind
  height : Nat
  kids : List Nat
  leafReqs : Nat
  deriving DecidableEq, Repr

/-- A dispute skeleton plus the model's numeric parameters.

Index 0 is the root; ids are BFS order (parent < child, siblings
ascending); `R` scopes are childless; height-1 `D` scopes carry
`leafReqs`, all others carry 0. `wellFormed` checks all of it. -/
structure Skel where
  scopes : List Scope
  rootH : Nat      -- even; Rust: 32
  fan : Nat        -- F; Rust: FAN = 256
  capLevel : Nat   -- AssemblyLevelReturns capacity; Rust: FAN
  deriving Repr

/-- Does party `p` ask (pair reply with query) for scopes at height `j`?
Initiator asks even heights, responder odd (MODEL.md §3). Quint: `asks`.
Skeleton-independent, hence outside `Skel`. -/
def asks (p : Party) (j : Nat) : Bool :=
  match p with
  | .I => j % 2 == 0
  | .R => j % 2 == 1

namespace Skel

variable (sk : Skel)

/-- Total scope access: out-of-range ids read as a degenerate scope.
Guards keep real accesses in range; the default only keeps `def`s total
(the Quint spec's `scAt` device). -/
def scope (i : Nat) : Scope :=
  sk.scopes.getD i ⟨Kind.R, 0, [], 0⟩

/-- Scope ids at height `h`, in processing order (= id order; BFS).
Quint: `scopesAt`. -/
def scopesAt (h : Nat) : List Nat :=
  (List.range sk.scopes.length).filter (fun i => (sk.scope i).height == h)

/-- Skeleton well-formedness (MODEL.md §2). Quint: `wellFormed`, minus
`NSC` (unneeded) plus the same `capLevel ≥ 1`. -/
def wellFormed : Bool :=
  let n := sk.scopes.length
  let perScope := (List.range n).all fun i =>
    let sc := sk.scope i
    (decide (sc.height ≥ 1)) &&
    (sc.kids.length ≤ sk.fan) &&
    (sc.leafReqs ≤ sk.fan) &&
    (sc.kind == Kind.D || (sc.kids.isEmpty && sc.leafReqs == 0)) &&
    (sc.height != 1 || sc.kids.isEmpty) &&
    (sc.leafReqs == 0 || (sc.height == 1 && sc.kind == Kind.D)) &&
    -- kids: ascending, above the parent id, in range, one height down
    (sc.kids.foldl (fun (acc : Nat × Bool) k =>
        (k, acc.2 && decide (k > acc.1) && decide (k < n) &&
            ((sk.scope k).height == sc.height - 1)))
      (i, true)).2
  let kidCount := sk.scopes.foldl (fun acc sc => acc + sc.kids.length) 0
  let kidList := sk.scopes.foldl (fun acc sc => acc ++ sc.kids) []
  decide (n > 0) &&
  ((sk.scope 0).height == sk.rootH) && ((sk.scope 0).kind == Kind.D) &&
  (sk.rootH % 2 == 0) &&
  perScope &&
  (kidCount == n - 1) && (kidList.eraseDups.length == n - 1) &&
  (!kidList.contains 0) &&
  decide (sk.capLevel ≥ 1)

/-- Walk stage keys: (party, consumed message index). Initiator stages
consume odd indices `rootH-1, rootH-3, …, 1`; responder even
`rootH-2, …, 0`. Quint: `walkKeys` (as a list — order is fixed for
enumeration; treat as a set in proofs). -/
def walkKeys : List (Party × Nat) :=
  ((List.range (sk.rootH / 2)).map fun k => (Party.I, sk.rootH - 1 - 2 * k)) ++
  ((List.range (sk.rootH / 2)).map fun k => (Party.R, sk.rootH - 2 - 2 * k))

/-- Assembler keys: (party, assembled scope height). Quint: `asmKeys`. -/
def asmKeys : List (Party × Nat) :=
  ((List.range sk.rootH).map fun j => (Party.I, j + 1)) ++
  ((List.range (sk.rootH - 1)).map fun j => (Party.R, j + 1))

/-- The scopes a stage at consume-height `h` processes: those at `h + 1`.
Quint: `stageScopes`. -/
def stageScopes (h : Nat) : List Nat := sk.scopesAt (h + 1)

/-- Stage length. Quint: `stageLen`. -/
def stageLen (h : Nat) : Nat := (sk.stageScopes h).length

/-- The id of the k-th scope of stage `h`, total (root as dummy past the
end — the Quint `scAt` device; invariants keep real reads in range). -/
def stageScope (h k : Nat) : Nat := (sk.stageScopes h).getD k 0

/-- Children of scope `s` as seen at stage `h`: at the leaf stage
(`h = 0`) the "children" are the scope's leaf requests (wire-send-only,
like R children). Quint: `nChildren`. -/
def nChildren (h s : Nat) : Nat :=
  if h == 0 then (sk.scope s).leafReqs else (sk.scope s).kids.length

/-- Is child `i` of scope `s` at stage `h` a two-sided dispute? Hard
false at the leaf stage and out of range. Quint: `childIsD`. -/
def childIsD (h s i : Nat) : Bool :=
  if h == 0 then false
  else match (sk.scope s).kids[i]? with
    | some k => (sk.scope k).kind == Kind.D
    | none => false

/-- Queries launched for D child `i` of scope `s` at stage `h`: one per
kid of the child scope, or its `leafReqs` at height 1. Quint: `qCount`. -/
def qCount (h s i : Nat) : Nat :=
  if !sk.childIsD h s i then 0
  else match (sk.scope s).kids[i]? with
    | some c =>
        let child := sk.scope c
        if child.height == 1 then child.leafReqs else child.kids.length
    | none => 0

/-- D children of scope `s` (the answerer recursion count). Quint:
`dCount`. -/
def dCount (s : Nat) : Nat :=
  ((sk.scope s).kids.filter (fun k => (sk.scope k).kind == Kind.D)).length

/-- D children of `s` as counted at stage `h` (leaf stage has none).
Quint: `dOf`. -/
def dOf (h s : Nat) : Nat := if h == 0 then 0 else sk.dCount s

/-- Queries scope `s` owes at stage `h`, summed over all children.
Quint: `qOf`. -/
def qOf (h s : Nat) : Nat :=
  (List.range (sk.nChildren h s)).foldl (fun acc i => acc + sk.qCount h s i) 0

/-- Total leaf requests below height-1 D scopes. Quint: `totalLeafReqs`. -/
def totalLeafReqs : Nat :=
  ((sk.scopesAt 1).filter (fun s => (sk.scope s).kind == Kind.D)).foldl
    (fun acc s => acc + (sk.scope s).leafReqs) 0

/-- Pending counts of the resolutions `Asm (p, j)` consumes, in arrival
order. Asker side: one per scope at `j`, pending = #D kids. Answerer
side: one per D scope at `j`, pending = #kids (or `leafReqs` at height
1). Quint: `asmResList`. -/
def asmResList (p : Party) (j : Nat) : List Nat :=
  if asks p j then
    (sk.scopesAt j).map fun s => sk.dCount s
  else
    ((sk.scopesAt j).filter (fun s => (sk.scope s).kind == Kind.D)).map fun s =>
      let sc := sk.scope s
      if sc.height == 1 then sc.leafReqs else sc.kids.length

/-- The responder-side root resolution's pending count. Quint:
`rootPending`. -/
def rootPending : Nat := (sk.scope 0).kids.length

-- Prefix sums over a stage's first `k` scopes: cumulative sends of a
-- walk that has completed `k` scopes. Quint: `wiresBefore` etc.

/-- Σ nChildren over the first `k` scopes of stage `h`. -/
def wiresBefore (h k : Nat) : Nat :=
  ((sk.stageScopes h).take k).foldl (fun acc s => acc + sk.nChildren h s) 0

/-- Σ dOf over the first `k` scopes of stage `h`. -/
def dsBefore (h k : Nat) : Nat :=
  ((sk.stageScopes h).take k).foldl (fun acc s => acc + sk.dOf h s) 0

/-- Σ qOf over the first `k` scopes of stage `h`. -/
def qsBefore (h k : Nat) : Nat :=
  ((sk.stageScopes h).take k).foldl (fun acc s => acc + sk.qOf h s) 0

/-- Σ of the first `k` pending counts of an assembler's resolution list.
Quint: `pendsBefore`. -/
def pendsBefore (p : Party) (j k : Nat) : Nat :=
  ((sk.asmResList p j).take k).foldl (· + ·) 0

/-- Pending count of resolution `i` for `Asm (p, j)`; 0 past the end.
Quint: `pendAt`. -/
def pendAt (p : Party) (j i : Nat) : Nat := (sk.asmResList p j).getD i 0

end Skel

/-- The axiom mode: which `Trace::assert_valid` ledgers guard the
committed-choice publisher. Quint: the six `AX_*`/`WIRE_FIRST` consts. -/
structure AxMode where
  w : Bool        -- wire before internal publication (wire ledger)
  d1root : Bool   -- root resolution before root child queries
  d1int : Bool    -- resolution before dependent queries, internal
  d2 : Bool       -- parent resolution after all D-child resolutions
  d3 : Bool       -- sibling contiguity (the ledger gap this work found)
  wireFirst : Bool -- control scaffolding, not an axiom (see Quint doc)
  deriving DecidableEq, Repr

/-- All axioms on, scaffolding off: the assumed interface of the Rust
implementation. -/
def AxMode.full : AxMode := ⟨true, true, true, true, true, false⟩

end StreamingMirror
