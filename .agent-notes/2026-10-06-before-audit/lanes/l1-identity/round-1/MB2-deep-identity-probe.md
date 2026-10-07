<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the identity lane's (L1) MB2 brief, revised for building after the owner's scope ruling; line references checked against main 97798357 and audit/deep-surfaces 20da73ad. -->

# Machinery brief MB2: drive every public operation at depth `2^18`

## The goal

The crate's hard rule says no library traversal recurses on depth the input
controls, and the owner's doctrine asks a committed deep-input stress test to
prove it. Today the proof covers only the operations the deep tests happen to
call, and it runs at a depth too shallow to rule out small frames. This brief
closes both gaps: it raises the committed stack-safety tests that still run at
100,000 levels to `2^18`, and it drives, at that depth, every public operation
whose entry point no deep test calls. (`sync_constructed`, at 10,000 levels,
and the borsh round-trip, at 48, check other things and stay as they are.)

Like the sibling tests, this proves the absence of *frame-keeping* recursion
only. A recursion the optimizer turns into a loop keeps no frames and passes;
the owner ruled that sufficient, so this brief does not depend on any
unoptimized-build leg.

## Scope

The census starts from the board's coverage tables
(`src/testing/meter/board/coverage.rs`, `BOARD_PRICED` and the trait families),
which the surface check holds complete against the public API. An operation is
covered when a committed test at depth 100,000 or more calls its entry point.
Parts B and C list every operation that walks a tree and is not covered.
Excluded, because they walk no tree:

- constant-time constructors and accessors (`Party::seed`, `is_seed`,
  `dangerously_alias`, `as_bytes`, `encoded_bits`, `Version::new`,
  `is_empty`, `Clock::from_parts`, `into_parts`, `party`, `version`);
- byte-level operations: `Eq` and `Hash` for `Party`, `Version`, `Clock`, and
  `Span`, `Display` for `Party` and `Version`, and `Party::encode`
  (`Clock::encode`, which the siblings drive, writes the same bytes);
- `Rank` and `Count` arithmetic, rendering, and parsing, which are numeric
  rather than tree walks (`Rank::decode` at a deep exponent is already
  driven);
- `Query::into_owned` (a clone) and the `From` conversions into `Span`,
  `Query`, and `Version`, which delegate to walks listed or already driven.

## Base

Stack the branch on `audit/deep-surfaces` (ready entry #34, tip `20da73ad`).
That branch adds `deep_right_spine_party` (`src/testing/generators.rs`, after
`deep_left_spine_party` at `:222`) and the sibling test
`deep_tree_shape_hull_and_fold_stack_safety`, and its doc states the depth
argument this brief reuses. MB2 edits the same file, so stacking avoids a
conflict. If #34 lands first, rebase onto `main`.

Line references below are for `main` at `97798357`; #34 shifts only
`src/clock/tests.rs` lines after `:718` (by 81 lines).

## The depth argument

libtest runs each test on a 2 MiB thread. At `2^18` levels that leaves 8 bytes
per level, and a call frame costs at least 16 bytes on x86_64 and aarch64, so
any walk whose compiled code keeps one frame per level overflows. At 100,000
levels the same stack leaves about 21 bytes per level, so a recursion with a
16- to 20-byte frame passes the committed tests today. Restate this once, on
the first deep test, and have the others point to it (as #34's doc does).

## Part A: raise the sibling deep tests to `2^18`

| Test | Change |
|---|---|
| `deep_tree_stack_safety` (`src/clock/tests.rs:558`) | `DEPTH` at `:559`: `100_000` to `1 << 18` |
| `deep_tree_query_and_causal_stack_safety` (`:657`) | `DEPTH` at `:661` |
| `deep_tree_min_ticks_stack_safety` (`:724`) | `DEPTH` at `:725` |
| `without_constructed` (`src/party/tests.rs:488`) | top of `SCALES` at `:494`: `100_000` to `1 << 18` |

Rewrite the prose that names the old depth: the three tests' doc comments
("depth-100k", "100,000 levels deep", "a 100k exponent", and the claim of "a
depth no program stack could carry", which is not true at 100,000 for small
frames), and the board's coverage row (`src/testing/meter/board/coverage.rs:570`,
whose #34 wording says "at 100,000 levels or more").

**What it catches that today's tests miss:** a walk rewritten as a recursion
whose compiled frame is 16 to 20 bytes, in any operation the siblings drive:
the clock and party codecs, `tick`, `fork`, `join`, `sync`, `is_disjoint`,
`send`/`recv`, the version lattice and order, `rank`, `distance`, `lag`, the
`Rank` and `Span` codecs, `Span::place` and `dominance`, the span algebra,
query `contains` and `coverage`, projection, `min_ticks`, and `without`'s
difference walk. This follows from the frame arithmetic; no mutant was built
for it.

**Measured at `2^18`** (sizing run below): `deep_tree_stack_safety` 0.72 s,
`deep_tree_query_and_causal_stack_safety` 1.61 s,
`deep_tree_min_ticks_stack_safety` 0.16 s, each `without_constructed` case
under 0.10 s. All pass.

## Part B: `deep_identity_stack_safety`, the identity entry points

A new sibling test in `src/clock/tests.rs`, beside
`deep_tree_shape_hull_and_fold_stack_safety`.

**Construction.** For each `cell` in
`[deep_left_spine_party(1 << 18), deep_right_spine_party(1 << 18)]`, let
`comb = Party::seed().without(&cell)`. The cell is one owned leaf at the end
of a unary spine. The comb, its complement, owns one sibling subtree at every
level, so every level of the comb is a two-child branch. Each operation below
then runs on both shapes and both spines.

**Operations** (19; each is a public entry point no committed deep test calls):

1. `Party::covers`, both outcomes: `seed.covers(&cell)`, `!cell.covers(&comb)`,
   `!comb.covers(&cell)`, `comb.covers(&alias)`.
2. `Party::without` through the public entry: the disjoint fast path
   (`comb \ cell == comb`) and the `None` arm (`comb \ comb`).
3. `Party::decode` of `comb.encode()`.
4. `Party`'s `FromStr`, parsing `comb.to_string()`.
5. `Party::forks`, fully drained (`forks(5)`), with `join_all` restoring the
   party.
6. `PartyForks` dropped after a partial drain (`forks(1000).take(3)`).
7. `Party::forks` with a count above `u128::MAX` (`take(2)`).
8. `Party::join_all` (rejoining each set of shares above).
9. `From<Party> for [Party; 5]`, then `join_all` of the five.
10. `Party::ticks` (`cell.ticks(&mut v, 3)`).
11. `Clock::ticks`.
12. `Clock::forks` (`forks(4)`, collected).
13. `Clock::sync_all` over the comb's clock, its four children, and the cell's
    clock.
14. `Clock::recv_all`.
15. `Clock::absorb_all`.
16. `Clock::own_version` materialized with `to_version`.
17. The heterogeneous joins `Clock | &Version`, `Clock |= Version`, and
    `Version | Clock`.
18. `From<Clock> for [Clock; 3]`.
19. `Clock::join_all` of every clock back to the seed (assert `is_seed`).

The reference implementation is
`crates/before/src/clock/tests.rs::deep_identity_stack_safety` on
`explore/l1-identity` at `fa7cfe0f`. Keep its assertions; each forces its
whole walk (the rejoin to the original, the `is_seed` at the end).

**What it catches that today's tests miss.** Three walks have no deep
coverage at all; they belong to the fork iterators:

- the plan's spatial path, `SharePath` (`src/party/forks.rs:34-108`);
- share construction, `PartyReader::select_path` (`src/party/fork.rs:31-69`);
- the removal walk, `Removal` (`src/party/fork/remove.rs`).

Demonstrated: mutant M26 rewrites `Removal::run`'s descent loop
(`src/party/fork/remove.rs:178-180`, `for right in path { removal.descend(..) }`)
as a self-recursive helper over the path iterator. Every committed test in
`--lib`, `--test meter`, and `--test forks_count` passes it (666 of 666; the
run's only failure, 1 of 686, was the explore probe aborting with a stack
overflow). The probe ran at depth 100,000 on a 256 KiB stack; the optimizer
did not remove the recursion there, so its frame is at least 16 bytes and it
also overflows at `2^18` on 2 MiB (inferred). A recursive rewrite of
`SharePath` or `select_path` is the same class (inferred).

The other items reach walks the committed deep tests already drive (for
example, `covers` shares `is_disjoint`'s comparison walk, and the splits,
`join_all`, and `sync_all` reduce to `fork` and `join`). For them the test
proves each entry point rather than a new walk: it catches an entry point
reimplemented with its own recursion instead of delegating, such as `covers`
rewritten as a recursive comparison. That is the weaker catch, and the reason
to keep these items cheap.

**Measured:** 2.14 s for both spines.

## Part C: `deep_tree_remaining_surfaces_stack_safety`, other lanes' entry points

A second new sibling test. These operations belong to the versions, ranks,
spans, and queries rather than to identity; the coordinator folded them into
this brief so that every public operation is driven at depth. Split this part
into its own branch if review by those lanes is preferred.

**Construction.** As #34: for each spine at `2^18`, `half = keeper.fork()`,
tick `a` on `keeper` and `b` on `half` (a concurrent pair), and
`late = a` ticked three more times through `Version::ticks`. Derive
`joined = a.join(&b)`, `met = a.meet(&b)` (empty), `span = [met, joined]`, and
`tail = [a, late]`.

**Operations** (35):

1. `Version::ticks` (`late.ticks(&keeper, 3)`).
2. `Version::join`, the named method.
3. `Version::meet`, the named method.
4. `Version ^ Version`.
5. `Version::decode`.
6. `Version`'s `FromStr`.
7. `Ranked::encode`.
8. `Ranked::encode_to`.
9. `Ranked::encode_rank`.
10. `Version::encode_rank`.
11. `Version::encode_rank_to`.
12. `Ranked::decode`, compared with `a.ranked()`.
13. `OwnVersion` compared with `OwnVersion` (`partial_cmp` of
    `&joined / &keeper` against `&joined / &half`, and `<=`).
14. `Span::precedence`.
15. `Span::contains` with a version argument and with a span argument.
16. `Span::union`, named.
17. `Span::intersect`, named.
18. `Span::join`, named.
19. `Span::meet`, named.
20. `Span::intersect_all`.
21. `Span::join_all`.
22. `Span::meet_all`.
23. `Option<Span>: Sum`, over borrowed spans.
24. `Option<Span>: Product`, over borrowed spans.
25. `causally::until`, then `contains`.
26. `causally::strictly_after`, then `contains`.
27. `causally::strictly_before`, then `contains`.
28. `causally::delta`, then `contains`.
29. `causally::toward`, then `contains`.
30. `Floor::contains` (from `causally::after`).
31. `Ceiling::contains` (from `causally::before`).
32. `Floor::or_concurrent`, then `contains`.
33. `Ceiling::or_concurrent`, then `contains`.
34. `!Floor`, then `contains`.
35. `Floor::coverage` and `Ceiling::coverage`, and `coverage` on an `until`
    query.

The reference implementation is
`deep_tree_remaining_surfaces_stack_safety` at the same commit. It wraps
results whose values I did not derive in `std::hint::black_box`; the builder
should replace each with the outcome the rustdoc implies (for example,
`span.precedence(&a)` and each query's verdict on `a`), so the assertion forces
the walk the way #34's do.

**What it catches.** Five of these have walks of their own that no committed
deep test reaches:

- `Span::precedence` (`src/span.rs:342`, `place::precedence`);
- `Span::contains` (`src/span.rs:410`, `place::contains`);
- `OwnVersion` against `OwnVersion` (`projection::Comparison` over two
  projected streams; `src/version/own.rs:162`);
- `Ranked::decode` (the rank stream, then the version validator);
- `Version::ticks`, whose fused walk is driven elsewhere only by the meter
  suite at 125,000 levels on the seed party (`tests/meter.rs:62`), about 17
  bytes per level of a 2 MiB stack.

A frame-keeping recursion in any of them passes today (inferred; no mutant
built). The rest delegate to walks the siblings drive, so they prove entry
points, as in Part B.

**Measured:** 4.32 s for both spines.

## Sizing run

One run on ox-east-1, explore commit `fa7cfe0f` (#34 merged, the drafts
above), box load average 206:

```
on-illumos.sh /Users/oxide/src/rumors-audit-l1-identity 'unset CARGO_TARGET_DIR; nice -n 10 cargo nextest run -p before --all-features --locked --build-jobs 24 -j 8 --lib --no-fail-fast -E "test(/deep_tree|deep_identity|without_constructed/)"'
```

- Incremental build of `before`'s lib tests: 2 min 15 s.
- 11 tests, all passing; 4.3 s wall at 8 threads, about 11 s summed.
- The slowest single test is Part C at 4.3 s, far under nextest's 60 s
  slow-test period, so no part needs splitting for time. Split only for
  ownership (Part C).

Every operation in Parts B and C passes at `2^18` on the default stack, so
today's code keeps no per-level frames on any public path; the branch is
pure evidence, with no fix expected.

## Calibration the builder must reproduce

1. All four deep tests and `without_constructed` pass at the branch tip.
2. Apply M26 (replace `src/party/fork/remove.rs:178-180` with a call to a new
   self-recursive helper that takes the path iterator by value, descends one
   level, and recurses on the rest): `deep_identity_stack_safety` aborts with a
   stack overflow, and the committed tests without the new one pass. Revert;
   `git diff` empty.

## Related queued work: S1 and SB1 go in separate branches

Keep MB2 test-only, in its own branch, and give S1 and SB1 one branch each:

- **S1 (writer retention)** changes production writers (`PartyWriter::finish`
  and the version writers). It can move heap readings on the board, so it may
  stop for an owner ruling on a ceiling or ranking; that must not hold up a
  test-only branch. It adds retention tests to `src/party/tests.rs` near
  `asymmetric_fork_sizes_the_small_result_independently`, away from
  `without_constructed`, so it conflicts with MB2 only if both rewrite the
  same lines.
- **SB1 (`sync_all` through `join_all`)** is a small production refactor
  confined to `Clock::sync_all` (`src/clock.rs:397`). It touches no file MB2
  touches. MB2's Part B drives `sync_all` at depth, so whichever lands second
  is checked by the first; no order is required.
- S1 and SB1 do not overlap, and they serve different purposes (memory
  retention and a refactor), so bundling them would only enlarge one review.
