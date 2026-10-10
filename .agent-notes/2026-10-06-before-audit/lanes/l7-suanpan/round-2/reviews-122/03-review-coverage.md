<!-- CAVEAT LECTOR: the test-coverage review of eedff98d, written by a Claude (Opus 5.5) agent and archived verbatim by the audit coordinator. Its claims are unverified unless the README says otherwise. -->

## Verdict: changes required (test coverage)

I mutation-tested `fix/suanpan-uniform-amortization` at tip `eedff98d` (base `a591f15ca`).

- **Base check:** HEAD equalled the tip and the worktree was clean.
- **What I ran:** 31 mutants of suanpan and 3 of `before`, compiled once behind an environment-variable switch, plus 9 probe tests of my own.
- **Survivors:** 14 of the mutants survive suanpan's own suite. Another 2 are caught only probabilistically or only by nextest's 300-second kill. One more survives suanpan's suite but is killed by `before`'s.
- **One defect at the tip:** a probe shows a documented cost claim is false (finding 2).
- **What holds:**
  - Same-cell handling and the RefCell re-entrancy hooks.
  - Formatting, hashing, `Ord`/`Eq`/`Hash` consistency across stored forms, and comparisons with primitives.
  - The new and first-commit meter pins.
  - The `before` call sites.
  - The deletion of `before/src/accumulator/tests.rs`.

The worktree is restored: `git status --short` prints nothing, `git diff` is 0 lines, HEAD is still `eedff98d`. The box's copy of the slot is re-synced and clean at the tip.

## Surviving mutants and gaps, most severe first

### 1. No test drives the operators with a borrowed left operand (8 surviving mutants)

The surface property applies operators only with an assigned or owned left operand. No test or doctest anywhere uses `&a + 5`, `&a + b`, `-&a`, or `&a << k`. The only tests of `&a + &b` and `&a - &b` use one cell twice (`same_cell.rs:64,72`). A mutant that reads the right operand twice therefore still gives `2a` and `0`.

Untested impls in `crates/suanpan/src/accumulator/operators.rs`:
- `:111` `&a + prim` and `:121` `&a - prim`
- `:268` `&a + b` and `:279` `&a - b`
- `:291` `&a + &b` and `:303` `&a - &b` (for a ≠ b)
- `:342` `-&a`
- `:397` `&a << k`, including its own panic arm at `:404`

Each mutant below survived all 92 existing tests and the 13 doctests. Only my probes killed them (`run-mutants-suanpan.log`, `run-batch2.log`):

| Mutant | Wrong behavior |
|---|---|
| `add_ref_prim_sub` | `&a + p` subtracts |
| `sub_ref_prim_add` | `&a - p` adds |
| `add_ref_owned_wrong` | `&a + b` returns `b - a` |
| `sub_ref_owned_no_neg` | `&a - b` returns `b - a` |
| `add_ref_ref_rhs_twice` | `&a + &b` returns `2b` |
| `sub_ref_ref_rhs_twice` | `&a - &b` returns `0` |
| `neg_ref_identity` | `-&a` returns `a` |
| `shl_ref_ignore_shift` | `&a << k` returns `a` |
| `shl_ref_no_assert` | `&nonzero << (2^64)` silently returns zero |

**Closing tests:**
- In `surface.rs`, add borrowed-left variants to `PrimitiveArithmetic` and `AccumulatorArithmetic`, a borrowed `Negate`, and a borrowed form of `Shift`. Invariant: every operator with a borrowed left operand agrees with exact arithmetic and leaves both borrowed operands' values unchanged.
- In `primitives.rs`, add `&value << count` to `primitive_shift_counts_are_checked`. Invariant: a borrowed nonzero value shifted past `u64::MAX` panics, and a borrowed zero (known or cancelled) shifts by `u128::MAX` to zero.

### 2. At the tip, `bits()` does not keep its "repeated read takes O(1)" claim

The claim is in `bits()`' rustdoc (`accumulator.rs:473`): "a repeated read before the next update takes O(1) time". The test doc of `bit_lengths_cost_their_top_digits` (`metered.rs:851-853`) says "every repeated read costs one touch". Both are false when another read comes between the two `bits()` calls.

Probe `bit_length_reread_after_a_sign_read`, at the tip with no mutant, on the normalized form of `2^(32·1000)`:
```
assertion `left == right` failed: bits() at 1000 digits: consecutive repeat, then repeat after sign()
  left: (1, 1006)
 right: (1, 1)
```
The mechanism, which I derived from the code before measuring (predicted k + 6):
- `sign()` compacts the top digit 1 into a single digit `2^32` one position lower, writing through `add_at`, which clears `normalized`.
- The next `bits()` cannot decide an exact power of two from the top three digits, so it re-normalizes all k digits.
- Alternating `sign()` and `bits()` therefore pays O(k) per `bits()` indefinitely.

The per-call amortized bound O(log(W+1) + Q) still holds; the O(1)-repeat sentence does not. The pin covers only back-to-back calls. **This needs the owner's ruling:** either `sign()`'s compaction should leave a normalized form intact, or both docs should restrict the claim to back-to-back `bits()` calls. The correctness reviewer may want this one. Once ruled, the closing test is the probe, extended into the pin.

### 3. The bit-length unread-digit bound (`digits/bits.rs:24,61-62`) is unprotected on one side and flaky on the other

The true bound on the unread digits is about 2.0000000005 units of the lowest digit read, so 3 is the smallest correct integer constant. But the existing tests barely press on it:

| Mutant | Existing exhaustion and pins | Surface property alone (20 fixed seeds, persistence off) |
|---|---|---|
| `bits_low_tight` (low uses 2) | pass | caught on 0 of 20 seeds |
| `bits_high_tight` (high uses 2) | pass | caught on 6 of 20 seeds (4, 7, 14, 17, 19, 20) |
| `bits_unread2` (both use 2) | pass | missed in its suite run |

Why the exhaustion misses: `Form::HeavyBelowTop` builds negative values by subtraction. Its redundant digit therefore always has the value's own sign and weighs about one unit. That probes neither the "light" direction nor the region beyond two units where these mutants go wrong.

**Closing test**, in `standard.rs`: for top partials within 4 of `2^p` (p in 64..=95), over 1 to 4 unread digits all at `+(2^33−1)` or all at `−(2^33−1)`, in both signs, `bits()` equals the big integer's bit length. My probe `bits_with_extreme_unread_digits_match_bigint` implements it (4,608 cases, 0.16 s). It kills all three mutants; the first failure was `2^64 + -2 over 2 digits, positive extremes true ... left: 128, right: 129`.

### 4. Nothing checks that a copy made with `From<&Accumulator>` sheds its source's cancellation

The rustdoc at `accumulator.rs:65-68` and `operators.rs:140-143` says the copy compacts the source first, "so the copy carries none of it". That compaction is the whole difference from the forbidden `Clone`.

Mutant `from_ref_clone` copies the stored form, as a `Clone` would. It passes all 93 existing tests (`run-mutants-suanpan-2.log`). My probe catches it: the copy keeps width 4 where the value needs 1 (`left: 4, right: 1`).

**Closing test**, in `same_cell.rs` or `standard.rs`: copying an accumulator whose top cancels leaves both the copy and the source at the value's own working width.

### 5. Nothing pins the comparison loop's `CLOSE_LEAD` boundary or its termination (`compare.rs:28`, loop at `:76-93`)

- **Boundary:** `cmp_close_lead3` survives. The pin `far_wider_comparisons_read_only_the_wider_top` uses a 996-digit lead, so it cannot see the documented "more than two digits" threshold.
- **Termination:** `cmp_close_lead1` loops forever. On this run it was caught only as `TIMEOUT [300.017s]` in `complete_surface_matches_bigint` (`run-batch5.log`).
- **Why the convergence property cannot help:** `two_sided_stability_converges_to_exact_comparison` (`differential.rs:463`) re-implements the protocol with its own literal `<= 2` (`:471`), so it never drives the production `compare`.

**Closing tests:**
- A metered pin: a lead of exactly three digits decides in 1 touch.
- A unit test: `2·B^(n+1)` compared with an n-digit value, for n in 1..=5, terminates and is exact in both orders. In this case the refused top digit re-carries the same digit 2.
- Add `prop_assert_eq!(left.cmp(&right), expected)` to the convergence property, so its refusal-targeting generator drives the real loop.

### 6. suanpan's own tests check `TryFrom<&Accumulator> for BigUint` (`interop.rs:119`) only at three points

The point test `biguint_conversion_refuses_negative_values` (`tests/interop.rs:67`) uses 7, 0, and −7, all single-limb, yet its doc states a family claim. Mutant `biguint_low_limb_only` survives suanpan's suite. `before`'s suite kills it (52 failures, via `rank` and `min_ticks`), so the landing check would catch it, but suanpan's contract is unprotected in suanpan.

**Closing test**, a proptest in `tests/interop.rs`: for every generated value, in any stored form, the conversion returns `Ok(|v|)` exactly when v ≥ 0, and `Err(NegativeValueError)` otherwise.

### 7. Test doc comments that claim more than their tests assert

- **`standard.rs:26-27`** (`HeavyBelowTop`): "the unread digits weigh nearly the full bound the top-digit read allows". They weigh about one unit, against a supremum just over two (finding 3).
- **`standard.rs:3-5`:** the module doc says "these cases decide whether the fast path defers to normalization when it must". They do not (finding 3).
- **`standard.rs:74`:** the doc says every value within four of a power of two "has the bit length and decimal text" of its big integer. Decimal is asserted only where `power % 97 == 0` (`:100`), which is 7 of 698 powers.
- **`surface.rs:1` and `:989`:** the module doc says "complete public arithmetic surface" and the test doc says "Every public arithmetic operation". The borrowed-left forms are absent (finding 1).
- **`metered.rs:851-853`:** "every repeated read costs one touch" is false (finding 2).
- **`before/src/version/measure/tests.rs:367-373`** (`zero_drift_never_freezes`): the doc says *neither* fold's freeze sees a zero drift. The test asserts only the integrator's `FREEZE_HITS`; `min_ticks`' freeze has no counter. A zero component there is still exact, so this is unasserted prose rather than a correctness hole.

## Non-blocking observations

- **The deleted `before` tests:** the builder's claim holds.
  - Mutants of `before`'s surviving `magnitude` packer are killed by `before`'s suite: `bmag_two_limb_shift32` (17 failures) and `bmag_wide_swap` (30).
  - So is `switch_swap` in `switch_delta` (143 failures).
  - The limb-entry tests are a strict subset of the surface property's `Limbs` operation.
  - One weakening: the replacement `bigint_operands_match_the_oracle` (`tests/interop.rs:42`) reads `BigInt::from(&acc)` in place after every step. Every operand therefore lands on a freshly normalized receiver, whereas the deleted test read only the sign and let redundancy build up. Reading a clone, as the surface property does, would restore that.
- **Killed by existing tests**, so these behaviors are protected: `bits_flag_ignored` (the bit-length pin), `bits_small_signed`, `read_no_writeback`, `hash_hold_borrow`, `hash_width`, `cmp_no_reverse`, `cmpprim_wide_greater`, `cmpprim_neg_swap`, `display_hold_borrow`, `display_no_flags`, `display_width18`, `operand_no_compact`, `tryfrom_no_query`. The builder's `ptr::eq` and `add_at` flag mutants are reported, not re-run here; both outcomes follow from the code.
- **Two equivalent mutants**, which are simplification candidates for the structure reviewer:
  - `bits.rs:63` `low >= 1 &&` is redundant, because `bit_length(0)` never equals `bit_length(high ≥ 2)`.
  - `bits.rs:42` `if top == 0` is redundant, because `highest_nonzero` is 0 whenever the top digit is 0.
- **`STOPPING_DIFF_BAND`:** the tree has `(8_973, 14_955)`. That is a reading of 11,964 at ×0.75 and ×1.25, per `eaad03a3c`'s message, which says "as the owner ruled". The ruling of record (question 125) quotes `(9_742, 16_237)`, a reading of about 12,989. The tree's band is lower, which the pinned-instrument rule permits. Still, the ready entry should cite the band actually in the tree, and the commit message should not attribute these exact numbers to the ruling.
- **Display coverage:** the surface property checks five format specs per step against num-bigint. Generating fill × align × sign × zero-padding × width would also name left alignment and plain zero padding. Optional.
- **Instrument note:** `before`'s heap meters caught my mutant switch's 4-byte `String` allocation. With `SUANPAN_MUTANT` set, `lattice_entry_points_clone_only_borrowed_results` reports "allocated 107 bytes, not 111", and `resident_space_comparison_is_scoped_by_shape` reports `left: 4`. Both pass with the variable unset. These are artifacts of my instrumentation, not branch defects, and I discounted them in every `before` count above.
- **Brief conflict:** `reviewer.md` asks for a `just gate` run; the task prompt assigns that to a sibling, and the prompt wins. I ran no landing check and took no timing measurements.

## Evidence (verbatim summaries; full logs in my scratch directory)

Scratch directory: `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/review-122-coverage/`

- **`schema.patch`:** every mutant, guarded by `SUANPAN_MUTANT`.
- **`review_probe.rs`:** all 9 probe tests, ready to adapt into the closing tests above.
- **`summarize.py`:** the per-mutant parser.
- **Logs:**
  - `run-baseline-suanpan.log`: `Summary [10.379s] 93 tests run: 93 passed, 0 skipped`
  - `run-mutants-suanpan.log`: 26 mutants, each run as `SUANPAN_MUTANT=$m cargo nextest run --locked -p suanpan --all-features --no-fail-fast`. Survivors show `93 tests run: 92 passed, 1 failed` with only a `review_probe` failing.
  - `run-mutants-suanpan-2.log`: `none: 94 passed`; `from_ref_clone: 93 passed, 1 failed (copies_carry_no_cancelled_width)`.
  - `run-batch2.log`: `before` with `none` gives `725 passed, 2 failed`, which are the schema artifacts. `bmag_two_limb_shift32` gives 17 failed, `bmag_wide_swap` 30, `switch_swap` 143. Doctests give `13 passed; 0 failed` under each of the 12 survivors.
  - `run-before-baseline-fails{,-2}.log`: the two artifact tests give `2 passed` with the variable unset and `0 passed, 2 failed` with it set to `none`.
  - `run-mutants-suanpan-3.log`: `none` fails only `bit_length_reread_after_a_sign_read` (`left: (1, 1006)`). `biguint_low_limb_only` fails that probe and `biguint_conversion_matches_the_sign_family` (`Ok(0)` vs `Ok(18446744073709551616)`).
  - `run-batch4.log`: the 40 fixed-seed runs (detection rates as in finding 3); `before` under `biguint_low_limb_only` gives `675 passed, 52 failed`.
  - `run-batch5.log`: `cmp_close_lead1` gives `97 tests run: 94 passed, 1 failed, 2 timed out` (`TIMEOUT [300.017s] review_probe::comparison_terminates_when_a_refusal_recarries`, `TIMEOUT [300.024s] surface::complete_surface_matches_bigint`). The `none` control passes the termination probe.
  - `final-sync.log`: the box reports `eedff98d…` with an empty `git status --short`.
