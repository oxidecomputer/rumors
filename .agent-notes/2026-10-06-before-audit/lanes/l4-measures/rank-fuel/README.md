# Rank operations under fuel: the investigation of the wasm scope's candidates

The auditor `audit-rank-fuel` read the rank path at #122's tip `e0956b25d`
to decide whether rank operations meet their documented bounds on the inputs
where the board-in-wasm scope (now the deferred note `.agent-notes/2026-10-20-board-in-wasm-fuel/`) saw fuel
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

## Owner's ruling (2026-10-20)

- **D1:** fix it now, in a subagent (`fix/rank-integrator-refill`).
- **The bound:** rank's `O(M(n))` means whatever the multiplication backend
  in use (`num-bigint`) achieves. Do not promise that in public
  documentation, which is not the altitude for it; state it in the
  maintainer-facing private docs where the bound is relied on.
- **D2:** resolved by #86. Re-measured at `d6c4a8118`
  ([`remeasure-after-written-set.md`](remeasure-after-written-set.md),
  evidence in [`remeasure/`](remeasure/)): fuel per byte is flat on both
  original families and four constructed ones, and `O(k + n)` holds. What
  remains is where the derivation is written down (`QUESTIONS.md`).
