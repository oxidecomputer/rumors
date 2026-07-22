/-
T3, `wc_impossibility`: one fixed skeleton
defeats everything — for every capacity C ≥ 1 and EVERY pair of
work-conserving strategies, local or not, the muxed `wedge` session
reaches a stuck non-terminal state.

# The technique of record: the forced run (no fooling, no pigeonhole)

The adversary schedules endpoints and withholds R→I deliveries so that
every strategy consultation happens at a state whose enabled-push set
is a SINGLETON; work-conservation forces the push; the strategies are
never meaningfully consulted (the singleton-consultation design,
verified step-by-step in cross-examination). Mechanized as a
σ-free executor:

- `fstep` takes, in priority order, the first enabled base action, a
  forced push (only when the enabled-push set is exactly `[h]`), a
  forward delivery, and — LAST — a reverse delivery. Putting
  `deliver .R` last IS the adversary: R's question frame is released
  only when nothing else can move, which is exactly after the wall has
  parked. Delivering only-when-forced also guarantees no enabled
  action hides at the drain's fixed point.
- `fdrain_replay` lifts the executor's run to a `MReachable` run of ANY
  work-conserving pair: base and deliver arms are σ-free, and at each
  forced push the singleton makes work-conservation pin σ's answer.
- `stuckShape`/`stuckShapeNoHands` decode the drained state as stuck
  for every strategy pair at once: no base action, no delivery, and an
  empty enabled-push set (no room, or no committed hand) disable every
  arm of `mcanStep` regardless of σ.

# Why ∀ C collapses to four kernel anchors

The executor is run at a BOUNDED capacity b, and every b-run replays
verbatim at any C ≥ b: pushes fire only at pipe occupancy < b, where
`enabledPushes`/`firePush` cannot tell b from C (`enabledPushes_agree`,
`firePush_agree` — the guards are `length < ·`, monotone). On `wedge`
the wall is 7 frames against 3 consumed + 1 demux slot, so the run has
exactly four shapes:

- C ∈ {1, 2, 3}: the pipe fills with provisions and the walk parks
  committed — the stuck state pins on room (`enabledPushes = []` by
  the full pipe); one anchor per capacity, at b = C;
- C ≥ 4: room never runs out; work-conservation forces the deep reply
  INTO the pipe behind three permanently undeliverable provisions (the
  b = 4 drain ends with pipe `[w5, w5, w5, w3]` — FIFO burial, the
  empirical six-link cycle's anatomy), every hand empties, and the
  stuck state pins on `noHands`, which kills pushes at EVERY capacity;
  the single b = 4 anchor covers all C ≥ 4.

The adjudication's proof skeleton (a)–(f) lands as: (a) commits forced
— consumed silently, the executor takes them as first-enabled base
actions and T1 (`commit_totality`) is why no commit choice exists to
adversarially explore; (b)+(c) `push_singleton` + replay =
`fdrain_replay`; (d)+(e) resident and burial — the kernel-decided
b = 4 anchor; (f) closed-form stuck decode = `mstuck_of_stuckShape`.

The bracketing controls (the shipped policy jams, an idler completes,
the unbounded-slot variant completes, C = 0 is vacuous, the close
guard's must-fail pin) live in Mux/Controls.lean.
-/
import StreamingMirror.Mux.Instances

namespace StreamingMirror.Mux

open Model

-- ================================================= capacity transfer

/-- A nonempty enabled-push set implies pipe room: the room conjunct
gates the whole list. -/
theorem enabledPushes_room {sk : Skel} {b : Nat} {p : Party} {s : MState}
    (h : enabledPushes sk b p s ≠ []) : (s.pipe p).length < b := by
  by_cases hroom : (s.pipe p).length < b
  · exact hroom
  · exact absurd (by unfold enabledPushes; rw [if_neg hroom]) h

/-- Below a bound b with room, the enabled-push set cannot tell b from
any C ≥ b: the guards are `length < ·` and the filter is
capacity-free. -/
theorem enabledPushes_agree {sk : Skel} {b C : Nat} {p : Party}
    {s : MState} (hroom : (s.pipe p).length < b) (hbC : b ≤ C) :
    enabledPushes sk C p s = enabledPushes sk b p s := by
  unfold enabledPushes
  rw [if_pos hroom, if_pos (Nat.lt_of_lt_of_le hroom hbC)]

/-- `firePush`'s capacity twin of `enabledPushes_agree`: with room at
b, the push's effect is identical at every C ≥ b. -/
theorem firePush_agree {sk : Skel} {b C : Nat} {p : Party} {h : Nat}
    {s : MState} (hroom : (s.pipe p).length < b) (hbC : b ≤ C) :
    firePush sk C p h s = firePush sk b p h s := by
  unfold firePush
  rw [if_pos hroom, if_pos (Nat.lt_of_lt_of_le hroom hbC)]

-- ============================================= push-guard soundness

/-- A successful push certifies its own guard: room, a committed hand,
and a stream the party produces — the soundness half of the intended
`enabledPushes_spec` (Mux/Basic.lean's doors-open note). -/
theorem firePush_isSome_sound {sk : Skel} {C : Nat} {p : Party}
    {h : Nat} {s : MState} (hf : (firePush sk C p h s).isSome = true) :
    (s.pipe p).length < C ∧ h ∈ wireHeights sk p ∧
      holdsWire sk p h s.base = true := by
  simp only [firePush] at hf
  by_cases hroom : (s.pipe p).length < C
  case neg => rw [if_neg hroom] at hf; cases hf
  rw [if_pos hroom] at hf
  refine ⟨hroom, ?_⟩
  by_cases hrh : (h == sk.rootH) = true
  · rw [if_pos hrh] at hf
    have hh : h = sk.rootH := by simpa using hrh
    cases p with
    | I =>
        cases hio : s.base.iopenCh with
        | none => rw [hio] at hf; cases hf
        | some o =>
            cases o with
            | query => rw [hio] at hf; cases hf
            | wire =>
                refine ⟨by rw [wireHeights, hh]; exact List.mem_cons_self,
                  ?_⟩
                simp only [holdsWire]
                rw [if_pos hrh, hio]
                rfl
    | R =>
        cases hro : s.base.ropenCh with
        | none => rw [hro] at hf; cases hf
        | some o =>
            cases o with
            | query => rw [hro] at hf; cases hf
            | res => rw [hro] at hf; cases hf
            | wire =>
                refine ⟨by rw [wireHeights, hh]; exact List.mem_cons_self,
                  ?_⟩
                simp only [holdsWire]
                rw [if_pos hrh, hro]
                rfl
  · rw [if_neg hrh] at hf
    split at hf
    next i heq =>
      split at hf
      next hg =>
        rw [Bool.and_eq_true] at hg
        obtain ⟨hcont, hph⟩ := hg
        have hmemKeys : (p, h) ∈ sk.walkKeys := by
          simpa using hcont
        constructor
        · rw [wireHeights]
          refine List.mem_cons_of_mem _
            (List.mem_filterMap.mpr ⟨(p, h), hmemKeys, ?_⟩)
          simp
        · simp only [holdsWire]
          rw [if_neg hrh]
          simp [heq, hph, hmemKeys]
      next hg => cases hf
    next x heq => cases hf

/-- Membership introduction for the enabled-push set: room plus a held
stream the party produces. -/
theorem mem_enabledPushes_intro {sk : Skel} {C : Nat} {p : Party} {h : Nat}
    {s : MState} (hroom : (s.pipe p).length < C)
    (hmem : h ∈ wireHeights sk p)
    (hhold : holdsWire sk p h s.base = true) :
    h ∈ enabledPushes sk C p s := by
  unfold enabledPushes
  rw [if_pos hroom]
  exact List.mem_filter.mpr ⟨hmem, hhold⟩

-- ====================================== the mux arms, party-selected

/-- The strategy a party consults, as a function: the `push` arm's
party selection factored out so the replay lemma can quantify over
it. -/
def sideOf (σI σR : Strategy) : Party → Strategy
  | .I => σI
  | .R => σR

/-- `apply`'s base arm, named: base actions never consult σ or C. -/
theorem apply_base {sk : Skel} {ax : AxMode} {C : Nat} {σI σR : Strategy}
    {a : Action} {s : MState} :
    apply sk ax C σI σR (.base a) s = applyBase sk ax a s := rfl

/-- `apply`'s push arm, named through `sideOf`. -/
theorem apply_push {sk : Skel} {ax : AxMode} {C : Nat} {σI σR : Strategy}
    {p : Party} {s : MState} :
    apply sk ax C σI σR (.push p) s =
      match sideOf σI σR p sk (s.hist p) with
      | some h => firePush sk C p h s
      | none => none := by
  cases p <;> rfl

/-- The demux move, σ- and C-free: `apply`'s deliver arm verbatim, so
the executor and the stuck decode can run it without a strategy in
scope. -/
def deliverStep (p : Party) (s : MState) : Option MState :=
  match s.pipe p with
  | c :: rest =>
      if s.base.chan c == 0 then
        some { base := { s.base with chan := bump s.base.chan c 1 }
               pipe := fun q => if q == p then rest else s.pipe q
               hist := recordObs s.hist p.other
                 (.delivered (wireHeight c)) }
      else none
  | [] => none

/-- `apply`'s deliver arm, named. -/
theorem apply_deliver {sk : Skel} {ax : AxMode} {C : Nat}
    {σI σR : Strategy} {p : Party} {s : MState} :
    apply sk ax C σI σR (.deliver p) s = deliverStep p s := rfl

-- ================================================ the forced-run executor

/-- Push party `p`'s frame ONLY when work-conservation would force it:
the enabled-push set at the bounded capacity b is exactly a singleton.

The b-bounded guard is what makes one kernel run cover every C ≥ b
(`enabledPushes_agree`): a push taken here is a push every
work-conserving strategy takes at every larger capacity. -/
def forcedPush (sk : Skel) (b : Nat) (p : Party) (s : MState) :
    Option MState :=
  match enabledPushes sk b p s with
  | [h] => firePush sk b p h s
  | _ => none

/-- One step of the forced-run adversary: first enabled base action,
else a forced push (I before R), else a forward delivery, else — last,
the adversary's signature — a reverse delivery.

`deliver .R` last is the delivery-withholding schedule of the forced
run (the cross-examination's withholding schedule): R's question frame
crosses only when the
system is otherwise quiescent, i.e. after the provision wall has
parked the consumer. -/
def fstep (sk : Skel) (ax : AxMode) (b : Nat) (s : MState) :
    Option MState :=
  match (Model.allActions sk).firstM (fun a => applyBase sk ax a s) with
  | some s' => some s'
  | none =>
    match forcedPush sk b .I s with
    | some s' => some s'
    | none =>
      match forcedPush sk b .R s with
      | some s' => some s'
      | none =>
        match deliverStep .I s with
        | some s' => some s'
        | none => deliverStep .R s

/-- Run the forced-run adversary to quiescence (fuel-bounded). -/
def fdrain (sk : Skel) (ax : AxMode) (b : Nat) : Nat → MState → MState
  | 0, s => s
  | fuel + 1, s =>
      match fstep sk ax b s with
      | some s' => fdrain sk ax b fuel s'
      | none => s

/-- `firstM` over `Option` succeeds only through one of its elements. -/
private theorem firstM_eq_some {α β : Type _} {f : α → Option β} {b : β} :
    ∀ {l : List α}, l.firstM f = some b → ∃ a ∈ l, f a = some b := by
  intro l
  induction l with
  | nil => intro h; simp [List.firstM] at h
  | cons x xs ih =>
      intro h
      cases hfx : f x with
      | some b' =>
          simp [List.firstM, hfx] at h
          exact ⟨x, List.mem_cons_self .., by rw [hfx, h]⟩
      | none =>
          simp [List.firstM, hfx] at h
          obtain ⟨a, ha, hfa⟩ := ih h
          exact ⟨a, List.mem_cons_of_mem x ha, hfa⟩

-- ======================================================== the replay

/-- A forced push replays under any work-conserving strategy: the
singleton enabled set transfers from b to C (`enabledPushes_agree`),
work-conservation makes σ name its only member, and the push's effect
is capacity-blind (`firePush_agree`). -/
private theorem forced_replay {sk : Skel} {ax : AxMode} {b C : Nat}
    {σI σR : Strategy} {s s' : MState} (hbC : b ≤ C) (p : Party)
    (hWC : WorkConserving p (sideOf σI σR p))
    (hr : MReachable sk ax C σI σR s)
    (hp : forcedPush sk b p s = some s') :
    MReachable sk ax C σI σR s' := by
  obtain ⟨hh, hE, hfire⟩ : ∃ hh, enabledPushes sk b p s = [hh] ∧
      firePush sk b p hh s = some s' := by
    unfold forcedPush at hp
    cases hE : enabledPushes sk b p s with
    | nil => rw [hE] at hp; cases hp
    | cons x tl =>
        cases tl with
        | nil => rw [hE] at hp; exact ⟨x, rfl, hp⟩
        | cons y tl' => rw [hE] at hp; cases hp
  have hroom : (s.pipe p).length < b :=
    enabledPushes_room (by rw [hE]; simp)
  have hEC : enabledPushes sk C p s = [hh] := by
    rw [enabledPushes_agree hroom hbC, hE]
  obtain ⟨h', hσ, hmem⟩ := hWC sk C s ⟨ax, C, σI, σR, hr⟩
    (by rw [hEC]; simp)
  rw [hEC] at hmem
  have hhh : h' = hh := by simpa using hmem
  subst hhh
  have happ : apply sk ax C σI σR (.push p) s = some s' := by
    rw [apply_push, hσ]
    show firePush sk C p h' s = some s'
    rw [firePush_agree hroom hbC]
    exact hfire
  exact .step (.push p) hr happ

/-- One executor step replays as one `MReachable` step of any
work-conserving pair at any capacity C ≥ b. -/
private theorem fstep_replay {sk : Skel} {ax : AxMode} {b C : Nat}
    {σI σR : Strategy} {s s' : MState} (hbC : b ≤ C)
    (hWI : WorkConserving .I σI) (hWR : WorkConserving .R σR)
    (hr : MReachable sk ax C σI σR s) (h : fstep sk ax b s = some s') :
    MReachable sk ax C σI σR s' := by
  unfold fstep at h
  cases hbase : (Model.allActions sk).firstM
      (fun a => applyBase sk ax a s) with
  | some s₁ =>
      rw [hbase] at h
      cases h
      obtain ⟨a, -, hap⟩ := firstM_eq_some hbase
      exact .step (.base a) hr hap
  | none =>
      rw [hbase] at h
      cases hpI : forcedPush sk b .I s with
      | some s₁ =>
          rw [hpI] at h
          cases h
          exact forced_replay hbC .I hWI hr hpI
      | none =>
          rw [hpI] at h
          cases hpR : forcedPush sk b .R s with
          | some s₁ =>
              rw [hpR] at h
              cases h
              exact forced_replay hbC .R hWR hr hpR
          | none =>
              rw [hpR] at h
              cases hdI : deliverStep .I s with
              | some s₁ =>
                  rw [hdI] at h
                  cases h
                  exact .step (.deliver .I) hr (apply_deliver.trans hdI)
              | none =>
                  rw [hdI] at h
                  exact .step (.deliver .R) hr (apply_deliver.trans h)

/-- The forced run replays end to end: the drained state is reachable
under EVERY work-conserving pair at every capacity C ≥ b — the
strategies are never meaningfully consulted. -/
theorem fdrain_replay {sk : Skel} {ax : AxMode} {b C : Nat}
    {σI σR : Strategy} (hbC : b ≤ C)
    (hWI : WorkConserving .I σI) (hWR : WorkConserving .R σR)
    (fuel : Nat) :
    ∀ {s : MState}, MReachable sk ax C σI σR s →
      MReachable sk ax C σI σR (fdrain sk ax b fuel s) := by
  induction fuel with
  | zero => intro s h; exact h
  | succ n ih =>
      intro s h
      unfold fdrain
      cases hf : fstep sk ax b s with
      | none => exact h
      | some s' => exact ih (fstep_replay hbC hWI hWR h hf)

-- ==================================================== the stuck decode

/-- Every σ-free arm of `mcanStep` is disabled: no base action applies
and neither demux can deliver. -/
def baseDeliverDisabled (sk : Skel) (ax : AxMode) (s : MState) : Bool :=
  ((Model.allActions sk).all fun a => (applyBase sk ax a s).isNone) &&
    (deliverStep .I s).isNone && (deliverStep .R s).isNone

/-- Party `p` holds no committed wire frame on any stream it produces:
the C-uniform reason pushes are disabled (the C ≥ 4 stuck flavor,
where room never runs out but every hand has emptied). -/
def noHands (sk : Skel) (p : Party) (s : MState) : Bool :=
  ((wireHeights sk p).filter fun h => holdsWire sk p h s.base).isEmpty

/-- The fixed-capacity stuck certificate: non-terminal, σ-free arms
disabled, and both enabled-push sets empty (at C ∈ {1, 2, 3} the pipe
is full, so emptiness comes from the room conjunct). -/
def stuckShape (sk : Skel) (ax : AxMode) (C : Nat) (s : MState) : Bool :=
  !mterminal sk s && baseDeliverDisabled sk ax s &&
    (enabledPushes sk C .I s).isEmpty && (enabledPushes sk C .R s).isEmpty

/-- The capacity-uniform stuck certificate: non-terminal, σ-free arms
disabled, and NO hands — pushes are disabled at every capacity because
nothing is committed, not because room ran out. -/
def stuckShapeNoHands (sk : Skel) (ax : AxMode) (s : MState) : Bool :=
  !mterminal sk s && baseDeliverDisabled sk ax s &&
    noHands sk .I s && noHands sk .R s

/-- With an empty enabled-push set, `p`'s push is disabled for every
strategy: a named frame could only fire through room + a held stream,
which would put it in the set. -/
theorem push_none_of_enabledPushes_nil {sk : Skel} {ax : AxMode}
    {C : Nat} {σI σR : Strategy} {p : Party} {s : MState}
    (hE : enabledPushes sk C p s = []) :
    apply sk ax C σI σR (.push p) s = none := by
  rw [apply_push]
  cases hσ : sideOf σI σR p sk (s.hist p) with
  | none => rfl
  | some hh =>
      show firePush sk C p hh s = none
      cases hf : firePush sk C p hh s with
      | none => rfl
      | some s₁ =>
          exfalso
          obtain ⟨hroom, hmem, hhold⟩ :=
            firePush_isSome_sound (C := C) (by rw [hf]; rfl)
          have := mem_enabledPushes_intro hroom hmem hhold
          rw [hE] at this
          cases this

/-- `noHands` empties the enabled-push set at EVERY capacity: the
filter is the set's capacity-free half. -/
theorem enabledPushes_nil_of_noHands {sk : Skel} {C : Nat} {p : Party}
    {s : MState} (hnh : noHands sk p s = true) :
    enabledPushes sk C p s = [] := by
  have hnil : ((wireHeights sk p).filter
      fun h => holdsWire sk p h s.base) = [] := by
    cases hl : (wireHeights sk p).filter
        (fun h => holdsWire sk p h s.base) with
    | nil => rfl
    | cons x xs => unfold noHands at hnh; rw [hl] at hnh; cases hnh
  unfold enabledPushes
  split
  · exact hnil
  · rfl

/-- Assemble `mstuck` from per-arm disabledness, for every strategy
pair at once. -/
private theorem mstuck_intro {sk : Skel} {ax : AxMode} {C : Nat}
    {σI σR : Strategy} {s : MState} (hterm : mterminal sk s = false)
    (hnone : ∀ a ∈ allMActions sk, apply sk ax C σI σR a s = none) :
    mstuck sk ax C σI σR s = true := by
  have hcan : mcanStep sk ax C σI σR s = false := by
    rw [mcanStep, List.any_eq_false]
    intro a ha
    rw [hnone a ha]
    simp
  rw [mstuck, hterm, hcan]
  rfl

/-- Decode the σ-free conjuncts shared by both certificates into the
per-arm facts `mstuck_intro` consumes. -/
private theorem base_deliver_none {sk : Skel} {ax : AxMode}
    {s : MState} (hbd : baseDeliverDisabled sk ax s = true) :
    (∀ a ∈ Model.allActions sk, applyBase sk ax a s = none) ∧
      deliverStep .I s = none ∧ deliverStep .R s = none := by
  simp only [baseDeliverDisabled, Bool.and_eq_true, List.all_eq_true,
    Option.isNone_iff_eq_none] at hbd
  exact ⟨fun a ha => hbd.1.1 a ha, hbd.1.2, hbd.2⟩

/-- Dispatch every enumerated mux action to its disabledness fact. -/
private theorem all_none_of_parts {sk : Skel} {ax : AxMode} {C : Nat}
    {σI σR : Strategy} {s : MState}
    (hbase : ∀ a ∈ Model.allActions sk, applyBase sk ax a s = none)
    (hdI : deliverStep .I s = none) (hdR : deliverStep .R s = none)
    (hpI : enabledPushes sk C .I s = [])
    (hpR : enabledPushes sk C .R s = []) :
    ∀ a ∈ allMActions sk, apply sk ax C σI σR a s = none := by
  intro a ha
  rw [allMActions] at ha
  rcases List.mem_append.mp ha with hb | hm
  · obtain ⟨a₀, ha₀, rfl⟩ := List.mem_map.mp hb
    rw [apply_base]
    exact hbase a₀ ha₀
  · simp only [List.mem_cons, List.not_mem_nil, or_false] at hm
    rcases hm with rfl | rfl | rfl | rfl
    · exact push_none_of_enabledPushes_nil hpI
    · exact push_none_of_enabledPushes_nil hpR
    · exact apply_deliver.trans hdI
    · exact apply_deliver.trans hdR

/-- The fixed-capacity certificate yields `mstuck` for every strategy
pair: the σ-quantified deadlock verdict from a σ-free kernel check. -/
theorem mstuck_of_stuckShape {sk : Skel} {ax : AxMode} {C : Nat}
    {s : MState} (σI σR : Strategy)
    (h : stuckShape sk ax C s = true) :
    mstuck sk ax C σI σR s = true := by
  simp only [stuckShape, Bool.and_eq_true, Bool.not_eq_true',
    List.isEmpty_iff] at h
  obtain ⟨⟨⟨hterm, hbd⟩, hpI⟩, hpR⟩ := h
  obtain ⟨hbase, hdI, hdR⟩ := base_deliver_none hbd
  exact mstuck_intro hterm (all_none_of_parts hbase hdI hdR hpI hpR)

/-- The capacity-uniform certificate yields `mstuck` at EVERY capacity
for every strategy pair: `noHands` disables pushes without consulting
the room conjunct. -/
theorem mstuck_of_stuckShapeNoHands {sk : Skel} {ax : AxMode}
    {s : MState} (C : Nat) (σI σR : Strategy)
    (h : stuckShapeNoHands sk ax s = true) :
    mstuck sk ax C σI σR s = true := by
  simp only [stuckShapeNoHands, Bool.and_eq_true, Bool.not_eq_true'] at h
  obtain ⟨⟨⟨hterm, hbd⟩, hnI⟩, hnR⟩ := h
  obtain ⟨hbase, hdI, hdR⟩ := base_deliver_none hbd
  exact mstuck_intro hterm (all_none_of_parts hbase hdI hdR
    (enabledPushes_nil_of_noHands hnI) (enabledPushes_nil_of_noHands hnR))

-- ================================================== the kernel anchors

set_option maxRecDepth 16000 in
set_option maxHeartbeats 1000000 in
/-- The forced run jams `wedge` at C = 1: the pipe-full park — the
provision wall fills the single pipe cell, the dispute walk parks
committed, and the deep reply is never even pushable. -/
theorem wedge_forced_stuck_C1 :
    stuckShape wedge .impl 1
      (fdrain wedge .impl 1 300 (init wedge)) = true := by
  decide

set_option maxRecDepth 16000 in
set_option maxHeartbeats 1000000 in
/-- The forced run jams `wedge` at C = 2 — same park, one more
provision in flight. -/
theorem wedge_forced_stuck_C2 :
    stuckShape wedge .impl 2
      (fdrain wedge .impl 2 300 (init wedge)) = true := by
  decide

set_option maxRecDepth 16000 in
set_option maxHeartbeats 1000000 in
/-- The forced run jams `wedge` at C = 3 — the whole wall in flight,
the dispute walk's hand empty, the deep walk parked on a full pipe. -/
theorem wedge_forced_stuck_C3 :
    stuckShape wedge .impl 3
      (fdrain wedge .impl 3 300 (init wedge)) = true := by
  decide

set_option maxRecDepth 16000 in
set_option maxHeartbeats 1000000 in
/-- The forced run jams `wedge` in the capacity-uniform shape: at
b = 4 the pipe never fills — work-conservation buries the deep reply
behind three permanently undeliverable provisions (final pipe
`[w5, w5, w5, w3]`) and every hand empties, so the certificate is
`noHands` and covers every C ≥ 4. -/
theorem wedge_forced_stuck_ge4 :
    stuckShapeNoHands wedge .impl
      (fdrain wedge .impl 4 300 (init wedge)) = true := by
  decide

-- ===================================================== the impossibility

/-- T3, `wc_impossibility`: one fixed, realizable,
margin-0 skeleton defeats every work-conserving pair at every capacity.

No locality hypotheses: even an omniscient work-conserving strategy
dies (the right to idle, not information, is
the entire frontier; the σ-side of the trichotomy is T4's, and the
idler control in Mux/Controls.lean shows the same skeleton completes
when idling is allowed). The un-muxed `wedge` session is inside the
kernel-proven `Sched.deadlock_free` class (`wedge_wellFormed`,
`wedge_margin0`), so the stuck state indicts the mux transport alone;
the Rust corollary rides the deterministic bridge pair of
`src/tree/mirror/streaming/tests/wedge.rs` (the committed proptest
seeds realize the jam mechanism, not the byte-exact shape; the bridge
pins the decoded skeleton to the `wedge` literal, heights 6 and 32).

The hypothesis class is kernel-inhabited: the shipped policy is a
member (`bottomMostReady_wc`, Mux/Proofs/Inhabitation.lean — and a
`LocalStrategy` member at that, `bottomMostReady_local`), so this
∀-class impossibility is not satisfiable-empty. Capacity is
message-denominated, and the impossibility transfers to byte
denomination unweakened (Mux/Basic.lean, # The byte-denomination
caveat). -/
theorem wc_impossibility (C : Nat) (hC : 1 ≤ C) (σI σR : Strategy)
    (hWI : WorkConserving .I σI) (hWR : WorkConserving .R σR) :
    ¬ MuxDeadlockFree wedge .impl C σI σR := by
  intro hdf
  rcases (by omega : C = 1 ∨ C = 2 ∨ C = 3 ∨ 4 ≤ C) with rfl | rfl | rfl | h4
  · have hr := fdrain_replay (Nat.le_refl 1) hWI hWR 300
      (MReachable.init (sk := wedge) (ax := .impl))
    have hstuck := mstuck_of_stuckShape σI σR wedge_forced_stuck_C1
    rw [hdf _ hr] at hstuck
    exact Bool.false_ne_true hstuck
  · have hr := fdrain_replay (Nat.le_refl 2) hWI hWR 300
      (MReachable.init (sk := wedge) (ax := .impl))
    have hstuck := mstuck_of_stuckShape σI σR wedge_forced_stuck_C2
    rw [hdf _ hr] at hstuck
    exact Bool.false_ne_true hstuck
  · have hr := fdrain_replay (Nat.le_refl 3) hWI hWR 300
      (MReachable.init (sk := wedge) (ax := .impl))
    have hstuck := mstuck_of_stuckShape σI σR wedge_forced_stuck_C3
    rw [hdf _ hr] at hstuck
    exact Bool.false_ne_true hstuck
  · have hr := fdrain_replay h4 hWI hWR 300
      (MReachable.init (sk := wedge) (ax := .impl))
    have hstuck := mstuck_of_stuckShapeNoHands C σI σR wedge_forced_stuck_ge4
    rw [hdf _ hr] at hstuck
    exact Bool.false_ne_true hstuck

end StreamingMirror.Mux
