# F1b: root cause and candidate designs for linear `before` bounds over suanpan

Owner's goal (round-2 ruling, relayed): fix suanpan so `before`'s linear bounds hold unconditionally, without weakening any other guarantee, suanpan's documented bounds included. This record gives the root cause precisely, the input families that expose it, and three candidate designs measured on those families, with the guarantees each keeps or changes.

## Root cause, precisely

Every cost is in the bookkeeping that lets descending scans skip zero digits: `ZeroRanges`, an ordered map of open intervals (`crates/suanpan/src/accumulator/digits/zero_ranges.rs`). The digit arithmetic and the touch-metered work are linear; the map is not. Three map operations sit on paths that `before`'s linear operations exercise per boundary or per limb:

1. **A lookup on every write in `Many` mode.** `Digits::add_at` calls `ZeroRanges::remove_written` after every deposit (`digits.rs:206`). Once two or more ranges coexist, that is a `BTreeMap::range(..to).next_back()`, `O(log R)`, even when the write cannot touch any range. Exposed by family F1 (one wide sparse opening, then many word-sized deltas).
2. **Insertion and removal at the top.** Every jump above the top inserts a range (`record_gap`), and every trim or scan step that crosses one removes it (`take_below`). Both are `O(log R)` map operations, and they occur once per sparse digit. Exposed by family ±S (repeated wide sparse deltas and their negations): `R` inserts and `R` removals per `O(R)`-byte delta.
3. **Splits.** A write strictly inside a range must find and split it, `O(log R)`. Exposed by family OS (sparse value, then deltas landing on every interior digit).

The per-update `log(W + 1)` that suanpan documents is exactly these three terms; `before`'s `O(n)` claims silently assume they are constant.

## Input families (all measured with wasmtime fuel; see "Reproduction")

| # | Family | Shape | Size `r` grows |
|---|---|---|---|
| F1 | sparse opening | root base `sum_{i<r} 4 * 2^(64i)` over `64r` leaves alternating `+1`, `0` | leaves and opening width |
| ±S | sparse oscillation | 32 leaves alternating `S`, `0`, `S = sum_{i<r} 4 * 2^(64i)` | delta width |
| OS | odd split | 32 leaves cycling `S`, `S + D`, `S`, `0`, `D = sum_{i<r} 2^(64i+32)` | delta width |

Each has a dense control: the same widths and tree shapes with `S` replaced by a value whose `2r` digits are all nonzero. For F1 and ±S no zero range is ever recorded; the OS control keeps the sparse `D` (see the caveat under the tables).

## Candidate designs

- **A (current):** the ordered map.
- **T (tags):** a per-position tag ("written, or not inside a range"); a write consults the map only when it lands on an untagged position at or below the old top. Prototype: feature `l7-proto` on `explore/l7-suanpan` (`94f56560` and later).
- **W (written-position bitset):** no ranges at all. Every unwritten position at or below the top is zero; a 64-ary hierarchical bitset records written positions; scans step from a position to the nearest marked one below it (a predecessor query whose cost is one step per level climbed, `O(1 + log_64 d)` for a skipped distance `d`); a write marks its positions; a scan or trim unmarks the positions it leaves. Prototype: feature `l7-proto-w` (`crates/suanpan/src/accumulator/digits/written.rs`, `31571595` and the fast-path fix `c045c8ba`), measured as "W2".

All three pass the full suanpan suite (70 tests, every exact touch pin included) and my pool model at 2,000 cases with their own invariants checked after every step (`S/proto2.log`, `S/protoW1.log`).

## Measurements: fuel per input byte (`before` operations) or per limb (suanpan alone)

Sizes `r` = 64, 256, 1,024, 4,096, 16,384. Encoded sizes from `crates/before/tests/explore_l7.rs` (`S/build_r2.log`): F1 about `56r` bytes; ±S and OS about `512r` bytes.

A is the current code, T the tag prototype, W the written-bitset prototype ("W2" in my logs). Each cell lists the readings for `r` = 64, 256, 1,024, 4,096, 16,384, left to right (`min_ticks` on F1 stops at 4,096 to bound run time). `before` rows are fuel per input byte; the suanpan row is fuel per word update (F1) or per streamed limb (±S, OS).

Caveat on one control: the OS dense control densifies `S` but keeps the sparse `D` (nonzero only at odd digits). Accumulators that receive `D` on its own, namely tick's walk, the projection comparison's heights, and `min_ticks`' total, still record ranges there, so those rows grow under A and T even in that "control"; the running differences behind decode, comparison, join, and meet hold `S` under `D` and stay flat.

**F1 sparse** (fuel per input byte; suanpan row per word update or per limb), sizes r = 64, 256, 1024, 4096, 16384

| operation | A | T | W |
|---|---|---|---|
| suanpan alone (per limb or update) | 372, 441, 510, 510, 579 | 211, 211, 211, 211, 211 | 235, 235, 235, 235, 235 |
| decode | 1487, 1569, 1654, 1662, 1748 | 1295, 1295, 1297, 1301, 1304 | 1312, 1308, 1307, 1306, 1306 |
| `<=` | 1648, 1729, 1815, 1822, 1908 | 1438, 1436, 1439, 1442, 1446 | 1478, 1473, 1471, 1471, 1471 |
| `partial_cmp` | 1659, 1740, 1825, 1832, 1918 | 1448, 1446, 1449, 1453, 1456 | 1485, 1479, 1478, 1478, 1478 |
| join | 2844, 2919, 3003, 3010, 3097 | 2634, 2626, 2627, 2630, 2635 | 2672, 2660, 2658, 2656, 2657 |
| meet | 2505, 2583, 2668, 2675, 2761 | 2295, 2290, 2292, 2295, 2299 | 2333, 2324, 2322, 2321, 2321 |
| tick (seed) | 1716, 1810, 1911, 1933, 2033 | 1485, 1489, 1498, 1509, 1519 | 1502, 1493, 1491, 1490, 1490 |
| tick (half) | 2559, 2610, 2671, 2693, 2754 | 2421, 2420, 2429, 2440, 2450 | 2424, 2411, 2408, 2407, 2407 |
| projection `<=` | 3362, 3454, 3557, 3581, 3683 | 3185, 3189, 3202, 3217, 3230 | 3263, 3252, 3249, 3248, 3248 |
| `min_ticks` | 3695, 3690, 3695, 3702, - | 3692, 3684, 3684, 3687, - | 3438, 3426, 3424, 3423, - |

**F1 dense control** (fuel per input byte; suanpan row per word update or per limb), sizes r = 64, 256, 1024, 4096, 16384

| operation | A | T | W |
|---|---|---|---|
| suanpan alone (per limb or update) | 182, 182, 182, 182, 182 | 211, 211, 211, 211, 211 | 235, 235, 235, 235, 235 |
| decode | 1250, 1248, 1247, 1247, 1247 | 1286, 1283, 1282, 1282, 1282 | 1315, 1313, 1312, 1312, 1312 |
| `<=` | 1411, 1408, 1407, 1407, 1407 | 1428, 1425, 1424, 1424, 1423 | 1481, 1478, 1477, 1477, 1477 |
| `partial_cmp` | 1421, 1418, 1418, 1417, 1417 | 1438, 1435, 1434, 1434, 1434 | 1488, 1485, 1484, 1483, 1483 |
| join | 2604, 2598, 2596, 2595, 2596 | 2621, 2614, 2612, 2611, 2612 | 2672, 2665, 2663, 2662, 2663 |
| meet | 2266, 2262, 2260, 2260, 2260 | 2283, 2278, 2277, 2277, 2276 | 2334, 2329, 2328, 2327, 2327 |
| tick (seed) | 1444, 1438, 1437, 1436, 1436 | 1460, 1453, 1451, 1451, 1450 | 1514, 1508, 1506, 1505, 1505 |
| tick (half) | 2394, 2386, 2384, 2384, 2383 | 2393, 2384, 2382, 2382, 2381 | 2434, 2425, 2423, 2423, 2423 |
| projection `<=` | 3083, 3077, 3075, 3075, 3075 | 3151, 3145, 3143, 3143, 3142 | 3276, 3269, 3268, 3267, 3267 |
| `min_ticks` | 3670, 3664, 3662, 3662, - | 3677, 3671, 3669, 3669, - | 3437, 3431, 3429, 3428, - |

**±S sparse** (fuel per input byte; suanpan row per word update or per limb), sizes r = 64, 256, 1024, 4096, 16384

| operation | A | T | W |
|---|---|---|---|
| suanpan alone (per limb or update) | 1617, 2086, 2592, 3092, 3538 | 1186, 1504, 1842, 2161, 2457 | 391, 391, 391, 391, 391 |
| decode | 206, 233, 264, 295, 323 | 179, 196, 217, 237, 255 | 129, 127, 127, 127, 127 |
| `<=` | 206, 232, 263, 294, 322 | 178, 195, 216, 236, 254 | 128, 126, 126, 126, 126 |
| `partial_cmp` | 206, 232, 263, 294, 322 | 178, 195, 216, 236, 254 | 128, 126, 126, 126, 126 |
| join | 261, 286, 316, 347, 375 | 234, 249, 269, 288, 307 | 184, 180, 179, 178, 178 |
| meet | 217, 242, 273, 304, 331 | 190, 206, 226, 245, 264 | 140, 136, 135, 135, 135 |
| tick (seed) | 315, 371, 433, 496, 552 | 259, 296, 337, 376, 413 | 158, 156, 155, 155, 155 |
| tick (half) | 342, 399, 465, 530, 589 | 284, 321, 364, 405, 444 | 179, 175, 174, 174, 174 |
| projection `<=` | 515, 598, 691, 785, 868 | 433, 488, 549, 609, 664 | 283, 279, 278, 278, 278 |
| `min_ticks` | 423, 493, 575, 660, 735 | 334, 372, 419, 466, 510 | 207, 199, 197, 196, 196 |

**±S dense control** (fuel per input byte; suanpan row per word update or per limb), sizes r = 64, 256, 1024, 4096, 16384

| operation | A | T | W |
|---|---|---|---|
| suanpan alone (per limb or update) | 508, 506, 505, 506, 506 | 568, 566, 565, 565, 565 | 730, 730, 730, 730, 730 |
| decode | 140, 139, 138, 138, 138 | 144, 143, 142, 142, 142 | 155, 153, 153, 153, 153 |
| `<=` | 139, 138, 137, 137, 137 | 143, 142, 141, 141, 141 | 154, 152, 152, 152, 152 |
| `partial_cmp` | 139, 138, 137, 137, 137 | 143, 142, 141, 141, 141 | 154, 152, 152, 152, 152 |
| join | 195, 191, 190, 190, 190 | 199, 195, 194, 194, 194 | 209, 206, 205, 204, 204 |
| meet | 151, 148, 147, 147, 147 | 155, 152, 151, 150, 150 | 166, 162, 161, 161, 161 |
| tick (seed) | 177, 175, 174, 174, 174 | 185, 183, 182, 182, 182 | 206, 204, 203, 203, 203 |
| tick (half) | 198, 195, 194, 194, 194 | 207, 204, 203, 202, 202 | 228, 225, 224, 224, 224 |
| projection `<=` | 313, 309, 309, 308, 308 | 324, 321, 320, 319, 319 | 356, 353, 352, 352, 352 |
| `min_ticks` | 238, 231, 229, 228, 228 | 257, 249, 247, 246, 246 | 278, 270, 268, 268, 268 |

**OS sparse** (fuel per input byte; suanpan row per word update or per limb), sizes r = 64, 256, 1024, 4096, 16384

| operation | A | T | W |
|---|---|---|---|
| suanpan alone (per limb or update) | 771, 927, 1078, 1212, 1353 | 684, 787, 882, 963, 1054 | 393, 392, 392, 392, 392 |
| decode | 153, 160, 169, 178, 187 | 147, 152, 157, 162, 168 | 129, 127, 127, 127, 127 |
| `<=` | 152, 160, 168, 177, 186 | 146, 151, 156, 161, 167 | 128, 126, 126, 126, 126 |
| `partial_cmp` | 152, 160, 168, 177, 186 | 146, 151, 156, 161, 167 | 128, 126, 126, 126, 126 |
| join | 204, 210, 218, 226, 235 | 198, 201, 206, 211, 216 | 180, 176, 176, 175, 175 |
| meet | 158, 164, 173, 181, 190 | 152, 156, 161, 166, 171 | 134, 131, 130, 130, 130 |
| tick (seed) | 210, 228, 247, 264, 282 | 199, 209, 221, 231, 243 | 162, 160, 159, 159, 159 |
| tick (half) | 235, 252, 273, 292, 311 | 222, 232, 244, 256, 268 | 183, 179, 178, 178, 178 |
| projection `<=` | 461, 525, 596, 667, 731 | 401, 442, 489, 534, 577 | 283, 280, 279, 278, 278 |
| `min_ticks` | 359, 415, 481, 548, 609 | 304, 338, 380, 421, 460 | 193, 187, 185, 185, 185 |

**OS dense control** (fuel per input byte; suanpan row per word update or per limb), sizes r = 64, 256, 1024, 4096, 16384

| operation | A | T | W |
|---|---|---|---|
| suanpan alone (per limb or update) | 385, 384, 383, 383, 383 | 430, 428, 428, 428, 428 | 520, 519, 519, 519, 519 |
| decode | 130, 129, 128, 128, 128 | 133, 132, 131, 131, 131 | 139, 138, 137, 137, 137 |
| `<=` | 129, 128, 127, 127, 127 | 133, 131, 130, 130, 130 | 138, 137, 136, 136, 136 |
| `partial_cmp` | 129, 128, 127, 127, 127 | 133, 131, 130, 130, 130 | 138, 137, 136, 136, 136 |
| join | 181, 178, 177, 177, 177 | 184, 181, 180, 180, 180 | 190, 187, 186, 186, 186 |
| meet | 135, 133, 132, 132, 132 | 139, 135, 135, 135, 135 | 144, 141, 141, 141, 141 |
| tick (seed) | 197, 206, 216, 226, 236 | 192, 196, 202, 207, 213 | 180, 178, 177, 177, 177 |
| tick (half) | 205, 210, 217, 224, 232 | 205, 206, 210, 214, 218 | 201, 198, 197, 197, 197 |
| projection `<=` | 370, 396, 427, 458, 486 | 350, 366, 386, 406, 424 | 317, 314, 313, 313, 313 |
| `min_ticks` | 288, 313, 344, 376, 405 | 272, 285, 305, 325, 344 | 228, 222, 220, 219, 219 |

### Reading the tables

- **A (part (a) of the request):** every operation grows per byte on all three sparse families, the ones previously only inferred included: `partial_cmp`, join, meet, tick by either party, the projection comparison, and `min_ticks` (on ±S and OS). On F1 the growth is 9% to 18% over the five sizes; on ±S it is 44% to 74% (`min_ticks` 423 to 735). The dense controls are flat. `min_ticks` on F1 is flat because that family's wide opening enters `HeightPrefixes`, not the `answer` accumulator; ±S and OS reach `answer`.
- **T:** flat on F1 apart from a residue of under 3% (the opening's `r` range insertions), but still growing on ±S (decode 179 to 255) and OS (decode 147 to 168). Tags remove only the first of the three map costs.
- **W:** flat or slightly falling on every family and operation. On sparse inputs at the largest size it is 1.3x (F1 decode) to 3.7x (±S `min_ticks`) cheaper than A. On range-free inputs it mostly costs more: from 2% (F1 dense tick, half party) to 18% (±S dense `min_ticks`) for `before` operations, with one row cheaper (F1 dense `min_ticks`, 6%), and 29% (word updates) to 44% (dense limb streams) for suanpan alone, the cost of marking and unmarking every written position.


## Guarantees each candidate keeps or changes

| | A (current) | T (tags) | W (written bitset) |
|---|---|---|---|
| suanpan time, `L`-limb update | amortized `O(L log(W+1) + G)` | same | amortized `O(L + G)` for writes landing next to existing marks (every shift-0 write); `O(L + G + h)` otherwise, `h <= log_64(W+1)` (at most 11 on 64-bit, 6 on 32-bit): strictly stronger than the documented bound |
| suanpan time, comparison | amortized `O(log(W+1))` | same | amortized `O(1)`: each predecessor climb is charged to the write that created the skipped stretch |
| suanpan time, `normalize` | `O(W + Q log(Q+1))` | same | `O(W + Q)` |
| `before`'s linear operations | `O(n log n)` on ±S, OS, F1 | still `O(n log n)` on ±S and OS (measured) | `O(n)` on all three families (measured), with an argument that every climb is paid by input in shift-0 workloads (below) |
| retained space | digits plus up to `W/2` map entries (BTreeMap nodes: tens of bytes each, so up to about twice the digit storage for sparse values) | A plus one tag per position (`Vec<bool>` here, 12.5% of digit storage; zero with spare digit bits) | digits plus `W/64` bitset words and their summaries (about 1.6% of digit storage); no map |
| peak memory during growth (the measures lane's O6 abort) | digit `Vec` doubling | unchanged | unchanged for digits; removes the map's allocations. The O6 peak needs a separate growth policy (exact reservation for large growth) |
| touch counts | baseline | identical: every committed pin, and per-program totals on 1,000 seeded random programs | identical: every committed pin, and per-program totals on the same 1,000 programs (`PROPTEST_RNG_SEED=20261007`, `S/m3/touchcmp-*.txt`); not proven for all sequences |
| representation invariants | disjoint ranges with zero interiors, within the stored prefix | A's plus the tag invariant | replaced by a positional invariant: unmarked at or below the top implies zero; nothing marked above the top; position 0 and the top marked; summaries exact |
| constant factors (measured) | baseline | cheaper than A on sparse inputs; 0% to 16% dearer on range-free inputs | 1.3x to 3.7x cheaper than A at the largest sparse sizes; up to 18% dearer on range-free inputs for `before` operations (6% cheaper for `min_ticks` on F1's dense control), and 29% to 44% for suanpan alone (marking every written position; tunable, for example by keeping level-0 marks in the digits' spare bits or skipping the mark when the word already holds it) |
| code | `zero_ranges.rs` (249 lines) | A plus about 60 lines of tag maintenance across five files | `written.rs` (about 200 lines) replacing `zero_ranges.rs`; `Digits` gains three unmark or clear calls |

### Why W is linear for `before`

`before`'s per-boundary folds are shift-0 deposits (`add_bigint` streams a magnitude's limbs from digit 0). A mark at position `p` climbs a level only if the aligned block of `64^j` positions containing `p` held no mark, and the block containing position 0 always holds one, so a shift-0 write's climb is at most `log_64 p`, which its own `p / 2` limbs pay for. Unmarking happens only at positions a scan visits individually (each paid by the write that marked it), and a predecessor climb over a stretch of length `d` costs `O(1 + log_64 d)`, paid by the write that created that stretch: in shift-0 workloads, a jump to position `p` costs `p / 2` input limbs. The shifted deposits that `before` makes (`Rank` addition and summation) land at offsets their operands' binary widths already pay for; integration's shifted adds sit under its `O(M(n))` bound. This is an argument, not a proof; the measurements are consistent with it on all three families.

### What W does not fix

A shifted write far from any mark costs `O(log_64 W)` climbing with `O(1)` input. suanpan's documented bound allows `log(W+1)`, so this stays inside the contract, but an operation built from many such writes is `O(n log_64 W)`, not `O(n)`. None of `before`'s linear operations does this.

## The adversary for T (part (b) of the request)

My round-1 option-2 reasoning (a digit oscillating through zero inside the span) does not defeat T: once written, a position stays tagged while it is zero, so the oscillation never consults the map. What defeats T is the other two map costs. ±S makes every delta insert `r` ranges at the top and remove them again; OS adds a split at every interior digit. Both families grow under T (tables above). W has neither cost: there is no map, a split is a mark, and a top insertion or removal is a mark or unmark at a block that is usually already nonempty.

## Recommendation

W, if the owner accepts a constant of up to 18% on `before` inputs that never create ranges (29% to 44% on suanpan-only microbenchmarks), before any tuning. It removes the ordered map, restores `before`'s linear bounds on every family I could construct, strengthens suanpan's documented per-update bound, lowers worst-case retained space, and leaves every committed touch pin unchanged. T is not sufficient: it removes only the first of the three map costs.

## Reproduction

- Prototypes: `explore/l7-suanpan`, features `l7-proto` (T) and `l7-proto-w` (W; W takes precedence if both are on).
- Fuel matrix: `crates/before/wasm32-pins/guest/src/l7_builders.rs` (families), `guest/src/checks.rs` (`l7_fuel`, modes `100 * family + op`), harness test `zz_l7_fuel_matrix` (`#[ignore]`; env `L7_FAMILIES`, `L7_OPS`, `L7_SIZES`). Guests built with `--features suanpan/l7-proto` or `suanpan/l7-proto-w` into separate target directories; the harness binary is run directly so jobs do not contend for cargo's lock. Logs: `S/m3/`.

## Owner's ruling

Adopt design W, the written-position bitset, in this order:

1. A machinery branch first: a deterministic counter, or a committed fuel
   pin, that grows on the F1 families today, so the fix moves a committed
   number.
2. The W branch, which flattens that counter. It tunes the constant on
   range-free inputs before review, for example by keeping the lowest
   level of marks in the digits' spare bits. It carries a model-based test
   of the bitset against a `BTreeSet` oracle.
