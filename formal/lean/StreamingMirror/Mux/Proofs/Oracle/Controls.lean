/-
The T5/T9 control suite (MUX-ADJUDICATION.md §3 T5 controls, §2.4
mandatory controls; stage-3 track E): kernel `decide` pins bracketing
the oracle theorem from every side the adjudication names.

- **the refuted primary form, pinned** — the receive-projection pusher
  `ofSchedule (demandOrder sk d)` (MUX-ADJUDICATION §1.3's π_d, the
  form the P2 gate and muxprobe executably refuted) jams a pinned
  margin-0 witness at C = 1 (`static_oracle_jams`), which the oracle of
  record completes at the same capacity (`piWedge_oracle_completes`);
- **non-vacuity anchors** — the oracle completes the positive smoke
  skeleton AND `wedge` at C = 1 (`wedge_oracle_completes`: the exact
  skeleton on which T3 kills every work-conserving pair — the
  trichotomy's positive half on the impossibility's own witness);
- **T9, locality** — `LocalEq` is nondegenerate (two distinct skeletons
  with equal views on BOTH sides); neither the demand order nor the
  oracle of record is view-invariant at projection strength
  (`demandOrder_not_local`, `oracle_not_local`); and the oracle fails
  `LocalStrategy` itself, `Consistent` certificates included
  (`oracle_not_localStrategy`, phase-4 F2): the oracle genuinely
  consumes remote structure, so T6's necessity reading is about a real
  hypothesis.

# Which witness carries the static jam

muxprobe's matrix instance is `rand2 = genSkelM0 2` (240 scopes,
executable tier, capacity- and interleaving-flat — Mux/Gen.lean). The
kernel pin here uses the SMALLEST generator witness instead:
`genSkelM0 2859` (19 scopes, rootH 4, found by sweeping seeds 0–2999),
materialized as the literal `piWedge` because `genSkelM0`'s `Id.run`
loops do not kernel-reduce, and a 240-scope drain is out of `decide`
range regardless. Same mechanism, same verdict, kernel-checked; rand2
stays pinned in the muxprobe golden matrix.

# The T9 witness pair

`viewPair`/`viewPair'` differ ONLY in a height-1 leaf-request count —
erased from BOTH parties' views (`viewEnc`; oracle-c2 §3.2's
adjudicated erasure), so `LocalEq p` holds for both parties while the
responder's supply-run length differs: every projection of τ on the
responder's side (receive order AND send order) sees the difference.
Searched and not found: an INITIATOR-side witness — on every generator
instance and leaf-request mutation swept (seeds 0–119, all height-1
scopes), `demandOrder · .I` and `sendProj · .I` were view-invariant.
That asymmetry (the initiator's frame order looks locally computable;
the responder's provably is not) is recorded here as an observation,
not a theorem; the responder-side witness is all T9 needs.
-/
import StreamingMirror.Mux.Proofs.Oracle
import StreamingMirror.Mux.Instances

namespace StreamingMirror.Mux

open Model
open Pin (sc)

-- ================================== the refuted primary form, as data

/-- The static list-pusher: entry `k` of a fixed height list names the
`k`-th push; idle while the next listed frame is not yet in hand
(MUX-ADJUDICATION §3 T5's `ofSchedule`, the thin `Strategy` wrapper).

`Gen.pushList` is its executable-tier twin (indexed by the same flush
count); minted here because a kernel control consumes it. -/
def ofSchedule (ord : List Nat) : Strategy := fun _ tr =>
  ord[(pushHeights tr).length]?

/-- Direction `d`'s wire frames in receiver-consumption order: the
RECEIVE projection of the canonical schedule, as stream heights —
MUX-ADJUDICATION §1.3's demand order π_d, the oracle form the P2 gate
executably refuted (`Gen.piOrder` is the executable-tier twin,
definitionally equal: `piOrder_eq_demandOrder`, Mux/Proofs/Twins.lean;
the theorem-bearing definition carries the adjudication's
vocabulary).

Retained as a refuted candidate: `static_oracle_jams` pins its failure
in the kernel, and `sendProj` (Oracle/Order.lean) is the projection
that provably works. -/
def demandOrder (sk : Skel) (d : Party) : List Nat :=
  (Sched.scheduleE sk).filterMap fun e =>
    match e with
    | (.wire p h, false, _) => if p == d then some h else none
    | _ => none

/-- The π-eligibility wedge: `Gen.genSkelM0 2859` materialized (module
doc) — a 19-scope, rootH-4, margin-0 skeleton on which the
receive-projection pusher deadlocks at every capacity tested. -/
def piWedge : Skel :=
  { scopes :=
      [ sc .D 4 [1, 2],
        sc .D 3 [3, 4, 5, 6, 7, 8],
        sc .D 3 [9],
        sc .R 2 [], sc .R 2 [],
        sc .D 2 [],
        sc .D 2 [10, 11, 12, 13, 14],
        sc .R 2 [], sc .R 2 [],
        sc .D 2 [15, 16, 17, 18],
        sc .D 1 [] (leafReqs := 5),
        sc .D 1 [] (leafReqs := 6),
        sc .D 1 [] (leafReqs := 2),
        sc .D 1 [] (leafReqs := 7),
        sc .R 1 [],
        sc .D 1 [] (leafReqs := 5),
        sc .D 1 [] (leafReqs := 2),
        sc .R 1 [],
        sc .D 1 [] (leafReqs := 7) ]
    rootH := 4, fan := 7, capLevel := 4 }

/-- The witness is inside the theorem class: well-formed, so the jam
below indicts the schedule, not the skeleton. -/
theorem piWedge_wellFormed : piWedge.wellFormed = true := by decide

/-- The witness satisfies the margin-0 capacity discipline: the un-muxed
session is kernel-proven deadlock-free (`Sched.deadlock_free`), and the
ORACLE completes the muxed one (`piWedge_oracle_completes`) — only the
receive-projection order is at fault. -/
theorem piWedge_margin0 : ∀ s, piWedge.dCount s ≤ piWedge.capLevel :=
  margin0_sound (by decide)

-- =============================================== the static-order jam

set_option maxRecDepth 400000 in
set_option maxHeartbeats 16000000 in
/-- The adjudication's primary T5 form is FALSE: the receive-projection
pusher `ofSchedule (demandOrder …)` — full bidirectional skeleton
knowledge, τ's own consumption order, precomputed — deadlocks `piWedge`
at C = 1 (the P2 π-eligibility failure, kernel tier; STAGE0-GATES.md
P2, MUX-PROGRESS.md log 2026-07-21).

Read with `oracle_deadlock_free_of_muxInv` and
`piWedge_oracle_completes`, this sharpens the muxprobe finding ("even
full skeleton knowledge does not make a non-adaptive schedule live"):
the oracle of record is EQUALLY non-adaptive and equally informed — a
fixed list indexed by own push count — and is live on the whole class.
Neither adaptivity nor information is the liveness ingredient; the
ORDER is. Pushing in consumption order jams because commit dependencies
can force a frame early whose consumption comes late (cross-stream
skew, the rand5016 anatomy); pushing in send order is safe because the
per-stream demux slots absorb exactly that skew. -/
theorem static_oracle_jams :
    mstuck piWedge .impl 1 (ofSchedule (demandOrder piWedge .I))
      (ofSchedule (demandOrder piWedge .R))
      (mdrain piWedge .impl 1 (ofSchedule (demandOrder piWedge .I))
        (ofSchedule (demandOrder piWedge .R)) 700 (init piWedge))
      = true := by
  decide

/-- `static_oracle_jams`, lifted to the refutation: the
receive-projection pusher is not deadlock-free — the negative control
T5's docstring points at. -/
theorem static_oracle_not_deadlockFree :
    ¬ MuxDeadlockFree piWedge .impl 1
      (ofSchedule (demandOrder piWedge .I))
      (ofSchedule (demandOrder piWedge .R)) := by
  intro h
  have hs := static_oracle_jams
  have hr := mdrain_reachable piWedge .impl 1
    (ofSchedule (demandOrder piWedge .I))
    (ofSchedule (demandOrder piWedge .R)) 700 (.init)
  rw [h _ hr] at hs
  exact Bool.false_ne_true hs

set_option maxRecDepth 400000 in
set_option maxHeartbeats 16000000 in
/-- The paired positive: the oracle of record completes the very
skeleton the receive-projection pusher jams, at the same capacity —
same information, same non-adaptivity, different order.
Message-denominated (Mux/Basic.lean, # The byte-denomination
caveat). -/
theorem piWedge_oracle_completes :
    muxCompletes piWedge .impl 1 (oracle .I) (oracle .R) 900 = true := by
  decide

-- ================================================ non-vacuity anchors

set_option maxRecDepth 100000 in
/-- The oracle completes the positive smoke skeleton at the minimum
capacity: the T5 statement is not vacuous on the smallest pin.
Message-denominated (Mux/Basic.lean, # The byte-denomination
caveat). -/
theorem smokeChain_oracle_completes :
    muxCompletes Pin.smokeChain .impl 1 (oracle .I) (oracle .R) 300
      = true := by
  decide

set_option maxRecDepth 100000 in
set_option maxHeartbeats 4000000 in
/-- The oracle completes `wedge` at C = 1 — the exact skeleton on which
every work-conserving pair deadlocks at every capacity
(`wc_impossibility`): the trichotomy's two halves pinned on one
witness. The oracle is not work-conserving precisely where it counts —
it idles the provision wall until the deep exchange's sends have gone
out in τ order. Message-denominated (Mux/Basic.lean, # The
byte-denomination caveat). -/
theorem wedge_oracle_completes :
    muxCompletes wedge .impl 1 (oracle .I) (oracle .R) 500 = true := by
  decide

-- ========================================================== T9: locality

/-- The T9 view pair, base half: a two-scope session whose single
height-1 dispute requests one leaf. -/
def viewPair : Skel :=
  { scopes := [sc .D 2 [1], sc .D 1 [] (leafReqs := 1)]
    rootH := 2, fan := 2, capLevel := 1 }

/-- The T9 view pair, mutated half: the same shape requesting TWO
leaves — a difference both parties' views erase. -/
def viewPair' : Skel :=
  { scopes := [sc .D 2 [1], sc .D 1 [] (leafReqs := 2)]
    rootH := 2, fan := 2, capLevel := 1 }

/-- Both halves are inside the theorem class. -/
theorem viewPair_wellFormed :
    viewPair.wellFormed = true ∧ viewPair'.wellFormed = true := by
  decide

/-- `LocalEq` is nondegenerate: two DISTINCT skeletons with equal views
on BOTH sides (MUX-ADJUDICATION §2.4's mandatory control — without
this, `LocalStrategy` would be vacuously satisfiable and T6's
class-relativity would say nothing). -/
theorem localEq_nondegenerate :
    viewPair.scopes ≠ viewPair'.scopes
      ∧ LocalEq .I viewPair viewPair' = true
      ∧ LocalEq .R viewPair viewPair' = true := by
  decide

/-- The demand order consumes remote structure: a `LocalEq` pair with
different receive projections (MUX-ADJUDICATION §3 T5's
`oracle_not_local`, stated for the adjudicated π_d form). The
responder's supply run is one frame longer on the mutated half — a
difference its own view erases. -/
theorem demandOrder_not_local :
    ∃ sk sk', LocalEq .R sk sk' = true
      ∧ demandOrder sk .R ≠ demandOrder sk' .R :=
  ⟨viewPair, viewPair', by decide, by decide⟩

/-- The oracle of record consumes remote structure at PROJECTION
strength: on the same `LocalEq` pair, the send projections differ.

This pin is about the lists, not the strategy Prop — the strategy-level
refutation `¬ LocalStrategy .R (oracle .R)`, with its `Consistent`
certificates, is `oracle_not_localStrategy` below. -/
theorem oracle_not_local :
    ∃ sk sk', LocalEq .R sk sk' = true
      ∧ sendProj sk .R ≠ sendProj sk' .R :=
  ⟨viewPair, viewPair', by decide, by decide⟩

/-- The non-locality is behavioral, not just structural: an observation
history (two flush receipts) on which the oracle's outputs differ
across the view pair — one side idles where the other pushes its
third frame.

The history here is NOT `Consistent` (a reachable push carries earlier
`.act` entries), so this pin does not by itself refute
`LocalStrategy`; the certified form is `oracle_not_localStrategy`. -/
theorem oracle_not_local_behavioral :
    ∃ tr : List MObs, oracle .R viewPair tr ≠ oracle .R viewPair' tr :=
  ⟨[.pushed 2, .pushed 0], by decide⟩

-- ============================== T9, at strategy strength (phase-4 F2)

/-- One driving prefix, enabled on BOTH halves of the view pair: open
both ends, exchange the openings, walk the single dispute down to the
responder's first supply commit, and push it. Every responder-visible
event along the way is identical on the two skeletons — the leaf-count
mutation only lengthens the supply run that comes AFTER. -/
def nonlocalActs : List MAction :=
  [.base (.iopenChoose .wire),
   .push .I,
   .base (.iopenChoose .query),
   .base .iopenFire,
   .deliver .I,
   .base .ropenRecv,
   .base (.ropenChoose .wire),
   .push .R,
   .base (.ropenChoose .res),
   .base .ropenFire,
   .base (.ropenChoose .query),
   .base .ropenFire,
   .base .finRes,
   .deliver .R,
   .base (.walkRecvWire (.I, 1)),
   .base (.walkRecvAsked (.I, 1)),
   .base (.walkCommit (.I, 1) (.wire 0)),
   .push .I,
   .base (.walkCommit (.I, 1) (.res 0)),
   .base (.walkFire (.I, 1)),
   .base (.walkCommit (.I, 1) (.query 0)),
   .base (.walkFire (.I, 1)),
   .deliver .I,
   .base (.walkRecvWire (.R, 0)),
   .base (.walkRecvAsked (.R, 0)),
   .base (.walkCommit (.R, 0) (.wire 0)),
   .push .R]

/-- The responder history `nonlocalActs` produces — verbatim the same
on `viewPair` and `viewPair'` (the two `decide`s inside
`oracle_not_localStrategy` check both replays), with two flush
receipts: exactly where the send projections part ways. -/
def nonlocalTrace : List MObs :=
  [.delivered 2,
   .act .ropenRecv,
   .act (.ropenChoose .wire),
   .pushed 2,
   .act (.ropenChoose .res),
   .act .ropenFire,
   .act (.ropenChoose .query),
   .act .ropenFire,
   .act .finRes,
   .delivered 1,
   .act (.walkRecvWire (.R, 0)),
   .act (.walkRecvAsked (.R, 0)),
   .act (.walkCommit (.R, 0) (.wire 0)),
   .pushed 0]

set_option maxRecDepth 100000 in
/-- The oracle is not a `LocalStrategy`, at the definition's full
strength: a trace `Consistent` with BOTH skeletons of a `LocalEq` pair
on which the oracle's outputs differ — after the same two pushes, the
`viewPair` oracle is done while the `viewPair'` oracle names a third
frame the responder's own view cannot see coming.

Both consistency certificates are kernel replays of the SAME driving
prefix (`nonlocalActs`), so the divergence survives the `Consistent`
guards that were added to protect σ*'s locality: T6's necessity
conjunct quantifies over a hypothesis the oracle genuinely fails. -/
theorem oracle_not_localStrategy : ¬ LocalStrategy .R (oracle .R) := by
  intro hloc
  have h := hloc viewPair viewPair' nonlocalTrace
    (by decide)
    (Consistent.of_obsOf .impl 1 bottomMostReady bottomMostReady
      nonlocalActs (by decide))
    (Consistent.of_obsOf .impl 1 bottomMostReady bottomMostReady
      nonlocalActs (by decide))
  exact absurd h (by decide)

end StreamingMirror.Mux
