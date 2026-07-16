/-
EventDag: scratch analysis tool (not part of the library; imported by
nothing). Enumerates the event dependency DAG of the streaming-mirror
protocol model for a pinned skeleton, checks acyclicity via Kahn's
algorithm, and computes longest-path depths.

Nodes: (chan, side, seq) for every message a completed session carries.
Edges: E1 message (snd → rcv, positionally paired), E2 back-pressure
(rcv n → snd n+cap), E3 forced program order per process (only what the
.full axiom guards force, not what a particular scheduler picks).

Per-channel totals are derived twice — analytically from the process-side
enumeration, and empirically from `sentOf`/`recvdOf` at the terminal
state of a greedy run — and asserted equal (the self-check against
mis-wiring).

Run: lake exe eventdag [outDir] [fuzzN]   (dumps default to
./eventdag-out, which is gitignored; fuzzN defaults to 100 random
skeletons, ~10s compiled; exit code is nonzero on any failed check, so
the `just all` sweep can gate on it.)

Role in the progress proof (PROGRESS.md §3, §5): this is the standing
ORACLE for the schedule construction. Acyclicity of this DAG is "a
valid timestamp exists"; the depth dumps are what any candidate
construction must be fitted against, and the validate-then-prove
workflow requires a candidate to pass this tool's edge check on every
pinned skeleton BEFORE proof effort is spent on it. The depth tables
already refuted the closed-form lex-timestamp design (PROGRESS.md §4):
depths jump at subtree boundaries, so the potential is tree-recursive.
-/
import StreamingMirror
import Std.Data.HashMap
import Std.Data.HashSet

open StreamingMirror
open StreamingMirror.Model

namespace EventDag

instance : Inhabited Chan := ⟨.leafRequests⟩

/-- Event: channel, side (`true` = snd, `false` = rcv), 0-based seq. -/
abbrev Ev := Chan × Bool × Nat

def pn : Party → Nat
  | .I => 0
  | .R => 1

def pStr : Party → String
  | .I => "I"
  | .R => "R"

def chanStr : Chan → String
  | .wire p h => s!"wire {pStr p} {h}"
  | .asked p h => s!"asked {pStr p} {h}"
  | .leafRequests => "leafRequests"
  | .upper p h => s!"upper {pStr p} {h}"
  | .lower p h => s!"lower {pStr p} {h}"
  | .level p j => s!"level {pStr p} {j}"
  | .rootret => "rootret"
  | .rootrets => "rootrets"
  | .rootres => "rootres"

/-- Injective Nat key for a channel (heights < 1000 assumed). -/
def chanKey : Chan → Nat
  | .wire p h => 10000 + 1000 * pn p + h
  | .asked p h => 20000 + 1000 * pn p + h
  | .leafRequests => 30000
  | .upper p h => 40000 + 1000 * pn p + h
  | .lower p h => 50000 + 1000 * pn p + h
  | .level p j => 60000 + 1000 * pn p + j
  | .rootret => 70000
  | .rootrets => 80000
  | .rootres => 90000

/-- Injective Nat key for an event (seqs < 10^6 assumed). -/
def evKey (e : Ev) : Nat :=
  (chanKey e.1 * 2 + (if e.2.1 then 1 else 0)) * 1000000 + e.2.2

def evStr (e : Ev) : String :=
  s!"{chanStr e.1}\t{if e.2.1 then "snd" else "rcv"}\t{e.2.2}"

-- ================================================= per-process E3 traces

/-- Walk (p, h): per scope k, rcvW → rcvA → sends → rcvW(k+1), with the
in-order wire chain, W (wire i → res i), d1int + in-order queries
(res i → q(i,0) → q(i,1) → …), in-order D-resolution prefix, d4
(last event of D child i's block → wire i+1), and d2 (res i → parent).
The parent send otherwise floats (only rcvA and d2 constrain it).

The NODES array is also the canonical E3 linearization the schedule
merge (`schedCandidate`) consumes, and the parent's floating position
is pinned there deliberately: immediately after the scope's final
resolution (directly after rcvA when the scope has no D children) —
NOT at the scope's end. Parent-last deadlocks the merge: the last D
block's trailing queries can require descent that requires assembly
that requires this very parent (fuzz seed 13's four-process cursor
cycle), while parent-after-last-res is safe — the upper window depends
only on strictly earlier scopes' subtrees. Edges are unaffected by the
placement. -/
def walkTrace (sk : Skel) (pk : Party × Nat) : Array Ev × Array (Ev × Ev) := Id.run do
  let h := pk.2
  let len := sk.stageLen h
  let mut nodes : Array Ev := #[]
  let mut edges : Array (Ev × Ev) := #[]
  let mut wireCnt := 0
  let mut resCnt := 0
  let mut qCnt := 0
  for k in [0:len] do
    let s := sk.stageScope h k
    let n := sk.nChildren h s
    let rcvW : Ev := (wireIn pk, false, k)
    let rcvA : Ev := (askedIn pk, false, k)
    nodes := nodes.push rcvW
    nodes := nodes.push rcvA
    edges := edges.push (rcvW, rcvA)
    let mut sends : Array Ev := #[]
    let mut wireEvs : Array Ev := #[]
    let mut resEvs : Array (Option Ev) := #[]
    let mut lastOfBlock : Array (Option Ev) := #[]
    let mut lastResPos : Nat := 0               -- insertion point for parent
    for i in [0:n] do
      let wEv : Ev := (wireOut pk, true, wireCnt)
      wireCnt := wireCnt + 1
      wireEvs := wireEvs.push wEv
      sends := sends.push wEv
      if sk.childIsD h s i then
        let rEv : Ev := (lowerOut pk, true, resCnt)
        resCnt := resCnt + 1
        sends := sends.push rEv
        lastResPos := sends.size
        edges := edges.push (wEv, rEv)          -- W: wire(i) → res(i)
        resEvs := resEvs.push (some rEv)
        let q := sk.qCount h s i
        let mut prev := rEv
        for _t in [0:q] do
          let qEv : Ev := (askedOut pk, true, qCnt)
          qCnt := qCnt + 1
          sends := sends.push qEv
          edges := edges.push (prev, qEv)       -- d1int, then in-order
          prev := qEv
        lastOfBlock := lastOfBlock.push (some prev)
      else
        resEvs := resEvs.push none
        lastOfBlock := lastOfBlock.push none
    let parentEv : Ev := (upperOut pk, true, k)
    sends := sends.insertIdx! lastResPos parentEv
    -- in-order wires; d4: D block i complete before wire(i+1)
    for i in [0:n] do
      if i + 1 < n then
        edges := edges.push (wireEvs[i]!, wireEvs[i+1]!)
        match lastOfBlock[i]! with
        | some lastEv => edges := edges.push (lastEv, wireEvs[i+1]!)
        | none => pure ()
    -- in-order D-resolution prefix (all the ORDER d3's contiguity
    -- ledger forces at event granularity); d2: res(i) → parent
    let mut prevRes : Option Ev := none
    for i in [0:n] do
      match resEvs[i]! with
      | some rEv =>
          match prevRes with
          | some pr => edges := edges.push (pr, rEv)
          | none => pure ()
          prevRes := some rEv
          edges := edges.push (rEv, parentEv)
      | none => pure ()
    -- prologue precedes all sends; scope completes before the next recv
    for e in sends do
      nodes := nodes.push e
      edges := edges.push (rcvA, e)
      if k + 1 < len then
        edges := edges.push (e, (wireIn pk, false, k + 1))
  return (nodes, edges)

/-- iopen: wire, then (W) query. -/
def iopenTrace (sk : Skel) : Array Ev × Array (Ev × Ev) :=
  let wEv : Ev := (Chan.wire Party.I sk.rootH, true, 0)
  let qEv : Ev := (Chan.asked Party.I (sk.rootH - 1), true, 0)
  (#[wEv, qEv], #[(wEv, qEv)])

/-- ropen: gotWire → {wire, res, queries}; W: wire → res; d1root:
res → query 0; queries in order. -/
def ropenTrace (sk : Skel) : Array Ev × Array (Ev × Ev) := Id.run do
  let gotW : Ev := (Chan.wire Party.I sk.rootH, false, 0)
  let wEv : Ev := (Chan.wire Party.R sk.rootH, true, 0)
  let rEv : Ev := (Chan.rootres, true, 0)
  let mut nodes : Array Ev := #[gotW, wEv, rEv]
  let mut edges : Array (Ev × Ev) := #[(gotW, wEv), (gotW, rEv), (wEv, rEv)]
  let mut prev := rEv
  for j in [0:sk.rootPending] do
    let qEv : Ev := (Chan.asked Party.R (sk.rootH - 2), true, j)
    nodes := nodes.push qEv
    edges := edges.push (gotW, qEv)
    edges := edges.push (prev, qEv)
    prev := qEv
  return (nodes, edges)

/-- Absorb: rcv wire → rcv leafRequest → snd level, looped. -/
def absorbTrace (sk : Skel) : Array Ev × Array (Ev × Ev) := Id.run do
  let mut nodes : Array Ev := #[]
  let mut edges : Array (Ev × Ev) := #[]
  for j in [0:sk.totalLeafReqs] do
    let rw : Ev := (Chan.wire Party.R 0, false, j)
    let ra : Ev := (Chan.leafRequests, false, j)
    let sd : Ev := (Chan.level Party.I 0, true, j)
    nodes := ((nodes.push rw).push ra).push sd
    edges := edges.push (rw, ra)
    edges := edges.push (ra, sd)
    if j + 1 < sk.totalLeafReqs then
      edges := edges.push (sd, (Chan.wire Party.R 0, false, j + 1))
  return (nodes, edges)

/-- Asm (p, j): rcv res idx → pendAt idx level rcvs (in order) → snd idx
→ next rcv res. -/
def asmTrace (sk : Skel) (pk : Party × Nat) : Array Ev × Array (Ev × Ev) := Id.run do
  let resList := sk.asmResList pk.1 pk.2
  let cRes := asmResChan pk
  let cLvl := asmLevelChan pk
  let cOut := sk.asmOutChan pk
  let mut nodes : Array Ev := #[]
  let mut edges : Array (Ev × Ev) := #[]
  let mut lvlCnt := 0
  for idx in [0:resList.length] do
    let rcvRes : Ev := (cRes, false, idx)
    nodes := nodes.push rcvRes
    let pend := sk.pendAt pk.1 pk.2 idx
    let mut prev := rcvRes
    for _t in [0:pend] do
      let lv : Ev := (cLvl, false, lvlCnt)
      lvlCnt := lvlCnt + 1
      nodes := nodes.push lv
      edges := edges.push (prev, lv)
      prev := lv
    let sd : Ev := (cOut, true, idx)
    nodes := nodes.push sd
    edges := edges.push (prev, sd)
    if idx + 1 < resList.length then
      edges := edges.push (sd, (cRes, false, idx + 1))
  return (nodes, edges)

/-- fins: one rootret rcv (floats); rootres rcv → rootrets rcvs in
order (the rfinGotRes guard). -/
def finTrace (sk : Skel) : Array Ev × Array (Ev × Ev) := Id.run do
  let retEv : Ev := (Chan.rootret, false, 0)
  let resEv : Ev := (Chan.rootres, false, 0)
  let mut nodes : Array Ev := #[retEv, resEv]
  let mut edges : Array (Ev × Ev) := #[]
  let mut prev := resEv
  for j in [0:sk.rootPending] do
    let rv : Ev := (Chan.rootrets, false, j)
    nodes := nodes.push rv
    edges := edges.push (prev, rv)
    prev := rv
  return (nodes, edges)

/-- All process-side events and E3 edges (close events ignored). -/
def procTraces (sk : Skel) : Array Ev × Array (Ev × Ev) :=
  let traces := #[iopenTrace sk, ropenTrace sk, absorbTrace sk, finTrace sk]
    ++ (sk.walkKeys.map (walkTrace sk)).toArray
    ++ (sk.asmKeys.map (asmTrace sk)).toArray
  traces.foldl (fun acc t => (acc.1 ++ t.1, acc.2 ++ t.2)) (#[], #[])

-- ============================================= empirical ground truth

/-- Greedy drain under the full axiom mode (Controls.drain, relocal). -/
def drainFull (sk : Skel) : Nat → State → State
  | 0, s => s
  | fuel + 1, s =>
      match (allActions sk).firstM (fun a => apply sk .full a s) with
      | some s' => drainFull sk fuel s'
      | none => s

/-- `Model.allChans`, deduplicated. -/
def chanList (sk : Skel) : Array Chan := Id.run do
  let mut seen : Std.HashSet Nat := {}
  let mut out : Array Chan := #[]
  for c in allChans sk do
    if !seen.contains (chanKey c) then
      seen := seen.insert (chanKey c)
      out := out.push c
  return out

-- ==================================================== the full edge set

/-- E1 (message) and E2 (back-pressure) edges for the given event set,
appended to the E3 process edges. `capOne` forces every capacity to 1
(the experiment knob). One definition shared by the analyzer and the
schedule validator, so a candidate schedule is checked against exactly
the edge set the acyclicity oracle certifies. -/
def dagEdges (sk : Skel) (nodes : Array Ev) (procEdges : Array (Ev × Ev))
    (capOne : Bool := false) : Array (Ev × Ev) := Id.run do
  let mut sndCnt : Std.HashMap Nat Nat := {}
  for (c, sd, _sq) in nodes do
    if sd then
      let k := chanKey c
      sndCnt := sndCnt.insert k (sndCnt.getD k 0 + 1)
  let mut out := procEdges
  for c in chanList sk do
    let t := sndCnt.getD (chanKey c) 0
    let cap := if capOne then 1 else sk.cap c
    for n in [0:t] do
      out := out.push ((c, true, n), (c, false, n))
      if n + cap < t then
        out := out.push ((c, false, n), (c, true, n + cap))
  return out

-- ================================================= candidate schedules

/-- The event trace of the greedy run: fire the first enabled action
until quiescent, recording each step's channel operations (receives
before sends) as the per-channel `sentOf`/`recvdOf` deltas.

Any completed run linearizes the DAG — channel occupancy forces E1/E2,
the `.full` guards force E3 — so this trace passing `validateSchedule`
is a coherence check between the model, the edge enumeration, and the
validator itself. It is NOT the §5 candidate (a run is not a structural
recursion over the skeleton); it is the reference interleaving the
candidate is fitted against. -/
def greedySchedule (sk : Skel) (fuel : Nat := 50000) : Array Ev := Id.run do
  let chans := chanList sk
  let mut s := init sk
  let mut out : Array Ev := #[]
  for _ in [0:fuel] do
    match (allActions sk).firstM (fun a => apply sk .full a s) with
    | some s' =>
        for c in chans do
          for n in [recvdOf sk s c : recvdOf sk s' c] do
            out := out.push (c, false, n)
        for c in chans do
          for n in [sentOf sk s c : sentOf sk s' c] do
            out := out.push (c, true, n)
        s := s'
    | none => break
  return out

-- ====================================== the §5 candidate construction

/-- Merge state: the emitted prefix, per-channel emitted snd/rcv
counts, and one cursor per process trace. -/
structure SB where
  out : Array Ev
  sent : Std.HashMap Nat Nat
  rcvd : Std.HashMap Nat Nat
  cur : Array Nat
  deriving Inhabited

def SB.emitEv (b : SB) (e : Ev) : SB :=
  let (c, sd, _) := e
  let k := chanKey c
  if sd then
    { b with out := b.out.push e, sent := b.sent.insert k (b.sent.getD k 0 + 1) }
  else
    { b with out := b.out.push e, rcvd := b.rcvd.insert k (b.rcvd.getD k 0 + 1) }

/-- One merge step: emit the first process's enabled next-event, in the
fixed priority order.

Enabledness is checked against the emitted prefix: a receive needs its
message's send emitted (E1), a send needs its cap window open (E2), and
the per-process cursor is E3 (each trace is an E3 linearization of its
process's forced order). An emitted event therefore has all its DAG
predecessors emitted, BY CONSTRUCTION: the merge cannot produce an
edge-violating order — its only failure mode is stalling with events
unemitted, which the permutation half of `validateSchedule` catches. -/
def mergeOnce (sk : Skel) (procs : Array (Array Ev)) (b : SB) : Option SB := Id.run do
  for i in [0:procs.size] do
    let t := procs[i]!
    let c := b.cur[i]!
    if c < t.size then
      let e := t[c]!
      let (ch, sd, n) := e
      let ok :=
        if sd then b.rcvd.getD (chanKey ch) 0 + sk.cap ch > n
        else b.sent.getD (chanKey ch) 0 > n
      if ok then
        return some { b.emitEv e with cur := b.cur.set! i (c + 1) }
  return none

/-- Merge to fixpoint (terminates: every step emits one of finitely
many events). -/
partial def mergeAll (sk : Skel) (procs : Array (Array Ev)) (b : SB) : SB :=
  match mergeOnce sk procs b with
  | some b' => mergeAll sk procs b'
  | none => b

/-- The §5 candidate: the deterministic priority merge of the
per-process event traces.

Every process contributes its E3-linear trace (the `procTraces` node
arrays, which linearize the forced partial order; `fins` splits in two
because its `rootret` receive floats). The traces are ordered descent
before assembly: openers, walks by descending stage, absorb, the asm
towers bottom-up (I then R), fins. The merge repeatedly emits the
first trace whose next event is enabled (see `mergeOnce`).

Positions are therefore merge-emergent, not static — the earlier
static-placement design is refuted (PROGRESS.md §4): stall regions
must move walk-side events past `post(parent)`, so no per-scope
position assignment survives. What the proofs get instead, by
construction: edge-respect, and τ monotone along every process trace.
The Lean obligation this prefigures is completeness — the merge drains
every trace — which is where the capLevel hypothesis (§5) must enter. -/
def schedCandidate (sk : Skel) : Array Ev :=
  let walkOrder : List (Party × Nat) :=
    (List.range sk.rootH).map fun i =>
      let h := sk.rootH - 1 - i
      (if h % 2 == 1 then Party.I else Party.R, h)
  let procs : Array (Array Ev) :=
    #[(iopenTrace sk).1, (ropenTrace sk).1]
    ++ (walkOrder.toArray.map (fun pk => (walkTrace sk pk).1))
    ++ #[(absorbTrace sk).1]
    ++ (sk.asmKeys.toArray.map (fun pk => (asmTrace sk pk).1))
    -- finTrace's nodes are [rootret rcv, rootres rcv, rootrets rcvs…];
    -- the rootret receive floats, so it merges as its own trace.
    ++ #[((finTrace sk).1.extract 0 1), ((finTrace sk).1.extract 1 ((finTrace sk).1.size))]
  (mergeAll sk procs ⟨#[], {}, {}, Array.replicate procs.size 0⟩).out

-- =================================================== the replay witness

/-- Compile one event into the model actions that perform it, given the
per-walk scope cursor (`k`) of the event's owner.

Walk sends become a `walkCommit`/`walkFire` pair (committed immediately
before firing — the replay never jams a commitment), with the
obligation's child index recovered from the scope's prefix sums.
Opener sends are `Choose`/`Fire` pairs likewise. -/
def evActions (sk : Skel) (walkScope : Party × Nat → Nat) (e : Ev) : List Action := Id.run do
  let (c, sd, sq) := e
  -- walk send helper: child index of the scope-k obligation
  let commitFire := fun (pk : Party × Nat) (o : Oblig) =>
    [Action.walkCommit pk o, Action.walkFire pk]
  match c, sd with
  | .wire p h, true =>
      if h == sk.rootH then
        if p == Party.I then return [.iopenChoose .wire, .iopenFire]
        else return [.ropenChoose .wire, .ropenFire]
      else
        let pk := (p, h)
        let k := walkScope pk
        return commitFire pk (.wire (sq - sk.wiresBefore h k))
  | .wire p h, false =>
      if h == sk.rootH then
        if p == Party.I then return [.ropenRecv]
        else return [.walkRecvWire (Party.I, sk.rootH - 1)]
      else if h == 0 then return [.absorbRecvWire]
      else return [.walkRecvWire (p.other, h - 1)]
  | .asked p h, true =>
      if p == Party.I && h == sk.rootH - 1 then
        return [.iopenChoose .query, .iopenFire]
      else if p == Party.R && h == sk.rootH - 2 then
        return [.ropenChoose .query, .ropenFire]
      else
        let pk := (p, h + 2)
        let k := walkScope pk
        let s := sk.stageScope (h + 2) k
        let mut rel := sq - sk.qsBefore (h + 2) k
        for i in [0:sk.nChildren (h + 2) s] do
          let qc := sk.qCount (h + 2) s i
          if rel < qc then
            return commitFire pk (.query i)
          rel := rel - qc
        return []  -- unreachable on a coherent trace
  | .asked p h, false => return [.walkRecvAsked (p, h)]
  | .leafRequests, true =>
      let pk := (Party.I, 1)
      let k := walkScope pk
      let s := sk.stageScope 1 k
      let mut rel := sq - sk.qsBefore 1 k
      for i in [0:sk.nChildren 1 s] do
        let qc := sk.qCount 1 s i
        if rel < qc then
          return commitFire pk (.query i)
        rel := rel - qc
      return []
  | .leafRequests, false => return [.absorbRecvAsked]
  | .lower p h, true =>
      let pk := (p, h)
      let k := walkScope pk
      let s := sk.stageScope h k
      let dRank := sq - sk.dsBefore h k
      let mut seen := 0
      for i in [0:sk.nChildren h s] do
        if sk.childIsD h s i then
          if seen == dRank then
            return commitFire pk (.res i)
          seen := seen + 1
      return []
  | .lower p h, false => return [.asmRecvRes (p, h)]
  | .upper p h, true => return commitFire (p, h) .parent
  | .upper p h, false => return [.asmRecvRes (p, h + 1)]
  | .level p j, true =>
      if p == Party.I && j == 0 then return [.absorbSend]
      else return [.asmSend (p, j)]
  | .level p j, false => return [.asmRecvLevel (p, j + 1)]
  | .rootret, true => return [.asmSend (Party.I, sk.rootH)]
  | .rootret, false => return [.finRet]
  | .rootrets, true => return [.asmSend (Party.R, sk.rootH - 1)]
  | .rootrets, false => return [.finRets]
  | .rootres, true => return [.ropenChoose .res, .ropenFire]
  | .rootres, false => return [.finRes]

/-- Replay a schedule as a REAL model run: compile every event to its
actions, apply them in schedule order under `AxMode.full`, then drain
the close tier greedily and demand `terminal`.

This is the two-sided check on the trace enumeration: the DAG edges
being RESPECTED says the schedule contradicts nothing the model forces;
the replay reaching terminal says the model REFUSES nothing the
schedule does — i.e. the E3 families are not merely sound but complete
enough that the schedule is a genuine execution. A `none` mid-replay
(index of the offending event returned) means an event ordering the
trace layer allows was rejected by a real guard. The successful replay
is also the shape of the Phase D termination witness: an explicit
action list from `init` to `terminal`. -/
def replaySchedule (sk : Skel) (sched : Array Ev) : Option Nat × Bool := Id.run do
  let mut st := init sk
  let mut walkScope : Std.HashMap (Nat × Nat) Nat := {}
  let key := fun (pk : Party × Nat) => (pn pk.1, pk.2)
  for i in [0:sched.size] do
    let e := sched[i]!
    -- the walk's scope cursor: rcvW #k enters scope k
    let lookup := fun pk => walkScope.getD (key pk) 0
    for a in evActions sk lookup e do
      match apply sk .full a st with
      | some st' => st := st'
      | none => return (some i, false)
    if let (Chan.wire p h, false, sq) := e then
      if h != sk.rootH && h != 0 then
        walkScope := walkScope.insert (key (p.other, h - 1)) sq
  return (none, terminal sk (drainFull sk 10000 st))

/-- The §5 acceptance check for a candidate schedule: (a) it is a
permutation of the event set, (b) every E1/E2/E3 edge `u ≺ v` has
`idx u < idx v`. Empty result = the candidate is a valid linearization
of the oracle's DAG. -/
def validateSchedule (sk : Skel) (sched : Array Ev) : Array String := Id.run do
  let (nodes, procEdges) := procTraces sk
  let mut errs : Array String := #[]
  let mut pos : Std.HashMap Nat Nat := {}
  for i in [0:sched.size] do
    let k := evKey sched[i]!
    if pos.contains k then
      errs := errs.push s!"duplicate event: {evStr sched[i]!}"
    pos := pos.insert k i
  if sched.size != nodes.size then
    errs := errs.push s!"size mismatch: schedule {sched.size} vs event set {nodes.size}"
  for e in nodes do
    if !pos.contains (evKey e) then
      errs := errs.push s!"missing event: {evStr e}"
  for (u, v) in dagEdges sk nodes procEdges do
    match pos.get? (evKey u), pos.get? (evKey v) with
    | some pu, some pv =>
        if pu ≥ pv then
          errs := errs.push
            s!"edge violated: [{evStr u}] @{pu} must precede [{evStr v}] @{pv}"
    | _, _ => pure ()  -- endpoint missing: already reported above
  return errs

-- ======================================================== the analysis

structure Analysis where
  totalErrs : Array String     -- totals cross-check failures (empty = pass)
  acyclic : Bool
  cycle : Array String         -- a cycle, if found (reverse-edge walk)
  nodes : Array Ev
  depths : Array Nat
  edgeCount : Nat
  maxDepth : Nat
  monoErrs : Array String      -- per-channel-side seq/depth monotonicity

/-- Full DAG analysis. `capOne` overrides every channel capacity to 1 in
the E2 back-pressure edges (an experiment knob, not the model); the
cap-1 pass also skips the greedy empirical drain, which is capacity-
independent and thus pure repeat work. -/
def analyze (sk : Skel) (fuel : Nat := 50000) (capOne : Bool := false) : Analysis := Id.run do
  let (nodes, procEdges) := procTraces sk
  let nN := nodes.size
  let mut errs : Array String := #[]
  -- analytic per-channel totals
  let mut sndCnt : Std.HashMap Nat Nat := {}
  let mut rcvCnt : Std.HashMap Nat Nat := {}
  for (c, sd, _sq) in nodes do
    let k := chanKey c
    if sd then sndCnt := sndCnt.insert k (sndCnt.getD k 0 + 1)
    else rcvCnt := rcvCnt.insert k (rcvCnt.getD k 0 + 1)
  let chans := chanList sk
  let mut chanKeys : Std.HashSet Nat := {}
  for c in chans do
    chanKeys := chanKeys.insert (chanKey c)
  for (c, _sd, _sq) in nodes do
    if !chanKeys.contains (chanKey c) then
      errs := errs.push s!"event channel {chanStr c} not in allChans"
  -- empirical: greedy run to terminal (capacity-independent; skipped
  -- under the cap-1 experiment to avoid repeating identical work)
  if !capOne then
    let fin := drainFull sk fuel (init sk)
    if !(terminal sk fin) then
      errs := errs.push "greedy run did NOT reach terminal"
    for c in chans do
      let eSnd := sentOf sk fin c
      let eRcv := recvdOf sk fin c
      let aSnd := sndCnt.getD (chanKey c) 0
      let aRcv := rcvCnt.getD (chanKey c) 0
      if aSnd != eSnd then
        errs := errs.push s!"{chanStr c}: analytic snd {aSnd} != sentOf {eSnd}"
      if aRcv != eRcv then
        errs := errs.push s!"{chanStr c}: analytic rcv {aRcv} != recvdOf {eRcv}"
      if eSnd != eRcv then
        errs := errs.push s!"{chanStr c}: sentOf {eSnd} != recvdOf {eRcv} at terminal"
  -- E1 (message) and E2 (back-pressure) edges
  let allEdges := dagEdges sk nodes procEdges (capOne := capOne)
  -- node ids
  let mut idOf : Std.HashMap Nat Nat := {}
  for i in [0:nN] do
    let k := evKey nodes[i]!
    if idOf.contains k then
      errs := errs.push s!"duplicate node {evStr nodes[i]!}"
    idOf := idOf.insert k i
  -- edge list by id, deduplicated
  let mut edgeSet : Std.HashSet Nat := {}
  let mut edges : Array (Nat × Nat) := #[]
  for (u, v) in allEdges do
    match idOf.get? (evKey u), idOf.get? (evKey v) with
    | some ui, some vi =>
        let key := ui * nN + vi
        if !edgeSet.contains key then
          edgeSet := edgeSet.insert key
          edges := edges.push (ui, vi)
    | _, _ =>
        errs := errs.push s!"edge endpoint missing: [{evStr u}] -> [{evStr v}]"
  -- Kahn topological sort with longest-path depths
  let mut indeg : Array Nat := Array.replicate nN 0
  let mut adj : Array (Array Nat) := Array.replicate nN #[]
  for (u, v) in edges do
    indeg := indeg.set! v (indeg[v]! + 1)
    adj := adj.set! u (adj[u]!.push v)
  let mut depth : Array Nat := Array.replicate nN 0
  let mut queue : Array Nat := #[]
  for i in [0:nN] do
    if indeg[i]! == 0 then queue := queue.push i
  let mut head := 0
  let mut emitted := 0
  for _ in [0:nN] do
    if head < queue.size then
      let u := queue[head]!
      head := head + 1
      emitted := emitted + 1
      for v in adj[u]! do
        depth := depth.set! v (max depth[v]! (depth[u]! + 1))
        indeg := indeg.set! v (indeg[v]! - 1)
        if indeg[v]! == 0 then queue := queue.push v
  let acyclic := emitted == nN
  -- cycle extraction (walk backwards through the un-emitted residue,
  -- whose every node retains an un-emitted predecessor)
  let mut cycle : Array String := #[]
  if !acyclic then
    let mut pred : Array Nat := Array.replicate nN nN
    for (u, v) in edges do
      if indeg[u]! > 0 && indeg[v]! > 0 then
        pred := pred.set! v u
    let mut cur := 0
    for i in [0:nN] do
      if indeg[i]! > 0 then cur := i
    let mut seen : Std.HashMap Nat Nat := {}
    let mut path : Array Nat := #[]
    let mut looping := true
    for _ in [0:nN + 1] do
      if looping then
        match seen.get? cur with
        | some pos =>
            for v in (path.extract pos path.size).reverse do
              cycle := cycle.push (evStr nodes[v]!)
            looping := false
        | none =>
            if pred[cur]! == nN then
              cycle := cycle.push s!"CYCLE WALK STUCK at {evStr nodes[cur]!}"
              looping := false
            else
              seen := seen.insert cur path.size
              path := path.push cur
              cur := pred[cur]!
  -- monotonicity: per channel-side, depth strictly increasing in seq
  let mut monoErrs : Array String := #[]
  if acyclic then
    let mut groups : Std.HashMap Nat (Array (Nat × Nat)) := {}
    for i in [0:nN] do
      let (c, sd, sq) := nodes[i]!
      let k := chanKey c * 2 + (if sd then 1 else 0)
      groups := groups.insert k ((groups.getD k #[]).push (sq, depth[i]!))
    for (_k, arr) in groups.toList do
      let sorted := arr.qsort (fun a b => a.1 < b.1)
      for j in [0:sorted.size] do
        if j + 1 < sorted.size then
          if sorted[j]!.2 ≥ sorted[j + 1]!.2 then
            -- recover a printable name from any node in the group
            monoErrs := monoErrs.push
              s!"non-monotone: seq {sorted[j]!.1} depth {sorted[j]!.2} vs seq {sorted[j+1]!.1} depth {sorted[j+1]!.2} (group key {_k})"
  let maxDepth := depth.foldl max 0
  return { totalErrs := errs, acyclic, cycle, nodes, depths := depth
           edgeCount := edges.size, maxDepth, monoErrs }

-- ================================================================ main

-- ======================================================= the fuzz sweep

/-- Deterministic LCG (Numerical Recipes constants), 32-bit state. -/
def lcgNext (s : Nat) : Nat := (s * 1664525 + 1013904223) % 4294967296

/-- Draw below `bound` and advance (high bits: low LCG bits alternate). -/
def draw (st bound : Nat) : Nat × Nat :=
  let s := lcgNext st
  ((s / 65536) % bound, s)

/-- Random well-formed skeleton from a seed: BFS-generated, rootH in
{2, 4, 6}, capLevel 1–4, kid counts 0–5, D-kind with probability 2/3,
height-1 D scopes draw leafReqs 0–5. Covers what the pins cannot: mixed
kinds, uneven fans, stalls crossing scope and stage boundaries, and
both sides of the §5 capLevel boundary. -/
def genSkel (seed : Nat) : Skel := Id.run do
  let mut st := seed
  let (rh2, st') := draw st 3
  st := st'
  let rootH := 2 * (rh2 + 1)
  let (cl, st') := draw st 4
  st := st'
  let capLevel := cl + 1
  let maxFan := 5
  let mut scopes : Array Scope := #[]
  let mut nextId := 1
  let (rk, st') := draw st maxFan
  st := st'
  let rootKids := (List.range (rk + 1)).map (· + nextId)
  nextId := nextId + rk + 1
  scopes := scopes.push ⟨Kind.D, rootH, rootKids, 0⟩
  let mut frontier : Array Nat := rootKids.toArray
  let mut h := rootH - 1
  while h ≥ 1 do
    let mut newFrontier : Array Nat := #[]
    for _i in frontier do
      let (kindRoll, st') := draw st 3
      st := st'
      if h == 1 then
        if kindRoll == 0 then
          scopes := scopes.push ⟨Kind.R, 1, [], 0⟩
        else
          let (lr, st') := draw st (maxFan + 1)
          st := st'
          scopes := scopes.push ⟨Kind.D, 1, [], lr⟩
      else
        if kindRoll == 0 then
          scopes := scopes.push ⟨Kind.R, h, [], 0⟩
        else
          let (nk, st') := draw st (maxFan + 1)
          st := st'
          let kids := (List.range nk).map (· + nextId)
          nextId := nextId + nk
          scopes := scopes.push ⟨Kind.D, h, kids, 0⟩
          newFrontier := newFrontier ++ kids.toArray
    frontier := newFrontier
    if h == 1 then break
    h := h - 1
  return { scopes := scopes.toList, rootH, fan := maxFan, capLevel }

/-- The §5 schedulability condition, conjectured ⟺ DAG acyclicity. -/
def schedulable (sk : Skel) : Bool :=
  (List.range sk.scopes.length).all fun i => sk.dCount i ≤ sk.capLevel + 2

/-- The random-skeleton sweep: per seed, (a) acyclicity must equal the
`schedulable` condition (both directions of the §5 conjecture), (b) on
acyclic skeletons the candidate must validate AND replay to terminal.
Returns error lines (empty = clean sweep). -/
def runFuzz (n : Nat) : Array String := Id.run do
  let mut errs : Array String := #[]
  let mut tested := 0
  for seed in [1:n+1] do
    let sk := genSkel seed
    if sk.wellFormed then
      tested := tested + 1
      let a := analyze sk
      if a.acyclic != schedulable sk then
        errs := errs.push
          s!"seed {seed}: acyclic={a.acyclic} but schedulable={schedulable sk}"
      if a.acyclic then
        let cand := schedCandidate sk
        let vErrs := validateSchedule sk cand
        if !vErrs.isEmpty then
          errs := errs.push s!"seed {seed}: candidate invalid ({vErrs.size} errors, first: {vErrs[0]!})"
        else
          let (stuckAt, term) := replaySchedule sk cand
          if let some i := stuckAt then
            errs := errs.push s!"seed {seed}: replay refused at event {i}"
          else if !term then
            errs := errs.push s!"seed {seed}: replay missed terminal"
  if tested == 0 then
    errs := errs.push "fuzz generated zero well-formed skeletons"
  return errs

def skels : List (String × Skel) :=
  [("smokeChain", Pin.smokeChain),
   ("rMix", Pin.rMix),
   ("comb6", Pin.comb6),
   ("pyramid4", Pin.pyramid 4),
   ("pyramid2", Pin.pyramid 2),
   ("jam", Control.jam)]

/-- Channels of the jam trap neighborhood (finding #6). -/
def jamTrapChans : List Chan :=
  [Chan.wire Party.R 2, Chan.asked Party.R 0, Chan.lower Party.R 2]

def dumpLines (a : Analysis) : Array String := Id.run do
  let idx := (Array.range a.nodes.size).qsort fun i j =>
    a.depths[i]! < a.depths[j]! ||
      (a.depths[i]! == a.depths[j]! && evKey a.nodes[i]! < evKey a.nodes[j]!)
  let mut out : Array String := #[]
  for i in idx do
    out := out.push s!"{a.depths[i]!}\t{evStr a.nodes[i]!}"
  return out

def runAll (outDir : System.FilePath) : IO Bool := do
  IO.FS.createDirAll outDir
  let mut table : Array String := #[]
  let mut allOk := true
  for (name, sk) in skels do
    IO.println s!"=== {name} ==="
    if !sk.wellFormed then
      IO.println "  WARNING: skeleton not wellFormed"
      allOk := false
    let a := analyze sk
    let totalsOk := a.totalErrs.isEmpty
    if !totalsOk || !a.acyclic || !a.monoErrs.isEmpty then
      allOk := false
    for e in a.totalErrs do
      IO.println s!"  TOTALS MISMATCH: {e}"
    IO.println s!"  totalsOk={totalsOk} acyclic={a.acyclic} nodes={a.nodes.size} edges={a.edgeCount} maxDepth={a.maxDepth}"
    table := table.push
      s!"{name}\ttotalsOk={totalsOk}\tacyclic={a.acyclic}\tnodes={a.nodes.size}\tedges={a.edgeCount}\tmaxDepth={a.maxDepth}"
    if !a.acyclic then
      IO.println "  CYCLE FOUND (listed along edge direction):"
      for l in a.cycle do
        IO.println s!"    {l}"
      IO.println "  (skipping dump and depth reporting for this skeleton)"
    else
      for e in a.monoErrs do
        IO.println s!"  MONOTONICITY VIOLATION: {e}"
      if a.monoErrs.isEmpty then
        IO.println "  per-channel-side depth strictly monotone in seq: OK"
      let f := outDir / s!"{name}.tsv"
      IO.FS.writeFile f
        (String.intercalate "\n" (dumpLines a).toList ++ "\n")
      IO.println s!"  dump: {f}"
      if name == "jam" then
        IO.println "  --- jam trap neighborhood (finding #6) ---"
        for c in jamTrapChans do
          for i in [0:a.nodes.size] do
            let (c', _sd, _sq) := a.nodes[i]!
            if chanKey c' == chanKey c then
              IO.println s!"    depth {a.depths[i]!}\t{evStr a.nodes[i]!}"
    -- coherence: the greedy run's event trace must be a valid
    -- linearization of the DAG (see greedySchedule) — gate on it.
    let gs := greedySchedule sk
    let gErrs := validateSchedule sk gs
    if !gErrs.isEmpty then
      allOk := false
      for e in gErrs.toSubarray 0 (min gErrs.size 20) do
        IO.println s!"  GREEDY TRACE INVALID: {e}"
    IO.println s!"  greedy trace: {gs.size} events, linearizes DAG: {gErrs.isEmpty}"
    if a.acyclic then
      let f := outDir / s!"{name}.greedy.tsv"
      IO.FS.writeFile f (String.intercalate "\n"
        ((gs.toList.zipIdx.map fun (e, i) => s!"{i}\t{evStr e}")) ++ "\n")
      IO.println s!"  greedy dump: {f}"
    -- the §5 candidate construction: deterministic priority merge
    let cand := schedCandidate sk
    let cErrs := validateSchedule sk cand
    if !cErrs.isEmpty then
      allOk := false
      for e in cErrs.toSubarray 0 (min cErrs.size 20) do
        IO.println s!"  CANDIDATE INVALID: {e}"
    IO.println s!"  candidate schedule: {cand.size} events, valid: {cErrs.isEmpty}"
    -- replay: the candidate must be a genuine model run to terminal
    -- (guards adjudicate every ordering; E3 completeness check)
    let (stuckAt, term) := replaySchedule sk cand
    match stuckAt with
    | some i =>
        allOk := false
        IO.println s!"  REPLAY REFUSED at event {i}: {evStr cand[i]!}"
    | none =>
        if !term then
          allOk := false
          IO.println "  REPLAY did not reach terminal after close drain"
        else
          IO.println "  candidate replays to terminal as a real model run: OK"
    if a.acyclic && cErrs.isEmpty then
      let f := outDir / s!"{name}.cand.tsv"
      IO.FS.writeFile f (String.intercalate "\n"
        ((cand.toList.zipIdx.map fun (e, i) => s!"{i}\t{evStr e}")) ++ "\n")
      IO.println s!"  candidate dump: {f}"
    -- cap-1 experiment: does acyclicity survive every capacity forced
    -- to 1?  Informational only: never affects allOk or the dumps.
    let a1 := analyze sk (capOne := true)
    IO.println s!"  cap1: acyclic={a1.acyclic} maxDepth={a1.maxDepth}"
    if !a1.acyclic then
      IO.println "  cap1 CYCLE FOUND (listed along edge direction):"
      for l in a1.cycle do
        IO.println s!"    {l}"
  -- Self-test: every gate above is one-sided (green when a check finds
  -- nothing), so pin that the checks CAN fail — a dropped edge family
  -- or a hollowed validator flips one of these and the exit code.
  IO.println "=== self-test (negative controls) ==="
  let pyr1 := Pin.pyramid 1
  let aNeg := analyze pyr1
  IO.println s!"  pyramid1 (schedulability boundary): acyclic={aNeg.acyclic} (want false)"
  if aNeg.acyclic then allOk := false
  let cNeg := validateSchedule pyr1 (schedCandidate pyr1)
  IO.println s!"  pyramid1 candidate rejected: {!cNeg.isEmpty} (want true)"
  if cNeg.isEmpty then allOk := false
  -- Mutation controls run on pyramid2: its channels carry multiple
  -- messages (smokeChain's all carry one, leaving E2 nothing to swap).
  let mutSk := Pin.pyramid 2
  let good := schedCandidate mutSk
  -- E1 mutation: swap a message's snd past its rcv.
  let mutated : Option (Array Ev) := Id.run do
    for i in [0:good.size] do
      let (c, sd, n) := good[i]!
      if sd then
        for j in [i+1:good.size] do
          if good[j]! == ((c, false, n) : Ev) then
            return some ((good.set! i good[j]!).set! j good[i]!)
    return none
  match mutated with
  | none =>
      IO.println "  E1 mutation could not be constructed (want a swap): FAIL"
      allOk := false
  | some m =>
      let mNeg := validateSchedule mutSk m
      IO.println s!"  E1-swapped mutation flagged: {!mNeg.isEmpty} (want true)"
      if mNeg.isEmpty then allOk := false
  -- E2 mutation: move a cap-1 channel's snd(n+1) before rcv(n). The
  -- pyramid1 control pins the LEVEL E2 family (its cycle needs it);
  -- this pins the cap-1 family, which no other control is sensitive to
  -- (the merge stalls via sk.cap, not via dagEdges).
  let mutated2 : Option (Array Ev) := Id.run do
    for i in [0:good.size] do
      let (c, sd, n) := good[i]!
      if !sd && mutSk.cap c == 1 then
        for j in [i+1:good.size] do
          if good[j]! == ((c, true, n + 1) : Ev) then
            return some ((good.set! i good[j]!).set! j good[i]!)
    return none
  match mutated2 with
  | none =>
      IO.println "  E2 mutation could not be constructed (want a swap): FAIL"
      allOk := false
  | some m =>
      let m2Neg := validateSchedule mutSk m
      IO.println s!"  E2-swapped mutation flagged: {!m2Neg.isEmpty} (want true)"
      if m2Neg.isEmpty then allOk := false
  IO.println "=== verdict table ==="
  for l in table do
    IO.println l
  return allOk

def runFuzzIO (n : Nat) : IO Bool := do
  IO.println s!"=== fuzz sweep ({n} seeds) ==="
  let errs := runFuzz n
  for e in errs.toSubarray 0 (min errs.size 20) do
    IO.println s!"  FUZZ: {e}"
  IO.println s!"  conjecture + candidate + replay on random skeletons: {if errs.isEmpty then "OK" else "FAILED"}"
  return errs.isEmpty

end EventDag

def main (args : List String) : IO UInt32 := do
  let outDir : System.FilePath := args.headD "eventdag-out"
  let fuzzN := ((args.drop 1).headD "100").toNat!
  let ok ← EventDag.runAll outDir
  let fuzzOk ← EventDag.runFuzzIO fuzzN
  return (if ok && fuzzOk then 0 else 1)
