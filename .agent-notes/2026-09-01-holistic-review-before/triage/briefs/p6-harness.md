<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P6 lane: the test harness, the envelopes, and the other suites

## Goal

The entries of `testing/`, `tests/meter.rs`, and the other `tests/*.rs` binaries that no pattern or decision grouped, landed per their Resolutions inside an approved roster, under rulings 4 (the unified harness), 38 (no segments), 43, 50, 88, and 91 (`doc_hidden.rs` retired).

## Rulings on this lane's mediums

Every medium in this lane is ruled (rulings 93 to 103): envelopes-a-14, envelopes-a-15, envelopes-a-17, envelopes-a-9, envelopes-b-21, envelopes-b-7, testing-diff-gen-14, testing-oracles-3, tests-other-10, tests-other-11, tests-other-16, tests-other-30. The decisions stand beside each entry under Members.

## Roster summary

4 ruled (1 medium, 3 low); 12 medium ruled (93 to 103); 57 roster members approved (ruling 104) (28 low, 29 nit).

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

Every P6 lane runs after the P1 to P5 and P7 lanes that touch its files have landed on main, or rebases onto them before its final gate run; the coordinator names the base SHA at launch. Lows and nits inside this lane's approved roster are swept without a question to Finch; every high and medium has an individual ruling. A change that would alter a rendered `before` doc panel is a stop (ruling 89). Follows `p1-harness` (it runs alone against `tests/meter.rs` first), `p1-suites`, `p2-rows`, `p4-rosters`, `p4-ghosts` (the header rewrite), and `p5-scanners`. Owns `src/testing/**`, `tests/**`.

## Members

### tests-other-13 (medium, verification): ruling 91

`doc_hidden.rs` pins a per-file count it calls "by name", misses every non-literal spelling, and is dissolvable by a rustdoc flag the pinned nightly accepts

- Owner-gated: yes: the recommended path retires an instrument and edits the gate's `surface-json` recipe

Resolution: Preferred: append `--document-hidden-items` to the rustdoc invocation at justfile:937, let surfacecheck reach `PartyLiteral` and record its exception with the sealed-trait rationale now at doc_hidden.rs:18-20, update extract.rs:31-32, and delete this file once `just surface-totality` fails on an un-excepted hidden item (the replacement demonstrating it catches what the instrument caught). Fallback: pin the declaration line after each attribute, as foreign_reexport.rs's `(file, line-content)` roster does, and match attribute syntax (`#!?\[(cfg_attr\([^\]]*?)?doc\([^)]*\bhidden\b`) instead of one literal. Acceptance: `#[cfg_attr(not(test), doc(hidden))] pub fn escape()` on any pub item reads red somewhere in the gate; relocating an existing attribute to another item reads red; HEAD passes. Construction: Remove `#[doc(hidden)]` from `fn into_id_bits` (party.rs:818) and add it to any other public item in party.rs: the count stays 2 and `doc_hidden_occurrences_match_the_committed_roster` passes. Separately, add `#[cfg_attr(all(), doc(hidden))] pub fn escape(&self) {}` to `Party`: rustdoc omits it, the count finds no new occurrence, and the item is invisible to all three checks.

Ruled (91): append `--document-hidden-items` to the surface-json recipe; surfacecheck reaches `PartyLiteral` and records its exception with the sealed-trait rationale; delete `tests/doc_hidden.rs` only after `just surface-totality` demonstrably fails on an un-excepted hidden item (quote the failure in the deletion commit). Coordinates with `p5-scanners`.

### testing-diff-gen-6 (low, simplification): ruling 92

The registration totality pin describes the lint's failure class, not its own; its source scan imposes a layout convention on the macro and duplicates `laws/tests.rs`

- Owner-gated: no (retiring the pin would be; the recommended step keeps it)

Resolution: Keep the pin and restate its doc to name what it alone catches (a declared group driven by a bespoke test but absent from the roster; the unreferenced case is the gate's `dead_code` error). Extract one shared `testing` helper `declared_statics(path, prefix) -> BTreeSet<String>` used by both this pin and `every_law_group_is_registered`. If the owner prefers retirement, first execute the construction below so the replacement demonstrably catches what the pin caught. Acceptance: one scan body in the crate; the pin's doc names the ad-hoc-driven case; the layout comment at diff_ops.rs:302-305 either stays with the shared helper cited or goes with the scan.

Approved roster (92, decision 72): lands per the quoted Resolution inside this lane.

### tests-other-2 (low, verification): ruling 92

The worst-map smoke pin transcribes the currency axis as four string literals and `rows == 4`

- Owner-gated: yes: the fix re-exports a private board constant under the `meter` feature

Resolution: Make `worst::MAP_CURRENCIES` `pub` and re-export it from `meter::board` beside `NEAR_TIE_RATIO`; build the accepted label set from `MAP_CURRENCIES.iter().map(Currency::label)` and assert `rows == MAP_CURRENCIES.len()`; drop the parenthetical list at line 169. Acceptance: no currency label literal and no literal `4` remain in `worst_map_covers_every_operation_row`; temporarily adding `Currency::Segments` to `MAP_CURRENCIES` fails the test and restoring passes.

Approved roster (92, decision 72): lands per the quoted Resolution inside this lane.

### tests-other-28 (low, claims): ruling 92

"At the smallest committed-valid knobs" is a claim nothing pins

- Owner-gated: yes: a knob-floor accessor on `Shape` is a registry API addition

Resolution: Either add a `smallest()` knob-floor accessor per `Shape` in the registry and derive the pool from it (owner call), or soften the claim to what is checkable ("at small knobs, each operand tens to hundreds of packed bytes") and, where a knob is at a constructor's precondition floor, say so in the arm's comment. Acceptance: the module doc and `matrix_operands`'s doc claim only what a test or the registry pins.

Approved roster (92, decision 72): lands per the quoted Resolution inside this lane.

## Mediums ruled 93 to 103

Each medium below now carries its ruling and any amendment beside its quoted Resolution; land per the ruling.

### envelopes-a-14 (medium, verification): ruling 94

`Rank::cmp`'s documented O(1) class-first leg is priced only jointly with `checked_sub` and `+`

- Owner-gated: no

Resolution: Split the row: a `RANK_PAIR_CMP` envelope whose body is `a.cmp(&b)` alone, with an absolute limb ceiling at the O(1) scale (the two `bits()` reads), plus the existing sub/add row; optionally a two-scale check (`RANK_PAIR_DEPTH` and 2×) asserting the cmp-only limb reading is identical at both depths, which pins the order rather than an envelope. Acceptance: replacing the class-first arm with an aligned shift-and-compare fails the cmp-only row; the sub/add row is unchanged.

Ruled (94): Split the row: a `RANK_PAIR_CMP` envelope pricing `a.cmp(&b)` alone with an absolute limb ceiling at the O(1) scale, plus the existing sub/add row, and the two-scale identity check on the cmp-only reading. See ../rulings.md.

### envelopes-a-15 (medium, simplification): ruling 94

Flatness helpers (`Run`, `assert_flat`, `assert_ceilings`, the 5/4 slack) are re-implemented per module and bypassed inside their own module

- Owner-gated: no

Resolution: Hoist one `Run` (with `deltas: Option<u64>`), one `assert_flat`, one `assert_ceilings`, and the slack constants into a file-level `#[cfg(feature = "limb-meter")] mod support` used by every band module; convert the tuple ceilings (`FREEZE_BAND_OVER_*`, `RANK_JUMP_*`, `DISTANCE_JUMP_PAIR_*`) to `[(u64, u64); 2]` and route the three inline loops through `assert_ceilings`; return a `Run` from `ledger_wide_arming::run`. Acceptance: one `const SLACK_NUM`, one `fn assert_flat`, one `struct Run` in the file; no inline `* 4 <= ... * 5` ratio remains.

Ruled (94): Dup: carried by ruling 4's harness unification in `p1-harness`; do not re-do the hoisting here. Nothing to land in this lane. See ../rulings.md.

### envelopes-a-17 (medium, verification): ruling 94

The "one-touch-per-operand-byte liveness floor" is undeclared and not universal; the file's own control families fall under it

- Owner-gated: no

Resolution: Replace `touches >= bytes` at the nine sites with a floor derived per family from its nonzero stored delta count (each generator's layout doc yields the count, as the weight-comb and freeze-parade runs already do; the in-tree precedent is fe39fca3/248d5539), passing the count in from the generator; or, if one shared floor is preferred, state the class premise once (dense topology, narrow codes) and restrict the helper to families that satisfy it by construction. Acceptance: every `touches >=` floor in `skyline_flatness` and `ledger_wide_arming` names the irreducible-work premise it rests on at the assertion site; `grep -n 'touches >= run.bytes\|touches >= bytes' crates/before/tests/meter.rs` returns nothing.

Ruled (94): Per-family derived floors: every `touches >= bytes` site is replaced by a floor derived from the family's nonzero stored delta count, the premise stated at the assertion; the shared-floor alternative is struck. See ../rulings.md.

### envelopes-a-9 (medium, verification): ruling 94

The public dense rows assert nothing about their result and pass a no-op body

- Owner-gated: no

Resolution: Add value legs from the generator's closed forms: `assert_eq!(r, Some(Ordering::Greater))` for `cmp_dense`, `cmp_bigroot`, `cmp_cliff` (each dominates the empty version); `assert_eq!(joined, expected)` for the join rows where `expected` is built outside the window through the same public operator or, for `join_dense`, the derivation that `S(d)` is 0 everywhere except one leaf at 1, so its pointwise max with the flat 1 is the flat 1 by canonical uniqueness; for the rank rows, the closed-form rank where the public API expresses it, or at least `r == v.rank()` computed outside the window. Acceptance: replacing the metered body of `cmp_dense` with `None` or of `join_dense` with `v.clone()` fails the test on its value leg.

Ruled (94): Add the value legs from the generators' closed forms as the Resolution states. See ../rulings.md.

### envelopes-b-21 (medium, verification): ruling 95

`memo_resolution_cost::assert_flat` pins a raw ×2.5 class signature only: six of seven tests have no absolute ceiling, and the doubling the ratio divides by is unchecked

- Owner-gated: no

Resolution: give each of the six ratio-only tests an absolute touch ceiling (measured ×1.25, rounded up) and a ×0.75 tripwire at its larger run, following `MEMO_FANOUT_TOUCH_CEILING`/`MEMO_FANOUT_TOUCH_TRIPWIRE`; change `assert_flat` (and the inline reveal/ascend growth checks) to the per-byte form `large.touches * small.input * 4 <= small.touches * large.input * 5`, or assert the input ratio it assumes. Acceptance: each test in the module asserts an absolute ceiling at its larger run; every growth band in `memo_resolution_cost` and `width_circulation_cost` divides by `input` or pins the input ratio; a constructed ×2 inflation of the resolution's per-link touches trips at least one ceiling while leaving the ratios green. Construction: in the frame ledger's site resolution, fold each ledger link into the raise decision twice. `memo_chain_distinct`'s touches roughly double at both scales (ratio about ×2.0, under ×2.5); the `input / 8` floor is trivially satisfied; the per-byte controls are unchanged. All six pass; only `memo_fanout`'s absolute ceiling (73,402) can trip. For the denominator: alter `Shape::MemoChain` so the large run's packed input grows ×1.6 while touches grow ×2.4; the raw band passes (2.4 ≤ 2.5) although per-byte touches rose ×1.5.

Ruled (95): Each of the six ratio-only bands gets an absolute touch ceiling at its larger run and the per-byte ratio form. Amendment (ruling 88): ceilings only; add no ×0.75 tripwire floor. See ../rulings.md.

### envelopes-b-7 (medium, verification): ruling 95

`accum_fan_touches_flat` is byte-identical to the comb test; the fan stream its doc prices is never constructed

- Owner-gated: no

Resolution: either write a `fan_run(k, n)` that drives the accumulator with the fan's own stream (`add_wide(2^k - 1)` once, then per tooth the enter `+1`/sign/leave `-1`/sign crossings of the `2^k` boundary from the root magnitude, its own denominator), keeping the shared ceiling as the measured claim that the streams cost the same; or delete `accum_fan_touches_flat` and move the one-sentence remark at 6711-6713 into the comb test's doc as the reason no fan row exists. Acceptance: no two `#[test]` fns in `accum_streams` share a body; if a fan row remains, its MEASURED line reports a fan-specific stream and the test fails when its per-tooth deltas are made to widen with k. Construction: break the fan generator's accumulator stream (make `cliff_fan` emit a sequence on which the accumulator is quadratic): `accum_fan_touches_flat` stays green because it never runs that stream.

Ruled (95): Delete `accum_fan_touches_flat`; move the one-sentence remark into the comb test's doc as the reason no fan row exists (the cliff-fan family retires under ruling 79). The `fan_run` alternative is struck. See ../rulings.md.

### testing-diff-gen-14 (medium, verification): ruling 99

The generator census pins classes, not arms: three `arb_base` arms have no discriminating class, and the doc claims per-arm detection

- Owner-gated: no for adding classes; yes for changing how floors are derived (the "a quarter to a half of measured" design is stated at tests.rs:89-93)

Resolution: Add classes with a unique feeding arm: `near_max` (`u64::MAX - 4 <= value <= u64::MAX`), `power_plus_one` (`value - 1` a power of two with exponent in `6..96`, disjoint from the dense arm), and `machine_word` (`6 <= value <= u64::MAX - 5`), each with a floor and a comment naming the arm it witnesses; re-state the generators.rs sentence to name exactly the classes the census pins. Owner-gated alternative: derive each floor from the arm's `prop_oneof` weight (weight `w` of 13 per base, at least one base per tree, pinned at half the expectation) and state the derivation beside the number. Acceptance: narrowing arm 5 to `0u32..64` or deleting arm 3 makes the census fail; each floor's comment names the arm it derives from.

Ruled (99): Add the unique-arm classes (`near_max`, `power_plus_one`, `machine_word`) and restate the generators.rs sentence. Amendment: each floor is derived from its arm's `prop_oneof` weight with the derivation stated beside it (the owner-gated alternative), never a measured count. See ../rulings.md.

### testing-oracles-3 (medium, simplification): ruling 99

Bridge id walks recurse bare while the version walks route through `descend!`; the literal depth `0` probes on every level

- Owner-gated: no

Resolution: Thread a `depth: usize` through all four walks and route every recursive call through `descend!(depth + 1, …)` as grow/tests.rs and meter/tests.rs do; if the id side is instead meant to be exempt, state its bound at `emit_id`/`read_id` (the oracle envelope plus `ORACLE_SCALE_MAX`) and correct recurse.rs:11-14 and AGENTS.md:32-36 to say which bridge walks are guarded. Acceptance: `grep -n 'descend!(0' crates/before/src/testing/bridge.rs` is empty and every recursive call in bridge.rs is inside `descend!(depth + 1, …)`; or the exemption is stated at the site and recurse.rs and AGENTS.md describe the bridge accurately.

Ruled (99): Thread a real depth through all four bridge walks and route every recursive call through `descend!(depth + 1, ...)`; the exemption alternative is struck. See ../rulings.md.

### tests-other-10 (medium, verification): ruling 99

`coincident_span.rs` pins two of the four clone-identity rungs; the precedence and `contains`-receiver rungs are unpinned anywhere

- Owner-gated: no

Resolution: Add `coincident_precedence_collapses_to_one_containment` mirroring the dominance test (`fast == collapsed` against the single containment the rung documents, `assert_ne!(walked, collapsed)` for the distinct-buffer leg since early exit varies by direction) and `coincident_receiver_and_argument_collapse_to_byte_equality` (`Span::at(&v).contains(&v.clone())` reads strictly fewer scan bits than `Span::new(&v, &redecoded).contains(&v)`; whether `codec::canonical_eq` is unmetered, so the reading is exactly 0, is not settled here, and any strict inequality suffices). Reword the module doc to enumerate the rungs it holds. Acceptance: with `if false {` at span.rs:380 only, the new precedence test reads red and everything else green; the same at span.rs:459 for the receiver test; both green at HEAD. Construction: Mutate span.rs:380 to `if false {`. `Span::precedence` on a coincident span takes the fused walk and returns the same verdict; verdict_matrix.rs:1017 compares verdicts only; no scan-parity test names `precedence` on a coincident span; the gate is green with the rung deleted.

Ruled (99): Add the two rung pins (precedence; contains-receiver) mirroring the dominance test and enumerate the rungs in the module doc. See ../rulings.md.

### tests-other-11 (medium, documentation): ruling 101

The dominance test's doc says "read strictly more"; its body asserts only inequality

- Owner-gated: no

Resolution: Re-state the test doc: "coincident endpoints in distinct buffers take the fused walk and read a different scan count (its early exit may read fewer, so only divergence is pinned)". Scope the module doc's "strictly more" to `place` and the `contains` argument rung, naming dominance as the divergence-only leg. Acceptance: every doc sentence in the file matches the assertion form beneath it.

Ruled (101): Restate the test doc to divergence-only with the reason; scope the module doc's "strictly more" to the legs where it holds; name dominance as the divergence-only leg. See ../rulings.md.

### tests-other-16 (medium, verification): ruling 99

The foreign re-export pin is a substring scan that `pub use <dep>;` and import-then-alias evade, over a manifest parse that reads only a flat `[dependencies]` table

- Owner-gated: no

Resolution: In surfacecheck's `walk_use`, when `index` misses and `paths` hits, record `(prefix::name, paths[id].path, crate_id)` as a foreign re-export row, do the same for `TypeAlias` items whose target resolves to a foreign id and for `ExternCrate` items, and reconcile against a committed empty census with the existing exception discipline; then delete this file (the census test is the replacement demonstration Principle 3 requires). If the text pin is kept meanwhile: match the dependency name at a word boundary on `pub use`/`pub type` lines, carry a `pub use ... {` group across lines to its `}`, and accept `[dependencies.<name>]` headers or read the manifest with a TOML parser; reword line 16 to what the scan closes. Acceptance: `pub use bytes;` and `use bytes::Bytes; pub type Blob = Bytes;` at the crate root each read red in the gate; the manifest parse finds the same dependency set when one entry is rewritten in table form. Construction: Add `pub use bytes;` (or `use bytes::Bytes;` and `pub type Blob = Bytes;`) to crates/before/src/lib.rs. `dependency_reexports_match_the_committed_roster` passes (no line contains `bytes::` or `::bytes`), `cargo fmt --check` is unaffected, and `just surface-totality` passes because `walk_use` returns on the foreign id.

Ruled (99): surfacecheck records foreign re-exports, foreign-target type aliases, and extern crates as census rows reconciled against a committed empty set; `tests/foreign_reexport.rs` is deleted once the census demonstrably catches `pub use bytes;`. The hardened-text-scan alternative is struck. This work lands in the surface lane (`p6-surface`, ruling 98's regime); this lane only deletes the file after the census fires. See ../rulings.md.

### tests-other-30 (medium, verification): ruling 100

The polarity-flipped twin is pinned per axis, not per strict-order leg, so a leg can be neutralized with every test green

- Owner-gated: no

Resolution: Add `const LEGS: &[(Axis, &str)]` rostering every leg name `check` can flag; assert the polarity twin fires on each strict-order leg by name (as 1408-1416 does for the equality twin); assert the union of the two twins' fired legs plus the three documented unreachable legs equals `LEGS`; have `flag` reject a leg not in the roster so a renamed leg cannot escape. Acceptance: replacing the condition at line 1139 with `if false` reads red in `polarity_flipped_sweep_reads_inverted_through_the_matrix`; HEAD is green. Construction: Change line 1139 to `if false {`. Production: no violation and the census insert at 1134 is unconditional, so it passes. Equality twin: `dominance` is not in its leg list and the ranked axis stays quiet, so it passes. Polarity twin: `Axis::Placement` still fires through `placement`, `precedence`, `containment`, and the coincident legs, so `axis_count > 0` holds and it passes.

Ruled (100): Roster every leg `check` can flag as typed data (ruling 43), assert the polarity twin per strict-order leg, assert the union of fired legs equals the roster, and have `flag` refuse an unrostered leg. See ../rulings.md.

## Roster members approved (ruling 104)

Lows and nits approved as this lane's roster by ruling 104. Land each per its quoted Resolution and Acceptance, swept with the ruled members; report rather than choose if a Resolution conflicts with a ruling or offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89).

### envelopes-a-12 (low, verification): roster: approved (ruling 104)

`heap_meter_floor_on_decode_dense` measures the construction-language transcode, not a decode, on a premise that no longer holds

- Owner-gated: no

Resolution: Build `let wire = version_of(&p).encode();` before the reset and meter `Version::decode(&wire[..])` with the floor `peak >= wire.len()` (the read buffer that becomes the storage, version.rs:1111-1126); rename and re-doc to match; or delete the test as redundant with the 1 MiB canary. Acceptance: the canary's body is the operation its name and doc describe, and its floor is derived from that operation's own allocation.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### envelopes-a-21 (low, verification): roster: approved (ruling 104)

The quadruple's delta-floor comment miscounts the sparse comb: `n + 1` leaves stated, `3n/2 + 1` built

- Owner-gated: no

Resolution: Set `deltas: 7 * scale as u64 / 2` (the generator asserts `scale` even) and reword: the sparse comb's `n/2` plain leaves and `n` tooth leaves put `3n/2` delta codes behind its first; the full comb adds `2n`. Acceptance: the comment's arithmetic matches `sparse_cliff_comb`'s per-level emission and the constant follows from it.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### envelopes-b-1 (low, verification): roster: approved (ruling 104)

The hoisted-window densify band admits ×1.25 growth its own doc says must be zero

- Owner-gated: no

Resolution: replace the ×1.25 band with `assert_eq!(small_densify, large_densify, ...)`, keeping the floor and the two absolute ceilings; if the readings are ever not equal, the doc must say why a bounded difference is admissible and pin that difference. Acceptance: the band asserts equality of the two densify readings and passes on the committed readings. Construction: model a densification that sizes each image `span + position/2048` digits: at tail 10,240 the position term adds about 5 digits and at 20,480 about 10 over the ~84-digit span-priced base (ratio about 1.06), which passes the ×1.25 band and every ceiling while violating the stated invariant; an equality pin fails it.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### envelopes-b-15 (low, verification): roster: approved (ruling 104)

Three pair rows discard their result without a value leg

- Owner-gated: no

Resolution: add the halves-sum leg to both lag rows and a rank closed form (or at least `consumed(r)`) to the concurrent rank row; borrowing closures (`|| a.lag(&b)`) remove the tuple-return shape copied from the distance row. Acceptance: every `query_metered` call in the range either asserts on its returned value or wraps it in `consumed`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### envelopes-b-16 (low, simplification): roster: approved (ruling 104)

The masked-hole depth band re-pins the envelope row's touch columns as separate constants

- Owner-gated: no

Resolution: have `masked_cmp_hole_depth_band` read `query_env::MASKED_CMP_HOLE.touches` and `.touch_floor` and delete the two constants; or reduce the band to `lo == hi` plus the row's ceiling applied to `lo`. Acceptance: `grep -c 'MASK_HOLE_TOUCH_' crates/before/tests/meter.rs` reads 0; a change to the row's touch column alone re-pins the band.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### envelopes-b-5 (low, simplification): roster: approved (ruling 104)

`id_walk_scan_cost`'s flatness and floor asserts are implied by its exact-equality pins, and its doc hand-maintains derivable byte counts

- Owner-gated: no

Resolution: keep the equality and the MEASURED line; delete `assert_flat` and the `bits >= bytes` floor from this module (or state the linearity of the two constants as a `const _: () = assert!(...)` with the byte counts derived from `ID_DEPTH`); replace the literal byte counts in the doc with the derivation. Acceptance: one runtime assertion per depth pair (the exact equality) and no literal byte count in the module's prose.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### envelopes-b-6 (low, documentation): roster: approved (ruling 104)

The fork row's docs contradict each other on whether a cost record exists, and name no instrument for the split's spine walk

- Owner-gated: no

Resolution: restate 6416-6419 positively ("Fork's cost has no counter of its own here: the halves materialize (heap), the spine walk is iterative (zero segments), the writes are raw (the scan pin above); the walk's linearity is priced by the fuzz-fit `ff_party_fork` band and the board's `party_fork` cell"). Acceptance: the header and the test doc agree, and the row names the instrument that prices the walk.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### recursion-2 (low, documentation): roster: approved (ruling 104)

Bridge id walks are unguarded; ev walks pass depth 0 to descend!, defeating its amortization

- Owner-gated: no

Resolution: Pick one policy and state it at bridge.rs: either thread a depth through all four walks and call `descend!(depth + 1, ...)` at each site (`grow/tests.rs:125-180` is the in-crate pattern), or document that the bridge is deliberately unguarded because the oracle's derived `Drop` bounds every input it can meet, and drop the two ev-side `descend!` pairs that argument makes decorative. Update recurse.rs:9-14 and AGENTS.md:32-36 to match. Acceptance: bridge.rs's four recursive walks share one documented guard policy, and recurse.rs and AGENTS.md describe it accurately.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-diff-gen-11 (low, simplification): roster: approved (ruling 104)

`shape_version` and `shape_version_wide` duplicate the spine loop; three `unreachable!("handled above")` arms; `debug_assert!` in test-only code

- Owner-gated: no

Resolution: `fn shape_version_with(shape, scale, leaf_base: &impl Fn(u64) -> Base)` with `shape_version` as the identity instance and `shape_version_wide` choosing the wide index up front; select the lean with one closure returned from a single match on `shape` so Bushy is handled once (the party twin at 244-261 can share it). Replace `debug_assert!` with `assert!`. Acceptance: one spine loop for versions; no `unreachable!("handled above")` remains; `scale == 0` panics for the wide builder.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-diff-gen-12 (low, verification): roster: approved (ruling 104)

`shape_version_wide`'s distinctness claim fails at `wide == 1`: the raised leaf collapses against its sibling

- Owner-gated: no

Resolution: Raise the chosen leaf by `wide + scale + 1` (plus its counter), which exceeds every other base for any `wide >= 0`, and pin the claim: assert `ev_depth(&to_oracle_version(&result))` equals the depth `shape_version(shape, scale)` produces before returning. Acceptance: a proptest over `arb_shape() × 1..=64 × arb_base() × bool` asserts the depth identity.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-diff-gen-15 (low, verification): roster: approved (ruling 104)

`op_strategy` draws indices from an unnamed `0..8`, so members beyond the eighth never act; the strategy is undocumented and `Sync`'s doc omits the self-pair no-op

- Owner-gated: no

Resolution: Name the cap (`const ACTOR_INDICES: Range<usize> = 0..8`) with its rationale, or draw from `0..MAX_TRACE_OPS` so any live member can act; document `op_strategy`; append "a self-pair is a no-op" to `Sync`'s doc. Acceptance: the module doc states which members can be actors; the index range is a named constant; `Sync`'s doc matches both appliers.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-diff-gen-18 (low, verification): roster: approved (ruling 104)

`comb`'s doc says its closed-form sizes are pinned by this module's tests; the node count is asserted nowhere and the bit formula at one point only

- Owner-gated: no

Resolution: In `alternating_combs_hold_the_envelope` (or in `comb` itself) assert `sample.tier2.nodes == 4 * pairs as u64 - 1` and `bits.len() == (pairs * pair_bits - 2) as u64` for every sampled parameter pair; drop "today" from the sentence. Acceptance: both closed forms are asserted on every `arb_comb_params` case; the doc names the test that pins them.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-diff-gen-2 (low, simplification): roster: approved (ruling 104)

Row tuples cross the shape_rows/diff_ops boundary unnamed and in inconsistent field order

- Owner-gated: no

Resolution: In `shape_rows.rs` define `PlateauRows`, `RegionRows`, `CellRows`, `OverlayRows` as newtypes over the vectors with one field order and `heights()`/`owned()` projections on `OverlayRows`; add `oracle_overlay(&oracle::Clock) -> OverlayRows` so the `clock_shape` tree spelling becomes one call. Acceptance: no bare row tuple type in `diff_ops.rs`; each of `clock_shape_matches_the_oracle`'s three spellings is one expression.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-diff-gen-20 (low, simplification): roster: approved (ruling 104)

`decode` in compactness/tests.rs renames a transcoding call as decoding

- Owner-gated: no

Resolution: Call `.version()` at the three sites and delete the wrapper; move the `Shape` import into the crate-imports group. Acceptance: no `fn decode` in compactness/tests.rs.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-diff-gen-29 (low, verification): roster: approved (ruling 104)

`balanced_terms` replicates `mul_into`'s recentering with nothing tying the two, and names a kernel that never compacts the parked factor in production

- Owner-gated: no

Resolution: Either factor the compaction out of `mul_into` into a `pub(crate)` balanced-digit iterator both `mul_into` and this pin consume (then the pin measures a production kernel by construction), or restate the pin: `balanced_terms` is the test's own base-2^32 balanced-digit spelling, a reference incompressibility measure, with `DIGIT_BITS` and `HALF_DIGIT` named and the "settle's own" wording dropped; in either case name which production path the instance settles through (`charge_digits` over the mass digits with the plunge as the whole factor). Acceptance: either `balanced_terms` calls a production function, or its doc no longer claims to be the settle's own compaction and its constants are named.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-diff-gen-3 (low, documentation): roster: approved (ruling 104)

`party_shape` descriptor's doc claims the anonymous id is in its population; no driver feeds it

- Owner-gated: no

Resolution: Drop the sentence; or, if the anonymous walk is meant to be covered, register a `party_shape` spelling in a group whose driver admits the anonymous id and say the value is crate-internal. Acceptance: the descriptor's doc describes only inputs its drivers produce.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-diff-gen-32 (low, verification): roster: approved (ruling 104)

The island include scan cannot see the seven macro-form include sites its own doc credits

- Owner-gated: no

Resolution: Teach the scan the macro form (for each `*_matrix! {` invocation, collect its first string-literal argument as an included island), or route every island include through one `island!("name")` macro whose spelling the scanner recognizes; at minimum state beside the charset filter that macro sites are invisible and that the matrices' islands are held by their method-doc twins, and correct the sentence at 19-21. Acceptance: removing the literal include at version.rs:505 while leaving `binop_matrix!` attached keeps the test green (the scan sees the macro), or the doc says plainly that it will not.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-diff-gen-7 (low, simplification): roster: approved (ruling 104)

The organic driver draws a third version (`k`, `vc`) that no drive arm reads

- Owner-gated: no

Resolution: Drop `k` and `vc`, make `v: [&'a oracle::Version; 2]`, and fix the field doc; or, if a three-version group was intended (a transitivity or `span_all` descriptor), add it so the pick is read. Acceptance: every field of `Organic` is read by at least one `organic_drive!` arm.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-diff-gen-9 (low, verification): roster: approved (ruling 104)

The conviction witnesses reach 2 of 11 `Matches` and 1 of 9 `FsMatches` comparisons; the two non-trivial party comparisons have no known-bad

- Owner-gated: no

Resolution: Add one known-bad `(disjoint_party, disjoint_party)` group whose production leg forgets the join (`prod: a`, `tree: { a.join(b)...; a }`) and whose fs leg lifts only `a`, with the existing two-direction pattern; state in the conviction tests' doc which comparison impls the witnesses reach. Acceptance: replacing the body of `Matches<oracle::Party> for Party` or `FsMatches<oracle::Party> for Id` with `true` makes a conviction test fail.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-oracles-10 (low, simplification): roster: approved (ruling 104)

Test-side grid caps at `GRID_N` cannot bind, contradict `fs_grid`'s stated policy, and are guarded by constant-only asserts

- Owner-gated: no

Resolution: Delete `grid_for` and call `fs_grid` at its six sites; replace the keystone's `.map_or(0, |d| d.min(GRID_N))` with `fs_grid` or an `assert!(d <= GRID_N, …)` and reword the comment at 185-189 to say `fork`'s assertion is what keeps the probed grid inside the ceiling; use `fs_grid(&[id_depth(&p) + 1])` at 288 (see testing-oracles-16 for the rate); turn the 606-609 assert into a `const _: () = assert!(...)` (its purpose, guarding a future redefinition of `GRID_N`, survives) and delete 614; restate the `GRID_N` doc at 62-64 in terms of the assert. Acceptance: `grep -n 'min(GRID_N)\|fn grid_for' crates/before/src/testing/semantic_oracle/tests.rs` is empty; the suite is green.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-oracles-16 (low, documentation): roster: approved (ruling 104)

Two test docs say the random `fork` refines up to two levels per call; the code, the module doc, and the equality pin say one

- Owner-gated: no

Resolution: Restate both as one level (children carry a ceiling at most one level below the id's resolution); at 287-288 use `fs_grid(&[id_depth(&p) + 1])` with the one-level reason; if testing-oracles-17 rewrites the sweep's doc, the 533-536 paragraph is replaced there. Acceptance: no "two levels" or "≤ 2 levels" remains in the file; the three statements of the rate agree; `fork_partitions` and the chain pin stay green.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-oracles-20 (low, simplification): roster: approved (ruling 104)

Hand-rolled injective dedup keys where the oracle types already derive `Hash + Eq`

- Owner-gated: no

Resolution: Replace `seen` with `HashSet<oracle::Party>`/`HashSet<oracle::Version>` (`seen.insert(t.clone())`), delete `id_key` and `ev_key`, and rewrite the comment to state only the every-level dedup rationale. Acceptance: `id_key`/`ev_key` are gone; `corpus_counts_are_exact` still reports `2^(2^d)` ids and 691 events with full denotation distinctness.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-oracles-23 (low, simplification): roster: approved (ruling 104)

`corpus_is_canonical`'s `> 20` non-triviality floors are superseded by the exact-count pin that names them inadequate

- Owner-gated: no

Resolution: Delete lines 384-395 and the parenthetical at 551-552; keep `corpus_is_canonical` as the normality sweep and reword its doc accordingly. Acceptance: `corpus_is_canonical` asserts only `is_normal`; no `> 20` literal remains; a deliberate enumeration shrink (drop `P::Leaf(true)` from the seed loop) still fails `corpus_counts_are_exact`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-oracles-27 (low, simplification): roster: approved (ruling 104)

The organic population replays the trace twice and takes versions and parties from the oracle through the bridge, although the impl replay already holds them

- Owner-gated: no

Resolution: Replay once on the impl; derive `v` from `imp[..].version().clone()`, `p` from `imp[..].party().dangerously_alias()`, and the lists likewise; drop `run(&ops)` and the `ver`/`party` calls in this test. Acceptance: the organic test imports neither `run` nor the bridge; every law group still drives; the committed seeds in proptest-regressions/testing/algebraic_laws/tests.txt replay green.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tests-other-21 (low, verification): roster: approved (ruling 104)

The "wide" tail checks test display length, not magnitude

- Owner-gated: no

Resolution: Require a bare digit run (`text.bytes().all(|b| b.is_ascii_digit())`) with the length bound, or parse the display as `Ticks`/`UBig` and assert `> u64::MAX`, at both sites. Acceptance: substituting a nested narrow version for `wide_leaf` in the derivation reads red on `saw_wide`. Construction: In fuzz_seed_set.rs:359-361 replace the 2^128 literal with `"(1, (2, 3), (4, (5, 6)))"`; regenerate; both `saw_wide` flags still set.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tests-other-29 (low, simplification): roster: approved (ruling 104)

`check` is a 490-line body with a closure-as-method and a free `intern` over three `&mut` fields

- Owner-gated: no

Resolution: `impl Outcome { fn flag(&mut self, axis, leg, detail) }`; a `PoolBuilder { versions, stable, index }` with `fn intern(&mut self, v, ordinal) -> usize`; split `check` into per-axis helpers plus `check_span_grid` and `check_conjunction_grid`, each carrying its axis's leg names; `dominance_label`/`precedence_label` for symmetry. Do it together with tests-other-30's leg roster. Acceptance: no closure takes `&mut Outcome`; `check` is a dispatcher; the twins and the production run pass unchanged with identical leg names.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tests-other-5 (low, documentation): roster: approved (ruling 104)

Generator and support docs disagree with their code in three small places

- Owner-gated: no

Resolution: answer_embedded.rs:9-11 and 68-69: "then the even-indexed of `n` forked leaves tick once" (or tick every leaf and re-derive the grid). fuzz_seed_set.rs:31-35: describe the corpus by genre for all five targets, or name none. amp_board_smoke.rs:137-138: "a deterministic counter reset per shard, under one-scenario-per-process isolation". Acceptance: each doc sentence is true of the code beneath it.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tests-other-9 (low, simplification): roster: approved (ruling 104)

`wide_display_pair_expectations_are_split` is a strict consequence of the red-membership pin

- Owner-gated: no

Resolution: Delete `wide_display_pair_expectations_are_split` and fold its two-sentence rationale into `roster_red_membership_is_pinned`'s doc. (The verbatim `TEXT_CEILING_CELLS` pin at 103-116 is deliberate tamper-evidence per the module doc and stays; a semantic predicate over the cell IDs would be an optional addition.) Acceptance: three tests remain, each failable by an edit that leaves the others green.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### envelopes-a-13 (nit, simplification): roster: approved (ruling 104)

Mixed idioms for one purpose: `consumed` beside `black_box`, a one-line `version_of` alias, qualified `Ordering`, bare-bool selectors

Where: `crates/before/tests/meter.rs:410-424`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `black_box` everywhere; delete `consumed` and `version_of`; import `Ordering`; an enum over the bools

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### envelopes-a-7 (nit, simplification): roster: approved (ruling 104)

Presentation hygiene: half-aligned rustfmt::skip tables, em-dashes in comments and messages, a whitespace run, a terse floor message

Where: `crates/before/tests/meter.rs:261-263`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Align or drop the skip tables; colons in messages; collapse the whitespace

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### envelopes-b-11 (nit, simplification): roster: approved (ruling 104)

Em-dashes in assertion messages and line comments

Where: `crates/before/tests/meter.rs:6960-6963`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Colons or semicolons; one site once the harnesses unify

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### envelopes-b-13 (nit, simplification): roster: approved (ruling 104)

Qualified paths where imports exist or would serve

Where: `crates/before/tests/meter.rs:7155-7157`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): File-level imports for `UBig`, `Rank`, `Ticks`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### envelopes-b-14 (nit, simplification): roster: approved (ruling 104)

Small redundancies: an operand built twice, a fixture built twice, a byte assert that restates `Eq`, a string-length proxy, a one-constant module

Where: `crates/before/tests/meter.rs:7181-7185`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Build each generator and fixture once; name the depths; `TryFrom<&Ticks>` for the word test

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### envelopes-b-2 (nit, verification): roster: approved (ruling 104)

`parse_wide_arming` and `answer_embedded_product` restate the generators' structural strides (33, 66) as literals.

Where: `crates/before/tests/meter.rs:5581-5584`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Expose the strides on the meter surface and cite them.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### envelopes-b-23 (nit, simplification): roster: approved (ruling 104)

A dropped line continuation leaves fourteen spaces inside an assertion message

Where: `crates/before/tests/meter.rs:8580-8580`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Rewrap the literal with `\`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### envelopes-b-29 (nit, documentation): roster: approved (ruling 104)

`span_decode_shares_the_second_payload_decode`'s doc says the gap is "exactly" one check per leaf delta; the test asserts only `>=`

Where: `crates/before/tests/meter.rs:10436-10446`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): either assert the gap (count the second component's leaf deltas through the public shape iterators and `assert_eq!(fused ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### envelopes-b-30 (nit, documentation): roster: approved (ruling 104)

`distinct_buffers_keep_the_walked_paths_covered`'s doc lists operations the body does not exercise

Where: `crates/before/tests/meter.rs:10649-10651`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): drop "/distance/lag" or point at `metric_fast_paths_skip_the_fold` as the test that pins them

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### envelopes-b-9 (nit, documentation): roster: approved (ruling 104)

The "expansion rows" header sits above the hole and masked rows it does not describe

Where: `crates/before/tests/meter.rs:6851-6854`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): move the comment to precede `TICK_EXPAND_SPINE` at 6861 and give the hole/masked rows a one-line header of their own

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### prose-hygiene-14 (nit, documentation): roster: approved (ruling 104)

"improvement tripwire" names the benign trigger, not the failure the floor detects

Where: `crates/before/tests/meter.rs:208-210`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): rename the genre to "bypass floor" (the file's own phrase at

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### recursion-10 (nit, documentation): roster: approved (ruling 104)

Oracle prose says "boxed" trees and clones; the trees are Arc and clones are refcount bumps

Where: `crates/before/src/oracle.rs:16-18`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): oracle.rs:17-18 -> "the derived `Drop` of the `Arc`-linked trees"

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-diff-gen-1 (nit, simplification): roster: approved (ruling 104)

Eleven identity `Matches`/`FsMatches` impls could be two blanket impls

Where: `crates/before/src/testing/diff_ops.rs:110-159`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Two blanket impls, or state the closed-roster intent

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-diff-gen-19 (nit, simplification): roster: approved (ruling 104)

`comb` hand-emits the min-lifted stream and self-checks it; the oracle path every other generator uses gives the same bits by construction

Where: `crates/before/src/testing/compactness.rs:137-165`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Build the comb as an `oracle::Version`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-diff-gen-21 (nit, simplification): roster: approved (ruling 104)

Gamma code widths are pinned twice: the snapshot table and codec's `gamma_costs`

Where: `crates/before/src/testing/snapshots.rs:45-82`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Keep the snapshot; make `gamma_costs` a closed-form proptest, or retire one

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-diff-gen-25 (nit, simplification): roster: approved (ruling 104)

Idiom nits: qualified paths where the name is imported, a `Shape` name collision, and an undocumented block helper

Where: `crates/before/src/testing/asymptotics.rs:97-101`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Imports; rename `generators::Shape` to `DeepShape`; doc `version_block`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-diff-gen-30 (nit, simplification): roster: approved (ruling 104)

The stack-merge tiling loop is written three times in `shape_rows.rs`

Where: `crates/before/src/testing/shape_rows.rs:43-54`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One `from_rows` closer

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-diff-gen-8 (nit, simplification): roster: approved (ruling 104)

`check_version_pair` and `check_party_pair` are one generic function written twice

Where: `crates/before/src/testing/diff_ops/tests.rs:529-553`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One generic `check_pair<A, B>`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-oracles-13 (nit, simplification): roster: approved (ruling 104)

An empty "operation cross-checks" section banner survived the move of its tests to `diff_ops`

Where: `crates/before/src/testing/semantic_oracle/tests.rs:218-220`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Delete the banner

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-oracles-14 (nit, verification): roster: approved (ruling 104)

`order_is_a_partial_order`'s transitivity arm is conditional on `le(a,b) && le(b,c)` for three independent draws; nothing constructs the chain.

Where: `crates/before/src/testing/semantic_oracle/tests.rs:233-237`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Construct `b = join(a, x)`, `c = join(b, y)` and assert unconditionally.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-oracles-15 (nit, simplification): roster: approved (ruling 104)

The paper's worked-value fixture and its expected vector are spelled twice

Where: `crates/before/src/testing/semantic_oracle/tests.rs:484-504`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One `paper_worked_example()` fixture

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-oracles-18 (nit, documentation): roster: approved (ruling 104)

The event corpus is the normalization closure over `{0, 1, 2}`, not "normal-form trees with bases in `{0, 1, 2}`"

Where: `crates/before/src/testing/exhaustive.rs:6-9`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): State it as "the normal forms of every raw tree over the alphabet `{0, 1, 2}` (lifting and collapse carry bases up to `2(d + 1)`)" at both sites ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-oracles-21 (nit, documentation): roster: approved (ruling 104)

"At the bottom of this file" is no longer true, and "180 seconds" recomputes a number the nextest profile owns

Where: `crates/before/src/testing/exhaustive/tests.rs:49-51`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "in the intrinsic symmetry laws section of this file"; "the workspace's nextest profile terminates slow tests at its `slow-timeout` budget ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-oracles-26 (nit, documentation): roster: approved (ruling 104)

The organic drive pairs versions with foreign clocks' regions without saying whether that is the intended regime

Where: `crates/before/src/testing/algebraic_laws/tests.rs:358-369`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One comment above the arms (or on `Organic`) stating the regime, for example that versions meet foreign live regions here because the owner pairing is ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-oracles-5 (nit, simplification): roster: approved (ruling 104)

Idiom residue across the partition: qualified paths beside imports, a re-spelled helper, an index before its `expect`, a `debug_assert!` in test-only code, a `Result<(), ()>`

Where: `crates/before/src/testing/bridge.rs:83-89`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): The listed imports and spellings; `assert!` in `fs_grid`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-oracles-7 (nit, simplification): roster: approved (ruling 104)

Em-dashes in `//` comments (a crate-wide pattern; four sites in this partition)

Where: `crates/before/src/testing/bridge.rs:102-103`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Crate-wide sweep

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### testing-oracles-8 (nit, verification): roster: approved (ruling 104)

`to_oracle_party`/`to_oracle_version` discard the end position, so a stored stream with trailing live bits lowers to its prefix silently (byte-equality legs elsewhere mask this).

Where: `crates/before/src/testing/bridge.rs:174-188`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Bind the position and assert it equals `bits.len()`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tests-other-19 (nit, simplification): roster: approved (ruling 104)

Small idiom slips: a qualified path beside its import, a bare `unwrap` among expect-proofs, a display comparison where `is_seed` exists, a broken doc wrap

Where: `crates/before/tests/fuzz_seeds.rs:52`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Import `BTreeMap`; an expect-proof; `is_seed()`; reflow the doc

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### tests-other-25 (nit, simplification): roster: approved (ruling 104)

Dead `let _ = ...version();` lines that suppress nothing

Where: `crates/before/tests/support/fuzz_seed_set.rs:129-131`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Delete the dead lines

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

