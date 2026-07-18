/-
Preservation for the absorber and the two finish processes — the
consumers at the bottom and the sinks at the top. `preserve_finRet` is
the template proof for the whole `Proofs/Preserve/` tree: guard
extraction by `split at hstep`, component equations by `rw [← hs']`,
frame dispatch by the `*_congr`/`*_ext` lemmas, and the touched
channel's flow equation by `omega` over the extracted counts.

Chain (shared foundation): the absorber/finish preservation cases,
consumed by Preserve.lean. Map: Proofs/Map.lean.
-/
import StreamingMirror.Proofs.Lemmas

namespace StreamingMirror.Model

variable {sk : Skel} {ax : AxMode} {s s' : State}

/-- `finRet` consumes the initiator's root return: occupancy drops by
one exactly as the consumer count rises by one. -/
theorem preserve_finRet (_hwf : sk.wellFormed = true)
    (hstep : apply sk ax .finRet s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, Bool.not_eq_true', decide_eq_true_eq] at hg
    obtain ⟨hifin, hpos⟩ := hg
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · rw [← hs']; exact hi.top
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = bump s.chan Chan.rootret (-1) := by rw [← hs']
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']; cases c <;> rfl
      by_cases hne : c = Chan.rootret
      · subst hne
        have hrecv : recvdOf sk s' Chan.rootret = 1 := by
          rw [← hs']; simp [recvdOf, b2n]
        have hrecv0 : recvdOf sk s Chan.rootret = 0 := by
          simp [recvdOf, b2n, hifin]
        rw [hchan, hsent, hrecv, bump_neg_one]
        rw [hrecv0] at heq
        exact ⟨by omega, by omega⟩
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          cases c <;> first | rfl | exact absurd rfl hne
        rw [hchan, hsent, hrecv, bump_ne _ _ hne]
        exact ⟨heq, hcap⟩

/-- `absorbCloseWire` moves the absorber past its wire input (phase
3→4): no channel or cursor changes, and both recvd counts read
`phase ≥ 3` on either side, so everything frames. -/
theorem preserve_absorbCloseWire (_hwf : sk.wellFormed = true)
    (hstep : apply sk ax .absorbCloseWire s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨hph, _hprod⟩, _hzero⟩ := hg
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · have htop := hi.top
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
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = s.chan := by rw [← hs']
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']; cases c <;> rfl
      have hrecv : recvdOf sk s' c = recvdOf sk s c := by
        rw [← hs']
        cases c <;>
          simp [recvdOf, wkWireRecvd, wkAskedRecvd, asmResRecvd,
            asmLevelRecvd, absorbWireRecvd, absorbAskedRecvd, hph]
      rw [hchan, hsent, hrecv]
      exact ⟨heq, hcap⟩

/-- `absorbCloseAsked` retires the absorber (phase 4→5): no channel or
cursor changes, and both recvd counts read `phase ≥ 3` on either side,
so everything frames. -/
theorem preserve_absorbCloseAsked (_hwf : sk.wellFormed = true)
    (hstep : apply sk ax .absorbCloseAsked s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨hph, _hprod⟩, _hzero⟩ := hg
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · have htop := hi.top
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
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = s.chan := by rw [← hs']
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']; cases c <;> rfl
      have hrecv : recvdOf sk s' c = recvdOf sk s c := by
        rw [← hs']
        cases c <;>
          simp [recvdOf, wkWireRecvd, wkAskedRecvd, asmResRecvd,
            asmLevelRecvd, absorbWireRecvd, absorbAskedRecvd, hph]
      rw [hchan, hsent, hrecv]
      exact ⟨heq, hcap⟩

/-- `finRes` consumes the responder's root resolution: occupancy drops
by one exactly as the consumer count rises by one, and the
`rfinGotRes || rfinGot == 0` conjunct becomes true by its left arm. -/
theorem preserve_finRes (_hwf : sk.wellFormed = true)
    (hstep : apply sk ax .finRes s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, Bool.not_eq_true', decide_eq_true_eq] at hg
    obtain ⟨hres, hpos⟩ := hg
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · have htop := hi.top
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
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = bump s.chan Chan.rootres (-1) := by rw [← hs']
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']; cases c <;> rfl
      by_cases hne : c = Chan.rootres
      · subst hne
        have hrecv : recvdOf sk s' Chan.rootres = 1 := by
          rw [← hs']; simp [recvdOf, b2n]
        have hrecv0 : recvdOf sk s Chan.rootres = 0 := by
          simp [recvdOf, b2n, hres]
        rw [hchan, hsent, hrecv, bump_neg_one]
        rw [hrecv0] at heq
        exact ⟨by omega, by omega⟩
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          cases c <;> first | rfl | exact absurd rfl hne
        rw [hchan, hsent, hrecv, bump_ne _ _ hne]
        exact ⟨heq, hcap⟩

/-- `finRets` consumes one root-level assembly return: `rfinGot` (the
`rootrets` consumer count) rises with the drop in occupancy, the guard's
strict bound re-establishes `rfinGot ≤ rootPending`, and the guard's
`rfinGotRes` keeps the disjunction true by its left arm. -/
theorem preserve_finRets (_hwf : sk.wellFormed = true)
    (hstep : apply sk ax .finRets s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, decide_eq_true_eq] at hg
    obtain ⟨⟨hres, hlt⟩, hpos⟩ := hg
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · have htop := hi.top
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
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = bump s.chan Chan.rootrets (-1) := by rw [← hs']
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']; cases c <;> rfl
      by_cases hne : c = Chan.rootrets
      · subst hne
        have hrecv : recvdOf sk s' Chan.rootrets = s.rfinGot + 1 := by
          rw [← hs']; simp [recvdOf]
        have hrecv0 : recvdOf sk s Chan.rootrets = s.rfinGot := rfl
        rw [hchan, hsent, hrecv, bump_neg_one]
        rw [hrecv0] at heq
        exact ⟨by omega, by omega⟩
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          cases c <;> first | rfl | exact absurd rfl hne
        rw [hchan, hsent, hrecv, bump_ne _ _ hne]
        exact ⟨heq, hcap⟩

/-- `absorbRecvWire` consumes one leaf wire message (phase 0→1):
occupancy on `Chan.wire Party.R 0` drops by one exactly as
`absorbWireRecvd` rises from `absorbIdx` to `absorbIdx + 1`
(`wf_rootH` keeps the channel out of the root-wire branch). -/
theorem preserve_absorbRecvWire (hwf : sk.wellFormed = true)
    (hstep : apply sk ax .absorbRecvWire s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
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
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · have htop := hi.top
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
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = bump s.chan (Chan.wire Party.R 0) (-1) := by
        rw [← hs']
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']; cases c <;> rfl
      by_cases hne : c = Chan.wire Party.R 0
      · subst hne
        have hrecv : recvdOf sk s' (Chan.wire Party.R 0)
            = s.absorbIdx + 1 := by
          rw [← hs']; simp [recvdOf, absorbWireRecvd, hrH]
        have hrecv0 : recvdOf sk s (Chan.wire Party.R 0) = s.absorbIdx := by
          simp [recvdOf, absorbWireRecvd, hrH, hph]
        rw [hchan, hsent, hrecv, bump_neg_one]
        rw [hrecv0] at heq
        exact ⟨by omega, by omega⟩
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          cases c with
          | wire p h =>
              cases p with
              | I => rfl
              | R =>
                  have hh : h ≠ 0 := fun h0 => hne (by rw [h0])
                  have h0 : (h == 0) = false := by simp [hh]
                  simp [recvdOf, wkWireRecvd, h0]
          | leafRequests => simp [recvdOf, absorbAskedRecvd, hph]
          | _ => rfl
        rw [hchan, hsent, hrecv, bump_ne _ _ hne]
        exact ⟨heq, hcap⟩

/-- `absorbRecvAsked` consumes one leaf request (phase 1→2): occupancy
on `Chan.leafRequests` drops by one exactly as `absorbAskedRecvd` rises
from `absorbIdx` to `absorbIdx + 1`; `absorbWireRecvd` reads
`absorbIdx + 1` in both phases, so the wire channel frames. -/
theorem preserve_absorbRecvAsked (_hwf : sk.wellFormed = true)
    (hstep : apply sk ax .absorbRecvAsked s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨hph, hpos⟩ := hg
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · have htop := hi.top
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
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = bump s.chan Chan.leafRequests (-1) := by
        rw [← hs']
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']; cases c <;> rfl
      by_cases hne : c = Chan.leafRequests
      · subst hne
        have hrecv : recvdOf sk s' Chan.leafRequests = s.absorbIdx + 1 := by
          rw [← hs']; simp [recvdOf, absorbAskedRecvd]
        have hrecv0 : recvdOf sk s Chan.leafRequests = s.absorbIdx := by
          simp [recvdOf, absorbAskedRecvd, hph]
        rw [hchan, hsent, hrecv, bump_neg_one]
        rw [hrecv0] at heq
        exact ⟨by omega, by omega⟩
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          cases c with
          | wire p h => simp [recvdOf, wkWireRecvd, absorbWireRecvd, hph]
          | leafRequests => exact absurd rfl hne
          | _ => rfl
        rw [hchan, hsent, hrecv, bump_ne _ _ hne]
        exact ⟨heq, hcap⟩

/-- `absorbSend` publishes one absorbed pair on `Chan.level Party.I 0`
(phase 2→0 or 2→3): the producer count of that channel is `absorbIdx`,
which rises with the send. Both consumer-side counts stay constant even
though phase and cursor both move: in phase 2 they read `absorbIdx + 1`,
and afterwards either the fresh phase 0 reads `(absorbIdx + 1) + 0` or
the closing phase 3 reads `totalLeafReqs`, which the failed guard and
the cursor bound pin to `absorbIdx + 1`. -/
theorem preserve_absorbSend (_hwf : sk.wellFormed = true)
    (hstep : apply sk ax .absorbSend s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨hph, hlt⟩ := hg
    have hidx : s.absorbIdx < sk.totalLeafReqs := by
      have htop := hi.top
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
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · have htop := hi.top
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
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = bump s.chan (Chan.level Party.I 0) 1 := by
        rw [← hs']
      by_cases hne : c = Chan.level Party.I 0
      · subst hne
        have hs1 : sentOf sk s' (Chan.level Party.I 0)
            = s.absorbIdx + 1 := by
          rw [← hs']; simp [sentOf]
        have hs0 : sentOf sk s (Chan.level Party.I 0) = s.absorbIdx := by
          simp [sentOf]
        rw [hchan, bump_one, hrecv, hs1]
        rw [hs0] at heq
        exact ⟨by omega, by omega⟩
      · rw [hchan, bump_ne _ _ hne, hsent c hne, hrecv c]
        exact ⟨heq, hcap⟩

end StreamingMirror.Model
