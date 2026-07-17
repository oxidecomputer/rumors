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
      have hcanon := wproj_canon sk hwf h.toWCount c false
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

end StreamingMirror.Sched
