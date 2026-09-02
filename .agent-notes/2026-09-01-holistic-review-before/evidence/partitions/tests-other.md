# Partition tests-other: The other integration suites: verdict matrix, board smoke, answer embedded, bench-judge roster, coincident span, doc hidden, fold skeleton, foreign reexport, forks max, fuzz seeds, stale state, superlinear tripwires

## Partition summary

The partition is thirteen files of instrument code and no production code: twelve integration-test binaries under `crates/before/tests/` and one `#[path]`-shared support module (`tests/support/fuzz_seed_set.rs`, the seed derivation that the `fuzz_seeds` example writes and the `fuzz_seeds` test byte-compares). They fall into three genres. The cross-kernel and resource pins run the library and judge what it does: `verdict_matrix.rs` (1530 lines) cross-checks every public answerer of the causal-relation question over a roster-derived operand pool, with liveness floors and two committed mutant twins; `coincident_span.rs` holds the `Span` clone-identity rungs live by scan parity; `fold_skeleton.rs` and `answer_embedded.rs` are two flatness criteria on populations the board does not drive; `amp_board_smoke.rs` runs the amplification board at a tiny scale and carries the merge's committed known-bad artifact; `forks_max.rs` and `stale_state.rs` pin boundary and model behavior. The roster and tamper pins scan source text: `superlinear_tripwires.rs` and `verdict_matrix.rs`'s own `_reads_inverted` roster hold the adequacy kernels by name, `doc_hidden.rs` and `foreign_reexport.rs` close two channels the surface-totality checks cannot see, and `bench_judge_roster.rs` pins the bench judge's expectation data. The corpus pins (`fuzz_seeds.rs` with the support module) hold the committed fuzz seeds to one derivation and to the contracts of four of the five fuzz targets.

Quality is good where the doctrine's order was followed literally. The verdict matrix derives its pool from an exhaustive match over `FamilyId`, asserts a budget derived from the roster count, demonstrates its verdict-class floors red on degenerate pools, and rosters by name the legs no mutant can reach. The board smoke's `merge_refuses_a_silently_shrunk_grid_for_every_family` is a real known-bad artifact swept over the axis the refusal discriminates on. `coincident_span.rs` asserts "a zero is a dead meter" before every comparison and documents why one leg pins divergence rather than direction. The seed derivation is a model of one definition feeding two consumers, with bit-level derivations beside every hand-authored non-canonical byte.

The defects cluster in four places. First, the two flatness criteria that landed together (`fold_skeleton.rs`, `answer_embedded.rs`) have no liveness floor: `growth()` returns 1.0 on a zero base counter, and the refutation pass's run shows the limb leg of the hull fold reading zero at both levels and passing; neither criterion has a committed known-bad kernel and neither is in any roster. Second, the text scanners are five hand-rolled variants of one algorithm the workspace already owns (`surface_scan::test_fns`), and their differences are live costs: the superlinear roster's scanner misses `pub fn` and cannot see `#[ignore]`, the inverted-twin roster's tree list already omits `wasm32-pins/`, the foreign re-export pin misses `pub use <dep>;`, and the doc-hidden pin counts one literal spelling. Third, the corpus gate holds the `fuzz_laws` framing by transcribed constants and the `fuzz_decode_ops` seeds by byte identity only, and the derivation's own comments describe a "concurrent" sibling version that `Clock::sync` has already made equal. Fourth, two instruments under-deliver what their docs claim: `weave_pair()` in the verdict matrix collapses by ITC normal form to `scatter_pair()`, so the Weave family contributes nothing to the pool, and the polarity-flipped twin is pinned per axis while the landing commit says both twins pin named legs. A tail of documentation drift (public `forks` docs silent or wrong at the boundary `forks_max.rs` pins, a "197 items" incident count, an unanchored "pincer/jaw" metaphor, "mint") rounds out the list.

Lines read: the thirteen partition files in full (3823 lines), plus the cross-reference sites cited under each finding (span.rs, clock.rs, party.rs, party/forks.rs, the two fuzz targets, surfacecheck/src/extract.rs and check.rs, meter/registry.rs, meter/board.rs and its family, worst, currency, and render modules, tests/meter.rs and src/meter.rs conventions, surface-scan/src/lib.rs, tools/citecheck, tools/benchjudge and its roster, benches/common/sidecar.rs, the justfile, and the validation index). Every file in the partition is test or instrument code; `tests/support/fuzz_seed_set.rs` is included by `#[path]` from both a test and an example.

## Findings

### tests-other-1: The validation index omits every instrument in this partition but the bench-judge roster
- Where: crates/before/src/testing/validation_index.rs:1-3 (related: crates/before/tests/verdict_matrix.rs:1-9, crates/before/tests/coincident_span.rs:1-11, crates/before/tests/superlinear_tripwires.rs:1-16)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (per-stem grep over validation_index.rs: only `bench_judge_roster` matches, at line 113); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no rationale found (the index's last edit, ba124e1d on 2026-08-12, predates the verdict matrix; earlier edits were targeted re-denominations, not totality passes)
- Owner-gated: no

The index promises "every instrument that guards this crate" and sets the bar for a new instrument as "a failure class no row below already catches", but of the twelve test binaries in this partition only `bench_judge_roster` is named; the verdict matrix (a semantic instrument with a stated failure class), the coincident-rung witnesses, the two flatness criteria, and the four roster pins have no row. Principle 3's own procedure cannot be applied to instruments the map does not list.

Evidence:

         1	//! The validation index: every instrument that guards this crate, what
         2	//! failure class each one catches that the others cannot, and where it
         3	//! lives.

Resolution: Add a semantic row for the verdict matrix (its class: a kernel-local verdict inversion on adversarial shapes the law populations under-hit; the twins as adequacy), a resource row for the deep-skeleton and answer-embedded criteria, and a short paragraph on the roster pins (`_reads_superlinear`, `_reads_inverted`, doc-hidden, foreign re-export, or their successors under tests-other-13 and tests-other-16) and the coincident-rung witnesses. Acceptance: every `crates/before/tests/*.rs` binary is named in the index or covered by a genre row that names its file.

### tests-other-2: The worst-map smoke pin transcribes the currency axis as four string literals and `rows == 4`
- Where: crates/before/tests/amp_board_smoke.rs:185-210 (related: crates/before/tests/amp_board_smoke.rs:167-170, crates/before/src/meter/board/worst.rs:82-89, crates/before/src/meter/board/currency.rs:29-40, crates/before/src/meter/board.rs:292-300)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read worst.rs:84-89 `const MAP_CURRENCIES: [Currency; 4]`, private; board.rs:300 re-exports only `NEAR_TIE_RATIO, WORST_MAP_SCALES` from `worst`; currency.rs:29-40 has five variants); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed, severity medium -> low (the const is meter-feature surface and the drift needs a deliberate edit to worst.rs); history: no rationale found (493ef543 landed the private axis and the hard-coded test together)
- Owner-gated: yes: the fix re-exports a private board constant under the `meter` feature

The filter and the count are copies of `worst.rs`'s private `MAP_CURRENCIES` (four of the five `Currency` variants; `Segments` is deliberately excluded there). A currency added to the map is filtered out by the `matches!` while `rows == 4` stays true, and a relabeled one fails for the wrong reason. Principle 5: a number that matters lives in a mechanically-enforced place the test can cite.

Evidence:

       191	        if marker != "worst" || !matches!(currency, "heap" | "limb" | "scan" | "touch") {
       192	            continue;
       193	        }
       ...
       207	    assert!(
       208	        per_op.values().all(|&rows| rows == 4),
       209	        "every operation renders one row per mapped currency: {per_op:?}"
       210	    );

Resolution: Make `worst::MAP_CURRENCIES` `pub` and re-export it from `meter::board` beside `NEAR_TIE_RATIO`; build the accepted label set from `MAP_CURRENCIES.iter().map(Currency::label)` and assert `rows == MAP_CURRENCIES.len()`; drop the parenthetical list at line 169. Acceptance: no currency label literal and no literal `4` remain in `worst_map_covers_every_operation_row`; temporarily adding `Currency::Segments` to `MAP_CURRENCIES` fails the test and restoring passes.

### tests-other-3: Five hand-rolled source scanners re-implement `surface_scan::test_fns` with divergent rules and universes
- Where: crates/before/tests/amp_board_smoke.rs:314-340 (related: crates/before/tests/superlinear_tripwires.rs:14-16, crates/before/tests/superlinear_tripwires.rs:72-98, crates/before/tests/superlinear_tripwires.rs:112-113, crates/before/tests/verdict_matrix.rs:1232-1287, crates/before/tests/verdict_matrix.rs:1438-1448, crates/before/tests/doc_hidden.rs:25-43, crates/before/tests/foreign_reexport.rs:62-99, crates/surface-scan/src/lib.rs:174-196, crates/before/Cargo.toml:50)
- Class / severity / confidence: scaffolding / medium / high
- Provenance: verified (read surface-scan/src/lib.rs:174-196; `surface-scan` is a dev-dependency at before/Cargo.toml:50 with consumers in surface_coverage.rs and suanpan's claims tests, none under crates/before/tests; `find crates/before/wasm32-pins -name '*.rs'` returns three files absent from the seven-entry tree list; Python replica of superlinear_tripwires.rs:80-87 returns None for `pub fn x_reads_superlinear_on_y()` and matches a non-test `fn helper_reads_superlinear_kernel()`); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed (with the reframe that the two rosters look for different markers, so the divergence is of universes: a `_reads_superlinear` fn in `benches/` or `examples/` and a `_reads_inverted` fn in `wasm32-pins/` are each invisible to their roster); history: no rationale found (fcb78e44's qualifier-aware, `target`-skipping fix landed in verdict_matrix.rs only; the seven-tree list was complete on 2026-08-13 and rotted on 2026-08-18 when eb6ba627 added wasm32-pins)
- Owner-gated: no

`band_test_names` is line-for-line `surface_scan::test_fns` plus a name filter, and four further recursive directory walkers with three different "what is a declared fn" rules live in this partition. The divergence has a live cost: superlinear_tripwires.rs's doc promises to match "the `#[test]` fns" but its scanner matches any bare `fn ` line, misses `pub fn`, and counts a non-test helper; verdict_matrix.rs's tree list claims "Every source tree the crate carries" and omits `wasm32-pins/`. Principle 3: infrastructure that reimplements a capability the workspace already owns and generates its own maintenance cascade (every fix landed in one copy).

Evidence:

       314	fn band_test_names(source: &str) -> BTreeSet<String> {
       315	    let mut names = BTreeSet::new();
       316	    let mut armed = false;
       317	    for line in source.lines() {
       318	        let t = line.trim();
       319	        if t == "#[test]" {

    (superlinear_tripwires.rs:14-16)
        14	//! decoration. This roster is the missing jaw: the committed list below
        15	//! must match the `#[test]` fns whose names carry `_reads_superlinear`,
        16	//! in both directions.

    (superlinear_tripwires.rs:80)
        80	                let Some(rest) = line.trim_start().strip_prefix("fn ") else {

    (verdict_matrix.rs:1438-1448)
      1438	    // Every source tree the crate carries: a twin declared in a side
      1439	    // tree must not escape the roster.
      1440	    for tree in [
      1441	        "src",
      1442	        "tests",
      1443	        "benches",
      1444	        "examples",
      1445	        "fuzz",
      1446	        "fuzzfit",
      1447	        "surfacecheck",
      1448	    ] {

Resolution: Add `tests/support/source_scan.rs` (the `#[path]` precedent `fuzz_seed_set.rs` sets) exposing one recursive walker over the crate root that skips any `target` directory (so the tree set is derived, not listed) and one fn-item extractor built on `surface_scan::test_fns` for attribute-gated names plus verdict_matrix's qualifier-aware `fn_name` for declaration lines; port the five callers and delete the local copies and the seven-entry list. Acceptance: `grep -n 'fn scan(' crates/before/tests` returns nothing outside `support/`; renaming any rostered kernel or twin still reads red; a `pub fn x_reads_superlinear_on_y` is rostered; `wasm32-pins/harness/tests/pins.rs` is scanned by the inverted-twin roster; superlinear_tripwires.rs's doc is true of its mechanism.

### tests-other-4: "answer-embedded" names two different claims
- Where: crates/before/tests/answer_embedded.rs:1-2 (related: crates/before/src/testing/asymptotics.rs:25-26, crates/before/src/meter.rs:2422, crates/before/src/version/skyline/query.rs:122)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (grep of `answer-embedded|answer_embedded` over crates/before/src: asymptotics.rs, meter.rs:2422, query.rs:122, integral.rs:248, query/tests.rs all use it for the rank's wide x dense value structure behind the Omega(M) floors); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no rationale found (e695d5cf coined the product meaning on 2026-07-28; the test file inherited its name from the r141 audit branch three days later)
- Owner-gated: no

This file uses "answer-embedded" for a cost-coupling claim (no `n x w` product in `min_ticks`/`rank` on wide-base tiny-tail inputs); `asymptotics.rs`, the registry, and the `answer_embedded_product` band use it for a value-structure claim. A coined term is anchored once; two anchors send a reader of the validation index to the wrong instrument.

Evidence:

         1	//! Answer-embedded shapes for the measure folds (`min_ticks`, `rank`).

Resolution: Rename the file and its doc to the mechanism it attacks ("wide-base tiny-tail and wide-ladder folds", or "answer-width coupling"), leaving "answer-embedded product" to the multiplication-bound claims. Acceptance: `grep -rn 'answer-embedded' crates/before` resolves to one meaning.

### tests-other-5: Generator and support docs disagree with their code in three small places
- Where: crates/before/tests/answer_embedded.rs:9-11 (related: crates/before/tests/answer_embedded.rs:68-69, crates/before/tests/answer_embedded.rs:77-79, crates/before/tests/support/fuzz_seed_set.rs:31-35, crates/before/tests/amp_board_smoke.rs:137-138)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read); executed: no
- Seen by: instrument-correctness (answer_embedded, seed_set), instrument-correctness (amp_board_smoke isolation wording); refutation: confirmed; history: the `seed_set` doc was complete when written (bb6ea8b7 seeded only two targets) and expired when 778c89f5 and 23e46a7c added three more; the `step_by(2)` mismatch is as-born in 4398dcd4
- Owner-gated: no

Three docs state something their code does not do. `wt`'s docs say `n` forked parties tick once each, but the loop ticks every other party (`step_by(2)`), so `n/2` ticks land on `n` leaves. `seed_set`'s doc enumerates the `fuzz_decode` and `fuzz_decode_ops` seeds as if they were the corpus, while the function derives seeds for five targets. `shard_protocol_round_trips` says every judged quantity is a counter "over state each shard owns privately", but the in-process spawner runs every shard in one process against one global `PeakAlloc`; byte-identity holds because each shard resets the peak, not because the state is private. Every test doc states its invariant in English and must be accurate.

Evidence:

         9	//! - **wide-base tiny-tail** `WT(n, w)`: one `ticks(seed, 10^w)` base
        10	//!   raise over the whole id space, then `n` forked parties tick once
        11	//!   each on alternating leaves. Every leaf height and every subtree
       ...
        77	    for p in parties.iter().step_by(2) {
        78	        v.tick(p);
        79	    }

    (fuzz_seed_set.rs:31-35)
        31	/// The `fuzz_decode` seeds are canonical encodings of a small family of
        32	/// known values (the seed clock, a forked pair, split parties, a nested
        33	/// version); the `fuzz_decode_ops` seeds are decode-then-operate scripts
        34	/// in that target's framing (flavour byte, length-prefixed value bytes,
        35	/// one op per trailing byte). Deterministic: no randomness, no clocks.

    (amp_board_smoke.rs:137-138)
       137	/// byte-identity is how it asserts, since every judged quantity is a
       138	/// deterministic counter over state each shard owns privately. Three

Resolution: answer_embedded.rs:9-11 and 68-69: "then the even-indexed of `n` forked leaves tick once" (or tick every leaf and re-derive the grid). fuzz_seed_set.rs:31-35: describe the corpus by genre for all five targets, or name none. amp_board_smoke.rs:137-138: "a deterministic counter reset per shard, under one-scenario-per-process isolation". Acceptance: each doc sentence is true of the code beneath it.

### tests-other-6: The two flatness criteria outside `tests/meter.rs` have no committed known-bad demonstration and sit outside every roster
- Where: crates/before/tests/answer_embedded.rs:19-23 (related: crates/before/tests/answer_embedded.rs:109-121, crates/before/tests/fold_skeleton.rs:40-48, crates/before/tests/amp_board_smoke.rs:331, crates/before/tests/amp_board_smoke.rs:357, crates/before/tests/superlinear_tripwires.rs:6-16, crates/before/src/meter/registry.rs:46-59)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep of `wide_base|wide ladder|tiny_tail|deep skeleton|mixed second` over crates/before/src and tests excluding the two files returns nothing; the band parity scan reads only `tests/meter.rs` (amp_board_smoke.rs:357) and matches only `_is_flat_per_unit`/`_band` names (line 331), which neither test name carries); executed: no
- Seen by: structure-prose (open question), adequacy, instrument-correctness; refutation: reframed (the registry's "coverage by bands alone" rule at registry.rs:54-59 governs families, and these populations are not families, so they sit outside that rule rather than in breach of it; the gap is the missing known-bad demonstration and the missing roster answer); history: the closed-form quarter-versus-tenth argument is inline and deliberate (4398dcd4); the "tripwire" vocabulary is consistent with the crate's "improvement tripwire" usage (tests/meter.rs:42-46, twenty occurrences), so the word is not misused; no ruling records that WT/WL/deep-skeleton may stay unregistered
- Owner-gated: yes: the cure is either registration as `Shape`s with band citations (a registry change) or an owner ruling stated at each site

The crate's own band discipline (superlinear_tripwires.rs:6-9: each flatness band's adequacy rests on a committed kernel that still reads red through the band's own meters) and Principle 2 (every criterion needs a committed demonstration that a known-bad mechanism fails it) require a demonstration these two criteria lack: nothing in the `_reads_superlinear` roster or beside these files drives a product-law or per-leaf-re-touch fold over the WT/WL grid or the deep skeleton. The analytical separation (a product law reads a quarter; the bound is a tenth) is an argument, not a committed demonstration, and the refutation run shows the mixed difference is exactly 0 on every leg, so the 0.10 bound has never been approached. The tests' names also escape the registry's band parity scan, so no family answers for them.

Evidence:

        19	//! The tripwire is the *mixed second difference* over a 2x2 (n, w)
        20	//! grid: for an additive cost `a*n + b*w` it vanishes; for a product
        21	//! law `c*n*w` it is a quarter of the top cell. Bounding it at a tenth
        22	//! of the top cell refutes any n x w coupling while tolerating
        23	//! amortization wobble.

    (superlinear_tripwires.rs:6-9)
         6	//! Each flatness band's adequacy rests on a committed kernel that
         7	//! demonstrates the refuted mechanism (absolute-position accounting, a
         8	//! schoolbook settle, a sequential reduce, ...) still reads red through
         9	//! the band's own meters — instruments-before-cures, held forever. But

Resolution: Owner call between two shapes. (a) Register WT, WL, and the forked deep spine as `Shape`s with `Bands` answers, move the three criteria into `tests/meter.rs` under the band naming convention so the parity scan sees them, and commit one kernel per criterion under the superlinear roster: for WT the schoolbook settle already driven by `schoolbook_run` in query/tests.rs, for WL a per-rung re-densification, for the D leg a per-frame path-sum fold (the mechanism `bigroot`'s doc names). (b) Keep them bespoke and state at each assertion that its adequacy rests on the closed-form separation, with the measured margin (mixed 0 against a bound of a tenth) recorded as the reason no kernel is committed. Acceptance: (a) `band_tests_and_registry_citations_stay_paired` cites the three bands and `superlinear_tripwires_match_the_committed_roster` rosters one kernel per band, each reading red through the band's meters; or (b) the ruling is stated at the site.
Construction: grep `answer_embedded|wide_base|wide ladder|skeleton` across registry.rs and tests/meter.rs: the only hits are the unrelated `answer_embedded_product` band and memo-chain skeleton prose; no `FamilyId` or `Shape` corresponds to WT, WL, or the forked deep spine, and no kernel is run over them.

### tests-other-7: Duplicated meter and fixture helpers across the metered binaries, with the isolation premise stated in none of them
- Where: crates/before/tests/answer_embedded.rs:31-61 (related: crates/before/tests/fold_skeleton.rs:20-30, crates/before/tests/fold_skeleton.rs:51-59, crates/before/tests/coincident_span.rs:20-24, crates/before/tests/coincident_span.rs:31-38, crates/before/tests/coincident_span.rs:146-153, crates/before/tests/meter.rs:9479, crates/before/tests/meter.rs:354-355, crates/before/src/meter.rs:3618-3630, crates/before/src/meter/board/family.rs:836-845, crates/before/src/party.rs:251-252)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (grep: `fn counters` at answer_embedded.rs:31 and fold_skeleton.rs:20, byte-identical; `fn scanned` at coincident_span.rs:20 and four sites in tests/meter.rs; the balanced-fork loop at answer_embedded.rs:49-61, fold_skeleton.rs:51-59, family.rs:836-845; the `rounds` closure twice in coincident_span.rs); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no rationale found; the isolation premise is documented at the facade (src/meter.rs:3619-3620) and the index (validation_index.rs:97-99) but not at these reading sites (deliberate-and-holds for the premise, not for the omission)
- Owner-gated: no

`counters` is byte-identical in two files, `scanned` is defined five times, the power-of-two fork tiling is hand-rolled three times (the public `Party::forks` and `From<Party> for [Party; N]` already provide it), and `rounds` is duplicated inside one file. `counters` also bypasses the `meter` facade (`meter::touch_ops`/`reset_touch_ops`) for `suanpan::touch_meter` directly, so these suites never meet the sentence that states the process-global isolation premise, and unlike `tests/meter.rs` they append no `ISOLATION_NOTE` to a failure. The duplicates are where the liveness-floor omission (tests-other-14) crept in independently of `coincident_span.rs`'s correct pattern.

Evidence:

        31	fn counters(f: impl FnOnce()) -> (u64, u64, u64) {
        32	    meter::reset_scan_bits();
        33	    meter::reset_limb_ops();
        34	    suanpan::touch_meter::reset();
        35	    f();
        36	    (
        37	        meter::scan_bits(),
        38	        meter::limb_ops(),
        39	        suanpan::touch_meter::touches(),
        40	    )
        41	}
       ...
        49	fn fork_parties_from(seed: Party, n: usize) -> Vec<Party> {
        50	    let mut parties = vec![seed];
        51	    while parties.len() < n {

Resolution: Add `tests/support/meters.rs` with a `Counters { scan, limb, touch }` struct, `counters(f)` routed through `meter::*` and carrying the dead-meter floor and the isolation note, `scanned(f)`, and `balanced_forks(n)`; include it by `#[path]` from the four binaries (and offer it to tests/meter.rs); hoist `rounds` into `fixture()` in coincident_span.rs. Replace the tiling loops with `<[Party; 16]>::from(Party::seed())` / `seed.forks(n - 1)` only after confirming the leaf order the `step_by(2)` alternation in `wt` depends on (see the open questions). Acceptance: one definition each of `counters` and `scanned` under `tests/support/`; no `while parties.len() < n` loop in the two files; MEASURED grids unchanged against the parent, or a changed grid explained by the leaf-order difference.

### tests-other-8: Register transplants and dash register across the partition
- Where: crates/before/tests/bench_judge_roster.rs:7-8 (related: crates/before/tests/bench_judge_roster.rs:57-58, crates/before/tests/bench_judge_roster.rs:118, crates/before/tests/amp_board_smoke.rs:213, crates/before/tests/amp_board_smoke.rs:231, crates/before/tests/answer_embedded.rs:3, crates/before/tests/answer_embedded.rs:116-120, crates/before/tests/answer_embedded.rs:162, crates/before/tests/superlinear_tripwires.rs:4, crates/before/tests/superlinear_tripwires.rs:24, crates/before/tests/verdict_matrix.rs:719, crates/before/tests/verdict_matrix.rs:889, crates/before/tests/verdict_matrix.rs:892, crates/before/tests/verdict_matrix.rs:988-989, crates/before/tests/verdict_matrix.rs:1398, crates/before/tests/coincident_span.rs:47, crates/before/tests/forks_max.rs:18, crates/before/tests/fuzz_seeds.rs:110, crates/before/tests/fuzz_seeds.rs:251, crates/before/tests/fuzz_seeds.rs:387, crates/before/tests/support/fuzz_seed_set.rs:109-110, crates/before/tests/support/fuzz_seed_set.rs:301-305, crates/before/tests/support/fuzz_seed_set.rs:339)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep over the thirteen files for the listed words and for em/en dashes on non-`///`/`//!` lines; `attacker` does not occur in crates/before/src; `honest` occurs on 146 lines of crates/before/src and em-dash `//` comments on 374 src lines, so the pattern is crate-wide); executed: no
- Seen by: structure-prose; refutation: confirmed (the "tripwire" clause disputed: the crate names measured bands "improvement tripwire", tests/meter.rs:42-46); history: no repo rule governs these words or the dash register; crate-wide dialect
- Owner-gated: yes: a consistent fix is a crate-wide sweep, not a per-file edit

"launder" imports an economy where a misclassified cell is the mechanism; "honest" names an untampered capture, a cell, a check, and a band; "genuine(ly)"/"real(ly)" carry no mechanism; "earns a reviewed row" and "the differential family's charge" are duty transplants; "attacker-controlled knob" is the one "attacker" in the crate, whose own word is "adversarial". Em-dashes appear in an assert message (answer_embedded.rs:118) and an en-dash in another (fuzz_seeds.rs:387, "merged–merged"), and in `//` comments at the listed sites; the owner's register rule wants colons or spaced double-hyphens there for terminal output. Each costs a reader a translation back to the mechanism.

Evidence:

         7	//! data a one-line edit could quietly reshape — un-rostering an owned red,
         8	//! widening the text class to launder a superlinear cell — so this suite

    (answer_embedded.rs:116-119)
       116	    assert!(
       117	        mixed.abs() <= bound,
       118	        "{name}/{op}/{label}: mixed second difference {mixed:.0} exceeds {bound:.0} — \
       119	         an n x w product term in an answer-embedded fold"

Resolution: "launder" -> "misclassify as expected"; `honest` -> `untampered`/`intact`; drop "genuine(ly)/real(ly)" or state the mechanism; "earns a reviewed row" -> "gets a reviewed row"; "charge" -> "is covered by"; "attacker-controlled" -> "adversarial"; colons or ` -- ` in the two assert messages and the `//` comments, and "merged-merged" (ASCII hyphen) for the compound adjective. Batch with the crate-wide sweep. Acceptance: the greps return nothing, or every hit names a mechanism in its sentence.

### tests-other-9: `wide_display_pair_expectations_are_split` is a strict consequence of the red-membership pin
- Where: crates/before/tests/bench_judge_roster.rs:124-129 (related: crates/before/tests/bench_judge_roster.rs:60-63, crates/before/tests/bench_judge_roster.rs:118-123)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (read: line 62 pins the red class to exactly `["display_schoolbook/hugeleaf"]`, which entails both assertions at 127-128); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate-but-expired (at 7542d613 the red set held sixteen bigroot cells plus the schoolbook cell and a `boundary` class existed; c95230c8 shrank the red set to one cell, which is when this test became implied)
- Owner-gated: no

A test that cannot fail while its sibling passes names nothing it alone catches (Principle 3) and costs a reader a comparison to discover that. The rationale in its doc (the pair separates the conversion classes only while one member is required red and the other green) is worth keeping, on the pin that enforces it.

Evidence:

        61	fn roster_red_membership_is_pinned() {
        62	    assert_eq!(class(&roster(), "red"), ["display_schoolbook/hugeleaf"]);
        63	}
       ...
       124	#[test]
       125	fn wide_display_pair_expectations_are_split() {
       126	    let red = class(&roster(), "red");
       127	    assert!(red.contains(&"display_schoolbook/hugeleaf".to_string()));
       128	    assert!(!red.contains(&"version_display_wide/hugeleaf".to_string()));
       129	}

Resolution: Delete `wide_display_pair_expectations_are_split` and fold its two-sentence rationale into `roster_red_membership_is_pinned`'s doc. (The verbatim `TEXT_CEILING_CELLS` pin at 103-116 is deliberate tamper-evidence per the module doc and stays; a semantic predicate over the cell IDs would be an optional addition.) Acceptance: three tests remain, each failable by an edit that leaves the others green.

### tests-other-10: `coincident_span.rs` pins two of the four clone-identity rungs; the precedence and `contains`-receiver rungs are unpinned anywhere
- Where: crates/before/tests/coincident_span.rs:1-11 (related: crates/before/src/span.rs:255, crates/before/src/span.rs:315, crates/before/src/span.rs:380, crates/before/src/span.rs:453-460, crates/before/tests/meter.rs:9969-9979, crates/before/tests/meter.rs:10552, crates/before/src/span/tests.rs:823-835)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep `ptr_eq` in span.rs: receiver rungs at 255 (place), 315 (dominance), 380 (precedence), 459 (contains receiver), argument rung at 453; this file scans place (63-78), dominance (100-115), and the argument door (166-180) only; meter.rs:9969-9979 measures `precedence` on proper spans `Span::new(&s, &div)`; the `identity_fast_paths` module from 10552 has no `.place(`/`.dominance(`/`.precedence(`/`Span::at(` call; span/tests.rs:823-835 asserts verdict equality with no scan read; `git blame`: span.rs:380 -> b3f09baa (2026-08-06), span.rs:459 -> 22cdfbe1 (2026-08-17); this file's rung witnesses date to 47b03e89 (2026-07-30)); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no rationale found (b3f09baa is a WIP docs-pass commit with no body; 22cdfbe1's message claims scan-meter rungs prove the fast paths but its diff adds only `coincident_argument_collapses_to_the_membership_walk`; 47b03e89 states the failure class, `Bits::ptr_eq -> always false` passing the entire meter binary, which applies to the two later rungs unchanged)
- Owner-gated: no

The module doc claims to hold "the coincident-span clone-identity rungs" live, but two rungs added since the file landed have no scan-parity pin: a `ptr_eq` rung deleted at span.rs:380 or 459 leaves every verdict correct through the general arm, so the verdict matrix's `coincident precedence` leg (verdict_matrix.rs:1017) and every law stay green. Principle 6: the cheapest passing artifact (the rung removed) passes the gate today.

Evidence:

         1	//! The coincident-span clone-identity rungs, held live by scan parity.
         2	//!
         3	//! `Span::place` and `Span::dominance` on a clone-coincident span must
         4	//! read exactly the scan bits of the collapsed form they document

    (span.rs:380)
       380	        if self.lo.view().ptr_eq(self.hi.view()) {

    (span.rs:459-460)
       459	            if self.lo.view().ptr_eq(self.hi.view()) {
       460	                return codec::canonical_eq(version.view(), self.lo().view());

Resolution: Add `coincident_precedence_collapses_to_one_containment` mirroring the dominance test (`fast == collapsed` against the single containment the rung documents, `assert_ne!(walked, collapsed)` for the distinct-buffer leg since early exit varies by direction) and `coincident_receiver_and_argument_collapse_to_byte_equality` (`Span::at(&v).contains(&v.clone())` reads strictly fewer scan bits than `Span::new(&v, &redecoded).contains(&v)`; whether `codec::canonical_eq` is unmetered, so the reading is exactly 0, is not settled here, and any strict inequality suffices). Reword the module doc to enumerate the rungs it holds. Acceptance: with `if false {` at span.rs:380 only, the new precedence test reads red and everything else green; the same at span.rs:459 for the receiver test; both green at HEAD.
Construction: Mutate span.rs:380 to `if false {`. `Span::precedence` on a coincident span takes the fused walk and returns the same verdict; verdict_matrix.rs:1017 compares verdicts only; no scan-parity test names `precedence` on a coincident span; the gate is green with the rung deleted.

### tests-other-11: The dominance test's doc says "read strictly more"; its body asserts only inequality
- Where: crates/before/tests/coincident_span.rs:91-93 (related: crates/before/tests/coincident_span.rs:6-8, crates/before/tests/coincident_span.rs:117-129)
- Class / severity / confidence: documentation / medium / high
- Provenance: assessed (read); executed: no
- Seen by: structure-prose; refutation: confirmed; history: as-born in 47b03e89, whose own message records the early-exit fact ("equality, since the fused walk's dominance early-exit can legitimately read fewer bits"); the doc sentence is a copy of the place test's
- Owner-gated: no

Every test's doc comment states its invariant in English and must be accurate; its incorrectness is a bug in the test. Here the doc (and the module doc at lines 6-8) says distinct-buffer coincident endpoints "read strictly more", while the body's own comment explains the fused walk's early exit can read fewer and asserts `assert_ne!`. The module doc's "so a lost rung and a dead scan meter both read red" also overclaims for this leg: the dead meter is caught by the separate `collapsed > 0` floor at 103-106, not by the direction.

Evidence:

        91	/// `Span::dominance` on a clone-coincident span reads exactly the
        92	/// collapsed containment's scan; coincident endpoints in distinct
        93	/// buffers take the fused walk and read strictly more.
       ...
       120	    // The fused walk's dominance early-exit can read *fewer* bits than
       121	    // the collapsed containment on a refuting probe, so the walking leg
       122	    // pins divergence, not direction: distinct buffers must not read
       123	    // scan-identical to the collapsed form.
       124	    assert_ne!(
       125	        walked, collapsed,

Resolution: Re-state the test doc: "coincident endpoints in distinct buffers take the fused walk and read a different scan count (its early exit may read fewer, so only divergence is pinned)". Scope the module doc's "strictly more" to `place` and the `contains` argument rung, naming dominance as the divergence-only leg. Acceptance: every doc sentence in the file matches the assertion form beneath it.

### tests-other-12: "pincer" and "jaw" are an unanchored metaphor used as jargon across four roster pins
- Where: crates/before/tests/doc_hidden.rs:1-8 (related: crates/before/tests/doc_hidden.rs:49, crates/before/tests/foreign_reexport.rs:3, crates/before/tests/foreign_reexport.rs:7, crates/before/tests/foreign_reexport.rs:104, crates/before/tests/foreign_reexport.rs:126, crates/before/tests/superlinear_tripwires.rs:14, crates/before/tests/verdict_matrix.rs:1215, crates/before/surfacecheck/src/main.rs:19)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep `pincer|\bjaws?\b` over crates/before src, surfacecheck/src, and tests: only the listed sites; no definition anywhere in src); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no rationale found (entered with the r141 witness commits 8409fe55 and 9daec56e and propagated by imitation)
- Owner-gated: no

"The totality pincer", "both jaws of the pincer", "the missing jaw", "the jaw that makes deleting ... a reviewable diff" appear in four test files and once in surfacecheck's main.rs, but the term is defined nowhere and "jaw" is never introduced by contrast. The metaphor rewrites as mechanism without loss: the two surface-totality checks (the rustdoc-JSON census in `surfacecheck` and the in-tree roster scan in `surface_coverage`). A reader of superlinear_tripwires.rs meets "the missing jaw" with no pincer in sight.

Evidence:

         1	//! The `#[doc(hidden)]` roster: every hidden public item is pinned by
         2	//! name, so hiding surface from the totality pincer is tamper-evident.
       ...
         7	//! named source files — a hidden public item is reachable API that both
         8	//! jaws of the pincer structurally miss. This pin closes that channel:

Resolution: Replace each use with the mechanism ("invisible to both totality checks: the rustdoc-JSON census omits hidden items and the roster scan reads only named files"), or define the term once where the two checks are described and cite that site. Acceptance: `grep -rn 'pincer\|\bjaw' crates/before` returns nothing, or every hit follows one definition site.

### tests-other-13: `doc_hidden.rs` pins a per-file count it calls "by name", misses every non-literal spelling, and is dissolvable by a rustdoc flag the pinned nightly accepts
- Where: crates/before/tests/doc_hidden.rs:21-32 (related: crates/before/tests/doc_hidden.rs:1-2, crates/before/tests/doc_hidden.rs:9-10, crates/before/src/party.rs:816-818, crates/before/surfacecheck/src/extract.rs:31-32, crates/before/surfacecheck/src/check.rs:37-48, justfile:937)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: verified (Python replica of the line-32 count: `#[cfg_attr(not(test), doc(hidden))]`, `#[doc(hidden, alias = "x")]`, `#![doc(hidden)]`, and `#[doc( hidden )]` each count 0 and a backticked prose mention counts 1; the two live occurrences are the literal spelling at party.rs:816 and 818, so the roster is accurate today; the refutation pass reports `rustdoc +nightly-2026-06-30 -Z unstable-options --help` lists `--document-hidden-items`, which I did not re-run, and whether the JSON backend honors the flag is unverified); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed; history: no rationale found (8409fe55 called a (file, count) pin "by name" from the first commit; `--document-hidden-items` appears nowhere in the tree)
- Owner-gated: yes: the recommended path retires an instrument and edits the gate's `surface-json` recipe

The doc says every hidden item is "pinned by name" but the roster is `(file, count)`: relocating one `#[doc(hidden)]` to a different item in party.rs keeps the count at 2 and passes. The count also matches one literal spelling, so a `cfg_attr`-wrapped or multi-argument `doc(hidden)` hides an item from rustdoc JSON unseen by all three checks. The compiler's own account can enumerate hidden items (`--document-hidden-items`), so the bespoke scan survives only by its own convention (Principle 3).

Evidence:

        21	const DOC_HIDDEN_ROSTER: &[(&str, usize)] = &[("party.rs", 2)];
       ...
        32	            let count = text.matches("#[doc(hidden)]").count();

Resolution: Preferred: append `--document-hidden-items` to the rustdoc invocation at justfile:937, let surfacecheck reach `PartyLiteral` and record its exception with the sealed-trait rationale now at doc_hidden.rs:18-20, update extract.rs:31-32, and delete this file once `just surface-totality` fails on an un-excepted hidden item (the replacement demonstrating it catches what the instrument caught). Fallback: pin the declaration line after each attribute, as foreign_reexport.rs's `(file, line-content)` roster does, and match attribute syntax (`#!?\[(cfg_attr\([^\]]*?)?doc\([^)]*\bhidden\b`) instead of one literal. Acceptance: `#[cfg_attr(not(test), doc(hidden))] pub fn escape()` on any pub item reads red somewhere in the gate; relocating an existing attribute to another item reads red; HEAD passes.
Construction: Remove `#[doc(hidden)]` from `fn into_id_bits` (party.rs:818) and add it to any other public item in party.rs: the count stays 2 and `doc_hidden_occurrences_match_the_committed_roster` passes. Separately, add `#[cfg_attr(all(), doc(hidden))] pub fn escape(&self) {}` to `Party`: rustdoc omits it, the count finds no new occurrence, and the item is invisible to all three checks.

### tests-other-14: The two flatness criteria pass on a dark meter, and the hull fold's limb leg is vacuous today
- Where: crates/before/tests/fold_skeleton.rs:33-38 (related: crates/before/tests/fold_skeleton.rs:85-96, crates/before/tests/answer_embedded.rs:111-121, crates/before/tests/answer_embedded.rs:189-196, crates/before/tests/coincident_span.rs:66-69, crates/before/tests/meter.rs:35-53)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (arithmetic at the cited lines; the refutation pass's run of `cargo nextest run -p before -p suanpan --all-features -E 'binary(fold_skeleton) | binary(answer_embedded)' --no-capture`, whose log I read at <session scratchpad>/refute-tests-other/run1_fold_answer.log; I did not re-run it); executed: yes: that run shows `limb 0` at both skeleton levels and `growth limb: 1.000` passing, and every answer_embedded cell nonzero
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness (six candidate findings merged); refutation: confirmed, with the correction that the limb reading on this population is legitimately zero (no limb record site is reachable from a dense spine of 0/1 heights folded through the accumulator path), so the floor belongs on scan and touch and limb is a model to declare, not a floor to derive; history: no rationale found (4398dcd4 is silent on floors; the audit branch it adopted from has no `growth()`; the owner's ruling of 2026-07-24 in the adversarial note, "a ceiling over a dead counter proves nothing", points the other way)
- Owner-gated: no

Principle 2: every meter needs a liveness floor so a ceiling cannot pass vacuously when a counter goes dark. `growth()` returns exactly 1.0 when the level-0 counter is zero, converting the dead-meter signal into a passing reading by construction; in answer_embedded the bound is `0.10 * t[3]` and `base * 1.10`, so all-zero readings give `0 <= 0` on both tests. Neither file asserts any counter positive, while `coincident_span.rs` three files over asserts `collapsed > 0, "a zero is a dead meter"` before every comparison. The run confirms the limb assertion in fold_skeleton is decoration today (`limb 0` at both levels), and shows every answer_embedded leg live, so per-leg floors are satisfiable there without carve-outs. The thresholds `0.10` (line 114, rationale in the module doc) and `1.10` (line 193, no rationale anywhere) are inline literals.

Evidence:

        33	fn growth(c0: u64, b0: usize, c1: u64, b1: usize) -> f64 {
        34	    if c0 == 0 {
        35	        return 1.0;
        36	    }
        37	    (c1 as f64 / b1 as f64) / (c0 as f64 / b0 as f64)
        38	}

    (answer_embedded.rs:113-114)
       113	    let mixed = t[3] as f64 - t[2] as f64 - t[1] as f64 + t[0] as f64;
       114	    let bound = 0.10 * t[3] as f64;

    (answer_embedded.rs:192-193)
       192	                assert!(
       193	                    *r <= base * 1.10,

    (run log, refutation pass)
    MEASURED span_all_skeleton lvl0: bytes 11295 scan 368915 limb 0 touch 46243
    MEASURED span_all_skeleton lvl1: bytes 22545 scan 736915 limb 0 touch 92243
    MEASURED span_all_skeleton growth limb: 1.000

Resolution: fold_skeleton: delete the `c0 == 0` branch; assert `scan > 0` and `touch > 0` at level 0 (better, derive them: the hull fold reads every operand at least once, so scan >= 8 * bytes minus per-boundary padding slack; touch >= one per leaf boundary), and either drop the limb leg or assert `limb == 0` at both levels with the model stated at the site (a dense spine of small heights does no `Base` arithmetic). answer_embedded: assert `t[0] > 0` per (op, label) in `assert_no_product` and `per_byte[0] > 0` in the ladder loop, with the crate's "a zero is a dead meter" message; name `MIXED_DIFFERENCE_FRACTION` and `PER_BYTE_GROWTH_BOUND` with their rationale beside them, as `GROWTH_BOUND` is. Acceptance: with `f()` moved above the three resets in `counters` (every reading zero), all three tests read red on a floor; at HEAD they stay green with the MEASURED lines unchanged; the limb leg is either gone or pinned to zero with its reason.
Construction: In `counters` (fold_skeleton.rs:20-30 or answer_embedded.rs:31-41) move `f();` above the three reset calls. Every reading is then 0: `growth(0, b0, 0, b1)` is 1.0 <= 1.35, `mixed = 0 <= 0.0`, and `0.0 <= 0.0 * 1.10`, so all three tests pass with the meters dark.

### tests-other-15: Prose that narrates history or restates enumerable facts
- Where: crates/before/tests/foreign_reexport.rs:12-14 (related: crates/before/tests/foreign_reexport.rs:18, crates/before/tests/foreign_reexport.rs:27, crates/before/tests/bench_judge_roster.rs:96, crates/before/tests/fold_skeleton.rs:8, crates/before/tests/fold_skeleton.rs:52, crates/before/tests/stale_state.rs:9, crates/before/tests/stale_state.rs:13-14, tools/benchjudge:138, crates/before/benches/common/sidecar.rs:38-39)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for `\b197\b` over surfacecheck/src, tools/, and .agent-notes finds no pin of the figure; tools/benchjudge:138 is `MAX_TEXT_SCALING_EXPONENT = 1.7`; stale_state.rs holds exactly four tests); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: the 197 is transcribed from 9daec56e's commit message; the dated-notes sweep d2a9d04e removed calendar dates only and never touched foreign_reexport.rs
- Owner-gated: no

Principle 5: prose speaks in the present tense, with no hand-maintained counts and no incident narration in the tree. `foreign_reexport.rs` records a past experiment with a count pinned nowhere ("Demonstrated: ... reads the same 197 items") and two git-register markers ("today: none", "empty at this tip"); `bench_judge_roster.rs:96` restates the judge's text ceiling as a literal that lives in tools/benchjudge; `fold_skeleton.rs:8` says "sixteen" beside a literal `16`; `stale_state.rs` counts its own tests three times ("Three pins", "The fourth witness", "All four").

Evidence:

        12	//! source files. Demonstrated: with `pub use bytes::Bytes;` added at
        13	//! the crate root, the surface-totality leg reads the same 197 items
        14	//! and exits clean. `pub extern crate <dep>` and a `pub type` alias of

    (bench_judge_roster.rs:96)
        96	/// The text ceiling (1.7) exists for conversion-dominated text IO only —

    (stale_state.rs:9, 13-14)
         9	//! how version-vector-style callers work. Three pins state that model.
        13	//! restored party overlaps its own descendant. The fourth witness pins
        14	//! that violation. All four are built from individually documented

Resolution: Delete the "Demonstrated: ... 197 items" sentence (the mechanism is already stated around it) and write "empty: `before` re-exports no foreign surface" for lines 18 and 27; cite the constant by name ("the judge's text ceiling, `MAX_TEXT_SCALING_EXPONENT` in tools/benchjudge") without the literal; `const FORKS: usize = 16;` used by the loop and the doc; "The version pins state that model ... A clock witness pins that violation. Every pin is built from ...". Acceptance: no numeral count of items, tests, or constants in these four files' prose that is not the name of an enforced home.

### tests-other-16: The foreign re-export pin is a substring scan that `pub use <dep>;` and import-then-alias evade, over a manifest parse that reads only a flat `[dependencies]` table
- Where: crates/before/tests/foreign_reexport.rs:78-88 (related: crates/before/tests/foreign_reexport.rs:16-21, crates/before/tests/foreign_reexport.rs:39-41, crates/before/surfacecheck/src/extract.rs:192-204, crates/before/Cargo.toml:23-31)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (Python replica of lines 70-88 with the live dependency keys: CAUGHT `pub use bytes::Bytes;`, `pub use ::bytes::Bytes;`, `pub extern crate bytes;`, `pub type Big = dashu_int::UBig;`; missed `pub use bytes;`, `pub use bytes as b;`, `use bytes::Bytes;` + `pub type Blob = Bytes;`, and each line of a wrapped brace group; false positive `pub use crate::serde_impls::X;`; extract.rs:197-204 returns on a use target absent from `index` after asserting it is in `paths`); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed, with one correction carried here: a single-item three-line `pub use {\n dep::X,\n};` is not an escape at the gate because `cargo fmt --check` normalizes it to one caught line; the wrapped form appears only past the line width, while the whole-crate and import-then-alias spellings are rustfmt-stable escapes; history: no rationale found (9daec56e chose the text pin; b1403c59 wrote extract.rs's "is not `before` surface" comment one day later; neither weighs which layer owns the check; 7f430798 closed exactly the two spellings a verification round named)
- Owner-gated: no

The module doc says "This pin closes the channel", but the `pathed` arm wants `dep::` or `::dep` on the line, so a whole-crate re-export (`pub use bytes;`, `pub use bytes as b;`) and an import-then-alias (`use bytes::Bytes; pub type Blob = Bytes;`) publish foreign surface unseen, and `::serde` matches a local path. `dependency_names` turns `in_deps` off at any `[` header, so a `[dependencies.<name>]` table or a target-specific table drops that crate without tripping the non-empty floor. surfacecheck already sees the resolved foreign id (`paths[id].crate_id`) and returns silently, which is where a spelling-proof check belongs (Principle 3).

Evidence:

        78	                let pathed = trimmed.contains("pub use") || trimmed.contains("pub type");
        79	                let whole_crate = trimmed.contains("pub extern crate");
        80	                if !(pathed || whole_crate) {
        81	                    continue;
        82	                }
        83	                if deps.iter().any(|dep| {
        84	                    (pathed
        85	                        && (trimmed.contains(&format!("{dep}::"))
        86	                            || trimmed.contains(&format!("::{dep}"))))
        87	                        || (whole_crate && trimmed.contains(&format!(" {dep}")))
        88	                }) {

    (foreign_reexport.rs:39-41)
        39	        if line.starts_with('[') {
        40	            in_deps = line == "[dependencies]";
        41	            continue;

    (surfacecheck/src/extract.rs:197-204)
       197	    let Some(id) = use_.id else { return };
       198	    let Some(target) = krate.index.get(&id) else {
       199	        assert!(
       200	            krate.paths.contains_key(&id),
       201	            "rustdoc JSON has use target {id:?} in neither index nor paths"
       202	        );
       203	        return;
       204	    };

Resolution: In surfacecheck's `walk_use`, when `index` misses and `paths` hits, record `(prefix::name, paths[id].path, crate_id)` as a foreign re-export row, do the same for `TypeAlias` items whose target resolves to a foreign id and for `ExternCrate` items, and reconcile against a committed empty census with the existing exception discipline; then delete this file (the census test is the replacement demonstration Principle 3 requires). If the text pin is kept meanwhile: match the dependency name at a word boundary on `pub use`/`pub type` lines, carry a `pub use ... {` group across lines to its `}`, and accept `[dependencies.<name>]` headers or read the manifest with a TOML parser; reword line 16 to what the scan closes. Acceptance: `pub use bytes;` and `use bytes::Bytes; pub type Blob = Bytes;` at the crate root each read red in the gate; the manifest parse finds the same dependency set when one entry is rewritten in table form.
Construction: Add `pub use bytes;` (or `use bytes::Bytes;` and `pub type Blob = Bytes;`) to crates/before/src/lib.rs. `dependency_reexports_match_the_committed_roster` passes (no line contains `bytes::` or `::bytes`), `cargo fmt --check` is unaffected, and `just surface-totality` passes because `walk_use` returns on the foreign id.

### tests-other-17: `forks_max.rs` pins a "documented behavior" no public doc states; `Clock::forks`'s doc is false at that boundary; both docs name `n` for a parameter called `k`; "both profiles" describes a dissolved mechanism
- Where: crates/before/tests/forks_max.rs:1-11 (related: crates/before/src/clock.rs:159-166, crates/before/src/clock.rs:192, crates/before/src/party.rs:239, crates/before/src/party.rs:274, crates/before/src/party/forks.rs:111-114, justfile:113-114)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read clock.rs:166 "The iterator yields `n` children" against the signature `forks(&mut self, k: u64)` at 192 and party.rs:239 "Splits `n` balanced shares" against `k` at 274; `grep saturat` over party.rs, clock.rs, party/forks.rs finds only the private comment at forks.rs:111-112; the justfile runs before's integration tests only through `test-all` (dev profile, line 114) and `--cargo-profile release` appears only for fuzzfit and wasm32-pins; `git show 14f8b019:crates/before/tests/forks_max.rs` shows `#[cfg(debug_assertions)]`/`#[cfg(not(debug_assertions))]` tests; cdad4606 added the saturating-corner sentence to both public docs and a6dcfbb4 (clock.rs) and b3f09baa (party.rs) removed them; 2efff149 renamed the count to `k` "so n means bytes alone"); executed: no
- Seen by: scaffolding, adequacy; refutation: confirmed; history: deliberate-but-expired ("both profiles" was literal under cfg branches that cdad4606 dissolved; the public saturation sentences were removed in Finch's own WIP docs-pass commits, so their return is an owner decision)
- Owner-gated: yes: the public rustdoc content was the owner's own edit; the test-doc clauses and the `n`/`k` mismatch are agent-correctable toward the code

Statement faithfulness: the test doc calls the saturation "the documented behavior", but the only statement of it is a private comment; `Clock::forks`'s rustdoc says the iterator yields `n` children, which is false at `n == u64::MAX` (it yields `u64::MAX - 1`, exactly the reading this test pins); both public docs call the parameter `n` while the signatures name it `k`; and "pinned in both profiles" names a mechanism (cfg-split tests) that no longer exists, while the gate runs these tests in the dev profile only and `saturating_add` leaves no profile-dependent behavior to pin.

Evidence:

         1	//! `forks(u64::MAX)`: the documented behavior at the split count's
         2	//! saturation boundary, pinned in both profiles.

    (clock.rs:166, 192)
       166	    /// The iterator yields `n` children and `self` keeps the last share, so it
       192	    pub fn forks(&mut self, k: u64) -> Forks<'_> {

    (party/forks.rs:111-112)
       111	        // The count saturates: at `k == u64::MAX` the residual's headroom is
       112	        // spent and the iterator yields one share fewer than asked.

Resolution: Owner call on the public docs: either restore a one-sentence boundary note in both `forks` rustdocs ("total over every `k`; at `u64::MAX` one share fewer than asked is yielded") or leave the boundary undocumented and have the test doc say it pins a property the public contract leaves implicit. Either way: rename `n` to `k` in both rustdocs (or the reverse in the signatures), and replace "pinned in both profiles" with the property actually held ("total: no panic in any profile"). Acceptance: `Clock::forks`'s rustdoc no longer claims `n` children unconditionally, or the test doc no longer says "documented"; doc and signature agree on the parameter name; no test doc claims a release-profile run the gate does not perform.
Construction: Read clock.rs:166, then run the test's own steps: `Clock::seed().forks(u64::MAX).len() == u64::MAX - 1` (forks_max.rs:49-50). The doc and the pinned reading disagree at the one input the test exists for.

### tests-other-18: The `fuzz_decode_ops` seeds are held byte-identical to the derivation but never re-parsed under the target's framing
- Where: crates/before/tests/fuzz_seeds.rs:7-11 (related: crates/before/tests/fuzz_seeds.rs:341-393, crates/before/tests/support/fuzz_seed_set.rs:263-299, crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:13-15, crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:40, crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:72)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read fuzz_seeds.rs in full: the per-target tests filter on `fuzz_decode` (103), `fuzz_decode_differential` (175), `fuzz_parse` (257), `fuzz_laws` (347); no test names `fuzz_decode_ops`; the target's framing at fuzz_decode_ops.rs:29-38 (flavour byte, one-byte length), 40 (`flavour & 1`), 72 (`op % 8`) is bound to fuzz_seed_set.rs:263-299 only by comments); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed; history: no rationale found (fd5c92bf chose comment cross-pointers; 23e46a7c added a per-framing test for `fuzz_laws` alone; the ops script grew from seven bytes to eight between bb6ea8b7 and HEAD with no test observing the new op)
- Owner-gated: no

The module doc promises every seed is held "to the contract it seeds", and the laws test (341-393) shows the intended shape (positional re-parse, in-band arity and pool indices, nothing left over). The two `fuzz_decode_ops` seeds get only byte identity: nothing asserts the flavour byte, the length prefix, that the value bytes decode as a `Clock`, that flavour-0 script bytes lie under the `% 8` table (the comment's "one full lap"), or that flavour-1's tail decodes as a `Version`. A framing change in the target regenerates nothing and reddens nothing; tests-other-26 is the kind of defect such a test would have surfaced.

Evidence:

         7	//! (`tests/support/fuzz_seed_set.rs`) and hold every seed to the
         8	//! contract it seeds — the decode targets' round-trips, the
         9	//! differential target's per-genre rejection witnesses, the parse
        10	//! target's display round-trips — so format drift is a red gate with a
        11	//! one-command fix (`cargo run -p before --example fuzz_seeds`).

    (fuzz_decode_ops.rs:72)
        72	        match op % 8 {

Resolution: Add `decode_ops_seeds_decode_per_framing`: for each `fuzz_decode_ops` seed, split flavour and length exactly as the target's `run` does, assert the value bytes decode as a `Clock` and re-encode identically, assert flavour-0 script bytes are each below the op-table span and together cover it (the claimed full lap), and assert flavour-1's tail decodes as a `Version` with the relation the derivation claims (after tests-other-26 fixes that relation). Share the span constant with the target per tests-other-22. Also list the ops and laws contract tests in the module doc (lines 7-10 omit both). Acceptance: changing the target's length prefix to two bytes, or dropping an index from the seed's op script, turns `fuzz_seeds` red naming the seed; HEAD (after the seed regeneration) is green.
Construction: Edit fuzz_decode_ops.rs so `drive_clock` dispatches on `op % 7`. The committed `clock_then_ops` seed's trailing byte 7 now aliases `tick`; the seed still matches its derivation byte-for-byte and the directory census is unchanged, so `committed_seeds_match_the_live_derivation` and `seed_directories_hold_exactly_the_set_of_record` stay green and no test observes that the seed no longer drives the op it was written for.

### tests-other-19: Small idiom slips: a qualified path beside its import, a bare `unwrap` among expect-proofs, a display comparison where `is_seed` exists, a broken doc wrap
- Where: crates/before/tests/fuzz_seeds.rs:52 (related: crates/before/tests/fuzz_seeds.rs:13, crates/before/tests/stale_state.rs:71, crates/before/tests/forks_max.rs:38, crates/before/tests/forks_max.rs:57-61, crates/before/tests/amp_board_smoke.rs:4-7)
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (read); executed: no
- Seen by: structure-prose, instrument-correctness; refutation: confirmed; history: as-born inconsistencies (forks_max.rs:38 and 57-61 are from the same commit, cdad4606)
- Owner-gated: no

Imports over long qualified paths; expect messages as one-line proofs; one spelling per predicate; legibility.

Evidence:

        52	    let mut expected: std::collections::BTreeMap<String, BTreeSet<String>> = Default::default();

    (stale_state.rs:71)
        71	    let mut restored = Clock::decode(&backup[..]).unwrap();

    (forks_max.rs:38, 58-61)
        38	    assert!(p.is_seed(), "the borrowed party recovers the entire region");
        58	    assert_eq!(
        59	        c.party().to_string(),
        60	        "1",
        61	        "the borrowed clock recovers the entire region"

Resolution: Import `BTreeMap` at fuzz_seeds.rs:13; `Clock::decode(&backup[..]).expect("a clock's own encoding decodes")`; `assert!(c.party().is_seed(), ...)` in forks_max.rs; reflow amp_board_smoke.rs:4-7 ("renders. It" / "deliberately asserts" is a two-word orphan line). Acceptance: the four sites read as described; `just fmt` and `just clippy` clean.

### tests-other-20: Nothing pins that every fuzz target has a seed directory
- Where: crates/before/tests/fuzz_seeds.rs:59-74 (related: crates/before/fuzz/fuzz_targets, crates/before/fuzz/seeds, justfile:554-559)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (`ls crates/before/fuzz/fuzz_targets crates/before/fuzz/seeds`: five targets and the same five seed directories today, so the gap is latent); executed: no
- Seen by: instrument-correctness; refutation: confirmed (the `just fuzz` recipe hand-enumerates targets and is outside the gate); history: no rationale found (fd5c92bf compared the seed root against the seed set's own targets, not the `fuzz_targets/` listing)
- Owner-gated: no

The strays test holds `fuzz/seeds/` equal to the seed set's targets, but nothing compares that set against `fuzz/fuzz_targets/*.rs`, so a new target added without seeds starts from an empty corpus with every gate leg green. The module doc's own rationale (the wide tiers random bytes essentially never reach) applies to any target.

Evidence:

        59	    // The root holds exactly one directory per target of record.
        60	    let listed_targets: BTreeSet<String> = fs::read_dir(seeds_root())
       ...
        70	    let expected_targets: BTreeSet<String> = expected.keys().cloned().collect();
        71	    assert_eq!(
        72	        listed_targets, expected_targets,
        73	        "fuzz/seeds holds directories outside the targets of record (or is missing some)"

Resolution: Assert the file stems under `fuzz/fuzz_targets/` equal `expected_targets`, so a new target must gain seeds or a documented exemption. Acceptance: adding `fuzz/fuzz_targets/fuzz_new.rs` with no seed directory reads red.
Construction: Create an empty sixth target file under `fuzz/fuzz_targets/`; `seed_directories_hold_exactly_the_set_of_record` stays green.

### tests-other-21: The "wide" tail checks test display length, not magnitude
- Where: crates/before/tests/fuzz_seeds.rs:277-279 (related: crates/before/tests/fuzz_seeds.rs:378-380, crates/before/tests/support/fuzz_seed_set.rs:359-361)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no rationale found (the length heuristic and its leaf premise were written with the parse target in 778c89f5 and copied into the laws test in 23e46a7c)
- Owner-gated: no

`text.len() >= 21` is satisfied by any version whose display is 21 or more characters, including a nested tree of small leaves (`(1, (2, 3), (4, (5, 6)))` is 24 characters), so the assertion that the corpus keeps a magnitude past `u64::MAX` can pass with no wide value present. The check should state the invariant it names.

Evidence:

       277	                // A version leaf displays as its bare magnitude; 21+ digits
       278	                // is past u64::MAX (20 digits), i.e. the wide-gamma tier.
       279	                saw_wide |= text.len() >= 21;

Resolution: Require a bare digit run (`text.bytes().all(|b| b.is_ascii_digit())`) with the length bound, or parse the display as `Ticks`/`UBig` and assert `> u64::MAX`, at both sites. Acceptance: substituting a nested narrow version for `wide_leaf` in the derivation reads red on `saw_wide`.
Construction: In fuzz_seed_set.rs:359-361 replace the 2^128 literal with `"(1, (2, 3), (4, (5, 6)))"`; regenerate; both `saw_wide` flags still set.

### tests-other-22: The `fuzz_laws` framing constants are transcribed across the workspace boundary and bound only by comments
- Where: crates/before/tests/fuzz_seeds.rs:307-316 (related: crates/before/tests/fuzz_seeds.rs:369-371, crates/before/tests/support/fuzz_seed_set.rs:263-268, crates/before/tests/support/fuzz_seed_set.rs:301-309, crates/before/fuzz/fuzz_targets/fuzz_laws.rs:57-65, crates/before/fuzz/fuzz_targets/fuzz_laws.rs:80-82, crates/before/fuzz/fuzz_targets/fuzz_laws.rs:197-212, crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:72)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read both sides: `const ARITY_SPAN: usize = 18` at fuzz_seeds.rs:308 and fuzz_laws.rs:65; pool sizes 4/3/3 as literals at fuzz_seeds.rs:369-371 against `vpool.len()`/`ppool.len()`/`cpool.len()` at fuzz_laws.rs:197-212; `picks` folds `% ARITY_SPAN` at line 81; `% 8` lives only at fuzz_decode_ops.rs:72 while the op list at fuzz_seed_set.rs:283 depends on it); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: no rationale found (the fuzz workspace's detachment, per fuzz/Cargo.toml:1-3, is a cargo-membership argument that does not bear on a `#[path]` file include; the gate compiles the fuzz targets separately through `fuzz-build`)
- Owner-gated: no

`laws_script` guards that each seed byte is in-band "so no seed byte silently aliases a smaller value", but the band it checks is a local copy: if the target's `ARITY_SPAN` moves, the checker keeps its 18 and stays green while the target folds the committed `laws_wide_gamma` scripts (arities 17/16/15) to other arities. The writer and checker cannot drift from each other (shared module), but both can drift from the target they claim to seed, at exactly the constants they transcribe. Principle 6: every hole becomes a committed check, never a convention held in comments.

Evidence:

       307	fn laws_script(name: &str, data: &mut &[u8], pool: usize) -> usize {
       308	    const ARITY_SPAN: usize = 18; // the target's arity band, per its framing
       ...
       313	    assert!(
       314	        usize::from(arity) < ARITY_SPAN,
       315	        "{name}: script arity {arity} is out of the target's arity band"
       316	    );

    (fuzz_laws.rs:65, 81)
        65	const ARITY_SPAN: usize = 18;
        81	    let arity = usize::from(byte(data)) % ARITY_SPAN;

Resolution: Move the framing (`ARITY_SPAN`, the three pool sizes, `chunk`, `picks`, and the ops target's op-table span and flavour mask) into one file, e.g. `crates/before/fuzz/framing.rs`, included by `#[path]` from the fuzz targets, `fuzz_seed_set.rs`, and `fuzz_seeds.rs` (the same mechanism that already shares the derivation between the example and the test); `laws_chunk`/`laws_script` become thin callers. Acceptance: `grep -rn 'ARITY_SPAN: usize = ' crates/before` returns one line; changing it there fails `laws_seeds_decode_per_framing_and_stay_wide` (the arity-17 seed goes out of band) instead of passing.
Construction: Edit fuzz_laws.rs to `const ARITY_SPAN: usize = 16;`. `fuzz_seeds` stays green (its own 18 still admits 17), while under the target the `laws_wide_gamma` seed's version script (arity byte 17) folds to arity 1 and its party script (16) to arity 0, so the seed no longer represents the second-octave crossings the corpus docs claim, with no gate leg red.

### tests-other-23: "mint" for constructing a value, including two test names
- Where: crates/before/tests/stale_state.rs:27 (related: crates/before/tests/stale_state.rs:6, crates/before/tests/stale_state.rs:22, crates/before/tests/stale_state.rs:79, crates/before/tests/stale_state.rs:86, crates/before/tests/stale_state.rs:93, crates/before/tests/amp_board_smoke.rs:354)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (`grep -n -i mint` over the thirteen files returns exactly these seven sites; the two test names are cited nowhere in src, tools, or .github; crates/before/src carries 58 further lines, so the crate-wide question is a separate sweep); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no repo rule forbids the word; the names were set in a474e189 by the agent that re-documented the witnesses
- Owner-gated: no

The vocabulary rule: never write "mint" for constructing a value. In function names the coinage also becomes a search key ("re_mints") no reader would guess, and it cuts against the file's own model (a version is knowledge, not a coin).

Evidence:

        27	fn same_party_ticks_on_divergent_clones_mint_equal_versions() {
       ...
        86	fn from_parts_over_an_earlier_version_re_mints_its_successor() {

    (amp_board_smoke.rs:354)
       354	/// band can only mint its operands through `registry::Shape`.

Resolution: Rename to `same_party_ticks_on_divergent_clones_produce_equal_versions` and `from_parts_over_an_earlier_version_reproduces_its_successor`; "produces"/"yields"/"re-derives" at lines 6, 22, 79, 93; "build its operands" at amp_board_smoke.rs:354. Acceptance: `grep -rn -i mint crates/before/tests` is empty.

### tests-other-24: Three rosters attest that a test is named, not that it is collected: `#[ignore]` on any kernel, twin, or band keeps every roster green
- Where: crates/before/tests/superlinear_tripwires.rs:80-87 (related: crates/before/tests/verdict_matrix.rs:1259-1287, crates/before/tests/amp_board_smoke.rs:323-327, tools/citecheck:14-19, tools/citecheck:78-81, tools/citecheck:332-344)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the three scanners: none inspects attributes; amp_board_smoke.rs:323-327 skips every `#[` line while armed, so `#[test]` then `#[ignore]` then `fn x_band()` counts; tools/citecheck fixes its citation sources at 78-81 (surface.rs, diff_ops.rs, surface_coverage.rs, laws.rs) and refuses ignored tests at 332-344; the tree's two legitimate `#[ignore]`s (codec/tests.rs:1969, exhaustive/tests.rs:445) show a blanket in-tree ban is not viable); executed: no
- Seen by: adequacy, instrument-correctness; refutation: confirmed; history: no rationale found (citecheck was extended over surface_coverage's TRIPWIRES the day it landed, 5c7e2d23, with this exact rationale; the superlinear roster existed then and was not added; the verdict-matrix roster landed the next day without joining)
- Owner-gated: no

"No mechanism for accepting known failures may exist, even as an empty buffer." The cheapest artifact for a kernel that stops reading red is `#[ignore]`, and that artifact passes all three rosters while the known-bad demonstration never runs. citecheck's own header names the hazard ("no source scan can attest *collection*") and its resolver already treats an ignored cited test as unresolved; it only lacks these citation sources.

Evidence:

        80	                let Some(rest) = line.trim_start().strip_prefix("fn ") else {
        81	                    continue;
        82	                };
        83	                let name: String = rest
        84	                    .chars()
        85	                    .take_while(|c| c.is_alphanumeric() || *c == '_')
        86	                    .collect();
        87	                if name.contains("_reads_superlinear") {

    (amp_board_smoke.rs:323-327)
       323	        if t.starts_with("#[") || t.is_empty() {
       324	            // cfg or other attributes between `#[test]` and the fn keep
       325	            // the arming; anything else below drops it.
       326	            continue;
       327	        }

    (tools/citecheck:342-343)
       342	            if case.get("ignored"):
       343	                continue

Resolution: Extend citecheck's citation sources to the two `TRIPWIRE_ROSTER` tables (tests/superlinear_tripwires.rs, tests/verdict_matrix.rs) and to the registry's `Bands::Priced` and `AXIS_BANDS` names (src/meter/registry.rs), with the same extraction floor and spelling-totality guard the existing sources get, and add a self-test fixture per source; keep the in-crate scans as the both-directions membership pins. If extending citecheck is not wanted, the fallback is for each scanner to capture the attribute run above a rostered fn and refuse `#[ignore` and `#[cfg` there, which re-implements the tool's job in three places (tests-other-3 would put it in one). Acceptance: `#[ignore]` on `sequential_meet_reduce_reads_superlinear_on_shade`, on `polarity_flipped_sweep_reads_inverted_through_the_matrix`, or on any registry-cited band test turns `just citecheck` red naming the test; removing it restores green.
Construction: Add `#[ignore]` immediately above `fn polarity_flipped_sweep_reads_inverted_through_the_matrix()` (verdict_matrix.rs:1370). `inverted_verdict_tripwires_match_the_committed_roster` still finds the name through `fn_name` and passes; the twin is never executed; `just gate` is green.

### tests-other-25: Dead `let _ = ...version();` lines that suppress nothing
- Where: crates/before/tests/support/fuzz_seed_set.rs:129-131 (related: crates/before/tests/support/fuzz_seed_set.rs:91, crates/before/tests/support/fuzz_seed_set.rs:366, crates/before/tests/support/fuzz_seed_set.rs:381, crates/before/tests/support/fuzz_seed_set.rs:387-388)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (read: `y` is used mutably by `y.fork()` at line 91 and `quarter_owner` by `.fork()` at 366 and `.party()` at 381, so no unused-variable or unused-mut lint fires without these statements); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: vestigial from birth (bb6ea8b7 had the mutable use on the line above the `let _`)
- Owner-gated: no

Code that does nothing invites a reader to look for what it does; the comments explain why no seed exists for something no reader would expect a seed for.

Evidence:

       129	    // `y` exists to nest `z`'s party a level deeper; its version stays
       130	    // empty and needs no seed of its own.
       131	    let _ = y.version();
       ...
       387	    // `quarter_owner` exists to nest the parties; only its party is read.
       388	    let _ = quarter_owner.version();

Resolution: Delete lines 129-131 and 387-388; if the "exists only to nest" intent is worth keeping, say it where `y` and `quarter_owner` are declared. Consider `quarter_owner.into_parts().0` at line 381 in place of `.party().dangerously_alias()`, removing an alias from the derivation. Acceptance: `just clippy` clean without the lines.

### tests-other-26: Two seed comments describe a "concurrent" sibling version that `Clock::sync` has already made equal
- Where: crates/before/tests/support/fuzz_seed_set.rs:290-294 (related: crates/before/tests/support/fuzz_seed_set.rs:273-275, crates/before/tests/support/fuzz_seed_set.rs:335-336, crates/before/tests/support/fuzz_seed_set.rs:346, crates/before/src/clock.rs:319-320, crates/before/src/clock.rs:333-334)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified (read clock.rs:333-334: `self.version |= &other.version; other.version = self.version.clone();`, so after line 275 `sibling.version() == clock.version()`; the doc example at 319-320 asserts the same); executed: no
- Seen by: adequacy; refutation: confirmed; history: no rationale found (the sync and the "concurrent" comment were born together in bb6ea8b7; 23e46a7c repeated the claim for `laws_family`)
- Owner-gated: no

After `clock.sync(&mut sibling)` both clocks hold one version, so the `clock_then_msg` payload is the clock's own version, not a message "concurrent to the clock's own history" (the fuzz target's flavour-1 path compares Equal and its `recv` is a no-op), and `laws_family`'s first two versions are one value, not "the synced clock's nested version, the sibling's concurrent version" (the family seeds `{v, v, empty}`). The suite exists so a seed cannot "quietly stop representing the values it was written for"; these two never represented the relation their comments claim.

Evidence:

       273	    clock
       274	        .sync(&mut sibling)
       275	        .expect("forked clocks are disjoint");
       ...
       290	    // Flavour 1: compare against, then receive, a canonical message (the
       291	    // sibling's version, concurrent to the clock's own history).
       292	    let mut msg = vec![1u8, len];
       293	    msg.extend_from_slice(&clock_bytes);
       294	    msg.extend_from_slice(&sibling.version().encode());

    (fuzz_seed_set.rs:335-336)
       335	    // A live family: the synced clock's nested version, the sibling's
       336	    // concurrent version, the empty version, and the two disjoint sibling

    (clock.rs:333-334)
       333	        self.version |= &other.version;
       334	        other.version = self.version.clone();

Resolution: Capture `let concurrent = sibling.version().clone();` before the sync (or build the message from a sibling that has not synced) and use it for the flavour-1 payload and the laws family's second version; assert the relation at the derivation (`assert!(clock.version().concurrent(&concurrent))`) and in the ops contract test of tests-other-18; regenerate with `cargo run -p before --example fuzz_seeds` and commit the changed seed files. Acceptance: the relation assertion fails against the current derivation and passes after the reorder; `committed_seeds_match_the_live_derivation` is red until regeneration and green after; the two comments read true of the bytes.
Construction: After fuzz_seed_set.rs:275 add `assert!(clock.version().concurrent(sibling.version()));`. It panics: `sync` joins both histories into both clocks, as the `Clock::sync` doc example asserts with `assert_eq!(a.version(), b.version())`.

### tests-other-27: `weave_pair()` equals `scatter_pair()` by value, so the Weave family contributes nothing to the matrix pool
- Where: crates/before/tests/verdict_matrix.rs:124-137 (related: crates/before/tests/verdict_matrix.rs:45-46, crates/before/tests/verdict_matrix.rs:116-122, crates/before/tests/verdict_matrix.rs:438-453, crates/before/tests/verdict_matrix.rs:1295-1302, crates/before/src/meter/board/family.rs:378, crates/before/src/meter/board/family.rs:897-904, crates/before/src/version/skyline.rs:38, crates/before/src/lib.rs:262-263, crates/before/src/version.rs:1483-1484)
- Class / severity / confidence: correctness / medium / high
- Provenance: assessed (ITC reasoning against the crate's stated canonical normal form; not executed); executed: no
- Seen by: refutation pass (new, as a reframe of the scaffolding lens's duplication finding); history: the duplication itself is forced by a deliberate design (a44502fe privatized the shape constructors; registry.rs:1031-1033 records that the organic families' construction lives in the board's private family module), which is why the matrix carries its own smallest instances; nothing records the collision
- Owner-gated: no: the one-line fix is local; a smallest-instance door in the family module (meter-feature API) is the optional durable form

`b = a.fork()` splits the seed into halves; `c = a.fork()` and `d = b.fork()` split each half into quarters; `a.tick()` and `c.tick()` produce height-1 events over sibling quarters of one half, whose join `(0, (0, 1, 1), 0)` collapses under ITC normal form (the crate rejects collapsible `(n, m, m)` nodes and every value is canonical) to `(0, 1, 0)`, the value `scatter_pair`'s `a.tick()` yields over the same half; likewise on the right. `intern` (438-453) dedups on `Eq`/`Hash`, and `every_family_answers_the_matrix_coverage_question` checks only that the answer is nonempty, so the module doc's "no hand-enumerated subset can silently exclude one" is false for Weave, and `weave_pair`'s doc ("interleave across the shared upper skeleton") describes a structure the pair lacks: the operands occupy disjoint halves and share only the root. The board's own weave deals leaves round-robin (`i % WEAVE_GROUPS`), which for four leaves and two groups pairs `a` with `b` and `c` with `d`.

Evidence:

       124	/// The weave population's smallest organic instance: four fork-tree
       125	/// parties of one universe, one tick each, joined round-robin so both
       126	/// operands interleave across the shared upper skeleton.
       127	fn weave_pair() -> (Version, Version) {
       128	    let mut a = Clock::seed();
       129	    let mut b = a.fork();
       130	    let mut c = a.fork();
       131	    let mut d = b.fork();
       132	    a.tick();
       133	    b.tick();
       134	    c.tick();
       135	    d.tick();
       136	    (a.version().join(c.version()), b.version().join(d.version()))
       137	}

    (verdict_matrix.rs:1298-1302)
      1298	        assert!(
      1299	            !answer.versions.is_empty() || !answer.masks.is_empty(),
      1300	            "{family:?} contributes no matrix operands: an empty answer silently \
      1301	             excludes the family from every axis"
      1302	        );

Resolution: Join the round-robin groups the doc describes: `(a.version().join(b.version()), c.version().join(d.version()))`, which yields `(0, (0,1,0), (0,1,0))` and `(0, (0,0,1), (0,0,1))`, both present at the root's two children. Add a pool-membership floor to `every_family_answers_the_matrix_coverage_question`: each family's answer interns at least one version not already in the pool, or the collision is declared at the arm. Optionally (owner-gated) expose a smallest-instance door from the board's family module under the `meter` feature so the three organic pairs derive from the same code as `scatter`/`weave`/`benign`. Acceptance: `assert_ne!(weave_pair(), scatter_pair())` holds; the pool grows by two versions against the parent commit; the new floor reads red on HEAD's `weave_pair` and green after the swap.
Construction: In `every_family_answers_the_matrix_coverage_question` add `assert_ne!(weave_pair(), scatter_pair(), "the weave pair collapses to the scatter pair");` and run `cargo nextest run -p before --all-features --test verdict_matrix`: red at HEAD by the normal-form argument above; green after the join swap.

### tests-other-28: "At the smallest committed-valid knobs" is a claim nothing pins
- Where: crates/before/tests/verdict_matrix.rs:151-161 (related: crates/before/tests/verdict_matrix.rs:48, crates/before/src/meter/registry.rs:393-510, crates/before/src/meter/registry.rs:859)
- Class / severity / confidence: claim / low / medium
- Provenance: verified (grep `smallest|min_knob|knob floor` over registry.rs finds only "the settle's smallest nonempty configuration" at 859; the registry records constructor knob preconditions in prose, no floor accessor); executed: no
- Seen by: scaffolding; refutation: confirmed; history: deliberate-and-holds for the exhaustive match (39d64f4d and the module doc state the compile-time tie); the minimality claim is prose only
- Owner-gated: yes: a knob-floor accessor on `Shape` is a registry API addition

The exhaustive match is a deliberate and defensible compile-time tie. The residual is a hand-chosen knob per arm (`packed1(8)`, `packed2(7, 3)`, `packed3(10, 2, 384)`, ...) that the doc calls "the smallest committed-valid knobs" while the registry records no floor and nothing checks minimality beyond the constructors' own preconditions.

Evidence:

       157	/// order — capped at two versions and two masks, the pool-budget rule the
       158	/// module doc derives — at the smallest committed-valid knobs, so every
       159	/// operand stays tens to hundreds of packed bytes while keeping its
       160	/// family's adversarial structure.

Resolution: Either add a `smallest()` knob-floor accessor per `Shape` in the registry and derive the pool from it (owner call), or soften the claim to what is checkable ("at small knobs, each operand tens to hundreds of packed bytes") and, where a knob is at a constructor's precondition floor, say so in the arm's comment. Acceptance: the module doc and `matrix_operands`'s doc claim only what a test or the registry pins.

### tests-other-29: `check` is a 490-line body with a closure-as-method and a free `intern` over three `&mut` fields
- Where: crates/before/tests/verdict_matrix.rs:760-765 (related: crates/before/tests/verdict_matrix.rs:438-453, crates/before/tests/verdict_matrix.rs:720-1207, crates/before/tests/verdict_matrix.rs:1134-1138, crates/before/tests/verdict_matrix.rs:1148-1152, crates/before/tests/verdict_matrix.rs:675-692)
- Class / severity / confidence: idiom / low / medium
- Provenance: assessed (read); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no rationale found (fcb78e44 reshaped the adequacy machinery without restructuring `check` or `intern`)
- Owner-gated: no

`flag` is a closure whose first parameter is `&mut Outcome`, a method spelled as a local; `intern` takes the three fields of a pool builder separately; `check` interleaves five axes so a reader auditing one axis's legs against the `Axis` docs must scan the whole function; and the dominance/precedence census labels are inline `match`es beside the named `placement_label`/`coverage_label` fns. The cost is the one tests-other-30 names: the leg roster per axis is not visible in one place.

Evidence:

       760	    let flag = |outcome: &mut Outcome, axis: Axis, leg: &'static str, detail: String| {
       761	        *outcome.counts.entry((axis, leg)).or_insert(0) += 1;
       762	        if outcome.samples.len() < SAMPLE_CAP {
       763	            outcome.samples.push(format!("{axis:?}/{leg}: {detail}"));
       764	        }
       765	    };

Resolution: `impl Outcome { fn flag(&mut self, axis, leg, detail) }`; a `PoolBuilder { versions, stable, index }` with `fn intern(&mut self, v, ordinal) -> usize`; split `check` into per-axis helpers plus `check_span_grid` and `check_conjunction_grid`, each carrying its axis's leg names; `dominance_label`/`precedence_label` for symmetry. Do it together with tests-other-30's leg roster. Acceptance: no closure takes `&mut Outcome`; `check` is a dispatcher; the twins and the production run pass unchanged with identical leg names.

### tests-other-30: The polarity-flipped twin is pinned per axis, not per strict-order leg, so a leg can be neutralized with every test green
- Where: crates/before/tests/verdict_matrix.rs:1373-1379 (related: crates/before/tests/verdict_matrix.rs:701-719, crates/before/tests/verdict_matrix.rs:1139, crates/before/tests/verdict_matrix.rs:1408-1416)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read 1373-1379 against 1408-1416; traced the construction at 1139 through the production run, the equality twin's leg list, and the polarity twin's axis check; `git log -1 fcb78e44` says "Violations are now counted per (axis, leg), so both twins pin named legs"); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: the landing commit's stated intent was per-leg pinning for both twins; the diff realized it for the equality twin only
- Owner-gated: no

The checker's doc says the polarity flip "dissents on the strict-order legs of every cross-surface axis", but the test asserts only `axis_count(axis) > 0`, and no roster of legs exists: deleting or neutralizing any one strict-order transcription (`dominance`, `rank monotonicity`, `floor coverage`, ...) leaves the production run, the equality twin (which names only equality legs), and the polarity twin green. Principle 6: a checker with one dead leg per axis passes this pin.

Evidence:

      1373	    for axis in [Axis::Masked, Axis::Placement, Axis::Ranked, Axis::Query] {
      1374	        assert!(
      1375	            outcome.axis_count(axis) > 0,
      1376	            "the flipped twin passed the {axis:?} axis: its strict-order legs \
      1377	             cannot catch a verdict inversion"
      1378	        );
      1379	    }

    (verdict_matrix.rs:1139)
      1139	            if dom != expected_dominance(lo_rel, hi_rel) {

Resolution: Add `const LEGS: &[(Axis, &str)]` rostering every leg name `check` can flag; assert the polarity twin fires on each strict-order leg by name (as 1408-1416 does for the equality twin); assert the union of the two twins' fired legs plus the three documented unreachable legs equals `LEGS`; have `flag` reject a leg not in the roster so a renamed leg cannot escape. Acceptance: replacing the condition at line 1139 with `if false` reads red in `polarity_flipped_sweep_reads_inverted_through_the_matrix`; HEAD is green.
Construction: Change line 1139 to `if false {`. Production: no violation and the census insert at 1134 is unconditional, so it passes. Equality twin: `dominance` is not in its leg list and the ranked axis stays quiet, so it passes. Polarity twin: `Axis::Placement` still fires through `placement`, `precedence`, `containment`, and the coincident legs, so `axis_count > 0` holds and it passes.

## Positives

- `verdict_matrix.rs` executes the doctrine's order literally: verdict-class liveness floors first (plain and projected, plus a rank-tie witness), a committed demonstration that the floors read red on degenerate pools with the exact census shape pinned (1337-1354), two committed mutant twins run through the same checker with the legs neither can reach rostered by name in `check`'s doc (701-719), and only then a green production run. The pool derives from an exhaustive match over `FamilyId`, the budget is derived from the roster count and asserted (507-512), the mask schedule is keyed on roster-stable ordinals so deduplication cannot reshuffle cells (421-428, 483-485), and the polarity twin's `Sweep`-axis-quiet assertion (1380-1386) is a sharp negative control.
- `amp_board_smoke.rs`'s `merge_refuses_a_silently_shrunk_grid_for_every_family` is a genuine committed known-bad artifact: well-formed shard output with exactly one defect, swept over the axis the refusal discriminates on plus the end-count boundary, and the refusal must name the shorted family. `band_tests_and_registry_citations_stay_paired` states its own vacuity defense (an empty scan fails on every cited name, 311-313), and `shard_protocol_round_trips` deals the grid unevenly across three shards so byte-identity cannot pass by an even split.
- `coincident_span.rs` asserts "a zero is a dead meter" before every scan comparison (66-69, 103-106, 167-170), pins exact equality where a rung must collapse to one code path and divergence where the fused walk's early exit makes direction unprovable, re-asserts verdict agreement across the rung boundary (131), and builds the buffer-distinct span outside the metered closures so construction's validating comparison cannot pollute a reading (161-164).
- `tests/support/fuzz_seed_set.rs` is the right shape for a derived corpus: one derivation, two consumers by `#[path]`, deterministic, public-API-only, with every hand-authored non-canonical byte carrying its bit-level derivation (111-128, 204-209, 229-243) so a reader can re-derive `0x75` and `0x4D 0xC0` by hand. `fuzz_seeds.rs` holds the directories to the exact set of record in both directions, demands genre-exact rejection witnesses (never an earlier parse error), and its laws test re-parses the target's framing positionally, asserts in-band arity and pool indices, and requires the wide, carry, and second-octave tails.
- `superlinear_tripwires.rs` and the `_reads_inverted` roster close a real hole (a committed-failing kernel that binds nowhere), hold membership in both directions with messages that name the failure mode, and state the naming convention as part of the contract.
- `bench_judge_roster.rs`'s `roster_schema_carries_expectations_only` pins the absence of an exemption class ("a cell is rostered red or held to its own ceiling, nothing else"), a direct enforcement of the no-known-failures rule; the roster's configuration is bound to the invocation by the judge itself.
- `stale_state.rs` turns a safety-rule ruling into four executable witnesses built from individually documented operations; `forks_max.rs` pins the saturating boundary with both `size_hint` and `len`, checks the drop-refold invariant (`is_seed()` after rejoin), and states its 64-bit host assumption in the `expect` message rather than hiding it.
- `foreign_reexport.rs`'s manifest parse asserts a non-empty dependency set so its own scan cannot go vacuous (53-56), and its module doc explains precisely why both totality checks are structurally blind to the channel it closes.

## Open questions for Finch

1. `fold_skeleton.rs`'s limb leg reads zero at both levels (verified by the refutation pass's run). Is `limb == 0` the intended model for the hull fold on a dense spine (no `Base` arithmetic, no wide gamma), to be pinned as such at the site, or is a nonzero limb reading expected and the population wrong? Recommendation: pin `limb == 0` with the reason, and put the floors on scan and touch (tests-other-14).
2. For the two flatness criteria in `answer_embedded.rs` and `fold_skeleton.rs` (tests-other-6): register WT/WL/deep-skeleton as `Shape`s with band citations and committed kernels, or record a ruling at each site that the closed-form separation stands in for a known-bad? Recommendation: register; the WT kernel (the schoolbook settle) already exists in query/tests.rs and needs only to be driven over the WT grid.
3. `answer_embedded.rs`, `fold_skeleton.rs`, and `coincident_span.rs` are `#![cfg(...)]`-gated whole binaries with no `[[test]] required-features` entries (Cargo.toml has them for the two examples only, lines 115-125), so under `just test` (no features) they compile to empty, green binaries; `just test-all` uses `--all-features`, so the gate is not affected. Is the silent empty binary the intended inner-loop behavior, or should `required-features` make an explicit `--test <name>` invocation loud, as the `amp_board` example's comment argues? Recommendation: add `[[test]]` entries with `required-features`.
4. Should `tools/citecheck` become the collection authority for every roster in the crate (superlinear kernels, inverted twins, band names), as tests-other-24 proposes, or are the in-crate scans a deliberately cheaper tier with the `#[ignore]` hole accepted? Recommendation: extend citecheck; its header already names this hazard and it refuses ignored tests.
5. Does rustdoc's JSON backend honor `--document-hidden-items` on the pinned nightly, and does surfacecheck's walk then reach the sealed `PartyLiteral` trait at a public path? The refutation pass verified the flag exists in `--help`; nobody ran the JSON build. Recommendation: one `just surface-json` run with the flag decides between tests-other-13's preferred and fallback resolutions.
6. Do `Party::forks(2^k - 1)` and `<[Party; N]>::from` yield the dyadic leaves in the same left-to-right order as the hand-rolled doubling loops? `wt` ticks every other leaf via `step_by(2)`, so the substitution in tests-other-7 needs an order check or a re-derivation of the MEASURED grid at the parent. Recommendation: check once; if the orders differ, keep one shared `balanced_forks` helper in `tests/support/` instead.
7. The `MEASURED ...` `eprintln!` lines in these two files (and in tests/meter.rs, asymptotics.rs, query/tests.rs, text/tests.rs) have no mechanical consumer I could find in tools/ or the justfile. If they are a `--no-capture` diagnostic convention, a one-line note in tests/meter.rs would anchor it; if nothing reads them, they are scaffolding. Recommendation: anchor the convention.
8. `verdict_matrix.rs`'s module doc estimates "on the order of a hundred thousand ordered pairs ... seconds of work"; the refutation run did not include it. Is its wall time under nextest acceptable at gate cadence, and does anything record it? Recommendation: none needed if `just gate` is comfortable; otherwise note the measured time beside the estimate.
9. "mint" (58 lines in `crates/before/src`, including validation_index.rs:142), "honest" (146 lines), and em-dashes in `//` comments (374 src lines) are crate dialect, not local slips. Do you want a crate-wide sweep (one batch, owner-reviewed), or only the partition sites? Recommendation: one sweep, batched with the prose pass.

## Dropped

- Candidate 39 (verdict-matrix half): `pool_witnesses_every_verdict_class` duplicates the production census; dropped as deliberate-and-holds: fcb78e44 explicitly deduplicated and kept exactly these two computations of one quantity held to agree (the production message reads "the checker's own class census disagrees with the floor"). The bench half survives as tests-other-9.
- Candidate 8 as filed (duplicated organic generators): the duplication is forced by the deliberate privatization of the shape constructors (a44502fe, registry.rs:1031-1033); folded into tests-other-27, where the copy turns out to be wrong rather than merely duplicated.
- Candidate 47's framing (breach of registry.rs:54-59): the "coverage by bands alone" rule governs registered families; these populations are not families, so they sit outside the rule rather than in breach. The gap is restated in tests-other-6.
- Candidates 24 and 42's "tripwire" clause: the crate names measured bands "improvement tripwire" (tests/meter.rs:42-46, twenty occurrences), so `answer_embedded.rs:19`'s use is consistent with crate vocabulary; the missing known-bad is tests-other-6.
- Candidates 4 and 49's three-line brace-group escape: refuted as stated; `cargo fmt --check` in the gate normalizes a single-item group to one caught line; the wrapped form appears only past the line width. The rustfmt-stable escapes are in tests-other-16.
- Candidate 0's suggested limb floor ("limb > 0 since the fold runs an Accumulator per boundary"): refuted by the run (`limb 0` at both levels); touches and limb operations are different currencies. Corrected in tests-other-14.
- Candidate 11's semantic predicate over `TEXT_CEILING_CELLS`: the verbatim pin is deliberate tamper-evidence per the module doc (9-10); an optional addition, noted under tests-other-9, not a defect.
- Candidate 26's "both profiles" clause: folded into tests-other-17. Its other sites are tests-other-15.
- Candidate 56 as a standalone finding (isolation premise): deliberate-and-holds at the facade and the index; folded into tests-other-7's resolution and tests-other-5's amp_board_smoke site.
- Candidates 44 and 60's forks_max spelling: folded into tests-other-19.
- Candidate 59's `seed_set` doc enumeration: folded into tests-other-5.
- Candidates 18, 32, 45, 46: duplicates of tests-other-14. Candidates 21, 52 (ops half): duplicates of tests-other-18. Candidates 33, 52 (constants half): duplicates of tests-other-22. Candidates 22, 49: duplicates of tests-other-16. Candidates 23, 51: duplicates of tests-other-13. Candidates 25, 29, 55: duplicates of tests-other-3. Candidate 48: duplicate of tests-other-24 (its qualifier miss is in tests-other-3). Candidates 27, 35, 57: duplicates of tests-other-23. Candidates 28, 30: duplicates of tests-other-2. Candidates 40, 59 (dead lines): duplicates of tests-other-25. Candidates 36, 58: duplicates of tests-other-15. Candidate 37: duplicate of tests-other-7. Candidate 43: folded into tests-other-15. Candidate 41: folded into tests-other-8.
- The structure-prose lens's observation that `pool.versions[i] != pool.versions[j]` (verdict_matrix.rs:754) is always true after interning: an observation that documents intent, not a finding.
