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
import StreamingMirror.Proofs.Sched.Weave.PrecE

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
    (hrest : ∀ g', g' < h →
      rest.filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSegE sk g' (descIdx sk g' (h - g') (k + 1))
            (sk.stageLen g')) :
    ∀ g', g' < h →
      (suffix ++ rest).filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSegE sk g'
            (descIdx sk g' (h - 1 - g') (sk.wiresBefore h k + i))
            (sk.stageLen g') := by
  intro g' hg'
  rw [List.filter_append, hsuf g' hg', hrest g' hg']
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

-- ============================================== the leaf-stage scopes

/-- The E master induction's leaf case: the prologue receives from
their in-flight predecessors, each slot's wire and feed query through
the absorber windows, and the parent summary LAST through the upper
window — the epilogue placement. -/
theorem emitOK_scope_zeroE (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) {k : Nat} {rest : List Ev}
    {A j t : Nat → Nat} (hk : k < sk.stageLen 0)
    (hlow : ∀ g', g' ≤ 0 →
      rest.filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSegE sk g' (descIdx sk g' (0 - g') (k + 1))
            (sk.stageLen g'))
    (hanc : AncTeleE sk 0 A j t rest)
    (hcoh0 : k = sk.wiresBefore 1 (A 1) + j 1)
    (hsat : (chunkQ sk 1 (A 1) (j 1)).drop (t 1) = [])
    (hfd : ∃ i₀, rest.filter (fun e => evOwner sk e == 1)
      = ((ropenEvents sk).drop 3).drop i₀) :
    EmitOKOnE sk (opEventsE sk (.scope 0 k (scopeFeed sk 0 k)))
      rest := by
  have hr2 : 1 < sk.rootH := by have := (wf_rootH hwf).2; omega
  have hr0 : 0 < sk.rootH := by omega
  obtain ⟨hA1, hj1⟩ := hanc.rng 1 (by omega) hr2
  have hF : (scopeFeed sk 0 k).length
      = sk.nChildren 0 (sk.stageScope 0 k) := scopeFeed_length sk 0 k
  have hFo : ∀ e ∈ scopeFeed sk 0 k, evOwner sk e = walkIdx sk 1 := by
    intro e he
    unfold scopeFeed seg at he
    obtain ⟨j', -, rfl⟩ := List.mem_map.1 he
    exact evOwner_askedOut sk (Nat.le_refl 1) hr2 _
  have hmF : walkIdx sk 1 < walkIdx sk 0 :=
    walkIdx_lt sk (by omega) hr2
  have hw2 : ∀ h', h' < sk.rootH → 2 ≤ walkIdx sk h' := by
    intro h' _
    unfold walkIdx
    omega
  have hFeq : chunkQ sk 1 (A 1) (j 1) = scopeFeed sk 0 k := by
    have hq := chunkQ_eq_feed sk hwf (hp := 0) hr2 hA1 hj1
    rw [← hcoh0] at hq
    exact hq
  have hlow0 : rest.filter (fun e => evOwner sk e == walkIdx sk 0)
      = walkSegE sk 0 (k + 1) (sk.stageLen 0) := by
    have h0 := hlow 0 (Nat.le_refl 0)
    rw [show ((0 : Nat) - 0) = 0 from rfl, descIdx_zero] at h0
    exact h0
  have hfil1 : rest.filter (fun e => evOwner sk e == walkIdx sk 1)
      = (List.range' (j 1 + 1)
            (sk.nChildren 1 (sk.stageScope 1 (A 1)) - (j 1 + 1))).flatMap
           (childChunk sk (wpk 1) (A 1))
        ++ ((upperOut (wpk 1), true, A 1) : Ev)
          :: walkSegE sk 1 (A 1 + 1) (sk.stageLen 1) := by
    have hf := hanc.fil 1 (by omega) hr2
    rw [hsat, List.nil_append] at hf
    exact hf
  -- the fold's tail carries the pending parent
  have hlow0' : ((((upperOut (wpk 0), true, k) : Ev) :: rest)).filter
      (fun e => evOwner sk e == walkIdx sk 0)
      = ((upperOut (wpk 0), true, k) : Ev)
        :: walkSegE sk 0 (k + 1) (sk.stageLen 0) := by
    rw [List.filter_cons_of_pos (by
        simp only [evOwner_upperOut, beq_self_eq_true]), hlow0]
  have hfil1' : ((((upperOut (wpk 0), true, k) : Ev) :: rest)).filter
      (fun e => evOwner sk e == walkIdx sk 1)
      = (List.range' (j 1 + 1)
            (sk.nChildren 1 (sk.stageScope 1 (A 1)) - (j 1 + 1))).flatMap
           (childChunk sk (wpk 1) (A 1))
        ++ ((upperOut (wpk 1), true, A 1) : Ev)
          :: walkSegE sk 1 (A 1 + 1) (sk.stageLen 1) := by
    rw [List.filter_cons_of_neg (by
        simp only [evOwner_upperOut, beq_iff_eq]
        intro hc
        exact absurd (walkIdx_inj hr0 hr2 hc) (by omega)), hfil1]
  have hone : ∀ (pre : List Ev),
      pre.filter (fun e => evOwner sk e == 1) = [] →
      ∃ i₀, (pre ++ (((upperOut (wpk 0), true, k) : Ev) :: rest)).filter
          (fun e => evOwner sk e == 1)
        = ((ropenEvents sk).drop 3).drop i₀ := by
    intro pre hpre
    obtain ⟨i₀, hf⟩ := hfd
    refine ⟨i₀, ?_⟩
    rw [List.filter_append, hpre, List.nil_append,
      List.filter_cons_of_neg (by
        simp only [evOwner_upperOut, beq_iff_eq]
        have := hw2 0 hr0
        omega), hf]
  have hpar : ∀ (pre : List Ev) (c : Nat),
      pre.filter (fun e => evOwner sk e == walkIdx sk 1)
        = (scopeFeed sk 0 k).drop c →
      (pre ++ (((upperOut (wpk 0), true, k) : Ev) :: rest)).filter
          (fun e => evOwner sk e == walkIdx sk 1)
        = (chunkQ sk 1 (A 1) (j 1)).drop c
          ++ (List.range' (j 1 + 1)
                (sk.nChildren 1 (sk.stageScope 1 (A 1))
                  - (j 1 + 1))).flatMap
               (childChunk sk (wpk 1) (A 1))
          ++ ((upperOut (wpk 1), true, A 1) : Ev)
            :: walkSegE sk 1 (A 1 + 1) (sk.stageLen 1) := by
    intro pre c hpre
    rw [List.filter_append, hpre, hfil1', hFeq, ← List.append_assoc]
  have hancU : AncTeleE sk 0 A j t
      (((upperOut (wpk 0), true, k) : Ev) :: rest) := by
    refine ⟨hanc.rng, hanc.isD, hanc.coh, ?_⟩
    intro G hG hGr
    rw [List.filter_cons_of_neg (by
        simp only [evOwner_upperOut, beq_iff_eq]
        intro hc
        exact absurd (walkIdx_inj hr0 hGr hc) (by omega))]
    exact hanc.fil G hG hGr
  -- the expansion
  have hE := opEventsE_scope_eq sk (Nat.le_of_lt hk) (scopeFeed sk 0 k)
  rw [List.range_eq_range'] at hE
  rw [hE]
  refine emitOKOn_cons sk
    (fun st hW hfix hpred => head_rcv_wireE sk hwf hW.toWCountP hpred)
    ?_
  refine emitOKOn_cons sk
    (fun st hW hfix hpred => head_rcv_askedE sk hwf hW.toWCountP hpred)
    ?_
  refine emitOKOn_append sk ?_ ?_
  · -- the leaf slots, folded against the pending parent
    have hfold : ∀ (m i : Nat),
        i + m = sk.nChildren 0 (sk.stageScope 0 k) →
        EmitOKOnP sk (procsE sk)
          ((List.range' i m).flatMap fun i' =>
            opEventsE sk (.kid 0 k (sk.stageScope 0 k)
              none (sk.wiresBefore 0 k) i' (scopeFeed sk 0 k)))
          (((upperOut (wpk 0), true, k) : Ev) :: rest) := by
      intro m
      induction m with
      | zero =>
          intro i _
          exact emitOKOn_nil sk _
      | succ m ihm =>
          intro i hin
          have hi : i < sk.nChildren 0 (sk.stageScope 0 k) := by
            omega
          obtain ⟨-, hksI2, hksI3, -⟩ :=
            align_kids_suffixE sk hwf hr0 hk hF hFo hmF
              (i := i + 1) (by omega)
          have hkidE : opEventsE sk (.kid 0 k (sk.stageScope 0 k)
                none (sk.wiresBefore 0 k) i (scopeFeed sk 0 k))
              = ((wireOut (wpk 0), true,
                    sk.wiresBefore 0 k + i) : Ev)
                :: (scopeFeed sk 0 k)[i]?.toList := by
            rw [opEventsE_kid_eq,
              if_neg (by
                rw [show sk.childIsD 0 (sk.stageScope 0 k) i = false
                    from rfl]
                exact Bool.false_ne_true),
              if_pos (show ((0 : Nat) == 0) = true from rfl),
              List.append_nil]
          have hqel := scopeFeed_getElem? sk (h := 0) (k := k) hi
          rw [show (0 : Nat) + 1 = 1 from rfl] at hqel
          have hdropi : (scopeFeed sk 0 k).drop i
              = ((askedOut (wpk 1), true,
                    sk.wiresBefore 0 k + i) : Ev)
                :: (scopeFeed sk 0 k).drop (i + 1) := by
            have hm := toList_drop_merge
              (l := scopeFeed sk 0 k) (i := i) (by rw [hF]; exact hi)
            rw [hqel, Option.toList_some, List.singleton_append] at hm
            exact hm.symm
          have hpeel : (List.range' i
                (sk.nChildren 0 (sk.stageScope 0 k) - i)).flatMap
                (childChunk sk (wpk 0) k)
              = ((wireOut (wpk 0), true,
                    sk.wiresBefore 0 k + i) : Ev)
                :: (List.range' (i + 1)
                    (sk.nChildren 0 (sk.stageScope 0 k)
                      - (i + 1))).flatMap
                  (childChunk sk (wpk 0) k) := by
            rw [show sk.nChildren 0 (sk.stageScope 0 k) - i
                = (sk.nChildren 0 (sk.stageScope 0 k) - (i + 1)) + 1
                from by omega, List.range'_succ, List.flatMap_cons,
              childChunk_eq,
              if_neg (by
                rw [show sk.childIsD 0 (sk.stageScope 0 k) i = false
                    from rfl]
                exact Bool.false_ne_true),
              List.singleton_append]
          have hknG : ∀ G, 2 ≤ G → G < sk.rootH →
              ((List.range' (i + 1)
                  (sk.nChildren 0 (sk.stageScope 0 k)
                    - (i + 1))).flatMap
                (fun i' => opEventsE sk (.kid 0 k (sk.stageScope 0 k)
                  none (sk.wiresBefore 0 k) i'
                  (scopeFeed sk 0 k)))).filter
                (fun e => evOwner sk e == walkIdx sk G) = [] := by
            intro G hG2 hGr
            exact kids_filter_neE sk hwf hr0 hk hF hFo hmF
              (i := i + 1) (by omega) (M := walkIdx sk G)
              (fun hc => absurd (walkIdx_inj hr2 hGr hc) (by omega))
              (fun h' hle hc =>
                absurd (walkIdx_inj (by omega) hGr hc) (by omega))
          have hkn1 : ((List.range' (i + 1)
              (sk.nChildren 0 (sk.stageScope 0 k)
                - (i + 1))).flatMap
              (fun i' => opEventsE sk (.kid 0 k (sk.stageScope 0 k)
                none (sk.wiresBefore 0 k) i'
                (scopeFeed sk 0 k)))).filter
              (fun e => evOwner sk e == 1) = [] :=
            kids_filter_neE sk hwf hr0 hk hF hFo hmF
              (i := i + 1) (by omega) (M := 1)
              (by have := hw2 1 hr2; omega)
              (fun h' hle => by have := hw2 h' (by omega); omega)
          rw [List.range'_succ, List.flatMap_cons, hkidE, hqel,
            Option.toList_some,
            show m = sk.nChildren 0 (sk.stageScope 0 k) - (i + 1)
              from by omega]
          refine emitOKOn_append sk ?_ (by
            have hih := ihm (i + 1) (by omega)
            rwa [show m = sk.nChildren 0 (sk.stageScope 0 k) - (i + 1)
              from by omega] at hih)
          generalize hLgen : (List.range' (i + 1)
              (sk.nChildren 0 (sk.stageScope 0 k)
                - (i + 1))).flatMap
              (fun i' => opEventsE sk (.kid 0 k (sk.stageScope 0 k)
                none (sk.wiresBefore 0 k) i'
                (scopeFeed sk 0 k))) = L
            at hksI2 hksI3 hknG hkn1 ⊢
          refine emitOKOn_cons sk ?_ ?_
          · -- W0: the slot's wire through the absorber's wire window
            intro st hW hfix hpred
            have hpreW : (((wireOut (wpk 0), true,
                  sk.wiresBefore 0 k + i) : Ev)
                :: ([((askedOut (wpk 1), true,
                      sk.wiresBefore 0 k + i) : Ev)] ++ L)).filter
                (fun e => evOwner sk e == walkIdx sk 1)
                = (scopeFeed sk 0 k).drop i := by
              rw [List.filter_cons_of_neg (by
                  simp only [evOwner_wireOut sk hr0, beq_iff_eq]
                  intro hc
                  exact absurd (walkIdx_inj hr0 hr2 hc) (by omega)),
                List.filter_append,
                List.filter_cons_of_pos (by
                  simp only [evOwner_askedOut sk (Nat.le_refl 1) hr2,
                    beq_self_eq_true]),
                List.filter_nil, List.singleton_append, hksI2,
                ← hdropi]
            refine ready_wire0E sk hwf hm0 hr0 hk hi (A := A)
              (j := j) (t := fun G => if G = 0 + 1 then i else t G)
              ?_ hcoh0 rfl ?_ ?_ st hW hfix
            · refine ancTeleE_rebase sk
                (pre := ((wireOut (wpk 0), true,
                    sk.wiresBefore 0 k + i) : Ev)
                  :: ([((askedOut (wpk 1), true,
                        sk.wiresBefore 0 k + i) : Ev)] ++ L))
                hancU ?_ ?_
              · intro G hG2 hGr
                rw [List.filter_cons_of_neg (by
                    simp only [evOwner_wireOut sk hr0, beq_iff_eq]
                    intro hc
                    exact absurd (walkIdx_inj hr0 hGr hc) (by omega)),
                  List.filter_append,
                  List.filter_cons_of_neg (by
                    simp only [evOwner_askedOut sk (Nat.le_refl 1) hr2,
                      beq_iff_eq]
                    intro hc
                    exact absurd (walkIdx_inj hr2 hGr hc) (by omega)),
                  List.filter_nil, List.nil_append]
                exact hknG G hG2 hGr
              · intro _
                exact hpar _ i hpreW
            · refine hone (((wireOut (wpk 0), true,
                  sk.wiresBefore 0 k + i) : Ev)
                :: ([((askedOut (wpk 1), true,
                      sk.wiresBefore 0 k + i) : Ev)] ++ L)) ?_
              rw [List.filter_cons_of_neg (by
                  simp only [evOwner_wireOut sk hr0, beq_iff_eq]
                  have := hw2 0 hr0
                  omega),
                List.filter_append,
                List.filter_cons_of_neg (by
                  simp only [evOwner_askedOut sk (Nat.le_refl 1) hr2,
                    beq_iff_eq]
                  have := hw2 1 hr2
                  omega),
                List.filter_nil, List.nil_append]
              exact hkn1
            · rw [List.filter_cons_of_pos (by
                  simp only [evOwner_wireOut sk hr0, beq_self_eq_true]),
                List.filter_append,
                List.filter_cons_of_neg (by
                  simp only [evOwner_askedOut sk (Nat.le_refl 1) hr2,
                    beq_iff_eq]
                  intro hc
                  exact absurd (walkIdx_inj hr2 hr0 hc) (by omega)),
                List.filter_nil, List.nil_append, List.filter_append,
                hksI3, hlow0', hpeel, List.cons_append]
          · -- Q0: the slot's feed query through the request window
            refine emitOKOn_cons sk ?_ (emitOKOn_nil sk _)
            intro st hW hfix hpred
            rw [List.nil_append] at hW
            have hpreQ : (((askedOut (wpk 1), true,
                  sk.wiresBefore 0 k + i) : Ev) :: L).filter
                (fun e => evOwner sk e == walkIdx sk 1)
                = (scopeFeed sk 0 k).drop i := by
              rw [List.filter_cons_of_pos (by
                  simp only [evOwner_askedOut sk (Nat.le_refl 1) hr2,
                    beq_self_eq_true]),
                hksI2, ← hdropi]
            refine ready_leafreqE sk hwf hm0 hr0 hk hi (A := A)
              (j := j) (t := fun G => if G = 0 + 1 then i else t G)
              ?_ hcoh0 rfl ?_ ?_ st hW hfix
            · refine ancTeleE_rebase sk
                (pre := ((askedOut (wpk 1), true,
                    sk.wiresBefore 0 k + i) : Ev) :: L) hancU ?_ ?_
              · intro G hG2 hGr
                rw [List.filter_cons_of_neg (by
                    simp only [evOwner_askedOut sk (Nat.le_refl 1) hr2,
                      beq_iff_eq]
                    intro hc
                    exact absurd (walkIdx_inj hr2 hGr hc) (by omega))]
                exact hknG G hG2 hGr
              · intro _
                exact hpar _ i hpreQ
            · refine hone (((askedOut (wpk 1), true,
                  sk.wiresBefore 0 k + i) : Ev) :: L) ?_
              rw [List.filter_cons_of_neg (by
                  simp only [evOwner_askedOut sk (Nat.le_refl 1) hr2,
                    beq_iff_eq]
                  have := hw2 1 hr2
                  omega)]
              exact hkn1
            · rw [List.filter_cons_of_neg (by
                  simp only [evOwner_askedOut sk (Nat.le_refl 1) hr2,
                    beq_iff_eq]
                  intro hc
                  exact absurd (walkIdx_inj hr2 hr0 hc) (by omega)),
                List.filter_append, hksI3, hlow0']
    have hgoal := hfold (sk.nChildren 0 (sk.stageScope 0 k)) 0
      (by omega)
    exact hgoal
  · -- the tail parent through the upper window
    refine emitOKOn_cons sk ?_ (emitOKOn_nil sk _)
    intro st hW hfix _
    rw [List.nil_append] at hW
    refine ready_upperE sk hwf hm0 hr0 hk hancU (fun _ => hcoh0) ?_
      (fun g' hg' => absurd hg' (Nat.not_lt_zero g')) hlow0' st hW hfix
    obtain ⟨i₀, hf⟩ := hfd
    refine ⟨i₀, ?_⟩
    rw [List.filter_cons_of_neg (by
        simp only [evOwner_upperOut, beq_iff_eq]
        have := hw2 0 hr0
        omega), hf]

-- ============================================ the interior-stage fold

/-- The E kids-fold at an interior stage (cf. `emitOK_kids`): wires
and feed queries from their manual predecessors, resolutions through
the lower window, each kid's subtree through the stage-below induction
hypothesis with the pushed rolling context. No splice case exists —
the parent summary rides `rest`'s head, placed by the scope's own
expansion. -/
private theorem emitOK_kidsE (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) {hp : Nat}
    (hh : hp + 1 < sk.rootH) {k : Nat}
    (hk : k < sk.stageLen (hp + 1)) {rest : List Ev}
    {A j t : Nat → Nat} {mF : Nat}
    (hFo : ∀ e ∈ scopeFeed sk (hp + 1) k, evOwner sk e = mF)
    (hmF : mF < walkIdx sk (hp + 1))
    (hmFeq : hp + 1 + 1 < sk.rootH → mF = walkIdx sk (hp + 1 + 1))
    (hlowD : ∀ g', g' ≤ hp →
      rest.filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSegE sk g' (descIdx sk g' (hp + 1 - g') (k + 1))
            (sk.stageLen g'))
    (hlowO : rest.filter (fun e => evOwner sk e == walkIdx sk (hp + 1))
      = ((upperOut (wpk (hp + 1)), true, k) : Ev)
          :: walkSegE sk (hp + 1) (k + 1) (sk.stageLen (hp + 1)))
    (hanc : AncTeleE sk (hp + 1) A j t rest)
    (hcoh0 : hp + 1 + 1 < sk.rootH →
      k = sk.wiresBefore (hp + 1 + 1) (A (hp + 1 + 1))
          + j (hp + 1 + 1))
    (hsat : hp + 1 + 1 < sk.rootH →
      (chunkQ sk (hp + 1 + 1) (A (hp + 1 + 1)) (j (hp + 1 + 1))).drop
        (t (hp + 1 + 1)) = [])
    (hfd : ∀ (pre : List Ev) (c : Nat),
      pre.filter (fun e => evOwner sk e == mF)
        = (scopeFeed sk (hp + 1) k).drop c →
      (∀ M, (∀ h', h' ≤ hp + 1 → walkIdx sk h' ≠ M) → mF ≠ M →
        pre.filter (fun e => evOwner sk e == M) = []) →
      ∃ i₀, (pre ++ rest).filter (fun e => evOwner sk e == 1)
        = ((ropenEvents sk).drop 3).drop i₀)
    (IH : ∀ (k' : Nat) (rest' : List Ev) (A' j' t' : Nat → Nat),
      k' < sk.stageLen hp →
      (∀ g', g' ≤ hp →
        rest'.filter (fun e => evOwner sk e == walkIdx sk g')
          = walkSegE sk g' (descIdx sk g' (hp - g') (k' + 1))
              (sk.stageLen g')) →
      AncTeleE sk hp A' j' t' rest' →
      (hp + 1 < sk.rootH →
        k' = sk.wiresBefore (hp + 1) (A' (hp + 1)) + j' (hp + 1)) →
      (hp + 1 < sk.rootH →
        (chunkQ sk (hp + 1) (A' (hp + 1)) (j' (hp + 1))).drop
          (t' (hp + 1)) = []) →
      (∀ (pre : List Ev) (c : Nat),
        pre.filter (fun e => evOwner sk e == walkIdx sk (hp + 1))
          = (scopeFeed sk hp k').drop c →
        (∀ M, (∀ h', h' ≤ hp → walkIdx sk h' ≠ M) →
          walkIdx sk (hp + 1) ≠ M →
          pre.filter (fun e => evOwner sk e == M) = []) →
        ∃ i₀, (pre ++ rest').filter (fun e => evOwner sk e == 1)
          = ((ropenEvents sk).drop 3).drop i₀) →
      EmitOKOnE sk (opEventsE sk (.scope hp k' (scopeFeed sk hp k')))
        rest') :
    ∀ (m i : Nat),
      i + m = sk.nChildren (hp + 1) (sk.stageScope (hp + 1) k) →
      EmitOKOnP sk (procsE sk)
        ((List.range' i m).flatMap fun i' =>
          opEventsE sk (.kid (hp + 1) k (sk.stageScope (hp + 1) k)
            none (sk.wiresBefore (hp + 1) k) i'
            (scopeFeed sk (hp + 1) k)))
        rest := by
  intro m
  induction m with
  | zero =>
      intro i _
      exact emitOKOn_nil sk rest
  | succ m ihm =>
      intro i hin
      have hi : i < sk.nChildren (hp + 1) (sk.stageScope (hp + 1) k) :=
        by omega
      have hkid : sk.wiresBefore (hp + 1) k + i < sk.stageLen hp :=
        kid_index_lt sk hwf (by omega) hh hk hi
      have hw2 : ∀ h', 2 ≤ walkIdx sk h' := by
        intro h'
        unfold walkIdx
        omega
      -- the feed read at slot i
      have hqel := scopeFeed_getElem? sk (h := hp + 1) (k := k) hi
      have hfQmem : ((askedOut (wpk (hp + 1 + 1)), true,
          sk.wiresBefore (hp + 1) k + i) : Ev)
          ∈ scopeFeed sk (hp + 1) k :=
        List.mem_of_getElem? hqel
      have hfQo : evOwner sk ((askedOut (wpk (hp + 1 + 1)), true,
          sk.wiresBefore (hp + 1) k + i) : Ev) = mF :=
        hFo _ hfQmem
      have hdropi : (scopeFeed sk (hp + 1) k).drop i
          = ((askedOut (wpk (hp + 1 + 1)), true,
              sk.wiresBefore (hp + 1) k + i) : Ev)
            :: (scopeFeed sk (hp + 1) k).drop (i + 1) := by
        have hm := toList_drop_merge (l := scopeFeed sk (hp + 1) k)
          (i := i) (by rw [scopeFeed_length]; exact hi)
        rw [hqel, Option.toList_some, List.singleton_append] at hm
        exact hm.symm
      -- the subtree's feed and alignment
      have hFeq : chunkQ sk (hp + 1) k i
          = scopeFeed sk hp (sk.wiresBefore (hp + 1) k + i) :=
        chunkQ_eq_feed sk hwf hh hk hi
      have hF' : (scopeFeed sk hp
            (sk.wiresBefore (hp + 1) k + i)).length
          = sk.nChildren hp
              (sk.stageScope hp (sk.wiresBefore (hp + 1) k + i)) :=
        scopeFeed_length sk hp _
      have hFo' : ∀ e ∈ scopeFeed sk hp
            (sk.wiresBefore (hp + 1) k + i),
          evOwner sk e = walkIdx sk (hp + 1) := by
        intro e he
        unfold scopeFeed seg at he
        obtain ⟨j', -, rfl⟩ := List.mem_map.1 he
        exact evOwner_askedOut sk (by omega) hh _
      have hmF' : walkIdx sk (hp + 1) < walkIdx sk hp :=
        walkIdx_lt sk (by omega) hh
      obtain ⟨-, hsc2, hsc3⟩ := align_scopeE sk hwf hp
        (sk.wiresBefore (hp + 1) k + i)
        (scopeFeed sk hp (sk.wiresBefore (hp + 1) k + i))
        (walkIdx sk (hp + 1)) (by omega) hkid hF' hFo' hmF'
      have hscnil : ∀ M, walkIdx sk (hp + 1) ≠ M →
          (∀ h', h' ≤ hp → walkIdx sk h' ≠ M) →
          (opEventsE sk (.scope hp (sk.wiresBefore (hp + 1) k + i)
              (scopeFeed sk hp
                (sk.wiresBefore (hp + 1) k + i)))).filter
            (fun e => evOwner sk e == M) = [] :=
        fun M h1 h2 => scope_filter_neE sk hwf (by omega) hkid hF' hFo'
          hmF' h1 h2
      -- expand the current slot in the goal
      rw [List.range'_succ, List.flatMap_cons,
        show m = sk.nChildren (hp + 1) (sk.stageScope (hp + 1) k)
          - (i + 1) from by omega]
      -- the kid-suffix clauses at i + 1
      obtain ⟨-, hks2, hks3, hks4⟩ := align_kids_suffixE sk hwf hh hk
        (scopeFeed_length sk (hp + 1) k) hFo hmF (i := i + 1)
        (by omega)
      have hksU : ∀ M, (∀ h', h' ≤ hp + 1 → walkIdx sk h' ≠ M) →
          mF ≠ M →
          ((List.range' (i + 1)
              (sk.nChildren (hp + 1) (sk.stageScope (hp + 1) k)
                - (i + 1))).flatMap
            (fun i' => opEventsE sk (.kid (hp + 1) k
              (sk.stageScope (hp + 1) k) none
              (sk.wiresBefore (hp + 1) k) i'
              (scopeFeed sk (hp + 1) k)))).filter
            (fun e => evOwner sk e == M) = [] :=
        fun M h1 h2 => kids_filter_neE sk hwf hh hk
          (scopeFeed_length sk (hp + 1) k) hFo hmF (by omega) h2 h1
      generalize hLgen : (List.range' (i + 1)
          (sk.nChildren (hp + 1) (sk.stageScope (hp + 1) k)
            - (i + 1))).flatMap
          (fun i' => opEventsE sk (.kid (hp + 1) k
            (sk.stageScope (hp + 1) k) none
            (sk.wiresBefore (hp + 1) k) i'
            (scopeFeed sk (hp + 1) k))) = L
        at hks2 hks3 hks4 hksU ⊢
      -- the deep windows over the tail
      have hglue := deep_glueE sk hwf (h := hp + 1) hh hk (i := i + 1)
        (by omega) hks4 (fun g' hg' => hlowD g' (by omega))
      -- the tail of the fold, converted
      have hih : EmitOKOnP sk (procsE sk) L rest := by
        have h0 := ihm (i + 1) (by omega)
        rwa [show m = sk.nChildren (hp + 1)
            (sk.stageScope (hp + 1) k) - (i + 1) from by omega,
          hLgen] at h0
      -- foreign-owner silence of the tail
      have hnilU : ∀ G, hp + 1 + 2 ≤ G → G < sk.rootH →
          L.filter (fun e => evOwner sk e == walkIdx sk G) = [] := by
        intro G hG2 hGr
        refine hksU (walkIdx sk G)
          (fun h' hle hc =>
            absurd (walkIdx_inj (by omega) hGr hc) (by omega)) ?_
        rw [hmFeq (by omega)]
        exact fun hc => absurd (walkIdx_inj (by omega) hGr hc)
          (by omega)
      -- the parent chunk is the feed
      have hcq : hp + 1 + 1 < sk.rootH →
          chunkQ sk (hp + 1 + 1) (A (hp + 1 + 1)) (j (hp + 1 + 1))
            = scopeFeed sk (hp + 1) k := by
        intro hGr
        obtain ⟨hA2, hj2⟩ := hanc.rng (hp + 1 + 1) (by omega) hGr
        rw [chunkQ_eq_feed sk hwf (hp := hp + 1) hGr hA2 hj2,
          ← hcoh0 hGr]
      -- the tail telescope: parent cursor at i + 1
      have hparlat : hp + 1 + 1 < sk.rootH →
          (L ++ rest).filter
              (fun e => evOwner sk e == walkIdx sk (hp + 1 + 1))
            = (chunkQ sk (hp + 1 + 1) (A (hp + 1 + 1))
                  (j (hp + 1 + 1))).drop (i + 1)
              ++ (List.range' (j (hp + 1 + 1) + 1)
                    (sk.nChildren (hp + 1 + 1)
                        (sk.stageScope (hp + 1 + 1) (A (hp + 1 + 1)))
                      - (j (hp + 1 + 1) + 1))).flatMap
                   (childChunk sk (wpk (hp + 1 + 1)) (A (hp + 1 + 1)))
              ++ ((upperOut (wpk (hp + 1 + 1)), true,
                    A (hp + 1 + 1)) : Ev)
                :: walkSegE sk (hp + 1 + 1) (A (hp + 1 + 1) + 1)
                    (sk.stageLen (hp + 1 + 1)) := by
        intro hGr
        have hks2' : L.filter
            (fun e => evOwner sk e == walkIdx sk (hp + 1 + 1))
            = (scopeFeed sk (hp + 1) k).drop (i + 1) := by
          rw [← hmFeq hGr]
          exact hks2
        rw [List.filter_append, hks2',
          hanc.fil (hp + 1 + 1) (by omega) hGr, hsat hGr,
          List.nil_append, hcq hGr, ← List.append_assoc]
      have htele_lat : AncTeleE sk (hp + 1) A j
          (fun G => if G = hp + 1 + 1 then i + 1 else t G)
          (L ++ rest) :=
        ancTeleE_rebase sk hanc hnilU hparlat
      -- the pushed subtree telescope
      have hqcnt : (chunkQ sk (hp + 1) k i).length
          = sk.qCount (hp + 1) (sk.stageScope (hp + 1) k) i :=
        chunkQ_length sk (hp + 1) k i
      have htele_sub : AncTeleE sk hp
          (fun G => if G = hp + 1 then k else A G)
          (fun G => if G = hp + 1 then i else j G)
          (fun G => if G = hp + 1
            then sk.qCount (hp + 1) (sk.stageScope (hp + 1) k) i
            else if G = hp + 1 + 1 then i + 1 else t G)
          (L ++ rest) := by
        refine ⟨?_, ?_, ?_, ?_⟩
        · intro G hG hGr
          by_cases hG1 : G = hp + 1
          · subst hG1
            simp only [reduceIte]
            exact ⟨hk, hi⟩
          · simp only [if_neg hG1]
            exact hanc.rng G (by omega) hGr
        · intro G hG2 hGr
          by_cases hG1 : G = hp + 1 + 1
          · subst hG1
            simp only [if_neg (show ¬(hp + 1 + 1 = hp + 1) from
              by omega)]
            obtain ⟨hA2, hj2⟩ := hanc.rng (hp + 1 + 1) (by omega) hGr
            exact parent_slot_isD sk hwf hGr hk hA2 hj2 (hcoh0 hGr)
              (by omega)
          · simp only [if_neg (show ¬(G = hp + 1) from by omega)]
            exact hanc.isD G (by omega) hGr
        · intro G hG1 hGr1
          by_cases hG : G = hp + 1
          · subst hG
            simp only [reduceIte,
              if_neg (show ¬(hp + 1 + 1 = hp + 1) from by omega)]
            exact hcoh0 hGr1
          · simp only [if_neg hG,
              if_neg (show ¬(G + 1 = hp + 1) from by omega)]
            exact hanc.coh G (by omega) hGr1
        · intro G hG hGr
          by_cases hG1 : G = hp + 1
          · subst hG1
            simp only [reduceIte]
            rw [List.filter_append, hks3, hlowO,
              show (chunkQ sk (hp + 1) k i).drop
                  (sk.qCount (hp + 1) (sk.stageScope (hp + 1) k) i)
                  = [] from by rw [← hqcnt]; exact List.drop_length,
              List.nil_append]
          · by_cases hG2 : G = hp + 1 + 1
            · subst hG2
              simp only [if_neg (show ¬(hp + 1 + 1 = hp + 1) from
                by omega), reduceIte]
              have hf := htele_lat.fil (hp + 1 + 1) (by omega) hGr
              simp only [reduceIte] at hf
              exact hf
            · simp only [if_neg hG1, if_neg hG2]
              have hf := htele_lat.fil G (by omega) hGr
              rw [if_neg hG2] at hf
              exact hf
      -- the pushed low windows
      have hlowsub : ∀ g', g' ≤ hp →
          (L ++ rest).filter
              (fun e => evOwner sk e == walkIdx sk g')
            = walkSegE sk g'
                (descIdx sk g' (hp - g')
                  (sk.wiresBefore (hp + 1) k + i + 1))
                (sk.stageLen g') :=
        fun g' hg' => hglue g' (by omega)
      -- the pushed owner-1 clause
      have hfdsub : ∀ (pre : List Ev) (c : Nat),
          pre.filter (fun e => evOwner sk e == walkIdx sk (hp + 1))
            = (scopeFeed sk hp
                (sk.wiresBefore (hp + 1) k + i)).drop c →
          (∀ M, (∀ h', h' ≤ hp → walkIdx sk h' ≠ M) →
            walkIdx sk (hp + 1) ≠ M →
            pre.filter (fun e => evOwner sk e == M) = []) →
          ∃ i₀, (pre ++ (L ++ rest)).filter
              (fun e => evOwner sk e == 1)
            = ((ropenEvents sk).drop 3).drop i₀ := by
        intro pre c hpre hprU
        obtain ⟨i₀, hlr⟩ := hfd L (i + 1) hks2 hksU
        refine ⟨i₀, ?_⟩
        rw [List.filter_append,
          hprU 1 (fun h' _ => by have := hw2 h'; omega)
            (by have := hw2 (hp + 1); omega),
          List.nil_append]
        exact hlr
      -- the subtree, ready
      have hsubOK : EmitOKOnP sk (procsE sk)
          (opEventsE sk (.scope hp (sk.wiresBefore (hp + 1) k + i)
            (scopeFeed sk hp (sk.wiresBefore (hp + 1) k + i))))
          (L ++ rest) := by
        refine IH (sk.wiresBefore (hp + 1) k + i) _ _ _ _ hkid hlowsub
          htele_sub ?_ ?_ hfdsub
        · intro _
          simp only [reduceIte]
        · intro _
          simp only [reduceIte]
          rw [← hqcnt]
          exact List.drop_length
      -- the slot's event list
      have hkidE := opEventsE_kid_eq sk (hp + 1) k none
        (sk.wiresBefore (hp + 1) k) i (scopeFeed sk (hp + 1) k)
      rw [hqel, Option.toList_some,
        show hp + 1 - 1 = hp from rfl, hFeq] at hkidE
      -- shared head facts
      have hOg : ∀ g', g' < hp + 1 → mF < walkIdx sk g' := by
        intro g' hg'
        have := walkIdx_lt sk (show g' < hp + 1 from hg') hh
        omega
      by_cases hD : sk.childIsD (hp + 1) (sk.stageScope (hp + 1) k) i
          = true
      · -- a disputed slot: wire, resolution, query, subtree
        rw [if_pos hD] at hkidE
        simp only [List.cons_append, List.nil_append] at hkidE
        rw [hkidE]
        simp only [List.cons_append]
        refine emitOKOn_cons sk (fun st hW hfix hpred =>
          head_snd_wireE sk hwf hW.toWCountP
            (show 1 ≤ hp + 1 by omega) hpred) ?_
        refine emitOKOn_cons sk ?_ ?_
        · intro st hW hfix _
          rw [List.cons_append] at hW
          refine ready_lowerE sk hwf hm0
            (t := fun G => if G = hp + 1 + 1 then i else t G)
            hh hk hi hD ?_ hcoh0 ?_ ?_ ?_ st hW hfix
          · refine ancTeleE_rebase sk
              (pre := ((lowerOut (wpk (hp + 1)), true,
                    sk.dsBefore (hp + 1) k
                      + dRank sk (wpk (hp + 1)) k i) : Ev)
                :: ((askedOut (wpk (hp + 1 + 1)), true,
                      sk.wiresBefore (hp + 1) k + i) : Ev)
                :: (opEventsE sk (.scope hp
                      (sk.wiresBefore (hp + 1) k + i)
                      (scopeFeed sk hp
                        (sk.wiresBefore (hp + 1) k + i))) ++ L))
              hanc ?_ ?_
            · intro G hG2 hGr
              rw [List.filter_cons_of_neg (by
                  simp only [evOwner_lowerOut, beq_iff_eq]
                  exact fun hc =>
                    absurd (walkIdx_inj hh hGr hc) (by omega)),
                List.filter_cons_of_neg (by
                  simp only [hfQo, beq_iff_eq]
                  rw [hmFeq (by omega)]
                  exact fun hc =>
                    absurd (walkIdx_inj (by omega) hGr hc)
                      (by omega)),
                List.filter_append,
                hscnil (walkIdx sk G)
                  (fun hc =>
                    absurd (walkIdx_inj hh hGr hc) (by omega))
                  (fun h' hle hc =>
                    absurd (walkIdx_inj (by omega) hGr hc)
                      (by omega)),
                List.nil_append,
                hnilU G hG2 hGr]
            · intro hGr
              rw [List.cons_append, List.cons_append,
                List.filter_cons_of_neg (by
                  simp only [evOwner_lowerOut, beq_iff_eq]
                  exact fun hc =>
                    absurd (walkIdx_inj hh (by omega) hc)
                      (by omega)),
                List.filter_cons_of_pos (by
                  simp only [hfQo, hmFeq hGr, beq_self_eq_true]),
                List.append_assoc, List.filter_append,
                hscnil (walkIdx sk (hp + 1 + 1))
                  (fun hc =>
                    absurd (walkIdx_inj hh (by omega) hc)
                      (by omega))
                  (fun h' hle hc =>
                    absurd (walkIdx_inj (by omega) (by omega) hc)
                      (by omega)),
                List.nil_append, hparlat hGr, hcq hGr, hdropi]
              simp only [List.cons_append]
          · have hpm : (((lowerOut (wpk (hp + 1)), true,
                    sk.dsBefore (hp + 1) k
                      + dRank sk (wpk (hp + 1)) k i) : Ev)
                  :: ((askedOut (wpk (hp + 1 + 1)), true,
                        sk.wiresBefore (hp + 1) k + i) : Ev)
                  :: (opEventsE sk (.scope hp
                        (sk.wiresBefore (hp + 1) k + i)
                        (scopeFeed sk hp
                          (sk.wiresBefore (hp + 1) k + i)))
                      ++ L)).filter
                (fun e => evOwner sk e == mF)
                = (scopeFeed sk (hp + 1) k).drop i := by
              rw [List.filter_cons_of_neg (by
                  simp only [evOwner_lowerOut, beq_iff_eq]
                  omega),
                List.filter_cons_of_pos (by
                  simp only [hfQo, beq_self_eq_true]),
                List.filter_append,
                hscnil mF (by omega)
                  (fun h' hle => by
                    have := hOg h' (by omega)
                    omega),
                List.nil_append, hks2, ← hdropi]
            have hpU : ∀ M, (∀ h', h' ≤ hp + 1 →
                  walkIdx sk h' ≠ M) → mF ≠ M →
                (((lowerOut (wpk (hp + 1)), true,
                      sk.dsBefore (hp + 1) k
                        + dRank sk (wpk (hp + 1)) k i) : Ev)
                  :: ((askedOut (wpk (hp + 1 + 1)), true,
                        sk.wiresBefore (hp + 1) k + i) : Ev)
                  :: (opEventsE sk (.scope hp
                        (sk.wiresBefore (hp + 1) k + i)
                        (scopeFeed sk hp
                          (sk.wiresBefore (hp + 1) k + i)))
                      ++ L)).filter
                  (fun e => evOwner sk e == M) = [] := by
              intro M hM1 hM2
              rw [List.filter_cons_of_neg (by
                  simp only [evOwner_lowerOut, beq_iff_eq]
                  exact hM1 (hp + 1) (Nat.le_refl _)),
                List.filter_cons_of_neg (by
                  simp only [hfQo, beq_iff_eq]
                  exact hM2),
                List.filter_append,
                hscnil M (hM1 (hp + 1) (Nat.le_refl _))
                  (fun h' hle => hM1 h' (by omega)),
                List.nil_append, hksU M hM1 hM2]
            obtain ⟨i₀, hf⟩ := hfd _ i hpm hpU
            exact ⟨i₀, hf⟩
          · intro g' hg'
            rw [List.filter_cons_of_neg (by
                simp only [evOwner_lowerOut, beq_iff_eq]
                exact fun hc =>
                  absurd (walkIdx_inj hh (by omega) hc) (by omega)),
              List.filter_cons_of_neg (by
                simp only [hfQo, beq_iff_eq]
                have := hOg g' hg'
                omega),
              List.append_assoc, List.filter_append,
              hsc3 g' (by omega), hglue g' hg']
            refine walkSegE_glue sk
              (descIdx_mono sk g' (hp - g') (by omega)) ?_
            have hle : sk.wiresBefore (hp + 1) k + i + 1
                ≤ sk.stageLen (g' + (hp - g')) := by
              rw [show g' + (hp - g') = hp from by omega]
              omega
            exact descIdx_le_stageLen sk hwf
              (by rw [show g' + (hp - g') = hp from by omega]
                  omega) hle
          · rw [List.filter_cons_of_pos (by
                simp only [evOwner_lowerOut, beq_self_eq_true]),
              List.filter_cons_of_neg (by
                simp only [hfQo, beq_iff_eq]
                omega),
              List.append_assoc, List.filter_append,
              List.filter_append, hsc2, hks3, hlowO, ← hFeq]
            simp only [List.append_assoc]
        · refine emitOKOn_cons sk ?_ ?_
          · intro st hW hfix hpred
            have haq : askedOut (wpk (hp + 1 + 1))
                = Chan.asked (wpk (hp + 1 + 1)).1 hp := by
              unfold askedOut
              rw [if_neg (show ¬((wpk (hp + 1 + 1)).2 < 2) from by
                show ¬(hp + 1 + 1 < 2)
                omega)]
              rfl
            rw [haq] at hpred ⊢
            exact head_snd_askedE sk hwf hW.toWCountP hpred
          · exact emitOKOn_append sk hsubOK hih
      · -- an undisputed slot: wire, query, childless subtree
        have hDf : sk.childIsD (hp + 1) (sk.stageScope (hp + 1) k) i
            = false := by
          cases hDb : sk.childIsD (hp + 1) (sk.stageScope (hp + 1) k) i
          · rfl
          · exact absurd hDb hD
        rw [if_neg hD,
          if_neg (show ¬((hp + 1 == 0) = true) from by simp)]
          at hkidE
        have hn0 : sk.nChildren hp
            (sk.stageScope hp (sk.wiresBefore (hp + 1) k + i)) = 0 :=
          nChildren_kid_notD sk hwf (by omega) hh hk hi hDf
        rw [show opEventsE sk (.scope hp
              (sk.wiresBefore (hp + 1) k + i) [])
            = opEventsE sk (.scope hp (sk.wiresBefore (hp + 1) k + i)
                (scopeFeed sk hp (sk.wiresBefore (hp + 1) k + i)))
          from by rw [scopeFeed_nil sk hn0]] at hkidE
        simp only [List.cons_append, List.nil_append] at hkidE
        rw [hkidE]
        simp only [List.cons_append]
        refine emitOKOn_cons sk (fun st hW hfix hpred =>
          head_snd_wireE sk hwf hW.toWCountP
            (show 1 ≤ hp + 1 by omega) hpred) ?_
        refine emitOKOn_cons sk ?_ ?_
        · intro st hW hfix hpred
          have haq : askedOut (wpk (hp + 1 + 1))
              = Chan.asked (wpk (hp + 1 + 1)).1 hp := by
            unfold askedOut
            rw [if_neg (show ¬((wpk (hp + 1 + 1)).2 < 2) from by
              show ¬(hp + 1 + 1 < 2)
              omega)]
            rfl
          rw [haq] at hpred ⊢
          exact head_snd_askedE sk hwf hW.toWCountP hpred
        · exact emitOKOn_append sk hsubOK hih

-- ============================================== the master induction

/-- THE E MASTER INDUCTION: every emission of an encoder-order scope's
subtree is ready, given the scope's rolling entry context. The site
sequence per scope is per-kid chunks then ONE tail parent site; the
capacity hypothesis (margin 0) replaces `schedulable` throughout. -/
theorem emitOK_scopeE (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) :
    ∀ (h : Nat), h < sk.rootH →
    ∀ (k : Nat) (rest : List Ev) (A j t : Nat → Nat) (mF : Nat),
      k < sk.stageLen h →
      (∀ e ∈ scopeFeed sk h k, evOwner sk e = mF) →
      mF < walkIdx sk h →
      (h + 1 < sk.rootH → mF = walkIdx sk (h + 1)) →
      (∀ g', g' ≤ h →
        rest.filter (fun e => evOwner sk e == walkIdx sk g')
          = walkSegE sk g' (descIdx sk g' (h - g') (k + 1))
              (sk.stageLen g')) →
      AncTeleE sk h A j t rest →
      (h + 1 < sk.rootH →
        k = sk.wiresBefore (h + 1) (A (h + 1)) + j (h + 1)) →
      (h + 1 < sk.rootH →
        (chunkQ sk (h + 1) (A (h + 1)) (j (h + 1))).drop (t (h + 1))
          = []) →
      (∀ (pre : List Ev) (c : Nat),
        pre.filter (fun e => evOwner sk e == mF)
          = (scopeFeed sk h k).drop c →
        (∀ M, (∀ h', h' ≤ h → walkIdx sk h' ≠ M) → mF ≠ M →
          pre.filter (fun e => evOwner sk e == M) = []) →
        ∃ i₀, (pre ++ rest).filter (fun e => evOwner sk e == 1)
          = ((ropenEvents sk).drop 3).drop i₀) →
      EmitOKOnE sk (opEventsE sk (.scope h k (scopeFeed sk h k)))
        rest := by
  intro h
  induction h with
  | zero =>
      intro hh k rest A j t mF hk hFo hmF hmFeq hlow hanc hcoh0 hsat
        hfd
      have hr2 : 1 < sk.rootH := by have := (wf_rootH hwf).2; omega
      refine emitOK_scope_zeroE sk hwf hm0 hk hlow hanc (hcoh0 hr2)
        (hsat hr2) ?_
      have h0 := hfd [] (scopeFeed sk 0 k).length
        (by simp [List.drop_length]) (fun M _ _ => rfl)
      simpa using h0
  | succ hp ih =>
      intro hh k rest A j t mF hk hFo hmF hmFeq hlow hanc hcoh0 hsat
        hfd
      have hE := opEventsE_scope_eq sk (Nat.le_of_lt hk)
        (scopeFeed sk (hp + 1) k)
      rw [List.range_eq_range'] at hE
      rw [hE]
      refine emitOKOn_cons sk (fun st hW hfix hpred =>
        head_rcv_wireE sk hwf hW.toWCountP hpred) ?_
      refine emitOKOn_cons sk (fun st hW hfix hpred =>
        head_rcv_askedE sk hwf hW.toWCountP hpred) ?_
      have hlow1 : rest.filter
            (fun e => evOwner sk e == walkIdx sk (hp + 1))
          = walkSegE sk (hp + 1) (k + 1) (sk.stageLen (hp + 1)) := by
        have hl := hlow (hp + 1) (Nat.le_refl _)
        rw [Nat.sub_self, descIdx_zero] at hl
        exact hl
      have hw2 : ∀ h', 2 ≤ walkIdx sk h' := by
        intro h'
        unfold walkIdx
        omega
      have hlowO : ((((upperOut (wpk (hp + 1)), true, k) : Ev))
            :: rest).filter
          (fun e => evOwner sk e == walkIdx sk (hp + 1))
          = ((upperOut (wpk (hp + 1)), true, k) : Ev)
            :: walkSegE sk (hp + 1) (k + 1) (sk.stageLen (hp + 1)) := by
        rw [List.filter_cons_of_pos (by
            simp only [evOwner_upperOut, beq_self_eq_true]), hlow1]
      have hlowD' : ∀ g', g' ≤ hp →
          ((((upperOut (wpk (hp + 1)), true, k) : Ev)) :: rest).filter
            (fun e => evOwner sk e == walkIdx sk g')
          = walkSegE sk g' (descIdx sk g' (hp + 1 - g') (k + 1))
              (sk.stageLen g') := by
        intro g' hg'
        rw [List.filter_cons_of_neg (by
            simp only [evOwner_upperOut, beq_iff_eq]
            intro hc
            exact absurd (walkIdx_inj hh (by omega) hc) (by omega)),
          hlow g' (by omega)]
      have hancU : AncTeleE sk (hp + 1) A j t
          ((((upperOut (wpk (hp + 1)), true, k) : Ev)) :: rest) := by
        refine ⟨hanc.rng, hanc.isD, hanc.coh, ?_⟩
        intro G hG hGr
        rw [List.filter_cons_of_neg (by
            simp only [evOwner_upperOut, beq_iff_eq]
            intro hc
            exact absurd (walkIdx_inj hh hGr hc) (by omega))]
        exact hanc.fil G hG hGr
      have hfdU : ∀ (pre : List Ev) (c : Nat),
          pre.filter (fun e => evOwner sk e == mF)
            = (scopeFeed sk (hp + 1) k).drop c →
          (∀ M, (∀ h', h' ≤ hp + 1 → walkIdx sk h' ≠ M) → mF ≠ M →
            pre.filter (fun e => evOwner sk e == M) = []) →
          ∃ i₀, (pre ++ ((((upperOut (wpk (hp + 1)), true, k) : Ev))
              :: rest)).filter (fun e => evOwner sk e == 1)
            = ((ropenEvents sk).drop 3).drop i₀ := by
        intro pre c hpre hprU
        obtain ⟨i₀, hf⟩ := hfd
          (pre ++ [((upperOut (wpk (hp + 1)), true, k) : Ev)]) c
          (by
            rw [List.filter_append, hpre,
              List.filter_cons_of_neg (by
                simp only [evOwner_upperOut, beq_iff_eq]
                omega),
              List.filter_nil, List.append_nil])
          (by
            intro M hM1 hM2
            rw [List.filter_append, hprU M hM1 hM2, List.nil_append,
              List.filter_cons_of_neg (by
                simp only [evOwner_upperOut, beq_iff_eq]
                exact hM1 (hp + 1) (Nat.le_refl _)),
              List.filter_nil])
        refine ⟨i₀, ?_⟩
        rw [List.append_assoc] at hf
        exact hf
      refine emitOKOn_append sk ?_ ?_
      · -- the kid slots against the pending parent
        refine emitOK_kidsE sk hwf hm0 hh hk hFo hmF hmFeq hlowD'
          hlowO hancU hcoh0 hsat hfdU ?_
          (sk.nChildren (hp + 1) (sk.stageScope (hp + 1) k)) 0
          (by omega)
        intro k' rest' A' j' t' hk' hlow' hanc' hcoh0' hsat' hfd'
        refine ih (by omega) k' rest' A' j' t' (walkIdx sk (hp + 1))
          hk' ?_ (walkIdx_lt sk (by omega) hh) (fun _ => rfl) hlow'
          hanc' hcoh0' hsat' hfd'
        intro e he
        unfold scopeFeed seg at he
        obtain ⟨j'', -, rfl⟩ := List.mem_map.1 he
        exact evOwner_askedOut sk (by omega) hh _
      · -- the tail parent through the upper window
        refine emitOKOn_cons sk ?_ (emitOKOn_nil sk _)
        intro st hW hfix _
        rw [List.nil_append] at hW
        have hdeepU : ∀ g', g' < hp + 1 →
            ((((upperOut (wpk (hp + 1)), true, k) : Ev)) :: rest).filter
              (fun e => evOwner sk e == walkIdx sk g')
            = walkSegE sk g'
                (descIdx sk g' (hp + 1 - 1 - g')
                  (sk.wiresBefore (hp + 1) (k + 1)))
                (sk.stageLen g') := by
          intro g' hg'
          have hd := hlowD' g' (by omega)
          rw [show hp + 1 - g' = (hp + 1 - 1 - g') + 1 from by omega,
            descIdx_succ,
            show g' + (hp + 1 - 1 - g') + 1 = hp + 1 from by omega]
            at hd
          exact hd
        refine ready_upperE sk hwf hm0 hh hk hancU hcoh0 ?_ hdeepU
          hlowO st hW hfix
        obtain ⟨i₀, hf⟩ := hfd [] (scopeFeed sk (hp + 1) k).length
          (by simp [List.drop_length]) (fun M _ _ => rfl)
        refine ⟨i₀, ?_⟩
        rw [List.filter_cons_of_neg (by
            simp only [evOwner_upperOut, beq_iff_eq]
            have := hw2 (hp + 1)
            omega)]
        simpa using hf

-- ================================================= the top assembly

/-- The full opening E future is pointwise-ready: the openers by
their seq-zero windows and in-flight predecessors, the root scope by
the E master induction with the trivial entry context. -/
theorem emitOK_weaveE (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) :
    EmitOKOnE sk ((weaveOps sk).flatMap (opEventsE sk)) [] := by
  have hge := (wf_rootH hwf).2
  rw [weave_flatMapE]
  refine emitOKOn_append sk ?_ ?_
  · -- the five openers
    show EmitOKOnP sk (procsE sk)
      [(Chan.wire Party.I sk.rootH, true, 0),
       (Chan.asked Party.I (sk.rootH - 1), true, 0),
       (Chan.wire Party.I sk.rootH, false, 0),
       (Chan.wire Party.R sk.rootH, true, 0),
       (Chan.rootres, true, 0)] _
    refine emitOKOn_cons sk
      (fun st _ _ _ => enabled_snd_low sk (cap_pos hwf _)) ?_
    refine emitOKOn_cons sk
      (fun st _ _ _ => enabled_snd_low sk (cap_pos hwf _)) ?_
    refine emitOKOn_cons sk
      (fun st hW hfix hpred =>
        head_rcv_wireE sk hwf hW.toWCountP hpred) ?_
    refine emitOKOn_cons sk
      (fun st _ _ _ => enabled_snd_low sk (cap_pos hwf _)) ?_
    exact emitOKOn_cons sk
      (fun st _ _ _ => enabled_snd_low sk (cap_pos hwf _))
      (emitOKOn_nil sk _)
  · -- the root scope, entered with the trivial context
    rw [ropen_drop_eq_feed sk hwf]
    refine emitOK_scopeE sk hwf hm0 (sk.rootH - 1) (by omega) 0 []
      (fun _ => 0) (fun _ => 0) (fun _ => 0) 1
      (by rw [wf_stageLen_top sk hwf]; omega) ?_ ?_ ?_ ?_ ?_ ?_ ?_ ?_
    · intro e he
      rw [← ropen_drop_eq_feed sk hwf] at he
      exact ropen_owner sk hwf e (List.mem_of_mem_drop he)
    · unfold walkIdx
      omega
    · intro hcon
      exact absurd hcon (by omega)
    · intro g' hg'
      simp only [Nat.zero_add]
      have hdt : descIdx sk g' (sk.rootH - 1 - g') 1
          = sk.stageLen g' := by
        have h2 := descIdx_total sk hwf (sk.rootH - 1 - g') g'
          (by omega)
        rwa [show g' + (sk.rootH - 1 - g') = sk.rootH - 1 from
          by omega, wf_stageLen_top sk hwf] at h2
      rw [List.filter_nil, hdt, walkSegE_empty]
    · exact ⟨fun G h1 h2 => absurd h2 (by omega),
        fun G h1 h2 => absurd h2 (by omega),
        fun G h1 h2 => absurd h2 (by omega),
        fun G h1 h2 => absurd h2 (by omega)⟩
    · intro hcon
      exact absurd hcon (by omega)
    · intro hcon
      exact absurd hcon (by omega)
    · intro pre c hpre _
      refine ⟨c, ?_⟩
      rw [List.append_nil, hpre, ← ropen_drop_eq_feed sk hwf]

/-- The eweave respects every edge GIVEN pointwise readiness (cf.
`weaveState_wedge_of_emitOK`). -/
theorem weaveStateE_wedge_of_emitOK (hwf : sk.wellFormed = true)
    (hemit : EmitOKOnE sk ((weaveOps sk).flatMap (opEventsE sk)) []) :
    WEdgeP sk (procsE sk) [] (weaveStateE sk) := by
  obtain ⟨hown, halign⟩ := weaveE_initial_alignment sk hwf
  have hgo : goEventsE sk (weaveFuel sk) (weaveOps sk)
      = (weaveOps sk).flatMap (opEventsE sk) :=
    goEventsE_weave sk (weave_events_lengthE sk hwf)
  have hinit : WEdgeP sk (procsE sk)
      (goEventsE sk (weaveFuel sk) (weaveOps sk)) (weaveInit sk) :=
    wEdge_initP sk (by rw [hgo]; exact halign) (procsE_drop_pumps sk)
      (by rw [hgo]; exact hown)
  have hdep : DepOK [] (goEventsE sk (weaveFuel sk) (weaveOps sk)) :=
    weaveE_goEvents_depOK sk hwf
  obtain ⟨f, hfuel⟩ : ∃ f, weaveFuel sk = f + 1 :=
    ⟨4 * totalEvents sk + 7, by unfold weaveFuel; omega⟩
  obtain ⟨e₁, opsTail, hops, he₁⟩ :
      ∃ (e₁ : Ev) (opsTail : List WOp),
        weaveOps sk = .emit e₁ :: opsTail
          ∧ e₁ = ((Chan.wire Party.I sk.rootH, true, 0) : Ev) :=
    ⟨_, _, rfl, rfl⟩
  have hgo1 : goEventsE sk (weaveFuel sk) (weaveOps sk)
      = e₁ :: goEventsE sk f opsTail := by
    rw [hfuel, hops]
    rfl
  have hen : enabled sk (weaveInit sk).sent (weaveInit sk).rcvd e₁
      = true := by
    rw [he₁]
    exact enabled_snd_low sk (cap_pos hwf _)
  have hW1 : WEdgeP sk (procsE sk) (e₁ :: goEventsE sk f opsTail)
      (weaveInit sk) := by
    rw [← hgo1]
    exact hinit
  show WEdgeP sk (procsE sk) []
    (wPump sk (weaveGoE sk (weaveFuel sk) (weaveOps sk)
      (weaveInit sk)))
  have hstep1 : weaveGoE sk (weaveFuel sk) (weaveOps sk)
        (weaveInit sk)
      = weaveGoE sk f opsTail (wEmitP sk (weaveInit sk) e₁) := by
    rw [hfuel, hops]
    rfl
  rw [hstep1]
  refine wEdge_pump sk ?_
  refine weaveGoE_wedge sk f opsTail _ [e₁]
    (wEdge_emitP sk hen hW1) ?_ ?_ ?_ (wPump_fixpoint sk _)
  · have hd1 : DepOK [] (e₁ :: goEventsE sk f opsTail) := by
      rw [← hgo1]
      exact hdep
    simpa using depOK_tail hd1
  · intro x hx
    have hxe : x = e₁ := List.mem_singleton.1 hx
    refine mem_out_wEmitP sk ?_
    rw [hxe]
    exact List.mem_append_right _ (List.mem_cons_self ..)
  · refine emitOKOn_tail sk (e := e₁) ?_
    rw [← hgo1, hgo]
    exact hemit

/-- UNIT 2b, CLOSED: the eweave's final state respects every edge at
the encoder-order family under margin 0 — the `.impl` completeness
witness is edge-respecting. -/
theorem weaveE_wedge (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) :
    WEdgeP sk (procsE sk) [] (weaveStateE sk) :=
  weaveStateE_wedge_of_emitOK sk hwf (emitOK_weaveE sk hwf hm0)

end StreamingMirror.Sched
