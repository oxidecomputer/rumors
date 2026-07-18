/-
The E worklist's fuel-free expansion semantics (unit 2a, PROGRESS.md
§9): `Expand.lean`'s tree-shaped ghost — what an op will emit,
expansions included — transcribed over the encoder-order expanders
`wScopeOpsE`/`wKidOpsE`, with the same fuel-sufficiency bridge to the
interpreter's `goEventsE` and the same steps-by-emissions bound. The
recursion measure and every argument shape are `Expand.lean`'s; only
the expansion membership lemmas differ (no splice case; the parent
emit closes each scope's op list).

The tail of the file crosses back into the counting layer: the E
interpreter preserves the family-generic `WCountP` exactly as the d5
interpreter does — `weaveGo_preserves`' twin, dispatching to the E
expanders.

Chain (.impl, stage B): mirrors Expand.lean over the E ops; provides the
E ghost and `weaveGoE_preserves` to MasterE.lean and FinalE.lean. Map:
Proofs/Map.lean.
-/
import StreamingMirror.Proofs.Sched.WeaveE
import StreamingMirror.Proofs.Sched.Weave.Expand

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ============================================== expansion membership

/-- What an E scope op expands to: prologue/parent emits and this
scope's kid ops (same height, same feed, `lastD` dead at `none`). -/
theorem mem_wScopeOpsE {op : WOp} {h k : Nat} {feed : List Ev}
    (hop : op ∈ wScopeOpsE sk h k feed) :
    (∃ e, op = .emit e)
      ∨ ∃ s kidBase i, op = .kid h k s none kidBase i feed := by
  simp only [wScopeOpsE] at hop
  rcases List.mem_append.1 hop with hop | hop
  · rcases List.mem_append.1 hop with hop | hop
    · rcases hop with _ | ⟨_, hop⟩
      · exact Or.inl ⟨_, rfl⟩
      · rcases hop with _ | ⟨_, hop⟩
        · exact Or.inl ⟨_, rfl⟩
        · cases hop
    · obtain ⟨i, -, rfl⟩ := List.mem_map.1 hop
      exact Or.inr ⟨_, _, _, rfl⟩
  · rcases hop with _ | ⟨_, hop⟩
    · exact Or.inl ⟨_, rfl⟩
    · cases hop

/-- What an E kid op expands to: chunk emits and — off the leaf
stage — one scope op a height down. The `h ≠ 0` guard is the
well-foundedness payload: the leaf stage never descends. -/
theorem mem_wKidOpsE {op : WOp} {h k s : Nat} {kidBase i : Nat}
    {feed : List Ev}
    (hop : op ∈ wKidOpsE sk h k s kidBase i feed) :
    (∃ e, op = .emit e)
      ∨ (h ≠ 0 ∧ ∃ k' feed', op = .scope (h - 1) k' feed') := by
  simp only [wKidOpsE] at hop
  rcases List.mem_append.1 hop with hop | hop
  · rcases hop with _ | ⟨_, hop⟩
    · exact Or.inl ⟨_, rfl⟩
    · cases hop
  · split at hop
    · -- D branch: `childIsD` pins `h ≠ 0`
      rename_i hD
      have hne : h ≠ 0 := by
        intro h0
        subst h0
        simp [Skel.childIsD] at hD
      rcases List.mem_append.1 hop with hop | hop
      · rcases List.mem_append.1 hop with hop | hop
        · rcases hop with _ | ⟨_, hop⟩
          · exact Or.inl ⟨_, rfl⟩
          · cases hop
        · rcases hfi : feed[i]? with - | q
          · rw [hfi] at hop; cases hop
          · rw [hfi] at hop
            rcases hop with _ | ⟨_, hop⟩
            · exact Or.inl ⟨_, rfl⟩
            · cases hop
      · rcases hop with _ | ⟨_, hop⟩
        · exact Or.inr ⟨hne, _, _, rfl⟩
        · cases hop
    · -- W branch
      rcases List.mem_append.1 hop with hop | hop
      · rcases hfi : feed[i]? with - | q
        · rw [hfi] at hop; cases hop
        · rw [hfi] at hop
          rcases hop with _ | ⟨_, hop⟩
          · exact Or.inl ⟨_, rfl⟩
          · cases hop
      · split at hop
        · cases hop
        · rename_i hne
          have hne' : h ≠ 0 := by simpa using hne
          rcases hop with _ | ⟨_, hop⟩
          · exact Or.inr ⟨hne', _, _, rfl⟩
          · cases hop

-- ======================================= the well-founded expansion

/-- The fuel-free expansion of one E op: the events it will emit (in
emission order) paired with the interpreter steps it will consume. -/
def opSpecE : WOp → List Ev × Nat
  | .emit e => ([e], 1)
  | .scope h k feed =>
      let sub := (wScopeOpsE sk h k feed).attach.map
        fun ⟨op, _⟩ => opSpecE op
      ((sub.map Prod.fst).flatten, 1 + (sub.map Prod.snd).sum)
  | .kid h k s _lastD kidBase i feed =>
      let sub := (wKidOpsE sk h k s kidBase i feed).attach.map
        fun ⟨op, _⟩ => opSpecE op
      ((sub.map Prod.fst).flatten, 1 + (sub.map Prod.snd).sum)
termination_by op => wopMeasure op
decreasing_by
  · rcases mem_wScopeOpsE sk ‹_› with ⟨e, rfl⟩ | ⟨s', kb, i', rfl⟩
    · simp [wopMeasure]
    · simp [wopMeasure]
  · rcases mem_wKidOpsE sk ‹_› with ⟨e, rfl⟩ | ⟨hne, k', f', rfl⟩
    · simp [wopMeasure]
    · simp only [wopMeasure]
      omega

/-- The events an E op will emit, expansions included. -/
def opEventsE (op : WOp) : List Ev := (opSpecE sk op).1

/-- The interpreter steps an E op will consume, expansions included. -/
def opStepsE (op : WOp) : Nat := (opSpecE sk op).2

/-- Mapping a function over `attach` forgets the membership proof. -/
private theorem attach_map_spec {α β : Type _} (l : List α)
    (f : α → β) : (l.attach.map fun x => f x.val) = l.map f := by
  calc l.attach.map (fun x => f x.val)
      = (l.attach.map Subtype.val).map f := by rw [List.map_map]; rfl
    _ = l.map f := by rw [List.attach_map_subtype_val]

theorem opEventsE_emit (e : Ev) : opEventsE sk (.emit e) = [e] := by
  unfold opEventsE
  simp [opSpecE]

theorem opStepsE_emit (e : Ev) : opStepsE sk (.emit e) = 1 := by
  unfold opStepsE
  simp [opSpecE]

theorem opEventsE_scope (h k : Nat) (feed : List Ev) :
    opEventsE sk (.scope h k feed)
      = (wScopeOpsE sk h k feed).flatMap (opEventsE sk) := by
  unfold opEventsE
  simp only [opSpecE, List.map_map]
  rw [show ((fun x : {op // op ∈ wScopeOpsE sk h k feed} =>
      opSpecE sk x.val) = fun x => opSpecE sk x.val) from rfl]
  rw [show (Prod.fst ∘ fun x : {op // op ∈ wScopeOpsE sk h k feed} =>
      opSpecE sk x.val) = fun x => (opSpecE sk x.val).1 from rfl]
  rw [attach_map_spec _ (fun op => (opSpecE sk op).1), List.flatMap_def]

theorem opStepsE_scope (h k : Nat) (feed : List Ev) :
    opStepsE sk (.scope h k feed)
      = 1 + ((wScopeOpsE sk h k feed).map (opStepsE sk)).sum := by
  unfold opStepsE
  simp only [opSpecE, List.map_map]
  rw [show (Prod.snd ∘ fun x : {op // op ∈ wScopeOpsE sk h k feed} =>
      opSpecE sk x.val) = fun x => (opSpecE sk x.val).2 from rfl]
  rw [attach_map_spec _ (fun op => (opSpecE sk op).2)]

theorem opEventsE_kid (h k s : Nat) (lastD : Option Nat)
    (kidBase i : Nat) (feed : List Ev) :
    opEventsE sk (.kid h k s lastD kidBase i feed)
      = (wKidOpsE sk h k s kidBase i feed).flatMap (opEventsE sk) := by
  unfold opEventsE
  simp only [opSpecE, List.map_map]
  rw [show (Prod.fst ∘ fun x :
      {op // op ∈ wKidOpsE sk h k s kidBase i feed} =>
      opSpecE sk x.val) = fun x => (opSpecE sk x.val).1 from rfl]
  rw [attach_map_spec _ (fun op => (opSpecE sk op).1), List.flatMap_def]

theorem opStepsE_kid (h k s : Nat) (lastD : Option Nat)
    (kidBase i : Nat) (feed : List Ev) :
    opStepsE sk (.kid h k s lastD kidBase i feed)
      = 1 + ((wKidOpsE sk h k s kidBase i feed).map
          (opStepsE sk)).sum := by
  unfold opStepsE
  simp only [opSpecE, List.map_map]
  rw [show (Prod.snd ∘ fun x :
      {op // op ∈ wKidOpsE sk h k s kidBase i feed} =>
      opSpecE sk x.val) = fun x => (opSpecE sk x.val).2 from rfl]
  rw [attach_map_spec _ (fun op => (opSpecE sk op).2)]

/-- Every E op costs at least its own step. -/
theorem opStepsE_pos (op : WOp) : 1 ≤ opStepsE sk op := by
  cases op with
  | emit e => rw [opStepsE_emit]; omega
  | scope h k feed => rw [opStepsE_scope]; omega
  | kid h k s lastD kidBase i feed => rw [opStepsE_kid]; omega

-- ====================================== steps bounded by emissions

/-- Summed per-op bounds (cf. `Expand.lean`). -/
private theorem steps_sum_leE {X : List WOp}
    (hX : ∀ x ∈ X, opStepsE sk x + 1 ≤ 3 * (opEventsE sk x).length) :
    (X.map (opStepsE sk)).sum + X.length
      ≤ 3 * (X.flatMap (opEventsE sk)).length := by
  induction X with
  | nil => simp
  | cons a X ih =>
      have ha := hX a (List.mem_cons_self ..)
      have hrest := ih fun x hx => hX x (List.mem_cons_of_mem _ hx)
      simp only [List.map_cons, List.sum_cons, List.length_cons,
        List.flatMap_cons, List.length_append]
      omega

/-- An E scope op expands to at least its two prologue emits. -/
private theorem wScopeOpsE_length (h k : Nat) (feed : List Ev) :
    2 ≤ (wScopeOpsE sk h k feed).length := by
  simp only [wScopeOpsE, List.append_assoc, List.length_append,
    List.length_cons, List.length_nil]
  omega

/-- An E kid op expands to at least its wire emit, cons-first. -/
private theorem wKidOpsE_shape (h k s : Nat) (kidBase i : Nat)
    (feed : List Ev) :
    ∃ e tail, wKidOpsE sk h k s kidBase i feed = .emit e :: tail :=
  ⟨_, _, rfl⟩

/-- Steps are dominated by emissions, uniformly over E ops (cf.
`opSteps_le`). -/
private theorem opStepsE_le_aux :
    ∀ (n : Nat) (op : WOp), wopMeasure op ≤ n →
      opStepsE sk op + 1 ≤ 3 * (opEventsE sk op).length := by
  intro n
  induction n with
  | zero =>
      intro op hop
      match op with
      | .emit e =>
          rw [opStepsE_emit, opEventsE_emit]
          simp
      | .scope h k feed => simp [wopMeasure] at hop
      | .kid h k s lastD kidBase i feed => simp [wopMeasure] at hop
  | succ n ih =>
      intro op hop
      match op with
      | .emit e =>
          rw [opStepsE_emit, opEventsE_emit]
          simp
      | .scope h k feed =>
          rw [opStepsE_scope, opEventsE_scope]
          have hsum := steps_sum_leE sk (X := wScopeOpsE sk h k feed)
            fun x hx => by
              rcases mem_wScopeOpsE sk hx with ⟨e, rfl⟩ | ⟨s', kb, i', rfl⟩
              · rw [opStepsE_emit, opEventsE_emit]; simp
              · refine ih _ ?_
                simp only [wopMeasure] at hop ⊢
                omega
          have hlen := wScopeOpsE_length sk h k feed
          omega
      | .kid h k s lastD kidBase i feed =>
          rw [opStepsE_kid, opEventsE_kid]
          obtain ⟨e₀, tail, hshape⟩ :=
            wKidOpsE_shape sk h k s kidBase i feed
          have hbound : ∀ x ∈ wKidOpsE sk h k s kidBase i feed,
              opStepsE sk x + 1 ≤ 3 * (opEventsE sk x).length := by
            intro x hx
            rcases mem_wKidOpsE sk hx with ⟨e, rfl⟩ | ⟨hne, k', f', rfl⟩
            · rw [opStepsE_emit, opEventsE_emit]; simp
            · refine ih _ ?_
              simp only [wopMeasure] at hop ⊢
              omega
          cases tail with
          | nil =>
              rw [hshape]
              simp only [List.map_cons, List.map_nil, List.sum_cons,
                List.sum_nil, List.flatMap_cons, List.flatMap_nil,
                opStepsE_emit, opEventsE_emit, List.append_nil,
                List.length_cons, List.length_nil]
              omega
          | cons t tail =>
              have hsum := steps_sum_leE sk hbound
              have hlen : 2 ≤ (wKidOpsE sk h k s kidBase i feed).length := by
                rw [hshape]
                simp
              omega

/-- Steps are dominated by emissions, top form. -/
theorem opStepsE_le (op : WOp) :
    opStepsE sk op + 1 ≤ 3 * (opEventsE sk op).length :=
  opStepsE_le_aux sk (wopMeasure op) op (Nat.le_refl _)

-- ============================================ the fuel-sufficiency

/-- With fuel at least the E worklist's total step count, the
interpreter's ghost futures ARE the fuel-free expansion. -/
theorem goEventsE_eq_of_fuel :
    ∀ (fuel : Nat) (ops : List WOp),
      (ops.map (opStepsE sk)).sum ≤ fuel →
      goEventsE sk fuel ops = ops.flatMap (opEventsE sk) := by
  intro fuel
  induction fuel with
  | zero =>
      intro ops h
      match ops with
      | [] => rfl
      | op :: rest =>
          exfalso
          have hpos := opStepsE_pos sk op
          simp only [List.map_cons, List.sum_cons, Nat.le_zero] at h
          omega
  | succ f ih =>
      intro ops h
      match ops with
      | [] => rfl
      | .emit e :: rest =>
          show e :: goEventsE sk f rest = _
          rw [List.flatMap_cons, opEventsE_emit,
            ih rest (by
              rw [List.map_cons, List.sum_cons, opStepsE_emit] at h
              omega)]
          rfl
      | .scope h' k feed :: rest =>
          show goEventsE sk f (wScopeOpsE sk h' k feed ++ rest) = _
          rw [ih _ (by
              rw [List.map_cons, List.sum_cons, opStepsE_scope] at h
              rw [List.map_append, List.sum_append]
              omega)]
          rw [List.flatMap_append, List.flatMap_cons, opEventsE_scope]
      | .kid h' k s lastD kidBase i feed :: rest =>
          show goEventsE sk f
            (wKidOpsE sk h' k s kidBase i feed ++ rest) = _
          rw [ih _ (by
              rw [List.map_cons, List.sum_cons, opStepsE_kid] at h
              rw [List.map_append, List.sum_append]
              omega)]
          rw [List.flatMap_append, List.flatMap_cons, opEventsE_kid]

/-- `weaveFuel` suffices for the E interpreter as soon as the opening
worklist's E emission count is bounded by the (shared) event total. -/
theorem goEventsE_weave
    (hlen : (((weaveOps sk).flatMap (opEventsE sk)).length)
      ≤ totalEvents sk) :
    goEventsE sk (weaveFuel sk) (weaveOps sk)
      = (weaveOps sk).flatMap (opEventsE sk) := by
  refine goEventsE_eq_of_fuel sk _ _ ?_
  have hsum := steps_sum_leE sk
    (X := weaveOps sk) fun x _ => opStepsE_le sk x
  unfold weaveFuel
  omega

-- =============================== counting through the E interpreter

/-- The counting invariant rides the E interpreter: `weaveGo_preserves`'
twin over the E expanders, at any trace family. -/
theorem weaveGoE_preserves {P : List (List Ev)} (fuel : Nat) :
    ∀ (ops : List WOp) (st : MState),
      WCountP sk P (goEventsE sk fuel ops) st →
      WCountP sk P [] (weaveGoE sk fuel ops st) := by
  induction fuel with
  | zero => intro ops st h; exact h
  | succ f ih =>
      intro ops st h
      match ops with
      | [] => exact h
      | .emit e :: rest =>
          exact ih rest _ (wEmitP_preserves sk h)
      | .scope h' k feed :: rest =>
          exact ih _ st h
      | .kid h' k s lastD kidBase i feed :: rest =>
          exact ih _ st h

/-- The eweave's final state carries the counting invariant at the
encoder-order family with no futures left, GIVEN the E initial
alignment — the analog of `weaveState_wcount`, with the pump half
supplied by `procsE_drop_pumps`. -/
theorem weaveStateE_wcount
    (halign : manFilters sk (goEventsE sk (weaveFuel sk) (weaveOps sk))
      = (procsE sk).take (manCount sk))
    (howners : ∀ e ∈ goEventsE sk (weaveFuel sk) (weaveOps sk),
      evOwner sk e < manCount sk) :
    WCountP sk (procsE sk) [] (weaveStateE sk) :=
  wPump_preserves sk
    (weaveGoE_preserves sk _ _ _
      (wcount_init sk halign (procsE_drop_pumps sk) howners))

end StreamingMirror.Sched
