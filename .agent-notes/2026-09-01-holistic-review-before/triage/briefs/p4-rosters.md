<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P4 lane: rosters, guards, scope, and hazard sections

## Goal

Every roster the compiler can hold, it holds; every runtime guard whose
property a committed differential test already samples is gone, with
no trace left in the code; the validation index that claimed totality
it did not have is gone; the traffic counters sit under the feature
their siblings use; the test suites sit where the crate's convention
puts them; and every fallible public function carries `# Errors` in one
form, held by `doclint`. Rulings 61, 62, 63, 64, 65, and 67 fix each
shape; this brief carries them, plus the verification- and
correctness-class lows and nits of P4 as a roster pending approval.

## Ground rules

These apply to every lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `10cdd255` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `10cdd255`, fast-forward; if it has diverged, stop and report. Never call
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

## Ordering

Ruling 65's relocation (version-core-27, rank-2, clock-26, suanpan-30)
moves files other lanes edit; it runs alone, as its own commit series,
with no other lane building against the same files, and the coordinator
sequences it. Ruling 61's roster work touches `src/meter/board/**` and
`registry.rs`, which `p1-board` and `p2-surface` (ruling 43) edit; run
after both have landed and rebase. Ruling 64's deletions are independent
and may land first. Ruling 67's doclint rule lands with its fixture
before the prose pass, so the pass reads red then green.

## Hazards and stops

- Ruling 64: no comment, marker, or `// was an assert` trace in code;
  the commit message carries the covering test's name.
- Ruling 63: nothing is made total; the index and its pins are deleted.
- The `meter` surface is instrument surface (ruling 8); a change there
  lands with the `rumors` test update in the same commit, and a change
  to any public signature is a stop.
- Relocation commits are pure moves (`git mv` plus `mod` lines), with
  the gate green at each commit; a test whose doc comment turns out to
  be wrong on the move is reported, not silently fixed.

## Members

### meter-core-7 (medium, simplification): ruling 61

Freeze-regime widths are hand-derived copies of `FREEZE_ALLOWANCE_DIGITS` with no binding test and no per-family freeze or promotion pin

Resolution: Widen `FREEZE_ALLOWANCE_DIGITS` to `pub(crate)`; in meter.rs define one `FREEZE_TRIP_BITS = 32 * (FREEZE_ALLOWANCE_DIGITS + 1)`, one `PROMOTION_TRIP_BITS = 32 * (FREEZE_ALLOWANCE_DIGITS + 1 + FREEZE_ALLOWANCE_DIGITS + 1)` (twenty digits), and one `DIGIT_ISOLATION_STRIDE = 33` with its derivation, and make the three 288s, the 608, and both 33s those names; have the tests reference the constants; add a unit test asserting the derivation predicates against `int_digits`/`base_digits`; add a `cfg(test)` promotion tap beside `FREEZE_HITS` and per-family pins for the "exactly one freeze", "one freeze per block", and "never promotes" claims. Acceptance: `grep -nE '\b(288|608|33)\b' crates/before/src/meter.rs crates/before/src/meter/tests.rs` shows only the constant definitions; the construction below fails a named test.

Ruled (61): yes, as the entry states, under ruling 43's direction (typed, compiler-held; a cleaner idiomatic shape may be adopted and is reported).

### meter-registry-tier2-11 (medium, verification): ruling 61

`FamilyId::ALL`, the 52-arm `index()`, and `ALL_SHAPES` are hand rosters checked only against each other; a variant absent from `ALL` reaches no instrument and `index()`'s doc claims the opposite

Resolution: (1) unconditional: fix or delete the sentence at 1170-1172, and reduce `index()` to `self as usize` (both enums are fieldless with declaration order equal to roster order, verified) so `roster_order_is_committed` pins `ALL[i] as usize == i` without a 52-arm match. (2) owner-gated: derive variant enumeration (`#[derive(strum::VariantArray)]` on `FamilyId` and `Shape`, so `FamilyId::VARIANTS` replaces `ALL`, `Shape::VARIANTS` replaces `ALL_SHAPES`, `index()` and its pin dissolve, and the "found by luck" list at 572-573 loses its first entry), or a local declaration macro producing the enum and its array from one list if a new dependency is unwanted. Acceptance: appending a variant to either enum without touching any roster either fails to compile or fails a committed test that enumerates variants totally; `grep -n 'fn index' crates/before/src/meter/registry.rs` finds nothing (or the surviving pin is a name snapshot); registry/tests.rs no longer carries the completeness caveat. Construction: add `FamilyId::Probe` after `LatentLadder` with `index() => 52`, a `spec()` arm copying LatentLadder's row with `name: "probe"` and `Bands::Unbanded`, and the or-pattern arms in board/family.rs:765-780 and board/ops.rs:164-182 extended; leave `ALL` untouched. `roster_order_is_committed`, `family_names_are_unique`, `every_shape_is_cited_by_a_family`, `board_roster_derives_from_coverage_answers`, `envelope_only_rulings_are_dated`, and `band_tests_and_registry_citations_stay_paired` all pass, and `FamilyId::board()` never yields the variant.

Ruled (61): yes, as the entry states, under ruling 43's direction (typed, compiler-held; a cleaner idiomatic shape may be adopted and is reported).

### board-families-floors-judge-6 (low, simplification): ruling 61

Board membership is declared in the registry and re-derived by a 19-variant `unreachable!` arm

Resolution: Carry the board sub-roster as its own type: `Coverage::Board { family: BoardFamily, cells }` with a `BoardFamily` enum, `FamilyId::board()` yielding `BoardFamily`, and `FamilyData::build(kind: BoardFamily, ..)` matching exhaustively; the shard and smoke-test destructurings dissolve with it. If the owner takes it, the base-size constant can ride the same declaration (see open questions). Acceptance: no `unreachable!` in the board module or its tests mentions `FamilyId::board()` or "envelope-only"; adding a Board variant fails to compile until it has a build arm.

Ruled (61): yes, as the entry states, under ruling 43's direction (typed, compiler-held; a cleaner idiomatic shape may be adopted and is reported).

### board-ops-render-2 (low, simplification): ruling 61

`designed()` is a shape-axis declaration living in the operation module, hand-mirroring the registry's envelope-only roster with no test binding the two

Resolution: (1) Add a unit test in tests.rs: for every `FamilyId::ALL` variant and every `OpGroup`, `catch_unwind(|| designed(kind, group)).is_err()` equals `matches!(kind.spec().coverage, Coverage::EnvelopeOnly { .. })`, so the arm and the registry cannot disagree in either direction. (2) Move `designed` (and `OpGroup`, or a re-export) into family.rs beside the bundle-build match, where the registry doc already says it lives; the envelope-only arm then sits beside its twin. Acceptance: flipping one envelope-only variant's registry coverage to `Board { .. }` fails `just test`; `grep -n 'fn designed' ops.rs` is empty and registry.rs:569-571 reads true.

Ruled (61): yes, as the entry states, under ruling 43's direction (typed, compiler-held; a cleaner idiomatic shape may be adopted and is reported).

### envelopes-a-19 (low, simplification): ruling 61

Closed forms, the tick-family fixture, and the `UBig`-to-`Ticks` round trip are spelled twice, and generator widths and file-level scales appear as literals

Resolution: Add `freeze_position_ticks(k)`, `promotion_rearm_ticks(p)`, a `tick_families()` fixture, and a `fn ticks(u: &UBig) -> before::Ticks` beside the existing closed-form helpers; reference `super::MASK_DRIFT_MAGNITUDE_BITS`, `super::MASK_DRIFT_TEETH`, `super::CLIFF_SCALE` in the band modules; name the small-run scales per band; name the word slack at 923/932; define the closed-form widths once in `skyline_flatness`; use `div_ceil(8)` for both terms at 2080. Acceptance: each closed form and the tick fixture have one definition; the decimal round trip appears once; no bare 512/1_024/2_048 where a file-level constant exists; no bare 288/608/33 in a closed form.

Ruled (61): yes, as the entry states, under ruling 43's direction (typed, compiler-held; a cleaner idiomatic shape may be adopted and is reported).

### meter-registry-tier2-8 (low, simplification): ruling 61

`FamilySpec.denominator` and `closed_form` are prose stored as data that nothing reads; `shapes` is checked in one direction only

Resolution: owner's call between consuming and demoting. Consume: render each column's denominator from the spec in the board header so the smoke suite can pin its presence, and make `closed_form` carry a test name the smoke suite's scan resolves like band names. Demote: move both into each variant's rustdoc (where the `FamilyId` docs already carry this altitude of prose) and shrink `FamilySpec` to the fields instruments consume (`name`, `shapes`, `coverage`, `bands`); make `AXIS_BANDS: &[&str]` with each disposition as a comment above its entry. For `shapes`, have board/family.rs's bundle build record the `Shape` variants it constructs and assert equality with `spec().shapes` in the smoke suite. Acceptance: every remaining `FamilySpec` field has a reader outside registry/tests.rs, or the fields are gone and the variant docs carry the sentences; a `shapes` row gaining a variant its bundle does not build fails a test.

Ruled (61): yes, as the entry states, under ruling 43's direction (typed, compiler-held; a cleaner idiomatic shape may be adopted and is reported).

### skyline-watermark-27 (low, simplification): ruling 62

The two watermark counters sit under different gates, and web_traffic misnames the gate of the idiom it cites

Resolution: owner's choice of direction. (a) Gate `web_traffic`'s `counter`, `use`, and `EmitTraffic`, plus `meter::emit_traffic`/`reset_emit_traffic` and the re-export at meter.rs:81, on `limb-meter` exactly as `pool_misses` is, and put `#[cfg(feature = "limb-meter")]` on the two `fill/tests.rs` witnesses' counter asserts (or on the tests). (b) Add to Cargo.toml's `meter` comment that per-decision liveness counters (`hull_traffic`, `web_traffic`) are admitted, and state why a per-emission bump in the tick kernel is acceptable in bench builds. In either case reword web_traffic.rs:19-20 (and hull_traffic.rs:16-17, outside this partition) to name the actual gate of the cited idiom. Acceptance: one stated policy the two watermark counters both satisfy; `grep -rn "codec::scan.*idiom"` finds no citation under a gate `codec::scan` does not use; under (a), `cargo build --features meter` compiles `emit_offset` with no `web_traffic` statics and `cargo test -p before --all-features` runs `dominated_undercut_cost` and both fill witnesses green.

Ruled (62): move under `scan-meter`; `rumors`' `meter` feature already enables it, so no `rumors` change is expected. If one turns out to be needed, it lands in the same commit (ruling 8).

### testing-oracles-28 (medium, claims): ruling 63

The validation index claims to map every instrument but has no row for at least eight committed instruments, misdescribes the exhaustive corpus as "reachable states", and gives the replay keystone no row

Resolution: Either add one row per omitted instrument at the same altitude (what it alone catches, where it lives, which recipe runs it), plus a row for the fs replay, or narrow the opening sentence to the classes the page covers and point at `just --list` and the justfile's recipe comments as the recipe-level inventory. Rewrite the exhaustive row as "every canonical normal-form id tree to `ID_SMALL_DEPTH` and event tree to `EV_SMALL_DEPTH`, every ordered pair, for the operations its check set names; kernel suites sweep further operations over the same corpus". Owner's call whether to pin the page mechanically (a test that every recipe under the gate/ci composition and every `tests/*.rs` binary is named in the index). Acceptance: every verification recipe the justfile's gate and ci compositions run for before, and every `crates/before/tests/*.rs` binary, is named in the index or the header states the page's scope and where the rest is indexed; the exhaustive row says "canonical normal-form trees"; `grep -n -i 'wasm32\|covcheck\|mutants\|surfacecheck\|verdict_matrix\|superlinear' validation_index.rs` finds each. Construction: The grep in the acceptance clause returns nothing at this commit; `ls crates/before/{fuzz,wasm32-pins,surfacecheck} tools/{covcheck,mutantcheck} .cargo/mutants.toml crates/before/tests/{verdict_matrix,superlinear_tripwires}.rs` all exist. For the exhaustive row: `all_normal_events(2)` contains trees with base patterns no tick sequence over a seed-derived party produces at that depth, and they are checked, so reachability is not the enumerated property.

Ruled (63): the index is deleted rather than made total.

### board-frame-26 (low, claims): ruling 63

The validation index calls the board's ceilings "class-scale", but the touch, κ, fold-scan, and family-stated constants are pinned at worst-reader ×1.25

Resolution: Owner's call. (a) Keep the ×1.25 convention and re-word the index: the touch, κ, fold-scan, and family-stated constant legs are envelope-class (×1.25 over the worst honest reader, re-pinned when the worst reader moves); the heap, limb, and scan globals and the exponent legs are class-scale. (b) Restore a class-scale touch ceiling and add `TouchEnvelope` rows to tests/meter.rs for the worst readers the pin commit names, so the board judges class and the envelopes judge constants as the index says. History favors (a): the convention is owner-ratified. Acceptance: the index sentence and ceilings.rs agree on which legs are class-scale; if (b), the re-pin and the new envelope rows land in one change and the board of record reads green.

Ruled (63): the validation index dissolves. Delete the index, its totality claim, and its rendering; the justfile's recipe comments are the map of record. This entry's other clauses (a wrong sentence, a stale pointer) go with the index; nothing is made total.

### fuzz-guests-pins-24 (low, documentation): ruling 63

The validation index has no row for the fuzz targets, the heap cap, or the wasm32 pins

Resolution: Add two rows, each with its failure class as a constructible input and its recipe: the fuzz targets and their heap cap; the wasm32 pins. Acceptance: every leg in the gate's stream list (justfile:464-471) has a row naming what it alone catches.

Ruled (63): the validation index dissolves. Delete the index, its totality claim, and its rendering; the justfile's recipe comments are the map of record. This entry's other clauses (a wrong sentence, a stale pointer) go with the index; nothing is made total.

### module-graph-11 (low, documentation): ruling 63

The validation index and the whole `testing` tree are invisible to both rustdoc passes

Resolution: Either move `validation_index` under the meter-gated tree (for example `crate::meter::validation`), where `docs --all-features` renders it and checks its links (links into `testing::*` would then need to become plain text or point at rendered items), or state at the top of validation_index.rs that it is source-only and drop the `pub`. Acceptance: every intra-doc link in the index is checked by a gate leg, or the file says it is source-only.

Ruled (63): the validation index dissolves. Delete the index, its totality claim, and its rendering; the justfile's recipe comments are the map of record. This entry's other clauses (a wrong sentence, a stale pointer) go with the index; nothing is made total.

### surface-roster-16 (low, documentation): ruling 63

The validation index promises every guarding instrument and omits surfacecheck, citecheck, and the two hidden-surface pins

Resolution: one entry each, stating what it alone catches as a constructible input. If the line scan is retired (surface-roster-9), restate the roster entry's "held equal, name for name, to the `pub fn` surface extracted from source" against rustdoc JSON. Acceptance: `grep -n 'surfacecheck\|citecheck\|doc_hidden\|foreign_reexport' crates/before/src/testing/validation_index.rs` returns one entry each.

Ruled (63): the validation index dissolves. Delete the index, its totality claim, and its rendering; the justfile's recipe comments are the map of record. This entry's other clauses (a wrong sentence, a stale pointer) go with the index; nothing is made total.

### testing-oracles-2 (low, simplification): ruling 63

The validation index is never rendered, so its intra-doc links are unchecked and its `pub` is dead visibility

Resolution: Owner's call between (a) rendering it: move the index out of `cfg(test)` (it holds no code) behind `#[cfg(doc)]` or a feature so `just docs-internal` checks its links, with the links pointed at renderable targets; or (b) keeping it source-read: change `pub mod` to `mod`, replace intra-doc link syntax with plain code spans, and say in the opening paragraph that it is read in source. Acceptance: either `just docs-internal` fails on a dead link in the index, or the file contains no intra-doc link syntax and its visibility matches its siblings.

Ruled (63): the validation index dissolves. Delete the index, its totality claim, and its rendering; the justfile's recipe comments are the map of record. This entry's other clauses (a wrong sentence, a stale pointer) go with the index; nothing is made total.

### tests-other-1 (low, documentation): ruling 63

The validation index omits every instrument in this partition but the bench-judge roster

Resolution: Add a semantic row for the verdict matrix (its class: a kernel-local verdict inversion on adversarial shapes the law populations under-hit; the twins as adequacy), a resource row for the deep-skeleton and answer-embedded criteria, and a short paragraph on the roster pins (`_reads_superlinear`, `_reads_inverted`, doc-hidden, foreign re-export, or their successors under tests-other-13 and tests-other-16) and the coincident-rung witnesses. Acceptance: every `crates/before/tests/*.rs` binary is named in the index or covered by a genre row that names its file.

Ruled (63): the validation index dissolves. Delete the index, its totality claim, and its rendering; the justfile's recipe comments are the map of record. This entry's other clauses (a wrong sentence, a stale pointer) go with the index; nothing is made total.

### codec-base-text-tree-6 (medium, verification): ruling 64

`base_dispatch_read_touches_no_digits` asserts a counter `to_word` cannot reach, so any implementation passes it

Resolution: Delete the test and the "dispatch pins … zero digit touches" clause at base.rs:270-273 and the module doc's "under `limb-meter`, touches no digits" at base/tests.rs:10-12. Keep `base_dispatch_answers_at_word_scale` as the semantic pin, and state at the `impl suanpan::Magnitude for Base` comment that `to_word`'s O(1) rests on the backend's `TryFrom<&UBig> for u64` being a representation check. Do not invent a counter for `to_word`: the limb meter is deliberately silent on `to_u64` (base.rs:37-44), and no observable in this crate distinguishes a constant-time read from a limb walk. Acceptance: no test in `base/tests.rs` claims a counter observes `to_word`; the impl comment no longer names a dispatch pin in touches.

Ruled (64): delete the test's unreachable-counter assertion as stated; no campaign confirmation (ruling 18).

### codec-base-text-tree-16 (low, simplification): ruling 64

`parse_id_str` re-validates bits its own parser built canonically; the test reference runs the same pass, and `validate_id`'s "single source of truth" is untrue

Resolution: Delete text.rs:82 and literal.rs:64 (behavior-preserving: both return `Ok` on every reachable input). Move the guarantee they stood in for ("text-door acceptance is contained in wire-decode acceptance") into the harness: in `assert_id_parse_matches_reference` (tests.rs:1747-1757), on `Ok(bits)` also assert `validate_id(built_view(&bits)).is_ok()`, and drop the reference's own `validate_id` call so the reference is a pure grammar transcription; add the same assertion to a literal-door test. Reword tree.rs:105 to "the wire decoders' normal-form check". If the owner prefers the runtime pass as a totality defense against a future emission bug, state that rationale at each call site instead of the "single source of truth" sentence. Acceptance: every accepted `parse_id_str` and `id_node` output is asserted canonical by a test, not by a production re-parse; `id_text_parser_matches_reference_exhaustively`, the three id-parser proptests, `id_text_parser_error_precedence_pins`, `deep_id_text_roundtrip`, and the literal tests stay green; tree.rs:105 claims no single source of truth.

Ruled (64): delete the guard. The commit message names the committed differential test that holds the guarded property; the code carries no comment, marker, or trace of the deletion. Finch's words: "By 'delete, citing' you mean in the commit message, right? Don't leave traces in the code." Where the entry's Resolution offers a restatement or a relocation of the assert instead of deletion, deletion wins.

### codec-base-text-tree-7 (low, simplification): ruling 64

`Sub`'s `debug_assert!` compares through the metered `Ord` and duplicates the backend's own underflow panic; `SubAssign` clones the whole magnitude

Resolution: Delete the `debug_assert!` and add a `# Panics` line to the impl stating that an underflowing difference panics (the backend's own check, every profile; programmer error per the crate's panic policy). Rewrite `sub_assign` as `meter_limbs2(self, rhs); self.0 -= &rhs.0;`. Measure every limb-denominated envelope whose cell subtracts at the parent commit, then tighten the committed ceilings in the same change with the movement attributed to the assert's removal; strike the profile caveat at tests/meter.rs:63-64 if no other source remains. Acceptance: under `--features limb-meter`, `limb_ops()` after one `Base - &Base` on two k-limb operands reads 2k in dev and release alike; envelopes re-pinned with the parent measurement recorded; the meter.rs header either drops the caveat or names its remaining source.

Ruled (64): delete the guard. The commit message names the committed differential test that holds the guarded property; the code carries no comment, marker, or trace of the deletion. Finch's words: "By 'delete, citing' you mean in the commit message, right? Don't leave traces in the code." Where the entry's Resolution offers a restatement or a relocation of the assert instead of deletion, deletion wins.

### version-core-13 (low, simplification): ruling 64

The `debug_assert!` on `hull.relation` is the only production reader of `emit::Hull.relation`, and the differential it re-runs is committed

Resolution: drop the `debug_assert!` and its comment. Then decide (skyline partition) whether `Hull.relation` and the directions fold in `emit::hull` keep a consumer; if the comparable-pair fast path is not planned, remove the field, the fold, and the three `hulled.relation` assertions in emit/tests.rs, and re-state the emit doc. If a consumer is planned, replace the assert with that consumer. Acceptance: `grep -rn 'hull.relation\|hulled.relation' crates/before/src` returns only emit-internal hits, or none; `identity_fast_paths_agree_across_buffer_identity` and the `span_is_the_pair_hull` law stay green.

Ruled (64): delete the guard. The commit message names the committed differential test that holds the guarded property; the code carries no comment, marker, or trace of the deletion. Finch's words: "By 'delete, citing' you mean in the commit message, right? Don't leave traces in the code." Where the entry's Resolution offers a restatement or a relocation of the assert instead of deletion, deletion wins.

### codec-bits-25 (nit, simplification): ruling 64

id_node re-parses a node it has proved normal by construction

Where: `crates/before/src/codec/literal.rs:51-65`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Delete the `validate_id` call and state the induction, or validate once in `finish_id`

Ruled (64): delete the guard. The commit message names the committed differential test that holds the guarded property; the code carries no comment, marker, or trace of the deletion. Finch's words: "By 'delete, citing' you mean in the commit message, right? Don't leave traces in the code." Where the entry's Resolution offers a restatement or a relocation of the assert instead of deletion, deletion wins.

### inventory-6 (nit, simplification): ruling 64

`id_node` re-validates the whole subtree at every tuple level

Where: `crates/before/src/codec/literal.rs:52-66`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Validate once in the tuple `TryFrom` impl

Ruled (64): delete the guard. The commit message names the committed differential test that holds the guarded property; the code carries no comment, marker, or trace of the deletion. Finch's words: "By 'delete, citing' you mean in the commit message, right? Don't leave traces in the code." Where the entry's Resolution offers a restatement or a relocation of the assert instead of deletion, deletion wins.

### clock-26 (low, simplification): ruling 65

clock/tests.rs holds the serde and borsh legs for three types and depth proofs for other partitions' surfaces

Resolution: Move the serde block to serde_impls/tests.rs and delete the pointer. Reconcile the borsh block against borsh_impls/tests.rs, keeping only what is not already asserted there (the two-value concatenation at 1191-1198 if no existing test covers the `Version` pair; the appended-zero-byte rejection if `non_canonical_borsh_bytes_report_invalid_data` does not). Move the two non-clock depth proofs beside their surfaces, or gather every depth-100k proof into one `testing/stack_safety.rs` and update AGENTS.md:37 and surface.rs:1221 (`GridCap { guard: "deep_tree_stack_safety" }`); hoist `DEPTH` to module scope. Acceptance: clock/tests.rs holds only clock differentials, protocol semantics, the clock depth proof, and the orbit pins; `tools/citecheck` resolves every roster citation; `just test-all --all-features` green.

Ruled (65): relocate as stated. This lane runs alone while the moves land, since the moved files are edited by other lanes.

### suanpan-30 (low, simplification): ruling 65

The roster reaches into before's test file as the sole evidence for `add_small`/`sub_small`

Resolution: an `i64` twin of `u64_comb_touches_are_flat_and_exact` driving `add_small(1)`/`sub_small(1)` on the `2^k - 1` cliff at k = 4096 and 8192 with the derived exact total, cited under both rows; keep or drop the `BANDS` citations as the owner rules (their `OWN` co-witnesses stand). Acceptance: `add_small` and `sub_small` each cite an `OWN` witness that reaches them; if `BANDS` is dropped, no path outside the manifest directory remains in claims.rs.

Ruled (65): relocate as stated. This lane runs alone while the moves land, since the moved files are edited by other lanes.

### version-core-27 (low, simplification): ruling 65

`version/tests.rs`'s module doc omits half the file, and the `Rank`/`Ranked` suites live two modules away from the types they test

Resolution: minimum, rewrite the module doc to map every `// ───` section in the file. Better, move the rank sections to `crates/before/src/version/rank/tests.rs` (the directory exists) with `mod tests;` in rank.rs, and the `Ranked`/composite-key sections to `version/ranked/tests.rs`, taking `stream_rank`/`seeded_rank`/`rank_parts`/`stairs` with their consumers; the `pub(crate)` items they reach (`from_raw`, `raw_parts`, `numerator_is_wide`, `content_bits`, `arm_ceiling`, `BACKEND_CAPACITY_BITS`) need no visibility change. The move crosses into the rank partition's ownership; this note is the pointer. Acceptance: `rank.rs` and `ranked.rs` each have a sibling `tests.rs`, `version/tests.rs`'s module doc names exactly its remaining sections, and `just gate` is green with no new seed files (a moved proptest that had a seed replays from its new path, per `tests/seed_liveness.rs`, or the seed moves with it).

Ruled (65): relocate as stated. This lane runs alone while the moves land, since the moved files are edited by other lanes.

### rank-2 (nit, simplification): ruling 65

Rank and Ranked tests live in the parent module's test file

Where: `crates/before/src/version/rank.rs:144-147`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Move the five sections to sibling `tests.rs` files; drop the re-export

Ruled (65): relocate as stated. This lane runs alone while the moves land, since the moved files are edited by other lanes.

### fresh-eyes-2 (medium, documentation): ruling 67

Version, Party, and Clock decode do not state the whole-input contract and lack # Errors sections

Resolution: Add a `# Errors` section to `Version::decode`, `Party::decode`, and `Clock::decode` in the form `Span::decode` uses, and state in each that the reader is read to end and must hold exactly one value. Owner-gated suggestion, separately: a prefix-decoding entry (for example `decode_prefix(&[u8]) -> Result<(Self, &[u8]), Decode>`) for callers who frame values without a length prefix; the borsh feature already relies on the encodings being prefix-free. Acceptance: `grep -c '# Errors' crates/before/src/version.rs` is at least 1 and each of the three `decode` docs names `TrailingBits` for spurious input.

Ruled (67): one pass copying `Rank::decode`'s `# Errors` form and `walk.rs`'s `# Panics` form; `doclint` gains the rule that every `pub fn` returning `Result` carries `# Errors`, with a committed fixture that fails without the section.

### api-audit-8 (low, documentation): ruling 67

`# Errors` sections cover about half the fallible public entries

Resolution: add `# Errors` to the listed sites, naming the `Decode`/`Parse`/`TooWide` variants each can return; `Rank::decode` (rank.rs:435-445) and `Span::decode` (span/wire.rs:79-85) are the variant-by-variant template, and `Rank::encode_to` (rank.rs:403-406) the one-line template for writers. Acceptance: every `pub fn` or trait impl returning `Result` carries a `# Errors` section.

Ruled (67): one pass copying `Rank::decode`'s `# Errors` form and `walk.rs`'s `# Panics` form; `doclint` gains the rule that every `pub fn` returning `Result` carries `# Errors`, with a committed fixture that fails without the section.

### clock-11 (low, documentation): ruling 67

The fallible construction doors carry no `# Errors` section while the joins do

Resolution: Add `# Errors` to the three clock doors naming the variants and their triggers (decode: `Io` from the reader, `Truncated` for exhausted input including the flush-id cut, `TrailingBits` for a spurious remainder, `NotCanonical` for a collapsible id pair or an invalid version stream, all checked before any byte is adopted; text and literal doors: `Syntax`, `NotCanonical`, `Anonymous`), and sweep `Party`/`Version`'s doors in the same pass. Acceptance: every public `-> Result<_, _>` item on `Clock`, `Party`, and `Version` has an `# Errors` section.

Ruled (67): one pass copying `Rank::decode`'s `# Errors` form and `walk.rs`'s `# Panics` form; `doclint` gains the rule that every `pub fn` returning `Result` carries `# Errors`, with a committed fixture that fails without the section.

### skyline-fill-grow-4 (low, documentation): ruling 67

`# Panics` promises a panic on any non-canonical stream; the code panics only on unreadable bits

Resolution: Reword the six sites to walk.rs's form: the operand must be canonical (every stored `Version` is; run `validate` on untrusted bytes); truncation and malformation panic; a well-formed non-canonical stream yields an unspecified result, per `causal_cmp`'s statement. Acceptance: no `# Panics` in fill.rs, fuse.rs, or grow.rs claims a panic for non-canonical input as such; each states the precondition and the malformed-stream panic separately.

Ruled (67): one pass copying `Rank::decode`'s `# Errors` form and `walk.rs`'s `# Panics` form; `doclint` gains the rule that every `pub fn` returning `Result` carries `# Errors`, with a committed fixture that fails without the section.

### skyline-sweep-place-masked-13 (low, documentation): ruling 67

`OpenedPair::open` and `BoundSide::open` overstate their Panics relative to the walk they wrap

Resolution: replace both with a citation of the one statement: "[`LeafCursor::open`]'s canonical-stream contract, on each stream." Acceptance: neither constructor claims an unconditional panic on non-canonical input.

Ruled (67): one pass copying `Rank::decode`'s `# Errors` form and `walk.rs`'s `# Panics` form; `doclint` gains the rule that every `pub fn` returning `Result` carries `# Errors`, with a committed fixture that fails without the section.

### version-core-14 (low, documentation): ruling 67

`Version::decode` and the text/literal constructors have no `# Errors` section; `Rank::decode` and `Ranked::decode` carry itemized ones

Resolution: add `# Errors` to `Version::decode` mirroring `Rank::decode`'s shape (`Decode::Truncated`, `Decode::NotCanonical`, `Decode::TrailingBits`, `Decode::Io`, each with its trigger drawn from `validate_prefix` and `require_marker_padding`); `# Errors` naming `Parse::Syntax`/`Parse::NotCanonical` on the two `FromStr` impls and the node literal; on `TryFrom<u64>`, "Never fails; the fallible spelling lets leaves compose with node literals, whose `TryFrom<T, Error = Parse>` bound it satisfies." Acceptance: every fallible public entry in version.rs, own.rs, and ticks.rs has an `# Errors` section naming each variant it can return, and the infallible `TryFrom<u64>` says so.

Ruled (67): one pass copying `Rank::decode`'s `# Errors` form and `walk.rs`'s `# Panics` form; `doclint` gains the rule that every `pub fn` returning `Result` carries `# Errors`, with a committed fixture that fails without the section.

### api-audit-21 (nit, documentation): ruling 67

TryFrom<u64> for Version can never fail, and its docs do not say so

Where: `crates/before/src/version.rs:1473-1478`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): one sentence in the impl doc: "Never fails; the `Result` is the shape the nested `(n, left ...

Ruled (67): one pass copying `Rank::decode`'s `# Errors` form and `walk.rs`'s `# Panics` form; `doclint` gains the rule that every `pub fn` returning `Result` carries `# Errors`, with a committed fixture that fails without the section.

### skyline-coding-35 (nit, documentation): ruling 67

a `# Panics` paragraph copied five times in walk.rs while claiming to be "stated once there"

Where: `crates/before/src/version/skyline/walk.rs:74-79`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): state it once in walk.rs's module doc and reduce each function's section to `# Panics` plus one line ("Canonical input required ...

Ruled (67): one pass copying `Rank::decode`'s `# Errors` form and `walk.rs`'s `# Panics` form; `doclint` gains the rule that every `pub fn` returning `Result` carries `# Errors`, with a committed fixture that fails without the section.

### skyline-sweep-place-masked-9 (nit, documentation): ruling 67

One Panics paragraph copied seven times in overlay.rs, each copy saying it is stated once elsewhere

Where: `crates/before/src/version/skyline/overlay.rs:331-336`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): state the contract once on each cursor struct's doc (`LeafCursor` at 302, `IdLeafCursor` at 460) and reduce every method's section to one line citing  ...

Ruled (67): one pass copying `Rank::decode`'s `# Errors` form and `walk.rs`'s `# Panics` form; `doclint` gains the rule that every `pub fn` returning `Result` carries `# Errors`, with a committed fixture that fails without the section.

## Roster members pending Finch's approval

Lows and nits no ruling has reached, placed here by the files they touch. Land only after the coordinator confirms the roster is approved.

### api-audit-7 (low, verification): roster: pending Finch's approval

auto_traits.rs claims to pin every public API type but omits Limbs, TooWide, the shape types, and the polarity markers

Resolution: add the missing `assert_impl_all!` lines (`shape::Cell<1>` and `shape::Cells<'static, 1>` for the const-generic pair), or derive the roster from the surface census so it cannot drift; otherwise narrow the module doc and the two surfacecheck comments to what is pinned. Acceptance: every struct and enum in rustdoc's `all.html` for the default feature set appears in an `assert_impl_all!` line, or a committed check compares the two lists.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### benches-examples-8 (low, correctness): roster: pending Finch's approval

`scale_from_env` says "positive number" but accepts zero, negatives, NaN, infinity, and saturating magnitudes; the tripwire has no downstream guard

Resolution: after parsing, require `scale > 0.0 && scale.is_finite()` inside `scale_from_env` (one site, both bench targets) and at amp_board.rs:195-197, keeping the existing messages. Acceptance: `BOARD_BENCH_SCALE=nan cargo bench -p before --bench tripwire` panics at the parameter naming `BOARD_BENCH_SCALE`; likewise for `0`, `-1`, `inf`; `cargo run --example amp_board ... -- -1` panics at the parse site. Construction: `BOARD_BENCH_SCALE=nan BOARD_BENCH_DENOMS=<scratch>/d.json cargo bench -p before --bench tripwire -- --sample-size 10 --measurement-time 1` completes, and d.json carries `"scale": NaN` and `"denominator_bytes": 0`.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-families-floors-judge-11 (low, verification): roster: pending Finch's approval

The "four cells watched by neither leg" disclosure is a hand count with no pin on either side, already stale, and its 10 µs restates a benchjudge constant

Resolution: Add a test beside the board tests that builds every board bundle, collects every cell whose `Floors` are all `NotApplicable`, and asserts the set equals a committed roster (the four named, the five text-rejection rows, and the `clock_fork` cells finding 10 leaves all-NA until it is fixed, or the smaller set once it is); have the disclosure here and at board.rs:99-100 state the class and cite that roster and `MIN_JUDGED_MEDIAN_NANOS` by name instead of "four" and "10 µs". If the wall-time half matters, have tools/benchjudge emit its sub-floor cell set and pin it in tests/bench_judge_roster.rs. Acceptance: a committed test fails when a new op declares NA on every floored currency without joining the roster; floors.rs:99-112 and board.rs:99-100 carry no cell count and no duration literal. Construction: Add a row to ops.rs whose `prepare` returns `Floors` with `na(..)` on heap, limb, scan, and touch, or observe that `clock_fork` on the dense family already does: nothing in the gate changes and floors.rs still reads "Four cells".

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-families-floors-judge-27 (low, correctness): roster: pending Finch's approval

`version_output_bytes` and the sibling ops.rs sites omit the marker bit and under-report the packed size by one byte on byte-aligned streams

Resolution: `v.as_bytes().len()` (O(1), public) and delete the `try_from` dance; at the ops.rs sites use `as_bytes().len()` / `encode().len()` for party and clock (1316 included, which also removes the NA collapse on tiny children). Acceptance: a committed test sweeping `study_family_versions(DEFAULT_SCALE)` plus one version with `encoded_bits() % 8 == 0` asserts `version_output_bytes(&v) == v.encode().len()` and passes. Construction: Tick a fresh `Version` with `Party::seed()` until `v.encoded_bits() % 8 == 0` (or pick one such stream from the family corpus), then `assert_eq!(version_output_bytes(&v), v.encode().len())`: the left side is one less today.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-frame-15 (low, correctness): roster: pending Finch's approval

The version output reader undercounts a flush stream by its marker byte

Resolution: `v.as_bytes().len()` in `version_output_bytes`. Acceptance: for a version whose `encode()` ends in `0x80`, `version_output_bytes(&v) == v.encode().len()`; a unit test beside the reader pins it. Construction: Any version with 8k live bits (`encoded_bits() == 8k`): `div_ceil(8) == k` while `encode().len() == k + 1`; `Version::new().encoded_bits()` is 2, so tick a seeded version until `encoded_bits() % 8 == 0` and compare.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-frame-21 (low, correctness): roster: pending Finch's approval

`truncated_bytes` argues its two-byte cut from a decoder verdict the decoder no longer produces, and the one-byte cut is both correct and more deferred

Resolution: Always cut one byte and re-state the doc: on a non-flush stream the cut removes the last live bits and the marker, and the tree walk runs out of input; on a flush stream it removes the marker byte alone, the whole tree parses, and the padding judge reports `Truncated` at the end, the most deferred placement byte granularity allows. Relax the guard to `bytes.len() > 1`. Acceptance: a unit test beside the builders: for a version whose `encode()` ends in `0x80`, `truncated_bytes(&bytes).len() == bytes.len() - 1` and `Version::decode(&truncated_bytes(&bytes)[..])` is `Err(Decode::Truncated)`; the truncation rows' `matches!(err, Decode::Truncated)` assertions stay green; scan readings on flush-stream families rise, never fall. Construction: Take any version whose live bits are a multiple of 8 (its `encode()` ends in `0x80`); drop only the final byte; `Version::decode` walks the complete tree, reaches `pos == total`, and `require_marker_padding` returns `Decode::Truncated` (remainder 0), not `TrailingBits`.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-frame-3 (low, verification): roster: pending Finch's approval

The "four cells no deterministic leg watches" disclosure is a prose-only roster; nothing pins the all-NA cell set

Resolution: Add a board/tests.rs test that prepares `ops()` × `FamilyId::board()` at the smoke scale, collects the cells whose `floors.each()` are all `NotApplicable`, and asserts the sorted `(op, family)` set equals a committed list (the hash rows on each family plus `version_eq` on the benign family, or whatever the enumeration shows); have board.rs:99-100 and floors.rs:99 cite the test by name and drop the count, noting which listed cells the time leg's 10 µs floor also excludes. Acceptance: changing any one cell's floor to NA, or adding an all-NA row, fails exactly one named test; the prose carries no count. Construction: Change `version_encode`'s heap floor in floors.rs to `na(...)`; the board's unit suite stays green while floors.rs:99 names four cells and five (or more) are all-NA.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### crate-root-7 (low, verification): roster: pending Finch's approval

auto_traits.rs claims every public API type but omits `Limbs`, `TooWide`, and the whole `shape` module, and surfacecheck defers totality to it

Resolution: Add pins for `crate::Limbs<'static>`, `crate::error::TooWide`, and each `crate::shape` type (`Plateau`, `Rise`, `Region`, `Cell<1>`, `Plateaus<'static>`, `Regions<'static>`, `Overlay<'static>`, `Cells<'static, 1>`). Then close the drift path mechanically: have surfacecheck's census compare the set of public struct and enum paths it already extracts against the names pinned in this file (a text scan suffices), so an unpinned public type fails `just surface-totality`; the exclusion rationale at extract.rs:21-26 and 267-269 then names that check. Acceptance: deleting one `assert_impl_all!` line fails a gate leg naming the type; the added pins compile. Construction: On a branch, add a `PhantomData<*const ()>` field to `shape::Plateaus` and run `just gate`: no pin names `Plateaus` and surfacecheck skips auto-trait rows by design, so a `!Send` public type ships with every instrument green.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### deps-5 (low, correctness): roster: pending Finch's approval

build.rs `expect("validated")` on `meta.base_seed` names a proof that does not exist

Resolution: validate the index's `meta` once (`base_seed` and `samples_per_column` as u64, `commit` as a non-empty string) in a `validate_meta(file, meta)` beside `validate`, after which the expect message is true; or replace the expect with `unwrap_or_else(|| panic!("{file}: meta.base_seed must be an integer"))`. Acceptance: the construction below panics naming fuelscape/index.json and the field.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuelscape-render-31 (low, correctness): roster: pending Finch's approval

`build.rs`'s re-validation compares untyped JSON values, so non-integer sizes and counts pass, and an `expect("validated")` names a check that never ran

Resolution: demand the types before comparing: collect `sizes` and each `c` into `Vec<u64>`, panicking with the file name on any non-integer entry, then compare plain integers; validate `base_seed` and `samples_per_column` as `u64` in the meta loop so the `expect` at 216 becomes true or dissolves into a checked value (or deserialize `meta`, `sizes`, `cols` into small typed structs with `deny_unknown_fields`, mirroring the compactor's). Acceptance: a committed document with `"sizes":[null,2,...]` or `"c":["x",1]` fails `cargo build -p before` naming the file and check; `"base_seed":"7"` in both files fails naming the parameter rather than panicking with `validated`. Construction: in a scratch copy of `crates/before/fuelscape/`, set `op.sizes[0]` to `null` in one document: line 304's `all(...)` returns true (`None < Some(2)`) and the emitted island carries the null. Delete `meta.base_seed` from both `index.json` and that document: lines 58-63 pass (`Null == Null`), then line 216 panics with `validated`.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuelscape-render-6 (low, correctness): roster: pending Finch's approval

Positivity of log-scaled quantities is enforced three ways for fuel and not at all for the size axis

Resolution: reject zero fuel (samples and overlay points) and zero size once at the format's strict gate, `dump::read` (and `DumpWriter::append`), and add a positivity check on `sizes[0]` to `compact::validate` and `build.rs`'s validator; then either drop the floor in `lg` or document it as unreachable given the gate, and extend the smoke assertion at render/tests.rs:39-43 to overlay points. Acceptance: a dump tamper case setting a sample's fuel to 0 is refused by `read` naming the check; a compact tamper case setting `sizes[0] = 0` is refused; `compact`'s own zero check is then a second line and says so, or goes. Construction: build an `AtlasData` with one `fuel: 0` sample, `DumpWriter::append` it, `dump::read` it back (accepted), `render_op` it (renders, the point at `log2(1) = 0`), then `compact` it (refused). Separately, in `compact/tests.rs`'s tamper closure set `doc["op"]["sizes"][0] = 0`: `read` accepts today.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzz-guests-pins-10 (low, verification): roster: pending Finch's approval

The fuzz framing is a prose wire contract duplicated across the detached boundary

Resolution: Extract the framing (`ARITY_SPAN`, `chunk`, `byte`/`picks`, the decode-ops flavour/length carve) into one file `#[path]`-included by the targets, `tests/fuzz_seeds.rs`, and `tests/support/fuzz_seed_set.rs` (the seed set already shares by `#[path]` between the example and the test). Acceptance: `ARITY_SPAN` has one definition; the seed test's in-band assertion reads the constant the target folds with. Construction: Set `ARITY_SPAN` to 16 in fuzz_laws.rs only: the seed test still passes (its copy asserts `< 18`), while the target now folds the seed's arity-17 script to 1 and never crosses the second octave it was written to cross.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzzfit-bands-30 (low, verification): roster: pending Finch's approval

The kernel roster in `sanity.rs` is a hand-maintained one-per-variant list nothing ties to `Op`

Resolution: Make totality a compile error without a new dependency: an exhaustive `match op { Op::ClockSeed { .. } => (), ... }` over the roster's variants in the test (or a `fn representative(op: &Op) -> Op` the test walks), so a new variant fails to compile until rostered; or colocate the roster beside `Op::kernel` in `ops.rs` as `pub const REPRESENTATIVES` so both edits land in one diff. Acceptance: adding an `Op` variant without a roster entry fails to compile or fails a test by name; the convention sentence is gone. Construction: Add `Op::VersionNoop { src: Reg }` with `kernel() => "ff_version_noop"`, omit it from the roster and from every strategy: `bands_and_op_roster_name_the_same_kernels` stays green and no test names the unpriced kernel.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzzfit-bands-6 (low, verification): roster: pending Finch's approval

The wasmtime-bump-is-a-re-pin sentence is a convention with no mechanism, contradicted at both patch bumps

Resolution: Either enforce it (build.rs parses the workspace Cargo.lock's `wasmtime` version into `FUZZFIT_WASMTIME_VERSION`; calibrate emits `PINNED_WASMTIME`; a sibling of `building_toolchain_matches_the_pin` asserts equality) or soften the sentence to what holds ("a wasmtime bump that moves the fuel schedule reads red through the staleness leg past `REFIT_TOLERANCE`; a bump that leaves the schedule alone stays green and is not a re-pin event"). Acceptance: a lockfile wasmtime bump without re-pin turns `just fuzzfit` red by name, or the sentence names the mechanism (`REFIT_TOLERANCE`) that actually bounds it. Construction: The two historical bumps are the demonstration: `git show --stat 8490af3f` and `git show --stat 4e64a4fb` touch only lockfiles, `bands.rs` is unchanged since e7a4b7b0, and 4e64a4fb's message records a clean gate; `grep -n wasmtime tests/enforce.rs` returns nothing.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzzfit-bands-7 (low, correctness): roster: pending Finch's approval

`fit()`'s floor fallback contradicts `FIT_FLOOR_BITS`'s doc, and "classifies constant" is not guaranteed

Resolution: Either state the actual rule at both docs ("when at least two floored samples exist"; drop "and classifies constant") or make the fallback classify constant by rule (the sub-floor law of record, as `fit_constant` does), so a size-law slope is never fitted through sub-floor points. Acceptance: a unit test in `fit/tests.rs` with sub-floor samples over 8..127 bits plus one at 128 either yields `constant == true` or the docs name the two-sample condition; the test's doc comment states which. Construction: `samples = [(8,f),(12,f),(16,f),(24,f),(32,f),(48,f),(64,f),(96,f),(100,f),(110,f),(120,f),(127,f),(128,f)]` with `f` constant: `floored.len() == 1`, so all thirteen are fitted; `decades = log10(128/8) = 1.204 >= 1.0`; populated buckets {1, 2, 3, 4} >= 3; `constant` is false and `min_denom` is 8.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzzfit-strategies-5 (low, verification): roster: pending Finch's approval

The pinned bands are not bound to the corpus that produced them

Resolution: have `calibrate` write a corpus digest beside `PINNED_RUSTC` (the harness already has an FNV; hash the ops of the first `REFIT_PREFIX_PROGRAMS` deterministic programs plus the bootstrap stream into `pub const CORPUS_DIGEST: u64`), and add an enforce.rs test beside `building_toolchain_matches_the_pin` that recomputes it from `for_each_deterministic_program`/`for_each_bootstrap_program` and names `just fuzzfit-calibrate` on mismatch; state in harness/Cargo.toml that proptest and rand_chacha are pin provenance. Acceptance: changing any draw range, weight, or family body, or bumping proptest/rand_chacha in Cargo.lock, fails `just fuzzfit` by name before any fuel is judged; a re-pin restores green and the diff shows the digest move.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzzfit-strategies-8 (low, verification): roster: pending Finch's approval

The roster test's hand list is a convention against a variant added without a roster entry; a derived variant list is a check

Resolution: derive a variant count or discriminant array on `Op` and have `bands_and_op_roster_name_the_same_kernels` iterate it (or assert the hand roster's length against the derived count so an omission fails by name). Acceptance: adding an `Op` variant with a kernel string but no roster entry fails the sanity suite before any generator emits it.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### rumors-dependence-3 (low, verification): roster: pending Finch's approval

The `as_bytes == encode` laws, tests, and the roster pin they anchor are tautological: `encode` is `as_bytes().to_vec()`

Resolution: re-denominate each body against the independent judge while keeping the names, so the roster and the duplicate-name table stay untouched: in `laws.rs`, `Version::decode(a.as_bytes()).is_ok_and(|d| d.as_bytes() == a.as_bytes())` (and the party twin; `laws.rs` has no field access); in the party and version test modules, `codec::padding_is_canonical(&v.0)` or the same decode form, so `after_fork` and `after_ticks` assert what their docs say without relying on the debug assertion. Re-state `clock/tests.rs:300-305` in the present tense (what it protects: stored padding after impl-driven `fork`/`join`/`sync`, judged by strict decode). Alternatively delete the two laws and drop the pin from `CODEC_PINS`, since the roundtrip laws already cover it. Acceptance: with the `debug_assert!` in `as_bytes` disabled and an unsealed tail introduced after `join` (the historical seam), the re-denominated tests fail; today they cannot.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### suanpan-28 (low, verification): roster: pending Finch's approval

`SOURCES` is a hand-kept file roster; a new module with `pub` items escapes the totality test

Resolution: a binding test that parses `src/lib.rs` for its `mod name;` / `pub mod name;` declarations and asserts each (except `claims`) has a `SourceSpec`, so an unlisted module fails by name. Acceptance and construction: add `mod scratch;` with a `pub fn` to lib.rs unlisted: the claims tests are green today and must fail afterwards; listing it then fails totality until a claim row exists.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### surface-roster-3 (low, verification): roster: pending Finch's approval

`Exclusion::FAMILIES` is a hand-maintained twin of the enum, and a variant missing from it escapes the inhabitation census

Resolution: derive the list from the enum. Dependency-free: a small `macro_rules!` that takes the variant list once and emits the `enum`, `FAMILIES`, and `family()`. With a dependency (owner's call): `strum::VariantNames` for `FAMILIES` and `strum::IntoStaticStr` for `family()`, `strum` optional under `meter`. Acceptance: adding a variant to `Exclusion` without an inhabitant fails `every_exclusion_family_is_inhabited` (or fails to compile). Construction: add `Probe { pins: &'static [&'static str] }` to `Exclusion`, add `Exclusion::Probe { .. } => "Probe"` to `family()`, leave `FAMILIES` unchanged, use the variant in no row; `cargo nextest run -p before every_exclusion_family_is_inhabited` stays green.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### tests-other-20 (low, verification): roster: pending Finch's approval

Nothing pins that every fuzz target has a seed directory

Resolution: Assert the file stems under `fuzz/fuzz_targets/` equal `expected_targets`, so a new target must gain seeds or a documented exemption. Acceptance: adding `fuzz/fuzz_targets/fuzz_new.rs` with no seed directory reads red. Construction: Create an empty sixth target file under `fuzz/fuzz_targets/`; `seed_directories_hold_exactly_the_set_of_record` stays green.

Roster note: lands only once the coordinator confirms the roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

