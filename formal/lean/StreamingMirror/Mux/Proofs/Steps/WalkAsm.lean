/-
Per-arm step facts for the walk stages (minus fires) and the
assemblers: the InvL bullet and count deltas of each base action,
extracted from the Preserve monoliths' local parts (Steps.lean's module
doc explains why the monoliths themselves cannot be invoked at muxed
states).
-/
import StreamingMirror.Mux.Proofs.Steps
import StreamingMirror.Proofs.Preserve.Asm

namespace StreamingMirror.Mux

open Model

variable {sk : Skel} {ax : AxMode} {s s' : State}

-- ====================================================== walk receives

/-- `walkRecvWire`: the prologue wire receive, phase 0 → 1 — one
receive on `wireIn pk`, no wire hand touched on either side. -/
theorem step_walkRecvWire (hwf : sk.wellFormed = true) (pk : Party × Nat)
    (hstep : Model.apply sk ax (.walkRecvWire pk) s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ RecvStep sk s s' (wireIn pk) ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph0⟩, hpos⟩ := hg
    have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
    injection hstep with hs'
    have hwalk : s'.walk pk
        = { s.walk pk with phase := 1, committed := none } := by
      rw [← hs']; simp
    have hwk : ∀ pk' ∈ sk.walkKeys, wkLocalOk sk ax s' pk' = true := by
      intro pk' hpk'
      by_cases hpkeq : pk' = pk
      · subst hpkeq
        have hwk0 := hL.wk pk' hpk'
        simp only [wkLocalOk, hwalk] at hwk0 ⊢
        rw [hph0] at hwk0
        simp at hwk0 ⊢
        exact ⟨hwk0.1, hwk0.2.1⟩
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']; exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hL.wk pk' hpk'
    have hsent : ∀ c ∈ allChans sk, sentOf sk s' c = sentOf sk s c := by
      intro c hc
      rw [← hs']
      exact sentOf_setWalk_same hwf _ pk
        { s.walk pk with phase := 1, committed := none } hmem'
        (by simp [wkWireSent, wkWireCount])
        (by simp [wkResSent, wkResCount])
        (by simp [wkQSentTot, wkQSum])
        (by simp [wkParentSent, hph0])
        hc
    have hrecvd : ∀ c ∈ allChans sk,
        recvdOf sk s' c = recvdOf sk s c
          + (if c = wireIn pk then 1 else 0) := by
      intro c hc
      by_cases h5 : c = wireIn pk
      · subst h5
        have hr' : recvdOf sk s' (wireIn pk)
            = wkWireRecvd sk s pk + 1 := by
          rw [← hs', recvdOf_wireIn hmem']
          simp [wkWireRecvd, hph0]
        have hr0 : recvdOf sk s (wireIn pk) = wkWireRecvd sk s pk :=
          recvdOf_wireIn hmem'
        simp [hr', hr0]
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          by_cases h6 : c = askedIn pk
          · subst h6
            rw [← hs', recvdOf_askedIn, recvdOf_askedIn]
            simp [wkAskedRecvd, hph0]
          · rw [← hs']
            exact recvdOf_setWalk_frame hwf _ pk _ hc h5 h6
        simp [hrecv, h5]
    have hhands : HandsEq sk s s' :=
      handsEq_of_walk pk (by rw [← hs']; rfl) (by rw [← hs']; rfl)
        (fun pk' hpkne => by rw [← hs']; exact setWalk_walk_ne _ _ hpkne)
        (by simp [wireHand, hph0]) (by rw [hwalk]; simp [wireHand])
    exact ⟨⟨hwk, fun pk' hpk' => by rw [← hs']; exact hL.asm pk' hpk',
        by rw [← hs']; exact hL.top⟩,
      ⟨hpos, by rw [← hs']; rfl, hsent, hrecvd⟩, hhands⟩

/-- `walkRecvAsked`: the prologue query receive, phase 1 → 2 — one
receive on `askedIn pk`; the embedded `normWalk` is the identity (the
freshly-phase-2 walk has empty machinery), so the walk lands
uncommitted and no hand appears. -/
theorem step_walkRecvAsked (hwf : sk.wellFormed = true) (pk : Party × Nat)
    (hstep : Model.apply sk ax (.walkRecvAsked pk) s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ RecvStep sk s s' (askedIn pk) ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph1⟩, hpos⟩ := hg
    have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
    injection hstep with hs'
    -- phase-1 facts from the invariant: cursor in range, machinery empty
    have hwk0 := hL.wk pk hmem'
    simp only [wkLocalOk] at hwk0
    rw [hph1] at hwk0
    simp at hwk0
    obtain ⟨hslt, ⟨hledger, hpd⟩, hcm⟩ := hwk0
    -- normWalk is the identity on the freshly-phase-2 walk
    have hnlt : ¬ (s.walk pk).scope ≥ sk.stageLen pk.2 := by omega
    have hscF : scopeComplete sk pk.2
        { s.walk pk with phase := 2, committed := none } = false := by
      simp [scopeComplete, hnlt, hpd]
    have hnw : normWalk sk pk.2
        { s.walk pk with phase := 2, committed := none }
        = { s.walk pk with phase := 2, committed := none } := by
      simp [normWalk, hscF]
    have hwalk : s'.walk pk
        = { s.walk pk with phase := 2, committed := none } := by
      rw [← hs']; simp [hnw]
    have hwk : ∀ pk' ∈ sk.walkKeys, wkLocalOk sk ax s' pk' = true := by
      intro pk' hpk'
      by_cases hpkeq : pk' = pk
      · subst hpkeq
        simp only [wkLocalOk, hwalk, hscF]
        simp [hslt]
        intro j hj
        simp [hledger j hj]
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']; exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hL.wk pk' hpk'
    have hsent : ∀ c ∈ allChans sk, sentOf sk s' c = sentOf sk s c := by
      intro c hc
      rw [← hs']
      rw [show setWalk { s with chan := bump s.chan (askedIn pk) (-1) } pk
          (normWalk sk pk.2
            { s.walk pk with phase := 2, committed := none })
          = setWalk { s with chan := bump s.chan (askedIn pk) (-1) } pk
            { s.walk pk with phase := 2, committed := none } from by
        rw [hnw]]
      exact sentOf_setWalk_same hwf _ pk
        { s.walk pk with phase := 2, committed := none } hmem'
        (by simp [wkWireSent, wkWireCount])
        (by simp [wkResSent, wkResCount])
        (by simp [wkQSentTot, wkQSum])
        (by simp [wkParentSent, hph1, hpd])
        hc
    have hrecvd : ∀ c ∈ allChans sk,
        recvdOf sk s' c = recvdOf sk s c
          + (if c = askedIn pk then 1 else 0) := by
      intro c hc
      by_cases h6 : c = askedIn pk
      · subst h6
        have hr' : recvdOf sk s' (askedIn pk)
            = wkAskedRecvd sk s pk + 1 := by
          rw [← hs', recvdOf_askedIn]
          simp [wkAskedRecvd, hnw, hph1]
        have hr0 : recvdOf sk s (askedIn pk) = wkAskedRecvd sk s pk :=
          recvdOf_askedIn
        simp [hr', hr0]
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          by_cases h5 : c = wireIn pk
          · subst h5
            rw [← hs', recvdOf_wireIn hmem', recvdOf_wireIn hmem']
            simp [wkWireRecvd, hnw, hph1]
          · rw [← hs']
            exact recvdOf_setWalk_frame hwf _ pk _ hc h5 h6
        simp [hrecv, h6]
    have hhands : HandsEq sk s s' :=
      handsEq_of_walk pk (by rw [← hs']; rfl) (by rw [← hs']; rfl)
        (fun pk' hpkne => by rw [← hs']; exact setWalk_walk_ne _ _ hpkne)
        (by simp [wireHand, hph1]) (by rw [hwalk]; simp [wireHand])
    exact ⟨⟨hwk, fun pk' hpk' => by rw [← hs']; exact hL.asm pk' hpk',
        by rw [← hs']; exact hL.top⟩,
      ⟨hpos, by rw [← hs']; rfl, hsent, hrecvd⟩, hhands⟩

-- ======================================================== walk commits

/-- Shared spine of the four commit lemmas: `InvL` preservation, the
quiet count frame, and the guard facts the hands clauses read off.

`wellFormed` is demanded only by the `.wire` arm (the frontier-counting
argument needs the fan bound), so it arrives conditionalized on the
obligation shape: the non-wire callers discharge it by `nomatch`. -/
private theorem walkCommit_core (pk : Party × Nat) (o : Oblig)
    (hwf : ∀ i, o = .wire i → sk.wellFormed = true)
    (hstep : Model.apply sk ax (.walkCommit pk o) s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ QuietStep sk s s' ∧ pk ∈ sk.walkKeys
      ∧ (s.walk pk).phase = 2 ∧ (s.walk pk).committed = none
      ∧ s'.walk pk = { s.walk pk with committed := some o }
      ∧ s'.iopenCh = s.iopenCh ∧ s'.ropenCh = s.ropenCh
      ∧ (∀ pk', pk' ≠ pk → s'.walk pk' = s.walk pk') := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true] at hg
    obtain ⟨hmem, hch⟩ := hg
    injection hstep with hs'
    rw [wkChoosable] at hch
    split at hch
    case isTrue => cases hch
    case isFalse hpc =>
      have hph2 : (s.walk pk).phase = 2 := by
        by_contra hne
        exact hpc (by simp [hne])
      have hcm : (s.walk pk).committed = none := by
        cases hcmv : (s.walk pk).committed with
        | none => rfl
        | some x => exact absurd (by simp [hcmv]) hpc
      have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
      have hwalk : s'.walk pk
          = { s.walk pk with committed := some o } := by
        rw [← hs']; simp
      have hio : s'.iopenCh = s.iopenCh := by rw [← hs']; rfl
      have hro : s'.ropenCh = s.ropenCh := by rw [← hs']; rfl
      have hwne : ∀ pk', pk' ≠ pk → s'.walk pk' = s.walk pk' := by
        intro pk' hpkne
        rw [← hs']; exact setWalk_walk_ne s _ hpkne
      have hquiet : QuietStep sk s s' := by
        refine ⟨by rw [← hs']; rfl, fun c _ => ?_, fun c _ => ?_⟩
        · rw [← hs', sentOf_setWalk_committed sk s pk (some o) c]
        · rw [← hs', recvdOf_setWalk_committed sk s pk (some o) c]
      refine ⟨⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩, hquiet,
        hmem', hph2, hcm, hwalk, hio, hro, hwne⟩
      · by_cases hpkeq : pk' = pk
        · subst hpkeq
          have hcount : wkWireCount sk s' pk'
              = wkWireCount sk s pk' := by
            simp [wkWireCount, hwalk]
          have hsc : scopeComplete sk pk'.2
              { s.walk pk' with committed := some o }
              = scopeComplete sk pk'.2 (s.walk pk') := rfl
          have hwk := hL.wk pk' hpk'
          simp only [wkLocalOk, hwalk, hcount, hsc] at hwk ⊢
          rw [hph2] at hwk ⊢
          rw [hcm] at hwk
          simp at hwk ⊢
          obtain ⟨hA, hB, hC⟩ := hwk
          refine ⟨hA, ⟨hB, hC⟩, ?_⟩
          -- the committed arm: three of four are the guard verbatim
          cases o with
          | res i => exact hch
          | query i => exact hch
          | parent => exact hch
          | wire i =>
              simp only [Bool.and_eq_true, decide_eq_true_eq,
                Bool.not_eq_true', List.all_eq_true,
                List.mem_range] at hch
              obtain ⟨⟨⟨⟨hin, hfront⟩, hlow⟩, hd4⟩, hd5⟩ := hch
              simp only [Bool.and_eq_true, beq_iff_eq,
                decide_eq_true_eq]
              have hn : sk.nChildren pk'.snd
                  (sk.stageScope pk'.snd (s.walk pk').scope)
                    ≤ sk.fan :=
                nChildren_le_fan (hwf i rfl) hA
              have hclosed : ∀ j < sk.fan,
                  (s.walk pk').wireDone j = true →
                  j = 0 ∨ (s.walk pk').wireDone (j - 1) = true := by
                intro j hj hwd
                rcases (hC j hj).1.1.1.1.1.1.1.1.1 with hf | ⟨-, h0⟩
                · rw [hwd] at hf; cases hf
                · exact h0
              refine ⟨⟨⟨?_, hin⟩, hd4⟩, hd5⟩
              rw [wkWireCount]
              exact (length_filter_of_frontier (by omega) hlow hfront
                hclosed).symm
        · have hw : s'.walk pk' = s.walk pk' := by
            rw [← hs']; exact setWalk_walk_ne s _ hpkeq
          rw [wkLocalOk_congr sk ax pk' hw]
          exact hL.wk pk' hpk'
      · rw [← hs']; exact hL.asm pk' hpk'
      · rw [← hs']; exact hL.top

/-- `walkCommit` on a wire obligation: quiet counts, and exactly walk
`pk`'s committed wire hand flips on — the one hands-changing arm of
this file. -/
theorem step_walkCommit_wire (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (i : Nat)
    (hstep : Model.apply sk ax (.walkCommit pk (.wire i)) s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ QuietStep sk s s'
      ∧ pk ∈ sk.walkKeys
      ∧ holdsWire sk pk.1 pk.2 s = false
      ∧ holdsWire sk pk.1 pk.2 s' = true
      ∧ ∀ p h, (p, h) ≠ pk →
          holdsWire sk p h s' = holdsWire sk p h s := by
  obtain ⟨hInv, hquiet, hmem', hph2, hcm, hwalk, hio, hro, hwne⟩ :=
    walkCommit_core pk (.wire i) (fun _ _ => hwf) hstep hL
  have hroot : pk.2 ≠ sk.rootH := by
    rcases walkKeys_cases hmem' with ⟨-, -, hb⟩ | ⟨-, hb⟩ <;> omega
  refine ⟨hInv, hquiet, hmem', ?_, ?_, ?_⟩
  · rw [holdsWire_eq_wireHand hroot]
    simp [wireHand, hcm]
  · rw [holdsWire_eq_wireHand hroot]
    simp [wireHand, hwalk, hph2, hmem']
  · intro p h hne
    by_cases hr : h = sk.rootH
    · subst hr
      rw [holdsWire.eq_def, holdsWire.eq_def]
      simp only [beq_self_eq_true, if_pos]
      cases p
      · rw [hio]
      · rw [hro]
    · rw [holdsWire_eq_wireHand hr, holdsWire_eq_wireHand hr,
        hwne _ hne]

/-- `walkCommit` on a res obligation: a pure choice — quiet counts,
hands unchanged (a non-wire commitment is never a hand). -/
theorem step_walkCommit_res (pk : Party × Nat) (i : Nat)
    (hstep : Model.apply sk ax (.walkCommit pk (.res i)) s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ QuietStep sk s s' ∧ HandsEq sk s s' := by
  obtain ⟨hInv, hquiet, _hmem, _hph2, hcm, hwalk, hio, hro, hwne⟩ :=
    walkCommit_core pk (.res i) (fun _ h => nomatch h) hstep hL
  exact ⟨hInv, hquiet, handsEq_of_walk pk hio hro hwne
    (by simp [wireHand, hcm]) (by rw [hwalk]; simp [wireHand])⟩

/-- `walkCommit` on a query obligation: a pure choice — quiet counts,
hands unchanged. -/
theorem step_walkCommit_query (pk : Party × Nat) (i : Nat)
    (hstep : Model.apply sk ax (.walkCommit pk (.query i)) s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ QuietStep sk s s' ∧ HandsEq sk s s' := by
  obtain ⟨hInv, hquiet, _hmem, _hph2, hcm, hwalk, hio, hro, hwne⟩ :=
    walkCommit_core pk (.query i) (fun _ h => nomatch h) hstep hL
  exact ⟨hInv, hquiet, handsEq_of_walk pk hio hro hwne
    (by simp [wireHand, hcm]) (by rw [hwalk]; simp [wireHand])⟩

/-- `walkCommit` on the parent obligation: a pure choice — quiet
counts, hands unchanged. -/
theorem step_walkCommit_parent (pk : Party × Nat)
    (hstep : Model.apply sk ax (.walkCommit pk .parent) s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ QuietStep sk s s' ∧ HandsEq sk s s' := by
  obtain ⟨hInv, hquiet, _hmem, _hph2, hcm, hwalk, hio, hro, hwne⟩ :=
    walkCommit_core pk .parent (fun _ h => nomatch h) hstep hL
  exact ⟨hInv, hquiet, handsEq_of_walk pk hio hro hwne
    (by simp [wireHand, hcm]) (by rw [hwalk]; simp [wireHand])⟩

-- ======================================================== walk closes

/-- `walkCloseWire`: the end-of-stream observation, phase 3 → 4 — no
channel operation, no hand (the walk is past its publishing phase on
both sides). -/
theorem step_walkCloseWire (hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.walkCloseWire pk) s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ QuietStep sk s s' ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨⟨hmem, hph3⟩, _hpd⟩, _hz⟩ := hg
    have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
    injection hstep with hs'
    have hwalk : s'.walk pk = { s.walk pk with phase := 4 } := by
      rw [← hs']; simp
    have hwk : ∀ pk' ∈ sk.walkKeys, wkLocalOk sk ax s' pk' = true := by
      intro pk' hpk'
      by_cases hpkeq : pk' = pk
      · subst hpkeq
        have hwk0 := hL.wk pk' hpk'
        simp only [wkLocalOk, hwalk] at hwk0 ⊢
        rw [hph3] at hwk0
        simp at hwk0 ⊢
        exact hwk0
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']; exact setWalk_walk_ne s _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hL.wk pk' hpk'
    have hquiet : QuietStep sk s s' := by
      refine ⟨by rw [← hs']; rfl, fun c hc => ?_, fun c hc => ?_⟩
      · rw [← hs']
        exact sentOf_setWalk_same hwf s pk
          { s.walk pk with phase := 4 } hmem'
          (by simp [wkWireSent, wkWireCount])
          (by simp [wkResSent, wkResCount])
          (by simp [wkQSentTot, wkQSum])
          (by simp [wkParentSent, hph3])
          hc
      · rw [← hs']
        exact recvdOf_setWalk_same hwf s pk
          { s.walk pk with phase := 4 } hmem'
          (by simp [wkWireRecvd, hph3])
          (by simp [wkAskedRecvd, hph3])
          hc
    have hhands : HandsEq sk s s' :=
      handsEq_of_walk pk (by rw [← hs']; rfl) (by rw [← hs']; rfl)
        (fun pk' hpkne => by rw [← hs']; exact setWalk_walk_ne _ _ hpkne)
        (by simp [wireHand, hph3]) (by rw [hwalk]; simp [wireHand])
    exact ⟨⟨hwk, fun pk' hpk' => by rw [← hs']; exact hL.asm pk' hpk',
        by rw [← hs']; exact hL.top⟩, hquiet, hhands⟩

/-- `walkCloseAsked`: the second close observation, phase 4 → 5 — same
shape as `walkCloseWire`. -/
theorem step_walkCloseAsked (hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.walkCloseAsked pk) s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ QuietStep sk s s' ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨⟨hmem, hph4⟩, _hpd⟩, _hz⟩ := hg
    have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
    injection hstep with hs'
    have hwalk : s'.walk pk = { s.walk pk with phase := 5 } := by
      rw [← hs']; simp
    have hwk : ∀ pk' ∈ sk.walkKeys, wkLocalOk sk ax s' pk' = true := by
      intro pk' hpk'
      by_cases hpkeq : pk' = pk
      · subst hpkeq
        have hwk0 := hL.wk pk' hpk'
        simp only [wkLocalOk, hwalk] at hwk0 ⊢
        rw [hph4] at hwk0
        simp at hwk0 ⊢
        exact hwk0
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']; exact setWalk_walk_ne s _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hL.wk pk' hpk'
    have hquiet : QuietStep sk s s' := by
      refine ⟨by rw [← hs']; rfl, fun c hc => ?_, fun c hc => ?_⟩
      · rw [← hs']
        exact sentOf_setWalk_same hwf s pk
          { s.walk pk with phase := 5 } hmem'
          (by simp [wkWireSent, wkWireCount])
          (by simp [wkResSent, wkResCount])
          (by simp [wkQSentTot, wkQSum])
          (by simp [wkParentSent, hph4])
          hc
      · rw [← hs']
        exact recvdOf_setWalk_same hwf s pk
          { s.walk pk with phase := 5 } hmem'
          (by simp [wkWireRecvd, hph4])
          (by simp [wkAskedRecvd, hph4])
          hc
    have hhands : HandsEq sk s s' :=
      handsEq_of_walk pk (by rw [← hs']; rfl) (by rw [← hs']; rfl)
        (fun pk' hpkne => by rw [← hs']; exact setWalk_walk_ne _ _ hpkne)
        (by simp [wireHand, hph4]) (by rw [hwalk]; simp [wireHand])
    exact ⟨⟨hwk, fun pk' hpk' => by rw [← hs']; exact hL.asm pk' hpk',
        by rw [← hs']; exact hL.top⟩, hquiet, hhands⟩

-- ========================================================== assemblers

/-- `asmRecvRes`: one resolution consumed — a receive on
`asmResChan pk`, phase 0 → 1/2; no walk or opener slot moves. -/
theorem step_asmRecvRes (hwf : sk.wellFormed = true) (pk : Party × Nat)
    (hstep : Model.apply sk ax (.asmRecvRes pk) s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ RecvStep sk s s' (asmResChan pk) ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph⟩, hpos⟩ := hg
    injection hstep with hs'
    have hpk2 : 1 ≤ pk.2 := asmKeys_snd_pos hmem
    have hpkmem : pk ∈ sk.asmKeys := List.contains_iff_mem.mp hmem
    have hold := hL.asm pk hpkmem
    simp only [asmLocalOk, hph] at hold
    simp at hold
    have hasmL : ∀ pk' ∈ sk.asmKeys, asmLocalOk sk s' pk' = true := by
      intro pk' hpk'
      by_cases hpkeq : pk' = pk
      · subst hpkeq
        have hasm : s'.asm pk' = { s.asm pk' with
            phase := if sk.pendAt pk'.1 pk'.2 (s.asm pk').idx > 0
              then 1 else 2,
            got := 0 } := by rw [← hs']; simp
        by_cases hpend : sk.pendAt pk'.1 pk'.2 (s.asm pk').idx > 0
        · simp [asmLocalOk, hasm, hpend]
          omega
        · simp [asmLocalOk, hasm, hpend]
          omega
      · have ha : s'.asm pk' = s.asm pk' := by
          rw [← hs']; exact setAsm_asm_ne _ _ hpkeq
        rw [asmLocalOk_congr sk pk' ha]; exact hL.asm pk' hpk'
    have hsent : ∀ c ∈ allChans sk, sentOf sk s' c = sentOf sk s c := by
      intro c _hc
      rw [← hs']
      apply sentOf_ext_idx
      · intro pk''
        by_cases hqq : pk'' = pk
        · subst hqq; simp
        · simp [setAsm_asm_ne _ _ hqq]
      all_goals rfl
    have hrecvd : ∀ c ∈ allChans sk,
        recvdOf sk s' c = recvdOf sk s c
          + (if c = asmResChan pk then 1 else 0) := by
      intro c hc
      by_cases hcc : c = asmResChan pk
      · subst hcc
        have h21 : pk.2 - 1 + 1 = pk.2 := by omega
        have hkey : ((pk.1, pk.2 - 1 + 1) : Party × Nat) = pk := by
          rw [h21]
        by_cases hask : asks pk.1 pk.2 = true
        · have hch : asmResChan pk = Chan.upper pk.1 (pk.2 - 1) := by
            simp [asmResChan, hask]
          have hrecvS : recvdOf sk s (asmResChan pk)
              = (s.asm pk).idx := by
            rw [hch]
            show asmResRecvd s (pk.1, pk.2 - 1 + 1) = _
            rw [hkey]
            simp [asmResRecvd, hph]
          have hrecvS' : recvdOf sk s' (asmResChan pk)
              = (s.asm pk).idx + 1 := by
            rw [← hs', hch]
            show asmResRecvd (setAsm _ pk _) (pk.1, pk.2 - 1 + 1) = _
            rw [hkey]
            simp only [asmResRecvd, setAsm_asm_self]
            split <;> simp
          rw [hrecvS', hrecvS]
          simp
        · have hch : asmResChan pk = Chan.lower pk.1 pk.2 := by
            simp [asmResChan, hask]
          have hctm : ((pk.1, pk.2) : Party × Nat) ∈ sk.asmKeys :=
            hpkmem
          have hrecvS : recvdOf sk s (asmResChan pk)
              = (s.asm pk).idx := by
            rw [hch]
            simp [recvdOf, hctm, asmResRecvd, hph]
          have hrecvS' : recvdOf sk s' (asmResChan pk)
              = (s.asm pk).idx + 1 := by
            rw [← hs', hch]
            simp only [recvdOf]
            rw [if_pos (by exact hmem)]
            simp only [asmResRecvd, setAsm_asm_self]
            split <;> simp
          rw [hrecvS', hrecvS]
          simp
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          refine Eq.trans
            (recvdOf_setAsm_frame_res hwf _ pk _ hc hcc ?_) ?_
          · simp [asmLevelRecvd, hold.2]
          · exact recvdOf_ext sk (fun _ => rfl) (fun _ => rfl)
              (fun _ => rfl) rfl rfl rfl rfl rfl rfl c
        simp [hrecv, hcc]
    exact ⟨⟨fun pk' hpk' => by rw [← hs']; exact hL.wk pk' hpk', hasmL,
        by rw [← hs']; exact hL.top⟩,
      ⟨hpos, by rw [← hs']; rfl, hsent, hrecvd⟩,
      handsEq_of_other (by rw [← hs']; rfl) (by rw [← hs']; rfl)
        (fun pk'' => by rw [← hs']; rfl)⟩

/-- `asmRecvLevel`: one level return consumed — a receive on
`asmLevelChan pk`, `got` up one; no walk or opener slot moves. -/
theorem step_asmRecvLevel (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.asmRecvLevel pk) s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ RecvStep sk s s' (asmLevelChan pk)
      ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph⟩, hpos⟩ := hg
    injection hstep with hs'
    have hpk2 : 1 ≤ pk.2 := asmKeys_snd_pos hmem
    have hpkmem : pk ∈ sk.asmKeys := List.contains_iff_mem.mp hmem
    have hold := hL.asm pk hpkmem
    simp only [asmLocalOk, hph] at hold
    simp at hold
    have hasmL : ∀ pk' ∈ sk.asmKeys, asmLocalOk sk s' pk' = true := by
      intro pk' hpk'
      by_cases hpkeq : pk' = pk
      · subst hpkeq
        have hasm : s'.asm pk' = { s.asm pk' with
            phase := if (s.asm pk').got + 1
                == sk.pendAt pk'.1 pk'.2 (s.asm pk').idx then 2 else 1,
            got := (s.asm pk').got + 1 } := by rw [← hs']; simp
        by_cases hfull : (s.asm pk').got + 1
            = sk.pendAt pk'.1 pk'.2 (s.asm pk').idx
        · simp [asmLocalOk, hasm, hfull]
          omega
        · simp [asmLocalOk, hasm, hfull]
          omega
      · have ha : s'.asm pk' = s.asm pk' := by
          rw [← hs']; exact setAsm_asm_ne _ _ hpkeq
        rw [asmLocalOk_congr sk pk' ha]; exact hL.asm pk' hpk'
    have hsent : ∀ c ∈ allChans sk, sentOf sk s' c = sentOf sk s c := by
      intro c _hc
      rw [← hs']
      apply sentOf_ext_idx
      · intro pk''
        by_cases hqq : pk'' = pk
        · subst hqq; simp
        · simp [setAsm_asm_ne _ _ hqq]
      all_goals rfl
    have hrecvd : ∀ c ∈ allChans sk,
        recvdOf sk s' c = recvdOf sk s c
          + (if c = asmLevelChan pk then 1 else 0) := by
      intro c _hc
      by_cases hcc : c = asmLevelChan pk
      · subst hcc
        have h21 : pk.2 - 1 + 1 = pk.2 := by omega
        have hkey : ((pk.1, pk.2 - 1 + 1) : Party × Nat) = pk := by
          rw [h21]
        have hrecvS : recvdOf sk s (asmLevelChan pk)
            = sk.pendsBefore pk.1 pk.2 (s.asm pk).idx
              + (s.asm pk).got := by
          show recvdOf sk s (Chan.level pk.1 (pk.2 - 1)) = _
          simp only [recvdOf]
          rw [hkey, if_pos (by exact hmem)]
          rfl
        have hrecvS' : recvdOf sk s' (asmLevelChan pk)
            = sk.pendsBefore pk.1 pk.2 (s.asm pk).idx
              + (s.asm pk).got + 1 := by
          rw [← hs']
          show recvdOf sk (setAsm _ pk _) (Chan.level pk.1 (pk.2 - 1))
              = _
          simp only [recvdOf]
          rw [hkey, if_pos (by exact hmem)]
          simp only [asmLevelRecvd, setAsm_asm_self]
          omega
        rw [hrecvS', hrecvS]
        simp
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          refine Eq.trans
            (recvdOf_setAsm_frame_level _ pk _ hcc ?_) ?_
          · simp only [asmResRecvd, setAsm_asm_self]
            split <;> simp [hph]
          · exact recvdOf_ext sk (fun _ => rfl) (fun _ => rfl)
              (fun _ => rfl) rfl rfl rfl rfl rfl rfl c
        simp [hrecv, hcc]
    exact ⟨⟨fun pk' hpk' => by rw [← hs']; exact hL.wk pk' hpk', hasmL,
        by rw [← hs']; exact hL.top⟩,
      ⟨hpos, by rw [← hs']; rfl, hsent, hrecvd⟩,
      handsEq_of_other (by rw [← hs']; rfl) (by rw [← hs']; rfl)
        (fun pk'' => by rw [← hs']; rfl)⟩

/-- `asmSend`: one assembled resolution published — a send into
`sk.asmOutChan pk`, cursor up one; no walk or opener slot moves. -/
theorem step_asmSend (_hwf : sk.wellFormed = true) (pk : Party × Nat)
    (hstep : Model.apply sk ax (.asmSend pk) s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ SendStep sk s s' (sk.asmOutChan pk)
      ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph⟩, hcaplt⟩ := hg
    injection hstep with hs'
    have hpk2 : 1 ≤ pk.2 := asmKeys_snd_pos hmem
    have hpkmem : pk ∈ sk.asmKeys := List.contains_iff_mem.mp hmem
    have hold := hL.asm pk hpkmem
    simp only [asmLocalOk, hph] at hold
    simp at hold
    have hasmL : ∀ pk' ∈ sk.asmKeys, asmLocalOk sk s' pk' = true := by
      intro pk' hpk'
      by_cases hpkeq : pk' = pk
      · subst hpkeq
        have hasm : s'.asm pk' =
            { idx := (s.asm pk').idx + 1,
              phase := if (s.asm pk').idx + 1
                  < (sk.asmResList pk'.1 pk'.2).length then 0 else 3,
              got := 0 } := by rw [← hs']; simp
        by_cases hlt : (s.asm pk').idx + 1
            < (sk.asmResList pk'.1 pk'.2).length
        · simp [asmLocalOk, hasm, hlt]
        · simp [asmLocalOk, hasm, hlt]
          omega
      · have ha : s'.asm pk' = s.asm pk' := by
          rw [← hs']; exact setAsm_asm_ne _ _ hpkeq
        rw [asmLocalOk_congr sk pk' ha]; exact hL.asm pk' hpk'
    have hrecvd : ∀ c ∈ allChans sk,
        recvdOf sk s' c = recvdOf sk s c := by
      intro c _hc
      rw [← hs']
      refine Eq.trans (recvdOf_setAsm_of_counts sk _ pk _ ?_ ?_ c) ?_
      · simp only [asmResRecvd, setAsm_asm_self]
        by_cases hlt2 : (s.asm pk).idx + 1
            < (sk.asmResList pk.1 pk.2).length
        · simp [hlt2, hph]
        · simp [hlt2, hph]
      · simp only [asmLevelRecvd, setAsm_asm_self]
        rw [pendsBefore_succ sk pk.1 pk.2 (s.asm pk).idx hold.1]
        omega
      · exact recvdOf_ext sk (fun _ => rfl) (fun _ => rfl)
          (fun _ => rfl) rfl rfl rfl rfl rfl rfl c
    have hsentd : ∀ c ∈ allChans sk,
        sentOf sk s' c = sentOf sk s c
          + (if c = sk.asmOutChan pk then 1 else 0) := by
      intro c _hc
      by_cases hcc : c = sk.asmOutChan pk
      · subst hcc
        by_cases h1 : (pk.1 == Party.I && pk.2 == sk.rootH) = true
        · have hpkI : pk = (Party.I, sk.rootH) := by
            simp only [Bool.and_eq_true, beq_iff_eq] at h1
            exact Prod.ext h1.1 h1.2
          have hch : sk.asmOutChan pk = Chan.rootret := by
            unfold Skel.asmOutChan
            rw [if_pos h1]
          have hsentS : sentOf sk s (sk.asmOutChan pk)
              = (s.asm pk).idx := by
            rw [hch]
            show asmOutSent s (Party.I, sk.rootH) = _
            rw [← hpkI]
            rfl
          have hsentS' : sentOf sk s' (sk.asmOutChan pk)
              = (s.asm pk).idx + 1 := by
            rw [← hs', hch]
            show asmOutSent (setAsm _ pk _) (Party.I, sk.rootH) = _
            rw [← hpkI]
            simp [asmOutSent]
          rw [hsentS', hsentS]
          simp
        · by_cases h2 : (pk.1 == Party.R
              && pk.2 == sk.rootH - 1) = true
          · have hpkR : pk = (Party.R, sk.rootH - 1) := by
              simp only [Bool.and_eq_true, beq_iff_eq] at h2
              exact Prod.ext h2.1 h2.2
            have hch : sk.asmOutChan pk = Chan.rootrets := by
              unfold Skel.asmOutChan
              rw [if_neg h1, if_pos h2]
            have hsentS : sentOf sk s (sk.asmOutChan pk)
                = (s.asm pk).idx := by
              rw [hch]
              show asmOutSent s (Party.R, sk.rootH - 1) = _
              rw [← hpkR]
              rfl
            have hsentS' : sentOf sk s' (sk.asmOutChan pk)
                = (s.asm pk).idx + 1 := by
              rw [← hs', hch]
              show asmOutSent (setAsm _ pk _) (Party.R, sk.rootH - 1)
                  = _
              rw [← hpkR]
              simp [asmOutSent]
            rw [hsentS', hsentS]
            simp
          · have hch : sk.asmOutChan pk = Chan.level pk.1 pk.2 := by
              unfold Skel.asmOutChan
              rw [if_neg h1, if_neg h2]
            have hroot : isRootOutKey sk pk = false := by
              rw [Bool.eq_false_iff]
              intro hr
              simp only [isRootOutKey, Bool.or_eq_true] at hr
              rcases hr with hr | hr
              · exact h1 hr
              · exact h2 hr
            have hnot0 : ¬((pk.1 == Party.I && pk.2 == (0 : Nat))
                = true) := by
              simp only [Bool.and_eq_true, beq_iff_eq]
              rintro ⟨-, h0⟩
              omega
            have hcond : (sk.asmKeys.contains (pk.1, pk.2)
                && !isRootOutKey sk (pk.1, pk.2)) = true := by
              rw [show ((pk.1, pk.2) : Party × Nat) = pk from rfl,
                hmem, hroot]
              rfl
            have hsentS : sentOf sk s (sk.asmOutChan pk)
                = (s.asm pk).idx := by
              rw [hch]
              show sentOf sk s (Chan.level pk.1 pk.2) = _
              simp only [sentOf]
              rw [if_neg hnot0, if_pos hcond]
              rfl
            have hsentS' : sentOf sk s' (sk.asmOutChan pk)
                = (s.asm pk).idx + 1 := by
              rw [← hs', hch]
              show sentOf sk (setAsm _ pk _) (Chan.level pk.1 pk.2)
                  = _
              simp only [sentOf]
              rw [if_neg hnot0, if_pos hcond]
              show asmOutSent (setAsm _ pk _) (pk.1, pk.2) = _
              rw [show ((pk.1, pk.2) : Party × Nat) = pk from rfl]
              simp [asmOutSent]
            rw [hsentS', hsentS]
            simp
      · have hsent : sentOf sk s' c = sentOf sk s c := by
          rw [← hs']
          refine Eq.trans (sentOf_setAsm_frame _ pk _ hcc) ?_
          exact sentOf_ext sk (fun _ => rfl) (fun _ => rfl)
            (fun _ => rfl) (fun _ => rfl) (fun _ => rfl)
            (fun _ => rfl) (fun _ => rfl) rfl rfl rfl rfl rfl rfl c
        simp [hsent, hcc]
    exact ⟨⟨fun pk' hpk' => by rw [← hs']; exact hL.wk pk' hpk', hasmL,
        by rw [← hs']; exact hL.top⟩,
      ⟨hcaplt, by rw [← hs']; rfl, hsentd, hrecvd⟩,
      handsEq_of_other (by rw [← hs']; rfl) (by rw [← hs']; rfl)
        (fun pk'' => by rw [← hs']; rfl)⟩

/-- `asmClose`: the end-of-stream observation, phase 3 → 4 — no
channel operation, no walk or opener slot moves. -/
theorem step_asmClose (_hwf : sk.wellFormed = true) (pk : Party × Nat)
    (hstep : Model.apply sk ax (.asmClose pk) s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ QuietStep sk s s' ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨⟨hmem, hph⟩, _hpd⟩, _hch0⟩ := hg
    injection hstep with hs'
    have hasmL : ∀ pk' ∈ sk.asmKeys, asmLocalOk sk s' pk' = true := by
      intro pk' hpk'
      by_cases hpkeq : pk' = pk
      · subst hpkeq
        have hasm : s'.asm pk' = { s.asm pk' with phase := 4 } := by
          rw [← hs']; simp
        have hold := hL.asm pk' hpk'
        simp only [asmLocalOk, hasm, hph] at hold ⊢
        simp at hold ⊢
        omega
      · have ha : s'.asm pk' = s.asm pk' := by
          rw [← hs']; exact setAsm_asm_ne s _ hpkeq
        rw [asmLocalOk_congr sk pk' ha]; exact hL.asm pk' hpk'
    have hquiet : QuietStep sk s s' := by
      refine ⟨by rw [← hs']; rfl, fun c _ => ?_, fun c _ => ?_⟩
      · rw [← hs']
        apply sentOf_ext_idx
        · intro pk''
          by_cases hqq : pk'' = pk
          · subst hqq; simp
          · simp [setAsm_asm_ne s _ hqq]
        all_goals rfl
      · rw [← hs']
        apply recvdOf_setAsm_of_counts
        · simp [asmResRecvd, hph]
        · simp [asmLevelRecvd]
    exact ⟨⟨fun pk' hpk' => by rw [← hs']; exact hL.wk pk' hpk', hasmL,
        by rw [← hs']; exact hL.top⟩, hquiet,
      handsEq_of_other (by rw [← hs']; rfl) (by rw [← hs']; rfl)
        (fun pk'' => by rw [← hs']; rfl)⟩

end StreamingMirror.Mux
