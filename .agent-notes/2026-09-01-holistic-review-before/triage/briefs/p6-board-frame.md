<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P6 lane, board 1 of 6: the frame (board.rs, ceilings, cell, coverage, currency, defect, export)

## Goal

The amplification board's frame entries, landed per their Resolutions inside an approved roster, under rulings 38 (no segments currency), 9 and 73 (the exponent ceiling's doc and the affine-residual leg), 43, and 88.

## Roster summary

0 ruled (); 0 medium ruled (93 to 103); 9 roster members approved (ruling 104) (4 low, 5 nit).

## Ground rules

These apply to every lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `5328537c` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `5328537c`, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The review documents are the specification.** Each member entry below
  quotes its Resolution and Acceptance verbatim from the class document in
  `.agent-notes/2026-09-01-holistic-review-before/`; the entry's full
  record (evidence, construction, witness outcome) is under `### <id>:`
  there (`#### <id>:` in `simplification.md`; nits in `documentation.md`,
  `simplification.md`, and `verification.md` are one-line table rows) and
  in `evidence/`. Line anchors are at the reviewed commit `9e5784fb`, and
  the tree outside `.agent-notes/` is byte-identical at your base, so they
  hold; re-anchor from the quoted evidence, never from a line number
  alone, once your own commits move the file.
- **The rulings govern.** `triage/rulings.md` records Finch's decisions.
  Where a quoted Resolution offers alternatives, the ruling named beside
  it picks one; where the ruling amends the Resolution, the amendment is
  stated under the quote and wins. Where a quoted Resolution and the lane
  goal come apart, the goal wins, and the discrepancy is reported.
- **Typed references, never strings (ruling 43).** Wherever this lane
- **Prose (ruling 105).** Every paragraph you touch passes the three tests in `PROSE.md` (altitude, concision, legibility); the reviewer applies its checks; the diff is net shorter in prose unless your report says what the additions buy.
  touches `meter/registry.rs`, a family roster, `TRIPWIRE_ROSTER`, the
  surface rosters, or any test that names another test, file, or line:
  reasons, pins, and enforcement homes are expressed as references the
  compiler resolves (function items, registered law names, `Shape` and
  `Op` values), never as strings naming a test function, a file, or a
  line number. A lane that sees a cleaner idiomatic shape for a roster is
  authorized to adopt it and reports the reshaping in its diff. Finch's
  words: "please make these instruments impossible to drift in the
  future. I *really don't like* the pattern of hard-coded strings and
  Rust source locations embedded in tests; the way these family rosters
  ended up is not really to my taste, but I haven't had time to make it
  more idiomatic and obviously correct. If you see a good way to clean it
  up, please do."
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin the brief does not
  name as moving; any change to a public signature or public rustdoc
  contract the resolution does not name (the meter surface under
  `any(test, feature = "meter")` is instrument surface by ruling 8 and
  not public API, but a change there lands with the `rumors` test update
  in the same commit); anything that contradicts a ruling; any deviation
  from the stated resolution; anything this brief marks as a stop. A stop
  on one entry does not block the others.
- **Negative controls.** Every repaired or added instrument lands with a
  committed demonstration that a known-bad artifact fails it. The
  constructions in `evidence/witness.md` and each entry's Construction
  line are those artifacts; convert each into a committed test
  (`should_panic`, an asserted `Err`, a judge test over synthetic samples,
  or a reversible mutation whose observed failure the commit message
  records verbatim). Ruling 20 is the one place this brief set says
  otherwise, and it says so at the entry.
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement; launch no daemons or services. One full
  `just gate` per agent, before the final commit, run in the background
  redirected to a log under `<scratchpad>/<lane>/` and polled with short
  foreground checks (the foreground command cap is ten minutes; a
  foreground gate run cannot finish). Keep every working file and evidence
  log under that directory, never loose at the scratchpad top level.
- **Commits.** One commit per logical unit, its message describing the
  change and naming the entry ids and the ruling. Every re-pin is measured
  at the parent and its movement named in the commit. Commit every
  proptest seed file that appears. Prose speaks in the present tense: no
  reference to code that no longer exists, no dated rationale at a
  declaration site. Comments use spaced double-hyphens, never em-dashes;
  every test has a doc comment stating its invariant. Commit with a
  descriptive message before finishing.
- **Never delete anything outside your worktree.** If the disk fills
  (ENOSPC), stop and report; it is the coordinator's problem, not a
  reason to trim caches you do not own.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For each entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim).
  Then anything left open and why, and every deviation from a stated
  resolution, explicitly. Your report is data: the coordinator verifies
  each entry's Acceptance against the tree at the reported sha before the
  ledger records it. Report what you could not do rather than working
  around it.
- **No lower bound on performance is pinned anywhere (ruling 88).** Every
  touch, scan, limb, or heap pin is a ceiling: a reading over it fails; a
  reading under it is an improvement that lands by tightening the ceiling
  with its attribution in the commit. The only floors are liveness floors
  derived from a mechanism's irreducible work, never from a measured
  reading. An entry whose Resolution asks for an exact pin or a measured
  floor is read as a ceiling plus any mechanism-derived floor it names.

## Ordering

Every P6 lane runs after the P1 to P5 and P7 lanes that touch its files have landed on main, or rebases onto them before its final gate run; the coordinator names the base SHA at launch. Lows and nits inside this lane's approved roster are swept without a question to Finch; every high and medium has an individual ruling. A change that would alter a rendered `before` doc panel is a stop (ruling 89). Follows `p1-board`, `p1-harness`, `p4-rosters`. Owns `src/meter/board/{board,ceilings,cell,coverage,currency,defect,export}.rs` and their tests.

## Roster members approved (ruling 104)

Lows and nits approved as this lane's roster by ruling 104. Land each per its quoted Resolution and Acceptance, swept with the ruled members; report rather than choose if a Resolution conflicts with a ruling or offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89).

### board-frame-10 (low, documentation): roster: approved (ruling 104)

`TICKS_BOARD_COUNT`'s one-line proof names the wrong leg

- Owner-gated: no

Resolution: "...an implementation iterating even a fraction of the count multiplies every per-byte constant by that fraction of 512, far over the scan, limb, and touch ceilings, so it cannot hide in headroom." Acceptance: the sentence names the constant ceilings as the leg that fires.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-frame-17 (low, documentation): roster: approved (ruling 104)

The heterogeneous `clock | version` joins cite `clock_hash`, a row that prices a byte compare

- Owner-gated: no

Resolution: Replace `clock_hash` with `version_join_assign` (the `|=` the impls run) or `version_join` as the `Clock::absorb` entry does; `clock_recv` may stay as the module doc's stated mechanism (recv is absorb plus tick). Acceptance: the entry cites only rows whose mechanism the impls execute; `board_coverage_tiles_the_public_surface` stays green. Construction: Change `clock_hash` here to any other live row name (e.g. `rank_decode`) and run the coverage tests: the tiling test still passes, showing it cannot distinguish a right citation from a wrong one.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-frame-24 (low, simplification): roster: approved (ruling 104)

`BenchCell::denominator_bytes` re-implements `measure`'s denominator rule by hand and runs the body when the denominator does not need it

- Owner-gated: no

Resolution: One method on `Cell` (or `Denom`), e.g. `fn exponent_denominator(&self, op: &str, content: Option<usize>, result: impl FnOnce() -> Box<dyn Any>) -> usize`, that performs the content-or-input choice, the lazy output read-back, and the honesty assertion; `measure` derives `exp_denom_bytes` from it and `export` returns it; lift the repeated `.expect` into one private `fn cell(&self) -> Cell`. Acceptance: exactly one `match` over `Denom` computes a denominator in the board module; for a `Denom::Input` cell, `denominator_bytes` does not invoke the body (a counting body in a unit test); the bench sidecar's denominator file is byte-identical before and after at the record scales.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-frame-9 (low, documentation): roster: approved (ruling 104)

`MAX_HEAP_BYTES_PER_INPUT_BYTE` carries no derivation

- Owner-gated: no

Resolution: State the derivation: the calibrating reader at the release profile and the margin convention, or that the constant operationalizes the crate-level "small constant multiple" promise at a stated multiple, with the reading left to the pin commit. Acceptance: the constant's doc names its calibrating reader or its derivation from the crate-level promise.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-frame-13 (nit, simplification): roster: approved (ruling 104)

`both_present_nodes` is an operand-content walk living in the constants module

Where: `crates/before/src/meter/board/ceilings.rs:286-298`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Move `both_present_nodes` to operand.rs

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-frame-14 (nit, documentation): roster: approved (ruling 104)

"The tripwire pair below" points at tests that live in another file

Where: `crates/before/src/meter/board/cell.rs:45-48`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Name the two tests in backticks

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-frame-16 (nit, simplification): roster: approved (ruling 104)

`Cell`'s three constructors repeat the same struct literal, and its two mutually exclusive heap models are flat fields whose precedence lives in the judge

Where: `crates/before/src/meter/board/cell.rs:204-308`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One private `with_denom` constructor; consider a `HeapModel` enum

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-frame-19 (nit, verification): roster: approved (ruling 104)

The tiling test derives the board's operation axis by building every family at a bare `0.02`, twice, instead of from `ops()`, and guards NA reasons with `reason.len() >= 20`.

Where: `crates/before/src/meter/board/coverage/tests.rs:9-16`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Derive the axis from `ops()`; name or drop the length guard.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-frame-20 (nit, simplification): roster: approved (ruling 104)

Em-dashes in two `//` comments and one assert message

Where: `crates/before/src/meter/board/coverage/tests.rs:78-79`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Decide crate-wide, then `; ` and ` -- `

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

