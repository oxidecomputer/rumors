/-
The chase (MUX-ADJUDICATION.md §3 T2, stage-2 track B): the shared
proof infrastructure both stage-3 theorems consume — T5's oracle
(argmin + π-eligibility) and T4's σ* (keystone + coverage).

# The module tree

- Chase/Ground.lean — history counting, the `MuxInv` ground facts, and
  the stuckness transfer lemmas (muxed-enabled ⇔ base-enabled off the
  wire fires).
- Chase/Closure.lean — the Certified/Inevitable demand closures
  (refute-c1 §1.3, repaired per attack-refute F1/F6).
- Chase/Decode.lean — the base pending layer packaged for the mux:
  pool inversions, the unified frontier decode, τ-comparison tools.
- Chase/Keystone.lean — T2: closure members are performed at stuck
  states (the push-time-tree route).
- this file — the τ-well-founded chase: a stuck, drained, incomplete
  state exhibits the τ-least unperformed event as a WITHHELD PUSH,
  every τ-earlier event performed.

# How the two consumers use the chase

T5 (oracle): the chase names the τ-least unperformed event as a wire
send whose hand is committed; π-eligibility then argues the oracle's
demand order has this exact frame at its head, so `ofSchedule` would
have pushed — stuckness refuted.

T4 (σ*): Step 1 uses the keystone (with `MuxInv.pushtime_delivered`
discharging the FIFO wall at the pipe head's push time) to drain the
pipes at any stuck candidate; Steps 2–3 are this chase; Step 4 (the
coverage induction) shows the withheld frame's demand proof succeeds
from the τ-below traffic, refuting σ*'s idling — the one genuinely new
induction left to stage 3, gated on the stage-0 probe.
-/
import StreamingMirror.Mux.Proofs.Chase.Keystone

namespace StreamingMirror.Mux

open Model
open Sched (Ev procsE scheduleE performed pends PendOkE evIdx)

variable {sk : Skel}

/-- The chase: at a stuck, pipes-drained, incomplete muxed state, the
τ-least unperformed event of the session is a withheld push.

The conclusion names the frame three ways at once: as the event (a
wire send at its live seq, scheduled, unperformed, τ-least among
unperformed — so every DAG predecessor is performed, the last
conjunct), as the pending pool entry carrying its enabledness contract
(`PendOkE`), and as the committed hand (`holdsWire` — with pipe room
free, only the strategy's word is missing; see `chase_withheld`).

Pipes-empty is a hypothesis, not a consequence: under an arbitrary
strategy, stuck states with buried frames exist (the wedge jam), and
there the τ-least unperformed event can be an undeliverable receive.
T4's Step 1 discharges it via the keystone; T5's oracle proof supplies
its own drain argument. -/
theorem chase (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel)
    {C : Nat} {σI σR : Strategy} {s : MState}
    (hm : MuxInv sk s)
    (hpI : s.pipe .I = []) (hpR : s.pipe .R = [])
    (hstuck : mstuck sk .impl C σI σR s = true)
    (hnt : mterminal sk s = false) :
    ∃ (f : Ev) (a : Action) (p : Party) (hh : Nat),
      f.1 = Chan.wire p hh ∧ f.2.1 = true
      ∧ f.2.2 = sentOf sk s.base f.1
      ∧ f ∈ scheduleE sk ∧ ¬ performed sk s.base f
      ∧ (∀ g ∈ scheduleE sk, ¬ performed sk s.base g →
          evIdx f (scheduleE sk) ≤ evIdx g (scheduleE sk))
      ∧ (∀ g ∈ scheduleE sk,
          evIdx g (scheduleE sk) < evIdx f (scheduleE sk) →
          performed sk s.base g)
      ∧ (f, a) ∈ pends sk s.base ∧ PendOkE sk s.base f a
      ∧ holdsWire sk p hh s.base = true := by
  have hi : InvP sk .impl s.base := hm.invP hpI hpR
  have hL : InvL sk .impl s.base := hm.invl
  have hioh := mstuck_ioh (sk := sk) hstuck
  have hroh := mstuck_roh (sk := sk) hL hstuck
  have hwkh := mstuck_wkh hwf hL hstuck rfl
  have hterm : Model.terminal sk s.base = false :=
    terminal_of_mterminal_false hpI hpR hnt
  -- an enabled non-fire base action refutes stuckness
  have hkill : ∀ {a : Action}, a ∈ allActions sk →
      isWireFire s.base a = false →
      (Model.apply sk .impl a s.base).isSome = true → False := by
    intro a hmem hnf hsome
    have hbase : (applyBase sk .impl a s).isSome = true := by
      rw [applyBase_isSome_of_empty hpI hpR hnf]
      exact hsome
    have hen := mcanStep_of_base (C := C) (σI := σI) (σR := σR)
      hmem hbase
    have hno : mcanStep sk .impl C σI σR s = false := by
      rw [mstuck, Bool.and_eq_true, Bool.not_eq_true',
        Bool.not_eq_true'] at hstuck
      exact hstuck.2
    rw [hen] at hno
    cases hno
  by_cases hp : pends sk s.base = []
  case pos =>
      -- no channel work pends, so the closes cascade: the enabled
      -- action progress guarantees cannot be a fire (no hands exist)
      exfalso
      have hnil := hp
      unfold Sched.pends at hnil
      rw [List.append_eq_nil_iff, List.append_eq_nil_iff,
        List.append_eq_nil_iff, List.append_eq_nil_iff,
        List.append_eq_nil_iff, List.append_eq_nil_iff] at hnil
      obtain ⟨⟨⟨⟨⟨⟨hio0, hro0⟩, hwk0⟩, hab0⟩, hasm0⟩, hrr0⟩, hfin0⟩ := hnil
      have hcan := Sched.progress_of_inv sk hwf hm0 hi.weak hterm
      rw [Model.canStep, List.any_eq_true] at hcan
      obtain ⟨a, hmem, hsome'⟩ := hcan
      have hnf : isWireFire s.base a = false := by
        cases hIF : isWireFire s.base a with
        | false => rfl
        | true =>
            exfalso
            cases a
            case iopenFire =>
                have hch : s.base.iopenCh = some .wire := by
                  simpa [isWireFire] using hIF
                simp [Sched.ioPend, hch] at hio0
            case ropenFire =>
                have hch : s.base.ropenCh = some .wire := by
                  simpa [isWireFire] using hIF
                simp only [Sched.roPend] at hro0
                split at hro0
                · simp at hro0
                · rw [hch] at hro0
                  simp at hro0
            case walkFire pk =>
                have hcm : ∃ i, (s.base.walk pk).committed
                    = some (.wire i) := by
                  simp only [isWireFire] at hIF
                  split at hIF
                  · next i heq => exact ⟨i, heq⟩
                  · cases hIF
                obtain ⟨i, hcm⟩ := hcm
                simp only [Model.apply, hcm] at hsome'
                split at hsome'
                case isTrue hcond =>
                    simp only [Bool.and_eq_true] at hcond
                    obtain ⟨⟨hcon, hph⟩, -⟩ := hcond
                    have hpk : pk ∈ sk.walkKeys :=
                      (List.contains_iff_mem ..).mp hcon
                    have hwkpk := List.flatMap_eq_nil_iff.1 hwk0 pk hpk
                    have hph2 : (s.base.walk pk).phase = 2 := by
                      simpa using hph
                    simp [Sched.wkPend, hph2, hcm] at hwkpk
                case isFalse => simp at hsome'
            all_goals simp [isWireFire] at hIF
      exact hkill hmem hnf hsome'
  case neg =>
      -- the τ-least pending event
      obtain ⟨fa, hfam, hfmin⟩ := Sched.exists_min_image
        (fun fa : Ev × Action => evIdx fa.1 (scheduleE sk)) hp
      obtain ⟨⟨c, b, n⟩, a⟩ := fa
      obtain ⟨hok, T, pre, suf, hT, hdec, hpre⟩ :=
        Sched.pends_soundE sk hwf hL hioh hroh hwkh _ hfam
      have hfsched : ((c, b, n) : Ev) ∈ scheduleE sk := by
        have hmemT : ((c, b, n) : Ev) ∈ T := by
          rw [hdec]
          exact List.mem_append.mpr (.inr (List.mem_cons_self ..))
        exact (Sched.trace_sublistE sk hwf hm0 hT).mem hmemT
      have hτget : (scheduleE sk)[evIdx ((c, b, n) : Ev)
          (scheduleE sk)]? = some (c, b, n) :=
        Sched.evIdx_getElem? hfsched
      -- τ-least among ALL unperformed scheduled events, via the cover
      have hcover : ∀ g ∈ scheduleE sk, ¬ performed sk s.base g →
          evIdx ((c, b, n) : Ev) (scheduleE sk)
            ≤ evIdx g (scheduleE sk) := by
        intro g hg hgnp
        obtain ⟨fa', hfam', hτle⟩ := Sched.pends_coverE sk hwf hm0 hL
          hioh hroh hwkh hg hgnp
        exact Nat.le_trans (hfmin fa' hfam') hτle
      have hflow := hi.flow c hok.chan_mem
      have hfnp : ¬ performed sk s.base ((c, b, n) : Ev) :=
        Sched.pend_not_performedE sk hok
      cases b with
      | false =>
          -- a starving receive is impossible with the pipes drained:
          -- its send is τ-below (E1), unperformed by its own seq —
          -- against the cover — so data is present and the receive
          -- fires, against stuckness
          exfalso
          have hseq2 : n = recvdOf sk s.base c := by simpa using hok.seq
          by_cases hdata : 0 < s.base.chan c
          · have hsome := hok.fire (by simpa using hdata)
            have hnf : isWireFire s.base a = false := by
              cases hIF : isWireFire s.base a with
              | false => rfl
              | true =>
                  obtain ⟨q₂, hh₂, -, hfb, -⟩ :=
                    pends_wireFire hfam hIF
                  simp at hfb
            exact hkill hok.act hnf hsome
          · have hE1 := Sched.scheduleE_e1 sk
              (evIdx ((c, false, n) : Ev) (scheduleE sk)) c n hτget
            have hsndlt : sentOf sk s.base c < Sched.sndCount c
                ((scheduleE sk).take
                  (evIdx ((c, false, n) : Ev) (scheduleE sk))) := by
              omega
            obtain ⟨j, hjlt, hjget⟩ := Sched.mem_take_snd
              (scheduleE_canon_self hwf c true) hsndlt
            have hgmem : ((c, true, sentOf sk s.base c) : Ev)
                ∈ scheduleE sk := List.mem_iff_getElem?.2 ⟨j, hjget⟩
            have hgnp : ¬ performed sk s.base
                ((c, true, sentOf sk s.base c) : Ev) := by
              rw [performed_snd_iff]
              omega
            have hjeq : j = evIdx ((c, true, sentOf sk s.base c) : Ev)
                (scheduleE sk) :=
              Sched.evIdx_unique
                (Sched.scheduleE_count_le_oneE sk hwf _) hjget
            have := hcover _ hgmem hgnp
            omega
      | true =>
          have hseq2 : n = sentOf sk s.base c := by simpa using hok.seq
          by_cases hroom : s.base.chan c < sk.cap c
          · -- the guard is open: the pending fire is enabled, and at a
            -- stuck state it can only be a wire fire — the withheld
            -- push, delivered with its τ-minimality certificate
            have hsome := hok.fire (by simpa using hroom)
            cases hIF : isWireFire s.base a with
            | false => exact absurd hsome (fun hs => hkill hok.act hIF hs)
            | true =>
                obtain ⟨q, hh, hfc, -, hhold⟩ :=
                  pends_wireFire hfam hIF
                simp only at hfc
                refine ⟨(c, true, n), a, q, hh, hfc, rfl, hseq2,
                  hfsched, hfnp, hcover, ?_, hfam, hok, hhold⟩
                intro g hg hlt
                by_cases hperf : performed sk s.base g
                · exact hperf
                · have := hcover g hg hperf
                  omega
          · -- a jammed send is impossible with the pipes drained: its
            -- cap-window receive is τ-below (E2), unperformed by its
            -- own seq — against the cover
            exfalso
            have hE2 := Sched.scheduleE_e2 sk
              (evIdx ((c, true, n) : Ev) (scheduleE sk)) c n hτget
            have hrcvlt : recvdOf sk s.base c < Sched.rcvCount c
                ((scheduleE sk).take
                  (evIdx ((c, true, n) : Ev) (scheduleE sk))) := by
              omega
            obtain ⟨j, hjlt, hjget⟩ := Sched.mem_take_rcv
              (scheduleE_canon_self hwf c false) hrcvlt
            have hgmem : ((c, false, recvdOf sk s.base c) : Ev)
                ∈ scheduleE sk := List.mem_iff_getElem?.2 ⟨j, hjget⟩
            have hgnp : ¬ performed sk s.base
                ((c, false, recvdOf sk s.base c) : Ev) := by
              rw [performed_rcv_iff]
              omega
            have hjeq : j = evIdx ((c, false, recvdOf sk s.base c) : Ev)
                (scheduleE sk) :=
              Sched.evIdx_unique
                (Sched.scheduleE_count_le_oneE sk hwf _) hjget
            have := hcover _ hgmem hgnp
            omega

/-- The chase's strategy-facing corollary: the withheld push is fully
enabled — hand committed, pipe room free — and the stuck strategy has
provably declined it. -/
theorem chase_withheld (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel)
    {C : Nat} {σI σR : Strategy} {s : MState}
    (hm : MuxInv sk s) (hC : 1 ≤ C)
    (hpI : s.pipe .I = []) (hpR : s.pipe .R = [])
    (hstuck : mstuck sk .impl C σI σR s = true)
    (hnt : mterminal sk s = false) :
    ∃ (p : Party) (hh : Nat), WithheldPush sk C p hh s
      ∧ (match p with | .I => σI | .R => σR) sk (s.hist p) ≠ some hh := by
  obtain ⟨f, a, p, hh, -, -, -, -, -, -, -, -, -, hhold⟩ :=
    chase hwf hm0 hm hpI hpR hstuck hnt
  have hroom : (s.pipe p).length < C := by
    have hempty : s.pipe p = [] := by
      cases p
      · exact hpI
      · exact hpR
    rw [hempty]
    simp only [List.length_nil]
    omega
  exact ⟨p, hh, ⟨hhold, hroom⟩, mstuck_withheld hstuck ⟨hhold, hroom⟩⟩

-- ============================================== stage-3 bridging bricks

/-- At a stuck state a nonempty pipe's head is slot-blocked: `deliver`
is never strategy-gated, so only a full slot can hold it.

This is T4-Step-1's opening move (refute-c1 §2.1): together with the
`MuxInv` count fields it pins the head-of-line configuration — the
slot frame is the head's per-stream predecessor at seq
`recvdOf = deliveredCount − 1` — whose unconsumed-ness the keystone
then contradicts. -/
theorem mstuck_deliver_blocked {C : Nat} {σI σR : Strategy}
    {s : MState} (hstuck : mstuck sk .impl C σI σR s = true)
    {p : Party} {c : Chan} {rest : List Chan}
    (hp : s.pipe p = c :: rest) : s.base.chan c ≠ 0 := by
  intro h0
  have hdis := mstuck_disabled hstuck (.deliver p)
    (by rw [allMActions]
        refine List.mem_append.mpr (.inr ?_)
        cases p <;> simp)
  simp [apply, hp, h0] at hdis

/-- Pipe entries are wire frames of their own pipe's party. -/
theorem MuxInv.pipe_mem_wire {s : MState} (hm : MuxInv sk s) {p : Party}
    {c : Chan} (hc : c ∈ s.pipe p) : ∃ hh, c = Chan.wire p hh := by
  rw [hm.hist_pipe p] at hc
  obtain ⟨hh, -, rfl⟩ := List.exists_of_mem_map hc
  exact ⟨hh, rfl⟩


/-- The ground facts hold at the initial muxed state: the base case of
the stage-3 `MuxInv` preservation induction, and the interface's
non-vacuity certificate. -/
theorem muxInv_init (sk : Skel) : MuxInv sk (init sk) := by
  refine ⟨((inv_iff sk .impl (Model.init sk)).mp (inv_init sk .impl)).local,
    ?_, ?_, ?_, ?_, ?_, ?_⟩
  · intro c _
    exact Nat.zero_le _
  · intro c _ _
    rw [show (init sk).base = Model.init sk from rfl,
      sentOf_init, recvdOf_init]
    rfl
  · intro p h
    rw [show (init sk).base = Model.init sk from rfl, sentOf_init]
    rfl
  · intro p
    rfl
  · intro p
    rfl
  · intro p h
    rw [show (init sk).base = Model.init sk from rfl, recvdOf_init,
      chan_init]
    rfl

-- ================================= kernel-tier non-vacuity anchors
-- The closure definitions would satisfy the keystone vacuously if they
-- never derived anything; the anchors pin, in the kernel, that the
-- fixpoint is empty exactly when it must be (self-containment: zero
-- evidence grounds zero events) and populated exactly when it can be
-- (one arrival grounds the peer's opening receive and the forced
-- query behind it).

set_option maxRecDepth 100000 in
/-- With no observations nothing is derivable: every session event
waits, directly or through its trace past, on an unevidenced push. -/
theorem smokeChain_inevitable_nil :
    inevitable Pin.smokeChain .I [] = [] := by decide

set_option maxRecDepth 100000 in
/-- One delivered opening frame grounds the responder's first receive:
the forward derivation genuinely derives. -/
theorem smokeChain_inevitable_arrival :
    ((Chan.wire .I 4, false, 0) : Sched.Ev)
      ∈ inevitable Pin.smokeChain .R [.delivered 4] := by decide

end StreamingMirror.Mux
