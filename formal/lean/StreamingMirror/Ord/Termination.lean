/-
The O termination layer: the base measure ρ is order-blind, so every
`applyO` arm is priced by the base step lemmas — the fifteen shared
arms through the definitional coercion to `Model.apply`, the eight
re-phased arms by the SAME walk/absorb step lemmas crossed per the
assignment. `walkRho` counts remaining receives by phase (`recvRem`),
never by channel, and `absorbRho` reads only the cursor and phase; no
component of ρ reads channel occupancy, so no new measure is minted
here. Concretely: a query-first wire receive is the phase-1→2 shape
that `walkRho_recvAsked_lt` prices, a query-first wire close is the
phase-4→5 bump that `walkRho_closeAsked_lt` prices, and the absorber
arms likewise swap which base absorb-phase case they mirror.

The corollaries ride exactly as in the base file: `run_length_leO`
pays a unit of ρ per `runO` step, `terminatingO` bounds every run
from `init` by ρ(init) at EVERY assignment of the two-point class,
and `drain_quiescentO` drives the greedy `applyO` drain (`drainO`, a
local mirror of `Control.drain`) to quiescence within ρ fuel. The
progress-consuming closures (the `maximal_run_terminal` family) land
with the flagship unit, not here.

Chain (ord, stage F): termination corollaries; consumed by the
flagship unit. Base mirror: Proofs/Termination.lean. Map:
PROGRESS.md §13.
-/
import StreamingMirror.Proofs.Termination
import StreamingMirror.Ord.Basic

namespace StreamingMirror.Ord

open Model

-- ==================================================== list-sum helpers

/-- Pointwise-dominated maps have dominated sums (a private twin of
Proofs/Termination.lean's `map_sum_le`, which is not exported). -/
private theorem map_sum_le {α : Type _} {f g : α → Nat} :
    ∀ {l : List α}, (∀ x ∈ l, g x ≤ f x) → (l.map g).sum ≤ (l.map f).sum
  | [], _ => Nat.le_refl _
  | x :: xs, h => by
      simp only [List.map_cons, List.sum_cons]
      have h1 := h x (List.mem_cons_self ..)
      have h2 := map_sum_le fun y hy => h y (List.mem_cons_of_mem x hy)
      omega

/-- A pointwise-dominated map with one strict member has a strictly
smaller sum (a private twin of Proofs/Termination.lean's
`map_sum_lt`). -/
private theorem map_sum_lt {α : Type _} {f g : α → Nat} {y : α} :
    ∀ {l : List α}, (∀ x ∈ l, g x ≤ f x) → y ∈ l → g y < f y →
      (l.map g).sum < (l.map f).sum
  | [], _, hy, _ => nomatch hy
  | x :: xs, h, hy, hlt => by
      simp only [List.map_cons, List.sum_cons]
      rcases List.mem_cons.mp hy with heq | hy'
      · subst heq
        have h2 := map_sum_le fun z hz => h z (List.mem_cons_of_mem _ hz)
        omega
      · have h1 := h x (List.mem_cons_self ..)
        have h2 := map_sum_lt (fun z hz => h z (List.mem_cons_of_mem x hz))
          hy' hlt
        omega

-- ================================================== component lifters

/-- Lift a strict walk decrease (with an arbitrary channel-field
rewrite) to the whole measure (a private twin of
Proofs/Termination.lean's `rho_walk_lt`). -/
private theorem rho_walk_lt (sk : Skel) {s : State} (f : Chan → Nat)
    {pk : Party × Nat} {ws' : WalkSt} (hmem : pk ∈ sk.walkKeys)
    (hlt : walkRho sk pk.2 ws' < walkRho sk pk.2 (s.walk pk)) :
    rho sk (setWalk { s with chan := f } pk ws') < rho sk s := by
  have h1 : walkSum sk (setWalk { s with chan := f } pk ws')
      < walkSum sk s := by
    unfold walkSum
    refine map_sum_lt (fun pk' _ => ?_) hmem ?_
    · by_cases he : pk' = pk
      · subst he
        rw [setWalk_walk_self]
        exact Nat.le_of_lt hlt
      · rw [setWalk_walk_ne _ _ he]
        exact Nat.le_refl _
    · rw [setWalk_walk_self]
      exact hlt
  have h2 : asmSum sk (setWalk { s with chan := f } pk ws')
      = asmSum sk s := rfl
  have h3 : iopenRho (setWalk { s with chan := f } pk ws')
      = iopenRho s := rfl
  have h4 : ropenRho sk (setWalk { s with chan := f } pk ws')
      = ropenRho sk s := rfl
  have h5 : absorbRho sk (setWalk { s with chan := f } pk ws')
      = absorbRho sk s := rfl
  have h6 : finRho sk (setWalk { s with chan := f } pk ws')
      = finRho sk s := rfl
  unfold rho
  omega

/-- Lift a strict absorber decrease to the whole measure (a private
twin of Proofs/Termination.lean's `rho_ab_lt`). -/
private theorem rho_ab_lt (sk : Skel) {s s' : State}
    (h1 : walkSum sk s' = walkSum sk s) (h2 : asmSum sk s' = asmSum sk s)
    (h3 : iopenRho s' = iopenRho s) (h4 : ropenRho sk s' = ropenRho sk s)
    (h6 : finRho sk s' = finRho sk s)
    (hab : absorbRho sk s' < absorbRho sk s) : rho sk s' < rho sk s := by
  unfold rho
  omega

-- ================================================ the level invariant

/-- Every O action preserves the level invariant (cf.
`asmLevelsOk_preserved`): the fifteen shared arms are the base arms
definitionally, and the eight re-phased arms never touch an assembler
record — `asmLevelsOk` reads only `s.asm`, which their state deltas
fix. -/
theorem asmLevelsOk_preservedO (sk : Skel) (ax : AxMode) (ord : OrdMap)
    {s s' : State} (a : Action)
    (hstep : applyO sk ax ord a s = some s')
    (hlv : asmLevelsOk sk s = true) : asmLevelsOk sk s' = true := by
  cases a with
  | iopenChoose o =>
      exact asmLevelsOk_preserved sk ax (.iopenChoose o) hstep hlv
  | iopenFire =>
      exact asmLevelsOk_preserved sk ax .iopenFire hstep hlv
  | ropenRecv =>
      exact asmLevelsOk_preserved sk ax .ropenRecv hstep hlv
  | ropenChoose o =>
      exact asmLevelsOk_preserved sk ax (.ropenChoose o) hstep hlv
  | ropenFire =>
      exact asmLevelsOk_preserved sk ax .ropenFire hstep hlv
  | walkRecvWire pk =>
      cases hord : ord.walk pk <;> simp only [applyO, hord] at hstep <;>
        split at hstep
      all_goals
        first
          | (injection hstep with hs'; subst hs'; exact hlv)
          | simp at hstep
  | walkRecvAsked pk =>
      cases hord : ord.walk pk <;> simp only [applyO, hord] at hstep <;>
        split at hstep
      all_goals
        first
          | (injection hstep with hs'; subst hs'; exact hlv)
          | simp at hstep
  | walkCommit pk o =>
      exact asmLevelsOk_preserved sk ax (.walkCommit pk o) hstep hlv
  | walkFire pk =>
      exact asmLevelsOk_preserved sk ax (.walkFire pk) hstep hlv
  | walkCloseWire pk =>
      cases hord : ord.walk pk <;> simp only [applyO, hord] at hstep <;>
        split at hstep
      all_goals
        first
          | (injection hstep with hs'; subst hs'; exact hlv)
          | simp at hstep
  | walkCloseAsked pk =>
      cases hord : ord.walk pk <;> simp only [applyO, hord] at hstep <;>
        split at hstep
      all_goals
        first
          | (injection hstep with hs'; subst hs'; exact hlv)
          | simp at hstep
  | asmRecvRes pk =>
      exact asmLevelsOk_preserved sk ax (.asmRecvRes pk) hstep hlv
  | asmRecvLevel pk =>
      exact asmLevelsOk_preserved sk ax (.asmRecvLevel pk) hstep hlv
  | asmSend pk =>
      exact asmLevelsOk_preserved sk ax (.asmSend pk) hstep hlv
  | asmClose pk =>
      exact asmLevelsOk_preserved sk ax (.asmClose pk) hstep hlv
  | absorbRecvWire =>
      cases hord : ord.absorb <;> simp only [applyO, hord] at hstep <;>
        split at hstep
      all_goals
        first
          | (injection hstep with hs'; subst hs'; exact hlv)
          | simp at hstep
  | absorbRecvAsked =>
      cases hord : ord.absorb <;> simp only [applyO, hord] at hstep <;>
        split at hstep
      all_goals
        first
          | (injection hstep with hs'; subst hs'; exact hlv)
          | simp at hstep
  | absorbSend =>
      exact asmLevelsOk_preserved sk ax .absorbSend hstep hlv
  | absorbCloseWire =>
      cases hord : ord.absorb <;> simp only [applyO, hord] at hstep <;>
        split at hstep
      all_goals
        first
          | (injection hstep with hs'; subst hs'; exact hlv)
          | simp at hstep
  | absorbCloseAsked =>
      cases hord : ord.absorb <;> simp only [applyO, hord] at hstep <;>
        split at hstep
      all_goals
        first
          | (injection hstep with hs'; subst hs'; exact hlv)
          | simp at hstep
  | finRet =>
      exact asmLevelsOk_preserved sk ax .finRet hstep hlv
  | finRes =>
      exact asmLevelsOk_preserved sk ax .finRes hstep hlv
  | finRets =>
      exact asmLevelsOk_preserved sk ax .finRets hstep hlv

-- ======================================================= the decrease

/-- Every enabled O action strictly decreases ρ, at any state
satisfying the level invariant (cf. `rho_decreases`).

The measure is order-blind (module doc), so the eight re-phased arms
are priced by the base step lemmas crossed per the assignment
dispatch — each branch of a `cases ord.walk pk` (or `ord.absorb`) is
exactly one of the two base prologue/close shapes — and the fifteen
shared arms delegate to `rho_decreases` through the definitional
coercion. -/
theorem rho_decreasesO (sk : Skel) (ax : AxMode) (ord : OrdMap)
    {s s' : State} (a : Action) (hlv : asmLevelsOk sk s = true)
    (hstep : applyO sk ax ord a s = some s') : rho sk s' < rho sk s := by
  cases a with
  | iopenChoose o => exact rho_decreases sk ax (.iopenChoose o) hlv hstep
  | iopenFire => exact rho_decreases sk ax .iopenFire hlv hstep
  | ropenRecv => exact rho_decreases sk ax .ropenRecv hlv hstep
  | ropenChoose o => exact rho_decreases sk ax (.ropenChoose o) hlv hstep
  | ropenFire => exact rho_decreases sk ax .ropenFire hlv hstep
  | walkRecvWire pk =>
      cases hord : ord.walk pk <;> simp only [applyO, hord] at hstep
      -- reply-first: the base phase-0 → 1 receive shape
      · split at hstep
        case isFalse => simp at hstep
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
          obtain ⟨⟨hmem, hph⟩, -⟩ := hg
          have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
          injection hstep with hs'; subst hs'
          exact rho_walk_lt sk _ hmem'
            (walkRho_recvWire_lt sk pk.2 (s.walk pk) hph)
      -- query-first: the base phase-1 → 2 shape, crossed to this arm
      · split at hstep
        case isFalse => simp at hstep
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
          obtain ⟨⟨hmem, hph⟩, -⟩ := hg
          have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
          injection hstep with hs'; subst hs'
          exact rho_walk_lt sk _ hmem'
            (walkRho_recvAsked_lt sk pk.2 (s.walk pk) hph)
  | walkRecvAsked pk =>
      cases hord : ord.walk pk <;> simp only [applyO, hord] at hstep
      -- reply-first: the base phase-1 → 2 receive shape
      · split at hstep
        case isFalse => simp at hstep
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
          obtain ⟨⟨hmem, hph⟩, -⟩ := hg
          have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
          injection hstep with hs'; subst hs'
          exact rho_walk_lt sk _ hmem'
            (walkRho_recvAsked_lt sk pk.2 (s.walk pk) hph)
      -- query-first: the base phase-0 → 1 shape, crossed to this arm
      · split at hstep
        case isFalse => simp at hstep
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
          obtain ⟨⟨hmem, hph⟩, -⟩ := hg
          have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
          injection hstep with hs'; subst hs'
          exact rho_walk_lt sk _ hmem'
            (walkRho_recvWire_lt sk pk.2 (s.walk pk) hph)
  | walkCommit pk o => exact rho_decreases sk ax (.walkCommit pk o) hlv hstep
  | walkFire pk => exact rho_decreases sk ax (.walkFire pk) hlv hstep
  | walkCloseWire pk =>
      cases hord : ord.walk pk <;> simp only [applyO, hord] at hstep
      -- reply-first: the base phase-3 → 4 bump
      · split at hstep
        case isFalse => simp at hstep
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq] at hg
          obtain ⟨⟨⟨hmem, hph⟩, -⟩, -⟩ := hg
          have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
          injection hstep with hs'; subst hs'
          exact rho_walk_lt sk s.chan hmem'
            (walkRho_closeWire_lt sk pk.2 (s.walk pk) hph)
      -- query-first: the base phase-4 → 5 bump, crossed to this arm
      · split at hstep
        case isFalse => simp at hstep
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq] at hg
          obtain ⟨⟨⟨hmem, hph⟩, -⟩, -⟩ := hg
          have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
          injection hstep with hs'; subst hs'
          exact rho_walk_lt sk s.chan hmem'
            (walkRho_closeAsked_lt sk pk.2 (s.walk pk) hph)
  | walkCloseAsked pk =>
      cases hord : ord.walk pk <;> simp only [applyO, hord] at hstep
      -- reply-first: the base phase-4 → 5 bump
      · split at hstep
        case isFalse => simp at hstep
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq] at hg
          obtain ⟨⟨⟨hmem, hph⟩, -⟩, -⟩ := hg
          have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
          injection hstep with hs'; subst hs'
          exact rho_walk_lt sk s.chan hmem'
            (walkRho_closeAsked_lt sk pk.2 (s.walk pk) hph)
      -- query-first: the base phase-3 → 4 bump, crossed to this arm
      · split at hstep
        case isFalse => simp at hstep
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq] at hg
          obtain ⟨⟨⟨hmem, hph⟩, -⟩, -⟩ := hg
          have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
          injection hstep with hs'; subst hs'
          exact rho_walk_lt sk s.chan hmem'
            (walkRho_closeWire_lt sk pk.2 (s.walk pk) hph)
  | asmRecvRes pk => exact rho_decreases sk ax (.asmRecvRes pk) hlv hstep
  | asmRecvLevel pk => exact rho_decreases sk ax (.asmRecvLevel pk) hlv hstep
  | asmSend pk => exact rho_decreases sk ax (.asmSend pk) hlv hstep
  | asmClose pk => exact rho_decreases sk ax (.asmClose pk) hlv hstep
  | absorbRecvWire =>
      cases hord : ord.absorb <;> simp only [applyO, hord] at hstep
      -- reply-first: the base absorb phase-0 → 1 receive
      · split at hstep
        case isFalse => simp at hstep
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
          obtain ⟨hph, -⟩ := hg
          injection hstep with hs'; subst hs'
          refine rho_ab_lt sk rfl rfl rfl rfl rfl ?_
          simp only [absorbRho, hph]
          simp
      -- query-first: the base absorb phase-1 → 2 receive, crossed
      · split at hstep
        case isFalse => simp at hstep
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
          obtain ⟨hph, -⟩ := hg
          injection hstep with hs'; subst hs'
          refine rho_ab_lt sk rfl rfl rfl rfl rfl ?_
          simp only [absorbRho, hph]
          simp
  | absorbRecvAsked =>
      cases hord : ord.absorb <;> simp only [applyO, hord] at hstep
      -- reply-first: the base absorb phase-1 → 2 receive
      · split at hstep
        case isFalse => simp at hstep
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
          obtain ⟨hph, -⟩ := hg
          injection hstep with hs'; subst hs'
          refine rho_ab_lt sk rfl rfl rfl rfl rfl ?_
          simp only [absorbRho, hph]
          simp
      -- query-first: the base absorb phase-0 → 1 receive, crossed
      · split at hstep
        case isFalse => simp at hstep
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
          obtain ⟨hph, -⟩ := hg
          injection hstep with hs'; subst hs'
          refine rho_ab_lt sk rfl rfl rfl rfl rfl ?_
          simp only [absorbRho, hph]
          simp
  | absorbSend => exact rho_decreases sk ax .absorbSend hlv hstep
  | absorbCloseWire =>
      cases hord : ord.absorb <;> simp only [applyO, hord] at hstep
      -- reply-first: the base absorb phase-3 → 4 bump
      · split at hstep
        case isFalse => simp at hstep
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq] at hg
          obtain ⟨⟨hph, -⟩, -⟩ := hg
          injection hstep with hs'; subst hs'
          refine rho_ab_lt sk rfl rfl rfl rfl rfl ?_
          simp only [absorbRho, hph]
          simp
      -- query-first: the base absorb phase-4 → 5 bump, crossed
      · split at hstep
        case isFalse => simp at hstep
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq] at hg
          obtain ⟨⟨hph, -⟩, -⟩ := hg
          injection hstep with hs'; subst hs'
          refine rho_ab_lt sk rfl rfl rfl rfl rfl ?_
          simp only [absorbRho, hph]
          simp
  | absorbCloseAsked =>
      cases hord : ord.absorb <;> simp only [applyO, hord] at hstep
      -- reply-first: the base absorb phase-4 → 5 bump
      · split at hstep
        case isFalse => simp at hstep
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq] at hg
          obtain ⟨⟨hph, -⟩, -⟩ := hg
          injection hstep with hs'; subst hs'
          refine rho_ab_lt sk rfl rfl rfl rfl rfl ?_
          simp only [absorbRho, hph]
          simp
      -- query-first: the base absorb phase-3 → 4 bump, crossed
      · split at hstep
        case isFalse => simp at hstep
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq] at hg
          obtain ⟨⟨hph, -⟩, -⟩ := hg
          injection hstep with hs'; subst hs'
          refine rho_ab_lt sk rfl rfl rfl rfl rfl ?_
          simp only [absorbRho, hph]
          simp
  | finRet => exact rho_decreases sk ax .finRet hlv hstep
  | finRes => exact rho_decreases sk ax .finRes hlv hstep
  | finRets => exact rho_decreases sk ax .finRets hlv hstep

-- ======================================================= run bounds

/-- Along any successful O run, the measure pays for every step (cf.
`run_length_le`). -/
theorem run_length_leO (sk : Skel) (ax : AxMode) (ord : OrdMap) :
    ∀ {acts : List Action} {s s' : State}, asmLevelsOk sk s = true →
      runO sk ax ord s acts = some s' →
      acts.length + rho sk s' ≤ rho sk s := by
  intro acts
  induction acts with
  | nil =>
      intro s s' _ hrun
      simp only [runO, Option.some.injEq] at hrun
      subst hrun
      simp
  | cons a rest ih =>
      intro s s' hlv hrun
      unfold runO at hrun
      cases happ : applyO sk ax ord a s with
      | none => simp [happ] at hrun
      | some s₁ =>
          have hrun' : runO sk ax ord s₁ rest = some s' := by
            simpa [happ] using hrun
          have hd := rho_decreasesO sk ax ord a hlv happ
          have hlv' := asmLevelsOk_preservedO sk ax ord a happ hlv
          have := ih hlv' hrun'
          simp only [List.length_cons]
          omega

/-- Every O run from `init` has length at most ρ(init) — no infinite
`applyO` runs exist, and bounded checking at depth ρ(init) + 1 is
exhaustive, at every assignment of the two-point class (cf.
`terminating`). -/
theorem terminatingO (sk : Skel) (ax : AxMode) (ord : OrdMap)
    {acts : List Action} {s' : State}
    (hrun : runO sk ax ord (init sk) acts = some s') :
    acts.length ≤ rho sk (init sk) := by
  have := run_length_leO sk ax ord (asmLevelsOk_init sk) hrun
  omega

-- ================================================== the greedy drain

/-- Greedy drain under the assignment: take the first enabled
`applyO` action until quiescence or the fuel runs out (a local mirror
of `Control.drain` over `applyO`). -/
def drainO (sk : Skel) (ax : AxMode) (ord : OrdMap) : Nat → State → State
  | 0, s => s
  | fuel + 1, s =>
      match (allActions sk).firstM (fun a => applyO sk ax ord a s) with
      | some s' => drainO sk ax ord fuel s'
      | none => s

/-- `firstM` over `Option` fails only if every element fails (a
private twin of Proofs/Termination.lean's `firstM_eq_none`). -/
private theorem firstM_eq_none {α β : Type _} {f : α → Option β} :
    ∀ {l : List α}, l.firstM f = none → ∀ a ∈ l, f a = none := by
  intro l
  induction l with
  | nil => intro _ a ha; cases ha
  | cons x xs ih =>
      intro h a ha
      cases hfx : f x with
      | some b => simp [List.firstM, hfx] at h
      | none =>
          rcases List.mem_cons.mp ha with rfl | ha'
          · exact hfx
          · exact ih (by simpa [List.firstM, hfx] using h) a ha'

/-- `firstM` over `Option` succeeds only through one of its elements
(a private twin of Proofs/Termination.lean's `firstM_eq_some`). -/
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

/-- The greedy O drain with fuel at least ρ reaches quiescence: each
step strictly decreases ρ, so the fixpoint arrives before the fuel
runs out (cf. `drain_quiescent`). -/
theorem drain_quiescentO (sk : Skel) (ax : AxMode) (ord : OrdMap) :
    ∀ (fuel : Nat) (s : State), asmLevelsOk sk s = true →
      rho sk s ≤ fuel →
      canStepO sk ax ord (drainO sk ax ord fuel s) = false := by
  intro fuel
  induction fuel with
  | zero =>
      intro s hlv hle
      unfold drainO
      rw [canStepO, List.any_eq_false]
      intro a _
      cases happ : applyO sk ax ord a s with
      | none => simp
      | some s₁ =>
          have := rho_decreasesO sk ax ord a hlv happ
          omega
  | succ n ih =>
      intro s hlv hle
      unfold drainO
      cases hf : (allActions sk).firstM (fun a => applyO sk ax ord a s) with
      | none =>
          rw [canStepO, List.any_eq_false]
          intro a ha
          rw [firstM_eq_none hf a ha]
          simp
      | some s₁ =>
          obtain ⟨a, -, ha⟩ := firstM_eq_some hf
          have hd := rho_decreasesO sk ax ord a hlv ha
          exact ih s₁ (asmLevelsOk_preservedO sk ax ord a ha hlv) (by omega)

end StreamingMirror.Ord
