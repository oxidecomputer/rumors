/-
The O pump layer, state tier: `FamOK`'s ord-parameterized twin
(`FamOKO`, pump suffix pinned to `weavePumpsO`) and the
`hfam`-consuming toolkit over it — the owner-projection collapse, the
two seq laws, the pump-row lookups, pump support, and the four per-row
stuck trichotomies.

# Shape

`FamOK.pumps` is a literal list equality against `weavePumps`, whose
absorber row is the reply-first `absorbEvents` — so the query-first
absorber assignments cannot inhabit `FamOK`, and every lemma consuming
that bundle needs a twin over `FamOKO` (pump suffix `weavePumpsO`,
inhabited by `procsO` at EVERY assignment: `famOKO_procsO`). Only the
absorber row differs from the base family, and only in the order of
its two per-block receives:

- The asm towers, the `rootret` floater, and fins are the base rows
  verbatim (`famOKO_tail_lookup` reads them straight out of `procs`),
  so `asm_stuckO`/`fin_stuckO`/`rootret_stuckO` are signature-swap
  transcriptions of their base mirrors.
- The absorber row genuinely dispatches on `ord.absorb`. Its
  trichotomy is stated ONCE, with the starved arms' count offsets read
  off `ord.absorb.wirePhase` (`0`/`1`): the first-received channel of
  a block runs `wirePhase`-many receives ahead of the wire and
  `1 - wirePhase` behind it, per arm. At `ord.absorb = .replyFirst`
  the offsets reduce to literals and the arms are the base
  `absorb_stuck`'s verbatim; consumers close the starved arms by
  `omega` with the offset symbolic — no dispatch at the call sites.
- The absorber row's channel-side SET is unchanged (the receives only
  swap), so `pump_supportO` is the base statement over `weavePumpsO`.

Chain (ord, stage D): the pump layer; consumed by Ord/Window.lean's O
chains and windows, then by the O master induction and the O drain
ladder. Base mirror: Proofs/Sched/Weave/Pump.lean (+ the family
lookups of Proofs/Sched/Weave/Window.lean). Map: PROGRESS.md §13.
-/
import StreamingMirror.Ord.Pumps
import StreamingMirror.Proofs.Sched.Weave.Window

namespace StreamingMirror.Ord

open Model
open Sched

variable (sk : Skel) (ord : OrdMap)

-- ================================================= the family bundle

/-- `FamOK`'s O twin: same canon and ownership fields, pump half
pinned to `weavePumpsO` — the absorber row in ITS assigned order.

Unlike `FamOK`, this bundle is inhabited by the O family at EVERY
assignment of the class (`famOKO_procsO`); at a reply-first absorber
the two bundles coincide (`weavePumpsO_rf`). -/
structure FamOKO (P : List (List Ev)) : Prop where
  canon : ∀ (c : Chan) (b : Bool), ∀ t ∈ P, ∃ m, proj c b t = canon c b m
  snd_owned : Owned (sndOwner sk) true 0 P
  rcv_owned : Owned (rcvOwner sk) false 0 P
  pumps : P.drop (manCount sk) = weavePumpsO sk ord

/-- The O family carries the bundle, at every assignment — the
instance the recorded FamOK deviation exists to restore. -/
theorem famOKO_procsO (hwf : sk.wellFormed = true) :
    FamOKO sk ord (procsO sk ord) :=
  ⟨procsO_canon sk ord, procsO_snd_owned sk ord hwf,
    procsO_rcv_owned sk ord hwf, procsO_drop_pumpsO sk ord⟩

-- ================================================== pump-row lookups

/-- Pump-half lookups are family-independent: past `manCount` every
`FamOKO` family reads `weavePumpsO`, exactly as `procsO` does. -/
theorem famOKO_pump_lookup {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {i : Nat} (hi : manCount sk ≤ i) :
    P[i]? = (procsO sk ord)[i]? := by
  obtain ⟨m, rfl⟩ := Nat.exists_eq_add_of_le hi
  rw [show P[manCount sk + m]? = (P.drop (manCount sk))[m]? from
      (List.getElem?_drop ..).symm,
    show (procsO sk ord)[manCount sk + m]?
        = ((procsO sk ord).drop (manCount sk))[m]? from
      (List.getElem?_drop ..).symm,
    hfam.pumps, procsO_drop_pumpsO]

/-- Past its absorber head the O pump family IS the base pump family:
the two lists share their tail verbatim. -/
private theorem weavePumpsO_getElem?_succ (m : Nat) :
    (weavePumpsO sk ord)[m + 1]? = (weavePumps sk)[m + 1]? := rfl

/-- Strictly past the absorber slot, every `FamOKO` family reads the
BASE family `procs`: the asm towers, the `rootret` floater, and fins
are order-independent rows. -/
theorem famOKO_tail_lookup {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {i : Nat} (hi : manCount sk + 1 ≤ i) :
    P[i]? = (procs sk)[i]? := by
  obtain ⟨m, rfl⟩ := Nat.exists_eq_add_of_le hi
  rw [show manCount sk + 1 + m = manCount sk + (m + 1) from by omega,
    show P[manCount sk + (m + 1)]? = (P.drop (manCount sk))[m + 1]? from
      (List.getElem?_drop ..).symm,
    show (procs sk)[manCount sk + (m + 1)]?
        = ((procs sk).drop (manCount sk))[m + 1]? from
      (List.getElem?_drop ..).symm,
    hfam.pumps, ← weavePumps_eq, weavePumpsO_getElem?_succ]

/-- The O absorb trace sits at slot `2 + rootH` of the O family
(cf. `procs_absorb`). -/
theorem procsO_absorb :
    (procsO sk ord)[2 + sk.rootH]? = some (absorbEventsO sk ord) := by
  have h : ((procsO sk ord).drop (manCount sk))[0]?
      = some (absorbEventsO sk ord) := by
    rw [procsO_drop_pumpsO]
    rfl
  rw [List.getElem?_drop, Nat.add_zero] at h
  show (procsO sk ord)[manCount sk]? = some (absorbEventsO sk ord)
  exact h

/-- `procsO_absorb`, any `FamOKO` family: the absorber slot holds the
O absorb trace. -/
theorem famOKO_absorb {P : List (List Ev)} (hfam : FamOKO sk ord P) :
    P[2 + sk.rootH]? = some (absorbEventsO sk ord) := by
  rw [famOKO_pump_lookup sk ord hfam
    (show manCount sk ≤ 2 + sk.rootH from Nat.le_of_eq rfl)]
  exact procsO_absorb sk ord

/-- `famOK_asm_procs`'s O twin: tower slots live in the shared,
order-independent pump tail. -/
theorem famOKO_asm_procs {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {p : Party} {top j : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (h1 : 1 ≤ j) (hjt : j ≤ top) :
    P[asmIdx sk p j]? = some (asmEvents sk (p, j)) := by
  rw [famOKO_tail_lookup sk ord hfam
    (show manCount sk + 1 ≤ asmIdx sk p j from by
      cases p
      · show 2 + sk.rootH + 1 ≤ 3 + sk.rootH + (j - 1)
        omega
      · show 2 + sk.rootH + 1 ≤ 3 + 2 * sk.rootH + (j - 1)
        omega)]
  exact asm_procs sk htop h1 hjt

/-- `famOK_asmI`'s O twin. -/
theorem famOKO_asmI {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {j : Nat} (h1 : 1 ≤ j) (hj : j ≤ sk.rootH) :
    P[asmIdx sk Party.I j]? = some (asmEvents sk (Party.I, j)) := by
  rw [famOKO_tail_lookup sk ord hfam
    (show manCount sk + 1 ≤ asmIdx sk Party.I j from by
      show 2 + sk.rootH + 1 ≤ 3 + sk.rootH + (j - 1)
      omega)]
  exact procs_asmI sk h1 hj

/-- `famOK_asmR`'s O twin. -/
theorem famOKO_asmR {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {j : Nat} (h1 : 1 ≤ j) (hj : j ≤ sk.rootH - 1) :
    P[asmIdx sk Party.R j]? = some (asmEvents sk (Party.R, j)) := by
  rw [famOKO_tail_lookup sk ord hfam
    (show manCount sk + 1 ≤ asmIdx sk Party.R j from by
      show 2 + sk.rootH + 1 ≤ 3 + 2 * sk.rootH + (j - 1)
      omega)]
  exact procs_asmR sk h1 hj

/-- `famOK_fin`'s O twin. -/
theorem famOKO_fin {P : List (List Ev)} (hfam : FamOKO sk ord P)
    (hge : 1 ≤ sk.rootH) :
    P[3 * sk.rootH + 3]? = some (finEvents sk) := by
  rw [famOKO_tail_lookup sk ord hfam
    (show manCount sk + 1 ≤ 3 * sk.rootH + 3 from by
      show 2 + sk.rootH + 1 ≤ 3 * sk.rootH + 3
      omega)]
  exact procs_fin sk hge

/-- `famOK_rootret`'s O twin. -/
theorem famOKO_rootret {P : List (List Ev)} (hfam : FamOKO sk ord P)
    (hge : 1 ≤ sk.rootH) :
    P[3 * sk.rootH + 2]? = some [(Chan.rootret, false, 0)] := by
  rw [famOKO_tail_lookup sk ord hfam
    (show manCount sk + 1 ≤ 3 * sk.rootH + 2 from by
      show 2 + sk.rootH + 1 ≤ 3 * sk.rootH + 2
      omega)]
  exact procs_rootret sk hge

-- ======================================= the owner-projection collapse

/-- Only the owner's pair feeds a channel-side (private twin of the
base `emittedCount_owner`, Proofs/Sched/Weave/Pump.lean): the
emitted-prefix counts of every other pair vanish, so the family total
is the owner's prefix projection. -/
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

/-- THE COLLAPSE, O twin of `out_proj_owner`: `out`'s projection on an
owned channel-side is its owner's emitted prefix's projection. -/
theorem out_proj_ownerO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) (c : Chan) (b : Bool)
    {M : Nat} (hM : (if b then sndOwner sk c else rcvOwner sk c) = M)
    {T r pre : List Ev}
    (hT : P[M]? = some T)
    (hr : (manFilters sk fut ++ st.rem)[M]? = some r)
    (hpre : T = pre ++ r) (hsub : pre.Sublist st.out) :
    proj c b st.out = proj c b pre := by
  have howned : Owned (if b then sndOwner sk else rcvOwner sk) b 0 P := by
    cases b
    · exact hfam.rcv_owned
    · exact hfam.snd_owned
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

/-- THE HEAD-SEQ LAW, O twin of `cell_head_seq`: when a cell's head
sits on a channel-side its trace owns, the head's seq is exactly the
current count. (`cell_of_owner` itself is family-generic — no twin.) -/
theorem cell_head_seqO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) (c : Chan) (b : Bool)
    {M : Nat} (hM : (if b then sndOwner sk c else rcvOwner sk c) = M)
    {T r pre : List Ev} {n : Nat} {rest : List Ev}
    (hT : P[M]? = some T)
    (hr : (manFilters sk fut ++ st.rem)[M]? = some r)
    (hpre : T = pre ++ r) (hsub : pre.Sublist st.out)
    (hhead : r = (c, b, n) :: rest) :
    n = (proj c b st.out).length := by
  have hTmem : T ∈ P := List.mem_of_getElem? hT
  obtain ⟨m, hcanon⟩ := hfam.canon c b T hTmem
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
  rw [hn, out_proj_ownerO sk ord hfam h c b hM hT hr hpre hsub]

/-- FRESHNESS, O twin of `cell_not_out`: an event still in a cell is
unemitted — its seq sits at or past the current count. -/
theorem cell_not_outO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) (c : Chan) (b : Bool)
    {M : Nat} (hM : (if b then sndOwner sk c else rcvOwner sk c) = M)
    {T r pre : List Ev} {n : Nat}
    (hT : P[M]? = some T)
    (hr : (manFilters sk fut ++ st.rem)[M]? = some r)
    (hpre : T = pre ++ r) (hsub : pre.Sublist st.out)
    (hmem : ((c, b, n) : Ev) ∈ r) :
    (proj c b st.out).length ≤ n := by
  have hTmem : T ∈ P := List.mem_of_getElem? hT
  obtain ⟨m, hcanon⟩ := hfam.canon c b T hTmem
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
  rw [out_proj_ownerO sk ord hfam h c b hM hT hr hpre hsub]
  omega

-- ================================================== rcvd never leads

/-- A prefix's count never exceeds the whole's (private twin of the
base `sndCount_take_le`, Proofs/Sched/Weave/Pump.lean). -/
private theorem sndCount_take_le (c : Chan) (l : List Ev) (k : Nat) :
    sndCount c (l.take k) ≤ sndCount c l := by
  rw [sndCount_eq_proj, sndCount_eq_proj]
  exact ((List.take_sublist k l).filter _).length_le

/-- Consumption never outruns production, O twin of
`wedge_rcvd_le_sent`: from guard-history plus canon. -/
theorem wedge_rcvd_le_sentO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {fut : List Ev}
    {st : MState} (h : WEdgeP sk P fut st) (c : Chan) :
    rcvCount c st.out ≤ sndCount c st.out := by
  cases hz : rcvCount c st.out with
  | zero => omega
  | succ q =>
      -- the top receive is at seq q; its guard held over a prefix
      have hcanon := wproj_canonP sk h.toWCountP c false
        (hfam.rcv_owned) (hfam.canon c false)
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

-- ====================================================== pump support

/-- Support of the O pump family, twin of `pump_support`: the absorber
row's receives only SWAP under the assignment, so its channel-side set
— and hence the support claim — is the base one; the other rows are
shared verbatim. -/
theorem pump_supportO {t : List Ev}
    (ht : t ∈ weavePumpsO sk ord) {e : Ev} (he : e ∈ t) :
    (∀ p hh, e.1 = Chan.wire p hh → hh = 0 ∧ e.2.1 = false) ∧
    ∀ p hh, e.1 ≠ Chan.asked p hh := by
  simp only [weavePumpsO, List.mem_append, List.mem_cons, List.mem_map,
    List.not_mem_nil, or_false] at ht
  rcases ht with (rfl | ⟨pk, hpk, rfl⟩) | rfl | rfl
  · -- the absorber row: same event set as the base row, per block
    obtain ⟨j, -, he⟩ := List.mem_flatMap.1 he
    have he' := (absorbBlockO_perm ord j).subset he
    rcases he' with _ | ⟨_, he'⟩
    · refine ⟨fun p hh hw => ?_, fun p hh hw => nomatch hw⟩
      injection hw with h1 h2
      exact ⟨h2.symm, rfl⟩
    · rcases he' with _ | ⟨_, he'⟩
      · exact ⟨fun p hh hw => (nomatch hw), fun p hh hw => nomatch hw⟩
      · rcases he' with _ | ⟨_, he'⟩
        · exact ⟨fun p hh hw => (nomatch hw), fun p hh hw => nomatch hw⟩
        · cases he'
  · -- an asm tower: a shared base row
    exact pump_support sk
      (by
        simp only [weavePumps, List.mem_append, List.mem_cons,
          List.mem_map, List.not_mem_nil, or_false]
        exact Or.inl (Or.inr ⟨pk, hpk, rfl⟩)) he
  · -- the floating rootret receive: a shared base row
    exact pump_support sk
      (by
        simp only [weavePumps, List.mem_append, List.mem_cons,
          List.mem_map, List.not_mem_nil, or_false]
        exact Or.inr (Or.inl trivial)) he
  · -- fins: a shared base row
    exact pump_support sk
      (by
        simp only [weavePumps, List.mem_append, List.mem_cons,
          List.mem_map, List.not_mem_nil, or_false]
        exact Or.inr (Or.inr trivial)) he

/-- O pump remainders never hold a wire event above the leaf stage's
receives (twin of `pump_rem_no_wireP`, pump pin re-denominated to
`weavePumpsO`). -/
theorem pump_rem_no_wirePO {P : List (List Ev)} {fut : List Ev}
    {st : MState}
    (h : WCountP sk P fut st)
    (hpumps : P.drop (manCount sk) = weavePumpsO sk ord)
    {p : Party} {hh n : Nat} {b : Bool}
    (hb : hh ≠ 0 ∨ b = true) :
    ∀ r ∈ st.rem, ((Chan.wire p hh, b, n) : Ev) ∉ r := by
  intro r hr hmem
  obtain ⟨t, ht, pre, hpre, -⟩ :=
    h.pump_struct.exists_of_mem_right hr
  rw [hpumps] at ht
  have het : ((Chan.wire p hh, b, n) : Ev) ∈ t := by
    rw [hpre]; exact List.mem_append_right _ hmem
  have hcl := (pump_supportO sk ord ht het).1 p hh rfl
  rcases hb with h0 | h1
  · exact h0 hcl.1
  · rw [show ((Chan.wire p hh, b, n) : Ev).2.1 = b from rfl] at hcl
    rw [h1] at hcl
    exact Bool.noConfusion hcl.2

/-- O pump remainders never hold an `asked` event (twin of
`pump_rem_no_askedP`). -/
theorem pump_rem_no_askedPO {P : List (List Ev)} {fut : List Ev}
    {st : MState}
    (h : WCountP sk P fut st)
    (hpumps : P.drop (manCount sk) = weavePumpsO sk ord)
    {p : Party} {hh n : Nat} {b : Bool} :
    ∀ r ∈ st.rem, ((Chan.asked p hh, b, n) : Ev) ∉ r := by
  intro r hr hmem
  obtain ⟨t, ht, pre, hpre, -⟩ :=
    h.pump_struct.exists_of_mem_right hr
  rw [hpumps] at ht
  have het : ((Chan.asked p hh, b, n) : Ev) ∈ t := by
    rw [hpre]; exact List.mem_append_right _ hmem
  exact absurd rfl ((pump_supportO sk ord ht het).2 p hh)

-- ================================================ fixpoint stuckness

/-- The manual filter family has one cell per manual trace (private
twin of the base `manFilters_length`). -/
private theorem manFilters_length (fut : List Ev) :
    (manFilters sk fut).length = manCount sk := by
  simp [manFilters]

/-- Every asm slot sits past the manual prefix (private twin of the
base `asmIdx_ge`). -/
private theorem asmIdx_ge (p : Party) {j : Nat} (_h1 : 1 ≤ j) :
    manCount sk < asmIdx sk p j := by
  cases p <;> (show manCount sk < 3 + _ + (j - 1); unfold manCount) <;>
    omega

/-- THE STUCK TRICHOTOMY for asm towers, O twin of `asm_stuck`: the
tower rows are order-independent, so the statement and arms are the
base lemma's verbatim over the O bundle. -/
theorem asm_stuckO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st)
    (hfix : step sk st = none) {p : Party} {j : Nat} (h1 : 1 ≤ j)
    (hIdx : P[asmIdx sk p j]?
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
      out_proj_ownerO sk ord hfam h _ false (by simpa using hro)
        hIdx hr hpre hsub]
  have hLc : rcvCount (asmLevelChan (p, j)) st.out
      = (proj (asmLevelChan (p, j)) false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_ownerO sk ord hfam h _ false (by simpa using hlo)
        hIdx hr hpre hsub]
  have hOc : sndCount (sk.asmOutChan (p, j)) st.out
      = (proj (sk.asmOutChan (p, j)) true pre).length := by
    rw [sndCount_eq_proj,
      out_proj_ownerO sk ord hfam h _ true (by simpa using hoo)
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

/-- The O absorb trace's whole-trace projections: per channel-side the
projection bridge collapses them to the base totals. -/
theorem absorb_totalsO :
    proj (Chan.wire Party.R 0) false (absorbEventsO sk ord)
        = seg (Chan.wire Party.R 0) false 0 sk.totalLeafReqs
    ∧ proj Chan.leafRequests false (absorbEventsO sk ord)
        = seg Chan.leafRequests false 0 sk.totalLeafReqs
    ∧ proj (Chan.level Party.I 0) true (absorbEventsO sk ord)
        = seg (Chan.level Party.I 0) true 0 sk.totalLeafReqs := by
  obtain ⟨h1, h2, h3⟩ := absorb_totals sk
  exact ⟨by rw [proj_absorbEventsO_eq]; exact h1,
    by rw [proj_absorbEventsO_eq]; exact h2,
    by rw [proj_absorbEventsO_eq]; exact h3⟩

/-- The O block's wire projection is one segment slot (any order). -/
private theorem proj_absorbBlockO_wire (j : Nat) :
    proj (Chan.wire Party.R 0) false (absorbBlockO ord j)
      = seg (Chan.wire Party.R 0) false j 1 := by
  rw [proj_absorbBlockO_eq, seg_one]
  rfl

/-- The O block's leaf-request projection is one segment slot. -/
private theorem proj_absorbBlockO_leaf (j : Nat) :
    proj Chan.leafRequests false (absorbBlockO ord j)
      = seg Chan.leafRequests false j 1 := by
  rw [proj_absorbBlockO_eq, seg_one]
  rfl

/-- The O block's level-0 projection is one segment slot. -/
private theorem proj_absorbBlockO_level (j : Nat) :
    proj (Chan.level Party.I 0) true (absorbBlockO ord j)
      = seg (Chan.level Party.I 0) true j 1 := by
  rw [proj_absorbBlockO_eq, seg_one]
  rfl

/-- Wire projection of a closed O block run (private twin of the base
`proj_run_awire`, over the O blocks). -/
private theorem proj_run_awireO :
    ∀ (m a : Nat),
      proj (Chan.wire Party.R 0) false
        ((List.range' a m).flatMap (absorbBlockO ord))
      = seg (Chan.wire Party.R 0) false a m
  | 0, _ => rfl
  | m + 1, a => by
      rw [List.range'_succ, List.flatMap_cons, proj_append,
        proj_absorbBlockO_wire, proj_run_awireO m (a + 1),
        seg_append, Nat.add_comm 1 m]

/-- Leaf-request projection of a closed O block run. -/
private theorem proj_run_aleafO :
    ∀ (m a : Nat),
      proj Chan.leafRequests false
        ((List.range' a m).flatMap (absorbBlockO ord))
      = seg Chan.leafRequests false a m
  | 0, _ => rfl
  | m + 1, a => by
      rw [List.range'_succ, List.flatMap_cons, proj_append,
        proj_absorbBlockO_leaf, proj_run_aleafO m (a + 1),
        seg_append, Nat.add_comm 1 m]

/-- Level-0 projection of a closed O block run. -/
private theorem proj_run_alevelO :
    ∀ (m a : Nat),
      proj (Chan.level Party.I 0) true
        ((List.range' a m).flatMap (absorbBlockO ord))
      = seg (Chan.level Party.I 0) true a m
  | 0, _ => rfl
  | m + 1, a => by
      rw [List.range'_succ, List.flatMap_cons, proj_append,
        proj_absorbBlockO_level, proj_run_alevelO m (a + 1),
        seg_append, Nat.add_comm 1 m]

/-- THE O ABSORB SUFFIX TRICHOTOMY, twin of `absorb_cell_shape`: a
nonempty unemitted cell of the O absorb trace heads at its next wire
receive, leaf request, or level-0 send, with the emitted prefix's
counts pinned per block position — the two receive counts offset by
`ord.absorb.wirePhase` per the assignment's block order.

At `ord.absorb = .replyFirst` the offsets reduce to literals and the
statement is the base lemma's; the query-first corner re-runs the same
block-split analysis over the swapped block. -/
theorem absorb_cell_shapeO {pre r : List Ev}
    (hsplit : absorbEventsO sk ord = pre ++ r) (hne : r ≠ []) :
    ∃ t, t < sk.totalLeafReqs ∧
      (((∃ rest, r = (Chan.wire Party.R 0, false, t) :: rest)
        ∧ (proj (Chan.wire Party.R 0) false pre).length = t
        ∧ (proj Chan.leafRequests false pre).length
            = t + ord.absorb.wirePhase
        ∧ (proj (Chan.level Party.I 0) true pre).length = t)
      ∨ ((∃ rest, r = (Chan.leafRequests, false, t) :: rest)
        ∧ (proj (Chan.wire Party.R 0) false pre).length
            = t + (1 - ord.absorb.wirePhase)
        ∧ (proj Chan.leafRequests false pre).length = t
        ∧ (proj (Chan.level Party.I 0) true pre).length = t)
      ∨ ((∃ rest, r = (Chan.level Party.I 0, true, t) :: rest)
        ∧ (proj (Chan.wire Party.R 0) false pre).length = t + 1
        ∧ (proj Chan.leafRequests false pre).length = t + 1
        ∧ (proj (Chan.level Party.I 0) true pre).length = t)) := by
  cases hord : ord.absorb with
  | replyFirst =>
      -- the assignment collapses the trace: delegate to the base shape
      rw [absorbEventsO_rf_absorb sk ord hord] at hsplit
      obtain ⟨t, htN, hshape⟩ := absorb_cell_shape sk hsplit hne
      refine ⟨t, htN, ?_⟩
      simp only [PairOrder.wirePhase, Nat.add_zero, Nat.sub_zero]
      exact hshape
  | queryFirst =>
      unfold absorbEventsO at hsplit
      rw [List.range_eq_range'] at hsplit
      obtain ⟨t, -, htN, p₂, r₂, hblock, hr₂, hpre, hr⟩ :=
        prefix_flatMap _ 0 hsplit hne
      rw [Nat.zero_add] at htN
      rw [Nat.sub_zero] at hpre
      have hw_run := proj_run_awireO ord t 0
      have hl_run := proj_run_aleafO ord t 0
      have hv_run := proj_run_alevelO ord t 0
      refine ⟨t, htN, ?_⟩
      simp only [PairOrder.wirePhase, Nat.sub_self, Nat.add_zero]
      simp only [absorbBlockO, hord, List.cons_append, List.nil_append]
        at hblock
      match p₂, hblock with
      | [], hblock =>
          -- block boundary: the cell heads at the leaf request
          rw [List.nil_append] at hblock
          rw [List.append_nil] at hpre
          refine Or.inr (Or.inl ⟨⟨(Chan.wire Party.R 0, false, t)
              :: (Chan.level Party.I 0, true, t)
              :: (List.range' (t + 1)
                  (0 + sk.totalLeafReqs - t - 1)).flatMap
                  (absorbBlockO ord),
            ?_⟩, ?_, ?_, ?_⟩)
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
              -- the cell heads at the wire receive
              rw [List.nil_append] at hinner
              refine Or.inl ⟨⟨(Chan.level Party.I 0, true, t)
                  :: (List.range' (t + 1)
                      (0 + sk.totalLeafReqs - t - 1)).flatMap
                      (absorbBlockO ord),
                ?_⟩, ?_, ?_, ?_⟩
              · rw [hr, ← hinner]
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
          | e' :: p₄, hinner =>
              rw [List.cons_append] at hinner
              injection hinner with he2 hinner₂
              subst he2
              match p₄, hinner₂ with
              | [], hinner₂ =>
                  -- the cell heads at the level-0 send
                  rw [List.nil_append] at hinner₂
                  refine Or.inr (Or.inr ⟨⟨(List.range' (t + 1)
                      (0 + sk.totalLeafReqs - t - 1)).flatMap
                      (absorbBlockO ord),
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

/-- THE O ABSORB STUCK TRICHOTOMY, twin of `absorb_stuck`: at a pump
fixpoint the absorber is exhausted, wire-starved, request-starved, or
blocked on its level-0 output — with all three counts pinned and the
failed guard recorded, the two starved arms' receive offsets read off
`ord.absorb.wirePhase` (the block's first-received channel runs
`wirePhase` receives ahead of the wire).

At `ord.absorb = .replyFirst` the offsets are `0`/`1` and the arms are
the base `absorb_stuck`'s verbatim. Consumers close the starved arms
by `omega` with the offset symbolic — no per-assignment dispatch at
the call sites. -/
theorem absorb_stuckO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st)
    (hfix : step sk st = none) :
    (rcvCount (Chan.wire Party.R 0) st.out = sk.totalLeafReqs
      ∧ rcvCount Chan.leafRequests st.out = sk.totalLeafReqs
      ∧ sndCount (Chan.level Party.I 0) st.out = sk.totalLeafReqs)
    ∨ (rcvCount (Chan.wire Party.R 0) st.out < sk.totalLeafReqs
      ∧ rcvCount Chan.leafRequests st.out
          = rcvCount (Chan.wire Party.R 0) st.out
            + ord.absorb.wirePhase
      ∧ sndCount (Chan.level Party.I 0) st.out
          = rcvCount (Chan.wire Party.R 0) st.out
      ∧ sndCount (Chan.wire Party.R 0) st.out
          ≤ rcvCount (Chan.wire Party.R 0) st.out)
    ∨ (rcvCount Chan.leafRequests st.out < sk.totalLeafReqs
      ∧ rcvCount (Chan.wire Party.R 0) st.out
          = rcvCount Chan.leafRequests st.out
            + (1 - ord.absorb.wirePhase)
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
  have hIdx : P[2 + sk.rootH]? = some (absorbEventsO sk ord) :=
    famOKO_absorb sk ord hfam
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hIdx
  have hWc : rcvCount (Chan.wire Party.R 0) st.out
      = (proj (Chan.wire Party.R 0) false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_ownerO sk ord hfam h _ false (by simpa using hwo)
        hIdx hr hpre hsub]
  have hLc : rcvCount Chan.leafRequests st.out
      = (proj Chan.leafRequests false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_ownerO sk ord hfam h _ false (by simpa using hlo)
        hIdx hr hpre hsub]
  have hVc : sndCount (Chan.level Party.I 0) st.out
      = (proj (Chan.level Party.I 0) true pre).length := by
    rw [sndCount_eq_proj,
      out_proj_ownerO sk ord hfam h _ true (by simpa using hvo)
        hIdx hr hpre hsub]
  cases r with
  | nil =>
      -- exhausted: the emitted prefix is the whole trace
      rw [List.append_nil] at hpre
      obtain ⟨ht1, ht2, ht3⟩ := absorb_totalsO sk ord
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
      obtain ⟨t, htN, hshape⟩ := absorb_cell_shapeO sk ord hpre (by simp)
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

/-- Fins as one resolution then the returns segment (private twin of
the base `finEvents_eq`). -/
private theorem finEvents_eq :
    finEvents sk = (Chan.rootres, false, 0)
      :: seg Chan.rootrets false 0 sk.rootPending := by
  unfold finEvents seg
  rw [List.range_eq_range']
  simp

/-- The two fins channels disagree (private twin of the base
`rootres_ne_rootrets`). -/
private theorem rootres_ne_rootrets :
    (Chan.rootres : Chan) ≠ Chan.rootrets := by simp

/-- FINS STUCKNESS, O twin of `fin_stuck`: the fins row is
order-independent, so statement and arms are the base lemma's over the
O bundle. -/
theorem fin_stuckO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st)
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
  have hIdx : P[3 * sk.rootH + 3]? = some (finEvents sk) :=
    famOKO_fin sk ord hfam hge
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hIdx
  have hAc : rcvCount Chan.rootres st.out
      = (proj Chan.rootres false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_ownerO sk ord hfam h _ false (by simp [rcvOwner])
        hIdx hr hpre hsub]
  have hBc : rcvCount Chan.rootrets st.out
      = (proj Chan.rootrets false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_ownerO sk ord hfam h _ false (by simp [rcvOwner])
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

/-- ROOTRET STUCKNESS, O twin of `rootret_stuck`: the floating
`rootret` receive has either fired or is starved. -/
theorem rootret_stuckO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st)
    (hfix : step sk st = none) (hge : 1 ≤ sk.rootH) :
    rcvCount Chan.rootret st.out = 1
    ∨ (rcvCount Chan.rootret st.out = 0
      ∧ sndCount Chan.rootret st.out = 0) := by
  have hIdx : P[3 * sk.rootH + 2]? = some [((Chan.rootret, false, 0) : Ev)] :=
    famOKO_rootret sk ord hfam hge
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hIdx
  have hAc : rcvCount Chan.rootret st.out
      = (proj Chan.rootret false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_ownerO sk ord hfam h _ false (by simp [rcvOwner])
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

end StreamingMirror.Ord
