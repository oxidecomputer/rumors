# Machinery brief MB-2: `min_ticks` is the exact minimum over histories

Written by auditor-l3 for a builder who has not seen the explore branch.

## The claim, and why nothing tests it today

`Version::min_ticks` (`crates/before/src/version.rs:308-345`) promises "the minimum number of
ticks that could have produced this Version": a floor ("every sequence of fork, tick, and join
that could have yielded this version must have performed at least this many ticks") that some
history attains.

Every committed check of `min_ticks` compares it with another *computation* of the same
quantity: the tree oracle's base sum (`testing::oracles::tree::Version::min_ticks`) and the
function-space floor (`testing::oracles::function::min_ticks`), through `diff_ops` and
`version/measure/tests.rs`. Those differentials pin the implementation to the definition; none
tests the definition against the claim. The only history-side check is the law
`ticks_line_realizes_min_ticks` (`testing/laws/version_party.rs:98-108`), which covers a single
party ticking from the empty version.

## The failure class it catches

A definition of `min_ticks` that is not the minimum over histories, implemented consistently in
production and both oracles. Two constructible instances:

1. Overcount (floor violated): a definition that counts more than the base sum, for example
   one that omits a class of internal-minimum subtractions. A linear history then produces
   versions whose `min_ticks` exceeds the ticks in their causal past. Demonstrated by
   reporting the base sum plus one (see "Calibration evidence").
2. Undercount (floor not attained): a definition that counts less than the base sum, which the
   constructive history below cannot reach. Demonstrated by reporting the base sum minus one.

Both oracles would have to change in step for the differentials to stay green, which is exactly
the common-mode case these properties exist for.

## Instruments

Two properties; extend `testing::optrace` rather than adding a parallel world model.

### (a) The floor, over linear histories

Extend the optrace world (or add a sibling strategy in `testing/optrace.rs`) so that each clock
carries the set of tick batches in its causal past, as a map from batch id to weight:

- `Tick(i)`: one fresh batch of weight 1 into clock `i`'s set.
- `Ticks(i, n)`: one fresh batch of weight `n`; draw `n` as `2^s + l` with `s` in 0..80 and
  `l` in 0..4 so wide counts appear, and call `Clock::ticks(Count)`.
- `Fork(i)`: the child copies the parent's set.
- `Send(i, j)`: `i` gains a fresh batch (the send), then `j` gains `i`'s set and a fresh batch
  (the receive).
- `Absorb(i, j)`: `j` gains `i`'s set.
- `Sync(i, j)` and `Join(i, j)`: both (or the survivor) take the union.
- `ForeignTick(i, j)`: `i`'s party ticks a copy of `j`'s version, which `j` then absorbs; `j`'s
  set gains one fresh batch.

Assertion after every step, for every live clock:
`clock.version().min_ticks() <= sum of the weights in its set` (compare as `Count`).

### (b) Tightness, by a constructive single-seed history

For an arbitrary normal-form `tree::Version` (`generators::arb_oracle_version`, plus deep
co-generated versions if MB-1 lands), realize it from one seed:

1. Walk the normal-form tree in preorder, carrying the dyadic path to each node.
2. For a node with base `b > 0`: fork the seed party down the path (at each step `fork()` keeps
   the left half and returns the right; keep the half on the path, park the other), call
   `version.ticks(&party, b)`, then join every parked half back so the seed is whole again.
3. Sum the bases spent.

Assertions: the realized version equals the input version, and
`input.min_ticks() == spent`. Also assert the seed reassembles (`is_seed()`).

The argument that the construction is valid, for the test's doc comment: in preorder, when a
node's base is applied, the version is constant on that node's interval (only ancestors'
bases have been applied there), and the sibling interval, if already processed, has base
zero (normal form gives one zero-base child), so `fill` raises nothing and each tick's grow
raises exactly that interval by one.

## Where it belongs

`crates/before/src/version/measure/tests.rs` (or a `min_ticks` sibling tests file), with the
world extension in `crates/before/src/testing/optrace.rs`. Register both in
`src/testing/validation_index.rs` under the failure class above.

## Calibration evidence

Explore branch `explore/l3-events` at 30379f8d plus runtime-switched mutations of
`Count::min_ticks_for` (log `calib-batch2.log` in the auditor-l3 scratch directory):

- Reporting the computed value plus one: the floor property fails ("min_ticks 3 exceeds the 2
  ticks in the causal past after Send(0, 0)"), and both tightness properties fail.
- Reporting the computed value minus one: exactly the two tightness properties fail (a smaller
  value is still a floor, so the floor property correctly passes).
- Unmutated: both pass at 2000 cases and in the long run (see the coverage record).

A plausible implementation slip (skipping the close-count increment in `close_subtree`'s
`Close::Equal` arm) is caught in debug builds by the internal assertion in
`HeightPrefixes::settle` before either property reads the value, so it does not calibrate
these properties; the definition-level shifts above do.
