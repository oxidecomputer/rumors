# Inventory of `explore/l7-suanpan` (head `4b277b0b`, base `58285ca5`)

One entry per probe, model, generator, and prototype. "Mutants" means my reversible string-swap calibrations (`S/mutate.py`; logs in `S/mut/`). The committed-suite comparison is from the same mutants run against every suanpan test except mine (`S/calibrate_existing.out`, `S/calibrate_zn.out`), and from the adequacy lane's survivor list.

## suanpan, `crates/suanpan/src/accumulator/tests/explore_l7.rs`

**1. Pool model, `l7_pool_programs_match_the_oracle`** (commits `7d5fae48`, `df000229`, `c30c045f`).

- Checks: every public operation over a pool of three accumulators against `num_bigint` oracles. The pool covers primitive updates of every width, shifted limb streams, borrowed, shifted, owned-clone, and owned-take accumulator operands, `<<=`, unary `-`, `reset`, `normalize`, `cmp_zero`, `cmp_zero_stable_under` with literal widths and with another member's `stored_bits`, `reserve_digits`, conversions to every primitive, clone-replace, borrowed and owned `Sum`, extreme-digit parking, and gap-split rounds.
- After every step, non-mutating checks only: both readouts against the oracle, the representation invariants, known-zero soundness, the stored-width bound, and the exact maximum.
- Inline: normalize's exact width and its growth bound, stability soundness including the accumulator-operand clause and the "at most two digits wider on `None`" clause, and conversion rejection preserving the representation.
- Reach beyond committed generators:
  - Comparisons happen only where the program asks; the surface suite compacts after every step.
  - Operands carry arbitrary histories.
  - Extreme digits meet `normalize` and shifts.
  - Swarm-weighted operation mixes, values near the small-path thresholds, and large repeated shifts.
- Measured reach: it catches the adequacy lane's survivor `normalize.rs:74` (`*` to `/`, my N1), which the whole committed suite misses.
- Calibration:
  - Caught M1-M7 and M9 (sign threshold, stale ranges, range rebuild, negative readout, `i128::MIN`, both stability margins, collapse) and N1. M6 needed the threshold-biased values.
  - Missed N2 and N3: the normalize shrink loop needs a top digit of exactly 3 over extreme digits, which it never builds.
  - The committed suite catches M1-M7 and M9 too; N1 is the pool model's only unique catch.
- Runs: 20,000 cases passed at `df000229`; 2,000-case runs with each prototype.

**2. Touch property, `metered::l7_touches_are_linear_in_table_work`** (`7d5fae48`, `df000229`, `c100e028`, `c30c045f`). Briefed as MB1.

- Checks: total touches at most `8 * work + 64`, with work taken from the crate page's cost table.
- Reach: the gap-split rounds (write at the top, write inside the gap, cancel both), which no committed shape builds.
- Calibration: caught M12, M14, M15, M16, Z3, Z4, Z5. The committed suite catches M12 and M16 but misses M14 and M15 (mine) and Z3, Z4, Z5 (the adequacy lane's `compact_storage` survivors). Survivors by design: M13 (a constant) and Z1 and Z2 (unreachable `take_below` boundaries).
- Runs: 20,000 serial cases passed at `c100e028`.

**3. Adversarial touch search, `metered::l7_adversarial_touch_search`** (`#[ignore]`; `df000229`).

- Hill-climbs on touches over work by mutating programs, then rescales the best program's shifts by 2, 4, and 8.
- Found no width-dependent ratio across 12 seeds of 30,000 mutations; the best ratio, 9.6, comes from pricing `i128` updates at one unit.
- Not calibrated (a search, not a check).

**4. Replay, `metered::l7_replay_touch_repro1`** (`6c393dbb`): prints per-step touches for one shrunk program. Diagnostic only; it exposed my process-global counter mistake.

**5. Probes** (print readings; assert little):
- `l7_probe_zero_shift_history` (Q1; `df000229`)
- `l7_probe_reserve_digits_extreme` (D2)
- `l7_probe_repeated_comparison_cost` (O1/S2; `880aa203`)
- `l7_probe_stability_huge_width_compacts` (U1-a; asserts compaction on 64-bit; `4fd78232`)

## Prototypes (suanpan features; explore only)

**6. `l7-proto` (T, tags)**, `94f56560`. Tag maintenance across `digits.rs`, `sign.rs`, `normalize.rs`, with a full tag check added to `assert_invariants`. Passes all 70 suanpan tests and the pool model at 2,000 cases.

**7. `l7-proto-w` (W, written bitset)**, `31571595` and `c045c8ba`. `crates/suanpan/src/accumulator/digits/written.rs`, with positional invariant checks in tests. Passes all 70 suanpan tests and the pool model at 2,000 cases; every exact touch pin holds.

**8. `l7-o1` (S2, identity-collapse skip)**, `f7a8560f`. Passes all 70 suanpan tests and 84 of `before`'s touch-instrument tests.

## 32-bit, `crates/before/wasm32-pins/` (guest `src/checks.rs`, harness `tests/pins.rs`)

**9. Landing cases 4, 5, and 6** (`d43fa15e`; D1), harness test `zz_l7_suanpan_limb_index_past_usize_max`: a limb stream past `usize::MAX`, its in-range control, and the classified variant.

**10. Zero-shift cases 7 and 8** (`a959e13c`; Q1), test `zz_l7_suanpan_zero_shift_representations`.

**11. Stability-width case 9** (`4fd78232`; U1-a), test `zz_l7_suanpan_stability_width_compacts`.

## Fuel instrument (`crates/before/wasm32-pins/`)

**12. `run_fueled`** (harness library): a second wasmtime engine with fuel metering.

**13. Probe families and operations:**
- `guest/src/l7_builders.rs`: families F1, ±S, OS and their dense controls, built through `before::testing::meter::Encoding`.
- `guest/src/checks.rs` `l7_fuel`: suanpan-only workloads, decode, `<=`, `partial_cmp`, join, meet, tick (seed and half party), projection comparison, `min_ticks`.
- Harness test `zz_l7_fuel_matrix` (`#[ignore]`; env-selected).
- Commits `d43fa15e`, `555aaf86`, `8c4a4784`, `4b277b0b`.
- Calibrated by its controls: the dense families read flat on the current code, and the sparse families' growth follows the `BTreeMap` height staircase. It needs the wasmtime toolchain and minutes per family; the owner may prefer to fold its families into the board's touch or scan columns, but those meters cannot see map work.

**14. Native size probe**, `crates/before/tests/explore_l7.rs` `l7_probe_version_sizes` (feature `meter`): exact encoded sizes for every family and size, built from the same `include!`d builders.

## Scratch tooling (not on the branch)

`S/mutate.py` (unique-string mutants M1-M16, Z1-Z5, N1-N3), `S/calibrate.sh`, `S/calibrate_f.sh` (filtered runs; they now pass `--features touch-meter`, never `--all-features`, which would enable the prototype features), and `S/m3/table.py` (fuel tables).
