/-
The inductive invariant, transcribed from the Phase B Quint `indInv`
(`formal/quint/streamingMirror.qnt`): local per-process consistency plus
per-channel flow equations. `Inv` is Bool-valued and executable, so the
transcription is validated the same way Apalache's obligation 1 validated
the Quint original — by checking it along whole executions — before any
proof effort is spent on it. The proof stack (init, per-action
preservation, progress) builds on this definition.

Transcription rules carried from the Phase B CTI (see formal/README.md):
mirror the guards EXACTLY (W does not order queries after wires — the
wire ledger never constrains dependent work), and every fired fact gets
its own shadow lemma (`ax.w → resDone ⊆ wireDone`, at walks and at the
responder opening alike).
-/
import StreamingMirror.Model
import StreamingMirror.Instances

namespace StreamingMirror.Model

variable (sk : Skel) (ax : AxMode)

-- ============================================== derived channel counts
-- Cumulative operations, derived from process-local state. Quint:
-- wkWireRecvd … absorbLevelSent. Prologue recvs happen once per scope
-- (wire in phase 0→1, query in phase 1→2); past phase 2 every scope of
-- the stage is done. Sends: completed scopes contribute prefix-sum
-- totals, the current scope its live ledgers (empty outside phase 2).

def wkWireRecvd (s : State) (pk : Party × Nat) : Nat :=
  let ws := s.walk pk
  if ws.phase ≥ 3 then sk.stageLen pk.2
  else ws.scope + (if ws.phase == 1 || ws.phase == 2 then 1 else 0)

def wkAskedRecvd (s : State) (pk : Party × Nat) : Nat :=
  let ws := s.walk pk
  if ws.phase ≥ 3 then sk.stageLen pk.2
  else ws.scope + (if ws.phase == 2 then 1 else 0)

/-- Support-bounded count of a walk's live wire ledger. -/
def wkWireCount (s : State) (pk : Party × Nat) : Nat :=
  let ws := s.walk pk
  ((List.range sk.fan).filter fun i => ws.wireDone i).length

def wkResCount (s : State) (pk : Party × Nat) : Nat :=
  let ws := s.walk pk
  ((List.range sk.fan).filter fun i => ws.resDone i).length

def wkQSum (s : State) (pk : Party × Nat) : Nat :=
  let ws := s.walk pk
  (List.range sk.fan).foldl (fun acc i => acc + ws.qSent i) 0

def wkWireSent (s : State) (pk : Party × Nat) : Nat :=
  sk.wiresBefore pk.2 (s.walk pk).scope + wkWireCount sk s pk

def wkResSent (s : State) (pk : Party × Nat) : Nat :=
  sk.dsBefore pk.2 (s.walk pk).scope + wkResCount sk s pk

def wkQSentTot (s : State) (pk : Party × Nat) : Nat :=
  sk.qsBefore pk.2 (s.walk pk).scope + wkQSum sk s pk

def wkParentSent (s : State) (pk : Party × Nat) : Nat :=
  let ws := s.walk pk
  ws.scope + (if ws.phase == 2 && ws.parentDone then 1 else 0)

def asmOutSent (s : State) (pk : Party × Nat) : Nat := (s.asm pk).idx

def asmResRecvd (s : State) (pk : Party × Nat) : Nat :=
  let a := s.asm pk
  a.idx + (if a.phase == 1 || a.phase == 2 then 1 else 0)

def asmLevelRecvd (s : State) (pk : Party × Nat) : Nat :=
  let a := s.asm pk
  sk.pendsBefore pk.1 pk.2 a.idx + a.got

def absorbWireRecvd (s : State) : Nat :=
  if s.absorbPhase ≥ 3 then sk.totalLeafReqs
  else s.absorbIdx + (if s.absorbPhase == 1 || s.absorbPhase == 2 then 1 else 0)

def absorbAskedRecvd (s : State) : Nat :=
  if s.absorbPhase ≥ 3 then sk.totalLeafReqs
  else s.absorbIdx + (if s.absorbPhase == 2 then 1 else 0)

def b2n (b : Bool) : Nat := if b then 1 else 0

/-- Does assembler `pk` send to a root singleton (so its `level` channel
is dead)? Quint: `isRootOutKey`. -/
def isRootOutKey (sk : Skel) (pk : Party × Nat) : Bool :=
  (pk.1 == Party.I && pk.2 == sk.rootH) ||
  (pk.1 == Party.R && pk.2 == sk.rootH - 1)

/-- Cumulative sends into channel `c` by its unique producer. Quint:
`sentOf`. -/
def sentOf (s : State) : Chan → Nat
  | .wire p h =>
      if h == sk.rootH then
        (if p == Party.I then b2n s.iopenWire else b2n s.ropenWire)
      else wkWireSent sk s (p, h)
  | .asked p h =>
      if p == Party.I && h == sk.rootH - 1 then b2n s.iopenQuery
      else if p == Party.R && h == sk.rootH - 2 then s.ropenQ
      else wkQSentTot sk s (p, h + 2)
  | .leafRequests => wkQSentTot sk s (Party.I, 1)
  | .upper p h => wkParentSent s (p, h)
  | .lower p h => wkResSent sk s (p, h)
  | .level p j =>
      if p == Party.I && j == 0 then s.absorbIdx
      else if (sk.asmKeys.contains (p, j)) && !isRootOutKey sk (p, j) then
        asmOutSent s (p, j)
      else 0
  | .rootret => asmOutSent s (Party.I, sk.rootH)
  | .rootrets => asmOutSent s (Party.R, sk.rootH - 1)
  | .rootres => b2n s.ropenRes

/-- Cumulative receives from channel `c` by its unique consumer. Quint:
`recvdOf`. -/
def recvdOf (s : State) : Chan → Nat
  | .wire p h =>
      if h == sk.rootH then
        (if p == Party.I then b2n s.ropenGotWire
         else wkWireRecvd sk s (Party.I, sk.rootH - 1))
      else if p == Party.R && h == 0 then absorbWireRecvd sk s
      else wkWireRecvd sk s (p.other, h - 1)
  | .asked p h => wkAskedRecvd sk s (p, h)
  | .leafRequests => absorbAskedRecvd sk s
  | .upper p h => asmResRecvd s (p, h + 1)
  | .lower p h =>
      if sk.asmKeys.contains (p, h) then asmResRecvd s (p, h) else 0
  | .level p j =>
      if sk.asmKeys.contains (p, j + 1) then asmLevelRecvd sk s (p, j + 1)
      else 0
  | .rootret => b2n s.ifin
  | .rootrets => s.rfinGot
  | .rootres => b2n s.rfinGotRes

/-- Every channel the model touches, for the flow quantifier. Quint:
`allChans` (including the two dead `level` channels — the flow equations
force them to zero). -/
def allChans : List Chan :=
  (sk.walkKeys.flatMap fun pk =>
    [wireOut pk, askedIn pk, upperOut pk, lowerOut pk]) ++
  (sk.asmKeys.map fun pk => Chan.level pk.1 pk.2) ++
  [Chan.wire Party.I sk.rootH, Chan.wire Party.R sk.rootH,
   Chan.leafRequests, Chan.level Party.I 0,
   Chan.rootret, Chan.rootrets, Chan.rootres]

-- ===================================================== local invariants

/-- Per-walk structural consistency. Quint: `wkLocalOk`. -/
def wkLocalOk (s : State) (pk : Party × Nat) : Bool :=
  let ws := s.walk pk
  let h := pk.2
  let len := sk.stageLen h
  let sc := sk.stageScope h ws.scope
  let n := sk.nChildren h sc
  -- scope cursor vs phase
  (if ws.phase ≤ 2 then ws.scope < len else ws.scope == len) &&
  (decide (ws.phase ≤ 5)) &&
  -- publishing machinery live only in phase 2
  (ws.phase == 2 ||
    ((List.range sk.fan).all fun i =>
      !ws.wireDone i && !ws.resDone i && ws.qSent i == 0) &&
    !ws.parentDone && ws.committed == none) &&
  (ws.phase != 2 ||
    (!scopeComplete sk h ws &&
     ((List.range sk.fan).all fun j =>
       (!ws.wireDone j || (decide (j < n) &&
          (j == 0 || ws.wireDone (j - 1)))) &&
       (!ws.resDone j || (decide (j < n) && sk.childIsD h sc j)) &&
       (ws.qSent j ≤ sk.qCount h sc j) &&
       (ws.qSent j == 0 || (List.range j).all fun j2 =>
         ws.qSent j2 == sk.qCount h sc j2) &&
       (!ws.resDone j || (List.range j).all fun j2 =>
         !sk.childIsD h sc j2 || ws.resDone j2) &&
       (!ax.w || !ws.resDone j || ws.wireDone j) &&
       (!ax.d1int || ws.qSent j == 0 || ws.resDone j) &&
       (!ax.wireFirst || ws.qSent j == 0 || ws.wireDone j) &&
       (!ax.d3 || !ws.resDone j || ws.qSent j == sk.qCount h sc j ||
         (List.range n).all fun j2 =>
           j2 == j || !ws.resDone j2 || ws.qSent j2 == sk.qCount h sc j2) &&
       -- fired-fact shadow of the d4 wire guard (finding #6)
       (!ax.d4 || !ws.wireDone j || (List.range j).all fun j2 =>
         !sk.childIsD h sc j2 ||
           (ws.resDone j2 && ws.qSent j2 == sk.qCount h sc j2))) &&
     (match ws.committed with
      | none => true
      | some (.wire i) =>
          (i == wkWireCount sk s pk) && decide (i < n) &&
          (!ax.d4 || (List.range i).all fun j =>
            !sk.childIsD h sc j ||
              (ws.resDone j && ws.qSent j == sk.qCount h sc j))
      | some (.res i) =>
          decide (i < n) && sk.childIsD h sc i && !ws.resDone i &&
          ((List.range i).all fun j => !sk.childIsD h sc j || ws.resDone j) &&
          (!ax.w || ws.wireDone i) &&
          (!ax.d3 || (List.range n).all fun j =>
            !ws.resDone j || ws.qSent j == sk.qCount h sc j)
      | some (.query i) =>
          decide (i < n) && sk.childIsD h sc i &&
          (ws.qSent i < sk.qCount h sc i) &&
          ((List.range i).all fun j => ws.qSent j == sk.qCount h sc j) &&
          (!ax.d1int || ws.resDone i) &&
          (!ax.wireFirst || ws.wireDone i)
      | some .parent =>
          !ws.parentDone &&
          (!ax.d2 || (List.range n).all fun j =>
            !sk.childIsD h sc j || ws.resDone j))))

/-- Per-assembler structural consistency. Quint: `asmLocalOk`. -/
def asmLocalOk (s : State) (pk : Party × Nat) : Bool :=
  let a := s.asm pk
  let len := (sk.asmResList pk.1 pk.2).length
  (if a.phase ≤ 2 then a.idx < len else a.idx == len) &&
  (decide (a.phase ≤ 4)) &&
  (a.phase != 1 || a.got < sk.pendAt pk.1 pk.2 a.idx) &&
  (a.phase != 2 || a.got == sk.pendAt pk.1 pk.2 a.idx) &&
  (!(a.phase == 0 || a.phase ≥ 3) || a.got == 0)

/-- Openers, absorb, finishes. Quint: `topLocalOk`, including the
fired-fact shadow the first CTI demanded. -/
def topLocalOk (s : State) : Bool :=
  (s.iopenCh != some .wire || !s.iopenWire) &&
  (s.iopenCh != some .query || (!s.iopenQuery && (!ax.w || s.iopenWire))) &&
  (s.ropenGotWire ||
    (!s.ropenWire && !s.ropenRes && s.ropenQ == 0 && s.ropenCh == none)) &&
  (s.ropenQ ≤ sk.rootPending) &&
  -- fired-fact shadow of the res guard (first Phase B CTI)
  (!ax.w || !s.ropenRes || s.ropenWire) &&
  (s.ropenCh != some .wire || !s.ropenWire) &&
  (s.ropenCh != some .res || (!s.ropenRes && (!ax.w || s.ropenWire))) &&
  (s.ropenCh != some .query ||
    ((s.ropenQ < sk.rootPending) &&
     (!ax.d1root || s.ropenRes) &&
     (!ax.wireFirst || s.ropenWire))) &&
  (if s.absorbPhase ≤ 2 then s.absorbIdx < sk.totalLeafReqs
   else s.absorbIdx == sk.totalLeafReqs) &&
  (decide (s.absorbPhase ≤ 5)) &&
  (s.rfinGotRes || s.rfinGot == 0) &&
  (s.rfinGot ≤ sk.rootPending)

/-- Flow conservation plus occupancy: every channel holds exactly what
its producer sent minus what its consumer received, within capacity.
Quint: `indFlow` and `occupancyOk`. -/
def flowOk (s : State) : Bool :=
  (allChans sk).all fun c =>
    (s.chan c + recvdOf sk s c == sentOf sk s c) && (s.chan c ≤ sk.cap c)

/-- The inductive invariant. Quint: `indInv` (typing is carried by the
Lean types plus the bounds above). -/
def Inv (s : State) : Bool :=
  (sk.walkKeys.all fun pk => wkLocalOk sk ax s pk) &&
  (sk.asmKeys.all fun pk => asmLocalOk sk s pk) &&
  topLocalOk sk ax s && flowOk sk s

end StreamingMirror.Model

namespace StreamingMirror.Pin

open Model

/-- Drive `fuel` steps greedily, checking `Inv` at every state along the
way — the executable transcription check (Apalache obligation 1, and a
strong approximation of obligation 2, replayed in Lean). -/
def invAlong (sk : Skel) (ax : AxMode) : Nat → State → Bool
  | 0, s => Inv sk ax s
  | fuel + 1, s =>
      Inv sk ax s &&
      match (allActions sk).firstM (fun a => apply sk ax a s) with
      | some s' => invAlong sk ax fuel s'
      | none => Inv sk ax s

#eval [invAlong smokeChain .full 300 (init smokeChain),
       invAlong rMix .full 500 (init rMix),
       invAlong comb6 .full 600 (init comb6),
       invAlong (pyramid 4) .full 700 (init (pyramid 4)),
       invAlong (pyramid 2) .full 700 (init (pyramid 2))]

/-- The invariant holds along entire greedy executions of the pinned
positive matrix — the Lean twin of Apalache's obligation-1 pass plus a
schedule's worth of obligation 2. -/
theorem inv_along_positives :
    (invAlong smokeChain .full 300 (init smokeChain)) &&
    (invAlong rMix .full 500 (init rMix)) &&
    (invAlong comb6 .full 600 (init comb6)) &&
    (invAlong (pyramid 4) .full 700 (init (pyramid 4))) &&
    (invAlong (pyramid 2) .full 700 (init (pyramid 2))) = true := by
  native_decide

end StreamingMirror.Pin
