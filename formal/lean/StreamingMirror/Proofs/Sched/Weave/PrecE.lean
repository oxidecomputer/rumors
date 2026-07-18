/-
Dep-closure of the encoder-order future (PROGRESS.md §9, unit 2a-dep,
route (i)): `DepOK` transfers from the d5 future to the E future
instead of re-running Prec.lean's expansion induction.

Why transfer is sound: `manDep` yields `some` only for wire/asked
events and maps every parent summary to `none` (`manDep_upper_snd`) —
no event's dep is a parent and parents have no deps. The E reorder
moves ONLY parent summaries (each scope's sole own-owner moved event),
so the wire/asked subsequence of the two futures is literally equal
(`opEventsE_filter_scope`), and every (dep, event) pair keeps its
relative order. The transfer lemma (`depOK_transfer`) makes that
argument positional: it needs the E future duplicate-free, which the
canon shapes supply (`trace_nodup` per trace, the E initial alignment
to partition the future into traces by owner).
-/
import StreamingMirror.Proofs.Sched.Weave.AlignE
import StreamingMirror.Proofs.Sched.Weave.Prec

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ============================================ the wire/asked class

/-- The dep-carrying channel class: wire and asked events, either
side — everything `manDep` reads or returns. -/
def isWA : Ev → Bool
  | (Chan.wire _ _, _, _) => true
  | (Chan.asked _ _, _, _) => true
  | _ => false

/-- Every dep-carrying event and every dep value is wire/asked. -/
theorem manDep_isWA (e d : Ev) (hd : manDep e = some d) :
    isWA e = true ∧ isWA d = true := by
  obtain ⟨c, b, n⟩ := e
  cases c <;> cases b <;> simp only [manDep] at hd
  all_goals first
    | (injection hd with h; subst h; exact ⟨rfl, rfl⟩)
    | (split at hd
       · cases hd
       · injection hd with h; subst h; exact ⟨rfl, rfl⟩)
    | cases hd

-- ================================================ Nodup from canon

/-- Class-local duplicate freedom lifts: if every element's class
filter is duplicate-free, the list is. -/
theorem nodup_of_class_filters {α : Type _} (cls : α → α → Bool)
    (hrefl : ∀ a, cls a a = true) :
    ∀ (l : List α), (∀ a ∈ l, (l.filter (cls a)).Nodup) → l.Nodup := by
  intro l
  induction l with
  | nil => intro _; exact List.nodup_nil
  | cons a l ih =>
      intro hfil
      rw [List.nodup_cons]
      refine ⟨?_, ?_⟩
      · intro hal
        have h := hfil a (List.mem_cons_self ..)
        rw [List.filter_cons_of_pos (hrefl a)] at h
        exact (List.nodup_cons.mp h).1
          (List.mem_filter.mpr ⟨hal, hrefl a⟩)
      · refine ih fun a' ha' => ?_
        have h := hfil a' (List.mem_cons_of_mem _ ha')
        exact ((List.sublist_cons_self a l).filter _).nodup h

/-- The canonical projection never repeats: its seqs enumerate a
range. -/
theorem canon_nodup (c : Chan) (b : Bool) (m : Nat) :
    (canon c b m).Nodup := by
  unfold canon
  refine List.Pairwise.map _ (fun x y (h : x < y) => ?_)
    List.pairwise_lt_range
  intro heq
  cases heq
  omega

/-- A trace all of whose projections are canon-shaped is
duplicate-free: a repeat would repeat inside its own channel-side
projection. -/
theorem trace_nodup {t : List Ev}
    (hcanon : ∀ c b, ∃ m, proj c b t = canon c b m) : t.Nodup := by
  refine nodup_of_class_filters
    (fun a e => decide (e.1 = a.1) && (e.2.1 == a.2.1))
    (fun a => by simp) t (fun a _ => ?_)
  obtain ⟨m, hm⟩ := hcanon a.1 a.2.1
  rw [show t.filter (fun e => decide (e.1 = a.1) && (e.2.1 == a.2.1))
    = proj a.1 a.2.1 t from rfl, hm]
  exact canon_nodup a.1 a.2.1 m

/-- The E future is duplicate-free: the alignment partitions it into
the `procsE` traces by owner, and every `procsE` trace is
canon-shaped. -/
theorem weaveE_future_nodup (hwf : sk.wellFormed = true) :
    ((weaveOps sk).flatMap (opEventsE sk)).Nodup := by
  obtain ⟨hown, halign⟩ := weaveE_initial_alignment sk hwf
  refine nodup_of_class_filters
    (fun a e => evOwner sk e == evOwner sk a)
    (fun a => beq_self_eq_true _) _ (fun a ha => ?_)
  have h1 : ((weaveOps sk).flatMap (opEventsE sk)).filter
      (fun e => evOwner sk e == evOwner sk a)
      ∈ manFilters sk ((weaveOps sk).flatMap (opEventsE sk)) := by
    unfold manFilters
    exact List.mem_map.mpr
      ⟨evOwner sk a, List.mem_range.mpr (hown a ha), rfl⟩
  rw [halign] at h1
  exact trace_nodup fun c b =>
    procsE_canon sk c b _ (List.mem_of_mem_take h1)

-- ============================================ the positional transfer

/-- In a duplicate-free list, a split around a fixed element is
unique on the left. -/
theorem nodup_append_cons_left_inj {α : Type _} {e : α} :
    ∀ {X X' Y Y' : List α}, (X ++ e :: Y).Nodup →
      X ++ e :: Y = X' ++ e :: Y' → X = X' := by
  intro X
  induction X with
  | nil =>
      intro X' Y Y' hnd heq
      cases X' with
      | nil => rfl
      | cons a X'' =>
          rw [List.nil_append] at heq
          injection heq with h1 h2
          subst h1
          refine absurd ?_ (List.nodup_cons.mp hnd).1
          rw [h2]
          exact List.mem_append_right _ (List.mem_cons_self ..)
  | cons a X ih =>
      intro X' Y Y' hnd heq
      cases X' with
      | nil =>
          rw [List.nil_append] at heq
          injection heq with h1 h2
          subst h1
          exact absurd
            (List.mem_append_right _ (List.mem_cons_self ..))
            (List.nodup_cons.mp hnd).1
      | cons a' X'' =>
          injection heq with h1 h2
          subst h1
          rw [ih (List.nodup_cons.mp hnd).2 h2]

/-- THE TRANSFER: dep-closure survives any reorder that fixes the
dep-carrying subsequence, provided the target is duplicate-free. -/
theorem depOK_transfer {done l l' : List Ev} (P : Ev → Bool)
    (hdep : ∀ e d, manDep e = some d → P e = true ∧ P d = true)
    (hfil : l'.filter P = l.filter P)
    (hnd : l'.Nodup) (h : DepOK done l) : DepOK done l' := by
  intro i e d hi hd
  obtain ⟨hPe, hPd⟩ := hdep e d hd
  obtain ⟨hi', hei⟩ := List.getElem?_eq_some_iff.mp hi
  have hl' : l' = l'.take i ++ e :: l'.drop (i + 1) := by
    conv => lhs; rw [← List.take_append_drop i l']
    rw [List.drop_eq_getElem_cons hi', hei]
  have hel : e ∈ l := by
    have hm : e ∈ l.filter P := by
      rw [← hfil]
      exact List.mem_filter.mpr ⟨List.mem_of_getElem? hi, hPe⟩
    exact (List.mem_filter.mp hm).1
  obtain ⟨j, hjlt, hje⟩ := List.getElem_of_mem hel
  have hj? : l[j]? = some e := by
    rw [List.getElem?_eq_getElem hjlt, hje]
  rcases List.mem_append.1 (h j e d hj? hd) with hind | hintk
  · exact List.mem_append_left _ hind
  · have hl : l = l.take j ++ e :: l.drop (j + 1) := by
      conv => lhs; rw [← List.take_append_drop j l]
      rw [List.drop_eq_getElem_cons hjlt, hje]
    have hfl : l.filter P
        = (l.take j).filter P ++ e :: (l.drop (j + 1)).filter P := by
      conv => lhs; rw [hl]
      rw [List.filter_append, List.filter_cons_of_pos hPe]
    have hfl' : l'.filter P
        = (l'.take i).filter P ++ e :: (l'.drop (i + 1)).filter P := by
      conv => lhs; rw [hl']
      rw [List.filter_append, List.filter_cons_of_pos hPe]
    have hndf : (l'.filter P).Nodup := (List.filter_sublist ..).nodup hnd
    have hXX : (l'.take i).filter P = (l.take j).filter P :=
      nodup_append_cons_left_inj (by rw [← hfl']; exact hndf)
        (by rw [← hfl', hfil, hfl])
    have hd' : d ∈ (l'.take i).filter P := by
      rw [hXX]
      exact List.mem_filter.mpr ⟨hintk, hPd⟩
    exact List.mem_append_right _ (List.mem_filter.mp hd').1

-- ==================================== the two futures' shared class

/-- E and d5 scope expansions agree on every parent-free filter: only
the parent summaries move between the orders. -/
theorem opEventsE_filter_scope (hwf : sk.wellFormed = true)
    (P : Ev → Bool)
    (hup : ∀ (pk : Party × Nat) (k : Nat),
      P ((upperOut pk, true, k) : Ev) = false) :
    ∀ (h k : Nat) (feed : List Ev), h < sk.rootH → k < sk.stageLen h →
      (opEventsE sk (.scope h k feed)).filter P
        = (opEvents sk (.scope h k feed)).filter P := by
  intro h
  induction h with
  | zero =>
      intro k feed hh hk
      have hD0 : ∀ i, sk.childIsD 0 (sk.stageScope 0 k) i = false :=
        fun _ => rfl
      have hLn : lastDOf sk 0 k = none := by
        unfold lastDOf
        rw [List.getLast?_eq_none_iff, List.filter_eq_nil_iff]
        intro a _
        rw [hD0 a]
        exact Bool.false_ne_true
      have hEE := opEventsE_scope_eq sk (Nat.le_of_lt hk) feed
      have hED := opEvents_scope_eq sk (Nat.le_of_lt hk) feed
      rw [hLn, if_pos (show ((none : Option Nat) == none) = true
        by rfl)] at hED
      have hkidE_E : ∀ i,
          opEventsE sk (.kid 0 k (sk.stageScope 0 k) none
            (sk.wiresBefore 0 k) i feed)
          = (wireOut (wpk 0), true, sk.wiresBefore 0 k + i)
              :: feed[i]?.toList := by
        intro i
        rw [opEventsE_kid_eq,
          if_neg (by rw [hD0 i]; exact Bool.false_ne_true),
          if_pos (show ((0 : Nat) == 0) = true by rfl), List.append_nil]
      have hkidE_D : ∀ i,
          opEvents sk (.kid 0 k (sk.stageScope 0 k) none
            (sk.wiresBefore 0 k) i feed)
          = (wireOut (wpk 0), true, sk.wiresBefore 0 k + i)
              :: feed[i]?.toList := by
        intro i
        rw [opEvents_kid_eq,
          if_neg (by rw [hD0 i]; exact Bool.false_ne_true),
          if_pos (show ((0 : Nat) == 0) = true by rfl), List.append_nil]
      rw [hEE, hED]
      have hFM : (List.range (sk.nChildren 0 (sk.stageScope 0 k))).flatMap
            (fun i => opEventsE sk (.kid 0 k (sk.stageScope 0 k) none
              (sk.wiresBefore 0 k) i feed))
          = (List.range (sk.nChildren 0 (sk.stageScope 0 k))).flatMap
            (fun i => opEvents sk (.kid 0 k (sk.stageScope 0 k) none
              (sk.wiresBefore 0 k) i feed)) :=
        flatMap_congr fun i _ => by rw [hkidE_E i, hkidE_D i]
      rw [hFM]
      have hmid : ((List.range (sk.nChildren 0 (sk.stageScope 0 k))).flatMap
              (fun i => opEvents sk (.kid 0 k (sk.stageScope 0 k) none
                (sk.wiresBefore 0 k) i feed))
            ++ [((upperOut (wpk 0), true, k) : Ev)]).filter P
          = ([((upperOut (wpk 0), true, k) : Ev)]
            ++ (List.range (sk.nChildren 0 (sk.stageScope 0 k))).flatMap
              (fun i => opEvents sk (.kid 0 k (sk.stageScope 0 k) none
                (sk.wiresBefore 0 k) i feed))).filter P := by
        rw [List.filter_append, List.filter_append,
          List.filter_cons_of_neg (by
            rw [hup (wpk 0) k]; exact Bool.false_ne_true),
          List.filter_nil, List.append_nil, List.nil_append]
      simp only [List.filter_cons]
      rw [hmid]
  | succ h ih =>
      intro k feed hh hk
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
      have hEE := opEventsE_scope_eq sk (Nat.le_of_lt hk) feed
      have hED := opEvents_scope_eq sk (Nat.le_of_lt hk) feed
      have hUdrop : ((if lastDOf sk (h + 1) k == none
            then [((upperOut (wpk (h + 1)), true, k) : Ev)]
            else []).filter P) = [] := by
        split
        · rw [List.filter_cons_of_neg (by rw [hup (wpk (h + 1)) k]; exact Bool.false_ne_true),
            List.filter_nil]
        · rfl
      have hkid : ∀ i ∈ List.range
            (sk.nChildren (h + 1) (sk.stageScope (h + 1) k)),
          (opEventsE sk (.kid (h + 1) k (sk.stageScope (h + 1) k) none
              (sk.wiresBefore (h + 1) k) i feed)).filter P
            = (opEvents sk (.kid (h + 1) k (sk.stageScope (h + 1) k)
                (lastDOf sk (h + 1) k) (sk.wiresBefore (h + 1) k) i
                feed)).filter P := by
        intro i hi
        rw [List.mem_range] at hi
        have hIH := ih (sk.wiresBefore (h + 1) k + i)
        rw [opEventsE_kid_eq, opEvents_kid_eq]
        simp only [Nat.add_sub_cancel,
          show ((h + 1 : Nat) == 0) = false from rfl, Bool.false_eq_true,
          if_false]
        by_cases hD : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
        · rw [if_pos hD, if_pos hD]
          simp only [List.filter_cons]
          have hUdropK : ((if lastDOf sk (h + 1) k == some i
                then [((upperOut (wpk (h + 1)), true, k) : Ev)]
                else []).filter P) = [] := by
            split
            · rw [List.filter_cons_of_neg (by rw [hup (wpk (h + 1)) k]; exact Bool.false_ne_true),
                List.filter_nil]
            · rfl
          rw [List.filter_append, List.filter_append, List.filter_append,
            hUdropK, List.nil_append,
            hIH (chunkQ sk (h + 1) k i) hh' (hsub i hi)]
        · rw [if_neg hD, if_neg hD]
          simp only [List.filter_cons]
          rw [List.filter_append, List.filter_append,
            hIH [] hh' (hsub i hi)]
      rw [hEE, hED]
      have hmid : ((List.range
              (sk.nChildren (h + 1) (sk.stageScope (h + 1) k))).flatMap
              (fun i => opEventsE sk (.kid (h + 1) k
                (sk.stageScope (h + 1) k) none
                (sk.wiresBefore (h + 1) k) i feed))
            ++ [((upperOut (wpk (h + 1)), true, k) : Ev)]).filter P
          = ((if lastDOf sk (h + 1) k == none
              then [((upperOut (wpk (h + 1)), true, k) : Ev)] else [])
            ++ (List.range
              (sk.nChildren (h + 1) (sk.stageScope (h + 1) k))).flatMap
              (fun i => opEvents sk (.kid (h + 1) k
                (sk.stageScope (h + 1) k) (lastDOf sk (h + 1) k)
                (sk.wiresBefore (h + 1) k) i feed))).filter P := by
        rw [List.filter_append, List.filter_append, hUdrop,
          List.nil_append,
          List.filter_cons_of_neg (by
            rw [hup (wpk (h + 1)) k]; exact Bool.false_ne_true),
          List.filter_nil, List.append_nil]
        simp only [List.filter_flatMap]
        exact flatMap_congr hkid
      simp only [List.filter_cons]
      rw [hmid]

/-- The two opening futures agree on the dep-carrying class. -/
theorem weave_filter_isWA (hwf : sk.wellFormed = true) :
    ((weaveOps sk).flatMap (opEventsE sk)).filter isWA
      = ((weaveOps sk).flatMap (opEvents sk)).filter isWA := by
  have hge := (wf_rootH hwf).2
  have hlen1 := wf_stageLen_top sk hwf
  rw [weave_flatMapE, weave_flatMap, List.filter_append,
    List.filter_append, List.filter_append, List.filter_append,
    opEventsE_filter_scope sk hwf isWA (fun pk k => rfl)
      (sk.rootH - 1) 0 _ (by omega) (by omega)]

-- ==================================================== the payoff

/-- Dep-closure of the E opening future: transferred from the d5
future along the shared wire/asked subsequence. -/
theorem weaveE_flatMap_depOK (hwf : sk.wellFormed = true) :
    DepOK [] ((weaveOps sk).flatMap (opEventsE sk)) := by
  refine depOK_transfer isWA (fun e d hd => manDep_isWA e d hd)
    (weave_filter_isWA sk hwf) (weaveE_future_nodup sk hwf) ?_
  have hgo := goEvents_weave sk (weave_events_length sk hwf)
  rw [← hgo]
  exact weave_goEvents_depOK sk hwf

/-- `weaveE_goEvents_depOK` (PROGRESS.md §9, 2a-dep): dep-closure of
the eweave's ghost future — `weaveGoE_wedge`'s precedence input. -/
theorem weaveE_goEvents_depOK (hwf : sk.wellFormed = true) :
    DepOK [] (goEventsE sk (weaveFuel sk) (weaveOps sk)) := by
  rw [goEventsE_weave sk (weave_events_lengthE sk hwf)]
  exact weaveE_flatMap_depOK sk hwf

end StreamingMirror.Sched
