<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), agent rank-sum-remeasure, re-measuring `Rank`'s `Sum` at `main` = d6c4a8118 for the before audit (defect D2's re-measurement). -->

# `Rank`'s `Sum` at `main`, re-measured after the written-position set

`O(k + n)` now holds as documented (`crates/before/src/rank.rs:987` and `:999`). #86 resolves D2, and no repair is needed.

I couldn't write `report.md` as you asked: the harness refuses report files from subagents. This message is the full report. My resumption record is `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/rank-sum-remeasure/NOTES.md`.

## Verdict

- **Measured.** At `main`, fuel per input byte on wasm32 is flat for both families from the defect record.
  - `comb-rank` (round one's adversary) reads 227.0 to 227.8 from m = 256 to 32,768. Just before #86, the same input reads 809.8 rising to 1,339.2.
  - `dense-suffix` (the board's `rank_sum` cell) reads 336.6 to 336.9 at every size from p = 16 to 2,048. Just before #86, it reads 348.9 rising to 914.6.
- **Constructed.** I built four more families, each aimed at one of the remaining costs that are not constant per operation: shifting a dense value, rebuilding the bitset at every shift, the reversed summand order, and a write below the run for every summand.
  - All four are flat at `main`: over their last four doublings, the slopes run from −0.55 to +0.04 fuel/B per doubling.
  - Three of them grow logarithmically before #86, at +42 to +118 fuel/B per doubling. So they also detect the old defect.
- **Read, then counted.** In a sum of nonnegative ranks, no write ever cancels the accumulator's top digit.
  - So the written set never removes a position during the fold, and each bitset word becomes nonempty at most once per digit buffer. That makes the set's total work linear.
  - A deterministic counter of bitset word operations agrees on all six families. Words per byte stay flat, and the fold makes zero descents. It makes no predecessor queries apart from the final read's.

What remains is a gap in the documentation and in the instruments, not a defect.

- `suanpan` still documents a log factor for both operations `Sum` uses:
  - adding limbs: amortized O(`L` log(`W` + 1) + `G`), at `crates/suanpan/src/lib.rs:93`
  - `<<=`: amortized O(`A` log(`S` + 1) + `S`), at `crates/suanpan/src/accumulator/operators.rs:488`
- Put together, those documented costs give `O((n + k) log n)` for `Sum`. The stronger bound rests on a property of the implementation that neither crate states.
- No committed instrument counts the written set's work.

Confirmed defects: none. Questions: one, which blocks nothing. I judge this question to be at diminishing returns.

## Readings: fuel per input byte on wasm32

I read three revisions. Each reading uses one fresh Wasmtime 48.0.5 instance and counts fuel only around the call `once(first).chain(rest).sum::<Rank>()`.

- **old** is `e0956b25d`: round one's numbers, from `lanes/l4-measures/rank-fuel/` (`fuel.tsv`).
- **parent** is `ac74c3e37`: the first parent of #86's merge, the tree #86 landed on.
- **main** is `d6c4a8118`.

All three guests use the same compiler, rustc 1.96.1, because that is what round one's guest was built with. I identified it from the rustc commit `31fca3ad` embedded in round one's wasm, which is the local `stable` = 1.96.1. The tree pins 1.97.1, but holding the compiler fixed keeps codegen changes out of the comparison. The denominator is round one's: the summed encoded bytes of the summands. Input bytes are identical in every column; the table script checks this.

### `comb-rank`: the comb with m zero digits, then 8m small ranks

| m | bytes | old | parent | main |
|---|---|---|---|---|
| 16 | 295 | 557.8 | 546.5 | 244.3 |
| 32 | 585 | 629.2 | 617.8 | 236.1 |
| 64 | 1,166 | 675.5 | 664.1 | 234.4 |
| 128 | 2,327 | 773.7 | 762.2 | 230.6 |
| 256 | 4,649 | 821.3 | 809.8 | 228.9 |
| 512 | 9,294 | 901.5 | 890.0 | 228.9 |
| 1,024 | 18,583 | 887.6 | 876.2 | 227.6 |
| 2,048 | 37,161 | 1,049.1 | 1,037.7 | 227.2 |
| 4,096 | 74,318 | 1,103.5 | 1,092.1 | 227.1 |
| 8,192 | 148,631 | 1,115.7 | 1,104.2 | 227.0 |
| 16,384 | 297,257 | 1,334.6 | 1,323.1 | 227.8 |
| 32,768 | 594,510 | 1,350.6 | 1,339.2 | 227.4 |

Over m = 2,048 to 32,768, the parent rises by 75.38 fuel/B per doubling and `main` by 0.04.

At m = 16,384, `main` rises by 0.8 for one size and falls back at the next. `comb-below` shows the same bump at the same size. I didn't investigate it.

### `dense-suffix`: DS(p, p)'s rank, then one small rank per encoded version byte

| p | bytes | old | parent | main |
|---|---|---|---|---|
| 16 | 4,648 | 371.1 | 348.9 | 336.7 |
| 32 | 9,098 | 371.2 | 348.9 | 336.8 |
| 64 | 18,019 | 545.8 | 523.5 | 336.9 |
| 128 | 35,861 | 573.1 | 550.8 | 336.8 |
| 256 | 71,544 | 683.3 | 661.0 | 336.7 |
| 512 | 142,911 | 771.4 | 749.0 | 336.7 |
| 1,024 | 285,644 | 881.6 | 859.2 | 336.6 |
| 2,048 | 571,109 | 936.9 | 914.6 | 336.6 |

Over p = 128 to 2,048, the parent rises by 90.95 fuel/B per doubling, and `main` falls by 0.05.

### Which change moved the numbers

Parent minus old is the same at every size: −11.4 to −11.5 fuel/B on `comb-rank`, and −22.3 to −22.4 on `dense-suffix`.

- The other `suanpan` commits between `e0956b25d` and the merge moved only a constant per byte. They are the integrator's buffer reuse (`81f3d9366`), clone normalization (`40b29270b`), and three refactors.
- The merge differs from its first parent only by #86's two commits, `069597735` and `a189389ae`. So the growth came from the zero-range map, and its removal is #86's.
- The constant gap also shows that my probe reproduces round one's readings.

### Constructed families (fuel/B, parent and main)

| knob | ones-ladder parent | ones-ladder main | comb-ladder parent | comb-ladder main | comb-rank-reversed parent | comb-rank-reversed main | comb-below parent | comb-below main |
|---|---|---|---|---|---|---|---|---|
| 16 | 234.1 | 198.9 | 327.5 | 161.8 | 334.2 | 194.5 | 562.9 | 283.2 |
| 32 | 237.0 | 179.5 | 344.8 | 147.6 | 363.4 | 190.8 | 555.0 | 280.7 |
| 64 | 236.2 | 162.3 | 373.5 | 134.0 | 397.1 | 185.3 | 683.3 | 277.6 |
| 128 | 234.6 | 153.0 | 416.4 | 129.2 | 439.7 | 182.7 | 780.9 | 274.7 |
| 256 | 233.0 | 146.6 | 461.2 | 122.9 | 474.3 | 181.4 | 744.1 | 273.2 |
| 512 | 231.9 | 142.6 | 507.5 | 119.4 | 516.5 | 180.6 | 908.6 | 273.2 |
| 1,024 | 231.2 | 140.0 | 556.6 | 117.5 | 559.7 | 180.2 | 894.7 | 271.9 |
| 2,048 | 230.8 | 138.4 | 607.5 | 117.3 | 595.4 | 180.1 | 887.6 | 271.6 |
| 4,096 | 230.5 | 137.5 | 662.8 | 117.1 | 641.3 | 180.0 | 1,110.6 | 271.5 |
| 8,192 | 230.4 | 137.0 | 715.4 | 116.9 | 679.4 | 179.9 | 1,122.8 | 271.4 |
| 16,384 | 230.3 | 136.7 | 772.0 | 115.3 | 717.7 | 179.9 | 1,257.5 | 272.3 |
| 32,768 | 230.2 | 136.5 | 827.0 | 115.1 | 763.6 | 179.9 | 1,357.7 | 271.8 |
| 65,536 | 230.2 | 136.4 | | | | | | |

Slopes over each family's last four doublings, in fuel/B per doubling:

| family | parent | main |
|---|---|---|
| `ones-ladder` | −0.07 | −0.26 |
| `comb-ladder` | +54.88 | −0.55 |
| `comb-rank-reversed` | +42.04 | −0.04 |
| `comb-below` | +117.53 | +0.04 |

The families are built as follows, with knob `2^e`:

- **`ones-ladder`** sums `1 − 2^−(32·2^j)` for `j = 0..=e`. Each summand needs a larger exponent than the one before, so each forces `acc <<= shift` on a dense held value.
- **`comb-ladder`** sums the comb with `2^j` zero digits, for `j = 0..=e`. Each summand forces a shift. The held value has zero digits, so every shift rebuilds the written set in bitset form.
- **`comb-rank-reversed`** is `comb-rank`'s summands with the comb last. After one shift, the comb's limbs land below the existing run.
- **`comb-below`** is `2^64 + comb(m)`, then 8m small ranks. The first summand leaves the run at its top digit, two digits above the units digit. Every small summand therefore writes below the run, into the bitset. That is the costliest path a single summand can take through the written set.

## The code argument: what `Sum` pays at `main`

`sum_iter` (`rank.rs:1018`) treats each summand in one of two ways:

- It adds the summand's limbs at bit offset `exp − rank.exp`.
- Or, if the summand needs a larger exponent, it first shifts the accumulator by `max(gap, stored_bits)` (`rank.rs:1045`).

The bookkeeping per summand is O(1): a zero test, one exponent comparison, and the limb iterator.

### Each write

Every nonzero 32-bit piece goes through `Digits::add_at` (`crates/suanpan/src/accumulator/digits.rs:274`), which pays for three things:

1. **The carry chain**, at amortized O(1) per piece. The centering argument in `digits.rs`'s module doc covers this.
2. **Buffer growth**, which zero-fills the new positions (`digits.rs:297`). In `Sum` the buffer only grows, so total fill is at most the final buffer length.
3. **One call to `insert_span`** (`digits.rs:320`). Its cost depends on where the written positions land relative to the run (`digits/written.rs:138-206`):

| Where the write lands | Cost |
|---|---|
| Inside the run, or extending it | Two comparisons. Most small summands take this path, including `comb-rank`'s and `dense-suffix`'s. |
| Above the run's end (a jump) | The old run moves below the new one. In prefix form, O(1), or the one-time conversion to a bitset. In bitset form, one insertion per run position. A position leaves the run only this way and never rejoins it, so each moves at most once. |
| Below the run, prefix form | O(1): the prefix absorbs it, or the run's start moves down. Otherwise, the one-time conversion. |
| Below the run, bitset form | One bitset insertion per position below the run (`written.rs:200-204`). This is `comb-below`'s path for every small summand. |

A bitset insertion (`written/bitset.rs:69`) checks coverage in O(1). It then climbs one level for each word that was empty, and stops at the first word that already held a bit.

Nothing removes a position during the fold (next paragraph). So each word at each level becomes nonempty at most once per digit buffer. Total climbing is therefore at most the number of insertions plus the number of words, which is about `W/63` words for `W` covered positions.

After the write, `trim_high_zeros` (`digits.rs:344`) costs one comparison. The descent at `digits.rs:361` is the only per-write path that queries or truncates the set, and it never runs in `Sum`:

- Every contribution is positive.
- A positive contribution either leaves the top digit at least 1, or overflows it.
- An overflowing top digit is recentered, and a positive digit carries to a new top.

So the top digit is never zero.

### Each shift (`acc <<= shift` at `rank.rs:1046`, implemented at `value.rs:187`)

- **The sign check.** `cmp_zero` on the old digits (`digits/sign.rs:64`) costs O(1). The top digit is at least 1, and one more digit lifts the running partial to at least `2^31`, so the check stops within two digits.
  - Its single truncate is O(1) when it falls on the run.
  - Otherwise it is one bitset predecessor query, whose climb is bounded by the distance it skips (`bitset.rs:23-28`). That distance is at most the result's width.
- **The rebuild.** The old digits are replaced by a fresh buffer (`value.rs:217`), and each held digit is deposited again at its shifted position. That costs:
  - one touch per held digit
  - one write (as above) per nonzero held digit
  - zero-filling up to the shifted bottom
  - if the new set converts, a bitset `cover` of about (`shift` + `A`)/2048 words
- **The cost per shift** is O(`A` + `shift`), where `A` is the held width. The code computes the shift as `gap.max(held_span)`, so `shift ≥ A`, and the cost is O(`shift`).
- **The total over the fold.** The shifts sum to the final exponent minus the first one:
  - A shift happens only for a summand whose exponent `e` exceeds `exp`. It sets `exp` to `max(e, exp + held)`, which is at most `2e + c`. Here `c = O(I + log k)`, where `I` is the widest integer part among the summands.
  - So the final exponent is at most `2·e_max + c = O(n + k)`.
  - This argument also covers shifts where the gap is larger than the held width, which `sum_iter`'s doubling argument (`rank.rs:1013-1016`) does not.

### The final read and the total

The final read (`rank.rs:1051-1052`) is linear in `W`:

- normalization costs O(`W`)
- clearing the bitset costs O(`W`/64), plus one word per level (`written.rs:281`, `bitset.rs:175`)
- building the `BigUint` costs O(`W`)
- `from_raw` (`rank.rs:476`) costs O(`n`)

The total is O(`k` + the sum of the limb counts + `W`), with `W` = O(`n` + `k`). That gives O(`k` + `n`). No case costs more than amortized O(1) per limb plus O(1) per summand.

### Why `suanpan`'s cost table shows a log factor

`suanpan`'s log factors pay for two things: descending through cancelled digits, and re-climbing words that a truncation emptied. A sequence that never cancels its top digit does neither, but that fact exists only in the code. `Sum`'s private comment (`rank.rs:1007-1016`) argues only about the shift widths.

## Deterministic counters at `main`

I built a patched copy of `d6c4a8118` natively (`probe-counters.patch`) and read it once per family. The table shows each family's largest size. The per-byte columns stay flat across all sizes (`logs/counters.tsv`).

| family | bytes | touch/B | bitset words | words/B | bitset insertions | spans beside the run | conversions | shifts | descents | queries | removals |
|---|---|---|---|---|---|---|---|---|---|---|---|
| comb-rank (m = 32,768) | 594,510 | 0.6614 | 35,893 | 0.0604 | 32,768 | 32,768 | 1 | 0 | 0 | 0 | 1 |
| dense-suffix (p = 2,048) | 571,109 | 0.8689 | 2,388 | 0.0042 | 2,273 | 63 | 1 | 0 | 0 | 0 | 1 |
| ones-ladder (65,536) | 589,836 | 0.7592 | 69,213 | 0.1173 | 65,542 | 65,550 | 13 | 15 | 0 | 0 | 1 |
| comb-ladder (32,768) | 589,895 | 0.6111 | 106,663 | 0.1808 | 98,313 | 98,316 | 14 | 15 | 0 | 0 | 1 |
| comb-rank-reversed (32,768) | 594,510 | 0.6614 | 35,900 | 0.0604 | 32,769 | 32,770 | 1 | 0 | 0 | 0 | 1 |
| comb-below (32,768) | 594,520 | 0.6614 | 298,045 | 0.5013 | 294,913 | 294,913 | 1 | 0 | 0 | 1 | 1 |

- **The counters count what they claim.** `comb-rank` makes exactly m insertions and m spans beside the run: one per jump while the comb streams in. `comb-below` makes 8m + m + 1: one span and one insertion per small summand.
- **Words per insertion** are 1.01 to 1.10 in every family, so levels above the first are almost never climbed.
- **The single removal** in each run is the final read's clear. `comb-below`'s single query is the final read's truncate.
- **`dense-suffix` makes 63 spans beside the run at p = 2,048.** That equals round one's zero-range tree size at the same scale. The same gap events still happen, but each now costs O(1).
  - Its insertions per byte rise from 0.0036 to 0.0040 over the last three doublings, in shrinking steps.
  - That count can't keep growing: insertions can't exceed the writes plus the positions, which is a constant per byte.
- **`comb-rank-reversed`'s one shift** happens while the accumulator is still a small value, a path my shift counter doesn't count. Its zero-fill of 65,537 = 2m + 1 digits shows the shift.
- **The path beside the run costs more, but by a constant.** It costs about 44 fuel/B more than `comb-rank`'s path inside the run, about 89 fuel per summand.

## Verified versus inferred

**Verified:**

- `/Users/oxide/src/rumors-verify` was detached at `d6c4a8118` with a clean tree at the start. At the end, `git diff` is empty and `git status --short --untracked-files=all` is empty.
- All three revisions' fuel readings, with identical input bytes.
- Every guest embeds rustc 1.96.1 (`31fca3ad`). Each links its intended revision: the `main` guest contains the source paths `digits/written.rs` and `written/bitset.rs`, and the parent guest contains `digits/zero_ranges.rs`.
- Before I patched my export, its `suanpan/src` was identical to `rumors-verify`'s (`diff -r` printed nothing).
- The counter readings, run natively on aarch64.
- The merge differs from its first parent only by #86's commits.
- Zero descents, and no queries during the fold, on all six families. This checks the premise that the top digit is never cancelled, on those inputs.

**Inferred:**

- The general argument that no nonnegative sum ever cancels its top digit, and so that bitset words climb at most once each. I checked it on the six families only.
- The bound on total shift work. The two ladder families measure flat, but the bound itself is argued, not measured.
- That the counts carry over to wasm32. They count algorithm steps on digit indices, which don't depend on pointer width, but I didn't run the counter probe on wasm32.
- The cause of the one-size bump at m = 16,384. I didn't investigate it; a step in allocation or memory growth is plausible.

## Recommendations and observations

1. **Write the derivation down beside the claim.** `Sum`'s `O(k + n)` rests on the add-only property above, and the code states that property nowhere. Without it, a `suanpan` change could bring back a per-write log factor. That change would still meet `suanpan`'s documented bounds, and nothing in either crate's prose would contradict it. Q1 below asks where the derivation should go.
2. **No committed instrument counts the written set's work.**
   - The touch meter excludes it. That is how D2 stayed invisible to the board: touch stayed flat at 0.661 per byte while fuel per byte grew 2.4×.
   - fuzzfit has no entry point for `Rank`'s `Sum`.
   - The wasm32-pins test `rank_sum_straddles_the_usize_alignment_limit` checks a result, not a cost.
   - Round one's proposed tree-work counter targeted the `BTreeMap`, which no longer exists.

   There are two ways to close this:
   - **(a)** Fold bitset word operations into `touch`. This extends an existing instrument, but it moves every pinned touch count in `suanpan`'s `metered.rs` and every board cell with a touch ceiling. Ceilings could rise, so that is the owner's call.
   - **(b)** Add a separate counter behind `touch-meter`, as `probe-counters.patch` does, with a band in words per byte on `comb-below` and `comb-rank`. It needs an entry in the validation index.

   I recommend (b). The failure it catches is real and can be constructed: the zero-range map read 0.661 touches per byte while its fuel per byte grew 2.4×.
3. **Constant factor: a conversion that a later limb makes unnecessary.** A limb stream that lands below the run with a gap converts the prefix to a bitset, even though the stream's later limbs close that gap.
   - `ones-ladder` converts 13 times in 15 shifts, at 0.111 bitset insertions per byte, while its final set has only one gap.
   - Inserting the stream's whole span once would keep the prefix form. That is sound: the set may hold positions whose digits are zero, and reading the operand already paid for every position it covers.
   - The stake is a few fuel per byte at most, so I don't think it's worth building now.
4. **#86's measured improvements.**
   - `dense-suffix` is 12.2 fuel/B cheaper even where no tree ever formed (at p = 16, 348.9 drops to 336.7).
   - `ones-ladder` shows no growth at the parent and is 93.8 fuel/B cheaper at `main` (230.2 to 136.4, −41%). I didn't trace which part of #86 causes this.
   - No committed fuel ceiling covers `Sum`, so there is nothing to lower.
5. **Round one's D2 options no longer apply.** Option 1 (restate the bound) is unnecessary. Option 2 (stop recording gaps inside one operand's span) survives only as item 3's constant factor. Option 3 (take `Sum` off the accumulator) is unnecessary. I recommend closing D2 as resolved by #86.

## Q1 (blocks nothing): where should `Sum`'s derivation live?

- **(a)** A paragraph for maintainers on `sum_iter` in `rank.rs`, stating the argument above. That is the level the owner chose for D1's `M(n)`. On its own, though, it ties `before`'s private prose to `suanpan` internals that `suanpan`'s contract allows to change.
- **(b)** A clause in `suanpan`'s public cost model that `before` can cite. For example: "starting from a new or reset accumulator, additions and left shifts that never cancel the top digit cost amortized O(`L` + `G`) and O(`A` + `S`)." That strengthens a public contract.
- **(c)** Both.

I recommend (c): (b) as the promise, and (a) as the pointer to it. Neither document is wrong today. `rank.rs` claims a true bound, and `suanpan`'s bounds are valid upper bounds; only the derivation is missing.

## Where I departed from the briefs

- **Builds ran on the Mac**, under `nice -n 10` with `CARGO_BUILD_JOBS=5`, as you said. `common.md` says to build only on ox-east-1; I followed your more specific instruction. `CARGO_BUILD_BUILD_DIR` kept every build inside my scratch directory, so nothing landed in `/Volumes/forge`.
- **I never modified `rumors-verify`, rather than patching it and reverting.**
  - The `main` guest builds against it read-only.
  - The counter patch went into a `git archive` export of `d6c4a8118`. I confirmed it matched `rumors-verify`'s `suanpan/src` before patching.
  - The parent revision is a second export.
- **I measured one extra revision**, the merge's first parent, to separate other commits' drift from #86's effect. That is one pass per family per revision.
- **The guests use rustc 1.96.1, not the tree's 1.97.1**, so the comparison with round one is like for like. The native counter probe uses 1.97.1, since step counts don't depend on the compiler.
- **One counter run read nothing.** Under the Bash tool's zsh, `set -- $spec` didn't split words, so the probe got "2 4 15" as a single argument and failed to parse it, for every family (`logs/counters-argsplit-failure.log`). I reran the same pass once, through `bash`. No readings existed to repeat.
- **I added `comb-below` after the first fuel pass**, to cover the per-summand path beside the run. Its case was appended to the shared family file. The measured function and families 2 through 7 are unchanged, and I didn't re-measure them.
- **`report.md` couldn't be written**: the harness refuses report files from subagents. This message is the report.

## Instruments, files, and cleanup

Everything is in `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/rank-sum-remeasure/`. None of it is committed, and there is no explore branch.

- **`probe/guest-src/families.rs`** builds the six families' summand lists. The wasm guests and the native probe both include it, so their inputs are identical.
- **The fuel probe** is `probe/guest-src/lib.rs`, `probe/guest-main/`, `probe/guest-parent/`, and `probe/host/`. It uses one fresh Wasmtime instance per reading and counts fuel around a single `Sum`. The host is round one's, unchanged.
- **The native counter probe** is `probe-counters.patch` plus `trees/main-probe/crates/before/examples/rank_sum_probe.rs`. It counts bitset words, insertions, removals, predecessor queries, spans beside the run, conversions, buffer zero-fill, shifts of a digit-form value, and descents.
- **`tables.py`** builds the tables from the logs and checks that input bytes are identical across revisions. Its output is in `tables.md` and `constructed-table.md`.
- **`logs/`** holds the build logs, `fuel-main.tsv`, `fuel-parent.tsv`, and `counters.tsv`.
- **`run-fuel.sh`, `run-counters.sh`, and `run-op8.sh`** are the exact commands I ran.
- **What these reach that the committed board doesn't.** `comb-rank`, the two ladders, and `comb-below` exercise the shift path and the path beside the run, at inputs up to 590 KB.
  - The board's `rank_sum` row puts the widest exponent first, so it never shifts.
  - I didn't check whether any board family's rank leaves its run two or more digits above its units digit. Such a family would drive the path beside the run once per summand.
- **Cleanup.** Delete this directory to retire everything: about 1.3 GB, mostly builds in `build/`, `probe/*/target`, and `trees/main-probe/target`, plus the source exports in `trees/`.
- **Resuming.** Read `NOTES.md` in this directory.

I made no commits, so there are no unsigned SHAs.
