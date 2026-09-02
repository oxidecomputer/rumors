# Partition fuelscape-pipeline: The fuelscape population atlas: plan, count, enumerate, select, sample, families, operation table

## Partition summary

`before-fuelscape` is a detached dev-tool workspace that renders, for each public operation of `before`, a log-log heatmap of deterministic wasm fuel against exact packed input size, sampled uniformly from every size's whole canonical input space. This partition is the measurement core. `count.rs` builds exact big-integer counting tables over the two codec grammars (the version skyline coding under the sibling rule, the party id coding); `sample.rs` turns those tables into exact-uniform samplers by counting-guided generation, restoring the version grammar's nonnegative-height rule by whole-sample rejection; `enumerate.rs` lists every member of a small exact-size space straight from the grammar so the tests have ground truth the samplers cannot supply about themselves; `plan.rs` draws each cell's inputs from a coordinate-seeded RNG (arity, then composition split, then members), runs one measured kernel per sample in a fresh guest, and appends the overlay points; `families.rs` ramps committed meter shapes into overlay inputs per operand signature; `ops.rs` is the roster of 104 `OpSpec` rows plus the 72-row `EXEMPTIONS` table that, with the panels' `covers` lists, tiles `before::surface`; `select.rs` implements the runner's name filters. The tests (`plan/tests.rs`, `count/tests.rs`, `select/tests.rs`, `sample/tests.rs`, `ops/tests.rs`) are the crate's committed checks: sampler adequacy, determinism, tiling parity, and the runner's empty-run guard. I read all fourteen partition files whole with line numbers (5,631 lines) and, for the anchors the findings rest on, the smoke test, the fuzzfit guest's kernels, `before`'s version/ranked/conjunction/party compare code, the board's `WORST_RANKINGS` and `BOARD_NOT_APPLICABLE`, `surface.rs`, `compact.rs`, `render.rs`, the spanbands binary, the entropy agent note, rayon 1.12.0's range sources, and the git history the history pass cited.

The instrument's core is in good shape and I could check it: the choice weights in both samplers partition exactly the pairs the recurrences count, the sibling-rule exclusion is transcribed the same way in the table (`leaf(j-3)`), the enumeration, and the sampler, and the adequacy pins form a genuine triangle of independent oracles (table versus enumeration to 24 bits, grammar versus the shipping decoders as set equality over every byte string to 2 bytes, decoder census to 23 bits, chi-square uniformity at a fixed seed, proptest round trips to 48 bytes). The run is a pure function of the plan and a replay test pins that, collection order included. `select.rs` closes the silent-empty-run hole per filter. The tiling test binds panels plus exemptions to the surface roster in both directions with a contradiction check, and every exemption reasons on mechanism identity or a missing size axis. Dependency rationales in `Cargo.toml` are exemplary.

The dominant issues are two doc claims the tree does not back and one instrument mislabel. `COMBINE_ARITY_CAP` says the host and guest caps are "kept equal by the pipeline smoke test's combine case", but the smoke plan runs at 8 bytes and no test compares the two constants. `lib.rs` says the overlay marks "the committed adversarial families" and shows "the adversarial frontier", but the overlay is a hand list of 18 of the registry's 68 shapes keyed by operand signature, and the board's own per-operation worst-case pin names families that never appear in it. On every `[Version, Version]` row the overlay adds two self-pairs, and on the eight rows whose measured kernel opens with `codec::canonical_eq` those points measure a byte compare plus a refcount clone under an adversarial label. Beneath those: duplicated scaffolding (two count tables and two samplers sharing every method but the recurrence; a hand-written parallel/sequential branch with a reference twin and a 2048-entry pin that rayon's `with_min_len` replaces; the member-draw closure copied seven times in `draw_inputs`; the chi-square statistic written three times), a second exemption roster restating the board's for 51 shared surface rows, a few pins of the weaker genre (reachability where uniformity is claimed; a sign-only fold-arity criterion), dead accessors, and a vocabulary sweep (`mint` at eight sites, `honest`, `real`, em-dashes in line comments).

## Findings

### fuelscape-pipeline-1: The overlay is a curated, signature-keyed subset presented as the committed adversarial frontier
- Where: crates/before-fuelscape/src/lib.rs:13-14 (related: crates/before-fuelscape/src/lib.rs:36-39; crates/before-fuelscape/src/families.rs:87-113, 167-180; crates/before/src/meter/board/worst.rs:401, 414, 417, 435, 456; crates/before/src/meter/registry.rs:97; crates/before/src/testing/validation_index.rs:135-136; crates/before-fuelscape/src/compact.rs:146-148; crates/before-fuelscape/src/render.rs:501-509)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (grep: 18 distinct `Shape::` variants used in families.rs against 68 variants of `pub enum Shape` at registry.rs:97; `grep -c` for FreezePosition, PromotionRearm, AscendCliff, DominatedUndercut, LoneFreeze in families.rs returns 0 while all five exist in the registry; read worst.rs:401-470); executed: no
- Seen by: scaffolding [0], adequacy [14]; refutation: reframed (the labels cannot all be `FamilyId::name()` because crosses and fixed-knob ramps have no `FamilyId`; the board's `designed()` applicability is per `OpGroup`, not per op; `WORST_RANKINGS` is `pub(super)`; severity argued down to low because the atlas enforces nothing); history: signature keying is the recorded design with its rationale inline at families.rs:87-97; the lib.rs wording is unchanged since the first overlay commit and predates the registry (68 shapes) and the board's per-op pin
- Owner-gated: yes: binding the overlay to the board's per-operation pin needs new public surface in `before::meter` (`WORST_RANKINGS` is `pub(super)` and its op names differ from atlas rows)

The crate doc promises that the marked points are the committed adversarial families and that one canvas shows the adversarial frontier; the mechanism is a hand list of eighteen registry shapes chosen per operand signature, with no parity pin against the registry or the board, and for many panels the board's tamper-evident worst-case pin names families the overlay never draws (version_tick: ascend-cliff, dominated-undercut, mirror-narrow absent; version_rank: freeze-pos absent; version_display: mirror-narrow, mirror-wide, dominated-undercut absent). This breaches Principle 8 at the instrument level (the doc reports a frontier the artifact does not derive from the thing that pins the frontier) and the roster idiom the same crate states at ops.rs:22, "membership is enforced, never remembered". I keep medium rather than the refutation's low because the claim is repeated in `before`'s validation index and the atlas exists to be read by a human auditing exactly this; a frontier that is not the frontier misleads the one reader the instrument has.

Evidence:

        13	//! committed adversarial families overlaid as marked points on the same
        14	//! axes. One canvas shows the bulk cloud and the adversarial frontier.

    (families.rs)
        98	pub fn overlay_inputs(op: &OpSpec, max_bytes: usize) -> Vec<FamilyInput> {
        99	    let (operands, distinct) = match op.inputs {
       100	        Inputs::Packed(operands) => (operands, false),
       ...
       167	        [Operand::Version, Operand::Party] => {
       168	            out.extend(ramp("dense × scattered_id", max_bytes, |t| {
       ...
       174	            out.extend(ramp("hugeleaf × id_spine", max_bytes, |t| {

    (crates/before/src/meter/board/worst.rs)
       414	    ("default", "version_tick", ["ascend-cliff", "dominated-undercut", "hugeleaf", "mirror-narrow"]),

Resolution: Either (a) derive the overlay from the board: expose a per-operation family roster from `before::meter` (the `WORST_RANKINGS` rows, or a `FamilyId`-per-op table), add an atlas-op to board-op map tested total, ramp exactly those families per row (keeping the fold-cure families for the slice rows), and pin it with a test that every board-pinned worst family for an atlas-covered operation has a point on that operation's panel at a span large enough to admit it; or (b) keep the curated list and re-state lib.rs:13-14 and :36-39, families.rs:5-9, and validation_index.rs:135-136 to say the overlay marks a fixed signature-keyed subset of the committed shapes, with a committed test that names each board-pinned worst family missing from its atlas panel so the gap is visible in the gate. Acceptance: under (a), adding a `Shape` the board prices for a version operation fails `just fuelscape-test` until an overlay or exemption names it; under (b), the atlas docs no longer use "the committed adversarial families" or "the adversarial frontier" for the overlay and the gap list is a committed test.
Construction: Take the board row at worst.rs:414. Call `overlay_inputs` on the `version_tick` row (`Inputs::Packed(&[Operand::Version, Operand::Party])`, ops.rs:407-420) at `max_bytes` 256 and list the `FamilyInput.family` labels: only `dense × scattered_id` and `hugeleaf × id_spine` appear, so three of the four board-pinned worst families for this operation have no point on its panel while lib.rs:13-14 tells the reader the frontier is drawn.

### fuelscape-pipeline-2: Vocabulary and register sweep: "mint" for constructing values, "honest" and "real" for properties, em-dashes in line comments
- Where: crates/before-fuelscape/src/lib.rs:19 (related: plan.rs:19, 168; ops.rs:105, 185, 800, 2448, 2507 ("mint"); ops.rs:2168, sample/tests.rs:99 ("honest"); count.rs:32, enumerate.rs:7, sample.rs:26, count/tests.rs:114, 129, families.rs:41, sample/tests.rs:73, 88, 226, 247 ("real" for the shipping decoders); sample.rs:298, 438, ops.rs:856, 1745, 2088 (em-dashes in `//` comments))
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep over the fourteen partition files for `mint`, `\bhonest\b`, `\breal\b`, and `^\s*//[^/!].*—`); executed: no
- Seen by: scaffolding [11], adequacy [21], structure-prose [32], [38]; refutation: confirmed; history: the rules live in the owner's writing-style doctrine (`~/.claude/writing-style.md`:149-152, 170-175, 326-330), tracked from 2026-08-19, after most of these sites were written (2026-07-28..31), so this is sweep work rather than an authoring-time breach
- Owner-gated: no

The owner's vocabulary rule bans "mint" for constructing a value outright and asks that "real"/"honest" be replaced by the property that holds; the register rule reserves em-dashes for rendered prose and gives `//` comments the spaced double hyphen. Eight "mint" sites, two "honest", about ten "real" where count/tests.rs:172 already has the precise term ("the shipping decoder"), and five em-dashes in line comments. Severity low rather than nit because these are stated rules, not taste, and the sweep is one batch.

Evidence:

    (lib.rs)
        18	//! pipeline smoke test, nothing else: no fuel threshold, percentile gate,
        19	//! or band is ever minted from atlas data — the envelope suite and the

    (ops.rs)
      2448	        "O(1) hole mint over the atom's bound: no comparison, no walk",
      2168	/// forms have no honest size axis to plot, and delegating wrappers are

    (sample/tests.rs)
        99	/// `2(k - 1)`, so six standard deviations above the mean rejects honest

    (sample.rs)
       298	                // right subtree, its remainder the leaf mantissa — but the

Resolution: "derived" (lib.rs:19); "split in the guest" / "guest-split" (plan.rs:19, 168; ops.rs:105, 800); "builds" (ops.rs:185); "O(1) hole construction" (ops.rs:2448); "produce" (ops.rs:2507); "no size axis" (ops.rs:2168); "rejects a uniform draw" (sample/tests.rs:99); "the shipping decoder(s)" for "real" and rename `real` to `decoded` in sample/tests.rs:73, 88; swap the five line-comment em-dashes for ` -- ` or a colon. Acceptance: `grep -n -i -w 'mint\|minted\|mints\|honest' crates/before-fuelscape/src` returns nothing; `grep -n -E '^\s*//[^/!].*—'` over the partition returns nothing.

### fuelscape-pipeline-3: History and roadmap in module prose: a split rule "unchanged", a "long-term fix" not built
- Where: crates/before-fuelscape/src/plan.rs:187-188 (related: crates/before-fuelscape/src/count.rs:37-39)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both sites; `git log -1 eb6b35f42` carries "the binary rule is the k = 2 case, stream-identical", the change the sentence records); executed: no
- Seen by: scaffolding [12], adequacy [22], structure-prose [33], instrument-correctness [53]; refutation: confirmed; history: "unchanged" was written by the commit that generalized the split (eb6b35f42) and compares to the implementation that commit replaced; the persistence paragraph dates from the parallelization commit (969cf3ae1) and nothing since tracked or built it
- Owner-gated: no

"unchanged" refers to a prior revision the tree no longer shows, which the root AGENTS.md hard rule forbids ("Nothing in the codebase refers to code that no longer exists"); the count module's closing paragraph is a plan, not a statement of what the module is (Principle 5). Both sentences stand without the offending clause.

Evidence:

    (plan.rs)
       187	/// For one part this draws nothing; for two it is a single
       188	/// `gen_range(1..total)` — the binary split rule, unchanged.

    (count.rs)
        37	//! Not built here: table persistence keyed by (grammar fingerprint, span)
        38	//! is the long-term fix for routine large-span surveys — a survey would
        39	//! load its tables instead of reconvolving them.

Resolution: Drop ", unchanged" at plan.rs:188; delete count.rs:37-39 and file the persistence idea in the shadow tracker if it is still wanted (a one-line negative-space statement, "tables are rebuilt per run", may stay). Acceptance: `grep -n 'unchanged\|long-term fix\|Not built here'` over the two files returns nothing.

### fuelscape-pipeline-4: draw_inputs copies the member-draw closure seven times; the two slice arms differ by one expression
- Where: crates/before-fuelscape/src/plan.rs:274-311 (related: crates/before-fuelscape/src/plan.rs:213-243, 312-352)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read); executed: no
- Seen by: scaffolding [8], structure-prose [26]; refutation: confirmed; history: the arms accreted by copying across three commits (eb6b35f42/c459b1d0, 98c63b26, 46eb64f9) with no discussion of shape
- Owner-gated: no

The version draw with `expect("every byte size down to 1 has canonical versions")` and the `rejected` accumulation appears at 226-231, 283-288, 302-307, and 330-335; the party draw at 234-238, 324-328, and 346-350; the `VersionSlice` and `VersionSliceCapped` arms are identical except for `size.min(cap as usize)`. The draw-stream order (arity, then split, then members) is the load-bearing fact behind the determinism test, and it is easiest to audit stated once. Legibility standard: finished code is obviously correct.

Evidence:

       293	        Inputs::VersionSliceCapped(cap) => {
       294	            // As `VersionSlice`, the arity draw capped at the roster
       295	            // row's declared bound.
       296	            let arity = draw_arity(size.min(cap as usize), rng);
       297	            let sizes = split_budget(size, arity, rng);
       298	            let mut rejected = 0;
       299	            let inputs = sizes
       300	                .into_iter()
       301	                .map(|n| {
       302	                    let draw = samplers
       303	                        .version
       304	                        .sample_bytes(n, rng)
       305	                        .expect("every byte size down to 1 has canonical versions");
       306	                    rejected += draw.rejected;
       307	                    draw.bytes
       308	                })
       309	                .collect();
       310	            (inputs, arity, rejected)
       311	        }

Resolution: Two helpers, `draw_version(samplers, n, rng, &mut rejected) -> Vec<u8>` and `draw_party(samplers, n, rng) -> Vec<u8>`, owning the two `expect` proofs; `draw_packed` and every arm call them. Match `Inputs::VersionSlice | Inputs::VersionSliceCapped(_)` in one arm with `let cap = match op.inputs { Inputs::VersionSliceCapped(c) => size.min(c as usize), _ => size }`, or give `VersionSlice` an `Option<u32>` cap. Acceptance: `run_op_is_deterministic_and_ordered` and the smoke test pass unchanged (the draw order is preserved); each `expect` string appears once in plan.rs.

### fuelscape-pipeline-5: The determinism test says its op list walks every input space; VersionSliceCapped has no replay pin
- Where: crates/before-fuelscape/src/plan/tests.rs:14-37 (related: crates/before-fuelscape/src/ops.rs:79, 393-406; crates/before-fuelscape/src/plan.rs:293-311)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (mapped the seven names to their `Inputs` variants in ops.rs: Packed (version_rank, party_covers, clock_sync), PackedDistinct (party_without), VersionSlice (version_join_all), ClockSlice (clock_join_all), PartyShares (party_join_all); `VersionSliceCapped` is used only by `shape_combine` at ops.rs:395 and is absent); executed: no
- Seen by: adequacy [17], structure-prose [28], instrument-correctness [48]; refutation: confirmed; history: deliberate-but-expired: the claim was true when written and the commit that added the seventh variant (46eb64f9) touched no test file
- Owner-gated: no

Every test's doc comment must state its invariant accurately; this one over-claims by one input space, and the list is a hand-maintained enumeration the `Inputs` enum can outgrow without touching the test. The capped row is also the only one whose measure dispatches through a per-arity table in the guest, which the replay would exercise for free.

Evidence:

        14	/// re-derives a cell's RNG from execution order fails it. The op list
        15	/// walks every input space: unary and binary packed draws (both
        16	/// samplers, the split rule, the version rejection path), the slice
        17	/// arity-and-composition draw, the distinct-pair rejection, the
        18	/// three-way split with in-guest fork preparation, and both
        19	/// variable-arity fold draws (the party-plus-version-slice clock fold
        20	/// and the guest-split party shares).
        ...
        29	    for name in [
        30	        "version_rank",
        31	        "party_covers",
        32	        "version_join_all",
        33	        "party_without",
        34	        "clock_sync",
        35	        "clock_join_all",
        36	        "party_join_all",
        37	    ] {

Resolution: Add `"shape_combine"` to the list and name the capped draw in the doc; better, derive the list from `ROSTER` by picking the first row per `Inputs` variant through a `match` with no wildcard arm, so a new variant is a compile error and the doc can say "one row per `Inputs` variant". Acceptance: every `Inputs` variant has a row in the replay; adding a variant without one fails to compile (derived) or the doc no longer says "every".

### fuelscape-pipeline-6: Arity-draw pins: the reachability test is subsumed by the chi-square pin, and the known-bad draw it cites exists only in prose
- Where: crates/before-fuelscape/src/plan/tests.rs:153-178 (related: crates/before-fuelscape/src/plan/tests.rs:179-202)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (arithmetic: a skipped arity at 2000 draws over 9 categories contributes (0 - 222.2)^2 / 222.2 = 222.2 to the statistic, above the 32.0 threshold at line 197); executed: no
- Seen by: instrument-correctness [49], [50]; refutation: confirmed; history: the uniformity pin's commit (8409fe55) recorded that the reachability pin stays green under the mutant the new pin catches, and kept the older pin without saying why; the 723.2 figure was re-measured out of tree at adoption (the track's message carried ~790)
- Owner-gated: no

Principle 3: an instrument earns its place by naming a failure the others miss, and the uniformity pin's own doc (171-175) is the argument that it strictly dominates the reachability test. Principle 6 and doctrine: "every criterion needs a committed demonstration that a known-bad mechanism fails it"; here the mechanism (min of two draws) and its statistic (723.2) live only in a comment, a hand-maintained number that asserts nothing.

Evidence:

       153	#[test]
       154	fn arity_draw_reaches_every_count() {
       155	    let mut rng = cell_rng(0xc0de, "arity-reach", 9, 0);
       156	    let mut seen = std::collections::BTreeSet::new();
       157	    for _ in 0..2_000 {
       158	        seen.insert(draw_arity(9, &mut rng));
       159	    }
       ...
       172	/// biased-but-total draw (e.g. the min of two `gen_range` draws,
       173	/// `P(1) = 17/81` vs the uniform `1/9` at cap 9, chi-square 723.2 at
       174	/// the committed seed over 2000 draws) reaches

Resolution: Delete `arity_draw_reaches_every_count`, folding its boundary-arity rationale (3, 4, 6 at lines 150-152) into the uniformity pin's doc; in the uniformity test, build the biased draw as a local closure (`draw_arity(9, rng).min(draw_arity(9, rng))`), tally it over the same 2000 draws, and assert its statistic exceeds the threshold, dropping the 723.2 literal. Acceptance: one arity pin remains, still red on a skipped count, and it fails if the threshold is raised past the known-bad statistic.

### fuelscape-pipeline-7: The chi-square statistic is written three times; plan/tests.rs hardcodes the threshold sample/tests.rs derives
- Where: crates/before-fuelscape/src/plan/tests.rs:188-197 (related: crates/before-fuelscape/src/sample/tests.rs:96-106, 140-152, 184-196)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read the three loops; `chi_square_threshold(9)` = 8 + 6 * sqrt(16) = 32.0, the literal at line 197); executed: no
- Seen by: structure-prose [29]; refutation: confirmed; history: the arity pin's commit names the sampler pins' 6-sigma idiom it copies and still re-derives the bound by hand
- Owner-gated: no

One formula, one home: the 6-sigma one-sided bound is a shared criterion, and a hand-copied 32.0 silently diverges if `chi_square_threshold` is ever tightened (Principle 5, hand-maintained numbers). The two sampler pins duplicate about forty lines of the same tally-and-assert.

Evidence:

       188	    let expected = DRAWS as f64 / TOTAL as f64;
       189	    let chi2: f64 = observed
       190	        .iter()
       191	        .map(|&o| {
       192	            let d = o as f64 - expected;
       193	            d * d / expected
       194	        })
       195	        .sum();
       196	    // 8 degrees of freedom: mean 8, variance 16, so 8 + 6·4 = 32.
       197	    let threshold = 32.0;

Resolution: A `#[cfg(test)]` helper module (`src/testing.rs`) with `chi_square(observed: &[u64], expected: f64) -> f64`, `chi_square_threshold(categories) -> f64`, and `assert_uniform(observed, label)`; the two sampler pins, the arity pin, and the split pin proposed in fuelscape-pipeline-9 call it. Acceptance: the literal `32.0` is gone from plan/tests.rs and one chi-square implementation exists in the crate.

### fuelscape-pipeline-8: fold_rows_expose_the_arity_axis_in_fuel asserts a sign with no margin, on a tuple sort that biases toward passing
- Where: crates/before-fuelscape/src/plan/tests.rs:236-252 (related: crates/before-fuelscape/src/plan/tests.rs:204-216; crates/before-fuelscape/src/plan.rs:388-408)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read; the test needs the guest wasm, which no permitted command builds here, so the pass probability under the null is reasoned, not measured); executed: no
- Seen by: adequacy [18], instrument-correctness [47]; refutation: confirmed (could not run the mutant either); history: deliberate-and-holds: the criterion and its purpose were stated in 98c63b26; the commit asserts without a demonstration that a fixed-arity kernel cannot produce the reading, so the null-hypothesis analysis is new
- Owner-gated: no

`column.sort_unstable()` sorts `(arity, fuel)` tuples, so within the median arity group the lower-fuel samples land in the "low" half and the higher-fuel ones in the "high" half; under a null of fuel independent of arity the two means still differ in the passing direction by that group's spread, and `high > low` accepts any positive difference. A known-bad measure (fuel independent of the recorded arity) therefore passes at a seed-determined fraction of seeds well above one half, and at the committed seed it passes or fails forever. Doctrine: a criterion a bad implementation can pass by luck is decoration.

Evidence:

       236	        column.sort_unstable();
       ...
       243	        let half = column.len() / 2;
       244	        let mean = |cells: &[(usize, u64)]| {
       245	            cells.iter().map(|&(_, fuel)| fuel).sum::<u64>() as f64 / cells.len() as f64
       246	        };
       247	        let (low, high) = (mean(&column[..half]), mean(&column[half..]));
       248	        assert!(
       249	            high > low,

Resolution: Sort by arity alone with a stable sort (`sort_by_key(|&(arity, _)| arity)`) and require a relationship with margin that costs no fuel threshold: a Spearman rank correlation between arity and fuel across the column at or above a stated bound, or the mean of the top-arity third exceeding the bottom-arity third by a factor the fold's `log k` model predicts. Commit the known-bad demonstration beside it. Acceptance: passing a constant arity to `op.measure` at plan.rs:394 while leaving `CellSample.arity` as drawn fails the test deterministically at the committed seed for both fold rows.
Construction: At plan.rs:394 replace `(op.measure)(&mut guest, &inputs, arity)` with `(op.measure)(&mut guest, &inputs, 4)` for the `PartyShares` row (for `clock_join_all`, whose measure ignores the argument, fix the drawn clock count instead). Fuel is then independent of the recorded arity; sorting by `(arity, fuel)` and splitting at the median yields two means whose order depends only on which party bytes landed in which half at seed 0x5eed.

### fuelscape-pipeline-9: split_budget is pinned for reachability where the module doc claims exact uniformity, and the suite's own argument says reachability cannot pin uniformity
- Where: crates/before-fuelscape/src/plan/tests.rs:263-276 (related: crates/before-fuelscape/src/plan.rs:180-206, 11-22; crates/before-fuelscape/src/plan/tests.rs:167-178, 127-143; crates/before-fuelscape/src/ops.rs:49-59)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read; the implementation, distinct cuts via `BTreeSet` rejection, is a uniform k-subset draw and is correct, so this is a missing pin, not a bug); executed: no
- Seen by: adequacy [16], instrument-correctness [46]; refutation: confirmed; history: the reachability test came with the k-way split (eb6b35f42); the audit that added the arity chi-square (04cbd1ab, adopted 8409fe55) discussed the arity draw only
- Owner-gated: no

plan.rs:180-185 claims the cut-point rejection "keeps the composition draw exactly uniform" and every multi-operand row's stamped size measure rests on it; the only pins are positivity/sum (127-143) and that the ten compositions of (6, 3) appear. The sibling test at 167-178 states the principle: a biased-but-total draw reaches every count and passes a reachability check. Principle 6: the worst artifact that passes here is a biased split, which skews the x-axis of every binary, ternary, and slice panel while the caption still says "split uniform".

Evidence:

       263	#[test]
       264	fn split_budget_reaches_every_composition() {
       265	    let mut rng = cell_rng(0xc0de, "split-reach", 6, 3);
       266	    let mut seen = std::collections::BTreeSet::new();
       267	    for _ in 0..2_000 {
       268	        seen.insert(split_budget(6, 3, &mut rng));
       269	    }
       270	    // C(5, 2) = 10 compositions of 6 into 3 positive parts.
       271	    assert_eq!(
       272	        seen.len(),
       273	        10,

Resolution: Replace the reachability test with a one-sided chi-square over the ten compositions of (6, 3) at 2000 draws (expected 200 each; threshold `chi_square_threshold(10)` = 9 + 6 * sqrt(18), about 34.5, the sampler pins' idiom), which subsumes reachability (an unreached composition alone contributes 200). Acceptance: green on the current `split_budget`, red on either known-bad split below, at the committed seed.
Construction: Known-bad A (stick-breaking): first part `gen_range(1..=total - parts + 1)`, recurse on the remainder; for (6, 3), P((4,1,1)) = 1/4 and P((1,1,4)) = 1/16 against the uniform 1/10; every composition is reachable, sum and positivity hold, and the chi-square over 2000 draws reads in the hundreds. Known-bad B (collision nudge): draw `parts - 1` cuts in `1..total` with replacement, sort, and move each duplicate to the next unused position; for (6, 3) the compositions with adjacent cuts carry 3/25 each and the rest 2/25, all reachable, statistic about 84.

### fuelscape-pipeline-10: The count module says every count is pinned two independent ways; the version table reaches the decoder only through the enumeration
- Where: crates/before-fuelscape/src/count.rs:31-35 (related: crates/before-fuelscape/src/count/tests.rs:23-33, 192-205, 219-230)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read the three tests: `version_counts_match_exhaustive_enumeration` compares table to enumeration; `version_decoder_census_matches_constrained_family` compares the constrained enumeration to `Version::decode`, never the table; `party_decoder_census_matches_count_table` compares the party table to `Party::decode` directly); executed: no
- Seen by: refutation (new); history: none
- Owner-gated: no

The version table counts the unconstrained sibling-rule family, so no direct table-versus-decoder comparison exists for it; the table reaches the shipping decoder transitively through the enumeration's grammar. The sentence is accurate for the party table and one hop short for the version table. Doc accuracy at the maintainer's altitude.

Evidence:

        31	//! Every count here is pinned against ground truth in two independent
        32	//! ways (`tests.rs`): exhaustive enumeration of the grammar, and the real
        33	//! decoders' accept sets over all short byte strings. The builds run their

Resolution: "Every count is pinned against the grammar enumeration; the party table is also pinned directly against `Party::decode`'s accept census, and the version table reaches `Version::decode` through the enumeration (the constrained enumeration equals the decoder census, the table equals the unconstrained enumeration)". Acceptance: the sentence names the two hops for the version table.

### fuelscape-pipeline-11: split_sum's hand-written parallel/sequential branch, its reference twin, and the 2048-bit pin dissolve into rayon's with_min_len
- Where: crates/before-fuelscape/src/count.rs:96-115 (related: crates/before-fuelscape/src/count.rs:73-84, 150-156, 224-230; crates/before-fuelscape/src/count/tests.rs:75-111)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (read rayon 1.12.0 in the cargo registry: `IndexedParallelIterator` for `RangeInclusive` is implemented only at u8/u16/i8/i16/char (range_inclusive.rs:223-226) while `Range<usize>` is indexed (range.rs:286); the enumeration pins at 24 bits reach at most 20 splits, below `PAR_SPLIT_THRESHOLD = 1536`, so the pin is today the only committed check that enters the parallel arm); executed: no
- Seen by: scaffolding [3]; refutation: reframed (the pin is not circular today; the one-liner needs a `Range<usize>`; "dated" is inaccurate: the threshold doc is machine-specific but undated); history: deliberate-and-holds: the reference build and pin were introduced on purpose with the rationale inline and in the commit (969cf3ae1)
- Owner-gated: no

`split_sum` branches by hand on a threshold between a rayon reduction and a sequential fold; `build_sequential` on both tables exists so `parallel_build_matches_sequential_reference` can compare the two transcriptions, four 2048-entry big-integer builds per test run. What the pin checks (order-independence of exact-integer addition across rayon's reduction tree) is a property of rayon and num-bigint, and rayon's `with_min_len` is exactly the "sequential below a length" rule written here by hand. Principle 3: infrastructure that reimplements a mature tool's capability, plus the maintenance it generates (a threshold constant, a reference twin, a pin with its own liveness guard). The pin is the only current reach into the parallel arm, so the dissolution and the rewrite land together, not the pin's deletion alone. The threshold's inline measurement table (count.rs:76-83) is what doctrine asks of a tuning constant and is not part of this finding.

Evidence:

        96	fn split_sum(subtree: &[BigUint], splits: RangeInclusive<usize>, pair_sum: usize) -> BigUint {
        97	    if splits.end() - splits.start() + 1 < PAR_SPLIT_THRESHOLD {
        98	        split_sum_sequential(subtree, splits, pair_sum)
        99	    } else {
       100	        splits
       101	            .into_par_iter()
       102	            .map(|a| &subtree[a] * &subtree[pair_sum - a])
       103	            .reduce(BigUint::zero, |x, y| x + y)
       104	    }
       105	}

Resolution: One chain over `*splits.start()..*splits.end() + 1` (a `Range<usize>`, which rayon indexes): `.into_par_iter().with_min_len(PAR_SPLIT_THRESHOLD).map(|a| &subtree[a] * &subtree[pair_sum - a]).sum::<BigUint>()`. Delete `split_sum_sequential`, both `build_sequential`, the `sum:` parameter of `build_with`, and `parallel_build_matches_sequential_reference`; restate the threshold doc as the `with_min_len` floor (rayon's splitter stops splitting when a half would fall below it, so the sequential regime is roughly twice the constant; the doc already calls the exact cut low-stakes). Acceptance: count.rs holds one convolution expression; count/tests.rs retains the enumeration and decoder pins; `just fuelscape-test` passes and its wall time drops by the four 2048-entry builds.

### fuelscape-pipeline-12: VersionCounts and PartyCounts, and the two samplers, duplicate every method but the recurrence
- Where: crates/before-fuelscape/src/count.rs:137-156 (related: crates/before-fuelscape/src/count.rs:204-278; crates/before-fuelscape/src/sample.rs:184-207, 380-400; crates/before-fuelscape/src/plan.rs:129-158)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read: `build`, `build_with_progress`, `build_sequential`, the `build_with` signature, `subtree`, `whole`, `max_bits` are written twice with identical bodies; only the two `build_with` loop bodies differ); executed: no
- Seen by: structure-prose [27]; refutation: confirmed, with the caveat that the two named types keep a `VersionSampler` from being handed `PartyCounts`; history: both impl blocks date from the crate's first commit (3d6ab1d49); the later `build_with` refactor deduplicated within each grammar only
- Owner-gated: no

The grammar recurrences are the payload and deserve to sit side by side; the sixty lines of identical scaffolding around them are what a reader skims past to find them, and a change to the progress protocol or the convolution strategy touches two (with the samplers, six) sites. Legibility standard.

Evidence:

       137	    pub fn build(max_bits: usize) -> VersionCounts {
       138	        Self::build_with(max_bits, split_sum, |_, _| {})
       139	    }
       ...
       143	    pub fn build_with_progress(
       144	        max_bits: usize,
       145	        progress: impl FnMut(usize, usize),
       146	    ) -> VersionCounts {
       147	        Self::build_with(max_bits, split_sum, progress)
       148	    }
       ...
       154	    pub fn build_sequential(max_bits: usize) -> VersionCounts {
       155	        Self::build_with(max_bits, split_sum_sequential, |_, _| {})
       156	    }

Resolution: One `Counts<G: Grammar>` with the shared methods, a `Grammar` trait with one method (`fn entry(j: usize, subtree: &[BigUint]) -> BigUint`) and two unit types holding the recurrences; `VersionCounts = Counts<VersionGrammar>` and `PartyCounts = Counts<PartyGrammar>` keep the typed distinction the refutation notes. Behavior-preserving. Acceptance: count/tests.rs passes with constructor names unchanged; both recurrences read in one screen.

### fuelscape-pipeline-13: Dead accessors: counts() on both samplers, max_bits() on both tables, BitSink::len and is_empty
- Where: crates/before-fuelscape/src/count.rs:191-194 (related: crates/before-fuelscape/src/count.rs:274-277; crates/before-fuelscape/src/sample.rs:62-70, 204-207, 397-400)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rn '\.counts()' crates/before-fuelscape/src` and `'\.max_bits()'` return nothing; the only `.len()` in sample.rs is `self.bytes.len()` at line 53; `BitSink` is used outside sample.rs only at enumerate.rs:157 through `default`/`push`/`into_bytes`; the spanbands binary uses `VersionSampler::new` and `sample_bytes` only); executed: no
- Seen by: structure-prose [25]; refutation: confirmed; history: no-rationale-found: the accessors and the doc naming "plan-time feasibility checks" arrived together (3d6ab1d49) and have had no caller at any commit since
- Owner-gated: no

Principle 3: an accessor earns its place by naming a caller outside itself; `counts()`'s docstring names a use ("plan-time feasibility checks") that does not exist, which is a ghost reference in the forward direction, and `is_empty` exists only to satisfy clippy's `len_without_is_empty` for a `len` nobody reads.

Evidence:

    (count.rs)
       191	    /// The largest subtree size the table covers.
       192	    pub fn max_bits(&self) -> usize {
       193	        self.subtree.len() - 1
       194	    }

    (sample.rs)
       204	    /// The count table (for tests and plan-time feasibility checks).
       205	    pub fn counts(&self) -> &VersionCounts {
       206	        &self.counts
       207	    }

Resolution: Delete the six methods. If a plan-time feasibility check is intended (a plan whose span exceeds the table), write it and keep `counts()` with that caller. Acceptance: `cargo check` in crates/before-fuelscape is clean with the methods removed.

### fuelscape-pipeline-14: Preconditions unstated where the arithmetic relies on them: bit_window(0), sample_bytes past the table, ClockSlice at one byte
- Where: crates/before-fuelscape/src/count.rs:286-290 (related: crates/before-fuelscape/src/sample.rs:209-217, 402-409; crates/before-fuelscape/src/count.rs:181-183; crates/before-fuelscape/src/plan.rs:316; crates/before-fuelscape/src/ops.rs:118-128)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: verified (read: `bit_window(0, _)` computes `8 * bytes - 1` and `bytes - 1` with no guard; `sample_bytes` returns `None` only when the window's counts sum to zero, which cannot happen for `bytes >= 1` since `bit_window(1, _) == 2..=7` and the 2-bit leaf and terminal are counted, while a `bytes` past the table indexes `self.subtree[bits]` out of bounds at count.rs:182 before the check; `draw_arity(size - 1, rng)` at plan.rs:316 hands `gen_range(1..=0)` an empty range at size 1; every plan-side caller writes `.expect(...)`); executed: no
- Seen by: adequacy [23], instrument-correctness [52]; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

Every panic message should be a one-line proof; here the unreachable failure has a channel (`Option`) and the reachable one (a size past the table, or zero bytes) has none, so the type tells the reader the wrong story and the premise "callers pass `1 <= bytes <= span`" is named nowhere near the arithmetic. All three are guarded today by `Inputs::min_bytes` and `Samplers::build`, far from the sites.

Evidence:

    (count.rs)
       286	pub fn bit_window(bytes: usize, min_bits: usize) -> std::ops::RangeInclusive<usize> {
       287	    let hi = 8 * bytes - 1;
       288	    let lo = (8 * (bytes - 1)).max(min_bits);
       289	    lo..=hi
       290	}

    (sample.rs)
       209	    /// Draw one version uniformly from the canonical versions whose packed
       210	    /// encoding is exactly `bytes` bytes. `None` if the space is empty
       211	    /// (it is not, for any `bytes >= 1` within the table).
       212	    pub fn sample_bytes(&self, bytes: usize, rng: &mut ChaCha12Rng) -> Option<VersionDraw> {

Resolution: `assert!(bytes >= 1, "a packed encoding has at least one byte")` at the head of `bit_window`; have `sample_bytes` return the draw directly with a `# Panics` section stating `1 <= bytes <= span` (or check the span and make that the `None`), removing the seven `.expect` sites in plan.rs; at plan.rs:316 either assert `size >= 2` naming the `ClockSlice` minimum or route the cap through `Inputs::min_bytes`. Also use the `RangeInclusive` import already at count.rs:41 in the return type. Acceptance: no `.expect` on `sample_bytes` remains in plan.rs; `bit_window(0, _)` panics with the named message in both profiles.

### fuelscape-pipeline-15: The census literals' claimed independent derivation is not in the tree
- Where: crates/before-fuelscape/src/count/tests.rs:35-49 (related: crates/before-fuelscape/src/count/tests.rs:172-205; .agent-notes/2026-07-27-before-version-entropy/before-version-entropy.md:315-319, 466-481)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn -i 'entropy census'` over crates/, tools/, and the justfile hits only count/tests.rs:40; the literal sequence appears in the tree only at count/tests.rs:48 and in the agent note at line 474-475, whose line 317 names the generating script at a path outside the repository); executed: no
- Seen by: scaffolding [4], adequacy [19], structure-prose [30], instrument-correctness [45]; refutation: reframed (the actionable part is the doc; deletion is an owner call); history: already-known: the note's section 10 specified this pin and its doc comment for a later agent, recorded the script as outside the repo, and the pin was transcribed by 3d6ab1d49; the decoder census (b858edec) later became the live second derivation; the `before` crate-doc sentence that once cited the census was removed on 2026-08-04
- Owner-gated: no

Principle 8 and Principle 5: the doc presents an out-of-tree program as a second derivation the test exercises, and cites it by description only; what the tree holds is one derivation plus a frozen literal. The literal keeps a real role the doc does not state: unlike the decoder census, it does not move when the enumeration and the decoder change together, so a canonical-form change must edit these integers deliberately (tamper evidence). `version_decoder_census_matches_constrained_family` already re-derives every listed length from `Version::decode` at 0..=23 bits, a superset of 1..=20.

Evidence:

        38	/// derived here by enumerating the coding grammar (topology bits + gamma
        39	/// payloads) and filtering on the nonnegative-height rule, must equal the
        40	/// counts independently derived by the entropy census of the same grammar
        41	/// (an exact dynamic program over the validator's accept rules,
        42	/// cross-pinned against brute force over all bit strings). Two
        43	/// derivations, one number: drift in either grammar transcription moves a
        44	/// committed integer.

Resolution: Re-state the doc against what the tree holds: the literals are the committed canonical-stream counts per bit length; the enumeration must reproduce them; the decoder census below re-derives the same numbers from the shipping parser, and this pin alone survives a coordinated change to both, so a canonical-form change edits these integers deliberately. Drop the reference to the out-of-tree program (or commit it as a test if its independence is wanted live). Acceptance: the doc comment names no derivation the tree does not contain and states the tamper-evidence role; the array is unchanged.

### fuelscape-pipeline-16: enumerate is a pub module with only test callers, and party_subtrees returns a documented tuple beside a named struct
- Where: crates/before-fuelscape/src/enumerate.rs:120 (related: crates/before-fuelscape/src/lib.rs:58; crates/before-fuelscape/src/enumerate.rs:14-23, 117-119)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -rn 'enumerate::'` over the crate hits only count/tests.rs:5 and sample/tests.rs:7; no `before_fuelscape::enumerate` use exists anywhere under crates/); executed: no
- Seen by: structure-prose [35]; refutation: confirmed; history: no-rationale-found: `pub mod` since the first commit, with only test callers ever, and the module doc scopes it to the adequacy pins
- Owner-gated: no

Types-first (a named field over a documented tuple position, which the version side already does with `VersionMember`), and no dead weight in the shipped binaries.

Evidence:

       117	/// Every canonical id subtree of exactly `n` bits: 2-bit presence tags,
       118	/// no node with two terminal children. The `bool` is "this subtree is the
       119	/// bare terminal".
       120	pub fn party_subtrees(n: usize) -> Vec<(Vec<bool>, bool)> {

Resolution: `#[cfg(test)] pub mod enumerate;` at lib.rs:58 (or move it under a `testing` module), and a `PartyMember { bits: Vec<bool>, terminal: bool }` mirroring `VersionMember`. Acceptance: `cargo build --bins` no longer compiles enumerate.rs; `party_subtrees` returns a named struct.

### fuelscape-pipeline-17: Two small doc inaccuracies: "zero pad" for the marker padding; EXHAUSTIVE_BYTES credits the wrong companion pin
- Where: crates/before-fuelscape/src/sample.rs:178-179 (related: crates/before-fuelscape/src/sample.rs:72-78, 376-377; crates/before-fuelscape/src/sample/tests.rs:11-17; crates/before-fuelscape/src/count/tests.rs:14, 119-127)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read: `BitSink::into_bytes` at 72-78 documents "one `1` marker bit, then zeros"; sample/tests.rs:14-16 credits "the grammar-level census pin (which reaches 24 bits member-by-member)", which is `version_counts_match_exhaustive_enumeration` at `EXHAUSTIVE_BITS = 24`, a table-versus-enumeration pin that never touches the decoder; decoder accept coverage to 23 bits comes from `decoder_census` at `CENSUS_BITS`); executed: no
- Seen by: structure-prose [40]; refutation: confirmed; history: deliberate-but-expired: "zero pad" was accurate before the marker-padding change (d800957e), which re-sealed `into_bytes` and left the two field docs
- Owner-gated: no

Doc comments must be accurate to the code they sit on; both are one-clause fixes.

Evidence:

    (sample.rs)
       178	    /// The live bit length before the zero pad.
       179	    pub bits: usize,

    (sample/tests.rs)
        14	/// pins at seconds under the dev profile. Together with the grammar-level
        15	/// census pin (which reaches 24 bits member-by-member) the accept set is
        16	/// covered into the low twenties of bits.

Resolution: "before the marker padding" at sample.rs:178 and 376; at sample/tests.rs:14-16 name count/tests.rs's decoder census (23 bits) as the companion that carries the accept set past two bytes. Acceptance: both comments name the mechanism the code implements.

### fuelscape-pipeline-18: The zero-leaf exclusion is spelled through version_leaf_count at a corner argument; the two samplers differ in assert strength; a dead .max(1)
- Where: crates/before-fuelscape/src/sample.rs:289-290 (related: crates/before-fuelscape/src/sample.rs:269, 445, 473-488; crates/before-fuelscape/src/count.rs:65-71; crates/before-fuelscape/src/families.rs:57, 74, 151, 197)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read: `version_leaf_count(2) == 1` and `version_leaf_count(0) == 0` at count.rs:65-71, so the expression is a 0/1 indicator; the party sampler spells the same exclusion as explicit `BigUint::from(1u32)`/`zero()` at 473-488; the membership check is `debug_assert!` at 269 and `assert!` at 445; in `ramp`, `t` starts at 1 and doubles, so `t.max(1)` at families.rs:151 and 197 is dead); executed: no
- Seen by: structure-prose [37], instrument-correctness [55]; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

Finished code should be obviously correct: the reader must evaluate a function at two magic arguments to see a 0/1, two spellings and two assert strengths exist for one fact, and `.max(1)` suggests a `t = 0` case that cannot occur.

Evidence:

       289	            let bare = version_leaf_count(a)
       290	                * (self.counts.subtree(b) - version_leaf_count(if b == 2 { 2 } else { 0 }));

Resolution: A named helper, e.g. `fn excluded_two_bit_leaf(bits: usize) -> BigUint { BigUint::from(u32::from(bits == 2)) }`, used by both samplers (the count table's `leaf(j - 3)` derivation at count.rs:172 can stay, it is correct and explained); one assert strength for the two `total.is_zero()` checks, saying what a failure means (a table/sampler mismatch); drop `.max(1)` at families.rs:151 and 197. Acceptance: both samplers express the exclusion through one named function; the two membership asserts match.

### fuelscape-pipeline-19: The samplers recurse on the drawn tree's depth
- Where: crates/before-fuelscape/src/sample.rs:307-315 (related: crates/before-fuelscape/src/sample.rs:463, 495-502; crates/before/AGENTS.md:26-37)
- Class / severity / confidence: correctness / nit / medium
- Provenance: assessed (read: `VersionSampler::subtree` recurses at 307, 312, 315 and `PartySampler::subtree` at 463, 495, 501-502, with no explicit stack and no stated depth bound); executed: no
- Seen by: instrument-correctness [51]; refutation: confirmed (before's rule binds the library, not this detached dev crate); history: no-rationale-found
- Owner-gated: no

`before`'s hard rule ("No library traversal recurses on tree depth") is scoped to the library, and the depth here is drawn from a uniform measure rather than chosen by an adversary, so this is a suggestion: a left-spine version costs 3 bits per level and a unary party chain 2, so a 4096-byte span (the spanbands default) admits depths near 11,000 and 16,000 frames, each carrying several `BigUint` temporaries, against rayon's 2 MiB worker stack. The probability under the uniform draw is negligible; the bound is unstated where the recursion lives.

Evidence:

       307	                return self.subtree(b, Mode::NoZeroLeaf, sink, walk, rng);
       ...
       312	                if !self.subtree(a, Mode::NoLeaf, sink, walk, rng) {
       313	                    return false;
       314	                }
       315	                return self.subtree(b, Mode::Free, sink, walk, rng);

Resolution: Either state the depth bound (bits/3 and bits/2) and the span it implies at the two recursive sites, or iterate with an explicit stack of pending `(bits, mode)` frames as the library's walks do. Acceptance: a comment or an explicit stack at both recursive sites; if iterative, the uniformity and round-trip pins stay green.
Construction: A demonstration, not a test: in a debug build measure one `subtree` frame's size, multiply by the spine depth at `--max-bytes 4096` (32,768 bits / 3), and compare with the 2 MiB rayon worker stack.

### fuelscape-pipeline-20: Two exhaustive byte-string sweeps of the decoders, in two modules, with two bound constants
- Where: crates/before-fuelscape/src/sample/tests.rs:40-61 (related: crates/before-fuelscape/src/sample/tests.rs:11-17; crates/before-fuelscape/src/count/tests.rs:113-170)
- Class / severity / confidence: simplification / nit / medium
- Provenance: assessed (read: `accepted_byte_strings`, a sequential odometer at `EXHAUSTIVE_BYTES = 2` yielding sets, and `decoder_census`, a rayon fold at `CENSUS_BYTES = 3` yielding per-bit-length counts, each with its own runtime rationale); executed: no
- Seen by: scaffolding [5], structure-prose [29] (the sweep remark); refutation: confirmed at nit; history: the second sweep landed a day after the first (b858edec) with no reason for a parallel implementation in another module
- Owner-gated: no

One rayon sweep yielding the accepted set per length gives set equality (the stronger check) at 3 bytes, from which the per-bit-length histogram follows by `encoded_bits`; the two constants and their rationales collapse to one. Principle 3.

Evidence:

    (sample/tests.rs)
        40	/// All byte strings of exactly `len` bytes that `accept` takes.
        41	fn accepted_byte_strings(len: usize, accept: impl Fn(&[u8]) -> bool) -> BTreeSet<Vec<u8>> {

    (count/tests.rs)
       119	const CENSUS_BYTES: usize = 3;

Resolution: One parallel `accepted_byte_strings(len) -> BTreeSet<Vec<u8>>` in a shared test helper; run the set-equality tests to 3 bytes and derive the census histogram from the sets; delete the odometer and one constant. Acceptance: one sweep function and one bound constant; set equality at 3 bytes for both grammars within the suite's current budget.

### fuelscape-pipeline-21: Stale hand-maintained count: "767 canonical members at exactly 2 bytes" is the pre-marker-padding window; the current window holds 433
- Where: crates/before-fuelscape/src/sample/tests.rs:118 (related: crates/before-fuelscape/src/count/tests.rs:47-49, 239; crates/before-fuelscape/src/count.rs:286-290)
- Class / severity / confidence: documentation / low / high
- Provenance: verified; executed: yes: a Python sum over the committed `CENSUS` literal (count/tests.rs:48) gives 433 for bits 8..=15 and 767 for bits 9..=16; `bit_window(2, MIN_VERSION_BITS) == 8..=15` is pinned at count/tests.rs:239; `git show d800957e -- count.rs` shows `bit_window` moving from `hi = 8 * bytes; lo = 8 * (bytes - 1) + 1` to `hi = 8 * bytes - 1; lo = 8 * (bytes - 1)`
- Seen by: scaffolding [6], instrument-correctness [44] (structure-prose [36] asserted the number is "correct today", which is false); refutation: confirmed; history: deliberate-but-expired: correct when written (3d6ab1d49), expired at d800957e
- Owner-gated: no

Principle 5: no hand-maintained counts in prose. The test computes `members.len()` itself and is unaffected; the comment has already rotted once and would mislead anyone calibrating the chi-square's category count from it.

Evidence:

       118	    let size = 2; // 767 canonical members at exactly 2 bytes.

Resolution: Delete the number; if a size rationale is wanted, "a few hundred members: enough categories for the chi-square, small enough to enumerate". Acceptance: no literal member count remains in sample/tests.rs.

### fuelscape-pipeline-22: ramp's 1 << 20 guard contradicts its comment and names what it catches nowhere
- Where: crates/before-fuelscape/src/families.rs:58-60
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read families.rs:48-77); executed: no
- Seen by: structure-prose [41]; refutation: confirmed, with the correction that the bound does catch a constructible failure (a generator whose packed size stops growing would otherwise loop forever), so the fix is to name it, not drop it; history: no-rationale-found
- Owner-gated: no

Named constants over magic numbers, and a guard must name what it catches: the comment argues the span bounds the loop, and the loop is bounded again by an unnamed `2^20`, so the reader gets two contradictory bounds.

Evidence:

        58	    // Doubling knobs; every generator here grows at least linearly in its
        59	    // knob, so the loop is bounded by the span.
        60	    while t <= 1 << 20 {

Resolution: `const MAX_RAMP_KNOB: usize = 1 << 20;` with a comment stating the failure it catches (a generator whose output stops growing with its knob would otherwise never exceed the span), and reword line 58-59 to say the span is the expected exit and the knob cap the guard. Acceptance: the loop has one stated bound and one named guard with its reason.

### fuelscape-pipeline-23: Self-pair overlay points on rows whose kernel opens with canonical_eq measure the equality rung, not the family
- Where: crates/before-fuelscape/src/families.rs:158-165 (related: crates/before-fuelscape/src/families.rs:91-92, 342-349; crates/before/src/version.rs:391, 432, 865, 890, 913, 935, 970; crates/before/src/version/ranked.rs:342; crates/before/src/causally/conjunction.rs:41, 45, 82, 98; crates/before/src/party/ops/compare.rs:16-21, 59-99; crates/before/src/version/skyline/sweep.rs:100; crates/before-fuelscape/src/ops.rs:530-547; crates/before-fuelscape/src/render.rs:501-509; crates/before-fuelscape/src/compact.rs:146-148)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified (read the routing: the guest's `ff_version_join` computes `va | vb` and `ff_version_meet` `va & vb`, whose views open with `codec::canonical_eq` at version.rs:865 and 913 (`join_refs`/`meet_refs` at 890 and 935 the same); `ff_version_span` calls `va.span(vb)`, which routes to `span_refs` and its rung at 970; `distance` and `lag` at 391 and 432; `ranked_cmp`'s `total_cmp` at ranked.rs:342; the conjunction's floor join and ceiling meet route to `join_refs`/`meet_refs` at conjunction.rs:41, 45, 82, 98; `IdReader::is_disjoint` is `lockstep_holds` with `a_settles = Empty`, and any `(Full, Full)` pairing returns `false` at compare.rs:98, so a self-pair refutes at its first owned leaf; `causal_cmp` has only a `ptr_eq` rung (sweep.rs:100) and `causally` contains no `canonical_eq`, so `version_cmp`, `version_concurrent`, and the contains rows are unaffected); executed: no
- Seen by: instrument-correctness [42]; refutation: confirmed end to end, adding the party `is_disjoint` early exit; history: the self-pairs were added as a labeled fallback for signatures without a committed pair generator (69a22b390), and `join_view`/`meet_view` already opened with `canonical_eq` that day, so the join and meet points have read the equality rung from the start; the distance, lag, and ranked rungs arrived two days later (1bb610d19), so that subset is deliberate-but-expired
- Owner-gated: no

The module doc (families.rs:5-9) says the overlay exists to show "where the adversarial frontier sits relative to the population" because engineered corners are measure-zero in the cloud. On version_join, version_meet, version_span, version_distance, version_lag, ranked_cmp, query_conjoin_floors, and query_conjoin_ceilings the "dense × self" and "hugeleaf × self" points feed byte-identical operands into a kernel whose first rung is a byte compare and a refcount clone, so the marked point is the operation's cheapest path under its most adversarial-sounding label; the party self-pairs on party_is_disjoint exit at the first owned leaf the same way. The roster already reasons about short-circuits correctly elsewhere: the `version_eq` row (ops.rs:534-538) measures the equal pair because there memcmp is the worst case, and the distinct-party row gets committed crosses (families.rs:321-322). The self-pairs are the inverse mistake. The SVG gallery draws these points (render.rs:501-509) and the committed compact datasets carry them (compact.rs:146-148 defers drawing to a later widget revision), so the mislabel is in the audit artifacts now and will reach the doc islands when a revision draws overlays. The sentence at families.rs:91-92 ("unless a committed pair generator exists") also misdescribes the `[Version, Version]` arm, which carries the three pair generators and the two self-pairs together.

Evidence:

       158	        out.extend(ramp("dense × self", max_bytes, |t| {
       159	            let v = version_bytes(&Shape::Dense.packed1(t));
       160	            Some(vec![v.clone(), v])
       161	        }));
       162	        out.extend(ramp("hugeleaf × self", max_bytes, |t| {
       163	            let v = version_bytes(&Shape::Hugeleaf.packed1(8 * t));
       164	            Some(vec![v.clone(), v])
       165	        }));

    (crates/before/src/version.rs)
       890	        if codec::canonical_eq(&a.0, &b.0) {
       891	            return a.clone(); // a ∨ a = a

    (families.rs)
        91	/// Binary rows pair a family with itself (declared by the label) unless a
        92	/// committed pair generator exists — `jump_pair`, `tooth_tail`, and

Resolution: On the `[Version, Version]` arm replace the two self-pairs with perturbed twins that defeat the equality rung while keeping the shape (the family and the same family after one tick on its first leaf, or dense(t) crossed with dense at the next ramp point), or drop them and rely on the three committed pair generators already present; do the same for the `[Party, Party]` self-pairs on rows with an early-exit predicate. State at `overlay_inputs` which rows own an equality short-circuit and that self-pairs are reserved for rows without one (version_cmp, version_concurrent, party_covers, the contains rows). Rewrite families.rs:91-92 to describe what the arm does. Acceptance: a committed test in a families.rs sibling `tests.rs`: for every `[Version, Version]` and `[Party, Party]` roster row, every `overlay_inputs` point has `inputs[0] != inputs[1]` unless the row is in an explicit allowlist of rows without an equality rung in the measured kernel.
Construction: Build `Plan { base_seed: 0x5eed, samples_per_column: 1, max_bytes: 64 }`, call `overlay_inputs` on the version_join row at 64, take the largest "dense × self" point, and run `(op.measure)(&mut Guest::new(), &fam.inputs, 2)`; compare its fuel with the `version_eq` row measured on `inputs[0]` at the same size. The two agree up to the clone's constant and both sit far below the "jump_pair" point at the same total size, which is the join sweep's cost.

### fuelscape-pipeline-24: slice_overlays dispatches on operation-name strings with a runtime panic as its only totality check
- Where: crates/before-fuelscape/src/families.rs:507-521 (related: crates/before-fuelscape/src/families.rs:98-113, 385; crates/before-fuelscape/src/render/tests.rs:32-43)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (read: nine name literals and an `other => panic!` arm; `overlay_inputs` panics on an unmapped signature at 385; the smoke test iterates every `ROSTER` row once at `max_bytes: 8`, which is what closes the hole today); executed: no
- Seen by: structure-prose [31]; refutation: confirmed (structure, not a gap); history: no-rationale-found: the string match arrived with the fold panels (eb6b35f42) and 46eb64f9 extended it
- Owner-gated: no

Types-first: which committed families mark which slice panel is roster knowledge held outside the roster, keyed by a string a rename does not update, and its totality is a runtime panic the smoke test happens to reach. A `SliceFamilies { Stagger, MeetShade, Both }` value on the `Inputs::VersionSlice`/`VersionSliceCapped` variants (or an `overlay` field on `OpSpec`) makes the assignment exhaustive at compile time and puts the per-row rationale beside the row it describes.

Evidence:

       507	fn slice_overlays(name: &str, max_bytes: usize) -> Vec<FamilyInput> {
       508	    match name {
       509	        "version_join_all" | "span_join_all" => stagger_ramps(max_bytes),
       ...
       513	        "shape_combine" => stagger_ramps(max_bytes),
       514	        "version_meet_all" | "span_meet_all" => meet_shade_ramp(max_bytes),
       ...
       520	        other => panic!("no committed slice families for {other}"),

Resolution: Move the slice-family choice into the roster (`Inputs::VersionSlice(SliceFamilies)`, `VersionSliceCapped(u32, SliceFamilies)`), delete the name match, and keep the signature-keyed table for fixed-arity rows (a genuine function of the signature). Keep the `other => panic!` at 385 only with a comment naming the smoke test as its check. Acceptance: no `match name` on string literals remains in families.rs; adding a slice row without choosing its families is a compile error.

### fuelscape-pipeline-25: "adding an operation is one OpSpec entry" names one step of the chain a new row requires
- Where: crates/before-fuelscape/src/ops.rs:4-5 (related: crates/before-fuelscape/src/families.rs:385; crates/before/src/testing/fuelscape_islands.rs:1-27; justfile:680-691)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (read: a row with a new operand signature panics at families.rs:385 in the smoke test; the measure needs a guest export; justfile:680-691 describes the re-measure, gzip, and `fuelscape-compact` re-pin; fuelscape_islands.rs requires every emitted island to be included by a doc comment or exempted); executed: no
- Seen by: scaffolding [13]; refutation: confirmed; history: deliberate-but-expired: written for a fifteen-row roster with no downstream artifacts (69a22b390); the compact datasets, committed dump, and islands totality test added the further steps
- Owner-gated: no

Maintainer-facing altitude: the module doc is where a future maintainer looks for the procedure, and the gate will otherwise teach it one failure at a time (guest kernel, overlay arm for a new signature, re-measure and `fuelscape-compact` re-pin, doc `include_str!` site in `before`).

Evidence:

         4	//! One table row per operation — adding an operation is one [`OpSpec`]
         5	//! entry. Each row names its input space (which picks the samplers and

Resolution: Replace the clause with a short "adding a row" list naming the kernel, the overlay arm (or its signature match), the re-measure and compact re-pin, and the doc include site. Acceptance: the module doc's procedure matches what the gate enforces.

### fuelscape-pipeline-26: "constant dispatch overhead, identical for every sample" is one register move per operand on the fold rows
- Where: crates/before-fuelscape/src/ops.rs:9-12 (related: crates/before/fuzzfit/guest/src/lib.rs:839-846, 1028-1035, 1165-1172, 1187-1200, 1212-1219)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read the guest: `ff_version_join_all`, `ff_party_join_all`, `ff_clock_join_all`, and `ff_clock_recv_all` take k registers into a `Vec` inside the measured call; `ff_clock_sync_all` also puts k clocks back); executed: no
- Seen by: instrument-correctness [54]; refutation: confirmed; history: deliberate-but-expired: accurate for the fifteen fixed-signature rows it was written for; the same day's commit added the first fold rows
- Owner-gated: no

The folds are Omega(k), so no asymptotic reading moves; the sentence still overstates, and a maintainer reading a fold panel should know the register bookkeeping rides at O(k) inside the window.

Evidence:

         9	//! one measured kernel, returning that call's fuel. Register loading and
        10	//! all other staging happen before the measured call, so a reading prices
        11	//! one public operation (plus the guest's constant dispatch overhead,
        12	//! identical for every sample).

Resolution: "plus the guest's dispatch overhead: constant for the fixed-signature rows, one register move per operand for the folds". Acceptance: the sentence matches the guest kernels.

### fuelscape-pipeline-27: The stratified-arity argument is stated three times and the totality chain four times
- Where: crates/before-fuelscape/src/ops.rs:61-68 (related: crates/before-fuelscape/src/plan.rs:11-22, 169-175 (arity); crates/before-fuelscape/src/lib.rs:22-28, ops.rs:14-22, 2160-2172, ops/tests.rs:7-16 (totality))
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (read all seven sites; no factual conflict among the copies today); executed: no
- Seen by: structure-prose [39]; refutation: confirmed; history: the stratification commit (c459b1d0) placed the full argument at the variant, the module doc, and the draw function at once; the totality chain accreted
- Owner-gated: no

Every sentence competes with the contract the reader came for; three copies of one argument drift, and the reader cannot tell which is authoritative.

Evidence:

        61	    /// Arity is stratified deliberately, not left to the composition
        62	    /// count: uniform over whole compositions would concentrate nearly
        63	    /// all mass at many-tiny-operand slices (the compositions of `N`
        64	    /// into `k` parts peak at `k ≈ N/2`), starving the thin-and-wide

Resolution: Keep the arity argument on the `Inputs::VersionSlice` declaration and have plan.rs cite it in one clause; keep the totality chain in ops.rs's module doc and let lib.rs, the `EXEMPTIONS` doc, and the test doc each point there in one sentence. Acceptance: each argument has one full statement and the other sites are one-line pointers.

### fuelscape-pipeline-28: COMBINE_ARITY_CAP claims a smoke-test pin to the guest's cap that does not exist
- Where: crates/before-fuelscape/src/ops.rs:198-203 (related: crates/before-fuelscape/src/ops.rs:205-207, 393-406; crates/before-fuelscape/src/plan.rs:293-311, 395-400; crates/before-fuelscape/src/render/tests.rs:15-22; crates/before/fuzzfit/guest/src/lib.rs:788-810, 831)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read: the smoke plan is `max_bytes: 8` (render/tests.rs:21), the capped draw is `draw_arity(size.min(cap as usize), rng)` (plan.rs:296), so no smoke arity exceeds 8; the guest's own `COMBINE_ARITY_CAP: u32 = 16` (guest lib.rs:791) refuses `n > COMBINE_ARITY_CAP` with `-1` (808-810) and its `dispatch!` list runs to 16 (831); `grep -rn 'shape_combine\|COMBINE_ARITY\|ff_shape_combine' crates/before-fuelscape/src` outside ops.rs hits only the overlay arm at families.rs:513; no test compares the constants); executed: no
- Seen by: scaffolding [2], adequacy [15], structure-prose [24], instrument-correctness [43]; refutation: confirmed, severity argued to low (dev-tool doc; the guest-below-host direction fails loudly in a survey; the host-below-guest direction leaves the stamped population self-consistent); history: no-rationale-found: the constant, the guest's constant, and the sentence landed in one commit (46eb64f9) that added no fuelscape test; the smoke plan's `max_bytes: 8` predates it
- Owner-gated: no

The doc names a check that does not exist: two hand-maintained constants in two crates must agree, the sentence tells a maintainer they are pinned, and nothing in the gate compares them. If the guest's cap drops below the host's, `ff_shape_combine` returns `-1` and the survey aborts at plan.rs:395-400 only when a column of 17 or more bytes draws an arity past the guest cap, hours after the gate read green; if the host's rises above the guest's, the same. Principle 8 (a claimed pin is "told", not verified) and Principle 6 (the cheapest passing artifact is a green gate over an inconsistent pair). I keep medium rather than the refutation's low because the sentence is a trap set for exactly the edit it describes: a maintainer bumping either cap will trust it and stop looking.

Evidence:

       198	/// The `shape_combine` row's arity cap.
       199	///
       200	/// The public combiner's arity is a compile-time constant, so the guest
       201	/// dispatches one instantiation per arity up to this bound (the guest's
       202	/// own cap, kept equal by the pipeline smoke test's combine case).
       203	const COMBINE_ARITY_CAP: u32 = 16;

    (render/tests.rs)
        18	    let plan = Plan {
        19	        base_seed: 0x5eed,
        20	        samples_per_column: 3,
        21	        max_bytes: 8,
        22	    };

Resolution: Add a boundary test beside the parity tests: build one `Guest`, load `COMBINE_ARITY_CAP` one-byte canonical versions, call `ff_shape_combine(0, COMBINE_ARITY_CAP)` and assert `ret >= 0`, load one more and call with `COMBINE_ARITY_CAP + 1`, asserting `ret == -1`. That pins host cap equal to guest cap at the boundary independently of the smoke's random draws (alternatively export the cap from the guest and derive the host constant from it). Rewrite lines 200-202 to name that test. Acceptance: changing either constant alone (and the guest's `dispatch!` list) fails `just fuelscape-test` by name.
Construction: Lower the guest constant at fuzzfit/guest/src/lib.rs:791 to 8 and trim `dispatch!` to `0..=8`; rebuild the guest. `just fuelscape-test` passes (every smoke arity is at most 8). `just fuelscape --max-bytes 32 shape_combine` then panics with "shape_combine: guest kernel reported -1 at size 32 sample N" on the first sample whose arity draw exceeds 8. Dually, raise the host constant to 32 with the guest at 16: same gate pass, same survey panic.

### fuelscape-pipeline-29: Constants restated as literals in stamped size-measure strings and overlay labels
- Where: crates/before-fuelscape/src/ops.rs:205-207 (related: crates/before-fuelscape/src/ops.rs:133, 142, 203, 425-428, 769-770, 980-981; crates/before-fuelscape/src/families.rs:433, 440-450; crates/before-fuelscape/src/compact.rs:260-270)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the pairs: `M_SLICE_CAPPED` "min(16, size)" beside `COMBINE_ARITY_CAP = 16`; `version_ticks` "10⁹" beside `TICKS_COUNT = 1_000_000_000`; `party_forks`/`clock_forks` "a declared constant, 8" beside `FORKS_SHARES = 8`; four "(k=8)" labels beside `PARTY_FOLD_OVERLAY_SHARES = 8`; compact.rs:260 compares the roster's `size_measure` against the dump's verbatim); executed: no
- Seen by: scaffolding [7], structure-prose [36]; refutation: reframed to nit on the ground that "a constant bump without a string edit fails `fuelscape-verify` loudly rather than lying silently"; history: each pair landed in one commit with no tie mechanism
- Owner-gated: no

The refutation's severity argument is backwards for the case this finding describes. compact.rs:260 compares the roster string to the dump's recorded string; if `COMBINE_ARITY_CAP` changes and `M_SLICE_CAPPED` does not, both strings still say 16, the comparison passes, and the stamped population description lies about what was sampled. Only the opposite edit (string changed, dump not re-measured) fails loudly, and that is the deliberate re-pin event. So these are load-bearing hand-maintained numbers (Principle 5) on strings that are stamped onto renders and committed widget data. The Cargo.toml gzip figure the lenses also cited is approximately right (257.9 MB / 245.9 MiB against "~242 MB") and is not an instance.

Evidence:

       203	const COMBINE_ARITY_CAP: u32 = 16;
       204	/// The size measure of the capped-arity slice row.
       205	const M_SLICE_CAPPED: &str = "total packed bytes; arity uniform over 1..=min(16, size) \
       206	     (the combiner's compile-time arity, capped at the guest's dispatch table), split \
       207	     uniform over the compositions";

    (compact.rs)
       260	        if spec.size_measure != data.size_measure {

Resolution: Build the stamped strings from the constants (`const_format::formatcp!`, a dependency in keeping with the crate's preference for libraries over hand-rolling) or assemble them at compaction time from `OpSpec` fields; label the party-fold overlays with `format!("... (k={PARTY_FOLD_OVERLAY_SHARES})")`. Failing that, one unit test in ops/tests.rs asserting each such string contains its constant formatted. Acceptance: changing `COMBINE_ARITY_CAP`, `FORKS_SHARES`, `TICKS_COUNT`, or `PARTY_FOLD_OVERLAY_SHARES` either needs no prose edit or fails a test.

### fuelscape-pipeline-30: Two exemption rosters over one surface: 51 rows excused twice with independently worded reasons
- Where: crates/before-fuelscape/src/ops.rs:2173-2229 (related: crates/before-fuelscape/src/ops.rs:2160-2172, 2229-2509; crates/before/src/meter/board/coverage.rs:271-300; crates/before/src/surface.rs:181-194; crates/before-fuelscape/src/ops/tests.rs:18-54)
- Class / severity / confidence: modularity / medium / high
- Provenance: verified; executed: yes: a Python extraction of the first string of each tuple gives `EXEMPTIONS` 72 rows, `BOARD_NOT_APPLICABLE` 79, `BOARD_PRICED` 91; 51 rows are excused by both tables, 21 are atlas-exempt but board-priced, 28 board-excused but atlas-panelled
- Seen by: scaffolding [1]; refutation: confirmed; history: no-rationale-found: the two tables landed the same day from two instruments' commits (eb6b35f42, 669cf310), each describing only its own tiling; neither the validation index nor the coverage module's doc records a decision to hold the classification twice
- Owner-gated: yes: the fix reshapes `SurfaceRow` in `before` (crates/before/src/surface.rs:183-194), the roster of record shared by the board and the atlas

Fifty-one `before::surface` rows carry two separately maintained prose reasons for one fact, that an O(1) accessor or constructor has no size axis, and every new such item costs a surface row plus two exemption lines in two crates. Principle 3: a duplicated roster generating its own maintenance cascade (two reason tables drifting in wording for one mechanism). The 21 rows the atlas exempts but the board prices, and the 28 the board excuses but the atlas panels, are genuine instrument-specific decisions and belong where they are; the 51 shared rows are a property of the surface row itself.

Evidence:

    (ops.rs)
      2196	    ("Party::as_bytes", "O(1) borrow of the stored packed bytes"),

    (crates/before/src/meter/board/coverage.rs)
       300	    ("Party::as_bytes", "a borrow of the stored canonical bytes"),

Resolution: Give `SurfaceRow` a size-axis classification (e.g. `axis: SizeAxis`, with `SizeAxis::None(&'static str)` carrying the one reviewed reason) and have both tilings treat `SizeAxis::None` rows as excused automatically; each instrument's table then shrinks to its own decisions (atlas: delegating wrappers priced at another panel; board: rows it excuses but the atlas panels). Acceptance: `EXEMPTIONS` and `BOARD_NOT_APPLICABLE` name no row whose `SurfaceRow` carries `SizeAxis::None`; both tiling tests pass; adding an O(1) accessor requires exactly one reason, on its surface row.

### fuelscape-pipeline-31: Exemption reasons cite panels in unchecked prose; the Ticks entry names "min_ticks", a panel that is not a roster name
- Where: crates/before-fuelscape/src/ops.rs:2354-2358 (related: crates/before-fuelscape/src/ops.rs:368, 2163-2165, 2364-2368, 2385-2391, 2424-2433; crates/before-fuelscape/src/ops/tests.rs:18-54)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (a script extracted `<token> panel` references from the `EXEMPTIONS` block and checked them against the roster's `name:` literals: 31 references, of which `min_ticks`, `codec`, `combine`, `membership`, and the article `a` resolve to no row; the roster row is `version_min_ticks` at ops.rs:368; the parity test checks only surface-row names); executed: no
- Seen by: scaffolding [9]; refutation: reframed (several references are shorthand for a family of panels, so "does not exist" overstates one instance; the structural point stands; structuring the reasons is a design choice); history: the row was already named `version_min_ticks` when the reason was written, so this is authoring shorthand, not a stale rename
- Owner-gated: no

The exemption table is the reviewed artifact of the totality binding (ops.rs:2163-2165); a reason that points at a panel is a claim about the roster, and it is a remembered one. The Ticks entry is the one that reads as a name and is not.

Evidence:

      2354	    (
      2355	        "Ticks ZERO / From / TryFrom / FromStr / Display / Add / Sum / Ord / Eq / Hash",
      2356	        "the opaque count carrier's own arithmetic and text, not a tree walk; its \
      2357	         semantics are priced at the min_ticks panel",
      2358	    ),

Resolution: Write `version_min_ticks` at 2357. If the panel references are worth enforcing, structure the exemptions as `Exemption::NoSizeAxis(&str)` and `Exemption::PricedAt { panels: &[&str], reason: &str }` and extend `panels_and_exemptions_tile_the_coverage_roster` to assert every `PricedAt` panel is a `ROSTER` name; the shorthand entries ("the codec panels", "the membership panels") then either list their panels or stay prose under `NoSizeAxis`-style free text. Acceptance: the Ticks entry names a live panel; if structured, renaming a roster row fails the tiling test at every exemption that cites it.

### fuelscape-pipeline-32: spanbands is a one-off investigation binary that re-implements the plan's split rule and hand-rolls its CLI
- Where: crates/before-fuelscape/src/bin/spanbands.rs:108-121 (related: crates/before-fuelscape/src/bin/spanbands.rs:65-83; crates/before-fuelscape/src/plan.rs:189-206, 213-243; crates/before-fuelscape/Cargo.toml:18-21, 36-40)
- Class / severity / confidence: vestigial / low / medium
- Provenance: verified (`git log -- crates/before-fuelscape/src/bin/spanbands.rs` shows one commit, 5b2ae58c, 2026-07-31; `grep -rln spanbands .agent-notes justfile` finds nothing; only `bin/fuelscape.rs` uses clap; spanbands.rs:114 inlines `rng.gen_range(1..size)` while `split_budget` and `draw_packed` are private); executed: no
- Seen by: scaffolding [10]; refutation: confirmed (at HEAD the equivalence holds exactly, since `split_budget(total, 2)` is one `gen_range(1..total)`; the hazard is future drift; whether the banding question is settled is owner knowledge); history: no-rationale-found: the commit states the question it instruments; no note, results file, or later commit records what the CSV showed
- Owner-gated: no

This anchor lies outside the fourteen listed partition files; it is included because the binary duplicates the partition's draw rule. It claims to re-draw the atlas's two-operand input space but inlines the binary split instead of calling `plan`'s, so a change to `split_budget` breaks the equivalence silently (Principle 3, duplicated generator); it parses flags by hand while Cargo.toml:36-40 says clap serves "the runner binaries"; and the classify-first-versus-emit-always question it was built to answer has no recorded verdict.

Evidence:

       111	            let mut rng = cell_rng(seed, "spanbands", size, index);
       112	            // The binary split rule of the atlas's two-operand rows: one
       113	            // uniform cut in `1..size`.
       114	            let split_a = rng.gen_range(1..size);
       115	            let split_b = size - split_a;

Resolution: If the banding question is settled, record the verdict in an agent note and delete the binary (and the `scan-meter`/`limb-meter` features Cargo.toml:18-21 pulls for it, if nothing else reads them). If it is still wanted, make it a `--native-counters` mode of the main runner that draws through `plan` (a `pub(crate) draw_pair`, or `draw_packed` made `pub(crate)`) and uses the shared clap `Args`. Acceptance: either the binary is gone with its conclusion recorded, or it draws through `plan` and a change to `split_budget` changes its pairs.

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
