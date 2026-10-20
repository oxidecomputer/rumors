# Rank operations under fuel: the investigation of the wasm scope's candidates

The auditor `audit-rank-fuel` read the rank path at #122's tip `e0956b25d`
to decide whether rank operations meet their documented bounds on the inputs
where the board-in-wasm scope (`../../l8-adequacy/board-in-wasm/`) saw fuel
grow superlinearly.

- [`report.md`](report.md): the hand-back: two defects, two questions, one
  simplification. It cites these files at scratchpad paths; the copies here
  are the archive.
- [`defect-rank-refill.md`](defect-rank-refill.md): the integrator replaces
  its segment-width and deferred-width accumulators at every freeze and
  deferral, so the next high write zero-fills a fresh buffer from digit 0:
  Θ(n²) bytes on `freeze-parade` and four other families. Test brief and
  the prototyped repair.
- [`defect-rank-sum-log.md`](defect-rank-sum-log.md): `Rank`'s `Sum`
  documented O(k + n), measured Θ(k log R) from `suanpan`'s zero-range map.
- [`simplification-integrator-one.md`](simplification-integrator-one.md),
  [`observations.md`](observations.md), [`coverage.md`](coverage.md),
  [`NOTES.md`](NOTES.md): the rest of the record.
- `patches/`: the probe, the repair prototype (`fix-only.patch`, and
  `seg-only.patch` with only the segment site fixed), and the
  allocation-volume prototype of the failing bands.

The coordinator verified on `main` at `6a329b17a` the two replacement sites
(`integral.rs`, `self.segment_width = Accumulator::new()`; `deferred.rs`,
`self.width = Accumulator::new()`), `Digits::add_at` growing an empty
buffer with `resize(position + 1, 0)`, and `Digits::reset` clearing from
digit 0. The measurements are the auditor's, not re-run.
