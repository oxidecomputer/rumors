/-
Fire-shape lemmas for the muxed system, in RAW-EFFECT form: the muxed
`push` move fires a committed obligation into a pipe WITHOUT bumping
the base channel, so the local-invariant and count-delta lemmas here
are stated over the bare `setWalk` effect. Wrappers re-attach the
channel bump for the base `walkFire` arm (the muxed `applyBase`
dispatch, which never sees a `.wire` fire — those route through
`push`).

Extraction source: Proofs/Preserve/WalkFire.lean. The monolith's
`preserve_walkFire_*` proofs interleave LOCAL bullets (wk/asm/top —
never reading `hi.flow`) with FLOW bullets; a muxed state does not
satisfy `InvP.flow` (frames ride the pipe), so the local parts
(`preserveL_fire`) and the per-arm count deltas (`delta_fire`) are
extracted here standalone. The local invariants never read `chan`,
and `sentOf`/`recvdOf` never read `chan`, so the monolith's proof
scripts apply with the chan bump dropped from the state. Private
helpers of the monolith are re-derived verbatim below; its public
helpers (`wkLocalOk_fresh`, `length_filter_insert`,
`foldl_add_update'`, `frontier_of_count`, the `*Before_succ`
telescopes) are imported.
-/
import StreamingMirror.Mux.Basic
import StreamingMirror.Proofs.Preserve

namespace StreamingMirror.Mux

open Model

variable {sk : Skel} {ax : AxMode} {s s' : State}

-- ================================================ counting infrastructure
-- Verbatim copies of the monolith's private counting helpers
-- (Proofs/Preserve/WalkFire.lean).

/-- A filter over `range fan` collapses to `range n` when the predicate
is dead above `n`.

Copy of the monolith's private `length_filter_range_ext`. -/
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

/-- Summing a constant-zero map is zero.

Copy of the monolith's private `sum_map_zero`. -/
private theorem sum_map_zero (l : List Nat) :
    (l.map (fun _ : Nat => (0 : Nat))).sum = 0 := by
  induction l with
  | nil => rfl
  | cons x xs ih => simp [ih]

/-- A fold-sum over `range fan` collapses to `range n` when the summand
is dead above `n`.

Copy of the monolith's private `sum_range_ext`. -/
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

/-- Positional filtering over indices equals filtering the list.

Copy of the monolith's private `length_filter_index`: the bridge
between `childIsD`'s `kids[j]?` reads and `kids.filter`. -/
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
-- Verbatim copies of the monolith's private skeleton helpers.

/-- A D child is a real child: its index is within the child count.

Copy of the monolith's private `lt_nChildren_of_childIsD`. -/
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

/-- Only non-leaf stages have D children.

Copy of the monolith's private `ne_zero_of_childIsD`. -/
private theorem ne_zero_of_childIsD {h sc j : Nat}
    (hd : sk.childIsD h sc j = true) : h ≠ 0 := by
  intro h0
  subst h0
  unfold Skel.childIsD at hd
  simp at hd

/-- The D-child test is dead past the child count.

Copy of the monolith's private `childIsD_eq_false_of_ge`. -/
private theorem childIsD_eq_false_of_ge {h sc j : Nat}
    (hj : sk.nChildren h sc ≤ j) : sk.childIsD h sc j = false := by
  unfold Skel.childIsD
  by_cases hh : (h == 0) = true
  · rw [if_pos hh]
  · rw [if_neg hh]
    unfold Skel.nChildren at hj
    rw [if_neg hh] at hj
    rw [List.getElem?_eq_none hj]

/-- Query budgets exist only for D children.

Copy of the monolith's private `qCount_eq_zero_of_not_childIsD`. -/
private theorem qCount_eq_zero_of_not_childIsD {h sc j : Nat}
    (hd : sk.childIsD h sc j = false) : sk.qCount h sc j = 0 := by
  unfold Skel.qCount
  rw [hd]
  simp

/-- Query budgets vanish past the child count.

Copy of the monolith's private `qCount_eq_zero_of_ge`. -/
private theorem qCount_eq_zero_of_ge {h sc j : Nat}
    (hj : sk.nChildren h sc ≤ j) : sk.qCount h sc j = 0 :=
  qCount_eq_zero_of_not_childIsD (childIsD_eq_false_of_ge hj)

-- ================================================= completion counting
-- Verbatim copies of the monolith's private completion counters.

/-- A bounded ledger that covers its bound counts to exactly the bound.

Copy of the monolith's private `wireCount_of_complete`: the wire ledger
of a completed scope counts to `nChildren`. -/
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

/-- A ledger that holds exactly the D children counts to `dOf`.

Copy of the monolith's private `count_eq_dOf`: the res ledger of a
completed scope. -/
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

/-- A query ledger that is pointwise the budget sums to `qOf`.

Copy of the monolith's private `qSum_eq_qOf`. -/
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
`qOf`.

Copy of the monolith's private `qSum_of_complete`: non-D and
out-of-range budgets are zero, so the bound pins them. -/
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

/-- A fresh cursor's prologue-wire count is its cursor position.

Copy of the monolith's private `wkWireRecvd_fresh`. -/
private theorem wkWireRecvd_fresh {t : State} (pk : Party × Nat) {k : Nat}
    (hk : k ≤ sk.stageLen pk.2) (hw : t.walk pk = freshWalk sk pk.2 k) :
    wkWireRecvd sk t pk = k := by
  by_cases hl : k < sk.stageLen pk.2
  · simp [wkWireRecvd, hw, freshWalk, hl]
  · have hz : k = sk.stageLen pk.2 := by omega
    simp [wkWireRecvd, hw, freshWalk, hl]
    omega

/-- A fresh cursor's prologue-query count is its cursor position.

Copy of the monolith's private `wkAskedRecvd_fresh`. -/
private theorem wkAskedRecvd_fresh {t : State} (pk : Party × Nat) {k : Nat}
    (hk : k ≤ sk.stageLen pk.2) (hw : t.walk pk = freshWalk sk pk.2 k) :
    wkAskedRecvd sk t pk = k := by
  by_cases hl : k < sk.stageLen pk.2
  · simp [wkAskedRecvd, hw, freshWalk, hl]
  · have hz : k = sk.stageLen pk.2 := by omega
    simp [wkAskedRecvd, hw, freshWalk, hl]
    omega

-- =================================================== raw flow frames

/-- Membership-free producer-side frame for a walk update.

Copy of `sentOf_setWalk_frame` (Proofs/Wiring.lean) with the `allChans`
membership hypothesis dropped: the producer routing never sends a
channel outside `allChans` to `pk` either, and the muxed deltas
quantify over ALL channels, so membership is dead weight here. -/
private theorem sentOf_setWalk_frame' (s : State) (pk : Party × Nat)
    (ws' : WalkSt) {c : Chan}
    (h1 : c ≠ wireOut pk) (h2 : c ≠ lowerOut pk) (h3 : c ≠ askedOut pk)
    (h4 : c ≠ upperOut pk) :
    sentOf sk (setWalk s pk ws') c = sentOf sk s c := by
  cases c with
  | wire p h =>
      cases hb : h == sk.rootH with
      | true => simp [sentOf, hb, setWalk]
      | false =>
          have hq : (p, h) ≠ pk := by
            intro he; subst he; exact h1 rfl
          simp [sentOf, hb, wkWireSent, wkWireCount,
            setWalk_walk_ne s ws' hq]
  | asked p h =>
      cases hb1 : p == Party.I && h == sk.rootH - 1 with
      | true => simp [sentOf, hb1, setWalk]
      | false =>
          cases hb2 : p == Party.R && h == sk.rootH - 2 with
          | true => simp [sentOf, hb1, hb2, setWalk]
          | false =>
              have hlt : ¬ (h + 2 < 2) := by omega
              have hq : (p, h + 2) ≠ pk := by
                intro he; subst he
                exact h3 (by simp [askedOut, hlt])
              simp [sentOf, hb1, hb2, wkQSentTot, wkQSum,
                setWalk_walk_ne s ws' hq]
  | leafRequests =>
      have hq : (Party.I, 1) ≠ pk := by
        intro he; subst he; exact h3 rfl
      simp [sentOf, wkQSentTot, wkQSum, setWalk_walk_ne s ws' hq]
  | upper p h =>
      have hq : (p, h) ≠ pk := by
        intro he; subst he; exact h4 rfl
      simp [sentOf, wkParentSent, setWalk_walk_ne s ws' hq]
  | lower p h =>
      have hq : (p, h) ≠ pk := by
        intro he; subst he; exact h2 rfl
      simp [sentOf, wkResSent, wkResCount, setWalk_walk_ne s ws' hq]
  | level p j => rfl
  | rootret => rfl
  | rootrets => rfl
  | rootres => rfl

/-- Sends frame for a raw fire update, away from the touched channel.

Raw-effect counterpart of the monolith's private `sentOf_fire_frame`
(Proofs/Preserve/WalkFire.lean): each of `pk`'s producer counts is
unchanged unless it feeds the touched channel `c₀`, so every other
channel — with no membership restriction — reads the same producer
count. -/
private theorem sentOf_fire_frame (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (ws' : WalkSt) (hmem : pk ∈ sk.walkKeys) (c₀ : Chan)
    (hW : c₀ ≠ wireOut pk →
      wkWireSent sk (setWalk s pk ws') pk = wkWireSent sk s pk)
    (hR : c₀ ≠ lowerOut pk →
      wkResSent sk (setWalk s pk ws') pk = wkResSent sk s pk)
    (hQ : c₀ ≠ askedOut pk →
      wkQSentTot sk (setWalk s pk ws') pk = wkQSentTot sk s pk)
    (hP : c₀ ≠ upperOut pk →
      wkParentSent (setWalk s pk ws') pk = wkParentSent s pk)
    {c : Chan} (hne : c ≠ c₀) :
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
  · exact sentOf_setWalk_frame' s pk ws' h1 h2 h3 h4

/-- Receives frame for a raw fire update, over all channels.

Raw-effect counterpart of the monolith's private `recvdOf_fire_frame`
(Proofs/Preserve/WalkFire.lean), restated without the `allChans`
membership hypothesis: any channel whose consumer key collides with
`pk` — including the phantom `wire I 0`, which routes to `(R, 0)` by
Nat subtraction — is dispatched by the two pinned consumer counts, so
membership never enters. -/
private theorem recvdOf_fire_frame (pk : Party × Nat) (ws' : WalkSt)
    (hWr : wkWireRecvd sk (setWalk s pk ws') pk = wkWireRecvd sk s pk)
    (hAr : wkAskedRecvd sk (setWalk s pk ws') pk = wkAskedRecvd sk s pk)
    (c : Chan) :
    recvdOf sk (setWalk s pk ws') c = recvdOf sk s c := by
  cases c with
  | wire p h =>
      by_cases hr : h = sk.rootH
      · subst hr
        cases p with
        | I => simp [recvdOf, setWalk]
        | R =>
            by_cases hq : ((Party.I, sk.rootH - 1) : Party × Nat) = pk
            · have hg : ∀ t : State,
                  recvdOf sk t (Chan.wire Party.R sk.rootH)
                    = wkWireRecvd sk t (Party.I, sk.rootH - 1) := by
                intro t; simp [recvdOf]
              rw [hg, hg, hq]
              exact hWr
            · simp [recvdOf, wkWireRecvd, setWalk_walk_ne _ _ hq]
      · by_cases hz : p = Party.R ∧ h = 0
        · obtain ⟨rfl, rfl⟩ := hz
          simp [recvdOf, hr, absorbWireRecvd, setWalk]
        · by_cases hq : ((p.other, h - 1) : Party × Nat) = pk
          · have hg : ∀ t : State, recvdOf sk t (Chan.wire p h)
                = wkWireRecvd sk t (p.other, h - 1) := by
              intro t; simp [recvdOf, hr, hz]
            rw [hg, hg, hq]
            exact hWr
          · simp [recvdOf, hr, hz, wkWireRecvd, setWalk_walk_ne _ _ hq]
  | asked p h =>
      by_cases hq : ((p, h) : Party × Nat) = pk
      · have hg : ∀ t : State, recvdOf sk t (Chan.asked p h)
            = wkAskedRecvd sk t (p, h) := fun _ => rfl
        rw [hg, hg, hq]
        exact hAr
      · simp [recvdOf, wkAskedRecvd, setWalk_walk_ne _ _ hq]
  | leafRequests => rfl
  | upper p h => rfl
  | lower p h => rfl
  | level p j => rfl
  | rootret => rfl
  | rootrets => rfl
  | rootres => rfl

/-- Assembles the raw count delta of a fire step.

Raw-effect counterpart of the monolith's private `flow_fire_assemble`
(Proofs/Preserve/WalkFire.lean): no channel bump and no flow field —
just the +1 on the fired channel's producer count and the frames on
every other count, quantified over all channels. -/
private theorem delta_fire_assemble (hwf : sk.wellFormed = true)
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
    sentOf sk s' c₀ = sentOf sk s c₀ + 1
    ∧ (∀ c, c ≠ c₀ → sentOf sk s' c = sentOf sk s c)
    ∧ (∀ c, recvdOf sk s' c = recvdOf sk s c) := by
  subst hs'
  exact ⟨htouch,
    fun c hc => sentOf_fire_frame hwf pk W hmem' c₀ hW hR hQ hP hc,
    fun c => recvdOf_fire_frame pk W hWr hAr c⟩

-- =============================== per-obligation local lemmas (raw form)

/-- Raw local preservation of a `.parent` fire.

Extracts the wk/asm/top bullets of `preserve_walkFire_parent`
(Proofs/Preserve/WalkFire.lean); the flow bullet and the channel bump
are dropped — the local invariants never read `chan`. -/
private theorem preserveL_fire_parent (_hwf : sk.wellFormed = true)
    (pk : Party × Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some Oblig.parent)
    (hs' : setWalk s pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) Oblig.parent)) = s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  have hwk := hi.wk pk hmem'
  simp only [wkLocalOk] at hwk
  rw [hph2, hcm] at hwk
  simp at hwk
  obtain ⟨hslt, ⟨-, hC⟩, -, -⟩ := hwk
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
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩
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
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩
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

/-- Raw local preservation of a `.wire i` fire.

Extracts the wk/asm/top bullets of `preserve_walkFire_wire`
(Proofs/Preserve/WalkFire.lean); the flow bullet, the channel bump and
the flow-only counting facts are dropped. -/
private theorem preserveL_fire_wire (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (i : Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some (Oblig.wire i))
    (hs' : setWalk s pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) (Oblig.wire i))) = s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
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
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩
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
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩
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

/-- Raw local preservation of a `.res i` fire.

Extracts the wk/asm/top bullets of `preserve_walkFire_res`
(Proofs/Preserve/WalkFire.lean); the flow bullet, the channel bump and
the flow-only counting facts are dropped. -/
private theorem preserveL_fire_res (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (i : Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some (Oblig.res i))
    (hs' : setWalk s pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) (Oblig.res i))) = s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  have hwk := hi.wk pk hmem'
  simp only [wkLocalOk] at hwk
  rw [hph2, hcm] at hwk
  simp at hwk
  obtain ⟨hslt, ⟨-, hC⟩, ⟨⟨⟨⟨hin, hDi⟩, -⟩, hpre⟩, hwi⟩, hd3⟩ := hwk
  have hn : sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
      ≤ sk.fan := nChildren_le_fan hwf hslt
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
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩
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
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩
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

/-- Raw local preservation of a `.query i` fire.

Extracts the wk/asm/top bullets of `preserve_walkFire_query`
(Proofs/Preserve/WalkFire.lean); the flow bullet, the channel bump and
the flow-only counting facts are dropped. -/
private theorem preserveL_fire_query (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (i : Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some (Oblig.query i))
    (hs' : setWalk s pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) (Oblig.query i))) = s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  have hwk := hi.wk pk hmem'
  simp only [wkLocalOk] at hwk
  rw [hph2, hcm] at hwk
  simp at hwk
  obtain ⟨hslt, ⟨-, hC⟩, ⟨⟨⟨⟨⟨hin, hDi⟩, hqlt⟩, hqpre⟩, hd1⟩, hwf1⟩, -⟩ := hwk
  have hn : sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
      ≤ sk.fan := nChildren_le_fan hwf hslt
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
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩
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
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩
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
        · -- d4 shadow: a shadowed budget cannot be `i`'s (strict arm)
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

-- ======================================= the raw local lemma, assembled

/-- Raw local preservation of a committed fire, over all four
obligation kinds: firing `o` and normalizing the cursor — with NO
channel bump — preserves the local invariant fragment.

This is the muxed `push` move's local obligation: the mux fires a
committed obligation into a pipe without touching the base channel.
Extracted from `preserve_walkFire_{parent,wire,res,query}`
(Proofs/Preserve/WalkFire.lean), local bullets only — the local
invariants never read `chan`, so the monolith's scripts apply with the
bump dropped. -/
theorem preserveL_fire (hwf : sk.wellFormed = true) (pk : Party × Nat)
    (hmem : pk ∈ sk.walkKeys) {o : Oblig}
    (hph : (s.walk pk).phase = 2) (hcm : (s.walk pk).committed = some o)
    (hi : InvL sk ax s) :
    InvL sk ax
      (setWalk s pk (normWalk sk pk.2 (fireOblig (s.walk pk) o))) := by
  cases o with
  | wire i => exact preserveL_fire_wire hwf pk i hmem hph hcm rfl hi
  | res i => exact preserveL_fire_res hwf pk i hmem hph hcm rfl hi
  | query i => exact preserveL_fire_query hwf pk i hmem hph hcm rfl hi
  | parent => exact preserveL_fire_parent hwf pk hmem hph hcm rfl hi

-- =============================== per-obligation delta lemmas (raw form)

/-- Raw count delta of a `.parent` fire: `upperOut` rises by one, all
other counts frame.

Extracts the flow bullet of `preserve_walkFire_parent`
(Proofs/Preserve/WalkFire.lean): the touch is `parentDone` flipping
(staying) or telescoping into the next cursor (advancing). -/
private theorem delta_fire_parent (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some Oblig.parent)
    (hs' : setWalk s pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) Oblig.parent)) = s')
    (hi : InvL sk ax s) :
    sentOf sk s' (upperOut pk) = sentOf sk s (upperOut pk) + 1
    ∧ (∀ c, c ≠ upperOut pk → sentOf sk s' c = sentOf sk s c)
    ∧ (∀ c, recvdOf sk s' c = recvdOf sk s c) := by
  have hwk := hi.wk pk hmem'
  simp only [wkLocalOk] at hwk
  rw [hph2, hcm] at hwk
  simp at hwk
  obtain ⟨hslt, ⟨-, hC⟩, hpd, -⟩ := hwk
  have hn : sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
      ≤ sk.fan := nChildren_le_fan hwf hslt
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
  · -- the scope completes: the counts telescope into the prefix sums
    rw [show normWalk sk pk.2
        { scope := (s.walk pk).scope, phase := 2,
          wireDone := (s.walk pk).wireDone, resDone := (s.walk pk).resDone,
          qSent := (s.walk pk).qSent, parentDone := true,
          committed := none }
        = freshWalk sk pk.2 ((s.walk pk).scope + 1) from by
      simp [normWalk, hadv]] at hs'
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
    refine delta_fire_assemble hwf pk hmem' (upperOut pk) _ hs'
      ?_ ?_ ?_ ?_ ?_ ?_ ?_
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
    refine delta_fire_assemble hwf pk hmem' (upperOut pk) _ hs'
      ?_ ?_ ?_ ?_ ?_ ?_ ?_
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

/-- Raw count delta of a `.wire i` fire: `wireOut` rises by one, all
other counts frame.

Extracts the flow bullet of `preserve_walkFire_wire`
(Proofs/Preserve/WalkFire.lean): staying inserts `i` into the live
ledger, advancing forces `i + 1 = nChildren` and telescopes the count
into `wiresBefore`. -/
private theorem delta_fire_wire (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (i : Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some (Oblig.wire i))
    (hs' : setWalk s pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) (Oblig.wire i))) = s')
    (hi : InvL sk ax s) :
    sentOf sk s' (wireOut pk) = sentOf sk s (wireOut pk) + 1
    ∧ (∀ c, c ≠ wireOut pk → sentOf sk s' c = sentOf sk s c)
    ∧ (∀ c, recvdOf sk s' c = recvdOf sk s c) := by
  have hwk := hi.wk pk hmem'
  simp only [wkLocalOk] at hwk
  rw [hph2, hcm] at hwk
  simp at hwk
  obtain ⟨hslt, ⟨-, hC⟩, ⟨⟨hieq, hin⟩, -⟩, -⟩ := hwk
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
    refine delta_fire_assemble hwf pk hmem' (wireOut pk) _ hs'
      ?_ ?_ ?_ ?_ ?_ ?_ ?_
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
    refine delta_fire_assemble hwf pk hmem' (wireOut pk) _ hs'
      ?_ ?_ ?_ ?_ ?_ ?_ ?_
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

/-- Raw count delta of a `.res i` fire: `lowerOut` rises by one, all
other counts frame.

Extracts the flow bullet of `preserve_walkFire_res`
(Proofs/Preserve/WalkFire.lean): staying inserts `i` into the live
ledger, advancing counts the fired ledger as exactly the scope's D
children and telescopes into `dsBefore`. -/
private theorem delta_fire_res (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (i : Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some (Oblig.res i))
    (hs' : setWalk s pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) (Oblig.res i))) = s')
    (hi : InvL sk ax s) :
    sentOf sk s' (lowerOut pk) = sentOf sk s (lowerOut pk) + 1
    ∧ (∀ c, c ≠ lowerOut pk → sentOf sk s' c = sentOf sk s c)
    ∧ (∀ c, recvdOf sk s' c = recvdOf sk s c) := by
  have hwk := hi.wk pk hmem'
  simp only [wkLocalOk] at hwk
  rw [hph2, hcm] at hwk
  simp at hwk
  obtain ⟨hslt, ⟨-, hC⟩, ⟨⟨⟨⟨hin, hDi⟩, hnrd⟩, -⟩, -⟩, -⟩ := hwk
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
    refine delta_fire_assemble hwf pk hmem' (lowerOut pk) _ hs'
      ?_ ?_ ?_ ?_ ?_ ?_ ?_
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
  · -- the scope is still incomplete: the live ledger gains `i`
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
    refine delta_fire_assemble hwf pk hmem' (lowerOut pk) _ hs'
      ?_ ?_ ?_ ?_ ?_ ?_ ?_
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

/-- Raw count delta of a `.query i` fire: `askedOut` rises by one, all
other counts frame.

Extracts the flow bullet of `preserve_walkFire_query`
(Proofs/Preserve/WalkFire.lean): staying bumps the budget ledger at
`i`, advancing saturates every budget and telescopes into `qsBefore`.
The stage is above the leaves (`childIsD` forces it), so the fired
channel is `askedOut` at `1 ≤ pk.2`. -/
private theorem delta_fire_query (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (i : Nat) (hmem' : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2)
    (hcm : (s.walk pk).committed = some (Oblig.query i))
    (hs' : setWalk s pk
      (normWalk sk pk.2 (fireOblig (s.walk pk) (Oblig.query i))) = s')
    (hi : InvL sk ax s) :
    sentOf sk s' (askedOut pk) = sentOf sk s (askedOut pk) + 1
    ∧ (∀ c, c ≠ askedOut pk → sentOf sk s' c = sentOf sk s c)
    ∧ (∀ c, recvdOf sk s' c = recvdOf sk s c) := by
  have hwk := hi.wk pk hmem'
  simp only [wkLocalOk] at hwk
  rw [hph2, hcm] at hwk
  simp at hwk
  obtain ⟨hslt, ⟨-, hC⟩, ⟨⟨⟨⟨⟨hin, hDi⟩, hqlt⟩, -⟩, -⟩, -⟩, -⟩ := hwk
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
    refine delta_fire_assemble hwf pk hmem' (askedOut pk) _ hs'
      ?_ ?_ ?_ ?_ ?_ ?_ ?_
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
  · -- the scope is still incomplete: the budget ledger bumps at `i`
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
    refine delta_fire_assemble hwf pk hmem' (askedOut pk) _ hs'
      ?_ ?_ ?_ ?_ ?_ ?_ ?_
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

-- ======================================= the raw delta lemma, assembled

/-- Raw count delta of a committed fire, over all four obligation
kinds: the fired channel's producer count rises by exactly one, every
other producer count frames, and every consumer count frames — with NO
channel bump.

`sentOf`/`recvdOf` never read `chan`, so this raw form serves both the
base `walkFire` arm (wrapper below) and the muxed `push` move, whose
effect is exactly this `setWalk` plus a pipe append. Extracted from the
flow bullets of `preserve_walkFire_{parent,wire,res,query}`
(Proofs/Preserve/WalkFire.lean). -/
theorem delta_fire (hwf : sk.wellFormed = true) (pk : Party × Nat)
    (hmem : pk ∈ sk.walkKeys) {o : Oblig}
    (hph : (s.walk pk).phase = 2) (hcm : (s.walk pk).committed = some o)
    (hi : InvL sk ax s) :
    sentOf sk (setWalk s pk (normWalk sk pk.2 (fireOblig (s.walk pk) o)))
        (obligChan pk o)
      = sentOf sk s (obligChan pk o) + 1
    ∧ (∀ c, c ≠ obligChan pk o →
        sentOf sk
            (setWalk s pk (normWalk sk pk.2 (fireOblig (s.walk pk) o))) c
          = sentOf sk s c)
    ∧ (∀ c,
        recvdOf sk
            (setWalk s pk (normWalk sk pk.2 (fireOblig (s.walk pk) o))) c
          = recvdOf sk s c) := by
  cases o with
  | wire i => exact delta_fire_wire hwf pk i hmem hph hcm rfl hi
  | res i => exact delta_fire_res hwf pk i hmem hph hcm rfl hi
  | query i => exact delta_fire_query hwf pk i hmem hph hcm rfl hi
  | parent => exact delta_fire_parent hwf pk hmem hph hcm rfl hi

-- ==================================== wrappers for the base walkFire arm
-- The base arm's effect is `setWalk { s with chan := bump … } pk W`,
-- which is `{ setWalk s pk W with chan := bump … }` by rfl; the glue
-- lemmas here say the local invariants and the counts are blind to the
-- chan component, so the raw lemmas above transport.

/-- The local invariant fragment is blind to channel occupancy.

`wkLocalOk`/`asmLocalOk`/`topLocalOk` never read `chan`, so a chan
override on a `setWalk` state preserves `InvL` (via the `*_congr`
lemmas of Proofs/Lemmas.lean, every read field untouched). -/
private theorem invL_setWalk_chan (ch : Chan → Nat) (pk : Party × Nat)
    (W : WalkSt) (hi : InvL sk ax (setWalk s pk W)) :
    InvL sk ax (setWalk { s with chan := ch } pk W) := by
  have hwk : ∀ pk',
      wkLocalOk sk ax (setWalk { s with chan := ch } pk W) pk'
        = wkLocalOk sk ax (setWalk s pk W) pk' :=
    fun pk' => wkLocalOk_congr sk ax pk' rfl
  have hasm : ∀ pk',
      asmLocalOk sk (setWalk { s with chan := ch } pk W) pk'
        = asmLocalOk sk (setWalk s pk W) pk' :=
    fun pk' => asmLocalOk_congr sk pk' rfl
  have htop : topLocalOk sk ax (setWalk { s with chan := ch } pk W)
      = topLocalOk sk ax (setWalk s pk W) :=
    topLocalOk_congr sk ax rfl rfl rfl rfl rfl rfl rfl rfl rfl rfl rfl rfl
  exact ⟨fun pk' hpk' => (hwk pk').trans (hi.wk pk' hpk'),
    fun pk' hpk' => (hasm pk').trans (hi.asm pk' hpk'),
    htop.trans hi.top⟩

/-- Producer counts are blind to channel occupancy.

Copy of the monolith's private `sentOf_chan_irrel`
(Proofs/Preserve/WalkFire.lean), stated in the `setWalk`-inside form
the wrapper's goal exposes. -/
private theorem sentOf_setWalk_chan (ch : Chan → Nat) (pk : Party × Nat)
    (W : WalkSt) (c : Chan) :
    sentOf sk (setWalk { s with chan := ch } pk W) c
      = sentOf sk (setWalk s pk W) c := by
  cases c <;> rfl

/-- Consumer counts are blind to channel occupancy.

Copy of the monolith's private `recvdOf_chan_irrel`
(Proofs/Preserve/WalkFire.lean), stated in the `setWalk`-inside form
the wrapper's goal exposes. -/
private theorem recvdOf_setWalk_chan (ch : Chan → Nat) (pk : Party × Nat)
    (W : WalkSt) (c : Chan) :
    recvdOf sk (setWalk { s with chan := ch } pk W) c
      = recvdOf sk (setWalk s pk W) c := by
  cases c <;> rfl

/-- A wire channel is never a non-wire obligation's target.

`obligChan` routes `.res`/`.query`/`.parent` to
`lower`/`asked`-or-`leafRequests`/`upper` — never the `wire`
constructor. -/
private theorem wire_ne_obligChan (pk : Party × Nat) {o : Oblig}
    (honw : ∀ i, o ≠ Oblig.wire i) (p : Party) (h : Nat) :
    Chan.wire p h ≠ obligChan pk o := by
  cases o with
  | wire i => exact absurd rfl (honw i)
  | res i => simp [obligChan, lowerOut]
  | query i =>
      simp only [obligChan, askedOut]
      split <;> simp
  | parent => simp [obligChan, upperOut]

/-- Every obligation channel has capacity one.

`obligChan` never routes to a `level` channel (the only family with
capacity `capLevel`), so `sk.cap` is 1 on all four kinds. -/
private theorem cap_obligChan (sk : Skel) (pk : Party × Nat) (o : Oblig) :
    sk.cap (obligChan pk o) = 1 := by
  cases o with
  | wire i => rfl
  | res i => rfl
  | query i =>
      show sk.cap (askedOut pk) = 1
      by_cases h2 : pk.2 < 2 <;> simp [askedOut, h2, Skel.cap]
  | parent => rfl

/-- Local preservation of the base `walkFire` arm, for the muxed
`applyBase` dispatch: the local invariant fragment survives the fire.

Wrapper over `preserveL_fire`: the arm's guard inversion mirrors the
monolith's `preserve_walkFire` (Proofs/Preserve/WalkFire.lean), and the
chan bump is discharged by chan-blindness of `InvL`. -/
theorem preserveL_walkFire (hwf : sk.wellFormed = true) (pk : Party × Nat)
    (hstep : Model.apply sk ax (.walkFire pk) s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  next o hcm =>
    split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
      obtain ⟨⟨hmem, hph2⟩, -⟩ := hg
      have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
      injection hstep with hs'
      rw [← hs']
      exact invL_setWalk_chan _ pk _
        (preserveL_fire hwf pk hmem' hph2 hcm hi)
  next hcm => simp at hstep

/-- Count and occupancy delta of the base `walkFire` arm, for the muxed
`applyBase` dispatch: wire channels frame completely, and internal
channels keep conservation and capacity.

The muxed system disables wire fires as base actions (they route
through `push`), hence the `hnw` hypothesis: with it, the guard pins
the fired channel to one of `lowerOut`/`askedOut`/`upperOut` — never a
`Chan.wire` — so every wire channel frames on all three observables,
while the touched internal channel gains +1 occupancy (guard
`chan < 1`) against +1 `sentOf`, within its capacity 1. Wrapper over
`delta_fire`; extracted from the flow bullet of `preserve_walkFire`
(Proofs/Preserve/WalkFire.lean). -/
theorem delta_walkFire (hwf : sk.wellFormed = true) (pk : Party × Nat)
    (hnw : ∀ i, (s.walk pk).committed ≠ some (Oblig.wire i))
    (hstep : Model.apply sk ax (.walkFire pk) s = some s')
    (hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
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
      obtain ⟨htouch, hframe, hrecv⟩ := delta_fire hwf pk hmem' hph2 hcm hi
      -- bridge the bumped successor to the raw effect
      have hchan : s'.chan = bump s.chan (obligChan pk o) 1 := by
        rw [← hs']
        rfl
      have hsent : ∀ c, sentOf sk s' c
          = sentOf sk
              (setWalk s pk (normWalk sk pk.2 (fireOblig (s.walk pk) o)))
              c := by
        intro c
        rw [← hs']
        exact sentOf_setWalk_chan _ pk _ c
      have hrecv' : ∀ c, recvdOf sk s' c
          = recvdOf sk
              (setWalk s pk (normWalk sk pk.2 (fireOblig (s.walk pk) o)))
              c := by
        intro c
        rw [← hs']
        exact recvdOf_setWalk_chan _ pk _ c
      have honw : ∀ i, o ≠ Oblig.wire i := by
        intro i he
        exact hnw i (by rw [hcm, he])
      refine ⟨?_, ?_, ?_, ?_, ?_⟩
      · -- wire producer counts frame: the fired channel is internal
        intro p h
        rw [hsent]
        exact hframe _ (wire_ne_obligChan pk honw p h)
      · -- wire occupancy-plus-received frames
        intro p h
        rw [hchan, hrecv', hrecv,
          bump_ne _ _ (wire_ne_obligChan pk honw p h)]
      · -- wire occupancy never rises
        intro p h
        rw [hchan, bump_ne _ _ (wire_ne_obligChan pk honw p h)]
        exact Nat.le_refl _
      · -- internal conservation: +1 occupancy against +1 sent
        intro c _ _ hflow
        by_cases hce : c = obligChan pk o
        · subst hce
          rw [hchan, bump_one, hsent, htouch, hrecv', hrecv]
          omega
        · rw [hchan, bump_ne _ _ hce, hsent, hframe c hce, hrecv', hrecv]
          exact hflow
      · -- internal capacity: the guard empties the fired channel first
        intro c _ _ hcap
        by_cases hce : c = obligChan pk o
        · subst hce
          rw [hchan, bump_one, cap_obligChan]
          omega
        · rw [hchan, bump_ne _ _ hce]
          exact hcap
  next hcm => simp at hstep

end StreamingMirror.Mux
