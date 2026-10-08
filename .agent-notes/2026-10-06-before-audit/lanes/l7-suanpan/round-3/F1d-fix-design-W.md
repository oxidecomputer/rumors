<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), auditor-l7, round 3. -->

# F1d: replace suanpan's zero-range map with a written-position bitset (fix brief, fixer)

This is item 2 of the owner's F1b ruling: "The W branch, which flattens that counter. It tunes the constant on range-free inputs before review, for example by keeping the lowest level of marks in the digits' spare bits. It carries a model-based test of the bitset against a `BTreeSet` oracle."

Stack it on item 1's branch (`F1c-machinery-zero-range-fuel-ladder.md`): the fuel ladder must exist and pass at your base before you change suanpan, so every movement you cause lands in a committed number.

## Goal

suanpan's per-update cost must not depend on how many zero ranges an accumulator holds, so that `before`'s operations documented as `O(n)` in total input bytes are linear on every input, with no guarantee weakened and no constant made worse on inputs that never create ranges. The goal wins over the mechanism below. If the design here fails it, stop and report rather than adapting the goal.

## Root cause (from F1b, restated so this brief stands alone)

`ZeroRanges` (`crates/suanpan/src/accumulator/digits/zero_ranges.rs`) records known-zero intervals inside the stored prefix so that descending scans can skip them. Once two or more ranges coexist, it keeps them in a `BTreeMap`, and three of its operations cost `O(log R)` on paths that `before` drives once per boundary or per limb:

1. A lookup on every write: `remove_written`, called from `Digits::add_at`.
2. Insertion and removal at the top, once per sparse digit: `record_gap` on a jump above the top, `take_below` in the trim and in the sign scan.
3. Splits, for writes strictly inside a range: `remove_written`.

Item 1's ladder exposes each term with one family: F1, ±S, and OS respectively.

## Design W

W keeps no ranges at all. Instead it records which positions have been *written*, in a 64-ary hierarchical bitset.

**Representation.** Level 0 has one bit per digit position, packed into `u64` words. Level `k + 1` has one bit per word of level `k`, set exactly when that word is nonzero. The hierarchy grows on demand to cover the highest position marked.

**The invariant, which replaces the range invariant:**

- every unmarked position at or below `highest_nonzero` holds 0;
- no position above `highest_nonzero` is marked;
- position 0 and `highest_nonzero` are marked while the digit form is active;
- every summary bit is set exactly when the word it summarizes is nonzero.

A zero digit may be marked; only the converse is required. That asymmetry is what the tuning below exploits.

**Operations:**

- `mark(p)` sets the level-0 bit and climbs only while a word turns from empty to nonempty.
- `unmark(p)` clears the bit and climbs only while a word turns from nonempty to empty.
- `pred(p)` finds the largest marked position strictly below `p`. It climbs while the current word holds nothing below the index, then descends through the highest set bit of each word: `O(1 + log_64 d)` for a skipped distance `d`.

**Integration at each `ZeroRanges` call site at main:**

- `Digits::add_at` marks every position the write and its carry chain touch (the `remove_written(first_written, position)` site, `digits.rs:206`). `record_gap` (`digits.rs:174`) disappears: a jump leaves the skipped positions unmarked, and that is the range.
- `trim_high_zeros` (`digits.rs:237`) unmarks the position it leaves, then steps to `pred` of it instead of `take_below`.
- The sign scan (`digits/sign.rs:74`) unmarks each digit it clears. With a zero partial it steps to `pred(index)` instead of `take_below(index)`. Its final `clear` (`sign.rs:95`) becomes the scan's own unmarking.
- Activation and `reset` (`digits.rs:100`, `:154`) unmark through the old top and mark 0.
- `normalize` (`digits/normalize.rs:86`) clears the marks through the old top, then marks 0 and every nonzero digit of the rewritten prefix.

**Cost argument** (an argument, not a proof; the ladder measurements agree with it):

- A shift-0 write marks positions next to existing marks, so its climbs are bounded by its own operand length.
- `before`'s per-boundary folds are all shift-0 (`add_bigint` streams limbs from digit 0).
- A `pred` climb over a stretch of length `d` is paid by the write that created the stretch: in shift-0 workloads, a jump to position `p` costs `p / 2` input limbs.
- Unmarking happens only at positions a scan visits individually, each paid by the write that marked it.

suanpan's documented bounds (`amortized O(L log(W + 1) + G)` per update, and the comparison and `normalize` rows) all still hold, and W meets stronger ones: `O(L + G)` amortized for shift-0 writes, `O(1)` amortized comparisons, `O(W + Q)` normalize. Do not strengthen the public docs in this branch. Propose the stronger text in your report, as a decision for the owner.

**Reference implementation.** `crates/suanpan/src/accumulator/digits/written.rs` on `explore/l7-suanpan` (feature `l7-proto-w`, commits `31571595` and `c045c8ba`), with integration hooks under `cfg(feature = "l7-proto-w")` in `digits.rs`, `sign.rs`, and `normalize.rs`. It passes the whole suanpan suite and my pool model, and leaves every committed touch pin unchanged. It is prototype quality: rewrite it to this codebase's standards rather than lifting it.

## Tuning on range-free inputs (before review)

The acceptance criterion is "identical or improved, every movement measured", judged on item 1's control rows. Untuned W, measured on item 1's ladder (fuel per byte, `r` = 4,096):

| control row | A (current) | W untuned |
|---|---|---|
| F1 control decode | 1252.4 | 1343.5 (+7.3%) |
| F1 control `<=` | 2508.6 | 2688.4 (+7.2%) |
| F1 control `min_ticks` | 3674.6 | 3436.1 (−6.5%) |
| ±S control decode | 136.9 | 153.3 (+12.0%) |
| ±S control `<=` | 263.2 | 295.0 (+12.1%) |
| ±S control `min_ticks` | 224.8 | 270.8 (+20.5%) |
| OS control decode | 135.5 | 148.2 (+9.4%) |
| OS control `<=` | 254.5 | 284.8 (+11.9%) |
| OS control `min_ticks` | 208.1 | 249.4 (+19.8%) |

The overhead is marking and unmarking every written position. Candidates, in the order I expect them to pay; measure each on the whole ladder and keep what helps:

1. **Skip the mark when the written digit was already nonzero.** The invariant makes every nonzero digit marked, and `add_at` already reads the old digit. Dense workloads rewrite nonzero digits almost always, so this removes most bitset traffic on the controls.
2. **Mark a carry chain or a limb stream one word at a time.** One read-modify-write per 64 positions, instead of one per position.
3. **The owner's example: keep level-0 marks in the digits' spare bits.** Stored digits satisfy `|d| < 2^33`, so an `i64` digit has bits to spare, but every arithmetic read and write must then mask them. Measure it against candidate 1, which needs no masking. Report it either way, because the owner named it.
4. **Unmark lazily where the invariant allows.** A cleared position at or below the new top may stay marked. Positions above the top must be unmarked, because a later jump relies on them.

Record, for every row of item 1's table, A's pinned reading, untuned W, and each tuning step kept, in your report and in the re-pinning commit's message.

## The model-based test the owner requires

Test the bitset against a `std::collections::BTreeSet<usize>` oracle, as a proptest in the bitset module's sibling `tests.rs`:

- **Operations:** sequences of `mark(p)`, `unmark(p)`, range marks if you add them (candidate 2), and the clearing operations the integration uses. Apply each to both the bitset and the oracle.
- **After every operation:** compare `contains(p)` and `pred(p)` against `set.range(..p).next_back()` at probe positions: every touched position, each one plus and minus one, 0, word boundaries, and a position above the highest mark. Also assert the summary invariant at every level.
- **Generator:** build positions directly from a mixture: small positions; positions near the word and level boundaries 63, 64, 65, 4,095, 4,096, 262,143, 262,144 (the third level begins at `64^3 = 262,144`); clustered runs; and isolated high positions. A uniform draw over a wide range almost never exercises the climb.
- **Calibration:** show it fails under each of these reversible swaps, and record the failing assertion:
  - `pred` masking one bit too many or too few within a word;
  - `unmark` not propagating an emptied word upward;
  - growth that creates a new level without filling it from the level below.

Keep the digit-level invariant check (`assert_invariants` in the tests) after every operation of the existing digits tests, restated for W's positional invariant.

**Retiring `ZeroRanges`' tests.** Retire them only after the replacement demonstrates it catches what they caught (the doctrine for retiring an instrument). The range-specific mutants in round 2's inventory (an upper remnant dropped on a split, the One/Many branch, the compact guards) have no direct W analogue. Show that the model test and the touch property (if MB1 has landed) catch W's analogues: a skipped mark, a missing unmark at the top, and a wrong `pred`.

## Expected movement of item 1's counter

- **Sparse rows flatten.** Per byte, the largest size reads within about 1% of the smallest, and every sparse cell falls below its tripwire. Untuned, I measured F1 decode at 1,343.9 to 1,337.6 fuel per byte (A: 1,462.8 to 1,637.8), ±S `min_ticks` at 208.8 to 198.9 (A: 399.8 to 639.5), and OS `<=` at 241.1 to 237.2 (A: 364.1 to 525.6). The full W column is in item 1's table.
- **Control rows end identical or lower than A's pins** after tuning.
- **Re-pin** the moved cells in the commit that lands W. Name the mechanism and give each cell's old and new reading.

## Other numbers that must not move

- **Every committed touch pin**, in suanpan and in `before`'s meter suites. W changes no digit arithmetic. Round 2 measured identical per-program touch totals under A and W on 1,000 seeded random programs (`PROPTEST_RNG_SEED=20261007`).
- **The board**: 5,311 green, with only the baseline's two `count_display × heap` drift lines. The heap column must not regress, because the bitset adds `W / 64` words plus summaries, about 1.6% of digit storage, and removes the map's nodes.
- **Every wire and storage snapshot.** The stored representation is internal and never serialized.

## Stop conditions

Stop and report instead of committing when any of these occurs:

- A committed touch pin moves.
- After tuning, any control row of item 1's ladder would raise its ceiling. Report each row's residue with its mechanism; the owner rules on accepting a constant.
- A sparse row fails to flatten: its per-byte reading at `r = 4,096` exceeds its reading at `r = 64` by more than 1%.
- The bitset model test, or any committed test, fails for a reason you cannot trace to your own code.
- A documented bound or guarantee would have to weaken.
- A snapshot, a `protocol_overhead` byte count, or `BOOKMARK_FORMAT_VERSION` would change.
- The landing check deviates from `baseline.md`.

## Coordination

- Other branches touch the same files: the zero-shift fix (`fix/suanpan-zero-shift`, in `Shl` and the shift path), the reserve-hint fix (`fix/suanpan-reserve-digits-hint`, in `reserve_digits`), and the O1 brief from round 2 (`S2-comparison-fixed-point.md`, in the sign scan) if it is dispatched. Ask the coordinator which have landed, and rebase onto them before tuning, because each changes the fuel of the paths you measure.
- The touch-bound property (MB1), if landed, drives gap-split rounds through the zero-range code. It must hold unchanged under W.

## Owner's ruling on suanpan's public space promise under W

Keep the public promise. `lib.rs`, its README, and `normalize`'s rustdoc keep
"rebuilt skip metadata occupies O(`Q`) space". The fix makes it true in every
case: `normalize` drops the written-position bitset (back to the inline
state) instead of clearing it in place, at the cost of one reallocation if
two or more gaps return afterwards.

## Owner's ruling on the mask variant's exponent fits

The `Below::Mask(u64)` variant removes every heap rise against the fuel-ladder
commit, but 314 fitted heap exponents rose by 0.01 to 0.03 and 14 top-scale
per-byte readings by 0.1. The owner: "a tiny negligible increase like this is
not a big deal." The variant stands without further attribution.

## Owner's ruling on the W fix's remaining costs (question 70)

Question 70 asked whether the W fix's remaining small costs are an acceptable
trade. The mask variant removed the 94 heap rises above 0.1 B/B, including the
two that grew with input, and `BAND_PERCENT` has a working known-bad again
(each write recorded twice). What remained were seven near-tied heap
worst-case rankings flipping from `arming-train` to `wide-arming` (re-pinned
in `b9b7e953`) and the Opening family's dense control rows at +0.72% fuel,
inside the ladder's ±2% band. The owner ruled the small increases acceptable
and confirmed that this settles question 70.

## Owner's ruling on the time-bounds table (question 71)

Option 1: publish the table of amortized time bounds in suanpan's public
docs after a reviewer verifies every bound; a bound the reviewer cannot
verify goes back to the owner rather than being softened. The table was
derived before the inline mask, so it is re-derived against W's final code
(`b9ec1fbd`) first.

## Owner's ruling on the remaining log factor (question 112)

Option 1: accept the asymptotic bound amortized O(`A` log(`W` + 1) + `G`)
for adding an accumulator at a nonzero shift. The logarithm is base 64 of a
digit position, so it is at most 11 levels on 64-bit targets (6 on 32-bit).
In the owner's words: "if the asymptotic log is practically bounded at 11
on 64-bit targets, I'd call this effectively constant. I think we should
accept the theoretical asymptotic log bound." The table on
`docs/suanpan-time-bounds` already states that bound; the cached-maximum
candidate for removing the factor is not pursued.

## Owner's ruling on how the cost table counts growth (question 113)

Option 1: `G` and `W` count each update's span, its shift plus its
operand's width, as `docs/suanpan-time-bounds` states. A shifted operand
that cancels is charged for the positions its span reaches, even when the
working width does not change. The deposit path is not redesigned.
