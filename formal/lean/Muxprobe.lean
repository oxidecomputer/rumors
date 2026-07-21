/-
Muxprobe: the executable evidence tier of the mux campaign
(MUX-ADJUDICATION.md §4, stage-2 track C; not part of the library;
imported by nothing). Runs the REAL mux semantics — the same
`Mux.apply`/`mterminal` definitions the theorems of record quantify
over — across the strategy × skeleton × capacity × interleaving matrix
the Python probe calibrated, and pins the outcomes.

The matrix: {bottomMostReady (the shipped policy), roundRobin (a second
work-conserving entry), demand (the idle-capable demand-order pusher,
π_d run executably)} × {the pinned positives, the margin-0-lifted
Controls shapes, the wedge family at growing provisions} × C ∈ {1,2,4}
× two deterministic interleavings (greedy = the kernel pins' order;
pushfirst = sender-runs-ahead). Expected shape, asserted:

- the smoke positives complete under bottomMostReady at every C;
- the wedge family jams under every work-conserving entry, and each
  member's jam verdict is C-FLAT — the mechanism is slot occupation +
  FIFO burial, not pipe exhaustion (MUX-ADJUDICATION §1.2 point 2);
- the demand-order pusher completes the pinned and wedge families at
  every C ≥ 1 (C₀ = 1 included — §1.3's positive SHAPE, tier 1) but
  WEDGES on the committed `rand2` instance at every C: the π-wedge
  finding (see `piWedge`) — π_d run exactly is NOT the state-feedback
  oracle it was adjudicated as the precomputed form of, and T5 must
  take §1.3's named fallback (or repair π-eligibility);
- along every bottomMostReady cell, every commit consultation is a
  SINGLETON (`commitScan`): the executable echo of `commit_totality`
  (T1), reconciling the Python probe's fused commit+push with this
  harness's adversarial commits (MUX-ADJUDICATION §6 item 6).

The full per-cell table is pinned byte-for-byte in the committed
golden file `muxprobe-expected.tsv` (the eventdag gate pattern: drift
fails loudly); regenerate deliberately with `--update` after a model
or matrix change and review the diff like a snapshot.

H-c lives here and only here (MUX-ADJUDICATION §1.3: the price of
idling is demoted to the executable tier — the model is
message-counted, payload-erased, and latency-free, which erases where
a real cost analysis would live). The step-count ratios printed at the
end are recorded commentary, never an assertion, and no statement of
record may consume them.

Run: lake exe muxprobe [randSeeds] [--update]   (randSeeds defaults to
25 random margin-0 skeletons for the non-golden sweep; exit code is
nonzero on any expectation violation or golden drift, so the `just
all` sweep can gate on it.)
-/
import StreamingMirror

open StreamingMirror
open StreamingMirror.Model
open StreamingMirror.Mux
open StreamingMirror.Mux.Gen

namespace Muxprobe

/-- One matrix cell: coordinates plus the probe verdict. -/
structure Cell where
  skel : String
  strat : String
  cap : Nat
  ord : String
  res : ProbeResult

/-- The cell's golden-file line (stable TSV; commit tallies stay out —
they are asserted, not pinned, so a scan tweak never churns the
snapshot). -/
def Cell.line (c : Cell) : String :=
  s!"{c.skel}\t{c.strat}\tC={c.cap}\t{c.ord}\t{c.res.outcome.str}\t{c.res.steps}"

-- ============================================================ the matrix

/-- The pinned positive shapes: the Instances matrix plus the two
Controls shapes lifted to margin 0 (the probe's `pinned_for_mux`,
minus the wedge family, which gets its own axis below). -/
def pins : List (String × Skel) :=
  [("smokeChain", Pin.smokeChain),
   ("rMix", Pin.rMix),
   ("comb6", Pin.comb6),
   ("pyramid4", Pin.pyramid 4),
   ("jam+m0", liftMargin0 Control.jam),
   ("pdelay+m0", liftMargin0 Control.pdelay)]

/-- The wedge-family provision widths swept (w = 6 is `wedge`, the T3
witness; the probe's minimal deadlocking width was 4, flat in C). -/
def wedgeWidths : List Nat := [1, 2, 3, 4, 6, 8]

/-- The wedge-family axis: growing provisions at rootH 6, plus the
larger `regression8` shape (w = 8 at rootH 8). -/
def wedges : List (String × Skel) :=
  wedgeWidths.map (fun w => (s!"wedge w={w}", wedgeFam w)) ++
    [("wedge8x8", wedgeFam 8 8)]

/-- THE π-WEDGE (found by this probe's random sweep, 2026-07-21; the
stage-0 P2 gate MUX-ADJUDICATION §4 scheduled and no tier had run): a
random margin-0 skeleton on which the demand-order pusher — π_d run
exactly, not the state-feedback proxy — deadlocks at every C and every
tested interleaving, while the state-feedback ('exit'-certificate) σ*
completes it.

Cross-confirmed in the Python probe's independent transcription
(identical stuck point, 17 + 29 frames pushed, C- and
interleaving-flat). Mechanism at the stuck state: walk (R,0) HOLDS a
committed provision the absorber is waiting for, but π_R schedules a
`wire R 2` frame first; that frame's producer walk (R,2) is parked on
a query into the full cap-1 `asked R 0` channel, which only drains
after the very frame π_R is waiting to see produced. The precomputed
order demands a frame the run can no longer produce first — the
feedback the 'exit' certificate reads per-state is load-bearing, so
per MUX-ADJUDICATION §1.3's named fallback, T5's oracle of record must
be the state-feedback form (or π-eligibility must be repaired), NOT
`ofSchedule (demandOrder …)` as drafted. Pinned here so the finding
cannot silently dissolve. -/
def piWedge : List (String × Skel) := [("rand2", genSkelM0 2)]

/-- Every skeleton in the golden matrix. -/
def matrixSkels : List (String × Skel) := pins ++ wedges ++ piWedge

/-- The capacity axis. -/
def caps : List Nat := [1, 2, 4]

/-- The strategy axis for one skeleton: name, the pair, and whether
the entry is work-conserving (the jam expectations quantify over the
work-conserving entries only). `demand` closes over the precomputed
π_d projections so a consultation is one count and one index. -/
def strategiesFor (sk : Skel) : List (String × Strategy × Strategy × Bool) :=
  [("bottom", bottomMostReady, bottomMostReady, true),
   ("rr", roundRobin, roundRobin, true),
   ("demand", pushList (piOrder sk .I), pushList (piOrder sk .R),
    false)]

/-- The interleaving axis for one skeleton. -/
def ordersFor (sk : Skel) : List (String × List MAction) :=
  [("greedy", orderGreedy sk), ("pushfirst", orderPushFirst sk)]

/-- Fuel for one skeleton's runs, derived from its total event count
(the probe's `6·ops + 400` idiom; a `fuel` outcome is always an
expectation violation, so an undersized budget fails loudly). -/
def fuelFor (sk : Skel) : Nat := 8 * (Sched.scheduleE sk).length + 400

/-- Run the full golden matrix. Commit scanning rides every
`bottomMostReady` cell — the task's pinned-matrix obligation — and
costs one `commitScan` per visited state there. -/
def runMatrix : Array Cell := Id.run do
  let mut out : Array Cell := #[]
  for (name, sk) in matrixSkels do
    let fuel := fuelFor sk
    for (sname, σI, σR, _wc) in strategiesFor sk do
      for c in caps do
        for (oname, order) in ordersFor sk do
          let r := runProbe sk .impl c σI σR order fuel (sname == "bottom")
          out := out.push ⟨name, sname, c, oname, r⟩
  return out

-- ====================================================== expectations

/-- Did this (skeleton, strategy, C) jam — i.e. did ANY tested
interleaving stick? (The probe's deadlock convention.) -/
def jams (cells : Array Cell) (skel strat : String) (c : Nat) : Bool :=
  cells.any fun x =>
    x.skel == skel && x.strat == strat && x.cap == c &&
      x.res.outcome == .stuck

/-- The expectation suite over the computed matrix: every violation is
one line, an empty result is a clean pass. The suite is the structural
claim; the golden file additionally pins the exact steps and verdicts
cell-for-cell. -/
def expectations (cells : Array Cell) : Array String := Id.run do
  let mut errs : Array String := #[]
  -- hypothesis class: every matrix skeleton is inside the class the
  -- statements of record quantify over (wellFormed + margin-0, hence
  -- schedulable), so every jam below indicts the mux alone
  for (name, sk) in matrixSkels do
    if !(sk.wellFormed && Mux.margin0 sk && sk.schedulable) then
      errs := errs.push s!"{name}: outside the margin-0 hypothesis class"
  -- family/witness coherence: the parameterized family reproduces the
  -- committed T0 witness literal at its calibration point
  if !(decide ((wedgeFam 6).scopes = wedge.scopes) &&
       (wedgeFam 6).rootH == wedge.rootH &&
       (wedgeFam 6).fan == wedge.fan &&
       (wedgeFam 6).capLevel == wedge.capLevel) then
    errs := errs.push "wedgeFam 6 diverges from the wedge witness literal"
  -- no cell may exhaust fuel: every run must decide terminal-or-stuck
  for x in cells do
    if x.res.outcome == .fuel then
      errs := errs.push s!"fuel exhausted: {x.line}"
  -- smoke positives complete under the shipped policy at every C and
  -- interleaving (the faithfulness half: the mux is fine off-wedge)
  for (name, _) in pins do
    for x in cells do
      if x.skel == name && x.strat == "bottom" &&
          x.res.outcome != .terminal then
        errs := errs.push s!"pin should complete under bottom: {x.line}"
  -- the wedge family jams under EVERY work-conserving entry at every
  -- capacity from width 4 up, and completes below it: the C1-WC shape
  -- (T3) with the probe's minimal deadlocking width (4, C-flat)
  -- reproduced on the real semantics, boundary pinned from both sides
  for w in wedgeWidths do
    for strat in ["bottom", "rr"] do
      for c in caps do
        if w ≥ 4 && !jams cells s!"wedge w={w}" strat c then
          errs := errs.push
            s!"wedge w={w} should jam under {strat} at C={c}"
        if w < 4 && jams cells s!"wedge w={w}" strat c then
          errs := errs.push
            s!"wedge w={w} should complete under {strat} at C={c} (minimal width moved)"
  for strat in ["bottom", "rr"] do
    for c in caps do
      if !jams cells "wedge8x8" strat c then
        errs := errs.push s!"wedge8x8 should jam under {strat} at C={c}"
  -- C-flatness of the jam verdicts: slot occupation + FIFO burial, not
  -- pipe exhaustion — capacity never rescues (or dooms) a member
  for (name, _) in wedges do
    for strat in ["bottom", "rr"] do
      let verdicts := caps.map (jams cells name strat)
      if !verdicts.all (· == verdicts.headD false) then
        errs := errs.push s!"{name} under {strat}: jam verdict not C-flat"
  -- the idle-capable entry completes the pinned and wedge families at
  -- every C ≥ 1 (C₀ = 1 included): the C2-positive SHAPE, tier-1 — but
  -- NOT a universal liveness pin, see the π-wedge below
  for x in cells do
    if x.strat == "demand" && x.skel != "rand2" &&
        x.res.outcome != .terminal then
      errs := errs.push s!"demand pusher should complete: {x.line}"
  -- the π-wedge: the demand-order pusher must KEEP deadlocking on the
  -- committed rand2 instance at every capacity (C-flat by the same
  -- loop) — the P2 finding stays load-bearing; if a model change makes
  -- this complete, π-eligibility has been repaired and T5's oracle
  -- choice needs a deliberate re-adjudication, not a silent pass
  for c in caps do
    if !jams cells "rand2" "demand" c then
      errs := errs.push
        s!"rand2 should wedge the demand-order pusher at C={c} (P2 finding dissolved: re-adjudicate T5)"
  -- commit totality, executably: along every scanned cell each commit
  -- consultation was a singleton, and the matrix witnessed plenty
  let mut consults := 0
  for x in cells do
    if x.res.multi > 0 then
      errs := errs.push
        s!"commit consultation with a CHOICE ({x.res.multi} states): {x.line}"
    consults := consults + x.res.consults
  if consults == 0 then
    errs := errs.push "commit scan is vacuous: zero consultations observed"
  return errs

-- ==================================================== the random sweep

/-- The non-golden random sweep: per seed a margin-0 skeleton
(`genSkelM0`, wellFormed-filtered). Per skeleton: the shipped policy's
run — terminal or stuck, both legitimate off the pinned families —
must keep every commit consultation singleton, and the demand-order
pusher at C ∈ {1, 2} must DECIDE (no fuel exhaustion; both verdicts
are legitimate since the π-wedge finding, and the wedging seeds are
counted). The finding itself must keep reproducing: a sweep of ≥ 2
seeds that finds zero demand wedges means a model or generator change
dissolved it and needs a deliberate re-audit (the `EventDag.runFuzz`
advStalls posture). Returns (errors, tested, bottomJams, demandJams). -/
def runRandom (n : Nat) : Array String × Nat × Nat × Nat := Id.run do
  let mut errs : Array String := #[]
  let mut tested := 0
  let mut bottomJams := 0
  let mut demandJams := 0
  for seed in [1:n+1] do
    let sk := genSkelM0 seed
    if sk.wellFormed then
      tested := tested + 1
      let fuel := fuelFor sk
      let rb := runProbe sk .impl 1 bottomMostReady bottomMostReady
        (orderGreedy sk) fuel true
      if rb.multi > 0 then
        errs := errs.push
          s!"seed {seed}: commit consultation with a choice under bottom"
      if rb.outcome == .fuel then
        errs := errs.push s!"seed {seed}: bottom C=1 exhausted fuel"
      if rb.outcome == .stuck then
        bottomJams := bottomJams + 1
      let σI := pushList (piOrder sk .I)
      let σR := pushList (piOrder sk .R)
      let mut wedged := false
      for c in [1, 2] do
        let rd := runProbe sk .impl c σI σR (orderGreedy sk) fuel false
        if rd.outcome == .fuel then
          errs := errs.push s!"seed {seed}: demand pusher exhausted fuel at C={c}"
        if rd.outcome == .stuck then
          wedged := true
      if wedged then
        demandJams := demandJams + 1
  if tested == 0 then
    errs := errs.push "random sweep generated zero well-formed skeletons"
  if n ≥ 2 && demandJams == 0 then
    errs := errs.push
      "random sweep found ZERO demand-pusher wedges (the π-wedge finding should reproduce; a model or generator change needs a deliberate re-audit)"
  return (errs, tested, bottomJams, demandJams)

-- ======================================================= golden file

/-- Compare (or, under `update`, rewrite) the committed golden matrix.
Line-for-line equality; any drift is printed and fails the run — the
eventdag gate pattern, with the snapshot half in a reviewable file. -/
def goldenGate (path : System.FilePath) (lines : Array String)
    (update : Bool) : IO Bool := do
  let body := String.intercalate "\n" lines.toList ++ "\n"
  if update then
    IO.FS.writeFile path body
    IO.println s!"  golden file rewritten: {path} ({lines.size} rows)"
    return true
  if !(← path.pathExists) then
    IO.println s!"  GOLDEN FILE MISSING: {path} (run with --update to create)"
    return false
  let want := ((← IO.FS.readFile path).splitOn "\n").filter (· != "")
  let got := lines.toList
  if want == got then
    IO.println s!"  golden matrix: {lines.size} rows match {path}"
    return true
  IO.println s!"  GOLDEN DRIFT against {path}:"
  let mut shown := 0
  for i in [0:max want.length got.length] do
    if shown < 20 && want[i]? != got[i]? then
      IO.println s!"    row {i}: expected {want[i]?.getD "<missing>"}"
      IO.println s!"    row {i}:      got {got[i]?.getD "<missing>"}"
      shown := shown + 1
  return false

-- ============================================================== main

/-- Informational H-c commentary: rounds-to-terminal (the parallel-time
proxy, `Gen.roundsBase`/`Gen.roundsMux`) of the unmuxed baseline
against the shipped policy and the demand-order pusher at C = 1 on the
pinned positives — the locus of the probe's 0.99× observation. Step
counts are useless here (a completed run's total op count is
strategy-invariant); rounds expose lockstep stalls. The model is
message-counted, payload-erased, and latency-free — it erases where a
real cost analysis lives (MUX-ADJUDICATION §1.3, H-c demoted to this
tier) — so this is recorded for the log, asserted nowhere, and no
statement of record may consume it. -/
def hcCommentary : IO Unit := do
  IO.println "  H-c commentary (informational; rounds-to-terminal, C=1):"
  for (name, sk) in pins do
    let fuel := fuelFor sk
    let (bo, base) := roundsBase sk .impl fuel
    let (mo, bottom) := roundsMux sk .impl 1 bottomMostReady bottomMostReady fuel
    let σI := pushList (piOrder sk .I)
    let σR := pushList (piOrder sk .R)
    let (dmo, demand) := roundsMux sk .impl 1 σI σR fuel
    IO.println
      s!"    {name}: base {base} ({bo.str}), bottom-mux {bottom} ({mo.str}), demand-mux {demand} ({dmo.str})"

def run (randN : Nat) (update : Bool) : IO UInt32 := do
  IO.println "=== muxprobe: the executable mux matrix (.impl, margin-0) ==="
  let cells := runMatrix
  let lines := cells.map (·.line)
  let mut ok := true
  -- expectations first, so a broken matrix never silently reaches the
  -- golden gate
  let errs := expectations cells
  for e in errs do
    IO.println s!"  EXPECTATION: {e}"
  IO.println s!"  structural expectations: {if errs.isEmpty then "OK" else "FAILED"} ({cells.size} cells)"
  if !errs.isEmpty then ok := false
  if !(← goldenGate "muxprobe-expected.tsv" lines update) then ok := false
  let consults := cells.foldl (fun a x => a + x.res.consults) 0
  IO.println s!"  commit consultations observed (all singleton): {consults}"
  IO.println s!"=== random sweep ({randN} seeds) ==="
  let (rerrs, tested, bottomJams, demandJams) := runRandom randN
  for e in rerrs.toSubarray 0 (min rerrs.size 20) do
    IO.println s!"  RANDOM: {e}"
  IO.println s!"  {tested} well-formed margin-0 skeletons; bottom C=1 jams: {bottomJams}; demand-pusher wedges (C∈1,2): {demandJams}; sweep: {if rerrs.isEmpty then "OK" else "FAILED"}"
  if !rerrs.isEmpty then ok := false
  hcCommentary
  IO.println s!"=== verdict: {if ok then "OK" else "FAILED"} ==="
  return (if ok then 0 else 1)

end Muxprobe

def main (args : List String) : IO UInt32 := do
  let update := args.contains "--update"
  let randN := ((args.filter (· != "--update")).headD "25").toNat!
  Muxprobe.run randN update
