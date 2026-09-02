<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P1 lane: the mutation campaign's surviving findings

## Goal

Finch ruled the mutation campaign a one-time exercise (ruling 18): no
recurring machinery, and its roster retires in the gate lane. What the
campaign found is still true of the code: two mutants survive every
committed test (a non-equivalent operator swap in `PackedBuilder::read_bits`'
committed-byte chunk arm; the annihilation reordered ahead of the follower
fold in `drop_below`), and two suanpan exclusions claimed equivalence for
mutants that change exact touch counts, which suanpan declares a public
contract. The invariant restored: each surviving mutant has a committed
test that kills it, and no codepoint exists whose only defense was an
exclusion entry. "Not wrong, but you couldn't tell if it were" is what
this lane repairs.

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

## Ordering against the gate lane

This lane runs after `p1-gate`'s roster-retirement commit lands, or
rebases onto it before its gate run. Reason: suanpan-40's `read_digits`
restructure removes two `>>=` codepoints, which changes the mutant count
the `mutantcheck` gate leg compares against
`tools/mutantcheck-expected.json`; while that leg exists your gate would
fail on a count that is retiring, not being re-pinned. You do not edit
`.cargo/mutants.toml` or `tools/mutantcheck-expected.json`; if they still
exist at your base, stop and report before running the gate.

## Members

### codec-bits-15 (medium, verification-gap): ruling 18

Resolution: Add `codec/build/tests.rs` mirroring the `BitsBuf` family: an arbitrary interleaving of `push_bit`, `push_code` (Small at every `len` in 1..=63, and Wide), `reserve` then `patch_bit` at committed and staged positions, `splice` from a view at every source alignment into every output alignment, `truncate` to byte-aligned, mid-byte, and empty targets, and `extract_code` at ranges below and above 63 bits spanning the committed/staged boundary; assert `len()` and `finish()` equal a clean `BitsBuf` rebuild at every step, and `extract_code`'s bits equal the model's slice. Add a `load_be` differential: for random buffers and every `(start, len <= 64)` within the live length, `load_be(start, len)` equals the fold of `bit(start + i)`. Acceptance: both suites are committed and red under each hand mutation: build.rs:218 `>> (8 - rem)` to `>> rem`; 297 `(8 - within - take)` to `(8 - within)`; 278 `carry << (8 - r)` to `carry << r`; bits.rs:340's `| (u64::from(buf[8]) >> (8 - shift))` dropped.

Ruled (18): as stated. The witness pass demonstrated the `read_bits`
chunk-arm survivor with the operator swap `>>` to `<<` at build.rs:297
(byte 0xB0, pos 0, n 2 reads 2 originally and 0 under the mutant);
include that mutation among the hand mutations you demonstrate, and
record every mutation's observed failure verbatim in the commit message.
Tests go in a sibling `tests.rs` (`mod tests;`), proptest for the family,
with the shrunk seed committed if one appears.

### skyline-watermark-24 (medium, verification-gap): ruling 18

Resolution: rewrite 360-363 to what runs ("the refusal left the web able to fire a true undercut; its residue passes the outer pair's zero run, the close pops that run, and the outer range reads the undercut's value exactly"), optionally adding `prop_assert!(matches!(web.close(), Close::ZeroRun))`. Then add a directed pin for the missing arm in the `dominated_latent_annihilates_into_the_undercut_residue` scenario: install `follower_set(0, acc)` holding `start` after the first arming; after `emit_offset(&below(50 + E))` take and `materialize` it and expect `start + D − E` (the arms fold `+D` and `+50` into the follower via `push_boundary`, the park tags it, the undercut subtracts the pre-annihilation `E + 50`). Acceptance: the new pin fails when the annihilation block is moved above the follower loop (reads `start + D + 50 − E`) and when line 537's `sub_accum` becomes `add_accum` (reads `start + D + 50 + E + 50`); the rewritten comment names no follower and no parked boundary.

Ruled (18): as stated, the optional `prop_assert!` included. The witness
pass ran both mutants against the whole fill, watermark, tick, and grow
suites: mutant 1 (the reorder) survived everything, mutant 2 (the
polarity flip) was killed by six fill tests. Your pin must kill mutant 1;
record both mutations' observed readings in the commit message. If the
expected value `start + D − E` does not match what the pin reads on the
unmutated code, stop and report the reading rather than adjusting the
expectation: the entry's arithmetic is a trace, not a run.

### suanpan-40 (medium, verification-gap): ruling 18, amended

Resolution: (1) `read_digits`: the site's own derivation (1076-1079) bounds `|carry| <= 3`, so `high` fits one digit; replace both `while high > 0 { ...; high >>= DIGIT_BITS; }` loops (1109-1113, 1118-1122) with `if high > 0 { touch(1); collected.push(u32::try_from(high).expect("the final carry fits one digit: |carry| <= 3")); }`, which moves no touch, removes both `>>=` codepoints, and dissolves the exclusion per ladder step (1). (2) `shl`: delete the exclusion and add exact pins to metered.rs: `shl(0)` on a digit-engine value and `shl(40)` on a literal-zero register both cost 0 touches, with `quick.is_some()` asserted after the latter; also pin a readout with a nonzero final carry (a single digit `-(2^33 - 1)` reads out as low 1, carry -2, one drain digit: 3 touches) so the negative read-out count is covered. (3) Update tools/mutantcheck-expected.json:4-11 (listed 2 / suppressed 2 and 1 / 1) in the same commit, or the mutantcheck leg fails. If the owner keeps either exclusion, restate its rationale to name the touch leg it waives, as the header demands. Acceptance: `cargo mutants --workspace` with both entries removed reports the `&&` mutant caught and the `>>=` mutant no longer listed. Construction: apply the `<<=` mutant by hand and run a metered scenario with a nonzero final carry (`park_extreme_negative_digit(&mut a, 0)` from accumulator/tests.rs:110, then `touch_meter::reset(); a.sign_magnitude()`): production counts 3 (digit, complement pass, one drain), the mutant 6. Apply the `&&` mutant: `a.add_wide(&(UBig::ONE << 2048usize)); touch_meter::reset(); a.shl(0);`: production 0 touches, mutant about 130.

Ruled (18): parts (1) and (2) land. Part (3) is moot: the roster and
its expected file are deleted by the gate lane, and the exclusions
dissolve by that deletion plus your code change, not by editing them.
The stated Acceptance (`cargo mutants --workspace` reporting) is
replaced: no campaign runs. Your acceptance is (a) `grep -n '>>= DIGIT_BITS'
crates/suanpan/src/accumulator.rs` returns nothing and the `expect`
message states its premise (`|carry| <= 3`) as the site's derivation
does; (b) the three new exact pins in `metered.rs` are green on the
committed code and red under the two hand mutations (the `&&` swap in
`shl`'s guard; the `<<=` in the carry drain, applied to the pre-refactor
code for the readout pin, or to the refactored `if` as an equivalent
count-moving edit), with the observed counts recorded in the commit
message (the witness measured 3 vs 6 and 0 vs 66); (c) suanpan's whole
suite green under `cargo nextest run -p suanpan --all-features`.
`Ticks::limbs`'s callers are unaffected; if the restructure changes any
committed touch pin elsewhere in the workspace, that is a stop.

### suanpan-tests-25 (medium, verification-gap): ruling 50

Resolution: pin the exact totals the way metered.rs does: assert `s1(n, d) == 16 * n + 2` for all four grid cells with the per-round derivation in the message (add 2: one limb read plus one deposit; sign 5: read, zero, deciding read, collapse zero, re-deposit; sub 2; sign 7: read, zero, read to partial 0, zero, certificate skip to digit 0, read −1, collapse zero and re-deposit; plus 2 once for the parked −1's spill). The measured number is the refutation pass's, not mine; measure once more before committing. At minimum add the universal floor `grid[0] >= 4 * n0` (four calls per round, one touch minimum each). Acceptance: with `touch()` stubbed to a no-op under the feature the test fails; with the shipped code it passes with the pinned totals, and the mixed second difference is exactly 0.
Construction: make `fn touch(count: u64)` (accumulator.rs:21-26) ignore `count` under `touch-meter` and run only `sign_flip_oscillation_has_no_width_product`: it passes with `grid [0, 0, 0, 0]` while every pin in metered.rs fails. Alternatively delete only the `touch(1)` at accumulator.rs:850 (the fold's per-digit read): the grid drops uniformly and the test still passes.

Ruled (50): pin the exact totals (`16 * n + 2` at all four grid cells)
with the per-round derivation in the assertion message, measured once at
the parent before committing (the number is the witness pass's, a
hypothesis until your run confirms it; if it does not, report the
measured totals and pin those with their derivation, or stop if no
derivation reproduces them). The negative control is the acceptance's
own: `touch()` stubbed to a no-op under the feature, a reversible
mutation, recorded in the commit message. Same file family as suanpan-40;
land after it so the pins read the restructured `read_digits`.

### suanpan-39 (low, verification-gap): roster: approved (ruling 104)

Resolution: a per-cell floor from irreducible work (two one-limb writes and two sign reads per round: `s1(n, d) >= 6 * n`); better, pin the exact total `16 * n + 2` at all four cells, which subsumes the floor and the no-product bound. Treat `16n + 2` as a hypothesis until one run confirms it. Acceptance: with `touch_meter::record` stubbed to a no-op the test fails; unmodified, the pinned counts hold at all four cells. Construction: stub `record` and run `cargo nextest run -p suanpan --features touch-meter sign_flip_oscillation_has_no_width_product`: it passes today.

Roster note: the same anchor and defect as suanpan-tests-25, filed by
the suanpan partition (its Resolution names the same `16 * n + 2` pin).
Landing suanpan-tests-25 satisfies this entry's Acceptance; the
coordinator may record it as `dup` of suanpan-tests-25 rather than a
separate fix. Nothing further to do here.

## Hazards and stops

- The touch counts are suanpan's public contract (`lib.rs:283-286`). Any
  movement of an existing exact pin is a stop; the restructure is
  specified to move no touch.
- Do not edit `.cargo/mutants.toml`, `tools/mutantcheck*`, or
  `AGENTS.md`; the gate lane retires them.
- The panic policy applies to the new `expect`: its message states the
  premise, and the premise is the site's own bound on `|carry|`, so the
  panic is programmer-error only. Say so in the message, not in a comment
  citing this review.
