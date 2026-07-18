/-
Weave edge-respect, the generic layer (PROGRESS.md §7 3b): the `WEdge`
invariant — the counting layer plus `MInv`'s guard-history fields —
and its preservation through everything except the guards themselves.

# Design of record for the edge-respect campaign

The weave's pump emissions go through `scan`, which checks `enabled`;
their guard-history is free (`wEdge_step`, the same argument as
`step_preserves`). The whole protocol content is the MANUAL emission
points, where `wEmit` appends unconditionally. Enumerating them by
channel (`Model.lean`'s wiring; caps are 1 everywhere except `level`):

- **E1, prologue receives** (`wireIn`, `askedIn`): both consume
  MANUAL sends — the parent stage's wire (`wireIn (wpk h) =
  wireOut (wpk (h+1))`), the feed query two stages up (`askedIn
  (wpk h)` is fed by `askedOut (wpk (h+2))`, or by the openers at the
  top). Weave order emits the send just before descending, so these
  are count arithmetic over the weave position — no pump, no
  `schedulable`.
- **E2, cap-1 manual-consumer sends** (`wireOut` at `h ≥ 1`,
  `askedOut` landing at stages `≥ 0`): the consumer is the lower
  stage's prologue, one receive per scope, and the weave opens scope
  `k` right after emitting its wire/query — count arithmetic again.
- **E2, pump-consumer sends** — the three real obligations, each a
  PUMP-PROGRESS lemma at the emission point:
  - `upperOut (wpk h)` seq `k`: the asker assembler `(p, h+1)`
    consumes `upper p h`; need it to have received all `k` earlier
    summaries.
  - `lowerOut (wpk h)` seq `d`: the answerer assembler `(p, h)`
    consumes `lower p h`; need `d` resolutions received.
  - `wireOut (wpk 0)` and `leafRequests`: absorb consumes both; need
    absorb's loop to have kept pace.

  Each is proven against `wPump_fixpoint` (this file): after the
  greedy pump the state admits NO step, so if the pump trace had not
  progressed past the needed receive, its head would be enabled —
  contradiction. Unwinding "its head would be enabled" up the asm
  towers crosses the `level` channels' `capLevel` windows: THAT is
  where `Skel.schedulable` (`dCount ≤ capLevel + 2`) enters, and
  nowhere else. E2-level itself never appears at a manual emission
  (level channels are pump-to-pump).

The layers above this file:

- Count characterization (layer B): per-channel counts at scope
  entry/exit as closed forms of the weave position (`wiresBefore`,
  `qsBefore`, scope index), derived from `WCount`'s `out_count`.
- Pump progress (layer C): the fixpoint-contradiction lemmas above,
  by induction up the asm towers, under `Skel.schedulable`.
- The master induction (layer D): `align_scope`-style Hoare triples
  over the scope tree — precondition/postcondition = layer B's count
  forms — discharging `wEdge_emit`'s `enabled` hypothesis at every
  manual point; top assembly yields `WEdge sk [] (weaveState sk)`
  under `wellFormed ∧ schedulable`, i.e. the weave is a VALID
  schedule, the potential the completeness argmin consumes.

Chain (both corners, stage B): provides `WEdgeP` (+ `WEdge`, the d5
abbrev) and its generic preservation to Master/MasterE; Final/FinalE
read the drained invariant back out. Map: Proofs/Map.lean.
-/
import StreamingMirror.Proofs.Sched.Weave.Count

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ================================================== the pump fixpoint

/-- A successful scan consumes exactly one event from the remainders. -/
theorem scan_sum (sent rcvd : Chan → Nat) :
    ∀ {ts : List (List Ev)} {e : Ev} {ts' : List (List Ev)},
      scan sk sent rcvd ts = some (e, ts') →
      (ts'.map List.length).sum + 1 = (ts.map List.length).sum := by
  intro ts
  induction ts with
  | nil => intro e ts' h; simp [scan] at h
  | cons t ts ih =>
      intro e ts' h
      match t with
      | [] =>
          cases hrec : scan sk sent rcvd ts with
          | none => rw [scan, hrec] at h; simp at h
          | some pr =>
              obtain ⟨e', ts₁⟩ := pr
              rw [scan, hrec] at h
              simp only [Option.map] at h
              cases h
              have hs := ih hrec
              simp only [List.map_cons, List.sum_cons, List.length_nil]
              omega
      | ev :: rest =>
          by_cases hen : enabled sk sent rcvd ev = true
          · rw [scan, if_pos hen] at h
            cases h
            simp only [List.map_cons, List.sum_cons, List.length_cons]
            omega
          · cases hrec : scan sk sent rcvd ts with
            | none => rw [scan, if_neg hen, hrec] at h; simp at h
            | some pr =>
                obtain ⟨e', ts₁⟩ := pr
                rw [scan, if_neg hen, hrec] at h
                simp only [Option.map] at h
                cases h
                have hs := ih hrec
                simp only [List.map_cons, List.sum_cons, List.length_cons]
                omega

/-- Drained remainders scan to nothing. -/
theorem scan_none_of_all_nil (sent rcvd : Chan → Nat) :
    ∀ {ts : List (List Ev)}, (∀ t ∈ ts, t = []) →
      scan sk sent rcvd ts = none := by
  intro ts
  induction ts with
  | nil => intro _; rfl
  | cons t ts ih =>
      intro h
      have ht : t = [] := h t (List.mem_cons_self ..)
      subst ht
      rw [scan, ih fun t' ht' => h t' (List.mem_cons_of_mem _ ht')]
      rfl

/-- Zero total length means every remainder is empty. -/
private theorem all_nil_of_sum_zero :
    ∀ {ts : List (List Ev)}, (ts.map List.length).sum = 0 →
      ∀ t ∈ ts, t = []
  | [], _, t, ht => by cases ht
  | t₀ :: ts', h, t, ht => by
      simp only [List.map_cons, List.sum_cons] at h
      rcases List.mem_cons.1 ht with rfl | ht'
      · cases t with
        | nil => rfl
        | cons a t' => simp only [List.length_cons] at h; omega
      · exact all_nil_of_sum_zero (by omega) t ht'

/-- With fuel at least the remainders' total length, the merge runs to
its FIXPOINT: the resulting state admits no further step.

This is the stuckness fact the pump-progress lemmas consume: after
`wPump`, any pump head that WOULD be enabled has already been
consumed, so un-progressed pumps yield contradictions. -/
theorem mergeN_fixpoint :
    ∀ (fuel : Nat) (st : MState),
      (st.rem.map List.length).sum ≤ fuel →
      step sk (mergeN sk fuel st) = none := by
  intro fuel
  induction fuel with
  | zero =>
      intro st hfuel
      show step sk st = none
      unfold step
      rw [scan_none_of_all_nil sk _ _
        (all_nil_of_sum_zero (by omega))]
      rfl
  | succ f ih =>
      intro st hfuel
      unfold mergeN
      cases hstep : step sk st with
      | none => exact hstep
      | some st' =>
          refine ih st' ?_
          unfold step at hstep
          cases hscan : scan sk st.sent st.rcvd st.rem with
          | none => rw [hscan] at hstep; simp at hstep
          | some pr =>
              obtain ⟨e, rem'⟩ := pr
              rw [hscan] at hstep
              simp only [Option.map] at hstep
              have hsum := scan_sum sk st.sent st.rcvd hscan
              obtain ⟨c, sd, n⟩ := e
              cases sd with
              | true =>
                  cases hstep
                  show (rem'.map List.length).sum ≤ f
                  omega
              | false =>
                  cases hstep
                  show (rem'.map List.length).sum ≤ f
                  omega

/-- The greedy pump runs to the merge's fixpoint. -/
theorem wPump_fixpoint (st : MState) : step sk (wPump sk st) = none :=
  mergeN_fixpoint sk _ st (Nat.le_refl _)

-- ===================================================== the invariant

/-- The weave validity invariant: the counting layer plus edge-respect
in counted guard-history form.

The two history fields are verbatim `MInv.e1_hist`/`e2_hist`, now for
weave states: at every receive's position its send count had already
passed its seq, at every send's position its cap window was open.
Preservation through the pump is free (`scan` emits only enabled
events); a manual emission carries an `enabled` HYPOTHESIS
(`wEdge_emit`) — discharging it at each of the weave's emission
points, under `Skel.schedulable`, is the master induction's content
(see the module doc). -/
structure WEdgeP (P : List (List Ev)) (fut : List Ev) (st : MState) :
    Prop extends WCountP sk P fut st where
  e1_hist : ∀ k c n, st.out[k]? = some (c, false, n) →
    n < sndCount c (st.out.take k)
  e2_hist : ∀ k c n, st.out[k]? = some (c, true, n) →
    n < rcvCount c (st.out.take k) + sk.cap c

/-- The d5 corner's instance of the validity invariant (cf. `WCount`):
the same guard-history layer at the merge's own trace family. -/
abbrev WEdge (fut : List Ev) (st : MState) : Prop :=
  WEdgeP sk (procs sk) fut st

/-- Taking at most the left part of an append never sees the right. -/
private theorem take_append_le {α : Type _} :
    ∀ (n : Nat) (l₁ l₂ : List α), n ≤ l₁.length →
      (l₁ ++ l₂).take n = l₁.take n
  | 0, _, _, _ => by simp
  | n + 1, [], _, h => by simp at h
  | n + 1, a :: l₁, l₂, h => by
      simp only [List.cons_append, List.take_succ_cons]
      rw [take_append_le n l₁ l₂ (by simpa using h)]

/-- Appending an ENABLED event preserves guard-history: old positions
keep their prefixes, and the new position's guard is the enabledness
check itself, read through the counter agreement. -/
private theorem hist_extend {out : List Ev} {e : Ev}
    {sent rcvd : Chan → Nat}
    (hsent : ∀ c, sent c = sndCount c out)
    (hrcvd : ∀ c, rcvd c = rcvCount c out)
    (hen : enabled sk sent rcvd e = true)
    (h1 : ∀ k c n, out[k]? = some (c, false, n) →
      n < sndCount c (out.take k))
    (h2 : ∀ k c n, out[k]? = some (c, true, n) →
      n < rcvCount c (out.take k) + sk.cap c) :
    (∀ k c n, (out ++ [e])[k]? = some (c, false, n) →
      n < sndCount c ((out ++ [e]).take k)) ∧
    (∀ k c n, (out ++ [e])[k]? = some (c, true, n) →
      n < rcvCount c ((out ++ [e]).take k) + sk.cap c) := by
  constructor
  · intro k c n hk
    rcases Nat.lt_or_ge k out.length with hlt | hge
    · rw [List.getElem?_append_left hlt] at hk
      rw [take_append_le _ _ _ (Nat.le_of_lt hlt)]
      exact h1 k c n hk
    · rw [List.getElem?_append_right hge] at hk
      cases hm : k - out.length with
      | zero =>
          rw [hm] at hk
          simp only [List.getElem?_cons_zero, Option.some.injEq] at hk
          subst hk
          have hkl : k = out.length := by omega
          subst hkl
          rw [take_len_append]
          have hs := hsent c
          simp only [enabled, decide_eq_true_eq] at hen
          omega
      | succ m => rw [hm] at hk; simp at hk
  · intro k c n hk
    rcases Nat.lt_or_ge k out.length with hlt | hge
    · rw [List.getElem?_append_left hlt] at hk
      rw [take_append_le _ _ _ (Nat.le_of_lt hlt)]
      exact h2 k c n hk
    · rw [List.getElem?_append_right hge] at hk
      cases hm : k - out.length with
      | zero =>
          rw [hm] at hk
          simp only [List.getElem?_cons_zero, Option.some.injEq] at hk
          subst hk
          have hkl : k = out.length := by omega
          subst hkl
          rw [take_len_append]
          have hr := hrcvd c
          simp only [enabled, decide_eq_true_eq] at hen
          omega
      | succ m => rw [hm] at hk; simp at hk

-- ============================================== preservation lemmas

/-- The weave's starting state, generic over the trace family:
counting from the initial alignment, guard-history vacuously (nothing
emitted). -/
theorem wEdge_initP {P : List (List Ev)} {fut : List Ev}
    (halign : manFilters sk fut = P.take (manCount sk))
    (hpumps : P.drop (manCount sk) = weavePumps sk)
    (howners : ∀ e ∈ fut, evOwner sk e < manCount sk) :
    WEdgeP sk P fut (weaveInit sk) := by
  refine ⟨wcount_init sk halign hpumps howners, ?_, ?_⟩
  · intro k c n h; simp [weaveInit] at h
  · intro k c n h; simp [weaveInit] at h

/-- The weave's starting state, d5 spelling. -/
theorem wEdge_init {fut : List Ev}
    (halign : manFilters sk fut = (procs sk).take (manCount sk))
    (howners : ∀ e ∈ fut, evOwner sk e < manCount sk) :
    WEdge sk fut (weaveInit sk) :=
  wEdge_initP sk halign (weavePumps_eq sk).symm howners

/-- A manual emission preserves the full invariant, GIVEN its guard is
open — the hypothesis the master induction discharges at each of the
weave's emission points. -/
theorem wEdge_emit {P : List (List Ev)} {fut : List Ev} {st : MState}
    {e : Ev}
    (hen : enabled sk st.sent st.rcvd e = true)
    (hinv : WEdgeP sk P (e :: fut) st) :
    WEdgeP sk P fut (wEmit st e) := by
  obtain ⟨h1, h2⟩ := hist_extend sk hinv.sent_eq hinv.rcvd_eq hen
    hinv.e1_hist hinv.e2_hist
  refine ⟨wEmit_preserves sk hinv.toWCountP, ?_, ?_⟩
  · rw [wEmit_out]; exact h1
  · rw [wEmit_out]; exact h2

/-- One pump step preserves the full invariant: the merge emits only
enabled events, so its guard-history extends for free. -/
theorem wEdge_step {P : List (List Ev)} {fut : List Ev} {st st' : MState}
    (hinv : WEdgeP sk P fut st) (hstep : step sk st = some st') :
    WEdgeP sk P fut st' := by
  have hcount := wStep_preserves sk hinv.toWCountP hstep
  unfold step at hstep
  cases hscan : scan sk st.sent st.rcvd st.rem with
  | none => rw [hscan] at hstep; simp at hstep
  | some pr =>
      obtain ⟨e, rem'⟩ := pr
      rw [hscan] at hstep
      simp only [Option.map] at hstep
      obtain ⟨hen, -, -⟩ := scan_step sk st.out st.sent st.rcvd
        hinv.pump_struct hscan
      obtain ⟨h1, h2⟩ := hist_extend sk hinv.sent_eq hinv.rcvd_eq hen
        hinv.e1_hist hinv.e2_hist
      obtain ⟨c, sd, n⟩ := e
      cases sd with
      | true => cases hstep; exact ⟨hcount, h1, h2⟩
      | false => cases hstep; exact ⟨hcount, h1, h2⟩

/-- The full invariant survives any amount of pump fuel. -/
theorem wEdge_mergeN {P : List (List Ev)} {fut : List Ev} (fuel : Nat)
    {st : MState}
    (hinv : WEdgeP sk P fut st) :
    WEdgeP sk P fut (mergeN sk fuel st) := by
  induction fuel generalizing st with
  | zero => exact hinv
  | succ f ih =>
      unfold mergeN
      cases hstep : step sk st with
      | some st' => exact ih (wEdge_step sk hinv hstep)
      | none => exact hinv

/-- The full invariant survives the greedy pump. -/
theorem wEdge_pump {P : List (List Ev)} {fut : List Ev} {st : MState}
    (hinv : WEdgeP sk P fut st) : WEdgeP sk P fut (wPump sk st) :=
  wEdge_mergeN sk _ hinv

/-- Emit-then-pump, under the emission's guard. -/
theorem wEdge_emitP {P : List (List Ev)} {fut : List Ev} {st : MState}
    {e : Ev}
    (hen : enabled sk st.sent st.rcvd e = true)
    (hinv : WEdgeP sk P (e :: fut) st) :
    WEdgeP sk P fut (wEmitP sk st e) :=
  wEdge_pump sk (wEdge_emit sk hen hinv)

-- =========================== canonical projections of a weave state
-- The discharge toolkit: at any state satisfying `WCount`, `out`'s
-- per-channel-side projections are canonical, so every guard reduces
-- to a MEMBERSHIP claim — the predecessor event is already out — and
-- membership follows from conservation (it is in some trace, not in
-- the remaining future, not in any pump remainder).

/-- Every right-list member has a related left partner: the mirror of
`Forall2.exists_of_mem_left`. -/
theorem Forall2.exists_of_mem_right {α β : Type _} {R : α → β → Prop} :
    ∀ {la : List α} {lb : List β}, Forall2 R la lb → ∀ {b}, b ∈ lb →
      ∃ a ∈ la, R a b
  | _, _, .cons hab t, b, hb => by
      rcases List.mem_cons.1 hb with rfl | hb'
      · exact ⟨_, List.mem_cons_self .., hab⟩
      · obtain ⟨a, ha, hr⟩ := t.exists_of_mem_right hb'
        exact ⟨a, List.mem_cons_of_mem _ ha, hr⟩

/-- The counting invariant's two halves, glued back into one pointwise
relation over the whole trace family. -/
theorem wcount_glue {P : List (List Ev)} {fut : List Ev} {st : MState}
    (h : WCountP sk P fut st) :
    Forall2 (fun t r => ∃ pre, t = pre ++ r ∧ pre.Sublist st.out)
      P (manFilters sk fut ++ st.rem) := by
  have hall := Forall2.append h.man_struct h.pump_struct
  rwa [List.take_append_drop] at hall

/-- `out_count`, read over the glued family. -/
theorem wcount_out_glued {P : List (List Ev)} {fut : List Ev}
    {st : MState}
    (h : WCountP sk P fut st) (p : Ev → Bool) :
    (st.out.filter p).length
      = emittedCount p P (manFilters sk fut ++ st.rem) := by
  have hEC : emittedCount p P (manFilters sk fut ++ st.rem)
      = emittedCount p (P.take (manCount sk))
          (manFilters sk fut)
        + emittedCount p (P.drop (manCount sk)) st.rem := by
    conv => lhs; rw [← List.take_append_drop (manCount sk) P]
    exact emittedCount_append p _ _ h.man_struct.length_eq
  rw [hEC]
  exact h.out_count p

/-- Weave-state projections are CANONICAL, generic form: the family's
ownership and per-trace canon-shape are hypotheses, discharged at each
corner by its numbering layer (`procs_*`/`procsE_*`). -/
theorem wproj_canonP {P : List (List Ev)} {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) (c : Chan) (b : Bool)
    (howned : Owned (if b then sndOwner sk else rcvOwner sk) b 0 P)
    (hcanon : ∀ t ∈ P, ∃ m, proj c b t = canon c b m) :
    proj c b st.out = canon c b (proj c b st.out).length := by
  obtain ⟨pre, hsub, hpre⟩ :=
    emitted_canon (wcount_glue sk h) howned hcanon
  have hcount : (proj c b st.out).length
      = emittedCount (fun e => decide (e.1 = c) && (e.2.1 == b))
          P (manFilters sk fut ++ st.rem) :=
    wcount_out_glued sk h _
  have hlenpre : (proj c b pre).length
      = emittedCount (fun e => decide (e.1 = c) && (e.2.1 == b))
          P (manFilters sk fut ++ st.rem) := by
    rw [hpre]
    simp [canon]
  have heq : proj c b pre = proj c b st.out :=
    (hsub.filter _).eq_of_length (by
      show (proj c b pre).length = (proj c b st.out).length
      rw [hlenpre, hcount])
  rw [hcount, ← heq]
  exact hpre

/-- Weave-state projections are canonical, d5 spelling. -/
theorem wproj_canon (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) (c : Chan) (b : Bool) :
    proj c b st.out = canon c b (proj c b st.out).length := by
  refine wproj_canonP sk h c b ?_ (procs_canon sk c b)
  cases b
  · exact procs_rcv_owned sk hwf
  · exact procs_snd_owned sk hwf

/-- Membership bounds the count, generic form (cf. `wproj_canonP`). -/
theorem wcount_mem_ltP {P : List (List Ev)} {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) {c : Chan} {b : Bool}
    {n : Nat}
    (howned : Owned (if b then sndOwner sk else rcvOwner sk) b 0 P)
    (hcanon : ∀ t ∈ P, ∃ m, proj c b t = canon c b m)
    (hmem : ((c, b, n) : Ev) ∈ st.out) :
    n < (proj c b st.out).length := by
  have hp : ((c, b, n) : Ev) ∈ proj c b st.out :=
    List.mem_filter.2 ⟨hmem, by simp⟩
  rw [wproj_canonP sk h c b howned hcanon] at hp
  simp only [canon, List.mem_map] at hp
  obtain ⟨j, hj, hje⟩ := hp
  have hn : j = n := by
    simpa using congrArg (fun e : Ev => e.2.2) hje
  subst hn
  exact List.mem_range.1 hj

/-- Membership bounds the count, d5 spelling. -/
theorem wcount_mem_lt (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {c : Chan} {b : Bool} {n : Nat}
    (hmem : ((c, b, n) : Ev) ∈ st.out) :
    n < (proj c b st.out).length := by
  refine wcount_mem_ltP sk h ?_ (procs_canon sk c b) hmem
  cases b
  · exact procs_rcv_owned sk hwf
  · exact procs_snd_owned sk hwf

-- ================================================ guard discharges

/-- E1 discharge, generic form: a receive is enabled once its own-seq
send is out. -/
theorem enabled_rcv_of_memP {P : List (List Ev)} {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) {c : Chan} {n : Nat}
    (howned : Owned (sndOwner sk) true 0 P)
    (hcanon : ∀ t ∈ P, ∃ m, proj c true t = canon c true m)
    (hmem : ((c, true, n) : Ev) ∈ st.out) :
    enabled sk st.sent st.rcvd (c, false, n) = true := by
  simp only [enabled, decide_eq_true_eq]
  rw [h.sent_eq c, sndCount_eq_proj]
  exact wcount_mem_ltP sk h (by simpa using howned) hcanon hmem

/-- E1 discharge, d5 spelling. -/
theorem enabled_rcv_of_mem (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {c : Chan} {n : Nat}
    (hmem : ((c, true, n) : Ev) ∈ st.out) :
    enabled sk st.sent st.rcvd (c, false, n) = true := by
  simp only [enabled, decide_eq_true_eq]
  rw [h.sent_eq c, sndCount_eq_proj]
  exact wcount_mem_lt sk hwf h hmem

/-- E2 discharge, seq under the cap: the window opens unconditionally. -/
theorem enabled_snd_low {st : MState} {c : Chan} {n : Nat}
    (hn : n < sk.cap c) :
    enabled sk st.sent st.rcvd (c, true, n) = true := by
  simp only [enabled, decide_eq_true_eq]
  omega

/-- E2 discharge, generic form: a send is enabled once the receive
sitting `cap` slots below its seq is out. -/
theorem enabled_snd_of_memP {P : List (List Ev)} {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) {c : Chan} {n : Nat}
    (howned : Owned (rcvOwner sk) false 0 P)
    (hcanon : ∀ t ∈ P, ∃ m, proj c false t = canon c false m)
    (hmem : ((c, false, n - sk.cap c) : Ev) ∈ st.out)
    (hn : sk.cap c ≤ n) :
    enabled sk st.sent st.rcvd (c, true, n) = true := by
  simp only [enabled, decide_eq_true_eq]
  rw [h.rcvd_eq c, rcvCount_eq_proj]
  have := wcount_mem_ltP sk h (by simpa using howned) hcanon hmem
  omega

/-- E2 discharge, d5 spelling. -/
theorem enabled_snd_of_mem (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {c : Chan} {n : Nat}
    (hmem : ((c, false, n - sk.cap c) : Ev) ∈ st.out)
    (hn : sk.cap c ≤ n) :
    enabled sk st.sent st.rcvd (c, true, n) = true := by
  simp only [enabled, decide_eq_true_eq]
  rw [h.rcvd_eq c, rcvCount_eq_proj]
  have := wcount_mem_lt sk hwf h hmem
  omega

/-- CONSERVATION: an event of some trace that is neither in the
remaining future nor in any pump remainder has been emitted. -/
theorem mem_out_of_elsewhere {P : List (List Ev)} {fut : List Ev}
    {st : MState}
    (h : WCountP sk P fut st) {e : Ev} {t : List Ev}
    (ht : t ∈ P) (het : e ∈ t)
    (hfut : e ∉ fut) (hrem : ∀ r ∈ st.rem, e ∉ r) :
    e ∈ st.out := by
  obtain ⟨r, hr, pre, hpre, hsub⟩ :=
    (wcount_glue sk h).exists_of_mem_left ht
  rw [hpre] at het
  rcases List.mem_append.1 het with hin | hin
  · exact hsub.subset hin
  · rcases List.mem_append.1 hr with hr' | hr'
    · unfold manFilters at hr'
      obtain ⟨m, -, rfl⟩ := List.mem_map.1 hr'
      exact absurd (List.mem_filter.1 hin).1 hfut
    · exact absurd hin (hrem r hr')

-- ============================================== pump-family support
-- Which channels the pump traces can touch at all: never a walk wire
-- above the leaf stage, never an `asked` channel. These make every
-- manual-vs-manual predecessor's "not in any pump remainder"
-- obligation a support fact.

private theorem asmOut_cases (pk : Party × Nat) :
    sk.asmOutChan pk = Chan.rootret ∨ sk.asmOutChan pk = Chan.rootrets
      ∨ ∃ p j, sk.asmOutChan pk = Chan.level p j := by
  unfold Skel.asmOutChan
  split
  · exact Or.inl rfl
  · split
    · exact Or.inr (Or.inl rfl)
    · exact Or.inr (Or.inr ⟨_, _, rfl⟩)

private theorem asmRes_cases (pk : Party × Nat) :
    (∃ p j, asmResChan pk = Chan.upper p j)
      ∨ ∃ p j, asmResChan pk = Chan.lower p j := by
  unfold asmResChan
  split
  · exact Or.inl ⟨_, _, rfl⟩
  · exact Or.inr ⟨_, _, rfl⟩

/-- Support of the pump family: a pump event's channel is never a
walk wire above the leaf stage (the only pump wire traffic is
absorb's `wire R 0` RECEIVES) and never an `asked` channel. Stated
over `weavePumps` — both corners' trace families drop to exactly it. -/
theorem pump_support {t : List Ev}
    (ht : t ∈ weavePumps sk) {e : Ev} (he : e ∈ t) :
    (∀ p hh, e.1 = Chan.wire p hh → hh = 0 ∧ e.2.1 = false) ∧
    ∀ p hh, e.1 ≠ Chan.asked p hh := by
  simp only [weavePumps, List.mem_append, List.mem_cons, List.mem_map,
    List.not_mem_nil, or_false] at ht
  rcases ht with (rfl | ⟨pk, -, rfl⟩) | rfl | rfl
  · -- absorb
    obtain ⟨j, -, he⟩ := List.mem_flatMap.1 he
    rcases he with _ | ⟨_, he⟩
    · refine ⟨fun p hh hw => ?_, fun p hh hw => nomatch hw⟩
      injection hw with h1 h2
      exact ⟨h2.symm, rfl⟩
    · rcases he with _ | ⟨_, he⟩
      · exact ⟨fun p hh hw => (nomatch hw), fun p hh hw => nomatch hw⟩
      · rcases he with _ | ⟨_, he⟩
        · exact ⟨fun p hh hw => (nomatch hw), fun p hh hw => nomatch hw⟩
        · cases he
  · -- an asm tower
    have hsup := asmEvents_support sk pk e he
    constructor
    · intro p hh hw
      cases hb : e.2.1 with
      | true =>
          have hc := hsup.1 hb
          rw [hc] at hw
          rcases asmOut_cases sk pk with h | h | ⟨p', j', h⟩ <;>
            rw [h] at hw <;> exact nomatch hw
      | false =>
          rcases hsup.2 hb with hc | hc
          · rw [hc] at hw
            rcases asmRes_cases pk with ⟨p', j', h⟩ | ⟨p', j', h⟩ <;>
              rw [h] at hw <;> exact nomatch hw
          · rw [hc] at hw
            exact nomatch hw
    · intro p hh hw
      cases hb : e.2.1 with
      | true =>
          have hc := hsup.1 hb
          rw [hc] at hw
          rcases asmOut_cases sk pk with h | h | ⟨p', j', h⟩ <;>
            rw [h] at hw <;> exact nomatch hw
      | false =>
          rcases hsup.2 hb with hc | hc
          · rw [hc] at hw
            rcases asmRes_cases pk with ⟨p', j', h⟩ | ⟨p', j', h⟩ <;>
              rw [h] at hw <;> exact nomatch hw
          · rw [hc] at hw
            exact nomatch hw
  · -- the floating rootret receive
    rcases he with _ | ⟨_, he⟩
    · exact ⟨fun p hh hw => (nomatch hw), fun p hh hw => nomatch hw⟩
    · cases he
  · -- fins
    unfold finEvents at he
    rcases he with _ | ⟨_, he⟩
    · exact ⟨fun p hh hw => (nomatch hw), fun p hh hw => nomatch hw⟩
    · obtain ⟨j, -, rfl⟩ := List.mem_map.1 he
      exact ⟨fun p hh hw => (nomatch hw), fun p hh hw => nomatch hw⟩

/-- Pump remainders never hold a wire event above the leaf stage's
receives. Generic over the trace family: both corners' families have
`weavePumps` as their pump half, which `hpumps` records. -/
theorem pump_rem_no_wireP {P : List (List Ev)} {fut : List Ev}
    {st : MState}
    (h : WCountP sk P fut st)
    (hpumps : P.drop (manCount sk) = weavePumps sk)
    {p : Party} {hh n : Nat} {b : Bool}
    (hb : hh ≠ 0 ∨ b = true) :
    ∀ r ∈ st.rem, ((Chan.wire p hh, b, n) : Ev) ∉ r := by
  intro r hr hmem
  obtain ⟨t, ht, pre, hpre, -⟩ :=
    h.pump_struct.exists_of_mem_right hr
  rw [hpumps] at ht
  have het : ((Chan.wire p hh, b, n) : Ev) ∈ t := by
    rw [hpre]; exact List.mem_append_right _ hmem
  have hcl := (pump_support sk ht het).1 p hh rfl
  rcases hb with h0 | h1
  · exact h0 hcl.1
  · rw [show ((Chan.wire p hh, b, n) : Ev).2.1 = b from rfl] at hcl
    rw [h1] at hcl
    exact Bool.noConfusion hcl.2

/-- Pump remainders never hold a wire event, d5 spelling. -/
theorem pump_rem_no_wire {fut : List Ev} {st : MState}
    (h : WCount sk fut st) {p : Party} {hh n : Nat} {b : Bool}
    (hb : hh ≠ 0 ∨ b = true) :
    ∀ r ∈ st.rem, ((Chan.wire p hh, b, n) : Ev) ∉ r :=
  pump_rem_no_wireP sk h (weavePumps_eq sk).symm hb

/-- Pump remainders never hold an `asked` event, generic form. -/
theorem pump_rem_no_askedP {P : List (List Ev)} {fut : List Ev}
    {st : MState}
    (h : WCountP sk P fut st)
    (hpumps : P.drop (manCount sk) = weavePumps sk)
    {p : Party} {hh n : Nat} {b : Bool} :
    ∀ r ∈ st.rem, ((Chan.asked p hh, b, n) : Ev) ∉ r := by
  intro r hr hmem
  obtain ⟨t, ht, pre, hpre, -⟩ :=
    h.pump_struct.exists_of_mem_right hr
  rw [hpumps] at ht
  have het : ((Chan.asked p hh, b, n) : Ev) ∈ t := by
    rw [hpre]; exact List.mem_append_right _ hmem
  exact absurd rfl ((pump_support sk ht het).2 p hh)

/-- Pump remainders never hold an `asked` event, d5 spelling. -/
theorem pump_rem_no_asked {fut : List Ev} {st : MState}
    (h : WCount sk fut st) {p : Party} {hh n : Nat} {b : Bool} :
    ∀ r ∈ st.rem, ((Chan.asked p hh, b, n) : Ev) ∉ r :=
  pump_rem_no_askedP sk h (weavePumps_eq sk).symm

end StreamingMirror.Sched
