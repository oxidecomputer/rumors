#!/usr/bin/env python3
"""Generate the instrument rescue's merged tables from one master list.

Each record is one instrument, listed in overall rank order. The category
tables (sections 10 to 16) and the overall ranking are both generated from
this list, so a position can never disagree between them.
"""
import collections
import sys

SEC = {
    "01": "01-identity.md", "02": "02-algebra.md", "03": "03-events.md",
    "04": "04-measures.md", "05": "05-spans.md", "06": "06-codecs.md",
    "07": "07-suanpan.md", "08": "08-adequacy.md",
    "09": "09-build-and-review-probes.md",
}

CATS = collections.OrderedDict([
    ("GEN", ("10-generators.md", "Generators and reach")),
    ("ORA", ("11-oracles.md", "Independent models and oracles")),
    ("PROP", ("12-properties.md", "Properties and predicates")),
    ("COST", ("13-cost.md", "Cost and resource instruments")),
    ("TGT", ("14-target.md", "Target-dependence instruments")),
    ("META", ("15-meta.md", "Diagnostics and meta-instruments")),
    ("PROBE", ("16-probes.md", "Probes and one-off tools")),
])

# (id, category, name, adds, evidence, fold-in, depends, reason)
R = []
def r(*a):
    assert len(a) == 8, a[0]
    R.append(a)

# ---------------------------------------------------------------- tier 1
r("05.4", "PROP", "Exhaustive boolean cube (`l5_exhaustive_boolean_cube`)",
  "Total exact coverage on a 4-cell lattice (antichains of 6, against 3 on the committed two-party grid): 921,456 verdicts, 11,865 decided only by refinement, against 508 on #61's grid.",
  "Caught nothing; never run against a mutant; passed on #61's parent and tip.",
  "Small: the cube is closed under join and meet, so the committed grid test's brute force serves unchanged. 7.2 s.",
  "None.",
  "It checks exact coverage totally and deterministically on a lattice twice as wide as the committed grid, with no new oracle, in about 7 seconds.")
r("09.A1", "META", "Exhaustive agreement test for the laws' multiplicity comparison",
  "Every family of at most three parties of depth at most 2 (3,616 families), checked against the tree oracle; #81's sampled property passes a wrong comparison at 4,096 cases.",
  "Fails every comparison mutant the reviewer tried; 0.42 s unmutated.",
  "Apply one written diff to #81's tests. 0.42 s.",
  "#81 (on #40).",
  "Written and half a second long, it closes a measured gap in the comparison every multiplicity law rests on.")
r("04.5", "ORA", "Rational `Rank` oracle `Q` with `render`, `arb_q`, `arb_related_q`",
  "Integral ranks 33%, even integers 24.5%, zero 6.8%, equal-value pairs 30%, against 0 for the committed `RANK_TRIPLE` driver; text rendered without production code.",
  "Its property caught R2 once the same-class mode existed (the committed suite also does).",
  "About 60 lines of oracle into `testing::oracles`, generators into `testing::generators`, and a second `RANK_TRIPLE` driver. Under 1 s.",
  "None (it would also drive #83's new law).",
  "About 60 lines and a second driver give all 15 rank laws the integers, zero, and equal values they never see.")
r("09.A2", "PROP", "Trait-impl laws that #83 narrowed out (`d653bb83`)",
  "Registry laws for every `Count` and `Rank` addition spelling, every `OwnVersion` comparison cell, span and clock operator cells, and shape `size_hint` brackets; their constant-body mutants passed every suite at the base.",
  "13 of 28 tests fail under ten mutants applied together (not attributed per mutant).",
  "Revert chosen hunks of `735b6b2e`; the laws ride the existing drivers. Runtime unmeasured.",
  "#83; `archive/rescue-d653bb83` keeps the commit if #83 is squashed.",
  "Reverting part of one archived commit restores checks on spellings the campaign showed survive as constant bodies.")
r("04.1", "GEN", "Multi-scale partition generator with related-pair modes (`arb_leaves`, `arb_related`)",
  "Sends arbitrary versions through the integrator's deferral path (50.8% of pairs, against 0 committed) and exact rank ties at freezing scale (29.1%, against 0); depth to 130, heights to 2,209 bits (a deep setting reaches about 3,000 levels).",
  "Its harness needed the deep prefix to catch M25; no defect.",
  "Strategies into `testing::generators` as a third input for the law drivers and the measure tests, with no new oracle; 3.2 s per 256 cases. The deep setting needs an explicit-stack thread (ruling 68).",
  "None (the deep setting waits on the stack-guard removal).",
  "As a third input to the existing law drivers, it puts every committed version law on the deferral and exact-tie paths no driver reaches.")
r("01.1", "GEN", "Party generators `arb_set`, `arb_disjoint`",
  "Parties to depth 80 (above 32 in 47%), 17 or more two-child branches in 42%, and accepted `join_all` families of up to 10 arbitrary parties (the committed families accept none above one item).",
  "No unique catch.",
  "About 90 lines of builder into `testing::generators`, returning `tree::Party`; the function-space leg must cap depth with `arb_set_upto`. 2.8 s per 4,000 draws.",
  "None.",
  "A small builder moves every party law and tree-oracle comparison from depth 4 to depth 80 and supplies the only large accepted families.")
r("03.2", "PROP", "`min_ticks` bounded by each clock's causal past",
  "Bounds each clock's `min_ticks` by its own causal past at every step, where the committed `min_ticks_floors_every_history` bounds final clocks by the whole history's total: about 40 times as many tight observations per committed history; its own histories reach counts of `2^64` and absorbs.",
  "Fails M22 (as does the committed floor); 100,000 cases pass.",
  "A small change to the committed test (a per-clock ledger over `world_strategy`). Wide counts and foreign ticks need a production-only world. 0.59 s.",
  "None; the foreign-tick step is an owner question.",
  "A small change to one committed test multiplies its chances of seeing an overcount about fortyfold.")
r("05.6", "PROP", "Span-to-query coverage agreement (`l5_span_query_agree`)",
  "A predicate nothing committed states: a segment query's coverage of another span is `Full`, `Empty`, or `Partial` as containment and intersection say.",
  "M3, M8, M9, M13 (each also caught by committed tests).",
  "One law in the `version_triple` group over existing candidates; no model. 0.73 s as written.",
  "None.",
  "One law with no model states a relation between two public APIs that nothing checks.")
r("03.1", "PROP", "`min_ticks` attained by a constructed history",
  "`min_ticks` is attained on versions with more than one region, and every sampled canonical version is reached by a rule-respecting, single-seed history; spine variant near depth 50 and heights near `2^201`.",
  "Fails M22, M23 (committed also), M7, M11, M12; 100,000 cases pass.",
  "Beside `min_ticks_floors_every_history`; `realize` recurses on generator-bounded depth. 0.16 s and 1.60 s.",
  "None (the spine variant's generator comes from #74).",
  "Two cheap properties state attainment and reachability, which no committed test asks.")
r("08.3", "PROP", "Gamma-window counterexample test (also 09.A23)",
  "The window's general contract on 59- to 63-bit codes at unaligned starts (42 codes); committed window tests use position 0 or a two-bit stream.",
  "Kills the three `gamma/window.rs:47` survivors (39, 26, 20 wrong); they are unreachable from production today.",
  "Move into `bits/tests.rs` and restate its doc. 0.05 s.",
  "None.",
  "A finished 50-millisecond test guards the window's contract before any caller relies on it.")
r("04.2", "COST", "`Rank` sum-order cost probe",
  "Exercises the adversarial summand order of closed fix `rank-20`, whose committed meter the consolidation deleted (`193a14744`) and which the board's `rank_sum` row cannot see; the mutant E8 would read about 11 to 85 touches per byte against 0.5 today (inferred).",
  "Caught nothing; never run against E8.",
  "A focused check in `tests/meter/` with a ceiling from the mechanism and a floor; E8 must be rebuilt as its known-bad. 0.32 s.",
  "None.",
  "A 0.3-second check restores coverage a closed fix lost, which no current meter can see.")
r("09.A3", "COST", "Touch-bound worst-case families (F1 to F7)",
  "Fixed programs reach 0.58 of #37's derived bound where #37's random programs reach 0.17, so a uniform 1.7-fold touch rise fails them where random programs need about 5.9.",
  "Caught nothing; measures reach; the reviewer reproduced 0.5813.",
  "`Vec<Step>` values judged by #37's `check_program`. 0.07 s.",
  "#37.",
  "Fixed worst cases make #37's property fail near its derivation, for 70 milliseconds.")
r("03.3", "PROP", "Tick claims #74 did not carry",
  "Two laws no registry states (`min_ticks` rises at most one per tick, and at most `a + b` under `ticks(a + b)`), plus strict domination, the output envelope, and locality on co-generated regimes.",
  "None attributable; passes 100,000 bushy and 30,000 spine cases.",
  "Five assertions in #74's `assert_co_generated`, or co-generated cases as a third `VERSION_PARTY` driver with two new laws. Negligible.",
  "#74.",
  "A few lines add two missing laws and run three existing ones where the tick walk's state is richest.")
r("02.3", "GEN", "Deep random partitions (`gen::partition`, `limits`)",
  "Random topology to depth 300 and 1,034 leaves (median depth 12), where committed random versions stop at depth 4 and 11 leaves.",
  "No unique catch.",
  "About 45 lines as a `prop_flat_map` strategy, with a size budget per component (the tape starves later draws) and a census floor.",
  "None.",
  "It is the cheapest built way to give the algebra's committed properties random topology past depth 4.")
r("04.8", "PROP", "`Count` boundary sweep",
  "Every conversion boundary (`MAX - 1`, `MAX`, `MAX + 1` of each integer type) judged by an independent `BigUint` comparison; the committed property shares its oracle with the code and cannot draw most of these values.",
  "Caught nothing.",
  "Drops into `count/tests.rs` as is. 0.05 s.",
  "None.",
  "A 50-millisecond exhaustive test checks what the committed property can neither reach nor judge independently.")
r("08.6", "TGT", "`usize` signature census (`usize_census.py`)",
  "Lists every public item whose signature mentions `usize` (30, of which 9 are production API); nothing committed reads signature types, so a new `usize` parameter passes unreported.",
  "Found no offending parameter.",
  "Port 61 lines of Python to a Rust census in `surfacecheck`, reconciled both ways. Negligible runtime.",
  "None.",
  "It turns your `usize`-invariance rule, which nothing checks, into a reviewed event for every new `usize` in `before`'s signatures.")
r("08.1", "META", "Generator census (`l8_census.rs`)",
  "The only measurement of generated pair relations, projection outcomes, trace degeneracy, populations, and family success; floors would hold those classes.",
  "Deterministic; reproduced at `main`; source of the baseline.",
  "Floors beside `generator_classes_stay_under_mass`, read degenerate operations from the driver. A few seconds at 3,000 draws.",
  "None.",
  "Floors from it would fail a generator change that silently narrows the relational classes the differentials depend on.")
r("04.6", "PROP", "`Rank` value property",
  "`Rank`'s formatter flags as a family against `str` formatting, `AddAssign<&Rank>` (survivor rank-42, missed by the whole suite), and normal form, all against the rational oracle.",
  "Kills R1 to R7; would kill rank-42 (inferred).",
  "A property in `version/tests.rs` over 04.5's generator. 0.17 s.",
  "None.",
  "A sub-second property is the only test of `Rank`'s formatter flags as a family and of `AddAssign<&Rank>`.")
r("02.1", "ORA", "Leaf-list model of versions and parties",
  "An oracle independent of both the tree representation and production's `VersionWriter`, practical at depth 300; the committed lattice differential builds its expected values through that writer.",
  "Its own assertions killed 9 of 20 round-1 mutants (the committed suite killed all 20).",
  "A new `testing::oracles` module (newtypes, docs) and a public-API differential beside the table. Merge with 04.3.",
  "None.",
  "It is the only built oracle that judges the algebra on deep inputs without sharing the tree representation or the writer.")
r("04.3", "ORA", "Flat sweep oracle for rank, distance, lag, and `rank_cmp`",
  "Measures computed without the recursion or a sampling grid, linear in leaves; it judged inputs 3,000 levels deep. It uses 02.1's representation.",
  "Kills the 12 integrator mutants (as does the committed suite).",
  "Merge into 02.1's module as its measures. 3.2 s per 256 cases.",
  "None.",
  "It adds the measures to the leaf-list model, at depths the committed oracles cannot reach.")
r("06.1", "ORA", "Specification-codec decode differential",
  "Every decoder's accept set and error classes, through every entry point (slice, chunked reader, text, postcard, CBOR, borsh, JSON spans), against a specification written from the docs, on deep and non-canonical inputs (version depth above 32 in 11.8%, collapsible pairs 15.5%, truncation 32.2%).",
  "17 of 17 mutants (committed also; M6 and M13 to M15 rest on one or two committed tests); 300,000 cases per type twice.",
  "Port about 1,800 lines as a test-only `oracles::codec`; make eight helpers iterative; replace its runner with `proptest!`; decide its narrowed verdicts first. 10.3 s for all eleven tests. A cheaper partial fold-in, entry-point agreement without the model, is about 150 lines.",
  "Your precedence decision (section 1 of the main file).",
  "It is the broadest independent check in the audit, at the highest port cost and one decision about error precedence.")
r("07.1", "ORA", "Pool model's exact-oracle checks",
  "A `num-bigint` oracle over operands with their own histories (33% of operand uses hold two or more zero ranges, against 0), values to 5,112 digits (against 116), extreme digits at `normalize` and shifts six to eight times as often; on #37's generator, 5.6% of steps leave uncompacted cancellation. Four predicates no committed test states.",
  "M1 to M5, M7, M9, N1 (N1 now caught by #66); no unique catch at `main` plus ready.",
  "About 360 lines of checks as a second property over #37's `arb_program`, adjusted for #28 and #86. 3.7 s; about 5 to 15 s on #37's generator (inferred).",
  "#37, #28, #86.",
  "About 360 lines put a value oracle over heavily cancelled, wide, history-laden states no committed value test reaches.")
r("07.2", "COST", "Fuel matrix's operations outside the ladder",
  "Join, meet, `partial_cmp`, tick by two parties, and the projection comparison on #52's families; #52 measures only decode, `<=`, and `min_ticks`, so a per-range suanpan regression on those paths could pass all 72 cells (inferred).",
  "Measured F1 and design W's repair; no mutant run.",
  "New `Operation` variants in #52's ladder over existing `ff_*` exports; compare against the lowered operand. About 144 cells, 25 to 40 s (inferred).",
  "#52, #86.",
  "It extends a committed linearity check from three operations to six more, through guest exports that already exist.")
r("05.1", "GEN", "Pairwise-concurrent holes (`with_bumps` and the antichain properties)",
  "Three or more concurrent holes under an exact oracle, at depth and width: 20,053 refinement-decided checks per 256 shaped cases (15,255 with two or more holes); the committed exact checks reach three holes 8 times per 256 cases, on two cells only.",
  "Uniform form killed 10 mutants (committed also); returns without checking in 2% to 8% of cases.",
  "Moderate: needs 05.2, 05.7, and the world generators; construct one hole directly. Shaped form 26 s.",
  "None.",
  "It gives the largest exact-coverage gain for queries, once the census oracle exists.")
r("05.2", "ORA", "Exact coverage oracle (`census`, `sublattice`)",
  "Exact coverage on any finite vector world, where committed exact checks need a complete grid; its exactness argument re-derived by the cataloguer.",
  "Behind every coverage kill (committed also); never hit its cap in 857,820 checks.",
  "Moderate, ideally on top of the function-space oracle; count capped skips and fail on any.",
  "None.",
  "No exact-coverage check can leave a complete grid without it.")
r("05.7", "ORA", "Vector model and verdict oracles (spans and queries)",
  "The only span and query oracle sharing no machinery with production `partial_cmp`, which shares the overlay advance, `OrderState`, readers, and accumulator with the walks it judges.",
  "Behind every verdict kill (committed also).",
  "Small to moderate: pointwise helpers beside `oracles::function` and the existing `ev_vector`.",
  "None.",
  "It removes the common mode between production comparison and the span walks every committed span oracle shares.")
r("01.2", "ORA", "Interval-set model of `Party`",
  "Exact canonical results for `fork`, `forks`, array splits, `sync`, and `sync_all` independent of tree recursion, at depths 20 to 126.",
  "Properties on it caught 21 of 24 mutants; M19, M20 only through its multiplicity check (now #40, #81).",
  "A third party oracle in `testing::oracles`; about 15 doc comments; keep one multiplicity helper. 0.15 to 0.96 s per property.",
  "None (#40 for the helper).",
  "It is the only exact party oracle independent of the tree recursion at depths the function space cannot reach.")
r("04.4", "COST", "`Ranked::cmp` tie-cost probe",
  "Meters the settle-to-zero comparison that closed fix `rank-33` bounds; the board's `ranked_cmp` row never ties in rank.",
  "Caught nothing; readings 1.02 to 1.07 per doubling.",
  "A focused check in `tests/meter/settle_flatness.rs` with a floor. 5.9 s.",
  "None.",
  "It is the only built meter of a closed fix's attainable worst case.")
r("06.4", "PROP", "Misbehaving-reader probe (`Chaos`)",
  "Short reads, `Interrupted`, and failure for five more decoders and all six borsh impls; committed coverage is `Rank` only (#78 makes it exact).",
  "One calibration mutant; its failing-reader oracle is permissive.",
  "Generalize #78's scripted reader and exact oracle, about 100 lines. 0.83 s.",
  "#78 (on #53).",
  "About 100 lines extend #78's reader discipline to every decoder.")
r("06.6", "PROP", "Failing-writer probe (`Sink`)",
  "Encoders must report a writer's failure and survive `Interrupted`; the only committed test writer never fails or interrupts.",
  "Never calibrated; caught nothing.",
  "About 80 lines beside the streaming-encoder tests; its prefix-written assertion needs a documentation decision. 0.25 s.",
  "Your decision on the partial-write contract.",
  "It is the only check that encoders propagate writer failures, for a small test and one decision.")
r("06.3", "COST", "Hostile `Rank` and `Ranked` rejection families",
  "The one decoder rejection path the board does not meter (no `rank_*` or `ranked_*` rejection rows).",
  "Flat readings; no finding.",
  "Board rows with floors, ceilings, and `WORST_RANKINGS` entries. 0.75 s as a probe.",
  "#53.",
  "It closes the board's one unmetered decoder rejection path with families already built.")
r("09.A10", "PROP", "Memo reuse property for consecutive pre-scans",
  "The memo's own reuse contract, one unit below #74's end-to-end checks.",
  "Kills M15 and M21; 1.3 to 2.0 s.",
  "One property in the file #74 creates.",
  "#74.",
  "One property states the memo's contract where its defects live.")
r("09.A13", "GEN", "Swarm close bound for the range-minima property",
  "Raises #82's detection of its target mutant from 16 of 20 seeds to 20 of 20.",
  "Measured over 20 seeds each.",
  "One strategy change in #82's property.",
  "#82.",
  "A one-line change makes #82's own mutant fail every run.")
r("09.A26", "PROP", "Fork-plan tails at every depth to 140",
  "Exact hints and last-share identity at every depth 0 to 140, where #43 checks four depths.",
  "Passed on the fix; 0.2 to 0.9 s.",
  "Port to #43's `skip_shares`.",
  "#43.",
  "It widens #43's four fixed depths to every depth, cheaply.")
r("01.6", "PROP", "Wide-count fork properties",
  "Exact share identity at counts past the machine word, to `2^100`; committed tests check conservation and hints only.",
  "M04 (committed also).",
  "Into `party/forks/tests.rs`; a production-fork reference can replace the model. 1.47 s.",
  "#43, for the exactness rule.",
  "It is the only check that fork shares stay balanced and exact past `usize`.")
r("04.7", "META", "`DEFER_HITS` tap and coverage census",
  "Shows a test reaches the deferred-height reduction rather than merely a freeze; committed floors count freezes only.",
  "Source of the lane's reach numbers.",
  "Four `cfg(test)` lines in `integral.rs` and deferral floors where tests claim deferral.",
  "None.",
  "Four lines make deferral liveness checkable, which 04.1's floor needs.")
r("02.7", "GEN", "Party region generator (`gen::party`)",
  "Deep fragmented masks (depth to 300, up to 834 regions) for the masked walks.",
  "No unique catch.",
  "Small, with 02.3; needs the late-draw starvation fixed.",
  "None.",
  "It gives the masked comparisons random deep masks; 01.1 is the alternative.")
r("02.5", "GEN", "Boundary-height palette (`gen::level`, `gen::levels`)",
  "Heights on the writer's 63-bit code boundary inside random deep topology and collapse cascades (63-bit codes in 22.9% of first operands).",
  "No unique catch.",
  "A few lines beside `arb_magnitude`; useful only with a topology generator.",
  "None.",
  "It is cheap, and no committed generator aims heights at the code-width boundary in random topology.")
r("02.4", "GEN", "Correlated pair families (`gen::pair`)",
  "Equal pairs 27.2%, comparable 38.7%, and forced writer collapse cascades at random depth; committed independent pairs are never equal.",
  "No unique catch.",
  "Folds in with 02.1, whose operations it calls.",
  "02.1.",
  "It generates the tie-heavy, collapse-heavy pairs the writer's cascades need at depth.")
r("02.2", "PROP", "`algebra_matches_model` and its checks",
  "Every algebra entry point against 02.1 at depth to 300, including operator cells checked by value, fold arity to 40, and span-buffer slices.",
  "54,400 long-run cases; killed 20 of 20 mutants (9 through the model).",
  "Split per family over 02.3's generator with per-component budgets; a public-API differential binary. 48 s today.",
  "02.1.",
  "It applies the independent model to every algebra entry point at once.")
r("05.5", "PROP", "Query membership and exact coverage across worlds",
  "Exact coverage of polar queries at depth (spines to 40) and width (`2^31` to `2^1000`), with random association in conjunctions of up to five clauses.",
  "Uniform form killed 10 mutants (committed also).",
  "Moderate; extend the committed clause enums; needs 05.2 and 05.8. 0.2 to 5.1 s.",
  "None.",
  "It checks exact coverage of polar queries where the committed suite checks soundness only.")
r("05.3", "META", "Census validator (`l5_census_is_exact_against_a_finer_universe`)",
  "An executable check of the exactness argument every exact-coverage test relies on, committed grid included.",
  "Never calibrated; its doc claims a witness check it does not make.",
  "About 60 lines with 05.2 or 05.4; correct the doc. 0.15 s.",
  "None.",
  "It turns a prose exactness argument into a sub-second check.")
r("05.9", "GEN", "Grid worlds and near-equal wide heights",
  "`Before` placements 9.6% and one-sided concurrency about 12.5%, against 1.5% to 2.4% committed; near-equal wide heights in adjacent cells.",
  "No unique catch.",
  "Small: value alphabets beside `arb_magnitude`, pool closure as in `verdict_matrix`.",
  "None.",
  "It makes the rare placements four to six times more common.")
r("05.8", "GEN", "Deep and random span shapes (`Layout`, `arb_layout`)",
  "Shapes to depth 40 (338 of 771 versions at 17 to 40) for span and query checks under an exact oracle.",
  "No mutant evidence.",
  "Moderate: a strategy in `testing::generators` with an explicit layout argument, and a census floor.",
  "None.",
  "It reaches ten times the committed generator's depth, for the walks whose cursors depth stresses.")
r("05.10", "PROP", "Span verdicts against vectors",
  "An independent oracle for `place`, `dominance`, `precedence`, and `contains` at depth, with the rare placements common.",
  "M6, M7, M13, M16, M20 (committed also).",
  "Moderate; a law group over vector worlds. 0.2 to 14 s.",
  "05.7.",
  "It is a second, independent oracle for every span verdict at depth.")
r("05.11", "GEN", "Clause model and conjunction folds",
  "Operand order on both sides of every typed `&` cell, and the `!before` spelling, inside long conjunctions.",
  "Part of 05.1 and 05.5's kills.",
  "Small, as a delta to the committed clause enums.",
  "None.",
  "It is worth folding only as a delta to the committed clause enums.")
r("01.5", "PROP", "Set algebra, fork, splits, and sync against the interval model",
  "The binary identity operations' exact results at depth 80; array splits for `N` from 1 to 17 (committed: `N = 4`, production against itself); `Clock::sync` atomicity on partial overlap.",
  "12 mutants (committed also).",
  "Needs 01.1 and 01.2. 0.15 to 0.37 s per test.",
  "None.",
  "It checks every binary identity operation deeply and independently, though most of its predicates exist at depth 4.")
r("01.4", "PROP", "`join_all` and `sync_all` against the interval model",
  "After the ready branches: `sync_all`'s exact dealing against an independent model, over families the committed generators cannot build.",
  "M08, M10, M11, M18 to M21; M19, M20 now carried by #40, #81.",
  "Registry family laws fed by 01.1 plus an exact-dealing check needing 01.2. Under 1 s.",
  "#40, #81, #76 decide what stays unique.",
  "It remains an independent check of `sync_all`'s dealing once the ready branches take its other parts.")
r("01.3", "GEN", "Rule-respecting history driver",
  "`forks`, array splits, `join_all`, `sync_all`, and byte round trips inside histories, with 51% of steps holding 8 or more clocks (committed traces exceed 8 in about 0.3% to 0.6%).",
  "No unique catch.",
  "Extend `optrace::Op` with five kinds for all three models and re-derive the function-space grid (moderate to high), or keep it production-only. 0.55 s.",
  "None.",
  "It is the only driver of the multi-party operations through long rule-respecting histories, at a real porting cost.")
r("06.5", "GEN", "Near-valid encoding generators",
  "The only random source of structurally whole but non-canonical encodings and deep spines of either kind.",
  "Through 06.1, all 17 mutants.",
  "Into `testing::generators` with model trees; usable alone for a totality property and entry-point agreement.",
  "None.",
  "It serves without the model as input to a decoder totality and agreement property, or as 06.1's input.")
r("08.2", "TGT", "wasm32 pin injection table and driver",
  "The only demonstration that the 32-bit pins fail on the narrowings their docs name (7 of 11), and where they cannot.",
  "Seven caught; four misses traced to the pins' inputs.",
  "A `just` recipe outside the landing check (about 11 minutes); all fifteen swaps apply at `main`; new pins lack injections.",
  "None.",
  "It is the known-bad set for the audit's instrument of record on 32-bit targets.")
r("09.A6", "META", "Ceiling-rule auditor",
  "Recomputes each board ceiling from its stated rule; nothing committed compares the two (survey 1.4).",
  "Found COMB's and QUERY's drift; one grouping bug.",
  "Survey 1.4's Rust form in `run_acceptance`, using the Python as reference. No extra sweep.",
  "Slot 26's one-rule branch (on #53).",
  "It makes the one-rule ruling arithmetic that fails on drift in either direction.")
r("09.A7", "META", "libtest main-thread race probe",
  "The only reproduction of the heap meters' race; refuted the single-thread fix.",
  "Decided question 91's premise.",
  "Planned as slot 47's calibration. 0.54 s.",
  "Slot 47 (shared counting allocator).",
  "It is the known-bad demonstration and acceptance test for the planned allocator.")
r("09.A5", "META", "Fuel comparison between two commits",
  "Makes every constant-factor fuel change visible and attributable per commit; the bands admit about 1.58-fold slack and nothing compares commits.",
  "Found three unrecorded movements that no band failed.",
  "A fuzz-fit capture and compare mode modeled on #95. Developer tool; no gate time.",
  "None (#95 for a shared format).",
  "Four branches rebuilt this comparison by hand; one mode would serve every later change.")
r("07.5", "TGT", "wasm32 landing case 5",
  "A limb index between `2^31` and `2^32` at shift 0, which no committed or #54 case reaches.",
  "Caught nothing; the defect it would catch is contrived.",
  "One more case in the existing loop. About 14 s.",
  "#31, #54.",
  "It adds a cheap extra boundary to the 32-bit landing pin.")
r("09.A11", "TGT", "Linear-memory reading after each wasm32 pin case",
  "Pages held per case (3.06 to 3.56 GiB of 4); the headroom #29 claims is unchecked.",
  "#63 cites it.",
  "Read `memory.size` after each call; assert a ceiling per check.",
  "None.",
  "It would warn before a pin runs out of address space and tests the allocator instead.")
r("04.16", "TGT", "wasm32 memory-size readout",
  "The same reading as 09.A11, built separately in the lane's prototype harness.",
  "Produced the page counts behind observation O6.",
  "Add the page count to the harness result.",
  "#31.",
  "It is a second build of 09.A11's reading, so only one of the two should be folded in.")
r("09.A27", "PROP", "Shifted-zero checks through every operator spelling",
  "Seven operator spellings keep #48's constant-space guarantee; #48 drives four entry points.",
  "Passes natively and in six wasm32 combinations.",
  "One test in `representation.rs`.",
  "#48.",
  "It checks every spelling keeps a guarantee that today holds only because they share one route.")
r("09.A22", "PROP", "Counterexamples behind `Clock::from_parts`'s warning",
  "Where 'reproduces issued stamps' fails, which #85's warning states.",
  "Pass.",
  "Three tests into `tests/stale_state.rs`.",
  "#85, slot 45.",
  "Existing cases keep a public warning true.")
r("09.A24", "TGT", "Guest panic records under memory exhaustion",
  "What #31's panic record captures in each case (literal panics, `expect`, `Option::unwrap`, overflow, abort).",
  "The evidence #31's documentation cites.",
  "One guest case each, after #58.",
  "#31, #58.",
  "It pins what the wasm32 failure channel can tell you.")
r("06.2", "TGT", "wasm32 borsh probe over a wide one-leaf version",
  "The only borsh decode on a 32-bit target, past bit position `2^32` in borsh's stream cursor; the guest builds without `borsh` today.",
  "Demonstrated a growth panic your ruling dissolved; no narrowing injected.",
  "Reviewed rewrite `8208efaeb` exists; restate it as a bit-position check, new variant after #58. About 150 s per case.",
  "#58; keeping `fix/before-wasm32-buffer-growth`.",
  "It is the only 32-bit borsh check, and its reviewed form sits on a branch slated for deletion.")
r("09.A14", "TGT", "wasm32 differential over accumulator histories at the stability-width boundary",
  "The only wasm32 test over varied histories: 600 histories at eight widths around the boundary; failed 498 of 4,800 at the defect's base.",
  "The reviewer judged (without a run) that its oracle blesses the truncating mutant #50's pin catches.",
  "Two `Check` variants after #58, two guest functions, a harness test. About 70 s.",
  "#58, #50.",
  "It varies histories on 32-bit, but its oracle is weaker than #50's pin against its own defect.")
r("03.9", "COST", "Random counter search for tick and `min_ticks`",
  "The board's touch and scan ceilings over random co-generated pairs; `min_ticks` reads exactly 8.00 scan bits per byte.",
  "Found nothing; maxima at 61% and 49% of the ceilings.",
  "A property over #74's strategies; deterministic counters, no allocator. 8.8 s release.",
  "#74.",
  "It extends two ceilings from fixed families to random inputs where tick's memo is busiest.")
r("04.12", "COST", "Integrator cost search",
  "The only random search over wide multi-scale shapes, at magnitudes neither the board's families nor fuzz-fit programs build.",
  "Caught nothing.",
  "A metered property over 04.1 with a touch ceiling, in its own process. 5.7 s.",
  "None.",
  "It searches random wide shapes against the measures' touch ceiling but only prints today.")
r("01.8", "COST", "Identity scan-cost probe",
  "`without` with a deep arbitrary receiver (the board meters only seed and self receivers), and a fork step dominated by count width.",
  "Flat; its `is_disjoint` row measures nothing.",
  "Two board families with ceilings and positive floors. 0.98 s.",
  "Slot 26's ceiling rule.",
  "It supplies two pessimal identity families the board lacks, ready to become rows.")
r("03.4", "PROP", "Deep co-generated spines checked without an oracle",
  "Random tick inputs 1 to 2,999 levels deep, between the oracle's depth and the stack tests' fixed spines.",
  "Caught several mutants, none uniquely; timed out under some.",
  "Highest in its lane: ignored or high-count, an explicit-stack thread, #74's strategy with a depth parameter. 3.9 s at 8 cases.",
  "#74; ruling 68.",
  "The only random tick inputs thousands of levels deep, at a cost that suits an ignored test.")
r("06.7", "PROP", "Encoder agreement with an independent pointwise model",
  "Production's bytes against a format-description encoder at random, including joins of deep spines.",
  "M12 fails only this leg of the harness (committed: 58 tests).",
  "Small once 06.1's encoders exist.",
  "06.1.",
  "Its byte-level encoder checks are largely duplicated by 02.6 with 02.1.")
r("02.6", "ORA", "Independent canonical encoders (`encode_version`, `encode_party`)",
  "Ties every algebra result to the written wire specification, byte for byte.",
  "Writer mutants were killed first by production assertions.",
  "Tiny with 02.1.",
  "02.1.",
  "It is the wire-specification half of 02.1, and 06.1's codec would serve as well.")
r("05.12", "PROP", "Projected-span views against masked vectors",
  "Independence from production projection for `OwnSpan`; masks at arbitrary leaf subsets of deep shapes.",
  "M14 (committed also); six placements nearly absent.",
  "Small once 05.7 exists; add masked probe versions.",
  "05.7.",
  "It is a modest second oracle for projected views, weakened by its verdict mix.")
r("01.10", "PROP", "Overlay refinement property",
  "`Clock::shape` against the coarsest common refinement at party depth above 32 in 47%, comparing the rise field itself.",
  "M23 (committed also).",
  "Swap production `Version::shape` for an oracle; becomes a deeper driver of the committed row. 0.29 s.",
  "None.",
  "Its added depth comes mostly from the party side, which 01.1 already feeds.")
r("04.13", "PROP", "Seven-route rank property",
  "A version's rank equals the `Sum` of its per-leaf areas, which ties the integrator to `Rank` arithmetic for the first time.",
  "Caught nothing.",
  "A property over 04.1. 0.96 s.",
  "None.",
  "It gives every version's rank a cheap second route.")
r("04.10", "PROP", "Pair predicate set over the flat oracle",
  "Exactness against 04.3 in 04.1's regime; most predicates are committed laws.",
  "70,000 plus 600 deep cases; 12 mutants.",
  "Keep only the exactness checks once 04.1 feeds the law drivers. 3.2 s.",
  "None.",
  "It holds little of its own beyond the combination of 04.1 and 04.3.")
r("04.14", "PROP", "`Sum` over any order against the rational oracle",
  "A second, independent reference for the committed `Sum` property, on 04.5's inputs.",
  "Caught nothing.",
  "A second strategy for the committed property. 0.23 s.",
  "None.",
  "It adds a second reference for a property the suite already states.")
r("06.8", "PROP", "Writer copy-path harness",
  "An independent oracle on the splice path; the committed copy tests compare two feeding paths through one writer.",
  "W1, W3 (60 and 65 committed tests also).",
  "About 150 lines; remove its `prop_assume!`; make helpers iterative.",
  "06.1's encoder.",
  "It is an independent oracle on a path the suite already exercises heavily.")
r("05.14", "PROP", "Span algebra against vectors",
  "Only the independent oracle and the regimes; the committed laws cover every spelling.",
  "M12 (committed also); its doc overclaims.",
  "Small; correct the doc.",
  "05.7.",
  "The committed laws already cover every spelling it checks.")
r("05.15", "GEN", "Organic overlays",
  "Exact coverage on organic shapes (depth at most 5).",
  "No mutant evidence.",
  "Small if 05.2 and 05.7 are folded.",
  "05.2, 05.7.",
  "Organic shapes are shallow, and the other generators already cover them.")
r("06.15", "PROP", "CBOR framing probe",
  "The serde bridging you ruled intended and #85 documents, which no test states.",
  "Produced question Q2.",
  "One test of a few lines.",
  "#85.",
  "It is a one-line guard for a documented bridging that comes from the format crate.")
r("09.A20", "PROP", "Serde round trips through third-party formats",
  "The serde contract against eight real formats (csv, MessagePack, bencode, XML, RON, YAML, TOML, JSON).",
  "Found the strict design's csv breakage.",
  "Eight new dev-dependencies, best in a detached workspace.",
  "#51.",
  "It is the only check against deserializers applications use, at the price of eight dependencies.")
# ---------------------------------------------------------------- blocked on a ruling or unbuilt branch
r("09.A4", "COST", "`min_ticks` heap probe over right spines, with a closed-form value oracle",
  "756 readings at sizes just past powers of two; on `main`'s code, 177 exceed the board's heap ceiling (up to 307.67 bytes per input byte), with every value matching an independent closed form.",
  "Measures `main`'s open `min_ticks` heap defect beyond its record.",
  "High: fails at `main` by design; installs a global allocator; builds through the recursive oracle on a 2 GiB stack. 0.3 to 14.2 s per test.",
  "Question 87.",
  "It is the widest measurement of an open defect on `main`, and it can land only beside a fix.")
r("03.5", "COST", "Jump-entered rising spine (also 09.A8, its board-family port in `08573e159`)",
  "The only family that breaks a committed ceiling (`min_ticks` heap 120 to 197 bytes per input byte against 20) and the strongest rank-fold touch case (1.28 to 1.32 times `harmonic`).",
  "D1's witness and demonstration.",
  "Ported in `08573e159` with 50 re-pins; fails acceptance at `main` by design.",
  "Question 87.",
  "It is the board family for `main`'s open `min_ticks` heap defect, and it lands with a fix or a declared model.")
r("09.A9", "COST", "Limb work of `min_ticks` on wide-offset combs",
  "`num-bigint` work no board currency counts; quadratic on the stopped design, linear at `main`.",
  "Caught the stopped design's quadratic work.",
  "Survey 1.1's form: comb families in #52's fuel ladder, with no hooks.",
  "#52; question 87.",
  "It guards any future `min_ticks` redesign against work no current meter sees.")
r("09.A15", "COST", "Real-scale stepped spine for `min_ticks`",
  "The only real-scale probe (a 91 MB input); showed the first fix design failing at a scale the board cannot reach.",
  "Above.",
  "Not a gate test; an on-demand developer check; `stepped_spine` ports small.",
  "Question 87.",
  "It is the probe the write-up asks for before any redesign is trusted.")
r("03.7", "COST", "Plain rising spine (heap)",
  "The doubling sawtooth alone (19.7 to 39.4 bytes per input byte), without the spill.",
  "D1's first witness.",
  "As a board family its verdict depends on where the ladder lands.",
  "Question 87.",
  "It shows the doubling mechanism alone and is secondary to 03.5.")
r("03.8", "COST", "Random tick heap search",
  "The per-sample heap ceiling over random co-generated pairs.",
  "Peak 0.43 of the ceiling; found nothing.",
  "A property over #74's strategies with a counting allocator. 13.1 s release.",
  "Question 91's shared allocator; #74.",
  "It extends the heap ceiling to random inputs once the allocator exists.")
r("09.A25", "COST", "Heap size sweeps for single operations",
  "Lazily decoded fold inputs and sizes between board samples; found a lookahead heap rise and a forks heap cliff no board row showed.",
  "Above.",
  "A board row with lazily decoded inputs; more sizes.",
  "Slots 23 and 33.",
  "It samples two regimes the board does not, behind two unbuilt branches.")
r("01.9", "COST", "Identity heap-retention probe",
  "Retained heap of six identity results after their inputs drop; only `fork` is checked today.",
  "Found S1 and O1.",
  "Capacity assertions (small) or the allocator.",
  "Slot 33; question 91.",
  "It is the measurement behind S1 and the only record of O1.")
# ---------------------------------------------------------------- meta: survivor records and mutant sets
r("08.4", "META", "Classified mutation survivors (282)",
  "A baseline for later campaigns; 96 cost-only mutants as the cost instruments' first calibration set; 110 written equivalence and unreachability arguments.",
  "Every lane-8 brief came from it.",
  "A record; committing it as a baseline needs your ruling (section 1 of the main file).",
  "#43, #63, #66, #67, #78, #82, #83 kill or delete survivors.",
  "It answers whether the suite is still as strong as it was, if you rule that it may be committed.")
r("08.5", "META", "Mutation campaign tooling",
  "The only total measure of what the suite misses (91.5% of viable mutants killed).",
  "Produced 08.4.",
  "A recipe for a phase boundary or release; about 11 hours a run.",
  "None.",
  "It is too slow for every commit and suited to a release.")
r("09.A19", "META", "Compile-time mutant switches (`option_env!`) for cost-only survivors",
  "Showed three cost-only survivors escape every committed meter, fuel bands included; a template for switches that compile to nothing.",
  "Above.",
  "A template for committing mutants, or survey 1.9's ladder cell instead.",
  "#52.",
  "It is the template for a committed home for production mutants, if you want one.")
r("09.A12", "META", "Mutant schema for `Rank::decode`'s reader paths",
  "Shows #78's oracle rejects a permissive window and accepts a correct early-stopping decoder.",
  "Above.",
  "A test-only copy of the decoder, about 100 lines.",
  "#78.",
  "It is the only evidence that #78's oracle is neither too loose nor too strict.")
r("09.A16", "META", "Mutant schema for suanpan's readout carry classes",
  "12 of 22 class mutants fail only #64's table.",
  "All 22 fail #64.",
  "A production switch with no committed home.",
  "#64.",
  "It shows #64's table is the only test of the readout's touch cost.")
r("09.A17", "META", "Stack-recursion mutants and frame-size probes",
  "Which deep test alone detects each recursion; why the depth is `2^18`.",
  "PAMT passes at 100,000 levels and fails at `2^18`.",
  "High: one aborting run per mutant.",
  "#34, #75.",
  "It is the only calibration of the deep-input tests, which stay after the guard's removal.")
r("09.A18", "META", "Mutants of suanpan's written-position set",
  "#86's calibration; the truncation mutants need about 130 of 256 cases.",
  "All eight fail #86's model test.",
  "A production switch with no committed home.",
  "#86.",
  "It calibrates #86 and shows where #86's generator thins out.")
r("07.3", "META", "Suanpan mutant schema and calibration scripts (27 mutants)",
  "Mutant forms cargo-mutants does not generate, and a kill matrix by instrument.",
  "Backs MB1, MB2, F1c.",
  "Re-derive against `main`; twelve target code #86 deletes.",
  "#86, #89.",
  "It is the suanpan lane's only record of which instrument kills which hand-made defect.")
r("03.6", "META", "Events calibration mutant set (M1 to M23)",
  "A ready calibration set for the pre-scan, memo, and `min_ticks`; four escaped at the base.",
  "The evidence behind #74.",
  "The switch edits library code; patches are against the base.",
  "None.",
  "It is a calibration set for any future change to the tick walk.")
r("01.12", "META", "Identity mutant lists and calibration driver",
  "24 semantic mutants (duplicated error groups, loops turned recursive).",
  "Backed MB1 to MB3.",
  "Needs a known-bad mechanism the project lacks.",
  "None.",
  "It holds known-bad identity implementations, valuable if you want calibration kept as a running check.")
r("02.13", "META", "Algebra round-1 mutants and `mutate.py`",
  "Nine are the only demonstration that 02.1's model, not production's assertions, detects errors.",
  "Committed suite killed all 20.",
  "No code; keep as 02.1's calibration.",
  "None.",
  "It is the calibration set 02.1 needs if folded in.")
r("04.11", "META", "Integrator mutant patch (12 mutants)",
  "A calibration set for 04.1, 04.3, and 04.10.",
  "All 12 killed by both harness and suite.",
  "Applies only at the explore tip.",
  "04.7's tap.",
  "It is the calibration set for a folded-in integrator instrument.")
r("04.17", "META", "`Rank` mutants R1 to R7 and E8 (source not kept)",
  "E8 is the known-bad 04.2's fold-in needs.",
  "Calibrated 04.6.",
  "Rebuild from the descriptions.",
  "None.",
  "Only E8 matters, as the known-bad input 04.2's fold-in needs.")
r("05.17", "META", "Spans calibration mutants (M1 to M20, table only)",
  "A known-bad list for the spans fold-ins.",
  "Committed suite caught all 17 non-equivalent ones.",
  "Rebuild from the table, about an hour.",
  "None.",
  "It is a ready calibration list for whatever is folded in.")
r("06.10", "META", "Decoder mutation schemas (M1 to M17)",
  "Data on thin committed checks: M6 rests on one test, M14 and M15 on two.",
  "Every mutant failed a committed test.",
  "Keep as data; the anchors rot.",
  "None.",
  "It records which decoder checks are thin.")
r("09.A36", "META", "Calibration sets for carried tests (24 sets)",
  "The evidence behind each ready entry's calibration claims.",
  "Reported per entry.",
  "Needs a committed home for production mutants.",
  "Their target branches.",
  "It is the only way to rerun the ready branches' calibrations after the code changes.")
r("04.9", "PROP", "Schedule-independence probes E13 to E15 (source lost)",
  "Values independent of the integrator's freeze and deferral schedule; the evidence behind five survivor classifications.",
  "Each passes.",
  "Rebuild as a metamorphic property with a test-only schedule override.",
  "None.",
  "It is the evidence behind five survivor classifications, and keeping it needs a rebuild.")
# ---------------------------------------------------------------- low or no fold-in value
r("07.4", "COST", "Hill-climbing touch adversary",
  "Searches toward #37's bound and rescales widths.",
  "Best ratio 9.6 under the old pricing; never calibrated.",
  "Port to #37's pricing; keep ignored.",
  "#37.",
  "It is a tool for re-measuring #37's margin, not a regression check.")
r("02.10", "COST", "Fragmented-mask cost families",
  "Masks with regions at every depth inside one plateau.",
  "Flat; no finding.",
  "A board family after the board branches.",
  "#93, #94, #95, ceiling rules.",
  "It is a differently shaped mask adversary for walks the board already attacks.")
r("05.13", "COST", "Carry-boundary touch family",
  "Probe-minus-bound differences crossing `2^k` in every cell.",
  "Flat; asserts nothing.",
  "A registry family with ceilings.",
  "Question 65's rule.",
  "It is a plausible board family that has never failed.")
r("02.11", "COST", "Carry-ripple cost families",
  "The cliff comb's alternation on a balanced grid.",
  "Flat.",
  "A board family.",
  "As 02.10.",
  "It restates the board's cliff comb in another topology.")
r("01.7", "PROP", "Deep identity probe on a 256 KiB stack",
  "A zigzag party at depth 100,000, which no committed or ready test builds; a stack bound `RUST_MIN_STACK` cannot loosen.",
  "No unique mutant.",
  "A few lines into #75 and a zigzag builder. 0.77 s.",
  "#34, #75.",
  "Its one unduplicated regime takes a few lines to add to #75.")
r("01.13", "GEN", "Deep-party builders",
  "The alternating spine and comb, and shallow-plus-deep parties.",
  "Inputs to 01.7 to 01.9.",
  "Small; worth taking only with those entries.",
  "#34.",
  "It is worth taking only together with 01.7, 01.8, or 01.9.")
r("09.A21", "META", "Board-cell bisection and allocator event log",
  "A commit and a mechanism for any heap movement.",
  "Traced COMB's rise to `63d01d903`.",
  "A recipe; the log needs `unsafe` or question 91's hooks.",
  "Question 91.",
  "It turns unexplained heap movement into a commit and a cause.")
r("08.8", "META", "Probe copy and hit recorder",
  "Which tests reach a marked line, without a profiler.",
  "Shaped #82's brief.",
  "Maintainer tooling; the marked sites are lost.",
  "None.",
  "It is worth keeping only if mutation work recurs.")
r("08.9", "META", "Branch-coverage map",
  "97.8% of lines and 95.4% of branch outcomes; every finding routed.",
  "Located five gaps.",
  "Mac or CI only.",
  "None.",
  "It is a periodic report that cannot run on the box.")
r("01.11", "META", "Identity generator statistics printers",
  "Two-child branch counts and unary chains, which the census lacks.",
  "Produced observation O8.",
  "Add two metrics to the census.",
  "None.",
  "It is useful only if reach floors are adopted.")
r("02.8", "META", "Writer-path census",
  "Counts writer collapse paths and code widths.",
  "Two flaws: a wrong totals line, first pair only.",
  "Reach floors for 02.3.",
  "02.1.",
  "It would hold floors on writer-path reach for a folded-in generator.")
r("03.13", "META", "Events generator histograms",
  "The only record of how rarely independent pairs reach tick's pre-scan regimes.",
  "Numbers behind #74.",
  "Its committed-generator row as a floor.",
  "None.",
  "Only its committed-generator row is worth turning into a floor.")
r("05.16", "META", "Committed-population reach probe (spans)",
  "The only measurement of the committed span population's placement mix.",
  "Not applicable.",
  "A placement-mix floor.",
  "None.",
  "It is useful only for placement-mix floors.")
r("06.12", "META", "Codec reach histograms",
  "Depth and violation rates of 06.5's inputs.",
  "Produced 06.1's reach numbers.",
  "Floors for 06.5 if folded.",
  "06.5.",
  "It is useful only as 06.5's liveness floor.")
r("06.9", "PROP", "First-detected-class model (unenforced)",
  "Would fix the detection order on every multiply-defective input.",
  "Never fails.",
  "A few lines once 06.1 exists.",
  "Your precedence decision.",
  "It is ready-made enforcement if you document a precedence.")
r("08.7", "PROP", "Lone-prefix padding tests",
  "A lone first field with a dirty padding bit; kills both padding survivors.",
  "Kill both survivors, at the base and on #67.",
  "Two unit tests, held.",
  "Your precedence decision.",
  "It is worth folding only if you document that precedence.")
r("09.A31", "META", "Scan for self-recursive test helpers",
  "Retargeted at library code, a mechanical check of the no-recursion rule (direct calls only).",
  "Led to question 68.",
  "A small script.",
  "Question 68.",
  "It is a static check of a rule only review enforces.")
r("08.11", "META", "Surface check end-to-end calibration",
  "A planted item through the extractor.",
  "Two of two caught.",
  "A fixture crate and nightly in tests.",
  "None.",
  "The committed anchors test already catches part of what it would add.")
r("08.12", "META", "Fold-pin calibration",
  "The log-factor pins check only a floor.",
  "A quadratic fold passes all five pins.",
  "A board sweep on the mutant.",
  "None.",
  "Its lesson is recorded; it is one member of 08.4's cost class.")
r("08.10", "META", "Counter bisect harness",
  "Attributes one reading's movement to a commit.",
  "Attributed a touch rise to `fdd1bf47`.",
  "A `git bisect run` predicate.",
  "None.",
  "What it adds reduces to a few lines of predicate for `git bisect run`.")
r("09.A28", "PROBE", "Probe of the integral's dense-span bound",
  "#72's derived bound is attained within one.",
  "2,063 against a bound of 2,064.",
  "A `debug_assert!` would need `S` threaded through.",
  "#72.",
  "It shows #72's derived bound is tight to within one.")
r("09.A29", "TGT", "Compare pin grown past bit `2^32`",
  "A comparison decided past bit `2^32` on 32-bit.",
  "Did not catch its target narrowing.",
  "A guest case, about 1 GiB, 36 s.",
  "#58.",
  "It reaches past bit `2^32`, but no constructed defect on that path is behind it.")
r("09.A30", "TGT", "Rank alignment cases on each side of a `2^32` gap",
  "Adds the gap `2^32 + 1` to #63's pin.",
  "The committed cases already catch the only mutant.",
  "Cases in the existing check; slow.",
  "#63, #28.",
  "It mostly duplicates #63's pin, whose cases already catch the only constructed mutant.")
r("04.15", "TGT", "wasm32 `Sum` footprint and replay cases",
  "Observation O6: a deep-first `Sum` aborts on allocation on wasm32.",
  "The reproduction of record for O6.",
  "An acceptance check for a growth-policy change, not a passing test today.",
  "#28, #31.",
  "It is the acceptance check for any suanpan growth change that targets O6.")
r("06.11", "TGT", "wasm32 rank group-stream probe",
  "One doubling past the committed `RankDecode` pin from a lazy source.",
  "Demonstrated a dissolved growth panic.",
  "Reviewed rewrite on `8208efaeb`; about 130 s per case.",
  "#58; keeping `fix/before-wasm32-buffer-growth`.",
  "It adds little beyond the committed pin, though its lazy reader is reusable.")
r("09.A33", "ORA", "Differentials against replaced implementations",
  "Old against new for the one-sweep lattice, `refine_partial`, and `sync_all`.",
  "No divergence; sweep mutants fail.",
  "Would keep deleted implementations as oracles; the sweep took 386 s.",
  "#30, #61, #76.",
  "They did their job at the change, and keeping them means keeping deleted code as oracles.")
r("02.9", "META", "Stack-overflow calibration",
  "A minimal recursion overflows at depth 100,000 on 2 MiB.",
  "Abort verified.",
  "Needs a child process.",
  "None.",
  "#75's derivation already states a stronger bound.")
r("03.10", "GEN", "Bushy strategy as a property",
  "Balanced non-spine shapes 7 to 10 levels deep.",
  "Caught nothing #74 missed.",
  "Low.",
  "#74.",
  "#74's spine strategy dominates it on every measured metric.")
r("03.11", "GEN", "Second late perturbation",
  "A second divergence site at an off-palette height.",
  "None attributable.",
  "Adds a third to each case's runtime.",
  "#74.",
  "No measurement credits the second perturbation with any catch.")
r("07.6", "COST", "Offset-comparison accounting probe",
  "Per-call cost of one range-minima comparison.",
  "Answered a lead with no defect.",
  "Needs a production hook.",
  "None.",
  "It confirmed a cost argument the board already enforces.")
r("09.A32", "PROBE", "Board recipe interrupt behavior on a toy justfile",
  "Regression evidence for #93's trap.",
  "Above.",
  "No harness for recipes exists.",
  "#93.",
  "It is regression evidence for one recipe, with no committed home.")
r("09.A34", "TGT", "32-bit decoder-growth instruments (`60d534ed`, `d0a6205f`, `8208efae`)",
  "Decoders past the 1 GiB doubling limit on 32-bit, which your ruling made unpromised.",
  "Two findings outlive the ruling (dense heights past `2^32` bits; `Chain::read_to_end`).",
  "Not applicable under the ruling; the streaming synthesizer is reusable.",
  "Your input-size ruling.",
  "Your input-size ruling leaves them little to check; the streaming synthesizer is the reusable part.")
r("06.13", "TGT", "wasm32 reader probe over a wide one-leaf version",
  "`std`'s fallible `read_to_end` refusing growth on wasm32.",
  "Evidence for #53's `Decode::Io` sentence.",
  "Not recommended as a check.",
  "None.",
  "It is evidence for #53's documentation, not an instrument to keep.")
r("08.13", "TGT", "`num-bigint` formatting probe",
  "The only executable demonstration that `count_display`'s ranking depends on the target.",
  "Became `ddfabe4cb`.",
  "Vendors about 15,000 lines; diagnostic only.",
  "None.",
  "Its finding is settled on `main` by `ddfabe4cb`.")
r("01.14", "TGT", "Unoptimized-build deep-test configuration (MB3)",
  "Stack safety without tail-call elimination.",
  "Caught M25.",
  "You ruled against it (question 68).",
  "None.",
  "You ruled against it under question 68; it is listed because it was built.")
r("06.14", "META", "Writer mutants and reach probes",
  "Showed an early return cannot run.",
  "Its finding is #67.",
  "None.",
  "None.",
  "Its finding became #67, and it checks nothing further.")
r("02.14", "META", "Survivor and tie-order patches with `run.sh`",
  "Tie order cannot change a verdict.",
  "3,200 cases each.",
  "None.",
  "None.",
  "It records a settled question.")
r("05.20", "PROBE", "Tie-order experiment",
  "The `overlay.rs:142` swap is equivalent for placement and filters.",
  "Supports the equivalence.",
  "None.",
  "None.",
  "The question it answered is settled, and #85 documents the answer.")
r("03.12", "COST", "Memo-dense tick heap families",
  "Same memo traffic as the board's memo families.",
  "Confirmed a closed fix.",
  "Not worth folding.",
  "None.",
  "The board's memo families already build the same memo traffic.")
r("03.14", "META", "Memo reach diagnostic",
  "Superseded by #74's census.",
  "Set the wide strategy's regime claims.",
  "None.",
  "None.",
  "#74's census floors supersede it.")
r("05.18", "COST", "Refinement scan-cost probe",
  "Superseded by #61 and #94.",
  "Motivated #61.",
  "Not worth folding.",
  "#61, #94.",
  "#61 used its finding, and #94 meters the same case on the board.")
r("05.19", "META", "Spans behavioral histograms",
  "Every reach number in the lane's records.",
  "Not applicable.",
  "Not as written: a global mutex.",
  "None.",
  "The numbers are worth keeping, but the code's global map is not.")
r("07.7", "META", "Readout carry census (`l7-high-log`)",
  "Which readout classes a suite reaches.",
  "Found the gap #64 closed.",
  "A file-writing feature does not fit.",
  "#27, #64.",
  "#64 tests every readout class directly, so it has nothing left to find.")
r("07.9", "META", "Native size probe",
  "Encoded sizes of the fuel families.",
  "Supplied F1's denominators.",
  "None; #52 records the same.",
  "None.",
  "#52's ladder records the same sizes as its denominators.")
r("08.14", "PROBE", "Split-writer hunt",
  "Random ticks never reach an early return #67 deletes.",
  "Two runs of 300,000 found nothing.",
  "None.",
  "#67.",
  "Its target disappears when #67 lands.")
r("08.15", "PROBE", "libtest invocation probe",
  "How nextest invokes a test binary on the box.",
  "Grounded the heap-race diagnosis.",
  "None.",
  "None.",
  "Its one finding, about the runner, is recorded.")
r("06.16", "TGT", "wasm32 `Vec` growth diagnostic",
  "A fact about `std`.",
  "Established a dissolved defect's mechanism.",
  "Not recommended.",
  "None.",
  "It explains a dissolved defect's mechanism and checks no contract.")
r("07.8", "PROBE", "Prototype T (position tags)",
  "A partial design for F1.",
  "Design evidence only.",
  "Not applicable.",
  "None.",
  "It is a partial design that #86's full repair superseded.")
r("07.10", "PROBE", "Touch replay probe",
  "Diagnosed a shared-meter false failure.",
  "Above.",
  "Not applicable.",
  "None.",
  "It diagnosed one false failure of a shared meter and checks nothing.")
r("03.15", "PROBE", "Bridge round trip on one pair",
  "None beyond every bridged differential.",
  "A sanity check.",
  "Not worth folding.",
  "None.",
  "Every bridged differential already depends on the round trip it checks.")
r("02.12", "PROBE", "Parallel long-run copies `par00` to `par15`",
  "A way to run 02.2 long.",
  "54,400 cases.",
  "None; `PROPTEST_CASES` serves.",
  "None.",
  "Nextest's high-count profile with `PROPTEST_CASES` runs 02.2 long without it.")
r("02.15", "PROBE", "`run_on_tree.sh`",
  "Runs the probe on an uncommitted merge.",
  "Used once.",
  "None.",
  "None.",
  "It is a runner for uncommitted merges, not an instrument.")
r("02.16", "PROBE", "`extract.py`",
  "Converts survivor records to patches.",
  "Mangled one patch.",
  "None.",
  "None.",
  "It is a converter used once, with nothing to fold in.")
r("07.11", "PROBE", "Suanpan scratch drafts and table scripts",
  "Drafts whose final forms are other entries.",
  "Produced the F1 tables.",
  "None.",
  "None.",
  "Its drafts' final forms are catalogued as other entries.")
r("09.A35", "PROBE", "Instruments superseded by a ruling or design",
  "Strict serde tests (`9123a4d8`), protocol sweeps (`789936bc`), a panic classifier, an adapted probe.",
  "Superseded.",
  "None.",
  "None.",
  "None adds coverage at `main` plus the ready branches.")
r("05.21", "ORA", "Witness heuristic (`witness_coverage`)",
  "Nothing; dead code.",
  "None.",
  "Delete it.",
  "None.",
  "Nothing calls it, so deleting it is the only action.")

ALL_IDS = (
    [f"01.{i}" for i in range(1, 15)] + [f"02.{i}" for i in range(1, 17)] +
    [f"03.{i}" for i in range(1, 16)] + [f"04.{i}" for i in range(1, 18)] +
    [f"05.{i}" for i in range(1, 22)] + [f"06.{i}" for i in range(1, 17)] +
    [f"07.{i}" for i in range(1, 12)] + [f"08.{i}" for i in range(1, 16)] +
    [f"09.A{i}" for i in range(1, 37)]
)
MERGED = {"09.A8": "03.5", "09.A23": "08.3"}


def check():
    ids = [x[0] for x in R]
    dup = [i for i, c in collections.Counter(ids).items() if c > 1]
    assert not dup, dup
    missing = [i for i in ALL_IDS if i not in ids and i not in MERGED]
    extra = [i for i in ids if i not in ALL_IDS]
    assert not missing, missing
    assert not extra, extra
    for x in R:
        assert x[1] in CATS, x
    return len(R)


def link(i):
    return f"[{i}]({SEC[i[:2]]})"


def cat_tables():
    by = collections.defaultdict(list)
    for pos, x in enumerate(R, 1):
        by[x[1]].append((pos, x))
    out = {}
    for c, (fname, title) in CATS.items():
        rows = by[c]
        lines = ["| Rank | Overall | Instrument | Adds beyond `main` and the ready branches | Evidence | Fold-in work and runtime | Depends on |",
                 "|---:|---:|---|---|---|---|---|"]
        for k, (pos, x) in enumerate(rows, 1):
            i, _, name, adds, ev, cost, deps, _reason = x
            lines.append(f"| {k} | {pos} | {link(i)} {name} | {adds} | {ev} | {cost} | {deps} |")
        out[c] = (fname, title, len(rows), "\n".join(lines))
    return out


def overall():
    lines = ["| Position | Instrument | Category | Why it sits here |", "|---:|---|---|---|"]
    for pos, x in enumerate(R, 1):
        i, c, name, *_rest, reason = x
        lines.append(f"| {pos} | {link(i)} {name} | {CATS[c][1]} | {reason} |")
    return "\n".join(lines)


if __name__ == "__main__":
    n = check()
    t = cat_tables()
    if sys.argv[1:] == ["counts"]:
        print(n)
        for c, (f, title, k, _) in t.items():
            print(c, f, title, k)
    elif sys.argv[1:2] == ["cat"]:
        f, title, k, tbl = t[sys.argv[2]]
        print(tbl)
    elif sys.argv[1:] == ["overall"]:
        print(overall())


CAVEAT = "<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the instrument rescue's integrator, generated from one ranked list of every instrument in sections 01 to 09; for Finch's review. -->"

COMMON = """Each row condenses the linked entry in sections 01 to 09, which gives the
full account and marks every figure as verified, reported, or inferred; a
row's figures carry the linked entry's marks. "Rank" orders this category;
"Overall" is the instrument's position in the single ranking in
[`../instrument-rescue.md`](../instrument-rescue.md), section 3, and both
rank by the coverage an instrument adds against the work of folding it in.
`#n` is entry `n` of `QUESTIONS.md`; "slot 23", "slot 26", "slot 33",
"slot 45", "slot 46", and "slot 47" are ruled branches not yet ready. Where two lanes
built overlapping instruments, [`17-overlaps.md`](17-overlaps.md) says which
belong together or which subsumes which."""

INTRO = {
 "GEN": """A generator adds coverage through the inputs it reaches that the committed
generators do not. [`00-baseline.md`](00-baseline.md) section 2 gives the committed
reach, and its section 2.7 lists the regimes the committed generators never
or almost never produce; most rows here are claims about that list. Some
generators fold in only with a model (02.4 calls 02.1's operations, 05.1
needs 05.2 and 05.7); the "Depends on" column says so.""",
 "ORA": """An oracle adds coverage through independence: what counts is the code and
structure it does not share with production.
[`00-baseline.md`](00-baseline.md) section 3 lists the committed oracles and
the production code each one shares: the recursive tree oracle shares
production's recursion, the bridge shares its writer and readers, and the
function-space oracle judges only shallow inputs. Several lanes built
independent models of the same objects; section 1 of
[`17-overlaps.md`](17-overlaps.md) reconciles them.""",
 "PROP": """A property adds coverage by stating a predicate no committed test states, or
by checking a stated predicate where the committed drivers never reach. Many rows depend
on a generator or oracle from sections 10 and 11, and their fold-in cost
assumes that dependency folds in too. The stack-safety probes are here,
since "no traversal recurses on input depth" is a property.""",
 "COST": """A cost instrument adds coverage when it sees work that the board, the
focused meters, and the fuzz-fit bands miss
([`00-baseline.md`](00-baseline.md) section 5). Five rows belong to the open
`min_ticks` heap defect that question 87 stopped (09.A4, 03.5, 09.A9,
09.A15, 03.7). They rank by what they would add once that work resumes,
and 09.A4 shows the defect is present in `main`'s code at sizes the board
does not sample.""",
 "TGT": """These instruments make 32-bit behavior and `usize` invariance visible
([`00-baseline.md`](00-baseline.md) section 6). Most run in the wasm32-pins
workspace, where one case costs tens to hundreds of seconds and #57 gives
the workspace a 20-minute limit. Your ruling that neither crate promises an
input size removes most of the value of the decoder-growth rows.""",
 "META": """These instruments measure or calibrate other instruments: censuses, mutant
sets, survivor records, and calibration harnesses. Their value is
calibration of whatever folds in, not new coverage of the crates. Almost
every mutant set edits production source and has no committed home;
decision 3 in the main file asks where such sets should live.""",
 "PROBE": """These are one-off tools and settled investigations. They are listed so
that nothing the audit built goes uncounted; none adds coverage to fold
in.""",
}


def write_files(outdir):
    import os
    t = cat_tables()
    for c, (fname, title, k, tbl) in t.items():
        num = fname[:2]
        body = [CAVEAT, "", f"# {num}. {title}", "", INTRO[c], "", COMMON, "",
                f"This category holds {k} of the {len(R)} rows.", "", tbl, ""]
        with open(os.path.join(outdir, fname), "w") as f:
            f.write("\n".join(body))


def overall_prefixed(prefix):
    lines = ["| Position | Instrument | Category | Why it sits here |", "|---:|---|---|---|"]
    for pos, x in enumerate(R, 1):
        i, c, name, *_rest, reason = x
        lines.append(f"| {pos} | [{i}]({prefix}{SEC[i[:2]]}) {name} | [{CATS[c][1]}]({prefix}{CATS[c][0]}) | {reason} |")
    return "\n".join(lines)


if __name__ == "__main__" and sys.argv[1:2] == ["write"]:
    write_files(sys.argv[2])
if __name__ == "__main__" and sys.argv[1:2] == ["overall-main"]:
    print(overall_prefixed("instrument-rescue/"))
