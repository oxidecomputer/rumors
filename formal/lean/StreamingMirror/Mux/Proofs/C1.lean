/-
The C1 verdicts (MUX-ADJUDICATION.md §3 T4 companions): the literal
conjecture at its two locality grains, its refutation shapes, and the
control pinning why the Inevitable closure is load-bearing.

# The grains, and which statement is of record (Finch's F3 ruling)

Phase 4's F3 found the legacy `LocalEq`/`LocalStrategy` grain
(Mux/Strategy.lean) FINER than session-start indistinguishability:
`viewEnc` encodes peer-determined merge labels (D vs R-cut vs M-absent)
of a party's own held children, which no party knows from its own tree.
Finch's ruling: theorem statements of record must bind the charter's
intended grain — information in the causal past of the party at the
decision point. In this model that grain is `CharterLocal`
(Mux/Causal.lean): invariance across skeletons with equal ANNOUNCED
VIEWS (the stage-0 probe's `KnownSkel`: session parameters at t = 0,
plus the records the arrived frames have determined, at
`.impl`-realizable observations). `C1StatementCharter` below is
therefore THE literal conjecture of record; `C1Statement` (the legacy
grain) is retained as an internal artifact for its landed consumers.

NOTE the two grains are INCOMPARABLE, not nested — the a-fortiori
transfer fails in BOTH directions. `LocalEq` pairs may differ in
announced content (answerer-side R children and `leafReqs` of announced
scopes are `viewEnc`-erased yet frame-announced), so a charter-local
strategy need not be legacy-local; announced-view pairs may differ in
unannounced view structure, so a legacy-local strategy need not be
charter-local. Each statement carries its own refutation witness.

# The refutation ledger

- `C1StatementOmniscient` (no locality hypothesis at all): refuted
  OUTRIGHT by ⟨1, σ*, σ*⟩ + T4 (`c1_omniscient_false`), unconditional.
- `C1StatementCharter` (the statement of record): refuted
  UNCONDITIONALLY by ⟨1, σ*-causal, σ*-causal⟩ (`c1_charter_false`):
  `sigmaStarCausal_charterLocal` is kernel-proven, and the liveness
  half is now kernel-proven end to end — Steps 1–3 in
  Proofs/CausalCoverage.lean and Proofs/CausalLive.lean, and Step 4's
  `CausalStuckCoverage` discharged by `causalStuckCoverage`
  (Proofs/CausalMint.lean: the minting ladder — every consulted
  record's minting frame send sits τ-below the consulting event, so
  the drained pipes turn the τ-wall into announced records — composed
  with the causal coverage induction and the closure's saturation).
  The theorem is exactly T8's "inference progress" conjunct, now
  available to the window-sliding argument as a lemma. The probe and
  wedge anchors (4,970/4,970 terminal causal runs, STAGE0-GATES.md P1;
  `wedge_sigmaStarCausal_completes`) stand behind it as executable
  witnesses rather than as the claim's support.
- `C1Statement` (legacy grain, internal): refutation
  `c1_literal_false`, carrying σ*'s legacy locality as its named
  hypothesis exactly as landed at stage 3 — `wireHeights`,
  `committedInHist`, and `rootH` are `LocalEq`-invariant by
  construction, but the omniscient closure and τ order read peer-side
  structure (`procsE`), and their `LocalEq`-invariance on `Consistent`
  traces is the A_p-sufficiency statement, [open] at kernel tier.

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

# T8 stub (positive half NOT attempted; corrected against the landed K-harness)

The K-deep parking generalization needs per-direction depths: the
single-socket design (design/single-socket.md) advertises windows per
direction, so the two parties may run different parking bounds
(K_I ≠ K_R). The K-harness LANDED with the impossibility half
(`applyK`/`MuxDeadlockFreeK`/`wc_impossibility_K`,
WcImpossibilityK.lean); a builder of the positive half must match its
conventions, which this stub originally got backwards on two counts:

- argument order is `MuxDeadlockFreeK sk ax KI KR C σI σR`
  (depths BEFORE capacity, matching `applyK`);
- the deliver dial is the RECEIVING party's advertised depth:
  `deliver .I` fills the responder's cells at depth `KR`,
  `deliver .R` the initiator's at depth `KI` (`recvDepth`) — not
  "deliver p parks up to K_p".

The intended positive theorem, over a lookahead strategy
`sigmaStarK K` demanding `rcv(c, k−K)`:

  theorem sigmaStarK_deadlock_free (KI KR : Nat)
      (hKI : 1 ≤ KI) (hKR : 1 ≤ KR)
      (hwf : sk.wellFormed = true)
      (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) (C : Nat) (hC : 1 ≤ C) :
      MuxDeadlockFreeK sk .impl KI KR C (sigmaStarK KR) (sigmaStarK KI)

(each party's lookahead is the depth its PEER's demux parks — the
depth its own sends were advertised). A single-K statement would not
cover the deployed configuration.
-/
import StreamingMirror.Mux.Proofs.SigmaStarLive
import StreamingMirror.Mux.Proofs.CausalMint
import StreamingMirror.Mux.Controls

namespace StreamingMirror.Mux

open Model

-- ======================================================= the statements

/-- C1 at the grain of record (Finch's F3 ruling): every capacity and
every pair of deterministic CHARTER-LOCAL strategies — invariant across
skeletons with equal announced views at `.impl`-realizable observations
(`CharterLocal`, Mux/Causal.lean) — has a killer skeleton in the
shipping encoder's class. Determinism is free (`Strategy` is a
function); the domain is the adjudicated `.impl`+margin-0 reading of
"schedulable". -/
def C1StatementCharter : Prop :=
  ∀ (C : Nat), 1 ≤ C → ∀ (σI σR : Strategy),
    CharterLocal .I σI → CharterLocal .R σR →
    ∃ sk : Skel, sk.wellFormed = true
      ∧ (∀ sc, sk.dCount sc ≤ sk.capLevel)
      ∧ ¬ MuxDeadlockFree sk .impl C σI σR

/-- C1 at the LEGACY grain (internal artifact — the module doc's grain
note): the strategy class is `LocalStrategy` over `viewEnc`/`LocalEq`,
which phase 4's F3 found finer than session-start honesty. Kept because
its landed refutation (`c1_literal_false`) and the stage-3 record cite
it; the statement of record is `C1StatementCharter`. -/
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
minimum pipe.

Rests on σ*'s (message-denominated) liveness; the W = 1 byte caveat of
record is Mux/Basic.lean's module doc (# The byte-denomination
caveat). -/
theorem c1_omniscient_false : ¬ C1StatementOmniscient := by
  intro hc1
  obtain ⟨sk, hwf, hm0, hnd⟩ := hc1 1 (Nat.le_refl 1) sigmaStar sigmaStar
  exact hnd (sigmaStar_deadlock_free hwf hm0 1 (Nat.le_refl 1))

/-- The statement of record is false, UNCONDITIONALLY: locality is
kernel-proven (`sigmaStarCausal_charterLocal`), and the liveness half
is kernel-proven end to end — Steps 1–3 through
`sigmaStarCausal_deadlock_free_of_coverage` (causal push certificates
drain the pipes via the causal keystone, the chase names the withheld
push, σ*-causal pushes whenever coverage proves the frame demanded),
and Step 4 through `causalStuckCoverage` (Proofs/CausalMint.lean: the
minting ladder plus the τ-staged causal coverage induction). This is
the charter's constructive witness in full: a deterministic,
charter-local strategy pair no skeleton of the class jams.
Rests on (message-denominated) liveness; the W = 1 byte caveat of
record is Mux/Basic.lean's module doc (# The byte-denomination
caveat). -/
theorem c1_charter_false : ¬ C1StatementCharter := by
  intro hc1
  obtain ⟨sk, hwf, hm0, hnd⟩ :=
    hc1 1 (Nat.le_refl 1) sigmaStarCausal sigmaStarCausal
      (sigmaStarCausal_charterLocal .I) (sigmaStarCausal_charterLocal .R)
  exact hnd (sigmaStarCausal_deadlock_free_of_coverage hwf hm0
    (causalStuckCoverage hwf hm0) 1 (Nat.le_refl 1))

/-- The LEGACY-grain statement is false, GIVEN σ*'s legacy locality —
the hypothesis exactly as landed at stage 3 (module doc: it is the
A_p-sufficiency theorem quantified over every reachable observation,
probe-adjacent and recorded [open] at kernel tier). Retained as the
internal artifact the F3 ruling anticipated; the claim of record is
`c1_charter_false`.
Rests on (message-denominated) liveness; the W = 1 byte caveat of
record is Mux/Basic.lean's module doc (# The byte-denomination
caveat). -/
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
deadlocks. Message-denominated (Mux/Basic.lean, # The
byte-denomination caveat). -/
theorem wedge_sigmaStar_deadlock_free :
    MuxDeadlockFree wedge .impl 1 sigmaStar sigmaStar :=
  sigmaStar_deadlock_free wedge_wellFormed wedge_margin0 1 (Nat.le_refl 1)

set_option maxRecDepth 1000000 in
/-- The σ*-driven drain completes the smoke pin in the kernel: party
inference, ledger reconstruction, the demand closure, and the τ argmin
all execute end to end — the strategy is a real scheduler, not only a
proof object. Message-denominated (Mux/Basic.lean, # The
byte-denomination caveat). -/
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
