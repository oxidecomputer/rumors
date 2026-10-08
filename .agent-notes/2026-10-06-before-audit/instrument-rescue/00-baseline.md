<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the rescue integrator, from the committed tree, the audit's records, and one census run on ox-east-1; for Finch's review. -->

# 00. The committed suite's reach

Every other section of the instrument rescue measures an explore-branch
instrument against what the committed suite already reaches. This file is
that baseline. It answers four questions about `main`:

1. What do the committed generators produce, by depth, size, relation mix,
   and equal-pair rate? (Section 2.)
2. Which oracles do the committed tests check against, and what is each one
   independent of? (Section 3.)
3. Which predicates do the committed laws and differentials state?
   (Section 4.)
4. Which input families do the cost instruments measure, and at what sizes?
   (Sections 5 and 6.)

Section 7 lists what the ready branches add, since a cataloguer must not
credit an explore instrument with reach a ready branch already carries.
Section 8 says what this baseline does not measure.

If you are cataloguing an instrument, start with section 2.7, the list of
regimes the committed generators never or almost never produce; most
coverage claims are claims about that list.

## 1. Base and provenance

**Base.** `main` at `ce67ab083`. Every number below describes that tree.
`main` has since moved to `4f7f45d43` by commits to `.agent-notes/` only;
outside that directory the two trees are identical (verified, `git diff
ce67ab083 4f7f45d43` excluding `.agent-notes` is empty).

- The audit's census ran at the audit base `58285ca5`. Between that base and
  `main`, the shared generators (`testing/generators.rs` and its tests), the
  trace driver (`testing/optrace.rs`), the oracles (`testing/oracles/`), the
  bridge, and the test RNG are byte-identical, and `Cargo.lock` is identical
  (verified: `git diff 58285ca5 main` over those paths and the lockfile is
  empty). The census samples under fixed seeds, and my rerun at `main`
  reproduced every one of the 30 recorded census blocks, and the family
  success census, byte for byte (verified; the run is described below).
- Four commits touch `crates/` since the audit base: `0b23cc36b` and
  `c9a0ea682` (suanpan's limb-stream index), `ddfabe4cb` (the board's
  `count_display` heap declaration), and `205199319`, which raises
  nextest's limit to 300 seconds in `.config/nextest.toml` and rewords one
  doc comment in `testing/exhaustive/tests.rs` (verified, `git log` and
  `git diff`).

**Marks.** Every claim is *verified* (I checked it against the tree, a log,
or my own run), *reported* (a record says so; the record is named), or
*inferred*.

**Where the census lives.**

- `lanes/l8-adequacy/round-1/census-baseline.txt`: the adequacy lane's
  census of the shared generators (reported as its run; I read the log).
- `<scratchpad>/auditor-l8/census-family2.log`: its census of the family
  generators' `join_all` success rate.
- The census source is `explore/l8-adequacy:crates/before/src/testing/l8_census.rs`
  (`0acb87bdc`), whose census functions take any iterator, so a cataloguer
  can census an explore generator with the same metrics (its module doc says
  how).
- **My run** (one on ox-east-1, at `ce67ab083`): that file copied into a
  scratch worktree, plus one added ignored test, `rescue_census_families`,
  run with the committed census test under `--no-capture`. Command:
  `cargo nextest run -p before --all-features --locked --run-ignored all -E
  "test(/l8_census|rescue_census|generator_classes_stay_under_mass/)"
  --no-capture`; 7 tests passed, exit 0. The log, the added test's source,
  and the registration diff are in `probes-preserved/ranker/` beside this
  directory, as `census-run1.log`, `rescue-l8_census.rs`, and
  `rescue-census-testing-rs.diff`.

## 2. What the committed generators produce

### 2.1 Arbitrary values

The shared generators live in `crates/before/src/testing/generators.rs`
(verified). Arbitrary trees recurse through `prop_recursive` with
`ARB_DEPTH = 4` and `ARB_NODES = 16`, and every node passes through the
recursive oracle's normalizing constructor, so every value is canonical.
`arb_magnitude` draws event bases from seven weighted arms: small values
(0 to 5) at weight 6, any `u64` at 2, and at weight 1 each the top five
`u64` values, `u128 + u64::MAX`, `2^k + 1` for `k < 96`, small multiples of
`2^64`, and odd multiples of `2^k` for `k < 512`.

Census, 20,000 draws each (verified from the census log):

| Metric | `arb_oracle_version` (seed 21) | `arb_oracle_party` (seed 11) |
|---|---|---|
| Depth | 0: 0.9%, 1: 5.1%, 2: 31.1%, 3: 53.6%, 4: 9.2%; never above 4 | 0: 11.8%, 1: 10.3%, 2: 37.4%, 3: 35.8%, 4: 4.7%; never above 4 |
| Size | nodes median 9, 99th percentile 15, max 21; leaves median 5, max 11 | leaves median 4, max 10; owned leaves median 2, max 6 |
| Encoded size | median 844 bits, 99th percentile 3,306, max 5,015 (about 627 bytes) | median 10 bits, max 28 (under 4 bytes) |
| Root | node 99.1%, nonzero leaf 0.9%, zero leaf 0.04% | node 88.2%, seed 5.8%, anonymous 6.1% |
| Magnitude | widest base: median 128 bits, max 514; 89.1% have a base wider than 64 bits | not applicable |
| Other | widest root-to-leaf height: median 128 bits, max 514; nonzero interior bases median 2, max 6 | `arb_oracle_party_nonempty` replaces the anonymous party with the seed: depth 4 in 4.8%, max encoded 26 bits |

The committed census test `generator_classes_stay_under_mass`
(`testing/generators/tests.rs`) holds liveness floors over 3,000 trees per
strategy at seed 7 (verified, source). Its floors: a base beyond `2^64`
650; three-limb bases 300; bases beyond every narrow arm 300; small
multiples of `2^64` 90; versions at full depth 70; full depth with a wide
base 70; parties at full depth 35. The measured counts at `main` (verified, my
run) are 2,674, 1,151, 1,150, 204, 255, 248, and 157 respectively, so the
floors sit at a quarter to a half of them, as its doc says. Versions reach
full depth in 8.5% of draws and parties in 5.2%.

Two filtered strategies, `arb_flush_version` and `arb_flush_party`, keep
only values whose live bits end on a byte boundary (verified, source); they
are not censused.

### 2.2 Pairs drawn independently

The law drivers (`testing/algebraic_laws/tests.rs`) and the differential
table's arbitrary drivers (`testing/diff_ops/tests.rs`) draw every operand
independently from the generators above (verified, source). The relation
mix of such pairs (verified, census log):

| Pair | Mix |
|---|---|
| Two versions (seed 31, 20,000 pairs) | concurrent 65.3%, greater 17.2%, less 17.6%, **equal 0** |
| Two nonempty parties (seed 32) | partial overlap 37.9%, first covers second 22.6%, the reverse 22.5%, disjoint 13.9%, equal 3.1% |
| A version projected onto a party (40,000) | unchanged 11.6%, empty 0.03%, other 88.4% |

So a law over two independent versions almost never sees an equal pair
from its inputs (none in 20,000); the law registry's doc says conditional laws construct their
antecedent's witness where they can (verified, `laws/mod.rs`).

### 2.3 Families: a receiver and a list of items

`arb_version_family`, `arb_party_family`, and `arb_clock_family` draw a
receiver and 0 to 17 items from a small pool: one to three arbitrary
versions plus the empty version; one to four arbitrary nonempty parties; or
every pairing of one to three parties with such a version pool (verified,
source). The list-group laws and the clock and party `join_all` laws read
them.

`join_all` succeeds only when the receiver and items are pairwise disjoint.
The adequacy lane's census (verified, `census-family2.log`):

- `arb_party_family`: 1,176 of 20,000 families pairwise disjoint (5.9%).
  Every one has at most one item: arity 0 always, arity 1 in 72 of 1,115,
  and arity 2 to 17 in **0 of 17,781**.
- `arb_clock_family`: 1,111 of 20,000 (5.6%).

So no arbitrary family drives a successful `join_all` of two or more
items. The laws
`party_join_all_reunites_forks_at_any_width` and its clock twin build
their own disjoint family by forking the receiver into as many shares as
there are items (verified, source), so successful multi-input joins are
checked, but only on balanced fork shares.

The pairwise relation mix inside each family, over every unordered pair
among the receiver and its items, 5,000 families each (verified, my run):

| Family (seed) | Pairs | Mix |
|---|---|---|
| `arb_version_family` (61) | 267,850 | equal 36.3%, greater 25.6%, less 25.5%, concurrent 12.6% |
| `arb_party_family` (62) | 265,619 | equal 53.7%, nested 21.8%, partial overlap 17.8%, disjoint 6.7% |
| `arb_clock_family` (63), parties | 270,567 | equal 61.9%, overlapping 32.6%, disjoint 5.5% |
| `arb_clock_family` (63), versions | 270,567 | equal 36.2%, greater 26.0%, less 25.5%, concurrent 12.3% |

Per family (verified, my run):

- Item counts are uniform over 0 to 17 (each about 5.5%; median 9).
- A version family holds **at most 4 distinct values** (1 in 9.1%, 2 in
  38.8%, 3 in 32.7%, 4 in 19.4%), because its pool holds at most three
  arbitrary versions and the empty one. 89.5% of version families hold an
  equal pair, 87.9% a strictly ordered pair, 88.8% the empty version, and
  in 83.5% the receiver reappears among the items.
- A party family holds at most 4 distinct parties (median 2); 91.1% hold an
  equal pair, and 280 of 5,000 (5.6%) are pairwise disjoint.

So the families are the committed suite's source of equal and ordered
pairs, which independent draws never give (section 2.2), but a
family-driven law folds at most four distinct values, however long the
list. The list-group laws' second driver indexes lists into an organic
population (verified, `algebraic_laws/tests.rs`), which holds a median of
2 and at most 11 clocks (section 2.4). Folds over many distinct operands
appear only in constructed families: the board's `stagger` rows,
`tests/fold_skeleton.rs`, and the asymptotics pins (verified, names).

### 2.4 Operation traces and organic populations

`optrace::world_strategy` draws fewer than `MAX_TRACE_OPS = 30` operations
over a population that starts as one seed clock: `tick`, `ticks(k)` for a
`u8` count, `fork`, `send`, `sync`, and `join` (verified, source). The
census, 5,000 traces at seed 41 (verified, census log):

- Length: median 14, 90th percentile 26, max 29.
- Final population: one clock in 30.9%, two in 26.9%, median 2, 90th
  percentile 5, max 11; nine or more clocks in 13 of 5,000 (0.26%).
- Operation mix: each of the six kinds about 16.7%.
- Degenerate operations: 25,109 of 72,429 (34.7%). A `join`, `send`, or
  `sync` whose two operands resolve to the same clock is skipped (or is a
  self-send): 64.6% of joins, 64.5% of sends, 64.8% of syncs. `ticks(0)`
  records nothing: 14.2% of `ticks`.
- Depth: deepest party median 1, max 7; deepest version median 1, max 6.
- Heights: the widest root-to-leaf height in any trace is at most 6 bits,
  so organic values never carry a height of 64 or more.

The census models the organic drivers' picks as `i % len` and `j % len`
for `i, j < 64` (reported, its doc): the same clock in 55.2% of picks, and the two versions are
equal in 67.2%, concurrent in 13.9%, ordered in 18.9% (verified, census
log).

The identity lane measured organic traces holding more than 8 live clocks in
0.6% (reported, `lanes/l1-identity/round-1/inventory.md`; it counts a
different event than the final population above).

### 2.5 Constructed deep and adversarial shapes

These are deterministic constructions, so they have no distribution; what
matters is the depth and size they reach (all verified from source unless
marked).

- **Deep shapes** (`generators::Shape`: left spine, right spine, zigzag,
  bushy). `deep_and_wide_ticks_match_iterated` draws them at depth 8 to
  128 with one leaf raised by an `arb_magnitude` value, and checks `ticks`
  against the iterated public `tick`, not against an oracle; other tick
  tests use fixed scales 1, 3, 17 and 5, 8, 32, 128
  (`version/tick/tests.rs`).
- **Stack-safety depth.** `deep_left_spine_party` and the clock tests reach
  depth 100,000 (`clock/tests.rs`, `const DEPTH: usize = 100_000`) against
  production alone, since the recursive oracle overflows at such depth by
  design (its "Operating envelope" doc).
- **The meter registry's 57 adversarial families**
  (`testing/meter/registry.rs`; names verified): `dense`, `bigroot`,
  `hugeleaf`, `cliff`, `id-pair`, `comb-scatter`, `harmonic`, `scatter`,
  `weave`, the three `stagger` variants, `nested-full`, `nested-wide`,
  `mirror-wide`, `mirror-narrow`, `staircase`, `reveal-comb`,
  `reveal-hifloor`, `pure-comb`, `ascend-cliff`, `ascend-plateau`,
  `dominated-undercut`, `jump-pair`, `freeze-pos`, `promo-rearm`,
  `weight-comb`, `freeze-parade`, `dense-suffix`, `wide-arming`,
  `plateau-puncture`, `lone-freeze`, `concurrent-pair`, `tooth-tail`,
  `benign`, `wide-tooth-comb`, `jump-comb`, `cliff-fan`, `cancelling-chain`,
  `alt-spine`, the six `memo-*` families, `descending-raises`, `mask-drift`,
  `meet-shade`, `arming-train`, the five `*-hole` families,
  `hoisted-window`, `propagate-seam`, and `latent-ladder`. The board, the
  lattice tests (`version/lattice/tests.rs` decodes registry shapes as
  operands), and the verdict matrix use them.
- **The verdict matrix's pool** (`tests/verdict_matrix.rs`): derived
  exhaustively from the registry and closed under join and meet, with a
  census that requires every verdict class (its module doc).
- **Exhaustive corpora** (`testing/exhaustive.rs`): every canonical party to
  depth 3 (256 parties) and every canonical version with bases in
  `{0, 1, 2}` to depth 2 (691 versions), with every operation on every value
  and every ordered pair. An ignored variant takes parties to depth 4
  (65,536) for the verdict legs and `tick`'s brute-force minimality check.

### 2.6 Per-module generators that no census covers

These strategies live beside the code they test. No committed census
measures them; where an auditor measured one, the number is reported here.

| Module | Strategies (verified, names) | Measured reach (reported) |
|---|---|---|
| Tick (`version/tick/tests.rs`) | `arb_latent_ladder`, `arb_ladder_dominant`, `arb_ladder_comparable`; independent party and version pools; the deep shapes above | Events lane: 81% of arbitrary pairs have no lookahead site, nesting at most 2, never two sites in one range (`lanes/l3-events/round-1/coverage-map.md`). |
| Queries and spans (`causally/tests.rs`, `span/tests.rs`, `version/place/`) | bound, down, and up clause strategies over the two-party grid; `arb_height` | Spans lane: walk operands at most 16 leaves and depth 4; exact coverage only on the 9-version two-party grid; placement `Before` 1.5%, `Concurrent(Start)` 2.4%, `Concurrent(End)` 2.2% (`lanes/l5-spans/round-1/coverage-record.md`). The committed random grid property reaches three concurrent hole bounds in 8 up-polar checks per 256 cases, all on two cells with heights at most 2 (verified by the spans cataloguer's census, `05-spans.md`). |
| Codecs (`bits/tests.rs`, `version/io/tests.rs`, `borsh_impls/tests.rs`) | `arb_gamma_stream`, `arb_build_op`, `arb_boundary_u64`, `arb_signed_magnitude`, `arb_nonzero_magnitude`, `arb_stream` | Codecs lane: generated values are canonical only, with canonicity violations from single-bit flips, planted pairs, and point tests; version depth at most 4 in the arbitrary generators (`lanes/l6-codecs/round-1/coverage.md`). |
| Range minima (`version/range_minima/tests.rs`) | inline property strategies | Each value is a single term `c << bits` (reported, ready entry #82, which changes it). |
| Fork counts (`party/forks/tests.rs`) | `arb_wide_count` | none recorded |
| suanpan (`accumulator/tests/surface.rs`, `differential.rs`) | `arb_step`, `arb_operation`, `arb_operand`, `arb_primitive`, `arb_shift`, `arb_limbs`; `arb_op`, `arb_probe_digit`, `arb_stability_value` | Suanpan lane: the surface suite compacts after every step (`lanes/l7-suanpan/round-1/coverage.md`); its generator reaches a digit of magnitude `2^33 − 1` in 202 of 1,000 programs, but such a digit meets `normalize` in only 2.2% of `normalize` steps and a shift in 1.7% of shifts (verified by the suanpan cataloguer's census, `07-suanpan.md`). Boundary weighting at 30-bit shifts, 32-bit digits, 64-bit limbs, and `2^96` (verified, `surface.rs` module doc). |

### 2.7 Regimes the committed generators never or almost never produce

Each line names its source.

- An arbitrary version or party deeper than 4 (never: `ARB_DEPTH` caps
  it), or a version with more than about 21 nodes (verified, census:
  maximum 21 nodes in 20,000).
- A party encoding longer than 28 bits (verified, census).
- Two independently drawn versions that are equal (verified, census: 0 of
  20,000).
- A successful `join_all` over two or more generated items (verified,
  census: 0 of 17,781).
- A generated fold over more than four distinct values from the families
  (verified, my run), or more than 11 from organic populations (verified,
  census).
- An organic population of nine or more clocks (verified, census: 0.26%),
  an organic party deeper than 7 or version deeper than 6, or any organic
  height of 64 bits or more (verified, census).
- Arbitrary pairs with several lookahead sites nested in one tick range
  (reported, events lane).
- Span placements `Before` and one-sided `Concurrent` above about 2.4%;
  exact coverage off the two-party grid; three or more concurrent holes,
  which the committed random grid property reaches 8 times per 256 cases
  and only on two cells (reported, spans lane; the three-hole count
  verified in `05-spans.md`).
- Non-canonical codec inputs beyond single defects (reported, codecs lane).
- Accumulator operands carrying their own histories (several recorded zero
  ranges, aliasing the receiver, uncompacted cancellation): never in the
  committed surface property; and extreme digits meeting `normalize` or a
  shift, which it reaches rarely (verified by the suanpan cataloguer's
  census, `07-suanpan.md`).
- Generated values deeper than 128 checked against any oracle. The
  deepest generated shapes stop at 128, and are checked against production
  itself; fixed cases go deeper against the recursive oracle (a dense spine
  at scale 512 in `lattice/tests.rs`'s `flat_over_deep_collapses_totally`);
  the 100,000-level tests check closed-form witnesses only (verified,
  source).

## 3. The committed oracles

| Oracle | Where | What it is independent of | What it shares or cannot reach |
|---|---|---|---|
| Recursive tree oracle | `testing::oracles::tree` | Production's code: separate types, written as the paper's recursion (verified, module doc). | It realizes each operation as the same tree recursion production descends from, so a defect in that recursion is common to both (stated by `oracles/function.rs`'s doc, verified). It overflows on deep inputs by design, so oracle-facing suites cap depth (verified, "Operating envelope"). |
| Function-space oracle | `testing::oracles::function` | The tree recursion: parties and versions are functions on `[0, 1)`, operations are closure combinators (verified, module doc). `fork` and `event` draw random valid policies, so it checks policy independence. | Trace replay is single-seed only, with its grid capped at `GRID_N = MAX_TRACE_OPS + 2 = 32` levels; the differential table's legs are not capped at `GRID_N`, but `fs_grid` asserts a grid under 64 levels, and every scan visits `2^g` cells (verified, `function.rs`). So in practice it judges only shallow inputs, whatever feeds it. |
| The bridge | `testing::bridge` | It builds production values from oracle trees without the public codec (verified, module doc). | Parties are written bit by bit through production's `BitsWriter`. Versions are written as a tree stream through `BitsWriter` (including `write_gamma`) and then converted to production's stored form by production's own `VersionWriter`, through `testing::version::from_tree_stream`. The inverse reads through production's internal `PartyReader` and `VersionTreeReader` (verified, `bridge.rs` and `testing/version.rs`). A defect in the writer or the readers is common to both sides of a bridged comparison; the codec suites check those paths against canonical bytes separately. |
| Brute-force grow | `testing::grow_brute_force` | `grow`'s dynamic program: it enumerates every feasible single-region inflation (verified, module doc). | Small trees only (exhaustive corpus and samples). |
| Interval walk for join and meet | `version/lattice/tests.rs` | The recursive model: it checks every output region is the pointwise maximum or minimum (verified, module doc). | It reads regions through production's `VersionRegionReader` (verified, imports). |
| Riemann-sum rank | `version/measure/tests.rs` | The tree oracle's rank (verified, module doc). | Small inputs, by its generators. |
| Brute force over the exhaustive corpus | `testing::exhaustive` | Sampling: total within its bounds (verified). | Checks against the recursive oracle, so it shares that oracle's recursion. |
| `num-bigint` integers | suanpan's `accumulator/tests/` | suanpan's balanced digit representation: a separate library (verified, imports in `differential.rs` and `surface.rs`). | Values only; representation invariants are separate checks. |
| Production against itself | laws (`testing::laws`), `tests/verdict_matrix.rs`, metamorphic identities | Any reference implementation: predicates from the ITC algebra (verified, `algebraic_laws.rs` doc). | A law catches only what its predicate states; `verdict_matrix` takes `Version::partial_cmp` as its reference relation (verified). |
| Canonical bytes and snapshots | codec suites, `tests/fuzz_seeds.rs`, `testing::snapshots` | Production's decoder for round trips; fixed bytes for snapshots. | Hand-written rejection corpora, not an independent specification codec. |

## 4. The committed predicates

### 4.1 The law registry

`testing::laws` registers 18 groups by signature (verified,
`for_each_law_group!` at `main`), 198 laws in all. The per-group counts
below come from matching each file's law declarations at `ce67ab083`
(verified); they are a snapshot, not a maintained figure.

| Group | Laws | Names (abridged where long) |
|---|---|---|
| `VERSION_SOLO` | 19 | idempotence, the bottom, `is_empty`, self-concurrency, self-distance and lag, `span` and `at` with self, `rank` and `min_ticks` zero iff empty, seed projection, `Ranked`, codec, `as_bytes`, `encoded_bits` |
| `VERSION_PAIR` | 33 | commutativity, bounds, absorption, method and operator spellings, antisymmetry, `eq` iff `Equal`, duality, concurrency, `rank` as a strictly monotone valuation, distance and lag identities, byte equality and hashing, prefix-free encoding, `Ranked` order, span gate, placement, hull, codec, atom membership, query shorthands |
| `VERSION_TRIPLE` | 30 | associativity, least and greatest bounds, both distributive laws, transitivity (constructed and incidental), triangle inequalities, lag monotonicity, query conjunction, coverage soundness and pointwise membership, span placement and its coarsenings, span algebra (union, intersect, join, meet, associativity) |
| `VERSION_LIST` | 2 | `Sum` is the sequential fold and order-invariant |
| `VERSION_AND_LIST` | 10 | `join_all`, `meet_all`, `span_all` against sequential folds and rotation; span folds; union, `Sum`, `collect`, product |
| `VERSION_PARTY` | 13 | `tick` strictly advances only within the region; `ticks` composition; `ticks` realizes `min_ticks` on a single-party line; projection is a sub-version, idempotent, additive over `fork`; operator spelling |
| `VERSION_PAIR_PARTY` | 9 | projection homomorphisms, short map, monotonicity; `ticks` composes; `OwnVersion` comparison and seed mask; projected span and its operator spelling |
| `VERSION_PARTY_PAIR` | 5 | projection commutes, monotone in region, disjoint projections share nothing, disjoint ticks differ, additivity over carved regions |
| `VERSION_PAIR_PARTY_PAIR` | 1 | `OwnVersion` pair comparison matches materialization |
| `PARTY_SOLO` | 17 | `fork`/`join` round trip, halves disjoint and covered, `forks` matches arrays, partial drop conserves, overlap hands back, `covers` reflexive and transitive, `without`, aliasing, `is_seed`, codec, bytes |
| `PARTY_PAIR` | 10 | `covers` antisymmetric, `is_disjoint` symmetric and excludes covering, `join` defined iff disjoint, commutative outcomes, `without` characterized, byte equality and hashing |
| `PARTY_TRIPLE` | 2 | incidental transitivity, `join` associative outcomes |
| `PARTY_AND_LIST` | 3 | `join_all` accepts iff pairwise disjoint, reunites forks at any width, conserves the region **union** on error |
| `RANK_TRIPLE` | 15 | addition commutative, associative, monotone, with zero as identity and bottom; subtraction inverts addition; `checked_sub` iff dominated; saturating subtraction; `cmp` antisymmetric; lexicographic order; codec round trip and prefix-free encoding; cross-path normalization |
| `CLOCK_SOLO` | 15 | `fork`, `forks`, `peek`, `tick`, own receive, `ticks`, `send`, `sync` of a fork, own version, parts and codec round trips |
| `CLOCK_PAIR` | 2 | byte equality and hashing |
| `CLOCK_VERSION` | 6 | `recv` learns and advances, fixes the party, is join then tick; `sync` is join then fork; anonymous join; `absorb` |
| `CLOCK_AND_LIST` | 6 | `join_all` accepts iff disjoint, reunites forks, conserves the union on error; `sync_all` is `join_all` then `forks`; `recv_all`; `absorb_all` |

Inputs: each group's proptest driver feeds independent arbitrary values
(section 2.2) or the families (section 2.3), and a second driver lands every
group on organic trace populations (section 2.4) (verified,
`algebraic_laws/tests.rs`). The fuzz workspace's law target is the third
consumer and is outside the audit's scope.

Predicates **no committed law states**, as the lanes found them: pointwise
multiplicity of owned regions (union only, until #40 and #81 land);
history independence and order independence of folds beyond rotation;
`min_ticks` as a floor over each clock's own causal past (the committed
proptest `min_ticks_floors_every_history`, `version/tests.rs:682`, states
the weaker floor of the whole history's total ticks, checked on final
clocks only; verified by the events cataloguer, `03-events.md`);
`Hash` agreement with the byte view, and `Debug` agreement with `Display`
(until #83 lands) (reported, survey sections 2.1 and 2.2, and the ready
entries).

### 4.2 The differential table and the trace differential

`testing::diff_ops` checks deterministic operations three ways: production
against the recursive oracle, and the recursive oracle against the function
space (verified, module doc). Nine groups hold these descriptors (verified,
source): projection; projected comparison against a version and against a
projected version; the clock's own version and shape; `is_seed` and party
shape; `covers`, `is_disjoint`, `without`; disjoint `join`; `min_ticks`,
version shape, `rank`; join, meet, `combine`, order, concurrency,
`distance`, `lag`. Two drivers apply every descriptor: independent
arbitrary operands, and pairs drawn from organic populations. Known-bad
descriptors check that the comparison rejects a mistranscription (verified,
`descriptor_checks_reject_a_mistranscribed_operation`).

`testing::optrace` replays one trace through production and both oracles,
then compares the causal order and disjointness over every pair of the
final population (verified, `function.rs` doc). Stateful operations
(`fork`, `tick`) are checked only there and in the per-module suites.

### 4.3 Other committed predicate suites

- Per-module differentials against the recursive oracle in each type's
  `tests.rs` (verified, module docs listed in section 2.6's sources).
- `tests/verdict_matrix.rs`: projected versions, spans, ranked versions,
  and queries agree with `Version::partial_cmp` over the registry pool.
- `tests/stale_state.rs`, `tests/forks_count.rs`, `tests/coincident_span.rs`,
  `tests/answer_embedded.rs`, `tests/fold_skeleton.rs`,
  `tests/representation_space.rs`, `tests/foreign_reexport.rs`,
  `tests/fuzz_seeds.rs` (verified, file list and module docs).
- suanpan: `differential.rs` (sign after every operation, value at
  snapshots, deterministic hard streams), `surface.rs` (every public query
  after every instruction against `BigInt`), `representation.rs`,
  `primitives.rs`, `witnesses.rs`, and the exact touch pins in
  `metered.rs` (verified, file list; module docs for the first two).

## 5. The cost instruments

### 5.1 The amplification board

`testing::meter::board` (verified, source unless marked):

- **Axes.** 57 registry families (section 2.5) against 135 operation rows
  (`board/ops.rs`). The acceptance run judges 5,311 cells (reported,
  `baseline.md`); the rendered board has 15,933 lines (reported, #61 and
  #76).
- **Currencies.** Peak heap, scan bits (feature `scan-meter`), and suanpan
  digit touches (feature `touch-meter`). Big-integer work outside suanpan's
  accumulator is invisible to all three (reported, survey 1.1).
- **Ladder.** Two sizes at scale 1 and two at scale 4: four points over an
  eightfold range feed one exponent fit; extra samples at scale 0.01 check
  ceilings and floors only. Families are sized at roughly 1 to 35 KiB at
  scale 1 (verified, the size comments in `board/family.rs`).
- **Ceilings.** Exponent at most 1.15; heap at most `1,024 + 20·D` bytes;
  scan at most 96 bits per input byte; touches at most 22 per input byte;
  operation-specific ceilings for folds, projection, deserialization, and
  query evaluation (`board/ceilings.rs`). Each cell also carries a liveness
  floor.
- **The worst-case ranking pin.** 270 pinned rows at the two sampling
  scales (reported, `baseline.md`).

### 5.2 Focused resource checks and liveness pins

- `tests/meter.rs` and its submodules vary axes the board holds fixed:
  placement, spans, span codecs, tick counts, version scaling (cliffs and
  freezes, freeze schedules, minimum boundaries and combs, pair crossings,
  skip adequacy), width circulation, hoisted windows, identity fast paths,
  equality early exits, settle flatness, answer-embedded products, deferred
  wide arming (verified, file list).
- `testing::asymptotics`: five fold log-factor liveness pins (`join_all`,
  `meet_all`, `span_all` for versions; `join_all` for parties and clocks)
  and three `mul_bound_*_embedding_is_alive` pins (verified, test names). The adequacy lane
  found the log-factor pins check only a floor: a quadratic fold mutant
  passes all five (reported, `findings/instrument-calibrations.md`).
- `tests/amp_board_smoke.rs` drives every board cell and exercises shard
  merging (verified, module doc).
- suanpan: exact touch pins in `accumulator/tests/metered.rs` and one
  amortized sign-flip sequence in `tests/amortized_sequences.rs` (verified,
  file list).

### 5.3 Wasm fuel

- **The fuzz-fit bands** (`crates/before/fuzzfit/`): 50 bands over 40 guest
  kernels, keyed by kernel and by success or rejection (verified,
  `harness/src/bands.rs`). Programs come from 18 families
  (`harness/src/strategies.rs`), dense spine through escalation, under
  budgets of at most 6,000 operations, 3,000 ticks, 1,536 forks, and folds
  of 1,024; the escalation family allows 9,000 operations and 2,048 forks
  (verified). The largest calibrated denominator is 187,950 bits,
  about 23 KB (verified). A sample passes within `ENFORCE_MARGIN = 0.2`
  above its band in `log₁₀` fuel (about 1.58 times) and
  `ENFORCE_MARGIN_BELOW = 0.8` below (verified). By its own doc, it never
  builds wide magnitudes (a `2^b` leaf costs `2^b` ticks), never measures
  codec rejection, and never folds an empty list.
- **The fuelscape atlas** (`before-fuelscape`) maps fuel over uniformly
  sampled inputs and enforces nothing (verified, validation index); it is
  outside the audit's scope.
- **The zero-range fuel ladder is not on `main`.** It is ready entry #52
  (72 cells, ±2% band).

## 6. Target-dependence instruments

- **The wasm32 pins** (`crates/before/wasm32-pins/`): eight tests at
  `main`, namely `harness_outcomes_are_live` and pins on forks past
  `usize`, version decode, version compare, and join output across the
  `usize` position boundary, rank decode past `usize`, rank arithmetic
  across the alignment limit, and suanpan's unaddressable digit landings
  (verified, `harness/tests/pins.rs`). The adequacy lane's calibration
  caught 7 of 11 injected narrowings; the compare pin cannot reach a cursor
  position past `2^32` (reported, `findings/wasm32-pins-calibration.md`).
  At `main` a panic and an allocation abort look alike (#31 changes that),
  and the workspace has no nextest time limit (#57 adds one) (reported,
  ready entries).
- **The surface check** (`crates/before/surfacecheck/`): every public
  function-like item has exactly one disposition, board-covered or
  excepted, from rustdoc JSON; 211 items, 134 covered and 77 excepted
  (reported, `baseline.md`). It does not pin which public signatures
  mention `usize`; the survey lists that pin as a candidate (reported,
  survey section 2.2).
- **The board's target declaration**: `count_display × heap` is declared
  target-dependent (verified, `ddfabe4cb`'s subject).
- No committed instrument measures cost on a 32-bit target (reported,
  survey).

## 7. What the ready branches add

None of these is on `main`. A cataloguer must not credit an explore
instrument with reach that one of these branches already carries, but
should say when an explore instrument would stand beside one (all reported,
from `QUESTIONS.md`'s ready entries).

| Entry | Adds |
|---|---|
| #74 | Co-generated tick pairs (spine, wide, multi-scan strategies), two witnesses, census floors on their regimes |
| #82 | `range_minima` values as open minima plus small offsets, and two exact cases |
| #38 | The nested range-minima property draws a small value pool, reaching equal minima |
| #37 | A suanpan touch bound over swarm-weighted programs on three accumulators, with gap rounds and cancelled chains |
| #48 | A suanpan accumulation property over shifted zeros |
| #66 | An exhaustive suanpan `normalize` test over boundary digits at widths 1 to 4 |
| #64 | A table over every reachable readout carry class, for value and touches |
| #78 | A `Rank::decode` property through a scripted reader that interrupts and fails |
| #40, #81 | Multiplicity on `join_all` failure, in model tests and in the law registry |
| #83 | Laws for `Hash` against the byte view, `Debug` against `Display`, and `ClockForks` hints |
| #76 | The law `sync_all_agrees_with_join_all`, and a late-overlap fixed shape |
| #34, #75 | Stack safety at depth `2^18` for every walk-bearing entry point |
| #61, #94 | The two-party coverage grid with holes beside floors and ceilings; a board row for a bounded query without holes |
| #52, #86 | The zero-range fuel ladder (72 cells), and a `BTreeSet` model of suanpan's written-position set |
| #51 | Serde leniency properties across key orders and sequence forms |
| #43 | Fork-plan hints driven through real steps past `usize::MAX` |
| #36, #89 | A lattice clone heap check; an exact touch pin and a liveness floor in `amortized_sequences.rs` |
| #28, #29, #31, #48, #50, #54 | wasm32 pins: unsatisfiable reservations, rank `Sum` across the alignment limit, panic message recording, zero shifts, stability width, direct deposits at digit `2^32` |
| #57, #93, #95 | Detached-workspace time limits, the ranking pin run whatever acceptance returns, exact comparison of two board runs |

## 8. What this baseline does not measure

- No census covers suanpan's generators, the tick and span per-module
  strategies, or the codec strategies; section 2.6 gives the lanes'
  measurements instead.
- The census measures shape and relation mix, not semantic events such as
  which kernel branches a generator reaches. Branch coverage at the audit
  base was 97.8% of lines and 95.4% of branch outcomes (reported, the
  adequacy lane's round-1 notes).
- Runtime: nextest terminates a test at 300 seconds on `main` (verified,
  `.config/nextest.toml`: `slow-timeout = { period = "60s",
  terminate-after = 5 }`); the `high-count` profile allows 600. A fold-in's
  runtime should be judged against the default profile.
