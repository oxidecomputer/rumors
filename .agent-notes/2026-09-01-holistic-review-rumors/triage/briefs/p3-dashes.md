<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from ruling T48 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P3 lane: em-dashes out of line comments, held by a gate check

> **Ruled after drafting.** T158 item 7: the workspace em-dash check is the `before` triage's to ship first; adopt it.

## Goal

The doctrine assigns the spaced double-hyphen to code comments and the
em-dash to rendered prose. Every U+2014 in a `//` comment, a `#` comment,
or a string literal leaves the tree, and a `tools/` check in `just gate`
keeps it out, so this sweep is the last one anyone runs by hand (T48).
Rustdoc em-dashes are outside this lane (owner decision 13, T58). The
check is shared with the `before` triage (its rulings 7 and 54: one
workspace check, shipped by whichever lane lands first, the other sweep
run against it). Effort: medium (a grep oracle and a build-free checker).

## Ground rules

These apply to every P3 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `<base sha>` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `<base sha>`, fast-forward; if it has diverged, stop and report. Never call
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
  run in the background redirected to a log under `<scratchpad>/p3-dashes/`,
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
- **Prose.** `PROSE.md` in this directory binds this lane (ruling T141): every
  paragraph you touch passes its three tests (altitude, concision,
  legibility), and the diff is net shorter in prose unless your report says
  what the added sentences buy.
- **Machine.** The illumos box (`ox-east-1`, per the `building-on-illumos` skill) is
  where a lane builds, tests, and gates. The wrapper syncs the Mac
  worktree to `~/src/<worktree basename>` on the box and runs one command
  there with its own target directory, so lanes do not collide; cargo
  runs `--locked` there; nothing is edited or committed on the box. A
  clean gate on the box is the gate of record for a commit; the Mac runs
  no gate (Finch's ruling). The box gate is `on-illumos.sh <worktree> 'just gate'`. One leg is
  expected red there and counts as clean when it is the only failure:
  `fuzz`, because libFuzzer has no illumos port (`FuzzerPlatform.h`
  refuses the target); a lane quotes that line and runs no fuzz build
  elsewhere (Finch's ruling: fuzzing is CI's). clippy's
  `missing_const_for_thread_local` misfires on illumos, where
  `thread_local!` expands through the OS-keyed path; `before` carries an
  illumos-scoped crate-level allow, so a lane based before that landed
  either rebases or passes `RUSTFLAGS="-A clippy::missing_const_for_thread_local"`
  in the remote command for that one run. Two legs that
  pin toolchain-derived numbers may fire on the box if its toolchains
  differ from the pinned ones; a lane reports such a leg with both numbers
  rather than re-pinning anything. `tools/memwatch` is deleted by the
  `p1-memwatch` lane, which runs first and unstacked; `p1-gate` rebases
  onto it. Benchmarks whose committed baselines are the Mac's run on the
  Mac, once, on a quiet machine. Clock guard, checked before every box
  run: rsync preserves mtimes and cargo's rebuild detection is
  mtime-based, so a box clock ahead of the Mac by more than a couple of
  seconds means a green build of stale code; on skew, a lane either runs
  with a fresh target directory on the box (a cold build, no stale
  artifact to trust) or waits, and says which. Stepping the box's clock is
  admin work on a shared machine and is Finch's, never a lane's. The
  builder cap counts Mac builders only; on the box (96 cores, 1 TiB) there
  is no lane cap, only the load: hold a launch while the one-minute load
  average sits above about 150 on 192 threads, and keep wall-time
  measurements under `pset-run` to one at a time, announced in the merge
  queue first.
- **Out of scope.** The formal tier (`lean`, `eventdag`, `muxprobe`, everything under
  `formal/`) and `before`'s bench judge (`bench-judge`,
  `bench-judge-tripwire`) are never run or edited by a rumors lane; a
  recipe that composes them (`all`) is exercised by its other legs
  individually. A rustdoc on a Rust-side literal derived from the Lean
  artifact is Rust prose and may be edited where a ruling names it.

## Mechanism, in order

1. **The check, one commit.** If `tools/` already holds the workspace
   em-dash check at base (the `before` lane landed first), adopt it and
   add this crate's roots to its recipe; else ship `tools/dashlint` (the
   name is the lane's call; the shape is doclint's: python3, explicit
   roots, `--self-test` first). The rule: in a `.rs` file, every line
   whose trimmed start is not `///`, `//!`, or `#[doc`/`#![doc` must not
   contain U+2014 (this covers `//` comments, trailing comments, and string
   literals); in `justfile`, `.toml`, `.yml`, and `.yaml` files, no line may.
   Markdown is never scanned. Roots in the recipe for this lane:
   `src tests benches examples justfile Cargo.toml deny.toml .cargo .config .github`;
   `crates` and `tools` are the `before` sweep's roots (its ruling 54) and
   join the recipe when that sweep lands. Wire it into `gate-lints` with a
   two-line recipe comment. Self-test cases, each the negative control: a
   `//` line with the character fails; a `///` line passes; a string
   literal fails; a justfile `#` line fails; a `.md` file is not scanned.
2. **The sweep, one commit per file family** (crate root and peer; tree;
   mirror; remote; testing scaffold; integration suites; the config files),
   each rewriting the character as ` -- `, a colon, a semicolon, or a
   sentence break, whichever reads best at the site, with the four assert
   strings taking a colon or semicolon (prose-hygiene-8). Where a row names
   other defects on the same lines (remote-adapter-tests-18's
   "compiler-checked" and the unexplained readout), land them in the same
   edit; the lane owns the prose it passes through.
3. **The gate leg at zero** in the sweep's last commit, and the surveyed
   census in its message.

Oracle, empty output is the acceptance (survey at `6c90bd7d`, rumors
paths: 160 leading `//` lines, 12 trailing comments, 32 string-literal
lines; justfile 57, `.cargo/mutants.toml` 20, `ci.yml` 9, `Cargo.toml` 5,
`nextest.toml` 3, `pages.yml` 1, `dependabot.yml` 1, `deny.toml` 1;
re-derive by grep, never from these numbers):

    git grep -n -P '\x{2014}' -- src tests benches examples | grep -v -P '^[^:]+:[0-9]+:\s*(///|//!|#\[doc|#!\[doc)'
    git grep -n -P '\x{2014}' -- justfile Cargo.toml deny.toml .cargo .config .github

## Members

Nit rows are one-line table rows in the topic document (`| <id> |` under
its module heading), quoted as `Resolution:` with no Acceptance of their
own; heading entries quote both. All rows are T48 (owner decision 3)
except the last; "Crate-wide pattern" as a Resolution means this sweep.

- **prose-hygiene-8** (low, documentation). Resolution: Replace with a colon or semicolon in each of the four strings. Acceptance: the grep above returns nothing.
- **prose-hygiene-9** (nit, the master census). Resolution: Crate-wide pattern; see the patterns section.
- **conformance-19** (nit). Resolution: Crate-wide pattern (owner-gated ruling plus a lint).
- **materialized-38** (nit). Resolution: Crate-wide pattern.
- **remote-adapter-streams-7** (nit). Resolution: Crate-wide pattern.
- **remote-adapter-tests-18** (nit). Resolution: `const` assert or "pinned"; ` -- `; state the readout's purpose.
- **remote-proxy-5** (nit). Resolution: replace with `--`, a colon, or a semicolon (start.rs:173 and 220 collapse to one site under remote-proxy-4)
- **remote-proxy-tests-19** (nit). Resolution: Crate-wide pattern.
- **session-bookmark-6** (nit). Resolution: Crate-wide pattern.
- **streaming-backend-window-7** (nit). Resolution: Crate-wide pattern.
- **streaming-tests-4** (nit). Resolution: Replace the eight em-dashes with a colon, semicolon, or ` -- `; add a `tools/` check for the character on `//` lines
- **testing-infra-14** (nit). Resolution: Rewrite each with a colon, semicolon, or parentheses
- **tests-bookmark-10** (nit). Resolution: a colon in the assert message
- **tests-common-24** (nit). Resolution: Crate-wide pattern.
- **tests-disruption-handshake-13** (nit). Resolution: Rewrite the twelve sites with colons, semicolons, or parentheses
- **tests-observation-33** (nit). Resolution: Replace `—` with ` -- ` or a colon at party_conservation.rs:93-94, causal.rs:144-145, 555-556, 655-656, listen.rs:497, 517, 634
- **tests-resource-link-window-23** (nit). Resolution: Replace with a colon, semicolon, or sentence break at each site
- **tests-wire-format-13** (nit). Resolution: Replace with a colon or semicolon at both sites
- **tree-core-15** (nit). Resolution: Crate-wide pattern.
- **swarm-example-11** (nit, T27, moot). Resolution: Crate-wide pattern. Disposed by the example's deletion (`p1-swarm`); the lane's only act is `test ! -e examples/swarm.rs` at base, quoted in the report.

## Hazards and stops

- The rows' line anchors sit in files the unmerged P1 and P2 lanes rewrite
  (`tests/bookmark_causality.rs` and `tests/common/flaky.rs` in
  `p1-causality`; `src/conformance/link/tests.rs`, `src/testing/transport.rs`,
  `tests/gossip_when.rs`, `tests/common/sim.rs` in `p1-harness-tests`;
  the justfile, `ci.yml`, and `Cargo.toml` in `p1-gate`, `p1-memwatch`,
  `p1-renderer`); the grep re-anchors every site. Launch after the P1 and
  P2 merges and after `p3-vocabulary`, whose prose edits remove some of
  these lines themselves; `p3-imports` follows this lane.
- A `.rs` line that is neither comment nor string but carries the
  character (a `char` literal, a test fixture that must contain it) is a
  finding about the rule: report it with the site, and do not exempt it in
  the tool without a ruling.
- If `before`'s check lands first with a different rule for `.rs` lines
  (its ruling 7 says "outside rustdoc and Markdown"), adopt its rule and
  report the difference; two checkers for one character is the failure
  mode the phase exists to end.
