<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the instrument-rescue cataloguer for lane L5 (spans and causal queries), from the lane's branch and records and one run on ox-east-1; for Finch's review. -->

# Instrument rescue, lane L5: spans and causal queries

## Lane summary

**Source.** `explore/l5-spans` at `2874e0c3d`, four commits on `58285ca51`
(verified, `git log`). The branch adds one file,
`crates/before/src/testing/audit_l5.rs` (2,172 lines), and its
`#[cfg(test)] mod audit_l5;` line in `src/testing.rs` (verified, `git diff
--stat`). The lane's production code and test support are unchanged on
`main` since the base; only board files under `src/testing/meter/` moved
(verified, `git diff --stat 58285ca51 main` over `span/`, `causally/`,
`version/place*`, and `testing/`). The file should therefore compile on
`main` (inferred; I built it only at `2874e0c3d`).

**What the lane built.** One model and everything around it. The model
represents a version as a vector of heights over a leaf partition that every
version in a test shares; the causal order is the pointwise order, and join,
meet, and projection are pointwise maximum, minimum, and masking. It shares
no code with production comparison, lattice, walk, or query code (verified
by reading: every verdict comes from `BigUint` vectors). Around it the lane
built an exact coverage oracle (`census`) and a validator for it; generators
of uniform grids, deep shapes, organic shapes, and pairwise-concurrent
holes; sixteen properties; an exhaustive check over a 16-version cube; two
cost probes; and diagnostics. Its calibration mutants live only in the
records.

**What the committed suite already does here** (verified by reading `main`):

- Every committed span and query oracle takes its relation from production
  `partial_cmp` or `<=`: `version/place/tests.rs`, `causally/tests.rs`,
  `tests/verdict_matrix.rs`, and the `version_triple` laws. Production
  `partial_cmp` (`version/order.rs`) shares the overlay advance,
  `OrderState`, the region readers, and suanpan's `Accumulator` with the
  span and filter walks it judges. `partial_cmp` is itself checked against
  the tree oracle elsewhere, on generated versions of depth at most 4.
- Exact coverage of queries *with holes* is checked only on the two-party
  grid: 9 versions over 2 cells, heights 0 to 2
  (`coverage_is_exact_on_the_two_party_grid`, and the small-grid half of
  `public_queries_match_their_relational_denotation`). Ready branch #61
  extends that grid's query family. On arbitrary versions, coverage is
  checked for soundness only (`coverage_bounds_membership`) or against the
  per-bound fold (`filter_coverage_matches_the_composed_sweeps`).
  `verdict_matrix` does check exact coverage of *neutral* floor-and-ceiling
  queries, crossed clamps included, on the deep registry families.
- The committed version generator, `arb_oracle_version`, stops at depth 4
  and 11 leaves, and two independent draws are never equal (reported, the
  adequacy lane's census, `lanes/l8-adequacy/round-1/census-baseline.txt`:
  20,000 pairs, 65% concurrent, 0% equal).

**How I assessed it.** I read every line of the harness, every lane record,
and the auditor's logs. Those logs are kept in the tree at
`lanes/l5-spans/scratch-tools/`, and a bare log name below (`mut-run.log`,
`stats1.log`, `reach.log`, and the like) refers to that directory. The #61
builder's logs are cited at `<scratchpad>/builder-refine-partial/`. I used one
box run (*run 1*) in the scratch worktree `/Users/oxide/src/rumors-rescue-l5`
at `2874e0c3d`, with one uncommitted census probe of my own added; the probe
is not a lane instrument and is not catalogued, but its patch is kept at
`<scratchpad>/rescue-l5/rescue-census-probe.diff`. Run 1 measured every L5
property's runtime under nextest at its default case count, the ignored
probes' runtimes, and the census described in the next section. Its log is
`<scratchpad>/rescue-l5/run1.log`. The box carried other agents' builds
throughout: load average 26 when the run started and 46 to 51 while the
tests ran, on 192 hardware threads. Every runtime below is a reading under
that load, not a benchmark. Every deterministic reading the records hold
reproduced exactly in run 1: the cube's tally, the carry family's touches,
and the refinement probe's scan bits.

**Headline findings.**

1. The lane's distinct contribution is *exact coverage of queries with
   holes beyond the two-party grid*, against an oracle independent of
   production. Per 256 cases, the shaped antichain property alone performs
   20,053 coverage checks that only the refinement step decides: 15,255 of
   them carry two or more pairwise-concurrent hole bounds, 19,305 sit at shape
   depth 5 or more, and 10,012 involve a height of `2^64` or more. The
   committed exact checks perform 337 checks of the first kind, all on two
   cells with heights at most 2, and none of the other two (verified, run 1;
   see the census below).
2. The model's independence is real but defends a narrow class: a defect
   that moves production comparison and the span walks together. No such
   defect was constructed.
3. No instrument here caught anything the committed suite missed. Of the
   auditor's 19 injected mutants, the committed suite caught all 17
   non-equivalent ones, each through at least two committed tests, and
   also the equivalent M18, which the harness missed (verified,
   `mut-run.log`). That calibration ran before the shaped,
   organic, cube, and carry instruments existed, so those carry no mutant
   evidence of their own. The adequacy campaign left no value-changing
   survivor in the lane's files for any of them to kill (reported, its
   survivor index).
4. Four instruments carry defects of their own, each noted in its entry:
   the antichain properties return without checking anything in 2% to 8%
   of cases; a doc comment claims a set-intersection check the code does
   not make; a doc comment names a `witness` oracle the test never calls;
   and that oracle is dead code.

## Ranking

The *needs* column names the other entries a fold-in would bring along.

| Rank | Instrument | Needs | Value in one line |
|---:|---|---|---|
| 1 | Pairwise-concurrent holes (`with_bumps`, antichain properties) | 2, 7, 8, 9, 11 | The only source of three or more concurrent holes under an exact oracle, at depth and width. |
| 2 | Exact coverage oracle (`census`) | 7 | The enabling piece: no exact-coverage check can leave a complete grid without it. |
| 3 | Census validator | 2 | Turns the exactness argument every exact-coverage test relies on into a sub-second check. |
| 4 | Exhaustive boolean cube | none (brute force suffices) | The cheapest rescue: total, deterministic exact coverage on a lattice twice as wide as the committed grid, in about 7 s. |
| 5 | Query membership and exact coverage across worlds | 2, 7, 8, 9, 11 | Exact coverage of polar queries at depth and width, where the committed suite checks soundness only. |
| 6 | Span-to-query coverage agreement | none | The cheapest new predicate: coverage and the containment algebra must agree. |
| 7 | Vector model and verdict oracles | none | The only span and query oracle that shares no machinery with the code it judges. |
| 8 | Deep and random shapes | 7 | Depth ten times the committed generator's, for operations whose cursors depth stresses. |
| 9 | Grid worlds and near-equal wide heights | 7 | Makes the rare placements and near-equal wide comparisons four to six times more common. |
| 10 | Span verdicts against vectors | 7, 8, 9 | A second, independent oracle for every span verdict at depth. |
| 11 | Clause model and conjunction folds | none | Worth folding as a delta to the committed clause enums. |
| 12 | Projected-span views against masked vectors | 7 | A modest second oracle for `OwnSpan`, weakened by a skewed verdict mix. |
| 13 | Carry-boundary touch family | none | A plausible board family that has never failed and checks nothing today. |
| 14 | Span algebra against vectors | 7 | Little beyond the committed laws. |
| 15 | Organic overlays | 2, 7 | Low: organic shapes are shallow and covered by the other generators. |
| 16 | Committed-population reach probe | none | Useful only for placement-mix floors on committed generators. |
| 17 | Calibration mutants M1 to M20 | none | A ready-made known-bad list for whatever is folded in. |
| 18 | Refinement scan-cost probe | none | Its job is done; #61 and #94 carry its purpose. |
| 19 | Behavioral histograms | none | Scaffolding; keep the numbers, not the code. |
| 20 | Tie-order experiment | none | Settled; nothing to rescue. |
| 21 | Witness heuristic (`witness_coverage`) | none | Dead code. |

Carried by ready branches, in one line each:

- The finding behind entry 18 is carried by #61 (`simplify/span-refine-partial`,
  the change itself and its grid extension) and #94
  (`proposal/board-neutral-coverage`, a board row for the case). The probe
  is not on any branch, so it keeps a full entry.
- The lane's three documentation defects and its tie-order prose are
  carried by #85 (`docs/audit-corrections`); they are corrections, not
  instruments.

## The refinement census (run 1)

The lane's claim beyond the committed suite rests on exact coverage. Exact
coverage is hard only where reasoning one bound at a time fails, so the
census counts those checks.

**The measure.** Call a coverage check *refinement-decided* when its exact
verdict is `Empty`, yet reading each normalized bound alone at the
segment's endpoints (floors joined into one floor, ceilings met into one
ceiling, each hole by itself) leaves the segment partly admitted. These are
exactly the checks that a missing or weakened refinement step gets wrong.
In production they are decided only by `Query::refine_partial` (inferred:
the fused walk equals the per-bound reading by the committed
`filter_coverage_matches_the_composed_sweeps`, and `refine_partial` runs on
the walk's `Partial`).

**The method.** The uncommitted probe replays each population's own
generators at 256 cases and classifies every coverage check with `census`
and with the per-bound reading, both computed on vectors; it touches no
production code. The committed exact-coverage tests are transcribed into the
same model: the two-party grid is two cells, one per party, with heights 0
to 2 (verified by reading `two_party_grid`). Three transcriptions:
`main`'s grid family, the same family with #61's added queries, and the
small-grid half of `public_queries_match_their_relational_denotation` with
its own clause ranges. As a check on the probe itself, across all 857,820
classified checks the per-bound reading never claimed `Empty` or `Full`
where `census` disagreed. As a check on the measure, it reproduces a gap
found independently: `main`'s grid family has no up-polar
refinement-decided check, the gap #61's commit `eae62d545` states and
closes ("its up-polar family had no query holding a floor"), and #61's
family has 232.

**The counts** (verified, run 1; `<scratchpad>/rescue-l5/census.txt`).
Hole counts are the number of distinct extremal hole bounds among the
query's clauses: pairwise-concurrent holes after absorption, counted before
production prunes holes that the floor or ceiling makes vacuous, so the
stored count can be lower. "Deep" is shape depth 5 or
more; "wide" is any endpoint or bound height of `2^64` or more. Columns
with three values read neutral / down / up; with two, down / up.

| Population | Cases | Checks | Refinement-decided | ≥ 2 holes | ≥ 3 holes | Deep | Wide |
|---|---:|---|---|---|---|---:|---:|
| Committed grid, `main` (one pass) | 1 | 576 / 972 / 972 | 36 / 64 / 0 | 0 / 0 | 0 / 0 | 0 | 0 |
| Committed grid with #61's family (one pass) | 1 | 576 / 2,268 / 2,268 | 36 / 240 / 232 | 0 / 0 | 0 / 0 | 0 | 0 |
| Committed random grid property | 256 | 9,216 each | 517 / 507 / 524 | 164 / 173 | 0 / 8 | 0 | 0 |
| L5 cube (each polar query once) | 1 | 42,768 / 219,672 / 219,672 | 1,695 / 5,085 / 5,085 | 0 / 0 | 0 / 0 | 0 | 0 |
| L5 uniform queries | 256 | 9,736 each | 275 / 418 / 370 | 133 / 140 | 25 / 20 | 0 | 502 |
| L5 uniform antichains | 250 | 12,801 / 12,801 | 797 / 651 | 479 / 367 | 139 / 65 | 0 | 868 |
| L5 shaped queries | 256 | 16,766 each | 477 / 744 / 745 | 306 / 154 | 86 / 22 | 1,136 | 960 |
| L5 shaped antichains | 256 | 89,825 / 89,825 | 11,223 / 8,830 | 9,950 / 5,305 | 4,656 / 4,334 | 19,305 | 10,012 |
| L5 organic queries | 256 | 8,452 each | 368 / 638 / 445 | 203 / 126 | 0 / 29 | 273 | 0 |
| L5 organic antichains | 235 | 15,157 / 15,157 | 994 / 707 | 640 / 429 | 144 / 130 | 205 | 0 |

Read the table this way:

- The committed exact checks do reach two- and three-hole refinement
  cases, but only through the random grid property, and only 8 per 256
  cases with three holes, all up-polar, all on two cells.
- The cube has about 23 times as many refinement-decided checks as #61's
  grid (11,865 against 508), on a wider lattice, but every one of them holds
  a single hole: a two-clause query cannot carry a ceiling and two holes at
  once.
- Depth and width appear only in the L5 populations, and the shaped
  antichain property dominates every column. The antichain populations'
  case counts are below 256 because some cases place no hole and return
  without checking (6 and 21 of 256; see entry 1).
- At the properties' default case counts (64, or 32 for organic), each row
  shrinks to about a quarter or an eighth (inferred: the counts are
  per-case sums).
- The baseline (`00-baseline.md`, sections 2.6 and 2.7) relays the lane
  record's claim that committed queries hold at most two holes. By this
  census the committed random grid property does reach three concurrent
  hole bounds, in 8 up-polar refinement-decided checks per 256 cases
  (verified), so "almost never" is accurate and "at most two" is not. The
  two-party grid's widest antichain has three versions, so three is its
  ceiling (inferred).

## Entries

### 1. Pairwise-concurrent holes (`with_bumps`, `l5_antichain_queries`, `l5_shaped_antichains`, `l5_organic_antichains`)

- **What it is.** A generator and three properties. `with_bumps` and the
  body `body_antichain_queries` are at lines 1536 to 1677 at `5cec22e4e`
  (about 140 lines); the shaped and organic drivers are at `19080507f`.
  `with_bumps` adds, for every cell, the first pool vector bumped one
  alphabet step up and one step down; bumps in different cells are pairwise
  concurrent. It also adds the bottom vector, the join of the up-bumps, and
  the meet of the down-bumps, so segments straddle the antichain. The body
  places a hole, strict or inclusive, at each of 2 to 8 picked bumps (up and
  down bumps mixed), adds 0 to 2 atoms, and checks membership of every pool
  version and coverage of every ordered segment against `census` (entry 2).
- **What it reaches or checks.** Per the census above: up to 11,223 down and
  8,830 up refinement-decided checks per 256 cases (shaped), 9,950 and 5,305
  of them with two or more concurrent holes, 1,663 and 829 with five or more,
  most of them at depth 5 or more and many with heights of `2^64` or more.
  The uniform and organic forms reach the same classes at smaller scale. The
  auditor's earlier count agrees in kind: two to five stored holes in 81 of
  251 down cases and 89 of 251 up cases, uniform form (reported,
  `long1.log`). The oracle is `census` over the vector model,
  independent of production.
- **Coverage beyond the committed suite.** Exact coverage with three or more
  concurrent holes in quantity, and any number of holes at depth or width.
  The committed exact checks reach three-hole refinement cases 8 times per
  256 cases and never at depth or width (verified, census). The fused walk's
  per-hole state, conjunction's hole pruning and absorption, and
  `refine_partial`'s hole loop see wide antichains only here. A defect that
  keeps or tests only the first two holes, for example, would show here
  first (inferred; not constructed).
- **Evidence.** The uniform form failed under ten of the auditor's mutants
  (M1 to M5, M8 to M10, M15, M17), all also caught by three to nine
  committed tests each (verified, `mut-run.log`). The shaped and organic
  forms were never run against a mutant. All three pass at the case counts
  the auditor ran (reported: 4,000 uniform, 3,000 shaped and organic) and in
  run 1. A vacuity: when every pick lands on an empty bump list, the body
  returns `Ok` without checking anything (verified, `audit_l5.rs:1608-1610`);
  that happened in 6 of 256 uniform cases and 21 of 256 organic cases
  (verified, run 1's `empty-case` counts). A fold-in should construct at
  least one hole directly instead.
- **Fold-in cost.** Moderate, because it needs entries 2, 7, 8, 9, and 11.
  `with_bumps` itself is a pool extension of about 40 lines that could feed
  the committed down and up clause generators in `causally/tests.rs`.
  Runtime in run 1: uniform 0.43 s, shaped 26.08 s, organic 0.15 s at the
  default counts. The shaped form is the slowest property in the lane but
  sits well inside nextest's per-test limit (300 s on `main` at
  `d095feb4f`). No new dependency. Each property needs an accurate doc
  comment (the shaped and organic ones are one line today) and a
  validation-index entry.
- **Overlaps.** Entry 5 shares the oracle and body structure. Committed:
  `conjunction_normalizes` pins one two-hole antichain by `Debug` count;
  the two-party grid tests cover single holes exactly. It would stand
  beside them as a second check, not replace them.
- **Dependencies.** None on unlanded branches. #61 changes the refinement
  this exercises; the census shows #61's grid covers its single-hole cases.
- **Value, in one sentence.** The largest coverage gain in the lane: the
  only source of three or more concurrent holes under an exact oracle, at
  depth and width.

### 2. The exact coverage oracle (`census` and `sublattice`)

- **What it is.** A model oracle; lines 715 to 781 at `bdaec1dcc`; 67
  lines. `sublattice` closes a set of vectors under pointwise join and
  meet, giving up at a cap. `census` closes the segment's two endpoints
  together with every query bound clamped into the segment, `(lo ∨ b) ∧
  hi`, and counts how many members the query's predicate admits: all of
  them is `Full`, none is `Empty`, otherwise `Partial`.
- **What it reaches or checks.** Exact coverage on any finite vector world:
  uniform grids to 64 cells, spines to depth 40, organic overlays, the cube.
  Its doc comment gives the exactness argument, and I re-derived it. `Full`
  is decided by the endpoints, because every polar admitted set is the
  intersection of an up-set and a down-set. When the admitted set is
  nonempty, the meet of the clamped ceilings is `lo ∨ (hi ∧ ceiling)` by
  distributivity, which is the clamped extreme `hi ∧ ceiling` whenever the
  ceiling admits `lo` at all; so the closure contains a witness, and dually
  for up-polar queries (inferred, my derivation). The cap is 20,000 members
  in the properties and `2^20` in the cube. No census in the auditor's
  statistics hit it (verified: no `census None` or `census-skip` line in
  `stats1.log`, `long1.log`, or `shape1.log`), and none of run 1's 857,820
  censuses did (verified).
- **Coverage beyond the committed suite.** The committed exact checks
  enumerate a complete grid and rely on the grid being closed under join and
  meet. `census` needs no complete grid, so it is what makes exact coverage
  checkable at depth, at wide heights, and on organic shapes. No committed
  oracle computes coverage exactly on arbitrary versions.
- **Evidence.** It is the oracle behind every coverage kill in the lane's
  calibration (M1 to M5, M8 to M10, M15, M17; verified, `mut-run.log`); the
  committed suite caught all ten too. It never disagreed with the per-bound
  reading where that reading decides (verified, run 1).
- **Fold-in cost.** Moderate. It would join `testing::oracles`, ideally on
  top of the function-space oracle: sampling an `oracles::function` `Event`
  at every cell of a shared grid yields exactly this vector, so entry 7's
  model could become a view of the committed oracle rather than a parallel
  one (inferred). Its doc comment would need the exactness argument as a
  proof, and the cap as a precondition with a failure mode: today a capped
  census silently skips the check, so a fold-in should count skips and fail
  on any. Runtime is part of each property's. No new dependency
  (`num-bigint` is already one).
- **Overlaps.** None committed. Entry 3 validates it; entry 21 is a dead
  heuristic it supersedes.
- **Dependencies.** None.
- **Value, in one sentence.** The enabling piece: without it, no
  exact-coverage check can leave a complete grid.

### 3. The census validator (`l5_census_is_exact_against_a_finer_universe`)

- **What it is.** A check of an oracle, not of production; lines 1472 to
  1534 at `bdaec1dcc`; 63 lines; not ignored. For random two-cell pools with
  heights 0 to 2 and random down and up conjunctions, it lifts each vector to
  four cells by duplicating every cell, enumerates all 81 four-cell
  functions with heights 0 to 2, and requires `census` to equal brute force
  over every function inside each segment, at 200 cases.
- **What it reaches or checks.** Segments containing versions off the coarse
  grid, which is where a coarse brute force could be wrong. It passes
  (verified, run 1: 0.15 s).
- **Coverage beyond the committed suite.** The committed grid test's brute
  force counts only the nine grid versions inside a segment. #61's reviewer
  constructed an off-grid version inside the grid's interval (reported,
  QUESTIONS #61), so that test's exactness rests on the same witness
  argument, which #85 now states in prose only. This validator is the one
  executable check of that argument, and it could validate the committed
  grid's `brute` function directly. Its doc comment overclaims: it says "the
  census and witness oracles agree", but it never calls `witness_coverage`
  (verified, by searching the file; see entry 21).
- **Evidence.** Never calibrated: no record shows it run against a
  deliberately weakened census, for example one that omits the clamped
  bounds from its generators (verified, the lane's records). It caught
  nothing.
- **Fold-in cost.** Small, paired with entry 2 or entry 4: about 60 lines, a
  corrected doc comment, and an index entry. Runtime 0.15 s (run 1).
- **Overlaps.** None.
- **Dependencies.** None.
- **Value, in one sentence.** It turns the exactness argument that every
  exact-coverage test relies on, committed ones included, from prose into a
  passing check, at a fraction of a second.

### 4. The exhaustive boolean cube (`l5_exhaustive_boolean_cube`)

- **What it is.** An exhaustive enumeration, `#[ignore]`d; lines 1921 to
  2026 at `19080507f`; 106 lines. It takes the 16 versions whose four cells
  each hold height 0 or 1 (as if four parties), every ordered segment of
  them (81), and every single clause and every same-polarity clause pair of
  the inclusive, strict, negated, and widened forms at every version, and
  checks membership and exact coverage against `census`. Each polar query is
  built twice, folded from each side. It asserts; it does not print a
  verdict for a reader to judge.
- **What it reaches or checks.** 921,456 coverage verdicts per run, all
  agreeing, tallied down `Empty` 263,546, `Full` 65,104, `Partial` 110,694,
  the same for up, and neutral 33,181, 2,177, 7,410 (verified, run 1,
  identical to the auditor's `shape1.log`). Counting each polar query once,
  1,695 neutral and 5,085 down and 5,085 up checks are refinement-decided,
  every one with a single hole; #61's grid has 36, 240, and 232 (verified,
  run 1). Its widest antichain has six versions, against three for the
  two-party grid (inferred: the middle rank of the Boolean lattice on four
  cells has six elements, and a product of two three-element chains has
  width three). It has no `delta` or `toward` form and no conjunction of
  more than two clauses.
- **Coverage beyond the committed suite.** Totally enumerated exact coverage
  on a four-cell lattice, where the committed exact checks cover two cells.
  A defect that appears only when a bound and a hole are concurrent in
  several independent directions would need the extra cells to show
  (inferred; not constructed). It adds no multi-hole refinement cases;
  entry 1 does.
- **Evidence.** Caught nothing. It was written after the mutant calibration
  and never run against a mutant (verified: `mut-run.log` predates
  `19080507f`). It passed under the equivalent tie-order mutant with the
  same tally (reported, `tie-order.md`). The #61 builder ran it, grafted,
  on #61's parent and tip, and it passed both (verified,
  `builder-refine-partial/parent-run.log` and `tip-run.log`).
- **Fold-in cost.** Small. The cube is closed under join and meet, so brute
  force over its 16 members gives the same verdicts as `census` (inferred,
  by the argument in entry 2), and the committed grid test's `brute`
  function would serve unchanged. Its home is beside
  `coverage_is_exact_on_the_two_party_grid` in `causally/tests.rs`. It
  could instead draw its versions from `testing::exhaustive`'s depth-2 event
  corpus, which contains all 16 (inferred: every boolean four-cell vector is
  a normal-form tree of depth at most 2 with bases in `{0, 1}`). Runtime
  7.24 s in run 1 under nextest; the auditor's run took 7.74 s and the
  builder's 8.4 s and 12.7 s (reported and verified in those logs). That
  fits nextest's limit unignored. No new dependency; it needs a doc comment
  with its exactness argument and an index entry.
- **Overlaps.** The committed two-party grid and #61's extension: the same
  kind of check on a larger lattice, so it would stand beside them. In the
  lane it shares `census` and the clause model with entries 1 and 5.
- **Dependencies.** None. It neither depends on #61 nor loses meaning if
  #61 lands.
- **Value, in one sentence.** The cheapest rescue in the lane: a
  deterministic, total check of exact coverage on a lattice twice as wide as
  the committed grid, in about seven seconds.

### 5. Query membership and exact coverage across worlds (`l5_queries_match_vectors`, `l5_shaped_queries`, `l5_organic_queries`)

- **What it is.** Three properties over one body,
  `body_queries_match_vectors` (lines 1110 to 1271 at `bdaec1dcc`; the
  shaped and organic drivers at `19080507f`). Each case builds a neutral, a
  down-polar, and an up-polar conjunction from the clause model (entry 11).
  It checks membership of every pool version and every bound against the
  clause predicate, and coverage of every ordered segment against `census`
  on both a borrowed and a separately built span. It also checks the
  coverage of bare atoms (`after(v)`, `before(v)`), of singleton queries
  (`Query::from(&v)`), and of point spans through `Span::at` and through a
  bare version.
- **What it reaches or checks.** Uniform grids of 1 to 8 cells, spines to
  depth 40 and random shapes to depth 10, and organic overlays to depth 5.
  Refinement-decided checks per 256 cases: uniform 275, 418, 370; shaped
  477, 744, 745, of which 1,136 deep and 960 wide; organic 368, 638, 445
  (neutral, down, up; verified, run 1). Stored holes reach 3 per query in
  the uniform form (reported, `stats1.log`).
- **Coverage beyond the committed suite.** Exact coverage of polar queries
  deeper than two cells, with heights near `2^31` to `2^1000`; and random
  association and operand order in conjunctions of up to five polar clauses
  plus up to five atoms. The committed grid properties fold left only.
- **Evidence.** The uniform form failed under the same ten coverage mutants
  as entry 1, all also caught by the committed suite (verified). The shaped
  and organic forms carry no mutant evidence. Passes at 4,000 cases
  (uniform) and 3,000 (shaped, organic) (reported, `long2.log`,
  `shape2.log`).
- **Fold-in cost.** Moderate. Runtime in run 1: uniform 0.82 s, shaped
  5.10 s, organic 0.23 s. The clause model duplicates the committed
  `BoundClause`, `DownClause`, and `UpClause`, so a fold-in should extend
  those; the oracle belongs in `testing::oracles` (entry 2), and the shapes
  in `testing::generators` (entry 8).
- **Overlaps.** Entry 1 (same oracle, wider holes). Committed: the two-party
  grid tests (exact, shallow) and `coverage_bounds_membership` (sound,
  deep). It would stand beside both.
- **Dependencies.** None.
- **Value, in one sentence.** Exact coverage of polar queries at depth and
  width, where the committed suite checks soundness only.

### 6. Span-to-query coverage agreement (`l5_span_query_agree`)

- **What it is.** A property; `body_span_query_agree`, lines 1273 to 1312
  at `bdaec1dcc`; 40 lines. For up to 12 ordered pairs: `Query::from(&s)` and
  its owned form admit exactly what `s.contains` admits; `Query::from(&v)`
  admits exactly `v`; and the segment query's coverage of every other span
  `t` is `Full` when `s` contains `t`, `Empty` when `s * t` is `None`, and
  `Partial` otherwise.
- **What it reaches or checks.** Verdicts at 256 cases: `Empty` 13,874,
  `Full` 7,283, `Partial` 9,702 (reported, `stats1.log`). Every `Empty`
  between spans whose endpoints straddle each other is a crossed clamp, the
  neutral case only the refinement decides (inferred).
- **Coverage beyond the committed suite.** A predicate no committed test
  states. The committed tie (`segment_query_matches_span_place`,
  `conversions_denote`) covers membership only, never a segment query's
  coverage of another span (verified, by searching `src` and `tests` for
  `Query::from` on spans). Its oracle is production span algebra, which the
  committed laws check, so this relates two APIs rather than adding an
  independent oracle.
- **Evidence.** M3, M8, M9, M13 (verified), each also caught by six to
  eight committed tests.
- **Fold-in cost.** Very small and standalone: one law in the
  `version_triple` group over the existing `span_candidates`, with no new
  oracle and no model. Runtime 0.73 s as written (run 1).
- **Overlaps.** `segment_query_matches_span_place` (its membership half).
- **Dependencies.** None.
- **Value, in one sentence.** The cheapest new predicate in the lane:
  coverage and the containment algebra must agree.

### 7. The vector model and its verdict oracles (`vle`, `vcmp`, `vlt`, `vjoin`, `vmeet`, `vproject`, `oracle_place`, `oracle_dominance`, `oracle_precedence`, `version_of`, `party_of`)

- **What it is.** A model and oracles; lines 35 to 231 (the comparison and
  lattice helpers and oracles at `bdaec1dcc`, the layout-aware builders at
  `19080507f`); about 200 lines. Versions are `Vec<BigUint>`; order, join,
  meet, and projection are pointwise. `oracle_place` transcribes the nine
  placements from two vector comparisons; `oracle_dominance` and
  `oracle_precedence` transcribe the public definitions directly rather
  than coarsening `oracle_place`. `version_of` and `party_of` build
  production values from vectors through the tree oracle's normalizing
  constructor and the committed bridge.
- **What it reaches or checks.** It is the oracle of entries 1, 5, 10, 12,
  and 14. Every `Span::new(...).unwrap()` on a vector-ordered pair also
  checks production `<=` on that pair (verified by reading).
- **Coverage beyond the committed suite.** Independence. Every committed
  span oracle derives from production `partial_cmp`, which shares the
  overlay advance, `OrderState`, the region readers, and the accumulator
  with the walks under test (verified, imports of `version/order.rs`,
  `version/place.rs`, `version/place/filter.rs`). A defect in that shared
  machinery that moves the walks and `partial_cmp` together passes every
  committed span test, and the committed differential of `partial_cmp`
  against the tree oracle samples depth 4 at most. This model would catch
  such a defect at any depth its generators reach (inferred; none was
  constructed).
- **Evidence.** Behind every verdict kill in the calibration: M6, M7, M13,
  M16, M20 for placement and membership, M12 for the algebra, M14 for
  projected views, all also caught by the committed suite (verified).
- **Fold-in cost.** Small to moderate. The committed function-space oracle
  (`testing::oracles::function`) already models an event as a step
  function, and a committed test already computes this vector form
  (`ev_vector` in `testing/exhaustive/tests.rs`); a fold-in would add
  pointwise order and lattice helpers there. `oracle_place` would be a
  fourth transcription of the nine-way table, beside production's walk
  finish, `OwnSpan::place`, and two committed test transcriptions; feeding
  the two vector relations into an existing test transcription avoids that.
- **Overlaps.** `oracles::function` (the same idea, with no span or query
  operations) and `ev_vector`.
- **Dependencies.** None.
- **Value, in one sentence.** The only span and query oracle that shares no
  machinery with the code it judges.

### 8. Deep and random shapes (`Layout`, `arb_layout`, `arb_shaped_world`, `tree_by_layout`, `party_by_layout`, `set_layout`)

- **What it is.** A generator; `Layout` and its builders at lines 72 to 186,
  `arb_layout` and `arb_shaped_world` at 357 to 400, all at `19080507f`;
  about 160 lines. A layout is a full binary shape whose leaves are the
  cells: a spine of 1 to 40 internal nodes with random turns, a spine of 1
  to 40 that always continues to the right, or a recursive random shape of
  depth at most 10. A thread-local holds the current world's layout so the
  vector builders use it.
- **What it reaches or checks.** 338 of 771 versions at depth 17 to 40, and
  up to 41 leaves (verified, `reach.log`). The records say
  "depth 17 to 64", which is a histogram bucket label: the generator caps
  depth at 40 (verified, `arb_layout`). The committed walk differentials'
  operands, measured by the same probe: all at depth 4 or less with 16
  leaves or fewer (verified, same log).
- **Coverage beyond the committed suite.** Random deep shapes for span and
  query checks under an exact oracle; see the census's "Deep" column. The
  committed deep inputs are fixed registry shapes (`verdict_matrix`, against
  production comparison) and the stack-safety tests (values only, no
  oracle). Spines exercise the walks' cursor sets with one deep side and
  one shallow side.
- **Evidence.** No mutant evidence (written after the calibration). Its
  properties pass at 3,000 cases (reported, `shape2.log`) and in run 1.
- **Fold-in cost.** Moderate. It belongs in `testing::generators` as a
  shape strategy feeding the oracle bridge, with an explicit layout argument
  in place of the thread-local. A census floor on its depth reach would
  keep a later edit from narrowing it. No new dependency.
- **Overlaps.** The algebra lane's tape generators reach depth 300 for the
  lattice operations (reported, `instruments.md`); this is the only deep
  generator paired with a span or query oracle.
- **Dependencies.** None.
- **Value, in one sentence.** Depth ten times the committed generator's,
  for the operations whose cursor machinery depth stresses.

### 9. Grid worlds and near-equal wide heights (`arb_world`, `alphabet_of`, `World`, `extended`, `span_pairs`)

- **What it is.** A generator; lines 233 to 264 and 402 to 500 at
  `bdaec1dcc`; about 130 lines. A world is a uniform grid of `2^d` cells
  and a pool of 2 to 5 vectors over one of seven alphabets: `{0,1,2}`,
  `{0,1,2,3}`, `{W, W+1, W+2}`, `{0, 1, W, W+1}`, `{0, W, 2W}`,
  `{W-1, W, W+1}`, and `{0, 1, W-1, W^2}`, with `W` from `2^31` to
  `2^1000`. `extended` closes the pool's first four vectors under pairwise
  join and meet and adds one-step bumps of the first vector's first four
  cells; `span_pairs` takes every ordered pair.
- **What it reaches or checks.** All nine placements, with `Before` at 9.6%,
  `Concurrent(Start)` at 12.6%, and `Concurrent(End)` at 12.3% of 263,066
  placement checks; the committed walk-differential population gives 1.5%,
  2.4%, and 2.2% of 2,785 (verified, recomputed from `stats1.log` and
  `reach.log`). Equal and comparable pairs are common by construction,
  where two independent committed draws are never equal (reported, the
  adequacy census).
- **Coverage beyond the committed suite.** Near-equal wide heights in
  adjacent cells and across versions, so the walks' private differences are
  built, discarded, and rebuilt on wide values while placement and exact
  coverage are checked. The committed
  `filters_materialize_a_difference_only_when_needed` constructs such pairs
  deliberately, but checks membership and the per-bound fold only.
- **Evidence.** Its properties carry the calibration kills of entries 1, 5,
  7, 10, 12, and 14; nothing beyond the committed suite.
- **Fold-in cost.** Small. The alphabets are a value strategy that
  `testing::generators` could offer beside `arb_magnitude`; `extended` is a
  pool closure in the style of `verdict_matrix`'s closure records.
- **Overlaps.** `arb_magnitude` (wide values, independent draws);
  `verdict_matrix`'s pool closure; the algebra lane's heights on the 63-bit
  boundaries.
- **Dependencies.** None.
- **Value, in one sentence.** It makes the rare placements and near-equal
  wide comparisons common, which the committed population reaches four to
  six times less often.

### 10. Span verdicts against vectors (`l5_span_verdicts_match_vectors`, `l5_deep_verdicts`, `l5_shaped_verdicts`, `l5_organic_verdicts`)

- **What it is.** Four properties over two bodies: `check_span_verdicts`
  and `body_span_verdicts_match_vectors` (lines 809 to 915 at `bdaec1dcc`),
  and `body_deep_verdicts` (at `5cec22e4e`). For every ordered pair in the
  extended pool, with every pool version (and, in the uniform form, four
  in-segment clamps) as probes, it checks `place`, `dominance`,
  `precedence`, and `contains` against the vector oracle. `contains` is
  checked with a borrowed version, an owned version, and a point span. The
  spans are built from shared buffers, separately built buffers, decoded
  storage, and `Span::at`. It also checks `Span::contains(&Span)` against
  endpoint containment.
- **What it reaches or checks.** Uniform grids to 16 cells (`arb_world(4)`)
  and to 64 cells (`arb_world(6)`), spines to depth 40, and organic
  overlays, with entry 9's placement mix.
- **Coverage beyond the committed suite.** An independent oracle at depth
  for all four verdicts, with the rare placements common. `verdict_matrix`
  reaches deep registry shapes against production comparison.
- **Evidence.** M6, M7, M13, M16, M20 (verified), each also caught by six
  to nine committed tests. Passes at 4,000 cases (reported).
- **Fold-in cost.** Moderate. Runtime in run 1: uniform 13.97 s, deep
  4.56 s, shaped 5.93 s, organic 0.23 s. At 3,000 cases under nextest the
  uniform and deep forms hit the 180-second limit of the time (reported,
  `long1.log`), so the case count cannot rise far above the default unless
  the probe set shrinks. The natural home is a law group over vector worlds
  or a second oracle in `version/place/tests.rs`.
- **Overlaps.** Committed: `span_walks_match_the_composed_sweeps`,
  `span_place_matches_relations`, `verdict_matrix`, and
  `point_span_classification_ignores_buffer_identity` (buffer identity for
  points). In the lane, entry 12 checks the same verdicts on projected views.
- **Dependencies.** None.
- **Value, in one sentence.** A second, independent oracle for every span
  verdict at depth; a short property once entry 7 is folded.

### 11. Clause model and conjunction folds (`NClause`, `DClause`, `UClause`, `arb_n`, `arb_d`, `arb_u`, `arb_nclauses`, `Item`, `conjoin_neutral`, `conjoin_down`, `conjoin_up`)

- **What it is.** A generator and a predicate model; lines 502 to 713 at
  `bdaec1dcc`; about 210 lines. Each clause names a public form (`after`,
  `before`, `since`, `!before`, `strictly_after`,
  `after(..).or_concurrent()`, `delta`, and their duals) with `admits` (the
  predicate over vectors), `bounds` (for the census), and `item` (the
  production atom or query). The folds seed with a rotated polar item and
  join each further item on the left or the right as a bit of `shape`
  picks, so both operand orders of every typed `&` cell occur.
- **What it reaches or checks.** Conjunctions of up to five polar clauses and
  up to five atoms, in mixed operand order. It spells `since(v)` and
  `!before(v)` separately (verified, `DClause::item`).
- **Coverage beyond the committed suite.** Random association and operand
  order inside long conjunctions under an exact oracle. The committed
  `conjunction_operand_forms_agree` law covers each typed `&` cell once, for
  membership; the committed grid properties fold left only.
- **Evidence.** Part of entries 1 and 5's kills; nothing beyond the
  committed suite.
- **Fold-in cost.** Small, as an extension of `causally/tests.rs`'s clause
  enums with a fold-side bit, rather than a parallel set.
- **Overlaps.** The committed `BoundClause`, `DownClause`, and `UpClause`,
  which it duplicates almost exactly.
- **Dependencies.** None.
- **Value, in one sentence.** Worth folding only as a delta to the committed
  clause enums: operand order and the `!before` spelling.

### 12. Projected-span views against masked vectors (`l5_own_span_matches_vectors`, `l5_shaped_own_span`, the own-span half of `l5_organic_own_and_algebra`)

- **What it is.** Three properties over one body (lines 1314 to 1356 at
  `bdaec1dcc`). A random mask over the world's leaves becomes a party; for
  every ordered pair, the view `span.project(&party)` must materialize to
  the masked endpoints and give the masked oracle's four verdicts for every
  pool version.
- **What it reaches or checks.** The verdict mix is lopsided: of 218,144
  checks, `After` 30%, `Concurrent(Both)` 41%, `Concurrent(End)` 26%, and
  each of the other six placements 1.3% or less, `At(Both)` 0.35% (verified,
  recomputed from `stats1.log`). Probes are unmasked versions and masked
  endpoints are lower, which explains the skew.
- **Coverage beyond the committed suite.** Independence from production
  projection: the committed `own_span_matches_the_projected_span` law
  compares the lazy view with the eager production projection, so a
  projection defect both share passes it (inferred). Masks at arbitrary
  leaf subsets of deep shapes.
- **Evidence.** M14 (`OwnSpan::dominance` uses `>`), also caught by three
  committed tests (verified).
- **Fold-in cost.** Small once entry 7 exists. Runtime in run 1: uniform
  1.93 s, shaped 3.38 s, organic 0.27 s (with entry 14's organic half). The
  probe set should add masked versions, which would balance the verdict
  mix.
- **Overlaps.** The committed law above and
  `own_span_place_reaches_every_concurrent_corner`.
- **Dependencies.** None.
- **Value, in one sentence.** A modest second oracle for projected views,
  weaker than it looks because six placements are nearly absent.

### 13. Carry-boundary touch family (`l5_touch_carry_family`)

- **What it is.** A cost probe, `#[ignore]`d, behind `touch-meter`; lines
  1738 to 1781 at `19080507f`; 44 lines. A probe whose cells alternate
  `2^m - 1` and `2^m`, against bounds placed so every running difference
  alternates across `2^k`, at four sizes (`m` from 256 to 2,048, cells from
  64 to 512). It prints touch counts for `partial_cmp`, `place`,
  `dominance`, `contains`, `Query::contains`, and `Query::coverage`, and
  asserts nothing.
- **What it reaches or checks.** Touches per input byte are flat across the
  four sizes for all six operations; for example `place` reads 293, 581,
  1,157, 2,309 touches at 234, 466, 930, 1,858 bytes (verified, run 1,
  identical to `long2.log`).
- **Coverage beyond the committed suite.** A family aimed at the walks'
  private differences crossing a carry boundary in every cell. The board
  prices the same operations on its registry families, whose cliff combs
  cross `2^k` in the heights themselves (verified, the registry's family
  docs), not in a probe-minus-bound difference (inferred; I did not trace
  every family's differences).
- **Evidence.** It found the walks linear, so there was no regression to
  catch. Its readings were identical under the tie-order mutant (reported).
- **Fold-in cost.** Moderate: a registry `Shape` family, its ceilings, and
  worst-case entries. As written it has no failure condition at all.
  Runtime 0.06 s (run 1).
- **Overlaps.** The board's span and query rows; the suanpan lane's touch
  adversary, which found no width-dependent cost (reported, survey 2.1).
- **Dependencies.** Question 65's ceiling rule (ruled, not yet built) would
  set its ceiling.
- **Value, in one sentence.** A plausible board family that has never
  failed and checks nothing today.

### 14. Span algebra against vectors (`l5_span_algebra_matches_vectors`, `l5_shaped_algebra`, the algebra half of `l5_organic_own_and_algebra`)

- **What it is.** Three properties over one body (lines 917 to 1108; the
  binary operators at `bdaec1dcc`, the hull and fold spellings at
  `5cec22e4e`); 192 lines. It checks `+`, `|`, `&`, `*`, `union_all`,
  `join_all`, `meet_all`, `intersect_all`, `span_all`, `Sum`, `Product`,
  `collect`, `Version::span` and `^`, the version-left operators, and `+=`,
  each against pointwise vector formulas.
- **What it reaches or checks.** Up to 8 by 8 span pairs per case and
  families of up to 6, over the worlds of entries 8 and 9.
- **Coverage beyond the committed suite.** Small. The committed laws
  `span_union_is_the_containment_join`,
  `span_intersect_is_the_shared_segment`, `span_join_is_the_pointwise_join`,
  `span_meet_is_the_pointwise_meet`, the fold laws in `version_list.rs`, and
  `span_is_the_pair_hull` cover every spelling here and more, against
  production lattice operations, which the committed lattice differentials
  check against the tree oracle. This property adds only the independent
  oracle and the regimes. Its doc comment claims "intersection is the set
  intersection over the generated sublattice", but the code checks the
  endpoint formula only (verified by reading); the two agree in a
  distributive lattice, but no set is enumerated.
- **Evidence.** M12 (`intersect_points` returns the hull), also caught by
  two committed tests (verified).
- **Fold-in cost.** Small, with low return; the doc comment needs
  correcting either way. Runtime in run 1: uniform 1.34 s, shaped 2.25 s.
- **Overlaps.** Nearly total with the committed laws.
- **Dependencies.** None.
- **Value, in one sentence.** Little beyond the committed laws; fold only
  as part of a vector-oracle law group, if at all.

### 15. Organic overlays (`arb_organic_world`, `overlay`, `heights`, `shape_of`, `party_shape`)

- **What it is.** A generator; lines 266 to 355 at `19080507f`; about 90
  lines. It runs a committed op-trace population (`optrace::world_strategy`),
  overlays every version's and party's shape into one layout, reads each
  version's heights on it, and asserts that every organic version
  round-trips through that overlay.
- **What it reaches or checks.** Depth at most 5 and at most 15 leaves in a
  256-case sample (reported, `shape1.log`). Its query population has 1,451
  refinement-decided checks per 256 cases, 273 at depth 5, none wide
  (verified, run 1); organic heights stay small.
- **Coverage beyond the committed suite.** Exact coverage on shapes that
  real schedules produce, which the committed suite checks only for
  soundness (`laws_hold_on_organic_populations`). The shapes are shallower
  than entry 8's.
- **Evidence.** None; written after the calibration.
- **Fold-in cost.** Small if entries 2 and 7 are folded. Runtime 0.15 to
  0.27 s per property (run 1).
- **Overlaps.** `optrace`; the function-space oracle's replay and its
  `lift_ev`, which also turns trees into functions.
- **Dependencies.** None.
- **Value, in one sentence.** Low: organic shapes are shallow, and the
  other generators already cover them.

### 16. Committed-population reach probe (`l5_reach_committed`, `tree_leaves_depth`)

- **What it is.** A diagnostic, `#[ignore]`d; lines 2028 to 2130 at
  `2874e0c3d`; 103 lines. It replays the committed walk differentials'
  population (three `arb_oracle_version` draws, spans from their meet and
  join) and histograms its operand shapes, placements, and coverage
  verdicts, beside the shaped worlds' shapes.
- **What it reaches or checks.** The comparison numbers in entries 8 and 9
  (verified, `reach.log`).
- **Coverage beyond the committed suite.** None as a check. It is the only
  measurement of the committed span population's placement mix.
- **Evidence.** Not applicable.
- **Fold-in cost.** As a census floor (a test failing when the committed
  population's rare-placement share drops below a floor) it would be small;
  as written it always passes.
- **Overlaps.** The adequacy lane's generator census (`l8_census.rs`),
  which measures shapes but not placements.
- **Dependencies.** None.
- **Value, in one sentence.** Useful only if the owner wants placement-mix
  floors on the committed generators.

### 17. The calibration mutants (M1 to M20)

- **What it is.** A mutant set that is not on the branch: 19
  `L5_MUT`-gated branches in nine production files, built once and run one
  at a time, then restored with `git restore --source=HEAD` (reported,
  `coverage-record.md`). Only the table of sites survives; no diff was
  kept. The logs are on the box under
  `~/src/rumors-audit-l5-spans/target/l5mut/` and summarized in
  `mut-run.log`.
- **What it reaches or checks.** Coverage settle direction, strict-hole
  finishing, `refine_partial`'s endpoint and crossed check, the placement
  finish, the dominance and precedence hooks, the filter's difference fold,
  `compare_heights`, hole absorption and survival, point intersection and
  containment, `OwnSpan::dominance`, and the point-span fast path.
- **Coverage beyond the committed suite.** None: the committed lane suite
  caught all 17 non-equivalent mutants and also M18, which the harness
  missed (verified, `mut-run.log`).
- **Evidence.** Committed kill counts per mutant: 2 for M12; 3 for M5, M10,
  and M14; 4 to 9 for the rest (verified, `mut-run.log`).
- **Fold-in cost.** The diffs must be rebuilt from the table, about an
  hour's work (inferred). Their use would be as the known-bad set for any
  fold-in above, run as a calibration rather than committed.
- **Overlaps.** The adequacy lane's cargo-mutants campaign over the same
  files (306 mutants in the span group; reported).
- **Dependencies.** None.
- **Value, in one sentence.** A ready-made calibration list for whatever is
  folded in, and nothing more.

### 18. Refinement scan-cost probe (`l5_refine_partial_scan_cost`)

- **What it is.** A cost probe, `#[ignore]`d, behind `scan-meter`; lines
  2132 to 2172 at `2874e0c3d`; 41 lines. It splits `Query::coverage`'s scan
  bits on a neutral `Partial` verdict into the fused walk's share and the
  refinement's, and prices the `floor <= ceiling` comparison, at four sizes.
- **What it reaches or checks.** The refinement scanned 1.54 times as many
  bits as the walk at every size, 64 to 512 cells (verified, run 1,
  identical to `reach.log`).
- **Coverage beyond the committed suite.** None now. It was the evidence for
  #61, which replaces the measured code, and #94 adds a board row that
  meters the bounded neutral case on every family. It calls the
  crate-private `filter::coverage` directly, so after #61 its "refine"
  column would measure the new code.
- **Evidence.** It motivated #61's 37% cut, which the builder reproduced
  exactly ("reproduces the brief's table exactly", verified,
  `builder-refine-partial/NOTES.md`).
- **Fold-in cost.** Not worth folding. Runtime 0.06 s (run 1).
- **Overlaps.** #94's `query_coverage_neutral` row; the #61 builder's
  grafted `l5_refine_partial_scan_by_polarity`, which is the build-and-review
  cataloguer's, not this lane's.
- **Dependencies.** #61 and #94 supersede it.
- **Value, in one sentence.** Its job is done; #94 carries its purpose.

### 19. Behavioral histograms (`stat`, `STATS`, `run_stats`, `l5_stats`, `l5_stats2`, `l5_stats3`)

- **What it is.** Diagnostics, `#[ignore]`d, spread across all four
  commits: lines 25 to 33, 1411 to 1470, 1715 to 1736, and 1872 to 1919. A
  global map of event counts that each property body increments, and three
  tests that run the bodies at 256 cases and print the map.
- **What it reaches or checks.** Every reach number in the lane's records.
  `l5_stats` took 88 s (verified, `stats1.log`).
- **Coverage beyond the committed suite.** None as a check.
- **Evidence.** Not applicable.
- **Fold-in cost.** Not worth folding as written: the global mutex makes
  counts depend on which tests share a process.
- **Overlaps.** `l8_census.rs`.
- **Dependencies.** None.
- **Value, in one sentence.** Scaffolding; keep the numbers, not the code.

### 20. The tie-order experiment (`overlay.rs:142`, `>` to `>=`)

- **What it is.** A one-off mutant experiment and its argument
  (`lanes/l5-spans/round-1/tie-order.md`).
- **What it reaches or checks.** The L5 harness, the cube, both cost probes,
  and 94 committed lane tests all passed with identical readings under the
  swap (reported, `tie.log`).
- **Coverage beyond the committed suite.** None. It shows the mutant is
  equivalent for the placement and filter walks; the adequacy lane classes
  the same survivor as equivalent for all four `CursorSet` users
  (reported), and #85 corrects the docs it found overstated.
- **Evidence.** Supports the equivalence.
- **Fold-in cost.** Nothing to fold.
- **Overlaps.** The adequacy survivor index; #85.
- **Dependencies.** None.
- **Value, in one sentence.** Settled; nothing to rescue.

### 21. The witness heuristic (`witness_coverage`)

- **What it is.** A heuristic coverage oracle; lines 783 to 805 at
  `bdaec1dcc`; 23 lines. Its own comment calls it a heuristic, and nothing
  calls it (verified, by searching the file).
- **What it reaches or checks.** Nothing.
- **Coverage beyond the committed suite.** None.
- **Evidence.** None.
- **Fold-in cost.** Delete it, and remove its mention from entry 3's doc
  comment.
- **Overlaps.** Entry 2 supersedes it.
- **Dependencies.** None.
- **Value, in one sentence.** None; dead code.

## What I could not assess

- Whether the harness compiles on current `main`. I built it only at
  `2874e0c3d`; the lane's code is unchanged since, so I expect it does.
- Mutant evidence for the shaped, organic, cube, carry, and validator
  instruments. None exists, and the brief limits box runs to census and
  runtime measurement, so I did not create mutants.
- Whether the carry family's difference regime is absent from every board
  family. I read the families' docs, not their differences.
- The validator's sensitivity to a weakened census.

## Run record

- Run 1: `on-illumos.sh /Users/oxide/src/rumors-rescue-l5 'unset
  CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; ...'`, with a `--no-run`
  build (4 min 9 s, cold), then `cargo nextest run -p before --all-features
  --locked -E "test(/audit_l5::/)"` (17 passed, 26.1 s wall), then the three
  ignored probes under nextest (3 passed, 7.2 s), then `cargo test ... --
  --ignored --nocapture --exact testing::audit_l5::rescue_l5_refine_census`
  (passed, 49.3 s). No phase reported a failure status. Log:
  `<scratchpad>/rescue-l5/run1.log`; census extract: `census.txt`; probe
  patch: `rescue-census-probe.diff`.
- `<scratchpad>` is
  `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad`.
