# `RangeMinima`'s wide boundaries: demonstration, two candidate fixes

The fixer `fixer-range-minima-wide` added a board family that exposes the
defect (`fix/range-minima-wide-boundaries`, commit `e9ab04ba2`) and stopped
before committing a fix, because each candidate needs an owner ruling.

- [`report.md`](report.md): the hand-back, with the measurements and the
  decision it asks for. It cites these files at scratchpad paths.
- `candidate-copy.patch` and `candidate-box.patch`: the two fixes, each
  applying on `e9ab04ba2`. `rmw_probe.rs`: the uncommitted heap probe.
- `evidence/`: board logs, comparisons, probe logs, and gzipped board
  captures for the base, the demonstration, and each candidate.
- [`NOTES.md`](NOTES.md): the fixer's working notes.

The measurements are the fixer's, not re-run by the coordinator.

## Owner's ruling (2026-10-20)

Design 1: wide boundaries of up to 256 working bits are stored as their
digits, wider ones move whole, so storage is proportional to each
boundary's width. The `seam_plunge` touch ceiling and its band are
re-derived from the copy's mechanism.
