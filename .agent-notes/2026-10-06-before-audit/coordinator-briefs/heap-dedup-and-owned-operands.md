<!-- CAVEAT LECTOR: a simplification brief written by the coordinator (Claude Opus 5.5) from the entry-delegation builder's notes (session scratchpad `builder-entry-delegation/NOTES.md`). It stacks on `simplify/lattice-entry-delegation` (ready #36). -->

# Simplification brief: duplicate filters without clones, and moved owned operands

Kind: two fixed-sign heap improvements, deleting redundant work. No public
API, format, or value changes. The criterion is identical values, every
reading identical or improved, each improvement measured, and each
regression a finding.

## 1. Duplicate filters that clone only to stay alive

Two adapters drop adjacent inputs that share storage, comparing buffer
identity (`ptr_eq`):

- `DedupRuns` (`crates/before/src/version.rs`, on #36's tip near line 1152),
  used by `Version`'s `join_all`, `meet_all`, and `Sum`-style folds.
- The filter inside `Span::fold_endpoints`
  (`crates/before/src/span/algebra.rs`, near line 388), used by the
  `span_*_all` operations.

Each keeps a *clone* of the last yielded item's version, or endpoints, so
that the previous buffer stays alive and no freed allocation can be reused at
the same address and pass as a duplicate. The clone is redundant work, and
it costs heap. Cloning a `Bytes`-backed version whose length equals its
capacity promotes it, allocating a 24-byte shared header that lives with the
buffer, permanently and on the caller's value. #36 made this visible: the
board's `span_*_all` heap readings rose in multiples of 24 bytes, because
`fold_endpoints`' clone now promotes inside the measured call.

**Proposed structure.** Hold the pending item one step behind instead.
- Keep the previous *item itself*, owned by the adapter, and yield it only
  once its successor has been seen to differ. A duplicate successor is
  dropped.
- The held item keeps its buffer alive with no clone.
- At the end, yield the held item.

Check that the borrowed-item case (`I::Item: Borrow<…>`) keeps the same
guarantee: a borrowed item's referent outlives the iteration by
construction. State the invariant at the adapter: what keeps a compared
buffer alive.

## 2. Owned right operands are cloned, not moved

The lattice ladder's `Outcome::Right` arms (`version.rs`, near lines 1524
and 1535 on #36's tip) return or assign `other.clone()`. When the caller
passed the right operand by value (`Version | Version`, `Version & Version`,
`|=`/`&=` with an owned right side), the clone is redundant: the owned value
can be moved. Route owned right operands so that `Right` moves them.

`tests/meter/lattice_clones.rs` pins clone counts and heap exactly, so its
pin must *lower* where moves land. Restate its doc to match.

## What to verify

- **Values.** These are unchanged. The committed lattice and span laws cover
  them.
- **Calibration.** By reversible swap, show that a filter comparing against
  a dropped buffer would be caught:
  - make the adapter hold a reference to a freed item, or compare against a
    stale address;
  - make `Right` clone where it should move.

  Committed tests must fail each. If nothing catches the first, add a test
  that constructs address reuse deliberately.
- **Board.** Every moved reading must be stated parent to new. The
  `span_*_all` heap rises #36 introduced should fall back. Re-pin any worst
  case whose flip follows from the improvement, with the movement in the
  commit message. Readings appear only in commit messages, never in tree
  prose.
- **Landing check.** One run at the tip. Expect the post-board-pin baseline
  plus #36's and #30's own counts.
