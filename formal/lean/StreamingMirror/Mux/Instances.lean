/-
The T0 instances: the `wedge` witness skeleton, the margin-0 hypothesis
in bounded form, the shipped-mux strategy, and the kernel-decided smoke
run (MUX-ADJUDICATION.md §3, T0).

`wedge` is the committed Rust regression shape in skeleton terms
(prove-c1 §4.2's family at the Rust trigger point; the probe's
`regression_shape(provisions := 6, rootH := 6)`): the root disputes its
FIRST radix child — a chain descending disputed levels to a leaf
request — and takes six whole-subtree provisions behind it on the same
stream. One fixed skeleton defeats everything: T3 (`wc_impossibility`,
stage 2) says every work-conserving pair jams it at every capacity,
while the same skeleton is inside the base flagship's hypothesis class
(`wedge_wellFormed` + `wedge_margin0` + `Sched.deadlock_free`), so each
witness carries its own in-context proof that only the mux is at fault
(MUX-ADJUDICATION §2.6).

`bottomMostReady` is the shipped mux's policy (outgoing.rs:199-224's
reverse-index poll): deepest ready stream first, memoryless. Here it is
the pinned concrete `Strategy`, reconstructing its ready set from the
observation history alone (a machine's local state is a deterministic
function of its own action history — MUX-ADJUDICATION §2.3).

The two smoke theorems pin the whole harness end to end in the kernel,
one from each side: the muxed `smokeChain` at C = 1 under
`bottomMostReady × bottomMostReady` drains to `mterminal`, and the same
strategy pair on `wedge` drains to `mstuck` — the deadlock doc
§7-item-4 faithfulness control, here at drain tier (the
`¬ MuxDeadlockFree` corollary and the full control table are
Mux/Controls, stage 2).
-/
import StreamingMirror.Mux.Strategy
import StreamingMirror.Instances

namespace StreamingMirror.Mux

open Model
open Pin (sc)

-- ========================================================== the wedge shape

/-- The regression-shape witness (MUX-ADJUDICATION §1.2, §3 T0): rootH 6,
root fan 7 — the first radix child deep-disputed down to a leaf request,
six whole-subtree provisions behind it on the same stream.

BFS ids; well-formed and margin-0 (`wedge_wellFormed`, `wedge_margin0`),
so the un-muxed `.impl` system is inside the kernel-proven
`Sched.deadlock_free`.

Realizability bridge (stage-2 track D, corrected provenance): the
committed proptest seeds realize the wedge's *jam mechanism* on the old
transport, NOT its byte-exact shape. The bridge of record is
`src/tree/mirror/streaming/tests/wedge.rs`, which constructs a
deterministic tree pair and pins the decoded skeleton to this literal —
generator against the rootH-6 literal, session against the generator at
the protocol's real root height 32. -/
def wedge : Skel :=
  { scopes :=
      [ sc .D 6 [1, 2, 3, 4, 5, 6, 7], -- 0: root
        sc .D 5 [8],                   -- 1: the deep dispute, radix-first
        sc .R 5 [], sc .R 5 [],        -- 2, 3: the provision wall…
        sc .R 5 [], sc .R 5 [],        -- 4, 5
        sc .R 5 [], sc .R 5 [],        -- 6, 7
        sc .D 4 [9],                   -- 8: chain
        sc .D 3 [10],                  -- 9: chain
        sc .D 2 [11],                  -- 10: chain
        sc .D 1 [] (leafReqs := 1) ]   -- 11: the demanded leaf
    rootH := 6, fan := 7, capLevel := 1 }

-- ==================================================== margin 0, decidably

/-- The margin-0 capacity discipline as a bounded boolean check: every
scope's dispute count within the assembler capacity — the shipping
encoder's `FAN ≥ kids` stance, the hypothesis class of every mux
statement of record (MUX-ADJUDICATION §2.6).

Bounded over the scope table so it is kernel-`decide`-friendly;
`margin0_sound` recovers the flagship theorem's unbounded form
(`dCount` vanishes past the table). -/
def margin0 (sk : Skel) : Bool :=
  (List.range sk.scopes.length).all fun s => sk.dCount s ≤ sk.capLevel

/-- `dCount` vanishes past the scope table: an out-of-range id reads as
the childless degenerate scope (the `Skel.scope` totalization device). -/
theorem dCount_past_table (sk : Skel) {s : Nat}
    (h : sk.scopes.length ≤ s) : sk.dCount s = 0 := by
  simp [Skel.dCount, Skel.scope, List.getD, List.getElem?_eq_none h]

/-- The bounded check implies the flagship's unbounded margin-0
hypothesis: checking the table suffices. -/
theorem margin0_sound {sk : Skel} (h : margin0 sk = true) :
    ∀ s, sk.dCount s ≤ sk.capLevel := by
  intro s
  by_cases hs : s < sk.scopes.length
  · exact of_decide_eq_true
      (List.all_eq_true.mp h s (List.mem_range.mpr hs))
  · rw [dCount_past_table sk (Nat.le_of_not_lt hs)]
    exact Nat.zero_le _

-- ================================================================= T0 pins

/-- The wedge is inside the theorem's skeleton class: the impossibility
is not an artifact of ill-formedness. -/
theorem wedge_wellFormed : wedge.wellFormed = true := by decide

/-- The wedge satisfies the shipping margin-0 capacity discipline, so
the UN-muxed `.impl` session is kernel-proven deadlock-free
(`Sched.deadlock_free`): every stuck muxed state indicts the mux alone.

Stated in the flagship hypothesis's unbounded form, discharged through
the bounded `margin0` check (`decide` cannot cross an unbounded `∀`;
the adjudication's `by decide` is read as this bridge). -/
theorem wedge_margin0 : ∀ s, wedge.dCount s ≤ wedge.capLevel :=
  margin0_sound (by decide)

-- ================================================= the shipped-mux strategy

/-- Does machine `p`'s history show a committed, not-yet-pushed wire
obligation on stream `h`? True iff wire commits on `h` outnumber flush
receipts for `h`.

Sound because the committed hand is a one-slot device: each wire commit
(a `walkCommit … (.wire i)`, or the opener's `wire` choice at
`h = rootH`) is cleared by exactly one push before the next commit on
the same stream, and wire fires happen ONLY through `push` (the base
fire arms are disabled), so the count difference is the hand's occupancy.
The producing party needs no match: `hist p` contains only `p`'s own
actions. -/
def committedInHist (rootH : Nat) (tr : List MObs) (h : Nat) : Bool :=
  let commits := tr.countP fun o =>
    match o with
    | .act (.walkCommit pk (.wire _)) => pk.2 == h
    | .act (.iopenChoose .wire) => h == rootH
    | .act (.ropenChoose .wire) => h == rootH
    | _ => false
  let pushes := tr.countP fun o =>
    match o with
    | .pushed h' => h' == h
    | _ => false
  pushes < commits

/-- Bottom-most-ready: the deepest (lowest-height) stream with a
committed hand wins — outgoing.rs:199-224's reverse-index poll, the
shipped mux's policy and the T3 faithfulness pin's strategy.

Memoryless in spirit: a function of the hand occupancies, reconstructed
from the observation history (`committedInHist`). Work-conserving and
local, kernel-checked: `bottomMostReady_wc` (with the K/E-universe
twins `bottomMostReady_wcK`/`bottomMostReady_wcE`) and
`bottomMostReady_local` in Mux/Proofs/Inhabitation.lean discharge the
MUX-ADJUDICATION §2.4 obligations — the non-vacuity certificates for
every ∀-class impossibility over `WorkConserving`. -/
def bottomMostReady : Strategy := fun sk tr =>
  (List.range (sk.rootH + 1)).find? fun h =>
    committedInHist sk.rootH tr h

-- ============================================================ the smoke run

/-- One muxed-completion verdict, the `Pin.completes` shape transported:
the greedy strategy-driven drain reaches `mterminal` — base terminal
with both pipes drained — within `fuel` steps. -/
def muxCompletes (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy)
    (fuel : Nat) : Bool :=
  mterminal sk (mdrain sk ax C σI σR fuel (init sk))

-- Fast feedback while developing (the pins below are the record):
#eval [wedge.wellFormed, margin0 wedge, wedge.schedulable]
#eval muxCompletes Pin.smokeChain .impl 1 bottomMostReady bottomMostReady 300

set_option maxRecDepth 16000 in
/-- The harness smoke pin, end to end in the kernel: the muxed
`smokeChain` at C = 1 under the shipped policy on both sides drains to
`mterminal`.

This exercises every layer at once — strategy consultation and the
history reconstruction behind it, `push` under the pipe bound, the
head-of-line `deliver`, the demux-slot receive arms, the strengthened
closes (a close fires only after its stream's last frame has left the
pipe), and the pipes-drained terminal — so a definition drifting from
the probe's calibrated semantics fails the build here first. The
capacity is the tight one (C = 1): completion at the minimum pipe is
the strongest smoke. Message-denominated (Mux/Basic.lean, # The
byte-denomination caveat). -/
theorem smokeChain_mux_completes :
    muxCompletes Pin.smokeChain .impl 1 bottomMostReady bottomMostReady 300
      = true := by
  decide

set_option maxRecDepth 16000 in
/-- The smoke pin's negative twin, and the transcription-parity anchor:
the muxed `wedge` at C = 1 under the shipped policy on both sides drains
to a stuck non-terminal state — the model twin of the committed Rust
regression and the probe's minimized jam (MUX-ADJUDICATION §1.2 point 2).

Same greedy drain, same strategy pair as the positive smoke above; only
the skeleton differs — the harness reproduces the deadlock it exists to
indict, on the exact witness T3 quantifies over. The full negative-
control table (`¬ MuxDeadlockFree` via `mdrain_reachable`, the idler
completion, the unbounded-slot escape, the close-guard must-fail pin)
is Mux/Controls, stage 2. -/
theorem wedge_bottomMostReady_jams :
    mstuck wedge .impl 1 bottomMostReady bottomMostReady
      (mdrain wedge .impl 1 bottomMostReady bottomMostReady 200
        (init wedge)) = true := by
  decide

end StreamingMirror.Mux
