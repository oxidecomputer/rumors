## Verdict

#86 is ported onto `fix/rank-integrator-refill` (`51498b272`) as briefed. The branch is `fix/suanpan-written-set` in `/Users/oxide/src/rumors-slot-24`, tip **`4f32b5f2b`**. Every check on your list passes at the tip. I left the landing check (`audit-check`) for you, as you asked.

## Commits

All three were made with `--no-gpg-sign`, so all three need re-signing.

1. `64c8229b3` suanpan: Replace the zero-range map with a set of written positions
2. `c73bfc754` suanpan: Hold written positions below 64 in an inline mask
3. `4f32b5f2b` board: Re-pin the heap worst cases the written-position set flips

I verified at `34104e2f8`, `a453a1798` and `904590c69`. After that I rewrote only the messages with `git commit-tree`. The trees are byte-identical: c04aaca6, 1cf8f834, b7dd5208. #52's commit and all of its ladder edits are gone: the fuzzfit `zero_range_ladder` files and the ladder paragraphs in fuzzfit `lib.rs` and `validation_index.rs`. No message cites the ladder.

## What I ported, deleted, or restated

**The space reader.** At #86's base, `lib.rs` promised that a public `normalize` keeps its "rebuilt skip metadata [in] O(`Q`) space". That method and that promise no longer exist, so **#86's space promise has no reader.**
- The only documented space bound on the set today is `clone`'s "O(`Q`) for the copy".
- So the rebuild moved into the two copy paths in `digits.rs`:
  - `copy_into` gives the copy a fresh inline set: position 0, plus one run from the floor to the top. The copy holds O(1) skip metadata whatever its source retains.
  - `Digits::clear`, which `clone_from` uses, replaces the receiver's set instead of clearing it, because clearing would cost the receiver's old width.

**Deleted.**
- `rebuild_written`, and with it the zero-result repair from #86's review. Nothing reads a bound on the source after normalization: the source may retain storage after cancellation, which `lib.rs` permits.
- The `normalize`-of-zero test. A fresh clone of zero holds no set storage, and `clone_from` may keep the receiver's allocation, so a zero-path test would assert more than the contract says.

**Ported test.** #86's nonzero `normalize` space test becomes `clones_take_written_set_space_for_the_value_alone` in `tests/copies.rs`, with the same 29,001-position source and the same "≤ 2× level-0 words" bound.

**Normalization** (`normalize_nonzero_from`, the path behind magnitude reads and `clone`). The pass writes every position from the floor to its top. So the set becomes position 0 plus one run, held inline. Any bitset storage is cleared in place and kept, as the digit buffer is.

**`reset` (#133's O(`R`)).** #86's `clear(highest)` swept bitset words from position 0 upward, which costs O(`W`/64). That breaks #133's documented O(`R`) bound, so I changed it:
- `WrittenPositions::clear(lowest, highest)` removes position 0, then clears only the words covering `lowest..=highest` at each level.
- It stops climbing at the first level whose words there are already empty. That stops `reset` from paying for every level of the hierarchy on each call.
- This relies on a new invariant, documented on the `written` field: apart from position 0, nothing below `lowest_written` is written. `digits/tests.rs` `assert_invariants` now checks it after every step.

**The model test** stays as it was, except its `Clear` op. The new signature forced one change: it now draws a slack below the least nonzero held position, to mirror how `Digits` clears.

**Prose.** I swept the remaining "zero range" wording in `suanpan`, and in `before`'s `board/family.rs`, `meter.rs`, `registry.rs` and `skip_adequacy.rs`.

## `min_ticks` and `RangeMinima`

Both reach accumulators only through the public API, so no value can change. I checked their board rows from base to tip:
- **Touch only falls.** On `wide-tooth-comb`, 208,216 → 170,913 (−17.9%); on `tooth-tail`, −0.7%; on `arming-train`, −0.4%.
- **Heap rises by at most 0.048 B/B** (`pure-comb`, +240 B). The one rise that grows with input is on `wide-tooth-comb`: +608 B at 652,841 input bytes.
- At commit 1 alone, `min_ticks` heap rose by up to 1.38 B/B on `wide-step-pairs`, from boxed bitsets below position 64. Commit 2 returns it to the base reading or below.

On a 64-bit target, `Accumulator` is 128 bytes both before and after: the map and the set each take 32 bytes. I worked this out from the type layouts and did not measure it.

## The `reset` question, reframed

#86 framed it around `normalize`, which is no longer public, so its "in every case" wording has no referent.

What remains: after `reset`, the cleared bitset is retained. That costs at most about 1/63 of the digit buffer, which `reset` also retains. `lib.rs` already allows an accumulator to "retain more allocation than its working width after cancellation or reset". Retention also helps #133's integrator, which now resets instead of rebuilding, because the bitset is not promoted again on each freeze.

**I don't think there is anything left to rule on.** The one reading that would reopen it: if the owner takes `reset`'s "retains its main allocation" to mean secondary storage must be released.

One asymmetry to mention: `clone_from` releases the receiver's bitset but keeps its digit buffer. Clearing the bitset would cost the receiver's old width, which `clone`'s cost bound does not include.

## The board, compared against a capture at `51498b272`

Three captures on ox-east-1 (base, commit 1, commit 2), compared with `just amp-board compare`. Base: 5545 green, pin clean (272 rows).

**Tip: 5545 green.** Of 16,635 rows, 16,027 are unchanged. No scan reading moves.

**Touch falls on 7 cells and rises on none, all from commit 1.**
- The three `min_ticks` falls above, plus `own_version_to_version` and `own_span_to_span` falling by a constant 30 or 1.
- Mechanism, from my reading of the code (not measured separately): the set's positions match the complement of the map's ranges everywhere except after a normalizing pass or a copy. There a descent now jumps from the floor to position 0, where the map stepped through each position.
- None of these families is its operation's worst case, so no ceiling lowers.

**Heap moves (608 rows):**
- Every constant shift is a fall in multiples of 192 B, which is the size of one `BTreeMap` leaf node.
- The largest falls are the rank rows on `dense-suffix` and `promo-rearm`: −0.80 B/B.
- Every remaining rise is at most 0.052 B/B at the largest size.
- Of the rises, 70 grow with input, with slopes up to 0.027 B/B (`wide-arming`). That is the bitset covering wide gaps at one bit per position, the same cost #86's entry reported.
- Odd-sized deltas appear only at the small inputs, where peak heap can shift to a different moment.
- From commit 1 to commit 2, only heap moves: 385 readings fall and none rises.

**The re-pin.** `version_rank`, `ranked_encode` and `ranked_encode_rank` flip from `arming-train` to `wide-arming` at the default scale. Heap per byte:

| family | base | tip |
|---|---|---|
| `arming-train` | 6.902 | 6.799 |
| `wide-arming` | 6.816 | 6.859 |

`arming-train`'s fall alone causes the flip: 6.799 is already below `wide-arming`'s base reading of 6.816. At the acceptance scale nothing flips (6.838 against 6.814). `ranked_borsh_deserialize` already pins `wide-arming` today, so it needed no change.

## Checks

| Where | Check | Result |
|---|---|---|
| Mac | clippy `-p suanpan`; nextest `suanpan` at each code commit | clean; 100/100 and 101/101 |
| ox-east-1, tip `904590c69` | `fmt-check`, `gate-lints`, `clippy`, `clippy-default` | ok |
| | `test-all` | 1958 passed, 2 skipped |
| | `doctest` | ok (`before` 190, `suanpan` 9) |
| | `docs`, `docs-internal`, `docs-docsrs`, `readme-check` | ok |
| | `amp-board-acceptance` | 5545 green, pin clean (272 rows) |
| | `fuzzfit` | 25/25 |

## Deviations and notes

- **Re-pin placement.** The builder brief says a flip is re-pinned "in the same commit" as the improvement. Your task asked me to redo #86's separate third commit, so I kept it separate. As a result, commits 1 and 2 fail the pin until commit 3. Squashing commit 3 into commit 2 would satisfy the rule; say if you want that.
- **Where I built.** The common brief says to build only on the box, apart from fmt; your task said to iterate on the Mac. I followed the task: Mac runs used `nice -n 10`, 4 jobs, 4 threads.
- **#117 and #118** (`docs/suanpan-time-bounds` and `fix/suanpan-cover-growing-levels`) still sit on the old #86 tip `b9ec1fbdb`. They edit `written.rs` and need rebasing onto this branch. Expect a conflict where #118's `cover` meets the new `clear`.

Files are in `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/port-written-set/`:
- `NOTES.md`
- `board-base-51498b272.txt`
- `board-c1.txt`
- `board-c2.txt`
- `compare-base-c1.log`
- `compare-base-c2.txt`
- `compare-c1-c2.txt`
- `worst-c2.txt`
- `verify.log`

The per-step verification logs are on the box at `~/src/rumors-slot-24/target/verify-*.log`.