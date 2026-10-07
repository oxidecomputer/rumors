# auditor-l4 resumption record (lane L4: measures)

Worktree `/Users/oxide/src/rumors-audit-l4-measures`, branch
`explore/l4-measures`, base `58285ca5`. Box mirror:
`~/src/rumors-audit-l4-measures` (logs under its `target/l4logs/`, copied test
binaries under `target/l4bin/`; never write elsewhere on the box). Scratch:
this directory. Execute only through
`on-illumos.sh <wt> 'unset CARGO_TARGET_DIR; ...'`; nextest's `-j` is the
test-thread count, so builds take `--build-jobs 24`. The wrapper runs under
`set -e`, so capture loop statuses with `cmd && rc=0 || rc=$?`. illumos grep
has no `-m`; scp logs back and filter locally.

## Explore-branch commits (all signed)

- `da737ac5` flat-oracle harness `crates/before/src/version/measure/explore_l4.rs`
  (+ `DEFER_HITS` test tap in `integral.rs`, `mod explore_l4` in `measure.rs`)
- `223ef294`, `a836c8b4` rational-oracle `Rank` harness `explore_l4_rank.rs`
- `c4ffb39d` cost search (`l4_cost_search`), `Count` boundary sweep
- `2d3a77a3` tie (mirror, same-depth rotation), causal-order, suffix, and
  deep-prefix coverage
- `a54a8b0f` `Ranked::cmp` tie-cost probe

## Established (verified on the box unless marked)

1. Integrator values: `rank`, `distance`, `lag`, `rank_cmp`, `Rank::cmp`,
   `Ranked::cmp`, `Ranked::encode` order (with suffixes), `encode_rank`,
   valuation law, causal-order strictness, distance-zero-iff-equal all agree
   with an independent flat sweep oracle. Generator: dyadic partition from
   random splits (+ optional 32..120-deep spine prefix) and multi-scale height
   script (k bands up to 2200 bits). Coverage (2000 cases): 86% freeze, 51%
   defer, 26% >=3 deferrals, 29% exact distinct ties (25% frozen ties).
   Runs: 40,000 cases pass (run1, pre-strengthening); 30,000 strengthened
   cases pass (run3, 449 s). Deep regime (L4_DEEP_MAX=3000, L4_K_MAX=6000):
   production touches/B 0.3-3.6 and dense/B <= 2 flat to 2.4 MB inputs
   (`cost-2.log`); nextest timed out on 1500 deep cases from input volume in
   the debug oracle, so run4 runs 600 deep cases outside nextest.
2. Calibration: 18 env-gated integrator mutants (patch saved at
   `mutants-integral.patch`; reapply with `git apply`, reverse with
   `git apply -R`; verify `git diff` empty on integral paths). Harness catches
   all real defects M2 M3 M4 M5 M6 M9 M15 M23 M25 M29 M30 M31 (M25 only after
   the deep prefix). Schedule-only probes E13 always-defer, E14 always-freeze,
   E15 never-freeze pass (values schedule-independent). The committed suite
   ALSO catches all 12 narrow mutants (sweep2, `target/l4logs/sweep2-*`), so
   the harness has no demonstrated unique detection: no machinery brief.
3. `Rank` values: rational oracle (cross-multiplication) for Ord/Eq/Hash,
   Add/AddAssign/checked_sub/saturating_sub, Sum in any order, encode order
   and suffix order, decode round trip, Display/Debug against an independent
   renderer, formatting width/precision/fill/align against `str`. 20,000 cases
   pass (run2). Rank mutants R1 R3-R7 caught; R2 (same-class numerator
   compare) caught after adding same-class independent-mantissa mode; the
   committed suite catches R2 in 26 tests. E8 (sum shift without doubling)
   value-preserving.
4. "One value, one behavior": version rank equals Sum of per-leaf areas in any
   order and equals distance-to-empty, lag-from-empty, decode(encode),
   parse(display), ranked().rank(); all agree on Eq/Hash/Ord/encode/text.
5. Count: boundaries MAX-1..MAX+1 of every width, limbs, text, conversions:
   pass. num-bigint 0.4.8 uses Burnikel-Ziegler division and D&C radix
   conversion, so the "subquadratic" Display claim holds (read in source).
6. Closed fixes re-attacked: Sum order cost flat (0.22-0.53 touches/content
   byte, W=2^12..2^18, four orders: `sumcost-1.log`); Ranked::cmp tie cost
   on mirrored arming trains and plateau punctures tracks two rank folds
   (x1.07, x1.05, x1.02 per doubling, plateau flat: `tiecost-1.log`); Display
   flags verified by harness; integrator cost search flat to 150 KB
   (`cost-1.log`).

## Findings so far (no confirmed contract defect in lane)

- Doc defect (low, maintainer prose): `rank.rs:10-11` and `rank.rs:130-131`
  claim the numerator is odd unless zero; even integers keep even numerators
  at exp 0. Brief: `briefs/simplification-rank-normalization-prose.md`.
- Prose inaccuracy (observation): `rank.rs:1035-1043` says version-derived
  exponents are "under 2^32"; `encoded_bits` is u64 and 32-bit storage allows
  about 2^34 bits. Conclusion (below 2^37) holds.
- Cross-lane (L6), inferred not demonstrated: `decode_stream` buffers fraction
  groups in `Vec<u8>`; on wasm32 >2^30 groups (~1.13 GiB stream) hits
  capacity-overflow panic. Record: `cross-lane-l6-rank-decode-wasm32.md`.

## Background jobs

None running. Completed: run4 deep 600 cases pass; run5 strengthened Rank
20,000 pass; wasm-2/3/4 wasm32 prototype runs (logs here).

## wasm32 work (explore commit `2c82c7ba`)

- Prototype `RankArithmetic` cases 5-12 + `run_diagnosed` (panic message and
  memory pages after a trap) + test `l4_rank_sum_cases_diagnosed` (select a
  case with `L4_CASE`). Build: in `crates/before/wasm32-pins`, guest with
  `--target wasm32-unknown-unknown --target-dir $R/target/wasm32-pins`,
  harness `--tests --release`, run with `WASM32_PINS_GUEST_WASM=...`.
- Results: small-first Sum passes (cases 7, 8); deep-first Sum aborts on
  allocation (cases 5, 6, 9; empty panic message; replay 11 aborts, 12 with
  exact reserve passes). Narrowing mutant `shift as usize as u64` in
  `sum_iter` caught by case 8. Briefs: `briefs/machinery-wasm32-rank-sum-pin.md`,
  `briefs/machinery-wasm32-trap-diagnosis.md`; observation O6, O7.

## Next

1. Write the report (verdict: no confirmed contract defect in lane; one
   low-severity prose defect brief; two machinery briefs; observations;
   cross-lane L6 lead; question on O6). Judge the lane at diminishing returns.
