<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), builder-board-fuel, of audit/board-fuel at 6a6c1f8c1. -->

## 1. Verdict

I built it as briefed, plus your two additions, and the work is committed. The fuel leg runs, deterministically, and is red on 14 cells. Each red is a defect fuel found on `main`, not something I declared as expected.

- Fuel is a fourth currency on the board's one generic path. The new `crates/before/board-fuel` workspace runs every cell in a fresh wasm32 guest instance. Its ceilings are calibrated per cell, its provenance is pinned, and it has a gate stream (`board-fuel`).
- Per-cell heap ceilings are built on the same machinery, generic over currency, with one table per architecture.
- The 14 red cells, all on the fuel exponent:

| Cells | Exponent | Cause | Evidence |
|---|---|---|---|
| 9 rank-family cells on `freeze-parade`: `version_rank`, `version_distance`, `version_lag`, `ranked_cmp`, `ranked_encode`, `ranked_encode_rank`, `ranked_decode`, `ranked_serde_deserialize` | 1.15 to 1.22 | The rank integrator's zero-fill at every freeze. `fix/rank-integrator-refill` exists but has no commits beyond `main`. | I applied the archived `lanes/l4-measures/rank-fuel/patches/fix-only.patch` to the box's synced copy only. All nine fell to an exponent of 1.15 or below, so the rank family gets **no** fuel model. |
| `rank_sum` on `dense-suffix` (1.17) and `wide-arming` (1.16) | 1.16, 1.17 | The zero-range map's log factor. | Matches your ruling D2 (hold until #86 lands). |
| `span_encode` on `jump-rising-spine`, `jump-pair`, `promo-rearm`, `hoisted-window` | 1.19 to 1.35 | A new finding: a constant factor that reads as an exponent. | See below. |

**The `span_encode` finding.** `Span::encode` (`crates/before/src/span/wire.rs`) is:

```rust
let mut bytes = self.lo.encode();
bytes.extend_from_slice(self.hi.as_bytes());
```

The extend reallocates, which copies the lower endpoint whenever dlmalloc cannot grow the buffer in place. That makes fuel per byte alternate between about 1 and 2 across ladder sizes. The cost is amortized linear, but the alternation fits exponents up to 1.35. The repair is to allocate `Vec::with_capacity(lo + hi)` once, which would also lower the heap peak. That is a production change, so I left it out of this branch.

## 2. Branch and commits

Branch `audit/board-fuel` in `/Users/oxide/src/rumors-slot-59`, at tip `6a6c1f8c1`. I verified the base was `c9c59b8d4` before starting.

- `d6f398a92` before: Draw the board's benign family identically on every pointer width. This is the `(rng.next() % clocks.len() as u64) as usize` fix; 64-bit readings are unchanged.
- `b4020ce8c` before: Judge the amplification board in Wasmtime fuel.
- `6a6c1f8c1` before: Hold every board cell's heap within 10% of its own reading. This is your addition, kept as its own commit.

All three are unsigned, since 1Password refused.

## 3. The pins

The provenance string recorded in `cell_ceilings/fuel_wasm32.rs`:
- rustc: `rustc 1.97.1 (8bab26f4f 2026-07-14)`.
- Wasmtime: `48.0.5`, read from the workspace `Cargo.lock` by `build.rs`.
- Cost table: flat 0 for `Unreachable Nop Block Loop Else End Return Drop`; 1 for the other 619 operators; every variable cost 1 except `memory_grow_per_page: 0`.

How the pins are held:
- The harness builds the whole cost table explicitly (`harness/src/costs.rs`): every operator through `wasmtime::wasmparser::for_each_operator!` (the `reexport-wasmparser` feature), and every variable-cost field by name. A Wasmtime release that adds an operator or a cost field fails to compile until it is priced.
- The test `fuel_ceilings_were_calibrated_under_this_build`, and the acceptance mode itself, refuse to judge under any other provenance.
- The board-fuel lockfile pins the guest's whole dependency graph to the root lock's versions. Copying fuzzfit's lock had given borsh 1.8.1 against the root's 1.6.1, and newer serde and thiserror.

## 4. Verification

Final verification ran on ox-east-1. Each leg ran either at the tip `6a6c1f8c1`, or at `8fd01a7d6`, whose tree differs from the tip only in one doc comment and the guest-build recipe line; those two legs I reran at the tip.
- `cargo fmt --all --check`: clean (Mac, at the tip).
- `just clippy` and `just clippy-default`: ok (box, `8fd01a7d6`).
- `just docs` and `just docs-internal` (rustdoc with `-D warnings`, both ways): ok (box, at the tip).
- `just test-all`: 1953 run, 1953 passed, 2 skipped (box, `8fd01a7d6`).
- `just doclint` and `just testdoc`: clean (Mac, at the tip).
- `just amp-board-acceptance`: "5545 green / 0 red" in all three matrices, and "worst-case pin: clean (272 pinned rows verified at 2 sampling scales)". It now runs under the per-cell heap tables, and the board still passes.
- `just amp-board-fuel` (fmt, clippy, the provenance test, then acceptance): exits 1 with "5531 green / 14 red" on the two growth matrices and 5545 green on the small inputs. The provenance test passed (1/1). Wall time was 415 s on the box, which is shared and was busy; the sweep alone took 281 s at 6 threads on the loaded Mac.
- **Determinism:** the tip's box capture (24 threads, x86_64 host) is byte-identical to the Mac's (6 threads, aarch64 host). `amp-board compare` reports "5545 rows: 5545 unchanged".
- Calibration by defect injection: not done, per your rulings.
- The landing check: not run, as you asked.

## 5. Design, and departures from the brief

- **The guest builds without touch, scan, and the counting allocator.** It compiles `before` with `meter`, `serde`, and `borsh` only, and supplies `Meters { heap: None, fuel: Some(..) }`. Those probes are instrumented work that fuel would otherwise count. A fuel run therefore judges fuel alone, through `Ladder::accept`, which is now public. The worst-case pin stays native.
- **One fresh instance per cell.** This reuses `emit_shard` and the merge unchanged, with `shards = max_useful_shards()` so each shard is one grid position. The merge refuses fuel readings from any other shard count. `FuelBoard::spawn` runs the shards on a work queue across threads.
- **The guest build is now path-independent.** The first box run differed from the Mac in 269 rows. The cause was the guest embedding host source paths: `/kache/workspace` on the Mac, `/home/agent/...` on the box, plus rust-src paths; their lengths shift the guest's memory layout. `amp-board-fuel-build` now remaps the checkout, `CARGO_HOME`, and the toolchain's rust-src to fixed prefixes, and sets `RUSTC_WRAPPER=` so kache cannot add mappings of its own. The modules still differ in crate-hash bytes, but every fuel reading is identical.
- **Captures carry the build that measured them.** Each shard child stamps its build in its header, so a fuel capture's header says `target=wasm32-`, not the host's target. The trailing dash is because `std::env::consts::OS` is empty for `wasm32-unknown-unknown`.
- **Model units widened to `u64`.** On wasm32, the old `Units::Scale` path cast float to `usize`, which saturates silently. The fuel models need units such as `d²`, which exceed 32 bits.
- **The no-op-model guard is removed.** `measure` asserted "a resource-model override must change a unit axis or ceiling". It misfires on `count_parse`'s `d²` model, because a family whose count stays at one digit gives `d² = d = 1` at every size. No test pinned the guard, and every override still shows on its rendered row.
- **The ceiling label now names which bound tripped.** The judge checks each sample against the tighter of the cell's own ceiling and its outer bound (the board-wide ceiling, or the row's declared ceiling). A trip of the outer bound reads `heap constant` as before; a trip of only the cell's own ceiling reads `heap cell constant`.
- **Departure: per-cell ceilings are stored in thousandths, not rounded to whole numbers.** `ceilings.rs`'s rule rounds up to a whole number. In the aarch64 heap table, 1,634 cells round to a ceiling of 1, so a cell reading 0.1 bytes per byte would be allowed ten times its own reading, which defeats your goal of locking in every cell's win. Thousandths keep the headroom within a point of 10% for any cell reading at least 0.1 per unit.
- **Departure: fuel's per-cell ceilings use 10% headroom.** Your brief said fuel ceilings follow `ceilings.rs`'s rule, which uses 25%. I applied your addition's 10% to every per-cell table, fuel included.
- **Judgment call: `FUEL_INTERCEPT = 1_000` and `FUEL_TREND_RESOLUTION = 1_000`.** Both are measured and documented beside their constants.
  - The intercept trades two kinds of looseness. With none, a cell's ceiling admits 1.25 times its top sample's fuel at the median cell and 32 times at the 90th percentile; at 1,000, it admits 1.12 and 3.3 times. The cost is that 368 cells whose every reading stays under 1,000 instructions are bounded by the intercept alone.
  - The resolution exists for `rank_cmp × benign`, which steps from 294 to 665 fuel between 4 and 7 bytes of input, as heap's fit resolution handles allocation steps.
- **Fuel models declared, each citing its documented bound:**
  - `count_parse`: growth against `d²` (documented `O(d²)`).
  - `count_display`: growth against `n^log2 3`, an envelope of `M(n)`. Its documented "superlinear but subquadratic" is accurate: num-bigint 0.4.8 converts to decimal by divide and conquer over Burnikel–Ziegler division. Under this model a quadratic regression reads red.
  - The balanced folds, through `fold_model` (`D log k`).
  - `party_forks_full` and `clock_forks_full`, sharing their scan work units (the public `k (D + log k)` bound).
  - `query_conjoin_many`, `query_contains_many`, `query_coverage_many`, sharing their touch work units, which lie within the documented `O(k·n)` and `O(n(k+m+1))`.
- **Not built, deliberately:**
  - The scope's memmove pricing probe: the rulings forbid known-bad demonstrations, and the explicit cost table plus the provenance pin hold pricing instead.
  - The cross-target identity check (scope item 6): it was not in your brief.
  - The family-snapshot speedup: excluded by your ruling.
- **Where I built:** calibration and pins on the Mac, as your brief said, overriding common.md's box-only rule.

## 6. The per-cell tables, as you asked

- **Size:** three generated files under `crates/before/src/testing/meter/board/cell_ceilings/`. Each has 5,545 entries in 5,560 lines: `heap_aarch64.rs` (275 KB), `heap_x86_64.rs` (275 KB), and `fuel_wasm32.rs` (291 KB). Each line is `("op", "family", ceiling_in_thousandths)`, sorted by operation then family.
- **What a re-derivation looks like in a diff:** only the moved cells' lines change. `just amp-board-calibrate` prints one line per moved cell for the commit message, for example `heap aarch64: version_ticks x wide-tooth-comb: 0.001 -> 0.852 (+85100.0%)`.
- **Tables are per architecture:** the aarch64 table was calibrated on macOS and the x86_64 table on illumos. They differ in exactly 42 cells, all `count_display`: num-bigint chooses its decimal-conversion divisor by architecture. The other 5,503 cells are identical.
- **One recipe, both currencies:** `amp-board-calibrate` rewrites the host architecture's heap table and the fuel table. Run it on each architecture of record; the fuel table should come out identical on every host.

## 7. What would make retiring touch and scan hard

Most of the board is generic: `ByCurrency::from_fn` and `each()` drive readings, models, the wire form, capture, compare, rendering, and judging, so removing those two currencies there is a deletion. The places that are not generic:
- `worst.rs`'s `MAP_CURRENCIES` and `WORST_RANKINGS` rows are positional `[&str; 3]` (heap, scan, touch). The pin table has to be reshaped across 272 rows.
- The cells' declared `Floors` struct has fields `heap`, `scan`, `touch`, and about 65 construction sites in `ops.rs` and `floors.rs` would each lose two lines.
- `Ladder::judge` requires the touch and scan features for the pin, and `amp_board`'s `required-features` and the justfile command name them.
- In the judge, the per-currency label `match` arms would lose two arms.

## 8. Other findings and housekeeping

- **A guard on the fuel board's inputs is missing.** Nothing checks that board-fuel's lockfile stays aligned with the root lock. If the root bumps num-bigint, the fuel board would measure the old version until someone aligns its lock. Fuzzfit's lock has the same gap.
- **Files I wrote outside my directories, left in place** because the rules forbid deleting outside my own directories:
  - On the Mac: `/tmp/x_guest_tree_unused`, a `cargo tree` listing I misdirected there.
  - On the box: `/tmp/builder-board-fuel-refill-fix.capture` and `/tmp/builder-board-fuel-refill-fix.log`.
- **Build directories to retire:**
  - Mac: the worktree's `target/board-fuel` and `target/board-fuel-remap-test`, and `/Volumes/forge/build/ba/1f95239292ee01`.
  - Box: `~/src/rumors-slot-59/target`, which includes `board-fuel` and `board-fuel-remap-test`.

Scratch logs and captures are in `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/builder-board-fuel/`; `NOTES.md` there is the resumption record.