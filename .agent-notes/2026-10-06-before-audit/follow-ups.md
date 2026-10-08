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
- **The gamma window's general contract is untested.** `pub(crate)` window
  decoding promises one complete code from the word at `position`, but
  `gamma_window_edge` tests a 63-bit code only at position 0, and three
  value mutants in `window.rs:47` (`load`'s ninth-byte merge) survive. They
  are unreachable today because borsh's stream reader buffers at most 7 bits
  past `position`; a future caller with a fuller buffer would expose them.
  `lanes/l8-adequacy/round-2/addendum-bits-party/witness/window_witness.rs`
  (59- to 63-bit codes at every unaligned start) kills all three and is a
  ready-made regression test. Source: the adequacy lane, round 2 addendum.
- **The borsh gamma window decodes only short codes in production.** For
  the same reason, the window in `borsh_impls.rs:129` saves at most 7 bit
  reads per code. Its doc is accurate but omits that ceiling; a candidate
  simplification, not a defect. Source: the adequacy lane, round 2 addendum.
- **`Count`'s borsh truncation is untested and reports `InvalidData`.** Its
  integer reads go through borsh's `unexpected_eof_to_unexpected_length_of_input`,
  so a truncated `Count` reports `InvalidData`, not `UnexpectedEof`, and its
  trailing-zero rejection carries a plain message rather than a `Decode`.
  Source: the documentation branch builder (read in borsh 1.6.1's source;
  unverified by a run). Pairs with the deferred borsh error-mapping prose
  (`builder-docs-branch/deferred-borsh-errors.patch`, deferred behind
  `fix/before-wasm32-buffer-growth`).
- **Two undocumented contracts.** The encoders' partial-write behavior
  (codecs lane obs 5) and the derivation of the `log |self|` term in
  `join_all`'s complexity (identity lane O7). Not in the docs brief.
