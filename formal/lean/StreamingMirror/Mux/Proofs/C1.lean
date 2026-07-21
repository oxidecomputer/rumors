/-
The C1 verdicts (MUX-ADJUDICATION.md §3 T4 companions): the literal
conjecture minted faithfully, its refutation shapes, and the control
pinning why the Inevitable closure is load-bearing.

# The two statements and the locality gap

`C1Statement` is MUX-PROGRESS §1's conjecture verbatim in the artifact's
vocabulary: for every capacity and every pair of deterministic
LOCAL-information-only strategies there is a killer skeleton in the
shipping encoder's class ("schedulable" read per the adjudication's
domain ruling: wellFormed + margin-0, whose un-muxed session
`Sched.deadlock_free` completes). `C1StatementOmniscient` widens the
pair quantifier to ALL strategies.

The omniscient form is refuted OUTRIGHT by ⟨1, σ*, σ*⟩ + T4
(`c1_omniscient_false`). The literal form's refutation additionally
needs σ*'s locality, and THIS σ* — built over the full-skeleton
`inevitable` closure and `scheduleE` τ order, per the stage-3 charter —
is not proven `LocalStrategy`: `wireHeights`, `committedInHist`, and
`rootH` are `LocalEq`-invariant by construction, but the closure and
the τ order read peer-side structure (`procsE`), and showing their
verdicts agree across `LocalEq` pairs on `Consistent` traces is exactly
the A_p-sufficiency theorem — refute-c1 §2.4's "coverage of A_p"
quantified over every reachable observation rather than only stuck
states. The stage-0 probe carries that fact at the checked tier (the
A_p-limited causal σ* is terminal on 4,970/4,970 runs, STAGE0-GATES.md
P1); the kernel form is recorded [open], and `c1_literal_false` names
the residue as an explicit hypothesis instead of forcing it.

# Controls

- `wedge_sigmaStar_deadlock_free`: σ* completes the impossibility
  witness itself — the same skeleton every work-conserving pair jams —
  by pure instantiation of T4 (no decide).
- `smokeChain_sigmaStar_completes` [decide]: the σ*-driven drain
  reaches `mterminal` in the kernel, so the strategy's executable
  spine (party inference, ledger reconstruction, closure, τ argmin) is
  pinned end to end.
- `wedge_evidence_starves` [decide] + its `¬ MuxDeadlockFree`
  corollary: the evidence-only variant (push evidence, no forward
  derivation) wedges on `wedge`'s provision wall — with `certified`
  grounding sends only, no later-than-first frame is ever
  proven-demanded, which is refute-c1 §5's boundary observation: the
  Inevitable closure is what C1's falsity rests on, not evidence.

# T8 stub (NOT attempted; the intended statement, for its builder)

The K-deep parking generalization needs per-direction depths: the
single-socket design (design/single-socket.md) advertises windows per
direction, so the two parties may run different parking bounds
(K_I ≠ K_R), each direction's demux parking exactly what its sender
was told. The intended theorem, over a K-slot demux variant semantics
`applyK` whose `deliver p` parks up to `K_p` frames per stream and a
lookahead strategy `sigmaStarK K` demanding `rcv(c, k−K)`:

  theorem sigmaStarK_deadlock_free (KI KR : Nat)
      (hKI : 1 ≤ KI) (hKR : 1 ≤ KR)
      (hwf : sk.wellFormed = true)
      (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) (C : Nat) (hC : 1 ≤ C) :
      MuxDeadlockFreeK sk .impl C KI KR (sigmaStarK KI) (sigmaStarK KR)

A single-K statement would not cover the deployed configuration.
-/
import StreamingMirror.Mux.Proofs.SigmaStarLive
import StreamingMirror.Mux.Controls

namespace StreamingMirror.Mux

open Model

-- ======================================================= the statements

/-- C1 as literally chartered (MUX-PROGRESS §1): every capacity and
every pair of deterministic local strategies has a killer skeleton in
the shipping encoder's class. Determinism is free (`Strategy` is a
function); locality is `LocalStrategy`; the domain is the adjudicated
`.impl`+margin-0 reading of "schedulable". -/
def C1Statement : Prop :=
  ∀ (C : Nat), 1 ≤ C → ∀ (σI σR : Strategy),
    LocalStrategy .I σI → LocalStrategy .R σR →
    ∃ sk : Skel, sk.wellFormed = true
      ∧ (∀ sc, sk.dCount sc ≤ sk.capLevel)
      ∧ ¬ MuxDeadlockFree sk .impl C σI σR

/-- C1 with the pair quantifier widened to every strategy, local or
not — the form whose falsity needs no locality argument at all. -/
def C1StatementOmniscient : Prop :=
  ∀ (C : Nat), 1 ≤ C → ∀ (σI σR : Strategy),
    ∃ sk : Skel, sk.wellFormed = true
      ∧ (∀ sc, sk.dCount sc ≤ sk.capLevel)
      ∧ ¬ MuxDeadlockFree sk .impl C σI σR

-- ====================================================== the refutations

/-- The wide form of C1 is false: ⟨1, σ*, σ*⟩ + T4. No skeleton of the
class jams the demand-lockstep pair at any capacity — already at the
minimum pipe. -/
theorem c1_omniscient_false : ¬ C1StatementOmniscient := by
  intro hc1
  obtain ⟨sk, hwf, hm0, hnd⟩ := hc1 1 (Nat.le_refl 1) sigmaStar sigmaStar
  exact hnd (sigmaStar_deadlock_free hwf hm0 1 (Nat.le_refl 1))

/-- C1 as literally chartered is false, GIVEN σ*'s locality — the one
hypothesis this artifact does not discharge (module doc: it is the
A_p-sufficiency theorem, probe-checked at 4,970/4,970 and recorded
[open] at kernel tier). Every other ingredient is kernel-proven. -/
theorem c1_literal_false
    (hlocI : LocalStrategy .I sigmaStar)
    (hlocR : LocalStrategy .R sigmaStar) : ¬ C1Statement := by
  intro hc1
  obtain ⟨sk, hwf, hm0, hnd⟩ :=
    hc1 1 (Nat.le_refl 1) sigmaStar sigmaStar hlocI hlocR
  exact hnd (sigmaStar_deadlock_free hwf hm0 1 (Nat.le_refl 1))

-- ========================================================= the controls

/-- σ* completes the impossibility witness: the skeleton that jams
every work-conserving pair (`wc_impossibility`) is live under
demand-lockstep at the minimum capacity — T4 instantiated, no drain
needed. With `Control.wedge_not_deadlockFree` this pins the trichotomy
on one skeleton: the class hypothesis, not the topology, is what
deadlocks. -/
theorem wedge_sigmaStar_deadlock_free :
    MuxDeadlockFree wedge .impl 1 sigmaStar sigmaStar :=
  sigmaStar_deadlock_free wedge_wellFormed wedge_margin0 1 (Nat.le_refl 1)

set_option maxRecDepth 1000000 in
/-- The σ*-driven drain completes the smoke pin in the kernel: party
inference, ledger reconstruction, the demand closure, and the τ argmin
all execute end to end — the strategy is a real scheduler, not only a
proof object. -/
theorem smokeChain_sigmaStar_completes :
    muxCompletes Pin.smokeChain .impl 1 sigmaStar sigmaStar 400
      = true := by
  decide

set_option maxRecDepth 1000000 in
/-- The evidence-only variant starves on the wedge's provision wall:
with `certified` grounding sends only, no frame past a stream's first
is ever proven-demanded, and the greedy drain parks in `mstuck`
(refute-c1 §5: the forward derivation, not evidence, is what refutes
C1). -/
theorem wedge_evidence_starves :
    mstuck wedge .impl 1 sigmaEvidence sigmaEvidence
      (mdrain wedge .impl 1 sigmaEvidence sigmaEvidence 800
        (init wedge)) = true := by
  decide

/-- The starvation, lifted to the liveness claim: the evidence-only
strategy pair is NOT deadlock-free where σ* is
(`wedge_sigmaStar_deadlock_free`) — the Inevitable closure is
load-bearing. -/
theorem wedge_evidence_not_deadlockFree :
    ¬ MuxDeadlockFree wedge .impl 1 sigmaEvidence sigmaEvidence := by
  intro h
  have hs := wedge_evidence_starves
  have hr := mdrain_reachable wedge .impl 1 sigmaEvidence sigmaEvidence
    800 (.init)
  rw [h _ hr] at hs
  exact Bool.false_ne_true hs

end StreamingMirror.Mux
