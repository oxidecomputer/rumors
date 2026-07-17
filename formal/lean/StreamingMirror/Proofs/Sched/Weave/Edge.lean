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
structure WEdge (fut : List Ev) (st : MState) : Prop
    extends WCount sk fut st where
  e1_hist : ∀ k c n, st.out[k]? = some (c, false, n) →
    n < sndCount c (st.out.take k)
  e2_hist : ∀ k c n, st.out[k]? = some (c, true, n) →
    n < rcvCount c (st.out.take k) + sk.cap c

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

/-- The weave's starting state: counting from the initial alignment,
guard-history vacuously (nothing emitted). -/
theorem wEdge_init {fut : List Ev}
    (halign : manFilters sk fut = (procs sk).take (manCount sk))
    (howners : ∀ e ∈ fut, evOwner sk e < manCount sk) :
    WEdge sk fut (weaveInit sk) := by
  refine ⟨wcount_init sk halign howners, ?_, ?_⟩
  · intro k c n h; simp [weaveInit] at h
  · intro k c n h; simp [weaveInit] at h

/-- A manual emission preserves the full invariant, GIVEN its guard is
open — the hypothesis the master induction discharges at each of the
weave's emission points. -/
theorem wEdge_emit {fut : List Ev} {st : MState} {e : Ev}
    (hen : enabled sk st.sent st.rcvd e = true)
    (hinv : WEdge sk (e :: fut) st) :
    WEdge sk fut (wEmit st e) := by
  obtain ⟨h1, h2⟩ := hist_extend sk hinv.sent_eq hinv.rcvd_eq hen
    hinv.e1_hist hinv.e2_hist
  refine ⟨wEmit_preserves sk hinv.toWCount, ?_, ?_⟩
  · rw [wEmit_out]; exact h1
  · rw [wEmit_out]; exact h2

/-- One pump step preserves the full invariant: the merge emits only
enabled events, so its guard-history extends for free. -/
theorem wEdge_step {fut : List Ev} {st st' : MState}
    (hinv : WEdge sk fut st) (hstep : step sk st = some st') :
    WEdge sk fut st' := by
  have hcount := wStep_preserves sk hinv.toWCount hstep
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
theorem wEdge_mergeN {fut : List Ev} (fuel : Nat) {st : MState}
    (hinv : WEdge sk fut st) : WEdge sk fut (mergeN sk fuel st) := by
  induction fuel generalizing st with
  | zero => exact hinv
  | succ f ih =>
      unfold mergeN
      cases hstep : step sk st with
      | some st' => exact ih (wEdge_step sk hinv hstep)
      | none => exact hinv

/-- The full invariant survives the greedy pump. -/
theorem wEdge_pump {fut : List Ev} {st : MState}
    (hinv : WEdge sk fut st) : WEdge sk fut (wPump sk st) :=
  wEdge_mergeN sk _ hinv

/-- Emit-then-pump, under the emission's guard. -/
theorem wEdge_emitP {fut : List Ev} {st : MState} {e : Ev}
    (hen : enabled sk st.sent st.rcvd e = true)
    (hinv : WEdge sk (e :: fut) st) :
    WEdge sk fut (wEmitP sk st e) :=
  wEdge_pump sk (wEdge_emit sk hen hinv)

end StreamingMirror.Sched
