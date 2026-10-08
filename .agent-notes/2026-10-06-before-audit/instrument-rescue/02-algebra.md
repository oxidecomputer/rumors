<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the instrument-rescue catalogue for lane L2 (version algebra), from `explore/l2-algebra` at `6748bcd41`, the lane's records, and one census run on ox-east-1 on 2026-10-08; for Finch's review. -->

# Lane L2 (version algebra): every instrument the lane built

## Summary

Lane L2 built one large instrument, a differential probe of the version
algebra, plus a cost probe and some mutation tooling. Everything lives on
`explore/l2-algebra` (tip `6748bcd41`, merge-base with `main` `1745d3779`)
except the tooling of entries 13 and 16, which lives in the auditor's session
scratchpad, where a reboot deletes it. This file gives a full entry to every
instrument, in my order of value, and one line to each thing a ready branch
carries and to each intermediate file that is not an instrument.

**What the lane adds.** The probe checks every algebra entry point against a
leaf-list model of versions and parties that shares no code with production.
This model is independent of the tree representation and practical at depth
300, and no committed oracle is both. The function-space oracle samples a grid
of `2^g` points with `g` capped at 32, and the committed lattice differential
builds its expected values through production's own `VersionWriter`; entry 1
gives the sources. The probe's generators reach depth 300 and 1,034 leaves,
where the committed arbitrary versions stop at depth 4 and 11 leaves. No L2
instrument caught a defect, or a mutant that the committed suite missed.

**Corrections to the lane's records.** I found five while cataloguing, each
verified:

1. The lane's census log (`census1.log`) was written at 09:43:46 on
   2026-10-07. That is before the probe's first commit (09:44:11), and before
   `117dcb1eb` (09:52:12) added the 900-split, depth-300 tier. So the census
   numbers in the round-1 coverage record and in `instruments.md` describe an
   earlier four-tier generator. The numbers below are measured at the tip.
2. The probe killed all 20 round-1 mutants, but only 9 of those kills came
   from its own model assertions (M01, M02, M08 to M13, M28). The other 11
   came from production debug assertions or panics on the mutated path, which
   any test that reaches the path would trip.
3. Round 2 ran the probe against 18 survivors from the adequacy lane (L8) and
   2 tie-order mutants, and the probe caught none. By L8's own classification,
   15 of the 18 change no value (classes C, E, and U), so no value check can
   catch them. The other three lie outside the probe's predicates or inputs: a
   constant `Hash`, a writer path only `tick` reaches, and a span buffer
   holding only a lower endpoint. So the survey's "caught none of 18" says
   little about the probe either way.
4. In the deepest size tier, most of the probe's later draws read an
   exhausted tape. In that tier, 97.7% of fold draws and 64.0% of first-party
   draws get only zero bytes, which yield an empty fold and the seed party. The
   deepest operands are therefore almost never folded, and usually projected
   through the seed party. This is a new measurement; see the exhaustion table
   below.
5. The 20 round-1 mutants and their driver exist only under `/private/tmp`
   (entry 13).

**Baseline.** `instrument-rescue/00-baseline.md` did not exist when I wrote
this. I compare against two sources: the adequacy lane's census of the
committed generators (`lanes/l8-adequacy/round-1/census-baseline.txt`, 20,000
draws per generator), and the committed tests I read at `main` `434cbfc82`.
`crates/before/src` at `main` differs from the explore tip's base only in one
test file (`testing/exhaustive/tests.rs`), so the lane's runs apply to
`main`'s production code (verified, `git diff --stat 1745d3779 main --
crates/before/src`).

**My measurement.** I used one of the three allowed box runs, in a detached
scratch worktree at the tip, with two uncommitted additions:

- a read-only `Tape::pos` accessor;
- an ignored test, `rescue_census`, that replays `run_case`'s draws in
  `run_case`'s own order: limits, levels, the pair, party `p`, party `q`, the
  perturbed version `c`, and the fold arity.

The census runs over the same ChaCha8-seeded tapes the lane's census uses,
10,000 of them, and changes no generator. For seeds 0 to 1999 its counts equal
the lane's own census rerun at the tip exactly: relations None 683, Equal 523,
Less 464, Greater 330, and all six family counts. So it replays the same tapes.
It took 12.1 s. The log and the diff are in the session scratchpad
(`rescue-l2/run1-census.keep.log`, `rescue-l2/census.diff`), which a reboot
deletes; this file copies every number that matters. Afterwards I restored the
two files (`git diff` empty) and removed the worktree with plain `git worktree
remove`. Its box copy, `~/src/rumors-rescue-l2`, remains for the coordinator to
retire.

**Provenance.** Every claim is marked *verified* (I checked it against code, a
log, or my run), *reported* (a record says so), or *inferred* (my reasoning,
not checked by a run).

## Reach against the committed generators

These committed generators reach the algebra's operations (verified, by
reading `testing/generators.rs`, `testing/exhaustive.rs`,
`testing/diff_ops.rs`, and `version/lattice/tests.rs`):

- `arb_oracle_version`, capped at depth 4 (`ARB_DEPTH`), feeds the
  differential table, the laws, and the lattice proptests.
- `exhaustive` enumerates events to depth 2 (691 events).
- Organic operation traces reach version depth 6 (reported, the adequacy
  census).
- Fixed families reach depth 64 (`Dense(64)`, `AltSpine(64)`), and one test
  reaches 512 (`flat_over_deep_collapses_totally`).
- The deep random shapes `shape_version` and `shape_party` feed only the tick
  tests.
- The depth-100,000 stack tests use spines only.

| Quantity | Committed (adequacy census unless noted) | L2 probe at the tip (my census, 10,000 tapes) |
|---|---|---|
| Version depth | max 4, median 3 | median 12, 90th percentile 100, max 300 |
| Leaves per version | max 11 | median 28, 90th percentile 278, max 1,034 |
| Encoded bits per version | median 844, max 5,015 | median 2,340, 90th percentile 30,202, max 485,963 |
| Widest leaf height, in bits | median 128, max 514 | median 65, 90th percentile 345, max 700 |
| Pair relation | concurrent 65.3%, less 17.6%, greater 17.2%, equal 0% | concurrent 34.2%, equal 27.2%, less 22.4%, greater 16.3% |
| Both operands deeper than 4 | never, in arbitrary pairs | 64.7% |
| Party | depth max 4, at most 10 leaves | first party: depth median 4, 90th percentile 68, max 300; up to 834 regions; deeper than 4 and not the seed in 48.6% |
| Fold arity | 0 to 17 (`arb_fold_arity`, verified) | 0 in 38.0%, 1 to 8 in 12.1%, 9 to 17 in 14.0%, 18 to 40 in 35.9% |
| Payload codes at the 63/65-bit boundary | no census measures them | a 63-bit code in 22.9% of first operands, a 65-bit code in 41.1% |

The committed column is the adequacy lane's measurement (reported), except
where marked; the probe column is my census (verified). The committed heights
are wider at the median because `arb_magnitude` weights `u128`-scale values. The probe instead aims its heights at the boundary between
narrow and wide codes (entry 5).

The next table shows how often a later draw starts on an exhausted tape, by
size tier (my census). The property's tapes are 0 to 5,999 bytes long. An
exhausted draw reads zeros, which yields the seed party for `p` and `q`, an
empty fold, and for `c` the first operand raised by one constant level
(verified, by reading `gen.rs`).

| Tier (splits, depth) | Share of cases | `p` | `q` | `c` | fold |
|---|---|---|---|---|---|
| 6, 4 | 19.9% | 1.1% | 1.3% | 1.7% | 2.1% |
| 40, 12 | 19.8% | 4.0% | 5.4% | 7.1% | 8.2% |
| 120, 140 | 20.6% | 12.1% | 16.0% | 19.6% | 24.2% |
| 300, 70 | 20.3% | 22.7% | 31.9% | 41.1% | 51.4% |
| 900, 300 | 19.5% | 64.0% | 81.0% | 92.3% | 97.7% |
| All | 100% | 20.6% | 26.9% | 32.1% | 36.5% |

The 16 long-run copies (entry 12) draw tapes up to 11,999 bytes, so they
exhaust less often; I did not measure how much less.

## Entries, in order of value

### 1. The leaf-list model of versions and parties

- **What it is.** An independent reference model, at
  `explore/l2-algebra:crates/before/tests/l2_probe/model.rs` (`6748bcd41`): 475
  lines, including the encoders of entry 6 and the census helpers of entry 8
  (verified).
- **What it reaches or checks.**
  - A version is its canonical list of `(depth, height)` leaves, left to
    right; a party is a list of `(depth, owned)` regions.
  - Every operation works on the common refinement of the operands' exact
    dyadic start positions, held as `BigUint` numerators over `2^k`.
  - Join and meet are the pointwise maximum and minimum, followed by a
    stack-merge canonicalization. `le` and `relation` read the pointwise order.
    `project` zeroes the unowned cells. `plateaus`, `cells`, and `overlay`
    render `shape`, `combine`, and `Clock::shape`.

  It lives in an integration-test crate, so it can reach only `before`'s
  public API, and it shares no cursor, delta arithmetic, or tree walk with
  production (verified). Its cost grows with the number of refined cells times
  the width of a position, not with `2^depth` (inferred, from the code).
- **Coverage beyond the committed suite.** The committed oracles cannot do
  this at depth:
  - The function-space oracle, the project's other structurally independent
    model, loops over `0..(1u64 << g)` sample points
    (`testing/oracles/function.rs:598`, `:621`, `:694`), with `g` capped at
    `GRID_N = 32`. So it is limited to shallow inputs (verified).
  - The recursive tree oracle shares production's tree representation, by the
    function-space module's own account (reported, the module doc of
    `testing/oracles/function.rs`).
  - The committed lattice differential converts the tree oracle's answers to
    `Version` through production's `VersionWriter`
    (`bridge::from_oracle_version` calls `testing::version::from_tree_stream`),
    and reads production's outputs back through `VersionTreeReader`
    (verified). A writer canonicalization error shared by both directions
    would pass that differential, and fail against this model's independently
    encoded bytes (inferred).

  So no committed model checks the algebra on versions deeper than 4
  independently of both the tree representation and the writer, and this one
  does (verified for the committed suite). The other lanes' explore-branch
  models are assessed in their own catalogues.
- **Evidence.** The model's own assertions killed 9 of the 20 round-1
  mutants:
  - two in the lattice's switch-delta arithmetic (M01, M02);
  - projection's leave-ownership step (M08);
  - three in the masked comparison's signs and block-skip heights (M09, M10,
    M28);
  - `<` answering true on equal versions (M11);
  - the fold's merged-with-merged arm (M12);
  - `span_all` building its upper endpoint from a lower one (M13).

  Source: the first failure line of each log in the scratchpad's
  `auditor-l2/mut/` (verified). The committed suite also killed all 20
  (verified, `mut/round2-suite/summary.txt`). The model caught no defect. The
  lane found one bug in the model itself: `Clock::shape` refined by the raw
  rather than the canonical party, and production was right (reported, lane
  NOTES).
- **Fold-in cost.**
  - The natural home is a third oracle module in `testing::oracles`, for
    example `oracles::leaf_list`. The `VM` and `PM` type synonyms should
    become newtypes, by the project's convention, and every item needs a doc
    comment (most have one now).
  - A fourth column in `diff_ops` would be costly: every descriptor would need
    a leaf-list spelling, and the model has no tick, fork, or party algebra.
    A focused public-API differential beside the table fits better (inferred).
  - The validation index needs an entry naming its failure class: semantic
    errors on deep inputs, and writer errors that production and the bridge
    share.
  - No new dependencies: `num-bigint` is already a dependency, and `rand` and
    `rand_chacha` are dev-dependencies (verified, `crates/before/Cargo.toml`).
  - Runtime: included in entry 2's.
- **Overlaps.** It plays the function-space oracle's role at depths that
  oracle cannot reach, and would stand beside both committed oracles, not
  replace either. The identity lane's interval-set model of parties (L1) and
  the spans lane's grid model (L5) are independent models of neighboring types
  (reported, `instruments.md`).
- **Dependencies.** None on unlanded branches; it uses only the public API.
- **Value, in one sentence.** It is an already-built oracle that judges the
  algebra's answers on deep, wide inputs without sharing the tree
  representation or the writer with the code under test, which no committed
  oracle does.

### 2. The differential property `algebra_matches_model` and its checks

- **What it is.** A proptest property and its check functions, in
  `explore/l2-algebra:crates/before/tests/l2_probe/main.rs` (`6748bcd41`):
  `run_case`, `check_lattice`, `check_order`, `check_projection`,
  `check_shape`, `check_folds`, and `check_spans`. They make up most of the
  file's 822 lines (verified).
- **What it reaches or checks.** Each case decodes model-encoded operands
  through the public strict decoder. It then compares production against the
  model, byte for byte wherever the result is a version (verified, by
  reading):
  - **Join and meet:** the named method, the four operator cells, and both
    assignment forms, plus `span` and `^`. The operand pairs are `(a, b)`,
    `(b, a)`, `(a, a.clone())`, `(a, redecode(a))`, the join with the meet,
    and the two endpoints of a decoded span.
  - **Comparisons:** every spelling on the same operand kinds: `partial_cmp`
    both ways and through references, the five operators in owned, borrowed,
    and mixed forms, and `concurrent` both ways. It also checks that equal
    values hash equally.
  - **Projection:** through `/` and `project`, materialized by `to_version`
    and `From`, and every view-against-version and view-against-view
    comparison, all against the model's restriction.
  - **Shapes:** `shape`, `combine` over zero, two, and three inputs,
    `Clock::shape`, and `Party::shape`.
  - **Folds:** `join_all`, `meet_all`, `span_all` (owned and borrowed),
    `Sum`, and `collect`, over up to 40 items mixing shared clones, separately
    decoded copies, adjacent repeats, and the empty version.
  - **Span placement:** `place`, `dominance`, `precedence`, and `contains` for
    coincident, unshared-point, and hull spans over a six-version pool, against
    relations the model computes.
  - **Smaller checks:** chained results (`(a | b) & c`, `(a & b) | c`, and a
    projection joined back), `is_empty`, `Default`, the bit and byte bounds on
    join and meet, and `encoded_bits` against the model's own bit count.

  Its reach is the generators' (entries 3, 4, 5, and 7) and the tables above.
- **Coverage beyond the committed suite.** The committed suite states most of
  these predicates already. What this property adds is mainly the oracle and
  the reach, not new predicates (verified, by reading the laws and the
  differential table):
  - The laws are oracle-free by design. `span_place_matches_relations` derives
    its expected verdict from production's own comparisons, and
    `own_version_cmp_matches_materialized` compares a view with production's
    own materialization. Here both expectations come from the model.
  - The differential table checks shapes, projection, and comparisons against
    both oracles, but on depth-4 operands.
  - I found three predicates stated nowhere in the committed suite:
    - The operator cells `Version | &Version` and `&Version | Version`, and
      the owned-right `|=` and `&=`, checked by value. The laws check
      `a.join(b) == a | b` and the full `^` matrix. This is inferred from a
      search, not an exhaustive reading.
    - Fold arity above 17. The committed laws stop at 17, which crosses the
      balanced fold's carry boundaries at 8 and 16 (verified, the doc of
      `arb_fold_arity`) but not the next one, at 32 (inferred, from the fold's
      binary counter).
    - Lattice and fold operands that are two slices of one decoded span
      buffer. Committed span decoding already compares two such slices
      (`span/wire.rs:142` and `:148`), so what is new is the rest of the
      battery on them (verified).
  - The byte bound adds nothing to the committed bit bound, which implies it
    (inferred).
- **Evidence.**
  - Two long runs totalled about 54,400 cases, all passing (the case counts
    reported; both logs show 16 of 16 copies passing, verified,
    `auditor-l2/long1.log` and `long2.log`).
  - Round 2 ran 3,200 cases against each of 20 mutants, and ran the probe on a
    trial merge of #30 with the clone-free branch as it stood then
    (`ab548dafd`, which contains #36) (verified, `round3-mutants.log` and
    `round3-branches.log`; the branch contents reported, round-2 report).
  - It killed all 20 round-1 mutants: 9 through the model's assertions
    (entry 1), and 11 through production debug assertions on paths it drove
    (verified).
  - After #36 lands, the operator cells stop being uniform one-line
    delegations: each maps a shared short-circuit outcome to a clone or a
    move (verified, by reading `simplify/lattice-entry-delegation`). The
    per-cell value checks then have distinct code to check. #36's own new
    check, `tests/meter/lattice_clones.rs`, is a heap check over three
    entry points that borrow their left operand (verified).
- **Fold-in cost.**
  - Runtime: 48.2 s at the default 256 cases in a debug build (verified,
    `auditor-l2/run9.log`, at `d78c6129`, whose probe source is identical to
    the tip's). That is inside nextest's 300-second limit, but long for one
    test.
  - The repo's conventions point toward splitting it: one property per family
    of checks, each with a doc comment stating its invariant, all sharing one
    generator.
  - The single byte tape should give way to per-component strategies with
    their own size budgets, since entry 3's large tiers starve the later
    draws (the exhaustion table).
  - Folding these predicates into `testing::laws` instead would discard the
    model. The natural home is a public-API differential test binary in
    `crates/before/tests/`, with a validation-index entry (inferred).
- **Overlaps.** It overlaps the laws, the differential table, and the lattice
  proptests predicate by predicate. It overlaps the spans lane's grid model on
  span placement (reported, `instruments.md`). It would stand beside all of
  them as the deep, independently checked instance.
- **Dependencies.** None to compile. Its per-cell checks gain value once #36
  and the queued owned-operand moves land. It passed on the trial merge of
  #30 with the clone-free branch described above.
- **Value, in one sentence.** It applies the independent model to every
  algebra entry point at once, so a future change to any fast path, operator
  cell, or fold arm meets an oracle that shares none of its assumptions, at
  depths to 300 where the committed generators stop at 4.

### 3. Deep random partitions (`gen::partition` and `limits`)

- **What it is.** A generator of dyadic partitions: `gen.rs`'s `partition`
  (about 45 lines) and `main.rs`'s `limits` (verified).
- **What it reaches or checks.** It splits leaves repeatedly under one of six
  strategies (verified):
  - uniform choice, which grows bushy trees;
  - always the leftmost leaf, which grows a left spine;
  - always the rightmost leaf, which grows a right spine;
  - a focus that wanders by at most one leaf per split, which grows a zigzag
    spine;
  - two mixtures of focus and uniform choice, which grow combs.

  Each case uses one of five size tiers, each a fifth of cases: 6 splits to
  depth 4, 40 to 12, 120 to 140, 300 to 70, and 900 to 300 (verified). At the
  tip, first operands reach a median depth of 12, a 90th percentile of 100,
  and a maximum of 300, with up to 1,034 leaves. 21.1% are deeper than 64 and
  36.6% have more than 64 leaves (my census, and the lane's census rerun at
  the tip; verified).
- **Coverage beyond the committed suite.** No committed algebra test draws
  random topology deeper than 4. The deeper committed operands are fixed
  families (verified). Depths above 64 also cross word boundaries, in random
  shapes, in the writer's stacks of per-ancestor bits (inferred, from the
  writer's module doc, which says the stacks "use bits per open ancestor").
- **Evidence.** No defect and no unique mutant. Reach as measured above.
- **Fold-in cost.** A `prop_flat_map` strategy in `testing::generators`
  producing a leaf list or a `tree::Version`, of about the 45 lines it has now
  (inferred).
  It needs a size budget per component (see the exhaustion table), and a
  census floor if you want its reach held. The 900-split tier dominates its
  runtime (inferred).
- **Overlaps.**
  - The committed `shape_version` family (spines, zigzag, bushy; all
    deterministic), which only the tick tests use.
  - The events lane's (L3) co-generated tick pairs on #74 (spine, wide, and
    multi-scan strategies).
  - The spans lane's (L5) shaped generators, which reach depth 17 to 64
    (reported).
- **Dependencies.** None.
- **Value, in one sentence.** It is the cheapest already-built way to give the
  algebra's committed properties random topology past depth 4.

### 4. Correlated pair families (`gen::pair`, `perturb`, `decomposition`, `sparse`)

- **What it is.** Pair generators in `gen.rs`, about 110 lines (verified).
- **What it reaches or checks.** Six families, each drawn in 13.5% to 19.6%
  of cases (my census; the code verified):
  - independent pairs;
  - `perturbed` and `perturbed-rev`: a version paired with itself after
    adding, subtracting (saturating at zero), joining, or meeting a sparse
    function; `-rev` swaps the pair;
  - join and meet decompositions. These build two operands whose join (or
    meet) is exactly a chosen target: on each cell, one operand holds the
    target and the other sits below it (for a join) or above it (for a meet).
    So the output collapses wherever the target is flat, while each operand
    keeps its own structure;
  - equal pairs, two separately decoded copies of one value.

  At the tip, 27.2% of pairs are equal, 38.7% comparable but unequal, and
  34.2% concurrent (my census). The lane's census rerun at the tip counts
  writer collapse paths over the first 2,000 pairs (verified):

  | Writer path | Joins | Meets |
  |---|---|---|
  | a direct collapse keeping a wide code | 14.9% | 15.2% |
  | a cascade collapse keeping a wide code | 15.4% | 16.3% |
  | a narrow cascade after a wide one | 7.4% | 8.9% |

- **Coverage beyond the committed suite.** Committed arbitrary pairs are never
  equal and are 65% concurrent (reported, the adequacy census). The committed
  lattice tests compensate by also pairing each operand with the pair's join
  and meet. Organic histories pick equal versions 67% of the time, and the
  same clock 55% of the time (reported, the adequacy census). No committed
  generator is built to force writer collapse cascades on random deep
  operands; the decomposition families are (verified, by reading). No
  committed census counts writer paths, so how often the committed pairs reach
  them is unknown (verified that no census counts them).
- **Evidence.** Nothing unique.
- **Fold-in cost.** These strategies call `model::pointwise`, `join`, `meet`,
  and `refine`, so they fold in with entry 1. Small (inferred).
- **Overlaps.** The committed `family_pairs_emit_identically` (pool pairs plus
  their join and meet), `flat_over_deep_collapses_totally`, and the
  re-anchoring scan check `reanchor_join_scan_is_linear_per_input_bit`.
- **Dependencies.** Entry 1.
- **Value, in one sentence.** It generates the tie-heavy and collapse-heavy
  pairs that the writer's cascade paths need at random depth, where the
  committed generators reach such pairs only at depth 4 or in fixed families.

### 5. The boundary-height palette (`gen::level`, `gen::levels`)

- **What it is.** Height generators in `gen.rs`, about 40 lines (verified).
- **What it reaches or checks.** Each case draws one to four levels, plus
  zero. A level is a base plus an offset (verified):
  - The base is zero half the time; otherwise it is `2^31`, `2^32 − 2`,
    `2^64 − 1`, `2^128`, or `2^k` for some `k < 700`.
  - The offset is 0 to 3, `2^31 − 1`, `2^31`, `2^31 + 1`, `2^32 − 2`,
    `2^32 − 1`, `2^32`, `2^63`, `2^64`, `2^64 + 1`, or `2^k` for some
    `k < 300`.

  So differences between levels land on the writer's narrow-code limit,
  `NARROW_CODE_BITS = 63` (`writer.rs:79`). A delta of `2^31 − 1` codes in 63
  bits and a delta of `2^31` in 65; a first height of `2^32 − 2` codes in 63
  bits and `2^32 − 1` in 65 (verified, by deriving the gamma lengths from
  `model::code_bits`).

  At the tip, a 63-bit code appears in 22.9% of first operands and a 65-bit
  code in 41.1% (22.2% and 40.2% in join outputs). A collapse whose surviving
  left code is exactly 63 bits occurs in 2.4% of joins directly and 1.8% by
  cascade, and in 2.4% and 1.9% of meets (my census, verified). That cascade
  case is where the writer's `take_left` chooses between its narrow and wide
  paths (inferred, from `writer.rs:154-163`).
- **Coverage beyond the committed suite.** Both sides of the code boundary are
  already committed, but only in fixed topologies (verified):
  `wide_grid_pairs_emit_identically` puts plateau widths of 29 to 34 and 60 to
  68 bits on a fixed 32-cell grid, and `cliff_staircase_pairs_emit_identically`
  steps through exponents 62 to 66. The palette adds boundary codes in random
  deep topology and inside collapse cascades. The round-2 record called the
  `take_left` case "plausible, not measured"; it is reached in about 2% of
  cases. The mutant at that comparison (`writer.rs:155`, `>` to `>=`) is
  cost-only by L8's classification, so no value check could catch it
  (reported, L8 class C).
- **Evidence.** Nothing unique.
- **Fold-in cost.** A few lines, as another arm of an `arb_magnitude`-style
  strategy in `testing::generators`. It matters only when paired with a
  topology generator (inferred).
- **Overlaps.** `arb_magnitude` (the `u64` and `u128` boundaries, sparse high
  bits), the wide grids, the staircases, and the board's cliff comb.
- **Dependencies.** None.
- **Value, in one sentence.** It is cheap, and no committed generator aims
  heights at the code-width boundary inside random topologies and collapse
  cascades.

### 6. Independent canonical encoders (`model::encode_version`, `encode_party`, `version_bits`, `Bits`)

- **What it is.** Bit-level encoders in `model.rs`, about 120 lines
  (verified).
- **What it reaches or checks.** It writes the documented canonical streams
  bit by bit: preorder topology flags; the first leaf's absolute height as the
  Elias gamma code of `h + 1`; each later leaf's zigzag delta as the gamma
  code of `code + 1`; then a marker bit and zero padding. Parties get two
  presence bits per node, with `00` for an owned leaf (verified). The probe
  compares every version production returns with this encoding, byte for
  byte, and checks `encoded_bits` against `version_bits` (verified).
- **Coverage beyond the committed suite.** The committed lattice tests get
  their expected values through production's writer (entry 1). The committed
  codec tests check production against itself: round trips, injectivity, and
  strict-decode rejection (inferred, from test names such as
  `canonical_encoding_is_injective` and `decode_encode_roundtrip`). This
  encoder ties every algebra result in the probe to the written
  specification.
- **Evidence.** Production debug assertions killed all six round-1 writer
  mutants (M03 to M07, M38) before the byte comparison ran (verified), so
  those kills do not demonstrate this encoder. In a release build the byte
  comparison would be the remaining check (inferred).
- **Fold-in cost.** Tiny if folded in with entry 1.
- **Overlaps.** The codecs lane's (L6) specification codec covers all six wire
  types, in both directions, with error classes (reported, `instruments.md`).
  One of the two suffices for encoding independence. L6's is broader; this
  one is already wired to the algebra's outputs.
- **Dependencies.** None.
- **Value, in one sentence.** It makes every algebra result a check of the
  wire specification, which L6's codec would do as well once wired to the
  algebra.

### 7. The party region generator (`gen::party`)

- **What it is.** A party generator in `gen.rs`, about 25 lines (verified).
- **What it reaches or checks.** It takes a partition from entry 3 and assigns
  ownership in one of four styles: a fair coin, mostly owned, mostly unowned,
  or alternating. It forces at least one owned region (verified). At the tip
  (my census):
  - First party `p`: depth median 4, 90th percentile 68, max 300; up to 834
    regions and 717 owned regions. It is the seed party in 32.9% of cases
    (mostly exhausted tapes), and both deeper than 4 and not the seed in
    48.6%.
  - Second party `q`: the seed in 38.2% of cases; deeper than 4 and not the
    seed in 43.3%.
- **Coverage beyond the committed suite.** `arb_oracle_party` stops at depth 4
  and 10 leaves (reported, the adequacy census). Deep fragmented masks reach
  the masked comparisons and projection's block skip at random depth.
- **Evidence.** Nothing unique. The two round-1 mutants in projection's
  skipping (M17 and M18) were killed by production assertions (verified).
- **Fold-in cost.** Small, folded in with entry 3. It needs the late-draw
  starvation fixed (see the exhaustion table).
- **Overlaps.** The identity lane's (L1) party generators reach depth about
  120 and 128 bytes, with their own interval-set model (reported,
  `instruments.md`). The board's `ScatteredId` and mask families cover cost.
- **Dependencies.** None.
- **Value, in one sentence.** It produces deep random masks for the masked
  walks, which no committed value test produces; L1's generator is the
  alternative.

### 8. The writer-path census (`census`, `model::canon_stats`, `CanonStats`, `code_bits`)

- **What it is.** An ignored test in `main.rs` (about 90 lines) and its model
  helpers (about 60 lines) (verified).
- **What it reaches or checks.** It draws 2,000 ChaCha8-seeded tapes. For each
  it records the family, size tier, relation, depth and leaf buckets, and the
  projection relation. Then it replays canonicalization on the model to count
  which writer paths each join and meet takes: a direct or cascade collapse,
  keeping a narrow or wide code, and a narrow cascade after a wide one
  (verified). It ran in 1.78 s at the tip (verified, my run).
- **Coverage beyond the committed suite.** The adequacy census measures the
  committed generators' shapes and sizes, but no writer paths or code widths
  (verified, `census-baseline.txt`). No committed instrument reports
  writer-path reach; this one does.
- **Evidence.** It is diagnostic and always passes. Its recorded output
  predates the tip (correction 1); entries 3 and 4 quote its output at the
  tip. Two flaws, both verified:
  - Its `totals` line sums only three of its six counters, so it prints zeros
    for the other three. This misleads a reader but corrupts no histogram.
  - It measures only the first pair and the first party, which is how the
    starvation of the later draws went unseen.
- **Fold-in cost.** As a census in the style of L8's `l8_census.rs`, or as
  reach floors on a folded-in generator. Depends on entry 1 (inferred).
- **Overlaps.** L8's census of the committed generators, and the events
  lane's (L3) reach diagnostics.
- **Dependencies.** Entry 1.
- **Value, in one sentence.** It would let a folded-in generator commit floors
  on writer-path reach, so that a later generator change cannot stop reaching
  collapse cascades without a test failing.

### 9. The stack-overflow calibration (`deep_calibration_recursion_overflows`)

- **What it is.** An ignored test in `main.rs`, about 20 lines (verified).
- **What it reaches or checks.** It recurses one call frame per level over a
  version 100,000 levels deep, on a 2 MiB thread, and is expected to abort. It
  does: `thread '<unknown>' (3) has overflowed its stack`, SIGABRT, after
  0.154 s (verified, `auditor-l2/run7.log`).
- **Coverage beyond the committed suite.** It shows, on the box's actual debug
  build, that this depth defeats a minimal recursion. #75 makes a stronger
  argument: its constant `STACK_SAFETY_DEPTH = 2^18` leaves 8 bytes of a
  2 MiB stack per level, below the 16-byte minimum frame, and its reviewer
  measured a 16-byte frame fitting at 100,000 levels and overflowing at `2^18`
  (reported, `QUESTIONS.md` entry #75). This calibration therefore
  demonstrates a weaker bound than the one #75 now relies on (inferred).
- **Evidence.** The abort above.
- **Fold-in cost.** A committed known-bad demonstration needs a child process
  to observe the abort, for example re-running the test binary under an
  environment variable and asserting the signal (inferred). Moderate work; no
  runtime concern.
- **Overlaps.** The derivations in #34's and #75's docs, and their reviewers'
  recursive rewrites (not committed).
- **Dependencies.** None.
- **Value, in one sentence.** Low: #75's constant and its recorded derivation
  already state the bound, and this calibration demonstrates less than that
  derivation needs.

### 10. The fragmented-mask cost families

- **What it is.** Ignored tests in `explore/l2-algebra:crates/before/tests/l2_cost/main.rs`
  that need the features `touch-meter` and `scan-meter`: `masked_battery`,
  `comb_party`, `fragmented_mask_family`, and
  `fragmented_mask_stepping_family`, about 95 lines (verified).
- **What it reaches or checks.** It meters digit touches and scanned bits per
  input bit for seven masked operations at `n` = 1,024, 2,048, 4,096, and
  8,192 (verified, `auditor-l2/cost2.log`):
  - *fragmented mask*: one wide plateau per operand (`2^n − 1` against
    `2^n − 2`) under comb parties with regions at every depth from 1 to `n`;
  - *stepping*: the same masks, with operands that step by one at each comb
    boundary.

  Every masked comparison stays flat: `view_cmp` reads 0.017 touches per bit
  at every size, 0.209 for the stepping variant. `project` grows linearly per
  input bit (3.30 to 25.7 touches per bit), consistent with its documented
  `O(|self|^2)` output (verified). The two tests take 0.36 s and 0.71 s
  (verified). The family divides `project`'s cost by input bits only.
  Divided by input plus output bits, the right denominator for an operation
  whose output can exceed its input, the growth reflects the documented output
  size rather than extra work (inferred).
- **Coverage beyond the committed suite.** The board's `mask-drift` family (a
  cliff comb under a scattered mask, against a `2^k` plateau, with `k` and the
  tooth count scaled together) and its `masked-hole` family (a deep spine
  under a shallow mask) target the same masked walks (verified, by reading
  `mask_drift_triple` and `masked_hole` in `testing/meter.rs`). The L2 masks
  differ in shape: regions at every depth inside one plateau, so each region
  boundary forces a sign read of an unchanging wide difference (inferred).
  Whether that reaches a cost regime the board misses is unmeasured.
- **Evidence.** No finding; every comparison is flat.
- **Fold-in cost.** A board family in `testing/meter/board/family.rs`, with
  registry shapes, a second version and masks, and re-recorded worst cases.
  Moderate work. It would land after #93, #94, #95, and the ceiling-rules
  work, which all touch the board (inferred).
- **Overlaps.** The board's `mask-drift`, `masked-hole`, and scattered-id
  families.
- **Dependencies.** Sequencing after the board branches above.
- **Value, in one sentence.** Low to moderate: a differently shaped mask
  adversary for walks the board already attacks with two families.

### 11. The carry-ripple cost families

- **What it is.** Ignored tests in `l2_cost/main.rs`: `battery`, `grid`,
  `ripple_family`, and `ripple_close_family`, about 110 lines (verified).
- **What it reaches or checks.** It meters 12 operations at 1,024 to 8,192
  cells: join, meet, span, `partial_cmp`, `<=`, `>=`, `concurrent`, a
  three-item `join_all`, three masked comparisons, and `project`. Operand `a`
  alternates `2^n` and `2^n − 1` on a balanced grid, so every unit delta
  crosses a power of two. Operand `b` is 1 on the left half and 0 on the
  right (*ripple*), or the constant `2^n − 1` (*ripple-close*). Every reading
  is flat per input bit; for example join reads 0.285 to 0.288 touches per bit
  (verified, `auditor-l2/cost1.log`). Each test takes about 0.19 s.
- **Coverage beyond the committed suite.** The board's `CliffComb`, whose teeth
  are `2^k − 1` and `2^k` with `k` scaled with `n`, is the same alternation on
  a spine. This family puts it on a balanced grid of depth `log n` (the board
  side verified, by reading the registry docs; the equivalence of regime
  inferred).
- **Evidence.** None.
- **Fold-in cost.** A board family, as for entry 10.
- **Overlaps.** The board's cliff comb.
- **Dependencies.** As entry 10.
- **Value, in one sentence.** Low: it restates the board's cliff comb in a
  balanced topology.

### 12. The parallel long-run copies (`par00` to `par15`)

- **What it is.** Sixteen macro-generated, ignored copies of entry 2's
  property, with tapes up to 11,999 bytes and no failure persistence
  (verified).
- **What it reaches or checks.** The same as entry 2, with longer tapes and so
  fewer exhausted draws (unmeasured).
- **Coverage beyond the committed suite.** None: this is a way to run entry 2,
  not a check. The committed equivalent is nextest's `high-count` profile with
  `PROPTEST_CASES`.
- **Evidence.** The 54,400 long-run cases of entry 2.
- **Fold-in cost.** None needed. A folded-in property can run long under
  `PROPTEST_CASES` (inferred).
- **Overlaps.** Entry 2.
- **Dependencies.** None.
- **Value, in one sentence.** None to fold in; they matter only while the
  probe is investigative.

### 13. The hand-chosen round-1 mutants and `mutate.py` (scratch only)

- **What it is.** `auditor-l2/mutants-all.json` in the session scratchpad: 20
  string swaps. With it, `mutate.py` (about 30 lines) applies one unique swap,
  runs a remote command, restores the file, and checks `git diff --quiet`
  (verified).
- **What it reaches or checks.** It mutates two sites in the lattice's switch
  delta, six in the writer, seven in projection and the masked comparison, one
  in the order, two in the folds, one in the shape walk, and one in the
  overlay (verified).
- **Coverage beyond the committed suite.** None: the committed suite killed all
  20 (verified; each log names its first failing committed test, and nextest
  stopped at the first failure). The probe's model killed 9, and production
  debug assertions the other 11 (verified).
- **Evidence.** As above. The files live under `/private/tmp` and do not
  survive a reboot (verified).
- **Fold-in cost.** No code to fold in. Copying the JSON beside the lane
  records would keep a ready calibration set for entries 1 and 2: a folded-in
  version of them should still kill the nine the model killed (inferred).
- **Overlaps.** L8's cargo-mutants campaign covers these files more broadly;
  other lanes made their own hand-chosen mutants.
- **Dependencies.** None.
- **Value, in one sentence.** Small, but at risk of loss: nine of these are the
  only demonstration that the model, rather than production's own assertions,
  detects errors.

### 14. The survivor and tie-order patches, with `run.sh`

- **What it is.** Twenty patch files and `run.sh` (41 lines), under
  `explore/l2-algebra:crates/before/tests/l2_probe/mutants/` (verified).
  Eighteen patches are L8's survivors; two reverse a cursor priority.
- **What it reaches or checks.** On the box, `run.sh` applies each patch, runs
  the 16 copies at 200 cases plus the two deep tests, reverts the patch, and
  checks `crates/before/src` against `HEAD` (verified). Patches 18 and 19
  reverse the cursor priority in projection's and placement's `CursorSet`s.
- **Coverage beyond the committed suite.** Patches 18 and 19 each passed 3,200
  cases, which supports the round-2 argument that tie order cannot change a
  verdict (the logs verified; the argument reported). The other 18 patches
  copy L8's diffs, and L8's record remains their source.
- **Evidence.** None of the 20 patches failed the probe; 15 of the 18
  survivors are value-neutral (correction 3).
- **Fold-in cost.** None as code. The doc comment at `projection.rs:417-419`
  already states the tie-order rule, and round 2 found it accurate
  (reported).
- **Overlaps.** L8's survivor records.
- **Dependencies.** None.
- **Value, in one sentence.** Low: a record of a settled question.

### 15. `run_on_tree.sh`

- **What it is.** A 39-line box script on the branch (verified).
- **What it reaches or checks.** It runs the probe and the named patches
  against the synced working tree as it stands, which may be an uncommitted
  trial merge. It checks restoration against a hash of the tree's starting
  diff from `HEAD`, rather than against `HEAD` (verified).
- **Coverage beyond the committed suite.** None.
- **Evidence.** Used once, on the trial merge of #30 with the clone-free
  branch (entry 2): the unmutated run and all four mutants passed (verified,
  `auditor-l2/round3-branches.log`).
- **Fold-in cost.** None.
- **Overlaps.** Entry 14's `run.sh`.
- **Dependencies.** None.
- **Value, in one sentence.** Low: a way to test an uncommitted merge, not an
  instrument to fold in.

### 16. `extract.py` (scratch only)

- **What it is.** A script of about 30 lines in `auditor-l2/` that turns L8's
  survivor markdown into git patches (verified).
- **What it reaches or checks.** Nothing; it is a converter.
- **Coverage beyond the committed suite.** None.
- **Evidence.** It mangled one patch: the markdown lost a blank context line
  for patch 08, which was regenerated by hand (reported, round-2 report;
  consistent with `writer_mut.rs`, verified).
- **Fold-in cost.** None.
- **Overlaps.** L8's own survivor export.
- **Dependencies.** None.
- **Value, in one sentence.** None to fold in; it was a one-off.

## Carried by ready branches

- `deep_fork_halves_are_stack_safe` is carried by #34 (`audit/deep-surfaces`),
  as `deep_tree_shape_hull_and_fold_stack_safety` at depth `2^18` on left and
  right spines (verified).
- `deep_surfaces_are_stack_safe` is carried by #34, and its view-against-view
  comparison by #75 Part C (`deep_tree_remaining_surfaces_stack_safety`).
  Neither branch restates one of its assertions: `to_version` of a projection
  onto a concurrent or disjoint region at depth (verified, by reading both
  branches).
- `spine_party`, a deep left spine whose tip or the tip's sibling is owned,
  built as canonical bytes, is carried by #34's `deep_right_spine_party`
  beside the committed `deep_left_spine_party` (verified).

Builders turned this lane's simplification briefs into #30 (one sweep) and
#36 (one short-circuit ladder). #36 adds a heap check,
`tests/meter/lattice_clones.rs`; it is a builder's instrument, not this
lane's.

## Intermediate files, not instruments

- `auditor-l2/span_check.rs.txt` is an earlier draft of `check_spans`,
  superseded by the version on the branch (verified).
- `auditor-l2/writer_mut.rs` is `writer.rs` with survivor 08 applied, used to
  regenerate patch 08 (verified, a one-line diff).
- `auditor-l2/18-tie-projection-reversed.rs` and
  `19-tie-place-reversed.rs` are the mutated copies behind patches 18 and 19
  (verified, one-line diffs).
- `auditor-l2/selected.txt` lists the 18 survivor names fed to `extract.py`
  (verified).
- `auditor-l2/mutants1.json` and `mutants2.json` are the two halves of
  `mutants-all.json` (verified).

The lane's two arguments, the proof sketch of the encoding-size bound
(`round-1/coverage.md`) and the tie-order settlement (`round-2/report.md`),
are records rather than instruments.

## What I could not assess

- How often the committed generators reach writer collapse cascades or codes
  at the width boundary: no census counts them, and building one was outside
  this brief.
- How much the long-run copies' longer tapes reduce the starvation of the
  later draws.
- Whether the fragmented-mask families reach a cost regime the board's mask
  families miss. Their readings are flat, so the question has no consequence
  today.
- The probe's behavior in a release build; every run was a debug build.
