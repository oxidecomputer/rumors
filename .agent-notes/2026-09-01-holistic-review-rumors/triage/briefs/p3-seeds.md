<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from ruling T59 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P3 lane: orphaned seeds and the seed-liveness parameter match

> **Ruled after drafting.** T158 item 2: `tests/async_wire.rs` and its seeds are the P5 tests lane's (T131); this lane touches neither.

## Goal

Every committed proptest seed replays a case a live property can generate,
and the tree can tell when one stops doing so. Each orphaned `cc` line
(the awaiting-disposition entry in `shadow_validity.txt`, the
deleted-property line in `retire.txt`, the two `faults.txt` lines whose
notes name values no strategy generates) is removed in a commit naming the
deleted or changed property, or its note corrected where the seed still
replays; the six "minted" comments are restated in the present tense; and
`tests/seed_liveness.rs` gains a check that every seed's shrink-note
parameter names match the live `proptest!` signature it anchors to, so the
next orphan is a failing test rather than a review finding (T59). Effort:
high (the sweep is an instrument; a wrong extension reads dead seeds as
live).

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
  run in the background redirected to a log under `<scratchpad>/p3-seeds/`,
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

1. **The extension first, red on the tree.** In `tests/seed_liveness.rs`,
   for every `cc <hash> # shrinks to <note>` line the existing walk
   resolves to a source file, parse the note's parameter names (`name =`
   tokens, and the names inside a leading `(a, b) =` tuple) and the
   `proptest!` blocks of that source (each `fn name(param in strategy, ...)`),
   and require the note's names to be a subset of one property's parameter
   set in that file. Land it with its fixture negative control in the
   test's existing fixture harness: a seed whose note names a parameter no
   property has fails the sweep; a matching note passes. Run once on the
   tree and record which committed lines it convicts; that list is the
   orphan roster, checked against the four the ruling names.
2. **The seed commits, one per file**, each message naming the property
   that was deleted or whose strategy changed, or stating that the note is
   corrected because the seed still replays: `shadow_validity.txt`,
   `retire.txt`, `tree/mirror/streaming/tests/faults.txt`, then the
   "minted" comments (`session_stats.txt`, `tree/mirror/streaming/tests/stats.txt`,
   `faults.txt`, `materialized/work/tests/violations.txt`,
   `remote/codec/decode/tests.txt`) restated as present-tense statements of
   what the case pins.
3. **Green.** `cargo nextest run -p rumors --test seed_liveness` clean;
   `grep -rn minted proptest-regressions/` empty; no comment in any seed
   file speaks of pending disposition. Never strip a `cc` line outside a
   commit that names its property.

If the sweep's walk covers `crates/`, a mismatch it convicts there is the
`before` triage's: report it as a stop, do not edit it.

## Members

All rows are T59 (owner decision 14); all are heading entries, quoted
with their Acceptance.

- **tests-observation-38** (low, documentation). Resolution: Rule now. Recommended: keep the `cc 86723fa8...` hash as a valid deterministic replay case under the current strategy, delete its stale shrink note after the `#` and the five-line comment, recording the ruling in the commit message; for the six "minted" comments, either delete them (git has the provenance) or restate each in the present tense ("# A deterministic minimal replay case; passes on the current code."). Acceptance: no comment in any seed file speaks of pending disposition; `grep -rn minted proptest-regressions/` is empty; `tests/seed_liveness.rs` still passes. (T59: remove the line naming the changed property, or correct the note where it still replays; "minted" comments are restated, not deleted.)
- **tests-common-32** (low, simplification). Resolution: owner rules on the entry: delete it (the failure it recorded is fixed, and its replay under the current strategy is a different case) or re-derive it by reintroducing the fixed defect on a branch and letting proptest write a fresh seed. Then extend seed_liveness.rs so a shrink note whose field skeleton disagrees with the strategy's current value type fails the sweep (per suite, a required-field list derived from one generated value's Debug rendering, or a per-suite regex), with a fixture seed demonstrating the verdict. Acceptance: no `cc` line in proptest-regressions/ carries a note lacking a field the current strategy always emits; the disposition comment is gone; seed_liveness fails a fixture seed whose note lacks a required field. (T59 fixes the extension's shape: parameter names against the `proptest!` signature, not a Debug-derived field skeleton.)
- **tests-lifecycle-1** (low, simplification). Resolution: Remove the line in a commit whose message names the deleted property, or record in the commit that it is retained deliberately. Consider extending `seed_liveness.rs` to parse each `cc` comment's parameter names against the live `proptest!` signatures in the owning file, so a deleted property's seeds surface mechanically. Acceptance: retire.txt carries only `cc` lines whose parameter names match a live property in tests/retire.rs, or the retention is recorded; if the sweep is extended, a fixture with a mismatched `# shrinks to` fails it.
- **streaming-tests-20** (low, verification). Resolution: Owner call: (a) keep the lines and correct the comments to what the seeds now generate, or (b) remove the two lines in a commit whose message names the orphaning. The "minted ... during mutation review" notes state what the seed pins and are recorded precedent; leave them unless the owner wants the campaign reference dropped. Acceptance: every `# shrinks to` comment in the file names a value the file's current strategies can generate. (T59: the campaign reference goes; present tense.)
- **tests-lifecycle-18** (medium, verification). Resolution: Keep the union law in pairwise.rs (the algebraic-laws file), move the `String` leg there as one generic body called for both payload types, delete tests/async_wire.rs, and rewrite bootstrap.rs:6-8 to point at pairwise.rs or drop the sentence; drop "asynchronous" from common/wire.rs:1. Acceptance: one binary owns the union-of-readouts property for u64 and String; `grep -rl async_gossip_converges tests/` is empty; bootstrap.rs's module doc references no removed file. **Stop:** T59 disposes only this row's "consequence" (the four `async_wire.txt` seeds re-home into `pairwise.txt` in the commit that deletes the binary), and that commit is T131's, in the P5 tests lane; the ledger places the whole row in P3 under T59. This lane does not delete the binary. Report which lane the row belongs to; until then its seeds stay in `async_wire.txt`, where the extension must find them live.

## Hazards and stops

- `proptest-regressions/disruption.txt` and `link/routed/header/tests.txt`
  are added or changed by `p1-harness-tests` and `p2-link` (unmerged);
  launch after the P1 and P2 merges so the extension reads their final
  notes. No other P3 lane touches the seed files or `seed_liveness.rs`, so
  this lane runs concurrently with `p3-modules` and `p3-lints`.
- A seed the extension convicts that the ruling does not name is a
  finding: report it with the line, never delete it in passing.
- A `cc` line's hash is the case; a note is commentary. Correcting a note
  never changes a hash.
