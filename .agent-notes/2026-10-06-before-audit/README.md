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
- `lanes/<lane>/round-<n>/`: each auditor round's report and deliverables
  (briefs, observations, coverage record, resumption notes). The coordinator
  copies them here from the auditor's scratch directory when the round ends,
  because the scratch directory lives under `/private/tmp` and does not
  survive a reboot. Builders and demonstrators read their briefs from here.
- [`instruments.md`](instruments.md): the probes, models, and generators on
  the auditors' `explore/` branches, kept for a triage of which to fold into
  the committed suites.
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
`fuelscape/` data, and the `rumors` crate. One exception: the cost text
(`contract` field) in the committed `fuelscape/` data mirrors rustdoc, and an
agent correcting that rustdoc may edit the matching text by hand, without
regenerating the data. 32-bit behavior is fully in scope and
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
The one exception to running everything on the box is branch coverage: no
toolchain on the box ships the profiler runtime that `llvm-cov` needs, so the
adequacy auditor runs coverage on the Mac, niced and capped at eight jobs.

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

**`usize` invariance.** `before` and `suanpan` should behave identically
whatever the width of `usize`. A `usize` anywhere other than indexing into
memory, or counting what memory holds (an array index, a length, a capacity
actually held), is a possible defect: a public parameter, a stored quantity, or
arithmetic whose meaning would change with the target's pointer width. Agents
report such sites as findings when behavior differs by target, and as
simplifications or design proposals otherwise.

**Rulings on questions raised during the audit.**

- `before`'s documented linear bounds must hold unconditionally. The
  composition with `suanpan`'s per-update logarithm (lane L7's F1) is to be
  fixed in `suanpan` without weakening any other guarantee. The suanpan
  auditor gathers the evidence and designs first, rather than `before`
  restating its bounds.
- Shifting a zero accumulator, or adding a zero-valued operand at a shift, is
  total and constant-space whatever the zero's stored form.
- `suanpan::Accumulator::reserve_digits(usize)` becomes
  `reserve_bits(bits: u64)`, a best-effort hint. Its bits-to-digits
  arithmetic saturates rather than overflowing, and a request the allocator
  refuses reserves nothing. It never reserves "as much as it can" short of
  the request. This public API change is approved; it matches
  `suanpan`'s other width parameters and removes the caller's conversion.
- On wasm32, `Rank`'s `Sum` can abort on allocation failure in one summand
  order where the other order and `+` succeed, because `Vec` doubles a
  digit buffer near the 4 GiB limit. This is accepted as an observation:
  the documented `O(n)` space bound holds. It is an input to the `suanpan`
  growth-policy design work, not a defect.
- The fork iterators (`PartyForks`, `ClockForks`) report an exact
  `size_hint` whenever the remaining count fits `usize`, by one rule on every
  target. This supersedes their rustdoc's allowance for wide counts and the
  test pinning it (lane L1's D1).
- `Clock::from_parts` documents that pairing a party with a version older
  than its latest tick reproduces stamps the party already issued.
- After every lane is done and the rest of the review is complete, the
  audit ends with a survey of the auditors' exploratory instruments: generator
  coverage, oracles, and predicates. It ends in polished, sequenced proposal
  branches, one per best enhancement. Nothing from the survey is folded in
  without the owner's review. [`instruments.md`](instruments.md) holds the
  plan.
- Debug-only assertions that scan whole buffers on hot paths are deleted,
  provided a mutation check shows committed tests catch every violation they
  would have caught.

**Worktrees and build reuse.** The box has no compiler cache: kache does not
build on illumos, because its `interprocess` dependency has no illumos peer
credentials. Builds stay warm instead through reused worktrees, under the
coordinator's management, as described below.

**Pinned instruments.** Agents may lower ceilings and floors without asking, as
long as every floor stays strictly positive and so still proves its meter is
counting. Agents never avoid a performance win: they capture it and lock it in
by lowering the ceiling. A worst-case ranking flip caused by a measured win is
re-pinned in the same commit, naming the win. Raising a ceiling, or a ranking
flip caused by a regression, stops work for the owner. No representation on
the wire may change, so any change to a wire or bookmark snapshot or to a
`protocol_overhead` byte count also stops work immediately.

## Worktrees, slots, and disk

This section is the coordinator's operating procedure. It is written so a
coordinator resuming after a context reset can continue without the
conversation.

### Where agents work

- **Auditors** each keep one worktree for the life of their lane:
  `/Users/oxide/src/rumors-audit-l<n>-<name>` on branch `explore/l<n>-<name>`.
- **Every other role** (demonstrator, builder, fixer, reviewer) works in a
  *slot*: one of a fixed pool of reusable worktrees,
  `/Users/oxide/src/rumors-slot-NN`, numbered from 01. The pool started at
  eight, the agent cap of 16 minus the eight auditors, and grows by a slot
  when a finished auditor frees agent capacity and every slot is assigned.
  `git worktree list` shows the current pool.

The remote wrapper syncs each worktree to `~/src/<basename>` on the box, and
every remote command unsets `CARGO_TARGET_DIR`, so each worktree builds in
`~/src/<basename>/target/`. The wrapper's rsync excludes `/target/`, so that
directory survives every sync. Reusing a worktree at the same path therefore
reuses a warm build.

### Slot state

`git worktree list` is the record of slot state; nothing else tracks it.

- A slot on a named branch is *assigned* to that branch.
- A detached slot is *free*.

### Assigning and releasing a slot

1. **Assign.** Pick a free slot and confirm `git -C <slot> status --short` is
   empty. Then switch it to the branch: `git -C <slot> switch -c <branch>
   <base>` for a new branch, or `git -C <slot> switch <branch>` for an
   existing one.
2. **Keep one slot per branch.** A branch's demonstrator, fixer, reviewer,
   and any second review round all run in the same slot, one after another,
   so each finds the build warm for nearly the tree it needs. A reviewer
   works on the checked-out branch and commits nothing.
3. **Release.** Release the slot when its branch converges or is paused.
   Confirm `git status --short` is empty, then run `git -C <slot> switch
   --detach main`. The branch itself remains.

Reuse is safe because cargo decides what to rebuild from mtimes. Switching
branches gives every file that differs a fresh mtime, and rsync carries that
mtime to the box, so cargo rebuilds exactly what changed. Files absent from
the new branch disappear from the box through rsync's `--delete`.
Third-party dependencies stay fresh because the slot's path never changes.

Each slot's first build is cold. At creation, every slot was warmed at
`58285ca5` with `cargo nextest run --workspace --all-features --locked
--no-run`, one slot at a time under `nice -n 19`.

### Disk duties

The coordinator owns disk usage on both machines.

**On every status pass, and before every dispatch,** check the box:

```
ssh ox-east-1-agent 'df -h /home; du -sh ~/src/rumors-audit-*/target ~/src/rumors-slot-*/target; swap -sh'
```

At launch, an auditor's `target/` held about 1 to 5 GB, and `/home` had 1.23 TB
free. The box's `/tmp` is swap-backed, so a large scratch copy there consumes
memory, not disk.

| Condition | Action |
|---|---|
| A slot's `target/` exceeds 30 GB | On release, delete that `target/`; the next use is cold. |
| An auditor's `target/` exceeds 60 GB | Ask the auditor, by message, to run `cargo clean` at its next pause. |
| `/home` has less than 300 GB free | Stop dispatching. Clean free slots' `target/` directories first, then ask the user. |
| Swap has less than 200 GB available | Find the process holding it (likely cargo-mutants scratch copies in `/tmp`) and ask its agent to stop. |

**On the Mac**, nothing builds, since every agent executes on the box. The
worktrees hold only source, so local disk needs no routine check. Agents'
`cargo fmt` runs touch no build directory.

### Agent compaction

Subagents compact automatically when their context fills, and each keeps a
resumption record at `<scratch>/<agent-name>/NOTES.md` (see `common.md`). On
each status pass, count `compact_boundary` entries in every running agent's
transcript, using `grep -c` and never reading the transcript whole. After an
agent compacts, check that its next actions follow from its `NOTES.md` rather
than restarting or repeating work.

### Retirement

When the audit ends, retire each slot and auditor worktree:

1. Confirm `git status --short` is empty.
2. Run `git worktree remove`, never with `--force`.
3. Delete `~/src/<basename>` on the box.

Keep every `explore/` branch until the owner has triaged its instruments in
[`instruments.md`](instruments.md).
