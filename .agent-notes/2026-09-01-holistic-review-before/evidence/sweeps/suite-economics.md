# Sweep suite-economics: Test suite timing and economics for before and suanpan

## Method and coverage

The sweep ran one full `cargo nextest run -p before -p suanpan -p surface-scan --all-features` (dev profile, under `tools/memwatch`) at 9e5784fb: 949 tests across 17 binaries, all passing, 2 skipped (`#[ignore]`), 1 flagged leaky, 0 retries configured, 19.656 s wall at load 5.19 rising to 19.23 on 16 logical cores. Its logs (build.log, run.log, load.txt, slowest30.txt, per_binary.txt) are under scratchpad/before/sweeps/suite-economics/ and I re-read them; the per-test times quoted from that run are indicative, not quiet-machine numbers.

This pass re-read every cited site with line numbers and checked each claim against the code: grep for any `NEXTEST` or process-per-test guard across crates/before, crates/suanpan, crates/surface-scan, the justfile, .config, .cargo, .github, and tools; grep for `run-ignored`, `--ignored`, `include-ignored`, `#[ignore`, `tempfile`, `temp_dir`, `surface_scan` use sites, and `pub static` declarations in laws.rs; a count of the VERSION_TRIPLE laws by the `laws!` macro's `fn name {` shape (30 laws: 12 at laws.rs:852-946, 18 at laws.rs:962-1605); the git history of every cited test file and the commits that created surface-scan (0a5bdaebd), coincident_span.rs (47b03e89), and answer_embedded.rs plus fold_skeleton.rs (4398dcd4f); a grep of .agent-notes (the before-prefixed notes) and crates/before/AGENTS.md for recorded rationales (none bear on these findings); and `strings` on the installed /opt/homebrew/bin/cargo-nextest for the `NEXTEST_EXECUTION_MODE` identifier (present, alongside the literal `process-per-test`).

Two filtered nextest invocations settled timing-dependent claims, logs under scratchpad/before/final-sweep:suite-economics/:

- run1: `version_triple_laws` alone: `PASS [ 10.910s]`, load 3.77 before and 7.50 after (kache recompiled before for about 30 s first, so the build was not fully warm).
- run2: `exhaustive_small`, the amp_board_smoke binary, and suanpan's `ledger_invariants_hold_exhaustively` together: exhaustive_small `PASS [ 12.914s]`; board_runs_to_completion 0.560 s, worst_map_covers_every_operation_row 0.645 s, merge_refuses_a_silently_shrunk_grid_for_every_family 0.771 s, shard_protocol_round_trips 0.948 s; ledger 2.545 s; load 5.33 before and 19.86 after (exhaustive_small's own rayon pool).

Not seen: per-binary link cost (no builds beyond the two runs; assessed from the binary inventory only); the meter suites' behavior under `cargo test` (finding 1's construction was not executed, since only nextest invocations were permitted); quiet-machine numbers (load never fell below 3.8).

## Findings

### suite-economics-1: Process-per-test isolation is a documented premise with no runtime check
- Where: crates/before/tests/meter.rs:15-21 (related: crates/before/tests/meter.rs:351-355 and 363-371; crates/before/src/meter/tests.rs:21-25; crates/before/src/meter.rs:3549-3551; crates/before/src/codec/base/tests.rs:59-61; crates/before/tests/answer_embedded.rs:31-41; crates/before/tests/fold_skeleton.rs:20-30; crates/before/tests/coincident_span.rs:20-24; crates/suanpan/tests/amortized_sequences.rs:30-34; .cargo/mutants.toml:78-80; justfile:1034 and 1041)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep of the crates for `NEXTEST`, `process-per-test`, and the `ISOLATION_NOTE` constants; `strings` on /opt/homebrew/bin/cargo-nextest shows `NEXTEST_EXECUTION_MODE` and `process-per-test`; that nextest sets the variable to `process-per-test` in its default mode is nextest's documented behavior, assessed, not fetched); executed: no
- Verification: reframed: the suite already anticipates a shared-process runner, but only as a diagnostic appended to failures (`ISOLATION_NOTE` in tests/meter.rs and src/meter/tests.rs). That covers the loud direction (a neighbor's allocations inflate a reading and a ceiling fails with the note attached) and not the silent one (a neighbor's `reset_peak_usage` inside a scenario window, or a neighbor freeing memory that sat in the scenario's baseline, lowers the reading so a ceiling passes on a peak that never covered its scenario); history: no-rationale-found
- Owner-gated: no

Every metered reading in tests/meter.rs, the process-global counters read by answer_embedded.rs, fold_skeleton.rs, and coincident_span.rs, and suanpan's touch-meter tests are meaningful only when each test runs in its own process. The suite states this in prose at the meter modules and appends it to failures, but nothing checks it, so under `cargo test`, an IDE runner, or any future execution mode that shares a process the ceilings can pass vacuously while floors and tripwires fire spuriously. The gate honors the premise by convention (.cargo/mutants.toml:78 `test_tool = "nextest"`; justfile:1034 and 1041 `cargo llvm-cov nextest`), which is exactly why a silent breach elsewhere would go unnoticed. This breaches the doctrine that every hole becomes a committed check, never a convention held in memory, and the warning that "we are the only writer" premises can be falsified by the environment without any programmer erring.

Evidence:

        15	//! - **Peak heap bytes**: the binary-wide counting allocator
        16	//!   ([`PeakAlloc`]), read as a delta over the scenario body. One global
        17	//!   allocator exists per test binary, and the counters are process-global,
        18	//!   so per-scenario peaks are meaningful **only under nextest's
        19	//!   process-per-test isolation** — this workspace's runner. Under a runner
        20	//!   that shares one process across tests, concurrent allocation would bleed
        21	//!   between scenarios.

       351	/// Appended to every envelope failure: the first cause to rule out is a
       352	/// shared-process test runner, under which the process-global meters bleed
       353	/// other tests' work into the scenario being measured.
       354	const ISOLATION_NOTE: &str = "note: the meters are process-global and meaningful only one \
       355	     scenario per process: run under cargo nextest, not a shared-process cargo test";

       363	fn metered<R>(name: &str, input_bytes: usize, env: &Envelope, f: impl FnOnce() -> R) -> R {
       364	    meter::reset_stack_segments();
       365	    #[cfg(feature = "limb-meter")]
       366	    meter::reset_limb_ops();
       367	    HEAP.reset_peak_usage();

Resolution: add one guard and call it from every metering helper: `metered` (tests/meter.rs:363), `ticks_counters` (769-781), `identity_fast_paths::scanned` (10556), coincident_span's `scanned`, the two `counters` helpers, suanpan's `touches`, and the codec/base touch pin. The guard asserts `std::env::var("NEXTEST_EXECUTION_MODE").as_deref() == Ok("process-per-test")` and fails with a message naming the required runner; `before::meter` behind the `meter` feature is a natural single home so suanpan and the satellite binaries share it. Acceptance: the nextest gate is unchanged, and `cargo test -p before --all-features --test meter` fails every metered scenario immediately with the isolation message instead of reporting readings.
Construction: run `cargo test -p before --all-features --test meter` (libtest's default shares one process across parallel threads, one `PeakAlloc`). Without the guard, readings differ from the nextest run and some ceilings or floors flip nondeterministically between runs; with the guard, every scenario fails at its first metered call.

### suite-economics-2: version_triple_laws alone defines the suite's critical path; the VERSION_TRIPLE group bundles two cost classes
- Where: crates/before/src/laws.rs:848 (related: laws.rs:94-102 and 109-134 (the roster and its zero-wiring argument); laws.rs:852-946 (twelve lattice, order, and metric laws); laws.rs:962-1605 (eighteen span and query laws); laws.rs:1663-1714 (the span helpers); crates/before/src/testing/algebraic_laws/tests.rs:81-94 and 346; crates/before/fuzz/fuzz_targets/fuzz_laws.rs:132; crates/before/src/laws/tests.rs:36-51; crates/before/src/span/tests.rs:442-443; crates/before/src/version/tests.rs:26-31)
- Class / severity / confidence: performance / low / high
- Provenance: verified (run1.log: `PASS [ 10.910s] (1/1) before testing::algebraic_laws::tests::version_triple_laws`, alone, load 3.8 to 7.5; the sweep's run.log: 18.311 s as the last of 949 to finish, the next-slowest done at 10.888 s; law count by reading laws.rs:848-1662); executed: yes (run1)
- Verification: confirmed with the absolute number corrected: 10.9 s alone, not 18.3 s (the sweep's figure carried about 1.7x load inflation), and the test remains the longest by a wide margin and the last to finish; history: deliberate-and-holds for the roster design (laws.rs:94-98 anticipates several groups per signature), no-rationale-found for bundling both cost classes in one group
- Owner-gated: no (`before::laws` is exposed only under the `laws` feature, which Cargo.toml:69-73 declares test/fuzz-only)

The suite's wall time is one test. VERSION_TRIPLE holds 30 laws: twelve cheap lattice, order, and metric laws (`merge_associative` through `lag_antitone_in_the_receiver`) and eighteen span and query laws (`conjunction_is_intersection` through `span_meet_associative`) whose bodies loop over `span_candidates` and `operand_spans` products and re-encode and decode per probe (laws.rs:1218-1219), all serialized in one proptest of 256 cases. The cost per law is intrinsic to the claims; the serialization is not. The roster keys every consumer on the input signature, so a second group with the `(version, version, version)` header is driven by construction in the proptest drivers, the organic drive list, and the fuzz target, and the totality pin picks up any new `pub static`.

Evidence:

       848	    pub static VERSION_TRIPLE: (a: &Version, b: &Version, c: &Version);

        94	/// A group added here is therefore *executed by construction* in every
        95	/// consumer: each consumer keys its expansion arms on the input
        96	/// signature, so a new group with a known signature is driven with no
        97	/// further wiring, and one with a novel signature refuses to compile
        98	/// until every consumer says how to feed it.

      1218	                let redecoded =
      1219	                    Version::decode(&probe.encode()[..]).expect("a stored stream re-decodes");

Resolution: split VERSION_TRIPLE into two `pub static` groups with the same header, the lattice/order/metric laws (852-946) staying and the span and query laws (962-1605) moving to a group such as SPAN_TRIPLE, registered in `for_each_law_group!` with its own driver name; no consumer arm changes. Re-point the two comments that name the group (span/tests.rs:442-443; version/tests.rs:26-31). Measure each half once on a quiet machine before choosing the cut. Acceptance: same predicates, same `arb_oracle_version` generator, 256 cases per group; `every_law_group_is_registered` and `law_names_are_unique_across_groups` pass unchanged; in a full `just test-all` the last test to finish is no longer alone by several seconds.

### suite-economics-3: Five hand-rolled source scanners in before/tests beside the surface-scan crate
- Where: crates/before/tests/amp_board_smoke.rs:314-340 (related: crates/surface-scan/src/lib.rs:168-196; crates/before/tests/doc_hidden.rs:25-43; crates/before/tests/foreign_reexport.rs:62-99; crates/before/tests/superlinear_tripwires.rs:14-16 and 72-98; crates/before/tests/verdict_matrix.rs:1232-1287; crates/before/Cargo.toml:50; crates/surface-scan/Cargo.toml:5)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (read all five scanners and `test_fns`; `grep -rn surface_scan crates/before` finds only src/testing/surface_coverage.rs:162 and 272, so no tests/ binary uses the dev-dependency declared at Cargo.toml:50); executed: no
- Verification: confirmed; history: no-rationale-found (surface-scan was cut down from a shared crate in 0a5bdaebd with `extract_public_fns` and `test_fns` as its scope; the roster pins predate it (f10f5b56f, 8409fe552, 9daec56ec) and were not migrated; nothing records a reason)
- Owner-gated: no

`band_test_names` is `surface_scan::test_fns` line for line with a name filter added, and four roster pins each carry a private recursive directory walk with its own fn-name discipline: superlinear_tripwires matches any trimmed `fn ` line (not attribute-gated, misses `pub fn`), verdict_matrix strips visibility and qualifiers but is also not attribute-gated, doc_hidden counts substring occurrences, foreign_reexport filters lines. surface-scan's own manifest calls it "the workspace's shared test machinery", so the circular-justification tell applies: the shared tool exists so rosters do not each re-derive the scan, yet four before rosters do, and three disciplines for one job means three blind spots a reviewer reasons about separately. The doc at superlinear_tripwires.rs:15 promises `#[test]` fns while the scanner at line 80 does not check the attribute.

Evidence:

       314	fn band_test_names(source: &str) -> BTreeSet<String> {
       315	    let mut names = BTreeSet::new();
       316	    let mut armed = false;
       317	    for line in source.lines() {
       318	        let t = line.trim();
       319	        if t == "#[test]" {
       320	            armed = true;
       321	            continue;
       322	        }

    (crates/surface-scan/src/lib.rs)
       174	pub fn test_fns(source: &str) -> BTreeSet<String> {
       175	    let mut names = BTreeSet::new();
       176	    let mut armed = false;
       177	    for line in source.lines() {
       178	        let t = line.trim();
       179	        if t == "#[test]" {
       180	            armed = true;
       181	            continue;
       182	        }

    (crates/before/tests/superlinear_tripwires.rs)
        15	//! must match the `#[test]` fns whose names carry `_reads_superlinear`,
        80	                let Some(rest) = line.trim_start().strip_prefix("fn ") else {

Resolution: add one directory walker to surface-scan (a `walk_rs_sources(root)` yielding `(PathBuf, String)` per `.rs` file) and compose each pin from it plus `test_fns` or a line filter; `band_test_names` becomes `test_fns(source)` filtered on the band convention. Acceptance: the four pins keep their rosters and pass unchanged; superlinear_tripwires.rs:15 is accurate because the scan is attribute-gated; no `fn scan(` remains under crates/before/tests.

### suite-economics-4: Satellite meter binaries duplicate meter.rs fixtures and helpers, and the copied fixture has already drifted
- Where: crates/before/tests/coincident_span.rs:20-50 (related: crates/before/tests/meter.rs:10556-10587 and 769-781; crates/before/tests/answer_embedded.rs:31-41, 114, and 193; crates/before/tests/fold_skeleton.rs:20-30 and 40-42; crates/before/tests/meter.rs:10; crates/before/Cargo.toml:49 and 110-125; crates/before/tests/coincident_span.rs:13; crates/before/tests/answer_embedded.rs:25; crates/before/tests/fold_skeleton.rs:14)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (compared by reading; binary inventory from `wc -l` and the sweep's per_binary.txt: nine of thirteen before integration binaries hold at most six tests and under 0.1 s of summed work); executed: no
- Verification: confirmed and sharpened: the sweep called the fixture a verbatim copy, but the bodies are identical while the return tuples differ in order (`(v, w, redecoded)` at coincident_span.rs:49 against `(v, redecoded, w)` at meter.rs:10586), so the copies have already diverged; history: no-rationale-found (47b03e89 and 4398dcd4f state each witness's purpose and mutant, not why it lives outside tests/meter.rs)
- Owner-gated: no

coincident_span.rs copies `identity_fast_paths::fixture` (same six forked clocks, same 24-round schedule, same re-decode) and its `scanned` helper; answer_embedded.rs and fold_skeleton.rs carry identical `counters` helpers whose reset trio is `ticks_counters`'s body. Each satellite also picks its own flatness tolerance (x1.10 and a tenth of the top cell in answer_embedded.rs:193 and :114; x1.35 in fold_skeleton.rs:42) beside the suite's stated x1.25 (meter.rs:10; fold_skeleton.rs:40-41 cites it and widens). The three are cfg-gated at the file top, so `just test` (default features: the self-dev-dependency at Cargo.toml:49 enables `oracle` and `meter` only) builds and links three empty harnesses; the manifest declares `required-features` for the examples (115-117, 123-125) and has no `[[test]]` section. A fixture that exists twice drifts twice (it already has), and a reader of the flatness bands meets three tolerances derived at three sites.

Evidence:

        28	fn fixture() -> (Version, Version, Version) {
        29	    let mut main = Clock::seed();
        30	    let mut others: Vec<Clock> = (0..6).map(|_| main.fork()).collect();
        49	    (v, w, redecoded)

    (crates/before/tests/meter.rs)
     10565	    fn fixture() -> (Version, Version, Version) {
     10566	        let mut main = Clock::seed();
     10567	        let mut others: Vec<Clock> = (0..6).map(|_| main.fork()).collect();
     10586	        (v, redecoded, w)

    (crates/before/tests/fold_skeleton.rs)
        40	/// The per-byte flatness band across a doubling: the meter suite's
        41	/// ×1.25 flatness convention with margin for amortization wobble.
        42	const GROWTH_BOUND: f64 = 1.35;

Resolution: move coincident_span's three tests into meter.rs's `identity_fast_paths` (one `fixture`, one `scanned`); place answer_embedded and fold_skeleton beside `answer_embedded_product` in meter.rs with one counters helper and each band's tolerance derived at its site; for any feature-gated binary that stays separate, declare `[[test]] required-features` as the examples already do, so the unfeatured build skips it instead of linking an empty harness. Folding the four tiny roster binaries into one is a taste call whose link-time benefit is assessed, not measured. Acceptance: one `fn fixture()` and one counters helper across tests/; `cargo nextest list --workspace` under default features lists no zero-test before binary.

### suite-economics-5: Wall-time claims and measurement narratives in test prose are unenforced, and exhaustive_small's is contradicted by an order of magnitude
- Where: crates/before/src/testing/exhaustive.rs:37-39 (related: exhaustive.rs:73-75; crates/before/src/testing/exhaustive/tests.rs:427-436 and 442-443; .config/nextest.toml:26; crates/suanpan/src/accumulator/tests/ledger.rs:212-214)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (run2.log: `PASS [ 12.914s] (7/7) before testing::exhaustive::tests::exhaustive_small`, load 5.3 rising to 19.9 from its own rayon pool; the sweep's run: 8.372 s at load 5 to 19; suanpan's ledger 2.545 s in run2 and 1.633 s in the sweep); executed: yes (run2)
- Verification: confirmed for exhaustive_small; reframed for the rest: the ledger and exhaustive_deep passages are measurement reports at declaration sites, not contradicted claims, and the amp_board_smoke site is dropped (see Dropped); history: no-rationale-found; exhaustive.rs:44-45 deliberately delegates "the measured state and how to run it" to the exhaustive_deep doc comment, which explains the narrative's placement but not its dated form
- Owner-gated: no

exhaustive.rs states twice that the small-scope cross-product is "well under a second"; in the dev profile the gate runs, it takes 8 to 13 s wall on a 16-core rayon pool. exhaustive/tests.rs:427-436 narrates two stopped runs and a profile as the deep variant's budget, and :442-443 restates nextest.toml:26's `60s` x `3` as "180 seconds"; ledger.rs:212-214 records "~4 s dev" and a one-time pass of the length-7 sweep "at pin time". Under Principle 5 a second-count in a doc is a number the code can change without touching the prose, and measurement reports live in commits, not declaration sites.

Evidence:

        37	//! - [`exhaustive_small`] runs every check in the normal gate at
        38	//!   [`ID_SMALL_DEPTH`] / [`EV_SMALL_DEPTH`] (256 ids, 691 events); the full op
        39	//!   cross-product is well under a second.

    (crates/before/src/testing/exhaustive/tests.rs)
       442	/// (`cargo test`, not nextest: the workspace's nextest profile terminates
       443	/// any test at 180 seconds, which this enumeration exceeds).

    (crates/suanpan/src/accumulator/tests/ledger.rs)
       212	/// Exhaustive over all 11-op schedules of length ≤ 6 (1,948,716
       213	/// states, each checked once; ~4 s dev — the length-≤ 7 sweep's
       214	/// 21.4M states also passed once, at pin time): word-scale deltas,

Resolution: state the mechanism that bounds the cost and drop the seconds. exhaustive.rs:37-39 and :73-75 keep the corpus sizes (256 ids, 691 events) and name the rayon pool; ledger.rs:212-214 keeps the schedule space and drops the timing and the pin-time note; exhaustive/tests.rs:427-436 keeps the order of magnitude ("budget upwards of an hour, run detached") and the reason a strided sample under-prices it (the structurally similar pairs concentrate near the diagonal) and drops the two-runs narrative; :442-443 names the nextest profile's terminate budget without restating its number. Acceptance: no second-count or "at pin time" narrative remains in the cited passages while the corpus sizes and the schedule count do.

### suite-economics-6: Two #[ignore]d tests are run by no recipe
- Where: crates/before/src/codec/tests.rs:1959-1976 (related: crates/before/src/codec/text.rs:192-194; crates/before/src/testing/exhaustive/tests.rs:436-446; justfile:1000-1003; .config/nextest.toml:26)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -rn 'run-ignored\|--ignored\|include-ignored' justfile .github .config` returns nothing; the sweep's run.log shows both tests SKIP); executed: no
- Verification: confirmed and reframed toward the cheaper fix: the depth counter is an explicit `i64` whose non-overflow argument sits inline at text.rs:192-193, so the dormant witness pins less than its doc argues (it pushes past i32::MAX only, not "any physically representable input"), and it discriminates an i32 counter only in the dev profile, where the overflow panics; in release the wrapped counter goes negative at the closing paren and the parse returns `Err(Parse::Syntax)` exactly as the test expects; history: no-rationale-found
- Owner-gated: no

Neither the justfile, the CI workflows, nor nextest.toml runs ignored tests, so `clock_text_split_survives_two_gib_of_parens` and `exhaustive_deep` are compiled and never executed by any sweep; a board nothing enforces is decoration. The deep enumeration is hour-scale and says so. The 2 GiB witness costs seconds and could ride `just all`, or the property it pins could be asserted at the counter without the allocation.

Evidence:

      1966	/// Ignored because the witness string alone costs ~2 GiB of memory; run it
      1967	/// deliberately with `--run-ignored all`.
      1968	#[test]
      1969	#[ignore = "allocates ~2 GiB to push the depth counter past i32::MAX"]
      1970	fn clock_text_split_survives_two_gib_of_parens() {

    (crates/before/src/codec/text.rs)
       192	    // i64 cannot overflow: depth moves by at most one per input byte, and an
       193	    // allocation holds at most `isize::MAX` (< 2⁶³) bytes.
       194	    let mut depth: i64 = 0;

Resolution: either add a filtered ignored run to the `all` recipe (`cargo nextest run -p before --all-features --run-ignored only -E 'test(clock_text_split_survives_two_gib_of_parens)'`), or pin the width at the site (name the counter type and add a `const` assertion that its bit width is at least 64, with the argument at text.rs:192-193 restated beside it) and retire the witness. exhaustive_deep stays as documented. Acceptance: `just all`'s nextest output shows the witness as PASS rather than SKIP, or the compile-time pin exists and the `#[ignore]` test is gone.

### suite-economics-7: surface-scan test fixtures are written under the shared temp dir and never removed
- Where: crates/surface-scan/src/tests.rs:14-25 (related: none)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read; `grep -rn tempfile Cargo.toml crates/*/Cargo.toml` returns nothing while Cargo.lock already carries `tempfile` transitively); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`fixture` hand-rolls a unique directory under `std::env::temp_dir()` keyed by test name and pid and never deletes it, so every run leaves `surface-scan-fixture-*` directories behind. The doctrine prefers a dependency over hand-rolling, and `tempfile::tempdir()` gives the uniqueness the comment argues for plus removal on drop.

Evidence:

        18	    let dir = std::env::temp_dir().join(format!(
        19	        "surface-scan-fixture-{name}-{}",
        20	        std::process::id()
        21	    ));
        22	    fs::create_dir_all(&dir).expect("creating the fixture dir");

Resolution: dev-depend on `tempfile` (already in the lock) and return a `TempDir` from `fixture`. Acceptance: no `surface-scan-fixture-*` directory remains under the temp dir after `cargo nextest run -p surface-scan`.

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
