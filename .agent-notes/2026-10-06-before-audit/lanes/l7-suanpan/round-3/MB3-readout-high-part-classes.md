<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), auditor-l7, round 3. -->

# MB3: exhaust the readout's final-carry classes, values and touches

Kind: machinery brief (builder). It answers lead 2: "the carry bound in `crates/suanpan/src/accumulator/digits/read.rs` puts [the high part] in [-3, 2], but the committed pin `nonzero_high_parts_cost_one_touch_in_each_sign` ... exercises only high part 1. Are the extremes reachable, and does any test read them (value and touches)?"

## Verdict on the lead

The extremes are reachable, every one, through public updates. All of them read back exactly today. None of their readout touches is pinned, and one class is reached by no committed test at all.

## The finite domain

`read_digits` (`read.rs:55` and following) ends its carry pass with a final carry `c` and a low part `M`, where `value = c * B^n + M`, `0 <= M < B^n`, `B = 2^32`. The module doc (`read.rs:5-7`) bounds `c` to `[-3, 2]`. The high part is `c` for `c >= 0`, and `|c| - [M != 0]` for `c < 0`. The reachable classes:

| class | high part | complement pass | readout touches over `d` stored digits | committed tests reaching it (census) | touch pin |
|---|---|---|---|---|---|
| `c = 0` | none | no | `d` | many | `held_width_rows_cost_the_held_digits` |
| `c = 1` | 1 | no | `d + 1` | yes | `nonzero_high_parts_cost_one_touch_in_each_sign` |
| `c = 2` | 2 | no | `d + 1` | `complete_surface_matches_bigint`, `stored_width_stability_is_sound` | no |
| `c = -1`, `M != 0` | none | yes | `2d` | many | none found (the negative-value check in `metered.rs` meters the subtraction, not the readout) |
| `c = -1`, `M = 0` | 1 | no | `d + 1` | `representation_invariants_hold_exhaustively`, `complete_surface_matches_bigint`, `carry_tie_streams_match_the_oracle`, `primitives::unsigned_*` | no |
| `c = -2`, `M != 0` | 1 | yes | `2d + 1` | yes | `nonzero_high_parts_cost_one_touch_in_each_sign` |
| `c = -2`, `M = 0` | 2 | no | `d + 1` | **none** | no |
| `c = -3` | 2 | yes | `2d + 1` | `complete_surface_matches_bigint`, `stored_width_stability_is_sound` | no |

`c = -3` with `M = 0` is impossible. A final carry of `-3` needs a last total in `[-2B - 2, -2B - 1]`, whose low digit is `B - 2` or `B - 1`, never 0. A high part of 3 is therefore unreachable, which is why one digit always holds it.

The census (verified at explore `b9f688a3`): explore feature `l7-high-log` logs every readout whose final carry is 2 or -3, or negative over an all-zero low part. I ran it over suanpan's whole committed suite (`cargo nextest run -p suanpan --features touch-meter,l7-high-log -E "not test(/l7_/)"`, 64 tests, all passed; log `target/l7m/high-suanpan.log` on the box, summary in `S/r3/leads1.log`). It found no readout with `c = -2` over a zero low part.

## Failure class it catches

The current code reads every class correctly; this brief closes a test gap, not a defect. Constructible failures that today's suite would miss or catch only by chance:

- A high part pushed twice, or not pushed, for `c = 2` or `c = -3`. No touch pin sees either; a missing push also corrupts the value, which the surface suite catches only in proportion to how often it reaches these classes.
- A mutant special to the zero-low negative path with high part 2, for example one that computes the high part as 1 whenever the complement pass is skipped. Every committed test passes it, because nothing reaches `c = -2` over zeros. It fails both value and touches on the construction below.

## Construction (verified at explore `b9f688a3`)

All five uncovered classes are built through public updates at `d` stored digits (I used `d` = 64 and 128, so the value is far outside the small representation). The explore test `l7_probe_extreme_high_parts` (`crates/suanpan/src/accumulator/tests/explore_l7.rs`, `explore/l7-suanpan`) passes with exactly these values and touches, in both `sign_biguint` and `sign_biguint_shl` (scaled shift 0, because digit 0 is written):

| class | construction (`A = 2^(32d) - 1`, the all-ones value) | value | touches |
|---|---|---|---|
| `c = 2` | add `A`, add `A`, add the repunit `sum over i < d of 2^(32 i)`: every digit holds `2^33 - 1` | `2A + repunit` | `d + 1` |
| `c = -3` | the negation of the previous value: every digit holds `-(2^33 - 1)` | `-(2A + repunit)` | `2d + 1` |
| `c = -1`, `M = 0` | subtract `A`, then subtract 1: digits `[-B, -(B - 1), ..., -(B - 1)]` | `-2^(32d)` | `d + 1` |
| `c = -2`, `M = 0` | the previous value; then subtract `(B - 1) * 2^(32(d - 1))` and `2^(32(d - 1))`, as one-word shifted subtractions: the top digit becomes `-(2B - 1)` | `-2 * 2^(32d)` | `d + 1` |
| `c = 2`, `M = 0` | the negation of the previous value | `2 * 2^(32d)` | `d + 1` |

Each construction asserts its premise, that `stored_digit_count() == d`, before reading. Without it, a change in recentering could rebuild the value in fewer digits and the test would pass without reaching its class.

## Test shape

Exhaust the table above as one table-driven test, `readout_high_part_costs_one_touch_in_every_carry_class`, in `crates/suanpan/src/accumulator/tests/metered.rs`. It subsumes `nonzero_high_parts_cost_one_touch_in_each_sign`: fold that test's two cases into the table rather than keeping two tests that state overlapping invariants.

- Each row gives a construction, the expected sign and magnitude, and the expected touch count as a function of `d`.
- Run every row at two widths (2,048 and 4,096 bits, as the existing pin does), through both readouts.
- Assert exact touches and the exact value.

The doc comment states the invariant: "The readout costs one touch per stored digit, one more per complemented digit, and exactly one for a nonzero high part, in every reachable final-carry class." Add one sentence giving the reason a high part of 3 cannot occur.

Where to stack it: the coordinator's branch `simplify/suanpan-read-high-part` (stacked on `audit/suanpan-readout-high-touch`) owns the existing pin and the one-digit simplification. The new test belongs on top of it, so the simplification is checked against every class before it lands.

## Calibration the builder should run

Two reversible swaps on the simplified readout, each of which the new test must fail and the committed suite (without it) passes:

1. In the negative branch, replace `-carry - i128::from(low_nonzero)` with `-carry - 1`. The `c = -1`, `M = 0` and `c = -2`, `M = 0` rows fail in value and touches. The committed suite may catch the first through the census tests; the second only the new test reaches.
2. In the nonnegative branch, push the high part twice when it is 2. Only the `c = 2` rows fail. Value tests in the committed suite would fail too, since the magnitude changes, so the decisive check here is the touch row. Report which of the existing tests catch it.

Report each swap's failing assertion and confirm `git diff` is empty after reverting.
