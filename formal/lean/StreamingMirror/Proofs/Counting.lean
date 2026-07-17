/-
The counting layer of the progress lemma: full-sweep supply totals for
every walk-produced channel, derived from `wellFormed`.

Each starving consumer's blame lemma refutes "the producer is done" by
a supply/demand count: a stage's whole-sweep output on a channel equals
the whole-sweep demand of that channel's consumer. The equations all
reduce, through the BFS-alignment conjunct (`wf_bfs_aligned`), to
`List.length_flatMap` — a stage's kid lists, flattened, ARE the next
stage down — plus the per-scope facts that non-dispute scopes are
childless and carry no leaf requests.
-/
import StreamingMirror.Proofs.Lemmas

namespace StreamingMirror.Model

-- ===================================================== list helpers

/-- Restricting a sum to a filter loses nothing when the function
vanishes off the filter. -/
theorem sum_map_filter_of_zero {α : Type} {p : α → Bool} {f : α → Nat}
    {l : List α} (h : ∀ x ∈ l, p x = false → f x = 0) :
    ((l.filter p).map f).sum = (l.map f).sum := by
  induction l with
  | nil => simp
  | cons a t ih =>
      have ht : ∀ x ∈ t, p x = false → f x = 0 := fun x hx =>
        h x (List.mem_cons_of_mem a hx)
      cases hp : p a with
      | true => simp [hp, ih ht]
      | false =>
          have ha : f a = 0 := h a List.mem_cons_self hp
          simp [hp, ih ht, ha]

/-- Summing a positional lookup over `range l.length` is summing over
the list: the bridge from `qCount`-style index functions to list sums. -/
theorem sum_range_index {α : Type} (l : List α) (g : α → Nat) :
    ((List.range l.length).map (fun i =>
      match l[i]? with
      | some a => g a
      | none => 0)).sum = (l.map g).sum := by
  induction l with
  | nil => simp
  | cons a t ih =>
      rw [List.length_cons, List.range_succ_eq_map]
      simp only [List.map_cons, List.map_map, List.sum_cons]
      have hcomp : ∀ i ∈ List.range t.length,
          ((fun i => match (a :: t)[i]? with
            | some x => g x
            | none => 0) ∘ (· + 1)) i =
          (fun i => match t[i]? with
            | some x => g x
            | none => 0) i := by
        intro i _
        simp [Function.comp]
      rw [List.map_congr_left hcomp, ih]
      simp

-- ======================================== well-formedness extraction

/-- Non-dispute scopes are childless and carry no leaf requests: the
`wellFormed` conjunct that lets stage sums range over D scopes only. -/
theorem wf_scope_nonD {sk : Skel} (hwf : sk.wellFormed = true)
    {i : Nat} (hi : i < sk.scopes.length)
    (hnd : (sk.scope i).kind ≠ Kind.D) :
    (sk.scope i).kids = [] ∧ (sk.scope i).leafReqs = 0 := by
  unfold Skel.wellFormed at hwf
  simp only [Bool.and_eq_true, List.all_eq_true, decide_eq_true_eq] at hwf
  have hbody := hwf.1.1.1.1.1.2 i (List.mem_range.mpr hi)
  have hkind := hbody.1.1.1.2
  rw [Bool.or_eq_true] at hkind
  rcases hkind with hd | hempty
  · exact absurd (by simpa using hd) hnd
  · rw [Bool.and_eq_true, List.isEmpty_iff, beq_iff_eq] at hempty
    exact hempty

/-- Membership in a height slice: a real id at that height. -/
theorem mem_scopesAt {sk : Skel} {h s : Nat} (hs : s ∈ sk.scopesAt h) :
    s < sk.scopes.length ∧ (sk.scope s).height = h := by
  unfold Skel.scopesAt at hs
  rw [List.mem_filter, List.mem_range] at hs
  exact ⟨hs.1, by simpa using hs.2⟩

-- ==================================================== full-sweep sums

/-- Interior wire supply = demand: the wires stage `h + 1` emits over
its whole sweep are one per scope of stage `h` (its consumer consumes
exactly one per scope). -/
theorem wiresBefore_full {sk : Skel} (hwf : sk.wellFormed = true)
    {h : Nat} (hr : h + 1 < sk.rootH) :
    sk.wiresBefore (h + 1) (sk.stageLen (h + 1)) = sk.stageLen h := by
  have halign := wf_bfs_aligned hwf hr
  unfold Skel.wiresBefore Skel.stageLen Skel.stageScopes
  rw [List.take_length, foldl_add_eq_sum, Nat.zero_add]
  have hmap : ∀ s ∈ sk.scopesAt (h + 1 + 1),
      sk.nChildren (h + 1) s = (sk.scope s).kids.length := by
    intro s _
    unfold Skel.nChildren
    simp
  rw [List.map_congr_left hmap, ← List.length_flatMap, halign]

/-- Leaf wire supply = demand: the wires stage 0 emits over its whole
sweep are one per leaf request, the absorber's whole-sweep demand. -/
theorem wiresBefore_full_leaf {sk : Skel} (hwf : sk.wellFormed = true) :
    sk.wiresBefore 0 (sk.stageLen 0) = sk.totalLeafReqs := by
  unfold Skel.wiresBefore Skel.stageLen Skel.stageScopes
    Skel.totalLeafReqs
  rw [List.take_length, foldl_add_eq_sum, foldl_add_eq_sum]
  simp only [Nat.zero_add]
  have hmap : ∀ s ∈ sk.scopesAt (0 + 1),
      sk.nChildren 0 s = (sk.scope s).leafReqs := by
    intro s _
    unfold Skel.nChildren
    simp
  rw [List.map_congr_left hmap]
  refine (sum_map_filter_of_zero ?_).symm
  intro s hs hnd
  simp only [beq_eq_false_iff_ne] at hnd
  exact (wf_scope_nonD hwf (mem_scopesAt hs).1 hnd).2

/-- Resolution supply = demand: the resolutions stage `h + 1` emits over
its whole sweep are one per D scope of its stage — the answerer-side
assembler's resolution list, positionally. -/
theorem dsBefore_full {sk : Skel} (hwf : sk.wellFormed = true)
    {h : Nat} (hr : h + 1 < sk.rootH) :
    sk.dsBefore (h + 1) (sk.stageLen (h + 1)) =
      ((sk.scopesAt (h + 1)).filter
        (fun s => (sk.scope s).kind == Kind.D)).length := by
  have halign := wf_bfs_aligned hwf hr
  unfold Skel.dsBefore Skel.stageLen Skel.stageScopes
  rw [List.take_length, foldl_add_eq_sum, Nat.zero_add]
  have hmap : ∀ s ∈ sk.scopesAt (h + 1 + 1),
      sk.dOf (h + 1) s =
        ((sk.scope s).kids.filter
          (fun k => (sk.scope k).kind == Kind.D)).length := by
    intro s _
    unfold Skel.dOf Skel.dCount
    simp
  rw [List.map_congr_left hmap, ← halign, List.filter_flatMap,
    List.length_flatMap]

/-- Sums of per-element sums flatten: the bridge from a stage's
per-scope query totals to one sum over the whole next stage. -/
theorem sum_map_sum_eq_sum_flatMap {α β : Type} (l : List α)
    (f : α → List β) (g : β → Nat) :
    (l.map (fun a => ((f a).map g).sum)).sum =
      ((l.flatMap f).map g).sum := by
  induction l with
  | nil => simp
  | cons a t ih => simp [List.flatMap_cons, ih]

/-- A guarded sum is the sum over the filter. -/
theorem sum_map_ite_filter {α : Type} (p : α → Bool) (f : α → Nat)
    (l : List α) :
    (l.map (fun a => if p a then f a else 0)).sum =
      ((l.filter p).map f).sum := by
  induction l with
  | nil => simp
  | cons a t ih =>
      cases hp : p a with
      | true => simp [hp, ih]
      | false => simp [hp, ih]

/-- `qOf` as a sum over the scope's kid list: one query per slot of
each D child (slots are the child's kids, or its leaf requests at
height 1). -/
theorem qOf_eq_sum {sk : Skel} {h : Nat} (s : Nat)
    (hh : (h == 0) = false) :
    sk.qOf h s = ((sk.scope s).kids.map (fun k =>
      if (sk.scope k).kind == Kind.D then
        (if (sk.scope k).height == 1 then (sk.scope k).leafReqs
         else (sk.scope k).kids.length)
      else 0)).sum := by
  unfold Skel.qOf
  rw [foldl_add_eq_sum]
  simp only [Nat.zero_add]
  have hn : sk.nChildren h s = (sk.scope s).kids.length := by
    unfold Skel.nChildren
    simp [hh]
  rw [hn, ← sum_range_index ((sk.scope s).kids) _]
  congr 1
  apply List.map_congr_left
  intro i _
  unfold Skel.qCount Skel.childIsD
  cases hk : (sk.scope s).kids[i]? with
  | none => simp [hh]
  | some k =>
      cases hkind : (sk.scope k).kind == Kind.D with
      | true => simp [hh, hkind]
      | false => simp [hh, hkind]

/-- Interior query supply = demand: the queries stage `h + 2` launches
over its whole sweep are one per scope of stage `h`, its consumer's
whole-sweep demand (one asked per scope, two stages below). -/
theorem qsBefore_full {sk : Skel} (hwf : sk.wellFormed = true)
    {h : Nat} (hr : h + 2 < sk.rootH) :
    sk.qsBefore (h + 2) (sk.stageLen (h + 2)) = sk.stageLen h := by
  have halign2 := wf_bfs_aligned hwf hr
  have halign1 := wf_bfs_aligned hwf (by omega : h + 1 < sk.rootH)
  unfold Skel.qsBefore Skel.stageLen Skel.stageScopes
  rw [List.take_length, foldl_add_eq_sum]
  simp only [Nat.zero_add]
  have hmap : ∀ s ∈ sk.scopesAt (h + 2 + 1),
      sk.qOf (h + 2) s = ((sk.scope s).kids.map (fun k =>
        if (sk.scope k).kind == Kind.D then
          (if (sk.scope k).height == 1 then (sk.scope k).leafReqs
           else (sk.scope k).kids.length)
        else 0)).sum := fun s _ => qOf_eq_sum s (by simp)
  rw [List.map_congr_left hmap, sum_map_sum_eq_sum_flatMap, halign2]
  -- Scopes at height h + 2 are never height 1, and non-D scopes are
  -- childless, so the guarded slot count is just the kid count.
  have hpt : ∀ k ∈ sk.scopesAt (h + 2),
      (if (sk.scope k).kind == Kind.D then
        (if (sk.scope k).height == 1 then (sk.scope k).leafReqs
         else (sk.scope k).kids.length)
      else 0) = (sk.scope k).kids.length := by
    intro k hk
    obtain ⟨hlt, hht⟩ := mem_scopesAt hk
    have hne1 : ((sk.scope k).height == 1) = false := by
      simp [hht]
    rw [hne1]
    cases hkind : (sk.scope k).kind == Kind.D with
    | true => simp
    | false =>
        have hnd : (sk.scope k).kind ≠ Kind.D := by
          simpa using hkind
        simp [(wf_scope_nonD hwf hlt hnd).1]
  rw [List.map_congr_left hpt, ← List.length_flatMap, halign1]

/-- Leaf query supply = demand: the queries stage 1 launches over its
whole sweep are one per leaf request — the absorber's whole-sweep
demand on the leaf-request channel. -/
theorem qsBefore_full_leaf {sk : Skel} (hwf : sk.wellFormed = true) :
    sk.qsBefore 1 (sk.stageLen 1) = sk.totalLeafReqs := by
  have hge : 2 ≤ sk.rootH := (wf_rootH hwf).2
  have halign := wf_bfs_aligned hwf (by omega : 1 < sk.rootH)
  unfold Skel.qsBefore Skel.stageLen Skel.stageScopes Skel.totalLeafReqs
  rw [List.take_length, foldl_add_eq_sum, foldl_add_eq_sum]
  simp only [Nat.zero_add]
  have hmap : ∀ s ∈ sk.scopesAt (1 + 1),
      sk.qOf 1 s = ((sk.scope s).kids.map (fun k =>
        if (sk.scope k).kind == Kind.D then
          (if (sk.scope k).height == 1 then (sk.scope k).leafReqs
           else (sk.scope k).kids.length)
        else 0)).sum := fun s _ => qOf_eq_sum s (by simp)
  rw [List.map_congr_left hmap, sum_map_sum_eq_sum_flatMap, halign]
  have hpt : ∀ k ∈ sk.scopesAt 1,
      (if (sk.scope k).kind == Kind.D then
        (if (sk.scope k).height == 1 then (sk.scope k).leafReqs
         else (sk.scope k).kids.length)
      else 0) =
      (if (sk.scope k).kind == Kind.D then (sk.scope k).leafReqs
       else 0) := by
    intro k hk
    have hht := (mem_scopesAt hk).2
    simp [hht]
  rw [List.map_congr_left hpt, sum_map_ite_filter]

/-- Folding `+` over a Nat list is its sum (the `pendsBefore` shape). -/
theorem foldl_add_sum (l : List Nat) : l.foldl (· + ·) 0 = l.sum := by
  rw [foldl_add_eq_sum (fun x => x) l 0, Nat.zero_add, List.map_id']

/-- Asker-side level demand = answerer supply below: the level returns
an asker assembler at height `j + 1` consumes over its whole sweep are
one per D scope at height `j` — exactly the answerer assembler's
whole-sweep output below it. -/
theorem pendsBefore_asker_full {sk : Skel} (hwf : sk.wellFormed = true)
    {p : Party} {j : Nat} (hasks : asks p (j + 1) = true)
    (hr : j < sk.rootH) :
    sk.pendsBefore p (j + 1) (sk.asmResList p (j + 1)).length =
      ((sk.scopesAt j).filter
        (fun s => (sk.scope s).kind == Kind.D)).length := by
  have halign := wf_bfs_aligned hwf hr
  unfold Skel.pendsBefore Skel.asmResList
  rw [if_pos hasks, List.take_length, foldl_add_sum, ← halign,
    List.filter_flatMap, List.length_flatMap]
  rfl

/-- Answerer-side level demand at interior heights = asker supply
below: the level returns an answerer assembler at height `j + 2`
consumes over its whole sweep are one per scope at height `j + 1` —
the asker assembler's whole-sweep output below it. -/
theorem pendsBefore_answerer_full {sk : Skel} (hwf : sk.wellFormed = true)
    {p : Party} {j : Nat} (hna : asks p (j + 2) = false)
    (hr : j + 1 < sk.rootH) :
    sk.pendsBefore p (j + 2) (sk.asmResList p (j + 2)).length =
      sk.stageLen j := by
  have halign := wf_bfs_aligned hwf hr
  have hcond : ¬ asks p (j + 2) = true := by simp [hna]
  unfold Skel.pendsBefore Skel.asmResList Skel.stageLen Skel.stageScopes
  rw [if_neg hcond, List.take_length, foldl_add_sum]
  have hpt : ∀ s ∈ (sk.scopesAt (j + 2)).filter
      (fun s => (sk.scope s).kind == Kind.D),
      (if (sk.scope s).height == 1 then (sk.scope s).leafReqs
       else (sk.scope s).kids.length) = (sk.scope s).kids.length := by
    intro s hs
    have hht := (mem_scopesAt (List.mem_filter.mp hs).1).2
    simp [hht]
  have hzero : ∀ s ∈ sk.scopesAt (j + 2),
      (fun s => (sk.scope s).kind == Kind.D) s = false →
      (fun s => (sk.scope s).kids.length) s = 0 := by
    intro s hs hnd
    simp only [beq_eq_false_iff_ne] at hnd
    simp [(wf_scope_nonD hwf (mem_scopesAt hs).1 hnd).1]
  rw [List.map_congr_left hpt, sum_map_filter_of_zero hzero,
    ← List.length_flatMap, halign]

/-- Answerer-side level demand at height 1 = the absorber's whole-sweep
output: one level return per leaf request. -/
theorem pendsBefore_answerer_leaf {sk : Skel}
    {p : Party} (hna : asks p 1 = false) :
    sk.pendsBefore p 1 (sk.asmResList p 1).length =
      sk.totalLeafReqs := by
  have hcond : ¬ asks p 1 = true := by simp [hna]
  unfold Skel.pendsBefore Skel.asmResList Skel.totalLeafReqs
  rw [if_neg hcond, List.take_length, foldl_add_sum, foldl_add_eq_sum]
  simp only [Nat.zero_add]
  have hpt : ∀ s ∈ (sk.scopesAt 1).filter
      (fun s => (sk.scope s).kind == Kind.D),
      (if (sk.scope s).height == 1 then (sk.scope s).leafReqs
       else (sk.scope s).kids.length) = (sk.scope s).leafReqs := by
    intro s hs
    have hht := (mem_scopesAt (List.mem_filter.mp hs).1).2
    simp [hht]
  rw [List.map_congr_left hpt]

-- ================================================== cursor accounting
-- The per-cursor forms of the identities above: what an assembler's
-- pending prefix sum means mid-sweep, in the walk layer's own
-- coordinates (`dsBefore`/`wiresBefore`). The full-sweep totals are
-- the `k = length` instances.

/-- A prefix's fold never exceeds the whole's (Nat addition). -/
theorem foldl_add_take_le (l : List Nat) (k : Nat) :
    (l.take k).foldl (· + ·) 0 ≤ l.foldl (· + ·) 0 := by
  have hsplit : l.sum = (l.take k).sum + (l.drop k).sum := by
    rw [← List.sum_append, List.take_append_drop]
  rw [foldl_add_sum, foldl_add_sum, hsplit]
  omega

/-- No scope lives at height zero. -/
theorem wf_scopesAt_zero {sk : Skel} (hwf : sk.wellFormed = true) :
    sk.scopesAt 0 = [] := by
  unfold Skel.scopesAt
  rw [List.filter_eq_nil_iff]
  intro i hi
  unfold Skel.wellFormed at hwf
  simp only [Bool.and_eq_true, List.all_eq_true, decide_eq_true_eq]
    at hwf
  have hge1 := (hwf.1.1.1.1.1.2 i hi).1.1.1.1.1.1
  simp only [beq_iff_eq]
  omega

/-- Asker resolutions enumerate the stage's scopes. -/
theorem asmResList_asker_length {sk : Skel} {p : Party} {j : Nat}
    (hasks : asks p j = true) :
    (sk.asmResList p j).length = (sk.scopesAt j).length := by
  unfold Skel.asmResList
  rw [if_pos hasks, List.length_map]

/-- Answerer resolutions enumerate the stage's D scopes. -/
theorem asmResList_answerer_length {sk : Skel} {p : Party} {j : Nat}
    (hna : asks p j = false) :
    (sk.asmResList p j).length
      = ((sk.scopesAt j).filter
          (fun s => (sk.scope s).kind == Kind.D)).length := by
  unfold Skel.asmResList
  rw [if_neg (by simp [hna]), List.length_map]

/-- Asker-side level demand at any cursor is the stage's own D prefix
sum: the level returns an asker assembler needs through its first `k`
resolutions are one per D child of the first `k` scopes — `dsBefore`,
the coordinate the walk's resolution seqs are minted in. -/
theorem pendsBefore_asker {sk : Skel} {p : Party} {j : Nat}
    (hasks : asks p j = true) (h2 : 2 ≤ j) (k : Nat) :
    sk.pendsBefore p j k = sk.dsBefore (j - 1) k := by
  unfold Skel.pendsBefore Skel.asmResList Skel.dsBefore
    Skel.stageScopes
  rw [if_pos hasks, ← List.map_take, foldl_add_sum, foldl_add_eq_sum,
    Nat.zero_add, show j - 1 + 1 = j from by omega]
  congr 1
  apply List.map_congr_left
  intro s _
  unfold Skel.dOf
  rw [if_neg (by simp; omega)]

/-- At height 1 the asker has nothing pending: height-1 scopes are
childless (their kids would live at height 0), so every resolution's
pending count is zero. -/
theorem pendsBefore_asker_one {sk : Skel} (hwf : sk.wellFormed = true)
    {p : Party} (hasks : asks p 1 = true) (k : Nat) :
    sk.pendsBefore p 1 k = 0 := by
  have h0 : 0 < sk.rootH := by have := wf_rootH hwf; omega
  have htot : sk.pendsBefore p 1 (sk.asmResList p 1).length
      = ((sk.scopesAt 0).filter
          (fun s => (sk.scope s).kind == Kind.D)).length :=
    pendsBefore_asker_full hwf (p := p) (j := 0) hasks h0
  rw [wf_scopesAt_zero hwf] at htot
  simp only [List.filter_nil, List.length_nil] at htot
  unfold Skel.pendsBefore at htot ⊢
  rw [List.take_length] at htot
  have hle := foldl_add_take_le (sk.asmResList p 1) k
  omega

/-- Answerer-side level demand at any D-scope cursor, bridged one stage
down: the pends of the first resolutions covering the stage's first `K`
scopes (only their D scopes carry resolutions) are the kid-stage wires
of those `K` scopes — `wiresBefore`, the coordinate the producing
assembler's outputs are minted in. Non-D scopes are childless, so the
filter drops only zeros. -/
theorem pendsBefore_answerer {sk : Skel} (hwf : sk.wellFormed = true)
    {p : Party} {j : Nat} (hna : asks p j = false) (h1 : 1 ≤ j)
    (K : Nat) :
    sk.pendsBefore p j
        (((sk.scopesAt j).take K).filter
          (fun s => (sk.scope s).kind == Kind.D)).length
      = sk.wiresBefore (j - 1) K := by
  have hcond : ¬ asks p j = true := by simp [hna]
  unfold Skel.pendsBefore Skel.asmResList Skel.wiresBefore
    Skel.stageScopes
  rw [if_neg hcond, show j - 1 + 1 = j from by omega]
  -- the taken filter is a prefix of the whole filter, cut at its
  -- own length: taking that many resolutions is filtering the taken
  -- stage prefix
  have hcut : ((sk.scopesAt j).filter
        (fun s => (sk.scope s).kind == Kind.D)).take
        (((sk.scopesAt j).take K).filter
          (fun s => (sk.scope s).kind == Kind.D)).length
      = ((sk.scopesAt j).take K).filter
          (fun s => (sk.scope s).kind == Kind.D) :=
    (List.prefix_iff_eq_take.1
      ((List.take_prefix K (sk.scopesAt j)).filter _)).symm
  rw [← List.map_take, hcut, foldl_add_sum, foldl_add_eq_sum,
    Nat.zero_add]
  -- on D scopes the resolution entry IS the kid count; off D it is 0
  have hpt : ∀ s ∈ ((sk.scopesAt j).take K).filter
      (fun s => (sk.scope s).kind == Kind.D),
      (if (sk.scope s).height == 1 then (sk.scope s).leafReqs
       else (sk.scope s).kids.length) = sk.nChildren (j - 1) s := by
    intro s hs
    have hht := (mem_scopesAt
      (List.mem_of_mem_take (List.mem_filter.mp hs).1)).2
    unfold Skel.nChildren
    by_cases hj1 : j = 1
    · rw [if_pos (by simp [hht, hj1]), if_pos (by simp [hj1])]
    · rw [if_neg (by simp [hht]; omega), if_neg (by simp; omega)]
  have hzero : ∀ s ∈ (sk.scopesAt j).take K,
      (fun s => (sk.scope s).kind == Kind.D) s = false →
      (fun s => sk.nChildren (j - 1) s) s = 0 := by
    intro s hs hnd
    simp only [beq_eq_false_iff_ne] at hnd
    have hnonD := wf_scope_nonD hwf
      (mem_scopesAt (List.mem_of_mem_take hs)).1 hnd
    unfold Skel.nChildren
    split
    · exact hnonD.2
    · show (sk.scope s).kids.length = 0
      rw [hnonD.1]
      rfl
  rw [List.map_congr_left hpt, sum_map_filter_of_zero hzero]

/-- Taking a block-boundary prefix of a flattening is flattening the
taken blocks. -/
theorem take_flatMap_blocks {α β : Type} (f : α → List β) :
    ∀ (l : List α) (K : Nat),
      (l.flatMap f).take (((l.take K).map fun a => (f a).length).sum)
        = (l.take K).flatMap f
  | _, 0 => by simp
  | [], _ + 1 => by simp
  | a :: l, K + 1 => by
      simp only [List.take_succ_cons, List.map_cons, List.sum_cons,
        List.flatMap_cons]
      rw [List.take_append,
        List.take_of_length_le (by omega : (f a).length
          ≤ (f a).length + ((List.take K l).map
              fun a => (f a).length).sum),
        Nat.add_sub_cancel_left, take_flatMap_blocks f l K]

/-- The stage's D prefix sum, read one stage down: the D scopes among
the kids of the first `K` scopes are `dsBefore` — the walk's own
resolution-seq coordinate meets the kid stage's scope order. -/
theorem ds_wires {sk : Skel} (hwf : sk.wellFormed = true)
    {j : Nat} (h1 : 1 ≤ j) (hjr : j < sk.rootH) (K : Nat) :
    (((sk.scopesAt j).take (sk.wiresBefore j K)).filter
      (fun s => (sk.scope s).kind == Kind.D)).length
      = sk.dsBefore j K := by
  have halign := wf_bfs_aligned hwf hjr
  have hwires : sk.wiresBefore j K
      = (((sk.stageScopes j).take K).map
          fun s => ((sk.scope s).kids).length).sum := by
    unfold Skel.wiresBefore
    rw [foldl_add_eq_sum, Nat.zero_add]
    congr 1
    apply List.map_congr_left
    intro s _
    unfold Skel.nChildren
    rw [if_neg (by simp; omega)]
  have htake : (sk.scopesAt j).take (sk.wiresBefore j K)
      = ((sk.stageScopes j).take K).flatMap
          fun s => (sk.scope s).kids := by
    rw [← halign, hwires]
    exact take_flatMap_blocks _ (sk.stageScopes j) K
  rw [htake, List.filter_flatMap, List.length_flatMap]
  unfold Skel.dsBefore
  rw [foldl_add_eq_sum, Nat.zero_add]
  congr 1
  apply List.map_congr_left
  intro s _
  unfold Skel.dOf Skel.dCount
  rw [if_neg (by simp; omega)]

/-- The descent telescope's answerer step: pends through the covered D
scopes are the kid-stage wires of the covered kids. -/
theorem pendsBefore_answerer_ds {sk : Skel} (hwf : sk.wellFormed = true)
    {p : Party} {j : Nat} (hna : asks p j = false) (h1 : 1 ≤ j)
    (hjr : j < sk.rootH) (C : Nat) :
    sk.pendsBefore p j (sk.dsBefore j C)
      = sk.wiresBefore (j - 1) (sk.wiresBefore j C) := by
  rw [← ds_wires hwf h1 hjr C]
  exact pendsBefore_answerer hwf hna h1 (sk.wiresBefore j C)

end StreamingMirror.Model
