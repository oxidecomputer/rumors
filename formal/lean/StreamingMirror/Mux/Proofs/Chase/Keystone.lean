/-
T2, the repaired Keystone Lemma (MUX-ADJUDICATION.md §3 T2; the F1
route of attack-refute, formalized): at a stuck muxed state, every
event a machine's demand closure derived at push time has been
performed.

# The push-time derivation tree, as hypotheses

The adjudicated repair runs the induction over the PUSH-TIME derivation
tree, with FIFO ancestry discharging the forward-delivery citations.
Here the tree is the closure `inevitable sk p tr` computed at the
push-time observation `tr`, and its two grounding walls arrive as count
hypotheses:

- `hfifo`: every own-push the closure could cite at `tr` is delivered
  at `s` — the FIFO-ancestry fact. Its canonical discharge is
  `MuxInv.pushtime_delivered`: when the frame pushed at time `tr` is
  the pipe head at `s`, everything before it has drained.
- `harr`: arrivals only accumulate — `tr` is an earlier observation of
  the same machine, so its delivery counts bound today's.

Stating the tree this way removes every delivery event from the
induction: the broken delivery case of the original lemma
(attack-refute F1) cannot even be written, and what remains is exactly
the panel's repaired remainder — non-push events discharged by the
counting/enabledness argument.

# The induction, one paragraph

Take the τ-least unperformed member `e` of the closure (τ = position
in `scheduleE`, total and injective on the event set by
`merge_completeE` + `scheduleE_inj`, consumed as black boxes). Grounded
members are performed by count arithmetic, so `e` entered by I-step.
Decode `e`'s trace: if `e` sits strictly above the frontier, the
frontier itself is in `e`'s I-step prefix — closure member, τ-below,
unperformed — contradicting minimality; so `e` IS the frontier, its
seq is the live count (`PendOkE.seq`), its guard opens by counting
(E1/E2 predecessors are performed by minimality; the wire case runs
through `hfifo`/`harr` and the slot equation), and its action — a
non-push, non-close channel operation — is muxed-enabled, contradicting
stuckness.
-/
import StreamingMirror.Mux.Proofs.Chase.Decode

namespace StreamingMirror.Mux

open Model
open Sched (Ev procsE scheduleE performed pends PendOkE evIdx)

/-- The two endpoints are each other's `other`. -/
theorem _root_.StreamingMirror.Party.other_other (p : Party) :
    p.other.other = p := by
  cases p <;> rfl

variable {sk : Skel}

/-- T2, the repaired keystone: at a reachable stuck state of the muxed
system, every event of the push-time demand closure is performed.

`tr` is the deriving machine's observation history at push time (or any
other point of the run); `hfifo` is the FIFO-ancestry wall — every
own-push `tr` can cite has been delivered by `s` — and `harr` is
arrival monotonicity. The ground facts `MuxInv` stand in for muxed
reachability (their preservation is the stage-3 obligation); given
them, no reachability induction appears — the proof is one argmin over
`scheduleE` positions, exactly the adjudicated plan. -/
theorem keystone (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel)
    {C : Nat} {σI σR : Strategy} {s : MState}
    (hm : MuxInv sk s)
    (hstuck : mstuck sk .impl C σI σR s = true)
    (p : Party) (tr : List MObs)
    (hfifo : ∀ h, pushedCount tr h ≤ deliveredCount (s.hist p.other) h)
    (harr : ∀ h, deliveredCount tr h ≤ deliveredCount (s.hist p) h) :
    ∀ e ∈ inevitable sk p tr, performed sk s.base e := by
  -- grounded members are performed outright: the count walls meet the
  -- slot equation and wire flow conservation
  have hground_perf : ∀ x, groundedPush p tr x = true →
      performed sk s.base x := by
    intro x hg
    obtain ⟨q, hh, n, rfl, hcase⟩ := groundedPush_inv hg
    rw [performed_snd_iff]
    by_cases hreal : Chan.wire q hh ∈ allChans sk
    · rcases hcase with ⟨rfl, hlt⟩ | ⟨rfl, hlt⟩
      · have h1 := hfifo hh
        have h2 := hm.delivered_eq q hh hreal
        have h3 := hm.flow_wire q hh hreal
        omega
      · have h1 := harr hh
        have h2 := hm.delivered_eq p.other hh hreal
        rw [Party.other_other] at h2
        have h3 := hm.flow_wire p.other hh hreal
        omega
    · -- phantom stream: the evidence walls are zero, the counts vacuous
      exfalso
      rcases hcase with ⟨rfl, hlt⟩ | ⟨rfl, hlt⟩
      · have h1 := hfifo hh
        have h2 := hm.delivered_real q hh hreal
        omega
      · have h1 := harr hh
        have h2 := hm.delivered_real p.other hh hreal
        rw [Party.other_other] at h2
        omega
  intro e₀ he₀
  by_contra hnp₀
  -- the τ-least unperformed closure member
  have hne : (inevitable sk p tr).filter
      (fun x => !decide (performed sk s.base x)) ≠ [] := by
    intro hnil
    have hmem : e₀ ∈ (inevitable sk p tr).filter
        (fun x => !decide (performed sk s.base x)) :=
      List.mem_filter.mpr ⟨he₀, by simp [hnp₀]⟩
    rw [hnil] at hmem
    cases hmem
  obtain ⟨e, heU, hmin⟩ :=
    Sched.exists_min_image (fun x => evIdx x (scheduleE sk)) hne
  obtain ⟨heI, hnpb⟩ := List.mem_filter.mp heU
  have hnp : ¬ performed sk s.base e := by simpa using hnpb
  have hperf_lt : ∀ x ∈ inevitable sk p tr,
      evIdx x (scheduleE sk) < evIdx e (scheduleE sk) →
      performed sk s.base x := by
    intro x hx hlt
    by_cases hperf : performed sk s.base x
    · exact hperf
    · have := hmin x (List.mem_filter.mpr ⟨hx, by simp [hperf]⟩)
      omega
  -- how did e enter the closure?
  rcases inevitable_inv heI with hg | hstep
  · exact hnp (hground_perf e hg)
  -- I-step member: decode its trace
  obtain ⟨T, hT, heT⟩ := mem_evUniv.mp (inevitable_subset_univ e heI)
  have hL : InvL sk .impl s.base := hm.invl
  have hioh := mstuck_ioh (sk := sk) hstuck
  have hroh := mstuck_roh (sk := sk) hL hstuck
  have hwkh := mstuck_wkh hwf hL hstuck rfl
  rcases trace_frontier hwf hL hioh hroh hwkh hT with hall | hfr
  · exact hnp (hall e heT)
  obtain ⟨f, a, pre, suf, hfa, hdec, hpre, hok⟩ := hfr
  have heT' := heT
  rw [hdec] at heT'
  rcases List.mem_append.1 heT' with hepre | hecons
  · exact hnp (hpre e hepre)
  rcases List.mem_cons.1 hecons with heqf | hesuf
  case inr =>
    -- e sits above the frontier: the frontier is in e's I-step prefix,
    -- unperformed and τ-below — against minimality
    have hfmem : f ∈ T.takeWhile (fun x => !(x == e)) :=
      frontier_mem_takeWhile hdec (trace_count_le_one hT e) hesuf
    have hfI : f ∈ inevitable sk p tr :=
      istepOk_prefix hstep hT heT f hfmem
    have hfnp : ¬ performed sk s.base f :=
      Sched.pend_not_performedE sk hok
    have hτ : evIdx f (scheduleE sk) < evIdx e (scheduleE sk) := by
      refine tau_lt_of_trace_pair hwf hm0 hT ?_
      rw [hdec]
      refine List.Sublist.trans ?_ (List.sublist_append_right ..)
      exact List.cons_sublist_cons.2 (List.singleton_sublist.2 hesuf)
    have := hmin f (List.mem_filter.mpr ⟨hfI, by simp [hfnp]⟩)
    omega
  case inl =>
    -- e IS the frontier: open its guard and fire, against stuckness
    subst heqf
    -- the uniform ending: a non-fire pool action enabled at the base
    -- state is muxed-enabled
    have hkill : isWireFire s.base a = false →
        (Model.apply sk .impl a s.base).isSome = true → False := by
      intro hnf hsome
      obtain ⟨hncw, hnab⟩ := pends_not_close hfa
      have hbase : (applyBase sk .impl a s).isSome = true := by
        rw [applyBase_isSome_of_not_close hnf hncw hnab]
        exact hsome
      have hen := mcanStep_of_base (C := C) (σI := σI) (σR := σR)
        hok.act hbase
      have hno : mcanStep sk .impl C σI σR s = false := by
        rw [mstuck, Bool.and_eq_true, Bool.not_eq_true',
          Bool.not_eq_true'] at hstuck
        exact hstuck.2
      rw [hen] at hno
      cases hno
    obtain ⟨c, b, n⟩ := e
    cases b with
    | false =>
        -- a receive: data is present
        have hseq : n = recvdOf sk s.base c := by simpa using hok.seq
        have hpred : ((c, true, n) : Ev) ∈ inevitable sk p tr :=
          istepOk_e1 hstep rfl
        have hguard : 0 < s.base.chan c := by
          by_cases hw : isWire c = true
          · -- wire receive: the send is grounded (pushes are never
            -- I-stepped), and grounded sends are DELIVERED, not merely
            -- pushed — the F1 crux
            obtain ⟨q, hh, rfl⟩ := isWire_eq hw
            have hpg : groundedPush p tr (Chan.wire q hh, true, n)
                = true := by
              rcases inevitable_inv hpred with hg | hst
              · exact hg
              · have h2 := istepOk_not_push hst
                simp [isWire] at h2
            obtain ⟨q', hh', n', heq, hcase⟩ := groundedPush_inv hpg
            simp only [Prod.mk.injEq, Chan.wire.injEq, true_and] at heq
            obtain ⟨⟨rfl, rfl⟩, rfl⟩ := heq
            by_cases hreal : Chan.wire q hh ∈ allChans sk
            · rcases hcase with ⟨rfl, hlt⟩ | ⟨rfl, hlt⟩
              · have h1 := hfifo hh
                have h2 := hm.delivered_eq q hh hreal
                omega
              · have h1 := harr hh
                have h2 := hm.delivered_eq p.other hh hreal
                rw [Party.other_other] at h2
                omega
            · exfalso
              rcases hcase with ⟨rfl, hlt⟩ | ⟨rfl, hlt⟩
              · have h1 := hfifo hh
                have h2 := hm.delivered_real q hh hreal
                omega
              · have h1 := harr hh
                have h2 := hm.delivered_real p.other hh hreal
                rw [Party.other_other] at h2
                omega
          · -- internal receive: the send is τ-below, hence performed,
            -- and internal channels never ride the pipe
            have hmem_e : ((c, false, n) : Ev) ∈ scheduleE sk :=
              inevitable_mem_scheduleE hwf hm0 heI
            obtain ⟨-, hτlt⟩ := tau_e1 hwf hmem_e
            have hpp := hperf_lt _ hpred hτlt
            rw [performed_snd_iff] at hpp
            have hflow := hm.flow_int c hok.chan_mem
              (by simpa using hw)
            omega
        exact hkill (by
          cases hIF : isWireFire s.base a with
          | false => rfl
          | true =>
              obtain ⟨q₂, hh₂, -, hfb, -⟩ := pends_wireFire hfa hIF
              simp at hfb)
          (hok.fire (by simpa using hguard))
    | true =>
        -- a send: never a push (I-step), so an internal channel with
        -- an open cap window
        have hw : isWire c = false := by
          have := istepOk_not_push hstep
          simpa using this
        have hseq : n = sentOf sk s.base c := by simpa using hok.seq
        have hflow := hm.flow_int c hok.chan_mem hw
        have hguard : s.base.chan c < sk.cap c := by
          by_cases hcap : n < sk.cap c
          · omega
          · have hpred : ((c, false, n - sk.cap c) : Ev)
                ∈ inevitable sk p tr := istepOk_e2 hstep rfl hcap
            have hmem_e : ((c, true, n) : Ev) ∈ scheduleE sk :=
              inevitable_mem_scheduleE hwf hm0 heI
            obtain ⟨-, hτlt⟩ := tau_e2 hwf hmem_e (by omega)
            have hpp := hperf_lt _ hpred hτlt
            rw [performed_rcv_iff] at hpp
            omega
        exact hkill (by
          cases hIF : isWireFire s.base a with
          | false => rfl
          | true =>
              obtain ⟨q₂, hh₂, hfc, -, -⟩ := pends_wireFire hfa hIF
              simp only at hfc
              rw [hfc] at hw
              simp [isWire] at hw)
          (hok.fire (by simpa using hguard))

end StreamingMirror.Mux
