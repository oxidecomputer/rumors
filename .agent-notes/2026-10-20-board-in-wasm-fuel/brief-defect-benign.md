<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), agent scope-board-wasm-fuel, for the before audit. -->

# Defect: the board's `benign` family depends on pointer width

Severity: low (test support; no production code, wire format, or 64-bit
reading is affected).

## Record

**Clause breached.** The audit's `usize`-invariance ruling (README, "Rulings of
record", Contract model): "Both crates should also behave identically whatever
the width of `usize`: a `usize` anywhere other than indexing into memory, or
counting what memory holds, is a possible defect." The board is in scope
("the amplification board and meters").

**Site.** `crates/before/src/testing/meter/board/family.rs:1337` on `main`
(`bf2ba264e`), in `FamilyData::benign`:

```rust
let i = (rng.next() as usize) % clocks.len();
```

`rng.next()` returns a `u64`. On a 32-bit target `as usize` keeps only its low
32 bits, so for any population length that is not a power of two the chosen
fork parent differs from the 64-bit choice, and the whole `benign` population
(its version, its two parties, and its fold population) differs by target.

**Reproduction (verified).** On the Mac, the board swept natively (aarch64)
and inside a wasm32 guest, then `amp_board compare native.capture
wasm.capture`:

```
amp-board compare: 16083 rows: 456 growing, 630 uneven, 372 resized, 1810 constant, 12815 unchanged
resized     scan  version_decode x benign    axes before 3 5 134 288 549 1117 after 3 5 146 284 561 1152
```

Every scan and touch difference between the two targets is a `benign` row
(124 cells, each "resized": the operand itself differs). No other family
differs in scan or touch. Logs: `q3-compare.txt`, `q2-wasm-board.log`,
`q5-native-board.log` in this directory.

**Family.** Appears at the board's default and top scales (256 and more
clocks). It disappears at the small-input scale, where the population length
is 1 or 2 and the modulus reads only low bits (axes 3 and 5 match there).
It disappears on every 64-bit target, where `as usize` is lossless.

**Root cause.** A random `u64` is narrowed to `usize` before the modulus.

**Formats.** The fix changes no public API and no wire or storage format. On
64-bit hosts the corrected expression computes the same index, so no ceiling,
floor, or worst-case pin moves.

## Test brief

- **Invariant.** Every board cell's input is the same on a 32-bit target as on
  the 64-bit host, so its denominators, and its scan and touch readings,
  agree exactly.
- **Why the test must run 32-bit code.** On a 64-bit host the buggy and fixed
  expressions agree, so a native test cannot fail on the base commit. The
  audit's instrument for 32-bit targets is `wasm32-pins`.
- **Form.** An exhaustive check over the board's finite cell grid at
  `DEFAULT_SCALE` (scale 1; the small-input scale cannot show the defect),
  run in the `wasm32-pins` guest:
  - Guest: call `before::testing::meter::board::emit_shard(DEFAULT_SCALE, 0,
    1, &heap, &mut out)` with a `HeapMeter` whose readers do nothing (heap is
    target-dependent and not compared), and return `out`. The guest needs
    `before` with the `amp_board` example's features (`meter`,
    `touch-meter`, `scan-meter`, `serde`, `borsh`) so its operation table
    matches the host's.
  - Harness: compute the same emission natively with the same features, and
    compare the two line by line. For each `cell` line, assert equal
    operation and family names, equal denominators (`denom_bytes`,
    `exp_denom_bytes`) for both samples, and equal scan and touch readings.
    Heap and the floor fields are not compared.
  - Assertion message: name the first differing cell and field, for example
    `version_decode x benign: sample 1 denom_bytes 134 (host) != 146
    (wasm32)`.
  - Cost: a single-threaded scale-1 sweep; in a fuel-metered guest the
    8-shard scale-1 sweep took 7.6 s wall, so expect tens of seconds without
    fuel, within the leg's existing minutes.
- **Module.** `crates/before/wasm32-pins` (a new `Check` variant or a
  separate export; the harness test in `harness/tests/pins.rs`).
- **Expected failure on the base commit.** The first `benign` cell in board
  order reports differing denominators (the scale-1 operand is 134 bytes on
  64-bit hosts and 146 on wasm32 for `version_decode`).
- This check generalizes: it is the cross-target identity check in
  `brief-machinery-fuel-board.md`, and it will catch any later pointer-width
  dependence in what the board measures.

## Fix note

Take the modulus in `u64` and narrow the in-range result:

```rust
let i = (rng.next() % clocks.len() as u64) as usize;
```

`clocks.len() as u64` is lossless on every target Rust supports, and the
result is below `clocks.len()`, so the narrowing cannot truncate. The tick
count beside it (`rng.next() % 3 + 1`) already stays in `u64`. I checked two
ways that nothing else shares the mechanism: `git grep "next() as usize"`
over `before` and `suanpan` finds only this line, and the cross-target
comparison finds no other family whose scan or touch readings differ.
