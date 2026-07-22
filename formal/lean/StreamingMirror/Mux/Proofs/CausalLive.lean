/-
σ*-causal liveness (MUX-PROGRESS §4, the residue's liveness half):
refute-c1 §2 assembled at the causal grain — Steps 1–3 (pipes drain
through the causal keystone; the chase's withheld push) proven here in
the kernel, with Step 4 (the τ-staged coverage of the ANNOUNCED
closure) isolated as `CausalStuckCoverage`, stated exactly at the
point the assembly consumes it and DISCHARGED in
Proofs/CausalMint.lean (`causalStuckCoverage`).

# The interface

`sigmaStarCausal_deadlock_free_of_coverage` needs only the Step-4
conjunct: at a reachable σ*-causal-stuck state with both pipes drained
(drained-ness is PROVEN, not assumed — `sigmaStarCausal_pipes_empty`),
the chase's withheld push is `demandedA` given that everything τ-below
it is performed. Everything else — the push certificates
(`pushProvenA_reachable`), the receive ledger, the causal keystone,
Steps 2–3's chase, and the final "σ*-causal names a push" inversion —
lives here.

`CausalStuckCoverage` is precisely T8's "inference progress" conjunct
(the window-sliding argument needs the same fact): its content is the
minting lemma — every consulted record's minting arrival sits τ-below
the consulting event, so the announced layouts extend past every
performed event — composed with the τ-staged closure induction of
SigmaStarLive's `closure_coverage` re-run over `inevitableA`. The
trace-grammar half is landed in CausalCoverage.lean
(`announcedProcs_prefix`); the minting ladder and coverage induction
in CausalMint.lean; the unconditional composition
`sigmaStarCausal_deadlock_free` sits at CausalMint.lean's foot. The
wedge pin (`wedge_sigmaStarCausal_completes`) remains as the
executable anchor of the same fact.
-/
import StreamingMirror.Mux.Proofs.CausalCoverage

namespace StreamingMirror.Mux

open Model
open Sched (Ev performed pends PendOkE evIdx scheduleE)

variable {sk : Skel}

/-- Step 4 at the causal grain, as a named hypothesis: at a reachable
σ*-causal-stuck state with pipes drained, a history-held stream whose
τ-prefix below the next frame's send is entirely performed is
proven-demanded under the ANNOUNCED closure.

This is the coverage re-run of refute-c1 §2.4 over `inevitableA`
(Mux/Causal.lean's module doc): the chase witness receive is τ-below
the withheld send, hence performed, hence announced-laid — the minting
lemma — and then enters the causal closure by its own τ stage.
Discharged by `causalStuckCoverage` (Proofs/CausalMint.lean); kept as
a named Prop because the assembly below and T8's window-sliding both
consume it at exactly this interface. -/
def CausalStuckCoverage (sk : Skel) : Prop :=
  ∀ (C : Nat) (s : MState),
    MReachable sk .impl C sigmaStarCausal sigmaStarCausal s →
    mstuck sk .impl C sigmaStarCausal sigmaStarCausal s = true →
    s.pipe .I = [] → s.pipe .R = [] →
    ∀ p hh, holdsWire sk p hh s.base = true →
      (∀ g ∈ scheduleE sk,
        evIdx g (scheduleE sk)
          < evIdx ((Chan.wire p hh, true,
              sentOf sk s.base (Chan.wire p hh)) : Ev) (scheduleE sk) →
        performed sk s.base g) →
      demandedA (aviewOf sk p (s.hist p)) (s.hist p) hh = true

/-- σ*-causal never idles on a demanded held stream: whenever the
machine has identified itself and some held stream is proven-demanded
under the announced closure, a push is named. -/
theorem sigmaStarCausal_isSome {tr : List MObs} {p : Party} {h : Nat}
    (hp : partyOf tr = some p)
    (hmem : h ∈ wireHeights sk p)
    (hcm : committedInHist sk.rootH tr h = true)
    (hdem : demandedA (aviewOf sk p tr) tr h = true) :
    (sigmaStarCausal sk tr).isSome = true := by
  rw [sigmaStarCausal, hp]
  show (causalCore (aviewOf sk p tr) tr).isSome = true
  rw [causalCore]
  rw [List.find?_isSome]
  refine ⟨h, ?_, ?_⟩
  · rw [wireHeightsA_aviewOf]
    exact hmem
  · rw [Bool.and_eq_true]
    exact ⟨hcm, hdem⟩

/-- σ*-causal is deadlock-free given the Step-4 coverage conjunct: the
demand-lockstep-over-announced-views pair completes every well-formed
margin-0 session at every capacity C ≥ 1.

Steps 1–3 are unconditional: the causal push certificates drain the
pipes at any stuck candidate through the causal keystone, and the
chase exhibits the τ-least withheld push with everything τ-below it
performed. `hcov` supplies exactly Step 4 — the withheld frame is
proven-demanded under the announced closure — and σ*-causal then names
a push, refuting stuckness. -/
theorem sigmaStarCausal_deadlock_free_of_coverage
    (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel)
    (hcov : CausalStuckCoverage sk)
    (C : Nat) (hC : 1 ≤ C) :
    MuxDeadlockFree sk .impl C sigmaStarCausal sigmaStarCausal := by
  intro s hr
  cases hst : mstuck sk .impl C sigmaStarCausal sigmaStarCausal s with
  | false => rfl
  | true =>
      exfalso
      have hm := sinv_reachable hwf hr
      have hpp := pushProvenA_reachable hwf hr
      have hrl := recvLedger_reachable hwf hr
      -- Step 1: the pipes drain
      have hpI : s.pipe .I = [] :=
        sigmaStarCausal_pipes_empty hwf hm0 hm hrl hpp hst .I
      have hpR : s.pipe .R = [] :=
        sigmaStarCausal_pipes_empty hwf hm0 hm hrl hpp hst .R
      have hnt : mterminal sk s = false := by
        rw [mstuck, Bool.and_eq_true, Bool.not_eq_true'] at hst
        exact hst.1
      -- Steps 2–3: the τ-least withheld push, everything below performed
      obtain ⟨f, a, p, hh, hfc, hfb, hfseq, hfsched, hfnp, hleast,
        hcover, hpend, hok, hhold⟩ :=
        chase hwf hm0 hm.mux hpI hpR hst hnt
      obtain ⟨c', b', n'⟩ := f
      simp only at hfc hfb hfseq
      subst hfc
      subst hfb
      subst hfseq
      have hhold' : holdsWire sk p hh s.base = true := by
        rw [holdsWire.eq_def] at hhold ⊢
        exact hhold
      have hcm : committedInHist sk.rootH (s.hist p) hh = true := by
        rw [committedInHist_iff_holdsWire hm.hist]
        exact hhold'
      -- Step 4: the coverage conjunct proves the frame demanded
      have hdem : demandedA (aviewOf sk p (s.hist p)) (s.hist p) hh
          = true :=
        hcov C s hr hst hpI hpR p hh hhold' hcover
      -- σ*-causal therefore names a push — against stuckness
      have hcz : commitsOf sk.rootH (s.hist p) hh ≠ 0 := by
        rw [committedInHist_eq, decide_eq_true_eq] at hcm
        omega
      obtain ⟨q, hq0⟩ := Option.isSome_iff_exists.mp
        (partyOf_isSome_of_commits hcz)
      have hqp : q = p := partyOf_eq hm.hist hq0
      have hq : partyOf (s.hist p) = some p := by
        rw [hqp] at hq0
        exact hq0
      have hsome := sigmaStarCausal_isSome hq
        (holdsWire_mem_wireHeights hhold') hcm hdem
      obtain ⟨h', hσ⟩ := Option.isSome_iff_exists.mp hsome
      obtain ⟨q', hq', hcm', -⟩ := sigmaStarCausal_some_inv hσ
      have hwp : WithheldPush sk C p h' s := by
        refine ⟨?_, ?_⟩
        · rw [← committedInHist_iff_holdsWire hm.hist]
          exact hcm'
        · have hempty : s.pipe p = [] := by
            cases p
            · exact hpI
            · exact hpR
          rw [hempty]
          simp only [List.length_nil]
          omega
      have hout := mstuck_withheld hst hwp
      apply hout
      cases p <;> exact hσ

end StreamingMirror.Mux
