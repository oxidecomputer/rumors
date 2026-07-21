/-
The T3 control suite: kernel-checked pins that each hypothesis of the
impossibility theorem is load-bearing (MUX-ADJUDICATION.md §3, T3
controls; the close-guard must-fail pin is T0's negative control).

Every control here is a `decide` — no `native_decide` — in the
Controls.lean idiom: a concrete run, a kernel verdict on its final
state, and where the verdict refutes a Prop, the `mdrain_reachable` /
`mrun_reachable` glue. The suite brackets `wc_impossibility`
(Mux/Proofs/WcImpossibility.lean) from every side the adjudication
names:

- **work-conservation is load-bearing** — the shipped work-conserving
  policy jams `wedge` (`wedge_not_deadlockFree`, the faithfulness pin,
  deadlock doc §7 item 4), while a hand-built idling strategy completes
  the same skeleton at the same capacity (`wedge_idler_completes`);
- **the one-slot demux state is load-bearing** — under an
  unbounded-slot demux variant the same work-conserving pair completes
  `wedge` (`wedge_unboundedSlot_completes`, the option-C escape: the
  jam is slot occupation + FIFO burial, never pipe capacity);
- **`1 ≤ C` is load-bearing** — at C = 0 even a completing session
  jams trivially (`smokeChain_C0_not_deadlockFree`), so the
  impossibility statement must exclude C = 0 or claim nothing;
- **the strengthened close is load-bearing** — with the F8 no-in-flight
  conjunct removed, a run reaches a base-terminal state with a frame
  still in flight, and even a full `mterminal` verdict over a frame
  parked forever in a demux slot (`noF8_bogus_terminal`,
  `noF8_bogus_mterminal`); the real guard refuses the same schedule
  (`f8_rejects_gadgetTrap`).

# The F8 boundary finding (recorded here so the pin is read right)

On a WELL-FORMED skeleton the F8 conjunct never bites: a wire close
requires the consumer past its last scope, so its receive count equals
the stage length, which equals the producer's total sends on that
channel (the `wellFormed` BFS-alignment conjunct is exactly this
count identity) — every pushed frame was delivered and consumed, and
the producer's pipe cannot hold a frame of the closed channel. The
counting protection fails exactly where `wellFormed` does: `gadget`
below violates one conjunct (leaf requests on an R-kind scope), its
producer sends more frames than its consumer consumes, and the
unstrengthened close then certifies a session "complete" over a frame
that never arrived. The pin therefore guards the guard: `mstuck` and
`mterminal` are total over arbitrary `Skel`, the theorem statements
quantify over strategy-reachable states of ANY skeleton a caller
writes down, and the close semantics must not depend on well-formedness
for its soundness — attack-refute F8's "Terminal/stuck classification
drifts at the session tail" is this drift.
-/
import StreamingMirror.Mux.Instances

namespace StreamingMirror.Mux.Control

open Model
open Mux

-- ===================================================== script strategies

/-- Push the fixed script's next stream height, indexed by own flush
count; idle once the script is exhausted.

The hand-built strategy shape of the control table: entry `k` names the
height of the machine's `k+1`-st push (`.pushed` observations count the
completed ones), so the strategy pins its own push ORDER and otherwise
idles — while the scripted height is uncommitted, the mux declines
every other enabled frame. Deterministic, local (it never reads the
skeleton), and definitionally outside `WorkConserving` whenever the
script withholds an enabled frame. -/
def pushScript (script : List Nat) : Strategy := fun _ tr =>
  script[tr.countP fun o => match o with | .pushed _ => true | _ => false]?

-- ==================== (a) the faithfulness pin, upgraded to a refutation

/-- The shipped mux policy is NOT deadlock-free on `wedge` at C = 1: the
kernel-decided jam (`wedge_bottomMostReady_jams`, Mux/Instances.lean)
lifted through `mdrain_reachable` to refute `MuxDeadlockFree` itself —
the model twin of the committed Rust regression (MUX-ADJUDICATION §1.2,
§3 T3 controls; deadlock doc §7 item 4). -/
theorem wedge_not_deadlockFree :
    ¬ MuxDeadlockFree wedge .impl 1 bottomMostReady bottomMostReady := by
  intro h
  have hs := wedge_bottomMostReady_jams
  have hr := mdrain_reachable wedge .impl 1 bottomMostReady bottomMostReady
    200 (.init)
  rw [h _ hr] at hs
  exact Bool.false_ne_true hs

-- ============================ (b) an idling strategy completes the wedge

/-- The initiator's completing push order on `wedge`: opening, the
scope-1 dispute reply, then the whole deep exchange (streams 3 and 1)
BEFORE the six provisions.

Where the work-conserving policy is forced to flood the provision wall,
this script idles the wall at exactly the fill observations — the mux
declines the committed provision until the deep frames have flushed —
which is the σ* move in its simplest hard-coded form (MUX-ADJUDICATION
§1.2 point 3: the right to idle, not frame choice, is the entire
frontier). -/
def wedgeIdlerI : Strategy := pushScript [6, 5, 3, 1, 5, 5, 5, 5, 5, 5]

/-- The responder's push order on `wedge`: opening, then one frame per
stage as the descent reaches it — the order every strategy is forced to
anyway (each consultation is a singleton). -/
def wedgeIdlerR : Strategy := pushScript [6, 4, 2, 0]

set_option maxRecDepth 16000 in
/-- Work-conservation is load-bearing in T3: a hand-built IDLING pair
completes `wedge` at the minimum capacity — same skeleton, same C, same
greedy scheduler as the jam pin; only the strategies' right to idle
differs (MUX-ADJUDICATION §3, T3 controls: "a hand-built idling
strategy completes wedge").

With `wedge_not_deadlockFree` this splits the deadlock's cause off the
skeleton and onto the strategy class: `wc_impossibility`'s hypothesis
is not decoration. Message-denominated (Mux/Basic.lean, # The
byte-denomination caveat). -/
theorem wedge_idler_completes :
    muxCompletes wedge .impl 1 wedgeIdlerI wedgeIdlerR 800 = true := by
  decide

-- ===================== (c) the unbounded-demux-slot variant completes it

/-- The unbounded-slot demux variant: `deliver` moves the pipe head into
the wire channel WITHOUT the slot-empty guard, so per-stream demux
state grows without bound; every other arm is the harness of record.

This is the option-C escape hatch of the boundedness caveat
(MUX-ADJUDICATION §1.2 point 2): with unbounded endpoint demux state
the pipe always drains, FIFO burial is impossible, and no send-order
impossibility can exist — C1 is trivially false without a bound on
demux state. Defined locally: it is a control's foil, not a semantics
anyone ships (the Rust demux is the one-slot handoff family,
incoming.rs:60-92). -/
def applyU (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy)
    (a : MAction) (s : MState) : Option MState :=
  match a with
  | .deliver p =>
      match s.pipe p with
      | c :: rest =>
          some { base := { s.base with chan := bump s.base.chan c 1 }
                 pipe := fun q => if q == p then rest else s.pipe q
                 hist := recordObs s.hist p.other
                   (.delivered (wireHeight c)) }
      | [] => none
  | a => apply sk ax C σI σR a s

/-- Greedy drain of the unbounded-slot variant; the `mdrain` pattern
over `applyU`. -/
def mdrainU (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy) :
    Nat → MState → MState
  | 0, s => s
  | fuel + 1, s =>
      match (allMActions sk).firstM (fun a => applyU sk ax C σI σR a s) with
      | some s' => mdrainU sk ax C σI σR fuel s'
      | none => s

set_option maxRecDepth 16000 in
set_option maxHeartbeats 1000000 in
/-- The one-slot demux state is load-bearing in T3: under the
unbounded-slot variant the SAME work-conserving pair that jams `wedge`
(`wedge_bottomMostReady_jams`) completes it at the same capacity
(MUX-ADJUDICATION §3, T3 controls: "unbounded-slot variant completes
wedge under bottomMostReady").

With capacity flatness (the probe's minimal w = 4 instance across
C ∈ {1..16}, §1.2 point 2 — the landed `wedge` literal is the
6-provision committed-regression shape) this pins the jam mechanism as
slot occupation + FIFO burial: relax the slot and the impossibility
dissolves; widen the pipe and it does not. Message-denominated
(Mux/Basic.lean, # The byte-denomination caveat). -/
theorem wedge_unboundedSlot_completes :
    mterminal wedge
      (mdrainU wedge .impl 1 bottomMostReady bottomMostReady 800
        (init wedge)) = true := by
  decide

-- ============================================== (d) the C = 0 vacuity pin

set_option maxRecDepth 16000 in
/-- At C = 0 the muxed `smokeChain` — the positive smoke skeleton that
completes at C = 1 (`smokeChain_mux_completes`) — jams at once: no push
can ever fire into a zero-capacity pipe. -/
theorem smokeChain_C0_stuck :
    mstuck Pin.smokeChain .impl 0 bottomMostReady bottomMostReady
      (mdrain Pin.smokeChain .impl 0 bottomMostReady bottomMostReady 50
        (init Pin.smokeChain)) = true := by
  decide

/-- The `1 ≤ C` hypothesis of T3 is load-bearing: at C = 0 deadlock is
capacity-vacuous — even the completing smoke skeleton jams under the
shipped policy — so an impossibility statement admitting C = 0 would
indict nothing about scheduling (MUX-ADJUDICATION §3, T3 controls:
"C = 0 vacuity"). -/
theorem smokeChain_C0_not_deadlockFree :
    ¬ MuxDeadlockFree Pin.smokeChain .impl 0
      bottomMostReady bottomMostReady := by
  intro h
  have hs := smokeChain_C0_stuck
  have hr := mdrain_reachable Pin.smokeChain .impl 0 bottomMostReady
    bottomMostReady 50 (.init)
  rw [h _ hr] at hs
  exact Bool.false_ne_true hs

-- ================================== (e) the close-guard must-fail pin

/-- The F8 gadget: a three-scope skeleton whose ONLY well-formedness
violation is a leaf request on an R-kind height-1 scope, so the
producer walk sends two `wire R 0` frames while the absorber consumes
`totalLeafReqs = 1` — the count mismatch the module doc's boundary
finding names, and the smallest shape on which the unstrengthened
close can certify completion over an in-flight frame. -/
def gadget : Skel :=
  { scopes := [ ⟨.D, 2, [1, 2], 0⟩,
                ⟨.D, 1, [], 1⟩,
                ⟨.R, 1, [], 1⟩ ],
    rootH := 2, fan := 2, capLevel := 1 }

/-- The gadget sits OUTSIDE the theorem class, deliberately: on
well-formed skeletons the close is counting-protected (module doc), so
the F8 conjunct's bite is exactly at — and the pin must live at — the
boundary the definitions do not police. -/
theorem gadget_not_wellFormed : gadget.wellFormed = false := by decide

/-- The harness of record with ONLY the F8 no-in-flight conjunct
removed from the wire close-receives: the base arms delegate straight
to `Model.apply` (still minus wire fires); push and deliver are
untouched. The exact pre-strengthening semantics, for the must-fail
replay. -/
def applyNoF8 (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy)
    (a : MAction) (s : MState) : Option MState :=
  match a with
  | .base a =>
      if isWireFire s.base a then none
      else
        (Model.apply sk ax a s.base).map fun b =>
          { s with base := b
                   hist := recordObs s.hist (actionParty a) (.act a) }
  | a => apply sk ax C σI σR a s

/-- `mrun` over the weakened semantics. -/
def mrunNoF8 (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy)
    (s : MState) : List MAction → Option MState
  | [] => some s
  | a :: rest =>
      match applyNoF8 sk ax C σI σR a s with
      | some s' => mrunNoF8 sk ax C σI σR s' rest
      | none => none

/-- The gadget's push scripts: each side's forced order (opening, then
the per-stage frames — including the producer's SECOND `wire R 0`
frame, the one the consumer never absorbs). -/
def gadgetI : Strategy := pushScript [2, 1, 1]

/-- Responder twin of `gadgetI`. -/
def gadgetR : Strategy := pushScript [2, 0, 0]

open Party in
/-- The bogus-close schedule: a full session on `gadget` in which the
producer's second `wire R 0` frame is pushed (index 57) but its
delivery is withheld, and the absorber's wire close fires (index 62)
with the frame still in the producer's pipe — legal without the F8
conjunct, refused with it (`f8_rejects_gadgetTrap`). Transcribed from
the greedy weakened-semantics drain, minus the final delivery. -/
def gadgetTrap : List MAction := [
  .base (.iopenChoose .wire),
  .push .I,
  .base (.iopenChoose .query),
  .base (.iopenFire),
  .deliver .I,
  .base (.ropenRecv),
  .base (.ropenChoose .wire),
  .push .R,
  .base (.ropenChoose .res),
  .base (.ropenFire),
  .base (.ropenChoose .query),
  .base (.ropenFire),
  .base (.ropenChoose .query),
  .base (.finRes),
  .deliver .R,
  .base (.walkRecvWire (I, 1)),
  .base (.walkRecvAsked (I, 1)),
  .base (.walkCommit (I, 1) (.wire 0)),
  .push .I,
  .base (.walkCommit (I, 1) (.res 0)),
  .base (.walkFire (I, 1)),
  .base (.walkCommit (I, 1) (.query 0)),
  .base (.walkFire (I, 1)),
  .base (.walkCommit (I, 1) (.wire 1)),
  .base (.asmRecvRes (I, 1)),
  .deliver .I,
  .base (.walkRecvWire (R, 0)),
  .base (.walkRecvAsked (R, 0)),
  .base (.ropenFire),
  .base (.walkCommit (R, 0) (.wire 0)),
  .push .I,
  .base (.walkCommit (I, 1) .parent),
  .base (.walkFire (I, 1)),
  .base (.walkCloseWire (I, 1)),
  .base (.walkCloseAsked (I, 1)),
  .base (.asmRecvRes (I, 2)),
  .push .R,
  .base (.walkCommit (R, 0) .parent),
  .base (.walkFire (R, 0)),
  .base (.asmRecvRes (R, 1)),
  .base (.asmSend (R, 1)),
  .base (.finRets),
  .deliver .I,
  .base (.walkRecvWire (R, 0)),
  .base (.walkRecvAsked (R, 0)),
  .base (.walkCommit (R, 0) (.wire 0)),
  .deliver .R,
  .base (.absorbRecvWire),
  .base (.absorbRecvAsked),
  .base (.absorbSend),
  .base (.asmRecvLevel (I, 1)),
  .base (.asmSend (I, 1)),
  .base (.asmClose (I, 1)),
  .base (.asmRecvLevel (I, 2)),
  .base (.asmSend (I, 2)),
  .base (.finRet),
  .base (.asmClose (I, 2)),
  .push .R,
  .base (.walkCommit (R, 0) .parent),
  .base (.walkFire (R, 0)),
  .base (.walkCloseWire (R, 0)),
  .base (.walkCloseAsked (R, 0)),
  .base (.absorbCloseWire),
  .base (.absorbCloseAsked),
  .base (.asmRecvRes (R, 1)),
  .base (.asmSend (R, 1)),
  .base (.finRets),
  .base (.asmClose (R, 1)) ]

/-- Without the F8 conjunct a BASE-terminal state with a frame in
flight is reachable: the schedule runs to completion in every base
process's eyes while the producer's pipe still carries the frame the
absorber closed over. `mterminal`'s pipes-drained conjunct still
catches this state — the two guards are independent defenses, and this
pin shows the second is live when the first is removed (T0's negative
control, MUX-ADJUDICATION §3). -/
theorem noF8_bogus_terminal :
    (match mrunNoF8 gadget .impl 1 gadgetI gadgetR (init gadget)
        gadgetTrap with
     | some s =>
         Model.terminal gadget s.base && !(s.pipe .R).isEmpty &&
           !mterminal gadget s
     | none => false) = true := by
  decide

/-- One delivery later even `mterminal` is fooled: the in-flight frame
lands in its demux slot AFTER the close cascade, and the muxed terminal
verdict — base terminal, both pipes drained — goes TRUE over a frame no
consumer will ever read. The weakened close does not merely reorder the
tail; it lets the harness certify completion of a session that dropped
a frame. -/
theorem noF8_bogus_mterminal :
    (match mrunNoF8 gadget .impl 1 gadgetI gadgetR (init gadget)
        (gadgetTrap ++ [.deliver .R]) with
     | some s =>
         mterminal gadget s &&
           s.base.chan (Chan.wire Party.R 0) == 1
     | none => false) = true := by
  decide

/-- The strengthened close refuses the trap: under the harness of
record the trap schedule is disabled — some action of the script fails
its guard, where the F8-free variant runs it to a bogus terminal.

What the kernel decides here is only `mrun … = none` (the script
contains two F8-guarded closes, and this pin does not name which one
refuses); that the refusal is the F8 conjunct — an in-flight
`wire R 0` frame visible in the producer's pipe — is pinned by the
CONTRAST with `noF8_bogus_terminal`/`noF8_bogus_mterminal`: same
script, close guard weakened, run completes bogusly. The must-fail
half of the pin: remove the conjunct and both bogus verdicts above
come back. -/
theorem f8_rejects_gadgetTrap :
    mrun gadget .impl 1 gadgetI gadgetR (init gadget) gadgetTrap
      = none := by
  decide

end StreamingMirror.Mux.Control
