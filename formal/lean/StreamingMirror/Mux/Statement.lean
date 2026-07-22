/-
The mux campaign's statement of record: what the suite claims, in one
file, audit-small — the Mux/ companion of the base artifact's
Statement.lean, on the same contract. Every theorem below RESTATES a
statement of record inline, fully quantified, and proves it by citing
the landed theorem: the kernel checks that this file's spelling IS the
claim, so any drift between the audit surface and the proofs fails the
build. All on the three standard axioms (`propext`, `Classical.choice`,
`Quot.sound`); no `sorry`, no `native_decide`.

THIS FILE IS THE AUDIT MECHANISM: every statement of record is
restated here inline, fully quantified, and proved by citation, so the
kernel re-certifies on every build that the restatement IS the theorem
— drift between claim and proof is a build failure, not a review
finding. (An external claim-grading table once played this role; the
kernel-tied form supersedes it.)

# The resolved trichotomy (the campaign's charter, restated)

The campaign's question: does the single-pipe transport NEED
flow-control credits or true channel independence, or does a local
send-order schedule of the protocol's existing messages suffice?

- **C1 as chartered: FALSE** (`c1_charter`; the no-locality companion
  `c1_omniscient`). A deterministic, charter-local strategy pair —
  σ*-causal on both sides — is deadlock-free on the whole
  `.impl` + margin-0 class at every capacity C ≥ 1, locality and
  liveness both kernel-proven, no hypothesis. Nothing needs to be
  added to the wire; the right to IDLE, not frame choice, is the
  entire frontier.
- **C1-WC: TRUE** (`wc_impossibility`; K-parking `wc_impossibility_K`).
  One fixed, Rust-realizable skeleton defeats every WORK-CONSERVING
  pair at every capacity — even omniscient WC dies — and K-deep reply
  parking moves the wall without removing it.
- **T8: the window-generalized positive half**
  (`sigmaStarK_deadlock_free`, `sigmaStarK_completes`): at every
  per-direction advertised depth pair, EVERY strategy sending in any
  order the window discipline permits — licensing causal, at parking
  arrears K — is deadlock-free and completes within `2·ρ(init)`.
  Its English specification of record — fixed before the build, per
  the spec-first method — lives with its clause-by-clause audit
  crosswalk in Proofs/SigmaStarKLive.lean's module doc.
- **C2: TRUE at C₀ = 1** (`oracle_deadlock_free`; the conjunction
  `necessity`). The full-skeleton oracle of record is the STATIC send
  projection of the canonical schedule τ — non-adaptive, consulted
  only through the machine's own flush count; the refuted receive
  projection (`static_oracle_jams`) shows the ORDER, not adaptivity
  and not information, is the liveness ingredient. Necessity is
  class-relative: nonlocal information is necessary under
  work-conservation, not for liveness alone.

Every positive "completes" is two kernel facts: stuck-freedom (the
statement) and termination (`mux_terminating`). Every capacity is
denominated in MESSAGES; the byte-soundness caveat of record is
Mux/Basic.lean's module doc (# The byte-denomination caveat) —
impossibilities transfer to bytes unweakened, positives do not.

# What a skeptical reader must read, in full

The statements below quantify over these definitions and nothing else:

- **The harness** (Mux/Basic.lean, ~200 lines of definitions): `MObs`
  and `Strategy` (the observation alphabet — the slot-peek and
  receipts-out rulings live on `MObs` — and the strategy type),
  `MState`/`MAction`/`apply` (hand + pipe(C) + demux slot; wire sends
  only through the strategy-gated `push`), `mstuck`, `MReachable`,
  `MuxDeadlockFree` (strategies fixed, endpoint interleaving
  adversarial; idling is not a move), and the module doc's
  byte-denomination section, which scopes every positive claim here.
- **The base statement surface** (the root Statement.lean):
  `Skel.wellFormed`, the margin-0 hypothesis
  (`∀ sc, dCount sc ≤ capLevel` — the shipping `FAN ≥ kids`
  discipline), `AxMode.impl`, `Model.apply` — the harness wraps the
  base model unchanged.
- **The strategy classes**: `WorkConserving` (Mux/Strategy.lean,
  ~10 lines: never decline while holding a pushable frame) with its
  K/E-universe twins (`KWorkConserving`, `EWorkConserving`), and
  `CharterLocal` (Mux/Causal.lean: invariance across skeletons with
  equal ANNOUNCED views at `.impl`-realizable observations — the F3
  grain). Auditing `CharterLocal` means auditing the announced view
  itself: `AView`, `announcedIds`, `aviewOf` (Mux/Causal.lean,
  ~60 lines — the positional B5 decode of what arrived frames have
  determined). That is the honest cost of the charter-grain claim;
  Mux/Proofs/Grains.lean pins the grain nondegenerate
  (`charterView_nondegenerate`) and discriminating
  (`oracle_not_charterLocal`). For T8: `demandedAK`/`licensedA` and
  `WindowDisciplined` (Mux/SigmaStarK.lean, ~40 lines — the arrears-K
  licensing over the same announced view, and the
  gate-plus-progress class it quantifies).
- **The witnesses' shapes**: the `wedge` literal (Mux/Instances.lean,
  12 scopes) and the `wedgeW` family plus `deliverStepK`/`applyK`
  (Proofs/WcImpossibilityK.lean, ~40 lines) for the K statement.
- The legacy grain (`LocalEq`/`LocalStrategy`, Mux/Strategy.lean) only
  when reading the internal-artifact statements: the grain of record
  is `CharterLocal`, and the two are INCOMPARABLE (Proofs/C1.lean's
  module doc, kernel-pinned in Mux/Proofs/Grains.lean).

The reader need NOT read: anything under `Mux/Proofs/` (proof
scaffolding — its map is Mux/Proofs/Map.lean); the strategy
DEFINITIONS `sigmaStar`, `sigmaStarCausal`, `oracle` (each appears
below only as a witness whose load-bearing properties — locality,
liveness, nonlocality — are themselves kernel theorems; read
Mux/Causal.lean to learn WHAT the schedule computes, not to trust
these claims); the announced trace-family layouts (consumed only by
the witnesses' proofs).

# The chain to the Rust implementation

- **The wedge is real**: `src/tree/mirror/streaming/tests/wedge.rs`
  pins a deterministic tree pair whose trace-decoded skeleton equals
  the `wedge` literal at rootH 6 and matches the generator at the
  protocol's real height 32; the committed proptest seeds realize the
  jam MECHANISM on the old transport (not the byte-exact shape).
- **The announced view is real**: bridge B5 (stage-2 track D)
  reconstructs the announced skeleton from a payload-erased frame
  transcript — the premise behind `aviewOf`'s positional decode.
- **The model runs**: `lake exe muxprobe` executes the same
  `Mux.apply`/`mterminal` these statements quantify over across the
  golden strategy × skeleton × capacity × interleaving matrix
  (byte-pinned; `just muxprobe`), and Mux/Proofs/Twins.lean pins the
  executable twins definitionally equal to the kernel objects.
- **Height gap, honestly**: the executable tier tops at rootH 8 and
  the kernel literals at rootH 6, against the protocol's 32; the
  height-transfer argument (the jam mechanism is height-independent)
  is [derived], with wedge.rs pinning both ends.

# Assumed, not proven

- **Byte denomination**: positive statements say less than their
  byte-level reading; the W = 1 structural argument is assumed at the
  model boundary (Mux/Basic.lean, # The byte-denomination caveat).
- **`KR ≥ 4`** in `wc_impossibility_K` is [derived] (the
  widened-family argument); the kernel anchors are KR ∈ {1, 2, 3}.
- **The `.full`/schedulable port** is open by ruling: every mux
  statement of record lives at `.impl` + margin-0.
- **Internal artifacts stay conditional forever**: `c1_literal_false`
  (Proofs/C1.lean) carries the omniscient σ*'s legacy-grain locality
  as named [open] hypotheses — superseded as a claim of record by
  `c1_charter` below, retained as the stage-3 record.
- The base model's premises (error-free conforming peers, SPSC
  channels, per-channel in-order delivery): the root Statement.lean.

# Conservativity notes

- `mcanStep` enumerates `allMActions`; an accidental omission makes
  `mstuck` easier and every positive `MuxDeadlockFree` claim HARDER to
  prove, so the enumeration cannot silently weaken one. For the
  impossibilities the concern inverts, and their stuck certificates
  are kernel-replayed on concrete runs — nothing is enumerated away.
- `mterminal` (base terminal + both pipes drained) is the definition
  that could weaken a claim by holding too early: the F8-strengthened
  wire closes police exactly that, and the gadget controls
  (`Control.noF8_bogus_terminal`, `Control.noF8_bogus_mterminal`,
  `Control.f8_rejects_gadgetTrap`, Mux/Controls.lean) pin the guard
  live from both sides.

# Non-vacuity

Every ∀-class statement cites a kernel inhabitant and every witness
skeleton its class membership: `wedge_wellFormed` + `wedge_margin0`
(the impossibility witness sits inside the base flagship's proven
class, so a muxed jam indicts the mux alone);
`bottomMostReady_wc`/`_wcK`/`_wcE` + `bottomMostReady_local`
(Mux/Proofs/Inhabitation.lean: the shipped policy inhabits every
indicted class); `localEq_nondegenerate` and
`charterView_nondegenerate` (both locality grains are nondegenerate);
`mreachable_init` below (the reachability quantifiers are inhabited);
and the completion pins (`smokeChain_mux_completes`,
`wedge_sigmaStarCausal_completes`, `piWedge_oracle_completes`, …) run
each positive witness end to end in the kernel.
-/
import StreamingMirror.Mux.Proofs.C1
import StreamingMirror.Mux.Proofs.Termination
import StreamingMirror.Mux.Proofs.Inhabitation
import StreamingMirror.Mux.Proofs.Grains
import StreamingMirror.Mux.Proofs.SigmaStarKLive

namespace StreamingMirror.Mux.Statement

open Model
open StreamingMirror.Mux

/-- C1 as chartered is FALSE — the campaign's headline, at the grain
of record per Finch's locality ruling: it is NOT the case that every
charter-local deterministic
pair has a killer skeleton. The refutation witness is
⟨C = 1, σ*-causal, σ*-causal⟩ with locality
(`sigmaStarCausal_charterLocal`) and liveness
(`sigmaStarCausal_deadlock_free`) both kernel-proven — no hypothesis.
The class hypothesis has bite: `charterView_nondegenerate` pins the
grain nondegenerate and `oracle_not_charterLocal` pins a strategy
outside it (Mux/Proofs/Grains.lean). Rests on message-denominated
liveness; the byte caveat of record is Mux/Basic.lean's module doc
(# The byte-denomination caveat). -/
theorem c1_charter :
    ¬ (∀ (C : Nat), 1 ≤ C → ∀ (σI σR : Strategy),
        CharterLocal .I σI → CharterLocal .R σR →
        ∃ sk : Skel, sk.wellFormed = true
          ∧ (∀ sc, sk.dCount sc ≤ sk.capLevel)
          ∧ ¬ MuxDeadlockFree sk .impl C σI σR) :=
  c1_charter_false

/-- C1 widened to EVERY strategy pair, local or not, is also false —
the no-locality-hypothesis companion: ⟨1, σ*, σ*⟩ plus T4. Rests on
message-denominated liveness; the byte caveat of record is
Mux/Basic.lean's module doc (# The byte-denomination caveat). -/
theorem c1_omniscient :
    ¬ (∀ (C : Nat), 1 ≤ C → ∀ (σI σR : Strategy),
        ∃ sk : Skel, sk.wellFormed = true
          ∧ (∀ sc, sk.dCount sc ≤ sk.capLevel)
          ∧ ¬ MuxDeadlockFree sk .impl C σI σR) :=
  c1_omniscient_false

/-- C1-WC is TRUE (T3; §1's second trichotomy bullet): one fixed
skeleton — `wedge`, the committed-regression shape, inside the base
flagship's proven class (`wedge_wellFormed`, `wedge_margin0`) —
defeats every work-conserving pair at every capacity, locality not
even assumed. Each hypothesis is pinned load-bearing:
work-conservation by `Control.wedge_idler_completes` (an idling pair
completes the same skeleton), the one-slot demux by
`Control.wedge_unboundedSlot_completes`, `1 ≤ C` by
`Control.smokeChain_C0_not_deadlockFree`; the class is
kernel-inhabited (`bottomMostReady_wc`), so the ∀ is not
satisfiable-empty. Message-denominated; transfers to bytes unweakened
(Mux/Basic.lean, # The byte-denomination caveat). -/
theorem wc_impossibility :
    ∀ (C : Nat), 1 ≤ C → ∀ (σI σR : Strategy),
      WorkConserving .I σI → WorkConserving .R σR →
      ¬ MuxDeadlockFree wedge .impl C σI σR :=
  _root_.StreamingMirror.Mux.wc_impossibility

/-- K-deep parking is mitigation, not cure (T8's impossibility half):
at every kernel-anchored responder depth KR ∈ {1, 2, 3}, every
initiator depth KI ≥ 1, and every capacity C ≥ 1, the width-(KR + 5)
wedge defeats every work-conserving pair. `KR ≥ 4` is [derived] (the
widened-family argument — each depth needs its own kernel replay); the
witnesses are inside the base flagship's class (`wedgeW_wellFormed`,
`wedgeW_margin0`); the class is kernel-inhabited
(`bottomMostReady_wcK`). Message-denominated; transfers to bytes
unweakened (Mux/Basic.lean, # The byte-denomination caveat). -/
theorem wc_impossibility_K :
    ∀ (KI KR : Nat), 1 ≤ KI → (KR = 1 ∨ KR = 2 ∨ KR = 3) →
      ∀ (C : Nat), 1 ≤ C → ∀ (σI σR : Strategy),
        KWorkConserving .I σI → KWorkConserving .R σR →
        ¬ MuxDeadlockFreeK (wedgeW (KR + 5)) .impl KI KR C σI σR :=
  _root_.StreamingMirror.Mux.wc_impossibility_K

/-- The strategy of record is deadlock-free, unconditionally: the
charter-local σ*-causal pair completes every well-formed margin-0
session at every capacity C ≥ 1 ("completes" = this stuck-freedom plus
`mux_terminating` below). This is the charter's constructive witness:
demand-lockstep over the ANNOUNCED sub-skeleton — the announcement
prefix the protocol already carries, FIFO positional arithmetic, and
the inevitability closure; nothing new on the wire (the "mysterious
third thing", named). The evidence-only control
(`wedge_evidence_not_deadlockFree`, Proofs/C1.lean) pins the closure
load-bearing. Message-denominated; the byte caveat of record is
Mux/Basic.lean's module doc (# The byte-denomination caveat). -/
theorem sigmaStarCausal_deadlock_free :
    ∀ (sk : Skel), sk.wellFormed = true →
      (∀ sc, sk.dCount sc ≤ sk.capLevel) →
      ∀ (C : Nat), 1 ≤ C →
        MuxDeadlockFree sk .impl C sigmaStarCausal sigmaStarCausal :=
  fun _ hwf hm0 C hC =>
    _root_.StreamingMirror.Mux.sigmaStarCausal_deadlock_free hwf hm0 C hC

/-- The strategy of record is charter-local, for both parties: its one
skeleton read is the announced view, so invariance across equal
announced views at `.impl`-realizable observations is a computation.
The locality half of `c1_charter`'s witness; the grain's
non-triviality controls are Mux/Proofs/Grains.lean. -/
theorem sigmaStarCausal_charterLocal :
    ∀ p : Party, CharterLocal p sigmaStarCausal :=
  _root_.StreamingMirror.Mux.sigmaStarCausal_charterLocal

/-- T4, the omniscient proof vehicle: σ* — demand-lockstep over the
FULL-skeleton closure — is deadlock-free on the same class. The
liveness half of the legacy-grain record; the claim of record at the
charter grain is `sigmaStarCausal_deadlock_free` (σ*'s own locality
stays [open] forever as `c1_literal_false`'s internal-artifact
hypothesis). Message-denominated; the byte caveat of record is
Mux/Basic.lean's module doc (# The byte-denomination caveat). -/
theorem sigmaStar_deadlock_free :
    ∀ (sk : Skel), sk.wellFormed = true →
      (∀ sc, sk.dCount sc ≤ sk.capLevel) →
      ∀ (C : Nat), 1 ≤ C →
        MuxDeadlockFree sk .impl C sigmaStar sigmaStar :=
  fun _ hwf hm0 C hC =>
    _root_.StreamingMirror.Mux.sigmaStar_deadlock_free hwf hm0 C hC

/-- C2 is TRUE at C₀ = 1 (T5): the oracle of record — the STATIC send
projection of the canonical schedule τ, consulted only through the
machine's own flush count — completes every well-formed margin-0
skeleton at every capacity C ≥ 1 ("completes" = this plus
`mux_terminating`; the greedy form is `oracle_greedy_run_terminal`).
The ingredient is the ORDER: the receive-projection pusher with the
same information jams (`static_oracle_jams`), and the oracle's
nonlocality is pinned at BOTH grains (`oracle_not_localStrategy`,
`oracle_not_charterLocal`). Message-denominated; the byte caveat of
record is Mux/Basic.lean's module doc (# The byte-denomination
caveat). -/
theorem oracle_deadlock_free :
    ∀ (sk : Skel), sk.wellFormed = true →
      (∀ sc, sk.dCount sc ≤ sk.capLevel) →
      ∀ (C : Nat), 1 ≤ C →
        MuxDeadlockFree sk .impl C (oracle .I) (oracle .R) :=
  fun _ hwf hm0 C hC =>
    _root_.StreamingMirror.Mux.oracle_deadlock_free hwf hm0 C hC

/-- T6, the trichotomy conjunction: the wedge kills every
work-conserving pair at every capacity, AND the oracle completes every
margin-0 skeleton at capacity one. Read class-relatively: nonlocal
information is necessary for liveness UNDER WORK-CONSERVATION — not
for liveness alone (`c1_charter`). Message-denominated (Mux/Basic.lean,
# The byte-denomination caveat). -/
theorem necessity :
    ∀ (C : Nat), 1 ≤ C →
      (∀ σI σR : Strategy,
          WorkConserving .I σI → WorkConserving .R σR →
          ¬ MuxDeadlockFree wedge .impl C σI σR)
      ∧ (∀ sk : Skel, sk.wellFormed = true →
          (∀ sc, sk.dCount sc ≤ sk.capLevel) →
          MuxDeadlockFree sk .impl 1 (oracle .I) (oracle .R)) :=
  _root_.StreamingMirror.Mux.necessity

/-- The K = ∞ endpoint of the parking dial: with unbounded reply
parking, EVERY pushes-when-nonempty pair is deadlock-free at every
capacity — bounded demux state, not scheduling, is what the
impossibilities indict (the eager-absorption design's formal echo).
The class is kernel-inhabited (`bottomMostReady_wcE`). Capacity and
parking are message-denominated; the byte caveat of record is
Mux/Basic.lean's module doc (# The byte-denomination caveat). -/
theorem elastic_deadlock_free :
    ∀ (sk : Skel), sk.wellFormed = true →
      (∀ sc, sk.dCount sc ≤ sk.capLevel) →
      ∀ (C : Nat), 1 ≤ C → ∀ (σI σR : Strategy),
        EWorkConserving .I σI → EWorkConserving .R σR →
        MuxDeadlockFreeE sk .impl C σI σR :=
  fun sk hwf hm0 _ hC _ _ hWI hWR =>
    _root_.StreamingMirror.Mux.elastic_deadlock_free sk hwf hm0 hC hWI hWR

/-- Mux-tier termination, the second half of every "completes": every
muxed run from `init` is bounded by `2·ρ(init)` — no infinite runs
under any strategy pair, any mode, any capacity (the phase-4 F5
mint). -/
theorem mux_terminating :
    ∀ (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy)
      (acts : List MAction) (s' : MState),
      mrun sk ax C σI σR (init sk) acts = some s' →
      acts.length ≤ 2 * rho sk (Model.init sk) :=
  fun _ _ _ _ _ _ _ hrun =>
    _root_.StreamingMirror.Mux.mux_terminating hrun

/-- Non-vacuity of muxed reachability: the initial state is always
reachable, so every `MuxDeadlockFree` above quantifies over an
inhabited set. -/
theorem mreachable_init :
    ∀ (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy),
      MReachable sk ax C σI σR (init sk) :=
  fun _ _ _ _ _ => .init

/-- T8, the window-generalized positive half (the spec's claim, clause
by clause — Proofs/SigmaStarKLive.lean's module doc is the
specification of record with its audit crosswalk): every
well-formed margin-0 skeleton (clause 1), every capacity C ≥ 1
(clause 2), every advertised depth pair K_I, K_R ≥ 1, independent and
possibly unequal (clause 3), and EVERY window-disciplined pair — any
selection rule among licensed frames, each side gated at its PEER's
advertised depth (clause 4; `WindowDisciplined`, with the canonical
σ*ₖ and the shipped-ladder shape as kernel-pinned inhabitants:
`sigmaStarK_windowDisciplined`, `sigmaLadderK_windowDisciplined`),
licensing causal at parking arrears K (clause 5; `demandedAK` over the
announced view; `sigmaStarK_one` pins the K = 1 degeneration to
σ*-causal) — the K-deep composition has no reachable stuck state.
Clause 6's other conjunct is `sigmaStarK_completes` below; the
executable pins are the asymmetric (1, 4) smoke and the (2, 2) wedge
(`smokeChain_sigmaStarK_completes_1_4`,
`wedge_sigmaStarK_completes_2_2`). Message-denominated; the byte
caveat of record is Mux/Basic.lean's module doc (# The
byte-denomination caveat). -/
theorem sigmaStarK_deadlock_free :
    ∀ (sk : Skel), sk.wellFormed = true →
      (∀ sc, sk.dCount sc ≤ sk.capLevel) →
      ∀ (KI KR : Nat), 1 ≤ KI → 1 ≤ KR →
        ∀ (C : Nat), 1 ≤ C → ∀ (σI σR : Strategy),
          WindowDisciplined KR .I σI → WindowDisciplined KI .R σR →
          MuxDeadlockFreeK sk .impl KI KR C σI σR :=
  fun _ hwf hm0 KI KR hKI hKR C hC _ _ hWI hWR =>
    _root_.StreamingMirror.Mux.sigmaStarK_deadlock_free hwf hm0
      KI KR hKI hKR C hC hWI hWR

/-- T8's "always completes", assembled (clause 6, both conjuncts in
one kernel fact): under any window-disciplined pair the greedy K drain
reaches `mterminal` within `2·ρ(init)` steps — progress plus the
K-variant termination bound (`mux_terminatingK`). Message-denominated;
the byte caveat of record is Mux/Basic.lean's module doc (# The
byte-denomination caveat). -/
theorem sigmaStarK_completes :
    ∀ (sk : Skel), sk.wellFormed = true →
      (∀ sc, sk.dCount sc ≤ sk.capLevel) →
      ∀ (KI KR : Nat), 1 ≤ KI → 1 ≤ KR →
        ∀ (C : Nat), 1 ≤ C → ∀ (σI σR : Strategy),
          WindowDisciplined KR .I σI → WindowDisciplined KI .R σR →
          mterminal sk
            (mdrainK sk .impl KI KR C σI σR
              (2 * rho sk (Model.init sk)) (init sk)) = true :=
  fun _ hwf hm0 KI KR hKI hKR C hC _ _ hWI hWR =>
    _root_.StreamingMirror.Mux.sigmaStarK_completes hwf hm0
      KI KR hKI hKR C hC hWI hWR

end StreamingMirror.Mux.Statement
