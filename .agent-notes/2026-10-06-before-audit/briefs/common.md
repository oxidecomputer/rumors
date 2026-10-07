<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the shared section of every agent brief in this audit; under review by Finch before launch. -->

# Ground rules for every agent in this audit

You are one agent in a coordinated audit of the `before` and `suanpan` crates in
the `rumors` workspace. A coordinator (the session that launched you) assigns
your role, verifies what you report, and dispatches all other agents. Your role
brief and your task prompt follow this section. Where they conflict with it,
the more specific text wins, and you say so in your report.

## Read first, in full

Your agent definition preloads two skills in full: `proptest-praxis` (the
investigation framework) and `building-on-illumos` (how to execute on the
remote box). They are required reading, not background. The owner has adapted
`proptest-praxis` in two ways:

- Nothing rough is ever committed to a branch someone will review.
- "PR" means a local branch. **Nothing is ever pushed.**

Then read each of these completely before doing anything else:

1. The project guideposts: `/Users/oxide/src/rumors/AGENTS.md` and
   `/Users/oxide/src/rumors/crates/before/AGENTS.md`.
2. The audit's rulings of record:
   `/Users/oxide/src/rumors/.agent-notes/2026-10-06-before-audit/README.md`.
3. The baseline your verification is judged against:
   `/Users/oxide/src/rumors/.agent-notes/2026-10-06-before-audit/baseline.md`.
4. Part IV of the owner's doctrine, "Writing Style", in `~/.claude/CLAUDE.md`.
   The whole file is already loaded into your context; study Part IV before
   writing any prose, because it governs every line you write: rustdoc,
   comments, assertion messages, commit messages, and reports. Rust
   documentation matters most here. Read these sections closely:
   - "Shaping a document"
   - "Sentences and voice"
   - "Vocabulary and naming"
   - "Contracts, claims, and invariants"
   - "Examples"
   - "Comments, commits, and errors"
   - "The default dialect to diverge from", with its lexicon

## Why this work exists

The owner maintains these crates for years, and `rumors` builds on them. A
defect found now is one nobody has to diagnose in production. The bar for
finished code is that it is *obviously* correct, reviewably and not merely
tested. Judge every finding by the contract clause it breaches, never by how
likely the triggering input is: "unreachable in practice" carries no weight.

## The contract you audit against

- **Totality and lawfulness.** Every operation must be total, panic-free,
  within its documented resource bounds, and algebraically lawful over *any*
  canonical values, whatever their provenance. Mixing universes or handling
  identity non-linearly forfeits only causal meaning. A law that fails only on
  valid values no rule-respecting history reaches is still a defect to fix.
- **Sources of the contract.** In order of authority:
  - the public rustdoc
  - the ITC paper (`crates/before/reference/itc2008.md`), as realized by the
    recursive tree oracle (`testing::oracles::tree`)
  - the function-space oracle (`testing::oracles::function`)
  - your own coherent extrapolation of what a type or operation means

  Where these disagree, or a documented contract is itself incoherent, report
  that as a finding. Never pick a winner silently.
- **32-bit targets.** These are fully in scope. The `wasm32-pins` executor is
  the current instrument; another means needs a stated reason in your report.
- **`usize` invariance.** Both crates should behave identically whatever
  the width of `usize`. Treat a `usize` anywhere other than indexing into
  memory or counting what memory holds (an index, a length, a capacity
  actually held) as a possible defect: a public parameter, a stored
  quantity, or arithmetic whose meaning would change with pointer width.
  Report it as a defect where behavior differs by target, and as a
  simplification or design proposal otherwise.
- **Performance.** Asymptotic findings come first. A finding is a
  deterministic counter (scan, touch, heap, or wasm fuel) growing faster than
  the documented bound across at least four sizes.
  - Report constant-factor improvements as clearly documented observations.
    Build one only when it is small, contained, and simple.
  - Never measure wall-clock time or benchmark.

## Where you work

- Use absolute paths, and `git -C <worktree>` for git. Never use the
  EnterWorktree tool.
- Your task prompt names your worktree, branch, and base SHA. Before anything
  else, check that the worktree's `HEAD` is that SHA. If it is an ancestor of
  that SHA, fast-forward; if it has diverged, stop and report.
- Keep working files under
  `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/<your-agent-name>/`.
  Agents share the scratchpad, so never write outside your own directory.
- Never delete anything outside your own worktree and scratch directory. On
  ENOSPC, stop and report.
- Never modify the main checkout at `/Users/oxide/src/rumors`, other agents'
  worktrees, or any directory on ox-east-1 other than your own worktree's. You
  may read the main checkout (these briefs live there), and you may read other
  worktrees when your task prompt points you at one.

## How you execute

- Every build, test, and run executes on ox-east-1 through
  `/Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh <worktree> 'unset CARGO_TARGET_DIR; <command>'`.
  Never run `cargo build`, `check`, `test`, `clippy`, `nextest`, or `run`
  locally. The one local exception is `cargo fmt`, because it rewrites source
  files and edits on the box never sync back.
- Begin every remote command with
  `unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24;`.
  - The box's cargo config caps build jobs at 24, but nextest runs one test
    process per hardware thread (192) unless told otherwise, and that
    includes every nextest run inside `just gate`. Several such runs at
    once push the load past 500, which slows every agent and risks
    spurious timeouts against nextest's 180-second limit.
  - The wrapper exports a build directory outside the tree, but the
    justfile's `docs` recipe reads `target/doc`. Under the wrapper's default,
    `just gate` therefore fails for a reason unrelated to your change.
  - Building in the synced tree's own `target/` is safe, because the wrapper's
    rsync excludes `/target/` and so never deletes it.
  - It also gives your worktree one build directory instead of two.
  - illumos's `env` has no `-u` flag; use `unset`.
- Pass `--locked` to cargo on the box, so `Cargo.lock` changes only on the Mac.
- Never edit files in your worktree while a build or test of it is running on
  the box. A sync during a build can leave cargo with a fingerprint newer
  than the source it compiled, so the next run reuses a stale binary and
  reports a stale result. If a result looks impossible, touch the edited
  file and rerun.
- Capture whole output to a file in your scratch directory, then filter the
  file. Never pipe a command into `tail`, `grep`, or `head` inside the remote
  command: the pipeline's exit status becomes the filter's, and a failing run
  reads as passing.
- Commands longer than a few minutes run in the background with output
  redirected to a file, which you poll; a foreground call dies at ten minutes.
- Up to 16 agents share the box.
  - Run long investigative jobs under `nice -n 10`.
  - Separate `--no-run` builds from test runs.
  - Never repeat a run hoping for a different result.
  - Committed tests must finish inside nextest's 180-second limit at the
    default case count. Raise `PROPTEST_CASES` only in investigative runs.
- Never launch services or daemons, never spawn agents, and never message
  other agents. Everything routes through the coordinator: requests for more
  investigation go in your report.
- The fuzz workspace (`crates/before/fuzz/`) is out of scope: do not audit,
  modify, or run it. libFuzzer does not build on illumos in any case.

## Standards for anything you commit

- Follow the repo's test conventions (`AGENTS.md`, "Writing tests"):
  - Tests live in a sibling `tests.rs`.
  - Every test has a doc comment stating the invariant it protects, and that
    comment must be accurate.
  - Families are tested as properties or by exhausting a finite domain.
  - Generators build constrained values directly; they never reject most of
    their output.
  - Every proptest regression file that appears gets committed.
- Extend the existing instruments before adding parallel ones: the
  differential table (`testing::diff_ops`), the operation traces
  (`testing::optrace`), the law registry (`testing::laws`), exhaustive
  enumeration (`testing::exhaustive`), and the amplification board. A new
  instrument must name a constructible failure that no existing instrument
  catches, and must get an entry in the validation index
  (`src/testing/validation_index.rs`).
- Leave every test, helper, or generator you touch more legible, without
  weakening its coverage. If you find one that is wrong, vacuous, or weaker
  than its claim, fix it and call out the behavioral correction.
- Prose is in the present tense: no history, no ghost references to deleted
  code, no private roster IDs, no hand-maintained counts.
- Commit messages are imperative, capitalized, and name the component, with
  one purpose per commit.
- Before every commit, read every changed line of prose against Part IV. In
  particular:
  - Does each doc comment open with one complete, verb-first,
    present-tense sentence that survives extraction into a one-line listing?
  - Is every claim true of the code as it now stands, stated at the altitude
    of the reader who meets it, with its argument beside it?
  - Are preconditions and hazards stated where the reader will look, under
    `# Panics`, `# Errors`, or the crate's other named sections?
  - Is every term either plain or defined at first use, with no
    default-dialect vocabulary (check the lexicon)?
  - Can any sentence be deleted without losing meaning?
- Sign commits normally. If signing fails (1Password locked, or a broken
  pipe), commit with `--no-gpg-sign` and list those SHAs in your report.

### Pinned instruments

- You may lower ceilings and floors without asking, as long as every floor
  stays strictly positive.
- Never avoid a performance improvement. Capture it and lock it in by lowering
  the committed ceiling, if one exists.
- A worst-case ranking flip caused by a measured improvement is re-pinned in
  the same commit, naming the improvement.
- No representation on the wire may change.
- Stop and report immediately, instead of committing, in each of these cases:
  - a ceiling would rise
  - a ranking flips because of a regression
  - a wire or storage representation would change: an `insta` snapshot moves,
    a `protocol_overhead` byte count moves, or `BOOKMARK_FORMAT_VERSION` would
    need to change
  - the `rumors` crate would need to change

### Verification before handoff

- Intermediate commits need only focused checks: `cargo fmt`, clippy, and the
  affected suites.
- A branch is ready for handoff only when its final state's `just gate` on
  ox-east-1 matches `baseline.md`:
  - every leg the baseline records as passing passes
  - every recorded failure reproduces exactly, with nothing new beside it
- The one exception is a fix branch's failing-test commit, which fails by
  design.
- The coordinator updates `baseline.md` whenever the baseline changes; read
  the current file, not a remembered version.

## Surviving context compaction

A long task may outgrow your context window. When it does, your context is
automatically compacted: earlier turns are replaced by a summary, and you
continue from it. Your agent definition and the owner's `CLAUDE.md` survive
compaction. These briefs, files you read, and the details of your own
reasoning survive only as far as the summary preserves them.

- Keep `<scratch>/NOTES.md` as your resumption record, current at every
  milestone:
  - what you have established, and how (commands, revision, results)
  - your open hypotheses and the evidence for each
  - the background jobs you started, with their log paths and remote process
    IDs
  - what you will do next

  Write it so a fresh agent holding only the briefs could continue your work.
- After a compaction, re-read `common.md`, your role brief, your task prompt's
  lane section or brief, and `<scratch>/NOTES.md` before acting. Verify
  anything the summary asserts that you are about to rely on, against the
  artifact it names.

## Honesty and reporting

- Distinguish what you verified from what you inferred, in every claim. Quote
  evidence (command, exit status, the failing assertion) verbatim rather than
  paraphrasing it.
- A blocked task is a valid outcome. If the task as stated is impossible or
  contradictory, the report of that is the deliverable.
- If you notice yourself building a workaround whose main effect is to avoid
  reporting something, stop: that noticing is the most important report.
- Ambiguity about the intended semantics goes to the coordinator as a
  question. Never invent a specification to make a test pass.
- Your final message is your report. Open with the verdict, then follow your
  role brief's report format.
- Before returning, commit your work and finish your final verification. The
  coordinator retires worktrees and remote build directories; do not delete
  them yourself.
