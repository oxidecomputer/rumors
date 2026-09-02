# Partition meter-registry-tier2: The family registry and the tier-2 meters

## Partition summary

The registry (`crates/before/src/meter/registry.rs`, 1988 lines) is the roster every resource instrument in `before` derives its family axis from. It has two halves. `Shape` is the single public door to the private adversarial generators: a 68-variant enum whose exhaustive `builder()` match binds each variant to one generator and whose thirteen accessors (`packed1`, `packed2`, `packed_pair`, `versions`, ...) dispatch on the generator's signature class at runtime. `FamilyId` is the 52-variant family roster; each variant's `spec()` arm is its row of record (`FamilySpec`: name, cited shapes, a `Coverage` answer that is either a board column with a declared cell reach or a dated envelope-only ruling, a `Bands` answer that is either a roster of `tests/meter.rs` test names or a dated no-band ruling, plus two free-text fields `denominator` and `closed_form`). `FamilyId::ALL` is the hand-written roster array, `index()` a 52-arm match restating its order, `board()` the filter on the coverage answer, and `AXIS_BANDS` the table of band names no family row carries. The registry's own tests (`registry/tests.rs`, 226 lines) pin the seams the compiler cannot: roster order, name uniqueness, every shape cited by some family, band citations unique and non-empty, the board roster non-empty with nonzero reach, and rulings dated.

The sizer (`crates/before/src/meter/tier2.rs`, 123 lines) is an instrument, not a codec: `tier2_size` walks a construction-language stream (the generators' min-lifted preorder form, one gamma base per node) iteratively over a heap stack of inherited path sums and returns the skyline coded size of the version it denotes, decomposed into topology bits, first-leaf bits, and delta bits, with its own `zigzag` and `gamma_bits` so that it is an implementation of the coding independent of the encoder. Its tests (`tier2/tests.rs`, 735 lines) hand-pin the sizer on small trees, pin the boundary comb's closed forms, commit the known-bad plain-accumulator sweep that motivates suanpan, and hold two join/meet coding lemmas under four emitters (the public operators and the emission kernels, the kernels also asserting their emitted stream length against the sizer): a 1-Lipschitz ceiling with a 4-bits-per-leaf slack and a subadditivity lemma with a derived, tight 2-bit margin.

The quality is high where the compiler or a committed test does the holding. The `compile_fail,E0603` doctest is a committed demonstration that the raw generators are unreachable outside the door; `Coverage` and `Bands` as sum types force an explicit written reason for every non-answer, with no expected-failure buffer anywhere; the declared `cells` reach is enforced at the shard merge and in the rendered smoke matrix; the band-name parity pin in `tests/amp_board_smoke.rs` is bidirectional and attribute-gated; the subadditivity constant is derived term by term and its tightness is a committed equality witness rather than an assertion; the plain-sweep tripwire is value-exact and two-scale on a deterministic counter. The dominant issues are prose that no instrument holds: `tier2.rs` still describes the stored coding as a candidate and the deleted construction-language coding as "today's" (inverting the code since the flag-day commit faf3cd0a), several registry reason strings name enforcement homes that do not hold the named pin, three flatness bands rest their adequacy on uncommitted "probe builds" where every sibling band has a rostered known-bad kernel, and the hand rosters (`ALL`, `index()`, `ALL_SHAPES`) are checked only against each other so a variant outside `ALL` reaches no instrument while `index()`'s doc claims the opposite. On the test side, the Lipschitz pin is implied pointwise by the subadditivity pin over the same emitters and populations and can fold into it.

Lines read: 3072 in the partition (registry.rs 1988, registry/tests.rs 226, tier2.rs 123, tier2/tests.rs 735), all with line numbers, plus the cited neighbors (version.rs, skyline.rs, skyline/tests.rs, testing/compactness.rs, board/cell.rs, board/family.rs, board/shard.rs, board/ops.rs, meter.rs, codec/base/limb_meter.rs, codec/base/limb_metered.rs, codec/base.rs, codec/bits.rs, recurse.rs, tests/meter.rs ranges, tests/amp_board_smoke.rs, tests/superlinear_tripwires.rs, query/tests.rs ranges, suanpan metered.rs header, surfacecheck check.rs, before/AGENTS.md) and the commit messages the history pass cited. `registry/tests.rs` and `tier2/tests.rs` are test files. No cargo, just, build, or test command was run; every "verified" below is reading, grep, or git.

## Findings

### meter-registry-tier2-1: Vocabulary tells in the registry prose: "mint" for construction, "earns", "sentinel", "honest", "luck", "mandate", "tombstone"
- Where: crates/before/src/meter/registry.rs:20-31 (related: registry.rs:43, 566, 572, 584, 636, 675, 734, 748, 762, 776, 794, 813, 832, 883, 887, 920, 1036, 1076; crates/before/src/meter/tier2/tests.rs:163)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -nwE` over the four partition files; every site listed was read); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: contradicts the owner's writing-style rule for "mint" (the rest is register guidance); the only "mint" purge (2c73d032) was scoped to the rumors crate
- Owner-gated: no

The registry is public rustdoc under the `meter` feature and uses "mint"/"minted" for constructing a value at three sites, which the owner's vocabulary rule forbids outright; "earns a column" (five sites), "honest-less-work witness", and "never mandate" are register transplants where no economy exists; "sentinel" in a watchman sense (seven family docs) is a coinage the roster already names as "probe"; "found by luck" and "coalescing luck" name an unchecked step instead of the mechanism; "tombstone" is a metaphor with no mechanism and collides with the rumors crate's redaction rule. One curly apostrophe sits in tier2/tests.rs:163.

Evidence:

        20	//!   or downstream crate — can mint an adversarial shape except through
        31	//!   // The registry door: the same comb, minted through its Shape row.
       572	/// compiler cannot force, in the order it is otherwise found by luck: the
       675	/// difference is minted at every consume and popped at every close — the
       883	    /// zero, so the pair is also the touch floor's honest-less-work witness
       887	    /// implementation, never mandate.
       920	    /// live records, the live-anchored followers' tombstone.

Resolution: "build"/"built" for mint; "has a column"/"gets no column" for earns; "probe" for sentinel; "minimal-work witness" and "never a requirement" at 883/887; name the mechanism at 920 ("the shape that retires every live-anchored follower"); "otherwise unchecked" at 572 and "the adjacent-slot coalescing that index order would allow" at 636; a straight apostrophe at tests.rs:163. Acceptance: `grep -nwiE 'mint|minted|earns?|sentinel|honest|luck|mandate|tombstone' crates/before/src/meter/registry.rs` returns nothing, and `grep -n "’" crates/before/src/meter/tier2/tests.rs` is empty.

### meter-registry-tier2-2: The module doc calls the band-to-family link compiler-checked; the compiler checks only band-to-Shape
- Where: crates/before/src/meter/registry.rs:46-50 (related: registry.rs:66-71; crates/before/tests/amp_board_smoke.rs:342-354)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read both paragraphs; `grep -n 'registry::' crates/before/tests/meter.rs` shows only `Shape` imported from the registry); executed: no
- Seen by: scaffolding; refutation: reframed (the two statements describe different links); history: the phrase is lifted from 608dea84's commit message and was never separately defended
- Owner-gated: no

The sentence names the compiler-checked link as "band-to-family" when the compiler checks only that a band builds its operands through `Shape`; the Shape-to-family link is the `shapes` data row and the band-to-family link is the `Bands::Priced` name roster, both pinned by tests, exactly as the module's own "What the compiler cannot reach" section says. Documentation accuracy: the first paragraph over-states what the second retracts.

Evidence:

        46	//! - **The bands.** The envelope suite's flatness/adequacy bands build
        47	//!   their operands through [`Shape`], so the band-to-family link is a
        48	//!   compiler-checked construction site, never a name mapping held in a
        49	//!   parallel table; each family's committed band roster is its spec's
        50	//!   [`Bands`] answer, and a family without a band carries the dated

Resolution: reword to "the band-to-shape link is a compiler-checked construction site; the band-to-family link is the spec's `Bands` roster, a name mapping the board smoke suite pins against the suite's `#[test]` names". Acceptance: the module doc's two statements about bands agree.

### meter-registry-tier2-3: Band-name parity is keyed on a naming convention; seven two-point flatness tests escape it
- Where: crates/before/src/meter/registry.rs:66-71 (related: crates/before/tests/amp_board_smoke.rs:306-340; crates/before/tests/meter.rs:6354, 6378, 6623, 6637, 6650, 6670, 6695; registry.rs:1663-1664, 1680-1681)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read `band_test_names` at amp_board_smoke.rs:314-340; `grep -c 'assert_flat(' tests/meter.rs` = 68 against 49 convention-named fns; the seven escapees each sit under `#[test]`, read at lines 6353, 6377, 6622, 6636, 6649, 6669, 6694); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: the name-keyed scan was deliberate (bf2221554, a44502fe) but neither commit nor the registry doc mentions the two-point tests outside the convention, which predate it
- Owner-gated: no

The registry says the smoke suite "holds them equal, name for name" to the citation rosters, but the scanner recognizes only names containing `_is_flat_per_unit` or ending in `_band`, so a two-point `assert_flat` test with any other name needs no registry answer. Seven exist today (`id_covers_scan_cost_is_pinned_and_flat`, `id_disjoint_scan_cost_is_pinned_and_flat`, and the five `accum_*_touches_flat`). Principle 6: the parity survivor's totality is over the convention, not over the band mechanism, and this is how the cliff-fan and cancelling-chain rows came to name the wrong enforcement home (finding 10): their real two-point pins never had to be cited.

Evidence:

        66	//! - **Band names.** The bands live in a separate test binary
        67	//!   (`tests/meter.rs`), and test function names are not items the
        68	//!   compiler can resolve across crates. The board smoke suite
        69	//!   (`tests/amp_board_smoke.rs`) scans that suite's band-named tests
        70	//!   and holds them equal, name for name, to the union of the specs'
        71	//!   [`Bands`] rosters and [`AXIS_BANDS`].

    amp_board_smoke.rs:
       331	                    if name.contains("_is_flat_per_unit") || name.ends_with("_band") {

Resolution: either key the scan on the mechanism (collect every `#[test]` fn whose body calls `assert_flat`/`assert_model_flat`) or rename the seven tests into the convention and cite them (the accumulator streams on their families' rows, the id scan pair in `AXIS_BANDS`); state in the module doc that the scan is total over the convention. Acceptance: adding `#[test] fn scratch_touches_flat()` with an `assert_flat` call and no registry citation fails `band_tests_and_registry_citations_stay_paired`.
Construction: add to tests/meter.rs `#[test] fn scratch_touches_flat() { let s = comb_run(4_096, 50_000); let l = comb_run(8_192, 100_000); assert_flat("scratch", &s, &l, envelope::COMB_MILLI_PER_DELTA); }` with no registry change; the parity test passes.

### meter-registry-tier2-4: Shape notation letters collide: B, F, W, A each name two shapes
- Where: crates/before/src/meter/registry.rs:100-174 (related: registry.rs:891, 922)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (read the variant docs at 100, 112, 115, 124, 146, 150, 159, 174 and the uses at 891, 922); executed: no
- Seen by: adequacy; refutation: confirmed; history: letters coined per family across separate commits, no shared table
- Owner-gated: no

Bigroot is `B(b, d)` and MemoComb is `B(d)`; CliffFan is `F(k, n)` and MemoFanout is `F(k, b)`; WideToothComb is `W(k, w, n)` and DescendingRaises is `W(d)`; AltSpine is `A(d)` and AscendCliff is `A(k, b)`. The letters are used as shorthand in family prose across the crate (`W(k, w, n)` at 891 and `W(d)` at 922 in the same doc block). A coined notation must resolve to one identifier.

Evidence:

       100	    /// The bigroot event `B(b, d)`, a `2^b − 1` root over `S(d)`:
       146	    /// The memo-comb event `B(d)`: [`Shape::packed1`]`(d)`.
       112	    /// The wide-tooth comb `W(k, w, n)`, `n` teeth of width `2^w`:
       159	    /// The descending-raises event `W(d)`: [`Shape::packed1`]`(d)`.

Resolution: give the later coinages distinct abbreviations (the two-letter style the newer families already use: `MB`, `MF`, `DR`, `AC`) and sweep the prose that uses them. Acceptance: no two `Shape` variant docs share a leading notation letter.

### meter-registry-tier2-5: `wrong_door` points at the variant doc instead of naming the accessor; the door argument is stated twice
- Where: crates/before/src/meter/registry.rs:384-386 (related: registry.rs:97-265, 272-299, 302-308, 12-21)
- Class / severity / confidence: documentation / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no rationale found (cc84df7e designed it so)
- Owner-gated: no

Each of the 68 variant docs ends with its accessor name by hand, `wrong_door` defers to that doc, and nothing checks the doc against the `Builder` arm; a generator whose signature class changes leaves a stale doc and a panic message pointing at it. Separately the compiler-tie argument appears in the module doc (12-21) and again on `builder` (302-308).

Evidence:

       384	    fn wrong_door(self, called: &str) -> ! {
       385	        panic!("{self:?} does not build through {called}: its variant doc names its accessor")
       386	    }

Resolution: give `Builder` an `fn accessor(&self) -> &'static str` and have `wrong_door` print "{self:?} builds through {right}, not {called}"; the variant docs may then drop the accessor suffix; keep the door argument once, in the module doc. Acceptance: the panic message names the correct accessor; the argument appears once.

### meter-registry-tier2-6: Measured tripwire readings (×1.50, ×1.74) and a constant's value are restated in family docs
- Where: crates/before/src/meter/registry.rs:741-745 (related: registry.rs:756-759, 609-610; crates/before/src/version/skyline/query/tests.rs:1487-1493, 1798-1804; crates/before/src/meter/board/family.rs:378)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read query/tests.rs 1484-1510 and 1795-1821: the asserted floors are `* 125` and `* 136`, the ×1.50/×1.74 figures are bracketed measurement notes; `WEAVE_GROUPS: usize = 16` at family.rs:378); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed; history: introduced at cc84df7e; the 500d4d09 sweep that excised measured snapshots from meter prose reworded the weight-comb/freeze-parade readings but left these
- Owner-gated: no

The FreezePos and PromoRearm docs quote the known-bad kernels' measured per-byte growth; the committed tripwires enforce floors of 1.25 and 1.36, so the quoted numbers are snapshots no test compares and will rot on the next re-measure (the notes say "measured in the dev profile"). Unlike DenseSuffix (803-805) and PlateauPuncture (843-846), these rows do not name the tripwire functions the roster holds live. The Weave doc restates `WEAVE_GROUPS` as "16 group parties". Principle 5: a number that matters lives in a mechanically-enforced place that prose cites by name.

Evidence:

       741	    /// while the family's positions compact to O(1) digits. The committed
       742	    /// known-bad kernel reads ×1.50 per byte across the doubling on this shape
       743	    /// (the query fold's adequacy tripwire); the anchored-segment discipline
       756	    /// to O(1) balanced terms. The committed known-bad kernel reads ×1.74 per
       609	    /// The weave fold population: the leaves of one balanced fork tree dealt
       610	    /// round-robin among 16 group parties (the board's weave-group constant),

Resolution: replace the readings with the kernel names (`absolute_position_accounting_reads_superlinear_on_freeze_position`, `span_promotion_accounting_reads_superlinear_on_rearm_spine`), as the DenseSuffix and PlateauPuncture rows do, and "16 group parties" with "`WEAVE_GROUPS` group parties". Acceptance: `grep -nE '×1\.[0-9]+|16 group' crates/before/src/meter/registry.rs` returns nothing.

### meter-registry-tier2-7: Weight-comb, freeze-parade, and tooth-tail bands cite uncommitted probe builds as their only adequacy witness
- Where: crates/before/src/meter/registry.rs:769-774 (related: registry.rs:784-786; crates/before/tests/meter.rs:4479-4498, 4544-4555, 4652-4665, 4737-4747; crates/before/tests/superlinear_tripwires.rs:27-68; crates/suanpan/src/accumulator/tests/metered.rs:8-11)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -rnE 'fn [a-z_0-9]+_reads_superlinear'` over crates/before and crates/suanpan lists ten kernels, none on weight_comb, freeze_parade, or tooth_tail; the `TRIPWIRE_ROSTER` at superlinear_tripwires.rs:27-68 read; `grep -rn 'probe build'` hits registry.rs:770, 785 and tests/meter.rs:4489, 4551, 4660, 4742; suanpan's one committed known-bad is `no_collapse_fold_re_scans_the_prefix`, metered.rs:8-11); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed; history: no rationale found (round #85 recorded kill-switch probe builds; f10f5b56 later made rostered committed kernels the crate's standard; 500d4d09 kept the wording)
- Owner-gated: no

The WeightComb and FreezeParade rows state that the known-bad mechanism was "demonstrated by a probe build", and the band docs in tests/meter.rs say the same for weight-comb, freeze-parade, and tooth-tail ("a local probe build"). No committed kernel fails on any of the three; every sibling rank band (freeze-position, rearm, dense-suffix, wide-arming, plateau-puncture, shade) has a `_reads_superlinear` kernel in the roster. Doctrine: every criterion needs a committed demonstration that a known-bad mechanism fails it; a demonstration that lives in someone's local tree is a told number with no run bound to it (Principle 8). If the zero-run certificate, the write watermark, or exact-top maintenance quietly stopped being what the family reaches, all three bands stay green and nothing committed notices.

Evidence:

       769	    /// never-written run. A settlement scan that steps the gap digit by digit
       770	    /// goes quadratic here (demonstrated by a probe build with certificate
       771	    /// consumption disabled); consuming one
       772	    /// zero-run certificate per jumped run reads flat (the `skyline_flatness`
       773	    /// weight-comb band). Designed against the linear-functional query rows.
       774	    WeightComb,
       784	    /// quadratic in the touch and limb currencies together (demonstrated by
       785	    /// a probe build whose scaled reads
       786	    /// start at digit 0); the watermark reads flat (the `skyline_flatness`

    tests/meter.rs:
      4551	    /// consumption disabled (a local probe build whose scans step
      4552	    /// digit by digit), the reading goes quadratic — `n² + O(n)`

Resolution: commit the three refuted mechanisms as kernels following the crate's own pattern (test-local variants in the query suite or suanpan's metered suite, mirroring `no_collapse_fold_re_scans_the_prefix`): a settle that steps the gap digit by digit, asserted superlinear across `WC(n) -> WC(2n)`; a scaled read starting at digit 0 (the plain `sign_magnitude` suanpan's metered.rs:20-23 already names as the full-held-width read), asserted superlinear across `FZ(k) -> FZ(2k)`; a high-water-bounded sign read on the tooth-tail pair. Add each to `TRIPWIRE_ROSTER`, then replace "demonstrated by a probe build" at registry.rs:770-771 and 784-786 and "a local probe build" at tests/meter.rs:4489, 4551, 4660, 4742 with the kernel names. The nearest existing mitigation, suanpan's exact-count row pins (`alternating_shifted_writes_cost_the_operand_not_the_gap`, `scaled_read_costs_the_written_span`, `held_width_rows_cost_the_held_digits`), holds the shipped mechanisms crate-locally but demonstrates no known-bad failing. Acceptance: the roster gains three entries, each reads red on its family at both scales, and `grep -rn 'probe build' crates/before crates/suanpan` returns nothing.
Construction: degrade `weight_comb` so no never-written gap exists (drop the parked-unit spine so the oscillation lands at digit 0); `skyline_rank_weight_comb_is_flat_per_unit` stays green (flat, under its absolute ceilings, above its `2n` nonzero-delta floor) and no committed test reports that the family has stopped exercising the zero-run ledger. A committed per-digit settle kernel would read flat on the degraded family and fail its floor, exposing the dark family; today nothing does.

### meter-registry-tier2-8: `FamilySpec.denominator` and `closed_form` are prose stored as data that nothing reads; `shapes` is checked in one direction only
- Where: crates/before/src/meter/registry.rs:1022-1047 (related: registry.rs:1959-1988; crates/before/src/meter/registry/tests.rs:111-129, 159-167; crates/before/tests/amp_board_smoke.rs:370)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rnE '\.(denominator|closed_form)\b'` over crates/before and before-fuelscape: the only `.denominator` hits are `run.denominator` on the `Run` struct in tests/meter.rs:6489-6520, no `.closed_form` reader; `.spec()` outside the registry is read at shard.rs:506 and amp_board_smoke.rs:75 for `coverage` and at :364 for `bands`; `.shapes` is read only at registry/tests.rs:120; the `AXIS_BANDS` disposition is bound to `_` at amp_board_smoke.rs:370 and checked non-empty at registry/tests.rs:164-167); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed (severity lowered to low: dead weight and a rot risk, not a live defect); history: no rationale found (both fields landed at cc84df7e with no consumer)
- Owner-gated: yes (public struct fields under the `meter` feature)

The struct is documented as "the answers every instrument derives from", but no code outside registry.rs reads `denominator` or `closed_form`; no test compares the denominator strings against the board's per-cell denomination (board/cell.rs decides that mechanically) or resolves the pins `closed_form` names in prose ("the meter suite pins the size closed form"). As string fields they render in no rustdoc and are checked by nothing, so a renamed pin or a re-denominated column leaves them stale silently. `shapes` is read once, in the "cited at least once" direction, so a family row citing a shape its bundle no longer builds passes. Principle 3: a row of record earns its place by what reads it.

Evidence:

      1022	/// One family's row of record: the answers every instrument derives
      1023	/// from.
      1041	    /// The denominator of record: what this family's priced readings are
      1042	    /// charged against.
      1043	    pub denominator: &'static str,
      1044	    /// The closed-form hook, where one exists: the quantity computable two ways
      1045	    /// and the pin that compares them.
      1046	    pub closed_form: Option<&'static str>,

Resolution: owner's call between consuming and demoting. Consume: render each column's denominator from the spec in the board header so the smoke suite can pin its presence, and make `closed_form` carry a test name the smoke suite's scan resolves like band names. Demote: move both into each variant's rustdoc (where the `FamilyId` docs already carry this altitude of prose) and shrink `FamilySpec` to the fields instruments consume (`name`, `shapes`, `coverage`, `bands`); make `AXIS_BANDS: &[&str]` with each disposition as a comment above its entry. For `shapes`, have board/family.rs's bundle build record the `Shape` variants it constructs and assert equality with `spec().shapes` in the smoke suite. Acceptance: every remaining `FamilySpec` field has a reader outside registry/tests.rs, or the fields are gone and the variant docs carry the sentences; a `shapes` row gaining a variant its bundle does not build fails a test.

### meter-registry-tier2-9: Dated rulings (`decided`, `REGISTRY_RATIFIED`) are dated rationale at declaration sites, enforced by a shape-only date test
- Where: crates/before/src/meter/registry.rs:1102-1103 (related: registry.rs:1075-1080, 1091-1096; crates/before/src/meter/registry/tests.rs:204-226; crates/before/surfacecheck/src/check.rs:259-273)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (read the fields and the test; `git show d2a9d04e` message reports the machinery as needing "a design round, not a prose sweep"; surfacecheck's `dated` closure at check.rs:262-273 checks month and day ranges); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: already known (d2a9d04e, 2026-07-31, reported it to the owner; no design round is recorded since)
- Owner-gated: yes (a documented convention shared with surfacecheck; one ruling covers both)

`Coverage::EnvelopeOnly { decided }`, `Bands::Unbanded { decided }`, and `REGISTRY_RATIFIED` carry calendar dates whose only reader is `envelope_only_rulings_are_dated`, a length-10-two-dashes check (`"abcd-ef-gh"` and `"2026-13-99"` pass) that surfacecheck's sibling check does properly. Doctrine under Principle 5: dated rationale at a declaration site is history in the tree; git blame carries the ruling date with no maintenance. The module doc treats these as decision records ("the dated ruling of record") without naming the exemption.

Evidence:

      1102	/// The date the registry's rulings were ratified as the rows of record.
      1103	const REGISTRY_RATIFIED: &str = "2026-07-29";
      1078	        /// The date of the ruling of record.
      1079	        decided: &'static str,

    registry/tests.rs:
       215	                decided.len() == 10 && decided.chars().filter(|&c| c == '-').count() == 2,

Resolution: rule once for the registry and surfacecheck. If the registry is a decision record: say so in one sentence of the module doc and lift surfacecheck's date parse into a shared helper. Otherwise: drop `decided` and `REGISTRY_RATIFIED`, keep `reason`, fold the non-empty-reason check into `band_citations_are_unique_and_nonempty`, and delete the date test. Acceptance: either the module doc names the decision-record exemption and one real date parse is shared, or `grep -nE '20[0-9]{2}-[0-9]{2}-[0-9]{2}' crates/before/src/meter/registry*` is empty.

### meter-registry-tier2-10: Reason strings name enforcement homes that do not hold the named pin or documentation
- Where: crates/before/src/meter/registry.rs:1108-1109 (related: registry.rs:1382, 1415, 1426, 1628-1636, 1642-1647, 1656-1666, 1673-1683; crates/before/tests/meter.rs:468-476, 483-485, 503-505, 529-531, 2688-2721, 2829-2852, 4504-4523, 6569-6585, 6637-6641, 6670-6673; crates/before/src/meter/board/family.rs:564-603)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read every `fn tick_*` operand at tests/meter.rs:468-560 and the ticks bands at 728-745, 888-900; read `rank_wide_tooth_run` 2688-2721 and `rank_jump_run` 2829-2852 and `grep -n -i internal tests/meter.rs` (hits only at 4604 and 7704, unrelated); read the accum tests 6560-6700 and `cancelling_run`; grep of `CliffFan|cliff_fan` over the skyline and tier2 suites shows corpus membership only, no counter read); executed: no
- Seen by: adequacy, instrument-correctness; refutation: confirmed on all three legs; history: all strings unedited since cc84df7e; the wide-tooth internal entry had a technical reason at the band's landing (f39751d67, one day before the flag day, when `Version::rank` was still the tree fold) that faf3cd0a removed
- Owner-gated: no (correcting the strings toward the code is sanctioned; whether wide-tooth and jump-comb become board columns is the owner's)

The registry is "the single source of truth ... from which every instrument derives" (lines 1-2), so a reason string that misdirects an audit is a claim contradicted (Principle 8). Three kinds:

(1) `TICK_CROSS_UNBANDED` is shared by NestedFull (Dense × NestedFullId), NestedWide, MirrorWide, MirrorNarrow (WideTail(1, d) × NestedLeftFullId), and Staircase (Staircase × IdSpine(d, false), per board/family.rs:564-603). tests/meter.rs holds tick pins for Bigroot × NestedFullId (`tick_nested_wide_envelope`, 483-485) and WideTail(s, s) × NestedLeftFullId (`tick_mirror_wide_envelope`, 503-505); `tick_dense_envelope` ticks the dense spine with `Party::seed()` (468-476), and the only staircase tick pin (`tick_ownership_hole_envelope`, 529-531) uses `IdSpine.packed_flagged(HOLE_ID_DEPTH, true)`, the diverted spine, a different cross. NestedFull, MirrorNarrow, and Staircase are priced by their board columns, not by a tick gate pin on their cross.

(2) CliffFan's and CancellingChain's reasons say their pins are "absolute envelopes in the in-crate skyline and tier2 suites, not two-point bands in tests/meter.rs". Those suites hold both shapes only as corpus members (`assert_agreement` at skyline/tests.rs:514-533 checks length, validity, and round trip with no counter; the subadditivity grid at tier2/tests.rs:531). The actual two-point flatness pins are `accum_fan_touches_flat` and `accum_cancelling_touches_flat` in tests/meter.rs (6637, 6670), the file the reason says they are not in, and they drive a raw `suanpan::Accumulator` (`cancelling_run`, 6569-6585) rather than a `before` operation; the fan test calls `comb_run` and never builds `Shape::CliffFan` (finding 12).

(3) WideToothComb and JumpComb claim "a deliberate, documented internal-entry decision at the band's citation site". `rank_wide_tooth_run` and `rank_jump_run` call `meter::skyline::query::rank(meter::skyline::view(&enc))` and document the liveness floor and the answer pin, never the entry choice; `rank_weight_comb_run` (4522) measures the same kernel through public `v.rank()`. Doctrine: suites exercise the public API "except as deliberate, documented decisions at the check site"; the registry asserts documentation a reader cannot find.

Evidence:

      1108	/// The tick crosses' shared no-band reason.
      1109	const TICK_CROSS_UNBANDED: &str = "tick cross: the tick gate pins in tests/meter.rs price its walk";
      1629	                    reason: "kernel-seam probe measured through the internal skyline \
      1630	                             entries, which the board's public-operation rows cannot host \
      1631	                             — a deliberate, documented internal-entry decision at the \
      1632	                             band's citation site",
      1663	                    reason: "its pins are absolute envelopes in the in-crate skyline and \
      1664	                             tier2 suites, not two-point bands in tests/meter.rs",

    tests/meter.rs:
      2704	        let r = meter::skyline::query::rank(meter::skyline::view(&enc));
      6637	    fn accum_fan_touches_flat() {
      6638	        let small = comb_run(4_096, 50_000);

Resolution: give each family its own truthful reason. Nested-full, mirror-narrow, staircase: "priced by its board column; no tick gate pin exists on this cross" (or add the pins). Cliff-fan, cancelling-chain: name `accum_fan_touches_flat`/`accum_cancelling_touches_flat` and say they price the accumulator's stream, not a `before` operation (or resolve per finding 12). Wide-tooth, jump-comb: either route both runs through `Version::rank` like the weight-comb run and re-rule the coverage answer, or write the internal-entry decision and its reason at the two run fns and quote it. Add a registry-side check that every `reason` naming a test fn resolves (the parity scanner already reads tests/meter.rs). Acceptance: for every `reason` string naming a test, file, or module, a grep of the named site finds the named artifact.
Construction: `grep -nE 'Shape::Dense\.packed1.*NestedFullId|WideTail\.packed2\(1,|IdSpine\.packed_flagged\([A-Z_]+, false\)' crates/before/tests/meter.rs` inside `fn tick_*` bodies finds none; `grep -n 'CliffFan\|cliff_fan' crates/before/src/version/skyline crates/before/src/meter/tier2` finds corpora only; `awk 'NR>=2688&&NR<=2697' crates/before/tests/meter.rs` finds no internal-entry rationale.

### meter-registry-tier2-11: `FamilyId::ALL`, the 52-arm `index()`, and `ALL_SHAPES` are hand rosters checked only against each other; a variant absent from `ALL` reaches no instrument and `index()`'s doc claims the opposite
- Where: crates/before/src/meter/registry.rs:1115-1228 (related: registry.rs:569-573, 1232-1236; crates/before/src/meter/registry/tests.rs:8-13, 84-97; crates/before/tests/amp_board_smoke.rs:363; crates/before/tests/verdict_matrix.rs:468, 1296; crates/before/src/meter/board/family.rs:765-780; crates/before/src/meter/board/ops.rs:164-182)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -rn '\.index()'` over crates/before and before-fuelscape: readers only at registry/tests.rs:91, 94; every `FamilyId::ALL` reader listed by grep iterates the array (registry/tests.rs:89, 103, 118, 137, 195, 208; amp_board_smoke.rs:363; verdict_matrix.rs:468, 1296; `board()` at 1233); awk over the enum bodies counts 52 `FamilyId` and 68 `Shape` variants against `[FamilyId; 52]` and `[Shape; 68]`; strum 0.28 is in Cargo.lock (2377) only transitively); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: already known in part (b3f09baa wrote the "found by luck" concession at 572-573 and the `ALL_SHAPES` caveat at tests.rs:10-12 in the same commit that reworded `index()`'s doc to the opposite claim); no derive or macro was ever weighed
- Owner-gated: yes for the mechanism (`index()` is `pub const fn` under the `meter` feature and a derive adds a direct dependency); correcting the false sentence at 1170-1172 is unconditional

`index()`'s doc says the roster-order tie means "a variant cannot be declared without joining the roster at a committed position", but `roster_order_is_committed` iterates `FamilyId::ALL` only, so a variant with `spec()` and `index()` arms (and the compiler-forced or-pattern arms in board/family.rs and board/ops.rs) and no `ALL` entry passes every registry test and is never seen by the board, the band parity pin, or the verdict matrix, all of which iterate `ALL`. `ALL: [FamilyId; 52]` fixes its own length; `index()` has no reader but its pin; `ALL_SHAPES` in the tests has the same hole and admits it. Principle 6 (the module's stated invariant is that every family exists inside every instrument's coverage, yet the one enumeration every instrument derives from is a hand list nothing checks for completeness) and Principle 3 (`index()` exists to be checked against `ALL` and nothing else). The doctrine prefers a dependency over hand-rolling this.

Evidence:

      1115	    pub const ALL: [FamilyId; 52] = [
      1170	    /// This family's position in [`FamilyId::ALL`] — the roster-order tie the
      1171	    /// registry tests hold against the array, so a variant cannot be declared
      1172	    /// without joining the roster at a committed position.
      1173	    pub const fn index(self) -> usize {

    registry/tests.rs:
        10	/// Completeness rides the same review discipline as [`FamilyId::ALL`]: a
        11	/// variant missing here escapes only the citation pin, never the compiler ties
        12	/// (its constructor arm in `Shape::builder` is still forced).

Resolution: (1) unconditional: fix or delete the sentence at 1170-1172, and reduce `index()` to `self as usize` (both enums are fieldless with declaration order equal to roster order, verified) so `roster_order_is_committed` pins `ALL[i] as usize == i` without a 52-arm match. (2) owner-gated: derive variant enumeration (`#[derive(strum::VariantArray)]` on `FamilyId` and `Shape`, so `FamilyId::VARIANTS` replaces `ALL`, `Shape::VARIANTS` replaces `ALL_SHAPES`, `index()` and its pin dissolve, and the "found by luck" list at 572-573 loses its first entry), or a local declaration macro producing the enum and its array from one list if a new dependency is unwanted. Acceptance: appending a variant to either enum without touching any roster either fails to compile or fails a committed test that enumerates variants totally; `grep -n 'fn index' crates/before/src/meter/registry.rs` finds nothing (or the surviving pin is a name snapshot); registry/tests.rs no longer carries the completeness caveat.
Construction: add `FamilyId::Probe` after `LatentLadder` with `index() => 52`, a `spec()` arm copying LatentLadder's row with `name: "probe"` and `Bands::Unbanded`, and the or-pattern arms in board/family.rs:765-780 and board/ops.rs:164-182 extended; leave `ALL` untouched. `roster_order_is_committed`, `family_names_are_unique`, `every_shape_is_cited_by_a_family`, `board_roster_derives_from_coverage_answers`, `envelope_only_rulings_are_dated`, and `band_tests_and_registry_citations_stay_paired` all pass, and `FamilyId::board()` never yields the variant.

### meter-registry-tier2-12: The cliff-fan family prices a path-sum walk no production operation performs, and its only named pin re-runs the comb stream
- Where: crates/before/src/meter/registry.rs:1653-1669 (related: registry.rs:115-117, 897-899; crates/before/src/meter.rs:342-356; crates/before/src/version/skyline.rs:11-12, 119-121; crates/before/tests/meter.rs:6629-6641; crates/before/src/version/skyline/tests.rs:555-556; crates/before/src/meter/tier2/tests.rs:531)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (read the generator doc at meter.rs:342-356, skyline.rs:11-12 and 119-121, `accum_fan_touches_flat` at tests/meter.rs:6637-6641, and every `CliffFan` use in the skyline and tier2 suites by grep); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: deliberate but expired (2b0884c0 landed the fan for entry/exit path-sum accumulation under the min-lifted coding and reused the comb stream on purpose; faf3cd0a removed every production walk that maintains path sums; the registry row, written four days later, restated the pre-flag-day hazard)
- Owner-gated: yes (removing an instrument family and a public `Shape` variant)

The fan's adversarial content is entry/exit accumulation of per-node stored bases: the generator doc says "a walk that maintains a running path sum (enter: add the stored base; leave: subtract it) crosses the `2^k` carry boundary *twice per tooth*" and "Consecutive-leaf *values* stay cliff-free". The stored coding has no per-node numbers (skyline.rs:11-12: "Internal nodes carry no numbers") and the only walk that materializes path sums is the test- and meter-only transcoder (skyline.rs:119-121). The family's threat model ended at the flag day. Its row points at "the in-crate skyline and tier2 suites' pinned envelopes", which hold it only as a corpus member, and the one test bearing its name (`accum_fan_touches_flat`) calls `comb_run` twice and never builds `Shape::CliffFan`. Principle 3: machinery outlived the constraint that justified it; Principle 5: the row and the `FamilyId` doc ("`n` sibling carry excursions funded by one stored magnitude") describe a deleted coding's hazard.

Evidence:

      1653	            FamilyId::CliffFan => FamilySpec {
      1654	                name: "cliff-fan",
      1655	                shapes: &[Shape::CliffFan],
      1656	                coverage: Coverage::EnvelopeOnly {
      1657	                    reason: "kernel-seam probe: sibling carry excursions funded by one \
      1658	                             stored magnitude, priced by the in-crate skyline and tier2 \
      1659	                             suites' pinned envelopes",

    meter.rs:
       355	/// input. Consecutive-leaf *values* stay cliff-free (`2^k ↔ 2^k + 1`): the fan
       356	/// prices entry/exit accumulation, the boundary comb prices leaf deltas.

    tests/meter.rs:
      6637	    fn accum_fan_touches_flat() {
      6638	        let small = comb_run(4_096, 50_000);
      6639	        let large = comb_run(8_192, 100_000);
      6640	        assert_flat("fan", &small, &large, envelope::COMB_MILLI_PER_DELTA);

Resolution: owner's call between two truthful states. (a) Dissolve the family: remove `FamilyId::CliffFan`, `Shape::CliffFan`, the `cliff_fan` generator, its size pin, and `accum_fan_touches_flat`; keep the shape in the agreement corpora only if it is re-justified as a coding corpus member rather than an adversary. (b) Re-justify it against a production walk that exists and add a two-point band built through `Shape::CliffFan` on that kernel, cited from the family's `Bands::Priced`, with a known-bad kernel that fails it. Either way the `EnvelopeOnly`/`Unbanded` reasons must name what exists (finding 10). Acceptance: either `grep -rn CliffFan crates/before` is empty, or a convention-named flatness test builds `Shape::CliffFan` at two scales over a public or documented-internal operation and the parity test passes.

### meter-registry-tier2-13: A tautological board-roster equality and a weak, duplicated date predicate with an under-stated doc
- Where: crates/before/src/meter/registry/tests.rs:176-202 (related: registry/tests.rs:204-226; crates/before/src/meter/registry.rs:1232-1236; crates/before/surfacecheck/src/check.rs:259-273)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read `board()` at registry.rs:1232-1236 and both tests); executed: no
- Seen by: adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: born tautological at cc84df7e (`board()` was already the same filter); surfacecheck's stricter date check landed the next day and was never back-ported
- Owner-gated: no

`board_roster_derives_from_coverage_answers` compares `FamilyId::board().count()` against `ALL.iter().filter(|f| matches!(f.spec().coverage, Coverage::Board { .. })).count()`, which is the literal body of `board()`; the `EnvelopeOnly` panic arm inside the loop is unreachable by that filter. The live assertions are `cells > 0` per column and `columns > 0`. In `envelope_only_rulings_are_dated` the date predicate is written twice (215, 221), admits any ten characters with two dashes, and the doc ("Every envelope-only ruling carries a dated, non-empty reason") omits the `Bands::Unbanded` clause the body also checks. Doctrine: recompute-and-compare on a pure function with the same expression is not defense in depth; a test doc must state what the body checks.

Evidence:

       193	    assert_eq!(
       194	        columns,
       195	        FamilyId::ALL
       196	            .iter()
       197	            .filter(|f| matches!(f.spec().coverage, Coverage::Board { .. }))
       198	            .count(),
       199	        "the board roster and the coverage answers disagree"
       200	    );

Resolution: rename the first test to what it holds (`board_columns_declare_nonzero_reach`), drop the equality and the unreachable arm; extract one `is_iso_date` predicate (or lift surfacecheck's month/day check) and fix the second test's doc to name both ruling kinds. If finding 9 resolves by deleting `decided`, the date test goes with it. Acceptance: no assertion in registry/tests.rs restates a registry function's body; one date predicate; each test doc matches its body.

### meter-registry-tier2-14: `tier2.rs` describes the stored coding as a candidate and the deleted construction-language coding as "today's"; the `Tier2Size` formula is wrong against `encoded_bits`
- Where: crates/before/src/meter/tier2.rs:1-32 (related: tier2.rs:53-56, 73, 89-93, 104-120; crates/before/src/meter/tier2/tests.rs:5-7, 106, 121, 197, 231-236, 705-709; crates/before/src/version.rs:1151-1153; crates/before/src/version/skyline.rs:26-27, 114-129; crates/before/src/version/skyline/tests.rs:3-7; crates/before/src/testing/compactness.rs:1-9, 78-80 (out of partition))
- Class / severity / confidence: documentation / high / high
- Provenance: verified (read version.rs:1151-1153 (`encoded_bits` is `self.0.len()`, the stored skyline length), skyline.rs:26-27 ("This coding is the stored and wire form"), tests.rs:145 (asserts `v.encoded_bits() == 11` with the message "the stored coding is Tier 2 itself"), compactness.rs:78-80 and :97 (computes the stored-base bits from `packed.len()`, not `encoded_bits`); `grep -rnE 'Tier [0-9]'` over crates/before/src and crates/suanpan/src finds only "Tier 2"; `git log -- crates/before/src/meter/tier2.rs`: 3e1a1631 create (2026-07-23), faf3cd0a flag day (2026-07-25), b3f09baa docs WIP, 5d167a63); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed, plus two new sites (the `expect`/`assert` messages at 73 and 89-93 still say "canonical Version" though the argument has been a construction-language bit view since faf3cd0a); history: contradicts the root AGENTS.md hard rule (nothing refers to code that no longer exists); the framing was written while the decision was pending, faf3cd0a changed only intra-doc links and the input type in tier2.rs and partially re-denominated tier2/tests.rs, and b3f09baa re-wrapped the lines
- Owner-gated: no for the prose correction toward the code (sanctioned); renaming the module (public under the `meter` feature) is the owner's

The module doc says `tier2_size` computes what a Version "would have" under the Tier 2 coding "if re-encoded", that its topology bits are "exactly as today", that "The compactness ratio between this size and today's encoded size is the evidence the representation decision turns on", and that deltas are gamma-coded "exactly like today's stored bases". The `Tier2Size` doc says "today's encoded size is `nodes` flag bits plus its stored gamma codes, so the stored-base code bits are exactly `encoded_bits - nodes`". The stored coding has been the skyline coding, which is Tier 2, since faf3cd0a; `Version::encoded_bits()` is the stored skyline length, so `encoded_bits - nodes` is the first-leaf plus delta bits, not any stored-base bits (compactness.rs:97 computes the quantity correctly from `packed.len()`). A maintainer reading this module alone is told a decision is pending that was made in July and that the production coding is the one that was deleted. The module also never states its live role, the independent second implementation of the coding that every emitted stream's length is pinned against (skyline.rs:125-129; skyline/tests.rs:3-7), so its duplicate `zigzag`/`gamma_bits` read as accidental. The `# Panics` input description ("the packed form") is ambiguous now that the stored form is also a packed bit stream: `tier2_size` reads one gamma base per node and panics (`BitsView::bit` asserts at bits.rs:309, `decode_int` at 73) on a stored skyline stream. "Tier 2" is a design-note tier label with no Tier 1 anywhere in the tree. The same drift runs through the tests: "the hand-derived current bit count" (5-7), `single_small_leaf_matches_current_size`, `single_big_leaf_matches_current_size`, `cliff_comb_tier2_size_is_linear_while_current_is_quadratic` (where "current" is `packed.bits`, the construction-language length), "Under today's coding the same tree pays `2k + 1` stored bits per crossing" (231-233; under the stored coding it pays 3, which is the point of 188-190), and "10 bits against today's 14" (705-706).

Evidence:

         1	//! The exact encoded size a [`Version`](crate::Version) would have under the
         2	//! Tier 2 coding: preorder topology bits plus delta-coded absolute leaf values.
         4	//! A measurement tool, not a codec: [`tier2_size`] computes, bit-exactly, how
         5	//! large a canonical [`Version`](crate::Version) would be if re-encoded as its
         6	//! preorder topology (one flag bit per node, exactly as today) plus its leaf
        10	//! compactness ratio between this size and today's encoded size is the evidence
        11	//! the representation decision turns on, so the walk here is written for
        29	/// leaf. The parts are exposed separately so the compactness suite can charge
        30	/// the delta stream against today's stored bases (today's encoded size is
        31	/// `nodes` flag bits plus its stored gamma codes, so the stored-base code bits
        32	/// are exactly `encoded_bits - nodes`).
        73	        let (base, next) = codec::decode_int(bits, pos).expect("canonical Version parses cleanly");
        92	        "canonical Version walk consumes every packed bit"

    tier2/tests.rs:
       145	    assert_eq!(v.encoded_bits(), 11, "the stored coding is Tier 2 itself");
       197	fn cliff_comb_tier2_size_is_linear_while_current_is_quadratic() {
       231	/// `Θ(W²)` total in wire bits `W`. Under today's coding the same tree pays
       232	/// `2k + 1` stored bits per crossing (the envelope suite pins those operations

Resolution: rewrite against today's code. The module is the independent sizer of the stored skyline coding, computed from the construction-language stream (the min-lifted packed preorder form `meter::Packed::as_bits` and `testing::bridge::packed_bits_of` produce), never from a stored stream; list the coding's terms; name its consumers (length agreement in `version/skyline/tests.rs`, the kernel emission length pins in this module's tests, the plain-sweep pin, and the compactness ratio against the construction-language size `Packed::bits` if that suite stays); state the independence rule beside `zigzag`/`gamma_bits` ("re-derived here on purpose: sharing them with `version::skyline` would make length agreement check nothing"). In `Tier2Size`, replace "today's encoded size ... `encoded_bits - nodes`" with "the construction-language size (`Packed::bits`) minus `nodes`". Fix the `expect`/`assert` messages at 73 and 92 to name the construction-language stream. In the tests, rename `*_matches_current_size` to `*_matches_packed_spelling_size` (the term the file already uses at 118, 135, 163, 183) and `..._while_current_is_quadratic` to `..._while_the_packed_spelling_is_quadratic`; re-state 5-7, 231-233, 705-709; end 236 by naming that the production validator's `suanpan::Accumulator` is the cliff-free design (skyline.rs:102-110). Owner-gated suggestion: rename the module (`meter::sizer` or `meter::skyline_size`) and `Tier2Size` with it. Acceptance: `grep -nE "today|current|would (have|be)|representation decision|canonical Version" crates/before/src/meter/tier2.rs crates/before/src/meter/tier2/tests.rs` returns only lines whose referent is unambiguous and true of the stored coding; the `Tier2Size` doc names `Packed::bits`; the independence rule appears beside the duplicate helpers.

### meter-registry-tier2-15: The sizer suite builds shapes through the raw generators, bypassing the registry door
- Where: crates/before/src/meter/tier2/tests.rs:15-17 (related: tier2/tests.rs:123, 166, 199, 314-317, 526-533, 557, 571; crates/before/src/meter/board/tests.rs:54, 80, 111-113, 162-163, 212-213, 353-358; crates/before/src/meter/registry.rs:12-21, 302-308)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (read the import and call sites; `grep -nE '\b(dense|bigroot|hugeleaf|cliff_comb|...)\('` in board/tests.rs shows the same pattern there); executed: no
- Seen by: structure-prose; refutation: reframed (board/tests.rs, also a child of `meter`, does the same, so the registry doc's "the module's own unit tests" is ambiguous about child suites rather than false); history: 608dea84 routed fifteen files through `Shape` and did not touch tier2/tests.rs; no recorded reason
- Owner-gated: no

Two child suites of `meter` (`tier2/tests.rs` and `board/tests.rs`) import and call the private generators directly, which visibility allows only because they are descendants of `meter`. The registry doc says `Shape::builder` is "the constructors' only caller outside the module's own unit tests" and that no "kernel unit test" can build a shape except through the registry. The sizer and board suites are sibling instruments' tests, not the generators' unit tests, so the door's universality claim is true only under a generous reading. Consistency with the registry invariant: a reader auditing who can build an unregistered shape finds two exceptions the wording does not anticipate.

Evidence:

        15	use crate::meter::{
        16	    alt_spine, bigroot, cancelling_chain, cliff_comb, cliff_fan, dense, hugeleaf, wide_tooth_comb,
        17	};

Resolution: route both suites through `Shape` (`Shape::Dense.packed1(512)`, `Shape::Bigroot.packed2(200, 100)`, `Shape::Hugeleaf.packed1(500)`, `Shape::CliffComb.packed2(64, 64)`, and so on), after which registry.rs:14-15 is exactly true; or name the child suites in the registry doc as the sanctioned exception. Acceptance: no generator name is imported into tier2/tests.rs or board/tests.rs, or the registry doc names them.

### meter-registry-tier2-16: The 1-Lipschitz coding pin is implied pointwise by the subadditivity pin over the same emitters and populations; its leaf clause asserts a count, not containment
- Where: crates/before/src/meter/tier2/tests.rs:27-80 (related: tier2/tests.rs:271-324, 326-373, 417-453, 526-533; crates/before/src/meter/board/cell.rs:72-75)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (read both checks and every test that calls them; the implication is arithmetic: `so.total + 2 <= sa.total + sb.total` implies `so.total <= sa.total + sb.total + 4·(sa.leaves + sb.leaves)` for any leaf counts; cell.rs:72-75 cites the Lipschitz pin by name); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed with one correction (the Lipschitz adversarial grid at 313-318 runs larger operands than the subadditivity grid at 526-533, so the merge must carry them) and one new point (the leaf clause); history: deliberate but expired (the Lipschitz pin landed at 6814d77b as the board's denomination premise; the subadditivity lemma landed four hours later at eec23b79 and was ruled the lemma of record; no reason to keep the weaker clause is recorded)
- Owner-gated: no

`check_join_meet_lipschitz` asserts `so.total_bits <= sa.total_bits + sb.total_bits + 4·(sa.leaves + sb.leaves)`; `check_subadditive` asserts `so.total_bits + 2 <= sa.total_bits + sb.total_bits` under the same `EMITTERS` table (54, 450). Every operand pair passing the second passes the first, and the four `*_hold_the_lipschitz_pin` tests run the same three proptest strategies as the subadditivity tests plus a four-shape grid. The only content the Lipschitz helper adds is the leaf clause at 59-66, and that clause asserts `so.leaves < sa.leaves + sb.leaves` while its doc (44) and message (61-62) speak of boundaries "contained in the union of the inputs'": a count inequality is a consequence of containment, not containment (an output that drops one union boundary and adds one foreign boundary passes). Principle 3: a check whose every failure is already a failure of a stronger committed check catches nothing on its own; two derived constants for one lemma is the maintenance cascade the doctrine names. Two structural leftovers vanish with the merge: `EMITTERS` and the emitter fns sit under the subadditivity rule (326) yet are consumed at 54, and `operator_meet`'s doc (369-370) copies `operator_join`'s "first emitter of record" while both docs scope the emitters to "the subadditivity pins" only.

Evidence:

        37	const JOIN_MEET_BOUNDARY_SLACK_BITS: u64 = 4;
        59	        assert!(
        60	            so.leaves < sa.leaves + sb.leaves,
        61	            "{name}: {} output leaves reach the input leaf total {} + {}: \
        62	             an output boundary appeared outside the inputs' boundaries",
        67	        let ceiling =
        68	            sa.total_bits + sb.total_bits + JOIN_MEET_BOUNDARY_SLACK_BITS * (sa.leaves + sb.leaves);
       346	const JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS: u64 = 2;
       437	    assert!(
       438	        so.total_bits + JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS <= sa.total_bits + sb.total_bits,
       369	/// The public `&` operator: the subadditivity pins' first emitter of record

Resolution: move the leaf clause into `check_subadditive` and state it as what it is (a leaf-count bound; or check containment by comparing boundary positions through the oracle's dyadic intervals if containment is the claim the board's denomination rests on); rename the merged helper to state both clauses; delete `JOIN_MEET_BOUNDARY_SLACK_BITS`, its derivation, `check_join_meet_lipschitz`, and the four Lipschitz tests; carry the Lipschitz grid's larger operands (dense(512), bigroot(200, 100), hugeleaf(500), cliff_comb(64, 64)) into `adversarial_crosses_hold_subadditivity`; move the sentence "the statement the board's input denomination of the packed-output mutators rests on" onto the subadditivity constant's doc; re-point cell.rs:72-75 and this file's module doc (1-3) to the subadditivity pin; fix the two emitter docs. Acceptance: one coding-lemma helper and one constant; four fewer tests; `grep -n Lipschitz crates/before/src` finds no referent in meter or board; the leaf clause survives inside the merged check and its doc matches its assertion.
Construction: dominance needs no run. To confirm the moved leaf clause is live, weaken it to `<=` in a scratch build and observe `empty_pair_is_the_subadditivity_equality_case`'s operands (1 + 1 leaves, output 1 leaf) still pass while `so.leaves == sa.leaves + sb.leaves - 1` cases distinguish `<` from `<=`; restore.

### meter-registry-tier2-17: The sizer-of-a-Version idiom is copied fifteen times; `built_view` and `dense` are fully qualified beside imported siblings; the two kernel wrappers are a copy-paste pair
- Where: crates/before/src/meter/tier2/tests.rs:48-58 (related: tier2/tests.rs:14, 16, 166, 380-415)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (`grep -c 'built_view(&packed_bits_of('` = 15 and `grep -c 'crate::codec::built_view'` = 21 in the file; `Base` imported from `crate::codec` at 14 and `dense` at 16 while 166 spells `crate::meter::dense(2)`; `skyline_join`/`skyline_meet` at 380-415 differ only in kernel and message); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed; history: two mechanical substitutions (faf3cd0a, 5d167a63) with no helper introduced
- Owner-gated: no

The one operation the file is about is buried in a four-deep bridge-lowering incantation at every site. Legibility and imports over long qualified paths.

Evidence:

        48	    let sa = tier2_size(crate::codec::built_view(&packed_bits_of(
        49	        &to_oracle_version(a),
        50	    )));
        51	    let sb = tier2_size(crate::codec::built_view(&packed_bits_of(
        52	        &to_oracle_version(b),
        53	    )));

Resolution: add `fn size_of(v: &Version) -> Tier2Size { tier2_size(built_view(&packed_bits_of(&to_oracle_version(v)))) }` with `use crate::codec::built_view;`, use it everywhere, use the imported `dense` at 166, merge the two `crate::testing::bridge::` imports (18-19), and collapse `skyline_join`/`skyline_meet` into one `kernel_emit(emit, a, b, what)` with two one-line wrappers. Acceptance: `grep -c 'built_view(&packed_bits_of(' tier2/tests.rs` is 1; no `crate::codec::built_view` or `crate::meter::dense` spelling remains.

### meter-registry-tier2-18: The ratio-floor loop divides two closed-form literals already asserted; it tests arithmetic, not code
- Where: crates/before/src/meter/tier2/tests.rs:213-220 (related: tier2/tests.rs:193-195, 198-212)
- Class / severity / confidence: test-quality / nit / high
- Provenance: assessed (read); executed: no
- Seen by: adequacy; refutation: confirmed; history: deliberate but expired (7c367719 landed it as adoption-decision evidence; faf3cd0a made the decision)
- Owner-gated: no

The first loop pins `tier2_size(...)` to `10n + 4k + 2` and `packed.bits` to `n(2k + 10) + 2`; the second computes both from the same formulas and asserts their quotient exceeds a literal, so no measured quantity enters. The three hand-computed ratios in the doc (194) are derivable from the formulas. Principle 3: an assertion over constants cannot fail unless the constants are edited.

Evidence:

       213	    for (n, ratio_floor) in [(64, 9.83), (1024, 146.97), (4096, 585.83)] {
       214	        let current = (n * (2 * n + 10) + 2) as f64;
       215	        let tier2 = (14 * n + 2) as f64;
       216	        assert!(
       217	            current / tier2 >= ratio_floor,

Resolution: delete the second loop and the three ratios in the doc; the closed forms state the unbounded-ratio claim by themselves. Acceptance: the test asserts only measured `tier2_size` and `packed.bits` values against closed forms.

### meter-registry-tier2-19: Lib-side tests reset and read the process-global limb counter with nothing enforcing the one-scenario-per-process premise
- Where: crates/before/src/meter/tier2/tests.rs:247-253 (related: crates/before/src/codec/base/limb_meter.rs:16-19, 31; crates/before/tests/meter.rs:354; the other reset sites: meter.rs, party/tests.rs, codec/dsi/tests.rs, testing/asymptotics.rs, version/skyline/query/tests.rs, meter/tests.rs, meter/board/tests.rs, meter/board/measure.rs)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read limb_meter.rs:1-52: `static LIMB_OPS: AtomicU64` with `Relaxed` ordering and the premise stated at 16-19; `grep -rn NEXTEST crates tools justfile .github .config` finds no guard; `grep -rln 'reset_limb_ops\|reset_touches\|reset_scan'` lists nine lib-side files; only tests/meter.rs carries `ISOLATION_NOTE`); executed: no
- Seen by: instrument-correctness; refutation: reframed (a crate-level observation; this test is one instance); history: the premise is deliberate (limb_meter.rs) and 380d470ea established the isolation-note convention for tests/meter.rs and src/meter/tests.rs; the plain-sweep test landed hours later without it
- Owner-gated: yes (where the premise is enforced, a runtime guard on the runner or an AGENTS.md rule, is a crate-level test-policy decision)

`LIMB_OPS` is process-global and its doc premises that "the metering binaries run one scenario per process"; the plain-sweep witness resets and reads it inside the `before` lib test binary. Under nextest (the configured runner) the premise holds; under `cargo test` any concurrent `Base` arithmetic bleeds into the delta and the 1.8 ratio can move either way, and for the ceiling-style pins elsewhere a dark counter can be masked by another thread's work. The premise is stated but nowhere enforced, and the lib-side sites carry no isolation note.

Evidence:

       247	        crate::meter::reset_limb_ops();
       248	        for i in 1..(2 * n) {
       249	            v = if i % 2 == 1 { &v + &one } else { v - &one };
       250	        }
       251	        let closing = Base::from(1u8) << k as u32;
       252	        v -= &closing;
       253	        let ops = crate::meter::limb_ops();

    limb_meter.rs:
        16	//! envelopes read continuously across that arm seam. Relaxed ordering
        17	//! suffices: the metering binaries run one
        18	//! scenario per process and read the counters only after the metered call
        19	//! returns.

Resolution: add a `meter::require_process_isolation()` that asserts `NEXTEST_EXECUTION_MODE == "process-per-test"` (nextest exports it) with the isolation note as its message, called from the reset entries under `cfg(test)`; or name the premise as a hard rule in `crates/before/AGENTS.md` and accept the unguarded state explicitly. Acceptance: `cargo test -p before --lib --features limb-meter` fails fast with the isolation message at the first counter reset, while `just test-all` is unaffected; or the AGENTS.md rule exists.
Construction: run `cargo test -p before --lib --features limb-meter -- cliff_comb_plain_delta_sweep skyline` with default test threads; `Base` arithmetic from the skyline tests lands between `reset_limb_ops()` and `limb_ops()`, and the measured `small`/`large` vary run to run.

### meter-registry-tier2-20: The plain-sweep witness's 1.8 floor is underived and sits 0.04 above the deterministic reading; setup runs inside the metered region
- Where: crates/before/src/meter/tier2/tests.rs:261-268 (related: tier2/tests.rs:243-260; crates/before/src/codec/base/limb_metered.rs:8-13, 49-54; crates/before/src/codec/base.rs:67-69, 333-339, 416-429, 439-445)
- Class / severity / confidence: test-quality / nit / medium
- Provenance: verified by hand re-derivation against the metering impls (not executed): `meter_limbs2` records `a.limbs() + b.limbs()`, `meter_limbs_shl` records `limbs + rhs/64 + 1`, `limbs()` is `bits.div_ceil(64).max(1)`, `SubAssign` clones then subtracts; at k = n = 512 the value alternates 8 and 9 limbs, so 512 adds record 9 and 511 subs record 10 (9718), the closing shift 10 and the closing sub 18, total 9746 over 7170 wire bits (1.359); at 1024: 1024·17 + 1023·18 + 18 + 34 = 35874 over 14338 (2.502); ratio 1.841; executed: no
- Seen by: instrument-correctness; refutation: confirmed (same derivation); history: 7c367719 introduced `>= 1.8` with only "roughly doubles" and wall-clock probe figures
- Owner-gated: no

The doc says the per-wire-bit cost "roughly doubles"; the asymptote is 2 but the finite-scale ratio has the shape `(2m + 3)/(m + 3)`, and the committed floor holds by 0.04 under the current scalar-operand recording convention (a `max(a, b)` convention would read 2.0; a constant-2-per-op convention about 1.76 and fail for a reason unrelated to the claim). `closing` is constructed after `reset_limb_ops()`, so its shift is charged to the delta stream. Principle 2: a number in an enforced place needs its derivation; the test is deterministic, so this is an undocumented margin, not flakiness.

Evidence:

       261	    let small = limb_ops_per_wire_bit(512);
       262	    let large = limb_ops_per_wire_bit(1024);
       263	    assert!(
       264	        large / small >= 1.8,
       265	        "per-wire-bit limb cost must roughly double per size doubling \

Resolution: state the derivation beside the floor (or compute the expected ratio from `k/64` in the test and assert within a stated band), and hoist `let closing = ...` above `reset_limb_ops()` so the metered region is exactly the delta stream. Acceptance: the floor's origin is stated or computed; `closing` is built before the reset.

### meter-registry-tier2-21: `grid_version::build` recurses on depth outside the `recurse.rs` inventory of test-local recursive witnesses
- Where: crates/before/src/meter/tier2/tests.rs:468-484 (related: crates/before/src/recurse.rs:9-14; crates/before/AGENTS.md:32-36; `descend!` sites: testing/bridge.rs:56-57, 150-151; version/skyline/grow/tests.rs:125, 172, 177; meter/tests.rs:417)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (read recurse.rs:1-20 and AGENTS.md:26-37; `grep -rn 'descend!('` over crates/before/src lists the bridge, the grow suite's probe, and the meter suite's dive, matching the inventory; `build` at 469-477 recurses without `descend!` and is not listed); executed: no
- Seen by: adequacy; refutation: confirmed; history: contradicts the letter of the AGENTS.md hard rule (a walk that must recurse routes through `descend!`; the inventory in recurse.rs names today's recursive surfaces); `build` (eec23b79) predates the inventory (1ddb5a48) by eight days
- Owner-gated: no

The grid builder recurses on `log2(values.len())` without `descend!`, and recurse.rs's inventory (which AGENTS.md points at as holding "the inventory and the keep decision") names only the bridge, the grow suite's reference cost probe, and the meter suite's segment-liveness dive. Depth is bounded by the tests' own constants (at most 64 cells, depth 6), so the rule's goal is met; the inventory is a hand-maintained enumeration that has drifted (Principle 5), and the rule's letter is not followed at this site.

Evidence:

       466	/// canonical whatever the values. Recursive over the grid's `O(log)` depth
       467	/// (test-only; the measured paths are iterative).
       468	fn grid_version(values: &[Base]) -> Version {
       469	    fn build(values: &[Base]) -> oracle::Version {
       470	        match values {
       471	            [v] => oracle::Version::leaf(v.clone()),
       472	            _ => {
       473	                let (l, r) = values.split_at(values.len() / 2);
       474	                oracle::Version::node(0u64, build(l), build(r))

Resolution: build the grid iteratively by pairing bottom-up (which removes the recursion and the inventory question), or route the two recursive calls through `descend!` and add the site to recurse.rs's inventory with the bounded-depth note. Acceptance: recurse.rs's inventory names every test-local recursive fn, or `build` is iterative.

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
