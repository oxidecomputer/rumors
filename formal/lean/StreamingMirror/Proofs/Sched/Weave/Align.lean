/-
The weave/trace alignment (PROGRESS.md §7 3b): the opening worklist's
per-owner filters ARE the manual traces — the two hypotheses
`weaveState_wcount` still owes. This file carries the design of
record for that proof and its Skel-arithmetic base layer.

# The master induction (design of record)

For the subtree op `.scope h k F` (stage `h`, scope index `k`, feed
`F`), under `wellFormed`, with `k < stageLen h`,
`F.length = nChildren h (stageScope h k)`, and every `F` event owned
by one index `mF < walkIdx h`:

1. `filter (evOwner = walkIdx h') (opEvents (.scope h k F))`, for
   every `h' ≤ h`, is the contiguous run of stage-`h'` scope blocks
   descended from `(h, k)`: indices
   `[descIdx h' (h - h') k, descIdx h' (h - h') (k+1))`, where
   `descIdx` iterates `wiresBefore` down the stages. At `h' = h`
   that run is `scopeBlock (wpk h) k` itself — the query-feed
   mechanism appears here as the kid subtrees' feed filters (owner
   `walkIdx h`, the kid's own `mF`) splicing each chunk's queries
   back between the wires, reproducing `scopeSends`' §5 splice
   exactly.
2. `filter (evOwner = mF) = F` — the feed passes through, one query
   emitted before each kid's descent.
3. Every subtree event is owned by `mF` or by `walkIdx h'` for some
   `h' ≤ h` (so all other filters are empty, and `owners_lt` holds).

Induction on `h`: expand `opEvents` one scope layer (`opEvents_scope`
/`opEvents_kid`), apply the induction hypothesis to each kid's
`.scope (h-1) (kidBase + i) myQ` — its clause 2 returns `myQ`, its
clause 1 returns the kid blocks — and glue the sibling runs with
`range'` segment algebra. The top assembly then instantiates
`(h, k, F) = (rootH - 1, 0, ropen's root queries)`: clause 1 at each
`h'` must cover the WHOLE stage (`descIdx … 0 = 0` and
`descIdx … 1 = stageLen h'`, the total-children telescope), clause 2
returns ropen's tail, and the openers' emits cover iopen and ropen's
head — together `manFilters = (procs).take manCount`, and clause 3
gives `owners_lt`; `goEvents_weave` then transfers both from the
fuel-free expansion to the interpreter's futures.

# This file so far: the base layer

The BFS facts the induction consumes, extracted from `wellFormed`'s
checkable promise (`wf_bfs_aligned`) and the per-scope pass:

- `kidBase_eq_wiresBefore`: the interpreter's inline kid base is the
  `Skel` prefix sum.
- `wiresBefore_total`: a stage's total children count the next stage
  down — the telescope's closing step.
- `stageScope_kid`: the `i`-th kid of stage scope `(h, k)` IS stage
  `h-1`'s scope `wiresBefore h k + i` — cross-parent kid alignment,
  positionally.
- `qCount_eq_kid_nChildren`: the queries chunk `i` owes are exactly
  the kid scope's child count — the feed the kid needs is the feed
  it gets.
-/
import StreamingMirror.Proofs.Sched.Weave.Expand
import StreamingMirror.Proofs.Counting

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ================================================ statement vocabulary

/-- The walk key of stage `h` (initiator on odd stages), as the weave
and `procs` both spell it. -/
def wpk (h : Nat) : Party × Nat :=
  (if h % 2 == 1 then Party.I else Party.R, h)

/-- A stage's walk never asks at its own stage: the walk is the
answerer side of its own summaries. -/
theorem asks_wpk_self (h : Nat) : asks (wpk h).1 h = false := by
  rcases Nat.mod_two_eq_zero_or_one h with hm | hm <;>
    simp [wpk, asks, hm]

/-- The answerer party at a stage is the stage's own walk key. -/
theorem wpk_fst_of_answerer {p : Party} {g : Nat}
    (hna : asks p g = false) : (wpk g).1 = p := by
  cases p <;>
    rcases Nat.mod_two_eq_zero_or_one g with hm | hm <;>
    simp [asks, hm] at hna <;>
    simp [wpk, hm]

/-- A contiguous run of stage-`h'` scope blocks: trace segment
`[a, b)` of the stage's walk. -/
def walkSeg (h' a b : Nat) : List Ev :=
  (List.range' a (b - a)).flatMap (scopeBlock sk (wpk h'))

/-- Iterated descent of a stage index: `descIdx h' d j` maps scope
index `j` at stage `h' + d` to the index of its subtree's first
scope at stage `h'`, one `wiresBefore` per stage. -/
def descIdx (h' : Nat) : Nat → Nat → Nat
  | 0, j => j
  | d + 1, j => descIdx h' d (sk.wiresBefore (h' + d + 1) j)

-- ========================================== wellFormed extraction

/-- A stage scope's height is its stage plus one (it is a member of
`scopesAt (h+1)` by construction). -/
theorem stageScope_height {h k : Nat} (hk : k < sk.stageLen h) :
    (sk.scope (sk.stageScope h k)).height = h + 1 := by
  have hmem : sk.stageScope h k ∈ sk.stageScopes h := by
    unfold Skel.stageScope
    unfold Skel.stageLen at hk
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hk,
      Option.getD_some]
    exact List.getElem_mem hk
  unfold Skel.stageScopes Skel.scopesAt at hmem
  rw [List.mem_filter] at hmem
  simpa using hmem.2

/-- The kids-fold of `wellFormed`'s per-scope pass, read pointwise:
every checked kid passed its own conjuncts (the ascending-chain part
is dropped; range and height are what the alignment needs). -/
private theorem kids_fold_facts {n hgt : Nat} :
    ∀ (kids : List Nat) (a : Nat) (b : Bool),
      ((kids.foldl (fun (acc : Nat × Bool) k =>
          (k, acc.2 && decide (k > acc.1) && decide (k < n) &&
              ((sk.scope k).height == hgt))) (a, b)).2 = true) →
      b = true ∧ ∀ k ∈ kids, k < n ∧ (sk.scope k).height = hgt := by
  intro kids
  induction kids with
  | nil =>
      intro a b h
      exact ⟨h, fun k hk => absurd hk (by simp)⟩
  | cons k' l ih =>
      intro a b h
      rw [List.foldl_cons] at h
      obtain ⟨hb, hall⟩ := ih k' _ h
      simp only [Bool.and_eq_true, decide_eq_true_eq, beq_iff_eq] at hb
      refine ⟨hb.1.1.1, fun k hk => ?_⟩
      rcases List.mem_cons.1 hk with rfl | hk'
      · exact ⟨hb.1.2, hb.2⟩
      · exact hall k hk'

/-- Kid range and height, extracted: every kid of a real scope is a
real scope one height down. -/
theorem wf_kid_facts {sk : Skel} (hwf : sk.wellFormed = true)
    {j : Nat} (hj : j < sk.scopes.length) :
    ∀ k ∈ (sk.scope j).kids,
      k < sk.scopes.length
        ∧ (sk.scope k).height = (sk.scope j).height - 1 := by
  unfold Skel.wellFormed at hwf
  simp only [Bool.and_eq_true, List.all_eq_true, decide_eq_true_eq,
    beq_iff_eq] at hwf
  have hper := hwf.1.1.1.1.1.2
  have hfold := (hper j (List.mem_range.mpr hj)).2
  exact (kids_fold_facts sk _ _ _ hfold).2

/-- Non-D scopes are childless and request nothing, extracted. -/
theorem wf_scope_notD {sk : Skel} (hwf : sk.wellFormed = true)
    {j : Nat} (hj : j < sk.scopes.length)
    (hnd : (sk.scope j).kind ≠ Kind.D) :
    (sk.scope j).kids = [] ∧ (sk.scope j).leafReqs = 0 := by
  unfold Skel.wellFormed at hwf
  simp only [Bool.and_eq_true, List.all_eq_true, decide_eq_true_eq,
    beq_iff_eq] at hwf
  have hper := hwf.1.1.1.1.1.2
  have hcond := (hper j (List.mem_range.mpr hj)).1.1.1.2
  simp only [Bool.or_eq_true, Bool.and_eq_true, beq_iff_eq,
    List.isEmpty_iff] at hcond
  rcases hcond with hD | h
  · exact absurd hD hnd
  · exact h

-- ================================================== prefix-sum bridges

/-- Above the leaf stage, a scope's stage children are its kids. -/
theorem nChildren_of_pos {h : Nat} (h1 : 1 ≤ h) (s : Nat) :
    sk.nChildren h s = (sk.scope s).kids.length := by
  unfold Skel.nChildren
  rw [if_neg]
  simp only [beq_iff_eq]
  omega

/-- The interpreter's inline kid base is the `Skel` prefix sum: the
`range` fold over stage cursors equals `wiresBefore`. -/
theorem kidBase_eq_wiresBefore (h : Nat) :
    ∀ k, k ≤ sk.stageLen h →
      (List.range k).foldl
        (fun a k' => a + sk.nChildren h (sk.stageScope h k')) 0
        = sk.wiresBefore h k := by
  intro k
  induction k with
  | zero => intro _; rfl
  | succ k ih =>
      intro hk
      rw [List.range_succ, List.foldl_append, ih (by omega),
        wiresBefore_succ sk (by omega)]
      rfl

/-- `wiresBefore` as a mapped sum over the stage prefix. -/
private theorem wiresBefore_eq_sum (h k : Nat) :
    sk.wiresBefore h k
      = (((sk.stageScopes h).take k).map (sk.nChildren h)).sum := by
  unfold Skel.wiresBefore
  rw [foldl_add_eq_sum, Nat.zero_add]

/-- A stage's total children count the next stage down: the closing
step of the `descIdx` telescope, from the BFS promise. -/
theorem wiresBefore_total (hwf : sk.wellFormed = true) {h : Nat}
    (h1 : 1 ≤ h) (hh : h < sk.rootH) :
    sk.wiresBefore h (sk.stageLen h) = sk.stageLen (h - 1) := by
  have hbfs := wf_bfs_aligned hwf hh
  have hstage : sk.stageScopes (h - 1) = sk.scopesAt h := by
    unfold Skel.stageScopes
    congr 1
    omega
  rw [wiresBefore_eq_sum]
  unfold Skel.stageLen
  rw [List.take_length, hstage, ← hbfs, List.length_flatMap]
  unfold Skel.stageScopes
  congr 1
  refine List.map_congr_left fun s _ => ?_
  exact nChildren_of_pos sk h1 s

/-- Positional read of a flatMap: the `i`-th element of block `k`
sits at the blocks-before prefix sum plus `i`. -/
private theorem flatMap_getD_pos {α β : Type _} (g : α → List β)
    (dα : α) (d : β) :
    ∀ (l : List α) (k i : Nat), k < l.length →
      i < (g (l.getD k dα)).length →
      (l.flatMap g).getD
          ((((l.take k).map fun x => (g x).length).sum) + i) d
        = (g (l.getD k dα)).getD i d := by
  intro l
  induction l with
  | nil =>
      intro k i hk
      simp at hk
  | cons a l' ih =>
      intro k i hk hi
      cases k with
      | zero =>
          rw [List.getD_cons_zero] at hi ⊢
          simp only [List.take_zero, List.map_nil, List.sum_nil,
            Nat.zero_add, List.flatMap_cons]
          rw [List.getD_eq_getElem?_getD, List.getElem?_append_left hi,
            ← List.getD_eq_getElem?_getD]
      | succ k' =>
          rw [List.getD_cons_succ] at hi ⊢
          simp only [List.take_succ_cons, List.map_cons, List.sum_cons,
            List.flatMap_cons]
          rw [List.getD_eq_getElem?_getD,
            List.getElem?_append_right (by omega),
            Nat.add_assoc, Nat.add_sub_cancel_left,
            ← List.getD_eq_getElem?_getD]
          exact ih k' i (by simpa using hk) hi

/-- Cross-parent kid alignment, positionally: the `i`-th kid of stage
scope `(h, k)` IS the `wiresBefore h k + i`-th scope of stage
`h - 1`. This is the correspondence `wellFormed`'s BFS conjunct
exists to license. -/
theorem stageScope_kid (hwf : sk.wellFormed = true) {h k i : Nat}
    (h1 : 1 ≤ h) (hh : h < sk.rootH) (hk : k < sk.stageLen h)
    (hi : i < sk.nChildren h (sk.stageScope h k)) :
    sk.stageScope (h - 1) (sk.wiresBefore h k + i)
      = (sk.scope (sk.stageScope h k)).kids.getD i 0 := by
  have hbfs := wf_bfs_aligned hwf hh
  have hstage : sk.stageScopes (h - 1) = sk.scopesAt h := by
    unfold Skel.stageScopes
    congr 1
    omega
  have hgl : ∀ s ∈ (sk.stageScopes h).take k,
      sk.nChildren h s = ((sk.scope s).kids).length := fun s _ =>
    nChildren_of_pos sk h1 s
  have hkl : i < ((sk.scope ((sk.stageScopes h).getD k 0)).kids).length := by
    rw [← nChildren_of_pos sk h1]
    exact hi
  unfold Skel.stageScope
  rw [hstage, ← hbfs, wiresBefore_eq_sum, List.map_congr_left hgl]
  have := flatMap_getD_pos (fun s => (sk.scope s).kids) 0 0
    (sk.stageScopes h) k i (by unfold Skel.stageLen at hk; exact hk) hkl
  unfold Skel.stageScopes at this ⊢
  exact this

/-- The queries chunk `i` owes are the kid scope's child count: the
feed a kid subtree receives is exactly one query per grandchild, so
the master induction's length precondition self-propagates. -/
theorem qCount_eq_kid_nChildren (hwf : sk.wellFormed = true)
    {h k i : Nat} (h1 : 1 ≤ h) (hh : h < sk.rootH)
    (hk : k < sk.stageLen h)
    (hi : i < sk.nChildren h (sk.stageScope h k)) :
    sk.qCount h (sk.stageScope h k) i
      = sk.nChildren (h - 1)
          (sk.stageScope (h - 1) (sk.wiresBefore h k + i)) := by
  rw [stageScope_kid sk hwf h1 hh hk hi]
  have hkl : i < ((sk.scope (sk.stageScope h k)).kids).length := by
    rw [← nChildren_of_pos sk h1]
    exact hi
  have hsome : ((sk.scope (sk.stageScope h k)).kids)[i]?
      = some (((sk.scope (sk.stageScope h k)).kids).getD i 0) := by
    rw [List.getElem?_eq_getElem hkl, List.getD_eq_getElem?_getD,
      List.getElem?_eq_getElem hkl]
    rfl
  have hmem : ((sk.scope (sk.stageScope h k)).kids).getD i 0
      ∈ (sk.scope (sk.stageScope h k)).kids := by
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hkl]
    exact List.getElem_mem hkl
  obtain ⟨hcn, hch⟩ := wf_kid_facts hwf (stageScope_lt_scopes sk hk) _ hmem
  have hsh := stageScope_height sk hk
  rw [hsh] at hch
  have hch' : (sk.scope (((sk.scope (sk.stageScope h k)).kids).getD
      i 0)).height = h := by omega
  have hne : (h == 0) = false := by
    simp only [beq_eq_false_iff_ne, ne_eq]
    omega
  cases hknd : (sk.scope (((sk.scope (sk.stageScope h k)).kids).getD
      i 0)).kind with
  | D =>
      simp only [Skel.qCount, Skel.childIsD, Skel.nChildren, hne, hsome,
        hknd, hch', Bool.false_eq_true, if_false, beq_self_eq_true,
        Bool.not_true]
      by_cases h1' : h = 1
      · subst h1'
        simp
      · have hb1 : ((h : Nat) == 1) = false := by
          simp only [beq_eq_false_iff_ne, ne_eq]
          omega
        have hb0 : ((h - 1 : Nat) == 0) = false := by
          simp only [beq_eq_false_iff_ne, ne_eq]
          omega
        rw [hb1, hb0]
  | R =>
      have hnd : (sk.scope (((sk.scope (sk.stageScope h k)).kids).getD
          i 0)).kind ≠ Kind.D := by
        rw [hknd]
        intro hcon
        cases hcon
      obtain ⟨hkids, hleaf⟩ := wf_scope_notD hwf hcn hnd
      simp only [Skel.qCount, Skel.childIsD, Skel.nChildren, hne, hsome,
        hknd, hkids, hleaf, Bool.false_eq_true, if_false]
      by_cases h0 : h - 1 = 0
      · simp [h0]
      · have hb0 : ((h - 1 : Nat) == 0) = false := by
          simp only [beq_eq_false_iff_ne, ne_eq]
          omega
        simp [hb0]

/-- A slot's D flag is its kid scope's kind, read one stage down. -/
theorem childIsD_eq_kid_kind (hwf : sk.wellFormed = true) {h k i : Nat}
    (h1 : 1 ≤ h) (hh : h < sk.rootH) (hk : k < sk.stageLen h)
    (hi : i < sk.nChildren h (sk.stageScope h k)) :
    sk.childIsD h (sk.stageScope h k) i
      = ((sk.scope (sk.stageScope (h - 1)
            (sk.wiresBefore h k + i))).kind == Kind.D) := by
  rw [stageScope_kid sk hwf h1 hh hk hi]
  have hkl : i < ((sk.scope (sk.stageScope h k)).kids).length := by
    rw [← nChildren_of_pos sk h1]
    exact hi
  have hsome : ((sk.scope (sk.stageScope h k)).kids)[i]?
      = some (((sk.scope (sk.stageScope h k)).kids).getD i 0) := by
    rw [List.getElem?_eq_getElem hkl, List.getD_eq_getElem?_getD,
      List.getElem?_eq_getElem hkl]
    rfl
  unfold Skel.childIsD
  rw [if_neg (by simp; omega), hsome]

-- ================================================== segment algebra

theorem walkSeg_empty (h' a : Nat) : walkSeg sk h' a a = [] := by
  unfold walkSeg
  rw [Nat.sub_self]
  rfl

theorem walkSeg_single (h' k : Nat) :
    walkSeg sk h' k (k + 1) = scopeBlock sk (wpk h') k := by
  unfold walkSeg
  rw [Nat.add_sub_cancel_left, List.range'_one, List.flatMap_cons,
    List.flatMap_nil, List.append_nil]

/-- Abutting stage runs glue into one. -/
theorem walkSeg_glue {h' a b c : Nat} (hab : a ≤ b) (hbc : b ≤ c) :
    walkSeg sk h' a b ++ walkSeg sk h' b c = walkSeg sk h' a c := by
  unfold walkSeg
  rw [← List.flatMap_append,
    show c - a = (b - a) + (c - b) from by omega,
    ← List.range'_append,
    show a + 1 * (b - a) = b from by omega]

/-- `wiresBefore` is monotone in the cursor (past the stage's end the
`take` saturates and the sum freezes). -/
theorem wiresBefore_mono (h : Nat) : ∀ {k k' : Nat}, k ≤ k' →
    sk.wiresBefore h k ≤ sk.wiresBefore h k' := by
  intro k k' hkk
  induction k' with
  | zero =>
      have hk0 : k = 0 := by omega
      subst hk0
      exact Nat.le_refl _
  | succ k' ih =>
      by_cases hlast : k = k' + 1
      · subst hlast
        exact Nat.le_refl _
      · have hstep : sk.wiresBefore h k' ≤ sk.wiresBefore h (k' + 1) := by
          by_cases hin : k' < sk.stageLen h
          · rw [wiresBefore_succ sk hin]
            omega
          · unfold Skel.wiresBefore
            rw [List.take_of_length_le (by
                unfold Skel.stageLen at hin
                omega),
              List.take_of_length_le (by
                unfold Skel.stageLen at hin
                omega)]
            exact Nat.le_refl _
        exact Nat.le_trans (ih (by omega)) hstep

/-- `dsBefore` is monotone in the cursor (same saturation argument as
`wiresBefore_mono`). -/
theorem dsBefore_mono (h : Nat) : ∀ {k k' : Nat}, k ≤ k' →
    sk.dsBefore h k ≤ sk.dsBefore h k' := by
  intro k k' hkk
  induction k' with
  | zero =>
      have hk0 : k = 0 := by omega
      subst hk0
      exact Nat.le_refl _
  | succ k' ih =>
      by_cases hlast : k = k' + 1
      · subst hlast
        exact Nat.le_refl _
      · have hstep : sk.dsBefore h k' ≤ sk.dsBefore h (k' + 1) := by
          by_cases hin : k' < sk.stageLen h
          · rw [dsBefore_succ sk hin]
            omega
          · unfold Skel.dsBefore
            rw [List.take_of_length_le (by
                unfold Skel.stageLen at hin
                omega),
              List.take_of_length_le (by
                unfold Skel.stageLen at hin
                omega)]
            exact Nat.le_refl _
        exact Nat.le_trans (ih (by omega)) hstep

/-- A kid cursor stays inside the kid stage (the `Prec` induction's
inline bound, named). -/
theorem kid_index_lt (hwf : sk.wellFormed = true) {j : Nat}
    (h1 : 1 ≤ j) (hjr : j < sk.rootH) {A i : Nat}
    (hA : A < sk.stageLen j)
    (hi : i < sk.nChildren j (sk.stageScope j A)) :
    sk.wiresBefore j A + i < sk.stageLen (j - 1) := by
  have htot := wiresBefore_total sk hwf h1 hjr
  have hmono := wiresBefore_mono sk j
    (show A + 1 ≤ sk.stageLen j from hA)
  have hsucc := wiresBefore_succ sk hA
  omega

/-- A kid cursor sits strictly inside its parent's wire window. -/
theorem spine_nest {g B t : Nat} (hB : B < sk.stageLen g)
    (ht : t < sk.nChildren g (sk.stageScope g B)) :
    sk.wiresBefore g B + t < sk.wiresBefore g (B + 1) := by
  have hsucc := wiresBefore_succ sk hB
  omega

/-- `ds_wires` at a mid-scope cursor: the D count through a partial
kid window is the boundary D prefix plus the slot rank.

At `i = nChildren` this recovers `ds_wires` at `k + 1` through
`dRank_total` and the `succ` prefix-sum steps. -/
theorem ds_wires_mid (hwf : sk.wellFormed = true) {j : Nat}
    (h1 : 1 ≤ j) (hjr : j < sk.rootH) {k : Nat}
    (hk : k < sk.stageLen j) :
    ∀ {i : Nat}, i ≤ sk.nChildren j (sk.stageScope j k) →
    (((sk.scopesAt j).take (sk.wiresBefore j k + i)).filter
        (fun s => (sk.scope s).kind == Kind.D)).length
      = sk.dsBefore j k + dRank sk (wpk j) k i := by
  intro i
  induction i with
  | zero =>
      intro _
      have h0 : dRank sk (wpk j) k 0 = 0 := rfl
      rw [Nat.add_zero, Model.ds_wires hwf h1 hjr k, h0]
      omega
  | succ i ih =>
      intro hi1
      have hilt : i < sk.nChildren j (sk.stageScope j k) := by omega
      have hcur : sk.wiresBefore j k + i < sk.stageLen (j - 1) :=
        kid_index_lt sk hwf h1 hjr hk hilt
      have hlen : (sk.scopesAt j).length = sk.stageLen (j - 1) := by
        unfold Skel.stageLen Skel.stageScopes
        rw [show j - 1 + 1 = j from by omega]
      have hcur' : sk.wiresBefore j k + i < (sk.scopesAt j).length := by
        omega
      have helem : (sk.scopesAt j)[sk.wiresBefore j k + i]
          = sk.stageScope (j - 1) (sk.wiresBefore j k + i) := by
        unfold Skel.stageScope Skel.stageScopes
        rw [show j - 1 + 1 = j from by omega,
          List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hcur']
        rfl
      rw [show sk.wiresBefore j k + (i + 1)
            = (sk.wiresBefore j k + i) + 1 from by omega,
        List.take_add_one, List.getElem?_eq_getElem hcur']
      simp only [Option.toList_some, List.filter_append,
        List.length_append]
      rw [ih (by omega), dRank_succ,
        show sk.childIsD (wpk j).2 (sk.stageScope (wpk j).2 k) i
          = sk.childIsD j (sk.stageScope j k) i from rfl,
        childIsD_eq_kid_kind sk hwf h1 hjr hk hilt, helem]
      simp only [List.filter_cons, List.filter_nil]
      by_cases hD : ((sk.scope (sk.stageScope (j - 1)
          (sk.wiresBefore j k + i))).kind == Kind.D) = true
      · rw [if_pos hD, if_pos hD]
        simp only [List.length_cons, List.length_nil]
        omega
      · rw [if_neg hD, if_neg hD]
        simp only [List.length_nil]
        omega

/-- The answerer pends line at a mid-scope resolution cursor is the
kid-stage wire cut at the same slot: `pendsBefore_answerer` composed
with `ds_wires_mid`. -/
theorem pends_cut_mid (hwf : sk.wellFormed = true) {p : Party}
    {j : Nat} (hna : asks p j = false) (h1 : 1 ≤ j)
    (hjr : j < sk.rootH) {k i : Nat} (hk : k < sk.stageLen j)
    (hi : i ≤ sk.nChildren j (sk.stageScope j k)) :
    sk.pendsBefore p j (sk.dsBefore j k + dRank sk (wpk j) k i)
      = sk.wiresBefore (j - 1) (sk.wiresBefore j k + i) := by
  rw [← ds_wires_mid sk hwf h1 hjr hk hi]
  exact Model.pendsBefore_answerer hwf hna h1 _

theorem descIdx_zero (h' j : Nat) : descIdx sk h' 0 j = j := rfl

theorem descIdx_succ (h' d j : Nat) :
    descIdx sk h' (d + 1) j
      = descIdx sk h' d (sk.wiresBefore (h' + d + 1) j) := rfl

/-- Descent preserves cursor order. -/
theorem descIdx_mono (h' : Nat) : ∀ (d : Nat) {j j' : Nat}, j ≤ j' →
    descIdx sk h' d j ≤ descIdx sk h' d j' := by
  intro d
  induction d with
  | zero => intro j j' hjj; exact hjj
  | succ d ih =>
      intro j j' hjj
      rw [descIdx_succ, descIdx_succ]
      exact ih (wiresBefore_mono sk _ hjj)

/-- Peeling `descIdx` from the bottom: a full descent is a shallower
descent followed by one wire hop — the bridge between the alignment's
cursor form and the descent telescope's nested `wiresBefore`. -/
theorem descIdx_peel : ∀ (d h' j : Nat),
    descIdx sk h' (d + 1) j
      = sk.wiresBefore (h' + 1) (descIdx sk (h' + 1) d j) := by
  intro d
  induction d with
  | zero => intro h' j; rfl
  | succ d ih =>
      intro h' j
      rw [descIdx_succ sk h' (d + 1) j, ih h' _,
        descIdx_succ sk (h' + 1) d j,
        show h' + 1 + d + 1 = h' + (d + 1) + 1 from by omega]

-- ============================================== per-channel ownership
-- The owner of every event shape the weave emits at stage `h`, as
-- `evOwner` computations (the numbering layer's owner functions read
-- back at the channels the recursion touches). `hh : h < rootH` keeps
-- the stage real; `hwf` supplies rootH's evenness where the opener
-- channels could collide.

theorem evOwner_wireIn (hwf : sk.wellFormed = true) (h n : Nat) :
    evOwner sk (wireIn (wpk h), false, n) = walkIdx sk h := by
  have hev := (wf_rootH hwf).1
  have hc1 : ¬((wpk h).1.other = Party.I ∧ h + 1 = sk.rootH) := by
    rintro ⟨hpI, hh1⟩
    have hodd : (h % 2 == 1) = true := by
      have hm : h % 2 = 1 := by omega
      simpa using hm
    unfold wpk at hpI
    rw [hodd] at hpI
    simp [Party.other] at hpI
  show rcvOwner sk (Chan.wire (wpk h).1.other (h + 1)) = walkIdx sk h
  simp only [rcvOwner]
  rw [if_neg hc1, if_neg (by omega : ¬(h + 1 = 0))]
  have h11 : h + 1 - 1 = h := by omega
  rw [h11]

theorem evOwner_askedIn {h : Nat} (n : Nat) :
    evOwner sk (askedIn (wpk h), false, n) = walkIdx sk h := rfl

theorem evOwner_upperOut {h : Nat} (n : Nat) :
    evOwner sk (upperOut (wpk h), true, n) = walkIdx sk h := rfl

theorem evOwner_lowerOut {h : Nat} (n : Nat) :
    evOwner sk (lowerOut (wpk h), true, n) = walkIdx sk h := rfl

theorem evOwner_wireOut {h : Nat} (hh : h < sk.rootH) (n : Nat) :
    evOwner sk (wireOut (wpk h), true, n) = walkIdx sk h := by
  show sndOwner sk (Chan.wire (wpk h).1 h) = walkIdx sk h
  simp only [sndOwner]
  rw [if_neg (Nat.ne_of_lt hh)]

theorem evOwner_askedOut {h : Nat} (h1 : 1 ≤ h) (hh : h < sk.rootH)
    (n : Nat) :
    evOwner sk (askedOut (wpk h), true, n) = walkIdx sk h := by
  by_cases h2 : h < 2
  · have hone : h = 1 := by omega
    subst hone
    rfl
  · have ha : askedOut (wpk h) = Chan.asked (wpk h).1 (h - 2) :=
      if_neg h2
    show sndOwner sk (askedOut (wpk h)) = walkIdx sk h
    rw [ha]
    simp only [sndOwner]
    rw [if_neg fun hcon => absurd hcon.2 (by omega),
      if_neg fun hcon => absurd hcon.2 (by omega)]
    unfold walkIdx
    congr 1
    omega

-- ===================================== expansion shape, closed forms
-- The interpreter's op expansions with the inline `let`s resolved to
-- the `Skel` prefix sums, so the master induction computes filters
-- without re-deriving the arithmetic at every use.

/-- The last disputed child slot of stage scope `(h, k)`: the splice
point `scopeSends` and the weave share. -/
def lastDOf (h k : Nat) : Option Nat :=
  ((List.range (sk.nChildren h (sk.stageScope h k))).filter fun i =>
    sk.childIsD h (sk.stageScope h k) i).getLast?

/-- The chunk-`i` queries of stage scope `(h, k)`: the feed kid `i`'s
subtree receives, in trace order (`dRank`/`qSum` are the numbering
layer's named forms of the inline counters). -/
def chunkQ (h k i : Nat) : List Ev :=
  (List.range (sk.qCount h (sk.stageScope h k) i)).map fun t =>
    (askedOut (wpk h), true,
      sk.qsBefore h k + qSum sk (wpk h) k i + t)

/-- Kid `i`'s events as the own-stage filter sees them: the trace's
chunk with the parent summary spliced in when this kid closes the
dispute list. -/
def splicedChunk (h k : Nat) (lastD : Option Nat) (i : Nat) :
    List Ev :=
  (wireOut (wpk h), true, sk.wiresBefore h k + i)
    :: if sk.childIsD h (sk.stageScope h k) i then
        (lowerOut (wpk h), true, sk.dsBefore h k + dRank sk (wpk h) k i)
          :: ((if lastD == some i
                then [((upperOut (wpk h), true, k) : Ev)] else [])
            ++ chunkQ sk h k i)
      else []

theorem chunkQ_length (h k i : Nat) :
    (chunkQ sk h k i).length = sk.qCount h (sk.stageScope h k) i := by
  unfold chunkQ
  rw [List.length_map, List.length_range]

/-- Every chunk query is owned by the issuing walk. -/
theorem chunkQ_owner {h : Nat} (h1 : 1 ≤ h) (hh : h < sk.rootH)
    (k i : Nat) :
    ∀ e ∈ chunkQ sk h k i, evOwner sk e = walkIdx sk h := by
  intro e he
  unfold chunkQ at he
  obtain ⟨t, -, rfl⟩ := List.mem_map.1 he
  exact evOwner_askedOut sk h1 hh _

/-- A scope op's expansion, events flattened: prologue receives, the
undisputed-parent summary, then the kid ops in slot order. -/
theorem opEvents_scope_eq {h k : Nat} (hk : k ≤ sk.stageLen h)
    (feed : List Ev) :
    opEvents sk (.scope h k feed)
      = (wireIn (wpk h), false, k) :: (askedIn (wpk h), false, k)
          :: ((if lastDOf sk h k == none
                then [((upperOut (wpk h), true, k) : Ev)] else [])
            ++ (List.range (sk.nChildren h (sk.stageScope h k))).flatMap
                fun i => opEvents sk (.kid h k (sk.stageScope h k)
                  (lastDOf sk h k) (sk.wiresBefore h k) i feed)) := by
  rw [opEvents_scope]
  simp only [wScopeOps]
  rw [kidBase_eq_wiresBefore sk h k hk]
  simp only [lastDOf, wpk]
  by_cases hL : (((List.range (sk.nChildren h (sk.stageScope h k))).filter
      fun i => sk.childIsD h (sk.stageScope h k) i).getLast? == none)
  · rw [if_pos hL, if_pos hL]
    simp [opEvents_emit, List.flatMap_map]
  · rw [if_neg hL, if_neg hL]
    simp [opEvents_emit, List.flatMap_map]

/-- A kid op's expansion, events flattened: the wire; for a D kid the
resolution, the splice-point summary, the feed query, the subtree; for
a W kid the feed query and — off the leaf stage — a childless
subtree. -/
theorem opEvents_kid_eq (h k : Nat) (lastD : Option Nat)
    (kidBase i : Nat) (feed : List Ev) :
    opEvents sk (.kid h k (sk.stageScope h k) lastD kidBase i feed)
      = (wireOut (wpk h), true, sk.wiresBefore h k + i)
          :: (if sk.childIsD h (sk.stageScope h k) i then
                (lowerOut (wpk h), true,
                    sk.dsBefore h k + dRank sk (wpk h) k i)
                  :: ((if lastD == some i
                        then [((upperOut (wpk h), true, k) : Ev)] else [])
                    ++ (feed[i]?.toList
                      ++ opEvents sk
                          (.scope (h - 1) (kidBase + i) (chunkQ sk h k i))))
              else feed[i]?.toList
                ++ (if h == 0 then []
                    else opEvents sk (.scope (h - 1) (kidBase + i) []))) := by
  rw [opEvents_kid]
  simp only [wKidOps, wpk]
  cases hfi : feed[i]? <;>
    by_cases hD : sk.childIsD h (sk.stageScope h k) i <;>
    by_cases hL : (lastD == some i) <;>
    by_cases h0 : (h == 0) <;>
    simp [hD, hL, h0, opEvents_emit, dRank, qSum, chunkQ, wpk]

-- ================================================ list splice algebra
-- Generic pieces the master induction assembles filters with: pointwise
-- flatMap congruence, feed reconstruction from positional reads, and
-- the take/getD/drop reads of a list spliced at a known index.

private theorem flatMap_congr {α β : Type _} {l : List α}
    {f g : α → List β} (h : ∀ a ∈ l, f a = g a) :
    l.flatMap f = l.flatMap g := by
  rw [List.flatMap_def, List.flatMap_def, List.map_congr_left h]

/-- A list is the flatMap of its positional reads. -/
private theorem flatMap_getElem?_toList {α : Type _} :
    ∀ (F : List α),
      (List.range F.length).flatMap (fun i => F[i]?.toList) = F := by
  intro F
  induction F with
  | nil => rfl
  | cons a F ih =>
      rw [List.length_cons, List.range_succ_eq_map, List.flatMap_cons,
        List.flatMap_map]
      simp only [Nat.succ_eq_add_one, List.getElem?_cons_zero,
        List.getElem?_cons_succ, Option.toList_some]
      rw [ih, List.singleton_append]

private theorem getD_append_cons {α : Type _} (A : List α) {j : Nat}
    (hj : A.length = j) (b : α) (C : List α) (d : α) :
    (A ++ b :: C).getD j d = b := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_append_right (by omega),
    show j - A.length = 0 from by omega]
  rfl

private theorem drop_append_cons {α : Type _} :
    ∀ (A : List α) {j : Nat}, A.length = j → ∀ (b : α) (C : List α),
      (A ++ b :: C).drop (j + 1) = C := by
  intro A
  induction A with
  | nil =>
      intro j hj b C
      subst hj
      rfl
  | cons a A ih =>
      intro j hj b C
      subst hj
      rw [List.length_cons, List.cons_append, List.drop_succ_cons]
      exact ih rfl b C

/-- `range n` split at a member `j`. -/
private theorem range_splice {j n : Nat} (hj : j < n) :
    List.range n = List.range j ++ j :: List.range' (j + 1) (n - j - 1) := by
  have happ := List.range'_append
    (s := 0) (m := j) (n := n - j - 1 + 1) (step := 1)
  rw [show 0 + 1 * j = j from by omega,
    show j + (n - j - 1 + 1) = n from by omega] at happ
  rw [List.range_eq_range', List.range_eq_range', ← happ, List.range'_succ]

-- ================================================== the splice, named

/-- A `some` last-disputed slot is a real disputed slot. -/
theorem lastDOf_isD {h k j : Nat} (hj : lastDOf sk h k = some j) :
    sk.childIsD h (sk.stageScope h k) j = true
      ∧ j < sk.nChildren h (sk.stageScope h k) := by
  unfold lastDOf at hj
  rw [List.getLast?_eq_some_iff] at hj
  obtain ⟨ys, hys⟩ := hj
  have hmem : j ∈ (List.range (sk.nChildren h (sk.stageScope h k))).filter
      fun i => sk.childIsD h (sk.stageScope h k) i := by
    rw [hys]
    exact List.mem_append_right _ (List.mem_singleton.mpr rfl)
  rw [List.mem_filter, List.mem_range] at hmem
  exact ⟨hmem.2, hmem.1⟩

/-- A `none` last-disputed slot means no slot disputes. -/
theorem lastDOf_none {h k : Nat} (hn : lastDOf sk h k = none) :
    ∀ i < sk.nChildren h (sk.stageScope h k),
      sk.childIsD h (sk.stageScope h k) i = false := by
  unfold lastDOf at hn
  rw [List.getLast?_eq_none_iff] at hn
  intro i hi
  have := List.filter_eq_nil_iff.mp hn i (List.mem_range.mpr hi)
  simpa using this

/-- Equality along a step-frozen chain: if `g` is constant on every
step of `[a, b)`, its endpoints agree. -/
private theorem chain_eq {g : Nat → Nat} : ∀ {a b : Nat}, a ≤ b →
    (∀ t, a ≤ t → t < b → g (t + 1) = g t) → g b = g a := by
  intro a b
  induction b with
  | zero =>
      intro hab _
      have h0 : a = 0 := by omega
      subst h0
      rfl
  | succ b ih =>
      intro hab h
      by_cases hb : a = b + 1
      · subst hb
        rfl
      · rw [h b (by omega) (by omega)]
        exact ih (by omega) (fun t ht htb => h t ht (by omega))

/-- No disputed slot lies past the last one: the filtered range's
`getLast?` dominates (the range survives filtering in order). -/
theorem lastDOf_max {h k j i : Nat} (hj : lastDOf sk h k = some j)
    (hgt : j < i) : sk.childIsD h (sk.stageScope h k) i = false := by
  by_contra hDc
  rw [Bool.not_eq_false] at hDc
  have hin : i < sk.nChildren h (sk.stageScope h k) := by
    by_cases h0 : h = 0
    · exact absurd hDc (by simp [Skel.childIsD, h0])
    · unfold Skel.childIsD at hDc
      rw [if_neg (by simpa using h0)] at hDc
      unfold Skel.nChildren
      rw [if_neg (by simpa using h0)]
      cases hg : (sk.scope (sk.stageScope h k)).kids[i]? with
      | none => rw [hg] at hDc; exact absurd hDc (by simp)
      | some c => exact (List.getElem?_eq_some_iff.mp hg).1
  unfold lastDOf at hj
  rw [List.getLast?_eq_some_iff] at hj
  obtain ⟨ys, hys⟩ := hj
  have hmem : i ∈ (List.range (sk.nChildren h (sk.stageScope h k))).filter
      (fun i' => sk.childIsD h (sk.stageScope h k) i') :=
    List.mem_filter.mpr ⟨List.mem_range.mpr hin, hDc⟩
  have hpw : ((List.range (sk.nChildren h (sk.stageScope h k))).filter
      (fun i' => sk.childIsD h (sk.stageScope h k) i')).Pairwise (· < ·) :=
    List.pairwise_lt_range.sublist List.filter_sublist
  rw [hys] at hmem hpw
  rcases List.mem_append.mp hmem with hy | hone
  · have := (List.pairwise_append.mp hpw).2.2 i hy j
      (List.mem_singleton.mpr rfl)
    omega
  · have := List.mem_singleton.mp hone
    omega

/-- A disputed slot in range forces a last disputed slot at or past
it. -/
theorem lastDOf_isSome_of_D {h k i : Nat}
    (hD : sk.childIsD h (sk.stageScope h k) i = true)
    (hi : i < sk.nChildren h (sk.stageScope h k)) :
    ∃ j, lastDOf sk h k = some j ∧ i ≤ j := by
  cases hL : lastDOf sk h k with
  | none =>
      have hfalse := lastDOf_none sk hL i hi
      rw [hD] at hfalse
      exact absurd hfalse (by simp)
  | some j =>
      refine ⟨j, rfl, ?_⟩
      by_contra hlt
      have hji : j < i := by omega
      have hfalse := lastDOf_max sk hL hji
      rw [hD] at hfalse
      exact absurd hfalse (by simp)

/-- The last disputed slot's rank is the scope's D total, less one:
the splice site closes the dispute list. -/
theorem dRank_lastD {h k j : Nat} (hj : lastDOf sk h k = some j) :
    dRank sk (wpk h) k j + 1 = sk.dOf h (sk.stageScope h k) := by
  obtain ⟨hDj, hjn⟩ := lastDOf_isD sk hj
  have hsucc := dRank_succ sk (wpk h) k j
  rw [show sk.childIsD (wpk h).2 (sk.stageScope (wpk h).2 k) j
      = sk.childIsD h (sk.stageScope h k) j from rfl,
    if_pos hDj] at hsucc
  have hchain : dRank sk (wpk h) k (sk.nChildren h (sk.stageScope h k))
      = dRank sk (wpk h) k (j + 1) :=
    chain_eq (by omega) (fun t ht htn => by
      have hf := lastDOf_max sk hj (show j < t by omega)
      have hst := dRank_succ sk (wpk h) k t
      rw [show sk.childIsD (wpk h).2 (sk.stageScope (wpk h).2 k) t
          = sk.childIsD h (sk.stageScope h k) t from rfl,
        if_neg (by simp [hf])] at hst
      omega)
  have htot : dRank sk (wpk h) k (sk.nChildren h (sk.stageScope h k))
      = sk.dOf h (sk.stageScope h k) := dRank_total sk (wpk h) k
  omega

/-- A disputed slot strictly before the last one leaves room for two:
its rank plus the last slot's stay inside the D total. -/
theorem dRank_below_lastD {h k i j : Nat} (hj : lastDOf sk h k = some j)
    (hD : sk.childIsD h (sk.stageScope h k) i = true) (hne : i ≠ j) :
    dRank sk (wpk h) k i + 2 ≤ sk.dOf h (sk.stageScope h k) := by
  have hij : i < j := by
    rcases Nat.lt_or_ge i j with hlt | hge
    · exact hlt
    · have hji : j < i := by omega
      have hfalse := lastDOf_max sk hj hji
      rw [hD] at hfalse
      exact absurd hfalse (by simp)
  have hsucc := dRank_succ sk (wpk h) k i
  rw [show sk.childIsD (wpk h).2 (sk.stageScope (wpk h).2 k) i
      = sk.childIsD h (sk.stageScope h k) i from rfl,
    if_pos hD] at hsucc
  have hmono := dRank_mono sk (wpk h) k (show i + 1 ≤ j from hij)
  have hlast := dRank_lastD sk hj
  omega

/-- `childChunk` at a walk key, `let`s resolved. -/
theorem childChunk_eq (h k i : Nat) :
    childChunk sk (wpk h) k i
      = if sk.childIsD h (sk.stageScope h k) i then
          (wireOut (wpk h), true, sk.wiresBefore h k + i)
            :: (lowerOut (wpk h), true,
                sk.dsBefore h k + dRank sk (wpk h) k i)
            :: chunkQ sk h k i
        else [(wireOut (wpk h), true, sk.wiresBefore h k + i)] := by
  by_cases hD : sk.childIsD h (sk.stageScope h k) i <;>
    simp [childChunk, hD, dRank, qSum, chunkQ, wpk]

/-- `scopeSends`' splice, resolved to a per-kid flatMap: the parent
summary rides the last disputed chunk (after its resolution, before
its queries), or leads when nothing disputes — `splicedChunk` is that
placement, kid by kid. -/
theorem scopeSends_eq (h k : Nat) :
    scopeSends sk (wpk h) k
      = (if lastDOf sk h k == none
            then [((upperOut (wpk h), true, k) : Ev)] else [])
        ++ (List.range (sk.nChildren h (sk.stageScope h k))).flatMap
            (splicedChunk sk h k (lastDOf sk h k)) := by
  cases hL : lastDOf sk h k with
  | none =>
      have hall := lastDOf_none sk hL
      unfold lastDOf at hL
      simp only [scopeSends, show (wpk h).2 = h from rfl]
      rw [hL]
      dsimp only
      rw [flatMap_congr (g := childChunk sk (wpk h) k) fun i hi => by
        have hD := hall i (List.mem_range.mp hi)
        rw [childChunk_eq, if_neg (by rw [hD]; exact Bool.false_ne_true),
          splicedChunk,
          if_neg (by rw [hD]; exact Bool.false_ne_true)]]
      simp [List.flatMap_def, wpk]
  | some j =>
      obtain ⟨hDj, hjn⟩ := lastDOf_isD sk hL
      unfold lastDOf at hL
      simp only [scopeSends, show (wpk h).2 = h from rfl]
      rw [hL]
      dsimp only
      rw [range_splice hjn, List.map_append, List.map_cons]
      rw [List.take_left' (by rw [List.length_map, List.length_range]),
        getD_append_cons _ (by rw [List.length_map, List.length_range]) _ _ _,
        drop_append_cons _ (by rw [List.length_map, List.length_range]) _ _]
      rw [List.flatMap_append, List.flatMap_cons]
      rw [flatMap_congr (l := List.range j)
          (g := childChunk sk (wpk h) k) fun i hi => by
        have hne : (j == i) = false := by
          have := List.mem_range.mp hi
          simp only [beq_eq_false_iff_ne, ne_eq]
          omega
        rw [splicedChunk, Option.some_beq_some, hne, if_neg Bool.false_ne_true,
          childChunk_eq]
        by_cases hD : sk.childIsD h (sk.stageScope h k) i
        · rw [if_pos hD, if_pos hD, List.nil_append]
        · rw [if_neg hD, if_neg hD]]
      rw [flatMap_congr (l := List.range' (j + 1) _)
          (g := childChunk sk (wpk h) k) fun i hi => by
        have hne : (j == i) = false := by
          have := List.mem_range'.mp hi
          simp only [beq_eq_false_iff_ne, ne_eq]
          omega
        rw [splicedChunk, Option.some_beq_some, hne, if_neg Bool.false_ne_true,
          childChunk_eq]
        by_cases hD : sk.childIsD h (sk.stageScope h k) i
        · rw [if_pos hD, if_pos hD, List.nil_append]
        · rw [if_neg hD, if_neg hD]]
      rw [splicedChunk, Option.some_beq_some, beq_self_eq_true,
        if_pos rfl, if_pos hDj, childChunk_eq, if_pos hDj]
      simp [List.flatMap_def, wpk]

-- ================================================ glue and monotonicity

/-- Adjacent runs over a monotone cursor glue into one run. -/
private theorem walkSeg_glue_range (h' : Nat) (g : Nat → Nat)
    (hmono : ∀ i, g i ≤ g (i + 1)) :
    ∀ n, (List.range n).flatMap (fun i => walkSeg sk h' (g i) (g (i + 1)))
      = walkSeg sk h' (g 0) (g n) := by
  intro n
  induction n with
  | zero => rw [List.range_zero, List.flatMap_nil, walkSeg_empty]
  | succ n ih =>
      have h0n : g 0 ≤ g n := by
        clear ih
        induction n with
        | zero => exact Nat.le_refl _
        | succ m ihm => exact Nat.le_trans ihm (hmono m)
      rw [List.range_succ, List.flatMap_append, ih, List.flatMap_cons,
        List.flatMap_nil, List.append_nil, walkSeg_glue sk h0n (hmono n)]

/-- Deeper stages sit later in `procs`: `walkIdx` is strictly antitone
on real stages. -/
theorem walkIdx_lt {h' h : Nat} (hlt : h' < h) (hh : h < sk.rootH) :
    walkIdx sk h < walkIdx sk h' := by
  unfold walkIdx
  omega

/-- An undisputed kid's subtree is childless: the W/R child of a stage
scope has no stage children of its own (its scope is real and non-D,
so `wellFormed` empties both its kid list and its leaf requests). -/
theorem nChildren_kid_notD (hwf : sk.wellFormed = true) {h k i : Nat}
    (h1 : 1 ≤ h) (hh : h < sk.rootH) (hk : k < sk.stageLen h)
    (hi : i < sk.nChildren h (sk.stageScope h k))
    (hnd : sk.childIsD h (sk.stageScope h k) i = false) :
    sk.nChildren (h - 1)
      (sk.stageScope (h - 1) (sk.wiresBefore h k + i)) = 0 := by
  rw [stageScope_kid sk hwf h1 hh hk hi]
  have hkl : i < ((sk.scope (sk.stageScope h k)).kids).length := by
    rw [← nChildren_of_pos sk h1]
    exact hi
  have hsome : ((sk.scope (sk.stageScope h k)).kids)[i]?
      = some (((sk.scope (sk.stageScope h k)).kids).getD i 0) := by
    rw [List.getElem?_eq_getElem hkl, List.getD_eq_getElem?_getD,
      List.getElem?_eq_getElem hkl]
    rfl
  have hmem : ((sk.scope (sk.stageScope h k)).kids).getD i 0
      ∈ (sk.scope (sk.stageScope h k)).kids := by
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hkl]
    exact List.getElem_mem hkl
  obtain ⟨hcn, -⟩ := wf_kid_facts hwf (stageScope_lt_scopes sk hk) _ hmem
  have hne : ((h : Nat) == 0) = false := by
    simp only [beq_eq_false_iff_ne, ne_eq]
    omega
  have hnd' : (sk.scope (((sk.scope (sk.stageScope h k)).kids).getD
      i 0)).kind ≠ Kind.D := by
    intro hcon
    simp only [Skel.childIsD, hne, hsome, hcon, Bool.false_eq_true,
      if_false, beq_self_eq_true] at hnd
    exact absurd hnd (by decide)
  obtain ⟨hkids, hleaf⟩ := wf_scope_notD hwf hcn hnd'
  unfold Skel.nChildren
  by_cases h0 : h - 1 = 0
  · rw [if_pos (by simpa using h0), hleaf]
  · rw [if_neg (by simpa using h0), hkids]
    rfl

-- ============================================== root-stage uniqueness
-- The weave descends from ONE root scope op; that this covers stage
-- `rootH - 1` entirely is `wellFormed`'s kid accounting: kids are
-- exactly the non-root ids (count + dedup + no-root-kid), each kid
-- sits strictly above its parent's id and one height down, so nothing
-- but the root reaches height `rootH`.

private theorem eraseDupsBy_loop_nodup :
    ∀ (l acc : List Nat), acc.Nodup →
      (List.eraseDupsBy.loop (· == ·) l acc).Nodup := by
  intro l
  induction l with
  | nil =>
      intro acc hacc
      show acc.reverse.Nodup
      exact ((List.reverse_perm acc).nodup_iff).mpr hacc
  | cons a l ih =>
      intro acc hacc
      show (match acc.any (fun b => a == b) with
        | true => List.eraseDupsBy.loop (· == ·) l acc
        | false => List.eraseDupsBy.loop (· == ·) l (a :: acc)).Nodup
      cases hany : acc.any (fun b => a == b) with
      | true => exact ih acc hacc
      | false =>
          refine ih (a :: acc) (List.nodup_cons.mpr ⟨?_, hacc⟩)
          intro hmem
          have hcon : acc.any (fun b => a == b) = true :=
            List.any_eq_true.mpr ⟨a, hmem, beq_self_eq_true a⟩
          rw [hany] at hcon
          exact absurd hcon (by decide)

private theorem eraseDups_nodup (l : List Nat) : l.eraseDups.Nodup :=
  eraseDupsBy_loop_nodup l [] List.nodup_nil

private theorem foldl_append_eq_flatMap {α β : Type _} (f : β → List α) :
    ∀ (l : List β) (acc : List α),
      l.foldl (fun acc b => acc ++ f b) acc = acc ++ l.flatMap f := by
  intro l
  induction l with
  | nil =>
      intro acc
      rw [List.foldl_nil, List.flatMap_nil, List.append_nil]
  | cons b l ih =>
      intro acc
      rw [List.foldl_cons, ih, List.flatMap_cons, List.append_assoc]

/-- The kids-fold's ascending-chain half, read pointwise: every
checked kid sits strictly above the fold's start. -/
private theorem kids_fold_gt {n hgt : Nat} :
    ∀ (kids : List Nat) (a : Nat) (b : Bool),
      ((kids.foldl (fun (acc : Nat × Bool) k =>
          (k, acc.2 && decide (k > acc.1) && decide (k < n) &&
              ((sk.scope k).height == hgt))) (a, b)).2 = true) →
      ∀ k ∈ kids, a < k := by
  intro kids
  induction kids with
  | nil =>
      intro a b _ k hk
      cases hk
  | cons k' l ih =>
      intro a b h
      rw [List.foldl_cons] at h
      have hb := (kids_fold_facts sk l k' _ h).1
      simp only [Bool.and_eq_true, decide_eq_true_eq, beq_iff_eq] at hb
      intro k hk
      rcases List.mem_cons.1 hk with rfl | hk'
      · exact hb.1.1.2
      · exact Nat.lt_trans hb.1.1.2 (ih k' _ h k hk')

/-- Kids sit strictly above their parent's id, extracted. -/
private theorem wf_kid_gt {sk : Skel} (hwf : sk.wellFormed = true)
    {j : Nat} (hj : j < sk.scopes.length) :
    ∀ k ∈ (sk.scope j).kids, j < k := by
  unfold Skel.wellFormed at hwf
  simp only [Bool.and_eq_true, List.all_eq_true, decide_eq_true_eq,
    beq_iff_eq] at hwf
  have hper := hwf.1.1.1.1.1.2
  have hfold := (hper j (List.mem_range.mpr hj)).2
  exact kids_fold_gt sk _ _ _ hfold

/-- The kid-accounting conjuncts, extracted: a root exists at
`rootH`, the deduplicated kid list counts every non-root id, and the
root is nobody's kid. -/
private theorem wf_counts {sk : Skel} (hwf : sk.wellFormed = true) :
    0 < sk.scopes.length
      ∧ (sk.scope 0).height = sk.rootH
      ∧ ((sk.scopes.flatMap (fun sc => sc.kids)).eraseDups).length
          = sk.scopes.length - 1
      ∧ 0 ∉ sk.scopes.flatMap (fun sc => sc.kids) := by
  have hkid : sk.scopes.foldl (fun acc sc => acc ++ sc.kids) []
      = sk.scopes.flatMap (fun sc => sc.kids) := by
    rw [foldl_append_eq_flatMap _ _ [], List.nil_append]
  unfold Skel.wellFormed at hwf
  simp only [Bool.and_eq_true, List.all_eq_true, decide_eq_true_eq,
    beq_iff_eq, Bool.not_eq_eq_eq_not, Bool.not_true] at hwf
  refine ⟨hwf.1.1.1.1.1.1.1.1.1, hwf.1.1.1.1.1.1.1.1.2, ?_, ?_⟩
  · rw [← hkid]
    exact hwf.1.1.1.2
  · intro h0
    have hcont := hwf.1.1.2
    rw [hkid] at hcont
    rw [List.contains_eq_mem, decide_eq_false_iff_not] at hcont
    exact hcont h0

/-- Every non-root scope is some scope's kid: the dedup'd kid list is
a `Nodup` sublist of the non-root ids of the same length, hence a
permutation. -/
private theorem wf_kid_coverage {sk : Skel} (hwf : sk.wellFormed = true)
    {j : Nat} (h1 : 1 ≤ j) (hj : j < sk.scopes.length) :
    ∃ p, p < sk.scopes.length ∧ j ∈ (sk.scope p).kids := by
  obtain ⟨hn, -, hlen, hno0⟩ := wf_counts hwf
  have hin : ∀ x ∈ (sk.scopes.flatMap (fun sc => sc.kids)).eraseDups,
      x ∈ List.range' 1 (sk.scopes.length - 1) := by
    intro x hx
    rw [List.mem_eraseDups] at hx
    obtain ⟨sc, hsc, hxk⟩ := List.mem_flatMap.1 hx
    obtain ⟨p, hp, rfl⟩ := List.mem_iff_getElem.1 hsc
    have hscope : sk.scope p = sk.scopes[p] := by
      unfold Skel.scope
      rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hp]
      rfl
    rw [← hscope] at hxk
    have hxn := (wf_kid_facts hwf hp x hxk).1
    have hx0 : x ≠ 0 := by
      intro hx0
      subst hx0
      exact hno0 hx
    rw [List.mem_range'_1]
    omega
  have hperm : ((sk.scopes.flatMap (fun sc => sc.kids)).eraseDups).Perm
      (List.range' 1 (sk.scopes.length - 1)) := by
    refine (List.subperm_of_subset
      (eraseDups_nodup _) hin).perm_of_length_le ?_
    rw [hlen, List.length_range']
    exact Nat.le_refl _
  have hjmem : j ∈ (sk.scopes.flatMap (fun sc => sc.kids)).eraseDups := by
    rw [hperm.mem_iff, List.mem_range'_1]
    omega
  rw [List.mem_eraseDups] at hjmem
  obtain ⟨sc, hsc, hjk⟩ := List.mem_flatMap.1 hjmem
  obtain ⟨p, hp, rfl⟩ := List.mem_iff_getElem.1 hsc
  have hscope : sk.scope p = sk.scopes[p] := by
    unfold Skel.scope
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hp]
    rfl
  rw [← hscope] at hjk
  exact ⟨p, hp, hjk⟩

/-- Only the root reaches the root height: every other scope descends
from a strictly earlier id, so its height falls below `rootH` by the
parent chain. -/
private theorem wf_height_lt {sk : Skel} (hwf : sk.wellFormed = true) :
    ∀ j, j < sk.scopes.length → j ≠ 0 →
      (sk.scope j).height < sk.rootH := by
  intro j
  induction j using Nat.strongRecOn with
  | _ j IH =>
      intro hj hj0
      obtain ⟨p, hpn, hpk⟩ := wf_kid_coverage hwf (by omega) hj
      have hplt : p < j := wf_kid_gt hwf hpn j hpk
      have hht := (wf_kid_facts hwf hpn j hpk).2
      by_cases hp0 : p = 0
      · subst hp0
        have hr0 := (wf_counts hwf).2.1
        have hge := (wf_rootH hwf).2
        omega
      · have hlt' := IH p hplt hpn hp0
        omega

/-- The root stage is the root alone. -/
theorem wf_root_stage {sk : Skel} (hwf : sk.wellFormed = true) :
    sk.scopesAt sk.rootH = [0] := by
  obtain ⟨hn, h0, -, -⟩ := wf_counts hwf
  unfold Skel.scopesAt
  rw [range_splice hn]
  simp only [List.range_zero, List.nil_append, Nat.zero_add,
    Nat.sub_zero]
  rw [List.filter_cons_of_pos (by
    simp only [h0, beq_self_eq_true])]
  have hnil : (List.range' 1 (sk.scopes.length - 1)).filter
      (fun i => (sk.scope i).height == sk.rootH) = [] := by
    rw [List.filter_eq_nil_iff]
    intro a ha
    have hmem := List.mem_range'_1.mp ha
    have hlt := wf_height_lt hwf a (by omega) (by omega)
    simp only [beq_iff_eq]
    omega
  rw [hnil]

/-- The top stage has exactly one scope slot. -/
theorem wf_stageLen_top (hwf : sk.wellFormed = true) :
    sk.stageLen (sk.rootH - 1) = 1 := by
  have hge := (wf_rootH hwf).2
  unfold Skel.stageLen Skel.stageScopes
  rw [show sk.rootH - 1 + 1 = sk.rootH from by omega, wf_root_stage hwf]
  rfl

/-- The top stage's scope is the root. -/
theorem wf_stageScope_top (hwf : sk.wellFormed = true) :
    sk.stageScope (sk.rootH - 1) 0 = 0 := by
  have hge := (wf_rootH hwf).2
  unfold Skel.stageScope Skel.stageScopes
  rw [show sk.rootH - 1 + 1 = sk.rootH from by omega, wf_root_stage hwf]
  rfl

-- ================================================ telescope endpoints

/-- Descent from index 0 stays at 0: empty prefixes sum to nothing. -/
theorem descIdx_zero_arg (h' : Nat) : ∀ d, descIdx sk h' d 0 = 0 := by
  intro d
  induction d with
  | zero => rfl
  | succ d ihd =>
      rw [descIdx_succ, show sk.wiresBefore (h' + d + 1) 0 = 0 from rfl,
        ihd]

/-- Descent from a stage's full length lands on the lower stage's full
length: `wiresBefore_total`, telescoped. -/
theorem descIdx_total (hwf : sk.wellFormed = true) :
    ∀ (d h' : Nat), h' + d < sk.rootH →
      descIdx sk h' d (sk.stageLen (h' + d)) = sk.stageLen h' := by
  intro d
  induction d with
  | zero =>
      intro h' _
      rw [Nat.add_zero, descIdx_zero]
  | succ d ihd =>
      intro h' hd
      rw [descIdx_succ,
        show h' + (d + 1) = h' + d + 1 from by omega,
        wiresBefore_total sk hwf (show 1 ≤ h' + d + 1 from by omega)
          (show h' + d + 1 < sk.rootH from by omega),
        show h' + d + 1 - 1 = h' + d from by omega]
      exact ihd h' (by omega)

/-- A descended in-range cursor stays in range. -/
theorem descIdx_le_stageLen (hwf : sk.wellFormed = true) {h' d j : Nat}
    (hd : h' + d < sk.rootH) (hj : j ≤ sk.stageLen (h' + d)) :
    descIdx sk h' d j ≤ sk.stageLen h' := by
  have hmono := descIdx_mono sk h' d hj
  rw [descIdx_total sk hwf d h' hd] at hmono
  exact hmono

-- ================================================ the master induction

/-- The subtree alignment (the module doc's master induction): under
the feed contract — one query per kid slot, all owned by one process
`mF` strictly before the scope's own walk — a subtree op's events
partition exactly into the manual traces' segments.

The three clauses, in the module doc's numbering: (3) every event is
owned by the feeder or a covered walk; (2) the feeder's filter is the
feed, in order; (1) each covered walk's filter is its contiguous
`descIdx` run — at the own stage, the scope block itself, with the
kid feeds resplicing the chunk queries into `scopeSends`' §5 shape. -/
theorem align_scope (hwf : sk.wellFormed = true) :
    ∀ (h k : Nat) (F : List Ev) (mF : Nat),
      h < sk.rootH → k < sk.stageLen h →
      F.length = sk.nChildren h (sk.stageScope h k) →
      (∀ e ∈ F, evOwner sk e = mF) →
      mF < walkIdx sk h →
      ((∀ e ∈ opEvents sk (.scope h k F),
          evOwner sk e = mF
            ∨ ∃ h', h' ≤ h ∧ evOwner sk e = walkIdx sk h')
        ∧ (opEvents sk (.scope h k F)).filter
            (fun e => evOwner sk e == mF) = F
        ∧ ∀ h' ≤ h,
            (opEvents sk (.scope h k F)).filter
                (fun e => evOwner sk e == walkIdx sk h')
              = walkSeg sk h' (descIdx sk h' (h - h') k)
                  (descIdx sk h' (h - h') (k + 1))) := by
  intro h
  induction h with
  | zero =>
      intro k F mF hh hk hF hFo hmF
      have hD0 : ∀ i, sk.childIsD 0 (sk.stageScope 0 k) i = false :=
        fun _ => rfl
      have hLn : lastDOf sk 0 k = none := by
        unfold lastDOf
        rw [List.getLast?_eq_none_iff, List.filter_eq_nil_iff]
        intro a _
        rw [hD0 a]
        exact Bool.false_ne_true
      have hE := opEvents_scope_eq sk (Nat.le_of_lt hk) F
      rw [hLn, if_pos (show ((none : Option Nat) == none) = true by rfl)] at hE
      have hkidE : ∀ i,
          opEvents sk (.kid 0 k (sk.stageScope 0 k) none
            (sk.wiresBefore 0 k) i F)
          = (wireOut (wpk 0), true, sk.wiresBefore 0 k + i)
              :: F[i]?.toList := by
        intro i
        rw [opEvents_kid_eq,
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
        · rcases he with _ | ⟨_, he⟩
          · exact Or.inr ⟨0, Nat.le_refl 0, evOwner_upperOut sk k⟩
          · cases he
        · obtain ⟨i, -, hei⟩ := List.mem_flatMap.1 he
          rw [hkidE i] at hei
          rcases hei with _ | ⟨_, hei⟩
          · exact Or.inr ⟨0, Nat.le_refl 0, evOwner_wireOut sk hh _⟩
          · exact Or.inl (hFo e
              (List.mem_of_getElem? (Option.mem_toList.1 hei)))
      · -- (2) the feeder's filter is the feed
        rw [hE,
          List.filter_cons_of_neg (by
            simp only [evOwner_wireIn sk hwf, beq_iff_eq]; omega),
          List.filter_cons_of_neg (by
            simp only [evOwner_askedIn, beq_iff_eq]; omega),
          List.filter_append,
          List.filter_cons_of_neg (by
            simp only [evOwner_upperOut, beq_iff_eq]; omega),
          List.filter_nil, List.nil_append]
        have hkMF : ∀ i ∈ List.range (sk.nChildren 0 (sk.stageScope 0 k)),
            (opEvents sk (.kid 0 k (sk.stageScope 0 k) none
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
      · -- (1) the leaf walk's filter is the scope block
        intro h' hle
        have h0 : h' = 0 := Nat.le_zero.mp hle
        subst h0
        rw [Nat.sub_self, descIdx_zero, descIdx_zero, walkSeg_single]
        have hkOwn : ∀ i ∈ List.range (sk.nChildren 0 (sk.stageScope 0 k)),
            (opEvents sk (.kid 0 k (sk.stageScope 0 k) none
                (sk.wiresBefore 0 k) i F)).filter
              (fun e => evOwner sk e == walkIdx sk 0)
            = splicedChunk sk 0 k none i := by
          intro i _
          rw [hkidE i,
            List.filter_cons_of_pos (by
              simp only [evOwner_wireOut sk hh, beq_self_eq_true]),
            splicedChunk,
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
        rw [flatMap_congr hkOwn, scopeBlock, scopeSends_eq, hLn,
          if_pos (show ((none : Option Nat) == none) = true by rfl)]
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
      have hE := opEvents_scope_eq sk (Nat.le_of_lt hk) F
      have hkidE : ∀ i,
          opEvents sk (.kid (h + 1) k (sk.stageScope (h + 1) k)
            (lastDOf sk (h + 1) k) (sk.wiresBefore (h + 1) k) i F)
          = (wireOut (wpk (h + 1)), true, sk.wiresBefore (h + 1) k + i)
              :: (if sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i then
                    (lowerOut (wpk (h + 1)), true,
                        sk.dsBefore (h + 1) k + dRank sk (wpk (h + 1)) k i)
                      :: ((if lastDOf sk (h + 1) k == some i
                            then [((upperOut (wpk (h + 1)), true, k) : Ev)]
                            else [])
                        ++ (F[i]?.toList
                          ++ opEvents sk (.scope h
                              (sk.wiresBefore (h + 1) k + i)
                              (chunkQ sk (h + 1) k i))))
                  else F[i]?.toList
                    ++ opEvents sk (.scope h
                        (sk.wiresBefore (h + 1) k + i) [])) := by
        intro i
        rw [opEvents_kid_eq]
        simp only [Nat.add_sub_cancel,
          show ((h + 1 : Nat) == 0) = false from rfl, Bool.false_eq_true,
          if_false]
      -- the induction hypothesis, instantiated per kid: the D feed is
      -- the chunk queries (empty when the kid is undisputed)
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
      -- (A) each kid's own-stage filter is its spliced chunk
      have hkidOwn : ∀ i ∈ List.range
            (sk.nChildren (h + 1) (sk.stageScope (h + 1) k)),
          (opEvents sk (.kid (h + 1) k (sk.stageScope (h + 1) k)
              (lastDOf sk (h + 1) k) (sk.wiresBefore (h + 1) k) i F)).filter
            (fun e => evOwner sk e == walkIdx sk (h + 1))
          = splicedChunk sk (h + 1) k (lastDOf sk (h + 1) k) i := by
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
        rw [hkidE i, splicedChunk]
        by_cases hD : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
        · have hUkeep : ((if lastDOf sk (h + 1) k == some i
                then [((upperOut (wpk (h + 1)), true, k) : Ev)]
                else []).filter
              (fun e => evOwner sk e == walkIdx sk (h + 1)))
              = (if lastDOf sk (h + 1) k == some i
                then [((upperOut (wpk (h + 1)), true, k) : Ev)]
                else []) := by
            split
            · rw [List.filter_cons_of_pos (by
                simp only [evOwner_upperOut, beq_self_eq_true]),
                List.filter_nil]
            · rfl
          rw [if_pos hD, if_pos hD,
            List.filter_cons_of_pos (by
              simp only [evOwner_wireOut sk hh, beq_self_eq_true]),
            List.filter_cons_of_pos (by
              simp only [evOwner_lowerOut, beq_self_eq_true]),
            List.filter_append, List.filter_append, hUkeep, hFeed,
            List.nil_append, (hIHsub i hi).2.1]
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
          (opEvents sk (.kid (h + 1) k (sk.stageScope (h + 1) k)
              (lastDOf sk (h + 1) k) (sk.wiresBefore (h + 1) k) i F)).filter
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
        · have hUdrop : ((if lastDOf sk (h + 1) k == some i
                then [((upperOut (wpk (h + 1)), true, k) : Ev)]
                else []).filter
              (fun e => evOwner sk e == mF)) = [] := by
            split
            · rw [List.filter_cons_of_neg (by
                simp only [evOwner_upperOut, beq_iff_eq]; omega),
                List.filter_nil]
            · rfl
          have hSubDrop : (opEvents sk (.scope h
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
            List.filter_append, List.filter_append, hUdrop, hFeedKeep,
            hSubDrop, List.nil_append, List.append_nil]
        · have hDf : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
              = false := by simpa using hD
          have hSubDrop : (opEvents sk (.scope h
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
      -- (C) each kid's descendant-stage filter is its subtree's run
      have hkidDesc : ∀ h', h' ≤ h → ∀ i ∈ List.range
            (sk.nChildren (h + 1) (sk.stageScope (h + 1) k)),
          (opEvents sk (.kid (h + 1) k (sk.stageScope (h + 1) k)
              (lastDOf sk (h + 1) k) (sk.wiresBefore (h + 1) k) i F)).filter
            (fun e => evOwner sk e == walkIdx sk h')
          = walkSeg sk h'
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
        · have hUdrop : ((if lastDOf sk (h + 1) k == some i
                then [((upperOut (wpk (h + 1)), true, k) : Ev)]
                else []).filter
              (fun e => evOwner sk e == walkIdx sk h')) = [] := by
            split
            · rw [List.filter_cons_of_neg (by
                simp only [evOwner_upperOut, beq_iff_eq]; omega),
                List.filter_nil]
            · rfl
          rw [if_pos hD,
            List.filter_cons_of_neg (by
              simp only [evOwner_wireOut sk hh, beq_iff_eq]; omega),
            List.filter_cons_of_neg (by
              simp only [evOwner_lowerOut, beq_iff_eq]; omega),
            List.filter_append, List.filter_append, hUdrop, hFeedDrop,
            List.nil_append, List.nil_append,
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
        · have ho : evOwner sk e = walkIdx sk (h + 1) := by
            revert he
            split
            · intro he
              rcases he with _ | ⟨_, he⟩
              · exact evOwner_upperOut sk k
              · cases he
            · intro he
              cases he
          exact Or.inr ⟨h + 1, Nat.le_refl _, ho⟩
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
            · have ho : evOwner sk e = walkIdx sk (h + 1) := by
                revert hei
                split
                · intro hei
                  rcases hei with _ | ⟨_, hei⟩
                  · exact evOwner_upperOut sk k
                  · cases hei
                · intro hei
                  cases hei
              exact Or.inr ⟨h + 1, Nat.le_refl _, ho⟩
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
      · -- (2) the feeder's filter is the feed
        rw [hE,
          List.filter_cons_of_neg (by
            simp only [evOwner_wireIn sk hwf, beq_iff_eq]; omega),
          List.filter_cons_of_neg (by
            simp only [evOwner_askedIn, beq_iff_eq]; omega),
          List.filter_append]
        have hUdropT : ((if lastDOf sk (h + 1) k == none
              then [((upperOut (wpk (h + 1)), true, k) : Ev)]
              else []).filter
            (fun e => evOwner sk e == mF)) = [] := by
          split
          · rw [List.filter_cons_of_neg (by
              simp only [evOwner_upperOut, beq_iff_eq]; omega),
              List.filter_nil]
          · rfl
        rw [hUdropT, List.nil_append]
        simp only [List.filter_flatMap]
        rw [flatMap_congr hkidMF, ← hF]
        exact flatMap_getElem?_toList F
      · -- (1) each covered walk's filter is its run
        intro h' hle
        rcases Nat.eq_or_lt_of_le hle with heq | hlt
        · -- own stage: the scope block, queries respliced
          subst heq
          rw [Nat.sub_self, descIdx_zero, descIdx_zero, walkSeg_single]
          have hUkeepT : ((if lastDOf sk (h + 1) k == none
                then [((upperOut (wpk (h + 1)), true, k) : Ev)]
                else []).filter
              (fun e => evOwner sk e == walkIdx sk (h + 1)))
              = (if lastDOf sk (h + 1) k == none
                then [((upperOut (wpk (h + 1)), true, k) : Ev)]
                else []) := by
            split
            · rw [List.filter_cons_of_pos (by
                simp only [evOwner_upperOut, beq_self_eq_true]),
                List.filter_nil]
            · rfl
          rw [hE,
            List.filter_cons_of_pos (by
              simp only [evOwner_wireIn sk hwf, beq_self_eq_true]),
            List.filter_cons_of_pos (by
              simp only [evOwner_askedIn, beq_self_eq_true]),
            List.filter_append, hUkeepT]
          simp only [List.filter_flatMap]
          rw [flatMap_congr hkidOwn, scopeBlock, scopeSends_eq]
        · -- descendant stage: glue the kid runs
          have hle' : h' ≤ h := by omega
          have hwlt := walkIdx_lt sk hlt hh
          have hUdropT : ((if lastDOf sk (h + 1) k == none
                then [((upperOut (wpk (h + 1)), true, k) : Ev)]
                else []).filter
              (fun e => evOwner sk e == walkIdx sk h')) = [] := by
            split
            · rw [List.filter_cons_of_neg (by
                simp only [evOwner_upperOut, beq_iff_eq]; omega),
                List.filter_nil]
            · rfl
          rw [hE,
            List.filter_cons_of_neg (by
              simp only [evOwner_wireIn sk hwf, beq_iff_eq]; omega),
            List.filter_cons_of_neg (by
              simp only [evOwner_askedIn, beq_iff_eq]; omega),
            List.filter_append, hUdropT, List.nil_append]
          simp only [List.filter_flatMap]
          rw [flatMap_congr (hkidDesc h' hle'),
            walkSeg_glue_range sk h'
              (fun i => descIdx sk h' (h - h')
                (sk.wiresBefore (h + 1) k + i))
              (fun i => descIdx_mono sk h' (h - h') (by omega))
              (sk.nChildren (h + 1) (sk.stageScope (h + 1) k)),
            show h + 1 - h' = (h - h') + 1 from by omega, descIdx_succ,
            descIdx_succ, show h' + (h - h') + 1 = h + 1 from by omega,
            wiresBefore_succ sk hk, Nat.add_zero]

-- ==================================================== the top assembly
-- Instantiate the master induction at the root scope op and read off
-- `weaveState_wcount`'s two hypotheses: the openers cover iopen and
-- ropen's head, clause 2 returns ropen's root queries, and clause 1
-- covers each whole stage via the telescope endpoints.

/-- A whole-stage run is the stage's walk trace. -/
theorem walkSeg_full (h' : Nat) :
    walkSeg sk h' 0 (sk.stageLen h') = walkEvents sk (wpk h') := by
  unfold walkSeg walkEvents
  rw [Nat.sub_zero, ← List.range_eq_range']
  rfl

private theorem filter_owner_all (l : List Ev) (m : Nat)
    (hall : ∀ e ∈ l, evOwner sk e = m) :
    l.filter (fun e => evOwner sk e == m) = l := by
  rw [List.filter_eq_self]
  intro a ha
  simp only [hall a ha, beq_self_eq_true]

private theorem filter_owner_none (l : List Ev) {m m' : Nat}
    (hall : ∀ e ∈ l, evOwner sk e = m) (hne : m ≠ m') :
    l.filter (fun e => evOwner sk e == m') = [] := by
  rw [List.filter_eq_nil_iff]
  intro a ha
  simp only [hall a ha, beq_iff_eq]
  exact hne

/-- iopen's events belong to `procs` slot 0. -/
private theorem iopen_owner (hwf : sk.wellFormed = true) :
    ∀ e ∈ iopenEvents sk, evOwner sk e = 0 := by
  have hge := (wf_rootH hwf).2
  intro e he
  rcases he with _ | ⟨_, he⟩
  · show sndOwner sk (Chan.wire Party.I sk.rootH) = 0
    simp [sndOwner]
  rcases he with _ | ⟨_, he⟩
  · show sndOwner sk (Chan.asked Party.I (sk.rootH - 1)) = 0
    have h11 : sk.rootH - 1 + 1 = sk.rootH := by omega
    simp [sndOwner, h11]
  · cases he

/-- ropen's events belong to `procs` slot 1. -/
theorem ropen_owner (hwf : sk.wellFormed = true) :
    ∀ e ∈ ropenEvents sk, evOwner sk e = 1 := by
  have hge := (wf_rootH hwf).2
  intro e he
  rcases he with _ | ⟨_, he⟩
  · show rcvOwner sk (Chan.wire Party.I sk.rootH) = 1
    simp [rcvOwner]
  rcases he with _ | ⟨_, he⟩
  · show sndOwner sk (Chan.wire Party.R sk.rootH) = 1
    simp [sndOwner]
  rcases he with _ | ⟨_, he⟩
  · rfl
  · obtain ⟨j, -, rfl⟩ := List.mem_map.1 he
    show sndOwner sk (Chan.asked Party.R (sk.rootH - 2)) = 1
    have h22 : sk.rootH - 2 + 2 = sk.rootH := by omega
    simp [sndOwner, h22]

/-- The opening worklist's fuel-free events: the openers, then the
root scope's subtree. -/
theorem weave_flatMap :
    (weaveOps sk).flatMap (opEvents sk)
      = (iopenEvents sk ++ (ropenEvents sk).take 3)
        ++ opEvents sk
            (.scope (sk.rootH - 1) 0 ((ropenEvents sk).drop 3)) := by
  unfold weaveOps
  rw [List.flatMap_append, List.flatMap_map, List.flatMap_singleton]
  have hemit : (fun e => opEvents sk (WOp.emit e)) = fun e : Ev => [e] :=
    funext fun e => opEvents_emit sk e
  rw [hemit, List.flatMap_singleton']

/-- THE INITIAL ALIGNMENT (PROGRESS.md §7 3b): the opening worklist's
future events have in-range owners, and their per-owner filters ARE
the manual traces — `weaveState_wcount`'s two hypotheses, discharged
from the master induction at the root scope op. -/
theorem weave_initial_alignment (hwf : sk.wellFormed = true) :
    (∀ e ∈ (weaveOps sk).flatMap (opEvents sk),
        evOwner sk e < manCount sk)
      ∧ manFilters sk ((weaveOps sk).flatMap (opEvents sk))
        = (procs sk).take (manCount sk) := by
  have hge := (wf_rootH hwf).2
  have hlen1 := wf_stageLen_top sk hwf
  have hss := wf_stageScope_top sk hwf
  have hF : ((ropenEvents sk).drop 3).length
      = sk.nChildren (sk.rootH - 1) (sk.stageScope (sk.rootH - 1) 0) := by
    rw [hss, nChildren_of_pos sk (by omega)]
    simp [ropenEvents, Skel.rootPending]
  have hFo : ∀ e ∈ (ropenEvents sk).drop 3, evOwner sk e = 1 :=
    fun e he => ropen_owner sk hwf e (List.mem_of_mem_drop he)
  obtain ⟨hown3, hfeed2, hwalk1⟩ := align_scope sk hwf (sk.rootH - 1) 0
    ((ropenEvents sk).drop 3) 1 (by omega) (by omega) hF hFo
    (by unfold walkIdx; omega)
  have htk3 : ∀ e ∈ (ropenEvents sk).take 3, evOwner sk e = 1 :=
    fun e he => ropen_owner sk hwf e (List.mem_of_mem_take he)
  have hio := iopen_owner sk hwf
  constructor
  · -- owners in range
    intro e he
    rw [weave_flatMap] at he
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
  · -- per-owner filters are the manual traces
    have hrange : List.range (manCount sk)
        = [0, 1] ++ List.range' 2 sk.rootH := by
      have happ := List.range'_append
        (s := 0) (m := 2) (n := sk.rootH) (step := 1)
      rw [show 0 + 1 * 2 = 2 from by omega] at happ
      rw [manCount, List.range_eq_range', ← happ]
      rfl
    have htake : (procs sk).take (manCount sk)
        = [iopenEvents sk, ropenEvents sk]
          ++ ((List.range sk.rootH).map fun i =>
              walkEvents sk (wpk (sk.rootH - 1 - i))) := by
      have hsplit : procs sk
          = ([iopenEvents sk, ropenEvents sk]
              ++ ((List.range sk.rootH).map fun i =>
                  walkEvents sk (wpk (sk.rootH - 1 - i))))
            ++ ([absorbEvents sk]
              ++ sk.asmKeys.map (asmEvents sk)
              ++ [[(Chan.rootret, false, 0)], finEvents sk]) := by
        simp [procs, wpk, List.append_assoc, Function.comp]
      rw [hsplit]
      refine List.take_left' ?_
      simp [manCount]
      omega
    rw [weave_flatMap, htake]
    unfold manFilters
    rw [hrange, List.map_append]
    have h0 : ((iopenEvents sk ++ (ropenEvents sk).take 3)
        ++ opEvents sk
            (.scope (sk.rootH - 1) 0 ((ropenEvents sk).drop 3))).filter
        (fun e => evOwner sk e == 0) = iopenEvents sk := by
      have hs0 : (opEvents sk (.scope (sk.rootH - 1) 0
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
        ++ opEvents sk
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
          ++ opEvents sk
              (.scope (sk.rootH - 1) 0 ((ropenEvents sk).drop 3))).filter
          (fun e => evOwner sk e == 2 + i)
        = walkEvents sk (wpk (sk.rootH - 1 - i))
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
      rw [hend, walkSeg_full] at hseg
      rw [List.filter_append, List.filter_append,
        filter_owner_none sk _ hio (by omega),
        filter_owner_none sk _ htk3 (by omega), hseg,
        List.nil_append, List.nil_append]

-- ============================================ fuel, discharged

/-- Per-owner filters partition an in-range future: the filter
lengths sum to the future's length. -/
private theorem manFilters_length_sum :
    ∀ (fut : List Ev), (∀ e ∈ fut, evOwner sk e < manCount sk) →
      ((manFilters sk fut).map List.length).sum = fut.length := by
  intro fut
  induction fut with
  | nil =>
      intro _
      unfold manFilters
      induction List.range (manCount sk) with
      | nil => rfl
      | cons m ms ih => simpa using ih
  | cons e fut ih =>
      intro hall
      obtain ⟨A, r, B, hcons, hprev⟩ := manFilters_cons sk fut
        (hall e (List.mem_cons_self ..))
      have hsum := ih fun e' he' => hall e' (List.mem_cons_of_mem _ he')
      rw [hprev] at hsum
      rw [hcons]
      simp only [List.map_append, List.map_cons, List.sum_append,
        List.sum_cons, List.length_cons] at hsum ⊢
      omega

/-- The opening worklist's emission count is bounded by the event
total: the emissions are exactly the manual traces' events, a prefix
of `procs`. This is `goEvents_weave`'s missing hypothesis — the
weave's fuel is sufficient. -/
theorem weave_events_length (hwf : sk.wellFormed = true) :
    ((weaveOps sk).flatMap (opEvents sk)).length ≤ totalEvents sk := by
  obtain ⟨hown, halign⟩ := weave_initial_alignment sk hwf
  have hsum := manFilters_length_sum sk _ hown
  rw [halign] at hsum
  have htot : totalEvents sk
      = (((procs sk).take (manCount sk)).map List.length).sum
        + (((procs sk).drop (manCount sk)).map List.length).sum := by
    unfold totalEvents
    conv => lhs; rw [← List.take_append_drop (manCount sk) (procs sk)]
    rw [List.map_append, List.sum_append]
  omega

/-- The weave state satisfies the counting invariant, hypothesis-free:
the initial alignment discharges `weaveState_wcount`. The weave's
output is a permutation of the manual traces' events riding on the
pump traces — the completeness witness's permutation half, closed. -/
theorem weave_wcount (hwf : sk.wellFormed = true) :
    WCount sk [] (weaveState sk) := by
  have hgo := goEvents_weave sk (weave_events_length sk hwf)
  obtain ⟨hown, halign⟩ := weave_initial_alignment sk hwf
  exact weaveState_wcount sk (by rw [hgo]; exact halign)
    (by rw [hgo]; exact hown)

-- ================================================ kid-suffix partition
-- The tail-partition family (PROGRESS.md §7 3b (f), RestCtx): at a
-- mid-scope worklist position — kid slots `i..n` of scope `(h, k)`
-- still unwoven — the remaining events partition per stage exactly as
-- `align_scope` partitions the whole subtree: the feeder's filter is
-- the unconsumed feed suffix, the own stage's is the remaining spliced
-- chunks, and each deeper stage's is a `descIdx` window suffix. Layer
-- D reads these through `WCount.man_struct` to pin every walk-owned
-- channel count at the position. No induction is needed here: each
-- unwoven kid's subtree is a whole scope, covered by `align_scope`.

private theorem drop_eq_flatMap_getElem? {α : Type _} :
    ∀ (m i : Nat) (F : List α), F.length = i + m →
      (List.range' i m).flatMap (fun j => F[j]?.toList) = F.drop i := by
  intro m
  induction m with
  | zero =>
      intro i F hlen
      rw [show i = F.length from by omega, List.drop_length]
      rfl
  | succ m ihm =>
      intro i F hlen
      have hilt : i < F.length := by omega
      rw [List.range'_succ, List.flatMap_cons, ihm (i + 1) F (by omega),
        List.getElem?_eq_getElem hilt, Option.toList_some,
        List.singleton_append]
      exact (List.drop_eq_getElem_cons hilt).symm

private theorem walkSeg_glue_range' (h' : Nat) (g : Nat → Nat)
    (hmono : ∀ i, g i ≤ g (i + 1)) :
    ∀ (m i : Nat),
      (List.range' i m).flatMap
          (fun j => walkSeg sk h' (g j) (g (j + 1)))
        = walkSeg sk h' (g i) (g (i + m)) := by
  have hchain : ∀ (d a : Nat), g a ≤ g (a + d) := by
    intro d
    induction d with
    | zero => intro a; exact Nat.le_refl _
    | succ d ihd => intro a; exact Nat.le_trans (ihd a) (hmono (a + d))
  intro m
  induction m with
  | zero =>
      intro i
      rw [Nat.add_zero]
      exact (walkSeg_empty sk h' (g i)).symm ▸ rfl
  | succ m ihm =>
      intro i
      rw [List.range'_succ, List.flatMap_cons, ihm (i + 1),
        walkSeg_glue sk (hmono i) (hchain m (i + 1)),
        show i + 1 + m = i + (m + 1) from by omega]

/-- One kid op's per-stage filters: the ownership cover, the feeder's
query, the own-stage spliced chunk, and the descendant windows. -/
private theorem kid_filters (hwf : sk.wellFormed = true)
    {h k : Nat} (hh : h < sk.rootH) (hk : k < sk.stageLen h)
    {F : List Ev} {mF : Nat}
    (hF : F.length = sk.nChildren h (sk.stageScope h k))
    (hFo : ∀ e ∈ F, evOwner sk e = mF)
    (hmF : mF < walkIdx sk h)
    {i : Nat} (hi : i < sk.nChildren h (sk.stageScope h k)) :
    (∀ e ∈ opEvents sk (.kid h k (sk.stageScope h k) (lastDOf sk h k)
        (sk.wiresBefore h k) i F),
      evOwner sk e = mF ∨ ∃ h', h' ≤ h ∧ evOwner sk e = walkIdx sk h')
    ∧ (opEvents sk (.kid h k (sk.stageScope h k) (lastDOf sk h k)
        (sk.wiresBefore h k) i F)).filter
          (fun e => evOwner sk e == mF) = F[i]?.toList
    ∧ (opEvents sk (.kid h k (sk.stageScope h k) (lastDOf sk h k)
        (sk.wiresBefore h k) i F)).filter
          (fun e => evOwner sk e == walkIdx sk h)
        = splicedChunk sk h k (lastDOf sk h k) i
    ∧ ∀ h', h' < h →
        (opEvents sk (.kid h k (sk.stageScope h k) (lastDOf sk h k)
            (sk.wiresBefore h k) i F)).filter
          (fun e => evOwner sk e == walkIdx sk h')
        = walkSeg sk h'
            (descIdx sk h' (h - 1 - h') (sk.wiresBefore h k + i))
            (descIdx sk h' (h - 1 - h')
              (sk.wiresBefore h k + (i + 1))) := by
  cases h with
  | zero =>
      have hD0 : sk.childIsD 0 (sk.stageScope 0 k) i = false := rfl
      have hkidE : opEvents sk (.kid 0 k (sk.stageScope 0 k)
            (lastDOf sk 0 k) (sk.wiresBefore 0 k) i F)
          = (wireOut (wpk 0), true, sk.wiresBefore 0 k + i)
              :: F[i]?.toList := by
        rw [opEvents_kid_eq,
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
          splicedChunk,
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
      have hsub : sk.wiresBefore (h + 1) k + i < sk.stageLen h := by
        have htot := wiresBefore_total sk hwf h1 hh
        simp only [Nat.add_sub_cancel] at htot
        have hmono := wiresBefore_mono sk (h + 1)
          (show k + 1 ≤ sk.stageLen (h + 1) from hk)
        have hstep := wiresBefore_succ sk hk
        omega
      have hmF' : walkIdx sk (h + 1) < walkIdx sk h :=
        walkIdx_lt sk (Nat.lt_succ_self h) hh
      have hkidE : opEvents sk (.kid (h + 1) k (sk.stageScope (h + 1) k)
            (lastDOf sk (h + 1) k) (sk.wiresBefore (h + 1) k) i F)
          = (wireOut (wpk (h + 1)), true, sk.wiresBefore (h + 1) k + i)
              :: (if sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i then
                    (lowerOut (wpk (h + 1)), true,
                        sk.dsBefore (h + 1) k + dRank sk (wpk (h + 1)) k i)
                      :: ((if lastDOf sk (h + 1) k == some i
                            then [((upperOut (wpk (h + 1)), true, k) : Ev)]
                            else [])
                        ++ (F[i]?.toList
                          ++ opEvents sk (.scope h
                              (sk.wiresBefore (h + 1) k + i)
                              (chunkQ sk (h + 1) k i))))
                  else F[i]?.toList
                    ++ opEvents sk (.scope h
                        (sk.wiresBefore (h + 1) k + i) [])) := by
        rw [opEvents_kid_eq]
        simp only [Nat.add_sub_cancel,
          show ((h + 1 : Nat) == 0) = false from rfl, Bool.false_eq_true,
          if_false]
      have hIHsub := align_scope sk hwf h (sk.wiresBefore (h + 1) k + i)
        (chunkQ sk (h + 1) k i) (walkIdx sk (h + 1)) hh' hsub
        (by
          have hq := qCount_eq_kid_nChildren sk hwf h1 hh hk hi
          simp only [Nat.add_sub_cancel] at hq
          rw [chunkQ_length, hq])
        (chunkQ_owner sk h1 hh k i) hmF'
      have hIHW := fun
          (hDf : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
            = false) =>
        align_scope sk hwf h (sk.wiresBefore (h + 1) k + i) []
          (walkIdx sk (h + 1)) hh' hsub
          (by
            have hz := nChildren_kid_notD sk hwf h1 hh hk hi hDf
            simp only [Nat.add_sub_cancel] at hz
            rw [List.length_nil, hz])
          (fun e he => absurd he (by simp)) hmF'
      have hFeedDropW : (F[i]?.toList).filter
          (fun e => evOwner sk e == walkIdx sk (h + 1)) = [] := by
        cases hfi : F[i]? with
        | none => rfl
        | some q =>
            rw [Option.toList_some,
              List.filter_cons_of_neg (by
                simp only [hFo q (List.mem_of_getElem? hfi), beq_iff_eq]
                omega),
              List.filter_nil]
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
      refine ⟨?_, ?_, ?_, ?_⟩
      · -- ownership cover
        intro e he
        rw [hkidE] at he
        rcases he with _ | ⟨_, he⟩
        · exact Or.inr ⟨h + 1, Nat.le_refl _, evOwner_wireOut sk hh _⟩
        by_cases hD : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
        · rw [if_pos hD] at he
          rcases he with _ | ⟨_, he⟩
          · exact Or.inr ⟨h + 1, Nat.le_refl _, evOwner_lowerOut sk _⟩
          rcases List.mem_append.1 he with he | he
          · have ho : evOwner sk e = walkIdx sk (h + 1) := by
              revert he
              split
              · intro he
                rcases he with _ | ⟨_, he⟩
                · exact evOwner_upperOut sk k
                · cases he
              · intro he
                cases he
            exact Or.inr ⟨h + 1, Nat.le_refl _, ho⟩
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
      · -- the feeder's filter is the feed query
        rw [hkidE]
        by_cases hD : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
        · have hUdrop : ((if lastDOf sk (h + 1) k == some i
                then [((upperOut (wpk (h + 1)), true, k) : Ev)]
                else []).filter
              (fun e => evOwner sk e == mF)) = [] := by
            split
            · rw [List.filter_cons_of_neg (by
                simp only [evOwner_upperOut, beq_iff_eq]; omega),
                List.filter_nil]
            · rfl
          have hSubDrop : (opEvents sk (.scope h
                (sk.wiresBefore (h + 1) k + i)
                (chunkQ sk (h + 1) k i))).filter
              (fun e => evOwner sk e == mF) = [] := by
            rw [List.filter_eq_nil_iff]
            intro e he
            rcases hIHsub.1 e he with ho | ⟨h'', hle'', ho⟩
            · simp only [ho, beq_iff_eq]
              omega
            · have hwlt := walkIdx_lt sk
                (show h'' < h + 1 from by omega) hh
              simp only [ho, beq_iff_eq]
              omega
          rw [if_pos hD,
            List.filter_cons_of_neg (by
              simp only [evOwner_wireOut sk hh, beq_iff_eq]; omega),
            List.filter_cons_of_neg (by
              simp only [evOwner_lowerOut, beq_iff_eq]; omega),
            List.filter_append, List.filter_append, hUdrop, hFeedKeep,
            hSubDrop, List.nil_append, List.append_nil]
        · have hDf : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
              = false := by simpa using hD
          have hSubDrop : (opEvents sk (.scope h
                (sk.wiresBefore (h + 1) k + i) [])).filter
              (fun e => evOwner sk e == mF) = [] := by
            rw [List.filter_eq_nil_iff]
            intro e he
            rcases (hIHW hDf).1 e he with ho | ⟨h'', hle'', ho⟩
            · simp only [ho, beq_iff_eq]
              omega
            · have hwlt := walkIdx_lt sk
                (show h'' < h + 1 from by omega) hh
              simp only [ho, beq_iff_eq]
              omega
          rw [if_neg hD,
            List.filter_cons_of_neg (by
              simp only [evOwner_wireOut sk hh, beq_iff_eq]; omega),
            List.filter_append, hFeedKeep, hSubDrop, List.append_nil]
      · -- the own-stage filter is the spliced chunk
        rw [hkidE, splicedChunk]
        by_cases hD : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
        · have hUkeep : ((if lastDOf sk (h + 1) k == some i
                then [((upperOut (wpk (h + 1)), true, k) : Ev)]
                else []).filter
              (fun e => evOwner sk e == walkIdx sk (h + 1)))
              = (if lastDOf sk (h + 1) k == some i
                then [((upperOut (wpk (h + 1)), true, k) : Ev)]
                else []) := by
            split
            · rw [List.filter_cons_of_pos (by
                simp only [evOwner_upperOut, beq_self_eq_true]),
                List.filter_nil]
            · rfl
          rw [if_pos hD, if_pos hD,
            List.filter_cons_of_pos (by
              simp only [evOwner_wireOut sk hh, beq_self_eq_true]),
            List.filter_cons_of_pos (by
              simp only [evOwner_lowerOut, beq_self_eq_true]),
            List.filter_append, List.filter_append, hUkeep, hFeedDropW,
            List.nil_append, hIHsub.2.1]
        · have hDf : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
              = false := by simpa using hD
          rw [if_neg hD, if_neg hD,
            List.filter_cons_of_pos (by
              simp only [evOwner_wireOut sk hh, beq_self_eq_true]),
            List.filter_append, hFeedDropW, List.nil_append,
            (hIHW hDf).2.1]
      · -- the descendant-stage filter is the subtree's run
        intro h' hlt
        have hle : h' ≤ h := by omega
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
        rw [hkidE, show h + 1 - 1 - h' = h - h' from by omega]
        by_cases hD : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
        · have hUdrop : ((if lastDOf sk (h + 1) k == some i
                then [((upperOut (wpk (h + 1)), true, k) : Ev)]
                else []).filter
              (fun e => evOwner sk e == walkIdx sk h')) = [] := by
            split
            · rw [List.filter_cons_of_neg (by
                simp only [evOwner_upperOut, beq_iff_eq]; omega),
                List.filter_nil]
            · rfl
          rw [if_pos hD,
            List.filter_cons_of_neg (by
              simp only [evOwner_wireOut sk hh, beq_iff_eq]; omega),
            List.filter_cons_of_neg (by
              simp only [evOwner_lowerOut, beq_iff_eq]; omega),
            List.filter_append, List.filter_append, hUdrop, hFeedDrop,
            List.nil_append, List.nil_append,
            hIHsub.2.2 h' hle, Nat.add_assoc]
        · have hDf : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
              = false := by simpa using hD
          rw [if_neg hD,
            List.filter_cons_of_neg (by
              simp only [evOwner_wireOut sk hh, beq_iff_eq]; omega),
            List.filter_append, hFeedDrop, List.nil_append,
            (hIHW hDf).2.2 h' hle, Nat.add_assoc]

/-- THE KID-SUFFIX PARTITION (RestCtx): a mid-scope position's
unwoven kid slots partition per stage like the whole subtree.

Under `align_scope`'s feed contract, the events of kid slots
`i..nChildren` of scope `(h, k)`: (3) are each the feeder's or a
covered walk's; (2) filter to the unconsumed feed suffix at the
feeder; (1) filter to the remaining spliced chunks at the own stage
and to the `descIdx` window suffix `[descIdx (wiresBefore h k + i),
descIdx (wiresBefore h (k+1)))` at each deeper stage. Layer D reads
the emitted-prefix counts off these through `WCount.man_struct`. -/
theorem align_kids_suffix (hwf : sk.wellFormed = true)
    {h k : Nat} (hh : h < sk.rootH) (hk : k < sk.stageLen h)
    {F : List Ev} {mF : Nat}
    (hF : F.length = sk.nChildren h (sk.stageScope h k))
    (hFo : ∀ e ∈ F, evOwner sk e = mF)
    (hmF : mF < walkIdx sk h)
    {i : Nat} (hi : i ≤ sk.nChildren h (sk.stageScope h k)) :
    (∀ e ∈ (List.range' i (sk.nChildren h (sk.stageScope h k) - i)).flatMap
        (fun j => opEvents sk (.kid h k (sk.stageScope h k)
          (lastDOf sk h k) (sk.wiresBefore h k) j F)),
      evOwner sk e = mF ∨ ∃ h', h' ≤ h ∧ evOwner sk e = walkIdx sk h')
    ∧ ((List.range' i (sk.nChildren h (sk.stageScope h k) - i)).flatMap
        (fun j => opEvents sk (.kid h k (sk.stageScope h k)
          (lastDOf sk h k) (sk.wiresBefore h k) j F))).filter
          (fun e => evOwner sk e == mF) = F.drop i
    ∧ ((List.range' i (sk.nChildren h (sk.stageScope h k) - i)).flatMap
        (fun j => opEvents sk (.kid h k (sk.stageScope h k)
          (lastDOf sk h k) (sk.wiresBefore h k) j F))).filter
          (fun e => evOwner sk e == walkIdx sk h)
        = (List.range' i (sk.nChildren h (sk.stageScope h k) - i)).flatMap
            (splicedChunk sk h k (lastDOf sk h k))
    ∧ ∀ h', h' < h →
        ((List.range' i (sk.nChildren h (sk.stageScope h k) - i)).flatMap
            (fun j => opEvents sk (.kid h k (sk.stageScope h k)
              (lastDOf sk h k) (sk.wiresBefore h k) j F))).filter
          (fun e => evOwner sk e == walkIdx sk h')
        = walkSeg sk h'
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
    exact (kid_filters sk hwf hh hk hF hFo hmF (hjlt j hj)).1 e hej
  · simp only [List.filter_flatMap]
    rw [flatMap_congr (fun j hj =>
      (kid_filters sk hwf hh hk hF hFo hmF (hjlt j hj)).2.1)]
    exact drop_eq_flatMap_getElem?
      (sk.nChildren h (sk.stageScope h k) - i) i F (by omega)
  · simp only [List.filter_flatMap]
    exact flatMap_congr (fun j hj =>
      (kid_filters sk hwf hh hk hF hFo hmF (hjlt j hj)).2.2.1)
  · intro h' hlt
    simp only [List.filter_flatMap]
    rw [flatMap_congr (fun j hj =>
        (kid_filters sk hwf hh hk hF hFo hmF (hjlt j hj)).2.2.2 h' hlt),
      walkSeg_glue_range' sk h'
        (fun j => descIdx sk h' (h - 1 - h') (sk.wiresBefore h k + j))
        (fun j => descIdx_mono sk h' (h - 1 - h') (by omega))
        (sk.nChildren h (sk.stageScope h k) - i) i,
      show i + (sk.nChildren h (sk.stageScope h k) - i)
        = sk.nChildren h (sk.stageScope h k) from by omega]

end StreamingMirror.Sched
