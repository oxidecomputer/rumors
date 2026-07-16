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

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ================================================ statement vocabulary

/-- The walk key of stage `h` (initiator on odd stages), as the weave
and `procs` both spell it. -/
def wpk (h : Nat) : Party × Nat :=
  (if h % 2 == 1 then Party.I else Party.R, h)

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

end StreamingMirror.Sched
