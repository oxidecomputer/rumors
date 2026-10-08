<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the events-lane (L3) cataloguer of the instrument rescue, from the explore branch, the lane's records and scratch files, ready entry #74, and two runs on ox-east-1; for Finch's review. -->

# 03. Instruments the events lane (L3) built

## Lane summary

**Source.** The events lane's instruments live on `explore/l3-events` at
`873f39991`, five signed commits on the audit base `58285ca51` (verified,
`git log`). The branch adds two test modules under `version::tick`, so
they can call the walk's internal `TickWalk::decide`:

- `crates/before/src/version/tick/l3_probe.rs`, 1,156 lines: co-generated
  tick pairs, the tick claim set, `min_ticks` over histories, deep spines,
  and four diagnostics.
- `crates/before/src/version/tick/l3_heap.rs`, 284 lines: release-profile
  peak-heap probes under a `PeakAlloc` global allocator.

The lane's calibration set, twenty runtime-switched mutants, is on no
branch. It exists only as patch files in the auditor's scratch directory
(`<scratchpad>/auditor-l3/`), which lives under `/private/tmp` and does
not survive a reboot (verified, file list; README, "Files").

**What ready entry #74 carries.** `audit/events-cogen-and-tidy` (tip
`183a094a4`) rebuilds the lane's co-generator as
`testing::generators::co_generated` (verified, by reading both). Each
carried piece gets one line:

- The region co-generator and its palette: `co_generated.rs`
  (`Region`, `VersionSkeleton`, `arb_palette`, `resolve`). The palette
  draws two to five heights rather than one to five, because a one-height
  palette collapses every drawn version to a single leaf.
- The spine strategy: `arb_spine_case`, driven by
  `co_generated_spines_tick_identically` (replaces `l3_cogen_spine` and
  `l3_family_spine`).
- The wide strategy: `arb_wide_case`, driven by
  `wide_pre_scans_tick_identically` at one sixth of the default case
  count (replaces `l3_family_wide`).
- The multi-scan strategy: `arb_multi_scan_case`, driven by
  `multiple_wide_pre_scans_tick_identically` at one twelfth (replaces
  `l3_family_multi_wide`).
- The oracle claims of `check_one`: the changed flag, the fill output,
  canonicality, and the oracle's event, through the committed
  `assert_tick`; `ticks(0)`, `ticks(1..=4)` against iterated ticks and the
  oracle, and composition at four wide counts, through
  `assert_co_generated` (`version/tick/tests.rs`).
- The fill-fixed successor and one late perturbation:
  `TickCase::versions`.
- Witnesses for the two value mutants M6 and M8: two literal cases, from
  #74's own shrinks rather than the lane's.
- The site metrics of the two reach diagnostics, as enforced floors:
  `co_generated_strategies_keep_mass_on_their_regimes`.

**What #74 left behind.** Everything else on the branch, catalogued in
full below:

| Explore instrument | On #74? |
|---|---|
| `check_one`'s strict domination, output envelope, region locality, and `min_ticks` step bounds | No (entry 3) |
| The second late perturbation in `check_family` | No (entry 11) |
| The bushy strategy as a property of its own | No; its region builder survives inside the spine and wide strategies (entry 10) |
| `l3_deep_spine` | No (entry 4) |
| `l3_min_ticks_floor_over_histories`, `l3_min_ticks_tight`, `l3_min_ticks_tight_spine` (machinery brief MB-2) | No; never built (entries 1, 2) |
| The heap probes and the two random searches | No (entries 5, 7, 8, 9, 12) |
| The histogram and reach printers, with their row for the committed generator | Only the site metrics, as floors (entries 13, 14) |
| `l3_bridge_round_trip` | No (entry 15) |
| The mutant set | No (entry 6) |

**Corrections to the lane's records.** Four things the records say are
wrong or incomplete, and one of them has propagated into the baseline:

1. **A committed floor over histories exists.** The coverage map, machinery
   brief MB-2, and `00-baseline.md` section 4.1 all say no committed test
   states `min_ticks` as a floor over histories. The committed proptest
   `min_ticks_floors_every_history` (`crates/before/src/version/tests.rs:682`
   on `main`) does, with a weaker bound: every final clock's `min_ticks` is
   at most the whole history's total ticks (verified, source). The lane's
   floor property is therefore a strengthening, not a first statement
   (entry 2).
2. **The calibration's "committed" set included lane probes.** The filter
   `test(/version::tick::/) - test(/l3_/) | test(/min_ticks/) | test(/ticks/)`
   re-admits every probe whose name contains `min_ticks`. Four were on the
   branch at calibration time, so the committed set was 68 tests, not 72,
   which matches #74's builder's count (verified, `calib-batch2.log`: under
   M16, `l3_min_ticks_floor_over_histories` fails as test 22 of 72). The
   committed-failure counts under M16, M22, and M23 include one, three,
   and two lane probes (verified). Under M3, M7, M11, and M12 they may too, since
   the inventory says the floor property failed there (inferred). The
   escape list (M6, M8, M15, M19) is unaffected, since 72 of 72 passing
   implies 68 of 68.
3. **The calibration set is not durable** (see above, and entry 6).
4. **The `2^16` control is not green at every size.** The defect record says
   the jump-entered heap breach "disappears" at a jump of `2^16`. At 98,304
   levels that control reads 26.27 bytes per input byte, red against the
   board's ceiling, from the plain spine's doubling mechanism (verified,
   `run-heap-release-3.log`).

**Box runs.** Two of the three allowed, on the scratch worktree
`/Users/oxide/src/rumors-rescue-l3` at `873f39991` plus one uncommitted
census module, `version/tick/l3_rescue_census.rs`:

- Run 1: the debug `--no-run` build, exit 0.
- Run 2: serial nextest at the default case count, over every explore
  property #74 did not carry, the spine pair for comparison, both
  diagnostics, and the census. 12 of 12 passed, exit 0, 61.3 seconds in
  total (`<scratchpad>/rescue-l3/run2-tests.log`). The box's load average
  was about 20 when the runs began, and the explore branch's lib test
  binary installs `PeakAlloc` as its global allocator, so every reading
  includes that allocator's overhead.

The census module replays both the lane's history generator and the
committed trace generator through one causal-past ledger. Its source is
kept in `<scratchpad>/rescue-l3/l3_rescue_census.rs`.

**Marks.** Every claim is *verified* (I checked it against the tree, a log,
or my own run), *reported* (a record says so; the record is named), or
*inferred*. "Committed" means `main` at `ce67ab083`, plus the ready
branches where an entry says so. Runtimes are debug builds on ox-east-1
under shared load, judged against nextest's 300-second limit on `main`.

## Entries, by value

### 1. `min_ticks` attained by a constructed history

- **What it is.** Two proptests and two helpers in
  `explore/l3-events:crates/before/src/version/tick/l3_probe.rs:768-847`,
  added at `30379f8d8` and unchanged at the tip; about 80 lines.
  `with_path_party` forks a seed party down a dyadic path, parks the halves
  it leaves, and joins them back afterwards. `realize` walks a normal-form
  version in preorder and, for each node with a nonzero base `b`, calls
  `ticks(b)` with a party owning exactly that node's interval.
  `l3_min_ticks_tight` draws the version from the committed
  `arb_oracle_version`, and `l3_min_ticks_tight_spine` from the lane's
  spine strategy.
- **What it reaches or checks.** Three assertions per case: the realized
  version equals the drawn one; `min_ticks` equals the ticks spent; the
  seed reassembles (verified, source).
  - The construction is its own reference. It shares no code with any
    `min_ticks` implementation, but it calls production's `ticks`, `fork`,
    and `join`. The ticks it spends equal the normal form's base sum by
    construction, which is exactly what the recursive oracle computes, so
    the count assertion alone repeats the oracle's definition (inferred).
    What the oracle cannot supply is the first assertion: the base sum is
    achievable by a real history.
  - The spine variant draws versions up to about 50 levels deep, from up
    to 39 spine levels plus the side regions and tip (inferred from the
    construction). Its heights reach about `2^201`: the palette's widest
    arm is `m·2^s` with `m < 4` and `s < 200`.
  - Each history is organic: one seed, forks, wide `ticks`, joins. Its
    parties are as deep as the version.
- **Coverage beyond the committed suite.** Two predicates no committed test
  states:
  - *Attainment for versions with more than one region.* The committed
    attainment checks are the law `ticks_line_realizes_min_ticks`, which
    ticks one party from the empty version, `min_ticks_known_values`
    (three fixed values), and `no_maximum_tick_count` (leaf 1) (verified,
    source).
  - *Reachability.* Every sampled canonical version is reached by a
    rule-respecting, single-seed history. The audit's contract model
    separates valid values some rule-respecting history reaches from
    those none reaches (README, "Contract model"). For versions alone,
    this probe places every sampled canonical version on the reachable
    side; no committed test asks.

  The histories also go where committed organic traces never do. Committed
  traces reach version depth at most 6 and heights under 64 bits
  (`00-baseline.md` section 2.4). This construction reaches every drawn
  arbitrary version (depth 4, bases up to 514 bits) and spine versions
  near depth 50 with heights near `2^201`.
- **Evidence.**
  - Under M22 (`min_ticks` reports one more) and M23 (one fewer), both
    properties fail (verified, `calib-batch2.log`). The committed suite
    also catches both shifts, through `min_ticks_known_values`,
    `min_ticks_floors_every_history`, `div_can_fragment_and_raise_min_ticks`,
    and `deep_tree_min_ticks_stack_safety` (verified, same log).
  - M7, M11, and M12 also fail it (reported, `instruments-inventory.md`).
  - 100,000 unmutated cases each passed (reported,
    `addendum-long-runs.md`, run A).
  - It caught no defect. The class it exists for, one definition error
    shared by production and both oracles, was never constructed
    (reported, survey section 2.1).
- **Fold-in cost.**
  - Home: beside `min_ticks_floors_every_history` in `version/tests.rs`,
    over the committed `arb_oracle_version` and, after #74 lands,
    `co_generated::arb_spine_case`'s versions.
  - `realize` recurses on version depth. That depth is bounded by the
    generator, so under ruling 68 it recurses directly and states its
    bound where it recurses.
  - Machinery brief MB-2 already states the validity argument the doc
    comment needs: each base lands where the version is constant, so fill
    is the identity and grow raises exactly that interval.
  - Runtime at the default 256 cases: 0.16 seconds for the arbitrary
    versions and 1.60 for the spine versions (verified, run 2). No new
    dependencies.
- **Overlaps.** It generalizes `ticks_line_realizes_min_ticks` and stands
  beside the two `min_ticks` differentials as a second, independent check
  of the definition. No other lane built a `min_ticks` attainment check
  (verified, grep of every lane's records).
- **Dependencies.** None for the arbitrary-version property; the spine
  variant's generator comes from #74 if it lands first.
- **Value.** It is the only check that `min_ticks` is attained on versions
  with more than one region, and the only demonstration that sampled
  canonical versions are reachable by rule-respecting histories, at small
  cost.

### 2. `min_ticks` bounded by each clock's causal past

- **What it is.** One proptest over a history model:
  `l3_min_ticks_floor_over_histories`, plus `Tracked`, the eight-operation
  `HOp`, `arb_hop`, and `run_history`. They live in
  `explore/l3-events:crates/before/src/version/tick/l3_probe.rs:378-541`,
  at `30379f8d8`; about 165 lines.
- **What it reaches or checks.**
  - The model: each clock carries a ledger of the tick batches in its
    causal past, mapping a batch id to its weight. Fork copies the ledger.
    A send adds a fresh batch to the sender, then the receiver takes the
    union plus a fresh batch. Absorb takes the union. Sync and join take
    the union. A *foreign tick* has another clock's party tick a copy of
    this clock's version, which this clock then absorbs; it adds one
    fresh batch.
  - The foreign tick is not a `Clock` operation. It calls `Version::tick`
    with a party on a version that party's clock never held, so one
    party's events need not be ordered. The floor still holds there,
    because the ledger counts every tick (inferred). A fold-in should
    decide whether that step belongs in a committed world.
  - The predicate: after every step, every live clock's `min_ticks` is at
    most the total weight in its own ledger (verified, source). The ledger
    is the reference, independent of all `min_ticks` code.
  - Inputs: up to 59 operations, including `ticks` counts of `2^s + l`
    with `s < 80` and `l < 4`, absorb, and foreign ticks.
  - The census, 2,000 histories from proptest's deterministic runner,
    compares the two generators (verified, run 2):

    | | Lane histories | Committed traces |
    |---|---|---|
    | Mean operations | 29.9 | 14.3 |
    | Final population, mean and maximum | 5.3 and 19 | 2.5 and 12 |
    | Histories with a count of at least `2^64` | 47.3% | 0 |
    | Histories with an absorb or a foreign tick | 92.3% | 0 |
    | Floor violations | 0 | 0 |
- **Coverage beyond the committed suite.** The committed
  `min_ticks_floors_every_history` differs in four ways:
  - Its bound is the whole history's total ticks, not the clock's own past.
  - It checks the final clocks only.
  - It uses `world_strategy`: under 30 operations, `ticks` counts 0 to 6,
    and no absorb (verified, source).
  - Its heights never reach 64 bits (`00-baseline.md` section 2.4).

  A definition error that overcounts by one on some class of versions
  fails a bound only at an observation where that bound is tight. The
  census counts, over (step, clock) observations with a nonzero
  `min_ticks`, how often each bound is tight (verified, run 2):

  | | Lane histories | Committed traces |
  |---|---|---|
  | Causal past smaller than the history's total | 87.9% | 55.7% |
  | Causal bound tight, every step | 37.9% | 61.5% |
  | Total bound tight, every step | 7.9% | 29.2% |
  | Total bound tight, final clocks only | 3.5% | 17.9% |
  | Histories with a causal-tight observation at any step | 97.2% | 91.5% |
  | Histories with a total-tight final observation | 14.2% | 32.2% |

  The committed test, as written, is the last row's right-hand cell: 32.2%
  of its histories offer even one tight observation. Per history, the
  numbers come out as follows (derived from the census counts):

  | Check | Tight observations per history |
  |---|---|
  | The committed test as written | about 0.42 |
  | The causal every-step check, on committed traces | about 17.8 |
  | The causal every-step check, on the lane's histories | about 44.6 |

  For an overcount confined to a rare class of versions, that is roughly
  40 to 100 times as many chances to see it.
- **Evidence.**
  - Under M22 it fails with "min_ticks 3 exceeds the 2 ticks in the causal
    past after Send(0, 0)" (reported, machinery brief MB-2; the failure
    verified in `calib-batch2.log`). The committed floor test also fails
    under M22 (verified, same log).
  - Under M23 it correctly passes, since an undercount is still a floor.
  - It also fails incidentally under M3, M7, M11, M12, and M16 (reported,
    inventory; M16 verified).
  - 100,000 unmutated cases passed (reported, addendum, run A).
  - It caught no defect.
- **Fold-in cost.**
  - The natural home is `min_ticks_floors_every_history` itself: replace
    the total with a per-clock ledger over `world_strategy`, checked after
    every step. That is a small change to one test.
  - The wide counts, absorb, and foreign ticks cannot join
    `optrace::Op`, because the recursive oracle's `ticks` applier iterates
    the count literally (verified, `optrace.rs`: "small `n`: the oracle
    applier iterates it literally"). They need a production-only world
    beside it, as the lane wrote.
  - Runtime at 256 cases: 0.59 seconds (verified, run 2). No new
    dependencies.
- **Overlaps.**
  - It would replace the committed test's total bound, or stand beside it.
  - The identity lane's history world (`tests/audit_l1/history.rs` on
    `explore/l1-identity`) extends the same trace vocabulary in another
    direction: `forks(k)`, `join_all`, `sync_all`, and byte round trips,
    checked against an interval model of parties, with no tick ledger
    (reported, `lanes/l1-identity/round-1/inventory.md`). A fold-in could
    merge the two vocabularies into one production-only world.
- **Dependencies.** None.
- **Value.** It tightens the committed floor from the whole history's
  total to each clock's own causal past. On the committed generator
  itself, that gives about 40 times as many tight observations per
  history. It works over histories with wide counts, absorbs, and larger
  populations than committed traces reach, in under a second.

### 3. The tick claims #74 did not carry

- **What it is.** Five assertions in `check_one`
  (`explore/l3-events:crates/before/src/version/tick/l3_probe.rs:234-278`
  and `333-337`, at `30379f8d8`), run on every co-generated case:
  - strict domination;
  - the committed output envelope, `2|e| + 4|i| + 32` bits;
  - region locality, checked as an unchanged projection onto the party's
    complement;
  - `min_ticks` rises by at most one per tick;
  - `min_ticks` rises by at most `a + b` under `ticks(a + b)`, for `a` and
    `b` in `{1, 2^64 − 1, 2^64 + 3, 2^200 − 1}`.
- **What it reaches or checks.** These predicates, over the spine, wide,
  and multi-scan regimes. The spine strategy nests lookahead sites 7 or
  more deep in 42% of draws and puts two sites in one range in 25%. The
  wide strategy reserves more than 64 memo slots in one pre-scan in 66%.
  The committed independent pairs have no site in 81% of draws and never
  two in one range (verified, `run-hist-1.log`, `run-diag-release-1.log`).
- **Coverage beyond the committed suite.**
  - Strict domination and locality are laws (`tick_strictly_advances`,
    `tick_only_inflates_the_region`). The envelope is
    `tick_output_is_input_bounded`. All three are driven only by
    independent arbitrary pairs and organic populations (verified,
    source; `00-baseline.md` section 4.1).
  - No law states the `min_ticks` step bounds. The nearest committed
    check is a value leg inside the meter test
    `ticks_wide_count_flatness_holds_the_width_band`
    (`crates/before/tests/meter/tick_counts.rs`). After one tick, it
    asserts that a grow-branch `ticks(n)` raises `min_ticks` by exactly
    `n`, on three fixed families (verified, source and a grep of every
    test file naming `min_ticks`). The lane's bound is an inequality that
    holds through fill-branch ticks too, on random co-generated pairs.
  - On co-generated cases, byte equality with the oracle implies
    domination and locality only if the oracle itself is right. Stating
    them checks production and oracle together against the paper's
    algebra, which guards against an error the two share (inferred).
- **Evidence.** The records count failing tests, not failing assertions,
  so no mutant is attributed to these five (reported, inventory). They
  passed every unmutated run, including 100,000 bushy and 30,000 spine
  cases (reported, addendum). They caught no defect.
- **Fold-in cost.** Small, two ways:
  - Five assertions in #74's `assert_co_generated`.
  - Better: drive the `VERSION_PARTY` law group with co-generated cases
    as a third driver, and add two laws for the `min_ticks` step bounds.
    That puts the predicates where every other driver reaches them too.

  Runtime: negligible beside the oracle comparisons already in each case
  (inferred).
- **Overlaps.** It stands beside the laws, extending their inputs rather
  than replacing them.
- **Dependencies.** #74.
- **Value.** It states two laws that no law in the registry states, and
  checks three existing ones where the tick walk's state is richest, for a
  few lines of code.

### 4. Deep co-generated spines, checked without an oracle

- **What it is.** The proptest `l3_deep_spine` and its predicate
  `check_deep`, in
  `explore/l3-events:crates/before/src/version/tick/l3_probe.rs:900-970`,
  at `30379f8d8`. It runs on a thread with a 1 GiB stack, at 8 cases by
  default (the environment variable `L3_DEEP_CASES` raises it).
- **What it reaches or checks.**
  - Spines of 1 to 2,999 levels from the lane's spine strategy (verified,
    `arb_spine(3000)`). The drawn pair is checked, then its one-tick
    successor, which the recursive oracle computes on the big stack.
  - The predicates need no oracle: canonical output, strict domination,
    region locality, fill idempotence (the walk's decision on its own
    filled output takes the grow branch), the `min_ticks` step bound,
    `ticks(1..=3)` against iterated ticks, and `ticks(2^70 + 5)` then
    `ticks(3)` against `ticks(2^70 + 8)`.
  - With right continuation at 0.7 and owned sides at weight 4 of 9,
    roughly 30% of levels nest a lookahead site. A draw of mean depth
    1,500 then holds hundreds of nested sites under one pre-scan, past
    several 64-slot memo blocks by nesting rather than by width (inferred
    from the construction; not measured).
- **Coverage beyond the committed suite.** Committed deep tick tests are
  all deterministic (verified, source):
  - closed-form spines at depth 4,096 (`deep_spines_tick_and_flag_identically`,
    `deep_spines_grow_identically`);
  - swept shapes at depth 8 to 128 carrying one wide leaf
    (`deep_and_wide_ticks_match_iterated`);
  - #34 and #75's stack-safety walks at depth `2^18`.

  `00-baseline.md` section 2.7 lists random content between depth 128 and
  100,000 as a regime the committed generators never produce. This probe
  reaches it, without an oracle.
- **Evidence.**
  - At 4 cases under the mutant set, it caught M3, M5, M7, M10 to M14, and
    M21. It timed out, inconclusively, under M2, M6, M8, and M16, and once
    unmutated at 32 cases (reported, inventory; M16's timeout and M21's
    failure verified in `calib-batch2.log`).
  - None of those catches was unique: the committed suite also caught
    each one (reported, machinery brief MB-1's calibration tables).
  - 300 unmutated cases passed in long run B (reported, addendum).
  - It caught no defect.
- **Fold-in cost.** The highest of the lane's correctness instruments.
  - Runtime varies widely, because each draw's depth is uniform from 1 to
    2,999 levels and proptest draws a fresh seed per run. Its default 8
    cases took 3.9 seconds in run 2 (verified). Yet it timed out at 32
    unmutated cases under the old 180-second limit (reported, inventory).
  - Eight cases sample thinly, and the timeouts under mutants show that a
    failing case's shrink can exceed the limit.
  - It needs the ruling-68 explicit-stack thread. Each deep tree should be
    dropped inside that thread, not leaked with `mem::forget` as the
    probe does.
  - The generator would be #74's `arb_spine_case` with its level count
    made a parameter.
- **Overlaps.** It complements #75's deterministic depth `2^18` tests with
  random content, and stands beside entry 3's claims at shallow depth.
- **Dependencies.** #74, for the strategy. Ruling 68's removal of
  `descend!` decides the recursion pattern: before it lands, the crate's
  rule routes recursive test helpers through `descend!`; after it, they
  run on an explicit-stack thread.
- **Value.** It is the only instrument that draws random tick inputs
  hundreds or thousands of levels deep. Its cost and thin sampling make it
  a candidate for an ignored or high-count-profile test rather than the
  default run.

### 5. The jump-entered rising spine (heap)

- **What it is.** The diagnostic `l3_heap_min_ticks_jump_rising_spine` and
  its builder `jump_rising_right_spine`, in
  `explore/l3-events:crates/before/src/version/tick/l3_heap.rs:188-228`,
  added at `1a8601b35`. Release profile; it prints and always passes.
- **What it reaches or checks.**
  - The family: a right spine whose preorder leaves are
    `0, J + 1, J + 2, …, J + n`, for `J` in `{2^16, 2^31, 2^40, 2^200}` and
    `n` from 1,024 to 98,304.
  - Each reading is `min_ticks`'s peak transient heap above its baseline,
    taken as the board takes it, beside the board's per-sample ceiling
    `1024 + 20·D` bytes.
  - Readings (verified, `run-heap-release-3.log`): at `J = 2^31` and
    `2^40`, 120.1 to 122.1 bytes per input byte through 65,536 levels and
    162.8 at 98,304; at `J = 2^200`, 137.9 to 147.5, and 196.8 at 98,304.
    The `2^16` control reads 19.6 to 19.7 through 65,536 levels.
- **Coverage beyond the committed suite.** The board has no family whose
  nested minima rise one level at a time after a jump between `2^31` and
  about `2^288` (verified, no `jump-rising-spine` in `main`'s registry).
  Ported to the board as `jump-rising-spine` at `b = 40`, it is the worst
  case for `version_min_ticks × heap` by a factor of 8.78 over
  `propagate-seam`. It also overtakes the pinned worst case in 16 touch
  cells, by factors of 1.0004 to 1.32 (reported, commit message of
  `08573e159`).
- **Evidence.** It is the witness for D1, the `min_ticks` transient-heap
  defect (reported, `defect-D1-min-ticks-heap.md`).
- **Fold-in cost.**
  - Already ported, on `fix/before-min-ticks-heap`'s signed demonstration
    commit `08573e159`: the registry family `JR(b, d)` plus a property
    checking its closed-form size and plateaus (reported, commit message).
  - That commit fails board acceptance on `main` by design. Under the
    owner's rule that no mechanism accepts a known failure, it can land
    only with a D1 fix, or with an owner-declared model for that one cell
    stated positively.
  - It also re-pins 17 rankings (reported, commit message).
- **Overlaps.** Entry 7 is the same shape without the jump. The D1
  write-up (`.agent-notes/2026-10-08-min-ticks-transient-heap/`) relies on
  these readings.
- **Dependencies.** D1, stopped by the owner (question 87).
- **Value.** It is the lane's strongest cost family, and the only one that
  breaks a committed ceiling. It can join the board only when the owner
  decides D1's fate.

### 6. The calibration mutant set

- **What it is.** A runtime switch and twenty one-line mutants, kept as
  patches in `<scratchpad>/auditor-l3/`: `mutations-batch1.patch`
  (9,103 bytes), `mutations-batch1-tick-only.patch`,
  `mutations-batch2.patch` (3,406 bytes), and the script
  `apply-batch2.py` (verified, file list).
  - The switch, `l3_mut(k)`, reads the environment variable `L3_MUT` once
    and compiles to `false` outside `cfg(test)`.
  - Batch 1 holds M1 to M14, mutating:
    - the route and expansion tie-breaks;
    - the memo-reference comparisons;
    - the resolution of a deferred minimum before a memo re-anchor;
    - the pre-scan's deferred link sign, its `latest_from_first`
      accumulation, and its seed offset;
    - raise's successor repairs;
    - `ticks`' remaining count;
    - the memo index;
    - `min_ticks`' equal-close count.

    M9 is a negative control that changes nothing observable.
  - Batch 2 holds M15, M16, M19, and M21 to M23: memo block clearing,
    block allocation, and cursor reset; the pre-scan's level restore; and
    `min_ticks` shifted by one either way.
  - The logs: `calib-batch1b.log`, `calib-batch1c.log`,
    `calib-batch1d.log` (522 MB), and `calib-batch2.log`.
- **What it reaches or checks.** Each mutant is a plausible defect in the
  tick walk or `min_ticks`. Run against any harness, the set measures what
  that harness can detect. The negative control M9 passes everywhere,
  which shows the probes raise no false alarms (reported, inventory).
- **Coverage beyond the committed suite.**
  - At the audit base, four mutants pass every committed tick test: M6,
    M8, M15, and M19 (reported; consistent with the logs).
  - #74 catches M6, M8, and M15 (reported, #74's entry). Its entry also
    names a fourth mutant "M21". That is #74's own label for a different
    defect, clearing only the last used memo block (verified,
    `builder-events-cogen/mutate.py`), not the lane's M21, which keeps the
    consumption cursor and which the committed suite catches.
  - #74 caught the lane's M19 in its first round, but M19 was not re-run
    after the second round changed the palette (reported, survey section
    5, item 7).
  - With #74 landed, the set has no known escape, M19 unconfirmed.
  - The lane's probes catch M11 only through the internal debug assertion
    in `HeightPrefixes::settle`, not through their own predicates
    (reported, the lane's `NOTES.md`).
- **Evidence.** The set is the evidence behind #74's existence, and behind
  every calibration line in this file.
- **Fold-in cost.** Its committed form is not obvious. The switch edits
  library code, which cannot land, and the patches are against the base
  `58285ca51`. A durable home could be the lane's record directory, or a
  cargo-mutants-style list of source edits.
- **Overlaps.** #74's builder kept its own script of five mutants and
  three calibrations. #74's reviewer kept a patch of five mutants, three
  of them shared with the builder's (section "Adjacent instruments"
  below).
  The adequacy lane's cargo-mutants survivor list is the crate-wide
  counterpart (reported, `instruments.md`).
- **Dependencies.** None.
- **Value.** It is a ready calibration set for any future change to the
  pre-scan, the memo, or `min_ticks`, and it is lost at the next reboot
  unless someone copies it somewhere durable.

### 7. The plain rising spine (heap)

- **What it is.** Two diagnostics in
  `explore/l3-events:crates/before/src/version/tick/l3_heap.rs`, release
  profile, printing only:
  - `l3_heap_min_ticks_rising_spine` (lines 60-112, at `30379f8d8`): the
    spine `(0, (1, (2, …)))` at sizes either side of `2^10` to `2^18`.
    Beside each `min_ticks` reading it takes one of a seed-party tick on
    the same spine.
  - `l3_heap_min_ticks_rising_spine_fine` (lines 158-186, at `01a240582`):
    every size within four levels of `2^k`, for `k` from 8 to 18, beside
    the board's ceiling.
- **What it reaches or checks.** `min_ticks`'s peak heap is a sawtooth,
  from a doubling `Vec` and the old-plus-new reallocation transient
  (verified, `run-heap-release-1.log` and `run-heap-release-2.log`):
  - 19.7 bytes per input byte just below `2^k + 2` levels;
  - 26.3 at `1.5·2^k`;
  - 39.4 at `2^k + 2`.

  The high side of every doubling is red against the board's ceiling. The
  seed tick reads 1.00 to 1.04 bytes per input byte (verified).
- **Coverage beyond the committed suite.** The board has no such family.
  `ascend-cliff` rises too and stays green; the auditor infers that its
  wide first leaf keeps later offsets small (reported, defect record).
- **Evidence.** It is D1's first witness.
- **Fold-in cost.** As a board family, its verdict depends on where the
  board's size ladder lands relative to powers of two. That is why the
  test brief chose entry 5's family as the robust witness (reported,
  `test-brief-D1.md`). It is blocked by D1 like entry 5.
- **Overlaps.** Entry 5.
- **Dependencies.** D1.
- **Value.** It is secondary to entry 5: it shows the doubling mechanism
  alone, without the spill.

### 8. Random tick heap search against the board's ceiling

- **What it is.** The diagnostic `l3_heap_search`, in
  `explore/l3-events:crates/before/src/version/tick/l3_heap.rs:230-284`,
  added at `873f39991`. Release profile; it prints the top four readings
  per strategy.
- **What it reaches or checks.** Peak heap of `tick` and
  `ticks(2^70 + 3)` over 3,000 spine, 3,000 bushy, and 300 wide
  co-generated pairs. Each reading is scored against `1024 + 20·D`, with
  `D` the encoded bytes of version and party. The largest fractions of the
  ceiling (verified, `run-heap-release-4.log`):
  - 0.428 on a bushy pair of 89 bytes, where the 1,024-byte intercept
    dominates;
  - 0.252 on spines;
  - 0.151 on wide pre-scans.
- **Coverage beyond the committed suite.** The board measures heap only on
  its 57 fixed families. This samples random pairs from the co-generated
  regimes. As written, it states no predicate.
- **Evidence.** It found no reading near the ceiling.
- **Fold-in cost.**
  - As a property, it would assert the per-sample ceiling over #74's
    strategies. That needs a counting global allocator in a test binary,
    and so inherits the libtest main-thread race (survey item 1.6).
  - Question 91's shared counting allocator, now being built on
    `audit/shared-counting-allocator`, is the clean way (reported,
    `STATE.md`).
  - Heap readings are judged in release on the board; in debug they
    differ. Release runtime: 13.1 seconds (verified,
    `run-heap-release-4.log`).
- **Overlaps.** The board's tick rows; entry 12.
- **Dependencies.** The shared counting allocator (question 91); #74.
- **Value.** It would extend the board's heap ceiling from fixed families
  to random inputs in the regimes where tick's memo is busiest. It has
  found nothing, and the readings sit far below the ceiling.

### 9. Random counter search for tick and `min_ticks`

- **What it is.** The diagnostic `l3_counter_search`, in
  `explore/l3-events:crates/before/src/version/tick/l3_probe.rs:1069-1156`,
  added at `bab0a3813`. It is gated on the `touch-meter` and `scan-meter`
  features.
- **What it reaches or checks.** It samples 4,000 spine pairs (up to 199
  levels), 4,000 bushy pairs, and 400 wide pairs, and skips any under 64
  bytes. For each, it reads suanpan digit touches and scan bits per input
  byte for `tick` and for `min_ticks`, and prints the top three of each.
  The maxima (verified, `run-diag-release-1.log`):
  - tick: 13.47 touches per byte (ceiling 22) and 47.41 scan bits per
    byte (ceiling 96);
  - `min_ticks`: 8.82 touches per byte, and exactly 8.00 scan bits per
    byte at every reported input, so it scans each input bit exactly once
    (inferred from the readings).
- **Coverage beyond the committed suite.** The same as entry 8, for the
  time meters. The exact 8.00 scan reading is a regularity no committed
  test pins (inferred from the readings).
- **Evidence.** It found no outlier.
- **Fold-in cost.**
  - As a property, it would assert the board's per-sample touch and scan
    ceilings over #74's strategies. The counters are deterministic, so a
    debug run measures the same thing (inferred).
  - No allocator dependency.
  - Release runtime: 8.8 seconds (verified, `run-diag-release-1.log`);
    debug unmeasured.
- **Overlaps.** The board's tick and `min_ticks` rows.
- **Dependencies.** #74, for the strategies.
- **Value.** It would extend the touch and scan ceilings to random inputs.
  It has found nothing, with readings at 61% of the touch ceiling and 49%
  of the scan ceiling.

### 10. The bushy strategy as a property

- **What it is.** `arb_pair` used as a strategy of its own, driven by
  `l3_cogen_bushy` (`arb_pair(7, 48)`, `check_one`) and `l3_family_bushy`
  (`arb_pair(10, 96)`, `check_family`). Both are in
  `explore/l3-events:crates/before/src/version/tick/l3_probe.rs`
  (lines 67-86, 343-349, 742-753), at `30379f8d8`.
- **What it reaches or checks.** Over 2,000 draws (verified,
  `run-hist-1.log`), the bushy strategy and the spine strategy #74 kept
  compare as follows:

  | Metric | Bushy | Spine |
  |---|---|---|
  | Draws with a lookahead site | 46.7% | 74.5% |
  | Deepest site nesting | 4 | 7 or more in 42.2% |
  | Depth 4 or more | 19.8% | 73.4% |
  | Draws with an owned-right raise | 41.2% | 69.7% |
  | Draws with two sites in one range | 0.1% | 25% |
- **Coverage beyond the committed suite.** None measured beyond #74. #74
  keeps the bushy region builder as side regions inside its spine and
  wide strategies (verified, `arb_bushy_region(2, 4)`). The spine reaches
  more on every metric above. Balanced trees 7 to 10 levels deep that are
  not spine-shaped are its only distinct shapes (inferred).
- **Evidence.** It caught nothing the other strategies missed (reported,
  #74's entry). #74's reviewer built a mutant aimed at it: at a party
  branch over a version leaf, the pre-scan skips only the left party
  child. The pre-existing `family_pairs` test and every co-generated
  strategy caught it too (reported, `reviewer-events-cogen/NOTES.md`).
- **Fold-in cost.** Low: #74 built it once and dropped it under the rule
  that a strategy must catch something. Runtime at 256 cases (verified,
  run 2):

  | Property | Seconds |
  |---|---|
  | `l3_cogen_bushy` | 1.06 |
  | `l3_family_bushy` | 2.83 |
  | `l3_cogen_spine`, for comparison | 5.08 |
  | `l3_family_spine`, for comparison | 12.64 |
  | #74's spine property, which ticks three versions per case | 12.7 (reported, `builder-events-cogen/r2-first.log`, round 2) |
- **Overlaps.** #74's spine strategy, which dominates it.
- **Dependencies.** #74.
- **Value.** Little.

### 11. The second late perturbation

- **What it is.** The fourth version that `check_family` ticks
  (`explore/l3-events:crates/before/src/version/tick/l3_probe.rs:736-739`,
  at `30379f8d8`): the fill-fixed successor with the leaf one place
  earlier than the first perturbation set to the palette height plus one.
- **What it reaches or checks.** A second divergence site after a long
  matched prefix, at a height one above a palette entry, which makes near
  ties with neighboring leaves likely (inferred).
- **Coverage beyond the committed suite.** #74's `TickCase::versions`
  keeps the drawn version, the fill-fixed successor, and one perturbation.
  This adds a second divergence site and an off-palette height. No
  measurement separates its contribution.
- **Evidence.** None attributable.
- **Fold-in cost.** A fourth element in `TickCase::versions`. That adds
  about a third to each case's runtime, which matters for the wide and
  multi-scan properties (inferred).
- **Overlaps.** #74's perturbation.
- **Dependencies.** #74.
- **Value.** Marginal.

### 12. Memo-dense tick heap families

- **What it is.** The diagnostic `l3_heap_memo_families`, in
  `explore/l3-events:crates/before/src/version/tick/l3_heap.rs:21-58` and
  `142-156`, at `30379f8d8`; release profile.
- **What it reaches or checks.** Tick's peak heap on three families, at
  1,024 to 65,536 sites (verified, `run-heap-release-1.log`):
  - sibling sites under one pre-scan with alternating minima: 6.73 to
    6.77 bytes per input byte;
  - the same with constant minima: 0.90 to 0.95;
  - a nested chain of sites: 11.71 to 11.72.

  All are linear.
- **Coverage beyond the committed suite.** Probably none. The board's
  `memo-chain`, `memo-comb`, `memo-fanout`, `memo-oscillating`, and
  `memo-churn` families build the same kinds of memo traffic, and
  `memo-comb` is `version_tick`'s pinned heap worst case (verified,
  `board/worst.rs`). The siblings here sit in a balanced tree rather than
  a spine; that is the only difference in shape (inferred).
- **Evidence.** It confirmed the closed fix for tick's memo storage
  (reported, coverage record).
- **Fold-in cost.** Not worth folding as is.
- **Overlaps.** The board's memo families.
- **Dependencies.** None.
- **Value.** None beyond the board, as far as I can tell.

### 13. Generator histograms

- **What it is.** The diagnostic `l3_generator_histograms` with `Stats`,
  `walk_stats`, and `histogram_of`, in
  `explore/l3-events:crates/before/src/version/tick/l3_probe.rs:543-661`,
  at `30379f8d8`.
- **What it reaches or checks.** Over 2,000 draws each of bushy, spine,
  and the committed independent pair, it prints histograms of:
  - fill-branch draws;
  - site count, deepest site nesting, and depth;
  - draws with two sites in one range;
  - draws with an owned-right raise, a party branch over a version leaf,
    or a leaf wider than 64 bits.

  It took 7.3 seconds in a debug build, and reproduced the round-1
  histograms exactly (verified, run 2 against `run-hist-1.log`).
- **Coverage beyond the committed suite.** #74's census enforces floors on
  nesting and shared ranges for its own strategies. Three things it does
  not carry:
  - the row for the committed independent generator;
  - the owned-right, over-leaf, and wide-leaf counts;
  - the printed distribution.
- **Evidence.** These are the numbers behind #74 and behind section 2.6
  of the baseline.
- **Fold-in cost.** The printer is a diagnostic. Its committed-generator
  row could become a floor or ceiling in the census of `generators/tests.rs`.
- **Overlaps.** #74's census; the adequacy lane's census
  (`explore/l8-adequacy:crates/before/src/testing/l8_census.rs`), which
  measures shape and relation mix but not tick sites.
- **Dependencies.** None.
- **Value.** Low as an instrument. The committed-generator row is the only
  recorded measurement of how rarely independent pairs reach tick's
  pre-scan regimes.

### 14. Memo reach diagnostic

- **What it is.** The diagnostic `l3_generator_reach` with `is_site`,
  `sites_in`, `scans`, and `reach_of`, in
  `explore/l3-events:crates/before/src/version/tick/l3_probe.rs:999-1067`,
  added at `bab0a3813`.
- **What it reaches or checks.** Over 500 draws per generator, it counts
  memo slots per pre-scan and pre-scans per walk (verified,
  `run-diag-release-1.log`):
  - wide: 329 draws with a pre-scan over 64 slots, up to 301 slots;
  - multi-scan: 125 draws with two such pre-scans;
  - spine: at most 23 slots;
  - bushy: at most 4;
  - the committed independent pair: at most 2.

  It took 19.6 seconds in a debug build, and reproduced the release
  readings exactly (verified, run 2 against `run-diag-release-1.log`).
- **Coverage beyond the committed suite.** #74's census carries the same
  slot counting as enforced floors (`Sites::scan_slots`). Left behind:
  the committed generator's row and the maximum-slot reading.
- **Evidence.** It set the wide strategy's regime claims.
- **Fold-in cost.** Nothing to fold beyond #74.
- **Overlaps.** #74's census, which supersedes it.
- **Dependencies.** None.
- **Value.** None beyond #74.

### 15. Bridge round trip on one co-generated pair

- **What it is.** The unit test `l3_bridge_round_trip`, in
  `explore/l3-events:crates/before/src/version/tick/l3_probe.rs:359-376`,
  at `30379f8d8`.
- **What it reaches or checks.** One fixed pair resolves to production
  values and converts back to the same oracle trees.
- **Coverage beyond the committed suite.** None. Every bridged
  differential depends on the same round trip, and the committed bridge
  tests cover rejection (verified, `testing/bridge/tests.rs`).
- **Evidence.** It is a sanity check on the co-generator.
- **Fold-in cost.** Not worth folding.
- **Overlaps.** Every bridged differential.
- **Dependencies.** None.
- **Value.** None.

## Adjacent instruments built downstream of this lane

Agents downstream of the lane built these in their own scratch
directories. They belong to `09-build-and-review-probes.md`, so each gets
one line here, so that none is counted twice or missed. When I read 09,
it was still being written and held entries A1 to A20 of 36:

- **#74's builder's mutant script**: `<scratchpad>/builder-events-cogen/mutate.py`.
  It holds M6, M8, M15, and M19 in its own spellings, and an M21 that
  clears only the last used memo block. That M21 differs from the lane's
  M21. The script also holds three calibrations: a halved multi-scan
  strategy, an off-by-one memo index, and a starved wide strategy
  (verified, file). It was not yet named in 09 when I read it; 09's
  grouped calibration entry A36 is the likely home.
- **#74's reviewer's memo model property**:
  `consecutive_scans_read_back_only_their_own_differences`, in
  `<scratchpad>/reviewer-events-cogen/experiment2.patch`. It drives `Memo`
  directly through one to four consecutive pre-scans of up to 191 slots
  and writes them in shuffled order, against a vector model. #74 did not
  include it (verified, patch). 09 catalogues it as A17.
- **D1's fixers' and reviewers' probes**: the stopped `min_ticks` designs
  on the `archive/min-ticks-*` branches and `fix/before-min-ticks-heap`,
  and the final reviewer's quadratic big-integer probe, which the survey
  carries as item 1.1 (reported, survey). 09 catalogues the fixer's heap
  probe as A2, the board family from entry 5 above as A5, the limb probe
  as A6, and the real-scale stepped spine as A10. Entry 5 here and 09's
  A5 describe the same commit, `08573e159`, from its source and its
  port.

## What I could not assess

- **M19 at #74's tip.** No run exists, and a mutant run is neither a
  census nor a runtime measurement, so it was outside my allowance
  (survey section 5, item 7, flags the same gap).
- **Debug runtime of the two random searches.** Only their release
  runtimes exist.
- **The deep spine's site nesting.** It is inferred from the construction,
  not counted.
