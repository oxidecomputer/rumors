# Test brief D1: `min_ticks` heap on rising spines stays under the board ceiling

For a demonstrator who has not seen the auditor's explore branch. Defect record:
`defect-D1-min-ticks-heap.md` (same directory).

## Invariant (one sentence)

`Version::min_ticks` holds at most `HEAP_INTERCEPT_BYTES + MAX_HEAP_BYTES_PER_INPUT_BYTE * n`
(= `1024 + 20 n`) bytes of peak transient heap for an `n`-byte version, including on right spines
whose nested subtree minima strictly rise.

## Form

A new amplification-board family (the instrument of record for peak heap; see the module doc of
`crates/before/src/testing/meter/board.rs` and `tests/meter.rs`'s module doc, which leaves
ordinary peak-heap coverage to the board). Every version-bearing board operation is then judged
on it, and the `version_min_ticks` cell is the one expected to read red on base.

If the coordinator prefers a smaller blast radius, the alternative is one focused test in
`crates/before/tests/meter.rs` (which already installs `peak_alloc::PeakAlloc`) asserting the
same inequality for `min_ticks` alone at the sizes below; state in its doc comment why the board
does not cover it.

## Generator construction (builds valid values directly)

The *jump-entered rising spine* `JR(d, J)`: a right spine of `d` internal nodes whose preorder
leaves are `0, J + 1, J + 2, …, J + d`: the root's left leaf is `0`, its right child is a right
spine whose left leaves rise by one from `J + 1`, and the deepest node's right leaf is `J + d`.
Absolute heights; every internal node's subtree minimum is its left leaf, no two sibling leaves are
equal, so the tree is in normal form. Use `J = 2^40` (any `2^31 <= J < 2^288` reproduces; keep
`J` well inside that band). Encoded cost: one wide code for the jump, then about 5 bits per level.

Build it in the meter registry's construction language (`crates/before/src/testing/meter.rs`:
node `1 · gamma(base)`, leaf `0 · gamma(base)`, min-lifted bases): root `1·γ(0)`, leaf `0·γ(0)`,
then the right subtree's root `1·γ(J + 1)`, then for each further level a left leaf `0·γ(0)` and
an internal node `1·γ(1)`, ending with a left leaf `0·γ(0)` and the final right leaf `0·γ(1)`.
Check normal form with the existing registry tests' validators.

Register it as a `Shape` (e.g. `JumpRisingSpine`) and a version-only `FamilyId` with
`VERSION_BUNDLE_CELLS`, sized like `Harmonic` (a base depth constant scaled by the board's
`size` closure), following `FamilyId::Harmonic` in `registry.rs` and `board/family.rs`.

A plain rising spine (`J = 0`, leaves `0, 1, 2, …`) also breaches, but only on the high side of a
`Vec` doubling (39.4 B/B at `d = 2^k + 2`, 19.7 just below), so its verdict depends on where the
board's ladder lands; the jump-entered family reads about 122 B/B at every size and is the robust
witness.

## Exact assertion

The board's own per-sample heap check for the `version_min_ticks × <new family>` cell:
`peak <= HEAP_INTERCEPT_BYTES + MAX_HEAP_BYTES_PER_INPUT_BYTE * n`. For the focused-test
alternative, assert exactly that with the constants imported from
`before::testing::meter::board`.

## Expected failure on the base commit

`just amp-board-acceptance` (release) exits nonzero with the `version_min_ticks` cell for the new
family red on heap, at roughly 120 to 160 bytes per input byte. The auditor measured, with the
board's allocator accounting in release (verbatim):

```
JUMP min_ticks jump=2^40 n=4096 encoded_bytes=2571 peak=312624 ratio=121.60 board_ceiling=52444 RED
JUMP min_ticks jump=2^40 n=65536 encoded_bytes=40971 peak=5001264 ratio=122.07 board_ceiling=820444 RED
```

## Hazards for the demonstrator

- Adding a family can move the worst-case map (`just worst-cases-pin`). If any operation's worst
  family changes, stop and report it to the coordinator with the drift lines; do not re-pin.
- Other version operations may also go red on the new family. Each is a separate finding to report,
  not to fix here.
