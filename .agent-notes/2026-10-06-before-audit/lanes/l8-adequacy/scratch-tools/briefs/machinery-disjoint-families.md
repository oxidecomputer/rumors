# Machinery brief: rebalance the family generators toward multi-input successes

## The gap

`arb_party_family` and `arb_clock_family` (`crates/before/src/testing/generators.rs:372-401`, functions at 376 and 384)
draw a receiver and 0 to 17 items with replacement from a pool of at most
four arbitrary parties. Measured over 20,000 draws: `join_all` succeeds in
5.88% of party families and 5.55% of clock families, and never at arity 2
or more (0 of about 17,700). The model differentials that consume them
(`party_join_all_matches_all_models`, `clock_join_all_matches_all_models`)
therefore check the success path only for a lone receiver or a single item.
Multi-input success is covered only by the fork-reunion laws, whose inputs are
the regular shares of `forks(width)`.

This is a reach gap without a demonstrated failure: no surviving mutant sits
on the success path. Brief it only if the owner wants the reach; it is cheap.

## Proposed generator

Keep the current arm (it drives the error path and conservation) and add a
disjoint-by-construction arm, chosen about half the time:

1. Draw an arbitrary nonempty party `P` (`arb_oracle_party_nonempty`) and an
   arity `k` from `arb_fold_arity`.
2. Split `P` into `k + 1` pieces by a random fork tree: repeatedly pick a
   piece by an `Index` and replace it with its two `fork` halves, skipping a
   piece that is a single leaf too fine to split if one arises. The pieces
   are pairwise disjoint and irregular in shape, unlike `forks(k)`'s balanced
   shares.
3. Optionally reshape further: replace piece `i` by `arbitrary_i.without(union
   of the other pieces)` when nonempty, so pieces come from independent
   shapes, not one fork lineage.
4. Shuffle by a drawn permutation; the first piece is the receiver.

For clocks, pair each piece with a version from the existing version pool.

## Expected effect

Success at every arity from 2 to 17 on about half the draws, with shapes the
fork laws never produce. Measure with the census
(`l8_census_party_family_success` on `explore/l8-adequacy`'s probe, or the
reusable census functions) before and after.

## Calibration

The current suite kills every value mutant in `fold.rs` and the party join
kernels, so there is no known-bad implementation to convict. A builder can
construct one: make the balanced fold's merge drop the right operand when its
first child is absent. The fork-reunion law's regular shapes may miss it; the
new arm should not.
