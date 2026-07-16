/-
Preservation for the two openers (the initiator's and responder's root
processes). The `Choose` actions are the committed-choice halves: they
move a guard's worth of facts into the `committed` field, and the
invariant's obligation arms are the guards verbatim — the transcription
rule "mirror the guards exactly" pays off here as `simp_all` closures.
-/
import StreamingMirror.Proofs.Lemmas

namespace StreamingMirror.Model

variable {sk : Skel} {ax : AxMode} {s s' : State}

/-- `iopenChoose` commits the initiator opener to an obligation its
ledger permits; the committed-arm conjuncts of `topLocalOk` are exactly
the `iopenChoosable` guard. -/
theorem preserve_iopenChoose (_hwf : sk.wellFormed = true) (o : IOblig)
    (hstep : apply sk ax (.iopenChoose o) s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨hnone, hch⟩ := hg
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · have htop := hi.top
      have hCh : s'.iopenCh = some o := by rw [← hs']
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
      cases o <;> simp_all [iopenChoosable]
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = s.chan := by rw [← hs']
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']; cases c <;> rfl
      have hrecv : recvdOf sk s' c = recvdOf sk s c := by
        rw [← hs']; cases c <;> rfl
      rw [hchan, hsent, hrecv]
      exact ⟨heq, hcap⟩

/-- `iopenFire` publishes the initiator's committed opening obligation:
occupancy at the fired channel rises by one exactly as the producer bit
flips 0→1 (the committed-arm conjunct of `topLocalOk` guarantees the bit
was clear), and clearing `iopenCh` leaves the commit arms vacuous. -/
theorem preserve_iopenFire (_hwf : sk.wellFormed = true)
    (hstep : apply sk ax .iopenFire s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  next hch =>
    -- committed obligation: .wire
    split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      injection hstep with hs'
      have hiw : s.iopenWire = false := by
        have htop := hi.top
        rw [topLocalOk] at htop
        simp only [Bool.and_eq_true] at htop
        have h1 := htop.1.1.1.1.1.1.1.1.1.1.1
        rw [hch] at h1
        simpa using h1
      refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
      · rw [← hs']; exact hi.wk pk hpk
      · rw [← hs']; exact hi.asm pk hpk
      · have htop := hi.top
        rw [topLocalOk] at htop ⊢
        rw [show s'.iopenCh = none from by rw [← hs'],
          show s'.iopenWire = true from by rw [← hs'],
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
        rw [hch] at htop
        simp_all
      · obtain ⟨heq, hcap⟩ := hi.flow c hc
        have hchan : s'.chan = bump s.chan (Chan.wire Party.I sk.rootH) 1 := by
          rw [← hs']
        have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']; cases c <;> rfl
        by_cases hne : c = Chan.wire Party.I sk.rootH
        · subst hne
          have hsent0 : sentOf sk s (Chan.wire Party.I sk.rootH) = 0 := by
            simp [sentOf, hiw, b2n]
          have hsent1 : sentOf sk s' (Chan.wire Party.I sk.rootH) = 1 := by
            rw [← hs']; simp [sentOf, b2n]
          have hcap1 : sk.cap (Chan.wire Party.I sk.rootH) = 1 := rfl
          rw [hchan, hrecv, hsent1, bump_one, hcap1]
          rw [hsent0] at heq
          exact ⟨by omega, by omega⟩
        · have hsent : sentOf sk s' c = sentOf sk s c := by
            rw [← hs']
            cases c with
            | wire p h =>
                by_cases hh : (h == sk.rootH) = true
                · by_cases hp : (p == Party.I) = true
                  · rw [beq_iff_eq] at hh hp
                    exact absurd (by rw [hh, hp]) hne
                  · simp [sentOf, hh, hp]
                · simp [sentOf, wkWireSent, wkWireCount, hh]
            | asked p h => rfl
            | leafRequests => rfl
            | upper p h => rfl
            | lower p h => rfl
            | level p j => rfl
            | rootret => rfl
            | rootrets => rfl
            | rootres => rfl
          rw [hchan, hrecv, hsent, bump_ne _ _ hne]
          exact ⟨heq, hcap⟩
  next hch =>
    -- committed obligation: .query
    split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      injection hstep with hs'
      have hiq : s.iopenQuery = false := by
        have htop := hi.top
        rw [topLocalOk] at htop
        simp only [Bool.and_eq_true] at htop
        have h2 := htop.1.1.1.1.1.1.1.1.1.1.2
        rw [hch] at h2
        simp at h2
        exact h2.1
      refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
      · rw [← hs']; exact hi.wk pk hpk
      · rw [← hs']; exact hi.asm pk hpk
      · have htop := hi.top
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
        rw [hch] at htop
        simp_all
      · obtain ⟨heq, hcap⟩ := hi.flow c hc
        have hchan :
            s'.chan = bump s.chan (Chan.asked Party.I (sk.rootH - 1)) 1 := by
          rw [← hs']
        have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']; cases c <;> rfl
        by_cases hne : c = Chan.asked Party.I (sk.rootH - 1)
        · subst hne
          have hsent0 :
              sentOf sk s (Chan.asked Party.I (sk.rootH - 1)) = 0 := by
            simp [sentOf, hiq, b2n]
          have hsent1 :
              sentOf sk s' (Chan.asked Party.I (sk.rootH - 1)) = 1 := by
            rw [← hs']; simp [sentOf, b2n]
          have hcap1 : sk.cap (Chan.asked Party.I (sk.rootH - 1)) = 1 := rfl
          rw [hchan, hrecv, hsent1, bump_one, hcap1]
          rw [hsent0] at heq
          exact ⟨by omega, by omega⟩
        · have hsent : sentOf sk s' c = sentOf sk s c := by
            rw [← hs']
            cases c with
            | asked p h =>
                by_cases h1 : (p == Party.I && h == sk.rootH - 1) = true
                · simp only [Bool.and_eq_true, beq_iff_eq] at h1
                  exact absurd (by rw [h1.1, h1.2]) hne
                · simp [sentOf, wkQSentTot, wkQSum, h1]
            | wire p h => rfl
            | leafRequests => rfl
            | upper p h => rfl
            | lower p h => rfl
            | level p j => rfl
            | rootret => rfl
            | rootrets => rfl
            | rootres => rfl
          rw [hchan, hrecv, hsent, bump_ne _ _ hne]
          exact ⟨heq, hcap⟩
  next => simp at hstep

/-- `ropenRecv` consumes the initiator's opening wire message: occupancy
drops by one exactly as the consumer bit flips 0→1, and the newly-true
`ropenGotWire` satisfies its `topLocalOk` disjunct by the left arm. -/
theorem preserve_ropenRecv (_hwf : sk.wellFormed = true)
    (hstep : apply sk ax .ropenRecv s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, Bool.not_eq_true', decide_eq_true_eq] at hg
    obtain ⟨hgot, hpos⟩ := hg
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · have htop := hi.top
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
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = bump s.chan (Chan.wire Party.I sk.rootH) (-1) := by
        rw [← hs']
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']; cases c <;> rfl
      by_cases hne : c = Chan.wire Party.I sk.rootH
      · subst hne
        have hrecv0 : recvdOf sk s (Chan.wire Party.I sk.rootH) = 0 := by
          simp [recvdOf, hgot, b2n]
        have hrecv1 : recvdOf sk s' (Chan.wire Party.I sk.rootH) = 1 := by
          rw [← hs']; simp [recvdOf, b2n]
        rw [hchan, hsent, hrecv1, bump_neg_one]
        rw [hrecv0] at heq
        exact ⟨by omega, by omega⟩
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          cases c with
          | wire p h =>
              by_cases hh : (h == sk.rootH) = true
              · by_cases hp : (p == Party.I) = true
                · rw [beq_iff_eq] at hh hp
                  exact absurd (by rw [hh, hp]) hne
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
        rw [hchan, hsent, hrecv, bump_ne _ _ hne]
        exact ⟨heq, hcap⟩

/-- `ropenChoose` commits the responder opener to an obligation its
ledger permits; the committed-arm conjuncts of `topLocalOk` are exactly
the `ropenChoosable` guard (with `rootPending` unfolded to the root
scope's child count). -/
theorem preserve_ropenChoose (_hwf : sk.wellFormed = true) (o : ROblig)
    (hstep : apply sk ax (.ropenChoose o) s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨hnone, hch⟩ := hg
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · have htop := hi.top
      have hCh : s'.ropenCh = some o := by rw [← hs']
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
      cases o <;> simp_all [ropenChoosable, Skel.rootPending]
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = s.chan := by rw [← hs']
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']; cases c <;> rfl
      have hrecv : recvdOf sk s' c = recvdOf sk s c := by
        rw [← hs']; cases c <;> rfl
      rw [hchan, hsent, hrecv]
      exact ⟨heq, hcap⟩

/-- `ropenFire` publishes the responder's committed opening obligation:
occupancy at the fired channel rises by one exactly as the producer
count moves (bit 0→1 for `.wire`/`.res`, `ropenQ` +1 for `.query`), the
committed-arm conjunct supplies the freshness of the fired fact, and the
fired-fact shadows re-establish from the same conjunct. -/
theorem preserve_ropenFire (_hwf : sk.wellFormed = true)
    (hstep : apply sk ax .ropenFire s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  next hch =>
    -- committed obligation: .wire
    split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      injection hstep with hs'
      have hrw : s.ropenWire = false := by
        have htop := hi.top
        rw [topLocalOk] at htop
        simp only [Bool.and_eq_true] at htop
        have h6 := htop.1.1.1.1.1.1.2
        rw [hch] at h6
        simpa using h6
      refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
      · rw [← hs']; exact hi.wk pk hpk
      · rw [← hs']; exact hi.asm pk hpk
      · have htop := hi.top
        rw [topLocalOk] at htop ⊢
        rw [show s'.iopenWire = s.iopenWire from by rw [← hs'],
          show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
          show s'.iopenCh = s.iopenCh from by rw [← hs'],
          show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
          show s'.ropenWire = true from by rw [← hs'],
          show s'.ropenRes = s.ropenRes from by rw [← hs'],
          show s'.ropenQ = s.ropenQ from by rw [← hs'],
          show s'.ropenCh = none from by rw [← hs'],
          show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
          show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
          show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
          show s'.rfinGot = s.rfinGot from by rw [← hs']]
        rw [hch] at htop
        simp_all
      · obtain ⟨heq, hcap⟩ := hi.flow c hc
        have hchan : s'.chan = bump s.chan (Chan.wire Party.R sk.rootH) 1 := by
          rw [← hs']
        have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']; cases c <;> rfl
        by_cases hne : c = Chan.wire Party.R sk.rootH
        · subst hne
          have hsent0 : sentOf sk s (Chan.wire Party.R sk.rootH) = 0 := by
            simp [sentOf, hrw, b2n]
          have hsent1 : sentOf sk s' (Chan.wire Party.R sk.rootH) = 1 := by
            rw [← hs']; simp [sentOf, b2n]
          have hcap1 : sk.cap (Chan.wire Party.R sk.rootH) = 1 := rfl
          rw [hchan, hrecv, hsent1, bump_one, hcap1]
          rw [hsent0] at heq
          exact ⟨by omega, by omega⟩
        · have hsent : sentOf sk s' c = sentOf sk s c := by
            rw [← hs']
            cases c with
            | wire p h =>
                by_cases hh : (h == sk.rootH) = true
                · by_cases hp : (p == Party.I) = true
                  · simp [sentOf, hh, hp]
                  · have hpR : p = Party.R := by
                      cases p
                      · exact absurd rfl hp
                      · rfl
                    rw [beq_iff_eq] at hh
                    exact absurd (by rw [hh, hpR]) hne
                · simp [sentOf, wkWireSent, wkWireCount, hh]
            | asked p h => rfl
            | leafRequests => rfl
            | upper p h => rfl
            | lower p h => rfl
            | level p j => rfl
            | rootret => rfl
            | rootrets => rfl
            | rootres => rfl
          rw [hchan, hrecv, hsent, bump_ne _ _ hne]
          exact ⟨heq, hcap⟩
  next hch =>
    -- committed obligation: .res
    split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      injection hstep with hs'
      have hrr : s.ropenRes = false := by
        have htop := hi.top
        rw [topLocalOk] at htop
        simp only [Bool.and_eq_true] at htop
        have h7 := htop.1.1.1.1.1.2
        rw [hch] at h7
        simp at h7
        exact h7.1
      refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
      · rw [← hs']; exact hi.wk pk hpk
      · rw [← hs']; exact hi.asm pk hpk
      · have htop := hi.top
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
        rw [hch] at htop
        simp_all
      · obtain ⟨heq, hcap⟩ := hi.flow c hc
        have hchan : s'.chan = bump s.chan Chan.rootres 1 := by
          rw [← hs']
        have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']; cases c <;> rfl
        by_cases hne : c = Chan.rootres
        · subst hne
          have hsent0 : sentOf sk s Chan.rootres = 0 := by
            simp [sentOf, hrr, b2n]
          have hsent1 : sentOf sk s' Chan.rootres = 1 := by
            rw [← hs']; simp [sentOf, b2n]
          have hcap1 : sk.cap Chan.rootres = 1 := rfl
          rw [hchan, hrecv, hsent1, bump_one, hcap1]
          rw [hsent0] at heq
          exact ⟨by omega, by omega⟩
        · have hsent : sentOf sk s' c = sentOf sk s c := by
            rw [← hs']
            cases c <;> first | rfl | exact absurd rfl hne
          rw [hchan, hrecv, hsent, bump_ne _ _ hne]
          exact ⟨heq, hcap⟩
  next hch =>
    -- committed obligation: .query
    split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      injection hstep with hs'
      have hq : s.ropenQ < sk.rootPending := by
        have htop := hi.top
        rw [topLocalOk] at htop
        simp only [Bool.and_eq_true] at htop
        have h8 := htop.1.1.1.1.2
        rw [hch] at h8
        simp at h8
        exact h8.1.1
      refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
      · rw [← hs']; exact hi.wk pk hpk
      · rw [← hs']; exact hi.asm pk hpk
      · have htop := hi.top
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
        rw [hch] at htop
        simp_all
        omega
      · obtain ⟨heq, hcap⟩ := hi.flow c hc
        have hchan :
            s'.chan = bump s.chan (Chan.asked Party.R (sk.rootH - 2)) 1 := by
          rw [← hs']
        have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']; cases c <;> rfl
        by_cases hne : c = Chan.asked Party.R (sk.rootH - 2)
        · subst hne
          have hsent0 :
              sentOf sk s (Chan.asked Party.R (sk.rootH - 2)) = s.ropenQ := by
            simp [sentOf]
          have hsent1 :
              sentOf sk s' (Chan.asked Party.R (sk.rootH - 2))
                = s.ropenQ + 1 := by
            rw [← hs']; simp [sentOf]
          have hcap1 : sk.cap (Chan.asked Party.R (sk.rootH - 2)) = 1 := rfl
          rw [hchan, hrecv, hsent1, bump_one, hcap1]
          rw [hsent0] at heq
          exact ⟨by omega, by omega⟩
        · have hsent : sentOf sk s' c = sentOf sk s c := by
            rw [← hs']
            cases c with
            | asked p h =>
                by_cases h2 : (p == Party.R && h == sk.rootH - 2) = true
                · simp only [Bool.and_eq_true, beq_iff_eq] at h2
                  exact absurd (by rw [h2.1, h2.2]) hne
                · simp [sentOf, wkQSentTot, wkQSum, h2]
            | wire p h => rfl
            | leafRequests => rfl
            | upper p h => rfl
            | lower p h => rfl
            | level p j => rfl
            | rootret => rfl
            | rootrets => rfl
            | rootres => rfl
          rw [hchan, hrecv, hsent, bump_ne _ _ hne]
          exact ⟨heq, hcap⟩
  next => simp at hstep

end StreamingMirror.Model
