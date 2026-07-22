/-
The chase's ground layer (MUX-ADJUDICATION.md §3 T2, stage-2 track B):
the observation-history counting vocabulary, the muxed ground-fact
interface `MuxInv`, and the lemmas that move between the muxed system
and the base model at a stuck state.

# The counting vocabulary

A machine's history is its own ledger: `.pushed h` receipts count the
frames it fired into its pipe, `.delivered h` receipts count the frames
its demux moved into slots. Everything the keystone and the chase say
about transport state is arithmetic over these counts plus the base
model's derived counts (`sentOf`/`recvdOf`) — no new event type is
minted; wire sends ARE pushes (the walk ledger advances at `firePush`,
so `sentOf` on a wire channel already counts pushes), and delivery is
the one genuinely new notion, carried by the `.delivered` receipts.

# `MuxInv`: the ground facts, as an interface

`MuxInv` packages exactly the reachability invariants the keystone and
chase consume: the base state's local invariant fragment (`InvL` — the
decode layer's hypothesis), the slot bound, internal-channel flow, and
the hist/pipe correspondence (pushes split into a delivered prefix and
the in-flight suffix, in order; delivery splits into consumed plus
slot). Its preservation along `MReachable` is the stage-3 obligation
(MUX-ADJUDICATION §4, stage F: the flowOk-template induction); every
theorem here takes `MuxInv` as a hypothesis so that both stage-3
consumers (T4's σ* and T5's oracle) instantiate one statement.

# Stuckness transfers

At an `mstuck` state no muxed action is enabled. The transfer lemmas
turn that into base-model facts: a base action that is neither a wire
fire nor a close is muxed-enabled iff base-enabled, so at a stuck state
every such enabled base action is a contradiction — which is how the
keystone and the chase convert "some process could act" into falsity,
leaving wire fires (withheld pushes) as the only survivors.
-/
import StreamingMirror.Mux.Instances
import StreamingMirror.Proofs.Progress

namespace StreamingMirror.Mux

open Model

-- ==================================================== history counting

/-- Heights of this machine's own pushes, in flush order. -/
def pushHeights (tr : List MObs) : List Nat :=
  tr.filterMap fun o => match o with | .pushed h => some h | _ => none

/-- Heights of the frames delivered into this machine's demux slots, in
arrival order. -/
def delHeights (tr : List MObs) : List Nat :=
  tr.filterMap fun o => match o with | .delivered h => some h | _ => none

/-- Frames this machine has pushed on stream `h`. -/
def pushedCount (tr : List MObs) (h : Nat) : Nat := (pushHeights tr).count h

/-- Frames delivered to this machine's endpoint on stream `h`. -/
def deliveredCount (tr : List MObs) (h : Nat) : Nat :=
  (delHeights tr).count h

/-- Total frames delivered to this machine's endpoint, all streams. -/
def delTotal (tr : List MObs) : Nat := (delHeights tr).length

/-- Frames of channel `c` in flight in its producer's pipe. -/
def pipeCount (s : MState) (c : Chan) : Nat := (s.pipe (wireParty c)).count c

/-- Push counts only grow along an observation history. -/
theorem pushedCount_le_of_prefix {tr tr' : List MObs} (hp : tr <+: tr')
    (h : Nat) : pushedCount tr h ≤ pushedCount tr' h :=
  (hp.sublist.filterMap _).count_le h

/-- Delivery counts only grow along an observation history. -/
theorem deliveredCount_le_of_prefix {tr tr' : List MObs} (hp : tr <+: tr')
    (h : Nat) : deliveredCount tr h ≤ deliveredCount tr' h :=
  (hp.sublist.filterMap _).count_le h

/-- Tagging heights with a wire constructor preserves counts. -/
theorem count_map_wire (p : Party) (h : Nat) (l : List Nat) :
    (l.map (Chan.wire p)).count (Chan.wire p h) = l.count h := by
  induction l with
  | nil => rfl
  | cons x l ih =>
      simp only [List.map_cons, List.count_cons, ih]
      congr 1
      by_cases hx : x = h
      · simp [hx]
      · simp [hx, Chan.wire.injEq]

-- ============================================== the real wire family

/-- Stream `(p, h)` is in the REAL wire family: the session actually
wires its channel (`allChans` membership).

This is the mandatory guard for transport-invariant fields about wire
counts — the shape is `∀ p h, RealWire sk p h → …` (AUDIT-NOTES
A11/A12; this definition is the phase-5 source fix). The unguarded
form `∀ p h, …` crosses the accessor layer's junk corner —
`recvdOf_phantom_alias` below characterizes it — and count equations
quantified that way have been unsatisfiable at reachable states in
four independent landings. A new invariant field about wire counts
either carries this guard or explains, at the field, why its
off-family instances hold. Definitionally transparent: any `allChans`
membership fact discharges it, and consumers destructure it as one. -/
def RealWire (sk : Skel) (p : Party) (h : Nat) : Prop :=
  Chan.wire p h ∈ allChans sk

/-- The phantom-channel alias, characterized once at the accessor
layer: at the off-family channel `wire I 0` — never an `allChans`
member once the skeleton has real height (Wiring.lean's
`wire_I_zero_not_mem`) — the total accessor `recvdOf` falls through
its wire arm's Nat subtraction (`h - 1` at `h = 0`) to walk `(R, 0)`'s
consumer count, while `sentOf` there stays zero.

Consequence, and why `RealWire` is mandatory: an UNGUARDED wire-family
count equation asserts at `wire I 0` that a real walk's consumption is
zero, which turns false the moment that walk consumes — the equation
is then not an invariant of the muxed system at all (the pin below
decides a reachable counterexample). The bug class this lemma exists
to end: AUDIT-NOTES A11 (tracks E and F, independently) and A12 (the
elastic twin, caught twice more). -/
theorem recvdOf_phantom_alias (sk : Skel)
    (hroot : (0 == sk.rootH) = false) (s : State) :
    recvdOf sk s (Chan.wire .I 0) = wkWireRecvd sk s (.R, 0) := by
  simp only [recvdOf, hroot, Bool.false_and, if_neg (by decide :
    ¬((Party.I == Party.R && (0 : Nat) == 0) = true))]
  rfl

set_option maxRecDepth 16000 in
/-- The unguarded `delivered_eq` shape is FALSE at a reachable state,
kernel-decided: at the smoke run's terminal state, the receiver-side
split equation read at the phantom `wire I 0` fails — nothing was ever
delivered on a channel that does not exist, while the aliased accessor
reports walk `(R, 0)`'s real consumption. This is the phase-4 review's
F1 probe, kernel-checked: a reviewer meeting an unguarded wire-family
field refutes it by inspection plus this one `decide`. -/
theorem phantom_refutes_unguarded_delivered_eq :
    (let s := mdrain Pin.smokeChain .impl 1 bottomMostReady
        bottomMostReady 300 (init Pin.smokeChain)
     decide (deliveredCount (s.hist .R) 0
       ≠ recvdOf Pin.smokeChain s.base (Chan.wire .I 0)
         + s.base.chan (Chan.wire .I 0))) = true := by
  decide

-- ================================================== the ground facts

/-- The muxed ground facts: what the keystone and the chase consume at
a reachable muxed state.

Preservation along `MReachable` is the stage-3 obligation (the
MUX-ADJUDICATION §4 stage-F `MuxInv` induction on the flowOk template);
stating the chase over this interface rather than over `MReachable`
keeps stage 2 free of the 28-arm preservation sweep, exactly as the
adjudication's T2 plan prescribes ("closure-order induction, no
reachability induction"). The fields:

- `invl`: the base state's cursors decode (the `InvL` fragment — flow
  is deliberately absent, because a muxed state with frames in flight
  does not satisfy the unmuxed conservation law).
- `slot`/`flow_int`: occupancy is capacity-bounded on `allChans` (the
  field is guarded, like every count field here — the phantom corner
  admits no unrelativized bound), and off the wire family the unmuxed
  conservation law still holds (the pipe carries wire frames only).
- `pushed_eq`: flush receipts are the wire send counts — a wire send
  IS a push.
- `hist_del`/`hist_pipe`: FIFO, in count-free form — the delivered
  heights are a prefix of the pushed heights, and the pipe is exactly
  the undelivered suffix, tagged.
- `delivered_eq`: a delivered frame is consumed or sitting in its
  slot — the receiver-side split.
- `pushed_mem`: pushed streams are real channels — what lets a consumer
  discharge the membership guards from history facts alone.

The two count equations carry the `RealWire` guard (the mandatory
shape — `recvdOf_phantom_alias` above is the trap it guards, and
`phantom_refutes_unguarded_delivered_eq` decides a reachable
counterexample to the unguarded form). Consumers recover the guard
from `pushed_mem` (transport-side channels) or from trace membership
(`evUniv_wire_mem`, Chase/Decode.lean).

Integration note (stage 3): tracks E and F each caught the unguarded
`delivered_eq` breaking at the phantom channel and repaired it
independently — F with the guards plus the `pushed_mem` field (this
structure, load-bearing under T4's stack), E with the guards plus a
`pushed_real` field, the contrapositive of `pushed_mem`. The merge
kept F's field; E's spelling survives as the derived lemmas
`pushed_real` and `delivered_real` below, which E's oracle consumers
use for phantom-corner vacuity. -/
structure MuxInv (sk : Skel) (s : MState) : Prop where
  invl : InvL sk .impl s.base
  slot : ∀ c ∈ allChans sk, s.base.chan c ≤ sk.cap c
  flow_int : ∀ c ∈ allChans sk, isWire c = false →
    s.base.chan c + recvdOf sk s.base c = sentOf sk s.base c
  pushed_eq : ∀ p h, RealWire sk p h →
    pushedCount (s.hist p) h = sentOf sk s.base (Chan.wire p h)
  hist_del : ∀ p, delHeights (s.hist p.other)
    = (pushHeights (s.hist p)).take (delTotal (s.hist p.other))
  hist_pipe : ∀ p, s.pipe p
    = ((pushHeights (s.hist p)).drop (delTotal (s.hist p.other))).map
        (Chan.wire p)
  delivered_eq : ∀ p h, RealWire sk p h →
    deliveredCount (s.hist p.other) h
      = recvdOf sk s.base (Chan.wire p h) + s.base.chan (Chan.wire p h)
  pushed_mem : ∀ p h, pushedCount (s.hist p) h ≠ 0 →
    Chan.wire p h ∈ allChans sk

namespace MuxInv

variable {sk : Skel} {s : MState}

/-- The delivered heights are a prefix of the pushed heights: FIFO in
its most consumable form. -/
theorem delivered_prefix (hm : MuxInv sk s) (p : Party) :
    delHeights (s.hist p.other) <+: pushHeights (s.hist p) := by
  rw [hm.hist_del p]
  exact List.take_prefix _ _

/-- Per stream, pushes split into the delivered frames plus the frames
still in flight. -/
theorem pushed_split (hm : MuxInv sk s) (p : Party) (h : Nat) :
    pushedCount (s.hist p) h
      = deliveredCount (s.hist p.other) h
        + pipeCount s (Chan.wire p h) := by
  have hparty : wireParty (Chan.wire p h) = p := rfl
  rw [pipeCount, hparty, hm.hist_pipe p, count_map_wire,
    deliveredCount, hm.hist_del p, pushedCount]
  conv => lhs; rw [← List.take_append_drop (delTotal (s.hist p.other))
    (pushHeights (s.hist p))]
  rw [List.count_append]

/-- Delivery never outruns pushing, per stream. -/
theorem delivered_le_pushed (hm : MuxInv sk s) (p : Party) (h : Nat) :
    deliveredCount (s.hist p.other) h ≤ pushedCount (s.hist p) h := by
  have := hm.pushed_split p h
  omega

/-- Wire flow conservation through the pipe: slot occupancy plus
in-flight frames plus consumption is production. -/
theorem flow_wire (hm : MuxInv sk s) (p : Party) (h : Nat)
    (hmem : Chan.wire p h ∈ allChans sk) :
    s.base.chan (Chan.wire p h) + pipeCount s (Chan.wire p h)
      + recvdOf sk s.base (Chan.wire p h)
      = sentOf sk s.base (Chan.wire p h) := by
  have h1 := hm.pushed_eq p h hmem
  have h2 := hm.pushed_split p h
  have h3 := hm.delivered_eq p h hmem
  omega

/-- Phantom wire channels are never pushed: `pushed_mem` in the
contrapositive spelling track E minted as a field — kept as a derived
lemma after the merge picked F's `pushed_mem`. -/
theorem pushed_real (hm : MuxInv sk s) (p : Party) (h : Nat)
    (hph : Chan.wire p h ∉ allChans sk) :
    pushedCount (s.hist p) h = 0 := by
  by_contra hne
  exact hph (hm.pushed_mem p h hne)

/-- Phantom wire channels carry no deliveries: FIFO plus `pushed_real`
— the vacuity door for every guarded wire fact. -/
theorem delivered_real (hm : MuxInv sk s) (p : Party) (h : Nat)
    (hph : Chan.wire p h ∉ allChans sk) :
    deliveredCount (s.hist p.other) h = 0 := by
  have h1 := hm.delivered_le_pushed p h
  have h2 := hm.pushed_real p h hph
  omega

/-- At the push time of the pipe head, every already-pushed frame is
among the delivered: the FIFO-ancestry input to the keystone
(attack-refute F1's repair), in count form.

`tr` is the observation history at the head's push time; its pushes
are exactly the pushed prefix the deliveries have fully covered. -/
theorem pushtime_delivered (hm : MuxInv sk s) (p : Party)
    {tr : List MObs}
    (htr : pushHeights tr
      = (pushHeights (s.hist p)).take (delTotal (s.hist p.other))) :
    ∀ h, pushedCount tr h ≤ deliveredCount (s.hist p.other) h := by
  intro h
  rw [pushedCount, htr, deliveredCount, hm.hist_del p]
  exact Nat.le_refl _

/-- A wire channel is the tag of some stream. -/
theorem _root_.StreamingMirror.Mux.isWire_eq {c : Chan}
    (hc : isWire c = true) : ∃ p h, c = Chan.wire p h := by
  cases c with
  | wire p h => exact ⟨p, h, rfl⟩
  | _ => simp [isWire] at hc

/-- With both pipes drained, the base state satisfies the full unmuxed
invariant: the muxed conservation law collapses to `InvP.flow`. -/
theorem invP (hm : MuxInv sk s) (hI : s.pipe .I = [])
    (hR : s.pipe .R = []) : InvP sk .impl s.base := by
  refine ⟨hm.invl.wk, hm.invl.asm, hm.invl.top, ?_⟩
  intro c hc
  refine ⟨?_, hm.slot c hc⟩
  cases hw : isWire c with
  | false => exact hm.flow_int c hc hw
  | true =>
      obtain ⟨p, h, rfl⟩ := isWire_eq hw
      have hflow := hm.flow_wire p h hc
      have hpipe : pipeCount s (Chan.wire p h) = 0 := by
        have hempty : s.pipe (wireParty (Chan.wire p h)) = [] := by
          show s.pipe p = []
          cases p
          · exact hI
          · exact hR
        rw [pipeCount, hempty]
        rfl
      omega

end MuxInv

-- ============================================== stuckness consequences

variable {sk : Skel} {ax : AxMode} {C : Nat} {σI σR : Strategy}
  {s : MState}

/-- At a stuck state every muxed action is disabled. -/
theorem mstuck_disabled (hstuck : mstuck sk ax C σI σR s = true) :
    ∀ ma ∈ allMActions sk, (apply sk ax C σI σR ma s).isSome = false := by
  rw [mstuck, Bool.and_eq_true, Bool.not_eq_true', Bool.not_eq_true']
    at hstuck
  rw [mcanStep, List.any_eq_false] at hstuck
  intro ma hma
  have := hstuck.2 ma hma
  simpa using this

/-- One enabled base action is enough for the muxed system to step. -/
theorem mcanStep_of_base {a : Action} (hmem : a ∈ allActions sk)
    (happ : (applyBase sk ax a s).isSome = true) :
    mcanStep sk ax C σI σR s = true := by
  rw [mcanStep, List.any_eq_true]
  refine ⟨.base a, ?_, happ⟩
  rw [allMActions]
  exact List.mem_append.mpr (.inl (List.mem_map_of_mem hmem))

/-- Off the wire fires and the two wire closes, the muxed base arm is
the base model's arm verbatim. -/
theorem applyBase_isSome_of_not_close {a : Action}
    (hnf : isWireFire s.base a = false)
    (hncw : ∀ pk, a ≠ .walkCloseWire pk) (hnab : a ≠ .absorbCloseWire) :
    (applyBase sk ax a s).isSome
      = (Model.apply sk ax a s.base).isSome := by
  cases a
  case walkCloseWire pk => exact absurd rfl (hncw pk)
  case absorbCloseWire => exact absurd rfl hnab
  all_goals simp [applyBase, hnf]

/-- With its producer's pipe empty, a channel's close guard is clear. -/
theorem pipeClear_of_empty {c : Chan}
    (hp : s.pipe (wireParty c) = []) : pipeClear s c = true := by
  simp [pipeClear, hp]

/-- With both pipes drained, the muxed base arm is the base model's arm
verbatim on everything but the wire fires. -/
theorem applyBase_isSome_of_empty (hI : s.pipe .I = [])
    (hR : s.pipe .R = []) {a : Action}
    (hnf : isWireFire s.base a = false) :
    (applyBase sk ax a s).isSome
      = (Model.apply sk ax a s.base).isSome := by
  have hclear : ∀ c, pipeClear s c = true := by
    intro c
    refine pipeClear_of_empty ?_
    cases wireParty c
    · exact hI
    · exact hR
  cases a
  all_goals simp [applyBase, hnf, hclear]

/-- With both pipes drained, the muxed session is complete iff the base
one is. -/
theorem terminal_of_mterminal_false (hI : s.pipe .I = [])
    (hR : s.pipe .R = []) (hnt : mterminal sk s = false) :
    Model.terminal sk s.base = false := by
  rw [mterminal, hI, hR] at hnt
  simpa using hnt

-- ============================================ choice points at mstuck

/-- At a stuck state no walk is parked uncommitted at its choice point:
the pillar would hand it a commit, and commits are muxed-enabled. -/
theorem mstuck_wkh (hwf : sk.wellFormed = true)
    (hL : InvL sk .impl s.base)
    (hstuck : mstuck sk ax C σI σR s = true) (himpl : ax = .impl) :
    ∀ pk ∈ sk.walkKeys,
      ¬((s.base.walk pk).phase = 2
        ∧ (s.base.walk pk).committed = none) := by
  subst himpl
  rintro pk hpk ⟨h2, hn⟩
  obtain ⟨o, hch, hmem⟩ :=
    walk_uncommitted_choosable hwf hL hpk h2 hn (Or.inl rfl)
  have happ : (Model.apply sk .impl (.walkCommit pk o) s.base).isSome
      = true := by
    simp [Model.apply, hpk, hch]
  have hbase : (applyBase sk .impl (.walkCommit pk o) s).isSome
      = true := by
    rw [applyBase_isSome_of_not_close rfl
      (fun _ h => Action.noConfusion h) (fun h => Action.noConfusion h)]
    exact happ
  have := mstuck_disabled hstuck (.base (.walkCommit pk o))
    (by rw [allMActions]
        exact List.mem_append.mpr (.inl (List.mem_map_of_mem hmem)))
  rw [show apply sk .impl C σI σR (.base (.walkCommit pk o)) s
      = applyBase sk .impl (.walkCommit pk o) s from rfl, hbase] at this
  cases this

/-- At a stuck state the initiator opening is not parked at its choice
point: with the choice slot empty it is done. -/
theorem mstuck_ioh (hstuck : mstuck sk ax C σI σR s = true) :
    s.base.iopenCh = none → doneIOpen s.base = true := by
  intro hch
  by_contra hnd
  rw [Bool.not_eq_true, doneIOpen, Bool.and_eq_false_iff] at hnd
  have happ : ∃ o, (Model.apply sk ax (.iopenChoose o) s.base).isSome
      = true := by
    cases hw : s.base.iopenWire with
    | false => exact ⟨.wire, by simp [Model.apply, hch, iopenChoosable, hw]⟩
    | true =>
        have hq : s.base.iopenQuery = false := by
          rcases hnd with h | h
          · rw [hw] at h; cases h
          · exact h
        exact ⟨.query,
          by simp [Model.apply, hch, iopenChoosable, hq, hw]⟩
  obtain ⟨o, happ⟩ := happ
  have hbase : (applyBase sk ax (.iopenChoose o) s).isSome = true := by
    rw [applyBase_isSome_of_not_close rfl
      (fun _ h => Action.noConfusion h) (fun h => Action.noConfusion h)]
    exact happ
  have := mstuck_disabled hstuck (.base (.iopenChoose o))
    (by rw [allMActions]
        exact List.mem_append.mpr
          (.inl (List.mem_map_of_mem iopenChoose_mem)))
  rw [show apply sk ax C σI σR (.base (.iopenChoose o)) s
      = applyBase sk ax (.iopenChoose o) s from rfl, hbase] at this
  cases this

/-- At a stuck state the responder opening is not parked at its choice
point: past its wire receive with the choice slot empty it is done. -/
theorem mstuck_roh (hL : InvL sk ax s.base)
    (hstuck : mstuck sk ax C σI σR s = true) :
    s.base.ropenGotWire = true → s.base.ropenCh = none →
      doneROpen sk s.base = true := by
  intro hgw hch
  by_contra hnd
  rw [Bool.not_eq_true] at hnd
  have happ : ∃ o, (Model.apply sk ax (.ropenChoose o) s.base).isSome
      = true := by
    cases hw : s.base.ropenWire with
    | false =>
        exact ⟨.wire, by simp [Model.apply, hch, ropenChoosable, hgw, hw]⟩
    | true =>
        cases hr : s.base.ropenRes with
        | false =>
            exact ⟨.res,
              by simp [Model.apply, hch, ropenChoosable, hgw, hr, hw]⟩
        | true =>
            have htop := hL.top
            simp only [topLocalOk, Bool.and_eq_true, decide_eq_true_eq]
              at htop
            obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨-, hqle⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩ := htop
            rw [doneROpen, hgw, hw, hr] at hnd
            simp only [Bool.true_and] at hnd
            have hqlt : s.base.ropenQ < (sk.scope 0).kids.length := by
              have : ¬ (s.base.ropenQ = (sk.scope 0).kids.length) := by
                intro heq
                rw [heq] at hnd
                simp at hnd
              have hle : s.base.ropenQ ≤ sk.rootPending := hqle
              rw [Skel.rootPending] at hle
              omega
            exact ⟨.query, by
              simp [Model.apply, hch, ropenChoosable, hgw, hr, hw, hqlt]⟩
  obtain ⟨o, happ⟩ := happ
  have hbase : (applyBase sk ax (.ropenChoose o) s).isSome = true := by
    rw [applyBase_isSome_of_not_close rfl
      (fun _ h => Action.noConfusion h) (fun h => Action.noConfusion h)]
    exact happ
  have := mstuck_disabled hstuck (.base (.ropenChoose o))
    (by rw [allMActions]
        exact List.mem_append.mpr
          (.inl (List.mem_map_of_mem ropenChoose_mem)))
  rw [show apply sk ax C σI σR (.base (.ropenChoose o)) s
      = applyBase sk ax (.ropenChoose o) s from rfl, hbase] at this
  cases this

-- =========================================== the strategy-gated moves

/-- A held stream with pipe room can always be pushed: the `push` guard
succeeds whenever the hand and the room are there (the `firePush`
half of the `enabledPushes` mirror promised in Mux/Basic). -/
theorem firePush_isSome {p : Party} {h : Nat}
    (hh : holdsWire sk p h s.base = true)
    (hroom : (s.pipe p).length < C) :
    (firePush sk C p h s).isSome = true := by
  rw [firePush, if_pos hroom]
  rw [holdsWire.eq_def] at hh
  by_cases hr : (h == sk.rootH) = true
  · rw [if_pos hr] at hh
    rw [if_pos hr]
    cases p with
    | I =>
        have : s.base.iopenCh = some .wire := by
          simpa using hh
        rw [this]
        rfl
    | R =>
        have : s.base.ropenCh = some .wire := by
          simpa using hh
        rw [this]
        rfl
  · rw [if_neg hr] at hh
    rw [if_neg hr]
    simp only [Bool.and_eq_true] at hh
    obtain ⟨⟨hcon, hph⟩, hcm⟩ := hh
    cases hcmm : (s.base.walk (p, h)).committed with
    | none => rw [hcmm] at hcm; cases hcm
    | some o =>
        cases o with
        | wire i =>
            simp [hcmm, hph]
            exact (List.contains_iff_mem ..).mp hcon
        | res i => rw [hcmm] at hcm; cases hcm
        | query i => rw [hcmm] at hcm; cases hcm
        | parent => rw [hcmm] at hcm; cases hcm

/-- A withheld push: a committed wire hand with pipe room — everything
enabled about it except the strategy's word. -/
def WithheldPush (sk : Skel) (C : Nat) (p : Party) (h : Nat)
    (s : MState) : Prop :=
  holdsWire sk p h s.base = true ∧ (s.pipe p).length < C

/-- A held stream is among the party's wire heights. -/
theorem holdsWire_mem_wireHeights {p : Party} {h : Nat}
    (hh : holdsWire sk p h s.base = true) : h ∈ wireHeights sk p := by
  rw [holdsWire.eq_def] at hh
  by_cases hr : (h == sk.rootH) = true
  · rw [wireHeights]
    have : h = sk.rootH := by simpa using hr
    rw [this]
    exact List.mem_cons_self ..
  · rw [if_neg hr] at hh
    simp only [Bool.and_eq_true] at hh
    obtain ⟨⟨hcon, -⟩, -⟩ := hh
    rw [wireHeights]
    refine List.mem_cons_of_mem _ (List.mem_filterMap.mpr ⟨(p, h), ?_, ?_⟩)
    · exact (List.contains_iff_mem ..).mp hcon
    · simp

/-- The `enabledPushes` list is exactly the withheld-or-taken pushes:
room plus a held stream (the `enabledPushes_spec` promised in
Mux/Strategy). -/
theorem mem_enabledPushes {p : Party} {h : Nat} :
    h ∈ enabledPushes sk C p s
      ↔ (s.pipe p).length < C ∧ holdsWire sk p h s.base = true := by
  rw [enabledPushes]
  by_cases hroom : (s.pipe p).length < C
  · rw [if_pos hroom, List.mem_filter]
    constructor
    · rintro ⟨-, hw⟩
      exact ⟨hroom, hw⟩
    · rintro ⟨-, hw⟩
      exact ⟨holdsWire_mem_wireHeights hw, hw⟩
  · rw [if_neg hroom]
    simp [hroom]

/-- At a stuck state the strategy declines every withheld push: naming
it would enable the `push` move. -/
theorem mstuck_withheld (hstuck : mstuck sk ax C σI σR s = true)
    {p : Party} {h : Nat} (hwp : WithheldPush sk C p h s) :
    (match p with | .I => σI | .R => σR) sk (s.hist p) ≠ some h := by
  intro hname
  have hdis := mstuck_disabled hstuck (.push p)
    (by rw [allMActions]
        refine List.mem_append.mpr (.inr ?_)
        cases p <;> simp)
  rw [show apply sk ax C σI σR (.push p) s
      = (match (match p with | .I => σI | .R => σR) sk (s.hist p) with
         | some h' => firePush sk C p h' s
         | none => none) from by cases p <;> rfl, hname] at hdis
  rw [firePush_isSome hwp.1 hwp.2] at hdis
  cases hdis

end StreamingMirror.Mux
