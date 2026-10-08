# L4 measures: coverage record

Base `58285ca5`; explore branch `explore/l4-measures` at `2c82c7ba` (all
commits signed). Every run below executed on ox-east-1 through
`on-illumos.sh /Users/oxide/src/rumors-audit-l4-measures 'unset CARGO_TARGET_DIR; ...'`
in the debug test profile (debug assertions on) unless stated.

## Instruments built on the explore branch

- `crates/before/src/version/measure/explore_l4.rs`
  - **Generator.** A version is a dyadic partition built by repeated leaf
    splits (random index, first, last, or penultimate leaf, with an optional
    spine of 32..`L4_DEEP_MAX` same-side splits first), plus a height script
    of twelve step kinds over five bit-width bands up to `L4_K_MAX` (default
    2200): small moves, `±2^k`, `2^k - 1 + s`, digit-wide runs of ones,
    plateaus, returns to earlier heights, near-cancellations, XOR flips.
    Second operands are independent, perturbed, refined (leaves split with
    narrow perturbations), single-leaf edits, mirror images (exact rank tie),
    or same-depth height rotations (exact rank tie).
  - **Oracle.** The flat sweep: `rank = Σ h·2^(S-d) / 2^S`, and pair measures
    by merging both partitions at the common scale and integrating `|a-b|`,
    `max(0, b-a)`, and `a-b` with plain `BigInt`. Results compared by
    cross-multiplication plus a separate normal-form check. Shares no code
    with the tree oracle or the integrator; cross-checked against the tree
    oracle on 500 shallow cases (`l4_flat_oracle_agrees_with_tree_oracle`).
  - **Checks per pair** (`check_pair`): `rank` both sides; `distance` both
    orders; `lag` both orders; `rank_cmp` both orders; `Rank::cmp`;
    `Ranked::cmp` (rank then bytes); `Ranked::encode` order with and without
    appended suffixes; `encode_rank` equals `rank().encode()` and keeps order
    under suffixes; causal order implies strict rank and view order and the
    one-sided lag; distance is zero exactly on equal versions; the valuation
    law `rank(a|b) + rank(a&b) == rank(a) + rank(b)` on production join/meet.
  - **Taps.** `FREEZE_HITS` (existing) and `DEFER_HITS` (added, test-only).
  - **Probes.** `l4_cost_search` (worst touches and densified digits per
    byte, binned by input size), `l4_ranked_cmp_tie_cost`.
- `crates/before/src/version/measure/explore_l4_rank.rs`
  - **Rank values** against a raw `(num, exp)` rational oracle: generator
    covers zero, small values, `2^k`, `2^k ± 1` at word boundaries (31-33,
    63-65, 127-129, 191-192, 255-256), random 1..80-byte numerators, even
    numerators, shifted runs of ones; exponents 0, small, word boundaries,
    up to 5000. Related pairs: unnormalized respelling, low-bit flip, deep
    tiny offset, same class with different word count, same class with
    independent mantissa.
  - Checks Ord/Eq/Hash, `+`, `+=`, `checked_sub`, `saturating_sub`, `Sum`
    over shuffled multisets (both impls), encode order and suffix order,
    decode round trip, `encode_to`, Display/Debug against an independent
    renderer, and width/precision/fill/alignment against `str` formatting
    (including a multibyte fill).
  - Version rank equals the shuffled `Sum` of per-leaf areas (each summand
    built by parsing independently rendered text), and agrees with
    `distance(v, empty)`, `lag(empty, v)`, `decode(encode)`,
    `parse(display)`, `ranked().rank()` on Eq, Hash, Ord, encode, and text.
  - `l4_count_boundaries`: every primitive's `MAX-1..=MAX+1`, limbs,
    exact size, decimal round trip, every `TryFrom`, `From<u128>`,
    `From<usize>`, subtraction at the neighbors, rejected text forms.
  - `l4_rank_sum_order_cost`: touches per content byte for four summand
    orders at four sizes.

## Contract clauses and what establishes them

| Clause (source) | Established by | Distribution | Residual |
|---|---|---|---|
| `rank` is the exact area (`Rank` type docs) | flat oracle, 40,000 + 30,000 + 600 deep cases (depth to 3000, heights to 6000 bits); committed tree-oracle and Riemann-sum suites | multi-scale partitions; 86% freeze, 51% defer, 26% >=3 deferrals | heights above 6000 bits and depth above 3000 only via committed named shapes |
| valuation law / modularity (lead 1) | `check_pair` on production join/meet | same | none observed |
| `distance` symmetric, zero only on equal, triangle (Version docs) | exactness against the L1-norm oracle implies all three; zero-iff-equal asserted directly | same | triangle not asserted separately (implied by exactness) |
| `lag(a,b)+lag(b,a)==distance` | exactness of both against the oracle | same | none |
| strict monotonicity (`v<w ⇒ rank(v)<rank(w)`) | `check_pair` causal branch | refinement and single-leaf modes produce comparable pairs | none |
| `Ranked` order = rank then bytes; encode order = Ord, ties included; suffix safety | `check_pair` | 29% exact distinct ties, 25% through freezes | none |
| one value, one behavior (lead 2) | `l4_version_rank_is_the_sum_of_leaf_areas`, `rank_is` normal-form check on every produced rank | seven routes | serde/borsh routes read by code only (they reuse `decode_stream`, `decode_bytes`, `FromStr`) |
| encode order and suffix property (lead 3) | `l4_rank_values_agree`, 20,000 + 20,000 cases (second run with the same-class mode); order argument re-derived by hand | word-boundary values, same-class pairs | decode error paths left to L6 |
| text canonical and round trips; formatter flags (closed fix) | `l4_rank_values_agree` | as above | none |
| `Count` boundaries (lead 4) | `l4_count_boundaries` exhaustive | all widths | none |
| `Sum` linear in content, any order (closed fix) | `l4_rank_sum_order_cost`: 0.22-0.53 touches/content byte, W = 2^12..2^18, four orders | constructed adversaries | touch meter does not see `BigUint` shifts in `from_raw` (linear by inspection) |
| `Ranked::cmp` worst case on settling families (closed fix) | `l4_ranked_cmp_tie_cost`: mirrored arming trains n = 4..32 grow x1.07, x1.05, x1.02 per doubling, tracking two rank folds; mirrored plateau punctures flat | committed settling shapes, mirrored | none |
| folds at `O(M(n))` (closed fix) | `l4_cost_search`: touches/B 0.3-3.6, dense/B <= 2, flat from 15 B to 2.4 MB, default and deep regimes | random shapes | random shapes do not reach the designed adversaries; those are the committed meters' job. `BigUint` products are invisible to touches; only wasm fuel sees them, and I did not use fuel |
| 32-bit arithmetic boundary | committed `wasm32-pins` `RankArithmetic` cases 1-4 rerun on the box (pass); prototype `Sum` cases across the 2^32 gap: small-first orders pass, deep-first orders abort on allocation (O6); narrowing mutant caught by the one-first case | exponent 2^32 | decode capacity limit: see the L6 cross-lane record |

## Calibration (harness can fail)

- Integrator mutants, env-selected, one build (patch:
  `mutants-integral.patch`, applied and reverse-applied with `git apply`,
  restoration verified by an empty `git diff` on the integral paths):
  M2 park-before-settle, M3/M4/M31 missing or misplaced deferred widths, M5
  wrong merge side, M6/M29 jump shift dropped, M9 orientation not reversed,
  M15 single deferral skipped, M23 deferred height sign lost, M25 width scale
  dropped, M30 negative opening skipped. Harness catches all twelve (M25 only
  after the deep-spine prefix was added). Committed suite also catches all
  twelve (`sweep2-suite-*` logs on the box under `target/l4logs/`).
- Schedule-only probes (must preserve values): E13 always defer, E14 always
  freeze, E15 never freeze: all pass at 1024 cases. The integrator's values
  are independent of its freeze and deferral schedule.
- Rank mutants: R1 normalization off by one, R3 center padding swapped, R4
  last fraction bit dropped, R5 first summand exponent off, R6 precision off
  by one, R7 `checked_sub` underflow returns zero: caught. R2 (compare raw
  numerators within a magnitude class) survived the first generator, was
  caught after adding the same-class independent-mantissa mode; the
  committed suite catches R2 in 26 tests. E8 (sum shift without doubling)
  preserves values, as expected.

## How to resume

- Re-run the value harnesses:
  `cargo nextest run -p before --all-features --locked --build-jobs 24 -E "test(/explore_l4/)"`
  with `L4_CASES`, `L4_DEEP_MAX`, `L4_K_MAX` to scale; long runs from a
  copied binary under `target/l4bin/` to escape nextest's 180 s limit.
- Re-calibrate: `git apply <scratch>/mutants-integral.patch`, run with
  `L4_MUT=<name>`, then `git apply -R` and confirm `git diff` is empty.
- Cost probes print `COST`, `SUMCOST`, `TIECOST` lines; run each alone
  (`--nocapture`), since the meters are process-global.

## Blind spots that remain

- Wasm fuel was not used; `BigUint` multiplication cost in the settle is
  covered only by the committed fuelscape and board.
- 32-bit: only `Rank` arithmetic and `Sum` were run in the wasm32 guest;
  Display, comparison, and the integrator at 32-bit scale were examined by
  reading only.
- Decode error paths, serde and borsh transports: L6's lane.
- `min_ticks`: L3's lane.
EOF
echo ok