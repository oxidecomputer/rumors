# T8: the specification of record

This document is the English statement of the theorem T8
(`sigmaStarK_deadlock_free`, the window-generalized liveness of the
single-connection transport) — fixed BEFORE the theorem is built, per
Finch's statement-faithfulness ruling (MUX-PROGRESS.md §4, phase-4
entry): theorem statements must output claims entirely accurate to
intent; implementation may be messy, claims may not. This page is the
intent. It binds three audiences:

- **The T8 build track**: the Lean statement must transcribe this page.
  Where transcription forces a choice this page does not determine, the
  choice is recorded here (amended, dated) before the theorem lands.
- **Review/audit passes**: grade the landed Lean statement against each
  clause below — EXACT / WEAKER (name what is lost) / STRONGER — in the
  MUX-STATEMENT-AUDIT.md style. Any WEAKER grade is a wrong-grade
  finding by definition.
- **Humans**: this is what the theorem means, with its boundaries. No
  Lean required.

## The claim

> For every pair of trees the protocol can synchronize at all, every
> single-channel capacity, and every pair of advertised window depths —
> the two directions independent and possibly unequal — the
> single-connection session cannot deadlock and always completes: each
> side sends its existing protocol messages in ANY order permitted by
> the window discipline, gated only by an inference computed from its
> own tree and the frames it has decoded so far, and every such session
> reaches successful completion of the synchronization in a bounded
> number of steps.

## The hypotheses, clause by clause

1. **"Every pair of trees the protocol can synchronize at all."**
   Every well-formed dispute skeleton in the `.impl` + margin-0 class —
   the shipping encoder's publication discipline at its deployed
   back-pressure bound; exactly the domain of the base flagship
   `Sched.deadlock_free`. T8 adds no tree assumptions beyond the
   protocol theorem's own.
2. **"Every single-channel capacity."** Pipe capacity C ≥ 1, universally
   quantified. Capacity is not a correctness parameter.
3. **"Every pair of advertised window depths."** K_I ≥ 1 and K_R ≥ 1,
   independent per direction (each sender gated at its peer's
   advertisement). K = 1 (demand-lockstep) is included; asymmetric
   pairings are included. A single-K statement is WEAKER and fails the
   audit.
4. **"ANY order permitted by the window discipline."** The statement
   MUST quantify over all selection rules among licensed frames: every
   strategy that, whenever at least one licensed push exists, performs
   some licensed push. The canonical least-frame scheduler is one
   instance; the shipped priority ladder is another. A theorem about
   one concrete scheduler is WEAKER and fails the audit — the point of
   this clause is that the implementation's frame ordering is provably
   impossible to get wrong.
5. **"Gated only by an inference from its own tree and decoded
   frames."** The licensing predicate is the causal closure over the
   announced sub-skeleton (`inevitableA`-based, at parking arrears K):
   charter-local in the sense of MUX-PROGRESS §4's F3 ruling —
   information in the party's causal past at each decision point, frame
   contents included, nothing else. An omniscient-closure statement is
   WEAKER (it specs an engine the implementation cannot compute) and
   fails the audit.
6. **"Cannot deadlock and always completes."** Both: no reachable state
   of the composition is stuck (progress), and every run terminates
   within a bounded step count (the mux termination measure, so
   "completes" is kernel-honest per the phase-4 F5 mint). Progress-only
   is WEAKER and fails the audit.

## The boundaries (not part of the claim; part of its honest reading)

- **Model tier.** A theorem about the Lean model (scope-level messages,
  payload-erased beyond labels, conforming error-free peers), connected
  to the Rust by the bridge suite (`assert_valid` proptests, wedge
  realizability, B5 announced-skeleton reconstruction) and, when built,
  tracecheck (TRACECHECK.md) — the kernel guarantees the model; the
  bridges argue the model is the Rust.
- **Reply-denominated.** Byte-level liveness additionally requires the
  byte pacing of design/single-socket.md; the W = 1 byte caveat of
  record (Mux/Basic.lean module doc) applies. Impossibility results
  transfer to bytes a fortiori; this positive result does not.
- **No performance content.** Latency is the chartered latency-
  conjectures campaign (MUX-PROGRESS §3b); at evidence tier the K-dial
  law (MUX-LATENCY.md §7) prices this construction.
- **Single conforming session.** Transport physics (loss coupling,
  byte-granularity interleaving), adversarial peers, and multi-session
  interaction are outside the model's premises, as for every theorem in
  this artifact.

## What T8 completes

Together with the landed suite (`wc_impossibility`(_K), T4,
`sigmaStarCausal_deadlock_free` + `c1_charter_false`,
`oracle_deadlock_free`, `necessity`, the termination and wide
theorems): the deadlock that motivated the Link abstraction is
impossible for the single-connection implementation as designed — at
every capacity, every window pairing, and every frame ordering the
implementation could choose — over exactly the class of
synchronizations the protocol itself is proven to perform. This is the
fully-quantified validity statement for the single-socket design — 
which, per the product conclusion of record (2026-07-22, exposition
@consequence and MUX-PROGRESS §3e), stands as the library's
CONTINGENCY of record rather than a successor to the `Link` contract:
the theorem guarantees the contingency, and the `Link` requirement
remains the product surface. The single-socket plan's §3
reconciliation table pointed here as its merge gate; that gate is
satisfied.

## Landed (2026-07-22): the audit crosswalk

The theorem of record is **`sigmaStarK_deadlock_free`**
(`formal/lean/StreamingMirror/Mux/Proofs/SigmaStarKLive.lean`), with
the termination half `mux_terminatingK` and the completion package
`sigmaStarK_completes`/`muxK_greedy_run_terminal`
(`Mux/Proofs/Termination.lean`). Clause by clause:

| Clause | Lean transcription | Grade |
|---|---|---|
| 1. every synchronizable tree pair | `hwf : sk.wellFormed = true`, `hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel` — verbatim the base flagship's domain; no added tree assumptions | EXACT |
| 2. every capacity | `(C : Nat) (hC : 1 ≤ C)`, universally quantified | EXACT |
| 3. every depth pair, per direction | `(KI KR : Nat) (hKI : 1 ≤ KI) (hKR : 1 ≤ KR)`, independent; the semantics is the landed K-harness (`MuxDeadlockFreeK sk .impl KI KR C σI σR`, `deliver` gated at the RECEIVING party's depth via `recvDepth`); the asymmetric pin `smokeChain_sigmaStarK_completes_1_4` executes (K_I, K_R) = (1, 4) | EXACT |
| 4. ANY order permitted by the discipline | class quantification: `{σI σR} (hWI : WindowDisciplined KR .I σI) (hWR : WindowDisciplined KI .R σR)` — the proof consults only the class's two conjuncts, never a selector; inhabitants checked: `sigmaStarK_windowDisciplined` (canonical) and `sigmaLadderK_windowDisciplined` (the shipped `bottomMostReady` ladder over the licensed set) | EXACT |
| 5. gated only by a causal inference | the licensing predicate is `demandedAK` (`Mux/SigmaStarK.lean`): the ANNOUNCED closure `inevitableA` over `aviewOf` at parking arrears K — never the omniscient `inevitable`; the class's GATE conjunct binds σ to it | EXACT |
| 6. cannot deadlock AND always completes | progress: `sigmaStarK_deadlock_free` (no reachable `mstuckK` state); bounded termination: `mux_terminatingK` (every K run ≤ 2·ρ(init) steps) and `muxK_greedy_run_terminal` — packaged as `sigmaStarK_completes` | EXACT |

Companions: `sigmaStarK_pair_deadlock_free` (the canonical pair, each
side gated at its peer's advertised depth — the C1.lean stub's
intended statement, now a corollary); `sigmaStarK_one`
(`sigmaStarK 1 = sigmaStarCausal`, the demand-lockstep degeneration,
strategy-level); `wedge_sigmaStarK_completes_2_2` (the canonical
adversarial shape completes under the two-deep window, kernel-decided);
`stuck_coverage_arrears` (`Mux/Proofs/CausalMint.lean`) — the landed
Step-4 minting ladder with the parking arrears as a parameter, whose
K = 1 instance re-derives the landed `causalStuckCoverage` verbatim.

### Determined transcription choices (recorded 2026-07-22, none amend the claim)

1. **The arrears numbering.** Clause 5's "announced-closure at parking
   arrears K" is transcribed 0-based (the artifact's `Sched.Ev`
   numbering): frame `n = pushedCount` licensed iff
   `n < K ∨ rcv(c, n − K) ∈ inevitableA` — identically the 1-based
   "frame k licensed iff k ≤ K ∨ rcv(c, k−K)". The form is DERIVED,
   not chosen: the K-deep demux guard (`chan < recvDepth`) makes a
   blocked cell hold exactly K frames past the consumer, and Step 1's
   keystone contradiction needs the head's push-time certificate to
   name exactly `rcv(c, recvdOf)` (`Mux/SigmaStarK.lean`, module doc).
2. **The class's two conjuncts.** Clause 4's sentence names the
   progress obligation ("performs some licensed push"); clause 5's
   "gated ONLY by" supplies the gate obligation (σ never names an
   unlicensed frame — without it the class contains work-conserving
   members and `wc_impossibility_K` refutes the claim). The class is
   their conjunction, guarded by realizability under the K-composition
   (`KConsistentAny`, mirroring `KWorkConserving`'s posture). The
   guard widens the class, so the ∀-class claim is at least as strong
   as the unguarded reading — auditable as EXACT-or-STRONGER, never
   weaker.
