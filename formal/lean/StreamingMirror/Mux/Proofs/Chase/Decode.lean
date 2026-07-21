/-
The decode bridge (MUX-ADJUDICATION.md §3 T2): the base model's pending
layer, packaged for the muxed proofs.

Three groups:

- a `Decidable` instance and iff-forms for `Sched.performed`, so the
  muxed argmin can filter unperformed events;
- τ-comparison tools over `scheduleE`: trace order, E1, and E2 all
  strictly lower the timestamp (`merge_completeE` + numbering,
  consumed as black boxes);
- inversions of the pending pool: pool actions are never closes, and a
  pool action that is a wire fire decodes to a held stream — the two
  facts that let stuckness transfers (Chase/Ground) classify every
  enabled base action a pend can produce;
- the unified frontier decode `trace_frontier`: any trace is fully
  performed or splits at a pending event with performed prefix — the
  per-family `*_pend_or_doneE` lemmas assembled once, so the keystone's
  induction dispatches through a single door.
-/
import StreamingMirror.Mux.Proofs.Chase.Closure
import StreamingMirror.Proofs.EndgameE

namespace StreamingMirror.Mux

open Model
open Sched (Ev procsE scheduleE performed pends PendOkE evIdx proj canon)

variable {sk : Skel}

-- ================================================== performedness API

/-- Performedness is decidable: it is a count comparison. -/
instance decPerformed (sk : Skel) (s : State) (e : Ev) :
    Decidable (performed sk s e) := by
  unfold Sched.performed
  split <;> infer_instance

/-- A send is performed iff its seq is below the producer count. -/
theorem performed_snd_iff {s : State} {c : Chan} {n : Nat} :
    performed sk s (c, true, n) ↔ n < sentOf sk s c := by
  unfold Sched.performed
  simp

/-- A receive is performed iff its seq is below the consumer count. -/
theorem performed_rcv_iff {s : State} {c : Chan} {n : Nat} :
    performed sk s (c, false, n) ↔ n < recvdOf sk s c := by
  unfold Sched.performed
  simp

-- ==================================================== τ-comparisons

/-- The canonical-projection device: every projection of `scheduleE`
is canon at its own length (the progress body's `hcanon`). -/
theorem scheduleE_canon_self (hwf : sk.wellFormed = true) (c : Chan)
    (b : Bool) :
    proj c b (scheduleE sk) = canon c b (proj c b (scheduleE sk)).length := by
  obtain ⟨m, hm⟩ := Sched.scheduleE_proj_canon sk hwf c b
  rw [hm]
  congr 1
  unfold Sched.canon
  rw [List.length_map, List.length_range]

/-- Strict trace order is strict τ order: traces embed in `scheduleE`
in order, and the schedule never repeats an event. -/
theorem tau_lt_of_trace_pair (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) {T : List Ev}
    (hT : T ∈ procsE sk) {x y : Ev} (hxy : ([x, y]).Sublist T) :
    evIdx x (scheduleE sk) < evIdx y (scheduleE sk) :=
  Sched.pos_lt_of_pair (Sched.scheduleE_count_le_oneE sk hwf)
    (hxy.trans (Sched.trace_sublistE sk hwf hm0 hT))

/-- A scheduled receive's own send is scheduled strictly τ-below it. -/
theorem tau_e1 (hwf : sk.wellFormed = true) {c : Chan} {n : Nat}
    (hmem : ((c, false, n) : Ev) ∈ scheduleE sk) :
    ((c, true, n) : Ev) ∈ scheduleE sk
      ∧ evIdx ((c, true, n) : Ev) (scheduleE sk)
          < evIdx ((c, false, n) : Ev) (scheduleE sk) := by
  obtain ⟨j, hjlt, hjget⟩ := Sched.scheduleE_e1_pos sk hwf
    (evIdx ((c, false, n) : Ev) (scheduleE sk)) c n
    (Sched.evIdx_getElem? hmem)
  have hmem' : ((c, true, n) : Ev) ∈ scheduleE sk :=
    List.mem_iff_getElem?.2 ⟨j, hjget⟩
  have hjeq : j = evIdx ((c, true, n) : Ev) (scheduleE sk) :=
    Sched.evIdx_unique (Sched.scheduleE_count_le_oneE sk hwf _) hjget
  exact ⟨hmem', by omega⟩

/-- A scheduled send's cap-window receive is scheduled strictly τ-below
it, once past the free window. -/
theorem tau_e2 (hwf : sk.wellFormed = true) {c : Chan} {n : Nat}
    (hmem : ((c, true, n) : Ev) ∈ scheduleE sk)
    (hcap : sk.cap c ≤ n) :
    ((c, false, n - sk.cap c) : Ev) ∈ scheduleE sk
      ∧ evIdx ((c, false, n - sk.cap c) : Ev) (scheduleE sk)
          < evIdx ((c, true, n) : Ev) (scheduleE sk) := by
  have hE2 := Sched.scheduleE_e2 sk
    (evIdx ((c, true, n) : Ev) (scheduleE sk)) c n
    (Sched.evIdx_getElem? hmem)
  have hrcvlt : n - sk.cap c < Sched.rcvCount c
      ((scheduleE sk).take (evIdx ((c, true, n) : Ev) (scheduleE sk))) := by
    omega
  obtain ⟨j, hjlt, hjget⟩ :=
    Sched.mem_take_rcv (scheduleE_canon_self hwf c false) hrcvlt
  have hmem' : ((c, false, n - sk.cap c) : Ev) ∈ scheduleE sk :=
    List.mem_iff_getElem?.2 ⟨j, hjget⟩
  have hjeq : j = evIdx ((c, false, n - sk.cap c) : Ev) (scheduleE sk) :=
    Sched.evIdx_unique (Sched.scheduleE_count_le_oneE sk hwf _) hjget
  exact ⟨hmem', by omega⟩

/-- Inevitable events are scheduled: they live in some trace, and merge
completeness embeds every trace. -/
theorem inevitable_mem_scheduleE (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) {p : Party}
    {tr : List MObs} {e : Ev} (he : e ∈ inevitable sk p tr) :
    e ∈ scheduleE sk := by
  obtain ⟨T, hT, heT⟩ := mem_evUniv.mp (inevitable_subset_univ e he)
  exact (Sched.trace_sublistE sk hwf hm0 hT).mem heT

-- ============================================== pending-pool inversion

/-- Stage keys sit strictly under the root height. -/
theorem walkKeys_height_lt {pk : Party × Nat} (hpk : pk ∈ sk.walkKeys) :
    pk.2 < sk.rootH := by
  rw [Skel.walkKeys] at hpk
  rcases List.mem_append.1 hpk with h | h <;>
    · obtain ⟨k, hk, rfl⟩ := List.mem_map.1 h
      rw [List.mem_range] at hk
      simp only
      omega

/-- Pool actions are never wire closes: every pending action is a
channel operation, and closes are not channel operations. -/
theorem pends_not_close {s : State} {f : Ev} {a : Action}
    (hfa : (f, a) ∈ pends sk s) :
    (∀ pk, a ≠ .walkCloseWire pk) ∧ a ≠ .absorbCloseWire := by
  unfold Sched.pends at hfa
  rcases List.mem_append.1 hfa with hfa | hmem
  rcases List.mem_append.1 hfa with hfa | hmem
  rcases List.mem_append.1 hfa with hfa | hmem'
  rcases List.mem_append.1 hfa with hfa | hmem
  rcases List.mem_append.1 hfa with hfa | hmem''
  rcases List.mem_append.1 hfa with hmem | hmem
  · simp only [Sched.ioPend] at hmem
    repeat' split at hmem
    all_goals first
      | exact absurd hmem (List.not_mem_nil)
      | (simp only [List.mem_singleton, Prod.mk.injEq] at hmem
         obtain ⟨-, rfl⟩ := hmem
         exact ⟨fun _ hh => Action.noConfusion hh,
           fun hh => Action.noConfusion hh⟩)
  · simp only [Sched.roPend] at hmem
    repeat' split at hmem
    all_goals first
      | exact absurd hmem (List.not_mem_nil)
      | (simp only [List.mem_singleton, Prod.mk.injEq] at hmem
         obtain ⟨-, rfl⟩ := hmem
         exact ⟨fun _ hh => Action.noConfusion hh,
           fun hh => Action.noConfusion hh⟩)
  · obtain ⟨pk, hpk, hmem⟩ := List.mem_flatMap.1 hmem''
    simp only [Sched.wkPend] at hmem
    repeat' split at hmem
    all_goals first
      | exact absurd hmem (List.not_mem_nil)
      | (simp only [List.mem_singleton, Prod.mk.injEq] at hmem
         obtain ⟨-, rfl⟩ := hmem
         exact ⟨fun _ hh => Action.noConfusion hh,
           fun hh => Action.noConfusion hh⟩)
  · simp only [Sched.abPend] at hmem
    repeat' split at hmem
    all_goals first
      | exact absurd hmem (List.not_mem_nil)
      | (simp only [List.mem_singleton, Prod.mk.injEq] at hmem
         obtain ⟨-, rfl⟩ := hmem
         exact ⟨fun _ hh => Action.noConfusion hh,
           fun hh => Action.noConfusion hh⟩)
  · obtain ⟨pk, hpk, hmem⟩ := List.mem_flatMap.1 hmem'
    simp only [Sched.asmPend] at hmem
    repeat' split at hmem
    all_goals first
      | exact absurd hmem (List.not_mem_nil)
      | (simp only [List.mem_singleton, Prod.mk.injEq] at hmem
         obtain ⟨-, rfl⟩ := hmem
         exact ⟨fun _ hh => Action.noConfusion hh,
           fun hh => Action.noConfusion hh⟩)
  · simp only [Sched.rrPend] at hmem
    repeat' split at hmem
    all_goals first
      | exact absurd hmem (List.not_mem_nil)
      | (simp only [List.mem_singleton, Prod.mk.injEq] at hmem
         obtain ⟨-, rfl⟩ := hmem
         exact ⟨fun _ hh => Action.noConfusion hh,
           fun hh => Action.noConfusion hh⟩)
  · simp only [Sched.finPend] at hmem
    repeat' split at hmem
    all_goals first
      | exact absurd hmem (List.not_mem_nil)
      | (simp only [List.mem_singleton, Prod.mk.injEq] at hmem
         obtain ⟨-, rfl⟩ := hmem
         exact ⟨fun _ hh => Action.noConfusion hh,
           fun hh => Action.noConfusion hh⟩)

/-- A pending wire fire decodes to a held stream: the event is that
stream's send, and the hand is committed with the slot equation the
`push` guard wants (the chase's withheld-push extractor). -/
theorem pends_wireFire (hwf : sk.wellFormed = true) {s : State}
    {f : Ev} {a : Action} (hfa : (f, a) ∈ pends sk s)
    (hfire : isWireFire s a = true) :
    ∃ q hh, f.1 = Chan.wire q hh ∧ f.2.1 = true
      ∧ holdsWire sk q hh s = true := by
  unfold Sched.pends at hfa
  rcases List.mem_append.1 hfa with hfa | hmem
  rcases List.mem_append.1 hfa with hfa | hmem
  rcases List.mem_append.1 hfa with hfa | hmem'
  rcases List.mem_append.1 hfa with hfa | hmem
  rcases List.mem_append.1 hfa with hfa | hmem''
  rcases List.mem_append.1 hfa with hmem | hmem
  · -- initiator opening
    simp only [Sched.ioPend] at hmem
    split at hmem
    case _ hio =>
      simp only [List.mem_singleton, Prod.mk.injEq] at hmem
      obtain ⟨rfl, rfl⟩ := hmem
      refine ⟨.I, sk.rootH, rfl, rfl, ?_⟩
      rw [holdsWire.eq_def, if_pos (by simp)]
      simp [hio]
    case _ hio =>
      simp only [List.mem_singleton, Prod.mk.injEq] at hmem
      obtain ⟨-, rfl⟩ := hmem
      simp [isWireFire, hio] at hfire
    case _ => exact absurd hmem (List.not_mem_nil)
  · -- responder opening
    simp only [Sched.roPend] at hmem
    split at hmem
    · simp only [List.mem_singleton, Prod.mk.injEq] at hmem
      obtain ⟨-, rfl⟩ := hmem
      simp [isWireFire] at hfire
    · split at hmem
      case _ hro =>
        simp only [List.mem_singleton, Prod.mk.injEq] at hmem
        obtain ⟨rfl, rfl⟩ := hmem
        refine ⟨.R, sk.rootH, rfl, rfl, ?_⟩
        rw [holdsWire.eq_def, if_pos (by simp)]
        simp [hro]
      case _ hro =>
        simp only [List.mem_singleton, Prod.mk.injEq] at hmem
        obtain ⟨-, rfl⟩ := hmem
        simp [isWireFire, hro] at hfire
      case _ hro =>
        simp only [List.mem_singleton, Prod.mk.injEq] at hmem
        obtain ⟨-, rfl⟩ := hmem
        simp [isWireFire, hro] at hfire
      case _ => exact absurd hmem (List.not_mem_nil)
  · -- walks
    obtain ⟨pk, hpk, hmem⟩ := List.mem_flatMap.1 hmem''
    have hlt : pk.2 < sk.rootH := walkKeys_height_lt hpk
    simp only [Sched.wkPend] at hmem
    split at hmem
    · simp only [List.mem_singleton, Prod.mk.injEq] at hmem
      obtain ⟨-, rfl⟩ := hmem
      simp [isWireFire] at hfire
    · split at hmem
      · simp only [List.mem_singleton, Prod.mk.injEq] at hmem
        obtain ⟨-, rfl⟩ := hmem
        simp [isWireFire] at hfire
      · split at hmem
        case _ hph2 =>
          split at hmem
          case _ i hcm =>
            simp only [List.mem_singleton, Prod.mk.injEq] at hmem
            obtain ⟨rfl, rfl⟩ := hmem
            refine ⟨pk.1, pk.2, rfl, rfl, ?_⟩
            rw [holdsWire.eq_def, if_neg (by simp; omega)]
            simp only [Bool.and_eq_true]
            refine ⟨⟨(List.contains_iff_mem ..).mpr hpk, by simp [hph2]⟩, ?_⟩
            rw [hcm]
          case _ i hcm =>
            simp only [List.mem_singleton, Prod.mk.injEq] at hmem
            obtain ⟨-, rfl⟩ := hmem
            simp [isWireFire, hcm] at hfire
          case _ i hcm =>
            simp only [List.mem_singleton, Prod.mk.injEq] at hmem
            obtain ⟨-, rfl⟩ := hmem
            simp [isWireFire, hcm] at hfire
          case _ hcm =>
            simp only [List.mem_singleton, Prod.mk.injEq] at hmem
            obtain ⟨-, rfl⟩ := hmem
            simp [isWireFire, hcm] at hfire
          case _ => exact absurd hmem (List.not_mem_nil)
        · exact absurd hmem (List.not_mem_nil)
  · -- absorber
    simp only [Sched.abPend] at hmem
    repeat' split at hmem
    all_goals first
      | exact absurd hmem (List.not_mem_nil)
      | (simp only [List.mem_singleton, Prod.mk.injEq] at hmem
         obtain ⟨-, rfl⟩ := hmem
         simp [isWireFire] at hfire)
  · -- assemblers
    obtain ⟨pk, hpk, hmem⟩ := List.mem_flatMap.1 hmem'
    simp only [Sched.asmPend] at hmem
    repeat' split at hmem
    all_goals first
      | exact absurd hmem (List.not_mem_nil)
      | (simp only [List.mem_singleton, Prod.mk.injEq] at hmem
         obtain ⟨-, rfl⟩ := hmem
         simp [isWireFire] at hfire)
  · -- root return
    simp only [Sched.rrPend] at hmem
    repeat' split at hmem
    all_goals first
      | exact absurd hmem (List.not_mem_nil)
      | (simp only [List.mem_singleton, Prod.mk.injEq] at hmem
         obtain ⟨-, rfl⟩ := hmem
         simp [isWireFire] at hfire)
  · -- responder finish
    simp only [Sched.finPend] at hmem
    repeat' split at hmem
    all_goals first
      | exact absurd hmem (List.not_mem_nil)
      | (simp only [List.mem_singleton, Prod.mk.injEq] at hmem
         obtain ⟨-, rfl⟩ := hmem
         simp [isWireFire] at hfire)

-- =============================================== the frontier decode

/-- The unified frontier decode: every trace is fully performed, or
splits at a pool event whose prefix is performed (the per-family
`*_pend_or_doneE` lemmas behind one door, in `pends`-membership form). -/
theorem trace_frontier (hwf : sk.wellFormed = true) {s : State}
    (hL : InvL sk .impl s)
    (hioh : s.iopenCh = none → doneIOpen s = true)
    (hroh : s.ropenGotWire = true → s.ropenCh = none →
      doneROpen sk s = true)
    (hwkh : ∀ pk ∈ sk.walkKeys,
      ¬((s.walk pk).phase = 2 ∧ (s.walk pk).committed = none))
    {T : List Ev} (hT : T ∈ procsE sk) :
    (∀ e ∈ T, performed sk s e)
    ∨ ∃ f a pre suf, (f, a) ∈ pends sk s ∧ T = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performed sk s e) ∧ PendOkE sk s f a := by
  obtain ⟨hlio, hlro, hlwk, hlab, hlasm, hlrr, hlfin⟩ :=
    Sched.pends_lift sk (s := s)
  rcases Sched.procsE_cases sk hT with rfl | hc
  · rcases Sched.iopen_pend_or_doneE sk hwf hL hioh with ⟨hall, -⟩ | h
    · exact Or.inl hall
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      exact Or.inr ⟨f, a, pre, suf,
        hlio _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        hdec, hpre, hok⟩
  rcases hc with rfl | hc
  · rcases Sched.ropen_pend_or_doneE sk hwf hL hroh with ⟨hall, -⟩ | h
    · exact Or.inl hall
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      exact Or.inr ⟨f, a, pre, suf,
        hlro _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        hdec, hpre, hok⟩
  rcases hc with ⟨i, hir, rfl⟩ | hc
  · have hpk := Sched.walkOrder_mem_keys sk hwf hir
    rcases Sched.walk_pend_or_doneE sk hwf hL hpk (hwkh _ hpk) with
      ⟨hall, -⟩ | h
    · exact Or.inl hall
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      exact Or.inr ⟨f, a, pre, suf,
        hlwk _ hpk _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        hdec, hpre, hok⟩
  rcases hc with rfl | hc
  · rcases Sched.absorb_pend_or_doneE sk hwf hL with ⟨hall, -⟩ | h
    · exact Or.inl hall
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      exact Or.inr ⟨f, a, pre, suf,
        hlab _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        hdec, hpre, hok⟩
  rcases hc with ⟨pk, hpk, rfl⟩ | hc
  · rcases Sched.asm_pend_or_doneE sk hwf hL hpk with ⟨hall, -⟩ | h
    · exact Or.inl hall
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      exact Or.inr ⟨f, a, pre, suf,
        hlasm _ hpk _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        hdec, hpre, hok⟩
  rcases hc with rfl | rfl
  · rcases Sched.rootret_pend_or_doneE sk (s := s) with ⟨hall, -⟩ | h
    · exact Or.inl hall
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      exact Or.inr ⟨f, a, pre, suf,
        hlrr _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        hdec, hpre, hok⟩
  · rcases Sched.fin_pend_or_doneE sk hL with ⟨hall, -⟩ | h
    · exact Or.inl hall
    · obtain ⟨f, a, pre, suf, heq, hdec, hpre, hok⟩ := h
      exact Or.inr ⟨f, a, pre, suf,
        hlfin _ (by rw [heq]; exact List.mem_singleton.2 rfl),
        hdec, hpre, hok⟩

-- ============================================ trace-prefix extraction

/-- Lists satisfying the predicate throughout are their own
`takeWhile`. -/
theorem takeWhile_eq_self {α : Type _} {p : α → Bool} :
    ∀ {l : List α}, (∀ x ∈ l, p x = true) → l.takeWhile p = l
  | [], _ => rfl
  | a :: l, h => by
      rw [List.takeWhile_cons, if_pos (h a (List.mem_cons_self ..))]
      rw [takeWhile_eq_self fun x hx => h x (List.mem_cons_of_mem a hx)]

/-- In a duplicate-free split `pre ++ f :: suf`, the frontier `f` sits
inside the segment before any suffix event's first occurrence. -/
theorem frontier_mem_takeWhile {T pre suf : List Ev} {f e : Ev}
    (hdec : T = pre ++ f :: suf) (hcnt : T.count e ≤ 1) (he : e ∈ suf) :
    f ∈ T.takeWhile (fun x => !(x == e)) := by
  have hesuf : 1 ≤ suf.count e := List.one_le_count_iff.2 he
  have hsplit : T.count e = pre.count e + ((f :: suf).count e) := by
    rw [hdec, List.count_append]
  have hfs : (f :: suf).count e = suf.count e
      + (if (f == e) = true then 1 else 0) := List.count_cons ..
  have hnpre : e ∉ pre := by
    intro hmem
    have := List.one_le_count_iff.2 hmem
    omega
  have hfne : (f == e) = false := by
    cases hfe : (f == e) with
    | false => rfl
    | true =>
        rw [hfe] at hfs
        simp at hfs
        omega
  have hpre_all : ∀ x ∈ pre, (!(x == e)) = true := by
    intro x hx
    cases hxe : (x == e) with
    | false => rfl
    | true =>
        have : x = e := by simpa using hxe
        exact absurd (this ▸ hx) hnpre
  rw [hdec, List.takeWhile_append,
    if_pos (by rw [takeWhile_eq_self hpre_all])]
  rw [List.takeWhile_cons, if_pos (by rw [hfne]; rfl)]
  exact List.mem_append.mpr (.inr (List.mem_cons_self ..))

end StreamingMirror.Mux
