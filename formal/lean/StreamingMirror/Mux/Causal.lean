/-
σ*-causal: the charter-grain local demand-lockstep strategy — the Lean
counterpart of the stage-0 probe's faulting view (causal-reference.py;
counterpart, not twin — see the probe-relationship section below),
discharging the definitional half of the σ*-locality residue
(MUX-PROGRESS §4, "The locality residue adjudicated by Finch"; F3
ruling: claims of record bind the charter grain).

# The grain (Finch's F3 ruling, phase 4)

The legacy `LocalEq`/`LocalStrategy` (Mux/Strategy.lean) compares
session-start VIEWS — but `viewEnc` encodes peer-determined merge
labels (D vs R-cut vs M-absent) of the party's own held children, which
no party knows at session start from its own tree. The charter's honest
grain is *information in the causal past of the party at the decision
point*: in this model, the session parameters plus the sub-skeleton
ANNOUNCED by the frames that have arrived (refute-c1 §1.2's two minting
rules; bridge axiom B5: frames are decoded at delivery, so an arrived
frame's decoded record is trace information even though the
payload-erased `MObs` does not carry it syntactically). `aviewOf` below
is exactly that announced view — the probe `KnownSkel`'s Lean
counterpart, whose t = 0 state is the bare parameters. `CharterLocal`
(invariance across equal announced views at `.impl`-consistent
observations) is the locality
class of record; note it is INCOMPARABLE to the legacy class, not
finer: `LocalEq` pairs may differ in announced content (answerer-side
R children, `leafReqs` of announced scopes), and announced-view pairs
may differ in unannounced view structure. The a-fortiori transfer
between the two classes therefore fails in both directions — recorded
here so nobody re-derives it hopefully (Proofs/C1.lean carries the
statement-of-record consequences).

# Why the strategy reads `sk` where it does

Every `sk`-read in `aviewOf` is announced-gated: positional decode of
an arrival to the scope it is about (B5 rule 1), the record of an
announced scope, and the kid labels a known record's frame carried
(B5 rule 2 — Python's `kinds[kd]` stratum). Everything downstream —
census, layouts, closure, demand — is a pure function of the `AView`
tuple, so `sigmaStarCausal_charterLocal` is a computation, not an
inference-sufficiency theorem. A payload-erased surrogate that pins the
same data from the party's own commit ledger (d4/d6 pacing) plus
arrival counts was designed and rejected for the claim of record: the
alternation parity puts the peer-stage kind marks on the wrong side
forever (they ride only payloads), so an `MObs`-only strategy
under-derives and starves on the wedge's provision wall — the
payload-erasure finding this track reports (MUX-PROGRESS §4 entry).

# The announced trace family and the causal closure

The closure needs only the PEER endpoint's traces: no internal channel
crosses the link, so a peer event's E1/E2/E3 past touches peer events,
wire sends (evidence: `groundedPush`, unchanged — it was always
history-only), and the party's own wire receives (C-own evidence from
its own `.act` stream). The layouts transcribe the `.impl` trace
grammar (`Sched.walkEventsE`/`asmEvents`/`absorbEvents`/openers/fins)
over the partial record table, TRUNCATING at the first unannounced
quantity — Python's `Unknown` discipline in trace form: derive less,
never guess. Announced traces are therefore literal prefixes of the
true traces, which is what the deferred containment lemma wants.

`istepOkA` is `istepOk` with the two skeleton reads rerouted: `capA`
(the `capLevel` parameter — `Skel.cap` reads nothing else) for the
positional slot-E2 window, the announced family for the E3 past. The
F6 membership discipline and the no-cross-stream-pipe-edges shape are
inherited verbatim.

# The probe relationship: counterpart, not twin

`sigmaStarCausal` is not extensionally the probe's strategy; two
divergence axes separate them, in opposite directions. (i) `groundedA`
grounds the party's own performed wire receives (`ownRecvCount`, read
off its own `.act` stream), where causal-reference.py implements the
strategy as a function of pushes and arrivals only — the divergence
STAGE0-GATES.md names as a live risk. (ii) The Python fixpoint derives
through simulated own-side events, so its closure can prove strictly
more on own-endpoint structure than `inevitableA`. Both directions are
conservative for the claims of record — the liveness of THIS object is
kernel-proven end to end and rests on the probe nowhere — but P1's
4,970/4,970 terminal runs validated the Python counterpart, not this
definition; every P1 citation in this file reads with that
qualification.

# AxMode binding

Everything here lives at `.impl`: the layouts transcribe the
d6/epilogue trace grammar, `ConsistentImpl` pins the mode, and the
probe evidence (4,970/4,970 terminal causal runs, STAGE0-GATES.md P1)
is `.impl`-only. Under other modes the layouts would be wrong-shaped
and every claim below is simply not made.

# Proof state (honest ledger)

Kernel-proven here: `sigmaStarCausal_charterLocal` (both parties,
definitional), the wf-free `partyOf` pinning it rests on, and the
executable pins (`smokeChain` and — the real derivation exercise —
`wedge`, whose provision wall demands closure-proven frames 2..7 on
the initiator's top stream).

Kernel-proven in Proofs/CausalCoverage.lean, Proofs/CausalLive.lean,
and Proofs/CausalMint.lean (the liveness track, now CLOSED): the
announced-prefix property — every `announcedProcs` trace is a literal
prefix of a true `.impl` process trace (`announcedProcs_prefix`, an
∃-pairing against `procsE`) —
the receive ledger (`RecvLedger`), the causal keystone (`keystoneA`),
σ*-causal's push certificates (`pushProvenA_reachable`), Step 1's
pipes-drain (`sigmaStarCausal_pipes_empty`), the liveness assembly
`sigmaStarCausal_deadlock_free_of_coverage`, AND its one conjunct:
`CausalStuckCoverage`, discharged by `causalStuckCoverage`
(Proofs/CausalMint.lean). The discharge is the minting ladder — at a
stuck drained state every record the closure consults was announced,
because the consulting event's τ-past contains the minting frame's
send and drained pipes make sent frames delivered — composed with the
τ-staged causal coverage induction and the closure's saturation
argument. `sigmaStarCausal_deadlock_free` is therefore unconditional,
`c1_charter_false` carries no hypothesis (Proofs/C1.lean), and T8's
"inference progress" conjunct is available as a lemma.
-/
import StreamingMirror.Mux.Proofs.SigmaStarInv
import StreamingMirror.Mux.Strategy

namespace StreamingMirror.Mux

open Model
open Sched (Ev)

-- ==================================================== the announced view

/-- The announced view: what the party's causal past determines about
the session — the parameters (`KnownSkel`'s t = 0 state) plus the
records of announced scopes and the kid-label stratum their frames
carried. Two skeletons with equal `AView`s at a shared observation
history are indistinguishable to the party AT THAT POINT; this tuple
is the charter grain's carrier. -/
structure AView where
  party : Party
  rootH : Nat
  fan : Nat
  capLevel : Nat
  recs : List (Nat × Scope)
  kinds : List (Nat × Kind)
  deriving DecidableEq, Repr

/-- The scope ids announced to party `p` by the arrivals in `tr`, in
a canonical enumeration order — root branch first, then per peer
height in stream-major position order — with multiplicity
(deduplication happens at the record table). Deliberately NOT arrival
order: the enumeration reads the trace only through per-stream
delivered counts, so the view is interleaving-independent. The
positional decode of refute-c1 §1.2 —

- the opening frame announces the root, and its initiator-side arrival
  (the responder's reply rides `wire R rootH`) also mints the root's
  kids (the receiver answers the stage below);
- frame `n` on `wire q h` (`0 < h < rootH`) is about the `n`-th scope
  of level `h` and mints its record and its kids' records;
- supplies (`h = 0`) announce nothing.

Reads `sk` only positionally at arrived frames — bridge axiom B5. -/
def announcedIds (sk : Skel) (p : Party) (tr : List MObs) : List Nat :=
  let peerHeights : List Nat :=
    if p == Party.I then
      (List.range (sk.rootH / 2)).map fun k => sk.rootH - 2 - 2 * k
    else
      (List.range (sk.rootH / 2)).map fun k => sk.rootH - 1 - 2 * k
  (if 0 < deliveredCount tr sk.rootH then
    0 :: (if p == Party.I then (sk.scope 0).kids else [])
  else []) ++
  peerHeights.flatMap fun h =>
    if h == 0 then []
    else
      (List.range (min (deliveredCount tr h) (sk.scopesAt h).length))
        |>.flatMap fun n =>
          let u := (sk.scopesAt h).getD n 0
          u :: (sk.scope u).kids

/-- Party `p`'s announced view of session `sk` at history `tr` — the
single point where the causal strategy reads the skeleton. The `kinds`
stratum carries the labels a known record's frame announced for its
kids (Python `KnownSkel`'s census kinds): a kid's KIND is known one
minting earlier than its own record. -/
def aviewOf (sk : Skel) (p : Party) (tr : List MObs) : AView :=
  let ids := (announcedIds sk p tr).eraseDups
  { party := p
    rootH := sk.rootH
    fan := sk.fan
    capLevel := sk.capLevel
    recs := ids.map fun u => (u, sk.scope u)
    kinds := ids.flatMap fun u =>
      (sk.scope u).kids.map fun v => (v, (sk.scope v).kind) }

/-- The record of scope `u`, if announced. -/
def AView.rec? (av : AView) (u : Nat) : Option Scope :=
  av.recs.lookup u

/-- The kind of scope `u`, if its parent's frame announced it (the
root's kind is session-static). -/
def AView.kind? (av : AView) (u : Nat) : Option Kind :=
  if u == 0 then some Kind.D else av.kinds.lookup u

-- ==================================================== the announced census

/-- The known prefix of the level `steps` descents below the root, with
its completeness flag: the BFS census of `KnownSkel._levels`. A level's
enumeration extends through each D member whose record is known and
stops (incomplete) at the first D member whose record is not; non-D
members contribute no kids and never block. -/
def levelA (av : AView) : Nat → List Nat × Bool
  | 0 => ([0], true)
  | steps + 1 =>
      let (up, upc) := levelA av steps
      let rec collect : List Nat → List Nat × Bool
        | [] => ([], true)
        | sid :: rest =>
            if av.kind? sid == some Kind.D then
              match av.rec? sid with
              | some sc =>
                  let (items, comp) := collect rest
                  (sc.kids ++ items, comp)
              | none => ([], false)
            else collect rest
      let (items, comp) := collect up
      (items, upc && comp)

/-- The announced stage-scope prefix of stage `h` (the scopes at level
`h + 1`), with completeness. -/
def stageScopesA (av : AView) (h : Nat) : List Nat × Bool :=
  if h + 1 == av.rootH then ([0], true)
  else if av.rootH < h + 1 then ([], true)
  else levelA av (av.rootH - (h + 1))

-- ============================================== the announced trace family

/-- Channel capacity from the session parameters (`Skel.cap` reads
only `capLevel`). -/
def capA (av : AView) : Chan → Nat
  | .level _ _ => av.capLevel
  | _ => 1

/-- The count a D child contributes to its scope's query train: its own
kid census, or its leaf requests at stage 1 (`Skel.qCount`'s reading,
over the announced record). -/
def qCountA (av : AView) (h : Nat) (v : Nat) : Option Nat :=
  (av.rec? v).map fun sc => if h == 1 then sc.leafReqs else sc.kids.length

/-- One peer scope block in the `.impl` trace grammar
(`Sched.scopeBlockE`: prologue receives, the child chunks in radix
order — wire, then for a D child its resolution and query train — the
parent summary last). Truncates at the first unannounced child kind or
census; a truncated block ends its stage's trace. Returns the events,
the advanced (wire, res, query) counters, and the may-continue flag. -/
def peerBlockA (av : AView) (q : Party) (h k : Nat)
    (u : Nat) (wires ds qs : Nat) :
    List Ev × (Nat × Nat × Nat) × Bool :=
  let prologue : List Ev :=
    [(Chan.wire q.other (h + 1), false, k), (Chan.asked q h, false, k)]
  match av.rec? u with
  | none => (prologue, (wires, ds, qs), false)
  | some sc =>
      let n := if h == 0 then sc.leafReqs else sc.kids.length
      let rec chunks (i : Nat) (w d qacc : Nat) (fuel : Nat) :
          List Ev × (Nat × Nat × Nat) × Bool :=
        match fuel with
        | 0 => ([], (w, d, qacc), true)
        | fuel + 1 =>
            if h == 0 then
              -- leaf supplies: wire per request, never disputed
              let evs := (List.range n).map fun j =>
                ((Chan.wire q h : Chan), true, wires + j)
              (evs, (w + n, d, qacc), true)
            else
              match sc.kids[i]? with
              | none => ([], (w, d, qacc), true)
              | some v =>
                  let wireEv : Ev := (Chan.wire q h, true, w)
                  match av.kind? v with
                  | none => ([], (w, d, qacc), false)
                  | some Kind.R =>
                      let (evs, cnts, ok) :=
                        chunks (i + 1) (w + 1) d qacc fuel
                      (wireEv :: evs, cnts, ok)
                  | some Kind.D =>
                      match qCountA av h v with
                      | none => ([wireEv], (w + 1, d, qacc), false)
                      | some t =>
                          let res : Ev := (Chan.lower q h, true, d)
                          let train := (List.range t).map fun j =>
                            (askedOut (q, h), true, qacc + j)
                          let (evs, cnts, ok) :=
                            chunks (i + 1) (w + 1) (d + 1) (qacc + t) fuel
                          (wireEv :: res :: train ++ evs, cnts, ok)
      let (evs, (w', d', q'), ok) :=
        chunks 0 wires ds qs (sc.kids.length + 1)
      let parent : List Ev :=
        if ok then [(Chan.upper q h, true, k)] else []
      (prologue ++ evs ++ parent, (w', d', q'), ok)

/-- A peer walk stage's announced trace: the known stage-scope prefix,
each block transcribed exactly, the whole trace cut at the first
truncated block (announced traces must be true-trace prefixes, so
nothing may be laid past a hole). -/
def peerWalkTraceA (av : AView) (h : Nat) : List Ev :=
  let q := av.party.other
  let (items, _) := stageScopesA av h
  let rec go (is : List Nat) (k wires ds qs : Nat) : List Ev :=
    match is with
    | [] => []
    | u :: rest =>
        let (evs, (w', d', q'), ok) := peerBlockA av q h k u wires ds qs
        if ok then evs ++ go rest (k + 1) w' d' q'
        else evs
  go items 0 0 0 0

/-- The peer's walk stage heights, top down (the peer of `av.party`). -/
def peerStagesA (av : AView) : List Nat :=
  if av.party == Party.I then
    (List.range (av.rootH / 2)).map fun k => av.rootH - 2 - 2 * k
  else
    (List.range (av.rootH / 2)).map fun k => av.rootH - 1 - 2 * k

/-- The peer's assembler heights (`Skel.asmKeys` for the peer party). -/
def peerAsmHeightsA (av : AView) : List Nat :=
  if av.party == Party.I then
    (List.range (av.rootH - 1)).map (· + 1)
  else
    (List.range av.rootH).map (· + 1)

/-- The announced pending counts of the peer assembler at height `j`
(`Skel.asmResList` over the census): asker side one entry per level-`j`
scope pending its dispute count, answerer side one per D scope pending
its kid census (leaf requests at height 1). `none` entries truncate. -/
def asmPendsA (av : AView) (j : Nat) : List (Option Nat) :=
  let q := av.party.other
  let (items, _comp) :=
    if j == av.rootH then ([0], true)
    else levelA av (av.rootH - j)
  if asks q j then
    items.map fun u =>
      match av.rec? u with
      | none => none
      | some sc =>
          some (sc.kids.countP fun v => av.kind? v == some Kind.D)
  else
    (items.filter (fun u => av.kind? u == some Kind.D)).map fun u =>
      match av.rec? u with
      | none => none
      | some sc => some (if j == 1 then sc.leafReqs else sc.kids.length)

/-- One peer assembler's announced trace (`Sched.asmBlock` transcribed,
truncating at the first unannounced pend). -/
def peerAsmTraceA (av : AView) (j : Nat) : List Ev :=
  let q := av.party.other
  let resChan := asmResChan (q, j)
  let levChan := asmLevelChan (q, j)
  let outChan : Chan :=
    if q == Party.I && j == av.rootH then .rootret
    else if q == Party.R && j == av.rootH - 1 then .rootrets
    else .level q j
  let rec go (ps : List (Option Nat)) (idx got : Nat) : List Ev :=
    match ps with
    | [] => []
    | none :: _ => []
    | some pend :: rest =>
        (resChan, false, idx)
          :: ((List.range pend).map fun t => (levChan, false, got + t))
          ++ (outChan, true, idx)
          :: go rest (idx + 1) (got + pend)
  go (asmPendsA av j) 0 0

/-- The peer-side absorb trace (initiator endpoint, so laid only by the
responder): constant block shape, one block per leaf request of the
announced complete level-1 D prefix. -/
def peerAbsorbTraceA (av : AView) : List Ev :=
  if av.party == Party.I then []
  else
    let (items, _) := levelA av (av.rootH - 1)
    let rec total (is : List Nat) : Nat × Bool :=
      match is with
      | [] => (0, true)
      | u :: rest =>
          if av.kind? u == some Kind.D then
            match av.rec? u with
            | none => (0, false)
            | some sc =>
                let (t, ok) := total rest
                (sc.leafReqs + t, ok)
          else total rest
    (List.range (total items).1).flatMap fun j =>
      [((Chan.wire Party.R 0 : Chan), false, j),
       ((Chan.leafRequests : Chan), false, j),
       ((Chan.level Party.I 0 : Chan), true, j)]

/-- The peer opener's announced trace: the responder opening for the
initiator (root queries once the root record is announced), the
constant initiator opening for the responder. -/
def peerOpenTraceA (av : AView) : List Ev :=
  if av.party == Party.I then
    [((Chan.wire Party.I av.rootH : Chan), false, 0),
     ((Chan.wire Party.R av.rootH : Chan), true, 0),
     ((Chan.rootres : Chan), true, 0)]
      ++ (match av.rec? 0 with
          | none => []
          | some sc =>
              (List.range sc.kids.length).map fun j =>
                ((Chan.asked Party.R (av.rootH - 2) : Chan), true, j))
  else
    [((Chan.wire Party.I av.rootH : Chan), true, 0),
     ((Chan.asked Party.I (av.rootH - 1) : Chan), true, 0)]

/-- The peer's finale traces: the responder finale for the initiator
(root returns gated on the announced root census), the floating
`rootret` receive for the responder. -/
def peerFinTracesA (av : AView) : List (List Ev) :=
  if av.party == Party.I then
    [((Chan.rootres : Chan), false, 0)
        :: (match av.rec? 0 with
            | none => []
            | some sc =>
                (List.range sc.kids.length).map fun j =>
                  ((Chan.rootrets : Chan), false, j))]
  else
    [[((Chan.rootret : Chan), false, 0)]]

/-- The announced trace family: every peer-endpoint process, laid to
the announced frontier. Own processes never appear — their events enter
as evidence (C-own), and no internal channel crosses the link, so peer
derivations never consult them. -/
def announcedProcs (av : AView) : List (List Ev) :=
  [peerOpenTraceA av]
    ++ (peerStagesA av).map (peerWalkTraceA av)
    ++ [peerAbsorbTraceA av]
    ++ (peerAsmHeightsA av).map (peerAsmTraceA av)
    ++ peerFinTracesA av

-- ======================================================== the causal closure

/-- The party's own performed wire receives, read off its own `.act`
stream — C-own evidence, pure history arithmetic. -/
def ownRecvCount (av : AView) (tr : List MObs) (h : Nat) : Nat :=
  tr.countP fun o =>
    match o with
    | .act (.walkRecvWire pk) => pk == (av.party, h - 1) && h != 0
    | .act .ropenRecv => av.party == Party.R && h == av.rootH
    | .act .absorbRecvWire => av.party == Party.I && h == 0
    | _ => false

/-- Causal evidence: wire sends grounded by flush/delivery counts
(exactly `groundedPush` — it was always history-only) plus the party's
own performed wire receives. -/
def groundedA (av : AView) (tr : List MObs) (e : Ev) : Bool :=
  groundedPush av.party tr e ||
    (match e with
     | (Chan.wire q h, false, n) =>
         q == av.party.other && decide (n < ownRecvCount av tr h)
     | _ => false)

/-- The wire stream heights of party `p`, from the parameters
(`wireHeights` with the skeleton read routed through the view). -/
def wireHeightsA (av : AView) (p : Party) : List Nat :=
  av.rootH ::
    (if p == Party.I then
      (List.range (av.rootH / 2)).map fun k => av.rootH - 1 - 2 * k
    else
      (List.range (av.rootH / 2)).map fun k => av.rootH - 2 - 2 * k)

/-- The announced event universe: the evidence events plus the
announced traces' events. -/
def evUnivA (av : AView) (tr : List MObs) : List Ev :=
  ((wireHeightsA av av.party).flatMap fun h =>
    (List.range (pushedCount tr h)).map fun n =>
      ((Chan.wire av.party h : Chan), true, n))
  ++ ((wireHeightsA av av.party.other).flatMap fun h =>
    (List.range (deliveredCount tr h)).map fun n =>
      ((Chan.wire av.party.other h : Chan), true, n))
  ++ ((wireHeightsA av av.party.other).flatMap fun h =>
    (List.range (ownRecvCount av tr h)).map fun n =>
      ((Chan.wire av.party.other h : Chan), false, n))
  ++ (announcedProcs av).flatten

/-- The causal I-step: `istepOk` with the two skeleton reads rerouted —
`capA` for the positional slot-E2 window, the announced family for the
E3 past. F6 membership guards and the no-cross-stream-pipe-edges shape
are inherited verbatim; pushes are never derived. -/
def istepOkA (av : AView) (procs : List (List Ev)) (D : List Ev)
    (e : Ev) : Bool :=
  !(isWire e.1 && e.2.1) &&
  (e.2.1 || D.contains (e.1, true, e.2.2)) &&
  (!e.2.1 || decide (e.2.2 < capA av e.1)
    || D.contains (e.1, false, e.2.2 - capA av e.1)) &&
  (procs.all fun T =>
    !(T.contains e)
      || (T.takeWhile (fun x => !(x == e))).all (D.contains ·))

/-- One causal saturation pass (cf. `closureStep`). -/
def closureStepA (av : AView) (tr : List MObs) (univ : List Ev)
    (procs : List (List Ev)) (D : List Ev) : List Ev :=
  univ.filter fun e =>
    D.contains e || groundedA av tr e || istepOkA av procs D e

/-- The causal saturation chain from the grounded evidence. -/
def closureNA (av : AView) (tr : List MObs) (univ : List Ev)
    (procs : List (List Ev)) : Nat → List Ev
  | 0 => univ.filter (groundedA av tr)
  | n + 1 =>
      closureStepA av tr univ procs (closureNA av tr univ procs n)

/-- The certified events, causal form: the grounded evidence itself
(C-own/C-arr of refute-c1 §1.3). -/
def certifiedA (av : AView) (tr : List MObs) : List Ev :=
  (evUnivA av tr).filter (groundedA av tr)

/-- The inevitable events, causal form: the forward closure of the
evidence over the announced traces, run to the announced universe's
depth (each productive pass adds an event). -/
def inevitableA (av : AView) (tr : List MObs) : List Ev :=
  let univ := evUnivA av tr
  closureNA av tr univ (announcedProcs av) univ.length

-- ============================================================== σ*-causal

/-- Is the next frame on stream `h` proven-demanded from announced
information? First frames are unconditionally demanded (every
consumer's first wire-channel operation is the receive itself); later
frames need the predecessor's consumption in the causal closure —
refute-c1 §1.4's rule, `Certified ∪ Inevitable` collapsed by
construction (stage 0 of `closureNA` is the evidence). -/
def demandedA (av : AView) (tr : List MObs) (h : Nat) : Bool :=
  pushedCount tr h == 0 ||
    (inevitableA av tr).contains
      (Chan.wire av.party h, false, pushedCount tr h - 1)

/-- The strategy core over the announced view: the locally-least
(stream-list order — `rootH` first, then the walk stages top down)
held, proven-demanded stream. Any selection rule serves the liveness
argument (the chase's withheld push is itself proven-demanded); the
fixed order keeps the strategy deterministic without the omniscient
τ. -/
def causalCore (av : AView) (tr : List MObs) : Option Nat :=
  (wireHeightsA av av.party).find? fun h =>
    committedInHist av.rootH tr h && demandedA av tr h

/-- σ*-causal: demand-lockstep over the announced sub-skeleton — the
strategy of record behind `c1_charter_false`. The single skeleton read
is `aviewOf sk p tr` — the parameters plus the announced records — so
charter-grain locality is definitional
(`sigmaStarCausal_charterLocal`). The machine identifies itself from
its own history exactly as σ* does. -/
def sigmaStarCausal : Strategy := fun sk tr =>
  match partyOf tr with
  | none => none
  | some p => causalCore (aviewOf sk p tr) tr

-- ================================================== the charter-grain class

/-- Observation realizability at the shipping interface: some
`.impl`-mode run of some capacity and strategy pair puts `tr` on
machine `p`. The mode is BOUND to `.impl` deliberately (the legacy
`Consistent` leaves it existential): every layout above transcribes the
d6/epilogue trace grammar and the probe evidence is `.impl`-only, so
the charter-grain claims quantify over exactly the runs they are about.
`ConsistentImpl p sk tr → Consistent p sk tr` by weakening. -/
def ConsistentImpl (p : Party) (sk : Skel) (tr : List MObs) : Prop :=
  ∃ (C : Nat) (σI σR : Strategy) (s : MState),
    MReachable sk .impl C σI σR s ∧ s.hist p = tr

/-- A kernel-checked `.impl` replay certifies shipping-interface
consistency: if `obsOf` computes the trace, some `.impl`-reachable
state carries it — `Consistent.of_obsOf` with the mode pinned, the
glue the charter-grain controls (Mux/Proofs/Grains.lean) use. -/
theorem ConsistentImpl.of_obsOf (C : Nat) (σI σR : Strategy)
    (acts : List MAction) {p : Party} {sk : Skel} {tr : List MObs}
    (h : obsOf sk .impl C σI σR acts p = some tr) :
    ConsistentImpl p sk tr := by
  rw [obsOf] at h
  cases hm : mrun sk .impl C σI σR (init sk) acts with
  | none => rw [hm] at h; cases h
  | some s =>
      rw [hm] at h
      injection h with h
      exact ⟨C, σI, σR, s, mrun_reachable hm, h⟩

/-- σ is charter-grain local for party `p`: invariant across skeletons
whose ANNOUNCED VIEWS agree at the observation, on every history both
skeletons can realize at the shipping interface.

This is Finch's F3 grain — "information in the causal past of that
party at the decision point": the announced view (`aviewOf`, the probe
`KnownSkel`'s counterpart) is precisely what the arrived frames have
determined, with the session parameters as its t = 0 state. It is
deliberately NOT the legacy `LocalStrategy` (Mux/Strategy.lean), whose
`viewEnc` grain encodes peer-determined merge labels a party cannot
know at session start; the two classes are incomparable (module doc),
and the claims of record quantify over THIS one (Proofs/C1.lean). -/
def CharterLocal (p : Party) (σ : Strategy) : Prop :=
  ∀ (sk sk' : Skel) (tr : List MObs),
    aviewOf sk p tr = aviewOf sk' p tr →
    ConsistentImpl p sk tr → ConsistentImpl p sk' tr →
    σ sk tr = σ sk' tr

-- ================================================ locality, kernel tier

/-- A successful push's only history effect is the flush receipt on the
pushing machine (every success arm of `firePush` builds through its
`push` constructor). Public: the K-variant's history attribution
(Mux/SigmaStarK.lean) reuses it through the shared push arm. -/
theorem firePush_hist {sk : Skel} {C : Nat} {q : Party} {h : Nat}
    {s s' : MState} (hstep : firePush sk C q h s = some s') :
    s'.hist = recordObs s.hist q (.pushed h) := by
  rw [firePush] at hstep
  simp only [] at hstep
  split at hstep
  · split at hstep
    · -- the opening stream: party match, then obligation match
      cases q with
      | I =>
          cases hob : s.base.iopenCh with
          | none => rw [hob] at hstep; cases hstep
          | some ob =>
              cases ob with
              | wire =>
                  rw [hob] at hstep
                  injection hstep with hs'
                  rw [← hs']
              | query => rw [hob] at hstep; cases hstep
      | R =>
          cases hob : s.base.ropenCh with
          | none => rw [hob] at hstep; cases hstep
          | some ob =>
              cases ob with
              | wire =>
                  rw [hob] at hstep
                  injection hstep with hs'
                  rw [← hs']
              | res => rw [hob] at hstep; cases hstep
              | query => rw [hob] at hstep; cases hstep
    · -- a walk stream: committed match, then the stage guard
      split at hstep
      · split at hstep
        · injection hstep with hs'
          rw [← hs']
        · cases hstep
      · cases hstep
  · cases hstep

/-- One muxed step's history effect, arm-generic: unchanged histories
except one machine's appended observation — an `.act` filed under its
own `actionParty`, or a non-`.act` receipt. Public: the K-variant's
base and push arms are shared definitionally, so its history
attribution (Mux/SigmaStarK.lean) delegates here. -/
theorem apply_hist_cases {sk : Skel} {ax : AxMode} {C : Nat}
    {σI σR : Strategy} {ma : MAction} {s₀ s₁ : MState}
    (hstep : apply sk ax C σI σR ma s₀ = some s₁) (p : Party) :
    s₁.hist p = s₀.hist p
      ∨ (∃ b, s₁.hist p = s₀.hist p ++ [MObs.act b] ∧ actionParty b = p)
      ∨ (∃ o, s₁.hist p = s₀.hist p ++ [o] ∧ ∀ b, o ≠ MObs.act b) := by
  have hrec : ∀ (q₀ : Party) (o : MObs),
      s₁.hist = recordObs s₀.hist q₀ o →
      (p = q₀ → s₁.hist p = s₀.hist p ++ [o])
        ∧ (p ≠ q₀ → s₁.hist p = s₀.hist p) := by
    intro q₀ o hh
    have hpq : s₁.hist p
        = if p == q₀ then s₀.hist p ++ [o] else s₀.hist p := by
      rw [hh]; rfl
    constructor
    · intro hp
      rwa [if_pos (by simp [hp])] at hpq
    · intro hp
      rwa [if_neg (by simp [hp])] at hpq
  cases ma with
  | base a =>
      obtain ⟨-, b, -, hs₁⟩ := applyBase_inv hstep
      have hh : s₁.hist = recordObs s₀.hist (actionParty a) (.act a) := by
        rw [hs₁]
      by_cases hp : p = actionParty a
      · exact Or.inr (Or.inl ⟨a, (hrec _ _ hh).1 hp, hp.symm⟩)
      · exact Or.inl ((hrec _ _ hh).2 hp)
  | push q =>
      simp only [apply] at hstep
      split at hstep
      next h _ =>
        have hh := firePush_hist hstep
        by_cases hp : p = q
        · exact Or.inr (Or.inr ⟨.pushed h, (hrec _ _ hh).1 hp,
            by intro b hb; cases hb⟩)
        · exact Or.inl ((hrec _ _ hh).2 hp)
      next => cases hstep
  | deliver q =>
      simp only [apply] at hstep
      split at hstep
      next c rest _ =>
        split at hstep
        · injection hstep with hs₁
          have hh : s₁.hist
              = recordObs s₀.hist q.other (.delivered (wireHeight c)) := by
            rw [← hs₁]
          by_cases hp : p = q.other
          · exact Or.inr (Or.inr ⟨.delivered (wireHeight c),
              (hrec _ _ hh).1 hp, by intro b hb; cases hb⟩)
          · exact Or.inl ((hrec _ _ hh).2 hp)
        · cases hstep
      next => cases hstep

/-- Histories attribute correctly at every reachable state, with no
well-formedness hypothesis and any axiom mode: `recordObs` files each
observation under the acting machine. (The `HistInv` bundle proves this
under `wellFormed`; the charter statements quantify over consistent
traces of arbitrary skeletons, so this standalone induction carries the
wf-free form.) -/
theorem histParty_reachable {sk : Skel} {ax : AxMode} {C : Nat}
    {σI σR : Strategy} {s : MState}
    (hr : MReachable sk ax C σI σR s) :
    ∀ p a, MObs.act a ∈ s.hist p → actionParty a = p := by
  induction hr with
  | init =>
      intro p a hmem
      cases hmem
  | step ma hr' hstep ih =>
      intro p a hmem
      rcases apply_hist_cases hstep p with heq | ⟨b, heq, hbp⟩
        | ⟨o, heq, hno⟩
      · rw [heq] at hmem
        exact ih p a hmem
      · rw [heq, List.mem_append] at hmem
        rcases hmem with hold | hnew
        · exact ih p a hold
        · rw [List.mem_singleton] at hnew
          injection hnew with hab
          rw [hab]
          exact hbp
      · rw [heq, List.mem_append] at hmem
        rcases hmem with hold | hnew
        · exact ih p a hold
        · rw [List.mem_singleton] at hnew
          exact absurd hnew.symm (hno a)

/-- `partyOf` pins the machine on any `.impl`-realizable trace: a hit
names the history's owner (the wf-free `partyOf_eq`). -/
theorem partyOf_consistentImpl {p : Party} {sk : Skel} {tr : List MObs}
    (hc : ConsistentImpl p sk tr) {q : Party}
    (hq : partyOf tr = some q) : q = p := by
  obtain ⟨C, σI, σR, s, hr, htr⟩ := hc
  rw [← htr] at hq
  rw [partyOf] at hq
  obtain ⟨o, ho, hsome⟩ := List.exists_of_findSome?_eq_some hq
  cases o with
  | act a =>
      have := histParty_reachable hr p a ho
      simp only [Option.some.injEq] at hsome
      rw [← hsome, this]
  | pushed h => cases hsome
  | delivered h => cases hsome

/-- σ*-causal is charter-grain local — for BOTH parties, by
computation: its verdict factors through the announced view and the
history, equal views rewrite, and `ConsistentImpl` pins the
self-identification. This is the definitional half of the locality
residue's discharge; the liveness half landed too
(`causalStuckCoverage`, Proofs/CausalMint.lean), making
`sigmaStarCausal_deadlock_free` unconditional. -/
theorem sigmaStarCausal_charterLocal (p : Party) :
    CharterLocal p sigmaStarCausal := by
  intro sk sk' tr hav hc hc'
  simp only [sigmaStarCausal]
  cases hq : partyOf tr with
  | none => rfl
  | some q =>
      have hqp : q = p := partyOf_consistentImpl hc hq
      subst hqp
      show causalCore (aviewOf sk q tr) tr = causalCore (aviewOf sk' q tr) tr
      rw [hav]

-- ==================================================== the executable pins

set_option maxRecDepth 1000000 in
/-- The σ*-causal-driven drain completes the smoke pin in the kernel
(`muxCompletes`: the drain reaches `mterminal` within the stated fuel
— completion in the literal kernel sense): announce decode, census,
layouts, closure, and selection all execute end to end — the strategy
is a real scheduler, not only a proof object. Capacity is
message-denominated; the byte caveat of record is Mux/Basic.lean's
module doc (# The byte-denomination caveat). -/
theorem smokeChain_sigmaStarCausal_completes :
    muxCompletes Pin.smokeChain .impl 1 sigmaStarCausal sigmaStarCausal
      400 = true := by
  decide

set_option maxRecDepth 4000000 in
set_option maxHeartbeats 4000000 in
/-- The σ*-causal drain completes the WEDGE at C = 1 — the pin that
exercises the derivation machinery for real: the provision wall is six
whole-subtree provisions behind the deep dispute on the initiator's top
stream, so frames 2..7 push only on closure-proven demand (every
work-conserving pair jams here, `wc_impossibility`). This is the
in-kernel companion of the stage-0 probe's 4,970 terminal causal runs,
on the campaign's canonical adversarial shape. Kernel cost: minutes,
not seconds (the closure re-derives per push decision); with the
coverage theorem landed (`causalStuckCoverage`,
Proofs/CausalMint.lean) the pin stands as an executable witness
behind `sigmaStarCausal_deadlock_free`, not as the claim's support
(`muxCompletes` = the drain reaches `mterminal` within the stated
fuel, completion in the literal kernel sense). Capacity is
message-denominated; the byte caveat of record is Mux/Basic.lean's
module doc (# The byte-denomination caveat). -/
theorem wedge_sigmaStarCausal_completes :
    muxCompletes wedge .impl 1 sigmaStarCausal sigmaStarCausal 800
      = true := by
  decide

end StreamingMirror.Mux
