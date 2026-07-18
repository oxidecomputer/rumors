/-
The abstract model of the streaming mirror protocol (MODEL.md §§3–5),
transcribed action-for-action from `formal/quint/streamingMirror.qnt`
(the split-variable Phase B revision).

Executable-first: every guard is a `Bool`, every transition is
`apply : Action → State → Option State` (`none` = guard failed), and the
step relation is the image of `apply`. `#eval`/`decide` re-pin this model
to the Quint validation matrix, and the negative-control witnesses are
`decide`-checked replays of concrete action lists.

Correspondences with the Quint spec (full table in formal/README.md):
`chKind`/`chChild` become `Option Oblig`; the stringly channel tuples
become the `Chan` inductive (`("asked","I",-1)` becomes
`Chan.leafRequests`); the committed-choice semantics — commit to an
axiom-consistent obligation, then fire it before choosing again — is
carried by the `walkCommit`/`walkFire` action split exactly as in Quint,
and remains load-bearing (may-fire semantics would let a scheduler dodge
every jam).
-/
import StreamingMirror.Skel

namespace StreamingMirror

/-- One channel of the pipeline (MODEL.md §4). Quint: the
`(kindTag, party, height)` tuples of `allChans`; `leafRequests` is
Quint's `("asked", "I", -1)`. -/
inductive Chan
  | wire (p : Party) (h : Nat)
  | asked (p : Party) (h : Nat)
  | leafRequests
  | upper (p : Party) (h : Nat)
  | lower (p : Party) (h : Nat)
  | level (p : Party) (j : Nat)
  | rootret | rootrets | rootres
  deriving DecidableEq, Repr

/-- Channel capacity: `capLevel` for the inter-level return boundaries,
one everywhere else (queues.rs; MODEL.md §4). -/
def Skel.cap (sk : Skel) : Chan → Nat
  | .level _ _ => sk.capLevel
  | _ => 1

-- Per-walk channel wiring. Quint: `wireIn`/`askedIn`/`wireOut`/
-- `lowerOut`/`upperOut`/`askedOut`.

def wireIn (pk : Party × Nat) : Chan := .wire pk.1.other (pk.2 + 1)
def askedIn (pk : Party × Nat) : Chan := .asked pk.1 pk.2
def wireOut (pk : Party × Nat) : Chan := .wire pk.1 pk.2
def lowerOut (pk : Party × Nat) : Chan := .lower pk.1 pk.2
def upperOut (pk : Party × Nat) : Chan := .upper pk.1 pk.2

/-- Where a walk's child queries go: two stages down, or the leaf-request
channel from the lowest initiator stage. (`askedOut (R, 0)` cannot fire —
`childIsD` is hard-false at the leaf stage — so its value is moot; see
the Quint spec's miswire note.) -/
def askedOut (pk : Party × Nat) : Chan :=
  if pk.2 < 2 then .leafRequests else .asked pk.1 (pk.2 - 2)

-- Per-assembler wiring. Quint: `asmResChan`/`asmLevelChan`/`asmOutChan`.

def asmResChan (pk : Party × Nat) : Chan :=
  if asks pk.1 pk.2 then .upper pk.1 (pk.2 - 1) else .lower pk.1 pk.2

def asmLevelChan (pk : Party × Nat) : Chan := .level pk.1 (pk.2 - 1)

def Skel.asmOutChan (sk : Skel) (pk : Party × Nat) : Chan :=
  if pk.1 == Party.I && pk.2 == sk.rootH then .rootret
  else if pk.1 == Party.R && pk.2 == sk.rootH - 1 then .rootrets
  else .level pk.1 pk.2

/-- A walk's publication obligation for the current scope. Quint:
`chKind`/`chChild` (`Option Oblig` replaces the `"none"`/`-1` encoding). -/
inductive Oblig
  | wire (i : Nat)
  | res (i : Nat)
  | query (i : Nat)
  | parent
  deriving DecidableEq, Repr

/-- One walk stage's state. Phases: 0 recvWire, 1 recvAsked,
2 publishing, 3 closeWire, 4 closeAsked, 5 done. Quint: the eight
`wk*` map variables, regathered. -/
structure WalkSt where
  scope : Nat
  phase : Nat
  wireDone : Nat → Bool
  resDone : Nat → Bool
  qSent : Nat → Nat
  parentDone : Bool
  committed : Option Oblig

/-- One assembler's state. Phases: 0 recvRes, 1 recvLevels, 2 send,
3 closeRes, 4 done. Quint: the three `asm*` map variables. -/
structure AsmSt where
  idx : Nat
  phase : Nat
  got : Nat
  deriving DecidableEq, Repr

/-- The opening obligations. Quint: `iopenCh` / `ropenCh` strings. -/
inductive IOblig | wire | query
  deriving DecidableEq, Repr
inductive ROblig | wire | res | query
  deriving DecidableEq, Repr

/-- The whole system state (MODEL.md §5): every process a finite program
over channel operations, channels as occupancy counters. -/
structure State where
  walk : Party × Nat → WalkSt
  asm : Party × Nat → AsmSt
  chan : Chan → Nat
  iopenWire : Bool
  iopenQuery : Bool
  iopenCh : Option IOblig
  ropenGotWire : Bool
  ropenWire : Bool
  ropenRes : Bool
  ropenQ : Nat
  ropenCh : Option ROblig
  absorbIdx : Nat
  absorbPhase : Nat
  ifin : Bool
  rfinGotRes : Bool
  rfinGot : Nat

namespace Model

variable (sk : Skel) (ax : AxMode)

/-- A fresh walk state at scope index `k` of stage `h`. Quint:
`freshWalk`. -/
def freshWalk (h k : Nat) : WalkSt :=
  { scope := k
    phase := if k < sk.stageLen h then 0 else 3
    wireDone := fun _ => false
    resDone := fun _ => false
    qSent := fun _ => 0
    parentDone := false
    committed := none }

/-- Every obligation of the walk's current scope has fired. Quint:
`scopeComplete`. -/
def scopeComplete (h : Nat) (ws : WalkSt) : Bool :=
  if ws.scope ≥ sk.stageLen h then true
  else
    let s := sk.stageScope h ws.scope
    ws.parentDone &&
    (List.range (sk.nChildren h s)).all fun i =>
      ws.wireDone i &&
      (!sk.childIsD h s i ||
        (ws.resDone i && (ws.qSent i == sk.qCount h s i)))

/-- Advance past a completed scope after an obligation fires. Quint:
`normWalk`. -/
def normWalk (h : Nat) (ws : WalkSt) : WalkSt :=
  if ws.phase == 2 && scopeComplete sk h ws then freshWalk sk h (ws.scope + 1)
  else ws

def doneWalk (ws : WalkSt) : Bool := ws.phase == 5
def doneAsm (a : AsmSt) : Bool := a.phase == 4

def doneIOpen (s : State) : Bool := s.iopenWire && s.iopenQuery
def doneROpen (s : State) : Bool :=
  s.ropenGotWire && s.ropenWire && s.ropenRes &&
  (s.ropenQ == (sk.scope 0).kids.length)

/-- Is the producer of channel `c` finished, so a consumer's
recv-on-empty takes the end branch? Quint: `producerDone`
(MODEL.md §5 recvClose). -/
def producerDone (s : State) : Chan → Bool
  | .wire p h =>
      if h == sk.rootH then
        (if p == Party.I then doneIOpen s else doneROpen sk s)
      else doneWalk (s.walk (p, h))
  | .asked p h =>
      if p == Party.I && h == sk.rootH - 1 then doneIOpen s
      else if p == Party.R && h == sk.rootH - 2 then doneROpen sk s
      else doneWalk (s.walk (p, h + 2))
  | .leafRequests => doneWalk (s.walk (Party.I, 1))
  | .upper p h => doneWalk (s.walk (p, h))
  | .lower p h => doneWalk (s.walk (p, h))
  | _ => false -- level/root channels: never close-recv'd

/-- The initial state. Quint: `init` (minus `wellFormed`, which theorems
carry as a hypothesis). -/
def init : State :=
  { walk := fun pk => freshWalk sk pk.2 0
    asm := fun pk =>
      { idx := 0
        phase := if (sk.asmResList pk.1 pk.2).length > 0 then 0 else 3
        got := 0 }
    chan := fun _ => 0
    iopenWire := false, iopenQuery := false, iopenCh := none
    ropenGotWire := false, ropenWire := false, ropenRes := false
    ropenQ := 0, ropenCh := none
    absorbIdx := 0
    absorbPhase := if sk.totalLeafReqs > 0 then 0 else 3
    ifin := false, rfinGotRes := false, rfinGot := 0 }

-- ====================================================== opener guards

/-- Quint: `iopenChoosable`. -/
def iopenChoosable (s : State) : IOblig → Bool
  | .wire => !s.iopenWire
  | .query => !s.iopenQuery && (!ax.w || s.iopenWire)

/-- Quint: `ropenChoosable`. The query arm is deliberately NOT gated on
`ax.w`: the wire ledger never constrains dependent work (the
n2unrestricted finding; first Phase B CTI). -/
def ropenChoosable (s : State) : ROblig → Bool
  | .wire => s.ropenGotWire && !s.ropenWire
  | .res => s.ropenGotWire && !s.ropenRes && (!ax.w || s.ropenWire)
  | .query =>
      s.ropenGotWire && (s.ropenQ < (sk.scope 0).kids.length) &&
      (!ax.d1root || s.ropenRes) && (!ax.wireFirst || s.ropenWire)

-- ======================================================== walk guards

/-- May walk `pk` (in phase 2, uncommitted) commit to obligation `o`?
Quint: `wkChoosable`. The in-order (child-order) conjuncts are program
structure, never relaxed: positional pairing is the protocol's identity
carrier (work.rs:512-515; checked in Rust by `assert_valid`'s radix-order
rule). The `d5` conjunct on the wire and query arms is the parent
ledger (finding #7): once every D child of the scope is resolved, the
parent summary must depart before any further wire or query — exactly
the placement the weave pins (parent immediately after the final
resolution; first in an undisputed scope). -/
def wkChoosable (pk : Party × Nat) (ws : WalkSt) (o : Oblig) : Bool :=
  if ws.phase != 2 || ws.committed.isSome then false
  else
    let h := pk.2
    let s := sk.stageScope h ws.scope
    let n := sk.nChildren h s
    match o with
    | .wire i =>
        (i < n) && !ws.wireDone i &&
        (List.range i).all (fun j => ws.wireDone j) &&
        (!ax.d4 || (List.range i).all fun j =>
          !sk.childIsD h s j || (ws.resDone j && ws.qSent j == sk.qCount h s j)) &&
        (!ax.d5 || ws.parentDone ||
          !(List.range n).all fun j => !sk.childIsD h s j || ws.resDone j)
    | .res i =>
        (i < n) && sk.childIsD h s i && !ws.resDone i &&
        (List.range i).all (fun j => !sk.childIsD h s j || ws.resDone j) &&
        (!ax.w || ws.wireDone i) &&
        (!ax.d3 || (List.range n).all fun j =>
          !ws.resDone j || (ws.qSent j == sk.qCount h s j))
    | .query i =>
        (i < n) && sk.childIsD h s i &&
        (ws.qSent i < sk.qCount h s i) &&
        (List.range i).all (fun j => ws.qSent j == sk.qCount h s j) &&
        (!ax.d1int || ws.resDone i) &&
        (!ax.wireFirst || ws.wireDone i) &&
        (!ax.d5 || ws.parentDone ||
          !(List.range n).all fun j => !sk.childIsD h s j || ws.resDone j)
    | .parent =>
        !ws.parentDone &&
        (!ax.d2 || (List.range n).all fun j =>
          !sk.childIsD h s j || ws.resDone j)

/-- The channel a committed obligation fires into. -/
def obligChan (pk : Party × Nat) : Oblig → Chan
  | .wire _ => wireOut pk
  | .res _ => lowerOut pk
  | .query _ => askedOut pk
  | .parent => upperOut pk

/-- The walk record after obligation `o` fires. -/
def fireOblig (ws : WalkSt) (o : Oblig) : WalkSt :=
  match o with
  | .wire i => { ws with wireDone := fun j => j == i || ws.wireDone j, committed := none }
  | .res i => { ws with resDone := fun j => j == i || ws.resDone j, committed := none }
  | .query i => { ws with qSent := fun j => if j == i then ws.qSent j + 1 else ws.qSent j, committed := none }
  | .parent => { ws with parentDone := true, committed := none }

-- ============================================================= actions

/-- One atomic step of one process (MODEL.md §5: interleaving, one
enabled channel operation or commit per step). Constructors mirror the
Quint action branches one-for-one. -/
inductive Action
  | iopenChoose (o : IOblig)
  | iopenFire
  | ropenRecv
  | ropenChoose (o : ROblig)
  | ropenFire
  | walkRecvWire (pk : Party × Nat)
  | walkRecvAsked (pk : Party × Nat)
  | walkCommit (pk : Party × Nat) (o : Oblig)
  | walkFire (pk : Party × Nat)
  | walkCloseWire (pk : Party × Nat)
  | walkCloseAsked (pk : Party × Nat)
  | asmRecvRes (pk : Party × Nat)
  | asmRecvLevel (pk : Party × Nat)
  | asmSend (pk : Party × Nat)
  | asmClose (pk : Party × Nat)
  | absorbRecvWire
  | absorbRecvAsked
  | absorbSend
  | absorbCloseWire
  | absorbCloseAsked
  | finRet
  | finRes
  | finRets
  deriving DecidableEq, Repr

open Action

/-- Channel-occupancy helpers. -/
def bump (f : Chan → Nat) (c : Chan) (d : Int) : Chan → Nat :=
  fun c' => if c' == c then (Int.toNat (Int.ofNat (f c) + d)) else f c'

def setWalk (s : State) (pk : Party × Nat) (ws : WalkSt) : State :=
  { s with walk := fun pk' => if pk' = pk then ws else s.walk pk' }

def setAsm (s : State) (pk : Party × Nat) (a : AsmSt) : State :=
  { s with asm := fun pk' => if pk' = pk then a else s.asm pk' }

/-- Guarded transition function: `none` when the action's guard fails.
Quint: the bodies of `iopenChoose … finStep`, branch-for-branch. -/
def apply (a : Action) (s : State) : Option State :=
  match a with
  | iopenChoose o =>
      if s.iopenCh == none && iopenChoosable ax s o then
        some { s with iopenCh := some o }
      else none
  | iopenFire =>
      match s.iopenCh with
      | some .wire =>
          let c := Chan.wire Party.I sk.rootH
          if s.chan c < 1 then
            some { s with chan := bump s.chan c 1, iopenWire := true, iopenCh := none }
          else none
      | some .query =>
          let c := Chan.asked Party.I (sk.rootH - 1)
          if s.chan c < 1 then
            some { s with chan := bump s.chan c 1, iopenQuery := true, iopenCh := none }
          else none
      | none => none
  | ropenRecv =>
      let c := Chan.wire Party.I sk.rootH
      if !s.ropenGotWire && s.chan c > 0 then
        some { s with chan := bump s.chan c (-1), ropenGotWire := true }
      else none
  | ropenChoose o =>
      if s.ropenCh == none && ropenChoosable sk ax s o then
        some { s with ropenCh := some o }
      else none
  | ropenFire =>
      match s.ropenCh with
      | some .wire =>
          let c := Chan.wire Party.R sk.rootH
          if s.chan c < 1 then
            some { s with chan := bump s.chan c 1, ropenWire := true, ropenCh := none }
          else none
      | some .res =>
          if s.chan Chan.rootres < 1 then
            some { s with chan := bump s.chan Chan.rootres 1, ropenRes := true, ropenCh := none }
          else none
      | some .query =>
          let c := Chan.asked Party.R (sk.rootH - 2)
          if s.chan c < 1 then
            some { s with chan := bump s.chan c 1, ropenQ := s.ropenQ + 1, ropenCh := none }
          else none
      | none => none
  | walkRecvWire pk =>
      let ws := s.walk pk
      let c := wireIn pk
      -- Key membership is part of every walk/asm guard: `State.walk` is a
      -- total function, so a "phantom" walk at a non-stage key would
      -- otherwise be live — and (R, rootH-1)'s wireIn aliases the real
      -- opening channel, letting a phantom steal the opening message and
      -- deadlock a positive instance (transcription-review finding;
      -- Quint scopes pk with `oneOf(walkKeys)` instead).
      if sk.walkKeys.contains pk && ws.phase == 0 && s.chan c > 0 then
        some (setWalk { s with chan := bump s.chan c (-1) } pk
          { ws with phase := 1, committed := none })
      else none
  | walkRecvAsked pk =>
      let ws := s.walk pk
      let c := askedIn pk
      if sk.walkKeys.contains pk && ws.phase == 1 && s.chan c > 0 then
        some (setWalk { s with chan := bump s.chan c (-1) } pk
          (normWalk sk pk.2 { ws with phase := 2, committed := none }))
      else none
  | walkCommit pk o =>
      let ws := s.walk pk
      if sk.walkKeys.contains pk && wkChoosable sk ax pk ws o then
        some (setWalk s pk { ws with committed := some o })
      else none
  | walkFire pk =>
      let ws := s.walk pk
      match ws.committed with
      | some o =>
          let c := obligChan pk o
          if sk.walkKeys.contains pk && ws.phase == 2 && s.chan c < 1 then
            some (setWalk { s with chan := bump s.chan c 1 } pk
              (normWalk sk pk.2 (fireOblig ws o)))
          else none
      | none => none
  | walkCloseWire pk =>
      let ws := s.walk pk
      if sk.walkKeys.contains pk && ws.phase == 3 && producerDone sk s (wireIn pk) && s.chan (wireIn pk) == 0 then
        some (setWalk s pk { ws with phase := 4 })
      else none
  | walkCloseAsked pk =>
      let ws := s.walk pk
      if sk.walkKeys.contains pk && ws.phase == 4 && producerDone sk s (askedIn pk) && s.chan (askedIn pk) == 0 then
        some (setWalk s pk { ws with phase := 5 })
      else none
  | asmRecvRes pk =>
      let a := s.asm pk
      let c := asmResChan pk
      if sk.asmKeys.contains pk && a.phase == 0 && s.chan c > 0 then
        some (setAsm { s with chan := bump s.chan c (-1) } pk
          { a with phase := if sk.pendAt pk.1 pk.2 a.idx > 0 then 1 else 2, got := 0 })
      else none
  | asmRecvLevel pk =>
      let a := s.asm pk
      let c := asmLevelChan pk
      if sk.asmKeys.contains pk && a.phase == 1 && s.chan c > 0 then
        some (setAsm { s with chan := bump s.chan c (-1) } pk
          { a with phase := if a.got + 1 == sk.pendAt pk.1 pk.2 a.idx then 2 else 1,
                   got := a.got + 1 })
      else none
  | asmSend pk =>
      let a := s.asm pk
      let c := sk.asmOutChan pk
      if sk.asmKeys.contains pk && a.phase == 2 && s.chan c < sk.cap c then
        some (setAsm { s with chan := bump s.chan c 1 } pk
          { idx := a.idx + 1
            phase := if a.idx + 1 < (sk.asmResList pk.1 pk.2).length then 0 else 3
            got := 0 })
      else none
  | asmClose pk =>
      let a := s.asm pk
      let c := asmResChan pk
      if sk.asmKeys.contains pk && a.phase == 3 && producerDone sk s c && s.chan c == 0 then
        some (setAsm s pk { a with phase := 4 })
      else none
  | absorbRecvWire =>
      let c := Chan.wire Party.R 0
      if s.absorbPhase == 0 && s.chan c > 0 then
        some { s with chan := bump s.chan c (-1), absorbPhase := 1 }
      else none
  | absorbRecvAsked =>
      if s.absorbPhase == 1 && s.chan Chan.leafRequests > 0 then
        some { s with chan := bump s.chan Chan.leafRequests (-1), absorbPhase := 2 }
      else none
  | absorbSend =>
      let c := Chan.level Party.I 0
      if s.absorbPhase == 2 && s.chan c < sk.cap c then
        some { s with chan := bump s.chan c 1, absorbIdx := s.absorbIdx + 1,
                      absorbPhase := if s.absorbIdx + 1 < sk.totalLeafReqs then 0 else 3 }
      else none
  | absorbCloseWire =>
      let c := Chan.wire Party.R 0
      if s.absorbPhase == 3 && producerDone sk s c && s.chan c == 0 then
        some { s with absorbPhase := 4 }
      else none
  | absorbCloseAsked =>
      if s.absorbPhase == 4 && producerDone sk s Chan.leafRequests &&
          s.chan Chan.leafRequests == 0 then
        some { s with absorbPhase := 5 }
      else none
  | finRet =>
      if !s.ifin && s.chan Chan.rootret > 0 then
        some { s with chan := bump s.chan Chan.rootret (-1), ifin := true }
      else none
  | finRes =>
      if !s.rfinGotRes && s.chan Chan.rootres > 0 then
        some { s with chan := bump s.chan Chan.rootres (-1), rfinGotRes := true }
      else none
  | finRets =>
      if s.rfinGotRes && s.rfinGot < sk.rootPending && s.chan Chan.rootrets > 0 then
        some { s with chan := bump s.chan Chan.rootrets (-1), rfinGot := s.rfinGot + 1 }
      else none

-- ==================================================== the step relation

/-- Every action that could ever be enabled, as a finite list: the
enumeration behind `canStep` and behind `decide`-checked reachability.
Obligation indices range over the fan bound. -/
def allActions : List Action :=
  [iopenChoose .wire, iopenChoose .query, iopenFire,
   ropenRecv, ropenChoose .wire, ropenChoose .res, ropenChoose .query,
   ropenFire,
   absorbRecvWire, absorbRecvAsked, absorbSend, absorbCloseWire,
   absorbCloseAsked, finRet, finRes, finRets] ++
  sk.walkKeys.flatMap (fun pk =>
    [walkRecvWire pk, walkRecvAsked pk, walkFire pk,
     walkCloseWire pk, walkCloseAsked pk, walkCommit pk .parent] ++
    (List.range sk.fan).flatMap fun i =>
      [walkCommit pk (.wire i), walkCommit pk (.res i),
       walkCommit pk (.query i)]) ++
  sk.asmKeys.flatMap (fun pk =>
    [asmRecvRes pk, asmRecvLevel pk, asmSend pk, asmClose pk])

/-- Some process can act. Quint: `canStep` (proved equal to the
per-process disjunction there; here the enumeration IS the definition —
`allActions` completeness is a lemma, not an axiom). -/
def canStep (s : State) : Bool :=
  (allActions sk).any fun a => (apply sk ax a s).isSome

/-- The session is complete. Quint: `terminal`. -/
def terminal (s : State) : Bool :=
  (sk.walkKeys.all fun pk => doneWalk (s.walk pk)) &&
  (sk.asmKeys.all fun pk => doneAsm (s.asm pk)) &&
  doneIOpen s && doneROpen sk s &&
  (s.absorbPhase == 5) && s.ifin && s.rfinGotRes &&
  (s.rfinGot == sk.rootPending)

/-- The deadlock predicate: the model twin of `Quiescence::Stalled`.
Quint: `stuck`. -/
def stuck (s : State) : Bool := !terminal sk s && !canStep sk ax s

/-- Reachability from the initial state under the interleaving
semantics. -/
inductive Reachable : State → Prop
  | init : Reachable (init sk)
  | step {s s' : State} (a : Action) :
      Reachable s → apply sk ax a s = some s' → Reachable s'

/-- Run a list of actions from a state, failing on the first disabled
action — the executable spine of the negative-control witnesses. -/
def run (s : State) : List Action → Option State
  | [] => some s
  | a :: rest =>
      match apply sk ax a s with
      | some s' => run s' rest
      | none => none

theorem run_reachable {acts : List Action} {s' : State}
    (h : run sk ax (init sk) acts = some s') : Reachable sk ax s' := by
  suffices general : ∀ (acts : List Action) (s s' : State),
      Reachable sk ax s → run sk ax s acts = some s' → Reachable sk ax s' by
    exact general acts _ _ (.init) h
  intro acts
  induction acts with
  | nil =>
      intro s s' hr hrun
      simp only [run, Option.some.injEq] at hrun
      exact hrun ▸ hr
  | cons a rest ih =>
      intro s s' hr hrun
      unfold run at hrun
      cases happ : apply sk ax a s with
      | none => simp [happ] at hrun
      | some s₁ =>
          exact ih s₁ s' (.step a hr happ) (by simpa [happ] using hrun)

end Model

end StreamingMirror
