# MUX-STATEMENT-AUDIT — the phase-4 charter-vs-Lean claim table

Provenance: produced by the mux-conjectures campaign's phase-4
adversarial review (the statement-strength auditor role), at worktree
HEAD 92b1c9bc (pre-repair; see below), and graduated into the repo
by the phase-4 repair track so the phase-5 legibility pass can consume
it — the audit surface `Mux/Statement.lean` should carry these grades
forward, and every WEAKER/PROSE grade is a candidate for the "weakest
honest form" rewrite that pass owes.

Repair status: the five "wrong"-grade findings below (W1–W5) were
confirmed by the phase-4 synthesis and REPAIRED by the phase-4 repair
track in this same branch — see MUX-PROGRESS §4's phase-4 entry for
the disposition of each (W1 → the canonical byte-caveat home;
W2 → wedge provenance corrected at all four surfaces; W3/W4 → the §1
superseded-markers; W5 → the kernel inhabitants,
Mux/Proofs/Inhabitation.lean). The table body is retained VERBATIM as
the audit's record — grades describe the pre-repair state it reviewed.

---

# Phase-4 statement-strength audit — the charter-vs-Lean claim table

Role: statement-strength auditor, mux-conjectures campaign phase 4.
Reviewed: worktree `/Users/oxide/src/rumors-mux`, branch `mux-conjectures`
(merged suite; T10 and σ*-causal tracks in flight elsewhere, NOT reviewed).
Ground truth: `formal/MUX-PROGRESS.md` (§1 charter + Resolution, §4
findings, §5 log), `formal/MUX-ADJUDICATION.md`, `formal/AUDIT-NOTES.md`,
`formal/mux-notes-phase2/STAGE0-GATES.md`. Artifact:
`formal/lean/StreamingMirror/Mux/**` + `Proofs/Termination.lean`.

Method: for every English claim in MUX-PROGRESS §1 (original conjectures
AND Resolution bullets with superseded-markers), find the Lean statement(s)
of record and grade the match. Grades:

- **EXACT** — the Lean statement says what the English says (modulo
  adjudicated rulings, which are themselves graded in Table C).
- **STRONGER** — the Lean proves more than claimed.
- **WEAKER** — something is lost (domain, quantifier, denomination,
  epistemic tier); the loss is named.
- **PROSE** — no Lean object exists or is expected; claim lives at
  prose/executable tier only.

All file paths below are relative to `/Users/oxide/src/rumors-mux/`.
The kernel has checked every proof; nothing here questions proof validity —
only whether statements, hypotheses, and docstrings say what the documents
claim.

---

## Table A — the original conjectures (MUX-PROGRESS §1, lines 40–62)

| # | English claim (charter) | Lean statement(s) of record | Grade | Notes |
|---|---|---|---|---|
| A1 | **C1**: ∀ capacity C, ∀ deterministic local strategy pairs, ∃ well-formed schedulable tree pair on which the muxed session deadlocks | `C1Statement` (Mux/Proofs/C1.lean:79–84); refuted by `c1_literal_false` (C1.lean:108–114) | EXACT minting; refutation **WEAKER** | Minting deltas, all adjudicated and disclosed in the module doc: "tree pair" → `Skel` (tree-argument collapse + Rust realizability rule); "schedulable" → wellFormed + margin-0 (domain ruling #4); `1 ≤ C` guard added (weakens the proposition, so refuting it is the *stronger* refutation — ¬guarded ⇒ ¬unguarded); "deterministic" free by function-ness; "local" = `LocalStrategy`. The refutation is **conditional**: `c1_literal_false` takes σ*'s locality as two explicit hypotheses (`hlocI`/`hlocR`), which is the [open] A_p-sufficiency residue — probe-checked 4,970/4,970, not kernel. Disclosed with precision in C1.lean:17–29 and §4; NOT visible from §1 (see finding W4). |
| A2 | C1 widened: even non-local pairs (implicit in the resolution's "omniscient WC dies") | `C1StatementOmniscient` (C1.lean:88–92), `c1_omniscient_false` (C1.lean:99–102) | STRONGER | Unconditional; witness ⟨C=1, σ*, σ*⟩. No analogue was chartered; this is the artifact's honest unconditional headline for C1-falsity. |
| A3 | **C2 positive**: an oracular send-order function of BOTH skeletons produces deadlock-free send orders over the bounded pipes, "conjecturally with small constant capacity" | `oracle_deadlock_free` (Mux/Proofs/Necessity.lean:57–61), engine in Mux/Proofs/Oracle.lean; `oracle` = `sendProj` pusher (Oracle/Order.lean:49–63) | STRONGER | Every C ≥ 1, so C₀ = 1 exactly — better than "small constant". The oracle is moreover NON-adaptive (fixed list indexed by own flush count), which the charter did not dare ask. Unconditional (`muxInv_reachable` discharges the ground facts). Reply-denominated (Table C, R4). |
| A4 | **C2 necessity**: "its dependence on remote information is essential; that necessity is exactly C1, so it should land as a corollary" | `necessity` (Necessity.lean:70–77) = T3 ∧ T5; `oracle_not_local` / `oracle_not_local_behavioral` / `demandOrder_not_local` (Oracle/Controls.lean:224–244) | WEAKER, by design and on the record | The chartered corollary died with C1-literal. Landed: class-relative restatement (necessary under work-conservation, not for liveness alone), plus kernel pins that the oracle genuinely consumes remote structure. The docstring states the class-relativity correctly. Note the WC conjunct is trivially ∀-strategy (even omniscient WC dies), so "nonlocal is necessary under WC" is subsumption, not tension — this is advertised in Necessity.lean's module doc. |
| A5 | Charter fallback: "if C1 is refuted, the refutation must be a constructive witness strategy plus a tight bound on the minimum pipe capacity" | `sigmaStar` (Mux/SigmaStar.lean:132–139); `sigmaStar_deadlock_free` at every C ≥ 1 (SigmaStarLive.lean:422–424); C = 1 suffices per the refutation witness | EXACT for capacity; WEAKER for "witness strategy" locality | The witness is constructive and the bound is the minimum possible (C = 1). But the charter's C1 is about *local* strategies, and the kernel σ* runs the full-skeleton closure — the constructive **local** witness is the in-flight σ*-causal object (Finch's definitional ruling, §4). On the merged suite, the "constructive witness" clause is satisfied only modulo the named locality hypothesis. |
| A6 | "The most interesting residual question … a natural signal strictly weaker than the full remote skeleton" | No Lean object (correctly); named in prose: Necessity.lean module doc:20–37, MUX-ADJUDICATION §1.3 | PROSE | Announcement prefix + FIFO positional arithmetic + inevitability closure. Its kernel form is exactly the σ*-causal A_p-sufficiency theorem — in flight, out of this review's scope. Consistent everywhere it appears. |

## Table B — the Resolution bullets (MUX-PROGRESS §1, lines 64–123)

| # | Resolution bullet | Lean statement(s) | Grade | Notes |
|---|---|---|---|---|
| B1 | **C1-literal FALSE** [derived, two named conditions]: σ* "adds zero control messages, is deterministic and local, and is deadlock-free at every C ≥ 1 per direction on the `.impl`/margin-0 class"; conditions (A) keystone over push-time derivation trees, (B) causal probe gate | `sigmaStar_deadlock_free` (SigmaStarLive.lean:422–424) ✓ every C ≥ 1, `.impl`+margin-0 ✓; `keystone` (Chase/Keystone.lean:67) ✓ push-time form; stage-0 P1 passed ✓; `c1_literal_false` conditional (C1.lean:108) | Liveness EXACT; **"local" WEAKER** | "Zero control messages" holds by construction (frozen `MAction` alphabet). "Deterministic" free. **"Local" is not a kernel fact of the merged suite**: kernel σ* is the omniscient-closure formulation; locality = [open] A_p-sufficiency, carried as `c1_literal_false`'s named hypothesis. Conditions A and B are both DISCHARGED; the bullet's condition list is stale and does not name the live residue → finding W4. |
| B2 | **C1-WC TRUE**: one fixed tree-realizable skeleton "(`wedge`, the regression shape at w = 4, rootH = 6, realized by the committed proptest seeds)" defeats every WC pair, locality dropped, every C ≥ 1, forced run with singleton consultations, no fooling/pigeonhole; mechanism slot-occupation + FIFO burial; capacity-flat [checked] | `wc_impossibility` (Mux/Proofs/WcImpossibility.lean:560–562); `wedge` (Mux/Instances.lean:51–62); forced-run executor `forcedPush`/`fstep`/`fdrain_replay` (WcImpossibility.lean:218–366); 4 kernel anchors (C∈{1,2,3} + `noHands` for C ≥ 4); mechanism pinned by `wedge_unboundedSlot_completes` (Mux/Controls.lean:169) + `wedge_elastic_completes` (Elastic.lean:513) | Theorem EXACT (matches the adjudication T3 template verbatim); **witness description WRONG in the docs** | The landed `wedge` has **six** whole-subtree provisions and root fan **7** (Instances.lean:53–62, "provisions := 6"), not "w = 4". MUX-PROGRESS §1:80–82 and MUX-ADJUDICATION §1.2/T0 ("w = 4, fan ≥ 5 root") both misdescribe the kernel witness. w = 4 is the *probe's minimal* jamming width; the artifact used the committed-regression 6-provision shape. Theorem unaffected (∃-witness); docs need the number fixed → finding W2. Singleton-consultation claim verified: `forcedPush` fires only on `enabledPushes = [h]`, and `forced_replay` uses WC exactly there. Capacity-flat stays [checked] (probe), correctly not claimed in Lean. |
| B3 | **C2 positive TRUE at C₀ = 1** (message = reply units) — original text names the receive projection; **[superseded marker: kernel oracle of record is the SEND projection]** | `oracle` = `sendProj` pusher (Oracle/Order.lean:49–63); `oracle_deadlock_free` (Necessity.lean:57); receive-projection refutation kernel-pinned: `static_oracle_jams` / `static_oracle_not_deadlockFree` (Oracle/Controls.lean:136–157) on `piWedge` | EXACT (with the superseded marker honored) | The superseded-marker discipline worked here: the marker's content ("send projection absorbed by demux slots; receive projection jams") is exactly what Oracle.lean proves and pins. The refuted `demandOrder` is retained as data with the negative control, per the marker. `Gen.piOrder` rename as recorded in §4. |
| B4 | **The third thing, named** (announcement prefix + FIFO arithmetic + inevitability closure; credits = computation and timing, not information; σ* is W = 1 credits inferred) | prose only: Necessity.lean module doc, MUX-ADJUDICATION §1.3 | PROSE | Consistent across docs and module docs. Kernel form is the in-flight σ*-causal coverage theorem. |
| B5 | **H-c demoted to executable tier**: "No quantitative overlap claim enters any theorem" | verified by absence; explicit "NOT claimed" scope notes in Oracle.lean:42–43, Necessity.lean:55–56 | EXACT | Swept the suite: no latency/overlap/optimality object anywhere in Lean. muxprobe carries the record at [checked] tier. |

## Table C — the standing rulings (MUX-PROGRESS §1, lines 110–123)

| # | Ruling | Artifact | Grade | Notes |
|---|---|---|---|---|
| R1 | **Observation = slot-peek** (frames observed at demux delivery, pre-consumption) | `MObs.delivered` recorded at `deliver`, pre-consumption (Mux/Basic.lean:82–98, 315–324) | EXACT | Faithfully implemented and documented. **But** the ruling's supporting clause "the no-peek variant is plausibly false via the two-height mutual proof-starvation gadget" (MUX-PROGRESS:115–116) is **contradicted by stage-0 P4** (no-peek causal σ* Terminal 3,470/3,470, incl. the F2 family; STAGE0-GATES.md:17; §5 log line ~613: "slot-peek stands as a modeling decision, NOT a demonstrated liveness necessity"). §1 is the ruling of record with superseded-markers — this clause never got one → finding W3. Related: Basic.lean:90–92 and Strategy.lean:17–19 say "peek is load-bearing for σ*'s coverage step" — true of the landed *proof* (`groundedPush` grounds arrivals in `.delivered`), not demonstrated for *liveness*; phase 5 should add the P4 nuance → C5. |
| R2 | **Consumption receipts OUT** (flush-paced `.pushed` only) | `MObs` = `.act`/`.pushed`/`.delivered`; `.pushed` recorded to the pusher at flush, `.delivered` to the receiver; no remote-drain observation exists (Basic.lean:94–98, 223–253, 315–324) | EXACT | Verified against every `recordObs` site. |
| R3 | **Theorem domain = `.impl` + margin-0**; `.full`/schedulable port recorded [open] | Every statement of record takes `.impl` and (where a class hypothesis exists) `hwf` + `hm0 : ∀ sc, dCount ≤ capLevel`; `margin0`/`margin0_sound` bridge (Instances.lean:74–92) | EXACT | Swept: no `.full` mux statement exists (the only `.full` in scope is base-model `maximal_run_terminal_d5`, correctly so). The [open] port is recorded in MUX-ADJUDICATION T7, not silently claimed. |
| R4 | **Capacity in messages (= replies), with the §5A W = 1 byte-soundness caveat stated in EVERY positive theorem's docstring** | Present: `oracle_deadlock_free` (Necessity.lean:52–56), `oracle_deadlock_free_of_muxInv` (Oracle.lean:631–633), harness module doc (Basic.lean:17–20), Elastic module doc (Elastic.lean:28–30). **Missing**: `sigmaStar_deadlock_free` docstring (SigmaStarLive.lean:412–421 — the T4 flagship), `necessity` docstring (Necessity.lean:63–69 — contains T5's positive conjunct), `elastic_deadlock_free` docstring (Elastic.lean:482–493 — module doc only), `wedge_sigmaStar_deadlock_free`, every positive completion pin (`smokeChain_mux_completes`, `wedge_idler_completes`, `wedge_unboundedSlot_completes`, `wedge_elastic_completes`, `piWedge_oracle_completes`, `wedge_oracle_completes`, `smokeChain_oracle_completes`, `smokeChain_sigmaStar_completes`), and the C1 refutations (`c1_omniscient_false`/`c1_literal_false`, which rest on σ*'s liveness — precisely where §2.5's "liveness claims are weaker than byte reality" bites) | **VIOLATED** for T4 and T6 | The ruling says "every positive theorem's docstring", verbatim. The single most important positive theorem of the campaign (T4) does not carry it, and neither does the trichotomy conjunction. → finding W1. The pins and refutations are a defensible lower tier ("theorem of record" reading) but phase 5 should sweep them too. |
| R5 | (implied by §2.6 / AUDIT-NOTES A7) **Capacity monotonicity consumed by no theorem** | Verified: the only capacity-flavored transfer in the suite is the pipe-length guard monotonicity (`enabledPushes_agree`/`firePush_agree`, WcImpossibility.lean:80–92), proven inline about `length < ·` — not the base model's channel-capacity monotonicity assumption | EXACT | A7's "keep it that way" holds. |

## Table D — §4 findings entries and §5 log claims vs the artifact

| # | Claim (§4/§5) | Artifact | Grade |
|---|---|---|---|
| D1 | Track E: `oracle_deadlock_free` + `necessity` kernel-checked, sendProj consulted only through own flush count | Oracle/Order.lean:49–63 (`oracle` reads `(pushHeights tr).length` only), `OracleInv` (Order.lean:305) | EXACT |
| D2 | Track E integration (1): `MuxInv` repair resolved to F's shape (`allChans` guards + `pushed_mem`); E's `pushed_real` survives as derived lemma with `delivered_real` | Chase/Ground.lean:136–152 (guarded fields + `pushed_mem`), :196–205 (`pushed_real` derived; `delivered_real` follows), integration note :128–135 | EXACT — matches AUDIT-NOTES A11 verbatim |
| D3 | Track E integration (2): one preservation sweep — `sinv_reachable` (∀-strategy), `muxInv_reachable` as its `MuxInv` projection, consumed verbatim by T5's assembly | SigmaStarInv.lean:1653–1681 (both strategy-generic: `{σI σR}` implicit ∀); Necessity.lean:60–61 consumes it | EXACT |
| D4 | Track E integration (3): `Gen.demandOrder` → `Gen.piOrder`; theorem-bearing `demandOrder` in Oracle/Controls.lean | Gen.lean:189–194, Oracle/Controls.lean:62–79 | EXACT |
| D5 | Track E: `static_oracle_jams` on `piWedge` = `genSkelM0 2859` materialized, 19 scopes, C = 1; muxprobe's 240-scope `rand2` stays executable-tier; T9 controls incl. behavioral form and the initiator-side observation | Oracle/Controls.lean:84–157, 194–244; module doc records the witness swap and the searched-not-found observation | EXACT |
| D6 | Track F: `sigmaStar_deadlock_free` zero-sorry, every C ≥ 1; `closure_coverage` stage-indexed by τ, no saturation lemma; proof never consults shared-FIFO order (slot E2 only) | SigmaStarLive.lean:278–408, 422–519 | EXACT (E2-only discipline visible in `istepOk`'s E2 branch and the head-cycle arguments) |
| D7 | Track F: `C1Statement` "minted verbatim from §1" | C1.lean:79–84 | EXACT modulo the three adjudicated deltas (Table A1) — "verbatim" is fair given the module doc discloses each |
| D8 | Track F: `wedge_evidence_starves` [decide] pins the Inevitable closure as load-bearing | C1.lean:144–161 + `sigmaEvidence` (SigmaStar.lean:145–153) | EXACT for the moral; **witness differs from the adjudicated one** — T4's control table chartered "an all-M instance"; landed on `wedge`'s provision wall (silent R-supply runs, same invisibility mechanism). Disclosed in the docstring; recorded here → C4 |
| D9 | Track F: `sigmaStar_local` [open], gap stated precisely in C1.lean's module doc (`wireHeights`/`committedInHist`/`rootH` LocalEq-invariant; `inevitable`/`scheduleE` not shown) | C1.lean:17–29, near-verbatim | EXACT |
| D10 | Track A (§5): T3 full ∀C via four anchors; T1 `commit_totality` Mathlib-free spelled-out uniqueness; controls `wedge_not_deadlockFree`, `wedge_idler_completes`, `wedge_unboundedSlot_completes`, C = 0 vacuity, F8 pins; A9 boundary finding (`gadget_not_wellFormed`) | WcImpossibility.lean:506–585, CommitTotality.lean:339–361, Mux/Controls.lean throughout | EXACT — CommitTotality matches the T1 template with `uncommittedPhase2` unfolded to `phase = 2 ∧ committed = none` and `∃!` spelled out; Controls matches A9 |
| D11 | Track G (§5): `rho`, `rho_decreases` (23-case, `asmLevelsOk` only), `terminating` (≤ ρ(init)), `maximal_run_terminal`(`_d5`), `greedy_run_terminal` (explicit fuel ρ(init)); run-level corollaries hypothesis-free | Proofs/Termination.lean:689, 1036, 1431–1435, 1508–1546 | EXACT — matches AUDIT-NOTES A1's RESOLUTION paragraph clause by clause |
| D12 | Track G (§5): `elastic_deadlock_free` — unbounded parking, `EWorkConserving` widest honest class, every C ≥ 1; adjudicated "projects to reachable base state" phrasing FALSE, honest route = `InvPW` weakening; `EMuxInv` preservation carried as explicit hypothesis, `eMuxInv_init` base case; `wedge_elastic_completes` pin | Elastic.lean:156–159, 191–237, 494–517; the `hinv` seam and its rationale are in the docstring and module doc | EXACT — the honest-residue disclosure (explicit `hinv` hypothesis) is properly stated at the theorem, not hidden |
| D13 | Track G (§5): `wc_impossibility_K` — KR ∈ {1,2,3} anchored, ∀ KI ≥ 1 genuine, ∀ C ≥ 1, `wedgeW (KR+5)`, 12 anchors, `deliverStepK_one` degeneration; KR ≥ 4 open at theorem tier | WcImpossibilityK.lean:560–660 (statement hypothesis `hKR : KR = 1 ∨ KR = 2 ∨ KR = 3` — the honest anchored form, not a ∀KR overclaim), :109–139, anchors :487–542 | EXACT — the docstring states the open KR ≥ 4 residue |
| D14 | Stage 1 (§5): `wedge_bottomMostReady_jams` kernel-decided at C = 1, "also verified jamming at C = 2" | Instances.lean:192–196 is C = 1 only; no C = 2 artifact exists | C = 1 EXACT; **C = 2 clause has no artifact** — a dev-time [checked] claim reading as kernel in context → C6 |
| D15 | Track C (§5): muxprobe golden 252-row matrix, `just muxprobe` gate | `formal/lean/muxprobe-expected.tsv` = 252 rows; justfile:200–201 | EXACT (executable tier; not re-run here) |
| D16 | Phase-1/track-D Rust-bridge claims (wedge realizability, LocalEq soundness, B5, A10) | Rust tier — out of this role's scope; not re-verified | not graded |

---

## Findings, ranked

### "This is wrong" grade

- **W1 — The byte-caveat ruling is violated by the flagship.**
  MUX-PROGRESS §1:120–123 rules the §5A W = 1 byte-soundness caveat
  "stated in every positive theorem's docstring." It is absent from
  `sigmaStar_deadlock_free`'s docstring (Mux/Proofs/SigmaStarLive.lean:412–421)
  — the campaign's centerpiece positive theorem — and from `necessity`
  (Mux/Proofs/Necessity.lean:63–69), which contains T5's positive conjunct.
  `elastic_deadlock_free` (Elastic.lean:482–493) carries it only at module-doc
  altitude. Tail (lower priority, same sweep): every positive completion pin
  and the two C1 refutations, which rest on σ*'s liveness — exactly the
  direction §2.5 says does NOT transfer to bytes for free. One-line fix per
  docstring; phase 5 should sweep.

- **W2 — Both ground-truth docs misdescribe the kernel wedge witness.**
  MUX-PROGRESS §1:80–82 ("`wedge`, the regression shape at w = 4, rootH = 6")
  and MUX-ADJUDICATION §1.2/T0 ("w = 4 … fan ≥ 5") describe the probe's
  minimal instance; the landed `wedge` (Mux/Instances.lean:51–62) has **six**
  provisions and root fan **7** (the committed-regression shape,
  `regression_shape(provisions := 6, rootH := 6)`). The theorems are
  unaffected (∃-witness, and the landed shape is inside the class), but the
  design-of-record's description of the statement of record is numerically
  false, and phase 5's provenance docstrings would propagate it.

- **W3 — §1's slot-peek ruling clause is stale against stage-0 P4.**
  MUX-PROGRESS §1:113–116 still says "the no-peek variant is plausibly false
  via the two-height mutual proof-starvation gadget." Stage-0 P4 reversed
  this at [checked] tier (no-peek causal σ* Terminal 3,470/3,470 including
  the F2 family; STAGE0-GATES.md:17; §5's own log entry records the
  reversal: "slot-peek stands as a modeling decision, NOT a demonstrated
  liveness necessity"). §1 is the ruling of record and maintains
  superseded-markers (the C2 bullet has one); this clause needs one.

- **W4 — §1's C1 bullet carries a stale epistemic status and hides the
  live residue.** The bullet (MUX-PROGRESS §1:67–78) is marked "[derived,
  two named conditions]" with conditions A (keystone repair) and B (stage-0
  gate) — both since DISCHARGED — while the residue that actually
  conditions the kernel refutation (σ*'s locality, the A_p-sufficiency
  hypothesis of `c1_literal_false`, C1.lean:104–114) is named only in §4.
  A reader of §1 alone concludes C1-literal-false is condition-free; the
  merged artifact says otherwise. The bullet's "σ* … is deterministic and
  local" is likewise kernel-unproven for the merged σ* (omniscient-closure
  formulation). One superseded-marker paragraph fixes it (and can note the
  σ*-causal track in flight).

- **W5 — The work-conserving class has no kernel inhabitant, and the
  docstring that promises one is stale.** MUX-ADJUDICATION §2.4 mandates
  `bottomMostReady_wc` and `bottomMostReady_local`; neither exists anywhere
  in the tree (grep-verified), and Mux/Instances.lean:142–144 still calls
  them "stage-2 obligations" after stage 2 closed. Consequence for
  statement strength: `wc_impossibility` (WcImpossibility.lean:560),
  `necessity`'s first conjunct, and `wc_impossibility_K` quantify over
  classes (`WorkConserving`, `KWorkConserving`, also `EWorkConserving` for
  the elastic positive) whose non-emptiness is proven nowhere in the kernel
  — the ∀-statements are not *pinned* non-vacuous, and the claim that the
  shipped policy is "the pinned concrete instance" of the indicted class is
  unproven (only its jam is pinned, via the class-independent
  `wedge_not_deadlockFree`). The intended one-lemma fix: prove
  `bottomMostReady` (or a trivial always-push-head strategy) is
  `WorkConserving` — at reachable states `HistInv.hand_count` ties
  `committedInHist` to `holdsWire`, so the pieces exist.

### "You might want to think about this" grade

- **C1 — T5's provenance label contradicts its own module doc.**
  Necessity.lean:47–48 and Oracle.lean:623–626 call `oracle_deadlock_free`
  "the state-feedback fallback form of record" in the same breath as "the
  send-projection pusher"; §4's track-E finding is precisely that the
  fallback "needs no state feedback." The label means "the adjudication's
  fallback *slot*," but reads as a description of the object. Phase-5
  rewording: "the adjudication's fallback slot, realized as the static
  send-projection pusher (no state feedback needed)".

- **C2 — Adjudicated controls silently not landed.** The skip-scan demux
  control (MUX-ADJUDICATION §2.2, T3 controls), the `prov C` secondary
  family (T0, T3), and T4's σ* pin sweep (jam+m0, pdelay+m0, pyramid
  margins, prov 1..3 — only `smokeChain` landed) are absent. None is
  load-bearing (T4 itself subsumes the σ* pins for truth; the elastic
  semantics subsumes one demux variant), but the adjudication chartered
  them, and no §4 entry records the drop as a decision. Either land them
  cheaply or record the negative space.

- **C3 — `WorkConserving`'s state universe is `MReachableAny`,** i.e. the
  class demands push obligations even at states only *other* strategy
  pairs can reach (Strategy.lean:88–117). The forced-run replay consults
  the hypothesis only at own-pair-reachable states, so the theorem holds
  verbatim for the larger (weaker-hypothesis) class "WC along its own
  runs" — a free strengthening phase 5 could take or document. Same shape
  for `KWorkConserving`; `EWorkConserving`'s elastic universe is
  deliberate and documented.

- **C4 — The evidence-starvation control's witness differs from the
  adjudicated one** (all-M instance → `wedge`'s provision wall;
  C1.lean:138–161). Same mechanism (silent consumptions unprovable from
  bare evidence), disclosed in the docstring; worth one clause in §4 so
  the adjudication's control table reconciles.

- **C5 — "Peek is load-bearing" docstrings need the P4 nuance**
  (Basic.lean:90–93, Strategy.lean:17–19): load-bearing for the landed
  coverage *proof*, not demonstrated necessary for *liveness* (P4).
  Phase 5's provenance pass should say which.

- **C6 — §5 stage-1 log: "also verified jamming at C = 2"** for
  `wedge_bottomMostReady_jams` has no artifact counterpart (kernel pin is
  C = 1 only; Instances.lean:192–196). Harmless if read as a dev-time
  [checked] note; the log should tag it so.

- **C7 — `necessity`'s positive conjunct is pinned at C = 1**
  (adjudication-template-faithful). Since T5 gives every C ≥ 1, phase 5
  could state the conjunct ∀C and keep C₀ = 1 as the headline corollary —
  cosmetic strengthening only.

### Examined and found clean

- `C1Statement`/`C1StatementOmniscient` minting vs the charter (all deltas
  adjudicated and disclosed; the 1 ≤ C guard strengthens the refutation).
- `wc_impossibility` vs the adjudication T3 template: statement identical
  (∀C ≥ 1, ∀ WC pairs, no locality hypotheses, fixed witness); the forced
  run's singleton-consultation claim is real (`forcedPush` fires only on a
  singleton `enabledPushes`); the ∀C two-shape split is as documented.
- `wc_impossibility_K` vs its log entry: honest anchored `hKR` disjunction
  (no ∀KR overclaim), genuine ∀KI ≥ 1, KR ≥ 4 residue stated in the
  docstring, `deliverStepK_one` degeneration proven.
- `oracle`/`sendProj`/`OracleInv`/`static_oracle_jams`/`piWedge` vs the
  superseded C2 resolution: exact, including the receive-projection
  retention-as-refuted-candidate and the `Gen.piOrder` rename.
- `commit_totality` vs T1; `keystone`/`chase` vs T2's repaired form
  (push-time derivation trees; delivery case unstatable by construction;
  `MuxInv`-interface posture as planned).
- `sinv_reachable`/`muxInv_reachable` strategy-genericity; Ground.lean's
  A11 repair (guarded `delivered_eq`, `pushed_mem` field, derived
  `pushed_real`/`delivered_real`) — matches §4 and AUDIT-NOTES A11 exactly.
- Termination.lean vs AUDIT-NOTES A1 resolution — clause-by-clause exact,
  run-level corollaries hypothesis-free as claimed.
- Elastic.lean vs its log entry, including the honest `hinv` seam stated
  at the theorem and the reply-denomination note.
- Slot-peek and receipts-out rulings in `MObs` (R1, R2); every `recordObs`
  site checked.
- `viewEnc`/`LocalEq` vs the adjudicated role-dependent fooling alphabet:
  `Kind` has only D and R, so "M-absent" holds by construction; asker sees
  D + R, answerer D only ("free insertion of R"); leafReqs erased on both
  sides; nondegeneracy + both non-locality pins + the behavioral pin landed.
- Domain sweep: every mux statement of record on `.impl` + wellFormed +
  margin-0; no `.full` claims; `margin0_sound` bridge correct.
- Capacity-monotonicity abstinence (A7): only pipe-length guard transfer,
  proven inline.
- F8 control suite vs AUDIT-NOTES A9 (gadget deliberately ill-formed and
  pinned as such; both bogus verdicts + the rejection replay).
- H-c demotion: no quantitative overlap/latency claim anywhere in Lean;
  explicit NOT-claimed notes present.
- muxprobe golden matrix is 252 rows with a justfile gate, as recorded.

## Handoff notes for phase 5 (legibility)

1. The byte caveat wants a single canonical home (a `Mux/Statement.lean`
   scope section) with one-line pointers from every positive docstring —
   that pattern satisfies R4 without 15 duplicated paragraphs.
2. When Statement.lean mints reader-facing names, fix W2's numbers at the
   source (wedge = 6 provisions, fan 7) and W1/C1's provenance labels.
3. `c1_literal_false`'s hypotheses will read strangely once σ*-causal
   lands — coordinate with that track before freezing names
   (`sigmaStar` vs `sigmaStarCausal` in the audit surface).
4. W5's inhabitation lemma (`bottomMostReady_wc`) belongs next to the
   class definition and should be cited from `wc_impossibility`'s
   docstring as its non-vacuity certificate.

---

# ADDENDUM — round-5 causal-track rows (2026-07-21)

The table body above is verbatim-frozen: it records the pre-repair
state phase 4 reviewed, and explicitly excluded the σ*-causal track
("in flight elsewhere, NOT reviewed"). This addendum extends the audit
with that track's statements of record — merged at `fbab795e`,
reviewed by round 5 (MUX-PROGRESS §4's round-5 entry) — and records
the grade revisions the merge obsoletes. Grades are POST-round-5-sweep
(the R5-3/R5-8 docstring fixes are in); at the pre-sweep merge HEAD
the E2/E3/E6 docstrings graded WEAKER-described (byte pointer and
"completes" convention missing — R5-3/R5-8, doc-tier only).

## Table E — the causal statements of record

| # | English claim | Lean statement(s) of record | Grade | Notes |
|---|---|---|---|---|
| E1 | C1 at the charter grain (the F3 statement of record): every capacity C ≥ 1, every deterministic charter-local pair has a killer skeleton in the shipping encoder's class | `C1StatementCharter` (Mux/Proofs/C1.lean); refuted by `c1_charter_false` — NO hypothesis | EXACT | The new A1-grade anchor. Minting deltas identical to A1's (adjudicated and disclosed in the module doc). The refutation witness is ⟨1, σ*-causal, σ*-causal⟩ with locality and liveness both kernel-proven. |
| E2 | σ*-causal is deadlock-free, unconditionally: every well-formed margin-0 session, every C ≥ 1, `.impl` | `sigmaStarCausal_deadlock_free` (Mux/Proofs/CausalMint.lean, the foot) | EXACT | The positive flagship at the grain of record. Docstring carries the two-kernel-facts "completes" (this theorem + `mux_terminating`) and the byte pointer (round-5 sweep). |
| E3 | Conditional form: deadlock freedom given the Step-4 coverage conjunct | `sigmaStarCausal_deadlock_free_of_coverage` (Mux/Proofs/CausalLive.lean); conjunct `CausalStuckCoverage` | EXACT | Interface theorem; the conjunct is verbatim T8's "inference progress" conjunct, kept as a named Prop because the window-sliding argument consumes it at exactly that interface. |
| E4 | Locality at the charter grain: σ*-causal invariant across skeletons with equal announced views at `.impl`-realizable observations, both parties | `CharterLocal` (Mux/Causal.lean), `sigmaStarCausal_charterLocal` | EXACT | Definitional (the strategy's one skeleton read is `aviewOf`); `ConsistentImpl` pins the mode. The grain is INCOMPARABLE to legacy `LocalEq` [derived; per-direction witness pins are phase-5 queue]. |
| E5 | Step 4: at a reachable stuck drained σ*-causal state, a held stream with performed τ-prefix is proven-demanded under the announced closure | `causalStuckCoverage` (Mux/Proofs/CausalMint.lean) | EXACT | The minting ladder + τ-staged coverage induction + closure saturation, as §4's mint entry describes clause by clause. |
| E6 | The causal pins: σ*-causal drives the smoke chain and the wedge to terminal in the kernel | `smokeChain_sigmaStarCausal_completes`, `wedge_sigmaStarCausal_completes` (Mux/Causal.lean) [decide] | EXACT | Executable witnesses behind E2, not its support (docstrings say so post-sweep); their "completes" is the literal `muxCompletes` kernel fact. |

## Grade revisions the merge obsoletes

- **A5** ("EXACT for capacity; WEAKER for 'witness strategy'
  locality"): the WEAKER clause is now DISCHARGEABLE — the
  constructive local witness landed (`sigmaStarCausal`, charter-grain
  locality kernel-proven), so on the post-merge suite the charter
  fallback is satisfied at grade EXACT via E1/E2/E4. A5's row text
  stands as the pre-merge record.
- **A6** ("its kernel form is exactly the σ*-causal A_p-sufficiency
  theorem — in flight"): LANDED, with a shape correction of record —
  the kernel form is not an A_p-sufficiency theorem for the omniscient
  σ* but the re-grounding at the charter grain (E4) plus the
  announced-closure liveness (E2/E5); the legacy A_p-sufficiency stays
  [open] forever as `c1_literal_false`'s internal-artifact hypothesis.
- **B1** ("'local' is not a kernel fact of the merged suite"): true of
  the omniscient σ* it graded, unchanged; the campaign-level claim "a
  deterministic local strategy refutes C1" is now kernel at the
  charter grain (E1). §1's C1-bullet marker closed accordingly
  (round 5, R5-2).
- **R4's tail** ("the pins and refutations are a defensible lower tier
  but phase 5 should sweep them too"): swept — the phase-4 repair
  covered the pre-causal pin and refutation sites (17 pointer sites),
  and round 5 covered the four causal surfaces (R5-3).

Handoff note 3's coordination item is resolved: σ*-causal landed, so
the naming decision for the audit surface (`sigmaStar` vs
`sigmaStarCausal` — candidate: the causal object as the strategy of
record, the omniscient σ* as the proof vehicle) is now decidable and
queued for `Mux/Statement.lean` (phase 5), which must inherit the
round-5 fixes (the incomparability statement, not the retracted
nesting claim; no `PView` ghost).

---

# ADDENDUM 2 — the phase-5 audit surface (2026-07-22)

Phase 5 minted `Mux/Statement.lean`: every statement of record
restated INLINE, fully quantified, with proof by citation, so the
kernel checks that the audit surface IS the claim (drift between the
surface and the proofs fails the build). The tables above map onto it
as follows; grades are unchanged except where noted. All names below
are in the `StreamingMirror.Mux.Statement` namespace.

| Row(s) | Statement.lean name |
|---|---|
| A1 / E1 (C1 at the charter grain, refuted) | `c1_charter` |
| A2 (the no-locality companion) | `c1_omniscient` |
| A3 (C2 positive, T5) | `oracle_deadlock_free` |
| A4 (necessity, T6) | `necessity` |
| A5 / E2 / E4 (the constructive witness) | `sigmaStarCausal_deadlock_free`, `sigmaStarCausal_charterLocal`; the omniscient proof vehicle `sigmaStar_deadlock_free` |
| B2 (C1-WC, T3) | `wc_impossibility`; the K-parking half `wc_impossibility_K` |
| (the F5 completion companion) | `mux_terminating` |
| (the elastic endpoint) | `elastic_deadlock_free` |

Grade revisions this phase:

- **E4's incomparability tag is retired**: the "[derived;
  per-direction witness pins are phase-5 queue]" clause is now
  kernel-pinned at relation AND class strength
  (Mux/Proofs/Grains.lean: `legacyEq_announced_differ`,
  `announcedEq_legacy_differ`; `announcedLeafProbe` charter-local but
  not legacy-local, `viewProbe` legacy-local but not charter-local).
- **The charter grain gained its mandated non-triviality controls**
  (the round-5 consider #2 gap): `charterView_nondegenerate` and
  `oracle_not_charterLocal` — the R5-1 consequence ("the oracle's
  nonlocality at the grain of record is established nowhere") is
  closed.
- **The executable twins are pinned** (`piOrder_eq_demandOrder`,
  `wedgeFam_eq_wedgeW`, `applyU_eq_applyE`, Mux/Proofs/Twins.lean),
  and muxprobe's self-test pins `piWedge` = `genSkelM0 2859` per run —
  the D5 row's provenance assertion now has an executable check.
