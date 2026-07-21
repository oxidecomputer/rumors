/-
T5, the oracle (MUX-ADJUDICATION.md §3, stage-3 track E): the
send-projection pusher is deadlock-free on the whole `.impl` + margin-0
class at every capacity C ≥ 1 — C₀ = 1 suffices.

# The proof, one paragraph

`OracleInv` (Oracle/Order.lean) makes a party's pushes the τ-prefix of
its send projection, so pipe positions ARE `scheduleE` positions. At a
hypothetical stuck state, either both pipes are drained — then the
landed chase (T2) exhibits the τ-least unperformed event as a withheld
push whose hand is committed and whose pipe has room, and
`oracle_names` proves the oracle names exactly that frame, against
`mstuck_withheld` — or some pipe is nonempty, and the τ-argmin runs
here without the drain hypothesis: the τ-least pending event `f`
cannot be an enabled receive or internal send (stuckness), cannot be a
starving receive or jammed send (E1/E2 + the cover, exactly the chase's
counting blocks, with `MuxInv.flow_wire` standing in for the unmuxed
flow), and every remaining shape runs into the HEAD CYCLE: the head of
a stuck pipe is slot-blocked (`mstuck_deliver_blocked`), its E2
predecessor `r₀` is unperformed, so τ(f) ≤ τ(r₀) < τ(head) by the
cover; but the head was PUSHED while `f`'s frame was not (or was pushed
later), so τ(head) < τ(f) by the τ-prefix invariant — a cycle. The
oracle needs no adaptivity and no drain lemma: send-order pushing makes
FIFO burial τ-decreasing, hence impossible.

# What this refutes, and what it does not

The receive-projection pusher `ofSchedule (demandOrder sk d)` jams
(MUX-PROGRESS.md, the π-eligibility failure; kernel-pinned by
`static_oracle_jams`, Oracle/Controls.lean) — yet this theorem shows a
different STATIC order is live everywhere. The muxprobe finding's moral
("adaptivity, not information, is the liveness ingredient") is thereby
sharpened at kernel tier: the ingredient is neither adaptivity nor
information beyond the skeleton — it is the ORDER. Pushing in
consumption order fails because commit dependencies can force a send
early whose consumption comes late (cross-stream skew); pushing in
SEND order (τ's own emission order) is always safe because the demux
slots absorb exactly that skew. Scope notes: capacity is denominated in
messages (= scope replies); byte-level soundness of one-reply slots is
design/streaming-wire-deadlock.md §5A's W = 1 structural argument,
assumed at the model boundary (MUX-ADJUDICATION §2.5). NOT claimed:
overlap/latency optimality (executable tier only, H-c demoted).

# The `MuxInv` hypothesis

The ground facts `MuxInv` (Chase/Ground.lean) enter as an explicit
reachability hypothesis (`oracle_deadlock_free_of_muxInv`), exactly as
the keystone and chase consume them; the MUX-ADJUDICATION §4 stage-F
obligation is discharged by the strategy-generic sweep in
Proofs/SigmaStarInv.lean (`muxInv_reachable`, the `MuxInv` projection
of `sinv_reachable`), which closes `oracle_deadlock_free`
unconditionally in Proofs/Necessity.lean. Track E's own stage-F sweep
was retired at the stage-3 merge in favor of that one — see the
integration notes in Chase/Ground.lean and SigmaStarInv.lean.
-/
import StreamingMirror.Mux.Proofs.Oracle.Order

namespace StreamingMirror.Mux

open Model
open Sched (Ev procsE scheduleE performed pends PendOkE evIdx)

variable {sk : Skel}

/-- A scheduled wire event rides a real channel: schedule provenance
(`sched_mem_traceE`) places it in the event universe, whose wire
channels are `allChans` members — the membership guard on `MuxInv`'s
count equations, discharged from scheduledness alone. -/
theorem scheduleE_wire_mem (hwf : sk.wellFormed = true) {q : Party}
    {g n : Nat} {b : Bool}
    (he : ((Chan.wire q g, b, n) : Ev) ∈ scheduleE sk) :
    Chan.wire q g ∈ allChans sk :=
  evUniv_wire_mem hwf (mem_evUniv.mpr (Sched.sched_mem_traceE sk he))

-- ================================================== the naming lemma

/-- At a state whose τ-least unperformed scheduled event is one's own
next wire send, the oracle names it.

The three hypotheses are exactly what the chase (drained case) and the
argmin (nonempty case) produce: the frame is scheduled, its seq is the
live count, and every τ-earlier scheduled event is performed. -/
theorem oracle_names (hwf : sk.wellFormed = true) {s : MState}
    {p : Party} (hm : MuxInv sk s) (ho : OracleInv sk p s) {hh n : Nat}
    (hmem : ((Chan.wire p hh, true, n) : Ev) ∈ scheduleE sk)
    (hseq : n = sentOf sk s.base (Chan.wire p hh))
    (hmin : ∀ g ∈ scheduleE sk, ¬ performed sk s.base g →
      evIdx ((Chan.wire p hh, true, n) : Ev) (scheduleE sk)
        ≤ evIdx g (scheduleE sk)) :
    oracle p sk (s.hist p) = some hh := by
  have hf : ((Chan.wire p hh, true, n) : Ev) ∈ sendProj sk p :=
    mem_sendProj hmem
  obtain ⟨jf, hjf⟩ := List.mem_iff_getElem?.mp hf
  have hjflt : jf < (sendProj sk p).length :=
    (List.getElem?_eq_some_iff.mp hjf).1
  -- own push count
  have hKle : (pushHeights (s.hist p)).length ≤ (sendProj sk p).length := by
    have hlen := congrArg List.length ho
    rw [pushEvs, evsOf_length, List.length_take] at hlen
    omega
  rcases Nat.lt_trichotomy jf (pushHeights (s.hist p)).length with
    hlt | heq | hgt
  · -- jf < K: the frame would already be pushed — its seq caps out
    exfalso
    have htake : ((sendProj sk p).take
        (pushHeights (s.hist p)).length)[jf]?
        = some (Chan.wire p hh, true, n) := by
      rw [List.getElem?_take_of_lt hlt]
      exact hjf
    have hmem' : ((Chan.wire p hh, true, n) : Ev)
        ∈ evsOf p (pushHeights (s.hist p)) := by
      rw [show (evsOf p (pushHeights (s.hist p)))
          = (sendProj sk p).take (pushHeights (s.hist p)).length from ho]
      exact List.mem_of_getElem? htake
    obtain ⟨h', n', heq', hlt'⟩ := evsOf_mem_inv hmem'
    have hh' : h' = hh ∧ n' = n := by
      have h1 := congrArg (fun e : Ev => e.1) heq'
      have h2 := congrArg (fun e : Ev => e.2.2) heq'
      simp only at h1 h2
      rw [Chan.wire.injEq] at h1
      exact ⟨h1.2.symm, h2.symm⟩
    obtain ⟨rfl, rfl⟩ := hh'
    have hpc : pushedCount (s.hist p) h' = sentOf sk s.base (Chan.wire p h') :=
      hm.pushed_eq p h' (scheduleE_wire_mem hwf hmem)
    unfold pushedCount at hpc
    omega
  · -- jf = K: the oracle reads exactly this entry
    unfold oracle
    rw [← heq, hjf]
    rfl
  · -- jf > K: the projection's K-th entry is τ-earlier and unperformed
    exfalso
    have hKlt : (pushHeights (s.hist p)).length < (sendProj sk p).length := by
      omega
    cases hg : (sendProj sk p)[(pushHeights (s.hist p)).length]? with
    | none =>
        rw [List.getElem?_eq_none_iff] at hg
        omega
    | some g =>
    have hgmem : g ∈ sendProj sk p := List.mem_of_getElem? hg
    obtain ⟨hgsched, hg', ng, hgdec⟩ := sendProj_mem hgmem
    -- its seq is the live count of its stream
    have hcanon : Sched.proj (Chan.wire p hg') true (sendProj sk p)
        = Sched.canon (Chan.wire p hg') true
            (Sched.proj (Chan.wire p hg') true (sendProj sk p)).length := by
      rw [proj_sendProj]
      exact scheduleE_canon_self hwf _ true
    have hgseq : ng = Sched.sndCount (Chan.wire p hg')
        ((sendProj sk p).take (pushHeights (s.hist p)).length) :=
      seq_eq_sndCount_take hcanon (hgdec ▸ hg)
    have hgcnt : ng = pushedCount (s.hist p) hg' := by
      unfold pushedCount
      rw [hgseq, ← ho, pushEvs, sndCount_evsOf]
    have hgsent : ng = sentOf sk s.base (Chan.wire p hg') := by
      rw [hgcnt, hm.pushed_eq p hg'
        (scheduleE_wire_mem hwf (hgdec ▸ hgsched))]
    have hgnp : ¬ performed sk s.base g := by
      rw [hgdec, performed_snd_iff]
      omega
    have hτle := hmin g hgsched hgnp
    have hτgt := sendProj_evIdx_lt hwf hgt hg hjf
    omega

-- ================================================== the pipe positions

/-- First occurrence: a member sits at a position it never occupies
earlier. -/
private theorem exists_first_getElem? {α : Type _} [BEq α] [LawfulBEq α]
    {a : α} : ∀ {l : List α}, a ∈ l →
      ∃ j, l[j]? = some a ∧ (l.take j).count a = 0 := by
  intro l
  induction l with
  | nil => intro h; cases h
  | cons b l ih =>
      intro h
      by_cases hb : b = a
      · exact ⟨0, by rw [hb]; rfl, by simp⟩
      · have hmem : a ∈ l := by
          rcases List.mem_cons.mp h with h' | h'
          · exact absurd h'.symm hb
          · exact h'
        obtain ⟨j, hj, hcnt⟩ := ih hmem
        refine ⟨j + 1, by simpa using hj, ?_⟩
        rw [List.take_succ_cons, List.count_cons]
        simp [hcnt, hb]

/-- The FIRST in-flight frame of channel `c` in pipe `q` is the send at
seq `recvdOf + chan`, sitting in `q`'s send projection at absolute
position `delTotal + j` where `j` is its pipe position: FIFO positions
become τ positions.

This is where the three `MuxInv` counting fields and the τ-prefix
invariant meet: `hist_pipe` names the frame's push index, `hist_del` +
`delivered_eq` compute its seq, and `OracleInv` places it in the
projection. -/
theorem pipe_first_frame_pos {s : MState} {q : Party}
    (hm : MuxInv sk s) (ho : OracleInv sk q s)
    {c : Chan} (hc : c ∈ s.pipe q) :
    ∃ j, (s.pipe q)[j]? = some c
      ∧ ((s.pipe q).take j).count c = 0
      ∧ delTotal (s.hist q.other) + j < (pushHeights (s.hist q)).length
      ∧ (sendProj sk q)[delTotal (s.hist q.other) + j]?
          = some (c, true, recvdOf sk s.base c + s.base.chan c) := by
  obtain ⟨j, hj, hcnt⟩ := exists_first_getElem? hc
  refine ⟨j, hj, hcnt, ?_⟩
  have hpipe := hm.hist_pipe q
  -- the frame's height and push index
  have hjmap : ((pushHeights (s.hist q)).drop
      (delTotal (s.hist q.other)))[j]?.map (Chan.wire q) = some c := by
    rw [← List.getElem?_map, ← hpipe]
    exact hj
  cases hget : ((pushHeights (s.hist q)).drop
      (delTotal (s.hist q.other)))[j]? with
  | none => rw [hget] at hjmap; cases hjmap
  | some hc' =>
      rw [hget] at hjmap
      have hceq : Chan.wire q hc' = c := by simpa using hjmap
      rw [List.getElem?_drop] at hget
      have hlt : delTotal (s.hist q.other) + j
          < (pushHeights (s.hist q)).length :=
        (List.getElem?_eq_some_iff.mp hget).1
      refine ⟨hlt, ?_⟩
      -- the numbered event at that push index
      have hev := evsOf_getElem? q (l := pushHeights (s.hist q)) hlt
      have hgetD : (pushHeights (s.hist q)).getD
          (delTotal (s.hist q.other) + j) 0 = hc' := by
        rw [List.getD_eq_getElem?_getD, hget]
        rfl
      rw [hgetD] at hev
      -- its seq: delivered count plus zero earlier in-flight occurrences
      have hsplit : (pushHeights (s.hist q)).take
          (delTotal (s.hist q.other) + j)
          = (pushHeights (s.hist q)).take (delTotal (s.hist q.other))
            ++ ((pushHeights (s.hist q)).drop
                (delTotal (s.hist q.other))).take j :=
        List.take_add ..
      have hreal : Chan.wire q hc' ∈ allChans sk := by
        by_contra hph
        have h0 := hm.pushed_real q hc' hph
        have hmemH : hc' ∈ pushHeights (s.hist q) :=
          List.mem_of_getElem? hget
        have h1 := List.one_le_count_iff.mpr hmemH
        unfold pushedCount at h0
        omega
      have hcnt1 : ((pushHeights (s.hist q)).take
          (delTotal (s.hist q.other))).count hc'
          = recvdOf sk s.base c + s.base.chan c := by
        rw [← hm.hist_del q, ← hceq]
        exact hm.delivered_eq q hc' hreal
      have hcnt2 : (((pushHeights (s.hist q)).drop
          (delTotal (s.hist q.other))).take j).count hc' = 0 := by
        have hmap : (s.pipe q).take j
            = (((pushHeights (s.hist q)).drop
                (delTotal (s.hist q.other))).take j).map (Chan.wire q) := by
          rw [hpipe, List.map_take]
        rw [hmap, ← hceq, count_map_wire] at hcnt
        exact hcnt
      have hcount : ((pushHeights (s.hist q)).take
          (delTotal (s.hist q.other) + j)).count hc'
          = recvdOf sk s.base c + s.base.chan c := by
        rw [hsplit, List.count_append, hcnt1, hcnt2]
        omega
      -- transport through the τ-prefix invariant
      have hev' : (evsOf q (pushHeights (s.hist q)))[delTotal
          (s.hist q.other) + j]?
          = some (c, true, recvdOf sk s.base c + s.base.chan c) := by
        rw [hev, hcount, hceq]
      rw [show evsOf q (pushHeights (s.hist q))
          = (sendProj sk q).take (pushHeights (s.hist q)).length
          from ho] at hev'
      rw [List.getElem?_take_of_lt hlt] at hev'
      exact hev'

-- ==================================================== the head kit

/-- The head kit: a stuck nonempty pipe's head is a pushed, scheduled
send sitting at projection position `delTotal`, slot-blocked, with its
E2 predecessor scheduled, unperformed, and τ-below it.

`r₀ := (c₀, false, m−1)` is the unconsumed slot frame's receive; every
τ-argmin contradiction in the stuck theorem routes through it. -/
theorem head_kit (hwf : sk.wellFormed = true) {s : MState} {q : Party}
    {C : Nat} {σI σR : Strategy}
    (hm : MuxInv sk s) (ho : OracleInv sk q s)
    (hstuck : mstuck sk .impl C σI σR s = true)
    {c₀ : Chan} {rest : List Chan} (hp : s.pipe q = c₀ :: rest) :
    ∃ m, m = recvdOf sk s.base c₀ + s.base.chan c₀
      ∧ s.base.chan c₀ ≠ 0
      ∧ delTotal (s.hist q.other) < (pushHeights (s.hist q)).length
      ∧ (sendProj sk q)[delTotal (s.hist q.other)]? = some (c₀, true, m)
      ∧ ((c₀, true, m) : Ev) ∈ scheduleE sk
      ∧ ((c₀, false, m - 1) : Ev) ∈ scheduleE sk
      ∧ ¬ performed sk s.base ((c₀, false, m - 1) : Ev)
      ∧ evIdx ((c₀, false, m - 1) : Ev) (scheduleE sk)
          < evIdx ((c₀, true, m) : Ev) (scheduleE sk) := by
  have hchan := mstuck_deliver_blocked hstuck hp
  have hcmem : c₀ ∈ s.pipe q := by
    rw [hp]
    exact List.mem_cons_self ..
  obtain ⟨hh₀, hc₀⟩ := hm.pipe_mem_wire hcmem
  obtain ⟨j, hj, hcnt, hlt, hpos⟩ := pipe_first_frame_pos hm ho hcmem
  -- the first occurrence of the head is the head
  have hj0 : j = 0 := by
    by_contra hne
    have hj1 : 1 ≤ j := Nat.one_le_iff_ne_zero.mpr hne
    have hmem0 : c₀ ∈ (s.pipe q).take j := by
      have : ((s.pipe q).take j)[0]? = some c₀ := by
        rw [List.getElem?_take_of_lt (by omega), hp]
        rfl
      exact List.mem_of_getElem? this
    have := List.one_le_count_iff.mpr hmem0
    omega
  subst hj0
  rw [Nat.add_zero] at hlt hpos
  have hmem : ((c₀, true, recvdOf sk s.base c₀ + s.base.chan c₀) : Ev)
      ∈ scheduleE sk :=
    (sendProj_mem (List.mem_of_getElem? hpos)).1
  -- the E2 predecessor: wire channels have cap 1
  have hcap : sk.cap c₀ = 1 := by
    rw [hc₀]
    rfl
  have hcaple : sk.cap c₀ ≤ recvdOf sk s.base c₀ + s.base.chan c₀ := by
    omega
  obtain ⟨hrmem, hτ⟩ := tau_e2 hwf hmem hcaple
  rw [hcap] at hrmem hτ
  refine ⟨recvdOf sk s.base c₀ + s.base.chan c₀, rfl, hchan, hlt, hpos,
    hmem, hrmem, ?_, hτ⟩
  rw [performed_rcv_iff]
  omega

-- ==================================================== the stuck theorem

/-- The τ-argmin at a stuck state with a nonempty pipe: impossible under
the oracle pair. The head cycle in every branch: τ(f) ≤ τ(r₀) < τ(head)
by the cover and E2, and τ(head) < τ(f's frame) by the τ-prefix
invariant — FIFO burial would have to be τ-decreasing. -/
theorem oracle_stuck_nonempty (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) {C : Nat} (hC : 1 ≤ C)
    {s : MState} (hm : MuxInv sk s)
    (hoI : OracleInv sk .I s) (hoR : OracleInv sk .R s)
    (hstuck : mstuck sk .impl C (oracle .I) (oracle .R) s = true)
    {q : Party} (hq : s.pipe q ≠ []) : False := by
  have hoP : ∀ p, OracleInv sk p s := fun p => by
    cases p
    · exact hoI
    · exact hoR
  have hL : InvL sk .impl s.base := hm.invl
  have hioh := mstuck_ioh (sk := sk) hstuck
  have hroh := mstuck_roh (sk := sk) hL hstuck
  have hwkh := mstuck_wkh hwf hL hstuck rfl
  -- the pool is nonempty: the head's slot predecessor is unperformed
  have hne : pends sk s.base ≠ [] := by
    cases hp : s.pipe q with
    | nil => exact absurd hp hq
    | cons c₀ rest =>
        obtain ⟨m, -, -, -, -, -, hrmem, hrnp, -⟩ :=
          head_kit hwf hm (hoP q) hstuck hp
        obtain ⟨fa₀, hfa₀, -⟩ :=
          Sched.pends_coverE sk hwf hm0 hL hioh hroh hwkh hrmem hrnp
        intro hnil
        rw [hnil] at hfa₀
        cases hfa₀
  -- the τ-least pool entry
  obtain ⟨fa, hfam, hfmin⟩ := Sched.exists_min_image
    (fun fa : Ev × Action => evIdx fa.1 (scheduleE sk)) hne
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
  have hcover : ∀ g ∈ scheduleE sk, ¬ performed sk s.base g →
      evIdx ((c, b, n) : Ev) (scheduleE sk) ≤ evIdx g (scheduleE sk) := by
    intro g hg hgnp
    obtain ⟨fa', hfam', hτle⟩ := Sched.pends_coverE sk hwf hm0 hL
      hioh hroh hwkh hg hgnp
    exact Nat.le_trans (hfmin fa' hfam') hτle
  have hfnp : ¬ performed sk s.base ((c, b, n) : Ev) :=
    Sched.pend_not_performedE sk hok
  -- an enabled non-fire pool action refutes stuckness
  have hkill : ∀ {a' : Action}, a' ∈ allActions sk →
      (∀ pk, a' ≠ .walkCloseWire pk) → a' ≠ .absorbCloseWire →
      isWireFire s.base a' = false →
      (Model.apply sk .impl a' s.base).isSome = true → False := by
    intro a' hmem hncw hnab hnf hsome
    have hbase : (applyBase sk .impl a' s).isSome = true := by
      rw [applyBase_isSome_of_not_close hnf hncw hnab]
      exact hsome
    have hen := mcanStep_of_base (C := C) (σI := oracle .I)
      (σR := oracle .R) hmem hbase
    have hno : mcanStep sk .impl C (oracle .I) (oracle .R) s = false := by
      rw [mstuck, Bool.and_eq_true, Bool.not_eq_true',
        Bool.not_eq_true'] at hstuck
      exact hstuck.2
    rw [hen] at hno
    cases hno
  obtain ⟨hncw, hnab⟩ := pends_not_close hfam
  cases b with
  | false =>
      -- a receive
      have hseq2 : n = recvdOf sk s.base c := by simpa using hok.seq
      by_cases hdata : 0 < s.base.chan c
      · -- data present: the receive fires, against stuckness
        have hsome := hok.fire (by simpa using hdata)
        have hnf : isWireFire s.base a = false := by
          cases hIF : isWireFire s.base a with
          | false => rfl
          | true =>
              obtain ⟨q₂, hh₂, -, hfb, -⟩ := pends_wireFire hfam hIF
              simp at hfb
        exact hkill hok.act hncw hnab hnf hsome
      · have hchan0 : s.base.chan c = 0 := by omega
        -- the send is performed: E1 against the cover
        have hE1 := Sched.scheduleE_e1 sk
          (evIdx ((c, false, n) : Ev) (scheduleE sk)) c n hτget
        have hsent_gt : n < sentOf sk s.base c := by
          by_contra hle
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
        by_cases hw : isWire c = true
        · -- a wire receive with its frame in flight: the head cycle
          obtain ⟨w, h_c, rfl⟩ := isWire_eq hw
          have hflow := hm.flow_wire w h_c hok.chan_mem
          have hcmem : Chan.wire w h_c ∈ s.pipe w := by
            refine List.one_le_count_iff.mp ?_
            have : 1 ≤ pipeCount s (Chan.wire w h_c) := by omega
            exact this
          cases hpw : s.pipe w with
          | nil => rw [hpw] at hcmem; cases hcmem
          | cons d₀ drest =>
              obtain ⟨m₀, hm₀eq, hchan₀, hD, hpos₀, he₀mem, hr₀mem,
                hr₀np, hτ₀⟩ := head_kit hwf hm (hoP w) hstuck hpw
              by_cases hd₀ : d₀ = Chan.wire w h_c
              · rw [hd₀] at hchan₀
                exact hchan₀ hchan0
              · -- the c-frame sits behind the head: τ(head) < τ(frame)
                obtain ⟨j, hjget, hjcnt, hjlt, hjpos⟩ :=
                  pipe_first_frame_pos hm (hoP w) hcmem
                rw [hpw] at hcmem hjget
                have hj1 : 1 ≤ j := by
                  by_contra hj0
                  have : j = 0 := by omega
                  subst this
                  have : some d₀ = some (Chan.wire w h_c) := by
                    rw [← hjget]
                    rfl
                  exact hd₀ (by injection this)
                have hframe : (sendProj sk w)[delTotal
                    (s.hist w.other) + j]?
                    = some (Chan.wire w h_c, true, n) := by
                  rw [hjpos, hchan0, hseq2]
                  simp
                have hτ1 : evIdx ((d₀, true, m₀) : Ev) (scheduleE sk)
                    < evIdx ((Chan.wire w h_c, true, n) : Ev)
                        (scheduleE sk) :=
                  sendProj_evIdx_lt hwf (by omega) hpos₀ hframe
                obtain ⟨-, hτE1⟩ := tau_e1 hwf hfsched
                have hτcov := hcover _ hr₀mem hr₀np
                omega
        · -- internal receive: conservation forces data present
          have hflow := hm.flow_int c hok.chan_mem (by simpa using hw)
          omega
  | true =>
      -- a send
      have hseq2 : n = sentOf sk s.base c := by simpa using hok.seq
      by_cases hroom : s.base.chan c < sk.cap c
      · cases hIF : isWireFire s.base a with
        | false =>
            exact hkill hok.act hncw hnab hIF
              (hok.fire (by simpa using hroom))
        | true =>
            obtain ⟨w, hh, hfc, -, hhold⟩ := pends_wireFire hfam hIF
            simp only at hfc
            subst hfc
            by_cases hproom : (s.pipe w).length < C
            · -- the withheld push: the oracle provably names it
              have hname := oracle_names hwf hm (hoP w) hfsched
                hseq2 hcover
              have hdecl := mstuck_withheld (sk := sk) hstuck
                ⟨hhold, hproom⟩
              cases w with
              | I => exact hdecl hname
              | R => exact hdecl hname
            · -- own pipe full: the head cycle against the unpushed f
              cases hpw : s.pipe w with
              | nil =>
                  rw [hpw] at hproom
                  simp at hproom
                  omega
              | cons d₀ drest =>
                  obtain ⟨m₀, hm₀eq, hchan₀, hD, hpos₀, he₀mem, hr₀mem,
                    hr₀np, hτ₀⟩ := head_kit hwf hm (hoP w) hstuck hpw
                  -- f is unpushed: its projection index is ≥ K > delTotal
                  obtain ⟨jf, hjf⟩ := List.mem_iff_getElem?.mp
                    (mem_sendProj hfsched)
                  have hjfK : (pushHeights (s.hist w)).length ≤ jf := by
                    by_contra hlt'
                    have htake : ((sendProj sk w).take
                        (pushHeights (s.hist w)).length)[jf]?
                        = some (Chan.wire w hh, true, n) := by
                      rw [List.getElem?_take_of_lt (by omega)]
                      exact hjf
                    have hmem' : ((Chan.wire w hh, true, n) : Ev)
                        ∈ evsOf w (pushHeights (s.hist w)) := by
                      rw [show evsOf w (pushHeights (s.hist w))
                          = (sendProj sk w).take
                              (pushHeights (s.hist w)).length
                          from hoP w]
                      exact List.mem_of_getElem? htake
                    obtain ⟨h', n', heq', hlt''⟩ := evsOf_mem_inv hmem'
                    have hinj : h' = hh ∧ n' = n := by
                      have h1 := congrArg (fun e : Ev => e.1) heq'
                      have h2 := congrArg (fun e : Ev => e.2.2) heq'
                      simp only at h1 h2
                      rw [Chan.wire.injEq] at h1
                      exact ⟨h1.2.symm, h2.symm⟩
                    obtain ⟨rfl, rfl⟩ := hinj
                    have hpc := hm.pushed_eq w h'
                      (scheduleE_wire_mem hwf hfsched)
                    unfold pushedCount at hpc
                    omega
                  have hτ2 : evIdx ((d₀, true, m₀) : Ev) (scheduleE sk)
                      < evIdx ((Chan.wire w hh, true, n) : Ev)
                          (scheduleE sk) :=
                    sendProj_evIdx_lt hwf (by omega) hpos₀ hjf
                  have hτcov := hcover _ hr₀mem hr₀np
                  omega
      · -- no channel room: E2 against the cover
        have hE2 := Sched.scheduleE_e2 sk
          (evIdx ((c, true, n) : Ev) (scheduleE sk)) c n hτget
        have hrcvlt : recvdOf sk s.base c < Sched.rcvCount c
            ((scheduleE sk).take
              (evIdx ((c, true, n) : Ev) (scheduleE sk))) := by
          by_cases hw : isWire c = true
          · obtain ⟨w, h_c, rfl⟩ := isWire_eq hw
            have hflow := hm.flow_wire w h_c hok.chan_mem
            have hcap : sk.cap (Chan.wire w h_c) = 1 := rfl
            omega
          · have hflow := hm.flow_int c hok.chan_mem (by simpa using hw)
            have hslot := hm.slot c hok.chan_mem
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

/-- No reachable-shaped state is stuck under the oracle pair: the
drained case routes through the landed chase (T2) plus the naming
lemma; the nonempty case is the head-cycle argmin. -/
theorem oracle_mstuck_eq_false (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) {C : Nat} (hC : 1 ≤ C)
    {s : MState} (hm : MuxInv sk s)
    (hoI : OracleInv sk .I s) (hoR : OracleInv sk .R s)
    (hnt : mterminal sk s = false) :
    mstuck sk .impl C (oracle .I) (oracle .R) s = false := by
  cases hstuck : mstuck sk .impl C (oracle .I) (oracle .R) s with
  | false => rfl
  | true =>
      exfalso
      by_cases hpI : s.pipe .I = []
      · by_cases hpR : s.pipe .R = []
        · -- both pipes drained: the chase names the withheld push
          obtain ⟨⟨cf, bf, nf⟩, a, p, hh, hfc, hfb, hfseq, hfsched, hfnp,
            hτmin, hτperf, hfam, hok, hhold⟩ :=
            chase hwf hm0 hm hpI hpR hstuck hnt
          simp only at hfc hfb hfseq
          subst hfc
          subst hfb
          have hname := oracle_names hwf hm
            (show OracleInv sk p s by cases p; exact hoI; exact hoR)
            hfsched hfseq hτmin
          have hroom : (s.pipe p).length < C := by
            have hempty : s.pipe p = [] := by
              cases p
              · exact hpI
              · exact hpR
            rw [hempty]
            simp only [List.length_nil]
            omega
          have hdecl := mstuck_withheld (sk := sk) hstuck ⟨hhold, hroom⟩
          cases p with
          | I => exact hdecl hname
          | R => exact hdecl hname
        · exact oracle_stuck_nonempty hwf hm0 hC hm hoI hoR hstuck
            (q := .R) hpR
      · exact oracle_stuck_nonempty hwf hm0 hC hm hoI hoR hstuck
          (q := .I) hpI

-- ============================================================== T5

/-- T5, `oracle_deadlock_free`, over the `MuxInv` ground facts as an
explicit reachability hypothesis (MUX-ADJUDICATION §3 T5, in the
state-feedback fallback form of record — see the module doc).

The hypothesis `hpres` is the MUX-ADJUDICATION §4 stage-F obligation
(`MuxInv` preservation along `MReachable`; `muxInv_init` is its landed
base case). Everything oracle-specific is discharged here
unconditionally: `OracleInv` preservation, the stuck-state argmin, and
the naming lemma. Capacity is denominated in messages (= scope
replies); the §5A W = 1 byte-soundness caveat applies verbatim
(MUX-ADJUDICATION §2.5). -/
theorem oracle_deadlock_free_of_muxInv (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) (C : Nat) (hC : 1 ≤ C)
    (hpres : ∀ s, MReachable sk .impl C (oracle .I) (oracle .R) s →
      MuxInv sk s) :
    MuxDeadlockFree sk .impl C (oracle .I) (oracle .R) := by
  intro s hr
  cases hterm : mterminal sk s with
  | true =>
      rw [mstuck, hterm]
      rfl
  | false =>
      obtain ⟨hoI, hoR⟩ := oracleInv_reachable hwf hr
      exact oracle_mstuck_eq_false hwf hm0 hC (hpres s hr) hoI hoR hterm

end StreamingMirror.Mux
