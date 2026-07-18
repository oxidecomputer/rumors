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

# Finding #7: the parent-placement ledger gap (2026-07-17)

The second refutation family (`pdelay`/`parentTrap` below) is the
parent-delay finding: the six-ledger interface `{W, D1, D2, D3, D4}`
(`fullNoD5` — what `AxMode.full` was between the findings) left exactly
one out-of-trace-order freedom. A walk whose D children are all
resolved could commit a last-chunk query or trailing W wire with its
floating parent summary still unsent; the unsent parent starves the
assembler two heights up, the level towers back up and stop draining
the walk's own `upper` channel below, and the walk two stages down
wedges on its parent fire — never reaching the asked-receive that would
unjam the first walk's committed query. `parentTrap` wedges BOTH
flavors at once (walk (R,2) on the last-chunk query, walk (I,1) on a
trailing wire) on a well-formed, schedulable skeleton, so
`parentTrap_not_deadlockFree` refutes the pre-finding target statement
itself. The fix is the `d5` axiom (parent placement, on in today's
`AxMode.full`), matching the weave's §5 parent position (immediately
after the final resolution; first in an undisputed scope) — the order
the Rust encoder always emitted, unchecked; `d5_rejects_parentTrap` and
`pdelay_completes_full` pin refusal and non-vacuity exactly as for #6.

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

/-- The pre-finding-#6 interface: every axiom of the old ledger set, no
wire contiguity (and no parent placement, which postdates it too).
`⟨w, d1root, d1int, d2, d3, d4, d5, wireFirst⟩`. -/
def fullNoD4 : AxMode := ⟨true, true, true, true, true, false, false, false, false⟩

/-- The pre-finding-#7 interface: what `AxMode.full` was from finding #6
until the parent-delay finding — every ledger but parent placement.
`parentTrap` below kernel-checks that this set does NOT imply
deadlock-freedom. -/
def fullNoD5 : AxMode := ⟨true, true, true, true, true, true, false, false, false⟩

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
`pyramid1_not_deadlockFree` below (one D kid more, and the greedy run
jams; the event-DAG analysis upgrades that to no-schedule-completes,
checked not kernel-proven), this pins the bound as exact from both
sides. -/
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

/-- The strengthened interface refuses the trap: under `fullNoD5` (the
ledger set as of finding #6, with `d4`) the schedule's first
contiguity-violating wire commit — (R,2)'s wire B2, while B1 is
unresolved — fails its guard. Under today's `AxMode.full` the refusal
comes even earlier ((I,3)'s post-resolution query with the parent
unsent, a `d5` violation), so the trap is rejected by both. -/
theorem d4_rejects_trap :
    run jam fullNoD5 (init jam) trap = none ∧
    run jam .full (init jam) trap = none := by
  constructor <;> decide

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

/-- `firstM` over `Option` succeeds only through one of its elements. -/
private theorem firstM_eq_some {α β : Type _} {f : α → Option β} {b : β} :
    ∀ {l : List α}, l.firstM f = some b → ∃ a ∈ l, f a = some b := by
  intro l
  induction l with
  | nil => intro h; simp [List.firstM] at h
  | cons x xs ih =>
      intro h
      cases hfx : f x with
      | some b' =>
          simp [List.firstM, hfx] at h
          exact ⟨x, List.mem_cons_self .., by rw [hfx, h]⟩
      | none =>
          simp [List.firstM, hfx] at h
          obtain ⟨a, ha, hfa⟩ := ih h
          exact ⟨a, List.mem_cons_of_mem x ha, hfa⟩

/-- The greedy drain preserves reachability: every step it takes is the
application of some enabled action. -/
theorem drain_reachable (sk : Skel) (ax : AxMode) (fuel : Nat) :
    ∀ {s : State}, Reachable sk ax s →
      Reachable sk ax (drain sk ax fuel s) := by
  induction fuel with
  | zero => intro s h; exact h
  | succ n ih =>
      intro s h
      unfold drain
      cases hf : (allActions sk).firstM (fun a => apply sk ax a s) with
      | none => exact h
      | some s' =>
          obtain ⟨a, -, ha⟩ := firstM_eq_some hf
          exact ih (.step a h ha)

set_option maxRecDepth 16000 in
/-- The greedy run on `pyramid 1` jams under the FULL axiom mode: one D
child past the `Skel.schedulable` bound, the drained state is stuck.
Kernel-`decide`d, like `jam_completes_full` — no schedule witness is
needed because pyramid 1 jams greedily. -/
theorem pyramid1_stuck :
    stuck (Pin.pyramid 1) .full
      (drain (Pin.pyramid 1) .full 600 (init (Pin.pyramid 1))) = true := by
  decide

/-- The `schedulable` hypothesis of the Phase C target is load-bearing,
as a THEOREM: `pyramid 1` is well-formed (`pyramid1_not_schedulable`
pins the wellFormed half in Statement.lean) yet not deadlock-free under
the full mode — dropping the hypothesis makes the target statement
false. The stronger claim that NO schedule completes pyramid 1 (not
just the greedy one) is the event-DAG cyclicity analysis, checked by
`lake exe eventdag`, not kernel-proven. -/
theorem pyramid1_not_deadlockFree :
    ¬ DeadlockFree (Pin.pyramid 1) AxMode.full := by
  intro h
  have hs := pyramid1_stuck
  have hr := drain_reachable (Pin.pyramid 1) .full 600
    (Reachable.init (sk := Pin.pyramid 1) (ax := AxMode.full))
  rw [h _ hr] at hs
  exact Bool.false_ne_true hs

-- =========================== finding #7: the parent-placement ledger gap

/-- root(D,4)─B(D,3)─{t1(D,2, childless), t2(D,2, childless), t3(D,2,
six R kids)}; every height-1 scope R-kind, no leaf requests. BFS ids;
well-formed AND schedulable (`pdelay_on_boundary`) — the refutation
sits inside the target theorem's hypothesis class. The shape is the
minimal parent-delay wedge: three D siblings put B exactly ON the
`dCount = capLevel + 2` bound (two D kids complete at ANY chunk size —
the tower always unwinds), the childless t1/t2 make asm(R,2) block on
its second summary send immediately, and the heavy LAST chunk (six is
minimal at `capLevel = 1`; five completes) keeps walk (R,2) owing
queries after the backed-up tower has wedged walk (R,0) on its parent
fire. -/
def pdelay : Skel :=
  { scopes :=
      [ ⟨.D, 4, [1], 0⟩,                -- 0: root
        ⟨.D, 3, [2, 3, 4], 0⟩,          -- 1: B
        ⟨.D, 2, [], 0⟩,                 -- 2: t1
        ⟨.D, 2, [], 0⟩,                 -- 3: t2
        ⟨.D, 2, [5, 6, 7, 8, 9, 10], 0⟩, -- 4: t3 (the heavy last chunk)
        ⟨.R, 1, [], 0⟩, ⟨.R, 1, [], 0⟩, ⟨.R, 1, [], 0⟩,
        ⟨.R, 1, [], 0⟩, ⟨.R, 1, [], 0⟩, ⟨.R, 1, [], 0⟩ ],
    rootH := 4, fan := 6, capLevel := 1 }

open Party in
/-- The parent-delay schedule: the greedy parent-delaying adversary's
run on `pdelay` (each walk commits any choosable wire/res/query before
its parent), transcribed action-for-action. Under `fullNoD5` every
guard passes and the run ends stuck (`parentTrap_stuck`): walk (R,2)
committed t3's next chunk query with its parent unsent (jamming the
cap-1 `asked R 0`), walk (I,1) committed a trailing W wire with its
parent unsent, and walk (R,0) sits committed on a parent fire the
backed-up level tower will never drain. Under today's `AxMode.full`
the run is refused at (I,3)'s first root query — committed after the
root's only D child resolved with the parent unsent, the first `d5`
violation (`d5_rejects_parentTrap`). -/
def parentTrap : List Action := [
  -- openers run to completion
  .iopenChoose .wire, .iopenFire,
  .iopenChoose .query, .iopenFire,
  .ropenRecv,
  .ropenChoose .wire, .ropenFire,
  .ropenChoose .res, .ropenFire,
  .ropenChoose .query, .ropenFire,
  .finRes,
  -- walk (I,3): root scope — wire B, res B, then queries with the
  -- parent delayed (the adversary's signature move)
  .walkRecvWire (I, 3), .walkRecvAsked (I, 3),
  .walkCommit (I, 3) (.wire 0), .walkFire (I, 3),
  .walkCommit (I, 3) (.res 0), .walkFire (I, 3),
  .walkCommit (I, 3) (.query 0), .walkFire (I, 3),
  .walkCommit (I, 3) (.query 0),
  -- walk (R,2): scope B — wire t1, res t1 (childless: no queries)
  .walkRecvWire (R, 2), .walkRecvAsked (R, 2),
  .walkCommit (R, 2) (.wire 0), .walkFire (R, 2),
  -- walk (I,1): scope t1 — childless, parent only
  .walkRecvWire (I, 1), .walkRecvAsked (I, 1),
  .walkFire (I, 3),
  .walkCommit (I, 3) (.query 0),
  .walkCommit (I, 1) .parent, .walkFire (I, 1),
  .walkCommit (R, 2) (.res 0), .walkFire (R, 2),
  .walkCommit (R, 2) (.wire 1), .walkFire (R, 2),
  -- walk (I,1): scope t2 — parent only again
  .walkRecvWire (I, 1), .walkRecvAsked (I, 1),
  .walkFire (I, 3),
  -- (I,3)'s queries are done; its parent is its last obligation
  .walkCommit (I, 3) .parent, .walkFire (I, 3),
  .walkCloseWire (I, 3), .walkCloseAsked (I, 3),
  .walkCommit (I, 1) .parent,
  .walkCommit (R, 2) (.res 1),
  -- the I-side tower drains what exists of the I material
  .asmRecvRes (I, 2),
  .walkFire (I, 1),
  .asmSend (I, 2), .asmRecvRes (I, 2),
  .asmRecvRes (I, 3), .asmRecvLevel (I, 3),
  .asmSend (I, 2),
  .asmRecvLevel (I, 3),
  .asmRecvRes (I, 4),
  .asmRecvRes (R, 2),
  .walkFire (R, 2),
  -- walk (R,2): wire t3, res t3 — now every D child is resolved and
  -- the parent-delaying run enters t3's six-query chunk parentless
  .walkCommit (R, 2) (.wire 2), .walkFire (R, 2),
  .walkRecvWire (I, 1), .walkRecvAsked (I, 1),
  .walkCommit (I, 1) (.wire 0), .walkFire (I, 1),
  .walkCommit (I, 1) (.wire 1),
  .walkCommit (R, 2) (.res 2),
  .walkRecvWire (R, 0),
  .walkFire (I, 1),
  .walkCommit (I, 1) (.wire 2),
  .asmSend (R, 2), .asmRecvRes (R, 2),
  .walkFire (R, 2),
  .walkCommit (R, 2) (.query 2), .walkFire (R, 2),
  .walkCommit (R, 2) (.query 2),
  -- walk (R,0)'s leaf scopes: recv wire + asked, then a parent fire
  -- racing the level tower; asm(R,2) blocks on its second summary send
  -- (asm(R,3) waits on (R,2)'s unsent parent), and the tower backs up
  .walkRecvAsked (R, 0),
  .walkFire (R, 2),
  .walkCommit (R, 2) (.query 2),
  .walkCommit (R, 0) .parent, .walkFire (R, 0),
  .walkRecvWire (R, 0),
  .walkFire (I, 1),
  .walkCommit (I, 1) (.wire 3),
  .walkRecvAsked (R, 0),
  .walkFire (R, 2),
  .walkCommit (R, 2) (.query 2),
  .walkCommit (R, 0) .parent,
  .asmRecvRes (R, 1),
  .walkFire (R, 0),
  .walkRecvWire (R, 0),
  .walkFire (I, 1),
  .walkCommit (I, 1) (.wire 4),
  .walkRecvAsked (R, 0),
  .walkFire (R, 2),
  .walkCommit (R, 2) (.query 2),
  .walkCommit (R, 0) .parent,
  .asmSend (R, 1), .asmRecvRes (R, 1),
  .walkFire (R, 0),
  .walkRecvWire (R, 0),
  .walkFire (I, 1),
  -- walk (I,1)'s trailing-wire flavor: wire 5 committed, parent unsent
  .walkCommit (I, 1) (.wire 5),
  .walkRecvAsked (R, 0),
  .walkFire (R, 2),
  -- walk (R,2)'s last-chunk-query flavor: committed, `asked R 0` full
  .walkCommit (R, 2) (.query 2),
  -- walk (R,0): parent committed; `upper R 0` never drains again
  .walkCommit (R, 0) .parent
]

/-- The trap skeleton is inside the theorem's skeleton class — AND
inside the `schedulable` bound, exactly ON it (B disputes
`capLevel + 2` children): unlike `pyramid 1`, nothing about `pdelay`'s
capacity forces a jam, so the stuck run below indicts the ledger set,
not the skeleton. -/
theorem pdelay_on_boundary :
    pdelay.wellFormed = true ∧ pdelay.schedulable = true ∧
    pdelay.dCount 1 = pdelay.capLevel + 2 := by
  refine ⟨by decide, by decide, by decide⟩

set_option maxRecDepth 8000 in
/-- Under the pre-finding interface (every ledger but parent placement)
the parent-delay schedule executes fully and ends in a non-terminal
state where no action is enabled. -/
theorem parentTrap_stuck :
    (match run pdelay fullNoD5 (init pdelay) parentTrap with
     | some s => stuck pdelay fullNoD5 s && !terminal pdelay s
     | none => false) = true := by decide

/-- Finding #7, as a theorem: the six-ledger interface
`{W, D1, D2, D3, D4}` — the exact `AxMode.full` between findings #6 and
#7 — does NOT imply deadlock-freedom on schedulable skeletons: nothing
forbade a walk from committing past its floating parent once its D
children were resolved. This refutes the pre-finding target statement
`wellFormed → schedulable → DeadlockFree sk full` at `sk := pdelay`. -/
theorem parentTrap_not_deadlockFree : ¬ DeadlockFree pdelay fullNoD5 := by
  intro h
  have key := parentTrap_stuck
  cases hr : run pdelay fullNoD5 (init pdelay) parentTrap with
  | none => simp only [hr] at key; exact Bool.false_ne_true key
  | some s =>
      simp only [hr] at key
      have hns := h s (run_reachable pdelay fullNoD5 hr)
      simp [hns] at key

set_option maxRecDepth 8000 in
/-- The strengthened interface refuses the trap: under today's
`AxMode.full` the schedule's first parent-delaying commit — (I,3)'s
root query, after its only D child resolved with the parent unsent —
fails the `d5` guard. -/
theorem d5_rejects_parentTrap :
    run pdelay .full (init pdelay) parentTrap = none := by decide

set_option maxRecDepth 8000 in
/-- Non-vacuity of the fix: the trap skeleton still completes under the
strengthened mode — `d5` removes schedules, not sessions. -/
theorem pdelay_completes_full :
    terminal pdelay (drain pdelay .full 400 (init pdelay)) = true := by
  decide

end StreamingMirror.Control
