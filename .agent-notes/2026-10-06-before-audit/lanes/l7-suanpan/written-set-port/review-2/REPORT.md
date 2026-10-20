<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), reviewer review-written-set-2, of fix/suanpan-written-set at 9ce81e542. -->

# Review of `fix/suanpan-written-set` (`9ce81e542` on `main` `054cdd1fb`), round 2

## 1. Verdict

**Changes required.** The three-layer structure works and the correctness of every transition holds up. Two blocking problems remain. I wrote and checked a repair for the first; the second has two possible fixes, one of which I also checked.

1. **Wrong cost claim.** The bitset's documented predecessor cost, O(1 + log₆₄ d) for a skip of d positions, is false. Measured: a one-position skip across an aligned boundary costs the height of the hierarchy. The repair is 13 lines and makes the documented bound true.
2. **Wrong conversion claim, and a test that pins the contradiction.** The set converts its prefix to a bitset even when the set still has only one gap. The module doc and commit message say only a second gap converts. The test named `a_second_gap_and_only_a_second_gap_converts_the_prefix` asserts the conversion in a one-gap case, so its name and doc are false.

There is also one item the owner must rule on, which is a process question rather than a code defect. The worst-case flip the porter re-pinned depends partly on a regression (section 4, first item).

Base check: the slot's HEAD is `9ce81e542`, its parent is `054cdd1fb`, and the tree is clean. The board captures the porter compared ran at `6ce9efd2f`; its tree (`3c6c89e7`) is identical to the tip's, so they apply to this commit.

- **Landing check.** I did not run it, because you said one is already running at the tip.
- **Local runs.** Where the common brief says to run everything on the box, I followed your prompt: targeted local tests under `nice -n 10` with `CARGO_BUILD_JOBS=4`.
- **No injected defects.** Every probe and alternative below is a measurement or a correct design alternative.

## 2. Blocking findings, most severe first

### B1. `Bitset::last_below` does not cost O(1 + log₆₄ d)

**Where the claim lives:**
- `crates/suanpan/src/accumulator/digits/written/bitset.rs:23-25`
- restated at `crates/suanpan/src/accumulator/digits/written.rs:49-51`
- the commit message ("answering predecessor queries in O(1 + log_64 d) for a skip of d")

**Mechanism.** The climb (`bitset.rs:118-125`) stops only when the query's own word, at the current level, holds a bit below the query. Take a query at p = 64^k whose answer is p − 1, so d = 1. At every level the query's bit is bit 0 of its word, so the mask is empty. The climb therefore rises k levels and descends k levels.

**Evidence.** I added a test-only counter of words read inside `last_below`, in a scratch worktree at the tip. Verbatim output:

```
PROBE k=1 query=64 answer=63 d=1 words_read=3
PROBE k=2 query=4096 answer=4095 d=1 words_read=5
PROBE k=3 query=262144 answer=262143 d=1 words_read=7
PROBE k=4 query=16777216 answer=16777215 d=1 words_read=9
PROBE k=5 query=1073741824 answer=1073741823 d=1 words_read=11
```

The same d = 1 skip inside a word reads 1 word at every k. The cost is O(1 + log₆₄ W), the height of the hierarchy, not O(1 + log₆₄ d).

**Contract.** `suanpan`'s public rows still hold. Updates and sign queries are already allowed amortized O(log(`W` + 1)), and `lib.rs` defines `W` as the greatest working width reached. The private module docs and the commit message state a bound stronger than the code meets. The message's motivation also rests on it: the old map's O(log N) per operation broke `before`'s documented O(n), and as shipped the bitset keeps an O(log₆₄ W) term per query.

That term is reachable only through queries inside the bitset's coverage. Queries in the run, in the prefix, or above the coverage cost O(1). I have not built a `before` input that drives it.

**Repair, written and checked.** At each level, check the whole preceding word before climbing. An answer within d positions is then found by level ⌈log₆₄ d⌉ − 1. The patch is `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/review-written-set-2/repair-last-below.patch`; it applies cleanly at the tip:

```rust
            // Nothing lies below the query in this word. The word before it
            // holds every position within one word's span below, so a query
            // whose answer lies `d` positions down stops climbing by level
            // log₆₄ `d`. The last level has one word, so the climb ends there.
            let previous = word_index.checked_sub(1)?;
            if words[previous] != 0 {
                found = Some((level, previous * WORD_BITS + highest_bit(words[previous])));
                break;
            }
            // Both words are empty, so the answer lies in an earlier word,
            // whose summary bit sits below `previous`'s one level up.
            word = previous / WORD_BITS;
            below = bit(previous) - 1;
```

Checks on that change:
- **Probe.** With the repair, the probe reads 2 words for d = 1 at every k from 1 to 5.
- **Format, lint, suite** (the clean patch, which is the tested version minus the probe counters):
  - `cargo fmt -p suanpan --check`: exit 0.
  - `cargo clippy -p suanpan --all-targets --all-features --locked -- -D warnings`: exit 0, after adopting clippy's `?` form.
  - `cargo nextest run -p suanpan --all-features`: "103 tests run: 103 passed, 0 skipped".
- **Model tests at 20,000 cases** (`PROPTEST_CASES=20000`, the same logic with probe counters): both `bitset_matches_an_ordered_set` and `written_positions_match_an_ordered_set` pass.
- **Branch coverage.** The new preceding-word branch fired 55,672 times across the bitset model test's 256 default cases, so the oracle exercises it heavily.
- **Follow-up prose.** The Costs bullet at `bitset.rs:23-25` should then say the climb continues "while neither the query's word below it nor the word before it holds a position".

Results are identical under the repair, so it preserves behavior; it changes only cost. The other fix is to weaken the prose to O(1 + log₆₄ W). I recommend the repair, because it makes the documented bound true.

### B2. One gap converts the prefix, and the conversion test pins it under a name that denies it

**Where:**
- module doc, `written.rs:26-29`: "any other span opens a second gap, which converts the prefix"
- `written.rs:87`: "allocated when a second gap first appears"
- `written.rs:176`: "The span opens a second gap, which only a bitset can hold."
- commit message: "held inline as a prefix 0..n until a second gap converts it", and "exhaustive tests cover when a prefix converts"
- the test, `written/tests.rs:236-241` and `:250-260`

**Mechanism.** `insert_span_beside_run` sends any span below the run to `insert_below`. A span that ends at `run.start − 1` and starts past the prefix's end therefore converts the prefix, even though it only shortens the existing gap.

**Evidence through the public API.** Adding at shift 32·100, then at 32·99, produces this (probe output, verbatim):

```
PROBE top 100, then 99: positions [0, 99, 100] form bitset words 5
PROBE descending 100, 99, 98, 97: positions [0, 97, 98, 99, 100] form bitset words 5
```

Each of those sets has one gap. The test's "split the gap" case with `run = prefix + 2` writes `prefix + 1`, which is adjacent to the run: `{0, 3}` plus 2 becomes `{0, 2, 3}`. The test expects `"bitset"`, while its doc says conversion happens "exactly when an operation opens a second gap".

**Two resolutions:**

- **(a) Design change, checked.** When the form is `Prefix(end)` with `end < first` and the span reaches the run (`last >= run.start - 1`), lower the run's start: `run = first..max(run.end, last + 1)`. That is O(1), and the run still lies above every other position. I tried this in the scratch worktree:
  - `cargo nextest run -p suanpan`: "105 tests run: 104 passed, 1 failed". The sole failure is `form after: split the gap with prefix 1 and run at 3`, the mislabeled case.
  - The set's model test passes at 20,000 cases.
  - Descending writes below the top stay inline: `[0, 97, 98, 99, 100] form prefix words 0`.
  - Its board effect is unmeasured; I expect it to remove some of the +56 B constant shifts. It would need a box capture and a pin check.
- **(b) Fix the text only.** Restate the three code sites and the commit message as "any other span converts the prefix". Then rename and redocument the test, or move "split the gap" to `run >= prefix + 3` and add the adjacent case as its own documented conversion.

## 3. Prose findings against Part IV

**P1. Blocking. `written.rs` describes the bitset's internals,** which answers your question 1 in the negative:
- `:44` counts "the [`Bitset`]'s own word operations";
- `:50` restates the bitset's cost, wrongly (B1);
- `:58-59` restates its space ("one bit per position ... plus summaries");
- `:259`: "A bitset clears only the words that can hold such a position".

Each bitset fact is then stated twice, and B1 shows the hazard: the duplicate went wrong. Replace these with links to `Bitset`'s Costs and Space sections.

**P2. Blocking. The commit message has factual errors:**
- The O(1 + log₆₄ d) claim (B1).
- "until a second gap converts it" (B2).
- "A normalizing pass ... so the set becomes position zero and one run, held inline". With a retained bitset and a floor above 1, position zero lands in the bitset. Probe, after `signed_magnitude()` on writes at 5, 10, 20, 50, and 100: `first [0, 5, 6] last Some(100) count 97 form bitset`. `normalize.rs`'s own doc doesn't say "inline" and is correct.
- A minor point: "208,216 -> 170,913" puts arrows in running prose.

**P3. Non-blocking. `Digits::reset`'s doc (`digits.rs:240-243`) carries the bitset's per-level amortization argument** ("one word for each of its bitset's levels") two layers above the mechanism. Meanwhile `WrittenPositions::clear` (`written.rs:256-260`) and the module's Costs list (`:55-56`) never say the clear is amortized. The argument should live in `written.rs` (or link `Bitset::clear`), with `reset` pointing down to it.

**P4. Non-blocking. The test doc at `tests/representation.rs:512-514` implies the shape "one gap each side" lacks a bitset.** It contrasts that shape with "enough gaps that the set keeps a bitset". But `{0, s/2, s}` has two gaps, and the probe shows `positions [0, 50, 100] form bitset`. The prefix form is covered only by the lone-top shape. The test's coverage is fine, but its doc misleads about which shape reaches which form.

**P5. Non-blocking. `WrittenPositions::clear` and `Bitset::clear` don't classify what violating their precondition does.** Both say "given that each is zero or lies in `lowest..=highest`". A violation in release builds clears summary words whose sibling words still hold bits, so the summary invariant breaks and later queries become unspecified. Part IV asks for that consequence to be stated.

**P6. Non-blocking. Smaller wording points:**
- `written.rs:1-2`, "above a small set": the positions below can be a bitset of thousands.
- `skip_adequacy.rs:9-12`: a comma list whose first item contains a comma clause makes "three families" hard to parse; semicolons fix it.
- `digits.rs:27-28` and `:78-80` state the third fact twice, against your "documented once" bar; the module doc could point to the field.

## 4. Non-blocking suggestions, an owner item, and checks

**Owner ruling needed: the regression-assisted flip.** Default-scale heap per input byte for `version_rank`, `ranked_encode`, and `ranked_encode_rank`, which I verified from the raw captures:

| Family | `main` | tip |
|---|---|---|
| arming-train | 6.9017 | 6.8372 |
| wide-arming | 6.8162 | 6.8431 |

arming-train's fall alone (6.837) would stay above wide-arming's `main` reading (6.816). The flip therefore needs wide-arming's +0.027 B/B rise, which is a regression. `common.md` lists "a ranking flips because of a regression" as a stop for the owner, not a self-re-pin. The commit message states this honestly ("Both moves contribute"). At the acceptance scale arming-train keeps the rows, 6.876 against 6.803.

**Simplifications:**
- **B1's repair** preserves behavior.
- **B2(a)** is a design proposal: it changes the representation and heap, not results. It also makes parking unnecessary for the prefix form; keep parking for the bitset form.

**The `roaring` rejection: the conclusion is defensible, but two of its three reasons are mis-stated.** I read `roaring` 0.11.5's source.
- **Accurate:** `RoaringBitmap` is u32-only, so `usize` positions need `RoaringTreemap`.
- **Wrong conclusion from accurate citations.** `treemap/inherent.rs:221-243` and `bitmap/inherent.rs:427-455` do iterate every entry. But both drop empty bitmaps and containers (`keys_to_remove`; `self.containers.remove(index)`), so roaring retains nothing. Their loops visit only nonempty chunks, which for `reset` are position zero's and the span's, so the cost follows the span, not retained storage.
- **True but not decisive.** The predecessor query does cost a binary search over containers, `iter.rs:189-219`. But the bitset as shipped is also O(levels) in the worst case (B1).
- **What actually separates them:**
  - A predecessor query inside a dense bitmap container scans backward up to 1,024 words (`bitmap_store.rs:729-810`).
  - With B1's repair, ours meets O(1 + log₆₄ d), which roaring cannot.
- **What the rejection omits.** Roaring's array containers take 2 B per position and nothing for empty 65,536-position chunks. On sparse wide sets that beats our one bit per covered position, and wide-arming's growing heap rise (slope 0.018 B/B) is exactly that cost. The rejection is sound if predictable predecessor cost outranks sparse space; that trade should be recorded.

**Checked and sound:**
- **Transitions.** I checked every one:
  - the run extending or jumping;
  - spans below the run;
  - one-way prefix-to-bitset conversion, charged to the writes that built the prefix;
  - `truncate` on all three branches, removals paid by the scan's visits;
  - `last_below` on run, prefix, and bitset;
  - `clear(lowest, highest)` under its precondition, including the early return and the guarantee that no level's span starts past its storage.
- **Parking.** Round 1's repair holds for stops in the run, the prefix, the gap, and the bitset: the write-back never grows a bitset. `sign_queries_add_written_set_space_independent_of_width` catches its loss, because the bitset would grow by about s/64 words.
- **`lowest_written`.** Every writer keeps "nothing below `lowest_written` but zero": `activate`, `copy_into`, `clear`, `reset`, `add_at`, `descend_to_nonzero`, the normalizing pass (`floor` taken after `cmp_zero`), and sign compaction (truncate, then `add_at`).
- **Bitset invariants.** Both hold under `insert`, `remove` (including beyond storage), `cover` (new levels summarize the level below; grown words are zero), and span clear.
- **`reset` cost.** It is amortized O(`R`), and the comment's charging is not circular. `reset` re-inserts zero into the *run*, so the bitset is empty afterwards. The later per-level work is charged to writes with an O(log(`W` + 1)) budget, under `lib.rs`'s historical `W`. Per call it is O(`R` + log₆₄ W), as round 1 recorded.
- **Clones.** A clone's set is inline, and the clone test catches a source-sized bitset (about 460 words against a bound of 2).
- **Board figures.** Every figure in the message matches the raw captures, recomputed with my own script:
  - touch falls on seven cells and rises on none, and wide-tooth-comb goes 208,216 to 170,913;
  - no scan reading moves;
  - the constant heap shifts are +56 on 93 rows, −136 on 75, −192 on 23, and +112 on 18;
  - 267 readings rise at the largest size, by at most 0.0739 B/B;
  - 49 rows are growing rises, the steepest 0.0746 B/B and 0.0182 B/B elsewhere;
  - the rank rows fall by up to 0.567 B/B.
- **Module boundaries.** `bitset.rs` stands alone: it contains no digit or scan vocabulary.
- **`usize`.** It appears only as buffer positions and indices, with nothing whose meaning depends on pointer width.

**Not checked:**
- the landing check;
- the message's claim that the set's positions are "exactly the complement of the map's ranges";
- the board effect of B2(a);
- the cost of `cover`'s growth, which another branch (`fix/suanpan-cover-growing-levels`) appears to address.

## 5. Evidence and housekeeping

Commands, all local under `nice -n 10` with `CARGO_BUILD_JOBS=4`, in a scratch worktree at `9ce81e542`:

| Run | Result, verbatim |
|---|---|
| Probes at the tip | `Summary ... 2 tests run: 2 passed` |
| Suite with B1's probe version | "105 tests run: 105 passed, 0 skipped" |
| B1's clean patch | "103 tests run: 103 passed, 0 skipped"; fmt and clippy exit 0 |
| Model tests with B1's probe version, 20,000 cases | 2 passed |
| Suite with B2(a) | "105 tests run: 104 passed, 1 failed" (the mislabeled case) |
| Set model test with B2(a), 20,000 cases | passed |

The scratch worktree was removed without `--force`. First I re-resolved and recorded its build directory, `/Volumes/forge/build/a6/b50e9b04ce6dcc`, then deleted it (170 MB) and the worktree's `target/`. The slot is untouched: HEAD `9ce81e542`, `git status --short` empty.

Files are in `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/review-written-set-2/`:
- `repair-last-below.patch`
- `experiments-all.patch` (probes, B1 and B2(a) changes, and the normalized-form probe)
- `probe1.log`, `probe-fix.log`, `probe-norm.log`, `hits.log`
- `suite-fix.log`, `suite-repair-clean.log`, `suite-downward.log`
- `model-fix-20k.log`, `model-downward-20k.log`, `clippy-repair.log`
- `py/board.py`
- `NOTES.md`