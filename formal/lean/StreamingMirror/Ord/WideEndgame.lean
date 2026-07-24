/-
The widened flagship: `Sched.deadlock_free_wide_anyOrder` — the
order-indifference metatheorem at every pointwise-widened capacity
vector. Assembles Ord/WidePreserve.lean's reachability invariant
(`invPWO_reachableWO`) with Ord/Wide.lean's progress engine
(`progressWO`), exactly as Proofs/Wide.lean assembles the base wide
flagship. The two corner pins close the square with the landed
flagships: at κ = `sk.cap` the wide-O system IS the floor O system
(`applyWO_cap`), so the metatheorem `Sched.deadlock_free_anyOrder`
re-derives; at `ord = .rf` it IS the base wide system (`applyWO_rf`),
so `Sched.deadlock_free_wide` re-derives.

Chain (ord, stage G, capstone): consumes Ord/WidePreserve.lean and
Ord/Wide.lean; concludes `Sched.deadlock_free_wide_anyOrder` and its
run-level corollary. Base mirror: Proofs/Wide.lean
(`Sched.deadlock_free_wide`). Map: PROGRESS.md §13.
-/
import StreamingMirror.Ord.WidePreserve

namespace StreamingMirror.Ord

open Model

variable (sk : Skel) (κ : Chan → Nat) (ax : AxMode) (ord : OrdMap)

-- ================================================= the rf corner transport

/-- All-reply-first wide-O reachability is base wide reachability,
right to left (`applyWO_rf` per step). -/
theorem reachableW_of_reachableWO_rf {s : State}
    (h : ReachableWO sk κ ax .rf s) : ReachableW sk κ ax s := by
  induction h with
  | init => exact .init
  | step a _ hstep ih =>
      exact .step a ih (by rwa [applyWO_rf] at hstep)

/-- All-reply-first wide-O reachability is base wide reachability,
left to right. -/
theorem reachableWO_rf_of_reachableW {s : State}
    (h : ReachableW sk κ ax s) : ReachableWO sk κ ax .rf s := by
  induction h with
  | init => exact .init
  | step a _ hstep ih =>
      exact .step a ih (by rwa [applyWO_rf])

/-- All-reply-first wide-O enabledness is base wide enabledness. -/
theorem canStepWO_rf (s : State) :
    canStepWO sk κ ax .rf s = canStepW sk κ ax s := by
  unfold canStepWO canStepW
  congr 1
  funext a
  rw [applyWO_rf]

/-- All-reply-first wide-O stuckness is base wide stuckness. -/
theorem stuckWO_rf (s : State) :
    stuckWO sk κ ax .rf s = stuckW sk κ ax s := by
  unfold stuckWO stuckW
  rw [canStepWO_rf]

/-- The base wide flagship's claim and the widened metatheorem's
reply-first corner are one claim. -/
theorem deadlockFreeWO_rf_iff :
    DeadlockFreeWO sk κ ax .rf ↔ StreamingMirror.DeadlockFreeW sk κ ax := by
  constructor
  · intro h s hr
    rw [← stuckWO_rf]
    exact h s (reachableWO_rf_of_reachableW sk κ ax hr)
  · intro h s hr
    rw [stuckWO_rf]
    exact h s (reachableW_of_reachableWO_rf sk κ ax hr)

-- ================================================ the floor corner transport

/-- Floor-capacity wide-O reachability is O reachability, right to
left (`applyWO_cap` per step). -/
theorem reachableO_of_reachableWO_cap {s : State}
    (h : ReachableWO sk sk.cap ax ord s) : ReachableO sk ax ord s := by
  induction h with
  | init => exact .init
  | step a _ hstep ih =>
      exact .step a ih (by rwa [applyWO_cap] at hstep)

/-- Floor-capacity wide-O reachability is O reachability, left to
right. -/
theorem reachableWO_cap_of_reachableO {s : State}
    (h : ReachableO sk ax ord s) : ReachableWO sk sk.cap ax ord s := by
  induction h with
  | init => exact .init
  | step a _ hstep ih =>
      exact .step a ih (by rwa [applyWO_cap])

/-- Floor-capacity wide-O enabledness is O enabledness. -/
theorem canStepWO_cap (s : State) :
    canStepWO sk sk.cap ax ord s = canStepO sk ax ord s := by
  unfold canStepWO canStepO
  congr 1
  funext a
  rw [applyWO_cap]

/-- Floor-capacity wide-O stuckness is O stuckness. -/
theorem stuckWO_cap (s : State) :
    stuckWO sk sk.cap ax ord s = stuckO sk ax ord s := by
  unfold stuckWO stuckO
  rw [canStepWO_cap]

/-- The floor metatheorem's claim and the widened metatheorem's
κ = `sk.cap` corner are one claim. -/
theorem deadlockFreeWO_cap_iff :
    DeadlockFreeWO sk sk.cap ax ord ↔ DeadlockFreeO sk ax ord := by
  constructor
  · intro h s hr
    rw [← stuckWO_cap]
    exact h s (reachableWO_cap_of_reachableO sk ax ord hr)
  · intro h s hr
    rw [stuckWO_cap]
    exact h s (reachableO_of_reachableWO_cap sk ax ord hr)

end StreamingMirror.Ord

namespace StreamingMirror.Sched

open Model Ord

/-- THE widened order-indifference metatheorem: the shipping encoder's
send order is deadlock-free at EVERY pointwise-widened capacity vector
κ ≥ κ₀ under EVERY assignment of the two-point per-loop dequeue-order
class.

The class (MODEL.md §5's dequeue-order subsection, the claim of
record): each pairing loop — each walk stage, and the absorber —
independently dequeues its per-scope wire reply and queued query in
one of exactly TWO orders, reply-first (the baseline transcription,
`OrdMap.rf`, definitionally `Model.apply`) or query-first (the
shipping Rust's order), with the end-of-stream close order TIED to the
loop's prologue choice. This is per-loop dequeue-order indifference
over that two-point class, NOT arbitrary-order indifference — loop
shapes outside the class (racing both queues, cross-scope prefetching,
prologues interleaved with sends) are named and excluded in MODEL.md's
amendment, and nothing is claimed for them.

The widening (cf. `Sched.deadlock_free_wide`): κ is per-channel —
widening the `level` family to the deployed window while keeping wires
at 1, widening wires, or any mix, are all instances — and covers
exactly the pointwise κ ≥ `sk.cap`, never more. The margin-0
hypothesis stays denominated at the FLOOR `capLevel` — the strongest
honest form, since widening never re-tightens it. The corners recover
the landed flagships definitionally: κ = `sk.cap` is
`Sched.deadlock_free_anyOrder` (`deadlock_free_anyOrder_cap_corner`),
`ord = .rf` is `Sched.deadlock_free_wide`
(`deadlock_free_wide_rf_corner`). Termination rides along at every κ
and every assignment: wide-O runs are ρ(init)-bounded
(`terminatingWO`). -/
theorem deadlock_free_wide_anyOrder (sk : Skel) (κ : Chan → Nat)
    (ord : OrdMap)
    (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel)
    (hκ : ∀ c, sk.cap c ≤ κ c) :
    DeadlockFreeWO sk κ .impl ord := by
  intro s hr
  unfold Ord.stuckWO
  cases ht : terminal sk s with
  | true => simp
  | false =>
      rw [progressWO sk κ ord hwf hm0 hκ (invPWO_reachableWO hwf hr) ht]
      simp

/-- A maximal wide-O run ends `terminal`, under the flagship's
hypotheses: the wide-O `maximal_run_terminal` (cf.
`maximal_run_terminal_wide`, `maximal_run_terminal_anyOrder`). -/
theorem maximal_run_terminal_wide_anyOrder (sk : Skel) (κ : Chan → Nat)
    (ord : OrdMap)
    (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel)
    (hκ : ∀ c, sk.cap c ≤ κ c) {acts : List Action} {s' : State}
    (hrun : runWO sk κ .impl ord (init sk) acts = some s')
    (hmax : canStepWO sk κ .impl ord s' = false) :
    terminal sk s' = true := by
  have hr := runWO_reachableWO sk κ .impl ord hrun
  have hdf := deadlock_free_wide_anyOrder sk κ ord hwf hm0 hκ s' hr
  unfold Ord.stuckWO at hdf
  rw [hmax] at hdf
  simpa using hdf

/-- The reply-first corner: the widened metatheorem at `OrdMap.rf`
re-derives the base wide flagship's claim — an internal consistency
pin that the quantified statement's reply-first instance IS
`Sched.deadlock_free_wide`'s theorem, not a new claim. -/
theorem deadlock_free_wide_rf_corner (sk : Skel) (κ : Chan → Nat)
    (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel)
    (hκ : ∀ c, sk.cap c ≤ κ c) :
    StreamingMirror.DeadlockFreeW sk κ .impl :=
  (deadlockFreeWO_rf_iff sk κ .impl).mp
    (deadlock_free_wide_anyOrder sk κ .rf hwf hm0 hκ)

/-- The floor corner: the widened metatheorem at κ = `sk.cap`
re-derives the floor metatheorem's claim — an internal consistency pin
that the widened statement's floor instance IS
`Sched.deadlock_free_anyOrder`'s theorem, not a new claim. -/
theorem deadlock_free_anyOrder_cap_corner (sk : Skel) (ord : OrdMap)
    (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) :
    DeadlockFreeO sk .impl ord :=
  (deadlockFreeWO_cap_iff sk .impl ord).mp
    (deadlock_free_wide_anyOrder sk sk.cap ord hwf hm0
      (fun _ => Nat.le_refl _))

end StreamingMirror.Sched
