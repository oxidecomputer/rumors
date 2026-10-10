# Presenting `before`'s values to humans

A living design note: how `Party`, `Version`, `Clock`, `Span`, and their
relations should read in `Debug` output, assertion failures, and visual tools.
It records what Finch has ruled, what remains open, and why. This README
supersedes the other sections wherever they differ:

1. [01-brainstorm.md](01-brainstorm.md): the round-one brainstorm, verbatim.
2. [02-measures.md](02-measures.md): what the numeric measures of versions and
   pairs mean (height, width, rank, `min_ticks`, the lag and distance
   families, bricks and floors), with proofs, probe evidence, and open
   problems.
3. [03-views.md](03-views.md): notation alternatives, the skyline drawing,
   the partial-order and knowledge-matrix views, and prior art.
4. [04-forking-api.md](04-forking-api.md): a tangent: `into_forks(n)` and a
   lazy `forking()` API whose fork count is chosen during iteration.
5. [05-further-observations.md](05-further-observations.md): further notes
   on notation, semantics, API, and process, including the notation's
   superlinear size.
6. [06-carry-forward.md](06-carry-forward.md): what to do with all of it,
   ordered by dependency and risk.
7. [07-signed-versions.md](07-signed-versions.md): saturating subtraction and
   a signed `Diff`, which reorganize 02's pairwise measures, the order, the
   drivers and folds, and the diff display.
8. [probe/](probe/): the throwaway crate behind 02's and 05's evidence.

## The question

**Question (Finch):**

> One difficulty with `before` is that it is tricky to present its various types in a way that is meaningfully comprehensible to humans. `Rank` and `Count` are exceptions because they are scalar values, but `Version`, `Party`, and composite types like `Span`, `Clock`, and `Query` are trickier because, per the @crates/before/reference/itc2008.md fundamental data structure, versions and parties are tree structures (or in our case, a "skyline" representation which is isomorphic). We have the "shape" iterator already implemented which yields the skyline shape of a party or version, but it's not hooked into any textual representation, and it's not clear whether showing the skyline is the best meaningful way to present such a thing to someone during debugging. Can you brainstorm about textual and/or visual ways to present these data types to humans so that they can meaningfully understand the identity-space and causality information, without loss of precision? Don't limit yourself to what could be implemented for Display/Debug (though those would be useful). Think outside the box: what context could be provided to make more meaningful visualization or textual display? What prior art exists in this space for more typical version vectors, and could it be adapted? What else am I missing?

**Follow-up (Finch), sent while the response was being prepared:**

> The text representation in the paper does not seem worth replicating, because imo it is not easy to read it and understand ownership or causality.

## Framing

A version carries three kinds of information, and only the first lives in the
value: *geometry* (which heights over which ids), *attribution* (who caused
them, which ITC discards when id space is recycled), and *relation* (order
against other versions). Positions in `[0, 1)` are arbitrary labels; what
means something is measure (width, area) and cross-reference (the same range
in a party and a version). Displays therefore treat ranges as keys and must
never let a narrow range vanish, since a 2⁻²⁰-wide range can be the only
evidence that two versions are concurrent.

## Decided

### Formatting traits

- **`Display` stays canonical hex.** It is the `FromStr` round trip and the
  `serde` human-readable form (`collect_str`), and to a program's end user a
  version is an opaque token, like a UUID.
- **`Debug` becomes the readable notation below.** It makes no stability
  promise, and it is what assertion failures, `dbg!`, and `tracing`'s `?v`
  show. A probe assertion during this discussion failed with
  `left: Version(0b0000001110111111111111)`, `right: Version(0b1010)`; the
  readable form, `Version { 0..0.00001 => 0, 0.00001..1 => 1 }`, diagnosed it
  at a glance.
- **`{:#?}` spreads the notation over lines like a match statement.** The
  alternate flag propagates through nested derived `Debug`, so it must look
  like ordinary pretty-printing.
- **`fmt::Binary` (`{:b}`) shows the canonical bit stream**, today's `Debug`
  output, for codec work.
- **The multi-line skyline drawing is a named adapter** (working name
  `skyline()`), not `{:#?}`.
- **`Debug` output may be superlinear in the encoded size** (ruled
  acceptable): it prints absolute positions and heights where the encoding
  stores deltas. Its bound is documented like `Display`'s; see
  [05-further-observations.md](05-further-observations.md#notation).

### Spelling numbers

One rule covers every number the crate prints: integer digits are decimal, and
digits after the point are binary.

- Heights are decimal integers: they are event counts, and `Count`'s
  `Display` is decimal.
- Positions and widths are binary fractions, `0.101`; their integer part is
  only ever 0 or 1. Binary shows the structure: each digit picks a half.
- Ranks have a decimal integer part and a binary fraction:
  `Version { 0..1 => 5 }` has rank `5`, and five and three quarters prints
  `5.11`. A reader who mistakes the fraction for decimal still orders ranks
  correctly, because digit strings over `{0, 1}` compare the same way in any
  base.
- This changes `Rank`'s `Display`, `FromStr`, and `serde` human-readable text;
  nothing shipped depends on them. The integer part inherits `Count`'s
  decimal conversion cost ("superlinear but subquadratic"), so `Display`'s
  documented bound changes and its fuelscape panel is regenerated.

### Party

Owned ranges in Rust's half-open range notation, in parentheses, maximal
(adjacent owned regions merge):

```
Party(0.001..0.01, 0.1101..0.111)
Party(0..1)                          // the seed
```

### Version

Match arms over maximal runs, in braces in both modes: adjacent ranges of
equal height merge, so the output is as small as the function allows. It
reads as the version's definition, `match id { … }`.

```
Version { 0..0.1 => 3, 0.1..0.101 => 1, 0.101..1 => 2 }
```

Pretty form, one arm per line:

```
Version {
    0..0.1 => 3,
    0.1..0.101 => 1,
    0.101..1 => 2,
}
```

**Why merging loses nothing.** Canonical form keeps a tree node exactly where
the function is non-constant on that node's interval (normalization collapses
`(n, m, m)`, `(0, 0)`, and `(1, 1)`), so the function determines the tree, and
the maximal runs determine the function. A parser that lives only in tests,
plus a round-trip proptest, commits this as a tested fact; there is no public
parser.

The alternatives weighed against this notation, the skyline drawing, and the
floors notation for the crate docs are in [03-views.md](03-views.md).

### Span

A band: each arm gives the range of heights the span allows over that range of
ids, with a single height where the endpoints agree. A version lies in the
span exactly when, over every arm's range, its heights fall within the arm's
band. Braces follow `Version`, since the arms are the same syntax (an
inference from the `Version` ruling, not a separate one).

```
Span { 0..0.11 => 2..=3, 0.11..1 => 1..=2 }
```

`Span` holds its endpoints by value; see [Spans by value](#spans-by-value).

### Views

A lazy view renders as the value it denotes, save the type name. `v / p`
(an `OwnVersion`) prints exactly as its materialization, the version masked
to zero outside `p`'s region:

```
OwnVersion { 0..0.1 => 3, 0.1..1 => 0 }
```

The view compares equal to that materialization with `==`, so any other
rendering would print two equal values differently. `OwnSpan` follows: the
band of both endpoints masked. `Debug` streams the masked runs from the same
lazy walk the comparisons use, never materializing the projection. The
materialized tree can outgrow its operands multiplicatively, per
`OwnVersion`'s docs, but every run boundary of the masked function is a
boundary of `v` or of `p`, so the runs are no more numerous than the
operands' boundaries together. `OwnVersion` derives `Debug` today, printing
its two fields; this replaces that with a hand-written impl.

### Clock

A clock renders as its version with its own arms annotated, as though by a
one-party legend containing only itself (ruled): an inline `/* own */`
comment on one line, an aligned `// own` comment when pretty-printed.

```
Clock { 0..0.1 => 1, 0.1..1 => 2 /* own */ }
Clock { 0..1 => 0 /* own */ }                  // the seed clock

Clock {
    0..0.1 => 1,
    0.1..1 => 2,    // own
}
```

A clock's first debugging question is "what does it know, and where can it
tick?". Two separate listings would make the reader cross-reference two sets
of binary fractions; co-locating them answers it in one line. Reusing the
legend's annotation slot means one annotation scheme serves both: under a
real legend, the clock's own arms show its name instead of `own`.

- **Lossless.** The version is the arms' heights; the party is the union of
  the annotated ranges.
- **Arms.** Arms split at ownership boundaries as well as height changes,
  and adjacent arms merge only when both height and ownership agree.
  `Clock::shape`'s `Overlay` already walks exactly this refinement.
- **Never unannotated.** A `Party` is never empty, so every clock has at
  least one arm annotated `own`.
- **Spellings weighed and set aside.** Brackets on the height
  (`0.1..1 => [2]`) are compact but mark the height when ownership belongs to
  the ids, and read as an array in Rust-flavored text. Brackets around the
  arm (`[0.1..1 => 2]`) are noisy inside braces. A range-side tag
  (`own @ 0.1..1 => 2`, Rust's binding-pattern syntax) is wordier and starts
  a second annotation scheme.

### Legend

A `Legend` names live parties so that renderings can say whose region a range
lies in.

- Names are any `N: Display`.
- It is built from `(N, &Party)` pairs. Name-first matches `enumerate()`, so
  unnamed parties need no convenience method:
  `parties.iter().enumerate().collect::<Legend<_>>()` numbers them from 0.
- Construction is infallible: a debugging aid must render the broken state it
  is debugging. Overlapping parties and uncovered ranges appear as marked rows.
  Overlap means identity linearity was violated. Incomplete coverage means a
  live party is missing from the legend or identity leaked through a dropped
  `Party`.
- With a legend, a version's arms split at ownership boundaries and each arm
  names its owner in Rust comment syntax (ruled): an inline `/* … */`
  comment on one line, placed before the arm's comma so it attaches to its
  arm, and a trailing `//` comment when pretty-printed, vertically aligned.
  An annotation is commentary on the value, not part of it, and comment
  syntax says so in both modes.

```
Version { 0..0.1 => 3 /* alice */, 0.1..0.101 => 1 /* bob */, 0.101..0.11 => 2 /* bob */, 0.11..1 => 2 /* carol */ }

Version {
    0..0.1 => 3,        // alice
    0.1..0.101 => 1,    // bob
    0.101..0.11 => 2,   // bob
    0.11..1 => 2,       // carol
}
```

Owner labels mean "the region this party owns now", never "this party's
events": after a retirement, the survivor's region carries the retiree's
history.

### Causal diff for assertions

Exported from the library; the legend is optional, and the predominant use has
none. Rows are the common refinement of both versions' runs (plus legend
boundaries when there is a legend):

```
assertion failed: a <= b
a and b are concurrent:
  ids            a  b
  0..0.1         3  3
  0.1..0.101     2  1   a ahead
  0.101..0.11    2  2
  0.11..1        1  2   b ahead
a is ahead by at most 1 over 0.001 of the id space; b by at most 1 over 0.01
```

Agreeing rows collapse to `(k agreeing ranges)` when there are many. The
assertion macros must be named (`assert_le!(a, b)`, `assert_lt!`,
`assert_concurrent!`): a `macro_rules` `$a:expr` may only be followed by `=>`,
`,`, or `;`, so `assert_causal!(a <= b)` cannot parse. The single-version case
of this layout, without owners, is the pretty `Debug` above. `Diff` carries no
lifetime, holding its versions by value as `Span` does; a legend-annotated
diff borrows the legend.

The summary line pairs a vertical and a horizontal measure of each side's
lead; see [02-measures.md](02-measures.md#pairwise-measures). The vertical one,
the largest height difference, does not depend on tree depth. The horizontal
one, the width of the ids where that side is ahead, does: like `lag`'s area,
it shrinks with the regions it covers.

### Spans by value

`Span` holds its versions by value instead of `Cow`, and `Crossed` carries
them back, so a failed construction can render the diff. `Query`, its bounds,
and `Ranked` follow. `Version`'s clone is a refcount increment (storage is
`Bytes::from(Vec<u8>)`; I believe `bytes` makes such a buffer's first clone
allocate a small shared header, unverified). Lifetimes remain wherever a
`Party` is borrowed (`OwnVersion`, `OwnSpan`, `Legend`), since `Party` is not
`Clone`. rumors' tree code returns `Span<'_>` from nodes and caches
`Span<'static>`; the migration there is mechanical. This is a public API
change.

### Dropped or tabled

- The paper's tuple notation: reading a height means summing along a branch,
  and positions are implicit in nesting depth.
- A public parser for the readable notation: in practice no one would write
  it.
- Trace views and shadow version vectors: the library does not preserve that
  information.
- Locating the overlap in `Overlap` or join errors: `Overlap` stays opaque for
  now.
- The stepping visualizer, tabled. An editable pane that accepts only a
  Rust-like subset would confuse users who expect Rust. Record-and-replay
  (a `trace` feature recording each operation's call site and canonical
  bytes, replayed beside the source) runs arbitrary Rust but is not what
  Finch wants either.

## For later triage

Recorded here, not done:

- **`Version::ranked`'s docs steer sorts toward the expensive path.** They
  advise preferring `Ranked` "when you … merely need a causal ordering", but
  each `Ranked` comparison repeats the rank traversal, so a sort pays it on
  every comparison. `Ranked`'s own docs already recommend materialized keys
  for large sorts; the same hazard belongs at `Version::ranked`.
- **`into_forks(n)` and `forking()`** for `Party` and `Clock`; see
  [04-forking-api.md](04-forking-api.md).
- **Laws about measures, none of which a test states today.**
  [02-measures.md](02-measures.md) gives each one's status (proved, probed, or
  conjectured); the probe found no counterexample to any probed law, and none
  is committed.
  - A tick raises a version's global maximum height by at most one (Finch
    confirms), so `ticks(k)` raises it by at most `k`; by at least `k − 1`
    when the party is a single dyadic interval containing a point of maximal
    height, such as the seed; and otherwise by possibly nothing.
  - `max ≤ min_ticks`, `rank ≤ min_ticks`, `width ≤ rank ≤ max × width`.
  - The maximum height preserves joins; the minimum height preserves meets.
  - `rank` and width are modular: `f(a | b) + f(a & b) = f(a) + f(b)`.
  - `min_ticks` equals the number of bricks (the floors' maximal dyadic
    pieces), is submodular, and is not monotone in the causal order.

## Open questions

1. **Lag measures for the diff and beyond.** The candidates `lag_height` (the
   largest height difference, depth-independent) and `lag_width` (the width
   where one side leads, depth-dependent) pair with `lag` (area):
   `width ≤ lag ≤ height × width`. Their symmetric forms are the same measures
   from meet to join. Proposal: implement them privately for the diff, and
   expose `lag_height` when a consumer appears. Names, return types, uses, and
   properties are in [02-measures.md](02-measures.md).
2. **A lower bound on missing ticks.** `max(b) − max(a)` is a valid lower
   bound on the ticks `b` has seen that `a` has not; pairwise height is not.
   Whether the greatest lower bound is computable is open; see
   [02-measures.md](02-measures.md#causal-content).
3. **The legend and assertion APIs** need development before they are
   specified: the `Diff` and `Legend` types, the macros, and how a legend
   attaches to `Debug`-style output.
4. **The partial-order view** (`git log --graph` over a collection of
   versions): type, ordering, parent selection, and legend labels are
   sketched in [03-views.md](03-views.md#the-partial-order-view).
5. **Implementation structure.** `rank` and `min_ticks` are two corners of
   one brick formula, but their kernels should stay separate; the reusable
   split is drivers (one version, or an oriented pair) against folds. See
   [02-measures.md](02-measures.md#computing-the-measures).
6. **The knowledge matrix** (owners' regions against each peer's version),
   and its conjecture that each owner's own cell is its row's maximum: see
   [03-views.md](03-views.md#the-knowledge-matrix).

## Corrections to earlier rounds

- `crates/before-viz` already exists: a gesture-driven wasm visualizer
  (fork, tick, join, send) that draws each clock as a skyline over an
  ownership strip and history as a graph, with undo/redo, URL-fragment
  sharing, and a GitHub Pages deploy. Round one's playground bullet (§4) and
  part of §3d describe it as new. Its glyph uses a linear axis with a width
  cap (`min(maxWidth, max(minWidth, 7px · 2^maxDepth))`), so past the cap
  ranges narrower than 7px stop being legible.
- §3a's pseudo-version-vector table is the single-version case of the diff
  layout, transposed; the legend-annotated `Debug` above replaces it.
- §3g's lag time series measured area, which depends on depth.
- Round two claimed that the width where one version is behind ("extent") is
  depth-independent. It is not: one missed tick on a 2⁻²⁰-wide fragment has
  width 2⁻²⁰. Only the vertical measure is.
- Prior art in round one comes from memory and has not been checked on the
  web.
