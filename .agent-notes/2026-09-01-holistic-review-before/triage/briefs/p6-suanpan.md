<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P6 lane: suanpan (the crate and its test suites)

## Goal

suanpan's per-module entries, landed per their Resolutions inside an approved roster, under ruling 88 (the exact-touch contract retired; every pin a ceiling), 43, and 65 (the witnesses relocated into `crates/suanpan/tests/`). Ruling 94 (suanpan drops dashu; limbs at the boundary) lands in `p7-api` before this lane: every row here is landed against a dashu-free suanpan, and any Resolution that names `UBig` in suanpan's surface is re-read against the limb-slice API and reported if it no longer applies.

## Rulings on this lane's mediums

Every medium in this lane is ruled (rulings 93 to 103): suanpan-tests-7. The decisions stand beside each entry under Members.

## Roster summary

1 ruled (1 medium); 1 medium ruled (93 to 103); 35 roster members approved (ruling 104) (17 low, 18 nit).

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

Every P6 lane runs after the P1 to P5 and P7 lanes that touch its files have landed on main, or rebases onto them before its final gate run; the coordinator names the base SHA at launch. Lows and nits inside this lane's approved roster are swept without a question to Finch; every high and medium has an individual ruling. A change that would alter a rendered `before` doc panel is a stop (ruling 89). Follows `p1-survivors`, `p7-api` (the `Limbs` impls, the shift deletions, the swap pin), `p8-performance` (suanpan-17, -14, the contract restatement), and `p4-rosters` (the relocation). suanpan-10, suanpan-tests-4, and suanpan-tests-8 are held by ruling 86 and are not this lane's. Owns `crates/suanpan/**`.

## Members

### suanpan-2 (medium, verification): ruling 91

The "on every input sequence" quantifier has no instrument over arbitrary sequences

- Owner-gated: yes (a documented instrument model)

Resolution: a touch-meter proptest (metered.rs or tests/amortized_sequences.rs) that drives the existing `arb_op()` streams with interleaved sign reads, records `touches` beside a work denominator (word-scale calls + limbs yielded + sign reads + one spill), and asserts `touches <= K * work + D` with K derived in the test's doc comment from the potential argument (each `add_at` iteration deposits one credit; fold reads and settle steps each spend one; carry steps are bounded by the zone refill). Commit a known-bad demonstration beside it (for example `settle_top` with the `consume_run_at` arm disabled) and show the property reads red on run-forming streams. Acceptance: the property runs in the gate under `--all-features`; the known-bad demonstration fails it while the fixed-schedule pins are shown not to notice. Construction: `proptest! { fn touches_are_linear_in_work(ops in vec(arb_op(), 1..300), engine_first: bool) { touch_meter::reset(); let mut acc = fresh(engine_first); let mut work = 0u64; for op in &ops { apply(&mut acc, &mut oracle, op); work += op.limbs_or_one(); acc.sign(); work += 1; } prop_assert!(touch_meter::touches() <= K * work + D); } }` with K and D derived in the doc comment; the metered suite must run serially (touch_meter.rs:16-20).

Ruled (91): the touch-meter proptest over arbitrary op streams with interleaved sign reads asserting `touches <= K·work + D`, `K` derived from the potential argument in the test's doc (a ceiling under ruling 88), and a committed known-bad (the disabled run-consumption arm) shown red on run-forming streams while the fixed-schedule pins do not notice.

## Mediums ruled 93 to 103

Each medium below now carries its ruling and any amendment beside its quoted Resolution; land per the ruling.

### suanpan-tests-7 (medium, verification): ruling 103

size_probe_covers_the_value asserts a full digit of slack its doc does not claim, and the doc's own bound is one bit too tight

- Owner-gated: no

Resolution: tighten to `* 32 + 2 >= bit_len` and restate the doc with the derivation: every digit is under 2^33 in magnitude, so the value is under (2 + 2^−31)·2^(32·digit_count), at most two bits past the counted width; a register value's `digit_count` is `ceil(bits/32)`, so the same bound holds there. Import `DIGIT_BITS` for the 32 if the owner prefers named constants in the model. Acceptance: with `+ 2`, `size_probe_covers_the_value` passes on the committed tree, and the mutant `None => self.top` in `Accumulator::digit_count` fails it by name. Construction: in accumulator.rs:946 change `None => self.top + 1,` to `None => self.top,`; run `size_probe_covers_the_value` (expected: passes with the current `+ 33`); change the test to `+ 2` and re-run (expected: fails). Restore the production line.

Ruled (103): Tighten to `32·D + 2` with the derivation in the doc. See ../rulings.md.

## Roster members approved (ruling 104)

Lows and nits approved as this lane's roster by ruling 104. Land each per its quoted Resolution and Acceptance, swept with the ruled members; report rather than choose if a Resolution conflicts with a ruling or offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89).

### paper-fidelity-9 (low, documentation): roster: approved (ruling 104)

suanpan's lazy-zone amortization is asserted, not derived, under a page that promises every argument "in full"

- Owner-gated: no

Resolution: add the potential (as above) to the lazy-zone section, or soften "in full" to "in outline" for that one argument. Acceptance: each of the three named arguments states the quantity it amortizes against.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-12 (low, simplification): roster: approved (ruling 104)

The shift split and its `expect` are duplicated at four sites; one helper would carry the 32-bit argument and the checked landing position

- Owner-gated: no

Resolution: `fn split_shift(shift: u64) -> (usize, u32)` whose doc carries the 32-bit-only argument and the single `expect`, called at the four sites; `bit_shift` as `u32` is the idiomatic shift-amount type. Acceptance: one `digit positions fit a usize` string remains; the `# Panics` sections need no contract change.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-22 (low, simplification): roster: approved (ruling 104)

The zero-run ledger reads as a type but lives as a field plus three methods on `Accumulator`

- Owner-gated: no

Resolution: a private `struct ZeroRuns(BTreeMap<usize, usize>)` (same file or a sibling `accumulator/ledger.rs`) with `record(lo, hi)`, `crop(from, to)`, `consume_below(above)`, `clear()`, and `iter()` for the ledger suite; move the certificate semantics onto it, and let the `Accumulator` field doc state only how the accumulator maintains it. Acceptance: no `self.zero_runs.insert/remove/range` outside the newtype; `ledger_invariants_hold_exhaustively` asserts the same clauses.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-26 (low, verification): roster: approved (ruling 104)

`Limbs::next_back` and the 32-bit word pairing are exercised only by a consumer crate

- Owner-gated: no

Resolution: a sibling `limbs/tests.rs` proptest: for random little-endian byte strings `b`, `Limbs::new(&UBig::from_le_bytes(&b)).collect::<Vec<u64>>()` equals the minimal LE `u64` limbs of `b`, `.rev()` equals the reverse, and zero yields an empty iterator; the same test covers the pairing if a 32-bit run is ever wired. Acceptance: a mutant replacing `next_back`'s body with `self.chunks.next().map(pack_limb)` fails inside suanpan. Construction: make that swap and run `cargo nextest run -p suanpan --all-features`: green today.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-32 (low, documentation): roster: approved (ruling 104)

Roster data states mechanisms and labels the code does not have

- Owner-gated: no

Resolution: rewrite the `new` reason ("builds a register-held zero: no allocation, no digit, no input axis to measure against"), the `digit_count` reason to cover both tiers, and the label to "(trait surface)" at 89 and 398 with the exclusion text at 400-404 adjusted. Acceptance: each sentence matches the current body it describes; `cited_witnesses_exist` and `claims_are_total_over_the_public_surface` still pass.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-33 (low, verification): roster: approved (ruling 104)

The only sequence-shaped adversarial instrument is cited by no claim

- Owner-gated: no

Resolution: `const SEQUENCES: &str = "tests/amortized_sequences.rs";` cited under `sign`, `add_wide_shl`, and `sub_wide_shl`; and state the roster's citation policy (minimal sets or every touch instrument) at claims.rs:111-115 so the next uncited instrument is a decision, not an omission. Acceptance: renaming the test fails `cited_witnesses_exist`; the reach test passes without a `REACH_EXEMPT` entry. Construction: delete the file on a scratch copy: the suanpan suite stays green today.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-37 (low, verification): roster: approved (ruling 104)

The testdoc says exclusions "state a mechanism"; the body checks a 20-character floor

- Owner-gated: no

Resolution: make the doc match the check at 262-263 and 23-24 ("every exclusion carries a reason of non-trivial length; its substance is review-held"); optionally strengthen the check (require a code identifier or a denomination word such as "word-scale", "digit", "allocation") and say so. Acceptance: the docstring's invariant is one the assertion can fail on. Construction: set the `new` reason to any 20+ character string; `cited_witnesses_exist` passes.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-38 (low, verification): roster: approved (ruling 104)

`scaled_read_costs_the_written_span` asserts a `<= 16` ceiling where the exact count is 3

- Owner-gated: no

Resolution: `assert_eq!(scaled_read, 3, ...)`, confirming 3 by one run before pinning; keep the `> 1000` control; amend the header to say adequacy legs (`> 1` at 793, the control at 51) are floors by design. Acceptance: the test passes at exactly 3 and the header sentence is true of the file.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-5 (low, documentation): roster: approved (ruling 104)

`reserve_digits` is absent from the crate page

- Owner-gated: no

Resolution: one sentence in this paragraph: "A caller that knows the scale its writes will reach can pre-size the buffer once with [`reserve_digits`](Accumulator::reserve_digits)." The table stays as is (no digit-touch axis). Acceptance: `grep reserve_digits lib.rs` finds it; `cost_table_rows_bind_to_the_roster` untouched; README re-derived.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-8 (low, verification): roster: approved (ruling 104)

The documented trait surface (`Send + Sync`, deliberately not `PartialEq`) has no compile-time pin

- Owner-gated: no

Resolution: `static_assertions` as a dev-dependency; in accumulator/tests.rs, `assert_impl_all!(Accumulator: Send, Sync, Clone, Default, Debug); assert_not_impl_any!(Accumulator: PartialEq);`. Acceptance: adding `#[derive(PartialEq)]` at accumulator.rs:89 fails to compile the test target. Construction: add that derive today; nothing fails.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-tests-10 (low, verification): roster: approved (ruling 104)

the ledger checker never observes a state produced by the fold, merge, shift, reset, or negate entry points

- Owner-gated: no

Resolution: cheapest: call `assert_ledger_invariants(&acc, &[])` at the end of `merges_match_the_oracle`, `in_place_shift_matches_the_oracle`, `width_ordered_merges_match_the_oracle`, and `fold_primitives_match_the_oracle` (it is a private-state checker in the same test tree). Better: add arms to `ledger_invariants_hold_on_run_forming_streams` for `negate`, `shl(shift)`, `reset` (then continue the stream, so post-reset spills over the retained buffer are checked), and `add_accum_shl(&snapshot, shift)` where the snapshot is a clone taken earlier with its oracle tracked. Acceptance: the checker runs on a state after each of `add_accum_shl`, `sub_accum_shl`, `merge_into_wider`, `shl`, `negate`, and `reset`; the construction below fails the suite. Construction: replace the engine branch of `shl` (accumulator.rs:626-627) with an in-place digit shift that moves the digits up by `digit_shift`, touches twice per digit (so the exact 2d pin at metered.rs:443-450 still holds), and leaves `zero_runs` un-reindexed. Every committed test passes (values are correct and no ledger driver calls `shl`); a certificate `(lo, hi)` now covers position `lo + 1`, which holds the old nonzero digit from `lo`, so the proposed `shl` arm fails the soundness clause at the first shifted state, and a subsequent zero-partial fold would skip over a nonzero digit.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-tests-13 (low, verification): roster: approved (ruling 104)

three inequality pins and six single-shape pins in a module whose doc says every pin is exact and doubled

- Owner-gated: no

Resolution: pin `scaled_read == 3`, the full read `== 1003`, and the below-bound descent `== 6` (measure the last once before committing; my 6 is by reading), each with its derivation in the message as the sibling pins do, and replace "O(1)-ish" at :22 with the number. Narrow the module doc's doubling clause to the pins that double, or add the doubling where it is cheap (a second parked digit index for the scaled reads, a second prefix width for the settlement scan). Acceptance: every assertion on `touch_meter::touches()` in metered.rs is `assert_eq!` against a derived constant, and the module doc's two sentences are true of every pin in the file. Construction: insert a second `for &digit in &self.digits[start..=self.top] { touch(1); }` loop into `read_digits`: `scaled_read` becomes 6, still `<= 16`; the full read becomes 2006, still `> 1000`; every exact pin elsewhere fails, so the gap is specific to these assertions.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-tests-15 (low, simplification): roster: approved (ruling 104)

alternating_shifted_writes repeats one loop body four times, and the word-magnitude path is the one never doubled

- Owner-gated: no

Resolution: table-drive the four scenarios over `[32_000, 64_000]` with an inline tuple of setup/sub/add closures, expected per-pair cost, and label (as the dispatch test does, with `#[allow(clippy::type_complexity)]`); expected totals stay 5_000 / 3_000 / 5_000 / 3_000. Acceptance: one loop body; the magnitude word path is asserted at both shifts; green.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-tests-18 (low, verification): roster: approved (ruling 104)

negative read-out cost (the complement pass) is unpinned; the held-width row is metered only on the positive spelling

- Owner-gated: no

Resolution: after the positive legs, negate once more and meter `sign_magnitude` and `sign_limbs` on the negative spelling, pinning `2 * held_digits` at both widths with the derivation (carry pass plus complement pass; the complement of a nonzero low part never carries out, so no high digit is pushed). Acceptance: an exact pin for both read-outs on a negative d-digit value at two widths. Construction: `acc.negate(); touch_meter::reset(); let _ = acc.sign_magnitude(); assert_eq!(touch_meter::touches(), 2 * held_digits);` (the 2d is my derivation; measure once before committing).

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-tests-2 (low, documentation): roster: approved (ruling 104)

park_extreme_negative_digit's stated reason for two deposits is false: one deposit of −(2^33 − 1) lands in the zone

- Owner-gated: no

Resolution: collapse to one call after the spill (`acc.sub_magnitude_shl(&UBig::from((1u64 << 33) - 1), 32 * index);`) and restate the doc: the zone is open at 2^33, so the extreme digit lands in one deposit; the spill first keeps the register from holding the value exactly instead of as a digit. Optionally add `debug_assert_eq!(acc.digits[index as usize], -((1i64 << 33) - 1))` so the helper pins its own postcondition. Acceptance: `witnesses.rs` and `differential.rs` pass unchanged with the one-call helper and the postcondition assert. Construction: make the one-call change and run the accumulator tests; a green run demonstrates the single deposit does not recenter, refuting the sentence.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-tests-23 (low, verification): roster: approved (ruling 104)

the executable headroom derivation does not check what its expect messages say: checked_shl never detects bits shifted out

- Owner-gated: no

Resolution: replace the three lines with a compile-time assertion beside the constants in accumulator.rs using overflow-detecting arithmetic, e.g. `const _: () = assert!((QUICK_MAX << QUICK_SHIFT_MAX) + QUICK_MAX <= i128::MAX as u128);` in `u128` (where a shift past 127 is itself a compile error), with the headroom argument restated at the constants (accumulator.rs:63-65 already states it in prose); keep the value-level extremes, which are the test's substantive content. Acceptance: the build fails if `QUICK_SHIFT_MAX` is raised past what `i128` admits; the witness body starts at the value-level extremes. Construction: set `QUICK_SHIFT_MAX` to 31 in a scratch build: lines 494-500 pass (`checked_shl(31)` returns `Some(i128::MIN)`), and the value legs at :525-546 fail in debug (overflow panic in `fold_accum`'s shift) or by oracle mismatch in release. Restore the constant.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-tests-26 (low, verification): roster: approved (ruling 104)

the one committed sign-flip instrument is not cited by the claims roster's sign rows

- Owner-gated: no

Resolution: add `const SEQUENCES: &str = "tests/amortized_sequences.rs";` and cite `(SEQUENCES, "sign_flip_oscillation_has_no_width_product")` on the `sign` claim, and on `is_negative` with a `REACH_EXEMPT` entry mirroring the existing delegation reason. Whether to instead move the test into metered.rs is an open question below. Acceptance: `cited_witnesses_exist` and `cited_witnesses_reach_their_operations` pass with the new edge; renaming the test fails `cited_witnesses_exist` by name.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### api-audit-23 (nit, documentation): roster: approved (ruling 104)

suanpan states there is no from-value constructor without saying why

Where: `crates/suanpan/src/lib.rs:244-248`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): add the clause that names what the absence serves, or add the constructor if no reason survives

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-11 (nit, simplification): roster: approved (ruling 104)

`negative: bool` threaded through three private paths yields seven `if negative { -x } else { x }` sites

Where: `crates/suanpan/src/accumulator.rs:542-546`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One sign multiplier per function

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-15 (nit, documentation): roster: approved (ruling 104)

`sign_dominates_at`'s public doc carries the margin proof at user altitude

Where: `crates/suanpan/src/accumulator.rs:786-794`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): move 786-794's inequality chain to the `SIGN_DECIDED` doc (45-50) or a `//` comment above 826 ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-16 (nit, simplification): roster: approved (ruling 104)

Literal `32` and `128` beside the named `DIGIT_BITS`

Where: `crates/suanpan/src/accumulator.rs:812-817`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `DIGIT_BITS` and `u128::BITS`; state the totality premise once

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-18 (nit, simplification): roster: approved (ruling 104)

Destructure-and-retuple in `sign_magnitude`; a redundant clamp between `sign_magnitude_shl` and `read_digits`

Where: `crates/suanpan/src/accumulator.rs:966-967`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `read_magnitude(0)`; one clamp

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-23 (nit, documentation): roster: approved (ruling 104)

Headroom comments state loose bit counts with an underived `33`

Where: `crates/suanpan/src/accumulator.rs:1473-1474`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "At most 32 + 31 bits" and "At most 64 + 31 bits"

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-29 (nit, simplification): roster: approved (ruling 104)

`pub(crate)` on `cfg(test)` roster items read only by their child module

Where: `crates/suanpan/src/claims.rs:36-36`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Drop `pub(crate)` on the nine items

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-31 (nit, simplification): roster: approved (ruling 104)

The `constant()` shorthand covers four rows while six rows of the same shape are longhand

Where: `crates/suanpan/src/claims.rs:101-109`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Rename to `excluded` and use it for all ten rows

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-34 (nit, verification): roster: approved (ruling 104)

`digit_count`'s exclusion reason names `top_settlement_steps_are_metered`, a test the roster never checks.

Where: `crates/suanpan/src/claims.rs:316-320`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Cite it as a witness with a `REACH_EXEMPT` entry, or drop the name.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-36 (nit, simplification): roster: approved (ruling 104)

The table locator hardcodes `Accumulator::`, an unreachable `else` arm, and `position` without the uniqueness the doc claims

Where: `crates/suanpan/src/claims/tests.rs:216-217`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Locate by `]({op})`; plain `rsplit`; assert one header

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-tests-1 (nit, simplification): roster: approved (ruling 104)

differential.txt carries three seeds whose shrink records name an arm its property cannot draw

Where: `crates/suanpan/proptest-regressions/accumulator/tests/differential.txt:7-9`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Comment the carried seeds, or prune them naming the split (owner's word)

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-tests-14 (nit, documentation): roster: approved (ruling 104)

two doc fragments left by the dated-note excision: "Green pin: an" and a verbless "Measured (...)"

Where: `crates/suanpan/src/accumulator/tests/metered.rs:69-71`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "An alternating shifted pair costs its operand, not the zero run under it: exact totals ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-tests-19 (nit, simplification): roster: approved (ruling 104)

em-dashes in two assert messages and four code comments

Where: `crates/suanpan/src/accumulator/tests/metered.rs:599-600`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Colon or semicolon at six sites

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-tests-20 (nit, documentation): roster: approved (ruling 104)

domination_reads' doc names sign_dominates_at for a loop that calls sign_dominates_word, and overclaims one touch per later read

Where: `crates/suanpan/src/accumulator/tests/metered.rs:719-726`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): state the shape: after the first collapse a decided top answers every later read in one touch; the thousand reads go through `sign_dominates_word` ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-tests-21 (nit, simplification): roster: approved (ruling 104)

the known-bad fold model hardcodes the decision threshold 3 where SIGN_DECIDED is importable

Where: `crates/suanpan/src/accumulator/tests/metered.rs:855-855`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `partial.abs() >= SIGN_DECIDED`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-tests-24 (nit, simplification): roster: approved (ruling 104)

amortized_sequences.rs housekeeping: an orphaned "S1" label, qualified Ordering, f64 over exact counters, an unnamed 0.10, and a cross-crate copy of the helper

Where: `crates/suanpan/tests/amortized_sequences.rs:8-8`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Import `Ordering`; rename `s1`; `i128` with a named bound

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-tests-3 (nit, verification): roster: approved (ruling 104)

The differential generator never draws `sub_small`, `add_u64`, `sub_u64`, `add_u64_shl`, `sub_u64_shl`; the last two run only under `touch-meter`; `merges_match_the_oracle` ignores `fresh`.

Where: `crates/suanpan/src/accumulator/tests/differential.rs:50-53`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Add `Op::Word`/`Op::WordShl` arms; route negative `Small` through `sub_small`; take engine flags in the merge test.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### suanpan-tests-5 (nit, simplification): roster: approved (ruling 104)

constructor and conversion idioms are spelled two ways for the same role

Where: `crates/suanpan/src/accumulator/tests/differential.rs:415-415`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One spelling per role

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

