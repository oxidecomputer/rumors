# Partition testing-diff-gen: The differential table, generators, op traces, rng, brute-force grow, compactness, snapshots, asymptotics pins, shape rows, fuelscape islands

## Partition summary

This partition is the test-side apparatus of `before`: thirteen files under `crates/before/src/testing/`, all compiled only under `cfg(test)` (`#[cfg(test)] mod testing;` at `crates/before/src/lib.rs:453-454`), so none of it ships. Six of the files hold `#[test]` functions directly: `diff_ops/tests.rs` (the descriptor table's guards, drivers, and known-bad convictions), `generators/tests.rs` (the generator liveness census), `compactness/tests.rs` (the skyline-versus-min-lifted size envelope), `snapshots.rs` (insta inline goldens), `asymptotics.rs` (the documented-asymptotics liveness pins), and `fuelscape_islands.rs` (the island doc-attachment totality pin). The other seven are scaffolding those suites and the rest of the crate's tests consume: `diff_ops.rs` (the `diff_ops!` descriptor table and its roster), `generators.rs` (adversarial deep shapes, arbitrary normal-form strategies, variadic-law families), `optrace.rs` (the seed-derived op-trace strategy and its two appliers), `rng.rs` (one home for seeded randomness), `grow_brute_force.rs` (the DP-independent grow-optimality reference), `compactness.rs` (the envelope check and the alternating-comb builder), and `shape_rows.rs` (the shape-walk row vocabulary). I read all 4192 lines with line numbers at commit 9e5784fb.

The two load-bearing designs hold up. The `diff_ops!` macro makes registration and execution the same act (a descriptor cannot be written without being registered under its own `stringify!`ed name, and every consumer expands the one roster), the tiling pin holds every `Bound` citation in the coverage roster to exactly one side in both directions, and the known-bad descriptors are convicted with two-direction witnesses (convicted where the spellings differ, passing where they coincide). `rng.rs` states the portability argument once, correctly (the workspace resolves `rand_core` 0.6.4, whose `seed_from_u64` is documented value-stable). `grow_brute_force.rs` is a real independent reference: full enumeration, no pruning, exact unchecked cost. The shape-row folds assert the walk vocabulary's own invariants on every population that touches them. `fuelscape_islands.rs` closes the one direction the compiler cannot. Nothing in the partition masks a production failure.

The dominant issues are in the two instruments whose prose has drifted from what they measure. `asymptotics.rs` binds its pins to rustdoc sentences that no longer exist: the Display pin quotes a "summary-merge" sentence removed at b3f09baa, the fold-door messages send a maintainer to `# Complexity` sections that now hold only an `include_str!` of a fuelscape island whose text is authored in `crates/before-fuelscape/src/ops.rs`, and the three `mul_bound_*` pins guard an `Ω(M(|v|))` floor the public contract does not state. Its module doc claims one pin per public door that documents the log factor, but the island contracts state `log k` for eleven fold doors and five are pinned. Its floors are transcribed midpoints between a linear reference the harness recomputes on every run and only prints, and for `Party::join_all` that midpoint sits 2% from either endpoint, so the doc's "a crossing is a class change, never noise" overstates. `compactness.rs` still speaks from before the flag day (faf3cd0a, 2026-07-25) on which the skyline became the stored coding: "the claim its adoption turns on", "today's size", "decision-era", and a `Sample.current_bits` field documented as "Today's live encoded bit length" that is in fact the reconstructed min-lifted reference stream. The keep of the compactness probes is a recorded owner ruling (the 2026-07-24 scaffolding sweep's defended keeps), so only the prose is open here.

The rest is smaller: a dead proptest dimension in the organic driver, a generator census whose doc promises per-arm detection it does not deliver for three `arb_base` arms, an empty exemption roster in the island pin, an island scan blind to the crate's own macro-form includes, a fixture builder whose distinctness claim fails on one draw, and the usual dedupe and vocabulary nits ("door" is used forty-odd times and defined nowhere; "mint" and "honest" appear at four sites). I ran no cargo, just, or test command; every "verified" claim below rests on reading files and read-only git at 9e5784fb.

## Findings

### testing-diff-gen-1: Eleven identity `Matches`/`FsMatches` impls could be two blanket impls
- Where: crates/before/src/testing/diff_ops.rs:110-159 (related: crates/before/src/testing/diff_ops.rs:199-215, crates/before/src/testing/diff_ops.rs:351, crates/before/src/testing/diff_ops.rs:373-374)
- Class / severity / confidence: simplification / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: scaffolding; refutation: confirmed (coherence permits the blanket beside the bespoke impls because each bespoke impl has distinct `Self` and `Reference` types); history: no-rationale-found (the per-type design was justified as type direction against a driver-side assert switch, which a blanket also satisfies)
- Owner-gated: no

Eight `Matches` impls and three `FsMatches` impls are `self == reference` with nothing else; `impl<T: PartialEq> Matches<T> for T` and its `FsMatches` twin express the predicate once. Legibility: eleven impls that carry no information beyond `PartialEq`.

Evidence:

       110	impl Matches<bool> for bool {
       111	    fn matches(&self, reference: &bool) -> bool {
       112	        self == reference
       113	    }
       114	}
       ...
       137	impl Matches<Vec<(Base, u64)>> for Vec<(Base, u64)> {
       138	    fn matches(&self, reference: &Vec<(Base, u64)>) -> bool {
       139	        self == reference
       140	    }
       141	}

Resolution: Replace the identity impls with the two blanket impls and adjust the doc sentence at 64-66 ("Implemented once per result type"). One caveat for the owner: today the identity impls double as a closed roster of admissible result types; the blanket opens it to every `PartialEq` type, so a descriptor returning a stray type would compile. If that roster is wanted, keep the impls and say so in the trait doc. Acceptance: only the bridge and fs-scan impls remain hand-written, or the trait doc states that the impl list is the roster of admissible result types.

### testing-diff-gen-2: Row tuples cross the shape_rows/diff_ops boundary unnamed and in inconsistent field order
- Where: crates/before/src/testing/diff_ops.rs:237-248 (related: crates/before/src/testing/diff_ops.rs:134-159, crates/before/src/testing/diff_ops.rs:519-533, crates/before/src/testing/shape_rows.rs:56-116)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (read the four fold signatures at shape_rows.rs:58, 73, 84-86, 103-105 and the two reordering impls); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found (the row vocabulary landed in one commit, 46eb64f9, with no discussion of its types)
- Owner-gated: no

Four row vocabularies are bare tuples with different field orders: `(Base, u64)` is height-first, `(bool, u64)` is owned-first, `(u64, Vec<Base>)` and `(u64, Base, bool)` are depth-first. The overlay `FsMatches` impl reorders fields to reuse the plateau and region comparisons, and the `clock_shape` tree spelling (521-531) is a ten-line inline transform in a table whose doc says a descriptor "states only the spellings" (285-286). Types-first: newtypes where the name carries semantic weight.

Evidence:

       237	impl FsMatches<Vec<(u64, Base, bool)>> for semantic_oracle::FunctionClock {
       238	    fn fs_matches(&self, reference: &Vec<(u64, Base, bool)>, grid: u32) -> bool {
       239	        let heights: Vec<(Base, u64)> = reference
       240	            .iter()
       241	            .map(|(depth, height, _)| (height.clone(), *depth))
       242	            .collect();
       243	        let owned: Vec<(bool, u64)> = reference
       244	            .iter()
       245	            .map(|(depth, _, owned)| (*owned, *depth))
       246	            .collect();

Resolution: In `shape_rows.rs` define `PlateauRows`, `RegionRows`, `CellRows`, `OverlayRows` as newtypes over the vectors with one field order and `heights()`/`owned()` projections on `OverlayRows`; add `oracle_overlay(&oracle::Clock) -> OverlayRows` so the `clock_shape` tree spelling becomes one call. Acceptance: no bare row tuple type in `diff_ops.rs`; each of `clock_shape_matches_the_oracle`'s three spellings is one expression.

### testing-diff-gen-3: `party_shape` descriptor's doc claims the anonymous id is in its population; no driver feeds it
- Where: crates/before/src/testing/diff_ops.rs:553-554 (related: crates/before/src/testing/diff_ops/tests.rs:258-266, crates/before/src/testing/diff_ops/tests.rs:400-402, crates/before/src/party.rs:643-654)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (tests.rs:263 `fn $driver(a in arb_oracle_party_nonempty())` for the `(party)` arm; tests.rs:401 `assert_diff_ops!(super::$group, $env.p[0]);` from live clocks; party.rs:646-647 "never a publicly constructible value"); executed: no
- Seen by: adequacy; refutation: confirmed; history: no-rationale-found (the sentence landed at 46eb64f9 after efc0f5f6 had already restricted the `(party)` driver to non-empty ids)
- Owner-gated: no

Every test's doc comment must state its population accurately. `PARTY_SOLO`'s arbitrary driver draws `arb_oracle_party_nonempty()` and its organic arm feeds live clocks' ids, so `Leaf(false)` never reaches `party_shape_matches_the_oracle`; the anonymous `Party` is also `pub(crate)`-only. The sentence describes neither the drivers' population nor a public state.

Evidence:

       553	    /// as a tree, against the geometric lift. The anonymous id is in the
       554	    /// population: its walk is the single unowned whole-interval region.

Resolution: Drop the sentence; or, if the anonymous walk is meant to be covered, register a `party_shape` spelling in a group whose driver admits the anonymous id and say the value is crate-internal. Acceptance: the descriptor's doc describes only inputs its drivers produce.

### testing-diff-gen-4: `BespokeGenre::GENRES` and `name()` restate the variant list twice
- Where: crates/before/src/testing/diff_ops.rs:848-870 (related: crates/before/src/testing/diff_ops/tests.rs:125-135)
- Class / severity / confidence: simplification / nit / high
- Provenance: assessed (read); executed: no
- Seen by: scaffolding, structure-prose; refutation: reframed (a bare `const ALL: &[BespokeGenre]` would drop the exhaustive-match totality nudge the current shape has; `strum` is not a dependency); history: deliberate-and-holds (the pairing is rationalized inline, but the rationale is internal to the pair)
- Owner-gated: no

The string roster and the exhaustive `name()` match both spell the enum's variants; `#[derive(Debug)]` already yields the name. The inline rationale (849-851) explains how the two stay in sync, not why two exist. The replacement must keep the compile-time totality the exhaustive `match` provides: a slice that forgets a new variant compiles, and the census (tests.rs:130-135) would skip it.

Evidence:

       849	    /// Every genre name, for the inhabitation census; the exhaustive match
       850	    /// in [`name`](BespokeGenre::name) beside this list keeps the two in
       851	    /// one diff when the vocabulary changes.
       852	    pub(crate) const GENRES: &'static [&'static str] = &[
       853	        "TraceLockstep",

Resolution: Derive the roster from the exhaustive match (a `const ALL` built by a `match` over every variant that returns the slice, so a new variant is a compile error) and format names with `{genre:?}`; or add `strum::EnumIter` (prefer a dependency over hand-rolling). Acceptance: no string list parallel to the enum; adding a variant without touching the roster fails to compile.

### testing-diff-gen-5: Prose sweep: em-dashes in line comments and one assert message, "mint", "honest", temporal and anticipatory phrasing
- Where: crates/before/src/testing/diff_ops/tests.rs:86-87 (related: crates/before/src/testing/diff_ops/tests.rs:193, 196, 408, 475-476, 531, 544; crates/before/src/testing/diff_ops.rs:27, 34; crates/before/src/testing/generators.rs:228; crates/before/src/testing/grow_brute_force.rs:55; crates/before/src/testing/asymptotics.rs:46-47, 78, 449; crates/before/src/testing/compactness.rs:43; crates/before/src/testing/fuelscape_islands.rs:19)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep of `^\s*//[^/!].*—` over the thirteen files and grep -w for `mint|honest|honestly`; every cited line read); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: contradicts-hard-rule (writing-style.md:149-152 dashes in comments, :170 "mint", :326-330 moralized code; CLAUDE.md "colons (or semicolons) over em-dashes in log messages and comments")
- Owner-gated: no

The owner's writing rules name each of these: the spaced double-hyphen (or a colon) is the dash of code comments; "mint" is never written for constructing a value; "real"/"genuine"/"honest" are replaced by the property that holds; prose speaks in the present tense without anticipating a named future change. Em-dashes in `//` comments: diff_ops/tests.rs:193, 196, 408, 475, 476; generators.rs:228; grow_brute_force.rs:55; asymptotics.rs:78, 449; and inside the assert message at diff_ops/tests.rs:87. "mint a synonym": diff_ops/tests.rs:531, 544. "states honestly": diff_ops.rs:34. "the honest constant": compactness.rs:43. "Currently empty": fuelscape_islands.rs:19. "When the render-merge cure lands": asymptotics.rs:46-47. "where a body per population was three independent spellings" (diff_ops.rs:27) is a hand count of a replaced design.

Evidence:

        86	            "{name}: derived from the descriptor table AND rostered as \
        87	             bespoke — the tiling sides must stay disjoint; remove one"
       ...
       531	// mint a synonym per signature to appease the lint.

    diff_ops.rs:
        34	//! The table covers what a value-returning descriptor states honestly. The

    compactness.rs:
        43	/// case and re-pin the honest constant.

    asymptotics.rs:
        46	/// cost that grows faster than the operand" sentence describes. When the
        47	/// render-merge cure lands this pin reads red, and the rustdoc and this

Resolution: Mechanical sweep: colons or semicolons for the listed em-dashes (`///` and `//!` doc comments keep theirs); "coin" or "add" for "mint"; "states" for "states honestly" and "the measured constant" for "the honest constant"; "a cure that removes the merge flips this pin" for the anticipatory sentence; drop "Currently"; drop the "three independent spellings" count. Acceptance: `grep -nE '^\s*//[^/!].*—'` over the partition returns nothing; no `mint`, `honest`, or `Currently` in the partition.

### testing-diff-gen-6: The registration totality pin describes the lint's failure class, not its own; its source scan imposes a layout convention on the macro and duplicates `laws/tests.rs`
- Where: crates/before/src/testing/diff_ops/tests.rs:138-179 (related: crates/before/src/testing/diff_ops.rs:297-305, crates/before/src/testing/diff_ops.rs:960-964, crates/before/src/laws/tests.rs:28-61, crates/before/src/lib.rs:453-454, justfile:127)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (both scan functions read; they differ only in prefix string and path; lib.rs:453-454 `#[cfg(test)] mod testing;` is private; justfile:127 runs `cargo clippy --workspace --all-targets --all-features -- -D warnings`); the lint's behavior on an unrostered `pub(crate) static` is assessed, not executed; executed: no
- Seen by: scaffolding, structure-prose; refutation: reframed (the pin's stated purpose is already a gate lint error by reading; what the pin alone holds is roster totality against an ad-hoc-driven group, which its doc does not say); history: no-rationale-found (the scan idiom was copied from laws/tests.rs, whose rationale concerns exported `pub static` groups the dead-code lint cannot see)
- Owner-gated: no (retiring the pin would be; the recommended step keeps it)

The pin's doc says it closes "a group static missing from the roster, which nothing would run". In a private `cfg(test)` module an unreferenced `pub(crate) static` is dead code, and the gate's clippy leg runs with `-D warnings`, so that failure class is already an error by reading. What the pin alone holds is stricter: a group that some bespoke test drives directly without rostering (referenced, so not dead) still fails the set equality. That unique catch is not in the doc. To stay blind to the macro's own definition the scan also obliges the `diff_ops!` matcher and transcriber to keep attributes and declaration on one line (diff_ops.rs:302-305), a stabilization convention that exists only to serve the instrument, and the scan body is `laws/tests.rs:36-61` with a different prefix and path.

Evidence:

       143	/// group is executed by construction and needs no per-consumer pin. The one
       144	/// door that leaves open is a group static missing from the roster, which
       145	/// nothing would run; this pin closes it against a source scan of the
       ...
       152	    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/testing/diff_ops.rs");
       153	    let text =
       154	        fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));

    diff_ops.rs:
       302	    // In both the matcher and the transcriber, the attributes and the
       303	    // declaration share a line: the registration totality pin's source scan
       304	    // reads any line starting `pub(crate) static` as a group declaration,
       305	    // and must see the invocations' headers only, never this definition.

Resolution: Keep the pin and restate its doc to name what it alone catches (a declared group driven by a bespoke test but absent from the roster; the unreferenced case is the gate's `dead_code` error). Extract one shared `testing` helper `declared_statics(path, prefix) -> BTreeSet<String>` used by both this pin and `every_law_group_is_registered`. If the owner prefers retirement, first execute the construction below so the replacement demonstrably catches what the pin caught. Acceptance: one scan body in the crate; the pin's doc names the ad-hoc-driven case; the layout comment at diff_ops.rs:302-305 either stays with the shared helper cited or goes with the scan.

Construction: In diff_ops.rs add, without touching `for_each_diff_group!`, `diff_ops! { pub(crate) static ORPHAN: (a: version); fn orphan_is_empty { prod: a.is_empty(), tree: a.is_empty() } }`; run `cargo clippy -p before --all-targets --all-features -- -D warnings`. Expected: `error: static ORPHAN is never used`. Then reference `ORPHAN` from a bespoke test body: the lint is silent and only the pin's set equality reads red. That second step is the pin's unique catch.

### testing-diff-gen-7: The organic driver draws a third version (`k`, `vc`) that no drive arm reads
- Where: crates/before/src/testing/diff_ops/tests.rs:371-382 (related: crates/before/src/testing/diff_ops/tests.rs:445-466)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -n 'v\[2\]' diff_ops/tests.rs` returns nothing; every `organic_drive!` arm at 391-426 reads only `$env.v[0]` and `$env.v[1]`; `k` at 449, `vc` at 456, stored at 461 and 464); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed; history: no-rationale-found (d1c27ce7 landed the three-slot array and the `k` pick with no `v[2]` read; no later commit added one)
- Owner-gated: no

`Organic.v` is documented as "Three versions from the trace" and the proptest draws `k in 0usize..64` to pick `vc`, but every arm indexes `v[0]` and `v[1]` only. The third pick widens the proptest case and shrink space for no coverage, and the struct doc claims a population the drivers do not exercise.

Evidence:

       372	    /// Three versions from the trace, causally related.
       373	    v: [&'a oracle::Version; 3],
       ...
       449	        k in 0usize..64,
       ...
       456	        let (_, vc) = cs[k % len].trees();

Resolution: Drop `k` and `vc`, make `v: [&'a oracle::Version; 2]`, and fix the field doc; or, if a three-version group was intended (a transitivity or `span_all` descriptor), add it so the pick is read. Acceptance: every field of `Organic` is read by at least one `organic_drive!` arm.

### testing-diff-gen-8: `check_version_pair` and `check_party_pair` are one generic function written twice
- Where: crates/before/src/testing/diff_ops/tests.rs:529-553
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (both bodies read; `assert_diff_ops!` iterates `(name, check)` and calls `check(a, b)`, so `fn check_pair<A, B>(group: &[DiffOp<fn(&A, &B) -> bool>], a: &A, b: &B)` type-checks); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

Both helpers exist only to give `assert_diff_ops!` (which uses `prop_assert!`) a `Result<(), TestCaseError>` frame; they differ only in carrier type and each carries the same two-line comment and `#[allow(clippy::type_complexity)]`.

Evidence:

       529	/// Run a version-pair group through the drivers' assertion vehicle.
       530	// The group's element type is the signature it carries; naming it would
       531	// mint a synonym per signature to appease the lint.
       532	#[allow(clippy::type_complexity)]
       533	fn check_version_pair(
       ...
       542	/// Run an id-pair group through the drivers' assertion vehicle.
       543	// The group's element type is the signature it carries; naming it would
       544	// mint a synonym per signature to appease the lint.

Resolution: One generic `check_pair<A, B>`; both conviction tests call it. Acceptance: one pair-check helper in the file.

### testing-diff-gen-9: The conviction witnesses reach 2 of 11 `Matches` and 1 of 9 `FsMatches` comparisons; the two non-trivial party comparisons have no known-bad
- Where: crates/before/src/testing/diff_ops/tests.rs:555-566 (related: crates/before/src/testing/diff_ops.rs:99-108, crates/before/src/testing/diff_ops.rs:189-197, crates/before/src/testing/diff_ops/tests.rs:480-527, crates/before/src/testing/diff_ops.rs:627-637)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (counted the impls at diff_ops.rs:77-159 and 179-261; the three known-bad groups at tests.rs:480-527 produce `Version`, `Option<Party>`, and `Event` results only); executed: no
- Seen by: adequacy; refutation: confirmed, severity lowered (seven unwitnessed `Matches` are `self == reference` on plain types with no drift path; the four row `FsMatches` delegate to the Event/Id scans; the live gaps are `Matches<oracle::Party> for Party` and `FsMatches<oracle::Party> for Id`); history: no-rationale-found (the design intent at 20c87ade was to catch a blinded comparison; later impls landed with no coverage decision)
- Owner-gated: no

The doc promises that a comparison "that had gone blind" is caught by the conviction witnesses, but the witnesses exercise only `Matches<oracle::Version> for Version`, `Matches<oracle::Party> for Option<Party>` (through `without`), and `FsMatches<oracle::Version> for Event`. The two comparisons with real bodies and no witness are `Matches<oracle::Party> for Party` (bridge both ways, used by `party_disjoint_join_matches_the_oracle`) and `FsMatches<oracle::Party> for Id` (the `id_order` scan, used by the same descriptor's fs leg and by `party_shape` through `party_from_rows`). `cargo-mutants` does not reach `cfg(test)` code, so nothing else would notice either returning `true`.

Evidence:

       558	/// Two directions, and the second is what makes the first mean anything. A
       559	/// comparison that had gone blind — a `Matches` implementation that always
       560	/// agrees, a bridge that erases the result — would let the wrong descriptor
       561	/// through, which the conviction witnesses catch. A comparison stuck at

Resolution: Add one known-bad `(disjoint_party, disjoint_party)` group whose production leg forgets the join (`prod: a`, `tree: { a.join(b)...; a }`) and whose fs leg lifts only `a`, with the existing two-direction pattern; state in the conviction tests' doc which comparison impls the witnesses reach. Acceptance: replacing the body of `Matches<oracle::Party> for Party` or `FsMatches<oracle::Party> for Id` with `true` makes a conviction test fail.

Construction: Edit diff_ops.rs:103-107 to `fn matches(&self, _: &oracle::Party) -> bool { true }` and run the `diff_ops` tests: every driver stays green and neither conviction test notices, because no known-bad descriptor produces a `Party` result.

### testing-diff-gen-10: The generators module doc enumerates contents and has rotted; "All trees" is contradicted by `deep_left_spine_party`
- Where: crates/before/src/testing/generators.rs:1-24 (related: crates/before/src/testing/generators.rs:263-283, crates/before/src/testing/generators.rs:368-495)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (line 1 says "in two families"; the third section header "variadic-law families" is at 368; `deep_left_spine_party` at 274-283 builds packed bits with a flat loop); executed: no
- Seen by: structure-prose; refutation: confirmed; history: deliberate-but-expired ("in two families" predates 6ed90b93's variadic section; "All trees" predates d8cd87d7d's bit-built spine)
- Owner-gated: no

Prose speaks in the present tense and does not hand-maintain enumerations of module contents. The doc lists functions by name and says "two families"; the file has three sections and omits `shape_version_wide`, `bushy_expand_party`, `arb_shape`, `arb_oracle_party_nonempty`, and the four family strategies. "All trees are built via the oracle's normalizing constructors" is false for the bit-built deep spine.

Evidence:

         1	//! Input generators for the property tests, in two families:
       ...
        18	//! All trees are built via the oracle's normalizing constructors (`O(1)` per
        19	//! node), then lowered to the impl with [`super::bridge`].

Resolution: State the structure, not the roster: three sections named as the file's headers name them, each with a one-sentence purpose; qualify "All trees" with the bit-built exception or drop "All". Acceptance: the module doc names sections, not functions, and every sentence in it is true of the file.

### testing-diff-gen-11: `shape_version` and `shape_version_wide` duplicate the spine loop; three `unreachable!("handled above")` arms; `debug_assert!` in test-only code
- Where: crates/before/src/testing/generators.rs:127-204 (related: crates/before/src/testing/generators.rs:72-94, crates/before/src/testing/generators.rs:244-261)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (the lean match at 135-141 and 195-201 is identical; `unreachable!("handled above")` at 140, 200, 257; `debug_assert!` at 166; traced `scale == 0, at_tip == false`: `mid = 0`, the loop never runs, `leaf(0)` returns without the wide leaf); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The two version builders run the same match over `k in 1..=scale` and differ only in how a leaf's base is chosen; `bushy_version_with` (79-94) already models the fix with a `leaf_base` closure. The early-return-for-Bushy structure leaves an `unreachable!` arm in three matches. The precondition guard is a `debug_assert!` in code that compiles only under `cfg(test)`; a plain `assert!` costs the same and holds under a release-profile test run too.

Evidence:

       135	        t = match shape {
       136	            Shape::LeftSpine => V::node(0u64, t, leaf),
       137	            Shape::RightSpine => V::node(0u64, leaf, t),
       138	            Shape::Zigzag if k % 2 == 0 => V::node(0u64, t, leaf),
       139	            Shape::Zigzag => V::node(0u64, leaf, t),
       140	            Shape::Bushy => unreachable!("handled above"),
       141	        };
       ...
       166	    debug_assert!(scale >= 1, "a scale-0 shape has nowhere to put the leaf");

Resolution: `fn shape_version_with(shape, scale, leaf_base: &impl Fn(u64) -> Base)` with `shape_version` as the identity instance and `shape_version_wide` choosing the wide index up front; select the lean with one closure returned from a single match on `shape` so Bushy is handled once (the party twin at 244-261 can share it). Replace `debug_assert!` with `assert!`. Acceptance: one spine loop for versions; no `unreachable!("handled above")` remains; `scale == 0` panics for the wide builder.

### testing-diff-gen-12: `shape_version_wide`'s distinctness claim fails at `wide == 1`: the raised leaf collapses against its sibling
- Where: crates/before/src/testing/generators.rs:153-158 (related: crates/before/src/testing/generators.rs:166-203, crates/before/src/oracle/version.rs:80-89, crates/before/src/version/skyline/fill/tests.rs:1626-1636)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified by tracing (oracle/version.rs:85-87 collapses equal sibling leaves; `LeftSpine, scale 8, wide 1, at_tip`: `t = leaf(1)`, `k = 1` builds `node(0, leaf(1), leaf(1))` which normalizes to `Leaf(1)`, so seven nodes remain and depth is 7; `Bushy, scale 8` (nine leaves): `bushy_version_with` splits 4/5 then 2/2 and 2/3, so leaves 0/1 and 4/5 are siblings, and `wide_at` is 0 at the tip and `ceil(8/2) = 4` interior, each colliding with its sibling when `wide == 1`); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The doc says raising one counter by `wide` "keeps every leaf base distinct, so the shape and size survive normalization unchanged". `arb_base`'s dense arm draws `1` with probability about 1/13, and on that draw the raised leaf equals its sibling: every spine comes out one level shallower than `scale`, and the bushy shape loses a leaf pair. The consumer (fill/tests.rs:1612-1614) is documented as sweeping "every deep shape at swept depths 8..=128"; on that draw the depth premise is off by one. The differential verdict stays valid (1 is not wide), so this is a false docstring and a slightly weaker family, not a wrong verdict.

Evidence:

       157	/// Raising one distinct counter by `wide` keeps every leaf base distinct, so
       158	/// the shape and size survive normalization unchanged.

Resolution: Raise the chosen leaf by `wide + scale + 1` (plus its counter), which exceeds every other base for any `wide >= 0`, and pin the claim: assert `ev_depth(&to_oracle_version(&result))` equals the depth `shape_version(shape, scale)` produces before returning. Acceptance: a proptest over `arb_shape() × 1..=64 × arb_base() × bool` asserts the depth identity.

Construction: `assert_eq!(ev_depth(&to_oracle_version(&shape_version_wide(Shape::LeftSpine, 8, &Base::from(1u8), true))), 8)` fails (reads 7); the same at `wide = 2` passes.

### testing-diff-gen-13: `skip_stress_pair` cites a "bounded lazy-skip" that no longer exists in either `is_disjoint` kernel
- Where: crates/before/src/testing/generators.rs:206-208 (related: crates/before/src/party/ops/compare.rs:59-101)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -rn -i lazy` over `party/` and `idbits.rs` finds only `forks.rs`'s lazy iterator; `git grep -i lazy-skip 7b11b3ead -- crates/before/src/party` shows the phrase last in party/tests.rs at that commit; compare.rs:59-101 skips a dominated `b` subtree with `b.skip()` at every settled level, unbounded); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: deliberate-but-expired (the mechanism was real at the crate's origin; the production doc dropped it at 7b11b3ead and the generators doc was never re-denominated)
- Owner-gated: no

Prose may not refer to code that no longer exists. The shape's mechanical claim (Θ(scale) skips of small subtrees, walk to completion) still holds against the current lockstep walk; only the name is a ghost.

Evidence:

       206	/// Build a disjoint "staircase" id pair `(a, b)` that drives the bounded
       207	/// lazy-skip in `is_disjoint` to its worst case: `Θ(scale)` distinct skips,
       208	/// each over a small subtree.

Resolution: Re-denominate: "drives the per-level dominated-subtree skip (`IdReader::skip`) in the lockstep disjointness walk to Θ(scale) skips of O(1) subtrees each". Acceptance: `grep -i 'lazy-skip' crates/before/src/testing` returns nothing.

### testing-diff-gen-14: The generator census pins classes, not arms: three `arb_base` arms have no discriminating class, and the doc claims per-arm detection
- Where: crates/before/src/testing/generators/tests.rs:78-93 (related: crates/before/src/testing/generators.rs:304-332, crates/before/src/testing/generators.rs:320-321, crates/before/src/testing/generators/tests.rs:125-137)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified by tracing each `arb_base` arm against `census_of` (tests.rs:58-63): arm 1 (`0..6`), arm 2 (`any::<u64>()`), and arm 3 (`u64::MAX - 4..=u64::MAX`) reach no class; arm 5 (`(1 << k) + 1`, `k < 96`) reaches only `wide` (for `k >= 64`), which arms 4, 6, and 7 also feed; arm 4's `u128 + u64::MAX` reaches `three_limb` only when the `u128` draw exceeds `2^128 - 2^64`, so `three_limb` and `beyond_narrow` are arm 7's alone; executed: no
- Seen by: adequacy; refutation: confirmed; history: no-rationale-found for the arm gap (28f6981e names exactly the seven classes and never the shifted-power, `any::<u64>`, or straddle arms); the fraction-of-measured floors are deliberate with an inline rationale
- Owner-gated: no for adding classes; yes for changing how floors are derived (the "a quarter to a half of measured" design is stated at tests.rs:89-93)

A liveness census earns its place by naming what dies red. The doc promises "a dead arm or depth regression reads red here", and generators.rs:320-321 says the census "pins each of these classes alive" after describing the dense range and the `u64::MAX` straddle, yet neither has a floor, `any::<u64>()` has none, and the shifted-power arm can die under the other three wide arms' mass with every counter above its floor. The module doc (generators.rs:287-291) calls this generator "the natural home for the path-sum-overflow regression class"; the straddle arm that carries it is unwitnessed.

Evidence:

        78	/// Every named input class stays under generator mass: sampling the arbitrary
        79	/// strategies under the committed seed meets a positive floor per class, so a
        80	/// dead arm or depth regression reads red here instead of nowhere.

    generators.rs:
       319	/// reads on wide top digits) are inside every random differential's sampled
       320	/// universe, not beyond it. [`tests::generator_classes_stay_under_mass`]
       321	/// pins each of these classes alive.

Resolution: Add classes with a unique feeding arm: `near_max` (`u64::MAX - 4 <= value <= u64::MAX`), `power_plus_one` (`value - 1` a power of two with exponent in `6..96`, disjoint from the dense arm), and `machine_word` (`6 <= value <= u64::MAX - 5`), each with a floor and a comment naming the arm it witnesses; re-state the generators.rs sentence to name exactly the classes the census pins. Owner-gated alternative: derive each floor from the arm's `prop_oneof` weight (weight `w` of 13 per base, at least one base per tree, pinned at half the expectation) and state the derivation beside the number. Acceptance: narrowing arm 5 to `0u32..64` or deleting arm 3 makes the census fail; each floor's comment names the arm it derives from.

Construction: Change generators.rs:328 to `1 => (0u32..64).prop_map(...)` (values at most `2^63 + 1`, never `> 2^64`): `wide` stays far above 650 on arms 4, 6, 7 and every floor holds. Or delete line 326 (the straddle arm): no counter moves.

### testing-diff-gen-15: `op_strategy` draws indices from an unnamed `0..8`, so members beyond the eighth never act; the strategy is undocumented and `Sync`'s doc omits the self-pair no-op
- Where: crates/before/src/testing/optrace.rs:37-46 (related: crates/before/src/testing/optrace.rs:17-19, crates/before/src/testing/optrace.rs:31-32, crates/before/src/testing/optrace.rs:56, crates/before/src/testing/optrace.rs:95-104, crates/before/src/testing/compactness/tests.rs:44)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (all six arms draw `0usize..8`; `i % n` with `i < 8 <= n` is `i`, so indices `>= 8` are selected only after a `Join` shifts them down; `world_strategy` draws up to 29 ops, `world_strategy_up_to(120)` up to 119); executed: no
- Seen by: adequacy, structure-prose; refutation: confirmed; history: no-rationale-found (the range is original to eecf9229; 28f6981e derived `GRID_N` from `MAX_TRACE_OPS` without touching the actor cap)
- Owner-gated: no

Every bound proved over organic populations is a bound over the universe the generator reaches. A hidden cap on which members act shapes that universe (later forks stay at fork-time state) and belongs in the doc or the constant vocabulary beside `MAX_TRACE_OPS`. `op_strategy` is the one undocumented function in an otherwise fully documented module and repeats the literal six times. `Op::Sync`'s doc ("Reconcile `i` and `j`") does not say `i == j` is a no-op in both appliers (97, 144).

Evidence:

        37	fn op_strategy() -> impl Strategy<Value = Op> {
        38	    prop_oneof![
        39	        (0usize..8).prop_map(Op::Tick),
        40	        (0usize..8, 0u8..=6).prop_map(|(i, n)| Op::Ticks(i, n)),
        41	        (0usize..8).prop_map(Op::Fork),
       ...
        17	/// One step of a seed-derived execution. Indices are reduced modulo the live
        18	/// population, so any index is valid and every member descends from one seed via

Resolution: Name the cap (`const ACTOR_INDICES: Range<usize> = 0..8`) with its rationale, or draw from `0..MAX_TRACE_OPS` so any live member can act; document `op_strategy`; append "a self-pair is a no-op" to `Sync`'s doc. Acceptance: the module doc states which members can be actors; the index range is a named constant; `Sync`'s doc matches both appliers.

Construction: A trace of nine `Fork(0)` steps yields ten members; for every later op, `i % n` with `i < 8` and `n >= 9` never selects index 8 or 9, so those clocks stay at fork-time state until a `Join` shifts them below 8.

### testing-diff-gen-16: The op applier is spelled twice in `optrace.rs` and twice more elsewhere; the module doc's op list omits `Ticks` at three sites
- Where: crates/before/src/testing/optrace.rs:71-165 (related: crates/before/src/testing/optrace.rs:1-2, crates/before/src/clock/tests.rs:208-265, crates/before/src/testing/semantic_oracle/tests.rs:59-153, crates/before/src/version/tests.rs:663-672, crates/before/src/testing/compactness/tests.rs:38, crates/before/src/testing/diff_ops/tests.rs:435)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (both bodies read side by side; `grep -rn 'Op::Tick('` lists the two external copies and the tick-count mirror; optrace.rs:2 lists "fork/tick/send/sync/join" and `Ticks` exists at 26); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed, severity lowered (the `replay` copy threads a third carrier with an rng and interposes disjointness asserts, so it has a stated reason to differ); history: no-rationale-found (8f253e4e added `Ticks` to the enum and every applier without editing the list)
- Owner-gated: no

`run` and `step_impl` are the same control flow: index reduction, `Fork` push, `Send` as send-then-receive, `Sync` via `split_at_mut(hi)`, `Join` as remove-and-reindex with the `i2` adjustment. The doc asks the reader to keep them in lockstep by hand. `clock/tests.rs:208-265` (`master_differential`) is a third hand copy. The module doc's op inventory is a hand-maintained list that has already drifted; the refutation pass found the same rotted list at compactness/tests.rs:38 ("fork/tick/send/sync/join history") and diff_ops/tests.rs:435 ("fork/tick/join/sync schedules").

Evidence:

         1	//! The seed-derived op-trace generator: a proptest strategy that produces a sequence of
         2	//! fork/tick/send/sync/join steps.
       ...
       122	/// Apply one op to an impl population, mirroring [`run`] for the oracle (same index
       123	/// arithmetic, so traces line up). Used by tests that drive the impl alone.
       124	pub(crate) fn step_impl(imp: &mut Vec<Clock>, op: &Op) {

Resolution: A small `pub(crate) trait Member` (`tick`, `ticks`, `fork`, `send`/`recv`, `sync`, `join`) implemented for `oracle::Clock` (with `ticks` as the literal loop and its comment) and `Clock`; one `step<M: Member>(pop: &mut Vec<M>, op: &Op)`; `run` as the fold from `vec![M::seed()]`; `master_differential` calls `step` per population. `replay` keeps its copy with a one-line reason at the site. State the op inventory as "the variants of [`Op`]" at all three sites. Acceptance: one applier body in optrace.rs; adding an `Op` variant fails to compile in exactly one place; no prose lists the op variants.

### testing-diff-gen-17: `compactness.rs` speaks from before the flag day: "today's"/"decision-era"/"adoption turns on", a `current_bits` doc that names the wrong quantity, and measurement-sweep readings in prose
- Where: crates/before/src/testing/compactness.rs:1-9 (related: crates/before/src/testing/compactness.rs:31-57, crates/before/src/testing/compactness.rs:59-68, crates/before/src/testing/compactness.rs:78-84, crates/before/src/testing/compactness.rs:91, crates/before/src/testing/compactness/tests.rs:1-10, crates/before/src/testing/compactness/tests.rs:30-32, 41-42, 58-59, 70-72, crates/before/src/meter/tier2.rs:1-19)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git log --date=short -- crates/before/src/testing/compactness.rs`: f2d0011b 2026-07-23 lands the suite, faf3cd0a 2026-07-25 is the flag day; `current_bits = packed.len()` where `packed = packed_bits_of(&to_oracle_version(version))` at 81-84 and `bridge::packed_bits_of` (bridge.rs:62-68) emits the min-lifted stream; tier2/tests.rs:145 asserts "the stored coding is Tier 2 itself"; tests.rs:44 runs `world_strategy_up_to(120)` against prose "trace lengths up to 400"; `arb_comb_params` tops out at 200/256 against prose "2048-bit teeth and 1024 pairs"; the one committed tightness point is `comb(1024, 1024)` at tests.rs:93); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: already-known for the dissolution branch (the compactness probes are a recorded defended keep, `.agent-notes/2026-07-22-before-adversarial-resource-amplification/before-adversarial-resource-amplification.md:1756-1760`, reaffirmed at faf3cd0a, so that branch is dropped); contradicts-hard-rule for the prose (root AGENTS.md: nothing refers to code that no longer exists; CLAUDE.md Principle 5: dated measurement reports are re-denominated or excised when the code they cite is gone; the d2a9d04e sweep removed calendar dates only)
- Owner-gated: no (a prose correction toward the code; the keep is already ruled, and where the 2x claim should live is an open question below)

The skyline is the stored coding. What the instrument measures today is the stored skyline stream's bit length against the min-lifted packed preorder stream re-derived through the oracle lowering, a fixed reference; the measurement is right and its labels are wrong. `Sample.current_bits` is documented as "Today's live encoded bit length" and is the reference stream's length. The module frames itself as pre-decision evidence, the two `f64` constants carry hand-counted sweep provenance (sample counts, family maxima) that the committed tests do not reproduce, and compactness/tests.rs opens with a "Ratio record of the measurement sweep" quoting 1.9966, 1.9633, 1.50, 30-40%, 0.75, 0.7502, of which only the 1024×1024 comb figures are asserted (tests.rs:94-97). The sibling asymptotics module had exactly this class of reading excised at 4797009a under the rule that a reading not pinned by an adjacent assertion is falsified silently.

Evidence:

         3	//! The Tier 2 coding stores preorder topology plus delta-coded absolute leaf
         4	//! values ([`crate::meter::tier2`]); the claim its adoption turns on is that
         5	//! its coded size never exceeds ~2x today's size plus O(1) bits per node.
       ...
        34	/// Provenance: measured (~13k samples: 5000 arbitrary trees,
        35	/// ~7800 organic-history versions at trace lengths up to 400, the
       ...
        64	    /// Today's live encoded bit length of the same version.
        65	    pub(crate) current_bits: u64,
       ...
        78	    // The decision-era "current" coding is the min-lifted packed preorder
        79	    // stream (one gamma-coded base per node), re-derived through the
        80	    // oracle lowering; the stored coding is Tier 2 itself.
       ...
        91	         (ratio {ratio:.4}): decision-critical, pin this witness: {tier2:?}",

    compactness/tests.rs:
         4	//! Ratio record of the measurement sweep (thousands of samples
         5	//! per random family; the deterministic grids in full): the global maximum
         6	//! ratio is 1.9966 at the alternating comb with 2048-bit teeth and 1024

Resolution: Rewrite the module doc, both constant docs, `Sample`, `check_sample`'s comment and messages, and compactness/tests.rs:1-10 in the present tense: the skyline coding's bit length is at most twice the min-lifted packed preorder reference (plus 0 bits per node) on every family; the reference is the oracle lowering's packed stream; the comb is the tightness witness. Rename `current_bits` to `reference_bits`. Keep the derivation (each stored base charged at most twice, O(1) bits per gamma merge) and the committed tightness test; move the sweep record and family maxima out of prose (they live in f2d0011b and the design note). The note's own present-tense sentence ("skyline ≤ 2× the packed-era coding outright on every sample", §11) is the ready replacement. Acceptance: no occurrence of "today", "current coding", "adoption", "decision-critical", "decision-era", "Provenance: measured", or "Ratio record" in compactness.rs or compactness/tests.rs; every figure remaining in prose is one a committed assertion pins.

### testing-diff-gen-18: `comb`'s doc says its closed-form sizes are pinned by this module's tests; the node count is asserted nowhere and the bit formula at one point only
- Where: crates/before/src/testing/compactness.rs:121-123 (related: crates/before/src/testing/compactness.rs:137-138, crates/before/src/testing/compactness/tests.rs:88-101)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`grep -n nodes compactness/tests.rs` hits only the header prose at line 8; re-derived the bit formula: pair subtree `1 + gamma(0) + 1 + gamma(0) + 1 + gamma(2^m - 1) = 2m + 6` bits, spine `2(pairs - 1)`, total `pairs(2m + 8) - 2 = 2,105,342` at (1024, 1024), matching tests.rs:95); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found (false for the node count from f2d0011b onward)
- Owner-gated: no

Doc accuracy ("pinned by this module's tests" is false for nodes), and any quantity computable two ways gets a committed test comparing them: the closed forms and the measured `Tier2Size` are two computations of the same numbers, and the builder already computes the bit closed form as its capacity (line 138).

Evidence:

       121	/// both the size envelope and the charge bound. Exact sizes (pinned by this
       122	/// module's tests): `4 * pairs - 1` nodes,
       123	/// `pairs * (2 * m_bits + 8) - 2` bits today. Strict normal form (every

Resolution: In `alternating_combs_hold_the_envelope` (or in `comb` itself) assert `sample.tier2.nodes == 4 * pairs as u64 - 1` and `bits.len() == (pairs * pair_bits - 2) as u64` for every sampled parameter pair; drop "today" from the sentence. Acceptance: both closed forms are asserted on every `arb_comb_params` case; the doc names the test that pins them.

Construction: Change the spine loop at line 141 to `0..pairs` (one extra spine node): the node count becomes `4 * pairs`, `check_sample` still passes on every `arb_comb_params` case, and only the exact `current_bits` at tests.rs:95 catches it at one point while the doc's node formula reads wrong with no test naming it.

### testing-diff-gen-19: `comb` hand-emits the min-lifted stream and self-checks it; the oracle path every other generator uses gives the same bits by construction
- Where: crates/before/src/testing/compactness.rs:137-165 (related: crates/before/src/testing/bridge.rs:47-60, crates/before/src/testing/bridge.rs:83-89, crates/before/src/oracle/version.rs:80-88)
- Class / severity / confidence: simplification / nit / medium
- Provenance: assessed (derived: `pair = node(0, leaf(0), leaf(M))` survives normalization since the minimum base is 0 and the sibling leaves differ for `M >= 1`; `t = pair; for _ in 1..pairs { t = node(0, t, pair) }` emits under `emit_ev` exactly `pairs - 1` spine headers `1.gamma(0)` then `pairs` pair subtrees, the hand-built order; byte identity not executed); executed: no
- Seen by: structure-prose; refutation: confirmed; history: deliberate-and-holds (f2d0011b: "built directly as packed bits with its exact sizes self-checked by round-trip"; the inline comment at 155-156 states what, not why)
- Owner-gated: no

A fixture builder that hand-codes bit emission and then recompute-and-compares (`decode(encode(v)) == v`) where the shared bridge gives canonicality by construction; the `# Panics` clause about the self-check disappears with it. Keep the closed-form size assertions (finding 18) as the exactness witness.

Evidence:

       155	    // The comb is hand-built in the min-lifted packed construction
       156	    // language; the transcoding bridge lifts it into the stored coding.
       ...
       161	    // Self-check: the built stream is canonical and round-trips the wire.
       162	    let decoded = Version::decode(version.encode().as_slice())
       163	        .expect("hand-built comb is strict normal form");

Resolution: Build the comb as an `oracle::Version` (a `pair()` helper and the spine fold) and return `from_oracle_version(&t)`; before switching, assert `as_bits()` equality between both forms for a few `(m_bits, pairs)`. Acceptance: `comb` contains no `encode_int`/`encode_bits` call and no self-decode; `comb_ratio_is_tight_against_the_factor_two_ceiling` still pins 4_198_399 / 2_105_342.

### testing-diff-gen-20: `decode` in compactness/tests.rs renames a transcoding call as decoding
- Where: crates/before/src/testing/compactness/tests.rs:21-24 (related: crates/before/src/testing/compactness/tests.rs:12, 76, 79, 83, crates/before/src/meter.rs:116-121)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (meter.rs:116-121 documents `Packed::version` as "transcoding the construction language ... into the skyline coding"); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: deliberate-but-expired (a real `Version::decode` until faf3cd0a replaced the body and kept the name and doc)
- Owner-gated: no

A name and doc that describe work the function no longer does: `decode` is the strict wire validator elsewhere in the crate; this wraps the transcoder. Also the `use crate::meter::registry::Shape;` at line 12 sits above the `proptest` import, outside the file's crate-import group.

Evidence:

        21	/// Decode a meter-generated packed shape into a `Version`.
        22	fn decode(packed: &meter::Packed) -> Version {
        23	    packed.version()
        24	}

Resolution: Call `.version()` at the three sites and delete the wrapper; move the `Shape` import into the crate-imports group. Acceptance: no `fn decode` in compactness/tests.rs.

### testing-diff-gen-21: Gamma code widths are pinned twice: the snapshot table and codec's `gamma_costs`
- Where: crates/before/src/testing/snapshots.rs:45-82 (related: crates/before/src/codec/tests.rs:64-83)
- Class / severity / confidence: simplification / nit / medium
- Provenance: verified (codec/tests.rs:78-82 pins costs of 0, 1, 2, 6, 7; the snapshot pins layout and bit count for those and nine more plus `2^64`); executed: no
- Seen by: scaffolding; refutation: confirmed (with the note that `gamma_costs` carries the closed form `2⌊log2(n+1)⌋ + 1`, which the snapshot lacks); history: no-rationale-found (both original to eecf9229, never discussed together)
- Owner-gated: yes (retiring an instrument)

Two pins of one surface; a deliberate gamma change is re-accepted in two places with two failure messages. The natural survivor beside the layout snapshot is the closed form as a proptest over `n`, which differs in kind from a point pin.

Evidence:

        45	#[test]
        46	fn gamma_bit_layout_table() {
        47	    let small: String = [0u64, 1, 2, 3, 4, 5, 6, 7, 8, 15, 16, 17, 255, 256]

Resolution: Owner's call: keep the snapshot as the layout pin and turn `gamma_costs` into a proptest of `cost(n) == 2 * floor(log2(n + 1)) + 1` over `arb_base()`-scale values, or retire one. Acceptance: one committed point pin of gamma widths; the closed form, if kept, is a family property.

### testing-diff-gen-22: The asymptotics pins cite rustdoc sentences that no longer exist; the claims of record are the fuelscape roster's `contract` strings, and the Ω floor is not a public claim
- Where: crates/before/src/testing/asymptotics.rs:1-28 (related: crates/before/src/testing/asymptotics.rs:42-48, crates/before/src/testing/asymptotics.rs:65-68, crates/before/src/testing/asymptotics.rs:234-237, crates/before/src/testing/asymptotics.rs:383-384, crates/before/src/version.rs:289-293, crates/before/src/version.rs:471-475, crates/before/src/version.rs:1410-1412, crates/before-fuelscape/src/ops.rs:331, crates/before-fuelscape/src/ops.rs:360, crates/before-fuelscape/src/ops.rs:630, crates/before/build.rs:201-236)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -rn 'summary-merge\|grows faster than the operand'` over `crates/before/src` outside `testing/` hits only a comment at board/ops.rs:1044; `O(D log k)` outside `testing/` appears only in meter/board sources; `Ω(M` outside `testing/` appears only in private module docs (query.rs:112, integral.rs:240, registry.rs:843, meter.rs:2269); version.rs:291, 473, 1412 are `include_str!` of islands, and version.rs:293 says only that `M` is the multiplication complexity; build.rs:201-236 renders the island from the JSON `contract` and `claim`; `git show b3f09baa -- version.rs` removed "pays a summary-merge cost that grows faster than the" and added "Superlinear but subquadratic time"); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed; history: deliberate-but-expired (the prose binding chosen at 0a5bdaeb expired in three steps: b3f09baa removed the Display sentence, 2efff149 re-denominated the fold bound and made claims O-only, and b5a81583/2efff149 moved the wording into the roster; the fuelscape note's "nothing the asymptotics pins describe changes" is true of the bound and false of the quoted wording)
- Owner-gated: yes (whether rank/distance/lag/Ranked publicly promise the Ω lower bound is a documentation-contract decision)

Prose speaks in the present tense: a quotation of text not in the tree is a ghost reference, and a pin whose failure message names the wrong document to update does not bind the documentation it exists to bind. The rendered `# Complexity` text for `Display` is "superlinear, subquadratic time; `O(|self|)` space" (ops.rs:331); for the fold doors "`O((|self| + |iter|) log k)` time, `k` the operand count" (ops.rs:630); for rank "`O(M(|self|) · log |self|)` time" (ops.rs:360), with no Ω statement. A maintainer told to "update its `# Complexity` section" opens version.rs and finds an include.

Evidence:

         1	//! Liveness pins for the documented asymptotics: the growth behaviors
         2	//! the public rustdoc's `# Complexity` sections claim, held alive
         3	//! against the deterministic meters.
       ...
        44	/// `Display` limb work on the wide left-full shape grows super-linearly
        45	/// across a doubling, which is exactly what the rustdoc's "summary-merge
        46	/// cost that grows faster than the operand" sentence describes. When the
       ...
       236	         the documented `O(D log k)` overstates for this door, so update \
       237	         its `# Complexity` section and this pin together"
       ...
       383	/// through an independent backend multiplication — the value
       384	/// structure behind the `Ω(M(|v|))` floor the rank rustdoc states:

Resolution: Re-word the module doc and each failure message to name the authoring site (the operation's `contract` field in `crates/before-fuelscape/src/ops.rs`, regenerated into `fuelscape/<op>.json` and rendered into the section by build.rs) and quote the contract's actual wording. For the Ω floor, owner call: add the lower bound to the public contracts if it is meant to be promised (the style rule keeps Ω out of headlines and in prose), otherwise re-scope the three `mul_bound_*` pins' docs to the private derivation in `query.rs`/`integral.rs` with the invariant restated inline. Acceptance: every quoted sentence in asymptotics.rs appears verbatim in the artifact it names; every failure message names a file and field that exists at HEAD.

Construction: Change `fuelscape/version_display.json`'s `contract` to "linear time" and leave version.rs untouched: the rendered rustdoc now contradicts the pin, yet no `.rs` `# Complexity` section changed, so the failure message's instruction cannot be followed as written.

### testing-diff-gen-23: The fold-door pin roster claims one pin per door that documents the log factor; the island contracts state `log k` for eleven fold doors and five are pinned
- Where: crates/before/src/testing/asymptotics.rs:16-22 (related: crates/before/src/testing/asymptotics.rs:259, 285, 311, 341, 368, crates/before/fuelscape/clock_recv_all.json, crates/before/fuelscape/clock_sync_all.json, crates/before/fuelscape/span_join_all.json, crates/before/fuelscape/span_meet_all.json, crates/before/fuelscape/span_intersect_all.json, crates/before/fuelscape/span_union_all.json, crates/before/src/span/algebra.rs:339-369, crates/before/src/clock.rs:398, crates/before/src/clock.rs:565-571, crates/before/src/meter/board/ceilings.rs:347-350)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -l '"contract".*log k' crates/before/fuelscape/*.json` lists fourteen files; excluding `clock_forks`, `party_forks`, and `version_ticks`, whose `log k` is a different term, eleven balanced-fold doors remain: `clock_join_all`, `clock_recv_all`, `clock_sync_all`, `party_join_all`, `span_join_all`, `span_intersect_all`, `span_union_all`, `span_meet_all`, `version_join_all`, `version_meet_all`, `version_span_all`; the `_log_factor_is_alive` tests are exactly five; span's `*_all` route through `crate::fold::balanced_reduce` (algebra.rs:369) and `Clock::sync_all` through `balanced_try_fold` (clock.rs:398); `Clock::absorb_all` delegates to `Version::join_all` (clock.rs:570)); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: deliberate-but-expired (totality was enforced by the dissolved `FoldLog` class contract, whose span excusal read "law-pinned identical to Version::span_all" at 0a5bdaeb^ complexity_claims.rs:1234-1238; 0a5bdaeb dissolved that machinery and the excusal with it, and 2efff149 then stated `log k` on eleven doors)
- Owner-gated: no

The module's own premise is that a door's wiring can drop the factor without touching the shared core, so an unpinned door is exactly the case the instrument exists for. A roster stated as total but hand-maintained is a hand-maintained count in disguise, and nothing mechanical ties the pin roster to the contract strings: a new `*_all` door with a `log k` contract lands with no pin and no red. `Clock::recv_all` delegates wholly to `Version::join_all`, so its gap is nominal; `Clock::sync_all` and the four `Span` doors have their own wiring.

Evidence:

        18	//! - **Fold doors** (`scan-meter`): one pin per public door whose
        19	//!   rustdoc claims the balanced reduction's `O(D log k)`, each measured
        20	//!   at its own door — the doors share the balanced core, but a door's
        21	//!   wiring (short-circuit arms, per-component walks, the hull's
        22	//!   two-direction carry) can drop the factor without touching the core.

Resolution: Add a `const PINNED_FOLD_DOORS: &[&str]` beside the pins and a totality test that reads `crates/before/fuelscape/*.json`, collects every op whose `contract` contains `log k` under a balanced fold (excluding the forks and `version_ticks` by a stated rule), and asserts each is either pinned or excused with a reason at the check site (`clock_recv_all`: delegates to `Version::join_all`). Then either pin the remaining five doors over the stagger population (versions lifted to spans for the `Span` doors; the stagger clocks for `sync_all`) or excuse each with a reason. Narrow the module doc to what is pinned until then. Acceptance: a test in this module fails when a `fuelscape/<op>.json` contract containing `log k` names a fold door absent from the roster and unexcused; it passes at HEAD only after the six doors are pinned or excused.

Construction: Replace `balanced_reduce` behind `Span::join_all` with a left fold today: its `log k` contract stands with nothing to read red. Conversely, add a new public `*_all` door whose contract says `log k` and implement it as a left fold: the suite stays green.

### testing-diff-gen-24: "door" is a crate-wide coinage never defined; "seam", "keystone", and "vehicle" point at things not named
- Where: crates/before/src/testing/asymptotics.rs:18-28 (related: crates/before/src/testing/diff_ops.rs:14, crates/before/src/testing/diff_ops.rs:725, crates/before/src/testing/diff_ops.rs:839, crates/before/src/testing/diff_ops/tests.rs:23, crates/before/src/testing/diff_ops/tests.rs:58, crates/before/src/testing/diff_ops/tests.rs:144, crates/before/src/meter/registry.rs:13, crates/before/src/laws/tests.rs:29, crates/before/src/testing/semantic_oracle/tests.rs:157)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (`grep -c -w 'door\|doors'`: 43 in asymptotics.rs, 2 in diff_ops/tests.rs, 1 in diff_ops.rs; a grep for a definitional form across `crates/before/AGENTS.md`, `lib.rs`, and `validation_index.rs` finds only uses; registry.rs:13 "the one public door"); executed: no
- Seen by: structure-prose; refutation: confirmed; history: contradicts-hard-rule (writing-style.md:164-169: every coinage is an identifier or a term defined once by contrast; "door" entered through commit messages and is now load-bearing)
- Owner-gated: yes (vocabulary choice)

"door" is the axis the asymptotics pins are organized on ("one pin per public door") and is anchored to no identifier and defined nowhere. "The seam this pin defends is a drift" (diff_ops/tests.rs:58) misuses seam. "the keystone replay" (diff_ops.rs:725, 839) names `replay_matches_across_references` only at semantic_oracle/tests.rs:157. "vehicle" (diff_ops/tests.rs:23, 529, 542) is metaphor for `assert_diff_ops!`.

Evidence:

        18	//! - **Fold doors** (`scan-meter`): one pin per public door whose
        19	//!   rustdoc claims the balanced reduction's `O(D log k)`, each measured
        20	//!   at its own door — the doors share the balanced core, but a door's

    diff_ops/tests.rs:
        58	/// The seam this pin defends is a drift, not an error: a pointwise pure

    diff_ops.rs:
       725	    /// maximum the keystone replay exercises through `join`/`send`.

Resolution: Define "door" once where the pins are organized (asymptotics.rs module doc: a *door* is one public method an operation is reachable through; the pins bind each separately because a door's wiring can drop a factor the shared core keeps) and cite that definition from registry.rs and laws/tests.rs, or replace with "public method"/"entry point". Name `replay_matches_across_references` where "keystone" is used; "the hole this pin closes is a drift" for the seam sentence; "the assertion macro" for "vehicle". Acceptance: every use of "door" resolves to one definition or is gone; "keystone" is followed by the test name on first use in each file.

### testing-diff-gen-25: Idiom nits: qualified paths where the name is imported, a `Shape` name collision, and an undocumented block helper
- Where: crates/before/src/testing/asymptotics.rs:97-101 (related: crates/before/src/testing/asymptotics.rs:30, 107, crates/before/src/testing/diff_ops.rs:52-54, 522-529, 656, 712-715, crates/before/src/testing/compactness.rs:82-83, 157-158, crates/before/src/testing/snapshots.rs:33, 77, 96, 179, crates/before/src/testing/diff_ops/tests.rs:207, crates/before/src/version/skyline/fill/tests.rs:30, 1680)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (asymptotics.rs:30 imports `Shape` yet 99 and 107 spell `crate::meter::registry::Shape::StaggerPopulation`; diff_ops.rs:52-54 import `Base` and `Ticks` yet 522-529 and 714-715 spell `crate::codec::Base::ZERO` and 656 `crate::Ticks`, while 448 shows module imports resolve inside descriptor bodies; fill/tests.rs:30 imports `meter::registry::Shape` and 1680 must spell `generators::Shape::Bushy`; snapshots.rs:96 `fn version_block` has no doc while `party_block` at 84-85 does); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found (the collision arose when the meter registry landed its own `Shape` at cc84df7e1 beside the original generators enum)
- Owner-gated: no

Imports over long qualified paths except where the qualification informs; two same-named test-apparatus enums are exactly the case where it is forced to inform. `generators::Shape` is documented as "A deep tree shape" and `DeepShape` would disambiguate.

Evidence:

        97	fn stagger_versions(n: usize) -> Vec<crate::Version> {
        98	    let (versions, _) =
        99	        crate::meter::registry::Shape::StaggerPopulation.population(n, FOLD_DOOR_TEETH);

    diff_ops.rs (with `use crate::codec::Base;` at 52):
       522	            (crate::codec::Base::ZERO, c.version()),

Resolution: Add the `use` lines and shorten the paths; rename `generators::Shape` to `DeepShape`; give `version_block` the doc `party_block` carries (or fold the two into one generic over `as_bits`/`Display`). Acceptance: no `crate::` path in the partition whose leading segment is already imported in that file; one `Shape` reachable unqualified per test file; every fn in snapshots.rs has a doc comment.

### testing-diff-gen-26: The fold-door floors are transcribed midpoints; the linear reference is recomputed on every run, printed, and never asserted
- Where: crates/before/src/testing/asymptotics.rs:117-123 (related: crates/before/src/testing/asymptotics.rs:227-239, crates/before/src/testing/asymptotics.rs:249-256, crates/before/src/testing/asymptotics.rs:260, 286, 312, 342, 369, crates/before/src/meter/board/ceilings.rs:56-62)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`door_scan_bits` receives `input_bytes`, prints it, and returns only the scan bits; `assert_log_factor_alive` never sees a byte ratio; `git show 4797009a -- asymptotics.rs` recovers the excised readings, bytes ×4.77/×4.77/×4.77/×4.80/×4.79 against scan ×5.82/×5.82/×5.76/×4.99/×5.32 for join/meet/span/party/clock, and every `MIN_GROWTH` (5.29, 5.29, 5.26, 4.89, 5.05) is the midpoint; the pin commit f0cd4ab2f's own "FINDING 1" records the predecessor floor (4.6) sitting below its linear reference (×5.0), exactly this failure class, cured by re-measurement rather than an in-test check); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed (with the caution that today's byte ratios sit under every floor, so nothing is vacuous yet); history: the measured-midpoint design and the print-only harness are owner rulings (f0cd4ab2f "Floors are measured, never transcribed"; 4797009a excised the readings from prose), but no-rationale-found for not asserting the reference the harness has in hand
- Owner-gated: no for asserting the in-run linear reference (it adds a liveness witness inside the ruling); yes for replacing the measured floors with model-derived margins (that reverses f0cd4ab2f)

Instruments before cures: every floor needs a committed demonstration that the known-bad mechanism fails it. The known-bad here is a fold without the log factor, which reads the population's byte-growth ratio; that ratio is computed in the same run and is the one number that shows the floor discriminates, and it is discarded after an `eprintln!`. A `Shape::StaggerPopulation`, `FOLD_DOOR_TEETH`, or bytes-per-block change that steepens byte growth past a constant makes that pin vacuous with nothing reading red, and the "midway" claim in each pin's doc cannot be checked from the tree, since both endpoints live only in git.

Evidence:

       117	fn door_scan_bits<R>(name: &str, n: usize, input_bytes: usize, run: impl FnOnce() -> R) -> u64 {
       118	    crate::meter::reset_scan_bits();
       119	    std::hint::black_box(run());
       120	    let bits = crate::meter::scan_bits();
       121	    eprintln!("MEASURED {name}: n={n} input_bytes={input_bytes} scan_bits={bits}");
       122	    bits
       123	}
       ...
       230	fn assert_log_factor_alive(door: &str, lo: u64, hi: u64, min_growth: f64) {
       231	    let growth = hi as f64 / lo.max(1) as f64;
       232	    assert!(
       233	        growth >= min_growth,

Resolution: Return `(scan_bits, input_bytes)` from `door_scan_bits` (and the five `*_scan_bits` helpers), and in `assert_log_factor_alive` also assert `bytes_hi / bytes_lo < min_growth` with a message naming it as the linear reference the floor must exceed; this keeps the measured floors and makes the "midway" claim checkable. Owner-gated alternative: assert the door's marginal over the in-run linear reference (`growth / linear >= 1 + margin`) with the margin derived from the model (`log2(2·1024)/log2(2·256) = 11/9 ≈ 1.22` at this quadrupling; the version doors read 5.82/4.77 = 1.22), so a `FOLD_DOOR_TEETH` change needs no hand re-pin of five numbers; the party door's 1.04 marginal then stands out as the witness that needs a sentence or a better population (finding 28). Acceptance: each fold-door pin asserts both `scan growth >= floor` and `byte growth < floor` in the same run; temporarily replacing a door with a linear pass over the population reads red.

Construction: Add `assert!(bytes_hi as f64 / bytes_lo as f64 < MIN_GROWTH)` beside each pin using the `input_bytes` already computed; if it fails for any door today, that pin is already vacuous. Alternatively change `FOLD_DOOR_TEETH` from 64 to 32: the printed `input_bytes` ratio moves and the five floors no longer sit midway between anything, with no message saying the reference moved.

### testing-diff-gen-27: The asymptotics pins read process-global counters with no shared-process isolation note or guard
- Where: crates/before/src/testing/asymptotics.rs:230-239 (related: crates/before/src/testing/asymptotics.rs:34-40, crates/before/src/testing/asymptotics.rs:117-123, crates/before/src/codec/scan.rs:21-28, crates/before/src/meter.rs:3549-3551, crates/before/tests/meter.rs:351-355, justfile:109, justfile:114)
- Class / severity / confidence: test-quality / low / medium
- Provenance: verified (codec/scan.rs:28 is a `static AtomicU64` with Relaxed ordering and its module doc assumes "one scenario per process"; meter.rs:3549-3551 states the isolation requirement; tests/meter.rs:354-355 appends `ISOLATION_NOTE` to every envelope failure; the gate runs `cargo nextest run` at justfile:109 and 114, so the hazard is outside the gate); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no-rationale-found (the pins never carried the note in either home)
- Owner-gated: no

Measurements bind to their run. Under a shared-process `cargo test -p before --all-features`, the five scan-meter pins and every other scan-metered test interleave on one counter, so `lo`/`hi` are arbitrary and the failure message diagnoses a documentation problem ("the documented `O(D log k)` overstates"). The meter readers' own docs state the requirement; the consumer here does not carry it forward, and its docs call the counters "Deterministic" (50, 249).

Evidence:

       230	fn assert_log_factor_alive(door: &str, lo: u64, hi: u64, min_growth: f64) {
       231	    let growth = hi as f64 / lo.max(1) as f64;
       232	    assert!(
       233	        growth >= min_growth,
       234	        "{door}'s scan work grew only x{growth:.2} across a x4 population \
       235	         growth ({lo} -> {hi} bits; the log factor reads >= x{min_growth}): \
       236	         the documented `O(D log k)` overstates for this door, so update \
       237	         its `# Complexity` section and this pin together"

Resolution: Mirror tests/meter.rs: append an isolation note to `assert_log_factor_alive`'s and the Display pin's messages and state the nextest requirement in the module doc; optionally fail closed with `assert!(std::env::var_os("NEXTEST").is_some())` at the top of each metered pin. Acceptance: a failing pin's message names the shared-process runner as the first cause to rule out, or the pins refuse to run outside nextest.

Construction: Run `cargo test -p before --all-features --lib asymptotics` (not nextest) on a multi-core machine: the five `_log_factor_is_alive` tests run concurrently, each resetting `SCAN_BITS` while others are mid-fold; the outcome varies run to run and every failure message blames the documentation.

### testing-diff-gen-28: The `Party::join_all` pin's discriminating margin is about 2%, and the module doc's "a crossing is a class change, never noise" overstates
- Where: crates/before/src/testing/asymptotics.rs:339-349 (related: crates/before/src/testing/asymptotics.rs:12-14, crates/before/src/testing/asymptotics.rs:330-338, crates/before/src/party/ops/index.rs)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (readings of record from `git show 4797009a`: bytes ×4.80 (40,960 → 196,608 B), door ×4.99 (7,260,488 → 36,191,048 bits, i.e. 4.985); the floor 4.89 sits 1.9% from each endpoint and the log factor's marginal on this population is 3.8%; f0cd4ab2f: "party_join_all is the narrowest gap of the five (3.9%, exact counters both sides)"); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: deliberate-and-holds for the gap (measured, recorded, and accepted at the pin commit, with the inline stability argument at 330-338), but that rationale answers noise, not the two-term ambiguity, so the module-doc claim stands unaddressed
- Owner-gated: yes (the floor and its population were ruled at f0cd4ab2f)

A ratio of `a·k·log k + b·k` across a quadrupling moves with `b/a`; exact counters remove noise, not that ambiguity. A constant-factor change in the linear index term (`metered_partition_point` in `party/ops/index.rs` recording one more scan word per probe) crosses 4.89 with the log factor intact and the message blames the documented `O(D log k)`; conversely a linear fold whose per-input scan grows 2% faster than bytes reads 4.90 and passes with the log factor gone. The doc at 330-338 concedes the narrow gap and argues determinism makes it stable, which is true and not the point.

Evidence:

        12	//! moves in the same commit. Floors sit midway between the linear
        13	//! reference and the measured reading; both endpoints are exact
        14	//! counters, so a crossing is a class change, never noise.
       ...
       341	fn party_join_all_log_factor_is_alive() {
       342	    const MIN_GROWTH: f64 = 4.89;

Resolution: Witness the id fold's log factor on a population whose unions do not densify (isolated owned leaves at maximal depth, so each balanced union's encoding is near the sum of its parts), where the door should read near ×6.0 against ×4.8 as the version doors do; independently, restate lines 12-14 to what the pins can attribute: a deterministic tightness pin at two scales whose crossing means re-derive, not class change. Acceptance: for every fold-door pin the measured reading exceeds the in-run byte ratio by a stated margin the population is shown to express, and a committed known-bad (a left fold behind the same door on the same population) reads below the floor.

Construction: With the door at 4.985 and bytes at 4.80, `b/a ≈ 43` (from `(a·10240 + b·1024)/(a·2048 + b·256) · (4.80/4) = 4.985`); raising the per-input index cost by about 5% moves the ratio a few hundredths and crosses 4.89 while the balanced fold is untouched.

### testing-diff-gen-29: `balanced_terms` replicates `mul_into`'s recentering with nothing tying the two, and names a kernel that never compacts the parked factor in production
- Where: crates/before/src/testing/asymptotics.rs:407-437 (related: crates/before/src/testing/asymptotics.rs:389-395, crates/before/src/version/skyline/query/web.rs:104-171, crates/before/src/version/skyline/query/web.rs:208-214, crates/before/src/version/skyline/query/web.rs:409-415, crates/before/src/version/skyline/query/integral.rs:479-491, crates/before/src/version/skyline/query/integral.rs:705-709, crates/before/src/version/skyline/query/integral.rs:728-729, crates/before/src/meter.rs:2433-2435)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (compared the replica arm by arm with web.rs:156-171: same base-2^32 digits, same `> 1 << 31` threshold, same zero-term skip, same trailing carry; `mul_into`'s doc (web.rs:104-120) says the compacted `digits` operand is "word-scale ... this module's ledgers' reference counts" and both production call sites pass a count (`Base::from(reign.count)` at 211, the ledger `suffix` at 408-412); the wide × dense settle goes through `charge_digits` (integral.rs:728-729), where the parked factor is multiplied whole per cluster (`product = factor.clone(); product *= digit` at 484-485) and the balanced recentering `(sum + (1 << 31)) >> 32` (705-709) applies to the mass's digits; meter.rs:2434-2435 names the settle "parked `−(x − 1)` against the punctured trailing mass"); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: [24]'s boundary claim refuted (the replica matches `mul_into`; the `>=` rule is `WindowMass::combine`, a different kernel), [3]/[33] confirmed (no committed tie); history: no-rationale-found (013334f20 wrote the replica as a content check; the settle path the instance takes is `charge_digits`, which never compacts `x − 1`)
- Owner-gated: no

Any quantity computable two ways gets a committed test comparing them; a replicated kernel inside a pin drifts silently from its original. Beyond drift, the pin's premise misnames its mechanism: "the settle's own balanced-digit compaction (`mul_into`'s recentering, replicated)" describes a compaction production applies only to word-scale reference counts, never to the parked plunge `x − 1`, which `charge_digits` multiplies whole. The 65-term assertion is still a sound incompressibility proxy (a dense factor is not an all-ones run that a smarter algorithm could shift), but it is a reference measure the test defines, not a property of what the settle runs, and its constants `1 << 31`, `1u64 << 32`, `chunks(4)` are unnamed.

Evidence:

       407	    /// The count of nonzero balanced signed digits the settle's own
       408	    /// compaction (`mul_into`'s recentering, replicated) spells a
       409	    /// magnitude into.
       410	    fn balanced_terms(value: &UBig) -> usize {
       ...
       422	        for digit in digits {
       423	            let t = digit + carry;
       424	            if t > 1 << 31 {

Resolution: Either factor the compaction out of `mul_into` into a `pub(crate)` balanced-digit iterator both `mul_into` and this pin consume (then the pin measures a production kernel by construction), or restate the pin: `balanced_terms` is the test's own base-2^32 balanced-digit spelling, a reference incompressibility measure, with `DIGIT_BITS` and `HALF_DIGIT` named and the "settle's own" wording dropped; in either case name which production path the instance settles through (`charge_digits` over the mass digits with the plunge as the whole factor). Acceptance: either `balanced_terms` calls a production function, or its doc no longer claims to be the settle's own compaction and its constants are named.

Construction: Change `mul_into`'s threshold at web.rs:158 from `>` to `>=`: `mul_bound_embedding_is_alive` still asserts 65 and stays green because its replica keeps the old threshold; nothing in the tree notices the two spellings differ.

### testing-diff-gen-30: The stack-merge tiling loop is written three times in `shape_rows.rs`
- Where: crates/before/src/testing/shape_rows.rs:43-54 (related: crates/before/src/testing/shape_rows.rs:218-235, crates/before/src/testing/shape_rows.rs:243-260)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (the three loops read: push, merge while the top two depths agree, assert `depth > 0`, close at one depth-0 entry; the `let [(tree, 0)] = try_from(stack)... else { panic!(...) }` closer is spelled twice with the same message); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: no-rationale-found (all three landed in 46eb64f9)
- Owner-gated: no

One semantics written three times; a fix to the tiling rule must land in three places. `assert_tiles` is the unit-payload instance of the two reconstructions.

Evidence:

        47	        while stack.len() >= 2 && stack[stack.len() - 1] == stack[stack.len() - 2] {
        48	            let merged = stack.pop().expect("two entries are on the stack");
        49	            assert!(merged > 0, "two whole intervals cannot both be present");
       ...
       222	        while stack.len() >= 2 && stack[stack.len() - 1].1 == stack[stack.len() - 2].1 {
       223	            let (right, depth) = stack.pop().expect("two entries are on the stack");
       224	            let (left, _) = stack.pop().expect("one entry remains");
       225	            assert!(depth > 0, "two whole intervals cannot both be present");

Resolution: `fn from_rows<T>(rows: impl IntoIterator<Item = (T, u64)>, node: impl Fn(T, T) -> T) -> T` with the single closer; `version_from_rows` and `party_from_rows` pass their normalizing constructors, `assert_tiles` passes `((), d)` and `|_, _| ()`. Acceptance: one merge loop in the file.

### testing-diff-gen-31: `EXEMPTIONS` in `fuelscape_islands.rs` is an empty acceptance buffer
- Where: crates/before/src/testing/fuelscape_islands.rs:16-22 (related: crates/before/src/testing/fuelscape_islands.rs:63-81, crates/before/src/testing/diff_ops.rs:799-801, crates/before/src/testing/diff_ops/tests.rs:122-136, crates/before-fuelscape/src/ops.rs:2173)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (before-fuelscape/src/ops.rs:2173 `pub const EXEMPTIONS` is inhabited with stated reasons, so the idiom's real instance lives there; `git show b5a81583:.../fuelscape_islands.rs:18-24` shows this roster inhabited with one reviewed entry, and `git show 2efff149` emptied it and added the "Currently empty" note); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed; history: deliberate-but-expired (the roster landed inhabited as the fuelscape design note anticipated; the review round removed its one entry and kept the mechanism)
- Owner-gated: no (reintroducing the roster with its first reviewed entry is a one-diff event)

The doctrine names this shape directly: no mechanism for accepting known failures may exist, even as an empty buffer waiting to hold them. The crate applies the same rule to itself at diff_ops.rs:799-800 ("an empty genre is a dead category, dissolved rather than carried"), enforced by `every_bespoke_genre_is_inhabited`. The exemption mechanism, its two validation loops, and the "or exempted" arm exist to hold entries that do not exist, and "Currently empty" is temporal prose.

Evidence:

        16	/// Operations whose islands deliberately appear in no doc comment, each
        17	/// with its reviewed reason.
        18	///
        19	/// Currently empty: every measured operation's island reaches the
        20	/// rendered docs (the operator matrices and the conjunction cells carry
        21	/// theirs through their generating macros).
        22	const EXEMPTIONS: &[(&str, &str)] = &[];

Resolution: Delete `EXEMPTIONS`, the exemption loop (63-72), and the `exempt` set; assert `emitted == included` as two `BTreeSet<String>`s so the messages at 77-80 and 85 become the two set-difference reports. Reintroduce a roster the day the first reviewed exemption exists, with its reason, as the bespoke-genre roster does. Acceptance: the test body compares two sets; no exemption mechanism and no "Currently empty" in the file.

### testing-diff-gen-32: The island include scan cannot see the seven macro-form include sites its own doc credits
- Where: crates/before/src/testing/fuelscape_islands.rs:44-58 (related: crates/before/src/testing/fuelscape_islands.rs:19-21, crates/before/src/testing/fuelscape_islands.rs:63-72, crates/before/src/clock.rs:1012, 1026, 1040, crates/before/src/clock.rs:1049-1050, crates/before/src/version.rs:1547, 1562, 1577, 1640, crates/before/src/version.rs:1587-1588, 1604-1605, 1651-1652, crates/before/src/clock.rs:485, crates/before/src/version.rs:445, 505, 571)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -rn '"/fuelscapes/", '` lists the seven macro sites spelling `"/fuelscapes/", $island, ".html"`, so `split_once(".html")` yields `", $island, "` and the charset filter drops it; the matrix invocations pass `"version_join"` (clock.rs:1050, version.rs:1588), `"version_meet"` (1605), `"version_span"` (1652), and each has a literal twin at clock.rs:485 and version.rs:445, 505, 571; the conjunction cells' include at conjunction.rs:154 is literal and visible); executed: no
- Seen by: adequacy, instrument-correctness; refutation: confirmed (adding that an exemption for a matrix-only island would pass the `!included.contains(op)` check at 68-71); history: deliberate-but-expired (the literal-only scan was adequate at b5a81583; 2efff149 introduced the macro include form and the coverage claim at 19-21 without teaching the scan the new form)
- Owner-gated: no

A surface extractor must be total over the syntax it claims, and its doc must state what it checks. The pin passes today only because each matrix island also has a literal include; the doc's claim that the matrices "carry theirs through their generating macros" describes an attachment the scan cannot verify. The main assertion fails closed (a matrix-only island reads red with a misleading "no doc comment includes it"), but the exemption blind spot is real: a stale exemption for a macro-only island is accepted because the include-versus-exempt check never sees the include.

Evidence:

        47	                for site in source.split("/fuelscapes/").skip(1) {
        48	                    let Some((op, _)) = site.split_once(".html") else {
        49	                        continue;
        50	                    };
        51	                    let island_shaped = !op.is_empty()
        52	                        && op
        53	                            .bytes()
        54	                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');

    clock.rs:
      1012	        #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/", $island, ".html"))]

Resolution: Teach the scan the macro form (for each `*_matrix! {` invocation, collect its first string-literal argument as an included island), or route every island include through one `island!("name")` macro whose spelling the scanner recognizes; at minimum state beside the charset filter that macro sites are invisible and that the matrices' islands are held by their method-doc twins, and correct the sentence at 19-21. Acceptance: removing the literal include at version.rs:505 while leaving `binop_matrix!` attached keeps the test green (the scan sees the macro), or the doc says plainly that it will not.

Construction: Add a new matrix island `version_xor` to `fuelscape/index.json` and include it only via `binop_matrix!("version_xor", ...)`: the test fails with "island version_xor is emitted but no doc comment includes it" although one does.

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

- [0]/[18] dissolution branch of the compactness suite: reopens a recorded owner ruling (defended keeps of the 2026-07-24 scaffolding sweep, `.agent-notes/2026-07-22-before-adversarial-resource-amplification/before-adversarial-resource-amplification.md:1756-1760`, reaffirmed at faf3cd0a) with no new evidence; the prose half is finding 17.
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
