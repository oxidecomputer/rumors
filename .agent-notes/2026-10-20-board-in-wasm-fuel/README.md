<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) for Finch. It records a scoped and deferred direction; the measurements are a scope agent's, not re-run. -->

# Running `before`'s amplification board in wasm under fuel (deferred)

**Status: built, under review** (owner, 2026-10-20). The branch
`audit/board-fuel` adds fuel as the board's time currency; its builder's
report is [`build-report.md`](build-report.md). The audit tracks it.

## The idea

The amplification board measures heap, scan, and touch per input byte. None
of them sees work in `num-bigint`, `Vec` element moves, or `memmove`. The
board missed a deliberately quadratic `min_ticks` stack, and so did the
suite and `fuzzfit`. Running the board's operation-by-family grid in a
Wasmtime guest would add fuel as a fourth currency: a deterministic count
of every wasm instruction, given families that reach the costly shapes.

## What the scope found

- **It works unchanged.** The whole board compiles to
  `wasm32-unknown-unknown` with no change to any family or operation.
  `peak_alloc`, `touch-meter`, and `scan-meter` read the same in the guest,
  and the largest guest memory was 124 MiB.
- **Moves are priced.** Wasmtime 48 charges fuel per byte for `memory.copy`
  and `memory.fill`; Wasmtime 47 did not. The price must be pinned
  explicitly through `Config::operator_cost`, with a known-bad probe that
  checks an n-byte move reads at least n fuel.
- **One fresh instance per cell.** Fuel depends on allocator state that
  earlier cells leave in the same guest. Sharding the way the native board
  does makes readings depend on the shard count, by up to 37%. With one
  instance per cell, readings are identical at any thread count.
- **It catches the quadratic stack.** The mutant turned two
  `version_min_ticks` cells red at the board's own sizes.
- **Cost.** About 2.5 minutes of 8-core time for the full ladder, and about
  7 to 8 agent-days to build. The design is in
  [`brief-machinery-fuel-board.md`](brief-machinery-fuel-board.md).
- **It already found a real defect.** Its fuel readings led to the rank
  integrator's quadratic refill, which was then root-caused and fixed in the
  audit (`2026-10-06-before-audit/lanes/l4-measures/rank-fuel/`).
  Allocation volume, which `alloc-meter` already reports, would also have
  caught that defect.

## Open when this is picked up

1. **Unsafe.** The guest's fuel import needs an `unsafe extern` block with a
   `safe fn` and no unsafe call sites. The alternative is a host-driven
   protocol that splits the board's measurement across calls.
2. **Placement.** Should it be a review tool first, or a gate leg from the
   start?
3. **`fuzzfit`.** It pins rustc but neither Wasmtime nor its cost table.
   Should its cost table be pinned explicitly too?
4. **Allocation volume.** Should the native board record it alongside peak
   heap?
5. **A small defect, unfixed.** The board's `benign` family indexes with
   `(rng.next() as usize) % clocks.len()` (`board/family.rs`), so on 32-bit
   targets every `benign` operand differs from the 64-bit one. The fix,
   `(rng.next() % clocks.len() as u64) as usize`, is identical on 64-bit,
   so nothing pinned moves. See
   [`brief-defect-benign.md`](brief-defect-benign.md).

## Files

- [`report.md`](report.md): the scope agent's hand-back. It cites these
  files at scratchpad paths; the copies here are the record.
- [`scope.md`](scope.md): the full scope, with commands and evidence.
- [`brief-machinery-fuel-board.md`](brief-machinery-fuel-board.md): the
  design proposal.
- [`brief-defect-benign.md`](brief-defect-benign.md): the `benign` defect.
- [`candidates-rank-fuel.md`](candidates-rank-fuel.md): the two fuel
  readings that led to the rank investigation.
- [`NOTES.md`](NOTES.md): the agent's working notes.

The spike is on the branch `explore/board-in-wasm`, as six unsigned WIP
commits from `bf2ba264`. It never lands, but it is the starting point:
`crates/before/board-wasm` holds a guest and a harness.
