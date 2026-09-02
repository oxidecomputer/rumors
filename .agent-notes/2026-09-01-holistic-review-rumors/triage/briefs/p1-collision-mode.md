<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P1 lane: the collision-schedule test mode

## Goal

No frame ever crosses a link on a logical stream with index two or
greater anywhere in the crate (demonstrated by a frame census in the second
witness pass), so the leaf-tier decode pump, the terminal stream grammars,
and fifteen of seventeen stream labels run on live traffic under nothing.
The cause is structural: a stream index of two needs a dispute at a
height-29 node, which needs leaves sharing a three-byte path prefix on both
sides, and under blake3-derived leaf paths that is a birthday search no
fixture can afford; deeper streams need longer shared prefixes still. The
witness pass also showed the proposed closing test cannot work: the deep
fixtures place leaves at hand-picked paths that violate version addressing,
and the wire correctly refuses them. Ruling T23 therefore resolves this
through the collision-schedule test mode designed in
`.agent-notes/2026-08-21-collision-schedule-test-mode/README.md`: an
injective, collision-planning leaf-path schedule, behind `test-internals`,
that lets every behavioral suite run under geometry blake3 cannot produce,
through the public API. The invariant restored: every code path
conditioned on deep branching is exercised by some committed check.

Read the design note in full before writing code; it is the design of
record and this brief does not restate it.

## Ground rules

These apply to every P1 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `0926fe32` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `0926fe32`, fast-forward; if it has diverged, stop and report. Never call
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
  `Err`, or a reversible mutation whose observed failure the commit message
  records verbatim).
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement. One full `just gate` before each commit,
  run in the background redirected to a log under
  `<scratchpad>/p1-collision-mode/`, polled with short foreground checks
  (the foreground command cap is ten minutes). Keep every working file
  under that directory. The first full-suite sweep in the mode is one run,
  captured whole to a log; triage from the log, do not re-run the suite to
  re-observe.
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

## The work, in order

1. **The seam.** In `Path::for_leaf` (`src/tree/typed/path.rs`), behind
   `test-internals`, a schedule that plans collisions per the note: cluster
   assignment as a pure function of (seed, version bytes) through a seeded
   non-cryptographic PRF; prefix length drawn over `0..=31` with weight on
   31; distinct final bytes within a cluster; a reverse set that resolves
   any planned full-width collision by deterministic spill, so full 32-byte
   paths are injective always. Merkle content hashing is untouched.
   Unset variable means blake3 and byte-identical behavior.
2. **nextest enforcement.** The installer checks
   `NEXTEST_EXECUTION_MODE=process-per-test` when the variable is set and
   panics with a message naming the requirement and its reason.
3. **`assume_blake3()`.** A `test-internals` function that forces the seam
   to blake3 for the process, panicking if the schedule already
   initialized. Each call site carries a one-line rationale.
4. **A unit suite for the schedule itself.** Injectivity over full paths
   (a proptest over version bytes at a fixed seed), determinism (same
   bytes, same path, independent of arrival order), and a liveness pin that
   a sample of the schedule produces at least one 31-byte shared prefix
   (the length that makes the leaf arm live). Negative control: a schedule
   with the spill disabled fails the injectivity proptest.
5. **The first-sweep triage.** One run of the full suite under the mode.
   Every failure resolves to exactly one of: a bug (a finding, reported
   with the failing test and its output, left as a stop for the
   coordinator); or a legitimate blake3 assumption, marked with
   `assume_blake3()` and its rationale. Marks are reviewed as a set; the
   report lists every mark with its rationale. Known candidates the note
   names: the byte-exact wire snapshot suites, structure pins over concrete
   inserts, the geometry-searching generators in `src/tree/arb.rs`, and
   possibly the uniform-hash quantitative suites.
6. **The recipe.** A justfile leg running nextest with the variable set at
   a pinned seed. Its tier is a stop (below).
7. **The acceptance test for `remote-proxy-tests-10`.**

## Members

### remote-proxy-tests-10 (high): ruling T23, resolution replaced

The entry's original resolution ("Drive the deep fixtures through the
production topology...") is superseded by T23; do not attempt it. Its
Acceptance stands, reached through the mode:

Acceptance: a test in this partition, driving `RemoteHandshaking` over a link, observes frames on a logical stream with index >= 2 in each direction (via `IoReport.connects >= 3` per side, or a hook capture) with reconciled roots equal to the oracle; the leaf-parent dispute and redaction fixtures both converge over the wire; a wire snapshot renders a stream header deeper than stream 1.

Amendments (T23): the frame census is run under the mode, and the test
skips with a named reason (not a silent pass) when the mode is off, so
the recipe leg is where it counts; the "leaf-parent dispute and redaction
fixtures converge over the wire" clause is replaced by a proptest over
`arb_deep_divergent_pair()` (or the mode's own generated pairs) at this
tier with `Tree::join` as oracle; the snapshot clause is dropped here
(see the note on `tests-wire-format-7` below). The witness pass's frame
census (`evidence/witness.md`, section `remote-proxy-tests-10`) is the
model for the observer.

### tests-wire-format-7 and tests-wire-format-18: P4, not this lane

Their streams-two-and-up halves are carried by T23; their remainders (a
depth floor on `deep_trie_divergence`, negative walker cases) stay in P4.
A committed `insta` snapshot taken under the mode would differ from the
blake3 snapshots and is a stop; do not add one. If the mode makes a
legibility-corpus capture with three or more data streams cheap, say so
in the report.

## Stops: the design note's four open questions

Each of these is Finch's to rule. Take a provisional answer, isolate it in
one named place, and list it in the report as awaiting ruling; do not
treat any as settled, and do not spend runs sweeping alternatives.

1. The environment variable's name and seed format (provisional:
   `RUMORS_PATH_SCHEDULE=<u64>`, the note's own example).
2. One pinned seed per leg versus a seed sweep (provisional: one seed).
3. Gate tier versus `ci` tier for the recipe (provisional: `ci` only;
   record the mode's full-suite wall time from the one sweep in the report
   so the ruling has its number).
4. Whether the schedule also plans partial clusters (30-, 29-byte prefixes)
   at higher weight (provisional: the `0..=31` draw as the note states).

## Hazards

- `src/tree/typed/path.rs` is production code. The seam must compile to
  the bare blake3 call without `test-internals`; a committed test asserts
  that the production feature set produces the blake3 path for a fixed
  version (byte-identical to today), and the wire snapshots, which run
  without the variable, must not move.
- The mutation campaign is disputed (T11); the note's remark that the
  campaign must include the collision leg does not apply.
- An agent under completion pressure marks to green. Every mark's
  rationale must name what the test pins about blake3 geometry; "fails
  under the mode" is not a rationale. A mark on a shared helper widens
  silently; the ordering guard in `assume_blake3()` is what prevents it,
  and it stays.
- This lane rebases onto `p1-harness-crate` before its sweep, so the
  sweep covers the tests that lane adds.
