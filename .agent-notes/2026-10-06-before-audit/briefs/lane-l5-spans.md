<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the lane section for the L5 auditor, modeled on the L6 codec lane; under review by Finch before launch. -->

# Lane L5: spans and causal queries

## Scope

This lane's theme is *intervals of causal history*: asking where a version sits
relative to a pair of bounds, or relative to a richer predicate. A `Span` is an
ordered pair `lo <= hi` with two algebras: one pointwise over the endpoints,
and one treating spans as sets of versions. Its verdict methods report a
version's relation to both endpoints at different granularities. A `Query`
generalizes the interval to optional floors and ceilings with holes, each
carrying a polarity.

Some things to consider, as starting points rather than a checklist. They
are prompts for your own exploration: pursue whatever else the code
suggests, and expect the most valuable questions to be ones not listed
here.

- Does every verdict agree with the two underlying comparisons against `lo`
  and `hi`?
- Do the algebras mean what they claim, as lattice operations or as set
  operations on the versions they contain?
- Does `coverage` classify a span exactly as enumerating its versions would?
- Do the fused, early-exiting implementations ever answer differently from
  the naive composition of comparisons?
- Do spans and queries agree wherever both can express a question?

The identifiers below are a starting map, not a boundary. Follow the theme
wherever the code takes it, including into code this list does not name.

- **`Span`:** construction (`new`, `at`, conversions), the verdicts (`place`,
  `dominance`, `precedence`, `contains`), the pointwise (`|`, `&`) and
  containment (`+`, `*`) algebras with their `_all` forms, `project`, and the
  accessors.
- **`OwnSpan`**, and the verdict types `Placement`, `Dominance`,
  `Precedence`, `Endpoint`.
- **`causally`:** the atom constructors, negation, `or_concurrent`,
  conjunction, `contains`, `coverage`, and the polarity types.
- **Implementation:** `src/span.rs`, `src/span/`, `src/causally.rs`,
  `src/causally/`.

Neighboring lanes explore adjacent themes. When your work crosses into one,
establish what you found and report it, rather than continuing the
investigation there. L6 explores the span's wire form, and L2 the `Version`
comparisons beneath every verdict.

## Contracts to read first

- `Span`'s module docs: the two algebras and their endpoint formulas.
- `Span::place`, `dominance`, `precedence`, and `contains`, and how each
  coarsens the one before it.
- The `causally` module docs: the atom table, polarity, and the conversion
  of a span into `after(lo) & before(hi)`.

## Prior coverage to map

- **Tests:** `src/span/tests.rs`, `src/causally/tests.rs`,
  `tests/verdict_matrix.rs`, `tests/coincident_span.rs`,
  `tests/meter/span.rs`, `tests/meter/placement.rs`.
- **September partitions:** `span-causally.md`.

## Closed fixes to re-attack

- Multi-hole query auxiliary space when numeric width and bound count vary
  independently.
- Query conjunction over independently growing hole sets.
- The corrected Span algebra identities.
- Causal-query construction verified against relation-level oracles over a
  complete finite interval.

## Candidate leads (evaluate, don't confirm)

1. **Every verdict agrees with two comparisons.** `place`, `dominance`,
   `precedence`, and `contains` agree with the pair of `partial_cmp` results
   against `lo` and `hi`, for arbitrary versions, concurrent ones included.
   The same must hold for `OwnSpan` against projected spans.
2. **Containment algebra as sets.** `a * b` is exactly the set intersection
   (or `None`); `a + b` is the least span containing both. Check both on
   small universes where every version can be enumerated.
3. **`coverage` matches enumeration.** On small universes, `coverage`
   classifies a span exactly as enumerating its member versions would, for
   every polarity and mixture of holes.
4. **Spans and queries agree.** A span's `contains` agrees with membership
   in its converted query.
