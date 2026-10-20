<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), reviewer review-written-set, of fix/suanpan-written-set at 4a4396269. -->

# Review of `fix/suanpan-written-set` (tip `4a4396269`, on `main` `e25bf4c65`)

## Verdict

**Changes required.** One code finding and a few factual errors in commit 1's message. I wrote and checked the repair for the code finding. The rest of the change holds up:

- The new invariant holds on every path.
- `reset`'s early stop is sound.
- `reset` costs O(`R`) once its log term is amortized; per call it is O(`R` + log₆₄ `W`).
- The model test reaches every form and transition.
- The porter's board numbers are exact.

Base check: the slot's HEAD is `4a4396269`, the tree is clean, and the merge-base is `main` = `e25bf4c65`. Outside `.agent-notes/`, `main`'s code equals the porter's base `51498b272`, and the tip's tree equals the porter's tip `4f32b5f2b`. The squash moved only the re-pin into commit 2, so the porter's board captures apply to this tip.

I did not run the landing check, because you said one is already running at this tip.

Where my brief and this task conflicted, I followed the task. You allowed local runs (`nice -n 10`, `CARGO_BUILD_JOBS=4`). You also ruled out injected-defect demonstrations, so I ran none.

## Blocking findings

### 1. A sign query allocates a bitset that grows with the value's width

**Where.**
- The comparison's write-back: `crates/suanpan/src/accumulator/digits/sign.rs:96-104`.
- How `truncate` parks the emptied run: `crates/suanpan/src/accumulator/digits/written.rs:263-280`.

**Mechanism.** Take the value 2^(32·S), which is one digit at position S. Its set is the run `S..S+1` above `Prefix(1)`.
- `sign()` reads `partial = 1` at S, steps to the unwritten position S−1, and stops there with `partial = B`.
- `truncate(S−1)` drops the whole run and parks the empty run at `S..S`, which sits above a gap.
- `add_at(S−1, B)` then starts below that empty run, so `insert_below(S−1, S−1)` joins it to the positions below.
- `Prefix(1)` plus a lone position at 64 or above converts to a bitset covering S positions.

The resulting set is `{0, S−1}`, which a prefix and a run hold inline. On `main`, the map handled the same path inline: `remove_written(S−1, S−1)` turns `One(0, S)` into `One(0, S−1)`.

**Evidence.** I built this as a probe in a scratch worktree at the tip. The tip's numbers are below; the repair numbers came after applying `repair-truncate.patch`.

| S | Bitset words before `sign()` | After `sign()` at the tip | After `sign()` with the repair |
|---|---|---|---|
| 1,000 | 0 | 17 | 0 |
| 10,000 | 0 | 161 | 0 |
| 100,000 | 0 | 1,589 | 0 |
| 1,000,000 | 0 | 15,875 | 0 |

Each allocation also zero-fills those words, so the time is Θ(S/64) as well.

**Contract.** `crates/suanpan/src/lib.rs:95` gives sign queries O(1) additional space, not amortized, and the branch leaves that row as it is. The owner's question-114 ruling ("say amortized everywhere") would admit this allocation, charged to earlier width growth. That wording is only on #117 (`docs/suanpan-time-bounds`), which is drafted against the older stack: its table still says `reset` is O(`W`) and documents a public `normalize`. It also notes that "a comparison with zero... write-back can grow the skip metadata by O(`V`)", so this growth was probably visible there.

If you'd rather route this as an owner question than a block, the repair removes the issue either way.

**Repair, written and checked.** When `truncate` drops the whole run, it parks the empty run just above the positions that remain below it:
- for a prefix, at its end;
- for a mask, at its highest bit plus one;
- for a bitset, at the position the removal loop already finds, plus one.

A later write at the new top then starts a run instead of joining the positions below. The repair is about 20 lines inside `truncate`; the patch file (`repair-truncate.patch`, 50 lines) is a sketch whose formatting still needs `rustfmt`. Checks:
- The probe reads 0 words at all four sizes.
- The full local suite, `cargo nextest run -p suanpan --all-features`: "102 tests run: 102 passed, 0 skipped". That count includes my probe.

I also ran a board capture on the box with the repair and compared it with the porter's tip capture:
- "amp-board: 5545 green / 0 red (5545 cells)" and "worst-case pin: clean (272 pinned rows verified at 2 sampling scales)", so the tip's re-pin still holds.
- Tip to repair: 258 heap readings fall at the largest size. Four rise, by at most 0.0017 B/B (`version_cmp`, `version_concurrent`, `version_span`, and `span_new` on `wide-arming`, each +256 B at 149,069).
- Base `51498b272` to repair, compared with base to tip:

| Base against | Rises at the largest size | Rows growing with input | Steepest slope | Largest rise |
|---|---|---|---|---|
| tip | 354 | 70 | 0.027 B/B | 0.0514 B/B |
| repair | 126 | 45 | 0.018 B/B | 0.0364 B/B |

### 2. Commit 1's message (`efd09c9e2`) contains claims the artifacts contradict

- **A stale reference.** "the commit after the next re-pins it": the re-pin is now in the next commit, `4a4396269`.
- **The 192-byte claim is false at commit 1.** "every constant shift is a fall in multiples of 192 bytes" holds for base to tip: every constant heap shift is −192·k or +0. At commit 1 alone, `compare-base-c1.log` has 122 constant shifts of +56 B, 74 of −136, 17 of +112, and others. The claim belongs in commit 2, or should be stated as true of the pair.
- **The space claim is false.** "the copy is now the only operation whose documented space bound covers the set": every row of `lib.rs`'s additional-space column bounds whatever its operation allocates, the set included. The sign-query row (finding 1) is the one the branch breaks.
- **R has two meanings.** The message uses "R ranges" for the map's size, then uses the crate's `R` (the written span) for `reset`'s bound.

The message's history sentence is accurate: `main` had a public `normalize` with an O(`Q`) promise until `5ad6c74f4`.

Commit 1 also fails the board leg on its own. The porter's `board-c1.log` shows three worst-case pin drift lines, and the re-pin lands only in commit 2. The builder rule says to re-pin in the same commit as the flip. Folding the commits at landing resolves it.

## Prose findings against Part IV

My judgment is that none of these block. Item d is the only rustdoc call that is open to dispute.

a. **`Digits::clear` (`digits.rs:178-180`) and `copy_into`'s comment (`digits.rs:162-166`).**
   - Both say clearing the set would cost "the width it covers". `WrittenPositions::clear` costs the cleared span, not the width.
   - The real reason to replace the set is that `clone_from`'s bound ("the magnitude read plus O(`Q`)") excludes the receiver's written span.

b. **`copy_into`'s first sentence (`digits.rs:147`).** "exactly this stored prefix and its scan metadata" is exact only for a normalized source. `clone` guarantees that, but `copy_into` itself doesn't require it.

c. **The module doc (`written.rs:30`, `written.rs:58-60`).**
   - "a run of consecutive positions holding its largest elements" is false once `truncate` empties the run. This holds even with the repair, since the top can then sit below the run.
   - "No form ever converts back" is followed at once by an exception, clearing. It would read better stated positively, as the code is: "Only clearing returns a form to the empty prefix."

d. **Noun-phrase first sentences on private functions** (`last_below`, `contains`, `form`, `storage_words`, `positions`, `below_positions`, `bits`, `word_positions`, `after`, `bit`, `highest_bit`). Part IV wants verb-first. The crate already uses noun phrases for private accessors, such as `written_floor` and `stored_digits`, so I don't block on this.

e. **`activate`'s assertion message (`digits.rs:115`).** "inactive digit storage *writes* no position but zero" should say "holds".

f. **`skip_adequacy.rs:9`.** "the record of written positions, which crosses each unwritten gap" gives the record the action. "which lets a scan cross" puts it on the scan.

g. **The representation suite's doc (`representation.rs:126-130`).** It lists the checks, but `assert_invariants` also checks the new watermark clause, which the doc doesn't name.

h. **`Bitset::clear`'s cost sentence (`written.rs:553-555`).** It omits `remove(0)`'s climb (see suggestion 2).

## Suggestions that don't block

1. **Behavior-preserving: simplify `Bitset::clear`'s early stop** (`written.rs:556-572`).
   - Under the precondition, any nonzero word in the level-0 span makes every higher level's span nonzero. So the stop can only fire at level 0; above level 0 it can't be reached.
   - Measured: the loop stopped above level 0 in 0 of 256 model-test cases. It stopped at level 0 in 67, climbed every level in 154, and found its span beyond the stored words in 7.
   - Proposed form: check the level-0 span once, then fill each level's span. Saying this is what the code does reads more evidently correct than the current prose.

2. **Observation on `reset`'s per-call cost.** It is O(`R` + log₆₄ `W_retained`), because `remove(0)` climbs every retained level. Amortized, that cost is charged to the insertion of 0, which climbed the same levels during a write. So the code meets "O(`R`)" amortized, and under question 112's ruling that a base-64 log of at most 11 is effectively constant. `lib.rs:103` states it without "amortized".

3. **Question about the regression seeds.** Commit 1 adds three seeds that were generated against the older stack's strategies:
   - two in the new `digits/written/tests.txt`;
   - one in the new `tests/surface.txt`, whose shrink comment names `Operation::Normalize`, which today's `surface.rs` doesn't have.

   Under today's strategies these seeds replay different cases. The doctrine says never strip seeds, but these reproduce nothing here. That is a policy question for you. Commit 2's surface seed matches today's operations.

## What I checked and found sound

**Question 1: the invariant.** I read every writer of `written`, `lowest_written`, and the digits:

| Writer | Why it keeps the invariant |
|---|---|
| `add_at` | Lowers `lowest_written` before it inserts from `position`. |
| `descend_to_nonzero` / `truncate` | Only remove positions. |
| `compact_until_order_known` | Truncates, then deposits through `add_at`. |
| `normalize_nonzero_from` | Takes `floor` after `cmp_zero`, then rebuilds `{0} ∪ [floor, highest]`. |
| `copy_into` | Sets the target's `lowest_written` to the floor it inserts from. |
| `Digits::clear` / `activate` / `reset` | Leave the set empty or `{0}`. |
| `negate` | Touches only digits. |
| `shift_left` (`<<=`) | Builds a fresh `Digits`, activates it, then uses `add_at`. |
| `clone` / `clone_from` | Normalize the source, then go through `copy_into` or `Digits::clear`. |
| The integrator (`integral.rs:280-289`) | Reuses accumulators only through `reset` and `+=`. |

Every return to the small representation happens while the value is already small.

The one sharp case is `lowest_written > highest_nonzero > 0`, which would leave the top held below the watermark. It can't happen, because a nonzero top digit lies at or above the watermark. `surface.rs:1056` checks the invariant after every operation, `Reset` and `Negate` included, and `copies.rs` checks every copy and receiver.

**`clear`'s early stop is sound.** Under the precondition, an all-zero span at a level (checked before that level is touched) means the whole level is zero. By the summary invariant, every level above it is zero too.

**Question 2: `reset`'s cost.** Each level's span is clamped to the words stored at that level, so the retained width is never iterated. The cost is O(1 + `R`/64) at level 0 plus at most one word per climbed level.

**Question 3: space.**
- The porter's first claim stands. `main` has no public `normalize`, removed in `5ad6c74f4`, and its `lib.rs` has no "skip metadata" text.
- The second claim, that only the copy's bound covers the set, I dispute (finding 2).
- A bitset kept through `reset` or normalization is retained storage, which `lib.rs:105-109` permits ("may retain more allocation than its working width after cancellation or reset").
- Normalization allocates nothing for the set.
- The copy's set is inline, and `clones_take_written_set_space_for_the_value_alone` checks it against a source that keeps a 454-word bitset. I re-derived the 29,001 positions and 454 words that test relies on.
- Deleting the zero-result repair is justified: the O(`Q`) promise it protected no longer exists, and a zero clone goes through `Digits::clear`.

**Question 4: the model test.** I measured coverage across 256 cases of the committed strategy. Each transition fired in this many cases:

| Event | Cases |
|---|---|
| Prefix to mask | 67 |
| Short prefix to bitset | 164 |
| Long prefix to bitset (also exhausted at prefix 65 by `the_mask_holds_exactly_the_positions_below_64`) | 3 |
| Mask to bitset | 59 |
| `clear` on a prefix | 74 |
| `clear` on a mask | 12 |
| `truncate` removing from a bitset | 168 |
| `cover` adding a level at 2 or above | 148 |
| `clear` clearing three or more levels | 96 |

`Clear(below, above)` only generates spans that satisfy the precondition. Any stale summary or bit would fail `assert_structure` or `positions()`.

**Question 5: the board.** I checked the porter's comparison files with a script.
- Tip against base: the largest rise is 0.0514 B/B (`ranked_cmp × jump-comb`). There are 70 positive-slope rows, the steepest 0.02698 (`version_rank × wide-arming`). Seven touch cells fall and none rises; `wide-tooth-comb` falls by exactly 37,303.
- Commit 2 against commit 1: 385 heap readings fall and none rises.
- The flip: the tip's worst-case map reads 6.86 against 6.80 at the default scale, and 6.84 against 6.81 at the acceptance scale. The porter's argument that the fall alone causes the flip is sound.
- The touch-fall mechanism matches the code, though I inferred this rather than measuring it: a normalized set is `{0} ∪ run`, so a descent crosses from the floor to zero in one step.
- The porter's "bitset covers wide gaps" explanation of the 70 growing rises is incomplete: 25 of them come from finding 1. `weight-comb`'s entire rise is finding 1 (+592 B at the tip, −592 with the repair).

**Not checked:** the landing check itself; whether #117 will restate these bounds; the `cover` growth cost, which belongs to #118.

## Housekeeping and my own slips

- I removed the scratch worktree without `--force`, after restoring its files; my edits are kept as patches. The slot is untouched: tip `4a4396269`, clean.
- **Slip 1.** I removed the worktree before resolving its forge build directory, against Part III's order. The orphan is `/Volumes/forge/build/6d/9675a802e1d29c` (127 MB). I confirmed it holds my probe builds, and I left it for you to delete.
- **Slip 2.** I briefly copied a capture to `/tmp/rws-board-c2.txt` on the box, outside my own directory. I've deleted it and confirmed it's gone.
- The box directory `~/src/rws-repair` (332 MB, release build) is left for you to retire.

## Files

All in `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/review-written-set/`:
- `repair-truncate.patch`
- `probe-sign-query.patch`
- `coverage-instrument.patch`
- `probe1.log`, `probe2.log`, `suite-fix.log`, `cover.log`
- `box/compare-c2-repair.txt`, `box/compare-base-repair.txt`, `box/board-repair.log`
- `cmpstat.py`
- `NOTES.md`