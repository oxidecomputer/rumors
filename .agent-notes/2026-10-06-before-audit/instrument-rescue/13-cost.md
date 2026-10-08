<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the instrument rescue's integrator, generated from one ranked list of every instrument in sections 01 to 09; for Finch's review. -->

# 13. Cost and resource instruments

A cost instrument adds coverage when it sees work that the board, the
focused meters, and the fuzz-fit bands miss
([`00-baseline.md`](00-baseline.md) section 5). Five rows belong to the open
`min_ticks` heap defect that question 87 stopped (09.A4, 03.5, 09.A9,
09.A15, 03.7). They rank by what they would add once that work resumes,
and 09.A4 shows the defect is present in `main`'s code at sizes the board
does not sample.

Each row condenses the linked entry in sections 01 to 09, which gives the
full account and marks every figure as verified, reported, or inferred; a
row's figures carry the linked entry's marks. "Rank" orders this category;
"Overall" is the instrument's position in the single ranking in
[`../instrument-rescue.md`](../instrument-rescue.md), section 3, and both
rank by the coverage an instrument adds against the work of folding it in.
`#n` is entry `n` of `QUESTIONS.md`; "slot 23", "slot 26", "slot 33",
"slot 45", "slot 46", and "slot 47" are ruled branches not yet ready. Where two lanes
built overlapping instruments, [`17-overlaps.md`](17-overlaps.md) says which
belong together or which subsumes which.

This category holds 23 of the 159 rows.

| Rank | Overall | Instrument | Adds beyond `main` and the ready branches | Evidence | Fold-in work and runtime | Depends on |
|---:|---:|---|---|---|---|---|
| 1 | 11 | [04.2](04-measures.md) `Rank` sum-order cost probe | Exercises the adversarial summand order of closed fix `rank-20`, whose committed meter the consolidation deleted (`193a14744`) and which the board's `rank_sum` row cannot see; the mutant E8 would read about 11 to 85 touches per byte against 0.5 today (inferred). | Caught nothing; never run against E8. | A focused check in `tests/meter/` with a ceiling from the mechanism and a floor; E8 must be rebuilt as its known-bad. 0.32 s. | None. |
| 2 | 12 | [09.A3](09-build-and-review-probes.md) Touch-bound worst-case families (F1 to F7) | Fixed programs reach 0.58 of #37's derived bound where #37's random programs reach 0.17, so a uniform 1.7-fold touch rise fails them where random programs need about 5.9. | Caught nothing; measures reach; the reviewer reproduced 0.5813. | `Vec<Step>` values judged by #37's `check_program`. 0.07 s. | #37. |
| 3 | 23 | [07.2](07-suanpan.md) Fuel matrix's operations outside the ladder | Join, meet, `partial_cmp`, tick by two parties, and the projection comparison on #52's families; #52 measures only decode, `<=`, and `min_ticks`, so a per-range suanpan regression on those paths could pass all 72 cells (inferred). | Measured F1 and design W's repair; no mutant run. | New `Operation` variants in #52's ladder over existing `ff_*` exports; compare against the lowered operand. About 144 cells, 25 to 40 s (inferred). | #52, #86. |
| 4 | 28 | [04.4](04-measures.md) `Ranked::cmp` tie-cost probe | Meters the settle-to-zero comparison that closed fix `rank-33` bounds; the board's `ranked_cmp` row never ties in rank. | Caught nothing; readings 1.02 to 1.07 per doubling. | A focused check in `tests/meter/settle_flatness.rs` with a floor. 5.9 s. | None. |
| 5 | 31 | [06.3](06-codecs.md) Hostile `Rank` and `Ranked` rejection families | The one decoder rejection path the board does not meter (no `rank_*` or `ranked_*` rejection rows). | Flat readings; no finding. | Board rows with floors, ceilings, and `WORST_RANKINGS` entries. 0.75 s as a probe. | #53. |
| 6 | 63 | [03.9](03-events.md) Random counter search for tick and `min_ticks` | The board's touch and scan ceilings over random co-generated pairs; `min_ticks` reads exactly 8.00 scan bits per byte. | Found nothing; maxima at 61% and 49% of the ceilings. | A property over #74's strategies; deterministic counters, no allocator. 8.8 s release. | #74. |
| 7 | 64 | [04.12](04-measures.md) Integrator cost search | The only random search over wide multi-scale shapes, at magnitudes neither the board's families nor fuzz-fit programs build. | Caught nothing. | A metered property over 04.1 with a touch ceiling, in its own process. 5.7 s. | None. |
| 8 | 65 | [01.8](01-identity.md) Identity scan-cost probe | `without` with a deep arbitrary receiver (the board meters only seed and self receivers), and a fork step dominated by count width. | Flat; its `is_disjoint` row measures nothing. | Two board families with ceilings and positive floors. 0.98 s. | Slot 26's ceiling rule. |
| 9 | 79 | [09.A4](09-build-and-review-probes.md) `min_ticks` heap probe over right spines, with a closed-form value oracle | 756 readings at sizes just past powers of two; on `main`'s code, 177 exceed the board's heap ceiling (up to 307.67 bytes per input byte), with every value matching an independent closed form. | Measures `main`'s open `min_ticks` heap defect beyond its record. | High: fails at `main` by design; installs a global allocator; builds through the recursive oracle on a 2 GiB stack. 0.3 to 14.2 s per test. | Question 87. |
| 10 | 80 | [03.5](03-events.md) Jump-entered rising spine (also 09.A8, its board-family port in `08573e159`) | The only family that breaks a committed ceiling (`min_ticks` heap 120 to 197 bytes per input byte against 20) and the strongest rank-fold touch case (1.28 to 1.32 times `harmonic`). | D1's witness and demonstration. | Ported in `08573e159` with 50 re-pins; fails acceptance at `main` by design. | Question 87. |
| 11 | 81 | [09.A9](09-build-and-review-probes.md) Limb work of `min_ticks` on wide-offset combs | `num-bigint` work no board currency counts; quadratic on the stopped design, linear at `main`. | Caught the stopped design's quadratic work. | Survey 1.1's form: comb families in #52's fuel ladder, with no hooks. | #52; question 87. |
| 12 | 82 | [09.A15](09-build-and-review-probes.md) Real-scale stepped spine for `min_ticks` | The only real-scale probe (a 91 MB input); showed the first fix design failing at a scale the board cannot reach. | Above. | Not a gate test; an on-demand developer check; `stepped_spine` ports small. | Question 87. |
| 13 | 83 | [03.7](03-events.md) Plain rising spine (heap) | The doubling sawtooth alone (19.7 to 39.4 bytes per input byte), without the spill. | D1's first witness. | As a board family its verdict depends on where the ladder lands. | Question 87. |
| 14 | 84 | [03.8](03-events.md) Random tick heap search | The per-sample heap ceiling over random co-generated pairs. | Peak 0.43 of the ceiling; found nothing. | A property over #74's strategies with a counting allocator. 13.1 s release. | Question 91's shared allocator; #74. |
| 15 | 85 | [09.A25](09-build-and-review-probes.md) Heap size sweeps for single operations | Lazily decoded fold inputs and sizes between board samples; found a lookahead heap rise and a forks heap cliff no board row showed. | Above. | A board row with lazily decoded inputs; more sizes. | Slots 23 and 33. |
| 16 | 86 | [01.9](01-identity.md) Identity heap-retention probe | Retained heap of six identity results after their inputs drop; only `fork` is checked today. | Found S1 and O1. | Capacity assertions (small) or the allocator. | Slot 33; question 91. |
| 17 | 104 | [07.4](07-suanpan.md) Hill-climbing touch adversary | Searches toward #37's bound and rescales widths. | Best ratio 9.6 under the old pricing; never calibrated. | Port to #37's pricing; keep ignored. | #37. |
| 18 | 105 | [02.10](02-algebra.md) Fragmented-mask cost families | Masks with regions at every depth inside one plateau. | Flat; no finding. | A board family after the board branches. | #93, #94, #95, ceiling rules. |
| 19 | 106 | [05.13](05-spans.md) Carry-boundary touch family | Probe-minus-bound differences crossing `2^k` in every cell. | Flat; asserts nothing. | A registry family with ceilings. | Question 65's rule. |
| 20 | 107 | [02.11](02-algebra.md) Carry-ripple cost families | The cliff comb's alternation on a balanced grid. | Flat. | A board family. | As 02.10. |
| 21 | 133 | [07.6](07-suanpan.md) Offset-comparison accounting probe | Per-call cost of one range-minima comparison. | Answered a lead with no defect. | Needs a production hook. | None. |
| 22 | 142 | [03.12](03-events.md) Memo-dense tick heap families | Same memo traffic as the board's memo families. | Confirmed a closed fix. | Not worth folding. | None. |
| 23 | 144 | [05.18](05-spans.md) Refinement scan-cost probe | Superseded by #61 and #94. | Motivated #61. | Not worth folding. | #61, #94. |
