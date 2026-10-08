# auditor-l8 (adequacy) status

Now: round 1 report returned. The mutation campaign continues on the box,
detached (no action needed from me until resumed):
- Process A (bash 17176, cargo-mutants 17180; log
  ~/src/rumors-audit-l8-adequacy/target/l8/campaign.log): version group,
  255 of 1,596 done at 15:36Z; then bits, then party.. (duplicates B; the
  coordinator stops A at "=== group party start").
- Process B (bash 13652; log target/l8/campaign2.log): rest (42 of 113),
  then span, rank, party.
Throughput fell to ~2 mutants/min at box load 375; version alone needs ~11 h.

Findings and briefs (round 1):
- findings/count-display-heap.md: worst-case pin drift is platform
  dependence (num-bigint FAST_DIV_WIDE) plus a 0.1% near-tie.
- findings/wasm32-pins-calibration.md: 7 of 11 injected narrowings caught;
  positions past 2^32 never exercised; two pin docs overstate.
- findings/instrument-calibrations.md: surface check 2/2; fold pins
  floor-only (fold degraded to quadratic passes them).
- findings/usize-invariance-instruments.md: what can see width dependence.
- coverage/NOTES.md, coverage/SUMMARY.txt: Mac branch coverage, 97.8% lines.
- survivors/INDEX.md: classified survivors (count, suanpan, partial version
  and rest).
- briefs/machinery-rank-decode-reader.md
- briefs/machinery-suanpan-normalize-bound.md (route to L7)
- briefs/machinery-trait-impl-coherence.md
- briefs/machinery-detached-nextest-timeouts.md
- briefs/simplification-hole-subtracts.md
- census-baseline.txt and l8_census.rs (reusable census, eec46dbb).

Blocking questions: none.

## Round 2 (hand-back about 23:30Z, 2026-10-07)

Item 1 done for every finished group and for the bits and party survivors
written so far (bits 269 of 628, party 277 of 467 at 23:13Z): 249 survivors
classified in survivors/INDEX.md (G 21, T* 3, T 39, C 84, E 53, U 38, I 10,
? 1). Diffs: survivors/diffs/<group>.md. Item 2: findings/coordinator-leads.md.
Briefs written or amended this round: machinery-range-minima-near-boundaries,
test-span-lone-endpoint-padding (span and clock), machinery-rank-decode-reader
(amendment), machinery-trait-impl-coherence (amendment, Bits note),
machinery-disjoint-families. Processes A and B keep running; the
classification recipe for their remaining survivors is at the end of
survivors/INDEX.md.

## Round 2 addendum (about 01:30Z, 2026-10-08)

Campaign complete: 3,919 mutants, 3,041 caught, 282 missed, 596 unviable.
All 282 survivors classified (G 21, T* 3, T 41, C 96, E 69, U 41, I 10, ? 1);
records under l8/records/ on explore/l8-adequacy.
