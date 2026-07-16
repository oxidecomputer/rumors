/-
Negative controls: kernel-checked refutations that pin each axiom's
load-bearing role by exhibiting a concrete schedule that deadlocks
without it.

# Finding #6: the wire-contiguity ledger gap (2026-07-16)

The first control here refutes `DeadlockFree` for the PRE-d4 axiom
interface `{W, D1, D2, D3}` — the exact ledger set `Trace::assert_valid`
checked at the time. On the `jam` skeleton below, a schedule permitted by
those ledgers lets walk (R,2)'s wire stream outrun its query stream for
child B1 (wires B1, B2 sent; B1's queries 1-of-4 done), then commits to
B3's wire, which jams behind an unconsumed wire; walk (R,0) starves
waiting for (R,2)'s second asked, and walk (I,1) jams its fourth wire
behind (R,0): a three-process wait cycle, `stuck` with every axiom of
the old interface satisfied.

The Rust publisher cannot emit this order — `yield_resolve_query!`
(materialized.rs) publishes each disputed child's wire, resolution, and
dependent queries contiguously, calling it "progress-critical order" —
but nothing in the CHECKED interface said so: the ledgers enforced
sibling contiguity for resolutions (D3) with no wire-stream twin. The
fix is the `d4` axiom (`AxMode.d4`, on in `AxMode.full`) and the
matching sixth check in `assert_valid`; `d4_rejects_trap` pins that the
strengthened interface refuses this schedule at the first non-contiguous
wire, and `jam_completes_full` pins that the skeleton still completes
greedily under the strengthened mode.

Everything here is checked by kernel `decide` — no `native_decide`
trust in any refutation.
-/
import StreamingMirror.Statement

namespace StreamingMirror.Control

open Model

/-- root(D,4)─A(D,3)─{B1(D,2, four kids), B2(D,2, one kid),
B3(D,2, one kid)}; B1's first kid c1 is D with one leaf request, every
other height-1 scope is R-kind. BFS ids; well-formed
(`jam_wellFormed`). The uneven siblings are essential: the trap needs an
early D child owing ≥ 2 queries with ≥ 2 sibling wires after it. -/
def jam : Skel :=
  { scopes :=
      [ ⟨.D, 4, [1], 0⟩,          -- 0: root
        ⟨.D, 3, [2, 3, 4], 0⟩,    -- 1: A
        ⟨.D, 2, [5, 6, 7, 8], 0⟩, -- 2: B1
        ⟨.D, 2, [9], 0⟩,          -- 3: B2
        ⟨.D, 2, [10], 0⟩,         -- 4: B3
        ⟨.D, 1, [], 1⟩,           -- 5: c1 (D, one leaf request)
        ⟨.R, 1, [], 0⟩,           -- 6: c2
        ⟨.R, 1, [], 0⟩,           -- 7: c3
        ⟨.R, 1, [], 0⟩,           -- 8: c4
        ⟨.R, 1, [], 0⟩,           -- 9: c5 (B2's kid)
        ⟨.R, 1, [], 0⟩ ],         -- 10: c6 (B3's kid)
    rootH := 4, fan := 4, capLevel := 1 }

/-- The pre-finding interface: every axiom of the old ledger set, no
wire contiguity. `⟨w, d1root, d1int, d2, d3, d4, wireFirst⟩`. -/
def fullNoD4 : AxMode := ⟨true, true, true, true, true, false, false⟩

open Party in
/-- The trap schedule. Under `fullNoD4` every guard passes and the run
ends stuck (`jam_not_deadlockFree`); under `AxMode.full` the run is
refused at (R,2)'s wire-B2 commit — the first wire that would depart
while B1 still owes work (`d4_rejects_trap`). -/
def trap : List Action := [
  -- openers run to completion
  .iopenChoose .wire, .iopenFire,
  .ropenRecv,
  .iopenChoose .query, .iopenFire,
  .ropenChoose .wire, .ropenFire,
  .ropenChoose .res, .ropenFire,
  .finRes,
  .ropenChoose .query, .ropenFire,
  -- walk (I,3): recv root scope, publish wire A, res A
  .walkRecvWire (I, 3), .walkRecvAsked (I, 3),
  .walkCommit (I, 3) (.wire 0), .walkFire (I, 3),
  .walkCommit (I, 3) (.res 0), .walkFire (I, 3),
  -- walk (R,2): recv scope A
  .walkRecvWire (R, 2), .walkRecvAsked (R, 2),
  -- (I,3): query #1 (of 3) toward (I,1)
  .walkCommit (I, 3) (.query 0), .walkFire (I, 3),
  -- (R,2): wire B1
  .walkCommit (R, 2) (.wire 0), .walkFire (R, 2),
  -- walk (I,1): recv scope B1
  .walkRecvWire (I, 1), .walkRecvAsked (I, 1),
  -- (R,2): wire B2 (fills `wire R 2`), res B1, query B1 #1 (of 4),
  -- then commit wire B3: jammed until (I,1) consumes wire #2
  .walkCommit (R, 2) (.wire 1), .walkFire (R, 2),
  .walkCommit (R, 2) (.res 0), .walkFire (R, 2),
  .walkCommit (R, 2) (.query 0), .walkFire (R, 2),
  .walkCommit (R, 2) (.wire 2),
  -- (I,1): wire c1
  .walkCommit (I, 1) (.wire 0), .walkFire (I, 1),
  -- walk (R,0): scope c1 (recv wire+asked, publish leaf wire, parent)
  .walkRecvWire (R, 0), .walkRecvAsked (R, 0),
  .walkCommit (R, 0) (.wire 0), .walkFire (R, 0),
  .walkCommit (R, 0) .parent, .walkFire (R, 0),
  -- (I,1): wire c2; (R,0) consumes it, then needs asked #2 (never comes)
  .walkCommit (I, 1) (.wire 1), .walkFire (I, 1),
  .walkRecvWire (R, 0),
  -- (I,1): wire c3 (fills `wire I 1`), res c1, query c1 (leaf request),
  -- then commit wire c4: jammed until (R,0) consumes wire #3
  .walkCommit (I, 1) (.wire 2), .walkFire (I, 1),
  .walkCommit (I, 1) (.res 0), .walkFire (I, 1),
  .walkCommit (I, 1) (.query 0), .walkFire (I, 1),
  .walkCommit (I, 1) (.wire 3),
  -- drain the bottom of the pipeline as far as it goes
  .absorbRecvWire, .absorbRecvAsked, .absorbSend,
  .asmRecvRes (I, 1), .asmRecvLevel (I, 1), .asmSend (I, 1),
  .asmRecvRes (R, 1), .asmSend (R, 1),
  .asmRecvRes (R, 2), .asmRecvLevel (R, 2),
  .asmRecvRes (I, 3),
  -- (I,3): parent, query #2, then commit query #3: jammed behind #2
  .walkCommit (I, 3) .parent, .walkFire (I, 3),
  .walkCommit (I, 3) (.query 0), .walkFire (I, 3),
  .walkCommit (I, 3) (.query 0),
  .asmRecvRes (I, 4)
]

/-- The trap skeleton is inside the theorem's skeleton class: the
refutation below is not an artifact of ill-formedness. -/
theorem jam_wellFormed : jam.wellFormed = true := by decide

/-- `jam` sits exactly ON the schedulability boundary — scope A disputes
`capLevel + 2` children, the most `Skel.schedulable` admits — and
`jam_completes_full` below shows it completes there. Together with
`pyramid1_not_schedulable` (one D kid more, and no schedule completes),
this pins the bound as exact from both sides. -/
theorem jam_on_boundary :
    jam.schedulable = true ∧ jam.dCount 1 = jam.capLevel + 2 := by decide

/-- Under the pre-d4 interface, the trap schedule executes fully and
ends in a non-terminal state where no action is enabled. -/
theorem trap_stuck :
    (match run jam fullNoD4 (init jam) trap with
     | some s => stuck jam fullNoD4 s && !terminal jam s
     | none => false) = true := by decide

/-- Finding #6, as a theorem: the old ledger interface `{W, D1, D2, D3}`
does NOT imply deadlock-freedom — the wire ledger never forbade a wire
departing while an earlier sibling owed dependent work. -/
theorem jam_not_deadlockFree : ¬ DeadlockFree jam fullNoD4 := by
  intro h
  have key := trap_stuck
  cases hr : run jam fullNoD4 (init jam) trap with
  | none => simp only [hr] at key; exact Bool.false_ne_true key
  | some s =>
      simp only [hr] at key
      have hns := h s (run_reachable jam fullNoD4 hr)
      simp [hns] at key

/-- The strengthened interface refuses the trap: under `AxMode.full`
(with `d4`) the schedule's first contiguity-violating wire commit —
(R,2)'s wire B2, while B1 is unresolved — fails its guard. -/
theorem d4_rejects_trap :
    run jam .full (init jam) trap = none := by decide

/-- Greedy scheduler, for completion pins: take the first enabled
action until quiescent. -/
def drain (sk : Skel) (ax : AxMode) : Nat → State → State
  | 0, s => s
  | fuel + 1, s =>
      match (allActions sk).firstM (fun a => apply sk ax a s) with
      | some s' => drain sk ax fuel s'
      | none => s

set_option maxRecDepth 8000 in
/-- Non-vacuity of the fix: the trap skeleton still completes under the
strengthened mode — `d4` removes schedules, not sessions. -/
theorem jam_completes_full :
    terminal jam (drain jam .full 300 (init jam)) = true := by decide

end StreamingMirror.Control
