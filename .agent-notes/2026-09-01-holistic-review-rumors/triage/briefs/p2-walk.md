<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P2 lane: the walk's end legs

## Goal

The materialized walk's middle levels classify a malformed reply through
one classifier, the `Resolver`, which maps each shape to the `Violation`
variant whose doc describes it. Its two ends, the terminal leg (`absorb`)
and the opening leg (`responder_level`), have hand-written arms that
disagree with it: too-many-reaction replies reported as `UnfinishedReply`,
a leading or trailing `Match` reported as `UnexpectedQuery`, early supplies
admitted without the structural checks every solicited supply gets, and no
check that the opening's one reply is the only one. None of this is
reachable from a conforming peer; the public taxonomy misdescribes itself,
and no test drives either leg with a malformed shape. The invariant
restored: one classifier serves every height, and every malformed shape at
either end maps to the variant whose doc describes it, pinned by an
injection.

## Ground rules

These apply to every P2 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `62b30e1f` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `62b30e1f`, fast-forward; if it has diverged, stop and report. Never call
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
  run in the background redirected to a log under `<scratchpad>/p2-walk/`,
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

## Ordering inside the lane

1. The recorded opening-leg liveness gap first: a violation raised in the
   responder's early-supply loop before the stage's one reply yields must
   return an error over a wire, not stall. Construct the probe the
   recorded commit held back, observe the stall, fix it, and commit the
   probe as the negative control.
2. Then route both legs through `Resolver::react`. If the ends genuinely
   differ from the middle in a way the `Resolver` cannot express, fall
   back to per-arm reclassification, and report the difference; do not
   assume it.
3. Then the early-supply structural checks and the trailing-reply check,
   each with its injection.

## Members

### materialized-14 (low): ruling T40, resolution amended toward the shared classifier

Resolution: In `absorb`, classify by the first offending reaction (`Match` -> `UnexpectedMatch`, `Query` -> `UnexpectedQuery`, a second reaction after a supply -> `UnexpectedSupply` or `InvalidSupply` by radix), and match over the owned `Vec` rather than `as_slice()` so the accepted leaf moves instead of cloning (`Some(leaf)`, removing the one `Arc` clone per requested leaf at 892). In `responder_level`, report a missing opening query as `UnfinishedReply` (or a documented new variant) and a non-`Supply` trailing reaction as `UnexpectedMatch`; check early radices strictly ascending and absent from `fan`, reporting `InvalidSupply`/`UnexpectedSupply`; add the trailing `requests.next()` check. Alternatively route both legs through `Resolver::react` so one classifier serves every height. Before landing the opening-leg changes, resolve the 50c8b0a3 liveness gap (a violation before the opening's one yield must abort typed over a wire). Then add `Completing`- and opening-leg injections to `violations.rs`, and widen `arb_connected_violation` to every variant `Faulting` can script. Acceptance: each malformed terminal and opening reply shape maps to the `Violation` whose doc describes it, pinned by committed injections that fail on the current arms; no `.clone()` in `absorb`; the four `terminal_absorb_*` tests still pass.

Amendment (T40): the "alternatively" branch is tried first, per the
ordering above. The constructions in the entry (the `absorb_scripted`
shapes and the two `responder_level` opening replies) are the injections;
each must fail on the current arms and pass after. A "documented new
variant" on the public `Violation` enum is a stop: use the existing
variants, and if none describes a shape, report it.

### Vocabulary: "fails typed", "aborts typed", and kin (T40)

The phrase family is excised crate-wide in favor of "returns an error".
The census at the base commit finds about sixteen lines, in
`src/tree/mirror/streaming/remote/adapter/tests/malformed.rs` and
`src/tree/mirror/streaming/remote/codec/decode/tests.rs` among others,
outside this lane's files. Land the sweep as one commit in this lane
anyway (it is prose only and touches no lane's code), running the
regenerating grep to zero in the same commit:

    grep -rn -i -E '\b(fail|fails|failed|abort|aborts|aborted)\s+typed\b|\btyped\s+(fail|abort|error)' src tests benches examples justfile

If a P1 lane has landed edits to a file the sweep touches, rebase first.
Report the final line count and the commit.

## Hazards and stops

- `MaterializedViolation` and `Violation` are public; no variant is added,
  removed, or renamed. The `Display` strings are unchanged. Any of these
  is a stop.
- The liveness fix in step 1 touches the walk's stage scheduling; the
  deadlock-freedom argument is enforced by `yield_resolve_query!` and must
  not be bypassed. If the fix needs a new yield point, state where it sits
  in the publication order and why the argument survives.
- The connected fault suite's `0..=15` step range cannot reach the
  terminal reply phase on the depth-32 comb (materialized-39, P4); do not
  widen it here.
