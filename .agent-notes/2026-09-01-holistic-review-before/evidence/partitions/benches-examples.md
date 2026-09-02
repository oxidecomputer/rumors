# Partition benches-examples: Criterion benches (party, version, clock, amplify, board, presize, tripwire, common, sidecar) and the examples

## Partition summary

This partition is `before`'s wall-clock instrumentation and its offline studies. Three differential benches (party.rs, version.rs, clock.rs) time the packed implementation against the recursive oracle on structurally identical randomized trees built by benches/common/mod.rs from one `Plan`; two judge-facing targets (board.rs, tripwire.rs) time every amplification-board cell and a known-bad quadratic, writing a stamped denominator sidecar (common/sidecar.rs) that `tools/benchjudge` divides by to fit time exponents across two scales; amplify.rs re-times three adversarial shapes at fixed sizes; presize.rs is an allocation-strategy A/B record with compile-time arms in the library. The examples are the board's process shell (amp_board.rs), the fuzz-seed regenerator (fuzz_seeds.rs), the paper's space-consumption experiment (space_consumption.rs), and three study or probe binaries (code_study.rs, perf_probe.rs, emit_probe.rs). I read all fifteen files in full with line numbers (3,709 lines), plus the out-of-partition anchors every finding rests on (the skyline `causal_cmp` rung, the board's op and family tables, `bench_cells`, the alloc seams, the judge, the roster pin test, the results directories, and the cargo registry sources of peak_alloc 0.3.0 and criterion 0.5.1). None of the partition files is a test; the partition's checker tests (tests/bench_judge_roster.rs, tests/fuzz_seeds.rs) were read where cited. No cargo, just, or bench command was run: every verdict rests on reading, grep, and read-only git.

The judge-facing apparatus is the strongest code here and is built the way the doctrine asks. board.rs enumerates the board's own cell table so a red board cell names the bench that times it and there is no second list; the sidecar stamps scale, profile, sampling mode, and source tip so the judge refuses a lo/hi pair from different configurations; the ceiling class rides the sidecar, never the roster; two committed known-bad demonstrators (the unmetered quadratic in tripwire.rs, the schoolbook decimal renderer in board.rs, asserted at setup to spell the identical value) must read red every sweep, and the judge's `--self-test` pins their measured shapes. amp_board.rs owns only what the library cannot (the global allocator, process spawning, the CLI) and passes the shard scale as exact f64 bits; fuzz_seeds.rs writes atomically and shares its derivation with its checker test by `#[path]`. space_consumption.rs reproduces the paper's parameters exactly and reports bits beside bytes with the reason stated.

The defects cluster at phase boundaries nobody re-audited. Three instruments outlived the constraint that justified them: amplify.rs (written one commit before board.rs, which derives the same three cells under the judge), emit_probe.rs (compares bitvec against a standalone re-implementation, so it cannot see the crate regress, and holds bitvec as a dev-dependency for a decision that landed), and the presize A/B leg (production seams, no recorded verdict, an unpinned deterministic column; its sibling stacks leg from the same commit was closed the next day with a ratified verdict). One hard-rule breach: code_study.rs's stated reason to exist links a `before::implementation` essay deleted on 2026-08-17, and no lint can see it because `cargo doc` never documents examples. One instrument defect of substance: the `version/partial_cmp` `equal` row passes the same reference twice and so times the ptr_eq identity rung against the oracle's two full walks, has done so in one form or another since the benches' first day, and its 2026-06-02 "~31x" leaked into a committed results directory whose plot script now names dead bench IDs. Around these sit the expected precision gaps: a ceiling-class assert that compares the pinned set with itself for every board cell after 514cbcea made board cells derive from the pin, a stamp doc stronger than the judge's cross-check, a scale knob whose message names a constraint the code does not check, hand-synchronized copies of the corpus and simulation with parity asserted in prose, and a handful of doc slips (recv borrows; Version is Clone; literals for the judge's constants).

## Findings

### benches-examples-1: benches/amplify.rs re-lists three board cells the judged board bench already times
- Where: crates/before/benches/amplify.rs:1-24 (related: crates/before/Cargo.toml:139-141; crates/before/benches/board.rs:4-12; crates/before/src/meter/board/export.rs:3-11, 144-150; crates/before/src/meter/board/ops.rs:112-115, 197, 266, 1428-1441; crates/before/src/meter/board/family.rs:524-533, 797-803; crates/before/src/meter/registry.rs:1261-1299)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (read the board's op rows `version_decode`, `version_join`, `party_without`, the family specs for hugeleaf, bigroot, id-pair with `Coverage::Board`, `designed` at ops.rs:112-115 admitting all three pairings, and `bench_cells`'s pinned rule; `grep -rn amplify` across the tree hits only Cargo.toml:140 and the file itself; `git log --follow` dates amplify.rs to 67a5afe6 and board.rs to 8138cb9c, both 2026-07-23, amplify one commit earlier); executed: no
- Seen by: scaffolding [0], adequacy [18], structure-prose [29], instrument-correctness [54]; refutation: confirmed, with one correction to the equivalence (below); history: no rationale found (never re-justified after board.rs landed; 608dea84 maintained it without saying why it stays)
- Owner-gated: yes (removes a `[[bench]]` target)

amplify.rs hand-lists three op x shape rows at two fixed sizes each, with no denominator sidecar, no ceiling, and no consumer, while benches/board.rs derives the same three pairings (`version_decode/hugeleaf`, `version_join/bigroot`, `party_without/id-pair`) from the board's axes and times them under the judge with the scale knob. board.rs:12 and export.rs:10-11 state that the bench cell set is "never a second list". Principle 3 (circular justification): the file's only reason to exist is that it predates board.rs by one commit. One operand differs: `bench_join_bigroot` joins bigroot with a fresh one-tick version (line 47) where the board's `version_join x bigroot` uses `version2`, the bigroot ticked once by `Party::seed()` (family.rs:799-802).

Evidence:

         1	//! Adversarial-shape benchmarks: the `before::meter` generator inputs, timed
         2	//! at small sizes so the resource-proportionality paths register as
         3	//! wall-clock numbers.
         4	//!
         5	//! No oracle comparison: these rows exist to make a superlinear regression
         6	//! on a worst-case shape visible in `cargo bench`, complementing the
         7	//! deterministic envelopes in `tests/meter.rs`.
        16	const HUGELEAF_BITS: &[usize] = &[8_192, 32_768];
        20	const BIGROOT_SIZES: &[(usize, usize)] = &[(4_096, 256), (16_384, 1_024)];
        24	const ID_SPINE_DEPTH: &[usize] = &[8_192, 32_768];
        47	        let one = Version::try_from(1u64).expect("a one-tick version is valid");

    board.rs:
        12	//! derived from the board's own axis declarations, never a second list. The deterministic record stays with the board and the

Resolution: Delete benches/amplify.rs and the `[[bench]] name = "amplify"` stanza (Cargo.toml:139-141). If the join-with-a-one-tick-version operand is wanted, add it as a board row so it gets the judge, rather than keeping a parallel list. Acceptance: `grep -rn amplify crates/before/benches crates/before/Cargo.toml` is empty; `just bench-build` and `just clippy` stay green; the pinned sidecar written by `just bench-judge` still lists `version_decode/hugeleaf`, `version_join/bigroot`, and `party_without/id-pair`.

### benches-examples-2: Mechanical slips in board.rs and presize.rs: a 126-column doc line, a split import group, a forward-looking clause
- Where: crates/before/benches/board.rs:12-12 (related: crates/before/benches/board.rs:81-86; crates/before/benches/presize.rs:258-259)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (awk `length` of board.rs:12 is 126; it is the only line over 100 columns in the benches apart from box-drawing separators; no rustfmt.toml in the repo, and rustfmt neither re-wraps comments nor merges import groups across a blank line); executed: no
- Seen by: adequacy [24], structure-prose [42], [44], scaffolding [11 part]; refutation: confirmed; history: no rationale for the first two (3eadcb10 inserted mid-paragraph; 608dea84 inserted the `Shape` import above the std group); the presize design stance is deliberate (b28c35ad: "no seam lands ahead of evidence"), so only the sentence's mood is at issue
- Owner-gated: no

Three legibility slips the formatter does not repair: an unwrapped doc line, `before` imports split around a stray std import, and a plan ("a seam waits on that evidence") where the doc should describe what the bench is (Principle 5: prose describes what IS).

Evidence:

        12	//! derived from the board's own axis declarations, never a second list. The deterministic record stays with the board and the

        81	use before::meter::registry::Shape;
        82	use std::time::Duration;
        83	
        84	use before::meter::board;
        85	use before::Version;

    presize.rs:
       258	/// No alternative arm is compiled for this site — the resident table
       259	/// carries its slack evidence, and a seam waits on that evidence.

Resolution: Re-wrap board.rs:12-13 at the paragraph's width; move `use std::time::Duration;` into its own leading group so the `before` imports are contiguous; reword presize.rs:258-259 to "No alternative arm is compiled for this site; the resident table records its slack." Acceptance: no module-doc line in board.rs exceeds the file's wrap width; `before` imports contiguous; the presize sentence carries no forward-looking clause.

### benches-examples-3: Bench doc comments misstate operand ownership: `recv` borrows, `Version` is `Clone`
- Where: crates/before/benches/clock.rs:167-168 (related: crates/before/benches/clock.rs:185-187, 199-201; crates/before/benches/common/mod.rs:22-24; crates/before/src/clock.rs:473; crates/before/src/oracle/clock.rs:140; crates/before/src/version.rs:91-94)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (src/clock.rs:473 `pub fn recv(&mut self, version: &Version) -> &Version`; src/oracle/clock.rs:140 `pub fn receive(&mut self, msg: Version)`; src/version.rs:94 `#[derive(Clone, Eq)] pub struct Version` with the comment that a clone shares the buffer; benches/clock.rs:185 passes `msg` through setup by reference with no clone, while the oracle arm at 199 clones); executed: no
- Seen by: scaffolding [11], adequacy [22], structure-prose [31]; refutation: confirmed; history: the recv doc expired at dc88e755 (the call was updated to `recv`, the doc was not); the "not `Clone`" phrase was inaccurate for Version from the benches' first commit, its referent having always been Party/Clock
- Owner-gated: no

Every bench doc comment must be accurate to its body. `bench_receive` says the message "is consumed" and "clones cheaply in setup", which is true only of the oracle arm; common/mod.rs says the impl is decoded because it "is not `Clone`", which is true of Party and Clock but false of Version, and hides the real reason decode is used for versions (each iteration gets a distinct buffer, which the ptr_eq rung makes observable).

Evidence:

       167	/// `receive`: merge an incoming message, then tick. The message is consumed; the clock is
       168	/// mutated. Both operands fresh per iteration (the message clones cheaply in setup).

    common/mod.rs:
        22	//! Generation lives outside the timed region; the benches only clone (oracle)
        23	//! or `decode` (impl, which is not `Clone`) a prebuilt template to get a fresh
        24	//! value per iteration.

Resolution: clock.rs: "The clock is mutated and rebuilt per iteration; the impl borrows the message (`recv(&Version)`), the oracle consumes a clone made in setup." common/mod.rs:22-24: "or `decode` (impl: `Party` and `Clock` are not `Clone`, and a `Version` clone shares its buffer, so decode gives each iteration a distinct one)". Acceptance: both comments match the signatures they describe; no bench doc claims Version is not Clone.

### benches-examples-4: benches/common spells the universe build and group fold five times, carries dead `map_err` adapters, and has `rng` copied into four targets
- Where: crates/before/benches/common/mod.rs:104-239 (related: crates/before/benches/common/mod.rs:169, 204, 232; crates/before/benches/party.rs:13-17; crates/before/benches/version.rs:15-17; crates/before/benches/clock.rs:12-14; crates/before/benches/presize.rs:53-55; crates/before/examples/perf_probe.rs:71, 104, 135, 181, 341; crates/before/src/clock.rs:218, 916)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (side-by-side read of `impl_parties`, `oracle_parties`, `impl_clocks`, `hole_pair`, `oracle_clocks`: the same fork-by-schedule loop and, in four of them, the same take_group/reduce/join fold; `impl Debug for Clock` at src/clock.rs:916 is unconditional and `Clock::join` returns `Result<&Version, Clock>` at :218, so `.expect` compiles without the adapter, and the Party fold at mod.rs:116 already calls `expect` directly; `fn rng(salt)` appears in four bench targets, documented in one); executed: no
- Seen by: scaffolding [12], structure-prose [32], [33]; refutation: confirmed; history: no rationale (the original 6e69b427 shape; the adapter was dead at origin)
- Owner-gated: no

Legibility: the module's central promise (line 124, "same plan, structurally identical trees") is a property of one code path only if there is one code path; five transcriptions must be diffed by eye. The `.map_err(|_| ())` before `expect` says nothing, because `Clock: Debug`; the party fold beside it shows the plain form.

Evidence:

       104	pub fn impl_parties(plan: &Plan, groups: u8) -> Vec<Party> {
       105	    let mut universe = vec![Party::seed()];
       106	    for &i in &plan.schedule {
       107	        let child = universe[i].fork();
       108	        universe.push(child);
       109	    }

       167	                .reduce(|mut acc, c| {
       168	                    acc.join(c)
       169	                        .map_err(|_| ())
       170	                        .expect("universe members are pairwise disjoint");

Resolution: two private generics, `universe<T>(seed: T, schedule: &[usize], fork: impl FnMut(&mut T) -> T) -> Vec<T>` and `fold_groups<T>(slots: Vec<T>, label: &[u8], groups: u8, join: impl FnMut(&mut T, T)) -> Vec<T>`; express the five builders through them (`hole_pair` reuses `universe` and a full fold); delete every `.map_err(|_| ())` (also in perf_probe.rs); move `rng` with party.rs's doc comment into common as `pub fn rng(salt: u64) -> StdRng`. Acceptance: one fork loop and one group fold in the module; `grep -rn 'map_err(|_| ())' crates/before/benches crates/before/examples` returns nothing; one `fn rng` under crates/before/benches; bench IDs, seeds, salts, and inputs unchanged.

### benches-examples-5: `alloc_arms` is a third hand-spelled copy of the A/B arm roster, with no check against the manifest
- Where: crates/before/benches/common/mod.rs:266-281 (related: crates/before/Cargo.toml:97-108; justfile:807-809; crates/before/benches/presize.rs:144-147)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (Cargo.toml:107 `check-cfg = ['cfg(before_alloc_ab, values("projection_growth", "projection_shrink", "display_growth"))']`; justfile:808 case list `(shipped|projection_growth|projection_shrink|display_growth)`; mod.rs:268-278 three `cfg!` literals with `"shipped"` as the empty-case fallback); executed: no
- Seen by: structure-prose [38]; refutation: confirmed; history: no rationale (Cargo.toml:103-105 names the mistyped-RUSTFLAGS hole and routes it to the recipe, not the omission hole)
- Owner-gated: no

Principle 6 (the cheapest passing artifact): the stamp exists so a baseline "can never be mis-attributed to the wrong build" (mod.rs:263-265), yet an arm added to the manifest and a seam but not to `alloc_arms` stamps `arm=shipped` on a non-shipped build; `deny(unexpected_cfgs)` catches a misspelled value in `cfg!`, never an omitted one. Moot if benches-examples-12 closes the leg.

Evidence:

       266	pub fn alloc_arms() -> String {
       267	    let mut arms = Vec::new();
       268	    if cfg!(before_alloc_ab = "projection_growth") {
       269	        arms.push("projection_growth");
       270	    }
       277	    if arms.is_empty() {
       278	        arms.push("shipped");
       279	    }

Resolution: one `pub const ALLOC_ARMS: &[&str]` from which a small macro generates the `cfg!` checks, and a test that parses Cargo.toml's check-cfg line and asserts its value list equals the constant; or have `bench-alloc-ab` pass the arm through an environment variable the bench stamps, cross-checked against one `cfg!`. Acceptance: adding a value at Cargo.toml:107 without touching benches/common fails a committed test.
Construction: add `"parse_growth"` to the `values(...)` list at Cargo.toml:107 and a `#[cfg(before_alloc_ab = "parse_growth")]` seam anywhere in the library; `RUSTFLAGS='--cfg before_alloc_ab="parse_growth"' cargo bench -p before --bench presize --no-run` builds clean, and running it prints `arm=shipped` on every `presize-resident` line.

### benches-examples-6: The sidecar stamp binds sidecar to sidecar and to `--tip`, not to the criterion baseline
- Where: crates/before/benches/common/sidecar.rs:16-18 (related: tools/benchjudge:373-393, 396-411; justfile:843-845; crates/before/benches/board.rs:159)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (tools/benchjudge `cross_check_stamps` compares `profile`, `sampling`, `tip` between the two sidecars, requires `scale` to grow, and checks each stamp against `--tip`; `read_median` reads only `estimates["median"]["point_estimate"]` from criterion's estimates.json, which carries no stamp; justfile:843-845 sets the tip to `git rev-parse HEAD`); executed: no
- Seen by: adequacy [20]; refutation: confirmed; history: no rationale (the claim and the judge both date to b4942461, and the judge's error text "a leftover baseline from another run?" refers to the stamp-disagreement path)
- Owner-gated: no

Provenance discipline ("bind every measurement to its run; refuse mismatches"): the doc promises the judge can "refuse a sidecar/baseline pair assembled from different runs", but criterion's saved baseline carries no stamp, so a sidecar and a baseline from different runs are never compared, and `git rev-parse HEAD` is shared by a dirty tree and its clean commit. The recipes rewrite both artifacts in one flow, which bounds the exposure; the stated guarantee is still stronger than the mechanism.

Evidence:

        16	//! The stamp binds the sidecar to the bench run that wrote it, so
        17	//! `tools/benchjudge` can refuse a sidecar/baseline pair assembled from
        18	//! different runs (exit 2) instead of silently re-scoring: the resolved

    tools/benchjudge:
       375	    for field in ("profile", "sampling", "tip"):
       407	        return estimates["median"]["point_estimate"]

Resolution: State the binding precisely at sidecar.rs:16-25 (sidecar-to-sidecar and sidecar-to-invocation; the baseline is trusted to be the same run's). To close the gap instead: stamp `git describe --always --dirty` (or refuse when `git status --porcelain` is nonempty) in the recipes, and have the judge require each judged cell's estimates.json to be no older than its sidecar, which is written before any cell runs (board.rs:159). Acceptance: the doc names exactly the two cross-checks the judge performs, or the construction below exits 2 naming the stale cell.
Construction: run `just bench-judge` clean; make an uncommitted edit that makes one pinned op quadratic; re-run only the lo pass with the recipe's environment (the justfile:843 line) and invoke the judge line (justfile:845) by hand: both sidecars stamp the same tip, profile, and sampling, the hi medians are the pre-edit tree's, and the judge scores the pair.

### benches-examples-7: Judge constants, measured exponents, and a hand cell count restated as literals at declaration sites
- Where: crates/before/benches/common/sidecar.rs:62-85 (related: crates/before/benches/common/sidecar.rs:36-43; crates/before/benches/board.rs:53-68, 91-96; crates/before/benches/tripwire.rs:8-14, 27-32; tools/benchjudge:121, 138, 160; tools/benchjudge-expected.json:2)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (the constants live as `MAX_WALL_SCALING_EXPONENT = 1.3` (benchjudge:121), `MAX_TEXT_SCALING_EXPONENT = 1.7` (:138), and `MIN_JUDGED_MEDIAN_NANOS` (:160); the same measured figures recur in the roster's `notes` at benchjudge-expected.json:2); executed: no
- Seen by: scaffolding [8], structure-prose [37], scaffolding [11 part]; refutation: confirmed; history: the measured exponents were placed deliberately (514cbcea, b1c07fe1) as the declared model's evidence per the house pattern of disclosing a declared model on its row face, a rationale that lives only in commit messages and the amplification note; the constant literals and the "~200 cells" count have no recorded rationale
- Owner-gated: no

Principle 5: a number that matters lives in one mechanically enforced place that prose cites by name. "general 1.3, text 1.7", "the 1.3 ceiling", and "the judge's 10 µs floor" are literals that rot when the judge's constants move; "the full surface is ~200 cells" is a hand-maintained count. The measured exponents (1.39/1.42, 1.28/1.33/1.30, "e ≈ 1.5", "e ≈ 2.0") appear both here and in the roster notes, so two homes hold one measurement. Since placing them at the declaration is a deliberate pattern, the fix is to say so at the site or to keep one home, not to delete them unexamined.

Evidence:

        73	/// (`version_display`, `clock_display`) renders binary→decimal —
        74	/// measured exponents 1.39/1.42. Inbound, the hugeleaf parse trio
        75	/// (`version_parse_trailing`, `version_parse_noncanon`,
        76	/// `clock_parse_trailing`) converts hugeleaf-width decimal literals
        77	/// decimal→binary on the way to the placed defect — measured exponents
        78	/// 1.28/1.33/1.30.
        79	/// (Both measurements: quick sampling, bench profile, against the 1.7
        80	/// text ceiling; a quadratic conversion still reads red there.)

    board.rs:
        95	/// full surface is ~200 cells — the committed windows are what keeps a

Resolution: cite `MAX_WALL_SCALING_EXPONENT`, `MAX_TEXT_SCALING_EXPONENT`, and `MIN_JUDGED_MEDIAN_NANOS` by name at sidecar.rs:38-39, board.rs:64-65, tripwire.rs:11 and :30; replace "~200 cells" with "the whole shape × operation product"; for the measured exponents, either state at sidecar.rs:62-85 that they are the declared model's evidence recorded at the declaration and drop the duplicate in the roster notes, or keep them in one home. Acceptance: `grep -n '1\.3\|1\.7\|10 µs\|~200 cells' crates/before/benches` returns only name-cites or nothing; the measured figures appear in exactly one committed place, or the declaration states why two.

### benches-examples-8: `scale_from_env` says "positive number" but accepts zero, negatives, NaN, infinity, and saturating magnitudes; the tripwire has no downstream guard
- Where: crates/before/benches/common/sidecar.rs:110-119 (related: crates/before/benches/tripwire.rs:56-59; crates/before/benches/common/sidecar.rs:160; crates/before/examples/amp_board.rs:193-198; crates/before/src/meter/board/export.rs:133-137; crates/before/src/meter/board/shard.rs:139-146)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (the parse arm only calls `raw.parse::<f64>()`; `bench_cells` asserts `scale > 0.0 && scale.is_finite()` at export.rs:134-137, which rescues the board target; tripwire.rs:57 computes `((TRIPWIRE_BASE as f64) * scale).round() as usize`, a saturating cast; `{scale:?}` at sidecar.rs:160 renders `NaN` and `inf`, neither of which is JSON; amp_board.rs:195-197 has the same shape and only the child's `assert_scale` (shard.rs:146) fires, so the parent reports "shard child failed"); executed: no
- Seen by: scaffolding [13], adequacy [23], structure-prose [35 part], instrument-correctness [56]; refutation: confirmed, adding the huge-scale saturation; history: no rationale (b4942461 applied its NaN/non-positive discipline to the judge's medians, not the knob; the export.rs assert arrived with sharding)
- Owner-gated: no

Principle 1: a panic message must be true of the check beside it. For the tripwire, `nan` or `-1` yields `n = 0`, a zero-work probe, a `denominator_bytes: 0` cell, and a stamp line `"scale": NaN` that is not JSON, refused by the judge two stages later with a less direct message; a huge value saturates `n` to `usize::MAX` and the `n²` loop never finishes.

Evidence:

       114	        Ok(raw) => raw.parse().unwrap_or_else(|_| {
       115	            panic!("{SCALE_ENV} must be a positive number or `acceptance`, got {raw:?}")
       116	        }),

    tripwire.rs:
        57	    let n = ((TRIPWIRE_BASE as f64) * scale).round() as usize;

Resolution: after parsing, require `scale > 0.0 && scale.is_finite()` inside `scale_from_env` (one site, both bench targets) and at amp_board.rs:195-197, keeping the existing messages. Acceptance: `BOARD_BENCH_SCALE=nan cargo bench -p before --bench tripwire` panics at the knob naming `BOARD_BENCH_SCALE`; likewise for `0`, `-1`, `inf`; `cargo run --example amp_board ... -- -1` panics at the parse site.
Construction: `BOARD_BENCH_SCALE=nan BOARD_BENCH_DENOMS=<scratch>/d.json cargo bench -p before --bench tripwire -- --sample-size 10 --measurement-time 1` completes, and d.json carries `"scale": NaN` and `"denominator_bytes": 0`.

### benches-examples-9: The denominator sidecar hand-rolls JSON with serde_json already a dev-dependency
- Where: crates/before/benches/common/sidecar.rs:159-187 (related: crates/before/Cargo.toml:38; Cargo.toml:69; crates/before/tests/bench_judge_roster.rs:19; tools/benchjudge:438; crates/before/benches/common/sidecar.rs:139-141)
- Class / severity / confidence: simplification / nit / medium
- Provenance: verified (string-built writer read in full; `serde_json = { workspace = true }` at crates/before/Cargo.toml:38, workspace `serde_json = "1"` at Cargo.toml:69 without `preserve_order`; the roster test parses the sidecar with serde_json); executed: no
- Seen by: scaffolding [6], structure-prose [35 part]; refutation: reframed (row order is part of the sidecar's contract and the judge's table follows it, so a plain derive needs `preserve_order` or a schema change); history: no rationale (the writer moved unchanged from board.rs's inline version with serde_json already present)
- Owner-gated: no

Prefer a dependency over hand-rolling: a `#[derive(Serialize)]` pair with `serde_json::to_string_pretty` cannot emit malformed JSON and retires the escaping half of the two charset asserts. The cost: the sidecar promises "one cell object ... per cell in the order given" (139-141) and the judge iterates the cells object in file order, so the swap needs `serde_json/preserve_order` (a workspace-wide feature) or an ordered cells array in the schema. Thirty lines held by tests is an acceptable alternative; the choice is recorded here so it is a choice.

Evidence:

       159	    let mut json = String::from("{\n  \"stamp\": {\n");
       160	    json.push_str(&format!("    \"scale\": {scale:?},\n"));
       161	    json.push_str(&format!("    \"profile\": \"{}\",\n", profile()));

Resolution: if the order caveat is acceptable, derive `Serialize` on a `Sidecar { stamp, cells }` with `preserve_order` enabled, drop the tip charset assert, and keep the cell-id assert only if the judge's ID vocabulary requires it (with a message saying so). Otherwise leave the writer as it is. Acceptance: `tools/benchjudge --self-test` and `just bench-judge-tripwire` behave as before; if swapped, no `format!`-built JSON remains in sidecar.rs.

### benches-examples-10: The ceiling-class assert compares the pinned set with itself for every board cell, and the sidecar prose describes a cell-site declaration board cells do not have
- Where: crates/before/benches/common/sidecar.rs:170-180 (related: crates/before/benches/board.rs:129-158; crates/before/benches/tripwire.rs:59; crates/before/benches/common/sidecar.rs:27-30, 82-85, 143-147; crates/before/tests/bench_judge_roster.rs:102-116; crates/before/src/meter/board/tests.rs:1094-1127; crates/before/src/meter/board/cell.rs:195-198)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (board.rs:141-145 derives `ceiling` from `TEXT_CEILING_CELLS.contains(&id.as_str())` and sidecar.rs:175-177 asserts `*ceiling == Ceiling::Text` against `TEXT_CEILING_CELLS.contains(id)`, the same predicate; only the two wide-pair literals (board.rs:152, 157) and the tripwire's `General` (tripwire.rs:59) are independent inputs, and both wide-pair IDs are in the set while the tripwire's is not; `grep -rn TEXT_CEILING_CELLS` hits sidecar.rs, board.rs, the roster test's literal copy, and the roster notes only; `bench_riders_name_declared_model_cells` (board/tests.rs:1095-1127) checks `BOARD_DECLARED_BENCH_RIDERS` against `bench_cells(0.02, BenchMode::Full)` and no analogous check exists for the text set); executed: no
- Seen by: scaffolding [7], adequacy [19], structure-prose [36], instrument-correctness [48]; refutation: confirmed, severity lowered (a stale entry can only tighten a cell's ceiling or go inert, never launder a class); history: deliberate-but-expired (514cbcea made board cells derive their class from the pin "so a class can only move by editing the pin", leaving f4c3e563's assert and its cell-site-declaration prose in place)
- Owner-gated: no

A guard must name a concrete failure it catches; for board cells this one compares a lookup with itself. sidecar.rs:27-30 ("declared here in bench code at the cell's definition site"), 82-85 ("asserts every declaration against this set ... a two-site edit"), and the `# Panics` section describe a second site board cells do not have; the real second site is the roster test's literal pin. Separately, nothing checks that the five board entries name live cells: a renamed op leaves its entry inert and drops the renamed cell to the stricter general ceiling, visible only as a spurious red at `just all` cadence with no diff pointing at the set.

Evidence:

       175	        assert_eq!(
       176	            *ceiling == Ceiling::Text,
       177	            TEXT_CEILING_CELLS.contains(id),
       178	            "{id}: the text-ceiling set is pinned as TEXT_CEILING_CELLS; \
       179	             declare the class there and at the cell together"
       180	        );

    board.rs:
       141	            let ceiling = if sidecar::TEXT_CEILING_CELLS.contains(&id.as_str()) {
       142	                sidecar::Ceiling::Text
       143	            } else {
       144	                sidecar::Ceiling::General
       145	            };

    sidecar.rs:
        82	/// [`write_denoms`] asserts every declaration against this set, and
        83	/// `tests/bench_judge_roster.rs` pins the set itself — so widening the
        84	/// text class is a two-site edit whose diff a reviewer sees, never a
        85	/// one-character class swap at a cell.

Resolution: make membership the single declaration: `write_denoms` takes `(id, denominator_bytes)` pairs and derives the class from `TEXT_CEILING_CELLS` internally (this reproduces today's sidecar byte for byte, since both wide-pair IDs are in the set and the tripwire's is not); drop the assert and the three literal `Ceiling` arguments; reword sidecar.rs:27-30, 82-85, and 143-147 to "the class is set membership, pinned by tests/bench_judge_roster.rs". Add to tests/bench_judge_roster.rs (which already includes the sidecar module) a test that every entry other than `version_display_wide/hugeleaf` and `display_schoolbook/hugeleaf` is an `op/family` of `board::bench_cells(0.02, BenchMode::Full)`, mirroring `bench_riders_name_declared_model_cells`. Acceptance: `Ceiling` no longer appears in board.rs or tripwire.rs; the sidecar written by `just bench-judge` is byte-identical to before; renaming a text-class op in ops.rs without editing the set fails a gate test naming the stale entry.
Construction: rename `clock_parse_trailing` to `clock_parse_trail` in src/meter/board/ops.rs and its callers, leave `TEXT_CEILING_CELLS` and the roster test untouched; `just test-all` is green; `just bench-judge` judges `clock_parse_trail/hugeleaf` at 1.3, and the stale entry never fires.

### benches-examples-11: fork/join/sync/send routines drop the consumed operand inside the timed region; tick/receive return it
- Where: crates/before/benches/party.rs:34-75 (related: crates/before/benches/party.rs:34, 41, 68, 75; crates/before/benches/clock.rs:59, 66, 92, 99, 126, 133, 152, 159; crates/before/benches/version.rs:47-50; crates/before/benches/clock.rs:27-30, 186-189)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (criterion 0.5.1 src/bencher.rs:251-256: `let output = routine(input); let end = self.measurement.end(start); ... drop(black_box(output));`, so anything the routine does not return is dropped inside the timed span); executed: no
- Seen by: instrument-correctness [49]; refutation: reframed (the inclusion is symmetric across impl and oracle, so it is a scope inconsistency, not a like-for-like bias); history: no rationale (both shapes are original to 6e69b427)
- Owner-gated: no

The suite is inconsistent about what a row times: tick and receive return the mutated value (`black_box(v)`) so its drop is untimed; fork, join, sync, and send return a small value and let the operand drop inside the measured span. Both arms pay their own drop (the impl frees a decoded buffer, the oracle frees the nodes its op built), so no arm is favored, but a reader comparing rows across the suite compares different scopes.

Evidence:

        34	                |mut p| black_box(p.fork()),
        68	                |(mut a, b)| black_box(a.join(b).is_ok()),

    version.rs:
        47	                |mut v| {
        48	                    v.tick(&iparty);
        49	                    black_box(v)
        50	                },

Resolution: return the surviving operands from each routine on both arms (`|mut p| { let s = p.fork(); (p, s) }`, `|(mut a, b)| { let ok = a.join(b).is_ok(); (a, ok) }`, and likewise for sync and send), or state once in common/mod.rs which scope the suite times. Acceptance: every `iter_batched` routine in party.rs and clock.rs returns its surviving operands, or the scope is documented once.

### benches-examples-12: The presize A/B record has arms compiled into production source, no recorded verdict, and an unpinned deterministic column
- Where: crates/before/benches/presize.rs:20-39 (related: crates/before/src/version/skyline/query.rs:506-517, 603-613; crates/before/src/version/skyline/text.rs:345-354; crates/before/Cargo.toml:97-108; justfile:792-809; crates/before/benches/common/mod.rs:259-281; crates/before/benches/presize.rs:136-150; crates/before/src/codec/bits.rs:120-124)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`git log -S projection_shrink` returns only b28c35ad, 2026-07-30; cd171c29 (2026-07-31) closed the sibling stacks leg from the same commit with a ratified verdict and retired its arms, bench, and dependency: "the seam existed to price the choice, and the choice is made"; grep of .agent-notes for the arm names returns nothing; the three cfg seams read at query.rs:513-516, 608-613 and text.rs:351-354; `Bits::freeze` adopts the Vec via `Bytes::from(buf.into_bytes())` (bits.rs:123) and bytes 1.11.1 keeps the Vec's capacity, so stranded slack is real; no committed test names resident bytes); executed: no
- Seen by: scaffolding [2]; refutation: confirmed (correcting the cited bytes version to 1.11.1); history: no rationale found; the sibling leg's closure is the crate's own precedent
- Owner-gated: yes (dissolving an instrument and production seams)

Principle 2 ("acceptance means committed measurements changing, not a report asserting improvement") and Principle 3 ("machinery outlives the constraint that justified it"): the record's one deterministic quantity, end-state resident bytes, is printed from a bench binary and pinned by nothing; the wall arms sit in production source that every reader of `own_version_to_version` and the text renderer must reason past; and a month on, no commit or note records what the record decided.

Evidence:

        20	//! The A/B sides are compile-time arms of the library, selected per site by
        21	//! `RUSTFLAGS='--cfg before_alloc_ab="<arm>"'` (the `bench-alloc-ab`
        22	//! recipe): `projection_growth` and `display_growth` start the site's
        23	//! buffer empty, `projection_shrink` adds one exact-size copy where the
        24	//! buffer becomes storage. Nothing in-process distinguishes the sides, so
        25	//! each run saves a criterion baseline named after its arm, and every line
        26	//! of the resident-bytes table below is stamped with the compiled arm.
        27	//!
        28	//! Beside the wall cells, the binary prints one `presize-resident` line per
        29	//! site and input before criterion runs: the live-heap delta of building
        30	//! and holding the operation's result, measured by this binary's counting
        31	//! allocator. That column is deterministic (allocation *requests*, in
        32	//! bytes; the platform allocator's size-class rounding is deliberately out
        33	//! of frame) and is the record's evidence on the copy-vs-stranded-slack
        34	//! trade.

Resolution: close the leg the way the stacks leg was closed: take the record per the recipe's protocol (one `shipped` run plus one run per arm), write the verdict into a note and the closing commit, then dissolve the three cfg seams, the check-cfg roster, `bench-alloc-ab`, and `alloc_arms`. Keep only what the other benches lack: the `projection_outgrow` family (unique to this file) can move to benches/version.rs's hole group; `presize/display` and `presize/parse` duplicate the board's `version_display` and `version_parse_*` rows. If end-state resident bytes matter, pin them as a deterministic test in tests/meter.rs (frozen capacity minus length per site, with a floor) rather than printing them. Acceptance: `grep -rn before_alloc_ab crates/before Cargo.toml justfile` is empty; the closing commit names the measured arm ratios; if a resident-bytes test lands, it fails when query.rs:514 is changed to `let capacity = 0;` and passes on the shipped arm.
Construction: `RUSTFLAGS='--cfg before_alloc_ab="projection_growth"' cargo bench -p before --bench presize -- --sample-size 10 --measurement-time 1`: the `presize-resident site=projection` lines move relative to the shipped build, and no committed test names the moved quantity; `just test-all` under the same RUSTFLAGS exercises transient peak-heap pins, which say nothing about end-state residency.

### benches-examples-13: The wall cells run under `peak_alloc`, whose `realloc` never grows in place, so the counting allocator's overhead is not "identical across arms"
- Where: crates/before/benches/presize.rs:37-51 (related: crates/before/benches/presize.rs:206-272; crates/before/src/version/skyline/query.rs:513-517; crates/before/src/version/skyline/text.rs:351-354; justfile:807-809)
- Class / severity / confidence: claim / low / high
- Provenance: verified (peak_alloc 0.3.0 src/lib.rs:168-188 from the cargo registry: `realloc` is `System.alloc(new_layout)`, `copy_nonoverlapping`, `System.dealloc(ptr, layout)`, never an in-place extension; presize.rs:50-51 installs it as the global allocator for the whole binary, wall cells included; the growth arms start empty and double, the shipped arms pre-size); executed: no
- Seen by: instrument-correctness [47]; refutation: confirmed, severity lowered (doubling's copy volume is a geometric series bounded by about twice the final buffer, and the record drives no decision yet); history: no rationale (the "identical across arms" claim was written at b28c35ad without a recorded check of the allocator)
- Owner-gated: no

The module doc's claim contradicts the dependency's mechanism: every doubling under `projection_growth` and `display_growth` pays a full copy that the pre-sized arm never pays, so the harness biases the A/B toward the shipped pre-size. The resident column (allocation requests) is unaffected; the wall column is. Moot if benches-examples-12 closes the leg.

Evidence:

        37	//! Wall times are compared *within* a machine and build only; the counting
        38	//! allocator adds a small uniform overhead to every allocation, identical
        39	//! across arms, so arm-to-arm deltas stay honest.
        50	#[global_allocator]
        51	static HEAP: PeakAlloc = PeakAlloc;

    peak_alloc-0.3.0/src/lib.rs:
       176	        let new_ptr = System.alloc(new_layout);
       182	            std::ptr::copy_nonoverlapping(ptr, new_ptr, std::cmp::min(size, new_size));
       184	            System.dealloc(ptr, layout);

Resolution: keep the wall cells on the system allocator: move `resident_report` into its own binary (it is deterministic and needs no criterion), or gate `#[global_allocator]` behind a cfg the wall runs leave off; or install a counting allocator whose `realloc` forwards to `System.realloc` and adjusts the counter by the delta. Restate lines 37-39 to what the harness then does. Acceptance: the binary producing the `presize/*` criterion cells has no copy-realloc counting allocator; `presize-resident` lines still print from a binary that counts.
Construction: run `just bench-alloc-ab presize display_growth presize/display` and `just bench-alloc-ab presize shipped presize/display` as committed and record the ratio; remove lines 50-51 and repeat: the ratio moves toward 1 because the per-doubling copy disappears from the growth arm only.

### benches-examples-14: `outgrow_family` asserts a relative sweep where its doc promises an absolute crossing of the pre-size
- Where: crates/before/benches/presize.rs:92-132 (related: crates/before/src/version/skyline/query.rs:514)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (`ratio` at 121-124 is output bits over `v.encoded_bits() + p.encoded_bits()`, which is the shipped pre-size at query.rs:514 `let capacity = event_bits.len() + id_bits.len();`; the assert at 128-129 is `last >= 4.0 * first` only); executed: no
- Seen by: instrument-correctness [50]; refutation: confirmed; history: no rationale (b28c35ad says the "straddle across the pre-size's growth doublings is asserted on construction" and chose the relative form without stating why)
- Owner-gated: no

Principle 6 (what is the worst artifact that passes): the doc says the output must sweep "from near the pre-size estimate ... to at least 4x past it", so crossing doublings needs `last >= 4.0` with `first` near 1; the assertion also passes for 0.2 -> 0.8, a family that never outgrows the pre-size, the exact flattening the doc says the assertion prevents. Today's family very likely satisfies the absolute claim, so the pin is looser than its sentence rather than failing.

Evidence:

        92	/// The family's design property is asserted on construction: the
        93	/// materialized output must sweep from near the pre-size estimate
        94	/// (operands' summed lengths) to at least 4x past it, so successive cells
        95	/// cross output-buffer growth doublings. Encoded sizes are
       128	    assert!(
       129	        last >= 4.0 * first,

Resolution: assert both clauses, `first <= NEAR_PRESIZE_RATIO && last >= 4.0` with a named constant (1.5, say) and both ratios in the panic message; or reword the doc to the relative sweep if that is the intent. Acceptance: the assertion's inequality matches the doc sentence; a synthetic pair (0.3, 1.2) fails it.
Construction: multiply `OUTGROW_FRAGMENTS` by 64 so the party's bits dominate every term: ratios fall well below 1 across the sweep while `last >= 4 * first` can still hold, and no growth doubling is crossed.

### benches-examples-15: `tools/benchjudge --self-test` runs only at the head of the bench-judge recipes, not in `gate-lints`; tripwire.rs overstates its cadence
- Where: crates/before/benches/tripwire.rs:12-14 (related: justfile:379-382, 403, 842, 849-850, 854, 1003; tools/benchjudge:643-668)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -n self-test justfile`: benchjudge appears at 842 and 854 only, inside `bench-judge` and `bench-judge-tripwire`, which only `all` (1003) runs; `gate-lints` (403) runs every other tool's `--self-test` at 180-322); executed: no
- Seen by: adequacy [14]; refutation: confirmed, severity lowered (every judge invocation runs the self-test first, so no verdict is ever drawn from a softened judge); history: the gate exclusion (feeefb2c; justfile:379-382) is stated for wall-time judging, which a deterministic Python self-test is not; every tool added since joined `gate-lints` with its self-test
- Owner-gated: no

The self-test is pure arithmetic, byte-identical under load, and belongs in the build-free lint tier beside its siblings; today a commit that softens `MAX_WALL_SCALING_EXPONENT` passes `just gate` and fails only when someone runs `just all`. The doc's "cannot soften silently between live demonstrations" describes a cadence the wiring does not deliver: the pin runs at the head of each live demonstration, not between them.

Evidence:

        12	//! judge's leg catches what no deterministic meter can. The same shape is
        13	//! pinned deterministically in the judge's `--self-test`, so the criterion
        14	//! cannot soften silently between live demonstrations.

    justfile:
       403	gate-lints: fmt-check doclint testdoc workflowlint manifestlint digestshare mutants-list readme-check

Resolution: add `./tools/benchjudge --self-test` to `gate-lints` in the position the other tool self-tests occupy; leave the two in-recipe invocations; reword tripwire.rs:12-14 to "pinned in the judge's `--self-test`, which the gate runs". Acceptance: `just gate-lints` invokes `tools/benchjudge --self-test`; with an uncommitted `MAX_WALL_SCALING_EXPONENT = 3.0`, `just gate` fails at the self-test before any build runs.
Construction: set `MAX_WALL_SCALING_EXPONENT = 3.0` in tools/benchjudge (uncommitted): `just gate` passes today, since nothing in gate-lints or gate-streams invokes the judge.

### benches-examples-16: Knuth's MMIX multiplier inlined as an unnamed literal in two instrument files
- Where: crates/before/benches/tripwire.rs:46-48 (related: crates/before/examples/emit_probe.rs:25, 147; crates/before/examples/code_study.rs:338; crates/before/examples/space_consumption.rs:372)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep: tripwire.rs:47 `6_364_136_223_846_793_005`; emit_probe.rs:25 and :147 `6364136223846793005` without separators; code_study.rs:338 and space_consumption.rs:372 name their mixer `GOLDEN`); executed: no
- Seen by: structure-prose [43]; refutation: confirmed; history: no rationale
- Owner-gated: no

Named constants over magic numbers: the sibling examples name their mixing constant; these two files do not, and emit_probe.rs spells it without digit separators. The name is also the one-line explanation (any full-period mixer serves; the value is irrelevant to the exponent).

Evidence:

        46	            acc = acc
        47	                .wrapping_mul(6_364_136_223_846_793_005)
        48	                .wrapping_add((i ^ j) as u64);

    emit_probe.rs:
        25	            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);

Resolution: `const LCG_MULTIPLIER: u64 = 6_364_136_223_846_793_005;` with a one-line doc, used at all three sites (emit_probe.rs only if that file stays; see benches-examples-21). Acceptance: no bare `6364136223846793005` literal under crates/before/benches or crates/before/examples.

### benches-examples-17: The `version/partial_cmp` `equal` row times the clone-identity rung, not a traversal, and its doc says otherwise
- Where: crates/before/benches/version.rs:153-180 (related: crates/before/src/version/skyline/sweep.rs:94-105; crates/before/src/version.rs:91-94; crates/before/src/oracle/version.rs:421-433; crates/before/benches/common/mod.rs:14-18; crates/before/benches/version.rs:222-224, 248-255)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (sweep.rs:100-102 `if a.ptr_eq(&b) { return Some(Ordering::Equal); }` precedes the sweep, with the comment "Equal streams in distinct buffers still take the sweep below"; version.rs:91-94 documents the derived `Clone` as a refcount share; the oracle's `partial_cmp` (oracle/version.rs:423-432) runs two `leq` walks, and `grep -n ptr_eq crates/before/src/oracle/*.rs` finds nothing); executed: no
- Seen by: scaffolding [3], structure-prose [30], instrument-correctness [46]; refutation: confirmed, noting that the committed results README's mechanism was already wrong for its own 2026-06-02 data, which ran under e7324dc2's memcmp rung; history: deliberate-but-expired (accurate for hours on 2026-05-30; two shortcut commits, neither touched the bench; b606f2ad built distinct buffers for the hole group only)
- Owner-gated: no

A bench row is an instrument, and its doc must state what its body measures. `("equal", &base, &base, ...)` hands the same reference twice, so the impl side returns in O(1) from the ptr_eq rung while the oracle walks the tree twice: the row is not like-for-like (common/mod.rs:17-18), a regression in the equal-exhaustion sweep is invisible on it, and both "each exercising a different traversal" and "a version against its own clone" are false, since a clone shares the buffer and hits the same rung. The same file already knows the fix: `bench_hole` builds "equal projections in distinct buffers (a full co-walk, no early exit)" by re-running the construction.

Evidence:

       153	/// `partial_cmp` (the causal order) over the three outcomes the comparison
       154	/// can take, each exercising a different traversal.
       155	///
       156	/// The outcomes: `concurrent` (two independent histories), `ordered`
       157	/// (one strictly precedes the other), and `equal` (a version against its own clone).
       179	            ("equal", &base, &base, &obase, &obase),

    sweep.rs:
        98	    // `order_reflexive` law in `crate::laws`. Equal streams in distinct buffers
        99	    // still take the sweep below.
       100	    if a.ptr_eq(&b) {
       101	        return Some(Ordering::Equal);
       102	    }

Resolution: decode a twin in a distinct buffer for the equal row (`let twin = Version::decode(&base.encode()[..]).unwrap();` and `("equal", &base, &twin, &obase, &obase)`); restate the doc ("equal streams in distinct buffers: the sweep runs to exhaustion"); optionally keep `&base, &base` as an explicitly named `identical` row if the rung's cost is worth tracking. Acceptance: at every `n`, the `before/equal` median scales with `n` like `before/ordered`; the doc names distinct buffers.
Construction: `just bench-quick version partial_cmp` at HEAD: `before/equal` reads near-constant (tens of nanoseconds) across n = 8..32768 while `oracle/equal` grows with n; after the twin change, `before/equal` grows with n.

### benches-examples-18: code_study.rs cites a deleted module and essay as its reason to exist
- Where: crates/before/examples/code_study.rs:5-10 (related: crates/before/examples/code_study.rs:55-57, 316-322; crates/before/src/meter/board/family.rs:1255-1262; crates/before/src/meter/board.rs:294; crates/before/Cargo.toml:119-125; justfile:99, 258, 270; crates/before/AGENTS.md:6 (outside this partition, the same ghost); .agent-notes/2026-07-27-before-constants-frontier/before-constants-frontier.md:277-313)
- Class / severity / confidence: documentation / high / high
- Provenance: verified (`grep -rn 'mod implementation\|before::implementation\|Small values' crates/before` excluding .agent-notes hits only code_study.rs:6; `git log -S'pub mod implementation' -- crates/before/src/lib.rs` shows 67970b75 (added 2026-07-27) and 22cdfbe1 (removed 2026-08-17, "The design-essay implementation module is retired"); justfile:258 and :270 are the only `cargo doc` invocations and pass no `--examples`, so rustdoc's broken-link lint never visits example docs; `just check` (justfile:99, `--all-targets`) compiles the example but nothing runs it; `study_family_versions` is consumed by this example alone and its own doc says "Nothing on the board consumes it"); executed: no
- Seen by: scaffolding [4], adequacy [15], structure-prose [26], instrument-correctness [51]; refutation: confirmed; history: contradicts the hard rule (22cdfbe1 deleted lib.rs's pointer but not this one); the instrument's standing IS recorded, in a note only: constants-frontier §3.1 "Ruling: keep gamma. Revisit trigger: ... re-running the committed study binary at the paper's weights"
- Owner-gated: no (the prose fix; the study's disposition is an open question below)

Root AGENTS.md hard rule: nothing in the codebase refers to code that no longer exists. The module doc's stated purpose links `before::implementation` and quotes a passage ("Small values over large") that exist nowhere in the tree, and lines 55-57 call the constants "the as-run parameters of the crate docs' committed figures" though the crate docs carry no integer-code figures. With the figures gone, the only in-tree answer to "why does this exist" is the example itself (Principle 3); the ruling that keeps it lives in a note the code may not cite. The reconciliation pin at 316-322 is dark: compiled by `just check`, run by nothing.

Evidence:

         5	//! This is the instrument behind the crate docs' integer-code figures (the
         6	//! [`implementation`](before::implementation) essay's "Small values over
         7	//! large" trade): the constants below are the as-run parameters of the
         8	//! measurement quoted there. It changes nothing and asserts its own
         9	//! taxonomy exactly (the reconciliation pin below), so a re-run at other
        10	//! parameters is safe and cheap.
        55	// ─── realistic-simulation parameters ────────────────────────────────────────
        56	// Reduced relative to the `space_consumption` example's defaults; these are
        57	// the as-run parameters of the crate docs' committed figures.

    family.rs:
      1258	/// A measure-only study surface for offline payload analysis — the `code_study`
      1259	/// example re-parses these stored streams into per-class integer histograms to
      1260	/// price candidate integer codes on the adversarial corpus. Nothing on the
      1261	/// board consumes it.

Resolution: rewrite lines 5-10 and 55-57 in the present tense without the ghost: what the study measures (the two emission classes priced closed-form on the realistic and adversarial corpora), that it is the workload-side instrument for the crate's integer-code choice, and that a re-run reproduces the committed histograms. Decide its status explicitly: either a tiny-parameter smoke leg (`RUNS=1`, adversarial corpus only) so the reconciliation pin stays live, or a doc sentence and a justfile note declaring it a manual instrument. If the owner considers the gamma question closed for good, dissolve the example, its `[[example]]` entry, and `study_family_versions` together. Fix crates/before/AGENTS.md:6 ("the public `implementation` module for the design essay") in the same pass. Acceptance: `grep -rn 'before::implementation\|Small values\|committed figures' crates/before` is empty; every link in the file names an item in the tree; either a gate leg runs the walker or the doc states it is manual.
Construction: `grep -rn 'pub mod implementation' crates/before/src` returns nothing; `git show --stat 22cdfbe1 | grep implementation.rs` shows the deletion.

### benches-examples-19: Hand-synchronized copies with parity asserted in prose: code_study.rs duplicates space_consumption.rs's simulation; perf_probe.rs duplicates benches/common
- Where: crates/before/examples/code_study.rs:335-340 (related: crates/before/examples/code_study.rs:39-40, 337-426; crates/before/examples/space_consumption.rs:266-374; crates/before/examples/perf_probe.rs:18-141, 220-221; crates/before/benches/common/mod.rs:36-239; crates/before/examples/fuzz_seeds.rs:13-14; crates/before/tests/bench_judge_roster.rs:15-17)
- Class / severity / confidence: modularity / medium / high
- Provenance: verified (side-by-side read: code_study.rs:337-397 equals space_consumption.rs:269-374 body for body with docs stripped and `Scenario` flattened to `tag: u64`; perf_probe.rs:18-141 equals benches/common/mod.rs:36-239 minus the two `plan` asserts and with `take_group` inlined; the `#[path]` sharing idiom is live at fuzz_seeds.rs:13-14 and bench_judge_roster.rs:15-17); executed: no. The resolution of `pub mod sidecar;` inside a `#[path]`-included file named mod.rs was assessed against rustc's module rules, not compiled.
- Seen by: scaffolding [5], adequacy [16], structure-prose [27 part], [34], instrument-correctness [52]; refutation: confirmed; history: no rationale (8da8f815 ported the study with the copy in it; 7d101fe7 wrote the probe's copy; both postdate the idiom, bb6ea8b7)
- Owner-gated: no

"No hand-maintained restatements of enumerable facts": "same step functions, same per-run seeding" (code_study.rs:39-40) and "Same salts as benches/clock.rs" (perf_probe.rs:220-221) are claims the code can falsify without touching the prose; a change to the paper's step model or to the bench corpus silently changes what the study's REALISTIC corpus and the probe's "same corpus" mean. The `Scenario` type is lost to a bare `u64` on the way.

Evidence:

       335	// ─── the realistic simulation (space_consumption replicated, reduced) ──────
       336	
       337	fn seed_for(tag: u64, n: usize, run: u64) -> u64 {
       338	    const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;
       339	    GOLDEN ^ (tag << 56) ^ ((n as u64) << 32) ^ run
       340	}
        39	//! - REALISTIC: the `space_consumption` simulation (same step functions,
        40	//!   same per-run seeding), at the reduced parameters in the constants

    perf_probe.rs:
       220	    // Same salts as benches/clock.rs: tick uses salt 1 (1 group), join salt 3
       221	    // (2 groups).

Resolution: move `Scenario`, `seed_for`, `checkpoints`, `build_population`, `step_data`, `step_process` into `examples/support/simulation.rs` and include it from both examples with `#[path]`, so code_study's `tag: u64` becomes `Scenario`; replace perf_probe.rs:18-141 with `#[path = "../benches/common/mod.rs"] mod common;` and `use common::{SEED, plan, impl_clocks, hole_pair, oracle_clocks}` (name the salts as constants in common if the correspondence with benches/clock.rs matters, and use them on both sides). The code_study half is moot if benches-examples-18 dissolves the example. Acceptance: one definition of each simulation and corpus function in the tree; `just check` green; the two parity comments disappear or read as mechanical ("shares `common::plan`").

### benches-examples-20: code_study.rs dispatches on a string label and keeps parallel per-code arrays
- Where: crates/before/examples/code_study.rs:513-525 (related: crates/before/examples/code_study.rs:133-139, 263)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read: the best-Rice loop iterates `[("FIRST", &c.first), ("DELTA", &c.delta)]` and re-derives the denominator with `if cls == "FIRST"`; `Corpus` keeps `code_bytes: Vec<u128>` beside `code_diverged: Vec<bool>` "Indexed as CODES" while `walk` already uses `Vec<Option<u128>>` at 263); executed: no
- Seen by: structure-prose [45]; refutation: confirmed; history: ported as written (8da8f815, "Output byte-identical before and after the lint-shape fixes")
- Owner-gated: no

Types-first: the loop owns the label, so the denominator belongs in the tuple; two parallel vectors keyed by roster position are the shape `Vec<Option<u128>>` (None = diverged) already expresses at line 263. Moot if the example dissolves.

Evidence:

       513	    for (cls, h) in [("FIRST", &c.first), ("DELTA", &c.delta)] {
       521	                    / (if cls == "FIRST" {
       522	                        gamma_first
       523	                    } else {
       524	                        gamma_delta
       525	                    }) as f64

Resolution: `[("FIRST", &c.first, gamma_first), ("DELTA", &c.delta, gamma_delta)]`; merge `code_bytes` and `code_diverged` into one `Vec<Option<u128>>`. Acceptance: no string comparison on `cls`; one per-code vector in `Corpus`.

### benches-examples-21: emit_probe.rs exercises no code in the tree, is the sole reason `bitvec` is a dev-dependency, and its one assert names a path that does not exist
- Where: crates/before/examples/emit_probe.rs:50-52 (related: crates/before/examples/emit_probe.rs:1-11, 32-48, 71; crates/before/Cargo.toml:33-36; Cargo.toml:22; .agent-notes/2026-08-04-perf-probe/README.md:15-17; .agent-notes/2026-08-04-perf-probe/probe-report.md:63-69)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (the file imports only `bitvec` and std; `grep -rln bitvec --include='*.rs' --include='*.toml'` hits the workspace Cargo.toml, crates/before/Cargo.toml, and this example; `git log --follow` shows 7d101fe7 (added 2026-08-04) then 83e61b4d (2026-08-18, "bitvec leaves the production dependency graph (it stays a dev-dependency as emit_probe's external baseline)"); the only `spill` in the file is the local at line 75; the numbers it produced are at probe-report.md:66-69); executed: no
- Seen by: scaffolding [1 part], adequacy [17], structure-prose [28], instrument-correctness [53 part]; refutation: confirmed; history: deliberate-but-expired (the keep at 83e61b4d and the note's "can still be run" name no consumer of the comparison going forward)
- Owner-gated: yes (a recorded keep decision)

Principle 3: machinery outlives the constraint that justified it. The probe compares `bitvec` against a standalone `WordWriter` "reproduced standalone so the comparison needs no crate internals"; neither arm is `before`, so it cannot observe the shipped `PackedBuilder`/`BitsBuf` regress; its timing loop hand-rolls what criterion provides; the decision it priced landed, and its record has a home. Line 71's message names a "spill path" this file never had.

Evidence:

        50	/// A minimal word-buffered MSB-first bit writer: the staging discipline
        51	/// the crate's own `PackedBuilder` ships, reproduced standalone so the
        52	/// comparison needs no crate internals.
        71	        debug_assert!(len <= 63, "codes wider than 63 bits take the spill path");

    Cargo.toml (crates/before):
        34	# The emit_probe example's external comparison baseline: the production
        35	# build buffer is the crate-owned BitsBuf, and nothing shipped links bitvec.
        36	bitvec = { workspace = true }

Resolution: delete the example and the `bitvec` dev-dependency (and the workspace entry, which nothing else uses); the note is the record. If a standing primitive-cost comparison is wanted, write it as a criterion `[[bench]]` that times the crate's actual `BitsBuf`/`PackedBuilder` against `bitvec`, so a regression is a criterion-tracked number, and fix the assert message to the guard it enforces (`(1u64 << spill) - 1` overflows at `len == 64`). Acceptance: `grep -rn bitvec crates/ Cargo.toml` returns nothing (or the probe times `before`'s builder); `just check` and `just bench-build` clean; the perf-probe README updated to say the probe retired and where its numbers live.

### benches-examples-22: perf_probe.rs frames itself as a one-off for a past investigation and documents a feature flag that is always on
- Where: crates/before/examples/perf_probe.rs:1-9 (related: crates/before/examples/perf_probe.rs:112, 318; crates/before/Cargo.toml:49; crates/before/benches/common/mod.rs:30; .agent-notes/2026-08-04-perf-probe/README.md:15-17)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (b606f2ad's message assigns the standing role: "the probe remains the sampling profiler's op-isolation loop"; Cargo.toml:49 `before = { workspace = true, features = ["oracle", "meter"] }` is a self dev-dependency, so `oracle` is enabled for every dev target, which is why benches/common/mod.rs:30 imports `before::oracle` unconditionally under the flagless `just bench-build`); executed: no
- Seen by: scaffolding [1 part], structure-prose [27 part], [39], instrument-correctness [53 part]; refutation: confirmed; history: the file's existence is deliberate-and-holds (b606f2ad; note README:15-17); line 1's framing predates that role and contradicts it; the `--features oracle` note was moot from the start (6e69b427: "a self dev-dependency turns it on so `cargo bench` builds with the oracle exposed without needing `--features oracle`")
- Owner-gated: no (whether criterion's `--profile-time` should replace the file is an open question below)

Principle 5: "One-off profiling harness for the perf-probe investigation" is history at a declaration site, and b606f2ad gave the file a present-tense role it does not state. The `#[cfg(feature = "oracle")]` gates at 112 and 318 and the usage line's `--features oracle` describe a switch cargo's feature unification has already thrown.

Evidence:

         1	//! One-off profiling harness for the perf-probe investigation.
         2	//!
         3	//! Rebuilds the bench suite's corpus shapes through the public API,
         4	//! then spins each hot operation in its own `#[inline(never)]` loop so
         5	//! a sampling profiler attributes cycles per operation. Run with
         6	//! `--features oracle` to also print the oracle's timings for the same
         7	//! plans (ratio anchor only).
         8	//!
         9	//! Usage: cargo run -p before --profile bench --example perf_probe --features oracle [n]

    Cargo.toml (crates/before):
        49	before = { workspace = true, features = ["oracle", "meter"] }

Resolution: rewrite the module doc as what the file is ("Profiler harness: one `#[inline(never)]` loop per hot operation over the bench corpus, so a sampling profiler attributes cycles per operation; the wall-time record is the criterion suite"); drop the cfg gates and the `--features oracle` note. Acceptance: no "one-off" or "investigation" framing in the file; `cargo check -p before --example perf_probe` with no feature flags builds and the oracle loops are unconditional.

### benches-examples-23: An unescaped `|` inside a code span splits the API-mapping table row
- Where: crates/before/examples/space_consumption.rs:31-31
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read; GFM and pulldown-cmark split cells on every unescaped pipe, code spans included); executed: no
- Seen by: instrument-correctness [57]; refutation: confirmed (impact limited: nothing in the gate renders example docs); history: fe3d5f44-original
- Owner-gated: no

The table is the example's contract map to the paper; the `peek` row renders with an extra cell in any GFM renderer.

Evidence:

        31	//! | `peek` then `join`     | snapshot [`Clock::version`], then `clock |= v` |

Resolution: escape it: `clock \|= v`. Acceptance: the row renders as three cells under `cargo doc --examples` or any GFM viewer.

### benches-examples-24: space_consumption accepts `--data-iters 0` and `--runs 0` under a "non-negative integer" message, then panics or writes NaN rows
- Where: crates/before/examples/space_consumption.rs:199-201 (related: crates/before/examples/space_consumption.rs:343-348, 352-367, 421-427)
- Class / severity / confidence: correctness / nit / high
- Provenance: verified by tracing the code: `parse_u64` accepts 0; `checkpoints(0)` computes `k_max` from `(0f64).log10()` = -inf, which saturates to `i64::MIN`, so the loop is empty and `points.push(max)` yields `[0]`; `simulate` iterates `1..=0` (empty) so `sizes` is empty; the aggregation indexes `run[ci]` at ci = 0 and panics; `--runs 0` makes `per_run` empty and `mean_std(&[])` divides by zero into NaN rows; executed: no
- Seen by: instrument-correctness [55]; refutation: confirmed; history: fe3d5f44-original
- Owner-gated: no

Principle 1 at harness weight: the parser owns the error message and should reject what the program cannot run, instead of an index panic downstream or a silently NaN CSV.

Evidence:

       200	                let bits: Vec<f64> = per_run.iter().map(|run| run[ci].0).collect();
       424	            "{flag} expects a non-negative integer, got {value:?}"

Resolution: require `runs >= 1`, `data_iters >= 1`, and `process_iters >= 1` in `parse_args`; change the message to "positive integer". Acceptance: `cargo run --example space_consumption -- --data-iters 0` exits 2 with the flag error; `--runs 0` likewise.
Construction: `cargo run --release --example space_consumption -- --runs 1 --data-iters 0 --process-iters 10 --entities 4` panics with an index out of bounds at line 200.

### benches-examples-25: results/benchmarks is a 2026-06-02 record of benches that no longer exist, with a wrong mechanism for its largest quoted win and a broken table row
- Where: crates/before/results/benchmarks/README.md:72-73 (related: crates/before/results/benchmarks/README.md:42-46; crates/before/results/benchmarks/speedup.md:10-16; crates/before/scripts/plot_benchmarks.py:117-120, 136; crates/before/benches/party.rs:120-127; .agent-notes/2026-08-04-perf-probe/probe-report.md:151-160)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (`git log -- crates/before/results/benchmarks`: figures from 786fd8e4 (2026-06-02), later commits (58a37d80, 35a09c5b) touched prose only; 7139904b the same day removed Party's `PartialOrd` and the party partial_cmp bench group; plot_benchmarks.py:117-120 still names `party/partial_cmp` `before/ancestor`/`before/equal`, absent from benches/party.rs's group (fork, join, is_disjoint, codec); at 786fd8e4 `causal_cmp` (version/compare.rs:172) returned Equal via `trivially_eq`, a memcmp, so README:43-46's "single-pass compare" never described the equal row; speedup.md:13 splits on the unescaped `|` in "merge  ( | , least-upper-bound)", which comes from plot_benchmarks.py:136; nothing outside the directory and the script references it); executed: no
- Seen by: scaffolding [9]; refutation: confirmed and reframed (the mechanism was wrong at origin, not because of the later ptr_eq rung) plus the new speedup.md:13 row; history: no rationale (stale within six hours of commit; two prose edits since without re-measuring)
- Owner-gated: yes (excise vs regenerate)

Principle 5: dated measurement reports are not exempt; when the code they cite is gone, re-denominate or excise. The record predates the skyline coding, the marker padding, and the perf campaign that moved version join from 2.3x slower to 0.78x (probe-report.md:155); its plot script names dead bench IDs; its explanation of the ~31x equal-case win is the wrong mechanism; a reader finding this directory takes away a performance picture two codings old.

Evidence:

        72	All curves in the committed figures were collected in a single run on
        73	2026-06-02.
        42	- **Big wins** come from operations where the packed form prunes whole subtrees
        43	  cheaply or avoids redundant traversal: `clock/fork` (~15×), `party/fork`
        44	  (~6×), and the `partial_cmp` *equal* case (`version` ~31×, `party` ~12×),
        45	  where the single-pass compare beats a two-pass containment formulation that
        46	  would walk the whole tree twice.

    speedup.md:
        13	| Version | merge  ( | , least-upper-bound) | 32768 | 2.12 ms | 2.10 ms | 1.0× |

    plot_benchmarks.py:
       117	            cmp_panel("party/partial_cmp", "partial_cmp: ancestor",
       118	                      "before/ancestor", "oracle/ancestor"),

Resolution: excise crates/before/results/benchmarks and scripts/plot_benchmarks.py, or regenerate under the live suite (drop the party partial_cmp panels, add `version/hole`, escape the pipe in the merge title, re-run `cargo bench -p before` on a quiet machine, date the README by commit rather than calendar). Drop the mechanism sentence at README:43-46 either way (see benches-examples-17). Acceptance: every group/function the plot script names exists in benches/*.rs; the speedup table matches the live bench roster; the README's mechanism claims match sweep.rs; no table row splits on a pipe.
Construction: `cargo bench -p before --bench party -- --sample-size 10 --measurement-time 1` then `python3 crates/before/scripts/plot_benchmarks.py`: the party partial_cmp panels render empty (`series()` returns None for absent groups) and the version join speedup inverts relative to the committed table.

### benches-examples-26: The space-consumption results README lists six CSV columns for an eight-column file; the figure of record predates the marker-padding change to `encode().len()`
- Where: crates/before/results/space_consumption/README.md:8-9 (related: crates/before/examples/space_consumption.rs:50, 168; crates/before/scripts/plot_space_consumption.py:52-53; crates/before/src/lib.rs:329; crates/before/build.rs:93-99; crates/before/src/version.rs:1131; crates/before/src/clock.rs:827)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`head -1 space.csv` is `scenario,entities,iteration,mean_bits,std_bits,mean_bytes,std_bytes,runs`; the example writes that header at :50 and :168; the data commit 4e7f2d3d is 2026-07-27; d800957e (2026-08-06) changed the size law to `encode().len() == (encoded_bits() + 1).div_ceil(8)` and touched nothing under results/; the plot reads `mean_bytes` by name; build.rs:93-99 inlines the SVG into lib.rs:329); executed: no
- Seen by: scaffolding [10], adequacy [25]; refutation: confirmed; history: no rationale (stale since dc88e755, five hours after the README was written; survived two later README edits)
- Owner-gated: no for the column list; the re-measure is the owner's call (long-running at paper parameters)

Prose contradicting code, and a public crate-docs figure whose byte column describes a wire one revision behind (at most +1 B per stamp, about +1/8 B on average, visible only on the 4-replica curves). Measurements bind to their run.

Evidence:

         8	- `space.csv` — raw measurements (100 runs, paper parameters). Columns:
         9	  `scenario,entities,iteration,mean_bytes,std_bytes,runs`.

Resolution: fix the column list now (or point at the example's `# Output` section as the column reference). Re-run `cargo run --release --example space_consumption` and the plot at the next convenient point, or state in the results README the commit the data was collected at. Acceptance: the README's column list equals `head -1 space.csv`; the README names the data's commit; after a re-run, the 4-replica final byte means in the README table match the new CSV.

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
