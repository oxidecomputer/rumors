# #86 ported: the written-position set on today's code

The port of #86 (the set of written positions replacing `suanpan`'s
zero-range map) onto `main` after #122 and #133, and its review.

- [`report-port.md`](report-port.md): the porter's hand-back.
- [`review-1/REPORT.md`](review-1/REPORT.md): round 1, which found that a
  sign query could allocate a bitset growing with the value's width, with
  the repair (`repair-truncate.patch`) and its probe.

The measurements are the agents', not re-run by the coordinator.
