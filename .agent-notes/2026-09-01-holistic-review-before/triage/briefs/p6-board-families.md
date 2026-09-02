<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P6 lane, board 2 of 6: families, floors, judge, measure, operand

## Goal

The board's family, floor, and judge entries, landed per their Resolutions inside an approved roster, under rulings 11 and 73 (the residual fit and the affine leg), 13, 61 (compiler-held rosters), 72 (`FloorKind`, the operand walk), 43, and 88.

## Roster summary

2 ruled (2 medium); 0 medium awaiting ruling; 9 pending roster approval (6 low, 3 nit).

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

Every P6 lane runs after the P1 to P5 and P7 lanes that touch its files have landed on main, or rebases onto them before its final gate run; the coordinator names the base SHA at launch. Lows and nits inside this lane's approved roster are swept without a question to Finch; every high and medium has, or awaits, an individual ruling. A change that would alter a rendered `before` doc panel is a stop (ruling 89). Follows `p1-board`, `p4-rosters`, `p4-structure` (operand.rs). Owns `src/meter/board/{family,floors,judge,measure,operand}.rs` and their tests.

## Members

### board-families-floors-judge-10 (medium, verification): ruling 93

Not-applicable declarations whose rendered reasons the code contradicts, leaving `clock_fork` unwatched on every version-only family

- Owner-gated: no

Resolution: (a) Floor the decode rows' touch at the max of `touch_delta_fold(stored_nonzero_deltas(v))` and the wide-stream limb count, and reword lines 60-64, NA_TOUCH_LAZY_BATCH, and the clause inside WHY_TOUCH_WIDE_STREAM to the meter's actual accounting (a zero delta is skipped; a nonzero word-scale delta costs one register or digit touch). (b) Replace NA_SCAN_SEED_PARTY with `scan_touch()` at ops.rs:1329-1333 and 1616-1620 and delete the constant. (c) Give `clock_fork` the party half's WHY_HEAP_FORK_HALF floor as `party_fork` does (the child's packed bytes, probed at prepare), or move both rows to one genre with the reason stated. Acceptance: with the counter features, a probe that decodes `dense(1_000)`'s bytes with the touch counter reset reads `touches() >= stored_nonzero_deltas(&v)`, and a `Sample` with `touch: Some(0)` under the new floor reads TOUCH_FLOOR_TRIP through `evaluate`; `Party::seed().fork()` with the scan counter reset reads `scan_bits() >= 2`; the `clock_fork` cells on version-only families render at least one `flr[..]` entry; `just amp-board-acceptance` stays green (any new red is a finding to triage). Construction: (a) In board/tests.rs under `limb-meter`: `let v = version_of(&dense(1_000)); suanpan::touch_meter::reset(); let _ = Version::decode(&v.encode()); assert!(suanpan::touch_meter::touches() >= stored_nonzero_deltas(&v));` passes today by the reading above, while `touch_wide_stream(&v)` returns `NotApplicable`: a validator moved onto an unmetered `i128` height would read 0 touches and stay green on every all-narrow family. (b) Under `scan-meter`: `crate::meter::reset_scan_bits(); let mut p = Party::seed(); let _ = p.fork(); assert!(crate::meter::scan_bits() >= 2);` while the cell declares "its packed form is empty". (c) Reset the peak allocator, `Clock::from_parts(Party::seed(), v).fork()`, observe peak >= the child party's packed byte: the allocation `party_fork` floors and `clock_fork` declares unforced.

Ruled (93): lands per the quoted Resolution under that ruling; see ../rulings.md.

### board-families-floors-judge-20 (medium, verification): ruling 93

Exponent legs can go unjudged with nothing pinning which cells may

- Owner-gated: no

Resolution: Commit a tamper-evident roster of the cells whose exponent legs are expected unjudged (today: the rank rows on the benign family, heap legs inside the flat allowance, capacity-model cells), in the style of the worst-rankings roster, and have `run_acceptance` (or the shard merge) refuse any compiled currency whose `Fit.judged` is false on a cell outside it; add the known-bad artifact beside `merge_refuses_a_silently_shrunk_grid_for_every_family`: a capture with one cell's second-sample `exp_denom` overwritten to equal the first's must be refused. Acceptance: the tampered-capture test fails against today's code (the board renders GREEN with ` -.--` on the tampered cell) and passes once the roster check lands; the smoke and acceptance boards match the committed roster exactly. Construction: In tests/amp_board_smoke.rs, reuse the capture-and-edit loop at 229-302: split one cell line on tabs, set the second sample's `exp_denom` field (the second field of the second sample block, per `emit_sample`'s order at shard.rs:205-241) equal to the first sample's, keep the end count, and call `board::run` on the tampered capture. Today it succeeds and the rendered row shows ` -.--` on that cell's limb/scan/touch exponents with a GREEN verdict.

Ruled (93): lands per the quoted Resolution under that ruling; see ../rulings.md.

## Roster members pending Finch's approval

Lows and nits no ruling has reached, placed here by the files they touch. Land only after the coordinator confirms the roster is approved.

### board-families-floors-judge-13 (low, simplification): roster: pending Finch's approval

floors.rs re-inlines its own helpers: five scan-floor casts, twin limb constructors, eight near-identical `Floors` literals, six zero-or-NA shapes, two rate types for one dimension

- Owner-gated: no (making the `pub` rate constant a `u64` is the one owner-call inside it)

Resolution: `fn scan_floor(bytes: usize, why: &'static str) -> Liveness` routing `scan_examines` and the three rejection constructors and `sync_floors`; `fn floor_or_na(min: u64, why, na_reason) -> Liveness` for the six zero-or-NA sites, with `limb_stream`/`limb_wide` collapsing into it; `fn in_place(scan: Liveness, touch: Liveness) -> Floors`, `fn equal_pair() -> Floors`, and `fn witness(touch_na: &'static str) -> Floors` for the literals; make `SCAN_FLOOR_BITS_PER_INPUT_BYTE` a `u64` multiplied with `saturating_mul` as the tick floor does (owner's call: the constant is `pub`); `usize::try_from` at operand.rs:124. Acceptance: one occurrence of `SCAN_FLOOR_BITS_PER_INPUT_BYTE` in an expression; `grep -c 'Liveness::NotApplicable {' floors.rs` is 1 (inside `na`); no site constructs `Liveness::Floor` with a possibly-zero `min` directly; the smoke board's rendered legend is byte-identical before and after.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-families-floors-judge-18 (low, documentation): roster: pending Finch's approval

`trend`'s docstring claims a lumpy counter "errs red, never green"; a counter dark at the larger point reads green

- Owner-gated: no

Resolution: State the direction-dependence: a zero at a smaller point steepens the fit (a conservative false red); a zero at a larger point flattens it and reads green, which is why every judged column carries a liveness declaration. Optionally pin `trend(&[(100, 5), (200, 0)]) < 0.0` in a one-line judge test. Acceptance: the docstring states the direction-dependence. Construction: `assert!(trend(&[(100, 5), (200, 0)]) < 0.0)`; through `evaluate` with all-NA floors and `limb: Some(60)` then `Some(0)` over denominators 100 -> 200, `red` is empty.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-families-floors-judge-22 (low, documentation): roster: pending Finch's approval

Constants are judged at each window's larger size only; board.rs says "per size across the ladder"

- Owner-gated: no for the doc fix; yes if both sizes are to be judged

Resolution: Correct board.rs:222-224 to "constants at each window's larger size; bands and floors at every size" and state the reason the smaller size is excluded; or (owner's call) judge `per_unit` at both samples of each window. Acceptance: board.rs and judge.rs agree on which sizes carry the constant legs. Construction: Through `evaluate`, `sample(n, limb = 10*128*n)` (over the 128/B ceiling) paired with `sample(2n, limb = 2n)` reads no "limb constant" red today, since only `s2` is judged.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-families-floors-judge-23 (low, simplification): roster: pending Finch's approval

`judge_window`'s ceiling resolution is a match split across four statements; `Score` duplicates `Fit`; the span test is written twice

- Owner-gated: no

Resolution: Fold the overrides into the match arms (the capacity-model `continue` can precede the match); `Score { fit: Fit, per_unit: Option<f64> }`; hoist `fn denominators_span(first: usize, last: usize) -> bool` to module level; `== Some(true)` and an `over_ceiling`/`under_floor` pair in place of `banded`; `fn floor_trip(c: Currency) -> &'static str` (tests.rs:409 imports `SCAN_FLOOR_TRIP`; the test can call the fn). Trim `trend`'s doc to the estimator's mechanics with a pointer to board.rs's exponent-policy section only if the owner wants the policy stated once. Acceptance: `judge_window` reads each currency's ceiling from a single match arm; one span helper; `acceptance_trend_absorbs_lumps_and_keeps_amplifiers_red`, `exponent_guards_skip_noise_and_keep_real_amplifiers_red`, `declared_capacity_model_bands_the_projection_peak`, and the smoke board pass unchanged.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-families-floors-judge-25 (low, verification): roster: pending Finch's approval

The touch floor's sole basis and the flat-denominator axis have no differential pin against the public shape iterator

- Owner-gated: no

Resolution: A proptest over the crate's version generators (and a sweep over `study_family_versions`) asserting `stored_nonzero_deltas(v) == v.shape().skip(1).filter(|p| p.rise.is_some()).count()` and that `value_content_bytes(v)` equals the byte-rounded sum over `shape()` of `bits(height).max(1)` with heights accumulated from the rises; optionally implement `stored_nonzero_deltas` through `shape()` and dissolve one decoder (finding 24). Acceptance: the committed differential test passes; flipping the `!bits.bit(pos)` polarity or the odd/even zigzag arm in either walk fails it. Construction: proptest over `crate::testing::generators`' `Version` strategy: compute both counts and assert equality; the identity follows from shape.rs:89-95.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-families-floors-judge-7 (low, simplification): roster: pending Finch's approval

`scatter` and `weave` hand-roll the balanced fork expansion `Party::forks` provides

- Owner-gated: no

Resolution: One `pub(crate) fn balanced_leaves(n: usize) -> Vec<Party>` in `meter` (or `[Party::seed()].into_iter().chain(first.forks(n as u64 - 1))` directly), used by family.rs, meter.rs, and the tests. Acceptance: one balanced-expansion implementation in the instrument code; `scatter` and `weave` produce byte-identical bundles at power-of-two `n` (compare `study_family_versions(1.0)` before and after).

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-families-floors-judge-26 (nit, simplification): roster: pending Finch's approval

`radix_units_party` carries a branch for a value `Party` cannot hold

Where: `crates/before/src/meter/board/operand.rs:261-263`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Delete the empty-stream branch

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-families-floors-judge-3 (nit, simplification): roster: pending Finch's approval

Idiom slips in family.rs and two floors.rs signatures

Where: `crates/before/src/meter/board/family.rs:547-548`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Bind the cross pair; rename the shadow; name the operand bytes; import `skip_subtree`

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### board-families-floors-judge-4 (nit, simplification): roster: pending Finch's approval

Em-dashes in `//` comments and in rendered legend strings

Where: `crates/before/src/meter/board/family.rs:614-615`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): ` -- ` in seven comments; colon or semicolon in three legend strings

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

