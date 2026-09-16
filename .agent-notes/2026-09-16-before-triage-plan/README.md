# A smaller triage for `before`

The [checklist](checklist.md) owns scope, status, dependencies, and the current
queue. The [inventory](inventory.md) records the evidence inspected at the
restart and the inherited work on `main`. [Style](style.md) is the standing
standard for every source file this triage touches. Keep the checklist to
current state and dispositions; implementation history belongs in Git or a
decision note when it will inform later work.

The 2026-09-01 review is evidence, not a verdict. Its aborted triage is useful
only for counterexamples, experiments, and questions to recheck. Its rulings,
ledger dispositions, lane briefs, packets, and prepared branches have no
authority here. Current code, reproduced behavior, and the owner's current
instructions decide the work.

## 1. Choosing work

Keep one implementation active and at most two next candidates. Checklist
groups organize outcomes; they do not prescribe patch size or require that a
whole group finish at once. Each change should be small enough to understand,
verify, and review on its own.

Choose by correctness risk, dependency, and the value of making the next area
easier to reason about. Prefer a partly finished area over opening another when
the risks are comparable. Proximity in the source is a tie-breaker, not a reason
to defer a more important defect.

After every approved commit, re-read the current checklist, new evidence, and
changes on `main` before choosing the next increment. Select the highest-priority
work by the rules above; do not continue a previously named sequence merely
because it was named. Then begin that increment immediately.

Correctness includes the published time and auxiliary-space bounds. They apply
to every valid input size unless investigation shows that a bound cannot be
met. In that case, stop with the construction, the best attainable contract,
and a recommendation; do not quietly weaken the promise or build machinery to
conceal the mismatch.

The scope is `before`, `suanpan`, the detached `before` workspaces, fuelscape,
and the shared tools and recipes that validate them. Organize work around the
behavior of `before`, even when the implementation or instrument lives in a
supporting crate.

## 2. How each change proceeds

Read the affected implementation, its callers, its tests, and the relevant
review evidence. State the concrete problem and the behavior the change should
provide. Check that the old finding still describes the current tree before
designing around it.

Prefer general verification when the claim is a family:

- Use proptests for algebraic laws, boundaries, orderings, schedules, and input
  shapes whose representative examples would leave obvious holes.
- Reuse an independent oracle or an existing meter when it directly observes
  the property. Consolidate overlapping drivers instead of adding another.
- A small, clean fix may use focused existing coverage or a focused regression.
  It does not owe a new framework or a committed demonstration that every
  imaginable wrong implementation fails.
- Tests must make their proof legible: one behavioral claim, a documented
  invariant, direct inputs, and assertions that reach the claimed path.

Simplify the implementation and explanation around every touched change.
Remove redundant states, wrappers, helpers, copies, counters, and claims.
Prefer code whose correctness follows from its representation and control flow
over code whose correctness depends on a parallel roster or a comment.

Audit every auxiliary allocation on a touched path. Relate its peak size to the
encoded inputs that can cause it, including worst-case shapes and small inputs;
a compact Party, Version, Clock, or related value must not induce a
disproportionately large temporary structure. Prefer eliminating auxiliary
representations to widening, duplicating, or instrumenting them. Record any
necessary structure's bound where a maintainer can verify it from the code.

Every external `before` API change stops for owner approval before
implementation. The same batch updates and tests Rumors against the proposed
surface. The hard rule against mixing independently seeded universes applies to
production code, tests, generators, and benchmarks.

Preserve every proptest regression seed. Re-accept a wire or bookmark snapshot
only for a deliberate, owner-approved format change under the repository's
snapshot rules. Run the narrow checks while iterating and `just gate` before
each code commit. Run `just readme` after crate-level rustdoc changes. A
notes-only commit needs no gate.

## 3. Decide whether an instrument earns its keep

The verification system is in scope as code. An instrument is worth its
maintenance cost only when all of these are true:

1. It observes a named failure class that matters to a current contract.
2. Its observation is independent enough to detect that failure rather than
   restating the implementation or another roster.
3. It can fail on the path and input family its documentation claims to cover.
4. A normal repository recipe runs it at the cadence its claim assumes.
5. Its fixtures, pins, and output are understandable without reconstructing a
   historical development campaign.
6. Its code, hooks, dependencies, generated artifacts, and runtime cost are
   proportionate to the distinct confidence it adds.

For each substantial instrument, choose one outcome:

- **Keep:** it catches a distinct failure class and its implementation is
  already clear and economical.
- **Consolidate:** retain the observation but share its generators, oracle,
  driver, roster, or measurement with another instrument.
- **Replace:** a smaller or more direct property establishes the same contract.
- **Retire:** the claim is obsolete, decorative, duplicated, or not usefully
  observable. Remove the whole support path: hooks, counters, fixtures,
  dependencies, recipes, generated data, and prose that existed for it.

Retirement does not require constructing a ceremonial mutant. Name the lost
signal and show where it is still established, or explain why no current
contract needs it. If no replacement catches a meaningful failure, keep the
gap visible rather than describing the old instrument as adequate.

The initial audit pays special attention to the resource-envelope harness, the
amplification board, stack-segment accounting, surface rosters and source
scanners, asymptotic liveness pins, the bench judge, fuzz-fit bands, fuelscape
artifacts, and coverage pins. Their size or sophistication is not evidence of
value.

## 4. Documentation and code quality

Apply [the style standard](style.md) to every definition and passage touched.
Public rustdoc is for a developer using the library: behavior, obligations,
guarantees, errors, and costs. Private rustdoc and comments are for a maintainer
changing it: invariants, representation choices, and reasoning that the code
does not already state.

Every definition receives a useful doc comment, including private helpers,
trait implementations, test helpers, and constants. This minimum does not
justify commentary that merely translates a name or body into English.

Delete history narration, stale names, copied rosters, measured numbers without
artifacts, invented terminology, duplicated contracts, and contorted prose.
Keep the explanation necessary to use or safely modify the code, stated once at
the level that owns it.

## 5. Tracking

The checklist is a completion index, not a work log. Each outcome has a short
description and source references. Status is open, active, ready for review, or
complete. Complete means verified on the integrated branch and recorded with
its commit, or closed by an explicit owner disposition.

Do not reproduce the old ledger, maintain finding counts, generate lane briefs,
or accumulate review transcripts and test timings. Add newly discovered work
to the checklist only when it is a distinct outcome. Git records the
implementation; the original reports retain their detailed arguments.

Before checking an outcome, confirm its source findings against the current
tree and verify that no unfinished clause was hidden by regrouping. The final
reconciliation reads every original finding once more against the integrated
result.

## 6. Branch and review

All work stays on the long-lived `codex/before-triage` branch until the whole
effort is approved. Keep commits small and coherent, and keep only one code
batch active. Rebase onto current `main` at outcome boundaries and whenever a
Rumors change affects the batch. After a rebase, rerun verification appropriate
to everything that moved; an old green run is not evidence for a changed tree.

Before each commit, present the purpose, actual diff, verification, and any
remaining decision. Stop editing that batch while it is under review so the
working tree remains the exact Zed diff the owner is reading. The owner's
“lgtm” means: commit that reviewed increment, rebase when this is an outcome
boundary, re-evaluate priorities as section 1 requires, and begin the selected
next increment. It does not authorize a merge.

Do not merge the branch into `main` until the owner says the parallel Rumors
work has concluded and explicitly authorizes the merge. Until then, keep this
branch current by rebasing only at outcome boundaries, with the working tree
clean and no increment under review.

The parallel Rumors triage owns Rumors implementation and shared root files.
This branch owns `before` and its supporting crates. Coordinate before editing
the justfile, root manifests and lockfile, CI workflows, root policy files, or
shared tools.

Prepared branches from the aborted triage are never merge inputs. Inspect a
specific diff only when it answers a current question, and rewrite any useful
idea in the smallest form justified by current code.
