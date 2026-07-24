/-
O preservation for the walk stages: the four order-dispatched arms
(`walkRecvWire`/`walkRecvAsked`/`walkCloseWire`/`walkCloseAsked`) and
the shared `walkCommit`.

This is the campaign's count-coupling probe (PROGRESS.md §13's risk
register): each receive arm case-splits on the walk's assignment, and
each branch is one of the base file's two prologue shapes — a
reply-first branch re-derives its own base body over `recvdOfO`, a
query-first branch re-derives the OTHER receive's base body (the
phase 0→1 shape for the first receive, the normWalk phase 1→2 shape
for the second), with the rising count routed to the branch's channel.
The close arms share one phase-bump core (every count is saturated at
phase ≥ 3 in both orders).

Chain (ord, stage B): the walk-family preservation cases, consumed by
Ord/Preserve.lean. Base mirror: Proofs/Preserve/Walk.lean. Map:
PROGRESS.md §13.
-/
import StreamingMirror.Proofs.Preserve.Walk
import StreamingMirror.Ord.Wiring

namespace StreamingMirror.Ord

open Model

variable {sk : Skel} {ax : AxMode} {ord : OrdMap} {s s' : State}

-- ================================================== the close-arm core

/-- The first end-of-stream wait, either order, either channel: phase
3 → 4 with nothing else moving. Every count is phase-insensitive
across 3 → 4 (receive counts saturated at `stageLen`, `wkParentSent`
reads `phase == 2`), so the whole invariant frames. -/
private theorem preserve_close3 (hwf : sk.wellFormed = true)
    {pk : Party × Nat}
    (hmem : pk ∈ sk.walkKeys) (hph : (s.walk pk).phase = 3)
    (hs' : s' = setWalk s pk { s.walk pk with phase := 4 })
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  have hwalk : s'.walk pk = { s.walk pk with phase := 4 } := by
    rw [hs']; simp
  refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_, fun c hc => ?_⟩
  · by_cases hpkeq : pk' = pk
    · subst hpkeq
      have hwk := hi.wk pk' hpk'
      simp only [wkLocalOk, hwalk] at hwk ⊢
      rw [hph] at hwk
      simp at hwk ⊢
      exact hwk
    · have hw : s'.walk pk' = s.walk pk' := by
        rw [hs']; exact setWalk_walk_ne s _ hpkeq
      rw [wkLocalOk_congr sk ax pk' hw]
      exact hi.wk pk' hpk'
  · rw [hs']; exact hi.asm pk' hpk'
  · rw [hs']; exact hi.top
  · obtain ⟨heq, hcap⟩ := hi.flow c hc
    have hchan : s'.chan = s.chan := by rw [hs']; rfl
    have hsent : sentOf sk s' c = sentOf sk s c := by
      rw [hs']
      exact sentOf_setWalk_same hwf s pk { s.walk pk with phase := 4 } hmem
        (by simp [wkWireSent, wkWireCount])
        (by simp [wkResSent, wkResCount])
        (by simp [wkQSentTot, wkQSum])
        (by simp [wkParentSent, hph])
        hc
    have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
      rw [hs']
      refine recvdOfO_setWalk_same hwf s pk
        { s.walk pk with phase := 4 } hmem ?_ ?_ hc
      · cases hord : ord.walk pk <;>
          simp [wkWireRecvdO, hord, wkWireRecvd, wkAskedRecvd, hph]
      · cases hord : ord.walk pk <;>
          simp [wkAskedRecvdO, hord, wkWireRecvd, wkAskedRecvd, hph]
    rw [hchan, hsent, hrecv]
    exact ⟨heq, hcap⟩

/-- The second end-of-stream wait: phase 4 → 5, same frame shape. -/
private theorem preserve_close4 (hwf : sk.wellFormed = true)
    {pk : Party × Nat}
    (hmem : pk ∈ sk.walkKeys) (hph : (s.walk pk).phase = 4)
    (hs' : s' = setWalk s pk { s.walk pk with phase := 5 })
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  have hwalk : s'.walk pk = { s.walk pk with phase := 5 } := by
    rw [hs']; simp
  refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_, fun c hc => ?_⟩
  · by_cases hpkeq : pk' = pk
    · subst hpkeq
      have hwk := hi.wk pk' hpk'
      simp only [wkLocalOk, hwalk] at hwk ⊢
      rw [hph] at hwk
      simp at hwk ⊢
      exact hwk
    · have hw : s'.walk pk' = s.walk pk' := by
        rw [hs']; exact setWalk_walk_ne s _ hpkeq
      rw [wkLocalOk_congr sk ax pk' hw]
      exact hi.wk pk' hpk'
  · rw [hs']; exact hi.asm pk' hpk'
  · rw [hs']; exact hi.top
  · obtain ⟨heq, hcap⟩ := hi.flow c hc
    have hchan : s'.chan = s.chan := by rw [hs']; rfl
    have hsent : sentOf sk s' c = sentOf sk s c := by
      rw [hs']
      exact sentOf_setWalk_same hwf s pk { s.walk pk with phase := 5 } hmem
        (by simp [wkWireSent, wkWireCount])
        (by simp [wkResSent, wkResCount])
        (by simp [wkQSentTot, wkQSum])
        (by simp [wkParentSent, hph])
        hc
    have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
      rw [hs']
      refine recvdOfO_setWalk_same hwf s pk
        { s.walk pk with phase := 5 } hmem ?_ ?_ hc
      · cases hord : ord.walk pk <;>
          simp [wkWireRecvdO, hord, wkWireRecvd, wkAskedRecvd, hph]
      · cases hord : ord.walk pk <;>
          simp [wkAskedRecvdO, hord, wkWireRecvd, wkAskedRecvd, hph]
    rw [hchan, hsent, hrecv]
    exact ⟨heq, hcap⟩

/-- `walkCloseWire` under the assignment: phase 3 → 4 (reply-first) or
4 → 5 (query-first); the core covers both. -/
theorem preserve_walkCloseWireO (hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : applyO sk ax ord (.walkCloseWire pk) s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  cases hord : ord.walk pk <;> simp only [applyO, hord] at hstep
  -- reply-first: phase 3 → 4
  · split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq] at hg
      obtain ⟨⟨⟨hmem, hph⟩, _hpd⟩, _hz⟩ := hg
      have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
      injection hstep with hs'
      exact preserve_close3 hwf hmem' hph hs'.symm hi
  -- query-first: phase 4 → 5
  · split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq] at hg
      obtain ⟨⟨⟨hmem, hph⟩, _hpd⟩, _hz⟩ := hg
      have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
      injection hstep with hs'
      exact preserve_close4 hwf hmem' hph hs'.symm hi

/-- `walkCloseAsked` under the assignment: phase 4 → 5 (reply-first) or
3 → 4 (query-first). -/
theorem preserve_walkCloseAskedO (hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : applyO sk ax ord (.walkCloseAsked pk) s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  cases hord : ord.walk pk <;> simp only [applyO, hord] at hstep
  -- reply-first: phase 4 → 5
  · split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq] at hg
      obtain ⟨⟨⟨hmem, hph⟩, _hpd⟩, _hz⟩ := hg
      have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
      injection hstep with hs'
      exact preserve_close4 hwf hmem' hph hs'.symm hi
  -- query-first: phase 3 → 4
  · split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq] at hg
      obtain ⟨⟨⟨hmem, hph⟩, _hpd⟩, _hz⟩ := hg
      have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
      injection hstep with hs'
      exact preserve_close3 hwf hmem' hph hs'.symm hi

-- ================================================ the prologue receives

/-- The first receive of a scope, either channel: phase 0 → 1, the
committed slot cleared, occupancy of the received channel dropping by
one exactly as its O count rises by one. The call site (which knows
the assignment) routes the rising count (`hrise`) and pins the partner
channel's count unchanged (`hkeep`). -/
private theorem preserve_recv_first_core (hwf : sk.wellFormed = true)
    {pk : Party × Nat} {cin cpart : Chan}
    (hmem : pk ∈ sk.walkKeys) (hph0 : (s.walk pk).phase = 0)
    (hpos : s.chan cin > 0)
    (hs' : s' = setWalk { s with chan := bump s.chan cin (-1) } pk
      { s.walk pk with phase := 1, committed := none })
    (hpair : (cin = wireIn pk ∧ cpart = askedIn pk) ∨
      (cin = askedIn pk ∧ cpart = wireIn pk))
    (hrise : recvdOfO sk ord (setWalk { s with chan := bump s.chan cin (-1) } pk
        { s.walk pk with phase := 1, committed := none }) cin
      = recvdOfO sk ord s cin + 1)
    (hkeep : recvdOfO sk ord (setWalk { s with chan := bump s.chan cin (-1) } pk
        { s.walk pk with phase := 1, committed := none }) cpart
      = recvdOfO sk ord s cpart)
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  have hwalk : s'.walk pk = { s.walk pk with phase := 1, committed := none } := by
    rw [hs']; simp
  refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_, fun c hc => ?_⟩
  · by_cases hpkeq : pk' = pk
    · subst hpkeq
      have hwk := hi.wk pk' hpk'
      simp only [wkLocalOk, hwalk] at hwk ⊢
      rw [hph0] at hwk
      simp at hwk ⊢
      exact ⟨hwk.1, hwk.2.1⟩
    · have hw : s'.walk pk' = s.walk pk' := by
        rw [hs']; exact setWalk_walk_ne _ _ hpkeq
      rw [wkLocalOk_congr sk ax pk' hw]
      exact hi.wk pk' hpk'
  · rw [hs']; exact hi.asm pk' hpk'
  · rw [hs']; exact hi.top
  · obtain ⟨heq, hcap⟩ := hi.flow c hc
    have hchan : s'.chan = bump s.chan cin (-1) := by
      rw [hs']; rfl
    have hsent : sentOf sk s' c = sentOf sk s c := by
      rw [hs']
      exact sentOf_setWalk_same hwf _ pk
        { s.walk pk with phase := 1, committed := none } hmem
        (by simp [wkWireSent, wkWireCount])
        (by simp [wkResSent, wkResCount])
        (by simp [wkQSentTot, wkQSum])
        (by simp [wkParentSent, hph0])
        hc
    by_cases h5 : c = cin
    · subst h5
      have hr' : recvdOfO sk ord s' c = recvdOfO sk ord s c + 1 := by
        rw [hs']; exact hrise
      rw [hchan, hsent, hr', bump_neg_one]
      exact ⟨by omega, by omega⟩
    · have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
        by_cases h6 : c = cpart
        · subst h6; rw [hs']; exact hkeep
        · rw [hs']
          have hw5 : c ≠ wireIn pk := by
            rcases hpair with ⟨hcw, hca⟩ | ⟨hcw, hca⟩
            · rw [← hcw]; exact h5
            · rw [← hca]; exact h6
          have hw6 : c ≠ askedIn pk := by
            rcases hpair with ⟨hcw, hca⟩ | ⟨hcw, hca⟩
            · rw [← hca]; exact h6
            · rw [← hcw]; exact h5
          exact recvdOfO_setWalk_frame hwf _ pk _ hc hw5 hw6
      rw [hchan, hsent, hrecv, bump_ne _ _ h5]
      exact ⟨heq, hcap⟩

/-- The second receive of a scope, either channel: phase 1 → 2 with the
embedded `normWalk` provably the identity (the phase-1 machinery is
empty, so `scopeComplete` is false). -/
private theorem preserve_recv_second_core (hwf : sk.wellFormed = true)
    {pk : Party × Nat} {cin cpart : Chan}
    (hmem : pk ∈ sk.walkKeys) (hph1 : (s.walk pk).phase = 1)
    (hpos : s.chan cin > 0)
    (hs' : s' = setWalk { s with chan := bump s.chan cin (-1) } pk
      (normWalk sk pk.2 { s.walk pk with phase := 2, committed := none }))
    (hpair : (cin = wireIn pk ∧ cpart = askedIn pk) ∨
      (cin = askedIn pk ∧ cpart = wireIn pk))
    (hrise : recvdOfO sk ord (setWalk { s with chan := bump s.chan cin (-1) } pk
        { s.walk pk with phase := 2, committed := none }) cin
      = recvdOfO sk ord s cin + 1)
    (hkeep : recvdOfO sk ord (setWalk { s with chan := bump s.chan cin (-1) } pk
        { s.walk pk with phase := 2, committed := none }) cpart
      = recvdOfO sk ord s cpart)
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  -- phase-1 facts from the invariant: cursor in range, machinery empty
  have hwk := hi.wk pk hmem
  simp only [wkLocalOk] at hwk
  rw [hph1] at hwk
  simp at hwk
  obtain ⟨hslt, ⟨hledger, hpd⟩, hcm⟩ := hwk
  have hnlt : ¬ (s.walk pk).scope ≥ sk.stageLen pk.2 := by omega
  have hscF : scopeComplete sk pk.2
      { s.walk pk with phase := 2, committed := none } = false := by
    simp [scopeComplete, hnlt, hpd]
  have hnw : normWalk sk pk.2 { s.walk pk with phase := 2, committed := none }
      = { s.walk pk with phase := 2, committed := none } := by
    simp [normWalk, hscF]
  rw [hnw] at hs'
  have hwalk : s'.walk pk = { s.walk pk with phase := 2, committed := none } := by
    rw [hs']; simp
  refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_, fun c hc => ?_⟩
  · by_cases hpkeq : pk' = pk
    · subst hpkeq
      simp only [wkLocalOk, hwalk, hscF]
      simp [hslt]
      intro j hj
      simp [hledger j hj]
    · have hw : s'.walk pk' = s.walk pk' := by
        rw [hs']; exact setWalk_walk_ne _ _ hpkeq
      rw [wkLocalOk_congr sk ax pk' hw]
      exact hi.wk pk' hpk'
  · rw [hs']; exact hi.asm pk' hpk'
  · rw [hs']; exact hi.top
  · obtain ⟨heq, hcap⟩ := hi.flow c hc
    have hchan : s'.chan = bump s.chan cin (-1) := by
      rw [hs']; rfl
    have hsent : sentOf sk s' c = sentOf sk s c := by
      rw [hs']
      exact sentOf_setWalk_same hwf _ pk
        { s.walk pk with phase := 2, committed := none } hmem
        (by simp [wkWireSent, wkWireCount])
        (by simp [wkResSent, wkResCount])
        (by simp [wkQSentTot, wkQSum])
        (by simp [wkParentSent, hph1, hpd])
        hc
    by_cases h5 : c = cin
    · subst h5
      have hr' : recvdOfO sk ord s' c = recvdOfO sk ord s c + 1 := by
        rw [hs']; exact hrise
      rw [hchan, hsent, hr', bump_neg_one]
      exact ⟨by omega, by omega⟩
    · have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
        by_cases h6 : c = cpart
        · subst h6; rw [hs']; exact hkeep
        · rw [hs']
          have hw5 : c ≠ wireIn pk := by
            rcases hpair with ⟨hcw, hca⟩ | ⟨hcw, hca⟩
            · rw [← hcw]; exact h5
            · rw [← hca]; exact h6
          have hw6 : c ≠ askedIn pk := by
            rcases hpair with ⟨hcw, hca⟩ | ⟨hcw, hca⟩
            · rw [← hca]; exact h6
            · rw [← hcw]; exact h5
          exact recvdOfO_setWalk_frame hwf _ pk _ hc hw5 hw6
      rw [hchan, hsent, hrecv, bump_ne _ _ h5]
      exact ⟨heq, hcap⟩

/-- `walkRecvWire` under the assignment: the first receive when
reply-first (phase 0 → 1), the second when query-first (phase 1 → 2,
`normWalk` embedded). -/
theorem preserve_walkRecvWireO (hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : applyO sk ax ord (.walkRecvWire pk) s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  cases hord : ord.walk pk <;> simp only [applyO, hord] at hstep
  · -- reply-first: phase 0 → 1
    split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
      obtain ⟨⟨hmem, hph0⟩, hpos⟩ := hg
      have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
      injection hstep with hs'
      refine preserve_recv_first_core hwf hmem' hph0 hpos hs'.symm
        (Or.inl ⟨rfl, rfl⟩) ?_ ?_ hi
      · rw [recvdOfO_wireIn hmem', recvdOfO_wireIn hmem']
        simp [wkWireRecvdO, hord, wkWireRecvd, hph0]
      · rw [recvdOfO_askedIn, recvdOfO_askedIn]
        simp [wkAskedRecvdO, hord, wkAskedRecvd, hph0]
  · -- query-first: phase 1 → 2
    split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
      obtain ⟨⟨hmem, hph1⟩, hpos⟩ := hg
      have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
      injection hstep with hs'
      refine preserve_recv_second_core hwf hmem' hph1 hpos ?_
        (Or.inl ⟨rfl, rfl⟩) ?_ ?_ hi
      · exact hs'.symm
      · rw [recvdOfO_wireIn hmem', recvdOfO_wireIn hmem']
        simp [wkWireRecvdO, hord, wkAskedRecvd, hph1]
      · rw [recvdOfO_askedIn, recvdOfO_askedIn]
        simp [wkAskedRecvdO, hord, wkWireRecvd, hph1]

/-- `walkRecvAsked` under the assignment: the second receive when
reply-first (phase 1 → 2, `normWalk` embedded), the first when
query-first (phase 0 → 1). -/
theorem preserve_walkRecvAskedO (hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : applyO sk ax ord (.walkRecvAsked pk) s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  cases hord : ord.walk pk <;> simp only [applyO, hord] at hstep
  · -- reply-first: phase 1 → 2
    split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
      obtain ⟨⟨hmem, hph1⟩, hpos⟩ := hg
      have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
      injection hstep with hs'
      refine preserve_recv_second_core hwf hmem' hph1 hpos ?_
        (Or.inr ⟨rfl, rfl⟩) ?_ ?_ hi
      · exact hs'.symm
      · rw [recvdOfO_askedIn, recvdOfO_askedIn]
        simp [wkAskedRecvdO, hord, wkAskedRecvd, hph1]
      · rw [recvdOfO_wireIn hmem', recvdOfO_wireIn hmem']
        simp [wkWireRecvdO, hord, wkWireRecvd, hph1]
  · -- query-first: phase 0 → 1
    split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
      obtain ⟨⟨hmem, hph0⟩, hpos⟩ := hg
      have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
      injection hstep with hs'
      refine preserve_recv_first_core hwf hmem' hph0 hpos hs'.symm
        (Or.inr ⟨rfl, rfl⟩) ?_ ?_ hi
      · rw [recvdOfO_askedIn, recvdOfO_askedIn]
        simp [wkAskedRecvdO, hord, wkWireRecvd, hph0]
      · rw [recvdOfO_wireIn hmem', recvdOfO_wireIn hmem']
        simp [wkWireRecvdO, hord, wkAskedRecvd, hph0]

-- ===================================================== the shared arms

/-- `walkCommit` under the assignment: the arm is the base model's
(`applyO` delegates), nothing observable to any count changes, and the
committed-arm ledger holds by the base argument verbatim. -/
theorem preserve_walkCommitO (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (o : Oblig)
    (hstep : applyO sk ax ord (.walkCommit pk o) s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  have hstep' : apply sk ax (.walkCommit pk o) s = some s' := hstep
  simp only [apply] at hstep'
  split at hstep'
  case isFalse => simp at hstep'
  case isTrue hg =>
    simp only [Bool.and_eq_true] at hg
    obtain ⟨hmem, hch⟩ := hg
    injection hstep' with hs'
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
          recvdOfO_setWalk_committed s pk (some o) c]
        exact hi.flow c hc

end StreamingMirror.Ord
