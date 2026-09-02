<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from rulings T46 and T53 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P3 lane: the pub-in-private convention and test-module placement

## Goal

Two module conventions the review found unstated or unenforced become one
stated line and one check. Under a private module, `pub` and `pub(crate)`
are both admitted and carry no API meaning; AGENTS.md says so in one line,
and the comment at `src/tree/typed.rs` on `pub(crate) mod untyped` is
reconciled with it (T46: the thirteen rows are `model`; no visibility
changes). Every test module lives in a sibling `tests.rs`, without
exception; the six inline bodies move, and a `tools/` check in `just gate`
holds the rule (T53). Effort: medium (pure moves, one prose line, one
build-free checker with a self-test).

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
  run in the background redirected to a log under `<scratchpad>/p3-modules/`,
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

1. **T46, one commit.** One AGENTS.md line stating the convention (the
   section is the lane's call; it is a code convention, not a hard rule the
   gate holds). Proposed text: "Under a private module, `pub` and
   `pub(crate)` are both admitted and carry no API meaning; the public
   surface is what `src/lib.rs` re-exports." Restate the `typed.rs` comment
   so it says the `pub(crate)` on `mod untyped` exists for rustdoc link
   resolution, consistent with that line. Nothing else changes: the rows'
   "drop `pub(crate)`" and "tighten to reach" halves are dissolved by the
   model, and their doc-voice halves (rustdoc on unreachable items written
   for a library user, `Message` among them) are the P5 module lanes'.
2. **T53, the moves, one commit.** Six bodies to sibling files:
   `src/tree/mirror/streaming/driver/tests.rs`, `src/testing/tests.rs`,
   `src/testing/transport/tests.rs`,
   `src/tree/mirror/streaming/backend/local/adversarial/tests.rs`,
   `src/tree/mirror/streaming/testing/failing/tests.rs`, and
   `src/tree/arb/tests.rs` (`arb.rs`'s block is `mod test`, singular;
   rename it, and drop the inner `#[cfg(test)]` that `arb.rs`'s own gate
   already implies). Pure moves: `#[cfg(test)] mod tests;` stays behind,
   imports become `use super::*`-style as each body needs, no test is
   reworded. `std::pin::pin!` replaces `pin_mut!` in the two `testing`
   bodies (testing-infra-5). Liveness: `cargo nextest list -p rumors` at the
   parent and at this commit name the same tests, diffed in the log.
3. **T53, the check, one commit.** `tools/modlint` (the name is the
   lane's call; the shape is doclint's: python3, explicit roots, a
   `--self-test` that runs first): a violation is a line declaring a module
   named `tests` or `test` with a brace body, outside comments and strings.
   Roots in the recipe: `benches crates examples src tests`, so
   `.agent-notes`, `.claude`, and `target` are never walked. Wire it into
   `gate-lints` beside `testdoc`, with the two-line recipe comment the
   other lint recipes carry. Self-test cases, each the negative control:
   an inline `mod tests {` body fails; `mod tests;` passes; `mod tests {`
   inside a `//` comment or a string passes; an inline `mod test {` fails.
   Oracle on the tree: `./tools/modlint benches crates examples src tests`
   prints nothing, and
   `git grep -n -P '^\s*(pub(\([^)]*\))?\s+)?mod\s+tests?\s*\{' -- '*.rs'`
   prints nothing.

## Members

Nit rows are one-line table rows in the topic document (`| <id> |` under
its module heading), quoted as `Resolution:` with no Acceptance of their
own; heading entries quote both.

### T46 (owner decision 1): `model`; no code change beyond step 1

- **api-audit-14** (low, api). Resolution: make the constructors and schedule helpers `pub(crate)`; keep the read accessors (`Stream::index`, `Speaker::role`, `InvalidSignalPlacement::{stream, class}`). Acceptance: the rendered `error/enum.Origin.html`, `error/struct.CodecEncodeError.html`, `error/struct.CodecDecodeError.html`, and `error/struct.Stream.html` list no constructors and no height arithmetic. (Its narrowing is T63's, P6; here it is a `model` row.)
- **inventory-10** (low, simplification). Resolution: Decide once. Either add `#![warn(unreachable_pub)]` to lib.rs and convert the flagged items to `pub(crate)` (the rustdoc-link rationale holds under `pub(crate)`), rewriting `Message`'s rustdoc at maintainer altitude as part of it; or record the pub-in-private convention in AGENTS.md so it is deliberate and reconcile typed.rs:19-22 with it. Acceptance: `unreachable_pub` is enabled and clean, or AGENTS.md states the convention. (T46 takes the second arm.)
- **inventory-14** (nit). Resolution: Drop `pub(crate)` on message.rs:91 `recursion_limit`, 175 `PayloadSerializer`, 181 `PayloadDeserializer`, 368 `try_from_arc`
- **inventory-19** (nit). Resolution: Drop the `pub(crate)` on both fields
- **link-22** (nit). Resolution: drop `pub(super)`
- **materialized-6** (nit). Resolution: Pick one spelling for the walk's three item types
- **mirror-common-20** (nit). Resolution: Make the four driver items private and `descend` private
- **module-graph-12** (nit). Resolution: Normalize the ten `pub mod` sites to `pub(crate)`, or adopt `#![warn(unreachable_pub)]` and follow every diagnostic
- **remote-codec-21** (nit). Resolution: Tighten the listed constants and helpers to their reach; replace the `thiserror::Error` derives on `StreamClass`/`FramePart` with `Display`; name `Entry::Complete`'s fields
- **session-bookmark-17** (nit). Resolution: Drop the `pub(crate)`
- **session-bookmark-44** (low, api). Resolution: If `Message` stays internal: `pub(crate)` the type and its methods, recast the docs at maintainer altitude, move `try_new`, `from_slice`, `from_bytes`, `from_arc` under `#[cfg(test)]` or into the test modules that use them, correct `try_new`'s doc to name `try_from_arc` as the send path, and add `#![warn(unreachable_pub)]` crate-wide (other partitions will show the same pattern). If `Message` is meant to become public API: re-export it deliberately from lib.rs so the docs have a real reader. Acceptance: either `unreachable_pub` is clean for message.rs, or `rumors::Message` appears in the public re-exports with its docs reviewed for that reader; `try_new`'s doc names `try_from_arc`. (The doc-voice half is the P5 core lane's.)
- **streaming-backend-window-2** (nit). Resolution: Enable `#![warn(unreachable_pub)]`, or record in AGENTS.md that bare `pub` inside private modules is the crate's style
- **tree-typed-1** (low, simplification). Resolution: pick one convention for src/tree/typed: either `#![warn(unreachable_pub)]` at the crate root with every item under `tree` spelled `pub(crate)`, or plain `pub` throughout with the `pub(crate)` modifiers dropped. The typed.rs:19-21 comment about rustdoc link resolution concerns `mod untyped` and stays valid under either. Acceptance: one spelling per item kind across the subtree; if `unreachable_pub` is adopted, `just clippy` is clean under it. (T46 admits both spellings; the comment is reconciled, nothing respelled.)

### T53 (owner decision 8)

- **module-graph-6** (low, simplification). Resolution: Move each body to a sibling file (`driver/tests.rs`, `testing/tests.rs`, `testing/transport/tests.rs`, `backend/local/adversarial/tests.rs`, `streaming/testing/failing/tests.rs`, `tree/arb/tests.rs`), leaving `#[cfg(test)] mod tests;`; rename `arb`'s `test` to `tests`. Pure moves. Acceptance: the grep above returns nothing.
- **mirror-common-22** (xref): cross-reference to module-graph-6; carried by it.
- **suite-economics-11** (nit). Resolution: carried by module-graph-6
- **streaming-backend-window-14** (nit). Resolution: Move each block to a `tests.rs` sibling, or amend the convention to exempt test-only modules explicitly (my recommendation (T53 takes the first arm, no exemption.)
- **testing-infra-5** (low, simplification). Resolution: Move the blocks to `src/testing/tests.rs` and `src/testing/transport/tests.rs`; use `std::pin::pin!` in place of `pin_mut!`. Acceptance: `grep -rln '^mod tests {' src/testing.rs src/testing/transport.rs` is empty; the four moved tests still run.
- **tree-core-25** (nit). Resolution: Move `distinct_indices_are_pairwise_disjoint` to `src/tree/arb/tests.rs` behind `mod tests;` and drop the inner cfg
- **module-graph-14** (nit). Resolution: Move each inline module body to a sibling file (`tree/meter.rs`, `tree/panic_injection.rs`, `erased/ops.rs`, `untyped/census.rs`, `decode/fan_probe.rs`, `backend/ledger.rs`). **Stop before touching it:** T53 says these five production modules are "a separate P5 question and untouched here", yet the ledger places the row in P3 under T53; report, do not move them.

## Hazards and stops

- The check's reading is a question for the coordinator before launch:
  T53's words, "no `.rs` file contains an inline `cfg(test)` module body",
  read literally, fire on `src/tree.rs`'s `#[cfg(test)] pub(crate) mod meter {`,
  which the same ruling leaves to P5 (module-graph-14). Step 3 checks by
  module name (`tests`/`test`); if the literal reading is meant, the
  module-graph-14 stop must be ruled first.
- `src/testing.rs` and `src/testing/transport.rs` are rewritten by
  `p1-harness-tests` (unmerged); `AGENTS.md` by `p1-renderer` and later by
  `p3-vocabulary`'s T57 line; the justfile by `p1-gate`, `p1-memwatch`,
  `p1-renderer`, and every P3 lane adding a gate leg. Launch after the P1
  and P2 merges; `p3-vocabulary` rebases onto this lane's sha. A move that
  changes a test's body, name, or doc comment is a deviation: stop.
