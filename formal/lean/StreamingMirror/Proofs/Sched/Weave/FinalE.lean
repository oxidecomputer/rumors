/-
The drained eweave (task #16, unit 3): merge completeness for the
encoder-order family. `Final.lean`'s drain machinery is family-generic
(over `FamOK` + `ManRows`); this file supplies the `procsE` instances
and re-runs the argmin over the encoder-order schedule.

The capacity hypothesis appears exactly once: `weaveE_wedge`'s margin-0
input. Everything downstream is the d5 argument verbatim — the drained
witness, the τ ranking, the blame step, and the stall refutation are
placement-independent once edge-respect of the witness is in hand.
-/
import StreamingMirror.Proofs.Sched.Weave.Final
import StreamingMirror.Proofs.Sched.Weave.MasterE

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ==================================== the encoder-order family's rows

/-- The encoder-order family reads the d5 manual rows through the
projection bridge: the parent moves within its scope's segment, never
across a channel-side. -/
theorem manRows_procsE : ManRows sk (procsE sk) :=
  ⟨fun hhr => ⟨_, procsE_walk sk hhr,
      fun c b => proj_walkEventsE_eq sk _ c b⟩,
    ⟨_, procsE_ropen sk, fun _ _ => rfl⟩⟩

-- ============================================ the drained eweave state

/-- The eweave, run to the merge fixpoint: the `.impl` potential's
carrier. -/
def wFinalE : MState := wPump sk (weaveStateE sk)

/-- Edge-respect survives the final pump. -/
theorem wfinalE_wedge (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) :
    WEdgeP sk (procsE sk) [] (wFinalE sk) :=
  wEdge_pump sk (weaveE_wedge sk hwf hm0)

/-- The final eweave state is a merge fixpoint. -/
theorem wfinalE_fix : step sk (wFinalE sk) = none :=
  wPump_fixpoint sk _

/-- EVERY encoder-order trace is a sublist of a drained-manual pump
fixpoint's output (cf. `all_sublist_final`; the case analysis is
`procsE`'s). -/
theorem all_sublist_finalE (hwf : sk.wellFormed = true) {st : MState}
    (h : WCountP sk (procsE sk) [] st) (hfix : step sk st = none) :
    ∀ T ∈ procsE sk, T.Sublist st.out := by
  have hge2 := (wf_rootH hwf).2
  have hfam := famOK_procsE sk hwf
  have hman := manRows_procsE sk
  have hroot : 1 ≤ sndCount Chan.rootres st.out := by
    have := rootres_full sk hfam hman h
    omega
  intro T hT
  simp only [procsE, List.mem_append, List.mem_cons,
    List.not_mem_nil, or_false, List.mem_map] at hT
  rcases hT with ((((rfl | rfl) | ⟨pk, hpk, rfl⟩) | rfl) | ⟨pk, hpk, rfl⟩)
    | rfl | rfl
  · -- iopen
    exact man_sublist sk h (M := 0) (by unfold manCount; omega) rfl
  · -- ropen
    exact man_sublist sk h (M := 1) (by unfold manCount; omega)
      (procsE_ropen sk)
  · -- a walk, in encoder order
    obtain ⟨i, hi, rfl⟩ := hpk
    have hilt : i < sk.rootH := List.mem_range.1 hi
    exact man_sublist sk h
      (M := walkIdx sk (sk.rootH - 1 - i))
      (by unfold walkIdx manCount; omega)
      (procsE_walk sk (by omega))
  · -- absorb
    exact absorb_sublist sk hfam h
      (absorb_counts_full sk hwf hfam hman h hfix hroot)
  · -- an assembler
    unfold Skel.asmKeys at hpk
    rcases List.mem_append.1 hpk with hk | hk
    · obtain ⟨q, hq, rfl⟩ := List.mem_map.1 hk
      have hqlt : q < sk.rootH := List.mem_range.1 hq
      exact asm_sublist sk hfam h (Or.inl ⟨rfl, rfl⟩) (by omega)
        (by omega)
        (asmI_counts sk hwf hfam hman h hfix hroot (q + 1) (by omega)
          (by omega))
    · obtain ⟨q, hq, rfl⟩ := List.mem_map.1 hk
      have hqlt : q < sk.rootH - 1 := List.mem_range.1 hq
      exact asm_sublist sk hfam h (Or.inr ⟨rfl, rfl⟩) (by omega)
        (by omega)
        (asmR_counts sk hwf hfam hman h hfix hroot (q + 1) (by omega)
          (by omega))
  · -- the floating rootret receive
    exact rootret_sublist sk hfam h (by omega)
      (rootret_fired sk hwf hfam hman h hfix hroot)
  · -- fins
    exact fin_sublist sk hfam h (by omega)
      (fin_counts_full sk hwf hfam hman h hfix hroot)

/-- THE EWEAVE IS TOTAL: every encoder-order trace rides inside the
final eweave output. -/
theorem all_sublist_wfinalE (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) :
    ∀ T ∈ procsE sk, T.Sublist (wFinalE sk).out :=
  all_sublist_finalE sk hwf
    (wfinalE_wedge sk hwf hm0).toWCountP (wfinalE_fix sk)

-- ================================================= canonical carrier

/-- Eweave-state projections are canonical. -/
theorem wprojE_canon (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsE sk) fut st) (c : Chan)
    (b : Bool) :
    proj c b st.out = canon c b (proj c b st.out).length := by
  refine wproj_canonP sk h c b ?_ (procsE_canon sk c b)
  cases b
  · exact procsE_rcv_owned sk hwf
  · exact procsE_snd_owned sk hwf

/-- The final eweave output carries each event at most once. -/
theorem wfinalE_count_le_one (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) :
    ∀ e : Ev, (wFinalE sk).out.count e ≤ 1 := by
  intro ⟨c, b, n⟩
  have hcanon := wprojE_canon sk hwf
    (wfinalE_wedge sk hwf hm0).toWCountP c b
  have hfilter : (wFinalE sk).out.count (c, b, n)
      = (proj c b (wFinalE sk).out).count (c, b, n) := by
    unfold proj
    exact (List.count_filter (by simp)).symm
  rw [hfilter, hcanon, count_canon]
  split <;> omega

/-- Freshness at the encoder-order merge: an event at or past its
side's schedule count was never emitted. -/
theorem not_mem_scheduleE_of_count (hwf : sk.wellFormed = true)
    {c : Chan} {b : Bool} {n : Nat}
    (hle : (proj c b (scheduleE sk)).length ≤ n) :
    ((c, b, n) : Ev) ∉ scheduleE sk := by
  intro hmem
  obtain ⟨ms, hms⟩ := scheduleE_proj_canon sk hwf c b
  have hmemp : ((c, b, n) : Ev) ∈ proj c b (scheduleE sk) :=
    List.mem_filter.2 ⟨hmem, by simp⟩
  have hlen : (proj c b (scheduleE sk)).length = ms := by
    rw [hms]
    unfold canon
    rw [List.length_map, List.length_range]
  rw [hms] at hmemp
  have := (mem_canon_lt hmemp).2.2
  omega

-- ================================================== the blame layer

/-- The blame step over the eweave: an unemitted protocol event sits
in its trace's final remainder, and that remainder's head sits at or
before it in the eweave. -/
theorem blame_headE (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) {g : Ev}
    (hgW : g ∈ (wFinalE sk).out) (hg_not : g ∉ scheduleE sk) :
    ∃ h', h' ∈ (finalStateE sk).rem.filterMap List.head?
      ∧ evIdx h' (wFinalE sk).out ≤ evIdx g (wFinalE sk).out := by
  obtain ⟨T', hT', hgT⟩ :=
    mem_some_trace sk (wfinalE_wedge sk hwf hm0).toWCountP hgW
  obtain ⟨r', hr'mem, pre', hpre', hsub'⟩ :=
    (trace_monotoneE sk).exists_of_mem_left hT'
  have hg_r : g ∈ r' := by
    rcases List.mem_append.1 (hpre' ▸ hgT) with hg | hg
    · exact absurd (hsub'.subset hg) hg_not
    · exact hg
  cases r' with
  | nil => cases hg_r
  | cons h' rest' =>
      refine ⟨h',
        List.mem_filterMap.2 ⟨h' :: rest', hr'mem, rfl⟩, ?_⟩
      rcases hg_r with _ | ⟨_, hg_r⟩
      · exact Nat.le_refl _
      · have hpair : ([h', g] : List Ev).Sublist (wFinalE sk).out := by
          have h1 : ([h', g] : List Ev).Sublist (h' :: rest') :=
            List.Sublist.cons_cons _ (List.singleton_sublist.2 hg_r)
          have h2 : (h' :: rest').Sublist T' :=
            hpre' ▸ List.sublist_append_right _ _
          exact (h1.trans h2).trans
            (all_sublist_wfinalE sk hwf hm0 T' hT')
        exact Nat.le_of_lt
          (pos_lt_of_pair (wfinalE_count_le_one sk hwf hm0) hpair)

-- ============================================== merge completeness

/-- MERGE COMPLETENESS at the encoder order: under well-formedness and
margin 0 the merge drains every trace — the `.impl` fixpoint's
remainders are all empty. The stall refutation is `merge_complete`'s,
ranked by eweave position. -/
theorem merge_completeE (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) :
    ((finalStateE sk).rem.all List.isEmpty) = true := by
  by_contra hcon
  rw [Bool.not_eq_true, List.all_eq_false] at hcon
  obtain ⟨r₀, hr₀mem, hr₀ne⟩ := hcon
  -- shared facts
  have hge2 := (wf_rootH hwf).2
  have hwedge := wfinalE_wedge sk hwf hm0
  have hcnt1 := wfinalE_count_le_one sk hwf hm0
  have hsubW := all_sublist_wfinalE sk hwf hm0
  have hWcanon : ∀ (c' : Chan) (b' : Bool),
      proj c' b' (wFinalE sk).out
        = canon c' b' (proj c' b' (wFinalE sk).out).length :=
    fun c' b' => wprojE_canon sk hwf hwedge.toWCountP c' b'
  have hminv := scheduleE_inv sk
  have hfix : step sk (finalStateE sk) = none :=
    mergeN_fixpoint sk (totalEventsE sk)
      ⟨[], fun _ => 0, fun _ => 0, procsE sk⟩ (Nat.le_refl _)
  have hscan : scan sk (finalStateE sk).sent (finalStateE sk).rcvd
      (finalStateE sk).rem = none := by
    unfold step at hfix
    cases hs : scan sk (finalStateE sk).sent (finalStateE sk).rcvd
        (finalStateE sk).rem with
    | none => rfl
    | some pr => rw [hs] at hfix; simp at hfix
  -- prefix counts never exceed the whole
  have htake_le : ∀ (c' : Chan) (b' : Bool) (k : Nat),
      (proj c' b' ((wFinalE sk).out.take k)).length
        ≤ (proj c' b' (wFinalE sk).out).length :=
    fun c' b' k => ((List.take_sublist k _).filter _).length_le
  -- the eweave bounds each side's total by the other
  have hRS : ∀ c' : Chan, (proj c' false (wFinalE sk).out).length
      ≤ (proj c' true (wFinalE sk).out).length := by
    intro c'
    rcases Nat.eq_zero_or_pos
        (proj c' false (wFinalE sk).out).length with hz | hpos
    · omega
    · have hmem : ((c', false,
          (proj c' false (wFinalE sk).out).length - 1) : Ev)
          ∈ (wFinalE sk).out :=
        proj_mem_of_lt (hWcanon c' false) (by omega)
      obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hmem
      have he1 := hwedge.e1_hist k c' _ hk
      rw [sndCount_eq_proj] at he1
      have := htake_le c' true k
      omega
  have hSR : ∀ c' : Chan, (proj c' true (wFinalE sk).out).length
      ≤ (proj c' false (wFinalE sk).out).length + sk.cap c' := by
    intro c'
    rcases Nat.eq_zero_or_pos
        (proj c' true (wFinalE sk).out).length with hz | hpos
    · omega
    · have hmem : ((c', true,
          (proj c' true (wFinalE sk).out).length - 1) : Ev)
          ∈ (wFinalE sk).out :=
        proj_mem_of_lt (hWcanon c' true) (by omega)
      obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hmem
      have he2 := hwedge.e2_hist k c' _ hk
      rw [rcvCount_eq_proj] at he2
      have := htake_le c' false k
      omega
  -- the minimum stalled head
  have hne : (finalStateE sk).rem.filterMap List.head? ≠ [] := by
    cases hr₀ : r₀ with
    | nil => rw [hr₀] at hr₀ne; simp at hr₀ne
    | cons e0 rest0 =>
        intro hnil
        have hmem : e0 ∈ (finalStateE sk).rem.filterMap List.head? :=
          List.mem_filterMap.2 ⟨r₀, hr₀mem, by rw [hr₀]; rfl⟩
        rw [hnil] at hmem
        cases hmem
  obtain ⟨estar, hstar_mem, hmin⟩ :=
    exists_min_image (fun e => evIdx e (wFinalE sk).out) hne
  obtain ⟨rs, hrs_mem, hrs_head⟩ := List.mem_filterMap.1 hstar_mem
  obtain ⟨rest, hrs⟩ := List.head?_eq_some_iff.1 hrs_head
  obtain ⟨is, his⟩ := List.mem_iff_getElem?.1 hrs_mem
  have hdis : enabled sk (finalStateE sk).sent (finalStateE sk).rcvd
      estar = false :=
    scan_none_heads sk hscan (i := is) (by rw [his, hrs])
  obtain ⟨Tstar, hTstar, preM, hpreM, hsubM⟩ :=
    Forall2.exists_rel_left hminv.rem_struct his
  have hTstar_mem : Tstar ∈ procsE sk := List.mem_of_getElem? hTstar
  have hestar_W : estar ∈ (wFinalE sk).out := by
    refine (hsubW Tstar hTstar_mem).subset ?_
    rw [hpreM, hrs]
    exact List.mem_append_right _ (List.mem_cons_self ..)
  obtain ⟨c, b, n⟩ := estar
  cases b with
  | false =>
      -- STARVED RECEIVE: blame the send at the current count
      have hsent : (finalStateE sk).sent c
          = sndCount c (scheduleE sk) := hminv.sent_eq c
      simp only [enabled, decide_eq_false_iff_not, Nat.not_lt] at hdis
      have hstarve : sndCount c (scheduleE sk) ≤ n := by omega
      have hnW : n < (proj c false (wFinalE sk).out).length := by
        have hm : ((c, false, n) : Ev)
            ∈ proj c false (wFinalE sk).out :=
          List.mem_filter.2 ⟨hestar_W, by simp⟩
        rw [hWcanon c false] at hm
        exact (mem_canon_lt hm).2.2
      have hsW : sndCount c (scheduleE sk)
          < (proj c true (wFinalE sk).out).length := by
        have := hRS c
        omega
      have hgW : ((c, true, sndCount c (scheduleE sk)) : Ev)
          ∈ (wFinalE sk).out :=
        proj_mem_of_lt (hWcanon c true) hsW
      have hg_not : ((c, true, sndCount c (scheduleE sk)) : Ev)
          ∉ scheduleE sk :=
        not_mem_scheduleE_of_count sk hwf
          (Nat.le_of_eq (sndCount_eq_proj c _).symm)
      obtain ⟨h', hh'pool, hh'le⟩ :=
        blame_headE sk hwf hm0 hgW hg_not
      -- the receive at the current count is in the eweave
      have hrW : ((c, false, sndCount c (scheduleE sk)) : Ev)
          ∈ (wFinalE sk).out :=
        proj_mem_of_lt (hWcanon c false) (by omega)
      -- E1 in the eweave: the blocker precedes that receive
      have hk1 := evIdx_getElem? hrW
      have he1 := hwedge.e1_hist _ c (sndCount c (scheduleE sk)) hk1
      obtain ⟨j, hjlt, hjget⟩ := mem_take_snd (hWcanon c true) he1
      have hgle := evIdx_le hjget
      -- that receive is at or before the head in the eweave
      have hminstar := hmin h' hh'pool
      rcases Nat.eq_or_lt_of_le hstarve with heq | hlt2
      · rw [heq] at hgle hjlt hh'le
        omega
      · have hpair := pair_sublist_canon (c := c) (b := false)
          hlt2 hnW
        have hcanonsub : (canon c false
            (proj c false (wFinalE sk).out).length).Sublist
            (wFinalE sk).out := by
          rw [← hWcanon c false]
          exact List.filter_sublist
        have hordered := pos_lt_of_pair hcnt1 (hpair.trans hcanonsub)
        omega
  | true =>
      -- JAMMED SEND: blame the receive the cap window awaits
      have hrcvd : (finalStateE sk).rcvd c
          = rcvCount c (scheduleE sk) := hminv.rcvd_eq c
      simp only [enabled, decide_eq_false_iff_not, Nat.not_lt] at hdis
      have hjam : rcvCount c (scheduleE sk) + sk.cap c ≤ n := by omega
      have hnW : n < (proj c true (wFinalE sk).out).length := by
        have hm : ((c, true, n) : Ev)
            ∈ proj c true (wFinalE sk).out :=
          List.mem_filter.2 ⟨hestar_W, by simp⟩
        rw [hWcanon c true] at hm
        exact (mem_canon_lt hm).2.2
      have hrlt : rcvCount c (scheduleE sk)
          < (proj c false (wFinalE sk).out).length := by
        have := hSR c
        omega
      have hgW : ((c, false, rcvCount c (scheduleE sk)) : Ev)
          ∈ (wFinalE sk).out :=
        proj_mem_of_lt (hWcanon c false) hrlt
      have hg_not : ((c, false, rcvCount c (scheduleE sk)) : Ev)
          ∉ scheduleE sk :=
        not_mem_scheduleE_of_count sk hwf
          (Nat.le_of_eq (rcvCount_eq_proj c _).symm)
      obtain ⟨h', hh'pool, hh'le⟩ :=
        blame_headE sk hwf hm0 hgW hg_not
      -- E2 in the eweave at the jammed send's own position
      have hkstar := evIdx_getElem? hestar_W
      have he2 := hwedge.e2_hist _ c n hkstar
      obtain ⟨j, hjlt, hjget⟩ := mem_take_rcv (hWcanon c false)
        (k := evIdx ((c, true, n) : Ev) (wFinalE sk).out)
        (n := rcvCount c (scheduleE sk)) (by omega)
      have hgle := evIdx_le hjget
      have hminstar := hmin h' hh'pool
      omega

end StreamingMirror.Sched
