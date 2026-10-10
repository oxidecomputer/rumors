<!-- CAVEAT LECTOR: the fix builder's targeted-mutant report at e0956b25d, written by a Claude (Opus 5.5) agent and archived verbatim by the audit coordinator. Its claims are unverified unless the README says otherwise. -->

## Verdict: every targeted mutant fails its test, 12 of 12, and the tree is restored

Following the owner's "127: run them." (relayed verbatim by the coordinator), I ran one env-switched mutant schema in slot 51. It was a single build, never `cargo mutants`, and nothing was committed. Slot 51 was detached at `e0956b25d`.

The control passed:
- `suanpan`: `Summary [  10.396s] 95 tests run: 95 passed, 0 skipped`.
- `before` width tests: `Summary [   0.252s] 4 tests run: 4 passed, 725 skipped`.

After restoring, `git diff` is empty, `git status --short --untracked-files=all` prints nothing, and HEAD is `e0956b25d35e488439bbb47e60c227fa00e7968b`. A follow-up sync left the box's copy clean too: no untracked files, no mutant code. The branch tip in the owner's checkout was not touched.

## Results, verbatim
Each line gives the injected defect, then the failing test and the assertion from `mutants2.log`.

- **`from_ref_clone`** — `From<&Accumulator>` copies the stored form, as a `Clone` would. `95 tests run: 94 passed, 1 failed`. Failing test: `representation::copies_carry_no_cancelled_width`, at `representation.rs:512`: `left: 4` / `right: 1`.
- **`lead3`** — the comparison's close lead is treated as 3. `94 passed, 1 failed`. Failing test: `metered::comparison_queries_from_a_two_digit_lead`, at `metered.rs:929`: `left: 18` / `right: 1`.
- **`at_lead_skip`** — no query at exactly the lead. `94 passed, 1 failed`. Same test and line: `left: 18` / `right: 1`.
- **`lead1`** — the close lead is treated as 1, so the loop queries forever at a two-digit lead whose top re-carries. Both the threshold pin and `comparison_terminates_when_a_refusal_recarries` were killed by `timeout 120`: `Summary [ 119.158s] 1 test run: 0 passed, 1 failed` / `SIGTERM [ 119.154s] … (test aborted with signal 15: SIGTERM)` / `status 124`. The termination test: `Summary [ 119.176s]`, `status 124`.
- **`at_lead_loops`** — the at-the-lead query moved inside the loop. Both tests hit the timeout: `Summary [ 119.167s]` and `[ 119.124s]`, each `1 failed`, `status 124`. The unmutated control runs these tests in under 0.2 s.
- **`biguint_low_limb_only`** — the interop keeps only the low limb. `93 passed, 2 failed`. Failing tests: `interop::conversions_to_bigints_are_exact` (at `interop.rs:84`) and `interop::bigint_operands_match_the_oracle` (at `interop.rs:57`), both `left: 0` / `right: 18446744073709551616`.
- **`absdiff_no_reverse`** — the order is not reversed after a receiver swap. `93 passed, 2 failed`. Failing tests: `metered::far_abs_diff_reads_the_wider_top_and_the_narrower_operand`, at `metered.rs:968`: `left: Less` / `right: Greater`; and `surface::complete_surface_matches_bigint`, at `surface.rs:843` (the `abs_diff` order assertion).
- **`absdiff_no_negate`** — a negative difference is not negated. `93 passed, 2 failed`. Failing tests: `metered::abs_diff_negates_within_the_result_width`, at `metered.rs:1014`, "5 under a top cancelled at 1000 against 4 digits": `left: 16` / `right: 20`; and `surface::complete_surface_matches_bigint`.
- **`absdiff_no_swap`** — always reuses `self`'s buffer, which is the "which buffer" check. `94 passed, 1 failed`. Failing test: `metered::far_abs_diff_reads_the_wider_top_and_the_narrower_operand`, at `metered.rs:969`, "abs_diff of 5·2^(32·1000) and 4 digits, reversed true": `left: 2005` / `right: 10`.
- **`debug_string`** — `Debug` renders through `format!("{:?}")`, the old behavior. `94 passed, 1 failed`. Failing test: `standard::debug_honors_the_formatters_flags`, at `standard.rs:113`: `assertion failed: format!("{small:x?}").contains("Some(ab)")`.
- **`sparse_no_offset`** — `SparseWidth::from_scaled` ignores `shift % 32`. Width tests: `3 passed, 1 failed`. Failing test: `width::tests::scaled_widths_convert_at_any_bit_shift`, at `width/tests.rs:100`: `left: 1` / `right: 2`.
- **`sparse_no_spill`** — the bits shifted past 32 are dropped. `3 passed, 1 failed`. Same test: `left: 0` / `right: 4294967296`.

Together with the earlier evidence, every new test and pin on the branch now has a mutant it kills. The earlier runs: six borrowed-left operator mutants, the `min_ticks` freeze counter, and the old `shift_left`. No mutant survived, so I have no findings from this run.

## Branch state, unchanged from my previous report
The tip is `e0956b25d` on `main` `d1aa90e04`, and its landing check was clean:
```
audit-check: ok      docs (39s)
audit-check: ok      surface (50s)
audit-check: ok      lints (56s)
audit-check: ok      tests (175s)
audit-check: ok      board (207s)
audit-check: ok      wasm (291s)
audit-check: clean in 291s; logs in target/audit-check-logs
```

Files are in `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/builder-122-fixes/`:
- `mutants2.log` — full output of every run.
- `schema2.py` — the reversible mutant schema.
- `mut2-remote.sh` — the run script.
- `NOTES.md` — resumption notes.
