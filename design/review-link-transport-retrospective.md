# link-transport review campaign: retrospective

**Dates**: 2026-07-23 → 2026-07-24. **Documents of record**:
design/review-link-transport-branch.md (the 82 findings),
design/review-link-transport-execution.md (rulings, branch inventory,
integration plans, environment lessons — the detail behind everything
summarized here). This document distills what the campaign taught; it is a
lessons artifact, not a change log.

## Shape and outcomes

A multi-agent skeptical review of the link-transport branch (82 findings,
R1–R82, every one ruled by Finch), executed as three waves of worktree-agent
fixes, each branch integrating only after passing an independent skeptical
review, with fix rounds iterating until clean. Landed across the waves:

- All 82 findings fixed, deliberately declined, or explicitly parked with a
  recorded ruling — none silently dropped.
- The opening-supply symmetrization (election by set size + supply-only
  opening), a deliberate wire change with every snapshot re-pin derived
  frame-by-frame in review.
- The window-pricing exactness package and the budget-is-policy rework:
  slot constants derived from `size_of`, the 512 MiB chosen default, the
  solve-derived tradeoff table (after a validation probe caught the
  closed-form table understating slowdown 1.33–1.45× in its constricted
  band), and the F-decomposition verified against the solve's arithmetic.
- The Bootstrap builder with the bookmark type-state and the
  budget-wire-invariance pin.
- Deterministic execution for the delay-sweep suites: the flake mechanism
  was the differencing readout (wall noise read as phantom hops), not the
  paused clock; the exact-virtual-time readout let every bound tighten and
  the nextest exclusivity retire. The wave-3 gate's window suites passed
  first time under full fleet load.
- The order-indifference metatheorem: `Sched.deadlock_free_anyOrder` and
  its wide variant, kernel-checked over the two-point per-loop dequeue
  class (~21.4k Lean lines, zero edits to existing proof files), resolving
  MODEL.md §5.1's divergence with the reply-first residue recorded.

## Practices that carried the campaign (keep doing these)

- **Spec-first, probe-first, pin-forever** (the formal track): the English
  statement of record was committed before any Lean; an executable BFS
  harness — calibrated in both polarities against kernel-pinned deadlocks —
  validated the theorem's truth and located the reachable-but-harmless
  configuration before induction effort was spent. The flagship landed
  verbatim to the scoped statement.
- **Statement-faithfulness as the review criterion**: claims exactly as
  strong as proven, in both directions. It caught real errors at every
  altitude: a "Measured" tag nothing measured, an "exactly" false at one of
  three cells (a corpus-denominator subtlety), "definitionally" where one
  arm was propositional, a validity caveat understating its own band by
  3–10×. The companion tone rule: a good approximation with a stated band
  is a calibration, not a failure narrative.
- **Every check gets a negative control.** The campaign's controls caught
  their own kind of bug repeatedly (the always-equal comparator, the dead
  instrument, the dipping cost function, the vacuous ceiling pin).
- **Reviews audit reports as part of the record.** Multiple agent reports
  contained overclaims a reviewer falsified against the tree; two reviewers
  independently re-derived entire arithmetic chains (one transcribed the
  window solve into Python and reproduced all 32 table cells).
- **Deterministic pins over timing wherever expressible** — hops, frames,
  bytes, counts. Every wall-clock assertion was eventually a flake or a
  finding; every deterministic pin survived fleet load byte-identically.
- **Any quantity computable two ways gets a pin comparing them.** The
  closed-form/solve drift was exactly the seam this rule closes; the
  campaign ended with the table byte-compared against the solve, the
  crossover pinned against the derivation, and the envelope probed against
  measured hops.
- **Iterate reviews until clean, with focused re-checks scoped to the fix
  range.** No fix round needed more than one iteration; review cost stayed
  proportional to the diff.

## Failure modes and their counter-rules (environment)

Full detail in the execution ledger §6; the durable rules:

- **Worktree spawn base is not guaranteed to be HEAD.** Three agent
  worktrees spawned at a stale ancestor. Counter-rule: every worktree brief
  opens with base verification (fast-forward if ancestor, stop if
  divergent).
- **Event-driven agents strand when their wake source dies.** Background
  gates, builds, and pipes died with the environment repeatedly; a
  47-minute "run" with 0.4 CPU-seconds was a pipe-blocked orphan.
  Counter-rules: liveness = child-process attribution (cwd via lsof, CPU
  burn) plus transcript growth over two samples, never claims or mtimes;
  final steps run foreground; a queued message to a parked agent can
  deadlock against its dead waiter — kill the blocked child so the harness
  wake delivers the message.
- **Message mis-routing across agents happened twice** (adjacent
  recipients). Counter-rule: verify the recipient pin on every cross-agent
  send.
- **The machine is never quiet.** Measurements must be load-tolerant by
  design; flake attribution requires byte-identical binaries and a base
  control run before blaming a diff; unexplained failures during load
  spikes warrant a clean rebuild first (poisoned incremental artifacts
  produced impossible failures until `rm -rf target`).
- **Merge-seam re-sweeps stay mandatory** even at zero textual conflicts:
  wave-2's sweep caught twelve stale Lean citations and a resurrected
  design-doc citation that no per-branch review could see.

## What the campaign changed about the codebase's epistemics

The docs now carry their own evidence: every stated number is labeled
derived, measured, calibrated, or chosen; the tradeoff table regenerates
from the production solve and is gate-compared byte-for-byte; the shipping
dequeue order is kernel-certified over its whole design class rather than
one transcription; and the test suite's measurement tier is deterministic,
so a future flake in those suites is a finding, not weather.
