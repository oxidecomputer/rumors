<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), agent scope-board-wasm-fuel, for the before audit. A design proposal: it changes the board's internal API and adds a gate stream, so it needs the owner's ruling before a builder starts. -->

# Machinery brief: a fuel column for the amplification board, measured in wasm guests

Kind: design proposal (substantive). It adds a currency to the board, a
detached workspace with Wasmtime, and a gate stream, so it is for the owner
to rule on, not for direct dispatch. `scope.md` beside this file holds the
evidence; this brief states what to build and how to know it works.

## The failure class it catches

Time regressions in work that no existing currency counts: element moves
inside `Vec` (`memmove`), arithmetic inside `num-bigint`, `before`'s own
stacks, hashing, and allocator calls.

The constructible witness is mutant 12 on `fix/min-ticks-transient-heap`
(`IntegerStack.separate` using `insert(0, ..)` and `remove(0)`). It passes
the suite, the native board (5,545 green), and the worst-case pin. Under a
fuel column judged by the board's own rules it turns exactly two cells red:
`version_min_ticks × dense-suffix` and `version_min_ticks × promo-rearm`.

## What it extends

It extends the amplification board itself, since every family, operation,
judgment, capture, and comparison is reused. No existing instrument can
catch this class:

- Touch and scan count only `suanpan` digits and encoded bits, which mutant
  12 leaves unchanged.
- Heap is unchanged because the moves allocate nothing.
- Fuzzfit samples random programs that never hold thousands of wide frozen
  records; the coordinator reports it passes with the mutant compiled in.

## What to build

The design is section "The design I would build" in `scope.md`. Its points
are listed here in dependency order:

1. `Currency::Fuel` through readings, models, the shard wire form, capture,
   comparison, rendering, and the worst-case map, with the reader injected
   like `HeapMeter`. `Floors` stays three-currency: implementation code
   cannot bypass fuel, so its liveness is checked at the runner (point 5).
2. A per-cell measurement entry (`emit_cells` in the spike), and a merge
   that applies the existing completeness refusal to the union of per-cell
   results.
3. The guest crate, from the spike's `board-wasm/guest`, with panic recording
   ported from `wasm32-pins`.
4. The harness, from the spike's `board-wasm/harness`:
   - an engine with fuel and an explicit `OperatorCost` whose
     `variable.memory_copy_per_byte` and `memory_fill_per_byte` are set to 1,
     not inherited (Wasmtime 47 charged nothing per byte);
   - pooled instances and **one fresh instance per cell** on a work queue;
   - the `amp_board` modes, plus `calibrate`.
5. Judging and adequacy:
   - per-cell fuel ceilings written by `calibrate` under the `ceilings.rs`
     rule, committed and reviewed like a snapshot;
   - the 1.15 exponent bound, with declared fuel models for the cells whose
     documented time bound is superlinear (decimal conversion, the `O(M(n))`
     rank operations);
   - a provenance test refusing a different rustc or Wasmtime;
   - committed known-bad demonstrations: an `n`-byte memmove reads at least
     `n` fuel, the front-insert probe reads quadratic, and one board cell
     with a deliberately quadratic body reads red.
6. A cross-target identity check: native and wasm captures agree exactly on
   scan, touch, and every denominator (`brief-defect-benign.md` is its first
   catch).
7. A `board-fuel` stream in `gate-streams`, and an entry in
   `testing::validation_index`.

## Acceptance criteria, each with its reason

- **Readings depend on the cell alone.** Running the same tree with two
  different worker counts gives identical fuel for every sample. Without
  per-cell instances, 8 versus 5 shards moved 33% of samples, up to 21.6%.
- **The cell list is complete.** The merged grid matches every family's
  declared reach, as the native board's merge already requires.
- **Pricing is live.** The memmove demonstration fails when the cost table's
  per-byte entries are zero; demonstrate this once by setting them to zero.
- **Mutant 12 is caught.** Under the mutant, the fuel leg is red on at least
  the two `version_min_ticks` cells named above. Demonstrate this once by
  the reversible swap in `mutant12.py`, and confirm restoration with
  `git diff`.
- **Tolerances are stated.** Across a rustc step within one LLVM major,
  ceilings calibrated on one compiler held every cell on the other. Record
  the provenance beside the table, and re-run `calibrate` at each bump.

## Calibration evidence from the spike

All measured on the Mac (aarch64); logs in this directory.

| Injected defect or condition | Spike result |
|---|---|
| Mutant 12 (non-allocating quadratic memmove) | 2 new red cells at board sizes; heap, scan, and touch all green |
| Per-byte bulk pricing zeroed | front-insert excess grows linearly; quadratic invisible |
| Shard deal changed, no code change | no ceiling reds; tighter drift checks false-positive on 35 to 56 cells |
| Per-cell isolation, worker count changed | identical readings |
| One rustc step | no ceiling reds, no worst-family flips |

The spike did not test a touch-only or scan-only mutant. Until that study
runs, the fuel column is an addition beside touch and scan, not a
replacement.

## Inputs it reaches that committed generators do not

None: it reuses the board's families. What it adds is a currency over them.
Already on the unmutated tree, it shows 93 to 95 cells growing faster than
linear in fuel while every existing currency reads linear (`scope.md`,
question 3).
