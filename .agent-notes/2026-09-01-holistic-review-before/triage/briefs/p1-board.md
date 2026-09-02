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
4. The two band-doc entries in `tests/meter.rs` (envelopes-a-16,
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
  edits rebase onto it.
- No bench judge runs; the wall-time acceptance clauses are the
  coordinator's to observe on a quiet machine after merge.
