<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), auditor-l7, round 3. -->

# F1c: a committed fuel ladder over the zero-range families (machinery brief, builder)

This is item 1 of the owner's F1b ruling: "A machinery branch first: a deterministic counter, or a committed fuel pin, that grows on the F1 families today, so the fix moves a committed number." Item 2 (`F1d-fix-design-W.md`) stacks on this branch.

## Goal

Commit a deterministic reading that, today, rises per input byte as the F1 families grow, so that the W fix visibly moves it, and that cannot pass vacuously. The goal wins over any mechanism below: if a mechanism here turns out not to serve it, say so in your report and do what serves the goal.

The reading must:

- grow with size on the sparse families today, through the suanpan map work behind F1;
- stay flat on a dense control of the same shape and widths, so the growth is attributable to recorded zero ranges and nothing else;
- carry a liveness floor derived from irreducible work;
- fail on a known-bad variant that amplifies the mechanism, and move when the mechanism is removed.

## The failure class it catches

The defect is per-write or per-range bookkeeping in suanpan whose cost grows with the number of zero ranges an accumulator records. It surfaces in `before` operations documented as `O(n)` in total input bytes as per-byte cost rising with size. Today that bookkeeping is the ordered map in `crates/suanpan/src/accumulator/digits/zero_ranges.rs`. It has three `O(log R)` terms, one per family below:

| family | map term it exposes | where |
|---|---|---|
| F1 | a map lookup on every write once two or more ranges coexist | `remove_written`, called from `Digits::add_at` |
| ±S | an insertion and a removal at the top for every sparse digit | `record_gap` and `take_below` |
| OS | a split for every write strictly inside a range | `remove_written` |

The contract clauses at stake are `before`'s complexity rows:

- `Version::decode`: "`O(n)` in total input bytes" (`crates/before/src/version.rs:1088-1092`);
- directional comparison `<=`: "`O(n)` in total input bytes; `O(|a| + |b|)`" (`version.rs:81-85`);
- `Version::min_ticks`: "`O(n)` in total input bytes" (`version.rs:322-324`).

The crate page (`crates/before/src/lib.rs`) calls every asymptotic claim a hard guarantee.

## Why fuel, and why the fuzz-fit harness

Only an external work measure sees the map. The alternatives, in the order the coordinator named them:

- **The board's counters** (heap, scan bits, digit touches) never see map work. Across 1,000 seeded random programs, touch totals are identical under the current map, the tag prototype, and the bitset prototype (round 2, `round-2/F1b-design-evidence.md`). The board's growth judge also allows an exponent up to `MAX_SCALING_EXPONENT = 1.15` (`crates/before/src/testing/meter/board/ceilings.rs:13`), and a log factor fits as an exponent of about 1.03. So even a fuel currency on the board would not flag F1.
- **The suanpan touch meter** could count map operations only by modeling their cost, and each design would then report its own work. A W implementation that undercounted its climbs would flatten a modeled counter without fixing anything. Fuel is counted by the wasm runtime, outside the code under test, so the cheapest way to flatten it is to remove the work.
- **The fuelscape** enforces nothing by design (its justfile recipe and `testing::validation_index` say so).

The fuzz-fit harness (`crates/before/fuzzfit/`) is the existing enforced fuel instrument. It provides the release wasm guest with one public operation per export; Wasmtime fuel (deterministic for fixed guest bytes); the compiler pin `PINNED_RUSTC` (`harness/src/bands.rs:172`), checked by its enforce suite; and a calibration binary (`just fuzzfit-calibrate`). Its numeric suite (`harness/tests/numeric.rs`) already runs deterministic sweeps for "public paths whose work does not reach the board counters". Your ladder is the same kind of check with pinned readings instead of growth limits, so give it its own test file, `harness/tests/zero_range_ladder.rs`, with its own module doc.

Do not register the families as board `Shape`s. `every_shape_is_cited_by_a_family` (`crates/before/src/testing/meter/registry/tests.rs:111`) would then require a board family, which adds every compatible board cell and can move worst-case rankings (a stop condition), while the board still could not see this work.

## The families

Build them on the host from the canonical version grammar, in a module of the harness (for example `harness/src/zero_range_families.rs`), so the calibration binary can reuse them. The grammar, as `crates/before/src/testing/meter.rs` writes it:

- A node is one flag bit (1 internal, 0 leaf) followed by the Elias gamma code of `value + 1`: for a mantissa of `k` bits, `k - 1` zero bits and then the mantissa, most significant bit first.
- Children follow their parent in preorder. Wrap the finished bits as `before::testing::meter::Encoding { bits, bytes }` and call `.version()`.
- Assert that every built value round-trips through `Version::decode`, so a construction slip fails loudly instead of measuring a different shape.

`crates/before/wasm32-pins/guest/src/l7_builders.rs` on `explore/l7-suanpan` implements all of this. Read it as illustration, not as code to copy.

Wide values, for size parameter `r`:

- `S(r) = sum over i < r of 4 * 2^(64 i)`: one nonzero 32-bit digit, then one zero digit, repeated. Every zero digit becomes a recorded range.
- `D(r) = sum over i < 2r of 4 * 2^(32 i)`: the same width with every digit nonzero, for the controls. It records no ranges.
- `O(r) = sum over i < r of 2^(64 i + 32)`: nonzero exactly at the odd digits.

Interior nodes have base 0 except where the table gives a base. Let `T(depth, bottom)` be the complete binary tree of base-0 nodes whose `2^depth` bottom nodes each have `bottom` as their two children (so `T(0, bottom)` is one node). Its top node is the version's root and carries the root base from the table. For example, F1 at `r = 64` is `T(11, ...)`, with 2,048 bottom nodes and 4,096 leaves.

| family | root base | tree | control (same shape, no ranges anywhere) |
|---|---|---|---|
| F1 | `S(r)` | `T(log2(64 r) - 1, leaves 1 and 0)`: `64 r` leaves | root base `D(r)` |
| ±S | 0 | `T(4, leaves S(r) and 0)`: 32 leaves | `S` replaced by `D(r)` |
| OS | 0 | `T(3, group)`, where a group is a left node of base `S(r)` over leaves 0 and `O(r)`, then a right node of base 0 over leaves `S(r)` and 0: heights `S, S + O, S, 0` | `S` and `O` both replaced by `D(r)` |

The OS control must densify `O` as well as `S`. Round 2's control kept the sparse `O`, and its comparison and `min_ticks` rows grew because the per-side heights still recorded ranges.

**The comparison operand.** Take the family's version with its root base set to 0 and its last bottom node (on the rightmost path) replaced by a leaf 0. Call it `u`. Then `u < v`, and `u <= v` must walk the whole structure.

- On F1 the running difference between `u` and `v` holds the opening `S(r)`. That puts the map lookup inside the comparison.
- Do not use `Version::new() <= v`. In normal form the minimum of `v` is its root base, so that comparison has a constant-time answer. Today's linear walk there is removable work, and pinning it would lock in an inefficiency.
- Do not use an operand that keeps the opening, such as only the lowered end. Then both sides hold `S`, the difference never does, and F1's `<=` row grows by only 1.7%. I measured this.

## Operations and kernels

All three are existing exports of `crates/before/fuzzfit/guest/src/lib.rs`:

- **decode:** stage `v.encode()`, then `ff_version_decode(dst)`.
- **`<=`:** decode `u` into one register and `v` into another (unmeasured), then measure `ff_version_le(u, v)`. Assert that it returns 1, and that `ff_version_le(v, u)` returns 0.
- **`min_ticks`:** `ff_count_from_version(dst, v)`, which computes the count into a register. Do not use `ff_version_min_ticks`. It renders the count in decimal, which is superlinear: through that kernel even the dense controls grow 45% to 94% across the ladder.

## Sizes and run time

Use `r` = 64, 256, 1,024, 4,096: four sizes, each four times the last. Measured inputs run from 3.6 KB (F1 at 64) to 2.1 MB (±S and OS at 4,096).

At `r = 16`, fixed overhead hides the F1 growth: the F1 decode reading falls from 1,474.8 to 1,462.8 fuel per byte between 16 and 64. The F1 per-byte reading also rises in steps rather than smoothly (+5.6%, +5.6%, +0.4% across the three steps). That is the B-tree's height, so judge the ladder as a whole, never one step.

All 90 cells of my prototype (five sizes, six families, three operations) took 16 s of test time for the current code and 18 s for W, single-threaded on ox-east-1 at load 460 to 990. Split the committed suite into one test per family pair (F1, ±S, OS, each with its control) so nextest runs them in parallel.

## What to commit and assert

1. **A pinned table** of exact fuel per cell (family × control × operation × size), as one source of truth in the harness, for example `harness/src/zero_range_ladder.rs`. Extend `just fuzzfit-calibrate` to regenerate it, so a toolchain bump re-pins it with the bands in one command.
2. **A band per cell.** The ceiling is the pinned reading × 1.02, rounded up; the improvement tripwire is the reading × 0.98, rounded down.
   - Fuel is deterministic, so any movement is a code change. The band is a judgment call; I recommend ±2% and list it as a decision for the owner below.
   - A trip above the ceiling is a regression, and the audit rule applies: stop and report.
   - A trip below the tripwire is an improvement: re-pin in the same commit and name it.
   - Two lower bounds are in play and must not be interpreted alike. The tripwire flags improvement; the liveness floor (next item) flags a dead meter. `crates/before/tests/meter/width_circulation_cost.rs` already draws this distinction; follow its wording.
3. **A liveness floor** of fuel ≥ input bytes for every cell. The derivation: every operation reads each byte of its input at least once, and a read costs at least one wasm instruction. It is far below every reading (the lowest is 127 fuel per byte), and it fails only if the measured call stops doing its work.
4. **Value checks** that make the measured work the real work:
   - decode re-encodes to the staged bytes (`ff_version_encode`, then `stage_read`);
   - both `<=` answers hold, as above;
   - at `r = 64`, the guest's `min_ticks` rendered unmeasured through `ff_count_display` equals the native `v.min_ticks().to_string()`. Rendering at larger sizes is quadratic, so skip it there.
5. **Control flatness.** For each control and operation, the per-byte reading at `r = 4,096` is at most 1.01 times the per-byte reading at `r = 64`. Every control falls slightly today. This guards the controls' meaning across re-pins: a control that starts recording ranges would grow.

Do not assert that the sparse rows grow. That would be a mechanism for requiring a known defect. The pinned table shows the growth, and the fix re-pins it flat.

Every test gets a doc comment stating its invariant. Extend the fuzz-fit entry in `crates/before/src/testing/validation_index.rs` with what this file alone catches: work that escapes the board's currencies on chosen families, as opposed to the bands' random programs.

## Expected readings at base

Measured at `explore/l7-suanpan` `b9f688a3`. Its suanpan and `before` production code is main `0bdeb588`'s, apart from feature-gated prototypes and test-only modules (I checked `git diff main..HEAD`; every addition is behind a `cfg` the guest does not enable). The guest was built with `PINNED_RUSTC` from the box's toolchain. Inputs are byte-identical for any builder who follows the family definitions, because the encoding of a given tree is canonical. Re-measure at your base anyway, and record both values if they differ; a difference means a commit landed on one of these paths after `0bdeb588`.

Exact fuel for the current code (A), with per-byte readings for A, for the W prototype, and for the known-bad K1. Per-byte readings divide by `v`'s encoded bytes for every operation, including `<=`.

| family | op | r | v bytes | A fuel | A / byte | W / byte | K1 / byte |
|---|---|---|---|---|---|---|---|
| F1 | decode | 64 | 3569 | 5220705 | 1462.8 | 1343.9 | 1646.4 |
| F1 | decode | 256 | 14321 | 22122134 | 1544.7 | 1339.2 | 1810.4 |
| F1 | decode | 1024 | 57329 | 93473515 | 1630.5 | 1338.0 | 1978.8 |
| F1 | decode | 4096 | 229361 | 375636916 | 1637.8 | 1337.6 | 1989.8 |
| F1 | le | 64 | 3569 | 10401687 | 2914.5 | 2694.2 | 3273.5 |
| F1 | le | 256 | 14321 | 43978365 | 3070.9 | 2685.3 | 3590.5 |
| F1 | le | 1024 | 57329 | 185420866 | 3234.3 | 2683.0 | 3915.3 |
| F1 | le | 4096 | 229361 | 743413824 | 3241.2 | 2682.5 | 3925.8 |
| F1 | min_ticks | 64 | 3569 | 13223988 | 3705.2 | 3445.9 | 3713.3 |
| F1 | min_ticks | 256 | 14321 | 52996488 | 3700.6 | 3433.9 | 3712.3 |
| F1 | min_ticks | 1024 | 57329 | 212438221 | 3705.6 | 3431.1 | 3721.2 |
| F1 | min_ticks | 4096 | 229361 | 851483885 | 3712.4 | 3430.2 | 3731.8 |
| F1 control | decode | 64 | 3577 | 4490446 | 1255.4 | 1347.1 | 1255.4 |
| F1 control | decode | 256 | 14329 | 17956066 | 1253.1 | 1344.5 | 1253.1 |
| F1 control | decode | 1024 | 57337 | 71821904 | 1252.6 | 1343.8 | 1252.6 |
| F1 control | decode | 4096 | 229369 | 287260233 | 1252.4 | 1343.5 | 1252.4 |
| F1 control | le | 64 | 3577 | 8992374 | 2513.9 | 2694.4 | 2513.9 |
| F1 control | le | 256 | 14329 | 35964362 | 2509.9 | 2689.8 | 2509.9 |
| F1 control | le | 1024 | 57337 | 143851102 | 2508.9 | 2688.6 | 2508.9 |
| F1 control | le | 4096 | 229369 | 575396850 | 2508.6 | 2688.4 | 2508.6 |
| F1 control | min_ticks | 64 | 3577 | 13174184 | 3683.0 | 3444.4 | 3683.0 |
| F1 control | min_ticks | 256 | 14329 | 52682238 | 3676.6 | 3438.0 | 3676.6 |
| F1 control | min_ticks | 1024 | 57337 | 210722005 | 3675.1 | 3436.6 | 3675.1 |
| F1 control | min_ticks | 4096 | 229369 | 842835382 | 3674.6 | 3436.1 | 3674.6 |
| ±S | decode | 64 | 32292 | 6394632 | 198.0 | 129.4 | 223.8 |
| ±S | decode | 256 | 130596 | 29452727 | 225.5 | 127.8 | 260.5 |
| ±S | decode | 1024 | 523812 | 134517341 | 256.8 | 127.4 | 302.2 |
| ±S | decode | 4096 | 2096676 | 604081312 | 288.1 | 127.3 | 344.8 |
| ±S | le | 64 | 32292 | 12314584 | 381.4 | 248.5 | 431.3 |
| ±S | le | 256 | 130596 | 56781084 | 434.8 | 245.5 | 502.5 |
| ±S | le | 1024 | 523812 | 259504520 | 495.4 | 244.7 | 583.3 |
| ±S | le | 4096 | 2096676 | 1165946697 | 556.1 | 244.5 | 665.9 |
| ±S | min_ticks | 64 | 32292 | 12910189 | 399.8 | 208.8 | 487.5 |
| ±S | min_ticks | 256 | 130596 | 61461726 | 470.6 | 201.5 | 588.9 |
| ±S | min_ticks | 1024 | 523812 | 290182450 | 554.0 | 199.4 | 707.0 |
| ±S | min_ticks | 4096 | 2096676 | 1340800900 | 639.5 | 198.9 | 830.8 |
| ±S control | decode | 64 | 32548 | 4517913 | 138.8 | 155.3 | 138.8 |
| ±S control | decode | 256 | 130852 | 17973453 | 137.4 | 153.8 | 137.4 |
| ±S control | decode | 1024 | 524068 | 71794209 | 137.0 | 153.4 | 137.0 |
| ±S control | decode | 4096 | 2096932 | 287113536 | 136.9 | 153.3 | 136.9 |
| ±S control | le | 64 | 32548 | 8676421 | 266.6 | 298.7 | 266.6 |
| ±S control | le | 256 | 130852 | 34542789 | 264.0 | 295.9 | 264.0 |
| ±S control | le | 1024 | 524068 | 138006725 | 263.3 | 295.2 | 263.3 |
| ±S control | le | 4096 | 2096932 | 551860933 | 263.2 | 295.0 | 263.2 |
| ±S control | min_ticks | 64 | 32548 | 7606820 | 233.7 | 280.3 | 233.7 |
| ±S control | min_ticks | 256 | 130852 | 29718899 | 227.1 | 273.4 | 227.1 |
| ±S control | min_ticks | 1024 | 524068 | 118081879 | 225.3 | 271.4 | 225.3 |
| ±S control | min_ticks | 4096 | 2096932 | 471472662 | 224.8 | 270.8 | 224.8 |
| OS | decode | 64 | 32412 | 4803211 | 148.2 | 129.5 | 158.0 |
| OS | decode | 256 | 130716 | 20461922 | 156.5 | 128.0 | 170.3 |
| OS | decode | 1024 | 523932 | 86810792 | 165.7 | 127.6 | 183.7 |
| OS | decode | 4096 | 2096796 | 364912585 | 174.0 | 127.5 | 195.9 |
| OS | le | 64 | 32412 | 11800330 | 364.1 | 241.1 | 410.2 |
| OS | le | 256 | 130716 | 54074114 | 413.7 | 238.1 | 476.5 |
| OS | le | 1024 | 523932 | 246095557 | 469.7 | 237.4 | 551.3 |
| OS | le | 4096 | 2096796 | 1102042525 | 525.6 | 237.2 | 627.5 |
| OS | min_ticks | 64 | 32412 | 11058666 | 341.2 | 195.4 | 398.6 |
| OS | min_ticks | 256 | 130716 | 52166495 | 399.1 | 189.8 | 477.9 |
| OS | min_ticks | 1024 | 523932 | 244150456 | 466.0 | 187.8 | 568.7 |
| OS | min_ticks | 4096 | 2096796 | 1117606413 | 533.0 | 187.3 | 661.1 |
| OS control | decode | 64 | 32548 | 4471753 | 137.4 | 150.2 | 137.4 |
| OS control | decode | 256 | 130852 | 17785981 | 135.9 | 148.7 | 135.9 |
| OS control | decode | 1024 | 524068 | 71041489 | 135.6 | 148.3 | 135.6 |
| OS control | decode | 4096 | 2096932 | 284099824 | 135.5 | 148.2 | 135.5 |
| OS control | le | 64 | 32548 | 8405299 | 258.2 | 288.6 | 258.2 |
| OS control | le | 256 | 130852 | 33418227 | 255.4 | 285.8 | 255.4 |
| OS control | le | 1024 | 524068 | 133468403 | 254.7 | 285.1 | 254.7 |
| OS control | le | 4096 | 2096932 | 533667571 | 254.5 | 284.8 | 254.5 |
| OS control | min_ticks | 64 | 32548 | 7003191 | 215.2 | 257.2 | 215.2 |
| OS control | min_ticks | 256 | 130852 | 27456399 | 209.8 | 251.7 | 209.8 |
| OS control | min_ticks | 1024 | 524068 | 109227791 | 208.4 | 249.8 | 208.4 |
| OS control | min_ticks | 4096 | 2096932 | 436274959 | 208.1 | 249.4 | 208.1 |

Reading the table:

- **A** grows on every sparse decode, `<=`, and `min_ticks` row except F1's `min_ticks` (F1's opening enters `min_ticks`' prefix structure, not an accumulator that records ranges).
  - From `r = 64` to 4,096: F1 decode +12.0% and `<=` +11.2%; ±S +46% (decode), +46% (`<=`), +60% (`min_ticks`); OS +17%, +44%, +56%.
  - Every control is flat or falling.
- **K1** raises the sparse rows and leaves every control row identical to A. That shows the controls record no ranges.
- **W** is flat on every sparse row. On the controls it is 7.2% to 20.5% dearer than A, except F1's `min_ticks`, which is 6.5% cheaper. Item 2 must close that gap.

## How you prove it detects the F1 mechanism

**Known-bad variant (required).** K1 repeats the map lookup in `ZeroRanges::remove_written`'s `Many` branch, which doubles the first map term without changing behavior. Apply this exact swap in `crates/suanpan/src/accumulator/digits/zero_ranges.rs`:

```rust
        if let Ranges::Many(ranges) = &mut self.ranges {
            while let Some((&lo, &hi)) = ranges.range(..to).next_back() {
```

becomes

```rust
        if let Ranges::Many(ranges) = &mut self.ranges {
            core::hint::black_box(ranges.range(..to).next_back());
            while let Some((&lo, &hi)) = ranges.range(..to).next_back() {
```

Rebuild the guest and run the ladder. Expected: every sparse decode and `<=` cell, and the ±S and OS `min_ticks` cells, exceed their ceilings (K1 adds 6.6% to 29.9% per cell; the smallest margin is OS decode at `r = 64`, +6.6%). F1 `min_ticks` moves by 0.2% to 0.5%, within its band. Every control cell is unchanged. Revert, confirm `git diff` is empty, and record the failure output in your report.

**Known-good variant (optional; item 2 delivers it).** The W prototype on `explore/l7-suanpan` (feature `l7-proto-w`, `crates/suanpan/src/accumulator/digits/written.rs`) trips the tripwire on every sparse cell (7.6% to 68.9% lower; F1 `min_ticks` 7.0% to 7.6% lower) and the ceiling on every control cell (7.2% to 20.5% higher) except F1 `min_ticks`, which it lowers by 6.5% and so trips that cell's tripwire instead. My readings are the W column above.

## Stop conditions

- The A readings at your base differ from the table by more than the band, and no commit on these paths explains the difference.
- Any control row is not flat at your base.
- K1 fails to trip a sparse decode or `<=` ceiling.
- The suite cannot finish inside nextest's 180-second limit per test at your base.

## Decisions for the owner (via the coordinator)

1. **Band width.** Recommend ±2%. A narrower band makes every small change on these hot paths a re-pin; a wider one lets W's 7% control regression or a 6.6% K1 cell pass. Exact pins (zero band, re-pinned by calibration like the fuzz-fit bands) are the strict alternative.
2. **Whether `min_ticks` on F1 stays in the table.** It shows no F1 growth (its opening never enters an accumulator that records ranges), but it is the one row where W is cheaper on a control. I recommend keeping it, for uniformity and as a control-like row.

## Owner's ruling

Each cell holds its base reading to within ±2% in both directions: a ceiling,
and an improvement tripwire that makes a win a deliberate re-pin. Exact pins
were rejected because codegen drift from unrelated changes already moves fuel
by 0.2% to 0.3% on kernels whose source is unchanged; ±2% absorbs that and
still fails K1's smallest cell (6.6%). F1's own `min_ticks` row stays,
because it records the one row where W reads cheaper.
