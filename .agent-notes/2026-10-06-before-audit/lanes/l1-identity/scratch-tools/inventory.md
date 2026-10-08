# Instrument inventory: `explore/l1-identity`

Branch head at report time: `26cbae22` (base `58285ca5`). Paths are relative
to `crates/before/`. "Committed also caught" counts my mutants that the
committed suite (`--lib --test forks_count --test stale_state`, plus `--test
meter` where noted) also kills, from `calib-run-{2,3,6,8,9}.log`.

## 1. Interval-set party model: `tests/audit_l1/model.rs` (`03213585`)

- **Checks:** nothing by itself; it is the oracle. A party is a sorted list of
  disjoint dyadic intervals (`u128` numerators over `2^126`). It has its own
  canonical encoder and decoder (the decoder asserts that production bytes
  are canonical), set algebra by interval arithmetic, `fork` by a hull
  characterization (descend to the smallest dyadic node containing the set,
  then split by halves), balanced `shares(n)`, region listing, and pointwise
  multiplicity comparison.
- **Independence:** it shares no code or tree recursion with production or
  `testing::oracles::tree`; parties enter production only through the public
  `Party::decode`.
- **Reach:** any finite union of dyadic intervals to depth 126.

## 2. Generators: `tests/audit_l1/gen.rs` (`03213585`, depth parameter `f3456ae7`)

- `arb_set` / `arb_set_upto(max)`: random ownership of the pieces between
  random dyadic cut points, mixed with cut points clustered at many depths
  around one point. `arb_disjoint(k)`: `k` pairwise-disjoint nonempty parties
  as colorings of the pieces.
- **Reach vs committed (measured, observations O8):** depth beyond 32 in 48%
  (committed: at most 4 arbitrary, 8 organic); up to 128 bytes (committed:
  4); 32 or more two-child branches in 42% (committed: at most 4); unary
  chains to 128 (committed: 4); disjoint families interleaved at every depth
  (committed `join_all` families accepted 5.5% of the time; the committed
  disjoint-pair population is only fork halves of one party).

## 3. Pairwise and n-ary properties: `tests/audit_l1/main.rs` (`03213585`)

- **Checks against the model:** `is_disjoint`, `covers`, `without`, `join`
  (both arms, operand handed back unchanged), `shape`, and `is_seed` on
  arbitrary, disjoint, and nested pairs; `fork`; `Clock::sync` (`Ok` exactly
  on disjoint pairs, result equals join-then-fork, both clocks byte-identical
  on `Err`); `forks(k)` with every prefix length, exact `size_hint` at each
  step, and the residual; the array split for `N` in 1 to 9, 13, 16, and 17
  (Party and Clock); `Clock::forks`; `join_all` success on shuffled disjoint
  families; `join_all` failure with **pointwise multiplicity conservation**
  and a receiver that never shrinks; `Clock::join_all` with regions
  (multiplicity) and versions (join) conserved; `sync_all` dealing the model's
  shares in participant order with the joined version, and byte-identical
  clocks on `Err`.
- **Calibration:** caught M01-M03, M05-M08, M10-M13, M15-M18 (15 of 15, first
  batch), M19-M21, and M24. Committed also caught: 15 of 15 first batch,
  M21, and M24; **not M19 or M20**, which only the
  multiplicity properties kill (MB1).

## 4. Rule-respecting histories: `tests/audit_l1/history.rs` (`03213585`)

- **Checks:** after every step of a one-seed history, every live party equals
  the model, the live parties are pairwise disjoint (both by model and by
  production `is_disjoint`), their union is the whole interval, and no
  `join`, `join_all`, `sync`, or `sync_all` fails.
- **Reach beyond `testing::optrace`:** operations the committed trace
  vocabulary lacks: `forks(k).take(t)` with partial drops, consuming array
  splits into 2, 3, 5, or 8, `join_all`, `sync_all`, and moves through bytes
  (`Clock` and `Party` encode/decode). Populations: 63% of histories end with
  more than 8 live clocks (cap 24); committed organic traces exceed 8 in 0.6%.
- **Calibration:** caught M01, M02, M03, M05, M06, M10, M15, M16, M17, and
  M24; the committed suite also caught all of them.

## 5. Deep probe: `tests/audit_l1/deep.rs` (`1b1c16e7`, both spines `f3456ae7`)

- **Checks:** every identity operation completes at depth 100,000 on a
  256 KiB thread stack: predicates, `without`, `join` collapsing 100,000
  levels, `sync`, `forks` (full drain, partial drop, count above `u128::MAX`),
  array split, `join_all`, `Clock::forks` with `sync_all`, both shape walks,
  hash, `Debug`, and the codec, on right and left cells, combs, and a zigzag.
- **Reach:** the fork iterators' walks (`SharePath`, `select_path`,
  `Removal`) at depth, which no committed test reaches.
- **Calibration:** M22 (recursive `skip`): also caught by the committed suite.
  M26 (recursive removal descent): **only this probe** (MB2). M25
  (tail-recursive region descent): nothing at `opt-level = 2`; at
  `opt-level = 0`, both this probe and the committed
  `deep_tree_query_and_causal_stack_safety` catch it (MB3).

## 6. Overlay refinement: `tests/audit_l1/overlay.rs` (`1b1c16e7`)

- **Checks:** `Clock::shape` equals the coarsest common refinement of the
  model's party regions and the version's plateaus, with each plateau's rise
  on its first cell only. The version side uses production `Version::shape`
  as its reference.
- **Reach:** versions built by ticking deep arbitrary parties (to depth 80),
  against the committed diff table's depth-4 populations.
- **Calibration:** M23 caught; the committed diff table also catches it
  (`clock_solo_ops`, `calib-run-9.log`).

## 7. Wide counts: `tests/audit_l1/wide.rs` (`1b1c16e7`)

- **Checks:** `forks(k)` for `k` around `usize::MAX`, around twice it, at
  arbitrary widths to `2^100`, and at powers of two: the first 40 shares
  equal the model's (one logarithmic descent per share), the residual equals
  the original minus the taken shares and covers share 0, and every hint is
  sound (and exact while the count fits `usize`). The same for
  `Clock::forks`.
- **Reach:** exact share identity at counts beyond `u64`; the committed
  `wide_count_prefixes_conserve_the_party` checks conservation for at most 4
  shares.
- **Calibration:** M04 caught; the committed
  `adjacent_wide_count_becomes_exact` also catches it (`calib-run-9.log`).

## 8. Heap-retention probe: `tests/audit_l1_retention.rs` (`8074f098`)

- **Checks (prints, no assertions):** live heap held by a result after its
  inputs are dropped, using a counting global allocator, at four depths.
- **Reach:** retained heap of `join`, `without`, `Clock::join`, the
  `forks(1)` residual, `join_all`, and parties from `Clock::decode`; the only
  committed retention check is the `fork` capacity test.
- **Calibration:** none; it is a measurement, with `fork` as its control
  (retains 1 B at every size).

## 9. Cost probe: `tests/audit_l1_cost.rs` (`26cbae22`)

- **Checks (prints, no assertions):** scan bits per documented-bound unit at
  `d` = 1k, 4k, 16k, 64k for `without`, `covers`, `is_disjoint`, `sync` (two
  pessimal pairs), `forks(64)` full drain on a deep unary party, `join_all` of
  its shares, one `next` of a `2^d` count, and `shape`.
- **Reach:** `without` with a deep arbitrary `self` (the board meters only
  `seed.without(b)`); fork steps at counts to `2^64000`.
- **Calibration:** none; readings are flat (`cost-1.log`).

## 10. Size-hint probe: `src/party/forks/tests.rs::explore_distant_plan_is_exact_once_the_remainder_fits` (`1a5f5f8b`)

- Fails on base by design (D1's reproduction).

## 11. Generator statistics

- `tests/audit_l1/main.rs::generator_stats` (`03213585`) and
  `src/party/tests.rs::explore_committed_generator_stats` (`c54c195c`):
  print the histograms behind O8. No assertions.

## Scratch-only tooling (not on the branch)

`calibrate.py` (apply an exact string swap, run a remote nextest selection,
revert, check `git diff`) and the mutant lists `muts1.py`-`muts4.py` in the
auditor scratch directory.
