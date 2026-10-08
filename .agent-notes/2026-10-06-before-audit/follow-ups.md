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
- **One notation for the n-ary folds' cost.** The documentation branch
  restates the receiverless islands' contract as `O(n log k)` with `n` the
  total input bytes; sibling contracts (`version_meet_all`,
  `version_span_all`, `span_join_all`, `span_meet_all`, `clock_recv_all`)
  keep `(|self| + |iter|) log k`, and the changed sections' space lines stay
  `O(|self| + |iter|)`. Unify toward `n` in the JSON `contract` fields, the
  `before-fuelscape/src/ops.rs` roster, and the `not(doc)` fallbacks. The
  `clock_join_all`/`clock_sync_all`/`party_join_all` restatement needs the
  `log |self|` term's derivation first (identity lane O7). No check compares
  the roster with the JSON, so a regeneration can revert hand edits.
  Source: the documentation branch's reviewer.
- **Two pre-existing doc imprecisions.** `O(n log k)` degenerates at
  `k = 1` (identity lane O7's `max(1, log k)` point), and `recv_all`'s "less
  efficient ... in the worst case" implies iterated `recv` is
  interchangeable with it, but iterated `recv` records `k` events, not one.
  Source: the documentation branch's reviewer.
- **A late-first-split COMB family.** `from_interleaved` keeps the writer
  and split streams alive together; a family whose first split comes late
  is inferred to read about 4 B/B against COMB's ceiling of 3. If built and
  confirmed, it is a defect finding, not an instrument, and it bears on
  question 65's choice of COMB's rule. Source: the board-ceiling reviewer;
  the instrument survey, section 2.2.
- **The verification map.** The validation index lacks entries for the
  worst-case ranking pin, the generator census floors, the stack-safety
  tests at depth, the `PeakAlloc` heap checks outside the board, the
  compile-time trait assertions and the fuelscape islands test, and every
  suanpan instrument; its last section points at a scaffolding index in
  `testing.rs`'s module doc that does not exist. `before`'s `## Testing`
  section omits the resource and 32-bit instruments; suanpan has none.
  For the second docs pass. Source: the instrument survey, section 3.
- **Three cost-only survivors no instrument sees.** `packed_u64.rs:63` and
  the two `words.rs:78` survivors pass every meter and the fuel bands
  (verified by the surveyor). Catching them needs a fuel-ladder family
  whose fuel those pops dominate; nobody has constructed one, and the cost
  is a constant factor. Source: the instrument survey, section 1.9.
- **Catching a narrowed cursor position on 32-bit.** No wasm32 pin opens a
  reader partway into a stream at a position of `2^32` or more, so narrowing
  a position before it becomes a byte index (`bits/reader.rs:148`) passes
  every pin. `Version::partial_cmp` cannot catch it, since it opens each
  operand at bit 0. A check over an operation that opens mid-stream (`tick`
  raising a leaf that starts past bit `2^32`, or a join copying such a
  subtree) would. The gamma window's narrowing (`gamma/window.rs:28`) needs
  `borsh` in the guest and a borsh decode check. Both are new `Check`
  variants, so after #58. Source: the compare-pin proposal branch (slot 42).
- **Two more wasm32 pin docs overclaim:** the join output pin and the
  rank-arithmetic route (the latter rewritten by #63). Source: the adequacy
  lane's wasm32-pins calibration, confirmed by the slot 42 builder.
