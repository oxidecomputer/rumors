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
import StreamingMirror.Proofs.Lemmas

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
theorem qSum_total (pk : Party × Nat) (k : Nat) :
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

-- ===================== cross-trace uniqueness: the ownership layer
-- Rather than a quadratic disjointness sweep over trace pairs, each
-- trace proves its own events name its index via an owner function;
-- two traces sharing a channel-side would then name two indices at
-- once — a Nat contradiction.

/-- Positional ownership: every side-`b` event of every trace names
its trace's position (counting from `i`) via `f`. -/
def Owned (f : Chan → Nat) (b : Bool) : Nat → List (List Ev) → Prop
  | _, [] => True
  | i, t :: ts => (∀ e ∈ t, e.2.1 = b → f e.1 = i) ∧ Owned f b (i + 1) ts

private theorem owned_append {f : Chan → Nat} {b : Bool} :
    ∀ {ts₁ : List (List Ev)} {i : Nat} {ts₂ : List (List Ev)},
      Owned f b i ts₁ → Owned f b (i + ts₁.length) ts₂ →
      Owned f b i (ts₁ ++ ts₂) := by
  intro ts₁
  induction ts₁ with
  | nil => intro i ts₂ _ h₂; exact h₂
  | cons t ts ih =>
      intro i ts₂ h₁ h₂
      refine ⟨h₁.1, ih h₁.2 ?_⟩
      have hL : i + (t :: ts).length = (i + 1) + ts.length := by
        rw [List.length_cons]; omega
      rwa [hL] at h₂

private theorem owned_map_range {f : Chan → Nat} {b : Bool}
    {g : Nat → List Ev} :
    ∀ (n i : Nat), (∀ j, j < n → ∀ e ∈ g j, e.2.1 = b → f e.1 = i + j) →
      Owned f b i ((List.range n).map g)
  | 0, _, _ => trivial
  | n + 1, i, h => by
      rw [List.range_succ, List.map_append]
      refine owned_append
        (owned_map_range n i fun j hj => h j (Nat.lt_succ_of_lt hj)) ?_
      rw [List.length_map, List.length_range]
      exact ⟨fun e he hs => h n (Nat.lt_succ_self n) e he hs, trivial⟩

theorem owned_ge {f : Chan → Nat} {b : Bool} :
    ∀ {i : Nat} {ts : List (List Ev)}, Owned f b i ts →
      ∀ t ∈ ts, ∀ e ∈ t, e.2.1 = b → i ≤ f e.1 := by
  intro i ts
  induction ts generalizing i with
  | nil => intro _ t ht; cases ht
  | cons t' ts' ih =>
      intro h t ht e he hs
      rcases ht with _ | ⟨_, ht⟩
      · exact Nat.le_of_eq (h.1 e he hs).symm
      · exact Nat.le_trans (Nat.le_succ i) (ih h.2 t ht e he hs)

/-- Index of the walk at consumed-height `h` in `procs`. -/
def walkIdx (h : Nat) : Nat := 2 + (sk.rootH - 1 - h)

/-- Index of the assembler `(p, j)` in `procs`. -/
def asmIdx : Party → Nat → Nat
  | .I, j => 3 + sk.rootH + (j - 1)
  | .R, j => 3 + 2 * sk.rootH + (j - 1)

/-- The producing trace of each channel, as an index into `procs`
(unconstrained on channels no trace sends — ownership is consulted
only at actual events). -/
def sndOwner : Chan → Nat
  | .wire p h =>
      if h = sk.rootH then (if p = Party.I then 0 else 1)
      else walkIdx sk h
  | .asked p h =>
      if p = Party.I ∧ h + 1 = sk.rootH then 0
      else if p = Party.R ∧ h + 2 = sk.rootH then 1
      else walkIdx sk (h + 2)
  | .leafRequests => walkIdx sk 1
  | .upper _ h => walkIdx sk h
  | .lower _ h => walkIdx sk h
  | .level p j =>
      if p = Party.I ∧ j = 0 then 2 + sk.rootH
      else asmIdx sk p j
  | .rootret => asmIdx sk Party.I sk.rootH
  | .rootrets => asmIdx sk Party.R (sk.rootH - 1)
  | .rootres => 1

/-- The consuming trace of each channel, as an index into `procs`. -/
def rcvOwner : Chan → Nat
  | .wire p h =>
      if p = Party.I ∧ h = sk.rootH then 1
      else if h = 0 then 2 + sk.rootH
      else walkIdx sk (h - 1)
  | .asked _ h => walkIdx sk h
  | .leafRequests => 2 + sk.rootH
  | .upper p h => asmIdx sk p (h + 1)
  | .lower p j => asmIdx sk p j
  | .level p j => asmIdx sk p (j + 1)
  | .rootret => 3 * sk.rootH + 2
  | .rootrets => 3 * sk.rootH + 3
  | .rootres => 3 * sk.rootH + 3

-- ============================ per-family ownership membership proofs

private theorem walk_snd_owner (hwf : sk.wellFormed = true)
    {pk : Party × Nat} (hh : pk.2 < sk.rootH) :
    ∀ e ∈ walkEvents sk pk, e.2.1 = true →
      sndOwner sk e.1 = walkIdx sk pk.2 := by
  have hge := (wf_rootH hwf).2
  intro e he hs
  rcases (walkEvents_support sk pk e he).1 hs with h | h | h | ⟨h, hnz⟩
  · rw [h]; rfl
  · rw [h]
    simp only [wireOut, sndOwner]
    rw [if_neg (by omega)]
  · rw [h]; rfl
  · rw [h]
    unfold askedOut
    by_cases h2 : pk.2 < 2
    · rw [if_pos h2]
      have h1 : pk.2 = 1 := by omega
      simp only [sndOwner, h1]
    · rw [if_neg h2]
      simp only [sndOwner]
      rw [if_neg fun hcon => absurd hcon.2 (by omega),
        if_neg fun hcon => absurd hcon.2 (by omega)]
      have h22 : pk.2 - 2 + 2 = pk.2 := by omega
      rw [h22]

private theorem walk_rcv_owner {pk : Party × Nat}
    (hR : pk.1 = Party.R → pk.2 + 1 ≠ sk.rootH) :
    ∀ e ∈ walkEvents sk pk, e.2.1 = false →
      rcvOwner sk e.1 = walkIdx sk pk.2 := by
  intro e he hs
  rcases (walkEvents_support sk pk e he).2 hs with h | h
  · rw [h]
    simp only [wireIn, rcvOwner]
    have hc1 : ¬(pk.1.other = Party.I ∧ pk.2 + 1 = sk.rootH) := by
      rintro ⟨hpI, hh1⟩
      have hpR : pk.1 = Party.R := by
        cases hp : pk.1 with
        | I => rw [hp] at hpI; exact absurd hpI (by simp [Party.other])
        | R => rfl
      exact hR hpR hh1
    rw [if_neg hc1, if_neg (by omega)]
    have h21 : pk.2 + 1 - 1 = pk.2 := by omega
    rw [h21]
  · rw [h]; rfl

private theorem asm_snd_owner {p : Party} {j : Nat} (hj1 : 1 ≤ j) :
    ∀ e ∈ asmEvents sk (p, j), e.2.1 = true →
      sndOwner sk e.1 = asmIdx sk p j := by
  intro e he hs
  rw [(asmEvents_support sk (p, j) e he).1 hs]
  cases p with
  | I =>
      by_cases hjr : j = sk.rootH
      · subst hjr
        simp [Skel.asmOutChan, sndOwner]
      · have hout : sk.asmOutChan (Party.I, j) = Chan.level Party.I j := by
          simp [Skel.asmOutChan, hjr]
        rw [hout]
        simp only [sndOwner]
        rw [if_neg fun hcon => absurd hcon.2 (by omega)]
  | R =>
      by_cases hjr : j = sk.rootH - 1
      · subst hjr
        simp [Skel.asmOutChan, sndOwner]
      · have hout : sk.asmOutChan (Party.R, j) = Chan.level Party.R j := by
          simp [Skel.asmOutChan, hjr]
        rw [hout]
        simp only [sndOwner]
        rw [if_neg fun hcon => Party.noConfusion hcon.1]

private theorem asm_rcv_owner {p : Party} {j : Nat} (hj1 : 1 ≤ j) :
    ∀ e ∈ asmEvents sk (p, j), e.2.1 = false →
      rcvOwner sk e.1 = asmIdx sk p j := by
  intro e he hs
  have hj11 : j - 1 + 1 = j := by omega
  rcases (asmEvents_support sk (p, j) e he).2 hs with h | h
  · rw [h]
    unfold asmResChan
    split
    · simp only [rcvOwner]
      rw [hj11]
    · rfl
  · rw [h]
    simp only [asmLevelChan, rcvOwner]
    rw [hj11]

-- ================================= the whole trace family, owned

/-- Sends: every trace's send events name that trace's index — one
producer per channel. -/
theorem procs_snd_owned (hwf : sk.wellFormed = true) :
    Owned (sndOwner sk) true 0 (procs sk) := by
  have hge := (wf_rootH hwf).2
  simp only [procs, Skel.asmKeys, List.map_append, List.map_map]
  refine owned_append (owned_append (owned_append
    (owned_append ?openers ?walks) ?absorb) ?asms) ?fins
  case openers =>
    refine ⟨?io, ?ro, trivial⟩
    case io =>
      intro e he hs
      unfold iopenEvents at he
      rcases he with _ | ⟨_, he⟩
      · simp [sndOwner]
      · rcases he with _ | ⟨_, he⟩
        · simp only [sndOwner]
          have hx : sk.rootH - 1 + 1 = sk.rootH := by omega
          simp [hx]
        · cases he
    case ro =>
      intro e he hs
      unfold ropenEvents at he
      rcases he with _ | ⟨_, he⟩
      · exact Bool.noConfusion hs
      · rcases he with _ | ⟨_, he⟩
        · simp [sndOwner]
        · rcases he with _ | ⟨_, he⟩
          · rfl
          · obtain ⟨q, -, rfl⟩ := List.mem_map.1 he
            simp only [sndOwner]
            have hx : sk.rootH - 2 + 2 = sk.rootH := by omega
            simp [hx]
  case walks =>
    refine owned_map_range _ _ fun j hj e he hs => ?_
    rw [walk_snd_owner sk hwf (by omega : sk.rootH - 1 - j < sk.rootH) e
      he hs]
    show 2 + (sk.rootH - 1 - (sk.rootH - 1 - j)) = _
    simp only [List.length_cons, List.length_nil]
    omega
  case absorb =>
    refine ⟨?_, trivial⟩
    intro e he hs
    unfold absorbEvents at he
    obtain ⟨q, -, he⟩ := List.mem_flatMap.1 he
    rcases he with _ | ⟨_, he⟩
    · exact Bool.noConfusion hs
    · rcases he with _ | ⟨_, he⟩
      · exact Bool.noConfusion hs
      · rcases he with _ | ⟨_, he⟩
        · simp only [sndOwner]
          rw [if_pos ⟨trivial, trivial⟩]
          simp only [List.length_append, List.length_cons, List.length_nil,
            List.length_map, List.length_range]
          omega
        · cases he
  case asms =>
    refine owned_append ?asmI ?asmR
    case asmI =>
      refine owned_map_range _ _ fun j hj e he hs => ?_
      rw [asm_snd_owner sk (p := Party.I) (j := j + 1) (by omega) e he hs]
      show 3 + sk.rootH + (j + 1 - 1) = _
      simp only [List.length_append, List.length_cons, List.length_nil,
        List.length_map, List.length_range]
      omega
    case asmR =>
      refine owned_map_range _ _ fun j hj e he hs => ?_
      rw [asm_snd_owner sk (p := Party.R) (j := j + 1) (by omega) e he hs]
      show 3 + 2 * sk.rootH + (j + 1 - 1) = _
      simp only [List.length_append, List.length_cons, List.length_nil,
        List.length_map, List.length_range]
      omega
  case fins =>
    refine ⟨?_, ?_, trivial⟩
    · intro e he hs
      rcases he with _ | ⟨_, he⟩
      · exact Bool.noConfusion hs
      · cases he
    · intro e he hs
      unfold finEvents at he
      rcases he with _ | ⟨_, he⟩
      · exact Bool.noConfusion hs
      · obtain ⟨q, -, rfl⟩ := List.mem_map.1 he
        exact Bool.noConfusion hs

/-- Receives: every trace's receive events name that trace's index —
one consumer per channel. -/
theorem procs_rcv_owned (hwf : sk.wellFormed = true) :
    Owned (rcvOwner sk) false 0 (procs sk) := by
  have hge := (wf_rootH hwf).2
  have hev := (wf_rootH hwf).1
  simp only [procs, Skel.asmKeys, List.map_append, List.map_map]
  refine owned_append (owned_append (owned_append
    (owned_append ?openers ?walks) ?absorb) ?asms) ?fins
  case openers =>
    refine ⟨?io, ?ro, trivial⟩
    case io =>
      intro e he hs
      unfold iopenEvents at he
      rcases he with _ | ⟨_, he⟩
      · exact Bool.noConfusion hs
      · rcases he with _ | ⟨_, he⟩
        · exact Bool.noConfusion hs
        · cases he
    case ro =>
      intro e he hs
      unfold ropenEvents at he
      rcases he with _ | ⟨_, he⟩
      · simp only [rcvOwner]
        rw [if_pos ⟨trivial, trivial⟩]
      · rcases he with _ | ⟨_, he⟩
        · exact Bool.noConfusion hs
        · rcases he with _ | ⟨_, he⟩
          · exact Bool.noConfusion hs
          · obtain ⟨q, -, rfl⟩ := List.mem_map.1 he
            exact Bool.noConfusion hs
  case walks =>
    refine owned_map_range _ _ fun j hj e he hs => ?_
    rw [walk_rcv_owner sk ?hpar e he hs]
    · show 2 + (sk.rootH - 1 - (sk.rootH - 1 - j)) = _
      simp only [List.length_cons, List.length_nil]
      omega
    case hpar =>
      intro hRR hcon
      dsimp only at hRR hcon
      have hodd : ((sk.rootH - 1 - j) % 2 == 1) = true := by
        have hm : (sk.rootH - 1 - j) % 2 = 1 := by omega
        simpa using hm
      rw [hodd] at hRR
      exact absurd hRR (by simp)
  case absorb =>
    refine ⟨?_, trivial⟩
    intro e he hs
    unfold absorbEvents at he
    obtain ⟨q, -, he⟩ := List.mem_flatMap.1 he
    rcases he with _ | ⟨_, he⟩
    · simp only [rcvOwner]
      rw [if_neg fun hcon => Party.noConfusion hcon.1, if_pos trivial]
      simp only [List.length_append, List.length_cons, List.length_nil,
        List.length_map, List.length_range]
      omega
    · rcases he with _ | ⟨_, he⟩
      · simp only [rcvOwner]
        simp only [List.length_append, List.length_cons, List.length_nil,
          List.length_map, List.length_range]
        omega
      · rcases he with _ | ⟨_, he⟩
        · exact Bool.noConfusion hs
        · cases he
  case asms =>
    refine owned_append ?asmI ?asmR
    case asmI =>
      refine owned_map_range _ _ fun j hj e he hs => ?_
      rw [asm_rcv_owner sk (p := Party.I) (j := j + 1) (by omega) e he hs]
      show 3 + sk.rootH + (j + 1 - 1) = _
      simp only [List.length_append, List.length_cons, List.length_nil,
        List.length_map, List.length_range]
      omega
    case asmR =>
      refine owned_map_range _ _ fun j hj e he hs => ?_
      rw [asm_rcv_owner sk (p := Party.R) (j := j + 1) (by omega) e he hs]
      show 3 + 2 * sk.rootH + (j + 1 - 1) = _
      simp only [List.length_append, List.length_cons, List.length_nil,
        List.length_map, List.length_range]
      omega
  case fins =>
    refine ⟨?fl, ?fn, trivial⟩
    case fl =>
      intro e he hs
      rcases he with _ | ⟨_, he⟩
      · show 3 * sk.rootH + 2 = _
        simp only [List.length_append, List.length_cons, List.length_nil,
          List.length_map, List.length_range]
        omega
      · cases he
    case fn =>
      intro e he hs
      unfold finEvents at he
      have hval : (3 : Nat) * sk.rootH + 3 = 3 * sk.rootH + 3 := rfl
      rcases he with _ | ⟨_, he⟩
      · show 3 * sk.rootH + 3 = _
        simp only [List.length_append, List.length_cons, List.length_nil,
          List.length_map, List.length_range]
        omega
      · obtain ⟨q, -, rfl⟩ := List.mem_map.1 he
        show 3 * sk.rootH + 3 = _
        simp only [List.length_append, List.length_cons, List.length_nil,
          List.length_map, List.length_range]
        omega

-- ================== the consumer: the SCHEDULE's projections are canon
-- `out_count` says the schedule holds exactly the traces' emitted
-- prefixes; ownership says each channel-side is fed by one trace; the
-- per-trace canon shape then transfers to the schedule itself.

/-- A prefix of a canonical projection is the canonical projection of
its own length. -/
theorem prefix_canon {c : Chan} {b : Bool} {m : Nat} {l : List Ev}
    (h : l <+: canon c b m) : l = canon c b l.length := by
  obtain ⟨s, hs⟩ := h
  have htake : (canon c b m).take l.length = canon c b l.length := by
    unfold canon
    rw [← List.map_take, List.take_range]
    have hle : l.length ≤ m := by
      have := congrArg List.length hs
      simp [canon] at this
      omega
    rw [Nat.min_eq_left hle]
  rw [← htake, ← hs, List.take_left]

/-- Prefixes project to prefixes: the emitted part of a trace never
projects past the trace's own canon stream. -/
private theorem proj_prefix {c : Chan} {b : Bool} {pre r : List Ev} :
    proj c b pre <+: proj c b (pre ++ r) := by
  rw [proj_append]
  exact List.prefix_append ..

/-- All-empty traces emit nothing on the channel-side. -/
theorem emitted_nil {c : Chan} {b : Bool} {out : List Ev} :
    ∀ {ts rs : List (List Ev)},
      Forall2 (fun t r => ∃ pre, t = pre ++ r ∧ pre.Sublist out) ts rs →
      (∀ t ∈ ts, proj c b t = []) →
      emittedCount (fun e => decide (e.1 = c) && (e.2.1 == b)) ts rs = 0
  | _, _, .nil, _ => rfl
  | _, _, .cons (a := t) (la := ts) (b := r) (lb := rs)
      ⟨pre, hpre, _⟩ htail, hnil => by
      have hcount : emittedCount
          (fun e => decide (e.1 = c) && (e.2.1 == b)) (t :: ts) (r :: rs)
          = (proj c b (t.take (t.length - r.length))).length
            + emittedCount (fun e => decide (e.1 = c) && (e.2.1 == b))
                ts rs := rfl
      have hpretake : t.take (t.length - r.length) = pre := by
        subst hpre
        have hlen : (pre ++ r).length - r.length = pre.length := by simp
        rw [hlen, List.take_left]
      have hp : proj c b pre = [] := by
        have h0 := hnil t (List.mem_cons_self ..)
        rw [hpre, proj_append, List.append_eq_nil_iff] at h0
        exact h0.1
      rw [hcount, hpretake, hp, emitted_nil htail fun t' ht' =>
        hnil t' (List.mem_cons_of_mem _ ht')]
      rfl

/-- The emitted prefixes of an owned, per-trace-canon family project
to one canonical stream inside `out`: at most one trace feeds the
channel-side, and its emitted prefix is a canon prefix. -/
theorem emitted_canon {c : Chan} {b : Bool} {out : List Ev}
    {f : Chan → Nat} :
    ∀ {i : Nat} {ts rs : List (List Ev)},
      Forall2 (fun t r => ∃ pre, t = pre ++ r ∧ pre.Sublist out) ts rs →
      Owned f b i ts →
      (∀ t ∈ ts, ∃ m, proj c b t = canon c b m) →
      ∃ pre', pre'.Sublist out ∧ proj c b pre' = canon c b
        (emittedCount (fun e => decide (e.1 = c) && (e.2.1 == b)) ts rs)
  | _, _, _, .nil, _, _ => ⟨[], List.nil_sublist _, rfl⟩
  | i, _, _, .cons (a := t) (la := ts) (b := r) (lb := rs)
      ⟨pre, hpre, hsub⟩ htail, hown, hcanon => by
      obtain ⟨m, hm⟩ := hcanon t (List.mem_cons_self ..)
      have hcount : emittedCount
          (fun e => decide (e.1 = c) && (e.2.1 == b)) (t :: ts) (r :: rs)
          = (proj c b (t.take (t.length - r.length))).length
            + emittedCount (fun e => decide (e.1 = c) && (e.2.1 == b))
                ts rs :=
        rfl
      have hpretake : t.take (t.length - r.length) = pre := by
        subst hpre
        have hlen : (pre ++ r).length - r.length = pre.length := by simp
        rw [hlen, List.take_left]
      by_cases hpt : proj c b t = []
      · -- the head is silent on this channel-side: recurse
        obtain ⟨pre', hsub', hpre'⟩ :=
          emitted_canon htail hown.2 fun t' ht' =>
            hcanon t' (List.mem_cons_of_mem _ ht')
        refine ⟨pre', hsub', ?_⟩
        rw [hcount, hpretake]
        have hp : proj c b pre = [] := by
          subst hpre
          rw [proj_append, List.append_eq_nil_iff] at hpt
          exact hpt.1
        rw [hp]
        simpa using hpre'
      · -- the head owns the channel-side: the tail is silent
        have hfc : f c = i := by
          cases hq : proj c b t with
          | nil => exact absurd hq hpt
          | cons e rest =>
              have hemem : e ∈ proj c b t := by
                rw [hq]; exact List.mem_cons_self ..
              have hin := List.mem_filter.1 hemem
              simp only [Bool.and_eq_true, decide_eq_true_eq, beq_iff_eq]
                at hin
              rw [← hin.2.1]
              exact hown.1 e hin.1 hin.2.2
        have htail_nil : ∀ t' ∈ ts, proj c b t' = [] := by
          intro t' ht'
          cases hq : proj c b t' with
          | nil => rfl
          | cons e' rest' =>
              have hemem' : e' ∈ proj c b t' := by
                rw [hq]; exact List.mem_cons_self ..
              have hin' := List.mem_filter.1 hemem'
              simp only [Bool.and_eq_true, decide_eq_true_eq, beq_iff_eq]
                at hin'
              have hge := owned_ge hown.2 t' ht' e' hin'.1 hin'.2.2
              rw [hin'.2.1, hfc] at hge
              omega
        refine ⟨pre, hsub, ?_⟩
        rw [hcount, hpretake, emitted_nil htail htail_nil, Nat.add_zero]
        have hpref : proj c b pre <+: canon c b m := by
          rw [← hm, hpre]
          exact proj_prefix
        exact prefix_canon hpref

/-- The schedule's own projections are canonical: on every channel and
side, the merge emits seqs `0, 1, 2, …` in order. This is the
numbering layer's conclusion; positional E1 and τ injectivity read off
it. -/
theorem schedule_proj_canon (hwf : sk.wellFormed = true) (c : Chan)
    (b : Bool) : ∃ m, proj c b (schedule sk) = canon c b m := by
  have howned : Owned (if b then sndOwner sk else rcvOwner sk) b 0
      (procs sk) := by
    cases b
    · exact procs_rcv_owned sk hwf
    · exact procs_snd_owned sk hwf
  obtain ⟨pre, hsub, hpre⟩ :=
    emitted_canon (trace_monotone sk) howned (procs_canon sk c b)
  refine ⟨emittedCount (fun e => decide (e.1 = c) && (e.2.1 == b))
    (procs sk) (finalState sk).rem, ?_⟩
  have hcount : (proj c b (schedule sk)).length
      = emittedCount (fun e => decide (e.1 = c) && (e.2.1 == b))
        (procs sk) (finalState sk).rem := schedule_count sk _
  have hlenpre : (proj c b pre).length
      = emittedCount (fun e => decide (e.1 = c) && (e.2.1 == b))
        (procs sk) (finalState sk).rem := by
    rw [hpre]
    simp [canon]
  have hsubp : (proj c b pre).Sublist (proj c b (schedule sk)) :=
    hsub.filter _
  have heq : proj c b pre = proj c b (schedule sk) :=
    hsubp.eq_of_length (by rw [hlenpre, hcount])
  rw [← heq, hpre]

-- ==================================== the corollaries the blame layer
-- and the argmin assembly consume

/-- Positional E1: every receive in the schedule is preceded by the
send with ITS OWN seq — the counted form (`schedule_e1`) upgraded
through canonical numbering. -/
theorem schedule_e1_pos (hwf : sk.wellFormed = true) (k : Nat) (c : Chan)
    (n : Nat) (h : (schedule sk)[k]? = some (c, false, n)) :
    ∃ j, j < k ∧ (schedule sk)[j]? = some (c, true, n) := by
  have hcount := schedule_e1 sk k c n h
  rw [sndCount_eq_proj] at hcount
  obtain ⟨m, hm⟩ := schedule_proj_canon sk hwf c true
  have hpref : proj c true ((schedule sk).take k) <+: canon c true m := by
    rw [← hm]
    conv => rhs; rw [← List.take_append_drop k (schedule sk)]
    exact proj_prefix
  have htake := prefix_canon hpref
  have hmem : ((c, true, n) : Ev) ∈ proj c true ((schedule sk).take k) := by
    rw [htake]
    exact List.mem_map.2 ⟨n, List.mem_range.2 hcount, rfl⟩
  have hmem' : ((c, true, n) : Ev) ∈ (schedule sk).take k :=
    (List.mem_filter.1 hmem).1
  obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hmem'
  rw [List.getElem?_take] at hj
  by_cases hjk : j < k
  · rw [if_pos hjk] at hj
    exact ⟨j, hjk, hj⟩
  · rw [if_neg hjk] at hj
    cases hj

private theorem count_canon (c : Chan) (b : Bool) (n : Nat) :
    ∀ m, (canon c b m).count (c, b, n) = if n < m then 1 else 0
  | 0 => by simp [canon_zero]
  | m + 1 => by
      rw [canon_succ, List.count_append, count_canon c b n m]
      by_cases hn : n = m
      · subst hn
        rw [if_neg (by omega), if_pos (by omega)]
        simp
      · have hb : (((c, b, m) : Ev) == (c, b, n)) = false := by
          simp
          omega
        rw [List.count_cons, List.count_nil, hb]
        by_cases hlt : n < m
        · rw [if_pos hlt, if_pos (show n < m + 1 by omega)]
          all_goals simp
        · rw [if_neg hlt, if_neg (show ¬n < m + 1 by omega)]
          all_goals simp

private theorem two_at_lt {l : List Ev} {i j : Nat} {e : Ev} (hij : i < j)
    (hi : l[i]? = some e) (hj : l[j]? = some e) : 2 ≤ l.count e := by
  have h1 : e ∈ l.take j :=
    List.mem_iff_getElem?.2 ⟨i, by rw [List.getElem?_take, if_pos hij]; exact hi⟩
  have h2 : e ∈ l.drop j :=
    List.mem_iff_getElem?.2 ⟨0, by rw [List.getElem?_drop]; simpa using hj⟩
  have hc1 : 0 < (l.take j).count e := List.count_pos_iff.2 h1
  have hc2 : 0 < (l.drop j).count e := List.count_pos_iff.2 h2
  have hsplit : l.count e = (l.take j).count e + (l.drop j).count e := by
    conv => lhs; rw [← List.take_append_drop j l]
    exact List.count_append
  omega

/-- τ injectivity: the schedule holds each event at most once, so
position-in-schedule is a well-defined timestamp for the event set. -/
theorem schedule_inj (hwf : sk.wellFormed = true) {i j : Nat} {e : Ev}
    (hi : (schedule sk)[i]? = some e) (hj : (schedule sk)[j]? = some e) :
    i = j := by
  obtain ⟨c, b, n⟩ := e
  obtain ⟨m, hm⟩ := schedule_proj_canon sk hwf c b
  have hpred : (fun e : Ev => decide (e.1 = c) && (e.2.1 == b))
      (c, b, n) = true := by simp
  have hcle : (schedule sk).count (c, b, n) ≤ 1 := by
    rw [← List.count_filter
      (p := fun e : Ev => decide (e.1 = c) && (e.2.1 == b))
      (l := schedule sk) hpred]
    rw [show (schedule sk).filter _ = proj c b (schedule sk) from rfl, hm,
      count_canon]
    split <;> omega
  by_contra hne
  rcases Nat.lt_or_ge i j with hij | hij
  · have := two_at_lt hij hi hj
    omega
  · have := two_at_lt (by omega : j < i) hj hi
    omega

-- ===================================== kernel-tier non-vacuity anchors
-- The theorems above hold for every well-formed skeleton; the anchors
-- below instantiate them on the smallest pin and make the kernel
-- COMPUTE the canon shapes, so `lake build` alone certifies the layer
-- is about the real merge output.

set_option maxRecDepth 16000 in
/-- The smallest pin's schedule holds no duplicate event: τ injectivity,
computed. -/
theorem smokeChain_schedule_nodup : (schedule Pin.smokeChain).Nodup := by
  decide

set_option maxRecDepth 16000 in
/-- The smallest pin's level-return stream is canon: the merge emits
`level I 0` receives with seqs `0, 1, …` in order. -/
theorem smokeChain_level_canon :
    proj (Chan.level Party.I 0) false (schedule Pin.smokeChain)
      = canon (Chan.level Party.I 0) false
          Pin.smokeChain.totalLeafReqs := by decide

end StreamingMirror.Sched
