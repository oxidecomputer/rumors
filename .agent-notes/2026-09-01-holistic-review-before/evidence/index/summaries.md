# Report summaries, positives, open questions, dropped

<!-- source: final/benches-examples.md -->
# Partition benches-examples: Criterion benches (party, version, clock, amplify, board, presize, tripwire, common, sidecar) and the examples

## Partition summary

This partition is `before`'s wall-clock instrumentation and its offline studies. Three differential benches (party.rs, version.rs, clock.rs) time the packed implementation against the recursive oracle on structurally identical randomized trees built by benches/common/mod.rs from one `Plan`; two judge-facing targets (board.rs, tripwire.rs) time every amplification-board cell and a known-bad quadratic, writing a stamped denominator sidecar (common/sidecar.rs) that `tools/benchjudge` divides by to fit time exponents across two scales; amplify.rs re-times three adversarial shapes at fixed sizes; presize.rs is an allocation-strategy A/B record with compile-time arms in the library. The examples are the board's process shell (amp_board.rs), the fuzz-seed regenerator (fuzz_seeds.rs), the paper's space-consumption experiment (space_consumption.rs), and three study or probe binaries (code_study.rs, perf_probe.rs, emit_probe.rs). I read all fifteen files in full with line numbers (3,709 lines), plus the out-of-partition anchors every finding rests on (the skyline `causal_cmp` rung, the board's op and family tables, `bench_cells`, the alloc seams, the judge, the roster pin test, the results directories, and the cargo registry sources of peak_alloc 0.3.0 and criterion 0.5.1). None of the partition files is a test; the partition's checker tests (tests/bench_judge_roster.rs, tests/fuzz_seeds.rs) were read where cited. No cargo, just, or bench command was run: every verdict rests on reading, grep, and read-only git.

The judge-facing apparatus is the strongest code here and is built the way the doctrine asks. board.rs enumerates the board's own cell table so a red board cell names the bench that times it and there is no second list; the sidecar stamps scale, profile, sampling mode, and source tip so the judge refuses a lo/hi pair from different configurations; the ceiling class rides the sidecar, never the roster; two committed known-bad demonstrators (the unmetered quadratic in tripwire.rs, the schoolbook decimal renderer in board.rs, asserted at setup to spell the identical value) must read red every sweep, and the judge's `--self-test` pins their measured shapes. amp_board.rs owns only what the library cannot (the global allocator, process spawning, the CLI) and passes the shard scale as exact f64 bits; fuzz_seeds.rs writes atomically and shares its derivation with its checker test by `#[path]`. space_consumption.rs reproduces the paper's parameters exactly and reports bits beside bytes with the reason stated.

The defects cluster at phase boundaries nobody re-audited. Three instruments outlived the constraint that justified them: amplify.rs (written one commit before board.rs, which derives the same three cells under the judge), emit_probe.rs (compares bitvec against a standalone re-implementation, so it cannot see the crate regress, and holds bitvec as a dev-dependency for a decision that landed), and the presize A/B leg (production seams, no recorded verdict, an unpinned deterministic column; its sibling stacks leg from the same commit was closed the next day with a ratified verdict). One hard-rule breach: code_study.rs's stated reason to exist links a `before::implementation` essay deleted on 2026-08-17, and no lint can see it because `cargo doc` never documents examples. One instrument defect of substance: the `version/partial_cmp` `equal` row passes the same reference twice and so times the ptr_eq identity rung against the oracle's two full walks, has done so in one form or another since the benches' first day, and its 2026-06-02 "~31x" leaked into a committed results directory whose plot script now names dead bench IDs. Around these sit the expected precision gaps: a ceiling-class assert that compares the pinned set with itself for every board cell after 514cbcea made board cells derive from the pin, a stamp doc stronger than the judge's cross-check, a scale knob whose message names a constraint the code does not check, hand-synchronized copies of the corpus and simulation with parity asserted in prose, and a handful of doc slips (recv borrows; Version is Clone; literals for the judge's constants).


## Positives

- benches/board.rs and src/meter/board/export.rs: bench IDs are the board's own op × family cell names, derived from the axis declarations with a rule-based pinned subset (`designed` plus `BOARD_DECLARED_BENCH_RIDERS`), so a red board cell names the bench that times it and there is no second list (amplify.rs aside). Verified by reading `bench_cells`.
- benches/tripwire.rs and board.rs's `schoolbook_decimal`: committed known-bad kernels the judge must read red every sweep. The schoolbook renderer is asserted at setup to spell the identical value as the crate's `Display`, its text length is held to the decimal-digit band so the text side of `n_io` cannot be padded, and both shapes are pinned deterministically in `tools/benchjudge --self-test` (lines 643-668) with the exit contract. Verified by reading.
- benches/common/sidecar.rs stamps scale, profile, sampling mode, and source tip on every sidecar, and the judge refuses a lo/hi pair that disagrees (exit 2) instead of re-scoring; the ceiling class rides the sidecar, never the roster, and tests/bench_judge_roster.rs pins the text set's exact membership. Verified by reading `cross_check_stamps` and the pin test.
- benches/presize.rs's `outgrow_family` asserts its own design property on construction so a construction drift fails loudly (looser than its sentence, per benches-examples-14, but the reflex is right); every `presize-resident` line is stamped with the compiled arm; the arm roster is `deny`-registered as check-cfg and validated at the recipe.
- examples/fuzz_seeds.rs shares one derivation with its checker test through `#[path]` and writes each seed atomically (temp file then rename), so writer and checker cannot drift and an interrupted run cannot truncate a committed seed.
- examples/amp_board.rs is a thin shell that owns only what the library cannot (the global allocator, process spawning, the CLI); it passes the shard scale to children as exact f64 hex bits, asserts every child's exit status, and `required-features` turns an unfeatured build into a cargo error rather than a board that looks judged and is not (Cargo.toml:110-117).
- benches/common/mod.rs builds every input through the public API from one `Plan` applied identically to impl and oracle with a fixed seed, and its module doc is maintainer-altitude explanation done right (the fork/preserve/join recipe, why the preserved subset randomizes the tree, why the two sides are structurally identical).
- benches/version.rs `bench_hole/masked_eq` deliberately builds a second, byte-identical pair in distinct buffers and says why ("a full co-walk, no early exit"); it is the model the `partial_cmp` equal row should follow.
- board.rs's `criterion_group!` config comment (313-316) states exactly what the code cannot show: the committed windows land before `configure_from_args` so the quick-mode flags keep CLI precedence.
- examples/space_consumption.rs: the paper-to-API mapping table, heavy-populations-first ordering so the ETA overestimates rather than underestimates, deterministic per-run seeds mixing scenario, population, and run, and both bits (the smooth quantity) and bytes (what storage pays) reported with the bias explained; rayon and indicatif rather than hand-rolled parallelism and progress.
- code_study.rs's closed-form code lengths (gamma 2l-1, Elias delta, Elias omega, Boldi-Vigna zeta with the k | (l-1) short branch, Rice q+1+k) are correct against the standard definitions, and its reconciliation pin (`nodes + gamma_bits == encoded_bits`) is exact by construction.

## Open questions for Finch

1. The presize allocation-strategy record (benches-examples-12): was it ever run to a verdict offline? If so, where should the verdict be recorded before the arms are dissolved; if not, is the question still live? Recommendation: close it the way the stacks leg was closed at cd171c29 (record, verdict in a note and the closing commit, dissolve seams, roster, recipe, and `alloc_arms`), keeping the `projection_outgrow` family in benches/version.rs's hole group if the growth-crossing shape is wanted as a criterion number. benches-examples-5, -13, and -14 become moot on closure.
2. perf_probe.rs (benches-examples-22): b606f2ad recorded its standing role as the sampling profiler's op-isolation loop. criterion 0.5.1 ships `--profile-time <secs>` (lib.rs:837), which runs any bench ID unmeasured under a profiler, and every loop in perf_probe has a criterion twin (tick → `version/tick`, holetick → `version/hole/tick`, holeproj → `version/hole/project`, holecmp → `version/hole/masked_eq`, vjoin → `version/merge`, vcmp → `version/partial_cmp`, cjoin → `clock/join`, decode/encode → `clock/codec`). Recommendation: retire it and name `just bench <target> <filter> -- --profile-time 10` as the profiler entry in the justfile's bench comment and the perf-probe note; if kept, apply benches-examples-19 and -22 so it shares `benches/common` and reads in the present tense.
3. code_study.rs (benches-examples-18): the constants-frontier ruling ("keep gamma; revisit trigger: re-running the committed study binary") keeps it as a manual instrument. Recommendation: keep it, reword the doc to stand on its own, and add a tiny-parameter smoke leg so the reconciliation pin stays live; the study's numbers already live in the note, so dissolving is also defensible if the gamma question is closed for good.
4. crates/before/results/benchmarks (benches-examples-25): a maintained artifact regenerated at release points, or a one-time 2026-06-02 comparison? Nothing in the tree links to it. Recommendation: excise it and scripts/plot_benchmarks.py; the perf-probe note and criterion's own baselines carry the current picture.
5. The hugeleaf parse trio sits in the text class on judged exponents of 1.28/1.33/1.30 (b1c07fe1), one under the general ceiling, giving the parse path about 0.4 of exponent headroom before a red. This reopens a recorded ruling with no new evidence, so it is a question, not a finding. Recommendation: no change to the class; if the headroom worries you, have the judge print an informational line for a text-class cell whose exponent fits under the general ceiling minus the fit-noise band, so a class declaration that stops being exercised is visible in the table.
6. `honest` as a term of art and true em-dashes in `//` comments are crate-wide house practice (146 and 374 occurrences in crates/before/src respectively; the dfd19c44 style round kept both). The vocabulary rule in the brief flags the first and the conversational-register rule the second, but re-wording four bench sites in isolation would make the partition inconsistent with the rest of the crate. Recommendation: leave both alone in this partition; if you want either changed, it is a crate-wide vocabulary decision for a dedicated prose pass.
7. `just bench-judge` defaults to quick sampling and `just all` calls it bare, while board.rs:31-35 says quick mode "is for agent iteration only" and full sampling "is required for any quoted number". The roster pins both modes as exponent-class expectations, so this is consistent by design. Recommendation: leave as is; if the schoolbook red attestation should be at record sampling, `all` can call `bench-judge record`.

## Dropped

- [21] Parse trio's text class grants ~0.4 of headroom: reopens the recorded ruling (b1c07fe1) with no new evidence, and the text ceiling's known-bad demonstrator is direction-independent; moved to open question 5.
- [40] "honest" used as a term of art: crate-wide house idiom (146 occurrences in src, one sense anchored to `assert_honest_text`); moved to open question 6.
- [41] Em-dashes in `//` comments: uniform house practice (374 in src); the rule cited is the owner's conversational register, not the repository's; moved to open question 6.
- [0], [18], [29], [54] amplify duplicates: merged into benches-examples-1.
- [1], [17], [28], [39], [53] probe duplicates: split by history into benches-examples-21 (emit_probe) and benches-examples-22 (perf_probe framing), with the `--profile-time` substitution as open question 2.
- [4], [15], [26], [51] code_study ghost duplicates: merged into benches-examples-18.
- [5], [16], [27 part], [34], [52] duplication duplicates: merged into benches-examples-19.
- [3], [30], [46] equal-row duplicates: merged into benches-examples-17.
- [7], [19], [36], [48] ceiling-assert duplicates: merged into benches-examples-10.
- [13], [23], [35 part], [56] scale-knob duplicates: merged into benches-examples-8.
- [8], [37], [11 part] literal-figure duplicates: merged into benches-examples-7.
- [11 part], [22], [31] ownership-prose duplicates: merged into benches-examples-3.
- [12], [32], [33] common-duplication duplicates: merged into benches-examples-4.
- [10], [25] space README duplicates: merged into benches-examples-26.
- [6], [35 part] hand-rolled JSON duplicates: merged into benches-examples-9 at nit with the ordering caveat.
- [24], [42], [44] mechanical slips: merged into benches-examples-2.
- Refutation new note 3 (bytes version 1.11.1 not 1.12.1): a citation correction folded into benches-examples-12, not a finding.

<!-- source: final/board-families-floors-judge.md -->
# Partition board-families-floors-judge: The board families, floors, judge, measure, operands

## Partition summary

The five files are the amplification board's judgment core, compiled only under the `meter` feature and reached by no production path. `family.rs` (1275 lines) is the shape axis: thirty base-size constants with their derivations, the `FamilyData` operand bundle, a `build` arm per board family, a uniform post-pass that derives the slots a shape does not natively fill, and the two mount adapters that lift one id shape into a disjoint or an overlapping party pair. `floors.rs` (793 lines) is the liveness vocabulary: the rendered `WHY_`/`NA_` derivation strings and the constructors that turn operand quantities into per-currency `Floors`. `judge.rs` (437 lines) fits the exponent trend, resolves each currency's ceiling, and pronounces the per-cell verdict; `measure.rs` (209 lines) brackets one cell body with every counter and settles the denominators from the actual result; `operand.rs` (297 lines) holds the iterative walks over the packed skyline stream that the floors and denominators are stated in. I read all 3011 lines with line numbers, and the neighbor sites every finding rests on (board.rs, ceilings.rs, currency.rs, cell.rs, render.rs, shard.rs, ops.rs call sites, board/tests.rs, registry.rs, the codec scan and build recorders, validate.rs, signed.rs, suanpan's accumulator and touch meter, party.rs, idbits.rs, split.rs, clock.rs, version.rs, forks.rs, tools/benchjudge, tests/superlinear_tripwires.rs, and the 500d4d09 commit). None of the five files is a test file; the sibling `board/tests.rs` was read in part for the adequacy demonstrations.

The apparatus is structurally sound and in several places exemplary. Floors are derived from operands at prepare, outside measurement, and fork on the verdict a cell will produce rather than on the family; `touch_pair_fold` is a model minimum-work derivation (max not sum, zero deltas excluded with the mechanism stated, the family a naive count would ban named at the site); `measure` resets every counter immediately before the body and reads it immediately after, keeping the result alive until the heap peak is read; the exponent estimator is a genuine log-log least-squares slope; and the committed known-bad artifacts (the meter-bypassing walk, the chunked schoolbook converter, the lump ladder versus the quadratic ladder, the fold model's fat constant, the stale capacity model) go through `evaluate` itself. The `ByCurrency` totality mechanism means a new currency is a compile error at every declaration and judgment site. Under the scaffolding lens nothing is dissolvable outright: every instrument names a failure class outside itself and no external tool produces these numbers.

The dominant issues are three. First, one genuine hole in the verdict of record: the heap exponent leg fits the raw readings while the constant leg subtracts the flat allowance, so a Theta(n^2) heap term of allowance-scale magnitude reads green across the whole acceptance ladder, weaker than the bare debugging view on the same artifact (finding 21). Second, a pattern of not-applicable declarations whose rendered reasons the code contradicts (the decode rows' touch, the seed party's scan, `clock_fork`'s heap), which leaves `clock_fork` on every version-only family watched by no floor at all and outside the module's own exposure disclosure (findings 10, 11). Third, prose drift of the kind the owner's own 2026-08-11 excision ruling addressed but did not finish in `family.rs`: probe-build readings quoted as present-tense fact, one mislabeled "committed", and a slot roster that has outgrown the code (findings 1, 2). The remainder is legibility work a maintainer would schedule: the same preorder walk and zigzag decode written out four and two times over in `operand.rs`, repeated `Floors` literals and scan-floor casts in `floors.rs`, an undefined "deterministic-liveness" class contradicted by the module's first sentence, and a vocabulary of "honest", "mint", and "today" that the writing-style rules now name.


## Positives

- Floors are derived from the operands at prepare, never from readings, and fork on the verdict the cell will produce: `comparison_floors`, `membership_floors`, and `masked_cmp_floors` (floors.rs:668-689, 718-739, 751-768) realize "one universal premise, no per-family carve-outs" in code rather than prose, and `membership_floors` gets the one-directional derivation right (refusal is the full-certification direction; admission needs one witness), agreeing with `since()`'s documented semantics.
- `touch_pair_fold` (floors.rs:458-492) is a model liveness floor: per boundary not per element, a max rather than a sum with the reason stated, zero deltas excluded as legitimate less-work inputs with the mechanism (an accumulator add of zero is a no-op), and the family a naive count would have banned (tooth-tail) named at the derivation.
- `touch_fold_first_merges`'s arrival-adjacent-pairs premise is exact against the implementation: `Version::join_all` seeds the receiver first, the binary-counter fold merges arrival-adjacent inputs at weight 0, and the dedup collapses on pointer identity only, so `chunks(2)` over the prepared list is the true first level.
- `measure` (measure.rs:84-95) resets every counter immediately before the body and reads it immediately after with no metered work between, keeps the result alive until the heap peak is read, settles the I/O denominator from the actual result (`(spec.output_bytes)(result.as_ref())`), and asserts text-output honesty at the measurement site; `HeapMeter` as caller-supplied fn pointers is the right shape for per-binary allocator state the library cannot own, and its doc says exactly why.
- `judge::trend` is a genuine log-log least-squares slope over every measured point with the clamp's direction documented, and the committed adequacy demonstrations go through `evaluate` itself: the meter-bypassing walk (green under ceilings, red under the scan floor), the chunked schoolbook converter (under kappa, red on the n_io exponent), the lump ladder versus the quadratic ladder through `evaluate_acceptance`, the fold model's fat constant and quadratic, and the capacity band red on both the regressed and the improved side.
- `amp_board` carries `required-features = ["limb-meter", "scan-meter"]` (Cargo.toml:115-117), so a dark counter column cannot reach the verdict of record; the `None`-reading exemption in `judge_window` is unreachable there.
- `ByCurrency`'s field-per-currency totality with exhaustive `each()` destructuring means a new meter is a compile error at every declaration, judgment, and render site; judge.rs's loops run over the axis itself, not a hand list.
- The mod-32 remainder-alignment derivations on the base constants (family.rs:127-139, 148-157, 205-215, 224-230, 241-244, 277-283, 294-296) are careful measurement design: they explain why a legitimate amortized-O(1) constant would otherwise read as growth across the ladder and choose the base to hold it fixed, as mechanism rather than observed number.
- `MIN_SIZE_PARAM`'s doc (family.rs:380-390) says precisely what the floor preserves (positivity) and what it does not (relations between knobs) and names the two repair conventions; `overlap_mounted_pair` (1128-1178) states plainly that its outputs are semantically void by design and why the cost claim still needs them.
- The stream-derived versus tree-derived limb floor split (operand.rs:47-56, 127-137) is argued from the coding (a plateau stores its width once) and pinned by a test that constructs the separating shape.
- The exposure disclosure at floors.rs:99-112 states the unwatched cells and bounds the exposure by mechanism instead of leaving it silent; finding 11 asks for it to be pinned, but the instinct to disclose is exactly right.
- The scan meter records builder writes as well as reads (codec/build.rs), so the tick rows' 8-bits-per-byte floor is in fact cleared by the shipped walk with wide margin; finding 17 is about the derivation's wording, not the floor's soundness.

## Open questions for Finch

1. The flat-denominator content axis (`Sample.exp_denom_bytes`, `FamilyData.content_bytes`, `value_content_bytes`, the `content` argument to `measure`, two tripwire tests, and cell.rs's derivation) exists for one family, comb-scatter, because its packed bytes grow ~1.2x per doubling at fixed tooth magnitude. Could the shape be rescaled so packed bytes track the knob (as Cliff scales k with n) while keeping the output-domination ratio the projection rows need, retiring the second denominator? Recommendation: keep it for now, but steelman the dissolution in cell.rs's derivation so the next reader sees it was weighed. Related: the content denominator is applied to version-only rows' exponents on comb-scatter too (family.rs:548 adds `p.len()`), harmless while both halves double per level, worth a sentence.
2. Kernel-pinned ("deterministic-liveness") floors pin the current metered representation and trip on an improvement; the envelope suite (tests/meter.rs) is the instrument whose pins are explicitly implementation-pinned. Should those floors live there, leaving the board's floors purely contract-derived, or is the board's rendered legend the disclosure surface you want? Recommendation: keep them on the board (the legend is where a trip is read) and define the class per finding 8.
3. The registry doc (registry.rs:569-583) lists the base-size constant and its derivation doc among the things "found by luck" when a family is added. Should `Coverage::Board` carry the base size (or a `BoardBase` struct) so the registry row is the single declaration a new column requires? Recommendation: yes, as part of finding 6's `BoardFamily` refactor, moving family.rs's thirty constants beside their coverage answers.
4. The disjoint-mount and overlap-mount adapters assert the property they construct on every bundle build (family.rs:1121-1124, 1150-1154, 1173-1176). The constructions are deterministic pure functions of the shape bytes and the smoke test builds every board family. Recommendation: keep them; they run once per family per scale at prepare, outside measurement, each message is a one-line proof, and the cost is negligible; convert to unit tests only if build time becomes a budget item.
5. Which board families present `version2 < version`, so the membership row's refusing (full-certification) direction is priced at all? For every non-pair shape `version2` is the seed tick of `version`, which `since(v).contains(w)` admits with one witness; only the pair shapes can exercise refusal. Recommendation: a one-line test asserting at least one board family's membership cell carries the full-examination floor, so the arm is not dead on the board.
6. The hugeleaf ladder (32k..256k value bits) and the envelope suite's hugeleaf pin (125k bits) both sit above the backend parser's algorithm switch at 16k-20k bits that HUGELEAF_BASE_MAGNITUDE_BITS documents. Is the sub-switch transient's magnitude pinned anywhere at a fixed size, so a regression from ~x4 to ~x40 there would be seen? Values under 16k bits are the common case. Recommendation: one fixed-size envelope pin below the switch.
7. Under a residual-based heap fit (finding 21), what materiality guard do you want: a fixed fraction of `HEAP_FLAT_ALLOWANCE_BYTES`, or a per-denominator-byte minimum tied to `MAX_HEAP_BYTES_PER_INPUT_BYTE`? The choice sets how small a super-linear heap term the board can resolve at KiB inputs. Recommendation: a fraction of the allowance (say a quarter), stated beside it.
8. Is the `meter` feature's counter-reader surface (`meter::limb_ops`, `scan_bits`, the resets) covered by the public-API stability rule? That decides whether Option-returning readers (finding 16(c)) are an internal refactor or an owner-gated API change.
9. Vocabulary (finding 15): 167 "honest", 66 "mint", 22 "today", and 374 em-dash `//` comments across crates/before/src are the dialect of the period, predating the 2026-08-19 writing-style rules. Recommendation: one crate-wide ruling and a single sweep commit rather than per-partition edits, with "genre" allowed to stand as established in-repo vocabulary (.cargo/mutants.toml uses it).
10. Weight comb and freeze parade: is the intent that their flatness bands eventually get a committed `_reads_superlinear` kernel, or is the band's adequacy deliberately resting on a local probe build? family.rs currently describes the latter as "committed" (finding 1).
11. Relayed for the board-frame/tests reviewer (outside this partition): board/tests.rs:520 writes `a_bytes.len() / 64` where `OVERLAP_FOLD_INPUT_DIVISOR` (family.rs:170) is the constant of record, and tests.rs:505 cites "the design doc's §3 entry" from code.

## Dropped

- [0] operand.rs walk/zigzag duplication (scaffolding): duplicate of board-families-floors-judge-24 (the structure-prose statement is the more complete one).
- [1] Base-size docs quote readings (scaffolding): merged into board-families-floors-judge-1 with [23]; owner-gating removed because the ruling exists.
- [4] Default-dialect vocabulary (scaffolding): the "today" half merged into board-families-floors-judge-8; the rest into board-families-floors-judge-15 (measure.rs's "3 honest" was a miscount; it has none).
- [5] floors.rs helper re-inlining (scaffolding): merged into board-families-floors-judge-13 with [25], [21], [44].
- [6] Module doc's universal rule contradicted (scaffolding): subsumed by board-families-floors-judge-8.
- [7] Hand-maintained "Four cells" and 10 µs (scaffolding): merged into board-families-floors-judge-11 with [27] and the count half of [43].
- [11] Sample field copying and cfg shims (scaffolding): merged into board-families-floors-judge-16 with [36].
- [12] judge.rs policy prose duplicated, spans twice, Fit/Score (scaffolding): the mechanical parts merged into board-families-floors-judge-23; the policy paragraph on `trend` and its "owner-ratified" tag are deliberate per 9e36dd28 and d2a9d04e (history), so the prose-removal half is dropped.
- [13] family.rs qualified path and double decode (scaffolding): subsumed by board-families-floors-judge-3.
- [19] Measured 2-5x reading in tick_walk_floors prose (adequacy): refuted; 500d4d09 explicitly keeps "the tick walk's 2-5x floor margin" among order-of-magnitude calibrations; converted into board-families-floors-judge-17's note that the ceilings.rs convention header should name the kept-calibration class.
- [20] SCAN_TOUCH_FLOOR_BITS half its derivation (adequacy): merged into board-families-floors-judge-9.
- [21] Zero-minimum Floor constructible (adequacy): merged into board-families-floors-judge-13 (the `floor_or_na` constructor makes the state unrepresentable).
- [28] NA_SCAN_SEED_PARTY says the seed is empty (structure-prose): merged into board-families-floors-judge-10(b).
- [29] "mint" seven times (structure-prose): merged into board-families-floors-judge-15.
- [30] Vocabulary sweep (structure-prose): merged into board-families-floors-judge-15.
- [31] Derivations restated (structure-prose): the floors.rs/operand.rs half is board-families-floors-judge-12; the judge.rs half is dropped for the same reason as [12].
- [32] judge_window match split (structure-prose): merged into board-families-floors-judge-23.
- [35] Constant placement convention (structure-prose): folded into board-families-floors-judge-13's resolution as a layout step while consolidating.
- [39] Tick-walk scan floor derivation (instrument-correctness): reframed by the refutation pass and merged with [10] into board-families-floors-judge-17.
- [40] NA_SCAN_SEED_PARTY and the empty branch (instrument-correctness): the NA half merged into board-families-floors-judge-10(b); the empty branch is board-families-floors-judge-26.
- [41] Decode rows' touch NA (instrument-correctness): duplicate of board-families-floors-judge-10(a).
- [43] "Four cells" and `w ≤ v, strictly` (instrument-correctness): the count merged into board-families-floors-judge-11; the inequality slip into board-families-floors-judge-9(c).
- [44] Count arithmetic idioms (instrument-correctness): merged into board-families-floors-judge-13.
- [45] &Option<Ordering> and qualified path (instrument-correctness): subsumed by board-families-floors-judge-3.
- Structure-prose open question "does Party::fork read the seed's tag through a metered primitive": answered yes by reading (idbits.rs:132-139 via split.rs:22), so it became evidence for board-families-floors-judge-10(b) rather than an open question.

<!-- source: final/board-frame.md -->
# Partition board-frame: The amplification board frame: module root, ceilings, cells, coverage, currencies, defects, export

## Partition summary

The board frame is the declarative skeleton of `before`'s amplification board: `board.rs` (300 lines) is the module root and its 260-line rustdoc states the criterion, the three-axis product, the liveness floors, the time leg, the ladder, denomination, declared models, the rejection surface, and the coverage tiling; `ceilings.rs` (467) holds every pinned global ceiling, floor parameter, and declared per-cell model with its derivation; `cell.rs` (323) is one prepared cell with its `Denom`/`IoSpec`/`TextSpec` denomination rule and the output-honesty assertion; `currency.rs` (141) defines the five-field `ByCurrency<T>` axis and `Liveness`; `defect.rs` (176) builds the rejection rows' maximally deferred defects; `coverage.rs` (663) is the two-table tiling over `before::surface`, with `coverage/tests.rs` (106, the only test file in the partition) enforcing it; `export.rs` (162) exposes cells to the bench suite and derives the pinned bench subset. I read all 2338 lines with line numbers, plus the sites outside the partition every surviving finding depends on (recurse.rs, Cargo.toml, the justfile recipes, judge.rs, measure.rs, operand.rs, floors.rs, ops.rs, board/tests.rs, codec/bits.rs, codec/text.rs, codec/display.rs, family.rs, clock.rs, version.rs, surface.rs, validation_index.rs). I ran no cargo, just, or build command; the one execution was a Python transcription of `judge::trend` to settle an arithmetic claim.

The frame is, on the whole, an instrument built the way the doctrine asks. `ByCurrency` is a genuine compile-time totality mechanism (no `Default`, no `..`, an exhaustive destructure in `each`), so a half-wired currency is a compile error rather than a convention. The ceilings file states and follows the present-tense discipline for measurements (readings live in pin commits, `git log -S` the constant; c0b5d701 carries the touch ceiling's basis). The output-honesty ceiling is derived, implemented as an assertion on every text stream entering a denominator on both the board and bench paths, and tripwired. The defect builders justify every placement, and the packed-side constructions are correct against the codings. The tiling test checks both directions plus disjointness. The bench export's criterion IDs are the board's own cell names, with the denominator read back from the actual result.

The dominant issues are of two kinds. First, machinery that outlived its constraint: the segments currency reads zero by construction in every board binary (its only writer is `#[cfg(test)]`), yet the front page presents it as a live meter and `LADDER_TOP_SCALE` derives its ×4 from segment-onset amplifiers the board cannot observe; the rider list is a hand-maintained cell roster in a module whose doc says none exists, with a one-direction pin, and it is the time leg the committed cadence actually judges. Second, prose that has drifted from code because it is stated in several places: the root doc's "all four counters" survived the fifth currency, the fold-model roster is stated three ways (two, three, four rows), two sites prescribe a "dated owner rationale" the tree excised, and the `truncated_bytes` rationale argues from a decoder verdict the decoder stopped producing. Two findings were false at birth rather than drifted: `party_noncanonical_text`'s "at the text's end" (the id notation spells absent children as `0`, so the last `1` is never the last token on any board operand) and `MAX_SCALING_EXPONENT`'s claim to exclude a log factor (an n·log n kernel fits 1.07-1.10 on every committed ladder). Everything else is low or nit: a duplicated denominator rule, five copies of one preorder skeleton, an underived heap ceiling, a wrong-leg one-line proof, vocabulary and punctuation the crate uses everywhere.


## Positives

- `currency.rs` is the strongest design in the frame: `ByCurrency<T>` has no `Default` and no `..` path, `each` destructures exhaustively into a `[_; 5]` the compiler checks, and `Floors = ByCurrency<Liveness>` forces every cell to answer floor-or-NA per currency. Adding a currency really is a compile error at every declaration and judgment site, and the module doc (:13-25) names the exact failure class this makes inexpressible rather than asserting safety in the abstract. (Verified by reading.)
- `ceilings.rs:56-62` states and follows the present-tense discipline for measurements: readings live in the pin commits, `git log -S` the constant, with the reason given (a quoted reading keeps asserting itself as fact while headroom absorbs drift). c0b5d701's message carries the touch ceiling's basis (16.0 and 17.18 touches per byte; ceil(17.18 × 1.25) = 22; 2,096 green / 0 red). (Verified via `git log -1 --format=%b`.)
- The output-honesty ceiling is derived rather than calibrated (`TEXT_BYTES_PER_RADIX_UNIT`, ceilings.rs:203-216: at most 6 syntax bytes plus digits per value, one radix unit minimum, hence under 7), enforced by `assert_honest_text` (cell.rs:317-323) on every text stream entering a denominator on both the board (measure.rs:114-116) and bench (export.rs:78-80) paths, and tripwired (`rendered_text_is_honest_and_padding_trips`, board/tests.rs:108). Derivation, implementation, and tripwire all present. (Verified by reading; the tripwire's name and location, not its body.)
- `defect.rs` places every rejection defect maximally deferred with a per-shape derivation of why that position is the last discoverable one; the packed-side constructions are correct against the codings (a `00` terminal rewritten to `11 00 00` is exactly the one id canonicity rule left to enforce; an equal-sibling zero delta is exactly the skyline minimality violation); every walk is iterative; the `expect` messages are one-line proofs ("a stored stream is canonical"); `clock_trailing_text` correctly anticipates `parse_clock_str`'s O(1) outer-paren check (text.rs:188) by riding the defect inside the version component. (Verified by reading.)
- `coverage/tests.rs` checks the tiling in both directions (every surface row priced or excused, never both; every cited row live; every board row cited) plus duplicates, and every assertion message names the fix. The tables are plain `&[(&str, ...)]` data. (Verified by reading.)
- `export.rs`: the bench suite's criterion IDs are exactly the board's `(op, family)` names, `denominator_bytes` reads the output side back from the actual result, and `bench_cells` derives its rows from `ops()` and `FamilyId::board()`, so board coverage is bench coverage with no second enumeration of cells (the rider list, board-frame-25, is the one exception). (Verified by reading.)
- `measure.rs:84-123`: the metered region is exactly the body; every counter resets immediately before `(cell.body)()` and is read immediately after in a fixed order; denominators and the honesty assertion are settled after the reads and before `drop(result)`; peak heap is `peak - baseline` so pre-built operands do not count. (Verified by reading.)
- Cargo.toml:115-117 `required-features = ["limb-meter", "scan-meter"]` on the example, so the board of record cannot be built with three columns dark while still printing verdict colors. (Verified.)
- The `Denom`/`IoSpec`/`TextSpec` shape (cell.rs:161-199) with `fn(&dyn Any) -> usize` output readers keeps the ops rows declarative and makes a predicted output size unable to substitute for a measured one. (Verified by reading.)

## Open questions for Finch

1. Segments (board-frame-1): dissolve the currency, or keep it as a documented structural-zero pin at ceiling 0? Recommendation: dissolve. 1ddb5a483's reason for the dev-dependency move is that no library code recurses; a column that can only ever read zero in the binary of record names no failure it catches, and `LADDER_TOP_SCALE` should be re-derived from the four-point trend or stated as owner-ratified with its calibration story in the pin commit. The envelopes reviewer should confirm whether `tests/meter.rs`'s `segments: 0` pins (an integration binary, also without cfg(test)) are vacuous by the same construction; its module doc (:22-26) and :440-441 present segments as live.
2. Riders (board-frame-25): does the "every declared-model cell keeps a wall-clock witness" rule apply to all four ratified models (fold, capacity, family-stated heap, mirror-wide limb) or to the heap/limb pair the current test checks? Recommendation: derive the pinned subset from `declared_heap`/`declared_limb` and state at `BenchMode::Pinned` that the fold and capacity models are excluded because their cells sit on designed pairings by construction (pin that with a test if it must hold). Also: should `just all` run `bench-judge quick full`, or should export.rs:9-11 say the pinned subset is the verdict of record?
3. Validation index (board-frame-26): re-word the index to the envelope-class legs, or return the touch ceiling to class-scale and add envelope rows for the worst readers? Recommendation: re-word the index; the ×1.25 convention is owner-ratified and the cascade is stated as intended.
4. Public surface (board-frame-6): is the meter-feature board surface (the `pub use` constants, `ByCurrency`, `Currency`, `Floors`, `Liveness`) part of before's declared-stable API, or instrument surface that may be narrowed? Recommendation: narrow the fifteen unreferenced constants to `pub(super)` now; decide the doc-linked ten explicitly and record the decision at the `pub use`.
5. Root doc (board-frame-5): state a05918df7's summary-plus-pointer rule inline and replace verbatim copies with links? Recommendation: yes; the two verified drifts are the cost of the verbatim-copy criterion.
6. Vocabulary and em-dashes (board-frame-18, -20): schedule a crate-wide prose pass under the Documentation Change Policy? Recommendation: yes, as one pass; a partition-local fix would make this module inconsistent with roughly 140 other sites.
7. Process note from the history pass: b3f09baa0, titled "Partial WIP for docs pass, additional API tweaks", is on main and carries two of the stale counts found here ("four cells"; the reflowed "two n-ary fold rows"). Worth a look at whether WIP-titled commits should reach main.
8. Two items belong to the judgment partition but surfaced from this one's constants: judge.rs judges constants at `s2` alone while board.rs:222-224 says constants "stay judged per size across the ladder" and :71 says "at the larger scale" (one wording should give); and ceilings.rs:342 describes the fold exponent ceiling as fitted "across the cell's two probes" while judge.rs fits a four-point trend and computes the ceiling from the ladder's endpoints. Recommendation: settle both in the judgment review and align the wording here.
9. The riders test and the bench export build every family at 0.02 and run every prepare; is that scale a documented floor of applicability for every generator, or could a generator legitimately return `None` there and silently narrow the axis these tests reason over? Recommendation: name the constant once and state the premise where it is defined.

## Dropped

- Duplicates merged: candidates 14 and 40 into board-frame-1 (segments); 17, 30, and 43 into board-frame-25 (riders); 16 into board-frame-8 (log factor); 25 and 38 into board-frame-24 (denominator); 26 into board-frame-22 (preorder skeleton); 22 and 45 into board-frame-2 (board.rs drift) with their "four cells" halves into board-frame-3; 28 into board-frame-4 (dated rationale); 27 into board-frame-13 (`both_present_nodes`); 39 and 49 into board-frame-16 (`Cell` literals); 34 and 47 into board-frame-19 (tiling test); 21, 31, 32, 37 (the "today" and past-tense sites), and 48 into board-frame-18 (vocabulary); 44 into board-frame-23 (party text); 41 into board-frame-17 (`clock_hash`).
- Candidate 12 (bare `0b1000_0000` marker literal): folded into board-frame-21, whose one-byte cut removes the literal from defect.rs; the codec-side `[0x80]` is outside this partition.
- Candidate 13 (the board tiling as a second string-keyed roster): deliberate and documented; surface.rs:3-5 and :12-17 state inline that the roster is public under `meter` so instrument crates bind coverage tables by row name, and the tiling test is that mechanism's promised totality. A `SurfaceRow` field would couple the public roster to one consumer.
- Candidate 37's "has caught" (ceilings.rs:458-459): folded into board-frame-1 (the segment-onset rationale is the expired premise).
- Candidate 46's limb-calibration sub-item ("tens per packed byte ... over a hundred", ceilings.rs:88-90) and the ~8 bits per byte at :133: deliberately kept by 500d4d094 as order-of-magnitude calibrations with stated bands; board-frame-12 keeps only the two untriaged readings.
- Candidate 2's sub-claim that tests/meter.rs already pins the worst touch readers per scenario: refuted (its `TouchEnvelope` rows are all `RANK_*` scenarios); board-frame-26 carries the corrected form.

<!-- source: final/board-ops-render.md -->
# Partition board-ops-render: The board operation rows, renderer, shards, worst-case families, and board tests

## Partition summary

This partition is the amplification board's instrument layer under `crates/before/src/meter/board/`: the operation row table (`ops.rs`, 2,278 lines), the per-cell measurement seam and the printed matrix (`render.rs`, 302), the child-process wire protocol and merge (`shard.rs`, 651), the argmax fold and its committed ranking pin (`worst.rs`, 670), and the board's sibling unit tests (`tests.rs`, 1,284, the one test file). Total read: 5,185 lines in the five files, plus the neighbours every finding leans on (judge.rs, ceilings.rs, floors.rs, family.rs, cell.rs, measure.rs, export.rs, coverage.rs, registry.rs, recurse.rs, place.rs, the smoke suite, the runner example, the justfile's board recipes, and the cited git history). No cargo, just, or build command was run; one Python snippet reproduced `mechanism()`'s substring tests over the judge's verbatim label strings.

The structure is sound and, in several places, exemplary. Each row in `ops.rs` prepares a cell from bundle slots through `FamilyData`'s accessors and commits a typed `Liveness` declaration per currency; `measure_cell` measures exactly the body at two sizes; a child emits raw `Sample`s as a stamped, tab-separated line with floats carried as IEEE-754 bit patterns and the parent refuses any header that is not byte-for-byte the one it commissioned; the merge's completeness refusal compares the union against the registry's declared reach, and the smoke suite commits a tampered capture per family that must be refused; `worst::rank` records exact ties whole so the pin cannot flap; and `tests.rs` pairs every judgment leg with a known-bad artifact that reads red and a correct one that reads green, through `evaluate` itself. Every `expect`/`unreachable!` message in the five files is a one-line proof.

The dominant issues are of two kinds. First, instrument liveness: the `segments` column has no writer in the binary of record (its only incrementer is `#[cfg(test)]`), so every cell's `seg[...]` reading is a compile-time zero presented as a measurement and the ladder-top rationale rests on an onset that cannot occur there; the ascend-cliff family-stated heap ceilings carry no under-side band or class pin, contrary to the board module doc's commitment; the placement rows declare the touch column not applicable while the placement kernel's own doc states the pair-fold premise the floor needs; the membership and covers rows measure their early-exit verdict on nearly every family; and the delegating-parser floors' separation from the bypass reading is remembered in prose rather than checked per run (git records it once failing silently). Second, prose residue from two dissolutions: the mechanism tag's justifying comment cites an excised triage buffer (and the tag mis-classifies every floor trip as `constant+floor`), a test doc carries a dated measurement and a design-doc citation, and the acceptance criterion's doc names a retired determinism tripwire. The remainder is low items and nits: duplicated scale guards, a two-thousand-line table function under a clippy allow, literals shadowing named constants, undemonstrated protocol refusals, and vocabulary.


## Positives

- shard.rs stages a child's whole emission in memory and writes it only after the last cell (149-154), with the pipe-stall failure it prevents stated at the site; the family-outer deal (44-63) commits its own counterexample (an op-outer deal aliasing whenever the roster length and the shard count share a factor) to prose beside the code.
- The completeness refusal in `merge_samples` (497-520) is exactly the doctrine's shape: a total check on the union that no per-capture check implies, with a committed known-bad artifact (`merge_refuses_a_silently_shrunk_grid_for_every_family`) swept over the axis the refusal discriminates on, firing at every scale including the acceptance ladder.
- tests.rs pairs every judgment leg with a probe that reads red under the leg and green without it, in both directions: the bypass walk for the floors, the chunked schoolbook for the `n_io` exponent leg, the lump ladder and quadratic ladder for the four-point trend, the ratified/regressed/improved trio for the capacity band, the straddling pair for the heap guard, the fold model's log factor against a quadratic and a fat constant. Each goes through `evaluate` itself, not a description.
- `worst::rank` records exact ties whole and name-sorted so the pin cannot flap, and the near-tie flag is kept out of the pin on a stated argument (73-79); `render.rs` prints `-.--` for an unjudged exponent rather than digits a reader would mistake for a measurement (74-81).
- ops.rs asserts output honesty at prepare on the I/O-denominated rank rows (`rank_encode` 582-586, `ranked_encode` 707-712, `ranked_encode_rank` 738-742) with the bound's derivation inline, so padding the output side of a denominator trips the run rather than greening a cell.
- Floats cross the shard wire as IEEE-754 bit patterns (184-188, 283-288) with the reason stated, and the merge refuses any header that is not byte-for-byte the commissioned one, so the byte-identity the smoke suite asserts across shard counts is exact by construction; `required-features` on the example makes an unfeatured board a cargo error rather than a matrix that looks judged.
- Every `expect`/`unreachable!` message in the five files is a one-line proof (shard.rs:193 "a cell's applicability depends on the family, never the size"; shard.rs:507 "FamilyId::board() filters on the Board coverage answer"; ops.rs:183-184 the envelope-only derivation).
- `measure()` meters exactly the body: counters reset, heap baseline taken after the reset, the boxed result kept alive until every meter is read, I/O denominators settled from the actual result.

## Open questions for Finch

1. The segments column (finding 15): dissolve it from the board, or keep it with a disclosure that the counter has no writer outside the lib's test build? Recommendation: dissolve, and re-state `LADDER_TOP_SCALE`'s rationale on the doubling-chain onset (worst.rs:65-66 already names it); handle tests/meter.rs's segments pins in the same change.
2. The worst-case ranking pin: keep it as a gate leg, computed from the acceptance sweep (finding 19), or retire it to an audit view like the fuelscape atlas? Recommendation: keep it. f34f4b34 records a catch (a heap movement on freeze-parade and bigroot) that the envelope suite's limb-only lower bounds would not have surfaced, so it is not circularly justified; the tied-set rows that enumerate the roster do carry information (a family leaving a saturated tie is a reading change), so leave them.
3. `designed()`'s destination (finding 2): family.rs beside the bundle-build match (internal, matches registry.rs:569-571), or a field on the public `Coverage::Board` (exposes `OpGroup` under the `meter` feature)? Recommendation: family.rs.
4. The shard codec: an optional `serde_json` dependency under the `meter` feature, or keep the hand-rolled positional codec and add the tamper table (finding 16)? Recommendation: keep the codec; its refusals are inseparable from its parsing and the protocol is runner-internal; the tamper table is the actionable gap.
5. Membership and covers rows (finding 9): replace the early-exit probes with certifying ones, or add certifying rows beside them? Recommendation: replace (the admit and disjoint verdicts are O(1)-class on most families and `party_disjoint` already prices the full-examination disjoint walk); re-pin the two `WORST_RANKINGS` rows with the movement annotated.
6. The ascend-cliff under-side (finding 6): a banded floor like the capacity model, or a class-liveness pin like the mirror-wide model? Recommendation: a class pin, since both constants are documented as "conditional on exactly this flat-constant profile" and a pin reads red the day the profile changes.
7. Should `query_coverage` take the placement touch floor (finding 10) or keep an NA with a positively stated reason? Its two-probe walk has clamp legs the placement premise does not cover; recommendation: derive separately and state whichever answer positively.

## Dropped

- [6] Hand-rolled TSV wire codec where a derive would do: refuted on the merits (refusals interleaved with parsing; a new library dependency for an internal protocol); moved to open question 4.
- [4] The WORST_RANKINGS pin as a dissolution candidate: refuted (f34f4b34 is a demonstrated catch outside the pin, so the justification is not circular; several counted "re-pins" add rows or families); the surviving sweep-sharing point is finding 19 and the retention question is open question 2.
- [23], [55]: duplicates of finding 1.
- [24], [33]: duplicates of finding 2.
- [29], [30]: merged into finding 4.
- [27], [48]: merged into finding 12.
- [8], [37]: merged into finding 14.
- [21], [50]: merged into finding 16.
- [26]: merged into finding 19; its "66.7 s pin leg" timing is not in the tree and is dropped as evidence.
- [13], [25], [38], [53]: merged into finding 20.
- [36]: merged into finding 24.
- [11]: merged into finding 26.
- [22], [28], [52]: merged into finding 29.
- Refutation new item 1 (registry.rs:569-571 doc/code mismatch): folded into finding 2 as its lever.
- Refutation new item 2 (the "roughly half" clause): folded into finding 26.
- Refutation new item 3 (recurse.rs:74-75 "always written"): folded into finding 15's evidence.
- Refutation new item 4 (`# Panics` finiteness omission): folded into finding 17.

<!-- source: final/clock.md -->
# Partition clock: Clock: the party plus version pairing, forks, and the clock test suite

## Partition summary

`crates/before/src/clock.rs` (1082 lines) defines `Clock`, a `Party` paired with a `Version`, and composes every operation out of the two components' doors: `tick`/`ticks` delegate to `Version`, `fork`/`forks`/`join`/`sync` to `Party` (`fork`, `sum_split`, `join`) with the version cloned or joined alongside, `join_all` and `sync_all` run the shared `fold::balanced_try_fold` with `Clock::join` as the combiner, and the codec doors concatenate two byte-aligned encodings (`encode_to`) or parse the id once and adopt two slices of one read buffer (`decode`). The `|`/`|=` matrix over `{Clock, Version}` is generated by `clock_join_matrix!` and its shape (no `Clock | Clock`, no borrowing `&Clock` receiver) is pinned by `static_assertions` at the definition, beside `assert_not_impl_any!(Clock: Clone, Copy)`. `crates/before/src/clock/forks.rs` (106 lines) wraps `party::Forks` with a borrowed `&Version` and adds the consuming `From<Clock> for [Clock; N]` with its `const { assert!(N >= 1) }` and paired `compile_fail` doctest. `crates/before/src/clock/tests.rs` (1474 lines) is the test file: the join-fold differentials against the recursive oracle, the master op-trace differential, the protocol-semantics differentials (`sync`, heterogeneous joins, `sync_all`), the normal-form sweep, three depth-100k proofs, `decode_never_panics`, the paper's worked example, text and literal doors, serde and borsh legs, and four deterministic orbit pins. Total lines read in the partition: 2662, plus the neighbor sites each finding cites.

The production code is thin and, under every lens, close to clean. I found no wrong answer reachable on a 64-bit target: the panic sites are one-line proofs, `decode` cannot admit an anonymous party or a trailing byte, both n-ary folds drop nothing, and `sync_all`'s alias-carried fold makes "nothing observable moves on overlap" structural rather than narrated. The two medium findings on the production side are inherited from `party::Forks` and reach `Clock` through its public docs and its `ExactSizeIterator` impl: the public "exactly `k` children" contract is false at `k == u64::MAX` (the saturation is pinned by `tests/forks_max.rs`, which calls it "the documented behavior", while the only surviving documentation is a private comment; git shows the public sentences were written at cdad4606 and removed by three undescribed doc-editing commits), and `len()` on a 32-bit target panics for counts past `usize` because `size_hint` returns `(usize::MAX, None)` under an `ExactSizeIterator` impl whose default `len` asserts `upper == Some(lower)`.

The test file carries the dominant issues, and they are prose-and-instrument issues rather than wrong assertions. A 50-line regression section describes a bit-packed clock framing the codec lost at 32a655438 (2026-06-18) and states the inverse of what `Clock::decode` does today; three other comments contradict the code beside them. The depth-100k proof that `AGENTS.md` names as the no-recursion rule's evidence says "every public op" while driving a listed subset over one left-only spine family, so both-present id frames and right-lean descents have no overflow-depth witness anywhere in the crate. The static-orbit pin's number is right and deterministic, but its stated mechanism (eight counters growing one bit each per doubling) contradicts the gamma coding, and its exchange schedule degenerates into four fixed partner pairs (I enumerated it), so it transcribes a two-process topology rather than the paper's scenario and carries a dead collision branch. Alongside these sit a cluster of documentation drifts left by the deliberate `k`/`iter` parameter rename (2efff149) that never reached the prose, and a handful of nits.

What is done well is worth naming: the `|` matrix's linearity argument is a compile-time fact, `sync_all` and `decode` state their why at every branch, the fold hand-back discipline has a deterministic witness plus a proptest and a known-bad oracle convicted in the party twin, the orbit pins state liveness floors and judge shape over point, and `deep_tree_stack_safety` reasons explicitly about which fast paths could make its sweep vacuous.


## Positives

- The `|` matrix's shape is a compile-time fact, not a convention: `assert_not_impl_any!(Clock: Clone, Copy)` sits at the definition with the linearity argument beside it (clock.rs:57-62), and the four static assertions at 1072-1082 pin "no `Clock | Clock` in any borrow shape" and "no `&Clock` receiver", with the comment at 1060-1071 stating exactly what the shape prevents (two holders of one share).
- `sync_all` (377-425) achieves "nothing observable moves on overlap" structurally, folding `O(1)` `dangerously_alias` handles and committing only on success, and its comment (383-396) says why no up-front accept test is needed here while `join_all` needs one.
- `Clock::decode` (787-823) parses the id once, validates both components against the borrowed buffer in the order the component decoders check, and adopts two slices of one read buffer; the truncation-genre reasoning at the id/version boundary is stated where the check lives, and the flush-cut branch has committed tests at every door in codec/tests.rs.
- Every `expect` and `unreachable` message in the partition is a one-line proof ("writing to a Vec is infallible"; "the id prefix ends within the read buffer" after the bound check at 808; "a split into k + 1 >= 1 shares yields a residual leaf").
- forks.rs pins the `N >= 1` bound with a `const { assert! }` at monomorphization and a paired `compile_fail,E0080` / compiling doctest twin (73-81, 97-101), so the compile-time guarantee is itself tested.
- The fold hand-back discipline has a deterministic witness for the over-full counter slot (tests.rs:65-93), a proptest over arbitrary mixes with repetition (141-169), and, in the party twin, an executable known-bad oracle convicted by the same differential: the differential's adequacy is demonstrated, not asserted.
- `deep_tree_stack_safety` is careful about vacuity: it snapshots `early` and explains (580-583, 606-612) that equal operands short-circuit on `canonical_eq`, so the join/meet sweep is driven on two distinct deep versions rather than through the fast path.
- The orbit pins (1218-1474) state a liveness floor per test (a mid-round non-identity, an exact closed form, a population count asserted every round) and judge shape across the whole trajectory at octave resolution; the fork+tick+join closed form `7 + 2·⌊log2 k⌋` matches a hand derivation from the skyline coding (3 topology bits + gamma(0) + gamma(2k)).
- The `sync` proptest (356-366) derives two distinct indices arithmetically instead of `prop_assume`, with the reason (the reject cap under high case counts) stated at the site.
- The `Clock` type doc (22-50) is model public rustdoc: the first sentence stands alone, the operation table answers which-do-I-use, and the deliberate absence of `Clock | Clock` is stated with its reason.

## Open questions for Finch

1. `Forks::len()` on 32-bit targets (clock-17): among (a) dropping `ExactSizeIterator` from the three iterators, (b) taking the count as `usize`, and (c) keeping the impl and documenting a `# Panics` (or a clamp) past `usize` on narrow targets, which contract do you want? Recommendation: (a). An `ExactSizeIterator` whose `size_hint` cannot be exact on a pinned gate target is a contract the type cannot keep; `len()` callers can use the count they passed. If `len()` must stay, (c) with `# Panics` is the honest minimum.
2. `from_parts`/`into_parts` carry measured fuelscape islands while `seed`, `party`, `version`, and `dangerously_alias` state `O(1)` inline. The roster's rule is stated and enforced (an exemption stands on a missing size axis or mechanism identity; `from_parts`/`into_parts` have a size axis, so they are measured, and the island-totality test requires the include). Do you want a third exemption genre for trivially-`O(1)` operations with a size axis, or is the atlas's totality worth the two expanders? Recommendation: keep the rule as it stands; it is coherent and the cost is two doc expanders.
3. Em-dashes in `//` comments: the partition has 17 such lines but crates/before/src has 374, and the one style sweep on record (dfd19c44) never touched before/src. A partition-local sweep would create inconsistency. One-shot crate-wide conversion to the doctrine's spaced double-hyphen, or accept em-dashes as before's house style? Recommendation: one mechanical crate-wide sweep in a commit of its own, not piecemeal.
4. The `join_all` differentials pin the hand-back vector's contents and order against `oracle::Clock::join_all`, a hand-copied spelling of the same counter (tests.rs:24-34 makes this deliberate), while the public contract says the absorbed/handed-back set is unspecified. Is the counter's grouping and hand-back policy a fixed private contract? Recommendation: yes; say so in one sentence in fold.rs ("the discipline of record; the oracle spells it independently"), so a future counter change knows it must move the oracle too.
5. coverage.rs:110 delegates `Clock::sync_all`'s cost to the `version_join_all` and `party_join_all` cells; `sync_all`'s own remainder (the `Vec<&mut Clock>`, the alias vector, the `forks(shares)` re-share) is `O(n)` handles plus a banded split. Should coverage.rs name that remainder explicitly, as it does for `Clock::join_all`'s hand-back at 25-27? Recommendation: yes, one clause.
6. Crate-wide (16 sites): "Auxiliary space is `O(|self| + |iter|)`" relies on `|x|` being defined as encoded bytes of an argument; for an iterator, is `|iter|` the total encoded size of its items or their count? Recommendation: define it beside `|x|` in lib.rs in one line.

## Dropped

- [5] `clock_join_matrix!` takes two never-varying parameters: history shows the shape mirrors `binop_matrix!` (which does vary its island) by deliberate uniformity at 2efff149; taste without a named cost. The `$opdoc` inaccuracy on the `|=` cells survives as clock-15.
- [10] `from_parts`/`into_parts` fuelscape islands: the roster's inclusion rule is stated and enforced (before-fuelscape ops.rs:2167-2172; fuelscape_islands.rs:16-22); a question about the rule, moved to Open questions.
- [29] Em-dashes in `//` comments: crate-wide house style at 374 sites; a partition sweep would create inconsistency. Moved to Open questions as a crate-wide decision.
- [32, first half] `encoded_bits` doc names instrument consumers: transcribes the owner ruling recorded in 05d87e1b; deliberate and documented.
- [30, partial] "the truncation genre" and "boundary-band arities": anchored vocabulary (codec/bits.rs:465-473; generators.rs:393-400), removed from clock-24's list.
- [38, `|c|` denomination] `Forks`'s early-drop cost in the clock's size rather than the party's: follows 3bba6cbb's rule that every complexity section is denominated in the operation's own argument names; deliberate.
- [41, forks drain] "no enforced instrument for `forks`": refuted; `ff_party_forks` has a committed fuzzfit band in the gate's wasm stream (bands.rs:568-579, 1010). The residual gaps are clock-9.
- [0] as stated ("dead allows"): refuted; both allows are live (closure inspection confirmed in the pinned clippy branch; `(Clock, Clock)` is exactly 128 bytes). Reframed into clock-5.
- [15], [22], [35a], [3], [18], [35b], [35c] duplicates of clock-25.
- [34], [42] duplicates of clock-3.
- [38, part], [45], [11] duplicates of clock-2.
- [39] duplicate of clock-11.
- [44] duplicate of clock-10.
- [36] merged into clock-22.
- [40] merged into clock-28 (the refutation's note that the churn guard is live is folded into its resolution).
- [14] merged into clock-12.

<!-- source: final/codec-base-text-tree.md -->
# Partition codec-base-text-tree: The arbitrary-precision Base (with its limb meter), display, text notation, the id-tree parser, and the codec test suite

## Partition summary

This partition is the codec's arithmetic and notation kernel. `Base` (`crates/before/src/codec/base.rs`) is a thin newtype over `dashu_int::UBig`: every arithmetic, comparison, equality, and hashing operator records its operands' 64-bit limb widths through the shims in `base/limb_metered.rs` (which compile to nothing without the `limb-meter` feature) and then delegates to the backend whole; `base/limb_meter.rs` holds the process-global counters and the argument for what this currency sees that the heap and scan meters cannot. `MsbWindows` and `msb_cmp_windows` stream an MSB-aligned comparison for the rank's class-tie ordering. `tree.rs` parses the packed 2-bit presence-tag id grammar on an explicit heap frame stack and wraps it as `validate_id`; `text.rs` parses the paper's `0 | 1 | (i1, i2)` id notation on its own frame stack, reads decimal bases by delegating the radix conversion to the backend, and splits a stamp `(i, e)` into its id bits and event text; `display.rs` renders an id with one to two control bits per open node on a `BitsBuf`. `codec/tests.rs` (1976 lines) and `base/tests.rs` (179 lines) are the test files; the remaining six files total 1047 lines of production code, 3202 lines read in all.

The production code is close to its simplest shipped form and honors the crate's hard rules everywhere I looked: no walk recurses on depth, every `expect` message is a one-line proof, the id normal form has exactly one wire-expressible violation and `parse_id_core` rejects it at every node close, and the 32-bit totality of `bit` and `Shr<u64>` is argued at the code. The test suite is the partition's strength: the id text parser is pinned by an exhaustive small-scope differential against a deliberately recursive reference plus whitespace-injected round trips, single-edit mutations, point pins, and a 100k-deep round trip; the build-history family is a lockstep model-based test of the buffer's representation invariant at every intermediate state; the padding and flush-cut families give every decode door a per-door witness for both rejection classes.

The dominant issues are prose and instruments that outlived the change that justified them. Three commits explain most of them: the 4-state pruned id encoding (2026-06-18) forced inline collapsible-pair checks into the text and literal doors and left the shared `validate_id` pass and its "single source of truth" sentence in place; the move of `Base` onto dashu (2026-07-24) removed the constraints behind the `Sub` debug assertion, the `SubAssign` clone, and the "spills at `u64`" test doc; and the recursion experiment and its same-day revert (2026-06-02) left a test doc describing a recursive validator. Two findings rise to medium: `base_dispatch_read_touches_no_digits` asserts a counter that `to_word` cannot reach, so any implementation passes it; and the `Party` literal door's public `O(n)` claim is quadratic on a nested spine because each level copies and re-validates the subtree below it. Two public-contract corners are owner questions: the clock door trims Unicode whitespace where every other door is ASCII-only, and the id and version text doors disagree on whether trailing junk outranks `NotCanonical`. The rest is low-grade simplification (a redundant validator pass, a two-pass stamp split with a depth counter that generated its own 2 GiB witness, a `BitsBuf` used as a stack where the crate owns `BitStack`) and nits.


## Positives

- `MsbWindows<I>` and `msb_cmp_windows` (base.rs:148-241): a streamed MSB-aligned comparison with one register of carry, no shifted copy, and cost exactly O(shared-prefix limbs) because the loop stops at the first differing window. The generic over the reversed limb source pays: the stored magnitude (via `suanpan::Limbs`) and the rank's wide limb vector stream through one implementation. I walked the limb-aligned, partial-top-limb, single-limb, and zero cases against the `next` arms and the doc holds, including "a zero value has no windows".
- `limb_metered.rs` keeps every operator body in base.rs to the shape "meter, then delegate" with no `cfg` at the call sites (the two exceptions are finding 2), and `limb_meter.rs:1-27` states precisely what this currency sees that the heap and scan meters cannot and why densified fill is a separate column.
- The Shl/Shr totality paragraph (base.rs:466-478) is a model of the one-line-proof rule: it names the exact values the `expect` can fire on, shows the backend's own capacity assert bounds the same set, and explains why the right shift clamps rather than fails; 32-bit totality is handled by clamping, not truncation (`bit` at 46-50, `Shr<u64>` at 491-500).
- `tree.rs:29-34` states the id grammar exactly where the parser lives (2-bit presence tags, `0` as structural absence, no empty production) and derives the Truncated-on-empty rule from it; the `summary` comment at 66-67 says what the code cannot show and nothing else; `IdFrame`'s doc (5-10) prices the frame stack against the tag reads in one sentence.
- `display.rs::write_id` documents the one place `IdNode::Empty` can fire (39-40, which checks out against idbits.rs:38-39 and 82-88), keeps its control state to one to two bits per open node with both phases named at their constants, and its single `expect` (62) is a one-line proof true by the push order at 45-46.
- `text.rs:192-193` is a one-line overflow proof at the declaration (finding 19 argues the counter should go, not that the proof is wrong); `parse_base` slices the whole digit run and delegates radix conversion to the backend, with the metering convention stated at `Base::parse_decimal` and cross-referenced from the gamma decoder.
- Every `expect` in the partition is a one-line proof (base.rs:91, text.rs:65, display.rs:62), and no assert, expect, or debug_assert message in the partition contains an em-dash (mechanically checked).
- The codec test suite: the build-history family (tests.rs:212-384) is a lockstep model-based property test of the buffer's representation invariant at every intermediate state; the mutation family (1225-1469) states the accept-canonically disjunction once and applies it to single-bit flips, byte truncations, and every padding position across all three doors; the padding and flush-cut families (914-1594) give every marker-padded door a per-door witness for both rejection classes, including the interior seams, and explain why round-trip checks alone would be blind to intra-byte padding defects; `base_eq_hash_agree_with_derived_semantics` (714-779) pins the manual metered `PartialEq`/`Hash` against a `#[derive]`d mirror, a clean way to state "metering never changes an answer".
- The id text parser pin (tests.rs:1634-1922) is complete along every axis that matters: exhaustive to length 7 over the grammar alphabet, a recursive reference whose error precedence mirrors production, rendered ids with pseudo-random ASCII whitespace injection round-tripping to `party.as_bits()`, single-character insert/delete/replace mutations, point pins for precedence and the bare `0`, and a 100k-deep render-and-parse round trip.
- `base/tests.rs` drives both dispatch arms against an exact `IBig` oracle with the sign checked after every step, and the pattern of pairing a zero-assertion with a liveness leg (79-84) is the right one, even though finding 6 shows the particular counter cannot see the operation under test.

## Open questions for Finch

1. Does `validate_id` stay in `parse_id_str` and `id_node` as a deliberate totality defense (every door's bits pass the wire validator before storage), or go? Recommendation: delete both calls and move the canonicity assertion into the test helpers (finding 16); if kept, the rationale belongs at the call sites and the "single source of truth" sentence goes either way.
2. Which `Parse` precedence is the notation's: the id door's per-node rule (`NotCanonical` at the node's `)` even with trailing junk) or the version door's whole-pass rule (any syntax defect, trailing junk included, outranks `NotCanonical`)? Recommendation: the whole-pass rule, documented on `Parse`; it is the simpler statement ("only well-formed strings get canonicality judgments") and the reference parser needs only a `canonical` flag to mirror it (finding 18).
3. Is the comment half of the em-dash rule live for this repo? The repo has applied it to messages; 374 em-dash `//` comments exist in before/src (23 in this partition) and were still being added on 2026-08-18. Recommendation: rule once, then sweep crate-wide with the grep in finding 24 rather than partition by partition.
4. Under the `meter` feature, `before::meter::skyline::query::min_ticks` returns `Base` while `Base` has no public path. Does the meter surface count as stable API for the purpose of trait impls on `Base` (finding 8's gate), and does the surface-totality check see `Base` as an unnameable type in a public signature? For the surface-roster reviewer as much as for the owner. Recommendation: treat the meter surface as an instrument surface, not the stable API, and record that ruling where the surface check reads it.
5. For the literal-door claim (finding 13): restate the public rustdoc as `O(n · d)`, or reshape the sealed `PartyLiteral` to emit top-down into one buffer and keep `O(n)`? Recommendation: the reshape; the trait is sealed and hidden, so it changes no reachable surface.
6. After removing `Sub`'s `debug_assert!` (finding 7), which limb envelopes in `tests/meter.rs` move, and by how much? This needs a measurement run at the parent commit, which this review did not perform; the re-pin should attribute the movement to the assert's removal.
7. Should the `BitStack` migration (finding 12) cover all ten `BitsBuf`-as-stack sites in one change so `BitsBuf::pop` can be deleted in the same commit? Recommendation: yes; a partial migration leaves the two disciplines coexisting, which is the finding.

## Dropped

- Five `From` impls for `Base` could be one generic impl (candidate 14): below the bar; a blanket `impl<T> From<T> for Base where UBig: From<T>` would also admit `From<bool>` and `From<usize>` (dashu-int 0.5.0 `convert.rs:717-719` implements both for `UBig`), widening the surface on a type whose contract is event counts, and the owner may prefer the explicit width list.
- `parse_base` defines the grammar by contrast with the deleted digit-at-a-time loop (candidate 19, ghost-reference half): refuted by history; the digit-by-digit strategy survives as the board's `schoolbook_limb_ops(text, 1)` probe (meter/board/tests.rs:221), so the comparison has a live referent. The duplicated-paragraph half survives as finding 14.
- `AddAssign<u32>` and `From<u32>` have no production caller (candidate 34, second half): refuted by grep; `Base::from(0u32)` at meter/board/defect.rs:102 and meter/board/tests.rs:266, 620, 627 and `acc += 1u32` at meter/board/tests.rs:623 are instrument-side callers.
- `Shl<i32> for Base` has no caller (candidate 46): reframed into finding 8; the impl is what lets the two unsuffixed `<< 64` literals type-check, so it is not dead, it is load-bearing for a test spelling.
- Candidates 15, 16, 35, 48, 51 (recursion prose; `parse_id_from`): duplicates of finding 23. Candidate 17 (`u64` spill): merged into finding 23.
- Candidates 20, 32, 43 (`validate_id`): duplicates of finding 16. Candidates 22, 33, 50 (`Sub` assert): duplicates of finding 7. Candidate 49 (`SubAssign` clone): merged into finding 7; its `from_utf8` half is finding 15.
- Candidate 38 (`BitsBuf` phase stack): duplicate of finding 12. Candidate 44 (single-pass stamp split): duplicate of finding 19. Candidate 45 (Unicode trim): duplicate of finding 20. Candidate 39 (`sep` and `SEP`): duplicate of finding 11. Candidate 40 (limb formula): duplicate of finding 2. Candidate 47 (1.49): duplicate of finding 3. Candidates 26, 37, 52 (dead `continue`, hasher, count): duplicates of findings 28, 22, 26. Candidate 53 ("honest", ragged wrap): merged into findings 25 and 1.
- Candidate 25's "Iterative." and repetition items: merged into finding 17 with candidate 13.
- The `Rank` Ord site's "historical path" wording (num.rs:281) and the `tests.rs:1154` paper citation (§3 versus §5.3.4): the first belongs to the rank partition and dissolves with finding 4; the second is loose but defensible and below the bar.

<!-- source: final/codec-bits.md -->
# Partition codec-bits: The bit-level substrate: bits, buf, build, code, cursor, dsi, gamma, int, literal, scan, stack

## Partition summary

This partition is the packed-bit kernel every coding in `before` stands on. `bits.rs` holds the frozen storage form (`Bits`, a marker-padded `Bytes` whose byte string is injective on bit streams, so `Eq` is one `memcmp` and `len` one `trailing_zeros`) and the borrowed `BitsView`; `buf.rs` holds the mutable `BitsBuf` under two invariants (exact bytes, zeroed dead bits) and the `seal_padding` step at the freeze; `build.rs` is `PackedBuilder`, the append/reserve-patch/splice/truncate move set the id and skyline emitters write through, with the scan meter applied at its primitives; `code.rs` and `int.rs` keep narrow payload codes and decoded integers in machine words end to end; `cursor.rs` defines `BitCursor` and the per-bit `SliceCursor`; `dsi.rs` is the word-parallel `DsiCursor` over `dsi-bitstream`'s buffered reader, with the accept/reject boundary kept in the wrapper; `gamma.rs` is the Elias-gamma code with a one-window fast path whose contract is that the per-bit loop remains the sole arbiter of every reject; `literal.rs` builds id streams for the `Party` literal door; `scan.rs` is the process-global scan meter hook; `stack.rs` is the word-backed `BitStack` and the bit-priced `PopStack` the deep walks hold their paths on. I read all fourteen files in full (2995 lines; `dsi/tests.rs` and `stack/tests.rs` are the two test files), plus the sibling `codec/tests.rs` ranges the findings cite, `codec/tree.rs`, the consumers in `party.rs`, `version.rs`, `clock.rs`, `borsh_impls.rs`, `version/skyline/{build,masked,overlay,query,signed}.rs`, the wasm32 pins, and the pinned sources of `dashu-int` 0.5.0, `bytes` 1.11.1, and `dsi-bitstream` 0.10.1.

The substrate is in good shape where it matters most. On 64-bit targets I found no reachable panic: every `expect` message is a one-line proof I could discharge, the window decoder declines every case it cannot prove, marker padding plus the one-bit-minimum grammars make the stored bytes injective, and the `BitsBuf` build-history family and the `DsiCursor` differential suite pin the two representations and the two readers against each other at every cut point. The prose is accurate at the contract level, with `# Panics` sections matching their asserts and the maintainer-facing "why" present where the code cannot show it (phantom zeros, the truncation-reject record, the `Truncated` ZST).

The dominant issues are three. First, a duplicated substrate: since the byte-backed `BitsBuf` landed, `PackedBuilder` keeps a second byte store plus a sub-byte staging register and reimplements six primitives `BitsBuf` already carries, at the price of two invariant sets and a per-bit copy in `extract_code`'s wide arm; the register's saving is a hypothesis nobody has measured. Second, verification gaps at the kernels: `PackedBuilder` and `BitsView::load_be` have no direct model test, and the `BitStack` differential names a method that does not exist, omits `set_last` and `trailing_ones`, and reaches the word spill it advertises with probability 4.0e-5 per case (computed exactly), so the multi-word `trailing_ones` loop appears to run under no oracle-checked test. Third, two claims contradicted by the code: `peek_flip` re-scans a parked cursor's trailing run on every masked or projection step, a term the derived linear bound omits and no meter sees; and on 32-bit targets the wide-gamma width guard admits 32 values past dashu's capacity cap, so a decoded stream can panic inside the backend where the comment promises a reject. The rest is prose: a false sentence about the builder wrapping a `BitsBuf`, a stale consumer roster in `cursor.rs`, ghost references to the retired `store_be` path, a drifted record-site roster in `scan.rs`, a mislabelled `k = 65` witness, and vocabulary the crate never anchors.


## Positives

- `BitsBuf`'s two representation invariants (exact bytes, zeroed dead bits; buf.rs:5-21) make byte equality one `memcmp`, sealing a single push, and the freeze a move; the build-history family in `codec/tests.rs` (`build_history_spelling_is_a_function_of_content`, `build_history_spellings_are_injective`) drives the whole move set against a clean rebuild at every intermediate state. This is the model codec-bits-15 asks `PackedBuilder` to inherit.
- `DsiCursor` keeps the accept/reject boundary in the wrapper, never the dependency (dsi.rs:9-12), refuses `dsi-bitstream`'s capped `read_gamma` with the reason stated (14-18), and is pinned to the per-bit loop at every cut point, across word seams, and on arbitrary interleavings (`dsi/tests.rs`). `gamma.rs:142-148`'s "the per-bit loop is the sole arbiter of every reject" is exactly the discipline that makes a fast path reviewable, and `gamma_word_paths_match_on_arbitrary_bytes` pins it on arbitrary bytes.
- `DsiCursor::truncated` (dsi.rs:124-136) records the examined tail before a reject surfaces so the truncation-reject scan floors stay live; the comment names the instrument it serves. The same instrument-aware prose sits at dsi/tests.rs:63-73 (`skip_int`'s two-sided floor witness).
- `Truncated` (cursor.rs:7-24) is a ZST whose doc names the concrete cost it removes (drop glue per successful bit read) and where the rich error is still paid.
- Every `expect` message in the partition reads as a one-line proof I could discharge (`a partial byte exists`, `the mantissa was proven to fit the live length`, `a mid-byte start has at least its own byte to skip within`, `buf holds 8 whole bytes`), and every asserting `pub(crate)` entry carries a `# Panics` section matching its assert.
- `Bits::ptr_eq`'s contract (bits.rs:200-216) states precisely what a rung may derive (equality, never clone provenance), names the counterexample that constrains it (`is_disjoint` on the empty share), and points at the committed seed; the injectivity argument is stated once at the type and reused by `canonical_eq`/`canonical_hash` without restatement.
- `padding_is_canonical`'s slice patterns (bits.rs:446-453) and `require_marker_padding`'s genre split (467-473) read as evidently correct; the flush-boundary `[0x80]` corner is handled by pattern, not arithmetic.
- `scan::record_bits` compiles to nothing without its feature (scan.rs:49-59), so every primitive calls it unconditionally, and the pricing convention (bits examined, however the path batches them) is stated at each batched record (cursor.rs:128-131, dsi.rs:178-179).
- `PopStack` prices depth in bits with the cost model stated (stack.rs:187-195) and is pinned against a `Vec` model across all `1..=64` widths, the testdoc explaining why uniform widths matter for the 62-continuation cap (stack/tests.rs:43-51).
- `Code` and `Int` keep narrow payloads in machine words end to end with the parking rule spelled out (int.rs:22-24, 56-62); `DsiCursor::read_int`'s wide arm routes through `Int::from_base`, so a `k = 64, rest = 0` value that fits a word lands `Small`, matching the per-bit path.
- `id_is_empty`'s debug assert (literal.rs:8-14) is justified by naming what a fuller check would break: dev builds would meter a different program than the release board.

## Open questions for Finch

1. Is the staging register's saving (no `Vec` traffic on sub-byte appends) measurable on the bench corpus? codec-bits-12's resolution is construct-and-measure. Recommendation: build `PackedBuilder { out: BitsBuf }`, run the judge at parent and change on a quiet machine, and take the consolidation unless a cell leaves its band; if the register wins, move the register form into `BitsBuf` instead so one implementation remains.
2. codec-bits-28 offers two shapes: delete the dead arm (minimal), or make `push_bits`/`pop_bits` total on `1..=64` and delete `PopStack`'s four `width == 64` splits. Recommendation: the deletion now; the larger change only if the splits bother you when you next touch `PopStack`, with `pop_stack_matches_a_vec_model_across_all_widths` as the oracle.
3. Is bits.rs meant as the single home of the rung policy (306e2de0 says so, the file does not)? Recommendation: keep it the home, say so in the essay's first line, and reduce the sites to pointers, so the policy lives once.
4. codec-bits-23's shared predicate encodes dashu's `Word` width. The alternative bounds the mantissa width by the input's own byte length (a `k`-bit mantissa cannot exceed `8 * bytes` on any path), which `DsiCursor` already knows and the borsh reader could learn. Recommendation: the shared predicate, with the derivation stated beside the existing dashu-pinning rule; it is the smaller change and matches 05d87e1b's intent.
5. Should covcheck's scope extend from `version/skyline/` to `codec/` so the kernels' branches (`load_be`'s shift merge, `trailing_ones`'s spilled-word arm, `PackedBuilder::truncate`'s two arms) are pinned covered by name? Recommendation: yes, once codec-bits-15 and codec-bits-30 land, since the roster then has tests to point at.
6. Should the board carry a currency for stack-word reads? Recommendation: charge 64 scan bits per word read inside `trailing_ones` (cheap, and it makes the peek visible to every existing floor and ceiling), plus the dual family from codec-bits-29.
7. Is an `Int::Wide` parking a word-scale value constructible in production? `Int::from_base`, `Int::from_ubig`, and both readers park such values as `Small`; if the signed arithmetic never leaves one parked wide, the `Some(b) => a.cmp(&b)` arms of `cmp_magnitude` (int.rs:70-77) are dead and unpinned. Recommendation: a committed witness or dissolve the arms; I did not trace the arithmetic and report it as a question, not a finding.
8. `crates/before/src/codec/tests.rs` (1976 lines) is the `mod tests` of codec.rs but is in no partition's file list; this review checked only the ranges its findings cite. Recommendation: assign it explicitly.
9. dsi.rs:255-265 and gamma.rs:249-251 classify a mantissa width that does not fit the target as `Decode::NotCanonical`, arguing the reject genre is the value's, not the machine's. On a 32-bit target that labels a canonically coded but unrepresentable value non-canonical. Recommendation: keep the genre (a public-API variant is the alternative) but say in `Decode::NotCanonical`'s doc that representability on the target is part of what the doors admit.

## Dropped

- [39] SliceCursor has no multi-bit read, so the id parser pays two reads per tag: refuted; `parse_id` reads through `DsiCursor` (tree.rs:36), so the proposed `SliceCursor::read_bits` would not touch `Party::decode`.
- [37] u64 word width for `ByteWords`: reframed; a 64-bit word forces `dsi-bitstream`'s 128-bit buffer, which its specialized refill exists to work around; the gather fast path survives as codec-bits-18.
- [13] injectivity and pricing-rule restatements: dropped from codec-bits-6; they sit at the sites that establish or rely on the argument, which the owner's writing doctrine and 7ea3df58/05d87e1b sanction.
- [19] duplicate of codec-bits-1 (ladder essay placement).
- [15] merged into codec-bits-16 (cursor.rs prose).
- [21] duplicate of codec-bits-24 (literal.rs module doc).
- [22], [28], [32] duplicates of codec-bits-28 (dead `len == 64` arm; 32 supplied the 35a09c5b provenance).
- [27], [34] merged into codec-bits-30 (BitStack model test) and codec-bits-15 (PackedBuilder and `load_be` model tests).
- [33] duplicate of codec-bits-27 (O(1) claim).
- [35] duplicate of codec-bits-8 (`[0x80]` at pos 0), at nit.
- [38] duplicate of codec-bits-25 (id_node re-validation), at nit.
- Refutation new 1 (gamma.rs:15 "one store") merged into codec-bits-20; new 2 (codec/tests.rs:168-172 mechanism) merged into codec-bits-7; new 3 (`extract_code` at `n == 0`) merged into codec-bits-13; new 4 was a correction to codec-bits-5's evidence, applied.

<!-- source: final/crate-root.md -->
# Partition crate-root: Crate root and cross-cutting: lib docs, error, fold, iter, auto traits, shape, recurse, serde and borsh impls, build.rs, Cargo.toml

## Partition summary

The crate root is where `before` states its contract and hosts the machinery every other module shares. `lib.rs` carries the crate-level docs (the model, the safety rules, the space and asymptotic promises, the feature list) and the module tree; `error.rs` the four error types; `fold.rs` the balanced binary-counter reduction that every n-ary operation runs on; `shape.rs` the public step-function vocabulary (`Plateau`, `Region`, `Cell`, and their iterators); `recurse.rs` the test-only stack-growth guard and its segment counter; `serde_impls.rs` and `borsh_impls.rs` the two serialization doors over the one canonical wire form; `auto_traits.rs` the compile-time `Send + Sync + Unpin` pins; `build.rs` a pure formatter that renders the committed fuelscape datasets into rustdoc islands and derives the README figure from the measurement SVG; and `Cargo.toml` the features, the `unexpected_cfgs` roster, and the `required-features` guard on the board example. I read all fourteen partition files in full with line numbers (3838 lines) plus the cross-file anchors each finding rests on (meter.rs and the board modules, tests/meter.rs, party.rs, clock.rs, version.rs, ticks.rs, skyline.rs and its shape and overlay modules, codec/stack.rs, laws.rs, surface.rs, surfacecheck, the fuelscape datasets, results/space_consumption, and the serde_core, serde_bytes, ciborium, and thiserror-impl sources in the local registry). The test files are `shape/tests.rs`, `serde_impls/tests.rs`, and `borsh_impls/tests.rs`. No cargo, just, or build command was run; every "verified" below means read or mechanically checked (grep, git, a python parse of committed data), never executed, except where a finding says otherwise.

The code itself is in good shape. Nothing in the partition is unsafe, nothing recurses on input depth in a library file, and the only panic sites in production code (fold.rs's two `expect`s, borsh_impls.rs's buffer index and shifts) are programmer-error-only with messages that argue them. `ReaderCursor` in borsh_impls.rs is the model wire door: one byte pulled per demanded bit, the word window provably unable to touch a byte the reader has not yielded, the read buffer adopted as storage without a copy, and a per-bit reference cursor that asserts byte consumption on accepts and rejects alike. `fold.rs` is one home for the counter discipline with the quadratic left-fold failure named beside it. `build.rs` panics on repository defects and computes nothing. `Cargo.toml` closes two quiet-failure paths mechanically (`unexpected_cfgs = deny` and `required-features` on `amp_board`).

The dominant issues are in prose and instruments, not code. First, the crate-level docs make three quantitative or asymptotic promises that either have no committed instrument or contradict the crate's own per-operation contracts: "approximately 100× more space-efficient" (no measurement exists), "asymptotically linear" (rank's committed contract is `O(M(|self|) · log |self|)`), and "100 parties and 1,000,000 events" (the committed artifact runs 4..128 parties at 100k/25k iterations). Second, the stack-segment counter that the board and the meter envelopes judge has no writer in any binary that judges it: its one `fetch_add` is `#[cfg(test)]`, while the readers compile under `any(test, feature = "meter")`, so the "measured fact" recurse.rs describes is a compile-time zero. Third, the serde impls serialize as the data model's `bytes` type and deserialize by requesting a `seq`; the three tested formats bridge the two, but a conforming strict Deserializer rejects what `before` itself wrote. The remainder is a scatter of hand-maintained rosters that drifted (auto_traits, fold's caller list, the `Parse` and `Overlap` producer lists), ghost references (the crates.io description, AGENTS.md, a `crate::bookmark` link into rumors), a public definition of *plateau* the code contradicts, and idiom nits in the test files.


## Positives

- borsh_impls.rs `ReaderCursor` (24-127): one byte pulled per demanded bit, the invariant `position <= 8 * bytes.len()` stated on the field that carries it, the `u64` position justified by the 512 MiB 32-bit seam, and the word-window fast path argued to be incapable of speculative reads ("the window's proven bits end at the buffer's end"); `finish` hands the buffered bytes over as storage without a copy. This is reviewably-correct performance code.
- borsh_impls/tests.rs: the per-bit `BitwiseReaderCursor` reference with byte consumption asserted on accepts and rejects alike (434-451 and siblings) over canonical, bit-flipped, truncated, noisy, and junk-tailed streams; the full ordered-pair composition matrix with re-serialization to each segment (1084-1174); the `Vec<Version>` element-N genre test; `coincident_span_keeps_borsh_container_framing` (1263-1299), whose doc names the exact failure every lone-value test would miss; and deterministic tripwires for the arms uniform sampling never reaches (negative-height join, collapsible join, tampered coincident padding). The strongest wire-door suite in the crate.
- One grammar body per tree: `validate_prefix` is `validate_from` over a slice cursor and `parse_id_core` is shared by the slice and reader doors, so the two doors can diverge only in cursor mechanics, which is exactly what the differential pins. The anonymous id is structurally unspellable on the wire (a zero presence bit, no bits of its own), so `i != 0` needs no runtime check at either door.
- fold.rs's module doc (1-16) states the goal beside the mechanism: why the balanced counter over a left fold, with the concrete lattice shapes (interleaved single-tick joins; one deep version among dominating meet operands) that make the left fold quadratic, and the combiner precondition (associative and commutative) that makes regrouping value-identical. Both `expect` messages are one-line proofs from the loop condition.
- error.rs draws the `Truncated`/`TrailingBits` boundary exactly at the flush-byte edge (69-85), the one place a reader would otherwise guess wrong, and the borsh suite holds both doors to that genre split (`flush_cut_*`). Every error type has a doctest that constructs it.
- shape.rs leads with which-of-these-do-I-want, explains why heights travel as rises (unbounded counts keep the walk linear in encoded size), reconstructs absolute heights in its example, handles and tests the `N = 0` refinement, and documents the iterators as fused but not exact-size. `advance_refinement` states the overlay-advance law once over `Refine` for any arity and both walk kinds.
- build.rs is a genuinely pure formatter: it recomputes nothing, checks each dataset's run parameters against the index (58-63), pairs each derived committed artifact with a freshness assert whose failure message names the regenerating recipe, and gates the one source-tree write behind an explicit env opt-in with the rationale at the check.
- Cargo.toml's feature comments (56-95) say what each counter sees that no other meter can; the `unexpected_cfgs = deny` roster closes the drift path for the alloc A/B cfg while its comment names the one hole it cannot see; `required-features` on `amp_board` makes an unfeatured board a cargo error rather than a green-looking judgment.
- recurse.rs's `descend!` design note (22-28): guarding the descent rather than the body, with the frame-cost argument for why, is a clear maintainer-facing explanation of a non-obvious choice.

## Open questions for Finch

1. Segments column (crate-root-32): dissolve, or keep and make live? Library kernels cannot reach `grow` in any build, so the column can only ever read zero for them; the depth-100k clock test and the `cfg(test)` gate are the proofs that hold. My recommendation is (a), dissolution, with `deep_tree_stack_safety` named as the instrument against reintroduced recursion; (b) costs an optional `stacker` dependency under `meter` and a liveness witness per enforcing binary.
2. The 100× figure (crate-root-24): is it backed by an off-tree measurement (for instance the oracle tree's boxed footprint against the packed bytes)? If so, commit that measurement beside results/space_consumption and cite it; if not, drop the number. Recommendation: extend `examples/space_consumption.rs` to emit the oracle size and state the observed ratio with its scenario.
3. The space figures (crate-root-29): were the 100-party / 10^6-event numbers produced by a run of `examples/space_consumption.rs` at non-paper parameters? If so, committing that CSV makes the paragraph checkable without rewording; otherwise re-denominate to the committed run. Recommendation: re-denominate; the paper-parameter artifact is what the figure below the paragraph draws.
4. The headline (crate-root-25): is the intended claim "linear for the core operations"? Recommendation: say exactly that and leave the exact bound to each `# Complexity` section.
5. serde (crate-root-34): a new optional `serde_bytes` dependency under the `serde` feature, or a local two-way visitor to keep the dependency surface fixed? Recommendation: `serde_bytes` (mature, tiny, and exactly this problem); either way, `serde_test` as a dev-dependency gives the strict-format witness without inventing a Deserializer.
6. auto_traits totality (crate-root-7): should surfacecheck own the totality check (it already walks the rustdoc JSON) with auto_traits.rs kept as the compile-time early warning, or should it assert the synthetic auto-trait impls directly and retire the roster? Recommendation: the first; the compile-time pin is the cheaper signal and the census makes its totality mechanical.
7. Plateau vocabulary (crate-root-38): is "plateau" intended to mean one leaf of the canonical coding (what the code yields), or a maximal constant run (what a renderer might expect)? The code cannot yield the latter without merging across subtree boundaries. Recommendation: define it as the leaf and say so once; a merging adapter is a consumer's one-liner.
8. Packaging (raised by the refutation pass, not a finding): crates/before/Cargo.toml has no `include`/`exclude`, so `cargo package` ships results/ (about 2.0M including the PNG and CSV nothing in the package reads), reference/ (the paper transcription, about 492K), fuelscape/ (about 1.5M), and docs/ (about 340K). build.rs needs fuelscape/, docs/, and results/space_consumption/itc_space_consumption.svg inside the package, so any include list must keep those; the license status of shipping the transcription under MPL-2.0 is the owner's call. Recommendation: an explicit `include` list before the first release.
9. Meter header (crate-root-40): is any operation today still over its documented bound? The board's green exponent gate says none; a ruling settles which prose is stale and whether the meter suite's header should be rewritten as regression pins under guaranteed bounds.

## Dropped

- [24], [45], [57] Segments counter (prose, correctness, claims lenses): duplicates of crate-root-32; their extra evidence (all 46 envelope rows at 0; tests/meter.rs:441; judge.rs's floor-trip scaffold) is folded in.
- [25], [49], [66] build.rs rerun triggers: duplicates of crate-root-5.
- [47] serde bytes/seq (correctness lens): duplicate of crate-root-34; its claim that serde_bytes is already a transitive dependency was refuted (Cargo.lock has no serde_bytes).
- [26], [46], [64] auto_traits totality: duplicates of crate-root-7; the `causally::{Down, Up, Neutral}` half is dropped per the history pass's dispute (`Polarity: Send + Sync + 'static` and the pinned `Query<'static, Down/Up>` already hold them).
- [30], [68] Cargo description: duplicates of crate-root-2.
- [32] AGENTS.md ghosts: duplicate of crate-root-1.
- [53] Decode::Io source: duplicate of crate-root-15.
- [31], [52] borsh tests cite rumors: duplicates of crate-root-10.
- [43] borsh test hand counts and formatting: merged into crate-root-12.
- [36] borsh test likelihood argument: merged into crate-root-12.
- [19], [50] PartialEq/PartialOrd: duplicates of crate-root-28; [41] ('static and surface) merged into it.
- [34], [55], [70] iter.rs backtick: duplicates of crate-root-23.
- [33], [56] "mint": duplicates of crate-root-26; [18]'s "new impls" half merged into crate-root-36.
- [59] headline "asymptotically linear": duplicate of crate-root-25.
- [61] 100× figure: duplicate of crate-root-24.
- [51] "exactly one hole": duplicate of crate-root-27.
- [38] borsh_impls.rs prose nits: the stray parenthesis and "in-memory wire form" merged into crate-root-9; the thrice-repeated one-wire-form sentence dropped per the history pass's dispute (borsh_impls is a private module whose doc never renders, while each impl doc renders on the public type's page, so the per-impl sentences are the only rendered statement of the contract).
- [44] build.rs version literal: merged into crate-root-4.
- [48] balanced-fold hand-back as a correctness bug: reframed to crate-root-18 (documentation) per the refutation pass; the mechanism is a decided, law-pinned design (laws.rs:2406-2410), so only the public `# Errors` prose is at issue.
- [65]'s status: kept as crate-root-6 despite the history pass's already-known verdict, because the recorded item is an open TODO for the x-axis caption, not a rationale for the current state, and the summary and noscript strings are two more sites of the same gap.
- [12]-adjacent: no separate finding on `merged.take()` idiom; it is the same simplification as crate-root-19.
- Version::shape's likelihood argument (version.rs:745-748, "realistically reachable Versions"), raised as an open question by the prose lens: out of this partition; belongs to the version-core review.
- The `testing/shape_rows.rs` oracle enumerations recursing without `descend!`, raised by the correctness lens: out of this partition; belongs to the testing-oracles review.

<!-- source: final/envelopes-a.md -->
# Partition envelopes-a: The resource-envelope pins, first half (tests/meter.rs lines 1-5305)

## Partition summary

`crates/before/tests/meter.rs` is `before`'s resource-envelope suite: a
single integration-test binary that runs each public operation (and each
skyline kernel behind it) on the adversarial input families from
`before::meter`, reads a set of deterministic process-global counters over
the metered body, and asserts every reading against committed constants.
The counters are peak heap (an external counting allocator, `peak_alloc`),
grown stack segments (`meter::stack_segments`), big-integer limb operations
(`limb-meter`), accumulator digit touches (`suanpan::touch_meter`), scanned
stream bits (`scan-meter`), and, in one band, densified digits. Lines 1-5305
hold the file doc, the scenario-size constants, four envelope tables
(`envelope`, `rank_env`, `sweep_env`, `emit_env`, `text_env`) with their four
harness functions (`metered`, `touch_metered`, `sweep_metered`, and
`query_metered`, the last defined past the range), the heap-meter canaries,
the tick flatness pins, and three band modules (`skyline_flatness`,
`eq_early_exit`, `ledger_wide_arming`) that judge per-unit cost across a
size doubling with derived liveness floors and absolute two-scale ceilings.
Every line in the range is test code; I read all 5305 lines plus the
`QueryEnvelope`/`query_metered` definitions past the range for context, and
the library sources each finding cites.

The measurement core is sound and, in several places, exemplary. Every
flatness band carries a value leg computed outside the metered body (a
closed-form tick total, rank modularity, byte-identity against the public
operator) before any counter is judged; the derived floors state their
derivation at the constant (`SEAM_PLUNGE_TOUCH_FLOOR`,
`LADDER_MARGINAL_TOUCH_FLOOR`, the weight-comb and freeze-parade floors);
each band names a committed known-bad kernel that fails through the same
meters, and every cited kernel resolves; the flatness judge cross-multiplies
in `u128`; wall time is kept out of the suite by design; and measurements of
record are excised from prose into pin commits.

The dominant issues are two. First, the file carries a layer of prose from
before the 2026-07-25 flag day (`faf3cd0a`, "Version stores the skyline
coding"): the header still says the implementation is "far from" the linear
contract that `lib.rs` now declares a hard guarantee, and a dozen test doc
comments describe recursion frames, quadratic path sums, and a decoder that
transcodes back to a packed form, all refuted by the pinned constants beside
them and by the code they describe. Second, several instruments are weaker
than their prose says: the `segments` column has no writer in this binary
(its only increment is `#[cfg(test)]`), so eighty-four `segments <= 0`
ceilings are tautologies presented as a live meter; the scan column has no
liveness floor in any table while the `DECODE_DENSE` row comment names one;
the public `cmp_dense`/`join_dense` rows assert nothing about their result
and pass a no-op; the nine "one-touch-per-operand-byte" floors have no
derivation and fail on the file's own control families (the refutation
pass's run shows `rank_bigroot` at 7,194 touches over 13,752 stored bytes);
and `Rank::cmp`'s documented O(1) leg is priced only jointly with two
O(width) operations. Around these sit a maintenance cascade (four envelope
structs and harnesses, three copies of the slack constants, five
`assert_flat`s, nine `Run` structs) that the campaign note already dockets
for unification, and a set of prose-hygiene and idiom nits.


## Positives

- Derived liveness floors done the way the doctrine asks, with the premise stated at the constant: `SEAM_PLUNGE_TOUCH_FLOOR` and `SEAM_STOP_TOUCH_FLOOR` from the dying folds' three-digit spans (3145-3154, 3301-3309), `LADDER_MARGINAL_TOUCH_FLOOR` from three register folds per decision leaf (3417-3427), the weight-comb and freeze-parade floors from nonzero-delta counts with the explicit sentence "the mechanism's irreducible work, not the family's typical work" (4529-4533, 4637-4641), and one touch per delta for the comb validate/cmp/join/parse runs (2361-2367, 2438-2441). (Verified by reading.)
- Value legs before cost legs: every flatness run asserts the family's closed-form tick total, rank modularity (`distance == lag + lag`, 4191-4195), pointwise-domination identities (3643-3649), verdicts (2451-2455), or byte-identity against the public operator (2529) before any counter is judged, and `ticks_counters_wide` holds `min_ticks` moving by exactly `n` (809-813). (Verified by reading.)
- Known-bad kernels are cited by name and exist: `absolute_position_accounting_reads_superlinear_on_freeze_position`, `span_promotion_accounting_reads_superlinear_on_rearm_spine`, `suffix_walk_settle_reads_superlinear_on_dense_suffix(_pair)`, and `schoolbook_settle_reads_superlinear_on_wide_arming` all resolve in `query/tests.rs`, and `tests/superlinear_tripwires.rs` binds the genre by name in both directions. (Verified by grep.)
- The wide-count ticks band (817-939) is an exemplary asymptotic pin: a growth bound derived from the mechanism (two count-carrying codes, width-linear arithmetic), the piecewise-linear regime knee analysed and every family placed relative to it, a derived scan floor of `2·(bits(n) − 64)`, and an exact `min_ticks` value leg. (Verified by reading.)
- `eq_early_exit` (4983-5150) turns a prose contract into a tail-independent two-scale number with the deciding interval's magnitude fixed so only the tail the exit must not read grows, and the mutants roster records the mutant it kills. (Verified by reading.)
- Two-sided bands where the standard growth band is one-sided: the clearance band (3283-3298) and the latent-ladder marginal (3467-3477) check both directions; the seam plunge/stop pairs difference out shared work against a wire-near-identical control (3160-3175, 3315-3327). (Verified by reading.)
- `assert_flat` (2393-2408) compares ratios by `u128` cross-multiplication with no floating point and prints milli-per-unit so re-pins are legible. (Verified by reading.)
- Measurements of record are excised from prose and live in pin commits (`git log -S` the constant); `ISOLATION_NOTE` rides every envelope failure; the heap meter has a real canary (305-326); the heap column uses an external counting allocator rather than a hand-rolled one. The instrument-correctness lens reports (unverified by me) that `peak_alloc 0.3.0`'s `reset_peak_usage` stores CURRENT into PEAK and counts `alloc_zeroed` and `realloc`, so the delta arithmetic at 367-370 matches the allocator's semantics.

## Open questions for Finch

1. Segments column (envelopes-a-2): the 2026-07-24 keep's premise changed on 2026-07-31 when `grow`/`descend!` became `#[cfg(test)]`. Dissolve the column from the envelope suite (and re-state the stack-cost story as the depth test), or make a writer reachable under `meter` and add a canary here? Recommendation: dissolve; the iterative-walk rule is proven by `clock::tests::deep_tree_stack_safety`, and the deep scenarios here crash on any recursion regression regardless. The same no-writer property should hold for `examples/amp_board.rs` (an example binary links the non-test library); the board reviewer should confirm.
2. Door versus kernel roster (envelopes-a-8, envelopes-a-9): which side is the roster of record? Recommendation: the door, with the kernel-only shapes ported and the value legs added; keep a kernel row only where the door demonstrably adds cost the row must exclude. The refutation pass observed a further twin pair past this range (`rank_bigroot`/`skyline_rank_bigroot` at heap 57_604, limb 2_191, touches 7_194; `rank_dense`/`skyline_rank_dense` at 24_576/3/5), for the envelopes-b finalizer.
3. Flatness slack (envelopes-a-16): share the board's exponent bar (`10/9` per doubling for 1.15) or keep ×1.25 and say so? Recommendation: keep ×1.25 where a band's declared model needs it (the dense-suffix log model) and state the exponent bound in each band doc; tighten the rest to `10/9` if the pinned readings allow (the run log shows the per-unit ratios within 1% on most bands).
4. Isolation guard (envelopes-a-3): is a `NEXTEST`/`RUST_TEST_THREADS` check wanted, or is `ISOLATION_NOTE` on failure text sufficient given the gate runs nextest? Recommendation: add the guard; it is one function, and the failure it prevents is a silent pass.
5. Public decode rows read roughly twice the kernel copy (`DECODE_DENSE` 120_035 against `SKYLINE_DECODE_DENSE` 61_440; `DECODE_CLIFF` 4_052 against a stream near 1.5 KB) even though `Version::decode` reads into one `Vec` and adopts it. What allocates the second copy's worth of peak at the door (`read_to_end` growth, or the `Bytes` conversion)? A question for the version-core reviewer; if it is growth, the crate doc's "the result reuses the read buffer" describes only half the peak.
6. Should the meter surface expose the generator width constants under the `meter` feature so the closed forms can cite them (envelopes-a-19)? Recommendation: no; keep the oracle's independence and name the widths once in the band module with a comment naming the generator constant each mirrors.
7. If a directory split of `tests/meter.rs` is wanted, `amp_board_smoke::band_tests_and_registry_citations_stay_paired` (reads `tests/meter.rs` by path) and suanpan's `claims.rs` `BANDS` path must move in the same commit. Recommendation: defer until the harness unification lands; the pointer comment at 265-267 dissolves with it.

## Dropped

- [9] dashu-int manifest pin: refuted. The committed `Cargo.lock` is the correct layer for a library's measurement determinism; an exact `=0.5.0` in the workspace manifest would propagate to every downstream consumer. The stale "thin margin" prose leg survives inside envelopes-a-20.
- [10] segments doc rewrite as a tamper pin: the proposed present role ("a nonzero reading means a depth-recursive walk entered the library") is itself false in this binary, since `descend!` does not compile outside `cfg(test)`; merged into envelopes-a-2.
- [23], [32], [48]: duplicates of envelopes-a-1 (header and row docs).
- [21], [31]: duplicates of envelopes-a-1 (transcode prose).
- [29]: `skyline_oracle` ghost name; merged into envelopes-a-1, with the value-leg consequence in envelopes-a-9.
- [22]: duplicate of envelopes-a-11.
- [25], [50]: duplicates of envelopes-a-17.
- [34], [54]: duplicates of envelopes-a-4.
- [35]: duplicate of envelopes-a-15.
- [27], [33], [49]: duplicates of envelopes-a-12.
- [28]: duplicate of envelopes-a-10.
- [17], [46]: duplicates of envelopes-a-2.
- [44]: eq early-exit leg reframed and merged into envelopes-a-22; the accumulator-band leg is envelopes-a-22.
- [42]: merged into envelopes-a-13; [16] (`version_of` alias) merged into envelopes-a-13.
- [37], [52]: merged into envelopes-a-19; [36] duplicate of envelopes-a-19.
- [43]: duplicate of envelopes-a-7; [30], [41] merged into envelopes-a-7.
- [38], [39]: merged into envelopes-a-20; [53] converted (deliberate calibration kept at 500d4d09; the rationale lives only in history) and merged into envelopes-a-20.
- [45]: the tick-row placement is folded into envelopes-a-4 (the docketed unification dissolves the pointer comment); the scanner constraint is open question 7.
- New from the refutation pass, out of partition: the board's segments column (open question 1) and the `rank_bigroot` twin pair (open question 2).

<!-- source: final/envelopes-b.md -->
# Partition envelopes-b: The resource-envelope pins, second half (tests/meter.rs lines 5306-10808)

## Partition summary

This range is the second half of before's resource-envelope test binary. It holds sixteen feature-gated band modules (`hoisted_window`, `parse_wide_arming`, `answer_embedded_product`, `settle_flatness`, `id_walk_scan_cost`, `accum_streams`, `fold_stagger`, `fold_alias`, `meet_fold`, `memo_resolution_cost`, `width_circulation_cost`, `dominated_undercut_cost`, `pool_recycle`, `placement`, `span`, `span_codec`, `identity_fast_paths`), the `fork_env` and `query_env` row tables with their `sweep_metered`/`query_metered` harness, and the per-row envelope tests for the skyline query kernels, the version-pair queries, the masked comparisons, the cheap-clone cell, and the join folds. Every file in the partition is test code: I read lines 5306-10808 in full with line numbers (5,503 lines) plus the file header (1-120), the `Envelope` harness (200-430), and the `TouchEnvelope` and `SweepEnvelope` harnesses (1110-1280, 1590-1735) for context.

The measurement design is strong and consistent. Each band's `run` resets every counter, takes the heap baseline after the resets, runs only the operation under test, and reads the counters before any formatting allocation; operands, closed forms, and reference folds are built outside the window. Nearly every cost pin rides beside a semantic leg on the same run (a `min_ticks` closed form, the exact rank product `2·x·y + 1`, text-literal expected trees, oracle equality). The derived liveness floors carry their derivations inline and the arithmetic checks (2·999 + 32 = 2,030; 4·1,999 + 64 = 8,060; 1,999 + 64 = 2,063; 1,024·(48 + 2) = 51,200; 2·40 = 80; the id-walk exact pins 500,004 and 1,000,004 equal 2·(2d + 2) at both depths). The `placement`, `span`, `span_codec`, and `identity_fast_paths` sections state their costs as relational identities against compositions on the same operands, each with a nonzero liveness read and a walking control, so nothing there can rot. The `meet_fold` band commits and rosters its known-bad kernel; `pool_recycle` and `dominated_undercut_cost` price properties no other meter can see. Every identifier the range's prose cites resolves in the tree.

The dominant issues are structural and historical rather than measurement defects. Structurally, the file carries four column-subset copies of one envelope struct and harness, and the ×1.25 two-scale ratio, the counter readers, the clock-history fixture, the `tick_run` helper, and the `UBig`-to-`Ticks` conversion are each hand-copied across sibling modules. Historically, the red-first pins of late July were flipped to green guards without renaming: three tests still carry names asserting the refuted reading, eighteen docs open with a `GREEN PIN:` label whose `RED PIN` counterpart no longer exists, and thirteen pins in the range are named outside the convention the band roster scans, so they bind to no roster while the registry describes three of them inaccurately. Two instrument gaps survive review: the stagger and scatter fold bands' known-bad mechanism is never metered (unlike `meet_fold`'s), and `memo_resolution_cost` pins class signatures with no absolute ceiling on six of seven tests. One test's doc describes a limb leg its body no longer has and narrates the removal, which breaches the root AGENTS.md no-ghost-references hard rule; it is the range's one high-severity finding and is a documentation fix.

I ran nothing. Every arithmetic claim below is hand-checked; measured readings quoted below come from the refutation pass's run log (`scratchpad/before/refute-envelopes-b/run1.log`, 24 tests passing), which I read as an artifact. Provenance is stated per finding.


## Positives

- The relational identity pins in `placement` (9471-10071), `span` (10084-10328), `span_codec` (10343-10539), and `identity_fast_paths` (10551-10808) are the strongest instruments in the range: each states the fused walk's cost as an exact identity against the composition on the same operands (`fused + cmp_ss / 2 == cmp_sv + cmp_se`; `fused + decode_a + decode_b == met + joined + cmp_ab`; `fused == decode_lo + cmp`), prices a probe decode as half a buffer-distinct self-comparison, and pairs every zero-cost claim with a `> 0` liveness read and a walking control, so there is no constant to re-pin and a dead meter cannot green a section.
- The derived liveness floors carry their derivations inline and the arithmetic checks at every site: `PURE_COMB_TOUCH_FLOOR` 2·999 + 32 = 2,030 (8831), `ASCEND_CLIFF_TOUCH_FLOOR` 4·1,999 + 64 = 8,060 (9024), `PLATEAU_TOUCH_FLOOR` 1,999 + 64 = 2,063 (9104), `DOMINATED_UNDERCUT_TOUCH_FLOOR` 1,024·(48 + 2) = 51,200 (9294), `HOISTED_WINDOW_DENSIFY_FLOOR` 2·40 = 80 (5473), `SEAM_STOP_POOL_WARMUP` = 2 from peak simultaneous demand (9411). Each names its premise and says what a trip means, and the two lower-bound genres are named apart with every assertion message saying which one tripped.
- `id_walk_scan_cost` pins the covers and disjoint walks with two-sided exact equality (500,004 and 1,000,004 = 2·(2d + 2) at both depths) and explains at 6297-6300 why a ceiling plus a slack floor cannot see a uniform tap undercount.
- `dominated_undercut_cost` (9179-9365) reads the decision counter (`emit_traffic().dominated_undercut >= k`) beside the touch band, with the module comment stating exactly why no differential and no cost band alone can prove the arm still fires.
- `pool_recycle` (9367-9461) names the failure class no other meter can see (a dead recycle leaves heap, limb, and touch byte-identical), derives its ceiling of 2 from peak simultaneous demand, asserts equality across the churn doubling, and `.cargo/mutants.toml:65-69` relies on it to kill the retire/lease deletion mutants; the two files agree.
- `masked_cmp_hole_depth_band` asserts `lo == hi`, an exact depth-independence claim no per-boundary mechanism can satisfy, and `join_all_equal_operands_is_clone_cheap` asserts byte-identical peak heap across a 4× operand growth: the strongest available forms of their O(1) claims.
- `fold_stagger` and `meet_fold` normalize every counter by the declared model's level count before the ×1.25 band, so they enforce the model's constant rather than a flat reading the documented log factor forbids, and hold the arity and size axes independently under absolute ceilings at every point.
- `meet_fold` commits its known-bad kernel beside the band, rosters it in `tests/superlinear_tripwires.rs`, and places its ×1.49 floor midway between the linear and quadratic signatures so only a class change crosses it.
- Delta placement is exact throughout: every harness and band `run` resets all counters, takes the heap baseline after the resets, runs only the operation, and reads every counter before any `format!`/`eprintln!`; operands, closed forms, and reference folds are built outside the window.
- Every cost pin but the three in envelopes-b-15 rides beside a semantic leg on the same run (closed-form `min_ticks`, the exact rank product, text-literal expected trees, oracle equality), so a ceiling cannot pass on a wrong value.
- Every identifier the range's prose cites (`ledger_wide_arming`, `mul_bound_embedding_is_alive`, `arbitrary_factors_embed_their_product_in_exact_rank`, `densify_tap_prices_the_cluster_span`, `span_is_the_pair_hull`, the `before::laws` names, the `*_log_factor_is_alive` pins, the counter accessors) resolves in the tree; I found no ghost identifiers.
- The zero-heap pins on the concurrent rows (`RANK_CONCURRENT`, `DISTANCE_CONCURRENT`, `LAG_CONCURRENT` at 6833, 6877, 6878) price the benign regime's constant so an adversarial-path cure cannot hide a charge to common inputs.

## Open questions for Finch

1. Should `accum_streams` (6448-6721) move into `crates/suanpan/tests/`? The module imports only `dashu_int::UBig` and `suanpan::{touch_meter, Accumulator}`, suanpan has the `touch-meter` feature and the dashu-int dependency, and suanpan's claims roster binds the witnesses by a cross-crate relative path (`claims.rs:99 const BANDS: &str = "../before/tests/meter.rs"`), so suanpan cannot verify its own cost table without before's tree. The placement was chosen explicitly (9e7b7ce33, "beside the consumer"), so I converted it from a finding to a question. Recommendation: move it and point `BANDS` at the in-crate path; the only tie to before is the shared ×1.25 comment convention.
2. Is a rostered live kernel owed for the compaction-off separator? The known-bad demonstration for `SKYLINE_MIN_TICKS_ASCEND` exists as a cargo-mutants disposition (.cargo/mutants.toml:70-76, campaign 9aec9aa2) and the gate runs only `mutants-list`, never the campaign (justfile:314-326). Recommendation: cite the disposition in the present tense now (envelopes-b-12); add a `_reads_superlinear` kernel through a documented internal entry in `query/web.rs` only if you want gate-time enforcement, since the seam is the cost.
3. `From<suanpan::UBig> for Ticks`: the sixteen `.to_string().parse::<before::Ticks>()` sites exist because `Ticks` converts from the primitives and `FromStr` only (src/version/ticks.rs:152-213), and before does not re-export `UBig`, so the impl would put suanpan's type in before's public API. Recommendation: the test-local `fn ticks(&UBig) -> Ticks` from envelopes-b-19 now; the API impl only if you want it.
4. The ×1.25 band carries two meanings in the range: `settle_flatness` (5830-5836) and `fold_alias`'s depth axis (8129-8134) let the model's log factor consume part of the slack, while `fold_stagger` and `meet_fold` divide the log factor out and hold a true flatness. Recommendation: normalize every model-bearing band by its model so ×1.25 means one thing (envelopes-b-4's first resolution).
5. Harness unification shape (envelopes-b-8): `Option`-typed columns (rows keep their pin sets, no re-measure) or every column pinned on every row (wider coverage, a one-time re-measure of the three-column tables)? Recommendation: `Option` columns first; widen row by row afterwards under the tightening rule.
6. Process-isolation guard (envelopes-b-10): an environment check forbids a single-threaded `cargo test` that is in fact isolation-safe; the status quo leaves a false pass with no note. Recommendation: the environment check, since `test-all` is nextest anyway and the rumors conformance backend already made the analogous choice with a static lock.
7. Cross-partition (meter-core): `arming_train`'s doc states `band = 32w + ⌈log₂ n⌉ + 2` (src/meter.rs:2471) while the code uses `bitlen(n)` (2490), which for the powers of two the probes use is `log₂ n + 1`; the test mirror at 5940 follows the code. The generator doc is off by one, not the test.
8. Cross-partition (skyline-watermark): `MinWeb::compacting`'s rustdoc (src/version/skyline/watermark.rs:253-254) quotes "×1.41 that row's pinned peak heap and ×2.0 its pinned touches" from the uncommitted hand swap, the measured-snapshot genre 500d4d094 swept from meter surfaces.
9. `SKYLINE_MIN_TICKS_ASCEND` pins 553,660 B peak heap (6839) on an operand of roughly 1.3 KB, an auxiliary-space amplification of hundreds of times; the file header (7-11) owns "today's implementation is far from that", while the crate docs promise auxiliary space at most a small constant multiple of the input. Which document is authoritative, and should the row's comment state the amplification it pins so the contradiction is visible at the row? (I did not verify the operand size; the adequacy lens reports it.)
10. The registry's `closed_form: Option<&'static str>` holds each family's closed form as prose while the executable forms live in this file's `run` bodies (for example `d + 2^(32w) + 2^288 + 3` at 5359-5362), and nothing compares them. Recommendation: leave as is unless you want the slot to become an executable hook the bands call.

## Dropped

- Fork scan column pins the meter's blindness (scaffolding [9]): deliberate and documented inline at 6400-6405 (fc862595d9's "the number moving is the point" ratchet); the useful residue, that the row names no instrument for the walk, lives in envelopes-b-6.
- suanpan's accumulator witnesses live in before's envelope binary ([10]): recorded intent (9e7b7ce33; claims.rs:96-99); converted to open question 1.
- Fan == comb duplicates ([19], [32], [48]): merged into envelopes-b-7.
- Red-mechanism names duplicates ([24], [38], [52]): merged into envelopes-b-25; [52]'s sub-claim about `party_fold_alias_rejection_depth_is_flat_per_unit` (8136) is deliberate-and-holds (the band convention name is what binds it to the registry's `AXIS_BANDS` at registry.rs:1985, and its doc at 8130-8134 states the model bound explicitly).
- Line 8580 duplicates ([27], [44], [53]): merged into envelopes-b-23.
- Masked-hole re-pin duplicates ([36], [58]): merged into envelopes-b-16.
- id-scan tautology duplicates ([40], [60]): merged into envelopes-b-5.
- Three rows without a value leg duplicate ([54]): merged into envelopes-b-15.
- ×1.17 arithmetic duplicate ([50]): merged into envelopes-b-4, which also carries its `assert_flat_step` doc half.
- span limb-leg doc duplicate ([33]): merged into envelopes-b-27.
- Double fixture / operand duplicates ([28], [59]) and the small-redundancies list ([46]): merged into envelopes-b-14; the tuple-return closures are addressed under envelopes-b-15.
- Literal floors duplicate ([51]): merged into envelopes-b-26.
- Harness quadruplication duplicate ([29]): merged into envelopes-b-8.
- Helper duplication ([30], [31]), the UBig-to-Ticks round trip ([14]), and the per-row bodies ([37]): merged into envelopes-b-19.
- Join-fold known-bad duplicate ([18]): merged with [11] into envelopes-b-18.
- Compaction-off demonstration ([34]) and the bracketed aside ([26]): merged into envelopes-b-12 as documentation, with the rostered-kernel question moved to open question 2.
- Raw ×2.5 growth ceilings ([5]): merged into envelopes-b-21 as its second point.
- `GREEN PIN` ([13]): merged into envelopes-b-3.
- `JOIN_EQUAL_OPERANDS_PEAK` split comment ([15]) and the fork docs contradiction ([39]): merged into envelopes-b-12 and envelopes-b-6 respectively.
- Fork row's unnamed pricing instrument ([23]): merged into envelopes-b-6.
- The adequacy lens's PP(500, 500) irreducible-work estimate: kept only as a lower-confidence related note inside envelopes-b-20, not as its own claim (unverified).
- watermark.rs:253-254 snapshot prose and the `arming_train` doc off-by-one: out of partition; routed to open questions 7 and 8.

<!-- source: final/fuelscape-pipeline.md -->
# Partition fuelscape-pipeline: The fuelscape population atlas: plan, count, enumerate, select, sample, families, operation table

## Partition summary

`before-fuelscape` is a detached dev-tool workspace that renders, for each public operation of `before`, a log-log heatmap of deterministic wasm fuel against exact packed input size, sampled uniformly from every size's whole canonical input space. This partition is the measurement core. `count.rs` builds exact big-integer counting tables over the two codec grammars (the version skyline coding under the sibling rule, the party id coding); `sample.rs` turns those tables into exact-uniform samplers by counting-guided generation, restoring the version grammar's nonnegative-height rule by whole-sample rejection; `enumerate.rs` lists every member of a small exact-size space straight from the grammar so the tests have ground truth the samplers cannot supply about themselves; `plan.rs` draws each cell's inputs from a coordinate-seeded RNG (arity, then composition split, then members), runs one measured kernel per sample in a fresh guest, and appends the overlay points; `families.rs` ramps committed meter shapes into overlay inputs per operand signature; `ops.rs` is the roster of 104 `OpSpec` rows plus the 72-row `EXEMPTIONS` table that, with the panels' `covers` lists, tiles `before::surface`; `select.rs` implements the runner's name filters. The tests (`plan/tests.rs`, `count/tests.rs`, `select/tests.rs`, `sample/tests.rs`, `ops/tests.rs`) are the crate's committed checks: sampler adequacy, determinism, tiling parity, and the runner's empty-run guard. I read all fourteen partition files whole with line numbers (5,631 lines) and, for the anchors the findings rest on, the smoke test, the fuzzfit guest's kernels, `before`'s version/ranked/conjunction/party compare code, the board's `WORST_RANKINGS` and `BOARD_NOT_APPLICABLE`, `surface.rs`, `compact.rs`, `render.rs`, the spanbands binary, the entropy agent note, rayon 1.12.0's range sources, and the git history the history pass cited.

The instrument's core is in good shape and I could check it: the choice weights in both samplers partition exactly the pairs the recurrences count, the sibling-rule exclusion is transcribed the same way in the table (`leaf(j-3)`), the enumeration, and the sampler, and the adequacy pins form a genuine triangle of independent oracles (table versus enumeration to 24 bits, grammar versus the shipping decoders as set equality over every byte string to 2 bytes, decoder census to 23 bits, chi-square uniformity at a fixed seed, proptest round trips to 48 bytes). The run is a pure function of the plan and a replay test pins that, collection order included. `select.rs` closes the silent-empty-run hole per filter. The tiling test binds panels plus exemptions to the surface roster in both directions with a contradiction check, and every exemption reasons on mechanism identity or a missing size axis. Dependency rationales in `Cargo.toml` are exemplary.

The dominant issues are two doc claims the tree does not back and one instrument mislabel. `COMBINE_ARITY_CAP` says the host and guest caps are "kept equal by the pipeline smoke test's combine case", but the smoke plan runs at 8 bytes and no test compares the two constants. `lib.rs` says the overlay marks "the committed adversarial families" and shows "the adversarial frontier", but the overlay is a hand list of 18 of the registry's 68 shapes keyed by operand signature, and the board's own per-operation worst-case pin names families that never appear in it. On every `[Version, Version]` row the overlay adds two self-pairs, and on the eight rows whose measured kernel opens with `codec::canonical_eq` those points measure a byte compare plus a refcount clone under an adversarial label. Beneath those: duplicated scaffolding (two count tables and two samplers sharing every method but the recurrence; a hand-written parallel/sequential branch with a reference twin and a 2048-entry pin that rayon's `with_min_len` replaces; the member-draw closure copied seven times in `draw_inputs`; the chi-square statistic written three times), a second exemption roster restating the board's for 51 shared surface rows, a few pins of the weaker genre (reachability where uniformity is claimed; a sign-only fold-arity criterion), dead accessors, and a vocabulary sweep (`mint` at eight sites, `honest`, `real`, em-dashes in line comments).


## Positives

- The sampler adequacy pins form a genuine triangle of independent oracles: table versus brute-force grammar enumeration at every bit length to 24 (count/tests.rs:23-33, 63-73); grammar versus the shipping decoders as set equality, not counts, over every byte string to 2 bytes (sample/tests.rs:69-94), so a validator gap or an enumeration slip fails by member; the decoder census to 23 bits bucketed by live bit length, which also holds the one-byte padding rule without a separate test (count/tests.rs:172-205, with the doc at 188-191 saying exactly how a padding drift would read); fixed-seed chi-square uniformity with alien draws failing by name (sample/tests.rs:117-197); and proptest round trips through the real codecs to 48 bytes with committed seeds at the path proptest resolves.
- The samplers are exact-uniform by construction and it is checkable from the code: the `bare`/`internal` weights in `VersionSampler::subtree` (sample.rs:289-292) and the `term_left`/`deep_left` weights in `PartySampler::subtree` (473-489) partition exactly the pairs the recurrences at count.rs:170-172 and 250-254 count, and every zero-total branch is provably never entered because its selecting weight is zero. The whole-sample rejection argument for the nonnegativity rule is stated as mechanism (sample.rs:10-23), with the Theta(1/sqrt(leaves)) acceptance rate derived and the rejection count surfaced per draw so a run can quote it.
- `parallel_build_matches_sequential_reference` guards its own liveness with `const { assert!(PIN_BITS - 5 >= PAR_SPLIT_THRESHOLD) }` (count/tests.rs:89-94), so moving the threshold past the pin's span is a compile error rather than a vacuous green. `PAR_SPLIT_THRESHOLD` (count.rs:73-84) carries bracketing measurements on both sides of the cut and says the exact value is low-stakes: how a tuning constant should be documented.
- `arity_draw_is_uniform_over_every_count` (plan/tests.rs:167-202) states in its doc why reachability cannot pin uniformity and names the biased-but-total mutant it kills; the adequacy argument is written where the threshold lives.
- The determinism story is complete and tested: `cell_rng` (sample.rs:106-131) mixes (seed, op, size, index) through a fixed, documented splitmix64 expansion instead of std's unstable `DefaultHasher`; every sample gets a fresh guest; `run_op_is_deterministic_and_ordered` replays whole atlases including collection order; `fold_rows_expose_the_arity_axis_in_fuel` demonstrates that the stratified arity draw buys something observable rather than asserting it.
- `split_budget` (plan.rs:189-206) is an exact uniform composition draw (sequential distinct-cut insertion is a uniform k-subset) and the stratification argument is stated where the choice is made (plan.rs:170-175).
- `select.rs` closes the silent-empty-run hole per filter, not on the union, and pins both invariants with synthetic rosters for the semantics and the real roster for the listing.
- `panels_and_exemptions_tile_the_coverage_roster` (ops/tests.rs:18-54) holds the totality binding in both directions and rejects a row that is both covered and exempted, each failure naming the offending row; `EXEMPTIONS` reasons on mechanism identity or a missing size axis and never on a missing kernel.
- The `version_eq` row (ops.rs:530-547) and `party_without` (843-871) show exactly the short-circuit reasoning the self-pair overlays lack: the equal pair is chosen because there memcmp is the worst case, and operand order is chosen so the difference exists.
- Every dependency in `Cargo.toml` states what it is for and what the cheaper alternative would have cost; `Inputs` variants and `OpSpec` fields are documented at the maintainer's altitude.

## Open questions for Finch

1. Overlay contract (fuelscape-pipeline-1). Should the overlay track the board's per-operation worst-case pin (every board-pinned family for an operation appears as a marked point, which needs `WORST_RANKINGS` or a `FamilyId`-per-op roster exposed from `before::meter` and an atlas-op to board-op map) or stay a curated subset? Recommendation: reword lib.rs and the validation index now (cheap and true), add the committed gap-listing test, and decide the binding as a design item; if binding, the shared roster should live in `before::meter` so the frontier the islands draw is the frontier the board enforces.
2. CENSUS literal (fuelscape-pipeline-15). Keep as an explicitly labeled tamper-evidence snapshot, or delete now that the decoder census covers a superset of its range? Recommendation: keep and reword; it is the one pin that survives a coordinated enumeration-plus-decoder change, which for canonical form is exactly the owner-ruled format-change event.
3. Shared size-axis classification (fuelscape-pipeline-30). Would you accept `SizeAxis` on `before::surface::SurfaceRow`, shared by the board and atlas tilings, given that it reshapes the roster of record? Recommendation: yes; the 51 shared rows are a property of the row, and the two instruments keep their own tables for their own decisions.
4. spanbands (fuelscape-pipeline-32). Is the classify-first-versus-emit-always investigation concluded? Recommendation: record the verdict in a note and delete the binary; if the question is live, fold it into the runner behind a flag that draws through `plan`.
5. `enumerate` visibility (fuelscape-pipeline-16). Is it meant to stay a public module for ad-hoc exploration from the binaries? Recommendation: `#[cfg(test)]`; today only the two test files use it.
6. Cross-word-width replay. `gen_biguint_below` is width-portable (num-bigint draws u32 digits), but rand 0.8's `gen_range` over `usize` in `draw_arity` and `split_budget` samples differently on a 32-bit host, so the same seed would draw a different stream there. All committed hosts are 64-bit. Recommendation: stamp the determinism contract as "on 64-bit hosts" in plan.rs's module doc, or draw those ranges over `u64` and cast; not filed as a finding because no 32-bit host runs the atlas.

## Dropped

- Lens adequacy [20], covers-to-kernel binding: below the bar. The only mechanical check available (a declared `kernel` string per row compared with the guest's last call) is authored by the same hand a few lines from the `g.call(...)` it must equal, so it adds little over the row layout that already places `covers` and the measured call side by side; the harness change is also outside this partition.
- Lens scaffolding [7] and structure-prose [36], the Cargo.toml gzip figure: not an instance. `gzip -l` over the committed dump gives 257,868,233 bytes uncompressed and 23,846,150 compressed, 245.9 MiB against the comment's "~242 MB"; approximate prose that is approximately right.
- Lens structure-prose [36], the sub-claim that the 767 comment is "correct today": refuted by computation (433 over bits 8..=15); folded into fuelscape-pipeline-21 with the correct number.
- Lens scaffolding [3], the "dated rationale" charge against `PAR_SPLIT_THRESHOLD`'s doc: refuted. The measurement table is machine-specific but undated, and an inline derivation is what doctrine asks of a tuning constant; the simplification survives as fuelscape-pipeline-11 with the rayon `Range<usize>` correction.
- Lens adequacy [14] and scaffolding [0]: duplicates of fuelscape-pipeline-1.
- Lens adequacy [15], structure-prose [24], instrument-correctness [43]: duplicates of fuelscape-pipeline-28.
- Lens adequacy [19], structure-prose [30], instrument-correctness [45]: duplicates of fuelscape-pipeline-15.
- Lens instrument-correctness [44]: duplicate of fuelscape-pipeline-21.
- Lens structure-prose [26]: duplicate of fuelscape-pipeline-4.
- Lens adequacy [21], structure-prose [32], [38]: duplicates of fuelscape-pipeline-2.
- Lens adequacy [22], structure-prose [33], instrument-correctness [53]: duplicates of fuelscape-pipeline-3.
- Lens instrument-correctness [46]: duplicate of fuelscape-pipeline-9.
- Lens structure-prose [28], instrument-correctness [48]: duplicates of fuelscape-pipeline-5.
- Lens instrument-correctness [47]: duplicate of fuelscape-pipeline-8.
- Lens instrument-correctness [55]: duplicate of fuelscape-pipeline-18.
- Lens instrument-correctness [50]: merged into fuelscape-pipeline-6.
- Lens structure-prose [29], the two-sweeps remark: merged into fuelscape-pipeline-20.
- Lens structure-prose [34], fully-qualified paths (`rand_chacha::ChaCha12Rng` at plan.rs:176, 189, 217, 254; `std::collections::BTreeSet` at plan.rs:194 and plan/tests.rs:156, 162, 237, 266; `super::OpAtlas` at plan/tests.rs:45, 58; `std::ops::RangeInclusive` at count.rs:286 beside the import at 41): verified and real, but pure taste with no cost beyond the read; folded the count.rs:286 case into fuelscape-pipeline-14's resolution and left the rest to rustfmt-adjacent cleanup when the files are next touched.

<!-- source: final/fuelscape-render.md -->
# Partition fuelscape-render: The fuelscape rendering and dataset pipeline: render, compact, dump, the two binaries, the rustdoc widget script and stylesheet, and build.rs

## Partition summary

This partition is the audit-view half of the population atlas. `render.rs` turns an operation's measured samples into a `HeatGrid` (log-log bin geometry over fuel and input size, one histogram per size column) and draws one SVG per operation plus a gallery page with plotters. `dump.rs` persists each operation's raw samples, overlay points, provenance, and its grid as one JSON document per operation with an index, and reads such a dump back strictly, recomputing the grid and refusing disagreement. `compact.rs` derives the committed widget datasets (`crates/before/fuelscape/*.json`: per-column log-fuel histograms at `RES` octaves per bin, stamped with the roster row's contract and claim strings) from a dump, and reads them back with the same strictness. `bin/fuelscape.rs` is the clap-driven runner with three modes (measure, replay a dump, compact a dump); `bin/spanbands.rs` is a one-off native-counter CSV emitter. `build.rs` in `before` re-reads the committed compact documents, formats them into single-line `<details>` islands for the `# Complexity` sections, pins the derived rustdoc header to the stylesheet-plus-script concatenation, and derives the theme-reactive space-consumption figure. `fuelscape.js` and `fuelscape.css` are the dependency-free browser widget: expression grammar, acceptance rule, density rendering, quantile probe, guides, keyboard access, and a client-side typesetting pass over complexity code spans.

The load-bearing instruments are sound and, in places, exemplary. The dump reader recomputes every stored grid from the raw samples and refuses a mismatch, with a committed tamper demonstration; replay from a dump is pinned byte-identical to a direct render across the whole roster; the compact tamper closure constructs seven known-bad documents and asserts the loader names each check; the header is held to exact equality with its sources (verified fresh at HEAD with `cmp`); the widget refuses a dataset without a positive `res`; and `tools/fuelscape-claims` parses every committed claim with the widget's own exported grammar rather than a reimplementation. Read-only measurement of the committed artifacts confirms the design holds in the data: 104 operation documents in both the dump and the compact dataset, 100 stamped f77011e3 and 4 stamped 46eb64f9, both ancestors of HEAD, no plain `atlas.json` shadowing the committed `.gz`, no zero-fuel sample or zero overlay point anywhere.

The findings cluster around seams the pins do not reach. Six are medium: the measurement commit is stamped from `git rev-parse HEAD` with no dirty-tree guard, falls back to the literal `untracked`, and no reader checks its shape; the dump format was redesigned to accrete but the only writer truncates the index, so the one accretion on record was a hand merge; the client-side typesetting pass runs on every workspace crate's rustdoc pages while the justfile says the script is inert off `.fuelscape` elements; the cross-platform grid pin's docstring names a sensitivity mechanism it does not have (by replicating its fixture, every hashed value except one `log2` is exact on any libm), and no known-bad demonstration exists; a scoped-thread panel pool with two mutexes and an atomic cursor serves a constant pinned to 1 fourteen minutes after it landed at 4; and both `FORMAT_VERSION` docs narrate the layout they replaced, which the root hard rules forbid. The lows are real but bounded: a CLI value reaches an assert documented as programmer error, the positivity of log-scaled quantities is enforced at three different places three different ways, `compact_dump` never runs `validate` on what it writes, `build.rs` omits two of its inputs from `rerun-if-changed` and validates untyped JSON values, a five-column smoothing kernel produces NaN for one- or two-column datasets, and several enumerated strictness rejections have no committed known-bad case. The rest are vocabulary, duplication, and prose nits, batched.

I read all eleven partition files in full with line numbers: 4,585 lines (`render.rs` 641, `render/tests.rs` 195, `compact.rs` 412, `compact/tests.rs` 275, `dump.rs` 306, `dump/tests.rs` 200, `bin/fuelscape.rs` 317, `bin/spanbands.rs` 170, `fuelscape.js` 1580, `fuelscape.css` 155, `build.rs` 334). The three `tests.rs` files are the test surfaces; `render/tests.rs` and `dump/tests.rs` drive the fuzz-fit guest. I also read the justfile's fuelscape and docs recipes, `crates/before-fuelscape/Cargo.toml`, `tools/fuelscape-claims`, the crate's `lib.rs`, the relevant excerpts of `plan.rs`, `ops.rs`, and `sample.rs`, the design note under `.agent-notes/2026-08-13-before-fuelscape-rustdoc/`, the typst consumer in `.agent-notes/2026-07-27-skyline-exposition/07-machine.typ`, the CI workflow's tier comments, plotters-svg 0.3.7 and rand_core 0.6.4 sources from the cargo registry, and the eleven commits the history pass cites (read-only git only). Programs run: `node` on the extracted `smoothSeries`, python over the committed datasets, the dump, and the platform pin's fixture generator, `wc -c` and `cmp` on the header. No cargo, just, build, or test command was run; nothing under the repository was modified.


## Positives

- `dump::read` recomputes every stored `HeatGrid` from the raw samples and refuses disagreement (dump.rs:237-243), and `read_rejects_a_dump_whose_grid_disagrees_with_its_samples` (dump/tests.rs:170-200) is the committed known-bad demonstration that the check fires: a real two-derivation check, not decoration.
- `dump_and_rerender_matches_direct_render_byte_for_byte` (dump/tests.rs:63-130) measures once, renders directly and from the persisted dump, and asserts byte-identical SVGs and gallery across the whole roster: exactly the shape the doctrine asks for when a quantity is computable two ways, and what makes `--render-from` a faithful substitute for re-measuring.
- The compact tamper closure (compact/tests.rs:148-183) constructs each known-bad document by JSON edit and asserts the loader names the check; the idiom is table-shaped, so the missing cases in finding 14 are one line each.
- `tools/fuelscape-claims` loads `docs/fuelscape.js` under node and asks the widget's own exported `Fuelscape.accepts` (lines 23-39), so the claim check and the grammar are one truth rather than a Rust reimplementation.
- The widget refuses a dataset without a positive `res` (fuelscape.js:374-377) and the compactor stamps `RES` into every document (compact.rs:223), closing the wrong-bin-height-forever hole the design note named; `quantAt` and `cdfAt` are exact inverses.
- `check_header_fresh` holds the derived header to exact equality with `<style>{css}</style>\n<script>{js}</script>\n` (build.rs:323-334); I verified with `cmp` that the committed header is fresh at HEAD. `build.rs` also escapes `</` inside the island JSON (223-225) and emits single-line islands so rustdoc's Markdown pass cannot re-enter.
- The libm routing is argued precisely where it matters (render.rs:312-316, compact.rs:200-203), and the accretion design holds in the data: 104 operation documents in both the dump and the compact dataset, 100 stamped f77011e3 and 4 stamped 46eb64f9, both ancestors of HEAD, the dump index naming exactly the files present.
- The runner's clap `Args` (bin/fuelscape.rs:82-119) makes the three modes' flag exclusions declarative with per-flag docs, and the module doc states the purity contract (same plan, same guest, byte-identical measurements in any order).
- `dump::parse` (dump.rs:255-279) reports the plain path's absence with "(nor a .gz sibling)", so strictness errors name the file the caller asked for while the gzip storage stays an implementation detail.
- fuelscape.js's comments consistently state what the code cannot show: the rAF negative-t clamp (53-57), the exact plot-box clip for guides (787-793), hover on the svg rather than the tiles (1004-1007), the tooltip-accumulation guard (944-946), the chip reappend flicker (1436-1439), and `armSummaryClicks`'s account of rustdoc's `preventDefault` (1514-1520). fuelscape.css documents its one specificity fight with rustdoc by naming the rule it out-specifies (26-30).

## Open questions for Finch

1. Was the `spanbands` discriminator's question (which pair coordinate separates the `version_span` bands; classify-first versus emit-always) answered on the span branch? Nothing in the tree records a result. Recommendation: record the answer in an agent note and retire the binary with its two `before` features and the `suanpan` dependency (finding 21, option a); keep it only if the question is still live.
2. Is `CONCURRENT_PANELS = 1` a measured tuning outcome (memory, ETA stability, bar legibility) you want to keep as a knob? Recommendation: dissolve the pool to a plain loop (finding 18); git keeps it if per-panel concurrency ever pays.
3. Should dump accretion be tooled (`DumpWriter::open`, `--append-to`), or is measure-alone-then-hand-merge the intended workflow to be documented in the justfile? Recommendation: tool it (finding 15); the hand merge has no check for run-parameter agreement or duplicate names until `fuelscape-verify` diffs.
4. `fuelscape-verify` is `ci`-tier (about a minute of strict dump reading per the justfile). Given that `build.rs` consumes the committed dataset on every build and the byte-derivation is the only thing binding it to the dump, should it be gate-tier? Recommendation: measure the minute on the gate host; if it fits the wasm stream's budget, promote it.
5. The cross-platform hash pin (finding 7) runs only in the local gate; the commit that introduced it names the illumos gate run as the second-architecture witness. Recommendation: state that in the docstring, strengthen the fixture so more than one hashed value is libm-sensitive, and treat `fuelscape-verify` on ubuntu CI as the real-data cross-host witness it already is.
6. Is the probe-trace smoothing (finding 23) a presentation choice to keep? Recommendation: draw raw quantiles; at 4096 samples per column the smoothing removes column-to-column structure rather than noise, and interior anchoring exposes the inconsistency.
7. Is `typesetDocMath` on every workspace crate's pages intended (finding 27)? Recommendation: gate on `data-current-crate === "before"`; the other crates' authors did not opt into italic math for their lone-letter code spans.
8. Outside this partition: `crates/before/src/testing/fuelscape_islands.rs:22` declares `const EXEMPTIONS: &[(&str, &str)] = &[];`. The doctrine forbids a mechanism for accepting known failures "even as an empty buffer"; the design note frames it as a modeled-exemption list for derived impls with no doc site. For the testing-oracles reviewer or you: is an empty, reason-carrying exemption table a sanctioned model declaration here?
9. The design note §2:104-109 promises reader rejections that were never implemented (a histogram whose counts do not sum to `samples_per_column`, and roster presence checked in the reader rather than only in `compact_dump`). If the note is treated as a spec, it and the code should be reconciled; if not, no action.

## Dropped

- [12] The widget's x-axis caption says "total input size" for every operation: deliberate and recorded (design note §8 Q2 fixes one denominator for the whole widget; build.rs:230 emits "in total input bytes" for every island; the tooltip carries `size_measure`); the SVG renderer's unary/total split is a separate audit view and also omits the party for `version_tick`.
- [50] Hand-rolled SplitMix64 contradicts the manifest's rand_chacha rationale: refuted; `cell_rng` uses `ChaCha12Rng::from_seed` over a 32-byte key (the documented-portable path the manifest describes), and the test declines to ride rand's minor-bump value-stability policy for a committed hash; rand_core 0.6.4's docs (lib.rs:303-307, 333-334) frame these as different axes, not a contradiction.
- [20], [39], [54]: duplicates of finding 30 (rerun-if-changed).
- [22]: duplicate of finding 15 (accretion writer).
- [31], [43], [64], [7]: merged into finding 29 (build.rs module doc).
- [29], [33], [58]: duplicates of finding 18 (panel pool).
- [21], [41], [42], [59]: merged into finding 21 (spanbands).
- [28], [35]: duplicates of finding 9 (version-history docs).
- [26], [37], [65]: merged into finding 14 (tamper coverage).
- [60], [23]'s overlay half, and the refutation's new zero-size item: merged into findings 6 and 8 (positivity policy; writer never validates).
- [32], [36]: merged into finding 13 (fixtures and temp dirs).
- [44]: merged into finding 2 (qualified paths).
- [30], [47], [63], and the refutation's new clamp nit: merged into finding 4 (render.rs vestigial), with the drop claim corrected (the drops are load-bearing).
- [27], [62]: duplicates of finding 16 (write_atomic).
- [45]: merged into finding 1 (vocabulary); the CSS font stack half of [17] is finding 28.
- [55]: duplicate of finding 31 (untyped validation).
- [57]: duplicate of finding 22 (smoothSeries NaN).
- [53]: duplicate of finding 3 (max-bytes panic).
- [49]: split into finding 10 (overlay doc tense) and finding 11 (expect messages).

<!-- source: final/fuzz-guests-pins.md -->
# Partition fuzz-guests-pins: The libFuzzer targets, the fuzz-fit wasm guest, and the 32-bit boundary pins (guest and harness)

## Partition summary

This partition holds three detached instruments that execute `before` where the ordinary test suite cannot. The five libFuzzer targets in `crates/before/fuzz` feed arbitrary bytes and text to the public decode and parse doors under a shared peak-heap cap: `fuzz_decode` asserts byte identity on accept, `fuzz_decode_differential` holds the fused, borsh, and postcard paths to agreement on accept, value, and rejection genre, `fuzz_decode_ops` drives the clock op set on decoded trees, `fuzz_laws` expands the law-group roster over decoded values, and `fuzz_parse` round-trips the display notation. The fuzz-fit guest (`crates/before/fuzzfit/guest`) is a C-ABI register machine over the public API, compiled to wasm32 so the harness can meter each public operation in wasmtime fuel; it is the only route to fuel readings and ships a quadratic self-test burner as the meter's adequacy check. The wasm32-pins workspace is the tree's one place 32-bit code executes: the guest synthesizes canonical streams of hundreds of megabytes in-guest and drives the doors, walks, emitters, and rank arithmetic at the 2^29-bit, 2^29-byte, backend-capacity, and exponent-gap coordinates, and the harness pins each coordinate's outcome with adjacency witnesses on both sides.

The construction is strong where it matters most. The seed corpus is derived from the live API in one place and held byte-identical by a gate test; the differential target models rejection genres and stages explicitly and spells its one allowed divergence at the arm that admits it; the law target expands from `before::for_each_law_group!`, so a new law group is fuzzed with no wiring; the fuzz-fit guest keeps each measured window to exactly one public operation and pre-reserves its register file; the wasm32 guest documents every synthesized layout bit by bit with the strict decoder as the synthesizer's oracle, and the release profile keeps overflow checks on.

The dominant issues are instruments whose passing set is wider than the failure class they name. The fuzz heap cap is a flat 1 GiB against 4096-byte inputs, so any amplification below 2^18 times the input (a unit-constant quadratic included) passes, and nothing committed demonstrates the cap fires. The three memory-terminal pins assert `Trap::UnreachableCodeReached`, which every guest abort produces, so an overflow-check panic (the class the suite audits) or a backend-capacity panic at those coordinates reads identically to the modeled allocation failure; those same pins carry a red-first label while describing the terminal as intended, so their genre is unsettled. The rank pins observe only coarse order, which a decoder that drops the seam bit satisfies. The overflow-checks premise has no liveness self-test. The fuzz targets' own oracles execute only at `just all` cadence, `wasm32-pins/Cargo.lock` sits outside the supply-chain audit, and the wasm32-pins leg has no CI counterpart and no declared exclusion. The rest is prose and small structure: opaque roster IDs and a ghost test name in the fuzz manifest and README, a vestigial `BUILD_CAP_BYTES` name for a cap the tree no longer has, a harness fallback path that resolves outside the repository, hand-expanded accessor and verdict-table patterns in the fuzz-fit guest, and vocabulary tells.

Lines read: the sixteen partition files total 4808 lines (fuzz/Cargo.toml 98, fuzz/README.md 80, fuzz/src/lib.rs 48, the five targets 872, fuzzfit/guest/src/lib.rs 2038, fuzzfit/guest/Cargo.toml 15, wasm32-pins/guest/src/lib.rs 726, wasm32-pins/harness/src/lib.rs 115, wasm32-pins/harness/tests/pins.rs 743, the three wasm32-pins manifests 73), all read in full with line numbers, plus the cited regions of the justfile, `tests/support/fuzz_seed_set.rs`, `tests/fuzz_seeds.rs`, the fuzz-fit harness (`wasm.rs`, `ops.rs`, `bands.rs`, `tests/enforce.rs`, `tests/main.rs`), `clock.rs`, `clock/tests.rs`, `laws.rs`, `testing/algebraic_laws/tests.rs`, `borsh_impls.rs`, `span/wire.rs`, `version/rank/num.rs`, `validation_index.rs`, `tests/meter.rs`, `.github/workflows/ci.yml`, the sibling `.cargo/config.toml` files, `tools/doclint`, the resource-amplification agent note, and the `peak_alloc` 0.3.0 and `wasmtime-environ` 47.0.3 sources from the registry. `pins.rs` is the partition's only test file; the fuzz targets are instrument binaries and the two guests are instrument code. No cargo, just, build, or test command was run; every claim below rests on reading, grep, git history, path normalization, or arithmetic, and each entry says which.


## Positives

- The seed corpus is derived from the live public API in one place (`tests/support/fuzz_seed_set.rs`) and held byte-identical and stray-free by `tests/fuzz_seeds.rs`, which also pins each rejection witness to the exact genre it was written for; the two non-derivable frontier witnesses carry their bit-level derivations inline, and the wide-gamma seeds close a tail (64-plus-zero unary prefixes) coverage-guided search would never reach. Seed rot is a red gate with a one-command fix.
- `fuzz_decode_differential`'s rejection-genre axis is the right complement to round-trip fuzzing: explicit `Genre` and `Stage` enums make the allowance table legible, the one allowed fused-versus-composed divergence is spelled at the arm that admits it (143-154, 169-172) and matches `Span::decode`'s public `# Errors` contract ("the components' structural genres win"), and a committed seed per genre (`span_negative_join`, `span_crossed_padding`) means a reintroduced ordering defect fails the first replay rather than waiting on random discovery.
- `fuzz_laws` expands its drive loop from `before::for_each_law_group!`, so a law group added to the roster is fuzzed with no wiring and a novel signature refuses to compile; the arity band `0..=17` is derived from the balanced counter's structure with the reasoning stated (57-65), and the `Env` field docs say which decoded value feeds which slot and why.
- `fuzz_decode` asserts byte identity between the accepted input and its re-encode, which refuses an accept-and-normalize decoder outright rather than merely checking a round trip.
- The heap cap takes `peak_alloc` as a dependency rather than hand-rolling a `GlobalAlloc`, in line with the zero-unsafe policy even inside a fuzz binary, and its module doc states its own limit precisely (a spike that outruns the process is libFuzzer's RSS limit's job).
- The fuzz-fit guest keeps a strict measured/unmeasured partition (staging, register reservation, and query construction outside the fuel window; exactly one public operation inside), mirrors the API's linearity in the register file so a replayed program cannot alias a `Party`, keeps happy paths move-only with no defensive clones, and uses `split_at_mut` rather than cloning for two-register mutations. `ff_regs_reserve` keeps `Vec` doubling out of every fuel window, and the harness binds the reserve to both budgets with a const assert.
- `ff_selftest_quadratic` is the adequacy demonstration the doctrine asks of a criterion: a `black_box`-pinned known-bad mechanism the enforcement suite drives through the real wasm, fuel, and band-judge path (enforce.rs:273-310), in both the Above and the Below (liveness) directions.
- The wasm32 guest synthesizes every input in-guest with the strict canonical decoder as the synthesizer's oracle, documents each layout bit by bit with the canonicality argument stated (31-46, 237-250, 507-524), and the harness gives each coordinate below/at/past adjacency witnesses so a failure attributes to its seam; the join-emit pins construct complementary two-leaf skylines whose join concatenates, exercising the emitter's output side at sizes larger than either operand. Each pin runs in a fresh instance so one pin's peak cannot become the next pin's baseline, and `version_small_roundtrips_and_rejects_typed` is an always-green liveness pin for the leg.
- The wasm32-pins workspace keeps `overflow-checks = true` in the release profile with the rationale beside it, and the harness manifest explains why fuel and pooling are absent (outcomes, not counts; a full 4 GiB address space per pin).
- The justfile names each guest wasm path once and passes it explicitly at producer and every consumer, with the ambient-`CARGO_TARGET_DIR` failure mode it prevents written beside the variable (56-78); the detached fuzz workspace is compiled in the gate so a rename in `before` breaks a target in the same commit, with the reasoning for keeping the smoke at `just all` stated.

## Open questions for Finch

1. Are the three memory-terminal pins (pins.rs:130-153, 359-379, 715-742) a declared model or a pending cure? The header and the pins' own docs say different things. Recommendation: declare the model (the address-space bound is the terminal, and it aborts loudly), restate the three pins positively, narrow the header to seam pins, and add the origin discriminator (finding 35) so the instrument rather than the prose establishes "allocation failure"; if a leaner working set is wanted, own that as a `tests/meter.rs` envelope on decode's working set rather than a wasm32 pin.
2. Does the fuzz heap cap become proportional to input length, or is the flat cap ratified? Recommendation: proportional, with `PER_INPUT_BYTE` measured first over the seeds and a smoke run and committed with slack; either branch owes a committed test that the cap fires.
3. Is a seeds-replay gate leg (`-runs=0` per target, seconds) acceptable gate spend, or do the targets' oracles stay `just all`-only by design? Recommendation: add it; it is the cheapest liveness check the seed corpus already makes possible.
4. Is `fuzz_decode` retained for raw-door throughput and transport-feature independence? Recommendation: keep it and say so in one sentence of its module doc.
5. Should `just fuzz` pass `-max_len` above libFuzzer's 4096 default so multi-kilobyte trees reach the heap cap and the ops target by mutation, not only through the seeds? Recommendation: raise it modestly (16 to 64 KiB) once the cap is proportional; before that a larger `-max_len` only widens the blind window.
6. Does the wasm32-pins leg belong in CI's `instruments` job, or is it declared local? Recommendation: declare it local in both comments for now (the leg needs a wasm32 target plus wasmtime and a few GiB), and revisit when the `instruments` job's runner fit is next reviewed.
7. Should the eighteen signature arms move into an exported `laws`-feature macro shared by `drive_groups!` and `organic_drive!` (finding 12)? Owner-gated as an instrument-surface addition; recommendation: yes, with the pool-index choice fixed once.
8. Does clippy's `missing_const_for_thread_local` still misfire on the fallback-TLS lowering on illumos (fuzzfit guest 84-87)? Nobody in this review ran clippy there; if the lint no longer fires, the two `#[allow]`s and their comment are vestigial. Recommendation: check on ox-east-1 when the guest is next touched.
9. The three wasmtime pins (wasm32-pins 47.0.3, fuzzfit and fuelscape 47.0.4) and the two near-identical `guest_wasm_path`/`engine_and_module` drivers are the kind of duplication a shared tool-side crate would remove; the differing engine configs (fuel and pooling versus neither) argue against. Recommendation: align the three pins when the locks are next touched; leave the drivers separate.

## Dropped

- Straddle literals beside the named constant (adequacy [34], structure-prose [57], instrument-correctness [70]): duplicate of fuzz-guests-pins-34.
- `BUILD_CAP_BYTES` names a removed cap (structure-prose [37]): duplicate of fuzz-guests-pins-34.
- Terminal pins labeled as pending cures while documented as a model (adequacy [21], structure-prose [38], instrument-correctness [63]): duplicate of fuzz-guests-pins-33.
- Terminal pins assert a trap kind any panic produces (instrument-correctness [61]): duplicate of fuzz-guests-pins-35.
- `synth_version` wraps six bytes above the terminal (instrument-correctness [62]): duplicate of fuzz-guests-pins-27, with the count corrected to seven.
- Harness fallback path (adequacy [30], structure-prose [36], instrument-correctness [64]): duplicate of fuzz-guests-pins-32; [36]'s medium lowered to low because no gate or CI path reaches the fallback.
- Flat heap cap (adequacy [24], instrument-correctness [60]) and "not yet proportional" (structure-prose [51]): duplicate of fuzz-guests-pins-14; [60]'s acceptance on "the first seed" corrected (the committed seeds are at most 130 bytes, so a quadratic body peaks at about 17 KB there; the construction needs fuzzer-generated inputs near `-max_len`).
- Roster IDs (structure-prose [49], instrument-correctness [66]) and ghost test name (structure-prose [48], instrument-correctness [65]), and the bundle (adequacy [27]): duplicates of fuzz-guests-pins-2 and fuzz-guests-pins-3; [27]'s Cargo.toml:68 ride-along lives in fuzz-guests-pins-8.
- Manifest run commands drifted (structure-prose [50]): duplicate of fuzz-guests-pins-3.
- `fuzz_decode_ops` framing doc (structure-prose [46], instrument-correctness [72]): duplicate of fuzz-guests-pins-8.
- `ff_regs_reserve` path (adequacy [28], structure-prose [43], instrument-correctness [67]): duplicate of fuzz-guests-pins-20.
- Clock kernels bypass `with_c` (adequacy [35]); FNV twice and the sign mask (structure-prose [41]); accessor duplication (structure-prose [40]): folded into fuzz-guests-pins-18, at low per the refutation's reframe (a divergent copy fails loudly against the harness's native expectation).
- `dispatch!` arity list (structure-prose [45]): duplicate of fuzz-guests-pins-22.
- `fuzz_decode::run` six copies (structure-prose [52]): duplicate of fuzz-guests-pins-5.
- `debug_assert_eq!` compiled out (instrument-correctness [68]): duplicate of fuzz-guests-pins-30; [58]'s cited line 626 corrected to 552.
- Vocabulary tells (instrument-correctness [71]): split between fuzz-guests-pins-21 ("mint") and fuzz-guests-pins-17 ("honest", "real").
- `synth_version` and `synth_rank` hand-roll `set_bit` (structure-prose [59]): folded into fuzz-guests-pins-27's resolution, since the `u64` rewrite of `synth_version` routes through `set_bit` anyway.
- Trailing commas in five asserts (structure-prose [57]): folded into fuzz-guests-pins-34 as a rider; below the bar on its own.
- Guest synthesizer failures share the trap channel (structure-prose [39]): folded into fuzz-guests-pins-27 (the in-band synthesis codes) and fuzz-guests-pins-35 (the discriminator).
- The refutation pass's evidence corrections (lines 626 versus 552, nine versus eight "honest" sites, six versus seven bytes): applied in place; not findings.

<!-- source: final/fuzzfit-bands.md -->
# Partition fuzzfit-bands: The fuzz-fit harness: pinned fuel bands, curve fitting, wasm bridge, calibrate/probe/diag binaries, and the enforcement tests

## Partition summary

This partition is the fuel-band instrument for `before`: `bands.rs` holds the committed per-kernel log-log cost laws (49 `Band` constants keyed by kernel and outcome, four constant-classified `SMALL_BANDS` for rumors' bootstrap-scale operands, the `REFIT_COVERAGE` expectation list, `PINNED_RUSTC`, and the four judgment constants `ENFORCE_MARGIN`, `ENFORCE_MARGIN_BELOW`, `REFIT_PREFIX_PROGRAMS`, `REFIT_TOLERANCE`); `fit.rs` fits bucket-median log-log lines with two one-sided residual widths and provides `line_divergence`, the staleness comparator; `curve.rs` is the within-case shape leg (`local_slope_excess` against `SLOPE_ALLOWANCE`, with the two fold kernels in `SHAPE_EXEMPT`); `wasm.rs` drives wasmtime with a pooled instance allocator, a pre-reserved register file, and per-call fuel measurement; `bin/calibrate` sweeps the deterministic corpus, rewrites the generated tail of `bands.rs` below a prose marker, and prints the judgment constants' evidence to stderr; `bin/probe` and `bin/diag` are hand-run tools; `tests/enforce.rs` is the gate leg (the 48-case random sentry, the 256-program deterministic prefix judged total and refitted, the bootstrap replay, two fixed escalation replays, the nop liveness check, the quadratic-burner adequacy check, and the toolchain pin); `tests/sanity.rs` checks generator invariants natively plus the roster parity and the judge's own tripwire. I read every partition file in full (3135 lines) and, for context, `drive.rs`, `build.rs`, the cited regions of `ops.rs`, `strategies.rs`, and the guest, the justfile recipes, the design note, `validation_index.rs`, and the proptest 1.11.0 and wasmtime 47.0.4 sources in the cargo registry. Test files: `src/fit/tests.rs`, `src/curve/tests.rs`, `tests/enforce.rs`, `tests/sanity.rs`, `tests/main.rs`.

The instrument is well designed at the level the doctrine cares most about. Every judgment leg has a committed synthetic tripwire that states what it does and does not prove; the ceiling and floor widths are priced separately with an argument for why (the one-sidedly heavy residual cloud); the liveness margin names both measured edges it sits between; the register-file reservation is tied to both budgets by a `const` assertion, so a budget raise past it is a compile error rather than a reallocation inside a measured window; the staleness cross-check walks a committed coverage roster where a classification flip fails by name; and the deterministic prefix turns a sampled verdict into a total one with the `(1 - q)^48` argument written where the leg lives. The blessed-drift window section in `bands.rs` argues against its own instrument candidly, which is rare and valuable. The vocabulary is mostly anchored (band, arm, leg, reach family, liveness and regression flag are each tied to an identifier or defined by contrast).

The dominant issue is a single mechanism with several faces: the judgment constants are justified by measurements that `bin/calibrate` computes and prints to stderr, and the tree persists those measurements only as hand-transcribed prose in the head of `bands.rs` that the generator preserves verbatim. The 2026-08-04 toolchain re-pin rewrote every constant and no prose, so the head now states small-band numbers the generated region contradicts, a floor gap that recomputes differently, a rejection-envelope range one arm sits outside, and a shape-leg maximum that three documents give three values for. The same stderr-only design means the floors' liveness claim (every pinned floor clears the nop reading) has no committed test, and calibrate's evidence loop evaluates it at one endpoint over the main bands only. Two further mediums are criterion questions for the owner: the shape leg subtracts the pooled slope, which the pin's own prose says is mixture-inflated on twelve non-exempt keys, so a `d^1.4` mechanism on `ff_version_decode` passes both legs by arithmetic over the committed constants; and the staleness leg judges an exactly reproducible per-key quantity against one global tolerance, which is why the design note has to accept a 2x uniform meter undercount. The determinism premise everything replays on has no committed two-execution comparison (the only one is the `probe` binary no recipe runs, written before the pooled allocator landed), and the enforcement sentry's doc says the bands price "every public operation" while the guest exports some sixty measured kernels no band prices and no tiling accounts for. The rest is duplication (bucket medians, the deterministic stream loop, the `Fit`-to-`Band` transcription, a second guest call path), stale or hand-maintained prose, and idiom nits.


## Positives

- Two one-sided residual widths with separately priced margins (fit.rs:13-23; bands.rs:161-194): the ceiling carries the regression claim with a tight margin priced from enforcement-context replays, the floor carries liveness with a wide margin sized between the dead-meter reading and the legitimately cheap tail, each margin names the two measured edges it sits between, and `bin/calibrate` re-derives both on every run. Derivation beside mechanism, exactly as the doctrine asks.
- `REGS_RESERVE` is tied to both budgets at compile time (wasm.rs:39-47): a calibration constant that could have drifted into a mid-measurement reallocation is a compile error when a budget is raised; the seed that found the reallocation artifact is committed.
- Every judgment leg ships a synthetic tripwire that states what it does not prove: the burner test (enforce.rs:260-310) is explicit that it proves the wasm-to-fuel-to-judge path and nothing about generator reach; `fit/tests.rs` proves the comparator reads its perturbation exactly; `curve/tests.rs` proves fire, quiet, and abstain; `judgment_flags_quadratic_and_dead_readings` proves both flags.
- The deterministic prefix as a total verdict (enforce.rs:328-343) with the `(1 - q)^48` argument makes the difference between sampled and total judgment concrete, and the same programs feed the refit, so the leg's cost is the verdict rather than runtime.
- The small-band leg carries its own liveness floor (enforce.rs:232-239): a rostered kernel that never lands a step inside its calibrated span fails by name instead of passing as decoration.
- `REFIT_COVERAGE` is a committed expectation list with classification-flip detection (bands.rs:977-1038; enforce.rs:367-395): coverage decay, a reach regression, and line drift each fail by name, and the splice-marker assertion in calibrate (calibrate.rs:347-353) fails by name rather than silently regenerating an empty file.
- Identity-outcome routing (drive.rs:57-66) names where the fast paths' liveness lives (before's `identity_fast_paths` meter pins, verified present at crates/before/tests/meter.rs:10552) rather than leaving them unpriced without an owner.
- The blessed-drift window (bands.rs:42-70) is disclosed candidly with its mechanism, its measured size, and the reason it is the price of the margins; the doc argues against its own instrument where it must.
- Determinism is stated as a model (wasm.rs:4-11): fuel as a pure function of guest bytes, call sequence, and payloads; fresh guest per program; every downstream choice, including the pooled allocator's per-slot reset, is justified against it.
- The committed seed file corresponds to the live property (six entries, each a `program = [...]` drawn by `any_program()`), anchored where `tests/seed_liveness.rs` resolves by the deliberate `tests/main.rs`.
- `fit_constant`'s panic (fit.rs:194-210) is a genuine programmer-error guard with a one-line proof, and `judge_small`'s two-sided span cut (bands.rs:295-298) is justified in place.
- The detached workspace carries its own fmt and clippy leg (justfile:588-597) because the root sweep cannot reach it, and the 08-04 re-pin's commit message attributes all 188 constants to the toolchain change with "nothing else moved" checked against the board, the envelopes, and the trybuild snapshots: provenance kept in git the way the doctrine asks.

## Open questions for Finch

1. Re-pin cadence. The last re-fit is e7a4b7b0 (2026-08-04); substrate changes to `before/src` and the harness's own allocator model have landed since, all inside the drift window with the gate green. Is red-triggered re-pinning the intended cadence, or should substrate changes re-pin so the pin of record describes the code that runs? This interacts with findings 4 and 5. Recommendation: since the corpus is byte-reproducible and a re-pin costs only a diff review, re-pin at each change that touches the walked paths, and let a per-key slack (finding 5) decide when a re-pin is mandatory rather than the global 0.7.
2. Shape-leg reference (finding 10). If the reference moves to `min(band.slope, 1.0)`, does any lane legitimately trend above 1.0 within a case (the note names `ff_rank_display`'s digits-times-limbs conversion)? Recommendation: one `bin/calibrate` run with the new reference answers it and names the rows needing a declared exponent; declare those per key rather than loosening the global allowance.
3. `crates/before/AGENTS.md` (45 lines) never names the fuzzfit workspace, the `just fuzzfit`/`fuzzfit-calibrate` recipes, or the re-pin discipline, though `rust-toolchain.toml`'s own comment routes toolchain bumps through `just fuzzfit-calibrate`. Recommendation: one guidepost line pointing at the justfile recipes and `bands.rs`'s module doc.
4. `REFIT_COVERAGE` lists 48 of the 49 band keys; the uncovered one is `ff_rank_checked_sub`'s underflow arm, the pin's only flat-slope rejection arm (verified). Is that intended (too few prefix samples at pin time) and worth a note? Recommendation: have calibrate emit the uncovered keys with their reason into the generated region as part of `PIN_EVIDENCE` (finding 2), so the reader does not need the stderr of a run they did not perform.
5. Reach demonstrations (finding 24). The five known-bad reconstructions that accepted the instrument live in git history by the note's §8 design; the tree's standing demonstrations prove the judge on synthetic bands. Should one real-kernel sabotage demonstration per genre be committed (a guest switch adding a `black_box`-pinned burner to a named kernel, judged red on `ESCALATION_REPLAYS[0]`)? Recommendation: the doc fix regardless; the sabotage tests only if the owner wants the reach argument standing in the tree at the cost of one re-pin.
6. Dead-reading reference (finding 17). The floors are compared to `ff_nop` (2 fuel); a stubbed pair kernel that still borrows two registers costs more than that, so the `ff_rank_cmp` floor (about 2.6 fuel at 128 bits) holds liveness only against a return-immediately stub. Recommendation: measure a dispatch-shaped control kernel once before restating the floor doc; if the `ff_rank_cmp` floor sits under it, say so at `ENFORCE_MARGIN_BELOW`.
7. Do the six committed seeds still replay in-band under the 1.97.1 pin (bands.rs:113-115 claims the 139-bit overlap seed does)? Not runnable under this review's rules; a green `just fuzzfit` at HEAD settles it.
8. `SMALL_BAND_KERNELS` prices four kernels below 128 bits on a consumer-relevance argument; the other 40 kernels are unjudged sub-floor. Recommendation: leave the scope as-is (the premise, rumors' bootstrap hot path, is stated at the constant) unless rumors' hot path grows; the argument for the four is the argument against widening.
9. Informational: the design note (a dated record, exempt from the present-tense rule) states `ENFORCE_MARGIN_BELOW` as 1.0 (lines 383, 425) where the code pins 0.8, and names the seed path `enforce.proptest-regressions` (line 462); a reader cross-checking finds the discrepancies without a pointer to 875c118b, the commit that moved the margin. The fuzzfit tier's absence from CI (ci.yml:117-120) is deliberate and documented there; 4e64a4fb's message records a clean local gate for the wasmtime bump, so that question is answered for that bump.

## Dropped

- Guest toolchain provenance test checks the harness's compiler as a stand-in for the guest's [54]: deliberate and documented at the code site (build.rs:4-7 states the stand-in; e7a4b7b0 pinned one toolchain for both halves; wasm.rs:94-95 names `FUZZFIT_GUEST_WASM` as explicit operator provenance). A hardening suggestion, not a defect.
- No-op `as u64` casts on `encoded_bits()` [59]: out of partition (ops.rs belongs to the ops/strategies partition); forwarded there. Verified `encoded_bits` returns `u64` in version.rs:1151, party.rs:597, clock.rs:852; whether `just fuzzfit`'s clippy leg flags the same-type cast was not run here.
- Pin-time evidence prose drift [18], [28], [49]: duplicates of fuzzfit-bands-2.
- Shape leg judges against the mixture-inflated pooled slope [16]: duplicate of fuzzfit-bands-10.
- Noisy-linear test doc overstates its jitter [58]: duplicate of fuzzfit-bands-12 (its "~+0.12" was imprecise; +0.082 is the reproduced value).
- Seed-location ghost references [22], [35], [51]: duplicates of fuzzfit-bands-20.
- PROPTEST_CASES override claim [34], [50]: duplicates of fuzzfit-bands-21.
- Hand-maintained probabilities and counts [27], [36]: duplicates of fuzzfit-bands-26.
- Kernel roster hand-maintained [25], [40], [55]: duplicates of fuzzfit-bands-30.
- call_i64 second call path [33]: duplicate of fuzzfit-bands-15.
- Splice marker and duplicated rustdoc [41]: duplicate of fuzzfit-bands-18.
- probe.rs not in the gate / fuel determinism untested [17], [32]: duplicates of fuzzfit-bands-19.
- wasmtime pin convention unenforced [38]: duplicate of fuzzfit-bands-6.
- Floor evidence at min_denom only, skipping small bands [37], [53]: merged into fuzzfit-bands-17.
- Duplicated bucket medians, MIN_* constants, diag stream loop, Fit/Band transcription [23], [24], [29], [30], [31], [57]: merged into fuzzfit-bands-11.
- Em-dashes and past-tense narration in wasm.rs [45]: merged into fuzzfit-bands-14.
- Reach demonstrations not committed [3] as a medium verification-gap: converted to fuzzfit-bands-24 (documentation, low) because the design note records the decision that the demonstrations live in git history and the defenses they forced stand in the tree; the design question is carried as open question 5.

<!-- source: final/fuzzfit-strategies.md -->
# Partition fuzzfit-strategies: The fuzz-fit harness: input strategies, operation table, and the wasm driver loop

## Partition summary

This partition is the input side of the fuzz-fit instrument: `ops.rs` defines the program vocabulary (`Op`, 44 variants, each naming one guest kernel by string), the native `Mirror` that executes a program over the same register discipline the wasm guest uses and predicts each step's return code and denominated size, and the denomination rules; `strategies.rs` defines the budgets, the family roster (`Family`, eighteen coupled archetypes plus `Independent`, `Bootstrap`, and `Escalation`), the builder `B` that emits ops under unconditional budget caps, the per-family constructions, the measurement battery, and the proptest strategies `any_family`/`any_program`; `drive.rs` replays a program in the mirror and the guest in lockstep, panics on any disagreement, and enumerates the two deterministic corpora (the calibration stream and the bootstrap stream). The four build files detach the workspace from the parent, fix the guest's codegen profile, keep wasmtime's generated sources out of the doclint sweep, justify each dependency, and embed the building toolchain's identity.

The core mechanism is sound and well defended. The differential is total, not sampled: every step's return code and every live register's final bytes are compared and any disagreement panics. The mirror's identity predicates reproduce before's own dispatch exactly where they are mirrored (`version_buffers_alias` is pointer-plus-length, matching `Bits::ptr_eq`; `va == *vb` is `codec::canonical_eq` through `Version`'s `PartialEq`). Rejection arms are predicted per case by running the real operation natively and are priced as their own band key. Budgets are enforced structurally in the builder, pinned after the fact by the sanity suite, and tied to the guest's register reserve by a compile-time assert. The scope section names what the generators never construct and which instrument owns each excluded regime. Toolchain provenance is mechanical.

Four issues carry weight. First, the mirror models one of `join_view`/`meet_view`'s three O(1) rungs (canonical equality) and omits the empty-operand rungs, while the generators produce empty versions routinely and battery arm 6 feeds them to join and meet; the pinned `ff_version_join`/`ff_version_meet` floors are 2.79 and 2.84 decades wide against 0.38 to 0.87 on every other pair kernel, which is the smearing the exclusion exists to prevent and voids the liveness floor's claim on those two keys. Second, the guest exports 107 kernels and the vocabulary reaches 44: after the 7 control exports and 7 unmeasured query constructors, 49 measured public-operation kernels have no `Op`, no band, and no roster pin that can see them, while `ops.rs` and `lib.rs` claim one op per public operation. Third, the two fixed escalation replays are the suite's deterministic reach proof, but every budget refusal in the builder is silent and nothing witnesses that the depth-cap program was emitted whole; by hand count it fits on a 12 percent op margin no test reads. Fourth, the builder's `Ty`/`slots` liveness model is written at 17 `Ty::Dead` sites and 32 `alloc` sites and read by no code path except `.len()`, while three doc sentences describe a check it does not perform.

The rest is low-severity work: a second call path in the driver, a triplicated (and slightly overstated) identity-routing argument, a tag table restating `Op::kernel`, pins unbound to the corpus that produced them, hand-maintained counts, family docs that misdescribe their constructions, dead guards settled by an executed replay, duplicated mirror arms and family epilogues, hand-copied ABI constants, and a batch of idiom nits. Lines read: 3180 across the seven partition files, plus the cited ranges of `bands.rs`, `wasm.rs`, `lib.rs`, `bin/calibrate.rs`, `tests/sanity.rs`, `tests/enforce.rs`, the guest's `lib.rs`, before's `version.rs`, `party.rs`, `clock.rs`, `party/ops/split.rs`, `codec/bits.rs`, `version/rank.rs`, `tests/meter.rs`, `meter/registry.rs`, `laws.rs`, `testing/validation_index.rs`, `before-fuelscape/src/ops.rs` and its tests, the justfile, `ci.yml`, and the fuzzfit design note. No partition file is a test file; `tests/sanity.rs` and `tests/enforce.rs` were read as context and belong to another partition.


## Positives

- The differential is total, not sampled: every step's return code and every live register's final bytes are compared, and disagreement panics rather than being filtered (drive.rs:52-56, 76-97); `min_ticks` is digested identically on both sides so an unbounded count still rides the i64 channel exactly (ops.rs:913-920, guest lib.rs:685-692).
- The mirror's identity predicates reproduce before's own dispatch exactly where they are mirrored: `version_buffers_alias` (pointer plus length) is `Bits::ptr_eq`, and `va == *vb` is `codec::canonical_eq` through `Version`'s `PartialEq` (version.rs:1724-1727), matching the rungs at version.rs:391, 432, 865, 890, 913; the comments say which rung each op dispatches on and why the two predicates differ (ops.rs:610-611, 650-651, 672-673).
- Rejection arms are predicted per case by running the real operation natively, never assumed from the regime, and the rejected operand is handed back in both executions so it stays in the end-of-program differential (ops.rs:487-503, 790-806); rejection arms are priced as their own band key (ops.rs:279-287).
- Budgets are enforced structurally by every builder method, pinned after the fact by `sanity::programs_respect_the_budget`, and tied to the guest's register reserve by a compile-time assert (wasm.rs:41-47), so a budget raise past the reserve is a compile error rather than a reallocation inside a measured window.
- The scope section (strategies.rs:41-62) is a precise map of the negative space: wide magnitude, codec rejection, and empty folds are each named with the instrument that owns them instead.
- `ESCALATION_MAX_DEPTH` is one constant shared by the draw range and the replay pin (strategies.rs:123-140), so widening the family cannot strand the replay below the true far end; `MAGNITUDE_SHIFT_CAP` makes the shift's definedness local to the construction rather than resting on the draw ranges (113-121).
- The denomination rules are stated once in the module doc (ops.rs:9-30), each departing arm carries a one-line comment naming its rule class, and the rank proxy states the direction of its error (under-counting reads as more fuel per bit, so it can mask no superlinear growth).
- The `Independent` regime keeps its pool of record consistent after consuming ops (`per_universe[j].parties.retain`, 1845, 1874, 1938), so cross-universe draws never hand the mirror a consumed register.
- The escalation construction is careful about linearity: the deep-overlap poison is placed off the snapshot cadence (1526-1533), every party-ladder arm leaves the lane as it found it (1576-1584), and the deepest-snapshot join and the seed/snapshot sync are disjoint by construction, so the at-scale success and rejection arms are each reached deliberately.
- The two deterministic streams (`for_each_deterministic_program`, `for_each_bootstrap_program`) are pure functions of case index, so calibration and the staleness cross-check are literally the same stream.
- `harness/Cargo.toml` justifies each dependency by what it serves outside itself, `fuzzfit/Cargo.toml` and `.cargo/config.toml` each state the mechanism and the failure it prevents, and `build.rs` plus `PINNED_RUSTC` make toolchain provenance mechanical rather than conventional.
- drive.rs's public functions carry accurate `# Panics` sections and its `expect` messages are one-line proofs ("live_regs only reports live slots", "family strategy cannot fail").

## Open questions for Finch

1. Is the fuzz-fit vocabulary deliberately frozen at the 44 operations it reached on 2026-07-26, with the fuelscape atlas as the sole (non-enforcing) coverage for the 49 measured kernels that have no `Op`? The design note and 61398dc9 record public-API additions as fuzzfit re-pin events that fail by name, and the six fuelscape commits that grew the guest each say the vocabulary was deliberately left unchanged, but no ruling records the division. Recommendation: bind the vocabulary to `METHOD_SURFACE` with a reviewed exemption list (finding 7), add the operations that fit the register machine today (`ticks`, `forks`, the `*_all` doors, `span`/`span_all`, `eq`, the `Rank` codec, `Ranked`, clock text I/O) and re-pin, and record the remaining exemptions per operation.
2. Does the fuzzfit clippy leg (`cargo clippy --all-targets -- -D warnings`, justfile:596) pass at HEAD? The two same-type casts at ops.rs:764 and 858 would ordinarily trip `unnecessary_cast`, and the mixed modulo idiom (`is_multiple_of` at 1648 and 1650 beside `% == 0` at 1528 and 1547) looks like a partial lint-driven edit. 4e64a4fb (2026-08-31) reports `just gate clean (398s, all legs)` with the casts present, and the gate's wasm stream runs this recipe; CI does not (ci.yml:117-120). Either clippy is silent here for a reason I could not run down by reading, or the wasm gate stream has not run clean since 05d87e1b (2026-08-19), which matters beyond the casts because it tests whether that stream is being run before commits. Recommendation: one `just fuzzfit` run settles it; if red, treat it as a process finding, not a lint nit.
3. Relay to the bands partition: 4e64a4fb bumped wasmtime to 47.0.4 in the fuzzfit Cargo.lock, which calibrate.rs:366-368 declares a re-pin event, without touching bands.rs. The staleness leg tolerates 0.7 decades, so whether fuel moved is unknown. Recommendation: pin the wasmtime version beside `PINNED_RUSTC` and assert it, the way the toolchain is asserted, so the convention becomes a check. Also for that partition: `ff_clock_join` and `ff_clock_sync` success arms carry `width_below` of about 1.5 decades (bands.rs:394, 466), far above the other construction kernels; is the version half hitting an identity rung while the denominator still counts both versions (the genre of finding 11), or something in the party join?
4. The bootstrap family also emits `ClockFork` sub-floor, which is not in `SMALL_BAND_KERNELS` and is therefore judged `BelowFloor` (skipped) in the bootstrap replay; is fork deliberately outside rumors' bootstrap hot path as the roster defines it? Recommendation: if fork is on the hot path, add it to the roster and re-pin; if not, one sentence on `SMALL_BAND_KERNELS` saying so.
5. In `IdPairLockstep`, both lanes fork every level and only one descends (strategies.rs:992-1000); the un-descended lane's fork is what deepens its id in lockstep, so I read it as intended, and finding 18 asks for it to be documented rather than removed. Recommendation: confirm, and let the field doc say so.

## Dropped

- Em-dashes in twelve `//` comments (strategies.rs): a crate-wide pattern (374 such comments in before/src per the history pass), governed by owner doctrine rather than a project hard rule; belongs to a crate-wide prose pass, not a partition finding.
- Tautological `denom_bits >= 1` assertion at sanity.rs:51 (raised by the refutation pass): out of this partition (tests/sanity.rs); relayed to the fuzzfit tests partition.
- "Honesty bound" sentence stated as false (candidate 37): reframed by the refutation and history passes to imprecision (true with the constant's dependence on the tick/fork/fold caps named); folded into finding 15.
- Comb draw ranges exceed the tick budget (candidate 41): silent truncation is the builder's stated design (strategies.rs:31-33, 358-360, and the design note); the residual field-doc inaccuracy is folded into finding 18.
- "organic" and "divert" as unanchored coinages (part of candidate 27): dropped per the refutation and history passes; `organic` is crate-wide vocabulary (laws.rs:11) and `divert` is the meter board's own flag name (registry.rs:598).
- Candidates 14, 18, 34 (write-only `Ty`): duplicates of finding 19. Candidates 16 and the cast half of 30: duplicates of finding 13. Candidates 26, 39: duplicates of finding 17. Candidates 28, 42: merged into finding 3. Candidate 33: duplicate of finding 11. Candidate 13 and the escalation half of 3: merged into finding 16. Candidates 7, 17, 31 and the guard half of 11: merged into finding 21. Candidates 0, 19, 35, 4: split into findings 6 (documentation) and 7 (coverage). Candidates 24, 25, 40, 15 and the doc residual of 41: merged into finding 18. Candidate 29: reframed into finding 8. The non-cast items of candidates 11 and 30 plus the refutation's arm-selector ranges: merged into finding 9.
- The refutation's `honest` qualifier sites in sanity.rs, enforce.rs, and bands.rs: out of this partition; noted at the end of finding 15 for those reviewers.

<!-- source: final/meter-core.md -->
# Partition meter-core: The meter module: adversarial generators and deterministic resource meters, with its tests

## Partition summary

`crates/before/src/meter.rs` (3726 lines) is the crate's instrument library. It holds about seventy private generators that build adversarial inputs in closed form (the dense spine, the boundary combs, the memo probes, the freeze and promotion families, the seam and ladder shapes, the multi-operand populations), the `Packed` output type with its `version()` door into the stored skyline coding via `skyline::encode_bits`, and the public counter readers (`stack_segments`, `limb_ops`, `densified_digits`, `touch_ops`, `span_traffic`, `emit_traffic`, `pool_misses`, `scan_bits`) that the envelope suite, the amplification board, and the rumors crate's metering tests read. `crates/before/src/meter/tests.rs` (1648 lines; the partition's only test file) pins a subset of the generators (closed-form length, strict wire round-trip, and a semantic leg such as `min_ticks`, an exact rank, or a text spelling) and witnesses the counters' liveness, reset, and determinism. Every generator is reached from exactly one arm of the registry's exhaustive `Shape::builder` match (registry.rs:309-379), which is the compile-time tie that keeps the roster and the generators in step.

The quality of what is pinned is high. Each generator states its parameter contract under `# Panics` and enforces it with an `assert!` whose message reads as the proof; the one non-obvious frontier (the ascend-cliff code band) is proptested from both sides. The pinned families carry a semantic leg independent of the construction: `min_ticks` stored-base sums, the harmonic telescoping rank `1 - rank(H(d)) = 1/2^d` at depth 4096, `plateau_puncture`'s answer-embedded rank `(2xy + 1)/2^(66d)` beside a pin of its stored size, and readable text spellings for the re-arm, gap-spine, parked-spine, and tooth-tail families. The counter witnesses floor on irreducible work (a strict decode reads every live bit; a join writes at least its output's bits) and every process-global comparison carries `ISOLATION_NOTE`. `meet_shade` documents why its shades are built in distinct buffers so the flatness band measures the byte compare and not the folds' clone-identity collapse, which is the worst-passing-artifact question asked and closed at the generator.

The dominant issues are three verification and documentation gaps that share one cause: the module's prose was written for a world where the construction language was the stored coding, and the flag-day change to skyline storage was never carried through here. First, the module doc says every generator is pinned by this module's tests and round-trips through `Version::decode`; twenty registry-dispatched generators have no pin at all, four of their stated exact sizes are wrong, and six memo-family event shapes reach every consumer through the unvalidating `Packed::version()` with no strict decode anywhere. Second, the `Packed` doc says `bytes` is what `decode` accepts, which is false for event shapes, and the `cliff_comb` and `wide_tooth_comb` funding arguments describe the stored coding as a hypothetical. Third, the stack-segments meter has no writer in any binary that reads it under the `meter` feature, so the envelope column and the board currency it feeds are compiled-in zeros. Beneath those sit two regime-claim problems: six constants copy the query fold's freeze allowance by hand with no binding test, and the `wide_arming`/`hoisted_window` guard admits widths at which the promotion their docs describe cannot fire. The remainder is duplication, a handful of doc-versus-code arithmetic slips, and prose nits.

Total lines read: 5374 in the partition, plus the anchors cited under each finding in recurse.rs, encode.rs, version.rs, query.rs, query/integral.rs, query/web.rs, board/family.rs, board/measure.rs, board/currency.rs, board/judge.rs, board/floors.rs, board/ceilings.rs, registry.rs, tests/meter.rs, Cargo.toml, gamma.rs, signed.rs, skyline.rs, ticks.rs, and the rumors crate's `src/tree/typed/untyped/tests.rs`. No cargo, just, or build command was run; every numeric claim below is a hand derivation from the code as read.


## Positives

- The registry's exhaustive `Shape::builder` match (registry.rs:301-380) is a genuine compile-time tie: every generator is reached from exactly one variant, and the module's helpers (`ev_leaf`, `ev_spine`, `hole_region`, `gap_spine`, `parked_unit_spine`, `ascend_spine`, `seam_stop_descent`, `jump_pair_operand`) are called only by generators. No dead generators.
- Every generator states its parameter contract under `# Panics` and enforces it with an `assert!` whose message reads as the proof (for example meter.rs:1605-1611 derives the ascend band bound inline), and the one non-obvious frontier is proptested from both sides: `ascend_cliff_band_guard_admits_exactly_the_documented_frontier` (tests.rs:1627-1648) builds at `k = 2^b − 2` and panics at `2^b − 1` through both callers.
- Where a family is pinned, the pin carries a semantic leg independent of the construction and through the public API: `min_ticks` stored-base sums for a dozen families; the harmonic telescoping witness `1 − rank(H(d)) = 1/2^d` that keeps the pin exact at depth 4096 (tests.rs:328-337); `plateau_puncture`'s exact rank `(2xy + 1)/2^(66d)` through the public fold beside a pin of its stored size `128w + 198d + 2` so the floor's denominator cannot drift (1207-1240); full-walk `Less` verdicts for the correlated triples and quadruples, fused and materialized alike (671-728, 778-796); and text spellings that pin the bit-level layout against the tree the doc reasons about (206-250, 896-919, 1358-1383, 1433-1465, 1476-1527).
- The counter witnesses are the right genre: `scan_meter_counts_deterministically_and_resets` floors at `first >= bits` (a strict decode must read every live bit) and the join leg at the output's own bits; `span_traffic_classifies_each_rung` reads the full four-cell snapshot per witness in both operand orders so a miswired arm moves its own cell rather than a total; every process-global comparison appends `ISOLATION_NOTE` (tests.rs:21-25).
- Every counter behind the readers is documented as a relaxed process-global atomic with its isolation requirement stated once per reader, and every reader's doc names the failure class only it can see (3543-3723).
- `meet_shade` (3345-3350) names the clone-identity hazard and builds each shade in a distinct buffer so the flatness band measures the byte compare and not the folds' collapse: the worst-passing-artifact question asked and closed at the generator.
- `factor_digit`/`dense_factor` (2325-2369) state why the content stream is hand-rolled: byte-for-byte reproducibility of committed factors, structureless under the settle's own balanced compaction, which is the domain payload that justifies not reaching for a PRNG dependency.
- The generators' only recursion (`weight_comb`, `freeze_parade`, `stagger_comb`, `stagger_id`) is bounded by the log of a power-of-two parameter and says so at the site (3199-3202); `encode_bits` is iterative over a heap stack.
- Every `usize`/`u32` narrowing goes through `try_from(..).expect(..)` with a one-line reason (`Packed::from_bits`, `pow2`, `plateau_puncture_factors`, `seam_stop_descent`), and the size-formula subtractions (`202n − 4`, `1546k − 2`, `m(4L + 6) − 2`) are guarded by the constructors' knob asserts.

## Open questions for Finch

1. Segments currency (meter-core-11): dissolve it, or keep it with every doc re-stated to say the reading is structurally zero under the `meter` feature? Recommendation: dissolve. The 2026-07-24 keep was adjudicated two days before 05bd2b16d gated the only writer behind `cfg(test)`, so the recorded rationale ("its zero reading over the library kernels is the measured fact") no longer describes a measurement; `deep_tree_stack_safety` and the `cfg(test)` gate on `descend!` already hold the property.
2. Where should the roster-wide canonicality pin live (meter-core-2): a table-driven test in meter/tests.rs over every `Shape`, or `Packed::version()` routed through `Version::decode` under `cfg(any(test, feature = "meter"))`, or both? Recommendation: both. The pin makes the closed forms explicit and catches size drift; the structural door means no consumer can ever measure a non-canonical stream, at the cost of one validation pass per shape before the counters are reset.
3. `Packed` as one type or two (meter-core-3): keep one type with an accurate doc, or split into id and event types so `version()` exists only for event shapes? Recommendation: split, since the meter surface is feature-gated instrument API rather than the library's production surface, but this is your call under the stable-API rule.
4. `wide_arming`/`hoisted_window` (meter-core-8): raise the guard to the promotion threshold and re-parameterize the `hoisted_window` band at a promoting width, or re-state the docs to the freeze-only mechanism the current widths realize? Recommendation: land the promotion tap and pin first (it settles the trace in a run), then raise the guards, deriving the threshold from `FREEZE_ALLOWANCE_DIGITS`; the band re-pin is a consequence for the envelope partition.
5. Should `FREEZE_ALLOWANCE_DIGITS` become `pub(crate)` so meter.rs derives its six constants from it (meter-core-7)? Recommendation: yes; the alternative is six hand-copied numbers and a test that pins the derivation from outside the query module.
6. Cross-partition pointer, not a question for this partition: skyline.rs:125-127 says the tier2 length agreement is "proptested over every adversarial generator family", while tier2/tests.rs imports eight meter generators. I did not read the tier2 test bodies; the tier2 or skyline partition should check whether that sentence is true as written.

## Dropped

- `touch_ops`/`reset_touch_ops` have no caller (adequacy [8]): refuted. The rumors crate calls both at `src/tree/typed/untyped/tests.rs:676` and `:682` under its `meter` feature, which is the audience the reader's doc names; the readers were added for that consumer. The residual observation that before's own suites call `suanpan::touch_meter` directly (121 sites) is a consistency asymmetry below the bar.
- "The luck-proof touch list" names nothing (instrument-correctness [26] and the same claim in [6], [12], [18]): corrected. The list exists at registry.rs:572-583; what remains is an unanchored coinage, merged into meter-core-1.
- "genre" as an unanchored coinage ([18]): below the bar. It reads as ordinary English for a cost class and tests/meter.rs:37-43 defines the two floor genres by contrast.
- The gap-spine lean/turn loops as byte-identical copies ([16] item b): reframed. The four spines share a skeleton but differ in phase and turn leaf; kept as a parameterization item inside meter-core-6.
- `seam_stop_descent` as part of the seam-assert triplication ([4]): wrong. Its asserts (`k >= 1`, `k <= 1 << 11`) are a different set; only `seam_plunge_control` repeats `seam_plunge`'s three.
- Envelope pin count of 62 ([22]): undercounts; the harness has 172 `envelope(` sites, none with a nonzero segments argument. Merged into meter-core-11.
- The `Ticks` string round-trip as a missing conversion ([19]): the string door is the deliberate API (`Ticks` has `From` for unsigned machine integers only, keeping the suanpan type off the stable surface); reduced to a test-local helper inside meter-core-5.
- Duplicates merged: [22] into meter-core-11; [7], [15], [25] into meter-core-2; [14], [23] into meter-core-3; [16] into meter-core-6; [11], [17], [28] into meter-core-14; [12], [13], [18], [21] items 1 and 2, [26] into meter-core-1 and meter-core-9; [21] item 3 and [27] into meter-core-12; [6] items e and f into meter-core-5 and meter-core-9.

<!-- source: final/meter-registry-tier2.md -->
# Partition meter-registry-tier2: The family registry and the tier-2 meters

## Partition summary

The registry (`crates/before/src/meter/registry.rs`, 1988 lines) is the roster every resource instrument in `before` derives its family axis from. It has two halves. `Shape` is the single public door to the private adversarial generators: a 68-variant enum whose exhaustive `builder()` match binds each variant to one generator and whose thirteen accessors (`packed1`, `packed2`, `packed_pair`, `versions`, ...) dispatch on the generator's signature class at runtime. `FamilyId` is the 52-variant family roster; each variant's `spec()` arm is its row of record (`FamilySpec`: name, cited shapes, a `Coverage` answer that is either a board column with a declared cell reach or a dated envelope-only ruling, a `Bands` answer that is either a roster of `tests/meter.rs` test names or a dated no-band ruling, plus two free-text fields `denominator` and `closed_form`). `FamilyId::ALL` is the hand-written roster array, `index()` a 52-arm match restating its order, `board()` the filter on the coverage answer, and `AXIS_BANDS` the table of band names no family row carries. The registry's own tests (`registry/tests.rs`, 226 lines) pin the seams the compiler cannot: roster order, name uniqueness, every shape cited by some family, band citations unique and non-empty, the board roster non-empty with nonzero reach, and rulings dated.

The sizer (`crates/before/src/meter/tier2.rs`, 123 lines) is an instrument, not a codec: `tier2_size` walks a construction-language stream (the generators' min-lifted preorder form, one gamma base per node) iteratively over a heap stack of inherited path sums and returns the skyline coded size of the version it denotes, decomposed into topology bits, first-leaf bits, and delta bits, with its own `zigzag` and `gamma_bits` so that it is an implementation of the coding independent of the encoder. Its tests (`tier2/tests.rs`, 735 lines) hand-pin the sizer on small trees, pin the boundary comb's closed forms, commit the known-bad plain-accumulator sweep that motivates suanpan, and hold two join/meet coding lemmas under four emitters (the public operators and the emission kernels, the kernels also asserting their emitted stream length against the sizer): a 1-Lipschitz ceiling with a 4-bits-per-leaf slack and a subadditivity lemma with a derived, tight 2-bit margin.

The quality is high where the compiler or a committed test does the holding. The `compile_fail,E0603` doctest is a committed demonstration that the raw generators are unreachable outside the door; `Coverage` and `Bands` as sum types force an explicit written reason for every non-answer, with no expected-failure buffer anywhere; the declared `cells` reach is enforced at the shard merge and in the rendered smoke matrix; the band-name parity pin in `tests/amp_board_smoke.rs` is bidirectional and attribute-gated; the subadditivity constant is derived term by term and its tightness is a committed equality witness rather than an assertion; the plain-sweep tripwire is value-exact and two-scale on a deterministic counter. The dominant issues are prose that no instrument holds: `tier2.rs` still describes the stored coding as a candidate and the deleted construction-language coding as "today's" (inverting the code since the flag-day commit faf3cd0a), several registry reason strings name enforcement homes that do not hold the named pin, three flatness bands rest their adequacy on uncommitted "probe builds" where every sibling band has a rostered known-bad kernel, and the hand rosters (`ALL`, `index()`, `ALL_SHAPES`) are checked only against each other so a variant outside `ALL` reaches no instrument while `index()`'s doc claims the opposite. On the test side, the Lipschitz pin is implied pointwise by the subadditivity pin over the same emitters and populations and can fold into it.

Lines read: 3072 in the partition (registry.rs 1988, registry/tests.rs 226, tier2.rs 123, tier2/tests.rs 735), all with line numbers, plus the cited neighbors (version.rs, skyline.rs, skyline/tests.rs, testing/compactness.rs, board/cell.rs, board/family.rs, board/shard.rs, board/ops.rs, meter.rs, codec/base/limb_meter.rs, codec/base/limb_metered.rs, codec/base.rs, codec/bits.rs, recurse.rs, tests/meter.rs ranges, tests/amp_board_smoke.rs, tests/superlinear_tripwires.rs, query/tests.rs ranges, suanpan metered.rs header, surfacecheck check.rs, before/AGENTS.md) and the commit messages the history pass cited. `registry/tests.rs` and `tier2/tests.rs` are test files. No cargo, just, build, or test command was run; every "verified" below is reading, grep, or git.


## Positives

- The construction door is demonstrated, not asserted: the generators are private and the `compile_fail,E0603` doctest at registry.rs:23-27 is a committed known-bad artifact (a direct generator call) held failing, with a passing doctest through `Shape::CliffComb.packed2(4, 4)` beside it. I found no raw constructor caller outside `meter`'s own module tree.
- `Coverage` and `Bands` as sum types force a written reason for every non-answer, and there is no expected-failure buffer anywhere in the registry: every `EnvelopeOnly` and `Unbanded` row states positively where its enforcement lives.
- The declared `cells` reach is hand-written but lives in a mechanically-enforced place: the shard merge (shard.rs:505-520) and the rendered smoke matrix (amp_board_smoke.rs:72-118) both hold it, so a bundle slot gained or lost fails loudly with the family named.
- The band-name parity pin (amp_board_smoke.rs:314-389) is bidirectional, `#[test]`-attribute-gated so helpers never count, and self-checking (a scan matching nothing fails on every citation). Within its convention it is exactly the right shape of pin.
- The registry's failure messages tell the reader which roster to edit, and `band_citations_are_unique_and_nonempty` refuses an empty `Priced` roster so a family cannot claim bands vacuously.
- `tier2_size` is iterative over a documented heap stack of inherited path sums, with the alignment argument stated where the stack lives (tier2.rs:59-62) and a totality assert on consumed bits; even as an instrument it honors the no-depth-recursion rule.
- `JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS` is derived term by term (topology, first leaf, the `gamma(2m) = gamma(2m − 1)` observation), and `empty_pair_is_the_subadditivity_equality_case` commits the tightness witness so the margin cannot loosen unnoticed; the emitter table pins the kernels' own emitted stream lengths against the sizer, so the lemma prices the output bytes, not just the value.
- `cliff_comb_plain_delta_sweep_is_quadratic_in_tier2_wire_bits` is the committed known-bad demonstration for the accumulator premise done right: deterministic counter, two-scale ratio, mechanism stated, and the value telescoping to zero asserted so a wrong demonstrator cannot pass. It is cited from production rustdoc (skyline.rs:106).
- The query-fold family docs (FreezePos through LatentLadder) each name the mechanism, the worst artifact, and the band that holds the cure, at the maintainer's altitude.

## Open questions for Finch

1. The compactness envelope and Euler-tour charge in `testing/compactness.rs` (out of this partition) relate two fixed codings on instrument-built inputs; since the flag day no production change can move them, and the module still calls the envelope "the claim its adoption turns on". faf3cd0a retained them deliberately as decision-era ratios. Recommendation: dissolve the envelope and charge checks and their measured constants (history lives in git and the design note), keep `tier2_size` as the independent sizer (its live consumers are the length-agreement pins), and re-denominate its prose per finding 14. If the ratio stays as a size-relationship record, its module doc needs the same present-tense correction.
2. Are the `decided` dates decision records (exempt from the dated-rationale rule) or declaration-site rationale? d2a9d04e reported this as needing a design round. Recommendation: drop them; git blame carries the ruling date. If kept, one sentence in the module doc naming the exemption and one shared real date parse for the registry and surfacecheck.
3. Should wide-tooth and jump-comb route through public `Version::rank` (as the weight-comb run does) and become board columns? Recommendation: route through the public entry regardless (the technical reason for the internal entry ended at the flag day); keep them `EnvelopeOnly` with a truthful reason unless the board wants the shapes, since their axes (window width, one eviction) are kernel-seam probes by the board-roster criterion.
4. Cliff-fan: dissolve or re-justify (finding 12)? Recommendation: dissolve the family and the generator; if the agreement corpora want the shape, keep it as a corpus member cited from a coding-corpus row, not as an adversary.
5. `strum::VariantArray` (already in Cargo.lock transitively, would be a new direct dependency of `before`) versus a local declaration macro for the variant rosters (finding 11). Recommendation: strum; declaration order is already the roster order, and the derive dissolves two tables and a test.
6. The em-dashes inside `reason` and `AXIS_BANDS` disposition strings (registry.rs:1631, 1644, 1970, 1975, 1982, 1986): d2a9d04e classified these as prose-in-string sites, and nothing renders them at runtime (they are read only for non-emptiness), so the colon rule for message text does not apply. Recommendation: leave them.
7. Two verified items outside this partition for the coordinator to carry: the validation index (`testing/validation_index.rs`) has no row for the family registry or the coding-lemma pins though it claims to list every instrument (grep for `registry|tier2|lipschitz|subadditiv` is empty); and meter.rs:26-27 speaks of "the luck-proof touch list" on `FamilyId`, a phrase that appears nowhere else in the crate.

## Dropped

- tier2 sizer and compactness envelope outlive the coding decision [0]: the sizer half is refuted (it is the independent second implementation the length-agreement pins at skyline/tests.rs:514-523 and the kernel emission pins at tier2/tests.rs:387-394, 406-413 rest on, with the independence rationale stated at skyline.rs:125-129); the compactness half is out of partition and carried as open question 1.
- Coverage::Board { cells } repeats five bundle-pattern counts across 33 families [4]: deliberate and documented (registry.rs:1055-1058, 1068-1070; a44502fe, a06e2be0, b4f50c08); the per-family declaration is the tamper-evidence design and the count is enforced at two sites; a categorical reach is an alternative design, not a defect.
- Validation index has no row for the registry or the coding-lemma pins [9]: out of partition (validation_index.rs); verified true by grep; handed to the coordinator in open question 7.
- Shape door dispatches signature at runtime [13]: deliberate and documented (registry.rs:12-15, 87-95, 302-308; `# Panics` on every accessor, programmer-error only); taste without a defect.
- Registry attributes an internal-entry decision to sites that contain none [17]: merged into meter-registry-tier2-10 as leg (3).
- tier2.rs prose framings [1], [16], [26], [40] and the tests' "current"/"today's" vocabulary [27]: merged into meter-registry-tier2-14.
- Lipschitz pin dominated [2], [20], [28], [43] and the emitter doc copy [49]: merged into meter-registry-tier2-16.
- Hand rosters and index() [3], [14], [29], [42]: merged into meter-registry-tier2-11.
- Probe builds as adequacy witnesses [5], [15], [39]: merged into meter-registry-tier2-7.
- Unread spec fields [6], [18], [30]: merged into meter-registry-tier2-8 (severity low per the refutation).
- Dated rulings [7], [31] and the dated half of [48]: merged into meter-registry-tier2-9.
- Restated readings [8], [19], [46]: merged into meter-registry-tier2-6.
- Vocabulary [10], [35], [47]: merged into meter-registry-tier2-1; the em-dash-in-strings sub-point is dropped as deliberate (d2a9d04e) and never rendered, see open question 6.
- Repeated incantation [12], [23], [32]: merged into meter-registry-tier2-17.
- Tautological roster test [22], [34] and the tautology half of [48]: merged into meter-registry-tier2-13.
- Reason strings [38] with the CliffFan depth [37]: [38] is meter-registry-tier2-10; [37] is kept separately as meter-registry-tier2-12 because its resolution (dissolve or re-justify a family) is distinct and owner-gated.

<!-- source: final/oracle-laws.md -->
# Partition oracle-laws: The paper-shaped reference oracle and the named algebraic and representational laws

## Partition summary

This partition is instrument code, compiled only under `cfg(test)` or the `oracle`/`laws` features (`lib.rs:435-445`). The oracle (`oracle.rs` and its three type files, 916 lines) transcribes the ITC paper's recursive trees: `Party` and `Version` are the trees, every paper operation is a method, children sit behind `Arc` so the derived `Clone` is a refcount bump, and the module doc states an operating envelope (small scope, bounded depth, literal ticks) that every harness cap I checked honors (`MAX_TRACE_OPS = 30`, `ARB_DEPTH`, the iterated-tick law's `0..=3`). Its property suite (`oracle/tests.rs`, 829 lines) is the ground-truth gate: lattice and order laws over op-trace populations, the paper's worked examples with section citations, and a grow-optimality suite pinned against a brute-force enumeration that shares no arithmetic with the DP. The laws module (`laws.rs`, 3,493 lines) is one macro-registered roster of `(name, fn) -> bool` predicates grouped by input signature; `laws!` makes registration structural (a law cannot exist unregistered or under a foreign name), `for_each_law_group!` gives every consumer (two proptest drivers, the organic drive list, the fuzz target, the surface-coverage citation haystack) compile-time refusal of a novel signature, and `laws/tests.rs` (135 lines) pins the collection's own invariants. I read all 5,373 partition lines with line numbers, plus `fold.rs`, the production `join_all` contracts, the `join_all` differentials in `party/tests.rs` and `clock/tests.rs`, the surface roster and its citation checks, the generators, the law drivers, the fuzz target, `grow.rs`, `grow_brute_force.rs`, and the git history the history pass cited. The test files are `oracle/tests.rs` and `laws/tests.rs`; `laws.rs` is itself an instrument.

The quality is high. Paper transcription is exact at every case I traced (`node`/`is_normal` implement the §5.2 normal form; `leq`, `join_off`, `fill`, `split`, `sum` match the reference); fallible operations are quantified over outcomes, arm and payload; conditional laws construct their antecedents where the module policy says to, with `projection_additive_over_carved_regions` falling back to a fork-half decomposition so no population leaves it vacuous; the conservation laws state at the law why they quantify over region unions rather than byte identity, and I traced the deterministic witness `[a, alias(a), b, alias(b), c]` through the counter and confirmed the hand-back really is a coalesced group. The arity sweep is derived from the counter's structural boundaries, not observed values.

The dominant issue is at the oracle's edge. `Party::join_all` and `Clock::join_all` are not paper definitions but two verbatim copies of production's `fold::balanced_try_fold`, and the differentials that consume them pin the hand-back vector element-wise, contents and order, although the public contract now declares which inputs are absorbed versus handed back unspecified. The history pass shows this was deliberate when the contract documented the discipline (ace236595) and that the premise expired in the owner's docs pass (b3f09baa0), which also deleted the oracle-side docs that stated the purpose. The oracle module doc still claims obvious-correctness and paper transcription for a file that carries forty lines of stack discipline. Everything else is smaller: a medium doc/body mismatch in two oracle tests, a false one-to-one mirror claim over a dead `has_seen` and a renamed `receive`, a production-cost import whose stated rationale names no constructible input, a totality pin that scans source text where a compile-time tie exists, and a batch of prose and idiom nits (the banned "mint", an undefined "door" used 41 times, `unwrap` inside law bodies, `% n` pool indexing).


## Positives

- The operating envelope (oracle.rs:14-32) states once where every input bound lives and why hardening the reference would be a fidelity loss, and the harness caps I checked honor it: `optrace::MAX_TRACE_OPS = 30`, the generators' `ARB_DEPTH`, the iterated-tick law's `0..=3` (laws.rs:2507-2519). Depth stress runs against the impl alone, as promised.
- `laws!` (laws.rs:186-246) makes registration structural, and every consumer expands from `for_each_law_group!` as the docs at laws.rs:84-92 claim: I verified `algebraic_laws/tests.rs:304` and `447`, `fuzz_laws.rs:225`, and `surface_coverage/tests.rs:110`. A novel signature refuses to compile in every consumer.
- Fallible operations are quantified over outcomes, arm and payload: `join_commutative_outcomes` (2203-2213) requires both orders to agree and to leave `self` unchanged on `Err`; `join_associative_outcomes` (2273-2290) and `span_intersect_associative_outcomes` (1570-1583) do the same over `Option`. The worst passing artifact of an `Ok`-only law is closed.
- Conditional laws construct their antecedents where the policy says to: `order_transitive_constructed` (896-900), the `lag_*` constructed-plus-incidental pairs (933-950), `projection_monotone_in_version` (2623-2628), and `projection_additive_over_carved_regions` (2787-2809), whose fork-half fallback through the same `without` entries guarantees the equation runs on every call.
- The conservation laws state at the law why they quantify over unions (2402-2416, 3352-3360), and the witness in laws/tests.rs:78-135 is genuine: tracing `[a, alias(a), b, alias(b), c]` through the counter yields hand-backs `alias(a)` and `alias(b) ∪ c`, the latter byte-distinct from every input, exactly as asserted.
- `arb_fold_arity` (generators.rs:370-406) derives its band from the counter's structural boundaries (first in-counter combine, first drain, first merged-merged carry, two octave crossings), not from observed values.
- The grow-optimality suite (oracle/tests.rs:537-643) is independent of the DP it checks: `grow_brute_force` enumerates the whole inflation space with exact `+ 1` arithmetic and no shared `deepen`, and its header states precisely what the impl-equals-oracle differential alone cannot catch; the §3 event-condition reading is stated with the counterexample that rules out the literal reading (610-623).
- `Arc` children on both oracle trees (party.rs:7-10, version.rs:32-35) are explained by the cost model they buy, and `Version::meet_all` (version.rs:366-379) is the paper-faithful `reduce` with the missing-top rationale stated.
- The paper's worked examples are transcribed with section citations (`event_normalization` §5.2, `split_equations` §5.3.2, `sum_and_join` §5.3.3, `event_fills_to_single_integer` §5.3.4, `worked_example` §5.1), so the oracle can be audited against the source directly.
- `#![allow(clippy::type_complexity)]` at laws.rs:60-63 carries its rationale inline, the doctrine's preferred shape; the `RANK_TRIPLE` `(a, b, _c)` renames make ignored inputs explicit and arity-checked.
- `join_all_differential_convicts_the_dropped_group_oracle` (party/tests.rs:266-290) proves the differential criterion can fail, at exactly the widths that reach the retention arm.

## Open questions for Finch

1. Is `join_all`'s hand-back grouping and drain order a pinned behavior? Today both `# Errors` sections say the absorbed set is unspecified, and the only pin is the oracle copy (oracle-laws-2). Recommendation: no; make the oracle sequential, compare verdict, accumulator, and region union, and let the laws carry the contract. If yes, say so in the contracts and pin it once at `fold.rs` (oracle-laws-26).
2. Should `Leg::Law` citations resolve against `laws::registered_names()` alone? Three roster rows (`surface.rs:761, 1127, 1133`) cite `#[test]` items with `Leg::Law`; two of those compare a public operation against an internal kernel entry. The leg's documented meaning ("on production alone, by test name") is satisfied, so the premise of the lens finding misreads the docs, and the site is outside this partition. Recommendation: route to the surface-roster partition as a question of whether the `Law` kind should mean "member of `crate::laws`".
3. Should the oracle `Clock` track production's method names (`recv`, and drop the deleted observer trio) so `optrace` stops translating, or be documented as a semantic mirror with named divergences (oracle-laws-1)? Recommendation: track the names; the translation layer is small and the false claim is the cost.
4. "door" has 237 uses across `crates/before/src` with no definition (oracle-laws-19). Recommendation: replace with the plain term crate-wide in one sweep, or define once in the crate docs; the partition-local edit alone would leave the word inconsistent.
5. For bundled laws (oracle-laws-16): change `Law<F>` to return the failing clause's name (a feature-gated public type change, roster stable) or split the bundles (roster citations move)? Recommendation: the `Result<(), &'static str>` form; it keeps the citation haystack stable.
6. Does cargo-mutants under `--all-features` mutate `src/laws.rs` and `src/oracle/`, which are gated `cfg(any(test, feature = ...))` rather than `cfg(test)`? If so, a law body mutated to `true` is a missed mutant by construction and the manual campaign carries that noise; if the campaign scope excludes instrument modules, where is that stated? Not verified; no tool was run. Recommendation: state the scope in `.cargo/mutants.toml`'s header.
7. `oracle::Version::grow`'s `(Party::Leaf(true), Version::Node)` arm (version.rs:272-286) extends the paper, whose `grow(1, e)` is defined only for a leaf `e`; the arm is unreachable through `event` (fill collapses the region first) and exists for the direct grow tests. Recommendation: one sentence at the arm naming it as the `1 ≡ (1, 1)` extension.

## Dropped

- [2] A `Leg::Law` citation may name any `#[test]`: out of scope (anchors in `surface.rs` and `surface_coverage/tests.rs`, the surface-roster partition), and the premise misreads the leg docs, which define `Law` as "on production alone, by test name" (history: 67956d1fc predates `crate::laws`). Carried as open question 2.
- [5, part] `Default for oracle::Version` has no caller: refuted; clippy's warn-by-default `new_without_default` under the gate's `cargo clippy ... -D warnings` requires it for a type with `pub fn new()`, and no `allow` exists anywhere in the crate.
- [11] The roster carries one consumer's driver names: deliberate and documented at laws.rs:88-89 ("into the per-group proptest drivers (by the driver names carried here)"); the one-sentence ask is already met.
- [25, shim half] Three `_for_test` visibility shims: refuted; `#[cfg(test)] pub(crate)` is the only way to scope crate visibility to test builds, so removing the wrappers is a taste trade, not a free simplification. The cost-pair half survives as oracle-laws-5.
- [12], [24], [36]: duplicates of oracle-laws-2 (merged; 36's `crate::fold` routing rejected per b9f6af2d7's recorded reason; 24's oracle-local helper is a sub-step of the design question, not an alternative).
- [15], [23]: duplicates of oracle-laws-1.
- [18], [32], [42]: duplicates of oracle-laws-3.
- [21], [37]: duplicates of oracle-laws-13.
- [31]: duplicate of oracle-laws-15.
- [40]: duplicate of oracle-laws-17.
- [43]: duplicate of oracle-laws-12.
- [27], [28]: duplicates of oracle-laws-22.
- [39]: merged into oracle-laws-25 (the compile-time tie dissolves it; the scan patch is the fallback).
- [9]: merged into oracle-laws-5.
- Refutation new item 2 (oracle.rs's single-omission claim): merged into oracle-laws-1.
- Refutation new item 3 (fc06c5e8's rationale names no constructible input): merged into oracle-laws-4.

<!-- source: final/party.md -->
# Partition party: Party: the id tree, its packed-bit operations (build, compare, diff, index, split, sum), forks, and idbits

## Partition summary

The partition is the identity half of `before`: `Party` (a nonempty dyadic share of `[0, 1)` stored as a canonical 2-bit-per-node preorder stream), the consuming cursor `IdReader` over that coding (`idbits.rs`), and the kernels that operate on it directly: `split` (a positional spine walk plus verbatim splices), `sum` (a two-cursor lockstep merge writing final tags and collapsing `(1, 1)` by fixed-width truncation), `covers`/`is_disjoint` (one shared predicate walk, `lockstep_holds`, parametrized by the single node that settles a pair), `diff` (a boolean-skyline sweep over the overlay law with covered-block early exits), `sum_split` (the fused join-then-fork behind `Clock::sync`), and `IdIndex` (a per-fold random-access table over one fixed operand, so `join_all` does not re-walk the accumulator per input). `forks.rs` layers the balanced k-way split and its residual-keeping iterator on `fork`. Every walk is iterative, with depth priced in bits, and the differential suite in `party/tests.rs` holds each kernel to the recursive oracle, to constructed deep witnesses at 10 000 and 100 000 levels, and (under `scan-meter`) to derived floors and ceilings.

The kernels are correct as far as reading establishes, and the maintainer-facing argument is unusually complete: `sum_split`'s fusion proof, `Lockstep`'s empty-stack-on-unary-chains invariant, `diff`'s covered-block taxonomy, `PosStack`'s delta coding, and `IdIndex`'s stated trade are each written where the code lives, and every `unreachable!`/`expect` reads as a one-line proof. The known-bad oracle in `join_all_differential_convicts_the_dropped_group_oracle` and the exact two-sided fork orbit pins are the adequacy and shape-over-point instruments the doctrine asks for. The dominant issues are not in the mechanisms but around them: a cluster of ghost references that breach the tree's hard rule (`IdLit`, `EvNode`, a removed `compare` op, a nonexistent oracle note, a "recursive form" label on a loop); public claims contradicted by the code or by the crate's own pins ("no allocation" on `covers`/`is_disjoint`, "nothing allocates" on `shape`, "strictly smaller than the operand" on the index table, `O(n)` on the tuple-literal door, "exactly `k`" shares at `u64::MAX`, the `join_all` `# Errors` contract, the fold index's `B` term); one headline promise with no instrument (`forks`' minimal-depth balance); a 32-bit `ExactSizeIterator::len` panic reachable from caller input; and a fold-index cliff past 2^32 packed bits that the public bound does not carry. The structural remainder is fixed-sign and small: the 2-bit tag decode and subtree skip hand-spelled at six sites, three walks stacking bits on the output buffer type where `BitStack` exists, a redundant right-child scan in `split`, and the `sum_split` spine accumulator duplicating input bits.

Files read in full with line numbers: `party.rs` (913), `party/forks.rs` (206), `party/ops.rs` (50), `party/ops/build.rs` (370), `compare.rs` (190), `diff.rs` (438), `index.rs` (291), `split.rs` (119), `sum_split.rs` (223), `sum.rs` (194), `party/tests.rs` (1162), `idbits.rs` (218): 4374 lines, of which `party/tests.rs` is the one test file (there are no other `tests.rs` siblings in the partition; the other files hold no inline test blocks). Supporting files read in part: `version/skyline/overlay.rs`, `codec/{stack,scan,buf,build,literal}.rs`, `tests/meter.rs`, `tests/forks_max.rs`, `before-fuelscape/src/ops.rs`, `laws.rs`, `fold.rs`, `clock.rs`, `clock/forks.rs`, `meter/board/{ops,floors,coverage}.rs`, `fuzzfit/harness/src/bands.rs`, `testing/{generators,exhaustive}.rs`, `version/skyline/{shape,grow,fill}.rs`, and the fold-unification survey in `.agent-notes`. No cargo, just, or test command was run; every "verified" item below is a grep, `git show`, or read fact, and every complexity derivation is a hand trace of the code.


## Positives

- `sum_split`'s method doc (sum_split.rs:13-66) is a real fusion argument written where the code lives: the spine-is-lockstep and branch-is-verbatim-or-merge lemmas, the no-double-read mode choice, and the one seam (`split`'s terminal arm) named with the differential that pins it. It is held by a total byte-equality oracle (`sum_split_is_sum_then_split`, `None` arm and empty-operand identities included) plus deterministic deep witnesses at 10 000 levels in every regime.
- `lockstep_holds` (compare.rs:59-101) parametrizes the two region predicates by the single `a_settles` node with the rest of the algebra fixed and shared; `Lockstep`'s doc (103-112) states the invariant that keeps the stack empty on unary chains and why, and each call site names its refuting mixes in one comment.
- `sum` writes each output tag final at descent and collapses `(1, 1) → 1` by retracting a fixed-width suffix (sum.rs:69-76, build.rs:137-150), so no position stack exists; the comment at 69-73 says exactly why the tag is knowable at first sight.
- `diff.rs`'s module doc (18-38) gives the covered-block taxonomy completely (all four pairings, which three are blocks and why the fourth is not, where lockstep descent takes over), and the private docs on `settle_a`/`settle_b`/`descend_pair` restate exactly the cover the caller guarantees.
- Every `unreachable!`/`expect` in the partition is a one-line proof that reads true against the surrounding code (diff.rs:126, 300, 419-421; sum_split.rs:87, 92, 183, 201; index.rs:253; sum.rs:183-190; build.rs:323; idbits.rs:190). None is reachable from decoded bytes, text, or the public API.
- `PosStack` (build.rs:328-370) delta-codes reserved tag positions so a deep `diff` output costs bits per open ancestor, with the off-by-one for delta 0 explained at the field.
- `IdIndex`'s module doc (index.rs:1-33) names the trade it makes (a search term not bounded by the operands), the instruments that price it, and why it exists outside itself (the fold's repeated test against one fixed side); `build_unindexed` pins the fallback arm without a 512 MiB input. That candor is what made party-22, party-23, and party-25 findable.
- `join_all_differential_convicts_the_dropped_group_oracle` (tests.rs:172-290) commits a known-bad reference and asserts the criterion rejects it at exactly the widths that reach the arm: the adequacy demonstration the doctrine asks for, spelled as a test.
- The fork orbit pins (tests.rs:1044-1101) are exact two-sided trajectories over 512 steps in both directions, closing the compounding-cost gap a per-call bound leaves open, and the constructed-shape module (586-649) builds deep witnesses tags-first in one allocation so `diff` and `sum_split` are byte-checked at 100 000 levels and oracle-checked at the scales the recursive oracle survives.
- `diff_block_scan_never_exceeds_the_complement_walk` (tests.rs:910-963) derives its floor (every operand tag once) and ceiling (plus the settled output once plus a named `BLOCK_SLACK`) from the mechanism rather than from a measurement, and states why a re-derived block would land far past it.
- `From<Party> for [Party; N]` rejects `N == 0` at monomorphization with a `const` assert and pins it with a paired `compile_fail,E0080` doctest against its compiling twin (forks.rs:164-172, 190). `Open` is `!Clone` and `#[must_use]` (build.rs:39-40); the doc overclaims what that buys (party-17), but the discipline itself is right.
- `Party::seed` documents the `static`-not-`const` choice at the declaration (party.rs:122-130): the one thing the code cannot show, said in five lines.

## Open questions for Finch

1. **The parity-halves search floor is a measured reading × 0.75** (tests.rs:1139-1147, `SEARCH_SCAN_FLOOR_BITS = 101_397`), following the envelope suite's stated convention (tests/meter.rs:6397-6398), while the board follows your derived-floor ruling. A galloping search from the left edge (about 2·log₂(left-subtree entries) + 1 probes per node) would do strictly less work and trip this floor; so would party-26's bounded search. Recommendation: derive the floor from the mechanism in the test (on the parity halves every skeleton node is both-present, so at least `2^d − 1` searches of at least one 32-bit probe each: `(2^d − 1) × 32` plus the tag reads, about 38 876 bits at `d = 10` against the 6 140-bit unmetered reading), and decide separately whether the envelope suite's × 0.75 convention should stay confined to `tests/meter.rs`. Not filed as a finding because the convention is recorded at the site and is yours to rule on.
2. **Should `split`'s raw spine walk and the `split`/`sum_split` copies be metered** (party-27's option (a): route through the shared tag helper and a `PackedBuilder`-shaped copy, re-pinning `ID_FORK`'s scan column from 3 to a spine-proportional number, and moving the `spliced == 8` constant), or stay raw with the exemption stated at the code (option (b), what party-27 resolves to)? The answer also decides party-34's fix (meter the fused side vs. compare reads only). Recommendation: (a), because `IdReader::read` plus `pos()` walks the same spine at the same cost and the "deliberately raw" record reads as an accident later ratified by a pin; but it is a re-pin, so it is your call.
3. **Three public-doc passages were replaced or removed in your 2026-08-04..08-07 doc passes** (a431eaf1, b3f09baa0) without messages, and in each case the earlier agent-written text is the correct reference for the repair: the `forks` saturation clause (party-14; cdad46060's wording), `Party::decode`'s Party-specific warning (party-7; e546b6d5e's text), and `join_all`'s coalescing clause in `# Errors` (party-8; 4f12b8218's text). Were the removals deliberate concision, or collateral? Recommendation: restore the substance of all three in your own words; the pin in `tests/forks_max.rs` and the laws roster both describe behavior the current prose contradicts.
4. **The `>2^32`-bit fold-index fallback** (party-23): widen the table (`Narrow`/`Wide` enum, or `Vec<u64>` if the `party_join_all` heap cell tolerates it) and dissolve `build_unindexed` and the fallback differentials, or carry the size clause in the island contract? Recommendation: widen; the crate's "for all input sizes" sentence is the one you chose to make load-bearing, and the fallback arm exists only to be held by tests that would then go away.
5. **`ExactSizeIterator::len` on 32-bit targets** (party-13): document a `# Panics`, override `len()` to saturate, or drop the impl? Recommendation: override `len()` to saturate at `usize::MAX` with one sentence saying so, and add the wasm32 pin; it keeps the public surface and removes the only caller-reachable panic in the partition.
6. **`IdIndex`'s per-ancestor state in machine words** (party-24) and the two `IdLeafCursor`s (party-20): the first is an exemption from the "depth costs bits" discipline nothing states; the second is a recorded drop nothing in the tree states. Recommendation: fix the first (the `u32` fit is already proven by the table guard) and state the second at diff.rs:229-244; if you would rather reopen the merge, the refutation pass confirmed a lazy-entry face on `overlay::IdLeafCursor` is additive for every eager client.

## Dropped

- [7] item 3, `IdReader::at`'s doc "see `split`'s `build_split`" is dangling: disputed by the refutation and by my read; `subtree_end` (split.rs:115-118) is `build_split`'s helper and does call `IdReader::at`, so the pointer is imprecise, not wrong. Below the bar.
- [7] item 4, overlay.rs:17's "two cursor instances" is a wrong count: module-scoped and deliberate (c6ba2208 trimmed the crate-wide roster); folded into party-20 as a scoping clarification.
- [29]/[44], the parity-halves floor is measured × 0.75: the rationale is recorded at the site and the convention is suite-wide; moved to open question 1.
- [40], `NA_SCAN_SEED_PARTY` says the seed's packed form is empty: the anchor (meter/board/floors.rs:143-145) is outside this partition. Route to the board partition; the false premise is real (the seed is the 2-bit terminal `00`, party.rs:122-123 and the doctest at 593-594; the anonymous id is the empty stream, party.rs:643-644).
- [38], [52]: duplicates of [0], merged into party-20.
- [43], [53]: duplicates of [3], merged into party-19.
- [27]'s `finish_id` half: duplicate of [8], merged into party-10.
- [24]: duplicate of [13], merged into party-5.
- [31]: duplicate of [15], merged into party-32; [15]'s bundled aliased-inputs item is party-31.
- [35]: duplicate of [19], merged into party-14.
- [39], [46]: duplicates of [18], merged into party-22.
- [41]: duplicate of [21]'s split half and [7]'s item 2, merged into party-3; [21]'s compare.rs half is in party-10.
- [42], [50]: merged with [11] into party-28 (one suffix precondition, three faces).
- [57]: merged with [2] into party-27 (the pointer and the exemption share one fix).
- [56]: merged into party-2 (a caller roster that drifted).
- [22], [25], [60]: merged into party-7 (public rustdoc slips in one file).
- [9], [12], [14]: merged into party-6 (idiom nits).
- [13]/[24], [32], [33]: merged into party-5 (register sweep).

<!-- source: final/rank.md -->
# Partition rank: Rank and Ranked: the total order extending causality, and its numeric core

## Partition summary

The partition is the causal rank kernel: `Rank` (crates/before/src/version/rank.rs, 1096 lines) is an exact dyadic rational `num · 2^-exp` with normalized storage (odd numerator whenever `exp > 0`, zero pinned to exponent zero) so that derived `Eq`/`Hash` are value equality; it carries `Ord` (an O(1) magnitude-class test, then an MSB-window stream on class ties), `Add`/`checked_sub`/`Sum` (a backend route when both aligned operands fit the big-integer backend, a suanpan `Accumulator` route past that), and a prefix-ascending wire form whose byte order equals `Ord` (an inverted-polarity Elias delta integral part, then continuation-framed eight-bit fraction groups, then a close bit). `Num` (rank/num.rs, 650 lines) is the two-arm numerator: a `Base` (dashu `UBig`) below the backend's capacity ceiling and a raw `Vec<u64>` limb vector above it, existing so that on 32-bit targets a decoded numerator past `2^32 - 32` bits is a value rather than a backend panic; a test-only thread-local override lowers the ceiling so both arms and their seam are reachable at host sizes. `Ranked` (ranked.rs, 389 lines) is the total-order view over a `Version`: rank first via a fused signed co-sweep, canonical version bytes on rank ties, with a composite key whose byte order is `Ord` on the views. rank/num/tests.rs (191 lines) is the partition's one test file: differential unit suites for the wide arm against `UBig` as oracle under a 96-bit ceiling. The partition's behavioral suites live in version/tests.rs (the rank, wire-form, wide-arm, ranked, and composite-key sections, roughly lines 753-2115) and in the `RANK_TRIPLE`, `VERSION_SOLO`, and `VERSION_PAIR` law groups in laws.rs; I read those sections, the skyline query kernel (`rank`, `rank_cmp`, `pair_fold`, `Integrator::finish`), suanpan's `shl` and `sign_limbs`, the meter and board pins for rank rows, the fuelscape roster and JSON, the borsh boundary, the wasm32 pins, and the pinned dashu-int 0.5.0 source to settle anchors. Total: 2326 partition lines read in full plus roughly 900 lines of anchors elsewhere.

The mechanisms are correct and total as far as reading can establish, and the argumentation is unusually complete: the wire-form module doc is a checkable proof sketch with every rejected alternative named and the concrete reason it fails, the decoder allocates only from bits actually read, `Rank::cmp` is obviously right and cheap, and the two-arm dispatch invariant is stated once and held to the real backend on the load-bearing side by the wasm32 pins. No correctness defect in the code was found by any lens or by me.

The dominant issues are claims the code does not honor, and one instrument aimed at the wrong adversary. `Ranked`'s public `# Complexity` promises `O(|self| + |other|)` while the kernel it calls runs the same mass-balanced settle as `distance`/`lag` (the crate's own doc stated the M-bound at commit 3bba6cbb and lost it when contracts moved into the fuelscape roster). `Ranked::encode_rank` is documented at four sites as a fused emission cheaper than `rank().encode()`, and has been `rank().encode()` spelled in three lines since the commit that introduced it, so the `raw_parts`/`encode_parts` seam and two public efficiency sentences describe nothing. The bold public size claim "never is larger than the version it measures" fails at small scale (a one-byte version whose rank encodes to two bytes). `sum_ranks`' amortization argument is false for ascending exponent order, and both committed pins fix the benign order while documenting it as the adversary. Below those: the wide arm's sixteen meter hooks feed a counter nothing observes, a handful of local dedupes (two routing copies, two shift kernels, a one-caller wrapper, four byte-source adaptors), a test doc that claims hashing coverage the body lacks, and prose habits (dated "historical" rationale, moralized "honest", hand-maintained literals, a sentence hand-copied eleven times) that the crate shares beyond this partition.


## Positives

- The wire-form module doc (rank.rs:8-131) is a complete, checkable argument: the inverted-polarity delta header is shown bijective (so no rejection genre is needed for non-minimal headers), the fraction's in-band framing is justified against a length header by the concrete 1/2-versus-7/16 counterexample, prefix-freeness is derived from the close bit, and every rejected alternative (gamma/omega/varints, dsi-bitstream's codes, ordered-varint, the FoundationDB tuple form) is named with the mechanism that rules it out. The decoder's comments (722-787) mirror the argument clause for clause. All four lenses independently singled this out; I confirm it by reading.
- The decoder's allocation discipline is stated and kept: every allocation is fed by bits actually read (`BitSink::new` grows while the encoder's `with_capacity_bits` preallocates, 827-843; the fraction's depth is counted from consumed bits, 784-788), the numerator is assembled by byte concatenation rather than a value-width shift a 32-bit `usize` cannot hold (792-811), and the exhaustive 0-2 byte sweep asserts both rejection genres fire and acceptance is live (version/tests.rs:1087-1095), so strictness cannot pass vacuously.
- `Rank::cmp` (882-906) is obviously right and cheap: zero settled first (the one even-numerator form whose class would collide with `(0, 1]`), an O(1) class test on `bits(num) - exp`, then MSB-aligned streaming windows through one shared kernel with zero allocation and a tail rule whose premise (odd numerators) is stated at the site; the alignment-oracle sweeps and the `RANK_TRIPLE` law group pin it against a definitionally correct oracle.
- num/tests.rs turns the backend into the exact oracle for the arm that exists because the backend cannot hold the value: under a lowered ceiling every wide-arm value is also a `UBig`, and each `Wide` operation is checked one at a time (materialize at every pad, `shr` with redispatch, the bias steps as inverses, window comparison against materialized shifts, decimal rendering). The 96-bit ceiling is deliberately not a multiple of 64, so partial-limb windows and sub-limb shifts are exercised, and the seam corners (all-ones carry across the ceiling, power-of-two borrow back) have exact unit witnesses.
- The test-only ceiling override (num.rs:101-139) is thread-local, scoped by a `Drop` guard that restores the previous value so scopes nest, and documented as routing-only with the caveat that values must not outlive their guard.
- `BACKEND_CAPACITY_BITS`'s stated derivation (`usize::MAX / word-bits` words) matches dashu-int 0.5.0's `Buffer::MAX_CAPACITY` (two lenses verified this in the registry source; I read the crate's `Word` and NTT dispatch but did not re-read `Buffer`).
- `Ranked`'s `Eq`/`Ord`/`Hash` coherence is stated once (rank then bytes; `Equal` is version identity) with the O(1) canonical-equality rung before the co-sweep, pinned by `ranked_orders_by_rank_then_bytes` and `ranked_encoding_orders_like_ord`; the composite decode carries the two-ways check (the rank stream must be the rank the version measures), witnessed from both sides.
- `sum_ranks`' value identity with the pairwise fold is pinned by `rank_sum_equals_the_pairwise_fold` over arbitrary multisets and orders, and `rank_cross_path_normalization` pins `Eq` and `Hash` agreement across add, sum, and add-then-subtract paths, under the lowered ceiling too.
- No em-dash appears in any `assert!`, `expect(`, or `unreachable!(` message in the partition; the messages that exist ("the top limb is nonzero", "a wide value is nonzero by invariant", "the Greater pre-check promises a strictly positive difference") state their premise.

## Open questions for Finch

1. Do you want the single-limbs-arm numerator (rank-22)? It is a design proposal, not a defect: the two-arm form is correct and instrumented. The trade is a large deletion of seam machinery (override module, routing predicate, dual routes, `Base` shims, the wide-regime suite copy) against re-pinning two envelopes, a possible `SmallVec` dependency to keep small ranks allocation-free, and keeping the encode walk's width record for the board's `rank_encode` limb floor. My recommendation: pursue it after rank-32 and rank-16 land, since both shrink the seam's surface first and make the remaining machinery easier to weigh.
2. For `Ranked::cmp` (rank-33): restore the M-bound contract now (a roster string and a regenerated JSON) and treat a sign-only kernel with a domination-certificate early exit as separate future work? I recommend restoring first; the linear class was never true and a cheaper kernel cannot make exact ties linear.
3. Should `Sum` carry a public `# Complexity` island? No fuelscape op exists for it; the board's `rank_sum` cell is high-first only. The crate's "every asymptotic claim is a guarantee" is silent where no claim is made, which is itself a gap for a public operation. I recommend adding the island after rank-20's instrument and cure land, so the stated bound is the achieved one.
4. Crate-wide vocabulary rulings the partition inherits: "honest" (127 uses), "door" (208 uses, no definition in lib.rs), and em-dashes in `//` comments (375 lines). I recommend one crate-level decision for each (define "door" by contrast once in lib.rs or retire it; replace "honest" with mechanisms in a crate-wide pass; settle the dash rule) and have the partition follow, rather than editing these three files alone.
5. Once `Ranked::encode_rank` collapses (rank-32), do you want `raw_parts` retained as the board's limb-denomination accessor (meter/board/ops.rs:2277) or replaced by a `#[cfg(any(test, feature = "meter"))]` accessor beside `content_bits`? I recommend the gated accessor: the remaining callers are tests and the meter board, and the `VERY IMPORTANT` warning (rank-11) then has one home.
6. Should suanpan export its digit width or a `reserve_bits(u64)` entry (rank-14)? I recommend `reserve_bits`: it keeps the digit base private and removes the only cross-crate literal.
7. The refutation pass inferred, without running it, that under `.cargo/mutants.toml`'s `--all-features` campaign the sixteen `meter_wide` call deletions (rank-26) have no killer on record. Worth a `cargo mutants --list -f crates/before/src/version/rank/num.rs` by whoever may run it; the outcome decides between the two resolutions in rank-26.
8. `Ranked::decode` and the borsh `Ranked` impl recompute `version.rank()` to verify the wire prefix, so decoding a composite key costs a full rank fold. This is a trust-boundary check and I judge it justified; if an O(|key|) decode is ever wanted, the alternative is to trust the prefix under the model of record (authenticated honest peers) and document the key as producer-authenticated. I recommend keeping the check.
9. The 32-bit seam's memory peaks (encode of an at-capacity integral rank; the `plus_one`/`minus_one` crossings) are unpinned on wasm32 (rank-16). Is a memory-terminal pin in the existing `*_memory_terminal_traps` style acceptable there, or does the working-set ceiling need the copy deletions first?
10. Where should `M` be defined once (rank-31): a crate-level complexity-notation section the fuelscape include links, or emitted by build.rs into the include itself? I recommend the crate-level section, since `M` appears in prose outside the islands too.

## Dropped

- "M is about O(n log n)" holds only on 64-bit targets (structure [10]): refuted. dashu-int 0.5.0 gates NTT out only under `force_bits = "16"` or `target_pointer_width = "16"` (mul/mod.rs:27, 31-33; arch/mod.rs:9); wasm32 selects generic_32_bit, which ships its own ntt.rs with `Word = u32` and the same 4000-word threshold; dashu's source comments saying "16/32-bit" contradict its cfg, and the cfg is what compiles. Verified in the registry source.
- M is Karatsuba/Toom-3 at typical rank widths, so "about O(n log n)" is misleading (claims [46], one clause): not a defect. The sentence is an asymptotic statement and NTT is the asymptotic tier on every target but 16-bit; the power-law tiers below 4000 words are already the subject of integral.rs's module doc.
- Docs describe a fused rank-only encode (prose [11]), Ranked::encode_rank is Rank::encode by another name (correctness [29]), encode_rank documented as fused (claims [40]): duplicates of rank-32; [40]'s fuelscape `size_measure` site is folded in.
- "historical" comparisons (prose [14]) and dated/moralized rationale (claims [44]): duplicates of rank-24 and rank-13; [44]'s `Base::msb_cmp` note duplicates rank-25.
- Register tells (correctness [33]) and moralized "honest" (prose [15]): merged into rank-13.
- Grammar slips, misplaced size section, repeated sentence (structure [9]), decode's misnamed section (prose [19]), copyedit slips (prose [21]), wording slips (correctness [34]), prose nits (claims [46]): merged into rank-3, rank-9, rank-31, and rank-1.
- Hand-maintained measured ratios (correctness [32]): merged into rank-1. The `dsi-bitstream 0.10` item from [22] is dropped: it names the minor line the claim was evaluated against, which is provenance, not rot.
- expect("output fits") (prose [26], correctness [35]) and idiom nits (structure [8]): merged into rank-17; [8]'s digit-width item is rank-14.
- Long qualified paths (correctness [36], claims [48]): merged into rank-17.
- NotCanonical 2 EiB (claims [41]): duplicate of rank-10.
- Vocabulary collisions (prose [25]): the "class" collision spans two files with distinct qualifiers and is below the bar on its own; "door" and "two-ways pin" are folded into rank-13.
- "schoolbook long division" as internal vocabulary (part of prose [20]): dropped; it is an established term. The rest of [20] is rank-19.
- Structure [7]'s option "add an early return at the shift-by-zero and re-pin": struck. cfa7c7ed shows the shift's limb record feeds the board's `rank_encode` limb floor; rank-24 states the rationale at the site instead.
- Structure [1]'s implication that the wasm32 pins hold arm dispatch as a whole: reframed into rank-23 (the pins hold the load-bearing upper side; the parenthetical over-describes).

<!-- source: final/skyline-coding.md -->
# Partition skyline-coding: The skyline coding: module root, admit, build, encode/decode, emit, literal, validate, text, shape, walk, and the skyline tests

## Partition summary

The skyline coding is the stored and wire form of a `Version`: preorder topology flags interleaved with delta-coded absolute leaf heights, canonical by three conditions the module root states (minimal topology, natural heights, exactness). The partition holds the coding's kernels: `validate.rs` (the strict one-pass validator on two bits per open ancestor plus one cliff-free `Accumulator`), `admit.rs` (the span wire form's fused parse-and-dominance walk, `CheckedCursor` beside a `LeafCursor`), `build.rs` (the collapsing output builder every emitter drives: absorb, re-anchor, cascade, the held leaf), `emit.rs` (join, meet, and the fused hull over one merge sweep), `text.rs` (the paper notation rendered from and parsed into streams without materializing heights), `literal.rs` (the `TryFrom` composers), `shape.rs` (the refinement walks under the public step-function iterators), `walk.rs` (the single-stream leaf-walk driver and its block scans), plus the test-only `decode.rs` and `encode.rs` transcoders. Four test files (`build/tests.rs`, `emit/tests.rs`, `text/tests.rs`, `tests.rs`) hold the differential, exhaustive, mutation, and reject suites. I read all fifteen files in full at 9e5784fb, 5092 lines, and the callees and consumers each finding cites (the codec builder, the span wire decoder, the borsh cursor, the meter shapes, the board ceilings, the fuelscape roster).

The code is in good shape at the altitude that matters. Every kernel is short and single-purpose, every deep walk keeps its transient on bit stacks (nothing in the partition recurses on input depth), and the load-bearing disciplines are argued at the code and pinned by tests a reader can re-derive by hand: the absorb face of the builder, the reset-not-subtract move in the parser (held red by a committed known-bad twin), the height-subsumption argument in the admission walk (pinned relationally, with no constant to rot), and the reject corpus whose accepted mutants are re-derived through the oracle bridge, the one comparison a lax validator cannot pass. Every test carries an accurate doc comment and lives in a sibling file.

Two cost arguments are wrong, and both name an input the committed instruments never run. The builder's amortization prices each cascade copy against its own deletion but never prices the flush that re-writes a re-anchored wide code one level up, so `join` of a flat wide leaf against a left spine of right-child pairs does Θ(depth × width) work on Θ(depth + width) input, against the door's published `O(|self| + |other|)`; every envelope row and board cell drives the absorb face only. The render merge re-adds a wide `span` once per spine level, a quadratic leading term on `WideTail`, while the fuelscape contract says "superlinear, subquadratic" with no argument. Beside those: the admission walk's mid-stream collapsible-pair arm is a second implementation of the validator's check with no gate-tier witness through `Span::decode`; the two strict parsers duplicate their obligations; a testdoc cites a design essay the crate retired; two test-local recursions sit outside the recursion inventory the hard rule points at. The remainder is leftovers of the tree's evolution (a predicate that outlived its runtime gate, stacks left on the output buffer by a mechanical migration, dual entry names) and prose nits.


## Positives

- build.rs's absorb face is a genuinely elegant amortized-O(1) design (truncate one parent flag per level around a held code that never moves), argued at the code and pinned by hand-derivable bit-level tests per collapse genre; `ZERO_DELTA_CODE_BITS`'s doc names the one other 1-bit code and argues why it can never fire a collapse, and `zero_delta_has_the_lone_shortest_code` pins the cross-module coupling as a red test rather than a convention.
- text/tests.rs's `parse_schoolbook` is a committed known-bad kernel doing double duty: the independent second seat of the twin-parity differentials, and the demonstration that the wide-arming flatness criterion is non-vacuous (asserted red at ≥×1.5 per byte while the shipped kernel stays ≤×1.25). This is "every criterion needs a committed demonstration that a known-bad mechanism fails it", done properly.
- admit.rs states the height-subsumption argument once, cites it by name at every later mention, warns callers that `Refuted` doubles as a rejection surface, and its fused-parse claim is pinned relationally (fused == standalone-lo-parse + comparison on scan bits and touches), so there is no constant to rot. The advance restatement matches `overlay::advance` arm for arm, and its done-cursor argument holds.
- validate.rs is exactly its module doc's contract: two bits per open ancestor, one accumulator whose sign is read only after a subtracting fold, no panic reachable from bytes; tests.rs covers every reject genre with its exact variant, sweeps truncation at every cut point, and `assert_mutation_never_aliases` explains precisely why only an oracle re-derivation can convict a lax validator (decode adopts bytes verbatim).
- emit.rs makes join and meet differ in exactly one pick function, shares the switch algebra, and is witnessed three independent ways (oracle bytes, a three-cursor pointwise walk, the lattice laws on emitted streams) over families, the exhaustive small scope, arbitrary pairs, organic histories, and the 29..=34-bit grid straddling the fused-gamma guard.
- The gate runs the internal doc build (`just gate` → `gate-streams` → `docs-internal`, justfile:400-470), so emit.rs's `#![allow(rustdoc::private_intra_doc_links)]` is backed by a check that holds the private links against rot, as its comment claims; I verified this rather than taking the comment's word.
- Small comments that state what the code cannot show: decode.rs:16-17 (why one spare bit of capacity), validate.rs:96-98 (why the first leaf's `zero_delta` stays false), emit.rs:292-301 (why the capacity is an estimate and what bounds the miss), text.rs:404-412 (why `node_index` may abort), walk.rs:124-131 (the two reset policies and the cost inversion that follows from violating the provenance contract).
- Every test in the partition carries a doc comment stating its invariant, the comments read accurately against their bodies (with the three exceptions in skyline-coding-13), and tests live in sibling files throughout.

## Open questions for Finch

1. Re-anchor quadratic (skyline-coding-9): cure with the two-stream builder, or accept the class and restate the join/meet/span contracts? Recommendation: cure. `O(|self| + |other|)` for join is the crate's headline guarantee and lib.rs makes every such claim a hard promise; the two-stream design also dissolves the held-leaf discipline, `lens`, and `extract_code`. Land the two-scale scan-bit pin on `join(Hugeleaf, spine_of_pairs(d))` first, red, then the cure.
2. Render class (skyline-coding-29): cure the per-level wide-span re-fold, or restate the class? Recommendation: restate now (delete "subquadratic"/"n log n", state Θ(|self| + depth × max interior summary width + k log k), replace the fitted-exponent ceiling with the closed-form witness) and schedule the cure; the model is ratified and the instruments are committed, so the doc-and-instrument change is the cheap consistent step.
3. Validator unification (skyline-coding-33) and the fused-door planted-pair proptest (skyline-coding-6): unify on `CheckedCursor` and extend the family to both doors, accepting a heap-column re-pin? Recommendation: yes; land the proptest first (it is red-capable today against a weakened step arm and green against the shipped code), then unify.
4. The `display_growth` allocation arm (skyline-coding-28): retire with a DECIDED entry on 1300ced09's template, or record it as a standing re-runnable record at the check-cfg registration? Recommendation: retire; the question it answers is settled and asserted.
5. Test-suite inventories (skyline-coding-3): keep the inventory in the tests module doc and reduce the kernel doc to invariants plus a pointer? Recommendation: yes, matching `validation_index.rs`.
6. Vocabulary at crate root (skyline-coding-1): should "door" be defined once by contrast or replaced by "entry point" crate-wide, and should "min-lifted" (anchored only at literal.rs:29-31) be defined once at the crate root or the oracle module? Recommendation: replace "door" (the style guide's own translation table lists it), and anchor "min-lifted" once at the crate root since the meter and bridge modules use it too.
7. Cross-partition items surfaced here, for their owners: the `implementation` ghost at crates/before/AGENTS.md:6 and examples/code_study.rs:6; asymptotics.rs:45-46 quotes a rustdoc sentence ("summary-merge cost that grows faster than the operand") that grep finds nowhere else in the crate; overlay.rs:12-17's "exactly two generic faces"; span/tests.rs:338's collapsible witness carries no padding marker after its 5-bit tree and passes only because `NotCanonical` fires before `require_marker_padding`; the fuelscape roster's `version_display` contract strings; `tests/meter.rs:411`'s `version_of` copy.

## Dropped

- The cited `meter::tier2` plain-sweep pin does not exist ([35]): refuted. The pin exists at crates/before/src/meter/tier2/tests.rs:239 (`cliff_comb_plain_delta_sweep_is_quadratic_in_tier2_wire_bits`, cfg `limb-meter`, added 7c3677192 before either citation was written); the lens's regex could not match `plain_delta_sweep` and its tier2 grep read tier2.rs, not tier2/tests.rs. Both citations resolve. The residual (the demonstrator drives a standalone `Base` over the comb's deltas rather than the validator's code path) is a strengthening, not a gap, and below the bar.
- Production assert on the render's exact sizing ([41]): deliberate and documented (e83ef6008; text.rs:221 and 345-350), an O(1) probe of the kind the doctrine's Assertions and Guards clause explicitly admits, and the only exact pin of `exact` under the `display_growth` arm; whether a sizing slip should panic `Display` in release is a severity judgment for the owner, not a finding.
- Ghost `implementation` essay ([30], [42]): duplicates of skyline-coding-14.
- Nested literal construction re-scans every subtree per level ([37]): duplicate of skyline-coding-20.
- Test files spell `crate::codec::built_view` 69 times and triplicate `version_of` ([26]): duplicate of skyline-coding-8.
- "mint" at skyline.rs:7 (half of [15]): the coinage sense, permitted under the owner's dissolve/link/mint ruling (a736ef14a: "skyline.rs carries a plain gloss naming overlay as the minting site"); only admit.rs:256 survives as skyline-coding-7.
- Whether `just docs-internal` is in the gate (prose lens open question): resolved by reading justfile:400-470; it is, so emit.rs's allow comment is accurate and no finding follows.

<!-- source: final/skyline-fill-grow.md -->
# Partition skyline-fill-grow: The fused tick: fill (fuse, memo, prescan) and grow

## Partition summary

This partition is the `tick` kernel of `before`'s skyline coding: `fill.rs` runs one iterative walk that pairs the packed id (`IdReader`) against the event stream, decides in-pass whether `fill(id, e)` moves the tree (the changed flag, realized as the `Out` output mode in `fuse.rs`), and folds grow's `(expansions, depth)` route DP over the same nodes (`RouteProbe`); `grow.rs` replays the recorded `Route` in one splice when the flag stays clear, compounding `+k` events at the chosen leaf. Left-full shortcut sites need a minimum from a range the walk has not reached, so `prescan.rs` runs one memoized pre-scan per uncovered site over the pre-scan's own watermark web, recording each interior site's minimum as a ledger link in `memo.rs`. Both walks hold suspended ancestors as bits (`Frames`, `PreFrames`) with word deltas on a `PopStack`, so no input depth touches the call stack. `fill/tests.rs` and `grow/tests.rs` pin the whole thing against the recursive oracle (`event`, `fill`, `grow`), a brute-force minimal inflation, a reference recursive route probe, the exhaustive small scope, organic histories, `ticks(n)` against iterated ticks and a monoid-action law at `2^100`, and closed-form witnesses at depth 4096.

I read all seven partition files in full with line numbers (5754 lines: fill.rs 1312, fuse.rs 463, memo.rs 146, prescan.rs 695, fill/tests.rs 1964, grow.rs 692, grow/tests.rs 482; the two tests.rs files are the test surfaces) plus the callees and instruments the findings cite (idbits.rs, codec/stack.rs, codec/buf.rs, codec/base.rs, codec/build.rs, skyline/build.rs, signed.rs, watermark.rs, sweep.rs, walk.rs, party.rs, oracle/version.rs, meter.rs's memo generators, meter/board/ops.rs, meter/board/ceilings.rs, tests/meter.rs's envelope struct and `memo_resolution_cost` module, tools/citecheck, tools/covcheck-expected.json, .cargo/mutants.toml, lib.rs, version.rs, suanpan's `Accumulator`, and the pinned toolchain's `debug_assert!` source), and ran read-only git (`blame`, `log -S`) on the one prose-contradicts-code finding. I ran no cargo or just command; every finding below is assessed by reading or verified by grep, git, or hand trace, and none is settled by execution.

The kernel is in very good shape. Every `expect`/`unreachable!` message is a one-line proof and none carries an em-dash; the compile-time bindings (`OUT_FOLLOWER`/`REL_FOLLOWER` against `FOLLOWER_SLOTS`, `Cost::CEILING < Cost::INFEASIBLE`, the `Option<NonZeroU32>` niche) put layout claims under the compiler; the changed flag as an output mode and the `Step`/`Relation` enums put invariants in types rather than asserts; and the grow suite pins its grow-branch pair counts exactly so a rerouting regression cannot pass vacuously. No correctness defect surfaced. The dominant issues are structural duplication (the frame bit-stack type and the paired-walk skeleton spelled in both walks, `IdReader`'s cursor re-spelled in grow.rs, `fold_block` inlined in `consume_payload`), encapsulation (fill.rs drives `Memo` and `PreScan` through `pub(super)` fields, so the ledger's documented lifetime is enforced from another file), and prose that outlived its code (a `usize` width the walk no longer has, measured exponents no instrument holds, a `# Panics` form the crate superseded elsewhere, a "recursion argument" in an iterative walk). The one substantive claim finding is a space instrument gap: the distinct-minima memo forests hold on the order of a hundred heap bytes per input byte in suspended `Accumulator` structs and nonzero links, against a crate-level "small constant multiple" promise, and no committed meter reads heap on those families.


## Positives

- The changed flag realized as an output mode (`Out::Unstarted`/`Verbatim`/`Built`, fuse.rs:78-114): the unchanged branch does zero output work, the first divergence materializes the prefix once, and the absolute-vs-delta first-leaf decision is carried by the state itself rather than a side flag; the `large_enum_variant` allow at 85-88 states its cost argument at the site.
- `Cost` derives `Ord` with the field order spelling the lexicographic rule and the doc saying so (grow.rs:99-114); `CEILING < INFEASIBLE` is pinned by a const assert with the no-aliasing argument and the reason a runtime test cannot probe it (grow.rs:166-171).
- Layout claims under the compiler, not prose: `OUT_FOLLOWER`/`REL_FOLLOWER` bound to `watermark::FOLLOWER_SLOTS` with the cross-file roster named (fill.rs:182-188); the `Option<NonZeroU32>` niche behind the per-site cost claim (memo.rs:96-97).
- Types over asserts: `Step`'s absent variant is argued from the types ("the variant does not exist, rather than being asserted away", grow.rs:405-410); `Relation`'s tag/slot invariant is stated with the `expect` a violation trips (fill.rs:394-401); `Repair` names the boundary semantics instead of an `Option<&Base>`.
- grow/tests.rs pins its grow-branch pair counts exactly (`FAMILY_GROW_PAIRS = 182`, `EXHAUSTIVE_GROW_PAIRS = 114_621`, 255-290) with a failure message that tells the reader what to confirm before re-deriving: the liveness-floor discipline applied to a differential, and the sanctioned form of a hand-maintained count.
- Witness tests bind themselves to the arm they exist to drive through the emit-traffic decision counter (`dominated_undercut >= 1`, fill/tests.rs:467-471, 599-603) and say why `>=` rather than `=`.
- `materialize_is_a_noop_once_built` documents its internal entry as a deliberate decision at the check site in exactly the doctrine's form (fill/tests.rs:401-403).
- Both walks are iterative on bit stacks with `DeltaReg` word deltas on a `PopStack`, the transient cost stated where it lives (fill.rs:1142-1148, 1179-1184); the depth-4096 closed-form witnesses cover the memo, reveal, cliff, staircase, nested-full, mirror, and spike regimes with each derivation stated in the test doc (fill/tests.rs:986-1027).
- Every `expect`/`unreachable!`/`debug_assert!` message in the partition is a one-line proof and none carries an em-dash (verified by grep); the release `assert_ne!` in `expand_subtree` states in its `# Panics` why a fabricated cost would be worse than a panic.
- The vocabulary caution in `PreScan::run` (prescan.rs:142-146) naming the `level` vs `depth` collision between the twin walks is a model maintainer note.
- `grow::emit`'s loop-not-helper comment (grow.rs:528-533) steelmans its own shape by naming the six pieces of state a helper boundary would thread.
- The debug-only ledger check is O(1) state (the FNV-style `position_check` pairing recorded and consumed positions, memo.rs:99-103) rather than a position buffer that would bill the heap meter for debug scaffolding.
- Width tests kept off the limb meter with the reason at the site (fill.rs:248-254; grow.rs:503-508), so dev and release readings agree.

## Open questions for Finch

1. Heap on the distinct-minima memo families (skyline-fill-grow-2). Should `MemoChain(distinct)` and `MemoComb` join the board's tick group under the 16 B/B ceiling, or get a heap column in `memo_resolution_cost` with a declared model? Recommendation: measure first through `memo_resolution_cost` (cheapest instrument), then decide between a declared model and the word-compaction cure by the reading; I expect the reading to be red against 16 B/B and the cure to be the `Boundary::Word | Wide` trade `MinWeb` already makes.
2. The `u32` link index (skyline-fill-grow-23). Widen to `Option<NonZeroUsize>` (8 bytes per queue cell, no cap) or keep the 4-byte cell and state the cap in `fill::tick`'s and `Version::tick`'s `# Panics`? Recommendation: widen; it touches no public surface, the memo rows' heap columns re-pin with attribution, and the `expect` on the walk path disappears.
3. Kernel-doc citations (skyline-fill-grow-17). Extend `citecheck` to backticked test names in `src/version/skyline/**` comments, or reduce kernel docs to module-level pointers? Recommendation: extend citecheck; the citations are useful navigation and the tool already exists.
4. "mint" (skyline-fill-grow-21). Is watermark.rs's latent-register sense a term of art to keep (then link it at first use in fill.rs and fill/tests.rs) or to rename crate-wide? Recommendation: rename in watermark.rs ("a boundary moves into the latent register" already says it) and fix the three plain-sense uses here regardless.
5. The orbit bound (skyline-fill-grow-31). Is the intended lemma `4·⌈log2(k + 1)⌉` (the landing commit's and the spec's statement) or `4·bitlen(k + 1)` (what the code asserts)? Recommendation: try the tighter form once (the construction in the finding); if it passes, keep the doc and tighten the code.
6. The `# Panics` form (skyline-fill-grow-4). walk.rs's truncation/malformation-vs-silent form cites `causal_cmp` as the one statement; if you agree it is the crate's form, the sweep is crate-wide (the `EvScan` methods and any other kernel still carrying "Panics if ... not canonical"). I have not surveyed the other kernels.
7. `PreScan::max_range`'s arm-by-arm proof at the use site (dropped below). The 0f13590b ruling put the distilled call-chain proof at the use site; the doctrine's rule against hand-maintained caller enumerations says such proofs rot when an arm changes. I verified the chain still matches `run()`'s arms at HEAD. Does the ruling stand, or should the chain move to `prescan_raise_shapes`'s module doc, which already names itself the invariant's negative space? Recommendation: move it; the debug assert keeps the invariant and the test module keeps the proof.
8. Em-dashes in `//` comments (skyline-fill-grow-10). The rule is yours and postdates the prose; 374 sites crate-wide. One mechanical pass or leave the crate's comment style as is? Recommendation: one pass, since the rule's reason (terminal rendering) applies uniformly.
9. From the claims lens, not filed as a finding: the memo and width-circulation envelope modules discriminate linear from the refuted quadratic at ×2.5 per doubling (exponent ≤ 1.32), looser than the board's 1.15, and those families never see the board's exponent. Deliberate calibration to the named known-bad mechanism, or would you want them under the board's exponent too? Recommendation: leave as is unless the families join the board (question 1), at which point the board's exponent judges them anyway.
10. From the claims lens, not filed: `ticks`'s changed branch runs a full second fused walk over the filled output solely to record the grow route. The nested/mirror-wide rows show the second walk's touch cost is small on those shapes; a route-only pass is a workload-dependent trade. Is there a shape where the second walk's accumulator work is not small? Recommendation: no action unless such a shape is constructed.

## Dropped

- max_range's doc justifies its debug assert by enumerating call sites (candidate 23): deliberate and recorded (0f13590b, "the use site carries the distilled proof"), the chain verified accurate against `run()`'s arms at HEAD; reopening the ruling needs new evidence, so it is raised as open question 7 instead.
- "parks no wide quantity per open site" reads against SuspendedLevel's two accumulators (candidate 30): merged into skyline-fill-grow-2 as its documentation half.
- prescan.rs says depth stays usize (candidates 34, 40): duplicates of skyline-fill-grow-24.
- Module doc pins measured exponents / measured exponents as unbound snapshots (candidates 37, 41): duplicates of skyline-fill-grow-1 (41's extra site at fill/tests.rs:1302-1304 folded in).
- The memo's u32 link index (candidate 42): duplicate of skyline-fill-grow-23.
- fill::tick's Panics documents an empty-id case (candidate 43): duplicate of skyline-fill-grow-5.
- grow/tests.rs's pools are a strict subset of fill's (candidate 36): duplicate of skyline-fill-grow-38.
- Route::dirs as a test-only member to delete (half of candidate 12): refuted; fill/tests.rs:1443 needs the accessor and `fill::tests` cannot see `grow`'s private field, so the `#[cfg(test)]` getter is the narrowest exposure.
- "type boundary" misnames the decode check (half of candidate 25): refuted; `Party`'s only constructors route through `finish_id`/decode, so "the type boundary" is a reasonable name for that gate.
- The ceiling parameter as circular justification (candidate 8's framing): refuted; the seam is documented at the declaration and the check site, the doctrine's sanctioned form. The oracle wrapper's inconsistency survives as skyline-fill-grow-32.
- The fill module doc restates the memo discipline six times (candidate 22's framing): reframed; the memo appears once per cost axis, which the Cost structure requires. The `# Testing` duplication survives as skyline-fill-grow-3.
- The heap paragraph contradicts SuspendedLevel (candidate 39's doc framing): reframed; the sentences reconcile by scoping "its frames" to `PreFrames`. The instrument gap survives as skyline-fill-grow-2.
- The recursion-argument item of the texture sweep (part of candidate 27): not dropped but split out as skyline-fill-grow-7, since it is a factual ghost rather than register.

<!-- source: final/skyline-query.md -->
# Partition skyline-query: The linear functionals and projection: query, integral, web, and the query test suite

## Partition summary

This partition is the query surface of the skyline codec: `query.rs` (653 lines) holds the five folds (`rank`, `distance`, `lag`, `rank_cmp`, `min_ticks`, `project`) and the shared pair co-sweep `pair_fold`; `query/integral.rs` (1164 lines) is the anchored-segment integral every rank-family fold runs on (the `h* = B + P + L` height split, the freeze trigger, the promotion ledger, the mass-balanced product-tree settle, and the funding certificate); `query/web.rs` (422 lines) is the min_ticks fold's bookkeeping (reign records over the shared `watermark::MinWeb`, the epoch ledger settled by summation by parts, and the word-scale `mul_into` settle move); `query/tests.rs` (2704 lines) is the sibling test suite: differential pins against the recursive tree oracle, the composed kernels, and the semantic Riemann sum, plus a `limb-meter`-gated `adequacy` module of eight committed known-bad kernels rostered by `tests/superlinear_tripwires.rs`. Total read: 4943 lines across the four files, plus the supporting ranges the findings cite (`codec/base.rs`, `codec/bits.rs`, `codec/buf.rs`, `overlay.rs`, `signed.rs`, `rank.rs`, `int.rs`, `testing/generators.rs`, suanpan's `accumulator.rs`, `tools/covcheck-expected.json`, the justfile's coverage section, `tests/meter.rs`, the fuzzfit bands, the board family constants, `.cargo/mutants.toml`, and dashu-int 0.5.0's threshold constants). `tests.rs` is the only test file.

The production code is in good shape. One `Integrator` serves rank, distance, lag, and rank_cmp through a single `orientation: impl Fn(Ordering) -> i8` closure, each shipped closure a three-line transcription of one row of the module doc's sigma table; the settle tree runs on an explicit `Step::{Open, Merge}` stack and states why it is not routed through `crate::fold`; `web.rs` reuses `MinWeb<P>` at payload `Reign` rather than re-implementing the anchored-minimum web; every public fold carries a uniform `# Panics` section; every charge in both funding certificates names its deposit; the cfg-gated meter taps compile to nothing without `limb-meter` and are called unconditionally. I traced every `expect`, `unreachable!`, index, and integer conversion and every fold against the callee contracts; nothing panics on decoded input except the one `u32` epoch narrowing reported below, and no library walk recurses on depth.

The dominant issue is in the test suite: the adequacy module hand-copies the shipped `rank` loop four times, the `pair_fold` loop twice, `Integrator::finish` twice, and the `settle_armings` reduction twice, each documented as the shipped body "verbatim", and three commits on one day changed the shipped bodies without mirroring the copies, so every "verbatim" claim is false today. The remaining findings are smaller: a public `O(M(|v|) · log |v|)` clause with no committed instrument at the tier where it applies (disclosed in the doc itself), a `u32` epoch narrowing reachable by input size, several comments whose premise the codec now denies or whose rationale expired (the storage cap, the `scale` alias, the `arb_base` ceiling, the `one` field), a test whose doc claims a regime the body does not observe, and a set of vocabulary and hand-count items, several of which are slices of crate-wide patterns and belong to one crate-level ruling rather than to this partition's worklist.


## Positives

- `pair_fold`'s `orientation: impl Fn(Ordering) -> i8` is the right amount of abstraction: one merge walk, three measures, each closure a three-line transcription of one row of the module doc's sigma table (query.rs:76-80), monomorphized with no trait or enum machinery. The sigma table turns "orientation" into a checkable contract: `pair_fold`'s doc states its two clauses (query.rs:314-322) and `Integrator::jump`'s doc derives the always-debit property from them (integral.rs:818-824), so the assert message reads as a one-line proof.
- One `Integrator` serves rank, distance, lag, and rank_cmp; the single-stream rank is the two-ledger integral with `Φ_b` empty, so the shipped code has no per-measure kernel duplication.
- `settle_armings` satisfies the no-depth-recursion rule with an explicit `Step::{Open, Merge}` control stack that still reads like the recursion it replaces (integral.rs:1096-1129), and the doc says why it is not routed through `crate::fold` (an online entry-count balancer versus an offline mass balancer with a side-effecting combiner).
- `web.rs` reuses `watermark::MinWeb<P>` generically at payload `Reign` instead of re-implementing the anchored-minimum web; `ReignWeb` adds only the fold's own semantics, and `EpochLedger` isolates the frozen component's summation by parts in about 70 lines.
- The `frozen` gate on the segment and window feeds is derived on the field it gates (integral.rs:550-562) rather than asserted, and it is pinned from both sides by the `LoneFreeze` family (tests.rs:256-263), so the never-freezing regime pays nothing toward the settle machinery.
- The funding certificates are complete in the sense the doctrine asks: every charge in the integral (integral.rs:151-182) and the min_ticks web (web.rs:54-77) names its deposit, and the arity paragraph (integral.rs:137-149) states why a two-ledger potential is required for two-stream operations, naming the rejected composed form with its failure mode.
- Every public fold carries a uniform `# Panics` section deferring to `validate` for untrusted bytes; `project`'s omits the id operand, which the `Party` type already guarantees canonical.
- The test suite is unusually strong: three independent witnesses per fold (tree oracle, composed kernels, function-space Riemann sum), both operand orders, exhaustive small scope for singles and ordered pairs, constructed cancellation and zero-drift schedules with a `FREEZE_HITS` liveness floor on the cancellation family, eight value-exact known-bad kernels rostered by name in `tests/superlinear_tripwires.rs`, and a multiplication floor that is a reduction from arbitrary integer multiplication with the stored size pinned linear inside the same proptest (tests.rs:754-769).
- The two internal-entry pins (`settle_product_tap_is_alive_on_the_wide_arming_close`, `densify_tap_prices_the_cluster_span`) open with "Deliberate internal-entry pin, decided here" and derive their floors from a universal per-boundary premise (tests.rs:1001-1017); the floor arithmetic in prose matches the code, and each carries a value leg so a tap that recorded the right number while computing the wrong integer proves nothing.
- The scaled-read hazard from suanpan (a collapsing sign read lowering the watermark shift) is cited at both consuming sites by witness name (integral.rs:903-907, 953-956), the right altitude for a cross-crate invariant.
- `mass_split`'s contract is pinned size-generically against a naive recursive reference cross-checked against the shipped explicit-stack expansion (tests.rs:1327-1380), and the exponential/doubled/uniform points (tests.rs:1296-1325) document why the bound is denominated in mass; the stated masses and entry counts check arithmetically.
- The cfg-gated taps (`meter_product`, `meter_window_digits`, `meter_densified_image`) compile to nothing without `limb-meter` and are called unconditionally, so the algorithm has no feature forks; each tap's doc names the concrete dark-tap artifact it excludes and the committed pin that would fire.

## Open questions for Finch

1. Adequacy kernels (finding 31): share the drivers test-locally (a small trait plus `fold_single`/`fold_pair`/`settle_with`/`finish_with` in tests.rs) or keep independent copies and re-sync them by hand, striking "verbatim"? Recommendation: the test-local shared driver; it is the precedent 982bd260 set for `mass_split` after a copy masked a mutation, and it avoids adding production hooks whose only second implementor is a test.
2. `mul_into`'s zero guard (finding 21): replace the metered `*factor == Base::ZERO` with an O(1) `factor.bits() == 0`, drop it as `charge_digits` did, or keep it as ruled in aa7c96a0 and write the cost trade at the site? Recommendation: `bits() == 0`, then re-pin the min_ticks limb rows attributed to this change. Separately, the `subtract: bool` keep from 4ac8fd70 should be stated in one comment line at the signature.
3. The allocation A/B seam in `project` (query.rs:506-517, 603-613) and its `display_growth` twin: the arms are deliberate, registered under `deny(unexpected_cfgs)`, and reasoned inline, but no committed number records the pre-size ruling the shipped arm embodies (benches/presize.rs saves baselines locally). Recommendation: run the record protocol once, write the verdict (resident bytes and wall deltas) into the bench doc or an agent note, and either delete the alternative arms or add one sentence at each seam saying they are retained for re-measurement by design.
4. Crate-wide vocabulary and register (findings 1, 4, 12, 23): "seam" (176), "genre" (143), "honest" (101), "mint" (59), and em-dashes in `//` comments (374 lines) are crate dialect, and "honest" appears in your own recorded rulings. Recommendation: one crate-level pass with an owner-confirmed word list, rather than per-partition edits that leave the crate inconsistent; this partition's sites are listed under the findings.
5. The `O(M(|v|) · log |v|)` clause (finding 9): instrument it (a multi-scale fuel fit past 65 KiB, or a `meter_product` operand-width check on the doc's tight construction) or narrow the public contract to what the committed counters pin and move the quasilinear-tier remark to a decision record? Recommendation: the fuel fit, since the fuzzfit harness already prices `ff_version_rank` in the currency that counts the backend's work; failing that, narrow.
6. `EpochLedger::epoch`'s `u32` (finding 24): widen to `usize`, or rule a 2^41-bit operand infeasible and write the argument into the expect message? Recommendation: widen; the compactness saving on `Reign` is a few bytes per stacked boundary.
7. `promoting_pool`'s exclusivity claim ("these shapes are the only ones that arm it", tests.rs:241): unverified either way now that `arb_base` reaches 2^514. Recommendation: a `#[cfg(test)]` promotion tap over `family_pool` and one run of the arbitrary sweep to settle it, then either keep the sentence with the tap as its witness or drop it.
8. Measured brackets in the adequacy docs (finding 30): confirm that 500d4d09's pin-commit convention applies to query/tests.rs as it does to the meter surface. Recommendation: yes, move them.
9. dashu thresholds (finding 28): one named constant with the bump note, or leave the literals? Recommendation: the constant; a dashu bump is already declared a breaking change in suanpan's docs, so one re-verification site is the cheap discipline.

## Dropped

- Hard assert in `Integrator::jump` justified as catching a failure the differential suite already catches [27]: refuted; f77011e3 records the owner's direction that impossible states panic, the three shipped closures are all monotone so no committed test exercises a violation, and the assert is the only observer of the private closure contract; its adequacy gap survives as finding 17.
- query.rs module doc restates integral.rs's height-split paragraph and tests.rs's testing map "nearly verbatim" [22]: below the bar; the `# Testing` section is owner-kept (14259c1f, plan D6) and style-prescribed, and only one derivation sentence is shared between query.rs:50-52 and integral.rs:34-36.
- Bench-only allocation A/B arms compiled into `project` [10]: deliberate and documented inline (query.rs:506-512, Cargo.toml:98-108, justfile:792-806); the missing recorded verdict is open question 3.
- Folding `mul_into` into `charge_segment` (the maximum of [1]): deliberate and documented at web.rs:115-120 (a word-scale kernel for O(1)-dense operands); the remaining sub-points are finding 21.
- Adequacy demonstrators "verbatim" while diverging [26], [38], [45]: duplicates of finding 31.
- `Integrator::one` workaround for `add_u64_shl` [48]: duplicate of finding 15.
- "mint" [13], [35] (mint half): duplicates of finding 23.
- `let scale = max_depth` alias [29]: duplicate of finding 2.
- Hand-maintained counts [16], [36]: duplicates of finding 22.
- Long qualified paths [30], [41], [53]: duplicates of finding 7.
- Zero-drift freeze arm liveness [34]: duplicate of finding 27.
- `EpochLedger::epoch()` panics on an input-size bound [49]: duplicate of finding 24.
- Measured records in prose [50]: duplicate of finding 30.
- `mul_into`'s limb-metered zero guard [52] and its zero shift parameter [40]: merged into finding 21.
- "genre" as an undefined synonym [19]: merged into finding 1.
- Moralized "honest"/"real" [35] (that half): merged into finding 12.

<!-- source: final/skyline-sweep-place-masked.md -->
# Partition skyline-sweep-place-masked: Comparison kernels: sweep, place (and its filter), masked, overlay, signed

## Partition summary

This partition is the comparison layer of `before`'s skyline encoding. `overlay.rs` supplies the cursor vocabulary (`PlateauCursor`, `LeafCursor` over a skyline stream, `IdLeafCursor` over a packed id stream), the overlay-advance law in a binary form (`advance`) and an N-ary form (`advance_set` over a `CursorSet`), and the pair-difference algebra (`OpenedPair`, `Side`, `fold`, `advance_diff`). `sweep.rs` folds `sign(height_a - height_b)` once per elementary interval into a `Directions` pair and asks four questions of it through one generic loop. `masked.rs` generalizes that to up to four streams (two event streams and two optional id masks) with per-side height integrators and a block skip over unowned runs. `place.rs` fuses one probe against a span's two bounds with per-question verdict hooks, and `place/filter.rs` fuses one or two probes against any number of demand-carrying bounds for `causally`'s membership and coverage verdicts. `signed.rs` is the sign-magnitude substrate: the zigzag maps, the signed folds and sums, the fused gamma coder, and the signed comparisons.

The four `tests.rs` files (`sweep/tests.rs`, `place/tests.rs`, `masked/tests.rs`, `signed/tests.rs`) are test code. I read all ten partition files in full (4077 lines) and, to settle cross-file claims, `shape.rs`, `admit.rs`, `walk.rs`, `query.rs`, `codec/stack.rs`, `codec/base.rs`, `version/own.rs`, `version.rs` (the `PartialOrd` impls), `causally/query.rs`, `causally/conjunction.rs`, `causally/polarity.rs`, `causally/tests.rs`, `laws.rs`, `party/ops/diff.rs`, `party.rs`, `party/forks.rs`, `idbits.rs`, `src/meter.rs`, `tests/meter.rs` (the placement, early-exit, and masked-hole rows), `.cargo/mutants.toml`, and suanpan's `accumulator.rs` at the cited ranges. No cargo, just, or test command was run; every finding below is settled by reading, grep, git history, or a hand trace, and each says which.

The kernel layer is correct as far as every lens and this pass can trace it. `overlay.rs` states the advance law's correctness argument (nesting, the flip-level tie test, exhaustion by the all-right path) once, beside the mechanism, with a debug assertion at every tie; every `expect` and `unreachable!` in the partition is a one-line proof drawn from that argument; the verdict hooks act only on permanent refutations so every early exit is sound by construction; the fused gamma coder is pinned against the unfused composition at the `2^31` seam. What the review finds is mostly at the verification and claim layer. The one high finding is an asymptotic violation on a production path: `masked::Walk::block_skip` evaluates `LeafCursor::peek_flip` on every round for an unowned masked side even when that side is not the deepest slot and will not move, and `peek_flip` costs one word read per 64 bits of the path's trailing right-branch run, so a stationary deep cursor overlaid by a deep other-side subtree pays a quadratic term that no committed meter counts. Two medium claim and instrument gaps sit beside it: the filter walks' `O(|v| + Σ|bound|)` omits a factor of the bound count that the code plainly pays, and the coverage walk's three documented early exits, the production `order_exit`, and the placement walks' write-sequence identity each have no committed instrument that fails when the mechanism is removed. A medium prose defect is the ghost of a retired oracle: `sweep.rs` names `Version`'s `PartialOrd` as the verdict oracle, but that has been the sweep itself since the flag-day commit.

The remaining findings are the duplication the brief asked about (the pair seeding spelled at four production sites while `OpenedPair` claims one home; the arity-N advance law restated in `shape.rs`; two same-named `IdLeafCursor` structs) and a batch of nits on idiom, vocabulary, and copy-pasted rationale paragraphs. Several are claims that were true the day they were written and expired within days without re-denomination; the history pass documents that pattern and it is worth Finch's attention as a process observation.


## Positives

- overlay.rs:27-67 is a complete correctness argument stated once: the three dyadic facts (nesting, the flip-level tie test, the all-right path as exhaustion) are exactly what `advance`, `advance_set`, `LeafCursor::step`, and `IdLeafCursor::step` implement, the mixed done/live, tied, and dropped-slot cases included, and every `expect` and `unreachable!` in the partition (overlay.rs:448, 595, 605 are the models) is a one-line proof drawn from it. The tie debug-asserts are a legitimate O(1) probe of a structural property no differential test observes directly.
- `sweep::sweep` (254-288) carries each question as an exit predicate whose `Break` payload is the verdict, so three questions share one loop and no stale direction is handed back; `Directions::relation`, `order_exit`, and `eq_exit` are reused by masked.rs and emit.rs rather than re-spelled.
- The pair sweep's cost story is fully instrumented: `skyline_cmp_*` envelopes with zero-segment pins, two flatness bands with per-delta liveness floors, and `eq_early_exit` as an absolute two-scale pin with a stated mutation that fails it. `masked_cmp_hole_depth_band`'s `assert_eq!(lo, hi)` across a spine-depth doubling, with a shared ceiling and a ×0.75 liveness floor, is an instrument that pins the order, not just an envelope. The placement rows are stated relationally against the pair sweep, so no measured constant can rot.
- `advance`'s dual channel (fold callback in step order, positional return for re-coding) documents why both exist (overlay.rs:153-162), and every `CursorSet::priority` doc says which committed identity pins its order and which reorders move nothing.
- place.rs's `Fate` is a two-variant enum where a bool would have been the lazy choice; every finish arm debug-asserts the control-flow argument its hooks prove instead of re-deciding the verdict; place.rs:76-82 records the rejected alternative (a stream-level verdict vocabulary) with its reason.
- filter.rs:395-409 (the settle-order argument) explains not only why each guard exists but which agreement is coincidental, and the per-interval `debug_assert!` at 441-450 carries the argument as its message. `block_skip`'s justification (masked.rs:296-311) is stated in full and is correct.
- The Panics contracts are uniform across the partition and cite one home (`causal_cmp`); masked/tests.rs pins the contract's negative space (a validator-rejected witness sweeps without panicking) rather than an unspecified verdict.
- signed.rs derives `GAMMA_SMALL_MAG_BOUND` inline (212-214), keeps `Sign` two-valued with the zero convention stated once, and signed/tests.rs is a model differential: the fused coder against the unfused composition, a deterministic seam sweep at `2^31 ± 4` beside the proptest, and `signed_le`/`signed_max` against an exact `IBig` value order over both `Int` spellings and negative zeros. `fold_signed_int` dispatches word-scale deltas straight to `add_u64`/`sub_u64`.
- Every test in the partition has a doc comment stating its invariant, and the ones checked against their bodies (the flush-right tie geometry, the seam-magnitude strategy, the small signed grid, the filter witness sets) are accurate; `demand_lists` enumerates the demand kinds so every `Demand` arm is reached rather than sampled.
- Every meter row, law, and cross-module name the docs cite exists, and the committed signed seed shrinks to exactly `2^31`, the fast-path bound the tests claim to hunt.

## Open questions for Finch

1. Is the `meter`-feature `pub mod skyline` surface (and so `sweep::le`/`sweep::concurrent`) part of the stable public API? Finding 35 is owner-gated on that basis. Recommendation: treat it as instrument surface, narrow the two to `#[cfg(test)]`, and keep `eq` because a meter row calls it.
2. For the filter cost claim (finding 21): restate the bound with a k-scaling row, or restructure the read loop and the deepest-slot pick? Recommendation: restate now and add the two-scale k row; restructure only if rumors' classifiers are expected to conjoin many concurrent holes, since the dirty-flag gate plus a depth heap is real machinery the current small-k callers never exercise.
3. For the masked height debug-asserts (finding 3): delete them, or amend the Panics contract? Recommendation: delete; the asserts check an input property the validator owns, and the differential laws separate any fold-orientation bug.
4. For the masked-hole band (finding 4): is a nonzero-delta spine variant wanted as the fold floor? Recommendation: state the zero-delta premise now; add the variant only if the fold cost, not just the sign reads, is meant to be pinned.
5. The `Step` vs `Signed` split (overlay.rs:288-300): I read it as carrying a real invariant (a `Step` from `unzigzag` is always normalized; a `Signed` may carry a negative zero by documented slack, signed.rs:98-99), and a736ef14 #11 ruled on the distinction. Recommendation: leave it.
6. The claims lens notes that query.rs:533-549 (the skyline-query partition) uses the same peek-then-break shape inside the projection walk. If the outer loop advances the id cursor repeatedly while the event cursor stays put, finding 5's construction applies there too. Recommendation: hand to the query reviewer.

## Dropped

- LeafCursor and walk::LeafWalk share a descend/backtrack skeleton (structure [4]): deliberate and documented; 248d5539 ruled "no structural unification", the rationale is stated at walk.rs:8-12 and overlay.rs:313-316, and the finding brought no new evidence.
- The coverage emptiness/fullness conditions are tested only against transcriptions of themselves (correctness [29]): refuted; `coverage_is_exact_on_the_two_party_grid` (causally/tests.rs:222-229) is a two-sided brute-force membership census over every version, segment, hole spelling, and conjunction of the two-party grid, and `coverage_bounds_membership` (laws.rs:1029-1039) is the one-sided soundness law; the seeded mutation also separates from `composed_coverage`'s `!lt(lo)` at `lo == bound`.
- The pair-difference seeding re-spelled in place.rs and filter.rs (correctness [30]): duplicate of finding 15.
- sweep::le's opener overstates its role; le and concurrent are test-only (correctness [32]): duplicate of finding 35.
- filter cost claim drops the per-interval factor (claims [35]): duplicate of finding 21.
- sweep.rs names its own PartialOrd as the oracle (claims [38]): duplicate of finding 33.
- masked Panics contract vs debug_assert (prose [13], claims [39]): duplicates of finding 3.
- "Partial is called conservative" (part of prose [22]): refuted; `Query::coverage` routes filter's `Partial` through `refine_partial` ("the exact emptiness decision the fused endpoint fold cannot reach", causally/query.rs:143-144), so at the stream layer the verdict is conservative by design.
- Duplicated first sentence (part of correctness [31]): merged into finding 26.
- laws.rs:1036-1038's reference to a `Coverage` precision contract that the `Coverage` rustdoc does not carry (raised as new by the refutation pass): out of scope for this partition; hand to the causally/laws sweep.

<!-- source: final/skyline-watermark.md -->
# Partition skyline-watermark: The watermark (min-ticks web) and the traffic modules

## Partition summary

`watermark.rs` holds `MinWeb<P>`, the anchored-minimum web both skyline sweeps share: one signed accumulator `gap = h − A`, an optional latent boundary `Λ = A − m`, a `Vec<Entry<P>>` difference stack with zero runs compressed and (per instantiation) word-scale boundaries compacted, two follower slots with one-bit anchor-relative tags, and an accumulator pool. The generic core (arming, close/park, undercut/propagate, latent resolution) is written once; `impl MinWeb<()>` adds the fill walk's emission vocabulary (`emit_here`, `emit_offset`, `compare_above`, the bridges, the follower slots), and the min-ticks client rides `P = Reign` through a lazy `FnOnce` payload constructor, an `FnMut` death hook, and the total `Close<P>` enum. `pool_traffic.rs` and `web_traffic.rs` are feature-gated relaxed-atomic counters: one counts pool misses (the one observable that separates a live recycle from a dead one), the other classifies the fill walk's post-sign domination decisions (the one observable that proves the dominated-undercut arm still fires). `watermark/tests.rs` is a declared internal-entry suite: four worked pins on the latent ladder's gates plus three proptest families.

The kernel is sound. Every arm's value flow was traced by the correctness and claims lenses against suanpan's `Accumulator` contract and I re-traced the arms the surviving findings rest on (the three arming paths into `push_boundary`, `close`/`park`, `undercuts_here`/`decide_undercut_through_latent`, `undercut`/`drop_below`/`propagate`, both `compare_above` readers, the `emit_offset` ladder). No traversal recurses; the difference stack is a `Vec`; every count is a `u64`; every `debug_assert`/`expect`/`unreachable!` message is a one-line proof of why its branch is programmer error; every cost claim in the module doc names a committed instrument, and each name resolves. The payload seam is a model of a zero-cost generic. The two counter modules justify their existence in writing by naming what no other meter can see.

The dominant issues are structural duplication inside `watermark.rs` and two inaccurate test-side claims. The latent-ladder decision is written three times, the undercut tail four times (once with `drop_below`'s follower loop hand-inlined at exactly the site where the residue-polarity bug of d7293057 lived), the first-arming preamble twice, and `propagate`'s domination guard twice with the operands swapped — the last costing a line-and-column-pinned mutant exclusion that git history shows re-pinned four times in eight days. On the verification side, the test module's doc says `drop_below`'s latent annihilation is unreachable from any packed stream, but the min-ticks client reaches it directly (`ReignWeb::leaf` calls `undercuts_here` then `undercut` with no `compare_above` in front), and a proptest's closing comment describes a follower and a parked boundary the test never builds while the one arm it names (`drop_below` with a live latent and a live follower) runs in no test. The rest are documentation-altitude and vocabulary items, most of which predate the rules they now breach, plus one owner decision about which feature gate the two watermark counters share.

Lines read: watermark.rs 1181, watermark/tests.rs 431 (the test file), pool_traffic.rs 60, web_traffic.rs 113 (1785 in the partition), plus the cross-referenced ranges of `tests/meter.rs`, `src/meter.rs`, `fill.rs`, `fill/prescan.rs`, `fill/tests.rs`, `query.rs`, `query/web.rs`, `signed.rs`, `codec/base.rs`, `hull_traffic.rs`, `codec/scan.rs`, `crates/suanpan/src/accumulator.rs`, `.cargo/mutants.toml`, `tools/covcheck-expected.json`, `tools/mutantcheck-expected.json`, `Cargo.toml`, `rust-toolchain.toml`, and the commit messages and blame the history pass cites. No cargo, just, test, or state-changing git command was run; every "verified" below means read, grepped, or traced by hand against the source at 9e5784fb.


## Positives

- The payload seam is a model of a zero-cost generic: `payload: impl FnOnce() -> P` constructs lazily only on the arms that store or kill a boundary (655, 664), `on_die: impl FnMut(P)` fires at exactly the difference's death (726, 732, 780, 826, 834), `Close<P>` makes the min-ticks dispatch total (query/web.rs:259-279 matches all three arms), and the fill client at `P = ()` spells `|| (), |()| ()` and pays nothing.
- Every cost claim in the module doc names a committed instrument and each name resolves: `skyline_min_ticks_latent_ladder_is_flat_per_unit` (tests/meter.rs:3449) for the O(1) latent decision, the `skyline_min_ticks_seam_*` bands for `propagate`'s wide hops, `pool_recycle` (9382) for the pool claim, `dominated_undercut_cost` (9209) for the arm-liveness floor. The suanpan witnesses are cited by stable test name with the assumed clause restated inline (488-494, 751-771), which is the citation discipline the hard rules ask for.
- The latent-ladder band (tests/meter.rs:3434-3490) is a genuine order pin, not an envelope: a per-decision `k`-marginal compared across a doubling of the parked latent's width in both directions, with a floor derived from three irreducible register folds.
- Every `debug_assert!`, `expect`, and `unreachable!` message in the partition is a one-line proof of why its branch is programmer error (318, 322, 326, 339, 370, 390, 422, 535, 785, 805, 857), and both `unreachable!` arms are rostered in tools/covcheck-expected.json (146-157) with the positivity argument rather than excluded.
- `propagate` states its loop invariant and the deferred-zeros flush in comments a reader can check against every `break`/`continue` (697-706), and `compact` refuses to normalize a wide difference just to learn it will not fit (855).
- `pool_traffic.rs:4-18` passes the circular-justification test in writing: it names the property no other meter can see (a dead recycle leaves peak heap and every touch reading byte-identical) and the row that pins both directions; the row's ceiling `SEAM_STOP_POOL_WARMUP = 2` is derived from peak simultaneous demand (tests/meter.rs:9399-9411), not measured, and .cargo/mutants.toml:65-70 records that the row, not an exclusion, kills the retire/lease deletion mutants.
- `watermark/tests.rs` declares itself an internal-entry suite at its head with a reachability argument, pairs each worked pin with the proptest family it is a point of (309-322), and each worked pin's doc states the exact displacement a polarity error would produce (72-73, 117-118, 177-178), so the three-point anchor probe is demonstrably sensitive in both directions.
- `MinWeb::new` and `MinWeb::compacting` each carry the basis for their compaction choice with the row named, so `compact_words` is a measured decision rather than a bare bool (finding 8 is about the form of that record, not its existence).

## Open questions for Finch

1. Fill-side unreachability of a live-latent undercut is prose only. tests.rs:4-10 argues it from "only behind `compare_above`"; what I could verify is a different, structural argument: every fill-side child, leaf included, gets its own `web.open(1)` (fill.rs:505, 535, 544, 602, 608; prescan.rs:267) and `copy_subtree`/`copy_range`'s per-leaf loops (fill.rs:1071-1073, prescan.rs:485-488) close nothing inside, so a post-park emission always arms and `push_boundary` consumes the latent by merge. I did not exhaust every emission site. Recommendation: when re-stating the module doc per finding 21, write the structural argument and, if you want it mechanical, a `debug_assert!(self.latent.is_none() || self.pending > 0)` at the top of `MinWeb<()>::emit_here`/`emit_offset`'s non-arming path would turn the claim into a checked one for the fill client (min_ticks does not go through `MinWeb<()>`).

2. The pool floor `small >= 1` (tests/meter.rs:9444-9448, "the fill phase always misses") rests on the first arming leasing before it retires the initial zero `gap` (569-571). On the seam-stop family at least one miss is irreducible regardless of order (gap plus one stacked boundary exceed the constructor's single buffer), so the floor's premise holds for that family, but it is not the premise the message states. Recommendation: reword the floor's message and doc to "the family's peak demand exceeds the constructor's one buffer", which survives the lease-order unification in finding 18 (envelopes partition).

3. `decide_undercut_through_latent` (485-506) reads domination with no clearance guard while `propagate` (772, 790) guards at `+ 2` digits; `a_drop_short_of_the_latent_minimum_refuses_the_undercut` (D = 2^36, drop 50) shows the unguarded read is load-bearing there (a register-held latent certifies at one digit of clearance). The two ladders are correctly different, but nothing at either site says why the other's guard would be wrong here. Recommendation: one cross-referencing sentence at 495-497; moot for `propagate` if finding 14's primary resolution lands.

4. By-value `Accumulator` moves: `Boundary::Wide(Accumulator)` is held by value, so every `Entry<P>` is on the order of 100 bytes and every `mem::replace`/`take` of `gap`, every `lease`/`retire`, and every `Entry` push/pop copies that much. Whether `Box<Accumulator>` (pointer-sized moves, one indirection per fold) wins is workload-dependent and unmeasured; `size_of::<Entry<Reign>>()` is estimated from field layout, not measured. Recommendation: not a finding; if you want to know, print the two sizes in a test and judge a prototype on the ascend row's heap ceiling and the tick/min_ticks bench cells on a quiet machine. Nothing lands on anticipated benefit.

5. The module doc and both constructors name the two clients by module and describe their behavior (8-9, 93-95, 207-209, 222-227, 245-256, 312-314). The module's thesis is that "each client contributes only its own semantics through the payload seam". Recommendation: keep the client names (a reader needs to know who drives the web) but move the client-specific measured bases to the clients' construction sites (fill.rs:303, query/web.rs:231), where the row a client's choice rests on is beside the choice; this also resolves finding 8's placement question.

6. Out of partition, with dispositions: hull_traffic.rs:16-17 carries the same wrong-gate idiom citation as web_traffic.rs:19-20 (whoever owns version/hull_traffic.rs); query/web.rs names a constructor `Reign::mint` and uses 'mint' eight times, inside writing-style.md:170's "in code" clause (skyline-query partition); 're-arm' is also the promotion re-arm meter family's name (src/meter.rs:1683-1802), a third sense colliding with finding 25.

## Dropped

- [21], [27], [38] (emit_here/emit_offset re-implement undercut): duplicates of skyline-watermark-18.
- [25], [37] (the proptest comment describes a follower and parked boundary): duplicates of skyline-watermark-24.
- [14], [36] (moralized 'honest', 'mint'): merged into skyline-watermark-1.
- [28] (compact's width claim, tight figure): merged into skyline-watermark-15 with its figure.
- [29], [34] (compacting ratios; mechanism): merged into skyline-watermark-8.
- [40] (hand-counted 'two'; arm_below fold attribution; 'fill phase'): split into skyline-watermark-4, -5, and -25.
- [2] (propagate's mirror-image guards): merged into skyline-watermark-14 as the fallback resolution.
- [41] (pool_traffic should be gated on `meter`): refuted; Cargo.toml:74-84 defines `limb-meter` by frequency class, not by a dependency on `suanpan/touch-meter`, so the "needs only `meter`" premise misreads the gate. Its capability point (a per-miss bump is rare) survives as one side of skyline-watermark-27's owner decision.
- [39] (Box<Accumulator>): below the bar as a finding; nothing verified and the sign is workload-dependent. Moved to open question 4.
- [19]'s second half (the `memo modules` citation): weak on its own (an informal descriptor, not a wrong identifier); folded into skyline-watermark-8 as a one-word correction.

<!-- source: final/span-causally.md -->
# Partition span-causally: Span (algebra, owned spans, verdicts, wire) and the causally query language

## Partition summary

`Span` is an ordered pair of `Version`s (`lo <= hi`) stored as two `Cow<Version>`
endpoints. `span.rs` carries the type, the validating constructor, the four
verdict methods (`place`, `dominance`, `precedence`, `contains`), and the
clone-identity fast paths that answer a coincident span (both endpoints one
shared buffer) with one pair sweep instead of the fused three-stream walk in
`version/skyline/place.rs`. `span/algebra.rs` gives the four binary operators
(`+` union, `*` intersect, `|` pointwise join, `&` pointwise meet) as borrowed
kernels plus a receiver-seeded n-ary fold over `fold::balanced_reduce`, and
generates the owned/borrowed operator matrices by macro. `span/own.rs` is the
lazy per-party projection view, answering verdicts by two masked comparisons.
`span/verdict.rs` is the verdict vocabulary. `span/wire.rs` is the concatenated
canonical encoding and a strict decode whose admission walk parses the second
component while proving dominance. `causally.rs` and its submodules are the
polarity-restricted query language: `Query<P>` is an optional floor and
ceiling plus a hole antichain, with `Down`/`Up`/`Neutral` dispatch sealed in
`polarity.rs`, construction in `forms.rs` and `convert.rs`, the same-polarity
merge in `conjunction.rs`, and the two verdicts (`contains`, `coverage`) in
`query.rs` compiling bounds into `filter::admits`/`filter::coverage` demands,
with a clamp refinement behind the `Partial` arm.

I read all thirteen partition files in full (5213 lines; `span/tests.rs` and
`causally/tests.rs` are the test files) and the out-of-partition code every
finding leans on: `version.rs` (the version folds, `DedupRuns`, `span_refs`,
`encode`, the `Div` comment), `version/skyline/sweep.rs`, `place.rs`,
`place/filter.rs`, `fold.rs`, `laws.rs`, `codec/bits.rs`, `codec/scan.rs`,
`error.rs`, the board tiling and cells, the fuelscape roster, the meter's
placement module, `tests/coincident_span.rs`, and the git history the history
pass cited. I ran no cargo, just, test, or bench command; every anchor below is
by reading, and every count is by grep.

The kernels are correct on every arm I traced, and the verification around
them is unusually strong: the wire decode is a strict door with a single
allocation and genre precedence pinned on multiply-defective composites; the
coincident fast paths derive only what `Bits::ptr_eq` licenses and are held to
the fused walk both for verdict agreement and for liveness; the two-party grid
census makes `Coverage` exact rather than merely sound; and the polarity
dispatch concentrates the whole `Down`/`Up` difference in one sealed table
whose `Neutral` `unreachable!` arms carry a proof a reader can check inside the
module. The dominant issue is a cost claim: `Query::coverage`'s `Partial`
refinement sweeps the clamped endpoint once per surviving hole, so a query
with k pairwise-concurrent holes against a wide span costs Θ(k·|span|) against
a published `O(|self| + |span|)`, and no committed instrument builds a query
with more than one hole. Two structural issues follow it: the span operator
table is transcribed twice (binary kernels and fold constants), and the
two-sided receiver-seeded fold is written in both `algebra.rs` and
`version.rs`. The remainder is prose: a cluster of dangling pointers and
inverted arguments that the history pass traced to four terse owner hand-edit
commits (a6dcfbb4, b3f09baa, 20c0515a, bbb9f802), banned and undefined
vocabulary, and typos in public rustdoc.


## Positives

- `polarity.rs` concentrates the entire `Down`/`Up` behavioral difference in one sealed dispatch table (lines 36-58), so the two polarities differ in exactly six one-line methods, and the `Neutral` `unreachable!` arms (174-204) carry a one-line proof a reader can check inside the module: every `Query<Neutral>` construction site (convert.rs 15-20, 27-32; conjunction.rs 110-115, 121-126; query.rs 64-69, 202-207) writes `holes: Vec::new()`, and `and` never changes `P`.
- `Span::decode` (wire.rs 116-178) validates both components against the borrowed read buffer, adopts slices of that one allocation for both endpoints, dedups the coincident span's storage on `Admission::Equal`, and pronounces the pair verdict after the padding check so structural defects win; `span_decode_structural_genres_outrank_the_pair_verdict` and its coincident twin pin exactly that precedence with byte-level witnesses whose comments derive each byte, and `span_single_bit_mutations_never_alias` re-derives an accepted mutant through the oracle bridge into a fresh composite so the accept side cannot be gamed by an admission walk that accepts a non-canonical spelling.
- The coincident-span certificate is one idea applied uniformly and held live: `is_coincident`'s doc (span.rs 546-552) states what `ptr_eq` proves and its limit (byte-equal endpoints in distinct buffers take the general walk), `coincident_span_rungs_agree_across_buffer_identity` (span/tests.rs 812-844) holds every fast rung to the fused walk across buffer identity, and `tests/coincident_span.rs` pins the rungs' liveness by scan parity, so a lost rung and a dead meter both read red. Every rung derives only byte equality from `ptr_eq`, which is exactly what `Bits::ptr_eq`'s doc (bits.rs 205-216) licenses.
- `coverage_is_exact_on_the_two_party_grid` (causally/tests.rs 222-323) is a total oracle: the grid is the complete two-party interval, closed under join and meet, so the brute-force census over every ordered segment and a query family spanning both polarities, every hole spelling, and two-hole antichains leaves no corner; together with `refine_partial`'s one-polarity exactness argument (query.rs 143-151, correct and tight) it makes `Coverage` exact, not merely sound.
- `span/algebra.rs`'s module doc (lines 10-33) states the four operators as one four-row leg table with each totality argument written once beside it, and the refusal of `Into<Span>` for the partial operator (lines 43-48) names the silent-vanish failure it prevents; `span_version_lhs_matrix!`'s doc (665-675) gives the coherence reason its cells are concrete rather than generic.
- `fold_endpoints`'s total `(Input, Merged)` arm (algebra.rs 400-410) documents why an unreachable case stays total instead of asserting, and the claim checks out against fold.rs's weight discipline: the closing drain only ever combines `(Merged, Merged)` or `(Merged, Input)`.
- `Placement`'s variant docs (verdict.rs 26-48) derive the forced relation behind each `Concurrent` payload rather than naming it; `span_place_places_every_witness` hits all nine, and the three coarsening tests enumerate each fiber's members.
- The meter's `placement` module (tests/meter.rs 9463-9530) states every fusion's cost relationally against its own composition (fused = composed minus one probe scan), so there is no pinned constant to rot.
- `conjunction.rs`'s `and` doc (20-37) states the normal-form invariant (a pairwise-unabsorbed antichain), why only cross pairs are probed, and the deliberate non-decision (comparative pruning, never semantic emptiness), with its consequence tested in `degenerate_holes_are_inert`.
- Every test in both `tests.rs` files carries a doc comment stating its invariant, and I found none misstating its body; the point-witness tests name the quantified laws they instantiate (`span_gate_admits_exactly_the_ordered`, `degenerate_span_place_is_partial_cmp`, `own_span_matches_the_projected_span`, `span_encoding_is_prefix_free`), all of which exist in `laws.rs`.
- Every `expect`/`unreachable!` message in the partition is a one-line proof (wire.rs 146, 172; algebra.rs 415; polarity.rs 178-202).

## Open questions for Finch

1. Multi-hole cost contract (span-causally-24, -36). Do you want to keep `O(|self| + |span|)`/`O(|self| + |version|)` as the promise for multi-hole queries, or restate with the hole-count factor? Recommendation: fuse `refine_partial` regardless (a fixed-sign deletion of k-1 endpoint decodes), then restate "linear time" as "linear in bits decoded, with per-interval work proportional to live holes", since the polarity restriction's real payoff is a polynomial exact decision, not linear; pin the k axis in the touch currency, not scan bits.
2. `sweep::le` in production (span-causally-26). Was the cfg gate meant to keep one comparison entry point for the differential suite to pin? Recommendation: lift it and add `le`/`lt` to the pinned surface; the one-direction exit is what the fused walks already advertise.
3. `Version::span_all`'s `I::Item: Borrow<Version>` convention (span-causally-9). The wrapper-newtype route keeps that stable signature; changing the convention to `Into<Span>` (an API change) would make `span_all` `Span::at(self).union_all(iter)` outright. Recommendation: the newtype; no API change.
4. Owner-authored prose cluster. The history pass traces the no-`Eq` pointer, the deleted `Coverage` exactness sentence, the inverted SAT direction and its register, "arbitary"/"intractible", the dropped "per stored bound", `after(p)`, the identity rationales, "mint", the uniform `conjoin!` doc, and "of a [`Span`]s" to your hand-edit commits a6dcfbb4, b3f09baa, 20c0515a, and bbb9f802; the db9dfa3e originals are quoted in span-causally-34 and -38 as restoration candidates. Recommendation: re-rule on these as a batch rather than treating them as agent drift.
5. "door" (208 crate-wide uses, none definitional) and "genre" (210 uses, defined only in a private codec doc). Recommendation: retire "door" for the plain noun, as 22cdfbe1 already did in algebra.rs; keep "genre" out of public docs (span-causally-22) and either define it once in `error.rs` or leave it private.
6. `core::` versus `std::` (32 files import `core::cmp::Ordering`, 15 `std::cmp::Ordering`; the crate is not `no_std`). Recommendation: pick one crate-wide (std, since `core::` carries no meaning here) in a mechanical sweep.
7. Em-dashes in `//` comments (374 lines in 76 files crate-wide). Recommendation: a mechanical sweep to colons in a dedicated prose commit; the fifteen partition sites are listed in span-causally-2.
8. `Span: Hash` (span-causally-1). Recommendation: add it; it is additive, consistent with every verdict type, and rests on the same byte equality as `Eq`.
9. The board's proxy pricing of the span folds (span-causally-8). Was mapping `Span::union_all` to `version_span_all` a ruling that `fold_endpoints` adds nothing, or a tiling convenience? Recommendation: if span-causally-9 lands, the proxy becomes sound and only the space argument is owed; otherwise add a span fold cell.
10. `OwnSpan`'s composed verdicts (span-causally-14). Do you want a fused masked three-stream placement kernel? Recommendation: state the composition as the design for now; the kernel is a design proposal whose payoff is one probe-plus-id decode per verdict.

## Dropped

- [13] `Query`'s manual `Clone` could be a derive: refuted as a drop-in (`From<&Query<P>> for Query<P>` under `P: Polarity` alone calls `clone()`, so a derive needs `Polarity: Clone` as a supertrait, a public-trait change); the marker derives are vacuous on uninhabited enums and harmless; below the bar.
- [29] "Union: meets meet, joins join" trades legibility for a pun: taste without a named cost beyond the module-doc table already stating the legs plainly; below the bar.
- [16]'s construction (uniform-height `B`): refuted (a version where every party ticked once normalizes to a single leaf, so each `le(B, v_i)` is O(log k)); the finding itself is merged into span-causally-36 with [40]'s construction.
- [35], [40]: duplicates of span-causally-36.
- [8], [21] (`Eq` half), [36] (`Eq` half), [47]: duplicates of span-causally-38.
- [36] (second half), [48]: duplicates of span-causally-34.
- [21] (monotonicity half), [38] item 6: merged into span-causally-16.
- [10], [20], [26], [38] items 1-4, [51]: merged into span-causally-5.
- [9], [23]: merged into span-causally-4.
- [11], [34]: merged into span-causally-20.
- [7], [49]: merged into span-causally-21.
- [18], [38] item 5, [46] (clone-discipline half): merged into span-causally-13; [46] (intersect parenthetical): merged into span-causally-11.
- [22], [43] (`into_owned` half): span-causally-37; [43] (module summary half): span-causally-25.
- [44]: reframed into span-causally-28 (the fuelscape is audit-only; the enforcement gap is the finding).
- [50]: converted per the history verdict into span-causally-14 (deliberate design, rationale only in history).
- [15]: reframed into span-causally-10 (the `core::`/`std::` mix is crate-wide; the partition-local defect is the inline qualified paths).
- Refutation new item 1 (fuelscape audit-only): folded into the acceptance clauses of span-causally-8, -28, -36 rather than a finding of its own.
- Refutation new items 2 and 3 (coincident-receiver span-argument equality; causally.rs:106 "stops as soon as its verdict is decided"): folded into span-causally-26.

<!-- source: final/suanpan-tests.md -->
# Partition suanpan-tests: suanpan test suites: differential, ledger, metered, witnesses, amortized sequences

Reviewed at commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764 (working tree clean).

## Partition summary

The partition is the whole test surface of `suanpan`'s `Accumulator`: a shared harness (`crates/suanpan/src/accumulator/tests.rs`, 116 lines) and four sibling suites under `tests/` (`differential.rs` 1051, `ledger.rs` 297, `metered.rs` 890, `witnesses.rs` 652), plus one integration binary (`crates/suanpan/tests/amortized_sequences.rs`, 78). Every file in the partition is a test file; 3084 lines read in full with line numbers, together with the production code each traced count rests on (`accumulator.rs`, `touch_meter.rs`, `limbs.rs`, `magnitude.rs`, `claims.rs`, `claims/tests.rs`), the two proptest seed files, the sibling `before` band harness, and the refutation pass's run log.

The structure is sound. The harness gives every suite one mode-forcing constructor (`fresh`), one total value check (`assert_value`, which drives all three read-outs against an exact `IBig` oracle on every call), and one zone-edge construction (`park_extreme_negative_digit`). `differential.rs` runs randomized streams with a biased generator and pins the deterministic adversarial shapes the crate docs name; `ledger.rs` holds the zero-run ledger's full letter invariant on a ~1.95M-state exhaustive prefix tree and a deep-shift proptest; `metered.rs` pins exact digit-touch totals for the claims roster's rows, most of them repeated across an axis doubling, and commits one executable known-bad mechanism; `witnesses.rs` pins the decision constants at their tight edges, each with the surviving mutation it was built to kill recorded; `amortized_sequences.rs` is the one instrument that flips the sign under load. I re-derived the exact pins the lenses traced (alternating pair 5 and 3, settlement 16, certified-run skip 6, u64 comb 6n+1, domination 5 then 1, no-collapse model 2k/32+3) against `accumulator.rs` and they agree; the refutation pass executed the sign-flip test once and its output (`grid [32770, 65538, 32770, 65538]`) confirms a shift-independent `16n + 2` total.

Three findings are worth scheduling. `size_probe_covers_the_value` asserts a whole digit more slack than its doc claims, so the probe alone admits a `digit_count` that under-reports by one digit (other tests catch that mutant; this one does not test its own sentence). `merge_into_wider`'s swap, the `min` in its cost row, is never on any metered path: delete it and every committed test stays green. `assert_no_product` in `amortized_sequences.rs` bounds a relative quantity with no liveness floor, so a counter that records nothing passes it vacuously, while an exact pin is available and now measured. Below those: the ledger checker never observes states left by the fold, merge, shift, reset, or negate entry points; the shared helper's stated reason for its two-deposit shape is false against `LAZY_LIMIT`; three inequality pins sit in a module whose doc says every pin is exact; and a cluster of prose and idiom nits (hand-maintained tallies and a wall-time figure in a testdoc, two fragments left by a dated-note excision, a `u8` alphabet with a hand-counted cardinality, copied replay loops and run-forming arms, workspace-wide vocabulary and dash conventions) that a maintainer would batch.


## Positives

- `assert_value` (tests.rs:58-92) drives every value check through all three read-outs (`sign_magnitude`, `sign_limbs` with the minimality clause, `sign_magnitude_shl` reconstructed at scale) against an exact `IBig` oracle that shares no code with the fold or the carry pass; one small helper makes the read surface total at every value check in every suite.
- `floor_domination_is_sound` (differential.rs:701-789) pins the register arm's contract as exact (`decided ⇔ |v| ≥ 3·2^(32·(floor+1))`) rather than merely sound, carries a point mass at the extremal lazy-zone spelling (`all_extreme`), and its strategy docs (:177-184) say where the mass is concentrated and why.
- `ledger.rs` states its alphabet as structure: `ledger_op`'s doc (:115-125) justifies each op by the ledger transition it reaches within a short schedule, and `assert_ledger_invariants` (:21-92) holds the full letter of the invariant with a named failure message per clause, on a ~1.95M-state exhaustive prefix tree.
- `no_collapse_fold_re_scans_the_prefix` (metered.rs:831-890) commits the known-bad mechanism as an executable model and shows the criterion reads red on it at two widths; `merge_tie_reads_the_operand` (:348-393) makes the tie routing observable by giving the operands different nonzero populations; `alternating_shifted_writes_cost_the_operand_not_the_gap`'s doc (:69-90) derives every pinned number and names what both known-bad mechanisms would read. This is adequacy discipline done right.
- Every exact pin the lenses re-derived from `accumulator.rs` matches the committed number (u64 comb 6n+1 including the one-time extra carry, alternating pair 5 and 3, settlement 16, certified-run fold 6, first domination read 5 then 1, no-collapse separation 2k/32+3 then 1); I checked the same traces and agree.
- `witnesses.rs` records the surviving mutation each constructed corner was built to kill (`SIGN_DECIDED: 3 → 2`, decision index `floor + 1`, register constant `3 → 2`), which is exactly the provenance a future reader needs to judge whether a corner can be retired; `sign_threshold_survives_extreme_cancellation` (:40-47) cross-checks the fold against the independent low-to-high read-out before reading the sign, so a wrong threshold is convicted by a path that does not share it.
- `amortized_sequences.rs`'s module doc makes checkable claims about other files (every sibling `accum_*` band holds one polarity; the comb band drives the dual attack), and they hold where I read them (`comb_run`, `static_prefix_run` in before/tests/meter.rs).
- `claims/tests.rs`'s reach test (:139-161) makes the witness names in metered.rs load-bearing: a hollowed-out witness fails by name, and the exemption list is held load-bearing in both directions.

## Open questions for Finch

1. Where should the sign-flip test live? It uses only the public API and the `touch-meter` feature the metered module is gated on, so it could move into metered.rs (dropping the `touches` closure helper and gaining the exact-pin idiom), or stay as the crate's one integration binary (the adopting commit placed the three audit-adopted witnesses symmetrically across before/tests and suanpan/tests). Recommendation: cite it by manifest-relative path from claims.rs now (suanpan-tests-26 is a one-line change either way), and decide placement when suanpan-tests-25 rewrites its assertion; if suanpan keeps no other integration test, folding it into metered.rs removes a link step and the seed-anchor question for this crate never arises.
2. Should a zero-valued wide operand spill the register? Today `add_wide(&UBig::ZERO)` retires the register and `add_magnitude(&UBig::ZERO)` does not (suanpan-tests-4). Recommendation: short-circuit zero before the spill in the wide entries, matching `add_magnitude_shl`'s `Some(0) => {}`; a value-neutral call should not change the representation or the cost class of every later word-scale call. Either answer wants the witness.
3. The collapse fixed point: a top digit of magnitude 2 over a zero digit re-deposits 2^33 one index down, which recenters straight back to the same spelling, so every later sign read costs a constant 6 touches and lowers `bottom` for nothing. Amortized O(1) holds, but lib.rs:117-121 ("the next sign read re-reads none of them") describes the k−1 landing, not this carry-back. Recommendation: pin the 6 exactly (suanpan-tests-13) and hand the prose to the suanpan-docs partition; whether to special-case the re-deposit (deposit at the original index when the partial is a multiple of 2^32) is a production question I would not open without a caller that reads such values in a hot loop.
4. `assert_no_product` is duplicated between suanpan/tests/amortized_sequences.rs and before/tests/answer_embedded.rs. Recommendation: if suanpan-tests-25 replaces the grid check with an exact pin, the suanpan copy dissolves and no shared helper is needed; otherwise a comment at each copy naming its twin is cheaper than a shared dev-dependency.
5. Outside this partition, for the envelopes reviewer: before/tests/meter.rs:6637-6641 `accum_fan_touches_flat` calls `comb_run` at both scales exactly as `accum_comb_touches_flat` does at :6623-6627, while its doc describes a fan (Dyck-walk) stream the body never constructs. Duplicate test or missing `fan_run`? Recommendation: route to that partition's final; I verified the two bodies are identical by reading.
6. The vocabulary and dash findings (suanpan-tests-17, -19) are instances of workspace-wide conventions (159 "honest", 75 "tripwire" in before/src; 398 em-dashes in meter.rs). Recommendation: a workspace prose pass owns them; partition-local edits alone would create inconsistency. The site lists here are the suanpan input to that pass.

## Dropped

- No public way to observe the tier (api-economics [34]): refuted. Sibling unit tests asserting private representation state is what the `mod tests;` layout exists for; the crate page publishes the register's bounds and the `sign_dominates_at` rustdoc names the tier only as a hazard a caller acts on by not treating `decided` as a value function; before's comment at watermark.rs:761-763 says the forfeited register certificate "only reroutes the hop", so the consumer does not need the probe.
- Multi-level descent through near-cancelling digits reached only two levels deep (blind-spots [22]): below the bar after the reframe. The cancelling prefix `+2^k` then `−(2^k − 1)` sustains a partial of exactly 1 across 16, 11, 64, and 128 levels in `cancelling_prefix_chain_matches_the_oracle`, `sign_collapse_tightens_the_top_and_arms_domination`, and `no_collapse_fold_re_scans_the_prefix` (the last metered exactly); only a sustained partial of magnitude 2 is limited to the two-level witness, and the fold branches only on `partial == 0` and `|partial| >= 3`, so that case exercises no code the magnitude-1 case does not.
- Metered module doc promises exact pins (blind-spots [20]) and three assertions are bounds (api-economics [32]): duplicates of suanpan-tests-13.
- Hand-maintained state count in the ledger doc (blind-spots [25]): duplicate of suanpan-tests-11.
- Fully qualified Ordering (blind-spots [27]): duplicate of suanpan-tests-24.
- assert_no_product has no liveness floor (api-economics [28]): duplicate of suanpan-tests-25; its executed run is credited there.
- Only sign-flipping instrument unrostered and alone in its binary (api-economics [29]): duplicate of suanpan-tests-26; placement moved to the open questions.
- Ledger invariants never observed after rebuild/clear ops (api-economics [30]): duplicate of suanpan-tests-10.
- Run-forming proptest duplicated with copied seeds (api-economics [31]): the test half is in suanpan-tests-6, the seed half in suanpan-tests-1.
- Test models restate production constants (api-economics [33]): split into suanpan-tests-21 (SIGN_DECIDED), suanpan-tests-9 (LEDGER_OPS), suanpan-tests-11 (tally), and suanpan-tests-7 (the `+ 33`, where the constant is wrong, not merely unnamed).

<!-- source: final/suanpan.md -->
# Partition suanpan: suanpan: the cliff-free signed accumulator, limbs, magnitude, touch meter, and the claims roster

Reviewed at commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764 (working tree clean). Nothing was built, run, or modified; every "verified" below means read with line numbers, grepped, hand-traced, or checked with read-only git.

## Partition summary

suanpan is a small crate with one load-bearing type. `Accumulator` (accumulator.rs, 1515 lines) holds a running signed integer as redundant balanced base-2^32 digits kept in the lazy zone `|d| < 2^33`, in front of which an exact `i128` quick register absorbs word-scale work until the first wide operand, wide shift, or outgrown sum spills it, once per reset epoch. Three arguments carry every cost the crate page quotes: recentering keeps carries amortized O(1) per word-scale write; the sign fold collapses the cancelling prefix it scans so a digit is read at most once per write that made it nonzero; and a zero-run ledger of `(lo, hi)` certificates lets top settlement skip never-written gaps in one touch. `Limbs` (limbs.rs) denominates wide operands in 64-bit limbs whatever the backend word width; `Magnitude` (magnitude.rs) is the width-dispatch seam; `touch_meter` (touch_meter.rs) is the feature-gated counter every cost row is denominated in; `claims.rs` and `claims/tests.rs` are a roster that binds the crate-page table, the public surface, and the committed witnesses to each other.

The kernel is reviewably correct on 64-bit targets. I traced `add_at`'s recentering (carry `(t + 2^31) >> 32`, remainder in `[-2^31, 2^31)`, `|carry| >= 2` whenever `|total| >= 2^33` so the chain always ends in the in-zone arm), `read_digits`' final-carry closure over `[-3, 2]`, the `SIGN_DECIDED = 3` margin against the `2.01` geometric tail, the register arm's checked chain in `sign_dominates_at`, and the ledger's three maintainers through collapse; every clause of the `zero_runs` field doc holds as written. The instruments are stronger than before's envelope convention: the metered pins assert exact totals across a doubling of the axis each row claims independence from, so the liveness floor and the flatness witness are one assertion, and the known-bad collapse-less fold is committed and shown red. The crate page is the best public prose in the two crates: every bound sits beside its derivation, hazards are stated where a user meets them, and coined terms are anchored to identifiers.

Four findings carry weight. One correctness defect: the digit-position arithmetic in `apply_limbs`, `fold_accum`, and `deposit_value` is unchecked `usize` addition after a checked `shift / 32`, and because zero contributions are skipped before any buffer growth, a shift near `usize::MAX` digits on a 32-bit release build deposits the operand at digit 0 with no panic and no allocation failure, against the `# Panics` contract (suanpan-24; 32-bit only, one helper fixes it). Two verification disputes: the `add_at` exit `debug_assert!` scans the whole buffer above `top` on every digit write in every debug build, the exact shape the owner's own assertion audit retired three days before it was added (suanpan-21); and the two suanpan entries in `.cargo/mutants.toml` are excluded as value-equivalent or work-only, but both mutants change exact touch counts, which the crate declares a public contract, and one of them dissolves by the header's own refactor step (suanpan-40). One instrument gap: the headline "on every input sequence" quantifier is pinned only at canonical schedules, never on the random streams the differential suite already drives (suanpan-2). The rest are small repeated forms in `accumulator.rs` that one private home each would collapse, roster data that describes the pre-register code, and prose hygiene under Principle 5.

Lines read: 2,939 across the eight partition files (Cargo.toml 28, lib.rs 365, accumulator.rs 1515, limbs.rs 72, magnitude.rs 54, touch_meter.rs 48, claims.rs 463, claims/tests.rs 394), plus the cited ranges of accumulator/tests/{metered,ledger,witnesses,differential}.rs, tests/amortized_sequences.rs, .cargo/mutants.toml, tools/mutantcheck-expected.json, the root Cargo.toml, and the before-side callers. Test files in the partition: claims/tests.rs; `claims.rs` itself is `#[cfg(test)]` roster data. magnitude.rs and touch_meter.rs are clean under every lens.


## Positives

- The crate page (lib.rs) is the strongest public prose in the two crates: every bound in the cost table has its argument on the same page, the amortization vocabulary is the standard potential-method one, hazards are stated where a user meets them (sign queries take `&mut self` and why, 123-126; no `PartialEq` and why, 307-309; `Sync` buys less than usual with the exact list, 299-307; `is_literally_zero` is one-sided with a worked example, accumulator.rs:888-911; `sign_magnitude_shl`'s pair is not a normal form, 970-993; touch readings need serial scenarios, touch_meter.rs:16-20), and the "When not to reach for it" section (250-268) does the which-one-do-I-use job explicitly, binary-counter argument included.
- Coined terms are anchored to identifiers almost without exception: lazy zone/`LAZY_LIMIT`, recenter/`RECENTER_BIAS`, quick register/`quick`, zero-run ledger/`zero_runs`, collapse/`fold_and_collapse`, spill/`spill`, digit engine/`enter_digit_engine`, written span/`bottom`, limb/`Limbs` (the 64-bit definition stated once at lib.rs:85-86 and enforced by `WORDS_PER_LIMB`).
- The `zero_runs` field doc (accumulator.rs:124-154) is a correct and complete statement of the ledger's invariants; I traced jump-insert, `crop_runs`' descending early stop, `consume_run_at`, and collapse through a certified run by hand, and every clause (soundness, disjointness, containment under the settled top, including the lower-remnant collapse case) holds as written. `ledger_invariants_hold_exhaustively` checks the letter of each clause after every step of every schedule at depth 6 over an alphabet chosen to reach every collapse-over-certificate interaction.
- The exact-count-at-two-scales pin form (metered.rs:1-11) makes the liveness floor and the flatness witness one assertion (e.g. `6 * pairs + 1` at k = 4096 and 8192), stronger than before's x1.25 envelope convention, and the known-bad mechanism (`no_collapse_fold_re_scans_the_prefix`) is committed and shown red at two widths. `scaled_read_costs_the_span_not_the_write_count` refutes the tempting misreading of the O(w) row with a 1,003-touch pin.
- The three constants are pinned tight by constructed witnesses that mutation testing motivated (`SIGN_DECIDED` 3 -> 2 and a decision index of floor + 1 each survive the differential suite and fail only in witnesses.rs); the 2.01 geometric bound and the 3 > 2.01 margin check out, and `sign_dominates_at`'s doc is precise that `decided` is a property of the representation, pinned both ways.
- `read_digits`' carry comment (1074-1079) is a real one-paragraph proof (I checked that with `|digit| < 2^33` the recurrence keeps carry in `[-3, 2]` from 0); `add_at`'s invariant-window comment (1356-1368) names exactly where the exact-top invariant does not hold and why the loop restores it; `sign_dominates_at`'s register arm is total by construction (`checked_add`/`checked_mul`/`try_from`/`checked_shl`) and the digit arm's `saturating_add(2)` states the wrap it prevents in one sentence.
- The metering seam (`touch`, accumulator.rs:16-26) compiles to nothing without the feature so the kernel carries no cfg noise; no `debug_assert!` body meters, so the exactness contract is independent of the debug-assertions setting; `Limbs` derives `WORDS_PER_LIMB` from `u64::BITS / Word::BITS` so the denomination is target-independent by construction and iteration borrows the stored words.
- The claims roster is total in both directions over the extracted surface, binds witnesses by reach rather than existence (closing the hollowed-out-witness path), and holds `REACH_EXEMPT` load-bearing so a stale exemption fails by name (claims/tests.rs:369-373). Module decomposition is clean: four single-responsibility modules, an acyclic dependency graph, and a crate root that re-exports without declaring anything of its own.

## Open questions for Finch

1. Is the exact-touch contract (lib.rs:283-286, "a change to any operation's count is a breaking change") meant to freeze constant-factor improvements to the kernels, or to guarantee determinism with the specific numbers versioned? suanpan-17 (the 6-touch collapse fixed point), suanpan-10 (lazy spill), and the fused per-limb pass hypothesis below are all blocked by the first reading. Recommendation: restate it as "deterministic and pinned; a count change is a versioned change named in its commit", which keeps the pins as the contract's enforcement without forbidding improvement.
2. Is suanpan meant to be publishable or testable standalone? The `../before/tests/meter.rs` path (suanpan-30) and the workspace-path `surface-scan` dev-dependency make `cargo test -p suanpan` fail outside this checkout. Recommendation: add the in-crate `add_small`/`sub_small` pin either way; decide on `BANDS` by whether the answer is yes.
3. Should suanpan's own suite ever run on a 32-bit target? Today crates/before/wasm32-pins names suanpan nowhere, so suanpan-24's red-first pin and suanpan-26's `WORDS_PER_LIMB = 2` coverage have no first-party home. Recommendation: extend the wasm32 guest with a small suanpan leg (the two constructions in suanpan-24 plus the `Limbs` round trip); the fix for suanpan-24 should land regardless.
4. What is the roster's citation charter: minimal citation sets per row (b8513643's stance, stated only in a commit message and about semantic pins), or every touch instrument cited? suanpan-33 turns on it. Recommendation: state it at claims.rs:111-115 and cite tests/amortized_sequences.rs either way, since it is a cost instrument, not a semantic pin.
5. A measure-first design hypothesis, not a finding: `apply_limbs` (1467-1489) makes two `add_at` calls per limb, each running the jump check, carry loop, `crop_runs`, and `settle_top`; a fused low-to-high pass over the operand span (one carry variable, one crop and settle at the end) would remove per-call overhead and touch a carried-into position once instead of twice. Release overhead per call is small (a bounds check, an empty-map early return, one loop-condition read), suanpan has no bench, and the change moves the public touch counts. Recommendation: only if before's bench-judge shows the wide path on a profile; measure at the parent first.
6. The touch-accounting contract is stated in full twice (lib.rs:272-292 and touch_meter.rs:3-20) plus a summary in Cargo.toml:23-27; because `touch_meter` is cfg'd out without the feature the crate page cannot simply link to it. Recommendation: keep both, add one line to each saying the two move together, and fold the suanpan-6 amendment into both at once.

## Dropped

- [49] Fused per-limb `add_at` pass: reframed by the refutation pass to a measure-first hypothesis with unmeasured gain and a contract cost; moved to open question 5.
- [15]'s `is_literally_zero` leg: "two field reads: no digit is touched" is accurate in the meter's read-modify-write denomination (a plain read of `digits[0]` is not a touch); the `new` and `digit_count` legs survive in suanpan-32.
- [20]'s "arm" leg: the arming family is before's established watermark-web vocabulary (245 uses in crates/before/src), not a register transplant; the other legs survive in suanpan-19.
- [22]'s "certificate" leg: each referent is defined under its own crate-page heading (128 and 170) and the bare uses in accumulator.rs sit under docs whose subject fixes the referent; the "derived" and pool-reuse legs survive in suanpan-19.
- [17]'s "Three maintainers" and "four `*_accum` entry points" legs: accurate as written (the list names the three certificate mechanisms; four public fns are so named); only the "both cost arguments" leg drifted (suanpan-7).
- [0]/[47]'s `enter_digit_engine` scan leg (1293-1296): once-per-epoch, explicitly within 9f68c475's ruling; the `add_at` exit scan survives as suanpan-21.
- [23]'s table-row leg: the table is denominated in digit touches (lib.rs:200) and `reserve_digits` has no digit-touch axis; the memory-paragraph leg survives as suanpan-5.
- [47] duplicate of suanpan-21; [39] duplicate of suanpan-24; [15], [43], [38] merged into suanpan-32; [16], [35], [51] duplicates of suanpan-27; [37] merged into suanpan-16; [46] merged into suanpan-30; [44], [52] merged into suanpan-7; [22] merged into suanpan-19.
- [0]'s construction as written (`reset()` then `add_small`): `reset()` returns the accumulator to the register (accumulator.rs:678), so the loop never reaches `add_at`; corrected in suanpan-21 with a spill before the loop.

<!-- source: final/surface-roster.md -->
# Partition surface-roster: The public-surface roster, the surface-coverage suite, surfacecheck, and surface-scan

## Partition summary

This partition is the machinery that holds `before`'s public surface total against its differential coverage. `crates/before/src/surface.rs` is the roster: `METHOD_SURFACE` (one row per inherent `pub fn`, three leg dispositions each), `FAMILY_SURFACE` (rows by operator or trait family), and the typed `Exclusion` vocabulary whose variants each defend one exclusion argument once. `crates/before/src/testing/surface_coverage.rs` and its `tests.rs` enforce the roster in-tree: a rustfmt-shape line scan over the hand-named `SURFACE_SOURCES` list is held equal to `METHOD_SURFACE` both ways, every cited name is resolved against a `#[test]`-attribute scan plus the law and descriptor registries, exclusion payloads resolve the same way, same-named tests are rostered, and the fold seeds are pinned committed. `crates/before/surfacecheck` is the second, stronger extractor: a detached binary that walks nightly rustdoc JSON, holds every function-like item to `METHOD_SURFACE` or a module exception, and reconciles every trait impl and non-function item two ways against the pinned censuses in `census.rs`. `crates/surface-scan` is the crate-agnostic line scanner shared with `suanpan`; `tools/citecheck` resolves every citation against the runner's own `cargo nextest list` inventory in the gate.

The instrument is well built where it is built. Every reconciliation is two-way, so no jaw can pass by a counter going dark; `reconcile_with` is a pure function with a committed red demonstration per finding category; the rustdoc `format_version` gate runs before any schema-typed parse and names both numbers; the extractors panic rather than under-report on the shapes they recognize; and the `Exclusion` enum turns exclusions into defended families with resolvable payloads and an inhabitation census over its own vocabulary. Citation integrity is closed from more sides than most rosters bother with (bare names, payloads, duplicate names across files, binding kinds shadowing each other, and collection by the runner).

The dominant issues are seams between what the prose promises and what the code enforces, plus machinery that outlived the constraint that justified it. Three prose sites say the surface-totality gate fails until a `FAMILY_SURFACE` row is added; surfacecheck never reads `FAMILY_SURFACE`, and today the census already holds impls (`Default for Version`, the `Cow<Version>` conversions, `error::Overlap`/`TooWide`) that no family row dispositions. The `Party::tick` row cites three tests that never call `Party::tick` while the law that does is cited by nothing. Five writer-sink rows carry no resolvable name on any leg, and the only evidence behind `Span::encode_to` is a doctest whose endpoints coincide. The line-scan extractor's justification dissolved when surfacecheck landed, and the tree now carries two extractors, three `#[test]` scanners with two rule sets, an empty per-item exception list with full plumbing, and a dated-ruling convention already reported to the owner as an open design item. A handful of Principle 5 defects (a deleted API name in a doc example, a pointer to a note that does not exist, temporal phrasing at a declaration site) round it out.

Lines read: the eleven partition files in full (4,037 lines; the test files are `surface_coverage/tests.rs`, `surfacecheck/src/check/tests.rs`, and `surface-scan/src/tests.rs`), plus the neighbors every finding rests on (the justfile's pin, citecheck, gate-stream, and surface-totality sections; `.github/workflows/ci.yml`; `tools/citecheck`'s header and inventory filter; `laws.rs` around the tick law; `codec/tests.rs`, `span/wire.rs`, `rank.rs`, `ranked.rs`, `version.rs`, `serde_impls.rs`, `borsh_impls.rs` and its tests for the writer-sink doors; `auto_traits.rs`; `shape.rs`; `meter/registry.rs` and its tests for the dated-ruling precedent; `validation_index.rs`; `tests/doc_hidden.rs`; `tests/foreign_reexport.rs`; `tests/amp_board_smoke.rs`; `suanpan/src/claims/tests.rs`; `meter/board/coverage/tests.rs`; `causally.rs`). Programs run: a Python count over `census.rs` and `surface.rs` (437 impl rows, 373 distinct impl strings, 64 doubled, 17 `StructuralPartialEq`; 40 `Law`-plus-twin-exclusion rows) and `rustfmt --check` on scratch copies of `surface.rs`. No cargo, just, or test command was run; every construction below is by reading unless marked executed.


## Positives

- surfacecheck refuses a rustdoc JSON `format_version` mismatch before any schema-typed parse (main.rs:67-85), names both numbers, and the bump procedure is written at the exact `rustdoc-types = "=0.59.0"` pin (Cargo.toml:22-29) and in the justfile recipe. A nightly bump can fail the leg but cannot make it silently wrong.
- Every reconciliation in the partition is two-way: roster against extraction (tests.rs:77-94), census pins against reachable impls and items (check.rs:243-258), duplicate names against their roster (282-303), binding kinds against each other (314-332), dead and shadowing exceptions (check.rs:305-321). Nothing here can go green by a counter going dark, and check.rs:12-15 states correctly why the censuses need no separate anchor.
- `reconcile_with` (check.rs:228-329) is a pure function over explicit tables, and check/tests.rs demonstrates every finding category red on synthetic input, including the empty extraction tripping the anchors and the module-prefix `::` non-leak.
- The refusal-over-under-report discipline is applied where the extractors recognize a shape: surface-scan panics on an unnamed `pub fn` and a nested impl (lib.rs:104-127); the rustdoc walk panics on dangling ids, unexpected item kinds, and unhandled type or generic-args shapes (extract.rs:68-73, 239, 277, 352, 361); a malformed `use` target asserts rather than shrinking the surface (197-204).
- The typed `Exclusion` vocabulary (surface.rs:57-150) makes an exclusion a defended family with a resolvable payload rather than free text; per-family structural obligations are enforced (a `GridCap` guard must be a `#[test]`, a `NotAPaperObject` `bound_at` must resolve to a row, test, or law), and `every_exclusion_family_is_inhabited` applies the scaffolding audit to the enum itself.
- One roster, several consumers, no copy: surfacecheck, the in-tree suite, before-fuelscape, and the board coverage tiling all bind to `before::surface::METHOD_SURFACE` under `meter`, so the enumeration the reviewer edits is the one every instrument enforces.
- Citation integrity is closed from more sides than most rosters bother with: bare names, payloads and `bound_at`, a positive-and-negative haystack liveness test, same-named tests across files held equal both ways, kinds that shadow each other, and collection by the runner (tools/citecheck, whose fabricated-citation tripwire runs on every live run and whose header records the rejected alternative).
- `tests/doc_hidden.rs` and `tests/foreign_reexport.rs` close the two channels both extractors structurally miss, and extract.rs:31-32 states the dependence explicitly. Together with surfacecheck the three are total over documented, hidden, and re-exported surface.
- `--list` (main.rs:141-176) renders every extracted row with its disposition, which is exactly the triage view a reviewer needs when a totality run goes red.

## Open questions for Finch

1. Retire the line-scan extractor (surface-roster-9), or keep it as a stated stable-toolchain inner-loop convenience? `just ci` runs `test-all` but not `surface-totality` (that runs in the `instruments` CI job and the gate's `surface` stream), so retirement moves the only totality feedback to the gate. Recommendation: retire; the JSON walk catches strictly more, the retirement bar is met, and the doc-hidden contradiction (surface-roster-21) dissolves with it.
2. For `FAMILY_SURFACE` (surface-roster-7): correct the three prose sites now and treat binding census pins to family rows as a design round, or build the binding directly? Recommendation: prose fix plus rows for the orphans now; the binding as a follow-on, since it reshapes the meter-public `SurfaceRow` and the three non-impl rows need a disposition.
3. The dated `decided` convention (surface-roster-17) is repo-wide (surfacecheck and the meter registry) and was reported to you as an open design item in d2a9d04e. Recommendation: drop the field at both sites and let git carry the when; if kept, record the exception to Principle 5 once in the project instructions and unify the validator.
4. Should the rustdoc census cover public struct fields, enum variants, and type aliases (surface-roster-22)? Recommendation: enumerate-and-panic and the prose fix now; extend the census only if you want shape changes diff-visible in the gate, since the roster's vocabulary is operations.
5. Per-suite citations versus `DUPLICATE_TEST_NAMES`: a cited-name uniqueness rule would let the roster go, but the roster's file pinning is what the seed tripwire composes with (moving `join_all_matches_the_recursive_oracle` to another file breaks `d1_seeds_stay_committed` through the file list). Recommendation: keep the roster; it is a deliberate, inline-defended choice.
6. `StructuralPartialEq` rows (surface-roster-24): keep for the derived-vs-hand-written signal, or skip beside `is_synthetic` to remove a perma-unstable marker from the pin list? Recommendation: skip, with the reason stated at the skip; a derive-to-manual change is visible in the diff that makes it.
7. "door" is used as crate-wide vocabulary in the roster (surface.rs:78, 103, 111, 235) with no defining site I could find. Is it defined somewhere outside this partition, or does it need one anchor?

## Dropped

- [3] DUPLICATE_TEST_NAMES dissolution: deliberate and documented in code (tests.rs:15-23, 1e6af44f); the proposed uniqueness rule drops the file pinning the seed tripwire composes with; carried as open question 5.
- [8] Vestigial forks entries: prophylactic mappings with no current payload, folded into surface-roster-9; the "coverage note" ghost is a site of surface-roster-20.
- [19], [32], [53]: duplicates of surface-roster-7.
- [2], [28], [38], [58]: duplicates merged into surface-roster-17 (dated rulings); [22], [33], [61]: merged into surface-roster-18 (ITEM_EXCEPTIONS).
- [21], [31], [57]: duplicates of surface-roster-10.
- [30]: duplicate of surface-roster-12.
- [25], [40], and the `d1_` item of [62]: duplicates of surface-roster-15; the `d_fork_join_roundtrip` item of [62] is outside this partition (party/tests.rs).
- [27], [37], [60]: duplicates of surface-roster-3; the `20`-threshold item of [27] is in surface-roster-26.
- [26], [42], [43], [59]: duplicates merged into surface-roster-2 (repetition and counts) and surface-roster-20 (ghost and temporal references).
- [45], and the idiom items of [62]: merged into surface-roster-26.
- [52]: duplicate of surface-roster-11.
- [20]: duplicate of surface-roster-28.
- [48]: duplicate of surface-roster-30.
- [35]: duplicate of surface-roster-6.
- [6], [39]: merged into surface-roster-20.
- [9], [34], [47]: merged into surface-roster-24.
- [24], [54]: merged into surface-roster-22.
- [15], the metaphor items of [41] and [62]: merged into surface-roster-25.
- [41] TRIPWIRES rename: the two-genre sense is defined at its site by contrast (surface_coverage.rs:71-92, 112-113); a vocabulary preference, not a defect.

<!-- source: final/testing-diff-gen.md -->
# Partition testing-diff-gen: The differential table, generators, op traces, rng, brute-force grow, compactness, snapshots, asymptotics pins, shape rows, fuelscape islands

## Partition summary

This partition is the test-side apparatus of `before`: thirteen files under `crates/before/src/testing/`, all compiled only under `cfg(test)` (`#[cfg(test)] mod testing;` at `crates/before/src/lib.rs:453-454`), so none of it ships. Six of the files hold `#[test]` functions directly: `diff_ops/tests.rs` (the descriptor table's guards, drivers, and known-bad convictions), `generators/tests.rs` (the generator liveness census), `compactness/tests.rs` (the skyline-versus-min-lifted size envelope), `snapshots.rs` (insta inline goldens), `asymptotics.rs` (the documented-asymptotics liveness pins), and `fuelscape_islands.rs` (the island doc-attachment totality pin). The other seven are scaffolding those suites and the rest of the crate's tests consume: `diff_ops.rs` (the `diff_ops!` descriptor table and its roster), `generators.rs` (adversarial deep shapes, arbitrary normal-form strategies, variadic-law families), `optrace.rs` (the seed-derived op-trace strategy and its two appliers), `rng.rs` (one home for seeded randomness), `grow_brute_force.rs` (the DP-independent grow-optimality reference), `compactness.rs` (the envelope check and the alternating-comb builder), and `shape_rows.rs` (the shape-walk row vocabulary). I read all 4192 lines with line numbers at commit 9e5784fb.

The two load-bearing designs hold up. The `diff_ops!` macro makes registration and execution the same act (a descriptor cannot be written without being registered under its own `stringify!`ed name, and every consumer expands the one roster), the tiling pin holds every `Bound` citation in the coverage roster to exactly one side in both directions, and the known-bad descriptors are convicted with two-direction witnesses (convicted where the spellings differ, passing where they coincide). `rng.rs` states the portability argument once, correctly (the workspace resolves `rand_core` 0.6.4, whose `seed_from_u64` is documented value-stable). `grow_brute_force.rs` is a real independent reference: full enumeration, no pruning, exact unchecked cost. The shape-row folds assert the walk vocabulary's own invariants on every population that touches them. `fuelscape_islands.rs` closes the one direction the compiler cannot. Nothing in the partition masks a production failure.

The dominant issues are in the two instruments whose prose has drifted from what they measure. `asymptotics.rs` binds its pins to rustdoc sentences that no longer exist: the Display pin quotes a "summary-merge" sentence removed at b3f09baa, the fold-door messages send a maintainer to `# Complexity` sections that now hold only an `include_str!` of a fuelscape island whose text is authored in `crates/before-fuelscape/src/ops.rs`, and the three `mul_bound_*` pins guard an `Ω(M(|v|))` floor the public contract does not state. Its module doc claims one pin per public door that documents the log factor, but the island contracts state `log k` for eleven fold doors and five are pinned. Its floors are transcribed midpoints between a linear reference the harness recomputes on every run and only prints, and for `Party::join_all` that midpoint sits 2% from either endpoint, so the doc's "a crossing is a class change, never noise" overstates. `compactness.rs` still speaks from before the flag day (faf3cd0a, 2026-07-25) on which the skyline became the stored coding: "the claim its adoption turns on", "today's size", "decision-era", and a `Sample.current_bits` field documented as "Today's live encoded bit length" that is in fact the reconstructed min-lifted reference stream. The keep of the compactness probes is a recorded owner ruling (the 2026-07-24 scaffolding sweep's defended keeps), so only the prose is open here.

The rest is smaller: a dead proptest dimension in the organic driver, a generator census whose doc promises per-arm detection it does not deliver for three `arb_base` arms, an empty exemption roster in the island pin, an island scan blind to the crate's own macro-form includes, a fixture builder whose distinctness claim fails on one draw, and the usual dedupe and vocabulary nits ("door" is used forty-odd times and defined nowhere; "mint" and "honest" appear at four sites). I ran no cargo, just, or test command; every "verified" claim below rests on reading files and read-only git at 9e5784fb.


## Positives

- `diff_ops.rs:263-419`: registration is execution. The `diff_ops!` macro cannot emit a descriptor without registering it under its own `stringify!`ed name; the roster's signature tuples are a compile-time tie (a novel signature is a type error at the driver, not a runtime miss); the three spellings sit side by side over identically named bindings in separate scopes so `!Clone` production types cost nothing; and the comparison is delegated to result types so no descriptor carries an assert-kind switch.
- `diff_ops/tests.rs:67-120`: the tiling pin is two-directional in both tables (a bespoke entry must be cited and must not be derived; a derived descriptor must be cited), so "bespoke" is a rostered status a reviewer diffs rather than a default; `every_bespoke_genre_is_inhabited` (122-136) applies the dissolution discipline to the instrument's own vocabulary.
- `diff_ops/tests.rs:480-630`: the known-bad descriptors are convicted with two-direction witnesses (convicted where the spellings differ, passing where they coincide), so the tests prove the comparison discriminates rather than merely fails; the fs known-bad keeps both walk legs agreeing so a conviction is evidence about `FsMatches` alone.
- `rng.rs`: one home for seeded randomness, with the portability argument stated once as mechanism (ChaCha8Rng's documented-portable output, `seed_from_u64`'s value stability, the proptest draw-pattern caveat) and the consequence drawn (a major bump is a deliberate corpus-regeneration event). I verified the workspace resolves `rand` 0.8.6 / `rand_chacha` 0.3.1 / `rand_core` 0.6.4 (Cargo.lock), so the "rand_core 0.6" claim is accurate at HEAD.
- `generators/tests.rs`: the census is the right instrument for the sampled tier (a totality pin under a committed seed with per-class floors), it prints its readings so a re-pin restates floors without editing the harness, and the one floor that deviates from the default fraction states why at the number (129-132).
- `generators.rs:369-406`: `arb_fold_arity` derives its boundaries from the balanced counter's structure (`k = 0, 1, 2, 3, 4, 6, 2^j, 2^j + 1`) rather than from observed values, which is exactly the mechanism-derived population the doctrine asks for.
- `generators.rs:263-283`: `deep_left_spine_party`'s hand-built canonical bits are pinned by a strict decode round-trip at depth 100,000 (clock/tests.rs:565-573) and at depth 48 (codec/tests.rs:1627), so the normal-form claim is enforced, not argued.
- `shape_rows.rs:22-54`: every fold asserts the item stream's own invariants in passing (nonzero rises, never-negative running height, exact dyadic tiling), so every descriptor and test that folds a walk holds those invariants on its whole population for free; `assert_tiles` is strictly stronger than "widths sum to 1" (it rejects `[2, 1, 2]`).
- `grow_brute_force.rs`: the independence claim is real: no pruning in `all_inflations`, costs from first principles, exact unchecked arithmetic deliberately distinct from the DPs' saturating folds, the paper's tie-break transcribed (`cl < cr`), and the module doc states what the oracle differentials cannot catch and why this can. Consumed at five sites.
- `asymptotics.rs:128-225`: every metered region is exactly the door call (population build, byte totals, and `remove(0)` all run before `reset_scan_bits`), and the ratio floors are inherently live: a counter that goes dark reads growth 0 and fails. `mul_bound_embedding_is_alive` guards the factors' content (balanced-term count, isolated non-uniform mass bits), not width alone.
- `compactness.rs:131-166` with `compactness/tests.rs:88-101`: the comb's closed forms check arithmetically (spine `2(pairs − 1)`, tooth `2·m_bits + 6`, total `pairs(2·m_bits + 8) − 2 = 2,105,342` at 1024/1024), and the tightness witness (ratio > 1.994) shows the factor-2 envelope is tight rather than loose, which is the adequacy question a ceiling must answer.
- `optrace.rs:56`: `MAX_TRACE_OPS` is derived into `semantic_oracle::GRID_N` (`optrace::MAX_TRACE_OPS as u32 + 2`, semantic_oracle.rs:80) rather than transcribed.
- `snapshots.rs:194-201`: `rank_row` pins `Debug ≡ Display` beside every golden row; no pending `.snap.new` or `.pending-snap` files exist in the tree.
- `fuelscape_islands.rs:63-87` with `build.rs:76-80`: the pin checks both directions (emitted ⊆ included-or-exempt, included ⊆ emitted), and build.rs keeps the `.open` variant outside the scan charset with the reason stated at the site.

## Open questions for Finch

1. Compactness envelope (finding 17): the keep is ruled; where should the 2x-over-min-lifted bound live in present tense? Recommendation: state it once in `version/skyline.rs`'s design essay beside the transcoder mention (88-89), re-denominate `compactness.rs` against it, and add a validation-index row naming what the envelope alone catches (a skyline coding change that exceeds the reference by more than the argued factor, which the bit-exact sizer identity at skyline/tests.rs:514-523 cannot see because both sides move together).
2. The `Ω(M(|v|))` floor (finding 22): is it a public promise of `rank`/`distance`/`lag`/`Ranked`, or private derivation? Recommendation: keep it private (the style rule keeps Ω out of headlines) and re-scope the three `mul_bound_*` pins' prose to the private derivation in `query.rs`/`integral.rs`, restating the invariant inline.
3. `Party::join_all`'s log-factor witness (finding 28): the stagger population expresses the id fold's log factor at about 4% where the version doors show about 22%. Is there a committed id population whose unions do not densify? Recommendation: construct one (isolated owned leaves at maximal depth); failing that, restate the module doc's "class change" sentence to what a two-scale tightness pin can attribute.
4. Fold-door roster totality (finding 23): should a test derive the pinned-door roster from the `fuelscape/*.json` contracts? Recommendation: yes, a `PINNED_FOLD_DOORS` const plus a JSON-reading totality check with reasoned exclusions at the check site (`clock_recv_all` delegates to `Version::join_all`); then pin or excuse `sync_all` and the four `Span` doors.
5. Registration totality pin (finding 6): keep the source scan (shared helper with `laws/tests.rs`) or retire it in favor of the gate's `dead_code` lint after executing the construction? Recommendation: keep, share the helper, and restate the doc; the pin holds roster totality against an ad-hoc-driven group, which the lint cannot see.
6. The word "door" (finding 24): define once or replace? Recommendation: define once in `asymptotics.rs`'s module doc, where it is load-bearing, and cite from `registry.rs` and `laws/tests.rs`.
7. Two cross-partition notes from the structure-prose lens, out of this partition's scope but affecting claims made here: `bridge::emit_id` (bridge.rs:29-45) recurses without `descend!` while `emit_ev` routes through it, and generators.rs:266-268 relies on that asymmetry to justify the bit-built deep spine; `recurse.rs:9-14`'s inventory of test-side depth recursion does not list the oracle-tree recursions in `shape_rows`, `grow_brute_force`, and `generators` (`bushy_*`). Both belong to the crate-root and bridge partitions.

## Dropped

- [0]/[18] dissolution branch of the compactness suite: reopens a recorded owner ruling (defended keeps of the 2026-07-24 scaffolding sweep, `.agent-notes/2026-07-22-.../before-adversarial-resource-amplification.md:1756-1760`, reaffirmed at faf3cd0a) with no new evidence; the prose half is finding 17.
- [24] `balanced_terms` differs from production at `t == 2^31`: refuted. The replica matches `mul_into`'s `>` rule (web.rs:158); the `(sum + 2^31) >> 32` rule is `WindowMass::combine` (integral.rs:708), a different kernel. The residual (no committed tie; misnamed settle path) is finding 29.
- [1]'s primary resolution (replace measured floors with model-derived margins): reverses the f0cd4ab2f ruling "Floors are measured, never transcribed"; carried as the owner-gated alternative under finding 26.
- [32]'s resolution (a) (name both endpoints as consts beside each pin): reverses the 4797009a excision of unpinned readings from prose; its (b) half is finding 26.
- [22] and [29]: duplicates of [4] (finding 7).
- [20] and [30]: duplicates of [6] (finding 31).
- [38]: duplicate of [7] (finding 20).
- [34]: duplicate of [8] (finding 30).
- [31]'s duplication half and [9]: merged into finding 16; [31]'s doc-rot half kept there with the two additional sites the refutation pass found.
- [49]: duplicate of [19] (finding 32).
- [47] and [15]: duplicates of [2] (finding 22).
- [26], [27], [52]: merged into finding 17.
- [33]: duplicate of [3] (finding 29).
- [42]: merged with [12] into finding 5.
- [43]'s `GENRES` half: merged into finding 4; its qualified-path and `Shape` halves are finding 25.
- [44]'s `version_block` item: folded into finding 25; its `op_strategy` and `Sync` items are finding 15.
- [10]: kept as finding 1 (nit) with the refutation's closed-roster caveat.
- [40]: kept as finding 19 (nit); the history rationale states what the builder does, not why the oracle path was not used.
- [46] as a separate liveness claim about the module doc: merged with the pin-specific margin into finding 28.
- The structure-prose lens's cross-partition questions on `bridge::emit_id` and `recurse.rs`: out of this partition's scope; listed under open questions as notes for the owning partitions.

<!-- source: final/testing-oracles.md -->
# Partition testing-oracles: The test-only harness core: bridge, semantic oracle, exhaustive enumeration, algebraic-law binding, validation index

## Partition summary

This partition is the test-only core of before's differential architecture. `testing.rs` is the front door that declares the scaffolding and suite modules. `bridge.rs` converts between the recursive paper oracle's trees and the impl's values by emitting and reading the impl's stored bits directly (never the public codec), so structural agreement is decoupled from codec correctness. `semantic_oracle.rs` is the third reference: the paper's section 4 function-space construction realized as closures over dyadic points, with random section-4-valid `fork` and `event` policies, a derived grid ceiling `GRID_N`, and resolution probes; its `tests.rs` replays one single-seed op trace against all three references (`replay_matches_across_references`), holds two known-bad references convicted, and pins the grid derivation's premises. `exhaustive.rs` enumerates every canonical id tree and event tree under small depth bounds and its `tests.rs` runs every public operation over every tree and ordered pair against the oracle, plus five hand-spelled symmetry tests and a closed-form corpus totality pin. `algebraic_laws.rs` is a thin binding whose `tests.rs` expands both driver genres from `crate::for_each_law_group!`. `validation_index.rs` is a documentation-only map of the crate's instruments. All nine files compile only under `cfg(test)` (`lib.rs:453-454` gates `mod testing`), so nothing here ships; the three `tests.rs` files are test modules and the other six are test scaffolding. I read all 3168 lines of the partition with line numbers, plus the supporting files each finding cites (`recurse.rs`, `oracle.rs`, `oracle/version.rs`, `oracle/party.rs`, `party.rs`, `version.rs`, `laws.rs`, `optrace.rs`, `generators.rs`, `grow_brute_force.rs`, `grow/tests.rs`, `party/tests.rs`, `surface.rs`, `surface_coverage.rs`, `encode.rs`, the justfile, `.config/nextest.toml`, `tools/citecheck`, before's `AGENTS.md`, and the 2026-07-22 decision record). I ran no cargo, just, or test command; every "verified" below means read, grepped, or hand-derived, never executed.

The adequacy discipline is the partition's strength and is done unusually well. The two known-bad references (the cell-dropping Riemann sum, the mirrored embedding) are committed behind inverted assertions, the mirrored-embedding test also demonstrates the pointwise differential's blindness that makes the absolute-geometry anchor necessary, and both are rostered by name in `surface_coverage::TRIPWIRES` and resolved against nextest's live inventory from outside the crate. `corpus_counts_are_exact` pins the id corpus with an in-test closed form (`2^(2^d)`) and the event corpus with denotation distinctness through an independent path-sum walk. `GRID_N` is derived from `optrace::MAX_TRACE_OPS`, `fork` asserts the ceiling instead of clamping, and `fork_chain_raises_resolution_one_level_per_fork` meets the derivation's rate premise with equality on the worst in-support schedule. The law drivers expand from the single roster so a novel signature refuses to compile. I found no harness bug that masks a production failure.

The dominant issues are residue of three arcs the code has already moved past, each leaving prose or machinery that describes the earlier state. The semantic-oracle grid machinery went from a hand-picked clamping `GRID_N` to a derived, asserted one (08f7ccab, 7487be16, 28f6981e, f55c8276), and the test-side clamps, the "two levels per fork" rationale, the oracle-depth legs of the sampled sweep, and `Dyadic`'s dead `Ord` impl with its `GRID_N` premise all remain. The bridge gained `descend!` on its version walks only (faf3cd0a), while `recurse.rs` and before's `AGENTS.md` describe the whole bridge as guarded and the literal depth `0` defeats the guard's documented amortization. The exhaustive suite gained an exact-count totality pin (ebec26fe) without retiring the `> 20` floors it supersedes, and its grow-minimality pin still has no liveness floor. The validation index, which `AGENTS.md` names as the orientation map, opens with a totality claim that the tree does not meet. Two owner rulings (#53 on `exhaustive_deep`, #75 on the exhaustive point laws) settle questions the lenses raised, but neither rationale is stated at the code site.


## Positives

- The known-bad conviction pair (`rank_differential_convicts_the_cell_dropping_riemann_sum`, `worked_value_anchor_convicts_the_mirrored_embedding`; semantic_oracle/tests.rs:375-527) is the adequacy doctrine done right: each defective reference is committed behind an inverted assertion over a committed asymmetric family, and the mirrored-embedding test proves rather than argues that the pointwise differential is blind to a twin-substituted mirror, which is why the absolute-geometry anchor is necessary. Both are rostered by name in `surface_coverage::TRIPWIRES` (133-138) and resolved against nextest's live inventory by `tools/citecheck` from outside the crate.
- `corpus_counts_are_exact` (exhaustive/tests.rs:567-627) is the right shape of totality pin: the id count is derived in-test in closed form (`2^(2^d)`, the bijection with owned-cell subsets), and event distinctness is checked by an independent path-sum walk sharing no code with `node`, normalization, or the dedup key. Its doc names the exact shrink it catches.
- The grid discipline is spec-first: `GRID_N` is derived from `optrace::MAX_TRACE_OPS` (semantic_oracle.rs:80), `fork` asserts the ceiling instead of clamping (378-383) so an out-of-derivation trace fails loudly rather than aliasing, `fs_grid` argues explicitly against a silent cap while asserting the one representation bound (86-104), and `fork_chain_raises_resolution_one_level_per_fork` meets the derivation's rate premise with equality on the worst in-support schedule, deterministically.
- The law drivers expand both genres from the single `for_each_law_group!` roster (algebraic_laws/tests.rs:57-304, 336-394): a group with a known signature is driven with no wiring, a novel signature refuses to compile until an arm says how to feed it, and every assertion names the violated law. The roster's 18 signatures match both macros' arm sets exactly.
- The bridge lowers the impl's stored bits directly rather than through the public codec (bridge.rs:1-12), so structural agreement and codec correctness cannot share a failure mode; `read_ev` decodes the zigzag skyline stream independently of production decode and carries `Base`-typed heights so lowering is lossless for any magnitude; `to_oracle_version` normalizes once at the root.
- The semantic oracle's random section-4-valid `fork` and `event` policies (semantic_oracle.rs:303-407) test the one property no other instrument reaches, policy independence of the observable partial order, and the single-seed restriction is argued from the model (several seeds each owning `[0,1)` is not a valid configuration), not from convenience.
- The exhaustive suite drives every impl reading through public ops with the mutating and consuming contracts asserted as such: `check_id_pair_algebra` (172-229) pins the refused join's leave-self-unmodified and hand-back, and `without`'s `None` exactly when the remainder is empty; `check_ev_pairs` pairs every structural leg with a byte-equality leg so representation drift cannot hide behind structural agreement; the diagonal is included deliberately.
- The deep variant's leg split (verdict legs plus `tick` at the deep bound, structural legs at the small bound) is recorded with an owner ruling, drops the anonymous id once at lowering with the public-domain reason stated, and correctly names the nextest terminate budget as the reason to run under `cargo test`.

## Open questions for Finch

1. Stale-signature proptest seeds. `crates/before/proptest-regressions/testing/semantic_oracle/tests.txt` lines 8-9 record `seeds = 2, ops = [...]` for a keystone signature that no longer has a `seeds` input (the live property is `replay_matches_across_references(ops, seed)`), and line 7 records a single-parameter `p` shrink. They still replay as RNG seeds for every property in the file, so they hold nothing specific but cost nothing. The doctrine forbids stripping seeds; do you want stale-signature seeds pruned as a deliberate, named act, or left in place? Recommendation: leave them (the cost is nil and the rule is bright-line), but this is worth a one-line ruling so the next reader does not relitigate it.
2. The validation index's charter (testing-oracles-2, -28): a total map with a light liveness pin (a test that every gate/ci recipe and every `tests/*.rs` binary is named), rendered under `just docs-internal`; or a scoped, source-read page that names the justfile as the inventory of record? Recommendation: total and rendered, because before's AGENTS.md routes maintainers there and the rest of the crate holds every roster to the surface it enumerates.
3. `exhaustive_deep` (testing-oracles-24): has it completed since the #53 leg split, and do you want a `just exhaustive-deep` manual-tier recipe with a scheduled run? Recommendation: add the recipe, run it once detached, and record the completed wall time in the doc in place of the aborted-run chronology; if no completed run exists, the doc should say so.
4. The bridge's id walks (testing-oracles-3): guard all four walks uniformly with a threaded depth, or document the id side's exemption and correct recurse.rs and AGENTS.md? Recommendation: guard uniformly; the cost is nil and the inventory statements become true as written. Either way, is `descend!(0, …)` on recursive calls intended (probe every level) or a misreading of the macro's depth argument? Recommendation: thread depth as the macro doc prescribes.
5. The sampled grid sweep (testing-oracles-17, with -10): convert the keystone's silent clamp into an assert so every keystone case guards both sides, retire `grid_cap_is_never_reached`, and re-point its two roster citations to the chain pin; or keep the sweep trimmed to its function-space legs as the event-side guard? Recommendation: assert in the keystone and retire the sweep; a silent clamp is exactly the failure mode `fs_grid`'s doc refuses, and an every-case assert subsumes a 400-case sample of the same distribution.
6. The exhaustive point laws (testing-oracles-25): #75 kept them outside the roster; driving `PARTY_PAIR`/`VERSION_PAIR` totally over the small corpus is a different proposal. Do you want that measured, or the ruling's rationale stated at the site and the five kept? Recommendation: measure the total drive; if it fits the gate, adopt it and retire the five; if not, place the version half in the deep tier and state the reason at the site.

## Dropped

- [51] `Id`/`Event`, `id_res`/`ev_res`, `id_depth`/`ev_depth` differ only in codomain: taste; the doctrine prefers newtypes over synonyms, the depth walks traverse different oracle enums, and no cost was named.
- [46] "canary" sites: established test jargon, exempt under the vocabulary rule; "widened codec" at exhaustive/tests.rs:233 is anchored crate-wide (codec/int.rs, version/tests.rs), so no rename; the "keystone"/"honest"/"genuine" sites survive as testing-oracles-12.
- [15] as stated (`fs_grid`'s `debug_assert!` compiled out by the prescribed `--release` run): premise wrong, since the `--release` line belongs to `exhaustive_deep`, whose suite never calls `fs_grid`, and the dev profile keeps debug assertions on; the stylistic point survives inside testing-oracles-5.
- [44]'s "four instruments" hand count as a defect: the sentence enumerates four enforcing kinds and the atlas is described as enforcing nothing, so the count is consistent today; kept only as a hand-count mention inside testing-oracles-29.
- [1]'s proposal to retire `grid_cap_is_never_reached` outright: refuted by the refutation pass (the `ev_res` legs are the only committed event-side bound); reframed into testing-oracles-17.
- [37]'s tautological `MAX_TRACE_OPS <= GRID_N` assert as a standalone finding: history shows it was written as a guard against redefining `GRID_N`, and its arithmetic is stated inline; folded into testing-oracles-10 as "make it a const assertion".
- [18]'s alternative to dissolve the grow-minimality comparison in `check_tick`: contradicts ruling #53, which places the pin at the deep bound; only the floor survives, as testing-oracles-22.
- [5]'s framing of "cache-tiled" as a ghost reference to deleted code: history shows it names an experimental run that never landed; the chronology point survives as testing-oracles-19.
- [22]'s mapping of `event_partial_cmp_is_antisymmetric` to `order_antisymmetric`: the matching law is `partial_cmp_is_dual` (laws.rs:524-526); corrected inside testing-oracles-25, where the misnomer becomes a nit.
- [41]'s claim that "master harness" recurs at clock/tests.rs:562: the crate-wide grep finds the phrase only at bridge.rs:104 and the banner "master differential harness" at clock/tests.rs:192; corrected inside testing-oracles-6.
- Refutation's new observation on `event_dominates_local_and_advances`'s grid (`ev_depth + 1` but not `id_depth + 1` at tests.rs:303): correct as written and harmless; below the bar.
- Duplicates merged: [0]=[22] into -25; [1]=[37] into -17; [2]=[38] into -20; [3]=[19]=[55] into -28; [4]=[20]=[33]=[59] into -16; [5]=[29]=[32]=[61] into -19; [6]=[39]=[66] into -23; [7]=[31]=[35] into -11; [8]=[21]=[36]=[58] into -10; [10]=[23]=[40]=[62] into -1; [11]=[48] into -15; [12]=[34]=[65a] into -13; [13]=[17]=[27]=[44] into -29 (with -12 for the semantic-oracle sites); [14]=[24]=[43]=[56] into -3; [16]=[30]=[42]=[50]=[65] into -5; [25]=[53] into -24; [26]=[47]=[60] into -9.

<!-- source: final/tests-other.md -->
# Partition tests-other: The other integration suites: verdict matrix, board smoke, answer embedded, bench-judge roster, coincident span, doc hidden, fold skeleton, foreign reexport, forks max, fuzz seeds, stale state, superlinear tripwires

## Partition summary

The partition is thirteen files of instrument code and no production code: twelve integration-test binaries under `crates/before/tests/` and one `#[path]`-shared support module (`tests/support/fuzz_seed_set.rs`, the seed derivation that the `fuzz_seeds` example writes and the `fuzz_seeds` test byte-compares). They fall into three genres. The cross-kernel and resource pins run the library and judge what it does: `verdict_matrix.rs` (1530 lines) cross-checks every public answerer of the causal-relation question over a roster-derived operand pool, with liveness floors and two committed mutant twins; `coincident_span.rs` holds the `Span` clone-identity rungs live by scan parity; `fold_skeleton.rs` and `answer_embedded.rs` are two flatness criteria on populations the board does not drive; `amp_board_smoke.rs` runs the amplification board at a tiny scale and carries the merge's committed known-bad artifact; `forks_max.rs` and `stale_state.rs` pin boundary and model behavior. The roster and tamper pins scan source text: `superlinear_tripwires.rs` and `verdict_matrix.rs`'s own `_reads_inverted` roster hold the adequacy kernels by name, `doc_hidden.rs` and `foreign_reexport.rs` close two channels the surface-totality checks cannot see, and `bench_judge_roster.rs` pins the bench judge's expectation data. The corpus pins (`fuzz_seeds.rs` with the support module) hold the committed fuzz seeds to one derivation and to the contracts of four of the five fuzz targets.

Quality is good where the doctrine's order was followed literally. The verdict matrix derives its pool from an exhaustive match over `FamilyId`, asserts a budget derived from the roster count, demonstrates its verdict-class floors red on degenerate pools, and rosters by name the legs no mutant can reach. The board smoke's `merge_refuses_a_silently_shrunk_grid_for_every_family` is a real known-bad artifact swept over the axis the refusal discriminates on. `coincident_span.rs` asserts "a zero is a dead meter" before every comparison and documents why one leg pins divergence rather than direction. The seed derivation is a model of one definition feeding two consumers, with bit-level derivations beside every hand-authored non-canonical byte.

The defects cluster in four places. First, the two flatness criteria that landed together (`fold_skeleton.rs`, `answer_embedded.rs`) have no liveness floor: `growth()` returns 1.0 on a zero base counter, and the refutation pass's run shows the limb leg of the hull fold reading zero at both levels and passing; neither criterion has a committed known-bad kernel and neither is in any roster. Second, the text scanners are five hand-rolled variants of one algorithm the workspace already owns (`surface_scan::test_fns`), and their differences are live costs: the superlinear roster's scanner misses `pub fn` and cannot see `#[ignore]`, the inverted-twin roster's tree list already omits `wasm32-pins/`, the foreign re-export pin misses `pub use <dep>;`, and the doc-hidden pin counts one literal spelling. Third, the corpus gate holds the `fuzz_laws` framing by transcribed constants and the `fuzz_decode_ops` seeds by byte identity only, and the derivation's own comments describe a "concurrent" sibling version that `Clock::sync` has already made equal. Fourth, two instruments under-deliver what their docs claim: `weave_pair()` in the verdict matrix collapses by ITC normal form to `scatter_pair()`, so the Weave family contributes nothing to the pool, and the polarity-flipped twin is pinned per axis while the landing commit says both twins pin named legs. A tail of documentation drift (public `forks` docs silent or wrong at the boundary `forks_max.rs` pins, a "197 items" incident count, an unanchored "pincer/jaw" metaphor, "mint") rounds out the list.

Lines read: the thirteen partition files in full (3823 lines), plus the cross-reference sites cited under each finding (span.rs, clock.rs, party.rs, party/forks.rs, the two fuzz targets, surfacecheck/src/extract.rs and check.rs, meter/registry.rs, meter/board.rs and its family, worst, currency, and render modules, tests/meter.rs and src/meter.rs conventions, surface-scan/src/lib.rs, tools/citecheck, tools/benchjudge and its roster, benches/common/sidecar.rs, the justfile, and the validation index). Every file in the partition is test or instrument code; `tests/support/fuzz_seed_set.rs` is included by `#[path]` from both a test and an example.


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

<!-- source: final/tools.md -->
# Partition tools: The workspace verification tools: benchjudge, citecheck, covcheck, digestshare, doclint, fuelscape-claims, manifestlint, memwatch, mutantcheck, readme, testdoc, workflowlint, and their expected-value rosters

## Partition summary

The `tools/` directory holds the gate's build-free checkers and three committed expectation rosters. Five of the tools are judges over captured artifacts: benchjudge fits wall-time exponents over criterion medians and enforces `tools/benchjudge-expected.json`; citecheck resolves the coverage roster's citations against a nextest listing; covcheck holds the skyline kernel's lcov residue to `tools/covcheck-expected.json`; mutantcheck holds `.cargo/mutants.toml`'s exclusion patterns to the counts in `tools/mutantcheck-expected.json`; workflowlint holds every `uses:` and every fetch pipeline in `.github/` to committed or digest-pinned code. Four are lints over the tree (doclint, testdoc, manifestlint, readme), two are meters (digestshare over the wire-capture corpus, fuelscape-claims over the widget datasets), and memwatch is a shell watchdog the codegen-running recipes wrap. The justfile's `gate-lints` line and `ci` line are the rosters that decide which of these run where.

The tools share one architecture that the owner's doctrine asks for and that is unusual to see executed this consistently: a single `run()` judgment over raw inputs, liveness floors on every extraction and haystack, spelling-totality guards that turn an unreadable entry into a red rather than a skip, input errors that exit 2 rather than scoring, thresholds derived at their constants, and a `--self-test` at the head of each recipe that pins the tool's own red paths. benchjudge's self-test asserts `main`'s exit codes end to end, so a judge that stops failing cannot pass its own self-test; its ceiling class rides the bench sidecar and is asserted at every sidecar write, so no roster edit can move a ceiling. citecheck's fabricated-citation tripwire and spelling-totality denominator close the partial-rot hole that floors alone cannot see. mutantcheck refuses colored and polluted captures, pins the tool version, and never writes its own pin. These are done well and are recorded under Positives.

The dominant issues are three. First, two instruments still carry a vocabulary for accepting known failures: covcheck's `remediation` disposition and benchjudge's open `red` class, both empty of library entries today, both used as buffers in their history, and both surviving the owner's 2026-08-07 ruling (920bfabb2) that no such mechanism may exist even empty, because that ruling's excision inventory was board-scoped. Second, several floors are one coarse premise that cannot see partial darkness: covcheck judges only the files the lcov names (twelve kernel files have no entry and would vanish silently), mutantcheck never asks whether a mutant missing from the filtered listing is claimed by any pinned pattern (a file-level `exclude_globs` shrinks the campaign with every pinned count unchanged), benchjudge lets every unrostered board cell drift into SKIP, and digestshare's floor is conditioned on having seen a file at all (an empty or missing corpus exits 0). Third, the bench judge's stated unique failure class ("work no counter column can see") is also claimed by the fuzz-fit row of the validation index, and the committed tripwire is a plain machine-word quadratic that wasmtime fuel would also read red; the leg's residual class (cost that is not instructions) is neither named nor demonstrated. Smaller items are a correctness hole in workflowlint's interpreter recognizer (the most common real installer spelling, `| sudo -E bash -`, passes), two hand-rolled file walkers that read generated code and a foreign worktree into gate legs, a `ci` roster that omits manifestlint, and prose that cites code no longer in the tree.

I read all fifteen partition files in full (4845 lines: benchjudge 999, citecheck 856, workflowlint 558, doclint 434, mutantcheck 406, covcheck 335, readme 259, covcheck-expected.json 227, memwatch 219, manifestlint 168, testdoc 151, digestshare 108, mutantcheck-expected.json 61, fuelscape-claims 49, benchjudge-expected.json 15) plus roughly 700 lines of related sites (justfile, ci.yml, crates/before/tests/bench_judge_roster.rs, benches/common/sidecar.rs, benches/board.rs, benches/tripwire.rs, src/testing/validation_index.rs, .cargo/mutants.toml, tests/seed_liveness.rs, the fuzzfit harness docs). No partition file is a test file; each tool carries its self-test inline, and bench_judge_roster.rs is the one test file consulted. HEAD is 7440d1a3, two commits past the briefed 9e5784fb; `git diff --stat 9e5784fb HEAD -- tools/ justfile .cargo .github crates/before` is empty, so every file read is byte-identical at both. Procedural disclosures: I ran the Python tools on synthetic input under my scratch directory and one shell function in isolation (no cargo, just, build, test, or bench); those module loads wrote `tools/__pycache__/benchjudgecpython-314.pyc` into the untracked `tools/__pycache__/` directory another reviewer had already created, which I left in place.


## Positives

- benchjudge's self-test drives `main` end to end on synthetic criterion trees and asserts exit codes 0/1/2 (lines 702-923), so the `return 0 on red` mutation and the dark-tripwire path are both convicted in `--self-test`; both ceiling-laundering attacks are pinned as constructed inputs (857-884). The ceiling class rides the sidecar and is asserted at every sidecar write (sidecar.rs:175-180); the derived constants check out (I recomputed `fit_noise_band(1.7)` = 0.0876 and `fit_noise_band(1.3)` = 0.0523, as the comments state).
- citecheck's four layers (extraction floors, spelling totality with an entry-count denominator, the shadow guard, and a fabricated citation pushed through the live resolver on every run) are a model of a checker that cannot go dark quietly, and the rejected alternative is recorded with its reason (59-68).
- mutantcheck refuses colored and polluted captures by name before structural parsing, pins the cargo-mutants version, never writes its own pin, prints the observed table for a reviewed re-pin, and states its accepted residual (47-55) and its regex dialect boundary (57-65) with the reason a full-name pin was rejected.
- covcheck anchors entries on source text with offsets, fails closed on vanished, ambiguous, or out-of-file anchors, refuses invented categories, and defers DA=0 lines to the line pin in branch mode; the scaffolding lens reports every one of the 34 committed anchors resolves exactly once against today's sources.
- workflowlint's `USES_ANYWHERE` totality turns every unreadable `uses:` shape into a loud failure, its logical-line joining scans split pipelines whole, and its `swap` helper fails on fixture rot (322-325).
- doclint's include_str! rule carries a genuine rendering derivation (the `</p>` landing mid-SVG) that no external tool checks, with fixtures for every shape of invisible separator; doclint, testdoc, and workflowlint refuse a missing root and say why at the site, and doclint's self-test pins that red path too (353-367).
- readme's `RDME_VERSION` pin (2.1.0) and ci.yml:86's `cargo-rdme@2.1.0` agree, and the tool refuses a mismatch naming both versions and the install command; `check` never mutates the committed file.
- memwatch's process snapshot is numeric-only with a per-pid re-check of size and command before any signal, defeating argv injection and pid recycling, with the residual kill-by-pid race stated exactly (132-139); the swap-abort path freezes the tree before killing so no compile survives orphaned.
- manifestlint asks cargo for member discovery instead of walking directories, the idiom tools-30 asks readme to copy.
- The justfile's gate-streams verdict requires every launched stream to record ok or failed (409-413, 474-481), so a stream killed from outside fails the gate rather than vanishing.

## Open questions for Finch

1. The bench judge (tools-5): do you regard cost that is not instructions (bulk-memory operations fuel prices as one unit, native codegen, memory hierarchy) as a failure class worth an instrument at the board's byte scales? Recommendation: decide this first, because tools-6, tools-7, and tools-12 and the roster test all go away under exit (b); if the answer is yes, exit (a)'s tripwire is the acceptance artifact.
2. covcheck's `remediation` disposition (tools-14): excise, or keep with a positive ruling recorded at the declaration? Recommendation: excise; the roster is empty and the 08-07 ruling's principle is general.
3. The bench roster's `red` class (tools-6): bind it to a sidecar-declared tripwire set? Recommendation: yes, and rename the key so an awaiting-cure entry needs a schema change.
4. digestshare (tools-3, tools-19): the tool measures rumors' wire corpus and lives in tools/. Recommendation: fix the floor regardless (tools-19); move the leg to conveniences unless the ratio gets a committed consumer.
5. covcheck's scope includes the twelve tests.rs siblings under skyline/, so uncovered test-helper lines are held to the same pin as kernel lines. Recommendation: state the decision at the scope declaration when landing tools-16; keeping them in is fail-closed.
6. Three mutant exclusions pin file:line:col (mutantcheck-expected.json:12, 28, 56; mutants.toml:117, 142, 150) and re-redded three times on 2026-08-18. The header at mutants.toml:39-46 accepts the fail-open drift knowingly, so this is not filed as a finding. Recommendation: where extracting the equivalent occurrence into a named helper is cheap, do it so the pattern can anchor on `in <fn>`; otherwise leave the ruling as is.
7. Should tools/ grow a small shared helper module (file enumeration for tools-22, self-test scaffolding)? Recommendation: yes for file enumeration; otherwise each tool stays self-contained.
8. The vocabulary rulings pending in the rumors review ("honest", em-dashes) should apply to tools/ in the same sweep (tools-8, tools-29).
9. Procedural: my module loads wrote `tools/__pycache__/benchjudgecpython-314.pyc` into the untracked `tools/__pycache__/` directory another reviewer had created. I left it in place; the directory is a deletable byproduct.

## Dropped

- Lens [15] (tests/bench_judge_roster.rs mirrors two constants): refuted. The roster test is the doctrine's prescribed tamper-evidence form and the only gate-cadence confrontation of a `just all`-cadence artifact (the JSON is read only by `just bench-judge`, TEXT_CEILING_CELLS is asserted only when a bench runs); moot only if the judge is retired under tools-5.
- Lens [32] (the tripwire recipe never hands `--tip` to the judge): refuted. justfile:857 ends with `--tip $(git rev-parse HEAD)`; the lens's quote was truncated. The secondary observation (a dirty-tree run is stamped with the clean commit) is below the bar.
- Lens [9] (three line:col-pinned exclusion patterns re-red the gate on line drift): deliberate and documented at .cargo/mutants.toml:39-46, which accepts the fail-open drift and puts refactoring first in its own ladder; moved to open question 6.
- Lens [1]/[25]: duplicate of tools-4. Lens [3]/[22]/[33]: duplicate of tools-14. Lens [4]/[27]/[37]/[28]/[57]: duplicate of tools-19. Lens [5]/[39]/[53] and the ghost half of [31]: duplicate of tools-9. Lens [6]/[21]: duplicate of tools-6. Lens [8]/[24]/[34]: duplicate of tools-22. Lens [10]/[36]/[56]: duplicate of tools-23. Lens [11]/[35]: duplicate of tools-30. Lens [12]/[45]: duplicate of tools-18. Lens [13]/[26]/[54]: duplicate of tools-1. Lens [17]/[58]: duplicate of tools-27. Lens [18]/[44]: duplicate of tools-24. Lens [19]/[31]/[40]: duplicate of tools-12 (reframed to deduplication per d2a9d04e). Lens [23]/[51]: duplicate of tools-16. Lens [42]/[59]: duplicate of tools-10. Lens [46]/[47]: duplicate of tools-8.

<!-- source: final/version-core.md -->
# Partition version-core: Version: the event tree type, owned versions, ticks, hull traffic, and the version test suite

## Partition summary

`Version` is a single `codec::Bits` (a refcounted, marker-padded canonical skyline stream) and every operation on it is a one-line door into `skyline::*`: `tick`/`ticks` into `fill`, `rank`/`distance`/`lag`/`min_ticks` into `query`, comparison into `sweep::causal_cmp`, equality into `codec::canonical_eq`. The lattice kernels sit in `version.rs` as five short-circuit ladders (`join_view`/`join_refs`, `meet_view`/`meet_refs`, `span_refs`) over `skyline::emit`; the n-ary folds (`join_all`, `meet_all`, `span_all`, `Sum`, `FromIterator`) run `crate::fold::balanced_reduce` with a `DedupRuns` adapter that collapses adjacent clone runs by pointer identity. The operator matrices (`|`, `&`, `^`, `/`, comparison) are macro-generated from one source each. `own.rs` holds `OwnVersion`, the lazy projection view whose comparison matrix routes every cell through the fused masked co-walk; `ticks.rs` holds `Ticks` (an unbounded count over `codec::Base`) and its `Limbs` iterator; `hull_traffic.rs` holds the feature-gated per-rung counters for the span ladder. The test files are `version/tests.rs` (2402 lines), `version/own/tests.rs` (144), and `version/ticks/tests.rs` (122); the production files total 2404 lines. All seven files were read in full with line numbers: 5072 lines.

The production code is clean under every lens: no recursion, no unsafe, every `expect` guarded by the receiver seed, the one `unreachable!` a genuine one-line proof resting on canonicality with the premise pinned (`eq_matches_causal_walk`), and both n-ary folds keeping their combine matches total rather than asserting on the arm the counter's weight discipline excludes. The maintainer comments at the two places a reader stumbles (`Version::new`'s static-not-const, `DedupRuns` holding a clone rather than an address) state the why. No correctness defect survives; the four lens reports, the refutation pass, and the history pass agree on that.

The dominant issue is machinery that outlived the `Batch` type (removed 2026-07-27): `join_view`/`meet_view` still take a foreign `&codec::Bits`, but every remaining caller passes a `Version`'s `.view()`; their bodies duplicate the `_refs` ladders rung for rung, held in step only by prose, and that duality in turn justifies the three-strategy `binop_matrix!`, the `view` parameter on `balanced_fold`, and the `lo_view`/`hi_view` fields in span algebra. Beside it, `span_all` copies `balanced_fold`'s four-arm dispatch (a third copy lives in span algebra), and the join/meet encoded-size subadditivity lemma that the folds' auxiliary-space bounds and rumors's window budget both rest on is derived only in test prose. The remainder is documentation and test hygiene: a stale module doc over a 2402-line test file hosting the `Rank`/`Ranked` suites, one duplicated proptest, five orphaned proptest seeds, an assert message pointing at prose a later commit excised, hand-transcribed measurements in a test doc, and a handful of public-doc slips (a wrong intra-doc link, a ghost example name, a sentence copied from `meet_all` without adaptation, two owner-authored paragraphs with syntax and likelihood problems).

Vocabulary and register items (`mints`, `honest`/`historical`, unanchored `door`, em-dashes in `//` comments) are real but crate-wide: the governing rules landed on 2026-08-10, after most of this prose; they are listed once with the partition's sites for a single sweep rather than as partition defects.


## Positives

- `DedupRuns` holds a clone rather than a raw address, with the reason stated (version.rs:1227-1229): a freed buffer reused at the same address mid-iteration cannot masquerade as a duplicate. Exactly the invisible-state hazard a comment should carry. (Verified by reading.)
- `Version::new`'s static-versus-const comment (version.rs:121-131) names the observable it protects and the laws that pin the constant; the twin at `Party::seed` (party.rs:122-129) matches word for word. (Verified.)
- The one `unreachable!` in the partition (version.rs:994-996) is a one-line proof resting on canonical uniqueness, and its premise is pinned by `eq_matches_causal_walk` (tests.rs:459-479); the refutation pass correctly held it against the mutants roster's sanctioned disposition for structurally-required unreachable arms. No assert, expect, or panic message in the partition carries an em-dash. (Verified.)
- Both n-ary folds keep their combine matches total on the `(Input, Merged)` arm with the commutativity argument stated (version.rs:675-679, 817-821), and the argument is correct against `fold.rs:41-80`: a weight-0 input is only ever the newest item in the counter or the last, right-hand group in the drain. The duplication is the finding, not the decision. (Verified against fold.rs.)
- `assert_cmp_cell` (tests.rs:33-61) is generic over `(L, R)` so each call resolves to exactly one impl cell, and its doc names the tell: a delegation cycle overflows the stack in the test rather than diverging in production. (Verified.)
- `own/tests.rs:1-13` maps the negative space: it says where the coherence and differential properties live so a reader knows where not to look here. (Verified.)
- `own.rs:133-141` explains why the mirror orientation drives its own co-walk rather than reversing the first (totality over every mask arrangement from the public surface), with the antisymmetry pinned by a named differential. (Verified.)
- `limbs_respell_the_count` (ticks/tests.rs:81-109) checks the exact-size contract at every step of the drain and the fused-past-the-end contract, not only at construction. (Verified.)
- `hull_traffic::record` compiles to nothing without the feature and is callable unconditionally (hull_traffic.rs:108-118), so the production ladder carries no cfg noise; the module doc earns the counters' existence by naming what no per-operation envelope can see (the caller's mix). The lenses report a per-rung liveness test at meter/tests.rs:565-622 and a consumer in rumors's tree tests; not re-read here.
- `deep_spine_marginal_cost_is_three_bits_per_level` (tests.rs:2375-2402) is a closed-form two-scale witness, the instrument shape the standard asks for; `boundary_arity_fan_folds_match_the_sequential_fold` (2315-2373) deliberately salts the fan with an adjacent clone run and an empty operand so three mechanisms meet in one deterministic construction. (Verified.)
- The join/meet subadditivity lemma is derived term by term in the tree (meter/tier2/tests.rs:328-346) with the tight constant identified and its equality case named; only its promotion to production prose is missing (version-core-5). (Verified.)
- The `Version` type doc's operation table and explicit "Comparison is **partial**" paragraph (version.rs:48-60), and `tick`'s which-one-do-I-use contrast with `Clock` (161-166), are the right altitude for a library user. (Verified.)
- The claims lens reports the `ticks` zero-count early return (version.rs:214-216) is load-bearing, not an optimization: `grow.rs:505-508` debug-asserts a nonzero splice. Not re-read here.

## Open questions for Finch

1. Was the `own`-strategy cell's saved refcount clone/drop pair ever measured to matter, or is anything pinning refcount traffic? Nothing I found does (the scan pins measure rungs). Recommendation: treat version-core-11's collapse as free and land it; if a reading is wanted, take it before, not after.
2. Does `emit::Hull.relation` have a planned consumer (the comparable-pair fast path 70eb67ab1 named)? Recommendation: if not, remove the field, the directions fold, the three emit-test assertions, and the `debug_assert!` together (version-core-13); if so, replace the assert with that consumer when it lands.
3. Moving the `Rank`/`Ranked` suites out of `version/tests.rs` (version-core-27) crosses into the rank partition. Recommendation: this note carries the module-doc fix; the move is a coordinated change for whoever owns rank.rs, with this finding as the pointer; citecheck's bare-name matching makes it roster-free.
4. Orphaned-seed disposition (version-core-36): prune, or retain as an RNG corpus? Recommendation: prune with the dissolved property named in the commit, and extend `tests/seed_liveness.rs` to match shrunk parameter names against live signatures so the class cannot recur; the "never strip" rule is about seeds a failure produced, which these no longer are.
5. `# Errors` uniformity (version-core-14): `Party::decode` shares `Version::decode`'s omission while the rank doors carry itemized sections. Recommendation: one crate-wide pass, and consider having `doclint` require the section on every `pub fn` returning `Result`.
6. Is `door` intended crate vocabulary (208 uses, no definition)? Recommendation: define it once in lib.rs if kept; otherwise the sweep replaces it per site (version-core-17).
7. ticks.rs:167 links `[`From<u64>`](Ticks#impl-From<u64>-for-Ticks)`; whether that fragment matches rustdoc's generated anchor cannot be checked without a docs build, which this review could not run. Recommendation: check once under `cargo doc`, or link `[`From`]` and let the sentence name `u64`.
8. `Version::concurrent` is generic over `V: PartialOrd<Version>` and so accepts an `OwnVersion`, but no law, differential row, or test instantiates it that way. Delegation is trivially correct; the question is whether the surface roster should record the view instantiation as a reachable cell.
9. Does rumors's long-lived version storage pay the retained-capacity slack (version-core-15) at a material rate? The tick and hull resident rows would answer it; if the stored versions are mostly tick outputs, a spilling tick's `Vec` doubling puts the resident cost near 2x the encoding.
10. From the dropped observation on the span ladder's order: the rung mix from rumors's reconciliation workload is already readable through `meter::span_traffic` in the tree tests. Recommendation: record one reading in a decision record so the comparable-first order rests on a number; a dependence-sweep item, not a partition change.

## Dropped

- `span_refs`'s `Some(Equal)` arm should hand back a value instead of panicking ([57]): refuted. The arm is the mutants roster's sanctioned shape for a structurally required unreachable branch, `causal_cmp` returns `Equal` only on byte-equal streams (pinned by `eq_matches_causal_walk` and the verdict matrix), and the crate's own rule wants a canonicality bug surfaced, not masked.
- Concurrent hulls pay a classifying sweep then the emitting walk ([56]): reframed to an observation. The design is documented at version.rs:956-967 and metered by `hull_traffic`; the reader's own resolution is "no code change". Carried as open question 10.
- `Ticks` text-I/O bound stated twice ([9]): merged into version-core-22.
- Wrong `span_all` link ([8], [43] part 1): merged into version-core-7.
- Ghost `alice` ([43] part 2): merged into version-core-6; `v / &p` spelling ([43] part 3): merged into version-core-3.
- Duplicate `|=` tests ([22], [47]): merged into version-core-28.
- Module doc omits half the file ([24]): merged into version-core-27.
- Provenance ratios in prose ([25], [53]) and the single-scale ceiling ([42]): merged into version-core-32.
- `trace_ticks` doc omits `Ticks` ([38], [45], [55]): merged into version-core-29.
- 64-bit literals ([40], [46], [54]): merged into version-core-34.
- Byte-compare rung labelled `O(1)` ([50]): merged into version-core-10.
- `shape`'s likelihood paragraph ([51]): merged into version-core-9.
- Subadditivity lemma unstated at the fold ([41]) and unpromised at the door ([48]): merged into version-core-5.
- `mints` ([17]), moralized adjectives ([29]), em-dashes ([30]), `door` ([31]): merged into version-core-17 as one crate-wide sweep item.
- Typographic bundle ([39]): version-core-3, with the own.rs:12 spelling and the hull_traffic.rs link added.
- Inert rustdoc links ([33]): version-core-4, with the version.rs:1349 site added.
- Hand-listed `reset` variants ([6]): version-core-26, with `snapshot` added.
- `Sum O(N)` per-summand term ([44]) and missing amortization ([59]): merged into version-core-23.

<!-- source: sweeps-final/api-audit.md -->
# Sweep api-audit: Public API surface audit of before and suanpan

## Method and coverage

This pass disputes the api-audit sweep's report finding by finding. For each of
the sweep's 23 findings I opened the cited lines with `awk`/`cat -n`, checked
the claim against the code, and looked for a recorded rationale in git history
(`git log -S`, `git show`, `git blame`), the before-prefixed `.agent-notes/`
directories, `crates/before/AGENTS.md`, and the header of `.cargo/mutants.toml`.
Mechanical checks run:

- Greps over `crates/before/src` and `crates/suanpan/src`: `must_use` (one hit,
  `party/ops/build.rs:39`), `non_exhaustive` (none), `# Errors`/`# Panics`
  (listed below), `Overlap` producers (only `clock.rs`), the six typos, and the
  words `mint`/`honest`.
- Dependency sources read, not run: `thiserror-impl-2.0.18/src/prop.rs`
  (the `source_field` rule), `dashu-int-0.5.0/src/ubig.rs` and `src/repr.rs`
  (`as_words` returns the normalized word slice), and the Rust 1.88 `core`
  sources for `Option`/`Result` (`#[must_use]` is on `Result` only).
- Rendered docs: the sweep's own `cargo doc` output under `target/doc/before/`
  (files dated Sep 1 22:43, produced at this commit) was re-inspected for
  `all.html`, `iter/index.html`, `struct.Clock.html`, `struct.Span.html`, and
  `struct.Version.html`. I did not re-run rustdoc.
- Git history: `crates/before/AGENTS.md` (blame and log), the
  `implementation` module (`67970b75` added, `22cdfbe1` deleted), the "Law of
  Disjointness" phrase (`46184a6a6` added, `a431eaf1d` removed), and the
  suanpan "no from-value constructor" sentence (`7ab518ce1`).

No cargo, just, test, or bench command was run; the two permitted test
invocations were not needed. No file under the repository was modified.

What this pass could not see: I did not audit the full private-doc footprint of
"mint" (it recurs across the skyline and meter modules, outside the public
surface this sweep covers), and I did not re-derive the surface-totality
pincer's item list beyond reading the rendered `all.html`.

Outcome: every sweep finding survives at its cited lines. Two are reframed with
factual corrections (api-audit-1: `Option` carries no `#[must_use]`, so
`Party::without` is not covered as the sweep said; api-audit-5: rustdoc does
form the `Clock::forks` link, with the stray backtick inside the link text).
One severity is lowered (api-audit-2). Nothing is dropped outright.


## Positives

- Linearity is pinned at the type definitions with `static_assertions`
  (party.rs:74, clock.rs:62), and the entire `|` surface a `Clock` participates
  in is pinned in both directions (clock.rs:1072-1082), with the reason for
  each rejected cell stated at the definition. I read all three pins.
- `Split::size_hint` (party/forks.rs:65-73) handles a `u64` count on a target
  whose `usize` is narrower by returning `(usize::MAX, None)`, the standard
  spelling, with the reason in a two-line comment: correct at all scales,
  stated where it lives.
- `Rank::decode`'s `# Errors` section (rank.rs:435-445) and `Span::decode`'s
  (span/wire.rs:79-85) are variant-by-variant and name the flush-byte case;
  `Rank::encode_to` (rank.rs:403-406) shows the one-line form for writers.
  They are the template api-audit-8 asks the rest of the surface to meet.
- tests/forks_max.rs pins the saturation boundary in both profiles, holding
  `len()` and `size_hint()` to an exact value rather than "does not panic".
- tests/doc_hidden.rs and tests/foreign_reexport.rs pin the hidden and
  re-exported surfaces by name; the rendered docs confirm no foreign re-export
  in before and exactly the documented `UBig` re-export in suanpan.
- Every fallible reunion hands identity back rather than dropping it:
  `Party::join`/`Clock::join` return the operand in `Err`, `join_all` returns
  the overlapping set, `Forks`' `Drop` rejoins unconsumed shares, and
  `sync`/`sync_all` leave every clock untouched on overlap (clock.rs:322-336,
  377-390, party/forks.rs:134-148).
- The text, literal, and byte doors each document the linearity hole at the
  entry (`Party::from_str` at party.rs:767-768, `Clock::from_str` at
  clock.rs:928-929, `Clock::decode` at clock.rs:768-772), and
  `dangerously_alias` states the exact protocol it exists for.

## Open questions for Finch

1. The brief lists `claims` among suanpan's public API; at this commit it is
   `#[cfg(test)] mod claims;` (crates/suanpan/src/lib.rs:364-365), a private
   test module. Was the brief's premise stale, or is exposing the roster (as
   before does with `surface` under `meter`) intended?
2. Should `Version::new`/`Party::seed`/`Clock::seed` be `const fn`? `Rank::ZERO`
   and `Ticks::ZERO` are consts. The constructors build from a `static` byte
   via `codec::Bits::from_canonical(bytes::Bytes::from_static(..))`;
   `Bytes::from_static` is const, and `from_canonical` (codec/bits.rs:139-146)
   is a plain `pub(crate) fn` whose only body beyond the struct literal is a
   `debug_assert!(padding_is_canonical(&bits), ..)`, so constness hinges on
   making that predicate const or adding a const static-only door. The
   comments at party.rs:125-127 and version.rs:126-128 explain why the buffer
   is a `static` rather than a `const` (shared address); a `const fn`
   returning a value built from that static is compatible with that.
3. `Query` deliberately has no `PartialEq` (query.rs:211-212, a private
   comment); `Floor`/`Ceiling` likewise. If the reason is that two spellings
   can denote one predicate, is a structural `PartialEq` on the normal form
   (the `and` merge maintains one) acceptable, or is the absence meant to keep
   callers from treating queries as values? api-audit-9 asks for the reason to
   be written down wherever the answer lands.
4. `Clock` and `Span` expose no O(1) encoded length (`Party`/`Version` have
   `as_bytes().len()`); a protocol sizing a frame must `encode()` (allocating)
   or write through a counting writer. Is an `encoded_len()` on the composite
   types wanted, or is `encode_to` into a counting writer the intended spelling?

## Dropped

- Sub-claim of sweep finding [0] that `Party::without` is "already covered by
  `Option`/`Result`'s built-in must_use": dropped; `core::option::Option` has no
  `#[must_use]` (only `Result` does), so `without` needs its own attribute. The
  finding survives as api-audit-1 with that correction.
- Sub-claim of sweep finding [4] that "the link does not form" and "only the
  first link [is] present": dropped; the rendered `iter/index.html` shows both
  links formed, the second with a literal backtick inside its text. The finding
  survives as api-audit-5 (nit) with the corrected symptom.
- No whole finding was dropped: every cited excerpt matched the file at the
  cited lines, and no recorded rationale in git history, `.agent-notes/`,
  `crates/before/AGENTS.md`, or `.cargo/mutants.toml` contradicted a claim.

<!-- source: sweeps-final/clippy-pedantic.md -->
# Sweep clippy-pedantic: Pedantic and nursery clippy lints over before and suanpan, judged

## Method and coverage

The sweep ran one clippy invocation with `-W clippy::pedantic -W clippy::nursery`
over the workspace members `before`, `suanpan`, and `surface-scan` (log:
`scratchpad/before/sweeps/clippy-pedantic.log`, 996 KB; the log records the
lint groups through clippy's own "implied by" notes but not the command line).
Its parser (`scratchpad/before/sweep-clippy-pedantic/parse.py`) produced
`hits.json`, which I re-read: 2175 unique hits across 60 lints, 2116 in
`before`, 57 in `suanpan`, 2 in `surface-scan`; the four largest lints
(`use_self` 1020, `redundant_pub_crate` 284, `cast_precision_loss` 116,
`missing_const_for_fn` 113) match the sweep's summary.

This pass disputed the sweep's eight numbered findings. For each I opened every
cited site with line numbers, grepped the use sites, and looked for a recorded
rationale in git history (`git blame`, `git log -S`, `git log -L`), the
before-prefixed `.agent-notes/` directories, `crates/before/AGENTS.md`, and the
`.cargo/mutants.toml` header and roster. I ran no cargo, just, or build
command: the two permitted `cargo nextest` invocations went unused because no
committed test exercises any of the disputed claims (`PackedBuilder` appears in
no `tests.rs`; `Shl<i32>` and `emit_offset` have no dedicated tests), and I may
not add one. Every finding below is therefore assessed by reading, and each
correctness or performance entry carries a construction the owner can run.

What this pass could not see: rustdoc's rendered output (finding 5's rendering
claim rests on CommonMark's rule that adjacent code spans are separate inline
nodes, not on a rendered page); the detached workspaces (`fuzz/`, `fuzzfit/`,
`wasm32-pins/`, `surfacecheck/`, `before-fuelscape`), which the sweep's
command did not lint; and the wasm32 target, on which the `u64 -> usize` casts
the sweep read would narrow.

All eight findings survive. Four are narrowed (2, 3, 5, 8), recorded in their
Verification lines and again under Dropped.


## Positives

- Every production-path narrowing cast the sweep read is bounded in the same
  function by a named constant or an explicit early return (`n <=
  SMALL_CODE_BITS` at build.rs:96-100 read here; the remainder sweep-reported).
  None is reachable from decoded bytes or caller input with a lossy result.
- `Shl<u64>` / `Shr<u64>` for `Base` (codec/base.rs:466-500) are the model for
  a width conversion in this crate: `usize::try_from(..).expect(..)` on the side
  that cannot be total, a value-preserving clamp on the side that can, and the
  totality argument written at the site (read here).
- `PackedBuilder::read_bits` (build.rs:287) checks its untruncated range at
  entry; the pattern finding 2 asks for already exists one function above.
- The `Open` token (party/ops/build.rs:36-40) pairs `!Clone` with a
  `#[must_use]` reason string and states, at the declaration, what the borrow
  checker thereby prevents (read here).
- `tick_walk_floors` (floors.rs:788) states an integer floor as an integer
  multiply with saturation; finding 8 only asks the scan floor to match it.
- `prescan.rs:439` already takes `&Signed`; finding 3 aligns its sibling.
- Sweep-reported, not re-verified here: the `suspicious_operation_groupings`
  and `nonminimal_bool` hits in laws.rs are false positives on symmetric law
  predicates written in the law's own words; worst.rs:133-137 documents why
  exact `f64` equality is correct before the two `float_cmp` sites; default
  clippy passes under `-D warnings` with 28 single-site `#[allow]`s.

## Open questions for Finch

1. Is the feature-gated `pub mod meter` (lib.rs:438-439) held to the stable-API
   rule? The `encoded_bits` re-homing commit (05d87e1b) treats the meter feature
   as "the instrument surface" distinct from "the unconditional public API".
   Finding 8's constant-type change is owner-gated only if the answer is yes;
   the answer also settles how future sweeps classify meter-surface changes.
2. Finding 4: do you want the type-level `#[must_use]` on `Party` and `Clock`
   (which also fires on a discarded `dangerously_alias()` and on any
   statement-position constructor), or only the method-level attributes on
   `fork` and the pure combinators? No site in before, suanpan, or rumors
   trips either today.
3. Finding 1: should unsuffixed-literal shifts on `Base` keep compiling
   (keep the impl, make its guard total), or should the two test spellings be
   suffixed and the impl deleted? The second is the smaller tree.

## Dropped

- Narrowed, not dropped: finding 2's claim that `read_bits` shares the
  truncated-check pattern. Its `debug_assert!` at build.rs:287 checks the
  untruncated `pos + u64::from(n) <= self.len()`; only `bit_at` shares the shape.
- Narrowed, not dropped: finding 3's "a wide magnitude clone is a big-integer
  allocation" generalized to every consumed site. `Int` is `Small(u64) |
  Wide(Base)`; the allocation occurs only for wide magnitudes.
- Narrowed, not dropped: finding 5's "6 in public rustdoc". party/tests.rs:629
  is test code and place.rs:51 is a private module doc; four before sites and
  two suanpan sites are public.
- Narrowed, not dropped: finding 8's framing against the `f64` ceilings. The
  sibling floors are `u64` and the integer form exists at floors.rs:788; the
  finding is stronger than the sweep stated, and owner-gated because the
  constant is `pub`.
- The sweep's "not adopted" lint groups (use_self, redundant_pub_crate,
  missing_const_for_fn, cast_precision_loss, option_if_let_else, and the rest
  of its open-questions list) were not re-litigated here; none is a finding,
  and the sweep's reasons stand as written.

<!-- source: sweeps-final/deps.md -->
# Sweep deps: Dependency, feature, and build-script audit

## Method and coverage

This is the verification pass over the deps sweep's sixteen findings, at
commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764 on a clean tree. For each
finding I opened the cited sites with line numbers, grepped use sites, and
checked the recorded rationales: `git log`/`git show` on the relevant
commits, `.agent-notes/2026-08-13-before-fuelscape-rustdoc/` (the build.rs
design), `.agent-notes/2026-08-04-perf-probe/` (the probe examples),
`crates/before/AGENTS.md`, and the deny.toml, justfile, and
rust-toolchain.toml headers. Mechanical checks run: `git ls-files --
'*Cargo.lock'` (six lockfiles), per-lock grep of `wasmtime`, `dashu-int`,
`borsh`, `bytes`, `thiserror` versions, the RustSec advisory files for
RUSTSEC-2026-0268/0269 in `~/.cargo/advisory-db`, a scan of the root
Cargo.lock's dependency arrays for what reaches rand 0.9 and thiserror 1,
grep counts of `suanpan::UBig` and `dashu_int::UBig` spellings, grep of
`(PROG|COV)-[0-9]+` across the tree including `.agent-notes/`, grep of
`derive(Serialize|Deserialize)` across before, `git ls-files` sizes of
`results/`, `reference/`, `scripts/`, and the dashu-int 0.5.0 feature table
from the registry source. Two external documents were fetched to settle
mechanism claims: cargo-deny's bans configuration page (the
`multiple-versions-include-dev` default) and the rustc book's check-cfg
page (whether command-line `--cfg` values are checked). CI history was read
with `gh run list`/`gh run view` for the last six `ci` runs on main and the
failed job's log.

Not run: cargo, just, cargo-audit, cargo-deny, or any build; the brief
forbade them, and no finding needed a test to settle. The two constructions
below that would settle by running (the build.rs rerun gap and the
`expect("validated")` panic) require editing files, so they stay unexecuted
and are stated as constructions. What this pass cannot see: whether
`cargo audit` would flag anything beyond the wasmtime entry in the omitted
lock, and whether `cargo deny` with dev duplicates included would report
anything beyond the four crate pairs found by lock inspection.

All sixteen findings survive; three are reframed (deps-4 becomes an
owner-gated design question against a recorded decision, deps-6's
operational edge is settled by CI history, deps-7 is marked owner-gated).
One out-of-scope observation from the CI log is recorded under open
questions with an explicit disposition.


## Positives

Verified by reading at the cited sites in this pass:

- dsi-bitstream is taken with `default-features = false, features = ["alloc"]`
  (Cargo.toml:51), and crates/before/src/codec/dsi.rs:1-30 states the exact
  trade: the production reader from the library, the writers in-house, and
  the reason the library's own `read_gamma` is not used (a `debug_assert`-guarded
  2^64 cap that would mis-decode in release). This is the model for how a
  dependency boundary should be documented.
- dashu-int is narrowed to `std` in the workspace table (Cargo.toml:50), and
  suanpan's crate docs (lib.rs:296-297) state that the re-exported `UBig`
  makes the dashu-int major a public-API fact and its bump a breaking change.
- borsh impls are hand-written against `borsh::io` with no `derive` feature
  (borsh_impls.rs:13-14); the fuzz workspace's manifest explains each
  transport dependency's role in one sentence (fuzz/Cargo.toml:29-35).
- stacker is test-only: recurse.rs gates `grow` and `descend!` behind
  `cfg(test)` (lines 100 and 118), matching crates/before/AGENTS.md:24-25.
- surfacecheck pins `rustdoc-types = "=0.59.0"` with the reason at the pin
  (Cargo.toml:23-29) and refuses a mismatched `format_version` naming both
  numbers and the bump procedure (main.rs:71-85); rust-toolchain.toml states
  why it pins and how to bump (lines 6-18).
- tools/manifestlint holds every member manifest to the workspace table with
  a self-test, and the root Cargo.toml states the inheritance convention
  beside the table (lines 10-16).
- The `required-features` entries on amp_board and code_study
  (crates/before/Cargo.toml:110-125) turn a mis-featured invocation into a
  cargo error with the reason stated; the self-dev-dependency excludes
  limb-meter and scan-meter so bench builds stay unmetered (lines 49 and
  80-83).
- suanpan's footprint is one normal dependency, two dev-dependencies, one
  feature, with claims.rs behind `cfg(test)` (lib.rs:364-365).
- The design note for the fuelscape islands records the per-consumer build
  cost of build.rs explicitly (§4) rather than leaving it implicit; that is
  what let this pass reframe deps-4 as a decision to revisit rather than an
  oversight.

Reported by the sweep and not re-read here: the fuzzfit bands' `PINNED_RUSTC`
and wasmtime lock-pin pairing (bands.rs:312-321), and the `oracle` feature's
bench/test-only claim across benches and the perf_probe example.

## Open questions for Finch

1. Dev-only duplicates in the deny leg (deps-2): include them (option a,
   roster rand 0.8/0.9 and thiserror 1/2 with holdouts) or state the
   exemption (option b)? If the rand convergence is wanted, it changes
   rumors' normal `rand` dependency (Cargo.toml:142) and reseeds every corpus
   drawn through `gen_range` (rand 0.9's Uniform integer sampling is
   documented as breaking value stability): which pinned artifacts (the
   fuzzfit fuel bands, benchjudge-expected rosters, any snapshot from
   `gen_range` draws) would re-pin? Recommendation: option (a) now, the rand
   convergence as its own owner-ruled commit.
2. wasm32-pins wasmtime bump (deps-1): the advisories concern WASI paths the
   harness does not compile, so the fix is a patch bump either way; confirm
   with `just wasm32-pins` that 47.0.4 leaves every pin's outcome unchanged.
3. build.rs direction (deps-4): committed derived islands emitted by the
   compactor and held fresh by `fuelscape-verify`, or the per-consumer
   build-time formatter as the accepted cost of a single JSON source? The
   deps-3 and deps-5 fixes stand regardless.
4. Out of this sweep's scope, recorded here because it was observed and
   disposed of nowhere else: the latest `ci` run on main at the briefed
   commit (run 33567211421, 2026-09-01) failed in the `coverage` job at
   `just coverage-kernel (line, stable)`, with `before::meter
   masked_cmp_hole_envelope` panicking at crates/before/tests/meter.rs:6929:
   "masked_cmp_hole: peak heap 1156 B exceeds the pinned envelope 480 B
   (input 755 B)" (MEASURED line: input_bytes=755 peak_heap=1156 segments=0
   limb_ops=0 touches=14 scan_bits=6028). The `ci` and `instruments` jobs
   passed, and the previous main run (33560347645) passed all three jobs, so
   the peak-heap envelope trips only under llvm-cov instrumentation and only
   on this run. Not investigated; it belongs to the metering partition. The
   captured log is at
   <session scratchpad>/final-sweep-deps/ci-failed.log.
5. indicatif for the space_consumption example: a progress bar for a
   minutes-long paper-reproduction run that produces a committed figure, at
   the cost of its transitive set in every before test build. A judgment
   call the sweep declined to make for you; I would keep it.

## Dropped

- Sweep open question "Does the CI coverage job's coverage-kernel-branch leg
  actually run today?": settled, not dropped as a finding but retired as a
  question. Run 33560347645 on main shows the coverage job green including
  the branch leg, so rustup provisions the dated nightly on demand and
  cargo-llvm-cov supplies its component there; folded into deps-6 as
  verified context.
- Sweep phrasing in finding 5 "ci.yml has since moved only by dependabot
  bumps": corrected in deps-6 (one hand edit, e4d92ae4 on 2026-08-17, touched
  the tool-install list only); the finding itself survives.
- No finding was dropped outright: every claim checked against the tree held
  at its cited lines.

<!-- source: sweeps-final/fresh-eyes.md -->
# Sweep fresh-eyes: Fresh-eyes user: a scratch application from the public docs alone

## Method and coverage

The sweep built a scratch application against `before` and `suanpan` from their public documentation alone and reported sixteen findings, all documentation or API-shape friction, no correctness defects. This pass disputes each one against the tree at 9e5784fb4dce977cfbdfd1619886d1482b5ce764 (clean working tree).

What ran, mechanically:

- Read in full with line numbers: `crates/before/src/lib.rs`, `error.rs`, `iter.rs`, `causally/convert.rs`, `serde_impls.rs` (lines 1-80); read every cited range in `clock.rs`, `party.rs`, `version.rs`, `span.rs`, `span/wire.rs`, `version/own.rs`, `version/ticks.rs`, `version/rank.rs`, `version/ranked.rs`, `causally.rs`, `causally/forms.rs`, `causally/query.rs`, `clock/forks.rs`, `party/forks.rs`, `shape.rs`, and `README.md`.
- Grepped the crate for: every `FromStr` impl (four: `Clock`, `Version`, `Party`, `Ticks`); every `pub fn decode` (six public plus one private skyline entry); `# Errors` counts per file (`version.rs`: 0, `party.rs`: 2, `clock.rs`: 4, `span.rs`: 1, `rank.rs`: 2, `ranked.rs`: 1); every non-test use of `Overlap` (constructed only in `Clock::sync` and `Clock::sync_all`); `#[source]`/`#[from]` (none in `before` or `suanpan`); `is_human_readable` (absent); the words `mint` and `door` (public sites: `lib.rs:49`, `party.rs:850`, `party.rs:872-873`; roughly two hundred private-prose sites for `door` across `laws.rs`, `meter*`, `codec/*`, `testing/*`, `surface.rs`); `Eq`/`equality` across `causally/` (no module doc states the no-`Eq` rationale).
- Checked history with `git log -S` for the sentences each finding cites (the `PartialEq` sentence, the `Io(io::Error)` variant, the `literal door` wording, the `iter` re-exports, the no-`Eq` comment, the `error` module summary) and searched `.agent-notes/` (the ten `before`-prefixed notes and the rest) for recorded rationales on serde representation, the fork iterator name, `Decode::Io`'s source, `Rank` parsing, and atom `coverage`. None was found; the one relevant commit (e546b6d5e) introduced the "identity-minting door" wording deliberately but records no argument for the terms themselves.
- Read the sweep's own evidence logs (`scratchpad/before/sweeps/fresh-eyes/logs/{check1,run2}.log`) and matched each executed claim to its log line (the eight compile-probe errors; the `TrailingBits`/`Truncated` probes; `source() is_some = false`; the serde_json byte arrays).
- Read the existing rustdoc build at `target/doc/before/` (file dates Sep 1 22:43, later than the HEAD commit; `iter.rs` and `clock/forks.rs` last changed Aug 14) to settle how the fork iterator's return type renders. That settles the sweep's first open question: the signature renders as `pub fn forks(&mut self, k: u64) -> Forks<'_>`, linking to a page titled "Clock in before::iter".

What this pass could not see: it ran no cargo command (the two permitted test invocations were not needed; no finding rests on a runtime claim the sweep's logs and the code do not already settle), and it did not independently re-read `suanpan`, on which the sweep reported no findings.

Verdict: all sixteen findings hold on the code. Two are reframed (Span::decode already carries a `# Errors` section; the iter.rs:10 link resolves and only its backtick is stray), one gains a second defect (the private no-`Eq` comment points at module docs that hold no such rationale), and one is upgraded from an open question to verified (the fork iterator's rendered name). Nothing is dropped whole.


## Positives

- The operation tables on `Clock` (lib.rs:27-34), `Version` (version.rs:50-56), `Party` (party.rs:38-46), and `Span` (span.rs:38-48), plus the "Version vector or vector clock?" section (lib.rs:79-216), were enough for the sweep to write the whole scratch program without opening private source; every operator it reached for meant what the table said, and the quickstart compiled as written. This pass read the tables and found them accurate against the impls it checked (`Div` on `&Version`, `PartialOrd` on `Version`, `|`/`|=` on `Clock`).
- The `Decode` variant docs (error.rs:69-85) draw the `Truncated`/`TrailingBits` boundary precisely, including the flush-against-a-byte case, and the sweep's hostile probes (empty input, a party alone, a value plus trailing bytes) each returned the documented variant with no panic.
- `Span::decode`, `Rank::decode`, and `Ranked::decode` carry model `# Errors` sections (span/wire.rs:79-89, rank.rs:435-445, ranked.rs:241-247): every variant named with the input class that produces it. Finding 2 asks only that the other three decoders match them.
- `Ranked`'s composite key did what the docs promise on real data: BTreeMap order over `ranked().encode()` bytes matched `Ord` on `Ranked`, causes sorted before effects across a 13-version fleet, and `Ranked::decode` recovered the version from the key alone (the sweep's run; not re-run here).
- `Query`'s `Debug` renders the expression vocabulary, so a failed assertion reads as source; the valuation law `rank(a|b) + rank(a&b) == rank(a) + rank(b)` and `lag(a,b) + lag(b,a) == distance(a,b)` held on the fleet (the sweep's run).
- `suanpan`: the crate page's first example was enough to write a correct running total across a 2^128 carry boundary on the first try; the `&mut self` on sign reads and the one-sided `is_literally_zero` are explained where a user meets them (the sweep's report; this pass did not re-read `suanpan`).
- Space claims read true at small scale: 13 parties encode in 1-2 bytes each, and the root's version after 23 events across the fleet is 9 bytes (the sweep's run).

## Open questions for Finch

1. Is a human-readable serde form wanted? Today every type serializes as `serialize_bytes` of its canonical encoding, so serde_json carries a number array. Branching on `Serializer::is_human_readable()` to emit the paper notation (the `Display`/`FromStr` pair already exists for `Party`, `Version`, `Clock`, and `Ticks`) is how uuid and similar crates do it, but `Rank`, `Ranked`, and `Span` would need text forms first, and it changes the serialized bytes for human-readable formats. Recommendation: document the current representation now (finding 3) and decide the rest separately.
2. The `door` coinage: two public sites are the subject of finding 4, but the word appears in roughly two hundred private-prose sites across `laws.rs`, `meter*`, `codec/*`, `testing/*`, and `surface.rs` as the maintainer term for a public entry point. Is it an accepted crate-internal term of art? If so it deserves one definition where a maintainer first meets it (the `implementation` essay or `party.rs`'s module doc), and the public sites should still use plain words; if not, it is a crate-wide vocabulary dissolution, larger than this sweep.
3. The polarity conflict (`since(a) & until(b)`) produces only std's generic "trait `BitAnd` not implemented" error listing the available impls; because `BitAnd` is std's, `#[diagnostic::on_unimplemented]` cannot annotate it. Is the type-level mention at query.rs:26-30 considered sufficient, or is a worked "this does not compile, and why" example in the `causally` module doc wanted?

## Dropped

- fresh-eyes [1], sub-claim "and Span::decode" in the resolution: `Span::decode` already carries a complete `# Errors` section at span/wire.rs:79-89; the finding (fresh-eyes-2) is scoped to `Version`, `Party`, and `Clock`.
- fresh-eyes [9], sub-claim "fix the broken intra-doc link at iter.rs:10": the link target `crate::Clock::forks` resolves; the defect is the unclosed backtick, kept under fresh-eyes-11.
- Sweep open question "how does rustdoc render `Clock::forks`'s return type": settled by reading the rustdoc build at target/doc (renders `Forks<'_>`, linked to a page titled "Clock in before::iter"); folded into fresh-eyes-9.

<!-- source: sweeps-final/gate-legs.md -->
# Sweep gate-legs: Gate legs, CI, and derived-artifact freshness for before and suanpan

## Method and coverage

Verification pass over the gate-legs sweep at 9e5784fb (working tree clean,
confirmed by `git rev-parse HEAD` and `git status`). For every finding I
opened the cited sites with line numbers (`cat -n`, `awk`), grepped use
sites, and checked git history for a recorded rationale.

Read in full: the justfile (1042 lines), .github/workflows/ci.yml,
.cargo/mutants.toml, tools/mutantcheck-expected.json, rust-toolchain.toml,
deny.toml, crates/before/fuzz/Cargo.toml and README.md, both detached
`.cargo/config.toml` files, tools/benchjudge-expected.json, and
crates/before/AGENTS.md. Read in ranges: tools/mutantcheck (1-160),
tools/testdoc (1-80), tools/doclint (360-400), tools/covcheck (1-100),
tools/benchjudge (55-95), tools/citecheck (285-335), crates/before/src/surface.rs
(55-160 plus every row carrying `pins: &[]`), crates/before/src/version/rank.rs
(396-428), crates/before/src/testing/surface_coverage/tests.rs (100-150,
190-240), crates/before/tests/meter.rs (1-45, 6850-6865, 6895-6940,
7430-7475), crates/before/tests/bench_judge_roster.rs (40-62),
crates/before/src/lib.rs (398-414), the fuzzfit harness's bands.rs (60-100,
200-210, 315-325), bin/calibrate.rs (60-70, 340-410), tests/enforce.rs
(14-40, 439), and crates/before/surfacecheck/Cargo.toml (15-32).

Mechanical checks run: `just --list --unsorted` (captured to the scratch
dir); `find . -name Cargo.lock` (six lockfiles in the tree, plus a copy under
another agent's `.claude/worktrees/` checkout, left alone); `gh run view` on
runs 33567211421 (HEAD, coverage red), 33560347645 (previous green main run,
head 3327a92b), and 33429688343 (2026-08-31 instruments red), with the ci and
coverage job logs saved and grepped; `git diff --stat 3327a92b 9e5784fb --
crates/before crates/suanpan` (empty) and the Cargo.lock package diff between
those two commits (blake3/arrayref/arrayvec/constant_time_eq out; sha3,
keccak, digest, crypto-common, hybrid-array, const-oid, sponge-cursor in; no
reverse dependency of any of these is in before's or suanpan's closure, per
crates/before/Cargo.toml, crates/suanpan/Cargo.toml, and a Cargo.lock
dependency walk); `sort -u target/mutants-raw.txt | wc -l` (54,918 distinct
mutants); two Python scans over `proptest!` blocks in the in-scope crates
(245 fn heads; 244 carry `#[test]` in source, the one exception a nested
helper); `cargo mutants --version` (27.1.0 locally).

One permitted test run: `cargo nextest run -p before -p suanpan
--all-features -E 'test(masked_cmp_hole_envelope)'` with
`--success-output immediate` added so the MEASURED line is shown for a
passing test; exit 0, `peak_heap=384`. The second permitted run was not
needed.

Not re-run here (reported by the sweep, unverified by me): `just
readme-check`, `just fuelscape-verify`, `just citecheck`, `just testdoc`,
`just manifestlint`, `just fuelscape-claims`. Not runnable here: any
coverage leg, so the mechanism behind gate-legs-1 stays open.


## Positives

- gate-streams carries a liveness floor on its own verdict (justfile:409-413, 478-481): a stream killed without writing an ok or failed marker fails the gate and its partial log is replayed, so an OOM-killed stream can never read as a pass. Read and confirmed.
- Every tools/ checker leads with a `--self-test` that pins its red paths against synthetic fixtures, refuses a missing root as a usage error rather than a clean sweep (doclint:387-394 read; testdoc and workflowlint per the sweep), and carries liveness floors so an extractor that stops matching fails by name (mutantcheck:81-84, 129-133 read).
- Provenance is bound to runs, not memory: mutantcheck pins the tool version and never writes its expectation file (mutantcheck:75-79 read); benchjudge refuses a roster inventing a ceiling class (benchjudge:85-88 read); tools/readme refuses any cargo-rdme but the pinned 2.1.0 and CI installs exactly that (ci.yml:76-86 read).
- The coverage pin is tamper-evident in both directions and anchored on source text with an offset, failing closed on a vanished or ambiguous anchor (covcheck:16-27, 70-100 read), and the design reason for rejecting a global threshold is stated at the recipe (justfile:1014-1015).
- The mutants exclusion roster's three line-pinned entries (watermark.rs:790:43, grow.rs:463:38, prescan.rs:337:26) each state genre, the leg suppressed per replacement operator, and the reason the sibling legs face the suite (mutants.toml:99-117, 139-142, 146-150 read); the header's disposition ladder (refactor, assert, exclude last) is stated as standing policy.
- The `proptest!` convention is uniform: every property test writes `#[test]` in source (244 of 244), which is exactly what lets testdoc's lexical checker and citecheck's inventory see them.
- CI invokes justfile recipes rather than re-listing their steps (ci.yml:14-15), so the only rosters that can drift are the recipe lines themselves, which is where gate-legs-4 found the one divergence.
- The wasm32-pins workspace keeps overflow checks on at release so a 32-bit wrap traps rather than wraps (justfile:624-625; eb6ba627's message records the pins landing red-first with adjacency witnesses).

## Open questions for Finch

1. What allocates the extra heap in `masked_cmp_hole_envelope` under `cargo llvm-cov nextest` (1156 B) but not under `cargo nextest run` (384 B), and why did the previous coverage run on an identical before tree pass? I could not run the coverage leg. The answer decides gate-legs-1's remedy: isolate the meter, or drop the envelope suite from the coverage recipes.
2. Is wasm32-pins' absence from the CI `instruments` job deliberate (runner memory, per justfile:634-636) or an oversight from eb6ba627? Either answer belongs in that job's "what stays local, and why" comment (gate-legs-3).
3. What cadence do you want for the legs that run only at `just all` (the fuzz smoke, the formal tier, the bench judge) and for a mutation campaign if gate-legs-6 lands: a scheduled workflow for the shared-runner-safe legs, a committed attestation of the last local run that a lint leg checks for staleness, or an explicit statement at each recipe that it is manual? At 54,918 listed mutants the full campaign cannot be a per-commit leg.
4. The nightly bump procedure (justfile:35-38) names only surface-totality, but the same nightly drives doctest, fuzz-build, and coverage-kernel-branch; the stable bump procedure (rust-toolchain.toml:16-18) names only fuzzfit-calibrate while covcheck's pins are toolchain-sensitive (justfile:1019-1022). Should both procedures enumerate every leg the pin feeds?
5. The codec reviewer should answer whether the inline insta goldens in crates/before/src/testing/snapshots.rs pin the byte layout of every codec door, since a self-consistent wrong wire format passes round-trips, laws, both byte-blind oracles, and the fuzz-seed gate; gate-legs-8 is the writer-sink corner of the same question.

## Dropped

- Sweep finding [9] (testdoc cannot see tests declared inside `proptest!` blocks): false. The `proptest!` macro passes `$(#[$meta])*` through and adds no `#[test]` of its own (proptest-1.11.0 src/sugar.rs:155-159), and every property test in the in-scope crates writes `#[test]` inside the block (244 of 245 fn heads; the one without is the nested helper `depth_by_recursion` at query/tests.rs:1351, not a test), so testdoc's TEST_ATTRIBUTE regex matches each one and `has_attached_doc` walks up from that attribute to the `///`. The checker's contract holds for property tests as written; the sweep's construction (an undocumented property test passing testdoc) cannot be built without also dropping `#[test]`, which stops the fn being a test at all.

<!-- source: sweeps-final/inventory.md -->
# Sweep inventory: Allow attributes, panic sites, and visibility inventory

## Method and coverage

The sweep ran three mechanical inventories over the shipped source of `before`
and `suanpan` at 9e5784fb (script: `sweeps/inventory/inventory.sh`, raw output
in `sweeps/inventory/out/`). Tier 1 is every `.rs` under `crates/before/src` and
`crates/suanpan/src` minus `tests.rs` files, `tests/` directories, and the
instrument modules (`meter*`, `surface.rs`, `laws*`, `oracle*`, `testing*`);
tier 2 is the instrument modules minus their tests. Over each tier it grepped
for `#[allow]`/`#[expect]` attributes (14 in tier 1, 13 in tier 2), `unwrap()`
(0 shipped non-doctest sites), `expect(` (143 shipped sites), `panic!` (1
shipped), `unreachable!` (34), hard and debug asserts (24 and 174 lines),
narrowing `as` casts (80 sites), shifts, division, and `checked_`/`saturating_`
families. `visibility.py` listed bare `pub` items in private modules and
`pub(crate)` items with at most one other referencing file; its first part is
noisy (it lists every method of a public type that lives in a private module),
and the sweep refined it by hand.

This verification pass opened every cited site with line numbers, grepped the
use sites each claim rests on, and checked `git log`/`git show`, the
before-prefixed `.agent-notes/`, `crates/before/AGENTS.md`, and
`.cargo/mutants.toml` for recorded rationales. No cargo, just, or test command
ran: no finding turned on a runtime outcome. Two of the sweep's findings
reverse on evidence (the `Shl<i32>` impl has callers; the segments column's
gap is wider than the sweep stated) and one is dropped.

What this pass could not see: the tier-2 instrument modules beyond `meter.rs`
were covered by grep only, as in the sweep; the `Shl<i32>` integer-fallback
claim rests on the language rule and the operand types read at the two sites,
not on a compile check (a compile check would require editing the tree).


## Positives

- Zero `unwrap()` in shipped `before`/`suanpan` code: the five grep hits are
  doctest lines (lib.rs:68, 73; shape.rs:48, 55, 56). The one shipped `panic!`
  (skyline/build.rs:172) carries its proof in the message.
- `stacker` is a dev-dependency (Cargo.toml:44), so downstream graphs build
  no platform stack-manipulation code; the `#![forbid(unsafe_code)]` claim
  in AGENTS.md:24-25 is accurate to the dependency table.
- The debug asserts on hot decode paths are O(1) by stated policy and the
  policy names its reason: literal.rs:11-14 ("asserted work here would make
  dev builds meter a different program than the release board of record"),
  and the justfile (866-871) carries the matching profile-of-record decision
  for the board.
- The capacity-hint constructors in buf.rs:68-78 and build.rs:61-72 document
  the hint as a hint and take the total form; finding inventory-11 is the one
  straggler.
- Every `#[allow]` in the shipped tiers but one (inventory-8) names what the
  lint would flag and why it is accepted at the site, and the five
  `rustdoc::private_intra_doc_links` allows scope the lint per module so it
  stays live everywhere else.
- `IdIndex::build` (index.rs:53-57, 73-75) degrades to the unindexed walk past
  `u32::MAX` bits instead of panicking; it is the pattern inventory-1 asks the
  frame ledger to follow.
- The `Forks` saturation edge is at least recorded at the code (forks.rs:111-
  112); inventory-13 is a doc-surface gap, not a hidden behavior.

## Open questions for Finch

1. inventory-1: widen the frame-ledger link index (8 bytes per site instead
   of 4, the ledger then bounded by memory alone, matching `IdIndex`'s
   choice), or keep the cap and surface it in the public `tick`/`ticks`
   `# Panics` sections? I recommend widening: the crate docs promise every
   bound for all input sizes, and the memory trade is a constant factor.
2. inventory-2: retire the segments column outright (my recommendation), or
   promote `stacker` to an optional dependency under `meter` and give the
   column a committed known-bad shape it can fail? The second contradicts
   1ddb5a483's deliberate dev-dependency placement and still could not see a
   natively recursive kernel; `deep_tree_stack_safety` already does.
3. inventory-12: the sweep suggests `#![warn(unreachable_pub)]`. Under
   default features `pub mod skyline` becomes `pub(crate)` (version.rs:26-29),
   so the lint would fire on every `pub` item under skyline in the non-meter
   build; adopting it means either cfg-conditional allows or accepting it only
   in the meter build. Worth it, or leave this to review?
4. inventory-4: is the `tests/meter.rs` limb envelope's inclusion of
   debug-assert compares (the dev profile is the one `just test-all` runs)
   an accepted property of that suite, in which case the justfile's
   "must never be pinned anywhere" for dev numbers deserves a caveat, or a
   gap the board's release-only rule was meant to close everywhere?

## Dropped

- Sweep [12] (five near-identical `rustdoc::private_intra_doc_links` allows
  could be one): per-module scoping keeps the lint live in every other module;
  the repetition is the price of narrow scope, not a maintenance cascade, and
  the sweep itself rated it taste-level with no cost beyond the repeated
  paragraph.

<!-- source: sweeps-final/meter-adequacy.md -->
# Sweep meter-adequacy: Adequacy of the resource instruments: envelopes, board, bench judge, fuzz-fit bands, wasm32 pins, coverage pins

## Method and coverage

Verification pass over the twelve findings of the meter-adequacy sweep at
commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764 (working tree clean,
confirmed with `git rev-parse HEAD` and `git status --porcelain`).

Mechanically:

- Opened every cited site with line-numbered `awk` reads and quoted from
  those reads; every excerpt below is verbatim from the file at this tip.
- Recounted the fuzz-fit coverage: `grep -o 'pub extern "C" fn ff_*'` over
  `crates/before/fuzzfit/guest/src/lib.rs` (107 exports) against
  `grep -o 'kernel: "ff_*"'` over `harness/src/bands.rs` (44 distinct
  kernels; 53 band lines including `SMALL_BANDS`), `comm -23` for the
  difference (63 exports, of which 7 are register-machine plumbing or the
  instrument self-test: `ff_nop`, `ff_reset`, `ff_regs_reserve`,
  `ff_stage_len`, `ff_stage_prepare`, `ff_stage_ptr`,
  `ff_selftest_quadratic`; 56 are measured public-operation kernels).
  Counted the `Op` enum in `harness/src/ops.rs`: 44 variants. Counted
  distinct `ff_*` kernels named in `crates/before-fuelscape/src/ops.rs`: 97.
- Grepped `tests/meter.rs`, `src/testing/asymptotics.rs`, the fuzz-fit
  harness, and `src/shape/tests.rs` for any reference to `shape()`,
  `shape::combine`, or the counters; none.
- Reproduced `judge.rs::trend`'s least-squares fit in Python on synthetic
  `n log2 n` ladders at the board's sizes (see finding 3).
- Read the git history the findings rest on: `46eb64f9` (shape surface),
  `1f829fc7b`, `54fdf8b53`, `2732a53ae` (guest span/query/rank kernels),
  `b49424614`, `c95230c8`, `ef8894ac`, `a066a8e9` (bench roster),
  `d2a9d04e` (dated-notes excision), `cc84df7e1` (registry `decided`).
- Read `.agent-notes/2026-07-26-before-fuzzfit-asymptotics` (scope
  paragraph), `.agent-notes/2026-08-13-before-fuelscape-rustdoc` (shape
  mentions), `crates/before/AGENTS.md`, and the `.cargo/mutants.toml`
  header for recorded rationales.
- Ran no cargo, just, bench, or test command (0 of the 2 permitted test
  invocations used: no finding turned on a runtime fact an existing test
  could settle).

Not seen: the bench judge's current SKIP set and the board's version_eq
readings (both need a run); `tests/meter.rs` was read at the cited sites
and their surrounding module docs, not end to end; the design document
`design/version-skyline-iterator.md` named in `46eb64f9`'s message is not
in the tree (only `design/rumors-frame-fuzz.md` exists), so no shape-walk
instrumentation rationale beyond the NA reason strings was found.


## Positives

Verified directly in this pass (the sweep's other positives about
`currency.rs`, the coverage tiling test bodies, and `board/tests.rs`
tripwires were not re-read here and are carried as the sweep's, not mine):

- The bench judge's self-test pins its whole exit contract, including the
  two ceiling-laundering attacks (benchjudge:857-884): a roster cannot
  select a ceiling class, and moving the schoolbook cell between roster
  classes cannot lower its bar. The roster schema pin
  (bench_judge_roster.rs:72-83) refuses any expectation vocabulary beyond
  `red`, so an exemption class cannot appear by edit.
- `MIN_JUDGED_MEDIAN_NANOS` is derived, not calibrated (benchjudge:157-160:
  timer error times a stated dominance factor), which is the right shape
  for a floor over a noisy quantity.
- `tests/superlinear_tripwires.rs` binds every committed known-bad kernel
  by name in both directions, with its module doc stating exactly the
  failure it closes (a kernel that binds nowhere is silently deletable).
- The asymptotics module states its floor discipline up front
  (asymptotics.rs:12-14: floors midway between the linear reference and
  the reading, both exact counters) and prints the linear reference beside
  every run; finding 5 is one missing assert away from the stated design.
- `floors.rs:99-112` discloses the four cells no leg watches by name and
  bounds the exposure by mechanism rather than leaving it silent.
- Fuelscape's Eq/Hash panels were deliberately built on equal pairs so the
  compare runs its whole length (2732a53ae); the board's version_eq row
  (finding 7) is the one place that convention did not reach.
- The dated-notes excision (d2a9d04e) reported the `decided` machinery as
  out of scope in its own message instead of half-dissolving it; finding
  12 is that report, still open.
- `.cargo/mutants.toml`'s header states the campaign configuration of
  record and the disposition ladder, and names the instruments that kill
  mutants deliberately absent from the roster.

## Open questions for Finch

1. Shape walks (finding 1): board row group or fuzz-fit ops? The board
   gives the scan floor and the ladder for free; fuzz-fit gives coverage of
   shapes nobody chose. Recommendation: board rows first (the operands
   exist), fuzz-fit ops when the vocabulary next grows.
2. Fuzz-fit vocabulary (finding 2): is the 44-kernel scope a standing
   decision (in which case the justfile wording is the whole fix) or the
   state the atlas panels outran? The commit history records the panels
   growing without a band decision either way.
3. Probe kernels (finding 6): is a test-only "naive scan" strategy in
   suanpan acceptable, or should the three bands drop the "adequacy
   witness" wording?
4. The `decided` design round (finding 12): d2a9d04e deferred it; is it
   wanted now?
5. Carried from the sweep, not re-examined here: family adequacy for the
   three largest declared constants (ascend-cliff tick and min_ticks heap,
   the weave search allowance); whether the wall-time leg should leave a
   committed "last judged at tip" record; whether the hugeleaf parse trio
   should carry its own text ceiling; whether the `causally & conjunction`
   hole axis is bounded by construction.

## Dropped

None dropped. Two reframed: the bench-judge SKIP finding (now finding 4)
lowered from medium to low and its resolution reshaped so it does not pin
a noisy threshold; the version_eq finding (now finding 7) raised from
medium to high confidence once the stream layout showed the divergence
point exactly. One extended: the probe-build finding (now finding 6)
covers three bands, not two. One corrected: the fuzz-fit count is 56
measured public-operation kernels unbanded, not 63 (7 exports are plumbing
and the self-test burner).

<!-- source: sweeps-final/module-graph.md -->
# Sweep module-graph: Module dependency graph, layering, and the production/instrument boundary

## Method and coverage

This pass disputes the module-graph sweep's twelve seeded findings against the tree at
9e5784fb4dce977cfbdfd1619886d1482b5ce764 (working tree clean, verified with `git rev-parse HEAD`
and `git status --porcelain`). Everything was read with line numbers (`cat -n`, `awk` ranges);
every excerpt below is verbatim from the cited lines.

Mechanical checks performed:

- Writers and readers of the stack-segments counter: `grep -rn` for `descend!`, `recurse::grow`,
  `segments_grown`, `stack_segments`, `SEGMENTS_GROWN`, and `Currency::Segments` over
  `crates/before/{src,tests,benches,examples}`. The only `descend!` sites outside `recurse.rs` are
  `src/testing/bridge.rs:56,57,150,151`, `src/version/skyline/grow/tests.rs:125,172,177`, and
  `src/meter/tests.rs:417`, all compiled only under `cfg(test)`.
- Envelope rows in `crates/before/tests/meter.rs`: `pub const ...: Envelope` 15, `SweepEnvelope` 31,
  `QueryEnvelope` 33, `TouchEnvelope` 5 (84 rows); the second argument of every row's constructor
  call is `0`.
- History: `git log -S'descend!'` over the non-test source, `git show` of 05bd2b16d (the fill-walk
  conversion), 22cdfbe1 (the `implementation` module's retirement), e4d4817f1 and 76579a3c (the two
  traffic counters), 5c990b9069 (the law-group macro), 81048ec3d (`touch_ops`); `git blame` on
  `recurse.rs:71-86`, `tests/meter.rs:22-26`, `before-fuelscape/Cargo.toml:23-26`, `laws.rs:107-108`.
- The workspace resolver: the root package declares `edition = "2024"` (Cargo.toml:86), so
  dev-dependency features unify onto a package only when its test targets build; finding 2's
  mechanism depends on this.
- The sweep's cycle list: each closing edge read at its cited line (the `use` statements quoted in
  finding 10).

No cargo, just, or test command was run. None of the surviving findings turns on a runtime outcome:
each is a `cfg` attribute, a recipe line, a manifest comment, or a prose/code contradiction, and
another review workflow may be building in this workspace.

What this pass could not see: whether `cargo clippy -p before --lib -- -D warnings` is clean today
(not permitted here); the sweep's full edge list (`modgraph.py` output) was taken as reported and
only the cycle-closing edges were re-read.


## Positives

- `version::skyline` is declared twice (version.rs:22-29): `pub` under `any(test, feature =
  "meter")`, `pub(crate)` otherwise, with the reason stated inline. The shipped build leaks no
  representation while the envelope suite can path-name kernels. Verified by reading.
- The registry door's privacy claim is mechanically checked: the `compile_fail,E0603` doctest at
  registry.rs:23-27 fails to compile `before::meter::cliff_comb(4, 4)` while the sibling doctest
  builds the same comb through `Shape::CliffComb`. Verified by reading.
- The compile-to-nothing shim idiom is uniform at `codec::scan`, `hull_traffic`, `web_traffic`,
  `pool_traffic`, and suanpan's `touch`: an ungated `#[inline(always)]` function over a cfg-gated
  `mod counter`, called unconditionally from the kernels. Verified by reading each.
- No inline `#[cfg(test)] mod tests { }` block exists anywhere under `crates/before/src` or
  `crates/suanpan/src` (grep for `mod tests {` returns nothing); every suite is a sibling file.
- suanpan's module graph is a DAG: `accumulator`, `limbs`, `magnitude`, `touch_meter` behind its
  feature, `claims` under `cfg(test)` (lib.rs:352-365). Verified by reading.
- The web_traffic counter's introduction (76579a3c) records the known-bad demonstration the doctrine
  asks for: under a guard-disable mutation the arm-liveness floor reads red at both homes while
  every value differential stays green. Assessed from the commit message and web_traffic.rs:11-17.
- The envelope table's comment (tests/meter.rs:245-257) keeps measurement history out of the tree:
  "the measurements of record — and every re-pin's movement and attribution — live in the pin
  commits (`git log -S` the constant), never in this prose."
- `tests/support/fuzz_seed_set.rs` is included by `#[path]` from both the checker test
  (tests/fuzz_seeds.rs:19-20) and the writer example (examples/fuzz_seeds.rs:13-14), so the seed
  corpus has one derivation. Verified by reading.
- The `features` recipe (justfile:508-522) checks every cfg-gated surface alone, so nothing rots
  behind `--all-features`; the feature implications (`laws -> meter`, `limb-meter -> meter,
  suanpan/touch-meter`, `scan-meter -> meter`) match where the read surfaces live.
- Four of the five detached workspaces (fuzzfit, wasm32-pins, fuelscape, surfacecheck) carry their
  own fmt+clippy leg with the discipline stated in the recipe comment. Verified by reading.

## Open questions for Finch

1. Stack-segments column (module-graph-1): is a production traversal ever expected to route through
   `crate::recurse::descend!`? If not, the column outside the lib unit-test binary measures a
   compile-time fact and retiring it (option a) is the honest state; if so, the guard needs to
   exist in the `meter` builds before the column can mean anything (option b). Either way the two
   doc passages need re-stating now. Recommendation: (a).
2. `meter` feature scope (module-graph-3): is `meter` meant to stay exposure-only, in which case
   `hull_traffic` and `web_traffic` move under a counter feature (and the ungated unit-test readers
   in `meter/tests.rs` and `fill/tests.rs` gain a cfg or the gate becomes `any(test, feature)`),
   or do you accept the two relaxed-atomic bumps in bench builds and want the `meter` comment to
   name them? Recommendation: move them under `scan-meter`, since rumors' own `meter` feature
   already lights it.
3. Design essay (module-graph-5): 22cdfbe1 says the `implementation` module was retired. Does its
   content survive anywhere you want AGENTS.md to point at, or does the clause go?
4. Validation index (module-graph-11): do you want it rendered (moved under the meter-gated tree)
   or declared source-only? The `--cfg test` rustdoc route is blocked by dev-dependency imports in
   the cfg(test) tree.

## Dropped

None: every seed survived against the code. Reframings are recorded in each finding's Verification
line (the segments counter has one live writer in the lib unit-test binary; the `features` recipe
prints but does not deny default-feature warnings; the fuelscape comment was true when written; the
`--cfg test` rustdoc alternative is blocked by dev-dependencies; codec's only idbits use is
display.rs).

<!-- source: sweeps-final/paper-fidelity.md -->
# Sweep paper-fidelity: Fidelity to the ITC 2008 paper, the oracle as ground truth, and the derivations

## Method and coverage

This is the verification-and-finalization pass over the paper-fidelity sweep's fourteen seed findings, at commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764 with a clean tree. For each seed I opened every cited site with line numbers, re-read the excerpt against the file, and checked the claim's mechanism: the paper transcription (`crates/before/reference/itc2008.md` §3 lines 100-143, §5.3 lines 430-550, the section headings), the oracle (`crates/before/src/oracle.rs` 1-45; `oracle/version.rs` 1-30, 100-300; `oracle/tests.rs` 85-110, 230-255, 275-306, 545-565, 600-650), the impl sites (`version/skyline/grow.rs` 20-50 and 130-145; `version/skyline.rs` 1-12 and 60-130; `fold.rs` in full; `party.rs` 300-385 and 755-770; `clock.rs` 225-245, 426-501, 620-650; `version.rs` 20-32, 41-62, 285-300, 444-491), the test surfaces (`testing/grow_brute_force.rs` 45-80; `testing/semantic_oracle.rs` 1-40 and 300-347; `testing/semantic_oracle/tests.rs` 285-325; `testing/optrace.rs` by grep; `laws.rs` 1-60, 2395-2470; `party/tests.rs` 95-125; `testing/validation_index.rs` 130-150; `tests/superlinear_tripwires.rs` 1-40), `crates/suanpan/src/lib.rs` 1-260, `crates/before/examples/space_consumption.rs` 1-80, `crates/before/results/space_consumption/README.md` in full and the CSV header plus the endpoint rows cited below, `crates/before/AGENTS.md` in full, `crates/before/README.md` 1-8 and 312-322, `crates/before/build.rs` by grep, the `contract:` strings of `crates/before-fuelscape/src/ops.rs` (105 rows by grep, with lines 622-635 and 2140-2150 read), and the `.cargo/mutants.toml` header.

Mechanical checks: `grep` for `mod implementation` and `Law of Disjointness` across `crates/before` (the first hits only AGENTS.md and one test comment, the second nothing); `grep -c '\bmint'` over `crates/before/src` (56 sites, per-file counts recorded); `grep` for `peek` and `anonymous` in the public source files; `grep` for `100×|100x|naïve|naive` across `results/`, `examples/`, `benches/`, `tests/`; `git log -S` for the strings `1,000,000 events`, `ln(N)`, `100× more`, `asymptotically linear`, and `pub mod implementation` in `crates/before/src/lib.rs`, with the resulting commits (4488d657e, d45597843, a431eaf1, 67970b75, 22cdfbe1, f204638d) read; `Cargo.lock` for `dashu-int` and the resolved 0.5.0 source's NTT `cfg`. I ran no cargo, just, or test command: none of the surviving findings turns on a runtime fact that a hand trace or the committed tests' own doc comments did not already settle, and another review workflow may be building in this workspace.

What this pass could not see: whether the deleted design essay (`src/implementation.rs`, 270 lines removed in 22cdfbe1) was relocated anywhere other than `skyline.rs`/`lib.rs` (the commit changes those by 6 and 7 lines, so not there); the derivation, if one exists, of the packed-size bound the `O((|self| + |iter|) log k)` fold contracts need (not in `fold.rs`, `version.rs`'s join docs, or `skyline.rs` 1-130; `emit.rs` and `tests/meter.rs` were read only by grep); and the process-regime event count per iteration in `examples/space_consumption.rs` (lines 80 onward unread), which the extrapolation in finding 1 hedges.


## Positives

- The oracle transcribes §5 faithfully on every definition I read line by line: `leq` is the paper's lift form threaded through offsets (oracle/version.rs:100-112), `join_off` and `meet_off` are the absolute-offset form (114-154), `fill` has the paper's six cases in the paper's order with `(1, ir)` before `(il, 1)` (246-263 against itc2008.md:513-518), and `grow`'s tie-break is the paper's (`cl < cr` goes left, else right; 275-285 against 541-543), using the lexicographic `(expansions, depth)` pair the paper's own closing paragraph sanctions in place of the `N`-weighted integer.
- `grow_dominates_no_more_than_needed` (oracle/tests.rs:609-643) explains exactly why the literal §3 clause `x < e′ ⇒ x ≤ e` is false over the full pointwise lattice and pins the correct scoped reading; that is the right way to transcribe an informally stated paper property.
- The brute-force witness (`testing/grow_brute_force.rs`) is genuinely independent of the DP (full enumeration, no saturation), and the oracle documents its one production coupling (`RouteCost::deepen`, oracle/version.rs:14-22) with the reason.
- The skyline canonicality argument (version/skyline.rs:65-90) re-derives: minimal topology is exactly the paper's normal form, gamma and zigzag are bijections with no negative-zero spelling, so byte equality is semantic equality.
- suanpan's sign-fold bound (`2.01 · 2^(32·i)`, stop at `|s| ≥ 3`; lib.rs:103-113) and the zero-run ledger's credit argument (150-161) hold as written and are stated with their potentials.
- The function-space oracle (testing/semantic_oracle.rs) is a real third reference: it shares no tree recursion with impl or oracle, draws random §4-valid inflations and partitions per call, and names its one concession (bisecting an indivisible piece).
- The laws and the party tests already know that `join_all` hands back coalesced groups (laws.rs:2402-2416; party/tests.rs:95-105) and convict the dropped-group variant; the fold policy is adequacy-pinned, only the public prose lags (finding 7).
- The results README (space_consumption/README.md:27-41) compares this crate's encoding to the paper's Appendix A figures honestly, including the regime where the crate lands inside the paper's band rather than below it.

## Open questions for Finch

1. Where is the packed-size bound the `O((|self| + |iter|) log k)` fold contracts need argued: that a join's (or meet's, or span's) output stays within a constant of its operands' total encoded size? I did not find it in `fold.rs`, `version.rs`'s `join`/`join_all` docs, or `skyline.rs` lines 1-130 (`emit.rs` and `tests/meter.rs` were read only by grep). Without it the counter argument alone gives per-input participation, not the stated bound. This also determines finding 5's resolution.
2. lib.rs:333-340 claims auxiliary space "at most a small constant multiple of the input size" for any operation; `tests/meter.rs` pins heap peaks per committed family (for example line 7589). Is that claim scoped to the committed families, or is there an instrument covering shapes nobody chose?
3. version.rs:293 says unbounded-integer multiplication is "about O(n log n) in this implementation". `Cargo.lock` resolves `dashu-int 0.5.0`, whose NTT dispatch is disabled only under `target_pointer_width = "16"` (mul/mod.rs:27-32), so the claim holds on wasm32 today; the same file's comment says "unavailable on 16/32-bit word targets", so a bump deserves a one-line re-check of the `cfg`.
4. The oracle's `Cost` steps depth through the production `RouteCost::deepen` (oracle/version.rs:8, 20-22), documented at grow.rs:137-142 as the one function every DP implementation shares. Is that coupling acceptable to you given oracle.rs's independence framing, or should the oracle carry its own saturating step and let the differential compare the two?

## Dropped

None. Every seed finding survived on the cited text; finding 1 was reframed by the `git log -S` history (the figures are outputs of a deleted closed-form estimate), finding 4 gained a second site (`version/skyline/build/tests.rs:394`), and findings 10 and 11 are carried at medium confidence because their cost is a reader's, not a caller's.

<!-- source: sweeps-final/prose-hygiene.md -->
# Sweep prose-hygiene: Ghost references, temporal language, coinages, dialect tells

## Method and coverage

This is the verification pass over the prose-hygiene sweep at commit
9e5784fb4dce977cfbdfd1619886d1482b5ce764 (working tree clean). Every one of
the sweep's fifteen findings was disputed against the tree: each cited site
was opened with line numbers and its excerpt compared verbatim; each count was
recomputed with an independent grep over the same file set (crates/before
src, tests, benches, examples, fuzz, fuzzfit, wasm32-pins, surfacecheck, docs,
scripts, README, AGENTS.md, Cargo.toml, build.rs; crates/before-fuelscape/src;
crates/suanpan; crates/surface-scan; the root justfile, .cargo/mutants.toml,
.github/workflows); and every phrase whose age matters was traced with `git
log -S`. Recorded rationales were sought in `.agent-notes/` (the
`before-`-prefixed notes and the design-directory migration note),
`crates/before/AGENTS.md`, `.cargo/mutants.toml`'s header, and the commit
messages of the introducing commits.

Recomputed counts: `mint` 82 lines (32 of them in non-test rustdoc);
`honest`/`honestly`/`honesty` 234 and `genuine`/`genuinely` 65; `door` 279
lines in 50 files, `knob` 107 in 19, `seam` 276 in 71, `luck-proof` 1;
`improvement tripwire` 20 (all in tests/meter.rs) against 171 `tripwire` in
total; em-dashes 4140 lines, of which 3255 are rustdoc (`///` or `//!`), 551
plain `//` comments, 104 Rust code lines (string literals), and 230 non-Rust
lines (justfile 57, suanpan README 47 and before README 11, both derived by
cargo-rdme, docs/fuelscape-header.html 24, docs/fuelscape.js 22,
.cargo/mutants.toml 20, ci.yml 9, scripts 7, Cargo.toml comments 13);
calendar dates outside `decided:` literals: three (REGISTRY_RATIFIED,
PINNED_RUSTC, the justfile nightly pin), plus ten `decided: "20..."`
literals and nine `decided: REGISTRY_RATIFIED` uses; TODO/FIXME/XXX/HACK: 0;
PROG-5/COV-7 outside the fuzz workspace: 0, `.agent-notes/` included.

The pass also swept for the word "dated" as an instruction or description and
found two ghost descriptions of the convention the dated-notes excision
commit (d2a9d04e, 2026-07-31) retired; these become one new finding
(prose-hygiene-6).

What this pass could not see: no test was run (none of the surviving
findings is a correctness claim); the comparison kernel's iterativity is
taken from `Version::partial_cmp` routing to `skyline::sweep::causal_cmp`
(version.rs:1730-1741), the skyline module doc, and the pinned
`segments = 0` rows rather than from a reading of `sweep.rs`; the sweep's
sampled positives about "silently" and "the walk" were spot-checked at three
sites, not re-sampled.


## Positives

- The dated-notes excision commit d2a9d04e is the doctrine applied at scale
  and reported honestly: it collapsed two accreted history ledgers into
  standing prose, kept the toolchain pin as data, and named the `decided`
  machinery it could not dissolve as an owner question in its own message
  (verified by reading the commit).
- The envelope-table comment at tests/meter.rs:245-257 pushes every
  measurement's history to `git log -S` on the constant and keeps only the
  pricing mechanism in prose (verified by reading).
- The recurse.rs module doc's inventory of depth-recursive test surfaces
  (lines 9-14) matches the `descend!` call sites: testing/bridge.rs (4),
  skyline/grow/tests.rs (3), meter/tests.rs (2); the query/tests.rs:1167 hit
  is a comment mention (verified by grep).
- Zero TODO/FIXME/XXX/HACK markers across every in-scope surface (verified
  by grep).
- The opaque-ID discipline holds everywhere except the three fuzz-workspace
  sites: no roster tags anywhere in src, tests, benches, fuelscape, suanpan
  or surface-scan (verified by grep).
- The skyline vocabulary is anchored at its definition site: skyline.rs:4-5
  defines *skyline* in italics by contrast with the step function it names
  and introduces plateau beside it (verified by reading); the sweep reports
  the same for plateau, anchor, tooth, spine, genre, liveness floor,
  currency, band, hole/floor/ceiling, roster and fuel (sweep-reported, not
  re-checked here).
- "silently" is paired with its mechanism at the three sites spot-checked
  here (clock.rs:71 names the corrupted causal history; query.rs:389 the
  mispriced trigger; grow/tests.rs:14 the rerouted pairs); the sweep reports
  the same at 26 sampled sites and "the walk" as an actual traversal at 20
  (sweep-reported).

## Open questions for Finch

1. prose-hygiene-2: keep the compactness ratio as a documented
   stored-coding-versus-per-node-coding size bound, or dissolve
   `meter::tier2` and `testing::compactness` now that the skyline coding is
   the stored form? Recommendation: dissolve unless a consumer of the ratio
   outside the module can be named; the prose fix is unconditional either
   way.
2. prose-hygiene-7: are the `decided` fields and `REGISTRY_RATIFIED` an
   intended embedded decision-record schema (then say so once at the field
   doc), or dated rationale to dissolve? d2a9d04e's message already put this
   question to you. Recommendation: dissolve; `git log -S` on the reason
   string recovers every date.
3. prose-hygiene-11: for door (279 lines) and seam (276), one definition site
   per term or a plain-term sweep? Recommendation: define once for door and
   the shape sense of seam; replace "kernel-seam probe" (21 uses) with
   "kernel-boundary probe" and luck-proof with the property.
4. tests/meter.rs:465-466 calls the tick's cost "the fill-splice round-trip
   cost" while its row `query_env::TICK_DENSE` (line 6848) says "the fused
   tick: copy-on-first-divergence defers the output buffer past the collapse
   scan". Is "fill-splice round-trip" still the mechanism's name? Not
   asserted as a finding because the fused walk was not read here.
5. The `deterministic-liveness` floors in floors.rs (173-183 and the four
   others) are derived from the present mechanism's pass count, with "today"
   marking that fact; the metering doctrine wants floors from irreducible
   work. Deleting "today" is safe, but whether these floors are of the right
   genre is an instruments-lens question.
6. justfile:731 cites `formal/PROGRESS.md` from a rumors-only recipe: a
   build-surface design-doc citation outside this review's scope, for the
   rumors review.
7. prose-hygiene-12 proposes a `tools/` linter leg rejecting U+2014 outside
   rustdoc and Markdown; wanted, or is a one-time sweep enough?

## Dropped

- Sweep finding 9, the "hardened" clause: the crate's own vocabulary opposes
  adversarial families to "organically reachable (i.e. non-adversarial)
  inputs", so the adversary is live and the word is not a register
  transplant; the rest of the finding survives as prose-hygiene-13.
- Sweep finding 9, "which we'll write `‖r‖`": the style guide directs
  "write shared reasoning as we"; dropped from prose-hygiene-13.
- Sweep finding 11's claim that no known-bad implementation fails the
  ×0.75 floor: tests/meter.rs:46-49 and 401-404 name one (a meter hook
  deleted from one `Base` operation); the finding survives only as a naming
  nit, prose-hygiene-14.
- Sweep finding 14's "mandate" sites at board.rs:173 and ceilings.rs:3, 237:
  "work their contracts mandate" is the plain verb, not the governance noun
  the style table targets; registry.rs:887 (the noun) stays in
  prose-hygiene-16.
- Sweep finding 0's site at tests/meter.rs:466 is kept only for the word
  "today"; whether "fill-splice round-trip" misnames the fused tick is an
  open question (item 4), not a finding, because the fused walk was not
  read.
- The sweep's positive "No .agent-notes or design/ citation reaches any
  in-scope code" is narrowed: no path citation does, but board/tests.rs:505
  cites "the design doc's §3 entry" by name (prose-hygiene-3).

<!-- source: sweeps-final/recursion.md -->
# Sweep recursion: Recursion and stack-depth audit against the no-depth-recursion rule

Verification-and-finalization pass over the recursion sweep, at commit
9e5784fb4dce977cfbdfd1619886d1482b5ce764 (working tree clean, verified with
`git rev-parse HEAD` and `git status --porcelain`).

## Method and coverage

The sweep's mechanical basis is a name-resolved call-graph scan of
`crates/before/src` and `crates/suanpan/src` (`callgraph2.py` -> `sccs2.txt`,
524 lines; `classify.py` -> `classified.txt`, 406 candidate self-call lines),
with every candidate then read at its self-call sites and summarized in
`INVENTORY.txt` under `scratchpad/before/sweeps/recursion/`. This pass did not
rerun the scan; it disputed each of the ten reported findings by opening the
cited lines, grepping use sites, and reading the git history and the
before-prefixed agent notes for recorded rationales. Everything below marked
"verified" was checked mechanically in this pass:

- `descend!` use sites: `grep -rn 'descend!'` over both crates finds exactly
  four call sites in `testing/bridge.rs` (all with the literal depth 0), three
  in `version/skyline/grow/tests.rs` (depth threaded), one in
  `meter/tests.rs` (depth threaded), plus prose mentions in `recurse.rs` and
  `query/tests.rs`.
- Writers of the segments counter: the only `fetch_add` on `SEGMENTS_GROWN`
  is `recurse.rs:106`, inside `grow`, which carries `#[cfg(test)]`; the
  macro that reaches `grow` is `#[cfg(test)]` too (`recurse.rs:118`). The
  readers are `meter::stack_segments` and the metered helpers in
  `tests/meter.rs` (lines 364-371, 1212-1221, 1675-1684, 6900-6911),
  `meter/board/measure.rs:84-92`, and `meter/tests.rs`. The board runs from
  `tests/amp_board_smoke.rs`, `benches/board.rs`, and `examples/amp_board.rs`.
  `stacker` is a dev-dependency (`Cargo.toml:44`, under the
  `[dev-dependencies]` header at line 33).
- The envelope table's segments column: a multi-line regex over every
  `envelope(` row in `tests/meter.rs` finds 84 rows, all pinning `0`.
- Shape-door drivers: `grep -rn '\.shape()\|combine(\['` over
  `clock/tests.rs`, `version/tests.rs`, `party/tests.rs`, `tests/`, `benches/`,
  `examples/`, and `src/meter` returns nothing; the only in-crate drivers are
  `shape/tests.rs` and `testing/diff_ops.rs` (proptest scales), and the
  fuzzfit guest drains all three doors (`fuzzfit/guest/src/lib.rs:748, 763,
  778`).
- Public `without` at depth: `tests/meter.rs:6176` and `:6255` call
  `Party::seed().without(..)` on a spine of `ID_DEPTH = 250_000`
  (`tests/meter.rs:94`).
- `BitStack::push_bits` callers: only `PopStack::push` at `stack.rs:221-229`
  (the other `push_bits` hits are `BitsBuf`'s method in `codec/buf.rs`).
- `mint` tally: 57 occurrences across 26 files under `crates/before/src` and
  `crates/suanpan/src` (`.rs` only), plus README, `tests/`, and the fuzzfit
  guest.
- History: `git log` on `recurse.rs`; the message of `1ddb5a483` ("stacker
  becomes a dev-dependency: library walks are iterative, the guard is
  test-surface only"); `git blame` on the three mis-worded comments; the
  segments mentions in
  `.agent-notes/2026-07-22-before-adversarial-resource-amplification/`.
  `.cargo/mutants.toml` mentions neither `recurse` nor segments.

No cargo, just, or test command ran in this pass (the two permitted test
invocations were not needed: no committed test can distinguish the disputed
claims, and the one correctness finding needs a multi-hundred-GiB input).
What this pass could not see: whether the fuzzfit pipeline stages registers
deep enough for its shape drains to count as a depth proof, and whether the
pre-scan bounds a scan span by anything other than the ledger's u32 index
(read `memo.rs` in full; `prescan.rs` only at the cited sites).


## Positives

- The hard rule holds: the sweep's call-graph scan over both crates finds no
  library function recursing on input tree depth, and this pass's reading of
  every candidate agrees. The three library self-calls that exist are
  constant (`BitStack::pop_bits`, one level, stack.rs:87-102), type-level
  (`TryFrom<(u64, T, S)>` for `Version`, `PartyLiteral` for tuples), or
  log2-of-size in the meter-feature generators.
- Every explicit stack in library code states its per-level cost at its
  declaration: `LeafCursor` and `IdLeafCursor` ("Root-to-leaf branch
  directions" as `BitStack`s, overlay.rs:317-324, 471-481), `IdLeafCursor` in
  diff.rs:245-255, the `sum` frames ("two or three bits on a bit stack",
  sum.rs:17-20), `write_id` ("one to two bits per open node on a bit stack",
  display.rs:20-23), the text emitter ("one phase bit per open node, no
  recursion", text.rs:219-221), and `PopStack` ("Each entry costs `2·w` bits",
  stack.rs:187-195). Exhaustion is uniformly `Vec` growth to allocation
  failure.
- The deep-input proof is broader than AGENTS.md:37 advertises: three
  depth-100k clock tests (`deep_tree_stack_safety`,
  `deep_tree_query_and_causal_stack_safety`,
  `deep_tree_text_and_min_ticks_stack_safety`, clock/tests.rs:565-761)
  covering the clock ops, the query/causal/span surface, and the text mirrors;
  `codec::tests::deep_id_text_roundtrip` for the id text parser (cited at
  clock/tests.rs:734); the id diff ladder at `SCALES = [256, 4096, 100_000]`
  (party/tests.rs:830); and the `tests/meter.rs` envelope rows at `ID_DEPTH =
  250_000`, including `covers` and the public `without` (tests/meter.rs:6135,
  6250).
- The settle's mass-balanced reduction keeps its control stack explicit and
  pins its depth bound `2 * total.ilog2() + 2` against a recursive reference
  over a proptest of the split rule (query/tests.rs:1342-1379): an instrument
  that pins the order, not an envelope.
- Every in-crate oracle-facing harness carries a stated cap: `ARB_DEPTH = 4`
  (generators.rs:298), `ORACLE_SCALE_MAX = 4096` with the ladder's top
  deliberately beyond it (party/tests.rs:832-834), the leaf-count cap on the
  split-depth proptest (query/tests.rs:1345), and the text-parser reference's
  small-scope note (codec/tests.rs:1640-1642).
- Where `descend!` is used as documented, the design is right: it guards the
  descent rather than the body (recurse.rs:22-28), the `STRIDE`/`RED_ZONE`
  relation is derived from a frame-size measurement (recurse.rs:41-51),
  `meter/tests.rs:407-439` proves at depth 200k that the guard grows the
  stack, and `grow/tests.rs:125-180` threads a real depth through it.
- `party/forks.rs:53-54` states the reason its `Split` is iterative ("The
  recursion of `Split` made iterative, so a huge `count` cannot overflow the
  call stack").

## Open questions for Finch

1. Segments currency (recursion-1): dissolve it from `tests/meter.rs` and the
   board outright, naming the depth-100k/250k tests as the no-recursion
   detector, or keep it and re-document it as a pin on the test-surface guard
   only? Either is consistent; today's prose claims a measurement the meter
   build cannot make, and the dissolution removes public `meter`-feature
   functions. Recommendation: dissolve; the depth tests already fail on
   overflow, which is the only signal a native-recursion regression emits.
2. Bridge guard policy (recursion-2): given every bridge input is bounded by
   the oracle's own recursive `Drop`, should the ev-side `descend!` calls stay
   (guarding heavier `Base`-arithmetic frames than the oracle's `Drop` frames)
   or go? If they stay, the id walks should be guarded the same way with real
   depths threaded. Recommendation: go, with the bound stated at bridge.rs.
3. Memo link index (recursion-5): keep the u32 cell as a memory pin with the
   corner documented and ruled at `set_link`, or make `tick` total over every
   decodable input? Recommendation: document and rule; the corner's cost is a
   link store of 2^32 accumulators, and the ruling replaces an undefined
   "link-storage contract" in prescan.rs with a real one.
4. Recursion inventory (recursion-4): a mechanically checked roster
   (surface-scan flagging self-recursive functions outside `descend!`), or the
   rule stated once with its bound classes? Recommendation: the rule; the
   roster's only stable content is the three `descend!` users, which a grep
   finds.

## Dropped

- The sub-claim of the sweep's finding [5] that `Party::without` appears in no
  100k-scale test: `tests/meter.rs:6172-6185` and `6250-6261` drive the public
  door at `ID_DEPTH = 250_000`; the finding survives as recursion-6 for the
  shape iterators alone.
- The sweep's count of three "mint" sites in finding [8]: the grep finds 57
  occurrences in 26 source files, one of them anchored to `Reign::mint`; the
  finding survives as recursion-9 with the tally and is deferred to the
  vocabulary sweep for disposition.
- The sweep's `owner_gated = false` on finding [0]: dissolving the segments
  currency removes `meter::stack_segments`/`reset_stack_segments`, public under
  the `meter` feature, so recursion-1 is marked owner-gated.

<!-- source: sweeps-final/rumors-dependence.md -->
# Sweep rumors-dependence: The guarantees rumors relies on from before, checked against before's contract and tests

Repository `/Users/oxide/src/rumors` at `9e5784fb4dce977cfbdfd1619886d1482b5ce764`, working tree clean. This is the verification-and-finalization pass over the sweep's ledger (`scratchpad/before/sweeps/rumors-dependence/ledger.md`) and its five findings. Every claim below was re-checked against the cited lines; where the sweep's framing did not survive, the entry says how it was reframed.

## Method and coverage

Mechanically:

- Read the sweep's ledger in full, then opened every site the five findings cite, in before (`version.rs`, `party.rs`, `clock.rs`, `span.rs`, `laws.rs`, `surface.rs`, `meter.rs`, `meter/tier2/tests.rs`, `party/tests.rs`, `version/tests.rs`, `clock/tests.rs`, `span/tests.rs`, `codec/bits.rs`, `serde_impls.rs`, `auto_traits.rs`, `lib.rs`) and in rumors (`tree.rs`, `tree/typed/path.rs`, `tree/typed/untyped.rs`, `tree/traverse/act.rs`, `peer/gossip.rs`, `bookmark.rs`, `peer.rs`, `rumors/causal.rs`, `tree/mirror/party.rs`, `tree/mirror/streaming/message.rs`, `tree/mirror/streaming/window.rs`, `tree/tests.rs`, `tests/party_conservation.rs`, `tests/bookmark_causality.rs`, `tree/traverse/unknown/tests.rs`), in the cited ranges with line numbers.
- Grepped the whole tree for every identifier a finding turns on (`as_bytes_matches_encode`, `subadditiv`, `is_coincident`, `ptr_eq`, `span_traffic`, `require_marker_padding`, `padding_is_canonical`, `pub mod implementation`, `dangerously_alias`).
- Git: `git log -S` on `self.as_bytes().to_vec()`, `as_bytes_matches_encode`, `padding_is_canonical`, `join_encoding_is_subadditive`, `tick_only_inflates_the_region`, `dropped without further use`, and `pub mod implementation`; `git show` on d0e54d955, 24ff18b93, d800957e8, and 22cdfbe1a; `git blame` on `party.rs:514-516`.
- Rationale sources: `crates/before/AGENTS.md`; `.cargo/mutants.toml`'s header; `.agent-notes/2026-07-22-before-adversarial-resource-amplification` (the subadditivity lemma of record, the Gate A ruling, the laws module's linearity note), `2026-07-25-before-tick-cost-spec`, `2026-07-26-before-formal-tick`.
- No cargo, just, build, or test command was run. The two permitted test invocations were not needed: every disputed claim was settled by reading definitions and history (finding 3 turns on `encode` being defined as `as_bytes().to_vec()`, which is a fact of the source).

Not seen: the rendered fuelscape HTML (the JSON's `contract` field was read instead: `version_tick` declares `O(|self| + |party|)`, `span_dominance` declares `O(|self| + |version|)`); whether rustdoc on the examples flags the broken intra-doc link in finding 6 (no build). `.claude/worktrees/agent-a0e01f4cff55d1ec9/` inside the repository duplicates the same files and belongs to another running agent; it was ignored and not touched.

Verdict on the sweep: its ledger holds. rumors stays inside before's documented contract at every use the ledger enumerates, and its own corpus would catch before regressions on every property it depends on. All five findings survive; two are reframed (2 and 5), one is extended (3), and one new documentation finding fell out of the verification (6).


## Positives

Everything here was verified by reading the cited lines in this pass.

- Every ingress of a before value into rumors routes through the strict decoders with typed errors and no `unwrap`: `src/tree/mirror/party.rs:131-138` maps `Decode::Io` separately and wraps every other defect as `HandOffMalformed`; serde deserialization of `Version` is `Version::decode` (`crates/before/src/serde_impls.rs:39-44`).
- rumors' mechanism for before's Causal Singularity rule is explicit: the `Network` check precedes any reconciliation (`src/peer/gossip.rs:1155-1163`), and the bookmark record is keyed by `Network` (`src/bookmark.rs:439`).
- The bookmark confronts the documented bytes hole with its obligations stated at the site: one critical section carries the persist-before-transmit and fork-at-snapshot obligations (`src/peer/gossip.rs:645-665`); reclaim gates on `clock.own_version() <= *version` with the reasoning written beside it (`src/bookmark.rs:444-455`); a restart re-bootstraps rather than resurrecting, and the reverted-bookmark hazard is named (`src/bookmark.rs:55-70`).
- `CausalMessages`' key `(Rank, Vec<u8>)` (`src/rumors/causal.rs:54-63`) is exactly the tuple before's law `ranked_orders_by_rank_then_bytes` (`crates/before/src/laws.rs:615-628`) pins equal to `Ranked`'s total order, so rumors' claim of sharing that order is pinned on before's side.
- `crates/before/src/auto_traits.rs` pins `Send + Sync + Unpin` for every public type at compile time; rumors' `Arc`-shared trees and watch channels depend on this without saying so, and the pin makes that safe.
- `encoding_views_agree_over_impl_history` (`crates/before/src/clock/tests.rs:296-341`) drives the implementation's own `fork`/`join`/`sync` and runs strict `decode` on `as_bytes`: the live before-side pin for the seam rumors' `retire_into_rebooted_absorber_absorbs_cleanly` guards from outside.
- The subadditivity lemma comes with a tight constant and its extremal witness: `JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS` (`crates/before/src/meter/tier2/tests.rs:328-346`) and `empty_pair_is_the_subadditivity_equality_case` (`:496-516`), checked across four emitters including the kernel without its short-circuits.
- The meter reach is gated on both sides: `pub mod meter` and `pub mod surface` sit under `#[cfg(any(test, feature = "meter"))]` (`crates/before/src/lib.rs:438-442`); rumors' `span_door_traffic` module is under `#[cfg(feature = "meter")]` (`src/tree/tests.rs:1398`).
- rumors states its alias discipline at the definition (`src/peer.rs:718-725`) and restates it at the head of the conservation suite (`tests/party_conservation.rs:12-14`).

## Open questions for Finch

1. Does the event contract (strict advance, region-locality, hence distinct stamps across disjoint parties) belong in `tick`'s public rustdoc, given that rumors' identity model and bookmark reclaim rest on it? The laws exist; the question is only where the contract is stated (finding 1).
2. `just citecheck` runs `tools/citecheck --root crates/before` over `-p before`'s test list (`justfile:291-292`), so rumors' maintainer prose citing before law names (`span_place_matches_relations`, `span_dominance_coarsens_place` at `src/tree/traverse/unknown/tests.rs:29-31`) is never checked. Should citecheck's haystack include before's law names for cross-crate citations, or should rumors avoid citing before test and law names?
3. Where should the subadditivity derivation of record live: the rustdoc of a test-module constant (today), or the public contract of `join`/`meet` (finding 2)?
4. Carried from the sweep's report for the rumors review, not verified in this pass: `PartyGuard::drop` (`src/peer/gossip.rs:1384-1403`) recovering a speculative fork with only a `debug_assert!(false)` on the impossible failure; `parse_record` (`src/remote/codec/frame.rs:82-84`, `:364-365`) accepting the version atom's CBOR byte-string head without spelling judgment; and `src/tree.rs:83-89`'s collision premise also assuming SHA3-256 path collision freedom, which is rumors' own hashing assumption.

## Dropped

- No finding dropped.
- From finding 5, the sub-claim that rumors' `span_door_traffic` test pins the dominance coincident rung: `meter::span_traffic` counts pair-hull constructions, not `Span::dominance`'s `ptr_eq` path (`crates/before/src/meter.rs:3640-3644`).
- From finding 2, the emphasis on "not laws, so the fuzz law target never exercises them" as the operative gap: true, but secondary once the tier2 grid and the churned and arbitrary proptests are counted as the instruments of record.
- From finding 3, "passes vacuously in any build without debug assertions" as the operative cost: the campaign configuration of record runs with debug assertions (`.cargo/mutants.toml` header), so the cost is the tautology itself and the roster pin that cites it, not release-mode vacuity.

<!-- source: sweeps-final/suite-economics.md -->
# Sweep suite-economics: Test suite timing and economics for before and suanpan

## Method and coverage

The sweep ran one full `cargo nextest run -p before -p suanpan -p surface-scan --all-features` (dev profile, under `tools/memwatch`) at 9e5784fb: 949 tests across 17 binaries, all passing, 2 skipped (`#[ignore]`), 1 flagged leaky, 0 retries configured, 19.656 s wall at load 5.19 rising to 19.23 on 16 logical cores. Its logs (build.log, run.log, load.txt, slowest30.txt, per_binary.txt) are under scratchpad/before/sweeps/suite-economics/ and I re-read them; the per-test times quoted from that run are indicative, not quiet-machine numbers.

This pass re-read every cited site with line numbers and checked each claim against the code: grep for any `NEXTEST` or process-per-test guard across crates/before, crates/suanpan, crates/surface-scan, the justfile, .config, .cargo, .github, and tools; grep for `run-ignored`, `--ignored`, `include-ignored`, `#[ignore`, `tempfile`, `temp_dir`, `surface_scan` use sites, and `pub static` declarations in laws.rs; a count of the VERSION_TRIPLE laws by the `laws!` macro's `fn name {` shape (30 laws: 12 at laws.rs:852-946, 18 at laws.rs:962-1605); the git history of every cited test file and the commits that created surface-scan (0a5bdaebd), coincident_span.rs (47b03e89), and answer_embedded.rs plus fold_skeleton.rs (4398dcd4f); a grep of .agent-notes (the before-prefixed notes) and crates/before/AGENTS.md for recorded rationales (none bear on these findings); and `strings` on the installed /opt/homebrew/bin/cargo-nextest for the `NEXTEST_EXECUTION_MODE` identifier (present, alongside the literal `process-per-test`).

Two filtered nextest invocations settled timing-dependent claims, logs under scratchpad/before/final-sweep:suite-economics/:

- run1: `version_triple_laws` alone: `PASS [ 10.910s]`, load 3.77 before and 7.50 after (kache recompiled before for about 30 s first, so the build was not fully warm).
- run2: `exhaustive_small`, the amp_board_smoke binary, and suanpan's `ledger_invariants_hold_exhaustively` together: exhaustive_small `PASS [ 12.914s]`; board_runs_to_completion 0.560 s, worst_map_covers_every_operation_row 0.645 s, merge_refuses_a_silently_shrunk_grid_for_every_family 0.771 s, shard_protocol_round_trips 0.948 s; ledger 2.545 s; load 5.33 before and 19.86 after (exhaustive_small's own rayon pool).

Not seen: per-binary link cost (no builds beyond the two runs; assessed from the binary inventory only); the meter suites' behavior under `cargo test` (finding 1's construction was not executed, since only nextest invocations were permitted); quiet-machine numbers (load never fell below 3.8).


## Positives

- The law roster (laws.rs:109-134) drives every group by construction in three consumers, verified by reading the `(version, version, version)` arms at algebraic_laws/tests.rs:81 and 346 and fuzz_laws.rs:132, and holds itself total against a `pub static` source scan (laws/tests.rs:36-51). A group split for suite economics is zero-wiring because of this design.
- The suite is fast and cannot hide flakiness: 949 tests in 19.7 s wall under a loaded machine, no `retries` key in .config/nextest.toml, no slow-timeout flags, and the one costly tail is a single well-identified proptest that measures 10.9 s alone.
- `ISOLATION_NOTE` (tests/meter.rs:351-355; src/meter/tests.rs:21-25) appends the first cause to rule out to every envelope failure. That is the right register for a diagnostic; finding 1 asks only that the same premise also be checked.
- The campaign configuration of record (.cargo/mutants.toml:78-80: nextest, whole workspace, all features) and both coverage legs (justfile:1034 and 1041) route through nextest, consistent with the meter suite's isolation requirement.
- verdict_matrix.rs:53-60 derives its budget from the roster count "never tuned by iteration" and asserts no timing anywhere; its three 3.4 s tests are intrinsic (each twin needs the full matrix).
- .config/nextest.toml's header carries its own failure analysis (the mid-shrink seed-loss collision and the `PROPTEST_MAX_SHRINK_ITERS` recovery), the right register for a config that judges liveness.
- fuzz_seeds.rs:19 shares the seed-set derivation with the writer by `#[path]` inclusion, so the corpus format cannot drift from its checker.
- suanpan's test economics are clean: 59 unit tests plus one integration binary, 5.3 s summed in the sweep's run, the exhaustive ledger sweep at 1.6 to 2.5 s, and the amortized-sequence attack stated as a mixed second difference over a 2x2 grid with its derivation in the module doc (amortized_sequences.rs:22-24).
- exhaustive.rs:41-63 documents exactly why the deep variant is ignored, what it covers that the small one cannot, and where deep structural coverage lives instead.

## Open questions for Finch

1. The VERSION_TRIPLE split (finding 2): measured 10.9 s alone. Is a cut between the lattice/metric laws and the span/query laws the grouping you want, or would you rather the span laws move to a `SPAN_TRIPLE` group as a semantic split independent of timing? Either way, a quiet-machine measurement of each half sizes it.
2. Why do coincident_span, answer_embedded, and fold_skeleton live outside tests/meter.rs (finding 4)? If it is the 10.8k-line file's compile time, `[[test]] required-features` is the cheaper change than merging, and the fixture duplication can still be removed by a shared support module under tests/support/.
3. The 2 GiB paren witness (finding 6): run it in `just all`, or replace it with the compile-time width pin?
4. Where should the `NEXTEST_EXECUTION_MODE` guard live (finding 1): one function in `before::meter` behind the `meter` feature, shared by suanpan's touch-meter tests and the satellite binaries, or a copy per binary?
5. .config/nextest.toml:7-12 reasons the terminate budget from the rumors tree fixture search "around 6 seconds" as the slowest honest test; before's law suite is now 10.9 s quiet and 18 s loaded. Rumors is out of this review's scope; the note is only that the shared config's rationale names a different crate.

## Dropped

- amp_board_smoke.rs:28-30 "stays well under a second" as a contradicted claim: the claim covers one `board::run(SMOKE_SCALE, ...)`; `board_runs_to_completion` measured 0.560 s in run2 and 0.632 s under the sweep's load, and `shard_protocol_round_trips` (0.948 s) runs two boards (lines 145 and 153). Not contradicted, and the sentence is the constant's rationale, a legitimate reason for its value.
- The sweep's statement that there is "no NEXTEST env guard anywhere" as literally stated: the `ISOLATION_NOTE` constants exist and are appended to failures; the finding survives only as reframed (a diagnostic, not a check).
- A separate finding proposing one `rosters.rs` for the four roster binaries: taste with an assessed, unmeasured link cost; folded into finding 4's resolution as optional and into open question 2.
- The exhaustive_deep doc's "`cargo test`, not nextest" instruction: accurate (the 180 s terminate budget would kill it); only its restated number is in finding 5.
- A scanner blind spot from detached-workspace `target/` directories (crates/before/{fuzz,surfacecheck,wasm32-pins}/target exist): the roster pins scan only `src/` and `tests/` (doc_hidden.rs:54-56, foreign_reexport.rs:110-116, superlinear_tripwires.rs:110-113), so no generated source is reached today.
