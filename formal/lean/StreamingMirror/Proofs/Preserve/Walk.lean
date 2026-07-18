/-
Preservation for the walk stages. `walkCommit` is where "mirror the
guards exactly" is cashed in: three of the four obligation arms of
`wkLocalOk`'s committed-match are the `wkChoosable` guard verbatim, and
the fourth (`.wire i`) is the frontier-counting argument
(`length_filter_of_frontier`) — the only place preservation needs
`wellFormed` (to fan-bound the child index).
-/
import StreamingMirror.Proofs.Lemmas
import StreamingMirror.Proofs.Wiring

namespace StreamingMirror.Model

variable {sk : Skel} {ax : AxMode} {s s' : State}

-- ================================================= flow dispatch helpers

/-- Sends frame for a walk update whose four producer counts at `pk` are
unchanged: dispatch each of `pk`'s output channels to its count, frame
the rest. The `askedOut` case falls back to congruence for the leaf
responder stage `(R, 0)` (whose `askedOut` aliases `leafRequests`, owned
by `(I, 1)`). -/
theorem sentOf_setWalk_same (hwf : sk.wellFormed = true)
    (s : State) (pk : Party × Nat) (ws' : WalkSt)
    (hmem : pk ∈ sk.walkKeys)
    (hW : wkWireSent sk (setWalk s pk ws') pk = wkWireSent sk s pk)
    (hR : wkResSent sk (setWalk s pk ws') pk = wkResSent sk s pk)
    (hQ : wkQSentTot sk (setWalk s pk ws') pk = wkQSentTot sk s pk)
    (hP : wkParentSent (setWalk s pk ws') pk = wkParentSent s pk)
    {c : Chan} (hc : c ∈ allChans sk) :
    sentOf sk (setWalk s pk ws') c = sentOf sk s c := by
  by_cases h1 : c = wireOut pk
  · subst h1; rw [sentOf_wireOut hmem, sentOf_wireOut hmem]; exact hW
  by_cases h2 : c = lowerOut pk
  · subst h2; rw [sentOf_lowerOut, sentOf_lowerOut]; exact hR
  by_cases h4 : c = upperOut pk
  · subst h4; rw [sentOf_upperOut, sentOf_upperOut]; exact hP
  by_cases h3 : c = askedOut pk
  · subst h3
    by_cases hp1 : 1 ≤ pk.2
    · rw [sentOf_askedOut hwf hmem hp1, sentOf_askedOut hwf hmem hp1]
      exact hQ
    · have h0 : pk.2 = 0 := by omega
      have hI1 : (Party.I, 1) ≠ pk := by
        intro h; rw [← h] at h0; simp at h0
      rw [show askedOut pk = Chan.leafRequests by simp [askedOut, h0]]
      simp [sentOf, wkQSentTot, wkQSum, setWalk_walk_ne s _ hI1]
  · exact sentOf_setWalk_frame s pk ws' hc h1 h2 h3 h4

/-- Receives frame for a walk update whose two consumer counts at `pk`
are unchanged. -/
theorem recvdOf_setWalk_same (hwf : sk.wellFormed = true)
    (s : State) (pk : Party × Nat) (ws' : WalkSt)
    (hmem : pk ∈ sk.walkKeys)
    (hWr : wkWireRecvd sk (setWalk s pk ws') pk = wkWireRecvd sk s pk)
    (hAr : wkAskedRecvd sk (setWalk s pk ws') pk = wkAskedRecvd sk s pk)
    {c : Chan} (hc : c ∈ allChans sk) :
    recvdOf sk (setWalk s pk ws') c = recvdOf sk s c := by
  by_cases h5 : c = wireIn pk
  · subst h5; rw [recvdOf_wireIn hmem, recvdOf_wireIn hmem]; exact hWr
  by_cases h6 : c = askedIn pk
  · subst h6; rw [recvdOf_askedIn, recvdOf_askedIn]; exact hAr
  · exact recvdOf_setWalk_frame hwf s pk ws' hc h5 h6

/-- `walkCommit` records an obligation choice; nothing observable to
any count changes, and the committed arm holds because it repeats the
guard (plus the wire-frontier count). -/
theorem preserve_walkCommit (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (o : Oblig)
    (hstep : apply sk ax (.walkCommit pk o) s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
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
      refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_, fun c hc => ?_⟩
      · by_cases hpkeq : pk' = pk
        · subst hpkeq
          have hwalk : s'.walk pk' = { s.walk pk' with committed := some o } := by
            rw [← hs']; simp
          have hcount : wkWireCount sk s' pk' = wkWireCount sk s pk' := by
            simp [wkWireCount, hwalk]
          have hsc : scopeComplete sk pk'.2
              { s.walk pk' with committed := some o }
              = scopeComplete sk pk'.2 (s.walk pk') := rfl
          have hwk := hi.wk pk' hpk'
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
                Bool.not_eq_true', List.all_eq_true, List.mem_range] at hch
              obtain ⟨⟨⟨⟨hin, hfront⟩, hlow⟩, hd4⟩, hd5⟩ := hch
              simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq]
              have hn : sk.nChildren pk'.snd
                  (sk.stageScope pk'.snd (s.walk pk').scope) ≤ sk.fan :=
                nChildren_le_fan hwf hA
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
          exact hi.wk pk' hpk'
      · rw [← hs']; exact hi.asm pk' hpk'
      · rw [← hs']; exact hi.top
      · rw [← hs',
          sentOf_setWalk_committed sk s pk (some o) c,
          recvdOf_setWalk_committed sk s pk (some o) c]
        exact hi.flow c hc

/-- `walkCloseWire` observes end-of-stream: phase 3 → 4, nothing else.
Every count is phase-insensitive across 3 → 4 (the receive counts have
already saturated at `stageLen`), so the whole invariant frames. -/
theorem preserve_walkCloseWire (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : apply sk ax (.walkCloseWire pk) s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨⟨hmem, hph3⟩, _hpd⟩, _hz⟩ := hg
    have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
    injection hstep with hs'
    have hwalk : s'.walk pk = { s.walk pk with phase := 4 } := by
      rw [← hs']; simp
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_, fun c hc => ?_⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        have hwk := hi.wk pk' hpk'
        simp only [wkLocalOk, hwalk] at hwk ⊢
        rw [hph3] at hwk
        simp at hwk ⊢
        exact hwk
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']; exact setWalk_walk_ne s _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hi.wk pk' hpk'
    · rw [← hs']; exact hi.asm pk' hpk'
    · rw [← hs']; exact hi.top
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = s.chan := by rw [← hs']; rfl
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']
        exact sentOf_setWalk_same _hwf s pk { s.walk pk with phase := 4 } hmem'
          (by simp [wkWireSent, wkWireCount])
          (by simp [wkResSent, wkResCount])
          (by simp [wkQSentTot, wkQSum])
          (by simp [wkParentSent, hph3])
          hc
      have hrecv : recvdOf sk s' c = recvdOf sk s c := by
        rw [← hs']
        exact recvdOf_setWalk_same _hwf s pk { s.walk pk with phase := 4 } hmem'
          (by simp [wkWireRecvd, hph3])
          (by simp [wkAskedRecvd, hph3])
          hc
      rw [hchan, hsent, hrecv]
      exact ⟨heq, hcap⟩

/-- `walkCloseAsked`: phase 4 → 5; same shape as `walkCloseWire`. -/
theorem preserve_walkCloseAsked (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : apply sk ax (.walkCloseAsked pk) s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨⟨hmem, hph4⟩, _hpd⟩, _hz⟩ := hg
    have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
    injection hstep with hs'
    have hwalk : s'.walk pk = { s.walk pk with phase := 5 } := by
      rw [← hs']; simp
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_, fun c hc => ?_⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        have hwk := hi.wk pk' hpk'
        simp only [wkLocalOk, hwalk] at hwk ⊢
        rw [hph4] at hwk
        simp at hwk ⊢
        exact hwk
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']; exact setWalk_walk_ne s _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hi.wk pk' hpk'
    · rw [← hs']; exact hi.asm pk' hpk'
    · rw [← hs']; exact hi.top
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = s.chan := by rw [← hs']; rfl
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']
        exact sentOf_setWalk_same _hwf s pk { s.walk pk with phase := 5 } hmem'
          (by simp [wkWireSent, wkWireCount])
          (by simp [wkResSent, wkResCount])
          (by simp [wkQSentTot, wkQSum])
          (by simp [wkParentSent, hph4])
          hc
      have hrecv : recvdOf sk s' c = recvdOf sk s c := by
        rw [← hs']
        exact recvdOf_setWalk_same _hwf s pk { s.walk pk with phase := 5 } hmem'
          (by simp [wkWireRecvd, hph4])
          (by simp [wkAskedRecvd, hph4])
          hc
      rw [hchan, hsent, hrecv]
      exact ⟨heq, hcap⟩

/-- `walkRecvWire`: the prologue wire receive, phase 0 → 1. Occupancy
of `wireIn pk` drops by one exactly as `wkWireRecvd` rises by one. -/
theorem preserve_walkRecvWire (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : apply sk ax (.walkRecvWire pk) s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph0⟩, hpos⟩ := hg
    have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
    injection hstep with hs'
    have hwalk : s'.walk pk = { s.walk pk with phase := 1, committed := none } := by
      rw [← hs']; simp
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_, fun c hc => ?_⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        have hwk := hi.wk pk' hpk'
        simp only [wkLocalOk, hwalk] at hwk ⊢
        rw [hph0] at hwk
        simp at hwk ⊢
        exact ⟨hwk.1, hwk.2.1⟩
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']; exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hi.wk pk' hpk'
    · rw [← hs']; exact hi.asm pk' hpk'
    · rw [← hs']; exact hi.top
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = bump s.chan (wireIn pk) (-1) := by
        rw [← hs']; rfl
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']
        exact sentOf_setWalk_same _hwf _ pk
          { s.walk pk with phase := 1, committed := none } hmem'
          (by simp [wkWireSent, wkWireCount])
          (by simp [wkResSent, wkResCount])
          (by simp [wkQSentTot, wkQSum])
          (by simp [wkParentSent, hph0])
          hc
      by_cases h5 : c = wireIn pk
      · subst h5
        have hr' : recvdOf sk s' (wireIn pk) = wkWireRecvd sk s pk + 1 := by
          rw [← hs', recvdOf_wireIn hmem']
          simp [wkWireRecvd, hph0]
        have hr0 : recvdOf sk s (wireIn pk) = wkWireRecvd sk s pk :=
          recvdOf_wireIn hmem'
        rw [hchan, hsent, hr', bump_neg_one]
        rw [hr0] at heq
        exact ⟨by omega, by omega⟩
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          by_cases h6 : c = askedIn pk
          · subst h6
            rw [← hs', recvdOf_askedIn, recvdOf_askedIn]
            simp [wkAskedRecvd, hph0]
          · rw [← hs']
            exact recvdOf_setWalk_frame _hwf _ pk _ hc h5 h6
        rw [hchan, hsent, hrecv, bump_ne _ _ h5]
        exact ⟨heq, hcap⟩

/-- `walkRecvAsked`: the prologue query receive, phase 1 → 2. The
embedded `normWalk` is provably the identity here: a freshly-phase-2
walk has empty machinery (phase-1 invariant), so `scopeComplete` is
false. Occupancy of `askedIn pk` drops as `wkAskedRecvd` rises. -/
theorem preserve_walkRecvAsked (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : apply sk ax (.walkRecvAsked pk) s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph1⟩, hpos⟩ := hg
    have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
    injection hstep with hs'
    -- phase-1 facts from the invariant: cursor in range, machinery empty
    have hwk := hi.wk pk hmem'
    simp only [wkLocalOk] at hwk
    rw [hph1] at hwk
    simp at hwk
    obtain ⟨hslt, ⟨hledger, hpd⟩, hcm⟩ := hwk
    -- normWalk is the identity on the freshly-phase-2 walk
    have hnlt : ¬ (s.walk pk).scope ≥ sk.stageLen pk.2 := by omega
    have hscF : scopeComplete sk pk.2
        { s.walk pk with phase := 2, committed := none } = false := by
      simp [scopeComplete, hnlt, hpd]
    have hnw : normWalk sk pk.2 { s.walk pk with phase := 2, committed := none }
        = { s.walk pk with phase := 2, committed := none } := by
      simp [normWalk, hscF]
    have hwalk : s'.walk pk = { s.walk pk with phase := 2, committed := none } := by
      rw [← hs']; simp [hnw]
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_, fun c hc => ?_⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        simp only [wkLocalOk, hwalk, hscF]
        simp [hslt]
        intro j hj
        simp [hledger j hj]
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']; exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hi.wk pk' hpk'
    · rw [← hs']; exact hi.asm pk' hpk'
    · rw [← hs']; exact hi.top
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = bump s.chan (askedIn pk) (-1) := by
        rw [← hs']; rfl
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']
        rw [show setWalk { s with chan := bump s.chan (askedIn pk) (-1) } pk
            (normWalk sk pk.2 { s.walk pk with phase := 2, committed := none })
            = setWalk { s with chan := bump s.chan (askedIn pk) (-1) } pk
              { s.walk pk with phase := 2, committed := none } from by rw [hnw]]
        exact sentOf_setWalk_same _hwf _ pk
          { s.walk pk with phase := 2, committed := none } hmem'
          (by simp [wkWireSent, wkWireCount])
          (by simp [wkResSent, wkResCount])
          (by simp [wkQSentTot, wkQSum])
          (by simp [wkParentSent, hph1, hpd])
          hc
      by_cases h6 : c = askedIn pk
      · subst h6
        have hr' : recvdOf sk s' (askedIn pk) = wkAskedRecvd sk s pk + 1 := by
          rw [← hs', recvdOf_askedIn]
          simp [wkAskedRecvd, hnw, hph1]
        have hr0 : recvdOf sk s (askedIn pk) = wkAskedRecvd sk s pk :=
          recvdOf_askedIn
        rw [hchan, hsent, hr', bump_neg_one]
        rw [hr0] at heq
        exact ⟨by omega, by omega⟩
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          by_cases h5 : c = wireIn pk
          · subst h5
            rw [← hs', recvdOf_wireIn hmem', recvdOf_wireIn hmem']
            simp [wkWireRecvd, hnw, hph1]
          · rw [← hs']
            exact recvdOf_setWalk_frame _hwf _ pk _ hc h5 h6
        rw [hchan, hsent, hrecv, bump_ne _ _ h6]
        exact ⟨heq, hcap⟩

end StreamingMirror.Model
