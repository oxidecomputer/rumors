<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), reviewer review-board-fuel, of audit/board-fuel at eb4c2f393. -->

## 1. Verdict

**Changes required.** The measurement design holds up under checking.
- Each cell runs in its own fresh guest instance.
- The merge refuses fuel readings from any other shard count.
- The guest build is path-independent.
- The cost table is explicit and matches Wasmtime 48.0.5's defaults.
- The calibration rule is implemented as stated: recomputing the fuel table from the builder's capture reproduces 5,469 of 5,729 entries exactly. The other 260 are model cells and the flat-denominator family, where a capture's axis is not the constant unit, so they cannot be recomputed that way.
- The `benign` fix is identical on 64-bit hosts.

Five things are wrong or blocking. Two are holes where the gate passes vacuously, one is the missing tests on the new judge and calibration logic, and two are rustdoc claims that are false.

The branch has 3 unsigned commits on `2fe38aa07`, tip `eb4c2f393` (tree `42cf95d6`) in `/Users/oxide/src/rumors-slot-59`. I left it unchanged (`git status` is empty, HEAD is still `eb4c2f393`) and created no scratch worktree.

## 2. Blocking findings, most severe first

**B1. A fuel run with no fuel readings passes green.** This is a liveness gap.
- `judge.rs` (`judge_window`) skips any currency whose `per_unit` is `None`, and the floor loop skips `None` readings too. So `FUEL_FLOOR` catches a reader that returns a constant, but not a missing one.
- If the guest supplied `fuel: None` (`board-fuel/guest/src/lib.rs`, `shard`), `Ladder::accept` would render a matrix with no columns, `summary.red == 0`, and `amp-board-fuel` would exit 0.
- Harness `calibrate` guards this ("the sweep measured no currency that has a per-cell table"); harness `acceptance` does not.
- This also loosens the discipline the example's `required-features` comment states ("a board that looks judged and isn't"): heap is now `Option` too.
- Repair: the fuel acceptance asserts that fuel was measured. Better, `accept` and `judge` take the currencies a run must have measured and refuse otherwise.
- Evidence: reading only. Per the rulings, I did not build a broken guest.

**B2. Recalibrating without the other architecture's capture silently turns off the per-cell heap ceilings on that architecture.**
- `recalibrate_table` (`cell_ceilings.rs`) sets `archs` to the ladder's arch plus each capture's arch. `just amp-board-calibrate` with no argument, which the justfile documents as "rewrites the heap table for this host's architecture alone", therefore writes `archs: &["aarch64"]`.
- From then on, `calibration("x86_64")` returns `None` for heap, so the box (where the landing check runs) judges heap against the board-wide 15 B/B alone. No leg fails.
- A same-architecture capture has the same effect. Nothing pins which architectures are of record.
- Repair: a committed constant listing the heap architectures of record, a test that the heap table's `archs` equals it, and a refusal in `recalibrate` when the merged set differs.

**B3. The new judgment and calibration logic has no committed test.**
- The only new tests are `tables_are_sorted_and_distinct` and the provenance check.
- Untested:
  - `recalibrate`'s rule: 10% headroom, rounding up to thousandths, the clamp at zero, the larger reading across captures, and its refusals for debug builds, missing rows, and size mismatches.
  - The judge's tighter-of-two check and its `cell constant` versus `constant` labels.
  - The merge's refusal of fuel readings from other shard counts.
  - `BuildStamp` render and parse, including wasm32's empty OS (`target=wasm32-`).
- `calibrate` and `judge` compute the same boundary two ways, so a property should tie them: every sample of a ladder judges green under the table recalibrated from it, and a reading pushed past its ceiling turns red. That is a test of the real code, not a broken implementation.

**B4. Rustdoc overstates what a per-cell ceiling holds.**
- `cell_ceilings.rs:9-10` says "so any regression larger than its headroom turns the cell red". `board.rs` says "each cell's heap is also held within 10% of its own calibrated reading". `validation_index.rs` says "locks in that cell's reading within 10%", and the heap commit's subject says the same.
- The rule holds each cell's *worst normalized sample* within 10%; its other samples get more room. I recomputed allowed-over-actual for the top sample:
  - **Fuel** (builder's `fuel-B-mac.capture` against the committed `fuel.rs`, 5,469 cells): median 1.122, p90 3.10. On 969 cells the top sample may more than double without turning red.
  - **Heap** (`heap-x86_64.capture`, x86-dominated cells): median 1.096, p90 1.21. 224 cells may more than double, and 1,257 cells are bounded by the intercept alone.
- The `FUEL_INTERCEPT` doc states this trade honestly. The prose above contradicts it.
- The fix is either to correct the prose or for the owner to choose per-sample ceilings (6 per cell) if "10% above its reading" is meant literally. That choice is a design question for the owner.

**B5. Factually wrong claims in docs.**
- `cell_ceilings.rs:8`: "under the heap ceiling of 18". `MAX_HEAP_BYTES_PER_INPUT_BYTE` is `15.0`. The number is also hand-maintained; it should be an intra-doc link.
- `board-fuel/harness/src/costs.rs` module doc: "A Wasmtime release that adds an operator … fails to compile here until the new entry is priced." That is false for operators.
  - Wasmtime 48.0.5 generates `OperatorCost` from the same `wasmparser::for_each_operator!` that `define_costs` expands, so a new operator would be priced 1 automatically and still compile.
  - What actually catches it is the provenance string ("1 for the other 619 operators" becomes 620).
  - Only a new `VariableOperatorCost` field fails to compile.
  - Verified in `wasmtime-environ-48.0.5/src/tunables.rs`.
- `ops.rs`, `multiplication_units`: "num-bigint multiplies by Karatsuba, and by Toom-3 above it, at the sizes the board's growth ladder reaches, so these units grow at least as fast as … M(n) there." That is false on wasm32.
  - A limb is 4 bytes there, and `mac3` uses long multiplication up to 32 limbs (128 bytes). Divide-and-conquer conversion starts at 32 limbs, and Burnikel–Ziegler needs a divisor over 64 limbs.
  - From the capture's axis (n^log2 3), `count_display × cliff` spans about 33 to 257 bytes, so it sits in the quadratic regime.
  - The cells pass because fixed costs dominate. The derivation should say the envelope is asymptotic and name the thresholds.
- `cell_ceilings.rs` "# Tables": "A currency the run measured with no table … is judged against its outer bound alone, and the rendered board says so." Fuel has no outer bound, so the judge panics instead (`accept`'s `# Panics` says so). For heap, the board says so only by omitting the clause.

## 3. Prose findings against Part IV

- **Blocking:** B4 and B5 above. They are inaccurate rustdoc.
- `validation_index.rs`: "Cost claims are guarded by four complementary instruments" is a hand-maintained count, and this branch had to bump it from three. State the structure instead. Non-blocking.
- `board.rs` "# Target dependence": "the board's ceilings and its worst-case pin are taken and checked on 64-bit hosts" is now false for fuel (calibrated in wasm32). Blocking-light: it is an accuracy fix.
- `harness/src/lib.rs` `MAX_WASM_STACK`: "guest stack use is bounded by the deepest static call chain". num-bigint's conversion and Burnikel–Ziegler division recurse to logarithmic depth. Weaken to "logarithmic in the input". Non-blocking.
- `FUEL_INTERCEPT` doc: its measured figures (800, 1.25 and 32 times, 1.12 and 3.3 times) already differ slightly from the tip's table (I get p90 3.10). Re-measure them at the final recalibration. Non-blocking.
- Commit `37e257eca`:
  - "pending #86" is an opaque roster ID.
  - "fixed on fix/span-encode-one-allocation" names an ephemeral branch.
  - The red list must be rewritten at the planned recalibration in any case.
- Commit `eb4c2f393`: the subject repeats the B4 overclaim.
- Commit `b32dcb2f0` is clean.
- `shard.rs`: the module doc's reflowed paragraph has an overlong line. Cosmetic.

## 4. Non-blocking suggestions and design questions

1. **Unsafe (ruling question).** The guest declares `unsafe extern "C" { safe fn fuel_used() -> u64; }`. The design README lists this as open question 1 for the owner. I find no recorded ruling, and the task prompt's rulings do not mention it. `alloc-meter` is precedent for test-side unsafe. Please confirm the owner ruled.
2. **Guest rustc provenance.** The harness's rustc stands in for the guest's. The guest module carries rustc's `producers` custom section, which the harness could read through the re-exported wasmparser, so it checks the guest's own compiler. `BOARD_FUEL_GUEST_WASM` can otherwise point at any module.
3. **Heap provenance.** The heap table's `provenance: "the amp_board example"` records nothing and nothing checks it. Either record rustc, or drop the field for heap.
4. **Path remapping.** Add a cheap committed check in `amp-board-fuel-build` that the built wasm contains no unmapped host path. Cross-host identity currently rests on one Mac-to-box comparison. I scanned the builder's `guest-box.wasm`: only `/rumors`, `/cargo`, `/rustc`, and std's own `/rust/deps` paths appear.
5. **Same-tree captures.** `recalibrate` documents "must come from … the same tree" but checks only sizes. The justfile procedure should say "at the same commit", or the capture could carry a tree id that `calibrate` alone checks.
6. **No-op model guard.** It was removed outright. A per-operation form ("an override changes units at some sample of its row") would not misfire on `count_parse`'s one-digit families.
7. **`usize` work units.** The commit says "Units count work … u64 on every target", but `ops.rs` still computes work products in `usize` with `checked_mul(...).expect(...)`, then casts `as u64`. On wasm32 those overflow at 2³² and panic in the guest. Compute them in `u64` at the source.
8. **Duplicated table writer.** The harness `calibrate()` duplicates the example's `write_recalibrations` and `write_atomically`. A shared library writer would remove the copy.
9. **Redundant test step.** The `nextest` step in `amp-board-fuel` repeats the check `acceptance` already performs first.
10. **Thread count.** The fuel harness defaults to `available_parallelism` (192 on the box), at up to 124 MiB of guest memory per instance. Consider a default cap for gate runs.
11. **Generality** (beyond the builder's list):
    - `amp_board_smoke.rs` hard-codes the wire layout as `chunks_exact(14)` and `sample[6]`, so retiring two currencies means editing magic numbers.
    - `Floors` changed from an alias into a struct with heap, scan, and touch fields (the builder lists this).
    - The `ceiling_text` and unit `match` arms in `render.rs`.
    - Everything else on the board path is driven by `ByCurrency::from_fn`/`each`.
12. **Landing check.** As described in `README.md`, the landing check does not run the new `board-fuel` stream. The fuel verdict needs its own run before landing.

## 5. Evidence I ran (all local, read-only; no builds or runs on the box)

- **Benign fix.** On 64-bit, `(x as usize) % len` equals `(x % len as u64) as usize`. No other pointer-width-dependent draw exists in the board (`grep` empty).
- **Cost table.** `wasmtime-environ-48.0.5/src/tunables.rs`: the default free set is {Nop, Drop, Block, Loop, Unreachable, Return, Else, End}, and the variable defaults match `VARIABLE_COSTS`, including `memory_copy_per_byte` and `memory_fill_per_byte` = 1 and `memory_grow_per_page` = 0. `OperatorCost` is generated by `wasmparser::for_each_operator!`.
- **num-bigint 0.4.8.** `to_radix_digits_le` switches to divide-and-conquer at 32 limbs; `div_rem_core` uses Burnikel–Ziegler for dividends over 128 limbs with divisors over 64; `mac3` uses long multiplication up to 32 limbs; `BigDigit` is u32 on 32-bit targets.
- **Deal.** `owns` at `count == max_useful_shards()` assigns exactly one grid position per shard. The ordering argument in `spawn` (an in-order scan meets the failure first) holds.
- **Features.** The guest uses `before` features `meter`, `serde`, `borsh`; `default = []`; suanpan's `touch-meter` is off by default; no `#[global_allocator]` in the guest.
- **Rustflags.** There is no repo or user rustflags config for the guest recipe's `RUSTFLAGS` to clobber.
- **Ceiling recomputation** (`slack.py` and inline scripts in my scratch directory):
  - Fuel: 5,469/5,729 entries exact. The rest are `count_display`, `count_parse`, the `_all` folds, and `comb-scatter`.
  - Slack figures as in B4.
- **Surface check.** `testing::meter::` is excluded by prefix, so the new public items do not move the count.
- **Gate.** I did not run it, since the task said a landing check is already running. At the time I read `landing-59/check.log`, it showed `ok surface (58s)` and `ok board (97s)`, still in progress.

Files: `/Users/oxide/src/rumors-slot-59/crates/before/src/testing/meter/board/cell_ceilings.rs`, `.../board/judge.rs`, `.../board/shard.rs`, `/Users/oxide/src/rumors-slot-59/crates/before/board-fuel/harness/src/costs.rs`, `/Users/oxide/src/rumors-slot-59/crates/before/board-fuel/guest/src/lib.rs`, `/Users/oxide/src/rumors-slot-59/justfile`. My notes and scripts are in `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/review-board-fuel/`.