/-
The widened O system: `applyO` at every pointwise capacity vector
κ ≥ `sk.cap` — the ord counterpart of Proofs/Wide.lean's `applyW`
layer.

# The transition function

`applyWO κ` is `applyO` with every push guard read against κ instead
of the floor. The eight order-dispatched arms (the pairing loops'
prologue receives and end-of-stream closes) are capacity-free —
receives and closes never consult a push guard — so they are
`applyO`'s arms VERBATIM, and every other arm delegates to `applyW`
(whose non-push arms are `Model.apply`'s, so the shared bodies agree
with `applyO`'s catch-all arm-for-arm). Three definitional anchors pin
the relationships: `applyWO_cap` (κ = `sk.cap` recovers `applyO`
exactly — the floor-recovery control), `applyWO_rf` (the all-reply-
first assignment recovers `applyW` exactly, so the base wide system is
the `ord = .rf` corner), and `applyWO_of_applyO` (guard monotonicity:
whatever the floor enables, every wider κ enables, with the same
successor).

# What this unit proves

Termination transfers to every κ and every assignment: ρ is chan-blind
and the wide-O guards differ from the floor's only at the pushes, so
every `applyWO` step shadows to an `applyO` step at chan-doctored
endpoints (`applyWO_floor_shadow`) and `rho_decreasesO` prices it —
`rho_decreasesWO`, `run_length_leWO`, `terminatingWO` (every wide-O
run from `init` is ρ(init)-bounded). Progress lifts thinly:
`progress_of_invO` is stated over `InvPWO` — conservation WITHOUT the
capacity half — so it holds at wide-O states as-is, and guard
monotonicity converts its floor-enabled action into a wide one
(`progressWO`). `DeadlockFreeWO` is the deadlock-freedom target shape
over these predicates; discharging its `InvPWO` premise along
`ReachableWO` runs is the preservation obligation, which this unit
states as `progressWO`'s explicit hypothesis rather than proving —
the wide-O counterpart of Proofs/Wide.lean's `invPW_preserved_W`
sweep is not part of this unit, and no theorem here claims it.

Chain (ord, wide): the widened transition layer and its termination/
progress transfers. Base mirror: Proofs/Wide.lean. Map: PROGRESS.md
§13.
-/
import StreamingMirror.Proofs.Wide
import StreamingMirror.Ord.Endgame

namespace StreamingMirror.Ord

open Model

variable (sk : Skel) (κ : Chan → Nat) (ax : AxMode) (ord : OrdMap)

/-- `applyO` with the push guards read against the capacity vector κ.

The eight order-dispatched arms are `applyO`'s verbatim (capacity-free:
prologue receives and closes never push); the catch-all delegates to
`applyW`, whose eight capacity comparisons are the model's entire
capacity surface. `applyWO sk sk.cap ax ord = applyO sk ax ord`
(`applyWO_cap`), and the guards are monotone in κ
(`applyWO_of_applyO`). -/
def applyWO (a : Action) (s : State) : Option State :=
  match a with
  | .walkRecvWire pk =>
      let ws := s.walk pk
      let c := wireIn pk
      let ph : Nat := match ord.walk pk with | .replyFirst => 0 | .queryFirst => 1
      if sk.walkKeys.contains pk && ws.phase == ph && s.chan c > 0 then
        some (setWalk { s with chan := bump s.chan c (-1) } pk
          (if ph == 1 then normWalk sk pk.2 { ws with phase := 2, committed := none }
           else { ws with phase := 1, committed := none }))
      else none
  | .walkRecvAsked pk =>
      let ws := s.walk pk
      let c := askedIn pk
      let ph : Nat := match ord.walk pk with | .replyFirst => 1 | .queryFirst => 0
      if sk.walkKeys.contains pk && ws.phase == ph && s.chan c > 0 then
        some (setWalk { s with chan := bump s.chan c (-1) } pk
          (if ph == 1 then normWalk sk pk.2 { ws with phase := 2, committed := none }
           else { ws with phase := 1, committed := none }))
      else none
  | .walkCloseWire pk =>
      let ws := s.walk pk
      let ph : Nat := match ord.walk pk with | .replyFirst => 3 | .queryFirst => 4
      if sk.walkKeys.contains pk && ws.phase == ph
          && producerDone sk s (wireIn pk) && s.chan (wireIn pk) == 0 then
        some (setWalk s pk { ws with phase := ph + 1 })
      else none
  | .walkCloseAsked pk =>
      let ws := s.walk pk
      let ph : Nat := match ord.walk pk with | .replyFirst => 4 | .queryFirst => 3
      if sk.walkKeys.contains pk && ws.phase == ph
          && producerDone sk s (askedIn pk) && s.chan (askedIn pk) == 0 then
        some (setWalk s pk { ws with phase := ph + 1 })
      else none
  | .absorbRecvWire =>
      let c := Chan.wire Party.R 0
      let ph : Nat := match ord.absorb with | .replyFirst => 0 | .queryFirst => 1
      if s.absorbPhase == ph && s.chan c > 0 then
        some { s with chan := bump s.chan c (-1), absorbPhase := ph + 1 }
      else none
  | .absorbRecvAsked =>
      let ph : Nat := match ord.absorb with | .replyFirst => 1 | .queryFirst => 0
      if s.absorbPhase == ph && s.chan Chan.leafRequests > 0 then
        some { s with chan := bump s.chan Chan.leafRequests (-1), absorbPhase := ph + 1 }
      else none
  | .absorbCloseWire =>
      let c := Chan.wire Party.R 0
      let ph : Nat := match ord.absorb with | .replyFirst => 3 | .queryFirst => 4
      if s.absorbPhase == ph && producerDone sk s c && s.chan c == 0 then
        some { s with absorbPhase := ph + 1 }
      else none
  | .absorbCloseAsked =>
      let ph : Nat := match ord.absorb with | .replyFirst => 4 | .queryFirst => 3
      if s.absorbPhase == ph && producerDone sk s Chan.leafRequests &&
          s.chan Chan.leafRequests == 0 then
        some { s with absorbPhase := ph + 1 }
      else none
  | a => applyW sk κ ax a s

-- =================================================== the two recoveries

/-- κ = `sk.cap` recovers the O semantics exactly: the floor-recovery
control pinning that the widened family contains the metatheorem's own
system (cf. `applyW_cap`). -/
theorem applyWO_cap (a : Action) (s : State) :
    applyWO sk sk.cap ax ord a s = applyO sk ax ord a s := by
  cases a
  case walkFire pk => exact applyW_cap sk ax (.walkFire pk) s
  all_goals rfl

/-- The all-reply-first assignment recovers the base wide system
exactly: the widened family's `ord = .rf` corner IS `applyW` (cf.
`applyO_rf`). -/
theorem applyWO_rf (a : Action) (s : State) :
    applyWO sk κ ax .rf a s = applyW sk κ ax a s := by
  cases a <;> rfl

-- ================================================= guard monotonicity

/-- Whatever the floor enables, every pointwise-wider κ enables, with
the same successor: the push guards are the only difference between
`applyWO` and `applyO`, and they are monotone (cf.
`applyW_of_apply`). -/
theorem applyWO_of_applyO (hκ : ∀ c, sk.cap c ≤ κ c) {a : Action}
    {s s' : State} (h : applyO sk ax ord a s = some s') :
    applyWO sk κ ax ord a s = some s' := by
  cases a
  case walkRecvWire pk => exact h
  case walkRecvAsked pk => exact h
  case walkCloseWire pk => exact h
  case walkCloseAsked pk => exact h
  case absorbRecvWire => exact h
  case absorbRecvAsked => exact h
  case absorbCloseWire => exact h
  case absorbCloseAsked => exact h
  case iopenChoose o =>
    have hb : Model.apply sk ax (.iopenChoose o) s = some s' := h
    exact applyW_of_apply sk κ ax hκ hb
  case iopenFire =>
    have hb : Model.apply sk ax .iopenFire s = some s' := h
    exact applyW_of_apply sk κ ax hκ hb
  case ropenRecv =>
    have hb : Model.apply sk ax .ropenRecv s = some s' := h
    exact applyW_of_apply sk κ ax hκ hb
  case ropenChoose o =>
    have hb : Model.apply sk ax (.ropenChoose o) s = some s' := h
    exact applyW_of_apply sk κ ax hκ hb
  case ropenFire =>
    have hb : Model.apply sk ax .ropenFire s = some s' := h
    exact applyW_of_apply sk κ ax hκ hb
  case walkCommit pk o =>
    have hb : Model.apply sk ax (.walkCommit pk o) s = some s' := h
    exact applyW_of_apply sk κ ax hκ hb
  case walkFire pk =>
    have hb : Model.apply sk ax (.walkFire pk) s = some s' := h
    exact applyW_of_apply sk κ ax hκ hb
  case asmRecvRes pk =>
    have hb : Model.apply sk ax (.asmRecvRes pk) s = some s' := h
    exact applyW_of_apply sk κ ax hκ hb
  case asmRecvLevel pk =>
    have hb : Model.apply sk ax (.asmRecvLevel pk) s = some s' := h
    exact applyW_of_apply sk κ ax hκ hb
  case asmSend pk =>
    have hb : Model.apply sk ax (.asmSend pk) s = some s' := h
    exact applyW_of_apply sk κ ax hκ hb
  case asmClose pk =>
    have hb : Model.apply sk ax (.asmClose pk) s = some s' := h
    exact applyW_of_apply sk κ ax hκ hb
  case absorbSend =>
    have hb : Model.apply sk ax .absorbSend s = some s' := h
    exact applyW_of_apply sk κ ax hκ hb
  case finRet =>
    have hb : Model.apply sk ax .finRet s = some s' := h
    exact applyW_of_apply sk κ ax hκ hb
  case finRes =>
    have hb : Model.apply sk ax .finRes s = some s' := h
    exact applyW_of_apply sk κ ax hκ hb
  case finRets =>
    have hb : Model.apply sk ax .finRets s = some s' := h
    exact applyW_of_apply sk κ ax hκ hb

/-- Some process can act in the widened O system. -/
def canStepWO (s : State) : Bool :=
  (allActions sk).any fun a => (applyWO sk κ ax ord a s).isSome

/-- The widened O deadlock predicate. -/
def stuckWO (s : State) : Bool := !terminal sk s && !canStepWO sk κ ax ord s

/-- Reachability of the widened O system. -/
inductive ReachableWO : State → Prop
  | init : ReachableWO (init sk)
  | step {s s' : State} (a : Action) :
      ReachableWO s → applyWO sk κ ax ord a s = some s' → ReachableWO s'

/-- Run a list of actions in the widened O system, failing on the
first disabled action. -/
def runWO (s : State) : List Action → Option State
  | [] => some s
  | a :: rest =>
      match applyWO sk κ ax ord a s with
      | some s' => runWO s' rest
      | none => none

/-- Greedy wide-O drain: first enabled action until quiescent. -/
def drainWO : Nat → State → State
  | 0, s => s
  | fuel + 1, s =>
      match (allActions sk).firstM (fun a => applyWO sk κ ax ord a s) with
      | some s' => drainWO fuel s'
      | none => s

/-- Whatever the floor can step, the wide-O system can step. -/
theorem canStepWO_of_canStepO (hκ : ∀ c, sk.cap c ≤ κ c) {s : State}
    (h : canStepO sk ax ord s = true) : canStepWO sk κ ax ord s = true := by
  rw [canStepO, List.any_eq_true] at h
  obtain ⟨a, hmem, happ⟩ := h
  rw [Option.isSome_iff_exists] at happ
  obtain ⟨s', hs'⟩ := happ
  rw [canStepWO, List.any_eq_true]
  exact ⟨a, hmem, by rw [applyWO_of_applyO sk κ ax ord hκ hs']; rfl⟩

/-- A successful wide-O run from `init` ends `ReachableWO`. -/
theorem runWO_reachableWO {acts : List Action} {s' : State}
    (h : runWO sk κ ax ord (init sk) acts = some s') :
    ReachableWO sk κ ax ord s' := by
  suffices general : ∀ (acts : List Action) (s s' : State),
      ReachableWO sk κ ax ord s → runWO sk κ ax ord s acts = some s' →
      ReachableWO sk κ ax ord s' by
    exact general acts _ _ (.init) h
  intro acts
  induction acts with
  | nil =>
      intro s s' hr hrun
      simp only [runWO, Option.some.injEq] at hrun
      exact hrun ▸ hr
  | cons a rest ih =>
      intro s s' hr hrun
      unfold runWO at hrun
      cases happ : applyWO sk κ ax ord a s with
      | none => simp [happ] at hrun
      | some s₁ =>
          exact ih s₁ s' (.step a hr happ) (by simpa [happ] using hrun)

-- =============================================== the floor companions

/-- Every wide-O step is an O step at chan-doctored endpoints: the
eighteen non-push arms are `applyO`'s verbatim, and every push guard
passes at zero occupancy (cf. `applyW_floor_shadow`). The counting
layer (ρ, `asmLevelsOk`, `InvL`, the producer/consumer counts) never
reads `chan`, so the doctored endpoints carry every cursor fact of the
real ones definitionally. -/
theorem applyWO_floor_shadow (hwf : sk.wellFormed = true) {a : Action}
    {s s' : State} (hstep : applyWO sk κ ax ord a s = some s') :
    ∃ ch ch' : Chan → Nat,
      applyO sk ax ord a { s with chan := ch }
        = some { s' with chan := ch' } := by
  cases a
  case iopenFire =>
    have hb : applyW sk κ ax .iopenFire s = some s' := hstep
    show ∃ ch ch', Model.apply sk ax .iopenFire { s with chan := ch }
      = some { s' with chan := ch' }
    exact applyW_floor_shadow sk κ ax hwf hb
  case ropenFire =>
    have hb : applyW sk κ ax .ropenFire s = some s' := hstep
    show ∃ ch ch', Model.apply sk ax .ropenFire { s with chan := ch }
      = some { s' with chan := ch' }
    exact applyW_floor_shadow sk κ ax hwf hb
  case walkFire pk =>
    have hb : applyW sk κ ax (.walkFire pk) s = some s' := hstep
    show ∃ ch ch', Model.apply sk ax (.walkFire pk) { s with chan := ch }
      = some { s' with chan := ch' }
    exact applyW_floor_shadow sk κ ax hwf hb
  case asmSend pk =>
    have hb : applyW sk κ ax (.asmSend pk) s = some s' := hstep
    show ∃ ch ch', Model.apply sk ax (.asmSend pk) { s with chan := ch }
      = some { s' with chan := ch' }
    exact applyW_floor_shadow sk κ ax hwf hb
  case absorbSend =>
    have hb : applyW sk κ ax .absorbSend s = some s' := hstep
    show ∃ ch ch', Model.apply sk ax .absorbSend { s with chan := ch }
      = some { s' with chan := ch' }
    exact applyW_floor_shadow sk κ ax hwf hb
  all_goals exact ⟨s.chan, s'.chan, hstep⟩

/-- ρ prices wide-O steps exactly as floor O steps: the measure is
chan-blind, so ρ_κ IS ρ, at every κ and every assignment (cf.
`rho_decreasesW`). -/
theorem rho_decreasesWO (hwf : sk.wellFormed = true) (a : Action)
    {s s' : State} (hlv : asmLevelsOk sk s = true)
    (hstep : applyWO sk κ ax ord a s = some s') :
    rho sk s' < rho sk s := by
  obtain ⟨ch, ch', h⟩ := applyWO_floor_shadow sk κ ax ord hwf hstep
  have h2 := rho_decreasesO sk ax ord a
    (show asmLevelsOk sk { s with chan := ch } = true from hlv) h
  have e1 : rho sk ({ s with chan := ch } : State) = rho sk s := rfl
  have e2 : rho sk ({ s' with chan := ch' } : State) = rho sk s' := rfl
  omega

/-- `asmLevelsOk` is inductive along wide-O runs, through the same
companion (cf. `asmLevelsOk_preservedW`). -/
theorem asmLevelsOk_preservedWO (hwf : sk.wellFormed = true) (a : Action)
    {s s' : State} (hstep : applyWO sk κ ax ord a s = some s')
    (hlv : asmLevelsOk sk s = true) : asmLevelsOk sk s' = true := by
  obtain ⟨ch, ch', h⟩ := applyWO_floor_shadow sk κ ax ord hwf hstep
  have h2 := asmLevelsOk_preservedO sk ax ord a h
    (show asmLevelsOk sk { s with chan := ch } = true from hlv)
  have e2 : asmLevelsOk sk ({ s' with chan := ch' } : State)
      = asmLevelsOk sk s' := rfl
  rw [← e2]
  exact h2

-- ============================================= the termination transfer

/-- Along any successful wide-O run, ρ pays for every step — the floor
bound, verbatim, at every κ and every assignment (cf.
`run_length_leW`). -/
theorem run_length_leWO (hwf : sk.wellFormed = true) :
    ∀ {acts : List Action} {s s' : State}, asmLevelsOk sk s = true →
      runWO sk κ ax ord s acts = some s' →
      acts.length + rho sk s' ≤ rho sk s := by
  intro acts
  induction acts with
  | nil =>
      intro s s' _ hrun
      simp only [runWO, Option.some.injEq] at hrun
      subst hrun
      simp
  | cons a rest ih =>
      intro s s' hlv hrun
      unfold runWO at hrun
      cases happ : applyWO sk κ ax ord a s with
      | none => simp [happ] at hrun
      | some s₁ =>
          have hrun' : runWO sk κ ax ord s₁ rest = some s' := by
            simpa [happ] using hrun
          have hd := rho_decreasesWO sk κ ax ord hwf a hlv happ
          have hlv' := asmLevelsOk_preservedWO sk κ ax ord hwf a happ hlv
          have := ih hlv' hrun'
          simp only [List.length_cons]
          omega

/-- Termination at every widened κ and every assignment of the
two-point class: every wide-O run from `init` has length at most
ρ(init) — ρ never reads occupancy, so the wide bound is the floor
bound (cf. `terminatingW`, `terminatingO`). -/
theorem terminatingWO (hwf : sk.wellFormed = true)
    {acts : List Action} {s' : State}
    (hrun : runWO sk κ ax ord (init sk) acts = some s') :
    acts.length ≤ rho sk (init sk) := by
  have := run_length_leWO sk κ ax ord hwf (asmLevelsOk_init sk) hrun
  omega

/-- The wide-O deadlock-freedom target shape: no `ReachableWO` state
is stuck. At κ = `sk.cap` this is `DeadlockFreeO` exactly
(`applyWO_cap`); at `ord = .rf` it is `DeadlockFreeW` exactly
(`applyWO_rf`). -/
def DeadlockFreeWO : Prop :=
  ∀ s : State, ReachableWO sk κ ax ord s → stuckWO sk κ ax ord s = false

/-- The wide-O progress lemma: a two-line lift of the O engine.

`progress_of_invO` holds at ANY `InvPWO` state — wide-O states
included, since the weak O invariant carries no capacity clause — and
concludes floor-O `canStepO`; guard monotonicity lifts the enabled
action into the wide system. The `InvPWO` hypothesis is the caller's
obligation: this unit does not derive it from `ReachableWO`
(Proofs/Wide.lean's `invPW_preserved_W` has no O twin here). -/
theorem progressWO (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel)
    (hκ : ∀ c, sk.cap c ≤ κ c) {s : State}
    (hi : InvPWO sk .impl ord s) (hnt : terminal sk s = false) :
    canStepWO sk κ .impl ord s = true :=
  canStepWO_of_canStepO sk κ .impl ord hκ
    (progress_of_invO sk ord hwf hm0 hi hnt)

end StreamingMirror.Ord

namespace StreamingMirror.Control

open Model Ord

set_option maxRecDepth 16000 in
set_option maxHeartbeats 1000000 in
/-- Positive anchor: the greedy wide-O drain runs `smokeChain` to
terminal at the mixed widened vector `κmix` under the all-query-first
assignment — the widened semantics is inhabited and live at the
shipping order's corner (cf. `wide_smoke_completes`). -/
theorem wideO_smoke_completes_qf :
    terminal Pin.smokeChain
      (drainWO Pin.smokeChain κmix .impl .qf 300 (init Pin.smokeChain))
      = true := by
  decide

/-- The widened O semantics is genuinely wider, kernel-decided: at the
base probe state the floor O guard refuses the opening push and κmix
accepts it. Together with `applyWO_cap` this pins that the wide-O
family strictly contains the metatheorem's own system (cf.
`applyW_strictly_wider`). -/
theorem applyWO_strictly_wider :
    (applyO Pin.smokeChain .impl .qf .iopenFire wideProbe).isSome = false
    ∧ (applyWO Pin.smokeChain κmix .impl .qf .iopenFire wideProbe).isSome
        = true := by
  constructor <;> decide

end StreamingMirror.Control
