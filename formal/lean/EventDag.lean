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

Run: lake exe eventdag [outDir]   (dumps default to ./eventdag-out,
which is gitignored; exit code is nonzero on any failed check, so the
`just all` sweep can gate on it.)
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
The parent send otherwise floats (only rcvA and d2 constrain it). -/
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
    for i in [0:n] do
      let wEv : Ev := (wireOut pk, true, wireCnt)
      wireCnt := wireCnt + 1
      wireEvs := wireEvs.push wEv
      sends := sends.push wEv
      if sk.childIsD h s i then
        let rEv : Ev := (lowerOut pk, true, resCnt)
        resCnt := resCnt + 1
        sends := sends.push rEv
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
    sends := sends.push parentEv
    -- in-order wires; d4: D block i complete before wire(i+1)
    for i in [0:n] do
      if i + 1 < n then
        edges := edges.push (wireEvs[i]!, wireEvs[i+1]!)
        match lastOfBlock[i]! with
        | some lastEv => edges := edges.push (lastEv, wireEvs[i+1]!)
        | none => pure ()
    -- in-order D-resolution prefix; d2: res(i) → parent
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

def analyze (sk : Skel) (fuel : Nat := 50000) : Analysis := Id.run do
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
  -- empirical: greedy run to terminal
  let fin := drainFull sk fuel (init sk)
  if !(terminal sk fin) then
    errs := errs.push "greedy run did NOT reach terminal"
  let chans := chanList sk
  let mut chanKeys : Std.HashSet Nat := {}
  for c in chans do
    chanKeys := chanKeys.insert (chanKey c)
  for (c, _sd, _sq) in nodes do
    if !chanKeys.contains (chanKey c) then
      errs := errs.push s!"event channel {chanStr c} not in allChans"
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
  let mut allEdges := procEdges
  for c in chans do
    let t := sndCnt.getD (chanKey c) 0
    let cap := sk.cap c
    for n in [0:t] do
      allEdges := allEdges.push ((c, true, n), (c, false, n))
      if n + cap < t then
        allEdges := allEdges.push ((c, false, n), (c, true, n + cap))
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
  IO.println "=== verdict table ==="
  for l in table do
    IO.println l
  return allOk

end EventDag

def main (args : List String) : IO UInt32 := do
  let outDir : System.FilePath := args.headD "eventdag-out"
  let ok ← EventDag.runAll outDir
  return (if ok then 0 else 1)
