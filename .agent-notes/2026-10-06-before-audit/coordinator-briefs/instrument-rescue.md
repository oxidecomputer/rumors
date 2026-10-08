<!-- CAVEAT LECTOR: a cataloguing brief written by the coordinator (Claude Opus 5.5) for the owner's instrument rescue (question 99). -->

# Brief: catalogue every instrument one lane built

## The owner's intent

"Rescue the *highest value* instruments which *add additional coverage*
that were already constructed as a part of the work done in this audit. It
does not need to be the case that they catch defects which can be
reasonably constructed or which existed in reality; I want instruments that
expand coverage to defend against future issues, provided we already
constructed them." And: "Every lane could have built useful instruments!
Treat them all with thoroughness."

So: catalogue **every** instrument your lane built, each with a **full**
entry. Exclude nothing; do not filter by whether it caught a defect. Do not
propose building anything new. An integrator ranks across lanes afterwards;
you describe and assess your lane thoroughly.

## What counts

Anything built to test, measure, generate, model, or observe: models and
oracles, generators and strategies, properties and predicates, exhaustive
enumerations, differential drivers, probes, cost families and meters,
target-dependence checks, diagnostics and census tooling, mutant schemas and
calibration harnesses, fixtures and witnesses. Read the whole of your source,
not only the inventory: diff the explore branch against its base
(`git -C /Users/oxide/src/rumors log --oneline <base>..<branch>` and
`git diff <base>...<branch> --stat`), and read every file it adds.

An instrument already committed on `main` or carried by a ready branch
(`QUESTIONS.md`, "Branches ready for your review") gets one line naming
where it lives; everything else gets a full entry.

## The entry template (use it for every instrument)

```
### <name>

- **What it is.** Kind; location (`branch:path` at commit); size.
- **What it reaches or checks.** Input regimes, with numbers (depth, size,
  mix, rates) measured against the committed baseline in
  `instrument-rescue/00-baseline.md` once it exists; the oracle and whether
  it is independent of production code; predicates no committed test
  states.
- **Coverage beyond the committed suite.** Concretely what a committed
  instrument cannot reach or state that this one does. "None" is a valid
  answer; say so plainly.
- **Evidence.** Mutants killed, defects caught, regimes reached, with
  sources. Say plainly when it caught nothing.
- **Fold-in cost.** Work to port it into the shared instruments
  (`testing::generators`, `testing::laws`, `testing::oracles`,
  `testing::diff_ops`, `testing::exhaustive`, the board, the fuzz-fit
  harness) rather than as a parallel harness; runtime against nextest's
  per-test limit (measured or "unmeasured"); new dependencies; the project's
  test-documentation standard.
- **Overlaps.** With other instruments in this lane, in other lanes you know
  of, and with committed ones; whether it would replace a committed
  instrument or stand beside it as a second, independent check.
- **Dependencies.** On unlanded branches in `QUESTIONS.md` notice 96.
- **Value, in one sentence.** Why the owner might want it, or why not.
```

Every claim is marked *verified* (you checked it), *reported* (a record
says so), or *inferred*. Follow Part IV of `/Users/oxide/.claude/CLAUDE.md`:
plain sentences, no coined terms, numbers with sources. The owner reads this
without the chat.

## Output and ground rules

- Write `/Users/oxide/src/rumors/.agent-notes/2026-10-06-before-audit/instrument-rescue/NN-<lane>.md`
  (your file name is in your prompt): a short lane summary, then one entry
  per instrument, ordered by your judgment of value. Do not commit.
- Read explore branches with `git show`, never by checking them out in the
  main worktree.
- Box runs only for a census or runtime measurement the records lack: at
  most three, via `~/.claude/skills/building-on-illumos/scripts/on-illumos.sh`
  on a scratch worktree of your own, `/Users/oxide/src/rumors-rescue-<lane>`
  (create with `git -C /Users/oxide/src/rumors worktree add --detach <path>
  <branch>`, remove with plain `git worktree remove` at the end). Outputs
  only under that worktree.
- Never use EnterWorktree; never modify an explore branch; no commits; delete
  nothing outside your scratch worktree and `<scratchpad>/rescue-<lane>/`.
- Session scratchpad: `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/`.
  The earlier ranker may have left handoff notes for your lane at
  `<scratchpad>/ranker/handoff-<lane>.md`; use them as leads, verified.
- Report: the number of instruments catalogued, the top three by your
  judgment in one line each, and anything you could not assess.
