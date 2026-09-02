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

- **Base.** Your worktree's HEAD must equal `bba0e31a` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `bba0e31a`, fast-forward; if it has diverged, stop and report. Never call
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

## Ordering inside the lane

1. Unify the harness with the current pin sets preserved exactly (the
   unified struct carries every column; rows that lack a pin today carry
   a placeholder the harness prints and does not yet assert). Run the
   suite: every MEASURED line byte-identical to the parent's. This is the
   behavior-preserving commit; capture the parent's MEASURED lines to
   `<scratchpad>/p1-harness/measured-parent.log` before you start and
   diff against them.
2. Delete the segments column (ruling 38): the `segments` field, every
   `segments:` pin, the segments fragment of the MEASURED line, and the
   file-doc prose that calls the zero a measurement, naming
   `clock::tests::deep_tree_stack_safety` as the no-recursion proof of
   record in its place. Every other fragment of every MEASURED line stays
   byte-identical to step 1's. The readers `meter::stack_segments` and
   `meter::reset_stack_segments` still exist at your base (the board lane
   deletes them after you land); simply stop calling them.
3. Widen to every column pinned on every row: measure each unpinned
   cell at the parent (the same log), set its ceiling by the file's
   stated pin convention, its floor by the file's stated floor rule, and
   name every new pin and its measured value in the commit message. This
   is the one-time re-measure ruling 4 sanctions. With step 2 done there
   is no segments cell to pin.
4. Scan floors on every nonzero row (envelopes-a-6, ruling 50), with the
   stubbed-validator construction committed as the known-bad.
5. Fold `skyline_render_records_zero_touches` into the `SKYLINE_RENDER_*`
   rows as a touch column pinned to 0; move the tick scenarios beside
   their table; delete the pointer comment at 265-267.
6. Retire the twin rows per envelopes-a-8 and envelopes-a-11 under the
   public-entry roster.
7. The `compile_error!` without the meter features (meter-adequacy-11,
   ruling 51), with the one justfile recipe change it needs.

Land nothing outside `tests/meter.rs` in this lane except the `just
test` recipe line step 7 needs. Other lanes hold their `meter.rs`
commits until you land (README, launch order).

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

### envelopes-a-2 (medium, verification-gap): ruling 38

Resolution: Present to Finch as a premise change on the 2026-07-24 keep, not a fresh dissolution argument. Two consistent end states: (a) dissolve the `segments` field from `Envelope`, `TouchEnvelope`, `SweepEnvelope`, `QueryEnvelope`, their constructors, tables, and harness asserts, and restate lines 13-26 and 60-65: library traversals are iterative by construction (AGENTS.md's rule), the committed proof is `clock::tests::deep_tree_stack_safety`, and a recursion regression fails the deep scenarios here by overflow; or (b) if a live meter in this binary is wanted, compile `grow`/`descend!` under `any(test, feature = "meter")`, make `stacker` optional behind `meter`, and add a canary beside `heap_meter_registers_known_allocation` that dives through a guarded recursion and asserts `stack_segments() >= 1`, while stating at line 22 that no library kernel can reach the counter, so the zero pin is a rule pin rather than a cost. Under either, correct `recurse.rs:17-20`'s "measured fact" to name the binaries in which the reading is measured (the lib's cfg(test) suite). Acceptance: either no `segments` field or column remains in `tests/meter.rs` and the doc names the depth test as the proof, or a committed canary in this binary fails when the `SEGMENTS_GROWN.fetch_add` line is deleted.

Ruled (38): end state (a), the dissolution. This lane owns the
`tests/meter.rs` half in full: the field, the constructors, the tables,
the harness asserts, and the file-doc restatement at 13-26 and 60-65
(library traversals are iterative by construction; the committed proof is
`clock::tests::deep_tree_stack_safety`; a recursion regression fails the
deep scenarios by overflow). The `recurse.rs:17-20` correction is the
board lane's (crate-root-32). No canary is added.

### crate-root-32 (medium, verification-gap): ruling 38, this lane's half

Resolution: The owner's call between two sound options. (a) Dissolve, my recommendation: remove the segments currency from the board (`Currency::Segments`, `seg_ceiling_only()` on every cell, `MAX_GROWN_STACK_SEGMENTS`, `SEG_FLOOR_TRIP`, the render column) and `Envelope.segments` with every `segments:` pin in tests/meter.rs, and `meter::{stack_segments, reset_stack_segments}`; confine `SEGMENTS_GROWN` and its readers to `cfg(test)` beside their one live client, the determinism dive; name `clock::tests::deep_tree_stack_safety` (depth 100k) plus the structural fact that `descend!` is `cfg(test)` and `stacker` a dev-dependency as the instruments against reintroduced depth recursion. (b) Keep and make it live: `stacker` becomes an optional dependency enabled by `meter`, `grow`/`descend!` compile under `any(test, feature = "meter")`, and a guarded 200k-deep descent in tests/meter.rs and in the board's self-check reads `stack_segments() > 0` in each enforcing binary before the kernels' zero is asserted. Either way, restate recurse.rs:16-20 and 74-76 as what IS (the guard and its counter exist for the test-only oracle bridge and its witnesses; in non-test meter builds the counter has no writer), fix line 37, and excise tests/meter.rs:441's clause. Acceptance: (a) `grep -rn 'stack_segments\|SEGMENTS_GROWN\|Currency::Segments' crates/before` finds only the `cfg(test)` determinism witness, and `just gate` is green; (b) a test in crates/before/tests/ built with `--features meter` asserts `before::meter::stack_segments() > 0` after a guarded deep descent. In both, no prose calls the segments zero a measured fact.
Construction: Read-only: in a build of the library with `--features meter,limb-meter,scan-meter` and without `cfg(test)` (what tests/meter.rs and examples/amp_board.rs link), no expression writes `SEGMENTS_GROWN`; the sole `fetch_add` is inside `#[cfg(test)] fn grow`. Runtime: add a temporary scenario to tests/meter.rs whose body recurses 10^6 frames through `stacker::grow` directly; `meter::stack_segments()` still reads 0 and every `segments: 0` envelope passes. Conversely, delete the body of `stack_segment_meter_counts_deterministically_and_resets` and run the meter suite and `just amp-board-acceptance`: every segments ceiling stays green, because nothing in those binaries could have moved the counter before the change either.

Ruled (38): option (a). This lane lands only the `tests/meter.rs`
clauses of that option: `Envelope.segments` and every `segments:` pin go,
and `tests/meter.rs:441`'s clause is excised. The board currency, the
`meter::{stack_segments, reset_stack_segments}` readers, the `cfg(test)`
confinement of `SEGMENTS_GROWN`, and the `recurse.rs` restatement are the
board lane's; the acceptance grep is checked by the coordinator once both
lanes have landed.

### envelopes-a-6 (medium, verification-gap): ruling 50

Resolution: Rewrite the `DECODE_DENSE`, `CMP_DENSE`, and `JOIN_DENSE` row comments to state the liveness signal those rows actually have (today: none beyond the heap ceiling; see envelopes-a-9), or move them onto the five-column shape so they gain the scan column. Add a scan floor (the ×0.75 tripwire, or a derived one bit per live input bit where the walk provably reads its whole input, as `id_walk_scan_cost` does) to the sweep and query envelopes under `scan-meter` and pin it for every nonzero row; the docketed unification is the natural vehicle. Acceptance: stubbing `skyline::validate_bits` to `Ok(())` fails at least one `SKYLINE_VALIDATE_*` row; no row comment names a floor its table lacks.

Ruled (50): the scan floors land on every nonzero row inside the
unification (step 4), so the first option's "move them onto the
five-column shape" is what step 3 already does and the second half (a
scan floor for every nonzero row under `scan-meter`) is the work. Derive
the floor where the walk provably reads its whole input (as
`id_walk_scan_cost` does) and use the tripwire fraction elsewhere; state
which at the row. The `DECODE_DENSE`, `CMP_DENSE`, and `JOIN_DENSE`
comments state the liveness signal their rows now carry. The negative
control is the entry's construction: stub `skyline::validate_bits` to
`Ok(())` as a reversible mutation, record which `SKYLINE_VALIDATE_*` rows
go red in the commit message, restore.

### meter-adequacy-11 (low, verification-gap): ruling 51

Resolution: either a `compile_error!` under
`not(all(feature = "limb-meter", feature = "scan-meter"))` at the top of
tests/meter.rs (with `just test` passing the features for `-p before`), or
an explicit `limb_ops=off scan_bits=off touches=off` marker on every
MEASURED line so absence cannot read as zero. Acceptance: a
default-features run either fails to build the meter binary or prints the
marker on every MEASURED line.

Ruled (51): the `compile_error!`. `just test`'s `-p before` invocation
passes `limb-meter` and `scan-meter` so the inner loop still builds the
binary; that recipe line is the one edit outside `tests/meter.rs` this
lane makes, and its recipe comment says why. Acceptance: a
default-features `cargo nextest run -p before --test meter --no-run`
fails at the `compile_error!` with a message naming the two features;
`just test` and `just gate` build it.

## Hazards and stops

- **Segments.** Ruling 38 dissolves the currency. Step 2 deletes the
  column from this file; nothing in this lane re-derives, pins, or
  carries a segments cell. The readers in `before::meter` and the board's
  currency are the board lane's to delete, after you land; if your gate
  run fails because a reader has already gone (the board lane landed
  first), rebase and drop the calls, nothing more.
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
