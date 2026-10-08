# Lane L1 coverage record

Revision: `explore/l1-identity` at `26cbae22` over base `58285ca5`; production
code identical to base throughout (every calibration run ends with an empty
`git diff`).

## Established, and how

Suite commands (on ox-east-1):

```
on-illumos.sh /Users/oxide/src/rumors-audit-l1-identity 'unset CARGO_TARGET_DIR; nice -n 10 cargo nextest run -p before --all-features --locked --build-jobs 24 -j 24 --test audit_l1 --no-fail-fast'
# long runs: prefix L1_CASES=<n> and add --profile high-count
```

| Contract clause or lead | How established | Volume |
|---|---|---|
| Lead 1: conservation at every prefix of a fork drain; exact hints | `forks_partial_drain` (k to 300, every prefix), `wide::forks_wide_counts` (to `2^100`), `clock_forks_*`, histories | 100k cases each (`long-1.log`); wide at 10k (`long-2.log`) |
| Lead 1 dispute | Exactness is promised only for initial counts fitting `usize`; the stronger lead fails for distant counts, now D1 | `d1-repro.log` |
| Lead 2: no false errors under rule-respecting histories | `rule_respecting_histories`, every step checked, with forks drains, splits, `join_all`, `sync_all`, byte moves | 100k histories of up to 40 ops |
| Lead 2 converse: disjoint implies `join` succeeds, for arbitrary canonical parties | `set_algebra_*` (join `Ok` exactly on model-disjoint pairs), `sync_*` | 100k each, arbitrary, disjoint, and nested pairs |
| Lead 3: region conservation on `join_all` failure | multiplicity (stronger than the lead's union equality) for Party and Clock | 100k each |
| Lead 4: `without`, `covers`, `is_disjoint` as set algebra on arbitrary canonical parties | `set_algebra_*` against interval arithmetic | 100k each, depth to 120 |
| Lead 5: `Regions` and `Overlay` partition `[0, 1)` and match membership; iterative at depth | `shape` in `check_pair`; `overlay_is_the_refinement`; deep probe on both spines | 100k each; depth 100,000 |
| Crate page: a `join`/`sync` error is definitive | Errors occur exactly on model overlap (`set_algebra_*`, `sync_*`, `sync_all_matches_model`); never in histories | as above |
| `sync`/`sync_all` leave clocks unmodified on `Err` | byte-identical encodings before and after | as above |
| Closed fix: 32-bit and maximum count | wasm32 pin reviewed; width-generic code read line by line; wide-count property at the 64-bit analogs; D1 found in the classification | `usize` sweep below |
| Closed fix: per-step cost linear in stored values and count width | read (construction `O(log k)`, step `O(|p| + log k)`, drop `O(1)`); scan meter on `forks(2^d).next()` for `d` = 1k, 4k, 16k, 64k: exactly `about 4,000 + 6d` bits, linear in `log k` | `cost-1.log` |
| Closed fix: full drains and `sync_all` aggregate bounds | `forks(64)` full drain on a deep unary party: 4.92 scan bits per `k(|p| + log k)` bit at all four sizes; `join_all` of the 65 shares: 0.82 per `(|self| + |iter|) log k` bit; `sync_all` read (O7) and board-metered | `cost-1.log` |
| Linear walks on pessimal pairs | scan per input bit, flat across `d` = 1k to 64k: `without` comb vs zigzag both ways 1.25; `covers` 0.50; `sync` comb vs its cell 3.00 and cell vs zigzag remainder 3.00; `shape` drain 1.00 | `cost-1.log` |
| No recursion on depth | deep probe at 256 KiB stack, at `opt-level` 2 and 0; all committed deep tests also pass at `opt-level = 0` | `run-opt0-deep.log` |
| Retained memory | allocator probe at four sizes | `retention-*.log` |

## `usize` invariance sweep (whole lane)

Every `usize` in `src/party/`, `src/clock.rs`, `src/clock/forks.rs`,
`src/fold.rs`, and `src/shape.rs` was classified:

- **Defect (D1):** `Remaining` and `has_saturated_lower_bound` in
  `src/party/forks.rs` classify the initial count against `usize::MAX`.
- **Nit (O5):** `fold.rs`'s `weight` level counter.
- **Legitimate:** `into_shares(count)` and `Shares` (shares held in memory);
  `[Party; N]`, `[Clock; N]`, `Cell<N>`, `combine<N>` (array lengths);
  `decode_prefix`'s byte count and `node_at`'s index (memory offsets);
  `TAG_BITS` (a bit count reserved in memory); `size_hint` signatures (fixed
  by `Iterator`); `sync_all`'s `others.len() + 1` (participants held).
- `Count`'s `From<usize>` and `TryFrom<&Count> for usize` are value-preserving
  conversions in another module; not examined beyond that.

## Blind spots

- **32-bit execution.** Nothing in this lane ran on a 32-bit target beyond
  the committed wasm32 pin (count `2^32`). The suite needs proptest, which the
  `wasm32-pins` guest does not host, and the box has no other 32-bit target.
  The divergent region of D1 lies about `2^33` steps away on wasm32, so it is
  demonstrated at the 64-bit analog.
- **Complexity.** My scan-meter probe (`tests/audit_l1_cost.rs`) covers the
  families above at four sizes, with no growth; it reads stream bits only, so
  `BigUint` work in the fork plan is invisible to it. The committed board
  meters `party_without` only as `seed.without(b)`, so `without` with an
  arbitrary `self` has no committed metered family; my comb-against-zigzag
  reading is flat at 1.25.
- **Equivalent tail-recursion.** At the workspace's `opt-level = 2`, no test
  can see a tail-recursive walk (MB3).
- **Version-side retention** (O9) and **projection** (`OwnVersion`, the
  algebra lane's): not examined.
- The overlay property trusts production `Version::shape` for the version
  side.

## How to resume

Read NOTES.md, then run the suite command above. Calibration:
`python3 <scratch>/calibrate.py <scratch>/mutsN.py '<nextest args>' [names...]`.
Next lines of work if asked to continue: a committed board family for
`without` with an arbitrary `self`; version-side retention (with the algebra lane); folding the
multiplicity check into the law registry once MB1 lands.
