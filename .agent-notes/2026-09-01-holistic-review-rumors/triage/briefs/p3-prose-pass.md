<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from ruling T58 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P3 lane: the dedicated prose pass (intensifiers and rustdoc dashes)

## Goal

Rustdoc reads without the intensifier "genuine(ly)" where it adds no
contrast, and without em-dashes where the dash carries no emphasis: those
become colons, semicolons, or sentence breaks (T58, owner decision 13).
This is a prose pass run with fresh-eyes readers under the documentation
change policy, not a mechanical sweep, and it lands after every other P3
lane so it reads the tree they leave. It carries no ledger rows: the
ruling disposes two prose-hygiene open questions, and the lane reports
findings by file rather than by id. Effort: medium for the pass, high for
its fresh-eyes readers (WORKFLOW.md: a reviewer below the author's effort
confirms rather than disputes).

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
  run in the background redirected to a log under `<scratchpad>/p3-prose-pass/`,
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

1. **The intensifier, one commit.** Every pure-intensifier "genuine",
   "genuinely" leaves; a contrastive use stays (a real socket against an
   in-memory one). Oracle:
   `git grep -n -i -P '\bgenuinel?y?\b' -- src tests benches examples design AGENTS.md README.md`
   prints only lines the commit message lists as contrastive, each with
   the counterpart it contrasts (89 sites at `6c90bd7d`, before the
   T49 and T50 sweeps remove their share).
2. **Rustdoc dashes, one commit per module family**, rewriting toward the
   plainer punctuation wherever the dash carries no emphasis; a dash that
   does carry it stays. There is no numeric threshold: the oracle is the
   fresh-eyes reader's verdict under PROSE.md's three tests, and the
   commit records `git grep -c -P '\x{2014}' -- 'src/*.rs' 'src/**/*.rs'`
   at the parent and at the commit so the movement is attributable, never
   as a gate.
3. **`just readme`** after any crate-doc edit; the READMEs are derived.

`tools/` and `crates/` are the `before` triage's (its ruling 53 sweeps
"genuine" there). The em-dash check `p3-dashes` lands never scans rustdoc;
nothing here widens it.

## Members

None by id. The ruling, quoted from `../rulings.md`:

> ## T58 (2026-09-02): A dedicated prose pass on intensifier "genuine(ly)" and rustdoc em-dash density
> Disposes: owner decision 13; prose-hygiene open questions 2 and 3 (fix)
> Decision: One prose pass, run with fresh-eyes readers per the documentation change policy, removes every pure-intensifier "genuine(ly)" (contrastive uses stay) and rewrites em-dash-heavy rustdoc toward colons, semicolons, and sentence breaks wherever the dash carries no emphasis. It lands after the mechanical P3 sweeps so it reads the tree they leave.
> Home: code.

## Hazards and stops

- Launch last in P3, after `p3-imports` has merged; the pass reads every
  file, so nothing may be in flight beside it.
- Paragraphs Finch wrote stay untouched unless a ruling names them
  (PROSE.md); a reader who cannot tell reports the paragraph rather than
  editing it.
- A rewrite that changes a contract sentence's meaning is a stop; this
  pass changes punctuation and intensifiers, never claims.
