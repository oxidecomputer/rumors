<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P1 lane: the amplification board's judge, probes, and declarations

## Goal

The board is the one instrument pinning cost and heap order across the
operation-by-family product, and the review demonstrated that its verdict
of record is weaker than it says: the heap-exponent leg fits allowance-
inclusive readings so a quadratic heap term of allowance-scale magnitude
passes both windows; two rows measure only their early-exit verdict on
nearly every family; a family-stated heap ceiling has no under-side band,
a touch column is declared not applicable where the kernel's own doc
states the floor's premise, an unjudged exponent leg is uncounted, a
scan floor is stated at a rate its derivation does not produce, and the
worst-cases pin re-sweeps a grid the acceptance run already holds. Two
band docs claim an exclusion their slack does not deliver. The invariant
restored: every judged leg fits the quantity it claims to fit, every
probe measures the walk the roster credits it with, every declaration has
its under side, and every ceiling's doc states exactly what it excludes,
with a committed known-bad reading red through each repaired leg.

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

## Ordering inside the lane

1. board-families-floors-judge-21 first: land the construction as a red
   tripwire beside `exponent_guards_skip_noise_and_keep_real_amplifiers_red`
   (asserting today's behavior, `red` empty, so the commit shows the gap),
   then the residual fit and the guard flipping it to "heap exponent" in
   both windows.
2. The judge and declaration repairs (rulings 13's five) next; each is a
   judge-level unit test over synthetic samples first, red on the
   committed judge, then the fix.
3. The probe re-orientations (ruling 12) after: they move two
   `WORST_RANKINGS` pin rows, and the board run that re-pins them wants
   the judge settled first.
4. The segments currency's dissolution (ruling 38: board-ops-render-15,
   inventory-2, module-graph-1, recursion-1, and crate-root-32's board
   half), after `p1-harness` has landed and the envelope field is gone:
   one commit removing the currency, the readers, and the column, with
   the acceptance and pin renders diffed byte-identical on the four
   remaining columns; the shard `PROTOCOL` bump rides in it.
5. tests-other-27 and benches-examples-17 (ruling 50), and the two
   roster entries if approved; independent of the above.
6. The two band-doc entries in `tests/meter.rs` (envelopes-a-16,
   envelopes-b-4) last, rebased onto `p1-harness` if it has landed.

Every board run for a re-pin is `just amp-board-acceptance` (and the
worst-cases recipes) once at the parent, once after, with both outputs
kept under `<scratchpad>/p1-board/`; the deterministic counters make the
diff exact. Do not run the bench judge.

## Members

### board-families-floors-judge-21 (high, verification-gap): ruling 11

Resolution: Fit the heap trend over the same quantity the constant leg judges, `m.saturating_sub(HEAP_FLAT_ALLOWANCE_BYTES as u64).max(1)` on the cleared points, keeping the `cleared.len() >= 2 && spans` guards, and add a materiality guard so a residual of a few bytes above the allowance cannot manufacture an exponent (for example, each cleared residual at least a stated fraction of the allowance, derived and documented beside `HEAP_FLAT_ALLOWANCE_BYTES`). Commit the construction below as a tripwire beside `exponent_guards_skip_noise_and_keep_real_amplifiers_red`. Under the residual fit the existing `over_allowance` probe stays red and `straddling` stays unjudged, so the committed pins survive. Acceptance: a committed judge test builds four `Sample`s with `heap: Some(8192 + n*n/20480)` at n = 4096, 8192, 16384, 32768 (other columns NA or zero) and asserts `evaluate_acceptance` returns "heap exponent" in `red` for both windows; the existing judge tests stay green; `just amp-board-acceptance` stays green on the committed board (any new red is a finding to triage).

Ruled (11): the materiality guard is a fixed fraction of
`HEAP_FLAT_ALLOWANCE_BYTES`, each cleared residual at least that fraction,
the fraction derived (state the argument: what residual an allocator
rounding or a `Vec` doubling can contribute at the board's sizes, and why
the fraction sits above it) and documented beside the constant. The
docstring at judge.rs:25-26 ("a genuine super-linearity bends every point
and still reads red") is restated to what the residual fit delivers. If
`just amp-board-acceptance` turns any committed cell red under the new
fit, that is a finding: stop on the cell, report the readings, do not
re-pin or widen.

### board-ops-render-9 (medium, verification-gap): ruling 12

Resolution: Orient each probe so the certifying verdict is what the cell measures, from operands the bundle already supplies: for membership, `causally::since(&w).contains(&v)` (with `w = v + tick`, `v` is covered, so the query refuses and certifies every region; keep `membership_floors` deriving from the actual verdict so a concurrent pairing still floors correctly); for covers, probe `a.covers(&decode_party(&a_bytes))` (the buffer-distinct re-decode the placement rows use at ops.rs:1128-1135) or a prepared `join(a, b).covers(&b)`, floored at `scan_examines(n)`. If both verdicts are wanted, add the certifying rows (this moves every family's `Coverage::Board { cells }` count and the bench mirror). Acceptance: on the release board both rows render a full-examination scan floor on every family, their scan constants sit near a full walk's reading, and the two pin rows move with the movement annotated; the smoke suite's per-family cell counts are unchanged (replace) or re-stated (add).

Ruled (12): replace, not add. The two `WORST_RANKINGS` rows are
re-pinned with the movement named in the commit, measured at the parent.
Cell counts are unchanged.

### meter-adequacy-7 (low, verification-gap): ruling 12

Resolution: give version_eq a byte-equal, buffer-distinct operand pair (decode `v`'s bytes twice, as tests/meter.rs:7569-7570 does) so the compare runs its full length and the time leg's backstop claim is true; keep the ticked pair as a second cell if the early-exit arm is wanted. The hash rows are unaffected (hashing folds the whole buffer regardless of operand relation). Acceptance: the bench cell's median scales with operand bytes across the two scales. Construction: replace the byte compare with one that rescans `0..i` for each `i`; on every non-pair family the loop exits after one step, and neither the board nor the judge moves.

Ruled (12): the byte-equal, buffer-distinct pair replaces the ticked
pair; no second cell. The acceptance names the bench cell's median; do
not run the bench judge in this lane. Your evidence is the board's scan
reading for `version_eq` moving to a full-length compare (near the
operand's bits) on every family, and the construction (the rescanning
compare) demonstrated once as a reversible mutation whose board reading
you record in the commit message.

### board-ops-render-6 (medium, verification-gap): ruling 13

Resolution: Either band the family-stated heap ceilings as the capacity model is banded (a `DECLARED_HEAP_FLOOR` fraction of the declared constant, red as "heap family-stated floor (stale model)"), or commit a class-liveness pin for the certificate-memory mechanism (an envelope or asymptotics test asserting the ascend-cliff tick heap reads above the global 16 B/B, which reads red the day consumption is cured) and cite it from the two constants' docs as the mirror-wide constant cites `render_merge_superlinearity_is_alive`. Add the "improved" probe to tests.rs beside `declared_capacity_model_bands_the_projection_peak`. Acceptance: a synthetic `declared_heap: Some(227.0)` sample pair reading 10 B/B evaluates red on a stale-model leg, or a committed pin fails when the ascend-cliff tick heap drops under the global ceiling; ceilings.rs:360-407 name the under-side check; the release board stays green.

Ruled (13): the under-side band (the first option), as the capacity
model is banded, so the judge holds every declared model the same way.
The "improved" probe lands red-first beside
`declared_capacity_model_bands_the_projection_peak`.

### board-ops-render-10 (medium, verification-gap): ruling 13

Resolution: Add `touch_placement_fold(probe, lo, hi)` in floors.rs returning `Floor { min: max(nz(probe), nz(lo), nz(hi)), why }` (NA only when all three store no nonzero delta), state the arity-three premise beside `touch_pair_fold`, and use it on the five placement rows; decide `query_coverage` separately (its two-probe walk has clamp legs) and either floor it the same way or state positively why not. Retire `NA_TOUCH_PLACEMENT` if nothing else uses it. Acceptance: the five rows render a nonzero touch floor on every family with stored nonzero deltas; a synthetic span_place-like sample with `readings.touch = Some(0)` evaluates red on `TOUCH_FLOOR_TRIP` (a unit test beside the bypass-walk probe); the release board stays green.

Ruled (13): as stated. For `query_coverage`, read the walk: floor it
the same way if its clamp legs still route every nonzero delta of each
operand through the running difference; otherwise state positively at
the row why the pair-fold premise does not hold there. Report which.

### board-ops-render-18 (low, verification-gap): ruling 13

Resolution: At minimum, count unjudged non-heap exponent legs (and heap legs whose readings clear the allowance) on the summary line so the number is diffable. Better, give the exponent leg the floors' totality: a per-cell declaration on the rows whose operands legitimately do not scale (the benign rank pair), rendered in the legend, with an undeclared unjudged leg counted red by `run_acceptance`. Acceptance: forged captures with one cell's denominators flat across the ladder and readings growing ×100 make `run_acceptance` exit nonzero; the benign rank pair's declared reason renders in the legend; the release board stays green.

Ruled (13): the exponent leg is judged with the floors' totality (the
"better" option): a typed per-cell declaration for the rows whose
operands legitimately do not scale, rendered in the legend, and an
undeclared unjudged leg red. If the release board shows unjudged legs on
cells other than the benign rank pair, each is either declared with its
reason (if the operands legitimately do not scale) or reported as a
finding (if a generator stopped scaling): do not declare to green.

### board-ops-render-19 (low, performance): ruling 13

Resolution: Let the acceptance mode also fold and check the worst map from its two merged sample sets (evaluate each window with `evaluate` and feed `check_with` a closure that returns the already-merged results by scale), make `worst-cases-pin` an alias or a view recipe, and collapse the gate line and the CI steps to one invocation; unify the scale labels. Acceptance: one `cargo run ... acceptance` produces both matrices, both map tables, and the pin verdict; the gate's board stream invokes the example once; the rendered map tables are byte-identical to the current `just worst-cases` output at both scales.

Ruled (13): keep the pin, compute it from the acceptance results. The
byte-identity of the rendered map tables at both scales is the
acceptance and is checked by diffing the parent's `just worst-cases`
output against the new path's. The gate's board stream line and the CI
`instruments` steps change; `just --list` stays complete sentences.

### board-families-floors-judge-17 (low, documentation): ruling 13

Resolution: Either floor the tick rows at the exact single examination (`version.encoded_bits() + party.encoded_bits()`, still eight times the universal floor) and restate WHY_SCAN_TICK_WALK in live bits, or keep 8 per byte and name the recorded emission in the derivation. State at `SCAN_FLOOR_BITS_PER_INPUT_BYTE` (or `TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE`) why the tick rows alone take the full rate. Add the kept-calibration class to the ceilings.rs:56-62 convention header so the 2-5x margin reads as sanctioned. Acceptance: the WHY string derives the number it states; the two scan-floor constants' docs explain their relation; the convention header names what readings may remain.

Ruled (13): the floor is derived from the mechanism, not stated as 8
bits per byte: the first option, the exact single examination in live
bits (`version.encoded_bits() + party.encoded_bits()`), with
`WHY_SCAN_TICK_WALK` restated in those terms and the two constants' docs
explaining their relation. The kept-calibration class joins the
convention header.

### board-frame-8 (low, claim): ruling 9

Resolution: Re-word: "1.15 excludes polynomial super-linearity (a quadratic reads ~2 on every committed ladder) while leaving 0.15 for allocator rounding and `Vec` doubling; a log factor in the operand size fits inside that slack at the ladder's sizes (an n·log n kernel reads about 1.07-1.10), so documented log factors are held by the asymptotics suite's liveness pins and the fold rows' declared model, and an undocumented one by the constant legs, not by this bound." Add a probe beside the quadratic tripwire in board/tests.rs asserting `trend` over `(n, n·log2(8n))` at the smallest committed base reads under the ceiling, so the stated limit is pinned in both directions. Acceptance: no sentence in ceilings.rs asserts the global exponent ceiling excludes a log factor; a committed test asserts the n·log n ladder reads under `MAX_SCALING_EXPONENT` beside the existing assertion that a quadratic reads over it.

Ruled (9): as stated. The 1.15 constant does not move.

### envelopes-a-16 (low, claim): ruling 9

Resolution: Either tighten the flatness slack toward the board's bar (`10/9` matches exponent 1.15 across one doubling; the dense-suffix band would need its own declared slack for its log model) or restate the band docs as "per-unit growth at most ×1.25 across the doubling (exponent at most 1.32), which the committed kernel's ×1.5+ reading exceeds" and record at `SLACK_NUM` why the envelopes' bar differs from the board's. Acceptance: each band doc states the exponent bound its assertion enforces, and that bound is either the board's or justified at the constant.

Ruled (9): the second option. The ×1.25 slack stays where a declared log
model needs it; each band doc states the exponent bound its assertion
enforces; `SLACK_NUM`'s doc records why the envelopes' bar differs from
the board's. This entry edits `tests/meter.rs`: rebase onto `p1-harness`
if it has landed, and touch only the band docs and the constant's doc.

### envelopes-b-4 (low, claim): ruling 9

Resolution: either normalize the settle probes by the level count as `fold_stagger::assert_model_flat` does, so ×1.25 judges the model's constant, or correct the comment to the ratios at the committed counts (×1.5 at 4->8 and ×1.33 at 8->16 under the comment's formula; ×1.33 and ×1.25 under the level model) and rest the touch band explicitly on the measured non-dominance of the settle. Make `assert_flat_step`'s doc name both bands (touches ×1.25, limb ops ×1.5) and why they differ. Acceptance: the comment's number equals its formula at the smallest committed n; the fn doc matches its two numerator/denominator pairs.

Ruled (9): the second option (correct the comment; keep the constants;
state the exponent each band enforces). Same file-sharing note as
envelopes-a-16.

### board-ops-render-15 (high, verification-gap): ruling 38

Resolution: Owner ruling. (a) Recommended: dissolve the segments currency from the board: drop `ByCurrency::segments`, every row's `seg_ceiling_only()` declaration, the `seg[...]` column, the legend clause, `SEG_FLOOR_TRIP`, and the judge's segments arm; bump the shard `PROTOCOL`; re-state `LADDER_TOP_SCALE`'s rationale on the onset effects that do occur at ×4 (worst.rs:65-66 names the doubling-chain steps); re-word recurse.rs:17-20 and 74-76 and meter/tests.rs:399-403 so the unit test's claim is about the counter mechanism in the test build, not the board; handle or explicitly defer tests/meter.rs's segments pins in the same change. (b) Minimum: disclose in the legend and in board.rs that the counter has no writer outside the lib's test build, so a reader does not take the column as a measurement. Acceptance for (a): `ByCurrency` has four fields; the example and smoke suite pass with no `segments` text on the board face; the acceptance and pin renders are byte-identical on the four remaining columns. For (b): the legend names the column as an inert pinned zero and cites the lib unit test as the counter's only live witness.

Ruled (38): option (a) in full. The shard `PROTOCOL` bump is part of
the dissolution the ruling authorizes (the shard format is the board's
own capture format, not the gossip wire; `tests/gossip_snapshot.rs` and
the `insta` snapshots do not move, and any movement there stays a stop).
`LADDER_TOP_SCALE`'s rationale is restated on the onset effects that do
occur at ×4. The `tests/meter.rs` segments pins are the harness lane's
and are gone at your rebase point; if they are not, stop and report
rather than deleting them here.

### inventory-2 (medium, verification-gap): ruling 38

Resolution: retire the segments column: drop the `segments` field and its
assert from the `Envelope`/`TouchEnvelope` harnesses in `tests/meter.rs`, the
`stack_segments` read in `board/measure.rs`, the segments currency and its
ceiling-only declaration in the board, the `meter::stack_segments` and
`reset_stack_segments` readers, and the `cfg(any(test, feature = "meter"))`
on the static and its accessors (leaving them `cfg(test)`); keep `grow`,
`descend!`, and the counter as the test-surface guard for the oracle bridge,
with `stack_segment_meter_counts_deterministically_and_resets` as that guard's
own liveness test; re-word recurse.rs:16-20 to state that the guard is
test-surface machinery and that `deep_tree_stack_safety` is the committed
no-recursion proof. The alternative, making the column able to fail, requires
promoting `stacker` to an optional dependency under `meter` and routing a
committed known-bad recursive library shape through it, which contradicts the
deliberate dev-dependency placement. Acceptance: no envelope, board cell, or
doc reports a segments reading; `deep_tree_stack_safety` remains the depth
proof; the guard's unit test still runs under `cargo nextest run -p before`.

Ruled (38): as stated, less the `tests/meter.rs` clause (the harness
lane's). `stacker` stays a dev-dependency; the static and its accessors
go to `cfg(test)`; `stack_segment_meter_counts_deterministically_and_resets`
stays as the guard's own liveness test; `recurse.rs:16-20` is restated to
name the guard as test-surface machinery and `deep_tree_stack_safety` as
the committed no-recursion proof.

### module-graph-1 (medium, verification-gap): ruling 38

Resolution: Two accurate dispositions, one of which the owner picks. (a) Retire the currency where it
cannot move: drop the `segments` field and column from `tests/meter.rs`'s four envelope kinds,
remove `Currency::Segments` and its ceiling, floor-trip string, judge arms, and worst-map arm from
the board, keep `stack_segment_meter_counts_deterministically_and_resets` as the guard's own unit
test and the tick pin, make `mod recurse` `#[cfg(test)]`, and remove `meter::stack_segments`
and `meter::reset_stack_segments`. (b) If a production `descend!` user is foreseen, gate `grow`,
`should_grow`, `descend!`, and the three constants under `any(test, feature = "meter")` so the
column has a writer in every build that reads it; the column still reads zero on every current
kernel, and the readers stay. Under either disposition, rewrite `recurse.rs:74-76` and
`tests/meter.rs:22-26` now so they describe the writer that exists in the build that reads the
counter. Acceptance: no binary carries a segments ceiling whose counter has no writer in that
binary; both doc passages name the actual writer.
Construction: Add to `tests/meter.rs` a helper that recurses through `before::recurse::descend!`
and a scenario expecting `segments > 0`. The binary fails to compile (`descend!` and `grow` are
`cfg(test)` and `pub(crate)`); that compile error is the demonstration that no reading in the
integration binary can be nonzero. Equivalently, every segments cell of `cargo run --release -p
before --example amp_board --features limb-meter,scan-meter` prints 0 on every ladder.

Ruled (38): disposition (a). `mod recurse` becomes `#[cfg(test)]`;
`meter::stack_segments` and `meter::reset_stack_segments` are removed
(instrument surface under ruling 8, so no stable-API stop; `rumors` does
not call them, and if it does the change lands with the `rumors` update
in the same commit). `recurse.rs:74-76` is rewritten now; the
`tests/meter.rs:22-26` passage is the harness lane's.

### recursion-1 (medium, verification-gap): ruling 38

Resolution: Either dissolve the segments currency from the envelope suite and
the board (`Envelope.segments` and the segments columns of the other envelope
structs, `MAX_GROWN_STACK_SEGMENTS`, `Currency::Segments` and its NA policy
declarations, `meter::stack_segments`/`reset_stack_segments`), naming the
depth-100k/250k tests as the no-recursion detector where the column was cited
(recurse.rs:16-20, tests/meter.rs:22-26 and 6131-6133, board.rs:47-49), and
keep `SEGMENTS_GROWN` under `cfg(test)` only if the `meter/tests.rs` dive stays
as a test of the guard itself (re-word recurse.rs:16-20 to say it measures the
test-surface guard, not the library kernels); or keep the column and land a
committed known-bad demonstration that moves it in the meter build, which today
cannot exist without un-gating `descend!` and restoring `stacker` as a
dependency. Acceptance: either the segments currency is gone from
`tests/meter.rs` and the board with the detector named in its place, or a
committed known-bad demonstration exists in the meter build whose segments
reading exceeds the ceiling.

Ruled (38): dissolve. One change with the three entries above; the
public readers under the `meter` feature go with the currency.

### crate-root-32 (medium, verification-gap): ruling 38, this lane's half

Resolution: The owner's call between two sound options. (a) Dissolve, my recommendation: remove the segments currency from the board (`Currency::Segments`, `seg_ceiling_only()` on every cell, `MAX_GROWN_STACK_SEGMENTS`, `SEG_FLOOR_TRIP`, the render column) and `Envelope.segments` with every `segments:` pin in tests/meter.rs, and `meter::{stack_segments, reset_stack_segments}`; confine `SEGMENTS_GROWN` and its readers to `cfg(test)` beside their one live client, the determinism dive; name `clock::tests::deep_tree_stack_safety` (depth 100k) plus the structural fact that `descend!` is `cfg(test)` and `stacker` a dev-dependency as the instruments against reintroduced depth recursion. (b) Keep and make it live: `stacker` becomes an optional dependency enabled by `meter`, `grow`/`descend!` compile under `any(test, feature = "meter")`, and a guarded 200k-deep descent in tests/meter.rs and in the board's self-check reads `stack_segments() > 0` in each enforcing binary before the kernels' zero is asserted. Either way, restate recurse.rs:16-20 and 74-76 as what IS (the guard and its counter exist for the test-only oracle bridge and its witnesses; in non-test meter builds the counter has no writer), fix line 37, and excise tests/meter.rs:441's clause. Acceptance: (a) `grep -rn 'stack_segments\|SEGMENTS_GROWN\|Currency::Segments' crates/before` finds only the `cfg(test)` determinism witness, and `just gate` is green; (b) a test in crates/before/tests/ built with `--features meter` asserts `before::meter::stack_segments() > 0` after a guarded deep descent. In both, no prose calls the segments zero a measured fact.
Construction: Read-only: in a build of the library with `--features meter,limb-meter,scan-meter` and without `cfg(test)` (what tests/meter.rs and examples/amp_board.rs link), no expression writes `SEGMENTS_GROWN`; the sole `fetch_add` is inside `#[cfg(test)] fn grow`. Runtime: add a temporary scenario to tests/meter.rs whose body recurses 10^6 frames through `stacker::grow` directly; `meter::stack_segments()` still reads 0 and every `segments: 0` envelope passes. Conversely, delete the body of `stack_segment_meter_counts_deterministically_and_resets` and run the meter suite and `just amp-board-acceptance`: every segments ceiling stays green, because nothing in those binaries could have moved the counter before the change either.

Ruled (38): option (a). This lane lands the board currency's removal
(`Currency::Segments`, `seg_ceiling_only()`, `MAX_GROWN_STACK_SEGMENTS`,
`SEG_FLOOR_TRIP`, the render column), the readers, the `cfg(test)`
confinement of `SEGMENTS_GROWN` beside the determinism dive, the
`recurse.rs:16-20` and `74-76` restatement (what IS: the guard and its
counter exist for the test-only oracle bridge and its witnesses), and
line 37. The `tests/meter.rs` clauses are the harness lane's. The
acceptance grep (`stack_segments`, `SEGMENTS_GROWN`, `Currency::Segments`
finding only the `cfg(test)` determinism witness) is yours to run after
the rebase, since both halves are then in your tree.

### tests-other-27 (medium, correctness): ruling 50

Resolution: Join the round-robin groups the doc describes: `(a.version().join(b.version()), c.version().join(d.version()))`, which yields `(0, (0,1,0), (0,1,0))` and `(0, (0,0,1), (0,0,1))`, both present at the root's two children. Add a pool-membership floor to `every_family_answers_the_matrix_coverage_question`: each family's answer interns at least one version not already in the pool, or the collision is declared at the arm. Optionally (owner-gated) expose a smallest-instance door from the board's family module under the `meter` feature so the three organic pairs derive from the same code as `scatter`/`weave`/`benign`. Acceptance: `assert_ne!(weave_pair(), scatter_pair())` holds; the pool grows by two versions against the parent commit; the new floor reads red on HEAD's `weave_pair` and green after the swap.
Construction: In `every_family_answers_the_matrix_coverage_question` add `assert_ne!(weave_pair(), scatter_pair(), "the weave pair collapses to the scatter pair");` and run `cargo nextest run -p before --all-features --test verdict_matrix`: red at HEAD by the normal-form argument above; green after the join swap.

Ruled (50): fix the pair (join the round-robin groups the doc
describes) and add the pool-membership floor; the optional
smallest-instance door is not taken. Negative control: the floor reads
red on the parent's `weave_pair` (record the run in the commit message)
and green after the swap; `assert_ne!(weave_pair(), scatter_pair())` is
committed.

### benches-examples-17 (medium, test-quality): ruling 50

Resolution: decode a twin in a distinct buffer for the equal row (`let twin = Version::decode(&base.encode()[..]).unwrap();` and `("equal", &base, &twin, &obase, &obase)`); restate the doc ("equal streams in distinct buffers: the sweep runs to exhaustion"); optionally keep `&base, &base` as an explicitly named `identical` row if the rung's cost is worth tracking. Acceptance: at every `n`, the `before/equal` median scales with `n` like `before/ordered`; the doc names distinct buffers.
Construction: `just bench-quick version partial_cmp` at HEAD: `before/equal` reads near-constant (tens of nanoseconds) across n = 8..32768 while `oracle/equal` grows with n; after the twin change, `before/equal` grows with n.

Ruled (50): the distinct-buffer twin for the `equal` row, the doc
restated, and the same-reference case kept only as an explicitly named
`identical` row if you judge the rung worth tracking (say which). This is
a source edit to `benches/version.rs` only: do not run the bench or the
judge; the acceptance's median-scaling clause is the coordinator's to
observe on a quiet machine. Your evidence is `cargo bench --no-run` for
the binary and the diff.

### board-families-floors-judge-19 (low, verification-gap): roster: approved (ruling 104)

Resolution: Extend the probe pattern at tests.rs:430-457 with one `Sample` pair per remaining floored currency: `touch: Some(0)` under `touch_pair_fold(v, w)` on a dense pair -> `red == [TOUCH_FLOOR_TRIP]`; `limb: Some(0)` under `limb_stream(mandatory_limbs_stream(&hugeleaf(256)))` -> `[LIMB_FLOOR_TRIP]`; `heap: Some(0)` under `heap_materializes(n)` -> `[HEAP_FLOOR_TRIP]`. Acceptance: each of the four live `*_FLOOR_TRIP` constants is asserted by name in a committed test that feeds a zero reading against a floor the floors.rs constructors derived.
Construction: Reuse the tests.rs:430-457 `sample` closure with `touch: Some(0)` and `floors: walk_floors(n, touch_pair_fold(&v, &w))` where `v = version_of(&dense(1_000))` and `w` is `v` ticked at the seed; `evaluate` on two such samples must give `red == vec![TOUCH_FLOOR_TRIP]`. Repeat with `limb: Some(0)` and `floors.limb = limb_stream(mandatory_limbs_stream(&hugeleaf(256)))` (4 limbs per tests.rs:55), expecting LIMB_FLOOR_TRIP.

Roster note: three `Sample` pairs beside the existing scan-bypass
probe, each feeding a zero reading against a derived floor and asserting
its `*_FLOOR_TRIP` by name; lands with step 2's judge tests if Finch
approves the roster. Under ruling 43 the trip constants are asserted as
the values the judge returns, not re-spelled strings.

### board-frame-23 (low, correctness): roster: approved (ruling 104)

Resolution: Target the last leaf token whichever it is, `rfind(|c: char| c == '0' || c == '1')`, and re-spell `t` as `(t, t)`; the parser rejects `(0, 0)` and `(1, 1)` identically at the `)`, so the row's `Parse::NotCanonical` assertion holds and only closing parens follow the defect. Re-word the doc ("its last leaf token `t` re-spelled `(t, t)`, the non-normal pair judged at the node's close, the text's last token"). Acceptance: a unit test beside the builder: for the mounted `id-pair` operand, the produced text's `(t, t)` closes at the last non-paren byte and `parse::<Party>()` returns `Parse::NotCanonical`; the row's heap readings on the left-mounted families do not fall.
Construction: `Party` text `(((((1, 0), 0), 0), 0), 0)` (the `id_spine(4, false)` shape mounted left, 26 bytes) becomes `((((((1, 1), 0), 0), 0), 0), 0)`; `parse_id_tree` returns `NotCanonical` after consuming the 12 bytes `((((((1, 1)`, leaving 19 unparsed. A test asserting `d.len() - d.find("(1, 1)").unwrap() <= 8` fails on the current placer.

Roster note: a defect placer that misses the text's end on every left-
mounted operand; the fix targets the last leaf token and re-spells it as
a non-normal pair. The row's heap readings on the left-mounted families
must not fall (that would be a re-pin, a stop). Lands only if Finch
approves the roster.

## Hazards and stops

- Any committed board cell that turns red under the repaired judge, the
  re-oriented probes, or the new floors is a finding to report with its
  readings, never a cell to re-pin or a declaration to add to green.
- The two `WORST_RANKINGS` rows (ruling 12) are the only pins this lane
  moves by design; each movement is measured at the parent and named in
  the commit. Any other pin movement is a stop.
- The 480 B `MASKED_CMP_HOLE` pin and every heap pin in `tests/meter.rs`
  are out of scope (ruling 24's reproduction runs in the gate lane).
- `tests/meter.rs` is the harness lane's file first; your two band-doc
  edits rebase onto it, and the segments dissolution (step 4) waits for
  the harness lane's deletion of the envelope field so the readers you
  remove have no caller left.
- The shard `PROTOCOL` bump is the one format change this lane makes, and
  ruling 38 covers it; an `insta` snapshot or a wire-format pin moving is
  still a stop.
- No bench judge runs; the wall-time acceptance clauses are the
  coordinator's to observe on a quiet machine after merge.
