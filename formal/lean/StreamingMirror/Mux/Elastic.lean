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
parked replies. That no FIXED parking bound survives work conservation
is `wc_impossibility_K` (Mux/Proofs/WcImpossibilityK.lean) at its
kernel-anchored responder depths KR ∈ {1, 2, 3}; for KR ≥ 4 the claim
is [derived] only (the widened-family argument — each further depth
needs its own kernel replay, per that theorem's own status note).
Per-direction window advertisement (the
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

# The invariant is proven, not assumed (the seam, closed by T10)

`elastic_deadlock_free` originally carried the ground facts `EMuxInv` —
the base cursors decode (`InvL`) plus the pipe-mediated conservation
law — as an explicit reachability-invariant hypothesis, matching the
chase's `MuxInv` interface posture. The preservation sweep now lands
here (`eMuxInv_reachable`), adapted from the stage-F sweep
(`sinv_reachable`, Mux/Proofs/SigmaStarInv.lean): the 23 base arms
assemble the same Steps-file deltas minus every occupancy-bound and
history field, the push arm reuses the opener/walk fire facts, and the
deliver arm — the one place the two systems differ — preserves
conservation trivially (one frame moves from the pipe term to the cell
term, no slot guard to respect). `eMuxInv_init` remains the base case.

REPAIR recorded (T10 audit, mux-notes-phase2/t10-audit.md §4): the
first-landed `EMuxInv.flow_wire` was unguarded (`∀ p hh`) and therefore
unsatisfiable past walk (R,0)'s first wire receive — `recvdOf` at the
phantom `wire I 0` Nat-subtraction-aliases that walk's consumer count
while its producer count stays zero, the exact `delivered_eq` bug the
stage-F landing fixed in `MuxInv`. The field is now `allChans`-guarded
(its consumer `EMuxInv.invPW` only ever reads it at `allChans`
members), and the sweep needs the pipe-content fact the record system
kept in `hist_pipe`, so `EMuxInv` carries it directly as `pipe_wire`.

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
import StreamingMirror.Mux.Proofs.SigmaStarInv
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
obey the unmuxed conservation law, wire flow is conserved through the
pipe, and pipes carry only own-direction wire tags.

The elastic twin of `MuxInv` (Chase/Ground.lean), minus every occupancy
bound — parked replies are unbounded by design, so there is no slot
field to state — and minus the history ledger (nothing here reads
`hist`). `flow_wire` is `allChans`-guarded, NOT `∀ p hh`: at the
phantom `wire I 0` the consumer count aliases walk (R,0)'s by Nat
subtraction while the producer count stays zero, so the unguarded form
is unsatisfiable at reachable states (module doc REPAIR note; the
track-F `delivered_eq` lesson). `pipe_wire` is `hist_pipe`'s residue
once the ledger is gone: the deliver arm needs to know the frame it
lands is a wire tag of the delivering direction. Preserved along
`EMReachable` by `eMuxInv_reachable`; `eMuxInv_init` is the base
case. -/
structure EMuxInv (sk : Skel) (s : MState) : Prop where
  invl : InvL sk .impl s.base
  flow_int : ∀ c ∈ allChans sk, isWire c = false →
    s.base.chan c + recvdOf sk s.base c = sentOf sk s.base c
  flow_wire : ∀ (p : Party) (hh : Nat), Chan.wire p hh ∈ allChans sk →
    s.base.chan (Chan.wire p hh) + pipeCount s (Chan.wire p hh)
      + recvdOf sk s.base (Chan.wire p hh)
      = sentOf sk s.base (Chan.wire p hh)
  pipe_wire : ∀ (p : Party), ∀ c ∈ s.pipe p, ∃ h, c = Chan.wire p h

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
      have hflow := hm.flow_wire p hh hc
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
    ?_, ?_, ?_⟩
  · intro c _ _
    rw [show (init sk).base = Model.init sk from rfl,
      sentOf_init, recvdOf_init]
    rfl
  · intro p hh _
    rw [show (init sk).base = Model.init sk from rfl,
      sentOf_init, recvdOf_init, chan_init]
    show 0 + pipeCount (init sk) (Chan.wire p hh) + 0 = 0
    rw [pipeCount]
    rfl
  · intro p c hc
    exact (List.not_mem_nil hc).elim

-- ============================================ the preservation assemblers

/-!
The stage-F sweep (`sinv_reachable`, SigmaStarInv.lean), re-assembled
for the elastic system: the same Steps-file per-arm deltas
(`QuietStep`/`RecvStep`/`SendStep`), met by `EMuxInv`'s three flow-side
fields instead of `MuxInv`'s eight — no slot bound to maintain (parked
replies are unbounded by design) and no history ledger (nothing here
reads `hist`, so every observation append is invisible). The deliver
arm is the one place the systems differ, and its delta is trivial:
one frame moves from the pipe term to the cell term of `flow_wire`.
-/

variable {sk : Skel}

/-- A hand-and-channel-neutral base arm preserves the elastic ground
facts: every count and occupancy is framed, and the pipes are
untouched. -/
theorem EMuxInv.quiet {s : MState} {b : State}
    {hist' : Party → List MObs}
    (hm : EMuxInv sk s) (hL' : InvL sk .impl b)
    (hq : QuietStep sk s.base b) :
    EMuxInv sk { s with base := b, hist := hist' } := by
  refine ⟨hL', ?_, ?_, hm.pipe_wire⟩
  · intro c hc hw
    show b.chan c + recvdOf sk b c = sentOf sk b c
    rw [hq.chan, hq.sent c hc, hq.recvd c hc]
    exact hm.flow_int c hc hw
  · intro p hh hc
    show b.chan (Chan.wire p hh) + pipeCount s (Chan.wire p hh)
      + recvdOf sk b (Chan.wire p hh) = sentOf sk b (Chan.wire p hh)
    rw [hq.chan, hq.sent _ hc, hq.recvd _ hc]
    exact hm.flow_wire p hh hc

/-- One receive on `c₀` preserves the elastic ground facts: the
occupancy drop balances the consumer-count rise, on the internal law
and (when `c₀` is a wire cell) on the pipe-mediated one alike. -/
theorem EMuxInv.recv {s : MState} {b : State} {c₀ : Chan}
    {hist' : Party → List MObs}
    (hm : EMuxInv sk s) (hL' : InvL sk .impl b)
    (hr : RecvStep sk s.base b c₀) :
    EMuxInv sk { s with base := b, hist := hist' } := by
  have hpos := hr.hpos
  refine ⟨hL', ?_, ?_, hm.pipe_wire⟩
  · intro c hc hw
    show b.chan c + recvdOf sk b c = sentOf sk b c
    have h0 := hm.flow_int c hc hw
    rw [hr.chan, hr.sent c hc, hr.recvd c hc]
    by_cases he : c = c₀
    · subst he
      rw [bump_neg_one, if_pos rfl]
      omega
    · rw [bump_ne _ _ he, if_neg he]
      omega
  · intro p hh hc
    show b.chan (Chan.wire p hh) + pipeCount s (Chan.wire p hh)
      + recvdOf sk b (Chan.wire p hh) = sentOf sk b (Chan.wire p hh)
    have h0 := hm.flow_wire p hh hc
    rw [hr.chan, hr.sent _ hc, hr.recvd _ hc]
    by_cases he : Chan.wire p hh = c₀
    · rw [he] at h0 ⊢
      rw [bump_neg_one, if_pos rfl]
      omega
    · rw [bump_ne _ _ he, if_neg he]
      omega

/-- One send into an INTERNAL channel preserves the elastic ground
facts. Stated on the raw occupancy/count equations rather than
`SendStep` so the walk-fire arm (whose Steps fact is chan-free) can
call it without minting the floor-capacity field it does not need. -/
theorem EMuxInv.send {s : MState} {b : State} {c₀ : Chan}
    {hist' : Party → List MObs}
    (hm : EMuxInv sk s) (hL' : InvL sk .impl b)
    (hw₀ : isWire c₀ = false)
    (hchan : b.chan = bump s.base.chan c₀ 1)
    (hsent : ∀ c ∈ allChans sk,
      sentOf sk b c = sentOf sk s.base c + (if c = c₀ then 1 else 0))
    (hrecv : ∀ c ∈ allChans sk, recvdOf sk b c = recvdOf sk s.base c) :
    EMuxInv sk { s with base := b, hist := hist' } := by
  refine ⟨hL', ?_, ?_, hm.pipe_wire⟩
  · intro c hc hw
    show b.chan c + recvdOf sk b c = sentOf sk b c
    have h0 := hm.flow_int c hc hw
    rw [hchan, hsent c hc, hrecv c hc]
    by_cases he : c = c₀
    · subst he
      rw [bump_one, if_pos rfl]
      omega
    · rw [bump_ne _ _ he, if_neg he]
      omega
  · intro p hh hc
    have hne : Chan.wire p hh ≠ c₀ := by
      intro he
      rw [← he] at hw₀
      simp [isWire] at hw₀
    show b.chan (Chan.wire p hh) + pipeCount s (Chan.wire p hh)
      + recvdOf sk b (Chan.wire p hh) = sentOf sk b (Chan.wire p hh)
    rw [hchan, hsent _ hc, hrecv _ hc, bump_ne _ _ hne, if_neg hne]
    exact hm.flow_wire p hh hc

/-- The push-side assembly: the sender's cursor advance raises the
pushed stream's producer count by exactly the frame the pipe gains, so
`flow_wire` balances; nothing internal moves. -/
theorem EMuxInv.push_assemble {s : MState} {b : State} {p : Party}
    {h : Nat}
    (hm : EMuxInv sk s) (hL' : InvL sk .impl b)
    (hchan : b.chan = s.base.chan)
    (hsw : ∀ q g, Chan.wire q g ∈ allChans sk →
      sentOf sk b (Chan.wire q g) = sentOf sk s.base (Chan.wire q g)
        + (if q = p ∧ g = h then 1 else 0))
    (hsint : ∀ c ∈ allChans sk, isWire c = false →
      sentOf sk b c = sentOf sk s.base c)
    (hrecv : ∀ c ∈ allChans sk, recvdOf sk b c = recvdOf sk s.base c) :
    EMuxInv sk { base := b
                 pipe := fun q => if q == p
                   then s.pipe q ++ [Chan.wire p h] else s.pipe q
                 hist := recordObs s.hist p (.pushed h) } := by
  refine ⟨hL', ?_, ?_, ?_⟩
  · intro c hc hw
    show b.chan c + recvdOf sk b c = sentOf sk b c
    rw [hchan, hsint c hc hw, hrecv c hc]
    exact hm.flow_int c hc hw
  · intro q g hc
    have hpc : (if (q == p) = true
          then s.pipe q ++ [Chan.wire p h] else s.pipe q).count
            (Chan.wire q g)
        = (s.pipe q).count (Chan.wire q g)
          + (if q = p ∧ g = h then 1 else 0) := by
      by_cases hq : q = p
      · subst hq
        rw [if_pos (by simp), List.count_append]
        by_cases hg : g = h
        · subst hg
          rw [if_pos ⟨rfl, rfl⟩]
          simp
        · rw [if_neg (fun hcon => hg hcon.2), List.count_cons,
            List.count_nil,
            if_neg (by
              simp only [beq_iff_eq, Chan.wire.injEq]
              exact fun hcon => hg hcon.2.symm)]
      · rw [if_neg (by simp [hq]), if_neg (fun hcon => hq hcon.1)]
        omega
    show b.chan (Chan.wire q g)
        + (if (q == p) = true
            then s.pipe q ++ [Chan.wire p h] else s.pipe q).count
              (Chan.wire q g)
        + recvdOf sk b (Chan.wire q g) = sentOf sk b (Chan.wire q g)
    rw [hchan, hsw q g hc, hrecv _ hc, hpc]
    have h0 := hm.flow_wire q g hc
    have hpcs : pipeCount s (Chan.wire q g)
        = (s.pipe q).count (Chan.wire q g) := rfl
    rw [hpcs] at h0
    omega
  · intro q c hc
    have hc' : c ∈ (if (q == p) = true
        then s.pipe q ++ [Chan.wire p h] else s.pipe q) := hc
    by_cases hq : q = p
    · subst hq
      rw [if_pos (by simp)] at hc'
      rcases List.mem_append.mp hc' with hold | hnew
      · exact hm.pipe_wire q c hold
      · exact ⟨h, List.mem_singleton.mp hnew⟩
    · rw [if_neg (by simp [hq])] at hc'
      exact hm.pipe_wire q c hc'

-- ================================================ the preservation sweep

/-- Every enabled base arm preserves the elastic ground facts: the
23-arm dispatch through the Steps files, assembling `EMuxInv`'s three
flow-side fields where the stage-F sweep assembled `MuxInv`'s eight. -/
theorem eMuxInv_base (hwf : sk.wellFormed = true) {a : Action}
    {s s' : MState} (hstep : applyBase sk .impl a s = some s')
    (hm : EMuxInv sk s) : EMuxInv sk s' := by
  obtain ⟨hnf, b, hb, hs'⟩ := applyBase_inv hstep
  have hL := hm.invl
  subst hs'
  cases a with
  | iopenChoose o =>
      cases o with
      | wire =>
          obtain ⟨hL', hq, -, -, -⟩ := step_iopenChoose_wire hb hL
          exact hm.quiet hL' hq
      | query =>
          obtain ⟨hL', hq, -⟩ := step_iopenChoose_query hb hL
          exact hm.quiet hL' hq
  | iopenFire =>
      have hch : s.base.iopenCh = some .query := by
        rw [Model.apply] at hb
        cases hio : s.base.iopenCh with
        | none => rw [hio] at hb; cases hb
        | some o =>
            cases o with
            | wire =>
                exfalso
                rw [isWireFire, hio] at hnf
                simp at hnf
            | query => rfl
      obtain ⟨hL', hsend, -⟩ := step_iopenFire_query hch hb hL
      exact hm.send hL' rfl hsend.chan hsend.sent hsend.recvd
  | ropenRecv =>
      obtain ⟨hL', hr, -⟩ := step_ropenRecv hb hL
      exact hm.recv hL' hr
  | ropenChoose o =>
      cases o with
      | wire =>
          obtain ⟨hL', hq, -, -, -⟩ := step_ropenChoose_wire hb hL
          exact hm.quiet hL' hq
      | res =>
          obtain ⟨hL', hq, -⟩ := step_ropenChoose_res hb hL
          exact hm.quiet hL' hq
      | query =>
          obtain ⟨hL', hq, -⟩ := step_ropenChoose_query hb hL
          exact hm.quiet hL' hq
  | ropenFire =>
      have hch : s.base.ropenCh = some .res
          ∨ s.base.ropenCh = some .query := by
        rw [Model.apply] at hb
        cases hro : s.base.ropenCh with
        | none => rw [hro] at hb; cases hb
        | some o =>
            cases o with
            | wire =>
                exfalso
                rw [isWireFire, hro] at hnf
                simp at hnf
            | res => exact Or.inl rfl
            | query => exact Or.inr rfl
      rcases hch with hch | hch
      · obtain ⟨hL', hsend, -⟩ := step_ropenFire_res hch hb hL
        exact hm.send hL' rfl hsend.chan hsend.sent hsend.recvd
      · obtain ⟨hL', hsend, -⟩ := step_ropenFire_query hch hb hL
        exact hm.send hL' rfl hsend.chan hsend.sent hsend.recvd
  | walkRecvWire pk =>
      obtain ⟨hL', hr, -⟩ := step_walkRecvWire hwf pk hb hL
      exact hm.recv hL' hr
  | walkRecvAsked pk =>
      obtain ⟨hL', hr, -⟩ := step_walkRecvAsked hwf pk hb hL
      exact hm.recv hL' hr
  | walkCommit pk o =>
      cases o with
      | wire i =>
          obtain ⟨hL', hq, -, -, -, -⟩ := step_walkCommit_wire hwf pk i hb hL
          exact hm.quiet hL' hq
      | res i =>
          obtain ⟨hL', hq, -⟩ := step_walkCommit_res pk i hb hL
          exact hm.quiet hL' hq
      | query i =>
          obtain ⟨hL', hq, -⟩ := step_walkCommit_query pk i hb hL
          exact hm.quiet hL' hq
      | parent =>
          obtain ⟨hL', hq, -⟩ := step_walkCommit_parent pk hb hL
          exact hm.quiet hL' hq
  | walkFire pk =>
      -- decompose the fire; the wire obligation is barred by `hnf`
      simp only [Model.apply] at hb
      split at hb
      next o hcm =>
        split at hb
        case isFalse => cases hb
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq]
            at hg
          obtain ⟨⟨hmem, hph2⟩, -⟩ := hg
          have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
          have hnw : ∀ i, o ≠ Oblig.wire i := by
            intro i hcon
            subst hcon
            rw [isWireFire, hcm] at hnf
            cases hnf
          injection hb with hbeq
          obtain ⟨hL', hsent, hrecv, -, -, -⟩ :=
            step_fire (s' := setWalk s.base pk
              (normWalk sk pk.2 (fireOblig (s.base.walk pk) o)))
              hwf pk o hmem' hph2 hcm rfl hL
          have hbt : b = { setWalk s.base pk
              (normWalk sk pk.2 (fireOblig (s.base.walk pk) o)) with
              chan := bump s.base.chan (obligChan pk o) 1 } := by
            rw [← hbeq]
            rfl
          have hL'' : InvL sk .impl b := by
            rw [hbt]
            exact InvL.chan_blind hL'
          refine hm.send hL'' (isWire_obligChan_nonwire pk hnw) ?_ ?_ ?_
          · rw [hbt]
          · intro c hc
            rw [hbt, sentOf_chan_blind]
            exact hsent c hc
          · intro c hc
            rw [hbt, recvdOf_chan_blind]
            exact hrecv c hc
      next hcm => cases hb
  | walkCloseWire pk =>
      obtain ⟨hL', hq, -⟩ := step_walkCloseWire hwf pk hb hL
      exact hm.quiet hL' hq
  | walkCloseAsked pk =>
      obtain ⟨hL', hq, -⟩ := step_walkCloseAsked hwf pk hb hL
      exact hm.quiet hL' hq
  | asmRecvRes pk =>
      obtain ⟨hL', hr, -⟩ := step_asmRecvRes hwf pk hb hL
      exact hm.recv hL' hr
  | asmRecvLevel pk =>
      obtain ⟨hL', hr, -⟩ := step_asmRecvLevel hwf pk hb hL
      exact hm.recv hL' hr
  | asmSend pk =>
      obtain ⟨hL', hsend, -⟩ := step_asmSend hwf pk hb hL
      refine hm.send hL' ?_ hsend.chan hsend.sent hsend.recvd
      rw [Skel.asmOutChan]
      split
      · rfl
      · split <;> rfl
  | asmClose pk =>
      obtain ⟨hL', hq, -⟩ := step_asmClose hwf pk hb hL
      exact hm.quiet hL' hq
  | absorbRecvWire =>
      obtain ⟨hL', hr, -⟩ := step_absorbRecvWire hwf hb hL
      exact hm.recv hL' hr
  | absorbRecvAsked =>
      obtain ⟨hL', hr, -⟩ := step_absorbRecvAsked hb hL
      exact hm.recv hL' hr
  | absorbSend =>
      obtain ⟨hL', hsend, -⟩ := step_absorbSend hb hL
      exact hm.send hL' rfl hsend.chan hsend.sent hsend.recvd
  | absorbCloseWire =>
      obtain ⟨hL', hq, -⟩ := step_absorbCloseWire hb hL
      exact hm.quiet hL' hq
  | absorbCloseAsked =>
      obtain ⟨hL', hq, -⟩ := step_absorbCloseAsked hb hL
      exact hm.quiet hL' hq
  | finRet =>
      obtain ⟨hL', hr, -⟩ := step_finRet hb hL
      exact hm.recv hL' hr
  | finRes =>
      obtain ⟨hL', hr, -⟩ := step_finRes hb hL
      exact hm.recv hL' hr
  | finRets =>
      obtain ⟨hL', hr, -⟩ := step_finRets hb hL
      exact hm.recv hL' hr

/-- A successful push preserves the elastic ground facts. -/
theorem eMuxInv_firePush (hwf : sk.wellFormed = true) {C : Nat}
    {p : Party} {h : Nat} {s s' : MState}
    (hfp : firePush sk C p h s = some s') (hm : EMuxInv sk s) :
    EMuxInv sk s' := by
  simp only [firePush] at hfp
  split at hfp
  case isFalse => cases hfp
  case isTrue hroom =>
    split at hfp
    · -- the opening stream
      next hr =>
      have hr' : h = sk.rootH := by simpa using hr
      subst hr'
      cases p with
      | I =>
          cases hch : s.base.iopenCh with
          | none => rw [hch] at hfp; cases hfp
          | some o =>
              cases o with
              | query => rw [hch] at hfp; cases hfp
              | wire =>
                  rw [hch] at hfp
                  injection hfp with hs'
                  obtain ⟨hL', hsw, hsint, hrecv, -, -, -⟩ :=
                    iopen_fire_facts hch hm.invl
                  subst hs'
                  exact hm.push_assemble hL' rfl
                    (fun q g _ => hsw q g)
                    (fun c _ hw => hsint c hw)
                    (fun c _ => hrecv c)
      | R =>
          cases hch : s.base.ropenCh with
          | none => rw [hch] at hfp; cases hfp
          | some o =>
              cases o with
              | query => rw [hch] at hfp; cases hfp
              | res => rw [hch] at hfp; cases hfp
              | wire =>
                  rw [hch] at hfp
                  injection hfp with hs'
                  obtain ⟨hL', hsw, hsint, hrecv, -, -, -⟩ :=
                    ropen_fire_facts hch hm.invl
                  subst hs'
                  exact hm.push_assemble hL' rfl
                    (fun q g _ => hsw q g)
                    (fun c _ hw => hsint c hw)
                    (fun c _ => hrecv c)
    · -- a walk stream
      next hr =>
      split at hfp
      next i hcm =>
        split at hfp
        case isFalse => cases hfp
        case isTrue hg =>
          simp only [Bool.and_eq_true] at hg
          obtain ⟨hcon, hph⟩ := hg
          have hmem' : (p, h) ∈ sk.walkKeys :=
            (List.contains_iff_mem ..).mp hcon
          have hph2 : (s.base.walk (p, h)).phase = 2 := by
            simpa using hph
          injection hfp with hs'
          obtain ⟨hL', hsent, hrecv, hchan, -, -⟩ :=
            step_fire (s' := setWalk s.base (p, h)
              (normWalk sk h (fireOblig (s.base.walk (p, h))
                (.wire i))))
            hwf (p, h) (.wire i) hmem' hph2 hcm rfl hm.invl
          subst hs'
          refine hm.push_assemble hL' hchan ?_
            (fun c hc hw => ?_) (fun c hc => hrecv c hc)
          · intro q g hc
            rw [hsent _ hc]
            congr 1
            rw [show obligChan (p, h) (Oblig.wire i) = Chan.wire p h
              from rfl]
            by_cases hqg : q = p ∧ g = h
            · obtain ⟨rfl, rfl⟩ := hqg
              rw [if_pos rfl, if_pos ⟨rfl, rfl⟩]
            · have hne : Chan.wire q g ≠ Chan.wire p h := by
                intro hcon2
                apply hqg
                have h1 := congrArg wireParty hcon2
                have h2 := congrArg wireHeight hcon2
                exact ⟨h1, h2⟩
              rw [if_neg hne, if_neg hqg]
          · have hne : c ≠ obligChan (p, h) (Oblig.wire i) := by
              intro hcon2
              rw [hcon2] at hw
              simp [isWire, obligChan, wireOut] at hw
            rw [hsent _ hc, if_neg hne]
            omega
      next hcm => cases hfp

/-- The elastic delivery preserves the ground facts: the FIFO head
moves from the pipe term of `flow_wire` to the cell term, everything
else framed — the arm whose slot guard the elastic semantics dropped,
and whose invariant delta was trivial all along (module doc). -/
theorem eMuxInv_deliver {p : Party} {s s' : MState}
    (hstep : deliverStepE p s = some s') (hm : EMuxInv sk s) :
    EMuxInv sk s' := by
  unfold deliverStepE at hstep
  split at hstep
  next c rest hp =>
    injection hstep with hs'
    obtain ⟨g, rfl⟩ := hm.pipe_wire p c (by rw [hp]; exact List.mem_cons_self ..)
    subst hs'
    refine ⟨InvL.chan_blind hm.invl, ?_, ?_, ?_⟩
    · intro c' hc' hw'
      have hne : c' ≠ Chan.wire p g := by
        intro he
        rw [he] at hw'
        cases hw'
      show bump s.base.chan (Chan.wire p g) 1 c'
          + recvdOf sk _ c' = sentOf sk _ c'
      rw [bump_ne _ _ hne, recvdOf_chan_blind, sentOf_chan_blind]
      exact hm.flow_int c' hc' hw'
    · intro q hh hc'
      have h0 := hm.flow_wire q hh hc'
      have hpcount : pipeCount s (Chan.wire q hh)
          = (s.pipe q).count (Chan.wire q hh) := rfl
      rw [hpcount] at h0
      show bump s.base.chan (Chan.wire p g) 1 (Chan.wire q hh)
          + (if (q == p) = true then rest else s.pipe q).count
              (Chan.wire q hh)
          + recvdOf sk { s.base with
              chan := bump s.base.chan (Chan.wire p g) 1 } (Chan.wire q hh)
          = sentOf sk { s.base with
              chan := bump s.base.chan (Chan.wire p g) 1 } (Chan.wire q hh)
      rw [recvdOf_chan_blind, sentOf_chan_blind]
      by_cases hq : q = p
      · subst hq
        rw [if_pos (by simp)]
        rw [hp, List.count_cons] at h0
        by_cases hg : Chan.wire q hh = Chan.wire q g
        · rw [hg] at h0 ⊢
          rw [bump_one]
          rw [if_pos (by simp)] at h0
          omega
        · rw [bump_ne _ _ hg]
          rw [if_neg (by
            simp only [beq_iff_eq]
            exact fun hcon => hg hcon.symm)] at h0
          omega
      · have hne : Chan.wire q hh ≠ Chan.wire p g := by
          intro hcon
          apply hq
          exact congrArg wireParty hcon
        rw [if_neg (by simp [hq]), bump_ne _ _ hne]
        exact h0
    · intro q c' hc'
      have hc'' : c' ∈ (if (q == p) = true then rest else s.pipe q) := hc'
      by_cases hq : q = p
      · subst hq
        rw [if_pos (by simp)] at hc''
        exact hm.pipe_wire q c'
          (by rw [hp]; exact List.mem_cons_of_mem _ hc'')
      · rw [if_neg (by simp [hq])] at hc''
        exact hm.pipe_wire q c' hc''
  next => cases hstep

/-- Every elastic step preserves the ground facts, under every
strategy pair: the strategy only selects which pushes happen, never
what a step does. -/
theorem eMuxInv_step (hwf : sk.wellFormed = true) {C : Nat}
    {σI σR : Strategy} {a : MAction} {s s' : MState}
    (hstep : applyE sk .impl C σI σR a s = some s')
    (hm : EMuxInv sk s) : EMuxInv sk s' := by
  cases a with
  | base a =>
      exact eMuxInv_base hwf
        (show applyBase sk .impl a s = some s' from hstep) hm
  | push p =>
      have hstep' : (match (match p with | .I => σI | .R => σR)
            sk (s.hist p) with
          | some h => firePush sk C p h s
          | none => none) = some s' := hstep
      cases hσ : (match p with | .I => σI | .R => σR) sk (s.hist p) with
      | none => rw [hσ] at hstep'; cases hstep'
      | some h =>
          rw [hσ] at hstep'
          exact eMuxInv_firePush hwf hstep' hm
  | deliver p =>
      exact eMuxInv_deliver
        (show deliverStepE p s = some s' from hstep) hm

/-- The elastic ground facts hold at every elastically reachable
state: the preservation sweep the seam hypothesis stood in for, now
discharged (module doc). -/
theorem eMuxInv_reachable (hwf : sk.wellFormed = true) {C : Nat}
    {σI σR : Strategy} {s : MState}
    (hr : EMReachable sk .impl C σI σR s) : EMuxInv sk s := by
  induction hr with
  | init => exact eMuxInv_init sk
  | step a _ hstep ih => exact eMuxInv_step hwf hstep ih

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

Unconditional (T10, MUX-PROGRESS §3.4b's secondary deliverable): the
former explicit `hinv` seam is discharged by `eMuxInv_reachable`, the
stage-F sweep's elastic twin. Nothing about the composition is
assumed beyond the class hypotheses, and the `EWorkConserving` class
is kernel-inhabited (`bottomMostReady_wcE`,
Mux/Proofs/Inhabitation.lean). Capacity and parking are
message-denominated; the byte caveat of record is Mux/Basic.lean's
module doc (# The byte-denomination caveat). -/
theorem elastic_deadlock_free (sk : Skel) (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) {C : Nat} (hC : 1 ≤ C)
    {σI σR : Strategy}
    (hWI : EWorkConserving .I σI) (hWR : EWorkConserving .R σR) :
    MuxDeadlockFreeE sk .impl C σI σR := by
  intro s hr
  exact elastic_no_stuck sk hwf hm0 hC hWI hWR hr
    (eMuxInv_reachable hwf hr)

-- ============================================== the executable pin

set_option maxRecDepth 16000 in
set_option maxHeartbeats 1000000 in
/-- The exact skeleton and work-conserving pair that `wc_impossibility`
kills under one-slot demux completes under elastic parking at the
minimum capacity: bounded demux state, not scheduling, is what the
impossibility indicts — the option-C escape as a first-class
semantics (the Mux/Controls.lean unbounded-slot control, transported;
kernel-decided). Message-denominated (Mux/Basic.lean, # The
byte-denomination caveat). -/
theorem wedge_elastic_completes :
    mterminal wedge
      (mdrainE wedge .impl 1 bottomMostReady bottomMostReady 800
        (init wedge)) = true := by
  decide

end StreamingMirror.Mux
