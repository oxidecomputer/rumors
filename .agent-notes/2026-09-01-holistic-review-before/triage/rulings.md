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
