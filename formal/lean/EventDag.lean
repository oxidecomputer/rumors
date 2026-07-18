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
The §5 candidate itself lives here too (`schedCandidate`, the
deterministic priority merge), gated by `validateSchedule`,
`replaySchedule` (the schedule re-run as real model actions to
`terminal`), the random-skeleton sweep (`runFuzz`, which also pins the
`Skel.schedulable` ⟺ acyclic conjecture — the predicate itself lives in
Skel.lean, on the statement layer's audit surface), and the
self-testing negative controls at the end of `runAll`.
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

/-- Injective Nat key for a channel (heights < 1000 assumed; a
violation aliases two channels and fails loud as a duplicate-node /
duplicate-event error in both the analyzer and the validator). -/
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

/-- `allActions` with each walk's floating-parent commit moved AFTER its
child-obligation commits: the parent-delaying adversary. The default
enumeration lists `.parent` first, so the greedy drain sends every
summary at its earliest legal point and never visits a
committed-past-unsent-parent state; this ordering makes the driver
commit a choosable wire/res/query first whenever one exists, steering
every run through exactly those states.

THE PARENT-DELAY FINDING (2026-07-17), now resolved by the `d5` ledger:
under the pre-`d5` interface (`Control.fullNoD5`, what `.full` was at
the time) this drain STALLS on schedulable random seeds (first witness:
seed 12 — two walks committed past unsent parents close a
commit/back-pressure cycle through the level towers); the minimized
kernel-checked twin is `Control.parentTrap`. Under today's `.full` the
`d5` guard forces the parent at its weave position even against this
ordering, so the drain must TERMINATE. `runFuzz` pins both directions:
stalls reproduce under `fullNoD5`, and every schedulable seed drains to
terminal under `.full`. Details: PROGRESS.md §7 item 5. -/
def advActions (sk : Skel) : List Action :=
  [.iopenChoose .wire, .iopenChoose .query, .iopenFire,
   .ropenRecv, .ropenChoose .wire, .ropenChoose .res, .ropenChoose .query,
   .ropenFire,
   .absorbRecvWire, .absorbRecvAsked, .absorbSend, .absorbCloseWire,
   .absorbCloseAsked, .finRet, .finRes, .finRets] ++
  sk.walkKeys.flatMap (fun pk =>
    [.walkRecvWire pk, .walkRecvAsked pk, .walkFire pk,
     .walkCloseWire pk, .walkCloseAsked pk] ++
    ((List.range sk.fan).flatMap fun i =>
      [.walkCommit pk (.wire i), .walkCommit pk (.res i),
       .walkCommit pk (.query i)]) ++
    [.walkCommit pk .parent]) ++
  sk.asmKeys.flatMap (fun pk =>
    [.asmRecvRes pk, .asmRecvLevel pk, .asmSend pk, .asmClose pk])

/-- Greedy drain over `advActions`: the parent-delaying adversarial run
under axiom mode `ax` (stalls under `Control.fullNoD5`, terminates
under `.full` — see `advActions`). Under `.impl` the `d6` guard FORCES
the delayed placement, so any `.impl` drain — stalling or not — is an
epilogue-legal run by construction: every step passed every `.impl`
guard. -/
def drainAdv (sk : Skel) (ax : AxMode) : Nat → State → State
  | 0, s => s
  | fuel + 1, s =>
      match (advActions sk).firstM (fun a => apply sk ax a s) with
      | some s' => drainAdv sk ax fuel s'
      | none => s

/-- Max per-scope dispute count: the margin-0 capacity reference. -/
def maxDCount (sk : Skel) : Nat :=
  (List.range sk.scopes.length).foldl (fun m s => max m (sk.dCount s)) 0

/-- The margin-0 capacity variant: `capLevel` raised to the max
per-scope dispute count (the encoder's `FAN ≥ kids` discipline),
topology unchanged. Well-formedness is preserved (`capLevel` only
grows past its `≥ 1` floor) and `schedulable` holds by construction
(`dCount ≤ capLevel` everywhere). This is the implementation-facing
theorem's capacity hypothesis (PLAN.md task #16), exercised executably
by the `.impl`-mode drains below. -/
def margin0 (sk : Skel) : Skel :=
  { sk with capLevel := max sk.capLevel (maxDCount sk) }

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
{2, 4, 6}, capLevel 1–4, kid counts 0–7, D-kind with probability 2/3,
height-1 D scopes draw leafReqs 0–7. Covers what the pins cannot: mixed
kinds, uneven fans, stalls crossing scope and stage boundaries, and —
the fan cap (7) exceeding capLevel + 3 at every drawn capLevel — both
sides of the §5 boundary. The deterministic per-capLevel exactness
matrix (including capLevels past this generator's range) is `runAll`'s
boundary self-test, via `boundaryProbe`. -/
def genSkel (seed : Nat) : Skel := Id.run do
  let mut st := seed
  let (rh2, st') := draw st 3
  st := st'
  let rootH := 2 * (rh2 + 1)
  let (cl, st') := draw st 4
  st := st'
  let capLevel := cl + 1
  let maxFan := 7
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

/-- Canonical per-channel numbering of the proof layer's trace family
(PROGRESS.md §7 item 3a), checked BEFORE the Lean lemmas are written —
validate-then-prove applied to the numbering layer's own statements.
Three claims, per (channel, side): (a) each trace of `Sched.procs`
projects to consecutive seqs from 0 (canon shape); (b) at most one
trace projects nonempty (one producer, one consumer per channel);
(c) the consumer form — the merged schedule's own projection is canon,
i.e. the n-th send (receive) on every channel carries seq n. -/
def numberingErrs (sk : Skel) : Array String := Id.run do
  let procs := (Sched.procs sk).toArray
  let mut errs : Array String := #[]
  -- (chanKey, side) → (first producing trace, seqs in trace order)
  let mut tbl : Std.HashMap (Nat × Bool) (Nat × Chan × Array Nat) := {}
  for i in [0:procs.size] do
    for (c, sd, sq) in procs[i]! do
      let key := (chanKey c, sd)
      match tbl.get? key with
      | none => tbl := tbl.insert key (i, c, #[sq])
      | some (j, c₀, seqs) =>
          if j != i then
            errs := errs.push
              s!"numbering: {chanStr c} side={sd}: traces {j} and {i} both project"
          tbl := tbl.insert key (j, c₀, seqs.push sq)
  for (_, (i, c, seqs)) in tbl.toList do
    for t in [0:seqs.size] do
      if seqs[t]! != t then
        errs := errs.push
          s!"numbering: {chanStr c} (trace {i}): seq {seqs[t]!} at projection index {t} (want {t})"
  -- consumer form, on the merged schedule
  let mut next : Std.HashMap (Nat × Bool) Nat := {}
  for (c, sd, sq) in Sched.schedule sk do
    let key := (chanKey c, sd)
    let want := next.getD key 0
    if sq != want then
      errs := errs.push
        s!"numbering: schedule {chanStr c} side={sd}: seq {sq} arrives {want}-th"
    next := next.insert key (want + 1)
  return errs

-- ================================ §7 3b: weak potential + blame probes

/-- The minimal *weak potential* φ (§7 3b): longest path in the event
DAG where message (E1) and back-pressure (E2) edges weigh 1 and
trace-order (E3-linearization) edges weigh 0. φ is then E1/E2-STRICT
and trace-WEAK — exactly what the completeness argmin consumes: at a
stalled state, a blocked head's blame target holds an earlier-φ head,
contradicting minimality. Exists iff the DAG is acyclic (`none` on
cycles); as the pointwise-least valid potential it is the mining
surface for a closed form. Returns (`evKey → φ`, `evKey → critical
in-edge`) — the second map names the parent that achieved each
event's max (with its weight), so the `.phi.tsv` dump reads as the
critical tree rather than bare numbers. -/
def weakPotential (sk : Skel) :
    Option (Std.HashMap Nat Nat × Std.HashMap Nat (Ev × Nat)) := Id.run do
  let procs := (Sched.procs sk).toArray.map List.toArray
  let mut nodes : Array Ev := #[]
  for t in procs do
    for e in t do nodes := nodes.push e
  let nN := nodes.size
  let mut idOf : Std.HashMap Nat Nat := {}
  for i in [0:nN] do
    idOf := idOf.insert (evKey nodes[i]!) i
  -- weighted edges: (u, v, w)
  let mut edges : Array (Nat × Nat × Nat) := #[]
  for t in procs do
    for i in [0:t.size] do
      if i + 1 < t.size then
        edges := edges.push
          (idOf.getD (evKey t[i]!) 0, idOf.getD (evKey t[i+1]!) 0, 0)
  -- E1/E2 at weight 1, generated per channel as in `dagEdges`
  let mut sndCnt : Std.HashMap Nat (Chan × Nat) := {}
  for (c, sd, _sq) in nodes do
    if sd then
      let k := chanKey c
      sndCnt := sndCnt.insert k (c, (sndCnt.getD k (c, 0)).2 + 1)
  for (_, (c, tot)) in sndCnt.toList do
    for n in [0:tot] do
      match idOf.get? (evKey (c, true, n)), idOf.get? (evKey (c, false, n)) with
      | some u, some v =>
          edges := edges.push (u, v, 1)
          if n + sk.cap c < tot then
            edges := edges.push (v, idOf.getD (evKey (c, true, n + sk.cap c)) 0, 1)
      | _, _ => pure ()  -- unpaired message: the totals gate reports it
  -- weighted Kahn
  let mut indeg : Array Nat := Array.replicate nN 0
  let mut adj : Array (Array (Nat × Nat)) := Array.replicate nN #[]
  for (u, v, w) in edges do
    indeg := indeg.set! v (indeg[v]! + 1)
    adj := adj.set! u (adj[u]!.push (v, w))
  let mut phi : Array Nat := Array.replicate nN 0
  -- provenance: the in-edge that achieved each event's max, as
  -- (parent id, weight); nN = no parent (a source)
  let mut critP : Array (Nat × Nat) := Array.replicate nN (nN, 0)
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
      for (v, w) in adj[u]! do
        if phi[u]! + w > phi[v]! || critP[v]!.1 == nN then
          phi := phi.set! v (max phi[v]! (phi[u]! + w))
          critP := critP.set! v (u, w)
        indeg := indeg.set! v (indeg[v]! - 1)
        if indeg[v]! == 0 then queue := queue.push v
  if emitted != nN then
    return none
  let mut out : Std.HashMap Nat Nat := {}
  for i in [0:nN] do
    out := out.insert (evKey nodes[i]!) phi[i]!
  let mut crit : Std.HashMap Nat (Ev × Nat) := {}
  for i in [0:nN] do
    let (p, w) := critP[i]!
    if p != nN then
      crit := crit.insert (evKey nodes[i]!) (nodes[p]!, w)
  return some (out, crit)

/-- One line per event, sorted by φ then event key, with the critical
in-edge that forced the value (`<- w=1 [parent]`; sources bare): the
`.phi.tsv` mining dump for the §7 3b closed form. Empty when the DAG
is cyclic. -/
def phiDump (sk : Skel) : Array String := Id.run do
  match weakPotential sk with
  | none => return #[]
  | some (phi, crit) =>
      let mut evs : Array (Nat × Ev) := #[]
      for t in Sched.procs sk do
        for e in t do
          evs := evs.push (phi.getD (evKey e) 0, e)
      let sorted := evs.qsort fun a b =>
        a.1 < b.1 || (a.1 == b.1 && evKey a.2 < evKey b.2)
      return sorted.map fun (p, e) =>
        match crit.get? (evKey e) with
        | some (par, w) => s!"{p}\t{evStr e}\t<- w={w} [{evStr par}]"
        | none => s!"{p}\t{evStr e}"

-- ================== §7 3b: the tree-recursive weave (the phi witness)

/-- Weave state: the emitted order, per-channel counts, and cursors
into the linear pump traces (absorb, the asm towers, float, fin).
`errs` records any weave emission whose E1/E2 guard does not hold at
its position — the weave is CONSTRUCTED to be valid, and a violation
here means the interleave design is wrong at that point. -/
structure WV where
  out : Array Ev
  sent : Std.HashMap Nat Nat
  rcvd : Std.HashMap Nat Nat
  pumps : Array (Array Ev)
  pumpCur : Array Nat
  errs : Array String

/-- Is `e` emittable now (E1 for receives, E2 cap window for sends)? -/
def WV.ok (st : WV) (sk : Skel) (e : Ev) : Bool :=
  let (c, sd, n) := e
  if sd then n < st.rcvd.getD (chanKey c) 0 + sk.cap c
  else n < st.sent.getD (chanKey c) 0

/-- Emit one event, recording an error if its guard is closed. -/
def WV.emit (st : WV) (sk : Skel) (e : Ev) : WV :=
  let (c, sd, _n) := e
  let st := if st.ok sk e then st else
    { st with errs := st.errs.push s!"weave emitted disabled [{evStr e}]" }
  if sd then
    { st with out := st.out.push e
              sent := st.sent.insert (chanKey c) (st.sent.getD (chanKey c) 0 + 1) }
  else
    { st with out := st.out.push e
              rcvd := st.rcvd.insert (chanKey c) (st.rcvd.getD (chanKey c) 0 + 1) }

/-- Greedily drain the pump traces: repeatedly emit the FIRST enabled
head in priority order, until none moves — `mergeN` restricted to the
pump traces, exactly the proof layer's `wPump`. Pump emissions only
raise counts, so greedy pumping is confluent. -/
def WV.pump (st : WV) (sk : Skel) : WV := Id.run do
  let mut st := st
  let fuel := (st.pumps.map (·.size)).foldl (· + ·) 0
  for _ in [0:fuel + 1] do
    let mut fired := false
    for i in [0:st.pumps.size] do
      if !fired then
        let t := st.pumps[i]!
        let c := st.pumpCur[i]!
        if c < t.size && st.ok sk t[c]! then
          st := { st.emit sk t[c]! with pumpCur := st.pumpCur.set! i (c + 1) }
          fired := true
    if !fired then
      return st
  return st

/-- Emit then pump: every weave emission may open assembly windows. -/
def WV.emitP (st : WV) (sk : Skel) (e : Ev) : WV :=
  (st.emit sk e).pump sk

/-- The descent weave for scope `k` of stage `h` (§7 3b): prologue,
then per kid — wire, resolution (D kids), the parent summary after
the last resolution (or first, when nothing disputes), the FEED query
for that kid, and the recursive descent, with this scope's own chunk
queries passed down as the kid's feed. `feed[i]` is the query event
for kid `i`, owned by this scope's parent's trace: emitting it here,
one per kid in order, is exactly what the cap-1 asked channel's E2
requires, and it preserves the parent's trace order (all of a chunk's
queries precede the next chunk's wire). At `h = 0` the kids are leaf
slots: the feed is the leaf-request and the absorb trace is pumped in
place of a recursive call. -/
partial def weaveScope (sk : Skel) (h k : Nat) (feed : Array Ev) (st : WV) : WV := Id.run do
  let pk : Party × Nat := (if h % 2 == 1 then Party.I else Party.R, h)
  let mut st := st
  st := st.emitP sk (wireIn pk, false, k)
  st := st.emitP sk (askedIn pk, false, k)
  let s := sk.stageScope h k
  let n := sk.nChildren h s
  let lastD := ((List.range n).filter (fun i => sk.childIsD h s i)).getLast?
  if lastD == none then
    st := st.emitP sk (upperOut pk, true, k)
  let kidBase := (List.range k).foldl
    (fun a k' => a + sk.nChildren h (sk.stageScope h k')) 0
  let mut dSeen := 0
  let mut qSeen := 0
  for i in [0:n] do
    st := st.emitP sk (wireOut pk, true, sk.wiresBefore h k + i)
    if sk.childIsD h s i then
      st := st.emitP sk (lowerOut pk, true, sk.dsBefore h k + dSeen)
      dSeen := dSeen + 1
      if lastD == some i then
        st := st.emitP sk (upperOut pk, true, k)
      if h == 0 then
        -- leaf slots dispute nothing; childIsD is hard-false at h = 0
        st := { st with errs := st.errs.push s!"weave: D kid at h=0 (scope {k})" }
      else
        let myQ := (Array.range (sk.qCount h s i)).map fun t =>
          ((askedOut pk, true, sk.qsBefore h k + qSeen + t) : Ev)
        if i < feed.size then
          st := st.emitP sk feed[i]!
        st := weaveScope sk (h - 1) (kidBase + i) myQ st
    else
      if i < feed.size then
        st := st.emitP sk feed[i]!
      if h ≥ 1 then
        st := weaveScope sk (h - 1) (kidBase + i) #[] st
    -- childChunk's query base sums qCount over ALL earlier kids
    qSeen := qSeen + sk.qCount h s i
  return st

/-- The §7 3b witness: a FULL topological order of the event DAG
(E1/E2/E3-trace), built by structural recursion over the scope tree —
openers, then the root scope's weave, then a final pump. Its position
function is the potential the completeness argmin consumes: strict
across every edge family (stronger than the weak potential needs).
Validated by the same `validateSchedule` as the merge candidate; on a
non-schedulable skeleton the weave emits through closed guards and is
rejected (`pyramid 1` pins this). NOT the schedule: τ and the blame
lemmas stay with the merge — the weave only witnesses that a valid
completion exists, which is where `Skel.schedulable` will enter the
Lean proof (the pump-progress lemmas at each emission point). -/
def weaveOrder (sk : Skel) : Array Ev × Array String := Id.run do
  let pumps : Array (Array Ev) :=
    #[(Sched.absorbEvents sk).toArray]
    ++ (sk.asmKeys.toArray.map fun pk => (Sched.asmEvents sk pk).toArray)
    ++ #[#[(Chan.rootret, false, 0)], (Sched.finEvents sk).toArray]
  let mut st : WV :=
    { out := #[], sent := {}, rcvd := {}, pumps
      pumpCur := Array.replicate pumps.size 0, errs := #[] }
  for e in Sched.iopenEvents sk do
    st := st.emitP sk e
  for e in (Sched.ropenEvents sk).take 3 do
    st := st.emitP sk e
  let rootFeed := ((Sched.ropenEvents sk).drop 3).toArray
  st := weaveScope sk (sk.rootH - 1) 0 rootFeed st
  st := st.pump sk
  return (st.out, st.errs)

/-- Trace labels in `Sched.procs` order, for the blame alphabet. -/
def traceLabels (sk : Skel) : Array String :=
  #["iopen", "ropen"]
  ++ ((List.range sk.rootH).map fun i =>
        let h := sk.rootH - 1 - i
        s!"walk{pStr (if h % 2 == 1 then Party.I else Party.R)}{h}").toArray
  ++ #["absorb"]
  ++ (sk.asmKeys.map fun pk => s!"asm{pStr pk.1}{pk.2}").toArray
  ++ #["float", "fin"]

/-- §7 3b instrumentation (validate-then-prove applied to the
completeness invariant): replay the merge; at EVERY state, for every
non-empty trace with a disabled head, derive the blame edge — the
blocking event is the earliest unemitted event on the blocking
channel-side (`snd(c, sent c)` for an E1-starved receive,
`rcv(c, rcvd c)` for an E2-jammed send), its owner the unique trace
holding it in its remaining suffix — and check executably what the
Lean proof will assert: (a) the blocker exists and its owner is
unique (ownership + totals), (b) the weak potential strictly drops
from the blocked head to the owner's head (the argmin step), (c) blame
chains reach an enabled head without revisiting a trace. Returns
(errors, alphabet); the alphabet aggregates observed blame edges with
counts — the §7 3b mining surface for WHERE `schedulable` binds. On a
cyclic skeleton the merge stalls and (c) reports the blame cycle: the
`pyramid 1` negative control pins that this probe can fail. -/
def blameProbe (sk : Skel) : Array String × Array String := Id.run do
  let procs := (Sched.procs sk).toArray.map List.toArray
  let labels := traceLabels sk
  let phi? := (weakPotential sk).map (·.1)
  let mut cur := Array.replicate procs.size 0
  let mut sent : Std.HashMap Nat Nat := {}
  let mut rcvd : Std.HashMap Nat Nat := {}
  let mut errs : Array String := #[]
  let mut alpha : Std.HashMap String Nat := {}
  let fuel := (procs.map (·.size)).foldl (· + ·) 0
  for _step in [0:fuel + 1] do
    -- enabledness of every head, blame edges of the disabled ones
    let enabledAt : Nat → Option Bool := fun i => Id.run do
      if cur[i]! < procs[i]!.size then
        let (c, sd, n) := procs[i]![cur[i]!]!
        if sd then return some (decide (n < rcvd.getD (chanKey c) 0 + sk.cap c))
        else return some (decide (n < sent.getD (chanKey c) 0))
      else return none
    let blameOf := fun (i : Nat) => Id.run do
      -- pre: head of i exists and is disabled
      let (c, sd, _n) := procs[i]![cur[i]!]!
      let blocker : Ev :=
        if sd then (c, false, rcvd.getD (chanKey c) 0)
        else (c, true, sent.getD (chanKey c) 0)
      let mut owners : Array Nat := #[]
      for j in [0:procs.size] do
        for idx in [cur[j]!:procs[j]!.size] do
          if procs[j]![idx]! == blocker then
            owners := owners.push j
      return (blocker, owners)
    let mut anyEnabled := false
    let mut anyNonEmpty := false
    for i in [0:procs.size] do
      match enabledAt i with
      | none => pure ()
      | some true => anyNonEmpty := true; anyEnabled := true
      | some false =>
          anyNonEmpty := true
          let head := procs[i]![cur[i]!]!
          let (blocker, owners) := blameOf i
          if owners.size != 1 then
            errs := errs.push
              s!"blame: {labels[i]!} head [{evStr head}] blocker [{evStr blocker}]: {owners.size} owners"
          else
            let j := owners[0]!
            let key := s!"{labels[i]!} [{evStr head}] blames {labels[j]!} [{evStr blocker}]"
            alpha := alpha.insert key (alpha.getD key 0 + 1)
            if let some phi := phi? then
              let pHead := phi.getD (evKey head) 0
              let pOwnerHead := phi.getD (evKey procs[j]![cur[j]!]!) 0
              if pOwnerHead ≥ pHead then
                errs := errs.push
                  s!"blame: phi does not drop: {labels[i]!} [{evStr head}] phi={pHead} -> {labels[j]!} head phi={pOwnerHead}"
    -- chain check from every disabled head
    for i0 in [0:procs.size] do
      if enabledAt i0 == some false then
        let mut visited : Array Nat := #[i0]
        let mut curT := i0
        let mut walking := true
        for _hop in [0:procs.size + 1] do
          if walking then
            match enabledAt curT with
            | some true => walking := false
            | none =>
                errs := errs.push s!"blame chain from {labels[i0]!} hit drained trace {labels[curT]!}"
                walking := false
            | some false =>
                let (_, owners) := blameOf curT
                if owners.size != 1 then
                  walking := false  -- already reported above
                else
                  let nxt := owners[0]!
                  if visited.contains nxt then
                    errs := errs.push
                      (s!"blame CYCLE from {labels[i0]!}: "
                        ++ String.intercalate " -> " ((visited.push nxt).toList.map (labels[·]!)))
                    walking := false
                  else
                    visited := visited.push nxt
                    curT := nxt
    -- one merge step: first enabled head, in priority order
    let mut fired := false
    for i in [0:procs.size] do
      if !fired && enabledAt i == some true then
        fired := true
        let (c, sd, _n) := procs[i]![cur[i]!]!
        cur := cur.set! i (cur[i]! + 1)
        if sd then sent := sent.insert (chanKey c) (sent.getD (chanKey c) 0 + 1)
        else rcvd := rcvd.insert (chanKey c) (rcvd.getD (chanKey c) 0 + 1)
    if !fired then
      if anyNonEmpty && !anyEnabled then
        errs := errs.push "blame: merge STALLED (expected on cyclic skeletons only)"
      break
  let alphaLines := (alpha.toArray.qsort fun a b => a.1 < b.1).map
    fun (k, n) => s!"{n}\t{k}"
  return (errs, alphaLines)

/-- The random-skeleton sweep: per seed, (a) acyclicity must equal the
`Skel.schedulable` condition (both directions of the §5 conjecture),
(b) on acyclic skeletons the candidate must validate AND replay to
terminal. Returns error lines (empty = clean sweep). -/
def runFuzz (n : Nat) : Array String := Id.run do
  let mut errs : Array String := #[]
  let mut tested := 0
  let mut advStalls : Array Nat := #[]
  let mut implStalls : Array Nat := #[]
  for seed in [1:n+1] do
    let sk := genSkel seed
    if sk.wellFormed then
      tested := tested + 1
      let a := analyze sk
      if a.acyclic != sk.schedulable then
        errs := errs.push
          s!"seed {seed}: acyclic={a.acyclic} but schedulable={sk.schedulable}"
      -- greedy + channel-total cross-checks must hold wherever a drain
      -- can terminate (non-schedulable seeds stall under every driver)
      if sk.schedulable && !a.totalErrs.isEmpty then
        errs := errs.push s!"seed {seed}: {a.totalErrs[0]!}"
      -- the parent-delay finding, pinned in both directions (see
      -- PROGRESS.md §7 item 5): under the pre-d5 ledger set the
      -- adversary reaches genuinely stuck states on SOME schedulable
      -- seeds (the ≥ 1 assertion below keeps the finding from silently
      -- dissolving under a model or generator change), while under
      -- today's `.full` the d5 guard must defuse every such stall —
      -- an adversarial stall under `.full` is a hard error again.
      if sk.schedulable && !(terminal sk
          (drainAdv sk Control.fullNoD5 50000 (init sk))) then
        advStalls := advStalls.push seed
      if sk.schedulable && !(terminal sk
          (drainAdv sk .full 50000 (init sk))) then
        errs := errs.push
          s!"seed {seed}: adversarial drain STALLS under .full (d5 should defuse the parent delay)"
      -- the impl (epilogue) mode, both directions: at the seed's own
      -- capLevel the epilogue order may stall (those stalls are what
      -- the capacity hypothesis exists for — counted below, ≥ 1
      -- required so the hypothesis stays load-bearing), while at
      -- margin 0 (capLevel ≥ max per-scope dCount) a stall falsifies
      -- the re-targeted theorem's hypothesis and is a hard error
      -- (PLAN.md task #15's falsifiable check).
      if sk.schedulable && !(terminal sk
          (drainAdv sk .impl 50000 (init sk))) then
        implStalls := implStalls.push seed
      if !(terminal (margin0 sk)
          (drainAdv (margin0 sk) .impl 50000 (init (margin0 sk)))) then
        errs := errs.push
          s!"seed {seed}: adversarial drain STALLS under .impl at margin 0 (the capacity hypothesis should defuse the epilogue parent delay)"
      if a.acyclic then
        let cand := schedCandidate sk
        let vErrs := validateSchedule sk cand
        if !vErrs.isEmpty then
          errs := errs.push s!"seed {seed}: candidate invalid ({vErrs.size} errors, first: {vErrs[0]!})"
        else if Sched.schedule sk != cand.toList then
          errs := errs.push s!"seed {seed}: Proofs/Sched.lean transcription diverges from candidate"
        else if !(numberingErrs sk).isEmpty then
          errs := errs.push s!"seed {seed}: {(numberingErrs sk)[0]!}"
        else
          let bErrs := (blameProbe sk).1
          let (weave, wErrs0) := weaveOrder sk
          let wErrs := wErrs0 ++ validateSchedule sk weave
          if !bErrs.isEmpty then
            errs := errs.push s!"seed {seed}: {bErrs[0]!}"
          else if !wErrs.isEmpty then
            errs := errs.push s!"seed {seed}: weave invalid ({wErrs.size} errors, first: {wErrs[0]!})"
          else if Sched.weave sk != weave.toList then
            errs := errs.push s!"seed {seed}: Weave.lean transcription diverges from the tool weave"
          else
            let (stuckAt, term) := replaySchedule sk cand
            if let some i := stuckAt then
              errs := errs.push s!"seed {seed}: replay refused at event {i}"
            else if !term then
              errs := errs.push s!"seed {seed}: replay missed terminal"
  if tested == 0 then
    errs := errs.push "fuzz generated zero well-formed skeletons"
  if advStalls.isEmpty then
    errs := errs.push
      "adversarial probe found ZERO stalls on schedulable seeds (the parent-delay finding should reproduce; a model or generator change needs a deliberate re-audit)"
  if implStalls.isEmpty then
    errs := errs.push
      "impl-mode probe found ZERO sub-margin stalls on schedulable seeds (the capacity hypothesis should be load-bearing, not vacuous; a model or generator change needs a deliberate re-audit)"
  return errs

def skels : List (String × Skel) :=
  [("smokeChain", Pin.smokeChain),
   ("rMix", Pin.rMix),
   ("comb6", Pin.comb6),
   ("pyramid4", Pin.pyramid 4),
   ("pyramid2", Pin.pyramid 2),
   ("jam", Control.jam)]

/-- capLevel-parametric boundary probe: a lone parent at height 3
disputing `d` childless D scopes. `d = capLevel + 2` sits ON the
schedulability boundary, `d = capLevel + 3` minimally past it — shapes
`genSkel`'s fan cap cannot always reach, so `runAll`'s boundary matrix
pins the §5 conjecture's exactness deterministically per capLevel. -/
def boundaryProbe (capLevel d : Nat) : Skel :=
  { scopes := ⟨Kind.D, 4, [1], 0⟩
      :: ⟨Kind.D, 3, (List.range d).map (· + 2), 0⟩
      :: (List.range d).map (fun _ => ⟨Kind.D, 2, [], 0⟩)
    rootH := 4, fan := max d 1, capLevel := capLevel }

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
    -- the six pins complete under the parent-delaying adversary in BOTH
    -- modes: their shapes never wedge the trap even pre-d5 (the stalls
    -- need random/parentTrap shapes; see runFuzz and Controls.lean)
    if !(terminal sk (drainAdv sk .full 50000 (init sk))) then
      IO.println "  ADVERSARIAL (parent-delayed) drain did NOT reach terminal under .full"
      allOk := false
    else if !(terminal sk (drainAdv sk Control.fullNoD5 50000 (init sk))) then
      IO.println "  ADVERSARIAL (parent-delayed) drain did NOT reach terminal under fullNoD5"
      allOk := false
    else
      IO.println "  adversarial (parent-delayed) drain reaches terminal: OK"
    -- the impl (epilogue) mode at margin 0: the re-targeted theorem's
    -- hypothesis, on the pinned shapes
    if !(terminal (margin0 sk)
        (drainAdv (margin0 sk) .impl 50000 (init (margin0 sk)))) then
      IO.println "  IMPL-MODE margin-0 adversarial drain did NOT reach terminal"
      allOk := false
    else
      IO.println "  impl-mode margin-0 adversarial drain reaches terminal: OK"
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
    -- transcription coherence: the proof layer's fold-and-fuel form
    -- (Proofs/Sched.lean) must reproduce the candidate exactly
    let schedF := Sched.schedule sk
    if schedF != cand.toList then
      allOk := false
      match (schedF.zip cand.toList).findIdx? (fun (a, b) => a != b) with
      | some i => IO.println s!"  TRANSCRIPTION DIVERGES at index {i}: Sched {evStr (schedF[i]!)} vs candidate {evStr cand[i]!}"
      | none => IO.println s!"  TRANSCRIPTION DIVERGES in length: Sched {schedF.length} vs candidate {cand.size}"
    else
      IO.println "  Proofs/Sched.lean transcription matches the candidate: OK"
    -- numbering: canon per-channel projections, one producer/consumer
    -- per channel, on the traces and on the merged schedule (§7 3a)
    let nErrs := numberingErrs sk
    if !nErrs.isEmpty then
      allOk := false
      for e in nErrs.toSubarray 0 (min nErrs.size 20) do
        IO.println s!"  NUMBERING: {e}"
    else
      IO.println "  per-channel canonical numbering (traces + schedule): OK"
    -- §7 3b: the tree-recursive weave must be a full valid
    -- linearization (permutation + every E1/E2/E3 edge), independent
    -- of the merge — it is the potential witness for completeness
    let (weave, wErrs) := weaveOrder sk
    let wvErrs := wErrs ++ validateSchedule sk weave
    if !wvErrs.isEmpty then
      allOk := false
      for e in wvErrs.toSubarray 0 (min wvErrs.size 20) do
        IO.println s!"  WEAVE INVALID: {e}"
    IO.println s!"  weave order: {weave.size} events, valid: {wvErrs.isEmpty}"
    -- transcription coherence: the proof layer's structural recursion
    -- (Proofs/Sched/Weave.lean) must reproduce the weave exactly
    if Sched.weave sk != weave.toList then
      allOk := false
      match ((Sched.weave sk).zip weave.toList).findIdx? (fun (a, b) => a != b) with
      | some i => IO.println s!"  WEAVE TRANSCRIPTION DIVERGES at index {i}"
      | none => IO.println s!"  WEAVE TRANSCRIPTION DIVERGES in length: {(Sched.weave sk).length} vs {weave.size}"
    else
      IO.println "  Proofs/Sched/Weave.lean transcription matches the weave: OK"
    if a.acyclic && wvErrs.isEmpty then
      let f := outDir / s!"{name}.weave.tsv"
      IO.FS.writeFile f (String.intercalate "\n"
        ((weave.toList.zipIdx.map fun (e, i) => s!"{i}\t{evStr e}")) ++ "\n")
      IO.println s!"  weave dump: {f}"
    -- §7 3b: weak potential + blame reduction, checked at every merge
    -- state; alphabet + phi dumps are the closed-form mining surface
    let (bErrs, bAlpha) := blameProbe sk
    if !bErrs.isEmpty then
      allOk := false
      for e in bErrs.toSubarray 0 (min bErrs.size 20) do
        IO.println s!"  BLAME: {e}"
    else
      IO.println "  blame reduction (owner unique, phi drops, chains terminate): OK"
    if a.acyclic then
      let f := outDir / s!"{name}.phi.tsv"
      IO.FS.writeFile f
        (String.intercalate "\n" (phiDump sk).toList ++ "\n")
      let g := outDir / s!"{name}.blame.tsv"
      IO.FS.writeFile g
        (String.intercalate "\n" bAlpha.toList ++ "\n")
      IO.println s!"  phi dump: {f}  blame alphabet: {g}"
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
  -- the blame probe must FIND the cycle at pyramid1's stall (and the
  -- weak potential must not exist): pins that §7 3b's probe can fail
  let (bNeg, _) := blameProbe pyr1
  let cycleFound := bNeg.any (·.startsWith "blame CYCLE")
  IO.println s!"  pyramid1 blame probe finds a cycle: {cycleFound} (want true)"
  if !cycleFound then allOk := false
  IO.println s!"  pyramid1 weak potential absent: {(weakPotential pyr1).isNone} (want true)"
  if (weakPotential pyr1).isSome then allOk := false
  -- the adversarial drain must ALSO stall there (any driver stalls on a
  -- non-schedulable skeleton): pins that the parent-delayed probe is a
  -- real run that can fail, not a vacuous pass
  let advNegTerm := terminal pyr1 (drainAdv pyr1 .full 50000 (init pyr1))
  IO.println s!"  pyramid1 adversarial drain stuck: {!advNegTerm} (want true)"
  if advNegTerm then allOk := false
  -- finding #7 under the impl (epilogue) mode, both directions: the
  -- raw pdelay boundary (dCount = capLevel + 2) stalls — and because
  -- `d6` forces the delayed placement, that stalling run is
  -- epilogue-LEGAL by construction, settling PLAN.md #15(5a): the −2
  -- floor fails adversarially even for the encoder's own per-walk
  -- order, so the tight floor is poll-schedule-specific and the
  -- theorem hypothesis is margin 0. Margin 0 defuses the same shape.
  let pdE := Control.pdelay
  let pdStuck := drainAdv pdE .impl 50000 (init pdE)
  let pdStall := !(terminal pdE pdStuck)
  IO.println s!"  pdelay adversarial drain under .impl stuck: {pdStall} (want true)"
  if !pdStall then allOk := false
  let pd0 := margin0 pdE
  let pd0Term := terminal pd0 (drainAdv pd0 .impl 50000 (init pd0))
  IO.println s!"  pdelay margin-0 drain under .impl terminal: {pd0Term} (want true)"
  if !pd0Term then allOk := false
  -- borrowed-slots accounting (design/parent-placement.md §2, PLAN.md
  -- #15(5b)), read off pdelay's stuck state: the +2 the schedulable
  -- bound allows over capLevel is one item in each hand — the level
  -- buffers full (the channel proper), consumers mid-collection
  -- (asm got > 0), and producers parked on committed sends.
  let lvlOcc := (chanList pdE).foldl (fun acc c =>
    match c with | .level _ _ => acc + pdStuck.chan c | _ => acc) 0
  let asmHold := pdE.asmKeys.foldl (fun acc pk =>
    acc + (if (pdStuck.asm pk).got > 0 then 1 else 0)) 0
  let committedN := pdE.walkKeys.foldl (fun acc pk =>
    acc + (if (pdStuck.walk pk).committed.isSome then 1 else 0)) 0
  IO.println s!"  pdelay .impl stuck-state accounting (informational): level occupancy={lvlOcc} asms mid-collection={asmHold} walks committed={committedN}"
  -- the weave must be rejected on a non-schedulable skeleton: its
  -- guards close and the emission errors / validator flag it
  let (wNeg, wNegErrs) := weaveOrder pyr1
  let wNegAll := wNegErrs ++ validateSchedule pyr1 wNeg
  IO.println s!"  pyramid1 weave rejected: {!wNegAll.isEmpty} (want true)"
  if wNegAll.isEmpty then allOk := false
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
  -- Boundary matrix: the §5 conjecture's exactness per capLevel, both
  -- sides, on shapes the fuzz envelope cannot always reach. ON the
  -- boundary: acyclic, schedulable, candidate valid, replays to
  -- terminal, transcription matches; one past: cyclic, not schedulable.
  IO.println "  boundary matrix (dCount = capLevel+2 completes, +3 jams):"
  for cl in [1, 2, 3, 4, 6] do
    let onB := boundaryProbe cl (cl + 2)
    let over := boundaryProbe cl (cl + 3)
    let aOn := analyze onB
    let aOver := analyze over
    let cand := schedCandidate onB
    let (stuckAt, term) := replaySchedule onB cand
    let (wOn, wOnErrs) := weaveOrder onB
    let (wOver, wOverErrs) := weaveOrder over
    let ok := onB.wellFormed && over.wellFormed
      && aOn.totalErrs.isEmpty
      && aOn.acyclic && onB.schedulable
      && !aOver.acyclic && !over.schedulable
      && (validateSchedule onB cand).isEmpty
      && stuckAt.isNone && term
      && (Sched.schedule onB == cand.toList)
      -- the weave completes ON the boundary and fails one past it
      && (wOnErrs ++ validateSchedule onB wOn).isEmpty
      && !(wOverErrs ++ validateSchedule over wOver).isEmpty
      -- d5 defuses the parent-delaying adversary on the boundary shapes
      && terminal onB (drainAdv onB .full 50000 (init onB))
      -- and margin 0 defuses it under the impl (epilogue) mode
      && terminal (margin0 onB)
          (drainAdv (margin0 onB) .impl 50000 (init (margin0 onB)))
    IO.println s!"    capLevel={cl}: {if ok then "OK" else "FAIL"}"
    if !ok then allOk := false
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
