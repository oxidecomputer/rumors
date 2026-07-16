/-
Preservation for the assemblers. `setAsm` parallels `setWalk`
(Preserve/Walk.lean): local consistency at the touched key re-derives
from the guard, every other assembler frames by `setAsm_asm_ne`, and
flow is pointwise per channel with bespoke frame lemmas ("a `setAsm`
at `pk` is invisible to channels whose asm key is not `pk`"). The
assembler-specific subtleties: `asmResChan` is one of two channels
(`asks` decides), and the OTHER res-side channel at the same key is
kept out of `allChans` by walk/asm parity (initiator walks sit at odd
heights, responder at even — the membership-relativized frame, as in
Proofs/Wiring.lean); `asmOutChan` is a three-way split (the root
singletons vs `level`); and `asmSend`'s level-count invariance is the
prefix-sum telescope `pendsBefore (idx+1) = pendsBefore idx + pendAt
idx`.
-/
import StreamingMirror.Proofs.Lemmas

namespace StreamingMirror.Model

variable {sk : Skel} {ax : AxMode} {s s' : State}

/-- Assembler keys sit at height ≥ 1 (`asmKeys` maps `j ↦ j + 1` on
both sides): the fact that keeps `pk.2 - 1 + 1 = pk.2` rewrites sound
on an assembler's input channels. -/
theorem asmKeys_snd_pos {pk : Party × Nat}
    (h : sk.asmKeys.contains pk = true) : 1 ≤ pk.2 := by
  rw [List.contains_iff_mem] at h
  simp only [Skel.asmKeys, List.mem_append, List.mem_map,
    List.mem_range] at h
  rcases h with ⟨j, _, heq⟩ | ⟨j, _, heq⟩ <;>
    · rw [← heq]; exact Nat.le_add_left 1 j

/-- `sentOf_ext` weakened at the assembler slot: producer counts read
an assembler only through its cursor, so any asm update that preserves
every `idx` is invisible to every producer count. -/
theorem sentOf_ext_idx (sk : Skel) {s s' : State}
    (hidx : ∀ pk, (s'.asm pk).idx = (s.asm pk).idx)
    (hwalk : s'.walk = s.walk)
    (h1 : s'.iopenWire = s.iopenWire) (h2 : s'.iopenQuery = s.iopenQuery)
    (h3 : s'.ropenWire = s.ropenWire) (h4 : s'.ropenRes = s.ropenRes)
    (h5 : s'.ropenQ = s.ropenQ) (h6 : s'.absorbIdx = s.absorbIdx) :
    ∀ c, sentOf sk s' c = sentOf sk s c := by
  intro c
  cases c <;>
    simp [sentOf, wkWireSent, wkResSent, wkQSentTot, wkParentSent,
      wkWireCount, wkResCount, wkQSum, asmOutSent,
      hidx, hwalk, h1, h2, h3, h4, h5, h6]

/-- An asm update at `pk` that preserves both of `pk`'s consumer counts
is invisible to every consumer count: the flow frame for `asmSend` and
`asmClose`, where the touched channel is on the producer side (or
nothing moves at all). -/
theorem recvdOf_setAsm_of_counts (sk : Skel) (s : State)
    (pk : Party × Nat) (a' : AsmSt)
    (hres : asmResRecvd (setAsm s pk a') pk = asmResRecvd s pk)
    (hlev : asmLevelRecvd sk (setAsm s pk a') pk = asmLevelRecvd sk s pk)
    (c : Chan) :
    recvdOf sk (setAsm s pk a') c = recvdOf sk s c := by
  cases c with
  | upper p h =>
      by_cases hkeq : ((p, h + 1) : Party × Nat) = pk
      · show asmResRecvd (setAsm s pk a') (p, h + 1)
          = asmResRecvd s (p, h + 1)
        rw [hkeq]; exact hres
      · show asmResRecvd (setAsm s pk a') (p, h + 1)
          = asmResRecvd s (p, h + 1)
        simp [asmResRecvd, setAsm_asm_ne s _ hkeq]
  | lower p h =>
      by_cases hkeq : ((p, h) : Party × Nat) = pk
      · simp only [recvdOf]
        rw [hkeq]
        rw [hres]
      · simp only [recvdOf]
        rw [asmResRecvd, asmResRecvd, setAsm_asm_ne s _ hkeq]
  | level p j =>
      by_cases hkeq : ((p, j + 1) : Party × Nat) = pk
      · simp only [recvdOf]
        rw [hkeq]
        rw [hlev]
      · simp only [recvdOf]
        rw [asmLevelRecvd, asmLevelRecvd, setAsm_asm_ne s _ hkeq]
  | wire p h => rfl
  | asked p h => rfl
  | leafRequests => rfl
  | rootret => rfl
  | rootrets => rfl
  | rootres => rfl

/-- Walk keys have party-determined parity (`rootH` is even): initiator
stages sit at odd heights, responder at even. This is what keeps the
"wrong-side" res channel of an assembler out of `allChans`. -/
theorem walkKeys_parity (hwf : sk.wellFormed = true) {p : Party} {h : Nat}
    (hmem : (p, h) ∈ sk.walkKeys) :
    (p = Party.I ∧ h % 2 = 1) ∨ (p = Party.R ∧ h % 2 = 0) := by
  have hev := (wf_rootH hwf).1
  have hge := (wf_rootH hwf).2
  simp only [Skel.walkKeys, List.mem_append, List.mem_map,
    List.mem_range] at hmem
  rcases hmem with ⟨k, hk, heq⟩ | ⟨k, hk, heq⟩ <;> injection heq with hp hh
  · subst hp; subst hh
    exact Or.inl ⟨rfl, by omega⟩
  · subst hp; subst hh
    exact Or.inr ⟨rfl, by omega⟩

/-- An `upper` channel of `allChans` is some walk's parent output. -/
theorem mem_allChans_upper {p : Party} {h : Nat}
    (hc : Chan.upper p h ∈ allChans sk) : (p, h) ∈ sk.walkKeys := by
  simp only [allChans, wireOut, askedIn, upperOut, lowerOut,
    List.mem_append, List.mem_flatMap, List.mem_map, List.mem_cons,
    List.not_mem_nil, or_false] at hc
  rcases hc with (⟨pk', hpk', hor⟩ | ⟨pk', _, heq⟩) | hor
  · rcases hor with h1 | h1 | h1 | h1
    · exact Chan.noConfusion h1
    · exact Chan.noConfusion h1
    · injection h1 with hp hh
      subst hp; subst hh
      simpa using hpk'
    · exact Chan.noConfusion h1
  · exact Chan.noConfusion heq
  · rcases hor with h1 | h1 | h1 | h1 | h1 | h1 | h1 <;>
      exact Chan.noConfusion h1

/-- A `lower` channel of `allChans` is some walk's res output. -/
theorem mem_allChans_lower {p : Party} {h : Nat}
    (hc : Chan.lower p h ∈ allChans sk) : (p, h) ∈ sk.walkKeys := by
  simp only [allChans, wireOut, askedIn, upperOut, lowerOut,
    List.mem_append, List.mem_flatMap, List.mem_map, List.mem_cons,
    List.not_mem_nil, or_false] at hc
  rcases hc with (⟨pk', hpk', hor⟩ | ⟨pk', _, heq⟩) | hor
  · rcases hor with h1 | h1 | h1 | h1
    · exact Chan.noConfusion h1
    · exact Chan.noConfusion h1
    · exact Chan.noConfusion h1
    · injection h1 with hp hh
      subst hp; subst hh
      simpa using hpk'
  · exact Chan.noConfusion heq
  · rcases hor with h1 | h1 | h1 | h1 | h1 | h1 | h1 <;>
      exact Chan.noConfusion h1

/-- Flow frame for `asmRecvRes`: away from `asmResChan pk`, a `setAsm`
at `pk` that preserves the level count is invisible to every `allChans`
consumer count. Membership is load-bearing: the res channel on the
side `pk` does NOT ask reads `asmResRecvd pk` too, but walk/asm parity
keeps it out of `allChans`. -/
theorem recvdOf_setAsm_frame_res (hwf : sk.wellFormed = true)
    (s : State) (pk : Party × Nat) (a' : AsmSt) {c : Chan}
    (hc : c ∈ allChans sk) (hne : c ≠ asmResChan pk)
    (hlev : asmLevelRecvd sk (setAsm s pk a') pk = asmLevelRecvd sk s pk) :
    recvdOf sk (setAsm s pk a') c = recvdOf sk s c := by
  cases c with
  | upper p h =>
      by_cases hkeq : ((p, h + 1) : Party × Nat) = pk
      · exfalso
        by_cases hask : asks p (h + 1) = true
        · apply hne
          rw [← hkeq]
          show Chan.upper p h = asmResChan (p, h + 1)
          simp [asmResChan, hask]
        · have hwk : (p, h) ∈ sk.walkKeys := mem_allChans_upper hc
          rcases walkKeys_parity hwf hwk with ⟨rfl, hpar⟩ | ⟨rfl, hpar⟩ <;>
            · apply hask
              simp only [asks, beq_iff_eq]
              omega
      · show asmResRecvd (setAsm s pk a') (p, h + 1)
          = asmResRecvd s (p, h + 1)
        simp [asmResRecvd, setAsm_asm_ne s _ hkeq]
  | lower p h =>
      by_cases hkeq : ((p, h) : Party × Nat) = pk
      · exfalso
        by_cases hask : asks p h = true
        · have hwk : (p, h) ∈ sk.walkKeys := mem_allChans_lower hc
          rcases walkKeys_parity hwf hwk with ⟨rfl, hpar⟩ | ⟨rfl, hpar⟩ <;>
            · simp only [asks, beq_iff_eq] at hask
              omega
        · apply hne
          rw [← hkeq]
          show Chan.lower p h = asmResChan (p, h)
          simp [asmResChan, hask]
      · simp only [recvdOf]
        rw [asmResRecvd, asmResRecvd, setAsm_asm_ne s _ hkeq]
  | level p j =>
      by_cases hkeq : ((p, j + 1) : Party × Nat) = pk
      · simp only [recvdOf]
        rw [hkeq, hlev]
      · simp only [recvdOf]
        rw [asmLevelRecvd, asmLevelRecvd, setAsm_asm_ne s _ hkeq]
  | wire p h => rfl
  | asked p h => rfl
  | leafRequests => rfl
  | rootret => rfl
  | rootrets => rfl
  | rootres => rfl

/-- Flow frame for `asmRecvLevel`: away from `asmLevelChan pk`, a
`setAsm` at `pk` that preserves the res count is invisible to every
consumer count (no membership needed: only one channel reads the level
count at `pk`). -/
theorem recvdOf_setAsm_frame_level (s : State)
    (pk : Party × Nat) (a' : AsmSt) {c : Chan}
    (hne : c ≠ asmLevelChan pk)
    (hres : asmResRecvd (setAsm s pk a') pk = asmResRecvd s pk) :
    recvdOf sk (setAsm s pk a') c = recvdOf sk s c := by
  cases c with
  | upper p h =>
      by_cases hkeq : ((p, h + 1) : Party × Nat) = pk
      · show asmResRecvd (setAsm s pk a') (p, h + 1)
          = asmResRecvd s (p, h + 1)
        rw [hkeq, hres]
      · show asmResRecvd (setAsm s pk a') (p, h + 1)
          = asmResRecvd s (p, h + 1)
        simp [asmResRecvd, setAsm_asm_ne s _ hkeq]
  | lower p h =>
      by_cases hkeq : ((p, h) : Party × Nat) = pk
      · simp only [recvdOf]
        rw [hkeq, hres]
      · simp only [recvdOf]
        rw [asmResRecvd, asmResRecvd, setAsm_asm_ne s _ hkeq]
  | level p j =>
      by_cases hkeq : ((p, j + 1) : Party × Nat) = pk
      · exfalso
        apply hne
        rw [← hkeq]
        show Chan.level p j = asmLevelChan (p, j + 1)
        simp [asmLevelChan]
      · simp only [recvdOf]
        rw [asmLevelRecvd, asmLevelRecvd, setAsm_asm_ne s _ hkeq]
  | wire p h => rfl
  | asked p h => rfl
  | leafRequests => rfl
  | rootret => rfl
  | rootrets => rfl
  | rootres => rfl

/-- The prefix-sum telescope: one more completed resolution adds its
own pending count. This is `asmSend`'s level-count invariance — the
new cursor's empty ledger (`got := 0`) absorbs exactly the finished
resolution's `pendAt`. -/
theorem pendsBefore_succ (sk : Skel) (p : Party) (j i : Nat)
    (hi : i < (sk.asmResList p j).length) :
    sk.pendsBefore p j (i + 1)
      = sk.pendsBefore p j i + sk.pendAt p j i := by
  unfold Skel.pendsBefore Skel.pendAt
  rw [List.take_succ_eq_append_getElem hi, List.foldl_append,
    List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hi]
  simp

/-- Producer-side flow frame: a `setAsm` at `pk` is invisible to
`sentOf` away from `sk.asmOutChan pk`. No membership needed: every
producer count that reads asm state at `pk` reconstructs `asmOutChan
pk` (the root singletons read fixed keys; the `level` branch is dead
at root-out keys). -/
theorem sentOf_setAsm_frame (s : State) (pk : Party × Nat) (a' : AsmSt)
    {c : Chan} (hne : c ≠ sk.asmOutChan pk) :
    sentOf sk (setAsm s pk a') c = sentOf sk s c := by
  cases c with
  | level p j =>
      by_cases hkeq : ((p, j) : Party × Nat) = pk
      · by_cases h1 : (p == Party.I && j == sk.rootH) = true
        · have hroot : isRootOutKey sk (p, j) = true := by
            simp only [isRootOutKey]
            simp [h1]
          simp [sentOf, hroot, setAsm]
        · by_cases h2 : (p == Party.R && j == sk.rootH - 1) = true
          · have hroot : isRootOutKey sk (p, j) = true := by
              simp only [isRootOutKey]
              simp [h2]
            simp [sentOf, hroot, setAsm]
          · exfalso
            apply hne
            rw [← hkeq]
            show Chan.level p j = sk.asmOutChan (p, j)
            unfold Skel.asmOutChan
            rw [if_neg h1, if_neg h2]
      · simp [sentOf, asmOutSent, setAsm, hkeq]
  | rootret =>
      have hkne : ((Party.I, sk.rootH) : Party × Nat) ≠ pk := by
        intro hk
        apply hne
        rw [← hk]
        show Chan.rootret = sk.asmOutChan (Party.I, sk.rootH)
        simp [Skel.asmOutChan]
      show asmOutSent (setAsm s pk a') (Party.I, sk.rootH)
          = asmOutSent s (Party.I, sk.rootH)
      simp [asmOutSent, setAsm_asm_ne s _ hkne]
  | rootrets =>
      have hkne : ((Party.R, sk.rootH - 1) : Party × Nat) ≠ pk := by
        intro hk
        apply hne
        rw [← hk]
        show Chan.rootrets = sk.asmOutChan (Party.R, sk.rootH - 1)
        simp [Skel.asmOutChan]
      show asmOutSent (setAsm s pk a') (Party.R, sk.rootH - 1)
          = asmOutSent s (Party.R, sk.rootH - 1)
      simp [asmOutSent, setAsm_asm_ne s _ hkne]
  | wire p h => rfl
  | asked p h => rfl
  | leafRequests => rfl
  | upper p h => rfl
  | lower p h => rfl
  | rootres => rfl

/-- `asmClose` moves phase 3 → 4 and nothing else: both consumer counts
read `phase == 1 || phase == 2` (false on both sides) or only the
cursor, so every count is unchanged and flow frames completely. -/
theorem preserve_asmClose (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : apply sk ax (.asmClose pk) s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨⟨hmem, hph⟩, _hpd⟩, _hch0⟩ := hg
    injection hstep with hs'
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
      have hrecv : recvdOf sk s' c = recvdOf sk s c := by
        rw [← hs']
        apply recvdOf_setAsm_of_counts
        · simp [asmResRecvd, hph]
        · simp [asmLevelRecvd]
      rw [hchan, hsent, hrecv]
      exact ⟨heq, hcap⟩

/-- `asmRecvRes` consumes one resolution: occupancy on `asmResChan pk`
drops by one exactly as `asmResRecvd pk` rises by one (phase 0 → 1/2
flips the prologue indicator), and `asmLocalOk` at `pk` re-establishes
from the `pendAt` branch condition. -/
theorem preserve_asmRecvRes (hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : apply sk ax (.asmRecvRes pk) s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph⟩, hpos⟩ := hg
    injection hstep with hs'
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
          rw [hchan, hsent, hrecvS', bump_neg_one]
          rw [hrecvS] at heq
          exact ⟨by omega, by omega⟩
        · have hch : asmResChan pk = Chan.lower pk.1 pk.2 := by
            simp [asmResChan, hask]
          have hctm : ((pk.1, pk.2) : Party × Nat) ∈ sk.asmKeys := hpkmem
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
          rw [hchan, hsent, hrecvS', bump_neg_one]
          rw [hrecvS] at heq
          exact ⟨by omega, by omega⟩
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          refine Eq.trans
            (recvdOf_setAsm_frame_res hwf _ pk _ hc hcc ?_) ?_
          · simp [asmLevelRecvd, hold.2]
          · exact recvdOf_ext sk (fun _ => rfl) (fun _ => rfl)
              (fun _ => rfl) rfl rfl rfl rfl rfl rfl c
        rw [hchan, hsent, hrecv, bump_ne _ _ hcc]
        exact ⟨heq, hcap⟩

/-- `asmRecvLevel` consumes one level return: occupancy on
`asmLevelChan pk` drops by one exactly as `asmLevelRecvd pk` rises by
one (`got + 1`), and the phase-2 fullness conjunct is the branch
condition verbatim. -/
theorem preserve_asmRecvLevel (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : apply sk ax (.asmRecvLevel pk) s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph⟩, hpos⟩ := hg
    injection hstep with hs'
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
          show recvdOf sk (setAsm _ pk _) (Chan.level pk.1 (pk.2 - 1)) = _
          simp only [recvdOf]
          rw [hkey, if_pos (by exact hmem)]
          simp only [asmLevelRecvd, setAsm_asm_self]
          omega
        rw [hchan, hsent, hrecvS', bump_neg_one]
        rw [hrecvS] at heq
        exact ⟨by omega, by omega⟩
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          refine Eq.trans
            (recvdOf_setAsm_frame_level _ pk _ hcc ?_) ?_
          · simp only [asmResRecvd, setAsm_asm_self]
            split <;> simp [hph]
          · exact recvdOf_ext sk (fun _ => rfl) (fun _ => rfl)
              (fun _ => rfl) rfl rfl rfl rfl rfl rfl c
        rw [hchan, hsent, hrecv, bump_ne _ _ hcc]
        exact ⟨heq, hcap⟩

/-- `asmSend` publishes one assembled resolution: occupancy on
`sk.asmOutChan pk` rises by one exactly as `asmOutSent pk` does (the
cursor advances). Both consumer counts at `pk` stay constant: the res
prologue indicator moves from the count into the cursor (phase 2 → 0/3),
and the level count telescopes (`pendsBefore_succ` plus the phase-2
fullness conjunct `got = pendAt`). -/
theorem preserve_asmSend (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : apply sk ax (.asmSend pk) s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph⟩, hcaplt⟩ := hg
    injection hstep with hs'
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
      have hrecv : recvdOf sk s' c = recvdOf sk s c := by
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

end StreamingMirror.Model
