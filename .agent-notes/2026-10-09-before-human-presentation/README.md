# Presenting `before`'s values to humans

A living design note: how `Party`, `Version`, `Clock`, `Span`, and their
relations should read in `Debug` output, assertion failures, and visual tools.
It records what Finch has ruled, what remains open, and why. The round-one
brainstorm is kept verbatim in [01-brainstorm.md](01-brainstorm.md); this
README supersedes it wherever they differ.

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
  show.
- **`{:#?}` spreads the notation over lines like a match statement.** The
  alternate flag propagates through nested derived `Debug`, so it must look
  like ordinary pretty-printing.
- **`fmt::Binary` (`{:b}`) shows the canonical bit stream**, today's `Debug`
  output, for codec work.
- **The multi-line skyline drawing is a named adapter** (working name
  `skyline()`), not `{:#?}`.

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

### Clock

```
Clock { party: Party(0..0.1), version: Version { 0..0.1 => 3, 0.1..0.11 => 2, 0.11..1 => 1 } }
```

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
  names its owner: a parenthetical on one line, a trailing comment when
  pretty-printed, with the comments vertically aligned.

```
Version { 0..0.1 => 3 (alice), 0.1..0.101 => 1 (bob), 0.101..0.11 => 2 (bob), 0.11..1 => 2 (carol) }

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
lead; see [Vertical and horizontal lag](#open-questions). The vertical one,
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

## Open questions

1. **Vertical and horizontal lag.** Write `g = (b − a)⁺` for the part of `b`
   above `a`; `a.lag(&b)` is its area, `∫ g`. Its *height* is `max g`, an
   integer, and its *width* is the measure of `{g > 0}`, a binary fraction.
   Slicing `g` into floors `{g ≥ 1} ⊇ {g ≥ 2} ⊇ …` gives `lag` as the sum of
   floor widths, width as the ground floor's width, and height as the number
   of floors, so `width ≤ lag ≤ height × width`.
   - All three are zero exactly when `b <= a`, shrink as `a` absorbs history,
     grow as `b` gains it, and obey the triangle inequality.
   - Only height is depth-independent. A depth-independent horizontal measure
     would be a count: how many maximal ranges have `b` above `a`.
   - Height is not a lower bound on missing ticks: `fill` can raise a point by
     many in one tick. Under static membership, where each party owns one
     dyadic interval and only its owner ticks there, height over a party's
     region equals the version-vector difference for that writer.
   - Candidate names: `lag_height` and `lag_width`, beside `lag`. They show
     the pairing and the inequality, and follow `lag`'s direction.
   - Candidate uses: progress measures in convergence tests; the bracketing
     inequality and triangle inequality as property tests; the diff's summary
     line; bounded-staleness predicates (`a.lag_height(&b) <= k`, in the
     spirit of TACT's error bounds and Cosmos DB's bounded staleness); peer
     selection for anti-entropy. The last two are application policies with
     no current consumer in rumors.
   - Proposal: implement privately for the diff; expose `lag_height` when a
     consumer appears. Height is computable today through `shape::combine` by
     summing rises (with the documented quadratic hazard for running `Count`
     sums); width cannot be built exactly as a `Rank` from the public API.
2. **A lower bound on missing ticks.** Is the fewest ticks `b`'s history must
   hold beyond `a`'s computable? It is finite even though the true count is
   unbounded above; from the empty version it is exactly `min_ticks`.
   `min_ticks(a | b) − min_ticks(a)` is not a valid bound, because `a`'s real
   history may hold more than `min_ticks(a)` events. `max(b) − max(a)` (global
   maximum heights) is a valid bound if a tick raises a version's global
   maximum by at most one: `fill` raises points only to heights already
   present, and `grow` adds one. Induction over the execution then gives
   `max(b) ≤ max(a) + |events b has seen that a has not|`. The premise is
   unverified against this crate's tick; a proptest can check it.
3. **The legend and assertion APIs** need development before they are
   specified: the `Diff` and `Legend` types, the macros, and how a legend
   attaches to `Debug`-style output.
4. **The partial-order view** (`git log --graph` over a collection). It is a
   newtype over `C` where `for<'a> &'a C: IntoIterator<Item = &'a T>`,
   `T: Borrow<Version>`, optionally with labels. It sorts by rank computed
   once per version (`sort_by_cached_key`), not by comparing `Ranked` views.
   A version's parents are the earlier versions below it that lie below no
   parent already chosen, found by walking down in rank: O(n³) comparisons at
   worst, fine for debugging. With a legend, edges read `+1 bob`, an edge
   spanning several owners lists each, and merges read `merge of v2, v3` or
   `merge of v2, v3, +1 carol`.
5. **The knowledge matrix.** Rows are owners' regions, columns are each peer's
   current version, and brackets mark a peer's view of its own region; `meet`
   and `join` columns give what everyone knows and the newest anyone has seen.
   Conjecture to check before relying on it: if every tick goes through its
   owner's `Clock`, each bracketed cell is its row's maximum, so a violation
   points at broken linearity.

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
