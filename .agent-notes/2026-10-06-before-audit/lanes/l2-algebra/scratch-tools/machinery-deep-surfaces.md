# Machinery brief: prove the shape walks, the concurrent-pair hull, and the receiverless folds at depth 100,000

## The gap

The owner's doctrine requires every deep traversal to be iterative *and* proven
so by a committed deep-input stress test. The crate's proofs are two tests in
`crates/before/src/clock/tests.rs`, `deep_tree_stack_safety` and
`deep_tree_query_and_causal_stack_safety` (depth 100,000). The board's coverage
table (`src/testing/meter/board/coverage.rs`, the "unbounded depth" row) cites
them as the depth pin.

Neither test drives these public surfaces at depth:

- the shape walks: `Version::shape` (`Plateaus`), `shape::combine` (`Cells`),
  `Clock::shape` (`Overlay`), and `Party::shape` (`Regions`);
- the pair hull on a *concurrent* pair. `Version::span`, `^`, and the leaf
  combine of `span_all` reach `Version::hull_bits` only when the operands are
  concurrent. Every span in the committed deep tests has comparable
  endpoints (`early <= late`), so it takes the comparable fast path and never
  runs the hull sweep;
- the receiverless folds `Sum` and `FromIterator` (owned and borrowed);
- join and meet on a concurrent deep pair. The committed test joins only
  `early <= current`.

All of these are iterative today: I verified it by running them at depth
100,000 on ox-east-1 (below). The gap is in the evidence, not in the code.

## The failure class it catches

A recursive rewrite of any of these walks. One example is a `VersionWalk` or
`Cells` iterator built on a recursive descent of the tree; another is a hull
sweep that recurses per level. Such a rewrite passes every committed test:

- the unit and property tests stay at depths of a few thousand or less;
- the board's deepest families reach about 32,000 levels (dense spine
  `8_000` × the ladder-top scale `4.0`, `src/testing/meter/board/family.rs`),
  run as a release example on the main thread's 8 MiB stack, where a frame of
  a hundred bytes or so still fits. This is inferred, not measured.

The proposed test fails such a rewrite by aborting with a stack overflow on
the default 2 MiB test-thread stack.

## Which instrument it extends

The deep-tree stress tests. Add a sibling test in
`crates/before/src/clock/tests.rs`, next to
`deep_tree_query_and_causal_stack_safety`, named for example
`deep_tree_shape_hull_and_fold_stack_safety`. Do not add a new mechanism.
Optionally, widen the board coverage row's disposition so it names the deep
tests that actually carry the proof.

## Construction (builds valid values directly)

Use the crate's existing generator `deep_left_spine_party(DEPTH)` with
`DEPTH = 100_000`:

```rust
let mut keeper = deep_left_spine_party(DEPTH);
let half = keeper.fork(); // splits the deep tip into two depth-100,001 halves
let mut a = Version::new();
keeper.tick(&mut a);
let mut b = Version::new();
half.tick(&mut b);
assert!(a.concurrent(&b)); // the hull sweep's precondition
```

## Assertions

Each assertion forces the full traversal, not merely its construction:

- `let joined = &a | &b; assert_eq!(a.span(&b).hi(), &joined);` and
  `assert!(a.span(&b).lo().is_empty());`, which run the concurrent hull sweep;
- `assert_eq!(a.span_all([&b, &joined, &a]).hi(), &joined);`
- `assert_eq!([a.clone(), b.clone()].into_iter().sum::<Version>(), joined);`
  and the borrowed `collect` spelling;
- `assert_eq!(a.meet_all([&b, &joined]), &a & &b);`
- `assert!(a.shape().count() > DEPTH);`,
  `assert!(combine([&a, &b, &joined]).count() > DEPTH);`,
  `assert!(Clock::from_parts(half, a.clone()).shape().count() > DEPTH);`
  (move `half` only after its tick), `assert!(keeper.shape().count() > DEPTH);`
- the masked view against a concurrent version:
  `assert!((&joined / &keeper) == a);` and
  `assert!((&joined / &keeper).partial_cmp(&b).is_none());`

The invariant, in one sentence: the version algebra's shape walks, its
concurrent-pair hull, and its receiverless folds use no call-stack depth
proportional to the tree's depth.

## Calibration evidence

Verified on ox-east-1 at explore commit `d78c6129`:

- `crates/before/tests/l2_probe/main.rs::deep_fork_halves_are_stack_safe` (the
  construction above) and `deep_surfaces_are_stack_safe` (two hand-built
  sibling-tip spine parties) pass in 0.24 s and 0.48 s. The command was
  `on-illumos.sh <wt> 'unset CARGO_TARGET_DIR; cargo nextest run -p before
  --locked --test l2_probe -E "test(deep_)"'`.
- `deep_calibration_recursion_overflows` (ignored, run with
  `--run-ignored only`) walks one stack frame per level of the same version
  on a 2 MiB thread and aborts:
  `thread '<unknown>' (3) has overflowed its stack` /
  `fatal runtime error: stack overflow, aborting` (SIGABRT). This shows the
  depth is enough to catch a one-frame-per-level recursion at the test
  thread's default stack size.

I did not construct a recursive rewrite of a production walk and watch the
proposed test fail. The calibration rests on the minimal-frame recursion
above.
