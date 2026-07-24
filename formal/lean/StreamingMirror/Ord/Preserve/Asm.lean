/-
O preservation for the assemblers (`asmRecvRes`/`asmRecvLevel`/
`asmSend`/`asmClose`). Every arm here is a base arm — `applyO`'s
catch-all delegates — and assemblers have no pairing loop: their input
channels' O consumer counts ARE the base counts (`recvdOfO`'s
catch-all), so the touched-channel flow equations transcribe verbatim.
The one delta is the frame layer: the three `setAsm` flow-frame lemmas
re-land over `recvdOfO` as private twins whose walk/absorber arms are
definitional (`setAsm` moves no field either base receive formula
reads) and whose assembler arms delegate to the base lemmas.

Chain (ord, stage B): the assembler-family preservation cases,
consumed by Ord/Preserve.lean. Base mirror: Proofs/Preserve/Asm.lean.
Map: PROGRESS.md §13.
-/
import StreamingMirror.Proofs.Preserve.Asm
import StreamingMirror.Ord.Wiring

namespace StreamingMirror.Ord

open Model

variable {sk : Skel} {ax : AxMode} {ord : OrdMap} {s s' : State}

-- ================================================== the setAsm O frames

/-- An asm update at `pk` that preserves both of `pk`'s consumer counts
is invisible to every O consumer count (a twin of the base
`recvdOf_setAsm_of_counts`): the walk/absorber arms are definitional,
the assembler arms delegate. -/
private theorem recvdOfO_setAsm_of_counts (s : State)
    (pk : Party × Nat) (a' : AsmSt)
    (hres : asmResRecvd (setAsm s pk a') pk = asmResRecvd s pk)
    (hlev : asmLevelRecvd sk (setAsm s pk a') pk = asmLevelRecvd sk s pk)
    (c : Chan) :
    recvdOfO sk ord (setAsm s pk a') c = recvdOfO sk ord s c := by
  cases c with
  | wire p h => rfl
  | asked p h => rfl
  | leafRequests => rfl
  | upper p h => exact recvdOf_setAsm_of_counts sk s pk a' hres hlev (.upper p h)
  | lower p h => exact recvdOf_setAsm_of_counts sk s pk a' hres hlev (.lower p h)
  | level p j => exact recvdOf_setAsm_of_counts sk s pk a' hres hlev (.level p j)
  | rootret => rfl
  | rootrets => rfl
  | rootres => rfl

/-- O flow frame for `asmRecvRes`: away from `asmResChan pk`, a `setAsm`
at `pk` that preserves the level count is invisible to every `allChans`
O consumer count (a twin of the base `recvdOf_setAsm_frame_res`,
membership relativization included). -/
private theorem recvdOfO_setAsm_frame_res (hwf : sk.wellFormed = true)
    (s : State) (pk : Party × Nat) (a' : AsmSt) {c : Chan}
    (hc : c ∈ allChans sk) (hne : c ≠ asmResChan pk)
    (hlev : asmLevelRecvd sk (setAsm s pk a') pk = asmLevelRecvd sk s pk) :
    recvdOfO sk ord (setAsm s pk a') c = recvdOfO sk ord s c := by
  cases c with
  | wire p h => rfl
  | asked p h => rfl
  | leafRequests => rfl
  | upper p h => exact recvdOf_setAsm_frame_res hwf s pk a' hc hne hlev
  | lower p h => exact recvdOf_setAsm_frame_res hwf s pk a' hc hne hlev
  | level p j => exact recvdOf_setAsm_frame_res hwf s pk a' hc hne hlev
  | rootret => rfl
  | rootrets => rfl
  | rootres => rfl

/-- O flow frame for `asmRecvLevel`: away from `asmLevelChan pk`, a
`setAsm` at `pk` that preserves the res count is invisible to every O
consumer count (a twin of the base `recvdOf_setAsm_frame_level`). -/
private theorem recvdOfO_setAsm_frame_level (s : State)
    (pk : Party × Nat) (a' : AsmSt) {c : Chan}
    (hne : c ≠ asmLevelChan pk)
    (hres : asmResRecvd (setAsm s pk a') pk = asmResRecvd s pk) :
    recvdOfO sk ord (setAsm s pk a') c = recvdOfO sk ord s c := by
  cases c with
  | wire p h => rfl
  | asked p h => rfl
  | leafRequests => rfl
  | upper p h => exact recvdOf_setAsm_frame_level (sk := sk) s pk a' hne hres
  | lower p h => exact recvdOf_setAsm_frame_level (sk := sk) s pk a' hne hres
  | level p j => exact recvdOf_setAsm_frame_level (sk := sk) s pk a' hne hres
  | rootret => rfl
  | rootrets => rfl
  | rootres => rfl

-- ====================================================== the four arms

/-- `asmClose` under the assignment (a shared base arm): phase 3 → 4
with nothing else moving; both consumer counts are cursor-only on
either side, so every O count frames (cf. `preserve_asmClose`). -/
theorem preserve_asmCloseO (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : applyO sk ax ord (.asmClose pk) s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  have hstep' : apply sk ax (.asmClose pk) s = some s' := hstep
  simp only [apply] at hstep'
  split at hstep'
  case isFalse => simp at hstep'
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨⟨hmem, hph⟩, _hpd⟩, _hch0⟩ := hg
    injection hstep' with hs'
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk' hpk'
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        have hasm : s'.asm pk' = { s.asm pk' with phase := 4 } := by
          rw [← hs']; simp
        have hold := hi.asm pk' hpk'
        simp only [asmLocalOk, hasm, hph] at hold ⊢
        simp at hold ⊢
        omega
      · have ha : s'.asm pk' = s.asm pk' := by
          rw [← hs']; exact setAsm_asm_ne s _ hpkeq
        rw [asmLocalOk_congr sk pk' ha]; exact hi.asm pk' hpk'
    · rw [← hs']; exact hi.top
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = s.chan := by rw [← hs']; rfl
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']
        apply sentOf_ext_idx
        · intro pk'
          by_cases hq : pk' = pk
          · subst hq; simp
          · simp [setAsm_asm_ne s _ hq]
        all_goals rfl
      have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
        rw [← hs']
        apply recvdOfO_setAsm_of_counts
        · simp [asmResRecvd, hph]
        · simp [asmLevelRecvd]
      rw [hchan, hsent, hrecv]
      exact ⟨heq, hcap⟩

/-- `asmRecvRes` under the assignment (a shared base arm): occupancy on
`asmResChan pk` drops by one exactly as the O consumer count — the base
`asmResRecvd`, by `recvdOfO`'s delegation — rises by one (cf.
`preserve_asmRecvRes`). -/
theorem preserve_asmRecvResO (hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : applyO sk ax ord (.asmRecvRes pk) s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  have hstep' : apply sk ax (.asmRecvRes pk) s = some s' := hstep
  simp only [apply] at hstep'
  split at hstep'
  case isFalse => simp at hstep'
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph⟩, hpos⟩ := hg
    injection hstep' with hs'
    have hpk2 : 1 ≤ pk.2 := asmKeys_snd_pos hmem
    have hpkmem : pk ∈ sk.asmKeys := List.contains_iff_mem.mp hmem
    have hold := hi.asm pk hpkmem
    simp only [asmLocalOk, hph] at hold
    simp at hold
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk' hpk'
    · by_cases hpkeq : pk' = pk
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
        rw [asmLocalOk_congr sk pk' ha]; exact hi.asm pk' hpk'
    · rw [← hs']; exact hi.top
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = bump s.chan (asmResChan pk) (-1) := by
        rw [← hs']; rfl
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']
        apply sentOf_ext_idx
        · intro pk''
          by_cases hq : pk'' = pk
          · subst hq; simp
          · simp [setAsm_asm_ne _ _ hq]
        all_goals rfl
      by_cases hcc : c = asmResChan pk
      · subst hcc
        have h21 : pk.2 - 1 + 1 = pk.2 := by omega
        have hkey : ((pk.1, pk.2 - 1 + 1) : Party × Nat) = pk := by
          rw [h21]
        by_cases hask : asks pk.1 pk.2 = true
        · have hch : asmResChan pk = Chan.upper pk.1 (pk.2 - 1) := by
            simp [asmResChan, hask]
          have hrecvS : recvdOfO sk ord s (asmResChan pk)
              = (s.asm pk).idx := by
            rw [hch]
            show asmResRecvd s (pk.1, pk.2 - 1 + 1) = _
            rw [hkey]
            simp [asmResRecvd, hph]
          have hrecvS' : recvdOfO sk ord s' (asmResChan pk)
              = (s.asm pk).idx + 1 := by
            rw [← hs', hch]
            show asmResRecvd (setAsm _ pk _) (pk.1, pk.2 - 1 + 1) = _
            rw [hkey]
            simp only [asmResRecvd, setAsm_asm_self]
            split <;> simp
          rw [hchan, hsent, hrecvS', bump_neg_one]
          rw [hrecvS] at heq
          exact ⟨by omega, by omega⟩
        · have hch : asmResChan pk = Chan.lower pk.1 pk.2 := by
            simp [asmResChan, hask]
          have hctm : ((pk.1, pk.2) : Party × Nat) ∈ sk.asmKeys := hpkmem
          have hrecvS : recvdOfO sk ord s (asmResChan pk)
              = (s.asm pk).idx := by
            rw [hch]
            simp [recvdOfO, recvdOf, hctm, asmResRecvd, hph]
          have hrecvS' : recvdOfO sk ord s' (asmResChan pk)
              = (s.asm pk).idx + 1 := by
            rw [← hs', hch]
            simp only [recvdOfO, recvdOf]
            rw [if_pos (by exact hmem)]
            simp only [asmResRecvd, setAsm_asm_self]
            split <;> simp
          rw [hchan, hsent, hrecvS', bump_neg_one]
          rw [hrecvS] at heq
          exact ⟨by omega, by omega⟩
      · have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
          rw [← hs']
          refine Eq.trans
            (recvdOfO_setAsm_frame_res hwf _ pk _ hc hcc ?_) ?_
          · simp [asmLevelRecvd, hold.2]
          · exact recvdOfO_ext (fun _ => rfl) (fun _ => rfl)
              (fun _ => rfl) rfl rfl rfl rfl rfl rfl c
        rw [hchan, hsent, hrecv, bump_ne _ _ hcc]
        exact ⟨heq, hcap⟩

/-- `asmRecvLevel` under the assignment (a shared base arm): occupancy
on `asmLevelChan pk` drops by one exactly as the O consumer count — the
base `asmLevelRecvd`, by `recvdOfO`'s delegation — rises by one (cf.
`preserve_asmRecvLevel`). -/
theorem preserve_asmRecvLevelO (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : applyO sk ax ord (.asmRecvLevel pk) s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  have hstep' : apply sk ax (.asmRecvLevel pk) s = some s' := hstep
  simp only [apply] at hstep'
  split at hstep'
  case isFalse => simp at hstep'
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph⟩, hpos⟩ := hg
    injection hstep' with hs'
    have hpk2 : 1 ≤ pk.2 := asmKeys_snd_pos hmem
    have hpkmem : pk ∈ sk.asmKeys := List.contains_iff_mem.mp hmem
    have hold := hi.asm pk hpkmem
    simp only [asmLocalOk, hph] at hold
    simp at hold
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk' hpk'
    · by_cases hpkeq : pk' = pk
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
        rw [asmLocalOk_congr sk pk' ha]; exact hi.asm pk' hpk'
    · rw [← hs']; exact hi.top
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = bump s.chan (asmLevelChan pk) (-1) := by
        rw [← hs']; rfl
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']
        apply sentOf_ext_idx
        · intro pk''
          by_cases hq : pk'' = pk
          · subst hq; simp
          · simp [setAsm_asm_ne _ _ hq]
        all_goals rfl
      by_cases hcc : c = asmLevelChan pk
      · subst hcc
        have h21 : pk.2 - 1 + 1 = pk.2 := by omega
        have hkey : ((pk.1, pk.2 - 1 + 1) : Party × Nat) = pk := by
          rw [h21]
        have hrecvS : recvdOfO sk ord s (asmLevelChan pk)
            = sk.pendsBefore pk.1 pk.2 (s.asm pk).idx
              + (s.asm pk).got := by
          show recvdOf sk s (Chan.level pk.1 (pk.2 - 1)) = _
          simp only [recvdOf]
          rw [hkey, if_pos (by exact hmem)]
          rfl
        have hrecvS' : recvdOfO sk ord s' (asmLevelChan pk)
            = sk.pendsBefore pk.1 pk.2 (s.asm pk).idx
              + (s.asm pk).got + 1 := by
          rw [← hs']
          show recvdOf sk (setAsm _ pk _) (Chan.level pk.1 (pk.2 - 1)) = _
          simp only [recvdOf]
          rw [hkey, if_pos (by exact hmem)]
          simp only [asmLevelRecvd, setAsm_asm_self]
          omega
        rw [hchan, hsent, hrecvS', bump_neg_one]
        rw [hrecvS] at heq
        exact ⟨by omega, by omega⟩
      · have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
          rw [← hs']
          refine Eq.trans
            (recvdOfO_setAsm_frame_level _ pk _ hcc ?_) ?_
          · simp only [asmResRecvd, setAsm_asm_self]
            split <;> simp [hph]
          · exact recvdOfO_ext (fun _ => rfl) (fun _ => rfl)
              (fun _ => rfl) rfl rfl rfl rfl rfl rfl c
        rw [hchan, hsent, hrecv, bump_ne _ _ hcc]
        exact ⟨heq, hcap⟩

/-- `asmSend` under the assignment (a shared base arm): occupancy on
`sk.asmOutChan pk` rises by one exactly as the cursor advances, and
both O consumer counts at `pk` stay constant — the res prologue moves
into the cursor, the level count telescopes by `pendsBefore_succ`
(cf. `preserve_asmSend`). -/
theorem preserve_asmSendO (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : applyO sk ax ord (.asmSend pk) s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  have hstep' : apply sk ax (.asmSend pk) s = some s' := hstep
  simp only [apply] at hstep'
  split at hstep'
  case isFalse => simp at hstep'
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph⟩, hcaplt⟩ := hg
    injection hstep' with hs'
    have hpk2 : 1 ≤ pk.2 := asmKeys_snd_pos hmem
    have hpkmem : pk ∈ sk.asmKeys := List.contains_iff_mem.mp hmem
    have hold := hi.asm pk hpkmem
    simp only [asmLocalOk, hph] at hold
    simp at hold
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk' hpk'
    · by_cases hpkeq : pk' = pk
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
        rw [asmLocalOk_congr sk pk' ha]; exact hi.asm pk' hpk'
    · rw [← hs']; exact hi.top
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = bump s.chan (sk.asmOutChan pk) 1 := by
        rw [← hs']; rfl
      have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
        rw [← hs']
        refine Eq.trans (recvdOfO_setAsm_of_counts _ pk _ ?_ ?_ c) ?_
        · simp only [asmResRecvd, setAsm_asm_self]
          by_cases hlt2 : (s.asm pk).idx + 1
              < (sk.asmResList pk.1 pk.2).length
          · simp [hlt2, hph]
          · simp [hlt2, hph]
        · simp only [asmLevelRecvd, setAsm_asm_self]
          rw [pendsBefore_succ sk pk.1 pk.2 (s.asm pk).idx hold.1]
          omega
        · exact recvdOfO_ext (fun _ => rfl) (fun _ => rfl)
            (fun _ => rfl) rfl rfl rfl rfl rfl rfl c
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
          rw [hchan, bump_one, hrecv, hsentS']
          rw [hsentS] at heq
          exact ⟨by omega, by omega⟩
        · by_cases h2 : (pk.1 == Party.R && pk.2 == sk.rootH - 1) = true
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
              show asmOutSent (setAsm _ pk _) (Party.R, sk.rootH - 1) = _
              rw [← hpkR]
              simp [asmOutSent]
            rw [hchan, bump_one, hrecv, hsentS']
            rw [hsentS] at heq
            exact ⟨by omega, by omega⟩
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
              show sentOf sk (setAsm _ pk _) (Chan.level pk.1 pk.2) = _
              simp only [sentOf]
              rw [if_neg hnot0, if_pos hcond]
              show asmOutSent (setAsm _ pk _) (pk.1, pk.2) = _
              rw [show ((pk.1, pk.2) : Party × Nat) = pk from rfl]
              simp [asmOutSent]
            rw [hchan, bump_one, hrecv, hsentS']
            rw [hsentS] at heq
            exact ⟨by omega, by omega⟩
      · have hsent : sentOf sk s' c = sentOf sk s c := by
          rw [← hs']
          refine Eq.trans (sentOf_setAsm_frame _ pk _ hcc) ?_
          exact sentOf_ext sk (fun _ => rfl) (fun _ => rfl) (fun _ => rfl)
            (fun _ => rfl) (fun _ => rfl) (fun _ => rfl) (fun _ => rfl)
            rfl rfl rfl rfl rfl rfl c
        rw [hchan, hsent, hrecv, bump_ne _ _ hcc]
        exact ⟨heq, hcap⟩

end StreamingMirror.Ord
