<!-- CAVEAT LECTOR: the ruling texts below are transcribed by Claude from Finch's decisions in review sessions; the decisions are Finch's, the wording is Claude's unless quoted. Read with the ground rules in ../../README.md. -->

# Rulings

One numbered entry per decision, in the order made. A ruling names the finding ids or the owner-decision items it disposes, states the decision positively, and records what follows for the work. Entries in the review documents that a ruling disposes carry a Disposition line citing the ruling by number.

## Ruling 1 (2026-09-02): the asymptotic claims are absolute

Disposes: skyline-coding-9; the "name the five as exceptions" alternative in crate-root-40, envelopes-a-1, README owner-decision items 12, 21, and 22, and claims open questions 1 and 3. Applies by the same sentence to the other demonstrated cost rows, rank-33, skyline-sweep-place-masked-5, and span-causally-36; the transient-space row skyline-fill-grow-2 is the same kind of promise (`lib.rs`'s "at most a small constant multiple of the input size"); ruling 2 confirms it is covered.

Finch's words: "The asymptotic claims *must* hold absolutely. I'm surprised doubly: that they don't, and that we failed to catch it using the instruments that exist. We should ensure the instruments can catch this case (and find out what else they aren't catching, if anything) and repair it."

Decision. `lib.rs:350-353` is the contract as intended: every asymptotic claim is a hard guarantee for all input sizes and shapes, including shapes reachable only through `decode`, text, or literal construction. Every demonstrated over-bound row is a defect to fix; none is an exception to declare in `lib.rs` or a model to adopt. The `tests/meter.rs` header may state which operations still measure over their bound and that each is under repair, and must not assert that the contract holds today or present the excess as accepted.

What follows. Instruments first: for each demonstrated row, land the breaching shape as an envelope row (and as a board family where the operation has a board row) so the breach reads as a failure before its cure; then fix. For skyline-coding-9 the candidate cure is to hold the re-anchored code in place at the stream's tail instead of extracting and re-splicing it (Claude's suggestion, unverified), with the two-stream builder as the fallback. Beyond the five rows: audit the join, meet, span, rank, masked-comparison, and coverage instruments for the same blind spot, an instrument that drives only the benign face of its operation, and report what else they miss (the claims document's crate-wide pattern "The committed instrument measures the benign case" is the starting list).

## Ruling 2 (2026-09-02): the transient-space promise is absolute too, and the memo families' heap is to be prevented

Disposes: skyline-fill-grow-2; README owner-decision item 26 and claims open question 8; the "declare a family model" alternative in that entry's resolution. Confirms ruling 1's reach over `lib.rs:333-340`.

Finch's words: "I'm surprised by the large heap overhead as well, and would like to prevent it also."

Decision. `lib.rs:333-340` (auxiliary space at most a small constant multiple of the input size, anything more a bug) is the contract as intended, for every input shape. The memo families' demonstrated 50 to 105 transient heap bytes per input byte on `tick` (`MemoComb` 104.7 and 98.6 B/B at d = 1000 and 2000; `MemoChain(distinct)` 53.6 and 50.5 B/B at k = 1000 and 2000) is a defect to prevent, not a family model to declare at the measured constant.

What follows. Instrument first: the witness's two-scale peak-heap reading on both families becomes an envelope row judged per input byte against the board ceiling, red before any cure, and records the suspend-stack depth and the link count at the peak so the two contributors are separated. Then the cure, measured at the parent: `Memo::links` and `SuspendedLevel`'s head and keeper stored word-or-wide (a machine-word arm and a boxed `Accumulator` arm, 16 bytes per entry, a checked fold that spills on overflow) through one shared type modeled on `MinWeb::Boundary`, which also settles the watermark partition's question about boxing `Boundary::Wide`; the suspend stack reserved once from the id tree's site-nesting depth; the deferred first-child head considered for direct entry into its ledger slot at suspend time, where its value is already final; and `fill.rs:130-134` restated to name `SuspendedLevel`. The expected result (the comb in the low twenties of bytes per input byte after the representation change, a floor of a few bytes per input byte) is Claude's reading of the struct layouts, to be confirmed by the row, not assumed.

## Ruling 3 (2026-09-02): the rumors triage's process rulings apply here unchanged

Disposes: TRIAGE.md's shared process questions 1 to 5 (the rumors plan's, which this plan adopts by reference).

Decision. The rumors triage's T1, T2, T3, T4, and T25 are this review's process rulings too. The record of what was decided is `triage/ledger.tsv` and the record of why is this file; the class documents stay as written, as evidence, and no Disposition lines are generated into them. Every high, every medium, every owner-gated entry, and anything a lane agent flags gets Finch's explicit disposition before it lands; lows and nits are swept inside lane rosters Finch approves and are reviewed as diffs. Instruments (P1) run before cures (P2). No entry takes `defer` by default rule, class, or pattern; each candidate deferral is put to Finch individually with its proposed home. The review README names `TRIAGE.md` and `triage/` as the disposition record in one sentence under "Reading the note".

What follows. The README sentence lands with this ruling. The TRIAGE.md session protocol reads with mediums in the individually ruled set.

## Ruling 4 (2026-09-02): the envelope harness unifies first, with every column pinned on every row

Disposes: TRIAGE.md process question 6; README owner-decision item 43; envelopes-a-4, envelopes-b-8, and the roster-of-record half of envelopes-a-8 and envelopes-a-11.

Decision. The four harness copies in `tests/meter.rs` unify before the breaching-shape rows rulings 1 and 2 call for are added, so those rows land once, in the unified harness. The unified envelope carries every column, pinned on every row (the second of envelopes-b-8's two shapes), not `Option`-typed columns widened row by row; the three-column tables take their one-time re-measure in the unification commit, each new pin attributed there. The public-entry rows are the roster of record for the twin rows; the scan column and the value legs port onto them.

What follows. Decision 43 moves from P5 to P1 and precedes the P2 rows. envelopes-a-4's `Option<Pin>` design is amended to full pinning; its other clauses (one `const fn`, one `metered` over a static column table, the pin convention stated once, the tick scenarios moved beside their table) stand. Acceptance beyond the entries': every pre-existing row's readings byte-identical before and after, and every newly pinned cell measured at the parent and named in the commit.

## Ruling 5 (2026-09-02): before's P1 and P2 precede the rumors plan's performance phase

Disposes: TRIAGE.md process question 7.

Decision. The one cross-plan ordering constraint: this review's P1 and P2 land before the rumors plan's P7 (rumors prices against `before`'s bounds, and the dependence document's cost table is that phase's input). Everything else interleaves by session. Lanes from the two plans never run concurrently against `just gate`; the coordinator sequences them on the machine.

## Ruling 6 (2026-09-02): the board lane splits six ways

Disposes: TRIAGE.md process question 8.

Decision. The P6 board lane runs as six lanes along the documents' own headings: frame; families, floors, and judge; ops and render; registry and tier2; oracle and laws; meter core.

What follows. The ledger's `lane` column keeps `board` until the P6 rosters are drawn; the roster drawing assigns the six.

## Ruling 7 (2026-09-02): one workspace em-dash check serves both sweeps

Disposes: TRIAGE.md process question 9; the tooling half of README owner-decision item 83 (the sweep itself is ruled in S3 with the rest of P3).

Decision. One `tools/` check for U+2014 outside rustdoc and Markdown, wired into `just gate`, serves this review's decision 83 and the rumors plan's owner decision 3. Whichever triage's lane lands first ships it; the other sweep runs against it.

## Ruling 8 (2026-09-02): the meter surface is instrument surface, outside the stability promise

Disposes: README owner-decision item 1; settles the owner-gating of board-frame-6, board-families-floors-judge-16, skyline-sweep-place-masked-35, skyline-coding-4, inventory-12, codec-base-text-tree-8, clippy-pedantic-1, clippy-pedantic-8, meter-core-3, module-graph-3, and recursion-1.

Decision. `pub mod meter` (with the skyline re-export, the board's `pub use` constants, the counter readers, and `Base` in `min_ticks`'s signature) and `pub mod surface` are instrument surface, not stable API. A change to that surface lands in the same commit as the `rumors` test update it entails. The rule is recorded at the `pub mod meter` declaration and in `crates/before/AGENTS.md`. The fifteen unreferenced board constants narrow to `pub(super)`.

What follows. The eleven entries are no longer owner-gated on API grounds; those gated only by this decision proceed on their stated resolution. Entries also cited by another owner decision (42, 51, 52, 71, 72) wait for that ruling.

## Ruling 9 (2026-09-02): flatness slack stays, its excluded exponent stated

Disposes: README owner-decision item 31; envelopes-a-16, envelopes-b-4, board-frame-8.

Decision. The ×1.25 slack stays where a declared log model needs it, and each band states the exponent bound it enforces. The board ceiling's doc reads "polynomial super-linearity". An n·log n probe joins the committed quadratic known-bad.

## Ruling 10 (2026-09-02): the rank clause gets a fuel-fit instrument at its tier

Disposes: README owner-decision item 33; skyline-query-9.

Decision. `Version::rank`'s `O(M(|v|) · log |v|)` clause is instrumented by a multi-scale fuel fit past 65 KiB in the fuzz-fit harness, which already prices `ff_version_rank` in the currency that counts the backend's work. The clause stands.

## Ruling 11 (2026-09-02): the heap exponent fits the residual, guarded by a fraction of the allowance

Disposes: README owner-decision item 53; board-families-floors-judge-21.

Decision. The board's heap-exponent leg fits the trend over the allowance-subtracted residual, the same quantity the constant leg judges, keeping the `cleared.len() >= 2 && spans` guards. The materiality guard is a fixed fraction of `HEAP_FLAT_ALLOWANCE_BYTES`: each cleared residual must be at least that fraction, the fraction derived and documented beside the constant. The entry's construction (8192 + n²/20480 over 4096..32768) is committed as a tripwire that reads "heap exponent" red in both windows.

## Ruling 12 (2026-09-02): the early-exit board rows become certifying

Disposes: README owner-decision item 54; board-ops-render-9, meter-adequacy-7.

Decision. The `causally_contains` and `party_covers` probes are replaced by certifying ones whose operands drive the full walk; the two affected `WORST_RANKINGS` rows are re-pinned with the movement named in the commit. The `version_eq` cell compares a byte-equal, buffer-distinct pair.

## Ruling 13 (2026-09-02): the five reopened board declarations are repaired as their entries state

Disposes: README owner-decision item 55; board-ops-render-6, board-ops-render-10, board-ops-render-18, board-ops-render-19, board-families-floors-judge-17.

Decision. The ascend-cliff family's heap ceilings gain an under-side band; the placement rows' touch column is pinned rather than NA; the exponent leg is judged; the tick rows' scan floor is derived from the mechanism, not stated as 8 bits per byte; `worst-cases-pin` stays and is computed from the acceptance results instead of re-sweeping the grid.

## Ruling 14 (2026-09-02): the fuzz-fit vocabulary binds to the method surface

Disposes: README owner-decision item 56; fuzzfit-strategies-7, meter-adequacy-2, fuzzfit-bands-27, fuzzfit-bands-10, fuzzfit-bands-5, fuzzfit-bands-4, fuzzfit-strategies-1.

Decision. The 44-kernel `Op` vocabulary is a state the panels outran, not a standing decision. The vocabulary binds to `METHOD_SURFACE` with a reviewed exemption list; the operations that fit the register machine today are added and the bands re-pinned. The riders land per their entries: the shape leg judges against `min(band.slope, 1.0)` with per-key exponents declared; one global `REFIT_TOLERANCE`; the deterministic prefix's ceiling margin goes; the release profile is guest-only; the reach demonstrations that live only in git history are committed.

## Ruling 15 (2026-09-02): the fuzz gate gains a seed replay and a clippy leg; the heap cap stays flat

Disposes: README owner-decision item 57; fuzz-guests-pins-14, fuzz-guests-pins-38, fuzz-guests-pins-39.

Decision. A `-runs=0` seed replay joins the gate as a seconds-long leg, and the fuzz workspace gets a clippy leg like its siblings. The 1 GiB heap cap stays flat: fuzz-guests-pins-14 takes its "ratified flat" branch (the doc restated positively, naming the class the flat cap is for) without the proportional cap and without the cap-fires test the entry asked for under either branch.

## Ruling 16 (2026-09-02): the wasm32 leg runs in CI and the lockfile audit derives its list

Disposes: README owner-decision item 58; deps-1, deps-2, deps-9, fuzz-guests-pins-1, fuzz-guests-pins-37, gate-legs-3.

Decision. The `wasm32-pins` leg joins the `instruments` CI job. The supply-chain recipe's lockfile list derives from `git ls-files` so no committed lockfile is omitted (the omitted one resolves wasmtime 47.0.3, RUSTSEC-2026-0268 and 0269). The detached lockfiles converge on a stated policy, and `cargo deny` covers dev-only duplicates.

## Ruling 17 (2026-09-02): the memory-terminal pins model the address-space bound

Disposes: README owner-decision item 59; fuzz-guests-pins-33, fuzz-guests-pins-35.

Decision. The address-space bound is the declared model, stated at the pins' rustdoc; the pins are restated positively; a panic-genre discriminator is added so the instrument establishes "allocation failure" rather than any trap.

## Ruling 18 (2026-09-02): the mutation campaign was one-time; its code findings land, its roster retires

Disposes: README owner-decision item 60; gate-legs-6, codec-bits-15, skyline-watermark-24, suanpan-40. Also settles the cargo-mutants half of item 68 (ruling 25).

Finch's words: "Don't do anything of the sort. This was a one-time campaign."

Decision. No scheduled campaign, no sharded CI job, no committed attestation, no lint leg holding one fresh. The campaign was a one-time exercise and is not made recurring. Its two surviving mutants are code findings and are fixed: the `read_bits` chunk arm in `PackedBuilder` and `drop_below`'s latent-annihilation ordering each get the test that kills them. The two suanpan exclusions dissolve per suanpan-40's parts (1) and (2): `read_digits` is restructured so the `>>=` codepoint no longer exists, and `shl(0)` on a digit-engine value, `shl(40)` on a literal-zero register, and a nonzero-final-carry readout are pinned at their exact touch counts. The exclusion roster (`.cargo/mutants.toml`) and the `mutantcheck` gate leg with its expected file retire as residue of the one-time campaign.

What follows. The retirement commit also restates `AGENTS.md`'s "Mutant exclusions" paragraph and any prose naming the roster as an authority, so nothing refers to a roster that no longer exists.

## Ruling 19 (2026-09-02): one process-isolation assertion, called from every metering helper

Disposes: README owner-decision item 61; suite-economics-1, envelopes-a-3, envelopes-b-10, meter-registry-tier2-19, testing-diff-gen-27, board-ops-render-27, suanpan-tests-12.

Decision. One `require_process_isolation()` in `before::meter` asserts `NEXTEST_EXECUTION_MODE == process-per-test`; every metering helper calls it on entry.

## Ruling 20 (2026-09-02): the accumulator bands state their mechanism; the probe citations go

Disposes: README owner-decision item 62; envelopes-a-22, meter-adequacy-6, meter-registry-tier2-7.

Finch's words: "This is ghost references in disguise. Just discuss the correct mechanism and why it's correct."

Decision. The weight-comb, freeze-parade, and tooth-tail bands drop every reference to the local probe build and its pin-time readings. In their place each band's doc states the mechanism the band protects and why that mechanism yields the flat reading. No disabled-mechanism kernel is committed and no mutant is cited; the "adequacy witness" wording goes with the citations. The eq early-exit band's pin-time aside is removed the same way.

## Ruling 21 (2026-09-02): fold floors assert their live reference; the roster derives from the contracts

Disposes: README owner-decision item 63; testing-diff-gen-23, testing-diff-gen-26, testing-diff-gen-28, meter-adequacy-5.

Decision. Each `MIN_GROWTH` floor is asserted against the in-run linear reference beside it. The pinned-operation roster derives from the `fuelscape/*.json` contracts. `sync_all` and the four `Span` folds are pinned, or excused with the reason at the site.

## Ruling 22 (2026-09-02): the bespoke flatness suites register as shapes; the three binaries fold into the meter suite

Disposes: README owner-decision item 64; tests-other-6, tests-other-14, suite-economics-4, tests-other-7.

Decision. WT, WL, and the forked deep spine register as `Shape`s with band citations and one known-bad each. The hull fold's limb leg pins `limb == 0` with its reason. `coincident_span`, `answer_embedded`, and `fold_skeleton` fold into `tests/meter.rs`.

## Ruling 23 (2026-09-02): the two ignored tests stay as they are

Disposes: README owner-decision item 65; testing-oracles-24, suite-economics-6.

Finch's words: "Do nothing."

Decision. `exhaustive_deep` and `clock_text_split_survives_two_gib_of_parens` stay `#[ignore]`d, documented manual invocations with no recipe and no cadence. Both entries close as intended behavior; the home is each test's own doc comment, which already states how it is run.

## Ruling 24 (2026-09-02): the coverage leg's reading is reproduced before anything moves; the pin never widens

Disposes: README owner-decision item 67; gate-legs-1.

Decision. The P1 gate lane runs `just coverage-kernel` twice at `9e5784fb` and compares the `MEASURED masked_cmp_hole` lines with the uninstrumented 384 B. If the instrumented reading moves between identical runs, the lane finds what allocates under `-C instrument-coverage` and isolates the meter from it or excludes the heap column under coverage with the reason at the exclusion. If the reading is stable but differs from 384, the coverage recipes filter the meter suite out. The 480 B pin is never widened.

## Ruling 25 (2026-09-02): the nightly derives from the justfile; cargo-mutants leaves CI

Disposes: README owner-decision item 68; gate-legs-2, deps-6, surface-roster-1.

Decision. Wherever a nightly recipe runs in CI, the toolchain derives from `just --evaluate nightly_toolchain`. `cargo-mutants` is not pinned; it leaves CI together with the roster and the `mutantcheck` leg (ruling 18). `cargo-rdme@2.1.0` stays pinned as it is.

## Ruling 26 (2026-09-02): the bench judge stamps the load average

Disposes: README owner-decision item 74; the performance document's open question 9 and the meter-adequacy sweep's open question on the judge.

Decision. `just all` keeps calling `bench-judge` bare. The judge stamps the load average and core count into its sidecar at run time, so any quoted number carries its own disclosure.

## Ruling 27 (2026-09-02): `VERSION_TRIPLE` splits along the law families

Disposes: README owner-decision item 75; suite-economics-2.

Decision. The generator splits semantically into a lattice/metric group and a span/query group, which coincides with the cost cut. Each half is timed once on a quiet machine and the timing recorded in the commit.

## Ruling 28 (2026-09-02): the fuzzfit gate stream is run once to settle whether it runs

Disposes: README owner-decision item 76; fuzzfit-strategies-13.

Decision. The P1 gate lane runs `just fuzzfit` once. The two same-type casts at `fuzzfit/harness/src/ops.rs:764` and `:858` are removed. If the run fails on them, the stream had not been running and that is a process finding the lane reports.
