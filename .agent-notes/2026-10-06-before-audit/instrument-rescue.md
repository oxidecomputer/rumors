<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the instrument rescue's integrator, merging nine cataloguers' sections (instrument-rescue/01 to 09) and its own baseline (00); for Finch's review as a whole. -->

# Instrument rescue: every instrument the audit built, ranked

You asked to "rescue the *highest value* instruments which *add additional
coverage* that were already constructed as a part of the work done in this
audit", whether or not they caught a defect, and to treat every lane's
instruments with thoroughness. Nine cataloguers wrote a full entry for every
instrument their lane or role built: 161 entries in all. This document
merges them. It ranks every instrument once, by the coverage it adds against
the work of folding it in, gathers the decisions the fold-ins need from you,
and reconciles the places where lanes built the same thing.

Instruments already committed on `main`, or carried by a ready branch, are
not ranked here; each section lists them in one line, so you can see the
whole picture there. Two pairs of entries are the same instrument
catalogued twice (section 5), so the tables have 159 rows.

## How to read this document

- **Start with section 1**, the decisions only you can make; several
  fold-ins wait on one of them. Then read the top of section 3, the overall
  ranking. Positions 1 to 18 are small fold-ins with real new coverage.
- **Each instrument has an ID** of the form `NN.k`: entry `k` of section
  `NN`, or `09.Ak` for entry `Ak` of section 09. Every ID links to its
  section, where the full entry gives what the instrument is, its reach
  against the committed suite, its evidence, its fold-in cost and runtime,
  its overlaps, and its dependencies.
- **`#n`** is entry `n` of `QUESTIONS.md`. "Slot 23", "slot 26", "slot 33",
  "slot 45", "slot 46", and "slot 47" are branches your rulings started that
  are not yet ready.
- **Marks.** Every claim in the sections is marked *verified*, *reported*,
  or *inferred*. The tables here condense those entries and carry their
  marks; claims I checked again myself say *verified here*.
- **The committed suite's reach**, against which every entry measures
  itself, is in [`instrument-rescue/00-baseline.md`](instrument-rescue/00-baseline.md):
  what the committed generators produce, which oracles exist and what each
  shares with production, which laws are registered, and what the cost
  instruments measure.

| File | Contents |
|---|---|
| [`00-baseline.md`](instrument-rescue/00-baseline.md) | The committed suite's reach at `main` |
| [`01-identity.md`](instrument-rescue/01-identity.md) to [`08-adequacy.md`](instrument-rescue/08-adequacy.md) | Each lane's instruments, full entries |
| [`09-build-and-review-probes.md`](instrument-rescue/09-build-and-review-probes.md) | Instruments built by builders, fixers, demonstrators, and reviewers |
| [`10-generators.md`](instrument-rescue/10-generators.md) to [`16-probes.md`](instrument-rescue/16-probes.md) | One ranked table per category |
| [`17-overlaps.md`](instrument-rescue/17-overlaps.md) | Where lanes built the same thing, and what one fold-in would keep |
| [`18-corrections.md`](instrument-rescue/18-corrections.md) | Corrections to the audit's records, found while cataloguing |

## 1. Decisions you face

Each decision says what waits on it and gives my recommendation.

1. **May the 282 classified mutation survivors be committed as a baseline?**
   - *What waits:* 08.4 (position 87) and, through it, any later campaign's
     comparison with this one.
   - *The conflict:* your doctrine says no mechanism may accept known
     failures. A committed list that a later campaign must match would
     accept the 41 trait-spelling survivors, the 96 cost-only survivors, and
     the ones held by decision. The 69 equivalent and 41 unreachable entries
     are different: each carries a one-line argument that the mutant cannot
     change behavior, which reads as a declared model stated positively.
   - *Options:* (a) keep the list as an unenforced record in the lane
     records, as it is now; (b) commit only the equivalence and
     unreachability arguments, as declared models, and use the cost-only
     class as calibration data for the cost instruments; (c) commit the whole
     list as an enforced baseline.
   - *Recommendation:* (a), with the cost-only class used as calibration
     data whenever a cost instrument is folded in. (c) conflicts with the
     rule, and (b) adds a document whose value depends on decision 3.
2. **Will any decoder promise which error a doubly malformed input
   returns?**
   - *What waits:* 06.1 (position 21), 06.9 (118), 08.7 (119), and #67's
     surviving mutant B.
   - *The context:* `Decode`'s rustdoc tells callers to handle any
     applicable variant unless a decoder documents a precedence; only span
     documents one. 06.1, the specification codec, admits only the
     first-detected class in three places, which is stricter than that
     contract, so folding it in as written would pin today's detection
     order (18, item 1).
   - *Options:* (a) promise nothing: widen 06.1's three narrowed verdicts to
     every applicable class, and leave 06.9 and 08.7 unbuilt; (b) document
     "the first defect a sequential parse meets" for every decoder, then keep
     06.1's verdicts and fold in 06.9 and 08.7.
   - *Recommendation:* (a), unless you want callers to read `Truncated` as
     "read more", which is the adequacy lane's argument for (b).
3. **Where should mutants of production code live, if anywhere?**
   - *What waits:* about eighteen calibration sets (15, positions 87 to
     102), and the known-bad demonstrations that several fold-ins need, such
     as E8 for 04.2.
   - *The context:* `main` keeps known-bad *reference implementations* for
     its oracles and descriptors, but no mutant of production source. Every
     set here edits production source, by string swap or by an environment
     switch.
   - *Options:* (a) keep the sets as records in `.agent-notes/`, now durable
     (section 6); (b) a `just` recipe outside the gate that applies string
     swaps to a scratch copy and requires named tests to fail, as 08.2 does
     for the wasm32 pins; (c) compile-time switches that compile to nothing
     when unset (09.A19), at the cost of extra source lines in production
     files; (d) test-only copies of each mutated function, per instrument
     (09.A12's pattern).
   - *Recommendation:* (b) for 08.2 alone, since it calibrates the audit's
     instrument of record for 32-bit targets; (a) for every other set.
4. **Do you want #83's narrowed-out laws back?**
   - *The context:* 09.A2 (position 4) is a set of registry laws in commit
     `d653bb83` that #83 later reverted. The commit is now pinned as
     `archive/rescue-d653bb83` (verified here), so squashing #83 at landing,
     as notice 96 proposes, no longer loses it.
   - *Recommendation:* squash #83 as proposed, then restore the laws you
     want in a follow-up branch by reverting `735b6b2e`'s hunks.
5. **Should `fix/before-wasm32-buffer-growth` be kept?**
   - *The context:* it holds `8208efaeb`, the reviewed rewrites of 06.2 and
     06.11 (verified here, `git branch --contains`). #53's entry says the
     branch is deleted at retirement unless you keep it.
   - *Recommendation:* pin `8208efaeb` as an `archive/rescue-*` branch like
     the other five, then delete the branch as planned. 06.2 (position 61)
     is the only 32-bit borsh check built.
6. **Should encoders promise what a failing writer has received?** 06.6
   (position 30) asserts that a failing encode has written a prefix of the
   canonical encoding, which no `# Errors` section promises
   (`follow-ups.md` records it). *Recommendation:* fold in only the
   assertions on error propagation and on exact output when the writer has
   room, and leave the prefix property unpromised unless a caller needs it.
7. **Is a "foreign tick" in your model?** 03.2's history world lets one
   clock's party tick a version that clock never held. The `min_ticks` floor
   still holds there, but the step is not a `Clock` operation.
   *Recommendation:* fold 03.2 into `min_ticks_floors_every_history` over
   `world_strategy`, which needs no foreign tick and already gives about 40
   times as many tight observations; decide on foreign ticks only if you
   want the production-only world as well.
8. **Should every folded-in generator carry a reach floor?** The survey cut
   generator-reach floors for lack of a named failure. With notice 88's
   correction, each generator folded in from section 10 can carry a census
   floor like `generator_classes_stay_under_mass`, so that a later edit
   cannot silently narrow it. *Recommendation:* yes, with one census module
   built from 08.1 and the metrics in 17, section 11.
9. **Two built instruments were declined under the caution notice 88
   narrowed.** #81 declined 09.A1 and #82 declined 09.A13's swarm bound
   (both verified here in the ready entries). *Recommendation:* take both;
   they rank 2 and 33. Two other declined suggestions, drawing position 0
   more often in #86's model test and weighting #38's value pool more
   heavily, were never built, so they are new construction and outside this
   rescue.
10. **Do you want eight new dev-dependencies for real serde formats?**
    09.A20 (position 78) round-trips `Clock`, `Span`, and `Ranked` through
    csv, MessagePack, bencode, XML, RON, YAML, TOML, and JSON.
    *Recommendation:* not now; if ever, in a detached workspace like
    `surfacecheck`.

One fact needs no new decision but bears on question 87: 09.A4 measures
`main`'s own `min_ticks` code above the board's heap ceiling at 177 of 756
sizes the board does not sample, up to 307.67 bytes per input byte (18,
item 3). The instruments that would hold a fix (09.A4, 03.5 with 09.A8,
09.A9, 09.A15, 03.7) wait on that question.

## 2. How the ranking works

Each position weighs two things:

- **Coverage added**: inputs the committed generators never reach,
  independence from code the committed oracles share, or a predicate no
  committed test states, measured against `00-baseline.md` and against what
  the ready branches already carry.
- **Fold-in cost**: the work to port the instrument into the shared
  instruments (`testing::generators`, `testing::oracles`, `testing::laws`,
  `testing::diff_ops`, `testing::exhaustive`, the board, the fuzz-fit
  harness) rather than as a parallel harness; its runtime against nextest's
  300-second limit; new dependencies; documentation to the test standard;
  and any wait on an unbuilt branch or an open decision.

Whether an instrument caught a defect does not enter the ranking. The
positions fall into rough bands:

| Positions | What they hold |
|---|---|
| 1 to 18 | Small fold-ins, under about a hundred lines or a revert and seconds of runtime, with real new coverage |
| 19 to 31 | The independent models, the pool model, the specification codec, the fuel-ladder extension, and mid-sized checks |
| 32 to 78 | Second-order additions: properties and generators that ride on the models, small checks behind ready branches, and target-dependence additions |
| 79 to 86 | Instruments waiting on question 87, question 91's allocator, or slots 23 and 33 |
| 87 to 103 | Calibration sets and records, valuable for calibrating whatever folds in |
| 104 to 159 | Low value as fold-ins, or nothing to fold in: settled, superseded, or spent |

Section 3 orders all 159 rows; sections 10 to 16 order each category, with
more columns. Both are generated from one ranked list,
`probes-preserved/ranker/gen_rescue.py`, so a category's order always
agrees with the overall one. To move an instrument, edit its place in
that list; then, from this directory, `python3
probes-preserved/ranker/gen_rescue.py write instrument-rescue` rewrites
sections 10 to 16, and `python3 probes-preserved/ranker/gen_rescue.py
overall-main` prints the table that replaces section 3's.

## 3. The overall ranking

| Position | Instrument | Category | Why it sits here |
|---:|---|---|---|
| 1 | [05.4](instrument-rescue/05-spans.md) Exhaustive boolean cube (`l5_exhaustive_boolean_cube`) | [Properties and predicates](instrument-rescue/12-properties.md) | It checks exact coverage totally and deterministically on a lattice twice as wide as the committed grid, with no new oracle, in about 7 seconds. |
| 2 | [09.A1](instrument-rescue/09-build-and-review-probes.md) Exhaustive agreement test for the laws' multiplicity comparison | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | Written and half a second long, it closes a measured gap in the comparison every multiplicity law rests on. |
| 3 | [04.5](instrument-rescue/04-measures.md) Rational `Rank` oracle `Q` with `render`, `arb_q`, `arb_related_q` | [Independent models and oracles](instrument-rescue/11-oracles.md) | About 60 lines and a second driver give all 15 rank laws the integers, zero, and equal values they never see. |
| 4 | [09.A2](instrument-rescue/09-build-and-review-probes.md) Trait-impl laws that #83 narrowed out (`d653bb83`) | [Properties and predicates](instrument-rescue/12-properties.md) | Reverting part of one archived commit restores checks on spellings the campaign showed survive as constant bodies. |
| 5 | [04.1](instrument-rescue/04-measures.md) Multi-scale partition generator with related-pair modes (`arb_leaves`, `arb_related`) | [Generators and reach](instrument-rescue/10-generators.md) | As a third input to the existing law drivers, it puts every committed version law on the deferral and exact-tie paths no driver reaches. |
| 6 | [01.1](instrument-rescue/01-identity.md) Party generators `arb_set`, `arb_disjoint` | [Generators and reach](instrument-rescue/10-generators.md) | A small builder moves every party law and tree-oracle comparison from depth 4 to depth 80 and supplies the only large accepted families. |
| 7 | [03.2](instrument-rescue/03-events.md) `min_ticks` bounded by each clock's causal past | [Properties and predicates](instrument-rescue/12-properties.md) | A small change to one committed test multiplies its chances of seeing an overcount about fortyfold. |
| 8 | [05.6](instrument-rescue/05-spans.md) Span-to-query coverage agreement (`l5_span_query_agree`) | [Properties and predicates](instrument-rescue/12-properties.md) | One law with no model states a relation between two public APIs that nothing checks. |
| 9 | [03.1](instrument-rescue/03-events.md) `min_ticks` attained by a constructed history | [Properties and predicates](instrument-rescue/12-properties.md) | Two cheap properties state attainment and reachability, which no committed test asks. |
| 10 | [08.3](instrument-rescue/08-adequacy.md) Gamma-window counterexample test (also 09.A23) | [Properties and predicates](instrument-rescue/12-properties.md) | A finished 50-millisecond test guards the window's contract before any caller relies on it. |
| 11 | [04.2](instrument-rescue/04-measures.md) `Rank` sum-order cost probe | [Cost and resource instruments](instrument-rescue/13-cost.md) | A 0.3-second check restores coverage a closed fix lost, which no current meter can see. |
| 12 | [09.A3](instrument-rescue/09-build-and-review-probes.md) Touch-bound worst-case families (F1 to F7) | [Cost and resource instruments](instrument-rescue/13-cost.md) | Fixed worst cases make #37's property fail near its derivation, for 70 milliseconds. |
| 13 | [03.3](instrument-rescue/03-events.md) Tick claims #74 did not carry | [Properties and predicates](instrument-rescue/12-properties.md) | A few lines add two missing laws and run three existing ones where the tick walk's state is richest. |
| 14 | [02.3](instrument-rescue/02-algebra.md) Deep random partitions (`gen::partition`, `limits`) | [Generators and reach](instrument-rescue/10-generators.md) | It is the cheapest built way to give the algebra's committed properties random topology past depth 4. |
| 15 | [04.8](instrument-rescue/04-measures.md) `Count` boundary sweep | [Properties and predicates](instrument-rescue/12-properties.md) | A 50-millisecond exhaustive test checks what the committed property can neither reach nor judge independently. |
| 16 | [08.6](instrument-rescue/08-adequacy.md) `usize` signature census (`usize_census.py`) | [Target-dependence instruments](instrument-rescue/14-target.md) | It turns your `usize`-invariance rule, which nothing checks, into a reviewed event for every new `usize` in `before`'s signatures. |
| 17 | [08.1](instrument-rescue/08-adequacy.md) Generator census (`l8_census.rs`) | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | Floors from it would fail a generator change that silently narrows the relational classes the differentials depend on. |
| 18 | [04.6](instrument-rescue/04-measures.md) `Rank` value property | [Properties and predicates](instrument-rescue/12-properties.md) | A sub-second property is the only test of `Rank`'s formatter flags as a family and of `AddAssign<&Rank>`. |
| 19 | [02.1](instrument-rescue/02-algebra.md) Leaf-list model of versions and parties | [Independent models and oracles](instrument-rescue/11-oracles.md) | It is the only built oracle that judges the algebra on deep inputs without sharing the tree representation or the writer. |
| 20 | [04.3](instrument-rescue/04-measures.md) Flat sweep oracle for rank, distance, lag, and `rank_cmp` | [Independent models and oracles](instrument-rescue/11-oracles.md) | It adds the measures to the leaf-list model, at depths the committed oracles cannot reach. |
| 21 | [06.1](instrument-rescue/06-codecs.md) Specification-codec decode differential | [Independent models and oracles](instrument-rescue/11-oracles.md) | It is the broadest independent check in the audit, at the highest port cost and one decision about error precedence. |
| 22 | [07.1](instrument-rescue/07-suanpan.md) Pool model's exact-oracle checks | [Independent models and oracles](instrument-rescue/11-oracles.md) | About 360 lines put a value oracle over heavily cancelled, wide, history-laden states no committed value test reaches. |
| 23 | [07.2](instrument-rescue/07-suanpan.md) Fuel matrix's operations outside the ladder | [Cost and resource instruments](instrument-rescue/13-cost.md) | It extends a committed linearity check from three operations to six more, through guest exports that already exist. |
| 24 | [05.1](instrument-rescue/05-spans.md) Pairwise-concurrent holes (`with_bumps` and the antichain properties) | [Generators and reach](instrument-rescue/10-generators.md) | It gives the largest exact-coverage gain for queries, once the census oracle exists. |
| 25 | [05.2](instrument-rescue/05-spans.md) Exact coverage oracle (`census`, `sublattice`) | [Independent models and oracles](instrument-rescue/11-oracles.md) | No exact-coverage check can leave a complete grid without it. |
| 26 | [05.7](instrument-rescue/05-spans.md) Vector model and verdict oracles (spans and queries) | [Independent models and oracles](instrument-rescue/11-oracles.md) | It removes the common mode between production comparison and the span walks every committed span oracle shares. |
| 27 | [01.2](instrument-rescue/01-identity.md) Interval-set model of `Party` | [Independent models and oracles](instrument-rescue/11-oracles.md) | It is the only exact party oracle independent of the tree recursion at depths the function space cannot reach. |
| 28 | [04.4](instrument-rescue/04-measures.md) `Ranked::cmp` tie-cost probe | [Cost and resource instruments](instrument-rescue/13-cost.md) | It is the only built meter of a closed fix's attainable worst case. |
| 29 | [06.4](instrument-rescue/06-codecs.md) Misbehaving-reader probe (`Chaos`) | [Properties and predicates](instrument-rescue/12-properties.md) | About 100 lines extend #78's reader discipline to every decoder. |
| 30 | [06.6](instrument-rescue/06-codecs.md) Failing-writer probe (`Sink`) | [Properties and predicates](instrument-rescue/12-properties.md) | It is the only check that encoders propagate writer failures, for a small test and one decision. |
| 31 | [06.3](instrument-rescue/06-codecs.md) Hostile `Rank` and `Ranked` rejection families | [Cost and resource instruments](instrument-rescue/13-cost.md) | It closes the board's one unmetered decoder rejection path with families already built. |
| 32 | [09.A10](instrument-rescue/09-build-and-review-probes.md) Memo reuse property for consecutive pre-scans | [Properties and predicates](instrument-rescue/12-properties.md) | One property states the memo's contract where its defects live. |
| 33 | [09.A13](instrument-rescue/09-build-and-review-probes.md) Swarm close bound for the range-minima property | [Generators and reach](instrument-rescue/10-generators.md) | A one-line change makes #82's own mutant fail every run. |
| 34 | [09.A26](instrument-rescue/09-build-and-review-probes.md) Fork-plan tails at every depth to 140 | [Properties and predicates](instrument-rescue/12-properties.md) | It widens #43's four fixed depths to every depth, cheaply. |
| 35 | [01.6](instrument-rescue/01-identity.md) Wide-count fork properties | [Properties and predicates](instrument-rescue/12-properties.md) | It is the only check that fork shares stay balanced and exact past `usize`. |
| 36 | [04.7](instrument-rescue/04-measures.md) `DEFER_HITS` tap and coverage census | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | Four lines make deferral liveness checkable, which 04.1's floor needs. |
| 37 | [02.7](instrument-rescue/02-algebra.md) Party region generator (`gen::party`) | [Generators and reach](instrument-rescue/10-generators.md) | It gives the masked comparisons random deep masks; 01.1 is the alternative. |
| 38 | [02.5](instrument-rescue/02-algebra.md) Boundary-height palette (`gen::level`, `gen::levels`) | [Generators and reach](instrument-rescue/10-generators.md) | It is cheap, and no committed generator aims heights at the code-width boundary in random topology. |
| 39 | [02.4](instrument-rescue/02-algebra.md) Correlated pair families (`gen::pair`) | [Generators and reach](instrument-rescue/10-generators.md) | It generates the tie-heavy, collapse-heavy pairs the writer's cascades need at depth. |
| 40 | [02.2](instrument-rescue/02-algebra.md) `algebra_matches_model` and its checks | [Properties and predicates](instrument-rescue/12-properties.md) | It applies the independent model to every algebra entry point at once. |
| 41 | [05.5](instrument-rescue/05-spans.md) Query membership and exact coverage across worlds | [Properties and predicates](instrument-rescue/12-properties.md) | It checks exact coverage of polar queries where the committed suite checks soundness only. |
| 42 | [05.3](instrument-rescue/05-spans.md) Census validator (`l5_census_is_exact_against_a_finer_universe`) | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It turns a prose exactness argument into a sub-second check. |
| 43 | [05.9](instrument-rescue/05-spans.md) Grid worlds and near-equal wide heights | [Generators and reach](instrument-rescue/10-generators.md) | It makes the rare placements four to six times more common. |
| 44 | [05.8](instrument-rescue/05-spans.md) Deep and random span shapes (`Layout`, `arb_layout`) | [Generators and reach](instrument-rescue/10-generators.md) | It reaches ten times the committed generator's depth, for the walks whose cursors depth stresses. |
| 45 | [05.10](instrument-rescue/05-spans.md) Span verdicts against vectors | [Properties and predicates](instrument-rescue/12-properties.md) | It is a second, independent oracle for every span verdict at depth. |
| 46 | [05.11](instrument-rescue/05-spans.md) Clause model and conjunction folds | [Generators and reach](instrument-rescue/10-generators.md) | It is worth folding only as a delta to the committed clause enums. |
| 47 | [01.5](instrument-rescue/01-identity.md) Set algebra, fork, splits, and sync against the interval model | [Properties and predicates](instrument-rescue/12-properties.md) | It checks every binary identity operation deeply and independently, though most of its predicates exist at depth 4. |
| 48 | [01.4](instrument-rescue/01-identity.md) `join_all` and `sync_all` against the interval model | [Properties and predicates](instrument-rescue/12-properties.md) | It remains an independent check of `sync_all`'s dealing once the ready branches take its other parts. |
| 49 | [01.3](instrument-rescue/01-identity.md) Rule-respecting history driver | [Generators and reach](instrument-rescue/10-generators.md) | It is the only driver of the multi-party operations through long rule-respecting histories, at a real porting cost. |
| 50 | [06.5](instrument-rescue/06-codecs.md) Near-valid encoding generators | [Generators and reach](instrument-rescue/10-generators.md) | It serves without the model as input to a decoder totality and agreement property, or as 06.1's input. |
| 51 | [08.2](instrument-rescue/08-adequacy.md) wasm32 pin injection table and driver | [Target-dependence instruments](instrument-rescue/14-target.md) | It is the known-bad set for the audit's instrument of record on 32-bit targets. |
| 52 | [09.A6](instrument-rescue/09-build-and-review-probes.md) Ceiling-rule auditor | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It makes the one-rule ruling arithmetic that fails on drift in either direction. |
| 53 | [09.A7](instrument-rescue/09-build-and-review-probes.md) libtest main-thread race probe | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It is the known-bad demonstration and acceptance test for the planned allocator. |
| 54 | [09.A5](instrument-rescue/09-build-and-review-probes.md) Fuel comparison between two commits | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | Four branches rebuilt this comparison by hand; one mode would serve every later change. |
| 55 | [07.5](instrument-rescue/07-suanpan.md) wasm32 landing case 5 | [Target-dependence instruments](instrument-rescue/14-target.md) | It adds a cheap extra boundary to the 32-bit landing pin. |
| 56 | [09.A11](instrument-rescue/09-build-and-review-probes.md) Linear-memory reading after each wasm32 pin case | [Target-dependence instruments](instrument-rescue/14-target.md) | It would warn before a pin runs out of address space and tests the allocator instead. |
| 57 | [04.16](instrument-rescue/04-measures.md) wasm32 memory-size readout | [Target-dependence instruments](instrument-rescue/14-target.md) | It is a second build of 09.A11's reading, so only one of the two should be folded in. |
| 58 | [09.A27](instrument-rescue/09-build-and-review-probes.md) Shifted-zero checks through every operator spelling | [Properties and predicates](instrument-rescue/12-properties.md) | It checks every spelling keeps a guarantee that today holds only because they share one route. |
| 59 | [09.A22](instrument-rescue/09-build-and-review-probes.md) Counterexamples behind `Clock::from_parts`'s warning | [Properties and predicates](instrument-rescue/12-properties.md) | Existing cases keep a public warning true. |
| 60 | [09.A24](instrument-rescue/09-build-and-review-probes.md) Guest panic records under memory exhaustion | [Target-dependence instruments](instrument-rescue/14-target.md) | It pins what the wasm32 failure channel can tell you. |
| 61 | [06.2](instrument-rescue/06-codecs.md) wasm32 borsh probe over a wide one-leaf version | [Target-dependence instruments](instrument-rescue/14-target.md) | It is the only 32-bit borsh check, and its reviewed form sits on a branch slated for deletion. |
| 62 | [09.A14](instrument-rescue/09-build-and-review-probes.md) wasm32 differential over accumulator histories at the stability-width boundary | [Target-dependence instruments](instrument-rescue/14-target.md) | It varies histories on 32-bit, but its oracle is weaker than #50's pin against its own defect. |
| 63 | [03.9](instrument-rescue/03-events.md) Random counter search for tick and `min_ticks` | [Cost and resource instruments](instrument-rescue/13-cost.md) | It extends two ceilings from fixed families to random inputs where tick's memo is busiest. |
| 64 | [04.12](instrument-rescue/04-measures.md) Integrator cost search | [Cost and resource instruments](instrument-rescue/13-cost.md) | It searches random wide shapes against the measures' touch ceiling but only prints today. |
| 65 | [01.8](instrument-rescue/01-identity.md) Identity scan-cost probe | [Cost and resource instruments](instrument-rescue/13-cost.md) | It supplies two pessimal identity families the board lacks, ready to become rows. |
| 66 | [03.4](instrument-rescue/03-events.md) Deep co-generated spines checked without an oracle | [Properties and predicates](instrument-rescue/12-properties.md) | The only random tick inputs thousands of levels deep, at a cost that suits an ignored test. |
| 67 | [06.7](instrument-rescue/06-codecs.md) Encoder agreement with an independent pointwise model | [Properties and predicates](instrument-rescue/12-properties.md) | Its byte-level encoder checks are largely duplicated by 02.6 with 02.1. |
| 68 | [02.6](instrument-rescue/02-algebra.md) Independent canonical encoders (`encode_version`, `encode_party`) | [Independent models and oracles](instrument-rescue/11-oracles.md) | It is the wire-specification half of 02.1, and 06.1's codec would serve as well. |
| 69 | [05.12](instrument-rescue/05-spans.md) Projected-span views against masked vectors | [Properties and predicates](instrument-rescue/12-properties.md) | It is a modest second oracle for projected views, weakened by its verdict mix. |
| 70 | [01.10](instrument-rescue/01-identity.md) Overlay refinement property | [Properties and predicates](instrument-rescue/12-properties.md) | Its added depth comes mostly from the party side, which 01.1 already feeds. |
| 71 | [04.13](instrument-rescue/04-measures.md) Seven-route rank property | [Properties and predicates](instrument-rescue/12-properties.md) | It gives every version's rank a cheap second route. |
| 72 | [04.10](instrument-rescue/04-measures.md) Pair predicate set over the flat oracle | [Properties and predicates](instrument-rescue/12-properties.md) | It holds little of its own beyond the combination of 04.1 and 04.3. |
| 73 | [04.14](instrument-rescue/04-measures.md) `Sum` over any order against the rational oracle | [Properties and predicates](instrument-rescue/12-properties.md) | It adds a second reference for a property the suite already states. |
| 74 | [06.8](instrument-rescue/06-codecs.md) Writer copy-path harness | [Properties and predicates](instrument-rescue/12-properties.md) | It is an independent oracle on a path the suite already exercises heavily. |
| 75 | [05.14](instrument-rescue/05-spans.md) Span algebra against vectors | [Properties and predicates](instrument-rescue/12-properties.md) | The committed laws already cover every spelling it checks. |
| 76 | [05.15](instrument-rescue/05-spans.md) Organic overlays | [Generators and reach](instrument-rescue/10-generators.md) | Organic shapes are shallow, and the other generators already cover them. |
| 77 | [06.15](instrument-rescue/06-codecs.md) CBOR framing probe | [Properties and predicates](instrument-rescue/12-properties.md) | It is a one-line guard for a documented bridging that comes from the format crate. |
| 78 | [09.A20](instrument-rescue/09-build-and-review-probes.md) Serde round trips through third-party formats | [Properties and predicates](instrument-rescue/12-properties.md) | It is the only check against deserializers applications use, at the price of eight dependencies. |
| 79 | [09.A4](instrument-rescue/09-build-and-review-probes.md) `min_ticks` heap probe over right spines, with a closed-form value oracle | [Cost and resource instruments](instrument-rescue/13-cost.md) | It is the widest measurement of an open defect on `main`, and it can land only beside a fix. |
| 80 | [03.5](instrument-rescue/03-events.md) Jump-entered rising spine (also 09.A8, its board-family port in `08573e159`) | [Cost and resource instruments](instrument-rescue/13-cost.md) | It is the board family for `main`'s open `min_ticks` heap defect, and it lands with a fix or a declared model. |
| 81 | [09.A9](instrument-rescue/09-build-and-review-probes.md) Limb work of `min_ticks` on wide-offset combs | [Cost and resource instruments](instrument-rescue/13-cost.md) | It guards any future `min_ticks` redesign against work no current meter sees. |
| 82 | [09.A15](instrument-rescue/09-build-and-review-probes.md) Real-scale stepped spine for `min_ticks` | [Cost and resource instruments](instrument-rescue/13-cost.md) | It is the probe the write-up asks for before any redesign is trusted. |
| 83 | [03.7](instrument-rescue/03-events.md) Plain rising spine (heap) | [Cost and resource instruments](instrument-rescue/13-cost.md) | It shows the doubling mechanism alone and is secondary to 03.5. |
| 84 | [03.8](instrument-rescue/03-events.md) Random tick heap search | [Cost and resource instruments](instrument-rescue/13-cost.md) | It extends the heap ceiling to random inputs once the allocator exists. |
| 85 | [09.A25](instrument-rescue/09-build-and-review-probes.md) Heap size sweeps for single operations | [Cost and resource instruments](instrument-rescue/13-cost.md) | It samples two regimes the board does not, behind two unbuilt branches. |
| 86 | [01.9](instrument-rescue/01-identity.md) Identity heap-retention probe | [Cost and resource instruments](instrument-rescue/13-cost.md) | It is the measurement behind S1 and the only record of O1. |
| 87 | [08.4](instrument-rescue/08-adequacy.md) Classified mutation survivors (282) | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It answers whether the suite is still as strong as it was, if you rule that it may be committed. |
| 88 | [08.5](instrument-rescue/08-adequacy.md) Mutation campaign tooling | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It is too slow for every commit and suited to a release. |
| 89 | [09.A19](instrument-rescue/09-build-and-review-probes.md) Compile-time mutant switches (`option_env!`) for cost-only survivors | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It is the template for a committed home for production mutants, if you want one. |
| 90 | [09.A12](instrument-rescue/09-build-and-review-probes.md) Mutant schema for `Rank::decode`'s reader paths | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It is the only evidence that #78's oracle is neither too loose nor too strict. |
| 91 | [09.A16](instrument-rescue/09-build-and-review-probes.md) Mutant schema for suanpan's readout carry classes | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It shows #64's table is the only test of the readout's touch cost. |
| 92 | [09.A17](instrument-rescue/09-build-and-review-probes.md) Stack-recursion mutants and frame-size probes | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It is the only calibration of the deep-input tests, which stay after the guard's removal. |
| 93 | [09.A18](instrument-rescue/09-build-and-review-probes.md) Mutants of suanpan's written-position set | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It calibrates #86 and shows where #86's generator thins out. |
| 94 | [07.3](instrument-rescue/07-suanpan.md) Suanpan mutant schema and calibration scripts (27 mutants) | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It is the suanpan lane's only record of which instrument kills which hand-made defect. |
| 95 | [03.6](instrument-rescue/03-events.md) Events calibration mutant set (M1 to M23) | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It is a calibration set for any future change to the tick walk. |
| 96 | [01.12](instrument-rescue/01-identity.md) Identity mutant lists and calibration driver | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It holds known-bad identity implementations, valuable if you want calibration kept as a running check. |
| 97 | [02.13](instrument-rescue/02-algebra.md) Algebra round-1 mutants and `mutate.py` | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It is the calibration set 02.1 needs if folded in. |
| 98 | [04.11](instrument-rescue/04-measures.md) Integrator mutant patch (12 mutants) | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It is the calibration set for a folded-in integrator instrument. |
| 99 | [04.17](instrument-rescue/04-measures.md) `Rank` mutants R1 to R7 and E8 (source not kept) | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | Only E8 matters, as the known-bad input 04.2's fold-in needs. |
| 100 | [05.17](instrument-rescue/05-spans.md) Spans calibration mutants (M1 to M20, table only) | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It is a ready calibration list for whatever is folded in. |
| 101 | [06.10](instrument-rescue/06-codecs.md) Decoder mutation schemas (M1 to M17) | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It records which decoder checks are thin. |
| 102 | [09.A36](instrument-rescue/09-build-and-review-probes.md) Calibration sets for carried tests (24 sets) | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It is the only way to rerun the ready branches' calibrations after the code changes. |
| 103 | [04.9](instrument-rescue/04-measures.md) Schedule-independence probes E13 to E15 (source lost) | [Properties and predicates](instrument-rescue/12-properties.md) | It is the evidence behind five survivor classifications, and keeping it needs a rebuild. |
| 104 | [07.4](instrument-rescue/07-suanpan.md) Hill-climbing touch adversary | [Cost and resource instruments](instrument-rescue/13-cost.md) | It is a tool for re-measuring #37's margin, not a regression check. |
| 105 | [02.10](instrument-rescue/02-algebra.md) Fragmented-mask cost families | [Cost and resource instruments](instrument-rescue/13-cost.md) | It is a differently shaped mask adversary for walks the board already attacks. |
| 106 | [05.13](instrument-rescue/05-spans.md) Carry-boundary touch family | [Cost and resource instruments](instrument-rescue/13-cost.md) | It is a plausible board family that has never failed. |
| 107 | [02.11](instrument-rescue/02-algebra.md) Carry-ripple cost families | [Cost and resource instruments](instrument-rescue/13-cost.md) | It restates the board's cliff comb in another topology. |
| 108 | [01.7](instrument-rescue/01-identity.md) Deep identity probe on a 256 KiB stack | [Properties and predicates](instrument-rescue/12-properties.md) | Its one unduplicated regime takes a few lines to add to #75. |
| 109 | [01.13](instrument-rescue/01-identity.md) Deep-party builders | [Generators and reach](instrument-rescue/10-generators.md) | It is worth taking only together with 01.7, 01.8, or 01.9. |
| 110 | [09.A21](instrument-rescue/09-build-and-review-probes.md) Board-cell bisection and allocator event log | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It turns unexplained heap movement into a commit and a cause. |
| 111 | [08.8](instrument-rescue/08-adequacy.md) Probe copy and hit recorder | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It is worth keeping only if mutation work recurs. |
| 112 | [08.9](instrument-rescue/08-adequacy.md) Branch-coverage map | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It is a periodic report that cannot run on the box. |
| 113 | [01.11](instrument-rescue/01-identity.md) Identity generator statistics printers | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It is useful only if reach floors are adopted. |
| 114 | [02.8](instrument-rescue/02-algebra.md) Writer-path census | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It would hold floors on writer-path reach for a folded-in generator. |
| 115 | [03.13](instrument-rescue/03-events.md) Events generator histograms | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | Only its committed-generator row is worth turning into a floor. |
| 116 | [05.16](instrument-rescue/05-spans.md) Committed-population reach probe (spans) | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It is useful only for placement-mix floors. |
| 117 | [06.12](instrument-rescue/06-codecs.md) Codec reach histograms | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It is useful only as 06.5's liveness floor. |
| 118 | [06.9](instrument-rescue/06-codecs.md) First-detected-class model (unenforced) | [Properties and predicates](instrument-rescue/12-properties.md) | It is ready-made enforcement if you document a precedence. |
| 119 | [08.7](instrument-rescue/08-adequacy.md) Lone-prefix padding tests | [Properties and predicates](instrument-rescue/12-properties.md) | It is worth folding only if you document that precedence. |
| 120 | [09.A31](instrument-rescue/09-build-and-review-probes.md) Scan for self-recursive test helpers | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It is a static check of a rule only review enforces. |
| 121 | [08.11](instrument-rescue/08-adequacy.md) Surface check end-to-end calibration | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | The committed anchors test already catches part of what it would add. |
| 122 | [08.12](instrument-rescue/08-adequacy.md) Fold-pin calibration | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | Its lesson is recorded; it is one member of 08.4's cost class. |
| 123 | [08.10](instrument-rescue/08-adequacy.md) Counter bisect harness | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | What it adds reduces to a few lines of predicate for `git bisect run`. |
| 124 | [09.A28](instrument-rescue/09-build-and-review-probes.md) Probe of the integral's dense-span bound | [Probes and one-off tools](instrument-rescue/16-probes.md) | It shows #72's derived bound is tight to within one. |
| 125 | [09.A29](instrument-rescue/09-build-and-review-probes.md) Compare pin grown past bit `2^32` | [Target-dependence instruments](instrument-rescue/14-target.md) | It reaches past bit `2^32`, but no constructed defect on that path is behind it. |
| 126 | [09.A30](instrument-rescue/09-build-and-review-probes.md) Rank alignment cases on each side of a `2^32` gap | [Target-dependence instruments](instrument-rescue/14-target.md) | It mostly duplicates #63's pin, whose cases already catch the only constructed mutant. |
| 127 | [04.15](instrument-rescue/04-measures.md) wasm32 `Sum` footprint and replay cases | [Target-dependence instruments](instrument-rescue/14-target.md) | It is the acceptance check for any suanpan growth change that targets O6. |
| 128 | [06.11](instrument-rescue/06-codecs.md) wasm32 rank group-stream probe | [Target-dependence instruments](instrument-rescue/14-target.md) | It adds little beyond the committed pin, though its lazy reader is reusable. |
| 129 | [09.A33](instrument-rescue/09-build-and-review-probes.md) Differentials against replaced implementations | [Independent models and oracles](instrument-rescue/11-oracles.md) | They did their job at the change, and keeping them means keeping deleted code as oracles. |
| 130 | [02.9](instrument-rescue/02-algebra.md) Stack-overflow calibration | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | #75's derivation already states a stronger bound. |
| 131 | [03.10](instrument-rescue/03-events.md) Bushy strategy as a property | [Generators and reach](instrument-rescue/10-generators.md) | #74's spine strategy dominates it on every measured metric. |
| 132 | [03.11](instrument-rescue/03-events.md) Second late perturbation | [Generators and reach](instrument-rescue/10-generators.md) | No measurement credits the second perturbation with any catch. |
| 133 | [07.6](instrument-rescue/07-suanpan.md) Offset-comparison accounting probe | [Cost and resource instruments](instrument-rescue/13-cost.md) | It confirmed a cost argument the board already enforces. |
| 134 | [09.A32](instrument-rescue/09-build-and-review-probes.md) Board recipe interrupt behavior on a toy justfile | [Probes and one-off tools](instrument-rescue/16-probes.md) | It is regression evidence for one recipe, with no committed home. |
| 135 | [09.A34](instrument-rescue/09-build-and-review-probes.md) 32-bit decoder-growth instruments (`60d534ed`, `d0a6205f`, `8208efae`) | [Target-dependence instruments](instrument-rescue/14-target.md) | Your input-size ruling leaves them little to check; the streaming synthesizer is the reusable part. |
| 136 | [06.13](instrument-rescue/06-codecs.md) wasm32 reader probe over a wide one-leaf version | [Target-dependence instruments](instrument-rescue/14-target.md) | It is evidence for #53's documentation, not an instrument to keep. |
| 137 | [08.13](instrument-rescue/08-adequacy.md) `num-bigint` formatting probe | [Target-dependence instruments](instrument-rescue/14-target.md) | Its finding is settled on `main` by `ddfabe4cb`. |
| 138 | [01.14](instrument-rescue/01-identity.md) Unoptimized-build deep-test configuration (MB3) | [Target-dependence instruments](instrument-rescue/14-target.md) | You ruled against it under question 68; it is listed because it was built. |
| 139 | [06.14](instrument-rescue/06-codecs.md) Writer mutants and reach probes | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | Its finding became #67, and it checks nothing further. |
| 140 | [02.14](instrument-rescue/02-algebra.md) Survivor and tie-order patches with `run.sh` | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | It records a settled question. |
| 141 | [05.20](instrument-rescue/05-spans.md) Tie-order experiment | [Probes and one-off tools](instrument-rescue/16-probes.md) | The question it answered is settled, and #85 documents the answer. |
| 142 | [03.12](instrument-rescue/03-events.md) Memo-dense tick heap families | [Cost and resource instruments](instrument-rescue/13-cost.md) | The board's memo families already build the same memo traffic. |
| 143 | [03.14](instrument-rescue/03-events.md) Memo reach diagnostic | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | #74's census floors supersede it. |
| 144 | [05.18](instrument-rescue/05-spans.md) Refinement scan-cost probe | [Cost and resource instruments](instrument-rescue/13-cost.md) | #61 used its finding, and #94 meters the same case on the board. |
| 145 | [05.19](instrument-rescue/05-spans.md) Spans behavioral histograms | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | The numbers are worth keeping, but the code's global map is not. |
| 146 | [07.7](instrument-rescue/07-suanpan.md) Readout carry census (`l7-high-log`) | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | #64 tests every readout class directly, so it has nothing left to find. |
| 147 | [07.9](instrument-rescue/07-suanpan.md) Native size probe | [Diagnostics and meta-instruments](instrument-rescue/15-meta.md) | #52's ladder records the same sizes as its denominators. |
| 148 | [08.14](instrument-rescue/08-adequacy.md) Split-writer hunt | [Probes and one-off tools](instrument-rescue/16-probes.md) | Its target disappears when #67 lands. |
| 149 | [08.15](instrument-rescue/08-adequacy.md) libtest invocation probe | [Probes and one-off tools](instrument-rescue/16-probes.md) | Its one finding, about the runner, is recorded. |
| 150 | [06.16](instrument-rescue/06-codecs.md) wasm32 `Vec` growth diagnostic | [Target-dependence instruments](instrument-rescue/14-target.md) | It explains a dissolved defect's mechanism and checks no contract. |
| 151 | [07.8](instrument-rescue/07-suanpan.md) Prototype T (position tags) | [Probes and one-off tools](instrument-rescue/16-probes.md) | It is a partial design that #86's full repair superseded. |
| 152 | [07.10](instrument-rescue/07-suanpan.md) Touch replay probe | [Probes and one-off tools](instrument-rescue/16-probes.md) | It diagnosed one false failure of a shared meter and checks nothing. |
| 153 | [03.15](instrument-rescue/03-events.md) Bridge round trip on one pair | [Probes and one-off tools](instrument-rescue/16-probes.md) | Every bridged differential already depends on the round trip it checks. |
| 154 | [02.12](instrument-rescue/02-algebra.md) Parallel long-run copies `par00` to `par15` | [Probes and one-off tools](instrument-rescue/16-probes.md) | Nextest's high-count profile with `PROPTEST_CASES` runs 02.2 long without it. |
| 155 | [02.15](instrument-rescue/02-algebra.md) `run_on_tree.sh` | [Probes and one-off tools](instrument-rescue/16-probes.md) | It is a runner for uncommitted merges, not an instrument. |
| 156 | [02.16](instrument-rescue/02-algebra.md) `extract.py` | [Probes and one-off tools](instrument-rescue/16-probes.md) | It is a converter used once, with nothing to fold in. |
| 157 | [07.11](instrument-rescue/07-suanpan.md) Suanpan scratch drafts and table scripts | [Probes and one-off tools](instrument-rescue/16-probes.md) | Its drafts' final forms are catalogued as other entries. |
| 158 | [09.A35](instrument-rescue/09-build-and-review-probes.md) Instruments superseded by a ruling or design | [Probes and one-off tools](instrument-rescue/16-probes.md) | None adds coverage at `main` plus the ready branches. |
| 159 | [05.21](instrument-rescue/05-spans.md) Witness heuristic (`witness_coverage`) | [Independent models and oracles](instrument-rescue/11-oracles.md) | Nothing calls it, so deleting it is the only action. |

## 4. The categories

| Category | Rows | Its top three, with overall positions | Table |
|---|---:|---|---|
| Generators and reach | 17 | 04.1 (5), 01.1 (6), 02.3 (14) | [`10-generators.md`](instrument-rescue/10-generators.md) |
| Independent models and oracles | 11 | 04.5 (3), 02.1 (19), 04.3 (20) | [`11-oracles.md`](instrument-rescue/11-oracles.md) |
| Properties and predicates | 36 | 05.4 (1), 09.A2 (4), 03.2 (7) | [`12-properties.md`](instrument-rescue/12-properties.md) |
| Cost and resource instruments | 23 | 04.2 (11), 09.A3 (12), 07.2 (23) | [`13-cost.md`](instrument-rescue/13-cost.md) |
| Target-dependence instruments | 17 | 08.6 (16), 08.2 (51), 07.5 (55) | [`14-target.md`](instrument-rescue/14-target.md) |
| Diagnostics and meta-instruments | 42 | 09.A1 (2), 08.1 (17), 04.7 (36) | [`15-meta.md`](instrument-rescue/15-meta.md) |
| Probes and one-off tools | 13 | 09.A28 (124), 09.A32 (134), 05.20 (141) | [`16-probes.md`](instrument-rescue/16-probes.md) |

## 5. Where lanes built the same thing

[`17-overlaps.md`](instrument-rescue/17-overlaps.md) reconciles every
overlap. The ones that shape a fold-in:

- **One model of versions.** The algebra lane's leaf-list model
  (02.1) and the measures lane's flat sweep (04.3) use the same
  representation, a list of `(depth, height)` leaves with absolute heights;
  they belong in one oracle module, with 04.3's functions as its measures.
  The spans lane's vector model (05.7) is that representation on a common
  refinement, adding spans, queries, and placement; it should be built on
  the merged model or on the function-space oracle, not as a third version
  type. The exact-coverage census (05.2) is an algorithm on top of it.
- **One party oracle.** The identity lane's interval-set model (01.2) is the
  only exact oracle for `fork`, `forks`, the splits, and `sync`; the
  algebra lane's party half is a mask and needs no separate fold-in.
- **One encoder.** The codecs lane's specification codec (06.1) covers all
  six wire types in both directions and subsumes the algebra lane's
  encoders (02.6).
- **One topology generator, several height palettes.** 02.3's partitions
  supply random topology; 02.5, 04.1's step script, and 05.9's alphabets
  aim heights at different boundaries and do not overlap. 01.1 subsumes
  02.7 for parties (inferred, from their reach figures).
- **Two pairs catalogued twice.** 08.3 and 09.A23 are the same file; 03.5
  and 09.A8 are one family before and after its port to the board. Each pair
  has one row.
- **One home question for mutants.** Every lane built a mutant set, and
  they share decision 3.

## 6. Where everything lives

The sections cite paths in the session scratchpad
(`/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/`),
which a reboot erases. Their durable copies, all beside this file in
`.agent-notes/2026-10-06-before-audit/`:

| Scratchpad path | Durable copy |
|---|---|
| `auditor-l<n>/` (each lane auditor's tools and logs) | `lanes/l<n>-<lane>/scratch-tools/` |
| `builder-*/`, `fixer-*/`, `demonstrator-*/`, `reviewer-*/`, `surveyor/` | `probes-preserved/<same name>/` |
| `rescue-l<n>/` (the cataloguers' census code) | `probes-preserved/rescue-l<n>/` |
| `ranker/` (the baseline's census run) | `probes-preserved/ranker/` |

The branches and pinned commits:

- **Explore branches**, at their tips: `explore/l1-identity` `346a82ac9`,
  `explore/l2-algebra` `6748bcd41`, `explore/l3-events` `873f39991`,
  `explore/l4-measures` `2c82c7bac`, `explore/l5-spans` `2874e0c3d`,
  `explore/l6-codecs` `431011b3c`, `explore/l7-suanpan` `5d5e33471`,
  `explore/l8-adequacy` `0acb87bdc` (verified here). Keep them until the
  fold-ins you choose are built.
- **Commits that no other branch reaches**, pinned as
  `archive/rescue-789936bc` (09.A35, the protocol sweeps),
  `archive/rescue-d0a6205f` and `archive/rescue-60d534ed` (09.A34, 32-bit
  growth), `archive/rescue-9123a4d8` (09.A35, strict serde), and
  `archive/rescue-d653bb83` (09.A2, #83's narrowed laws) (verified here).
- **`8208efaeb`** on `fix/before-wasm32-buffer-growth` (decision 5), and
  **`08573e159`** on `fix/before-min-ticks-heap` and the five
  `archive/min-ticks-*` branches (03.5 and 09.A8) (verified here).

**What I added to the durable copies.** While checking the sections'
paths, I found that the copy kept the first file each section 09 entry
names and missed later ones. I copied 34 small files into
`probes-preserved/`, under the scratchpad's layout. They include 09.A1's
applicable diff, 09.A13's swarm variant and its simulation driver, 09.A15's
`stepped_spine` patch, 09.A19's scaffold diff, 09.A20's probe crate, the
rest of 09.A6's and 09.A21's scripts, and the remaining files of 09.A4,
09.A9, 09.A12, 09.A14, 09.A16, 09.A17, 09.A24, 09.A32, 09.A33, and 09.A34.
I also copied the suanpan lane's seven `calibrate*.out` files, the kill
matrix that 07.3 describes, into `lanes/l7-suanpan/scratch-tools/`.

**What is still only in the scratchpad.** These logs are evidence for
verified claims, not instruments, and their size kept me from copying them:
the events lane's `calib-batch1b.log`, `calib-batch1c.log`,
`calib-batch1d.log`, and `calib-batch2.log` (2.2 MB, 1.7 MB, 522 MB, and
13.5 MB), and the algebra lane's `run4.log` (32.6 MB). Section 03's
calibration claims and correction 21 in section 18 rest on
`calib-batch2.log`.

## 7. Corrections

[`18-corrections.md`](instrument-rescue/18-corrections.md) collects 42
corrections to the audit's records, its baseline, and the instruments
themselves. Three change a conclusion you might act on:

1. The codecs lane's specification codec does catch the two padding
   survivors, at low rates, because it pins today's detection order; survey
   section 2.1 says it cannot (decision 2).
2. A wasm32 differential over suanpan histories exists (09.A14); survey
   section 2.2 says none does.
3. `main`'s `min_ticks` code exceeds the board's heap ceiling at sizes the
   board does not sample (09.A4).

## 8. What this document could not settle

- **Rankings rest on the sections' assessments.** I read every section in
  full and checked a sample of their claims against the branches and
  `main`, but I did not re-run their measurements. Where two sections
  disagreed, section 17 or 18 records how I resolved it.
- **Runtimes of several folded forms are estimates.** The sections mark
  them inferred or unmeasured, among them 07.1 on #37's generator, 07.2's new
  ladder cells, 09.A2's laws, and every census floor.
- **No instrument was re-run against a mutant for this document.** Claims
  that an instrument would catch a mutant it was never run against are
  marked inferred in their entries (for example 04.2 against E8, and 06.1
  against the padding survivors).
- **The planned branches' own instruments** (slots 23, 33, 45, 46, and 47)
  are unreviewed, so section 09 lists them without ranking them, and so do
  I.
