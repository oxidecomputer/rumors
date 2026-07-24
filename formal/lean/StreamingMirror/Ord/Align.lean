/-
The O initial alignment: `align_scopeE`'s master induction transcribed
over the O scope expander, and its top assembly against `procsO`.

One delta drives every difference from `AlignE.lean`: each scope's
two-receive prologue sits in ITS walk's assigned order (`prologueO`),
not the fixed wire-then-asked conses — so the own-walk filter of a
scope's expansion is `scopeBlockO` (the ordered prologue, per-kid
`childChunk`s, parent last). Both prologue receives are owned by the
scope's own walk, so the own filter KEEPS them in the assignment's
order and every feeder/descendant filter DROPS the pair whole — the
induction shape is `align_scopeE`'s with the two per-receive
`filter_cons` steps replaced by one `filter_append` against the
`prologueO` block. The send suffixes, kid dispatch, and descent
algebra are the E proof's verbatim.

The payoff at the bottom of the file: `weaveO_wcount` — the O weave's
final state carries the counting invariant at the `procsO` family,
every assignment — the O consumption frame's entry fact.

Chain (ord, stage D): the witness and its alignment; consumed by the
O master induction. Base mirror: AlignE.lean. Map: PROGRESS.md §13.
-/
import StreamingMirror.Ord.Expand
import StreamingMirror.Proofs.Sched.Weave.AlignE

namespace StreamingMirror.Ord

open Model
open Sched

variable (sk : Skel) (ord : OrdMap)

-- ================================================ statement vocabulary

/-- A contiguous run of stage-`h'` O scope blocks: trace segment
`[a, b)` of the stage's walk, prologue in the assigned order. -/
def walkSegO (h' a b : Nat) : List Ev :=
  (List.range' a (b - a)).flatMap (scopeBlockO sk ord (wpk h'))

theorem walkSegO_empty (h' a : Nat) : walkSegO sk ord h' a a = [] := by
  unfold walkSegO
  rw [Nat.sub_self]
  rfl

theorem walkSegO_single (h' k : Nat) :
    walkSegO sk ord h' k (k + 1) = scopeBlockO sk ord (wpk h') k := by
  unfold walkSegO
  rw [Nat.add_sub_cancel_left, List.range'_one, List.flatMap_cons,
    List.flatMap_nil, List.append_nil]

/-- Abutting O stage runs glue into one. -/
theorem walkSegO_glue {h' a b c : Nat} (hab : a ≤ b) (hbc : b ≤ c) :
    walkSegO sk ord h' a b ++ walkSegO sk ord h' b c
      = walkSegO sk ord h' a c := by
  unfold walkSegO
  rw [← List.flatMap_append,
    show c - a = (b - a) + (c - b) from by omega,
    ← List.range'_append,
    show a + 1 * (b - a) = b from by omega]

theorem walkSegO_glue_range (h' : Nat) (g : Nat → Nat)
    (hmono : ∀ i, g i ≤ g (i + 1)) :
    ∀ n, (List.range n).flatMap
        (fun i => walkSegO sk ord h' (g i) (g (i + 1)))
      = walkSegO sk ord h' (g 0) (g n) := by
  intro n
  induction n with
  | zero => rw [List.range_zero, List.flatMap_nil, walkSegO_empty]
  | succ n ih =>
      have h0n : g 0 ≤ g n := by
        clear ih
        induction n with
        | zero => exact Nat.le_refl _
        | succ m ihm => exact Nat.le_trans ihm (hmono m)
      rw [List.range_succ, List.flatMap_append, ih, List.flatMap_cons,
        List.flatMap_nil, List.append_nil, walkSegO_glue sk ord h0n (hmono n)]

/-- A whole-stage O run is the stage's O walk trace. -/
theorem walkSegO_full (h' : Nat) :
    walkSegO sk ord h' 0 (sk.stageLen h')
      = walkEventsO sk ord (wpk h') := by
  unfold walkSegO walkEventsO
  rw [Nat.sub_zero, ← List.range_eq_range']
  rfl

-- ============================================== the prologue's filters

/-- The ordered prologue's members are exactly the two receives. -/
theorem mem_prologueO {e : Ev} {pk : Party × Nat} {k : Nat}
    (he : e ∈ prologueO ord pk k) :
    e = (wireIn pk, false, k) ∨ e = (askedIn pk, false, k) := by
  unfold prologueO at he
  revert he
  cases ord.walk pk with
  | replyFirst =>
      intro he
      rcases he with _ | ⟨_, he⟩
      · exact Or.inl rfl
      rcases he with _ | ⟨_, he⟩
      · exact Or.inr rfl
      · cases he
  | queryFirst =>
      intro he
      rcases he with _ | ⟨_, he⟩
      · exact Or.inr rfl
      rcases he with _ | ⟨_, he⟩
      · exact Or.inl rfl
      · cases he

/-- Both prologue receives belong to the scope's own walk, either
order. -/
theorem prologueO_owner (hwf : sk.wellFormed = true) (h k : Nat) :
    ∀ e ∈ prologueO ord (wpk h) k, evOwner sk e = walkIdx sk h := by
  intro e he
  rcases mem_prologueO ord he with rfl | rfl
  · exact evOwner_wireIn sk hwf h k
  · exact evOwner_askedIn sk k

/-- The own-walk filter keeps the whole ordered prologue, in the
assignment's order. -/
theorem prologueO_filter_own (hwf : sk.wellFormed = true) (h k : Nat) :
    (prologueO ord (wpk h) k).filter
        (fun e => evOwner sk e == walkIdx sk h)
      = prologueO ord (wpk h) k :=
  filter_owner_all sk _ _ (prologueO_owner sk ord hwf h k)

/-- Every other owner's filter drops the ordered prologue whole. -/
theorem prologueO_filter_ne (hwf : sk.wellFormed = true) (h k : Nat)
    {m : Nat} (hne : walkIdx sk h ≠ m) :
    (prologueO ord (wpk h) k).filter (fun e => evOwner sk e == m) = [] :=
  filter_owner_none sk _ (prologueO_owner sk ord hwf h k) hne

-- ==================================================== the O expansions

/-- An O scope op's expansion, events flattened: the ordered prologue,
the kid ops in slot order, the parent summary last. -/
theorem opEventsO_scope_eq {h k : Nat} (hk : k ≤ sk.stageLen h)
    (feed : List Ev) :
    opEventsO sk ord (.scope h k feed)
      = prologueO ord (wpk h) k
          ++ ((List.range (sk.nChildren h (sk.stageScope h k))).flatMap
                (fun i => opEventsO sk ord (.kid h k (sk.stageScope h k)
                  none (sk.wiresBefore h k) i feed))
            ++ [((upperOut (wpk h), true, k) : Ev)]) := by
  rw [opEventsO_scope]
  simp only [wScopeOpsO]
  rw [kidBase_eq_wiresBefore sk h k hk]
  simp only [wpk]
  simp [opEventsO_emit, List.flatMap_map, List.flatMap_append]

/-- An O kid op's expansion, events flattened: the trace chunk with
the kid's feed query and subtree in place — never a parent, never a
prologue (the kid's own prologue dispatches inside its subtree's
`.scope` op). -/
theorem opEventsO_kid_eq (h k : Nat) (lastD : Option Nat)
    (kidBase i : Nat) (feed : List Ev) :
    opEventsO sk ord (.kid h k (sk.stageScope h k) lastD kidBase i feed)
      = (wireOut (wpk h), true, sk.wiresBefore h k + i)
          :: (if sk.childIsD h (sk.stageScope h k) i then
                (lowerOut (wpk h), true,
                    sk.dsBefore h k + dRank sk (wpk h) k i)
                  :: (feed[i]?.toList
                    ++ opEventsO sk ord
                        (.scope (h - 1) (kidBase + i) (chunkQ sk h k i)))
              else feed[i]?.toList
                ++ (if h == 0 then []
                    else opEventsO sk ord
                      (.scope (h - 1) (kidBase + i) []))) := by
  rw [opEventsO_kid]
  simp only [wKidOpsE, wpk]
  cases hfi : feed[i]? <;>
    by_cases hD : sk.childIsD h (sk.stageScope h k) i <;>
    by_cases h0 : (h == 0) <;>
    simp [hD, h0, opEventsO_emit, dRank, qSum, chunkQ, wpk]

-- ================================================ the master induction

/-- The O subtree alignment: `align_scopeE`'s three clauses over the
ord-dispatched expansion. Clause (1)'s own-stage form is `walkSegO` —
the assignment-ordered scope blocks; the ownership and feeder clauses
match the E statement shape. -/
theorem align_scopeO (hwf : sk.wellFormed = true) :
    ∀ (h k : Nat) (F : List Ev) (mF : Nat),
      h < sk.rootH → k < sk.stageLen h →
      F.length = sk.nChildren h (sk.stageScope h k) →
      (∀ e ∈ F, evOwner sk e = mF) →
      mF < walkIdx sk h →
      ((∀ e ∈ opEventsO sk ord (.scope h k F),
          evOwner sk e = mF
            ∨ ∃ h', h' ≤ h ∧ evOwner sk e = walkIdx sk h')
        ∧ (opEventsO sk ord (.scope h k F)).filter
            (fun e => evOwner sk e == mF) = F
        ∧ ∀ h' ≤ h,
            (opEventsO sk ord (.scope h k F)).filter
                (fun e => evOwner sk e == walkIdx sk h')
              = walkSegO sk ord h' (descIdx sk h' (h - h') k)
                  (descIdx sk h' (h - h') (k + 1))) := by
  intro h
  induction h with
  | zero =>
      intro k F mF hh hk hF hFo hmF
      have hD0 : ∀ i, sk.childIsD 0 (sk.stageScope 0 k) i = false :=
        fun _ => rfl
      have hE := opEventsO_scope_eq sk ord (Nat.le_of_lt hk) F
      have hkidE : ∀ i,
          opEventsO sk ord (.kid 0 k (sk.stageScope 0 k) none
            (sk.wiresBefore 0 k) i F)
          = (wireOut (wpk 0), true, sk.wiresBefore 0 k + i)
              :: F[i]?.toList := by
        intro i
        rw [opEventsO_kid_eq,
          if_neg (by rw [hD0 i]; exact Bool.false_ne_true),
          if_pos (show ((0 : Nat) == 0) = true by rfl), List.append_nil]
      refine ⟨?_, ?_, ?_⟩
      · -- (3) ownership: everything is the feeder's or the leaf walk's
        intro e he
        rw [hE] at he
        rcases List.mem_append.1 he with he | he
        · exact Or.inr ⟨0, Nat.le_refl 0,
            prologueO_owner sk ord hwf 0 k e he⟩
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
        rw [hE, List.filter_append,
          prologueO_filter_ne sk ord hwf 0 k (by omega),
          List.nil_append, List.filter_append,
          List.filter_cons_of_neg (by
            simp only [evOwner_upperOut, beq_iff_eq]; omega),
          List.filter_nil, List.append_nil]
        have hkMF : ∀ i ∈ List.range (sk.nChildren 0 (sk.stageScope 0 k)),
            (opEventsO sk ord (.kid 0 k (sk.stageScope 0 k) none
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
      · -- (1) the leaf walk's filter is the O scope block
        intro h' hle
        have h0 : h' = 0 := Nat.le_zero.mp hle
        subst h0
        rw [Nat.sub_self, descIdx_zero, descIdx_zero, walkSegO_single]
        have hkOwn : ∀ i ∈ List.range (sk.nChildren 0 (sk.stageScope 0 k)),
            (opEventsO sk ord (.kid 0 k (sk.stageScope 0 k) none
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
        rw [hE, List.filter_append, prologueO_filter_own sk ord hwf 0 k,
          List.filter_append,
          List.filter_cons_of_pos (by
            simp only [evOwner_upperOut, beq_self_eq_true]),
          List.filter_nil]
        simp only [List.filter_flatMap]
        rw [flatMap_congr hkOwn, scopeBlockO, scopeSendsE_eq]
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
      have hE := opEventsO_scope_eq sk ord (Nat.le_of_lt hk) F
      have hkidE : ∀ i,
          opEventsO sk ord (.kid (h + 1) k (sk.stageScope (h + 1) k)
            none (sk.wiresBefore (h + 1) k) i F)
          = (wireOut (wpk (h + 1)), true, sk.wiresBefore (h + 1) k + i)
              :: (if sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i then
                    (lowerOut (wpk (h + 1)), true,
                        sk.dsBefore (h + 1) k + dRank sk (wpk (h + 1)) k i)
                      :: (F[i]?.toList
                        ++ opEventsO sk ord (.scope h
                            (sk.wiresBefore (h + 1) k + i)
                            (chunkQ sk (h + 1) k i)))
                  else F[i]?.toList
                    ++ opEventsO sk ord (.scope h
                        (sk.wiresBefore (h + 1) k + i) [])) := by
        intro i
        rw [opEventsO_kid_eq]
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
          (opEventsO sk ord (.kid (h + 1) k (sk.stageScope (h + 1) k)
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
          (opEventsO sk ord (.kid (h + 1) k (sk.stageScope (h + 1) k)
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
        · have hSubDrop : (opEventsO sk ord (.scope h
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
          have hSubDrop : (opEventsO sk ord (.scope h
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
      -- (C) each kid's descendant-stage filter is its subtree's O run
      have hkidDesc : ∀ h', h' ≤ h → ∀ i ∈ List.range
            (sk.nChildren (h + 1) (sk.stageScope (h + 1) k)),
          (opEventsO sk ord (.kid (h + 1) k (sk.stageScope (h + 1) k)
              none (sk.wiresBefore (h + 1) k) i F)).filter
            (fun e => evOwner sk e == walkIdx sk h')
          = walkSegO sk ord h'
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
        rcases List.mem_append.1 he with he | he
        · exact Or.inr ⟨h + 1, Nat.le_refl _,
            prologueO_owner sk ord hwf (h + 1) k e he⟩
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
        rw [hE, List.filter_append,
          prologueO_filter_ne sk ord hwf (h + 1) k (by omega),
          List.nil_append, List.filter_append,
          List.filter_cons_of_neg (by
            simp only [evOwner_upperOut, beq_iff_eq]; omega),
          List.filter_nil, List.append_nil]
        simp only [List.filter_flatMap]
        rw [flatMap_congr hkidMF, ← hF]
        exact flatMap_getElem?_toList F
      · -- (1) each covered walk's filter is its O run
        intro h' hle
        rcases Nat.eq_or_lt_of_le hle with heq | hlt
        · -- own stage: the O scope block, ordered prologue, parent tail
          subst heq
          rw [Nat.sub_self, descIdx_zero, descIdx_zero, walkSegO_single]
          rw [hE, List.filter_append,
            prologueO_filter_own sk ord hwf (h + 1) k,
            List.filter_append,
            List.filter_cons_of_pos (by
              simp only [evOwner_upperOut, beq_self_eq_true]),
            List.filter_nil]
          simp only [List.filter_flatMap]
          rw [flatMap_congr hkidOwn, scopeBlockO, scopeSendsE_eq]
        · -- descendant stage: glue the kid runs
          have hle' : h' ≤ h := by omega
          have hwlt := walkIdx_lt sk hlt hh
          rw [hE, List.filter_append,
            prologueO_filter_ne sk ord hwf (h + 1) k (by omega),
            List.nil_append, List.filter_append,
            List.filter_cons_of_neg (by
              simp only [evOwner_upperOut, beq_iff_eq]; omega),
            List.filter_nil, List.append_nil]
          simp only [List.filter_flatMap]
          rw [flatMap_congr (hkidDesc h' hle'),
            walkSegO_glue_range sk ord h'
              (fun i => descIdx sk h' (h - h')
                (sk.wiresBefore (h + 1) k + i))
              (fun i => descIdx_mono sk h' (h - h') (by omega))
              (sk.nChildren (h + 1) (sk.stageScope (h + 1) k)),
            show h + 1 - h' = (h - h') + 1 from by omega, descIdx_succ,
            descIdx_succ, show h' + (h - h') + 1 = h + 1 from by omega,
            wiresBefore_succ sk hk, Nat.add_zero]

-- ==================================================== the top assembly

/-- The opening worklist's fuel-free O events: the openers, then the
root scope's O subtree. -/
theorem weave_flatMapO :
    (weaveOps sk).flatMap (opEventsO sk ord)
      = (iopenEvents sk ++ (ropenEvents sk).take 3)
        ++ opEventsO sk ord
            (.scope (sk.rootH - 1) 0 ((ropenEvents sk).drop 3)) := by
  unfold weaveOps
  rw [List.flatMap_append, List.flatMap_map, List.flatMap_singleton]
  have hemit : (fun e => opEventsO sk ord (WOp.emit e))
      = fun e : Ev => [e] :=
    funext fun e => opEventsO_emit sk ord e
  rw [hemit, List.flatMap_singleton']

/-- THE O INITIAL ALIGNMENT: the opening worklist's future O events
have in-range owners, and their per-owner filters ARE the O manual
traces — every assignment of the class. -/
theorem weaveO_initial_alignment (hwf : sk.wellFormed = true) :
    (∀ e ∈ (weaveOps sk).flatMap (opEventsO sk ord),
        evOwner sk e < manCount sk)
      ∧ manFilters sk ((weaveOps sk).flatMap (opEventsO sk ord))
        = (procsO sk ord).take (manCount sk) := by
  have hge := (wf_rootH hwf).2
  have hlen1 := wf_stageLen_top sk hwf
  have hss := wf_stageScope_top sk hwf
  have hF : ((ropenEvents sk).drop 3).length
      = sk.nChildren (sk.rootH - 1) (sk.stageScope (sk.rootH - 1) 0) := by
    rw [hss, nChildren_of_pos sk (by omega)]
    simp [ropenEvents, Skel.rootPending]
  have hFo : ∀ e ∈ (ropenEvents sk).drop 3, evOwner sk e = 1 :=
    fun e he => ropen_owner sk hwf e (List.mem_of_mem_drop he)
  obtain ⟨hown3, hfeed2, hwalk1⟩ := align_scopeO sk ord hwf (sk.rootH - 1) 0
    ((ropenEvents sk).drop 3) 1 (by omega) (by omega) hF hFo
    (by unfold walkIdx; omega)
  have htk3 : ∀ e ∈ (ropenEvents sk).take 3, evOwner sk e = 1 :=
    fun e he => ropen_owner sk hwf e (List.mem_of_mem_take he)
  have hio := iopen_owner sk hwf
  constructor
  · -- owners in range
    intro e he
    rw [weave_flatMapO] at he
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
  · -- per-owner filters are the O manual traces
    have hrange : List.range (manCount sk)
        = [0, 1] ++ List.range' 2 sk.rootH := by
      have happ := List.range'_append
        (s := 0) (m := 2) (n := sk.rootH) (step := 1)
      rw [show 0 + 1 * 2 = 2 from by omega] at happ
      rw [manCount, List.range_eq_range', ← happ]
      rfl
    have htake : (procsO sk ord).take (manCount sk)
        = [iopenEvents sk, ropenEvents sk]
          ++ ((List.range sk.rootH).map fun i =>
              walkEventsO sk ord (wpk (sk.rootH - 1 - i))) := by
      have hsplit : procsO sk ord
          = ([iopenEvents sk, ropenEvents sk]
              ++ ((List.range sk.rootH).map fun i =>
                  walkEventsO sk ord (wpk (sk.rootH - 1 - i))))
            ++ ([absorbEventsO sk ord]
              ++ sk.asmKeys.map (asmEvents sk)
              ++ [[(Chan.rootret, false, 0)], finEvents sk]) := by
        simp [procsO, wpk, List.append_assoc, Function.comp]
      rw [hsplit]
      refine List.take_left' ?_
      simp [manCount]
      omega
    rw [weave_flatMapO, htake]
    unfold manFilters
    rw [hrange, List.map_append]
    have h0 : ((iopenEvents sk ++ (ropenEvents sk).take 3)
        ++ opEventsO sk ord
            (.scope (sk.rootH - 1) 0 ((ropenEvents sk).drop 3))).filter
        (fun e => evOwner sk e == 0) = iopenEvents sk := by
      have hs0 : (opEventsO sk ord (.scope (sk.rootH - 1) 0
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
        ++ opEventsO sk ord
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
          ++ opEventsO sk ord
              (.scope (sk.rootH - 1) 0 ((ropenEvents sk).drop 3))).filter
          (fun e => evOwner sk e == 2 + i)
        = walkEventsO sk ord (wpk (sk.rootH - 1 - i))
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
      rw [hend, walkSegO_full] at hseg
      rw [List.filter_append, List.filter_append,
        filter_owner_none sk _ hio (by omega),
        filter_owner_none sk _ htk3 (by omega), hseg,
        List.nil_append, List.nil_append]

-- ============================================ fuel and the invariant

/-- The opening worklist's O emission count is bounded by the (shared)
event total: `goEventsO_weave`'s missing hypothesis. -/
theorem weave_events_lengthO (hwf : sk.wellFormed = true) :
    ((weaveOps sk).flatMap (opEventsO sk ord)).length
      ≤ totalEvents sk := by
  obtain ⟨hown, halign⟩ := weaveO_initial_alignment sk ord hwf
  have hsum := manFilters_length_sum sk _ hown
  rw [halign] at hsum
  have htot : totalEventsO sk ord
      = (((procsO sk ord).take (manCount sk)).map List.length).sum
        + (((procsO sk ord).drop (manCount sk)).map List.length).sum := by
    unfold totalEventsO
    conv => lhs; rw [← List.take_append_drop (manCount sk) (procsO sk ord)]
    rw [List.map_append, List.sum_append]
  have heqO := totalEventsO_eq sk ord
  have heqE := totalEventsE_eq sk
  omega

/-- The O weave's state carries the counting invariant at the O
family, hypothesis-free: the O initial alignment discharges
`weaveStateO_wcount` — every assignment of the class. -/
theorem weaveO_wcount (hwf : sk.wellFormed = true) :
    WCountP sk (procsO sk ord) [] (weaveStateO sk ord) := by
  have hgo := goEventsO_weave sk ord (weave_events_lengthO sk ord hwf)
  obtain ⟨hown, halign⟩ := weaveO_initial_alignment sk ord hwf
  exact weaveStateO_wcount sk ord (by rw [hgo]; exact halign)
    (by rw [hgo]; exact hown)

end StreamingMirror.Ord
