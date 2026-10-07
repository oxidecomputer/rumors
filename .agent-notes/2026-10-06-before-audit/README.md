<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) for Finch while planning an audit of `before` and `suanpan`; the rulings below transcribe Finch's answers in the planning conversation of 2026-10-06. -->

# Correctness audit of `before` and `suanpan`

This directory holds the plan, briefs, and results of an adversarial audit of
the `before` and `suanpan` crates at `main` = `455e97de`. The goal is to find
any input to any operation that violates the crates' semantics, as documented
or as coherently extrapolated, and to demonstrate each violation with a
legible, standalone test. Asymptotic cost is a secondary target.

The 2026-09-01 holistic review and the 2026-09-16 triage are closed. This audit
treats them as a map of examined ground and pushes into what they did not
examine.

## Files

- [`briefs/common.md`](briefs/common.md): ground rules every agent follows.
- [`briefs/auditor.md`](briefs/auditor.md): the auditor role.
- `briefs/lane-*.md`: the eight lane sections, one per auditor:
  [L1 identity](briefs/lane-l1-identity.md),
  [L2 algebra](briefs/lane-l2-algebra.md),
  [L3 events](briefs/lane-l3-events.md),
  [L4 measures](briefs/lane-l4-measures.md),
  [L5 spans](briefs/lane-l5-spans.md),
  [L6 codecs](briefs/lane-l6-codecs.md),
  [L7 suanpan](briefs/lane-l7-suanpan.md),
  [L8 adequacy](briefs/lane-l8-adequacy.md).
- [`briefs/demonstrator.md`](briefs/demonstrator.md),
  [`briefs/builder.md`](briefs/builder.md),
  [`briefs/fixer.md`](briefs/fixer.md),
  [`briefs/reviewer.md`](briefs/reviewer.md): the downstream roles.
- [`baseline.md`](baseline.md): what passes and fails on ox-east-1 at the
  audit's base commit, against which every later failure is judged.
- `QUESTIONS.md`: the questions currently awaiting the owner's ruling, each
  written to be answerable without the conversation. The coordinator keeps it
  current, deleting each entry once it is answered, and the file's header sets
  how entries are written. It is excluded from git (in `.git/info/exclude`)
  because it holds only live state; rulings worth keeping move into the
  rulings of record below.

## Rulings of record

The owner ruled on each of these in the planning conversation.

**Contract model.** Every operation must be total, panic-free, within its
documented resource bounds, and algebraically lawful over *any* canonical
values, whatever their provenance. Mixing universes or handling identity
non-linearly forfeits only causal meaning. A law that fails only on valid values
which no rule-respecting history reaches is still a violation of a law, and is a
defect in need of fixing.

**Scope.** In: `before`, `suanpan`, their tests and test-support modules, the
`wasm32-pins/` and `surfacecheck/` workspaces, and the amplification board and
meters. Out: `before-viz`, `before-fuelscape`, the `fuzz/` and `fuzzfit/`
workspaces, `build.rs`'s fuelscape rendering, the `docs/` widget, the committed
`fuelscape/` data, and the `rumors` crate. 32-bit behavior is fully in scope and
is tested currently through the `wasm32-pins` executor.

**Performance.** Findings are asymptotic primarily: a deterministic counter
(scan, touch, heap, or wasm fuel) growing faster than the documented bound
across at least four sizes. Note potential improvements in constant factors as
clearly documented observed findings, but do not spend time constructing these
improvements unless they are small, contained, and simple. No wall-clock
measurement or benchmarking.

**Execution.** Every build, test, and run executes on ox-east-1, including
inputs of several gigabytes. Allowed tools: cargo-mutants, llvm-cov branch
coverage, and long property-test runs. libFuzzer is not used: it does not
build on illumos, and the fuzz workspace that depends on it is out of scope.

**Organization.** Eight lanes launch together. The coordinator dispatches every
agent centrally (lanes never spawn their own agents), with at most 16 agents
running at once, enforced by coordinator pacing. Agents run on Opus 5.5. Five
roles, each a project agent definition with its own effort level:

| Role | Effort | Job |
|---|---|---|
| auditor | xhigh | investigates one lane; writes briefs for the other roles |
| demonstrator | medium | implements one defect's standalone failing test from a brief |
| builder | high | implements gap-closing test machinery or a simplification from a brief |
| fixer | high | implements the smallest complete fix on a demonstrated branch |
| reviewer | high | reviews any branch skeptically before it reaches the owner |

Auditors, and the coordinator as a whole, stop at diminishing returns: when
findings shift from "this is wrong" to "you might want to consider".

**Priorities.** Broad attention everywhere. Agents also report structural
simplifications that would make code more obviously correct even where it is
correct today, and implement proposed simplifications if they are cleanly
self-contained (i.e. unlikely to cause large merge conflicts). More substantive
structural simplification refactors are noted as observations to triage after
the fact.

**Branches.** Nothing lands on `main` autonomously, except notes in this
directory. Every branch presented for review is production quality and
mergeable; no rough branches. Each confirmed defect gets a branch from `main`
whose first commit is a standalone failing test and whose second is the smallest
complete fix, when that fix preserves the public API and the wire and storage
formats. Otherwise the branch stops at the failing test. Every self-contained
behavior-preserving simplification is built as its own clean branch. When a fix
awaits review, the lane merges it and keeps searching past it.

**Review.** Every branch gets a skeptical review by a separate agent before it
reaches the owner, aiming for one round and allowing two. A branch that has not
converged after two rounds is paused and reported durably with its outstanding
issues clearly noted.

**Verification and commits.** Agents commit freely on their branches.
Intermediate commits run focused checks; each branch's final state runs the
full `just gate` on ox-east-1 before handoff, and its result matches
[`baseline.md`](baseline.md). Commits are signed; when 1Password
cannot sign, agents commit with `--no-gpg-sign` and the coordinator re-signs
exactly those commits before handoff, preserving trees, messages, and dates.

**Pinned instruments.** Agents may lower ceilings and floors without asking, as
long as every floor stays strictly positive and so still proves its meter is
counting. Agents never avoid a performance win: they capture it and lock it in
by lowering the ceiling. A worst-case ranking flip caused by a measured win is
re-pinned in the same commit, naming the win. Raising a ceiling, or a ranking
flip caused by a regression, stops work for the owner. No representation on
the wire may change, so any change to a wire or bookmark snapshot or to a
`protocol_overhead` byte count also stops work immediately.
