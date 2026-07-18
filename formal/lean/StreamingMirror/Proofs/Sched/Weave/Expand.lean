/-
The worklist's fuel-free expansion semantics (PROGRESS.md §7 3b): the
tree-shaped ghost `opEvents` — what a weave op will emit, including
everything its expansions emit — and the fuel-sufficiency bridge from
it to the interpreter's fuel-indexed `goEvents`.

# Why a second semantics

`weaveGo`/`goEvents` are structural on fuel so the kernel can
evaluate them, but the alignment proof (per-owner filters of the
futures = the manual traces) wants to induct over the SCOPE TREE, not
over fuel. `opEvents` is the same expansion as a well-founded
recursion on `wopMeasure` (scope > kid at a height > everything at
lower heights), which gives the tree induction; `goEvents_eq_of_fuel`
then discharges the fuel side once and for all: with fuel at least
the interpreter's step count (`opSteps`), the two semantics agree.
`opSteps_le` bounds the step count by the emission count — every op
emits at least one event, and each expansion layer costs one step —
so `weaveFuel`'s `4 * totalEvents + 8` is sufficient as soon as the
alignment pins the emission count to the manual traces' total.

Chain (d5, stage B): provides the expansion ghost (`opEvents`) and the
fuel bridge to Master.lean and Final.lean. E mirror: ExpandE.lean. Map:
Proofs/Map.lean.
-/
import StreamingMirror.Proofs.Sched.Weave.Count

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ============================================== expansion membership

/-- What a scope op expands to: prologue/summary emits and this
scope's kid ops (same height, same feed). -/
theorem mem_wScopeOps {op : WOp} {h k : Nat} {feed : List Ev}
    (hop : op ∈ wScopeOps sk h k feed) :
    (∃ e, op = .emit e)
      ∨ ∃ s lastD kidBase i, op = .kid h k s lastD kidBase i feed := by
  simp only [wScopeOps] at hop
  rcases List.mem_append.1 hop with hop | hop
  · rcases List.mem_append.1 hop with hop | hop
    · rcases hop with _ | ⟨_, hop⟩
      · exact Or.inl ⟨_, rfl⟩
      · rcases hop with _ | ⟨_, hop⟩
        · exact Or.inl ⟨_, rfl⟩
        · cases hop
    · split at hop
      · rcases hop with _ | ⟨_, hop⟩
        · exact Or.inl ⟨_, rfl⟩
        · cases hop
      · cases hop
  · obtain ⟨i, -, rfl⟩ := List.mem_map.1 hop
    exact Or.inr ⟨_, _, _, _, rfl⟩

/-- What a kid op expands to: chunk emits and — off the leaf stage —
one scope op a height down. The `h ≠ 0` guard is the well-foundedness
payload: the leaf stage never descends. -/
theorem mem_wKidOps {op : WOp} {h k s : Nat} {lastD : Option Nat}
    {kidBase i : Nat} {feed : List Ev}
    (hop : op ∈ wKidOps sk h k s lastD kidBase i feed) :
    (∃ e, op = .emit e)
      ∨ (h ≠ 0 ∧ ∃ k' feed', op = .scope (h - 1) k' feed') := by
  simp only [wKidOps] at hop
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
        · rcases List.mem_append.1 hop with hop | hop
          · rcases hop with _ | ⟨_, hop⟩
            · exact Or.inl ⟨_, rfl⟩
            · cases hop
          · split at hop
            · rcases hop with _ | ⟨_, hop⟩
              · exact Or.inl ⟨_, rfl⟩
              · cases hop
            · cases hop
        · -- the feed op
          rcases hfi : feed[i]? with - | q
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

/-- Expansion rank: scope beats kid at a height, and any op at a
height beats everything below — the measure the tree recursion
descends. -/
def wopMeasure : WOp → Nat
  | .emit _ => 0
  | .kid h _ _ _ _ _ _ => 2 * h + 1
  | .scope h _ _ => 2 * h + 2

/-- The fuel-free expansion of one op: the events it will emit (in
emission order) paired with the interpreter steps it will consume. -/
def opSpec : WOp → List Ev × Nat
  | .emit e => ([e], 1)
  | .scope h k feed =>
      let sub := (wScopeOps sk h k feed).attach.map
        fun ⟨op, _⟩ => opSpec op
      ((sub.map Prod.fst).flatten, 1 + (sub.map Prod.snd).sum)
  | .kid h k s lastD kidBase i feed =>
      let sub := (wKidOps sk h k s lastD kidBase i feed).attach.map
        fun ⟨op, _⟩ => opSpec op
      ((sub.map Prod.fst).flatten, 1 + (sub.map Prod.snd).sum)
termination_by op => wopMeasure op
decreasing_by
  · rcases mem_wScopeOps sk ‹_› with ⟨e, rfl⟩ | ⟨s', lD, kb, i', rfl⟩
    · simp [wopMeasure]
    · simp [wopMeasure]
  · rcases mem_wKidOps sk ‹_› with ⟨e, rfl⟩ | ⟨hne, k', f', rfl⟩
    · simp [wopMeasure]
    · simp only [wopMeasure]
      omega

/-- The events an op will emit, expansions included. -/
def opEvents (op : WOp) : List Ev := (opSpec sk op).1

/-- The interpreter steps an op will consume, expansions included. -/
def opSteps (op : WOp) : Nat := (opSpec sk op).2

/-- Mapping a function over `attach` forgets the membership proof. -/
private theorem attach_map_spec {α β : Type _} (l : List α)
    (f : α → β) : (l.attach.map fun x => f x.val) = l.map f := by
  calc l.attach.map (fun x => f x.val)
      = (l.attach.map Subtype.val).map f := by rw [List.map_map]; rfl
    _ = l.map f := by rw [List.attach_map_subtype_val]

theorem opEvents_emit (e : Ev) : opEvents sk (.emit e) = [e] := by
  unfold opEvents
  simp [opSpec]

theorem opSteps_emit (e : Ev) : opSteps sk (.emit e) = 1 := by
  unfold opSteps
  simp [opSpec]

theorem opEvents_scope (h k : Nat) (feed : List Ev) :
    opEvents sk (.scope h k feed)
      = (wScopeOps sk h k feed).flatMap (opEvents sk) := by
  unfold opEvents
  simp only [opSpec, List.map_map]
  rw [show ((fun x : {op // op ∈ wScopeOps sk h k feed} =>
      opSpec sk x.val) = fun x => opSpec sk x.val) from rfl]
  rw [show (Prod.fst ∘ fun x : {op // op ∈ wScopeOps sk h k feed} =>
      opSpec sk x.val) = fun x => (opSpec sk x.val).1 from rfl]
  rw [attach_map_spec _ (fun op => (opSpec sk op).1), List.flatMap_def]

theorem opSteps_scope (h k : Nat) (feed : List Ev) :
    opSteps sk (.scope h k feed)
      = 1 + ((wScopeOps sk h k feed).map (opSteps sk)).sum := by
  unfold opSteps
  simp only [opSpec, List.map_map]
  rw [show (Prod.snd ∘ fun x : {op // op ∈ wScopeOps sk h k feed} =>
      opSpec sk x.val) = fun x => (opSpec sk x.val).2 from rfl]
  rw [attach_map_spec _ (fun op => (opSpec sk op).2)]

theorem opEvents_kid (h k s : Nat) (lastD : Option Nat)
    (kidBase i : Nat) (feed : List Ev) :
    opEvents sk (.kid h k s lastD kidBase i feed)
      = (wKidOps sk h k s lastD kidBase i feed).flatMap (opEvents sk) := by
  unfold opEvents
  simp only [opSpec, List.map_map]
  rw [show (Prod.fst ∘ fun x :
      {op // op ∈ wKidOps sk h k s lastD kidBase i feed} =>
      opSpec sk x.val) = fun x => (opSpec sk x.val).1 from rfl]
  rw [attach_map_spec _ (fun op => (opSpec sk op).1), List.flatMap_def]

theorem opSteps_kid (h k s : Nat) (lastD : Option Nat)
    (kidBase i : Nat) (feed : List Ev) :
    opSteps sk (.kid h k s lastD kidBase i feed)
      = 1 + ((wKidOps sk h k s lastD kidBase i feed).map
          (opSteps sk)).sum := by
  unfold opSteps
  simp only [opSpec, List.map_map]
  rw [show (Prod.snd ∘ fun x :
      {op // op ∈ wKidOps sk h k s lastD kidBase i feed} =>
      opSpec sk x.val) = fun x => (opSpec sk x.val).2 from rfl]
  rw [attach_map_spec _ (fun op => (opSpec sk op).2)]

/-- Every op costs at least its own step. -/
theorem opSteps_pos (op : WOp) : 1 ≤ opSteps sk op := by
  cases op with
  | emit e => rw [opSteps_emit]; omega
  | scope h k feed => rw [opSteps_scope]; omega
  | kid h k s lastD kidBase i feed => rw [opSteps_kid]; omega

-- ====================================== steps bounded by emissions

/-- Summed per-op bounds: if each op's steps undershoot three times
its emissions by one, the list's total steps undershoot by its
length. -/
private theorem steps_sum_le {X : List WOp}
    (hX : ∀ x ∈ X, opSteps sk x + 1 ≤ 3 * (opEvents sk x).length) :
    (X.map (opSteps sk)).sum + X.length
      ≤ 3 * (X.flatMap (opEvents sk)).length := by
  induction X with
  | nil => simp
  | cons a X ih =>
      have ha := hX a (List.mem_cons_self ..)
      have hrest := ih fun x hx => hX x (List.mem_cons_of_mem _ hx)
      simp only [List.map_cons, List.sum_cons, List.length_cons,
        List.flatMap_cons, List.length_append]
      omega

/-- A scope op expands to at least its two prologue emits. -/
private theorem wScopeOps_length (h k : Nat) (feed : List Ev) :
    2 ≤ (wScopeOps sk h k feed).length := by
  simp only [wScopeOps, List.append_assoc, List.length_append,
    List.length_cons, List.length_nil]
  omega

/-- A kid op expands to at least its wire emit, cons-first. -/
private theorem wKidOps_shape (h k s : Nat) (lastD : Option Nat)
    (kidBase i : Nat) (feed : List Ev) :
    ∃ e tail, wKidOps sk h k s lastD kidBase i feed
      = .emit e :: tail :=
  ⟨_, _, rfl⟩

/-- Steps are dominated by emissions: `steps + 1 ≤ 3 · emissions`,
uniformly over ops. Each expansion layer costs one step and is paid
for by the two prologue emits (scopes) or the wire emit (kids); the
`aux` recursion is strong induction on the expansion rank. -/
private theorem opSteps_le_aux :
    ∀ (n : Nat) (op : WOp), wopMeasure op ≤ n →
      opSteps sk op + 1 ≤ 3 * (opEvents sk op).length := by
  intro n
  induction n with
  | zero =>
      intro op hop
      match op with
      | .emit e =>
          rw [opSteps_emit, opEvents_emit]
          simp
      | .scope h k feed => simp [wopMeasure] at hop
      | .kid h k s lastD kidBase i feed => simp [wopMeasure] at hop
  | succ n ih =>
      intro op hop
      match op with
      | .emit e =>
          rw [opSteps_emit, opEvents_emit]
          simp
      | .scope h k feed =>
          rw [opSteps_scope, opEvents_scope]
          have hsum := steps_sum_le sk (X := wScopeOps sk h k feed)
            fun x hx => by
              rcases mem_wScopeOps sk hx with ⟨e, rfl⟩ | ⟨s', lD, kb, i', rfl⟩
              · rw [opSteps_emit, opEvents_emit]; simp
              · refine ih _ ?_
                simp only [wopMeasure] at hop ⊢
                omega
          have hlen := wScopeOps_length sk h k feed
          omega
      | .kid h k s lastD kidBase i feed =>
          rw [opSteps_kid, opEvents_kid]
          obtain ⟨e₀, tail, hshape⟩ :=
            wKidOps_shape sk h k s lastD kidBase i feed
          have hbound : ∀ x ∈ wKidOps sk h k s lastD kidBase i feed,
              opSteps sk x + 1 ≤ 3 * (opEvents sk x).length := by
            intro x hx
            rcases mem_wKidOps sk hx with ⟨e, rfl⟩ | ⟨hne, k', f', rfl⟩
            · rw [opSteps_emit, opEvents_emit]; simp
            · refine ih _ ?_
              simp only [wopMeasure] at hop ⊢
              omega
          cases tail with
          | nil =>
              -- the lone-wire case: compute both sides outright
              rw [hshape]
              simp only [List.map_cons, List.map_nil, List.sum_cons,
                List.sum_nil, List.flatMap_cons, List.flatMap_nil,
                opSteps_emit, opEvents_emit, List.append_nil,
                List.length_cons, List.length_nil]
              omega
          | cons t tail =>
              have hsum := steps_sum_le sk hbound
              have hlen : 2 ≤ (wKidOps sk h k s lastD kidBase
                  i feed).length := by
                rw [hshape]
                simp
              omega

/-- Steps are dominated by emissions, top form. -/
theorem opSteps_le (op : WOp) :
    opSteps sk op + 1 ≤ 3 * (opEvents sk op).length :=
  opSteps_le_aux sk (wopMeasure op) op (Nat.le_refl _)

-- ============================================ the fuel-sufficiency

/-- With fuel at least the worklist's total step count, the
interpreter's ghost futures ARE the fuel-free expansion: the two
semantics agree, and extra fuel is harmless. -/
theorem goEvents_eq_of_fuel :
    ∀ (fuel : Nat) (ops : List WOp),
      (ops.map (opSteps sk)).sum ≤ fuel →
      goEvents sk fuel ops = ops.flatMap (opEvents sk) := by
  intro fuel
  induction fuel with
  | zero =>
      intro ops h
      match ops with
      | [] => rfl
      | op :: rest =>
          exfalso
          have hpos := opSteps_pos sk op
          simp only [List.map_cons, List.sum_cons, Nat.le_zero] at h
          omega
  | succ f ih =>
      intro ops h
      match ops with
      | [] => rfl
      | .emit e :: rest =>
          show e :: goEvents sk f rest = _
          rw [List.flatMap_cons, opEvents_emit,
            ih rest (by
              rw [List.map_cons, List.sum_cons, opSteps_emit] at h
              omega)]
          rfl
      | .scope h' k feed :: rest =>
          show goEvents sk f (wScopeOps sk h' k feed ++ rest) = _
          rw [ih _ (by
              rw [List.map_cons, List.sum_cons, opSteps_scope] at h
              rw [List.map_append, List.sum_append]
              omega)]
          rw [List.flatMap_append, List.flatMap_cons, opEvents_scope]
      | .kid h' k s lastD kidBase i feed :: rest =>
          show goEvents sk f
            (wKidOps sk h' k s lastD kidBase i feed ++ rest) = _
          rw [ih _ (by
              rw [List.map_cons, List.sum_cons, opSteps_kid] at h
              rw [List.map_append, List.sum_append]
              omega)]
          rw [List.flatMap_append, List.flatMap_cons, opEvents_kid]

/-- `weaveFuel` suffices as soon as the opening worklist's emission
count is bounded by the event total — which the alignment supplies,
since the futures are exactly the manual traces' events. -/
theorem goEvents_weave
    (hlen : (((weaveOps sk).flatMap (opEvents sk)).length)
      ≤ totalEvents sk) :
    goEvents sk (weaveFuel sk) (weaveOps sk)
      = (weaveOps sk).flatMap (opEvents sk) := by
  refine goEvents_eq_of_fuel sk _ _ ?_
  have hsum := steps_sum_le sk
    (X := weaveOps sk) fun x _ => opSteps_le sk x
  unfold weaveFuel
  omega
