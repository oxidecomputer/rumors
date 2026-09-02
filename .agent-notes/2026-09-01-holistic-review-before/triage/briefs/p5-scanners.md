<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P5 lane: one typed scanner

## Goal

The workspace has one source-collection authority, and it is typed:
`tools/citecheck` as a Rust checker that collects tests, envelope rows,
and band kernels by parsing, sees `#[ignore]`, and is the authority the
superlinear, inverted-twin, and band rosters resolve against, so a
rostered kernel that does not run reads as failing. before's line-scan
surface extractor retires in favor of surfacecheck's two-way
reconciliation; doclint's summary-length rule retires in favor of
clippy's lint. Rulings 78 (decision 44) and 80 (decision 79) fix the
shape; this brief carries them, plus one pending low.

## The retirement discipline

Every dissolution in this lane follows the doctrine's rule for retiring
an instrument: the replacement demonstrates it catches what the
instrument caught before the instrument is deleted. Concretely, each
retirement is one commit series: first the replacement (a unit test,
a fuzz-fit case, a clippy lint confirmed on a fixture, a surfacecheck
reconciliation) landed and shown firing on the artifact the old
instrument existed to catch, with the firing quoted in the commit
message; then the deletion, in a commit that names the replacement.
A retirement whose replacement cannot be shown to fire does not land:
it is reported as a finding about the replacement and the entry stays
open. Deletions leave no trace in the code (no "formerly", no retired
names in prose); the commit message carries the provenance.

## Ground rules

These apply to every lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `33779b10` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `33779b10`, fast-forward; if it has diverged, stop and report. Never call
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

- surface-roster-9 (retire before's line scan) lands only after
  surface-roster-28's `syn`-based extractor (ruling 47, `p2-surface`)
  has landed on main, because suanpan's claims roster keeps using the
  shared extractor and must not be left on the line scan. If
  `p2-surface` has not landed at launch, this lane lands everything
  else and holds that retirement; the coordinator names the SHA.
- The citecheck rewrite (surface-roster-31, tools-13) precedes the
  roster rebinding (tests-other-24, surface-roster-11, envelopes-b-22,
  envelopes-b-25, meter-registry-tier2-3), which precedes deleting the
  old roster scanners (tests-other-3, suite-economics-3,
  surface-roster-10).
- tools-20's clippy lint lands on the root and detached-workspace
  clippy legs first; the fixture confirmation is quoted in the commit
  that drops rule 1.
- This lane owns `tools/citecheck*`, `tools/doclint*`,
  `crates/before/tests/superlinear_tripwires.rs` and the twin and band
  roster tests, `crates/before/src/surface.rs`'s scan half, and the
  `justfile` legs that call them. `p1-gate` also edits the justfile;
  rebase onto it.

## Hazards and stops

- A kernel the new authority reports as `#[ignore]`d or unreachable is
  a finding, not something to un-ignore; report it.
- Rewriting citecheck in Rust changes a gate leg's implementation; its
  self-test cases (the fabricated-citation check, the shadow guard,
  the string-literal awareness of tools-13) are reproduced as unit
  tests and green before the Python tool is deleted.
- Ruling 43 applies: rosters bind by typed reference; a string naming a
  test, file, or line that the rewrite would otherwise keep is a stop.

## Members

### envelopes-b-22 (medium, verification): ruling 78

Thirteen cost pins bind to no roster, and the registry excuses their families in prose that misdescribes three of them

Resolution: rename the thirteen to the convention (`..._is_flat_per_unit` for the ratio bands, `..._band` for the equality and liveness pins) and replace each `Bands::Unbanded { reason: "priced by ... tests/meter.rs" }` with `Bands::Priced(&[...])` naming them; alternatively extend `band_test_names` to the `_reads_linear`/`_touches_flat`/`_is_pinned_and_flat` genres. Either way correct the three registry reasons that call ratio bands "absolute pins", and revisit the Dense ruling against the clone-cheap pin. Acceptance: `band_tests_and_registry_citations_stay_paired` fails when any one of the thirteen is deleted or renamed; `grep -c 'absolute pins outside the band convention' crates/before/src/meter/registry.rs` reads 0. Construction: delete `fn memo_comb_resolution_reads_linear` (8496-8504) and run `just gate`: no leg fails, because neither `band_tests_and_registry_citations_stay_paired` nor `superlinear_tripwires_match_the_committed_roster` scans that name.

Ruled (78, decision 44): the workspace gets one typed collection authority. before's line-scan extractor retires in favor of surfacecheck (suanpan keeps the `syn`-based extractor ruling 47 lands in `p2-surface`); `tools/citecheck` becomes a typed Rust checker and the collection authority for the superlinear, inverted-twin, and band rosters, so an `#[ignore]`d kernel reads as failing; the fabricated-citation check and the shadow guard are reproduced as unit tests of the new checker before any old scanner is deleted. `Copy` on `Leg` and `Exclusion` rides along. Where the Resolution offers an alternative that keeps a hand scanner, that alternative is struck.

### envelopes-b-25 (medium, verification): ruling 78

Three green pins carry the names of the red readings they were born asserting

Resolution: rename to the invariant in the band convention (`reveal_comb_close_reveal_cycle_is_flat_per_unit`, `pure_comb_width_cycle_is_flat_per_unit`, `ascend_cliff_undercut_cascade_is_flat_per_unit`, and `memo_chain_distinct_resolution_is_flat_per_unit`/`memo_comb_resolution_is_flat_per_unit` if envelopes-b-21's per-byte form lands), keeping `_reads_superlinear_on_...` exclusively for red kernels, and cite them from their families' `Bands::Priced`. Acceptance: `grep -n 'fn .*_reads_' crates/before/tests/meter.rs` returns only `sequential_meet_reduce_reads_superlinear_on_shade`.

Ruled (78, decision 44): the workspace gets one typed collection authority. before's line-scan extractor retires in favor of surfacecheck (suanpan keeps the `syn`-based extractor ruling 47 lands in `p2-surface`); `tools/citecheck` becomes a typed Rust checker and the collection authority for the superlinear, inverted-twin, and band rosters, so an `#[ignore]`d kernel reads as failing; the fabricated-citation check and the shadow guard are reproduced as unit tests of the new checker before any old scanner is deleted. `Copy` on `Leg` and `Exclusion` rides along. Where the Resolution offers an alternative that keeps a hand scanner, that alternative is struck.

### surface-roster-9 (medium, simplification): ruling 78

Two extractors of the same `pub fn` surface: the line-scan's justifying constraint dissolved when surfacecheck landed

Resolution: retire before's line-scan extractor: delete `SURFACE_SOURCES`, `extract_public_fns`, and `roster_is_total_over_the_public_fn_surface`; have `meter/board/coverage/tests.rs::public_surface` read `METHOD_SURFACE` op names (it is held equal to the extraction anyway); re-word the "two jaws" prose in tests/doc_hidden.rs, tests/foreign_reexport.rs, the surface_coverage.rs module doc (drop "# Tamper-evident totality"; totality is surfacecheck's), and the justfile to name one totality check. `surface-scan` stays for suanpan's claims roster. If the owner wants the stable-toolchain inner-loop check kept, the accurate alternative is to keep the scan retitled as a convenience and delete the two forks entries. Acceptance: with a `pub fn` added to any public type and no roster row, `just surface-totality` reads red naming it; with a `METHOD_SURFACE` row removed it reads red as orphaned; `cargo nextest run -p before --all-features` is green with `SURFACE_SOURCES` gone; no prose under crates/before mentions a second extractor or a pincer.

Ruled (78, decision 44): the workspace gets one typed collection authority. before's line-scan extractor retires in favor of surfacecheck (suanpan keeps the `syn`-based extractor ruling 47 lands in `p2-surface`); `tools/citecheck` becomes a typed Rust checker and the collection authority for the superlinear, inverted-twin, and band rosters, so an `#[ignore]`d kernel reads as failing; the fabricated-citation check and the shadow guard are reproduced as unit tests of the new checker before any old scanner is deleted. `Copy` on `Leg` and `Exclusion` rides along. Where the Resolution offers an alternative that keeps a hand scanner, that alternative is struck.

### tests-other-24 (medium, verification): ruling 78

Three rosters attest that a test is named, not that it is collected: `#[ignore]` on any kernel, twin, or band keeps every roster green

Resolution: Extend citecheck's citation sources to the two `TRIPWIRE_ROSTER` tables (tests/superlinear_tripwires.rs, tests/verdict_matrix.rs) and to the registry's `Bands::Priced` and `AXIS_BANDS` names (src/meter/registry.rs), with the same extraction floor and spelling-totality guard the existing sources get, and add a self-test fixture per source; keep the in-crate scans as the both-directions membership pins. If extending citecheck is not wanted, the fallback is for each scanner to capture the attribute run above a rostered fn and refuse `#[ignore` and `#[cfg` there, which re-implements the tool's job in three places (tests-other-3 would put it in one). Acceptance: `#[ignore]` on `sequential_meet_reduce_reads_superlinear_on_shade`, on `polarity_flipped_sweep_reads_inverted_through_the_matrix`, or on any registry-cited band test turns `just citecheck` red naming the test; removing it restores green. Construction: Add `#[ignore]` immediately above `fn polarity_flipped_sweep_reads_inverted_through_the_matrix()` (verdict_matrix.rs:1370). `inverted_verdict_tripwires_match_the_committed_roster` still finds the name through `fn_name` and passes; the twin is never executed; `just gate` is green.

Ruled (78, decision 44): the workspace gets one typed collection authority. before's line-scan extractor retires in favor of surfacecheck (suanpan keeps the `syn`-based extractor ruling 47 lands in `p2-surface`); `tools/citecheck` becomes a typed Rust checker and the collection authority for the superlinear, inverted-twin, and band rosters, so an `#[ignore]`d kernel reads as failing; the fabricated-citation check and the shadow guard are reproduced as unit tests of the new checker before any old scanner is deleted. `Copy` on `Leg` and `Exclusion` rides along. Where the Resolution offers an alternative that keeps a hand scanner, that alternative is struck.

### tests-other-3 (medium, simplification): ruling 78

Five hand-rolled source scanners re-implement `surface_scan::test_fns` with divergent rules and universes

Resolution: Add `tests/support/source_scan.rs` (the `#[path]` precedent `fuzz_seed_set.rs` sets) exposing one recursive walker over the crate root that skips any `target` directory (so the tree set is derived, not listed) and one fn-item extractor built on `surface_scan::test_fns` for attribute-gated names plus verdict_matrix's qualifier-aware `fn_name` for declaration lines; port the five callers and delete the local copies and the seven-entry list. Acceptance: `grep -n 'fn scan(' crates/before/tests` returns nothing outside `support/`; renaming any rostered kernel or twin still reads red; a `pub fn x_reads_superlinear_on_y` is rostered; `wasm32-pins/harness/tests/pins.rs` is scanned by the inverted-twin roster; superlinear_tripwires.rs's doc is true of its mechanism.

Ruled (78, decision 44): the workspace gets one typed collection authority. before's line-scan extractor retires in favor of surfacecheck (suanpan keeps the `syn`-based extractor ruling 47 lands in `p2-surface`); `tools/citecheck` becomes a typed Rust checker and the collection authority for the superlinear, inverted-twin, and band rosters, so an `#[ignore]`d kernel reads as failing; the fabricated-citation check and the shadow guard are reproduced as unit tests of the new checker before any old scanner is deleted. `Copy` on `Leg` and `Exclusion` rides along. Where the Resolution offers an alternative that keeps a hand scanner, that alternative is struck.

### meter-registry-tier2-3 (low, verification): ruling 78

Band-name parity is keyed on a naming convention; seven two-point flatness tests escape it

Resolution: either key the scan on the mechanism (collect every `#[test]` fn whose body calls `assert_flat`/`assert_model_flat`) or rename the seven tests into the convention and cite them (the accumulator streams on their families' rows, the id scan pair in `AXIS_BANDS`); state in the module doc that the scan is total over the convention. Acceptance: adding `#[test] fn scratch_touches_flat()` with an `assert_flat` call and no registry citation fails `band_tests_and_registry_citations_stay_paired`. Construction: add to tests/meter.rs `#[test] fn scratch_touches_flat() { let s = comb_run(4_096, 50_000); let l = comb_run(8_192, 100_000); assert_flat("scratch", &s, &l, envelope::COMB_MILLI_PER_DELTA); }` with no registry change; the parity test passes.

Ruled (78, decision 44): the workspace gets one typed collection authority. before's line-scan extractor retires in favor of surfacecheck (suanpan keeps the `syn`-based extractor ruling 47 lands in `p2-surface`); `tools/citecheck` becomes a typed Rust checker and the collection authority for the superlinear, inverted-twin, and band rosters, so an `#[ignore]`d kernel reads as failing; the fabricated-citation check and the shadow guard are reproduced as unit tests of the new checker before any old scanner is deleted. `Copy` on `Leg` and `Exclusion` rides along. Where the Resolution offers an alternative that keeps a hand scanner, that alternative is struck.

### suite-economics-3 (low, simplification): ruling 78

Five hand-rolled source scanners in before/tests beside the surface-scan crate

Resolution: add one directory walker to surface-scan (a `walk_rs_sources(root)` yielding `(PathBuf, String)` per `.rs` file) and compose each pin from it plus `test_fns` or a line filter; `band_test_names` becomes `test_fns(source)` filtered on the band convention. Acceptance: the four pins keep their rosters and pass unchanged; superlinear_tripwires.rs:15 is accurate because the scan is attribute-gated; no `fn scan(` remains under crates/before/tests.

Ruled (78, decision 44): the workspace gets one typed collection authority. before's line-scan extractor retires in favor of surfacecheck (suanpan keeps the `syn`-based extractor ruling 47 lands in `p2-surface`); `tools/citecheck` becomes a typed Rust checker and the collection authority for the superlinear, inverted-twin, and band rosters, so an `#[ignore]`d kernel reads as failing; the fabricated-citation check and the shadow guard are reproduced as unit tests of the new checker before any old scanner is deleted. `Copy` on `Leg` and `Exclusion` rides along. Where the Resolution offers an alternative that keeps a hand scanner, that alternative is struck.

### surface-roster-10 (low, simplification): ruling 78

Three `#[test]`-attribute scanners with two rule sets, in a workspace that created `surface-scan` to have one

Resolution: make `surface_scan::test_fns` the one scanner, adopting this copy's richer rule (comments between attribute and `fn` keep the arming; `fn ` after a qualifier is found via `fn_name` with a word-boundary check) and adding a per-file entry point; rewrite `declared_test_names_by_file` as a directory walk calling it per file and `band_test_names` as `test_fns(source).into_iter().filter(..)`; move the before-side fixture behaviors (doc comment between; `proptest!` property) into surface-scan's tests; reword 159-162 to what is shared. Acceptance: `grep -rn 'starts_with("#\[test\]")\|== "#\[test\]"' crates tools` matches only crates/surface-scan/src/lib.rs; before's coverage tests, suanpan's claims tests, and amp_board_smoke's parity test pass unchanged.

Ruled (78, decision 44): the workspace gets one typed collection authority. before's line-scan extractor retires in favor of surfacecheck (suanpan keeps the `syn`-based extractor ruling 47 lands in `p2-surface`); `tools/citecheck` becomes a typed Rust checker and the collection authority for the superlinear, inverted-twin, and band rosters, so an `#[ignore]`d kernel reads as failing; the fabricated-citation check and the shadow guard are reproduced as unit tests of the new checker before any old scanner is deleted. `Copy` on `Leg` and `Exclusion` rides along. Where the Resolution offers an alternative that keeps a hand scanner, that alternative is struck.

### surface-roster-11 (low, verification): ruling 78

`#[ignore]`d tests satisfy the in-tree citation checks, including the `GridCap` guard that "must run"; citecheck closes the hole for before, not for suanpan

Resolution: in the (ideally single, per surface-roster-10) witness scanner, note `#[ignore` while armed and either drop the name or return ignored names separately; have the citation tests reject a cited ignored name; change the surface-scan fixture so `attr_between` is asserted ignored rather than admitted; reword surface_coverage.rs:296-298 and tests.rs:183-185 to what the in-tree scan attests (attribution) and name citecheck as the collection authority for before. Acceptance: the construction below reads red in `exclusion_payload_citations_resolve` (or a new `no_cited_test_is_ignored`); the surface-scan fixture change is red-then-green. Construction: insert `#[ignore = "probe"]` between `#[test]` (semantic_oracle/tests.rs:550) and `fn grid_cap_is_never_reached()` (551). `cargo nextest run -p before` (the in-tree suite alone) stays green while the guard never runs; only `just citecheck` reads red.

Ruled (78, decision 44): the workspace gets one typed collection authority. before's line-scan extractor retires in favor of surfacecheck (suanpan keeps the `syn`-based extractor ruling 47 lands in `p2-surface`); `tools/citecheck` becomes a typed Rust checker and the collection authority for the superlinear, inverted-twin, and band rosters, so an `#[ignore]`d kernel reads as failing; the fabricated-citation check and the shadow guard are reproduced as unit tests of the new checker before any old scanner is deleted. `Copy` on `Leg` and `Exclusion` rides along. Where the Resolution offers an alternative that keeps a hand scanner, that alternative is struck.

### surface-roster-31 (low, simplification): ruling 78

citecheck re-parses the typed roster lexically with its own extraction guards, and its rejected-alternative note omits the external-binary route surfacecheck already demonstrates

Resolution: a second surfacecheck subcommand (or sibling binary) that iterates `METHOD_SURFACE`/`FAMILY_SURFACE` and `laws::registered_names`, reads the `cargo nextest list --message-format json` artifact the recipe hands it, and keeps the fabricated-citation tripwire and the shadow guard; move `TRIPWIRES` and `DIFF_BESPOKE` (or their name lists) under `meter`; reproduce citecheck's self-test red paths as unit tests; then retire `tools/citecheck`. If the owner keeps the tools/ route, add the omitted option to the note with the reason it was not taken. Acceptance: the justfile's `citecheck` recipe invokes the Rust checker and `tools/citecheck` is deleted; or the note records the third option.

Ruled (78, decision 44): the workspace gets one typed collection authority. before's line-scan extractor retires in favor of surfacecheck (suanpan keeps the `syn`-based extractor ruling 47 lands in `p2-surface`); `tools/citecheck` becomes a typed Rust checker and the collection authority for the superlinear, inverted-twin, and band rosters, so an `#[ignore]`d kernel reads as failing; the fabricated-citation check and the shadow guard are reproduced as unit tests of the new checker before any old scanner is deleted. `Copy` on `Leg` and `Exclusion` rides along. Where the Resolution offers an alternative that keeps a hand scanner, that alternative is struck.

### surface-roster-5 (low, simplification): ruling 78

Forty rows spell one exclusion twice; a `Copy` derive and a `law_row` helper state the disposition once per row

Resolution: `#[derive(Debug, Clone, Copy)]` on `Leg` and `Exclusion`; `const fn law_row(op: &'static str, law: &'static str, fs: Exclusion) -> SurfaceRow`; named consts for the recurring empty exclusions (`DEFINITIONAL`, `LINEARITY`, `NO_WIRE_FORMAT`); rewrite the forty rows as one-liners. In the same change, extend `tools/citecheck`'s `extract_legs` to classify the helper's `Leg::Law(law)` body as a binding and to extract the law literal from `law_row("op", "law", ..)` call sites, with a `--self-test` fixture for each. Acceptance: every surface_coverage test and `just citecheck` green, with citecheck's leg count unchanged (40 law citations still extracted); the file shrinks by roughly 120 lines.

Ruled (78, decision 44): the workspace gets one typed collection authority. before's line-scan extractor retires in favor of surfacecheck (suanpan keeps the `syn`-based extractor ruling 47 lands in `p2-surface`); `tools/citecheck` becomes a typed Rust checker and the collection authority for the superlinear, inverted-twin, and band rosters, so an `#[ignore]`d kernel reads as failing; the fabricated-citation check and the shadow guard are reproduced as unit tests of the new checker before any old scanner is deleted. `Copy` on `Leg` and `Exclusion` rides along. Where the Resolution offers an alternative that keeps a hand scanner, that alternative is struck.

### tools-13 (low, simplification): ruling 78

citecheck carries three string-literal-aware scanners with three escape conventions

Resolution: one `code_spans(text)` generator that skips string and char literals and both comment forms and yields code text with offsets; `strip_line_comments`, `top_level_entries`, and the `macro_block_fn_names` brace walk consume it. Acceptance: one lexer function; the existing fixtures (the quoted-brace descriptor at 481-486 included) still pass.

Ruled (78, decision 44): the workspace gets one typed collection authority. before's line-scan extractor retires in favor of surfacecheck (suanpan keeps the `syn`-based extractor ruling 47 lands in `p2-surface`); `tools/citecheck` becomes a typed Rust checker and the collection authority for the superlinear, inverted-twin, and band rosters, so an `#[ignore]`d kernel reads as failing; the fabricated-citation check and the shadow guard are reproduced as unit tests of the new checker before any old scanner is deleted. `Copy` on `Leg` and `Exclusion` rides along. Where the Resolution offers an alternative that keeps a hand scanner, that alternative is struck.

### tools-20 (low, simplification): ruling 80

doclint's summary-length rule reimplements clippy's `too_long_first_doc_paragraph`

Resolution: owner check: enable `-W clippy::too_long_first_doc_paragraph` on the root and detached-workspace clippy legs, confirm it fires on doclint's summary fixtures, then drop rule 1 (keeping rule 2 and its self-test). If the lint's nursery grade, its compiled-cfg coverage, or its threshold is judged insufficient, record that at MAX_SUMMARY_CHARS as the reason the in-house rule exists. Acceptance: either `just clippy` fails on a 250-character first paragraph and doclint no longer measures summaries, or the constant's comment states why clippy's lint does not suffice.

Ruled (80, decision 79): rule 1 is replaced by clippy's `too_long_first_doc_paragraph`, the fixtures confirming the lint fires first; rule 2 stays.

## Roster members approved (ruling 104)

Lows and nits approved as this lane's roster by ruling 104, placed here by the files they touch and the kind of dissolution. Land each per its quoted Resolution and Acceptance, swept with the ruled members; report rather than choose if a Resolution conflicts with a ruling or offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89).

### gate-legs-10 (low, simplification): roster: approved (ruling 104)

doclint sweeps in-tree build outputs; only two of five detached workspaces redirect their target dirs to dodge it

Resolution: Exclude `target`, `.git`, and `node_modules` in doclint's walk exactly as testdoc does, with a self-test fixture for the skip; then the two `.cargo/config.toml` redirects lose their stated reason and can be dissolved or re-justified at the file. Acceptance: doclint's self-test pins that a `.rs` under a `target/` directory is not visited, and no `.cargo/config.toml` cites doclint as its reason.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names and the retirement discipline. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

