/-
T8's impossibility half, `wc_impossibility_K` (MUX-PROGRESS.md, the
T8 log entry; design/eager-absorption.md §1): K-deep reply parking is
a real mitigation and not a cure — for every FIXED parking depth the
widened wedge family kills every work-conserving pair at every pipe
capacity.

# Per-direction depths (the single-socket design's advertisement)

The deployed configuration advertises parking depths per direction
(design/single-socket.md: parties may run K_I ≠ K_R, each sender gated
at its peer's advertised value), so the variant here carries two
parameters: `KI` is the depth of the initiator's demux (gating
`deliver .R`, the R→I frames), `KR` the responder's (gating
`deliver .I`). The burial mechanism of the wedge lives entirely in the
I→R direction — the provision wall and the deep reply both ride
initiator streams into the responder's parked cells — so the witness
family scales past `KR`, the depth of the direction it kills, and the
statement is uniform in `KI`: the reverse direction's depth never
bites, because the forced run's stuck states carry an EMPTY R→I pipe
(the executor releases R's frames only when nothing else moves, and
each such release replays at every deeper `KI` — the deliver guard is
monotone in the receiving depth). A single-K statement would undersell
this: the theorem kills every (K_I, K_R) pair with K_R at the anchored
depths, not just the diagonal.

# The technique: the T3 forced run, transported

Everything is the WcImpossibility.lean machinery with the deliver arm
re-gated: the σ-free executor takes base actions first, forced
(singleton) pushes next, deliveries last with `deliver .R` at the very
end (the withholding adversary); the b-bounded replay lifts the one
kernel run to every work-conserving pair, every C ≥ b (push guards
monotone in C, `enabledPushes_agree`/`firePush_agree`), and now every
KI ≥ 1 (`deliverStepK_mono_I`); the stuck decode is σ-free and
KI-free (its `deliver .R` conjunct is pipe-emptiness, not a depth
check). Per anchored `KR` the ∀C split is T3's: pipe-full parks at
small C, and a `noHands` burial certificate — every hand empty, the
deep reply a permanent pipe resident behind undeliverable provisions —
covering all larger C at once (no C-induction).

# What is anchored and what stays open

Kernel anchors land `KR ∈ {1, 2, 3}` (each with its own four C-shape
anchors on `wedgeW (KR + 5)`, the regression shape widened past the
depth). The construction is uniform in `KR` — scale the wall past the
parked cells and the same burial replays — but each depth needs its
own kernel replay because the stuck certificate's `deliver .I`
conjunct reads the concrete depth; a ∀KR statement would need a
symbolic-in-KR run script, which the `decide`-only discipline does not
reach. Honest status: `KR ≥ 4` is [open] at theorem tier and covered
by the family argument at [derived] tier; `wedgeW`'s width formula
gives slack past the probe's minimal jamming width at every anchored
depth. At `(KI, KR) = (1, 1)` the variant's SEMANTICS degenerates to
the record harness exactly (`deliverStepK_one` — the deliver arms
coincide), and the witness is `wedge` itself (`wedgeW_six`); the
THEOREM does not degenerate to T3's `wc_impossibility`, because
`KWorkConserving` demands the push obligation over the larger
`KMReachableAny` universe — a strictly stronger hypothesis on the
strategy — so the (1, 1) instance is a weaker statement than T3, not
the same one.

The `KWorkConserving` class quantifies over the K-variant's own
reachable universe (`KMReachableAny`), mirroring the record class —
a strategy is constrained only where the composition can actually put
it. The right to idle stays the entire frontier: the class hypothesis
has the same shape as T3's, met at a deeper wall.
-/
import StreamingMirror.Mux.Proofs.WcImpossibility

namespace StreamingMirror.Mux

open Model
open Pin (sc)

-- ===================================================== the K semantics

/-- The advertised parking depth of the RECEIVING party of pipe `p`:
frames from the initiator park at the responder's depth and vice
versa (module doc; design/single-socket.md's per-direction
advertisement). -/
def recvDepth (KI KR : Nat) : Party → Nat
  | .I => KR
  | .R => KI

/-- The K-deep demux move: pipe head into the receiving party's parked
cells, gated by THAT party's advertised depth — `deliver .I` fills the
responder's cells (depth `KR`), `deliver .R` the initiator's (depth
`KI`). At depths (1, 1) this is the record harness's cap-1 slot
(`deliverStepK_one`). -/
def deliverStepK (KI KR : Nat) (p : Party) (s : MState) :
    Option MState :=
  match s.pipe p with
  | c :: rest =>
      if s.base.chan c < recvDepth KI KR p then
        some { base := { s.base with chan := bump s.base.chan c 1 }
               pipe := fun q => if q == p then rest else s.pipe q
               hist := recordObs s.hist p.other
                 (.delivered (wireHeight c)) }
      else none
  | [] => none

/-- The K-variant transition: the record harness with per-direction
parking depths on the deliver arm; base and push arms shared with
`Mux.apply` definitionally. -/
def applyK (sk : Skel) (ax : AxMode) (KI KR C : Nat) (σI σR : Strategy)
    (a : MAction) (s : MState) : Option MState :=
  match a with
  | .deliver p => deliverStepK KI KR p s
  | a => apply sk ax C σI σR a s

/-- At depths (1, 1) the K-variant deliver is the record deliver:
`chan < 1` is `chan = 0`. -/
theorem deliverStepK_one (p : Party) (s : MState) :
    deliverStepK 1 1 p s = deliverStep p s := by
  unfold deliverStepK deliverStep
  cases hp : s.pipe p with
  | nil => rfl
  | cons c rest =>
      have hd : recvDepth 1 1 p = 1 := by cases p <;> rfl
      rw [hd]
      by_cases hc : s.base.chan c = 0
      · simp [hc]
      · simp [Nat.lt_one_iff, hc]

/-- The deliver guard is monotone in the receiving depth: a delivery
legal at depths `(KIa, KR)` is legal at any `KIb ≥ KIa`, with the same
effect — the KI-lift of the forced run's reverse deliveries. -/
theorem deliverStepK_mono_I {KIa KIb KR : Nat} (hK : KIa ≤ KIb)
    {p : Party} {s s' : MState}
    (h : deliverStepK KIa KR p s = some s') :
    deliverStepK KIb KR p s = some s' := by
  unfold deliverStepK at h ⊢
  cases hp : s.pipe p with
  | nil => rw [hp] at h; cases h
  | cons c rest =>
      simp only [hp] at h ⊢
      split at h
      case isFalse => cases h
      case isTrue hg =>
        have hmono : recvDepth KIa KR p ≤ recvDepth KIb KR p := by
          cases p <;> simp [recvDepth] <;> omega
        rw [if_pos (by omega)]
        exact h

-- ======================================================= the K spine

/-- Some process, mux, or K-demux can act. -/
def mcanStepK (sk : Skel) (ax : AxMode) (KI KR C : Nat)
    (σI σR : Strategy) (s : MState) : Bool :=
  (allMActions sk).any fun a => (applyK sk ax KI KR C σI σR a s).isSome

/-- The K-variant deadlock predicate. -/
def mstuckK (sk : Skel) (ax : AxMode) (KI KR C : Nat)
    (σI σR : Strategy) (s : MState) : Bool :=
  !mterminal sk s && !mcanStepK sk ax KI KR C σI σR s

/-- Reachability of the K-variant composition. -/
inductive KMReachable (sk : Skel) (ax : AxMode) (KI KR C : Nat)
    (σI σR : Strategy) : MState → Prop
  | init : KMReachable sk ax KI KR C σI σR (init sk)
  | step {s s' : MState} (a : MAction) :
      KMReachable sk ax KI KR C σI σR s →
      applyK sk ax KI KR C σI σR a s = some s' →
      KMReachable sk ax KI KR C σI σR s'

/-- Deadlock freedom of the K-variant composition. -/
def MuxDeadlockFreeK (sk : Skel) (ax : AxMode) (KI KR C : Nat)
    (σI σR : Strategy) : Prop :=
  ∀ s, KMReachable sk ax KI KR C σI σR s →
    mstuckK sk ax KI KR C σI σR s = false

/-- Reachable under SOME depths, mode, capacity, and pair. -/
def KMReachableAny (sk : Skel) (s : MState) : Prop :=
  ∃ (ax : AxMode) (KI KR C : Nat) (σI σR : Strategy),
    KMReachable sk ax KI KR C σI σR s

/-- σ pushes whenever it holds a pushable frame, at every K-reachable
state: the work-conserving class over the K-variant's own universe
(the record class quantifies over record-reachable states, which
K-parked compositions leave). -/
def KWorkConserving (p : Party) (σ : Strategy) : Prop :=
  ∀ (sk : Skel) (C : Nat) (s : MState), KMReachableAny sk s →
    enabledPushes sk C p s ≠ [] →
    ∃ h, σ sk (s.hist p) = some h ∧ h ∈ enabledPushes sk C p s

-- ================================================= the witness family

/-- The regression shape at provision width `w`: the root disputes its
FIRST radix child — a chain descending disputed levels to a leaf
request — and takes `w` whole-subtree provisions behind it on the same
stream. `wedgeW 6 = wedge` (`wedgeW_six`); the T8 anchors use
`wedgeW (KR + 5)`, the wall scaled past the parked cells. Margin-0 by
construction; the statement-path copy of the executable tier's
`Gen.wedgeFam` (nothing of record may quantify over Gen.lean). -/
def wedgeW (w : Nat) : Skel :=
  let rootH := 6
  let chainTop := w + 2
  let chain := (List.range (rootH - 2)).map fun k =>
    let h := rootH - 2 - k
    if h == 1 then sc .D 1 [] (leafReqs := 1)
    else sc .D h [chainTop + k + 1]
  { scopes :=
      sc .D rootH ((List.range (w + 1)).map (· + 1))
        :: sc .D (rootH - 1) [chainTop]
        :: (List.range w).map (fun _ => sc .R (rootH - 1) [])
        ++ chain
    rootH := rootH, fan := w + 1, capLevel := 1 }

/-- The family passes through the T0 witness literal at width 6,
field by field (`Skel` derives no `DecidableEq`; the fields do). -/
theorem wedgeW_six :
    (wedgeW 6).scopes = wedge.scopes ∧ (wedgeW 6).rootH = wedge.rootH
      ∧ (wedgeW 6).fan = wedge.fan
      ∧ (wedgeW 6).capLevel = wedge.capLevel := by decide

-- ================================================ the forced-run executor

/-- One step of the K-variant forced-run adversary: first enabled base
action, else a forced (singleton) push at the b-bound, else a forward
delivery, else — last, the withholding adversary — a reverse delivery.
The executor runs at `KI = 1`; the replay lifts its reverse deliveries
to every deeper `KI` (`deliverStepK_mono_I`). -/
def fstepK (sk : Skel) (ax : AxMode) (KR b : Nat) (s : MState) :
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
        match deliverStepK 1 KR .I s with
        | some s' => some s'
        | none => deliverStepK 1 KR .R s

/-- Run the K-variant forced-run adversary to quiescence. -/
def fdrainK (sk : Skel) (ax : AxMode) (KR b : Nat) :
    Nat → MState → MState
  | 0, s => s
  | fuel + 1, s =>
      match fstepK sk ax KR b s with
      | some s' => fdrainK sk ax KR b fuel s'
      | none => s

-- ==================================================== the K replay

/-- `applyK`'s base arm, named. -/
private theorem applyK_base {sk : Skel} {ax : AxMode} {KI KR C : Nat}
    {σI σR : Strategy} {a : Action} {s : MState} :
    applyK sk ax KI KR C σI σR (.base a) s = applyBase sk ax a s := rfl

/-- `applyK`'s push arm is the record push arm. -/
private theorem applyK_push {sk : Skel} {ax : AxMode} {KI KR C : Nat}
    {σI σR : Strategy} {p : Party} {s : MState} :
    applyK sk ax KI KR C σI σR (.push p) s
      = apply sk ax C σI σR (.push p) s := rfl

/-- `deliver .I` never reads `KI`: the forward direction parks at the
responder's depth alone. -/
private theorem deliverStepK_I_free (KI KI' KR : Nat) (s : MState) :
    deliverStepK KI KR .I s = deliverStepK KI' KR .I s := rfl

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

/-- A forced push replays under any `KWorkConserving` strategy at any
capacity `C ≥ b`: the T3 `forced_replay`, transported to the K
universe. -/
private theorem forcedK_replay {sk : Skel} {ax : AxMode}
    {KI KR b C : Nat} {σI σR : Strategy} {s s' : MState} (hbC : b ≤ C)
    (p : Party)
    (hWC : KWorkConserving p (sideOf σI σR p))
    (hr : KMReachable sk ax KI KR C σI σR s)
    (hp : forcedPush sk b p s = some s') :
    KMReachable sk ax KI KR C σI σR s' := by
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
  obtain ⟨h', hσ, hmem⟩ := hWC sk C s ⟨ax, KI, KR, C, σI, σR, hr⟩
    (by rw [hEC]; simp)
  rw [hEC] at hmem
  have hhh : h' = hh := by simpa using hmem
  subst hhh
  have happ : applyK sk ax KI KR C σI σR (.push p) s = some s' := by
    rw [applyK_push, apply_push, hσ]
    show firePush sk C p h' s = some s'
    rw [firePush_agree hroom hbC]
    exact hfire
  exact .step (.push p) hr happ

/-- One executor step replays as one `KMReachable` step of any
`KWorkConserving` pair, at any capacity `C ≥ b` and any depth
`KI ≥ 1`: base and forward-deliver arms are (σ, C, KI)-free, forced
pushes transfer by singleton work conservation, and the executor's
reverse deliveries (taken at the floor depth 1) lift by guard
monotonicity. -/
private theorem fstepK_replay {sk : Skel} {ax : AxMode}
    {KI KR b C : Nat} {σI σR : Strategy} {s s' : MState} (hbC : b ≤ C)
    (hKI : 1 ≤ KI)
    (hWI : KWorkConserving .I σI) (hWR : KWorkConserving .R σR)
    (hr : KMReachable sk ax KI KR C σI σR s)
    (h : fstepK sk ax KR b s = some s') :
    KMReachable sk ax KI KR C σI σR s' := by
  unfold fstepK at h
  cases hbase : (Model.allActions sk).firstM
      (fun a => applyBase sk ax a s) with
  | some s₁ =>
      rw [hbase] at h
      cases h
      obtain ⟨a, -, hap⟩ := firstM_eq_some hbase
      exact .step (.base a) hr (applyK_base.trans hap)
  | none =>
      rw [hbase] at h
      cases hpI : forcedPush sk b .I s with
      | some s₁ =>
          rw [hpI] at h
          cases h
          exact forcedK_replay hbC .I hWI hr hpI
      | none =>
          rw [hpI] at h
          cases hpR : forcedPush sk b .R s with
          | some s₁ =>
              rw [hpR] at h
              cases h
              exact forcedK_replay hbC .R hWR hr hpR
          | none =>
              rw [hpR] at h
              cases hdI : deliverStepK 1 KR .I s with
              | some s₁ =>
                  rw [hdI] at h
                  cases h
                  exact .step (.deliver .I) hr
                    ((deliverStepK_I_free KI 1 KR s).trans hdI)
              | none =>
                  rw [hdI] at h
                  exact .step (.deliver .R) hr
                    (deliverStepK_mono_I hKI h)

/-- The forced run replays end to end: the drained state is reachable
under EVERY `KWorkConserving` pair at every `C ≥ b` and `KI ≥ 1` — the
strategies and the reverse depth are never meaningfully consulted. -/
theorem fdrainK_replay {sk : Skel} {ax : AxMode} {KI KR b C : Nat}
    {σI σR : Strategy} (hbC : b ≤ C) (hKI : 1 ≤ KI)
    (hWI : KWorkConserving .I σI) (hWR : KWorkConserving .R σR)
    (fuel : Nat) :
    ∀ {s : MState}, KMReachable sk ax KI KR C σI σR s →
      KMReachable sk ax KI KR C σI σR (fdrainK sk ax KR b fuel s) := by
  induction fuel with
  | zero => intro s h; exact h
  | succ n ih =>
      intro s h
      unfold fdrainK
      cases hf : fstepK sk ax KR b s with
      | none => exact h
      | some s' => exact ih (fstepK_replay hbC hKI hWI hWR h hf)

-- ==================================================== the stuck decode

/-- The K-variant's σ-free and KI-free disabled certificate: no base
action applies, the forward deliver is depth-blocked (`deliver .I`
never reads `KI`), and the reverse pipe is EMPTY — which disables
`deliver .R` at every depth, the conjunct that buys KI-uniformity
(module doc). -/
def baseDeliverDisabledK (sk : Skel) (ax : AxMode) (KR : Nat)
    (s : MState) : Bool :=
  ((Model.allActions sk).all fun a => (applyBase sk ax a s).isNone) &&
    (deliverStepK 1 KR .I s).isNone && (s.pipe .R).isEmpty

/-- The fixed-capacity stuck certificate (the pipe-full park). -/
def stuckShapeK (sk : Skel) (ax : AxMode) (KR C : Nat) (s : MState) :
    Bool :=
  !mterminal sk s && baseDeliverDisabledK sk ax KR s &&
    (enabledPushes sk C .I s).isEmpty && (enabledPushes sk C .R s).isEmpty

/-- The capacity-uniform stuck certificate (the burial: no hands at
any capacity). -/
def stuckShapeKNoHands (sk : Skel) (ax : AxMode) (KR : Nat)
    (s : MState) : Bool :=
  !mterminal sk s && baseDeliverDisabledK sk ax KR s &&
    noHands sk .I s && noHands sk .R s

/-- Assemble `mstuckK` from per-arm disabledness. -/
private theorem mstuckK_intro {sk : Skel} {ax : AxMode} {KI KR C : Nat}
    {σI σR : Strategy} {s : MState} (hterm : mterminal sk s = false)
    (hnone : ∀ a ∈ allMActions sk,
      applyK sk ax KI KR C σI σR a s = none) :
    mstuckK sk ax KI KR C σI σR s = true := by
  have hcan : mcanStepK sk ax KI KR C σI σR s = false := by
    rw [mcanStepK, List.any_eq_false]
    intro a ha
    rw [hnone a ha]
    simp
  rw [mstuckK, hterm, hcan]
  rfl

/-- Dispatch every enumerated K action to its disabledness fact. -/
private theorem allK_none_of_parts {sk : Skel} {ax : AxMode}
    {KI KR C : Nat} {σI σR : Strategy} {s : MState}
    (hbd : baseDeliverDisabledK sk ax KR s = true)
    (hpI : enabledPushes sk C .I s = [])
    (hpR : enabledPushes sk C .R s = []) :
    ∀ a ∈ allMActions sk, applyK sk ax KI KR C σI σR a s = none := by
  simp only [baseDeliverDisabledK, Bool.and_eq_true, List.all_eq_true,
    Option.isNone_iff_eq_none, List.isEmpty_iff] at hbd
  obtain ⟨⟨hbase, hdI⟩, hpRnil⟩ := hbd
  intro a ha
  rw [allMActions] at ha
  rcases List.mem_append.mp ha with hb | hm
  · obtain ⟨a₀, ha₀, rfl⟩ := List.mem_map.mp hb
    rw [applyK_base]
    exact hbase a₀ ha₀
  · simp only [List.mem_cons, List.not_mem_nil, or_false] at hm
    rcases hm with rfl | rfl | rfl | rfl
    · rw [applyK_push]
      exact push_none_of_enabledPushes_nil hpI
    · rw [applyK_push]
      exact push_none_of_enabledPushes_nil hpR
    · exact (deliverStepK_I_free KI 1 KR s).trans hdI
    · show deliverStepK KI KR .R s = none
      unfold deliverStepK
      rw [hpRnil]

/-- The fixed-capacity certificate yields `mstuckK` for every strategy
pair at every depth `KI`. -/
theorem mstuckK_of_stuckShapeK {sk : Skel} {ax : AxMode} {KR C : Nat}
    {s : MState} (KI : Nat) (σI σR : Strategy)
    (h : stuckShapeK sk ax KR C s = true) :
    mstuckK sk ax KI KR C σI σR s = true := by
  simp only [stuckShapeK, Bool.and_eq_true, Bool.not_eq_true',
    List.isEmpty_iff] at h
  obtain ⟨⟨⟨hterm, hbd⟩, hpI⟩, hpR⟩ := h
  exact mstuckK_intro hterm (allK_none_of_parts hbd hpI hpR)

/-- The capacity-uniform certificate yields `mstuckK` at every capacity
and depth for every strategy pair. -/
theorem mstuckK_of_stuckShapeKNoHands {sk : Skel} {ax : AxMode}
    {KR : Nat} {s : MState} (KI C : Nat) (σI σR : Strategy)
    (h : stuckShapeKNoHands sk ax KR s = true) :
    mstuckK sk ax KI KR C σI σR s = true := by
  simp only [stuckShapeKNoHands, Bool.and_eq_true, Bool.not_eq_true'] at h
  obtain ⟨⟨⟨hterm, hbd⟩, hnI⟩, hnR⟩ := h
  exact mstuckK_intro hterm (allK_none_of_parts hbd
    (enabledPushes_nil_of_noHands hnI) (enabledPushes_nil_of_noHands hnR))

-- ================================================== the kernel anchors

/-- The three anchored witnesses are inside the theorem class: each is
well-formed, so the UN-muxed `.impl` session is kernel-proven
deadlock-free and every stuck K-state indicts the parked transport
alone. -/
theorem wedgeW_wellFormed :
    (wedgeW 6).wellFormed && (wedgeW 7).wellFormed
      && (wedgeW 8).wellFormed = true := by decide

/-- The anchored witnesses satisfy the margin-0 capacity discipline
(stated through the bounded check; `margin0_sound` recovers the
flagship's unbounded form per instance). -/
theorem wedgeW_margin0 :
    margin0 (wedgeW 6) && margin0 (wedgeW 7) && margin0 (wedgeW 8)
      = true := by decide

set_option maxRecDepth 16000 in
set_option maxHeartbeats 1000000 in
/-- KR = 1: the pipe-full parks at C ∈ {1, 2, 3}. -/
theorem wedgeW6_K_stuck_C123 :
    (stuckShapeK (wedgeW 6) .impl 1 1
        (fdrainK (wedgeW 6) .impl 1 1 400 (init (wedgeW 6))) &&
      stuckShapeK (wedgeW 6) .impl 1 2
        (fdrainK (wedgeW 6) .impl 1 2 400 (init (wedgeW 6))) &&
      stuckShapeK (wedgeW 6) .impl 1 3
        (fdrainK (wedgeW 6) .impl 1 3 400 (init (wedgeW 6)))) = true := by
  decide

set_option maxRecDepth 16000 in
set_option maxHeartbeats 1000000 in
/-- KR = 1: the capacity-uniform burial at b = 4 (covers all C ≥ 4). -/
theorem wedgeW6_K_stuck_ge4 :
    stuckShapeKNoHands (wedgeW 6) .impl 1
      (fdrainK (wedgeW 6) .impl 1 4 400 (init (wedgeW 6))) = true := by
  decide

set_option maxRecDepth 16000 in
set_option maxHeartbeats 1000000 in
/-- KR = 2: the pipe-full parks at C ∈ {1, 2, 3}. -/
theorem wedgeW7_K_stuck_C123 :
    (stuckShapeK (wedgeW 7) .impl 2 1
        (fdrainK (wedgeW 7) .impl 2 1 400 (init (wedgeW 7))) &&
      stuckShapeK (wedgeW 7) .impl 2 2
        (fdrainK (wedgeW 7) .impl 2 2 400 (init (wedgeW 7))) &&
      stuckShapeK (wedgeW 7) .impl 2 3
        (fdrainK (wedgeW 7) .impl 2 3 400 (init (wedgeW 7)))) = true := by
  decide

set_option maxRecDepth 16000 in
set_option maxHeartbeats 1000000 in
/-- KR = 2: the capacity-uniform burial at b = 4. -/
theorem wedgeW7_K_stuck_ge4 :
    stuckShapeKNoHands (wedgeW 7) .impl 2
      (fdrainK (wedgeW 7) .impl 2 4 400 (init (wedgeW 7))) = true := by
  decide

set_option maxRecDepth 16000 in
set_option maxHeartbeats 1000000 in
/-- KR = 3: the pipe-full parks at C ∈ {1, 2, 3}. -/
theorem wedgeW8_K_stuck_C123 :
    (stuckShapeK (wedgeW 8) .impl 3 1
        (fdrainK (wedgeW 8) .impl 3 1 400 (init (wedgeW 8))) &&
      stuckShapeK (wedgeW 8) .impl 3 2
        (fdrainK (wedgeW 8) .impl 3 2 400 (init (wedgeW 8))) &&
      stuckShapeK (wedgeW 8) .impl 3 3
        (fdrainK (wedgeW 8) .impl 3 3 400 (init (wedgeW 8)))) = true := by
  decide

set_option maxRecDepth 16000 in
set_option maxHeartbeats 1000000 in
/-- KR = 3: the capacity-uniform burial at b = 4. -/
theorem wedgeW8_K_stuck_ge4 :
    stuckShapeKNoHands (wedgeW 8) .impl 3
      (fdrainK (wedgeW 8) .impl 3 4 400 (init (wedgeW 8))) = true := by
  decide

-- ===================================================== the impossibility

/-- T8's impossibility half: at every anchored responder depth
`KR ∈ {1, 2, 3}`, EVERY initiator depth `KI ≥ 1`, and EVERY pipe
capacity `C ≥ 1`, the width-`KR + 5` wedge defeats every
work-conserving pair — K-deep parking moves the wall, never removes it
(design/eager-absorption.md §1: "K-parking alone converts the
deterministic w = 4 wedge into a w > K wedge … not a liveness proof").

The `∀ KI` quantifier is genuine (not an anchor grid): the burial is
directional and the forced run's reverse pipe drains, so the
initiator's own parking depth never enters the stuck certificate
(module doc). `KR ≥ 4` remains open at theorem tier — each depth needs
its own kernel replay — with the widened-family argument at [derived]
tier; the un-muxed witnesses are inside the flagship's proven class
(`wedgeW_wellFormed`, `wedgeW_margin0`, `Sched.deadlock_free`). The
`KWorkConserving` class is kernel-inhabited (`bottomMostReady_wcK`,
Mux/Proofs/Inhabitation.lean). -/
theorem wc_impossibility_K (KI KR : Nat) (hKI : 1 ≤ KI)
    (hKR : KR = 1 ∨ KR = 2 ∨ KR = 3) (C : Nat) (hC : 1 ≤ C)
    (σI σR : Strategy)
    (hWI : KWorkConserving .I σI) (hWR : KWorkConserving .R σR) :
    ¬ MuxDeadlockFreeK (wedgeW (KR + 5)) .impl KI KR C σI σR := by
  intro hdf
  have hsplit : C = 1 ∨ C = 2 ∨ C = 3 ∨ 4 ≤ C := by omega
  rcases hKR with rfl | rfl | rfl
  · have hanch := wedgeW6_K_stuck_C123
    simp only [Bool.and_eq_true] at hanch
    rcases hsplit with rfl | rfl | rfl | h4
    · have hr := fdrainK_replay (Nat.le_refl 1) hKI hWI hWR 400
        (KMReachable.init (sk := wedgeW 6) (ax := .impl) (KI := KI)
          (KR := 1) (C := 1) (σI := σI) (σR := σR))
      have hstuck := mstuckK_of_stuckShapeK KI σI σR
        hanch.1.1
      rw [hdf _ hr] at hstuck
      exact Bool.false_ne_true hstuck
    · have hr := fdrainK_replay (Nat.le_refl 2) hKI hWI hWR 400
        (KMReachable.init (sk := wedgeW 6) (ax := .impl) (KI := KI)
          (KR := 1) (C := 2) (σI := σI) (σR := σR))
      have hstuck := mstuckK_of_stuckShapeK KI σI σR
        hanch.1.2
      rw [hdf _ hr] at hstuck
      exact Bool.false_ne_true hstuck
    · have hr := fdrainK_replay (Nat.le_refl 3) hKI hWI hWR 400
        (KMReachable.init (sk := wedgeW 6) (ax := .impl) (KI := KI)
          (KR := 1) (C := 3) (σI := σI) (σR := σR))
      have hstuck := mstuckK_of_stuckShapeK KI σI σR
        hanch.2
      rw [hdf _ hr] at hstuck
      exact Bool.false_ne_true hstuck
    · have hr := fdrainK_replay h4 hKI hWI hWR 400
        (KMReachable.init (sk := wedgeW 6) (ax := .impl) (KI := KI)
          (KR := 1) (C := C) (σI := σI) (σR := σR))
      have hstuck := mstuckK_of_stuckShapeKNoHands KI C σI σR
        wedgeW6_K_stuck_ge4
      rw [hdf _ hr] at hstuck
      exact Bool.false_ne_true hstuck
  · have hanch := wedgeW7_K_stuck_C123
    simp only [Bool.and_eq_true] at hanch
    rcases hsplit with rfl | rfl | rfl | h4
    · have hr := fdrainK_replay (Nat.le_refl 1) hKI hWI hWR 400
        (KMReachable.init (sk := wedgeW 7) (ax := .impl) (KI := KI)
          (KR := 2) (C := 1) (σI := σI) (σR := σR))
      have hstuck := mstuckK_of_stuckShapeK KI σI σR
        hanch.1.1
      rw [hdf _ hr] at hstuck
      exact Bool.false_ne_true hstuck
    · have hr := fdrainK_replay (Nat.le_refl 2) hKI hWI hWR 400
        (KMReachable.init (sk := wedgeW 7) (ax := .impl) (KI := KI)
          (KR := 2) (C := 2) (σI := σI) (σR := σR))
      have hstuck := mstuckK_of_stuckShapeK KI σI σR
        hanch.1.2
      rw [hdf _ hr] at hstuck
      exact Bool.false_ne_true hstuck
    · have hr := fdrainK_replay (Nat.le_refl 3) hKI hWI hWR 400
        (KMReachable.init (sk := wedgeW 7) (ax := .impl) (KI := KI)
          (KR := 2) (C := 3) (σI := σI) (σR := σR))
      have hstuck := mstuckK_of_stuckShapeK KI σI σR
        hanch.2
      rw [hdf _ hr] at hstuck
      exact Bool.false_ne_true hstuck
    · have hr := fdrainK_replay h4 hKI hWI hWR 400
        (KMReachable.init (sk := wedgeW 7) (ax := .impl) (KI := KI)
          (KR := 2) (C := C) (σI := σI) (σR := σR))
      have hstuck := mstuckK_of_stuckShapeKNoHands KI C σI σR
        wedgeW7_K_stuck_ge4
      rw [hdf _ hr] at hstuck
      exact Bool.false_ne_true hstuck
  · have hanch := wedgeW8_K_stuck_C123
    simp only [Bool.and_eq_true] at hanch
    rcases hsplit with rfl | rfl | rfl | h4
    · have hr := fdrainK_replay (Nat.le_refl 1) hKI hWI hWR 400
        (KMReachable.init (sk := wedgeW 8) (ax := .impl) (KI := KI)
          (KR := 3) (C := 1) (σI := σI) (σR := σR))
      have hstuck := mstuckK_of_stuckShapeK KI σI σR
        hanch.1.1
      rw [hdf _ hr] at hstuck
      exact Bool.false_ne_true hstuck
    · have hr := fdrainK_replay (Nat.le_refl 2) hKI hWI hWR 400
        (KMReachable.init (sk := wedgeW 8) (ax := .impl) (KI := KI)
          (KR := 3) (C := 2) (σI := σI) (σR := σR))
      have hstuck := mstuckK_of_stuckShapeK KI σI σR
        hanch.1.2
      rw [hdf _ hr] at hstuck
      exact Bool.false_ne_true hstuck
    · have hr := fdrainK_replay (Nat.le_refl 3) hKI hWI hWR 400
        (KMReachable.init (sk := wedgeW 8) (ax := .impl) (KI := KI)
          (KR := 3) (C := 3) (σI := σI) (σR := σR))
      have hstuck := mstuckK_of_stuckShapeK KI σI σR
        hanch.2
      rw [hdf _ hr] at hstuck
      exact Bool.false_ne_true hstuck
    · have hr := fdrainK_replay h4 hKI hWI hWR 400
        (KMReachable.init (sk := wedgeW 8) (ax := .impl) (KI := KI)
          (KR := 3) (C := C) (σI := σI) (σR := σR))
      have hstuck := mstuckK_of_stuckShapeKNoHands KI C σI σR
        wedgeW8_K_stuck_ge4
      rw [hdf _ hr] at hstuck
      exact Bool.false_ne_true hstuck

end StreamingMirror.Mux
