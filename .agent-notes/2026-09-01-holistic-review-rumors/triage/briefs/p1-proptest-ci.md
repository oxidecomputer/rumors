<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from ruling T148 in ../rulings.md; not authored, audited, or endorsed by Finch. Read with the ground rules in ../../README.md. -->

# P1 lane: fast proptest counts in the gate, many in CI under release

## Goal

Every committed proptest case count in `rumors` is a gate-time compromise:
small enough that `just gate` stays affordable, far smaller than the
coverage the property deserves. The invariant this lane lands: the gate's
counts stay what they are, and coverage comes from CI, which runs the same
suites under the release profile with a case count many times larger. One
number reaches every suite through a helper the explicit configs route
through, because proptest reads `PROPTEST_CASES` only into
`ProptestConfig::default()`, and a `cases` literal or `with_cases(n)`
(which spreads the default under the literal) silently ignores it. A
committed check keeps every config routed.

## Ground rules

- **Base.** Your worktree's HEAD must equal `<base sha>` before you start
  (the coordinator fills it: `main` after the P1 and P2 lanes that touch
  test files have merged). Run `git -C <worktree> rev-parse HEAD`. If HEAD
  is an ancestor of `<base sha>`, fast-forward; if it has diverged, stop
  and report. Never call EnterWorktree; operate on the worktree through
  `git -C <path>` and absolute paths, one shell invocation at a time.
- **The ruling is the specification.** The member entry below quotes T148
  verbatim; the ruling is the authority where this brief and it disagree.
  Line anchors are at `2643c18e`; re-derive every site from the grep in
  the Resolution, never from the line numbers.
- **Goal beside mechanism.** Where the ruling's mechanism and the goal
  above come apart, the goal wins, and the discrepancy is reported.
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin; any change to a public
  signature or public rustdoc contract the ruling does not name; anything
  that contradicts a ruling in `triage/rulings.md`; any deviation from the
  stated resolution; anything this brief marks as a stop.
- **Negative controls.** Every landed instrument carries a committed
  `--self-test` case a known-bad artifact fails; reversible-mutation
  runs are recorded verbatim in the commit message.
- **Where you build.** The illumos box (`ox-east-1`, per the
  `building-on-illumos` skill) is where this lane builds, tests, and
  gates; the Mac runs no gate. The gate of record is
  `on-illumos.sh <worktree> 'just gate'`, one run per commit series,
  redirected to a log under `<scratchpad>/p1-proptest-ci/` and polled with
  short foreground checks (the foreground cap is ten minutes). `fuzz` is
  expected red there (libFuzzer has no illumos port; `FuzzerPlatform.h`
  refuses the target) and counts as clean when it is the only failure:
  quote that line, and run no fuzz build elsewhere. Never run `just all`
  or `just ci`; `ci` is GitHub's, and the release recipe this lane adds
  is run alone on the box. Clock guard before every box run: rsync keeps
  mtimes and cargo's rebuild detection is mtime-based, so a box clock more
  than a couple of seconds ahead of the Mac means a green build of stale
  code; on skew, use a fresh target directory on the box or wait, and say
  which. Hold a launch while the box's one-minute load average is above
  about 150; a wall-time measurement runs under `pset-run`, one at a
  time, announced in `.agent-notes/merge-queue.md` first.
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries you are routing; never
  iterate on a timing measurement (the procedure below runs once per
  candidate). Keep every working file under the scratchpad directory named
  above. Never delete anything outside your worktree; on a full disk, stop
  and report.
- **Commits.** One commit per logical unit (the helper and the sweep; the
  check; the recipe and the workflow job), its message naming the ruling.
  Commit every proptest seed file that appears. Prose speaks in the present
  tense: no reference to code that no longer exists, no dated rationale at
  a declaration site. Comments use spaced double-hyphens, never
  em-dashes; every test has a doc comment stating its invariant. Apply
  `PROSE.md`'s three tests to every paragraph you touch.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For the entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim);
  the site table (file, count before, call after); the timing table; the
  stop block. Your report is data: the coordinator verifies the Acceptance
  against the tree at the reported sha before the ledger records it.
  Report what you could not do rather than working around it.

## Members

### proptest-ci: ruling T148

> ## T148 (2026-09-02): The gate keeps fast proptest counts; CI runs the suites in release with many
> Disposes: the causality case-count question (tests-bookmark-9, T8) and, crate-wide, every proptest whose committed count is a gate-time compromise
> Decision: Committed case counts stay what the gate can afford; coverage comes from CI, which runs the test suites under the release profile with a case count many times larger. Mechanism: one helper in the crate's test support (`rumors::testing` or the tests' common module, whichever both unit and integration suites can reach) returns the case count for a suite, honoring `PROPTEST_CASES` from the environment when set and the suite's committed default otherwise; every explicit `ProptestConfig` in the workspace takes its `cases` from that helper, since an explicit literal ignores the environment, and a committed check (a grep leg in the gate's lint tier) fails on any `cases:` literal that bypasses it. CI's test job runs `cargo nextest run --cargo-profile release` with `PROPTEST_CASES` set; the number is measured once on the box per suite in release before it is pinned in the workflow, chosen as the largest that keeps the job inside a stated wall-time budget, and Finch rules the number from the measurement. The gate's own test legs are unchanged.
> Home: lane `p1-proptest-ci` (a sweep over every `ProptestConfig` site, the helper, the check, the `ci.yml` job, and the justfile's `ci` composition), launched after the P1 and P2 lanes that touch test files have merged; brief `briefs/p1-proptest-ci.md`.

Resolution:

1. **The helper.** `pub fn cases(default: u32) -> u32` in `src/testing.rs`
   (`rumors::testing`, built under `cfg(any(test, feature =
   "test-internals"))`; the integration suites already reach it through
   the self dev-dependency, so one function serves both). Doc: returns
   `PROPTEST_CASES` when the environment sets it, else `default`, the
   suite's committed count; states why it exists (proptest reads the
   variable only into `ProptestConfig::default()`; an explicit `cases`
   ignores it); the gate never sets the variable, and the CI release run
   sets it to the justfile's pin. `proptest` is a dev-dependency only, so
   the helper parses the variable itself; a set but unparsable value
   panics naming the variable and the value, because a silent fallback
   would let a mistyped pin run the gate's counts under CI's job and read
   green. Unit tests: unset gives the default; set gives the value.
2. **The sweep.** Every explicit `ProptestConfig` under `src`, `tests`,
   `benches`, `examples` takes its count from the helper, committed
   defaults unchanged: `ProptestConfig::with_cases(cases(64))` and
   `ProptestConfig { cases: cases(32), ..ProptestConfig::default() }`. The
   survey at `2643c18e` (`git grep -n -E 'ProptestConfig \{|ProptestConfig::with_cases|cases: [0-9]' -- src tests benches crates`):
   `remote/proxy/tests.rs:532` (`cases: CASES`, a `const` of 48: the
   const goes, the call takes its place; a local `cases` binding sits in
   that test, so call the helper qualified), `remote/proxy/tests/failures.rs:152`
   (32), `remote/proxy/tests/transport.rs:52` (32), `streaming/tests/capacity.rs:357`
   (64), `streaming/tests/stats.rs:221` (64), `tests/bookmark_when.rs:683`
   (128), `tests/disruption.rs:569` (8; deleted on `triage/p1-harness-tests`),
   `tests/observe.rs:305` (24), `tests/party_conservation.rs:392` (32),
   `tests/session_overlap.rs:137` (48), `tests/session_stats.rs:295` (24),
   `tests/wire_legibility.rs:119` (24). Re-derive at your base: merged
   lanes move and add sites (`streaming/tests.rs` gains a config with no
   `cases` field, which is already routed). The three `crates/before` sites
   (`src/testing/semantic_oracle/tests.rs:559`, 400; `fuzzfit/harness/tests/enforce.rs:439`,
   48; `fuzzfit/harness/tests/sanity.rs:14`, 64) belong to the `before`
   triage, which routes them through a twin of the helper in `before`'s
   own test support; do not touch them.
3. **The check.** `tools/caselint`, a python linter in the gate's lint
   tier (`gate-lints`, beside `manifestlint`; `--self-test` first, then
   the run over `src tests benches examples`), failing on any `cases:`
   field or `with_cases(` argument whose value is not a call to the
   helper: an integer literal, a `const`, any other expression. Its
   header carries the argument above. Self-test cases: `cases: 32,` red;
   `with_cases(64)` red; `cases: CASES` red; `cases: cases(32)` and
   `with_cases(testing::cases(64))` green; a config with no `cases` field
   green. Extending the roots to `crates/` is the `before` triage's, once
   its twin lands; say so in the tool's usage text.
4. **The recipe and the pin.** A justfile variable `proptest_ci_cases`
   beside `nightly_toolchain`, the one site of the number, its comment
   carrying the budget and the measurement; a recipe `test-release`:
   `PROPTEST_CASES={{ proptest_ci_cases }} cargo nextest run --workspace
   --all-features --cargo-profile release {{ args }}`, its comment saying
   why it exists and why it is not a gate leg. Until Finch rules the
   number, the variable holds the measured recommendation and the recipe
   lands; the pin is a stop (below), not a blocker on the rest.
5. **The workflow.** `.github/workflows/ci.yml` gains a job `proptest`
   (checkout; the tools step; read and install the pinned stable, in the
   order the `ci` job establishes: tools, stable, nightly, with nightly
   omitted since nothing here needs it; the cache step; `just
   test-release`), its comment saying what it runs and that the count
   lives in the justfile. The `ci` recipe line stays as it is: the job is
   parallel to `ci`, so its budget is its own, and the justfile header
   and the workflow header name it as they name `instruments` and
   `coverage`; `all` composes `test-release` as it composes the coverage
   legs (T15). `tools/workflowlint` must pass.
6. **The measurement.** On the box, release profile, `--no-run` first and
   excluded from timing, then once per candidate under `pset-run -n 4`
   (four cores approximate the GitHub runner's parallelism; report the
   one-minute load average before and after): `PROPTEST_CASES` unset (the
   committed counts, the baseline), then 1024, 4096, and 16384. Record
   whole-run wall time and nextest's per-test times per binary. The table
   and a recommended number go to Finch against a budget of thirty minutes
   of four-core wall time for the run, a placeholder he ratifies or
   replaces with the number.

Acceptance, all mechanical:

- `./tools/caselint --self-test` passes; `just gate-lints` runs it.
- `git grep -n -E 'cases: [0-9]|with_cases\([0-9]|cases: [A-Z_]+' -- src tests benches examples` is empty.
- Negative control: restore one literal (`tests/observe.rs`, say); `just
  caselint` fails naming that file and line; revert; recorded verbatim in
  the commit message.
- `PROPTEST_VERBOSE=2 PROPTEST_CASES=1 cargo nextest run --no-capture -p
  rumors --test <suite>` on each routed integration suite prints one
  `Test case passed` per property (proptest's trace level); the same for
  one routed unit binary under `--lib`. Per-test times against the
  baseline corroborate.
- The box gate is clean with `fuzz` the only red; `just test-release`
  alone on the box completes at the recommended number.

## Hazards and stops

- **The number is a stop.** The lane reports the timing table and its
  recommendation as a numbered block; nothing is pinned as ruled until
  Finch rules. Item 2 of the same block: the job shape (a parallel
  `proptest` job with `ci`'s recipe line unchanged, versus `test-release`
  on the `ci` line inside the `ci` job); recommend the parallel job.
- A suite whose runtime does not scale with the count (per-run setup
  dominates, or a config the check cannot see) is reported by name, with
  the two times; it is either a note in the table or a hole in the check.
- The GitHub runner has four cores and sixteen gigabytes; the workflow's
  `instruments` comment says the bench-tier compile exceeds that. A
  release test-tier build is a different artifact with unmeasured peak
  memory: report a figure if the box can give one, else say so.
- `crates/before`'s three sites are the other triage's; hand it, through
  the merge queue, the finding that `enforce.rs:36`'s claim ("override
  with `PROPTEST_CASES`") is false under `with_cases(48)`.
- `ci.yml` and the justfile are shared root files: `triage/p1-gate` owns
  them until it merges (its pin order and `just --evaluate
  nightly_toolchain` are what the job above copies; `before`'s ruling 25
  wants the nightly derived the same way). Launch after gate merges, or
  stack on its tip; announce in `.agent-notes/merge-queue.md` either way.
- T116's schedule-executor configs route through the helper whenever
  they land; landed later, the check fails their literal for that lane.
