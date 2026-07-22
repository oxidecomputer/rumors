/-
Non-vacuity certificates for the strategy classes (the adjudication's
mandated controls, landed as a phase-4 repair): the shipped policy
`bottomMostReady` is `WorkConserving` — and its K/E-universe twins —
and is `LocalStrategy`, so every ∀-class impossibility in the suite
quantifies over a kernel-inhabited class.

# Why the sweep here is generic where the stage-3 sweep is not

The carrying fact is `HistInv` (SigmaStarInv.lean): the commit/flush
ledger reads the committed hand exactly. The stage-3 preservation sweep
(`sinv_reachable`) proves it bundled with `MuxInv`, whose flow fields
genuinely need `.impl` and well-formedness — but the strategy classes
quantify over `MReachableAny`, every axiom mode and every skeleton. So
this file re-proves `HistInv` alone, and the induction goes through at
full generality because nothing the hand ledger reads is mode- or
shape-sensitive:

- the axiom-mode flags appear only in the choosability guards
  (`iopenChoosable`/`ropenChoosable`/`wkChoosable`), and `wkChoosable`'s
  phase-2/uncommitted conjunct — the only part `hand_count` consumes —
  is unconditional (Model.lean's `if ws.phase != 2 || ws.committed.isSome
  then false`);
- the `walkKeys`-membership guard is in the transition itself, and every
  walk key's height is below `rootH` (`walkKeys_height_lt`), so a
  walk commit can never alias the opener's ledger row;
- the K and E deliver variants differ from the record only in the slot
  guard — none of them touches a hand, a commit, or a flush receipt.

The same generic sweep therefore serves all three state universes
(`MReachableAny`, `KMReachableAny`, `EMReachableAny`) with one set of
per-arm lemmas.

# The certificates

- `bottomMostReady_wc` (+ `bottomMostReady_wcK`, `bottomMostReady_wcE`):
  the shipped policy is work-conserving — `wc_impossibility`,
  `wc_impossibility_K`, `necessity`'s first conjunct, and
  `elastic_deadlock_free` quantify over non-empty classes, witnessed in
  the kernel.
- `bottomMostReady_local`: the policy reads only `rootH` and its own
  history, so it is `LocalStrategy` — the impossibility class contains
  a strategy that is simultaneously work-conserving and local, which is
  the honest reading of "the shipped mux is inside the indicted class".
- `idler_local`: the trivial inhabitant (`fun _ _ => none`), pinning
  `LocalStrategy` non-vacuous independently of the policy.
-/
import StreamingMirror.Mux.Proofs.SigmaStarInv
import StreamingMirror.Mux.Proofs.WcImpossibilityK
import StreamingMirror.Mux.Elastic

namespace StreamingMirror.Mux

open Model

variable {sk : Skel}

-- ====================================================== shape lemmas

/-- A held wire stream is within the strategy's scan range
(`rootH` inclusive): the root height directly, or below it through the
walk-key membership `holdsWire` carries (`walkKeys_height_lt`). -/
theorem holdsWire_le_rootH {p : Party} {h : Nat} {s : State}
    (hw : holdsWire sk p h s = true) : h ≤ sk.rootH := by
  rw [holdsWire.eq_def] at hw
  by_cases hr : (h == sk.rootH) = true
  · have := beq_iff_eq.mp hr
    omega
  · rw [if_neg hr] at hw
    simp only [Bool.and_eq_true] at hw
    have hmem : (p, h) ∈ sk.walkKeys :=
      (List.contains_iff_mem ..).mp hw.1.1
    have hlt : h < sk.rootH := walkKeys_height_lt hmem
    omega

-- ================================== the history ledger, one observation

/-- Appending one observation keeps the action-attribution field,
provided the observation is either not an `.act` or an `.act` of the
machine it lands on. -/
private theorem hist_party_append {s : MState} {p₀ : Party} {o : MObs}
    (hm : ∀ p a, MObs.act a ∈ s.hist p → actionParty a = p)
    (ho : ∀ a, o = .act a → actionParty a = p₀) :
    ∀ p a, MObs.act a ∈ recordObs s.hist p₀ o p → actionParty a = p := by
  intro p a hmem
  have hmem' : MObs.act a
      ∈ (if p == p₀ then s.hist p ++ [o] else s.hist p) := hmem
  by_cases hq : (p == p₀) = true
  · rw [if_pos hq] at hmem'
    rcases List.mem_append.mp hmem' with hold | hnew
    · exact hm p a hold
    · have he := List.mem_singleton.mp hnew
      rw [ho a he.symm]
      exact (beq_iff_eq.mp hq).symm
  · rw [if_neg hq] at hmem'
    exact hm p a hmem'

/-- A hand-neutral `.act` append preserves the history ledger: no
commit is recorded and no hand moves, so every ledger row is
untouched. -/
private theorem histInv_of_act {s s' : MState} {a : Action}
    (hm : HistInv sk s)
    (hh : s'.hist = recordObs s.hist (actionParty a) (.act a))
    (hhand : HandsEq sk s.base s'.base)
    (hnc : ∀ h, wireCommitOn sk.rootH a h = false) :
    HistInv sk s' := by
  constructor
  · rw [hh]
    exact hist_party_append hm.hist_party
      (fun a' ha' => by injection ha' with ha'; rw [ha'])
  · intro p h
    rw [hh, hhand]
    show commitsOf sk.rootH
        (if p == actionParty a then s.hist p ++ [.act a] else s.hist p) h
      = pushesOf
        (if p == actionParty a then s.hist p ++ [.act a] else s.hist p) h
        + _
    have hold := hm.hand_count p h
    by_cases hq : (p == actionParty a) = true
    · rw [if_pos hq, commitsOf_append_act, hnc h, if_neg (by simp),
        pushesOf_append]
      omega
    · rw [if_neg hq]
      exact hold

/-- A wire-commit `.act` append preserves the history ledger: exactly
one hand flips on while its machine's ledger gains exactly one
commit. -/
private theorem histInv_of_act_commit {s s' : MState} {a : Action}
    {p₀ : Party} {h₀ : Nat}
    (hm : HistInv sk s)
    (hh : s'.hist = recordObs s.hist (actionParty a) (.act a))
    (hap : actionParty a = p₀)
    (hcn : ∀ h, wireCommitOn sk.rootH a h = decide (h = h₀))
    (hoff : holdsWire sk p₀ h₀ s.base = false)
    (hon : holdsWire sk p₀ h₀ s'.base = true)
    (hother : ∀ p h, ¬(p = p₀ ∧ h = h₀) →
      holdsWire sk p h s'.base = holdsWire sk p h s.base) :
    HistInv sk s' := by
  constructor
  · rw [hh]
    exact hist_party_append hm.hist_party
      (fun a' ha' => by injection ha' with ha'; rw [ha'])
  · intro p h
    rw [hh]
    show commitsOf sk.rootH
        (if p == actionParty a then s.hist p ++ [.act a] else s.hist p) h
      = pushesOf
        (if p == actionParty a then s.hist p ++ [.act a] else s.hist p) h
        + _
    have hold := hm.hand_count p h
    by_cases hq : (p == actionParty a) = true
    · have hp₀ : p = p₀ := by rw [beq_iff_eq.mp hq, hap]
      rw [if_pos hq, commitsOf_append_act, hcn h, pushesOf_append]
      by_cases hhe : h = h₀
      · subst hhe
        subst hp₀
        rw [hoff, if_neg (by simp)] at hold
        rw [hon]
        simp only [decide_true, if_true]
        omega
      · rw [hother p h (fun hcon => hhe hcon.2)]
        simp only [decide_eq_true_eq]
        rw [if_neg hhe]
        omega
    · rw [if_neg hq]
      by_cases hpe : p = p₀ ∧ h = h₀
      · exfalso
        rw [hpe.1, ← hap] at hq
        simp at hq
      · rw [hother p h hpe]
        exact hold

/-- A flush-receipt append preserves the history ledger: the pushed
stream's hand flips off while its machine's ledger gains exactly one
flush. -/
private theorem histInv_of_pushed {s s' : MState} {p : Party} {h : Nat}
    (hm : HistInv sk s)
    (hh : s'.hist = recordObs s.hist p (.pushed h))
    (hon : holdsWire sk p h s.base = true)
    (hoff : holdsWire sk p h s'.base = false)
    (hother : ∀ q g, ¬(q = p ∧ g = h) →
      holdsWire sk q g s'.base = holdsWire sk q g s.base) :
    HistInv sk s' := by
  constructor
  · rw [hh]
    exact hist_party_append hm.hist_party (fun a' ha' => by cases ha')
  · intro q g
    rw [hh]
    show commitsOf sk.rootH
        (if q == p then s.hist q ++ [.pushed h] else s.hist q) g
      = pushesOf
        (if q == p then s.hist q ++ [.pushed h] else s.hist q) g + _
    have hold := hm.hand_count q g
    by_cases hq : (q == p) = true
    · have hqp : q = p := beq_iff_eq.mp hq
      subst hqp
      rw [if_pos (by simp),
        commitsOf_append_other sk.rootH (s.hist q)
          (fun a hc => MObs.noConfusion hc),
        pushesOf_append]
      show commitsOf sk.rootH (s.hist q) g
          = pushesOf (s.hist q) g + (if h = g then 1 else 0)
            + (if holdsWire sk q g s'.base = true then 1 else 0)
      by_cases hg : h = g
      · rw [← hg] at hold ⊢
        rw [hon, if_pos rfl] at hold
        rw [hoff, if_pos rfl]
        simp only [Bool.false_eq_true, if_false]
        omega
      · rw [hother q g (fun hcon => hg hcon.2.symm), if_neg hg]
        omega
    · rw [if_neg hq]
      rw [hother q g (fun hcon => by rw [hcon.1] at hq; simp at hq)]
      exact hold

/-- A delivery-receipt append preserves the history ledger: no commit,
no flush, no hand moves. -/
private theorem histInv_of_delivered {s s' : MState} {p : Party} {g : Nat}
    (hm : HistInv sk s)
    (hh : s'.hist = recordObs s.hist p (.delivered g))
    (hhand : HandsEq sk s.base s'.base) :
    HistInv sk s' := by
  constructor
  · rw [hh]
    exact hist_party_append hm.hist_party (fun a' ha' => by cases ha')
  · intro q h
    rw [hh, hhand]
    show commitsOf sk.rootH
        (if q == p then s.hist q ++ [.delivered g] else s.hist q) h
      = pushesOf
        (if q == p then s.hist q ++ [.delivered g] else s.hist q) h + _
    have hold := hm.hand_count q h
    by_cases hq : (q == p) = true
    · rw [if_pos hq,
        commitsOf_append_other sk.rootH (s.hist q)
          (fun a hc => MObs.noConfusion hc),
        pushesOf_append]
      omega
    · rw [if_neg hq]
      exact hold

-- =============================================== per-arm hand framing

/-- Hands framing for an arm that touches only the openers' choice
slots, leaving both wire-flags of the choices equal (a non-wire choice
or a non-wire fire). -/
private theorem handsEq_of_top {s s' : State}
    (hio : (s'.iopenCh == some IOblig.wire)
      = (s.iopenCh == some IOblig.wire))
    (hro : (s'.ropenCh == some ROblig.wire)
      = (s.ropenCh == some ROblig.wire))
    (hwk : ∀ pk, s'.walk pk = s.walk pk) :
    HandsEq sk s s' := by
  intro p h
  by_cases hr : h = sk.rootH
  · subst hr
    rw [holdsWire.eq_def, holdsWire.eq_def]
    simp only [beq_self_eq_true, if_pos]
    cases p
    · exact hio
    · exact hro
  · rw [holdsWire_eq_wireHand hr, holdsWire_eq_wireHand hr, hwk]

/-- `normWalk` of an uncommitted record stays uncommitted, so its
`wireHand` is off: both branches (`freshWalk` or the identity) carry
`committed = none`. -/
private theorem wireHand_normWalk_none {h : Nat} {ws : WalkSt}
    (hcm : ws.committed = none) :
    wireHand (normWalk sk h ws) = false := by
  rw [normWalk]
  split
  · simp [wireHand, freshWalk]
  · simp [wireHand, hcm]

/-- `fireOblig` clears the committed hand on every arm. -/
private theorem fireOblig_committed (ws : WalkSt) (o : Oblig) :
    (fireOblig ws o).committed = none := by
  cases o <;> rfl

-- ============================================ the base-arm dispatcher

set_option maxHeartbeats 1000000 in
/-- Every enabled muxed base action preserves the history ledger, at
every axiom mode and every skeleton.

The 23-arm dispatch, hands-only: each arm either moves no hand
(`histInv_of_act`) or is a wire commit flipping exactly one on
(`histInv_of_act_commit`). No `InvL`, no well-formedness — see the
module doc for why this sweep is generic where the stage-3 sweep is
not. -/
theorem histInv_applyBase {ax : AxMode} {a : Action} {s s' : MState}
    (hstep : applyBase sk ax a s = some s') (hm : HistInv sk s) :
    HistInv sk s' := by
  obtain ⟨hnf, b, hb, hs'⟩ := applyBase_inv hstep
  have hhist : s'.hist = recordObs s.hist (actionParty a) (.act a) := by
    rw [hs']
  have hbase : s'.base = b := by rw [hs']
  cases a with
  | iopenChoose o =>
      simp only [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue hg =>
        simp only [Bool.and_eq_true] at hg
        obtain ⟨hnone, -⟩ := hg
        have hio : s.base.iopenCh = none := by simpa using hnone
        injection hb with hbe
        cases o with
        | wire =>
            refine histInv_of_act_commit (p₀ := .I) (h₀ := sk.rootH)
              hm hhist rfl
              (fun h => by
                show (h == sk.rootH) = decide (h = sk.rootH)
                by_cases hh : h = sk.rootH <;> simp [hh])
              ?_ ?_ ?_
            · rw [holdsWire.eq_def, if_pos (by simp), hio]
              rfl
            · rw [hbase, ← hbe, holdsWire.eq_def, if_pos (by simp)]
              rfl
            · intro p h hne
              rw [hbase, ← hbe]
              by_cases hr : h = sk.rootH
              · subst hr
                have hp : p = Party.R := by
                  cases p
                  · exact absurd ⟨rfl, rfl⟩ hne
                  · rfl
                subst hp
                rw [holdsWire.eq_def, holdsWire.eq_def]
              · rw [holdsWire_eq_wireHand hr, holdsWire_eq_wireHand hr]
        | query =>
            refine histInv_of_act hm hhist ?_ (fun h => rfl)
            rw [hbase, ← hbe]
            exact handsEq_of_top (by rw [hio]; rfl) rfl (fun pk => rfl)
  | iopenFire =>
      have hch : s.base.iopenCh = some .query := by
        rw [Model.apply] at hb
        cases hio : s.base.iopenCh with
        | none => rw [hio] at hb; cases hb
        | some o =>
            cases o with
            | wire =>
                exfalso
                rw [isWireFire, hio] at hnf
                simp at hnf
            | query => rfl
      simp only [Model.apply, hch] at hb
      split at hb
      case isFalse => cases hb
      case isTrue =>
        injection hb with hbe
        refine histInv_of_act hm hhist ?_ (fun h => rfl)
        rw [hbase, ← hbe]
        exact handsEq_of_top (by rw [hch]; rfl) rfl (fun pk => rfl)
  | ropenRecv =>
      rw [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue =>
        injection hb with hbe
        refine histInv_of_act hm hhist ?_ (fun h => rfl)
        rw [hbase, ← hbe]
        exact handsEq_of_top rfl rfl (fun pk => rfl)
  | ropenChoose o =>
      simp only [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue hg =>
        simp only [Bool.and_eq_true] at hg
        obtain ⟨hnone, -⟩ := hg
        have hro : s.base.ropenCh = none := by simpa using hnone
        injection hb with hbe
        cases o with
        | wire =>
            refine histInv_of_act_commit (p₀ := .R) (h₀ := sk.rootH)
              hm hhist rfl
              (fun h => by
                show (h == sk.rootH) = decide (h = sk.rootH)
                by_cases hh : h = sk.rootH <;> simp [hh])
              ?_ ?_ ?_
            · rw [holdsWire.eq_def, if_pos (by simp), hro]
              rfl
            · rw [hbase, ← hbe, holdsWire.eq_def, if_pos (by simp)]
              rfl
            · intro p h hne
              rw [hbase, ← hbe]
              by_cases hr : h = sk.rootH
              · subst hr
                have hp : p = Party.I := by
                  cases p
                  · rfl
                  · exact absurd ⟨rfl, rfl⟩ hne
                subst hp
                rw [holdsWire.eq_def, holdsWire.eq_def]
              · rw [holdsWire_eq_wireHand hr, holdsWire_eq_wireHand hr]
        | res =>
            refine histInv_of_act hm hhist ?_ (fun h => rfl)
            rw [hbase, ← hbe]
            exact handsEq_of_top rfl (by rw [hro]; rfl) (fun pk => rfl)
        | query =>
            refine histInv_of_act hm hhist ?_ (fun h => rfl)
            rw [hbase, ← hbe]
            exact handsEq_of_top rfl (by rw [hro]; rfl) (fun pk => rfl)
  | ropenFire =>
      have hch : s.base.ropenCh = some .res ∨ s.base.ropenCh = some .query := by
        rw [Model.apply] at hb
        cases hro : s.base.ropenCh with
        | none => rw [hro] at hb; cases hb
        | some o =>
            cases o with
            | wire =>
                exfalso
                rw [isWireFire, hro] at hnf
                simp at hnf
            | res => exact Or.inl rfl
            | query => exact Or.inr rfl
      refine histInv_of_act hm hhist ?_ (fun h => rfl)
      rw [hbase]
      rcases hch with hch | hch <;>
        · simp only [Model.apply, hch] at hb
          split at hb
          case isFalse => cases hb
          case isTrue =>
            injection hb with hbe
            rw [← hbe]
            exact handsEq_of_top rfl (by rw [hch]; rfl) (fun pk => rfl)
  | walkRecvWire pk =>
      simp only [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq] at hg
        obtain ⟨⟨-, hph0⟩, -⟩ := hg
        injection hb with hbe
        refine histInv_of_act hm hhist ?_ (fun h => rfl)
        rw [hbase, ← hbe]
        refine handsEq_of_walk pk rfl rfl
          (fun pk' hne => setWalk_walk_ne _ _ hne) ?_ ?_
        · simp [wireHand, hph0]
        · rw [setWalk_walk_self]
          simp [wireHand]
  | walkRecvAsked pk =>
      simp only [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq] at hg
        obtain ⟨⟨-, hph1⟩, -⟩ := hg
        injection hb with hbe
        refine histInv_of_act hm hhist ?_ (fun h => rfl)
        rw [hbase, ← hbe]
        refine handsEq_of_walk pk rfl rfl
          (fun pk' hne => setWalk_walk_ne _ _ hne) ?_ ?_
        · simp [wireHand, hph1]
        · rw [setWalk_walk_self]
          exact wireHand_normWalk_none rfl
  | walkCommit pk o =>
      simp only [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue hg =>
        simp only [Bool.and_eq_true] at hg
        obtain ⟨hcon, hcho⟩ := hg
        have hph2 : (s.base.walk pk).phase = 2
            ∧ (s.base.walk pk).committed = none := by
          rw [wkChoosable] at hcho
          split at hcho
          · cases hcho
          next hcond =>
            simp only [Bool.or_eq_true, bne_iff_ne, ne_eq,
              Option.isSome_iff_exists, not_or, not_exists] at hcond
            refine ⟨by omega, ?_⟩
            cases hcm : (s.base.walk pk).committed with
            | none => rfl
            | some o' => exact absurd hcm (by simpa using hcond.2 o')
        have hmem : pk ∈ sk.walkKeys := (List.contains_iff_mem ..).mp hcon
        have hlt : pk.2 ≠ sk.rootH :=
          fun hcon' => absurd (walkKeys_height_lt hmem) (by omega)
        injection hb with hbe
        cases o with
        | wire i =>
            refine histInv_of_act_commit (p₀ := pk.1) (h₀ := pk.2)
              hm hhist rfl
              (fun h => by
                show (pk.2 == h) = decide (h = pk.2)
                by_cases hh : h = pk.2
                · subst hh
                  simp
                · have hne : ¬pk.2 = h := fun hc => hh hc.symm
                  simp [hh, hne])
              ?_ ?_ ?_
            · rw [holdsWire_eq_wireHand hlt]
              simp [wireHand, hph2.2]
            · rw [hbase, ← hbe, holdsWire_eq_wireHand hlt]
              rw [show (setWalk s.base pk
                    { s.base.walk pk with committed := some (.wire i) }).walk
                    (pk.1, pk.2) = { s.base.walk pk with
                      committed := some (.wire i) } by
                  rw [show ((pk.1, pk.2) : Party × Nat) = pk from rfl]
                  exact setWalk_walk_self _ _ _]
              rw [show sk.walkKeys.contains (pk.1, pk.2) = true from hcon]
              simp [wireHand, hph2.1]
            · intro p h hne
              rw [hbase, ← hbe]
              by_cases hr : h = sk.rootH
              · subst hr
                rw [holdsWire.eq_def, holdsWire.eq_def]
                simp only [beq_self_eq_true, if_pos]
                cases p <;> rfl
              · rw [holdsWire_eq_wireHand hr, holdsWire_eq_wireHand hr,
                  setWalk_walk_ne]
                intro hcon'
                exact hne ⟨congrArg Prod.fst hcon',
                  congrArg Prod.snd hcon'⟩
        | res i =>
            refine histInv_of_act hm hhist ?_ (fun h => rfl)
            rw [hbase, ← hbe]
            refine handsEq_of_walk pk rfl rfl
              (fun pk' hne => setWalk_walk_ne _ _ hne) ?_ ?_
            · simp [wireHand, hph2.2]
            · rw [setWalk_walk_self]
              simp [wireHand]
        | query i =>
            refine histInv_of_act hm hhist ?_ (fun h => rfl)
            rw [hbase, ← hbe]
            refine handsEq_of_walk pk rfl rfl
              (fun pk' hne => setWalk_walk_ne _ _ hne) ?_ ?_
            · simp [wireHand, hph2.2]
            · rw [setWalk_walk_self]
              simp [wireHand]
        | parent =>
            refine histInv_of_act hm hhist ?_ (fun h => rfl)
            rw [hbase, ← hbe]
            refine handsEq_of_walk pk rfl rfl
              (fun pk' hne => setWalk_walk_ne _ _ hne) ?_ ?_
            · simp [wireHand, hph2.2]
            · rw [setWalk_walk_self]
              simp [wireHand]
  | walkFire pk =>
      simp only [Model.apply] at hb
      split at hb
      next o hcm =>
        split at hb
        case isFalse => cases hb
        case isTrue =>
          have hnw : ∀ i, o ≠ Oblig.wire i := by
            intro i hcon
            subst hcon
            rw [isWireFire, hcm] at hnf
            cases hnf
          injection hb with hbe
          refine histInv_of_act hm hhist ?_ (fun h => rfl)
          rw [hbase, ← hbe]
          refine handsEq_of_walk pk rfl rfl
            (fun pk' hne => setWalk_walk_ne _ _ hne) ?_ ?_
          · rw [wireHand, hcm]
            cases o with
            | wire i => exact absurd rfl (hnw i)
            | res i => simp
            | query i => simp
            | parent => simp
          · rw [setWalk_walk_self]
            exact wireHand_normWalk_none (fireOblig_committed _ _)
      next => cases hb
  | walkCloseWire pk =>
      simp only [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq] at hg
        obtain ⟨⟨⟨-, hph⟩, -⟩, -⟩ := hg
        injection hb with hbe
        refine histInv_of_act hm hhist ?_ (fun h => rfl)
        rw [hbase, ← hbe]
        refine handsEq_of_walk pk rfl rfl
          (fun pk' hne => setWalk_walk_ne _ _ hne) ?_ ?_
        · simp [wireHand, hph]
        · rw [setWalk_walk_self]
          simp [wireHand]
  | walkCloseAsked pk =>
      simp only [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq] at hg
        obtain ⟨⟨⟨-, hph⟩, -⟩, -⟩ := hg
        injection hb with hbe
        refine histInv_of_act hm hhist ?_ (fun h => rfl)
        rw [hbase, ← hbe]
        refine handsEq_of_walk pk rfl rfl
          (fun pk' hne => setWalk_walk_ne _ _ hne) ?_ ?_
        · simp [wireHand, hph]
        · rw [setWalk_walk_self]
          simp [wireHand]
  | asmRecvRes pk =>
      simp only [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue =>
        injection hb with hbe
        refine histInv_of_act hm hhist ?_ (fun h => rfl)
        rw [hbase, ← hbe]
        exact handsEq_of_top rfl rfl (fun pk' => rfl)
  | asmRecvLevel pk =>
      simp only [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue =>
        injection hb with hbe
        refine histInv_of_act hm hhist ?_ (fun h => rfl)
        rw [hbase, ← hbe]
        exact handsEq_of_top rfl rfl (fun pk' => rfl)
  | asmSend pk =>
      simp only [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue =>
        injection hb with hbe
        refine histInv_of_act hm hhist ?_ (fun h => rfl)
        rw [hbase, ← hbe]
        exact handsEq_of_top rfl rfl (fun pk' => rfl)
  | asmClose pk =>
      simp only [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue =>
        injection hb with hbe
        refine histInv_of_act hm hhist ?_ (fun h => rfl)
        rw [hbase, ← hbe]
        exact handsEq_of_top rfl rfl (fun pk' => rfl)
  | absorbRecvWire =>
      rw [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue =>
        injection hb with hbe
        refine histInv_of_act hm hhist ?_ (fun h => rfl)
        rw [hbase, ← hbe]
        exact handsEq_of_top rfl rfl (fun pk' => rfl)
  | absorbRecvAsked =>
      rw [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue =>
        injection hb with hbe
        refine histInv_of_act hm hhist ?_ (fun h => rfl)
        rw [hbase, ← hbe]
        exact handsEq_of_top rfl rfl (fun pk' => rfl)
  | absorbSend =>
      rw [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue =>
        injection hb with hbe
        refine histInv_of_act hm hhist ?_ (fun h => rfl)
        rw [hbase, ← hbe]
        exact handsEq_of_top rfl rfl (fun pk' => rfl)
  | absorbCloseWire =>
      rw [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue =>
        injection hb with hbe
        refine histInv_of_act hm hhist ?_ (fun h => rfl)
        rw [hbase, ← hbe]
        exact handsEq_of_top rfl rfl (fun pk' => rfl)
  | absorbCloseAsked =>
      rw [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue =>
        injection hb with hbe
        refine histInv_of_act hm hhist ?_ (fun h => rfl)
        rw [hbase, ← hbe]
        exact handsEq_of_top rfl rfl (fun pk' => rfl)
  | finRet =>
      rw [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue =>
        injection hb with hbe
        refine histInv_of_act hm hhist ?_ (fun h => rfl)
        rw [hbase, ← hbe]
        exact handsEq_of_top rfl rfl (fun pk' => rfl)
  | finRes =>
      rw [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue =>
        injection hb with hbe
        refine histInv_of_act hm hhist ?_ (fun h => rfl)
        rw [hbase, ← hbe]
        exact handsEq_of_top rfl rfl (fun pk' => rfl)
  | finRets =>
      rw [Model.apply] at hb
      split at hb
      case isFalse => cases hb
      case isTrue =>
        injection hb with hbe
        refine histInv_of_act hm hhist ?_ (fun h => rfl)
        rw [hbase, ← hbe]
        exact handsEq_of_top rfl rfl (fun pk' => rfl)

-- ================================================== push and deliver

/-- A successful push preserves the history ledger, at every capacity:
the fired stream's hand flips off against its flush receipt. -/
theorem histInv_firePush {C : Nat} {p : Party} {h : Nat} {s s' : MState}
    (hfp : firePush sk C p h s = some s') (hm : HistInv sk s) :
    HistInv sk s' := by
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
                  refine histInv_of_pushed hm (by rw [← hs']) ?_ ?_ ?_
                  · rw [holdsWire.eq_def, if_pos (by simp), hch]
                    rfl
                  · rw [← hs', holdsWire.eq_def, if_pos (by simp)]
                    rfl
                  · intro q g hne
                    rw [← hs']
                    by_cases hg : g = sk.rootH
                    · subst hg
                      have hq : q = Party.R := by
                        cases q
                        · exact absurd ⟨rfl, rfl⟩ hne
                        · rfl
                      subst hq
                      rw [holdsWire.eq_def, holdsWire.eq_def]
                    · rw [holdsWire_eq_wireHand hg,
                        holdsWire_eq_wireHand hg]
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
                  refine histInv_of_pushed hm (by rw [← hs']) ?_ ?_ ?_
                  · rw [holdsWire.eq_def, if_pos (by simp), hch]
                    rfl
                  · rw [← hs', holdsWire.eq_def, if_pos (by simp)]
                    rfl
                  · intro q g hne
                    rw [← hs']
                    by_cases hg : g = sk.rootH
                    · subst hg
                      have hq : q = Party.I := by
                        cases q
                        · rfl
                        · exact absurd ⟨rfl, rfl⟩ hne
                      subst hq
                      rw [holdsWire.eq_def, holdsWire.eq_def]
                    · rw [holdsWire_eq_wireHand hg,
                        holdsWire_eq_wireHand hg]
    · -- a walk stream
      next hr =>
      have hr' : h ≠ sk.rootH := by simpa using hr
      split at hfp
      next i hcm =>
        split at hfp
        case isFalse => cases hfp
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq] at hg
          obtain ⟨hcon, hph⟩ := hg
          injection hfp with hs'
          refine histInv_of_pushed hm (by rw [← hs']) ?_ ?_ ?_
          · rw [holdsWire_eq_wireHand hr', hcon]
            simp [wireHand, hph, hcm]
          · rw [← hs', holdsWire_eq_wireHand hr']
            show (sk.walkKeys.contains (p, h)
              && wireHand ((setWalk s.base (p, h)
                (normWalk sk h (fireOblig (s.base.walk (p, h))
                  (.wire i)))).walk (p, h))) = false
            rw [setWalk_walk_self,
              wireHand_normWalk_none (fireOblig_committed _ _)]
            simp
          · intro q g hne
            rw [← hs']
            by_cases hg' : g = sk.rootH
            · subst hg'
              rw [holdsWire.eq_def, holdsWire.eq_def]
              simp only [beq_self_eq_true, if_pos]
              cases q <;> rfl
            · rw [holdsWire_eq_wireHand hg', holdsWire_eq_wireHand hg',
                setWalk_walk_ne]
              intro hcon'
              exact hne ⟨congrArg Prod.fst hcon',
                congrArg Prod.snd hcon'⟩
      next => cases hfp

/-- The record deliver preserves the history ledger: the head frame
moves cell-ward, touching no hand and no ledger row. -/
theorem histInv_deliver {ax : AxMode} {C : Nat} {σI σR : Strategy}
    {p : Party} {s s' : MState}
    (hstep : apply sk ax C σI σR (.deliver p) s = some s')
    (hm : HistInv sk s) : HistInv sk s' := by
  simp only [apply] at hstep
  split at hstep
  case h_2 => cases hstep
  case h_1 c rest hp =>
      split at hstep
      case isFalse => cases hstep
      case isTrue =>
        injection hstep with hs'
        exact histInv_of_delivered hm (by rw [← hs'])
          (by rw [← hs']; exact fun q g => holdsWire_chan_blind _ q g)

/-- The K-variant deliver preserves the history ledger: only the slot
guard differs from the record deliver, and the guard reads no hand. -/
theorem histInv_deliverK {KI KR : Nat} {p : Party} {s s' : MState}
    (hstep : deliverStepK KI KR p s = some s') (hm : HistInv sk s) :
    HistInv sk s' := by
  simp only [deliverStepK] at hstep
  split at hstep
  case h_2 => cases hstep
  case h_1 c rest hp =>
      split at hstep
      case isFalse => cases hstep
      case isTrue =>
        injection hstep with hs'
        exact histInv_of_delivered hm (by rw [← hs'])
          (by rw [← hs']; exact fun q g => holdsWire_chan_blind _ q g)

/-- The elastic deliver preserves the history ledger: parking is
unbounded but still touches no hand and no ledger row. -/
theorem histInv_deliverE {p : Party} {s s' : MState}
    (hstep : deliverStepE p s = some s') (hm : HistInv sk s) :
    HistInv sk s' := by
  simp only [deliverStepE] at hstep
  split at hstep
  case h_2 => cases hstep
  case h_1 c rest hp =>
      injection hstep with hs'
      exact histInv_of_delivered hm (by rw [← hs'])
        (by rw [← hs']; exact fun q g => holdsWire_chan_blind _ q g)

-- ======================================== reachability, three universes

/-- No committed wire hand exists at the initial state (the
`SigmaStarInv` fact, restated here because that file keeps it
private). -/
private theorem holdsWire_init' (p : Party) (h : Nat) :
    holdsWire sk p h (init sk).base = false := by
  rw [holdsWire.eq_def]
  split
  · cases p <;> rfl
  · show (sk.walkKeys.contains (p, h)
      && ((Model.init sk).walk (p, h)).phase == 2
      && _) = false
    have hc : ((init sk).base.walk (p, h)).committed = none := by
      show (freshWalk sk h 0).committed = none
      rw [freshWalk]
    rw [hc]
    simp

/-- The history ledger holds at the initial state: empty histories,
no hands. -/
theorem histInv_init : HistInv sk (init sk) := by
  constructor
  · intro p a hmem
    cases hmem
  · intro p h
    rw [show (init sk).hist p = [] from rfl, holdsWire_init']
    rfl

/-- One record-semantics step preserves the history ledger, at every
axiom mode, capacity, and strategy pair. -/
theorem histInv_step {ax : AxMode} {C : Nat} {σI σR : Strategy}
    {a : MAction} {s s' : MState}
    (hstep : apply sk ax C σI σR a s = some s') (hm : HistInv sk s) :
    HistInv sk s' := by
  cases a with
  | base a => exact histInv_applyBase hstep hm
  | push p =>
      simp only [apply] at hstep
      split at hstep
      next h hσ => exact histInv_firePush hstep hm
      next => cases hstep
  | deliver p => exact histInv_deliver hstep hm

/-- The history ledger holds at every record-reachable muxed state —
strategy-, mode-, capacity-, and skeleton-generic. -/
theorem histInv_reachable {ax : AxMode} {C : Nat} {σI σR : Strategy}
    {s : MState} (hr : MReachable sk ax C σI σR s) : HistInv sk s := by
  induction hr with
  | init => exact histInv_init
  | step a hr' hstep ih => exact histInv_step hstep ih

/-- The history ledger holds on the whole record state universe the
strategy classes quantify over. -/
theorem histInv_reachable_any {s : MState} (hr : MReachableAny sk s) :
    HistInv sk s := by
  obtain ⟨ax, C, σI, σR, hr⟩ := hr
  exact histInv_reachable hr

/-- The history ledger holds on the K-variant state universe: base and
push arms are shared with the record semantics, and `deliverStepK`
touches no ledger row. -/
theorem histInv_reachable_anyK {s : MState} (hr : KMReachableAny sk s) :
    HistInv sk s := by
  obtain ⟨ax, KI, KR, C, σI, σR, hr⟩ := hr
  induction hr with
  | init => exact histInv_init
  | step a hr' hstep ih =>
      cases a with
      | base a => exact histInv_applyBase hstep ih
      | push p =>
          simp only [applyK, apply] at hstep
          split at hstep
          next h hσ => exact histInv_firePush hstep ih
          next => cases hstep
      | deliver p => exact histInv_deliverK hstep ih

/-- The history ledger holds on the elastic state universe: base and
push arms are shared with the record semantics, and `deliverStepE`
touches no ledger row. -/
theorem histInv_reachable_anyE {s : MState} (hr : EMReachableAny sk s) :
    HistInv sk s := by
  obtain ⟨ax, C, σI, σR, hr⟩ := hr
  induction hr with
  | init => exact histInv_init
  | step a hr' hstep ih =>
      cases a with
      | base a => exact histInv_applyBase hstep ih
      | push p =>
          simp only [applyE, apply] at hstep
          split at hstep
          next h hσ => exact histInv_firePush hstep ih
          next => cases hstep
      | deliver p => exact histInv_deliverE hstep ih

-- ====================================== the work-conservation witness

/-- Wherever the enabled-push set is nonempty, the shipped policy names
a member of it: the ledger decode (`committedInHist_iff_holdsWire`)
turns the history scan into a `holdsWire` scan, and a held stream is
always inside the scan range and the wire-height family. -/
theorem bottomMostReady_names_enabled {s : MState} (hm : HistInv sk s)
    {C : Nat} {p : Party} (hne : enabledPushes sk C p s ≠ []) :
    ∃ h, bottomMostReady sk (s.hist p) = some h
      ∧ h ∈ enabledPushes sk C p s := by
  -- a held witness exists in the scan range
  obtain ⟨h₀, hh₀⟩ := List.exists_mem_of_ne_nil _ hne
  obtain ⟨hroom, hw₀⟩ := mem_enabledPushes.mp hh₀
  have hmem₀ : h₀ ∈ List.range (sk.rootH + 1) := by
    have := holdsWire_le_rootH hw₀
    exact List.mem_range.mpr (by omega)
  have hpred₀ : committedInHist sk.rootH (s.hist p) h₀ = true := by
    rw [committedInHist_iff_holdsWire hm]
    exact hw₀
  cases hfind : bottomMostReady sk (s.hist p) with
  | none =>
      exfalso
      rw [bottomMostReady] at hfind
      exact absurd hpred₀
        (by simpa using List.find?_eq_none.mp hfind h₀ hmem₀)
  | some h =>
      refine ⟨h, rfl, ?_⟩
      have hpred : committedInHist sk.rootH (s.hist p) h = true :=
        List.find?_some (by rw [bottomMostReady] at hfind; exact hfind)
      have hw : holdsWire sk p h s.base = true := by
        rw [← committedInHist_iff_holdsWire hm]
        exact hpred
      exact mem_enabledPushes.mpr ⟨hroom, hw⟩

/-- The shipped policy is work-conserving: the mandated
`bottomMostReady_wc`, the non-vacuity certificate for
`wc_impossibility`'s and T6's hypothesis class.

At the class's full generality — every axiom mode, every skeleton,
well-formed or not — because the carrying `HistInv` sweep is generic
(module doc). -/
theorem bottomMostReady_wc (p : Party) :
    WorkConserving p bottomMostReady := by
  intro sk C s hr hne
  exact bottomMostReady_names_enabled (histInv_reachable_any hr) hne

/-- The shipped policy is work-conserving on the K-variant universe:
`wc_impossibility_K` quantifies over a kernel-inhabited class. -/
theorem bottomMostReady_wcK (p : Party) :
    KWorkConserving p bottomMostReady := by
  intro sk C s hr hne
  exact bottomMostReady_names_enabled (histInv_reachable_anyK hr) hne

/-- The shipped policy is work-conserving on the elastic universe:
`elastic_deadlock_free` quantifies over a kernel-inhabited class. -/
theorem bottomMostReady_wcE (p : Party) :
    EWorkConserving p bottomMostReady := by
  intro sk C s hr hne
  exact bottomMostReady_names_enabled (histInv_reachable_anyE hr) hne

-- ================================================ the locality witnesses

/-- The shipped policy is local: the mandated `bottomMostReady_local`.

Stronger than the class demands: the policy reads only `rootH` (a
commonly-known session parameter, the first `LocalEq` conjunct) and its
own observation history, so invariance needs neither the view
projection nor the `Consistent` guards. Together with
`bottomMostReady_wc` this pins a strategy that is simultaneously
work-conserving and local — the class the impossibility theorems
indict contains the shipped mux, kernel-checked from both sides. -/
theorem bottomMostReady_local (p : Party) :
    LocalStrategy p bottomMostReady := by
  intro sk sk' tr hleq _ _
  have hroot : sk.rootH = sk'.rootH := by
    rw [LocalEq] at hleq
    simp only [Bool.and_eq_true, beq_iff_eq] at hleq
    exact hleq.1.1.1
  simp only [bottomMostReady, hroot]

/-- The idler is local: the trivial `LocalStrategy` inhabitant, pinning
the class non-vacuous independently of any policy (the `LocalEq`
nondegeneracy control pins the relation itself). -/
theorem idler_local (p : Party) :
    LocalStrategy p (fun _ _ => none) :=
  fun _ _ _ _ _ _ => rfl

end StreamingMirror.Mux
