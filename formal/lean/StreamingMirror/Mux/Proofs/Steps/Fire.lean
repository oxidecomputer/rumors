/-
Per-obligation step facts for `walkFire` — the fire's cursor effect
(`setWalk s pk (normWalk … (fireOblig … o))`) with NO channel bump:
exactly the shape both muxed fire sites share. The base `walkFire` arm
adds a `bump` on the fired channel and `firePush`'s walk case adds a
pipe append instead; `InvL` and the counts are chan-blind, so one
lemma serves both (Steps.lean's module doc).

The local bullets and the completion-counting scripts are transplanted
from Proofs/Preserve/WalkFire.lean (whose `InvP` form cannot be invoked
at muxed states); its private completion-counting lemmas are restated
here verbatim — the flagged stage-3 extraction cost, paid once.
-/
import StreamingMirror.Mux.Proofs.Steps
import StreamingMirror.Proofs.Preserve.WalkFire

namespace StreamingMirror.Mux

open Model

variable {sk : Skel} {ax : AxMode} {s s' : State}

-- ==================== restated completion counting (WalkFire privates)

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

/-- A freshly cut walk cursor's prologue-wire count is its position. -/
private theorem wkWireRecvd_fresh {t : State} (pk : Party × Nat) {k : Nat}
    (hk : k ≤ sk.stageLen pk.2) (hw : t.walk pk = freshWalk sk pk.2 k) :
    wkWireRecvd sk t pk = k := by
  by_cases hl : k < sk.stageLen pk.2
  · simp [wkWireRecvd, hw, freshWalk, hl]
  · have hz : k = sk.stageLen pk.2 := by omega
    simp [wkWireRecvd, hw, freshWalk, hl]
    omega

/-- A freshly cut walk cursor's prologue-query count is its position. -/
private theorem wkAskedRecvd_fresh {t : State} (pk : Party × Nat) {k : Nat}
    (hk : k ≤ sk.stageLen pk.2) (hw : t.walk pk = freshWalk sk pk.2 k) :
    wkAskedRecvd sk t pk = k := by
  by_cases hl : k < sk.stageLen pk.2
  · simp [wkAskedRecvd, hw, freshWalk, hl]
  · have hz : k = sk.stageLen pk.2 := by omega
    simp [wkAskedRecvd, hw, freshWalk, hl]
    omega

-- ============================== chan-free fire frames (WalkFire privates)

private theorem other_ne_self (p : Party) : p.other ≠ p := by
  cases p <;> simp [Party.other]

/-- A key of the other party never collides with `pk`. -/
private theorem key_other_ne (p : Party) (a b : Nat) :
    ((p.other, a) : Party × Nat) ≠ (p, b) := by
  intro hcon
  have hfst := congrArg Prod.fst hcon
  simp at hfst
  exact other_ne_self p hfst

/-- Sends frame for a chan-free walk update: each of `pk`'s producer
counts is unchanged unless it feeds the touched channel `c₀`, so every
channel other than `c₀` reads the same producer count. -/
private theorem sentOf_fire_frame' (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (ws' : WalkSt) (hmem : pk ∈ sk.walkKeys) (c₀ : Chan)
    (hW : c₀ ≠ wireOut pk →
      wkWireSent sk (setWalk s pk ws') pk = wkWireSent sk s pk)
    (hR : c₀ ≠ lowerOut pk →
      wkResSent sk (setWalk s pk ws') pk = wkResSent sk s pk)
    (hQ : c₀ ≠ askedOut pk →
      wkQSentTot sk (setWalk s pk ws') pk = wkQSentTot sk s pk)
    (hP : c₀ ≠ upperOut pk →
      wkParentSent (setWalk s pk ws') pk = wkParentSent s pk)
    {c : Chan} (hc : c ∈ allChans sk) (hne : c ≠ c₀) :
    sentOf sk (setWalk s pk ws') c = sentOf sk s c := by
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
  · exact sentOf_setWalk_frame s pk ws' hc h1 h2 h3 h4

/-- Receives frame for a chan-free walk update whose two consumer
counts at `pk` are unchanged: every channel reads the same consumer
count. -/
private theorem recvdOf_fire_frame' (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (ws' : WalkSt) (hmem : pk ∈ sk.walkKeys)
    (hWr : wkWireRecvd sk (setWalk s pk ws') pk = wkWireRecvd sk s pk)
    (hAr : wkAskedRecvd sk (setWalk s pk ws') pk = wkAskedRecvd sk s pk)
    {c : Chan} (hc : c ∈ allChans sk) :
    recvdOf sk (setWalk s pk ws') c = recvdOf sk s c := by
  by_cases h5 : c = wireIn pk
  · subst h5
    rw [recvdOf_wireIn hmem, recvdOf_wireIn hmem]
    exact hWr
  by_cases h6 : c = askedIn pk
  · subst h6
    rw [recvdOf_askedIn, recvdOf_askedIn]
    exact hAr
  · exact recvdOf_setWalk_frame hwf s pk ws' hc h5 h6

/-- Assembles the count deltas for a chan-free fire: the touched
channel's producer count rises by one, everything else frames. -/
private theorem counts_fire_assemble (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (hmem' : pk ∈ sk.walkKeys) (c₀ : Chan) (W : WalkSt)
    (hs' : setWalk s pk W = s')
    (htouch : sentOf sk (setWalk s pk W) c₀ = sentOf sk s c₀ + 1)
    (hW : c₀ ≠ wireOut pk →
      wkWireSent sk (setWalk s pk W) pk = wkWireSent sk s pk)
    (hR : c₀ ≠ lowerOut pk →
      wkResSent sk (setWalk s pk W) pk = wkResSent sk s pk)
    (hQ : c₀ ≠ askedOut pk →
      wkQSentTot sk (setWalk s pk W) pk = wkQSentTot sk s pk)
    (hP : c₀ ≠ upperOut pk →
      wkParentSent (setWalk s pk W) pk = wkParentSent s pk)
    (hWr : wkWireRecvd sk (setWalk s pk W) pk = wkWireRecvd sk s pk)
    (hAr : wkAskedRecvd sk (setWalk s pk W) pk = wkAskedRecvd sk s pk) :
    (∀ c ∈ allChans sk,
      sentOf sk s' c = sentOf sk s c + (if c = c₀ then 1 else 0))
    ∧ (∀ c ∈ allChans sk, recvdOf sk s' c = recvdOf sk s c) := by
  constructor
  · intro c hc
    by_cases hne : c = c₀
    · subst hne
      rw [if_pos rfl, ← hs']
      exact htouch
    · rw [if_neg hne, ← hs']
      exact sentOf_fire_frame' hwf pk W hmem' c₀ hW hR hQ hP hc hne
  · intro c hc
    rw [← hs']
    exact recvdOf_fire_frame' hwf pk W hmem' hWr hAr hc

-- ================================================== hands after a fire

/-- Firing clears the committed slot, whatever the obligation. -/
private theorem fireOblig_committed (ws : WalkSt) (o : Oblig) :
    (fireOblig ws o).committed = none := by
  cases o <;> rfl

/-- `normWalk` never mints a commitment. -/
private theorem normWalk_committed {h : Nat} {ws : WalkSt}
    (hc : ws.committed = none) :
    (normWalk sk h ws).committed = none := by
  rw [normWalk]
  split
  · rfl
  · exact hc

/-- The hands clause of any fire: the fired walk's wire hand is off
afterwards (the slot is cleared) and every other stream's hand reads
untouched state. -/
private theorem fire_hands (pk : Party × Nat) (o : Oblig)
    (hs' : setWalk s pk (normWalk sk pk.2 (fireOblig (s.walk pk) o)) = s') :
    (∀ p h, (p, h) ≠ pk → holdsWire sk p h s' = holdsWire sk p h s)
    ∧ wireHand (s'.walk pk) = false := by
  constructor
  · intro p h hne
    by_cases hr : h = sk.rootH
    · subst hr
      rw [holdsWire.eq_def, holdsWire.eq_def]
      simp only [beq_self_eq_true, if_pos]
      have hio : s'.iopenCh = s.iopenCh := by rw [← hs']; rfl
      have hro : s'.ropenCh = s.ropenCh := by rw [← hs']; rfl
      cases p
      · rw [hio]
      · rw [hro]
    · rw [holdsWire_eq_wireHand hr, holdsWire_eq_wireHand hr]
      have hw : s'.walk (p, h) = s.walk (p, h) := by
        rw [← hs']
        exact setWalk_walk_ne _ _ hne
      rw [hw]
  · have hw : s'.walk pk
        = normWalk sk pk.2 (fireOblig (s.walk pk) o) := by
      rw [← hs']
      exact setWalk_walk_self _ _ _
    rw [wireHand, hw, normWalk_committed (fireOblig_committed ..)]
    simp

-- ================================================ per-obligation lemmas

/-- The fire's cursor effect on a committed `.parent`, chan-free: the
`InvL` bullet and count deltas of `preserve_walkFire_parent`'s local
part. -/
private theorem step_fire_parent (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some Oblig.parent)
    (hs' : setWalk s pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) Oblig.parent)) = s')
    (hL : InvL sk ax s) :
    InvL sk ax s'
    ∧ (∀ c ∈ allChans sk,
        sentOf sk s' c = sentOf sk s c
          + (if c = upperOut pk then 1 else 0))
    ∧ (∀ c ∈ allChans sk, recvdOf sk s' c = recvdOf sk s c) := by
  have hwk := hL.wk pk hmem'
  simp only [wkLocalOk] at hwk
  rw [hph2, hcm] at hwk
  simp at hwk
  obtain ⟨hslt, ⟨hnsc, hC⟩, hpd, hd2⟩ := hwk
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
    have hnge : ¬((s.walk pk).scope ≥ sk.stageLen pk.2) := by omega
    simp only [scopeComplete] at hadv
    rw [if_neg hnge] at hadv
    simp at hadv
    have hn : sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
        ≤ sk.fan := nChildren_le_fan hwf hslt
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
    obtain ⟨hsent, hrecv⟩ := counts_fire_assemble hwf pk hmem'
      (upperOut pk) _ hs'
      (by
        rw [sentOf_upperOut, sentOf_upperOut]
        by_cases hl2 : (s.walk pk).scope + 1 < sk.stageLen pk.2 <;>
          simp [wkParentSent, freshWalk, hph2, hpd, hl2])
      (by
        intro _
        simp only [wkWireSent, wkWireCount, setWalk_walk_self, freshWalk]
        rw [wiresBefore_succ sk hslt, hwc]
        simp)
      (by
        intro _
        simp only [wkResSent, wkResCount, setWalk_walk_self, freshWalk]
        rw [dsBefore_succ sk hslt, hrc]
        simp)
      (by
        intro _
        simp only [wkQSentTot, wkQSum, setWalk_walk_self, freshWalk]
        rw [qsBefore_succ sk hslt, hqc]
        simp [foldl_const])
      (fun hab => absurd rfl hab)
      (by
        rw [wkWireRecvd_fresh pk (by omega) (setWalk_walk_self _ _ _)]
        simp [wkWireRecvd, hph2])
      (by
        rw [wkAskedRecvd_fresh pk (by omega) (setWalk_walk_self _ _ _)]
        simp [wkAskedRecvd, hph2])
    refine ⟨⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩, hsent, hrecv⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        exact wkLocalOk_fresh pk' _ (by omega) hwalk'
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']
          exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hL.wk pk' hpk'
    · rw [← hs']
      exact hL.asm pk' hpk'
    · rw [← hs']
      exact hL.top
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
    obtain ⟨hsent, hrecv⟩ := counts_fire_assemble hwf pk hmem'
      (upperOut pk) _ hs'
      (by
        rw [sentOf_upperOut, sentOf_upperOut]
        simp [wkParentSent, hph2, hpd])
      (by
        intro _
        simp [wkWireSent, wkWireCount])
      (by
        intro _
        simp [wkResSent, wkResCount])
      (by
        intro _
        simp [wkQSentTot, wkQSum])
      (fun hab => absurd rfl hab)
      (by simp [wkWireRecvd, hph2])
      (by simp [wkAskedRecvd, hph2])
    refine ⟨⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩, hsent, hrecv⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        simp only [wkLocalOk, hwalk']
        simp
        exact ⟨hslt, hadv', hC⟩
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']
          exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hL.wk pk' hpk'
    · rw [← hs']
      exact hL.asm pk' hpk'
    · rw [← hs']
      exact hL.top

/-- The fire's cursor effect on a committed `.wire i`, chan-free: the
`InvL` bullet and count deltas of `preserve_walkFire_wire`'s local
part. This is the arm `firePush` runs — the mux's only wire send. -/
private theorem step_fire_wire (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (i : Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some (Oblig.wire i))
    (hs' : setWalk s pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) (Oblig.wire i))) = s')
    (hL : InvL sk ax s) :
    InvL sk ax s'
    ∧ (∀ c ∈ allChans sk,
        sentOf sk s' c = sentOf sk s c
          + (if c = wireOut pk then 1 else 0))
    ∧ (∀ c ∈ allChans sk, recvdOf sk s' c = recvdOf sk s c) := by
  have hwk := hL.wk pk hmem'
  simp only [wkLocalOk] at hwk
  rw [hph2, hcm] at hwk
  simp at hwk
  obtain ⟨hslt, ⟨-, hC⟩, ⟨⟨hieq, hin⟩, hd4⟩, -⟩ := hwk
  have hn : sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
      ≤ sk.fan := nChildren_le_fan hwf hslt
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
    obtain ⟨hsent, hrecv⟩ := counts_fire_assemble hwf pk hmem'
      (wireOut pk) _ hs'
      (by
        rw [sentOf_wireOut hmem', sentOf_wireOut hmem']
        simp only [wkWireSent, wkWireCount, setWalk_walk_self, freshWalk]
        rw [wiresBefore_succ sk hslt, hcnt, hn1]
        have hff : ((List.range sk.fan).filter
            (fun _ : Nat => false)).length = 0 := by simp
        omega)
      (fun hab => absurd rfl hab)
      (by
        intro _
        simp only [wkResSent, wkResCount, setWalk_walk_self, freshWalk]
        rw [dsBefore_succ sk hslt, hrc]
        simp)
      (by
        intro _
        simp only [wkQSentTot, wkQSum, setWalk_walk_self, freshWalk]
        rw [qsBefore_succ sk hslt, hqc]
        simp [foldl_const])
      (by
        intro _
        by_cases hl2 : (s.walk pk).scope + 1 < sk.stageLen pk.2 <;>
          simp [wkParentSent, freshWalk, hph2, hpdT, hl2])
      (by
        rw [wkWireRecvd_fresh pk (by omega) (setWalk_walk_self _ _ _)]
        simp [wkWireRecvd, hph2])
      (by
        rw [wkAskedRecvd_fresh pk (by omega) (setWalk_walk_self _ _ _)]
        simp [wkAskedRecvd, hph2])
    refine ⟨⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩, hsent, hrecv⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        exact wkLocalOk_fresh pk' _ (by omega) hwalk'
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']
          exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hL.wk pk' hpk'
    · rw [← hs']
      exact hL.asm pk' hpk'
    · rw [← hs']
      exact hL.top
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
    obtain ⟨hsent, hrecv⟩ := counts_fire_assemble hwf pk hmem'
      (wireOut pk) _ hs'
      (by
        rw [sentOf_wireOut hmem', sentOf_wireOut hmem']
        simp only [wkWireSent, wkWireCount, setWalk_walk_self]
        rw [length_filter_insert (p := fun j => (s.walk pk).wireDone j)
          hifan hwdi]
        omega)
      (fun hab => absurd rfl hab)
      (by
        intro _
        simp [wkResSent, wkResCount])
      (by
        intro _
        simp [wkQSentTot, wkQSum])
      (by
        intro _
        simp [wkParentSent, hph2])
      (by simp [wkWireRecvd, hph2])
      (by simp [wkAskedRecvd, hph2])
    refine ⟨⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩, hsent, hrecv⟩
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
        exact hL.wk pk' hpk'
    · rw [← hs']
      exact hL.asm pk' hpk'
    · rw [← hs']
      exact hL.top

/-- The fire's cursor effect on a committed `.res i`, chan-free: the
`InvL` bullet and count deltas of `preserve_walkFire_res`'s local
part. -/
private theorem step_fire_res (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (i : Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some (Oblig.res i))
    (hs' : setWalk s pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) (Oblig.res i))) = s')
    (hL : InvL sk ax s) :
    InvL sk ax s'
    ∧ (∀ c ∈ allChans sk,
        sentOf sk s' c = sentOf sk s c
          + (if c = lowerOut pk then 1 else 0))
    ∧ (∀ c ∈ allChans sk, recvdOf sk s' c = recvdOf sk s c) := by
  have hwk := hL.wk pk hmem'
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
    obtain ⟨hsent, hrecv⟩ := counts_fire_assemble hwf pk hmem'
      (lowerOut pk) _ hs'
      (by
        rw [sentOf_lowerOut, sentOf_lowerOut]
        simp only [wkResSent, wkResCount, setWalk_walk_self, freshWalk]
        rw [dsBefore_succ sk hslt, ← hrc',
          length_filter_insert (p := fun j => (s.walk pk).resDone j)
            hifan hnrd]
        have hff : ((List.range sk.fan).filter
            (fun _ : Nat => false)).length = 0 := by simp
        omega)
      (by
        intro _
        simp only [wkWireSent, wkWireCount, setWalk_walk_self, freshWalk]
        rw [wiresBefore_succ sk hslt, hwc]
        simp)
      (fun hab => absurd rfl hab)
      (by
        intro _
        simp only [wkQSentTot, wkQSum, setWalk_walk_self, freshWalk]
        rw [qsBefore_succ sk hslt, hqc]
        simp [foldl_const])
      (by
        intro _
        by_cases hl2 : (s.walk pk).scope + 1 < sk.stageLen pk.2 <;>
          simp [wkParentSent, freshWalk, hph2, hpdT, hl2])
      (by
        rw [wkWireRecvd_fresh pk (by omega) (setWalk_walk_self _ _ _)]
        simp [wkWireRecvd, hph2])
      (by
        rw [wkAskedRecvd_fresh pk (by omega) (setWalk_walk_self _ _ _)]
        simp [wkAskedRecvd, hph2])
    refine ⟨⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩, hsent, hrecv⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        exact wkLocalOk_fresh pk' _ (by omega) hwalk'
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']
          exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hL.wk pk' hpk'
    · rw [← hs']
      exact hL.asm pk' hpk'
    · rw [← hs']
      exact hL.top
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
    obtain ⟨hsent, hrecv⟩ := counts_fire_assemble hwf pk hmem'
      (lowerOut pk) _ hs'
      (by
        rw [sentOf_lowerOut, sentOf_lowerOut]
        simp only [wkResSent, wkResCount, setWalk_walk_self]
        rw [length_filter_insert (p := fun j => (s.walk pk).resDone j)
          hifan hnrd]
        omega)
      (by
        intro _
        simp [wkWireSent, wkWireCount])
      (fun hab => absurd rfl hab)
      (by
        intro _
        simp [wkQSentTot, wkQSum])
      (by
        intro _
        simp [wkParentSent, hph2])
      (by simp [wkWireRecvd, hph2])
      (by simp [wkAskedRecvd, hph2])
    refine ⟨⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩, hsent, hrecv⟩
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
        exact hL.wk pk' hpk'
    · rw [← hs']
      exact hL.asm pk' hpk'
    · rw [← hs']
      exact hL.top

/-- The fire's cursor effect on a committed `.query i`, chan-free: the
`InvL` bullet and count deltas of `preserve_walkFire_query`'s local
part. -/
private theorem step_fire_query (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (i : Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some (Oblig.query i))
    (hs' : setWalk s pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) (Oblig.query i))) = s')
    (hL : InvL sk ax s) :
    InvL sk ax s'
    ∧ (∀ c ∈ allChans sk,
        sentOf sk s' c = sentOf sk s c
          + (if c = askedOut pk then 1 else 0))
    ∧ (∀ c ∈ allChans sk, recvdOf sk s' c = recvdOf sk s c) := by
  have hwk := hL.wk pk hmem'
  simp only [wkLocalOk] at hwk
  rw [hph2, hcm] at hwk
  simp at hwk
  obtain ⟨hslt, ⟨-, hC⟩, ⟨⟨⟨⟨⟨hin, hDi⟩, hqlt⟩, hqpre⟩, hd1⟩, hwf1⟩, -⟩ := hwk
  have hn : sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
      ≤ sk.fan := nChildren_le_fan hwf hslt
  have hifan : i < sk.fan := by omega
  have hp1 : 1 ≤ pk.2 := by
    have := ne_zero_of_childIsD (sk := sk) hDi
    omega
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
    obtain ⟨hsent, hrecv⟩ := counts_fire_assemble hwf pk hmem'
      (askedOut pk) _ hs'
      (by
        rw [sentOf_askedOut hwf hmem' hp1, sentOf_askedOut hwf hmem' hp1]
        simp only [wkQSentTot, wkQSum, setWalk_walk_self, freshWalk]
        rw [qsBefore_succ sk hslt, ← hqof, foldl_add_update' hifan]
        simp [foldl_const]
        omega)
      (by
        intro _
        simp only [wkWireSent, wkWireCount, setWalk_walk_self, freshWalk]
        rw [wiresBefore_succ sk hslt, hwc]
        simp)
      (by
        intro _
        simp only [wkResSent, wkResCount, setWalk_walk_self, freshWalk]
        rw [dsBefore_succ sk hslt, hrc]
        simp)
      (fun hab => absurd rfl hab)
      (by
        intro _
        by_cases hl2 : (s.walk pk).scope + 1 < sk.stageLen pk.2 <;>
          simp [wkParentSent, freshWalk, hph2, hpdT, hl2])
      (by
        rw [wkWireRecvd_fresh pk (by omega) (setWalk_walk_self _ _ _)]
        simp [wkWireRecvd, hph2])
      (by
        rw [wkAskedRecvd_fresh pk (by omega) (setWalk_walk_self _ _ _)]
        simp [wkAskedRecvd, hph2])
    refine ⟨⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩, hsent, hrecv⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        exact wkLocalOk_fresh pk' _ (by omega) hwalk'
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']
          exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hL.wk pk' hpk'
    · rw [← hs']
      exact hL.asm pk' hpk'
    · rw [← hs']
      exact hL.top
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
    obtain ⟨hsent, hrecv⟩ := counts_fire_assemble hwf pk hmem'
      (askedOut pk) _ hs'
      (by
        rw [sentOf_askedOut hwf hmem' hp1, sentOf_askedOut hwf hmem' hp1]
        simp only [wkQSentTot, wkQSum, setWalk_walk_self]
        rw [foldl_add_update' hifan]
        omega)
      (by
        intro _
        simp [wkWireSent, wkWireCount])
      (by
        intro _
        simp [wkResSent, wkResCount])
      (fun hab => absurd rfl hab)
      (by
        intro _
        simp [wkParentSent, hph2])
      (by simp [wkWireRecvd, hph2])
      (by simp [wkAskedRecvd, hph2])
    refine ⟨⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩, hsent, hrecv⟩
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
        · -- d4 shadow: a shadowed budget cannot be `i`'s
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
        exact hL.wk pk' hpk'
    · rw [← hs']
      exact hL.asm pk' hpk'
    · rw [← hs']
      exact hL.top

-- ========================================================== the theorem

/-- The fire step fact: the cursor effect of firing walk `pk`'s
committed obligation — shared by the base `walkFire` arm (which also
bumps the fired channel) and by `firePush`'s walk case (which appends
to the pipe instead) — preserves `InvL`, raises the fired channel's
producer count by one, frames every other count, leaves occupancy
untouched, and clears the wire hand at `pk` while framing every other
stream's. -/
theorem step_fire (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (o : Oblig) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some o)
    (hs' : setWalk s pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) o)) = s')
    (hL : InvL sk ax s) :
    InvL sk ax s'
    ∧ (∀ c ∈ allChans sk,
        sentOf sk s' c = sentOf sk s c
          + (if c = obligChan pk o then 1 else 0))
    ∧ (∀ c ∈ allChans sk, recvdOf sk s' c = recvdOf sk s c)
    ∧ s'.chan = s.chan
    ∧ (∀ p h, (p, h) ≠ pk → holdsWire sk p h s' = holdsWire sk p h s)
    ∧ wireHand (s'.walk pk) = false := by
  have hchan : s'.chan = s.chan := by
    rw [← hs']
    rfl
  obtain ⟨hoff, hhand⟩ := fire_hands pk o hs'
  cases o with
  | wire i =>
      obtain ⟨hL', hsent, hrecv⟩ :=
        step_fire_wire hwf pk i hmem' hph2 hcm hs' hL
      exact ⟨hL', hsent, hrecv, hchan, hoff, hhand⟩
  | res i =>
      obtain ⟨hL', hsent, hrecv⟩ :=
        step_fire_res hwf pk i hmem' hph2 hcm hs' hL
      exact ⟨hL', hsent, hrecv, hchan, hoff, hhand⟩
  | query i =>
      obtain ⟨hL', hsent, hrecv⟩ :=
        step_fire_query hwf pk i hmem' hph2 hcm hs' hL
      exact ⟨hL', hsent, hrecv, hchan, hoff, hhand⟩
  | parent =>
      obtain ⟨hL', hsent, hrecv⟩ :=
        step_fire_parent hwf pk hmem' hph2 hcm hs' hL
      exact ⟨hL', hsent, hrecv, hchan, hoff, hhand⟩

end StreamingMirror.Mux
