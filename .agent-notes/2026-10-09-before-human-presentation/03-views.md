# Notation alternatives and multi-version views

The alternatives weighed against the chosen `Version` notation, the
multi-line drawing, the views over several versions, and background on prior
art. Part of the [presentation note](README.md).

## Version notation: the alternatives

All five spell the same version: height 3 on `0..0.1`, 1 on `0.1..0.101`, 2
on `0.101..1`.

**A. Match arms (chosen for `Debug`).** Reads the skyline column by column,
each arm a range and its height, as the definition `match id { … }`. Rust has
exclusive range patterns, so the arms look like valid syntax, and the ranges
share `Party`'s notation.

```
Version { 0..0.1 => 3, 0.1..0.101 => 1, 0.101..1 => 2 }
```

**B. Walls (rejected).** `Version(3 |0.1| 1 |0.101| 2)`, a variant of an
earlier `3 @0.1 1 @0.101 2`. Heights and positions alternate with nothing in
the syntax to tell them apart, and `|` already means join.

**C. Base plus bumps.** The minimum is factored out, and unlisted ranges sit
at the base. It loses nothing, and it helps when every height is near a
million.

```
Version(1 + (0..0.1 => 2, 0.101..1 => 1))
```

**D. Floors (for the crate docs).** Reads the skyline row by row, top floor
first. Each line is a floor `{v ≥ k}`, written exactly like a `Party`. It
teaches the order: `v <= w` exactly when every floor of `v` lies inside the
same floor of `w`. [02-measures.md](02-measures.md) builds on the same floors.

```
≥3  0..0.1
≥2  0..0.1, 0.101..1
≥1  0..1
```

**E. The drawing (the `skyline()` adapter).** Both readings at once. Both axes
are ordinal: each run gets a column as wide as its label, so a 2⁻⁴⁰-wide run
is as visible as a wide one, and rows exist only for heights that occur, so
heights near a million do not make the drawing tall. Every breakpoint and
height is labeled, so it loses nothing. A `Clock` adds an ownership strip
beneath the axis.

```
3 ┌─────────┐
2 │         │         ┌───────────┐
1 │         └─────────┘           │
  0         0.1       0.101       1
```

## The partial-order view

A `git log --graph` rendering of a collection of versions, newest first.
Edges are the covering relation, each labeled with the delta from its parent:

```
*   v4  { 0..0.11 => 3, 0.11..1 => 2 }                     merge of v2, v3
|\
| * v3  { 0..0.1 => 3, 0.1..0.11 => 2, 0.11..1 => 2 }      +1 on 0.11..1
* | v2  { 0..0.11 => 3, 0.11..1 => 1 }                     +1 on 0.1..0.11
|/
* v1  { 0..0.1 => 3, 0.1..0.11 => 2, 0.11..1 => 1 }
```

- **Type.** A newtype over a collection `C` where
  `for<'a> &'a C: IntoIterator<Item = &'a T>` and `T: Borrow<Version>`,
  optionally paired with labels. Formatting iterates `&C` once per pass, so
  nothing is copied.
- **Order.** Rank strictly increases with causal order, so sorting by rank
  gives a deterministic topological order. Compute each rank once
  (`sort_by_cached_key`): comparing `Ranked` views repeats the rank traversal
  on every comparison.
- **Parents.** Walking earlier versions from highest rank down, a version is a
  parent if it lies below the node and below no parent already chosen. That
  is O(n³) comparisons at worst, fine for debugging.
- **With a legend.** Edges read `+1 bob`, and an edge spanning several owners
  lists each one (`+1 bob, +2 carol`), which happens whenever the collection
  omits intermediate versions. Merges read `merge of v2, v3`, or
  `merge of v2, v3, +1 carol` when the merge sits strictly above the join of
  its parents. When the collection is rumors' message versions, an edge says
  whose region the message was written in.

## The knowledge matrix

Rows are owners' regions, columns are each peer's current version, and
brackets mark a peer's view of its own region:

```
                      held by
region                alice  bob  carol    meet  join
alice  0..0.1         [3]    2    3        2     3
bob    0.1..0.11      2      [3]  2        2     3
carol  0.11..1        1      1    [2]      1     2
```

`meet` is the minimum in each row, what every peer knows about that region,
and `join` the maximum, the newest anyone has seen. A region whose height
varies within some version splits into sub-rows.

Conjecture, to check before relying on it: if every tick goes through its
owner's `Clock`, only the owner raises its own region, so each bracketed cell
is its row's maximum, and a row where someone exceeds the owner points at a
linearity violation. `Version::tick` accepts any party and any version, so the
premise is a usage discipline, not something the types enforce.

## One table renderer

The single-version annotated listing, the diff, and the matrix are one
layout. Rows are the common refinement of the versions' runs, plus legend
boundaries when there is a legend. The owner column is optional, and there is
one column per version: one column is the pretty `Debug`, two the diff, and
`N` the matrix.

## Prior art and background

**CIDR allocation and route aggregation.** An IPv4 address is a 32-bit
string, and a CIDR block such as `10.0.0.0/8` names every address whose first
8 bits match: a dyadic interval of width 2⁻⁸. Splitting a `/16` into two
`/17`s is ITC's fork. A router holding routes for both halves of a block via
the same next hop replaces them with one route for the whole block, which is
normalization's `(1, 1) → 1` and what a join does when it reunites siblings.
A flattened routing table is a step function over the address space with
dyadic breakpoints, the structure of a version with next hops in place of
heights. A binary-point position such as `0.101` is the prefix `101/3`, and
"range to CIDR" tools convert between ranges and prefix lists, as between
runs and canonical plateaus; the probe's piece count in
[02-measures.md](02-measures.md) is that conversion. This is background, not a
proposal: ranges are the chosen notation.

**Other prior art considered in round one**, from memory and not checked on
the web: version-vector printing in Dynamo, Riak, and Yjs; ShiViz with
ShiVector and GoVector; Lamport space-time diagrams; `git log --graph` and
Automerge heads; matrix clocks; `pretty_assertions` and `insta` diffs;
waveform viewers (GTKWave, Surfer); genome browsers (IGV); flame and icicle
graphs; Jepsen's Elle; Python Tutor's record-and-replay.
