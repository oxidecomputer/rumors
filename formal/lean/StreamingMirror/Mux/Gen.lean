/-
Executable-tier generation and probe machinery for the mux harness
(executable tier only). Everything here serves
`lake exe muxprobe`; nothing here is theorem-bearing, and nothing a
theorem of record quantifies over may live here (the statement-path
strategies and classes stay in Mux/{Basic,Strategy,Instances}.lean).

Three kinds of content:

- **skeleton families** — the parameterized `wedgeFam` (the Python
  probe's `regression_shape`: the committed regression witness at
  growing provision widths, `wedgeFam 6 = wedge` pinned by the exe's
  self-test), the margin-0 lift for the pinned Controls shapes, and a
  deterministic LCG skeleton generator mirroring `EventDag.genSkel`'s
  approach but emitting margin-0 skeletons directly (the mux statements
  of record live on the margin-0 class).
- **executable-tier strategies** — `roundRobin` (the probe's `rr`
  policy: a second work-conserving entry, deliberately NOT in the
  theorem modules) and `pushList`/`piOrder` (the demand-order
  pusher run executably: pushes π_d in order,
  idles otherwise — the idle-capable matrix entry and the executable
  forerunner of T5's `ofSchedule (demandOrder …)`).
- **the probe runners** — a step-counted drain over an explicit action
  order (the deterministic interleaving is part of each matrix cell's
  identity) with an optional per-state commit-consultation scan, the
  executable echo of `commit_totality` (T1): the Python probe fused
  walkCommit+push while this harness keeps commits adversarial
  (the probe-reconciliation obligation), and the scan checks the fact that
  reconciles them — at every reachable state each walk has at most one
  choosable obligation, so the fusion was WLOG. Plus the rounds
  runners (`roundsBase`/`roundsMux`), the parallel-time proxy behind
  the H-c commentary — informational tier only.
-/
import StreamingMirror.Mux.Instances
import StreamingMirror.Proofs.Sched

namespace StreamingMirror.Mux.Gen

open Model

-- ==================================================== the wedge family

/-- The regression shape at provision width `w` (the Python probe's
`regression_shape(provisions := w, rootH)`): the root disputes its
FIRST radix child — a chain descending disputed levels to a leaf
request — and takes `w` whole-subtree provisions behind it on the same
stream.

`wedgeFam 6 = wedge` (the T0 witness literal; the exe's self-test pins
the equality), and the family at growing `w` is where the minimal
deadlocking width and its C-flatness — the slot-occupation mechanism,
not pipe exhaustion — are measured.
Margin-0 by construction (every scope disputes at most one child), so
each member is inside the base flagship's kernel-proven class. -/
def wedgeFam (w : Nat) (rootH : Nat := 6) : Skel :=
  let chainTop := w + 2
  let chain := (List.range (rootH - 2)).map fun k =>
    let h := rootH - 2 - k
    if h == 1 then Pin.sc .D 1 [] (leafReqs := 1)
    else Pin.sc .D h [chainTop + k + 1]
  { scopes :=
      Pin.sc .D rootH ((List.range (w + 1)).map (· + 1))
        :: Pin.sc .D (rootH - 1) [chainTop]
        :: (List.range w).map (fun _ => Pin.sc .R (rootH - 1) [])
        ++ chain
    rootH, fan := w + 1, capLevel := 1 }

-- ====================================================== margin-0 lift

/-- Max per-scope dispute count: the margin-0 capacity reference
(`EventDag.maxDCount`, mirrored). -/
def maxDCount (sk : Skel) : Nat :=
  (List.range sk.scopes.length).foldl (fun m s => max m (sk.dCount s)) 0

/-- The margin-0 capacity variant: `capLevel` raised to the max
per-scope dispute count, topology unchanged (the probe's
`lift_margin0`, `EventDag.margin0` mirrored).

This is how the sub-margin Controls shapes (`jam`, `pdelay`) enter the
mux matrix: their topologies stress the mux, but the statements of
record need the UN-muxed `.impl` system inside the kernel-proven
margin-0 class, so a muxed jam indicts the mux alone. -/
def liftMargin0 (sk : Skel) : Skel :=
  { sk with capLevel := max sk.capLevel (maxDCount sk) }

-- ============================================== the random family

/-- Deterministic LCG (Numerical Recipes constants), 32-bit state —
`EventDag.lcgNext`, mirrored so the probes stay independent. -/
def lcgNext (s : Nat) : Nat := (s * 1664525 + 1013904223) % 4294967296

/-- Draw below `bound` and advance (high bits: low LCG bits alternate). -/
def draw (st bound : Nat) : Nat × Nat :=
  let s := lcgNext st
  ((s / 65536) % bound, s)

/-- Random margin-0 skeleton from a seed: BFS-generated in the
`EventDag.genSkel` idiom (rootH in {2, 4, 6}, kid counts 0–7, D-kind
with probability 2/3, height-1 D scopes draw leafReqs 0–7), then
`liftMargin0` in place of a drawn capLevel — every emitted skeleton
sits in the hypothesis class of the mux statements of record.

Callers filter on `wellFormed` exactly as `EventDag.runFuzz` does; the
generator is deterministic per seed, so a sweep is reproducible from
its seed range alone. -/
def genSkelM0 (seed : Nat) : Skel := Id.run do
  let mut st := seed
  let (rh2, st') := draw st 3
  st := st'
  let rootH := 2 * (rh2 + 1)
  let maxFan := 7
  let mut scopes : Array Scope := #[]
  let mut nextId := 1
  let (rk, st') := draw st maxFan
  st := st'
  let rootKids := (List.range (rk + 1)).map (· + nextId)
  nextId := nextId + rk + 1
  scopes := scopes.push ⟨.D, rootH, rootKids, 0⟩
  let mut frontier : Array Nat := rootKids.toArray
  for off in [0:rootH - 1] do
    let h := rootH - 1 - off
    let mut newFrontier : Array Nat := #[]
    for _i in frontier do
      let (kindRoll, st') := draw st 3
      st := st'
      if kindRoll == 0 then
        scopes := scopes.push ⟨.R, h, [], 0⟩
      else if h == 1 then
        let (lr, st') := draw st (maxFan + 1)
        st := st'
        scopes := scopes.push ⟨.D, 1, [], lr⟩
      else
        let (nk, st') := draw st (maxFan + 1)
        st := st'
        let kids := (List.range nk).map (· + nextId)
        nextId := nextId + nk
        scopes := scopes.push ⟨.D, h, kids, 0⟩
        newFrontier := newFrontier ++ kids.toArray
    frontier := newFrontier
  return liftMargin0 { scopes := scopes.toList, rootH, fan := maxFan, capLevel := 1 }

-- ======================================== executable-tier strategies

/-- Round-robin over stream heights: the probe's `rr` policy, a second
work-conserving matrix entry.

Among the streams whose hand is committed (reconstructed from the
observation history exactly as `bottomMostReady` does), pick the first
at or above the height after the last push, wrapping to the lowest.
Work-conserving — whenever any hand is committed it names a held
stream — and deliberately NOT in the theorem modules: T3 quantifies
over the whole `WorkConserving` class, and this entry exists to
exercise a second member executably, not to carry a statement. -/
def roundRobin : Strategy := fun sk tr =>
  let ready := (List.range (sk.rootH + 1)).filter (committedInHist sk.rootH tr)
  let next := (tr.foldl (fun acc o =>
    match o with | .pushed h => some (h + 1) | _ => acc)
    (none : Option Nat)).getD 0
  ((ready.find? fun h => decide (next ≤ h)).orElse fun _ => ready.head?)

/-- Push a fixed frame list in order, idling between: entry `k` names
the stream of the machine's `k`-th push, so the strategy is a pure
function of the push count in its own history.

The idle-capable matrix entry: when the next listed frame is not yet
the committed hand, the strategy idles (the `push` guard fails on an
unheld stream) — exactly the right to idle that separates it from the
work-conserving class (the right to idle). -/
def pushList (frames : List Nat) : Strategy := fun _ tr =>
  frames[tr.countP fun o =>
    match o with | .pushed _ => true | _ => false]?

/-- Direction `d`'s wire frames in the receiver's consumption order:
the receive events on `d`'s wire channels, projected from the `.impl`
canonical schedule (π_d, run executably).

`pushList (piOrder sk d)` is the demand-order pusher — T5's
`ofSchedule (demandOrder …)` run exactly, not the state-feedback
proxy. Matrix-tier evidence (the muxprobe golden matrix, C ∈ {1, 2, 4};
the kernel form is `static_oracle_jams` at C = 1): it completes the
pinned and wedge families at every matrix capacity, but it is NOT
deadlock-free on the margin-0 class — the committed `rand2` instance
wedges it at every matrix capacity (the π-wedge finding,
cross-confirmed in the Python probe — see `Muxprobe.piWedge`), so the
"precomputed form of the state-feedback oracle" reading of §1.3 is
executably false and T5 takes the adjudication's fallback slot. The
projection stays here, in the executable tier, precisely because it is
a refuted candidate: the matrix keeps both its positive shape and its
wedge pinned.

Named `piOrder` (π_d as a frame list): the theorem-bearing definition
is `demandOrder` (Oracle/Controls.lean, where `static_oracle_jams`
consumes it) and carries the adjudication's vocabulary; the two are
one definition (`piOrder_eq_demandOrder`, Mux/Proofs/Twins.lean — the
drift guard), and Muxprobe opens both namespaces. -/
def piOrder (sk : Skel) (d : Party) : List Nat :=
  (Sched.scheduleE sk).filterMap fun e =>
    match e with
    | (.wire p h, false, _) => if p == d then some h else none
    | _ => none

-- ============================================== deterministic orders

/-- The greedy interleaving: `allMActions` order (base arms, then the
four mux moves) — the order `mdrain` and the kernel smoke pins use, so
a muxprobe cell at this order replays the pinned drains exactly. -/
def orderGreedy (sk : Skel) : List MAction := allMActions sk

/-- The sender-runs-ahead interleaving: pushes before everything,
deliveries last — the probe's `push_first`, the flush-paced sender
racing the consumers. Fills the pipe at the earliest opportunity, so
it is the burial-mechanism-facing order of the two. -/
def orderPushFirst (sk : Skel) : List MAction :=
  [.push .I, .push .R] ++ (Model.allActions sk).map .base ++
    [.deliver .I, .deliver .R]

-- ==================================================== the probe run

/-- One matrix cell's verdict. -/
inductive Outcome
  | terminal | stuck | fuel
  deriving DecidableEq, Repr

/-- Stable TSV token for an outcome. -/
def Outcome.str : Outcome → String
  | .terminal => "terminal"
  | .stuck => "stuck"
  | .fuel => "fuel"

/-- One probe run's result: the verdict, the step count, and the
commit-consultation tallies (zero unless the scan was requested). -/
structure ProbeResult where
  outcome : Outcome
  steps : Nat
  consults : Nat
  multi : Nat

/-- The walk's publication obligations, enumerated to the fan bound —
the commit alphabet `allActions` ranges over. -/
def obligs (sk : Skel) : List Oblig :=
  ((List.range sk.fan).flatMap fun i => [.wire i, .res i, .query i]) ++
    [.parent]

/-- Count each walk's choosable obligations at one state: the number of
`singleton` walks (exactly one choosable) and of violating walks (two
or more) — the executable echo of `commit_totality` (T1).

`wkChoosable` is exactly the `walkCommit` guard for a key in
`walkKeys` (it refuses non-phase-2 and already-committed walks itself),
so counting it over the commit alphabet counts the enabled
`walkCommit` arms without paying `Model.apply`'s state rebuild. A
violation means a commit consultation with a genuine choice — the
Python probe's commit+push fusion would NOT have been WLOG, and the
`.impl` forced-order claim is executably false. -/
def commitScan (sk : Skel) (ax : AxMode) (s : MState) : Nat × Nat :=
  sk.walkKeys.foldl (init := (0, 0)) fun acc pk =>
    let ws := s.base.walk pk
    let n := (obligs sk).countP fun o => wkChoosable sk ax pk ws o
    if n == 1 then (acc.1 + 1, acc.2)
    else if n > 1 then (acc.1, acc.2 + 1)
    else acc

/-- Step-counted drain over an explicit action order: fire the first
enabled action until terminal, stuck, or fuel exhaustion, optionally
running `commitScan` at every visited state.

This is `mdrain` with the interleaving reified (the order is part of
the cell's identity in the muxprobe matrix) and the bookkeeping the
golden file pins. Enabledness is order-independent, so `stuck` here
coincides with `mstuck` — an order can only change WHICH run is
taken, never whether the reached fixpoint counts as stuck. -/
def runProbe (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy)
    (order : List MAction) (fuel : Nat) (scanCommits : Bool) :
    ProbeResult := Id.run do
  let mut s := Mux.init sk
  let mut consults := 0
  let mut multi := 0
  for n in [0:fuel] do
    if scanCommits then
      let (c1, cm) := commitScan sk ax s
      consults := consults + c1
      multi := multi + cm
    if mterminal sk s then
      return ⟨.terminal, n, consults, multi⟩
    match order.firstM (fun a => Mux.apply sk ax C σI σR a s) with
    | some s' => s := s'
    | none => return ⟨.stuck, n, consults, multi⟩
  return ⟨.fuel, fuel, consults, multi⟩

-- ============================================== rounds (parallel time)

/-- The base model's processes, each with its action alphabet in a
fixed internal order — the probe's `rounds.py` agent decomposition,
mirrored: openers, absorber, the two finales, the walks, the
assemblers. -/
def agentsBase (sk : Skel) : List (List Action) :=
  [[.iopenChoose .wire, .iopenChoose .query, .iopenFire],
   [.ropenRecv, .ropenChoose .wire, .ropenChoose .res,
    .ropenChoose .query, .ropenFire],
   [.absorbRecvWire, .absorbRecvAsked, .absorbSend, .absorbCloseWire,
    .absorbCloseAsked],
   [.finRet], [.finRes, .finRets]] ++
  sk.walkKeys.map (fun pk =>
    [.walkRecvWire pk, .walkRecvAsked pk, .walkFire pk,
     .walkCloseWire pk, .walkCloseAsked pk, .walkCommit pk .parent] ++
    (List.range sk.fan).flatMap fun i =>
      [.walkCommit pk (.wire i), .walkCommit pk (.res i),
       .walkCommit pk (.query i)]) ++
  sk.asmKeys.map (fun pk => [.asmRecvRes pk, .asmRecvLevel pk,
    .asmSend pk, .asmClose pk])

/-- Rounds-to-terminal of the UN-muxed base model: per round each
process fires at most one enabled action, in fixed order; the round
count is the parallel-time proxy the H-c commentary compares against
(the probe's `rounds_run` with `C=None`). -/
def roundsBase (sk : Skel) (ax : AxMode) (fuel : Nat) : Outcome × Nat := Id.run do
  let agents := agentsBase sk
  let mut s := Model.init sk
  for rnd in [0:fuel] do
    if Model.terminal sk s then
      return (.terminal, rnd)
    let mut fired := false
    for acts in agents do
      match acts.firstM (fun a => Model.apply sk ax a s) with
      | some s' => s := s'; fired := true
      | none => pure ()
    if !fired then
      return (.stuck, rnd)
  return (.fuel, fuel)

/-- Rounds-to-terminal of the muxed system under a strategy pair: the
base agents plus four mux agents — the two σ-gated pushers and the two
demuxes, placed FIRST in the round order so a frame committed in round
r is pushed no earlier than round r + 1 and delivered a round after
that. The convention charges the mux its store-and-forward hop
(generous to the baseline, so a mux ≤ base reading is conservative);
`rounds.py` charged the same two rounds through its fused-composite
`busy` accounting. Informational tier only: H-c consumes these, no
statement of record does. -/
def roundsMux (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy)
    (fuel : Nat) : Outcome × Nat := Id.run do
  let agents : List (List MAction) :=
    [[.push .I], [.push .R], [.deliver .I], [.deliver .R]] ++
      (agentsBase sk).map (·.map .base)
  let mut s := Mux.init sk
  for rnd in [0:fuel] do
    if mterminal sk s then
      return (.terminal, rnd)
    let mut fired := false
    for acts in agents do
      match acts.firstM (fun a => Mux.apply sk ax C σI σR a s) with
      | some s' => s := s'; fired := true
      | none => pure ()
    if !fired then
      return (.stuck, rnd)
  return (.fuel, fuel)

end StreamingMirror.Mux.Gen
