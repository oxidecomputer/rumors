# link-transport review: execution ledger

**Purpose**: session-survivable state for the review campaign driven from
design/review-link-transport-branch.md (the findings document, R1–R82). This
file records rulings, branch inventory, the integration plan, queued work, and
open decisions, so a fresh context can resume the campaign without the
conversation. Both files are uncommitted working artifacts; commit together at
close-out. Base for all branches: link-transport @ 22b61c02.

## 1. Ruling summary (Finch, 2026-07-23)

All 82 findings ruled. Dispositions not individually restated here; the
categories: fixed-in-branch (most), queued to the deferred wave (§5), declined
(R6 — pre-release; becomes binding at first release), subsumed (R24 by the R4
branch; R69 by R5's fold-and-delete; R29 by the deterministic-table decision),
question-resolved (R59 fix-both, R79 builder API queued, R80 bound derived,
R81 fixed, R82 amended). Standing rules issued during triage:
- Every conformance check gets a **negative control** proving a nonconforming
  subject is caught.
- Design-doc citations from code: fix all, fold content inline (do not
  re-report; the ban stands).
- Code may cite **Lean theorem/definition names** (abstractly, never file
  paths); never proof-progress documentation (formal/PROGRESS.md). Codified in
  AGENTS.md by the escalations branch. Sub-question RESOLVED (Finch,
  2026-07-23): code never cites MODEL.md either — cite the Lean artifact
  directly by theorem/definition name, or state assumed premises inline; a
  worktree agent is re-denominating the 9 affected sites and the AGENTS.md
  policy line.
- Measurement doctrine: this workstation is never quiet; measured pins must be
  load-tolerant by design (floors/best-of-N, nextest `threads-required`
  exclusivity, virtual-time exactness, or full determinism). Deterministic
  pins (hops, frames, bytes, kernel checks) preferred wherever expressible.

## 2. Branch inventory (all on 22b61c02 unless noted)

| Branch (worktree-agent-…) | Content | Head | Status |
|---|---|---|---|
| ac265cf2b7181346d | R4 aggregate repair + meet lemma | 206c288a | Reviewed clean; integrates via the async-leaf stack |
| a7becdd6c6426fce7 | Async Leaf::leaf + priced pipeline + R81 + ~15 triage items (stacked ON 206c288a) | 4add7f8a (8 commits atop 206c288a) | Reviewed, SEALED (micro-round landed: corners in nextest filter, re-arrival clause scoped, FAN_SLOT_BYTES provenance exact) |
| ab2d31b7d003ca9fa | Bookmark soundness (reclaim collision, F1 cancellation seam, R53/R54) | ccd88401 | Round-2 clean, integration-ready |
| a81e27ae3faeb9a81 | Link contract + conformance probes + R80 bound + R3 arm deletion + R40/R42 | 9c184576 | Reviewed, sealed |
| ae554b9cc584bc02b | Codec atlas + guard (R56–58) + R59/R71–74/R76 | a53f056e | Reviewed clean (2 nits → punch list) |
| a1a45996c885939fe | R7 target-size tests + R19 liveness cells | a50fe57c | Round-2 clean |
| a29bbd9522eb873bd | Ghost-ref/citation sweep (18 files, prose-only) | 023546e4 | Mechanical verify at integration |
| a22be085edf7de181 | §6 design-doc repairs (R64–67) + R82 tracecheck amendment | 0996fd41 | Mechanical verify (re-run SHA ancestry + number cross-refs) |
| aa0a923f39f842a99 | Doc-accuracy cluster (R46/52/60/61/62 + paths) | 4d179d80 tip (post-seal prose fix by Finch: wire-snapshot pin citations at their real module path; inspected and ruled in) | Mechanical verify (eyeball the two tools/readme regexes) |
| a8181af6d4c8b6964 | Swarm: drop guard + rate + hysteresis fixes | d987a2be | Mechanical verify (red-green test is its evidence) |
| a724b7affd783f7b9 | Tools: R25/26/27 + R77 + AGENTS.md + trap + witnessed red | ccbcede8 | Reviewed, sealed |
| ad36fd51bbcc1487c | Knee per-cell placement + parking coexistence pin + 7/8 boundary fix | f4d7c57d | Reviewed, sealed |
| aa930d0f65b72a5c6 | R21: tcp link through conformance suite (passes; harness buffer knob) | fabdd4f | Sealed (test-only; mechanical verify) |
| a42fd5b93de79d500 | R44: docs.rs metadata + gated-link fixes | e38153f6 | Sealed (doc/metadata only) |
| a013eabd581b510bd | Escalation rulings + Lean-citation policy + full MODEL.md audit | 9489a085 | Sealed; contains the §5.1 OPEN DIVERGENCE record |
| ad82ac09757669272 | examples/envelope_sim.rs (Python port, byte-identical manifests) | 3824c775 | Sealed |
| a98db1c81d098b995 | R30 hash bench + hash.rs claim re-grade (~2x confirmed: 1.7–2.5x, never slower; bench named in the claim) | 3e912ce8 | Sealed |
| a98fce0ef969241b0 | design/opening-supply-symmetrization.md | 2647c58b | Done (rides integration; brief for the symmetrization fork) |
| a801d0785a4626f93 | Lean scoping (census delivered; no repo changes) | — | Report-only; timing footnote outstanding |

## 3. Integration plan

One agent, sequential, function-level review at any conflict resolution, then
a merge-seam re-sweep over every seam (mechanical ghost/citation greps +
ledger checks), then ONE `just gate` on a clean-built target with the fleet
drained (Finch's ruling: `just all` is not required for integration).

**Order** (behavioral first, prose last): async-leaf stack (contains R4) →
bookmark → link → codec → escalations → knee/parking → tools → R21 → R44 →
R7/R19 → swarm → sim-port → sweep → §6 docs → doc-accuracy → symmetrization
design doc.

**Known seams**:
- gossip.rs: bookmark × link accessor changes — verified disjoint and
  compiles (link reviewer checked).
- peer.rs: async-leaf budget-paragraph rewrite × R44's two `Protocol::V1`
  de-linkings — token-level; re-apply R44's fix if async-leaf's text wins.
- message.rs "≈ 2.2 MB": async-leaf claims done; knee branch flagged it —
  verify once, dedupe.
- Cargo.toml: R44 metadata block + hash-bench entry + swarm `[[example]]`
  stanza — three disjoint regions, trivial merges.
- AGENTS.md: tools' line-21 reword + escalations' policy line — different
  hunks.
- R7 binding test's role-attribution comment: correct per the symmetrization
  investigation (initiator/responder labels inverted vs protocol roles).

**Punch list** (apply on the integrated base, one cleanup commit):
1. codec/capture.rs:19 `FRAME_LEN` → `framing::LENGTH_HEADER_LEN` (pre-existing dup).
2. Atlas module doc: half-sentence on stale exemptions for deleted variants.
3. erased.rs:8-11: two allocations per open (not one); add the +0.7 GiB/tower
   figure inline as the stated reason the erased funnel is load-bearing.
4. materialized/work/levels.rs:56: "the pre-greeting protocol exploded here" ghost phrase.
5. streaming local_eq.rs: normalize Lean-citation short-forms to the AGENTS.md rule.
6. window_knee: optionally add E = C(n,2)/2¹⁶ to the margin constant's doc.
7. README regen (R61 + R44 fixes converge; expect near-no-op).
8. Re-judge window_operator's solo-quiet failure at the final gate (R25 fix +
   guards may cure; else R17/R32 recalibration item). Note: nextest
   exclusivity protects sweeps from sibling TESTS only, not sibling OS
   processes — single-tenant CI is the durable answer for these suites.
   The committed tradeoff table's default-row label still reads correctly
   for the new default (271,397,212 rounds to the same "271 MB"); the
   deterministic hop-ratio generator (§4.3) supersedes any regen obligation.
9. Commit design/review-link-transport-branch.md + this ledger.

**Integration outcome (2026-07-23)**: complete at `2921784c` on branch
`worktree-agent-a75e5ef6962e79c73` (17 merges in order, re-sweep commit,
punch commit, follow-up prose commit for the two residuals + the knee-band
exactness). `just gate` passed on a clean-built target twice (893/893, 2
deliberate skips). One conflict total (README.md; regen confirmed the
resolution byte-correct). Re-sweep caught 12 Lean file-path citations and one
design-doc citation the branch fixes had missed. Landed: `link-transport`
fast-forwarded to `2921784c` (Finch, 2026-07-23); this document and the
findings document committed as close-out (punch item 9).

## 4. Post-integration tracks (all ruled)

1. **Symmetrization fork**: election-bias first (smaller set initiates —
   `set_len` already in greeting), then the supply-only opening, as separate
   reviewed commits. Brief = design/opening-supply-symmetrization.md (step-0
   corrections included: role election is lexicographic byte-luck;
   bootstrap-good-path is an encoding accident; retire-into-bootstrapper
   CONFIRMED decomposed). Wire change; snapshots + hop-trace re-pins are
   deliberate commits.
2. **Lean divergence repair** — Finch directed scoping of OPTION 2 (the
   order-indifference metatheorem) before any go/no-go; a read-only scoping
   agent is assessing it (candidate statements, executable probe of the
   committed-choice candidate cycle at small rootH, effort in option-1
   denomination, interaction/subsumption analysis). SCOPING DELIVERED
   (2026-07-23): recommendation GO on the ord-parameterized candidate A
   (8-12 runs, 2-3 agent-days, ~0.7 confidence), which subsumes option 1;
   simulation route rejected as unsound (stuckness does not reflect).
   Option 1's census below was found INCOMPLETE: five pairing loops flipped
   (walk x3 + terminal absorb + proxy encoder), 8 step constructors, plus
   Mux-campaign import coupling that makes any destructive in-place repair
   poisonous - non-destructive parallel definitions are mandatory. Probe:
   QF deadlock-free on all flagship-class pins; candidate cycle unreachable
   in-hypothesis; sibling back-pressure configuration reachable-but-harmless
   under both orders (witness re-earned from capacity facts). Open box:
   comb6 exhaustive BFS outstanding (random sweeps clean; rMix closed
   exhaustively post-report: ~10M states, stuck=0). AWAITING
   FINCH: go/no-go on candidate A vs B, and the d5-corner scope ruling.
   Census: order lives in 4 places (Sched.lean:106/:258 prologue, 4 step
   constructors, Invariant.lean:33-39 counts, Pending.lean decode tables);
   ~15 trivial definition lines + ~150 mechanical repairs + ~20 genuine
   re-derivations (phase-0/1 witnesses change producer; close-order
   termination lemmas); 2–4 focused agent runs (1.5–3 agent-days), medium
   confidence. Substantive wrinkle: under committed-choice semantics,
   query-first admits a candidate cycle (lower stage waits on `asked` while
   upper stage is committed into the full wire channel); the phase-0 witness
   must be re-earned from capacity facts. Option 2 (order-indifference
   metatheorem from acknowledged-question acyclicity): 2–3× cost, durable;
   recorded as follow-up. Until resolved, `Sched.deadlock_free` certifies the
   OLD loop order — MODEL.md §5.1 carries the dated OPEN DIVERGENCE.
3. **Deferred window/backend wave** (on the integrated base, each reviewed):
   R5 (fold b05 math into window.rs comments, carrying R69's sampled-sweep
   honesty; then delete design/b05-uniformity-envelope.md AND
   design/b05-envelope-sim.py — the Rust example is the tool of record),
   R12 (PRICED_HEADER premise +
   serialization + honest default — must also cover the async-leaf branch's
   new third store in leaf_underpricing_fails_at_construction), R14 (version_bytes equivalence asserts),
   R15 full delegation (leaves/assemble through Charged), R16 (size_of pins
   for REFERENCE_SLOT_BYTES family + the 16→24 correction and envelope
   re-derivation), R17 (DISPUTE_WIRE_BYTES calibration pin), R18 (clamp
   budgets at the u32 framing ceiling), R20 (WindowConfig::default must not
   flip under the additive test-internals feature), R22 (node_bytes
   monotonicity sweep + negative control), R23 (leaf-edge charge/capacity
   same bound; resolve toward the more generous grant within the user limit),
   R31 (saturating window arithmetic), R79 (bootstrap builder API for
   target/budget/protocol), **deterministic tradeoff table** (hop-ratio
   generator: cells = hops_B/hops_∞ from hop-trace on the deterministic link,
   seeded corpora shared per cell pair; retires the 1.0× clamp and turns
   R28/R29 into a byte-compare gate test; one validation run against the
   measured table before the measured methodology retires).
3a. **Default-budget policy rework** (RULED, Finch 2026-07-23; implement as
   the window branch's post-review fix round): DEFAULT_SYNC_MEMORY_BUDGET
   becomes an arbitrary stated policy choice of 512 MiB (536,870,912); the
   compile-time BDP/DISPUTE_WIRE minting expression retires. Docs derive
   behavior in closed form instead: slowdown(budget, m) = max(1, BDP x
   SCOPE_ENVELOPE_BYTES / (budget x (28 + m))), m = mean encoded record
   size, 28 B = the calibrated intercept; spec BDP = 12.5 MB (1 Gbps x
   100 ms and 100 Gbps x 1 ms coincide). Ship a deterministic budget x
   mean-record tradeoff table (byte-compare gate test, per the R28/R29
   plan); DISPUTE_WIRE_BYTES = 200 survives only as the m=172 design-record
   column. Spot check: at 512 MiB, slowdown-1 crossover is m* = BDP x
   ENVELOPE/budget - 28 ~ 85.3 B (docs state m >= 86 B; never round the
   inequality across its own boundary); u64 corpora ~ 3.1x (latency only,
   never memory). Blessed headline (Finch): at the spec BDP, the default
   budget imposes no window-induced serialization for mean encoded record
   size >= 86 B — the in-flight disputes' own transfer covers the RTT. This supersedes the
   deterministic hop-ratio generator's role for THIS table; the hop-ratio
   validation run (one, against the closed form) is retained.

3b. **R8 occupancy-ceiling pin** (queued, small): promote the exploratory
   fan-probe (branch worktree-agent-a6d879fe6a15c3053 @ e7f72447, kept as
   reference until this lands, then delete) into an invariant test: an eager
   source drives reader/assembler channel occupancy TO FAN+1 (regime
   reachable) and never PAST it (the premise under the flat supply-decode
   envelope charge, 209,712 = 257 x 816); deterministic count, probe stays
   test-gated.

**Wave-2 integration outcome (2026-07-23)**: complete; `link-transport`
fast-forwarded to `8b7c4607` (symmetrization 55d76d5c + window 4d0b3db2 +
backend 8d959048 + citation 07f33b0c, all skeptically reviewed to
INTEGRATION-READY, + one integration-pass commit: hop-denominated departure
prose, the symmetrization design's A1-A4 amendments, sync-budget §1.6
consistency, the crossover's fourth input). Zero textual conflicts; seams
verified function-level. Clean-target gate green, 918/918 (first run flaked
on a test whose identity was lost to output truncation under fleet load; two
subsequent full passes; the deterministic-executor task is the cure).
Flagged for Finch: tests/stale_floor.rs:3 "Under the old API" (pre-existing
regression-pin module doc, not merge-introduced).

3c. **Post-wave-2 dispatch set (RULED, Finch 2026-07-23)** — all launch on
   the integrated base without further adjudication:
   - Deterministic executor for the load-sensitive sweep pins (the named
     offenders in §4.4): single-threaded deterministic executor or
     exact-advance, retiring the paused-clock auto-advance sensitivity.
     Own agent, reviewed.
   - Swarm above-target offset: re-observe under the production-default
     regime (post-R20); characterize only if it persists. Own agent.
   - R79 bootstrap builder API: implement in a subagent, BUT the merge
     waits on Finch's approval of the API design — present the design
     with the report before integrating.
   - Small combined agent: R5 (b05 fold + delete doc AND Python sim) +
     the one-shot hop-ratio validation of the closed-form table + R8
     occupancy pin (3b). (R6: untracked by Finch's ruling — the doc is
     honest at first outside read; no change, no follow-up.)
   - Metatheorem (§4.2): Finch pre-authorized merge on completion; still
     gets the standing skeptical review, but no adjudication pause.
   - Wave-2 close-out mechanics (worktree/branch cleanup, ledger commit)
     proceed autonomously as they become applicable.

4. **Low-prio ledger**: swarm's above-target population offset (session-
   coupling effect, wrong-sign vs propagation-lag model; needs quiet
   characterization); R11's timing residual on real transports (admitted in
   "cannot see"); R32/R70 load-tolerant retuning if sweeps flake at the final
   gate (mechanism: paused-clock auto-advance manufactures idle under OS
   descheduling — durable fix is exact-advance or single-threaded
   deterministic executor for those pins). Named offenders observed under
   fleet contention, passing in isolation and uncontended gates:
   window_corners::asymmetric_catch_up_is_ladder_bound_at_the_floor (31 hops
   vs <=24), window_knee::above_the_knee_hops_grow, window_operator and
   window_knee full-suite flakes during the pricing wave.

## 5. Open decisions (Finch)

1. Lean option-1 go/no-go (§4.2).
2. MODEL.md-section citation strictness (§1; ~8 sites).
3. tm-guard re-enable timing: `rm ~/.cache/tm-guard/disabled` activates the
   hardened version (global 60 s spacing, keep-6 manifest retention, pressure
   thinning, record-before-mint, orphan adoption). Recommended: after
   integration completes.

## 6. Environment lessons (retrospective raw material)

- Fork narration capture: the attend hook presses forks to run listeners;
  forks must decline or they swallow live narration (two batches lost early;
  later forks declined correctly).
- tm-guard flood: per-subagent $TMPDIR state ⇒ one snapshot timer per fork;
  no retention; hook 10 s timeout killed mint-then-record scripts post-mint
  (orphan snapshots invisible to retention); APFS local snapshots honor NO
  path exclusions (exclusions govern backups only). Fixed script:
  record-before-mint + orphan adoption + manifest-scoped retention.
- Worktree disk economics: ~15 concurrent worktrees ≈ 530 GB of target/;
  clear-on-completion policy required; deletions are pinned by local
  snapshots until thinned.
- SIGKILLed cargo orphans park on the global ~/.cargo/.package-cache flock →
  invisible convoys (readme-check/cargo-metadata hangs). Diagnose via lsof +
  PPID 1; private CARGO_HOME with symlinked registry is a viable bypass.
- Background-watcher fragility: agents parked on background gate monitors die
  silently with the machine; foreground-only completion discipline for final
  steps, and liveness = process count + transcript growth, never claims.
- Reviewer record-correction norm: three agent reports contained overclaims
  (nonexistent #[ignore] instrument; 8,192 vs 4,096 payloads; "244 GB main
  target" stale) caught only because reviewers audit reports as part of the
  record. Reports are the record — correct them explicitly.
- The flake-attribution discipline that held all day: binaries byte-identical
  to base + a base control run under identical load, before attributing any
  timing failure.
- Worktree spawn base is NOT guaranteed to be the current HEAD: three agent
  worktrees in one session spawned at a stale ancestor (fab65c2c); two
  self-corrected (integration fast-forwarded, window-pricing hard-reset its
  clean tree to the briefed base), one did not (citation branch). Every worktree-agent brief must open
  with: verify HEAD is the briefed base SHA; if it is an ancestor,
  fast-forward before any work; if divergent, stop and report. The
  citation-policy branch (5d135a21) was built on the stale base and needs a
  mechanical rebase onto adbb4462 before integration (routing decisions
  unaffected; its two "Stalled" gate failures are the stale base's, absent
  on the landed head).
- Poisoned incremental artifacts: the SIGKILL/disk-full chaos left corrupt
  incremental state that produced impossible test failures (degenerate inf,
  phantom multi-suite failures) tracking innocuous diffs; vanished on
  rm -rf target + clean rebuild. Consequence: the authoritative
  post-integration gate MUST run on a clean-built target, and unexplained
  failures during load spikes warrant a clean rebuild before investigation.
