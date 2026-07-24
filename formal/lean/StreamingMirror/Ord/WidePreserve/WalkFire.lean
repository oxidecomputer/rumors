/-
Wide-O preservation for `walkFire` over `InvPWO` at every capacity
vector κ: the κ-guarded push, whose guard value is never consumed —
the fired channel's conservation needs only the producer-count rise,
so the entire per-obligation analysis (ledger re-establishment when
staying, the prefix-sum telescopes when advancing) is
Ord/Preserve/WalkFire.lean's verbatim, with the flow assembler's
occupancy-zero and capacity-one hypotheses dropped alongside the
capacity conjunct. The counting infrastructure re-lands as private
twins (cited per docstring), exactly as it did there.

Chain (ord, stage G): the widened flagship's walk fire-step
preservation case, consumed by Ord/WidePreserve.lean. Base mirror:
Proofs/Wide.lean (`invPW_preserved_W`'s fire arm); minus-cap source:
Ord/Preserve/WalkFire.lean. Map: PROGRESS.md §13.
-/
import StreamingMirror.Ord.Wide

namespace StreamingMirror.Ord

open Model

variable {sk : Skel} {κ : Chan → Nat} {ax : AxMode} {ord : OrdMap} {s s' : State}

-- ================================================ counting infrastructure
-- Private twins of Ord/Preserve/WalkFire.lean's private counting layer,
-- verbatim.

/-- A filter over `range fan` collapses to `range n` when the predicate
is dead above `n` (a private twin of `length_filter_range_ext`). -/
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

/-- Summing a constant-zero map is zero (a private twin of
`sum_map_zero`). -/
private theorem sum_map_zero (l : List Nat) :
    (l.map (fun _ : Nat => (0 : Nat))).sum = 0 := by
  induction l with
  | nil => rfl
  | cons x xs ih => simp [ih]

/-- A fold-sum over `range fan` collapses to `range n` when the summand
is dead above `n` (a private twin of `sum_range_ext`). -/
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

/-- Positional filtering over indices equals filtering the list (a
private twin of `length_filter_index`). -/
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

/-- A D child is a real child (a private twin of
`lt_nChildren_of_childIsD`). -/
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

/-- Only non-leaf stages have D children (a private twin of
`ne_zero_of_childIsD`). -/
private theorem ne_zero_of_childIsD {h sc j : Nat}
    (hd : sk.childIsD h sc j = true) : h ≠ 0 := by
  intro h0
  subst h0
  unfold Skel.childIsD at hd
  simp at hd

/-- The D-child test is dead past the child count (a private twin of
`childIsD_eq_false_of_ge`). -/
private theorem childIsD_eq_false_of_ge {h sc j : Nat}
    (hj : sk.nChildren h sc ≤ j) : sk.childIsD h sc j = false := by
  unfold Skel.childIsD
  by_cases hh : (h == 0) = true
  · rw [if_pos hh]
  · rw [if_neg hh]
    unfold Skel.nChildren at hj
    rw [if_neg hh] at hj
    rw [List.getElem?_eq_none hj]

/-- Query budgets exist only for D children (a private twin of
`qCount_eq_zero_of_not_childIsD`). -/
private theorem qCount_eq_zero_of_not_childIsD {h sc j : Nat}
    (hd : sk.childIsD h sc j = false) : sk.qCount h sc j = 0 := by
  unfold Skel.qCount
  rw [hd]
  simp

private theorem qCount_eq_zero_of_ge {h sc j : Nat}
    (hj : sk.nChildren h sc ≤ j) : sk.qCount h sc j = 0 :=
  qCount_eq_zero_of_not_childIsD (childIsD_eq_false_of_ge hj)

-- ================================================= completion counting

/-- A bounded ledger that covers its bound counts to exactly the bound
(a private twin of `wireCount_of_complete`). -/
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

/-- A ledger that holds exactly the D children counts to `dOf` (a
private twin of `count_eq_dOf`). -/
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

/-- A query ledger that is pointwise the budget sums to `qOf` (a
private twin of `qSum_eq_qOf`). -/
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
`qOf` (a private twin of `qSum_of_complete`). -/
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

/-- A fresh cursor's wire receive count is its scope index (a private
twin of `wkWireRecvd_fresh`). -/
private theorem wkWireRecvd_fresh {t : State} (pk : Party × Nat) {k : Nat}
    (hk : k ≤ sk.stageLen pk.2) (hw : t.walk pk = freshWalk sk pk.2 k) :
    wkWireRecvd sk t pk = k := by
  by_cases hl : k < sk.stageLen pk.2
  · simp [wkWireRecvd, hw, freshWalk, hl]
  · have hz : k = sk.stageLen pk.2 := by omega
    simp [wkWireRecvd, hw, freshWalk, hl]
    omega

/-- A fresh cursor's query receive count is its scope index (a private
twin of `wkAskedRecvd_fresh`). -/
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

/-- A key of the other party never collides with `pk` (a private twin
of `key_other_ne`). -/
private theorem key_other_ne (p : Party) (a b : Nat) :
    ((p.other, a) : Party × Nat) ≠ (p, b) := by
  intro hcon
  have hfst := congrArg Prod.fst hcon
  simp at hfst
  exact other_ne_self p hfst

/-- Producer counts never read channel occupancy (a private twin of
`sentOf_chan_irrel`). -/
private theorem sentOf_chan_irrel (ch : Chan → Nat) (c : Chan) :
    sentOf sk { s with chan := ch } c = sentOf sk s c := by
  cases c <;> rfl

/-- Sends frame for a walk-plus-channel update (a private twin of
`sentOf_fire_frame`; the producer side is order-blind). -/
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

/-- Assembles the flow field of `InvPWO` for a fire step (a minus-cap
twin of Ord/Preserve/WalkFire.lean's `flow_fireO_assemble`): the
touched channel's occupancy rises by one exactly as its producer count
rises by one — no occupancy-zero or capacity hypothesis anywhere;
every other channel frames, the O receive counts riding the two
unchanged base counts through `recvdOfO_setWalk_same`. -/
private theorem flow_fireWO_assemble (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (hmem' : pk ∈ sk.walkKeys) (c₀ : Chan) (W : WalkSt)
    (hs' : setWalk { s with chan := bump s.chan c₀ 1 } pk W = s')
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
    (hi : InvPWO sk ax ord s) :
    ∀ c ∈ allChans sk,
      s'.chan c + recvdOfO sk ord s' c = sentOf sk s' c := by
  intro c hc
  have heq := hi.flow c hc
  have hchan : s'.chan = bump s.chan c₀ 1 := by
    rw [← hs']
    rfl
  have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
    rw [← hs']
    refine Eq.trans
      (recvdOfO_setWalk_same hwf { s with chan := bump s.chan c₀ 1 } pk W
        hmem' (wkWireRecvdO_congr pk hWr hAr)
        (wkAskedRecvdO_congr pk hWr hAr) hc) ?_
    exact recvdOfO_ext (fun _ => rfl) (fun _ => rfl) (fun _ => rfl)
      rfl rfl rfl rfl rfl rfl c
  by_cases hne : c = c₀
  · subst hne
    have hsent : sentOf sk s' c = sentOf sk s c + 1 := by
      rw [← hs']
      exact htouch
    rw [hchan, hrecv, hsent, bump_one]
    omega
  · have hsent : sentOf sk s' c = sentOf sk s c := by
      rw [← hs']
      exact sentOf_fire_frame hwf _ pk W hmem' c₀ hW hR hQ hP hc hne
    rw [hchan, hsent, hrecv, bump_ne _ _ hne]
    exact heq

-- ================================================ per-obligation lemmas

/-- `walkFire` on a committed `.parent`, over the weak O invariant (a
minus-cap twin of `preserve_walkFire_parentO`; the local analysis is
verbatim, the flow field rides `flow_fireWO_assemble`). -/
private theorem preserve_walkFire_parentWO (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some Oblig.parent)
    (hs' : setWalk { s with chan := bump s.chan (upperOut pk) 1 } pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) Oblig.parent)) = s')
    (hi : InvPWO sk ax ord s) : InvPWO sk ax ord s' := by
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
      flow_fireWO_assemble hwf pk hmem' (upperOut pk) _ hs'
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
      flow_fireWO_assemble hwf pk hmem' (upperOut pk) _ hs'
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

/-- `walkFire` on a committed `.wire i`, over the weak O invariant (a
minus-cap twin of `preserve_walkFire_wireO`). -/
private theorem preserve_walkFire_wireWO (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (i : Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some (Oblig.wire i))
    (hs' : setWalk { s with chan := bump s.chan (wireOut pk) 1 } pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) (Oblig.wire i))) = s')
    (hi : InvPWO sk ax ord s) : InvPWO sk ax ord s' := by
  have hwk := hi.wk pk hmem'
  simp only [wkLocalOk] at hwk
  rw [hph2, hcm] at hwk
  simp at hwk
  obtain ⟨hslt, ⟨-, hC⟩, ⟨⟨hieq, hin⟩, hd4⟩, -⟩ := hwk
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
      flow_fireWO_assemble hwf pk hmem' (wireOut pk) _ hs'
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
      flow_fireWO_assemble hwf pk hmem' (wireOut pk) _ hs'
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

/-- `walkFire` on a committed `.res i`, over the weak O invariant (a
minus-cap twin of `preserve_walkFire_resO`). -/
private theorem preserve_walkFire_resWO (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (i : Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some (Oblig.res i))
    (hs' : setWalk { s with chan := bump s.chan (lowerOut pk) 1 } pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) (Oblig.res i))) = s')
    (hi : InvPWO sk ax ord s) : InvPWO sk ax ord s' := by
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
      flow_fireWO_assemble hwf pk hmem' (lowerOut pk) _ hs'
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
      flow_fireWO_assemble hwf pk hmem' (lowerOut pk) _ hs'
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

/-- `walkFire` on a committed `.query i`, over the weak O invariant (a
minus-cap twin of `preserve_walkFire_queryO`). -/
private theorem preserve_walkFire_queryWO (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (i : Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some (Oblig.query i))
    (hs' : setWalk { s with chan := bump s.chan (askedOut pk) 1 } pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) (Oblig.query i))) = s')
    (hi : InvPWO sk ax ord s) : InvPWO sk ax ord s' := by
  have hwk := hi.wk pk hmem'
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
      flow_fireWO_assemble hwf pk hmem' (askedOut pk) _ hs'
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
      flow_fireWO_assemble hwf pk hmem' (askedOut pk) _ hs'
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

/-- `walkFire` wide: the κ-guarded push, whose guard value is never
consumed — firing moves no receive count in either order, and the O
invariant rides the per-obligation analysis unchanged at every κ (cf.
`preserve_walkFireO` minus the capacity half). -/
theorem preserve_walkFireWO (hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : applyWO sk κ ax ord (.walkFire pk) s = some s')
    (hi : InvPWO sk ax ord s) : InvPWO sk ax ord s' := by
  have hstep' : applyW sk κ ax (.walkFire pk) s = some s' := hstep
  simp only [applyW] at hstep'
  split at hstep'
  next o hcm =>
    split at hstep'
    case isFalse => simp at hstep'
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
      obtain ⟨⟨hmem, hph2⟩, -⟩ := hg
      have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
      injection hstep' with hs'
      cases o with
      | wire i =>
          exact preserve_walkFire_wireWO hwf pk i hmem' hph2 hcm hs' hi
      | res i =>
          exact preserve_walkFire_resWO hwf pk i hmem' hph2 hcm hs' hi
      | query i =>
          exact preserve_walkFire_queryWO hwf pk i hmem' hph2 hcm hs' hi
      | parent =>
          exact preserve_walkFire_parentWO hwf pk hmem' hph2 hcm hs' hi
  next hcm => simp at hstep'

end StreamingMirror.Ord
