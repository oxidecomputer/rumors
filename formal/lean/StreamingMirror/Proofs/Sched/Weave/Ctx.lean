/-
Weave pump-progress, the position layer (PROGRESS.md §7 3b, step (f)
of the pump case-tree): what a walk's own trace position pins at a
pump-facing emission. The first brick is the splice-aware prefix
bound: when a walk's cell heads at its scope-`k` parent summary, its
emitted prefix already carries every resolution of the earlier scopes
— the §5 splice only ever ADDS the current scope's resolutions in
front of the parent. This is the own-walk component of the descent
supply (`DescSupply`'s top level); the cross-walk components (the
completed-subtree boundary memberships along the coverage telescope)
and the ascent coverage are the remaining CtxOK obligations, built by
the weave-order induction (see the design of record in PROGRESS.md).
-/
import StreamingMirror.Proofs.Sched.Weave.Window

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

/-- The prefix of a walk cut at its scope-`k` parent summary carries
the resolutions of all earlier scopes. -/
theorem walk_prefix_lower {pk : Party × Nat} {k : Nat}
    {pre rest : List Ev}
    (hsplit : walkEvents sk pk
      = pre ++ (upperOut pk, true, k) :: rest) :
    sk.dsBefore pk.2 k ≤ (proj (lowerOut pk) true pre).length := by
  unfold walkEvents at hsplit
  rw [List.range_eq_range'] at hsplit
  obtain ⟨t, -, htN, p₂, r₂, hblock, hr₂, hpre, hr⟩ :=
    prefix_flatMap _ 0 hsplit (by simp)
  rw [Nat.zero_add] at htN
  rw [Nat.sub_zero] at hpre
  -- the head is block t's sole parent event, so t = k
  have hmem : ((upperOut pk, true, k) : Ev) ∈ scopeBlock sk pk t := by
    rw [hblock]
    refine List.mem_append_right _ ?_
    cases r₂ with
    | nil => exact absurd rfl hr₂
    | cons x r₃ =>
        have hx : x = (upperOut pk, true, k) := by
          have := congrArg (fun l : List Ev => l[0]?) hr
          simpa using this.symm
        rw [hx]
        exact List.mem_cons_self ..
  have hmp : ((upperOut pk, true, k) : Ev)
      ∈ proj (upperOut pk) true (scopeBlock sk pk t) :=
    List.mem_filter.2 ⟨hmem, by simp⟩
  rw [proj_block_upper, seg_one] at hmp
  have htk : t = k := by
    have h := List.mem_singleton.1 hmp
    simpa using (congrArg (fun e : Ev => e.2.2) h).symm
  subst htk
  -- the closed blocks before t carry their full resolution segments
  have hrun : proj (lowerOut pk) true
      ((List.range t).flatMap (scopeBlock sk pk))
      = seg (lowerOut pk) true (sk.dsBefore pk.2 0)
          (sk.dsBefore pk.2 t - sk.dsBefore pk.2 0) :=
    proj_flatMap_seg t
      (fun i hi => proj_block_res sk pk (by omega))
      (fun i hi => by
        have := dsBefore_succ sk (h := pk.2) (k := i) (by omega)
        omega)
  rw [hpre, proj_append, List.length_append, ← List.range_eq_range',
    hrun, seg_len]
  have h0 : sk.dsBefore pk.2 0 = 0 := rfl
  omega
