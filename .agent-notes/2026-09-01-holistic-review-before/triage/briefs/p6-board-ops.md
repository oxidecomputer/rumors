<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P6 lane, board 3 of 6: ops, render, shards, worst, the board tests

## Goal

The board's ops and render entries, landed per their Resolutions inside an approved roster, under rulings 12 (certifying probes), 13, 38, 48 (`mechanism()` dissolved), 43, and 88.

## Roster summary

1 ruled (1 medium); 0 medium ruled (93 to 103); 12 roster members approved (ruling 104) (8 low, 4 nit).

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
- **Prose (ruling 105).** Every paragraph you touch passes the three tests in
  `PROSE.md` (altitude, concision, legibility); the reviewer applies its
  checks; the diff is net shorter in prose unless your report says what the
  additions buy.
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

Every P6 lane runs after the P1 to P5 and P7 lanes that touch its files have landed on main, or rebases onto them before its final gate run; the coordinator names the base SHA at launch. Lows and nits inside this lane's approved roster are swept without a question to Finch; every high and medium has an individual ruling. A change that would alter a rendered `before` doc panel is a stop (ruling 89). Follows `p1-board`, `p2-rows` (the shape-walk rows), `p4-ghosts`. Owns `src/meter/board/{ops,render,shard,worst}.rs` and `board/tests.rs`.

## Members

### board-ops-render-26 (medium, verification): ruling 93

The delegating-parser pinned floors' separation from the bypass reading is prose, not a per-run check, and the table is a measured band labelled a liveness floor

- Owner-gated: no

Resolution: Make the separation a per-run check inside `delegating_parser_stays_under_the_text_limb_ceiling`: compute the radix site's contribution outside measurement (the hypothesis is `stored_bases(&v).iter().map(|b| b.bits().div_ceil(64).max(1)).sum::<u64>()`, since `parse_decimal` records exactly that per spelled value; measure it rather than transcribe it), and assert `ops - radix < pinned_floor && pinned_floor <= ops`, so the floor is proven to sit strictly between the bypass reading and the live reading on every run. Delete the "roughly half" clause. Re-label the constant's doc to the genre it is (a measured tripwire with a per-run separation witness), or, structurally, give the radix delegation its own counter site and floor it at `mandatory_limbs_version(&v)`, which dissolves the table. Acceptance: setting a pinned floor to `radix - 1` fails the separation leg; the doc claims no separation it does not check and no reading it does not measure.

Amendment (ruling 93): take the structural branch: give the radix delegation its own counter site floored from the derived mandatory limb count, dissolving the measured pinned-floor table and its quoted readings (ruling 88: a measured floor is not a floor). The per-run separation check is struck. See ../rulings.md.

## Roster members approved (ruling 104)

Lows and nits approved as this lane's roster by ruling 104. Land each per its quoted Resolution and Acceptance, swept with the ruled members; report rather than choose if a Resolution conflicts with a ruling or offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89).

### board-ops-render-1 (low, documentation): roster: approved (ruling 104)

The row table's module doc states an absolute the declared-model attachments break

- Owner-gated: no

Resolution: Name the exception where the rule is stated: "...never from the shape's identity, except to attach an owner-declared per-cell judgment model (the `ceilings` module's declared-models section), which by construction is a statement about one named cell and never about reach." Apply the same amendment to board.rs:25-29. Relocating the declarations onto the bundle (a `declared` slot the family builder fills, as `output_dominated` is) is an optional design proposal; the declared constants differ per operation (the tick trio versus `version_min_ticks`), so a per-family slot is not obviously cleaner. Acceptance: the two docs and the code agree; a reader of ops.rs:3-5 is told where identity is consulted and why.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-ops-render-14 (low, simplification): roster: approved (ruling 104)

render.rs houses the measurement seam beside the renderer, and its "one scale guard" has two verbatim copies

- Owner-gated: no

Resolution: Call `assert_scale` from `merge_samples` and `bench_cells`; move `assert_scale`, `build_pair`, and `measure_cell` into measure.rs beside `measure` (shard.rs already imports from both modules), leaving render.rs with `Summary`, `row`, and `render_results`, and rewrite its first sentence to name the one responsibility. Acceptance: one `scale > 0.0 && scale.is_finite()` in the board; render.rs imports nothing from `measure`, `ops`, or `family`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-ops-render-16 (low, verification): roster: approved (ruling 104)

Six of the seven shard-merge refusals have no committed known-bad demonstration, and the round-trip test's doc claims coverage of guards it only exercises on the accept path

- Owner-gated: no

Resolution: Extend the smoke suite's tamper test into a table over the untampered capture: flip one hex digit of the header's scale bits (stamp mismatch); rename one cell's op or family (unknown operation/family); in a two- or three-shard deal move one cell line into another shard's capture and restate both counts (outside its slice); duplicate a cell line and bump the end count (duplicate); drop the end line (truncated); leave the count unchanged after dropping a line (count disagreement); append a field (trailing fields). Assert each is refused with the documented message fragment. Re-word `shard_protocol_round_trips`'s doc to claim the accept path only. Acceptance: one test per refusal in the module doc's list, each failing if its assert is deleted.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-ops-render-20 (low, simplification): roster: approved (ruling 104)

Literals shadow named constants: `1.0` for `DEFAULT_SCALE`, `/ 64` for `OVERLAP_FOLD_INPUT_DIVISOR`, `0.02` for the smoke scale, and the seed and empty-version packed sizes as arithmetic

- Owner-gated: no

Resolution: `("default", DEFAULT_SCALE)` with the import, and one label pair for both renderers; `(a_bytes.len() / OVERLAP_FOLD_INPUT_DIVISOR).max(MIN_SIZE_PARAM)` in the test; a lib-side `PROBE_SCALE` constant for the `0.02` sites; `SEED_PARTY_BYTES`/`EMPTY_VERSION_BYTES` constants in family.rs (or compute `Party::seed().encode().len()` once) used by both files; name the `% 7` modulus. Acceptance: no `"default", 1.0` in worst.rs; no `/ 64` in tests.rs; no bare `n + 1`/`n + 2` denominators in ops.rs.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-ops-render-22 (low, verification): roster: approved (ruling 104)

The two verdict-of-record entry points have no committed wrong-artifact demonstration

- Owner-gated: no

Resolution: In tests.rs, (a) feed `check_with` a one-operation synthetic sweep (built from `Sample`s through `evaluate`) whose argmax disagrees with `WORST_RANKINGS` on one currency and assert `Ok(false)` plus a drift line naming op, currency, and both worsts; and a sweep missing a pinned op, asserting the stale-entry line. (b) For `run_acceptance`, restamp an untampered smoke capture to the two ladder scales, raise one cell's top-window heap reading over its ceiling, and assert `Summary.red == 1` counted once. Acceptance: both tests fail under a mutated `check_with` that never clears `clean` and under a `run_acceptance` that unions only `lo_cells`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-ops-render-24 (low, documentation): roster: approved (ruling 104)

tests.rs prose carries a partial hand-maintained inventory and two counts its bodies contradict

- Owner-gated: no

Resolution: Rewrite the header as the file's structure, not a tally: derivation pins (radix units, mandatory limbs, output honesty), known-bad probes through `evaluate`/`evaluate_acceptance` that must read red on exactly one leg, pins that tie tables to live axes (bench riders, the ranking pin), and the argmax kernel and near-tie rendering tests. Rewrite 860 to name the probes without a numeral; rewrite 104 as "Four representative shapes render under the ceiling", pointing at the per-cell assertion. Acceptance: no numeral count of probes in a test doc; the header names genres, not tests; adding a probe test requires no header edit.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-ops-render-4 (low, simplification): roster: approved (ruling 104)

The operation table is a two-thousand-line function under a clippy allow, with its row templates repeated verbatim

- Owner-gated: no

Resolution: Two independent levers, either or both. (a) Make the table data: `pub(super) static OPS: &[Op] = &[ .. ]` (or split into `version_rows()`, `party_rows()`, `clock_rows()`, `rejection_rows()` at the existing section headers), removing the allow and the per-call construction. (b) Add row-builder helpers beside the table: a generic `decode_rejection(fed, floors, decode: fn(&[u8]) -> Result<R, Decode>, expected: fn(&Decode) -> bool, defect_name)` (Version, Span, Party, and Clock all decode `&[u8]` to `Result<_, Decode>`), a `placement_row(span, probe, method)`, a `hash_row(value)`, and a `declared_for(kind, cell)` step. Weigh the greppability trade the owner prefers. Acceptance: no `too_many_lines` allow in ops.rs; the `matches!(err, Decode::..)` assertion appears once per defect kind; the smoke suite's per-family cell counts and the rendered row names are unchanged.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-ops-render-7 (low, simplification): roster: approved (ruling 104)

Rows reach around `party_pair()` for one side's length or bytes at thirteen sites

- Owner-gated: no

Resolution: Add per-side accessors on `FamilyData` (`party_a(&self) -> Option<(Party, usize)>`, `party_b`, and `party_a_bytes(&self) -> Option<&[u8]>` for the rejection rows), or have `party_pair` return per-side lengths; use them at every site, including `clock()`. Acceptance: `grep -c 'parties.as_ref().map' ops.rs family.rs` is zero outside the accessors.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-ops-render-11 (nit, documentation): roster: approved (ruling 104)

`Summary.red`'s doc calls every red an amplification finding

Where: `crates/before/src/meter/board/render.rs:26-29`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "Cells with at least one red leg: an exponent or constant over its bound, a reading outside a declared model's band ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-ops-render-13 (nit, simplification): roster: approved (ruling 104)

render.rs idiom nits: guard-then-`expect` match arms, an inline `std::collections::` path, and an em-dash in one printed legend line

Where: `crates/before/src/meter/board/render.rs:118-126`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `if let` chains; import `BTreeSet`; replace the em-dash

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-ops-render-21 (nit, simplification): roster: approved (ruling 104)

Hand-rolled run grouping in `worst::fold` where `slice::chunk_by` expresses it

Where: `crates/before/src/meter/board/worst.rs:176-183`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `results.chunk_by(\

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-ops-render-25 (nit, verification): roster: approved (ruling 104)

An assertion message at board/tests.rs:73 carries a run of ten spaces mid-sentence.

Where: `crates/before/src/meter/board/tests.rs:73-73`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Collapse to one space.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

