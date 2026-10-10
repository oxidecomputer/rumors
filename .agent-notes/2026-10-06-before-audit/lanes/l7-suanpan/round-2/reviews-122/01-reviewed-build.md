<!-- CAVEAT LECTOR: the build report for the reviewed tip eedff98d, written by a Claude (Opus 5.5) agent and archived verbatim by the audit coordinator. Its claims are unverified unless the README says otherwise. -->

## Verdict

Items 1 through 5 and 7 of the API ruling are built, plus the owner's additions (`Display` and a value bit length). The landing check is clean on every leg at the tip. I stopped at item 6 as instructed: the inherent-method list below awaits the owner's approval, and nothing beyond `cmp_zero` is removed. Two points need the owner's attention:

- **Assignment examples cannot compile.** The ruling's same-cell examples for assignments (`a += &a`, `a -= &a`, `a.add_shifted(k, &a)`) can't be written in Rust at all. I made that impossibility the protection and proved it with compile-fail examples (details under the same-cell cases).
- **The user's mid-task question found a real panic.** I fixed it and covered it with tests. I answer the question directly near the end of this report.

## Start state and base

- **HEAD at start.** When I first looked, HEAD resolved to `b34f086f`. A moment later it was `78dccf74e`, clean. The reflog shows a checkout from a detached `b34f086f` onto the branch, completing concurrently with my first command.
- **Rebases.** I rebased onto `830e3453f`. `main` then moved twice more while I worked: to `a8d72e28a` (exact sealing; one conflict in `worst.rs`) and to `a591f15ca` (the counting allocator; clean).
- **Final base.** `a591f15ca`, verified as an ancestor of the tip.

## SHAs (all signed; none unsigned)

The branch `fix/suanpan-uniform-amortization` sits on `a591f15ca`:

- `eaad03a3c`: suanpan: Compact what every read and accumulator operand touches.
- `4d7701f02`: before: Drop the measure folds' unreachable zero-drift freeze cases.
- `0d6e24ea4`: your affine commit (no `Clone` outside tests).
- `eedff98d65309eaeeb333d7ad84ccc025d39740b`: suanpan: Make Accumulator a standard numeric type, not Clone or Sync.

I kept the rework as one commit. `before` calls the removed methods, so splitting it would leave intermediate commits that don't build. Rebasing re-signed the first three; their content is unchanged.

## What was built

**1. Interior mutability.** A private `Value` holds the representation and every amortized algorithm on `&mut`. `Accumulator` is now `{ value: RefCell<Value> }`. Its `&mut self` methods go through `get_mut`, so the common path does no runtime borrow check.
- Reads take `&self`: `sign`, the magnitude reads, `cmp_zero_stable_under`, `bits`, and `Display`.
- Operands are `&Accumulator` again: `acc += &x`, `add_shifted(k, &x)`, and `iter().sum()`.
- The type is `Send` (tested by moving one across a thread) and not `Sync` (a compile-fail example). It is still not `Clone`.

**`Sync` check.** I compiled the whole workspace with every target and feature (`--keep-going`), plus wasm32-pins (guest on wasm32, and harness), fuzzfit, before-fuelscape, and surfacecheck. All compile, and nothing requires `Sync`. The fuzz workspace stops at `libfuzzer-sys`'s build script, as it always does on illumos. Its sources don't use `suanpan`, and `before`'s public API is unchanged, so it cannot depend on `Sync`.

**2. Same-cell safety.** Operations taking two shared accumulators handle the same one twice by construction:
- Comparisons check `ptr::eq` before borrowing either cell.
- `&a + &a`, `&a - &a`, and sums of repeated borrows borrow the cell one read at a time, so they need no special case.
- Every operation with a `&mut` or owned receiver is rejected by the compiler when given its own receiver.

**3. Standard traits.**
- `Add`, `Sub`, `Neg`, `Shl`, their assign forms, and `Sum`, over owned, borrowed, and primitive operands. Borrowed left operands are included: `&a + &b`, `&a - b`, `-&a`, `&a << k`, `&a + 5`.
- `From<&Accumulator> for Accumulator`, which copies a value by adding it to zero (compacting the source first).
- `Ord`, `PartialOrd`, `Eq`, and `PartialEq`. A far-wider side decides through its stability query without reading the narrower one; otherwise the comparison subtracts into a temporary.
- `PartialOrd` and `PartialEq` against all 12 primitive types, with the accumulator on the left.
- `Hash` of the normalized sign and magnitude, so equal values hash equally across stored forms.
- `sign() -> Ordering`, `is_zero`, `is_positive`, and `is_negative`. These replace `cmp_zero`. I named the three-way query `sign`, as `num-bigint` does, because num-traits' `signum` returns `Self`, which here would allocate.
- No multiplication, division, or right shift.

**Finding: no primitive on the left.** I first added impls with the primitive on the left (`5 < acc`). They broke type inference in `rumors-testkit`, turning an `Arc<u64>` against `u64` comparison ambiguous, because an impl of a standard comparison trait for a primitive type changes inference in every crate that links the impl. That would have forced changes in `rumors` crates, which is a stop condition, so I removed them. The reason is written in `compare.rs`.

**Owner's additions.**
- **`Display`**: decimal through the normalizing read, padded by `Formatter::pad_integral` as `num-bigint` does, so `{:+}`, width, fill, and zero padding all work. Converting `Q` limbs to decimal is O(`Q`²), like `num-bigint`'s non-power-of-two radix, and documented.
- **`bits()`**: the bit length of the magnitude, distinct from `stored_bits`.
  - **Fast path.** After the compacting comparison with zero, it reads up to three top digits. The unread lower digits are worth less than 3 units of the lowest digit read, so when both ends of the resulting interval have the same bit length, that length is exact.
  - **Fallback.** Near a power of two the top digits cannot decide, so the read normalizes the value once.
  - **Repeats.** A `normalized` flag in `Digits` (set by the normalizing pass, cleared at the single write boundary `add_at`, preserved by negation) lets every repeat before the next update answer in one touch.

**4. `num-bigint` feature.**
- Conversions into an accumulator: `From<BigInt>`, `From<&BigInt>`, `From<BigUint>`, `From<&BigUint>`.
- Conversions out: `From<&Accumulator> for BigInt`, and `TryFrom<&Accumulator> for BigUint`, which fails with `NegativeValueError`, a unit struct implementing `Display` and `Error`, exported at the root under the feature.
- `+=` and `-=` with owned and borrowed `BigInt` and `BigUint` operands.

`before` enables the feature. The detached workspaces' lockfiles gain a three-line entry for `suanpan`'s new optional dependency; I resolved them on the Mac with `cargo metadata` and built nothing locally.

**5. `Debug`.** Its doc says the output describes the stored form, so it can change when the value does not.

**7. Docs.** I rewrote the crate page in short, plain sentences:
- operations as a standard integer;
- a section on why reads compact what they touch, and so take `&self`, with no `Sync` and no `Clone`;
- an updated cost table, with rows for comparisons, `bits`, `Display`, and `Hash`;
- a Scope section with no `PartialEq` caveat and no subtract-and-compare advice;
- both features.

I regenerated the README with `tools/readme write`. Per the user's mid-task request, every file I touched got a pass for concise, maintainer-altitude prose. In the operator impls, I replaced repeated multi-sentence cost paragraphs with one line each in the crate cost table's terms. I also simplified code: the bit-length loop, `switch_delta`, and the test-only borrow helpers, which duplicated `peek()` and `value.get_mut()`.

## Same-cell cases and their tests

Same accumulator as both operands, possible at runtime (tests in `tests/same_cell.rs`):

| Case | Expected | Test |
|---|---|---|
| `a == a` | true | `equals_itself` |
| `cmp`, `partial_cmp`, `<=`, `>=`, `<`, `>` with itself | `Equal` | `orders_equal_to_itself` |
| `&a + &a` | `2a` | `borrowed_sum_with_itself_doubles` |
| `&a - &a` | `0` | `borrowed_difference_with_itself_is_zero` |
| `[&a, &a, &a].sum()` | `3a` | `sum_of_repeated_borrows_multiplies` |
| hashing twice | same hash | `hashes_itself_consistently` |

Each value carries a cancelled top, so these reads compact it.

Statically excluded, each with a compile-fail doctest:
- `total += &total` and `total -= &total` (E0502);
- `total.add_shifted(1, &total)` and `total.sub_shifted(1, &total)`;
- `total + &total` (owned receiver moved while borrowed).

**Mutant.** Removing the `ptr::eq` guard fails `equals_itself` and `orders_equal_to_itself` (a RefCell borrow panic). The file was restored and checked with `cmp`.

## Your question: can aliasing panic the RefCell?

Yes, comprehensively covered now. There are two ways to reach a borrow panic.

1. **The same accumulator on both sides.** That is the table above, plus the compile-fail examples.
2. **Caller code running while a borrow is held and reading the same accumulator.** Caller code enters this API at exactly three hooks: the `fmt::Write` sink behind a `Formatter` (`Debug`, `Display`), the `Hasher` (`Hash`), and iterators (`Sum`; the limb streams run under `&mut self`, which no caller can read).

Auditing those hooks found a real bug: **`Debug` held a shared borrow while writing to the formatter.** A sink that called `acc.sign()` during `write!(sink, "{:?}", acc)` panicked with "RefCell already borrowed". Restoring the old `Debug` as a mutant reproduces exactly that panic in the new test.

The fix: `Debug` renders into a `String` under the borrow (a `String` runs no caller code) and writes it out only after the borrow ends. `Display` and `Hash` already released their borrows before running caller code, and iterators run only between borrows.

New tests in `tests/same_cell.rs`:
- `formatting_sinks_may_read_the_accumulator`: every formatting trait, writing to a sink that reads the accumulator on each write.
- `hashers_may_read_the_accumulator`: a hasher that reads it while hashing.
- `sum_iterators_may_read_their_operands`: an iterator that reads its operands between yields.

The field doc on `Accumulator.value` states the invariant: no borrow is held while caller code or another accumulator method runs, and two cells are borrowed only after checking that they differ.

## Inherent methods: keep or remove (awaiting approval; only `cmp_zero` is removed)

| Method | Recommendation | Reason |
|---|---|---|
| `new` | keep | The conventional constructor; `Default` also exists. |
| `add_shifted_limbs`, `sub_shifted_limbs` | keep | Streams arbitrary widths with no allocation and no `num-bigint`; `before`'s whole numeric layer feeds through them. |
| `add_shifted`, `sub_shifted` | keep | Merge a shifted accumulator in O(`A`); `acc += &(&x << k)` would first build an O(`A` + `k`) temporary. The integrator depends on them. |
| `reset` | keep | Clears while retaining the allocation; `acc = Accumulator::new()` releases it. |
| `reserve_bits` | keep | The only allocation hint. |
| `sign`, `is_zero`, `is_positive`, `is_negative` | keep | The ruled standard sign queries. `is_zero` overlaps `acc == 0`, but follows num-traits naming. |
| `cmp_zero_stable_under` | keep | Ruled to stay; comparisons use it internally. |
| `is_known_zero` | keep | Ruled to stay; it is the O(1) check that reads nothing. |
| `stored_bits` | keep | The working width that `cmp_zero_stable_under`'s protocol is written in. |
| `stored_digit_count` | **remove (proposed)** | Duplicates `stored_bits / 32`. `before` calls it at about 15 sites (freeze triggers, `HeightWidths`), which would compare widths in bits instead. |
| `normalize` | **remove (proposed)** | Every read now normalizes what it touches, and nothing outside `suanpan`'s own tests calls it. Its only visible effect is tightening `stored_bits` without producing output. Keep it if an explicit compaction without a read is wanted. |
| `bits` | keep | New, as the owner asked. |
| `signed_magnitude` | keep | The limb readout without `num-bigint`; `Hash`, `Display`, and the interop use it. |
| `scaled_signed_magnitude` | keep | `before`'s `ScaledWidth` relies on it to skip a known-zero prefix. |

A candidate I didn't build: `TryFrom<&Accumulator>` for the primitives, as `num-bigint` offers for `&BigInt`. It would need an error type.

## What `before` code was deleted

- **The `BigIntAccumulator` trait and its impl.** `biguint_parts`, `to_bigint`, and `into_bigint` become `BigInt::from(&acc)` and `BigUint::try_from(&acc)`. `add_bigint`/`sub_bigint` become `+=`/`-=` with a `&BigInt`.
- **Kept as free functions:** `digit_len`, and `scaled_biguint_parts`, which the interop doesn't replace (it keeps the factored scale).
- **`before/src/accumulator/tests.rs` (three proptests).** Two exercised `suanpan`'s limb entry points, which its surface property already covers against an oracle. The signed big-integer operand test moved to `suanpan`'s interop tests.
- **Signatures back to `main`'s.** Thirteen files that only threaded `&mut` through read paths are restored exactly to `main`'s versions, signatures included. Only their call sites moved to `sign()` and the interop.
- **`switch_delta`** now returns `D` for side A and `-D` for side B directly.
- **`rank`'s numerator and `min_ticks`' count** read through `BigUint::try_from(..).expect(..)`. That turns their former `debug_assert`s into release panics, reachable only by programmer error.

## Moved readings, with mechanisms

**The rework commit alone.** I compared `0d6e24ea4`'s parent tree (captured as `e237eae29` before the final rebase) with the rework tip, using the board's exact capture and compare:
- **No touch reading moves.** Every touch path does the same digit work.
- **641 heap rows move.** `RefCell` adds a borrow-flag word to every accumulator.
  - 579 move by a constant per cell (for example +8, +96, +104, or −192 bytes).
  - 44 grow linearly, with slope at most +0.067 heap bytes per input byte: rank, distance, lag, and the ranked codecs over `promo-rearm`, `dense-suffix`, and `arming-train`, which hold Θ(n) accumulators. For example, rank over `promo-rearm` reads +8,208 bytes at 126,465 input bytes and +16,400 at twice the size, about 8 bytes per held accumulator.
  - 18 move unevenly across sizes.
- **Verdict:** all within ceilings (board 5,361 green), and the worst-case pin is clean at 272 rows.

**The new meter pins**, derived from the code before running:
- `bit_lengths_cost_their_top_digits`: `5·2^(32k)` costs 3 touches at any width. `2^(32k) − 1` costs `k + 5` once, then 1 per repeat.
- `far_wider_comparisons_read_only_the_wider_top`: 1 touch, in either operand order.
- No existing pin moved.

**Adequacy finding.** A mutant shrinking the unread-digit bound from 3 to 1 slipped past my first near-power exhaustion; only the surface property and a meter pin caught it. I added a stored form that puts a redundant digit `2^32` just below the three digits read. The exhaustion alone now catches that mutant: "first read of 2^160 + 0, negative false, HeavyBelowTop: left: 160, right: 161".

**Other mutants, each restored and checked with `cmp`.** Not clearing the `normalized` flag at `add_at` fails 6 tests (bits, invariants, surface). Removing the `ptr::eq` guard fails 2 tests. Restoring the old `Debug` fails the formatting-sink test.

## Landing check at `eedff98d6`: clean

| Leg | Result |
|---|---|
| tests | `before`+`suanpan` 815 run, 815 passed (1 skipped); `rumors` snapshots 142/142 |
| docs | doctests 193 (`before`) and 13 (`suanpan`: 4 earlier, plus the new examples and compile-fail cases) |
| lints | clean, including `doclint` and clippy in both feature configurations |
| surface | 14 |
| board | 5,361 green; worst-case pin clean (272 rows) |
| wasm | wasm32-pins 12, fuzzfit 25, fuelscape 43 |

The test count against `main` changes by +3 metered (the first commit), −3 (`before`'s deleted accumulator tests), and +17 new in `suanpan`: 10 same-cell, 2 bit-length and decimal, 3 interop, 2 metered.

The first two landing attempts failed on things I then fixed: two doc summaries over `doclint`'s 220-character limit, and an `unused_mut` in the wasm guest. Clean logs are in `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/builder-122/landing7.log`, the board captures are in `cap/`, and my notes are in `NOTES.md` there.

## Main files

- `/Users/oxide/src/rumors-slot-51/crates/suanpan/src/accumulator.rs`
- `.../accumulator/value.rs`, `compare.rs`, `format.rs`, `interop.rs`, `operators.rs`, `digits/bits.rs`
- `.../accumulator/tests/same_cell.rs`, `standard.rs`, `interop.rs`
- `/Users/oxide/src/rumors-slot-51/crates/suanpan/src/lib.rs`
- `/Users/oxide/src/rumors-slot-51/crates/before/src/accumulator.rs`
