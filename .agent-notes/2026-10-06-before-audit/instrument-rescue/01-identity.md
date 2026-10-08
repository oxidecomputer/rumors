<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the identity lane's (L1) section of the instrument rescue, from the explore branch, the lane's records, and two measurement runs on ox-east-1 on 2026-10-08; not reviewed. -->

# Instrument rescue, lane L1 (identity)

## Summary

The identity lane's explore branch, `explore/l1-identity`, is at
`346a82ac9`. Its merge-base with `main` is `d5e80103a`, and it carries 12
commits past it (verified, `git log`). Its instruments are:

- a black-box discovery suite in `crates/before/tests/audit_l1/`, built
  around an interval-set model of `Party`;
- two printing probes, `tests/audit_l1_cost.rs` and
  `tests/audit_l1_retention.rs`;
- two lib-test additions: a statistics printer for the committed party
  generators and the D1 size-hint probe;
- the MB2 drafts.

The branch also carries #34's commit by merge. Two more instruments live
only in the auditor's scratch directory: the mutant lists with their
calibration driver, and the unoptimized-build configuration of the deep
tests.

This file gives 14 instruments full entries. Six more items get one line
each, because a ready branch carries them or they are superseded.

The three I judge most valuable:

1. **The party generators** (`arb_set`, `arb_disjoint`). They reach depth
   80 and more than 16 two-child branches in 42% of draws, against depth 4
   and at most 4 committed. They also produce accepted `join_all` families
   of 4 to 10 arbitrary parties, which the committed family generator
   cannot produce at all.
2. **The interval-set model.** It is the only exact party oracle that
   shares no code with the tree recursion and still works past depth 20.
3. **The rule-respecting history driver.** It applies `forks`, array
   splits, `join_all`, `sync_all`, and byte round trips inside histories.
   The committed operation traces have none of these, and its populations
   exceed 8 parties about 100 times as often.

### How I measured

The rescue brief asks for reach measured against
`instrument-rescue/00-baseline.md`. That file did not exist while I wrote
this. I compared against the committed generators' own sources and the
auditor's census of them instead.

- **The census still holds.** The committed generators,
  `testing::optrace`, `testing::bridge`, and every identity source file
  are byte-identical between the lane's base `58285ca5` and `main`
  (verified: `git diff --stat 58285ca5 main` over those paths is empty).
  So the auditor's census of the committed generators,
  `stats-committed.log`, still describes `main`.
- **Path shorthand.** `<scratch>` means the session scratchpad,
  `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad`.
  The auditor's logs are in `<scratch>/auditor-l1/`, and mine are in
  `<scratch>/rescue-l1/`. Both live under `/private/tmp` and do not
  survive a reboot.
- **Two box runs.** I ran twice on a scratch worktree at the tip, now
  removed. Run 1 built the three integration-test binaries
  (`run1-build.log`, exit 0). Run 2 ran every L1 integration test at its
  default case count, plus a census I added for this catalogue
  (`run2-default-and-census.log`, exit 0, 25 of 25 passed).
  - Run 2's conditions: box load 19.8 at start and 21.2 at end, 24 test
    threads, and the dev profile, which builds `before` at `opt-level = 2`
    (`Cargo.toml:189-190`, verified).
  - The census code is `<scratch>/rescue-l1/census.rs`. It is on no
    branch.
  - The cost and retention probes reproduced every reading in
    `cost-1.log` and `retention-6.log` exactly (verified).
- **The time limit.** Fold-in runtimes are judged against nextest's limit
  at `main`: a test is flagged slow after 60 seconds and terminated at 300
  (`.config/nextest.toml:29`, verified).

### Corrections to the lane's records

These matter to anyone reading `lanes/l1-identity/round-1/`.

1. **M26 was not unique to the deep probe.** The inventory says only the
   explore deep probe caught M26, a recursive removal descent. #75's
   reviewer found five `amp_board_smoke` tests that catch it at the base
   (reported, ready entry #75). The auditor's selection never ran that
   binary.
2. **The two-child branch figure misreads a bucket.** Observation O8 says
   `arb_set` has "32 or more two-child branches in 42%". In
   `stats-1.log`, the bucket labelled 32 holds 17 to 32. The true figures
   are 17 or more in 42%, and 33 or more in 20.5% (verified, from the log).
   Depth above 32 is 46.75%, not 48% (verified).
3. **The fork-step cost is off by about 6,000 bits.** `coverage.md` gives
   the scan cost of `forks(2^d).next()` as "about 4,000 + 6d" bits. The
   readings in `cost-1.log` give 16,015, 34,011, 106,013, and 394,002 bits
   at d = 1,000, 4,000, 16,000, and 64,000. That is about 10,000 + 6d,
   still linear in `log k` (verified arithmetic).
4. **The cost probe's `is_disjoint` row measures nothing.** It reads 0.00
   scan bits at every size. Both operands own `[0, 1/2)` as their first
   leaf, so the comparison stops at once (inferred from the constructions;
   the 0.00 reading agrees). None of the lane's records says so.

### Carried by ready branches, or superseded (one line each)

- **MB1, the multiplicity check on a failed `join_all`:** #40
  (`tree::Party::same_multiplicity` and the `# Errors` contract) and #81
  (the laws `*_conserves_multiplicity`).
- **The MB2 drafts:** #75, built from scratch. It carries
  `deep_identity_stack_safety`, `deep_tree_remaining_surfaces_stack_safety`,
  the depth constant `STACK_SAFETY_DEPTH = 1 << 18`, and
  `without_constructed`'s raised top scale.
- **#34's content, present here only by merge:**
  `deep_tree_shape_hull_and_fold_stack_safety` and
  `deep_right_spine_party`, both from #34, the algebra lane's branch.
- **The D1 probe** `explore_distant_plan_is_exact_once_the_remainder_fits`:
  superseded by #43's regression tests, which drive the plan through real
  steps.
- **SB1's coverage:** #76's law `sync_all_agrees_with_join_all` and its
  fixed late-overlap shape.
- **MB3:** declined by the owner (question 68). Entry 14 records the
  configuration anyway, because it was constructed and run.

Not instruments, and not catalogued:

- the MB2 brief committed on the branch;
- the S1 retention patches and `movementB-by-op.txt` in
  `lanes/l1-identity/round-1/S1-retention-variants/`, which are candidate
  fixes and a record of board movement for slot 33.

### Ranking

| Rank | Instrument | Kind | Location on `explore/l1-identity` |
|---:|---|---|---|
| 1 | Party generators `arb_set`, `arb_disjoint` | generator | `tests/audit_l1/gen.rs` |
| 2 | Interval-set model of `Party` | oracle | `tests/audit_l1/model.rs` |
| 3 | Rule-respecting history driver | operation-sequence property | `tests/audit_l1/history.rs` |
| 4 | `join_all` and `sync_all` against the model | properties | `tests/audit_l1/main.rs` |
| 5 | Set algebra, fork, splits, and sync against the model | properties | `tests/audit_l1/main.rs` |
| 6 | Wide-count fork properties | properties | `tests/audit_l1/wide.rs` |
| 7 | Deep probe on a 256 KiB stack | stack-safety probe | `tests/audit_l1/deep.rs` |
| 8 | Scan-cost probe | cost probe (prints) | `tests/audit_l1_cost.rs` |
| 9 | Heap-retention probe | resource probe (prints) | `tests/audit_l1_retention.rs` |
| 10 | Overlay refinement property | property | `tests/audit_l1/overlay.rs` |
| 11 | Generator statistics printers | census | `tests/audit_l1/main.rs`, `src/party/tests.rs` |
| 12 | Mutant lists and calibration driver | calibration | scratch only |
| 13 | Deep-party builders | fixtures | `deep.rs`, `audit_l1_cost.rs`, `audit_l1_retention.rs` |
| 14 | Unoptimized-build deep-test configuration | build configuration | scratch only |

All `explore/l1-identity` paths are relative to `crates/before/`.

---

## 1. Party generators `arb_set` and `arb_disjoint`

- **What it is.** Proptest strategies in
  `explore/l1-identity:crates/before/tests/audit_l1/gen.rs` at `346a82ac9`,
  added in `03213585e`, with the depth parameter from `1b1c16e72`; 172
  lines. The construction:
  - `arb_set_upto(max)` draws sorted dyadic cut points. Some are single
    points at depths weighted toward the shallow end, and some are clusters
    at many depths around one base point. It then owns a random nonempty
    subset of the pieces between the cuts.
  - `arb_disjoint(k)` colors the pieces with `k` colors or none, and forces
    each color onto at least one piece. It returns `k` pairwise-disjoint
    nonempty parties.
  - Values are model sets, and tests build production parties through the
    public `Party::decode` of the model's canonical bytes.
- **What it reaches or checks.** It checks nothing itself. Its reach comes
  from 4,000 draws each (`<scratch>/auditor-l1/stats-1.log`, verified from
  the log; buckets are exact below 4, then the next power of two):
  - Tree depth: above 4 in 76%, above 32 in 47%, and from 65 up to the
    generator's cap of 80 in 23%.
  - Encoded size: above 4 bytes in 67%, above 16 bytes in 42%, and above
    32 bytes in 5.4%.
  - Two-child branches: more than 4 in 66%, 17 or more in 42%, and 33 or
    more in 20.5%.
  - Longest unary chain: more than 4 in 41%, and 33 or more in 5.1%.
  - Relations between two independent draws: disjoint 14.8%, nested 50.7%,
    equal 1.3%.
  - Members of `arb_disjoint(4)`: deeper than 32 in 42%.

  The committed `arb_oracle_party_nonempty` stops at depth 4, 4 bytes, 4
  two-child branches, and a unary chain of 4. That bound holds by
  construction, since `ARB_DEPTH = 4` (`testing/generators.rs:246`), and it
  shows in the census. Its pairs are 14.5% disjoint, 48.6% nested, and 3.2%
  equal. The parties of committed operation traces stay within depth 8 and
  3 bytes, with no two-child branch in 93% (`stats-committed.log`,
  verified).
- **Coverage beyond the committed suite.**
  - *Depth.* Every party law, differential row, and registry law today
    sees arbitrary parties of depth 4 at most, and organic ones of depth 8
    at most. These generators reach depth 80 and nest many two-child
    branches along one path.
  - *Accepted families.* The committed `arb_party_family` draws a receiver
    and items from a pool of 1 to 4 parties, with repetition
    (`generators.rs:376-378`, verified). A repeated party overlaps itself,
    so a family that `join_all` accepts holds at most 3 items. Only 5.55%
    of committed families are accepted (`stats-committed.log`). The clock
    family's pool holds at most 3 parties, so an accepted clock family
    holds at most 2 items (verified, `generators.rs:384-401`).
    `arb_disjoint(k)` gives accepted families of arbitrary interleaved
    parties at any `k`; the suite uses `k` up to 10. The committed suite
    reaches larger accepted families in only two ways. One is fork shares
    of one party (`party_join_all_reunites_forks_at_any_width`), whose
    structure is fixed. The other is organic trace populations, which
    exceed 8 parties in 0.6% of traces (`stats-committed.log`).
- **Evidence.** It caught no defect through reach alone, and the auditor
  found no traversal mutant that only these generators catch (reported,
  observation O8). Every mutant the suite caught, it caught with these
  generators driving it (entries 3 to 6).
- **Fold-in cost.**
  - *Where it goes.* Port the strategies into `testing::generators`,
    returning `tree::Party` so the existing bridge and oracles take them.
    The port carries `Set::from_intervals`, `cover`, and `encode` from the
    model (about 90 lines), or an equivalent builder from cut points to a
    tree.
  - *Oracles.* The tree oracle recurses on depth, which is safe at 80: the
    committed `without_constructed` runs it at depth 4,096
    (`ORACLE_SCALE_MAX`, `party/tests.rs:497`, verified). The
    function-space oracle cannot take these operands.
    - Its comparison grid must stay under 64 levels, which an assertion
      in `fs_grid` enforces (`oracles/function.rs:96-101`).
    - Its resolution probe scans `2^ceiling` cells
      (`function.rs:160-165`).

    So a differential row fed by `arb_set` must cap the function leg's
    operands with `arb_set_upto(max)` at a small `max`, or skip that leg
    (verified by reading). Laws need no oracle and take the deep parties
    directly.
  - *Runtime and dependencies.* Drawing 4,000 parties with their
    statistics took 2.8 s (`run2`, measured). No new dependency.
- **Overlaps.**
  - *Committed.* It stands beside `arb_oracle_party(_nonempty)`,
    `arb_party_family`, and `arb_clock_family` rather than replacing them,
    because their small pools supply the aliases and repeats the family
    laws need. The committed `shape_party` builds spines, zigzags, and
    bushy parties to depth 128, but only the tick tests use it (verified,
    `git grep`).
  - *Other lanes.* The adequacy lane's census measured the committed side.
- **Dependencies.** None.
- **Value, in one sentence.** It is the cheapest way to move every
  existing party law and tree-oracle comparison from depth 4 to depth 80,
  and the only generator of large accepted families of arbitrary parties.

## 2. Interval-set model of `Party`

- **What it is.** A test-support oracle in
  `tests/audit_l1/model.rs` at `346a82ac9` (added in `03213585e`); 298
  lines.
  - *Representation.* A party is a sorted list of disjoint, non-adjacent
    half-open intervals of `[0, 1)`, with `u128` numerators over `2^126`.
  - *Set algebra* (union, intersection, complement, difference,
    disjointness, cover) is interval arithmetic.
  - *Codec.* `encode` derives the canonical bytes by a preorder descent
    over dyadic nodes, classifying each as full, empty, or partial.
    `decode` parses production bytes and asserts that they re-encode
    identically, so non-canonical output from production fails loudly.
  - *Fork.* `fork` is the paper's split, characterized without trees:
    descend to the smallest dyadic node containing the set, then split it
    by halves. `shares(n)` is the balanced ceil-left, floor-right
    partition.
  - *Other.* `regions` lists the canonical partition, and `same_multiset`
    compares pointwise multiplicity.
- **What it reaches or checks.** It is an oracle and checks nothing by
  itself.
  - *Independence.* It imports nothing from `before`. Production parties
    enter it only through `as_bytes` and return only through
    `Party::decode` (verified, reading `model.rs` and its callers). Its
    representation is not a tree, so it shares no recursion with
    production or with `testing::oracles::tree`.
  - *Range and traversal.* It represents any party to depth 126 and
    asserts beyond that. Every traversal uses an explicit stack, except
    `shares`, which recurses on `log n` rather than on depth (verified).
- **Coverage beyond the committed suite.** The committed suite has two
  party oracles, and neither covers what this one does:
  - *The tree oracle* realizes the same tree recursion as production. The
    function-space oracle's own documentation says that "impl == oracle"
    is therefore "blind to a bug the two share" (`function.rs:4-7`).
  - *The function-space oracle* is independent, but it has three limits
    (verified, `function.rs:1-45`, `76-102`, `140-175`):
    - its grid must stay under 64 levels, and it scans `2^ceiling` cells;
    - it chooses a random valid split for `fork`;
    - it checks relations (disjointness, order, containment) rather than
      the exact balanced shares.

  This model is therefore the only oracle that computes exact canonical
  results for `fork`, `forks`, the array splits, `sync`, and `sync_all`
  independently of tree recursion, at depths from 20 to 126.
- **Evidence.** It is the oracle for entries 3 to 6 and 10.
  - *Mutants.* The properties built on it (entries 3 to 6 and 10) caught
    21 of the auditor's 24 mutants (reported, `calibration.md` and the
    inventory). The other three, M22, M25, and M26, rewrite a loop as
    recursion, which only a deep stress test can see. M19 and M20, which
    duplicate a rejected group, were caught only through the model's
    `same_multiset` (verified, `calib-run-3.log`). #40 and #81 now carry
    that check.
  - *Defects.* It caught no production defect.
- **Fold-in cost.**
  - *Where it goes.* Move it to `testing::oracles` as a third party
    oracle, test-only or behind the `oracle` feature.
  - *Documentation.* About 15 items need doc comments: the constants
    `MAXD` and `ONE`, `Cover`, and the basic set operations.
  - *The multiplicity helper.* `same_multiset` duplicates #40's
    `tree::Party::same_multiplicity`, so keep one.
  - *Runtime.* The properties that use it run in 0.15 to 0.96 s each at
    default counts (`run2`, measured).
  - *Limits and dependencies.* Its 126-level ceiling means the deep
    stress tests cannot use it. It needs only `std`.
- **Overlaps.**
  - *Other lanes.* The codecs lane's specification codec decodes parties
    independently too, and also judges malformed input, which this model
    never sees (reported, ranker handoff). The algebra lane's leaf-list
    model is the version-side counterpart (reported).
  - *Committed.* It would stand beside the tree oracle as a second,
    independent check, not replace it: the tree oracle also covers
    versions and clocks.
- **Dependencies.** None. If #40 lands, keep one multiplicity helper.
- **Value, in one sentence.** It is the only instrument that could tell a
  bug shared by production and the tree oracle in `fork`, `forks`, or
  `sync` from correct behavior, at depths the function-space oracle cannot
  reach.

## 3. Rule-respecting history driver

- **What it is.** An operation-sequence property in
  `tests/audit_l1/history.rs` (`03213585e`); 269 lines, driven by
  `rule_respecting_histories` in `main.rs` at 256 cases.
  - *Operations.* Its `Op` enum has ten kinds:
    - `Tick`, `Fork`, `Join`, and `Sync`;
    - `Forks(i, k, t)`: `forks(k).take(t)`, with `k` and `t` under 9;
    - `Split`: the consuming array split into 2, 3, 5, or 8;
    - `JoinAll` and `SyncAll`, each with up to 5 others;
    - `Move`: a `Clock` encode and decode;
    - `MoveParty`: `into_parts`, a `Party` encode and decode, then
      `from_parts`.
  - *Population.* Each history starts from one seed clock and is capped at
    24 clocks; an operation that would exceed the cap is skipped.
- **What it reaches or checks.** After every step, it checks four things:
  - every live party equals the model's;
  - the model's parties are pairwise disjoint, and their union is the
    whole interval;
  - every pair of live production parties is disjoint;
  - no `join`, `join_all`, `sync`, or `sync_all` failed.

  It does not check versions: `Tick` changes nothing in the model.

  The census over 2,000 histories and 40,446 steps (`run2`, measured)
  shows which operations actually execute:

  | Operation | Executed | Detail |
  |---|---:|---|
  | `fork` | 5,551 of 5,697 | |
  | `forks` | 5,520 of 5,822 | full drain 2,382; partial drain 1,837; built and dropped untaken 604; zero count 697 |
  | array split | 3,612 of 3,902 | into 2: 931; 3: 867; 5: 918; 8: 896 |
  | `join` | 4,054 of 5,833 | the rest pick one clock twice |
  | `join_all` | 3,885 | 1,645 with two or more others; 1,388 with none |
  | `sync` | 3,969 of 5,702 | the rest pick one clock twice |
  | `sync_all` | 3,817 | 1,619 with two or more others; 1,280 with none |
  | byte moves | 1,931 clock, 1,914 party | |
  | `tick` | 1,943 | |

  Population: 51% of steps end with at least 8 clocks, and 68% of
  histories peak above 8 (`run2`). 63% end above 8 (`stats-1.log`, 500
  histories).
- **Coverage beyond the committed suite.**
  - *Operations.* `testing::optrace::Op` has only `Tick`, `Ticks`, `Fork`,
    `Send`, `Sync`, and `Join` (`optrace.rs:201-215`, verified). So no
    committed history applies `forks`, an array split, `join_all`,
    `sync_all`, or a round trip through bytes, and no committed history
    checks that those operations never fail under rule-respecting use.
    The crate page promises that a `join` or `sync` error is always
    definitive.
  - *Populations.* Committed traces end with more than 8 clocks in 0.6% of
    traces (`stats-committed.log`, 1,000 traces), about a hundredth of
    this driver's rate.
- **Evidence.** It caught M01, M02, M03, M05, M06, M10, M15, M16, M17,
  and M24 (reported, inventory item 4). The committed suite caught each of
  them too (verified, `calib-run-8.log` and `calib-run-9.log`). It has no
  unique catch.
- **Fold-in cost.** There are two paths, with runtime the same either way.
  - *Path A: extend `testing::optrace::Op`* with the five missing kinds,
    and give every `TraceModel` an applier: production, the tree oracle,
    and the function-space oracle.
    - *What it gains.* Every consumer of organic populations then sees
      them, including `laws_hold_on_organic_populations`, the
      differential table's organic drivers, and the function-space
      replay.
    - *The grid.* The function-space grid is derived from fork depth per
      operation, `GRID_N = MAX_TRACE_OPS + 2` (`function.rs:78-80`, which
      says replaying deeper generators requires rederiving it). A
      `forks(8)` adds 3 levels in one operation, so the grid needs
      rederiving (verified by reading; the size of the change is
      inferred).
    - *Cost.* Moderate to high.
  - *Path B: port the driver and the model* as a standalone property.
    That is a parallel harness, which the audit's rules disfavor.
  - *Runtime.* 0.55 s at 256 cases (`run2`), and 204 s at 100,000 cases
    under load near 200 (`long-1.log`).
  - *Documentation.* `Op`, its variants, `arb_op`, `m`, and `run` need doc
    comments.
- **Overlaps.**
  - *Committed.* It extends `testing::optrace` rather than duplicating
    it.
  - *Ready branches.* #76's fixed shape reaches one late-overlap path of
    `sync_all` deterministically. This driver never produces an overlap,
    by design.
- **Dependencies.** None.
- **Value, in one sentence.** It is the only instrument that drives the
  multi-party operations through long rule-respecting histories with large
  populations, where the crate page's promise that an error is always
  definitive must hold.

## 4. `join_all` and `sync_all` against the model

- **What it is.** Four proptests in `tests/audit_l1/main.rs`
  (`03213585e`), at 512 cases each:
  - `join_all_disjoint`: 1 to 10 disjoint parties in shuffled order must
    be accepted, and the result must equal their union.
  - `join_all_failure_conserves_multiplicity`: 2 to 10 disjoint parties
    plus one extra at a random position. The extra is an alias of a
    member, an arbitrary party, or an arbitrary party unioned with a
    member.
    - On `Err`, the receiver never shrinks, and pointwise multiplicity is
      conserved.
    - On `Ok`, the family must have been pairwise disjoint.
  - `clock_join_all_conserves`: the same for clocks with versions.
    - On `Ok`, the version is the join of all versions and the party the
      union.
    - On `Err`, multiplicity is conserved, and the versions are conserved
      by join.
  - `sync_all_matches_model`: 1 to 9 disjoint clocks, with an optional
    alias at a position past the receiver.
    - On `Ok`, `sync_all` must deal the model's balanced shares of the
      union in participant order, and every clock must carry the joined
      version.
    - On `Err`, every clock's bytes, the receiver's included, must be
      unchanged.
- **What it reaches or checks.** The census of the failure property's
  inputs (`run2`, measured, 2,000 draws):
  - Production rejects 1,991 of the families. The property's `Ok` arm ran
    for only 9 draws, so `join_all_disjoint` carries the success path.
  - A rejection returns 1 to 10 parties, and 3 or more in 60% of
    rejections.
  - Each call has 2 to 10 inputs, close to uniformly.
- **Coverage beyond the committed suite, at `main`.**
  - *Multiplicity.* M19 and M20 return a rejected group twice and pass
    every committed test (verified, `calib-run-3.log`). #40 and #81 carry
    this check once they land.
  - *Dropped groups.* M11 drops the untested groups after a failure in
    `join_all`'s final loop. One committed test caught it in each of two
    runs, a different test each time: `party_and_list_laws` in
    `calib-run-2.log`, `laws_hold_on_organic_populations` in
    `calib-run-8.log` (verified). So the committed catch is likely
    probabilistic (inferred). This property returns 3 or more groups in
    60% of rejections.
  - *Late overlaps in `sync_all`.* #76 found that on `main`, a `sync_all`
    that changes its receiver on a late overlap passes every committed
    test (reported, ready entry #76). A late overlap is one the merge
    meets only after absorbing earlier groups. `sync_all_matches_model`
    compares every clock's bytes after any `Err`, over families of up to
    10 with the alias at a random position. So it would likely catch
    that mutant (inferred; I ran no mutant).
  - *Exact dealing.* It compares the dealt shares against an independent
    model. The committed law `sync_all_is_join_all_then_forks` compares
    two production spellings with each other, and
    `sync_all_reconciles_one_world` checks only invariants (verified).
  - *Large accepted families* of arbitrary parties (entry 1).
- **Evidence.** It caught M08, M10, M11, M18, M19, M20, and M21
  (reported, `calibration.md`; verified for M19 to M21 in
  `calib-run-3.log`). The committed suite missed M19 and M20 (verified).
- **Fold-in cost.**
  - *What is already carried.* #40 and #81 carry the multiplicity part.
  - *The rest.* The rest becomes registry family laws fed by
    `arb_disjoint` (entry 1), plus an exact-dealing comparison that needs
    the model (entry 2), or the tree oracle's balanced forks of the union.
  - *Runtime.* 0.27, 0.34, 0.91, and 0.96 s at 512 cases (`run2`), and
    184 to 200 s at 100,000 cases under load near 200 (`long-1.log`).
  - *Documentation.* All four tests have doc comments.
- **Overlaps.**
  - *Ready branches.* #40, #81 (multiplicity), and #76 (the
    `sync_all_agrees_with_join_all` law and the late-overlap shape).
  - *Committed.* `clock_join_all_matches_all_models` already catches
    M18's dropped version.
- **Dependencies.** #40, #81, and #76 decide how much stays unique. After
  all three land, what remains is exact dealing against an independent
  model, and large arbitrary families.
- **Value, in one sentence.** On `main` today it is the only check on
  three of `join_all`'s and `sync_all`'s error paths. After the ready
  branches land, it remains an independent check of `sync_all`'s dealing
  over families the committed generators cannot build.

## 5. Set algebra, fork, splits, and sync against the model

- **What it is.** Ten proptests in `tests/audit_l1/main.rs`
  (`03213585e`), at 512 cases each, with the helpers `check_pair` and
  `check_sync`:
  - *Pairs* (`set_algebra_arbitrary`, `set_algebra_disjoint`,
    `set_algebra_nested`, through `check_pair`):
    - `is_disjoint`, `covers`, and the `shape` regions agree with the
      model;
    - `without` returns `None` exactly when the difference is empty, and
      otherwise the difference;
    - `join` succeeds exactly on disjoint pairs, and on `Err` hands back
      the operand and leaves the receiver byte-identical.
  - *Single parties:* `model_roundtrip` and `fork_matches_model`.
  - *The fork iterator* (`forks_partial_drain`): counts up to 300, every
    prefix length, an exact `size_hint` at each step, the shares, and the
    residual.
  - *Array splits* (`array_split_matches_model`): `N` in 1 to 9, 13, 16,
    and 17, for both `Party` and `Clock`.
  - *Clock forks* (`clock_forks_match_party_forks`).
  - *Sync* (`sync_arbitrary`, `sync_disjoint`, through `check_sync`):
    `Ok` exactly on disjoint pairs, matching the model's fork of the union
    with joined versions; on `Err`, both clocks byte-identical.
- **What it reaches or checks.** The model is the oracle for every
  value. The operands come from entry 1, and random pairs overlap 85% of
  the time.
- **Coverage beyond the committed suite.**
  - *Depth.* The predicates mostly exist in committed laws: for example
    `join_defined_iff_disjoint`, `join_overlap_hands_back`, and
    `without_characterization`. The differential table checks `covers`,
    `is_disjoint`, and `without` against two oracles, but at depth 4. What
    this group adds is depth 80, with an independent oracle.
  - *Fork shares.* The committed share-identity check,
    `compact_plan_matches_recursive_forking`, takes counts up to 64 and
    compares against a recursion over production `fork`. The committed
    partial-drop checks compare only conservation (verified, reading
    `party/forks/tests.rs`).
  - *Array splits.* The committed laws `forks_matches_from_array` and
    `clock_forks_matches_from_array` check only `N = 4`, by comparing the
    two production forms with each other (verified).
  - *Sync atomicity.* Committed `Clock::sync` rejection checks use a
    whole-clock alias (`sync_is_join_then_fork`) or the party-level sync
    at depth 4. Byte-identical clocks after a *partial* overlap at the
    `Clock` level appear only here (inferred, from the committed tests I
    read).
- **Evidence.** It caught M01, M02, M03, M05, M06, M07, M12, M13, M15,
  M16, M17, and M24 (reported, `calibration.md`). The committed suite
  caught each of them too (verified, `calib-run-8.log` and
  `calib-run-9.log`).
- **Fold-in cost.**
  - *What carries over without the model.* With entry 1's generators in
    `testing::generators`, the committed laws and the differential
    table's tree leg cover the predicates at depth.
  - *What needs the model.* Exact values for `fork`, `forks`, the splits,
    and `sync` need entry 2, or a reference built on production `fork`.
  - *Runtime.* 0.15 to 0.37 s per test at 512 cases (`run2`).
  - *Documentation.* `check_pair`, `check_sync`, and `clock_bytes` need
    doc comments.
- **Overlaps.** It overlaps the committed party laws, the differential
  table's party rows, `party/forks/tests.rs`, and the clock laws. It would
  add a second, independent check beside them, not replace them.
- **Dependencies.** None.
- **Value, in one sentence.** It is the independent, deep check of every
  binary identity operation's exact result; most of its predicates already
  exist at depth 4.

## 6. Wide-count fork properties

- **What it is.** Two proptests in `tests/audit_l1/wide.rs`
  (`1b1c16e72`), at 256 cases each; 109 lines.
  - *Counts.* `arb_wide_k` draws counts near `usize::MAX`, just above
    twice it, at arbitrary widths up to `2^100`, and at powers of two and
    their neighbors.
  - *Party test* (`forks_wide_counts`). For up to 40 shares at
    arbitrary-party depth up to 20, it checks four things:
    - each share equals the model's, computed by one logarithmic descent
      per share with `BigUint` arithmetic;
    - the residual is the original minus the shares taken;
    - the residual covers share 0;
    - every size hint is sound, and exact while the count fits `usize`.
  - *Clock test* (`clock_forks_wide_counts`). It checks up to 6 shares,
    and that each carries the parent's version.
- **What it reaches or checks.** Exact shares at counts beyond `u64`.
- **Coverage beyond the committed suite.** No committed test checks share
  identity at a count wider than `usize` (verified, reading
  `party/forks/tests.rs` and `tests/forks_count.rs`). The committed tests
  stop short of it:
  - `wide_count_prefixes_conserve_the_party` takes at most 4 shares and
    checks only that rejoining restores the party.
  - `adjacent_wide_count_becomes_exact`, the unbounded-count tests in
    `tests/forks_count.rs`, and the wasm32 pin check hints and
    conservation.

  So a wide-count plan that deals unbalanced shares but conserves the
  party passes the committed suite (inferred).
- **Evidence.** It caught M04, an off-by-one at the transition from near
  to exact; the committed `adjacent_wide_count_becomes_exact` caught it too
  (verified, `calib-run-9.log`). It caught no defect. D1 came from reading
  the classification, not from this probe (reported).
- **Fold-in cost.**
  - *Where it goes.* It belongs in `party/forks/tests.rs`, beside
    `wide_count_prefixes_conserve_the_party`.
  - *Reference.* The model's share descent could be replaced by the same
    descent over aliases of production `fork`. That reference is not
    independent, but it needs no model (inferred design).
  - *After #43.* The exactness clause should widen to #43's rule, "exact
    whenever the remainder fits `usize`". The current clause stays valid
    under #43 (inferred: #43's rule is stronger).
  - *Runtime.* 1.47 s and 0.27 s at 256 cases (`run2`). At 100,000 cases
    the party test timed out at 600 s (`long-1.log`). At 10,000 cases it
    passed in 73 s under load near 200 (`long-2.log`). Its cost comes from
    the model's descent.
  - *Documentation.* `count` needs a doc comment.
- **Overlaps.** It overlaps #43's regression tests, which check hints
  through real steps, and the wasm32 pin
  `forks_accept_the_first_count_past_usize`.
- **Dependencies.** #43, for the exactness rule.
- **Value, in one sentence.** It is the only check that fork shares stay
  balanced and exact at counts past the machine word, where the plan runs
  on `BigUint` arithmetic.

## 7. Deep probe on a 256 KiB stack

- **What it is.** One test, `deep_identity_operations_are_iterative`, in
  `tests/audit_l1/deep.rs` (`1b1c16e72`; left-spine shapes in
  `f3456ae74`); 224 lines. It runs on a thread spawned with
  `std::thread::Builder::stack_size(256 KiB)`, at depth 100,000.
  - *Operands:* right and left cells, their complements (combs), and a
    *zigzag*, a unary spine that alternates left and right at every level.
  - *Party operations:* `covers` and `is_disjoint` with both outcomes; and
    `without`, including `comb \ zigzag`, which must build the complement
    along the zigzag's path.
  - *Joins and syncs:* `join` collapsing 100,000 levels to the seed, on
    both spines; and `sync`.
  - *Fork iterators:* `forks` fully drained, partially dropped, and with
    a count above `u128::MAX`; the `[Party; 5]` split; and `join_all`.
  - *Clock operations:* `Clock::forks`, `sync_all`, `join_all`, `recv` of
    a deep version, and every shape traversal.
  - *Other:* `Hash`, `Debug`, and the codec.
- **What it reaches or checks.** The stack leaves 2.6 bytes per level,
  against a minimum call frame of 16 bytes. So any traversal that keeps a
  frame per level overflows.
- **Coverage beyond the committed suite and #75.** #75 drives almost every
  entry point here at depth `2^18`, on left and right spines and their
  combs (reported, ready entry #75). This probe adds two things:
  - *The zigzag at depth.* No committed or ready test builds a zigzag
    party at stack-safety depth (verified: `git grep -i zigzag` on `main`
    and on #75's branch finds only the tick tests' `shape_party`, up to
    depth 128). A traversal that loops along runs in one direction but
    recurses where the direction changes would keep no frames on spines
    or combs, and one frame per level here (inferred). #34 found the
    one-sided form of this class: a mutant that loops down left runs and
    recurses into right subtrees passed the whole suite until a right
    spine was added (reported, ready entry #34). I did not check whether
    any board family alternates directions at depth.
  - *A stack bound the environment cannot loosen.* #34's doc says its
    bound "rests on libtest's default thread stack; a larger
    `RUST_MIN_STACK` weakens it" (verified, branch diff). This probe sets
    its stack explicitly, which `RUST_MIN_STACK` does not override
    (inferred, from the standard library's documentation of
    `Builder::stack_size`).
- **Evidence.**
  - M22, a recursive `skip`, was caught by the committed suite too
    (reported).
  - M26 was caught here, and also by five `amp_board_smoke` tests at the
    base (reported, ready entry #75).
  - M25, a tail-recursive region descent, is caught by nothing at
    `opt-level = 2` (verified, `calib-run-6.log` and `calib-run-7.log`).
  - It caught no unique mutant.
- **Fold-in cost.**
  - *Where it goes.* Add a zigzag builder beside `deep_left_spine_party`
    and `deep_right_spine_party` in `testing::generators`. Add the zigzag
    to the loop in #75's `deep_identity_stack_safety`, and `comb \ zigzag`
    to its `without` checks.
  - *The explicit stack.* Moving #75's tests onto an explicit-stack thread
    is a separate choice for the owner.
  - *Runtime and dependencies.* 0.77 s (`run2`). No new dependency.
- **Overlaps.** It overlaps #75 and #34 almost entirely.
- **Dependencies.** #34 and #75, which must land first.
- **Value, in one sentence.** Small: its one unduplicated regime is an
  alternating-direction spine at depth, which takes a few lines to add to
  #75.

## 8. Scan-cost probe

- **What it is.** One printing test, `cost_report`, in
  `tests/audit_l1_cost.rs` (`26cbae227`); 162 lines. It reads the scan-bit
  meter (`before::testing::meter`) at d = 1,000, 4,000, 16,000, and
  64,000, and divides by each operation's documented bound. It asserts
  nothing.
- **What it reaches or checks.** Its readings (`cost-1.log`, reproduced
  exactly by `run2`):

  | Family | Denominator | Reading at all four sizes |
  |---|---|---|
  | `without(comb, zigzag comb)`, and the reverse | input bits | 1.25 |
  | `covers`, both ways | input bits | 0.50 |
  | `is_disjoint(comb, zigzag comb)` | input bits | 0.00, an immediate exit (correction 4) |
  | `sync(comb, its cell)` | input bits | 2.99 to 3.00 |
  | `sync(cell, zigzag minus cell)` | input bits | 3.00 |
  | `forks(64)` full drain, deep unary party | `k(\|p\| + log k)` bits | 4.92 |
  | `join_all` of those 65 shares | `(\|self\| + \|iter\|) log k` bits | 0.82 |
  | `forks(2^d).next()`, `\|p\|` = 2,008 bits | `\|p\| + log k` bits | 5.32 to 5.97; absolute about 10,000 + 6d, linear |
  | `shape` drain of the zigzag comb | input bits | 1.00 |

- **Coverage beyond the committed suite.**
  - *`without` with a deep arbitrary receiver.* The board meters
    `without` only as `Party::seed().without(&b)` and `a.without(&a)`
    (verified, `testing/meter/board/ops.rs:2221-2234` and `:2943-2958`).
    This probe meters it with a deep arbitrary receiver, in both orders.
  - *Count width apart from party size.* The board's fork count is as
    wide as the party (reported, the owner's ruling on D1). This probe
    holds the party at 2,008 bits while the count grows to `2^64000`.
  - *Blind spot.* `BigUint` work in the fork plan is invisible to the scan
    meter (reported, `coverage.md`); the survey's item 1.1 concerns
    exactly that.
- **Evidence.** No growth at any of the four sizes, so it found no defect.
  It has no mutants.
- **Fold-in cost.**
  - *Where it goes.* Two board families: a `party_without` cell with a
    deep arbitrary receiver (the comb against the zigzag comb, both
    orders), and a fork-step cell whose count width is independent of
    party size.
  - *Ceilings.* Each needs a ceiling under the owner's one-rule ruling
    (question 65, slot 26).
  - *Floors.* Each needs a strictly positive floor. The `is_disjoint` row
    shows why: it reads zero at every size, so a ceiling over it passes
    while it measures nothing. A pessimal `is_disjoint` needs disjoint operands that share the long
    path, such as a comb and its cell, as the `sync` row already uses.
  - *Runtime and dependencies.* 0.98 s (`run2`). No new dependency.
- **Overlaps.** It overlaps the board's `party_without`, `party_forks`,
  `party_forks_full`, `party_join_all`, and `clock_sync` rows, and the
  survey's item 1.1.
- **Dependencies.** Slot 26's ceiling rule, for any new board family.
- **Value, in one sentence.** It supplies the two pessimal families the
  board lacks for identity, `without` with a deep receiver and a fork
  step dominated by count width, ready to become board rows.

## 9. Heap-retention probe

- **What it is.** One printing test, `retention_report`, in
  `tests/audit_l1_retention.rs` (`8074f0985`); 163 lines.
  - *Method.* A counting global allocator (`peak_alloc`, already a dev
    dependency) measures the live heap a result holds after its inputs
    are dropped.
  - *Depths.* d = 100, 1,000, 10,000, and 100,000.
  - *It asserts nothing.*
- **What it reaches or checks.** Every result is 1 byte (`retention-6.log`,
  reproduced exactly by `run2`). Retained heap at d = 100, 1,000, 10,000,
  and 100,000:

  | Operation | 100 | 1,000 | 10,000 | 100,000 |
  |---|---:|---:|---:|---:|
  | `join`, comb with its cell | 100 B | 775 B | 7,525 B | 75,025 B |
  | `Clock::join`, same operands | 100 B | 775 B | 7,525 B | 75,025 B |
  | `join_all` of a comb's forks, then `join` | 100 B | 775 B | 7,525 B | 75,025 B |
  | `without`, half and cell minus cell | 75 B | 525 B | 5,025 B | 50,025 B |
  | `forks(1)` residual | 50 B | 275 B | 2,525 B | 25,025 B |
  | party from `Clock::decode`, version dropped | 64 B | 401 B | 3,776 B | 37,526 B |
  | `fork` (control) | 1 B | 1 B | 1 B | 1 B |

- **Coverage beyond the committed suite.** The only committed retention
  check is `asymmetric_fork_sizes_the_small_result_independently`, which
  covers `fork` alone through `allocation_capacity()`
  (`party/tests.rs:133-144`, verified). The probe measures six more
  operations. S1's planned tests cover three of them: `join`, `without`,
  and the `forks(1)` residual. No planned test covers the party from a
  decoded clock (observation O1), a design trade nobody has briefed.
- **Evidence.** It found S1 and O1, both constant factors with no breach
  of a resource bound (reported). The owner ruled S1 as a declared trade,
  variant B (question 73).
- **Fold-in cost.**
  - *As assertions on capacity.* S1's planned tests read
    `allocation_capacity()`, which needs no allocator. That form avoids
    the race the survey's addendum found: libtest's main thread allocates
    inside a measurement window (reported).
  - *Through the allocator.* This needs the shared counting allocator
    with per-thread attribution (question 91).
  - *Cost.* Small for the capacity form. Runtime 0.16 s (`run2`).
- **Overlaps.** It overlaps slot 33's planned retention tests (S1, variant
  B) and the committed `fork` capacity test. The board's heap meter
  measures peaks, not what a result retains afterwards.
- **Dependencies.** Slot 33, ruled under question 73 and not yet built,
  and question 91.
- **Value, in one sentence.** It is the measurement behind S1 and the
  only record of O1. Folded in as capacity assertions, it would keep
  every small identity result from holding its inputs' buffers.

## 10. Overlay refinement property

- **What it is.** One proptest, `overlay_is_the_refinement`, in
  `tests/audit_l1/overlay.rs` (`1b1c16e72`); 77 lines, 512 cases.
  - *The check.* `Clock::shape` must equal the coarsest common refinement
    of the model's party regions and the version's plateaus, with each
    plateau's rise carried only on its first cell.
  - *Inputs.* An `arb_set` party, and a version built by ticking 0 to 4
    arbitrary parties.
- **What it reaches or checks.** The census of 2,000 draws (`run2`,
  measured):
  - *Party depth:* above 4 in 78%, above 32 in 47%.
  - *Version depth:* above 4 in 11.75%, above 32 in 2.6%; 27% of versions
    are empty.
  - *Cells per clock:* more than 16 in 65%, more than 64 in 30%.
  - *Deepest cell:* deeper than 4 in 81% of clocks.

  The party side is independent of production. The version side trusts
  production `Version::shape` (verified, reading `overlay.rs`).
- **Coverage beyond the committed suite.** The committed
  `clock_shape_matches_the_oracle` compares `(depth, absolute height,
  owned)` rows against the tree oracle's refinement
  (`testing/diff_ops.rs:452-470`, `shape_rows::fold_overlay`, verified).
  Its arbitrary operands stay at depth 4 or less, so at most 16 cells.
  This property adds depth, and it compares the `rise` field itself rather
  than heights folded from it.
- **Evidence.** It caught M23; the committed suite caught it too
  (`clock_solo_ops` and the organic differential, verified,
  `calib-run-9.log`). It has no unique catch.
- **Fold-in cost.**
  - *Independence.* Replace production `Version::shape` with the tree
    oracle's `shape_rows::oracle_plateaus`, which recurses but is safe at
    these depths, or with the algebra lane's leaf-list model.
  - *Where it goes.* It then becomes a deeper-operand driver for the
    committed differential row, not a parallel test.
  - *Runtime.* 0.29 s (`run2`).
- **Overlaps.** The committed `clock_shape_matches_the_oracle`, and the
  algebra lane's version model.
- **Dependencies.** None.
- **Value, in one sentence.** Modest: its versions are mostly shallow, so
  most of its added depth comes from the party side, which entry 1 would
  already feed into the committed differential row.

## 11. Generator statistics printers

- **What it is.** Two printing tests that compute the same statistics:
  - `generator_stats` in `tests/audit_l1/main.rs` (`03213585e`), about
    100 lines. It covers `arb_set`, pair relations, `arb_disjoint(4)`, and
    the history driver's final populations.
  - `explore_committed_generator_stats` in `src/party/tests.rs`
    (`c54c195c1`), 108 lines. It covers `arb_oracle_party_nonempty`, pair
    relations, how often `arb_party_family` is accepted, and the parties
    of `optrace` populations.

  Both report depth, encoded bytes, two-child branch count, and longest
  unary chain, as histograms.
- **What it reaches or checks.** Measurement only; the logs are
  `stats-1.log` and `stats-committed.log`.
- **Coverage beyond the committed suite.** The adequacy lane's census,
  `l8_census.rs`, measures depth, leaves, owned leaves, root kind, size,
  relation mix, and family acceptance by arity (verified, its module
  header). It does not count two-child branches or unary chains. Those
  two metrics are the ones that separate the explore generators from the
  committed ones most sharply (entry 1).
- **Evidence.** It produced observation O8 and the reach figures in entry
  1. It always passes.
- **Fold-in cost.**
  - *Into the census.* Add the two metrics to the adequacy lane's census.
  - *As reach floors.* Commit them as floors: assertions that a committed
    generator keeps reaching, for example, depth above 32 in some fraction
    of draws. The survey cut these floors under its old rule.
  - *Runtime.* 2.8 s and 2.6 s (`run2`, `stats-committed.log`).
- **Overlaps.** The adequacy lane's census.
- **Dependencies.** None.
- **Value, in one sentence.** Useful only if the owner adopts reach floors
  for generators. There, its two extra metrics would fail a generator
  change that stops producing deep two-child branches or long unary
  chains, which no committed test measures today.

## 12. Mutant lists and calibration driver

- **What it is.** Scratch tooling, not on any branch.
  - *The mutants.* `<scratch>/auditor-l1/muts1.py` to `muts4.py` hold 24
    mutants, M01 to M26 without M09 and M14, each an exact source string
    swap with an old and a new text.
  - *The driver.* `calibrate.py` applies one swap, runs a nextest
    selection on the box, restores the file, records which tests failed,
    and prints `git diff --stat` to prove the restoration. It skips a swap
    whose old text does not occur exactly once.
- **What it reaches or checks.** Calibration of any test selection
  against hand-written identity mutants:
  - error-path duplication (M19, M20) and dropping (M08, M11, M21);
  - recursive rewrites of iterative traversals (M22, M25, M26);
  - swaps of ceiling and floor, halves, and tags (M05, M15, M17, M24);
  - an off-by-one at a width boundary (M04);
  - an overlay depth swapped (M23).
- **Coverage beyond the committed suite.** Nothing committed calibrates
  identity tests. The adequacy lane's cargo-mutants campaign is the
  general instrument: 282 survivors of 3,919 mutants (reported, survey
  section 2.3). The semantic rewrites here, such as returning a rejected
  group twice or turning a loop into recursion, are not among cargo-mutants'
  usual mutation operators (inferred).
- **Evidence.** It backed MB1, MB2, and MB3. The identity source is
  unchanged from `58285ca5` to `main` (verified), so every swap should
  still apply (inferred; the driver would report a skip otherwise).
- **Fold-in cost.** The project has no mechanism that keeps known-bad
  variants and checks that they still fail (inferred). Folding these in
  means building one, for example a scripted check outside nextest. The
  cost is moderate to high. The algebra lane kept 20 mutant patches on its
  branch, a similar asset (reported, ranker handoff).
- **Overlaps.** The adequacy lane's survivor index and the algebra lane's
  mutant patches.
- **Dependencies.** None.
- **Value, in one sentence.** It is a ready-made set of known-bad
  implementations for identity, valuable only if the owner wants the
  committed suite's ability to fail demonstrated as an ongoing check.

## 13. Deep-party builders

- **What it is.** Fixtures that pack two-bit tag streams straight into
  canonical bytes and decode them through the public `Party::decode`. All
  are iterative.
  - In `deep.rs`: `pack`, `right_cell`, `left_cell`, `right_comb`,
    `left_comb`, and `zigzag`.
  - In `audit_l1_cost.rs`: `zigzag_comb`, a two-child comb that
    alternates sides.
  - In `audit_l1_retention.rs`: `half_and_cell`, and the
    `[0, 1/4) ∪ deep cell` builder inline.

  `pack` and the cell and comb builders are copied across the three
  files.
- **What it reaches or checks.** It builds inputs only.
- **Coverage beyond the committed suite.** The committed suite has
  `deep_left_spine_party`, #34's `deep_right_spine_party`, and the
  `constructed` module in `party/tests.rs`: `leftmost`, `spine`, `node`,
  `full`, and complements (verified). The new shapes are the alternating
  spine, the alternating two-child comb, and the shallow-plus-deep parties
  the retention probe needs.
- **Evidence.** Inputs to entries 7 to 9.
- **Fold-in cost.**
  - *Where it goes.* One builder per new shape beside
    `deep_left_spine_party`, writing bits as that function does. Small.
  - *Who needs it.* Only an entry that is itself folded in needs these.
- **Overlaps.** The committed deep builders.
- **Dependencies.** #34, for the right-spine sibling.
- **Value, in one sentence.** It is worth taking only together with entry
  7, 8, or 9.

## 14. Unoptimized-build deep-test configuration (MB3)

- **What it is.** A nextest invocation, never committed. It builds
  `before` and `suanpan` at `opt-level = 0` through `--config`, in its own
  target directory, filtered to the deep and stack-safety tests. The logs
  are `run-opt0-deep.log`, `run-m25-opt0.log`, and
  `run-m25-opt0-existing.log`.
- **What it reaches or checks.** Stack safety without LLVM's tail-call
  elimination. The workspace builds `before` at `opt-level = 2` even in
  the dev profile (verified), and that optimization turns a self tail call
  into a loop.
- **Coverage beyond the committed suite.** M25 passes everything at the
  workspace profile (verified, `calib-run-6.log`). At `opt-level = 0`, both
  the committed `deep_tree_query_and_causal_stack_safety` and the explore
  probe overflow on it (reported, MB3).
- **Evidence.** It caught M25. It found no defect: the unmutated tree
  passes all 34 deep tests at `opt-level = 0` (reported).
- **Fold-in cost.** About 59 s of cold build and 7 s of run (reported),
  plus one more build directory.
- **Overlaps.** The committed deep tests, run under a second
  configuration.
- **Dependencies.** None.
- **Value, in one sentence.** The owner already ruled against a check leg
  for it (question 68), so it is catalogued only because it was built.
