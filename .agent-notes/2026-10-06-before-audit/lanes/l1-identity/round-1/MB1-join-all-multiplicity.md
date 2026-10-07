# Machinery brief MB1: `join_all` error paths must return each region exactly once

## The failure class it catches

`Party::join_all` and `Clock::join_all` promise that on error they return
"every input region not absorbed into `self`, without dropping any region",
where "returned parties may be unions of inputs"
(`crates/before/src/party.rs:384-388`, `crates/before/src/clock.rs:255-258`).
The coherent reading, and what `fold::balanced_try_fold` states for its own
output ("represent every unconsumed input exactly once", `src/fold.rs:16-17`),
is that the inputs are *partitioned*: each input lands in the receiver or in
exactly one returned party.

A constructible defect breaks this without dropping anything: the error path
returns some region twice. The caller then holds two live handles to one
identity, which is a linearity violation that the library itself
manufactured. Every existing conservation check compares *unions* of regions
(`oracle_union_all` in `src/party/tests.rs:37-45` and its clock twin
`oracle_clock_union` in `src/clock/tests.rs:42-51`, and the laws
`party_join_all_err_conserves_the_region_union` /
`clock_join_all_err_conserves_the_region_union`, which check only that the
union covers). A union cannot see a duplicate.

Verified by calibration (verbatim from `calib-run-3.log`, exact string swaps,
reverted, `git diff` empty):

- M19, `src/party.rs` final loop of `Party::join_all`:
  `let mut uncombined = vec![back];` became
  `let mut uncombined = vec![back.dangerously_alias(), back];`.
  Existing suite (`--lib --test forks_count`): 610 run, all pass except the
  explore suite's `join_all_failure_conserves_multiplicity`.
- M20, the same swap in `Clock::join_all` (`src/clock.rs`): passes the
  entire existing suite; caught only by the explore suite's
  `clock_join_all_conserves`.

## Which instrument it extends

Extend the two existing oracle comparisons rather than adding a parallel
test. In the `Err` arm of `assert_join_all_matches_all_models` in
`src/party/tests.rs` and in `src/clock/tests.rs`, keep the union assertions,
and add one assertion on the *production* result: pointwise multiplicity is
conserved.

> For every point `x` of `[0, 1)`, the number of parties among
> `{receiver before} ∪ inputs` that own `x` equals the number among
> `{receiver after} ∪ returned` that own `x`.

The oracle trees are already in hand (`to_oracle_party` of each). A
multiplicity comparison over `tree::Party` values is a short recursive helper
(test-only, bounded depth, routed through `recurse::descend!` as the crate's
hard rule requires for deliberately recursive test helpers):

- at a node where every tree is a leaf, compare the counts of `Leaf(true)` on
  both sides;
- otherwise split every tree into its two children (a leaf splits into two
  copies of itself) and require agreement on both halves.

A natural home is `tree::Party` itself, as
`#[cfg(test)] pub(crate) fn same_multiplicity(lhs: &[&Party], rhs: &[&Party]) -> bool`,
beside the existing test-only `union`.

For the clock helper, versions need no multiset treatment: version join is
idempotent, so the existing joined-version equality is the right check there.

## Generators

Unchanged. `arb_party_family` and `arb_clock_family` already draw items from
a small pool, so aliases (the case that makes duplication invisible to unions)
appear at nearly every arity. The two fixed populations in each module
(`join_all_preserves_regions_on_overlap`,
`join_all_agrees_with_oracle_on_forked_and_aliased_populations`) also feed
the helper.

## Exact assertion

```rust
assert!(
    tree::Party::same_multiplicity(&before_refs, &after_refs),
    "join_all returned a region more or fewer times than it received it",
);
```

with `before_refs` = the initial receiver's tree plus every input's tree, and
`after_refs` = the final receiver's tree plus every returned party's tree.

## Calibration the builder must reproduce

1. Base: the extended helpers pass.
2. Apply M19, run `cargo nextest run -p before --all-features --lib -E 'test(join_all)'`:
   `party::tests::party_join_all_matches_all_models` (or the fixed
   population test) fails with the new message. Revert.
3. Apply M20: `clock::tests::clock_join_all_matches_all_models` fails. Revert.
4. Confirm `git diff` is empty.

This extends the focused behavioral properties, an instrument the validation
index (`src/testing/validation_index.rs`) already lists, so the index needs
no new row.

## Owner's ruling

State the multiplicity contract in the rustdoc. The `# Errors` sections of
`Party::join_all` and `Clock::join_all` promise that no region is dropped or
duplicated: each point is owned as many times across `self` and the
returned parties as it was across `self` and the inputs. That is the
property that keeps the linearity of parties intact through a failed join,
and it is what the extended model tests check.
