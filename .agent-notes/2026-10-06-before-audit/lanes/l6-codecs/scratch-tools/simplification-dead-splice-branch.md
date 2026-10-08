# Simplification brief: delete the unreachable early return in `SplitOutput::splice_continuation`

Kind: self-contained, behavior-preserving, no public API or format change.
Base: `main` at `58285ca5`.

## Current code

`crates/before/src/version/io/writer.rs:242-272`, `SplitOutput::splice_continuation`:

```rust
while cursor.position() < end {
    let internal_nodes = cursor.read_unary().expect(...);
    for _ in 0..internal_nodes { self.topology.push(false); }
    if cursor.position() > end {
        debug_assert_eq!(cursor.position() - 1, end);
        return;
    }
    self.topology.push(true);
    ...
}
debug_assert_eq!(cursor.position(), end);
```

The branch at lines 256-259 handles a unary read that crosses `end` by
consuming the final leaf's flag.

## Why the branch is unreachable

The only caller is `VersionWriter::copy_subtree_remainder`
(`writer.rs:570` and `:579`, reached from `tick.rs:794` and, through
`VersionSubtree::copy_remainder`, from `tick/raise.rs:360`), which passes
`end` as either the copied
subtree's end or `last_flag`, the final leaf's flag (`writer.rs:557`). The
range always starts just after the subtree's first leaf's payload, so it is a
sequence of whole units (a run of internal-node flags, a leaf flag, a
payload). The final leaf is the rightmost leaf in preorder, the right child of
a node whose left subtree's encoding ends with a payload, so its flag
*immediately* follows the penultimate leaf's payload, with no internal-node
flag in between. The loop therefore reaches `position == end` at the top of an
iteration and exits through its condition; no `read_unary` ever runs across
`end`.

**Evidence (verified).** The function itself is well exercised: a panic
probe at its entry (R31, `reach5.py` in the auditor-l6 scratch directory)
fails 16 committed tests (the `tick` suites, the algebraic laws, and
`split_output_splices_like_leaf_feeding`) plus my explore copy-path harness.
Yet an injected mutation that pushes a stray leaf flag inside the branch
(calibration W2, `mutate4.py`) survived all 695 committed `before` tests and
4000 harness cases, while two neighboring mutations on the same path (W1, W3)
failed 60 and 65 committed tests. The function runs; the branch never does,
which matches the structural argument above.

## Proposed structure

Delete the branch. Keep the post-payload
`assert!(cursor.position() <= end, ...)` and the closing
`debug_assert_eq!(cursor.position(), end)`. Without the branch, an `end` that
a future caller places inside a run of internal-node flags fails loudly at
that assert instead of returning early with the run half copied. Reword the doc
comment to say why the stop is exact:

> `end` is a unit boundary: the subtree's end, or its final leaf's flag when
> that leaf's narrow payload is held outside the output. The final leaf's flag
> immediately follows the previous payload, so the loop stops exactly there.

## Why the result is more obviously correct

The function then has one exit and one invariant (whole units until `end`).
The deleted branch implied that crossing `end` mid-run was a normal case,
which a reader must disprove to trust the copy.

## Coverage

`version::io::writer::tests::split_output_splices_like_leaf_feeding`,
`copy_subtree_remainder_matches_per_leaf_feeding`,
`collapse_after_a_splice_matches_per_leaf_feeding`, and every `tick` suite
(the only production caller) exercise the function. `just gate` must
reproduce the baseline; the board's `version_tick*` scan cells cannot move,
because the branch never executes.
