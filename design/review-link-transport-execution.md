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
  AGENTS.md by the escalations branch. **Open sub-question**: whether
  MODEL.md-*section* citations are permitted (escalations agent's read: yes,
  spec-of-record; ~8 sites in streaming tests/skeleton.rs affected if Finch
  rules stricter).
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
2. **Lean query-first repair (option 1)** — **AWAITING FINCH GO/NO-GO**.
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
   R10 (backend/tests.rs design-citation), R12 (PRICED_HEADER premise +
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
4. **Low-prio ledger**: swarm's above-target population offset (session-
   coupling effect, wrong-sign vs propagation-lag model; needs quiet
   characterization); R11's timing residual on real transports (admitted in
   "cannot see"); R32/R70 load-tolerant retuning if sweeps flake at the final
   gate (mechanism: paused-clock auto-advance manufactures idle under OS
   descheduling — durable fix is exact-advance or single-threaded
   deterministic executor for those pins).

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
- Poisoned incremental artifacts: the SIGKILL/disk-full chaos left corrupt
  incremental state that produced impossible test failures (degenerate inf,
  phantom multi-suite failures) tracking innocuous diffs; vanished on
  rm -rf target + clean rebuild. Consequence: the authoritative
  post-integration gate MUST run on a clean-built target, and unexplained
  failures during load spikes warrant a clean rebuild before investigation.
