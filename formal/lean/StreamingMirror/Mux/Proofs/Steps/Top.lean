/-
Per-arm step facts for the top components (openers, absorber,
finishes): the InvL bullet and count deltas of each base action,
extracted from the Preserve monoliths' local parts (Steps.lean's module
doc explains why the monoliths themselves cannot be invoked at muxed
states).
-/
import StreamingMirror.Mux.Proofs.Steps

namespace StreamingMirror.Mux

open Model

variable {sk : Skel} {ax : AxMode} {s s' : State}

-- ============================================================= helpers

/-- Hands framing for an opener arm: the two root comparisons are
supplied pointwise, and no walk moves. -/
private theorem handsEq_root_frame
    (hi : (s'.iopenCh == some IOblig.wire)
        = (s.iopenCh == some IOblig.wire))
    (hr : (s'.ropenCh == some ROblig.wire)
        = (s.ropenCh == some ROblig.wire))
    (hwk : s'.walk = s.walk) :
    HandsEq sk s s' := by
  intro p h
  by_cases hh : h = sk.rootH
  · subst hh
    rw [holdsWire.eq_def, holdsWire.eq_def]
    simp only [beq_self_eq_true, if_pos]
    cases p
    · exact hi
    · exact hr
  · rw [holdsWire_eq_wireHand hh, holdsWire_eq_wireHand hh, hwk]

/-- Hands framing off the initiator's root entry: every stream other
than `(I, rootH)` reads the same hand. -/
private theorem holdsWire_frame_I
    (hro : (s'.ropenCh == some ROblig.wire)
        = (s.ropenCh == some ROblig.wire))
    (hwk : s'.walk = s.walk) :
    ∀ p h, ¬(p = Party.I ∧ h = sk.rootH) →
      holdsWire sk p h s' = holdsWire sk p h s := by
  intro p h hph
  by_cases hh : h = sk.rootH
  · subst hh
    rw [holdsWire.eq_def, holdsWire.eq_def]
    simp only [beq_self_eq_true, if_pos]
    cases p
    · exact absurd ⟨rfl, rfl⟩ hph
    · exact hro
  · rw [holdsWire_eq_wireHand hh, holdsWire_eq_wireHand hh, hwk]

/-- Hands framing off the responder's root entry: every stream other
than `(R, rootH)` reads the same hand. -/
private theorem holdsWire_frame_R
    (hio : (s'.iopenCh == some IOblig.wire)
        = (s.iopenCh == some IOblig.wire))
    (hwk : s'.walk = s.walk) :
    ∀ p h, ¬(p = Party.R ∧ h = sk.rootH) →
      holdsWire sk p h s' = holdsWire sk p h s := by
  intro p h hph
  by_cases hh : h = sk.rootH
  · subst hh
    rw [holdsWire.eq_def, holdsWire.eq_def]
    simp only [beq_self_eq_true, if_pos]
    cases p
    · exact hio
    · exact absurd ⟨rfl, rfl⟩ hph
  · rw [holdsWire_eq_wireHand hh, holdsWire_eq_wireHand hh, hwk]

-- ============================================================= openers

/-- `iopenChoose .wire` commits the initiator opener to its wire
obligation: no channel operation, and the initiator's root wire hand
turns on at exactly that stream. -/
theorem step_iopenChoose_wire
    (hstep : Model.apply sk ax (.iopenChoose .wire) s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ QuietStep sk s s'
      ∧ holdsWire sk .I sk.rootH s = false
      ∧ holdsWire sk .I sk.rootH s' = true
      ∧ ∀ p h, ¬(p = Party.I ∧ h = sk.rootH) →
          holdsWire sk p h s' = holdsWire sk p h s := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨hnone, hch⟩ := hg
    injection hstep with hs'
    have hCh : s'.iopenCh = some IOblig.wire := by rw [← hs']
    refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ⟨?_, ?_, ?_⟩,
      ?_, ?_, ?_⟩
    · rw [← hs']; exact hL.wk pk hpk
    · rw [← hs']; exact hL.asm pk hpk
    · have htop := hL.top
      rw [topLocalOk] at htop ⊢
      rw [hCh,
        show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.ropenCh = s.ropenCh from by rw [← hs'],
        show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
        show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
        show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
        show s'.rfinGot = s.rfinGot from by rw [← hs']]
      simp_all [iopenChoosable]
    · rw [← hs']
    · intro c _; rw [← hs']; cases c <;> rfl
    · intro c _; rw [← hs']; cases c <;> rfl
    · rw [holdsWire.eq_def]
      simp only [beq_self_eq_true, if_pos]
      show (s.iopenCh == some IOblig.wire) = false
      simp only [hnone]
      decide
    · rw [holdsWire.eq_def]
      simp only [beq_self_eq_true, if_pos]
      show (s'.iopenCh == some IOblig.wire) = true
      simp only [hCh]
      decide
    · have hro : s'.ropenCh = s.ropenCh := by rw [← hs']
      have hwk : s'.walk = s.walk := by rw [← hs']
      exact holdsWire_frame_I (by rw [hro]) hwk

/-- `iopenChoose .query` commits the initiator opener to its query
obligation: no channel operation, and no wire hand moves. -/
theorem step_iopenChoose_query
    (hstep : Model.apply sk ax (.iopenChoose .query) s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ QuietStep sk s s' ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨hnone, hch⟩ := hg
    injection hstep with hs'
    have hCh : s'.iopenCh = some IOblig.query := by rw [← hs']
    refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ⟨?_, ?_, ?_⟩, ?_⟩
    · rw [← hs']; exact hL.wk pk hpk
    · rw [← hs']; exact hL.asm pk hpk
    · have htop := hL.top
      rw [topLocalOk] at htop ⊢
      rw [hCh,
        show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.ropenCh = s.ropenCh from by rw [← hs'],
        show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
        show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
        show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
        show s'.rfinGot = s.rfinGot from by rw [← hs']]
      simp_all [iopenChoosable]
    · rw [← hs']
    · intro c _; rw [← hs']; cases c <;> rfl
    · intro c _; rw [← hs']; cases c <;> rfl
    · have hro : s'.ropenCh = s.ropenCh := by rw [← hs']
      have hwk : s'.walk = s.walk := by rw [← hs']
      exact handsEq_root_frame (by simp only [hCh, hnone]; decide)
        (by rw [hro]) hwk

/-- `iopenFire` on a committed query publishes the initiator's opening
query: one send into `asked I (rootH - 1)`, and no wire hand moves. -/
theorem step_iopenFire_query (hch : s.iopenCh = some .query)
    (hstep : Model.apply sk ax .iopenFire s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ SendStep sk s s' (Chan.asked Party.I (sk.rootH - 1))
      ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  next hcw => rw [hch] at hcw; simp at hcw
  next hcq =>
    split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      injection hstep with hs'
      have hiq : s.iopenQuery = false := by
        have htop := hL.top
        rw [topLocalOk] at htop
        simp only [Bool.and_eq_true] at htop
        have h2 := htop.1.1.1.1.1.1.1.1.1.1.2
        rw [hcq] at h2
        simp at h2
        exact h2.1
      refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ⟨?_, ?_, ?_, ?_⟩,
        ?_⟩
      · rw [← hs']; exact hL.wk pk hpk
      · rw [← hs']; exact hL.asm pk hpk
      · have htop := hL.top
        rw [topLocalOk] at htop ⊢
        rw [show s'.iopenCh = none from by rw [← hs'],
          show s'.iopenWire = s.iopenWire from by rw [← hs'],
          show s'.iopenQuery = true from by rw [← hs'],
          show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
          show s'.ropenWire = s.ropenWire from by rw [← hs'],
          show s'.ropenRes = s.ropenRes from by rw [← hs'],
          show s'.ropenQ = s.ropenQ from by rw [← hs'],
          show s'.ropenCh = s.ropenCh from by rw [← hs'],
          show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
          show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
          show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
          show s'.rfinGot = s.rfinGot from by rw [← hs']]
        rw [hcq] at htop
        simp_all
      · exact hg
      · rw [← hs']
      · intro c _
        by_cases hc0 : c = Chan.asked Party.I (sk.rootH - 1)
        · subst hc0
          have hsent1 :
              sentOf sk s' (Chan.asked Party.I (sk.rootH - 1)) = 1 := by
            rw [← hs']; simp [sentOf, b2n]
          have hsent0 :
              sentOf sk s (Chan.asked Party.I (sk.rootH - 1)) = 0 := by
            simp [sentOf, hiq, b2n]
          simp [hsent1, hsent0]
        · have hsent : sentOf sk s' c = sentOf sk s c := by
            rw [← hs']
            cases c with
            | asked p h =>
                by_cases h1 : (p == Party.I && h == sk.rootH - 1) = true
                · simp only [Bool.and_eq_true, beq_iff_eq] at h1
                  exact absurd (by rw [h1.1, h1.2]) hc0
                · simp [sentOf, wkQSentTot, wkQSum, h1]
            | wire p h => rfl
            | leafRequests => rfl
            | upper p h => rfl
            | lower p h => rfl
            | level p j => rfl
            | rootret => rfl
            | rootrets => rfl
            | rootres => rfl
          rw [hsent, if_neg hc0]; rfl
      · intro c _; rw [← hs']; cases c <;> rfl
      · have hCh : s'.iopenCh = none := by rw [← hs']
        have hro : s'.ropenCh = s.ropenCh := by rw [← hs']
        have hwk : s'.walk = s.walk := by rw [← hs']
        exact handsEq_root_frame (by simp only [hCh, hcq]; decide)
          (by rw [hro]) hwk
  next => simp at hstep

/-- `ropenRecv` consumes the initiator's opening wire message: one
receive on the initiator's root wire channel, and no wire hand moves. -/
theorem step_ropenRecv
    (hstep : Model.apply sk ax .ropenRecv s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ RecvStep sk s s' (Chan.wire Party.I sk.rootH)
      ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, Bool.not_eq_true', decide_eq_true_eq] at hg
    obtain ⟨hgot, hpos⟩ := hg
    injection hstep with hs'
    refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ⟨?_, ?_, ?_, ?_⟩, ?_⟩
    · rw [← hs']; exact hL.wk pk hpk
    · rw [← hs']; exact hL.asm pk hpk
    · have htop := hL.top
      rw [topLocalOk] at htop ⊢
      rw [show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.iopenCh = s.iopenCh from by rw [← hs'],
        show s'.ropenGotWire = true from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.ropenCh = s.ropenCh from by rw [← hs'],
        show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
        show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
        show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
        show s'.rfinGot = s.rfinGot from by rw [← hs']]
      simp_all
    · exact hpos
    · rw [← hs']
    · intro c _; rw [← hs']; cases c <;> rfl
    · intro c _
      by_cases hc0 : c = Chan.wire Party.I sk.rootH
      · subst hc0
        have hrecv1 : recvdOf sk s' (Chan.wire Party.I sk.rootH) = 1 := by
          rw [← hs']; simp [recvdOf, b2n]
        have hrecv0 : recvdOf sk s (Chan.wire Party.I sk.rootH) = 0 := by
          simp [recvdOf, hgot, b2n]
        simp [hrecv1, hrecv0]
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          cases c with
          | wire p h =>
              by_cases hh : (h == sk.rootH) = true
              · by_cases hp : (p == Party.I) = true
                · rw [beq_iff_eq] at hh hp
                  exact absurd (by rw [hh, hp]) hc0
                · simp [recvdOf, wkWireRecvd, hh, hp]
              · simp [recvdOf, wkWireRecvd, absorbWireRecvd, hh]
          | asked p h => rfl
          | leafRequests => rfl
          | upper p h => rfl
          | lower p h => rfl
          | level p j => rfl
          | rootret => rfl
          | rootrets => rfl
          | rootres => rfl
        rw [hrecv, if_neg hc0]; rfl
    · have hio : s'.iopenCh = s.iopenCh := by rw [← hs']
      have hro : s'.ropenCh = s.ropenCh := by rw [← hs']
      have hwk : ∀ pk, s'.walk pk = s.walk pk := fun pk => by rw [← hs']
      exact handsEq_of_other hio hro hwk

/-- `ropenChoose .wire` commits the responder opener to its wire
obligation: no channel operation, and the responder's root wire hand
turns on at exactly that stream. -/
theorem step_ropenChoose_wire
    (hstep : Model.apply sk ax (.ropenChoose .wire) s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ QuietStep sk s s'
      ∧ holdsWire sk .R sk.rootH s = false
      ∧ holdsWire sk .R sk.rootH s' = true
      ∧ ∀ p h, ¬(p = Party.R ∧ h = sk.rootH) →
          holdsWire sk p h s' = holdsWire sk p h s := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨hnone, hch⟩ := hg
    injection hstep with hs'
    have hCh : s'.ropenCh = some ROblig.wire := by rw [← hs']
    refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ⟨?_, ?_, ?_⟩,
      ?_, ?_, ?_⟩
    · rw [← hs']; exact hL.wk pk hpk
    · rw [← hs']; exact hL.asm pk hpk
    · have htop := hL.top
      rw [topLocalOk] at htop ⊢
      rw [hCh,
        show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.iopenCh = s.iopenCh from by rw [← hs'],
        show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
        show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
        show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
        show s'.rfinGot = s.rfinGot from by rw [← hs']]
      simp_all [ropenChoosable, Skel.rootPending]
    · rw [← hs']
    · intro c _; rw [← hs']; cases c <;> rfl
    · intro c _; rw [← hs']; cases c <;> rfl
    · rw [holdsWire.eq_def]
      simp only [beq_self_eq_true, if_pos]
      show (s.ropenCh == some ROblig.wire) = false
      simp only [hnone]
      decide
    · rw [holdsWire.eq_def]
      simp only [beq_self_eq_true, if_pos]
      show (s'.ropenCh == some ROblig.wire) = true
      simp only [hCh]
      decide
    · have hio : s'.iopenCh = s.iopenCh := by rw [← hs']
      have hwk : s'.walk = s.walk := by rw [← hs']
      exact holdsWire_frame_R (by rw [hio]) hwk

/-- `ropenChoose .res` commits the responder opener to its resolution
obligation: no channel operation, and no wire hand moves. -/
theorem step_ropenChoose_res
    (hstep : Model.apply sk ax (.ropenChoose .res) s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ QuietStep sk s s' ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨hnone, hch⟩ := hg
    injection hstep with hs'
    have hCh : s'.ropenCh = some ROblig.res := by rw [← hs']
    refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ⟨?_, ?_, ?_⟩, ?_⟩
    · rw [← hs']; exact hL.wk pk hpk
    · rw [← hs']; exact hL.asm pk hpk
    · have htop := hL.top
      rw [topLocalOk] at htop ⊢
      rw [hCh,
        show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.iopenCh = s.iopenCh from by rw [← hs'],
        show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
        show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
        show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
        show s'.rfinGot = s.rfinGot from by rw [← hs']]
      simp_all [ropenChoosable, Skel.rootPending]
    · rw [← hs']
    · intro c _; rw [← hs']; cases c <;> rfl
    · intro c _; rw [← hs']; cases c <;> rfl
    · have hio : s'.iopenCh = s.iopenCh := by rw [← hs']
      have hwk : s'.walk = s.walk := by rw [← hs']
      exact handsEq_root_frame (by rw [hio])
        (by simp only [hCh, hnone]; decide) hwk

/-- `ropenChoose .query` commits the responder opener to its query
obligation: no channel operation, and no wire hand moves. -/
theorem step_ropenChoose_query
    (hstep : Model.apply sk ax (.ropenChoose .query) s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ QuietStep sk s s' ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨hnone, hch⟩ := hg
    injection hstep with hs'
    have hCh : s'.ropenCh = some ROblig.query := by rw [← hs']
    refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ⟨?_, ?_, ?_⟩, ?_⟩
    · rw [← hs']; exact hL.wk pk hpk
    · rw [← hs']; exact hL.asm pk hpk
    · have htop := hL.top
      rw [topLocalOk] at htop ⊢
      rw [hCh,
        show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.iopenCh = s.iopenCh from by rw [← hs'],
        show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
        show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
        show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
        show s'.rfinGot = s.rfinGot from by rw [← hs']]
      simp_all [ropenChoosable, Skel.rootPending]
    · rw [← hs']
    · intro c _; rw [← hs']; cases c <;> rfl
    · intro c _; rw [← hs']; cases c <;> rfl
    · have hio : s'.iopenCh = s.iopenCh := by rw [← hs']
      have hwk : s'.walk = s.walk := by rw [← hs']
      exact handsEq_root_frame (by rw [hio])
        (by simp only [hCh, hnone]; decide) hwk

/-- `ropenFire` on a committed resolution publishes the responder's root
resolution: one send into `rootres`, and no wire hand moves. -/
theorem step_ropenFire_res (hch : s.ropenCh = some .res)
    (hstep : Model.apply sk ax .ropenFire s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ SendStep sk s s' Chan.rootres ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  next hcw => rw [hch] at hcw; simp at hcw
  next hcr =>
    split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      injection hstep with hs'
      have hrr : s.ropenRes = false := by
        have htop := hL.top
        rw [topLocalOk] at htop
        simp only [Bool.and_eq_true] at htop
        have h7 := htop.1.1.1.1.1.2
        rw [hcr] at h7
        simp at h7
        exact h7.1
      refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ⟨?_, ?_, ?_, ?_⟩,
        ?_⟩
      · rw [← hs']; exact hL.wk pk hpk
      · rw [← hs']; exact hL.asm pk hpk
      · have htop := hL.top
        rw [topLocalOk] at htop ⊢
        rw [show s'.iopenWire = s.iopenWire from by rw [← hs'],
          show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
          show s'.iopenCh = s.iopenCh from by rw [← hs'],
          show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
          show s'.ropenWire = s.ropenWire from by rw [← hs'],
          show s'.ropenRes = true from by rw [← hs'],
          show s'.ropenQ = s.ropenQ from by rw [← hs'],
          show s'.ropenCh = none from by rw [← hs'],
          show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
          show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
          show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
          show s'.rfinGot = s.rfinGot from by rw [← hs']]
        rw [hcr] at htop
        simp_all
      · exact hg
      · rw [← hs']
      · intro c _
        by_cases hc0 : c = Chan.rootres
        · subst hc0
          have hsent1 : sentOf sk s' Chan.rootres = 1 := by
            rw [← hs']; simp [sentOf, b2n]
          have hsent0 : sentOf sk s Chan.rootres = 0 := by
            simp [sentOf, hrr, b2n]
          simp [hsent1, hsent0]
        · have hsent : sentOf sk s' c = sentOf sk s c := by
            rw [← hs']
            cases c <;> first | rfl | exact absurd rfl hc0
          rw [hsent, if_neg hc0]; rfl
      · intro c _; rw [← hs']; cases c <;> rfl
      · have hio : s'.iopenCh = s.iopenCh := by rw [← hs']
        have hCh : s'.ropenCh = none := by rw [← hs']
        have hwk : s'.walk = s.walk := by rw [← hs']
        exact handsEq_root_frame (by rw [hio])
          (by simp only [hCh, hcr]; decide) hwk
  next hcq => rw [hch] at hcq; simp at hcq
  next => simp at hstep

/-- `ropenFire` on a committed query publishes one responder opening
query: one send into `asked R (rootH - 2)`, and no wire hand moves. -/
theorem step_ropenFire_query (hch : s.ropenCh = some .query)
    (hstep : Model.apply sk ax .ropenFire s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ SendStep sk s s' (Chan.asked Party.R (sk.rootH - 2))
      ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  next hcw => rw [hch] at hcw; simp at hcw
  next hcr => rw [hch] at hcr; simp at hcr
  next hcq =>
    split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      injection hstep with hs'
      have hqlt : s.ropenQ < sk.rootPending := by
        have htop := hL.top
        rw [topLocalOk] at htop
        simp only [Bool.and_eq_true] at htop
        have h8 := htop.1.1.1.1.2
        rw [hcq] at h8
        simp at h8
        exact h8.1.1
      refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ⟨?_, ?_, ?_, ?_⟩,
        ?_⟩
      · rw [← hs']; exact hL.wk pk hpk
      · rw [← hs']; exact hL.asm pk hpk
      · have htop := hL.top
        rw [topLocalOk] at htop ⊢
        rw [show s'.iopenWire = s.iopenWire from by rw [← hs'],
          show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
          show s'.iopenCh = s.iopenCh from by rw [← hs'],
          show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
          show s'.ropenWire = s.ropenWire from by rw [← hs'],
          show s'.ropenRes = s.ropenRes from by rw [← hs'],
          show s'.ropenQ = s.ropenQ + 1 from by rw [← hs'],
          show s'.ropenCh = none from by rw [← hs'],
          show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
          show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
          show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
          show s'.rfinGot = s.rfinGot from by rw [← hs']]
        rw [hcq] at htop
        simp_all
        omega
      · exact hg
      · rw [← hs']
      · intro c _
        by_cases hc0 : c = Chan.asked Party.R (sk.rootH - 2)
        · subst hc0
          have hsent1 :
              sentOf sk s' (Chan.asked Party.R (sk.rootH - 2))
                = s.ropenQ + 1 := by
            rw [← hs']; simp [sentOf]
          have hsent0 :
              sentOf sk s (Chan.asked Party.R (sk.rootH - 2))
                = s.ropenQ := by
            simp [sentOf]
          simp [hsent1, hsent0]
        · have hsent : sentOf sk s' c = sentOf sk s c := by
            rw [← hs']
            cases c with
            | asked p h =>
                by_cases h2 : (p == Party.R && h == sk.rootH - 2) = true
                · simp only [Bool.and_eq_true, beq_iff_eq] at h2
                  exact absurd (by rw [h2.1, h2.2]) hc0
                · simp [sentOf, wkQSentTot, wkQSum, h2]
            | wire p h => rfl
            | leafRequests => rfl
            | upper p h => rfl
            | lower p h => rfl
            | level p j => rfl
            | rootret => rfl
            | rootrets => rfl
            | rootres => rfl
          rw [hsent, if_neg hc0]; rfl
      · intro c _; rw [← hs']; cases c <;> rfl
      · have hio : s'.iopenCh = s.iopenCh := by rw [← hs']
        have hCh : s'.ropenCh = none := by rw [← hs']
        have hwk : s'.walk = s.walk := by rw [← hs']
        exact handsEq_root_frame (by rw [hio])
          (by simp only [hCh, hcq]; decide) hwk
  next => simp at hstep

-- ============================================================ absorber

/-- `absorbRecvWire` consumes one leaf wire message (phase 0→1): one
receive on the absorber's wire input, and no wire hand moves.

Takes `wellFormed`, unlike its siblings: `wf_rootH` keeps `wire R 0`
out of `recvdOf`'s root-wire branch, exactly as in the monolith. -/
theorem step_absorbRecvWire (hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .absorbRecvWire s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ RecvStep sk s s' (Chan.wire Party.R 0)
      ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨hph, hpos⟩ := hg
    have hrH : (0 == sk.rootH) = false := by
      have h2 := (wf_rootH hwf).2
      have : (0 : Nat) ≠ sk.rootH := by omega
      simp [this]
    injection hstep with hs'
    refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ⟨?_, ?_, ?_, ?_⟩, ?_⟩
    · rw [← hs']; exact hL.wk pk hpk
    · rw [← hs']; exact hL.asm pk hpk
    · have htop := hL.top
      rw [topLocalOk] at htop ⊢
      rw [show s'.absorbPhase = 1 from by rw [← hs'],
        show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.iopenCh = s.iopenCh from by rw [← hs'],
        show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.ropenCh = s.ropenCh from by rw [← hs'],
        show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
        show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
        show s'.rfinGot = s.rfinGot from by rw [← hs']]
      simp only [Bool.and_eq_true] at htop ⊢
      obtain ⟨⟨⟨⟨hpre, h9⟩, _h10⟩, h11⟩, h12⟩ := htop
      refine ⟨⟨⟨⟨hpre, ?_⟩, rfl⟩, h11⟩, h12⟩
      rw [hph] at h9
      simpa using h9
    · exact hpos
    · rw [← hs']
    · intro c _; rw [← hs']; cases c <;> rfl
    · intro c _
      by_cases hc0 : c = Chan.wire Party.R 0
      · subst hc0
        have hrecv1 : recvdOf sk s' (Chan.wire Party.R 0)
            = s.absorbIdx + 1 := by
          rw [← hs']; simp [recvdOf, absorbWireRecvd, hrH]
        have hrecv0 : recvdOf sk s (Chan.wire Party.R 0)
            = s.absorbIdx := by
          simp [recvdOf, absorbWireRecvd, hrH, hph]
        simp [hrecv1, hrecv0]
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          cases c with
          | wire p h =>
              cases p with
              | I => rfl
              | R =>
                  have hh : h ≠ 0 := fun h0 => hc0 (by rw [h0])
                  have h0 : (h == 0) = false := by simp [hh]
                  simp [recvdOf, wkWireRecvd, h0]
          | leafRequests => simp [recvdOf, absorbAskedRecvd, hph]
          | _ => rfl
        rw [hrecv, if_neg hc0]; rfl
    · have hio : s'.iopenCh = s.iopenCh := by rw [← hs']
      have hro : s'.ropenCh = s.ropenCh := by rw [← hs']
      have hwk : ∀ pk, s'.walk pk = s.walk pk := fun pk => by rw [← hs']
      exact handsEq_of_other hio hro hwk

/-- `absorbRecvAsked` consumes one leaf request (phase 1→2): one receive
on `leafRequests`, and no wire hand moves. -/
theorem step_absorbRecvAsked
    (hstep : Model.apply sk ax .absorbRecvAsked s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ RecvStep sk s s' Chan.leafRequests
      ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨hph, hpos⟩ := hg
    injection hstep with hs'
    refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ⟨?_, ?_, ?_, ?_⟩, ?_⟩
    · rw [← hs']; exact hL.wk pk hpk
    · rw [← hs']; exact hL.asm pk hpk
    · have htop := hL.top
      rw [topLocalOk] at htop ⊢
      rw [show s'.absorbPhase = 2 from by rw [← hs'],
        show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.iopenCh = s.iopenCh from by rw [← hs'],
        show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.ropenCh = s.ropenCh from by rw [← hs'],
        show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
        show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
        show s'.rfinGot = s.rfinGot from by rw [← hs']]
      simp only [Bool.and_eq_true] at htop ⊢
      obtain ⟨⟨⟨⟨hpre, h9⟩, _h10⟩, h11⟩, h12⟩ := htop
      refine ⟨⟨⟨⟨hpre, ?_⟩, rfl⟩, h11⟩, h12⟩
      rw [hph] at h9
      simpa using h9
    · exact hpos
    · rw [← hs']
    · intro c _; rw [← hs']; cases c <;> rfl
    · intro c _
      by_cases hc0 : c = Chan.leafRequests
      · subst hc0
        have hrecv1 : recvdOf sk s' Chan.leafRequests
            = s.absorbIdx + 1 := by
          rw [← hs']; simp [recvdOf, absorbAskedRecvd]
        have hrecv0 : recvdOf sk s Chan.leafRequests = s.absorbIdx := by
          simp [recvdOf, absorbAskedRecvd, hph]
        simp [hrecv1, hrecv0]
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          cases c with
          | wire p h => simp [recvdOf, wkWireRecvd, absorbWireRecvd, hph]
          | leafRequests => exact absurd rfl hc0
          | _ => rfl
        rw [hrecv, if_neg hc0]; rfl
    · have hio : s'.iopenCh = s.iopenCh := by rw [← hs']
      have hro : s'.ropenCh = s.ropenCh := by rw [← hs']
      have hwk : ∀ pk, s'.walk pk = s.walk pk := fun pk => by rw [← hs']
      exact handsEq_of_other hio hro hwk

/-- `absorbSend` publishes one absorbed pair (phase 2→0 or 2→3): one
send into `level I 0`, and no wire hand moves. -/
theorem step_absorbSend
    (hstep : Model.apply sk ax .absorbSend s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ SendStep sk s s' (Chan.level Party.I 0)
      ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨hph, hlt⟩ := hg
    have hidx : s.absorbIdx < sk.totalLeafReqs := by
      have htop := hL.top
      rw [topLocalOk] at htop
      simp only [Bool.and_eq_true] at htop
      have h9 := htop.1.1.1.2
      rw [hph] at h9
      simpa using h9
    injection hstep with hs'
    -- the absorb consumer counts are constant across the send
    have habsW : absorbWireRecvd sk s' = absorbWireRecvd sk s := by
      rw [← hs']
      by_cases hlt2 : s.absorbIdx + 1 < sk.totalLeafReqs
      · simp [absorbWireRecvd, hph, hlt2]
      · simp [absorbWireRecvd, hph, hlt2]
        omega
    have habsA : absorbAskedRecvd sk s' = absorbAskedRecvd sk s := by
      rw [← hs']
      by_cases hlt2 : s.absorbIdx + 1 < sk.totalLeafReqs
      · simp [absorbAskedRecvd, hph, hlt2]
      · simp [absorbAskedRecvd, hph, hlt2]
        omega
    -- untouched components, at observation granularity
    have hwalk : s'.walk = s.walk := by rw [← hs']
    have hasm : s'.asm = s.asm := by rw [← hs']
    have hgw : s'.ropenGotWire = s.ropenGotWire := by rw [← hs']
    have hifin : s'.ifin = s.ifin := by rw [← hs']
    have hrg : s'.rfinGot = s.rfinGot := by rw [← hs']
    have hrgr : s'.rfinGotRes = s.rfinGotRes := by rw [← hs']
    have hio1 : s'.iopenWire = s.iopenWire := by rw [← hs']
    have hio2 : s'.iopenQuery = s.iopenQuery := by rw [← hs']
    have hro1 : s'.ropenWire = s.ropenWire := by rw [← hs']
    have hro2 : s'.ropenRes = s.ropenRes := by rw [← hs']
    have hro3 : s'.ropenQ = s.ropenQ := by rw [← hs']
    have hrecv : ∀ c, recvdOf sk s' c = recvdOf sk s c := by
      intro c
      cases c <;>
        simp [recvdOf, wkWireRecvd, wkAskedRecvd, asmResRecvd,
          asmLevelRecvd, habsW, habsA, hwalk, hasm, hgw, hifin, hrg, hrgr]
    have hsent : ∀ c, c ≠ Chan.level Party.I 0 →
        sentOf sk s' c = sentOf sk s c := by
      intro c hnc
      cases c with
      | level p j =>
          cases p with
          | I =>
              have hj : (j == 0) = false := by
                have : j ≠ 0 := fun h0 => hnc (by rw [h0])
                simp [this]
              simp [sentOf, asmOutSent, hj, hasm]
          | R => simp [sentOf, asmOutSent, hasm]
      | wire p h =>
          simp [sentOf, wkWireSent, wkWireCount, hwalk, hio1, hro1]
      | asked p h =>
          simp [sentOf, wkQSentTot, wkQSum, hwalk, hio2, hro3]
      | leafRequests => simp [sentOf, wkQSentTot, wkQSum, hwalk]
      | upper p h => simp [sentOf, wkParentSent, hwalk]
      | lower p h => simp [sentOf, wkResSent, wkResCount, hwalk]
      | rootret => simp [sentOf, asmOutSent, hasm]
      | rootrets => simp [sentOf, asmOutSent, hasm]
      | rootres => simp [sentOf, hro2]
    refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ⟨?_, ?_, ?_, ?_⟩, ?_⟩
    · rw [← hs']; exact hL.wk pk hpk
    · rw [← hs']; exact hL.asm pk hpk
    · have htop := hL.top
      rw [topLocalOk] at htop ⊢
      rw [show s'.absorbIdx = s.absorbIdx + 1 from by rw [← hs'],
        show s'.absorbPhase
            = (if s.absorbIdx + 1 < sk.totalLeafReqs then 0 else 3) from by
          rw [← hs'],
        show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.iopenCh = s.iopenCh from by rw [← hs'],
        show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.ropenCh = s.ropenCh from by rw [← hs'],
        show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
        show s'.rfinGot = s.rfinGot from by rw [← hs']]
      simp only [Bool.and_eq_true] at htop ⊢
      obtain ⟨⟨⟨⟨hpre, _h9⟩, _h10⟩, h11⟩, h12⟩ := htop
      refine ⟨⟨⟨⟨hpre, ?_⟩, ?_⟩, h11⟩, h12⟩
      · by_cases hlt2 : s.absorbIdx + 1 < sk.totalLeafReqs
        · simp [hlt2]
        · simp [hlt2]
          omega
      · have h5 : (if s.absorbIdx + 1 < sk.totalLeafReqs then 0 else 3)
            ≤ 5 := by
          split <;> omega
        simpa using h5
    · exact hlt
    · rw [← hs']
    · intro c _
      by_cases hc0 : c = Chan.level Party.I 0
      · subst hc0
        have hs1 : sentOf sk s' (Chan.level Party.I 0)
            = s.absorbIdx + 1 := by
          rw [← hs']; simp [sentOf]
        have hs0 : sentOf sk s (Chan.level Party.I 0) = s.absorbIdx := by
          simp [sentOf]
        simp [hs1, hs0]
      · rw [hsent c hc0, if_neg hc0]; rfl
    · intro c _; exact hrecv c
    · have hio : s'.iopenCh = s.iopenCh := by rw [← hs']
      have hro : s'.ropenCh = s.ropenCh := by rw [← hs']
      exact handsEq_of_other hio hro (fun pk => by rw [hwalk])

/-- `absorbCloseWire` moves the absorber past its wire input (phase
3→4): no channel operation, and no wire hand moves. -/
theorem step_absorbCloseWire
    (hstep : Model.apply sk ax .absorbCloseWire s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ QuietStep sk s s' ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨hph, _hprod⟩, _hzero⟩ := hg
    injection hstep with hs'
    refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ⟨?_, ?_, ?_⟩, ?_⟩
    · rw [← hs']; exact hL.wk pk hpk
    · rw [← hs']; exact hL.asm pk hpk
    · have htop := hL.top
      rw [topLocalOk] at htop ⊢
      rw [show s'.absorbPhase = 4 from by rw [← hs'],
        show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.iopenCh = s.iopenCh from by rw [← hs'],
        show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.ropenCh = s.ropenCh from by rw [← hs'],
        show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
        show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
        show s'.rfinGot = s.rfinGot from by rw [← hs']]
      simp only [Bool.and_eq_true] at htop ⊢
      obtain ⟨⟨⟨⟨hpre, h9⟩, _h10⟩, h11⟩, h12⟩ := htop
      refine ⟨⟨⟨⟨hpre, ?_⟩, rfl⟩, h11⟩, h12⟩
      rw [hph] at h9
      simpa using h9
    · rw [← hs']
    · intro c _; rw [← hs']; cases c <;> rfl
    · intro c _
      rw [← hs']
      cases c <;>
        simp [recvdOf, wkWireRecvd, wkAskedRecvd, asmResRecvd,
          asmLevelRecvd, absorbWireRecvd, absorbAskedRecvd, hph]
    · have hio : s'.iopenCh = s.iopenCh := by rw [← hs']
      have hro : s'.ropenCh = s.ropenCh := by rw [← hs']
      have hwk : ∀ pk, s'.walk pk = s.walk pk := fun pk => by rw [← hs']
      exact handsEq_of_other hio hro hwk

/-- `absorbCloseAsked` retires the absorber (phase 4→5): no channel
operation, and no wire hand moves. -/
theorem step_absorbCloseAsked
    (hstep : Model.apply sk ax .absorbCloseAsked s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ QuietStep sk s s' ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨hph, _hprod⟩, _hzero⟩ := hg
    injection hstep with hs'
    refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ⟨?_, ?_, ?_⟩, ?_⟩
    · rw [← hs']; exact hL.wk pk hpk
    · rw [← hs']; exact hL.asm pk hpk
    · have htop := hL.top
      rw [topLocalOk] at htop ⊢
      rw [show s'.absorbPhase = 5 from by rw [← hs'],
        show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.iopenCh = s.iopenCh from by rw [← hs'],
        show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.ropenCh = s.ropenCh from by rw [← hs'],
        show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
        show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
        show s'.rfinGot = s.rfinGot from by rw [← hs']]
      simp only [Bool.and_eq_true] at htop ⊢
      obtain ⟨⟨⟨⟨hpre, h9⟩, _h10⟩, h11⟩, h12⟩ := htop
      refine ⟨⟨⟨⟨hpre, ?_⟩, rfl⟩, h11⟩, h12⟩
      rw [hph] at h9
      simpa using h9
    · rw [← hs']
    · intro c _; rw [← hs']; cases c <;> rfl
    · intro c _
      rw [← hs']
      cases c <;>
        simp [recvdOf, wkWireRecvd, wkAskedRecvd, asmResRecvd,
          asmLevelRecvd, absorbWireRecvd, absorbAskedRecvd, hph]
    · have hio : s'.iopenCh = s.iopenCh := by rw [← hs']
      have hro : s'.ropenCh = s.ropenCh := by rw [← hs']
      have hwk : ∀ pk, s'.walk pk = s.walk pk := fun pk => by rw [← hs']
      exact handsEq_of_other hio hro hwk

-- ============================================================ finishes

/-- `finRet` consumes the initiator's root return: one receive on
`rootret`, and no wire hand moves. -/
theorem step_finRet
    (hstep : Model.apply sk ax .finRet s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ RecvStep sk s s' Chan.rootret ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, Bool.not_eq_true', decide_eq_true_eq] at hg
    obtain ⟨hifin, hpos⟩ := hg
    injection hstep with hs'
    refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ⟨?_, ?_, ?_, ?_⟩, ?_⟩
    · rw [← hs']; exact hL.wk pk hpk
    · rw [← hs']; exact hL.asm pk hpk
    · rw [← hs']; exact hL.top
    · exact hpos
    · rw [← hs']
    · intro c _; rw [← hs']; cases c <;> rfl
    · intro c _
      by_cases hc0 : c = Chan.rootret
      · subst hc0
        have hrecv1 : recvdOf sk s' Chan.rootret = 1 := by
          rw [← hs']; simp [recvdOf, b2n]
        have hrecv0 : recvdOf sk s Chan.rootret = 0 := by
          simp [recvdOf, b2n, hifin]
        simp [hrecv1, hrecv0]
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          cases c <;> first | rfl | exact absurd rfl hc0
        rw [hrecv, if_neg hc0]; rfl
    · have hio : s'.iopenCh = s.iopenCh := by rw [← hs']
      have hro : s'.ropenCh = s.ropenCh := by rw [← hs']
      have hwk : ∀ pk, s'.walk pk = s.walk pk := fun pk => by rw [← hs']
      exact handsEq_of_other hio hro hwk

/-- `finRes` consumes the responder's root resolution: one receive on
`rootres`, and no wire hand moves. -/
theorem step_finRes
    (hstep : Model.apply sk ax .finRes s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ RecvStep sk s s' Chan.rootres ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, Bool.not_eq_true', decide_eq_true_eq] at hg
    obtain ⟨hres, hpos⟩ := hg
    injection hstep with hs'
    refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ⟨?_, ?_, ?_, ?_⟩, ?_⟩
    · rw [← hs']; exact hL.wk pk hpk
    · rw [← hs']; exact hL.asm pk hpk
    · have htop := hL.top
      rw [topLocalOk] at htop ⊢
      rw [show s'.rfinGotRes = true from by rw [← hs'],
        show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.iopenCh = s.iopenCh from by rw [← hs'],
        show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.ropenCh = s.ropenCh from by rw [← hs'],
        show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
        show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
        show s'.rfinGot = s.rfinGot from by rw [← hs']]
      simp only [Bool.and_eq_true] at htop ⊢
      obtain ⟨⟨hpre, _h11⟩, h12⟩ := htop
      exact ⟨⟨hpre, rfl⟩, h12⟩
    · exact hpos
    · rw [← hs']
    · intro c _; rw [← hs']; cases c <;> rfl
    · intro c _
      by_cases hc0 : c = Chan.rootres
      · subst hc0
        have hrecv1 : recvdOf sk s' Chan.rootres = 1 := by
          rw [← hs']; simp [recvdOf, b2n]
        have hrecv0 : recvdOf sk s Chan.rootres = 0 := by
          simp [recvdOf, b2n, hres]
        simp [hrecv1, hrecv0]
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          cases c <;> first | rfl | exact absurd rfl hc0
        rw [hrecv, if_neg hc0]; rfl
    · have hio : s'.iopenCh = s.iopenCh := by rw [← hs']
      have hro : s'.ropenCh = s.ropenCh := by rw [← hs']
      have hwk : ∀ pk, s'.walk pk = s.walk pk := fun pk => by rw [← hs']
      exact handsEq_of_other hio hro hwk

/-- `finRets` consumes one root-level assembly return: one receive on
`rootrets`, and no wire hand moves. -/
theorem step_finRets
    (hstep : Model.apply sk ax .finRets s = some s')
    (hL : InvL sk ax s) :
    InvL sk ax s' ∧ RecvStep sk s s' Chan.rootrets ∧ HandsEq sk s s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, decide_eq_true_eq] at hg
    obtain ⟨⟨hres, hlt⟩, hpos⟩ := hg
    injection hstep with hs'
    refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ⟨?_, ?_, ?_, ?_⟩, ?_⟩
    · rw [← hs']; exact hL.wk pk hpk
    · rw [← hs']; exact hL.asm pk hpk
    · have htop := hL.top
      rw [topLocalOk] at htop ⊢
      rw [show s'.rfinGot = s.rfinGot + 1 from by rw [← hs'],
        show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.iopenCh = s.iopenCh from by rw [← hs'],
        show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.ropenCh = s.ropenCh from by rw [← hs'],
        show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
        show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
        show s'.rfinGotRes = s.rfinGotRes from by rw [← hs']]
      simp only [Bool.and_eq_true] at htop ⊢
      obtain ⟨⟨hpre, _h11⟩, _h12⟩ := htop
      refine ⟨⟨hpre, ?_⟩, ?_⟩
      · rw [hres]; rfl
      · simp only [decide_eq_true_eq]
        omega
    · exact hpos
    · rw [← hs']
    · intro c _; rw [← hs']; cases c <;> rfl
    · intro c _
      by_cases hc0 : c = Chan.rootrets
      · subst hc0
        have hrecv1 : recvdOf sk s' Chan.rootrets = s.rfinGot + 1 := by
          rw [← hs']; simp [recvdOf]
        have hrecv0 : recvdOf sk s Chan.rootrets = s.rfinGot := rfl
        simp [hrecv1, hrecv0]
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          cases c <;> first | rfl | exact absurd rfl hc0
        rw [hrecv, if_neg hc0]; rfl
    · have hio : s'.iopenCh = s.iopenCh := by rw [← hs']
      have hro : s'.ropenCh = s.ropenCh := by rw [← hs']
      have hwk : ∀ pk, s'.walk pk = s.walk pk := fun pk => by rw [← hs']
      exact handsEq_of_other hio hro hwk

end StreamingMirror.Mux
