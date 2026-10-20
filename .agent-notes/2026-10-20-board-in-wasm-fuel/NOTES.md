# scope-board-wasm-fuel: resumption record

Task: scoping study, can the amp board run in a wasm guest with Wasmtime fuel as
a deterministic time currency. Six questions (bulk memory/fuel; feasibility;
what fuel adds/misses; determinism/pinning; cost; motivating memmove mutant).
Deliverable is a written scope via SubagentHandback, spikes committed WIP on
`explore/board-in-wasm` in /Users/oxide/src/rumors-slot-52.

## Established (verified unless marked)

- Base verified 2026-10-20: slot-52 HEAD = bf2ba264e (main), clean, on
  explore/board-in-wasm.
- Q1 source reading (verified in ~/.cargo/registry sources):
  - Wasmtime 48.0.3/48.0.5 `wasmtime-environ/src/tunables.rs`:
    `VariableOperatorCost::new()` charges memory_copy_per_byte = 1,
    memory_fill_per_byte = 1, memory_init_per_byte = 1, memory_grow_per_page
    = 0, on top of the flat 1 per operator. Charged in
    `wasmtime-internal-cranelift-48.0.5/src/func_environ.rs`
    `translate_memory_copy` -> `pre_translate_bulk_op` (dynamic len: adds
    len*cost to fuel var; const len folded).
  - Wasmtime 47.0.4 has no VariableOperatorCost: bulk ops flat-priced there.
  - All three detached workspaces (fuzzfit, wasm32-pins, before-fuelscape)
    lock wasmtime 48.0.5. fuzzfit's Config sets no operator_cost (default).
  - `Config::operator_cost(OperatorCost)` exists in 48 (config.rs:655).
- rustc 1.97.1 (LLVM 22.1.6) `--print cfg --target wasm32-unknown-unknown`:
  bulk-memory, multivalue, mutable-globals, nontrapping-fptoint,
  reference-types, sign-ext on by default; target_has_atomic 8..64.
- No file under fuzzfit/wasm32-pins/fuelscape sets target-feature flags.
- fuzzfit's adequacy burner `ff_selftest_quadratic` is an ALU quadratic, not a
  memmove quadratic.
- fuzzfit pins rustc (`bands::PINNED_RUSTC` + `building_toolchain_matches_the_pin`)
  but NOT the wasmtime version or its cost table (only Cargo.lock does).
- Board: lives in `before::testing::meter::board` under feature `meter`; deps
  pure Rust; wasm32-pins guest already builds `before` with `meter` for
  wasm32 (baseline: wasm32-pins 8/8). `emit_shard` + host-side
  `run_acceptance(shards, spawner, out)` = natural guest/host split.
  Capture header is stamped by the host's cfg (target arch/os) -> a wasm
  capture is mislabeled as the host's target (design issue; spike exploits it
  to compare native vs wasm readings).
- No `cfg(target...)` in before/ or suanpan/ src; suanpan limbs are u64.
  Prediction (unverified): scan and touch identical on wasm32; heap differs.
- Mutant 12 (reviewer, l4 reviews-min-ticks/03-review-coverage.md): IntegerStack
  `separate` uses insert(0)/remove(0). Shapes WP(320,d), JR(320,d). Lives on
  `fix/min-ticks-transient-heap` only (not merged to main). Board samples WP at
  d=128*scale (ladder 128..1024), JR at b=40, d=8000*scale.

## Spike written: commit 385fc0996 (UNSIGNED: 1Password locked at 11:18)

- Board hook: measure.rs `install_time_meter`, `Sample.time`; shard.rs wire
  +1 field (PROTOCOL "amp-board-shard v6-spike-time"), `time_rows`,
  `TimeRow`, `TimeSample`; board.rs re-exports; tests.rs `time: None`.
- `crates/before/board-wasm/` detached workspace: guest (board_shard, probes,
  min_ticks_* behind feature min-ticks-shapes), harness (static, bulk, board,
  ladder). Cargo.lock seeded from fuzzfit's, resolved offline on the Mac,
  before's runtime deps pinned to root-lock versions (borsh 1.6.1, bytes
  1.11.1, serde 1.0.228, thiserror 2.0.18).

## Execution host (coordinator direction, 11:30)

- Owner via coordinator: ox-east-1 down; build and run on this Mac (aarch64,
  16 cores) under nice -n 10, -j 8, AMP_BOARD_SHARDS=8, tracked background
  tasks. Label every reading's host. One wall time per config, load disclosed.
- Build dirs: spike default -> /Volumes/forge/build/f9/1a8d1317361d79,
  target /Users/oxide/src/rumors-slot-52/target/board-wasm; nobulk guest ->
  CARGO_BUILD_BUILD_DIR=/Volumes/forge/build/scope-board-wasm-fuel-nobulk,
  target .../target/board-wasm-nobulk. (Coordinator retires these.)
- Guest (default flags) built OK 11:32: 1,143,342-byte wasm.

## Q1 measured (Mac aarch64; fuel host-independent) -- q1-bulk.log, q1-attribution*.log

- Default guest: 762 memory.copy, 10 memory.fill. Wasmtime 48 default costs:
  memmove n bytes = n + 54 fuel, memset n + 45; front-insert stack of k
  5-limb boxes minus back-stack = 8.0 k^2 (k=64..16384) -> quadratic visible.
- Per-byte costs zeroed (Wasmtime 47 schedule): memmove flat 54, memset 45 at
  every n; front-minus-back grows linearly (~13/k) -> quadratic INVISIBLE.
- `-C target-feature=-bulk-memory,-bulk-memory-opt` (warning: bulk-memory-opt
  "unknown and unstable"): 2 memory.copy left (prebuilt std), loops at 7.75
  fuel/byte (memmove), 2.5 (memset); front-insert 30 k^2; same under both
  pricings.
- `-bulk-memory` alone: no codegen change (nokache default vs nokache
  -bulk-memory byte-identical, 1,146,006 B). bulk-memory-opt is operative.
- kache vs no-kache default guest differ (1,143,342 vs 1,146,006 B): kache
  remaps panic paths to /kache/workspace; no-kache embeds worktree path.
  OPEN: does fuel differ between them (heap base shift)? Test per-cell.
- target_features section lists +bulk-memory in every build (prebuilt std).

## Q2/Q3/Q5 measured (Mac M4 Max, aarch64; load ~5.6-7.4) -- q2-wasm-board.log, q5-native-board.log, q3-compare.txt

- Whole board (5361 cells, main+hook) compiles to wasm32 and runs in 8 guest
  threads: scales 0.01/1/4 = 0.11/7.61/84.37 s; total 93 s wall (389% CPU);
  host peak RSS 1068 MiB; largest shard linear memory 4.7/26.9/123.9 MiB.
  Fuel totals 1.16e10 / 6.87e11 / 6.34e12. Acceptance (64-bit ceilings over
  wasm32 readings) 5361 green 0 red; worst pin drift: 16 lines, ALL heap.
- Native aarch64 amp_board acceptance, AMP_BOARD_SHARDS=8: 40.2 s wall (385%
  CPU), 5361 green; peak summed RSS 3237 MiB (single 541 MiB); pin clean.
- compare native vs wasm captures: 16083 rows; scan+touch differ ONLY on
  family `benign` (124 cells, resized axes); 5237 cells' scan and touch are
  identical across targets. Heap: 3020 cells differ, 2341 identical.
- DEFECT (test support, usize invariance): family.rs:1337 benign builder
  `(rng.next() as usize) % clocks.len()` truncates u64 on 32-bit ->
  different benign operands per pointer width. Fix: `(rng.next() %
  clocks.len() as u64) as usize`.
- Fuel fits (q2-fuel-summary.txt): 93 cells > 1.15. count_parse ~1.97
  (doc O(n^2)), count_display ~1.6 (doc superlinear subquadratic),
  freeze-parade rank ops 1.15-1.22 (doc O(M(n))): within documented bounds
  -> fuel needs per-cell time models. rank_sum x dense-suffix 1.17,
  x wide-arming 1.16: doc `O(k + n)` -> CANDIDATE finding; per-byte
  779/903/965/1128 over 127K..1M. Investigating with longer ladder (cells).
- version_rank x freeze-parade fuel = 914 n + 0.00167 n^2 (fits 4 points).

## Q4 determinism measured (q4-determinism.log, det-A/B/C, fueldiff.py)

- A (kache guest, 8 shards) vs C (no-kache guest, 8 shards): 32166/32166
  samples identical fuel. Build path / panic-path strings do not move fuel.
- wasm-main (guest without emit_cells) vs A: 32166/32166 identical.
- A vs B (5 shards): 10657/32166 samples differ (3738 cells); median rel
  4.3e-5, p99 4.7e-2, max 21.6% (small cells, +212 fuel). heap/scan/touch
  captures identical (16083 unchanged). => fuel depends on guest history
  (allocator state), i.e. on shard deal; breaks the board's "shard count is
  never an input" invariant for fuel. Remedy candidate: snapshot linear memory
  after family build, fresh instance + restore per cell (prototype pending).
- Same operand measured as s2@scale s and s1@scale 2s differs (e.g.
  version_rank x freeze-parade 108277733 vs 108398369): same mechanism.
- rank ladder (rank-ladder*.log): touch/scan/heap per byte FLAT over 64x;
  fuel per byte: rank_sum dense-suffix 779->1314->1230 (doc O(k+n));
  version_rank freeze-parade 997->6278 (local slope 1.80 at 3.2 MB; doc
  O(M(n)); Integrator add_product num-bigint products). Hand to coordinator.

## Q6 setup

- Merged fix/min-ticks-transient-heap into explore (8ae755aed, unsigned
  merge; conflict in board.rs re-exports resolved). Spike Cargo.lock gained
  suanpan->num-bigint edge (uncommitted at 11:58).
- mutant12.py apply|revert <integers.rs> (exact single-match swaps).

## Q6 measured (q6-base.log, q6-mutant12.log, q6-board.log; guests/q6-*.wasm)

- Baseline (merged tree 8ae755aed + lock 9fe6fa5d8): WP(320,d) 423 fuel/B
  flat d=64..16384; JR(320,d) -> 7876 fuel/B flat; board cells slopes 1.000.
- Mutant 12 (insert(0)/remove(0)): WP(320,d) fuel/B 431 -> 1230 at d=16384;
  excess = 2 d^2 exactly (8-byte Box<[u64]> on wasm32). JR(320,d) unchanged
  (+9 fuel): JR never holds >1 wide entry -> brief's JR lead disputed.
- Board-size cells: WP exponent 1.044, per-byte 474 < 534 ceiling -> WP alone
  would NOT catch at board sizes (scale 16: slope 1.25, 625/B: caught).
- Full fuel board base vs mutant (8 shards): 25 cells moved; exactly 2 NEW
  reds under board rules (ceilings.rs ceiling from base, exp<=1.15):
  version_min_ticks x dense-suffix (exp 1.291; 1051.8/B > 679) and
  x promo-rearm (exp 1.204; 1336.5/B > 1038). Each +537,026,551 fuel at top
  (= WP(320,16384) excess). Acceptance heap/scan/touch: 5545 green both.
- Mutant reverted, `git diff` over crates/before/src + suanpan empty.
- History noise alone (det-A vs det-B): 0 ceiling reds either direction;
  J2 exponent drift>0.02 flags 35 cells, J3 curvature>0.03 flags 56 -> tight
  judgments need isolation. Under mutant: J2/J3 flag exactly the 3 min_ticks
  cells (dense-suffix, promo-rearm, wide-step-pairs).
- Stray file /tmp/x written by a typo'd redirect (left in place).
- Process slip: q6 board pair launched with `&` (untracked); attached tracked
  watcher bw9cy6df0; completed exit 0.

## Isolation remedy measured (iso.log, iso/*.tsv; commit bac29de70)

- `isolated`: each cell in a fresh guest (family rebuilt per cell).
  Scale 1: 14.5 s (8 thr) / 23.1 s (5 thr); scale 4: 136.8 s (8 thr, 798%
  CPU, peak RSS 403 MiB; load 8-14 at end). vs shard mode 7.9 s / 85.7 s.
- Isolated t8 vs t5: 11090/11090 identical. Shard mode vs isolated: 85%
  of samples differ, p99 8%, max 37% (small clone/add cells).

## Toolchain churn measured (toolchain-bump.log, churn.py)

- rustc 1.96.1 (LLVM 22.1.2) vs 1.97.1 (LLVM 22.1.6), isolated, scales 1+4:
  39% samples identical; ratio p1..p99 within +-3%; max 1.22; 0 of 5545 cells
  over a 1.25x ceiling calibrated on the other; 0 of 272 worst flips.

## Blockers

- 2026-10-20 ~11:00: ox-east-1 unreachable ("Host is down" by IP
  192.168.118.110; mDNS unresolvable). Mac LAN up. Background watcher task
  b7p1niu9r (3h cap) notifies when ssh works.

## Analysis tooling

- `fuelfit.py summary|judge|cell` in this scratch dir: board-style 4-point
  fuel fits, per-byte maxima, per-cell ceilings C = ceil(1.25 x max/B).

## State at handback (12:20)

- Deliverables written: scope.md, brief-defect-benign.md,
  brief-machinery-fuel-board.md, candidates-rank-fuel.md.
- Unsigned commits (1Password locked): 385fc0996 967303848 3ee40592a
  8ae755aed(merge) 9fe6fa5d8 bac29de70. Tree clean.
- Build dirs to retire (coordinator): /Volumes/forge/build/f9/1a8d1317361d79
  (spike), /Volumes/forge/build/0f/feff6a16b64a27 (slot workspace),
  /Volumes/forge/build/scope-board-wasm-fuel-{default-nokache,nobulk,
  nobulk-only,rustc-1.96.1}; target dirs
  /Users/oxide/src/rumors-slot-52/target/board-wasm*.
- If resumed: next lines of work are the snapshot optimization, a
  touch/scan-only mutant through the fuel board, and an LLVM-major churn
  measurement.

## Old next

1. When box returns: build guest (default + `-C target-feature=-bulk-memory`
   into target/board-wasm-nobulk) and harness, `--locked`, niced.
2. `static` + `bulk` on both guests (Q1).
3. `board` with 24 shards (Q2/Q5/Q3); native `amp_board acceptance` and
   `capture` with AMP_BOARD_SHARDS=24 for comparison; compare captures.
4. Merge fix/min-ticks-transient-heap into explore branch; build with
   feature min-ticks-shapes; ladder WP(320,d), JR(320,d) baseline and mutant
   12 (insert(0)/remove(0) string swap, reverted, `git diff` checked).
5. Board-level fuel for mutant 12 at board sizes (fuel.tsv baseline vs mutant).
