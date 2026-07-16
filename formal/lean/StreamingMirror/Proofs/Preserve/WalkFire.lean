/-
Preservation for `walkFire` — the hardest walk action: an obligation
fires into its channel, and the embedded `normWalk` either stays in the
scope (the fired ledger is still incomplete) or advances the cursor to a
fresh walk at `scope + 1`. Staying re-establishes the per-child ledger
invariants from the committed-arm facts; advancing telescopes the live
ledger counts into the prefix sums (`wiresBefore`/`dsBefore`/`qsBefore`),
which is where the completion-count lemmas below earn their keep.
-/
import StreamingMirror.Proofs.Wiring
import StreamingMirror.Proofs.Preserve.Walk

namespace StreamingMirror.Model

variable {sk : Skel} {ax : AxMode} {s s' : State}

-- ================================================ counting infrastructure

/-- Filter counts are monotone under pointwise implication of the
predicates. -/
private theorem length_filter_mono {p q : Nat → Bool} {l : List Nat}
    (h : ∀ x ∈ l, p x = true → q x = true) :
    (l.filter p).length ≤ (l.filter q).length := by
  induction l with
  | nil => simp
  | cons x xs ih =>
      have hx := h x (List.mem_cons_self ..)
      have hxs : ∀ y ∈ xs, p y = true → q y = true :=
        fun y hy => h y (List.mem_cons_of_mem x hy)
      rw [List.filter_cons, List.filter_cons]
      by_cases hp : p x = true
      · rw [if_pos hp, if_pos (hx hp), List.length_cons, List.length_cons]
        exact Nat.succ_le_succ (ih hxs)
      · rw [if_neg hp]
        have := ih hxs
        split
        · simp only [List.length_cons]
          omega
        · omega

/-- Inserting index `i` into a ledger it misses raises the count by one:
`fun j => j == i || p j` is exactly `fireOblig`'s done-ledger update. -/
theorem length_filter_insert {p : Nat → Bool} {i fan : Nat}
    (hif : i < fan) (hfp : p i = false) :
    ((List.range fan).filter (fun j => j == i || p j)).length
      = ((List.range fan).filter p).length + 1 := by
  induction fan with
  | zero => omega
  | succ m ih =>
      rw [List.range_succ, List.filter_append, List.filter_append,
        List.length_append, List.length_append]
      by_cases him : i < m
      · have hmi : (m == i) = false := by simp; omega
        have htail : ([m].filter (fun j => j == i || p j)) = [m].filter p := by
          simp [List.filter_cons, hmi]
        rw [htail, ih him]
        omega
      · have him' : i = m := by omega
        subst him'
        have hhead : (List.range i).filter (fun j => j == i || p j)
            = (List.range i).filter p := by
          apply List.filter_congr
          intro j hj
          rw [List.mem_range] at hj
          have : (j == i) = false := by simp; omega
          simp [this]
        rw [hhead]
        simp [hfp]

/-- Bumping slot `i` of a Nat ledger raises the fold-sum by one:
`fun j => if j == i then q j + 1 else q j` is exactly `fireOblig`'s
query-ledger update. -/
theorem foldl_add_update {q : Nat → Nat} {i fan : Nat} (hif : i < fan) :
    (List.range fan).foldl
        (fun acc j => acc + (if j == i then q j + 1 else q j)) 0
      = (List.range fan).foldl (fun acc j => acc + q j) 0 + 1 := by
  rw [foldl_add_eq_sum, foldl_add_eq_sum]
  induction fan with
  | zero => omega
  | succ m ih =>
      rw [List.range_succ, List.map_append, List.map_append,
        List.sum_append, List.sum_append]
      by_cases him : i < m
      · have hmi : (m == i) = false := by simp; omega
        have := ih him
        simp only [List.map_cons, List.map_nil, hmi, Bool.false_eq_true,
          if_false, List.sum_cons, List.sum_nil]
        omega
      · have him' : i = m := by omega
        subst him'
        have hhead : (List.range i).map
            (fun j => if j == i then q j + 1 else q j)
            = (List.range i).map q := by
          apply List.map_congr_left
          intro j hj
          rw [List.mem_range] at hj
          have : (j == i) = false := by simp; omega
          simp [this]
        rw [hhead]
        simp
        omega

/-- `foldl_add_update` with the propositional-`ite` spelling that `simp`
normalization produces. -/
theorem foldl_add_update' {q : Nat → Nat} {i fan : Nat} (hif : i < fan) :
    (List.range fan).foldl
        (fun acc j => acc + (if j = i then q j + 1 else q j)) 0
      = (List.range fan).foldl (fun acc j => acc + q j) 0 + 1 := by
  rw [foldl_add_eq_sum, foldl_add_eq_sum]
  induction fan with
  | zero => omega
  | succ m ih =>
      rw [List.range_succ, List.map_append, List.map_append,
        List.sum_append, List.sum_append]
      by_cases him : i < m
      · have hmi : ¬(m = i) := by omega
        have := ih him
        simp only [List.map_cons, List.map_nil, if_neg hmi,
          List.sum_cons, List.sum_nil]
        omega
      · have him' : i = m := by omega
        subst him'
        have hhead : (List.range i).map
            (fun j => if j = i then q j + 1 else q j)
            = (List.range i).map q := by
          apply List.map_congr_left
          intro j hj
          rw [List.mem_range] at hj
          have hji : ¬(j = i) := by omega
          simp [hji]
        rw [hhead]
        simp
        omega

/-- A prefix-closed ledger is characterized pointwise by its count: the
inverse of `length_filter_of_frontier`. -/
theorem frontier_of_count {p : Nat → Bool} {i fan : Nat}
    (hcnt : ((List.range fan).filter p).length = i)
    (hclosed : ∀ j < fan, p j = true → j = 0 ∨ p (j - 1) = true) :
    ∀ j < fan, p j = decide (j < i) := by
  -- downward closure: anything below a set index is set
  have hdc : ∀ j < fan, p j = true → ∀ j2 ≤ j, p j2 = true := by
    intro j hj hpj j2 hj2
    have key : ∀ d, d ≤ j → p (j - d) = true := by
      intro d
      induction d with
      | zero => intro _; simpa using hpj
      | succ e ihe =>
          intro he
          have hpe : p (j - e) = true := ihe (by omega)
          have hje : j - e < fan := by omega
          rcases hclosed (j - e) hje hpe with h0 | hprev
          · have : j - (e + 1) = j - e := by omega
            rw [this]; exact hpe
          · have : j - (e + 1) = (j - e) - 1 := by omega
            rw [this]; exact hprev
    have : j2 = j - (j - j2) := by omega
    rw [this]; exact key (j - j2) (by omega)
  intro j hj
  cases hpj : p j with
  | true =>
      -- count ≥ j + 1, so j < i
      have hle : j + 1 ≤ fan := by omega
      have hmono : ((List.range fan).filter
          (fun x => decide (x < j + 1))).length
            ≤ ((List.range fan).filter p).length := by
        apply length_filter_mono
        intro x hx hxlt
        rw [decide_eq_true_eq] at hxlt
        exact hdc j hj hpj x (by omega)
      rw [length_filter_range_lt hle, hcnt] at hmono
      simp; omega
  | false =>
      -- everything set lies strictly below j, so i ≤ j
      have hmono : ((List.range fan).filter p).length
          ≤ ((List.range fan).filter (fun x => decide (x < j))).length := by
        apply length_filter_mono
        intro x hx hpx
        rw [List.mem_range] at hx
        rw [decide_eq_true_eq]
        by_contra hxj
        have : j ≤ x := by omega
        have := hdc x hx hpx j this
        rw [hpj] at this
        cases this
      rw [length_filter_range_lt (by omega), hcnt] at hmono
      simp; omega

/-- A filter over `range fan` collapses to `range n` when the predicate
is dead above `n`. -/
private theorem length_filter_range_ext {p : Nat → Bool} {n fan : Nat}
    (hn : n ≤ fan) (hz : ∀ j, n ≤ j → p j = false) :
    ((List.range fan).filter p).length
      = ((List.range n).filter p).length := by
  have hfan : fan = n + (fan - n) := by omega
  rw [hfan, List.range_add, List.filter_append, List.length_append,
    List.filter_map]
  have : (List.range (fan - n)).filter (p ∘ fun x => n + x) = [] := by
    rw [List.filter_eq_nil_iff]
    intro a _
    simp [hz (n + a) (by omega)]
  rw [this]
  simp

/-- Summing a constant-zero map is zero. -/
private theorem sum_map_zero (l : List Nat) :
    (l.map (fun _ : Nat => (0 : Nat))).sum = 0 := by
  induction l with
  | nil => rfl
  | cons x xs ih => simp [ih]

/-- A fold-sum over `range fan` collapses to `range n` when the summand
is dead above `n`. -/
private theorem sum_range_ext {q : Nat → Nat} {n fan : Nat}
    (hn : n ≤ fan) (hz : ∀ j, n ≤ j → q j = 0) :
    (List.range fan).foldl (fun acc j => acc + q j) 0
      = (List.range n).foldl (fun acc j => acc + q j) 0 := by
  rw [foldl_add_eq_sum, foldl_add_eq_sum]
  have hfan : fan = n + (fan - n) := by omega
  rw [hfan, List.range_add, List.map_append, List.sum_append,
    List.map_map]
  have hzero : (List.range (fan - n)).map (q ∘ fun x => n + x)
      = (List.range (fan - n)).map (fun _ => 0) := by
    apply List.map_congr_left
    intro a _
    exact hz (n + a) (by omega)
  rw [hzero, sum_map_zero]
  omega

/-- Positional filtering over indices equals filtering the list: the
bridge between `childIsD`'s `kids[j]?` reads and `kids.filter`. -/
private theorem length_filter_index (P : Nat → Bool) (l : List Nat) :
    ((List.range l.length).filter
        (fun j => match l[j]? with
          | some k => P k
          | none => false)).length
      = (l.filter P).length := by
  induction l with
  | nil => simp
  | cons x xs ih =>
      rw [show (x :: xs).length = xs.length + 1 from rfl,
        List.range_succ_eq_map, List.filter_cons, List.filter_cons]
      have hc : (match (x :: xs)[0]? with
          | some k => P k
          | none => false) = P x := by simp
      have htail : ((List.map Nat.succ (List.range xs.length)).filter
          (fun j => match (x :: xs)[j]? with
            | some k => P k
            | none => false)).length
          = (xs.filter P).length := by
        rw [List.filter_map, List.length_map,
          List.filter_congr (q := fun j => match xs[j]? with
            | some k => P k
            | none => false)
            (fun j _ => by simp [Function.comp, List.getElem?_cons_succ])]
        exact ih
      rw [hc]
      by_cases hPx : P x = true
      · rw [if_pos hPx, if_pos hPx, List.length_cons, List.length_cons,
          htail]
      · rw [if_neg hPx, if_neg hPx, htail]

-- ==================================================== skeleton structure

/-- A D child is a real child: its index is within the child count. -/
private theorem lt_nChildren_of_childIsD {h sc j : Nat}
    (hd : sk.childIsD h sc j = true) : j < sk.nChildren h sc := by
  unfold Skel.childIsD at hd
  by_cases hh : (h == 0) = true
  · rw [if_pos hh] at hd; cases hd
  · rw [if_neg hh] at hd
    unfold Skel.nChildren
    rw [if_neg hh]
    cases hj : (sk.scope sc).kids[j]? with
    | none => rw [hj] at hd; cases hd
    | some k =>
        obtain ⟨hlt, -⟩ := List.getElem?_eq_some_iff.mp hj
        exact hlt

/-- Only non-leaf stages have D children. -/
private theorem ne_zero_of_childIsD {h sc j : Nat}
    (hd : sk.childIsD h sc j = true) : h ≠ 0 := by
  intro h0
  subst h0
  unfold Skel.childIsD at hd
  simp at hd

/-- The D-child test is dead past the child count. -/
private theorem childIsD_eq_false_of_ge {h sc j : Nat}
    (hj : sk.nChildren h sc ≤ j) : sk.childIsD h sc j = false := by
  unfold Skel.childIsD
  by_cases hh : (h == 0) = true
  · rw [if_pos hh]
  · rw [if_neg hh]
    unfold Skel.nChildren at hj
    rw [if_neg hh] at hj
    rw [List.getElem?_eq_none hj]

/-- Query budgets exist only for D children. -/
private theorem qCount_eq_zero_of_not_childIsD {h sc j : Nat}
    (hd : sk.childIsD h sc j = false) : sk.qCount h sc j = 0 := by
  unfold Skel.qCount
  rw [hd]
  simp

private theorem qCount_eq_zero_of_ge {h sc j : Nat}
    (hj : sk.nChildren h sc ≤ j) : sk.qCount h sc j = 0 :=
  qCount_eq_zero_of_not_childIsD (childIsD_eq_false_of_ge hj)

-- ========================================================== telescopes

/-- One step of a prefix-sum fold over a stage's scope list. -/
private theorem foldl_take_succ (l : List Nat) (f : Nat → Nat) {k : Nat}
    (hk : k < l.length) :
    (l.take (k + 1)).foldl (fun acc s => acc + f s) 0
      = (l.take k).foldl (fun acc s => acc + f s) 0 + f (l.getD k 0) := by
  rw [List.take_add_one, List.getElem?_eq_getElem hk,
    show (some l[k]).toList = [l[k]] from rfl, List.foldl_append,
    List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hk]
  rfl

theorem wiresBefore_succ (sk : Skel) {h k : Nat} (hk : k < sk.stageLen h) :
    sk.wiresBefore h (k + 1)
      = sk.wiresBefore h k + sk.nChildren h (sk.stageScope h k) :=
  foldl_take_succ (sk.stageScopes h) (sk.nChildren h) hk

theorem dsBefore_succ (sk : Skel) {h k : Nat} (hk : k < sk.stageLen h) :
    sk.dsBefore h (k + 1)
      = sk.dsBefore h k + sk.dOf h (sk.stageScope h k) :=
  foldl_take_succ (sk.stageScopes h) (sk.dOf h) hk

theorem qsBefore_succ (sk : Skel) {h k : Nat} (hk : k < sk.stageLen h) :
    sk.qsBefore h (k + 1)
      = sk.qsBefore h k + sk.qOf h (sk.stageScope h k) :=
  foldl_take_succ (sk.stageScopes h) (sk.qOf h) hk

-- ================================================= completion counting

/-- A bounded ledger that covers its bound counts to exactly the bound:
the wire ledger of a completed scope counts to `nChildren`. -/
private theorem wireCount_of_complete {p : Nat → Bool} {n fan : Nat}
    (hn : n ≤ fan)
    (hb : ∀ j < fan, p j = true → j < n)
    (hcp : ∀ j < n, p j = true) :
    ((List.range fan).filter p).length = n := by
  have hpoint : ∀ j ∈ List.range fan, p j = decide (j < n) := by
    intro j hj
    rw [List.mem_range] at hj
    by_cases hjn : j < n
    · simp [hcp j hjn, hjn]
    · cases hpj : p j with
      | false => simp [hjn]
      | true => exact absurd (hb j hj hpj) hjn
  rw [List.filter_congr hpoint]
  exact length_filter_range_lt hn

/-- A ledger that holds exactly the D children counts to `dOf`: the res
ledger of a completed scope. -/
private theorem count_eq_dOf (sk : Skel) {h sc fan : Nat} {p : Nat → Bool}
    (hfan : sk.nChildren h sc ≤ fan)
    (hb : ∀ j < fan, p j = true →
      j < sk.nChildren h sc ∧ sk.childIsD h sc j = true)
    (hcp : ∀ j < sk.nChildren h sc, sk.childIsD h sc j = true → p j = true) :
    ((List.range fan).filter p).length = sk.dOf h sc := by
  have hpoint : ∀ j ∈ List.range fan, p j = sk.childIsD h sc j := by
    intro j hj
    rw [List.mem_range] at hj
    cases hpj : p j with
    | true => exact ((hb j hj hpj).2).symm
    | false =>
        cases hd : sk.childIsD h sc j with
        | false => rfl
        | true =>
            have := hcp j (lt_nChildren_of_childIsD hd) hd
            rw [hpj] at this
            cases this
  rw [List.filter_congr hpoint]
  by_cases hh : h = 0
  · subst hh
    have hdead : ∀ j ∈ List.range fan, sk.childIsD 0 sc j = false := by
      intro j _
      unfold Skel.childIsD
      simp
    rw [List.filter_congr (q := fun _ => false) hdead]
    simp [Skel.dOf]
  · have hkids : sk.nChildren h sc = (sk.scope sc).kids.length := by
      unfold Skel.nChildren
      simp [hh]
    rw [length_filter_range_ext hfan
      (fun j hj => childIsD_eq_false_of_ge hj)]
    rw [hkids]
    have hpoint2 : ∀ j ∈ List.range (sk.scope sc).kids.length,
        sk.childIsD h sc j = (match (sk.scope sc).kids[j]? with
          | some k => (sk.scope k).kind == Kind.D
          | none => false) := by
      intro j _
      unfold Skel.childIsD
      rw [if_neg (by simpa using hh)]
      rfl
    rw [List.filter_congr hpoint2, length_filter_index]
    unfold Skel.dOf Skel.dCount
    rw [if_neg (by simpa using hh)]

/-- A query ledger that is pointwise the budget sums to `qOf`. -/
private theorem qSum_eq_qOf (sk : Skel) {h sc fan : Nat} {q : Nat → Nat}
    (hfan : sk.nChildren h sc ≤ fan)
    (hpoint : ∀ j < fan, q j = sk.qCount h sc j) :
    (List.range fan).foldl (fun acc j => acc + q j) 0 = sk.qOf h sc := by
  have h1 : (List.range fan).foldl (fun acc j => acc + q j) 0
      = (List.range fan).foldl (fun acc j => acc + sk.qCount h sc j) 0 := by
    rw [foldl_add_eq_sum, foldl_add_eq_sum,
      List.map_congr_left (fun j hj => hpoint j (List.mem_range.mp hj))]
  rw [h1, sum_range_ext hfan (fun j hj => qCount_eq_zero_of_ge hj)]
  rfl

/-- A budget-bounded query ledger that saturates the D children sums to
`qOf`: non-D and out-of-range budgets are zero, so the bound pins them. -/
private theorem qSum_of_complete (sk : Skel) {h sc fan : Nat} {q : Nat → Nat}
    (hfan : sk.nChildren h sc ≤ fan)
    (hb : ∀ j < fan, q j ≤ sk.qCount h sc j)
    (hcp : ∀ j < sk.nChildren h sc, sk.childIsD h sc j = true →
      q j = sk.qCount h sc j) :
    (List.range fan).foldl (fun acc j => acc + q j) 0 = sk.qOf h sc := by
  apply qSum_eq_qOf sk hfan
  intro j hj
  by_cases hd : sk.childIsD h sc j = true
  · exact hcp j (lt_nChildren_of_childIsD hd) hd
  · have h0 : sk.qCount h sc j = 0 :=
      qCount_eq_zero_of_not_childIsD (by simpa using hd)
    have := hb j hj
    omega

-- ==================================================== fresh-walk facts

/-- A freshly cut walk cursor satisfies the local invariant, at any
cursor position up to (and including) the end of the stage. -/
theorem wkLocalOk_fresh (pk : Party × Nat) (k : Nat)
    (hk : k ≤ sk.stageLen pk.2) (hw : s'.walk pk = freshWalk sk pk.2 k) :
    wkLocalOk sk ax s' pk = true := by
  by_cases hl : k < sk.stageLen pk.2
  · simp [wkLocalOk, hw, freshWalk, hl]
  · have hz : k = sk.stageLen pk.2 := by omega
    simp [wkLocalOk, hw, freshWalk, hz]

private theorem wkWireRecvd_fresh {t : State} (pk : Party × Nat) {k : Nat}
    (hk : k ≤ sk.stageLen pk.2) (hw : t.walk pk = freshWalk sk pk.2 k) :
    wkWireRecvd sk t pk = k := by
  by_cases hl : k < sk.stageLen pk.2
  · simp [wkWireRecvd, hw, freshWalk, hl]
  · have hz : k = sk.stageLen pk.2 := by omega
    simp [wkWireRecvd, hw, freshWalk, hl]
    omega

private theorem wkAskedRecvd_fresh {t : State} (pk : Party × Nat) {k : Nat}
    (hk : k ≤ sk.stageLen pk.2) (hw : t.walk pk = freshWalk sk pk.2 k) :
    wkAskedRecvd sk t pk = k := by
  by_cases hl : k < sk.stageLen pk.2
  · simp [wkAskedRecvd, hw, freshWalk, hl]
  · have hz : k = sk.stageLen pk.2 := by omega
    simp [wkAskedRecvd, hw, freshWalk, hl]
    omega

-- ===================================================== flow frame plumbing

private theorem other_ne_self (p : Party) : p.other ≠ p := by
  cases p <;> simp [Party.other]

/-- A key of the other party never collides with `pk`. -/
private theorem key_other_ne (p : Party) (a b : Nat) :
    ((p.other, a) : Party × Nat) ≠ (p, b) := by
  intro hcon
  have hfst := congrArg Prod.fst hcon
  simp at hfst
  exact other_ne_self p hfst

/-- Producer counts never read channel occupancy. -/
private theorem sentOf_chan_irrel (ch : Chan → Nat) (c : Chan) :
    sentOf sk { s with chan := ch } c = sentOf sk s c := by
  cases c <;> rfl

/-- Consumer counts never read channel occupancy. -/
private theorem recvdOf_chan_irrel (ch : Chan → Nat) (c : Chan) :
    recvdOf sk { s with chan := ch } c = recvdOf sk s c := by
  cases c <;> rfl

/-- Sends frame for a walk-plus-channel update: each of `pk`'s producer
counts is unchanged unless it feeds the touched channel `c₀`, so every
channel other than `c₀` reads the same producer count. The two input
channels of `pk` and the leaf-responder `askedOut` alias route to counts
at other keys, untouched by the update. -/
private theorem sentOf_fire_frame (hwf : sk.wellFormed = true)
    (ch : Chan → Nat) (pk : Party × Nat)
    (ws' : WalkSt) (hmem : pk ∈ sk.walkKeys) (c₀ : Chan)
    (hW : c₀ ≠ wireOut pk →
      wkWireSent sk (setWalk { s with chan := ch } pk ws') pk
        = wkWireSent sk s pk)
    (hR : c₀ ≠ lowerOut pk →
      wkResSent sk (setWalk { s with chan := ch } pk ws') pk
        = wkResSent sk s pk)
    (hQ : c₀ ≠ askedOut pk →
      wkQSentTot sk (setWalk { s with chan := ch } pk ws') pk
        = wkQSentTot sk s pk)
    (hP : c₀ ≠ upperOut pk →
      wkParentSent (setWalk { s with chan := ch } pk ws') pk
        = wkParentSent s pk)
    {c : Chan} (hc : c ∈ allChans sk) (hne : c ≠ c₀) :
    sentOf sk (setWalk { s with chan := ch } pk ws') c = sentOf sk s c := by
  by_cases h1 : c = wireOut pk
  · subst h1
    rw [sentOf_wireOut hmem, sentOf_wireOut hmem]
    exact hW (Ne.symm hne)
  by_cases h2 : c = lowerOut pk
  · subst h2
    rw [sentOf_lowerOut, sentOf_lowerOut]
    exact hR (Ne.symm hne)
  by_cases h4 : c = upperOut pk
  · subst h4
    rw [sentOf_upperOut, sentOf_upperOut]
    exact hP (Ne.symm hne)
  by_cases h3 : c = askedOut pk
  · subst h3
    by_cases hp1 : 1 ≤ pk.2
    · rw [sentOf_askedOut hwf hmem hp1, sentOf_askedOut hwf hmem hp1]
      exact hQ (Ne.symm hne)
    · have h0 : pk.2 = 0 := by omega
      have hI1 : ((Party.I, 1) : Party × Nat) ≠ pk := by
        intro hcon
        rw [← hcon] at h0
        simp at h0
      rw [show askedOut pk = Chan.leafRequests from by simp [askedOut, h0]]
      simp [sentOf, wkQSentTot, wkQSum, setWalk_walk_ne _ _ hI1]
  by_cases h5 : c = wireIn pk
  · subst h5
    by_cases hh : pk.2 + 1 = sk.rootH
    · simp [sentOf, wireIn, hh, setWalk]
    · have hne1 : ((pk.1.other, pk.2 + 1) : Party × Nat) ≠ pk :=
        key_other_ne pk.1 (pk.2 + 1) pk.2
      simp [sentOf, wireIn, hh, wkWireSent, wkWireCount,
        setWalk_walk_ne _ _ hne1]
  by_cases h6 : c = askedIn pk
  · subst h6
    by_cases hA : pk.1 = Party.I ∧ pk.2 = sk.rootH - 1
    · simp [sentOf, askedIn, hA, setWalk]
    · by_cases hB : pk.1 = Party.R ∧ pk.2 = sk.rootH - 2
      · simp [sentOf, askedIn, hB, setWalk]
      · have hne2 : ((pk.1, pk.2 + 2) : Party × Nat) ≠ pk := by
          intro hcon
          have := congrArg Prod.snd hcon
          simp at this
        simp [sentOf, askedIn, hA, hB, wkQSentTot, wkQSum,
          setWalk_walk_ne _ _ hne2]
  · have := (flow_setWalk_frame hwf { s with chan := ch } pk ws' hc
      h1 h2 h3 h4 h5 h6).1
    rw [sentOf_chan_irrel] at this
    exact this

/-- Receives frame for a walk-plus-channel update whose two consumer
counts at `pk` are unchanged: every channel reads the same consumer
count. `pk`'s own output channels are consumed elsewhere (a walk never
feeds itself), so they frame too. -/
private theorem recvdOf_fire_frame (hwf : sk.wellFormed = true)
    (ch : Chan → Nat) (pk : Party × Nat)
    (ws' : WalkSt) (hmem : pk ∈ sk.walkKeys)
    (hWr : wkWireRecvd sk (setWalk { s with chan := ch } pk ws') pk
      = wkWireRecvd sk s pk)
    (hAr : wkAskedRecvd sk (setWalk { s with chan := ch } pk ws') pk
      = wkAskedRecvd sk s pk)
    {c : Chan} (hc : c ∈ allChans sk) :
    recvdOf sk (setWalk { s with chan := ch } pk ws') c
      = recvdOf sk s c := by
  by_cases h5 : c = wireIn pk
  · subst h5
    rw [recvdOf_wireIn hmem, recvdOf_wireIn hmem]
    exact hWr
  by_cases h6 : c = askedIn pk
  · subst h6
    rw [recvdOf_askedIn, recvdOf_askedIn]
    exact hAr
  by_cases h1 : c = wireOut pk
  · subst h1
    have hlt : pk.2 < sk.rootH := by
      rcases walkKeys_cases hmem with ⟨-, -, h⟩ | ⟨-, h⟩ <;> omega
    have hh : ¬(pk.2 = sk.rootH) := by omega
    by_cases hz : pk.1 = Party.R ∧ pk.2 = 0
    · have hz0 : ¬(0 = sk.rootH) := by omega
      simp [recvdOf, wireOut, hz, hz0, absorbWireRecvd, setWalk]
    · have hne1 : ((pk.1.other, pk.2 - 1) : Party × Nat) ≠ pk :=
        key_other_ne pk.1 (pk.2 - 1) pk.2
      simp [recvdOf, wireOut, hh, hz, wkWireRecvd,
        setWalk_walk_ne _ _ hne1]
  by_cases h2 : c = lowerOut pk
  · subst h2
    simp [recvdOf, lowerOut, asmResRecvd, setWalk]
  by_cases h4 : c = upperOut pk
  · subst h4
    simp [recvdOf, upperOut, asmResRecvd, setWalk]
  by_cases h3 : c = askedOut pk
  · subst h3
    by_cases hp2 : pk.2 < 2
    · rw [show askedOut pk = Chan.leafRequests from by simp [askedOut, hp2]]
      simp [recvdOf, absorbAskedRecvd, setWalk]
    · rw [show askedOut pk = Chan.asked pk.1 (pk.2 - 2) from by
        simp [askedOut, hp2]]
      have hne2 : ((pk.1, pk.2 - 2) : Party × Nat) ≠ pk := by
        intro hcon
        have := congrArg Prod.snd hcon
        simp at this
        omega
      simp [recvdOf, wkAskedRecvd, setWalk_walk_ne _ _ hne2]
  · have := (flow_setWalk_frame hwf { s with chan := ch } pk ws' hc
      h1 h2 h3 h4 h5 h6).2
    rw [recvdOf_chan_irrel] at this
    exact this

/-- Assembles the flow field of `InvP` for a fire step: the touched
channel's occupancy rises from empty to one exactly as its producer
count rises by one; every other channel frames. -/
private theorem flow_fire_assemble (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (hmem' : pk ∈ sk.walkKeys) (c₀ : Chan) (W : WalkSt)
    (hs' : setWalk { s with chan := bump s.chan c₀ 1 } pk W = s')
    (hchan0 : s.chan c₀ = 0) (hcap1 : sk.cap c₀ = 1)
    (htouch : sentOf sk (setWalk { s with chan := bump s.chan c₀ 1 } pk W) c₀
      = sentOf sk s c₀ + 1)
    (hW : c₀ ≠ wireOut pk →
      wkWireSent sk (setWalk { s with chan := bump s.chan c₀ 1 } pk W) pk
        = wkWireSent sk s pk)
    (hR : c₀ ≠ lowerOut pk →
      wkResSent sk (setWalk { s with chan := bump s.chan c₀ 1 } pk W) pk
        = wkResSent sk s pk)
    (hQ : c₀ ≠ askedOut pk →
      wkQSentTot sk (setWalk { s with chan := bump s.chan c₀ 1 } pk W) pk
        = wkQSentTot sk s pk)
    (hP : c₀ ≠ upperOut pk →
      wkParentSent (setWalk { s with chan := bump s.chan c₀ 1 } pk W) pk
        = wkParentSent s pk)
    (hWr : wkWireRecvd sk (setWalk { s with chan := bump s.chan c₀ 1 } pk W) pk
      = wkWireRecvd sk s pk)
    (hAr : wkAskedRecvd sk (setWalk { s with chan := bump s.chan c₀ 1 } pk W) pk
      = wkAskedRecvd sk s pk)
    (hi : InvP sk ax s) :
    ∀ c ∈ allChans sk,
      s'.chan c + recvdOf sk s' c = sentOf sk s' c ∧ s'.chan c ≤ sk.cap c := by
  intro c hc
  obtain ⟨heq, hcap⟩ := hi.flow c hc
  have hchan : s'.chan = bump s.chan c₀ 1 := by
    rw [← hs']
    rfl
  have hrecv : recvdOf sk s' c = recvdOf sk s c := by
    rw [← hs']
    exact recvdOf_fire_frame hwf _ pk W hmem' hWr hAr hc
  by_cases hne : c = c₀
  · subst hne
    have hsent : sentOf sk s' c = sentOf sk s c + 1 := by
      rw [← hs']
      exact htouch
    rw [hchan, hrecv, hsent, bump_one, hcap1]
    exact ⟨by omega, by omega⟩
  · have hsent : sentOf sk s' c = sentOf sk s c := by
      rw [← hs']
      exact sentOf_fire_frame hwf _ pk W hmem' c₀ hW hR hQ hP hc hne
    rw [hchan, hsent, hrecv, bump_ne _ _ hne]
    exact ⟨heq, hcap⟩

-- ================================================ per-obligation lemmas

/-- `walkFire` on a committed `.parent`: the ledgers are untouched, so
the per-child block carries over verbatim; only `parentDone` flips, and
`upperOut` rises by one (staying) or telescopes into the next scope's
cursor (advancing). -/
private theorem preserve_walkFire_parent (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some Oblig.parent)
    (hchan0 : s.chan (upperOut pk) = 0)
    (hs' : setWalk { s with chan := bump s.chan (upperOut pk) 1 } pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) Oblig.parent)) = s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  have hwk := hi.wk pk hmem'
  simp only [wkLocalOk] at hwk
  rw [hph2, hcm] at hwk
  simp at hwk
  obtain ⟨hslt, ⟨hnsc, hC⟩, hpd, hd2⟩ := hwk
  have hn : sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
      ≤ sk.fan := nChildren_le_fan hwf hslt
  -- the fired walk record, with the phase pinned to 2
  have hfw : fireOblig (s.walk pk) Oblig.parent =
      { scope := (s.walk pk).scope, phase := 2,
        wireDone := (s.walk pk).wireDone, resDone := (s.walk pk).resDone,
        qSent := (s.walk pk).qSent, parentDone := true,
        committed := none } := by
    simp [fireOblig, hph2]
  rw [hfw] at hs'
  by_cases hadv : scopeComplete sk pk.2
      { scope := (s.walk pk).scope, phase := 2,
        wireDone := (s.walk pk).wireDone, resDone := (s.walk pk).resDone,
        qSent := (s.walk pk).qSent, parentDone := true,
        committed := none } = true
  · -- the scope completes: the walk advances to a fresh cursor
    rw [show normWalk sk pk.2
        { scope := (s.walk pk).scope, phase := 2,
          wireDone := (s.walk pk).wireDone, resDone := (s.walk pk).resDone,
          qSent := (s.walk pk).qSent, parentDone := true,
          committed := none }
        = freshWalk sk pk.2 ((s.walk pk).scope + 1) from by
      simp [normWalk, hadv]] at hs'
    have hwalk' : s'.walk pk = freshWalk sk pk.2 ((s.walk pk).scope + 1) := by
      rw [← hs']
      exact setWalk_walk_self _ _ _
    -- completion facts on the (parent-fired) ledger
    have hnge : ¬((s.walk pk).scope ≥ sk.stageLen pk.2) := by omega
    simp only [scopeComplete] at hadv
    rw [if_neg hnge] at hadv
    simp at hadv
    -- ledger counts of the completed scope
    have hwc : ((List.range sk.fan).filter
        (fun j => (s.walk pk).wireDone j)).length
        = sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope) := by
      refine wireCount_of_complete hn ?_ (fun j hj => (hadv j hj).1)
      intro j hj hwd
      rcases (hC j hj).1.1.1.1.1.1.1.1.1 with hf | ⟨hlt, -⟩
      · rw [hwd] at hf; cases hf
      · exact hlt
    have hrc : ((List.range sk.fan).filter
        (fun j => (s.walk pk).resDone j)).length
        = sk.dOf pk.2 (sk.stageScope pk.2 (s.walk pk).scope) := by
      refine count_eq_dOf sk hn ?_ ?_
      · intro j hj hrd
        rcases (hC j hj).1.1.1.1.1.1.1.1.2 with hf | h
        · rw [hrd] at hf; cases hf
        · exact h
      · intro j hj hd
        rcases (hadv j hj).2 with h | h
        · rw [hd] at h; cases h
        · exact h.1
    have hqc : (List.range sk.fan).foldl
        (fun acc j => acc + (s.walk pk).qSent j) 0
        = sk.qOf pk.2 (sk.stageScope pk.2 (s.walk pk).scope) := by
      refine qSum_of_complete sk hn
        (fun j hj => (hC j hj).1.1.1.1.1.1.1.2) ?_
      intro j hj hd
      rcases (hadv j hj).2 with h | h
      · rw [hd] at h; cases h
      · exact h.2
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_,
      flow_fire_assemble hwf pk hmem' (upperOut pk) _ hs' hchan0 rfl
        ?_ ?_ ?_ ?_ ?_ ?_ ?_ hi⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        exact wkLocalOk_fresh pk' _ (by omega) hwalk'
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']
          exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hi.wk pk' hpk'
    · rw [← hs']
      exact hi.asm pk' hpk'
    · rw [← hs']
      exact hi.top
    · -- touched channel: parent send telescopes into the next cursor
      rw [sentOf_upperOut, sentOf_upperOut]
      by_cases hl2 : (s.walk pk).scope + 1 < sk.stageLen pk.2 <;>
        simp [wkParentSent, freshWalk, hph2, hpd, hl2]
    · -- wire count telescopes
      intro _
      simp only [wkWireSent, wkWireCount, setWalk_walk_self, freshWalk]
      rw [wiresBefore_succ sk hslt, hwc]
      simp
    · intro _
      simp only [wkResSent, wkResCount, setWalk_walk_self, freshWalk]
      rw [dsBefore_succ sk hslt, hrc]
      simp
    · intro _
      simp only [wkQSentTot, wkQSum, setWalk_walk_self, freshWalk]
      rw [qsBefore_succ sk hslt, hqc]
      simp [foldl_const]
    · exact fun hab => absurd rfl hab
    · rw [wkWireRecvd_fresh pk (by omega) (setWalk_walk_self _ _ _)]
      simp [wkWireRecvd, hph2]
    · rw [wkAskedRecvd_fresh pk (by omega) (setWalk_walk_self _ _ _)]
      simp [wkAskedRecvd, hph2]
  · -- the scope is still incomplete: the walk stays put
    have hadv' : scopeComplete sk pk.2
        { scope := (s.walk pk).scope, phase := 2,
          wireDone := (s.walk pk).wireDone, resDone := (s.walk pk).resDone,
          qSent := (s.walk pk).qSent, parentDone := true,
          committed := none } = false := by
      simpa using hadv
    rw [show normWalk sk pk.2
        { scope := (s.walk pk).scope, phase := 2,
          wireDone := (s.walk pk).wireDone, resDone := (s.walk pk).resDone,
          qSent := (s.walk pk).qSent, parentDone := true,
          committed := none }
        = { scope := (s.walk pk).scope, phase := 2,
            wireDone := (s.walk pk).wireDone, resDone := (s.walk pk).resDone,
            qSent := (s.walk pk).qSent, parentDone := true,
            committed := none } from by
      simp [normWalk, hadv']] at hs'
    have hwalk' : s'.walk pk =
        { scope := (s.walk pk).scope, phase := 2,
          wireDone := (s.walk pk).wireDone, resDone := (s.walk pk).resDone,
          qSent := (s.walk pk).qSent, parentDone := true,
          committed := none } := by
      rw [← hs']
      exact setWalk_walk_self _ _ _
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_,
      flow_fire_assemble hwf pk hmem' (upperOut pk) _ hs' hchan0 rfl
        ?_ ?_ ?_ ?_ ?_ ?_ ?_ hi⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        simp only [wkLocalOk, hwalk']
        simp
        exact ⟨hslt, hadv', hC⟩
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']
          exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hi.wk pk' hpk'
    · rw [← hs']
      exact hi.asm pk' hpk'
    · rw [← hs']
      exact hi.top
    · -- touched channel: parentDone flips 0 → 1
      rw [sentOf_upperOut, sentOf_upperOut]
      simp [wkParentSent, hph2, hpd]
    · intro _
      simp [wkWireSent, wkWireCount]
    · intro _
      simp [wkResSent, wkResCount]
    · intro _
      simp [wkQSentTot, wkQSum]
    · exact fun hab => absurd rfl hab
    · simp [wkWireRecvd, hph2]
    · simp [wkAskedRecvd, hph2]

/-- `walkFire` on a committed `.wire i`: the arm pins `i` as the wire
frontier, so the fired ledger is the `< i + 1` prefix; staying keeps the
prefix shape, advancing forces `i + 1 = nChildren` and telescopes the
count into `wiresBefore`. -/
private theorem preserve_walkFire_wire (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (i : Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some (Oblig.wire i))
    (hchan0 : s.chan (wireOut pk) = 0)
    (hs' : setWalk { s with chan := bump s.chan (wireOut pk) 1 } pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) (Oblig.wire i))) = s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  have hwk := hi.wk pk hmem'
  simp only [wkLocalOk] at hwk
  rw [hph2, hcm] at hwk
  simp at hwk
  obtain ⟨hslt, ⟨-, hC⟩, ⟨hieq, hin⟩, hd4⟩ := hwk
  have hn : sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
      ≤ sk.fan := nChildren_le_fan hwf hslt
  -- the committed arm pins the ledger to the `< i` prefix
  have hcnt : ((List.range sk.fan).filter
      (fun j => (s.walk pk).wireDone j)).length = i := by
    have h2 := hieq
    simp only [wkWireCount] at h2
    exact h2.symm
  have hclosed : ∀ j < sk.fan, (s.walk pk).wireDone j = true →
      j = 0 ∨ (s.walk pk).wireDone (j - 1) = true := by
    intro j hj hwd
    rcases (hC j hj).1.1.1.1.1.1.1.1.1 with hf | ⟨-, h0⟩
    · rw [hwd] at hf; cases hf
    · exact h0
  have hfront := frontier_of_count hcnt hclosed
  have hifan : i < sk.fan := by omega
  have hwdi : (s.walk pk).wireDone i = false := by
    have h2 := hfront i hifan
    simpa using h2
  have hfw : fireOblig (s.walk pk) (Oblig.wire i) =
      { scope := (s.walk pk).scope, phase := 2,
        wireDone := fun j => j == i || (s.walk pk).wireDone j,
        resDone := (s.walk pk).resDone,
        qSent := (s.walk pk).qSent, parentDone := (s.walk pk).parentDone,
        committed := none } := by
    simp [fireOblig, hph2]
  rw [hfw] at hs'
  by_cases hadv : scopeComplete sk pk.2
      { scope := (s.walk pk).scope, phase := 2,
        wireDone := fun j => j == i || (s.walk pk).wireDone j,
        resDone := (s.walk pk).resDone,
        qSent := (s.walk pk).qSent, parentDone := (s.walk pk).parentDone,
        committed := none } = true
  · -- the scope completes: `i` was the last missing wire
    rw [show normWalk sk pk.2
        { scope := (s.walk pk).scope, phase := 2,
          wireDone := fun j => j == i || (s.walk pk).wireDone j,
          resDone := (s.walk pk).resDone,
          qSent := (s.walk pk).qSent, parentDone := (s.walk pk).parentDone,
          committed := none }
        = freshWalk sk pk.2 ((s.walk pk).scope + 1) from by
      simp [normWalk, hadv]] at hs'
    have hwalk' : s'.walk pk = freshWalk sk pk.2 ((s.walk pk).scope + 1) := by
      rw [← hs']
      exact setWalk_walk_self _ _ _
    have hnge : ¬((s.walk pk).scope ≥ sk.stageLen pk.2) := by omega
    simp only [scopeComplete] at hadv
    rw [if_neg hnge] at hadv
    simp at hadv
    obtain ⟨hpdT, hcompl⟩ := hadv
    -- the fired frontier covers the scope, so i + 1 = nChildren
    have hn1 : sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
        = i + 1 := by
      by_contra hcon
      have hi1n : i + 1
          < sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope) := by
        omega
      rcases (hcompl (i + 1) hi1n).1 with he | hwd
      · omega
      · have h2 := hfront (i + 1) (by omega)
        rw [hwd] at h2
        have := of_decide_eq_true h2.symm
        omega
    have hrc : ((List.range sk.fan).filter
        (fun j => (s.walk pk).resDone j)).length
        = sk.dOf pk.2 (sk.stageScope pk.2 (s.walk pk).scope) := by
      refine count_eq_dOf sk hn ?_ ?_
      · intro j hj hrd
        rcases (hC j hj).1.1.1.1.1.1.1.1.2 with hf | h
        · rw [hrd] at hf; cases hf
        · exact h
      · intro j hj hd
        rcases (hcompl j hj).2 with h | h
        · rw [hd] at h; cases h
        · exact h.1
    have hqc : (List.range sk.fan).foldl
        (fun acc j => acc + (s.walk pk).qSent j) 0
        = sk.qOf pk.2 (sk.stageScope pk.2 (s.walk pk).scope) := by
      refine qSum_of_complete sk hn
        (fun j hj => (hC j hj).1.1.1.1.1.1.1.2) ?_
      intro j hj hd
      rcases (hcompl j hj).2 with h | h
      · rw [hd] at h; cases h
      · exact h.2
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_,
      flow_fire_assemble hwf pk hmem' (wireOut pk) _ hs' hchan0 rfl
        ?_ ?_ ?_ ?_ ?_ ?_ ?_ hi⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        exact wkLocalOk_fresh pk' _ (by omega) hwalk'
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']
          exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hi.wk pk' hpk'
    · rw [← hs']
      exact hi.asm pk' hpk'
    · rw [← hs']
      exact hi.top
    · -- touched channel: the wire count telescopes into `wiresBefore`
      rw [sentOf_wireOut hmem', sentOf_wireOut hmem']
      simp only [wkWireSent, wkWireCount, setWalk_walk_self, freshWalk]
      rw [wiresBefore_succ sk hslt, hcnt, hn1]
      have hff : ((List.range sk.fan).filter (fun _ : Nat => false)).length
          = 0 := by simp
      omega
    · exact fun hab => absurd rfl hab
    · intro _
      simp only [wkResSent, wkResCount, setWalk_walk_self, freshWalk]
      rw [dsBefore_succ sk hslt, hrc]
      simp
    · intro _
      simp only [wkQSentTot, wkQSum, setWalk_walk_self, freshWalk]
      rw [qsBefore_succ sk hslt, hqc]
      simp [foldl_const]
    · intro _
      by_cases hl2 : (s.walk pk).scope + 1 < sk.stageLen pk.2 <;>
        simp [wkParentSent, freshWalk, hph2, hpdT, hl2]
    · rw [wkWireRecvd_fresh pk (by omega) (setWalk_walk_self _ _ _)]
      simp [wkWireRecvd, hph2]
    · rw [wkAskedRecvd_fresh pk (by omega) (setWalk_walk_self _ _ _)]
      simp [wkAskedRecvd, hph2]
  · -- the scope is still incomplete: the frontier moves to `i + 1`
    have hadv' : scopeComplete sk pk.2
        { scope := (s.walk pk).scope, phase := 2,
          wireDone := fun j => j == i || (s.walk pk).wireDone j,
          resDone := (s.walk pk).resDone,
          qSent := (s.walk pk).qSent, parentDone := (s.walk pk).parentDone,
          committed := none } = false := by
      simpa using hadv
    rw [show normWalk sk pk.2
        { scope := (s.walk pk).scope, phase := 2,
          wireDone := fun j => j == i || (s.walk pk).wireDone j,
          resDone := (s.walk pk).resDone,
          qSent := (s.walk pk).qSent, parentDone := (s.walk pk).parentDone,
          committed := none }
        = { scope := (s.walk pk).scope, phase := 2,
            wireDone := fun j => j == i || (s.walk pk).wireDone j,
            resDone := (s.walk pk).resDone,
            qSent := (s.walk pk).qSent, parentDone := (s.walk pk).parentDone,
            committed := none } from by
      simp [normWalk, hadv']] at hs'
    have hwalk' : s'.walk pk =
        { scope := (s.walk pk).scope, phase := 2,
          wireDone := fun j => j == i || (s.walk pk).wireDone j,
          resDone := (s.walk pk).resDone,
          qSent := (s.walk pk).qSent, parentDone := (s.walk pk).parentDone,
          committed := none } := by
      rw [← hs']
      exact setWalk_walk_self _ _ _
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_,
      flow_fire_assemble hwf pk hmem' (wireOut pk) _ hs' hchan0 rfl
        ?_ ?_ ?_ ?_ ?_ ?_ ?_ hi⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        simp only [wkLocalOk, hwalk']
        simp
        refine ⟨hslt, hadv', ?_⟩
        intro x hx
        obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨c1, c2⟩, c3⟩, c4⟩, c5⟩, c6⟩, c7⟩, c8⟩, c9⟩, c10⟩ :=
          hC x hx
        refine ⟨⟨⟨⟨⟨⟨⟨⟨⟨?_, c2⟩, c3⟩, c4⟩, c5⟩, ?_⟩, c7⟩, ?_⟩, c9⟩, ?_⟩
        · -- the new frontier is prefix-closed at `i + 1`
          rcases Nat.lt_trichotomy x i with hxi | hxi | hxi
          · right
            refine ⟨by omega, ?_⟩
            by_cases hx0 : x = 0
            · exact Or.inl hx0
            · right; right
              have h2 := hfront (x - 1) (by omega)
              rw [h2]
              simp
              omega
          · right
            refine ⟨by omega, ?_⟩
            by_cases hx0 : x = 0
            · exact Or.inl hx0
            · by_cases hx1 : x - 1 = i
              · exact Or.inr (Or.inl hx1)
              · right; right
                have h2 := hfront (x - 1) (by omega)
                rw [h2]
                simp
                omega
          · left
            refine ⟨by omega, ?_⟩
            have h2 := hfront x hx
            rw [h2]
            simp
            omega
        · rcases c6 with h | h
          · exact Or.inl h
          · exact Or.inr (Or.inr h)
        · rcases c8 with h | h
          · exact Or.inl h
          · exact Or.inr (Or.inr h)
        · -- d4 shadow: the committed clause covers the newly wired `i`
          rcases c10 with (h | h) | h
          · exact Or.inl (Or.inl h)
          · by_cases hxi : x = i
            · subst hxi
              rcases hd4 with hd | hall
              · exact Or.inl (Or.inl hd)
              · exact Or.inr hall
            · exact Or.inl (Or.inr ⟨hxi, h⟩)
          · exact Or.inr h
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']
          exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hi.wk pk' hpk'
    · rw [← hs']
      exact hi.asm pk' hpk'
    · rw [← hs']
      exact hi.top
    · -- touched channel: one more wire in the live ledger
      rw [sentOf_wireOut hmem', sentOf_wireOut hmem']
      simp only [wkWireSent, wkWireCount, setWalk_walk_self]
      rw [length_filter_insert (p := fun j => (s.walk pk).wireDone j)
        hifan hwdi]
      omega
    · exact fun hab => absurd rfl hab
    · intro _
      simp [wkResSent, wkResCount]
    · intro _
      simp [wkQSentTot, wkQSum]
    · intro _
      simp [wkParentSent, hph2]
    · simp [wkWireRecvd, hph2]
    · simp [wkAskedRecvd, hph2]

/-- `walkFire` on a committed `.res i`: the D-child `i` resolves.
Staying re-establishes the D-prefix and D3 blocks from the arm facts;
advancing counts the fired ledger as exactly the scope's D children and
telescopes into `dsBefore`. -/
private theorem preserve_walkFire_res (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (i : Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some (Oblig.res i))
    (hchan0 : s.chan (lowerOut pk) = 0)
    (hs' : setWalk { s with chan := bump s.chan (lowerOut pk) 1 } pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) (Oblig.res i))) = s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  have hwk := hi.wk pk hmem'
  simp only [wkLocalOk] at hwk
  rw [hph2, hcm] at hwk
  simp at hwk
  obtain ⟨hslt, ⟨-, hC⟩, ⟨⟨⟨⟨hin, hDi⟩, hnrd⟩, hpre⟩, hwi⟩, hd3⟩ := hwk
  have hn : sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
      ≤ sk.fan := nChildren_le_fan hwf hslt
  have hifan : i < sk.fan := by omega
  have hfw : fireOblig (s.walk pk) (Oblig.res i) =
      { scope := (s.walk pk).scope, phase := 2,
        wireDone := (s.walk pk).wireDone,
        resDone := fun j => j == i || (s.walk pk).resDone j,
        qSent := (s.walk pk).qSent, parentDone := (s.walk pk).parentDone,
        committed := none } := by
    simp [fireOblig, hph2]
  rw [hfw] at hs'
  by_cases hadv : scopeComplete sk pk.2
      { scope := (s.walk pk).scope, phase := 2,
        wireDone := (s.walk pk).wireDone,
        resDone := fun j => j == i || (s.walk pk).resDone j,
        qSent := (s.walk pk).qSent, parentDone := (s.walk pk).parentDone,
        committed := none } = true
  · -- the scope completes: `i` was the last missing resolution
    rw [show normWalk sk pk.2
        { scope := (s.walk pk).scope, phase := 2,
          wireDone := (s.walk pk).wireDone,
          resDone := fun j => j == i || (s.walk pk).resDone j,
          qSent := (s.walk pk).qSent, parentDone := (s.walk pk).parentDone,
          committed := none }
        = freshWalk sk pk.2 ((s.walk pk).scope + 1) from by
      simp [normWalk, hadv]] at hs'
    have hwalk' : s'.walk pk = freshWalk sk pk.2 ((s.walk pk).scope + 1) := by
      rw [← hs']
      exact setWalk_walk_self _ _ _
    have hnge : ¬((s.walk pk).scope ≥ sk.stageLen pk.2) := by omega
    simp only [scopeComplete] at hadv
    rw [if_neg hnge] at hadv
    simp at hadv
    obtain ⟨hpdT, hcompl⟩ := hadv
    have hwc : ((List.range sk.fan).filter
        (fun j => (s.walk pk).wireDone j)).length
        = sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope) := by
      refine wireCount_of_complete hn ?_ (fun j hj => (hcompl j hj).1)
      intro j hj hwd
      rcases (hC j hj).1.1.1.1.1.1.1.1.1 with hf | ⟨hlt, -⟩
      · rw [hwd] at hf; cases hf
      · exact hlt
    -- the fired ledger holds exactly the D children
    have hrc' : ((List.range sk.fan).filter
        (fun j => j == i || (s.walk pk).resDone j)).length
        = sk.dOf pk.2 (sk.stageScope pk.2 (s.walk pk).scope) := by
      refine count_eq_dOf sk hn ?_ ?_
      · intro j hj hpj
        simp at hpj
        rcases hpj with he | hr
        · subst he
          exact ⟨by omega, hDi⟩
        · rcases (hC j hj).1.1.1.1.1.1.1.1.2 with hf | h
          · rw [hr] at hf; cases hf
          · exact h
      · intro j hj hd
        rcases (hcompl j hj).2 with h | h
        · rw [hd] at h; cases h
        · rcases h.1 with he | hr
          · simp [he]
          · simp [hr]
    have hqc : (List.range sk.fan).foldl
        (fun acc j => acc + (s.walk pk).qSent j) 0
        = sk.qOf pk.2 (sk.stageScope pk.2 (s.walk pk).scope) := by
      refine qSum_of_complete sk hn
        (fun j hj => (hC j hj).1.1.1.1.1.1.1.2) ?_
      intro j hj hd
      rcases (hcompl j hj).2 with h | h
      · rw [hd] at h; cases h
      · exact h.2
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_,
      flow_fire_assemble hwf pk hmem' (lowerOut pk) _ hs' hchan0 rfl
        ?_ ?_ ?_ ?_ ?_ ?_ ?_ hi⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        exact wkLocalOk_fresh pk' _ (by omega) hwalk'
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']
          exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hi.wk pk' hpk'
    · rw [← hs']
      exact hi.asm pk' hpk'
    · rw [← hs']
      exact hi.top
    · -- touched channel: the res count telescopes into `dsBefore`
      rw [sentOf_lowerOut, sentOf_lowerOut]
      simp only [wkResSent, wkResCount, setWalk_walk_self, freshWalk]
      rw [dsBefore_succ sk hslt, ← hrc',
        length_filter_insert (p := fun j => (s.walk pk).resDone j)
          hifan hnrd]
      have hff : ((List.range sk.fan).filter (fun _ : Nat => false)).length
          = 0 := by simp
      omega
    · intro _
      simp only [wkWireSent, wkWireCount, setWalk_walk_self, freshWalk]
      rw [wiresBefore_succ sk hslt, hwc]
      simp
    · exact fun hab => absurd rfl hab
    · intro _
      simp only [wkQSentTot, wkQSum, setWalk_walk_self, freshWalk]
      rw [qsBefore_succ sk hslt, hqc]
      simp [foldl_const]
    · intro _
      by_cases hl2 : (s.walk pk).scope + 1 < sk.stageLen pk.2 <;>
        simp [wkParentSent, freshWalk, hph2, hpdT, hl2]
    · rw [wkWireRecvd_fresh pk (by omega) (setWalk_walk_self _ _ _)]
      simp [wkWireRecvd, hph2]
    · rw [wkAskedRecvd_fresh pk (by omega) (setWalk_walk_self _ _ _)]
      simp [wkAskedRecvd, hph2]
  · -- the scope is still incomplete: re-establish the per-child block
    have hadv' : scopeComplete sk pk.2
        { scope := (s.walk pk).scope, phase := 2,
          wireDone := (s.walk pk).wireDone,
          resDone := fun j => j == i || (s.walk pk).resDone j,
          qSent := (s.walk pk).qSent, parentDone := (s.walk pk).parentDone,
          committed := none } = false := by
      simpa using hadv
    rw [show normWalk sk pk.2
        { scope := (s.walk pk).scope, phase := 2,
          wireDone := (s.walk pk).wireDone,
          resDone := fun j => j == i || (s.walk pk).resDone j,
          qSent := (s.walk pk).qSent, parentDone := (s.walk pk).parentDone,
          committed := none }
        = { scope := (s.walk pk).scope, phase := 2,
            wireDone := (s.walk pk).wireDone,
            resDone := fun j => j == i || (s.walk pk).resDone j,
            qSent := (s.walk pk).qSent, parentDone := (s.walk pk).parentDone,
            committed := none } from by
      simp [normWalk, hadv']] at hs'
    have hwalk' : s'.walk pk =
        { scope := (s.walk pk).scope, phase := 2,
          wireDone := (s.walk pk).wireDone,
          resDone := fun j => j == i || (s.walk pk).resDone j,
          qSent := (s.walk pk).qSent, parentDone := (s.walk pk).parentDone,
          committed := none } := by
      rw [← hs']
      exact setWalk_walk_self _ _ _
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_,
      flow_fire_assemble hwf pk hmem' (lowerOut pk) _ hs' hchan0 rfl
        ?_ ?_ ?_ ?_ ?_ ?_ ?_ hi⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        simp only [wkLocalOk, hwalk']
        simp
        refine ⟨hslt, hadv', ?_⟩
        intro x hx
        obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨c1, c2⟩, c3⟩, c4⟩, c5⟩, c6⟩, c7⟩, c8⟩, c9⟩, c10⟩ :=
          hC x hx
        refine ⟨⟨⟨⟨⟨⟨⟨⟨⟨c1, ?_⟩, c3⟩, c4⟩, ?_⟩, ?_⟩, ?_⟩, c8⟩, ?_⟩, ?_⟩
        · -- res prefix: the new child is real and D
          by_cases hxi : x = i
          · exact Or.inr ⟨by omega, by rw [hxi]; exact hDi⟩
          · rcases c2 with hf | h
            · exact Or.inl ⟨hxi, hf⟩
            · exact Or.inr h
        · -- D-prefix closure: the arm's prefix covers `i`
          by_cases hxi : x = i
          · right
            intro x1 hx1
            rcases hpre x1 (by omega) with h | h
            · exact Or.inl h
            · exact Or.inr (Or.inr h)
          · rcases c5 with hf | h
            · exact Or.inl ⟨hxi, hf⟩
            · right
              intro x1 hx1
              rcases h x1 hx1 with h2 | h2
              · exact Or.inl h2
              · exact Or.inr (Or.inr h2)
        · -- W-axiom shadow: the arm carries `wireDone i`
          by_cases hxi : x = i
          · rcases hwi with h | h
            · exact Or.inl (Or.inl h)
            · exact Or.inr (by rw [hxi]; exact h)
          · rcases c6 with (h | h) | h
            · exact Or.inl (Or.inl h)
            · exact Or.inl (Or.inr ⟨hxi, h⟩)
            · exact Or.inr h
        · -- D1 gets weaker: resolutions only grow
          rcases c7 with h | h
          · exact Or.inl h
          · exact Or.inr (Or.inr h)
        · -- D3: the arm says every old resolution is fed
          rcases hd3 with h | hall
          · exact Or.inl (Or.inl (Or.inl h))
          · by_cases hxi : x = i
            · right
              intro x1 hx1
              by_cases hx1i : x1 = i
              · exact Or.inl (Or.inl (hx1i.trans hxi.symm))
              · rcases hall x1 hx1 with h | h
                · exact Or.inl (Or.inr ⟨hx1i, h⟩)
                · exact Or.inr h
            · cases hrdx : (s.walk pk').resDone x with
              | false => exact Or.inl (Or.inl (Or.inr ⟨hxi, rfl⟩))
              | true =>
                  rcases c2 with hf | ⟨hxn, -⟩
                  · rw [hrdx] at hf; cases hf
                  · rcases hall x hxn with h | h
                    · rw [hrdx] at h; cases h
                    · exact Or.inl (Or.inr h)
        · -- d4 shadow: resolutions only grow under a res fire
          rcases c10 with h | hall
          · exact Or.inl h
          · right
            intro x1 hx1
            rcases hall x1 hx1 with h | ⟨hrd, hq⟩
            · exact Or.inl h
            · exact Or.inr ⟨Or.inr hrd, hq⟩
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']
          exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hi.wk pk' hpk'
    · rw [← hs']
      exact hi.asm pk' hpk'
    · rw [← hs']
      exact hi.top
    · -- touched channel: one more resolution in the live ledger
      rw [sentOf_lowerOut, sentOf_lowerOut]
      simp only [wkResSent, wkResCount, setWalk_walk_self]
      rw [length_filter_insert (p := fun j => (s.walk pk).resDone j)
        hifan hnrd]
      omega
    · intro _
      simp [wkWireSent, wkWireCount]
    · exact fun hab => absurd rfl hab
    · intro _
      simp [wkQSentTot, wkQSum]
    · intro _
      simp [wkParentSent, hph2]
    · simp [wkWireRecvd, hph2]
    · simp [wkAskedRecvd, hph2]

/-- `walkFire` on a committed `.query i`: one more query for the D-child
`i`. The arm's strict budget bound keeps the in-order block consistent
(a later child with sends would force `i`'s budget closed); advancing
saturates every budget and telescopes into `qsBefore`. The stage is
above the leaves (`childIsD` forces it), so the fired channel is
`askedOut` at `1 ≤ pk.2`. -/
private theorem preserve_walkFire_query (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (i : Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some (Oblig.query i))
    (hchan0 : s.chan (askedOut pk) = 0)
    (hs' : setWalk { s with chan := bump s.chan (askedOut pk) 1 } pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) (Oblig.query i))) = s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  have hwk := hi.wk pk hmem'
  simp only [wkLocalOk] at hwk
  rw [hph2, hcm] at hwk
  simp at hwk
  obtain ⟨hslt, ⟨-, hC⟩, ⟨⟨⟨⟨hin, hDi⟩, hqlt⟩, hqpre⟩, hd1⟩, hwf1⟩ := hwk
  have hn : sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
      ≤ sk.fan := nChildren_le_fan hwf hslt
  have hifan : i < sk.fan := by omega
  have hp1 : 1 ≤ pk.2 := by
    have := ne_zero_of_childIsD (sk := sk) hDi
    omega
  have hcap1 : sk.cap (askedOut pk) = 1 := by
    by_cases h2 : pk.2 < 2 <;> simp [askedOut, h2, Skel.cap]
  have hfw : fireOblig (s.walk pk) (Oblig.query i) =
      { scope := (s.walk pk).scope, phase := 2,
        wireDone := (s.walk pk).wireDone, resDone := (s.walk pk).resDone,
        qSent := fun j => if j = i then (s.walk pk).qSent j + 1
          else (s.walk pk).qSent j,
        parentDone := (s.walk pk).parentDone,
        committed := none } := by
    simp [fireOblig, hph2]
  rw [hfw] at hs'
  by_cases hadv : scopeComplete sk pk.2
      { scope := (s.walk pk).scope, phase := 2,
        wireDone := (s.walk pk).wireDone, resDone := (s.walk pk).resDone,
        qSent := fun j => if j = i then (s.walk pk).qSent j + 1
          else (s.walk pk).qSent j,
        parentDone := (s.walk pk).parentDone,
        committed := none } = true
  · -- the scope completes: this query saturated the last budget
    rw [show normWalk sk pk.2
        { scope := (s.walk pk).scope, phase := 2,
          wireDone := (s.walk pk).wireDone, resDone := (s.walk pk).resDone,
          qSent := fun j => if j = i then (s.walk pk).qSent j + 1
            else (s.walk pk).qSent j,
          parentDone := (s.walk pk).parentDone,
          committed := none }
        = freshWalk sk pk.2 ((s.walk pk).scope + 1) from by
      simp [normWalk, hadv]] at hs'
    have hwalk' : s'.walk pk = freshWalk sk pk.2 ((s.walk pk).scope + 1) := by
      rw [← hs']
      exact setWalk_walk_self _ _ _
    have hnge : ¬((s.walk pk).scope ≥ sk.stageLen pk.2) := by omega
    simp only [scopeComplete] at hadv
    rw [if_neg hnge] at hadv
    simp at hadv
    obtain ⟨hpdT, hcompl⟩ := hadv
    have hwc : ((List.range sk.fan).filter
        (fun j => (s.walk pk).wireDone j)).length
        = sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope) := by
      refine wireCount_of_complete hn ?_ (fun j hj => (hcompl j hj).1)
      intro j hj hwd
      rcases (hC j hj).1.1.1.1.1.1.1.1.1 with hf | ⟨hlt, -⟩
      · rw [hwd] at hf; cases hf
      · exact hlt
    have hrc : ((List.range sk.fan).filter
        (fun j => (s.walk pk).resDone j)).length
        = sk.dOf pk.2 (sk.stageScope pk.2 (s.walk pk).scope) := by
      refine count_eq_dOf sk hn ?_ ?_
      · intro j hj hrd
        rcases (hC j hj).1.1.1.1.1.1.1.1.2 with hf | h
        · rw [hrd] at hf; cases hf
        · exact h
      · intro j hj hd
        rcases (hcompl j hj).2 with h | h
        · rw [hd] at h; cases h
        · exact h.1
    -- the fired budgets are saturated everywhere
    have hqof : (List.range sk.fan).foldl
        (fun acc j => acc + (if j = i then (s.walk pk).qSent j + 1
          else (s.walk pk).qSent j)) 0
        = sk.qOf pk.2 (sk.stageScope pk.2 (s.walk pk).scope) := by
      refine qSum_of_complete sk hn ?_ ?_
      · intro j hj
        by_cases hji : j = i
        · simp only [hji, if_pos]
          omega
        · rw [if_neg hji]
          exact (hC j hj).1.1.1.1.1.1.1.2
      · intro j hj hd
        rcases (hcompl j hj).2 with h | h
        · rw [hd] at h; cases h
        · exact h.2
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_,
      flow_fire_assemble hwf pk hmem' (askedOut pk) _ hs' hchan0 hcap1
        ?_ ?_ ?_ ?_ ?_ ?_ ?_ hi⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        exact wkLocalOk_fresh pk' _ (by omega) hwalk'
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']
          exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hi.wk pk' hpk'
    · rw [← hs']
      exact hi.asm pk' hpk'
    · rw [← hs']
      exact hi.top
    · -- touched channel: the query total telescopes into `qsBefore`
      rw [sentOf_askedOut hwf hmem' hp1, sentOf_askedOut hwf hmem' hp1]
      simp only [wkQSentTot, wkQSum, setWalk_walk_self, freshWalk]
      rw [qsBefore_succ sk hslt, ← hqof, foldl_add_update' hifan]
      simp [foldl_const]
      omega
    · intro _
      simp only [wkWireSent, wkWireCount, setWalk_walk_self, freshWalk]
      rw [wiresBefore_succ sk hslt, hwc]
      simp
    · intro _
      simp only [wkResSent, wkResCount, setWalk_walk_self, freshWalk]
      rw [dsBefore_succ sk hslt, hrc]
      simp
    · exact fun hab => absurd rfl hab
    · intro _
      by_cases hl2 : (s.walk pk).scope + 1 < sk.stageLen pk.2 <;>
        simp [wkParentSent, freshWalk, hph2, hpdT, hl2]
    · rw [wkWireRecvd_fresh pk (by omega) (setWalk_walk_self _ _ _)]
      simp [wkWireRecvd, hph2]
    · rw [wkAskedRecvd_fresh pk (by omega) (setWalk_walk_self _ _ _)]
      simp [wkAskedRecvd, hph2]
  · -- the scope is still incomplete: re-establish the in-order block
    have hadv' : scopeComplete sk pk.2
        { scope := (s.walk pk).scope, phase := 2,
          wireDone := (s.walk pk).wireDone, resDone := (s.walk pk).resDone,
          qSent := fun j => if j = i then (s.walk pk).qSent j + 1
            else (s.walk pk).qSent j,
          parentDone := (s.walk pk).parentDone,
          committed := none } = false := by
      simpa using hadv
    rw [show normWalk sk pk.2
        { scope := (s.walk pk).scope, phase := 2,
          wireDone := (s.walk pk).wireDone, resDone := (s.walk pk).resDone,
          qSent := fun j => if j = i then (s.walk pk).qSent j + 1
            else (s.walk pk).qSent j,
          parentDone := (s.walk pk).parentDone,
          committed := none }
        = { scope := (s.walk pk).scope, phase := 2,
            wireDone := (s.walk pk).wireDone, resDone := (s.walk pk).resDone,
            qSent := fun j => if j = i then (s.walk pk).qSent j + 1
              else (s.walk pk).qSent j,
            parentDone := (s.walk pk).parentDone,
            committed := none } from by
      simp [normWalk, hadv']] at hs'
    have hwalk' : s'.walk pk =
        { scope := (s.walk pk).scope, phase := 2,
          wireDone := (s.walk pk).wireDone, resDone := (s.walk pk).resDone,
          qSent := fun j => if j = i then (s.walk pk).qSent j + 1
            else (s.walk pk).qSent j,
          parentDone := (s.walk pk).parentDone,
          committed := none } := by
      rw [← hs']
      exact setWalk_walk_self _ _ _
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_,
      flow_fire_assemble hwf pk hmem' (askedOut pk) _ hs' hchan0 hcap1
        ?_ ?_ ?_ ?_ ?_ ?_ ?_ hi⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        simp only [wkLocalOk, hwalk']
        simp
        refine ⟨hslt, hadv', ?_⟩
        intro x hx
        obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨c1, c2⟩, c3⟩, c4⟩, c5⟩, c6⟩, c7⟩, c8⟩, c9⟩, c10⟩ :=
          hC x hx
        refine ⟨⟨⟨⟨⟨⟨⟨⟨⟨c1, c2⟩, ?_⟩, ?_⟩, c5⟩, c6⟩, ?_⟩, ?_⟩, ?_⟩, ?_⟩
        · -- budget bound: strict at `i`, unchanged elsewhere
          by_cases hxi : x = i
          · rw [if_pos hxi, hxi]
            omega
          · rw [if_neg hxi]
            exact c3
        · -- in-order block: sends below `i` are saturated by the arm
          by_cases hxi : x = i
          · right
            intro x1 hx1
            rw [if_neg (by omega : ¬(x1 = i))]
            exact hqpre x1 (by omega)
          · rw [if_neg hxi]
            rcases c4 with h0 | hall4
            · exact Or.inl h0
            · by_cases hix : i < x
              · exfalso
                have := hall4 i hix
                omega
              · right
                intro x1 hx1
                rw [if_neg (by omega : ¬(x1 = i))]
                exact hall4 x1 hx1
        · -- D1 shadow: the arm carries `resDone i`
          by_cases hxi : x = i
          · rcases hd1 with h | h
            · exact Or.inl (Or.inl h)
            · exact Or.inr (by rw [hxi]; exact h)
          · rcases c7 with (h | h) | h
            · exact Or.inl (Or.inl h)
            · exact Or.inl (Or.inr (by rw [if_neg hxi]; exact h))
            · exact Or.inr h
        · -- wireFirst shadow: the arm carries `wireDone i`
          by_cases hxi : x = i
          · rcases hwf1 with h | h
            · exact Or.inl (Or.inl h)
            · exact Or.inr (by rw [hxi]; exact h)
          · rcases c8 with (h | h) | h
            · exact Or.inl (Or.inl h)
            · exact Or.inl (Or.inr (by rw [if_neg hxi]; exact h))
            · exact Or.inr h
        · -- D3: a closed budget cannot be `i`'s (the arm is strict)
          rcases c9 with ((h | h) | h) | hall9
          · exact Or.inl (Or.inl (Or.inl h))
          · exact Or.inl (Or.inl (Or.inr h))
          · by_cases hxi : x = i
            · exfalso
              rw [hxi] at h
              omega
            · exact Or.inl (Or.inr (by rw [if_neg hxi]; exact h))
          · right
            intro x1 hx1
            rcases hall9 x1 hx1 with (h | h) | h
            · exact Or.inl (Or.inl h)
            · exact Or.inl (Or.inr h)
            · by_cases hx1i : x1 = i
              · exfalso
                rw [hx1i] at h
                omega
              · exact Or.inr (by rw [if_neg hx1i]; exact h)
        · -- d4 shadow: a shadowed budget cannot be `i`'s (the arm is strict)
          rcases c10 with h | hall
          · exact Or.inl h
          · right
            intro x1 hx1
            rcases hall x1 hx1 with h | ⟨hrd, hq⟩
            · exact Or.inl h
            · refine Or.inr ⟨hrd, ?_⟩
              by_cases hx1i : x1 = i
              · exfalso
                rw [hx1i] at hq
                omega
              · rw [if_neg hx1i]
                exact hq
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']
          exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hi.wk pk' hpk'
    · rw [← hs']
      exact hi.asm pk' hpk'
    · rw [← hs']
      exact hi.top
    · -- touched channel: one more query in the live ledger
      rw [sentOf_askedOut hwf hmem' hp1, sentOf_askedOut hwf hmem' hp1]
      simp only [wkQSentTot, wkQSum, setWalk_walk_self]
      rw [foldl_add_update' hifan]
      omega
    · intro _
      simp [wkWireSent, wkWireCount]
    · intro _
      simp [wkResSent, wkResCount]
    · exact fun hab => absurd rfl hab
    · intro _
      simp [wkParentSent, hph2]
    · simp [wkWireRecvd, hph2]
    · simp [wkAskedRecvd, hph2]

-- ======================================================== the theorem

/-- `walkFire` publishes a committed obligation: occupancy of the fired
channel rises 0 → 1 exactly as its producer count rises by one, and the
embedded `normWalk` either keeps the scope (per-child block re-derived
from the committed-arm facts) or advances the cursor (live counts
telescope into the prefix sums). -/
theorem preserve_walkFire (hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : apply sk ax (.walkFire pk) s = some s')
    (hi : InvP sk ax s) : InvP sk ax s' := by
  simp only [apply] at hstep
  split at hstep
  next o hcm =>
    split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
      obtain ⟨⟨hmem, hph2⟩, hlt1⟩ := hg
      have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
      have hchan0 : s.chan (obligChan pk o) = 0 := by omega
      injection hstep with hs'
      cases o with
      | wire i =>
          exact preserve_walkFire_wire hwf pk i hmem' hph2 hcm hchan0 hs' hi
      | res i =>
          exact preserve_walkFire_res hwf pk i hmem' hph2 hcm hchan0 hs' hi
      | query i =>
          exact preserve_walkFire_query hwf pk i hmem' hph2 hcm hchan0 hs' hi
      | parent =>
          exact preserve_walkFire_parent hwf pk hmem' hph2 hcm hchan0 hs' hi
  next hcm => simp at hstep

end StreamingMirror.Model
