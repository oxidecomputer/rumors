/-
The endgame (PROGRESS.md §7 items 5–6, closing): the τ-least pending
event of a reachable non-terminal state is enabled, so the system can
always step — `progress`, and with it `deadlock_free`.

# Shape

Every determined process holds a pending event whose seq is its
channel's current count (`Pending.lean`); choice points are owned by
the pillar and the opener mirrors. Rank the pending events by position
in the canonical schedule (τ, well-defined by merge completeness and
τ injectivity) and take the least, `e*`. If `e*` is a starving
receive, E1 in the schedule puts its matching send strictly τ-below —
that send is unperformed (its seq is its channel's current count), so
its owner's pending head sits τ-below `e*`: contradiction. A jammed
send is symmetric through E2 against the receive its cap window
awaits. So `e*`'s channel guard is open and its owner's action fires.

With no pending events at all, every process is past its channel work
and only closes remain: closes cascade from the openers down (the
producer of every close target is done, and its channel is drained by
flow conservation against the supply = demand totals), ending at
`terminal` — contradicting non-terminality.

Chain (d5, stage E): consumes Pending.lean, `merge_complete`, and the
pillar; concludes `progress_d5` and `deadlock_free_d5` (the
Statement.lean counterpart). E mirror: EndgameE.lean. Map:
Proofs/Map.lean.
-/
import StreamingMirror.Proofs.Pending
import StreamingMirror.Statement

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

/-- Every pending event of the state, across all processes. -/
def pends (s : State) : List (Ev × Action) :=
  ioPend sk s ++ roPend sk s
    ++ sk.walkKeys.flatMap (wkPend sk s)
    ++ abPend s
    ++ sk.asmKeys.flatMap (asmPend sk s)
    ++ rrPend s ++ finPend sk s

/-- A walkOrder entry is a walk key. -/
theorem walkOrder_mem_keys (hwf : sk.wellFormed = true) {i : Nat}
    (hi : i < sk.rootH) :
    ((if (sk.rootH - 1 - i) % 2 == 1 then Party.I else Party.R),
      sk.rootH - 1 - i) ∈ sk.walkKeys := by
  have hev : sk.rootH % 2 = 0 := (wf_rootH hwf).1
  by_cases hpar : (sk.rootH - 1 - i) % 2 = 1
  · rw [if_pos (by simp [hpar])]
    exact mem_walkKeys_of sk hwf (by omega) (Or.inl ⟨rfl, hpar⟩)
  · rw [if_neg (by simp [hpar])]
    exact mem_walkKeys_of sk hwf (by omega) (Or.inr ⟨rfl, by omega⟩)

/-- The per-family split of a merge-input trace. -/
theorem procs_cases {T : List Ev} (hT : T ∈ procs sk) :
    T = iopenEvents sk ∨ T = ropenEvents sk
    ∨ (∃ i, i < sk.rootH ∧ T = walkEvents sk
        ((if (sk.rootH - 1 - i) % 2 == 1 then Party.I else Party.R),
          sk.rootH - 1 - i))
    ∨ T = absorbEvents sk
    ∨ (∃ pk ∈ sk.asmKeys, T = asmEvents sk pk)
    ∨ T = [(Chan.rootret, false, 0)] ∨ T = finEvents sk := by
  simp only [procs, List.mem_append, List.mem_cons, List.mem_map,
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

/-- The fixed family traces are merge inputs. -/
theorem fixed_mem_procs :
    iopenEvents sk ∈ procs sk ∧ ropenEvents sk ∈ procs sk
    ∧ absorbEvents sk ∈ procs sk
    ∧ [((Chan.rootret, false, 0) : Ev)] ∈ procs sk
    ∧ finEvents sk ∈ procs sk := by
  refine ⟨?_, ?_, ?_, ?_, ?_⟩ <;> simp [procs]

/-- Family pending lists inject into the state's pending pool. -/
theorem pends_lift {s : State} :
    (∀ fa ∈ ioPend sk s, fa ∈ pends sk s)
    ∧ (∀ fa ∈ roPend sk s, fa ∈ pends sk s)
    ∧ (∀ pk ∈ sk.walkKeys, ∀ fa ∈ wkPend sk s pk, fa ∈ pends sk s)
    ∧ (∀ fa ∈ abPend s, fa ∈ pends sk s)
    ∧ (∀ pk ∈ sk.asmKeys, ∀ fa ∈ asmPend sk s pk, fa ∈ pends sk s)
    ∧ (∀ fa ∈ rrPend s, fa ∈ pends sk s)
    ∧ (∀ fa ∈ finPend sk s, fa ∈ pends sk s) := by
  unfold pends
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

/-- Soundness of the pool: every pending entry is `PendOk` and sits at
its trace's performed frontier. -/
theorem pends_sound (hwf : sk.wellFormed = true) {s : State}
    (hi : InvP sk .full s)
    (hioh : s.iopenCh = none → doneIOpen s = true)
    (hroh : s.ropenGotWire = true → s.ropenCh = none →
      doneROpen sk s = true)
    (hwkh : ∀ pk ∈ sk.walkKeys,
      ¬((s.walk pk).phase = 2 ∧ (s.walk pk).committed = none)) :
    ∀ fa ∈ pends sk s, PendOk sk s fa.1 fa.2
      ∧ ∃ T pre suf, T ∈ procs sk ∧ T = pre ++ fa.1 :: suf
        ∧ ∀ e ∈ pre, performed sk s e := by
  intro fa hfa
  unfold pends at hfa
  rcases List.mem_append.1 hfa with hfa | hfin
  rcases List.mem_append.1 hfa with hfa | hrr
  rcases List.mem_append.1 hfa with hfa | hasm
  rcases List.mem_append.1 hfa with hfa | hab
  rcases List.mem_append.1 hfa with hfa | hwk
  rcases List.mem_append.1 hfa with hio | hro
  · rcases iopen_pend_or_done sk hwf hi hioh with ⟨-, hnil⟩ | h
    · rw [hnil] at hio; cases hio
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      rw [heq, List.mem_singleton] at hio
      subst hio
      exact ⟨hok, iopenEvents sk, pre, suf, (fixed_mem_procs sk).1,
        hdec, hpre⟩
  · rcases ropen_pend_or_done sk hwf hi hroh with ⟨-, hnil⟩ | h
    · rw [hnil] at hro; cases hro
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      rw [heq, List.mem_singleton] at hro
      subst hro
      exact ⟨hok, ropenEvents sk, pre, suf, (fixed_mem_procs sk).2.1,
        hdec, hpre⟩
  · obtain ⟨pk, hpk, hfa⟩ := List.mem_flatMap.1 hwk
    rcases walk_pend_or_done sk hwf hi hpk (hwkh pk hpk) with ⟨-, hnil⟩ | h
    · rw [hnil] at hfa; cases hfa
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      rw [heq, List.mem_singleton] at hfa
      subst hfa
      exact ⟨hok, walkEvents sk pk, pre, suf,
        walkEvents_mem_procs sk hwf hpk, hdec, hpre⟩
  · rcases absorb_pend_or_done sk hwf hi with ⟨-, hnil⟩ | h
    · rw [hnil] at hab; cases hab
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      rw [heq, List.mem_singleton] at hab
      subst hab
      exact ⟨hok, absorbEvents sk, pre, suf, (fixed_mem_procs sk).2.2.1,
        hdec, hpre⟩
  · obtain ⟨pk, hpk, hfa⟩ := List.mem_flatMap.1 hasm
    rcases asm_pend_or_done sk hwf hi hpk with ⟨-, hnil⟩ | h
    · rw [hnil] at hfa; cases hfa
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      rw [heq, List.mem_singleton] at hfa
      subst hfa
      exact ⟨hok, asmEvents sk pk, pre, suf,
        asmEvents_mem_procs sk hpk, hdec, hpre⟩
  · rcases rootret_pend_or_done sk (s := s) with ⟨-, hnil⟩ | h
    · rw [hnil] at hrr; cases hrr
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      rw [heq, List.mem_singleton] at hrr
      subst hrr
      exact ⟨hok, [(Chan.rootret, false, 0)], pre, suf,
        (fixed_mem_procs sk).2.2.2.1, hdec, hpre⟩
  · rcases fin_pend_or_done sk hi with ⟨-, hnil⟩ | h
    · rw [hnil] at hfin; cases hfin
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      rw [heq, List.mem_singleton] at hfin
      subst hfin
      exact ⟨hok, finEvents sk, pre, suf, (fixed_mem_procs sk).2.2.2.2,
        hdec, hpre⟩

/-- The cover: an unperformed schedule event is τ-dominated by some
pending entry — its own trace's frontier sits at or before it. -/
theorem pends_cover (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) {s : State}
    (hi : InvP sk .full s)
    (hioh : s.iopenCh = none → doneIOpen s = true)
    (hroh : s.ropenGotWire = true → s.ropenCh = none →
      doneROpen sk s = true)
    (hwkh : ∀ pk ∈ sk.walkKeys,
      ¬((s.walk pk).phase = 2 ∧ (s.walk pk).committed = none))
    {g : Ev} (hmem : g ∈ schedule sk) (hnp : ¬ performed sk s g) :
    ∃ fa ∈ pends sk s,
      evIdx fa.1 (schedule sk) ≤ evIdx g (schedule sk) := by
  obtain ⟨T, hT, hgT⟩ := sched_mem_trace sk hmem
  obtain ⟨hlio, hlro, hlwk, hlab, hlasm, hlrr, hlfin⟩ :=
    pends_lift sk (s := s)
  rcases procs_cases sk hT with rfl | hc
  · rcases iopen_pend_or_done sk hwf hi hioh with ⟨hall, -⟩ | h
    · exact absurd (hall g hgT) hnp
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      refine ⟨(f, a), hlio _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        tau_le_of_pend sk hwf hsched hT hdec hpre hgT hnp⟩
  rcases hc with rfl | hc
  · rcases ropen_pend_or_done sk hwf hi hroh with ⟨hall, -⟩ | h
    · exact absurd (hall g hgT) hnp
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      refine ⟨(f, a), hlro _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        tau_le_of_pend sk hwf hsched hT hdec hpre hgT hnp⟩
  rcases hc with ⟨i, hir, rfl⟩ | hc
  · have hpk := walkOrder_mem_keys sk hwf hir
    rcases walk_pend_or_done sk hwf hi hpk (hwkh _ hpk) with ⟨hall, -⟩ | h
    · exact absurd (hall g hgT) hnp
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      refine ⟨(f, a),
        hlwk _ hpk _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        tau_le_of_pend sk hwf hsched hT hdec hpre hgT hnp⟩
  rcases hc with rfl | hc
  · rcases absorb_pend_or_done sk hwf hi with ⟨hall, -⟩ | h
    · exact absurd (hall g hgT) hnp
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      refine ⟨(f, a), hlab _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        tau_le_of_pend sk hwf hsched hT hdec hpre hgT hnp⟩
  rcases hc with ⟨pk, hpk, rfl⟩ | hc
  · rcases asm_pend_or_done sk hwf hi hpk with ⟨hall, -⟩ | h
    · exact absurd (hall g hgT) hnp
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      refine ⟨(f, a),
        hlasm _ hpk _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        tau_le_of_pend sk hwf hsched hT hdec hpre hgT hnp⟩
  rcases hc with rfl | rfl
  · rcases rootret_pend_or_done sk (s := s) with ⟨hall, -⟩ | h
    · exact absurd (hall g hgT) hnp
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      refine ⟨(f, a), hlrr _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        tau_le_of_pend sk hwf hsched hT hdec hpre hgT hnp⟩
  · rcases fin_pend_or_done sk hi with ⟨hall, -⟩ | h
    · exact absurd (hall g hgT) hnp
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      refine ⟨(f, a), hlfin _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        tau_le_of_pend sk hwf hsched hT hdec hpre hgT hnp⟩

-- ================================================== the close cascade

/-- Root fan-out = the stage two below the root, positionally. -/
theorem wf_rootPending (hwf : sk.wellFormed = true) :
    sk.rootPending = sk.stageLen (sk.rootH - 2) := by
  have hge : 2 ≤ sk.rootH := (wf_rootH hwf).2
  have halign := wf_bfs_aligned hwf (show sk.rootH - 1 < sk.rootH by omega)
  rw [show sk.rootH - 1 + 1 = sk.rootH from by omega, wf_root_stage hwf]
    at halign
  unfold Skel.rootPending Skel.stageLen Skel.stageScopes
  rw [show sk.rootH - 2 + 1 = sk.rootH - 1 from by omega, ← halign]
  simp

/-- A walk key is determined by its height. -/
theorem walkKeys_eq_of_height (hwf : sk.wellFormed = true)
    {pk pk' : Party × Nat} (h : pk ∈ sk.walkKeys) (h' : pk' ∈ sk.walkKeys)
    (heq : pk.2 = pk'.2) : pk = pk' := by
  obtain ⟨p, k⟩ := pk
  obtain ⟨p', k'⟩ := pk'
  simp only at heq
  subst heq
  obtain ⟨-, hpar⟩ := walkKeys_parity sk hwf h
  obtain ⟨-, hpar'⟩ := walkKeys_parity sk hwf h'
  rcases hpar with ⟨rfl, ho⟩ | ⟨rfl, he⟩ <;>
    rcases hpar' with ⟨rfl, ho'⟩ | ⟨rfl, he'⟩ <;>
    first | rfl | omega

/-- With every process past its channel work, either a close fires or
the session is terminal. -/
theorem close_cascade (hwf : sk.wellFormed = true) {s : State}
    (hi : InvP sk .full s)
    (hIOd : doneIOpen s = true) (hROd : doneROpen sk s = true)
    (hwkph : ∀ pk ∈ sk.walkKeys, 3 ≤ (s.walk pk).phase)
    (habph : 3 ≤ s.absorbPhase)
    (hasmph : ∀ pk ∈ sk.asmKeys, 3 ≤ (s.asm pk).phase)
    (hfin : s.ifin = true) (hres : s.rfinGotRes = true)
    (hgot : s.rfinGot = sk.rootPending) :
    canStep sk .full s = true ∨ terminal sk s = true := by
  have hge : 2 ≤ sk.rootH := (wf_rootH hwf).2
  have hev : sk.rootH % 2 = 0 := (wf_rootH hwf).1
  simp only [doneIOpen, Bool.and_eq_true] at hIOd
  simp only [doneROpen, Bool.and_eq_true, beq_iff_eq] at hROd
  obtain ⟨hiw, hiq⟩ := hIOd
  obtain ⟨⟨⟨hgw, hrw⟩, hrr⟩, hrq⟩ := hROd
  -- the per-walk drained totals
  have hWfacts : ∀ pk ∈ sk.walkKeys,
      wkWireSent sk s pk = sk.wiresBefore pk.2 (sk.stageLen pk.2)
      ∧ wkQSentTot sk s pk = sk.qsBefore pk.2 (sk.stageLen pk.2)
      ∧ wkParentSent s pk = sk.stageLen pk.2
      ∧ wkResSent sk s pk = sk.dsBefore pk.2 (sk.stageLen pk.2)
      ∧ wkWireRecvd sk s pk = sk.stageLen pk.2
      ∧ wkAskedRecvd sk s pk = sk.stageLen pk.2 := by
    intro pk hpk
    have hph := hwkph pk hpk
    have hsc := (walk_scope_bound sk hi hpk).2 hph
    obtain ⟨hled, hpd, -⟩ := walk_ledgers_empty sk hi hpk (by omega)
    obtain ⟨hw0, hr0, hq0⟩ := counts_of_empty sk hled
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
    · unfold wkWireRecvd
      rw [if_pos (by omega)]
    · unfold wkAskedRecvd
      rw [if_pos (by omega)]
  -- drained channels, from flow at equal totals
  have hchan0 : ∀ c ∈ allChans sk, sentOf sk s c = recvdOf sk s c →
      s.chan c = 0 := by
    intro c hc heq
    have := (hi.flow c hc).1
    omega
  -- descending sweep: the highest undone walk can close
  have hdesc : ∀ d, canStep sk .full s = true
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
            have hph := hwkph pkh hpkh
            have hple : (s.walk pkh).phase ≤ 5 := by
              have hwk := hi.wk pkh hpkh
              simp only [wkLocalOk] at hwk
              rcases Bool.and_eq_true .. ▸ hwk with ⟨hcur, -⟩
              simp only [Bool.and_eq_true] at hcur
              obtain ⟨⟨-, hle⟩, -⟩ := hcur
              simpa using hle
            by_cases h5 : (s.walk pkh).phase = 5
            · right
              intro pk hpk hgep
              by_cases hup : sk.rootH - d ≤ pk.2
              · exact hdone pk hpk hup
              · have : pk = pkh :=
                  walkKeys_eq_of_height sk hwf hpk hpkh (by omega)
                rw [this]
                simp [doneWalk, h5]
            · -- phase 3 or 4: the close above is enabled
              left
              have hup_done : ∀ h2, sk.rootH - (d + 1) < h2 →
                  h2 < sk.rootH →
                  ∀ pk2 ∈ sk.walkKeys, pk2.2 = h2 →
                  doneWalk (s.walk pk2) = true := by
                intro h2 hlt2 hltr pk2 hpk2 hpk2h
                exact hdone pk2 hpk2 (by omega)
              rcases Nat.lt_or_ge (s.walk pkh).phase 4 with h3 | h4
              · -- phase 3: close the prologue wire
                have hph3 : (s.walk pkh).phase = 3 := by omega
                have hprod : producerDone sk s (wireIn pkh) = true := by
                  show producerDone sk s
                    (Chan.wire pkh.1.other (pkh.2 + 1)) = true
                  simp only [producerDone]
                  by_cases htop : pkh.2 + 1 = sk.rootH
                  · rw [if_pos (by simp [htop])]
                    have hparh : pkh.1 = Party.I := by
                      obtain ⟨p2, k2⟩ := pkh
                      rcases (walkKeys_parity sk hwf hpkh).2 with
                        ⟨hp, -⟩ | ⟨hp, he⟩
                      · exact hp
                      · exfalso
                        simp only at htop he
                        omega
                    rw [hparh]
                    show (if (Party.R == Party.I) = true then _ else _) = true
                    rw [if_neg (by simp)]
                    simp only [doneROpen, Bool.and_eq_true, beq_iff_eq]
                    exact ⟨⟨⟨hgw, hrw⟩, hrr⟩, hrq⟩
                  · rw [if_neg (by simp [htop])]
                    have hpk2 : (pkh.1.other, pkh.2 + 1) ∈ sk.walkKeys := by
                      obtain ⟨-, hpar⟩ := walkKeys_parity sk hwf hpkh
                      refine mem_walkKeys_of sk hwf (by omega) ?_
                      rcases hpar with ⟨hp, ho⟩ | ⟨hp, he⟩
                      · rw [hp]
                        exact Or.inr ⟨rfl, by omega⟩
                      · rw [hp]
                        exact Or.inl ⟨rfl, by omega⟩
                    exact hup_done (pkh.2 + 1) (by omega) (by omega)
                      _ hpk2 rfl
                have hchan : s.chan (wireIn pkh) = 0 := by
                  refine hchan0 _ (wireIn_mem_allChans sk hwf hpkh) ?_
                  show sentOf sk s (Chan.wire pkh.1.other (pkh.2 + 1))
                    = recvdOf sk s (wireIn pkh)
                  rw [recvdOf_wireIn hpkh, (hWfacts pkh hpkh).2.2.2.2.1]
                  by_cases htop : pkh.2 + 1 = sk.rootH
                  · have hparh : pkh.1 = Party.I := by
                      obtain ⟨p2, k2⟩ := pkh
                      rcases (walkKeys_parity sk hwf hpkh).2 with
                        ⟨hp, -⟩ | ⟨hp, he⟩
                      · exact hp
                      · exfalso
                        simp only at htop he
                        omega
                    rw [hparh]
                    show sentOf sk s (Chan.wire Party.R (pkh.2 + 1)) = _
                    rw [htop]
                    simp only [sentOf]
                    rw [if_pos (by simp), if_neg (by simp), hrw,
                      show pkh.2 = sk.rootH - 1 from by omega,
                      wf_stageLen_top sk hwf]
                    rfl
                  · have hpk2 : (pkh.1.other, pkh.2 + 1) ∈ sk.walkKeys := by
                      obtain ⟨-, hpar⟩ := walkKeys_parity sk hwf hpkh
                      refine mem_walkKeys_of sk hwf (by omega) ?_
                      rcases hpar with ⟨hp, ho⟩ | ⟨hp, he⟩
                      · rw [hp]
                        exact Or.inr ⟨rfl, by omega⟩
                      · rw [hp]
                        exact Or.inl ⟨rfl, by omega⟩
                    have : Chan.wire pkh.1.other (pkh.2 + 1)
                        = wireOut (pkh.1.other, pkh.2 + 1) := rfl
                    rw [this, sentOf_wireOut hpk2,
                      (hWfacts _ hpk2).1,
                      wiresBefore_full hwf (by omega)]
                have happ : (apply sk .full (.walkCloseWire pkh) s).isSome
                    = true := by
                  simp [apply, hpkh, hph3, hprod, hchan]
                exact canStep_of_action
                  (walk_action_mem sk hpkh (by simp)) happ
              · -- phase 4: close the query prologue
                have hph4 : (s.walk pkh).phase = 4 := by omega
                obtain ⟨p, k⟩ := pkh
                obtain ⟨-, hpar⟩ := walkKeys_parity sk hwf hpkh
                have hprod : producerDone sk s (askedIn (p, k)) = true := by
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
                have hchan : s.chan (askedIn (p, k)) = 0 := by
                  refine hchan0 _ (walk_chans_mem sk hpkh).2.1 ?_
                  show sentOf sk s (Chan.asked p k)
                    = recvdOf sk s (askedIn (p, k))
                  rw [recvdOf_askedIn, (hWfacts _ hpkh).2.2.2.2.2]
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
                have happ : (apply sk .full (.walkCloseAsked (p, k)) s).isSome
                    = true := by
                  simp [apply, hpkh, hph4, hprod, hchan]
                exact canStep_of_action
                  (walk_action_mem sk hpkh (by simp)) happ
  rcases hdesc sk.rootH with hstep | hAllW
  · exact Or.inl hstep
  have hAllW' : ∀ pk ∈ sk.walkKeys, doneWalk (s.walk pk) = true :=
    fun pk hpk => hAllW pk hpk (by omega)
  -- the absorber's closes
  have hable : s.absorbPhase ≤ 5 := by
    have htop := hi.top
    simp only [topLocalOk, Bool.and_eq_true, decide_eq_true_eq] at htop
    obtain ⟨⟨⟨-, hle⟩, -⟩, -⟩ := htop
    exact hle
  by_cases hab5 : s.absorbPhase = 5
  · -- the assemblers' closes
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
      have happ : (apply sk .full (.asmClose (p, j)) s).isSome = true := by
        simp [apply, hpk, h3, hprod, hchan]
      exact canStep_of_action (asm_action_mem sk hpk (by simp)) happ
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
  · -- absorb phase 3 or 4: its close is enabled
    left
    have hIkey : (Party.I, 1) ∈ sk.walkKeys :=
      mem_walkKeys_of sk hwf (by omega) (Or.inl ⟨rfl, by omega⟩)
    have hRkey : (Party.R, 0) ∈ sk.walkKeys :=
      mem_walkKeys_of sk hwf (by omega) (Or.inr ⟨rfl, by omega⟩)
    rcases Nat.lt_or_ge s.absorbPhase 4 with h3 | h4
    · have hph3 : s.absorbPhase = 3 := by omega
      have hprod : producerDone sk s (Chan.wire Party.R 0) = true := by
        simp only [producerDone]
        rw [if_neg (by simp; omega)]
        exact hAllW' _ hRkey
      have hchan : s.chan (Chan.wire Party.R 0) = 0 := by
        have hwr0mem : Chan.wire Party.R 0 ∈ allChans sk := by
          have : Chan.wire Party.R 0 = wireOut (Party.R, 0) := rfl
          rw [this]
          exact (walk_chans_mem sk hRkey).1
        refine hchan0 _ hwr0mem ?_
        have hs : Chan.wire Party.R 0 = wireOut (Party.R, 0) := rfl
        conv => lhs; rw [hs]
        rw [sentOf_wireOut hRkey, (hWfacts _ hRkey).1]
        have hne : (0 == sk.rootH) = false := by simp; omega
        show sk.wiresBefore 0 (sk.stageLen 0) = recvdOf sk s (Chan.wire Party.R 0)
        simp only [recvdOf]
        rw [if_neg (by simp [hne]), if_pos (by simp)]
        rw [wiresBefore_full_leaf hwf]
        unfold absorbWireRecvd
        rw [if_pos (by omega)]
      have happ : (apply sk .full .absorbCloseWire s).isSome = true := by
        simp [apply, hph3, hprod, hchan]
      exact canStep_of_action (fixed_action_mem sk (by simp)) happ
    · have hph4 : s.absorbPhase = 4 := by omega
      have hprod : producerDone sk s Chan.leafRequests = true := by
        simp only [producerDone]
        exact hAllW' _ hIkey
      have hchan : s.chan Chan.leafRequests = 0 := by
        refine hchan0 _ (root_chans_mem sk).2.2.1 ?_
        have hs : Chan.leafRequests = askedOut (Party.I, 1) := by
          unfold askedOut
          rw [if_pos (by simp)]
        conv => lhs; rw [hs]
        rw [sentOf_askedOut hwf hIkey (by omega), (hWfacts _ hIkey).2.1]
        show sk.qsBefore 1 (sk.stageLen 1) = recvdOf sk s Chan.leafRequests
        rw [qsBefore_full_leaf hwf]
        show sk.totalLeafReqs = absorbAskedRecvd sk s
        unfold absorbAskedRecvd
        rw [if_pos (by omega)]
      have happ : (apply sk .full .absorbCloseAsked s).isSome = true := by
        simp [apply, hph4, hprod, hchan]
      exact canStep_of_action (fixed_action_mem sk (by simp)) happ

-- ============================================ the top-level theorems

/-- The `d5`-corner progress lemma: a reachable, non-terminal state of
a well-formed, schedulable session can always step under the
parent-early ledger set (`AxMode.full`). See `deadlock_free_d5` for the
design-space framing. -/
theorem progress_d5 (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) {s : State}
    (hr : Reachable sk .full s) (hnt : terminal sk s = false) :
    canStep sk .full s = true := by
  have hi : InvP sk .full s :=
    (inv_iff sk .full s).mp (inv_reachable hwf hr)
  -- choice points first: the pillar and the opener mirrors
  by_cases hwkc : ∃ pk ∈ sk.walkKeys,
      (s.walk pk).phase = 2 ∧ (s.walk pk).committed = none
  · obtain ⟨pk, hpk, h2, hn⟩ := hwkc
    exact walk_uncommitted_canStep hwf hi.local hpk h2 hn (Or.inr rfl)
  have hwkh : ∀ pk ∈ sk.walkKeys,
      ¬((s.walk pk).phase = 2 ∧ (s.walk pk).committed = none) :=
    fun pk hpk h => hwkc ⟨pk, hpk, h⟩
  by_cases hioc : s.iopenCh = none ∧ doneIOpen s = false
  · exact iopen_unchosen_canStep hioc.2 hioc.1
  have hioh : s.iopenCh = none → doneIOpen s = true := by
    intro h
    cases hd : doneIOpen s with
    | false => exact absurd ⟨h, hd⟩ hioc
    | true => rfl
  by_cases hroc : s.ropenGotWire = true ∧ s.ropenCh = none
      ∧ doneROpen sk s = false
  · exact ropen_unchosen_canStep hi.local hroc.1 hroc.2.2 hroc.2.1
  have hroh : s.ropenGotWire = true → s.ropenCh = none →
      doneROpen sk s = true := by
    intro hg hc
    cases hd : doneROpen sk s with
    | false => exact absurd ⟨hg, hc, hd⟩ hroc
    | true => rfl
  -- the canonical projections of the schedule
  have hcanon : ∀ c b, proj c b (schedule sk)
      = canon c b (proj c b (schedule sk)).length := by
    intro c b
    obtain ⟨m, hm⟩ := schedule_proj_canon sk hwf c b
    rw [hm]
    congr 1
    unfold canon
    rw [List.length_map, List.length_range]
  cases hp : pends sk s with
  | nil =>
      -- no channel work remains: the closes cascade to terminal
      have hnil := hp
      unfold pends at hnil
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
        · rw [wkPend, if_pos hph0] at h0
          cases h0
        by_cases hph1 : (s.walk pk).phase = 1
        · rw [wkPend, if_neg (by omega), if_pos hph1] at h0
          cases h0
        by_cases hph2 : (s.walk pk).phase = 2
        · cases hcm : (s.walk pk).committed with
          | none => exact absurd ⟨hph2, hcm⟩ (hwkh pk hpk)
          | some o =>
              rw [wkPend, if_neg (by omega), if_neg (by omega),
                if_pos hph2, hcm] at h0
              cases o <;> cases h0
        omega
      have habph : 3 ≤ s.absorbPhase := by
        by_cases h0 : s.absorbPhase = 0
        · rw [abPend, if_pos h0] at hab0
          cases hab0
        by_cases h1 : s.absorbPhase = 1
        · rw [abPend, if_neg (by omega), if_pos h1] at hab0
          cases hab0
        by_cases h2 : s.absorbPhase = 2
        · rw [abPend, if_neg (by omega), if_neg (by omega),
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
      rcases close_cascade sk hwf hi (by simpa using hIOd)
          (by simpa using hROd) hwkph habph hasmph hfin hres hgot with
        hstep | hterm
      · exact hstep
      · rw [hterm] at hnt
        cases hnt
  | cons fa0 rest =>
      -- the τ-least pending event fires
      obtain ⟨fa, hfam, hfmin⟩ := exists_min_image
        (fun fa : Ev × Action => evIdx fa.1 (schedule sk))
        (l := pends sk s) (by rw [hp]; simp)
      obtain ⟨hok, T, pre, suf, hT, hdec, hpre⟩ :=
        pends_sound sk hwf hi hioh hroh hwkh fa hfam
      have hfsched : fa.1 ∈ schedule sk := by
        have hmemT : fa.1 ∈ T := by
          rw [hdec]
          exact List.mem_append.mpr (.inr (List.mem_cons_self ..))
        exact (trace_sublist sk hwf hsched hT).mem hmemT
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
          · exact canStep_of_action hok.act
              (hok.fire (by rw [if_pos rfl]; exact hroom))
          · exfalso
            have hE2 := schedule_e2 sk
              (evIdx ((c, true, n) : Ev) (schedule sk)) c n hτget
            have hrcvlt : rcvCount c ((schedule sk).take
                (evIdx ((c, true, n) : Ev) (schedule sk)))
                > recvdOf sk s c := by
              omega
            obtain ⟨j, hjlt, hjget⟩ :=
              mem_take_rcv (hcanon c false) hrcvlt
            have hgmem : ((c, false, recvdOf sk s c) : Ev)
                ∈ schedule sk :=
              List.mem_iff_getElem?.2 ⟨j, hjget⟩
            have hgnp : ¬ performed sk s (c, false, recvdOf sk s c) := by
              unfold performed
              rw [if_neg (by simp)]
              show ¬(recvdOf sk s c < recvdOf sk s c)
              omega
            obtain ⟨fa', hfam', hτle⟩ := pends_cover sk hwf hsched hi
              hioh hroh hwkh hgmem hgnp
            have hjeq : j = evIdx ((c, false, recvdOf sk s c) : Ev)
                (schedule sk) :=
              evIdx_unique (schedule_count_le_one sk hwf _) hjget
            have hmin' : evIdx ((c, true, n) : Ev) (schedule sk)
                ≤ evIdx fa'.1 (schedule sk) := hfmin fa' hfam'
            have hchain : evIdx ((c, true, n) : Ev) (schedule sk) ≤ j :=
              calc evIdx ((c, true, n) : Ev) (schedule sk)
                  ≤ evIdx fa'.1 (schedule sk) := hmin'
                _ ≤ evIdx ((c, false, recvdOf sk s c) : Ev)
                    (schedule sk) := hτle
                _ = j := hjeq.symm
            omega
      | false =>
          rw [if_neg (by simp)] at hseq
          have hseq2 : n = recvdOf sk s c := hseq
          clear hseq
          by_cases hdata : 0 < s.chan c
          · exact canStep_of_action hok.act
              (hok.fire (by rw [if_neg (by simp)]; exact hdata))
          · exfalso
            have hE1 := schedule_e1 sk
              (evIdx ((c, false, n) : Ev) (schedule sk)) c n hτget
            have hsndlt : sndCount c ((schedule sk).take
                (evIdx ((c, false, n) : Ev) (schedule sk)))
                > sentOf sk s c := by
              omega
            obtain ⟨j, hjlt, hjget⟩ :=
              mem_take_snd (hcanon c true) hsndlt
            have hgmem : ((c, true, sentOf sk s c) : Ev)
                ∈ schedule sk :=
              List.mem_iff_getElem?.2 ⟨j, hjget⟩
            have hgnp : ¬ performed sk s (c, true, sentOf sk s c) := by
              unfold performed
              rw [if_pos rfl]
              show ¬(sentOf sk s c < sentOf sk s c)
              omega
            obtain ⟨fa', hfam', hτle⟩ := pends_cover sk hwf hsched hi
              hioh hroh hwkh hgmem hgnp
            have hjeq : j = evIdx ((c, true, sentOf sk s c) : Ev)
                (schedule sk) :=
              evIdx_unique (schedule_count_le_one sk hwf _) hjget
            have hmin' : evIdx ((c, false, n) : Ev) (schedule sk)
                ≤ evIdx fa'.1 (schedule sk) := hfmin fa' hfam'
            have hchain : evIdx ((c, false, n) : Ev) (schedule sk) ≤ j :=
              calc evIdx ((c, false, n) : Ev) (schedule sk)
                  ≤ evIdx fa'.1 (schedule sk) := hmin'
                _ ≤ evIdx ((c, true, sentOf sk s c) : Ev)
                    (schedule sk) := hτle
                _ = j := hjeq.symm
            omega

/-- The `d5`-corner top-level theorem: under the parent-EARLY ledger
interface (`AxMode.full`, the weave's placement), every well-formed,
schedulable session is deadlock-free — no reachable state is stuck, at
ANY `capLevel ≥ 1`. This is the capacity-universal corner of the
parent-placement design space (design/parent-placement.md): the priced
alternative encoder discipline, NOT the shipping encoder's order. The
implementation-facing flagship theorem — deadlock freedom under
`AxMode.impl` (the `d6`/epilogue corner, the order the Rust encoder
actually has) given the margin-0 capacity hypothesis — is task #16 of
formal/PLAN.md, in progress. -/
theorem deadlock_free_d5 (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) :
    StreamingMirror.DeadlockFree sk .full := by
  intro s hr
  unfold Model.stuck
  cases ht : terminal sk s with
  | true => simp
  | false =>
      rw [progress_d5 sk hwf hsched hr ht]
      simp

end StreamingMirror.Sched
