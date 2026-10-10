I agree about the paper's notation. Its event trees store each height as a base plus offsets down the path, so reading the height at any id means adding along a branch, and position is only implied by nesting depth. Neither ownership nor causality is visible without doing arithmetic in your head. I've left it out below.

**What I checked in the tree:**
- `Party` and `Version` print as their raw bit stream in `Debug` (`Version(0b…)`). Their `Display` is canonical hex, and `FromStr` parses it back.
- `Clock`, `Span`, and `Query` build their `Debug` output from those, so they show the bit stream too.
- `shape` already provides `Plateaus`, `Regions`, `Overlay` and `combine`, but nothing renders them.
- `error::Overlap` is a unit struct, so it says nothing about where two parties overlap.
- The only `Debug` text a test expects is three strings in `causally/tests.rs`. No rumors `.snap` file contains `Debug` output.

The prior art in §5 comes from memory. I haven't checked it on the web.

## 1. What a value can tell a human, and what it can't

A version carries three kinds of information, and only the first is in the value itself:

- **Geometry:** the step function, meaning which heights sit over which ids. It is exact and complete in the value.
- **Attribution:** who did those events. ITC throws this away on purpose. When alice retires into bob, her region and its heights become bob's. A lone version can never say "alice's events". At most it can say "events recorded in the region alice owns now".
- **Relation:** whether it is ordered or concurrent with other versions. That needs the other versions.

So positions in `[0, 1)` are arbitrary labels: nothing about "0.3" means anything. Two things do mean something:
- **Measures:** how much of the id space a region covers, and the area under the function, which is the rank.
- **Cross-reference:** the same region showing up in a party and in a version.

A good display uses coordinates as keys (a name, a column, a color) and spends the reader's attention on measures and relations. A linear x-axis is actively misleading here. A region of width 2⁻²⁰ is invisible on it, yet it can be the only evidence that two versions are concurrent.

## 2. Exact notations from the value alone

**Listing maximal runs loses nothing.** Canonical form keeps a tree node only where the function is non-constant on that node's interval (normalization collapses `(n, m, m)`, `(0, 0)` and `(1, 1)`). So the function alone determines the tree. That means you can merge adjacent equal plateaus, which the dyadic partition keeps apart, without losing anything. Every endpoint is dyadic, so it has a finite exact spelling. A round-trip proptest, `parse(fmt(v)) == v`, would turn this argument into a tested fact.

**Running example:** alice owns `[0, ½)`, bob `[½, ¾)`, carol `[¾, 1)`.
- In version `a`, alice's region is at 3, bob's at 2 and carol's at 1.
- In version `b`, part of bob's region is at 1 and carol's is at 2.

A compact `{:?}`, read left to right as "3 until 0.1, then 2 until 0.11, then 1":

```
Version(3 @0.1 2 @0.11 1)                rank 10.01
Version(3 @0.1 1 @0.101 2)               rank 10.011
Party[0, 0.1)                            owns 50%, 1 run
Party[0.001, 0.01) ∪ [0.1101, 0.111)     owns 18.75%, 2 runs
Clock { party: [0, 0.1), version: 3 @0.1 2 @0.11 1 }
```

The breakpoint marker is `@` rather than `|` because `|` already means join. `{:#?}` could print the same runs as a multi-line table with a width column.

**Binary or decimal coordinates:** both are exact, because `k/2^d` has exactly `d` digits after the point in either base.
- Binary shows structure: the digits name which halves a region sits in, siblings share a prefix, and it matches `Rank`'s binary `Display`.
- Decimal shows size: `0.625` is easier to judge than `0.101`.

I'd use binary for positions and decimal or percent for measures. "Owns 18.75%" is an exact statement. Very deep positions can shorten repeated digits, e.g. `0.0{19}1`.

**One-line summaries** lose information, so they belong in a separately named method:
- For a version: rank, height range, number of runs, depth and byte size.
- For a party: share of the id space, number of runs and depth. The number of runs and the depth are exactly what grows under churn.

`Rank`'s `Display` already honours precision (`{:.N}`). `Version`'s `Debug` could do the same with `{:.N?}`: print the first N runs, then `… (k more runs)`. Truncation would then be something the caller asks for and the output announces.

## 3. Views that take extra context

**3a. A legend of names for live parties.** I think this is the most valuable single piece. A `Legend` maps names to parties, supplied by test fixtures (`nth_party(i)` becomes `p{i}`), by rumors peers, or by a simulation. It gives two diagnostics for free:
- If two named parties overlap, identity linearity was violated. That diagnosis is definitive.
- If the named parties cover less than 100% of the id space, either a live party is missing from the legend or identity leaked through a dropped `Party`. In a test that knows every peer, full coverage is itself an assertion.

It also lets a version print as a pseudo version vector. Each column is the projection `v / party`, which the API already has:

```
          alice   bob      carol
share     50%     25%      25%
a         3       2        1
b         3       1..=2    2
```

`1..=2` marks a region with more than one height; expanding it shows the exact runs, so nothing is lost. The column header should mean "heights in the region X owns now", never "X's events". Internally, `advance_refinement` already steps through party and version walks together through `Refine`. A public `combine` that mixes parties and versions would be an API addition.

**3b. A second version: a causal diff.** This is where better output pays off most, in assertion failures. When `a <= b` fails:

```
a ≰ b and b ≰ a: concurrent
  ids                owner   a  b
  [0,     0.1)       alice   3  3
  [0.1,   0.101)     bob     2  1   a ahead
  [0.101, 0.11)      bob     2  2
  [0.11,  1)         carol   1  2   b ahead
  a.lag(b) = 0.01    b.lag(a) = 0.001
```

The two "ahead" rows are the smallest complete evidence of concurrency. By default the diff should show only the rows that differ. Without a legend, the owner column just disappears. This is `pretty_assertions` applied to causality, and `Crossed` from `Span::new` could carry the same rows.

**3c. A baseline.** Heights grow without limit, and the interesting part is usually a difference. Possible references:
- the meet of all known versions ("everyone knows at least this")
- a span's `lo`
- a peer's previous version

Versions shown relative to the meet never go negative.

**3d. A set of versions: the partial order.** Examples are the message versions in a rumors `Snapshot`, or every peer's current version. You'd draw the covering edges only, like `git log --graph`:
- Nodes are labelled with a short hash of the canonical bytes, as git abbreviates commit IDs.
- Edges are labelled with their delta, named through the legend. If `a < b` differ only inside bob's current region, then the events between them happened in that region, so "bob +1" is a fair label for that edge.
- The maximal elements are the concurrent heads, like Automerge's `heads`.

```
* 7c1e  carol +1
| * 2a90  bob +1     (concurrent with 7c1e)
|/
* f03b  alice +3
```

**3e. Peers against regions: a knowledge matrix.** Project N peers' versions onto N peers' regions and you get something like a matrix clock:
- The diagonal is each peer's own region.
- A column's minimum is what everyone knows in that region.
- A row's lag to the join of all versions says how far behind that peer is.

It answers "who is behind on whom" directly.

**3f. A trace: a shadow version vector.** Only history gives true attribution. A test or simulation wrapper would record each fork, tick and join under stable names, and keep a classic version vector of name→count alongside the ITC. Printed as a run diagram, this is the paper's own §5.1 figure, made live.

I'd justify this as a display aid only. The crate's existing oracles already check the ordering semantics, so a new oracle would need to name a failure they miss.

**3g. Time series.** `lag` and `distance` already measure convergence in rank units. Each peer's lag to the network-wide join, plotted over time as a sparkline, would make a tokio-console-style dashboard for rumors.

## 4. Visual media

- **Waveform viewers** such as GTKWave and Surfer (which is written in Rust) are built to show many aligned step functions, with values printed inside segments, plus zoom and cursors. You'd put the id space on the x-axis instead of time and export VCD, with each version or party as one signal. That gives a real visualizer with no viewer code to write. The limit is that VCD times are integers, so the scale is 2^maxdepth. Tools with 64-bit timestamps fail beyond roughly depth 63.
- **Genome browsers** such as IGV solve the same problem: one coordinate axis spanning nine orders of magnitude, many aligned tracks, summaries when zoomed out and exact features when zoomed in. The rule to copy for summaries is min and max per pixel bucket, so a counterexample region can never vanish.
- **Icicle or flame graphs** of the dyadic tree, zoomable in the style of inferno or speedscope. Width shows share, rows show depth, color shows height, and a band shows ownership.
- **Terminals** should use an ordinal axis: one equal-width column per refined cell, with its true width written in. Never use a linear axis without a guaranteed minimum width.
- **A wasm playground in rustdoc.** The crate already ships interactive HTML in its docs (the fuelscapes). Forking, ticking and joining named clocks while watching the stamps change would teach ITC better than any static format.
- **Smaller options:** DOT export for the partial order and for party lineage, and `#[debugger_visualizer]` scripts that decode the bit stream inside a debugger.

## 5. Prior art and how it would adapt

| Prior art | Adaptation |
|---|---|
| Dynamo and Riak vector clocks; Yjs state vectors `{client: clock}` | Legend projection table (3a), worded as regions rather than participants |
| ShiViz with ShiVector/GoVector (logs carrying vector clocks become space-time diagrams) | Logs already carry hex versions, so a tool can compute `<=` directly. Exporting legend projections as vector clocks would let ShiViz itself run unchanged, at a loss of detail. |
| Lamport space-time diagrams; the ITC paper's run figure | Trace view (3f) |
| `git log --graph`; Automerge heads | Partial-order view (3d) |
| Matrix clocks | Knowledge matrix (3e) |
| CIDR allocation and route aggregation | Parties as address prefixes: fork is subnetting, join is aggregation. This also supports binary coordinates. |
| `pretty_assertions`, `insta` diffs | Causal diff (3b) |
| Jepsen's Elle | Explaining an anomaly by its smallest counterexample: show only the regions that differ |

## 6. What you might be missing

1. **Attribution isn't in a lone version.** After churn, any label claiming "X's events" from the version alone is wrong.
2. **Every visual needs a guarantee that counterexample regions stay visible,** because a linear axis hides the very regions that decide concurrency.
3. **The legend's overlap and coverage checks come free,** and catch linearity violations and identity leaks.
4. **A readable syntax that also parses would double as test-fixture literals** (`"3 @0.1 2 @0.11 1"`), replacing tick sequences like rumors' `version_for`. Versions copy freely, so this is safe for them. A `Party` literal creates identity out of nothing. That is the same hole `decode` already has, but much easier to reach, so it should stay test-only or carry a `dangerously_` name.
5. **`Debug` is the text people actually see,** in assertion failures and inside `Query`'s `Debug`. Changing it moves the three expected strings in `causally/tests.rs`.
6. **`Overlap` carries no location.** I believe the disjointness check already finds the first overlapping region, though I haven't verified it. Returning that region would make the definitive diagnosis actionable: which stale copy is it? This changes the API.
7. **Keep hex for `Display`.** It's the form in logs that any after-the-fact tool can recover exactly.

## 7. Recommendation

In order of increasing cost:
1. Lossless run-based `Debug` for `Version`, `Party` and `Clock` (`Span` and `Query` inherit it), plus the round-trip proptest.
2. The causal diff helper, with the legend optional, starting in `rumors-testkit`.
3. A `Legend` type with the overlap and coverage checks and the projection table.
4. VCD export as the cheapest real visualizer. A partial-order tool over logged hex can come later.

Decisions for you:
1. **Coordinate spelling:** binary for positions and decimal for measures (my recommendation), or decimal or fractions throughout?
2. **May `Debug` change?** I'd say yes, since nothing outside one test module expects its exact text, but whether `Debug` output counts as stable API is your call.
3. **Where does `Legend` live?** I'd start in `rumors-testkit` and promote it into `before`'s public API only once it has proved useful.
4. **A parseable version-literal syntax:** I'd add it. It needs its own entry point, because `FromStr` already means hex, so it's an API addition.

I can write this up as a page with rendered mockups, or save it to `.agent-notes/`, if either would help.
