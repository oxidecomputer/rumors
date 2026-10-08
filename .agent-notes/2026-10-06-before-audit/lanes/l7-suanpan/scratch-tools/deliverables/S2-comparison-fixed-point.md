# S2: a comparison that would rewrite the same two digits skips the rewrite

Kind: constant-factor brief (small, contained, simple; behavior-preserving; only touch counts change, and only downward).
Prototype: feature `l7-o1` on `explore/l7-suanpan` (commit "Add L7 O1 prototype", `crates/suanpan/src/accumulator/digits/sign.rs`). Illustration only.

## The current cost (verified)

`Digits::compact_until_order_known` (`crates/suanpan/src/accumulator/digits/sign.rs:57-100`) reads the top digit. When that digit is below the decision threshold (`|partial| < 3`) it clears the top, descends, and, after deciding one digit down, collapses: it clears that digit and re-deposits the partial through `add_at`. When the top digit is `+2` and the digit below lies in `[0, 2^31)` (or the mirror: `-2` over `[-2^31, 0)`), the partial `2 * 2^32 + d` reaches the digit limit, so `add_at` recenters it straight back to `(2, d)`. The collapse is a no-op on the representation, and the next comparison repeats it. Measured (`l7_probe_repeated_comparison_cost`): after the first read, each `cmp_zero` on `2 * 2^2048` costs 6 touches, against 1 on `1 * 2^2048` or `3 * 2^2048`.

## Proposed structure

In `compact_until_order_known`:

1. On the first iteration (index equals the old top), do not clear the top digit; remember that its clear is deferred and step down.
2. After the loop, if the clear was deferred:
   - if the scan decided exactly one digit down, and `Self::recenter(partial)` returns `(remainder, carry)` equal to `(digits[index], digits[old_top])`, return `(index, partial)` without writing anything: the collapse would have reproduced these two digits exactly;
   - otherwise clear the top digit now (one touch, the same touch the loop would have spent) and continue exactly as today.

Every other path performs the same writes and the same number of touches, merely with the top's clear moved after the descent. The no-op case drops from 6 touches to 2 (two reads). The prototype is about 20 lines.

## Why it is correct, and more obviously so

The early return happens only when the collapse is provably the identity on the stored digits: the recentering of the scanned partial equals the two digits it would overwrite. The decision index and partial returned are the ones the full path returns, so `cmp_zero` and `cmp_zero_stable_under` answer identically. `lowest_written` is unaffected: if the collapse would have lowered it, the digit at that index is zero below the old watermark, and the skipped write would have written zero there. The zero ranges can differ between the two paths but stay valid in both: the full path's `add_at` would split a range ending at the top if the digit below lies inside it, while the skip keeps that range, whose interior digit still holds zero (the identity case writes back zero there). The next comparison then takes the same two reads either way.

## Pins it moves (measured with the prototype)

- suanpan: none. All 70 suanpan tests pass, including every exact touch pin, and the pool model at 1,000 cases (`S/o1.log`).
- `before`: none among the touch instruments. 84 tests pass: the `meter`, `fold_skeleton`, and `answer_embedded` binaries and the board's library tests, with `--features suanpan/l7-o1` (`S/o1_before.log`).
- Board acceptance and the worst-case ranking pin, with `--features suanpan/l7-o1`: `amp-board: 5311 green / 0 red (5311 cells)`, identical to the baseline; the worst-case check reports exactly the two baseline drift lines (`count_display x heap` at the default and acceptance scales) and nothing else.
- Random programs: on 1,000 seeded programs from my touch property (`PROPTEST_RNG_SEED=20261007`), O1 never costs more: 890 programs cost the same, 110 cost fewer, 953 touches saved in total (`S/m3/touchcmp-*.txt`).
- New exact pin to add in `crates/suanpan/src/accumulator/tests/metered.rs`: repeated comparisons on `2 * 2^2048` cost 2 touches each after the first; on `1 * 2^2048` and `3 * 2^2048`, 1 each. The pin fails on the current code (6), which is the known-bad demonstration.

## Coverage

Value behavior is covered by the existing comparison and stability properties (`differential.rs`, `surface.rs`, the witnesses) and, in my calibration, by every value mutant of the sign scan that the suite catches today. The new pin protects the touch claim.
