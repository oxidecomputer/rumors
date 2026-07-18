/-
Weave pump-progress, the state layer (PROGRESS.md §7 3b, edge-respect
step (a) of the pump case-tree): what any weave state says about one
trace's cell — the projection collapse to the owner's emitted prefix,
the head-seq law (a cell head's seq IS its channel-side count), the
`rcvd ≤ sent` invariant, and future freshness.

# Shape

Everything here reads one `(trace, cell)` pair of the glued family
(`wcount_glue`) through canonical projections: the trace's projection
is canon (`procs_canon`), the emitted prefix's projection is a canon
prefix, and — because each channel-side has ONE owner — `out`'s whole
projection IS the owner's prefix's (`out_proj_owner`). Consequences:

- `cell_head_seq`: if a cell's head sits on its owner's channel-side,
  its seq equals the current count. This is the law both the manual
  emissions and the pump stuck-analysis consult: every pointer the
  case-tree compares is a seq read off a head.
- `fut_not_out`: future events are unemitted (their seqs sit at or
  past the count) — the upper bounds the case-tree's ascent needs.
- `wedge_rcvd_le_sent`: consumption never outruns production, from
  guard-history plus canon.
-/
import StreamingMirror.Proofs.Sched.Weave.Edge
import StreamingMirror.Proofs.Sched.Weave.Prec

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ================================================= positional access

/-- Positional read of a pointwise relation. -/
theorem Forall2.rel_of_getElem? {α β : Type _} {R : α → β → Prop} :
    ∀ {la : List α} {lb : List β}, Forall2 R la lb →
      ∀ {i : Nat} {a : α} {b : β},
        la[i]? = some a → lb[i]? = some b → R a b
  | _, _, .nil, i, _, _, ha, _ => by simp at ha
  | _, _, .cons hab t, i, a, b, ha, hb => by
      match i with
      | 0 =>
          simp only [List.getElem?_cons_zero, Option.some.injEq] at ha hb
          subst ha
          subst hb
          exact hab
      | i + 1 =>
          simp only [List.getElem?_cons_succ] at ha hb
          exact t.rel_of_getElem? ha hb

/-- A left read always has a related right read. -/
theorem Forall2.exists_rel_right {α β : Type _} {R : α → β → Prop} :
    ∀ {la : List α} {lb : List β}, Forall2 R la lb →
      ∀ {i : Nat} {a : α}, la[i]? = some a →
        ∃ b, lb[i]? = some b ∧ R a b
  | _, _, .nil, i, _, ha => by simp at ha
  | _, _, .cons hab t, i, a, ha => by
      match i with
      | 0 =>
          simp only [List.getElem?_cons_zero, Option.some.injEq] at ha
          subst ha
          exact ⟨_, rfl, hab⟩
      | i + 1 =>
          simp only [List.getElem?_cons_succ] at ha
          obtain ⟨b, hb, hr⟩ := t.exists_rel_right ha
          exact ⟨b, by simpa using hb, hr⟩

/-- Positional read of a canon stream. -/
theorem canon_getElem? (c : Chan) (b : Bool) {m i : Nat} (hi : i < m) :
    (canon c b m)[i]? = some (c, b, i) := by
  unfold canon
  rw [List.getElem?_map, List.getElem?_range hi]
  rfl

-- ======================================= the owner-projection collapse

/-- Only the owner's pair feeds a channel-side: the emitted-prefix
counts of every other pair vanish, so the family total is the owner's
prefix projection. -/
private theorem emittedCount_owner {c : Chan} {b : Bool} {out : List Ev}
    {f : Chan → Nat} :
    ∀ {i : Nat} {ts rs : List (List Ev)},
      Forall2 (fun t r => ∃ pre, t = pre ++ r ∧ pre.Sublist out) ts rs →
      Owned f b i ts →
      ∀ {j : Nat} {T r : List Ev}, ts[j]? = some T → rs[j]? = some r →
        f c = i + j →
        ∀ {pre : List Ev}, T = pre ++ r →
        emittedCount (fun e => decide (e.1 = c) && (e.2.1 == b)) ts rs
          = (proj c b pre).length
  | _, _, _, .nil, _, _, _, _, hT, _, _, _, _ => by simp at hT
  | i, _, _, .cons (a := t₀) (la := ts) (b := r₀) (lb := rs)
      ⟨pre₀, hpre₀, hsub₀⟩ htail, hown, j, T, r, hT, hr, hfc, pre,
      hpre => by
      have hcount : emittedCount
          (fun e => decide (e.1 = c) && (e.2.1 == b))
          (t₀ :: ts) (r₀ :: rs)
          = (proj c b (t₀.take (t₀.length - r₀.length))).length
            + emittedCount (fun e => decide (e.1 = c) && (e.2.1 == b))
                ts rs := rfl
      have hpretake : t₀.take (t₀.length - r₀.length) = pre₀ := by
        subst hpre₀
        have hlen : (pre₀ ++ r₀).length - r₀.length = pre₀.length := by
          simp
        rw [hlen, List.take_left]
      match j with
      | 0 =>
          simp only [List.getElem?_cons_zero, Option.some.injEq] at hT hr
          subst hT
          subst hr
          -- the owner heads the family: the tail is silent
          have htail_nil : ∀ t' ∈ ts, proj c b t' = [] := by
            intro t' ht'
            cases hq : proj c b t' with
            | nil => rfl
            | cons e' rest' =>
                have hemem' : e' ∈ proj c b t' := by
                  rw [hq]; exact List.mem_cons_self ..
                have hin' := List.mem_filter.1 hemem'
                simp only [Bool.and_eq_true, decide_eq_true_eq,
                  beq_iff_eq] at hin'
                have hge := owned_ge hown.2 t' ht' e' hin'.1 hin'.2.2
                rw [hin'.2.1, hfc] at hge
                omega
          have hpp : pre₀ = pre := by
            have := hpre₀.symm.trans hpre
            exact List.append_cancel_right this
          rw [hcount, hpretake, hpp,
            emitted_nil htail htail_nil]
          omega
      | j + 1 =>
          simp only [List.getElem?_cons_succ] at hT hr
          -- the head is silent on this channel-side
          have hhead_nil : proj c b t₀ = [] := by
            cases hq : proj c b t₀ with
            | nil => rfl
            | cons e' rest' =>
                have hemem' : e' ∈ proj c b t₀ := by
                  rw [hq]; exact List.mem_cons_self ..
                have hin' := List.mem_filter.1 hemem'
                simp only [Bool.and_eq_true, decide_eq_true_eq,
                  beq_iff_eq] at hin'
                have h0 := hown.1 e' hin'.1 hin'.2.2
                rw [hin'.2.1, hfc] at h0
                omega
          have hpre_nil : proj c b pre₀ = [] := by
            rw [hpre₀, proj_append, List.append_eq_nil_iff] at hhead_nil
            exact hhead_nil.1
          rw [hcount, hpretake, hpre_nil,
            emittedCount_owner htail hown.2 hT hr
              (by rw [hfc]; omega) hpre]
          simp

/-- THE COLLAPSE: `out`'s projection on an owned channel-side is its
owner's emitted prefix's projection — nothing else feeds it. -/
theorem out_proj_owner (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) (c : Chan) (b : Bool)
    {M : Nat} (hM : (if b then sndOwner sk c else rcvOwner sk c) = M)
    {T r pre : List Ev}
    (hT : (procs sk)[M]? = some T)
    (hr : (manFilters sk fut ++ st.rem)[M]? = some r)
    (hpre : T = pre ++ r) (hsub : pre.Sublist st.out) :
    proj c b st.out = proj c b pre := by
  have howned : Owned (if b then sndOwner sk else rcvOwner sk) b 0
      (procs sk) := by
    cases b
    · exact procs_rcv_owned sk hwf
    · exact procs_snd_owned sk hwf
  have hEC := emittedCount_owner (out := st.out)
    (wcount_glue sk h) howned hT hr
    (by cases b <;> simpa using hM) hpre
  have hlen : (proj c b st.out).length = (proj c b pre).length := by
    show (st.out.filter _).length = _
    rw [wcount_out_glued sk h _, hEC]
  exact ((hsub.filter _).eq_of_length
    (by
      show (proj c b pre).length = (proj c b st.out).length
      rw [hlen])).symm

-- ============================================== the two seq laws

/-- The cell decomposition at an owned index: the trace, its emitted
prefix, and its unemitted cell, with the collapse pre-applied. -/
theorem cell_of_owner {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {M : Nat}
    {T : List Ev} (hT : (procs sk)[M]? = some T) :
    ∃ r pre, (manFilters sk fut ++ st.rem)[M]? = some r
      ∧ T = pre ++ r ∧ pre.Sublist st.out := by
  obtain ⟨r, hr, pre, hpre, hsub⟩ :=
    (wcount_glue sk h).exists_rel_right hT
  exact ⟨r, pre, hr, hpre, hsub⟩

/-- THE HEAD-SEQ LAW: when a cell's head sits on a channel-side its
trace owns, the head's seq is exactly the current count. -/
theorem cell_head_seq (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) (c : Chan) (b : Bool)
    {M : Nat} (hM : (if b then sndOwner sk c else rcvOwner sk c) = M)
    {T r pre : List Ev} {n : Nat} {rest : List Ev}
    (hT : (procs sk)[M]? = some T)
    (hr : (manFilters sk fut ++ st.rem)[M]? = some r)
    (hpre : T = pre ++ r) (hsub : pre.Sublist st.out)
    (hhead : r = (c, b, n) :: rest) :
    n = (proj c b st.out).length := by
  have hTmem : T ∈ procs sk := List.mem_of_getElem? hT
  obtain ⟨m, hcanon⟩ := procs_canon sk c b T hTmem
  have hsplit : proj c b pre ++ proj c b r = canon c b m := by
    rw [← proj_append, ← hpre, hcanon]
  have hrhead : proj c b r = (c, b, n) :: proj c b rest := by
    rw [hhead]
    unfold proj
    rw [List.filter_cons_of_pos (by simp)]
  have hpos : (canon c b m)[(proj c b pre).length]?
      = some (c, b, n) := by
    rw [← hsplit, List.getElem?_append_right (Nat.le_refl _),
      Nat.sub_self, hrhead]
    rfl
  have hlt : (proj c b pre).length < m := by
    by_contra hge
    rw [show (canon c b m)[(proj c b pre).length]? = none from by
        apply List.getElem?_eq_none
        simp [canon]
        omega] at hpos
    cases hpos
  rw [canon_getElem? c b hlt] at hpos
  have hn : n = (proj c b pre).length := by
    have := congrArg (fun o : Option Ev =>
      (o.getD (c, b, 0)).2.2) hpos
    simpa using this.symm
  rw [hn, out_proj_owner sk hwf h c b hM hT hr hpre hsub]

/-- FRESHNESS: an event still in a cell is unemitted — its seq sits at
or past the current count, and the emitted stream is canonical. -/
theorem cell_not_out (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) (c : Chan) (b : Bool)
    {M : Nat} (hM : (if b then sndOwner sk c else rcvOwner sk c) = M)
    {T r pre : List Ev} {n : Nat}
    (hT : (procs sk)[M]? = some T)
    (hr : (manFilters sk fut ++ st.rem)[M]? = some r)
    (hpre : T = pre ++ r) (hsub : pre.Sublist st.out)
    (hmem : ((c, b, n) : Ev) ∈ r) :
    (proj c b st.out).length ≤ n := by
  have hTmem : T ∈ procs sk := List.mem_of_getElem? hT
  obtain ⟨m, hcanon⟩ := procs_canon sk c b T hTmem
  have hsplit : proj c b pre ++ proj c b r = canon c b m := by
    rw [← proj_append, ← hpre, hcanon]
  have hmemp : ((c, b, n) : Ev) ∈ proj c b r :=
    List.mem_filter.2 ⟨hmem, by simp⟩
  obtain ⟨t, ht⟩ := List.mem_iff_getElem?.1 hmemp
  have hread : (canon c b m)[(proj c b pre).length + t]?
      = some (c, b, n) := by
    rw [← hsplit, List.getElem?_append_right (by omega)]
    rw [show (proj c b pre).length + t - (proj c b pre).length
      = t from by omega]
    exact ht
  have hlt : (proj c b pre).length + t < m := by
    by_contra hge
    rw [show (canon c b m)[(proj c b pre).length + t]? = none from by
        apply List.getElem?_eq_none
        simp [canon]
        omega] at hread
    cases hread
  rw [canon_getElem? c b hlt] at hread
  have hn : n = (proj c b pre).length + t := by
    have := congrArg (fun o : Option Ev =>
      (o.getD (c, b, 0)).2.2) hread
    simpa using this.symm
  rw [out_proj_owner sk hwf h c b hM hT hr hpre hsub]
  omega

-- ================================================== rcvd never leads

/-- A prefix's count never exceeds the whole's. -/
private theorem sndCount_take_le (c : Chan) (l : List Ev) (k : Nat) :
    sndCount c (l.take k) ≤ sndCount c l := by
  rw [sndCount_eq_proj, sndCount_eq_proj]
  exact ((List.take_sublist k l).filter _).length_le

/-- Consumption never outruns production: from guard-history plus
canonical receives. -/
theorem wedge_rcvd_le_sent (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WEdge sk fut st) (c : Chan) :
    rcvCount c st.out ≤ sndCount c st.out := by
  cases hz : rcvCount c st.out with
  | zero => omega
  | succ q =>
      -- the top receive is at seq q; its guard held over a prefix
      have hcanon := wproj_canon sk hwf h.toWCountP c false
      have hmem : ((c, false, q) : Ev) ∈ proj c false st.out := by
        rw [hcanon]
        have hlen : (proj c false st.out).length = q + 1 := by
          rw [← rcvCount_eq_proj, hz]
        rw [hlen]
        unfold canon
        exact List.mem_map.2 ⟨q, List.mem_range.2 (by omega), rfl⟩
      have hmem' : ((c, false, q) : Ev) ∈ st.out :=
        (List.mem_filter.1 hmem).1
      obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hmem'
      have hguard := h.e1_hist k c q hk
      have := sndCount_take_le c st.out k
      omega

-- ============================================ pump positional reads
-- Where each pump trace sits in `procs`: the indices the channel
-- owner functions point at.

private theorem procs_len_prefix :
    ([iopenEvents sk, ropenEvents sk]
      ++ ((List.range sk.rootH).map fun i =>
        ((if (sk.rootH - 1 - i) % 2 == 1 then Party.I else Party.R,
          sk.rootH - 1 - i) : Party × Nat)).map
          (walkEvents sk)).length = 2 + sk.rootH := by
  simp
  omega

/-- Absorb sits at slot `2 + rootH`. -/
theorem procs_absorb :
    (procs sk)[2 + sk.rootH]? = some (absorbEvents sk) := by
  unfold procs
  rw [List.getElem?_append_left (by
      simp [Skel.asmKeys]
      omega),
    List.getElem?_append_left (by simp; omega),
    List.getElem?_append_right (by simp; omega)]
  simp
  rw [show 2 + sk.rootH - (sk.rootH + 1 + 1) = 0 from by omega]
  rfl

private theorem asmKeys_I {j : Nat} (h1 : 1 ≤ j) (hj : j ≤ sk.rootH) :
    sk.asmKeys[j - 1]? = some (Party.I, j) := by
  unfold Skel.asmKeys
  rw [List.getElem?_append_left (by simp; omega),
    List.getElem?_map, List.getElem?_range (by omega)]
  simp
  omega

private theorem asmKeys_R {j : Nat} (h1 : 1 ≤ j)
    (hj : j ≤ sk.rootH - 1) :
    sk.asmKeys[sk.rootH + j - 1]? = some (Party.R, j) := by
  unfold Skel.asmKeys
  rw [List.getElem?_append_right (by simp; omega),
    List.getElem?_map, List.getElem?_range (by simp; omega)]
  simp
  omega

private theorem procs_asm_at {q : Nat} {pk : Party × Nat}
    (hq : sk.asmKeys[q]? = some pk) :
    (procs sk)[3 + sk.rootH + q]? = some (asmEvents sk pk) := by
  have hqlen : q < sk.asmKeys.length := by
    by_contra hge
    rw [List.getElem?_eq_none (by omega)] at hq
    cases hq
  unfold procs
  rw [List.getElem?_append_left (by
      simp [Skel.asmKeys] at hqlen ⊢
      omega),
    List.getElem?_append_right (by simp; omega)]
  have hidx : 3 + sk.rootH + q
      - (([iopenEvents sk, ropenEvents sk]
        ++ ((List.range sk.rootH).map fun i =>
          ((if (sk.rootH - 1 - i) % 2 == 1 then Party.I else Party.R,
            sk.rootH - 1 - i) : Party × Nat)).map (walkEvents sk)
        ++ [absorbEvents sk]).length) = q := by
    simp
    omega
  rw [hidx, List.getElem?_map, hq]
  rfl

/-- The initiator tower at height `j` sits at slot `asmIdx I j`. -/
theorem procs_asmI {j : Nat} (h1 : 1 ≤ j) (hj : j ≤ sk.rootH) :
    (procs sk)[asmIdx sk Party.I j]? = some (asmEvents sk (Party.I, j)) := by
  have h := procs_asm_at sk (asmKeys_I sk h1 hj)
  have hidx : asmIdx sk Party.I j = 3 + sk.rootH + (j - 1) := rfl
  rw [hidx]
  exact h

/-- The responder tower at height `j` sits at slot `asmIdx R j`. -/
theorem procs_asmR {j : Nat} (h1 : 1 ≤ j) (hj : j ≤ sk.rootH - 1) :
    (procs sk)[asmIdx sk Party.R j]? = some (asmEvents sk (Party.R, j)) := by
  have h := procs_asm_at sk (asmKeys_R sk h1 hj)
  have hidx : asmIdx sk Party.R j = 3 + sk.rootH + (sk.rootH + j - 1) := by
    show 3 + 2 * sk.rootH + (j - 1) = _
    omega
  rw [hidx]
  exact h

/-- The floating `rootret` receive sits at slot `3·rootH + 2`. -/
theorem procs_rootret (hge : 1 ≤ sk.rootH) :
    (procs sk)[3 * sk.rootH + 2]? = some [(Chan.rootret, false, 0)] := by
  unfold procs
  rw [List.getElem?_append_right (by
    simp [Skel.asmKeys]
    omega)]
  have hidx : 3 * sk.rootH + 2
      - (([iopenEvents sk, ropenEvents sk]
        ++ ((List.range sk.rootH).map fun i =>
          ((if (sk.rootH - 1 - i) % 2 == 1 then Party.I else Party.R,
            sk.rootH - 1 - i) : Party × Nat)).map (walkEvents sk)
        ++ [absorbEvents sk]
        ++ sk.asmKeys.map (asmEvents sk)).length) = 0 := by
    simp [Skel.asmKeys]
    omega
  rw [hidx]
  rfl

/-- A nonempty suffix of a block run splits at a block: full blocks,
then a split block whose right part heads the suffix. -/
theorem prefix_flatMap {α : Type _} {f : Nat → List α} :
    ∀ (m a : Nat) {pre r : List α},
      (List.range' a m).flatMap f = pre ++ r → r ≠ [] →
      ∃ t, a ≤ t ∧ t < a + m ∧ ∃ p₂ r₂,
        f t = p₂ ++ r₂ ∧ r₂ ≠ []
        ∧ pre = (List.range' a (t - a)).flatMap f ++ p₂
        ∧ r = r₂ ++ (List.range' (t + 1) (a + m - t - 1)).flatMap f
  | 0, a, pre, r, hsplit, hne => by
      simp only [List.range'_zero, List.flatMap_nil] at hsplit
      obtain ⟨-, hr⟩ := List.append_eq_nil_iff.1 hsplit.symm
      exact absurd hr hne
  | m + 1, a, pre, r, hsplit, hne => by
      rw [List.range'_succ, List.flatMap_cons] at hsplit
      rcases List.append_eq_append_iff.1 hsplit with
        ⟨a', hpre, hrest⟩ | ⟨c', hfa, hr⟩
      · -- the prefix swallows block `a`: recurse into the tail run
        obtain ⟨t, hat, htm, p₂, r₂, hft, hr₂, hpre', hr'⟩ :=
          prefix_flatMap m (a + 1) hrest hne
        refine ⟨t, by omega, by omega, p₂, r₂, hft, hr₂, ?_, ?_⟩
        · rw [hpre, hpre',
            show t - a = (t - (a + 1)) + 1 from by omega]
          rw [List.range'_succ, List.flatMap_cons, List.append_assoc]
        · rw [hr', show a + (m + 1) - t - 1 = a + 1 + m - t - 1
            from by omega]
      · -- the suffix starts inside block `a`
        cases c' with
        | nil =>
            -- boundary: block `a` is exactly the prefix; recurse
            rw [List.append_nil] at hfa
            rw [List.nil_append] at hr
            obtain ⟨t, hat, htm, p₂, r₂, hft, hr₂, hpre', hr'⟩ :=
              prefix_flatMap m (a + 1) (pre := [])
                (by simpa using hr.symm) hne
            refine ⟨t, by omega, by omega, p₂, r₂, hft, hr₂, ?_, ?_⟩
            · rw [show t - a = (t - (a + 1)) + 1 from by omega,
                List.range'_succ, List.flatMap_cons, ← hfa,
                List.append_assoc, ← hpre', List.append_nil]
            · rw [hr', show a + (m + 1) - t - 1 = a + 1 + m - t - 1
                from by omega]
        | cons x c'' =>
            refine ⟨a, Nat.le_refl a, by omega, pre, x :: c'',
              hfa, by simp, ?_, ?_⟩
            · rw [Nat.sub_self]
              rfl
            · rw [hr, show a + (m + 1) - a - 1 = m from by omega]

private theorem seg_take (c : Chan) (b : Bool) (lo n m : Nat) :
    (seg c b lo n).take m = seg c b lo (min m n) := by
  unfold seg
  rw [← List.map_take, List.take_range]

theorem seg_len (c : Chan) (b : Bool) (lo n : Nat) :
    (seg c b lo n).length = n := by
  simp [seg]

-- ============================================= asm block-run counts

private theorem proj_run_res (pk : Party × Nat) :
    ∀ (m a : Nat),
      proj (asmResChan pk) false
        ((List.range' a m).flatMap (asmBlock sk pk))
      = seg (asmResChan pk) false a m
  | 0, a => rfl
  | m + 1, a => by
      rw [List.range'_succ, List.flatMap_cons, proj_append,
        proj_asmBlock_res, proj_run_res pk m (a + 1),
        seg_append, Nat.add_comm 1 m]

private theorem proj_run_out (pk : Party × Nat) :
    ∀ (m a : Nat),
      proj (sk.asmOutChan pk) true
        ((List.range' a m).flatMap (asmBlock sk pk))
      = seg (sk.asmOutChan pk) true a m
  | 0, a => rfl
  | m + 1, a => by
      rw [List.range'_succ, List.flatMap_cons, proj_append,
        proj_asmBlock_out, proj_run_out pk m (a + 1),
        seg_append, Nat.add_comm 1 m]

private theorem foldl_add_init_le : ∀ (l : List Nat) (acc : Nat),
    acc ≤ l.foldl (· + ·) acc
  | [], _ => Nat.le_refl _
  | x :: l, acc => by
      have := foldl_add_init_le l (acc + x)
      simp only [List.foldl_cons]
      omega

/-- Pending prefix sums are monotone in the cursor. -/
theorem pendsBefore_mono (p : Party) (j : Nat) {k k' : Nat}
    (hkk : k ≤ k') :
    sk.pendsBefore p j k ≤ sk.pendsBefore p j k' := by
  unfold Skel.pendsBefore
  rw [show k' = k + (k' - k) from by omega, List.take_add,
    List.foldl_append]
  exact foldl_add_init_le _ _

private theorem proj_run_level (pk : Party × Nat) :
    ∀ (m a : Nat),
      a + m ≤ (sk.asmResList pk.1 pk.2).length →
      proj (asmLevelChan pk) false
        ((List.range' a m).flatMap (asmBlock sk pk))
      = seg (asmLevelChan pk) false (sk.pendsBefore pk.1 pk.2 a)
          (sk.pendsBefore pk.1 pk.2 (a + m)
            - sk.pendsBefore pk.1 pk.2 a)
  | 0, a, _ => by
      rw [Nat.add_zero, Nat.sub_self]
      rfl
  | m + 1, a, h => by
      have hsucc := pendsBefore_succ sk
        (p := pk.1) (j := pk.2) (k := a) (by omega)
      have hmono := pendsBefore_mono sk pk.1 pk.2
        (show a + 1 ≤ a + 1 + m by omega)
      rw [show a + (m + 1) = a + 1 + m from by omega,
        List.range'_succ, List.flatMap_cons, proj_append,
        proj_asmBlock_level, proj_run_level pk m (a + 1) (by omega),
        show sk.pendsBefore pk.1 pk.2 (a + 1)
          = sk.pendsBefore pk.1 pk.2 a + sk.pendAt pk.1 pk.2 a
          from by omega,
        seg_append]
      congr 1
      omega

/-- THE ASM SUFFIX TRICHOTOMY: a nonempty unemitted cell of an asm
trace heads at its next resolution, mid-pends, or its next out — and
the block position pins all three channel-side counts of the emitted
prefix. -/
theorem asm_cell_shape (pk : Party × Nat) {pre r : List Ev}
    (hsplit : asmEvents sk pk = pre ++ r) (hne : r ≠ []) :
    ∃ idx, idx < (sk.asmResList pk.1 pk.2).length ∧
      (((∃ rest, r = (asmResChan pk, false, idx) :: rest)
        ∧ (proj (asmResChan pk) false pre).length = idx
        ∧ (proj (asmLevelChan pk) false pre).length
            = sk.pendsBefore pk.1 pk.2 idx
        ∧ (proj (sk.asmOutChan pk) true pre).length = idx)
      ∨ (∃ tlv rest, r = (asmLevelChan pk, false, tlv) :: rest
        ∧ sk.pendsBefore pk.1 pk.2 idx ≤ tlv
        ∧ tlv < sk.pendsBefore pk.1 pk.2 (idx + 1)
        ∧ (proj (asmResChan pk) false pre).length = idx + 1
        ∧ (proj (asmLevelChan pk) false pre).length = tlv
        ∧ (proj (sk.asmOutChan pk) true pre).length = idx)
      ∨ ((∃ rest, r = (sk.asmOutChan pk, true, idx) :: rest)
        ∧ (proj (asmResChan pk) false pre).length = idx + 1
        ∧ (proj (asmLevelChan pk) false pre).length
            = sk.pendsBefore pk.1 pk.2 (idx + 1)
        ∧ (proj (sk.asmOutChan pk) true pre).length = idx)) := by
  unfold asmEvents at hsplit
  rw [List.range_eq_range'] at hsplit
  obtain ⟨t, -, htN, p₂, r₂, hblock, hr₂, hpre, hr⟩ :=
    prefix_flatMap _ 0 hsplit hne
  rw [Nat.zero_add] at htN
  rw [Nat.sub_zero] at hpre
  have hN := htN
  -- prefix projections: the closed block run plus the split block part
  have hres_run := proj_run_res sk pk t 0
  have hout_run := proj_run_out sk pk t 0
  have hlvl_run := proj_run_level sk pk t 0 (by omega)
  rw [Nat.zero_add] at hlvl_run
  have hp0 : sk.pendsBefore pk.1 pk.2 0 = 0 := rfl
  rw [hp0, Nat.sub_zero] at hlvl_run
  have hres_pre : proj (asmResChan pk) false pre
      = seg (asmResChan pk) false 0 t ++ proj (asmResChan pk) false p₂ := by
    rw [hpre, proj_append, hres_run]
  have hout_pre : proj (sk.asmOutChan pk) true pre
      = seg (sk.asmOutChan pk) true 0 t
        ++ proj (sk.asmOutChan pk) true p₂ := by
    rw [hpre, proj_append, hout_run]
  have hlvl_pre : proj (asmLevelChan pk) false pre
      = seg (asmLevelChan pk) false 0 (sk.pendsBefore pk.1 pk.2 t)
        ++ proj (asmLevelChan pk) false p₂ := by
    rw [hpre, proj_append, hlvl_run]
  rw [asmBlock_eq] at hblock
  refine ⟨t, htN, ?_⟩
  match p₂, hblock with
  | [], hblock =>
      -- block boundary: the cell heads at the next resolution
      rw [List.nil_append] at hblock
      refine Or.inl ⟨⟨(seg (asmLevelChan pk) false
          (sk.pendsBefore pk.1 pk.2 t) (sk.pendAt pk.1 pk.2 t)
          ++ [(sk.asmOutChan pk, true, t)])
          ++ (List.range' (t + 1)
              (0 + (sk.asmResList pk.1 pk.2).length - t - 1)).flatMap
              (asmBlock sk pk), ?_⟩, ?_, ?_, ?_⟩
      · rw [hr, ← hblock]
        rfl
      · rw [hres_pre]
        simp [seg_len, proj_nil]
      · rw [hlvl_pre]
        simp [seg_len, proj_nil]
      · rw [hout_pre]
        simp [seg_len, proj_nil]
  | e :: p₃, hblock =>
      rw [List.cons_append] at hblock
      injection hblock with he1 hinner
      subst he1
      have hsucc := pendsBefore_succ sk
        (p := pk.1) (j := pk.2) (k := t) (by omega)
      rcases List.append_eq_append_iff.1 hinner with
        ⟨a', hseg, hra⟩ | ⟨c', hp₃, hout⟩
      · -- `[out] = a' ++ r₂`: the out heads the cell (a' must be nil)
        cases a' with
        | nil =>
            rw [List.append_nil] at hseg
            rw [List.nil_append] at hra
            refine Or.inr (Or.inr ⟨⟨(List.range' (t + 1)
                (0 + (sk.asmResList pk.1 pk.2).length - t - 1)).flatMap
                (asmBlock sk pk), ?_⟩, ?_, ?_, ?_⟩)
            · rw [hr, ← hra]
              rfl
            · rw [hres_pre]
              simp only [proj_cons_self, List.length_append, seg_len]
              rw [hseg, proj_seg_ne (fun hh => res_ne_level pk hh.1.symm)]
              simp
            · rw [hlvl_pre]
              simp only [List.length_append, seg_len]
              rw [proj_cons_ne_chan (res_ne_level pk), hseg,
                proj_seg_self, hsucc, seg_len]
            · rw [hout_pre]
              simp only [List.length_append, seg_len]
              rw [proj_cons_ne_side (by simp), hseg,
                proj_seg_ne (by simp)]
              simp
        | cons x a'' =>
            exfalso
            have hlen := congrArg List.length hra
            simp at hlen
            cases r₂ with
            | nil => exact hr₂ rfl
            | cons z r₃ => simp at hlen
      · -- `seg = p₃ ++ c'`: mid-pends when `c'` is inhabited
        cases c' with
        | nil =>
            rw [List.append_nil] at hp₃
            rw [List.nil_append] at hout
            refine Or.inr (Or.inr ⟨⟨(List.range' (t + 1)
                (0 + (sk.asmResList pk.1 pk.2).length - t - 1)).flatMap
                (asmBlock sk pk), ?_⟩, ?_, ?_, ?_⟩)
            · rw [hr, hout]
              rfl
            · rw [hres_pre]
              simp only [proj_cons_self, List.length_append, seg_len]
              rw [← hp₃, proj_seg_ne (fun hh => res_ne_level pk hh.1.symm)]
              simp
            · rw [hlvl_pre]
              simp only [List.length_append, seg_len]
              rw [proj_cons_ne_chan (res_ne_level pk), ← hp₃,
                proj_seg_self, hsucc, seg_len]
            · rw [hout_pre]
              simp only [List.length_append, seg_len]
              rw [proj_cons_ne_side (by simp), ← hp₃,
                proj_seg_ne (by simp)]
              simp
        | cons x c'' =>
            -- mid-pends: the cell heads at the next level return
            have hp3take : p₃ = seg (asmLevelChan pk) false
                (sk.pendsBefore pk.1 pk.2 t)
                (min p₃.length (sk.pendAt pk.1 pk.2 t)) := by
              have hthis := congrArg (List.take p₃.length) hp₃
              rw [List.take_append_of_le_length (Nat.le_refl _),
                List.take_of_length_le (Nat.le_refl _)] at hthis
              rw [← seg_take]
              exact hthis.symm
            have hlen3 : p₃.length < sk.pendAt pk.1 pk.2 t := by
              have hthis := congrArg List.length hp₃
              simp [seg_len] at hthis
              omega
            have hmin : min p₃.length (sk.pendAt pk.1 pk.2 t)
                = p₃.length := by omega
            rw [hmin] at hp3take
            have hx : x = (asmLevelChan pk, false,
                sk.pendsBefore pk.1 pk.2 t + p₃.length) := by
              have hread : (seg (asmLevelChan pk) false
                  (sk.pendsBefore pk.1 pk.2 t)
                  (sk.pendAt pk.1 pk.2 t))[p₃.length]? = some x := by
                rw [hp₃, List.getElem?_append_right (Nat.le_refl _),
                  Nat.sub_self]
                rfl
              rw [seg_getElem? _ _ _ _ _ hlen3] at hread
              simp only [Option.some.injEq] at hread
              exact hread.symm
            refine Or.inr (Or.inl ⟨sk.pendsBefore pk.1 pk.2 t
              + p₃.length, (c'' ++ [(sk.asmOutChan pk, true, t)])
                ++ (List.range' (t + 1)
                  (0 + (sk.asmResList pk.1 pk.2).length - t - 1)).flatMap
                  (asmBlock sk pk), ?_, by omega, ?_, ?_, ?_, ?_⟩)
            · rw [hr, hout, hx]
              rfl
            · rw [hsucc]
              omega
            · rw [hres_pre]
              simp only [proj_cons_self, List.length_append, seg_len]
              rw [hp3take, proj_seg_ne (fun hh => res_ne_level pk hh.1.symm)]
              simp
            · rw [hlvl_pre]
              simp only [List.length_append, seg_len]
              rw [proj_cons_ne_chan (res_ne_level pk), hp3take,
                proj_seg_self, seg_len]
            · rw [hout_pre]
              simp only [List.length_append, seg_len]
              rw [proj_cons_ne_side (by simp), hp3take,
                proj_seg_ne (by simp)]
              simp

/-- fins sit at slot `3·rootH + 3`. -/
theorem procs_fin (hge : 1 ≤ sk.rootH) :
    (procs sk)[3 * sk.rootH + 3]? = some (finEvents sk) := by
  unfold procs
  rw [List.getElem?_append_right (by
    simp [Skel.asmKeys]
    omega)]
  have hidx : 3 * sk.rootH + 3
      - (([iopenEvents sk, ropenEvents sk]
        ++ ((List.range sk.rootH).map fun i =>
          ((if (sk.rootH - 1 - i) % 2 == 1 then Party.I else Party.R,
            sk.rootH - 1 - i) : Party × Nat)).map (walkEvents sk)
        ++ [absorbEvents sk]
        ++ sk.asmKeys.map (asmEvents sk)).length) = 1 := by
    simp [Skel.asmKeys]
    omega
  rw [hidx]
  rfl

-- ================================================ fixpoint stuckness

/-- A failed scan means every cell's head is disabled. -/
theorem scan_none_heads {sent rcvd : Chan → Nat} :
    ∀ {ts : List (List Ev)}, scan sk sent rcvd ts = none →
      ∀ {i : Nat} {e : Ev} {rest : List Ev},
        ts[i]? = some (e :: rest) →
        enabled sk sent rcvd e = false := by
  intro ts
  induction ts with
  | nil => intro _ i e rest hi; simp at hi
  | cons t ts ih =>
      intro hscan i e rest hi
      match t, i with
      | [], 0 => simp at hi
      | [], i + 1 =>
          rw [scan] at hscan
          cases hrec : scan sk sent rcvd ts with
          | none =>
              simp only [List.getElem?_cons_succ] at hi
              exact ih hrec hi
          | some pr => rw [hrec] at hscan; simp at hscan
      | e₀ :: rest₀, 0 =>
          simp only [List.getElem?_cons_zero, Option.some.injEq] at hi
          cases hen : enabled sk sent rcvd e₀ with
          | false =>
              have : e = e₀ := by
                have := congrArg (fun l : List Ev => l[0]?) hi
                simpa using this.symm
              rw [this]
              exact hen
          | true => rw [scan, if_pos hen] at hscan; cases hscan
      | e₀ :: rest₀, i + 1 =>
          cases hen : enabled sk sent rcvd e₀ with
          | false =>
              rw [scan, if_neg (by rw [hen]; simp)] at hscan
              cases hrec : scan sk sent rcvd ts with
              | none =>
                  simp only [List.getElem?_cons_succ] at hi
                  exact ih hrec hi
              | some pr => rw [hrec] at hscan; simp at hscan
          | true => rw [scan, if_pos hen] at hscan; cases hscan

/-- The manual filter family has one cell per manual trace. -/
private theorem manFilters_length (fut : List Ev) :
    (manFilters sk fut).length = manCount sk := by
  simp [manFilters]

/-- Every asm slot sits past the manual prefix. -/
private theorem asmIdx_ge (p : Party) {j : Nat} (h1 : 1 ≤ j) :
    manCount sk < asmIdx sk p j := by
  cases p <;> (show manCount sk < 3 + _ + (j - 1); unfold manCount) <;>
    omega

/-- The asm channels' owners all point at the tower's slot. -/
theorem asm_owners (p : Party) {j : Nat} (h1 : 1 ≤ j) :
    rcvOwner sk (asmResChan (p, j)) = asmIdx sk p j
    ∧ rcvOwner sk (asmLevelChan (p, j)) = asmIdx sk p j
    ∧ sndOwner sk (sk.asmOutChan (p, j)) = asmIdx sk p j := by
  refine ⟨?_, ?_, ?_⟩
  · unfold asmResChan
    split
    · show rcvOwner sk (Chan.upper p ((p, j).2 - 1)) = _
      simp only [rcvOwner]
      rw [show ((p, j).2 - 1 + 1) = j from by omega]
    · rfl
  · show rcvOwner sk (Chan.level p ((p, j).2 - 1)) = _
    simp only [rcvOwner]
    rw [show ((p, j).2 - 1 + 1) = j from by omega]
  · unfold Skel.asmOutChan
    split
    · rename_i hc
      simp only [Bool.and_eq_true, beq_iff_eq] at hc
      obtain ⟨hp, hj⟩ := hc
      show sndOwner sk Chan.rootret = _
      simp only [sndOwner]
      rw [hp, hj]
    · split
      · rename_i hc
        simp only [Bool.and_eq_true, beq_iff_eq] at hc
        obtain ⟨hp, hj⟩ := hc
        show sndOwner sk Chan.rootrets = _
        simp only [sndOwner]
        rw [hp, hj]
      · show sndOwner sk (Chan.level p j) = _
        simp only [sndOwner]
        rw [if_neg (by omega)]

/-- The asm trace's whole-trace projections. -/
theorem asm_totals (pk : Party × Nat) :
    proj (asmResChan pk) false (asmEvents sk pk)
        = seg (asmResChan pk) false 0 (sk.asmResList pk.1 pk.2).length
    ∧ proj (asmLevelChan pk) false (asmEvents sk pk)
        = seg (asmLevelChan pk) false 0
            (sk.pendsBefore pk.1 pk.2 (sk.asmResList pk.1 pk.2).length)
    ∧ proj (sk.asmOutChan pk) true (asmEvents sk pk)
        = seg (sk.asmOutChan pk) true 0
            (sk.asmResList pk.1 pk.2).length := by
  unfold asmEvents
  rw [List.range_eq_range']
  refine ⟨proj_run_res sk pk _ 0, ?_, proj_run_out sk pk _ 0⟩
  have := proj_run_level sk pk (sk.asmResList pk.1 pk.2).length 0
    (by omega)
  rw [Nat.zero_add] at this
  rw [this, show sk.pendsBefore pk.1 pk.2 0 = 0 from rfl,
    Nat.sub_zero]

/-- THE STUCK TRICHOTOMY, in counts: at a pump fixpoint an asm tower
is exhausted, res-starved at a block boundary, level-starved
mid-window, or out-blocked — each with its three counts pinned and
the starving/blocking guard recorded. -/
theorem asm_stuck (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st)
    (hfix : step sk st = none) {p : Party} {j : Nat} (h1 : 1 ≤ j)
    (hIdx : (procs sk)[asmIdx sk p j]?
      = some (asmEvents sk (p, j))) :
    (rcvCount (asmResChan (p, j)) st.out
        = (sk.asmResList p j).length
      ∧ rcvCount (asmLevelChan (p, j)) st.out
          = sk.pendsBefore p j (sk.asmResList p j).length
      ∧ sndCount (sk.asmOutChan (p, j)) st.out
          = (sk.asmResList p j).length)
    ∨ (rcvCount (asmResChan (p, j)) st.out < (sk.asmResList p j).length
      ∧ rcvCount (asmLevelChan (p, j)) st.out
          = sk.pendsBefore p j (rcvCount (asmResChan (p, j)) st.out)
      ∧ sndCount (sk.asmOutChan (p, j)) st.out
          = rcvCount (asmResChan (p, j)) st.out
      ∧ sndCount (asmResChan (p, j)) st.out
          ≤ rcvCount (asmResChan (p, j)) st.out)
    ∨ (rcvCount (asmResChan (p, j)) st.out ≤ (sk.asmResList p j).length
      ∧ 1 ≤ rcvCount (asmResChan (p, j)) st.out
      ∧ sk.pendsBefore p j (rcvCount (asmResChan (p, j)) st.out - 1)
          ≤ rcvCount (asmLevelChan (p, j)) st.out
      ∧ rcvCount (asmLevelChan (p, j)) st.out
          < sk.pendsBefore p j (rcvCount (asmResChan (p, j)) st.out)
      ∧ sndCount (sk.asmOutChan (p, j)) st.out
          = rcvCount (asmResChan (p, j)) st.out - 1
      ∧ sndCount (asmLevelChan (p, j)) st.out
          ≤ rcvCount (asmLevelChan (p, j)) st.out)
    ∨ (rcvCount (asmResChan (p, j)) st.out ≤ (sk.asmResList p j).length
      ∧ 1 ≤ rcvCount (asmResChan (p, j)) st.out
      ∧ rcvCount (asmLevelChan (p, j)) st.out
          = sk.pendsBefore p j (rcvCount (asmResChan (p, j)) st.out)
      ∧ sndCount (sk.asmOutChan (p, j)) st.out
          = rcvCount (asmResChan (p, j)) st.out - 1
      ∧ rcvCount (sk.asmOutChan (p, j)) st.out
          + sk.cap (sk.asmOutChan (p, j))
          ≤ sndCount (sk.asmOutChan (p, j)) st.out) := by
  obtain ⟨hro, hlo, hoo⟩ := asm_owners sk p h1
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hIdx
  have hRc : rcvCount (asmResChan (p, j)) st.out
      = (proj (asmResChan (p, j)) false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_owner sk hwf h _ false (by simpa using hro)
        hIdx hr hpre hsub]
  have hLc : rcvCount (asmLevelChan (p, j)) st.out
      = (proj (asmLevelChan (p, j)) false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_owner sk hwf h _ false (by simpa using hlo)
        hIdx hr hpre hsub]
  have hOc : sndCount (sk.asmOutChan (p, j)) st.out
      = (proj (sk.asmOutChan (p, j)) true pre).length := by
    rw [sndCount_eq_proj,
      out_proj_owner sk hwf h _ true (by simpa using hoo)
        hIdx hr hpre hsub]
  cases r with
  | nil =>
      -- exhausted: the emitted prefix is the whole trace
      rw [List.append_nil] at hpre
      obtain ⟨ht1, ht2, ht3⟩ := asm_totals sk (p, j)
      rw [hpre] at ht1 ht2 ht3
      refine Or.inl ⟨?_, ?_, ?_⟩
      · rw [hRc, ht1, seg_len]
      · rw [hLc, ht2, seg_len]
      · rw [hOc, ht3, seg_len]
  | cons e₀ rest₀ =>
      -- a live head, disabled at the fixpoint
      have hrem : st.rem[asmIdx sk p j - manCount sk]?
          = some (e₀ :: rest₀) := by
        rw [List.getElem?_append_right
          (by rw [manFilters_length]
              exact Nat.le_of_lt (asmIdx_ge sk p h1)),
          manFilters_length] at hr
        exact hr
      have hdis : enabled sk st.sent st.rcvd e₀ = false := by
        unfold step at hfix
        cases hscan : scan sk st.sent st.rcvd st.rem with
        | some pr => rw [hscan] at hfix; simp at hfix
        | none => exact scan_none_heads sk hscan hrem
      obtain ⟨idx, hidxN, hshape⟩ :=
        asm_cell_shape sk (p, j) hpre (by simp)
      have hidxN' : idx < (sk.asmResList p j).length := hidxN
      rcases hshape with ⟨⟨rest, hhead⟩, hc1, hc2, hc3⟩
        | ⟨tlv, rest, hhead, htl, hth, hc1, hc2, hc3⟩
        | ⟨⟨rest, hhead⟩, hc1, hc2, hc3⟩
      · -- res-starved
        have he₀ : e₀ = (asmResChan (p, j), false, idx) := by
          have := congrArg (fun l : List Ev => l[0]?) hhead
          simpa using this
        rw [he₀] at hdis
        simp only [enabled, decide_eq_false_iff_not] at hdis
        rw [h.sent_eq] at hdis
        refine Or.inr (Or.inl ⟨?_, ?_, ?_, ?_⟩)
        · rw [hRc, hc1]; exact hidxN
        · rw [hLc, hc2, hRc, hc1]
        · rw [hOc, hc3, hRc, hc1]
        · rw [hRc, hc1]
          omega
      · -- level-starved
        have he₀ : e₀ = (asmLevelChan (p, j), false, tlv) := by
          have := congrArg (fun l : List Ev => l[0]?) hhead
          simpa using this
        rw [he₀] at hdis
        simp only [enabled, decide_eq_false_iff_not] at hdis
        rw [h.sent_eq] at hdis
        refine Or.inr (Or.inr (Or.inl ⟨?_, ?_, ?_, ?_, ?_, ?_⟩))
        · rw [hRc, hc1]; omega
        · rw [hRc, hc1]; omega
        · rw [hLc, hc2, hRc, hc1]
          simpa using htl
        · rw [hLc, hc2, hRc, hc1]
          simpa using hth
        · rw [hOc, hc3, hRc, hc1]
          omega
        · rw [hLc, hc2]
          omega
      · -- out-blocked
        have he₀ : e₀ = (sk.asmOutChan (p, j), true, idx) := by
          have := congrArg (fun l : List Ev => l[0]?) hhead
          simpa using this
        rw [he₀] at hdis
        simp only [enabled, decide_eq_false_iff_not] at hdis
        rw [h.rcvd_eq] at hdis
        refine Or.inr (Or.inr (Or.inr ⟨?_, ?_, ?_, ?_, ?_⟩))
        · rw [hRc, hc1]; omega
        · rw [hRc, hc1]; omega
        · rw [hLc, hc2, hRc, hc1]
        · rw [hOc, hc3, hRc, hc1]
          omega
        · rw [hOc, hc3]
          omega

-- ============================================ absorb block-run counts

/-- Absorb's per-leaf block, named for the run machinery. -/
private def absorbBlock (j : Nat) : List Ev :=
  [(Chan.wire Party.R 0, false, j),
   (Chan.leafRequests, false, j),
   (Chan.level Party.I 0, true, j)]

private theorem absorbEvents_eq :
    absorbEvents sk
      = (List.range sk.totalLeafReqs).flatMap absorbBlock := rfl

private theorem proj_absorbBlock_wire (j : Nat) :
    proj (Chan.wire Party.R 0) false (absorbBlock j)
      = seg (Chan.wire Party.R 0) false j 1 := by
  rw [seg_one]; rfl

private theorem proj_absorbBlock_leaf (j : Nat) :
    proj Chan.leafRequests false (absorbBlock j)
      = seg Chan.leafRequests false j 1 := by
  rw [seg_one]; rfl

private theorem proj_absorbBlock_level (j : Nat) :
    proj (Chan.level Party.I 0) true (absorbBlock j)
      = seg (Chan.level Party.I 0) true j 1 := by
  rw [seg_one]; rfl

private theorem proj_run_awire :
    ∀ (m a : Nat),
      proj (Chan.wire Party.R 0) false
        ((List.range' a m).flatMap absorbBlock)
      = seg (Chan.wire Party.R 0) false a m
  | 0, _ => rfl
  | m + 1, a => by
      rw [List.range'_succ, List.flatMap_cons, proj_append,
        proj_absorbBlock_wire, proj_run_awire m (a + 1),
        seg_append, Nat.add_comm 1 m]

private theorem proj_run_aleaf :
    ∀ (m a : Nat),
      proj Chan.leafRequests false
        ((List.range' a m).flatMap absorbBlock)
      = seg Chan.leafRequests false a m
  | 0, _ => rfl
  | m + 1, a => by
      rw [List.range'_succ, List.flatMap_cons, proj_append,
        proj_absorbBlock_leaf, proj_run_aleaf m (a + 1),
        seg_append, Nat.add_comm 1 m]

private theorem proj_run_alevel :
    ∀ (m a : Nat),
      proj (Chan.level Party.I 0) true
        ((List.range' a m).flatMap absorbBlock)
      = seg (Chan.level Party.I 0) true a m
  | 0, _ => rfl
  | m + 1, a => by
      rw [List.range'_succ, List.flatMap_cons, proj_append,
        proj_absorbBlock_level, proj_run_alevel m (a + 1),
        seg_append, Nat.add_comm 1 m]

/-- The absorb trace's whole-trace projections. -/
theorem absorb_totals :
    proj (Chan.wire Party.R 0) false (absorbEvents sk)
        = seg (Chan.wire Party.R 0) false 0 sk.totalLeafReqs
    ∧ proj Chan.leafRequests false (absorbEvents sk)
        = seg Chan.leafRequests false 0 sk.totalLeafReqs
    ∧ proj (Chan.level Party.I 0) true (absorbEvents sk)
        = seg (Chan.level Party.I 0) true 0 sk.totalLeafReqs := by
  rw [absorbEvents_eq, List.range_eq_range']
  exact ⟨proj_run_awire _ 0, proj_run_aleaf _ 0, proj_run_alevel _ 0⟩

/-- THE ABSORB SUFFIX TRICHOTOMY: a nonempty unemitted cell of the
absorb trace heads at its next wire receive, leaf request, or level-0
send — the block position pins all three channel-side counts of the
emitted prefix. -/
theorem absorb_cell_shape {pre r : List Ev}
    (hsplit : absorbEvents sk = pre ++ r) (hne : r ≠ []) :
    ∃ t, t < sk.totalLeafReqs ∧
      (((∃ rest, r = (Chan.wire Party.R 0, false, t) :: rest)
        ∧ (proj (Chan.wire Party.R 0) false pre).length = t
        ∧ (proj Chan.leafRequests false pre).length = t
        ∧ (proj (Chan.level Party.I 0) true pre).length = t)
      ∨ ((∃ rest, r = (Chan.leafRequests, false, t) :: rest)
        ∧ (proj (Chan.wire Party.R 0) false pre).length = t + 1
        ∧ (proj Chan.leafRequests false pre).length = t
        ∧ (proj (Chan.level Party.I 0) true pre).length = t)
      ∨ ((∃ rest, r = (Chan.level Party.I 0, true, t) :: rest)
        ∧ (proj (Chan.wire Party.R 0) false pre).length = t + 1
        ∧ (proj Chan.leafRequests false pre).length = t + 1
        ∧ (proj (Chan.level Party.I 0) true pre).length = t)) := by
  rw [absorbEvents_eq, List.range_eq_range'] at hsplit
  obtain ⟨t, -, htN, p₂, r₂, hblock, hr₂, hpre, hr⟩ :=
    prefix_flatMap _ 0 hsplit hne
  rw [Nat.zero_add] at htN
  rw [Nat.sub_zero] at hpre
  have hw_run := proj_run_awire t 0
  have hl_run := proj_run_aleaf t 0
  have hv_run := proj_run_alevel t 0
  refine ⟨t, htN, ?_⟩
  match p₂, hblock with
  | [], hblock =>
      -- block boundary: the cell heads at the wire receive
      rw [List.nil_append] at hblock
      rw [List.append_nil] at hpre
      refine Or.inl ⟨⟨(Chan.leafRequests, false, t)
          :: (Chan.level Party.I 0, true, t)
          :: (List.range' (t + 1)
              (0 + sk.totalLeafReqs - t - 1)).flatMap absorbBlock,
        ?_⟩, ?_, ?_, ?_⟩
      · rw [hr, ← hblock]
        rfl
      · rw [hpre, hw_run, seg_len]
      · rw [hpre, hl_run, seg_len]
      · rw [hpre, hv_run, seg_len]
  | e :: p₃, hblock =>
      rw [List.cons_append] at hblock
      injection hblock with he1 hinner
      subst he1
      match p₃, hinner with
      | [], hinner =>
          -- the cell heads at the leaf request
          rw [List.nil_append] at hinner
          refine Or.inr (Or.inl ⟨⟨(Chan.level Party.I 0, true, t)
              :: (List.range' (t + 1)
                  (0 + sk.totalLeafReqs - t - 1)).flatMap absorbBlock,
            ?_⟩, ?_, ?_, ?_⟩)
          · rw [hr, ← hinner]
            rfl
          · rw [hpre, proj_append, hw_run, List.length_append, seg_len]
            rfl
          · rw [hpre, proj_append, hl_run, List.length_append, seg_len]
            rfl
          · rw [hpre, proj_append, hv_run, List.length_append, seg_len]
            rfl
      | e' :: p₄, hinner =>
          rw [List.cons_append] at hinner
          injection hinner with he2 hinner₂
          subst he2
          match p₄, hinner₂ with
          | [], hinner₂ =>
              -- the cell heads at the level-0 send
              rw [List.nil_append] at hinner₂
              refine Or.inr (Or.inr ⟨⟨(List.range' (t + 1)
                  (0 + sk.totalLeafReqs - t - 1)).flatMap absorbBlock,
                ?_⟩, ?_, ?_, ?_⟩)
              · rw [hr, ← hinner₂]
                rfl
              · rw [hpre, proj_append, hw_run, List.length_append,
                  seg_len]
                rfl
              · rw [hpre, proj_append, hl_run, List.length_append,
                  seg_len]
                rfl
              · rw [hpre, proj_append, hv_run, List.length_append,
                  seg_len]
                rfl
          | e'' :: p₅, hinner₂ =>
              exfalso
              rw [List.cons_append] at hinner₂
              injection hinner₂ with he3 hinner₃
              exact hr₂ (List.append_eq_nil_iff.1 hinner₃.symm).2

/-- THE ABSORB STUCK TRICHOTOMY: at a pump fixpoint the absorber is
exhausted, wire-starved, request-starved, or blocked on its level-0
output — each with all three counts pinned and the failed guard
recorded. -/
theorem absorb_stuck (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st)
    (hfix : step sk st = none) :
    (rcvCount (Chan.wire Party.R 0) st.out = sk.totalLeafReqs
      ∧ rcvCount Chan.leafRequests st.out = sk.totalLeafReqs
      ∧ sndCount (Chan.level Party.I 0) st.out = sk.totalLeafReqs)
    ∨ (rcvCount (Chan.wire Party.R 0) st.out < sk.totalLeafReqs
      ∧ rcvCount Chan.leafRequests st.out
          = rcvCount (Chan.wire Party.R 0) st.out
      ∧ sndCount (Chan.level Party.I 0) st.out
          = rcvCount (Chan.wire Party.R 0) st.out
      ∧ sndCount (Chan.wire Party.R 0) st.out
          ≤ rcvCount (Chan.wire Party.R 0) st.out)
    ∨ (rcvCount Chan.leafRequests st.out < sk.totalLeafReqs
      ∧ rcvCount (Chan.wire Party.R 0) st.out
          = rcvCount Chan.leafRequests st.out + 1
      ∧ sndCount (Chan.level Party.I 0) st.out
          = rcvCount Chan.leafRequests st.out
      ∧ sndCount Chan.leafRequests st.out
          ≤ rcvCount Chan.leafRequests st.out)
    ∨ (sndCount (Chan.level Party.I 0) st.out < sk.totalLeafReqs
      ∧ rcvCount (Chan.wire Party.R 0) st.out
          = sndCount (Chan.level Party.I 0) st.out + 1
      ∧ rcvCount Chan.leafRequests st.out
          = sndCount (Chan.level Party.I 0) st.out + 1
      ∧ rcvCount (Chan.level Party.I 0) st.out
          + sk.cap (Chan.level Party.I 0)
          ≤ sndCount (Chan.level Party.I 0) st.out) := by
  have hwo : rcvOwner sk (Chan.wire Party.R 0) = 2 + sk.rootH := by
    simp [rcvOwner]
  have hlo : rcvOwner sk Chan.leafRequests = 2 + sk.rootH := rfl
  have hvo : sndOwner sk (Chan.level Party.I 0) = 2 + sk.rootH := by
    simp [sndOwner]
  have hIdx := procs_absorb sk
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hIdx
  have hWc : rcvCount (Chan.wire Party.R 0) st.out
      = (proj (Chan.wire Party.R 0) false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_owner sk hwf h _ false (by simpa using hwo)
        hIdx hr hpre hsub]
  have hLc : rcvCount Chan.leafRequests st.out
      = (proj Chan.leafRequests false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_owner sk hwf h _ false (by simpa using hlo)
        hIdx hr hpre hsub]
  have hVc : sndCount (Chan.level Party.I 0) st.out
      = (proj (Chan.level Party.I 0) true pre).length := by
    rw [sndCount_eq_proj,
      out_proj_owner sk hwf h _ true (by simpa using hvo)
        hIdx hr hpre hsub]
  cases r with
  | nil =>
      -- exhausted: the emitted prefix is the whole trace
      rw [List.append_nil] at hpre
      obtain ⟨ht1, ht2, ht3⟩ := absorb_totals sk
      rw [hpre] at ht1 ht2 ht3
      refine Or.inl ⟨?_, ?_, ?_⟩
      · rw [hWc, ht1, seg_len]
      · rw [hLc, ht2, seg_len]
      · rw [hVc, ht3, seg_len]
  | cons e₀ rest₀ =>
      -- a live head, disabled at the fixpoint
      have hrem : st.rem[2 + sk.rootH - manCount sk]?
          = some (e₀ :: rest₀) := by
        rw [List.getElem?_append_right
          (by rw [manFilters_length]; exact Nat.le_refl _),
          manFilters_length] at hr
        exact hr
      have hdis : enabled sk st.sent st.rcvd e₀ = false := by
        unfold step at hfix
        cases hscan : scan sk st.sent st.rcvd st.rem with
        | some pr => rw [hscan] at hfix; simp at hfix
        | none => exact scan_none_heads sk hscan hrem
      obtain ⟨t, htN, hshape⟩ := absorb_cell_shape sk hpre (by simp)
      rcases hshape with ⟨⟨rest, hhead⟩, hc1, hc2, hc3⟩
        | ⟨⟨rest, hhead⟩, hc1, hc2, hc3⟩
        | ⟨⟨rest, hhead⟩, hc1, hc2, hc3⟩
      · -- wire-starved
        have he₀ : e₀ = (Chan.wire Party.R 0, false, t) := by
          have := congrArg (fun l : List Ev => l[0]?) hhead
          simpa using this
        rw [he₀] at hdis
        simp only [enabled, decide_eq_false_iff_not] at hdis
        rw [h.sent_eq] at hdis
        refine Or.inr (Or.inl ⟨?_, ?_, ?_, ?_⟩)
        · rw [hWc, hc1]; exact htN
        · rw [hLc, hc2, hWc, hc1]
        · rw [hVc, hc3, hWc, hc1]
        · rw [hWc, hc1]; omega
      · -- request-starved
        have he₀ : e₀ = (Chan.leafRequests, false, t) := by
          have := congrArg (fun l : List Ev => l[0]?) hhead
          simpa using this
        rw [he₀] at hdis
        simp only [enabled, decide_eq_false_iff_not] at hdis
        rw [h.sent_eq] at hdis
        refine Or.inr (Or.inr (Or.inl ⟨?_, ?_, ?_, ?_⟩))
        · rw [hLc, hc2]; exact htN
        · rw [hWc, hc1, hLc, hc2]
        · rw [hVc, hc3, hLc, hc2]
        · rw [hLc, hc2]; omega
      · -- level-blocked
        have he₀ : e₀ = (Chan.level Party.I 0, true, t) := by
          have := congrArg (fun l : List Ev => l[0]?) hhead
          simpa using this
        rw [he₀] at hdis
        simp only [enabled, decide_eq_false_iff_not] at hdis
        rw [h.rcvd_eq] at hdis
        refine Or.inr (Or.inr (Or.inr ⟨?_, ?_, ?_, ?_⟩))
        · rw [hVc, hc3]; exact htN
        · rw [hWc, hc1, hVc, hc3]
        · rw [hLc, hc2, hVc, hc3]
        · rw [hVc, hc3]; omega

-- ==================================================== fins stuckness

private theorem finEvents_eq :
    finEvents sk = (Chan.rootres, false, 0)
      :: seg Chan.rootrets false 0 sk.rootPending := by
  unfold finEvents seg
  rw [List.range_eq_range']
  simp

private theorem rootres_ne_rootrets :
    (Chan.rootres : Chan) ≠ Chan.rootrets := by simp

/-- THE FINS SUFFIX SHAPE: a nonempty unemitted cell of the fins trace
heads at the root resolution or at a root return, with both receive
counts of the emitted prefix pinned. -/
theorem fin_cell_shape {pre r : List Ev}
    (hsplit : finEvents sk = pre ++ r) (hne : r ≠ []) :
    ((∃ rest, r = (Chan.rootres, false, 0) :: rest)
      ∧ (proj Chan.rootres false pre).length = 0
      ∧ (proj Chan.rootrets false pre).length = 0)
    ∨ (∃ t, t < sk.rootPending
      ∧ (∃ rest, r = (Chan.rootrets, false, t) :: rest)
      ∧ (proj Chan.rootres false pre).length = 1
      ∧ (proj Chan.rootrets false pre).length = t) := by
  rw [finEvents_eq] at hsplit
  match pre, hsplit with
  | [], hsplit =>
      rw [List.nil_append] at hsplit
      exact Or.inl ⟨⟨seg Chan.rootrets false 0 sk.rootPending,
        hsplit.symm⟩, rfl, rfl⟩
  | e :: pre', hsplit =>
      rw [List.cons_append] at hsplit
      injection hsplit with he1 hinner
      subst he1
      -- the cell is a suffix of the returns segment
      have hlen : sk.rootPending = pre'.length + r.length := by
        have := congrArg List.length hinner
        simpa [seg_len] using this
      have hrlen : 1 ≤ r.length := by
        cases r with
        | nil => exact absurd rfl hne
        | cons _ _ => simp
      have htN : pre'.length < sk.rootPending := by omega
      have hp'take : pre' = seg Chan.rootrets false 0 pre'.length := by
        have hthis := congrArg (List.take pre'.length) hinner
        rw [List.take_append_of_le_length (Nat.le_refl _),
          List.take_of_length_le (Nat.le_refl _), seg_take] at hthis
        rw [show min pre'.length sk.rootPending = pre'.length
          from by omega] at hthis
        exact hthis.symm
      have hhead : r[0]? = some (Chan.rootrets, false, pre'.length) := by
        have hread := congrArg (fun l : List Ev => l[pre'.length]?)
          hinner
        rw [seg_getElem? _ _ _ _ _ htN,
          List.getElem?_append_right (Nat.le_refl _), Nat.sub_self]
          at hread
        simpa using hread.symm
      refine Or.inr ⟨pre'.length, htN, ?_, ?_, ?_⟩
      · cases r with
        | nil => exact absurd rfl hne
        | cons x rest =>
            refine ⟨rest, ?_⟩
            simp only [List.getElem?_cons_zero, Option.some.injEq]
              at hhead
            rw [hhead]
      · rw [proj_cons_self, hp'take,
          proj_seg_ne (fun hh => rootres_ne_rootrets hh.1.symm)]
        rfl
      · rw [proj_cons_ne_chan rootres_ne_rootrets, hp'take,
          proj_seg_self, seg_len]

/-- FINS STUCKNESS: at a pump fixpoint the finisher is exhausted,
starved of the root resolution, or starved of a root return — with
both counts pinned and the failed guard recorded. -/
theorem fin_stuck (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st)
    (hfix : step sk st = none) (hge : 1 ≤ sk.rootH) :
    (rcvCount Chan.rootres st.out = 1
      ∧ rcvCount Chan.rootrets st.out = sk.rootPending)
    ∨ (rcvCount Chan.rootres st.out = 0
      ∧ rcvCount Chan.rootrets st.out = 0
      ∧ sndCount Chan.rootres st.out = 0)
    ∨ (rcvCount Chan.rootrets st.out < sk.rootPending
      ∧ rcvCount Chan.rootres st.out = 1
      ∧ sndCount Chan.rootrets st.out
          ≤ rcvCount Chan.rootrets st.out) := by
  have hIdx := procs_fin sk hge
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hIdx
  have hAc : rcvCount Chan.rootres st.out
      = (proj Chan.rootres false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_owner sk hwf h _ false (by simp [rcvOwner])
        hIdx hr hpre hsub]
  have hBc : rcvCount Chan.rootrets st.out
      = (proj Chan.rootrets false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_owner sk hwf h _ false (by simp [rcvOwner])
        hIdx hr hpre hsub]
  cases r with
  | nil =>
      rw [List.append_nil] at hpre
      rw [finEvents_eq] at hpre
      refine Or.inl ⟨?_, ?_⟩
      · rw [hAc, ← hpre, proj_cons_self,
          proj_seg_ne (fun hh => rootres_ne_rootrets hh.1.symm)]
        rfl
      · rw [hBc, ← hpre, proj_cons_ne_chan rootres_ne_rootrets,
          proj_seg_self, seg_len]
  | cons e₀ rest₀ =>
      have hrem : st.rem[3 * sk.rootH + 3 - manCount sk]?
          = some (e₀ :: rest₀) := by
        rw [List.getElem?_append_right
          (by rw [manFilters_length]; show manCount sk ≤ _; unfold manCount; omega),
          manFilters_length] at hr
        exact hr
      have hdis : enabled sk st.sent st.rcvd e₀ = false := by
        unfold step at hfix
        cases hscan : scan sk st.sent st.rcvd st.rem with
        | some pr => rw [hscan] at hfix; simp at hfix
        | none => exact scan_none_heads sk hscan hrem
      rcases fin_cell_shape sk hpre (by simp) with
        ⟨⟨rest, hhead⟩, hc1, hc2⟩ | ⟨t, htN, ⟨rest, hhead⟩, hc1, hc2⟩
      · -- rootres-starved
        have he₀ : e₀ = (Chan.rootres, false, 0) := by
          have := congrArg (fun l : List Ev => l[0]?) hhead
          simpa using this
        rw [he₀] at hdis
        simp only [enabled, decide_eq_false_iff_not] at hdis
        rw [h.sent_eq] at hdis
        refine Or.inr (Or.inl ⟨?_, ?_, ?_⟩)
        · rw [hAc, hc1]
        · rw [hBc, hc2]
        · omega
      · -- rootrets-starved
        have he₀ : e₀ = (Chan.rootrets, false, t) := by
          have := congrArg (fun l : List Ev => l[0]?) hhead
          simpa using this
        rw [he₀] at hdis
        simp only [enabled, decide_eq_false_iff_not] at hdis
        rw [h.sent_eq] at hdis
        refine Or.inr (Or.inr ⟨?_, ?_, ?_⟩)
        · rw [hBc, hc2]; exact htN
        · rw [hAc, hc1]
        · rw [hBc, hc2]; omega

/-- ROOTRET STUCKNESS: at a pump fixpoint the floating `rootret`
receive has either fired or is starved. -/
theorem rootret_stuck (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st)
    (hfix : step sk st = none) (hge : 1 ≤ sk.rootH) :
    rcvCount Chan.rootret st.out = 1
    ∨ (rcvCount Chan.rootret st.out = 0
      ∧ sndCount Chan.rootret st.out = 0) := by
  have hIdx := procs_rootret sk hge
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hIdx
  have hAc : rcvCount Chan.rootret st.out
      = (proj Chan.rootret false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_owner sk hwf h _ false (by simp [rcvOwner])
        hIdx hr hpre hsub]
  cases r with
  | nil =>
      rw [List.append_nil] at hpre
      refine Or.inl ?_
      rw [hAc, ← hpre]
      rfl
  | cons e₀ rest₀ =>
      have hpre_nil : pre = [] := by
        cases pre with
        | nil => rfl
        | cons p ps =>
            exfalso
            rw [List.cons_append] at hpre
            injection hpre with hp1 hp2
            have := congrArg List.length hp2
            simp at this
      have he₀ : e₀ = (Chan.rootret, false, 0) := by
        rw [hpre_nil, List.nil_append] at hpre
        have := congrArg (fun l : List Ev => l[0]?) hpre
        simpa using this.symm
      have hrem : st.rem[3 * sk.rootH + 2 - manCount sk]?
          = some (e₀ :: rest₀) := by
        rw [List.getElem?_append_right
          (by rw [manFilters_length]; show manCount sk ≤ _; unfold manCount; omega),
          manFilters_length] at hr
        exact hr
      have hdis : enabled sk st.sent st.rcvd e₀ = false := by
        unfold step at hfix
        cases hscan : scan sk st.sent st.rcvd st.rem with
        | some pr => rw [hscan] at hfix; simp at hfix
        | none => exact scan_none_heads sk hscan hrem
      rw [he₀] at hdis
      simp only [enabled, decide_eq_false_iff_not] at hdis
      rw [h.sent_eq] at hdis
      refine Or.inr ⟨?_, ?_⟩
      · rw [hAc, hpre_nil]
        rfl
      · omega

end StreamingMirror.Sched
