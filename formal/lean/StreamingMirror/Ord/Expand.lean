/-
The O worklist's fuel-free expansion semantics: `ExpandE.lean`'s
tree-shaped ghost — what an op will emit, expansions included —
transcribed over the O scope expander `wScopeOpsO` (kid ops are
`wKidOpsE`'s, shared), with the same fuel-sufficiency bridge to the
interpreter's `goEventsO` and the same steps-by-emissions bound. The
recursion measure and every argument shape are `ExpandE.lean`'s; only
the scope-expansion membership lemma differs (the prologue emits
arrive through `prologueO`'s map instead of two literal conses).

The tail of the file crosses back into the counting layer: the O
interpreter preserves the family-generic `WCountP` exactly as the E
interpreter does, and the O initial state inhabits it against any
family whose pump suffix is `weavePumpsO` — `wcount_init` transcribed
with the rewrite target swapped (the base lemma pins the pump half to
`weavePumps`, which a query-first absorber cannot satisfy; the O
weave's `rem` is `weavePumpsO`, pinned unconditionally by
`procsO_drop_pumpsO`).

Chain (ord, stage D): the witness and its alignment; consumed by the
O master induction. Base mirror: ExpandE.lean + Weave/Count.lean's
`wcount_init`. Map: PROGRESS.md §13.
-/
import StreamingMirror.Ord.Weave
import StreamingMirror.Proofs.Sched.Weave.ExpandE

namespace StreamingMirror.Ord

open Model
open Sched

variable (sk : Skel) (ord : OrdMap)

-- ============================================== expansion membership

/-- What an O scope op expands to: prologue/parent emits and this
scope's kid ops (same height, same feed, `lastD` dead at `none`) —
`mem_wScopeOpsE`'s shape, with the prologue emits read off
`prologueO`'s map. -/
theorem mem_wScopeOpsO {op : WOp} {h k : Nat} {feed : List Ev}
    (hop : op ∈ wScopeOpsO sk ord h k feed) :
    (∃ e, op = .emit e)
      ∨ ∃ s kidBase i, op = .kid h k s none kidBase i feed := by
  simp only [wScopeOpsO] at hop
  rcases List.mem_append.1 hop with hop | hop
  · rcases List.mem_append.1 hop with hop | hop
    · obtain ⟨e, -, rfl⟩ := List.mem_map.1 hop
      exact Or.inl ⟨_, rfl⟩
    · obtain ⟨i, -, rfl⟩ := List.mem_map.1 hop
      exact Or.inr ⟨_, _, _, rfl⟩
  · rcases hop with _ | ⟨_, hop⟩
    · exact Or.inl ⟨_, rfl⟩
    · cases hop

-- ======================================= the well-founded expansion

/-- The fuel-free expansion of one O op: the events it will emit (in
emission order) paired with the interpreter steps it will consume. -/
def opSpecO : WOp → List Ev × Nat
  | .emit e => ([e], 1)
  | .scope h k feed =>
      let sub := (wScopeOpsO sk ord h k feed).attach.map
        fun ⟨op, _⟩ => opSpecO op
      ((sub.map Prod.fst).flatten, 1 + (sub.map Prod.snd).sum)
  | .kid h k s _lastD kidBase i feed =>
      let sub := (wKidOpsE sk h k s kidBase i feed).attach.map
        fun ⟨op, _⟩ => opSpecO op
      ((sub.map Prod.fst).flatten, 1 + (sub.map Prod.snd).sum)
termination_by op => wopMeasure op
decreasing_by
  · rcases mem_wScopeOpsO sk ord ‹_› with ⟨e, rfl⟩ | ⟨s', kb, i', rfl⟩
    · simp [wopMeasure]
    · simp [wopMeasure]
  · rcases mem_wKidOpsE sk ‹_› with ⟨e, rfl⟩ | ⟨hne, k', f', rfl⟩
    · simp [wopMeasure]
    · simp only [wopMeasure]
      omega

/-- The events an O op will emit, expansions included. -/
def opEventsO (op : WOp) : List Ev := (opSpecO sk ord op).1

/-- The interpreter steps an O op will consume, expansions included. -/
def opStepsO (op : WOp) : Nat := (opSpecO sk ord op).2

/-- Mapping a function over `attach` forgets the membership proof
(private twin of ExpandE's `attach_map_spec`). -/
private theorem attach_map_spec {α β : Type _} (l : List α)
    (f : α → β) : (l.attach.map fun x => f x.val) = l.map f := by
  calc l.attach.map (fun x => f x.val)
      = (l.attach.map Subtype.val).map f := by rw [List.map_map]; rfl
    _ = l.map f := by rw [List.attach_map_subtype_val]

theorem opEventsO_emit (e : Ev) : opEventsO sk ord (.emit e) = [e] := by
  unfold opEventsO
  simp [opSpecO]

theorem opStepsO_emit (e : Ev) : opStepsO sk ord (.emit e) = 1 := by
  unfold opStepsO
  simp [opSpecO]

theorem opEventsO_scope (h k : Nat) (feed : List Ev) :
    opEventsO sk ord (.scope h k feed)
      = (wScopeOpsO sk ord h k feed).flatMap (opEventsO sk ord) := by
  unfold opEventsO
  simp only [opSpecO, List.map_map]
  rw [show ((fun x : {op // op ∈ wScopeOpsO sk ord h k feed} =>
      opSpecO sk ord x.val) = fun x => opSpecO sk ord x.val) from rfl]
  rw [show (Prod.fst ∘ fun x : {op // op ∈ wScopeOpsO sk ord h k feed} =>
      opSpecO sk ord x.val) = fun x => (opSpecO sk ord x.val).1 from rfl]
  rw [attach_map_spec _ (fun op => (opSpecO sk ord op).1), List.flatMap_def]

theorem opStepsO_scope (h k : Nat) (feed : List Ev) :
    opStepsO sk ord (.scope h k feed)
      = 1 + ((wScopeOpsO sk ord h k feed).map (opStepsO sk ord)).sum := by
  unfold opStepsO
  simp only [opSpecO, List.map_map]
  rw [show (Prod.snd ∘ fun x : {op // op ∈ wScopeOpsO sk ord h k feed} =>
      opSpecO sk ord x.val) = fun x => (opSpecO sk ord x.val).2 from rfl]
  rw [attach_map_spec _ (fun op => (opSpecO sk ord op).2)]

theorem opEventsO_kid (h k s : Nat) (lastD : Option Nat)
    (kidBase i : Nat) (feed : List Ev) :
    opEventsO sk ord (.kid h k s lastD kidBase i feed)
      = (wKidOpsE sk h k s kidBase i feed).flatMap (opEventsO sk ord) := by
  unfold opEventsO
  simp only [opSpecO, List.map_map]
  rw [show (Prod.fst ∘ fun x :
      {op // op ∈ wKidOpsE sk h k s kidBase i feed} =>
      opSpecO sk ord x.val) = fun x => (opSpecO sk ord x.val).1 from rfl]
  rw [attach_map_spec _ (fun op => (opSpecO sk ord op).1), List.flatMap_def]

theorem opStepsO_kid (h k s : Nat) (lastD : Option Nat)
    (kidBase i : Nat) (feed : List Ev) :
    opStepsO sk ord (.kid h k s lastD kidBase i feed)
      = 1 + ((wKidOpsE sk h k s kidBase i feed).map
          (opStepsO sk ord)).sum := by
  unfold opStepsO
  simp only [opSpecO, List.map_map]
  rw [show (Prod.snd ∘ fun x :
      {op // op ∈ wKidOpsE sk h k s kidBase i feed} =>
      opSpecO sk ord x.val) = fun x => (opSpecO sk ord x.val).2 from rfl]
  rw [attach_map_spec _ (fun op => (opSpecO sk ord op).2)]

/-- Every O op costs at least its own step. -/
theorem opStepsO_pos (op : WOp) : 1 ≤ opStepsO sk ord op := by
  cases op with
  | emit e => rw [opStepsO_emit]; omega
  | scope h k feed => rw [opStepsO_scope]; omega
  | kid h k s lastD kidBase i feed => rw [opStepsO_kid]; omega

-- ====================================== steps bounded by emissions

/-- Summed per-op bounds (private twin of ExpandE's `steps_sum_leE`). -/
private theorem steps_sum_leO {X : List WOp}
    (hX : ∀ x ∈ X, opStepsO sk ord x + 1 ≤ 3 * (opEventsO sk ord x).length) :
    (X.map (opStepsO sk ord)).sum + X.length
      ≤ 3 * (X.flatMap (opEventsO sk ord)).length := by
  induction X with
  | nil => simp
  | cons a X ih =>
      have ha := hX a (List.mem_cons_self ..)
      have hrest := ih fun x hx => hX x (List.mem_cons_of_mem _ hx)
      simp only [List.map_cons, List.sum_cons, List.length_cons,
        List.flatMap_cons, List.length_append]
      omega

/-- An O scope op expands to at least its two prologue emits (private
twin of ExpandE's `wScopeOpsE_length`). -/
private theorem wScopeOpsO_length (h k : Nat) (feed : List Ev) :
    2 ≤ (wScopeOpsO sk ord h k feed).length := by
  simp only [wScopeOpsO, List.append_assoc, List.length_append,
    List.length_map, List.length_cons, List.length_nil, prologueO_length]
  omega

/-- A kid op expands to at least its wire emit, cons-first (private
twin of ExpandE's `wKidOpsE_shape`). -/
private theorem wKidOpsE_shape (h k s : Nat) (kidBase i : Nat)
    (feed : List Ev) :
    ∃ e tail, wKidOpsE sk h k s kidBase i feed = .emit e :: tail :=
  ⟨_, _, rfl⟩

/-- Steps are dominated by emissions, uniformly over O ops (private
twin of ExpandE's `opStepsE_le_aux`). -/
private theorem opStepsO_le_aux :
    ∀ (n : Nat) (op : WOp), wopMeasure op ≤ n →
      opStepsO sk ord op + 1 ≤ 3 * (opEventsO sk ord op).length := by
  intro n
  induction n with
  | zero =>
      intro op hop
      match op with
      | .emit e =>
          rw [opStepsO_emit, opEventsO_emit]
          simp
      | .scope h k feed => simp [wopMeasure] at hop
      | .kid h k s lastD kidBase i feed => simp [wopMeasure] at hop
  | succ n ih =>
      intro op hop
      match op with
      | .emit e =>
          rw [opStepsO_emit, opEventsO_emit]
          simp
      | .scope h k feed =>
          rw [opStepsO_scope, opEventsO_scope]
          have hsum := steps_sum_leO sk ord (X := wScopeOpsO sk ord h k feed)
            fun x hx => by
              rcases mem_wScopeOpsO sk ord hx with ⟨e, rfl⟩ | ⟨s', kb, i', rfl⟩
              · rw [opStepsO_emit, opEventsO_emit]; simp
              · refine ih _ ?_
                simp only [wopMeasure] at hop ⊢
                omega
          have hlen := wScopeOpsO_length sk ord h k feed
          omega
      | .kid h k s lastD kidBase i feed =>
          rw [opStepsO_kid, opEventsO_kid]
          obtain ⟨e₀, tail, hshape⟩ :=
            wKidOpsE_shape sk h k s kidBase i feed
          have hbound : ∀ x ∈ wKidOpsE sk h k s kidBase i feed,
              opStepsO sk ord x + 1 ≤ 3 * (opEventsO sk ord x).length := by
            intro x hx
            rcases mem_wKidOpsE sk hx with ⟨e, rfl⟩ | ⟨hne, k', f', rfl⟩
            · rw [opStepsO_emit, opEventsO_emit]; simp
            · refine ih _ ?_
              simp only [wopMeasure] at hop ⊢
              omega
          cases tail with
          | nil =>
              rw [hshape]
              simp only [List.map_cons, List.map_nil, List.sum_cons,
                List.sum_nil, List.flatMap_cons, List.flatMap_nil,
                opStepsO_emit, opEventsO_emit, List.append_nil,
                List.length_cons, List.length_nil]
              omega
          | cons t tail =>
              have hsum := steps_sum_leO sk ord hbound
              have hlen : 2 ≤ (wKidOpsE sk h k s kidBase i feed).length := by
                rw [hshape]
                simp
              omega

/-- Steps are dominated by emissions, top form. -/
theorem opStepsO_le (op : WOp) :
    opStepsO sk ord op + 1 ≤ 3 * (opEventsO sk ord op).length :=
  opStepsO_le_aux sk ord (wopMeasure op) op (Nat.le_refl _)

-- ============================================ the fuel-sufficiency

/-- With fuel at least the O worklist's total step count, the
interpreter's ghost futures ARE the fuel-free expansion. -/
theorem goEventsO_eq_of_fuel :
    ∀ (fuel : Nat) (ops : List WOp),
      (ops.map (opStepsO sk ord)).sum ≤ fuel →
      goEventsO sk ord fuel ops = ops.flatMap (opEventsO sk ord) := by
  intro fuel
  induction fuel with
  | zero =>
      intro ops h
      match ops with
      | [] => rfl
      | op :: rest =>
          exfalso
          have hpos := opStepsO_pos sk ord op
          simp only [List.map_cons, List.sum_cons, Nat.le_zero] at h
          omega
  | succ f ih =>
      intro ops h
      match ops with
      | [] => rfl
      | .emit e :: rest =>
          show e :: goEventsO sk ord f rest = _
          rw [List.flatMap_cons, opEventsO_emit,
            ih rest (by
              rw [List.map_cons, List.sum_cons, opStepsO_emit] at h
              omega)]
          rfl
      | .scope h' k feed :: rest =>
          show goEventsO sk ord f (wScopeOpsO sk ord h' k feed ++ rest) = _
          rw [ih _ (by
              rw [List.map_cons, List.sum_cons, opStepsO_scope] at h
              rw [List.map_append, List.sum_append]
              omega)]
          rw [List.flatMap_append, List.flatMap_cons, opEventsO_scope]
      | .kid h' k s lastD kidBase i feed :: rest =>
          show goEventsO sk ord f
            (wKidOpsE sk h' k s kidBase i feed ++ rest) = _
          rw [ih _ (by
              rw [List.map_cons, List.sum_cons, opStepsO_kid] at h
              rw [List.map_append, List.sum_append]
              omega)]
          rw [List.flatMap_append, List.flatMap_cons, opEventsO_kid]

/-- `weaveFuel` suffices for the O interpreter as soon as the opening
worklist's O emission count is bounded by the (shared) event total. -/
theorem goEventsO_weave
    (hlen : (((weaveOps sk).flatMap (opEventsO sk ord)).length)
      ≤ totalEvents sk) :
    goEventsO sk ord (weaveFuel sk) (weaveOps sk)
      = (weaveOps sk).flatMap (opEventsO sk ord) := by
  refine goEventsO_eq_of_fuel sk ord _ _ ?_
  have hsum := steps_sum_leO sk ord
    (X := weaveOps sk) fun x _ => opStepsO_le sk ord x
  unfold weaveFuel
  omega

-- =============================== counting through the O interpreter

/-- The counting invariant rides the O interpreter: `weaveGo_preserves`'
twin over the O expanders, at any trace family. -/
theorem weaveGoO_preserves {P : List (List Ev)} (fuel : Nat) :
    ∀ (ops : List WOp) (st : MState),
      WCountP sk P (goEventsO sk ord fuel ops) st →
      WCountP sk P [] (weaveGoO sk ord fuel ops st) := by
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

/-- The O weave's starting state satisfies the counting invariant,
given the initial alignment: `wcount_init` with the pump half pinned
to `weavePumpsO` — the base lemma's rewrite target swapped, because
`weaveInitO` racks the O pump rows and a query-first absorber cannot
satisfy the base `weavePumps` equality. -/
theorem wcount_initO {P : List (List Ev)} {fut : List Ev}
    (halign : manFilters sk fut = P.take (manCount sk))
    (hpumps : P.drop (manCount sk) = weavePumpsO sk ord)
    (howners : ∀ e ∈ fut, evOwner sk e < manCount sk) :
    WCountP sk P fut (weaveInitO sk ord) := by
  refine ⟨howners, ?_, ?_, fun c => rfl, fun c => rfl, ?_⟩
  · rw [halign]
    exact Forall2.self fun t _ => ⟨[], rfl, List.nil_sublist _⟩
  · show Forall2 _ _ (weavePumpsO sk ord)
    rw [← hpumps]
    exact Forall2.self fun t _ => ⟨[], rfl, List.nil_sublist _⟩
  · intro p
    show (0 : Nat) = _ + emittedCount p _ (weavePumpsO sk ord)
    rw [halign, ← hpumps, emittedCount_refl, emittedCount_refl]

/-- The O weave's final state carries the counting invariant at the O
family with no futures left, GIVEN the O initial alignment — the
analog of `weaveStateE_wcount`, with the pump half supplied by the
unconditional `procsO_drop_pumpsO`. -/
theorem weaveStateO_wcount
    (halign : manFilters sk (goEventsO sk ord (weaveFuel sk) (weaveOps sk))
      = (procsO sk ord).take (manCount sk))
    (howners : ∀ e ∈ goEventsO sk ord (weaveFuel sk) (weaveOps sk),
      evOwner sk e < manCount sk) :
    WCountP sk (procsO sk ord) [] (weaveStateO sk ord) :=
  wPump_preserves sk
    (weaveGoO_preserves sk ord _ _ _
      (wcount_initO sk ord halign (procsO_drop_pumpsO sk ord) howners))

end StreamingMirror.Ord
