# Machinery brief MB-1: co-generated (version, party) pairs for the tick differentials

Written by auditor-l3 for a builder who has not seen the explore branch.

## The failure class it catches

The fused tick walk (`crates/before/src/version/tick.rs` and `tick/`) has its most
intricate state in the pre-scan (`tick/prescan.rs`, `tick/prescan/suspended.rs`) and
the memo (`tick/memo.rs`): minima recorded ahead of the main cursor for *lookahead
sites* (a party branch whose left child is wholly owned and whose right child is
partially owned, over a version branch), stored as differences from sibling or
enclosing minima, with deferred first entries resolved when an enclosing range
closes. A defect in that state shows up only when one pre-scan covers several sites:
nested at depth three or more, two or more sites inside one range, or more than 64
sites (one memo block).

The committed arbitrary-pair suites (`tick/tests.rs::arbitrary_pairs_tick_and_flag_identically`,
`tick_is_ticks_one`, `ticks_matches_iterated_ticks_arbitrary`, `simplification_is_idempotent`,
`grow_branch_is_absorbing`) draw the party (`generators::arb_oracle_party_nonempty`) and
the version (`generators::arb_oracle_version`) independently at `prop_recursive` depth 4.
Measured over 2000 draws (explore branch, `l3_generator_histograms`):

| Generator | no site | max site nesting | depth >= 4 | a range with >= 2 sites |
|---|---|---|---|---|
| committed independent pair | 81% | <= 2 | 0.1% | 0% |
| co-generated bushy | 53% | <= 4 | 20% | 0.1% |
| co-generated spine | 26% | 7+ in 42% | 73% | 25% |

So no committed *random* generator ever puts two lookahead sites in one pre-scan range,
and none crosses a memo block. Memo reach over 500 draws each (`run-diag-release-1.log`):
the committed independent generator reserves at most 2 memo slots in any pre-scan; the wide
strategy exceeds 64 slots in one pre-scan in 66% of draws (up to 301); the multi-scan strategy
runs two such pre-scans in one walk in 25% of draws. Among committed tests, only the hand-built
meter families (`MemoChain`, `MemoComb`, `MemoFanout`, ...) reach these regimes, at fixed shapes.

## Constructible defects it catches that the committed suites miss

Four plausible one-line defects pass every committed tick, `ticks`, and `min_ticks` test
(72 of 72) and fail the co-generated probes: two value defects in the memo-reference logic,
below, and two memo-block defects, in the batch 2 table under "Calibration evidence".

- **Tie read as domination in the `Min` arm.** In `TickWalk::consume_lookahead`
  (`tick.rs`, `MemoReference::Min` arm), treating
  `compare_above_vs(above, &arm_offset) == Ordering::Equal` like `Less` arms the
  memoized minimum and emits at it on a tie, which forces a divergence where the walk
  should still match its input. Shrunk witness from `l3_cogen_spine` ("flag tripped but
  oracle fill is identity"):
  version `Node(0, Leaf(3), Node(0, Node(0, Leaf(3), Node(0, Leaf(3), Leaf(0))),
  Node(0, Leaf(0), Node(0, Leaf(3), Node(0, Leaf(3), Leaf(0))))))`, party
  `(1, ((1, (1, 0)), (1, (1, 0))))`: an outer lookahead whose range holds two sibling
  sites, each nesting a site, with tied minima. The same defect also reaches the
  `unreachable!("simplification is idempotent")` in `TickWalk::ticks`.
- **Missing `resolve_deferred` in `pop_lookahead`.** Skipping
  `self.minima.resolve_deferred()` before re-zeroing `REL_FOLLOWER` for a non-outermost
  lookahead leaves the relation anchored above the true minimum. Shrunk witness ("flag
  tripped but oracle fill is identity"): version `Node(0, Leaf(2), Node(0, Node(0, Leaf(2),
  Node(0, Node(0, Leaf(2), Node(0, Leaf(2), Leaf(0))), Leaf(3))), Node(2, Leaf(1),
  Leaf(0))))`, party `(1, ((1, ((1, 0), 0)), (1, (0, 1))))`; wide-value variants fail the
  byte-for-byte oracle comparison.

Two more (a sign error in `PreScan::resolve_inner`'s deferred first link, and a skipped
`latest_from_first` accumulation in `PreScan::record`) are caught by the committed suites
only through two hand-built families (`family_pairs_tick_and_flag_identically` and
`undercut_under_a_live_relation_family_ticks_identically`); none of the committed random
suites catches them, while four or five co-generated probes do.

## Which instrument it extends

`testing::generators` (a new co-generating strategy beside `arb_oracle_party_nonempty` /
`arb_oracle_version`) and the existing `assert_tick` differential in
`crates/before/src/version/tick/tests.rs`. No new oracle: the recursive tree oracle's
`event` and `fill` remain the reference. Register the generator in
`src/testing/validation_index.rs` with the failure class above.

## Generator construction (builds valid values directly; nothing is rejected)

Build both trees together, region by region, as a skeleton whose version leaves carry a
small tag, then resolve the tags against a per-case *palette* of absolute heights.

1. Skeleton type: a region is one of
   - `Owned(v)`: party `1` over an arbitrary version skeleton `v` (depth <= 2);
   - `Unowned(v)`: party `0` over `v`;
   - `OverLeaf(pl, pr, h)`: a party branch `(pl, pr)` (each an arbitrary party skeleton of
     depth <= 2) over one version leaf tagged `h`;
   - `Aligned(l, r)`: both trees branch; recurse.
2. Bushy strategy: `prop_recursive(depth 7..10, nodes 48..96)` whose recursive case is
   weighted 6 : 3 : 2 among `Aligned(inner, inner)`, the lookahead pattern
   `Aligned(Owned(v), inner)`, and the owned-right pattern `Aligned(inner, Owned(v))`.
3. Spine strategy: a vector of 1..48 levels; each level says which side continues
   (right with probability 0.7) and what sits on the other side (`Owned` 4, `Unowned` 2,
   `OverLeaf` 1, a small bushy region 2); fold the levels around a small bushy tip.
4. Wide strategy: one outermost lookahead `Aligned(Owned(leaf), X)` where `X` is a balanced
   tree over 40..400 regions, each a minimal site `Aligned(Owned(leaf), OverLeaf(1, 0, c))`
   (weight 4), a site nesting a site (2), `Unowned` (2), `Owned` (1), or a small bushy
   region (2). This crosses memo blocks under one pre-scan while staying oracle-shallow.
4b. Multi-scan strategy: 2..=4 wide trees (each over 20..160 regions, each rooted at its own
   outermost lookahead) joined by ordinary `Aligned` nodes, so one walk runs several pre-scans
   and reuses memo blocks across them.
5. Palette: 1..=5 absolute heights per case, each drawn from small (0..6), word-range,
   near `u64::MAX`, `2^64 + 0..4`, `2^130 + 0..4`, and `m * 2^s` (60 <= s < 200). Small
   palettes force ties and equal minima (zero memo differences); `2^k + j` entries force
   near-ties at wide scale.
6. Resolve: version leaves are `Leaf(palette[tag % len])`; version branches are
   `tree::Version::node(0, l, r)` (which normalizes and collapses equal leaf pairs);
   party branches are `tree::Party::node(l, r)` (which collapses `(1, 1)` and `(0, 0)`).
   Replace an empty party with the seed. Both results are normal by construction.

Derive two more inputs from each pair, which the bushy and spine distributions otherwise
leave thin (78% of draws take the fill branch at the first tick):

- the *fixed point*: the oracle's one-tick successor, which takes the grow branch at depth;
- a *late perturbation*: the fixed point with its `j`-th-from-last leaf (`j` in 0..6) set to
  a palette height, which forces the walk's first divergence after a long matched prefix.

## The exact assertions (per pair)

The existing `assert_tick` body, plus, on each of the three inputs:

- `ticks(k) == ` k iterated public ticks `==` the oracle's k-fold `tick`, for `k` in 1..=4;
- `ticks(0)` is the identity;
- `ticks(a)` then `ticks(b)` equals `ticks(a + b)` for `a, b` in
  `{1, 2^64 - 1, 2^64 + 3, 2^200 - 1}`.

## Where it belongs

`crates/before/src/testing/generators.rs` for the strategies (with a histogram test in
`generators/tests.rs` that asserts each regime above keeps generator mass: sites >= 2 in
one range, nesting >= 3, > 64 memo slots in one scan), and new proptests in
`crates/before/src/version/tick/tests.rs` beside `arbitrary_pairs_tick_and_flag_identically`.
Keep the wide strategy's default case count low enough for nextest's 180-second limit in a
debug build: at 1000 cases the wide strategy timed out at 180 s on ox-east-1 under load, while at
40 cases the wide and multi-scan strategies each finished inside the 23.5 s their group took.

## Calibration evidence

Explore branch `explore/l3-events` at 30379f8d plus runtime-switched mutations (logs
`calib-batch1{b,c,d}.log` in the auditor-l3 scratch directory). Probe counts are distinct failing probe tests, timeouts excluded; committed counts are the
nextest Summary lines. Probes at
`PROPTEST_CASES=1000` (wide family at 40, deep harness at 4); committed set = nextest filter
`test(/version::tick::/) - test(/l3_/) | test(/min_ticks/) | test(/ticks/)` (72 tests).

| Mutation | Committed failing | Co-generated probes failing |
|---|---|---|
| route `Cost::prefer` tie goes left | 10 | 4 |
| expansion `Distance::prefer` tie goes left | 5 | 4 |
| `consume_h_anchored` tie diverges | 15 | 7 |
| `resolve_inner` drops the link negation | 2 (hand-built families) | 5 |
| `record` skips `latest_from_first` | 2 (hand-built families) | 4 |
| `Min`-arm tie read as domination | **0** | 3 |
| raise writes a zero delta for the expansion's first right sibling | 21 | 9 |
| `pop_lookahead` skips `resolve_deferred` | **0** | 3 |
| `StoredAccumulator::new` always boxes (negative control, behavior-preserving) | 0 | 0 |
| memo `Block::take` index off by one | 2 | 4 |
| raise drops the trailing successor repair | 27 | 9 |
| `ticks` raises `n` instead of `n - 1` after a fill | 6 | 6 |
| pre-scan `seed_relation` drops the offset | 4 | 6 |
| `min_ticks` skips the equal-close count | 7 | 9 (all through the `HeightPrefixes::settle` debug assertion) |

The negative control passing everywhere shows the probes raise no false alarms on a
behavior-preserving change.

Batch 2 targeted the regimes only the wide and multi-scan strategies reach (log
`calib-batch2.log`; the unmutated baseline passes every probe, the multi-scan family included):

| Mutation | Committed failing | Co-generated probes failing |
|---|---|---|
| `Memo::begin_scan` clears only the first used block | **0** | multi-scan only ("a memo slot is written once") |
| `PreScan::resolve_inner` decrements `reference_level` instead of restoring the parked level | 6 | 7 |
| `Memo::reserve` allocates the block for slots 64..127 one reservation late | **0** | wide and multi-scan (index out of bounds) |
| `Memo::begin_scan` keeps the consumption cursor | 9 | 6 |

So four constructible defects (two value defects in the `Min` memo reference, one stale-block
defect needing two large pre-scans in one walk, one block-boundary crash needing more than 64
slots in one pre-scan) pass every committed test and fail the co-generated strategies. The
stale-block detection rests on a debug assertion; in a release build the stale value would
surface as a wrong tick, which the oracle comparison reads (inferred; not run in release).
