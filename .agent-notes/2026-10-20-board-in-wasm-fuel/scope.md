<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), agent scope-board-wasm-fuel, as a scoping study for the before audit; every measurement below was taken on the Mac per the coordinator's relayed direction while ox-east-1 was down. -->

# Scope: the amplification board in a wasm guest, with fuel as a time currency

## Verdict

Feasible, and worth building, with one design requirement the obvious port
misses. The whole board (5,361 cells on `main`, 5,545 with the `min_ticks`
fix merged) compiles to `wasm32-unknown-unknown` unchanged and runs inside
Wasmtime guests at board sizes. Wasmtime 48 prices `memory.copy` and
`memory.fill` per byte, so a quadratic `Vec` shift is visible to fuel. Judged
by the board's own rules (a per-cell ceiling derived by the `ceilings.rs` rule,
and the 1.15 exponent bound), a fuel column turns the motivating mutant red at
the board's own sizes, on two cells, while heap, scan, and touch stay green.

The requirement: fuel, unlike the board's counters, depends on the allocator
state that earlier work left in the same guest. Sharded the way the native
board shards, a cell's fuel moves by up to 37% when the shard deal changes.
Measuring every cell in its own fresh instance removes the dependence
entirely (identical readings across thread counts) at about 1.7 times the wall
time.

Fuel is an addition to touch and scan, not a replacement: it sees work they
cannot (it already exposes superlinear work in `num-bigint` that every
existing currency reads as linear), but it is empirical, codegen-pinned, and
less sensitive per unit of work. Retiring touch or scan would need a mutant
study this scope did not run.

## Hosts, provenance, and deviations

- Every measurement ran on the Mac (Apple M4 Max, aarch64, 16 cores), under
  `nice -n 10` with `-j 8` and 8 workers, because ox-east-1 was unreachable
  ("Host is down") and the coordinator relayed the owner's direction to work
  locally. Fuel is a count of wasm operators, so it does not depend on the
  host. Native readings are aarch64 readings.
- Wall times are one measurement each with the load average stated; they
  estimate run cost and are pinned nowhere. Load was 5 to 7 for most runs and
  rose to 14 at the end of the isolation runs.
- The `common.md` rule against wall-clock measurement yields to this task's
  question 5, which asks for one.
- Rust 1.97.1 (LLVM 22.1.6), Wasmtime 48.0.5 (the version all three detached
  workspaces lock), `num-bigint` 0.4.8. The spike's lockfile matches the root
  lock for every runtime dependency of `before`.

## Question 1: bulk memory and fuel

**Measured.** Wasmtime 48 charges 1 fuel per byte for `memory.copy` and
`memory.fill` on top of the flat 1 per instruction; Wasmtime 47 did not. The
toolchain enables bulk memory by default, and the guest uses it.

- rustc 1.97.1's `wasm32-unknown-unknown` enables `bulk-memory` and
  `bulk-memory-opt` by default (`rustc --print cfg`). The default-flags guest
  contains 762 `memory.copy` and 10 `memory.fill` instructions.
- Source (read, Wasmtime 48.0.5): `VariableOperatorCost::new()` sets
  `memory_copy_per_byte = 1` and `memory_fill_per_byte = 1`; the cranelift
  translation adds `len * cost` to the fuel counter before the copy.
  Wasmtime 47.0.4 has no variable cost at all.
- Probe, default flags, Wasmtime 48 defaults: a memmove of `n` bytes costs
  exactly `n + 54` fuel and a memset `n + 45`, from `n` = 1 KiB to 4 MiB.
  Pushing `k` five-limb boxes at the front of a `Vec` and popping them from
  the front costs `8.0·k²` more than doing it at the back (k = 64 to 16,384):
  every shifted 8-byte wasm32 element costs 1 fuel per byte on insert and on
  remove.
- Same probes with the per-byte costs zeroed (the Wasmtime 47 schedule): the
  memmove is a flat 54 fuel at every size, and the front-insert excess grows
  only linearly. A quadratic shift is invisible there.
- With `-C target-feature=-bulk-memory,-bulk-memory-opt` the moves compile to
  loops costing 7.75 fuel per byte (memset 2.5), and the front-insert excess
  is `30·k²` under either pricing. `-bulk-memory` alone changes nothing: the
  module is byte-identical to the default build, because `bulk-memory-opt`
  governs `memcpy` lowering (rustc warns that it is an unknown and unstable
  feature name).

**Remedy and cost.** Keep Wasmtime 48's pricing, but set it explicitly
(`Config::operator_cost` with `variable.memory_copy_per_byte` and
`memory_fill_per_byte` stated as 1) in every fuel harness, and commit a
known-bad demonstration: an `n`-byte memmove must read at least `n` fuel, and
the front-insert probe must read quadratic. That costs nothing at run time and
about forty lines across fuzzfit and the new harness. Compiling bulk memory
out is not recommended: it depends on an unstable feature name, leaves two
`memory.copy` sites in the prebuilt std, and moves every reading.

## Question 2: feasibility of the grid in a guest

**Measured.**

- The board lives in `before::testing::meter::board` under the `meter`
  feature, with only pure-Rust dependencies. The guest builds with the
  `amp_board` example's features (`touch-meter`, `scan-meter`, `serde`,
  `borsh`) and needs no change to any family or operation.
- `board::emit_shard` and the host-side `run_acceptance`, `check_worst_map`,
  and `capture` are already the guest/host boundary: a guest emits the shard
  wire form, and the host merges and judges it with the board's own code,
  through a `ShardSpawner` that runs guests instead of processes. Every
  family generator ran at board sizes; acceptance over the wasm32 readings is
  5,361 green and 0 red against the 64-bit ceilings.
- Heap: `peak_alloc` works unchanged as the guest's global allocator (it is
  pure `std` with atomics, and it counts requested sizes, so the method
  matches native). Scan and touch are `AtomicU64` counters and work unchanged.
- Memory: the largest shard's linear memory is 124 MiB at the top scale, about
  thirty times below wasm32's 4 GiB. A 32× deeper ladder of single cells
  peaked at 425 MiB. Families of several gigabytes cannot run in wasm32;
  memory64 is the escape hatch (reasoned, not tried: Rust's `wasm64` target is
  tier 3).
- Sharding: threads replace processes. Each instance owns its linear memory
  and allocator, so the board's single-threaded-per-process rule dissolves.
  The native board's cell deal leaves CPU at 385 to 390% of 8 workers because
  the largest shard carries about twice the mean work; a work queue over
  individual cells reaches 798%.
- The fuel reading needs one host import (`spike.fuel_used`, read from the
  store). Declaring it takes an `unsafe extern` block with a `safe fn` item;
  no call site is unsafe. Under the owner's zero-unsafe rule this is a
  decision for the owner (see Risks).

## Question 3: what fuel sees that the native board misses, and the converse

**Measured: cross-target identity.** The board's `compare` mode, run on a
native aarch64 capture against a wasm32 capture, classifies all 16,083
readings:

- Scan and touch are identical for every cell except those of the `benign`
  family, whose operands differ by pointer width. That is a defect in the
  family's generator (`family.rs:1337`, `(rng.next() as usize) %
  clocks.len()` truncates the draw on 32-bit targets); see
  `brief-defect-benign.md`.
- Heap differs in 3,020 of 5,361 cells (pointer-sized fields halve, and
  `num-bigint` stores 32-bit digits), and the worst-case pin reports 16 heap
  ranking flips under wasm32 readings, none in scan or touch.

**Measured: work only fuel sees.** On the unmutated tree, 93 cells (95 with
the fix merged) fit a fuel exponent above 1.15 although every one of them is
green in every existing currency:

- `count_parse` near 2.0 and `count_display` near 1.6 (decimal conversion in
  `num-bigint`), within their documented bounds of `O(n²)` and "superlinear
  but subquadratic".
- `version_rank`, the ranked operations, `version_lag`, and
  `version_distance` on `freeze-parade`, documented `O(M(n))`. Along a 64×
  ladder, `version_rank`'s fuel per byte grows from 997 to 6,278 (local slope
  1.80 at 3.2 MB) while its heap, scan, and touch per byte stay exactly flat.
  Whether 1.80 exceeds `M(n)` for `num-bigint`'s multiplication is a question
  for the measures lane (`candidates-rank-fuel.md`).
- `rank_sum`, documented `O(k + n)`, drifts from 779 to 1,314 fuel per byte
  and back to 1,230 over 127 KB to 8 MB, with touch flat at 0.974 per byte.
  This is a candidate finding, not yet root-caused.

**Reasoned: what the wasm board cannot stand in for.**

- Heap on 64-bit targets, which is the production heap.
- Code that only 64-bit targets run: `num-bigint`'s `u64`-digit paths and its
  x86_64 intrinsics, and any `cfg(target_pointer_width)` path (none exists in
  `before` or `suanpan` today).
- The weights of real instructions: fuel prices a division like an addition,
  lowers `u128` arithmetic to library calls, prices allocation by dlmalloc's
  instruction count rather than the production allocator's time, and charges
  `memory.grow` nothing per page. Cache and memory-hierarchy effects are
  invisible.

**Replacement or addition.** An addition. Every extra digit touch or scanned
bit costs at least one instruction, so fuel sees what touch and scan see in
principle, but not with their sensitivity: their units are semantic, their
ceilings are derived from the algorithm, and they do not move with codegen.
Retiring either would require showing that the fuel board fails every mutant
they fail (the owner's retirement rule); I ran no such study. Because scan
and touch are target-invariant, the wasm board can carry them too, and a
committed native-versus-wasm identity check on scan, touch, and denominators
catches any pointer-width dependence in what the board measures (it found the
`benign` defect).

## Question 4: determinism and pinning

**How fuzzfit handles it today (read).** `bands::PINNED_RUSTC` names the
calibrating compiler, `build.rs` embeds the building rustc, and
`building_toolchain_matches_the_pin` refuses to judge under any other. `just
fuzzfit-calibrate` refits every band from a fixed 4,096-program corpus and
rewrites `bands.rs`, which is reviewed like a snapshot. Judgment uses
log₁₀ margins (0.2 above, 0.8 below). Nothing pins Wasmtime's version beyond
the lockfile, or its cost table at all, although the 47-to-48 change shows
the table can move.

**Measured.**

| Change | Samples with identical fuel | Spread among the rest |
|---|---|---|
| Kache build (remapped panic paths) vs plain build | 32,166 of 32,166 | none |
| Guest gains unrelated dead code | 32,166 of 32,166 | none |
| 8 shards vs 5 shards, same guest | 21,509 of 32,166 | median 4×10⁻⁵, p99 4.7%, max 21.6% |
| Shard mode vs one fresh instance per cell | 1,727 of 11,090 (scale 1) | p99 8%, max 37% |
| One instance per cell, 8 vs 5 threads | 11,090 of 11,090 | none |
| rustc 1.96.1 (LLVM 22.1.2) vs 1.97.1, per-cell instances | 8,668 of 22,180 | p1 to p99 within ±3%, max 22% |

- The shard-count dependence comes from allocator history: the same operand
  measured after different earlier cells reads differently, while heap, scan,
  and touch stay identical (all 16,083 readings unchanged). It breaks the
  board's documented invariant that the shard count never affects a reading,
  and it defeats tight judgments: with no code change, an exponent-drift
  check at 0.02 flags 35 cells and a curvature check at 0.03 flags 56.
- History noise alone never crossed a 1.25× ceiling in either direction, but
  its 21.6% maximum sits close to that headroom.
- Across the one toolchain step measured, no cell exceeded a 1.25× ceiling
  calibrated on the other compiler in either direction, and none of 272
  operation-and-scale worst families changed. An LLVM major bump is
  unmeasured.

**What the board would need.** One fresh instance per cell; per-cell fuel
ceilings generated by a calibrate mode under the `ceilings.rs` rule and
committed as a reviewed table; the exponent bound with declared fuel models
for the operations whose documented time bound is superlinear; and a
provenance record beside the table (rustc, Wasmtime version, and the explicit
cost table), with a test that refuses to judge under any other, as fuzzfit
does. A fuel worst-case pin is feasible on this evidence, re-pinned with the
ceilings at a toolchain bump.

## Question 5: cost

**Measured on the M4 Max, one run each.**

| Run | Wall | CPU | Peak memory |
|---|---|---|---|
| Native acceptance, 8 process shards | 40.2 s | 385% | 3,237 MiB summed RSS (541 MiB largest process) |
| Wasm, shard mode, all three scales, 8 threads | 93.1 s (0.1, 7.6, 84.4) | 389% | 1,068 MiB RSS; 124 MiB largest linear memory |
| Wasm, one instance per cell, scales 1 and 4, 8 threads | 151 s (14.5, 136.8) | 798% | 403 MiB RSS |

- Guest build is about 10 seconds warm; module compile under Cranelift is
  0.5 seconds; the harness's first build is 83 seconds (Wasmtime).
- Estimate for the whole ladder done right (one instance per cell): about 2.5
  minutes on 8 M4 Max cores, roughly 3.4 times the CPU of shard mode, because
  every cell rebuilds its family. Snapshotting a family's linear memory once
  and restoring it into each cell's fresh instance would recover most of that
  CPU (reasoned, not prototyped). On ox-east-1 at 24 jobs the leg should take
  one to two minutes (unmeasured).
- For the development loop, the spike's `cells` mode reads one cell's fuel
  along any ladder in well under a second for most cells.

## Question 6: the motivating mutant

**Measured.** Mutant 12 (`IntegerStack.separate` using `insert(0, ..)` and
`remove(0)`) on `fix/min-ticks-transient-heap`, applied by an exact string
swap and reverted (`git diff` empty afterwards):

| Input | Baseline fuel per byte | Mutant fuel per byte |
|---|---|---|
| `WP(320, d)`, d = 64 to 16,384 | 428 to 423 (flat) | 431 to 1,230 |
| `JR(320, d)`, d = 1,000 to 128,000 | 7,048 to 7,876 | identical (+9 fuel) |

- On `WP(320, d)` the excess is exactly `2·d²` fuel: `d/2` wide entries, each
  insert and remove shifting 8-byte wasm32 boxes at 1 fuel per byte.
- `JR(320, d)` never holds more than one wide entry, so the jump-entered
  rising spine does not exercise this mutant; the task's lead is wrong there.
- At the board's own sizes the `wide-step-pairs` cell does not catch it
  (exponent 1.044 against 1.15; 474 fuel per byte against a 534 ceiling), but
  two other cells do. A full fuel board on the baseline and on the mutant,
  judged by the board's rules applied to fuel, finds exactly two new red cells
  and loses none: `version_min_ticks × dense-suffix` (exponent 1.291; 1,051.8
  per byte against a 679 ceiling) and `version_min_ticks × promo-rearm`
  (exponent 1.204; 1,336.5 against 1,038). Each carries the same 537 million
  fuel excess at its top sample as `WP(320, 16384)`. Acceptance in heap, scan,
  and touch stays 5,545 green under both.
- Fuzzfit's miss (25 of 25 tests pass, the coordinator reports) is therefore
  reach, not pricing: its programs never hold thousands of wide frozen
  records, and its bands' 0.2-decade margin (1.58×) would need depths near
  8,000 even if they did.

## The design I would build

1. **A fuel currency in the board.** Add `Fuel` to `Currency` and to the
   readings, models, wire form, capture, comparison, rendering, and worst-case
   map, so the compiler forces every site to handle it. Inject the reader the
   way `HeapMeter` is injected, rather than through the spike's global.
   Keep `Floors` at three currencies: implementation code cannot bypass fuel,
   only the runner can, so its liveness belongs to the runner (point 5).
   Natively the column reads `off`, and the native board never renders it.
2. **Per-cell measurement.** A board entry that measures one named cell (the
   spike's `emit_cells`), and a merge that applies the existing completeness
   refusal to the union of per-cell results.
3. **The guest** (`crates/before/board-wasm/guest`, from the spike): the
   counting allocator, the fuel import, a staging buffer, the per-cell entry,
   and panic recording ported from `wasm32-pins`.
4. **The harness**: an engine with fuel and an explicit operator-cost table,
   pooled instances (fuzzfit's configuration), a work queue over cells with
   one fresh instance each, and the `amp_board` modes (acceptance,
   worst-case check, capture, compare), plus a calibrate mode that writes the
   fuel ceiling table.
5. **Judging and calibration.** Per-cell fuel ceilings by the `ceilings.rs`
   rule; the global exponent bound with declared fuel models for the
   operations documented as superlinear; a provenance test like fuzzfit's; and
   committed known-bad demonstrations: the memmove and front-insert probes
   (pricing), the ALU burner (liveness), and one board-level known-bad cell.
6. **A cross-target identity check.** Native and wasm captures must agree
   exactly on scan, touch, and every denominator.
7. **Relation to the other instruments.** The native board stays the board
   of record for heap, scan, and touch. Fuzzfit stays, because it samples the
   distributional bulk while the board samples constructed adversarial
   families. It should gain the explicit cost table and the memmove
   demonstration too, but fuzzfit is outside this audit's scope, so that is an
   observation for the owner rather than a brief. The two harnesses should
   share one pinned engine configuration and one rustc pin.
8. **Placement.** A `board-fuel` stream in `gate-streams`, beside the native
   board; review tooling (exact capture comparison, curvature and drift
   checks) stays outside the gate.

## Risks and remedies

1. **History dependence** (measured). Remedy: one fresh instance per cell
   (measured to work); optionally family snapshots to cut CPU.
2. **Sensitivity at board sizes** (measured). Mutant 12 is caught on two
   families but not on the family built for it; a small-coefficient quadratic
   on a family whose stack stays shallow escapes. Remedies: per-cell ceilings;
   once readings are isolated, curvature or per-cell drift checks become
   usable; an optional deeper fuel ladder outside the gate.
3. **Pricing set by Wasmtime defaults** (measured). Remedy: an explicit cost
   table and the memmove demonstration.
4. **Toolchain churn** (measured for one step, within an LLVM major).
   Remedy: the calibrate mode, the provenance refusal, and a reviewed diff at
   each bump; expect larger movement at LLVM major bumps.
5. **Model debt** (measured). About 95 cells need declared fuel models before
   the exponent check can pass, and deriving each takes the board's usual
   argument beside its row. Some may turn out to be findings (`rank_sum`).
6. **Fidelity** (reasoned). Fuel is an abstract instruction count on a
   32-bit target with dlmalloc; it serves asymptotic findings, which the audit
   puts first, better than constant factors.
7. **Unsafe** (decision for the owner). The fuel import needs an
   `unsafe extern` declaration in a tool-side crate. The safe alternative is a
   host-driven protocol in which the host reads fuel around a call that runs
   exactly one sample's body. That splits the board's `measure` across calls,
   and it charges the observer's constant work to the reading.
8. **Memory** (measured). Thirty times headroom at board sizes; families of
   gigabytes need memory64.

## Effort, in concrete pieces

| Piece | Estimate |
|---|---|
| Fuel currency through `ByCurrency`, wire form, capture, compare, render, worst-case map; injected reader | 1.5 days |
| Per-cell entry and union completeness check | 0.5 day |
| Guest crate from the spike, with panic recording | 0.5 day |
| Harness: engine, pooled per-cell instances, work queue, modes, calibrate, provenance test | 1.5 days |
| Fuel ceiling table and declared models for the ~95 superlinear cells | 1.5 to 2 days |
| Known-bad demonstrations and validation-index entry | 0.5 day |
| Cross-target identity check | 0.5 day |
| Justfile recipe, gate stream, board and guidepost docs | 0.5 day |
| Core total | about 7 to 8 agent-days |
| Optional: family snapshots to cut CPU | 1 day |
| Optional: touch and scan retirement study over the mutant catalog | 1 to 2 days |

The same pricing pin for fuzzfit is independent and small (a quarter day),
and could land first if the owner brings fuzzfit into scope for it.

## Briefs and records in this directory

- `brief-defect-benign.md`: the `benign` family's pointer-width dependence
  (defect record, test brief, fix note).
- `brief-machinery-fuel-board.md`: the build brief for the design above.
- `candidates-rank-fuel.md`: the `rank_sum` and `freeze-parade` rank
  readings, for the measures lane.
- `NOTES.md`: the resumption record, with every log path.
