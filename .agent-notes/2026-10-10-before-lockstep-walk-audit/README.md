# Lockstep walks in `before`: an audit

`before` often walks several dyadic partitions together: two versions for a
comparison, a version and a party for a projection, a probe and many bounds
for a query. This note inventories those walks, identifies which mechanisms
duplicate one another, and recommends what to consolidate. The question came
from the [presentation note](../2026-10-09-before-human-presentation/06-carry-forward.md)
(section 6), which proposes separating leaf-stream drivers from measure
folds.

**Scope and evidence.** The code read is `main`'s `crates/before/src`,
identical to
[`4bcf274`](https://github.com/oxidecomputer/rumors/tree/4bcf274add9d959ddc9239267211960e3b36bc7d/crates/before/src),
which the links below use. `fix/suanpan-uniform-amortization` and
`fix/min-ticks-transient-heap` both edit walker files (`overlay.rs`,
`lattice.rs`, `order.rs`, `place.rs`, `filter.rs`, `projection.rs`,
`io/regions.rs`), but a search of their diffs finds no changed line touching
cursor structure (`CursorSet`, `advance`, `priority`, `depth`, `step`,
`skip_deeper`, `peek_flip`, `RegionReader`); their edits are accumulator
arithmetic. Everything here comes from reading. No test was run and nothing
was measured, so every consolidation claim below is a hypothesis for the
fuelscape boards and test suite to confirm.

## Summary

1. **One law, three implementations.** The rule for advancing several
   partitions to their next shared boundary is implemented three times:
   `overlay::advance` (two cursors), `overlay::advance_set` (any number of
   slots behind a `CursorSet`), and `version::shape::advance_refinement` (a
   slice of `Refine` walks).
2. **Three cursor traits.** `RegionReader`, `Refine`, and `CursorSet` all
   describe a cursor that has a depth and steps to a flip level.
3. **`Refine` and `advance_refinement` are dissolvable.** The shape
   iterators' needs are a subset of what `RegionReader` and `advance_set`
   already provide. This is the main recommendation, to construct and
   measure.
4. **`advance` earns its separate existence** as a two-cursor specialization
   that hands crossings back to its callers. It should be stated as an
   instance of the same law, and checked against it by a committed
   differential test.
5. **Party-with-party operations use two paradigms.** `join`, `sync`,
   `is_disjoint`, and `covers` descend both trees together; `without` walks
   region boundaries through `overlay::advance`. Whether that split is
   justified is an open question.
6. **Smaller items:** projection holds two copies of one subtree skip; the
   law is restated in many module docs, and one restatement is inaccurate
   crate-wide; and the drivers-and-folds plan needs `advance` to return the
   flip level.

## The law

A version partitions `[0, 1)` into constant-height regions and a party into
constant-ownership regions, each a dyadic interval. At any moment every
cursor's current region contains the same point, so the regions nest. The
deepest region ends first. After it steps, its *flip level* (the depth of the
lowest common ancestor of the region it left and the region it entered)
identifies every other region ending at the same boundary: exactly those at
least as deep as the flip level. They step too. A cursor on its final region
is never the deepest while another cursor is unfinished, and never reaches
the flip level, because its region runs to `1` and contains the boundary in
its interior. ([`overlay.rs`'s module docs](https://github.com/oxidecomputer/rumors/blob/4bcf274add9d959ddc9239267211960e3b36bc7d/crates/before/src/version/overlay.rs#L1-L20)
state it.)

## Inventory

| Family | Mechanism | Consumers |
|---|---|---|
| region overlay, two cursors | [`overlay::advance`](https://github.com/oxidecomputer/rumors/blob/4bcf274add9d959ddc9239267211960e3b36bc7d/crates/before/src/version/overlay.rs#L47-L93) over two `RegionReader`s, returning each side's crossing | through [`advance_diff`](https://github.com/oxidecomputer/rumors/blob/4bcf274add9d959ddc9239267211960e3b36bc7d/crates/before/src/version/overlay.rs#L211-L223) and `OpenedPair`: order comparison (`order.rs`), join and meet emission (`lattice.rs`), the pair integrals (`measure/integral/pair.rs`); directly: projection materialization ([`projection.rs`](https://github.com/oxidecomputer/rumors/blob/4bcf274add9d959ddc9239267211960e3b36bc7d/crates/before/src/version/projection.rs#L135)), party difference ([`without/difference.rs`](https://github.com/oxidecomputer/rumors/blob/4bcf274add9d959ddc9239267211960e3b36bc7d/crates/before/src/party/without/difference.rs#L66-L67)) |
| region overlay, any number of slots | [`overlay::advance_set`](https://github.com/oxidecomputer/rumors/blob/4bcf274add9d959ddc9239267211960e3b36bc7d/crates/before/src/version/overlay.rs#L136-L154) over a [`CursorSet`](https://github.com/oxidecomputer/rumors/blob/4bcf274add9d959ddc9239267211960e3b36bc7d/crates/before/src/version/overlay.rs#L106-L123), whose `step` folds each crossing into the set's own state | span placement (`place.rs`, three slots, endpoints droppable), lazy projected comparison (`projection.rs`, four slots, parties optional), the query filter (`place/filter.rs`, `MemberCursors` and `SpanCursors`, one slot per bound) |
| region overlay, shape walks | [`advance_refinement`](https://github.com/oxidecomputer/rumors/blob/4bcf274add9d959ddc9239267211960e3b36bc7d/crates/before/src/version/shape.rs#L166-L189) over [`Refine`](https://github.com/oxidecomputer/rumors/blob/4bcf274add9d959ddc9239267211960e3b36bc7d/crates/before/src/version/shape.rs#L94-L108) walks (`VersionWalk`, `PartyWalk`) | `shape::combine` (`N` versions), `Clock::shape`'s `Overlay` (a version and a party, as `&mut dyn Refine`) |
| tree co-descent | `PartyReader` node and subtree walks that skip a whole subtree when one side decides the pair | party `join`, `sync`, `is_disjoint`, `covers` (`party/compare.rs`, `join.rs`, `sync.rs`) |
| tree co-descent, version with party | `VersionTreeReader`, `VersionSubtreeReader`, `PartyReader`, `RangeMinima` | `tick` (`version/tick/*`) |
| single stream | `VersionRegionReader`, `PartyRegionReader`, or `PartyReader` alone | `rank`, `min_ticks`, `max_depth`, `shape::Plateaus` and `Regions`, balanced forks (`party/forks.rs`) |

Test support does not reuse the production walkers: `testing/shape_rows.rs`
folds the public item streams and compares against the recursive oracle's
own enumerations, so the oracles stay independent.

## Findings

### 1. The three implementations agree, and two of them are general

All three pick the first deepest unfinished cursor, step it, and then step
every other cursor whose depth reaches the flip level. They differ in four
ways:

| | `advance` | `advance_set` | `advance_refinement` |
|---|---|---|---|
| arity | two, heterogeneous types | any; slots by index | any; a homogeneous slice, or `&mut dyn Refine` for mixed kinds |
| crossings | returned, and passed to a `fold` callback in order | folded inside `CursorSet::step` | stored inside the walk as a pending rise |
| finished cursors | caller guarantees one is unfinished | an absent or dropped slot reads depth 0 and never steps; a present finished cursor is excluded by the nesting argument | skipped by explicit `done()` checks |
| termination | caller checks | caller checks | returns `true` when all are finished |

The explicit `done()` checks in `advance_refinement` are redundant whenever
some walk is unfinished, by the argument in [The law](#the-law). Its only
behavior beyond `advance_set` is the all-finished return value, which a
caller computes in one line. So `advance_refinement` is `advance_set` with a
slice adapter.

### 2. `Refine` restates `RegionReader`

`RegionReader` has `depth`, `done`, and `step`, which returns the flip level
and the crossing. `Refine` has `depth`, `done`, and `advance`, which returns
the flip level and stores the crossing. Both readers already implement
`RegionReader`: `VersionRegionReader` with height changes as crossings,
`PartyRegionReader` (in `overlay.rs`) with `()`. `PartyWalk` is a shim, in its
own docs' words "a thin visibility shim", over a reader that already
implements `RegionReader`. `VersionWalk` adds one thing, the pending rise
that only the first refined fragment of a plateau reports. A `CursorSet` over
a slice of `VersionRegionReader`s plus one pending-crossing slot per input
holds the same state.

### 3. `advance` is justified, but should be stated as an instance

Its callers use the returned crossings after the step. The lattice sweep
re-codes them into output deltas; the pair integrals re-fold them under an
orientation; projection materialization reads the version's step to compute
the output change at ownership boundaries; party difference passes them
through. Moving each of those algebras inside a `CursorSet::step` would be a
real restructuring of every consumer, and the two-cursor match avoids the
priority iteration on the comparison and lattice paths, which I judge the
hottest (not measured). Keeping it is reasonable. Two independent implementations of one
law without a committed check that they agree is the gap to close.

### 4. Party-with-party operations use two paradigms

`join`, `sync`, `is_disjoint`, and `covers` descend both party trees
together through `PartyReader` nodes, skipping the other side's subtree when
an owned or unowned region decides the pair. `without` instead builds a
`DifferenceReader` that implements `RegionReader`
([`without/reader.rs`](https://github.com/oxidecomputer/rumors/blob/4bcf274add9d959ddc9239267211960e3b36bc7d/crates/before/src/party/without/reader.rs#L145))
and walks region boundaries through `overlay::advance`, with its own subtree
dispositions. Both paradigms are linear. Tree co-descent fits operations that
emit structure (`join` copies subtrees) or decide early (`is_disjoint`).
Whether `without` should join them, or the others could share its machinery,
needs a closer reading of `join.rs`, `sync.rs`, and `without/` than this
audit made. It is recorded as an open question, not a finding.

### 5. Projection skips unowned subtrees twice

Inside an unowned party region every projected height is zero, so both
projection walks fold a version's deeper regions in bulk with `peek_flip` and
`skip_deeper`: once in [materialization](https://github.com/oxidecomputer/rumors/blob/4bcf274add9d959ddc9239267211960e3b36bc7d/crates/before/src/version/projection.rs#L110-L130),
emitting one unchanged output region per skipped subtree, and once in the
comparison's [`block_skip`](https://github.com/oxidecomputer/rumors/blob/4bcf274add9d959ddc9239267211960e3b36bc7d/crates/before/src/version/projection.rs#L370-L398),
folding the net change into the difference and height accumulators. The
consumers differ, so the shared part is small, perhaps a helper that yields
each skipped subtree's flip level and net change. Minor.

### 6. Documentation

- The law is restated in the module docs of `overlay.rs`, `version/shape.rs`,
  `order.rs`, `lattice.rs`, `place.rs`, and `projection.rs`, and in the docs
  of all three functions. One canonical statement, in `overlay.rs`, with
  links from the rest, would match the crate's rule that a second
  implementation of a scheme points back at the first by name.
- `Refine`'s docs say the blanket `&mut` impl lets heterogeneous walks enter
  `advance_refinement` "so the law has one implementation for the
  homogeneous combiner and the version × party overlay alike". That is true
  within the shape module. The crate as a whole has three.

### 7. What the drivers-and-folds plan needs

The presentation note's plan feeds a pair driver into folds such as
`min_ticks`'s, which open and close subtrees from turn depths. `advance`
computes the flip level of every boundary for its tie test, then discards it.
Returning it costs nothing and supplies the turn depth the plan needs. More
broadly, the overlay layer already is the driver layer for comparisons, the
lattice, and the pair integrals; `CursorSet` is the many-input form with the
fold inside `step`. The plan extends this layer; it does not need a new one.

## Recommendations

Ordered so that fixed-sign, low-risk steps come first.

1. **Commit a differential test of the law.** On random inputs, the boundary
   sequence and each round's set of stepped cursors from `advance` must match
   `advance_set`'s over the same two cursors, and `advance_refinement`'s
   while it exists. It catches drift between implementations regardless of
   what is consolidated, and the cheapest artifact that passes it is a
   correct implementation of the law.
2. **Dissolve `Refine` and `advance_refinement`.** Express the shape
   iterators as `CursorSet`s over `RegionReader`s, with pending rises held in
   the set. Construct it, then measure: the `version_shape`, `party_shape`,
   `clock_shape`, and `shape_combine` fuel panels will move, and the
   criterion is "identical or improved, every regression a finding". The
   deletion removes one trait, one shim, and one implementation of the law,
   and `Clock::shape`'s `&mut dyn Refine` dispatch along with them.
3. **Return the flip level from `advance`.** It is a small API change inside
   the crate, needed by the drivers-and-folds plan.
4. **Make `overlay.rs` the law's single statement**, with the other docs
   linking to it, and correct the "one implementation" sentence if `Refine`
   survives.
5. **Open questions:** whether `without` and the tree co-descent operations
   should share one paradigm, and whether projection's two subtree skips
   should share a helper.

The `tick` walk is out of scope. It is a tree walk shaped by the paper's
recursive `fill` and `grow`, with its own prescan and `RangeMinima`
machinery, not a lockstep walk over region boundaries.
