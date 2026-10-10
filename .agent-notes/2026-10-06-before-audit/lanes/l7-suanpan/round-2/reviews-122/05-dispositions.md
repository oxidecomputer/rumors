<!-- CAVEAT LECTOR: the fix builder's report at e0956b25d, with a disposition per finding, written by a Claude (Opus 5.5) agent and archived verbatim by the audit coordinator. Its claims are unverified unless the README says otherwise. -->

## Verdict: built and landing-check clean at `e0956b25d`, with mutation evidence left partial

Branch `fix/suanpan-uniform-amortization` in `/Users/oxide/src/rumors-slot-51` is rebased onto `main` = `d1aa90e04`. That is the `main` at release, notes-only since `a591f15ca`; the rebase applied with no stops. The final tip is `e0956b25d35e488439bbb47e60c227fa00e7968b`. All 19 commits are signed (none unsigned), and every commit passed `cargo clippy --all-targets --all-features -D warnings` on its own before the last two folds; those folds changed a doc comment and two docs/tests in the last commits.

**Mutation evidence is partial, by choice.** You told me directly, mid-task, "Don't run mutation testing. That is overkill." The coordinator then relayed an owner clarification that only `cargo mutants` was meant, and asked for targeted env-switched mutants. That relay contradicts your direct instruction, so I did not act on it. If you confirm that the targeted schema is wanted, it is one build plus seven short runs: the `From<&Accumulator>` copy, the comparison threshold and termination, the `BigUint` property, `abs_diff` (order, magnitude, and which buffer is reused), the `Debug` flags, the query at exactly the lead, and the bit-offset `SparseWidth`. What exists is listed below.

## Final SHAs, oldest first
| SHA | Purpose |
|---|---|
| `b64ec2c17` | suanpan: Compact what every read and accumulator operand touches. Same tree as `eaad03a3c`, reworded: the band paragraph no longer attributes (8,973, 14,955) to the ruling. |
| `e6780cf49` | before: Drop the measure folds' unreachable zero-drift freeze cases. Adds a test-only `min_ticks::FREEZE_HITS`; `zero_drift_never_freezes` asserts both folds' freeze counts. |
| `543878f41` | suanpan: Make Accumulator affine (unchanged). |
| `708991d49` | suanpan: Make Accumulator a standard numeric type, not Clone or Sync. Folded in: `bits`, the `normalized` flag, and `digits/bits.rs` removed; `read`/`peek` take closures; `Value::addressable_shift`; the `Value::assert_invariants` test helper; `before`'s `&mut` residue; interop one/two-limb fast paths; `Debug` honors the caller's flags; `value.rs` module doc; `sub_shifted`'s `# Complexity`; new message. |
| `5ad6c74f4` | suanpan: Remove normalize. |
| `ce3d63a92` | suanpan: Keep a shifted zero's digit representation. The code was the wrong side; it now matches the module doc. |
| `5566af172` | suanpan: Decompose primitives into sign and magnitude in one place. |
| `114429402` | suanpan: Test borrowed-left operators, copies, and the comparison threshold. |
| `a4a2e5126` | suanpan: Accept primitives on the left and by reference (ruling 2). |
| `153095c68` | before: Drop dead sign reads and finish rank integrals as accumulators. |
| `278565a8b` | suanpan: Return magnitude limbs as a named `Limbs` type, inline below 2^128 (24 bytes, as small as a `Vec`). |
| `e8df0474a` | before: `ScaledWidth` stores `Limbs` directly; `scaled_biguint_parts`, its converter, and `before/src/accumulator.rs` deleted. |
| `417af8cf2` | before: State why retained values skip conversion when wide. Both guards kept, as `stored_bits() > 64`. |
| `52fb9baa3` | before: Compare far-apart query heights through `Ord`. `HeightWidths` and the loop removed. |
| `f55b7c655` | before: Measure accumulator widths in bits: 256-bit freeze allowance, bit weights, wasm32 pins, `digit_len` gone. |
| `1b0cf41bc` | suanpan: Ask the stability query once at the close comparison lead. |
| `2ace92f51` | suanpan: Add `abs_diff`, which compares and subtracts in one pass. |
| `185c82227` | before: Lower wide boundaries through `abs_diff`. The boundary is the receiver, so a tie needs no negation. |
| `e0956b25d` | suanpan: Keep the representation's base out of the public API. `stored_digit_count` is crate-private; the scaled readout's bit shift has no granularity promise, and its limbs are packed from the lowest written digit; `SparseWidth` places limbs at any bit offset. |

## Review findings and dispositions
**Correctness review**

| Finding | Disposition |
|---|---|
| F1–F3 (`bits` cost claims) | Dissolved by removing `bits` (`708991d49`). |
| Heap-account corrections | Confirmed: my capture of the revised rework is byte-identical to the reviewer's fast-path counterfactual. |
| Interop fast paths | Fixed (`708991d49`); all movement down. |
| `Debug {:x?}` | Fixed by another route. `Formatter::debug_lower_hex` is private and `Formatter::options` is unstable on 1.97, so `Debug` copies the stored form under the borrow and formats the copy through the caller's formatter. All flags work; tested `x?`, `X?`, `#?`. |
| `value.rs` doc | Fixed. |
| `shift_left` versus module doc | Code fixed (`ce3d63a92`); the old code fails the strengthened property. |
| Surface leg | The 3 extra excepted items are `before`'s own `testing::meter` `stored_nonzero_deltas`. `suanpan` is outside the leg, and the module exception is right. |
| `tick.rs` pre-reads | Fixed (`153095c68`). |

**Coverage review**

| Finding | Disposition |
|---|---|
| F1, borrowed-left operators | Fixed (`114429402`). |
| F2, F3 | Dissolved with `bits`. |
| F4, `From<&>` copy | `copies_carry_no_cancelled_width`. |
| F5, threshold, termination, convergence | Threshold pin (revised in `1b0cf41bc`), termination test, convergence property calls `cmp`. |
| F6, `BigUint` conversion | `conversions_to_bigints_are_exact`. |
| F7, test docs | Fixed. `zero_drift_never_freezes` now asserts its claim. |
| Interop oracle reading a copy | Fixed. |

**Structure review**
- Candidates 1, 2, 4, and 6: folded into the rework.
- Candidate 3: `153095c68`.
- Candidate 5: `5566af172`.
- `ScaledWidth`: kept as a type, because deferred entries store one.
- Not done: moving `interop.rs` to the crate root and the `regions.rs` binding. Both are cosmetic.

## API added
- **Ruling 2, all twelve primitive types:**
  - `AddAssign`/`SubAssign<&p>`.
  - `Add`/`Sub<&p>` on owned and borrowed accumulators.
  - `p ± acc` and `&p ± acc`, over owned and borrowed accumulators (`p − a` costs a negation, documented).
  - `Sum<&p>`.
  - `Shl`/`ShlAssign<&p>`.
  - The whole workspace, `rumors` crates included, compiles unchanged.
- **`suanpan::Limbs`** (`Deref<Target = [u64]>`, `AsRef<[u64]>`).
- **`Accumulator::abs_diff(self, other) -> (Ordering, Accumulator)`.**
- **Removed or narrowed:** `bits`, `normalize`, and public `stored_digit_count`.

## Board movements (capture/compare per commit; every measured state 5,361 green and pin clean)
| Commit | Movement and mechanism |
|---|---|
| Rework | 356 heap rows down, none up: the interop fast path. Byte-identical to the reviewer's counterfactual. |
| `normalize` removal, shift fix, sign-magnitude, coverage tests, ruling 2 | Unchanged. |
| `153095c68` | 434 rows, all down: −1 touch per dropped pre-read; `ranked_cmp` reads a sign instead of materializing the integral; rank-family heap drops, which I infer is because the integrator's other accumulators are freed before conversion. |
| `278565a8b` | 19 rows down, none up: no `Vec<u32>` temporary, inline below 2^128. |
| `e8df0474a` | 12 rows down, none up. |
| `417af8cf2`, `f55b7c655`, `1b0cf41bc`, `2ace92f51` | Unchanged. |
| `52fb9baa3` | 4 rows up by a constant +2 or +4 touches on `dense-suffix` queries. Inferred mechanism: a bound whose heights start far apart and compact to close pays `Ord`'s temporary, then rebuilds its kept difference on the next read. |
| `185c82227` | Unchanged; seam meters at 20,525 and 29,387. |
| `e0956b25d` | Unchanged against `lowerwide2.cap`; the 81 rows the limb-count version raised came back. |

**Measured rulings that hit limits, all resolved by later rulings:**
- **Guard removal:** `seam_plunge` 20,525 → 25,645 and `seam_stop` 29,387 → 36,562 (+5 touches per hop); `tick.rs` `mirror-wide` +0.89 touches per input byte. Guards kept.
- **`lower_wide` via `Ord`:** +13 touches per hop. Replaced with `abs_diff`.
- **`abs_diff` with `decrease` as receiver:** 15 rows moved; switching the receiver to the boundary gives exact parity.

## Pins and mutation evidence
- **Exact meter pins added, each derived before pinning:**
  - Comparison at leads 3, 2, and 1: 1, 1, 3n + 22 when the top re-carries, and 3n + 5.
  - Far `abs_diff`: 2n + 2.
  - `abs_diff` negation: 3n + 8 for nonnegative operands, independent of the receiver's former width; p + 2n + 6 when the wider operand is negative. The doc says O(A), or up to O(W), the result's own width.
- **Mutation, completed before the stop:**
  - Env-switched schema: the control passed, and `add_ref_prim_sub`, `sub_ref_prim_add`, `add_ref_owned_wrong`, `sub_ref_owned_no_neg`, `add_ref_ref_rhs_twice`, and `sub_ref_ref_rhs_twice` each failed 1 test (`mutants1.log`).
  - `min_ticks` freeze forced on fails `zero_drift_never_freezes`: `left: (0, 12), right: (0, 8)`.
  - The old `shift_left` fails the zero-shift property: "a digit-stored zero shifted by 33554432 bits stays in digits".
  - The tree was restored and `git diff` was empty each time.

## Landing check at `e0956b25d`, verbatim
The first run, at `ad5ad2046`, failed only in lints:
```
crates/suanpan/src/accumulator/tests/primitives.rs:146: doc summary renders to 290 chars (max 220)
audit-check: FAILED after 339s: lints
```
I split that summary (folded into `114429402`), ran the remaining lint tools locally (all clean), and re-ran once:
```
audit-check: ok      docs (39s)
audit-check: ok      surface (50s)
audit-check: ok      lints (56s)
audit-check: ok      tests (175s)
audit-check: ok      board (207s)
audit-check: ok      wasm (291s)
audit-check: clean in 291s; logs in target/audit-check-logs
exit 0
```
Counts from the leg logs:
- Tests: `823 tests run: 823 passed, 1 skipped` (`before` + `suanpan`); snapshots `142 tests run: 142 passed (1 slow), 594 skipped`.
- Doctests: `193 passed`, `15 passed`.
- Surface: `214 public function-like items = 134 board-covered + 80 excepted`.
- Board: `amp-board: 5361 green / 0 red`; `worst-case pin: clean (272 pinned rows verified at 2 sampling scales)`.
- wasm: 12 / 25 / 43 passed.

## Deviations
- **`Debug` hex:** done by copying and formatting through the caller's formatter, because the suggested API is private or unstable.
- **`abs_diff`:** no far-case stability query (owner accepted).
- **`Limbs`:** got its own commit rather than folding into the `ScaledWidth` commit, because it is a `suanpan` API change.
- **Mutation evidence:** see the verdict.
- **Process lapses:** two unquoted heredocs (one repaired from HEAD; the other wrote nothing), and one informational remote run piped into `grep`, which decided nothing.

Notes and logs: `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/builder-122-fixes/` (`NOTES.md`, `landing2.log`, `cap-*.log`). Board captures are on the box in `~/src/rumors-slot-51/target/b122f/`.
