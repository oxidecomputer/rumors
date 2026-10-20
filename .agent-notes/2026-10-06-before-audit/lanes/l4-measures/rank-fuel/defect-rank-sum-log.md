<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), agent audit-rank-fuel, for the before audit at e0956b25d. -->

# Defect: `Rank`'s `Sum` pays a logarithmic factor per summand once its accumulator holds two zero ranges

## Summary

`impl Sum<Rank> for Rank` and `impl Sum<&Rank> for Rank` document `O(k + n)`
time for `k` ranks whose binary widths sum to `n`. They add every summand
through `suanpan::Accumulator::add_shifted_limbs`, which `suanpan` documents
as amortized `O(L log(W + 1) + G)`. The logarithm comes from `ZeroRanges`
(`crates/suanpan/src/accumulator/digits/zero_ranges.rs`), an ordered map of
known-zero gaps. Every digit write consults it (`remove_written`), and every
write that lands above the highest digit plus one extends it (`record_gap`).
While at most one gap exists the map is stored inline and costs `O(1)`. Once
two or more gaps coexist, every write pays a `BTreeMap` lookup of
`O(log R)` for `R` gaps.

A wide summand whose numerator contains zero 32-bit digits creates one gap
per zero run as it is ingested, because `apply_limbs` skips zero pieces. Every
later summand then probes the tree. The total is `Θ((k + R) log R)`, with `R`
up to about `n/64`, which is superlinear against the documented bound.

Severity: low. The excess is a logarithmic factor. But the documented bound
disagrees with the `suanpan` cost model it rests on, and the board's own
`dense-suffix` family reaches the excess.

## Contract clauses

- `crates/before/src/rank.rs:987` and `:999`: "For `k` ranks whose binary
  widths sum to `n`, `O(k + n)` time and `O(n)` space, including the
  result."
- `crates/suanpan/src/lib.rs:92`: "Add or subtract `L` limbs | Amortized
  O(`L` log(`W` + 1) + `G`)"; `zero_ranges.rs:21`: "The ordered map adds
  logarithmic bookkeeping per lookup or edit."

For `k` one-limb summands, `suanpan`'s own model gives `O(k log n + n)`. The
two sources disagree; the measurements below side with `suanpan`'s.

## Evidence

**Deterministic counts.** These come from a probe counter, in
`probe-only.patch`, on each `ZeroRanges` operation made while the ranges are
a tree, summing `ceil(log2(R + 1))` over those operations. Logs:
`ranksum-tree.tsv`.

- *`comb-rank`, my adversary.* The first summand is `0.` followed by
  `2m + 1` 32-bit digits alternating 1 and 0, which gives `m` zero runs. Then
  come `8m` small integer ranks, each the rank of the seed party ticked 1 to
  7 times.
  - Tree operations per input byte stay at 0.551.
  - Summed tree depth per input byte is 2.53, 3.08, 3.64, 4.19, 4.74, 5.29,
    5.84, 6.39, 6.95, 7.50, 8.05, 8.60, 9.15 for `m = 16 … 65,536`: +0.551
    per doubling across 13 doublings.
  - Touch per byte stays at 0.661, and fill and allocation volume per byte
    are flat.
- *`dense-suffix`, the board's `rank_sum` cell.* The cell sums `DS(p, p)`'s
  rank with one small rank per encoded byte. The tree's size is 0, 0, 2, 3,
  7, 15, 31, 63, 127 for `p = 16 … 4,096`, and there are 0.86 tree
  operations per byte once it exists.

**Fuel.** Measured on wasm32 with Wasmtime 48.0.5, one fresh instance per
reading, through the scratch `fuelprobe`. Log: `fuel.tsv`.

- *`comb-rank`.* Fuel per byte is 558, 629, 676, 774, 821, 902, 888, 1,049,
  1,104, 1,116, 1,335, 1,351 for `m = 16 … 32,768`: rising with `log m`, in
  steps.
- *`dense-suffix` `rank_sum`.* Fuel per byte is 371 and 371 at `p = 16` and
  `32`, where no tree exists. It jumps to 546 at `p = 64`, exactly where the
  tree forms, then reads 573, 683, 771, 882 and 937 up to `p = 2,048`.
- *Rate.* Both families cost about 113 to 130 fuel per unit of summed
  `log2` tree size per byte. The same rate on unrelated inputs ties the
  growth to the tree.

## What the scope's fuel slope reflects

The scope's `rank_sum × dense-suffix` reading, 779 → 1,314 → 1,230 fuel per
byte, is this tree, which forms and deepens as the dense-suffix rank's
numerator gains zero digit runs with scale. Heap, scan, touch, and allocation
volume are flat (verified), and `Sum` performs no multiplication.

The non-monotone top of the scope's ladder (1,314 → 1,230) is consistent with
the tree's step structure. This part is inferred, not measured: Rust's
`BTreeMap` searches each node on its right edge linearly, so a probe's cost
depends on how full those nodes are as well as on the height, while my
counter uses a smooth `log2` model.

## Failure family

The excess appears whenever the accumulator holds two or more zero ranges
while summands keep arriving. A summand whose numerator has two or more zero
32-bit digit runs produces that state, as do summands landing at fresh high
shifts. It disappears when every numerator is dense in 32-bit digits (the
`freeze-parade` rank never forms a tree; `ranksum-tree.tsv`), or when at most
one range exists.

## Root cause

`ZeroRanges` records gaps *inside* a single operand's span. When
`apply_limbs` skips a zero piece, the next nonzero piece lands above
`highest_nonzero + 1`, and `record_gap` inserts a range. Such a gap needs no
range: the operand's input already paid to read those limbs, so a later
descending scan through them is already funded. The tree is needed only for
gaps no write paid for, the shift gaps between separate writes.

## Question for the owner (blocks the test brief)

Which of these should hold?

1. **Restate the bound.** Change `rank.rs:987` and `:999` to match
   `suanpan`'s model, for example `O(n + k log n)`. This is documentation
   only.
2. **Stop recording gaps inside one operand's span.** This is a `suanpan`
   change. It removes the tree in every case I measured, so the board's
   family and `comb-rank` become linear, and it benefits every accumulator
   caller. It keeps `zero_ranges.rs`'s amortization argument ("any position
   visited individually was previously written individually") intact in
   substance: `apply_limbs` already meters every yielded limb, zero halves
   included, so the operand's own read pays for any later scan of those
   positions. A residual `O(k log R)` would remain when summands land at many
   distinct shifts. I infer, without having built it, that distinct shift
   gaps number at most about `√(2n)`, because distinct exponents must sum to
   at most `n`. So the documented bound would still need either that
   qualifier or a proof that the residual is absorbed.
3. **Keep `Sum` off the zero-range bookkeeping entirely.** For example,
   `suanpan` could gain a nonnegative append path, or `Sum` could use a
   ripple-carry digit buffer, whose carries amortize to `O(1)` per limb for
   nonnegative addends. That would make `O(k + n)` true as written. Note that
   the board's `rank_sum` row takes a touch liveness floor of `k + 1`
   accumulator touches (`WHY_TOUCH_RANK_SUM`), so moving `Sum` off the
   accumulator also moves that floor.

My recommendation is option 2 now, a `suanpan` improvement with a wide
benefit, together with option 1's restatement for the residual, until
someone proves or refutes the residual bound.

## Test brief (provisional, by option)

None of the existing meters sees the excess: touches, allocation volume,
peak heap and scan are all flat.

- **Option 1** needs no failing test, only a corrected rustdoc.
- **Option 2.** The failing test belongs in `suanpan`, at the root cause.
  - *Invariant.* Adding one limb stream into an accumulator records no
    known-zero range inside the stream's own span.
  - *Form.* A property over arbitrary limb vectors that contain zero 32-bit
    halves, applied at an arbitrary shift to an empty accumulator and to a
    nonempty one.
  - *Assertion.* Check the zero ranges through the test-visible
    `ZeroRanges::assert_invariants` path or a test accessor. After the
    write, no range lies strictly inside `[shift/32, shift/32 + 2·limbs]`.
  - *File.* `crates/suanpan/src/accumulator/digits/tests.rs`.
  - *Base failure.* The limbs `[1, 1, 1]` at shift 0 already produce two
    ranges, `(0, 2)` and `(2, 4)`.
  - *Before-level witness.* A `before` band then needs a tree-work counter,
    which is a new instrument. See "Machinery" below.
- **Option 3.** The failing test is the before-level band with the tree-work
  counter, over `comb-rank`.

**Machinery: tree-work counter.**

- *Failure class.* A per-write `O(log R)` bookkeeping cost, invisible to
  touch, heap, and scan.
- *Design.* A second counter beside `suanpan::touch_meter::touches`, gated by
  the same feature, that adds `ceil(log2(R + 1))` per zero-range operation
  made in tree form. My prototype is in `probe-only.patch`
  (`PROBE_TREE_OPS`, `PROBE_TREE_LOG`).
- *Band.* `Rank` `Sum` over `comb-rank` at `m = 2^4 … 2^14` must keep its tree
  work per summand below a constant ceiling. Base reads above 5 per summand
  at `m = 16` and rises by about 1 per doubling.
- *Calibration.* The base itself is the positive control. Under option 2's
  repair the counter must read 0 on `comb-rank`. That is predicted, not
  measured, since I did not prototype option 2.
- *Why a new instrument.* No existing instrument sees this failure. It needs
  a validation-index entry (`src/testing/validation_index.rs`).

## API and formats

Options 1 and 2 preserve the public API and every format. Option 3 preserves
them too, but moves the board's `rank_sum` touch floor.

## Wider reach (lead, not examined)

The same per-write tree cost applies to every `suanpan` accumulator that
holds two or more ranges. `before`'s operations documented as `O(n)` that
stream writes into accumulators include `min_ticks`, `tick`, projection,
placement, `range_minima`, and the region reader. They could carry the same
hidden logarithmic factor on operands with zero digit runs. I did not examine
them; they belong to the lanes that own them.
