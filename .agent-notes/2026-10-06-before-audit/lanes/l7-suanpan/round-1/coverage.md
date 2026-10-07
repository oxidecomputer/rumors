# Coverage record (lane L7: suanpan and its uses in before)

Revision: `explore/l7-suanpan` at `880aa203`, on base `58285ca5`; every commit signed. All runs on ox-east-1; logs in `S` = `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/auditor-l7`. Resumption commands are in `S/NOTES.md`.

## What I established, and how

### suanpan values: no defect found
- Instrument: a pool-model stateful differential, `l7_pool_programs_match_the_oracle` in `crates/suanpan/src/accumulator/tests/explore_l7.rs`.
- Scope:
  - every public operation, over three accumulators whose histories feed each other as operands
  - comparisons (`cmp_zero`, `cmp_zero_stable_under` against literal widths and against a pool member's `stored_bits`) only where the program asks, so states with stacked, uncompacted cancellation are reached; the existing surface suite compacts after every step
  - after every step, non-mutating checks only: both readouts against an exact `BigInt` oracle, the private representation invariants, known-zero soundness, the `2.01 * 2^stored_bits` bound, and the exact representable maximum
  - inline checks of normalize's exact width and growth, stability soundness including the accumulator-operand clause and the "at most two digits wider on `None`" clause, conversions with rejection preserving the representation, and reset
- Generators: swarm weights, representation-boundary shifts, large repeated shifts, extreme-digit parking, values near `k * 2^(32d)`, and embedded gap-split rounds.
- Runs: 20,000 cases passed (`S/long1.log`), plus several 2,000-case runs.
- Calibration (`S/calibrate.out`, `S/calibrate2.out`): caught M1 (decision threshold 3 to 2), M2 (stale range after a write), M3 (range rebuild without clear), M4 (negative readout complement), M5 (`i128::MIN` conversion), M6 (small stability threshold 3 to 2, after adding threshold-biased values), M7 (stability margin `f + 2` to `f + 1`), and M9 (collapse leaving the final digit). M8 (no recentering bias) trips production's own `debug_assert`, so it calibrates nothing.
- The existing suites catch every one of those value mutants (`S/calibrate_existing.out`), so I do not propose the pool model for committing: I cannot name a failure it alone catches. It stays on the explore branch as reusable search machinery.

### suanpan amortized bounds: hold, and I found an instrument gap
- Touch property: 20,000 serial cases pass (`S/long3.log`).
- Hill-climbing adversary, 12 seeds of 30,000 mutations: the best touch-to-work ratio is 9.6. That figure reflects my pricing of `i128` updates at 1 unit; every best program's ratio is unchanged or lower when all shifts are scaled by 2, 4, and 8 (`S/adversary2.log`, `S/adversary3.log`). No width-dependent cost was found.
- Instrument gap: the whole existing suanpan suite misses two amortization mutants (M14, M15: a range split drops its upper remnant), and the touch property with embedded gap rounds catches both. See MB1.

### suanpan at 32 bits
- Verified on wasm32: D1 (limb-index wrap); the shift-landing panics for three routes (existing pins plus my index-`2^31` control); the zero-shift representation dependence (Q1).

### suanpan contract probes
- D2 (`reserve_digits` panic).
- Q1 (zero shift keeps or demands `O(shift)` width).
- O1 (comparison fixed point at 6 touches).

### before's uses, by reading (each site priced against suanpan's table)
- Sound: `overlay`/`order` (one difference per walk); `lattice` (side-switch reads bounded by the boundary's deltas after compaction); `range_minima` (every negation, copy, and subtraction is of a value in transit that `cmp_zero` has just compacted); `place::filter` (rebuilds are priced by its documented `k * p` term); `integral` (freeze and deferral bound every repeated read); `min_ticks` (freeze bounds the per-leaf read); `tick` (emissions compact before reading; `StoredAccumulator` conversions are width-at-most-two); `tick::prescan` (one read per resolved level); `io::regions::Extremum`; `Rank::accumulate` and `Rank::sum_iter` (shift widths double).
- Not sound in composition: F1, measured with wasmtime fuel through a new explore-only fuel mode of the `wasm32-pins` harness, with a dense control.

## Blind spots that remain

- F1 is measured for `Version::decode` and `<=` only; `partial_cmp`, `join`, `meet`, `tick`, `projection`, `place`, and `admit` share the pattern by reading but are unmeasured. `min_ticks` needs a different family (its `answer` accumulator does not receive the opening height).
- `before`'s algorithm-level costs (tick, range minima, placement) belong to lanes L2 to L4 and the amplification board; I examined only whether their accumulator calls stay inside suanpan's contract.
- 32-bit paths whose triggers need gigabytes inside the guest were not run: `normalize`'s push at the last index, and `stored_bits` near `usize::MAX`.
- The touch meter does not see `ZeroRanges` map work, so no in-scope deterministic counter can pin F1 at the suanpan level. If the owner chooses a suanpan-side fix, a counter of map operations bounded by range-creating events would be the natural instrument.
- `Debug` output and the exactness of individual touch counts beyond the existing pins were not examined.

## Process notes

- A two-thread libtest run gave a spurious touch failure, because the touch meter is process-global. Serial runs and nextest are clean.
- My first `CARGO_TARGET_DIR` override sat outside `target/`, which the wrapper's `rsync --delete` would remove; I moved it under `target/`.
- To stop that run, I sent `pkill -f` with a pattern naming only my own target path on the shared `agent` account. Other agents' processes (L8's suanpan campaigns) could not match it, and the job I meant to stop in fact survived and finished; its result agrees with the clean rerun. I have since avoided pattern-based kills.
- One reversible mutation (M14's first spelling) became irreversible after application. I restored the three lines by hand and verified `git diff` empty before anything else synced.
