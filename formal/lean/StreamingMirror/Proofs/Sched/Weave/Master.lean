/-
Layer D's master induction, the consumption half (PROGRESS.md §7 3b):
the pointwise emission-readiness property `EmitOKOn` of the weave's
ghost future, and the fuel induction that rides it through the
interpreter to `WEdge sk [] (weaveState sk)`.

# Shape

`wEdge_emit` wants `enabled` at every manual emission. Everything a
guard consults is determined by the REMAINING future: the counting
invariant pins each owned count to its whole-trace total minus the
future's share (`count_pin`), so a site's enabledness is a property
of the future's filter shapes — a pure list property. `EmitOKOn l
rest` states it pointwise: at every position of `l`, the event is
emittable from ANY state that satisfies `WEdge` over the position's
suffix (with `rest` glued after `l`), sits at a pump fixpoint, and
holds the event's `manDep` predecessor in its output (supplied at
consumption time by the precedence layer's `DepOK`).

The fuel induction (`weaveGo_wedge`) consumes the property one
emission at a time, exactly as `weaveGo_preserves` consumes
`WCount`: pump steps are free (`wEdge_step`), and each manual
emission discharges its guard from the property's head, `DepOK`'s
head, and the pump fixpoint the previous `wEmitP` left behind. The
one state the interpreter ever emits from that is NOT a pump
fixpoint is `weaveInit`, whose first emission is iopen's seq-0
opening wire — `weaveState_wedge_of_emitOK` peels it by hand with
`enabled_snd_low` before entering the induction.

Establishing `EmitOKOn` over the opening worklist — the tree
induction threading the rolling ancestor context through the scope
recursion — is the production half (see the RestCtx sections below).
-/
import StreamingMirror.Proofs.Sched.Weave.Site

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ============================================ the pointwise property

/-- Pointwise emission-readiness of a future against a glued tail.

At every position of `l`, the event is emittable from any state that
satisfies the edge invariant over the position's suffix (with `rest`
appended), sits at a pump fixpoint, and has the event's manual
predecessor already emitted. -/
def EmitOKOn (l rest : List Ev) : Prop :=
  ∀ n e, l[n]? = some e →
    ∀ st : MState, WEdge sk (l.drop n ++ rest) st →
      step sk st = none →
      (∀ d, manDep e = some d → d ∈ st.out) →
      enabled sk st.sent st.rcvd e = true

theorem emitOKOn_nil (rest : List Ev) : EmitOKOn sk [] rest := by
  intro n e h
  simp at h

/-- Extend readiness by one head whose own discharge is supplied. -/
theorem emitOKOn_cons {e : Ev} {l rest : List Ev}
    (hhead : ∀ st : MState, WEdge sk (e :: (l ++ rest)) st →
      step sk st = none →
      (∀ d, manDep e = some d → d ∈ st.out) →
      enabled sk st.sent st.rcvd e = true)
    (htail : EmitOKOn sk l rest) : EmitOKOn sk (e :: l) rest := by
  intro n e' hn st hW hfix hpred
  match n with
  | 0 =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hn
      subst hn
      exact hhead st hW hfix hpred
  | n + 1 =>
      simp only [List.getElem?_cons_succ] at hn
      exact htail n e' hn st hW hfix hpred

/-- Consuming the head keeps the readiness of the tail. -/
theorem emitOKOn_tail {e : Ev} {l rest : List Ev}
    (h : EmitOKOn sk (e :: l) rest) : EmitOKOn sk l rest :=
  fun n e' hn st hW hfix hpred =>
    h (n + 1) e' (by simpa using hn) st hW hfix hpred

/-- Glue readiness: the left part sees the right as its tail. -/
theorem emitOKOn_append {A B rest : List Ev}
    (hA : EmitOKOn sk A (B ++ rest)) (hB : EmitOKOn sk B rest) :
    EmitOKOn sk (A ++ B) rest := by
  intro n e hn st hW hfix hpred
  rcases Nat.lt_or_ge n A.length with hlt | hge
  · rw [List.getElem?_append_left hlt] at hn
    refine hA n e hn st ?_ hfix hpred
    rwa [List.drop_append_of_le_length (Nat.le_of_lt hlt),
      List.append_assoc] at hW
  · rw [List.getElem?_append_right hge] at hn
    refine hB (n - A.length) e hn st ?_ hfix hpred
    rw [show n = A.length + (n - A.length) from by omega,
      List.drop_append] at hW
    rwa [List.drop_eq_nil_of_le (by omega), Nat.add_sub_cancel_left,
      List.nil_append] at hW

-- ======================================= output growth through pumps

/-- One merge step only appends to the output. -/
theorem mem_out_step {st st' : MState} (hstep : step sk st = some st')
    {x : Ev} (hx : x ∈ st.out) : x ∈ st'.out := by
  unfold step at hstep
  cases hscan : scan sk st.sent st.rcvd st.rem with
  | none => rw [hscan] at hstep; simp at hstep
  | some pr =>
      obtain ⟨e, rem'⟩ := pr
      rw [hscan] at hstep
      simp only [Option.map] at hstep
      obtain ⟨c, sd, n⟩ := e
      cases sd <;> cases hstep <;> exact List.mem_append_left _ hx

/-- The merge only appends to the output, any amount of fuel. -/
theorem mem_out_mergeN (fuel : Nat) :
    ∀ {st : MState} {x : Ev}, x ∈ st.out →
      x ∈ (mergeN sk fuel st).out := by
  induction fuel with
  | zero => intro st x hx; exact hx
  | succ f ih =>
      intro st x hx
      unfold mergeN
      cases hstep : step sk st with
      | some st' => exact ih (mem_out_step sk hstep hx)
      | none => exact hx

/-- Emit-then-pump keeps the emitted prefix and the new event. -/
theorem mem_out_wEmitP {st : MState} {e x : Ev}
    (hx : x ∈ st.out ++ [e]) : x ∈ (wEmitP sk st e).out := by
  unfold wEmitP wPump
  refine mem_out_mergeN sk _ ?_
  rw [wEmit_out]
  exact hx

-- ======================================= the consumption induction

/-- THE CONSUMPTION INDUCTION: the edge invariant rides the
interpreter, each manual guard discharged from the pointwise
readiness property, the precedence layer, and the pump fixpoint the
previous emission left behind. -/
theorem weaveGo_wedge (fuel : Nat) :
    ∀ (ops : List WOp) (st : MState) (done : List Ev),
      WEdge sk (goEvents sk fuel ops) st →
      DepOK done (goEvents sk fuel ops) →
      (∀ x ∈ done, x ∈ st.out) →
      EmitOKOn sk (goEvents sk fuel ops) [] →
      step sk st = none →
      WEdge sk [] (weaveGo sk fuel ops st) := by
  induction fuel with
  | zero => intro ops st done hW _ _ _ _; exact hW
  | succ f ih =>
      intro ops st done hW hdep hdone hemit hfix
      match ops with
      | [] => exact hW
      | .emit e :: rest =>
          have hgo : goEvents sk (f + 1) (.emit e :: rest)
              = e :: goEvents sk f rest := rfl
          rw [hgo] at hW hdep hemit
          have hen : enabled sk st.sent st.rcvd e = true := by
            refine hemit 0 e rfl st (by simpa using hW) hfix ?_
            intro d hd
            exact hdone d (depOK_head hdep d hd)
          show WEdge sk [] (weaveGo sk f rest (wEmitP sk st e))
          refine ih rest (wEmitP sk st e) (done ++ [e])
            (wEdge_emitP sk hen hW) (depOK_tail hdep) ?_
            (emitOKOn_tail sk hemit) (wPump_fixpoint sk _)
          intro x hx
          rcases List.mem_append.1 hx with hx | hx
          · exact mem_out_wEmitP sk
              (List.mem_append_left _ (hdone x hx))
          · have hxe : x = e := List.mem_singleton.1 hx
            subst hxe
            exact mem_out_wEmitP sk
              (List.mem_append_right _ (List.mem_cons_self ..))
      | .scope h' k feed :: rest =>
          exact ih _ st done hW hdep hdone hemit hfix
      | .kid h' k s lastD kidBase i feed :: rest =>
          exact ih _ st done hW hdep hdone hemit hfix

-- =============================================== the top assembly

/-- The weave respects every edge GIVEN the pointwise readiness of
the opening worklist's future.

The initial alignment and the precedence layer are already closed
(`weave_initial_alignment`, `weave_goEvents_depOK`); the first
emission — iopen's seq-0 opening wire, the only emission from a
state that is not a pump fixpoint — is peeled by hand with
`enabled_snd_low` before the consumption induction takes over. -/
theorem weaveState_wedge_of_emitOK (hwf : sk.wellFormed = true)
    (hemit : EmitOKOn sk ((weaveOps sk).flatMap (opEvents sk)) []) :
    WEdge sk [] (weaveState sk) := by
  obtain ⟨hown, halign⟩ := weave_initial_alignment sk hwf
  have hgo : goEvents sk (weaveFuel sk) (weaveOps sk)
      = (weaveOps sk).flatMap (opEvents sk) :=
    goEvents_weave sk (weave_events_length sk hwf)
  have hinit : WEdge sk (goEvents sk (weaveFuel sk) (weaveOps sk))
      (weaveInit sk) :=
    wEdge_init sk (by rw [hgo]; exact halign)
      (by rw [hgo]; exact hown)
  have hdep : DepOK [] (goEvents sk (weaveFuel sk) (weaveOps sk)) :=
    weave_goEvents_depOK sk hwf
  obtain ⟨f, hfuel⟩ : ∃ f, weaveFuel sk = f + 1 :=
    ⟨4 * totalEvents sk + 7, by unfold weaveFuel; omega⟩
  -- the head opener, and the worklist behind it
  obtain ⟨e₁, opsTail, hops, he₁⟩ :
      ∃ (e₁ : Ev) (opsTail : List WOp),
        weaveOps sk = .emit e₁ :: opsTail
          ∧ e₁ = ((Chan.wire Party.I sk.rootH, true, 0) : Ev) :=
    ⟨_, _, rfl, rfl⟩
  have hgo1 : goEvents sk (weaveFuel sk) (weaveOps sk)
      = e₁ :: goEvents sk f opsTail := by
    rw [hfuel, hops]
    rfl
  have hen : enabled sk (weaveInit sk).sent (weaveInit sk).rcvd e₁
      = true := by
    rw [he₁]
    exact enabled_snd_low sk (cap_pos hwf _)
  have hW1 : WEdge sk (e₁ :: goEvents sk f opsTail) (weaveInit sk) := by
    rw [← hgo1]
    exact hinit
  show WEdge sk []
    (wPump sk (weaveGo sk (weaveFuel sk) (weaveOps sk) (weaveInit sk)))
  have hstep1 : weaveGo sk (weaveFuel sk) (weaveOps sk) (weaveInit sk)
      = weaveGo sk f opsTail (wEmitP sk (weaveInit sk) e₁) := by
    rw [hfuel, hops]
    rfl
  rw [hstep1]
  refine wEdge_pump sk ?_
  refine weaveGo_wedge sk f opsTail _ [e₁]
    (wEdge_emitP sk hen hW1) ?_ ?_ ?_ (wPump_fixpoint sk _)
  · have hd1 : DepOK [] (e₁ :: goEvents sk f opsTail) := by
      rw [← hgo1]
      exact hdep
    simpa using depOK_tail hd1
  · intro x hx
    have hxe : x = e₁ := List.mem_singleton.1 hx
    refine mem_out_wEmitP sk ?_
    rw [hxe]
    exact List.mem_append_right _ (List.mem_cons_self ..)
  · refine emitOKOn_tail sk (e := e₁) ?_
    rw [← hgo1, hgo]
    exact hemit

end StreamingMirror.Sched
