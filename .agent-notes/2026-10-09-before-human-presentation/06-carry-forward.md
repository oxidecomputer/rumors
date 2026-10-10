# Carrying it forward

What this discussion suggests doing to the crate, ordered by dependency and
risk. Part of the [presentation note](README.md). Items marked *ruled* follow
Finch's rulings; the rest are proposals for triage. Public API changes are
proposals until Finch accepts them. Each item names its reason and where the
detail lives.

## 1. Documentation, no code risk

- **Hazard at `Version::ranked`.** Its docs steer sorts toward repeating the
  rank traversal on every comparison; repeat `Ranked`'s own advice there.
  (README, For later triage.)
- **`Rank`'s docs.** Explain the number spelling (decimal integer digits,
  binary fraction digits) once the spelling lands, and consider the reading
  "average events seen per point of identity space" beside "area".
  ([05](05-further-observations.md#semantics).)
- **`Party`'s docs.** A dropped party's region can never be raised again, so
  it freezes, and the global minimum height can never pass it.
  ([05](05-further-observations.md#semantics).)
- **A "which fork API?" table** covering `fork`, `forks(k)`, the `From` impls
  for arrays, and the proposed `into_forks(n)` and `forking()`.
  ([04](04-forking-api.md), [05](05-further-observations.md#api).)

## 2. Instruments before the notation and the laws

- **A round-trip check for the notation.** A parser that lives only in
  tests, plus a proptest that `Debug` output parses back to the same value,
  makes "the notation loses nothing" a committed fact (ruled: no public
  parser). It is also an independent text decoder of the canonical form,
  checked against the binary one.
- **Laws as property tests**, each listed in the README's triage section
  with its status in [02](02-measures.md): the one-tick bound on the peak,
  the `ticks(k)` bounds, `min_ticks ≥ peak`, `rank ≤ min_ticks`, modularity
  of `rank` and width, submodularity of `min_ticks`, and `min_ticks` as the
  floors' brick count. Check first which the function-space oracle already
  implies.
- **A floors oracle.** Computing `rank` (widths summed over floors) and
  `min_ticks` (pieces counted over floors) from the floors is a third
  algorithm, independent of both the leaf integral and the subtree-minima
  fold. Each quantity is then computable two ways, so each deserves a
  committed comparison.
- **A fuelscape panel and a documented bound for `Debug`** before it lands.
  The bound is superlinear in encoded size (ruled acceptable;
  [05](05-further-observations.md#notation)).

## 3. The notation (ruled)

- `Debug` for `Party` (`Party(0.001..0.01, …)`), `Version` and `Span`
  (`Version { … }` match arms in braces, merged maximal runs; `Span` as a
  height band), `Clock` (its version with its own arms annotated
  `/* own */`),
  and the `causally` forms, which inherit it.
- Views render as the value they denote: `OwnVersion`, `OwnSpan`, `Ranked`
  (README, Views).
- `{:#?}` as one arm per line, owner comments vertically aligned when a
  legend is attached.
- `fmt::Binary` for today's bit-stream view.
- `Rank`'s text: decimal integer part, binary fraction, through `Display`,
  `FromStr`, and `serde`'s human-readable form; regenerate its fuelscape
  panel.
- Update the expected strings in `causally/tests.rs`.
- The `skyline()` drawing adapter, with ordinal axes
  ([03](03-views.md#version-notation-the-alternatives)).

## 4. Public API reshaping (proposals Finch has endorsed in direction)

- **Spans by value.** `Span` holds `Version`s, `Crossed` carries them back,
  `Query`'s bounds and `Ranked` follow, and rumors' tree code migrates (README,
  Spans by value). It precedes the diff, which builds on `Crossed`.
- **`into_forks(n)`** for `Party` and `Clock` ([04](04-forking-api.md)).

## 5. Diagnostics built on the notation

- **The causal diff and named assertion macros**, with a summary line from
  private lag-height and lag-width folds (README, Causal diff;
  [02](02-measures.md#computing-the-measures)). Settle the macro names
  ([05](05-further-observations.md#notation)). If `Diff` lands, the display
  is its `Debug` and the summary line measures its parts
  ([07](07-signed-versions.md)).
- **`Legend`**: names of any `N: Display`, built from `(name, &Party)` pairs,
  infallible, collapsing to version-vector form wherever each owner's region
  is flat (README, Legend; [05](05-further-observations.md#notation)).
- **Explanations for failing predicates**: `Span::place` as a three-column
  table, and `Query::contains` naming the failed condition
  ([05](05-further-observations.md#notation)).

## 6. Architecture, after the measures land

- **Drivers and folds.** Separate the leaf-stream drivers (one version, or a
  pair with an orientation) from the folds (width-weighted integral, brick
  count, running maximum, indicator integral). The pair driver must expose
  turn depths and funding bits ([02](02-measures.md#computing-the-measures)).
- **Lockstep walks.** [The audit](../2026-10-10-before-lockstep-walk-audit/README.md)
  finds one boundary-advance rule implemented three times (`overlay::advance`,
  `overlay::advance_set`, `shape::advance_refinement`) behind three cursor
  traits. It recommends a differential test of the rule, dissolving `Refine`
  and `advance_refinement` into `advance_set`, and returning the flip level
  from `advance`, which the drivers-and-folds separation needs.

## 7. Deferred until a consumer appears

- A structured twin of the notation (runs with positions), which needs a
  public dyadic position type and inherits the notation's superlinear size
  ([05](05-further-observations.md#api)).
- Public `peak` and `base`, and their skews ([02](02-measures.md#signed-comparison-skew)).
- The partial-order view and the knowledge matrix ([03](03-views.md)).
- `forking()` ([04](04-forking-api.md)).
- Parsimony distance ([02](02-measures.md#grounding-the-signed-brick-distance)).
- `Overlap` naming the overlapping region (ruled: opaque for now).

## 8. Beyond the crate

- **`before-viz`.** An ordinal axis, or a minimum width per region, so narrow
  regions stay visible past the width cap; and the textual notation on
  hover.
- **Figures in rustdoc generated from real runs.** The paper's §5.1 run
  figure, rebuilt from actual `fork`, `tick`, and `join` calls and rendered
  as theme-reactive inline SVG, the way the space-consumption figure is
  derived from a committed artifact today.
- **rumors.** `rumors-tracing` choosing between `%v` and `?v` deliberately;
  a ruling on whether readable `Debug` may appear in `insta` snapshots; and
  convergence tests that use the peak as a depth-independent progress
  measure.

## Process

- Probes against the public API, kept with notes, produced three findings
  here: the missing positions, the opaque `Debug` in a real failure, and an
  easy misreading of `forks`. They are cheap to repeat.
