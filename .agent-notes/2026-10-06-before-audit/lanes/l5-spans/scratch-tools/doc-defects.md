# L5 documentation defects (prose contradicting code)

Three rustdoc statements in the lane are false against the code at `58285ca5`.
None has a failing test; each is a prose correction toward the code. All three
are self-contained and behavior-preserving, so one builder branch can carry
them (one commit each, or one commit "Correct causal-query prose").

Provenance: all three verified by reading the cited lines at `58285ca5` and
checking the claim against the code named beside it. Item 1 was first raised
as `span-causally-34` in the 2026-09-01 review and remains open; item 2's
`after(p)` half was raised inside `span-causally-5`.

## 1. A test law's doc claims `Empty` coverage "cannot be complete"

- Where: `crates/before/src/testing/laws/version_triple.rs:203-213`, doc of
  `coverage_bounds_membership`.
- Text: "`Partial` promises nothing pointwise: the [`Coverage`] docs carry the
  precision contract, including why `Empty` cannot be complete."
- Contradiction:
  - `Coverage`'s docs (`crates/before/src/causally/query.rs:51-60`) state
    exact meanings for all three verdicts ("Some covered versions are
    admitted and some are not"; "No version the span covers is admitted").
    They contain no precision contract and no incompleteness argument.
  - `Query::coverage` is exact. `refine_partial` (`query.rs:220-252`)
    decides the emptiness the fused walk cannot, and both
    `coverage_is_exact_on_the_two_party_grid` (`causally/tests.rs`) and this
    audit's census harness (921,456 exhaustive verdicts on the 4-cell
    boolean cube, and sampled census properties run at 4,000 cases each)
    confirm exactness.
- Why it matters: the repo's rule is that every test doc states its invariant
  correctly. This one tells a maintainer that `Empty` is incomplete, which
  invites a "fix" that weakens `coverage`.
- Correction: state what the law pins and point at where completeness is
  pinned, for example:
  "This law pins soundness only: `Full` admits and `Empty` rejects the
  constructed in-segment probes. `Partial` is not checked pointwise here;
  exactness of all three verdicts is pinned by
  `coverage_is_exact_on_the_two_party_grid`."

## 2. `toward`'s rustdoc names parameters that do not exist

- Where: `crates/before/src/causally/forms.rs:236-239`.
- Signature: `pub fn toward<'a>(s: ..., t: ...) -> Query<'a, Up>`.
- Text: "Everything in the causal future of `s` (including `s` itself) but
  nothing in the causal future of `e` (including `e` itself).
  Equivalent to `after(p) & until(t)`."
- Contradiction: there is no `e` and no `p`; the body is `after(s) & until(t)`.
- Correction: "Everything in the causal future of `s` (including `s` itself)
  but nothing in the causal future of `t` (including `t` itself).
  Equivalent to `after(s) & until(t)`."
- Related readability nit in the same example: `let t = toward(&a1, &a3);`
  binds the *query* as `t`, then the comment "`t` is already reached" means
  the *parameter* `t` (that is, `a3`). Renaming the binding (`let q = ...`)
  removes the collision.

## 3. `Polarity` says `Down`/`Up` queries have "one or more holes"

- Where: `crates/before/src/causally/polarity.rs:227-233`.
- Text: "The marker types describe which holes a query may contain: ...
  A [`Down`] query has one or more holes ... An [`Up`] query has one or more
  holes ..."
- Contradiction: conjunction prunes holes the merged floor or ceiling cannot
  reach (`Query::and`, `conjunction.rs`). The resulting `Query<Down>` or
  `Query<Up>` can hold zero holes. `conjunction_normalizes` in
  `causally/tests.rs` constructs one: `after(&w.a2) & since(&w.a1)` renders 0
  holes and has type `Query<'_, Down>`. The bullet list also contradicts its
  own lead sentence ("may contain").
- Correction: "A [`Down`] query's holes, if any, are *down-sets* ..." and
  dually for [`Up`].
