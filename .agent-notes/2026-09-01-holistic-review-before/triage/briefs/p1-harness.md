<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P1 lane: the envelope harness unification

## Goal

`crates/before/tests/meter.rs` runs one measurement procedure (reset the
process-global meters, run the scenario, read, print the MEASURED line,
assert ceilings and floors) through four envelope structs, four `const fn`
constructors, and four harness bodies that differ only in which columns
exist. The split manufactures blind spots: a row can pin only the columns
its struct happens to carry, so door rows lack the scan column their
kernel twins have, a separate test exists because one struct has no touch
column, the tick scenarios moved tables to find a column, and the tripwire
message and pin convention are copied ten and five times with drift.
Rulings 1 and 2 call for new breaching-shape envelope rows in P2, and
Finch ruled (ruling 4) that the harness unifies first so those rows land
once. The invariant restored: one envelope type carrying every column,
pinned on every row; one harness; one statement of the pin convention;
every pre-existing reading byte-identical before and after; every newly
pinned cell measured at the parent and attributed in the commit.

## Ground rules

These apply to every P1 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `0fc1921e` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `0fc1921e`, fast-forward; if it has diverged, stop and report. Never call
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

## Ordering inside the lane

1. Unify the harness with the current pin sets preserved exactly (the
   unified struct carries every column; rows that lack a pin today carry
   a placeholder the harness prints and does not yet assert). Run the
   suite: every MEASURED line byte-identical to the parent's. This is the
   behavior-preserving commit; capture the parent's MEASURED lines to
   `<scratchpad>/p1-harness/measured-parent.log` before you start and
   diff against them.
2. Widen to every column pinned on every row: measure each unpinned
   cell at the parent (the same log), set its ceiling by the file's
   stated pin convention, its floor by the file's stated floor rule, and
   name every new pin and its measured value in the commit message. This
   is the one-time re-measure ruling 4 sanctions. Segments cells are
   excluded from this step (see hazards).
3. Fold `skyline_render_records_zero_touches` into the `SKYLINE_RENDER_*`
   rows as a touch column pinned to 0; move the tick scenarios beside
   their table; delete the pointer comment at 265-267.
4. Retire the twin rows per envelopes-a-8 and envelopes-a-11 under the
   public-entry roster.

Land nothing outside `tests/meter.rs` in this lane. Other lanes hold
their `meter.rs` commits until you land (README, launch order).

## Members

### envelopes-a-4 (medium, simplification): ruling 4, amended

Resolution: This is the docketed unification; the note records it as a deferred disposition, so treat this entry as a reminder with the range's evidence attached. One `Envelope` with `peak_heap` and, per optional column (limb, scan, touch, densify), an `Option<Pin { ceiling, floor }>` where `None` means not watched; one `const fn`; one `metered` that walks a static column table and composes the MEASURED line from fragments as `sweep_metered` already does; the pin convention stated once in the file doc and cited by each table in a sentence. Fold `skyline_render_records_zero_touches` into the `SKYLINE_RENDER_*` rows as a touch column pinned to 0, move the tick scenarios (465-713, 2043-2122) beside the table they use and delete 265-267. Acceptance: one harness function and one envelope type serve every table; every existing row's numbers are unchanged; the tripwire message appears once; the pin convention appears once.

Amendment (ruling 4): no `Option<Pin>`; every column is pinned on every
row (envelopes-b-8's second shape). The placeholder in step 1 above is a
transitional state inside this lane, gone by the end of step 2. The rest
of the resolution stands: one `const fn`, one `metered` over a static
column table, the pin convention once, the render test folded, the tick
scenarios moved, 265-267 deleted. "Every existing row's numbers are
unchanged" means every pin that exists at the parent keeps its value;
new pins are additions attributed in the commit.

### envelopes-b-8 (medium, modularity): ruling 4

Resolution: collapse to one envelope struct carrying every column (heap, segments, limb ops, scan bits, touches, limb floor, touch floor; `before::meter::board::ByCurrency<Bound>` is the ready-made shape) and one `metered` that stores all columns unconditionally (they are constants) and cfg-gates only the reads and asserts. Two shapes are possible and the owner should pick: `Option<u64>` for unpinned columns (rows keep their current pin sets; the harness prints but does not assert `None`), or every column pinned on every row (wider coverage, but a one-time re-measure of the three-column tables). Acceptance: one struct and one `metered` in the file; the row tables unchanged in shape; `grep -c 'tripwire (measured x0.75)' crates/before/tests/meter.rs` drops from ten to one harness copy plus the band bodies; readings byte-identical before and after.

Ruled (4): the second shape, every column pinned on every row. Whether
`ByCurrency<Bound>` is the container is your call; the column set is the
entry's. "Readings byte-identical before and after" is the step-1
acceptance; step 2 adds pins without moving readings.

### envelopes-a-8 (low, scaffolding): ruling 4

Resolution: Decide which side is the roster of record. If the door: give the door rows the scan column (falls out of envelopes-a-4) and the value legs (envelopes-a-9), port the kernel-only shapes (`SKYLINE_CMP_DENSE_SELF`, `SKYLINE_CMP_WIDE_TOOTH`, `SKYLINE_JOIN_ABSORB`, `SKYLINE_JOIN_WIDE_TOOTH`, `SKYLINE_MEET_*`) to `partial_cmp`, `|`, `&`, and retire the five twins with their tests. If the kernel: state at each retained door row the door cost it exists to catch. Acceptance: no two rows pin the same (shape, operation) through a door and its kernel; every retained kernel row's comment names the door cost it deliberately excludes.

Ruled (4): the door (public-entry) rows are the roster of record. Give
them the scan column (step 2 does this for every row) and the value legs
the kernel twins carry, port the kernel-only shapes to the public
operations, and retire the five twins with their tests. The ported rows'
new pins are measured at the parent through the public entry and named
in the commit; a ported row whose public-entry reading differs from its
kernel twin's is expected (the door adds work) and is attributed, not a
stop. envelopes-a-9 (the value legs) is a P6 entry; take only what this
resolution names: the value or byte-identity leg each retiring twin
carries moves onto its door row.

### envelopes-a-11 (low, vestigial): ruling 4

Resolution: Dissolve the five `SKYLINE_DECODE_*` rows and their tests (1500-1576), keeping the round-trip equality as a plain unit test if `skyline::decode` stays for the tests' vocabulary; if WideTooth and AltSpine matter at the decode door, add `DECODE_WIDE_TOOTH` and `DECODE_ALT_SPINE` through `Version::decode`. Acceptance: no envelope row measures `meter::skyline::decode`; the public `DECODE_*` rows cover every shape the owner wants pinned at decode.

Ruled (4): under the public-entry roster, dissolve the five rows; add
`DECODE_WIDE_TOOTH` and `DECODE_ALT_SPINE` through `Version::decode` so
no shape loses its decode pin (measured at the parent through the public
door, attributed in the commit); keep the round-trip equality as a plain
unit test if `skyline::decode` remains in the tests' vocabulary.

## Hazards and stops

- **Segments.** The `segments` column's only writer is `#[cfg(test)] fn
  grow`, and its disposition (decision 42: dissolve the column or give it
  a writer) is ruled in S4. Carry the column through the unified struct
  exactly as the four structs carry it today; re-derive no segments cell;
  add no segments pin to a row that lacks one. If "every column pinned
  on every row" cannot be satisfied without pinning a segments cell that
  is unpinned today, stop on that row's segments cell (leave the
  placeholder, say so) and land everything else.
- **Heap pins.** Every row already carries a heap pin; this lane moves
  none. The 480 B pin at `MASKED_CMP_HOLE` is under ruling 24's
  reproduction in the gate lane and never widens. A heap reading that
  differs from the parent after step 1 is a bug in the unification, not a
  re-pin.
- **Board rows and bands.** The band modules (`hoisted_window`,
  `fold_stagger`, the flatness bands with their own `run` helpers) are
  not the four harnesses; leave them for the suites lane. The
  `tripwire (measured x0.75)` count drops to one harness copy plus the
  band bodies, as the acceptance says.
- The file's isolation note and the reset sequence are being given a
  `require_process_isolation()` call by the suites lane (ruling 19). Do
  not add it here; one harness makes their edit a one-liner.
- A row whose public-entry re-measure reads over the kernel twin's
  ceiling by more than the door's expected extra work is a finding to
  report with both numbers, not a pin to set.
