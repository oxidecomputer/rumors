/-
Capacity monotonicity: the flagship at every pointwise-widened
capacity vector (MUX-PROGRESS §3.4b, track T10; resolves AUDIT-NOTES
A7 — the window.rs / latency-doc Kahn argument, [derived]-tier until
here).

# The widened system

`applyW κ` is `apply` with every push guard read against `κ : Chan →
Nat` instead of the floor `Skel.cap` (the eight guard literals are the
model's entire capacity surface; receives and closes are
capacity-free). Everything else — every successor state expression —
is verbatim `apply`. Two definitional anchors pin the relationship:
`applyW_cap` (κ = κ₀ recovers `apply` exactly — the non-vacuity
control) and `applyW_of_apply` (guards are monotone: whatever the
floor enables, every wider κ enables, with the same successor).

# The route (t10-audit.md: route (2), the InvPW route)

The progress engine needs no widening at all. Track G re-typed
`Sched.progress_of_inv` over `InvPW` — conservation without the
`chan ≤ cap` half — and its argmin argument holds at ANY such state:
the τ-least pending send provably has room at the FLOOR (else the E2
edge manufactures an earlier unperformed receive, contradicting
minimality), so the engine concludes floor-`canStep`, which
`applyW_of_apply` lifts to the wide system. What remains is exactly
one new obligation: `InvPW` is inductive along wide runs
(`invPW_preserved_W`). The track-F Steps extraction supplies every
per-arm fact: the counting layer (`InvL`, `sentOf`, `recvdOf`, `rho`,
`asmLevelsOk`) never reads channel occupancy, so a wide push's facts
are read off the floor step at a chan-doctored companion state
(`{ s with chan := fun _ => 0 }` — every push guard passes at zero),
and definitional chan-blindness transports them back. No preservation
monolith is touched, no diamond lemma is minted.

Termination transfers the same way: ρ is chan-blind, so
`rho_decreases` prices wide steps through the same companion
(`applyW_floor_shadow`), and run length ≤ ρ(init) holds verbatim at
every κ (`terminatingW`) — ρ_κ IS ρ.

# Scope

`deadlock_free_wide` covers the flagship corner: `.impl`, margin 0,
κ ≥ `sk.cap` pointwise (per channel — finer than per-family; widening
levels while keeping wires at 1, or any mix, is an instance). The
d5/schedulable corner is NOT covered: Endgame.lean's d5 chain still
consumes full `InvP` (it was never re-typed to `InvPW`; only the
E-side was) — see t10-audit.md §3 for the deferral note.

# Anchors (kernel decide)

`wide_smoke_completes`: `Pin.smokeChain` runs to terminal under the
greedy wide drain at a mixed widened vector (levels ×4, everything
else ×2). `applyW_strictly_wider`: a state (synthetic, not claimed
reachable) where the floor guard refuses the opening push and the
wide guard accepts it — the semantics is genuinely wider, so
`applyW_cap` is not an identity of identical functions.

Chain (wide capstone, T10): consumes `Sched.progress_of_inv` at
`InvPW` (EndgameE), the Steps extraction (Mux/Proofs/Steps/*) and the
opener fire facts (Mux/Proofs/SigmaStarInv), and Termination's ρ
stack; concludes `deadlock_free_wide` / `terminatingW`. Map:
Proofs/Map.lean.
-/
import StreamingMirror.Proofs.Termination
import StreamingMirror.Mux.Proofs.SigmaStarInv

namespace StreamingMirror.Model

open Action

-- ================================================= the widened system

/-- `apply` with the push guards read against the capacity vector `κ`.

Every arm's successor expression is verbatim `Model.apply`'s; the only
differences are the eight capacity comparisons (the two opener fires,
the three responder-opener fire arms, `walkFire`, `asmSend`,
`absorbSend`). `applyW sk sk.cap ax = apply sk ax` (`applyW_cap`), and
the guards are monotone in κ (`applyW_of_apply`). -/
def applyW (sk : Skel) (κ : Chan → Nat) (ax : AxMode) (a : Action)
    (s : State) : Option State :=
  match a with
  | iopenChoose o =>
      if s.iopenCh == none && iopenChoosable ax s o then
        some { s with iopenCh := some o }
      else none
  | iopenFire =>
      match s.iopenCh with
      | some .wire =>
          let c := Chan.wire Party.I sk.rootH
          if s.chan c < κ c then
            some { s with chan := bump s.chan c 1, iopenWire := true, iopenCh := none }
          else none
      | some .query =>
          let c := Chan.asked Party.I (sk.rootH - 1)
          if s.chan c < κ c then
            some { s with chan := bump s.chan c 1, iopenQuery := true, iopenCh := none }
          else none
      | none => none
  | ropenRecv =>
      let c := Chan.wire Party.I sk.rootH
      if !s.ropenGotWire && s.chan c > 0 then
        some { s with chan := bump s.chan c (-1), ropenGotWire := true }
      else none
  | ropenChoose o =>
      if s.ropenCh == none && ropenChoosable sk ax s o then
        some { s with ropenCh := some o }
      else none
  | ropenFire =>
      match s.ropenCh with
      | some .wire =>
          let c := Chan.wire Party.R sk.rootH
          if s.chan c < κ c then
            some { s with chan := bump s.chan c 1, ropenWire := true, ropenCh := none }
          else none
      | some .res =>
          if s.chan Chan.rootres < κ Chan.rootres then
            some { s with chan := bump s.chan Chan.rootres 1, ropenRes := true, ropenCh := none }
          else none
      | some .query =>
          let c := Chan.asked Party.R (sk.rootH - 2)
          if s.chan c < κ c then
            some { s with chan := bump s.chan c 1, ropenQ := s.ropenQ + 1, ropenCh := none }
          else none
      | none => none
  | walkRecvWire pk =>
      let ws := s.walk pk
      let c := wireIn pk
      if sk.walkKeys.contains pk && ws.phase == 0 && s.chan c > 0 then
        some (setWalk { s with chan := bump s.chan c (-1) } pk
          { ws with phase := 1, committed := none })
      else none
  | walkRecvAsked pk =>
      let ws := s.walk pk
      let c := askedIn pk
      if sk.walkKeys.contains pk && ws.phase == 1 && s.chan c > 0 then
        some (setWalk { s with chan := bump s.chan c (-1) } pk
          (normWalk sk pk.2 { ws with phase := 2, committed := none }))
      else none
  | walkCommit pk o =>
      let ws := s.walk pk
      if sk.walkKeys.contains pk && wkChoosable sk ax pk ws o then
        some (setWalk s pk { ws with committed := some o })
      else none
  | walkFire pk =>
      let ws := s.walk pk
      match ws.committed with
      | some o =>
          let c := obligChan pk o
          if sk.walkKeys.contains pk && ws.phase == 2 && s.chan c < κ c then
            some (setWalk { s with chan := bump s.chan c 1 } pk
              (normWalk sk pk.2 (fireOblig ws o)))
          else none
      | none => none
  | walkCloseWire pk =>
      let ws := s.walk pk
      if sk.walkKeys.contains pk && ws.phase == 3 && producerDone sk s (wireIn pk) && s.chan (wireIn pk) == 0 then
        some (setWalk s pk { ws with phase := 4 })
      else none
  | walkCloseAsked pk =>
      let ws := s.walk pk
      if sk.walkKeys.contains pk && ws.phase == 4 && producerDone sk s (askedIn pk) && s.chan (askedIn pk) == 0 then
        some (setWalk s pk { ws with phase := 5 })
      else none
  | asmRecvRes pk =>
      let a := s.asm pk
      let c := asmResChan pk
      if sk.asmKeys.contains pk && a.phase == 0 && s.chan c > 0 then
        some (setAsm { s with chan := bump s.chan c (-1) } pk
          { a with phase := if sk.pendAt pk.1 pk.2 a.idx > 0 then 1 else 2, got := 0 })
      else none
  | asmRecvLevel pk =>
      let a := s.asm pk
      let c := asmLevelChan pk
      if sk.asmKeys.contains pk && a.phase == 1 && s.chan c > 0 then
        some (setAsm { s with chan := bump s.chan c (-1) } pk
          { a with phase := if a.got + 1 == sk.pendAt pk.1 pk.2 a.idx then 2 else 1,
                   got := a.got + 1 })
      else none
  | asmSend pk =>
      let a := s.asm pk
      let c := sk.asmOutChan pk
      if sk.asmKeys.contains pk && a.phase == 2 && s.chan c < κ c then
        some (setAsm { s with chan := bump s.chan c 1 } pk
          { idx := a.idx + 1
            phase := if a.idx + 1 < (sk.asmResList pk.1 pk.2).length then 0 else 3
            got := 0 })
      else none
  | asmClose pk =>
      let a := s.asm pk
      let c := asmResChan pk
      if sk.asmKeys.contains pk && a.phase == 3 && producerDone sk s c && s.chan c == 0 then
        some (setAsm s pk { a with phase := 4 })
      else none
  | absorbRecvWire =>
      let c := Chan.wire Party.R 0
      if s.absorbPhase == 0 && s.chan c > 0 then
        some { s with chan := bump s.chan c (-1), absorbPhase := 1 }
      else none
  | absorbRecvAsked =>
      if s.absorbPhase == 1 && s.chan Chan.leafRequests > 0 then
        some { s with chan := bump s.chan Chan.leafRequests (-1), absorbPhase := 2 }
      else none
  | absorbSend =>
      let c := Chan.level Party.I 0
      if s.absorbPhase == 2 && s.chan c < κ c then
        some { s with chan := bump s.chan c 1, absorbIdx := s.absorbIdx + 1,
                      absorbPhase := if s.absorbIdx + 1 < sk.totalLeafReqs then 0 else 3 }
      else none
  | absorbCloseWire =>
      let c := Chan.wire Party.R 0
      if s.absorbPhase == 3 && producerDone sk s c && s.chan c == 0 then
        some { s with absorbPhase := 4 }
      else none
  | absorbCloseAsked =>
      if s.absorbPhase == 4 && producerDone sk s Chan.leafRequests &&
          s.chan Chan.leafRequests == 0 then
        some { s with absorbPhase := 5 }
      else none
  | finRet =>
      if !s.ifin && s.chan Chan.rootret > 0 then
        some { s with chan := bump s.chan Chan.rootret (-1), ifin := true }
      else none
  | finRes =>
      if !s.rfinGotRes && s.chan Chan.rootres > 0 then
        some { s with chan := bump s.chan Chan.rootres (-1), rfinGotRes := true }
      else none
  | finRets =>
      if s.rfinGotRes && s.rfinGot < sk.rootPending && s.chan Chan.rootrets > 0 then
        some { s with chan := bump s.chan Chan.rootrets (-1), rfinGot := s.rfinGot + 1 }
      else none

variable (sk : Skel) (κ : Chan → Nat) (ax : AxMode)

/-- Some process can act in the widened system. -/
def canStepW (s : State) : Bool :=
  (allActions sk).any fun a => (applyW sk κ ax a s).isSome

/-- The widened deadlock predicate. -/
def stuckW (s : State) : Bool := !terminal sk s && !canStepW sk κ ax s

/-- Reachability of the widened system. -/
inductive ReachableW : State → Prop
  | init : ReachableW (init sk)
  | step {s s' : State} (a : Action) :
      ReachableW s → applyW sk κ ax a s = some s' → ReachableW s'

/-- Run a list of actions in the widened system, failing on the first
disabled action — the executable spine of the wide anchors. -/
def runW (s : State) : List Action → Option State
  | [] => some s
  | a :: rest =>
      match applyW sk κ ax a s with
      | some s' => runW s' rest
      | none => none

/-- Greedy wide drain: first enabled action until quiescent. -/
def drainW : Nat → State → State
  | 0, s => s
  | fuel + 1, s =>
      match (allActions sk).firstM (fun a => applyW sk κ ax a s) with
      | some s' => drainW fuel s'
      | none => s

theorem runW_reachable {acts : List Action} {s' : State}
    (h : runW sk κ ax (init sk) acts = some s') :
    ReachableW sk κ ax s' := by
  suffices general : ∀ (acts : List Action) (s s' : State),
      ReachableW sk κ ax s → runW sk κ ax s acts = some s' →
      ReachableW sk κ ax s' by
    exact general acts _ _ (.init) h
  intro acts
  induction acts with
  | nil =>
      intro s s' hr hrun
      simp only [runW, Option.some.injEq] at hrun
      exact hrun ▸ hr
  | cons a rest ih =>
      intro s s' hr hrun
      unfold runW at hrun
      cases happ : applyW sk κ ax a s with
      | none => simp [happ] at hrun
      | some s₁ =>
          exact ih s₁ s' (.step a hr happ) (by simpa [happ] using hrun)

-- =================================================== the κ₀ recovery

/-- Every channel a walk publishes into has floor capacity one (the
wire arm included — this is the total form of the mux sweep's
`cap_obligChan_nonwire`). -/
theorem cap_obligChan_one (pk : Party × Nat) (o : Oblig) :
    sk.cap (obligChan pk o) = 1 := by
  cases o with
  | wire i => rfl
  | res i => rfl
  | query i =>
      show sk.cap (askedOut pk) = 1
      unfold askedOut
      split <;> rfl
  | parent => rfl

/-- κ = κ₀ recovers the record semantics exactly: the negative control
pinning that `deadlock_free_wide` is about the flagship's own model,
not a lookalike. -/
theorem applyW_cap (a : Action) (s : State) :
    applyW sk sk.cap ax a s = apply sk ax a s := by
  cases a
  case walkFire pk =>
    cases hcm : (s.walk pk).committed with
    | none => simp only [applyW, apply, hcm]
    | some o => simp only [applyW, apply, hcm, cap_obligChan_one]
  all_goals rfl

/-- The function form of the κ₀ recovery. -/
theorem applyW_cap_eq : applyW sk sk.cap ax = apply sk ax := by
  funext a s
  exact applyW_cap sk ax a s

-- ================================================= guard monotonicity

/-- Whatever the floor enables, every pointwise-wider κ enables, with
the same successor: the push guards are the only difference between
the two systems, and they are monotone. -/
theorem applyW_of_apply (hκ : ∀ c, sk.cap c ≤ κ c) {a : Action}
    {s s' : State} (h : apply sk ax a s = some s') :
    applyW sk κ ax a s = some s' := by
  cases a
  case iopenFire =>
    simp only [apply] at h
    simp only [applyW]
    split at h
    next hio =>
      simp only [hio]
      split at h
      case isFalse => cases h
      case isTrue hg =>
        split
        case isTrue => exact h
        case isFalse hnc =>
          exfalso
          have hcap := hκ (Chan.wire Party.I sk.rootH)
          have h1 : sk.cap (Chan.wire Party.I sk.rootH) = 1 := rfl
          omega
    next hio =>
      simp only [hio]
      split at h
      case isFalse => cases h
      case isTrue hg =>
        split
        case isTrue => exact h
        case isFalse hnc =>
          exfalso
          have hcap := hκ (Chan.asked Party.I (sk.rootH - 1))
          have h1 : sk.cap (Chan.asked Party.I (sk.rootH - 1)) = 1 := rfl
          omega
    next => cases h
  case ropenFire =>
    simp only [apply] at h
    simp only [applyW]
    split at h
    next hro =>
      simp only [hro]
      split at h
      case isFalse => cases h
      case isTrue hg =>
        split
        case isTrue => exact h
        case isFalse hnc =>
          exfalso
          have hcap := hκ (Chan.wire Party.R sk.rootH)
          have h1 : sk.cap (Chan.wire Party.R sk.rootH) = 1 := rfl
          omega
    next hro =>
      simp only [hro]
      split at h
      case isFalse => cases h
      case isTrue hg =>
        split
        case isTrue => exact h
        case isFalse hnc =>
          exfalso
          have hcap := hκ Chan.rootres
          have h1 : sk.cap Chan.rootres = 1 := rfl
          omega
    next hro =>
      simp only [hro]
      split at h
      case isFalse => cases h
      case isTrue hg =>
        split
        case isTrue => exact h
        case isFalse hnc =>
          exfalso
          have hcap := hκ (Chan.asked Party.R (sk.rootH - 2))
          have h1 : sk.cap (Chan.asked Party.R (sk.rootH - 2)) = 1 := rfl
          omega
    next => cases h
  case walkFire pk =>
    simp only [apply] at h
    simp only [applyW]
    split at h
    next o hcm =>
      simp only [hcm]
      split at h
      case isFalse => cases h
      case isTrue hg =>
        split
        case isTrue => exact h
        case isFalse hnc =>
          exfalso
          apply hnc
          simp only [Bool.and_eq_true, decide_eq_true_eq] at hg ⊢
          obtain ⟨⟨hmem, hph⟩, hlt⟩ := hg
          have hcap := hκ (obligChan pk o)
          have h1 := cap_obligChan_one sk pk o
          exact ⟨⟨hmem, hph⟩, by omega⟩
    next => cases h
  case asmSend pk =>
    simp only [apply] at h
    simp only [applyW]
    split at h
    case isFalse => cases h
    case isTrue hg =>
      split
      case isTrue => exact h
      case isFalse hnc =>
        exfalso
        apply hnc
        simp only [Bool.and_eq_true, decide_eq_true_eq] at hg ⊢
        obtain ⟨⟨hmem, hph⟩, hlt⟩ := hg
        have hcap := hκ (sk.asmOutChan pk)
        exact ⟨⟨hmem, hph⟩, by omega⟩
  case absorbSend =>
    simp only [apply] at h
    simp only [applyW]
    split at h
    case isFalse => cases h
    case isTrue hg =>
      split
      case isTrue => exact h
      case isFalse hnc =>
        exfalso
        apply hnc
        simp only [Bool.and_eq_true, decide_eq_true_eq] at hg ⊢
        obtain ⟨hph, hlt⟩ := hg
        have hcap := hκ (Chan.level Party.I 0)
        exact ⟨hph, by omega⟩
  all_goals exact h

/-- Whatever the floor can step, the wide system can step. -/
theorem canStepW_of_canStep (hκ : ∀ c, sk.cap c ≤ κ c) {s : State}
    (h : canStep sk ax s = true) : canStepW sk κ ax s = true := by
  rw [canStep, List.any_eq_true] at h
  obtain ⟨a, hmem, happ⟩ := h
  rw [Option.isSome_iff_exists] at happ
  obtain ⟨s', hs'⟩ := happ
  rw [canStepW, List.any_eq_true]
  exact ⟨a, hmem, by rw [applyW_of_apply sk κ ax hκ hs']; rfl⟩

-- =============================================== the floor companions

/-- Every wide step is a floor step at chan-doctored endpoints: the
guard is the only difference between the systems, only the pushes
consult it, and every push guard passes at zero occupancy. The
counting layer (ρ, `asmLevelsOk`, `InvL`, the producer/consumer
counts) never reads `chan`, so the doctored endpoints carry every
cursor fact of the real ones definitionally. `wellFormed` supplies
`1 ≤ cap` for the assembler and absorber sends. -/
theorem applyW_floor_shadow (hwf : sk.wellFormed = true) {a : Action}
    {s s' : State} (hstep : applyW sk κ ax a s = some s') :
    ∃ ch ch' : Chan → Nat,
      apply sk ax a { s with chan := ch } = some { s' with chan := ch' } := by
  cases a
  case iopenFire =>
    simp only [applyW] at hstep
    split at hstep
    next hio =>
      split at hstep
      case isFalse => cases hstep
      case isTrue =>
        injection hstep with hs'
        refine ⟨fun _ => 0,
          bump (fun _ => 0) (Chan.wire Party.I sk.rootH) 1, ?_⟩
        rw [← hs']
        simp only [apply, hio]
        rw [if_pos Nat.zero_lt_one]
    next hio =>
      split at hstep
      case isFalse => cases hstep
      case isTrue =>
        injection hstep with hs'
        refine ⟨fun _ => 0,
          bump (fun _ => 0) (Chan.asked Party.I (sk.rootH - 1)) 1, ?_⟩
        rw [← hs']
        simp only [apply, hio]
        rw [if_pos Nat.zero_lt_one]
    next => cases hstep
  case ropenFire =>
    simp only [applyW] at hstep
    split at hstep
    next hro =>
      split at hstep
      case isFalse => cases hstep
      case isTrue =>
        injection hstep with hs'
        refine ⟨fun _ => 0,
          bump (fun _ => 0) (Chan.wire Party.R sk.rootH) 1, ?_⟩
        rw [← hs']
        simp only [apply, hro]
        rw [if_pos Nat.zero_lt_one]
    next hro =>
      split at hstep
      case isFalse => cases hstep
      case isTrue =>
        injection hstep with hs'
        refine ⟨fun _ => 0, bump (fun _ => 0) Chan.rootres 1, ?_⟩
        rw [← hs']
        simp only [apply, hro]
        rw [if_pos Nat.zero_lt_one]
    next hro =>
      split at hstep
      case isFalse => cases hstep
      case isTrue =>
        injection hstep with hs'
        refine ⟨fun _ => 0,
          bump (fun _ => 0) (Chan.asked Party.R (sk.rootH - 2)) 1, ?_⟩
        rw [← hs']
        simp only [apply, hro]
        rw [if_pos Nat.zero_lt_one]
    next => cases hstep
  case walkFire pk =>
    simp only [applyW] at hstep
    split at hstep
    next o hcm =>
      split at hstep
      case isFalse => cases hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, decide_eq_true_eq] at hg
        obtain ⟨⟨hmem, hph⟩, -⟩ := hg
        injection hstep with hs'
        refine ⟨fun _ => 0, bump (fun _ => 0) (obligChan pk o) 1, ?_⟩
        rw [← hs']
        simp only [apply, hcm]
        rw [if_pos (by
          simp only [Bool.and_eq_true, decide_eq_true_eq]
          exact ⟨⟨hmem, hph⟩, Nat.zero_lt_one⟩)]
        rfl
    next => cases hstep
  case asmSend pk =>
    simp only [applyW] at hstep
    split at hstep
    case isFalse => cases hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, decide_eq_true_eq] at hg
      obtain ⟨⟨hmem, hph⟩, -⟩ := hg
      injection hstep with hs'
      refine ⟨fun _ => 0, bump (fun _ => 0) (sk.asmOutChan pk) 1, ?_⟩
      rw [← hs']
      simp only [apply]
      rw [if_pos (by
        simp only [Bool.and_eq_true, decide_eq_true_eq]
        have hcap := Sched.cap_pos hwf (sk.asmOutChan pk)
        exact ⟨⟨hmem, hph⟩, by omega⟩)]
      rfl
  case absorbSend =>
    simp only [applyW] at hstep
    split at hstep
    case isFalse => cases hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, decide_eq_true_eq] at hg
      obtain ⟨hph, -⟩ := hg
      injection hstep with hs'
      refine ⟨fun _ => 0, bump (fun _ => 0) (Chan.level Party.I 0) 1, ?_⟩
      rw [← hs']
      simp only [apply]
      rw [if_pos (by
        simp only [Bool.and_eq_true, decide_eq_true_eq]
        have hcap := Sched.cap_pos hwf (Chan.level Party.I 0)
        exact ⟨hph, by omega⟩)]
  all_goals exact ⟨s.chan, s'.chan, hstep⟩

/-- ρ prices wide steps exactly as floor steps: the measure is
chan-blind, so ρ_κ IS ρ, at every κ. The `asmLevelsOk` hypothesis is
the floor lemma's own (chan-blind as well). -/
theorem rho_decreasesW (hwf : sk.wellFormed = true) (a : Action)
    {s s' : State} (hlv : asmLevelsOk sk s = true)
    (hstep : applyW sk κ ax a s = some s') : rho sk s' < rho sk s := by
  obtain ⟨ch, ch', h⟩ := applyW_floor_shadow sk κ ax hwf hstep
  have h2 := rho_decreases sk ax a
    (show asmLevelsOk sk { s with chan := ch } = true from hlv) h
  have e1 : rho sk ({ s with chan := ch } : State) = rho sk s := rfl
  have e2 : rho sk ({ s' with chan := ch' } : State) = rho sk s' := rfl
  omega

/-- `asmLevelsOk` is inductive along wide runs, through the same
companion. -/
theorem asmLevelsOk_preservedW (hwf : sk.wellFormed = true) (a : Action)
    {s s' : State} (hstep : applyW sk κ ax a s = some s')
    (hlv : asmLevelsOk sk s = true) : asmLevelsOk sk s' = true := by
  obtain ⟨ch, ch', h⟩ := applyW_floor_shadow sk κ ax hwf hstep
  have h2 := asmLevelsOk_preserved sk ax a h
    (show asmLevelsOk sk { s with chan := ch } = true from hlv)
  have e2 : asmLevelsOk sk ({ s' with chan := ch' } : State)
      = asmLevelsOk sk s' := rfl
  rw [← e2]
  exact h2

-- ============================================== the InvPW assemblers

variable {sk : Skel} {κ : Chan → Nat} {ax : AxMode}

/-- A channel-neutral arm preserves the weak invariant. -/
theorem InvPW.quiet_step {s s' : State} (hi : InvPW sk ax s)
    (hL' : InvL sk ax s') (hq : Mux.QuietStep sk s s') :
    InvPW sk ax s' := by
  refine ⟨hL'.wk, hL'.asm, hL'.top, ?_⟩
  intro c hc
  rw [hq.chan, hq.sent c hc, hq.recvd c hc]
  exact hi.flow c hc

/-- One receive preserves the weak invariant: the occupancy drop
balances the consumer-count rise. -/
theorem InvPW.recv_step {s s' : State} {c₀ : Chan} (hi : InvPW sk ax s)
    (hL' : InvL sk ax s') (hr : Mux.RecvStep sk s s' c₀) :
    InvPW sk ax s' := by
  have hpos := hr.hpos
  refine ⟨hL'.wk, hL'.asm, hL'.top, ?_⟩
  intro c hc
  have h0 := hi.flow c hc
  rw [hr.chan, hr.sent c hc, hr.recvd c hc]
  by_cases he : c = c₀
  · subst he
    rw [bump_neg_one, if_pos rfl]
    omega
  · rw [bump_ne _ _ he, if_neg he]
    omega

/-- One send preserves the weak invariant — with NO capacity
obligation: this assembler covers the wire pushes the mux sweep's
`BaseFacts.of_send` deliberately could not, which is the whole point
of the `InvPW` route. -/
theorem InvPW.send_step {s s' : State} {c₀ : Chan} (hi : InvPW sk ax s)
    (hL' : InvL sk ax s')
    (hchan : s'.chan = bump s.chan c₀ 1)
    (hsent : ∀ c ∈ allChans sk,
      sentOf sk s' c = sentOf sk s c + (if c = c₀ then 1 else 0))
    (hrecv : ∀ c ∈ allChans sk, recvdOf sk s' c = recvdOf sk s c) :
    InvPW sk ax s' := by
  refine ⟨hL'.wk, hL'.asm, hL'.top, ?_⟩
  intro c hc
  have h0 := hi.flow c hc
  rw [hchan, hsent c hc, hrecv c hc]
  by_cases he : c = c₀
  · subst he
    rw [bump_one, if_pos rfl]
    omega
  · rw [bump_ne _ _ he, if_neg he]
    omega

/-- Undo a channel-field doctoring: the local invariant never reads
occupancy, in either direction (the converse of
`Mux.InvL.chan_blind`, which the companion transfers below need). -/
theorem InvL_unchan {s : State} {ch : Chan → Nat}
    (h : InvL sk ax { s with chan := ch }) : InvL sk ax s := by
  refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
  · have hc := wkLocalOk_congr sk ax (s := s)
      (s' := { s with chan := ch }) pk rfl
    rw [← hc]
    exact h.wk pk hpk
  · have hc := asmLocalOk_congr sk (s := s)
      (s' := { s with chan := ch }) pk rfl
    rw [← hc]
    exact h.asm pk hpk
  · have hc := topLocalOk_congr sk ax (s := s)
      (s' := { s with chan := ch }) rfl rfl rfl rfl rfl rfl rfl rfl rfl
      rfl rfl rfl
    rw [← hc]
    exact h.top

/-- Convert an opener fire-facts wire delta (indexed by party and
height) into the uniform per-channel delta `send_step` consumes. -/
theorem send_delta_of_wire_facts {s b : State} {p₀ : Party} {h₀ : Nat}
    (hsw : ∀ q g, sentOf sk b (Chan.wire q g)
      = sentOf sk s (Chan.wire q g) + (if q = p₀ ∧ g = h₀ then 1 else 0))
    (hsint : ∀ c, Mux.isWire c = false → sentOf sk b c = sentOf sk s c) :
    ∀ c ∈ allChans sk,
      sentOf sk b c = sentOf sk s c
        + (if c = Chan.wire p₀ h₀ then 1 else 0) := by
  intro c _
  cases hw : Mux.isWire c with
  | true =>
      obtain ⟨q, g, rfl⟩ := Mux.isWire_eq hw
      rw [hsw q g]
      congr 1
      by_cases hqg : q = p₀ ∧ g = h₀
      · obtain ⟨rfl, rfl⟩ := hqg
        rw [if_pos ⟨rfl, rfl⟩, if_pos rfl]
      · rw [if_neg hqg, if_neg (fun hcon => hqg (by
          injection hcon with h1 h2
          exact ⟨h1, h2⟩))]
  | false =>
      rw [hsint c hw, if_neg (fun hcon => by rw [hcon] at hw; cases hw)]
      omega

-- ================================================ the InvPW sweep

/-- The weak invariant is inductive along wide steps: the 23-arm
dispatch. Guard-identical arms feed the Steps extraction directly (a
wide non-push step IS the floor step); the eight push sub-arms read
their cursor facts off the floor step at the chan-doctored companion
(or off the shape-based fire facts directly) and re-attach the real
occupancy bump, which `send_step` accepts uncapped. -/
theorem invPW_preserved_W (hwf : sk.wellFormed = true) (a : Action)
    {s s' : State} (hstep : applyW sk κ ax a s = some s')
    (hi : InvPW sk ax s) : InvPW sk ax s' := by
  have hL := hi.local
  cases a
  case iopenChoose o =>
    cases o with
    | wire =>
        obtain ⟨hL', hq, -, -, -⟩ := Mux.step_iopenChoose_wire
          (show apply sk ax _ s = some s' from hstep) hL
        exact hi.quiet_step hL' hq
    | query =>
        obtain ⟨hL', hq, -⟩ := Mux.step_iopenChoose_query
          (show apply sk ax _ s = some s' from hstep) hL
        exact hi.quiet_step hL' hq
  case iopenFire =>
    simp only [applyW] at hstep
    split at hstep
    next hio =>
      split at hstep
      case isFalse => cases hstep
      case isTrue =>
        injection hstep with hs'
        have hs'' : s' = { ({ s with iopenWire := true, iopenCh := none }
            : State) with
            chan := bump s.chan (Chan.wire Party.I sk.rootH) 1 } := by
          rw [← hs']
        obtain ⟨hL', hsw, hsint, hrecv, -, -, -⟩ :=
          Mux.iopen_fire_facts hio hL
        refine hi.send_step (c₀ := Chan.wire Party.I sk.rootH) ?_ ?_ ?_ ?_
        · rw [hs'']
          exact Mux.InvL.chan_blind hL'
        · rw [hs'']
        · intro c hc
          rw [hs'', Mux.sentOf_chan_blind]
          exact send_delta_of_wire_facts hsw hsint c hc
        · intro c hc
          rw [hs'', Mux.recvdOf_chan_blind]
          exact hrecv c
    next hio =>
      split at hstep
      case isFalse => cases hstep
      case isTrue =>
        injection hstep with hs'
        have hs'' : s' = { ({ s with iopenQuery := true, iopenCh := none }
            : State) with
            chan := bump s.chan (Chan.asked Party.I (sk.rootH - 1)) 1 } := by
          rw [← hs']
        have happly : apply sk ax .iopenFire { s with chan := fun _ => 0 }
            = some { ({ s with iopenQuery := true, iopenCh := none }
                : State) with
                chan := bump (fun _ => 0)
                  (Chan.asked Party.I (sk.rootH - 1)) 1 } := by
          simp only [apply, hio]
          rw [if_pos Nat.zero_lt_one]
        obtain ⟨hL', hsend, -⟩ :=
          Mux.step_iopenFire_query (s := { s with chan := fun _ => 0 })
            hio happly (Mux.InvL.chan_blind hL)
        refine hi.send_step (c₀ := Chan.asked Party.I (sk.rootH - 1))
          ?_ ?_ ?_ ?_
        · rw [hs'']
          exact Mux.InvL.chan_blind (InvL_unchan hL')
        · rw [hs'']
        · intro c hc
          have h := hsend.sent c hc
          rw [Mux.sentOf_chan_blind, Mux.sentOf_chan_blind] at h
          rw [hs'', Mux.sentOf_chan_blind]
          exact h
        · intro c hc
          have h := hsend.recvd c hc
          rw [Mux.recvdOf_chan_blind, Mux.recvdOf_chan_blind] at h
          rw [hs'', Mux.recvdOf_chan_blind]
          exact h
    next => cases hstep
  case ropenRecv =>
    obtain ⟨hL', hr, -⟩ := Mux.step_ropenRecv
      (show apply sk ax _ s = some s' from hstep) hL
    exact hi.recv_step hL' hr
  case ropenChoose o =>
    cases o with
    | wire =>
        obtain ⟨hL', hq, -, -, -⟩ := Mux.step_ropenChoose_wire
          (show apply sk ax _ s = some s' from hstep) hL
        exact hi.quiet_step hL' hq
    | res =>
        obtain ⟨hL', hq, -⟩ := Mux.step_ropenChoose_res
          (show apply sk ax _ s = some s' from hstep) hL
        exact hi.quiet_step hL' hq
    | query =>
        obtain ⟨hL', hq, -⟩ := Mux.step_ropenChoose_query
          (show apply sk ax _ s = some s' from hstep) hL
        exact hi.quiet_step hL' hq
  case ropenFire =>
    simp only [applyW] at hstep
    split at hstep
    next hro =>
      split at hstep
      case isFalse => cases hstep
      case isTrue =>
        injection hstep with hs'
        have hs'' : s' = { ({ s with ropenWire := true, ropenCh := none }
            : State) with
            chan := bump s.chan (Chan.wire Party.R sk.rootH) 1 } := by
          rw [← hs']
        obtain ⟨hL', hsw, hsint, hrecv, -, -, -⟩ :=
          Mux.ropen_fire_facts hro hL
        refine hi.send_step (c₀ := Chan.wire Party.R sk.rootH) ?_ ?_ ?_ ?_
        · rw [hs'']
          exact Mux.InvL.chan_blind hL'
        · rw [hs'']
        · intro c hc
          rw [hs'', Mux.sentOf_chan_blind]
          exact send_delta_of_wire_facts hsw hsint c hc
        · intro c hc
          rw [hs'', Mux.recvdOf_chan_blind]
          exact hrecv c
    next hro =>
      split at hstep
      case isFalse => cases hstep
      case isTrue =>
        injection hstep with hs'
        have hs'' : s' = { ({ s with ropenRes := true, ropenCh := none }
            : State) with chan := bump s.chan Chan.rootres 1 } := by
          rw [← hs']
        have happly : apply sk ax .ropenFire { s with chan := fun _ => 0 }
            = some { ({ s with ropenRes := true, ropenCh := none }
                : State) with
                chan := bump (fun _ => 0) Chan.rootres 1 } := by
          simp only [apply, hro]
          rw [if_pos Nat.zero_lt_one]
        obtain ⟨hL', hsend, -⟩ :=
          Mux.step_ropenFire_res (s := { s with chan := fun _ => 0 })
            hro happly (Mux.InvL.chan_blind hL)
        refine hi.send_step (c₀ := Chan.rootres) ?_ ?_ ?_ ?_
        · rw [hs'']
          exact Mux.InvL.chan_blind (InvL_unchan hL')
        · rw [hs'']
        · intro c hc
          have h := hsend.sent c hc
          rw [Mux.sentOf_chan_blind, Mux.sentOf_chan_blind] at h
          rw [hs'', Mux.sentOf_chan_blind]
          exact h
        · intro c hc
          have h := hsend.recvd c hc
          rw [Mux.recvdOf_chan_blind, Mux.recvdOf_chan_blind] at h
          rw [hs'', Mux.recvdOf_chan_blind]
          exact h
    next hro =>
      split at hstep
      case isFalse => cases hstep
      case isTrue =>
        injection hstep with hs'
        have hs'' : s' =
            { ({ s with ropenQ := s.ropenQ + 1, ropenCh := none }
                : State) with
              chan := bump s.chan (Chan.asked Party.R (sk.rootH - 2)) 1 } := by
          rw [← hs']
        have happly : apply sk ax .ropenFire { s with chan := fun _ => 0 }
            = some { ({ s with ropenQ := s.ropenQ + 1, ropenCh := none }
                : State) with
                chan := bump (fun _ => 0)
                  (Chan.asked Party.R (sk.rootH - 2)) 1 } := by
          simp only [apply, hro]
          rw [if_pos Nat.zero_lt_one]
        obtain ⟨hL', hsend, -⟩ :=
          Mux.step_ropenFire_query (s := { s with chan := fun _ => 0 })
            hro happly (Mux.InvL.chan_blind hL)
        refine hi.send_step (c₀ := Chan.asked Party.R (sk.rootH - 2))
          ?_ ?_ ?_ ?_
        · rw [hs'']
          exact Mux.InvL.chan_blind (InvL_unchan hL')
        · rw [hs'']
        · intro c hc
          have h := hsend.sent c hc
          rw [Mux.sentOf_chan_blind, Mux.sentOf_chan_blind] at h
          rw [hs'', Mux.sentOf_chan_blind]
          exact h
        · intro c hc
          have h := hsend.recvd c hc
          rw [Mux.recvdOf_chan_blind, Mux.recvdOf_chan_blind] at h
          rw [hs'', Mux.recvdOf_chan_blind]
          exact h
    next => cases hstep
  case walkRecvWire pk =>
    obtain ⟨hL', hr, -⟩ := Mux.step_walkRecvWire hwf pk
      (show apply sk ax _ s = some s' from hstep) hL
    exact hi.recv_step hL' hr
  case walkRecvAsked pk =>
    obtain ⟨hL', hr, -⟩ := Mux.step_walkRecvAsked hwf pk
      (show apply sk ax _ s = some s' from hstep) hL
    exact hi.recv_step hL' hr
  case walkCommit pk o =>
    cases o with
    | wire i =>
        obtain ⟨hL', hq, -, -, -, -⟩ := Mux.step_walkCommit_wire hwf pk i
          (show apply sk ax _ s = some s' from hstep) hL
        exact hi.quiet_step hL' hq
    | res i =>
        obtain ⟨hL', hq, -⟩ := Mux.step_walkCommit_res pk i
          (show apply sk ax _ s = some s' from hstep) hL
        exact hi.quiet_step hL' hq
    | query i =>
        obtain ⟨hL', hq, -⟩ := Mux.step_walkCommit_query pk i
          (show apply sk ax _ s = some s' from hstep) hL
        exact hi.quiet_step hL' hq
    | parent =>
        obtain ⟨hL', hq, -⟩ := Mux.step_walkCommit_parent pk
          (show apply sk ax _ s = some s' from hstep) hL
        exact hi.quiet_step hL' hq
  case walkFire pk =>
    simp only [applyW] at hstep
    split at hstep
    next o hcm =>
      split at hstep
      case isFalse => cases hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, decide_eq_true_eq] at hg
        obtain ⟨⟨hmem, hph⟩, -⟩ := hg
        have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
        have hph2 : (s.walk pk).phase = 2 := by simpa using hph
        injection hstep with hs'
        have hs'' : s' = { setWalk s pk
            (normWalk sk pk.2 (fireOblig (s.walk pk) o)) with
            chan := bump s.chan (obligChan pk o) 1 } := by
          rw [← hs']
          rfl
        obtain ⟨hL', hsent, hrecv, -, -, -⟩ :=
          Mux.step_fire (s' := setWalk s pk
            (normWalk sk pk.2 (fireOblig (s.walk pk) o)))
            hwf pk o hmem' hph2 hcm rfl hL
        refine hi.send_step (c₀ := obligChan pk o) ?_ ?_ ?_ ?_
        · rw [hs'']
          exact Mux.InvL.chan_blind hL'
        · rw [hs'']
        · intro c hc
          rw [hs'', Mux.sentOf_chan_blind]
          exact hsent c hc
        · intro c hc
          rw [hs'', Mux.recvdOf_chan_blind]
          exact hrecv c hc
    next => cases hstep
  case walkCloseWire pk =>
    obtain ⟨hL', hq, -⟩ := Mux.step_walkCloseWire hwf pk
      (show apply sk ax _ s = some s' from hstep) hL
    exact hi.quiet_step hL' hq
  case walkCloseAsked pk =>
    obtain ⟨hL', hq, -⟩ := Mux.step_walkCloseAsked hwf pk
      (show apply sk ax _ s = some s' from hstep) hL
    exact hi.quiet_step hL' hq
  case asmRecvRes pk =>
    obtain ⟨hL', hr, -⟩ := Mux.step_asmRecvRes hwf pk
      (show apply sk ax _ s = some s' from hstep) hL
    exact hi.recv_step hL' hr
  case asmRecvLevel pk =>
    obtain ⟨hL', hr, -⟩ := Mux.step_asmRecvLevel hwf pk
      (show apply sk ax _ s = some s' from hstep) hL
    exact hi.recv_step hL' hr
  case asmSend pk =>
    simp only [applyW] at hstep
    split at hstep
    case isFalse => cases hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, decide_eq_true_eq] at hg
      obtain ⟨⟨hmem, hph⟩, -⟩ := hg
      injection hstep with hs'
      have hs'' : s' = { setAsm s pk
          { idx := (s.asm pk).idx + 1
            phase := if (s.asm pk).idx + 1
                < (sk.asmResList pk.1 pk.2).length then 0 else 3
            got := 0 } with
          chan := bump s.chan (sk.asmOutChan pk) 1 } := by
        rw [← hs']
        rfl
      have happly : apply sk ax (.asmSend pk)
          { s with chan := fun _ => 0 }
          = some { setAsm s pk
              { idx := (s.asm pk).idx + 1
                phase := if (s.asm pk).idx + 1
                    < (sk.asmResList pk.1 pk.2).length then 0 else 3
                got := 0 } with
              chan := bump (fun _ => 0) (sk.asmOutChan pk) 1 } := by
        simp only [apply]
        rw [if_pos (by
          simp only [Bool.and_eq_true, decide_eq_true_eq]
          have hcap := Sched.cap_pos hwf (sk.asmOutChan pk)
          exact ⟨⟨hmem, hph⟩, by omega⟩)]
        rfl
      obtain ⟨hL', hsend, -⟩ :=
        Mux.step_asmSend (s := { s with chan := fun _ => 0 })
          hwf pk happly (Mux.InvL.chan_blind hL)
      refine hi.send_step (c₀ := sk.asmOutChan pk) ?_ ?_ ?_ ?_
      · rw [hs'']
        exact Mux.InvL.chan_blind (InvL_unchan hL')
      · rw [hs'']
      · intro c hc
        have h := hsend.sent c hc
        rw [Mux.sentOf_chan_blind, Mux.sentOf_chan_blind] at h
        rw [hs'', Mux.sentOf_chan_blind]
        exact h
      · intro c hc
        have h := hsend.recvd c hc
        rw [Mux.recvdOf_chan_blind, Mux.recvdOf_chan_blind] at h
        rw [hs'', Mux.recvdOf_chan_blind]
        exact h
  case asmClose pk =>
    obtain ⟨hL', hq, -⟩ := Mux.step_asmClose hwf pk
      (show apply sk ax _ s = some s' from hstep) hL
    exact hi.quiet_step hL' hq
  case absorbRecvWire =>
    obtain ⟨hL', hr, -⟩ := Mux.step_absorbRecvWire hwf
      (show apply sk ax _ s = some s' from hstep) hL
    exact hi.recv_step hL' hr
  case absorbRecvAsked =>
    obtain ⟨hL', hr, -⟩ := Mux.step_absorbRecvAsked
      (show apply sk ax _ s = some s' from hstep) hL
    exact hi.recv_step hL' hr
  case absorbSend =>
    simp only [applyW] at hstep
    split at hstep
    case isFalse => cases hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, decide_eq_true_eq] at hg
      obtain ⟨hph, -⟩ := hg
      injection hstep with hs'
      have hs'' : s' =
          { ({ s with
                absorbIdx := s.absorbIdx + 1
                absorbPhase := if s.absorbIdx + 1 < sk.totalLeafReqs
                  then 0 else 3 } : State) with
            chan := bump s.chan (Chan.level Party.I 0) 1 } := by
        rw [← hs']
      have happly : apply sk ax .absorbSend { s with chan := fun _ => 0 }
          = some { ({ s with
                absorbIdx := s.absorbIdx + 1
                absorbPhase := if s.absorbIdx + 1 < sk.totalLeafReqs
                  then 0 else 3 } : State) with
              chan := bump (fun _ => 0) (Chan.level Party.I 0) 1 } := by
        simp only [apply]
        rw [if_pos (by
          simp only [Bool.and_eq_true, decide_eq_true_eq]
          have hcap := Sched.cap_pos hwf (Chan.level Party.I 0)
          exact ⟨hph, by omega⟩)]
      obtain ⟨hL', hsend, -⟩ :=
        Mux.step_absorbSend (s := { s with chan := fun _ => 0 }) happly
          (Mux.InvL.chan_blind hL)
      refine hi.send_step (c₀ := Chan.level Party.I 0) ?_ ?_ ?_ ?_
      · rw [hs'']
        exact Mux.InvL.chan_blind (InvL_unchan hL')
      · rw [hs'']
      · intro c hc
        have h := hsend.sent c hc
        rw [Mux.sentOf_chan_blind, Mux.sentOf_chan_blind] at h
        rw [hs'', Mux.sentOf_chan_blind]
        exact h
      · intro c hc
        have h := hsend.recvd c hc
        rw [Mux.recvdOf_chan_blind, Mux.recvdOf_chan_blind] at h
        rw [hs'', Mux.recvdOf_chan_blind]
        exact h
  case absorbCloseWire =>
    obtain ⟨hL', hq, -⟩ := Mux.step_absorbCloseWire
      (show apply sk ax _ s = some s' from hstep) hL
    exact hi.quiet_step hL' hq
  case absorbCloseAsked =>
    obtain ⟨hL', hq, -⟩ := Mux.step_absorbCloseAsked
      (show apply sk ax _ s = some s' from hstep) hL
    exact hi.quiet_step hL' hq
  case finRet =>
    obtain ⟨hL', hr, -⟩ := Mux.step_finRet
      (show apply sk ax _ s = some s' from hstep) hL
    exact hi.recv_step hL' hr
  case finRes =>
    obtain ⟨hL', hr, -⟩ := Mux.step_finRes
      (show apply sk ax _ s = some s' from hstep) hL
    exact hi.recv_step hL' hr
  case finRets =>
    obtain ⟨hL', hr, -⟩ := Mux.step_finRets
      (show apply sk ax _ s = some s' from hstep) hL
    exact hi.recv_step hL' hr

-- ================================================ wide reachability

/-- The weak invariant holds at the initial state (the full invariant
does, and weakens). -/
theorem invPW_init (sk : Skel) (ax : AxMode) : InvPW sk ax (init sk) :=
  ((inv_iff sk ax (init sk)).mp (inv_init sk ax)).weak

/-- The weak invariant holds at every wide-reachable state. -/
theorem invPW_reachableW {sk : Skel} {κ : Chan → Nat} {ax : AxMode}
    (hwf : sk.wellFormed = true) {s : State}
    (hr : ReachableW sk κ ax s) : InvPW sk ax s := by
  induction hr with
  | init => exact invPW_init sk ax
  | step a _ hstep ih => exact invPW_preserved_W hwf a hstep ih

-- ============================================= the termination transfer

/-- Along any successful wide run, ρ pays for every step — the floor
bound, verbatim, at every κ. -/
theorem run_length_leW {sk : Skel} (κ : Chan → Nat) {ax : AxMode}
    (hwf : sk.wellFormed = true) :
    ∀ {acts : List Action} {s s' : State}, asmLevelsOk sk s = true →
      runW sk κ ax s acts = some s' →
      acts.length + rho sk s' ≤ rho sk s := by
  intro acts
  induction acts with
  | nil =>
      intro s s' _ hrun
      simp only [runW, Option.some.injEq] at hrun
      subst hrun
      simp
  | cons a rest ih =>
      intro s s' hlv hrun
      unfold runW at hrun
      cases happ : applyW sk κ ax a s with
      | none => simp [happ] at hrun
      | some s₁ =>
          have hrun' : runW sk κ ax s₁ rest = some s' := by
            simpa [happ] using hrun
          have hd := rho_decreasesW sk κ ax hwf a hlv happ
          have hlv' := asmLevelsOk_preservedW sk κ ax hwf a happ hlv
          have := ih hlv' hrun'
          simp only [List.length_cons]
          omega

/-- Termination at every widened κ: every wide run from `init` has
length at most ρ(init). ρ never reads occupancy, so the wide bound is
the floor bound — ρ_κ IS ρ. -/
theorem terminatingW {sk : Skel} (κ : Chan → Nat) {ax : AxMode}
    (hwf : sk.wellFormed = true) {acts : List Action} {s' : State}
    (hrun : runW sk κ ax (init sk) acts = some s') :
    acts.length ≤ rho sk (init sk) := by
  have := run_length_leW κ hwf (asmLevelsOk_init sk) hrun
  omega

end StreamingMirror.Model

namespace StreamingMirror

/-- Deadlock freedom of the widened system: no wide-reachable state is
stuck. At κ = `sk.cap` this is `DeadlockFree` exactly (`applyW_cap`). -/
def DeadlockFreeW (sk : Skel) (κ : Chan → Nat) (ax : AxMode) : Prop :=
  ∀ s : State, Model.ReachableW sk κ ax s → Model.stuckW sk κ ax s = false

end StreamingMirror

namespace StreamingMirror.Sched

open Model

/-- The wide progress lemma: a two-line lift of the flagship engine.

`progress_of_inv` holds at ANY `InvPW` state — wide states included —
and concludes floor-`canStep`; guard monotonicity lifts the enabled
action into the wide system. This is the audit's monotonicity verdict
cashed in: no re-derivation, the argmin argument was already
capacity-semi-blind (t10-audit.md §2). -/
theorem progressW (sk : Skel) {κ : Chan → Nat}
    (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel)
    (hκ : ∀ c, sk.cap c ≤ κ c) {s : State}
    (hi : InvPW sk .impl s) (hnt : terminal sk s = false) :
    canStepW sk κ .impl s = true :=
  canStepW_of_canStep sk κ .impl hκ (progress_of_inv sk hwf hm0 hi hnt)

/-- THE capacity-monotonicity theorem (MUX-PROGRESS §3.4b; resolves
AUDIT-NOTES A7): the shipping encoder's order is deadlock-free at
EVERY pointwise-widened capacity vector κ ≥ κ₀.

κ is per-channel: widening the `level` family to the deployed window
while keeping wires at 1, widening wires, or any mix, are all
instances. The margin-0 hypothesis stays denominated at the FLOOR
`capLevel` — the strongest honest form, since widening never
re-tightens it. At κ = `sk.cap` the statement is the flagship
`deadlock_free` exactly (`applyW_cap` is definitional). Termination
rides along: wide runs are ρ(init)-bounded (`terminatingW`). -/
theorem deadlock_free_wide (sk : Skel) (κ : Chan → Nat)
    (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel)
    (hκ : ∀ c, sk.cap c ≤ κ c) :
    StreamingMirror.DeadlockFreeW sk κ .impl := by
  intro s hr
  unfold Model.stuckW
  cases ht : terminal sk s with
  | true => simp
  | false =>
      rw [progressW sk hwf hm0 hκ (invPW_reachableW hwf hr) ht]
      simp

/-- A maximal wide run ends `Terminal`, under the flagship's
hypotheses: the wide `maximal_run_terminal`. -/
theorem maximal_run_terminal_wide (sk : Skel) (κ : Chan → Nat)
    (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel)
    (hκ : ∀ c, sk.cap c ≤ κ c) {acts : List Action} {s' : State}
    (hrun : runW sk κ .impl (init sk) acts = some s')
    (hmax : canStepW sk κ .impl s' = false) :
    terminal sk s' = true := by
  have hr := runW_reachable sk κ .impl hrun
  have hdf := deadlock_free_wide sk κ hwf hm0 hκ s' hr
  unfold Model.stuckW at hdf
  rw [hmax] at hdf
  simpa using hdf

end StreamingMirror.Sched

namespace StreamingMirror.Control

open Model

/-- The anchor vector: `Pin.smokeChain`'s levels at 4× floor, every
other channel at 2 — a genuine per-family mix (κ ≥ κ₀ everywhere,
equal nowhere). -/
def κmix : Chan → Nat
  | .level _ _ => 8
  | _ => 2

set_option maxRecDepth 16000 in
set_option maxHeartbeats 1000000 in
/-- Positive anchor, kernel-decided: the greedy wide drain runs
`smokeChain` to terminal at the mixed widened vector. -/
theorem wide_smoke_completes :
    terminal Pin.smokeChain
      (drainW Pin.smokeChain κmix .impl 300 (init Pin.smokeChain))
      = true := by
  decide

/-- A synthetic probe state: `smokeChain`'s init with the opening wire
obligation chosen and its cell already occupied. Occupancy doctored by
hand — no reachability claimed; the state exists only to compare the
two guards. -/
def wideProbe : State :=
  { init Pin.smokeChain with
    iopenCh := some IOblig.wire
    chan := fun c => if c == Chan.wire Party.I 4 then 1 else 0 }

/-- The widened semantics is genuinely wider, kernel-decided: at the
probe state the floor guard refuses the opening push and κmix accepts
it. Together with `applyW_cap` (κ = κ₀ recovers `apply`
definitionally) this pins that `deadlock_free_wide` quantifies over a
strictly larger family of systems than the flagship's. -/
theorem applyW_strictly_wider :
    (apply Pin.smokeChain .impl .iopenFire wideProbe).isSome = false
    ∧ (applyW Pin.smokeChain κmix .impl .iopenFire wideProbe).isSome
        = true := by
  constructor <;> decide

end StreamingMirror.Control
