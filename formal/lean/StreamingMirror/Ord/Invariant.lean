/-
The ord-parameterized inductive invariant: `Model.Inv` with the four
order-sensitive receive counts reading each loop's assignment.

Everything else is SHARED with the base invariant, deliberately: the
local fragments (`wkLocalOk`, `asmLocalOk`, `topLocalOk`) never mention
which channel a phase consumes — phases 0/1 are "first receive/second
receive" in both orders — so they are reused verbatim, and the decode
layer's hypothesis (`InvL`) is order-blind. Only the flow equations
change, and only through four counts: a query-first walk consumes its
wire at phase 1→2 (the SECOND-receive formula, `wkAskedRecvd`'s shape)
and its query at 0→1 (the FIRST-receive formula, `wkWireRecvd`'s
shape) — so each O count is a per-key SELECTION between the two
existing formulas, never a new formula. `sentOf` is untouched: sends
do not move in the dequeue-order class.

Chain (ord, stage A): the invariant; consumed by Ord/Wiring,
Ord/Init, Ord/Preserve. Base mirror: Invariant.lean +
Proofs/Lemmas.lean's `InvP`. Map: PROGRESS.md §13.
-/
import StreamingMirror.Proofs.Lemmas
import StreamingMirror.Ord.Basic

namespace StreamingMirror.Ord

open Model

variable (sk : Skel) (ax : AxMode) (ord : OrdMap)

/-- Wire receives performed by walk `pk`, under its assignment: the
first-receive formula when reply-first, the second-receive formula
when query-first (there the wire is consumed at phase 1→2). -/
def wkWireRecvdO (s : State) (pk : Party × Nat) : Nat :=
  match ord.walk pk with
  | .replyFirst => wkWireRecvd sk s pk
  | .queryFirst => wkAskedRecvd sk s pk

/-- Query receives performed by walk `pk`, under its assignment: the
second-receive formula when reply-first, the first-receive formula
when query-first (there the query is consumed at phase 0→1). -/
def wkAskedRecvdO (s : State) (pk : Party × Nat) : Nat :=
  match ord.walk pk with
  | .replyFirst => wkAskedRecvd sk s pk
  | .queryFirst => wkWireRecvd sk s pk

/-- Leaf wires consumed by the absorber, under its assignment. -/
def absorbWireRecvdO (s : State) : Nat :=
  match ord.absorb with
  | .replyFirst => absorbWireRecvd sk s
  | .queryFirst => absorbAskedRecvd sk s

/-- Leaf requests consumed by the absorber, under its assignment. -/
def absorbAskedRecvdO (s : State) : Nat :=
  match ord.absorb with
  | .replyFirst => absorbAskedRecvd sk s
  | .queryFirst => absorbWireRecvd sk s

/-- Cumulative receives from channel `c` under the assignment:
`recvdOf` with the four order-sensitive counts swapped in. The
catch-all arm delegates — assemblers, openers, and finishes have no
pairing loop. -/
def recvdOfO (s : State) : Chan → Nat
  | .wire p h =>
      if h == sk.rootH then
        (if p == Party.I then b2n s.ropenGotWire
         else wkWireRecvdO sk ord s (Party.I, sk.rootH - 1))
      else if p == Party.R && h == 0 then absorbWireRecvdO sk ord s
      else wkWireRecvdO sk ord s (p.other, h - 1)
  | .asked p h => wkAskedRecvdO sk ord s (p, h)
  | .leafRequests => absorbAskedRecvdO sk ord s
  | c => recvdOf sk s c

/-- Flow conservation plus occupancy under the assignment (cf.
`Model.flowOk`; the producer side is order-blind). -/
def flowOkO (s : State) : Bool :=
  (allChans sk).all fun c =>
    (s.chan c + recvdOfO sk ord s c == sentOf sk s c) && (s.chan c ≤ sk.cap c)

/-- The ord-parameterized inductive invariant: the base local fragments
verbatim, the O flow equations. -/
def InvO (s : State) : Bool :=
  (sk.walkKeys.all fun pk => wkLocalOk sk ax s pk) &&
  (sk.asmKeys.all fun pk => asmLocalOk sk s pk) &&
  topLocalOk sk ax s && flowOkO sk ord s

/-- The O invariant at the Prop level (cf. `Model.InvP`): what the O
preservation proofs consume and produce. Its local fields are the BASE
model's — `InvPO.local` hands the decode layer its order-blind
hypothesis unchanged. -/
structure InvPO (sk : Skel) (ax : AxMode) (ord : OrdMap) (s : State) : Prop where
  wk : ∀ pk ∈ sk.walkKeys, wkLocalOk sk ax s pk = true
  asm : ∀ pk ∈ sk.asmKeys, asmLocalOk sk s pk = true
  top : topLocalOk sk ax s = true
  flow : ∀ c ∈ allChans sk,
    s.chan c + recvdOfO sk ord s c = sentOf sk s c ∧ s.chan c ≤ sk.cap c

/-- The O invariant projects onto the base model's local fragment —
the decode layer's hypothesis is order-blind by construction. -/
theorem InvPO.local {sk : Skel} {ax : AxMode} {ord : OrdMap} {s : State}
    (hi : InvPO sk ax ord s) : InvL sk ax s :=
  ⟨hi.wk, hi.asm, hi.top⟩

/-- The O invariant with the capacity half of `flow` dropped (cf.
`Model.InvPW`): the progress engine's exact hypothesis shape. -/
structure InvPWO (sk : Skel) (ax : AxMode) (ord : OrdMap) (s : State) : Prop where
  wk : ∀ pk ∈ sk.walkKeys, wkLocalOk sk ax s pk = true
  asm : ∀ pk ∈ sk.asmKeys, asmLocalOk sk s pk = true
  top : topLocalOk sk ax s = true
  flow : ∀ c ∈ allChans sk, s.chan c + recvdOfO sk ord s c = sentOf sk s c

/-- The weak O invariant projects onto the local fragment. -/
theorem InvPWO.local {sk : Skel} {ax : AxMode} {ord : OrdMap} {s : State}
    (hi : InvPWO sk ax ord s) : InvL sk ax s :=
  ⟨hi.wk, hi.asm, hi.top⟩

/-- The full O invariant weakens: drop the capacity half of `flow`. -/
theorem InvPO.weak {sk : Skel} {ax : AxMode} {ord : OrdMap} {s : State}
    (hi : InvPO sk ax ord s) : InvPWO sk ax ord s :=
  ⟨hi.wk, hi.asm, hi.top, fun c hc => (hi.flow c hc).1⟩

theorem invO_iff (s : State) :
    InvO sk ax ord s = true ↔ InvPO sk ax ord s := by
  constructor
  · intro h
    simp only [InvO, Bool.and_eq_true, List.all_eq_true] at h
    obtain ⟨⟨⟨hwk, hasm⟩, htop⟩, hflow⟩ := h
    refine ⟨hwk, hasm, htop, fun c hc => ?_⟩
    rw [flowOkO, List.all_eq_true] at hflow
    have := hflow c hc
    simpa using this
  · intro ⟨hwk, hasm, htop, hflow⟩
    simp only [InvO, Bool.and_eq_true, List.all_eq_true]
    refine ⟨⟨⟨hwk, hasm⟩, htop⟩, ?_⟩
    rw [flowOkO, List.all_eq_true]
    intro c hc
    have := hflow c hc
    simpa using this

-- ============================================ the reply-first identity

/-- At the all-reply-first assignment every O count is the base count:
the O invariant IS the base invariant there (the transport that makes
the baseline flagships the `ord = .rf` instances). -/
theorem recvdOfO_rf (s : State) (c : Chan) :
    recvdOfO sk .rf s c = recvdOf sk s c := by
  cases c <;>
    simp [recvdOfO, recvdOf, wkWireRecvdO, wkAskedRecvdO,
      absorbWireRecvdO, absorbAskedRecvdO, OrdMap.rf]

theorem invPO_rf_iff (s : State) : InvPO sk ax .rf s ↔ InvP sk ax s := by
  constructor
  · intro hi
    exact ⟨hi.wk, hi.asm, hi.top, fun c hc => by
      have := hi.flow c hc
      rwa [recvdOfO_rf] at this⟩
  · intro hi
    exact ⟨hi.wk, hi.asm, hi.top, fun c hc => by
      have := hi.flow c hc
      rwa [← recvdOfO_rf sk] at this⟩

end StreamingMirror.Ord
