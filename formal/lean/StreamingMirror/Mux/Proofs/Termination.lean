/-
Mux-tier termination (the phase-4 F5 repair): every muxed run is
finite, with the explicit bound `2·ρ(init)`, so "deadlock-free"
upgrades to "completes" — a maximal run cannot stall (`MuxDeadlockFree`)
and cannot go on forever (this file), hence ends `mterminal`.

# The measure

`mrho = 2·ρ(base) + |pipe I| + |pipe R|`. The factor two prices the
mux's split of one base wire send into push + deliver:

- a base arm steps the base cursor machine, so ρ drops by at least one
  and the pipes are untouched: `mrho` drops by at least two;
- a push performs the base fire's cursor effect (ρ drops by at least
  one, worth two) and parks the frame (one pipe grows by one): net
  drop at least one;
- a deliver moves a frame out of a pipe into its slot and touches no
  cursor: exactly minus one.

The push case rides the base `rho_decreases` through a drained-channel
shadow state: `ρ` and `asmLevelsOk` never read channel occupancy, and
`firePush`'s cursor effect equals the disabled base fire's, so the
base decrease transfers verbatim (`rho` is chan-blind by definition —
every summand reads cursor fields only).

The same measure covers the K-variant and elastic compositions
(`mrho_decreasesK`, `mrho_decreasesE`): their deliver arms differ from
the record's only in the slot guard, and the guard does not enter the
measure argument.

# What this closes

AUDIT-NOTES A1's remedy (i) landed one tier short: the base artifact
got its ρ, the mux tier did not, and the T5/T6 docstrings said
"completes" on stuck-freedom alone. With `mux_maximal_run_terminal`
and `oracle_greedy_run_terminal` the word is honest: completion =
deadlock freedom (per pair) + termination (this file), both kernel.
-/
import StreamingMirror.Proofs.Termination
import StreamingMirror.Mux.Proofs.WcImpossibilityK
import StreamingMirror.Mux.Elastic
import StreamingMirror.Mux.Proofs.Necessity

namespace StreamingMirror.Mux

open Model

variable {sk : Skel}

-- ========================================================== the measure

/-- Total remaining operations of the muxed system: the base measure
doubled (each disabled wire send is repriced as push + deliver) plus
the frames in flight. -/
def mrho (sk : Skel) (s : MState) : Nat :=
  2 * rho sk s.base + (s.pipe .I).length + (s.pipe .R).length

/-- `ρ` never reads channel occupancy: every summand is a cursor
read. -/
theorem rho_chan_blind (f : Chan → Nat) (s : State) :
    rho sk { s with chan := f } = rho sk s := rfl

/-- `asmLevelsOk` never reads channel occupancy. -/
theorem asmLevelsOk_chan_blind (f : Chan → Nat) (s : State) :
    asmLevelsOk sk { s with chan := f } = asmLevelsOk sk s := rfl

-- ================================================== the per-arm deltas

/-- A base arm drops the base measure: `applyBase` delegates to
`Model.apply` on the base state, where `rho_decreases` applies
directly. -/
theorem rho_applyBase {ax : AxMode} {a : Action} {s s' : MState}
    (hstep : applyBase sk ax a s = some s')
    (hlv : asmLevelsOk sk s.base = true) :
    rho sk s'.base < rho sk s.base ∧ s'.pipe = s.pipe := by
  obtain ⟨-, b, hb, hs'⟩ := applyBase_inv hstep
  subst hs'
  exact ⟨rho_decreases sk ax a hlv hb, rfl⟩

/-- A push drops the base measure: `firePush`'s cursor effect is the
disabled base fire's, transferred through a drained-channel shadow
state where the fire's slot guard holds trivially. -/
theorem rho_firePush {C : Nat} {p : Party} {h : Nat} {s s' : MState}
    (hfp : firePush sk C p h s = some s')
    (hlv : asmLevelsOk sk s.base = true) :
    rho sk s'.base < rho sk s.base
      ∧ s'.pipe = fun q =>
          if q == p then s.pipe q ++ [Chan.wire p h] else s.pipe q := by
  have hlv₀ : asmLevelsOk sk { s.base with chan := fun _ => 0 } = true := by
    rw [asmLevelsOk_chan_blind]
    exact hlv
  simp only [firePush] at hfp
  split at hfp
  case isFalse => cases hfp
  case isTrue =>
    split at hfp
    · -- the opening stream
      next hr =>
      have hr' : h = sk.rootH := by simpa using hr
      subst hr'
      cases p with
      | I =>
          cases hch : s.base.iopenCh with
          | none => rw [hch] at hfp; cases hfp
          | some o =>
              cases o with
              | query => rw [hch] at hfp; cases hfp
              | wire =>
                  rw [hch] at hfp
                  injection hfp with hs'
                  have happ₀ : Model.apply sk .impl .iopenFire
                      { s.base with chan := fun _ => 0 }
                      = some { s.base with
                          chan := bump (fun _ => 0)
                            (Chan.wire Party.I sk.rootH) 1,
                          iopenWire := true, iopenCh := none } := by
                    show (match ({ s.base with
                        chan := fun _ => 0 } : State).iopenCh with
                      | some .wire => _ | some .query => _
                      | none => none) = _
                    show (match s.base.iopenCh with
                      | some .wire => _ | some .query => _
                      | none => none) = _
                    rw [hch]
                    rfl
                  have hd := rho_decreases sk .impl .iopenFire hlv₀ happ₀
                  rw [rho_chan_blind] at hd
                  constructor
                  · rw [← hs']
                    exact hd
                  · rw [← hs']
      | R =>
          cases hch : s.base.ropenCh with
          | none => rw [hch] at hfp; cases hfp
          | some o =>
              cases o with
              | query => rw [hch] at hfp; cases hfp
              | res => rw [hch] at hfp; cases hfp
              | wire =>
                  rw [hch] at hfp
                  injection hfp with hs'
                  have happ₀ : Model.apply sk .impl .ropenFire
                      { s.base with chan := fun _ => 0 }
                      = some { s.base with
                          chan := bump (fun _ => 0)
                            (Chan.wire Party.R sk.rootH) 1,
                          ropenWire := true, ropenCh := none } := by
                    show (match ({ s.base with
                        chan := fun _ => 0 } : State).ropenCh with
                      | some .wire => _ | some .res => _
                      | some .query => _ | none => none) = _
                    show (match s.base.ropenCh with
                      | some .wire => _ | some .res => _
                      | some .query => _ | none => none) = _
                    rw [hch]
                    rfl
                  have hd := rho_decreases sk .impl .ropenFire hlv₀ happ₀
                  rw [rho_chan_blind] at hd
                  constructor
                  · rw [← hs']
                    exact hd
                  · rw [← hs']
    · -- a walk stream
      split at hfp
      next i hcm =>
        split at hfp
        case isFalse => cases hfp
        case isTrue hg =>
          injection hfp with hs'
          have happ₀ : Model.apply sk .impl (.walkFire (p, h))
              { s.base with chan := fun _ => 0 }
              = some (setWalk { s.base with
                    chan := bump (fun _ => 0) (obligChan (p, h)
                      (.wire i)) 1 } (p, h)
                  (normWalk sk h (fireOblig (s.base.walk (p, h))
                    (.wire i)))) := by
            show (match ({ s.base with
                chan := fun _ => 0 } : State).walk (p, h) |>.committed with
              | some o => _ | none => none) = _
            show (match (s.base.walk (p, h)).committed with
              | some o => _ | none => none) = _
            rw [hcm]
            simp only [Bool.and_eq_true] at hg ⊢
            rw [if_pos (by
              refine ⟨⟨hg.1, hg.2⟩, ?_⟩
              show decide ((0 : Nat) < 1) = true
              decide)]
          have hd := rho_decreases sk .impl (.walkFire (p, h)) hlv₀ happ₀
          rw [show rho sk (setWalk { s.base with
                chan := bump (fun _ => 0) (obligChan (p, h) (.wire i)) 1 }
                (p, h) (normWalk sk h (fireOblig (s.base.walk (p, h))
                  (.wire i))))
              = rho sk (setWalk s.base (p, h)
                (normWalk sk h (fireOblig (s.base.walk (p, h))
                  (.wire i)))) from rfl,
            rho_chan_blind] at hd
          constructor
          · rw [← hs']
            exact hd
          · rw [← hs']
      next => cases hfp

/-- A deliver-shaped step is measure-neutral on the base and drops one
pipe entry: the chan bump is invisible to `ρ`. -/
theorem mrho_deliver_shape {p : Party} {c : Chan} {rest : List Chan}
    {s s' : MState} (hp : s.pipe p = c :: rest)
    (hbase : rho sk s'.base = rho sk s.base)
    (hpipe : s'.pipe = fun q => if q == p then rest else s.pipe q) :
    mrho sk s' < mrho sk s := by
  cases p with
  | I =>
      have hI : s'.pipe Party.I = rest := congrFun hpipe Party.I
      have hR : s'.pipe Party.R = s.pipe Party.R :=
        congrFun hpipe Party.R
      unfold mrho
      rw [hbase, hI, hR, hp]
      simp only [List.length_cons]
      omega
  | R =>
      have hI : s'.pipe Party.I = s.pipe Party.I :=
        congrFun hpipe Party.I
      have hR : s'.pipe Party.R = rest := congrFun hpipe Party.R
      unfold mrho
      rw [hbase, hI, hR, hp]
      simp only [List.length_cons]
      omega

-- =============================================== the three decreases

/-- Every enabled record-semantics mux action strictly decreases the
muxed measure. -/
theorem mrho_decreases {ax : AxMode} {C : Nat} {σI σR : Strategy}
    {a : MAction} {s s' : MState}
    (hstep : apply sk ax C σI σR a s = some s')
    (hlv : asmLevelsOk sk s.base = true) :
    mrho sk s' < mrho sk s := by
  cases a with
  | base a =>
      obtain ⟨hd, hpipe⟩ := rho_applyBase hstep hlv
      have hI : s'.pipe Party.I = s.pipe Party.I := by rw [hpipe]
      have hR : s'.pipe Party.R = s.pipe Party.R := by rw [hpipe]
      unfold mrho
      rw [hI, hR]
      omega
  | push p =>
      simp only [apply] at hstep
      split at hstep
      next h hσ =>
        obtain ⟨hd, hpipe⟩ := rho_firePush hstep hlv
        cases p with
        | I =>
            have hI : s'.pipe Party.I
                = s.pipe Party.I ++ [Chan.wire Party.I h] :=
              congrFun hpipe Party.I
            have hR : s'.pipe Party.R = s.pipe Party.R :=
              congrFun hpipe Party.R
            unfold mrho
            rw [hI, hR, List.length_append, List.length_cons, List.length_nil]
            omega
        | R =>
            have hI : s'.pipe Party.I = s.pipe Party.I :=
              congrFun hpipe Party.I
            have hR : s'.pipe Party.R
                = s.pipe Party.R ++ [Chan.wire Party.R h] :=
              congrFun hpipe Party.R
            unfold mrho
            rw [hI, hR, List.length_append, List.length_cons, List.length_nil]
            omega
      next => cases hstep
  | deliver p =>
      simp only [apply] at hstep
      split at hstep
      case h_2 => cases hstep
      case h_1 c rest hp =>
          split at hstep
          case isFalse => cases hstep
          case isTrue =>
            injection hstep with hs'
            subst hs'
            exact mrho_deliver_shape hp rfl rfl

/-- Every enabled K-variant mux action strictly decreases the muxed
measure: base and push arms are the record's, and `deliverStepK`
differs only in the slot guard. -/
theorem mrho_decreasesK {ax : AxMode} {KI KR C : Nat} {σI σR : Strategy}
    {a : MAction} {s s' : MState}
    (hstep : applyK sk ax KI KR C σI σR a s = some s')
    (hlv : asmLevelsOk sk s.base = true) :
    mrho sk s' < mrho sk s := by
  cases a with
  | base a =>
      obtain ⟨hd, hpipe⟩ := rho_applyBase hstep hlv
      have hI : s'.pipe Party.I = s.pipe Party.I := by rw [hpipe]
      have hR : s'.pipe Party.R = s.pipe Party.R := by rw [hpipe]
      unfold mrho
      rw [hI, hR]
      omega
  | push p =>
      exact mrho_decreases (ax := ax) (a := .push p) hstep hlv
  | deliver p =>
      simp only [applyK, deliverStepK] at hstep
      split at hstep
      case h_2 => cases hstep
      case h_1 c rest hp =>
          split at hstep
          case isFalse => cases hstep
          case isTrue =>
            injection hstep with hs'
            subst hs'
            exact mrho_deliver_shape hp rfl rfl

/-- Every enabled elastic mux action strictly decreases the muxed
measure: `deliverStepE` drops the slot guard, which the measure never
read. -/
theorem mrho_decreasesE {ax : AxMode} {C : Nat} {σI σR : Strategy}
    {a : MAction} {s s' : MState}
    (hstep : applyE sk ax C σI σR a s = some s')
    (hlv : asmLevelsOk sk s.base = true) :
    mrho sk s' < mrho sk s := by
  cases a with
  | base a =>
      obtain ⟨hd, hpipe⟩ := rho_applyBase hstep hlv
      have hI : s'.pipe Party.I = s.pipe Party.I := by rw [hpipe]
      have hR : s'.pipe Party.R = s.pipe Party.R := by rw [hpipe]
      unfold mrho
      rw [hI, hR]
      omega
  | push p =>
      exact mrho_decreases (ax := ax) (a := .push p) hstep hlv
  | deliver p =>
      simp only [applyE, deliverStepE] at hstep
      split at hstep
      case h_2 => cases hstep
      case h_1 c rest hp =>
          injection hstep with hs'
          subst hs'
          exact mrho_deliver_shape hp rfl rfl

-- ===================================== the level invariant, mux-lifted

/-- The level invariant reads only assembler cursors, which pushes and
delivers never touch; base arms preserve it through the base sweep. -/
theorem asmLevelsOk_mstep {ax : AxMode} {C : Nat} {σI σR : Strategy}
    {a : MAction} {s s' : MState}
    (hstep : apply sk ax C σI σR a s = some s')
    (hlv : asmLevelsOk sk s.base = true) :
    asmLevelsOk sk s'.base = true := by
  cases a with
  | base a =>
      obtain ⟨-, b, hb, hs'⟩ := applyBase_inv hstep
      subst hs'
      exact asmLevelsOk_preserved sk ax a hb hlv
  | push p =>
      simp only [apply] at hstep
      split at hstep
      next h hσ =>
        simp only [firePush] at hstep
        split at hstep
        case isFalse => cases hstep
        case isTrue =>
          split at hstep
          · cases p with
            | I =>
                cases hch : s.base.iopenCh with
                | none => rw [hch] at hstep; cases hstep
                | some o =>
                    cases o with
                    | query => rw [hch] at hstep; cases hstep
                    | wire =>
                        rw [hch] at hstep
                        injection hstep with hs'
                        rw [← hs']
                        exact hlv
            | R =>
                cases hch : s.base.ropenCh with
                | none => rw [hch] at hstep; cases hstep
                | some o =>
                    cases o with
                    | query => rw [hch] at hstep; cases hstep
                    | res => rw [hch] at hstep; cases hstep
                    | wire =>
                        rw [hch] at hstep
                        injection hstep with hs'
                        rw [← hs']
                        exact hlv
          · split at hstep
            next i hcm =>
              split at hstep
              case isFalse => cases hstep
              case isTrue =>
                injection hstep with hs'
                subst hs'
                exact hlv
            next => cases hstep
      next => cases hstep
  | deliver p =>
      simp only [apply] at hstep
      split at hstep
      case h_2 => cases hstep
      case h_1 c rest hp =>
          split at hstep
          case isFalse => cases hstep
          case isTrue =>
            injection hstep with hs'
            rw [← hs']
            exact hlv

-- ========================================================= run bounds

/-- Along any successful muxed run, the measure pays for every step. -/
theorem mrun_length_le {ax : AxMode} {C : Nat} {σI σR : Strategy} :
    ∀ {acts : List MAction} {s s' : MState},
      asmLevelsOk sk s.base = true →
      mrun sk ax C σI σR s acts = some s' →
      acts.length + mrho sk s' ≤ mrho sk s := by
  intro acts
  induction acts with
  | nil =>
      intro s s' _ hrun
      simp only [mrun, Option.some.injEq] at hrun
      subst hrun
      simp
  | cons a rest ih =>
      intro s s' hlv hrun
      unfold mrun at hrun
      cases happ : apply sk ax C σI σR a s with
      | none => simp [happ] at hrun
      | some s₁ =>
          have hrun' : mrun sk ax C σI σR s₁ rest = some s' := by
            simpa [happ] using hrun
          have hd := mrho_decreases happ hlv
          have hlv' := asmLevelsOk_mstep happ hlv
          have := ih hlv' hrun'
          simp only [List.length_cons]
          omega

/-- Mux-tier termination: every muxed run from `init` has length at
most `2·ρ(init)` — no infinite muxed runs exist, under any strategy
pair, any axiom mode, any capacity.

The other half of "completes": `MuxDeadlockFree` says a maximal run
cannot stall short of `mterminal`; this says every run is a prefix of
a maximal one after at most `2·ρ(init)` steps. -/
theorem mux_terminating {ax : AxMode} {C : Nat} {σI σR : Strategy}
    {acts : List MAction} {s' : MState}
    (hrun : mrun sk ax C σI σR (init sk) acts = some s') :
    acts.length ≤ 2 * rho sk (Model.init sk) := by
  have hlv : asmLevelsOk sk (init sk).base = true := asmLevelsOk_init sk
  have := mrun_length_le hlv hrun
  have hinit : mrho sk (init sk) = 2 * rho sk (Model.init sk) := by
    rw [mrho]
    rfl
  omega

-- ================================================ maximal-run closure

/-- `firstM` over `Option` fails only if every element fails. -/
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

/-- The greedy muxed drain with fuel at least `mrho` reaches
quiescence. -/
theorem mdrain_quiescent {ax : AxMode} {C : Nat} {σI σR : Strategy} :
    ∀ (fuel : Nat) (s : MState), asmLevelsOk sk s.base = true →
      mrho sk s ≤ fuel →
      mcanStep sk ax C σI σR (mdrain sk ax C σI σR fuel s) = false := by
  intro fuel
  induction fuel with
  | zero =>
      intro s hlv hle
      unfold mdrain
      rw [mcanStep, List.any_eq_false]
      intro a _
      cases happ : apply sk ax C σI σR a s with
      | none => simp
      | some s₁ =>
          have := mrho_decreases happ hlv
          omega
  | succ n ih =>
      intro s hlv hle
      unfold mdrain
      cases hf : (allMActions sk).firstM
          (fun a => apply sk ax C σI σR a s) with
      | none =>
          rw [mcanStep, List.any_eq_false]
          intro a ha
          rw [firstM_eq_none hf a ha]
          simp
      | some s₁ =>
          obtain ⟨a, -, ha⟩ := firstM_eq_some hf
          have hd := mrho_decreases ha hlv
          exact ih s₁ (asmLevelsOk_mstep ha hlv) (by omega)

/-- A maximal muxed run under a deadlock-free pair ends complete: the
run cannot stall (`hdf`) and cannot go on forever (`mux_terminating`),
so its final quiescent state is `mterminal`. -/
theorem mux_maximal_run_terminal {ax : AxMode} {C : Nat}
    {σI σR : Strategy} (hdf : MuxDeadlockFree sk ax C σI σR)
    {acts : List MAction} {s' : MState}
    (hrun : mrun sk ax C σI σR (init sk) acts = some s')
    (hmax : mcanStep sk ax C σI σR s' = false) :
    mterminal sk s' = true := by
  have hr := mrun_reachable hrun
  have hstuck := hdf s' hr
  unfold mstuck at hstuck
  rw [hmax] at hstuck
  simpa using hstuck

/-- The constructive completion package: under any deadlock-free pair
the greedy strategy-driven drain reaches `mterminal` within
`2·ρ(init)` steps — termination with an explicit fuel bound, no
fairness hypothesis anywhere. Message-denominated (Mux/Basic.lean,
# The byte-denomination caveat). -/
theorem mux_greedy_run_terminal {ax : AxMode} {C : Nat}
    {σI σR : Strategy} (hdf : MuxDeadlockFree sk ax C σI σR) :
    mterminal sk
      (mdrain sk ax C σI σR (2 * rho sk (Model.init sk)) (init sk))
      = true := by
  have hq := mdrain_quiescent (sk := sk) (ax := ax) (C := C)
    (σI := σI) (σR := σR) (2 * rho sk (Model.init sk)) (init sk)
    (asmLevelsOk_init sk) (Nat.le_refl _)
  have hr := mdrain_reachable sk ax C σI σR (2 * rho sk (Model.init sk))
    (MReachable.init)
  have hstuck := hdf _ hr
  unfold mstuck at hstuck
  rw [hq] at hstuck
  simpa using hstuck

-- ================================================ the K-variant spine
-- T8's termination half (the round-5 tripwire: K "completes" claims
-- need mux_terminatingK first). `mrho_decreasesK` landed above; here
-- the run bounds and maximal-run closure re-assemble over `applyK`.

/-- Run a list of K-variant actions from a state, failing on the first
disabled one — the K executable spine. -/
def mrunK (sk : Skel) (ax : AxMode) (KI KR C : Nat) (σI σR : Strategy)
    (s : MState) : List MAction → Option MState
  | [] => some s
  | a :: rest =>
      match applyK sk ax KI KR C σI σR a s with
      | some s' => mrunK sk ax KI KR C σI σR s' rest
      | none => none

/-- A successful `mrunK` from init lands on a K-reachable state. -/
theorem mrunK_reachable {sk : Skel} {ax : AxMode} {KI KR C : Nat}
    {σI σR : Strategy} {acts : List MAction} {s' : MState}
    (h : mrunK sk ax KI KR C σI σR (init sk) acts = some s') :
    KMReachable sk ax KI KR C σI σR s' := by
  suffices general : ∀ (acts : List MAction) (s s' : MState),
      KMReachable sk ax KI KR C σI σR s →
      mrunK sk ax KI KR C σI σR s acts = some s' →
      KMReachable sk ax KI KR C σI σR s' by
    exact general acts _ _ (.init) h
  intro acts
  induction acts with
  | nil =>
      intro s s' hr hrun
      simp only [mrunK, Option.some.injEq] at hrun
      exact hrun ▸ hr
  | cons a rest ih =>
      intro s s' hr hrun
      unfold mrunK at hrun
      cases happ : applyK sk ax KI KR C σI σR a s with
      | none => simp [happ] at hrun
      | some s₁ =>
          exact ih s₁ s' (.step a hr happ) (by simpa [happ] using hrun)

/-- Greedy K-variant drain: first enabled action until quiescent. -/
def mdrainK (sk : Skel) (ax : AxMode) (KI KR C : Nat)
    (σI σR : Strategy) : Nat → MState → MState
  | 0, s => s
  | fuel + 1, s =>
      match (allMActions sk).firstM
          (fun a => applyK sk ax KI KR C σI σR a s) with
      | some s' => mdrainK sk ax KI KR C σI σR fuel s'
      | none => s

/-- The greedy K drain preserves K-reachability. -/
theorem mdrainK_reachable (sk : Skel) (ax : AxMode) (KI KR C : Nat)
    (σI σR : Strategy) (fuel : Nat) :
    ∀ {s : MState}, KMReachable sk ax KI KR C σI σR s →
      KMReachable sk ax KI KR C σI σR
        (mdrainK sk ax KI KR C σI σR fuel s) := by
  induction fuel with
  | zero => intro s h; exact h
  | succ n ih =>
      intro s h
      unfold mdrainK
      cases hf : (allMActions sk).firstM
          (fun a => applyK sk ax KI KR C σI σR a s) with
      | none => exact h
      | some s' =>
          obtain ⟨a, -, ha⟩ := firstM_eq_some hf
          exact ih (.step a h ha)

/-- The level invariant survives every K-variant step: base and push
arms are the record harness's, and the K deliver touches no cursor. -/
theorem asmLevelsOk_mstepK {sk : Skel} {ax : AxMode} {KI KR C : Nat}
    {σI σR : Strategy} {a : MAction} {s s' : MState}
    (hstep : applyK sk ax KI KR C σI σR a s = some s')
    (hlv : asmLevelsOk sk s.base = true) :
    asmLevelsOk sk s'.base = true := by
  cases a with
  | base a =>
      have hstep' : apply sk ax C σI σR (.base a) s = some s' := hstep
      exact asmLevelsOk_mstep hstep' hlv
  | push p =>
      have hstep' : apply sk ax C σI σR (.push p) s = some s' := hstep
      exact asmLevelsOk_mstep hstep' hlv
  | deliver p =>
      have hstep' : deliverStepK KI KR p s = some s' := hstep
      unfold deliverStepK at hstep'
      split at hstep'
      next c rest hp =>
          split at hstep'
          case isFalse => cases hstep'
          case isTrue =>
            injection hstep' with hs'
            rw [← hs']
            exact hlv
      next => cases hstep'

/-- Along any successful K run, the measure pays for every step. -/
theorem mrunK_length_le {sk : Skel} {ax : AxMode} {KI KR C : Nat}
    {σI σR : Strategy} :
    ∀ {acts : List MAction} {s s' : MState},
      asmLevelsOk sk s.base = true →
      mrunK sk ax KI KR C σI σR s acts = some s' →
      acts.length + mrho sk s' ≤ mrho sk s := by
  intro acts
  induction acts with
  | nil =>
      intro s s' _ hrun
      simp only [mrunK, Option.some.injEq] at hrun
      subst hrun
      simp
  | cons a rest ih =>
      intro s s' hlv hrun
      unfold mrunK at hrun
      cases happ : applyK sk ax KI KR C σI σR a s with
      | none => simp [happ] at hrun
      | some s₁ =>
          have hrun' : mrunK sk ax KI KR C σI σR s₁ rest = some s' := by
            simpa [happ] using hrun
          have hd := mrho_decreasesK happ hlv
          have hlv' := asmLevelsOk_mstepK happ hlv
          have := ih hlv' hrun'
          simp only [List.length_cons]
          omega

/-- K-variant termination: every K run from `init` has length at most
`2·ρ(init)` — no infinite K runs exist, under any strategy pair, any
depths, any mode, any capacity. T8's bounded-step half ("completes",
T8-SPEC clause 6): `MuxDeadlockFreeK` says a maximal run cannot stall;
this says every run ends within the bound. -/
theorem mux_terminatingK {sk : Skel} {ax : AxMode} {KI KR C : Nat}
    {σI σR : Strategy} {acts : List MAction} {s' : MState}
    (hrun : mrunK sk ax KI KR C σI σR (init sk) acts = some s') :
    acts.length ≤ 2 * rho sk (Model.init sk) := by
  have hlv : asmLevelsOk sk (init sk).base = true := asmLevelsOk_init sk
  have := mrunK_length_le hlv hrun
  have hinit : mrho sk (init sk) = 2 * rho sk (Model.init sk) := by
    rw [mrho]
    rfl
  omega

/-- The greedy K drain with fuel at least `mrho` reaches quiescence. -/
theorem mdrainK_quiescent {sk : Skel} {ax : AxMode} {KI KR C : Nat}
    {σI σR : Strategy} :
    ∀ (fuel : Nat) (s : MState), asmLevelsOk sk s.base = true →
      mrho sk s ≤ fuel →
      mcanStepK sk ax KI KR C σI σR
        (mdrainK sk ax KI KR C σI σR fuel s) = false := by
  intro fuel
  induction fuel with
  | zero =>
      intro s hlv hle
      unfold mdrainK
      rw [mcanStepK, List.any_eq_false]
      intro a _
      cases happ : applyK sk ax KI KR C σI σR a s with
      | none => simp
      | some s₁ =>
          have := mrho_decreasesK happ hlv
          omega
  | succ n ih =>
      intro s hlv hle
      unfold mdrainK
      cases hf : (allMActions sk).firstM
          (fun a => applyK sk ax KI KR C σI σR a s) with
      | none =>
          rw [mcanStepK, List.any_eq_false]
          intro a ha
          rw [firstM_eq_none hf a ha]
          simp
      | some s₁ =>
          obtain ⟨a, -, ha⟩ := firstM_eq_some hf
          have hd := mrho_decreasesK ha hlv
          exact ih s₁ (asmLevelsOk_mstepK ha hlv) (by omega)

/-- A maximal K run under a K-deadlock-free pair ends complete: the
run cannot stall and cannot go on forever (`mux_terminatingK`), so its
final quiescent state is `mterminal`. -/
theorem muxK_maximal_run_terminal {sk : Skel} {ax : AxMode}
    {KI KR C : Nat} {σI σR : Strategy}
    (hdf : MuxDeadlockFreeK sk ax KI KR C σI σR)
    {acts : List MAction} {s' : MState}
    (hrun : mrunK sk ax KI KR C σI σR (init sk) acts = some s')
    (hmax : mcanStepK sk ax KI KR C σI σR s' = false) :
    mterminal sk s' = true := by
  have hr := mrunK_reachable hrun
  have hstuck := hdf s' hr
  unfold mstuckK at hstuck
  rw [hmax] at hstuck
  simpa using hstuck

/-- The constructive K completion package: under any K-deadlock-free
pair the greedy drain reaches `mterminal` within `2·ρ(init)` steps —
termination with an explicit fuel bound, no fairness hypothesis
anywhere. Message-denominated (Mux/Basic.lean, # The
byte-denomination caveat). -/
theorem muxK_greedy_run_terminal {sk : Skel} {ax : AxMode}
    {KI KR C : Nat} {σI σR : Strategy}
    (hdf : MuxDeadlockFreeK sk ax KI KR C σI σR) :
    mterminal sk
      (mdrainK sk ax KI KR C σI σR (2 * rho sk (Model.init sk))
        (init sk)) = true := by
  have hq := mdrainK_quiescent (sk := sk) (ax := ax) (KI := KI)
    (KR := KR) (C := C) (σI := σI) (σR := σR)
    (2 * rho sk (Model.init sk)) (init sk)
    (asmLevelsOk_init sk) (Nat.le_refl _)
  have hr := mdrainK_reachable sk ax KI KR C σI σR
    (2 * rho sk (Model.init sk)) (KMReachable.init)
  have hstuck := hdf _ hr
  unfold mstuckK at hstuck
  rw [hq] at hstuck
  simpa using hstuck

/-- T5 in completion form: on every well-formed margin-0 skeleton, at
every capacity C ≥ 1, the oracle pair's greedy drain reaches
`mterminal` within `2·ρ(init)` steps — the kernel content of "the
send-projection pusher completes every well-formed margin-0
skeleton". Message-denominated (Mux/Basic.lean, # The
byte-denomination caveat). -/
theorem oracle_greedy_run_terminal (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) (C : Nat) (hC : 1 ≤ C) :
    mterminal sk
      (mdrain sk .impl C (oracle .I) (oracle .R)
        (2 * rho sk (Model.init sk)) (init sk)) = true :=
  mux_greedy_run_terminal (oracle_deadlock_free hwf hm0 C hC)

end StreamingMirror.Mux
