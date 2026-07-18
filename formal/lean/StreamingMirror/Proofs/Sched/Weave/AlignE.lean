/-
The encoder-order initial alignment (PROGRESS.md §9, unit 2a-align):
`align_scope`'s master induction transcribed over the E expanders, and
its top assembly against `procsE`.

One delta drives every difference from `Align.lean`: the parent
summary sits at the scope TAIL (`wScopeOpsE`), not spliced after the
final resolution — so the own-walk filter of a scope's expansion is
`scopeBlockE` (per-kid `childChunk`s, parent last) instead of
`scopeBlock`'s spliced form, and the upper-splice case splits of the
d5 proof vanish. The feeder and descendant arms are positionally
identical between the two orders (the parent is each scope's sole
own-owner moved event), so those clauses transcribe verbatim modulo
the tail-parent drop.

The payoff at the bottom of the file: `weaveE_wcount` — the eweave's
final state carries the counting invariant at the `procsE` family —
which is the E consumption frame's entry fact (`MasterE.lean`).
-/
import StreamingMirror.Proofs.Sched.Weave.Align
import StreamingMirror.Proofs.Sched.Weave.ExpandE

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ================================================ statement vocabulary

/-- A contiguous run of stage-`h'` encoder-order scope blocks: trace
segment `[a, b)` of the stage's walk, epilogue order. -/
def walkSegE (h' a b : Nat) : List Ev :=
  (List.range' a (b - a)).flatMap (scopeBlockE sk (wpk h'))

theorem walkSegE_empty (h' a : Nat) : walkSegE sk h' a a = by
    exact [] := by
  unfold walkSegE
  rw [Nat.sub_self]
  rfl

theorem walkSegE_single (h' k : Nat) :
    walkSegE sk h' k (k + 1) = scopeBlockE sk (wpk h') k := by
  unfold walkSegE
  rw [Nat.add_sub_cancel_left, List.range'_one, List.flatMap_cons,
    List.flatMap_nil, List.append_nil]

/-- Abutting encoder-order stage runs glue into one. -/
theorem walkSegE_glue {h' a b c : Nat} (hab : a ≤ b) (hbc : b ≤ c) :
    walkSegE sk h' a b ++ walkSegE sk h' b c = walkSegE sk h' a c := by
  unfold walkSegE
  rw [← List.flatMap_append,
    show c - a = (b - a) + (c - b) from by omega,
    ← List.range'_append,
    show a + 1 * (b - a) = b from by omega]

theorem walkSegE_glue_range (h' : Nat) (g : Nat → Nat)
    (hmono : ∀ i, g i ≤ g (i + 1)) :
    ∀ n, (List.range n).flatMap (fun i => walkSegE sk h' (g i) (g (i + 1)))
      = walkSegE sk h' (g 0) (g n) := by
  intro n
  induction n with
  | zero => rw [List.range_zero, List.flatMap_nil, walkSegE_empty]
  | succ n ih =>
      have h0n : g 0 ≤ g n := by
        clear ih
        induction n with
        | zero => exact Nat.le_refl _
        | succ m ihm => exact Nat.le_trans ihm (hmono m)
      rw [List.range_succ, List.flatMap_append, ih, List.flatMap_cons,
        List.flatMap_nil, List.append_nil, walkSegE_glue sk h0n (hmono n)]

/-- A whole-stage encoder-order run is the stage's E walk trace. -/
theorem walkSegE_full (h' : Nat) :
    walkSegE sk h' 0 (sk.stageLen h') = walkEventsE sk (wpk h') := by
  unfold walkSegE walkEventsE
  rw [Nat.sub_zero, ← List.range_eq_range']
  rfl

/-- `scopeSendsE`, resolved to a per-kid flatMap with the parent as
the tail: the E own-walk assembly target. -/
theorem scopeSendsE_eq (h k : Nat) :
    scopeSendsE sk (wpk h) k
      = (List.range (sk.nChildren h (sk.stageScope h k))).flatMap
          (childChunk sk (wpk h) k)
        ++ [((upperOut (wpk h), true, k) : Ev)] := by
  simp only [scopeSendsE, List.flatMap_def]
  rfl

-- ==================================================== the E expansions

/-- An E scope op's expansion, events flattened: prologue receives,
the kid ops in slot order, the parent summary last. -/
theorem opEventsE_scope_eq {h k : Nat} (hk : k ≤ sk.stageLen h)
    (feed : List Ev) :
    opEventsE sk (.scope h k feed)
      = (wireIn (wpk h), false, k) :: (askedIn (wpk h), false, k)
          :: ((List.range (sk.nChildren h (sk.stageScope h k))).flatMap
                (fun i => opEventsE sk (.kid h k (sk.stageScope h k)
                  none (sk.wiresBefore h k) i feed))
            ++ [((upperOut (wpk h), true, k) : Ev)]) := by
  rw [opEventsE_scope]
  simp only [wScopeOpsE]
  rw [kidBase_eq_wiresBefore sk h k hk]
  simp only [wpk]
  simp [opEventsE_emit, List.flatMap_map, List.flatMap_append]

/-- An E kid op's expansion, events flattened: the trace chunk with
the kid's feed query and subtree in place — never a parent. -/
theorem opEventsE_kid_eq (h k : Nat) (lastD : Option Nat)
    (kidBase i : Nat) (feed : List Ev) :
    opEventsE sk (.kid h k (sk.stageScope h k) lastD kidBase i feed)
      = (wireOut (wpk h), true, sk.wiresBefore h k + i)
          :: (if sk.childIsD h (sk.stageScope h k) i then
                (lowerOut (wpk h), true,
                    sk.dsBefore h k + dRank sk (wpk h) k i)
                  :: (feed[i]?.toList
                    ++ opEventsE sk
                        (.scope (h - 1) (kidBase + i) (chunkQ sk h k i)))
              else feed[i]?.toList
                ++ (if h == 0 then []
                    else opEventsE sk (.scope (h - 1) (kidBase + i) []))) := by
  rw [opEventsE_kid]
  simp only [wKidOpsE, wpk]
  cases hfi : feed[i]? <;>
    by_cases hD : sk.childIsD h (sk.stageScope h k) i <;>
    by_cases h0 : (h == 0) <;>
    simp [hD, h0, opEventsE_emit, dRank, qSum, chunkQ, wpk]

-- ================================================ the master induction

/-- The E subtree alignment: `align_scope`'s three clauses over the
encoder-order expansion. Clause (1)'s own-stage form is `walkSegE` —
the epilogue-order scope blocks — and the descent runs are E segments;
the ownership and feeder clauses match the d5 statement shape. -/
theorem align_scopeE (hwf : sk.wellFormed = true) :
    ∀ (h k : Nat) (F : List Ev) (mF : Nat),
      h < sk.rootH → k < sk.stageLen h →
      F.length = sk.nChildren h (sk.stageScope h k) →
      (∀ e ∈ F, evOwner sk e = mF) →
      mF < walkIdx sk h →
      ((∀ e ∈ opEventsE sk (.scope h k F),
          evOwner sk e = mF
            ∨ ∃ h', h' ≤ h ∧ evOwner sk e = walkIdx sk h')
        ∧ (opEventsE sk (.scope h k F)).filter
            (fun e => evOwner sk e == mF) = F
        ∧ ∀ h' ≤ h,
            (opEventsE sk (.scope h k F)).filter
                (fun e => evOwner sk e == walkIdx sk h')
              = walkSegE sk h' (descIdx sk h' (h - h') k)
                  (descIdx sk h' (h - h') (k + 1))) := by
  intro h
  induction h with
  | zero =>
      intro k F mF hh hk hF hFo hmF
      have hD0 : ∀ i, sk.childIsD 0 (sk.stageScope 0 k) i = false :=
        fun _ => rfl
      have hE := opEventsE_scope_eq sk (Nat.le_of_lt hk) F
      have hkidE : ∀ i,
          opEventsE sk (.kid 0 k (sk.stageScope 0 k) none
            (sk.wiresBefore 0 k) i F)
          = (wireOut (wpk 0), true, sk.wiresBefore 0 k + i)
              :: F[i]?.toList := by
        intro i
        rw [opEventsE_kid_eq,
          if_neg (by rw [hD0 i]; exact Bool.false_ne_true),
          if_pos (show ((0 : Nat) == 0) = true by rfl), List.append_nil]
      refine ⟨?_, ?_, ?_⟩
      · -- (3) ownership: everything is the feeder's or the leaf walk's
        intro e he
        rw [hE] at he
        rcases he with _ | ⟨_, he⟩
        · exact Or.inr ⟨0, Nat.le_refl 0, evOwner_wireIn sk hwf 0 k⟩
        rcases he with _ | ⟨_, he⟩
        · exact Or.inr ⟨0, Nat.le_refl 0, evOwner_askedIn sk k⟩
        rcases List.mem_append.1 he with he | he
        · obtain ⟨i, -, hei⟩ := List.mem_flatMap.1 he
          rw [hkidE i] at hei
          rcases hei with _ | ⟨_, hei⟩
          · exact Or.inr ⟨0, Nat.le_refl 0, evOwner_wireOut sk hh _⟩
          · exact Or.inl (hFo e
              (List.mem_of_getElem? (Option.mem_toList.1 hei)))
        · rcases he with _ | ⟨_, he⟩
          · exact Or.inr ⟨0, Nat.le_refl 0, evOwner_upperOut sk k⟩
          · cases he
      · -- (2) the feeder's filter is the feed
        rw [hE,
          List.filter_cons_of_neg (by
            simp only [evOwner_wireIn sk hwf, beq_iff_eq]; omega),
          List.filter_cons_of_neg (by
            simp only [evOwner_askedIn, beq_iff_eq]; omega),
          List.filter_append,
          List.filter_cons_of_neg (by
            simp only [evOwner_upperOut, beq_iff_eq]; omega),
          List.filter_nil, List.append_nil]
        have hkMF : ∀ i ∈ List.range (sk.nChildren 0 (sk.stageScope 0 k)),
            (opEventsE sk (.kid 0 k (sk.stageScope 0 k) none
                (sk.wiresBefore 0 k) i F)).filter
              (fun e => evOwner sk e == mF) = F[i]?.toList := by
          intro i _
          rw [hkidE i,
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
        simp only [List.filter_flatMap]
        rw [flatMap_congr hkMF, ← hF]
        exact flatMap_getElem?_toList F
      · -- (1) the leaf walk's filter is the E scope block
        intro h' hle
        have h0 : h' = 0 := Nat.le_zero.mp hle
        subst h0
        rw [Nat.sub_self, descIdx_zero, descIdx_zero, walkSegE_single]
        have hkOwn : ∀ i ∈ List.range (sk.nChildren 0 (sk.stageScope 0 k)),
            (opEventsE sk (.kid 0 k (sk.stageScope 0 k) none
                (sk.wiresBefore 0 k) i F)).filter
              (fun e => evOwner sk e == walkIdx sk 0)
            = childChunk sk (wpk 0) k i := by
          intro i _
          rw [hkidE i,
            List.filter_cons_of_pos (by
              simp only [evOwner_wireOut sk hh, beq_self_eq_true]),
            childChunk_eq,
            if_neg (by rw [hD0 i]; exact Bool.false_ne_true)]
          congr 1
          cases hfi : F[i]? with
          | none => rfl
          | some q =>
              rw [Option.toList_some,
                List.filter_cons_of_neg (by
                  simp only [hFo q (List.mem_of_getElem? hfi), beq_iff_eq]
                  omega),
                List.filter_nil]
        rw [hE,
          List.filter_cons_of_pos (by
            simp only [evOwner_wireIn sk hwf, beq_self_eq_true]),
          List.filter_cons_of_pos (by
            simp only [evOwner_askedIn, beq_self_eq_true]),
          List.filter_append,
          List.filter_cons_of_pos (by
            simp only [evOwner_upperOut, beq_self_eq_true]),
          List.filter_nil]
        simp only [List.filter_flatMap]
        rw [flatMap_congr hkOwn, scopeBlockE, scopeSendsE_eq]
  | succ h ih =>
      intro k F mF hh hk hF hFo hmF
      have hh' : h < sk.rootH := by omega
      have h1 : (1 : Nat) ≤ h + 1 := by omega
      have hsub : ∀ i, i < sk.nChildren (h + 1) (sk.stageScope (h + 1) k) →
          sk.wiresBefore (h + 1) k + i < sk.stageLen h := by
        intro i hi
        have htot := wiresBefore_total sk hwf h1 hh
        simp only [Nat.add_sub_cancel] at htot
        have hmono := wiresBefore_mono sk (h + 1)
          (show k + 1 ≤ sk.stageLen (h + 1) from hk)
        have hstep := wiresBefore_succ sk hk
        omega
      have hmF' : walkIdx sk (h + 1) < walkIdx sk h :=
        walkIdx_lt sk (Nat.lt_succ_self h) hh
      have hE := opEventsE_scope_eq sk (Nat.le_of_lt hk) F
      have hkidE : ∀ i,
          opEventsE sk (.kid (h + 1) k (sk.stageScope (h + 1) k)
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
        intro i
        rw [opEventsE_kid_eq]
        simp only [Nat.add_sub_cancel,
          show ((h + 1 : Nat) == 0) = false from rfl, Bool.false_eq_true,
          if_false]
      -- the induction hypothesis, instantiated per kid
      have hIHsub := fun (i : Nat)
          (hi : i < sk.nChildren (h + 1) (sk.stageScope (h + 1) k)) =>
        ih (sk.wiresBefore (h + 1) k + i) (chunkQ sk (h + 1) k i)
          (walkIdx sk (h + 1)) hh' (hsub i hi)
          (by
            have hq := qCount_eq_kid_nChildren sk hwf h1 hh hk hi
            simp only [Nat.add_sub_cancel] at hq
            rw [chunkQ_length, hq])
          (chunkQ_owner sk h1 hh k i) hmF'
      have hIHW := fun (i : Nat)
          (hi : i < sk.nChildren (h + 1) (sk.stageScope (h + 1) k))
          (hDf : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i = false) =>
        ih (sk.wiresBefore (h + 1) k + i) [] (walkIdx sk (h + 1)) hh'
          (hsub i hi)
          (by
            have hz := nChildren_kid_notD sk hwf h1 hh hk hi hDf
            simp only [Nat.add_sub_cancel] at hz
            rw [List.length_nil, hz])
          (fun e he => absurd he (by simp)) hmF'
      -- (A) each kid's own-stage filter is its trace chunk
      have hkidOwn : ∀ i ∈ List.range
            (sk.nChildren (h + 1) (sk.stageScope (h + 1) k)),
          (opEventsE sk (.kid (h + 1) k (sk.stageScope (h + 1) k)
              none (sk.wiresBefore (h + 1) k) i F)).filter
            (fun e => evOwner sk e == walkIdx sk (h + 1))
          = childChunk sk (wpk (h + 1)) k i := by
        intro i hi
        rw [List.mem_range] at hi
        have hFeed : (F[i]?.toList).filter
            (fun e => evOwner sk e == walkIdx sk (h + 1)) = [] := by
          cases hfi : F[i]? with
          | none => rfl
          | some q =>
              rw [Option.toList_some,
                List.filter_cons_of_neg (by
                  simp only [hFo q (List.mem_of_getElem? hfi), beq_iff_eq]
                  omega),
                List.filter_nil]
        rw [hkidE i, childChunk_eq]
        by_cases hD : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
        · rw [if_pos hD, if_pos hD,
            List.filter_cons_of_pos (by
              simp only [evOwner_wireOut sk hh, beq_self_eq_true]),
            List.filter_cons_of_pos (by
              simp only [evOwner_lowerOut, beq_self_eq_true]),
            List.filter_append, hFeed, List.nil_append,
            (hIHsub i hi).2.1]
        · have hDf : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
              = false := by simpa using hD
          rw [if_neg hD, if_neg hD,
            List.filter_cons_of_pos (by
              simp only [evOwner_wireOut sk hh, beq_self_eq_true]),
            List.filter_append, hFeed, List.nil_append,
            (hIHW i hi hDf).2.1]
      -- (B) each kid's feeder filter is its feed query
      have hkidMF : ∀ i ∈ List.range
            (sk.nChildren (h + 1) (sk.stageScope (h + 1) k)),
          (opEventsE sk (.kid (h + 1) k (sk.stageScope (h + 1) k)
              none (sk.wiresBefore (h + 1) k) i F)).filter
            (fun e => evOwner sk e == mF) = F[i]?.toList := by
        intro i hi
        rw [List.mem_range] at hi
        have hFeedKeep : (F[i]?.toList).filter
            (fun e => evOwner sk e == mF) = F[i]?.toList := by
          cases hfi : F[i]? with
          | none => rfl
          | some q =>
              rw [Option.toList_some,
                List.filter_cons_of_pos (by
                  simp only [hFo q (List.mem_of_getElem? hfi),
                    beq_self_eq_true]),
                List.filter_nil]
        rw [hkidE i]
        by_cases hD : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
        · have hSubDrop : (opEventsE sk (.scope h
                (sk.wiresBefore (h + 1) k + i)
                (chunkQ sk (h + 1) k i))).filter
              (fun e => evOwner sk e == mF) = [] := by
            rw [List.filter_eq_nil_iff]
            intro e he
            rcases (hIHsub i hi).1 e he with ho | ⟨h'', hle'', ho⟩
            · simp only [ho, beq_iff_eq]
              omega
            · have hwlt := walkIdx_lt sk (show h'' < h + 1 from by omega) hh
              simp only [ho, beq_iff_eq]
              omega
          rw [if_pos hD,
            List.filter_cons_of_neg (by
              simp only [evOwner_wireOut sk hh, beq_iff_eq]; omega),
            List.filter_cons_of_neg (by
              simp only [evOwner_lowerOut, beq_iff_eq]; omega),
            List.filter_append, hFeedKeep, hSubDrop, List.append_nil]
        · have hDf : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
              = false := by simpa using hD
          have hSubDrop : (opEventsE sk (.scope h
                (sk.wiresBefore (h + 1) k + i) [])).filter
              (fun e => evOwner sk e == mF) = [] := by
            rw [List.filter_eq_nil_iff]
            intro e he
            rcases (hIHW i hi hDf).1 e he with ho | ⟨h'', hle'', ho⟩
            · simp only [ho, beq_iff_eq]
              omega
            · have hwlt := walkIdx_lt sk (show h'' < h + 1 from by omega) hh
              simp only [ho, beq_iff_eq]
              omega
          rw [if_neg hD,
            List.filter_cons_of_neg (by
              simp only [evOwner_wireOut sk hh, beq_iff_eq]; omega),
            List.filter_append, hFeedKeep, hSubDrop, List.append_nil]
      -- (C) each kid's descendant-stage filter is its subtree's E run
      have hkidDesc : ∀ h', h' ≤ h → ∀ i ∈ List.range
            (sk.nChildren (h + 1) (sk.stageScope (h + 1) k)),
          (opEventsE sk (.kid (h + 1) k (sk.stageScope (h + 1) k)
              none (sk.wiresBefore (h + 1) k) i F)).filter
            (fun e => evOwner sk e == walkIdx sk h')
          = walkSegE sk h'
              (descIdx sk h' (h - h') (sk.wiresBefore (h + 1) k + i))
              (descIdx sk h' (h - h')
                (sk.wiresBefore (h + 1) k + (i + 1))) := by
        intro h' hle i hi
        rw [List.mem_range] at hi
        have hwlt : walkIdx sk (h + 1) < walkIdx sk h' :=
          walkIdx_lt sk (by omega) hh
        have hFeedDrop : (F[i]?.toList).filter
            (fun e => evOwner sk e == walkIdx sk h') = [] := by
          cases hfi : F[i]? with
          | none => rfl
          | some q =>
              rw [Option.toList_some,
                List.filter_cons_of_neg (by
                  simp only [hFo q (List.mem_of_getElem? hfi), beq_iff_eq]
                  omega),
                List.filter_nil]
        rw [hkidE i]
        by_cases hD : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
        · rw [if_pos hD,
            List.filter_cons_of_neg (by
              simp only [evOwner_wireOut sk hh, beq_iff_eq]; omega),
            List.filter_cons_of_neg (by
              simp only [evOwner_lowerOut, beq_iff_eq]; omega),
            List.filter_append, hFeedDrop, List.nil_append,
            ((hIHsub i hi).2.2) h' hle, Nat.add_assoc]
        · have hDf : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
              = false := by simpa using hD
          rw [if_neg hD,
            List.filter_cons_of_neg (by
              simp only [evOwner_wireOut sk hh, beq_iff_eq]; omega),
            List.filter_append, hFeedDrop, List.nil_append,
            ((hIHW i hi hDf).2.2) h' hle, Nat.add_assoc]
      refine ⟨?_, ?_, ?_⟩
      · -- (3) ownership
        intro e he
        rw [hE] at he
        rcases he with _ | ⟨_, he⟩
        · exact Or.inr ⟨h + 1, Nat.le_refl _, evOwner_wireIn sk hwf (h + 1) k⟩
        rcases he with _ | ⟨_, he⟩
        · exact Or.inr ⟨h + 1, Nat.le_refl _, evOwner_askedIn sk k⟩
        rcases List.mem_append.1 he with he | he
        · obtain ⟨i, hi, hei⟩ := List.mem_flatMap.1 he
          rw [List.mem_range] at hi
          rw [hkidE i] at hei
          rcases hei with _ | ⟨_, hei⟩
          · exact Or.inr ⟨h + 1, Nat.le_refl _, evOwner_wireOut sk hh _⟩
          by_cases hD : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
          · rw [if_pos hD] at hei
            rcases hei with _ | ⟨_, hei⟩
            · exact Or.inr ⟨h + 1, Nat.le_refl _, evOwner_lowerOut sk _⟩
            rcases List.mem_append.1 hei with hei | hei
            · exact Or.inl (hFo e
                (List.mem_of_getElem? (Option.mem_toList.1 hei)))
            · rcases (hIHsub i hi).1 e hei with ho | ⟨h'', hle'', ho⟩
              · exact Or.inr ⟨h + 1, Nat.le_refl _, ho⟩
              · exact Or.inr ⟨h'', by omega, ho⟩
          · rw [if_neg hD] at hei
            have hDf : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
                = false := by simpa using hD
            rcases List.mem_append.1 hei with hei | hei
            · exact Or.inl (hFo e
                (List.mem_of_getElem? (Option.mem_toList.1 hei)))
            · rcases (hIHW i hi hDf).1 e hei with ho | ⟨h'', hle'', ho⟩
              · exact Or.inr ⟨h + 1, Nat.le_refl _, ho⟩
              · exact Or.inr ⟨h'', by omega, ho⟩
        · rcases he with _ | ⟨_, he⟩
          · exact Or.inr ⟨h + 1, Nat.le_refl _, evOwner_upperOut sk k⟩
          · cases he
      · -- (2) the feeder's filter is the feed
        rw [hE,
          List.filter_cons_of_neg (by
            simp only [evOwner_wireIn sk hwf, beq_iff_eq]; omega),
          List.filter_cons_of_neg (by
            simp only [evOwner_askedIn, beq_iff_eq]; omega),
          List.filter_append,
          List.filter_cons_of_neg (by
            simp only [evOwner_upperOut, beq_iff_eq]; omega),
          List.filter_nil, List.append_nil]
        simp only [List.filter_flatMap]
        rw [flatMap_congr hkidMF, ← hF]
        exact flatMap_getElem?_toList F
      · -- (1) each covered walk's filter is its E run
        intro h' hle
        rcases Nat.eq_or_lt_of_le hle with heq | hlt
        · -- own stage: the E scope block, parent at the tail
          subst heq
          rw [Nat.sub_self, descIdx_zero, descIdx_zero, walkSegE_single]
          rw [hE,
            List.filter_cons_of_pos (by
              simp only [evOwner_wireIn sk hwf, beq_self_eq_true]),
            List.filter_cons_of_pos (by
              simp only [evOwner_askedIn, beq_self_eq_true]),
            List.filter_append,
            List.filter_cons_of_pos (by
              simp only [evOwner_upperOut, beq_self_eq_true]),
            List.filter_nil]
          simp only [List.filter_flatMap]
          rw [flatMap_congr hkidOwn, scopeBlockE, scopeSendsE_eq]
        · -- descendant stage: glue the kid runs
          have hle' : h' ≤ h := by omega
          have hwlt := walkIdx_lt sk hlt hh
          rw [hE,
            List.filter_cons_of_neg (by
              simp only [evOwner_wireIn sk hwf, beq_iff_eq]; omega),
            List.filter_cons_of_neg (by
              simp only [evOwner_askedIn, beq_iff_eq]; omega),
            List.filter_append,
            List.filter_cons_of_neg (by
              simp only [evOwner_upperOut, beq_iff_eq]; omega),
            List.filter_nil, List.append_nil]
          simp only [List.filter_flatMap]
          rw [flatMap_congr (hkidDesc h' hle'),
            walkSegE_glue_range sk h'
              (fun i => descIdx sk h' (h - h')
                (sk.wiresBefore (h + 1) k + i))
              (fun i => descIdx_mono sk h' (h - h') (by omega))
              (sk.nChildren (h + 1) (sk.stageScope (h + 1) k)),
            show h + 1 - h' = (h - h') + 1 from by omega, descIdx_succ,
            descIdx_succ, show h' + (h - h') + 1 = h + 1 from by omega,
            wiresBefore_succ sk hk, Nat.add_zero]

-- ==================================================== the top assembly

/-- The opening worklist's fuel-free E events: the openers, then the
root scope's E subtree. -/
theorem weave_flatMapE :
    (weaveOps sk).flatMap (opEventsE sk)
      = (iopenEvents sk ++ (ropenEvents sk).take 3)
        ++ opEventsE sk
            (.scope (sk.rootH - 1) 0 ((ropenEvents sk).drop 3)) := by
  unfold weaveOps
  rw [List.flatMap_append, List.flatMap_map, List.flatMap_singleton]
  have hemit : (fun e => opEventsE sk (WOp.emit e)) = fun e : Ev => [e] :=
    funext fun e => opEventsE_emit sk e
  rw [hemit, List.flatMap_singleton']

/-- THE E INITIAL ALIGNMENT (PROGRESS.md §9, 2a-align): the opening
worklist's future E events have in-range owners, and their per-owner
filters ARE the encoder-order manual traces. -/
theorem weaveE_initial_alignment (hwf : sk.wellFormed = true) :
    (∀ e ∈ (weaveOps sk).flatMap (opEventsE sk),
        evOwner sk e < manCount sk)
      ∧ manFilters sk ((weaveOps sk).flatMap (opEventsE sk))
        = (procsE sk).take (manCount sk) := by
  have hge := (wf_rootH hwf).2
  have hlen1 := wf_stageLen_top sk hwf
  have hss := wf_stageScope_top sk hwf
  have hF : ((ropenEvents sk).drop 3).length
      = sk.nChildren (sk.rootH - 1) (sk.stageScope (sk.rootH - 1) 0) := by
    rw [hss, nChildren_of_pos sk (by omega)]
    simp [ropenEvents, Skel.rootPending]
  have hFo : ∀ e ∈ (ropenEvents sk).drop 3, evOwner sk e = 1 :=
    fun e he => ropen_owner sk hwf e (List.mem_of_mem_drop he)
  obtain ⟨hown3, hfeed2, hwalk1⟩ := align_scopeE sk hwf (sk.rootH - 1) 0
    ((ropenEvents sk).drop 3) 1 (by omega) (by omega) hF hFo
    (by unfold walkIdx; omega)
  have htk3 : ∀ e ∈ (ropenEvents sk).take 3, evOwner sk e = 1 :=
    fun e he => ropen_owner sk hwf e (List.mem_of_mem_take he)
  have hio := iopen_owner sk hwf
  constructor
  · -- owners in range
    intro e he
    rw [weave_flatMapE] at he
    rcases List.mem_append.1 he with he | he
    · rcases List.mem_append.1 he with he | he
      · rw [hio e he]
        unfold manCount
        omega
      · rw [htk3 e he]
        unfold manCount
        omega
    · rcases hown3 e he with ho | ⟨h', -, ho⟩
      · rw [ho]
        unfold manCount
        omega
      · rw [ho]
        unfold walkIdx manCount
        omega
  · -- per-owner filters are the E manual traces
    have hrange : List.range (manCount sk)
        = [0, 1] ++ List.range' 2 sk.rootH := by
      have happ := List.range'_append
        (s := 0) (m := 2) (n := sk.rootH) (step := 1)
      rw [show 0 + 1 * 2 = 2 from by omega] at happ
      rw [manCount, List.range_eq_range', ← happ]
      rfl
    have htake : (procsE sk).take (manCount sk)
        = [iopenEvents sk, ropenEvents sk]
          ++ ((List.range sk.rootH).map fun i =>
              walkEventsE sk (wpk (sk.rootH - 1 - i))) := by
      have hsplit : procsE sk
          = ([iopenEvents sk, ropenEvents sk]
              ++ ((List.range sk.rootH).map fun i =>
                  walkEventsE sk (wpk (sk.rootH - 1 - i))))
            ++ ([absorbEvents sk]
              ++ sk.asmKeys.map (asmEvents sk)
              ++ [[(Chan.rootret, false, 0)], finEvents sk]) := by
        simp [procsE, wpk, List.append_assoc, Function.comp]
      rw [hsplit]
      refine List.take_left' ?_
      simp [manCount]
      omega
    rw [weave_flatMapE, htake]
    unfold manFilters
    rw [hrange, List.map_append]
    have h0 : ((iopenEvents sk ++ (ropenEvents sk).take 3)
        ++ opEventsE sk
            (.scope (sk.rootH - 1) 0 ((ropenEvents sk).drop 3))).filter
        (fun e => evOwner sk e == 0) = iopenEvents sk := by
      have hs0 : (opEventsE sk (.scope (sk.rootH - 1) 0
          ((ropenEvents sk).drop 3))).filter
          (fun e => evOwner sk e == 0) = [] := by
        rw [List.filter_eq_nil_iff]
        intro e he
        rcases hown3 e he with ho | ⟨h', -, ho⟩
        · simp only [ho, beq_iff_eq]
          omega
        · have : 2 ≤ walkIdx sk h' := by
            unfold walkIdx
            omega
          simp only [ho, beq_iff_eq]
          omega
      rw [List.filter_append, List.filter_append,
        filter_owner_all sk _ 0 hio,
        filter_owner_none sk _ htk3 (by omega), hs0,
        List.append_nil, List.append_nil]
    have h1 : ((iopenEvents sk ++ (ropenEvents sk).take 3)
        ++ opEventsE sk
            (.scope (sk.rootH - 1) 0 ((ropenEvents sk).drop 3))).filter
        (fun e => evOwner sk e == 1) = ropenEvents sk := by
      rw [List.filter_append, List.filter_append,
        filter_owner_none sk _ hio (by omega),
        filter_owner_all sk _ 1 htk3, hfeed2,
        List.nil_append, List.take_append_drop]
    congr 1
    · rw [List.map_cons, List.map_cons, List.map_nil, h0, h1]
    · rw [List.range'_eq_map_range, List.map_map]
      refine List.map_congr_left fun i hi => ?_
      rw [List.mem_range] at hi
      show ((iopenEvents sk ++ (ropenEvents sk).take 3)
          ++ opEventsE sk
              (.scope (sk.rootH - 1) 0 ((ropenEvents sk).drop 3))).filter
          (fun e => evOwner sk e == 2 + i)
        = walkEventsE sk (wpk (sk.rootH - 1 - i))
      have hwi : walkIdx sk (sk.rootH - 1 - i) = 2 + i := by
        unfold walkIdx
        omega
      rw [← hwi]
      have hseg := hwalk1 (sk.rootH - 1 - i) (by omega)
      rw [show sk.rootH - 1 - (sk.rootH - 1 - i) = i from by omega,
        descIdx_zero_arg] at hseg
      have hend : descIdx sk (sk.rootH - 1 - i) i (0 + 1)
          = sk.stageLen (sk.rootH - 1 - i) := by
        have hd := descIdx_total sk hwf i (sk.rootH - 1 - i) (by omega)
        rw [show sk.rootH - 1 - i + i = sk.rootH - 1 from by omega,
          hlen1] at hd
        rw [show (0 + 1 : Nat) = 1 from rfl]
        exact hd
      rw [hend, walkSegE_full] at hseg
      rw [List.filter_append, List.filter_append,
        filter_owner_none sk _ hio (by omega),
        filter_owner_none sk _ htk3 (by omega), hseg,
        List.nil_append, List.nil_append]

-- ============================================ fuel and the invariant

/-- The opening worklist's E emission count is bounded by the (shared)
event total: `goEventsE_weave`'s missing hypothesis. -/
theorem weave_events_lengthE (hwf : sk.wellFormed = true) :
    ((weaveOps sk).flatMap (opEventsE sk)).length ≤ totalEvents sk := by
  obtain ⟨hown, halign⟩ := weaveE_initial_alignment sk hwf
  have hsum := manFilters_length_sum sk _ hown
  rw [halign] at hsum
  have htot : totalEventsE sk
      = (((procsE sk).take (manCount sk)).map List.length).sum
        + (((procsE sk).drop (manCount sk)).map List.length).sum := by
    unfold totalEventsE
    conv => lhs; rw [← List.take_append_drop (manCount sk) (procsE sk)]
    rw [List.map_append, List.sum_append]
  have heq := totalEventsE_eq sk
  omega

/-- The eweave's state carries the counting invariant at the
encoder-order family, hypothesis-free: the E initial alignment
discharges `weaveStateE_wcount`. -/
theorem weaveE_wcount (hwf : sk.wellFormed = true) :
    WCountP sk (procsE sk) [] (weaveStateE sk) := by
  have hgo := goEventsE_weave sk (weave_events_lengthE sk hwf)
  obtain ⟨hown, halign⟩ := weaveE_initial_alignment sk hwf
  exact weaveStateE_wcount sk (by rw [hgo]; exact halign)
    (by rw [hgo]; exact hown)

end StreamingMirror.Sched
