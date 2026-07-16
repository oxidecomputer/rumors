/-
The canonical per-channel numbering layer (PROGRESS.md §7 item 3a):
on every channel-side, the trace family's events are the seqs
0, 1, 2, … in order, and they live in exactly one trace. Combined with
the merge invariant's provenance (`out_count`) and trace monotonicity
(`rem_struct`), this upgrades the counted guard-history facts of
Proofs/Sched.lean to positional ones — the schedule's n-th send on a
channel IS `snd(c,n)` — yielding `snd(c,n)` precedes `rcv(c,n)`
(`schedule_e1_pos`) and τ injectivity (`schedule_inj`).

# Validated before proved

`lake exe eventdag` checks exactly these claims — canon-shaped
per-trace projections, one producer/consumer per channel-side, and the
canon shape of the merged schedule's own projections — on every pin
and every acyclic fuzz seed (`numberingErrs`), per the
validate-then-prove discipline.

# Shape of the argument

- `proj c b l` filters a list to one channel-side; `seg`/`canon` name
  the target shapes (consecutive seqs from an offset / from zero).
- Per trace: every block's projection is a `seg` whose offset is a
  `Skel` prefix sum, so the trace's projection folds to `canon` by
  `proj_flatMap_canon` — the prefix-sum telescopes (`wiresBefore_succ`
  &c.) supply the step. The parent splice is projection-invisible
  (`proj_scopeSends`): the parent is the sole `upperOut` event and
  everything else keeps its relative order.
- Across traces: `Owned` assigns every event's channel-side an owner
  index and each trace proves its own events point at itself, making
  "two traces on one channel-side" a Nat contradiction rather than a
  quadratic disjointness sweep.
-/
import StreamingMirror.Proofs.Sched

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ==================================================== the projections

/-- Side-`b` events on channel `c`, in order: the subsequence the
counted guards measure (`sndCount`/`rcvCount` are its lengths on the
two sides). -/
def proj (c : Chan) (b : Bool) (l : List Ev) : List Ev :=
  l.filter fun e => decide (e.1 = c) && (e.2.1 == b)

/-- `n` consecutive side-`b` events on `c` starting at seq `lo` — the
shape of every block-level projection. -/
def seg (c : Chan) (b : Bool) (lo n : Nat) : List Ev :=
  (List.range n).map fun t => (c, b, lo + t)

/-- The canonical projection, seqs `0, 1, …, m-1`: the numbering
layer's claims all end here. -/
def canon (c : Chan) (b : Bool) (m : Nat) : List Ev :=
  (List.range m).map fun j => (c, b, j)

theorem sndCount_eq_proj (c : Chan) (l : List Ev) :
    sndCount c l = (proj c true l).length := by
  unfold sndCount proj
  congr 1
  exact List.filter_congr fun e _ => by cases e.2.1 <;> simp

theorem rcvCount_eq_proj (c : Chan) (l : List Ev) :
    rcvCount c l = (proj c false l).length := by
  unfold rcvCount proj
  congr 1
  exact List.filter_congr fun e _ => by cases e.2.1 <;> simp

-- ================================================= seg/canon algebra

theorem canon_zero (c : Chan) (b : Bool) : canon c b 0 = [] := rfl

theorem seg_zero (c : Chan) (b : Bool) (lo : Nat) : seg c b lo 0 = [] := rfl

theorem seg_one (c : Chan) (b : Bool) (lo : Nat) :
    seg c b lo 1 = [(c, b, lo)] := by
  simp [seg, List.range_succ]

theorem seg_succ (c : Chan) (b : Bool) (lo n : Nat) :
    seg c b lo (n + 1) = seg c b lo n ++ [(c, b, lo + n)] := by
  simp [seg, List.range_succ]

theorem canon_succ (c : Chan) (b : Bool) (m : Nat) :
    canon c b (m + 1) = canon c b m ++ [(c, b, m)] := by
  simp [canon, List.range_succ]

theorem canon_one (c : Chan) (b : Bool) : canon c b 1 = [(c, b, 0)] := by
  simp [canon, List.range_succ]

theorem canon_eq_seg (c : Chan) (b : Bool) (m : Nat) :
    canon c b m = seg c b 0 m := by
  unfold canon seg
  exact List.map_congr_left fun t _ => by rw [Nat.zero_add]

theorem seg_append (c : Chan) (b : Bool) (lo d₁ d₂ : Nat) :
    seg c b lo d₁ ++ seg c b (lo + d₁) d₂ = seg c b lo (d₁ + d₂) := by
  induction d₂ with
  | zero => simp [seg_zero]
  | succ n ih =>
      rw [seg_succ, ← List.append_assoc, ih, Nat.add_assoc, ← seg_succ,
        Nat.add_assoc]

/-- Two abutting segments glue: the flatMap fold's step. -/
private theorem seg_glue (c : Chan) (b : Bool) {x y z : Nat}
    (hxy : x ≤ y) (hyz : y ≤ z) :
    seg c b x (y - x) ++ seg c b y (z - y) = seg c b x (z - x) := by
  have h1 : y = x + (y - x) := by omega
  calc seg c b x (y - x) ++ seg c b y (z - y)
      = seg c b x (y - x) ++ seg c b (x + (y - x)) (z - y) := by rw [← h1]
    _ = seg c b x ((y - x) + (z - y)) := seg_append ..
    _ = seg c b x (z - x) := by congr 1; omega

-- ============================================ proj distribution laws

theorem proj_nil (c : Chan) (b : Bool) : proj c b [] = [] := rfl

theorem proj_append (c : Chan) (b : Bool) (l₁ l₂ : List Ev) :
    proj c b (l₁ ++ l₂) = proj c b l₁ ++ proj c b l₂ :=
  List.filter_append l₁ l₂

theorem proj_cons_self (c : Chan) (b : Bool) (n : Nat) (l : List Ev) :
    proj c b ((c, b, n) :: l) = (c, b, n) :: proj c b l := by
  simp [proj]

theorem proj_cons_ne_chan {c c' : Chan} {b b' : Bool} {n : Nat}
    {l : List Ev} (h : c' ≠ c) :
    proj c b ((c', b', n) :: l) = proj c b l := by
  simp [proj, h]

theorem proj_cons_ne_side {c c' : Chan} {b b' : Bool} {n : Nat}
    {l : List Ev} (h : b' ≠ b) :
    proj c b ((c', b', n) :: l) = proj c b l := by
  simp [proj, h]

/-- Emptiness from support: no event of `l` sits on side `b` of `c`. -/
theorem proj_eq_nil {c : Chan} {b : Bool} {l : List Ev}
    (h : ∀ e ∈ l, e.1 = c → e.2.1 ≠ b) : proj c b l = [] := by
  unfold proj
  rw [List.filter_eq_nil_iff]
  intro e he
  simp only [Bool.and_eq_true, decide_eq_true_eq, beq_iff_eq, not_and]
  exact h e he

theorem proj_seg_self (c : Chan) (b : Bool) (lo n : Nat) :
    proj c b (seg c b lo n) = seg c b lo n := by
  unfold proj seg
  rw [List.filter_map]
  congr 1
  exact List.filter_eq_self.2 fun t _ => by simp [Function.comp]

theorem proj_seg_ne {c c' : Chan} {b b' : Bool} (h : ¬(c' = c ∧ b' = b))
    (lo n : Nat) : proj c b (seg c' b' lo n) = [] := by
  apply proj_eq_nil
  intro e he h1
  obtain ⟨t, -, rfl⟩ := List.mem_map.1 he
  intro h2
  exact h ⟨h1, h2⟩

theorem proj_canon_self (c : Chan) (b : Bool) (m : Nat) :
    proj c b (canon c b m) = canon c b m := by
  rw [canon_eq_seg]; exact proj_seg_self ..

theorem proj_canon_ne {c c' : Chan} {b b' : Bool} (h : ¬(c' = c ∧ b' = b))
    (m : Nat) : proj c b (canon c' b' m) = [] := by
  rw [canon_eq_seg]; exact proj_seg_ne h ..

-- =============================================== the segment folds

private theorem le_of_chain {g : Nat → Nat} :
    ∀ {n : Nat}, (∀ k, k < n → g k ≤ g (k + 1)) → g 0 ≤ g n
  | 0, _ => Nat.le_refl _
  | n + 1, h =>
      Nat.le_trans (le_of_chain fun k hk => h k (Nat.lt_succ_of_lt hk))
        (h n (Nat.lt_succ_self n))

/-- The workhorse: blocks whose projections are consecutive segments
concatenate to one segment, with the prefix-sum `g` supplying each
block's offset. -/
theorem proj_flatMap_seg {f : Nat → List Ev} {g : Nat → Nat} {c : Chan}
    {b : Bool} :
    ∀ (len : Nat),
      (∀ k, k < len → proj c b (f k) = seg c b (g k) (g (k + 1) - g k)) →
      (∀ k, k < len → g k ≤ g (k + 1)) →
      proj c b ((List.range len).flatMap f) = seg c b (g 0) (g len - g 0)
  | 0, _, _ => by simp [proj_nil, seg_zero]
  | n + 1, hseg, hmono => by
      have h0n : g 0 ≤ g n :=
        le_of_chain fun k hk => hmono k (Nat.lt_succ_of_lt hk)
      have hnn : g n ≤ g (n + 1) := hmono n (Nat.lt_succ_self n)
      rw [List.range_succ, List.flatMap_append, proj_append,
        proj_flatMap_seg n (fun k hk => hseg k (Nat.lt_succ_of_lt hk))
          (fun k hk => hmono k (Nat.lt_succ_of_lt hk)),
        List.flatMap_cons, List.flatMap_nil, List.append_nil,
        hseg n (Nat.lt_succ_self n)]
      exact seg_glue c b h0n hnn

/-- Segment fold anchored at zero: the per-trace canonical form. -/
theorem proj_flatMap_canon {f : Nat → List Ev} {g : Nat → Nat} {c : Chan}
    {b : Bool} (len : Nat) (h0 : g 0 = 0)
    (hseg : ∀ k, k < len → proj c b (f k) = seg c b (g k) (g (k + 1) - g k))
    (hmono : ∀ k, k < len → g k ≤ g (k + 1)) :
    proj c b ((List.range len).flatMap f) = canon c b (g len) := by
  rw [proj_flatMap_seg len hseg hmono, canon_eq_seg, h0, Nat.sub_zero]

-- ===================================== the Skel prefix-sum telescopes

private theorem foldl_add_take_succ {α : Type _} (f : α → Nat)
    (l : List α) (d : α) {k : Nat} (hk : k < l.length) :
    (l.take (k + 1)).foldl (fun acc s => acc + f s) 0
      = (l.take k).foldl (fun acc s => acc + f s) 0 + f (l.getD k d) := by
  rw [List.take_add_one, List.getElem?_eq_getElem hk]
  rw [List.foldl_append]
  simp [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hk]

theorem wiresBefore_succ {h k : Nat} (hk : k < sk.stageLen h) :
    sk.wiresBefore h (k + 1)
      = sk.wiresBefore h k + sk.nChildren h (sk.stageScope h k) :=
  foldl_add_take_succ _ _ _ hk

theorem dsBefore_succ {h k : Nat} (hk : k < sk.stageLen h) :
    sk.dsBefore h (k + 1)
      = sk.dsBefore h k + sk.dOf h (sk.stageScope h k) :=
  foldl_add_take_succ _ _ _ hk

theorem qsBefore_succ {h k : Nat} (hk : k < sk.stageLen h) :
    sk.qsBefore h (k + 1)
      = sk.qsBefore h k + sk.qOf h (sk.stageScope h k) :=
  foldl_add_take_succ _ _ _ hk

theorem pendsBefore_succ {p : Party} {j k : Nat}
    (hk : k < (sk.asmResList p j).length) :
    sk.pendsBefore p j (k + 1)
      = sk.pendsBefore p j k + sk.pendAt p j k :=
  foldl_add_take_succ (fun s => s) _ _ hk

-- ============================================== the small trace families
-- Per family: the canon-shape master (every channel-side's projection
-- is an initial segment of the naturals; empty counts, `canon 0`).

private theorem canon_nil {c : Chan} {b : Bool} {l : List Ev}
    (h : ∀ e ∈ l, e.1 = c → e.2.1 ≠ b) : ∃ m, proj c b l = canon c b m :=
  ⟨0, proj_eq_nil h⟩

/-- iopen: two seq-0 sends on distinct channels. -/
theorem iopen_canon (c : Chan) (b : Bool) :
    ∃ m, proj c b (iopenEvents sk) = canon c b m := by
  unfold iopenEvents
  by_cases hb : b = true
  · subst hb
    by_cases h1 : c = Chan.wire Party.I sk.rootH
    · subst h1
      exact ⟨1, by rw [proj_cons_self, proj_cons_ne_chan (by simp),
        proj_nil, canon_one]⟩
    · by_cases h2 : c = Chan.asked Party.I (sk.rootH - 1)
      · subst h2
        exact ⟨1, by rw [proj_cons_ne_chan (fun hh => h1 hh.symm),
          proj_cons_self, proj_nil, canon_one]⟩
      · refine canon_nil fun e he hc _ => ?_
        rcases he with _ | ⟨_, he⟩
        · exact h1 hc.symm
        · rcases he with _ | ⟨_, he⟩
          · exact h2 hc.symm
          · cases he
  · refine canon_nil fun e he _ hcb => ?_
    rcases he with _ | ⟨_, he⟩
    · exact hb hcb.symm
    · rcases he with _ | ⟨_, he⟩
      · exact hb hcb.symm
      · cases he

/-- ropen: three seq-0 singles, then the root queries in canon order. -/
theorem ropen_canon (c : Chan) (b : Bool) :
    ∃ m, proj c b (ropenEvents sk) = canon c b m := by
  unfold ropenEvents
  have hq : ((List.range sk.rootPending).map fun j =>
      ((Chan.asked Party.R (sk.rootH - 2), true, j) : Ev))
      = canon (Chan.asked Party.R (sk.rootH - 2)) true sk.rootPending := rfl
  by_cases hb : b = true
  · subst hb
    by_cases h1 : c = Chan.wire Party.R sk.rootH
    · subst h1
      refine ⟨1, ?_⟩
      rw [proj_cons_ne_chan (by simp), proj_cons_self,
        proj_cons_ne_chan (by simp), hq,
        proj_canon_ne (by simp), canon_one]
    · by_cases h2 : c = Chan.rootres
      · subst h2
        refine ⟨1, ?_⟩
        rw [proj_cons_ne_chan (by simp), proj_cons_ne_chan (by simp),
          proj_cons_self, hq, proj_canon_ne (by simp), canon_one]
      · by_cases h3 : c = Chan.asked Party.R (sk.rootH - 2)
        · subst h3
          refine ⟨sk.rootPending, ?_⟩
          rw [proj_cons_ne_chan (by simp),
            proj_cons_ne_chan (fun hh => h1 hh.symm),
            proj_cons_ne_chan (fun hh => h2 hh.symm), hq, proj_canon_self]
        · refine canon_nil fun e he hc hcb => ?_
          rcases he with _ | ⟨_, he⟩
          · exact Bool.noConfusion hcb
          · rcases he with _ | ⟨_, he⟩
            · exact h1 hc.symm
            · rcases he with _ | ⟨_, he⟩
              · exact h2 hc.symm
              · obtain ⟨j, -, rfl⟩ := List.mem_map.1 he
                exact h3 hc.symm
  · by_cases h1 : c = Chan.wire Party.I sk.rootH
    · subst h1
      have hbf : b = false := by cases b <;> simp_all
      subst hbf
      refine ⟨1, ?_⟩
      rw [proj_cons_self, proj_cons_ne_chan (by simp),
        proj_cons_ne_chan (by simp), hq, proj_canon_ne (by simp), canon_one]
    · refine canon_nil fun e he hc hcb => ?_
      rcases he with _ | ⟨_, he⟩
      · exact h1 hc.symm
      · rcases he with _ | ⟨_, he⟩
        · exact hb hcb.symm
        · rcases he with _ | ⟨_, he⟩
          · exact hb hcb.symm
          · obtain ⟨j, -, rfl⟩ := List.mem_map.1 he
            exact hb hcb.symm

/-- Absorb: three interleaved canon streams, one per channel. -/
theorem absorb_canon (c : Chan) (b : Bool) :
    ∃ m, proj c b (absorbEvents sk) = canon c b m := by
  unfold absorbEvents
  have hone : ∀ k : Nat, k + 1 - k = 1 := fun k => by omega
  by_cases h1 : c = Chan.wire Party.R 0 ∧ b = false
  · obtain ⟨rfl, rfl⟩ := h1
    refine ⟨sk.totalLeafReqs, proj_flatMap_canon (g := fun k => k) _ rfl
      (fun k _ => ?_) (fun k _ => by omega)⟩
    rw [hone, seg_one, proj_cons_self, proj_cons_ne_chan (by simp),
      proj_cons_ne_chan (by simp), proj_nil]
  · by_cases h2 : c = Chan.leafRequests ∧ b = false
    · obtain ⟨rfl, rfl⟩ := h2
      refine ⟨sk.totalLeafReqs, proj_flatMap_canon (g := fun k => k) _ rfl
        (fun k _ => ?_) (fun k _ => by omega)⟩
      rw [hone, seg_one, proj_cons_ne_chan (by simp), proj_cons_self,
        proj_cons_ne_chan (by simp), proj_nil]
    · by_cases h3 : c = Chan.level Party.I 0 ∧ b = true
      · obtain ⟨rfl, rfl⟩ := h3
        refine ⟨sk.totalLeafReqs, proj_flatMap_canon (g := fun k => k) _ rfl
          (fun k _ => ?_) (fun k _ => by omega)⟩
        rw [hone, seg_one, proj_cons_ne_side (by simp),
          proj_cons_ne_side (by simp), proj_cons_self, proj_nil]
      · refine canon_nil fun e he hc hcb => ?_
        obtain ⟨j, -, he⟩ := List.mem_flatMap.1 he
        rcases he with _ | ⟨_, he⟩
        · exact h1 ⟨hc.symm, hcb.symm⟩
        · rcases he with _ | ⟨_, he⟩
          · exact h2 ⟨hc.symm, hcb.symm⟩
          · rcases he with _ | ⟨_, he⟩
            · exact h3 ⟨hc.symm, hcb.symm⟩
            · cases he

/-- fins (sans the floating rootret): the root resolution then the
root returns in canon order. -/
theorem fin_canon (c : Chan) (b : Bool) :
    ∃ m, proj c b (finEvents sk) = canon c b m := by
  unfold finEvents
  have hq : ((List.range sk.rootPending).map fun j =>
      ((Chan.rootrets, false, j) : Ev))
      = canon Chan.rootrets false sk.rootPending := rfl
  by_cases hb : b = false
  · subst hb
    by_cases h1 : c = Chan.rootres
    · subst h1
      exact ⟨1, by rw [proj_cons_self, hq, proj_canon_ne (by simp),
        canon_one]⟩
    · by_cases h2 : c = Chan.rootrets
      · subst h2
        exact ⟨sk.rootPending, by
          rw [proj_cons_ne_chan (by simp), hq, proj_canon_self]⟩
      · refine canon_nil fun e he hc _ => ?_
        rcases he with _ | ⟨_, he⟩
        · exact h1 hc.symm
        · obtain ⟨j, -, rfl⟩ := List.mem_map.1 he
          exact h2 hc.symm
  · refine canon_nil fun e he _ hcb => ?_
    rcases he with _ | ⟨_, he⟩
    · exact hb hcb.symm
    · obtain ⟨j, -, rfl⟩ := List.mem_map.1 he
      exact hb hcb.symm

-- ================================================== the walk family
-- A walk trace is a flatMap of scope blocks; each block's per-channel
-- projection is a segment whose offset is a `Skel` prefix sum, except
-- that the parent splice reorders the sends. `proj_scopeSends` shows
-- the splice is projection-invisible, and everything downstream works
-- on the unspliced parent-first form.

/-- D children among the first `i` of scope `k`'s children at walk
`pk`: the res-seq rank (`childChunk`'s inline counter, named). -/
def dRank (pk : Party × Nat) (k i : Nat) : Nat :=
  ((List.range i).filter fun i' =>
    sk.childIsD pk.2 (sk.stageScope pk.2 k) i').length

/-- Queries owed by the first `i` children of scope `k` at walk `pk`
(`childChunk`'s inline query base, named). -/
def qSum (pk : Party × Nat) (k i : Nat) : Nat :=
  ((List.range i).map fun i' =>
    sk.qCount pk.2 (sk.stageScope pk.2 k) i').sum

theorem dRank_succ (pk : Party × Nat) (k i : Nat) :
    dRank sk pk k (i + 1) = dRank sk pk k i
      + (if sk.childIsD pk.2 (sk.stageScope pk.2 k) i then 1 else 0) := by
  unfold dRank
  rw [List.range_succ, List.filter_append, List.length_append]
  by_cases h : sk.childIsD pk.2 (sk.stageScope pk.2 k) i <;> simp [h]

theorem qSum_succ (pk : Party × Nat) (k i : Nat) :
    qSum sk pk k (i + 1)
      = qSum sk pk k i + sk.qCount pk.2 (sk.stageScope pk.2 k) i := by
  unfold qSum
  rw [List.range_succ, List.map_append, List.sum_append]
  simp

private theorem countP_range_getD {α : Type _} (q : α → Bool) (d : α) :
    ∀ (l : List α) (p : Nat → Bool),
      (∀ i, i < l.length → p i = q (l.getD i d)) →
      (List.range l.length).countP p = l.countP q
  | [], _, _ => rfl
  | a :: l', p, hp => by
      have h0 : p 0 = q a := hp 0 (by simp)
      rw [List.length_cons, List.range_succ_eq_map, List.countP_cons,
        List.countP_map, List.countP_cons,
        countP_range_getD q d l' (p ∘ Nat.succ) fun i hi => by
          simpa using hp (i + 1) (by simpa using Nat.succ_lt_succ hi),
        h0]

/-- Within one scope, the D ranks total the scope's `dOf`: the inner
chunk offsets meet the outer `dsBefore` telescope. -/
private theorem dRank_total (pk : Party × Nat) (k : Nat) :
    dRank sk pk k (sk.nChildren pk.2 (sk.stageScope pk.2 k))
      = sk.dOf pk.2 (sk.stageScope pk.2 k) := by
  unfold dRank Skel.dOf
  cases hh : pk.2 == 0 with
  | true =>
      have h0 : pk.2 = 0 := by simpa using hh
      simp [h0, Skel.childIsD]
  | false =>
      have hn : sk.nChildren pk.2 (sk.stageScope pk.2 k)
          = (sk.scope (sk.stageScope pk.2 k)).kids.length := by
        simp [Skel.nChildren, hh]
      rw [hn]
      simp only [Skel.dCount]
      rw [if_neg fun h => Bool.noConfusion h,
        ← List.countP_eq_length_filter, ← List.countP_eq_length_filter]
      exact countP_range_getD _ 0 _ _ fun i hi => by
        simp [Skel.childIsD, hh, List.getElem?_eq_getElem hi,
          List.getD_eq_getElem?_getD]

private theorem foldl_add_eq_map_sum (f : Nat → Nat) :
    ∀ (l : List Nat) (b : Nat),
      l.foldl (fun acc i => acc + f i) b = b + (l.map f).sum
  | [], b => by simp
  | x :: l, b => by
      rw [List.foldl_cons, foldl_add_eq_map_sum f l (b + f x),
        List.map_cons, List.sum_cons]
      omega

/-- Within one scope, the query counts total the scope's `qOf`: the
inner chunk offsets meet the outer `qsBefore` telescope. -/
private theorem qSum_total (pk : Party × Nat) (k : Nat) :
    qSum sk pk k (sk.nChildren pk.2 (sk.stageScope pk.2 k))
      = sk.qOf pk.2 (sk.stageScope pk.2 k) := by
  unfold qSum Skel.qOf
  rw [foldl_add_eq_map_sum]
  simp

-- channel discrimination within one walk: constructors differ
private theorem lower_ne_wire (pk : Party × Nat) :
    lowerOut pk ≠ wireOut pk := by simp [lowerOut, wireOut]
private theorem wire_ne_lower (pk : Party × Nat) :
    wireOut pk ≠ lowerOut pk := by simp [lowerOut, wireOut]
private theorem asked_ne_wire (pk : Party × Nat) :
    askedOut pk ≠ wireOut pk := by
  unfold askedOut wireOut; split <;> simp
private theorem asked_ne_lower (pk : Party × Nat) :
    askedOut pk ≠ lowerOut pk := by
  unfold askedOut lowerOut; split <;> simp
private theorem wire_ne_upper (pk : Party × Nat) :
    wireOut pk ≠ upperOut pk := by simp [wireOut, upperOut]
private theorem lower_ne_upper (pk : Party × Nat) :
    lowerOut pk ≠ upperOut pk := by simp [lowerOut, upperOut]
private theorem asked_ne_upper (pk : Party × Nat) :
    askedOut pk ≠ upperOut pk := by
  unfold askedOut upperOut; split <;> simp
private theorem wire_ne_asked (pk : Party × Nat) :
    wireOut pk ≠ askedOut pk := by
  unfold askedOut wireOut; split <;> simp
private theorem lower_ne_asked (pk : Party × Nat) :
    lowerOut pk ≠ askedOut pk := by
  unfold askedOut lowerOut; split <;> simp
private theorem askedIn_ne_wireIn (pk : Party × Nat) :
    askedIn pk ≠ wireIn pk := by simp [askedIn, wireIn]

/-- `childIsD` is hard-false at the leaf stage, so a true reading pins
the stage above it. -/
private theorem childIsD_ne_zero {h s i : Nat}
    (hD : sk.childIsD h s i = true) : h ≠ 0 := by
  intro h0; subst h0; simp [Skel.childIsD] at hD

private theorem chunkD (pk : Party × Nat) (k i : Nat)
    (hD : sk.childIsD pk.2 (sk.stageScope pk.2 k) i = true) :
    childChunk sk pk k i
      = (wireOut pk, true, sk.wiresBefore pk.2 k + i)
        :: (lowerOut pk, true, sk.dsBefore pk.2 k + dRank sk pk k i)
        :: seg (askedOut pk) true
            (sk.qsBefore pk.2 k + qSum sk pk k i)
            (sk.qCount pk.2 (sk.stageScope pk.2 k) i) := by
  simp only [childChunk, seg, dRank, qSum, hD, if_true]

private theorem chunkR (pk : Party × Nat) (k i : Nat)
    (hD : sk.childIsD pk.2 (sk.stageScope pk.2 k) i = false) :
    childChunk sk pk k i
      = [(wireOut pk, true, sk.wiresBefore pk.2 k + i)] := by
  simp only [childChunk, hD]
  rfl

private theorem chunk_support (pk : Party × Nat) (k i : Nat) :
    ∀ e ∈ childChunk sk pk k i,
      e.2.1 = true ∧ (e.1 = wireOut pk ∨ e.1 = lowerOut pk
        ∨ (e.1 = askedOut pk ∧ pk.2 ≠ 0)) := by
  intro e he
  cases hD : sk.childIsD pk.2 (sk.stageScope pk.2 k) i with
  | true =>
      have hnz := childIsD_ne_zero sk hD
      rw [chunkD sk pk k i hD] at he
      rcases he with _ | ⟨_, he⟩
      · exact ⟨rfl, Or.inl rfl⟩
      · rcases he with _ | ⟨_, he⟩
        · exact ⟨rfl, Or.inr (Or.inl rfl)⟩
        · obtain ⟨t, -, rfl⟩ := List.mem_map.1 he
          exact ⟨rfl, Or.inr (Or.inr ⟨rfl, hnz⟩)⟩
  | false =>
      rw [chunkR sk pk k i hD] at he
      rcases he with _ | ⟨_, he⟩
      · exact ⟨rfl, Or.inl rfl⟩
      · cases he

private theorem chunk_no_upper (pk : Party × Nat) (k i : Nat) :
    ∀ e ∈ childChunk sk pk k i, e.1 ≠ upperOut pk := by
  intro e he
  rcases (chunk_support sk pk k i e he).2 with h | h | ⟨h, -⟩ <;> rw [h]
  · exact wire_ne_upper pk
  · exact lower_ne_upper pk
  · exact asked_ne_upper pk

private theorem proj_chunk_wire (pk : Party × Nat) (k i : Nat) :
    proj (wireOut pk) true (childChunk sk pk k i)
      = seg (wireOut pk) true (sk.wiresBefore pk.2 k + i) 1 := by
  rw [seg_one]
  cases hD : sk.childIsD pk.2 (sk.stageScope pk.2 k) i with
  | true =>
      rw [chunkD sk pk k i hD, proj_cons_self,
        proj_cons_ne_chan (lower_ne_wire pk),
        proj_seg_ne fun hh => asked_ne_wire pk hh.1]
  | false =>
      rw [chunkR sk pk k i hD, proj_cons_self, proj_nil]

private theorem proj_chunk_res (pk : Party × Nat) (k i : Nat) :
    proj (lowerOut pk) true (childChunk sk pk k i)
      = seg (lowerOut pk) true (sk.dsBefore pk.2 k + dRank sk pk k i)
          (dRank sk pk k (i + 1) - dRank sk pk k i) := by
  cases hD : sk.childIsD pk.2 (sk.stageScope pk.2 k) i with
  | true =>
      have hd : dRank sk pk k (i + 1) - dRank sk pk k i = 1 := by
        rw [dRank_succ]; simp [hD]
      rw [hd, seg_one, chunkD sk pk k i hD,
        proj_cons_ne_chan (wire_ne_lower pk), proj_cons_self,
        proj_seg_ne fun hh => asked_ne_lower pk hh.1]
  | false =>
      have hd : dRank sk pk k (i + 1) - dRank sk pk k i = 0 := by
        rw [dRank_succ]; simp [hD]
      rw [hd, seg_zero, chunkR sk pk k i hD,
        proj_cons_ne_chan (wire_ne_lower pk), proj_nil]

private theorem proj_chunk_q (pk : Party × Nat) (k i : Nat) :
    proj (askedOut pk) true (childChunk sk pk k i)
      = seg (askedOut pk) true (sk.qsBefore pk.2 k + qSum sk pk k i)
          (qSum sk pk k (i + 1) - qSum sk pk k i) := by
  have hq : qSum sk pk k (i + 1) - qSum sk pk k i
      = sk.qCount pk.2 (sk.stageScope pk.2 k) i := by
    rw [qSum_succ]; omega
  rw [hq]
  cases hD : sk.childIsD pk.2 (sk.stageScope pk.2 k) i with
  | true =>
      rw [chunkD sk pk k i hD, proj_cons_ne_chan (wire_ne_asked pk),
        proj_cons_ne_chan (lower_ne_asked pk), proj_seg_self]
  | false =>
      have h0 : sk.qCount pk.2 (sk.stageScope pk.2 k) i = 0 := by
        simp [Skel.qCount, hD]
      rw [h0, seg_zero, chunkR sk pk k i hD,
        proj_cons_ne_chan (wire_ne_asked pk), proj_nil]

private theorem lastD_mem {n : Nat} {q : Nat → Bool} {j : Nat}
    (hj : ((List.range n).filter q).getLast? = some j) :
    j < n ∧ q j = true := by
  have hm := List.mem_of_getLast? hj
  rw [List.mem_filter] at hm
  exact ⟨List.mem_range.1 hm.1, hm.2⟩

/-- The parent splice is invisible to every projection: the parent is
the scope's sole `upperOut` event, and the chunk pieces keep their
relative order around it, so filtering to any one channel-side erases
the difference from the parent-first form. -/
theorem proj_scopeSends (pk : Party × Nat) (k : Nat) (c : Chan) (b : Bool) :
    proj c b (scopeSends sk pk k)
      = proj c b ((upperOut pk, true, k)
          :: ((List.range (sk.nChildren pk.2 (sk.stageScope pk.2 k))).map
              (childChunk sk pk k)).flatten) := by
  simp only [scopeSends]
  split
  · rfl
  · rename_i j hlast
    have hj : j < sk.nChildren pk.2 (sk.stageScope pk.2 k) :=
      (lastD_mem hlast).1
    generalize hchunks : (List.range
      (sk.nChildren pk.2 (sk.stageScope pk.2 k))).map (childChunk sk pk k)
      = chunks
    have hjc : j < chunks.length := by
      rw [← hchunks, List.length_map, List.length_range]; exact hj
    have hget : chunks.getD j [] = childChunk sk pk k j := by
      rw [← hchunks, List.getD_eq_getElem?_getD, List.getElem?_map,
        List.getElem?_range hj]
      rfl
    have hgetE : chunks[j] = childChunk sk pk k j := by
      rw [← hget, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjc]
      rfl
    have hsplit : chunks
        = chunks.take j ++ childChunk sk pk k j :: chunks.drop (j + 1) := by
      conv => lhs; rw [← List.take_append_drop j chunks]
      rw [List.drop_eq_getElem_cons hjc, hgetE]
    -- membership discharger: no chunk event sits on `upperOut`
    have hmem : ∀ e ∈ chunks.flatten, e.1 ≠ upperOut pk := by
      intro e he
      obtain ⟨l, hl, hel⟩ := List.mem_flatten.1 he
      rw [← hchunks] at hl
      obtain ⟨i, -, rfl⟩ := List.mem_map.1 hl
      exact chunk_no_upper sk pk k i e hel
    rw [hget]
    by_cases hup : c = upperOut pk ∧ b = true
    · obtain ⟨rfl, rfl⟩ := hup
      have hnil : ∀ (l : List Ev), (∀ e ∈ l, e.1 ≠ upperOut pk) →
          proj (upperOut pk) true l = [] := fun l h =>
        proj_eq_nil fun e he h1 _ => absurd h1 (h e he)
      have hA : proj (upperOut pk) true ((chunks.take j).flatten) = [] :=
        hnil _ fun e he => by
          obtain ⟨l, hl, hel⟩ := List.mem_flatten.1 he
          exact hmem e (List.mem_flatten.2
            ⟨l, List.mem_of_mem_take hl, hel⟩)
      have hB : proj (upperOut pk) true ((childChunk sk pk k j).take 2)
          = [] := hnil _ fun e he =>
        chunk_no_upper sk pk k j e (List.mem_of_mem_take he)
      have hC : proj (upperOut pk) true ((childChunk sk pk k j).drop 2)
          = [] := hnil _ fun e he =>
        chunk_no_upper sk pk k j e (List.mem_of_mem_drop he)
      have hD : proj (upperOut pk) true ((chunks.drop (j + 1)).flatten)
          = [] := hnil _ fun e he => by
          obtain ⟨l, hl, hel⟩ := List.mem_flatten.1 he
          exact hmem e (List.mem_flatten.2
            ⟨l, List.mem_of_mem_drop hl, hel⟩)
      have hF : proj (upperOut pk) true chunks.flatten = [] := hnil _ hmem
      rw [proj_append, proj_append, proj_cons_self, proj_append,
        proj_cons_self, hA, hB, hC, hD, hF]
      rfl
    · have hpar : ∀ l : List Ev,
          proj c b ((upperOut pk, true, k) :: l) = proj c b l := by
        intro l
        by_cases hcu : c = upperOut pk
        · subst hcu
          exact proj_cons_ne_side fun hb => hup ⟨rfl, hb.symm⟩
        · exact proj_cons_ne_chan fun hh => hcu hh.symm
      rw [proj_append, proj_append, hpar, hpar, proj_append,
        ← List.append_assoc]
      conv => rhs; rw [hsplit]
      rw [List.flatten_append, List.flatten_cons]
      conv => rhs; rw [← List.take_append_drop 2 (childChunk sk pk k j)]
      simp only [proj_append, List.append_assoc]

private theorem mem_scopeSends {pk : Party × Nat} {k : Nat} {e : Ev}
    (he : e ∈ scopeSends sk pk k) :
    e = (upperOut pk, true, k) ∨ ∃ i, e ∈ childChunk sk pk k i := by
  obtain ⟨c, b, n⟩ := e
  have hp : (c, b, n) ∈ proj c b (scopeSends sk pk k) := by
    unfold proj
    exact List.mem_filter.2 ⟨he, by simp⟩
  rw [proj_scopeSends] at hp
  have hm : ((c, b, n) : Ev) ∈ (upperOut pk, true, k)
      :: ((List.range (sk.nChildren pk.2 (sk.stageScope pk.2 k))).map
          (childChunk sk pk k)).flatten := (List.mem_filter.1 hp).1
  rcases hm with _ | ⟨_, hm⟩
  · exact Or.inl rfl
  · obtain ⟨l, hl, hel⟩ := List.mem_flatten.1 hm
    obtain ⟨i, -, rfl⟩ := List.mem_map.1 hl
    exact Or.inr ⟨i, hel⟩

/-- Support of a scope's sends: side true, on one of the four output
channels (queries pin a non-leaf stage). -/
theorem scopeSends_support {pk : Party × Nat} {k : Nat} {e : Ev}
    (he : e ∈ scopeSends sk pk k) :
    e.2.1 = true ∧ (e.1 = upperOut pk ∨ e.1 = wireOut pk
      ∨ e.1 = lowerOut pk ∨ (e.1 = askedOut pk ∧ pk.2 ≠ 0)) := by
  rcases mem_scopeSends sk he with rfl | ⟨i, hi⟩
  · exact ⟨rfl, Or.inl rfl⟩
  · obtain ⟨hs, hc⟩ := chunk_support sk pk k i e hi
    rcases hc with h | h | h
    · exact ⟨hs, Or.inr (Or.inl h)⟩
    · exact ⟨hs, Or.inr (Or.inr (Or.inl h))⟩
    · exact ⟨hs, Or.inr (Or.inr (Or.inr h))⟩

-- ============================== scope blocks, per own channel-side

private theorem proj_scopeBlock_snd (pk : Party × Nat) (k : Nat)
    {c : Chan} :
    proj c true (scopeBlock sk pk k) = proj c true (scopeSends sk pk k) := by
  unfold scopeBlock
  rw [proj_cons_ne_side (by simp), proj_cons_ne_side (by simp)]

private theorem proj_block_wire (pk : Party × Nat) {k : Nat}
    (hk : k < sk.stageLen pk.2) :
    proj (wireOut pk) true (scopeBlock sk pk k)
      = seg (wireOut pk) true (sk.wiresBefore pk.2 k)
          (sk.wiresBefore pk.2 (k + 1) - sk.wiresBefore pk.2 k) := by
  rw [proj_scopeBlock_snd, proj_scopeSends,
    proj_cons_ne_chan (Ne.symm (wire_ne_upper pk)), ← List.flatMap_def]
  have hfold := proj_flatMap_seg (f := childChunk sk pk k)
    (g := fun i => sk.wiresBefore pk.2 k + i)
    (sk.nChildren pk.2 (sk.stageScope pk.2 k))
    (fun i _ => by
      have h1 : sk.wiresBefore pk.2 k + (i + 1)
          - (sk.wiresBefore pk.2 k + i) = 1 := by omega
      rw [h1]; exact proj_chunk_wire sk pk k i)
    (fun i _ => by omega)
  rw [hfold]
  simp only [Nat.add_zero]
  rw [wiresBefore_succ sk hk]

private theorem proj_block_res (pk : Party × Nat) {k : Nat}
    (hk : k < sk.stageLen pk.2) :
    proj (lowerOut pk) true (scopeBlock sk pk k)
      = seg (lowerOut pk) true (sk.dsBefore pk.2 k)
          (sk.dsBefore pk.2 (k + 1) - sk.dsBefore pk.2 k) := by
  rw [proj_scopeBlock_snd, proj_scopeSends,
    proj_cons_ne_chan (Ne.symm (lower_ne_upper pk)), ← List.flatMap_def]
  have hfold := proj_flatMap_seg (f := childChunk sk pk k)
    (g := fun i => sk.dsBefore pk.2 k + dRank sk pk k i)
    (sk.nChildren pk.2 (sk.stageScope pk.2 k))
    (fun i _ => by
      have h1 : sk.dsBefore pk.2 k + dRank sk pk k (i + 1)
          - (sk.dsBefore pk.2 k + dRank sk pk k i)
          = dRank sk pk k (i + 1) - dRank sk pk k i := by omega
      rw [h1]; exact proj_chunk_res sk pk k i)
    (fun i _ => by
      have := dRank_succ sk pk k i
      omega)
  rw [hfold]
  have h0 : dRank sk pk k 0 = 0 := rfl
  rw [h0, dsBefore_succ sk hk, dRank_total sk pk k]
  simp

private theorem proj_block_q (pk : Party × Nat) {k : Nat}
    (hk : k < sk.stageLen pk.2) :
    proj (askedOut pk) true (scopeBlock sk pk k)
      = seg (askedOut pk) true (sk.qsBefore pk.2 k)
          (sk.qsBefore pk.2 (k + 1) - sk.qsBefore pk.2 k) := by
  rw [proj_scopeBlock_snd, proj_scopeSends,
    proj_cons_ne_chan (Ne.symm (asked_ne_upper pk)), ← List.flatMap_def]
  have hfold := proj_flatMap_seg (f := childChunk sk pk k)
    (g := fun i => sk.qsBefore pk.2 k + qSum sk pk k i)
    (sk.nChildren pk.2 (sk.stageScope pk.2 k))
    (fun i _ => by
      have h1 : sk.qsBefore pk.2 k + qSum sk pk k (i + 1)
          - (sk.qsBefore pk.2 k + qSum sk pk k i)
          = qSum sk pk k (i + 1) - qSum sk pk k i := by omega
      rw [h1]; exact proj_chunk_q sk pk k i)
    (fun i _ => by
      have := qSum_succ sk pk k i
      omega)
  rw [hfold]
  have h0 : qSum sk pk k 0 = 0 := rfl
  rw [h0, qsBefore_succ sk hk, qSum_total sk pk k]
  simp

private theorem proj_block_upper (pk : Party × Nat) (k : Nat) :
    proj (upperOut pk) true (scopeBlock sk pk k)
      = seg (upperOut pk) true k 1 := by
  rw [proj_scopeBlock_snd, proj_scopeSends, proj_cons_self, seg_one]
  have hnil : proj (upperOut pk) true
      (((List.range (sk.nChildren pk.2 (sk.stageScope pk.2 k))).map
        (childChunk sk pk k)).flatten) = [] := by
    refine proj_eq_nil fun e he h1 _ => ?_
    obtain ⟨l, hl, hel⟩ := List.mem_flatten.1 he
    obtain ⟨i, -, rfl⟩ := List.mem_map.1 hl
    exact absurd h1 (chunk_no_upper sk pk k i e hel)
  rw [hnil]

private theorem scopeSends_rcv_nil (pk : Party × Nat) (k : Nat)
    (c : Chan) : proj c false (scopeSends sk pk k) = [] :=
  proj_eq_nil fun e he _ hcb => by
    rw [(scopeSends_support sk he).1] at hcb
    exact Bool.noConfusion hcb

private theorem proj_block_wireIn (pk : Party × Nat) (k : Nat) :
    proj (wireIn pk) false (scopeBlock sk pk k)
      = seg (wireIn pk) false k 1 := by
  unfold scopeBlock
  rw [proj_cons_self, proj_cons_ne_chan (askedIn_ne_wireIn pk),
    scopeSends_rcv_nil, seg_one]

private theorem proj_block_askedIn (pk : Party × Nat) (k : Nat) :
    proj (askedIn pk) false (scopeBlock sk pk k)
      = seg (askedIn pk) false k 1 := by
  unfold scopeBlock
  rw [proj_cons_ne_chan (Ne.symm (askedIn_ne_wireIn pk)), proj_cons_self,
    scopeSends_rcv_nil, seg_one]

-- ======================================= the walk master and support

/-- Support of a walk trace: sends on the four output channels
(queries only above the leaf stage), receives on the two inputs. -/
theorem walkEvents_support (pk : Party × Nat) :
    ∀ e ∈ walkEvents sk pk,
      (e.2.1 = true → e.1 = upperOut pk ∨ e.1 = wireOut pk
        ∨ e.1 = lowerOut pk ∨ (e.1 = askedOut pk ∧ pk.2 ≠ 0)) ∧
      (e.2.1 = false → e.1 = wireIn pk ∨ e.1 = askedIn pk) := by
  intro e he
  obtain ⟨k, -, he⟩ := List.mem_flatMap.1 he
  unfold scopeBlock at he
  rcases he with _ | ⟨_, he⟩
  · exact ⟨fun hs => Bool.noConfusion hs, fun _ => Or.inl rfl⟩
  · rcases he with _ | ⟨_, he⟩
    · exact ⟨fun hs => Bool.noConfusion hs, fun _ => Or.inr rfl⟩
    · obtain ⟨hs, hc⟩ := scopeSends_support sk he
      exact ⟨fun _ => hc, fun hr => by rw [hs] at hr; cases hr⟩

/-- Walk `pk`: every channel-side projection is canon-shaped, with the
prefix sums (`wiresBefore` &c.) as the totals. -/
theorem walk_canon (pk : Party × Nat) (c : Chan) (b : Bool) :
    ∃ m, proj c b (walkEvents sk pk) = canon c b m := by
  unfold walkEvents
  by_cases hb : b = true
  · subst hb
    by_cases h1 : c = wireOut pk
    · subst h1
      exact ⟨sk.wiresBefore pk.2 (sk.stageLen pk.2),
        proj_flatMap_canon (g := sk.wiresBefore pk.2) _ rfl
          (fun k hk => proj_block_wire sk pk hk)
          (fun k hk => by rw [wiresBefore_succ sk hk]; omega)⟩
    · by_cases h2 : c = lowerOut pk
      · subst h2
        exact ⟨sk.dsBefore pk.2 (sk.stageLen pk.2),
          proj_flatMap_canon (g := sk.dsBefore pk.2) _ rfl
            (fun k hk => proj_block_res sk pk hk)
            (fun k hk => by rw [dsBefore_succ sk hk]; omega)⟩
      · by_cases h3 : c = askedOut pk
        · subst h3
          exact ⟨sk.qsBefore pk.2 (sk.stageLen pk.2),
            proj_flatMap_canon (g := sk.qsBefore pk.2) _ rfl
              (fun k hk => proj_block_q sk pk hk)
              (fun k hk => by rw [qsBefore_succ sk hk]; omega)⟩
        · by_cases h4 : c = upperOut pk
          · subst h4
            refine ⟨sk.stageLen pk.2,
              proj_flatMap_canon (g := fun k => k) _ rfl
                (fun k _ => ?_) (fun k _ => by omega)⟩
            have h1 : k + 1 - k = 1 := by omega
            rw [h1]
            exact proj_block_upper sk pk k
          · refine canon_nil fun e he hc hcb => ?_
            obtain ⟨k, -, he⟩ := List.mem_flatMap.1 he
            unfold scopeBlock at he
            rcases he with _ | ⟨_, he⟩
            · exact Bool.noConfusion hcb
            · rcases he with _ | ⟨_, he⟩
              · exact Bool.noConfusion hcb
              · rcases (scopeSends_support sk he).2 with h | h | h | ⟨h, -⟩
                · exact h4 (hc.symm.trans h)
                · exact h1 (hc.symm.trans h)
                · exact h2 (hc.symm.trans h)
                · exact h3 (hc.symm.trans h)
  · have hbf : b = false := by cases b <;> simp_all
    subst hbf
    by_cases h1 : c = wireIn pk
    · subst h1
      refine ⟨sk.stageLen pk.2,
        proj_flatMap_canon (g := fun k => k) _ rfl
          (fun k _ => ?_) (fun k _ => by omega)⟩
      have hone : k + 1 - k = 1 := by omega
      rw [hone]
      exact proj_block_wireIn sk pk k
    · by_cases h2 : c = askedIn pk
      · subst h2
        refine ⟨sk.stageLen pk.2,
          proj_flatMap_canon (g := fun k => k) _ rfl
            (fun k _ => ?_) (fun k _ => by omega)⟩
        have hone : k + 1 - k = 1 := by omega
        rw [hone]
        exact proj_block_askedIn sk pk k
      · refine canon_nil fun e he hc hcb => ?_
        obtain ⟨k, -, he⟩ := List.mem_flatMap.1 he
        unfold scopeBlock at he
        rcases he with _ | ⟨_, he⟩
        · exact h1 hc.symm
        · rcases he with _ | ⟨_, he⟩
          · exact h2 hc.symm
          · rw [(scopeSends_support sk he).1] at hcb
            exact Bool.noConfusion hcb

-- =================================================== the asm family

private theorem res_ne_level (pk : Party × Nat) :
    asmResChan pk ≠ asmLevelChan pk := by
  unfold asmResChan asmLevelChan; split <;> simp

/-- One asm block, restated with its level segment named. -/
private theorem asmBlock_eq (pk : Party × Nat) (idx : Nat) :
    asmBlock sk pk idx
      = (asmResChan pk, false, idx)
        :: seg (asmLevelChan pk) false (sk.pendsBefore pk.1 pk.2 idx)
            (sk.pendAt pk.1 pk.2 idx)
        ++ [(sk.asmOutChan pk, true, idx)] := rfl

private theorem proj_asmBlock_res (pk : Party × Nat) (idx : Nat) :
    proj (asmResChan pk) false (asmBlock sk pk idx)
      = seg (asmResChan pk) false idx 1 := by
  rw [asmBlock_eq, seg_one, proj_append, proj_cons_self,
    proj_seg_ne fun hh => res_ne_level pk hh.1.symm,
    proj_cons_ne_side (by simp), proj_nil, List.append_nil]

private theorem proj_asmBlock_level (pk : Party × Nat) (idx : Nat) :
    proj (asmLevelChan pk) false (asmBlock sk pk idx)
      = seg (asmLevelChan pk) false (sk.pendsBefore pk.1 pk.2 idx)
          (sk.pendAt pk.1 pk.2 idx) := by
  rw [asmBlock_eq, proj_append, proj_cons_ne_chan (res_ne_level pk),
    proj_seg_self, proj_cons_ne_side (by simp), proj_nil,
    List.append_nil]

private theorem proj_asmBlock_out (pk : Party × Nat) (idx : Nat) :
    proj (sk.asmOutChan pk) true (asmBlock sk pk idx)
      = seg (sk.asmOutChan pk) true idx 1 := by
  rw [asmBlock_eq, seg_one, proj_append, proj_cons_ne_side (by simp),
    proj_seg_ne (by simp), proj_cons_self, proj_nil, List.nil_append]

/-- Support of an asm trace: sends on the output channel only,
receives on the resolution and level channels. -/
theorem asmEvents_support (pk : Party × Nat) :
    ∀ e ∈ asmEvents sk pk,
      (e.2.1 = true → e.1 = sk.asmOutChan pk) ∧
      (e.2.1 = false → e.1 = asmResChan pk ∨ e.1 = asmLevelChan pk) := by
  intro e he
  obtain ⟨idx, -, he⟩ := List.mem_flatMap.1 he
  rw [asmBlock_eq] at he
  rcases he with _ | ⟨_, he⟩
  · exact ⟨fun hs => Bool.noConfusion hs, fun _ => Or.inl rfl⟩
  · rcases List.mem_append.1 he with he | he
    · obtain ⟨t, -, rfl⟩ := List.mem_map.1 he
      exact ⟨fun hs => Bool.noConfusion hs, fun _ => Or.inr rfl⟩
    · rcases he with _ | ⟨_, he⟩
      · exact ⟨fun _ => rfl, fun hr => Bool.noConfusion hr⟩
      · cases he

/-- Asm `pk`: every channel-side projection is canon-shaped, with the
pending prefix sums as the level totals. -/
theorem asm_canon (pk : Party × Nat) (c : Chan) (b : Bool) :
    ∃ m, proj c b (asmEvents sk pk) = canon c b m := by
  unfold asmEvents
  by_cases hb : b = true
  · subst hb
    by_cases h1 : c = sk.asmOutChan pk
    · subst h1
      refine ⟨(sk.asmResList pk.1 pk.2).length,
        proj_flatMap_canon (g := fun k => k) _ rfl
          (fun k _ => ?_) (fun k _ => by omega)⟩
      have hone : k + 1 - k = 1 := by omega
      rw [hone]
      exact proj_asmBlock_out sk pk k
    · refine canon_nil fun e he hc hcb => ?_
      exact h1 (hc.symm.trans ((asmEvents_support sk pk e he).1 hcb))
  · have hbf : b = false := by cases b <;> simp_all
    subst hbf
    by_cases h1 : c = asmResChan pk
    · subst h1
      refine ⟨(sk.asmResList pk.1 pk.2).length,
        proj_flatMap_canon (g := fun k => k) _ rfl
          (fun k _ => ?_) (fun k _ => by omega)⟩
      have hone : k + 1 - k = 1 := by omega
      rw [hone]
      exact proj_asmBlock_res sk pk k
    · by_cases h2 : c = asmLevelChan pk
      · subst h2
        exact ⟨sk.pendsBefore pk.1 pk.2 (sk.asmResList pk.1 pk.2).length,
          proj_flatMap_canon (g := sk.pendsBefore pk.1 pk.2) _ rfl
            (fun k hk => by
              have hd : sk.pendsBefore pk.1 pk.2 (k + 1)
                  - sk.pendsBefore pk.1 pk.2 k
                  = sk.pendAt pk.1 pk.2 k := by
                rw [pendsBefore_succ sk hk]; omega
              rw [hd]
              exact proj_asmBlock_level sk pk k)
            (fun k hk => by rw [pendsBefore_succ sk hk]; omega)⟩
      · refine canon_nil fun e he hc hcb => ?_
        rcases (asmEvents_support sk pk e he).2 hcb with h | h
        · exact h1 (hc.symm.trans h)
        · exact h2 (hc.symm.trans h)

-- ============================== the whole trace family, canon-shaped

/-- Every trace's every channel-side projection is canon-shaped: the
per-trace half of the numbering layer. -/
theorem procs_canon (c : Chan) (b : Bool) :
    ∀ t ∈ procs sk, ∃ m, proj c b t = canon c b m := by
  intro t ht
  simp only [procs, List.mem_append, List.mem_cons,
    List.not_mem_nil, or_false, List.mem_map] at ht
  rcases ht with ((((rfl | rfl) | ⟨pk, -, rfl⟩) | rfl) | ⟨pk, -, rfl⟩)
    | rfl | rfl
  · exact iopen_canon sk c b
  · exact ropen_canon sk c b
  · exact walk_canon sk pk c b
  · exact absorb_canon sk c b
  · exact asm_canon sk pk c b
  · -- the floating rootret receive
    by_cases h1 : c = Chan.rootret ∧ b = false
    · obtain ⟨rfl, rfl⟩ := h1
      exact ⟨1, by rw [proj_cons_self, proj_nil, canon_one]⟩
    · refine canon_nil fun e he hc hcb => ?_
      rcases he with _ | ⟨_, he⟩
      · exact h1 ⟨hc.symm, hcb.symm⟩
      · cases he
  · exact fin_canon sk c b

end StreamingMirror.Sched
