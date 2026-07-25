/-
The O endgame: the metatheorem. `EndgameE.lean`'s argmin assembly
re-targeted at the ord-parameterized traces (`procsO`/`scheduleO`)
under `AxMode.impl`, for every assignment of the two-point per-loop
dequeue-order class — the O decode layer (Ord/Pending.lean) supplying
the per-family splits in `performedO`/`PendOkO` denomination, the
merge completeness `hrem` seam discharged by `merge_completeO`
(Ord/Final.lean) at the flagship's hypotheses, the pillar consumed at
`hmode := Or.inl rfl` through the order-blind commit arms, and the
close cascade dispatched per assignment (a phase-3 loop closes
whichever input its order dequeues FIRST — the wire reply-first, the
asked channel query-first — with the producer-done facts for BOTH
channels available at the descending sweep's frontier).

Concludes `progress_of_invO`/`progressO` and the flagship
`Sched.deadlock_free_anyOrder`, plus the run-level corollaries
(`maximal_run_terminal_anyOrder`, `greedy_run_terminal_anyOrder`) and
the reply-first consistency pin (`deadlock_free_rf_corner`).

Chain (ord, stage E exit): the metatheorem. Base mirror:
EndgameE.lean + Proofs/Termination.lean's flagship corollaries. Map:
PROGRESS.md §13.
-/
import StreamingMirror.Ord.Pending
import StreamingMirror.Ord.Final
import StreamingMirror.Ord.Termination
import StreamingMirror.Ord.Preserve

namespace StreamingMirror.Ord

open Model
open Sched

variable (sk : Skel) (ord : OrdMap)

/-- Every pending event of the state under the assignment, across all
processes: the base pool with the two loop pend lists (`wkPendO`,
`abPendO`) reading the assignment (cf. `Sched.pends`, its reply-first
instance). -/
def pendsO (s : State) : List (Ev × Action) :=
  ioPend sk s ++ roPend sk s
    ++ sk.walkKeys.flatMap (wkPendO sk ord s)
    ++ abPendO ord s
    ++ sk.asmKeys.flatMap (asmPend sk s)
    ++ rrPend s ++ finPend sk s

/-- One enabled enumerated action is enough to step under the
assignment (cf. `Model.canStep_of_action`). -/
theorem canStepO_of_action {ax : AxMode} {s : State} {a : Action}
    (ha : a ∈ allActions sk)
    (happ : (applyO sk ax ord a s).isSome = true) :
    canStepO sk ax ord s = true := by
  rw [canStepO, List.any_eq_true]
  exact ⟨a, ha, happ⟩

/-- The per-family split of an O merge-input trace (cf.
`Sched.procsE_cases`). -/
theorem procsO_cases {T : List Ev} (hT : T ∈ procsO sk ord) :
    T = iopenEvents sk ∨ T = ropenEvents sk
    ∨ (∃ i, i < sk.rootH ∧ T = walkEventsO sk ord
        ((if (sk.rootH - 1 - i) % 2 == 1 then Party.I else Party.R),
          sk.rootH - 1 - i))
    ∨ T = absorbEventsO sk ord
    ∨ (∃ pk ∈ sk.asmKeys, T = asmEvents sk pk)
    ∨ T = [(Chan.rootret, false, 0)] ∨ T = finEvents sk := by
  simp only [procsO, List.mem_append, List.mem_cons, List.mem_map,
    List.not_mem_nil, or_false] at hT
  rcases hT with ((((hT | hT) | ⟨a, ⟨i, hir, rfl⟩, rfl⟩) | hT)
    | ⟨pk2, hpk2, rfl⟩) | hT | hT
  · exact Or.inl hT
  · exact Or.inr (Or.inl hT)
  · rw [List.mem_range] at hir
    exact Or.inr (Or.inr (Or.inl ⟨i, hir, rfl⟩))
  · exact Or.inr (Or.inr (Or.inr (Or.inl hT)))
  · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inl ⟨pk2, hpk2, rfl⟩))))
  · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inl hT)))))
  · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr hT)))))

/-- The fixed family traces are O merge inputs. -/
theorem fixed_mem_procsO :
    iopenEvents sk ∈ procsO sk ord ∧ ropenEvents sk ∈ procsO sk ord
    ∧ absorbEventsO sk ord ∈ procsO sk ord
    ∧ [((Chan.rootret, false, 0) : Ev)] ∈ procsO sk ord
    ∧ finEvents sk ∈ procsO sk ord := by
  refine ⟨?_, ?_, ?_, ?_, ?_⟩ <;> simp [procsO]

/-- Every walk key's O trace is a merge input. -/
theorem walkEventsO_mem_procsO (hwf : sk.wellFormed = true)
    {pk : Party × Nat} (hpk : pk ∈ sk.walkKeys) :
    walkEventsO sk ord pk ∈ procsO sk ord := by
  obtain ⟨p, k⟩ := pk
  obtain ⟨hkr, hpar⟩ := walkKeys_parity sk hwf hpk
  simp only [procsO]
  refine List.mem_append.mpr (Or.inl (List.mem_append.mpr (Or.inl
    (List.mem_append.mpr (Or.inl (List.mem_append.mpr (Or.inr ?_)))))))
  refine List.mem_map.mpr ⟨(p, k), ?_, rfl⟩
  refine List.mem_map.mpr ⟨sk.rootH - 1 - k, List.mem_range.mpr (by omega), ?_⟩
  have hh : sk.rootH - 1 - (sk.rootH - 1 - k) = k := by omega
  rw [hh]
  rcases hpar with ⟨rfl, hodd⟩ | ⟨rfl, heven⟩
  · rw [if_pos (by simp [hodd])]
  · rw [if_neg (by simp [heven])]

/-- Every assembler key's trace is an O merge input. -/
theorem asmEvents_mem_procsO {pk : Party × Nat} (hpk : pk ∈ sk.asmKeys) :
    asmEvents sk pk ∈ procsO sk ord := by
  simp only [procsO]
  refine List.mem_append.mpr (Or.inl (List.mem_append.mpr (Or.inr ?_)))
  exact List.mem_map.mpr ⟨pk, hpk, rfl⟩

/-- Family pending lists inject into the O pending pool (cf.
`Sched.pends_lift`). -/
theorem pends_liftO {s : State} :
    (∀ fa ∈ ioPend sk s, fa ∈ pendsO sk ord s)
    ∧ (∀ fa ∈ roPend sk s, fa ∈ pendsO sk ord s)
    ∧ (∀ pk ∈ sk.walkKeys, ∀ fa ∈ wkPendO sk ord s pk,
        fa ∈ pendsO sk ord s)
    ∧ (∀ fa ∈ abPendO ord s, fa ∈ pendsO sk ord s)
    ∧ (∀ pk ∈ sk.asmKeys, ∀ fa ∈ asmPend sk s pk, fa ∈ pendsO sk ord s)
    ∧ (∀ fa ∈ rrPend s, fa ∈ pendsO sk ord s)
    ∧ (∀ fa ∈ finPend sk s, fa ∈ pendsO sk ord s) := by
  unfold pendsO
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · intro fa h
    exact List.mem_append.mpr (.inl (List.mem_append.mpr (.inl
      (List.mem_append.mpr (.inl (List.mem_append.mpr (.inl
        (List.mem_append.mpr (.inl (List.mem_append.mpr (.inl h)))))))))))
  · intro fa h
    exact List.mem_append.mpr (.inl (List.mem_append.mpr (.inl
      (List.mem_append.mpr (.inl (List.mem_append.mpr (.inl
        (List.mem_append.mpr (.inl (List.mem_append.mpr (.inr h)))))))))))
  · intro pk hpk fa h
    exact List.mem_append.mpr (.inl (List.mem_append.mpr (.inl
      (List.mem_append.mpr (.inl (List.mem_append.mpr (.inl
        (List.mem_append.mpr (.inr
          (List.mem_flatMap.mpr ⟨pk, hpk, h⟩))))))))))
  · intro fa h
    exact List.mem_append.mpr (.inl (List.mem_append.mpr (.inl
      (List.mem_append.mpr (.inl (List.mem_append.mpr (.inr h)))))))
  · intro pk hpk fa h
    exact List.mem_append.mpr (.inl (List.mem_append.mpr (.inl
      (List.mem_append.mpr (.inr (List.mem_flatMap.mpr ⟨pk, hpk, h⟩))))))
  · intro fa h
    exact List.mem_append.mpr (.inl (List.mem_append.mpr (.inr h)))
  · intro fa h
    exact List.mem_append.mpr (.inr h)

/-- Soundness of the O pool: every pending entry is `PendOkO` and sits
at its trace's O-performed frontier (cf. `Sched.pends_soundE`; the
walk and absorber cases place the pend in `walkEventsO`/
`absorbEventsO`, everything else transfers by bridge). -/
theorem pends_soundO (hwf : sk.wellFormed = true) {s : State}
    (hi : InvL sk .impl s)
    (hioh : s.iopenCh = none → doneIOpen s = true)
    (hroh : s.ropenGotWire = true → s.ropenCh = none →
      doneROpen sk s = true)
    (hwkh : ∀ pk ∈ sk.walkKeys,
      ¬((s.walk pk).phase = 2 ∧ (s.walk pk).committed = none)) :
    ∀ fa ∈ pendsO sk ord s, PendOkO sk ord s fa.1 fa.2
      ∧ ∃ T pre suf, T ∈ procsO sk ord ∧ T = pre ++ fa.1 :: suf
        ∧ ∀ e ∈ pre, performedO sk ord s e := by
  intro fa hfa
  unfold pendsO at hfa
  rcases List.mem_append.1 hfa with hfa | hfin
  rcases List.mem_append.1 hfa with hfa | hrr
  rcases List.mem_append.1 hfa with hfa | hasm
  rcases List.mem_append.1 hfa with hfa | hab
  rcases List.mem_append.1 hfa with hfa | hwk
  rcases List.mem_append.1 hfa with hio | hro
  · rcases iopen_pend_or_doneO sk ord hwf hi hioh with ⟨-, hnil⟩ | h
    · rw [hnil] at hio; cases hio
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      rw [heq, List.mem_singleton] at hio
      subst hio
      exact ⟨hok, iopenEvents sk, pre, suf, (fixed_mem_procsO sk ord).1,
        hdec, hpre⟩
  · rcases ropen_pend_or_doneO sk ord hwf hi hroh with ⟨-, hnil⟩ | h
    · rw [hnil] at hro; cases hro
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      rw [heq, List.mem_singleton] at hro
      subst hro
      exact ⟨hok, ropenEvents sk, pre, suf, (fixed_mem_procsO sk ord).2.1,
        hdec, hpre⟩
  · obtain ⟨pk, hpk, hfa⟩ := List.mem_flatMap.1 hwk
    rcases walk_pend_or_doneO sk ord hwf hi hpk (hwkh pk hpk) with
      ⟨-, hnil⟩ | h
    · rw [hnil] at hfa; cases hfa
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      rw [heq, List.mem_singleton] at hfa
      subst hfa
      exact ⟨hok, walkEventsO sk ord pk, pre, suf,
        walkEventsO_mem_procsO sk ord hwf hpk, hdec, hpre⟩
  · rcases absorb_pend_or_doneO sk ord hwf hi with ⟨-, hnil⟩ | h
    · rw [hnil] at hab; cases hab
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      rw [heq, List.mem_singleton] at hab
      subst hab
      exact ⟨hok, absorbEventsO sk ord, pre, suf,
        (fixed_mem_procsO sk ord).2.2.1, hdec, hpre⟩
  · obtain ⟨pk, hpk, hfa⟩ := List.mem_flatMap.1 hasm
    rcases asm_pend_or_doneO sk ord hwf hi hpk with ⟨-, hnil⟩ | h
    · rw [hnil] at hfa; cases hfa
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      rw [heq, List.mem_singleton] at hfa
      subst hfa
      exact ⟨hok, asmEvents sk pk, pre, suf,
        asmEvents_mem_procsO sk ord hpk, hdec, hpre⟩
  · rcases rootret_pend_or_doneO sk ord (s := s) with ⟨-, hnil⟩ | h
    · rw [hnil] at hrr; cases hrr
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      rw [heq, List.mem_singleton] at hrr
      subst hrr
      exact ⟨hok, [(Chan.rootret, false, 0)], pre, suf,
        (fixed_mem_procsO sk ord).2.2.2.1, hdec, hpre⟩
  · rcases fin_pend_or_doneO sk ord hi with ⟨-, hnil⟩ | h
    · rw [hnil] at hfin; cases hfin
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      rw [heq, List.mem_singleton] at hfin
      subst hfin
      exact ⟨hok, finEvents sk, pre, suf,
        (fixed_mem_procsO sk ord).2.2.2.2, hdec, hpre⟩

/-- The O cover: an O-unperformed `scheduleO` event is τ-dominated by
some pending entry — its own trace's frontier sits at or before it
(cf. `Sched.pends_coverE`; the `hrem` seam of `tau_le_of_pendO` is
discharged by `merge_completeO` at the margin-0 hypothesis). -/
theorem pends_coverO (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) {s : State}
    (hi : InvL sk .impl s)
    (hioh : s.iopenCh = none → doneIOpen s = true)
    (hroh : s.ropenGotWire = true → s.ropenCh = none →
      doneROpen sk s = true)
    (hwkh : ∀ pk ∈ sk.walkKeys,
      ¬((s.walk pk).phase = 2 ∧ (s.walk pk).committed = none))
    {g : Ev} (hmem : g ∈ scheduleO sk ord)
    (hnp : ¬ performedO sk ord s g) :
    ∃ fa ∈ pendsO sk ord s,
      evIdx fa.1 (scheduleO sk ord) ≤ evIdx g (scheduleO sk ord) := by
  have hrem := merge_completeO sk ord hwf hm0
  obtain ⟨T, hT, hgT⟩ := sched_mem_traceO sk ord hmem
  obtain ⟨hlio, hlro, hlwk, hlab, hlasm, hlrr, hlfin⟩ :=
    pends_liftO sk ord (s := s)
  rcases procsO_cases sk ord hT with rfl | hc
  · rcases iopen_pend_or_doneO sk ord hwf hi hioh with ⟨hall, -⟩ | h
    · exact absurd (hall g hgT) hnp
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      refine ⟨(f, a), hlio _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        tau_le_of_pendO sk ord hwf hrem hT hdec hpre hgT hnp⟩
  rcases hc with rfl | hc
  · rcases ropen_pend_or_doneO sk ord hwf hi hroh with ⟨hall, -⟩ | h
    · exact absurd (hall g hgT) hnp
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      refine ⟨(f, a), hlro _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        tau_le_of_pendO sk ord hwf hrem hT hdec hpre hgT hnp⟩
  rcases hc with ⟨i, hir, rfl⟩ | hc
  · have hpk := walkOrder_mem_keys sk hwf hir
    rcases walk_pend_or_doneO sk ord hwf hi hpk (hwkh _ hpk) with
      ⟨hall, -⟩ | h
    · exact absurd (hall g hgT) hnp
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      refine ⟨(f, a),
        hlwk _ hpk _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        tau_le_of_pendO sk ord hwf hrem hT hdec hpre hgT hnp⟩
  rcases hc with rfl | hc
  · rcases absorb_pend_or_doneO sk ord hwf hi with ⟨hall, -⟩ | h
    · exact absurd (hall g hgT) hnp
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      refine ⟨(f, a), hlab _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        tau_le_of_pendO sk ord hwf hrem hT hdec hpre hgT hnp⟩
  rcases hc with ⟨pk, hpk, rfl⟩ | hc
  · rcases asm_pend_or_doneO sk ord hwf hi hpk with ⟨hall, -⟩ | h
    · exact absurd (hall g hgT) hnp
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      refine ⟨(f, a),
        hlasm _ hpk _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        tau_le_of_pendO sk ord hwf hrem hT hdec hpre hgT hnp⟩
  rcases hc with rfl | rfl
  · rcases rootret_pend_or_doneO sk ord (s := s) with ⟨hall, -⟩ | h
    · exact absurd (hall g hgT) hnp
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      refine ⟨(f, a), hlrr _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        tau_le_of_pendO sk ord hwf hrem hT hdec hpre hgT hnp⟩
  · rcases fin_pend_or_doneO sk ord hi with ⟨hall, -⟩ | h
    · exact absurd (hall g hgT) hnp
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      refine ⟨(f, a), hlfin _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        tau_le_of_pendO sk ord hwf hrem hT hdec hpre hgT hnp⟩

-- ================================================== the close cascade

/-- With every process past its channel work, either a close fires or
the session is terminal — the O counterpart of `Sched.close_cascadeE`,
over the weak O invariant.

The frontier close is dispatched on the loop's assignment: a phase-3
walk closes whichever input it dequeues FIRST (the wire reply-first,
the asked channel query-first), phase 4 the other — and the
producer-done facts for BOTH channels are available at the descending
sweep's frontier (the wire's producer is the walk one stage up or the
responder opener; the asked producer is the walk two stages up or an
opener — all done above the frontier). The absorber's two closes
dispatch likewise on its assignment. -/
theorem close_cascadeO (hwf : sk.wellFormed = true) {s : State}
    (hi : InvPWO sk .impl ord s)
    (hIOd : doneIOpen s = true) (hROd : doneROpen sk s = true)
    (hwkph : ∀ pk ∈ sk.walkKeys, 3 ≤ (s.walk pk).phase)
    (habph : 3 ≤ s.absorbPhase)
    (hasmph : ∀ pk ∈ sk.asmKeys, 3 ≤ (s.asm pk).phase)
    (hfin : s.ifin = true) (hres : s.rfinGotRes = true)
    (hgot : s.rfinGot = sk.rootPending) :
    canStepO sk .impl ord s = true ∨ terminal sk s = true := by
  have hge : 2 ≤ sk.rootH := (wf_rootH hwf).2
  have hev : sk.rootH % 2 = 0 := (wf_rootH hwf).1
  simp only [doneIOpen, Bool.and_eq_true] at hIOd
  simp only [doneROpen, Bool.and_eq_true, beq_iff_eq] at hROd
  obtain ⟨hiw, hiq⟩ := hIOd
  obtain ⟨⟨⟨hgw, hrw⟩, hrr⟩, hrq⟩ := hROd
  -- the per-walk drained totals, with the O receive counts
  have hWfacts : ∀ pk ∈ sk.walkKeys,
      wkWireSent sk s pk = sk.wiresBefore pk.2 (sk.stageLen pk.2)
      ∧ wkQSentTot sk s pk = sk.qsBefore pk.2 (sk.stageLen pk.2)
      ∧ wkParentSent s pk = sk.stageLen pk.2
      ∧ wkResSent sk s pk = sk.dsBefore pk.2 (sk.stageLen pk.2)
      ∧ wkWireRecvdO sk ord s pk = sk.stageLen pk.2
      ∧ wkAskedRecvdO sk ord s pk = sk.stageLen pk.2 := by
    intro pk hpk
    have hph := hwkph pk hpk
    have hsc := (walk_scope_boundE sk hi.local hpk).2 hph
    obtain ⟨hled, hpd, -⟩ := walk_ledgers_emptyE sk hi.local hpk (by omega)
    obtain ⟨hw0, hr0, hq0⟩ := counts_of_emptyE sk hled
    have hWR : wkWireRecvd sk s pk = sk.stageLen pk.2 := by
      unfold wkWireRecvd
      rw [if_pos (by omega)]
    have hAR : wkAskedRecvd sk s pk = sk.stageLen pk.2 := by
      unfold wkAskedRecvd
      rw [if_pos (by omega)]
    refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩
    · unfold wkWireSent
      rw [hsc, hw0]
      omega
    · unfold wkQSentTot
      rw [hsc, hq0]
      omega
    · simp only [wkParentSent]
      rw [hsc, if_neg (by simp; omega)]
      omega
    · unfold wkResSent
      rw [hsc, hr0]
      omega
    · cases hord : ord.walk pk <;>
        simp only [wkWireRecvdO, hord] <;> assumption
    · cases hord : ord.walk pk <;>
        simp only [wkAskedRecvdO, hord] <;> assumption
  -- drained channels, from O flow at equal totals
  have hchan0 : ∀ c ∈ allChans sk, sentOf sk s c = recvdOfO sk ord s c →
      s.chan c = 0 := by
    intro c hc heq
    have := hi.flow c hc
    omega
  -- descending sweep: the highest undone walk can close
  have hdesc : ∀ d, canStepO sk .impl ord s = true
      ∨ ∀ pk ∈ sk.walkKeys, sk.rootH - d ≤ pk.2 →
          doneWalk (s.walk pk) = true := by
    intro d
    induction d with
    | zero =>
        right
        intro pk hpk hgep
        obtain ⟨p, k⟩ := pk
        obtain ⟨hkr, -⟩ := walkKeys_parity sk hwf hpk
        omega
    | succ d ih =>
        rcases ih with hstep | hdone
        · exact Or.inl hstep
        · by_cases hsat : sk.rootH ≤ d
          · right
            intro pk hpk hgep
            exact hdone pk hpk (by omega)
          · -- the walk at the frontier height
            have hhlt : sk.rootH - (d + 1) < sk.rootH := by omega
            obtain ⟨pkh, hpkh, hpkh2⟩ :
                ∃ pkh ∈ sk.walkKeys, pkh.2 = sk.rootH - (d + 1) := by
              by_cases hpar : (sk.rootH - (d + 1)) % 2 = 1
              · exact ⟨(Party.I, sk.rootH - (d + 1)),
                  mem_walkKeys_of sk hwf hhlt (Or.inl ⟨rfl, hpar⟩), rfl⟩
              · exact ⟨(Party.R, sk.rootH - (d + 1)),
                  mem_walkKeys_of sk hwf hhlt (Or.inr ⟨rfl, by omega⟩), rfl⟩
            obtain ⟨p, k⟩ := pkh
            have hk2 : k = sk.rootH - (d + 1) := hpkh2
            have hph := hwkph (p, k) hpkh
            have hple : (s.walk (p, k)).phase ≤ 5 := by
              have hwk := hi.wk (p, k) hpkh
              simp only [wkLocalOk] at hwk
              rcases Bool.and_eq_true .. ▸ hwk with ⟨hcur, -⟩
              simp only [Bool.and_eq_true] at hcur
              obtain ⟨⟨-, hle⟩, -⟩ := hcur
              simpa using hle
            by_cases h5 : (s.walk (p, k)).phase = 5
            · right
              intro pk hpk hgep
              by_cases hup : sk.rootH - d ≤ pk.2
              · exact hdone pk hpk hup
              · have : pk = (p, k) :=
                  walkKeys_eq_of_height sk hwf hpk hpkh (by omega)
                rw [this]
                simp [doneWalk, h5]
            · -- phase 3 or 4: the assignment's next close is enabled
              left
              have hup_done : ∀ h2, sk.rootH - (d + 1) < h2 →
                  h2 < sk.rootH →
                  ∀ pk2 ∈ sk.walkKeys, pk2.2 = h2 →
                  doneWalk (s.walk pk2) = true := by
                intro h2 hlt2 hltr pk2 hpk2 hpk2h
                exact hdone pk2 hpk2 (by omega)
              obtain ⟨-, hpar⟩ := walkKeys_parity sk hwf hpkh
              -- the prologue wire's producer done and channel drained
              have hprodW : producerDone sk s (wireIn (p, k)) = true := by
                show producerDone sk s
                  (Chan.wire p.other (k + 1)) = true
                simp only [producerDone]
                by_cases htop : k + 1 = sk.rootH
                · rw [if_pos (by simp [htop])]
                  have hparh : p = Party.I := by
                    rcases hpar with ⟨hp, -⟩ | ⟨hp, he⟩
                    · exact hp
                    · exfalso
                      have he' : k % 2 = 0 := he
                      omega
                  rw [hparh]
                  show (if (Party.R == Party.I) = true then _ else _) = true
                  rw [if_neg (by simp)]
                  simp only [doneROpen, Bool.and_eq_true, beq_iff_eq]
                  exact ⟨⟨⟨hgw, hrw⟩, hrr⟩, hrq⟩
                · rw [if_neg (by simp [htop])]
                  have hpk2 : (p.other, k + 1) ∈ sk.walkKeys := by
                    refine mem_walkKeys_of sk hwf (by omega) ?_
                    rcases hpar with ⟨hp, ho⟩ | ⟨hp, he⟩
                    · rw [hp]
                      exact Or.inr ⟨rfl, by omega⟩
                    · rw [hp]
                      exact Or.inl ⟨rfl, by omega⟩
                  exact hup_done (k + 1) (by omega) (by omega)
                    _ hpk2 rfl
              have hchanW : s.chan (wireIn (p, k)) = 0 := by
                refine hchan0 _ (wireIn_mem_allChans sk hwf hpkh) ?_
                show sentOf sk s (Chan.wire p.other (k + 1))
                  = recvdOfO sk ord s (wireIn (p, k))
                rw [recvdOfO_wireIn hpkh, (hWfacts _ hpkh).2.2.2.2.1]
                by_cases htop : k + 1 = sk.rootH
                · have hparh : p = Party.I := by
                    rcases hpar with ⟨hp, -⟩ | ⟨hp, he⟩
                    · exact hp
                    · exfalso
                      have he' : k % 2 = 0 := he
                      omega
                  rw [hparh]
                  show sentOf sk s (Chan.wire Party.R (k + 1)) = _
                  rw [htop]
                  simp only [sentOf]
                  rw [if_pos (by simp), if_neg (by simp), hrw,
                    show k = sk.rootH - 1 from by omega,
                    wf_stageLen_top sk hwf]
                  rfl
                · have hpk2 : (p.other, k + 1) ∈ sk.walkKeys := by
                    refine mem_walkKeys_of sk hwf (by omega) ?_
                    rcases hpar with ⟨hp, ho⟩ | ⟨hp, he⟩
                    · rw [hp]
                      exact Or.inr ⟨rfl, by omega⟩
                    · rw [hp]
                      exact Or.inl ⟨rfl, by omega⟩
                  have : Chan.wire p.other (k + 1)
                      = wireOut (p.other, k + 1) := rfl
                  rw [this, sentOf_wireOut hpk2,
                    (hWfacts _ hpk2).1,
                    wiresBefore_full hwf (by omega)]
              -- the asked channel's producer done and channel drained
              have hprodA : producerDone sk s (askedIn (p, k)) = true := by
                show producerDone sk s (Chan.asked p k) = true
                simp only [producerDone]
                by_cases hI : p = Party.I ∧ k = sk.rootH - 1
                · rw [if_pos (by simp [hI.1, hI.2])]
                  simp only [doneIOpen, Bool.and_eq_true]
                  exact ⟨hiw, hiq⟩
                · rw [if_neg (by
                    rcases hpar with ⟨rfl, -⟩ | ⟨rfl, -⟩ <;> simp_all)]
                  by_cases hR : p = Party.R ∧ k = sk.rootH - 2
                  · rw [if_pos (by simp [hR.1, hR.2])]
                    simp only [doneROpen, Bool.and_eq_true, beq_iff_eq]
                    exact ⟨⟨⟨hgw, hrw⟩, hrr⟩, hrq⟩
                  · rw [if_neg (by
                      rcases hpar with ⟨rfl, -⟩ | ⟨rfl, -⟩ <;> simp_all)]
                    have hklt : k + 2 < sk.rootH := by
                      rcases hpar with ⟨hp, ho⟩ | ⟨hp, he⟩ <;>
                        · subst hp
                          simp_all
                          omega
                    have hpk2 : (p, k + 2) ∈ sk.walkKeys := by
                      refine mem_walkKeys_of sk hwf (by omega) ?_
                      rcases hpar with ⟨rfl, ho⟩ | ⟨rfl, he⟩
                      · exact Or.inl ⟨rfl, by omega⟩
                      · exact Or.inr ⟨rfl, by omega⟩
                    exact hup_done (k + 2) (by omega) (by omega)
                      _ hpk2 rfl
              have hchanA : s.chan (askedIn (p, k)) = 0 := by
                refine hchan0 _ (walk_chans_mem sk hpkh).2.1 ?_
                show sentOf sk s (Chan.asked p k)
                  = recvdOfO sk ord s (askedIn (p, k))
                rw [recvdOfO_askedIn, (hWfacts _ hpkh).2.2.2.2.2]
                by_cases hI : p = Party.I ∧ k = sk.rootH - 1
                · obtain ⟨rfl, rfl⟩ := hI
                  simp only [sentOf]
                  rw [if_pos (by simp), hiq, wf_stageLen_top sk hwf]
                  rfl
                · by_cases hR : p = Party.R ∧ k = sk.rootH - 2
                  · obtain ⟨rfl, rfl⟩ := hR
                    simp only [sentOf]
                    rw [if_neg (by simp), if_pos (by simp), hrq]
                    exact wf_rootPending sk hwf
                  · have hklt : k + 2 < sk.rootH := by
                      rcases hpar with ⟨hp, ho⟩ | ⟨hp, he⟩ <;>
                        · subst hp
                          simp_all
                          omega
                    have hpk2 : (p, k + 2) ∈ sk.walkKeys := by
                      refine mem_walkKeys_of sk hwf (by omega) ?_
                      rcases hpar with ⟨rfl, ho⟩ | ⟨rfl, he⟩
                      · exact Or.inl ⟨rfl, by omega⟩
                      · exact Or.inr ⟨rfl, by omega⟩
                    have hasked : Chan.asked p k = askedOut (p, k + 2) := by
                      unfold askedOut
                      rw [if_neg (by omega)]
                      rfl
                    rw [hasked, sentOf_askedOut hwf hpk2 (by omega),
                      (hWfacts _ hpk2).2.1, qsBefore_full hwf hklt]
              -- dispatch the frontier close on the assignment
              rcases Nat.lt_or_ge (s.walk (p, k)).phase 4 with h3 | h4
              · have hph3 : (s.walk (p, k)).phase = 3 := by omega
                cases hord : ord.walk (p, k) with
                | replyFirst =>
                    have happ : (applyO sk .impl ord
                        (.walkCloseWire (p, k)) s).isSome = true := by
                      simp [applyO, hord, hpkh, hph3, hprodW, hchanW]
                    exact canStepO_of_action sk ord
                      (walk_action_mem sk hpkh (by simp)) happ
                | queryFirst =>
                    have happ : (applyO sk .impl ord
                        (.walkCloseAsked (p, k)) s).isSome = true := by
                      simp [applyO, hord, hpkh, hph3, hprodA, hchanA]
                    exact canStepO_of_action sk ord
                      (walk_action_mem sk hpkh (by simp)) happ
              · have hph4 : (s.walk (p, k)).phase = 4 := by omega
                cases hord : ord.walk (p, k) with
                | replyFirst =>
                    have happ : (applyO sk .impl ord
                        (.walkCloseAsked (p, k)) s).isSome = true := by
                      simp [applyO, hord, hpkh, hph4, hprodA, hchanA]
                    exact canStepO_of_action sk ord
                      (walk_action_mem sk hpkh (by simp)) happ
                | queryFirst =>
                    have happ : (applyO sk .impl ord
                        (.walkCloseWire (p, k)) s).isSome = true := by
                      simp [applyO, hord, hpkh, hph4, hprodW, hchanW]
                    exact canStepO_of_action sk ord
                      (walk_action_mem sk hpkh (by simp)) happ
  rcases hdesc sk.rootH with hstep | hAllW
  · exact Or.inl hstep
  have hAllW' : ∀ pk ∈ sk.walkKeys, doneWalk (s.walk pk) = true :=
    fun pk hpk => hAllW pk hpk (by omega)
  -- the absorber's closes, dispatched on its assignment
  have hable : s.absorbPhase ≤ 5 := by
    have htop := hi.top
    simp only [topLocalOk, Bool.and_eq_true, decide_eq_true_eq] at htop
    obtain ⟨⟨⟨-, hle⟩, -⟩, -⟩ := htop
    exact hle
  by_cases hab5 : s.absorbPhase = 5
  · -- the assemblers' closes (order-blind: shared arms)
    by_cases hasm3 : ∃ pk ∈ sk.asmKeys, (s.asm pk).phase = 3
    · obtain ⟨pk, hpk, h3⟩ := hasm3
      left
      obtain ⟨p, j⟩ := pk
      obtain ⟨h1, -, -⟩ := asmKeys_bounds sk hpk
      have hasm := hi.asm (p, j) hpk
      simp only [asmLocalOk, Bool.and_eq_true, decide_eq_true_eq] at hasm
      obtain ⟨⟨⟨⟨hcur, -⟩, -⟩, -⟩, -⟩ := hasm
      rw [if_neg (by omega)] at hcur
      have hidx : (s.asm (p, j)).idx = (sk.asmResList p j).length := by
        simpa using hcur
      have hprod : producerDone sk s (asmResChan (p, j)) = true := by
        unfold asmResChan
        by_cases ha : asks p j = true
        · rw [if_pos ha]
          show producerDone sk s (Chan.upper p (j - 1)) = true
          simp only [producerDone]
          have hkey : (p, j - 1) ∈ sk.walkKeys := by
            refine mem_walkKeys_of sk hwf ?_ ?_
            · obtain ⟨-, hIb, hRb⟩ := asmKeys_bounds sk hpk
              cases p
              · have := hIb rfl
                unfold asks at ha
                simp at ha
                omega
              · have := hRb rfl
                omega
            · cases p
              · unfold asks at ha
                simp at ha
                exact Or.inl ⟨rfl, by omega⟩
              · unfold asks at ha
                simp at ha
                exact Or.inr ⟨rfl, by omega⟩
          exact hAllW' _ hkey
        · rw [if_neg ha]
          show producerDone sk s (Chan.lower p j) = true
          simp only [producerDone]
          have hkey : (p, j) ∈ sk.walkKeys := by
            refine mem_walkKeys_of sk hwf ?_ ?_
            · obtain ⟨-, hIb, hRb⟩ := asmKeys_bounds sk hpk
              cases p
              · have := hIb rfl
                unfold asks at ha
                simp at ha
                omega
              · have := hRb rfl
                omega
            · cases p
              · unfold asks at ha
                simp at ha
                exact Or.inl ⟨rfl, by omega⟩
              · unfold asks at ha
                simp at ha
                exact Or.inr ⟨rfl, by omega⟩
          exact hAllW' _ hkey
      have hchan : s.chan (asmResChan (p, j)) = 0 := by
        refine hchan0 _ (asmResChan_mem sk hwf hpk) ?_
        rw [recvdOfO_asmResChan]
        rw [recvdOf_asmRes sk (s := s) hpk]
        have hrecv : asmResRecvd s (p, j) = (sk.asmResList p j).length := by
          simp only [asmResRecvd]
          rw [if_neg (by simp; omega)]
          omega
        rw [hrecv]
        unfold asmResChan
        by_cases ha : asks p j = true
        · rw [if_pos ha]
          have hkey : (p, j - 1) ∈ sk.walkKeys := by
            refine mem_walkKeys_of sk hwf ?_ ?_
            · obtain ⟨-, hIb, hRb⟩ := asmKeys_bounds sk hpk
              cases p
              · have := hIb rfl
                unfold asks at ha
                simp at ha
                omega
              · have := hRb rfl
                omega
            · cases p
              · unfold asks at ha
                simp at ha
                exact Or.inl ⟨rfl, by omega⟩
              · unfold asks at ha
                simp at ha
                exact Or.inr ⟨rfl, by omega⟩
          have : Chan.upper p (j - 1) = upperOut (p, j - 1) := rfl
          rw [this, sentOf_upperOut, (hWfacts _ hkey).2.2.1]
          unfold Skel.asmResList
          rw [if_pos ha, List.length_map]
          show sk.stageLen ((p, j - 1).2) = (sk.scopesAt j).length
          unfold Skel.stageLen Skel.stageScopes
          rw [show (p, j - 1).2 + 1 = j from by omega]
        · rw [if_neg ha]
          have hkey : (p, j) ∈ sk.walkKeys := by
            refine mem_walkKeys_of sk hwf ?_ ?_
            · obtain ⟨-, hIb, hRb⟩ := asmKeys_bounds sk hpk
              cases p
              · have := hIb rfl
                unfold asks at ha
                simp at ha
                omega
              · have := hRb rfl
                omega
            · cases p
              · unfold asks at ha
                simp at ha
                exact Or.inl ⟨rfl, by omega⟩
              · unfold asks at ha
                simp at ha
                exact Or.inr ⟨rfl, by omega⟩
          have hjlt : j < sk.rootH := by
            obtain ⟨hkr, -⟩ := walkKeys_parity sk hwf hkey
            omega
          have : Chan.lower p j = lowerOut (p, j) := rfl
          rw [this, sentOf_lowerOut, (hWfacts _ hkey).2.2.2.1,
            answerer_resList_total hwf (by simpa using ha) h1 hjlt]
      have happ : (applyO sk .impl ord (.asmClose (p, j)) s).isSome
          = true := by
        show (Model.apply sk .impl (.asmClose (p, j)) s).isSome = true
        simp [apply, hpk, h3, hprod, hchan]
      exact canStepO_of_action sk ord (asm_action_mem sk hpk (by simp)) happ
    · -- everything is done: terminal
      right
      unfold terminal
      simp only [Bool.and_eq_true, List.all_eq_true, beq_iff_eq]
      refine ⟨⟨⟨⟨⟨⟨⟨fun pk hpk => hAllW' pk hpk, fun pk hpk => ?_⟩, ?_⟩,
        ?_⟩, ?_⟩, hfin⟩, hres⟩, hgot⟩
      · -- asm phases: ≥ 3, ≤ 4, not 3 → 4 → done
        have hasm := hi.asm pk hpk
        simp only [asmLocalOk, Bool.and_eq_true, decide_eq_true_eq] at hasm
        obtain ⟨⟨⟨⟨-, hle⟩, -⟩, -⟩, -⟩ := hasm
        have h3 : (s.asm pk).phase ≠ 3 := fun h => hasm3 ⟨pk, hpk, h⟩
        have := hasmph pk hpk
        simp [doneAsm]
        omega
      · simp [doneIOpen, hiw, hiq]
      · simp [doneROpen, hgw, hrw, hrr, hrq]
      · exact hab5
  · -- absorb phase 3 or 4: the assignment's next close is enabled
    left
    have hIkey : (Party.I, 1) ∈ sk.walkKeys :=
      mem_walkKeys_of sk hwf (by omega) (Or.inl ⟨rfl, by omega⟩)
    have hRkey : (Party.R, 0) ∈ sk.walkKeys :=
      mem_walkKeys_of sk hwf (by omega) (Or.inr ⟨rfl, by omega⟩)
    -- both O absorber counts are total past the channel work
    have habW : absorbWireRecvdO sk ord s = sk.totalLeafReqs := by
      have h1 : absorbWireRecvd sk s = sk.totalLeafReqs := by
        unfold absorbWireRecvd
        rw [if_pos (by omega)]
      have h2 : absorbAskedRecvd sk s = sk.totalLeafReqs := by
        unfold absorbAskedRecvd
        rw [if_pos (by omega)]
      cases hord : ord.absorb <;>
        simp only [absorbWireRecvdO, hord] <;> assumption
    have habA : absorbAskedRecvdO sk ord s = sk.totalLeafReqs := by
      have h1 : absorbWireRecvd sk s = sk.totalLeafReqs := by
        unfold absorbWireRecvd
        rw [if_pos (by omega)]
      have h2 : absorbAskedRecvd sk s = sk.totalLeafReqs := by
        unfold absorbAskedRecvd
        rw [if_pos (by omega)]
      cases hord : ord.absorb <;>
        simp only [absorbAskedRecvdO, hord] <;> assumption
    have hrwO : recvdOfO sk ord s (Chan.wire Party.R 0)
        = absorbWireRecvdO sk ord s := by
      have hne : (0 == sk.rootH) = false := by
        simp
        omega
      simp [recvdOfO, hne]
    have hrqO : recvdOfO sk ord s Chan.leafRequests
        = absorbAskedRecvdO sk ord s := rfl
    -- the leaf wire's producer done and channel drained
    have hprodRW : producerDone sk s (Chan.wire Party.R 0) = true := by
      simp only [producerDone]
      rw [if_neg (by simp; omega)]
      exact hAllW' _ hRkey
    have hchanRW : s.chan (Chan.wire Party.R 0) = 0 := by
      have hwr0mem : Chan.wire Party.R 0 ∈ allChans sk := by
        have : Chan.wire Party.R 0 = wireOut (Party.R, 0) := rfl
        rw [this]
        exact (walk_chans_mem sk hRkey).1
      refine hchan0 _ hwr0mem ?_
      have hs : Chan.wire Party.R 0 = wireOut (Party.R, 0) := rfl
      conv => lhs; rw [hs]
      rw [sentOf_wireOut hRkey, (hWfacts _ hRkey).1]
      show sk.wiresBefore 0 (sk.stageLen 0)
        = recvdOfO sk ord s (Chan.wire Party.R 0)
      rw [hrwO, habW, wiresBefore_full_leaf hwf]
    -- the leaf-request producer done and channel drained
    have hprodLR : producerDone sk s Chan.leafRequests = true := by
      simp only [producerDone]
      exact hAllW' _ hIkey
    have hchanLR : s.chan Chan.leafRequests = 0 := by
      refine hchan0 _ (root_chans_mem sk).2.2.1 ?_
      have hs : Chan.leafRequests = askedOut (Party.I, 1) := by
        unfold askedOut
        rw [if_pos (by simp)]
      conv => lhs; rw [hs]
      rw [sentOf_askedOut hwf hIkey (by omega), (hWfacts _ hIkey).2.1]
      show sk.qsBefore 1 (sk.stageLen 1)
        = recvdOfO sk ord s Chan.leafRequests
      rw [hrqO, habA, qsBefore_full_leaf hwf]
    rcases Nat.lt_or_ge s.absorbPhase 4 with h3 | h4
    · have hph3 : s.absorbPhase = 3 := by omega
      cases hord : ord.absorb with
      | replyFirst =>
          have happ : (applyO sk .impl ord .absorbCloseWire s).isSome
              = true := by
            simp [applyO, hord, hph3, hprodRW, hchanRW]
          exact canStepO_of_action sk ord (fixed_action_mem sk (by simp))
            happ
      | queryFirst =>
          have happ : (applyO sk .impl ord .absorbCloseAsked s).isSome
              = true := by
            simp [applyO, hord, hph3, hprodLR, hchanLR]
          exact canStepO_of_action sk ord (fixed_action_mem sk (by simp))
            happ
    · have hph4 : s.absorbPhase = 4 := by omega
      cases hord : ord.absorb with
      | replyFirst =>
          have happ : (applyO sk .impl ord .absorbCloseAsked s).isSome
              = true := by
            simp [applyO, hord, hph4, hprodLR, hchanLR]
          exact canStepO_of_action sk ord (fixed_action_mem sk (by simp))
            happ
      | queryFirst =>
          have happ : (applyO sk .impl ord .absorbCloseWire s).isSome
              = true := by
            simp [applyO, hord, hph4, hprodRW, hchanRW]
          exact canStepO_of_action sk ord (fixed_action_mem sk (by simp))
            happ

-- ================================== the pillar and opener mirrors
-- Commits and chooses are shared arms of `applyO` (the catch-all is
-- `Model.apply` definitionally), so the base choosability facts
-- conclude O enabledness directly.

/-- A phase-2 uncommitted walk can always commit under the assignment:
the base pillar's choosable obligation fires through the shared
`walkCommit` arm (cf. `Model.walk_uncommitted_canStep`). -/
theorem walk_uncommitted_canStepO {ax : AxMode} {s : State}
    {pk : Party × Nat} (hwf : sk.wellFormed = true)
    (hi : InvL sk ax s) (hpk : pk ∈ sk.walkKeys)
    (hph : (s.walk pk).phase = 2) (hco : (s.walk pk).committed = none)
    (hmode : ax.d5 = false ∨ ax.d6 = false) :
    canStepO sk ax ord s = true := by
  obtain ⟨o, hch, hmem⟩ :=
    walk_uncommitted_choosable hwf hi hpk hph hco hmode
  have happ : (applyO sk ax ord (.walkCommit pk o) s).isSome = true := by
    show (Model.apply sk ax (.walkCommit pk o) s).isSome = true
    simp [apply, hpk, hch]
  exact canStepO_of_action sk ord hmem happ

/-- An unfinished initiator opening at its choice point can always
commit under the assignment (cf. `Model.iopen_unchosen_canStep`). -/
theorem iopen_unchosen_canStepO {ax : AxMode} {s : State}
    (hnd : doneIOpen s = false) (hch : s.iopenCh = none) :
    canStepO sk ax ord s = true := by
  rw [doneIOpen, Bool.and_eq_false_iff] at hnd
  cases hw : s.iopenWire with
  | false =>
      have happ : (applyO sk ax ord (.iopenChoose .wire) s).isSome
          = true := by
        show (Model.apply sk ax (.iopenChoose .wire) s).isSome = true
        simp [apply, hch, iopenChoosable, hw]
      exact canStepO_of_action sk ord iopenChoose_mem happ
  | true =>
      have hq : s.iopenQuery = false := by
        rcases hnd with h | h
        · rw [hw] at h; cases h
        · exact h
      have happ : (applyO sk ax ord (.iopenChoose .query) s).isSome
          = true := by
        show (Model.apply sk ax (.iopenChoose .query) s).isSome = true
        simp [apply, hch, iopenChoosable, hq, hw]
      exact canStepO_of_action sk ord iopenChoose_mem happ

/-- An unfinished responder opening past its wire receive can always
commit under the assignment (cf. `Model.ropen_unchosen_canStep`). -/
theorem ropen_unchosen_canStepO {ax : AxMode} {s : State}
    (hi : InvL sk ax s) (hgw : s.ropenGotWire = true)
    (hnd : doneROpen sk s = false) (hch : s.ropenCh = none) :
    canStepO sk ax ord s = true := by
  cases hw : s.ropenWire with
  | false =>
      have happ : (applyO sk ax ord (.ropenChoose .wire) s).isSome
          = true := by
        show (Model.apply sk ax (.ropenChoose .wire) s).isSome = true
        simp [apply, hch, ropenChoosable, hgw, hw]
      exact canStepO_of_action sk ord ropenChoose_mem happ
  | true =>
      cases hr : s.ropenRes with
      | false =>
          have happ : (applyO sk ax ord (.ropenChoose .res) s).isSome
              = true := by
            show (Model.apply sk ax (.ropenChoose .res) s).isSome = true
            simp [apply, hch, ropenChoosable, hgw, hr, hw]
          exact canStepO_of_action sk ord ropenChoose_mem happ
      | true =>
          have htop := hi.top
          simp only [topLocalOk, Bool.and_eq_true, decide_eq_true_eq]
            at htop
          obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨-, hqle⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩ := htop
          rw [doneROpen, hgw, hw, hr] at hnd
          simp only [Bool.true_and] at hnd
          have hqlt : s.ropenQ < (sk.scope 0).kids.length := by
            have : ¬ (s.ropenQ = (sk.scope 0).kids.length) := by
              intro heq
              rw [heq] at hnd
              simp at hnd
            have hle : s.ropenQ ≤ (sk.scope 0).kids.length := hqle
            omega
          have happ : (applyO sk ax ord (.ropenChoose .query) s).isSome
              = true := by
            show (Model.apply sk ax (.ropenChoose .query) s).isSome = true
            simp [apply, hch, ropenChoosable, hgw, hr, hw, hqlt]
          exact canStepO_of_action sk ord ropenChoose_mem happ

-- ============================================ the top-level theorems

/-- The O progress engine: an `InvPWO`-satisfying, non-terminal state
can always step under the assignment — the argmin over `scheduleO`
(cf. `Sched.progress_of_inv`).

The τ-least O pending event fires, or its channel guard manufactures
an earlier O-unperformed event through the schedule's E1/E2 counting
edges, contradicting minimality; with no pending events at all the
closes cascade (`close_cascadeO`) to a step or to `terminal`. Stated
over the weak O invariant — conservation WITHOUT the capacity half —
so the wide-capacity chain can consume it unchanged. -/
theorem progress_of_invO (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) {s : State}
    (hi : InvPWO sk .impl ord s) (hnt : terminal sk s = false) :
    canStepO sk .impl ord s = true := by
  -- choice points first: the pillar and the opener mirrors
  by_cases hwkc : ∃ pk ∈ sk.walkKeys,
      (s.walk pk).phase = 2 ∧ (s.walk pk).committed = none
  · obtain ⟨pk, hpk, h2, hn⟩ := hwkc
    exact walk_uncommitted_canStepO sk ord hwf hi.local hpk h2 hn
      (Or.inl rfl)
  have hwkh : ∀ pk ∈ sk.walkKeys,
      ¬((s.walk pk).phase = 2 ∧ (s.walk pk).committed = none) :=
    fun pk hpk h => hwkc ⟨pk, hpk, h⟩
  by_cases hioc : s.iopenCh = none ∧ doneIOpen s = false
  · exact iopen_unchosen_canStepO sk ord hioc.2 hioc.1
  have hioh : s.iopenCh = none → doneIOpen s = true := by
    intro h
    cases hd : doneIOpen s with
    | false => exact absurd ⟨h, hd⟩ hioc
    | true => rfl
  by_cases hroc : s.ropenGotWire = true ∧ s.ropenCh = none
      ∧ doneROpen sk s = false
  · exact ropen_unchosen_canStepO sk ord hi.local hroc.1 hroc.2.2 hroc.2.1
  have hroh : s.ropenGotWire = true → s.ropenCh = none →
      doneROpen sk s = true := by
    intro hg hc
    cases hd : doneROpen sk s with
    | false => exact absurd ⟨hg, hc, hd⟩ hroc
    | true => rfl
  -- the canonical projections of the O schedule
  have hcanon : ∀ c b, proj c b (scheduleO sk ord)
      = canon c b (proj c b (scheduleO sk ord)).length := by
    intro c b
    obtain ⟨m, hm⟩ := scheduleO_proj_canon sk ord hwf c b
    rw [hm]
    congr 1
    unfold canon
    rw [List.length_map, List.length_range]
  cases hp : pendsO sk ord s with
  | nil =>
      -- no channel work remains: the closes cascade to terminal
      have hnil := hp
      unfold pendsO at hnil
      rw [List.append_eq_nil_iff, List.append_eq_nil_iff,
        List.append_eq_nil_iff, List.append_eq_nil_iff,
        List.append_eq_nil_iff, List.append_eq_nil_iff] at hnil
      obtain ⟨⟨⟨⟨⟨⟨hio0, hro0⟩, hwk0⟩, hab0⟩, hasm0⟩, hrr0⟩, hfin0⟩ := hnil
      have hIOd : doneIOpen s = true := by
        refine hioh ?_
        cases hc : s.iopenCh with
        | none => rfl
        | some o =>
            rw [ioPend] at hio0
            rw [hc] at hio0
            cases o <;> cases hio0
      have hgw : s.ropenGotWire = true := by
        cases hg : s.ropenGotWire with
        | true => rfl
        | false =>
            rw [roPend, if_pos hg] at hro0
            cases hro0
      have hROd : doneROpen sk s = true := by
        refine hroh hgw ?_
        cases hc : s.ropenCh with
        | none => rfl
        | some o =>
            rw [roPend, if_neg (by rw [hgw]; simp), hc] at hro0
            cases o <;> cases hro0
      have hwkph : ∀ pk ∈ sk.walkKeys, 3 ≤ (s.walk pk).phase := by
        intro pk hpk
        have h0 := List.flatMap_eq_nil_iff.1 hwk0 pk hpk
        by_cases hph0 : (s.walk pk).phase = 0
        · rw [wkPendO, if_pos hph0] at h0
          cases hordw : ord.walk pk <;> simp [hordw] at h0
        by_cases hph1 : (s.walk pk).phase = 1
        · rw [wkPendO, if_neg (by omega), if_pos hph1] at h0
          cases hordw : ord.walk pk <;> simp [hordw] at h0
        by_cases hph2 : (s.walk pk).phase = 2
        · cases hcm : (s.walk pk).committed with
          | none => exact absurd ⟨hph2, hcm⟩ (hwkh pk hpk)
          | some o =>
              rw [wkPendO, if_neg (by omega), if_neg (by omega),
                if_pos hph2, hcm] at h0
              cases o <;> cases h0
        omega
      have habph : 3 ≤ s.absorbPhase := by
        by_cases h0 : s.absorbPhase = 0
        · rw [abPendO, if_pos h0] at hab0
          cases horda : ord.absorb <;> simp [horda] at hab0
        by_cases h1 : s.absorbPhase = 1
        · rw [abPendO, if_neg (by omega), if_pos h1] at hab0
          cases horda : ord.absorb <;> simp [horda] at hab0
        by_cases h2 : s.absorbPhase = 2
        · rw [abPendO, if_neg (by omega), if_neg (by omega),
            if_pos h2] at hab0
          cases hab0
        omega
      have hasmph : ∀ pk ∈ sk.asmKeys, 3 ≤ (s.asm pk).phase := by
        intro pk hpk
        have h0 := List.flatMap_eq_nil_iff.1 hasm0 pk hpk
        by_cases hph0 : (s.asm pk).phase = 0
        · rw [asmPend, if_pos hph0] at h0
          cases h0
        by_cases hph1 : (s.asm pk).phase = 1
        · rw [asmPend, if_neg (by omega), if_pos hph1] at h0
          cases h0
        by_cases hph2 : (s.asm pk).phase = 2
        · rw [asmPend, if_neg (by omega), if_neg (by omega),
            if_pos hph2] at h0
          cases h0
        omega
      have hfin : s.ifin = true := by
        cases hf : s.ifin with
        | true => rfl
        | false =>
            rw [rrPend, if_pos hf] at hrr0
            cases hrr0
      have hres : s.rfinGotRes = true := by
        cases hf : s.rfinGotRes with
        | true => rfl
        | false =>
            rw [finPend, if_pos hf] at hfin0
            cases hfin0
      have hgot : s.rfinGot = sk.rootPending := by
        have htop := hi.top
        simp only [topLocalOk, Bool.and_eq_true, decide_eq_true_eq]
          at htop
        obtain ⟨-, hgle⟩ := htop
        by_cases hlt : s.rfinGot < sk.rootPending
        · exfalso
          rw [finPend, if_neg (by rw [hres]; simp),
            if_pos (by exact hlt)] at hfin0
          cases hfin0
        · omega
      rcases close_cascadeO sk ord hwf hi (by simpa using hIOd)
          (by simpa using hROd) hwkph habph hasmph hfin hres hgot with
        hstep | hterm
      · exact hstep
      · rw [hterm] at hnt
        cases hnt
  | cons fa0 rest =>
      -- the τ-least O pending event fires
      obtain ⟨fa, hfam, hfmin⟩ := exists_min_image
        (fun fa : Ev × Action => evIdx fa.1 (scheduleO sk ord))
        (l := pendsO sk ord s) (by rw [hp]; simp)
      obtain ⟨hok, T, pre, suf, hT, hdec, hpre⟩ :=
        pends_soundO sk ord hwf hi.local hioh hroh hwkh fa hfam
      have hfsched : fa.1 ∈ scheduleO sk ord := by
        have hmemT : fa.1 ∈ T := by
          rw [hdec]
          exact List.mem_append.mpr (.inr (List.mem_cons_self ..))
        exact (trace_sublistO' sk ord hwf hm0 T hT).mem hmemT
      have hτget := evIdx_getElem? hfsched
      obtain ⟨⟨c, b, n⟩, a⟩ := fa
      have hflow := hi.flow c hok.chan_mem
      have hseq := hok.seq
      cases b with
      | true =>
          rw [if_pos rfl] at hseq
          have hseq2 : n = sentOf sk s c := hseq
          clear hseq
          by_cases hroom : s.chan c < sk.cap c
          · exact canStepO_of_action sk ord hok.act
              (hok.fire (by rw [if_pos rfl]; exact hroom))
          · exfalso
            have hE2 := scheduleO_e2 sk ord
              (evIdx ((c, true, n) : Ev) (scheduleO sk ord)) c n hτget
            have hrcvlt : rcvCount c ((scheduleO sk ord).take
                (evIdx ((c, true, n) : Ev) (scheduleO sk ord)))
                > recvdOfO sk ord s c := by
              omega
            obtain ⟨j, hjlt, hjget⟩ :=
              mem_take_rcv (hcanon c false) hrcvlt
            have hgmem : ((c, false, recvdOfO sk ord s c) : Ev)
                ∈ scheduleO sk ord :=
              List.mem_iff_getElem?.2 ⟨j, hjget⟩
            have hgnp : ¬ performedO sk ord s
                (c, false, recvdOfO sk ord s c) := by
              unfold performedO
              rw [if_neg (by simp)]
              show ¬(recvdOfO sk ord s c < recvdOfO sk ord s c)
              omega
            obtain ⟨fa', hfam', hτle⟩ := pends_coverO sk ord hwf hm0
              hi.local hioh hroh hwkh hgmem hgnp
            have hjeq : j = evIdx ((c, false, recvdOfO sk ord s c) : Ev)
                (scheduleO sk ord) :=
              evIdx_unique (scheduleO_count_le_oneO sk ord hwf _) hjget
            have hmin' : evIdx ((c, true, n) : Ev) (scheduleO sk ord)
                ≤ evIdx fa'.1 (scheduleO sk ord) := hfmin fa' hfam'
            have hchain : evIdx ((c, true, n) : Ev) (scheduleO sk ord)
                ≤ j :=
              calc evIdx ((c, true, n) : Ev) (scheduleO sk ord)
                  ≤ evIdx fa'.1 (scheduleO sk ord) := hmin'
                _ ≤ evIdx ((c, false, recvdOfO sk ord s c) : Ev)
                    (scheduleO sk ord) := hτle
                _ = j := hjeq.symm
            omega
      | false =>
          rw [if_neg (by simp)] at hseq
          have hseq2 : n = recvdOfO sk ord s c := hseq
          clear hseq
          by_cases hdata : 0 < s.chan c
          · exact canStepO_of_action sk ord hok.act
              (hok.fire (by rw [if_neg (by simp)]; exact hdata))
          · exfalso
            have hE1 := scheduleO_e1 sk ord
              (evIdx ((c, false, n) : Ev) (scheduleO sk ord)) c n hτget
            have hsndlt : sndCount c ((scheduleO sk ord).take
                (evIdx ((c, false, n) : Ev) (scheduleO sk ord)))
                > sentOf sk s c := by
              omega
            obtain ⟨j, hjlt, hjget⟩ :=
              mem_take_snd (hcanon c true) hsndlt
            have hgmem : ((c, true, sentOf sk s c) : Ev)
                ∈ scheduleO sk ord :=
              List.mem_iff_getElem?.2 ⟨j, hjget⟩
            have hgnp : ¬ performedO sk ord s (c, true, sentOf sk s c) := by
              unfold performedO
              rw [if_pos rfl]
              show ¬(sentOf sk s c < sentOf sk s c)
              omega
            obtain ⟨fa', hfam', hτle⟩ := pends_coverO sk ord hwf hm0
              hi.local hioh hroh hwkh hgmem hgnp
            have hjeq : j = evIdx ((c, true, sentOf sk s c) : Ev)
                (scheduleO sk ord) :=
              evIdx_unique (scheduleO_count_le_oneO sk ord hwf _) hjget
            have hmin' : evIdx ((c, false, n) : Ev) (scheduleO sk ord)
                ≤ evIdx fa'.1 (scheduleO sk ord) := hfmin fa' hfam'
            have hchain : evIdx ((c, false, n) : Ev) (scheduleO sk ord)
                ≤ j :=
              calc evIdx ((c, false, n) : Ev) (scheduleO sk ord)
                  ≤ evIdx fa'.1 (scheduleO sk ord) := hmin'
                _ ≤ evIdx ((c, true, sentOf sk s c) : Ev)
                    (scheduleO sk ord) := hτle
                _ = j := hjeq.symm
            omega

/-- The O progress lemma at `ReachableO` states: the O invariant holds
by `invO_reachable`, and `progress_of_invO` does the rest. -/
theorem progressO (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) {s : State}
    (hr : ReachableO sk .impl ord s) (hnt : terminal sk s = false) :
    canStepO sk .impl ord s = true :=
  progress_of_invO sk ord hwf hm0 (invO_reachable hwf hr).weak hnt

-- ============================================= run-level reachability

/-- A successful O run from `init` ends at a `ReachableO` state (cf.
`Model.run_reachable`). -/
theorem runO_reachableO (ax : AxMode) {acts : List Action} {s' : State}
    (h : runO sk ax ord (init sk) acts = some s') :
    ReachableO sk ax ord s' := by
  suffices general : ∀ (acts : List Action) (s s' : State),
      ReachableO sk ax ord s → runO sk ax ord s acts = some s' →
      ReachableO sk ax ord s' by
    exact general acts _ _ (.init) h
  intro acts
  induction acts with
  | nil =>
      intro s s' hr hrun
      simp only [runO, Option.some.injEq] at hrun
      exact hrun ▸ hr
  | cons a rest ih =>
      intro s s' hr hrun
      unfold runO at hrun
      cases happ : applyO sk ax ord a s with
      | none => simp [happ] at hrun
      | some s₁ =>
          exact ih s₁ s' (.step a hr happ) (by simpa [happ] using hrun)

/-- `firstM` over `Option` succeeds only through one of its elements (a
private twin of Proofs/Termination.lean's `firstM_eq_some`, which is
not exported). -/
private theorem firstM_eq_some {α β : Type _} {f : α → Option β} {b : β} :
    ∀ {l : List α}, l.firstM f = some b → ∃ a ∈ l, f a = some b := by
  intro l
  induction l with
  | nil => intro h; simp [List.firstM] at h
  | cons x xs ih =>
      intro h
      cases hfx : f x with
      | some b' =>
          simp [List.firstM, hfx] at h
          exact ⟨x, List.mem_cons_self .., by rw [hfx, h]⟩
      | none =>
          simp [List.firstM, hfx] at h
          obtain ⟨a, ha, hfa⟩ := ih h
          exact ⟨a, List.mem_cons_of_mem x ha, hfa⟩

/-- The greedy O drain preserves `ReachableO`: every step it takes is
the application of some enabled `applyO` action (cf.
`Control.drain_reachable`). -/
theorem drainO_reachableO (ax : AxMode) (fuel : Nat) :
    ∀ {s : State}, ReachableO sk ax ord s →
      ReachableO sk ax ord (drainO sk ax ord fuel s) := by
  induction fuel with
  | zero => intro s h; exact h
  | succ n ih =>
      intro s h
      unfold drainO
      cases hf : (allActions sk).firstM (fun a => applyO sk ax ord a s) with
      | none => exact h
      | some s' =>
          obtain ⟨a, -, ha⟩ := firstM_eq_some hf
          exact ih (.step a h ha)

end StreamingMirror.Ord

-- ================================================== the flagships

namespace StreamingMirror.Sched

open Model
open Ord

variable (sk : Skel)

/-- THE order-indifference metatheorem: the shipping encoder's send
order is deadlock-free at the shipping capacities under EVERY
assignment of the two-point per-loop dequeue-order class.

The class (MODEL.md §5's dequeue-order subsection, the claim of
record): each pairing loop — each walk stage, and the absorber —
independently dequeues its per-scope wire reply and queued query in
one of exactly TWO orders, reply-first (the baseline transcription,
`OrdMap.rf`, definitionally `Model.apply`) or query-first (the
shipping Rust's order, `OrdMap.qf`), with the end-of-stream close
order TIED to the loop's prologue choice. This is per-loop
dequeue-order indifference over that two-point class, NOT
arbitrary-order indifference — loop shapes outside the class (racing
both queues, cross-scope prefetching, prologues interleaved with
sends) are named and excluded in MODEL.md's amendment, and nothing is
claimed for them.

Mode and hypotheses are the base flagship's (`Sched.deadlock_free`):
`AxMode.impl` — the `d6`/epilogue ledger interface, the per-walk
publication order the Rust encoder actually has — and margin-0
capacity (assembler capacity at least every scope's dispute count,
the shipping `FAN ≥ kids` discipline), which subsumes `schedulable`.
The capacity hypothesis is load-bearing at every assignment: the
greedy query-first drain of the sub-margin `Control.pdelay` sticks
(Ord/Statement.lean's negative control). At `ord = .rf` this
statement coincides with the base flagship (`deadlock_free_rf_corner`
below pins the round trip). -/
theorem deadlock_free_anyOrder (ord : OrdMap)
    (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) :
    DeadlockFreeO sk .impl ord := by
  intro s hr
  unfold Ord.stuckO
  cases ht : terminal sk s with
  | true => simp
  | false =>
      rw [progressO sk ord hwf hm0 hr ht]
      simp

/-- A maximal O run from `init` — one whose final state admits no
`applyO` step — ends `terminal`, at every assignment of the two-point
class (cf. `Model.maximal_run_terminal`; composes `rho_decreasesO`'s
run bound with the flagship). -/
theorem maximal_run_terminal_anyOrder (ord : OrdMap)
    (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) {acts : List Action}
    {s' : State} (hrun : runO sk .impl ord (init sk) acts = some s')
    (hmax : canStepO sk .impl ord s' = false) :
    terminal sk s' = true := by
  have hr := runO_reachableO sk ord .impl hrun
  have hdf := deadlock_free_anyOrder sk ord hwf hm0 s' hr
  unfold Ord.stuckO at hdf
  rw [hmax] at hdf
  simpa using hdf

/-- The constructive package at every assignment: the greedy O drain
with fuel ρ(init) reaches `terminal` on every well-formed margin-0
skeleton — termination with an explicit fuel bound, no fairness
hypothesis anywhere (cf. `Model.greedy_run_terminal`; composes
`drain_quiescentO` with the flagship). -/
theorem greedy_run_terminal_anyOrder (ord : OrdMap)
    (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) :
    terminal sk
      (drainO sk .impl ord (rho sk (init sk)) (init sk)) = true := by
  have hq := drain_quiescentO sk .impl ord (rho sk (init sk)) (init sk)
    (asmLevelsOk_init sk) (Nat.le_refl _)
  have hr := drainO_reachableO sk ord .impl (rho sk (init sk))
    (ReachableO.init)
  have hdf := deadlock_free_anyOrder sk ord hwf hm0 _ hr
  unfold Ord.stuckO at hdf
  rw [hq] at hdf
  simpa using hdf

/-- The reply-first transport corner: the metatheorem at `OrdMap.rf`
re-derives the base flagship's claim through `deadlockFreeO_rf_iff`.
This coincides with `Sched.deadlock_free` (Proofs/EndgameE.lean) — an
internal consistency pin that the quantified statement's reply-first
instance IS the model of record's theorem, not a new claim. -/
theorem deadlock_free_rf_corner (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) :
    StreamingMirror.DeadlockFree sk AxMode.impl :=
  (deadlockFreeO_rf_iff sk .impl).mp
    (deadlock_free_anyOrder sk .rf hwf hm0)

end StreamingMirror.Sched
