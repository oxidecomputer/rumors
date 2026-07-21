/-
The elastic-demux variant and its deadlock-freedom by simulation
(MUX-PROGRESS.md, the T8/eager-absorption log entries;
design/eager-absorption.md is the design this formalizes).

# The semantics

`applyE` is the harness of record (Mux/Basic.lean) with ONE arm
changed: `deliver` moves the pipe head into its per-stream cell WITHOUT
the slot-empty guard. The cell stops being a capacity-1 demux slot and
becomes the parked-reply queue of the eager-absorption design: incoming
frames are converted to logical replies at arrival and parked, in
order, until the consumer's cursor reaches them (design/
eager-absorption.md §2–§3 — the receiver half, at unbounded parking
depth). Everything else — the committed hand, the bounded pipe, the
strategy-gated push, the F8-strengthened closes — is the record
harness verbatim; the base arms and the push arm are shared
definitionally, so every lemma about them transports.

# Memory accounting (the §7.1 bound, restated at the model boundary)

Parking is denominated in logical replies, so the parked residue per
stream is fan-bounded PER REPLY: a parked reply is `O(fan)` node
handles (a provision run's subtree rides as one cheap handle — payload
custody is the `Backend`'s, whose nodes are persistent-structure
pointers), worst-case `fan²` hashes for a maximally disputed reply
(design/eager-absorption.md §7.1). The model's unbounded cell counts
REPLIES, not bytes — the same reply denomination as the pipe capacity
(MUX-ADJUDICATION.md §2.5), so byte soundness sits at the same model
boundary, and what the model leaves unbounded is exactly the number of
parked replies, which is what `wc_impossibility_K`
(Mux/Proofs/WcImpossibilityK.lean) shows no FIXED bound survives under
work conservation. Per-direction window advertisement (the
single-socket design's K_I ≠ K_R) is moot here: this variant is
per-direction unbounded, so both advertised depths are ∞; the bounded
per-direction form lives with the K-variant.

# The theorem: liveness by simulation, no new liveness induction

`elastic_deadlock_free`: with unbounded parking, EVERY strategy that
pushes whenever it holds a pushable frame (`EWorkConserving` — the
widest honest class: only elastic-REACHABLE states constrain σ) is
deadlock-free, at every capacity C ≥ 1, on every well-formed margin-0
skeleton. The proof is a reduction to the base flagship's progress
engine, not a new induction:

- at a hypothetical stuck state the pipes are empty (the unguarded
  deliver is always enabled on a nonempty pipe) and no hand is
  committed (else the class hypothesis forces a push, whose success is
  `firePush_isSome_of_mem`);
- with empty pipes the elastic conservation law collapses to the
  base's conservation equation (`EMuxInv.invPW`) — but NOT to the full
  base invariant: parked replies over-fill wire cells past the cap-1
  discipline, which is why the progress engine was weakened to `InvPW`
  (Proofs/Lemmas.lean): the argmin argument never consumed the
  `chan ≤ cap` half;
- `Sched.progress_of_inv` then yields an enabled base action, which is
  either a wire fire — impossible, no hands — or a non-fire arm that
  is enabled in the elastic system too (`applyBase_isSome_of_empty`),
  contradicting stuckness.

# The invariant seam (deliberate, precedented)

`elastic_deadlock_free` takes the ground facts `EMuxInv` — the base
cursors decode (`InvL`) plus the pipe-mediated conservation law — as a
reachability-invariant hypothesis, exactly as the chase does with
`MuxInv` (Mux/Proofs/Chase/Ground.lean: "stating the chase over this
interface … keeps stage 2 free of the 28-arm preservation sweep").
`eMuxInv_init` is the interface's non-vacuity certificate and the base
case of the preservation induction, which is the stage-F `MuxInv`
obligation's elastic twin: the two differ only in the deliver arm
(which preserves conservation trivially — it moves one frame from the
pipe term to the cell term) and in `EMuxInv` carrying no slot bound at
all, so the preservation sweep should land once, against the record
harness, and be adapted here — not duplicated ahead of it.

The kernel-decided completion pin at the bottom (`wedge` completes
under the shipped work-conserving policy at C = 1 with elastic
parking) is the executable half: the exact skeleton and strategy pair
that `wc_impossibility` kills under one-slot demux runs to `mterminal`
here — bounded demux state, not scheduling, is what the impossibility
indicts (the Mux/Controls.lean `wedge_unboundedSlot_completes` control,
now attached to a first-class semantics).
-/
import StreamingMirror.Mux.Proofs.WcImpossibility
import StreamingMirror.Mux.Proofs.Chase.Ground
import StreamingMirror.Proofs.EndgameE

namespace StreamingMirror.Mux

open Model

-- ========================================================= the semantics

/-- The elastic demux move: pipe head into the per-stream parked-reply
queue, unconditionally — the receiver absorbs at line rate
(design/eager-absorption.md §3.1; the `deliver` arm of the record
harness minus the slot-empty guard). -/
def deliverStepE (p : Party) (s : MState) : Option MState :=
  match s.pipe p with
  | c :: rest =>
      some { base := { s.base with chan := bump s.base.chan c 1 }
             pipe := fun q => if q == p then rest else s.pipe q
             hist := recordObs s.hist p.other
               (.delivered (wireHeight c)) }
  | [] => none

/-- The elastic muxed transition: the record harness with the demux
slot unbounded (module doc). Base and push arms are shared with
`Mux.apply` definitionally. -/
def applyE (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy)
    (a : MAction) (s : MState) : Option MState :=
  match a with
  | .deliver p => deliverStepE p s
  | a => apply sk ax C σI σR a s

/-- Some process, mux, or elastic demux can act. -/
def mcanStepE (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy)
    (s : MState) : Bool :=
  (allMActions sk).any fun a => (applyE sk ax C σI σR a s).isSome

/-- The elastic deadlock predicate (the record `mstuck` over
`applyE`). -/
def mstuckE (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy)
    (s : MState) : Bool :=
  !mterminal sk s && !mcanStepE sk ax C σI σR s

/-- Reachability of the elastic composition. -/
inductive EMReachable (sk : Skel) (ax : AxMode) (C : Nat)
    (σI σR : Strategy) : MState → Prop
  | init : EMReachable sk ax C σI σR (init sk)
  | step {s s' : MState} (a : MAction) :
      EMReachable sk ax C σI σR s → applyE sk ax C σI σR a s = some s' →
      EMReachable sk ax C σI σR s'

/-- Deadlock freedom of the elastic composition under a fixed strategy
pair: no reachable state is stuck. -/
def MuxDeadlockFreeE (sk : Skel) (ax : AxMode) (C : Nat)
    (σI σR : Strategy) : Prop :=
  ∀ s, EMReachable sk ax C σI σR s → mstuckE sk ax C σI σR s = false

/-- Elastically reachable under SOME mode, capacity, and pair — the
state universe the elastic strategy class quantifies over. -/
def EMReachableAny (sk : Skel) (s : MState) : Prop :=
  ∃ (ax : AxMode) (C : Nat) (σI σR : Strategy), EMReachable sk ax C σI σR s

/-- σ pushes whenever it holds a pushable frame, at every elastically
reachable state: the widest honest hypothesis class for
`elastic_deadlock_free`.

This is `WorkConserving` transported to the elastic state universe —
the record class quantifies over record-reachable states, which the
elastic composition leaves (parked replies over-fill the demux cells),
so its guarantee says nothing where this theorem needs it. Only the
push obligation matters; which member σ names stays free. -/
def EWorkConserving (p : Party) (σ : Strategy) : Prop :=
  ∀ (sk : Skel) (C : Nat) (s : MState), EMReachableAny sk s →
    enabledPushes sk C p s ≠ [] →
    ∃ h, σ sk (s.hist p) = some h ∧ h ∈ enabledPushes sk C p s

-- ==================================================== executable spine

/-- Run a list of mux actions under the elastic semantics, failing on
the first disabled action. -/
def mrunE (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy)
    (s : MState) : List MAction → Option MState
  | [] => some s
  | a :: rest =>
      match applyE sk ax C σI σR a s with
      | some s' => mrunE sk ax C σI σR s' rest
      | none => none

/-- Greedy elastic drain: first enabled action until quiescent. -/
def mdrainE (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy) :
    Nat → MState → MState
  | 0, s => s
  | fuel + 1, s =>
      match (allMActions sk).firstM (fun a => applyE sk ax C σI σR a s) with
      | some s' => mdrainE sk ax C σI σR fuel s'
      | none => s

-- ================================================== the elastic ground facts

/-- The elastic ground facts: base cursors decode, internal channels
obey the unmuxed conservation law, and wire flow is conserved through
the pipe. The elastic twin of `MuxInv` (Chase/Ground.lean), minus every
occupancy bound — parked replies are unbounded by design, so there is
no slot field to state. Preservation along `EMReachable` is the
stage-F obligation's elastic twin (module doc); `eMuxInv_init` is the
base case and non-vacuity certificate. -/
structure EMuxInv (sk : Skel) (s : MState) : Prop where
  invl : InvL sk .impl s.base
  flow_int : ∀ c ∈ allChans sk, isWire c = false →
    s.base.chan c + recvdOf sk s.base c = sentOf sk s.base c
  flow_wire : ∀ (p : Party) (hh : Nat),
    s.base.chan (Chan.wire p hh) + pipeCount s (Chan.wire p hh)
      + recvdOf sk s.base (Chan.wire p hh)
      = sentOf sk s.base (Chan.wire p hh)

/-- With both pipes drained, the elastic conservation law collapses to
the base's weak invariant: conservation without the capacity half —
exactly what the weakened progress engine consumes. The full `InvP` is
unavailable here BY DESIGN: parked replies over-fill wire cells. -/
theorem EMuxInv.invPW {sk : Skel} {s : MState} (hm : EMuxInv sk s)
    (hI : s.pipe .I = []) (hR : s.pipe .R = []) :
    InvPW sk .impl s.base := by
  refine ⟨hm.invl.wk, hm.invl.asm, hm.invl.top, ?_⟩
  intro c hc
  cases hw : isWire c with
  | false => exact hm.flow_int c hc hw
  | true =>
      obtain ⟨p, hh, rfl⟩ := isWire_eq hw
      have hflow := hm.flow_wire p hh
      have hpipe : pipeCount s (Chan.wire p hh) = 0 := by
        have hempty : s.pipe (wireParty (Chan.wire p hh)) = [] := by
          show s.pipe p = []
          cases p
          · exact hI
          · exact hR
        rw [pipeCount, hempty]
        rfl
      omega

/-- The elastic ground facts hold initially. -/
theorem eMuxInv_init (sk : Skel) : EMuxInv sk (init sk) := by
  refine ⟨((inv_iff sk .impl (Model.init sk)).mp (inv_init sk .impl)).local,
    ?_, ?_⟩
  · intro c _ _
    rw [show (init sk).base = Model.init sk from rfl,
      sentOf_init, recvdOf_init]
    rfl
  · intro p hh
    rw [show (init sk).base = Model.init sk from rfl,
      sentOf_init, recvdOf_init, chan_init]
    show 0 + pipeCount (init sk) (Chan.wire p hh) + 0 = 0
    rw [pipeCount]
    rfl

-- =============================================== push-guard completeness

/-- Membership in the enabled-push set makes the push succeed: the
completeness half of `firePush_isSome_sound` (the intended
`enabledPushes_spec`, Mux/Basic.lean's doors-open note). -/
theorem firePush_isSome_of_mem {sk : Skel} {C : Nat} {p : Party}
    {h : Nat} {s : MState} (hmem : h ∈ enabledPushes sk C p s) :
    (firePush sk C p h s).isSome = true := by
  have hroom : (s.pipe p).length < C :=
    enabledPushes_room (List.ne_nil_of_mem hmem)
  unfold enabledPushes at hmem
  rw [if_pos hroom] at hmem
  obtain ⟨hin, hhold⟩ := List.mem_filter.mp hmem
  unfold firePush
  rw [if_pos hroom]
  by_cases hrh : (h == sk.rootH) = true
  · rw [if_pos hrh]
    unfold holdsWire at hhold
    rw [if_pos hrh] at hhold
    cases p with
    | I =>
        have hch : s.base.iopenCh = some .wire := by simpa using hhold
        rw [hch]
        rfl
    | R =>
        have hch : s.base.ropenCh = some .wire := by simpa using hhold
        rw [hch]
        rfl
  · rw [if_neg hrh]
    unfold holdsWire at hhold
    rw [if_neg hrh] at hhold
    simp only [Bool.and_eq_true] at hhold
    obtain ⟨⟨hcont, hph⟩, hcm⟩ := hhold
    cases hcmm : (s.base.walk (p, h)).committed with
    | none => rw [hcmm] at hcm; cases hcm
    | some o =>
        cases o with
        | wire i =>
            simp only [hcmm]
            rw [if_pos (by rw [Bool.and_eq_true]; exact ⟨hcont, hph⟩)]
            rfl
        | res i => rw [hcmm] at hcm; cases hcm
        | query i => rw [hcmm] at hcm; cases hcm
        | parent => rw [hcmm] at hcm; cases hcm

-- ================================================== the stuck reduction

/-- At an elastic stuck state every enumerated action is disabled. -/
private theorem mstuckE_disabled {sk : Skel} {ax : AxMode} {C : Nat}
    {σI σR : Strategy} {s : MState}
    (hstuck : mstuckE sk ax C σI σR s = true) :
    ∀ ma ∈ allMActions sk, applyE sk ax C σI σR ma s = none := by
  rw [mstuckE, Bool.and_eq_true, Bool.not_eq_true',
    Bool.not_eq_true'] at hstuck
  have := hstuck.2
  rw [mcanStepE, List.any_eq_false] at this
  intro ma hma
  have h := this ma hma
  cases happ : applyE sk ax C σI σR ma s with
  | none => rfl
  | some s' => exact absurd (by rw [happ]; rfl) h

/-- `deliver p` is in the enumeration. -/
private theorem deliver_mem_allMActions (sk : Skel) (p : Party) :
    MAction.deliver p ∈ allMActions sk := by
  rw [allMActions]
  refine List.mem_append.mpr (.inr ?_)
  cases p <;> simp

/-- `push p` is in the enumeration. -/
private theorem push_mem_allMActions (sk : Skel) (p : Party) :
    MAction.push p ∈ allMActions sk := by
  rw [allMActions]
  refine List.mem_append.mpr (.inr ?_)
  cases p <;> simp

/-- No elastic stuck state is reachable: with unbounded parking, the
pipes drain, the class hypothesis empties the hands, and the base
progress engine (over the weak invariant the parked state still
satisfies) produces an enabled action the elastic system must share —
the simulation core (module doc). -/
theorem elastic_no_stuck (sk : Skel) (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) {C : Nat} (hC : 1 ≤ C)
    {σI σR : Strategy}
    (hWI : EWorkConserving .I σI) (hWR : EWorkConserving .R σR)
    {s : MState} (hr : EMReachable sk .impl C σI σR s)
    (hm : EMuxInv sk s) :
    mstuckE sk .impl C σI σR s = false := by
  cases hst : mstuckE sk .impl C σI σR s with
  | false => rfl
  | true =>
      exfalso
      have hdis := mstuckE_disabled hst
      -- the pipes are empty: the elastic deliver has no guard
      have hpipe : ∀ p, s.pipe p = [] := by
        intro p
        have hd := hdis (.deliver p) (deliver_mem_allMActions sk p)
        cases hp : s.pipe p with
        | nil => rfl
        | cons c rest =>
            rw [show applyE sk .impl C σI σR (.deliver p) s
                = deliverStepE p s from rfl] at hd
            unfold deliverStepE at hd
            rw [hp] at hd
            cases hd
      -- the hands are empty: a nonempty enabled-push set forces a push
      have hpush : ∀ p, enabledPushes sk C p s = [] := by
        intro p
        by_contra hne
        have hany : EMReachableAny sk s := ⟨.impl, C, σI, σR, hr⟩
        have hd := hdis (.push p) (push_mem_allMActions sk p)
        rw [show applyE sk .impl C σI σR (.push p) s
            = apply sk .impl C σI σR (.push p) s from rfl,
          apply_push] at hd
        cases p with
        | I =>
            obtain ⟨hh, hσ, hmem⟩ := hWI sk C s hany hne
            simp only [sideOf, hσ] at hd
            have hsome := firePush_isSome_of_mem hmem
            rw [hd] at hsome
            cases hsome
        | R =>
            obtain ⟨hh, hσ, hmem⟩ := hWR sk C s hany hne
            simp only [sideOf, hσ] at hd
            have hsome := firePush_isSome_of_mem hmem
            rw [hd] at hsome
            cases hsome
      -- the base state is non-terminal and satisfies the weak invariant
      have hnt : mterminal sk s = false := by
        rw [mstuckE, Bool.and_eq_true, Bool.not_eq_true'] at hst
        exact hst.1
      have hbnt : Model.terminal sk s.base = false :=
        terminal_of_mterminal_false (hpipe .I) (hpipe .R) hnt
      have hipw : InvPW sk .impl s.base :=
        hm.invPW (hpipe .I) (hpipe .R)
      -- the progress engine produces an enabled base action
      have hcan := Sched.progress_of_inv sk hwf hm0 hipw hbnt
      rw [Model.canStep, List.any_eq_true] at hcan
      obtain ⟨a, hamem, happ⟩ := hcan
      cases hIF : isWireFire s.base a with
      | false =>
          -- a non-fire arm is enabled elastically too
          have hb : (applyBase sk .impl a s).isSome = true := by
            rw [applyBase_isSome_of_empty (hpipe .I) (hpipe .R) hIF]
            exact happ
          have hd := hdis (.base a)
            (by rw [allMActions]
                exact List.mem_append.mpr (.inl (List.mem_map_of_mem hamem)))
          rw [show applyE sk .impl C σI σR (.base a) s
              = applyBase sk .impl a s from rfl] at hd
          rw [hd] at hb
          cases hb
      | true =>
          -- a wire fire needs a hand, and there are none
          have hroom : (s.pipe .I).length < C := by
            rw [hpipe .I]
            simp only [List.length_nil]
            omega
          have hroomR : (s.pipe .R).length < C := by
            rw [hpipe .R]
            simp only [List.length_nil]
            omega
          cases a with
          | iopenFire =>
              have hch : s.base.iopenCh = some .wire := by
                simpa [isWireFire] using hIF
              have hhold : holdsWire sk .I sk.rootH s.base = true := by
                simp [holdsWire, hch]
              have hmem := mem_enabledPushes_intro (C := C) hroom
                (by rw [wireHeights]; exact List.mem_cons_self ..) hhold
              rw [hpush .I] at hmem
              cases hmem
          | ropenFire =>
              have hch : s.base.ropenCh = some .wire := by
                simpa [isWireFire] using hIF
              have hhold : holdsWire sk .R sk.rootH s.base = true := by
                simp [holdsWire, hch]
              have hmem := mem_enabledPushes_intro (C := C) hroomR
                (by rw [wireHeights]; exact List.mem_cons_self ..) hhold
              rw [hpush .R] at hmem
              cases hmem
          | walkFire pk =>
              cases hcmm : (s.base.walk pk).committed with
              | none => simp [isWireFire, hcmm] at hIF
              | some o =>
                  cases o with
                  | wire i =>
                      simp only [Model.apply, hcmm] at happ
                      split at happ
                      case isFalse => cases happ
                      case isTrue hg =>
                        simp only [Bool.and_eq_true, beq_iff_eq,
                          decide_eq_true_eq] at hg
                        obtain ⟨⟨hcont, hph⟩, -⟩ := hg
                        have hmemk : pk ∈ sk.walkKeys := by
                          simpa using hcont
                        have hlt := (Sched.walkKeys_parity sk hwf
                          (p := pk.1) (k := pk.2) hmemk).1
                        have hne : (pk.2 == sk.rootH) = false := by
                          simp
                          omega
                        have hhold : holdsWire sk pk.1 pk.2 s.base
                            = true := by
                          unfold holdsWire
                          rw [if_neg (by simp at hne ⊢; omega)]
                          simp [hmemk, hph, hcmm]
                        have hroomP : (s.pipe pk.1).length < C := by
                          rw [hpipe pk.1]
                          simp only [List.length_nil]
                          omega
                        have hmemW : pk.2 ∈ wireHeights sk pk.1 := by
                          rw [wireHeights]
                          refine List.mem_cons_of_mem _
                            (List.mem_filterMap.mpr ⟨pk, hmemk, ?_⟩)
                          simp
                        have hmem := mem_enabledPushes_intro (C := C)
                          hroomP hmemW hhold
                        rw [hpush pk.1] at hmem
                        cases hmem
                  | res i => simp [isWireFire, hcmm] at hIF
                  | query i => simp [isWireFire, hcmm] at hIF
                  | parent => simp [isWireFire, hcmm] at hIF
          | iopenChoose o => simp [isWireFire] at hIF
          | ropenRecv => simp [isWireFire] at hIF
          | ropenChoose o => simp [isWireFire] at hIF
          | walkRecvWire pk => simp [isWireFire] at hIF
          | walkRecvAsked pk => simp [isWireFire] at hIF
          | walkCommit pk o => simp [isWireFire] at hIF
          | walkCloseWire pk => simp [isWireFire] at hIF
          | walkCloseAsked pk => simp [isWireFire] at hIF
          | asmRecvRes pk => simp [isWireFire] at hIF
          | asmRecvLevel pk => simp [isWireFire] at hIF
          | asmSend pk => simp [isWireFire] at hIF
          | asmClose pk => simp [isWireFire] at hIF
          | absorbRecvWire => simp [isWireFire] at hIF
          | absorbRecvAsked => simp [isWireFire] at hIF
          | absorbSend => simp [isWireFire] at hIF
          | absorbCloseWire => simp [isWireFire] at hIF
          | absorbCloseAsked => simp [isWireFire] at hIF
          | finRet => simp [isWireFire] at hIF
          | finRes => simp [isWireFire] at hIF
          | finRets => simp [isWireFire] at hIF

/-- Elastic deadlock freedom, T8's simulation capstone: with unbounded
reply parking, every pair from the pushes-when-nonempty class is
deadlock-free at every capacity C ≥ 1 — liveness inherited from the
base flagship through the weak-invariant reduction, no new liveness
induction (module doc; design/eager-absorption.md §7.4: receiver
parking supplies the buffer a credit window grants explicitly, and at
unbounded depth no sender inference is needed at all).

The `hinv` hypothesis is the invariant seam (module doc): the ground
facts along elastic runs, whose preservation induction is the stage-F
obligation's elastic twin. `eMuxInv_init` certifies its base case;
nothing else about the composition is assumed. -/
theorem elastic_deadlock_free (sk : Skel) (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) {C : Nat} (hC : 1 ≤ C)
    {σI σR : Strategy}
    (hWI : EWorkConserving .I σI) (hWR : EWorkConserving .R σR)
    (hinv : ∀ s, EMReachable sk .impl C σI σR s → EMuxInv sk s) :
    MuxDeadlockFreeE sk .impl C σI σR := by
  intro s hr
  exact elastic_no_stuck sk hwf hm0 hC hWI hWR hr (hinv s hr)

-- ============================================== the executable pin

set_option maxRecDepth 16000 in
set_option maxHeartbeats 1000000 in
/-- The exact skeleton and work-conserving pair that `wc_impossibility`
kills under one-slot demux completes under elastic parking at the
minimum capacity: bounded demux state, not scheduling, is what the
impossibility indicts — the option-C escape as a first-class
semantics (the Mux/Controls.lean unbounded-slot control, transported;
kernel-decided). -/
theorem wedge_elastic_completes :
    mterminal wedge
      (mdrainE wedge .impl 1 bottomMostReady bottomMostReady 800
        (init wedge)) = true := by
  decide

end StreamingMirror.Mux
