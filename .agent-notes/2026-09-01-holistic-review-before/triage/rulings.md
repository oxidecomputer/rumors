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

## Ruling 29 (2026-09-02): `Ranked::cmp`'s contract is the multiplication-bound class

Disposes: README owner-decision item 23; rank-33.

Decision. The public `# Complexity` of `Ranked::cmp` states the pair measures' class, `O(M(|self| + |other|) · log(|self| + |other|))` time and `O(|self| + |other|)` space, mirroring `version_distance`; a sign-only fold cannot be linear on exact ties. `fuelscape/ranked_cmp.json` is regenerated through the compactor, the plateau_puncture and wide_arming pair families join the `ranked_cmp` OpSpec so the fit confronts the superlinear shapes, and the type doc's "cheaper than two folds and a compare" is restated as the constant-factor fact it is. A domination-certificate early exit for the non-tie case stays optional; the worst case is M-bound either way. The two-scale row lands red first (ruling 1).

## Ruling 30 (2026-09-02): the masked skip's peek is guarded, and stack-word reads are charged to the scan currency

Disposes: README owner-decision item 24; skyline-sweep-place-masked-5, codec-bits-29.

Decision. Each `peek_flip` in `block_skip` is guarded by `depth() > bound` (and its twin on the other cursor); the projection loop gets the same check; the amortization premise the caller must keep is stated at `peek_flip`'s doc. The scan currency charges 64 scan bits per word read inside `trailing_ones`, so the board sees stack-word reads; that charge and the dual masked family land red before the guard.

## Ruling 31 (2026-09-02): the multi-hole refinement fuses, the contract is restated, and the `k` axis is pinned

Disposes: README owner-decision item 25; span-causally-36, span-causally-24, span-causally-25, span-causally-28, skyline-sweep-place-masked-21.

Decision. `refine_partial` becomes one fused membership walk over the polarity's deciding clamp end. The published "linear time" is restated as linear in bits decoded, with per-interval work proportional to live holes. A deterministic scan-meter row in the placement module measures (k, |hi|), (2k, |hi|), and (k, 2|hi|) and asserts the marginal cost of doubling |hi| is independent of k; it lands red first. The `causally` module summary is rewritten as three clauses.

## Ruling 32 (2026-09-02): `sum_ranks` gets its ascending-order row, then the headroom cure

Disposes: README owner-decision item 27; rank-20.

Decision. The ascending-order touch row lands red, then the geometric-headroom cure; both `impl Sum` blocks carry `# Complexity`, and the pins' "adversarial order" prose is corrected.

## Ruling 33 (2026-09-02): the `u32` caps widen; every structure is bounded by memory alone

Disposes: README owner-decision item 28; inventory-1, party-23, skyline-fill-grow-23, skyline-query-24, recursion-5.

Decision. The fill walk's frame-ledger link index and `EpochLedger`'s freeze count widen to `usize`; the fold index keeps its order at every size (`Rights` as a narrow-or-wide table chosen by `bits.len()`, or `Vec<u64>` unconditionally if the board's `party_join_all` heap reading tolerates it, measured at the parent first), dissolving the unindexed fallback arm, `build_unindexed`, and the fallback differentials (or converting them to a forced-wide-table differential). `lib.rs`'s "for all input sizes" is the contract; no `# Panics` clause states a cap.

## Ruling 34 (2026-09-02): the tuple-literal constructors emit in one pass

Disposes: README owner-decision item 29; party-11, codec-base-text-tree-13, clock-14, skyline-coding-20, version-core-16.

Decision. The sealed, doc-hidden `PartyLiteral::into_id_bits` is reshaped to thread one `IdBuilder` through the literal (reserve, patch, close per level), making the door truly `O(n)`; the per-level `validate_id` is deleted; the version composer takes the same shape. The sealed trait's signature change is not a stable-API break.

## Ruling 35 (2026-09-02): `forks` yields exactly `k` children at every `k`, `u64::MAX` included

Disposes: README owner-decision item 30; clock-3, tests-other-17, party-14, api-audit-10.

Decision. The behavior changes rather than the doc: `Clock::forks` and `Party::forks` yield exactly `k` children for every `k`, including `k == u64::MAX`, where today the residual consumes the count's headroom and `u64::MAX - 1` are yielded. The public docs keep "exactly `k`" unqualified, the parameter is named `k`, and the "keeps the last share" sentence is reworded to name the residual's true position. `tests/forks_max.rs` pins the new boundary. This is an owner-directed behavior change on the stable surface; its commit names it as such.

## Ruling 36 (2026-09-02): the render merge is cured and its declared model retires

Disposes: README owner-decision item 32; skyline-coding-29. Reopens and retires the ratified render-merge model.

Decision. `span` and `drop` are carried up the spine instead of re-summed at each level (an `Accumulator` per flowing summary keeps the fold amortized O(1) across carry cliffs; only a printed base pays a magnitude read), measured at the parent on the `SKYLINE_RENDER_*` rows and the mirror-wide cells. With the cure landed, the declared model and its liveness pin retire together as `ceilings.rs` prescribes, and the class prose in `ops.rs` and the island states the derived bound.

## Ruling 37 (2026-09-02): the shape walks get their instruments; ungrounded allocation sentences are elided

Disposes: README owner-decision item 34; party-9, party-12, crate-root-37, clock-9, recursion-6, meter-adequacy-1. Ruling 44 governs the prose half.

Decision. The four public shape walks get board rows as their enforcing home, and a closed-form depth pin with a 3:1 known-bad committed; `forks`' minimal-depth balance is pinned. `Party::shape`'s "nothing allocates" sentence is elided under ruling 44, not restated.

## Ruling 38 (2026-09-02): the stack-segments currency dissolves

Disposes: README owner-decision item 42; crate-root-32, envelopes-a-2, board-ops-render-15, module-graph-1, recursion-1, inventory-2. Supersedes the 2026-07-24 defended keep of the segment meter, on the premise change that no binary judging the column can write it.

Decision. The segments currency is removed from the board (`Currency::Segments`, the per-cell ceiling, `MAX_GROWN_STACK_SEGMENTS`, `SEG_FLOOR_TRIP`, the render column), from every envelope type and every `segments:` pin in `tests/meter.rs`, and from `meter::{stack_segments, reset_stack_segments}`. `SEGMENTS_GROWN` and its readers are confined to `cfg(test)` beside their one live client, the determinism dive. The committed no-recursion proof of record is `clock::tests::deep_tree_stack_safety`, named where the column's prose stood. The harness unification (ruling 4) performs the envelope half in the same series.

## Ruling 39 (2026-09-02): suanpan computes digit positions in `u64` and converts once

Disposes: suanpan-24.

Decision. Every landing position is computed in `u64` and converted with one `usize::try_from` per `add_at`, wired to the documented panic; `# Panics` is restated in terms of the landing position. A red-first pin on a 32-bit target rides the `wasm32-pins` guest (which gains a suanpan export) under the CI leg ruling 16 adds.

## Ruling 40 (2026-09-02): the family surface binds to the census both ways

Disposes: surface-roster-7.

Decision. Each `FAMILY_SURFACE` row carries a machine-checkable membership (census-row prefixes or exact rows), and surfacecheck reconciles the two layers in both directions, the three non-impl rows excepted by name. The missing rows (`Default` on `Version`, `Rank`, `Ticks`; the `Cow<Version>` conversions; `error::Overlap` and `TooWide`) are added, and the three prose sites that describe a gate which never fires are restated.

## Ruling 41 (2026-09-02): the wide-gamma guard uses the existing reject with the exact platform threshold

Disposes: codec-bits-23.

Finch's words: "Don't we already have a mechanism in place for falling back when dashu cannot represent our data? We should ensure we use it."

Decision. The mechanism is the decoders' typed `Decode::NotCanonical` reject for a width the backend cannot hold; it stays. Its threshold is derived from `usize::BITS` so it rejects exactly the widths dashu cannot represent on the running platform (`k / W >= usize::MAX / W`), in both the per-bit and word-parallel readers through one shared predicate, with the derivation and the dependence on dashu's word width stated inline. Pinned red-first in the wasm32 guest with a bit cursor that yields the prefix without materializing it.

## Ruling 42 (2026-09-02): the combine arity cap is exported by the guest

Disposes: fuelscape-pipeline-28.

Decision. The guest exports its arity cap and the host constant derives from that export; the doc sentence that claimed a nonexistent smoke pin goes. No second hand-maintained constant remains to drift.

## Ruling 43 (2026-09-02): registry reasons become accurate, and the rosters are made drift-proof and idiomatic

Disposes: meter-registry-tier2-10. Standing direction for every lane that touches `meter/registry.rs`, the family rosters, and the tests they name.

Finch's words: "please make these instruments impossible to drift in the future. I *really don't like* the pattern of hard-coded strings and Rust source locations embedded in tests; the way these family rosters ended up is not really to my taste, but I haven't had time to make it more idiomatic and obviously correct. If you see a good way to clean it up, please do."

Decision. Each family's reason is corrected toward the code (nested-full, mirror-narrow, staircase priced by their board columns; cliff-fan and cancelling-chain naming the accumulator-stream pins they actually have; wide-tooth and jump-comb routed through public `Version::rank`). Beyond the correction: reasons, pins, and enforcement homes are to be expressed as typed references the compiler resolves (function items, registered law names, `Shape` and `Op` values), never as strings naming test functions, files, or line numbers; a lane that finds a cleaner idiomatic shape for the family rosters is authorized to adopt it, reporting the reshaping in its diff. The verification patterns "Rosters that attest a name, not a run" and "Hand rosters checked only against each other" are read under this direction.

## Ruling 44 (2026-09-02): allocation and size claims not grounded in a measurement are elided, not restated

Disposes: party-1, party-22; governs the prose half of ruling 37 (party-9) and any sibling sentence a lane meets.

Finch's words: "Get rid of all the claims about allocation that aren't grounded in reality; don't restate them, just elide them."

Decision. A rustdoc or comment sentence asserting "no allocation", "nothing allocates", or a size comparison the code does not honor is deleted, leaving the complexity clause the code does honor. `Party::covers` and `Party::is_disjoint` keep `O(|self| + |other|)` and lose the allocation sentence; `IdIndex`'s doc loses the "strictly smaller than the operand" sentence; `Party::shape` loses "nothing allocates". No replacement bound is written unless a committed instrument pins it.

## Ruling 45 (2026-09-02): the envelope suite's docs speak of the present implementation

Disposes: prose-hygiene-1.

Decision. `tests/meter.rs`'s module doc states what the suite pins; the cmp, decode, and tick scenario docs are restated from their own table comments; every recursion or quadratic description and the word "today" go.

## Ruling 46 (2026-09-02): `Rank`'s size claim becomes a derived exact bound, pinned by proptest

Disposes: rank-4.

Finch's words: "Derive and pin (using a proptest) an exact upper bound."

Decision. The unqualified "never larger" sentence is replaced by a derived exact upper bound on the rank encoding's size in terms of the version's packed size (per-level bit accounting: topology and payload bits against fraction bits; a b-bit counter's gamma cost against its packed cost), with the small-scale behavior stated, and a proptest over generated versions asserts the bound. The small-scale witness (`"(0, 1, 0)"` encodes to two bytes; its rank to two) is committed beside it.

## Ruling 47 (2026-09-02): the surface extractor is replaced by a real parser

Disposes: surface-roster-28, suanpan-35.

Decision. `surface-scan`'s line-scanning extractor is replaced by `syn`-based parsing that enumerates public functions structurally, so no indent, qualifier (`const`, `async`, `unsafe`), or nesting shape can drop silently. The fixtures the entry names become tests of the parser. surface-roster-9's proposal to retire before's line scan is read against this replacement when its P5 ruling comes.

## Ruling 48 (2026-09-02): the board's `mechanism()` column dissolves

Disposes: board-ops-render-12.

Decision. The clause citing the excised red-triage buffer is deleted; `mechanism()` and the `mech[...]` render column go, the `<- {reasons}` list being the record of every red leg.

## Ruling 49 (2026-09-02): auto-trait impls are pinned in the census; the hand list dissolves

Disposes: surface-roster-23.

Decision. Synthetic `Send`, `Sync`, and `Unpin` impls stop being excluded from the surface census and are pinned for every reachable type; `auto_traits.rs` dissolves. A public type losing `Send` fails `just surface-totality`.

## Ruling 50 (2026-09-02): fifteen pattern-placed P1 mediums land per their stated resolution, with three choices fixed

Disposes: benches-examples-17, envelopes-a-6, envelopes-b-18, fuzz-guests-pins-26, fuzzfit-bands-17, gate-legs-4, gate-legs-8, meter-core-8, skyline-sweep-place-masked-20, skyline-sweep-place-masked-32, suanpan-tests-25, testing-oracles-22, tests-other-26, tests-other-27, fuelscape-pipeline-23.

Decision. Each lands per its entry's Resolution and Acceptance, walked through individually. Where an entry offered a choice: gate-legs-4 defines `ci` from `gate-lints` plus the build legs so the rosters cannot diverge; meter-core-8 adds the promotion tap and pin, raises both guards to the threshold derived from the freeze allowance, and re-parameterizes the hoisted-window band at a promoting width with its re-pin measured at the parent; envelopes-a-6's scan floors land on every nonzero row inside the harness unification; tests-other-27 lands without the optional smallest-instance door; fuelscape-pipeline-23 uses perturbed twins with the distinctness test; suanpan-tests-25 pins exact totals re-measured once at the parent.

## Ruling 51 (2026-09-02): the envelope suite refuses to build without its meter features

Disposes: meter-adequacy-11.

Decision. `tests/meter.rs` carries a `compile_error!` when the limb and scan meter features are off; `just test` passes the features for the package.

## Ruling 52 (2026-09-02): the coinages retire everywhere

Disposes: README owner-decision item 81; crate-root-21, codec-bits-5, rank-13, skyline-coding-1, oracle-laws-19, span-causally-17, span-causally-22, testing-diff-gen-24, version-core-17, fresh-eyes-4, prose-hygiene-11, meter-core-1, and the sites the documentation document's vocabulary pattern lists.

Decision. "door", "seam", "genre", "knob", "luck-proof", "keystone", "pincer", "jaw", and "sentry" are retired from every surface, public rustdoc and maintainer prose alike; each use becomes the plain noun or the property it named. No definition site survives. The census greps hold zero.

## Ruling 53 (2026-09-02): "honest" and "genuine" sweep to plain words

Disposes: README owner-decision item 82; prose-hygiene-10, board-families-floors-judge-15, board-frame-18, skyline-query-12, tools-8 (its "honest" sites).

Decision. One crate-wide pass replaces the moralized uses with measured, real, unmodified, or baseline as each sentence needs; the replacement list is drafted in the lane brief for Finch's review. The anchored technical names (`assert_honest_text`, benchjudge's algorithm class) stay.

## Ruling 54 (2026-09-02): the em-dash check lands first, then one sweep

Disposes: README owner-decision item 83's sweep half; prose-hygiene-12, tools-29. Ruling 7 holds the tool.

Decision. The workspace U+2014 check lands in `just gate`, reads red on the existing lines, and one mechanical sweep commit takes it to zero across `crates/before` and `tools/`.

## Ruling 55 (2026-09-02): `Reign::mint` becomes `Reign::new`; "mint" sweeps to the plain verb

Disposes: README owner-decision item 84; prose-hygiene-5, skyline-fill-grow-21, tools-8 (its "mint" sites).

Decision. The method is renamed and the prose uses construct, create, build, or the specific operation. `watermark.rs`'s latent-register sense of the word comes back as its own question when the lane presents it.

## Ruling 56 (2026-09-02): "plateau" keeps both senses

Disposes: README owner-decision item 87; crate-root-38.

Finch's words: "I think this overloading is fine. Two adjacent same-height plateaux are the same as one."

Decision. Model. A leaf and a maximal constant run are the same object under canonical form, since adjacent same-height plateaux merge; the word may name either. Home: one sentence saying so on `shape::Plateau`'s doc.

## Ruling 57 (2026-09-02): `std` paths, edition 2024 for the five crates, `results/benchmarks` excised

Disposes: README owner-decision item 91; span-causally-10, oracle-laws-7, suanpan-1, benches-examples-25.

Decision. `std` over `core` crate-wide; the five edition-2021 member crates migrate to 2024 in one commit with `manifestlint` holding the edition; the stale benchmark record directory is deleted (git keeps it).

## Ruling 58 (2026-09-02): `shape`'s likelihood paragraph goes

Disposes: README owner-decision item 35; version-core-9.

Decision. The "realistically reachable" framing is dropped; the paragraph states what the shape is and how the walk handles it, and the file's doctest stands as the counterexample.

## Ruling 59 (2026-09-02): complexity notation is defined once in `lib.rs`

Disposes: README owner-decision item 36; rank-31, skyline-query-28, the clock partition's open question 6.

Decision. One crate-level section defines `M`, `|x|`, and `|iter|`, linked from the fuelscape include; the dashu tier thresholds are named once with the bump note beside them.

## Ruling 60 (2026-09-02): the space figures are measured and committed

Disposes: README owner-decision item 37; crate-root-24, crate-root-29, paper-fidelity-1, paper-fidelity-2.

Decision. `examples/space_consumption.rs` emits the oracle tree's boxed footprint beside `encode().len()`; the run is committed and the front page and paper-fidelity paragraphs cite it with the denominator named, replacing the unsourced "100×" and "100 parties, 1,000,000 events" figures.

## Ruling 61 (2026-09-02): the compiler holds the rosters

Disposes: README owner-decision item 51; board-frame-25, board-ops-render-2, board-families-floors-judge-6, meter-core-7, envelopes-a-19, meter-registry-tier2-11, meter-registry-tier2-8, board-families-floors-judge-16.

Decision. Yes to each: the bench-rider list derives from the declared ceilings; `designed()` and the envelope-only arms fold into a `BoardFamily` enum; `FREEZE_ALLOWANCE_DIGITS` is `pub(crate)`; `FamilyId::ALL`, `index()`, and `ALL_SHAPES` derive; the two unread `FamilySpec` fields go; the policy ceremony is restated per site. Ruling 43 governs the shape.

## Ruling 62 (2026-09-02): `hull_traffic` and `web_traffic` move under `scan-meter`

Disposes: README owner-decision item 52; module-graph-3, skyline-watermark-27.

Decision. Both move; `rumors`' `meter` feature already enables `scan-meter`, so nothing in `rumors` changes.

## Ruling 63 (2026-09-02): the validation index dissolves

Disposes: README owner-decision item 69; testing-oracles-28, testing-oracles-2, board-frame-26, surface-roster-16, module-graph-11, tests-other-1, fuzz-guests-pins-24.

Decision. The validation index and its totality claim are deleted; the justfile's recipe comments are the map of record for verification artifacts. Each entry's other clauses (a wrong sentence, a stale pointer) go with the index.

## Ruling 64 (2026-09-02): the recomputing guards are deleted, with the covering test named only in the commit message

Disposes: README owner-decision item 70; codec-base-text-tree-6, suanpan-21, inventory-4, skyline-sweep-place-masked-3, codec-base-text-tree-16, codec-bits-25, inventory-6, codec-base-text-tree-7, version-core-13. Supersedes commit 9f68c475's keep of `add_at`'s exit assert.

Finch's words: "By 'delete, citing' you mean in the commit message, right? Don't leave traces in the code."

Decision. Each guard is deleted. The commit message names the committed differential test that holds the guarded property; the code carries no comment, marker, or trace of the deletion.

## Ruling 65 (2026-09-02): test suites relocate to the crate's convention

Disposes: README owner-decision item 73; version-core-27, rank-2, clock-26, suanpan-30, the envelopes-b partition's open question 1.

Decision. The `Rank` and `Ranked` suites move to sibling `tests.rs` files, the serde legs to `serde_impls/tests.rs`, and the accumulator witnesses into `crates/suanpan/tests/` so `claims.rs` cites nothing across the crate boundary.

## Ruling 66 (2026-09-02): the pointers to hand-deleted sentences are deleted

Disposes: README owner-decision item 85; party-7, party-8, span-causally-5, span-causally-33, span-causally-34, span-causally-38 (party-14 is disposed by ruling 35).

Decision. The deletions in a6dcfbb4, b3f09baa0, 20c0515a, and bbb9f802 stand as intended; the prose that still points at the deleted sentences is deleted or restated so it points at nothing missing. No substance is restored.

## Ruling 67 (2026-09-02): `# Errors` and `# Panics` are made uniform, held by doclint

Disposes: README owner-decision item 86; fresh-eyes-2, api-audit-8, version-core-14, clock-11, api-audit-21, skyline-fill-grow-4, skyline-sweep-place-masked-9, skyline-sweep-place-masked-13, skyline-coding-35.

Decision. One pass copies `Rank::decode`'s form; `doclint` requires `# Errors` on every `pub fn` returning `Result`; `walk.rs`'s accurate `# Panics` form is adopted crate-wide.

## Ruling 68 (2026-09-02): guideposts toward reality; the essay waits for Finch

Disposes: README owner-decision item 88; api-audit-3, recursion-4, board-frame-5, tests-other-4, skyline-watermark-8; codec-bits-1 deferred.

Finch's words: "I will re-write the essay in my own words, some day. Do not rewrite it. Otherwise, fix AGENTS.md towards reality, etc. Please keep agent instructions compact and drift-proof."

Decision. `crates/before/AGENTS.md` is corrected toward the tree, kept compact, and shaped so it cannot drift (it names structure and points at docs of record, never restating enumerable facts). The board root doc follows its own summary-plus-pointer rule; the `answer-embedded` test name splits into its two claims; `MinWeb::compacting`'s dated ratios are excised. The identity-ladder essay (codec-bits-1) is not moved and not rewritten; it is deferred, home: Finch's own rewrite.

## Ruling 69 (2026-09-02): kernel-doc citations stay names, resolved by citecheck

Disposes: README owner-decision item 89; skyline-fill-grow-17.

Decision. `tools/citecheck` extends to backticked identifiers under `src/version/skyline/**` that match a collected test or envelope name. Real rustdoc links were considered and set aside: they would require the rows and laws to become documented items of the crate.

## Ruling 70 (2026-09-02): the `Ω(M)` floor stays private

Disposes: README owner-decision item 90; testing-diff-gen-22.

Decision. The floor is a meter liveness floor, not a contract; the `mul_bound_*` pins' docs are re-scoped to the derivation.

## Ruling 71 (2026-09-02): the five P4 highs

Disposes: benches-examples-18, envelopes-a-1, envelopes-b-27, party-3, surface-roster-20.

Decision. `examples/code_study.rs`, its `[[example]]` entry, and `study_family_versions` are deleted; the integer-code question is closed. `tests/meter.rs`'s header and row docs are rewritten per ruling 1 (the under-repair operations named by finding id, never an assertion that the contract holds today), and `span_shares_the_crossing_folds`'s doc states the touch leg alone. party-3's six ghost sites and surface-roster-20's four are fixed as their entries state.

## Ruling 72 (2026-09-02): walked P3 and P4 mediums, first group

Disposes: board-families-floors-judge-8, prose-hygiene-4, board-families-floors-judge-1, board-families-floors-judge-24, board-frame-22, board-ops-render-29, clock-25, fresh-eyes-1.

Decision. Each lands per its entry, walked individually, with these choices fixed: the two floor kinds are defined by contrast and carried as a typed `FloorKind` the legend renders from; the six base-size docs take commit 500d4d09's treatment with ruling 20 governing the two uncommitted kernels; operand.rs gets one stored-code walk and one height decode with defect.rs (board-frame-22) and bridge.rs routed through them; the clock tests' canonicity section dissolves and the three contradicting comments are fixed.

## Ruling 73 (2026-09-02): walked mediums, second group; the board excludes a log factor by affine residual

Disposes: fuelscape-render-9, fuzzfit-bands-2, fuzzfit-strategies-6, meter-adequacy-3. Amends ruling 9's reading of the exponent ceiling.

Finch's words (meter-adequacy-3): "Can we make it so that we *do* exclude n log n?"

Decision. fuelscape-render-9, fuzzfit-bands-2 (a typed `PIN_EVIDENCE` constant emitted by calibrate, docs citing its fields, an ordering test), and fuzzfit-strategies-6 (claims corrected; ranks snapshotted through `encode()`) land per their entries. For meter-adequacy-3: the board's acceptance ladder holds every deterministic currency (touch, scan, limb; heap on the allowance-subtracted residual of ruling 11) to an affine model fitted on its two smallest points and asserted at the larger points with a tolerance derived from rounding and the O(1) setup term alone, so an n log n term reads red; a committed n log n ladder is the known-bad. The slope ceiling stays as the coarse first leg. The two-point envelope bands state that they admit a logarithmic factor and that the board excludes it for every operation with a board row.

## Ruling 74 (2026-09-02): walked mediums, third group

Disposes: meter-core-4, party-4, prose-hygiene-3, skyline-coding-14.

Decision. Each lands per its entry: the comb rationales rewritten to the stored delta coding and the accumulator; the id tag read and subtree skip given two shared `pub(crate)` helpers in `idbits`, with `build_split`'s spine re-pin (party-27) attributed in the same commit; the design-doc clause deleted (prose-hygiene-3 is the same edit as board-ops-render-29 under ruling 72); the two-bit-zero counterfactual restated without the essay.

## Ruling 75 (2026-09-02): walked mediums, fourth group

Disposes: skyline-coding-33, skyline-fill-grow-27, skyline-sweep-place-masked-15, skyline-sweep-place-masked-7.

Decision. Each lands per its entry: `CheckedCursor` becomes the one strict skyline parser with `validate_from` a height fold over it and the planted-pair proptest driving both entries; the suspended-ancestor control bits become one `FrameBits` type and one `Frame` enum shared by the fill and prescan walks; one `Pair` type with one seeding constructor serves place, filter, overlay, and admit; `advance_refinement` routes through `advance_set`. Every re-pin any layout change causes is measured at the parent and attributed.

## Ruling 76 (2026-09-02): walked mediums, fifth group

Disposes: skyline-sweep-place-masked-33, skyline-watermark-18, skyline-watermark-19, span-causally-9.

Decision. Each lands per its entry: sweep.rs names the recursive oracle reached through the bridge as its verdict witness; watermark.rs gets one undercut tail with the lease-after order documented and one `cmp_min` ladder; the receiver-seeded two-sided fold has one home shared by `fold_endpoints` and `Version::span_all`, its envelopes re-measured at the parent and unchanged.

## Ruling 77 (2026-09-02): walked mediums, sixth group; S3's individually ruled set is complete

Disposes: surface-roster-6, tests-other-18, tests-other-22.

Decision. surface-roster-6 is a duplicate of gate-legs-8 (ruling 50), its exposure clause refuted by the witness and its roster half carried there. tests-other-18 adds the per-framing decode test for the `fuzz_decode_ops` seeds. tests-other-22 moves the fuzz framing constants into one `fuzz/framing.rs` shared by `#[path]` with the targets, the seed writer, and the checker. With this ruling every P3 and P4 high and medium has an individual disposition; the lows and nits sweep inside the lane rosters.

## Ruling 78 (2026-09-02): one typed scanner, a sequential oracle fold, no roster dates, and the bench judge retires

Disposes: README owner-decision items 44, 46, 47, 77; surface-roster-9, tests-other-3, suite-economics-3, surface-roster-10, surface-roster-31, tools-13, tests-other-24, surface-roster-11, envelopes-b-22, envelopes-b-25, meter-registry-tier2-3, surface-roster-5; oracle-laws-2, oracle-laws-26; surface-roster-17, meter-registry-tier2-9, meter-registry-tier2-13, meter-adequacy-12, prose-hygiene-7; tools-5, tools-7, tools-12, and the bench-judge half of tools-6 and meter-adequacy-4. Supersedes ruling 26 (the load-average stamp), which is moot with the judge retired.

Decision. (44) before's line-scan extractor retires in favor of surfacecheck (suanpan keeps ruling 47's `syn`-based extractor); `tools/citecheck` becomes a typed Rust checker and the one collection authority for the superlinear, inverted-twin, and band rosters, so an `#[ignore]`d kernel reads as failing; the fabricated-citation check and the shadow guard are reproduced as unit tests before any old scanner is deleted; `Copy` on `Leg` and `Exclusion` rides along. (46) The n-ary fold's hand-back grouping and order are unspecified, as the public `# Errors` sections say; the oracle's `join_all` becomes the sequential reference, the differential compares verdict, final accumulator, and the hand-backs' region union (plus history join for clocks), and `fold.rs` gets its own retention-arm witness. (47) The `decided` fields and `REGISTRY_RATIFIED` dissolve; git holds the dates. (77) The bench judge retires now: `tools/benchjudge`, its roster test, its sidecar classes, and `just all`'s call go; fuzz-fit's fuel bands are the cost-drift instrument of record, and `benches/tripwire.rs`'s quadratic is committed as a fuzz-fit above-band demonstration before the judge is deleted, per the retirement discipline. The criterion benches themselves stay as benchmarks, not judged instruments.

## Ruling 79 (2026-09-02): the acceptance buffers, expired instruments, Tier 2 prose, and digestshare

Disposes: README owner-decision items 48, 49, 50, 78; tools-14, testing-diff-gen-31, surface-roster-18, gate-legs-9, meter-adequacy-8; benches-examples-1, benches-examples-12, benches-examples-21, benches-examples-22, deps-7, skyline-coding-28, deps-10, fuelscape-pipeline-32, fuelscape-render-21, meter-registry-tier2-12, fuelscape-render-10, fuzz-guests-pins-4; meter-registry-tier2-14, prose-hygiene-2, testing-diff-gen-17; tools-3, tools-19. Reopens and closes fd997888's digestshare recommendation.

Decision. (48) The `EXEMPTIONS` list, `ITEM_EXCEPTIONS`, and covcheck's `remediation` disposition are deleted with their supporting code; covcheck keeps `panic-arm` and `unreachable`, each with its argument at the entry, and its self-test refuses `remediation` by name; a reachable uncovered kernel line is red until a directed test lands. (49) `benches/amplify.rs`, `emit_probe.rs`, `perf_probe.rs` and the `bitvec` dev-dependency, the presize and `display_growth` A/B arms, `spanbands`, the cliff-fan family, and the `overlay` field retire, each retirement commit naming what settled its question, the A/B records closed as cd171c29 closed the stacks leg; `fuzz_decode` stays with one sentence naming its unique payload. (50) `tier2.rs`'s prose is re-denominated to the stored representation and `Tier2Size`'s formula corrected; `tier2_size` stays as the independent sizer under the 2026-07-24 keep; `testing::compactness`'s envelope is dissolved unless the lane names a consumer of its ratio outside the module, in which case it reports back. (78) `tools/digestshare` is deleted along with its gate and ci legs; the byte-pinned snapshots are the pin of the renderer vocabulary.

## Ruling 80 (2026-09-02): doclint's summary rule yields to clippy; two P5 mediums; S4's individually ruled set is complete

Disposes: README owner-decision item 79; tools-20; fuzzfit-strategies-19; version-core-11.

Decision. clippy's `too_long_first_doc_paragraph` is enabled on the root and detached-workspace clippy legs, confirmed to fire on doclint's summary fixtures, and doclint's rule 1 is dropped; rule 2 stays. The fuzz-fit builder's write-only `Ty`/`slots` record dissolves into a register counter with the docs restated (corpus byte-identical). The `_view` join and meet doors collapse onto the `_refs` ladders, with the operator macro's extra arms, `balanced_fold`'s `view` parameter, the span fold ops' two fields, and the lockstep sentences going; this lands before the two-sided fold consolidation of ruling 76. With this ruling every P5 high and medium has an individual disposition.

## Ruling 81 (2026-09-02): API decisions 2 to 5

Disposes: README owner-decision items 2, 3, 4, 5; api-audit-1, clippy-pedantic-4; api-audit-6, fresh-eyes-9; clock-17, party-13; api-audit-13, fresh-eyes-14, meter-core-5, envelopes-b-19.

Decision. (2) `#[must_use]` with a reason string on `Party` and `Clock`, and method-level on `Version::join`, `join_all`, `meet`, `meet_all`, `Rank::checked_sub`, `saturating_sub`, and `Party::without`. (3) Finch's words: "I like the public names. Please have those be the only visible ones." The fork iterator types are renamed so `iter::Clock` and `iter::Party` are their defining names; `Forks` is not visible anywhere in the rendered docs; the public path is unchanged. (4) Finch's words: "Can we conditionally make it ExactSizeIterator on 64-bit?" Yes: the `ExactSizeIterator` impls on both fork iterators are gated `#[cfg(target_pointer_width = "64")]`, documented as such, so a 32-bit target has no `len()` rather than a trap; the wasm32 pin becomes the demonstration that the impl is absent there. (5) `TryFrom<Ticks>` by value, the `u128`/`usize`/`u32` duals, and `checked_sub`/`saturating_sub` are adopted; `From<suanpan::UBig> for Ticks` is declined, and the envelope suite gets a test-local helper.

## Ruling 82 (2026-09-02): `forks` takes a `usize`

Disposes: README owner-decision item 4 (again); clock-17, party-13, api-audit-10. Supersedes ruling 81's item (4); restates ruling 35's boundary.

Finch's words: "can we have it take a usize as an argument?"

Decision. `Clock::forks` and `Party::forks` take the child count as `usize`. The fork iterators implement `ExactSizeIterator` unconditionally, `len()` and the count sharing one type on every target, so no 32-bit trap exists to document or gate. Ruling 35 reads with `usize::MAX` in place of `u64::MAX`: exactly `k` children for every `k`, the boundary pinned by `tests/forks_max.rs`. This is an owner-directed signature change on the stable surface; its commit names it as such.

## Ruling 83 (2026-09-02): API decisions 6 to 9

Disposes: README owner-decision items 6, 7, 8, 9; api-audit-11, span-causally-1; fresh-eyes-5, api-audit-2, crate-root-15; fresh-eyes-15; fresh-eyes-6.

Decision. (6) `Hash` is derived on `Span`, `shape::Plateau`, `Rise`, and `Region`. (7) `Decode::Io`'s field is annotated `#[source]`. (8) `Clock::join` keeps `Err(Clock)`, the handed-back share being the design; model, home: `Clock::join`'s rustdoc, which states why the share comes back. Finch's words: "don't document the idiom, it's an anti-pattern": no `map_err(|_| Overlap)` example appears anywhere. (9) `Floor` and `Ceiling` docs point at `Query::from` now, and a delegating `coverage` method lands as an additive change.

## Ruling 84 (2026-09-02): API decisions 10, 11, 13; decision 12 opens the human-readable design

Disposes: README owner-decision items 10, 11, 13; api-audit-14; codec-base-text-tree-18, codec-base-text-tree-19, codec-base-text-tree-20; crate-root-34. Item 12 (fresh-eyes-3, fresh-eyes-16) is decided in direction and awaits the design conversation for its forms.

Decision. (10) `#[non_exhaustive]` on `Decode` before the first release; `Parse` stays closed; `NotCanonical`'s variant doc names the representation-bound case. (11) One whole-pass precedence rule documented on `Parse`; the three text entries use the ASCII whitespace predicate; pinned beside the existing precedence pins. (13) Deserialization goes through `serde_bytes` under the `serde` feature, each of the six types pinned against `Token::Bytes` with `serde_test`, the six impl pairs folded into one macro. (12) Finch's words: "3, and let's chat about precisely what those should be." The human-readable serde branch (`is_human_readable()`) is built in this triage, which needs text forms for `Rank`, `Ranked`, and `Span` and `FromStr` for each; the forms themselves are the subject of a design conversation whose outcome lands as a later ruling.

## Ruling 85 (2026-09-02): the human-readable forms

Disposes: README owner-decision item 12; fresh-eyes-3, fresh-eyes-16; the api document's open question 7. Completes ruling 84's item (12).

Finch's words: "we cannot allow this, because it produces an exponential blowup if parsed from this representation and serialized to binary. We should change the representation so that it's proportionate in size to the binary one." (on the decimal `n/2^k` form); "Maybe `v <= w`?" (on spans).

Decision. `Rank` renders and parses in binary with a binary point: the integer part with no leading zeros (a lone `0` for zero), a fraction present only when the exponent is nonzero, never a trailing zero digit (the numerator is odd, so the form is canonical by construction); `5` is `101`, one half is `0.1`, three halves is `1.1`. The decimal `n/2^k` rendering goes, `Debug` stays equal to `Display`, and `FromStr for Rank` accepts exactly that grammar, rejecting a non-canonical spelling as a `Parse` error. Conversion is linear in both directions, so the `Display` cost prose and its fuelscape include are re-pinned to the linear class, measured at the parent. `Ranked`'s text form is its version's text, `FromStr` deriving the rank; nothing else is rendered. `Span`'s text form is `lo <= hi`, the notation its rustdoc already uses, parsed through `Span::new` so an inverted or incomparable pair reports `Crossed`. Under `is_human_readable()` every type serializes as its `Display` string and deserializes through `FromStr`; the byte form stays accepted by every deserializer; binary formats are unchanged; both branches are pinned with `serde_test`. The new parsers follow ruling 84's whole-pass precedence rule and ASCII whitespace predicate.

## Ruling 86 (2026-09-02): API decisions 14 to 17

Disposes: README owner-decision items 14, 15, 16, 17; rank-21; api-audit-9, fresh-eyes-8, span-causally-38 (the `Query` sentence; the deleted rationale stays deleted under ruling 66); suanpan-tests-16, suanpan-20, version-core-24, api-audit-12; suanpan-10, suanpan-tests-4, suanpan-tests-8 held until their pins read.

Decision. (14) `Rank`'s `Display` pads uniformly through `f.pad`; the doc states that sign and zero flags do not apply. (15) `const fn` constructors are adopted, with `Bits::from_canonical`'s `debug_assert!` restructured so constness holds; `encoded_len()` on `Clock` and `Span` is declined; `Query` keeps no `PartialEq`, with one present-tense sentence on the type stating that and why. (16) suanpan's `claims` roster stays `#[cfg(test)]`; model, home: the module's cfg and doc. (17) `ExactSizeIterator` and a `Base` accessor on `Limbs` land now; the `merge_into_wider` swap pin lands red-first; `add_u64_shl` and `sub_u64_shl` are deleted now as suanpan API with no caller; the `Drained` newtype (suanpan-tests-8) and the zero-wide-operand question (suanpan-10, suanpan-tests-4) return as questions once the pins read.

## Ruling 87 (2026-09-02): API decisions 18 to 20; S5 is complete

Disposes: README owner-decision items 18, 19, 20; rumors-dependence-1, rumors-dependence-2, rumors-dependence-5, version-core-5, the claims document's open question 7; the dependence document's open question 2; deps-16, crate-root-2, deps-8.

Finch's words (19): "Regardless, rumors should not cite the laws *IN PUBLIC DOCS*; just in private docs."

Decision. (18) All three contracts are stated in before's public rustdoc: tick's event contract on `Version::tick`, `Party::tick`, and `Clock::tick`, with the disjoint-parties consequence added as a committed pair law; join and meet encoding subadditivity in their public `# Complexity`, the derivation at its named constant and linked from `fold.rs`, written only after the subadditivity proptests are extended to deeper generators and the meter families and read clean; the coincident span's single-comparison cost at `Span::at` and the verdict methods. (19) rumors' public rustdoc never cites a before law name; citations of law names are permitted in rumors' private docs and comments only, and citecheck gains a rumors root with `laws::registered_names()` in its haystack to hold those, after ruling 78's checker and item 18 land. Any existing public-doc citation in rumors is a finding for the rumors ledger. (20) An explicit `include` list (`src`, `docs`, the README, `build.rs` while it exists); the crates.io description corrected toward the code; `derive` dropped from the `serde` feature. With this ruling every P7 decision is made; the four P7 nits sweep in the API lane's roster.

## Ruling 88 (2026-09-02): performance decisions 38 to 41

Disposes: README owner-decision items 38, 39, 40, 41; suanpan-17, suanpan-14 (and unblocks suanpan-10's lazy spill); party-26; codec-bits-12, codec-bits-13, skyline-coding-18, version-core-15, skyline-fill-grow-36, the skyline-watermark partition's open question 4, the skyline-fill-grow partition's open question 10, the dependence document's open question 5. Re-reads ruling 50's suanpan-tests-25 choice.

Finch's words (38): "It is *absurd* to pin a lower-bound on performance. We should accept improvements!"

Decision. (38) suanpan's exact-touch contract is retired: no pin anywhere in the workspace fixes a lower bound on performance. Every touch, scan, limb, or heap pin is a ceiling; a reading under the ceiling is an improvement that lands by tightening the ceiling with its attribution in the commit; a reading over it is a failure. The only floors are liveness floors derived from a mechanism's irreducible work, never from a measured reading. suanpan-tests-25 (ruling 50) is read accordingly: its grid is pinned as ceilings plus the universal four-touches-per-round floor, not as exact totals. suanpan-17's fixed-point skip and suanpan-14's pre-reserve land as improvements. (39) Fixed-sign deletions land as a batch: one series, measured at the parent, every moved reading re-pinned in the batch with its attribution. (40) The parity-halves search floor is derived from the per-node search model before party-26 lands; the ×0.75 measured-floor convention is confined to `tests/meter.rs` and is itself a ceiling-side slack, not a floor, under item (38). (41) Each resource trade (the staging register, boxing `Boundary::Wide`, `ticks`' second walk, retained-capacity slack) is constructed and measured on a quiet machine against the parent, and lands only on measured evidence; one resident-bytes row each for the tick and hull outputs decides `shrink_to_fit`.

## Ruling 89 (2026-09-02): decisions 45, 71, 66 and the IdIndex bound

Disposes: README owner-decision items 45, 71; codec-base-text-tree-12, party-19, skyline-coding-32; span-causally-26, skyline-sweep-place-masked-35; party-25; README owner-decision item 66 except fuelscape-render-23: fuelscape-render-7, fuelscape-render-27, fuelscape-render-15, fuelscape-render-17, fuelscape-pipeline-1, fuelscape-pipeline-30, deps-4.

Finch's words (66): "All as stated, unless anything would change the rendering of the `before` docs, in which case I want to rule deliberately on that."

Decision. (45) After ruling 88's A/B of the staging register, the bit-buffer consolidation lands in the measured direction (onto `BitsBuf`, or the register given to `BitsBuf` if it wins) and all ten `BitStack` sites migrate in one change so `BitsBuf::pop` is deleted. (71) `sweep::le` is lifted into production with a sibling `lt`, routed through `causally::le`/`lt`, the coincident rungs, and `Span::contains`, its two relational scan rows landed red-first, measured at the parent. party-25: the right-child table search runs lazily and the bound is restated with `B` defined as visited pairs whose input node has a right child. (66) fuelscape-render-7 (mechanism restated, live fixture edges, the two-architecture gate run named as the check of record or a `f64::log2` shadow committed), fuelscape-render-27 (the typesetting pass gated on the current crate being `before`; other crates' pages stop being rewritten, before's are unchanged), fuelscape-render-15 (an open-and-append dump writer with the params-mismatch, duplicate-op, and plain-beside-gz refusals; the accretion recipe documented), fuelscape-render-17 (the regrid path tooled when next needed), fuelscape-pipeline-1 (the overlay derived from the board's per-operation worst-case roster with a totality test; the datasets change, the widget does not draw overlay points, so no rendering changes), fuelscape-pipeline-30 (`SurfaceRow` gains a size-axis classification both tilings honor), deps-4 (`build.rs` stays; model, home: its header comment). fuelscape-render-23 changes the widget's rendering and is ruled separately. Any later change that alters a rendered before panel is a stop for a deliberate ruling.

## Ruling 90 (2026-09-02): the probe trace, the limbs-only numerator, and two decision 72 mediums

Disposes: fuelscape-render-23; rank-22; rank-32; skyline-query-31.

Finch's words (rank-22): "Does it implement those operations as efficiently, especially asymptotically? If yes, the simpler approach seems great. But we don't want to regress on performance."

Decision. fuelscape-render-23: the probe trace keeps its five-column smooth, and the label, drag handle, readout, and guide anchor move onto the smoothed value so every element agrees with the drawn line; a deliberate rendering change to before's docs. rank-22: `Num` becomes limbs-only, on the finding that every operation `Rank` performs (add, checked subtraction, comparison, equality, hashing, cloning, and under ruling 85 display) is linear in the limb vector, the same class the backend arm gave them or better; the two-arm dispatch, the rank-only `Base` shims, and the wide-regime suites dissolve; `RANK_PAIR_MISMATCH` and `RANK_SUM_MIXED` are re-pinned at the parent first, and any reading over its ceiling after the change is a regression that stops the lane. rank-32: `Ranked::encode_rank` is `self.version.rank().encode()`, `encode_parts` private, every fused-emission sentence replaced by "equivalent to `v.rank().encode()`", the fuelscape JSON regenerated. skyline-query-31: the adequacy kernels share one integrator trait, one single and one pair driver, one settle reducer and one finish, differing from the shipped code only in the component each refutes.

## Ruling 91 (2026-09-02): the certificate ladder, the sequence bound, doc_hidden, and covcheck's file floor

Disposes: skyline-watermark-14 (reopens and retires d0501cd7's keep); suanpan-2; tests-other-13; tools-16 and decision 80's covcheck-scope rider.

Decision. `propagate`'s certificate ladder is replaced by `merge_into_wider`'s rule with a dispatch on the resulting sign and which operand was wider; the two `unreachable!` arms, their covcheck entries, and the line-pinned exclusion go; the seam-stop bands are re-run before the change as its construction and stay as the width-conservation instrument. suanpan gains a touch-meter proptest over arbitrary op streams asserting `touches <= K·work + D` with `K` derived from the potential argument in the test's doc (a ceiling under ruling 88), with a committed known-bad shown red on run-forming streams. `--document-hidden-items` joins the surface-json recipe, surfacecheck records the sealed trait's exception with its rationale, and `tests/doc_hidden.rs` is deleted once `just surface-totality` demonstrably fails on an un-excepted hidden item. covcheck gains a per-file presence floor over `git ls-files` under its scope, each absent file red unless excused with a reason; the twelve `tests.rs` siblings stay in scope, the decision stated at the scope declaration.

## Ruling 92 (2026-09-02): the shared file walker, the decision 72 and P8 rosters, and the hand-off to rumors; S6 and S7 complete

Disposes: tools-22 and decision 80's helper-module rider; decision 72's remaining rows (clippy-pedantic-1, codec-base-text-tree-8, inventory-5, oracle-laws-25, testing-diff-gen-6, oracle-laws-16, testing-oracles-17, testing-oracles-25, tests-other-2, tests-other-28, skyline-query-21, fuzz-guests-pins-12, version-core-36, rank-18, rank-16, skyline-fill-grow-8, skyline-fill-grow-25, span-causally-35); the P8 rows citing no owner decision (clippy-pedantic-3, clock-10, crate-root-35, party-24, rank-29, skyline-coding-15, skyline-query-3, codec-base-text-tree-15, codec-bits-18, party-29, skyline-fill-grow-11, skyline-sweep-place-masked-10, suanpan-25); README owner-decision item 92 and oracle-laws-4.

Decision. `tools/` gains one shared file-enumeration helper over `git ls-files` used by doclint, testdoc, and the test that copies their walk; no shared self-test scaffold. Decision 72's lows and nits are approved as a roster and land per their entries inside their module lanes (skyline-fill-grow-8 and -25 measured at the parent; rank-16 ahead of rank-22). The P8 rows citing no decision are approved as the performance lane's roster and land as ruling 88's batch. Decision 92's three unverified observations about rumors and its two owner items are drafted as rows for the rumors ledger in `triage/handoff-to-rumors.md`, for the rumors session to seed under a note citing this review; the oracle's `RouteCost` coupling (oracle-laws-4) takes the brute force's `Option<Cost>` idiom here. With this ruling every owner decision in the README has a ruling and every P1 to P5, P7, and P8 high and medium has an individual disposition; what remains is P6's module lanes.

## Ruling 93 (2026-09-02): the headline, and P6 board mediums, first group

Disposes: crate-root-25, paper-fidelity-3 (decision 21's residual); board-families-floors-judge-10, board-families-floors-judge-20, board-ops-render-26; benches-examples-19 (both halves moot: `code_study.rs` under ruling 71, `perf_probe.rs` under ruling 79).

Decision. Once the five breaches are fixed, `lib.rs`'s headline is tightened to the per-operation classes (linear for the core operations, near-linear for the n-ary folds, multiplication-bound where the answer is a wide integer), every operation's `# Complexity` section carrying its own bound; this is the P2 cures lane's final commit. The three not-applicable declarations become floors as the entry states (decode touch floored from nonzero stored deltas with the accounting reworded; the seed party's scan NA replaced by the scan floor; `clock_fork` given the party half's heap floor). The cells whose exponent legs are expected unjudged are rostered as typed data (ruling 43), the shard merge refuses an unjudged leg outside the roster, and the tampered-denominator capture is the committed known-bad. The text parser's radix delegation gets its own counter site floored from the derived mandatory limb count; the measured pinned-floor table and its quoted readings dissolve, per ruling 88.

## Ruling 94 (2026-09-02): suanpan drops dashu; P6 harness mediums, first group

Disposes: envelopes-a-14, envelopes-a-17, envelopes-a-9; envelopes-a-15 (carried by ruling 4's harness unification). Opens a suanpan API item with no finding id, recorded here.

Finch's words: "Does suanpan need dashu? I am wondering if we can simplify things here along the way."

Decision. suanpan drops its dashu dependency: `UBig` is only a boundary type there (a word view in, a magnitude out), never arithmetic, so the public API takes limb slices and returns limb vectors (or a small magnitude newtype), the `Magnitude` trait stays generic with before implementing it for `Base`, before converts at the seam at the linear cost it already pays, and the `UBig` re-export and the `Magnitude for UBig` rows leave suanpan's claims roster. before keeps dashu for multiplication in the rank integrator, `Base`'s decimal conversion, and the codecs' bit operations. This is a suanpan public API change, owner-directed, landing in the API lane after ruling 90's limbs-only `Num`. envelopes-a-14: the `Rank::cmp` row splits so the O(1) class-first leg is pinned alone, with a two-scale identity check. envelopes-a-17: every `touches >= bytes` floor is replaced by a per-family floor derived from the nonzero stored delta count, the premise stated at the assertion. envelopes-a-9: the public dense rows gain value legs from the generators' closed forms. envelopes-a-15's shared `Run`, `assert_flat`, `assert_ceilings`, and slack constants are the harness unification's.

## Ruling 95 (2026-09-02): P6 harness and fuelscape mediums, second group

Disposes: envelopes-b-21, envelopes-b-7, fuelscape-render-18, fuelscape-render-19.

Decision. The `memo_resolution_cost` bands each gain an absolute touch ceiling at the larger run and normalize their ratios per byte (ceilings only, per ruling 88). `accum_fan_touches_flat` is deleted, one sentence in the comb test's doc stating why no fan row exists (the cliff-fan family retires under ruling 79). The fuelscape render binary's width-1 panel pool becomes a plain sequential loop with its constant and the overlap prose gone. The survey's commit stamp is bound: a dirty tree under `crates/before` or `crates/suanpan` refuses the survey (or stamps `--dirty` visibly), `--dump` refuses without the tip variable, and compact's validator and `build.rs` require exactly forty lowercase hex characters.

## Ruling 96 (2026-09-02): P6 fuzz mediums; two tools items carried elsewhere

Disposes: fuzz-guests-pins-29, fuzzfit-bands-19, fuzzfit-strategies-11, fuzzfit-strategies-16; tools-4 (the same defect as gate-legs-4, ruling 50); tools-28 (moot: mutantcheck retires under ruling 18).

Finch's words (fuzzfit-bands-19): "Delete the probe, don't build the test. We know things are deterministic."

Decision. Every wasm32 rank pin gets a seam-bit witness (a second stream differing only in the deep bit with strict order asserted, the algebraic inverse for add and checked subtraction, tight brackets for the version rank, clone checks on encodings), coordinated with the API lane's binary text form. `probe.rs` is deleted; no fuel-determinism test is added, fuel determinism being taken as known. The fuzz-fit mirror models the empty-operand identity rungs for join and meet, pins the predicate on the four pair kinds, and re-pins the bands with the movement annotated. The builder counts refusals, and sanity tests assert zero on both escalation replays and every bootstrap program, with the bootstrap stream's sub-floor coverage asserted.

## Ruling 97 (2026-09-02): P6 meter-core, registry, and oracle mediums

Disposes: meter-core-2, meter-registry-tier2-16, oracle-laws-13.

Decision. All twenty unpinned generators get size-and-canonicality pins at two sizes with the six closed forms corrected, one typed roster-wide pin walks every `Shape` variant through its check, and `Packed::version()` routes through `Version::decode` under the meter feature. The Lipschitz coding pin folds into the subadditivity check as a leaf-count bound, its helper, constant, and four tests deleted, its larger operands carried into the subadditivity grid. The two oracle test docs' claims are asserted in their bodies.

## Ruling 98 (2026-09-02): the surface roster's citations dissolve; coverage totality over the public surface replaces them

Disposes: surface-roster-4. Re-reads ruling 40 (surface-roster-7), ruling 50's gate-legs-8 and ruling 77's surface-roster-6 (their citation halves), and the open surface-roster lows and nits, which the P6 surface lane re-reads under this regime and reports as landed or moot. Ruling 49 (auto-trait pins in the census) and ruling 78's citecheck authority over the superlinear, twin, and band rosters are unaffected.

Finch's words: "Note the trend: lots of surface roster claims that are just flat wrong. How can this be obviated mechanically? If there's a part of the surface roster that can't be mechanically enforced in a meaningful way, it really doesn't deserve to exist; it's just window-dressing, verification cosplay, at that point."

Decision. The per-row test citations, the `Trans` legs' "anchors the reduction" claims, the exclusion payloads' prose reasons, and the `FAMILY_SURFACE` rows dissolve: nothing can check that a cited test exercises the item it is cited for. What replaces them is mechanical and total: surfacecheck enumerates the public surface from rustdoc JSON (with hidden items, ruling 91, and auto traits, ruling 49), and a coverage floor requires every public item's body to be hit by the test suite, built on covcheck's per-file floor (ruling 91) extended to per-public-item; the only surviving roster payload is the fail-closed exclusion of a deliberately untested item with its reason, and an item leaving the report reads red. surface-roster-7's missing impls are covered by totality, so its binding work and its three prose sites go with the roster; gate-legs-8's law extension (`encode_to_matches_encode` over the rank, ranked, and span writers) still lands as a test, its citation half moot. The surface lane lands the retirement under the doctrine's discipline: the coverage floor demonstrably fires on an unexercised public item before the citations are deleted.

## Ruling 99 (2026-09-02): P6 harness and surface mediums, third group

Disposes: testing-diff-gen-14, testing-oracles-3, tests-other-10, tests-other-16.

Decision. The generator census gains classes each fed by exactly one `arb_base` arm, with floors derived from the arms' `prop_oneof` weights and the derivation stated beside each. All four bridge walks thread a depth and route every recursive call through `descend!(depth + 1, ...)`. The precedence and contains-receiver clone-identity rungs get their scan-parity pins and the module doc enumerates the rungs it holds. Foreign re-exports, foreign-target type aliases, and extern crates become surfacecheck census rows reconciled against a committed empty set, and `tests/foreign_reexport.rs` is deleted once the census demonstrably catches `pub use bytes;`.

## Ruling 100 (2026-09-02): P6 harness, codec, and core mediums, fourth group

Disposes: tests-other-30, codec-bits-30, clock-22, clock-28.

Decision. The verdict matrix's legs are rostered as typed data, the polarity twin asserted per strict-order leg, the union of fired legs asserted equal to the roster, and `flag` refusing an unrostered leg. The `BitStack` model test reaches the word spill by construction, models `set_last`, checks `trailing_ones` against the model over two spilled words, and names only what it checks. `deep_tree_stack_safety` extends to right-spine and both-present families at depth 100k with its operation set enumerated in the doc. The static-orbit pin gets a schedule covering every ordered peer pair, a re-measured octave array, a mechanism sentence derived from the coding, and no dead branch.

## Ruling 101 (2026-09-02): P6 core, harness, and skyline mediums, fifth group

Disposes: deps-3, module-graph-2, tests-other-11, skyline-coding-23.

Decision. `build.rs`'s `rerun-if-changed` list is derived from the one enumeration of inputs the script reads. The justfile lints before's bare library (default features and none) under `-D warnings` with no test targets, the recipe comment naming feature unification as the mechanism. The coincident-span docs match their assertions, dominance named as the divergence-only leg. Both test-local recursions route through `descend!`.

## Ruling 102 (2026-09-02): P6 skyline mediums, sixth group

Disposes: skyline-coding-6, skyline-fill-grow-12, skyline-fill-grow-24, skyline-sweep-place-masked-19.

Decision. The mid-stream collapsible-pair composite is committed as a span test through both `Span::decode` and the borsh leg, and the planted-pair proptest drives the admission entry (with or before ruling 75's unification). `Memo` and `PreScan` own their lifetime methods and every `pub(super)` field becomes private. Walk-surface depth widths have one rationale at `codec/stack.rs`, cited elsewhere, the false `usize` contrast deleted. The placement rows gain touch-identity legs on a carry-boundary fixture, and the three docs name them.

## Ruling 103 (2026-09-02): the last P6 mediums; every high and medium in the ledger is ruled

Disposes: skyline-watermark-21, suanpan-tests-7, gate-legs-5, tools-33.

Finch's words (tools-33): "Get rid of this entirely."

Decision. A directed `min_ticks` witness lands beside the query differentials and the watermark test module's reachability sentence is restated. `size_probe_covers_the_value` tightens to `32·D + 2` with the derivation in its doc. CI installs exactly the toolchains the recipes name, sourced from the justfile's pin, the floating steps dropped and the three prose sites rewritten for the pinned regime. workflowlint's interpreter recognizer (the pipe-to-shell detection) is deleted entirely with its docstring claims and self-test cases; the tool's `uses:` pinning role is untouched (read as the scope of "this"; a stop if Finch meant the whole tool). With this ruling every high and medium entry in the ledger carries an individual disposition; the lows and nits await lane roster approval.

## Ruling 104 (2026-09-02): every lane roster of lows and nits is approved

Disposes: the roster question for every lane brief under `triage/briefs/`: the P1 to P5 and P7 lanes' pending rows, and the sixteen P6 lanes' pending rows (board split six ways per ruling 6; core; skyline; suanpan; codec; tools; fuelscape; harness; fuzz; benches; surface).

Decision. The lows and nits placed in each lane brief as "roster: pending Finch's approval" are approved as rosters. Each lands per its entry's Resolution and Acceptance inside its lane, swept with the lane's ruled members, and is reviewed as a diff; a lane agent that finds a row's Resolution in conflict with a ruling, or that would move a snapshot, change an unnamed public signature, or alter a rendered before panel, stops and reports (rulings 6 and 89's stops). The surface lane re-reads its rows under ruling 98 and reports which survive. The ledger's open rows move to `fix` as their lanes land them, cited to this ruling.
