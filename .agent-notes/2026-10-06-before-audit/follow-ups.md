<!-- CAVEAT LECTOR: kept by Claude (Opus 5.5), the audit coordinator. -->

# Follow-ups after the audit

The owner ruled (question 59, `coordinator-briefs/audit-scope-cutoff.md`)
that new work the remaining audit turns up goes here rather than into the
audit. Each entry names its source and what it would catch.

- **Readout touches over random histories.** The readout-class table
  (`audit/suanpan-readout-class-table`) tests suanpan's readout touch rule
  on constructed rows. No instrument checks the rule over arbitrary digit
  layouts from random update histories. Folding the rule into an existing
  random-history differential suite would sample that space. Source: the
  readout-class builder. Its reviewer found no constructible failure such
  a suite would catch today, once the table runs at an odd width; what it
  would still sample is scaled readouts with a nonzero start and interior
  zero digits jointly with the class. Not recommended until such a failure
  can be named.

- **Bound-digit states reaching other operations.** suanpan's surface
  model never parks digits at the representation's bound, so negation,
  shifts, `add_shifted` with extreme operands, and
  `cmp_zero_stable_under` never see such states. The normalize test covers
  normalization alone. Source: the normalize-bound reviewer.

- **`arb_clock_family` never reaches a late overlap.** It draws at most
  three unrelated parties, which are rarely pairwise disjoint, so the
  arbitrary driver never exercises `sync_all`'s or `join_all`'s rejection
  between groups (0 of 4,000 cases). A pool that includes fork shares of one
  party would reach it. Source: the `sync_all` reviewer. (Overlaps the
  adequacy lane's `machinery-disjoint-families.md` brief.)
- **One `join_all` implementation.** `Party::join_all` still inlines the
  algorithm that `Clock::join_all` now splits into a fold and an absorb; a
  generic helper in `fold.rs` would leave one. Source: the identity lane's
  SB1 brief, optional step, and the `sync_all` reviewer.
- **No variadic rejection row on the board.** `clock_sync_all` and
  `clock_join_all` are metered only on success, so their rejection paths'
  allocations are unmetered. Source: the `sync_all` reviewer.
