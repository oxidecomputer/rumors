/-
O preservation for the two openers (`iopenChoose`/`iopenFire`/
`ropenRecv`/`ropenChoose`/`ropenFire`). Every arm here is a base arm —
`applyO`'s catch-all delegates, so each `hstep` IS a base step
definitionally — and the wk/asm/top fields transcribe verbatim from
the base file (opener guards and ledgers are order-blind). The one
delta is the flow field: the opener channels' O consumer counts
delegate to the base counts (`recvdOfO`'s root-wire arm and catch-all),
and every walk/absorber O count frames because neither base formula
moves — either by plain `rfl`/`recvdOfO_ext` (the opener scalars are
outside the observation set) or, where `ropenGotWire` flips, by the
per-assignment dispatch over the two unchanged base formulas.

Chain (ord, stage B): the opener preservation cases, consumed by
Ord/Preserve.lean. Base mirror: Proofs/Preserve/Top.lean. Map:
PROGRESS.md §13.
-/
import StreamingMirror.Ord.Wiring

namespace StreamingMirror.Ord

open Model

variable {sk : Skel} {ax : AxMode} {ord : OrdMap} {s s' : State}

/-- `iopenChoose` under the assignment (a shared base arm): the
committed-arm conjuncts of `topLocalOk` are exactly the
`iopenChoosable` guard, and no count moves (cf. `preserve_iopenChoose`). -/
theorem preserve_iopenChooseO (_hwf : sk.wellFormed = true) (o : IOblig)
    (hstep : applyO sk ax ord (.iopenChoose o) s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  have hstep' : apply sk ax (.iopenChoose o) s = some s' := hstep
  simp only [apply] at hstep'
  split at hstep'
  case isFalse => simp at hstep'
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨hnone, hch⟩ := hg
    injection hstep' with hs'
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
      have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
        rw [← hs']
        exact recvdOfO_ext (fun _ => rfl) (fun _ => rfl) (fun _ => rfl)
          rfl rfl rfl rfl rfl rfl c
      rw [hchan, hsent, hrecv]
      exact ⟨heq, hcap⟩

/-- `iopenFire` under the assignment (a shared base arm): occupancy at
the fired channel rises by one exactly as the producer bit flips 0→1;
no O consumer count reads the opener producer scalars (cf.
`preserve_iopenFire`). -/
theorem preserve_iopenFireO (_hwf : sk.wellFormed = true)
    (hstep : applyO sk ax ord .iopenFire s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  have hstep' : apply sk ax .iopenFire s = some s' := hstep
  simp only [apply] at hstep'
  split at hstep'
  next hch =>
    -- committed obligation: .wire
    split at hstep'
    case isFalse => simp at hstep'
    case isTrue hg =>
      injection hstep' with hs'
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
        have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
          rw [← hs']
          exact recvdOfO_ext (fun _ => rfl) (fun _ => rfl) (fun _ => rfl)
            rfl rfl rfl rfl rfl rfl c
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
    split at hstep'
    case isFalse => simp at hstep'
    case isTrue hg =>
      injection hstep' with hs'
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
        have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
          rw [← hs']
          exact recvdOfO_ext (fun _ => rfl) (fun _ => rfl) (fun _ => rfl)
            rfl rfl rfl rfl rfl rfl c
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
  next => simp at hstep'

/-- `ropenRecv` under the assignment (a shared base arm): occupancy of
the root wire drops by one exactly as its O consumer count — the
order-blind `ropenGotWire` bit — flips 0→1; every walk/absorber O
count frames because neither base formula moves (cf.
`preserve_ropenRecv`). -/
theorem preserve_ropenRecvO (_hwf : sk.wellFormed = true)
    (hstep : applyO sk ax ord .ropenRecv s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  have hstep' : apply sk ax .ropenRecv s = some s' := hstep
  simp only [apply] at hstep'
  split at hstep'
  case isFalse => simp at hstep'
  case isTrue hg =>
    simp only [Bool.and_eq_true, Bool.not_eq_true', decide_eq_true_eq] at hg
    obtain ⟨hgot, hpos⟩ := hg
    injection hstep' with hs'
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
        have hrecv0 : recvdOfO sk ord s (Chan.wire Party.I sk.rootH) = 0 := by
          simp [recvdOfO, b2n, hgot]
        have hrecv1 : recvdOfO sk ord s' (Chan.wire Party.I sk.rootH) = 1 := by
          rw [← hs']; simp [recvdOfO, b2n]
        rw [hchan, hsent, hrecv1, bump_neg_one]
        rw [hrecv0] at heq
        exact ⟨by omega, by omega⟩
      · have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
          rw [← hs']
          cases c with
          | wire p h =>
              by_cases hh : (h == sk.rootH) = true
              · by_cases hp : (p == Party.I) = true
                · rw [beq_iff_eq] at hh hp
                  exact absurd (by rw [hh, hp]) hne
                · cases hord : ord.walk (Party.I, sk.rootH - 1) <;>
                    simp [recvdOfO, hh, hp, wkWireRecvdO, hord,
                      wkWireRecvd, wkAskedRecvd]
              · cases hord : ord.walk (p.other, h - 1) <;>
                cases horda : ord.absorb <;>
                  simp [recvdOfO, hh, wkWireRecvdO, hord, absorbWireRecvdO,
                    horda, wkWireRecvd, wkAskedRecvd, absorbWireRecvd,
                    absorbAskedRecvd]
          | asked p h =>
              cases hord : ord.walk (p, h) <;>
                simp [recvdOfO, wkAskedRecvdO, hord, wkWireRecvd, wkAskedRecvd]
          | leafRequests =>
              cases horda : ord.absorb <;>
                simp [recvdOfO, absorbAskedRecvdO, horda, absorbWireRecvd,
                  absorbAskedRecvd]
          | upper p h => rfl
          | lower p h => rfl
          | level p j => rfl
          | rootret => rfl
          | rootrets => rfl
          | rootres => rfl
        rw [hchan, hsent, hrecv, bump_ne _ _ hne]
        exact ⟨heq, hcap⟩

/-- `ropenChoose` under the assignment (a shared base arm): the
committed-arm conjuncts of `topLocalOk` are exactly the
`ropenChoosable` guard, and no count moves (cf. `preserve_ropenChoose`). -/
theorem preserve_ropenChooseO (_hwf : sk.wellFormed = true) (o : ROblig)
    (hstep : applyO sk ax ord (.ropenChoose o) s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  have hstep' : apply sk ax (.ropenChoose o) s = some s' := hstep
  simp only [apply] at hstep'
  split at hstep'
  case isFalse => simp at hstep'
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨hnone, hch⟩ := hg
    injection hstep' with hs'
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
      have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
        rw [← hs']
        exact recvdOfO_ext (fun _ => rfl) (fun _ => rfl) (fun _ => rfl)
          rfl rfl rfl rfl rfl rfl c
      rw [hchan, hsent, hrecv]
      exact ⟨heq, hcap⟩

/-- `ropenFire` under the assignment (a shared base arm): occupancy at
the fired channel rises by one exactly as the producer count moves; no
O consumer count reads the opener producer scalars (cf.
`preserve_ropenFire`). -/
theorem preserve_ropenFireO (_hwf : sk.wellFormed = true)
    (hstep : applyO sk ax ord .ropenFire s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  have hstep' : apply sk ax .ropenFire s = some s' := hstep
  simp only [apply] at hstep'
  split at hstep'
  next hch =>
    -- committed obligation: .wire
    split at hstep'
    case isFalse => simp at hstep'
    case isTrue hg =>
      injection hstep' with hs'
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
        have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
          rw [← hs']
          exact recvdOfO_ext (fun _ => rfl) (fun _ => rfl) (fun _ => rfl)
            rfl rfl rfl rfl rfl rfl c
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
    split at hstep'
    case isFalse => simp at hstep'
    case isTrue hg =>
      injection hstep' with hs'
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
        have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
          rw [← hs']
          exact recvdOfO_ext (fun _ => rfl) (fun _ => rfl) (fun _ => rfl)
            rfl rfl rfl rfl rfl rfl c
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
    split at hstep'
    case isFalse => simp at hstep'
    case isTrue hg =>
      injection hstep' with hs'
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
        have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
          rw [← hs']
          exact recvdOfO_ext (fun _ => rfl) (fun _ => rfl) (fun _ => rfl)
            rfl rfl rfl rfl rfl rfl c
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
  next => simp at hstep'

end StreamingMirror.Ord
