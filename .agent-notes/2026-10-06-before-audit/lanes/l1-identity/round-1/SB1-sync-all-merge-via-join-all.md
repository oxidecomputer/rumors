# Simplification brief SB1: `sync_all` merges through `join_all`

## Current code

`Clock::sync_all` (`crates/before/src/clock.rs:403-420`) merges the
participants with a private copy of `Clock::join_all`'s algorithm: a
`balanced_try_fold` over aliases of the other clocks with the same combine
closure, then a loop joining each group into an alias of `self`, mapping
either failure to `Overlap`. `Clock::join_all` (`src/clock.rs:292`, at `main` `97798357`) is
exactly that fold followed by exactly that loop.

## Proposed structure

```rust
// Merge aliases so an error leaves the original clocks unchanged.
let mut whole = self.dangerously_alias();
if whole
    .join_all(others.iter().map(|other| other.dangerously_alias()))
    .is_err()
{
    return Err(Overlap);
}
```

The rest of `sync_all` (dealing the merged party into `others.len() + 1`
shares) is unchanged.

## Why it is more obviously correct

The two forms perform the same joins in the same order on the same values:
`join_all` folds its iterator with `balanced_try_fold` and the identical
closure, then joins the groups into the receiver in arrival order, returning
`Err` on the first failure at either stage. The merged clock's party and
version are therefore identical, and every error path still returns
`Overlap` before any participant is touched (all work happens on aliases).
After the change, `sync_all`'s overlap semantics *are* `join_all`'s, so the
two cannot drift, and the reader checks one fold instead of two.

Optional second step, same reasoning: `Party::join_all`
(`src/party.rs:413-428`) and `Clock::join_all` are textually the same
algorithm over different types. A generic helper in `src/fold.rs` (for
example, `join_all_into(acc, iter, join)` returning the unabsorbed inputs)
would state the error-path partition property once, where MB1's
multiplicity check then guards a single implementation.

## Coverage

- `clock::tests::sync_all_reconciles_one_world` and the clock law groups
  cover success.
- The explore property `sync_all_matches_model`
  (`crates/before/tests/audit_l1/main.rs`) checks success against an
  independent model and failure (an aliased participant) for byte-identical
  untouched clocks; the board's `clock_sync_all` row pins the cost.
- No board ceiling should move: the same joins run. The builder should still
  compare the `clock_sync_all` row against `baseline.md`.
