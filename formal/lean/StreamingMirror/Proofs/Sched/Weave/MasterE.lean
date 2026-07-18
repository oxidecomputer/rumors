/-
The E consumption induction (unit 2a, PROGRESS.md §9): `weaveGo_wedge`'s
twin over the encoder-order interpreter — the edge invariant at the
`procsE` family rides `weaveGoE`, each manual guard discharged from the
pointwise readiness property (`EmitOKOnP` at `procsE`), the precedence
layer, and the pump fixpoint the previous emission left behind.

The readiness property itself — discharging `EmitOKOnP` at every
position of the eweave's future, where the U-sites consume the margin-0
capacity hypothesis — is unit 2b, the E master induction. This file
only carries the generic consumption frame that turns that readiness
into `WEdgeP sk (procsE sk) [] (weaveStateE sk)`.
-/
import StreamingMirror.Proofs.Sched.Weave.Master
import StreamingMirror.Proofs.Sched.Weave.ExpandE
import StreamingMirror.Proofs.Sched.Weave.TeleE

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

/-- THE E CONSUMPTION INDUCTION: `weaveGo_wedge` over the encoder-order
expanders, at the encoder-order family. -/
theorem weaveGoE_wedge (fuel : Nat) :
    ∀ (ops : List WOp) (st : MState) (done : List Ev),
      WEdgeP sk (procsE sk) (goEventsE sk fuel ops) st →
      DepOK done (goEventsE sk fuel ops) →
      (∀ x ∈ done, x ∈ st.out) →
      EmitOKOnP sk (procsE sk) (goEventsE sk fuel ops) [] →
      step sk st = none →
      WEdgeP sk (procsE sk) [] (weaveGoE sk fuel ops st) := by
  induction fuel with
  | zero => intro ops st done hW _ _ _ _; exact hW
  | succ f ih =>
      intro ops st done hW hdep hdone hemit hfix
      match ops with
      | [] => exact hW
      | .emit e :: rest =>
          have hgo : goEventsE sk (f + 1) (.emit e :: rest)
              = e :: goEventsE sk f rest := rfl
          rw [hgo] at hW hdep hemit
          have hen : enabled sk st.sent st.rcvd e = true := by
            refine hemit 0 e rfl st (by simpa using hW) hfix ?_
            intro d hd
            exact hdone d (depOK_head hdep d hd)
          show WEdgeP sk (procsE sk) []
            (weaveGoE sk f rest (wEmitP sk st e))
          refine ih rest (wEmitP sk st e) (done ++ [e])
            (wEdge_emitP sk hen hW) (depOK_tail hdep) ?_
            (emitOKOn_tail sk hemit) (wPump_fixpoint sk _)
          intro x hx
          rcases List.mem_append.1 hx with hx | hx
          · exact mem_out_wEmitP sk
              (List.mem_append_left _ (hdone x hx))
          · have hxe : x = e := List.mem_singleton.1 hx
            subst hxe
            exact mem_out_wEmitP sk
              (List.mem_append_right _ (List.mem_cons_self ..))
      | .scope h' k feed :: rest =>
          exact ih _ st done hW hdep hdone hemit hfix
      | .kid h' k s lastD kidBase i feed :: rest =>
          exact ih _ st done hW hdep hdone hemit hfix

/-- Pointwise emission-readiness at the encoder-order family. -/
abbrev EmitOKOnE (l rest : List Ev) : Prop :=
  EmitOKOnP sk (procsE sk) l rest

-- ==================================================== window plumbing

/-- A window conclusion opens the guard (family-free; cf. the d5
private `enabled_of_window`). -/
private theorem enabled_of_windowE {st : MState} {c : Chan} {n : Nat}
    (hwf : sk.wellFormed = true) (hwin : n ≤ rcvCount c st.out)
    (hrcvd : st.rcvd c = rcvCount c st.out) :
    enabled sk st.sent st.rcvd (c, true, n) = true := by
  simp only [enabled, decide_eq_true_eq]
  have hcap := cap_pos hwf c
  omega

/-- A prologue wire receive discharges from its in-flight send, at the
encoder-order family. -/
theorem head_rcv_wireE (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (hW : WCountP sk (procsE sk) fut st) {p : Party}
    {hh n : Nat}
    (hpred : ∀ d, manDep ((Chan.wire p hh, false, n) : Ev) = some d →
      d ∈ st.out) :
    enabled sk st.sent st.rcvd (Chan.wire p hh, false, n) = true :=
  enabled_rcv_of_memP sk hW (procsE_snd_owned sk hwf)
    (procsE_canon sk _ true) (hpred _ (manDep_wire_rcv p hh n))

/-- A prologue asked receive discharges from its in-flight send, at
the encoder-order family. -/
theorem head_rcv_askedE (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (hW : WCountP sk (procsE sk) fut st) {p : Party}
    {hh n : Nat}
    (hpred : ∀ d, manDep ((Chan.asked p hh, false, n) : Ev) = some d →
      d ∈ st.out) :
    enabled sk st.sent st.rcvd (Chan.asked p hh, false, n) = true :=
  enabled_rcv_of_memP sk hW (procsE_snd_owned sk hwf)
    (procsE_canon sk _ true) (hpred _ (manDep_asked_rcv p hh n))

/-- A manual-consumed wire send discharges from its predecessor
receive, or opens on a fresh window at seq zero (E family). -/
theorem head_snd_wireE (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (hW : WCountP sk (procsE sk) fut st) {p : Party}
    {hh n : Nat} (hh1 : 1 ≤ hh)
    (hpred : ∀ d, manDep ((Chan.wire p hh, true, n) : Ev) = some d →
      d ∈ st.out) :
    enabled sk st.sent st.rcvd (Chan.wire p hh, true, n) = true := by
  rcases Nat.eq_zero_or_pos n with rfl | hn
  · exact enabled_snd_low sk (cap_pos hwf _)
  · have hc : sk.cap (Chan.wire p hh) = 1 := rfl
    refine enabled_snd_of_memP sk hW (procsE_rcv_owned sk hwf)
      (procsE_canon sk _ false) ?_ (by omega)
    have hmem := hpred _ (manDep_wire_snd_pos (by omega) (by omega))
    rw [hc]
    exact hmem

/-- A manual-consumed query send discharges from its predecessor
receive, or opens on a fresh window at seq zero (E family). -/
theorem head_snd_askedE (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (hW : WCountP sk (procsE sk) fut st) {p : Party}
    {hh n : Nat}
    (hpred : ∀ d, manDep ((Chan.asked p hh, true, n) : Ev) = some d →
      d ∈ st.out) :
    enabled sk st.sent st.rcvd (Chan.asked p hh, true, n) = true := by
  rcases Nat.eq_zero_or_pos n with rfl | hn
  · exact enabled_snd_low sk (cap_pos hwf _)
  · have hc : sk.cap (Chan.asked p hh) = 1 := rfl
    refine enabled_snd_of_memP sk hW (procsE_rcv_owned sk hwf)
      (procsE_canon sk _ false) ?_ (by omega)
    have hmem := hpred _ (manDep_asked_snd_pos (by omega))
    rw [hc]
    exact hmem

-- ======================================== the E kid-suffix alignment

private theorem drop_eq_flatMap_getElem?E {α : Type _} :
    ∀ (m i : Nat) (F : List α), i + m = F.length →
      (List.range' i m).flatMap (fun j => (F[j]?).toList) = F.drop i
  | 0, i, F, h => by
      rw [List.range'_zero, List.flatMap_nil,
        List.drop_eq_nil_of_le (by omega)]
  | m + 1, i, F, h => by
      rw [List.range'_succ, List.flatMap_cons,
        drop_eq_flatMap_getElem?E m (i + 1) F (by omega),
        List.getElem?_eq_getElem (by omega), Option.toList_some,
        List.singleton_append, ← List.drop_eq_getElem_cons (by omega)]

private theorem walkSegE_glue_range' (h' : Nat) (g : Nat → Nat)
    (hmono : ∀ i, g i ≤ g (i + 1)) :
    ∀ (m i : Nat),
      (List.range' i m).flatMap
          (fun j => walkSegE sk h' (g j) (g (j + 1)))
        = walkSegE sk h' (g i) (g (i + m)) := by
  have hchain : ∀ (d a : Nat), g a ≤ g (a + d) := by
    intro d
    induction d with
    | zero => intro a; exact Nat.le_refl _
    | succ d ihd => intro a; exact Nat.le_trans (ihd a) (hmono (a + d))
  intro m
  induction m with
  | zero =>
      intro i
      rw [Nat.add_zero, List.range'_zero, List.flatMap_nil,
        walkSegE_empty]
  | succ m ihm =>
      intro i
      rw [List.range'_succ, List.flatMap_cons, ihm (i + 1),
        walkSegE_glue sk (hmono i) (hchain m (i + 1)),
        show i + 1 + m = i + (m + 1) from by omega]

/-- One E kid op's per-stage filters (cf. `kid_filters`): the
ownership cover, the feeder's query, the own-stage chunk — a plain
`childChunk`, never a parent — and the descendant E windows. -/
theorem kid_filtersE (hwf : sk.wellFormed = true)
    {h k : Nat} (hh : h < sk.rootH) (hk : k < sk.stageLen h)
    {F : List Ev} {mF : Nat}
    (hF : F.length = sk.nChildren h (sk.stageScope h k))
    (hFo : ∀ e ∈ F, evOwner sk e = mF)
    (hmF : mF < walkIdx sk h)
    {i : Nat} (hi : i < sk.nChildren h (sk.stageScope h k)) :
    (∀ e ∈ opEventsE sk (.kid h k (sk.stageScope h k) none
        (sk.wiresBefore h k) i F),
      evOwner sk e = mF ∨ ∃ h', h' ≤ h ∧ evOwner sk e = walkIdx sk h')
    ∧ (opEventsE sk (.kid h k (sk.stageScope h k) none
        (sk.wiresBefore h k) i F)).filter
          (fun e => evOwner sk e == mF) = F[i]?.toList
    ∧ (opEventsE sk (.kid h k (sk.stageScope h k) none
        (sk.wiresBefore h k) i F)).filter
          (fun e => evOwner sk e == walkIdx sk h)
        = childChunk sk (wpk h) k i
    ∧ ∀ h', h' < h →
        (opEventsE sk (.kid h k (sk.stageScope h k) none
            (sk.wiresBefore h k) i F)).filter
          (fun e => evOwner sk e == walkIdx sk h')
        = walkSegE sk h'
            (descIdx sk h' (h - 1 - h') (sk.wiresBefore h k + i))
            (descIdx sk h' (h - 1 - h')
              (sk.wiresBefore h k + (i + 1))) := by
  cases h with
  | zero =>
      have hD0 : sk.childIsD 0 (sk.stageScope 0 k) i = false := rfl
      have hkidE : opEventsE sk (.kid 0 k (sk.stageScope 0 k)
            none (sk.wiresBefore 0 k) i F)
          = (wireOut (wpk 0), true, sk.wiresBefore 0 k + i)
              :: F[i]?.toList := by
        rw [opEventsE_kid_eq,
          if_neg (by rw [hD0]; exact Bool.false_ne_true),
          if_pos (show ((0 : Nat) == 0) = true by rfl), List.append_nil]
      refine ⟨?_, ?_, ?_, ?_⟩
      · intro e he
        rw [hkidE] at he
        rcases he with _ | ⟨_, he⟩
        · exact Or.inr ⟨0, Nat.le_refl 0, evOwner_wireOut sk hh _⟩
        · exact Or.inl (hFo e
            (List.mem_of_getElem? (Option.mem_toList.1 he)))
      · rw [hkidE,
          List.filter_cons_of_neg (by
            simp only [evOwner_wireOut sk hh, beq_iff_eq]; omega)]
        cases hfi : F[i]? with
        | none => rfl
        | some q =>
            rw [Option.toList_some,
              List.filter_cons_of_pos (by
                simp only [hFo q (List.mem_of_getElem? hfi),
                  beq_self_eq_true]),
              List.filter_nil]
      · rw [hkidE,
          List.filter_cons_of_pos (by
            simp only [evOwner_wireOut sk hh, beq_self_eq_true]),
          childChunk_eq,
          if_neg (by rw [hD0]; exact Bool.false_ne_true)]
        congr 1
        cases hfi : F[i]? with
        | none => rfl
        | some q =>
            rw [Option.toList_some,
              List.filter_cons_of_neg (by
                simp only [hFo q (List.mem_of_getElem? hfi), beq_iff_eq]
                omega),
              List.filter_nil]
      · intro h' hlt
        exact absurd hlt (Nat.not_lt_zero h')
  | succ h =>
      have hh' : h < sk.rootH := by omega
      have h1 : (1 : Nat) ≤ h + 1 := by omega
      have hkid : sk.wiresBefore (h + 1) k + i < sk.stageLen h := by
        have htot := wiresBefore_total sk hwf h1 hh
        simp only [Nat.add_sub_cancel] at htot
        have hmono := wiresBefore_mono sk (h + 1)
          (show k + 1 ≤ sk.stageLen (h + 1) from hk)
        have hstep := wiresBefore_succ sk hk
        omega
      have hmF' : walkIdx sk (h + 1) < walkIdx sk h :=
        walkIdx_lt sk (Nat.lt_succ_self h) hh
      have hkidE : opEventsE sk (.kid (h + 1) k (sk.stageScope (h + 1) k)
            none (sk.wiresBefore (h + 1) k) i F)
          = (wireOut (wpk (h + 1)), true, sk.wiresBefore (h + 1) k + i)
              :: (if sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i then
                    (lowerOut (wpk (h + 1)), true,
                        sk.dsBefore (h + 1) k + dRank sk (wpk (h + 1)) k i)
                      :: (F[i]?.toList
                        ++ opEventsE sk (.scope h
                            (sk.wiresBefore (h + 1) k + i)
                            (chunkQ sk (h + 1) k i)))
                  else F[i]?.toList
                    ++ opEventsE sk (.scope h
                        (sk.wiresBefore (h + 1) k + i) [])) := by
        rw [opEventsE_kid_eq]
        simp only [Nat.add_sub_cancel,
          show ((h + 1 : Nat) == 0) = false from rfl, Bool.false_eq_true,
          if_false]
      have hIHsub := align_scopeE sk hwf h
        (sk.wiresBefore (h + 1) k + i) (chunkQ sk (h + 1) k i)
        (walkIdx sk (h + 1)) hh' hkid
        (by
          have hq := qCount_eq_kid_nChildren sk hwf h1 hh hk hi
          simp only [Nat.add_sub_cancel] at hq
          rw [chunkQ_length, hq])
        (chunkQ_owner sk h1 hh k i) hmF'
      have hIHW := fun
          (hDf : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i = false) =>
        align_scopeE sk hwf h (sk.wiresBefore (h + 1) k + i) []
          (walkIdx sk (h + 1)) hh' hkid
          (by
            have hz := nChildren_kid_notD sk hwf h1 hh hk hi hDf
            simp only [Nat.add_sub_cancel] at hz
            rw [List.length_nil, hz])
          (fun e he => absurd he (by simp)) hmF'
      have hFeedMF : (F[i]?.toList).filter
          (fun e => evOwner sk e == mF) = F[i]?.toList := by
        cases hfi : F[i]? with
        | none => rfl
        | some q =>
            rw [Option.toList_some,
              List.filter_cons_of_pos (by
                simp only [hFo q (List.mem_of_getElem? hfi),
                  beq_self_eq_true]),
              List.filter_nil]
      have hFeedNe : ∀ M, mF ≠ M → (F[i]?.toList).filter
          (fun e => evOwner sk e == M) = [] := by
        intro M hne
        cases hfi : F[i]? with
        | none => rfl
        | some q =>
            rw [Option.toList_some,
              List.filter_cons_of_neg (by
                simp only [hFo q (List.mem_of_getElem? hfi), beq_iff_eq]
                exact hne),
              List.filter_nil]
      refine ⟨?_, ?_, ?_, ?_⟩
      · -- ownership
        intro e he
        rw [hkidE] at he
        rcases he with _ | ⟨_, he⟩
        · exact Or.inr ⟨h + 1, Nat.le_refl _, evOwner_wireOut sk hh _⟩
        by_cases hD : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
        · rw [if_pos hD] at he
          rcases he with _ | ⟨_, he⟩
          · exact Or.inr ⟨h + 1, Nat.le_refl _, evOwner_lowerOut sk _⟩
          rcases List.mem_append.1 he with he | he
          · exact Or.inl (hFo e
              (List.mem_of_getElem? (Option.mem_toList.1 he)))
          · rcases hIHsub.1 e he with ho | ⟨h'', hle'', ho⟩
            · exact Or.inr ⟨h + 1, Nat.le_refl _, ho⟩
            · exact Or.inr ⟨h'', by omega, ho⟩
        · rw [if_neg hD] at he
          have hDf : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
              = false := by simpa using hD
          rcases List.mem_append.1 he with he | he
          · exact Or.inl (hFo e
              (List.mem_of_getElem? (Option.mem_toList.1 he)))
          · rcases (hIHW hDf).1 e he with ho | ⟨h'', hle'', ho⟩
            · exact Or.inr ⟨h + 1, Nat.le_refl _, ho⟩
            · exact Or.inr ⟨h'', by omega, ho⟩
      · -- the feeder's filter
        have hSubDropD : (opEventsE sk (.scope h
              (sk.wiresBefore (h + 1) k + i)
              (chunkQ sk (h + 1) k i))).filter
            (fun e => evOwner sk e == mF) = [] := by
          rw [List.filter_eq_nil_iff]
          intro e he
          rcases hIHsub.1 e he with ho | ⟨h'', hle'', ho⟩
          · simp only [ho, beq_iff_eq]
            omega
          · have hwlt := walkIdx_lt sk (show h'' < h + 1 from by omega) hh
            simp only [ho, beq_iff_eq]
            omega
        rw [hkidE]
        by_cases hD : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
        · rw [if_pos hD,
            List.filter_cons_of_neg (by
              simp only [evOwner_wireOut sk hh, beq_iff_eq]; omega),
            List.filter_cons_of_neg (by
              simp only [evOwner_lowerOut, beq_iff_eq]; omega),
            List.filter_append, hFeedMF, hSubDropD, List.append_nil]
        · have hDf : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
              = false := by simpa using hD
          have hSubDropW : (opEventsE sk (.scope h
                (sk.wiresBefore (h + 1) k + i) [])).filter
              (fun e => evOwner sk e == mF) = [] := by
            rw [List.filter_eq_nil_iff]
            intro e he
            rcases (hIHW hDf).1 e he with ho | ⟨h'', hle'', ho⟩
            · simp only [ho, beq_iff_eq]
              omega
            · have hwlt := walkIdx_lt sk (show h'' < h + 1 from by omega)
                hh
              simp only [ho, beq_iff_eq]
              omega
          rw [if_neg hD,
            List.filter_cons_of_neg (by
              simp only [evOwner_wireOut sk hh, beq_iff_eq]; omega),
            List.filter_append, hFeedMF, hSubDropW, List.append_nil]
      · -- the own-stage chunk
        have hFeedOwn : (F[i]?.toList).filter
            (fun e => evOwner sk e == walkIdx sk (h + 1)) = [] :=
          hFeedNe _ (by omega)
        rw [hkidE, childChunk_eq]
        by_cases hD : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
        · rw [if_pos hD, if_pos hD,
            List.filter_cons_of_pos (by
              simp only [evOwner_wireOut sk hh, beq_self_eq_true]),
            List.filter_cons_of_pos (by
              simp only [evOwner_lowerOut, beq_self_eq_true]),
            List.filter_append, hFeedOwn, List.nil_append,
            hIHsub.2.1]
        · have hDf : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
              = false := by simpa using hD
          rw [if_neg hD, if_neg hD,
            List.filter_cons_of_pos (by
              simp only [evOwner_wireOut sk hh, beq_self_eq_true]),
            List.filter_append, hFeedOwn, List.nil_append,
            (hIHW hDf).2.1]
      · -- the descendant windows
        intro h' hlt
        have hle : h' ≤ h := by omega
        have hwlt : walkIdx sk (h + 1) < walkIdx sk h' :=
          walkIdx_lt sk (by omega) hh
        have hFeedDrop : (F[i]?.toList).filter
            (fun e => evOwner sk e == walkIdx sk h') = [] :=
          hFeedNe _ (by omega)
        rw [hkidE]
        rw [show h + 1 - 1 - h' = h - h' from by omega]
        by_cases hD : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
        · rw [if_pos hD,
            List.filter_cons_of_neg (by
              simp only [evOwner_wireOut sk hh, beq_iff_eq]; omega),
            List.filter_cons_of_neg (by
              simp only [evOwner_lowerOut, beq_iff_eq]; omega),
            List.filter_append, hFeedDrop, List.nil_append,
            hIHsub.2.2 h' hle, Nat.add_assoc]
        · have hDf : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
              = false := by simpa using hD
          rw [if_neg hD,
            List.filter_cons_of_neg (by
              simp only [evOwner_wireOut sk hh, beq_iff_eq]; omega),
            List.filter_append, hFeedDrop, List.nil_append,
            (hIHW hDf).2.2 h' hle, Nat.add_assoc]

/-- The E kid-suffix clauses (cf. `align_kids_suffix`): ownership,
the feeder's residue, the own-stage chunk run — parent-free — and the
descendant E windows. -/
theorem align_kids_suffixE (hwf : sk.wellFormed = true)
    {h k : Nat} (hh : h < sk.rootH) (hk : k < sk.stageLen h)
    {F : List Ev} {mF : Nat}
    (hF : F.length = sk.nChildren h (sk.stageScope h k))
    (hFo : ∀ e ∈ F, evOwner sk e = mF)
    (hmF : mF < walkIdx sk h)
    {i : Nat} (hi : i ≤ sk.nChildren h (sk.stageScope h k)) :
    (∀ e ∈ (List.range' i (sk.nChildren h (sk.stageScope h k) - i)).flatMap
        (fun j => opEventsE sk (.kid h k (sk.stageScope h k)
          none (sk.wiresBefore h k) j F)),
      evOwner sk e = mF ∨ ∃ h', h' ≤ h ∧ evOwner sk e = walkIdx sk h')
    ∧ ((List.range' i (sk.nChildren h (sk.stageScope h k) - i)).flatMap
        (fun j => opEventsE sk (.kid h k (sk.stageScope h k)
          none (sk.wiresBefore h k) j F))).filter
          (fun e => evOwner sk e == mF) = F.drop i
    ∧ ((List.range' i (sk.nChildren h (sk.stageScope h k) - i)).flatMap
        (fun j => opEventsE sk (.kid h k (sk.stageScope h k)
          none (sk.wiresBefore h k) j F))).filter
          (fun e => evOwner sk e == walkIdx sk h)
        = (List.range' i (sk.nChildren h (sk.stageScope h k) - i)).flatMap
            (childChunk sk (wpk h) k)
    ∧ ∀ h', h' < h →
        ((List.range' i (sk.nChildren h (sk.stageScope h k) - i)).flatMap
            (fun j => opEventsE sk (.kid h k (sk.stageScope h k)
              none (sk.wiresBefore h k) j F))).filter
          (fun e => evOwner sk e == walkIdx sk h')
        = walkSegE sk h'
            (descIdx sk h' (h - 1 - h') (sk.wiresBefore h k + i))
            (descIdx sk h' (h - 1 - h')
              (sk.wiresBefore h k
                + sk.nChildren h (sk.stageScope h k))) := by
  have hjlt : ∀ j ∈ List.range' i
      (sk.nChildren h (sk.stageScope h k) - i),
      j < sk.nChildren h (sk.stageScope h k) := by
    intro j hj
    have := List.mem_range'_1.mp hj
    omega
  refine ⟨?_, ?_, ?_, ?_⟩
  · intro e he
    obtain ⟨j, hj, hej⟩ := List.mem_flatMap.1 he
    exact (kid_filtersE sk hwf hh hk hF hFo hmF (hjlt j hj)).1 e hej
  · simp only [List.filter_flatMap]
    rw [flatMap_congr (fun j hj =>
      (kid_filtersE sk hwf hh hk hF hFo hmF (hjlt j hj)).2.1)]
    exact drop_eq_flatMap_getElem?E
      (sk.nChildren h (sk.stageScope h k) - i) i F (by omega)
  · simp only [List.filter_flatMap]
    exact flatMap_congr (fun j hj =>
      (kid_filtersE sk hwf hh hk hF hFo hmF (hjlt j hj)).2.2.1)
  · intro h' hlt
    simp only [List.filter_flatMap]
    rw [flatMap_congr (fun j hj =>
        (kid_filtersE sk hwf hh hk hF hFo hmF (hjlt j hj)).2.2.2 h' hlt),
      walkSegE_glue_range' sk h'
        (fun j => descIdx sk h' (h - 1 - h') (sk.wiresBefore h k + j))
        (fun j => descIdx_mono sk h' (h - 1 - h') (by omega))
        (sk.nChildren h (sk.stageScope h k) - i) i,
      show i + (sk.nChildren h (sk.stageScope h k) - i)
        = sk.nChildren h (sk.stageScope h k) from by omega]

/-- An E subtree owns nothing at a foreign index. -/
theorem scope_filter_neE (hwf : sk.wellFormed = true) {h k : Nat}
    {F : List Ev} {mF M : Nat} (hh : h < sk.rootH)
    (hk : k < sk.stageLen h)
    (hF : F.length = sk.nChildren h (sk.stageScope h k))
    (hFo : ∀ e ∈ F, evOwner sk e = mF) (hmF : mF < walkIdx sk h)
    (hMne : mF ≠ M) (hMhigh : ∀ h', h' ≤ h → walkIdx sk h' ≠ M) :
    (opEventsE sk (.scope h k F)).filter
      (fun e => evOwner sk e == M) = [] := by
  rw [List.filter_eq_nil_iff]
  intro e he
  rcases (align_scopeE sk hwf h k F mF hh hk hF hFo hmF).1 e he with
    ho | ⟨h', hle, ho⟩
  · simp only [ho, beq_iff_eq]
    exact hMne
  · simp only [ho, beq_iff_eq]
    exact hMhigh h' hle

/-- An E kid suffix owns nothing at a foreign index. -/
theorem kids_filter_neE (hwf : sk.wellFormed = true) {h k : Nat}
    {F : List Ev} {mF M : Nat} (hh : h < sk.rootH)
    (hk : k < sk.stageLen h)
    (hF : F.length = sk.nChildren h (sk.stageScope h k))
    (hFo : ∀ e ∈ F, evOwner sk e = mF) (hmF : mF < walkIdx sk h)
    {i : Nat} (hi : i ≤ sk.nChildren h (sk.stageScope h k))
    (hMne : mF ≠ M) (hMhigh : ∀ h', h' ≤ h → walkIdx sk h' ≠ M) :
    ((List.range' i (sk.nChildren h (sk.stageScope h k) - i)).flatMap
        (fun i' => opEventsE sk (.kid h k (sk.stageScope h k)
          none (sk.wiresBefore h k) i' F))).filter
      (fun e => evOwner sk e == M) = [] := by
  rw [List.filter_eq_nil_iff]
  intro e he
  rcases (align_kids_suffixE sk hwf hh hk hF hFo hmF hi).1 e he with
    ho | ⟨h', hle, ho⟩
  · simp only [ho, beq_iff_eq]
    exact hMne
  · simp only [ho, beq_iff_eq]
    exact hMhigh h' hle

-- ================================= rebasing and gluing the E context

/-- Rebase the E telescope across a local prefix (cf.
`ancTele_rebase`). -/
theorem ancTeleE_rebase {h : Nat} {A j t : Nat → Nat}
    {pre rest : List Ev} (hanc : AncTeleE sk h A j t rest)
    (hnil : ∀ G, h + 2 ≤ G → G < sk.rootH →
      pre.filter (fun e => evOwner sk e == walkIdx sk G) = [])
    {c : Nat}
    (hpar : h + 1 < sk.rootH →
      (pre ++ rest).filter
          (fun e => evOwner sk e == walkIdx sk (h + 1))
        = (chunkQ sk (h + 1) (A (h + 1)) (j (h + 1))).drop c
          ++ (List.range' (j (h + 1) + 1)
                (sk.nChildren (h + 1)
                    (sk.stageScope (h + 1) (A (h + 1)))
                  - (j (h + 1) + 1))).flatMap
               (childChunk sk (wpk (h + 1)) (A (h + 1)))
          ++ ((upperOut (wpk (h + 1)), true, A (h + 1)) : Ev)
            :: walkSegE sk (h + 1) (A (h + 1) + 1)
                (sk.stageLen (h + 1))) :
    AncTeleE sk h A j (fun G => if G = h + 1 then c else t G)
      (pre ++ rest) := by
  refine ⟨hanc.rng, hanc.isD, hanc.coh, ?_⟩
  intro G hG hGr
  by_cases hG1 : G = h + 1
  · subst hG1
    simp only [reduceIte]
    exact hpar hGr
  · simp only [if_neg hG1]
    rw [List.filter_append, hnil G (by omega) hGr, List.nil_append]
    exact hanc.fil G hG hGr

/-- The deep E windows at a mid-scope slot: the kid suffix's windows
glued to the after-scope remainder (cf. `deep_glue`). -/
theorem deep_glueE (hwf : sk.wellFormed = true) {h k : Nat}
    (hhr : h < sk.rootH) (hk : k < sk.stageLen h) {i : Nat}
    (hi : i ≤ sk.nChildren h (sk.stageScope h k))
    {suffix rest : List Ev}
    (hsuf : ∀ g', g' < h →
      suffix.filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSegE sk g'
            (descIdx sk g' (h - 1 - g') (sk.wiresBefore h k + i))
            (descIdx sk g' (h - 1 - g')
              (sk.wiresBefore h k
                + sk.nChildren h (sk.stageScope h k))))
    (hrest : ∀ g', g' ≤ h →
      rest.filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSegE sk g' (descIdx sk g' (h - g') (k + 1))
            (sk.stageLen g')) :
    ∀ g', g' < h →
      (suffix ++ rest).filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSegE sk g'
            (descIdx sk g' (h - 1 - g') (sk.wiresBefore h k + i))
            (sk.stageLen g') := by
  intro g' hg'
  rw [List.filter_append, hsuf g' hg', hrest g' (Nat.le_of_lt hg')]
  have hbc : descIdx sk g' (h - g') (k + 1)
      = descIdx sk g' (h - 1 - g')
          (sk.wiresBefore h k
            + sk.nChildren h (sk.stageScope h k)) := by
    rw [show sk.wiresBefore h k + sk.nChildren h (sk.stageScope h k)
        = sk.wiresBefore h (k + 1) from (wiresBefore_succ sk hk).symm,
      show h - g' = h - 1 - g' + 1 from by omega, descIdx_succ,
      show g' + (h - 1 - g') + 1 = h from by omega]
  rw [hbc]
  have hmono := descIdx_mono sk g' (h - 1 - g')
    (show sk.wiresBefore h k + i
        ≤ sk.wiresBefore h k + sk.nChildren h (sk.stageScope h k)
      from by omega)
  have hend : descIdx sk g' (h - 1 - g')
      (sk.wiresBefore h k + sk.nChildren h (sk.stageScope h k))
      ≤ sk.stageLen g' := by
    rw [← hbc]
    refine descIdx_le_stageLen sk hwf ?_ ?_
    · rw [show g' + (h - g') = h from by omega]
      exact hhr
    · rw [show g' + (h - g') = h from by omega]
      exact hk
  exact walkSegE_glue sk hmono hend

-- ==================================== the E site futLen floors

private theorem qSum_monoE (pk : Party × Nat) (k : Nat) :
    ∀ {i i' : Nat}, i ≤ i' → qSum sk pk k i ≤ qSum sk pk k i' := by
  intro i i' hii
  induction i' with
  | zero =>
      have h0 : i = 0 := by omega
      subst h0
      exact Nat.le_refl _
  | succ i' ih =>
      by_cases hlast : i = i' + 1
      · subst hlast
        exact Nat.le_refl _
      · have hstep : qSum sk pk k i' ≤ qSum sk pk k (i' + 1) := by
          have := qSum_succ sk pk k i'
          omega
        exact Nat.le_trans (ih (by omega)) hstep

/-- The E resolution site's floors: the remaining resolution share
with its bound, and the pending-parent summary share — no splice
discriminant (cf. `futLen_site_lower`). -/
private theorem futLenE_site_lower {fut : List Ev} {h k i : Nat}
    (hk : k < sk.stageLen h)
    (hi : i < sk.nChildren h (sk.stageScope h k))
    (hD : sk.childIsD h (sk.stageScope h k) i = true)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((lowerOut (wpk h), true,
            sk.dsBefore h k + dRank sk (wpk h) k i) : Ev)
          :: (chunkQ sk h k i
              ++ (List.range' (i + 1)
                    (sk.nChildren h (sk.stageScope h k)
                      - (i + 1))).flatMap
                   (childChunk sk (wpk h) k)
              ++ ((upperOut (wpk h), true, k) : Ev)
                :: walkSegE sk h (k + 1) (sk.stageLen h))) :
    futLen sk fut (walkIdx sk h) (lowerOut (wpk h)) true
        = sk.dsBefore h (sk.stageLen h)
          - (sk.dsBefore h k + dRank sk (wpk h) k i)
      ∧ sk.dsBefore h k + dRank sk (wpk h) k i
          < sk.dsBefore h (sk.stageLen h)
      ∧ futLen sk fut (walkIdx sk h) (upperOut (wpk h)) true
        = sk.stageLen h - k := by
  have htot : dRank sk (wpk h) k (sk.nChildren h (sk.stageScope h k))
      = sk.dOf h (sk.stageScope h k) := dRank_total sk (wpk h) k
  have hds := dRank_succ sk (wpk h) k i
  rw [show sk.childIsD (wpk h).2 (sk.stageScope (wpk h).2 k) i
      = sk.childIsD h (sk.stageScope h k) i from rfl, hD,
    if_pos rfl] at hds
  have hsc : sk.dsBefore h (k + 1)
      = sk.dsBefore h k + sk.dOf h (sk.stageScope h k) :=
    dsBefore_succ sk hk
  have hmono : sk.dsBefore h (k + 1) ≤ sk.dsBefore h (sk.stageLen h) :=
    dsBefore_mono sk h (by omega)
  have hle : dRank sk (wpk h) k i + 1 ≤ sk.dOf h (sk.stageScope h k) :=
    dRank_succ_le_dOf sk (wpk h) hi hD
  have hidx : i + 1 + (sk.nChildren h (sk.stageScope h k) - (i + 1))
      = sk.nChildren h (sk.stageScope h k) := by omega
  refine ⟨?_, by omega, ?_⟩
  · have hqne : proj (lowerOut (wpk h)) true (chunkQ sk h k i) = [] :=
      chunkQ_proj_ne sk h k i (by
        rintro ⟨hc, -⟩
        simp only [askedOut, lowerOut] at hc
        split at hc <;> exact Chan.noConfusion hc)
    rw [futLen_of_filter sk hfil, proj_cons_self, proj_append,
      proj_append, hqne, childChunk_run_spliced,
      chunks_proj_res sk h k none _ (i + 1),
      proj_cons_ne_chan (by simp [upperOut, lowerOut]),
      walkSegE_proj_eq,
      walkSeg_proj_res sk (show k + 1 ≤ sk.stageLen h by omega)
        (Nat.le_refl _)]
    simp only [List.nil_append, List.length_cons, List.length_append,
      seg_len]
    rw [hidx]
    omega
  · have hqne : proj (upperOut (wpk h)) true (chunkQ sk h k i) = [] :=
      chunkQ_proj_ne sk h k i (by
        rintro ⟨hc, -⟩
        simp only [askedOut, upperOut] at hc
        split at hc <;> exact Chan.noConfusion hc)
    rw [futLen_of_filter sk hfil,
      proj_cons_ne_chan (by simp [lowerOut, upperOut]),
      proj_append, proj_append, hqne, childChunk_run_spliced,
      chunksNone_proj_upper, proj_cons_self, walkSegE_proj_eq,
      walkSeg_proj_upper sk (show k + 1 ≤ sk.stageLen h by omega)]
    simp only [List.nil_append, List.length_cons, seg_len]
    omega

/-- The E resolution site's query floor (cf. `futLen_SL_q`). -/
private theorem futLenE_SL_q {fut : List Ev} {h k i : Nat}
    (hk : k < sk.stageLen h)
    (hi : i < sk.nChildren h (sk.stageScope h k))
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((lowerOut (wpk h), true,
            sk.dsBefore h k + dRank sk (wpk h) k i) : Ev)
          :: (chunkQ sk h k i
              ++ (List.range' (i + 1)
                    (sk.nChildren h (sk.stageScope h k)
                      - (i + 1))).flatMap
                   (childChunk sk (wpk h) k)
              ++ ((upperOut (wpk h), true, k) : Ev)
                :: walkSegE sk h (k + 1) (sk.stageLen h))) :
    futLen sk fut (walkIdx sk h) (askedOut (wpk h)) true
      = sk.qsBefore h (sk.stageLen h)
        - (sk.qsBefore h k + qSum sk (wpk h) k i) := by
  have hcq : chunkQ sk h k i
      = seg (askedOut (wpk h)) true
          (sk.qsBefore h k + qSum sk (wpk h) k i)
          (sk.qCount h (sk.stageScope h k) i) := rfl
  rw [futLen_of_filter sk hfil,
    proj_cons_ne_chan (by
      unfold askedOut lowerOut
      split <;> simp),
    proj_append, proj_append, hcq, proj_seg_self,
    childChunk_run_spliced,
    chunks_proj_q sk h k none _ (i + 1),
    proj_cons_ne_chan (by
      unfold askedOut upperOut
      split <;> simp),
    walkSegE_proj_eq,
    walkSeg_proj_q sk (show k + 1 ≤ sk.stageLen h by omega)
      (Nat.le_refl _)]
  simp only [List.length_append, seg_len]
  have hidx : i + 1 + (sk.nChildren h (sk.stageScope h k) - (i + 1))
      = sk.nChildren h (sk.stageScope h k) := by omega
  rw [hidx]
  have htot : qSum sk (wpk h) k (sk.nChildren h (sk.stageScope h k))
      = sk.qOf h (sk.stageScope h k) := qSum_total sk (wpk h) k
  have hqs1 : qSum sk (wpk h) k (i + 1)
      = qSum sk (wpk h) k i + sk.qCount h (sk.stageScope h k) i :=
    qSum_succ sk (wpk h) k i
  have hqsuc := qsBefore_succ sk hk
  have hmono := qsBefore_mono sk h
    (show k + 1 ≤ sk.stageLen h from hk)
  have hqm : qSum sk (wpk h) k (i + 1)
      ≤ qSum sk (wpk h) k (sk.nChildren h (sk.stageScope h k)) :=
    qSum_monoE sk (wpk h) k hi
  omega

/-- The E wire site's floor and bound (cf. `futLen_site_wire`): the
pending parent rides the tail and contributes nothing. -/
private theorem futLenE_site_wire {fut : List Ev} {h k i : Nat}
    (hk : k < sk.stageLen h)
    (hi : i < sk.nChildren h (sk.stageScope h k))
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = (List.range' i
            (sk.nChildren h (sk.stageScope h k) - i)).flatMap
            (childChunk sk (wpk h) k)
          ++ ((upperOut (wpk h), true, k) : Ev)
            :: walkSegE sk h (k + 1) (sk.stageLen h)) :
    futLen sk fut (walkIdx sk h) (wireOut (wpk h)) true
        = sk.wiresBefore h (sk.stageLen h) - (sk.wiresBefore h k + i)
      ∧ sk.wiresBefore h k + i < sk.wiresBefore h (sk.stageLen h) := by
  have hws : sk.wiresBefore h (k + 1)
      = sk.wiresBefore h k + sk.nChildren h (sk.stageScope h k) :=
    wiresBefore_succ sk hk
  have hwm : sk.wiresBefore h (k + 1)
      ≤ sk.wiresBefore h (sk.stageLen h) :=
    wiresBefore_mono sk h (by omega)
  refine ⟨?_, by omega⟩
  rw [futLen_of_filter sk hfil, proj_append, childChunk_run_spliced,
    chunks_proj_wire sk h k none _ i,
    proj_cons_ne_chan (by simp [wireOut, upperOut]),
    walkSegE_proj_eq,
    walkSeg_proj_wire sk (show k + 1 ≤ sk.stageLen h by omega)
      (Nat.le_refl _)]
  simp only [List.length_append, seg_len]
  omega

/-- The E leaf-request site's wire count (cf. `futLen_Q0_wire`). -/
private theorem futLenE_Q0_wire {fut : List Ev} {k i0 : Nat}
    (hk : k < sk.stageLen 0)
    (hi0 : i0 < sk.nChildren 0 (sk.stageScope 0 k))
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk 0)
      = (List.range' (i0 + 1)
            (sk.nChildren 0 (sk.stageScope 0 k) - (i0 + 1))).flatMap
            (childChunk sk (wpk 0) k)
          ++ ((upperOut (wpk 0), true, k) : Ev)
            :: walkSegE sk 0 (k + 1) (sk.stageLen 0)) :
    futLen sk fut (walkIdx sk 0) (wireOut (wpk 0)) true
      = sk.wiresBefore 0 (sk.stageLen 0)
        - (sk.wiresBefore 0 k + i0 + 1) := by
  rw [futLen_of_filter sk hfil, proj_append, childChunk_run_spliced,
    chunks_proj_wire sk 0 k none _ (i0 + 1),
    proj_cons_ne_chan (by simp [wireOut, upperOut]),
    walkSegE_proj_eq,
    walkSeg_proj_wire sk (show k + 1 ≤ sk.stageLen 0 from hk)
      (Nat.le_refl _)]
  simp only [List.length_append, seg_len]
  have hws := wiresBefore_succ sk hk
  have hmono := wiresBefore_mono sk 0
    (show k + 1 ≤ sk.stageLen 0 from hk)
  omega

/-- The E ancestor query floor at a feed cursor (cf. `futLen_site_q`),
read off the telescope's fil shape. -/
private theorem futLenE_site_q {fut : List Ev} {h K i t : Nat}
    (hK : K < sk.stageLen h)
    (hi : i < sk.nChildren h (sk.stageScope h K))
    (ht : t < sk.qCount h (sk.stageScope h K) i)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = (chunkQ sk h K i).drop t
          ++ (List.range' (i + 1)
                (sk.nChildren h (sk.stageScope h K)
                  - (i + 1))).flatMap
               (childChunk sk (wpk h) K)
          ++ ((upperOut (wpk h), true, K) : Ev)
            :: walkSegE sk h (K + 1) (sk.stageLen h)) :
    futLen sk fut (walkIdx sk h) (askedOut (wpk h)) true
        = sk.qsBefore h (sk.stageLen h)
          - (sk.qsBefore h K + qSum sk (wpk h) K i + t)
      ∧ sk.qsBefore h K + qSum sk (wpk h) K i + t
          < sk.qsBefore h (sk.stageLen h) := by
  have hqs := qSum_succ sk (wpk h) K i
  rw [show sk.qCount (wpk h).2 (sk.stageScope (wpk h).2 K) i
      = sk.qCount h (sk.stageScope h K) i from rfl] at hqs
  have hidx : i + 1 + (sk.nChildren h (sk.stageScope h K) - (i + 1))
      = sk.nChildren h (sk.stageScope h K) := by omega
  have htot : qSum sk (wpk h) K (sk.nChildren h (sk.stageScope h K))
      = sk.qOf h (sk.stageScope h K) := qSum_total sk (wpk h) K
  have hqm : qSum sk (wpk h) K (i + 1)
      ≤ qSum sk (wpk h) K (sk.nChildren h (sk.stageScope h K)) :=
    qSum_monoE sk (wpk h) K hi
  have hsuc : sk.qsBefore h (K + 1)
      = sk.qsBefore h K + sk.qOf h (sk.stageScope h K) :=
    qsBefore_succ sk hK
  have hqsm : sk.qsBefore h (K + 1)
      ≤ sk.qsBefore h (sk.stageLen h) :=
    qsBefore_mono sk h (show K + 1 ≤ sk.stageLen h from hK)
  refine ⟨?_, by omega⟩
  rw [futLen_of_filter sk hfil, proj_append, proj_append,
    chunkQ_drop_proj_q sk h K i (by omega),
    childChunk_run_spliced,
    chunks_proj_q sk h K none _ (i + 1),
    proj_cons_ne_chan (by
      unfold askedOut upperOut
      split <;> simp),
    walkSegE_proj_eq,
    walkSeg_proj_q sk (show K + 1 ≤ sk.stageLen h by omega)
      (Nat.le_refl _)]
  simp only [List.length_append, seg_len]
  rw [hidx]
  omega

-- ==================================================== the ready sites

/-- The E parent site (the ONE new site shape of the campaign): a
scope's tail summary is emittable through the upper window — descent
from the clean subtree boundary, ascent and `P1` from margin 0. -/
theorem ready_upperE (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) {fut : List Ev} {h k : Nat}
    {A j t : Nat → Nat} (hhr : h < sk.rootH) (hk : k < sk.stageLen h)
    (hanc : AncTeleE sk h A j t fut)
    (hcoh0 : h + 1 < sk.rootH →
      k = sk.wiresBefore (h + 1) (A (h + 1)) + j (h + 1))
    (hfeed : ∃ i₀, fut.filter (fun e => evOwner sk e == 1)
      = ((ropenEvents sk).drop 3).drop i₀)
    (hdeep : ∀ g', g' < h →
      fut.filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSegE sk g'
            (descIdx sk g' (h - 1 - g') (sk.wiresBefore h (k + 1)))
            (sk.stageLen g'))
    (hown : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((upperOut (wpk h), true, k) : Ev)
          :: walkSegE sk h (k + 1) (sk.stageLen h)) :
    ∀ st : MState, WEdgeP sk (procsE sk) fut st → step sk st = none →
      enabled sk st.sent st.rcvd (upperOut (wpk h), true, k)
        = true := by
  intro st hW hfix
  have hna : asks ((wpk h).1) h = false := asks_wpk_self h
  have hasks : asks ((wpk h).1) (h + 1) = true := by
    have hs := asks_succ ((wpk h).1) h
    rw [hna] at hs
    simpa using hs
  have hfu := futLen_siteE_upper sk hk hown
  have hsnd : sndCount (Chan.upper ((wpk h).1) h) st.out = k :=
    upper_site_hsndE sk hwf hW.toWCountP hna hhr hk hfu
  have hcov := ancTele_covE sk hwf hm0 hW hanc hcoh0 hsnd
  have hroot := root_bankedE sk hwf hW.toWCountP hfeed
  have hdesc : DescSupply sk st ((wpk h).1) h
      (sk.pendsBefore ((wpk h).1) (h + 1) k) := by
    rcases Nat.eq_zero_or_pos h with rfl | h1
    · exact descSupply_upper_site_zero sk hasks _
    · have hfl := futLen_siteE_upper_res sk hk hown
      have hlpin := lower_snd_pinE sk hwf hW.toWCountP hhr
      have hdm1 := dsBefore_mono sk h
        (show k ≤ k + 1 from by omega)
      have hdm2 := dsBefore_mono sk h
        (show k + 1 ≤ sk.stageLen h from hk)
      have hXW : sk.wiresBefore h k ≤ sk.wiresBefore h (k + 1) :=
        wiresBefore_mono sk h (by omega)
      have hXle : sk.wiresBefore h (k + 1) ≤ sk.stageLen (h - 1) := by
        have h2' := wiresBefore_mono sk h
          (show k + 1 ≤ sk.stageLen h from hk)
        have h3' := wiresBefore_total sk hwf h1 hhr
        omega
      refine descSupply_upper_of_ctxE sk hwf hW.toWCountP h1 hhr hk
        hasks hXW hXle hdeep (by omega) ?_
      intro h1'
      subst h1'
      have hfq := futLen_siteE_upper_q sk hk hown
      have hqpin := asked_snd_pinE sk hwf hW.toWCountP (Nat.le_refl 1)
        hhr
      have hqm1 := qsBefore_mono sk 1
        (show k ≤ k + 1 from by omega)
      have hqm2 := qsBefore_mono sk 1
        (show k + 1 ≤ sk.stageLen 1 from hk)
      rw [show Chan.leafRequests = askedOut (wpk 1) from rfl]
      omega
  have hwin := upper_window sk hwf (famOK_procsE sk hwf) hW hfix
    (wpk_htop sk h) hasks (wtop_ge hwf hhr) hk hsnd hdesc hcov hroot
  exact enabled_of_windowE sk hwf hwin (hW.rcvd_eq _)

/-- The E resolution site (cf. `ready_lower`): margin 0 supplies the
schedulable slack and the ascent overhangs. -/
theorem ready_lowerE (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) {fut : List Ev} {h k i : Nat}
    {A j t : Nat → Nat} (hhr : h < sk.rootH) (hk : k < sk.stageLen h)
    (hi : i < sk.nChildren h (sk.stageScope h k))
    (hD : sk.childIsD h (sk.stageScope h k) i = true)
    (hanc : AncTeleE sk h A j t fut)
    (hcoh0 : h + 1 < sk.rootH →
      k = sk.wiresBefore (h + 1) (A (h + 1)) + j (h + 1))
    (hfeed : ∃ i₀, fut.filter (fun e => evOwner sk e == 1)
      = ((ropenEvents sk).drop 3).drop i₀)
    (hdeep : ∀ g', g' < h →
      fut.filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSegE sk g'
            (descIdx sk g' (h - 1 - g') (sk.wiresBefore h k + i))
            (sk.stageLen g'))
    (hown : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((lowerOut (wpk h), true,
            sk.dsBefore h k + dRank sk (wpk h) k i) : Ev)
          :: (chunkQ sk h k i
              ++ (List.range' (i + 1)
                    (sk.nChildren h (sk.stageScope h k)
                      - (i + 1))).flatMap
                   (childChunk sk (wpk h) k)
              ++ ((upperOut (wpk h), true, k) : Ev)
                :: walkSegE sk h (k + 1) (sk.stageLen h))) :
    ∀ st : MState, WEdgeP sk (procsE sk) fut st → step sk st = none →
      enabled sk st.sent st.rcvd
        (lowerOut (wpk h), true,
          sk.dsBefore h k + dRank sk (wpk h) k i) = true := by
  intro st hW hfix
  have h1 : 1 ≤ h := by
    rcases Nat.eq_zero_or_pos h with rfl | h1
    · exact Bool.noConfusion
        ((show sk.childIsD 0 (sk.stageScope 0 k) i = false from rfl)
          ▸ hD)
    · exact h1
  have hna : asks ((wpk h).1) h = false := asks_wpk_self h
  have hasks : asks ((wpk h).1) (h + 1) = true := by
    have hs := asks_succ ((wpk h).1) h
    rw [hna] at hs
    simpa using hs
  obtain ⟨hfl, hbnd, hfu⟩ := futLenE_site_lower sk hk hi hD hown
  have hsnd := lower_site_hsndE sk hwf hW.toWCountP hna hhr hfl hbnd
  have hupk := upper_site_hsndE sk hwf hW.toWCountP hna hhr hk hfu
  have hsched := margin0_schedulable sk hm0
  have hp1full := p1_of_lower_site sk hsched hk hi hD hupk hsnd
  have hroot := root_bankedE sk hwf hW.toWCountP hfeed
  have hcov : AscCover sk st ((wpk h).1) (h + 1) (wtop sk h) :=
    ascCover_pred sk (ancTele_covE sk hwf hm0 hW hanc hcoh0 hupk)
      hasks
  have hq1 : h = 1 →
      sk.wiresBefore 0 (sk.wiresBefore 1 k + i)
        ≤ sndCount Chan.leafRequests st.out := by
    intro h1'
    subst h1'
    have hfq := futLenE_SL_q sk hk hi hown
    have hqpin := asked_snd_pinE sk hwf hW.toWCountP (Nat.le_refl 1)
      hhr
    have hqw := qs_wires_mid sk hwf (Nat.le_refl 1) hhr hk
      (Nat.le_of_lt hi)
    rw [show (1 : Nat) - 1 = 0 from rfl] at hqw
    have hqs1 : qSum sk (wpk 1) k i
        ≤ qSum sk (wpk 1) k (sk.nChildren 1 (sk.stageScope 1 k)) :=
      qSum_monoE sk (wpk 1) k (Nat.le_of_lt hi)
    have htotq : qSum sk (wpk 1) k
        (sk.nChildren 1 (sk.stageScope 1 k))
        = sk.qOf 1 (sk.stageScope 1 k) := qSum_total sk (wpk 1) k
    have hqsuc := qsBefore_succ sk hk
    have hqmono := qsBefore_mono sk 1
      (show k + 1 ≤ sk.stageLen 1 from hk)
    rw [show Chan.leafRequests = askedOut (wpk 1) from rfl]
    omega
  have hdesc := descSupply_lower_of_ctxE sk hwf hW.toWCountP h1 hhr hk
    (Nat.le_of_lt hi) hna hdeep hq1
  have hd : sk.dsBefore h k + dRank sk (wpk h) k i
      < (sk.asmResList ((wpk h).1) h).length := by
    rw [answerer_resList_total hwf hna h1 hhr]
    exact hbnd
  rw [hsnd] at hp1full
  have hwin := lower_window sk hwf (famOK_procsE sk hwf) hW hfix
    (wpk_htop sk h) hna h1
    (show h < wtop sk h from by have := wtop_ge hwf hhr; omega)
    hd hsnd hp1full hdesc hcov hroot
  exact enabled_of_windowE sk hwf hwin (hW.rcvd_eq _)

/-- The E leaf-wire site (cf. `ready_wire0`). -/
theorem ready_wire0E (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) {fut : List Ev} {k i0 : Nat}
    {A j t : Nat → Nat} (hr : 0 < sk.rootH) (hk : k < sk.stageLen 0)
    (hi0 : i0 < sk.nChildren 0 (sk.stageScope 0 k))
    (hanc : AncTeleE sk 0 A j t fut)
    (hcoh0 : k = sk.wiresBefore 1 (A 1) + j 1) (ht1 : t 1 = i0)
    (hfeed : ∃ i₀, fut.filter (fun e => evOwner sk e == 1)
      = ((ropenEvents sk).drop 3).drop i₀)
    (hown : fut.filter (fun e => evOwner sk e == walkIdx sk 0)
      = (List.range' i0
            (sk.nChildren 0 (sk.stageScope 0 k) - i0)).flatMap
            (childChunk sk (wpk 0) k)
          ++ ((upperOut (wpk 0), true, k) : Ev)
            :: walkSegE sk 0 (k + 1) (sk.stageLen 0)) :
    ∀ st : MState, WEdgeP sk (procsE sk) fut st → step sk st = none →
      enabled sk st.sent st.rcvd
        (wireOut (wpk 0), true, sk.wiresBefore 0 k + i0) = true := by
  intro st hW hfix
  have hr2 : 1 < sk.rootH := by have := (wf_rootH hwf).2; omega
  obtain ⟨hA1, hj1⟩ := hanc.rng 1 (by omega) hr2
  obtain ⟨hfw, hwbnd⟩ := futLenE_site_wire sk hk hi0 hown
  have hsnd := wire0_site_hsndE sk hwf hW.toWCountP hr hfw hwbnd
  have hw : sk.wiresBefore 0 k + i0 < sk.totalLeafReqs := by
    have := wiresBefore_full_leaf hwf
    omega
  have hqc : sk.qCount 1 (sk.stageScope 1 (A 1)) (j 1)
      = sk.nChildren 0 (sk.stageScope 0 k) := by
    have hq := qCount_eq_kid_nChildren sk hwf (Nat.le_refl 1) hr2
      hA1 hj1
    rw [show (1 : Nat) - 1 = 0 from rfl, ← hcoh0] at hq
    exact hq
  obtain ⟨hfq, hqbnd⟩ := futLenE_site_q sk hA1 hj1
    (by rw [ht1, hqc]; exact hi0) (hanc.fil 1 (by omega) hr2)
  have hsndq := leafreq_site_hsndE sk hwf hW.toWCountP hr2 hfq hqbnd
  have hqw := qs_wires_mid sk hwf (Nat.le_refl 1) hr2 hA1
    (Nat.le_of_lt hj1)
  rw [show (1 : Nat) - 1 = 0 from rfl] at hqw
  rw [hqw, ← hcoh0, ht1] at hsndq
  have hreq : sk.wiresBefore 0 k + i0
      ≤ sndCount Chan.leafRequests st.out + 1 := by omega
  have hcov := ancTele_cov_leafE sk hwf hm0 hW hanc hr2 hk hcoh0
    hi0 hsndq
  have hroot := root_bankedE sk hwf hW.toWCountP hfeed
  have hwin := wire0_window sk hwf (famOK_procsE sk hwf) hW hfix hw
    hsnd hreq hcov hroot
  exact enabled_of_windowE sk hwf hwin (hW.rcvd_eq _)

/-- The E leaf-request site (cf. `ready_leafreq`). -/
theorem ready_leafreqE (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) {fut : List Ev} {k i0 : Nat}
    {A j t : Nat → Nat} (hr : 0 < sk.rootH) (hk : k < sk.stageLen 0)
    (hi0 : i0 < sk.nChildren 0 (sk.stageScope 0 k))
    (hanc : AncTeleE sk 0 A j t fut)
    (hcoh0 : k = sk.wiresBefore 1 (A 1) + j 1) (ht1 : t 1 = i0)
    (hfeed : ∃ i₀, fut.filter (fun e => evOwner sk e == 1)
      = ((ropenEvents sk).drop 3).drop i₀)
    (hown : fut.filter (fun e => evOwner sk e == walkIdx sk 0)
      = (List.range' (i0 + 1)
            (sk.nChildren 0 (sk.stageScope 0 k) - (i0 + 1))).flatMap
            (childChunk sk (wpk 0) k)
          ++ ((upperOut (wpk 0), true, k) : Ev)
            :: walkSegE sk 0 (k + 1) (sk.stageLen 0)) :
    ∀ st : MState, WEdgeP sk (procsE sk) fut st → step sk st = none →
      enabled sk st.sent st.rcvd
        (askedOut (wpk 1), true, sk.wiresBefore 0 k + i0) = true := by
  intro st hW hfix
  have hr2 : 1 < sk.rootH := by have := (wf_rootH hwf).2; omega
  obtain ⟨hA1, hj1⟩ := hanc.rng 1 (by omega) hr2
  have hqc : sk.qCount 1 (sk.stageScope 1 (A 1)) (j 1)
      = sk.nChildren 0 (sk.stageScope 0 k) := by
    have hq := qCount_eq_kid_nChildren sk hwf (Nat.le_refl 1) hr2
      hA1 hj1
    rw [show (1 : Nat) - 1 = 0 from rfl, ← hcoh0] at hq
    exact hq
  obtain ⟨hfq, hqbnd⟩ := futLenE_site_q sk hA1 hj1
    (by rw [ht1, hqc]; exact hi0) (hanc.fil 1 (by omega) hr2)
  have hsndq := leafreq_site_hsndE sk hwf hW.toWCountP hr2 hfq hqbnd
  have hqw := qs_wires_mid sk hwf (Nat.le_refl 1) hr2 hA1
    (Nat.le_of_lt hj1)
  rw [show (1 : Nat) - 1 = 0 from rfl] at hqw
  rw [hqw, ← hcoh0, ht1] at hsndq
  have hq : sk.wiresBefore 0 k + i0 < sk.totalLeafReqs := by
    have hfull := qsBefore_full_leaf hwf
    have hcong : sk.wiresBefore 0 k
        = sk.wiresBefore 0 (sk.wiresBefore 1 (A 1) + j 1) := by
      rw [hcoh0]
    omega
  have hfw := futLenE_Q0_wire sk hk hi0 hown
  have hwpin := wire_snd_pinE sk hwf hW.toWCountP hr
  have hwire : sk.wiresBefore 0 k + i0
      ≤ sndCount (Chan.wire Party.R 0) st.out := by
    rw [show Chan.wire Party.R 0 = wireOut (wpk 0) from rfl]
    have hws := wiresBefore_succ sk hk
    have hmono := wiresBefore_mono sk 0
      (show k + 1 ≤ sk.stageLen 0 from hk)
    omega
  have hcov := ancTele_cov_leafE sk hwf hm0 hW hanc hr2 hk hcoh0
    hi0 hsndq
  have hroot := root_bankedE sk hwf hW.toWCountP hfeed
  have hwin := leafreq_window sk hwf (famOK_procsE sk hwf) hW hfix hq
    hsndq hwire hcov hroot
  exact enabled_of_windowE sk hwf hwin (hW.rcvd_eq _)

end StreamingMirror.Sched
