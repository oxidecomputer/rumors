<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from ruling T136 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P1 lane: memwatch retirement

## Goal

`tools/memwatch` wraps every codegen-running justfile recipe in a memory
watchdog with two checks. The swap check reads a figure the kernel drains
lazily, so it stays high after live pressure has passed and kills healthy
gate legs on a machine with tens of gigabytes free. The per-process
ceiling has no committed demonstration of a runaway it caught. Retiring an
instrument states what replaces each check: nothing, by owner decision;
the resource control of record is the cap of four concurrent builders per
machine, and a runaway ever observed is the demonstration a replacement
lands with. This lane deletes the script and every trace of it and changes
nothing else: each recipe runs the same command with the same arguments,
environment, working directory, and exit status, minus the wrapper.

## Ruling, quoted verbatim from `../rulings.md`

> ## T136 (2026-09-02): memwatch is retired
> Disposes: a finding from the first lane wave, not a roster entry
> Decision: `tools/memwatch` is deleted, with every justfile recipe that wraps a command in it unwrapped, and every mention of it (CI workflow comments, the `Cargo.toml` comment on the fuzzfit harness, `design/rumors-frame-fuzz.md`) re-stated in terms of what remains. The instrument's global swap check reads macOS's swap-used figure, which the kernel drains lazily, so it stays above the abort line long after live pressure has passed and kills every gate leg on a machine with tens of gigabytes free; its per-process ceiling has no committed demonstration of a runaway build it caught. Retiring an instrument follows the discipline of landing one, so the retirement states what replaces each check: nothing, by owner decision, on a 128 GiB machine whose concurrent-builder cap is the resource control of record (WORKFLOW.md, at most four builders). If a runaway build is ever observed, that observation is the demonstration a replacement instrument lands with.
> Home: a `p1-memwatch` lane stacked on `p1-gate` (both edit the justfile), brief `briefs/p1-memwatch.md`.

## Ground rules

These apply to every P1 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `<parent sha>` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `<parent sha>`, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The review documents are the specification.** Each member entry below
  quotes its Resolution and Acceptance verbatim from the topic document in
  `.agent-notes/2026-09-01-holistic-review-rumors/`; the entry's full record
  (evidence, construction, demonstration) is under `### <id>:` there and in
  `evidence/`. Line anchors are at the reviewed commit `9e5784fb`; re-anchor
  from the quoted evidence, never from the line numbers.
- **Goal beside mechanism.** Where a quoted resolution and the goal above
  come apart, the goal wins, and the discrepancy is reported.
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin; any change to a public
  signature or public rustdoc contract the resolution does not name;
  anything that contradicts a ruling in `triage/rulings.md`; any deviation
  from the stated resolution; anything this brief marks as a stop. A stop on
  one entry does not block the others.
- **Negative controls.** Every repaired instrument lands with a committed
  demonstration that a known-bad artifact fails it. The constructions in
  `evidence/witness.md` and each entry's Construction line are those
  artifacts; convert each into a committed test (`should_panic`, an asserted
  `Err`, a `--self-test` case, or a reversible mutation whose observed
  failure the commit message records verbatim).
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement. One full `just gate` before each commit,
  run in the background redirected to a log under `<scratchpad>/p1-memwatch/`,
  polled with short foreground checks (the foreground command cap is ten
  minutes). Keep every working file under that directory. `just all` and
  `just ci` are run once each at the end, not per commit.
- **Commits.** One commit per logical unit, its message describing the
  change and naming the entry ids and ruling. Commit every proptest seed
  file that appears. Prose speaks in the present tense: no reference to code
  that no longer exists, no dated rationale at a declaration site. Comments
  use spaced double-hyphens, never em-dashes; every test has a doc comment
  stating its invariant. Never delete anything outside your worktree; if the
  disk fills, stop and report.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For each entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim).
  Then anything left open and why. Your report is data: the coordinator
  verifies each entry's Acceptance against the tree at the reported sha
  before the ledger records it. Report what you could not do rather than
  working around it.

Two ground rules have no object here: the member is not a roster entry
(the ruling above is its specification), and nothing is repaired, so no
negative control lands. The base, filled in at launch, is `p1-gate`'s sha.

## Members

### memwatch-retirement: ruling T136

Resolution: delete `tools/memwatch`. Unwrap every justfile recipe that
runs a command through it, so each runs its command directly with the same
arguments, environment assignments, `[working-directory]` attribute, and
exit status; the only text removed from a recipe line is the wrapper (in
`citecheck`, the `bash -c '...'` exists only to carry a redirect through
the wrapper, so it goes too, leaving the plain command and its redirect).
Re-state every prose mention in terms of what remains, present tense, no
"formerly", no reference to the deleted script or its env knobs
(`PROC_LIMIT_GB`, `SWAP_LIMIT_GB`, `LOG`, `INTERVAL`, `HEARTBEAT_EVERY`)
or its `memwatch.log`. Leave `target/gate-logs` and the `gate-streams`
orchestration as they are: they capture each stream's output and ok/failed
marker, not the watchdog's log; if any of it turns out to exist only for
memwatch, say so and stop.

The footprint, surveyed at `9a6a934e` (re-anchor by grep, never by line;
`.gitignore`, `AGENTS.md`, the READMEs, and the rustdoc carry no mention):

- Wrapper sites, one per recipe: `test`, `test-all`, `doctest`,
  `citecheck`, `fuzz-build`, `bench-build`, `fuzzfit-build`, `fuzzfit`,
  `fuelscape-test`, `surface-totality`, `coverage-kernel`,
  `coverage-kernel-branch`.
- Justfile prose: the comment block above `test` ("Codegen-running recipes
  go through tools/memwatch ..." through the `PROC_LIMIT_GB=64 just test`
  sentence) explains only the wrapper and goes whole. The `gate-streams`
  paragraph opening "Concurrency multiplies peak memory, not just cores"
  keeps that clause and drops the watchdog sentences: nothing in the
  justfile caps memory; how many gates run at once is the operator's call.
- `.github/workflows/ci.yml`, the prerequisites comment: the bullet naming
  "bash for tools/memwatch" restates to python3 for the `tools/` linters
  (bash stays a prerequisite of the `#!/usr/bin/env bash` recipes); the
  paragraph arguing the wrapper is harmless off macOS is deleted. No step
  installs or invokes the script; `tools/workflowlint` must still pass.
- `Cargo.toml`, the `[lib] bench = false` comment: drop the parenthetical
  "(it trips the memwatch per-process ceiling)"; keep the sentence; no
  figure in its place (an unmeasured number is a hypothesis).
- `design/rumors-frame-fuzz.md`, the justfile-wiring bullet: "built under
  `tools/memwatch` with the nightly toolchain" loses its wrapper clause only.

Acceptance: `git grep -n -i -E 'memwatch|SWAP_LIMIT_GB|PROC_LIMIT_GB' -- .
':!.agent-notes'` prints nothing; `test ! -e tools/memwatch` succeeds;
recipe names are unchanged against the parent: `git show <parent
sha>:justfile > <scratchpad>/p1-memwatch/justfile.parent`, then `diff
<(just --justfile <scratchpad>/p1-memwatch/justfile.parent --list
--unsorted | awk 'NR>1{print $1}') <(just --list --unsorted | awk
'NR>1{print $1}')` prints nothing (report any difference in the full
listing too); one full `just gate` clean at the final commit, its log kept
under the lane's scratchpad directory. One commit, naming T136.

## Hazards and stops

- Two lanes in flight edit the justfile: `p1-gate` (this lane's base) and
  `p1-envelope` (only its `window.rs` citation). The rebase order is gate,
  memwatch, envelope; the coordinator rebases envelope onto this lane's sha.
- Any recipe whose behavior changes beyond dropping the wrapper is a stop:
  a different command, argument, environment assignment, working
  directory, or exit status (the wrapper passes its child's status
  through; the only path that disappears is the watchdog's own abort).
- A CI step that installs or references anything for the script's sake is
  edited; a CI step that fails without it is a stop.
- A gate leg failing at the base for an unrelated reason is reported with
  its log, never repaired here.
