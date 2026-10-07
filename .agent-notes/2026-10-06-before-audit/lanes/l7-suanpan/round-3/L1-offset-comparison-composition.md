<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), auditor-l7, round 3. -->

# L1: `Anchor::compare_offset_to` re-reads the gap; is its cost composed correctly?

Kind: lead evaluation (no defect). It answers lead 1: "`compare_offset_to`: the events lane (L3) noted that it re-reads the gap, and whether suanpan's amortization holds across that re-read is unexamined. Is there a cost-composition defect like F1 here?"

The function lives at `crates/before/src/version/range_minima/anchor.rs` (`compare_offset_to`, around line 272), not `crates/before/src/anchor.rs`.

## Verdict

No new defect. The touch cost of each call is paid by that call's own operands, measured on every board family that reaches the function at four doubling sizes. A wide gap is read in constant touches per call. The one non-constant factor that can enter this path is F1's per-write map lookup, inferred rather than measured here; design W removes it on every path, this one included.

## The mechanism

`compare_offset_to(above, candidate)` computes the sign of `gap - candidate + above` on the anchor's persistent `gap` accumulator, then restores it:

```rust
self.gap -= candidate;
self.gap.add_bigint(above);
let sign = self.gap.cmp_zero();
self.gap.sub_bigint(above);
self.gap += candidate;
```

Its only caller is tick's memo protocol (`crates/before/src/version/tick.rs`, `consume_lookahead`, the `MemoReference::Min` branch). For each consumed memo entry there, the follower `A - reference` is negated, added to the memo link, compared, and (on the non-raising arm) negated back and stored.

Why the re-read cannot compound (reading, then measured):

- **The writes cost their operands.** The four updates write `O(|candidate| + |above|)` digits. That is the documented cost of those operations.
- **The sign scan is paid by writes to the same accumulator.** `gap` is one long-lived accumulator, so suanpan's amortization applies across every call on it. A scan step that does not decide clears a digit someone wrote. A step that decides reads a constant number of top digits.
- **Restoring does not undo compaction.** The restoring writes are as narrow as the operands. A top the scan compacted stays compacted, so the next call does not re-pay it.
- **The candidate's width is paid by the input.** The candidate is `target - A`, where `A` is the live anchor. This is an argument, not a proof. The deferred part of `A` (`A - m`) is consumed by the arming this same consume performs, and it was created by one boundary fold. The true part `m - reference` is bounded by the values in the input region between the previous memo site and this one, and those regions are disjoint across consumes. So the candidates' total width is linear in the input plus the number of consumes.

## Measurements (verified, explore `5358b7f8`, `S/r3/leads2.log`)

An explore hook records, per call, the touches spent inside `compare_offset_to`, the candidate's stored digits, the digits of `above`, and the gap's stored digits on entry. The probe `l7_compare_offset_accounting` (in `crates/before/src/testing/meter/board/l7_compare_probe.rs`) ticks every board family that has a cross bundle once, at levels 0 through 3 (each doubling the last), and reads the counters.

Ten families reach the function. Per family, across the four sizes:

| family | calls | touches per operand unit | candidate digits per input byte | share of tick's touches | largest single call |
|---|---|---|---|---|---|
| MirrorWide | 499 to 3,999 | 1.00 | 0.885 to 0.888 | 0.21 | 3 |
| MirrorNarrow | 1,499 to 11,999 | 1.00 | 1.140 to 1.143 | 0.21 | 3 |
| RevealComb | 499 to 3,999 | 1.00 | 0.362 to 0.364 | 0.10 | 3 |
| RevealHifloor | 500 to 4,000 | 1.00 | 0.363 to 0.364 | 0.09 | 7 |
| MemoChain | 249 to 1,999 | 1.67 | 0.181 to 0.143 | 0.12 | 5 |
| MemoComb | 250 to 2,000 | 1.33 | 0.337 to 0.271 | 0.13 | 5 |
| MemoFanout | 249 to 1,999 | 1.00 | 0.151 to 0.152 | 0.08 | 3 |
| MemoOscillating | 249 to 1,999 | 2.45 to 2.49 | 0.122 to 0.125 | 0.18 to 0.19 | 82 to 642 |
| MemoChurn | 199 to 1,599 | 1.67 | 0.149 to 0.123 | 0.09 | 5 |
| DescendingRaises | 199 to 1,599 | 1.67 | 0.183 to 0.145 | 0.12 | 5 |

"Touches per operand unit" is the call's touches divided by `2 * (candidate digits + above digits) + calls`: the writes it must make plus one decision step per call.

Two rows answer the lead directly:

- **MirrorWide is the re-read itself.** The gap on entry averages 16, 32, 63, and 125 stored digits as the family doubles, while every call costs exactly 3 touches: one write, one decisive scan step, one restoring write. A wide gap is re-read every call, at constant cost.
- **MemoOscillating has wide candidates.** Its largest call grows with the family's width (82 to 642 touches), but candidate digits per input byte stay flat (0.122 to 0.125), so wide candidates appear exactly as often as the input pays for them.

## What I constructed that did not reach the function

I built two chains whose left leaves are `2^b - 1` over zero floors, crossed with the left-full id (`left-chain-all` and `left-chain-top`, `b` = 256 to 2,048, depths 64 to 512), meaning to force a wide follower. They made zero calls. Every consume on them stays on the height-relative branch (`consume_h_anchored`), because the memoized minimum never exceeds the owned maximum there. The `Min` reference, and with it this function, arises only after a raise (the memoized minimum dominates) or after a nested pre-scanned range closes (`pop_lookahead`). The board's memo families construct exactly those transitions, which is why they are the evidence above. The negative result also shows that a wide owned maximum alone does not reach this path.

## What remains inferred

- **Map work on this path.** If `gap`, the candidate, or the follower carries recorded zero ranges, each of their writes pays F1's map lookup (and a jump pays an insertion), as on every other path. The touch meter cannot see it. I did not measure fuel on the memo path. Item 1's ladder measures decode, `<=`, and `min_ticks`, and round 2 measured tick only with parties that never consume memo entries. Design W removes the lookup from every write, so no separate fix is needed. A tick row on the ladder, over a version whose memo sites sit under a sparse wide opening, would turn this inference into a measurement; I would add it only if the owner wants tick in item 1's table.
- **The candidate-width argument** is an argument. The committed check that enforces its consequence already exists: the board judges tick's total touches per input byte on all of these families, and a regression that made candidates wider than the input pays for would raise those readings. I propose no new instrument. Every concrete failure here that I can construct is one the board's tick cells already fail on.
